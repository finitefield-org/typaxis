use super::*;
use std::fs;
use std::os::unix::fs::symlink;

#[test]
fn borrowed_candidate_checks_match_owned_resolution_and_restore_scratch() {
    let tree = tests::TempTree::new();
    fs::create_dir(tree.0.join("dir")).unwrap();
    fs::create_dir(tree.0.join("outside")).unwrap();
    fs::write(tree.0.join("dir/file.bin"), b"inside").unwrap();
    fs::write(tree.0.join("outside/file.bin"), b"outside").unwrap();
    symlink("dir", tree.0.join("linked-dir")).unwrap();
    symlink("absent", tree.0.join("dangling-dir")).unwrap();
    symlink("cycle-b", tree.0.join("cycle-a")).unwrap();
    symlink("cycle-a", tree.0.join("cycle-b")).unwrap();
    fs::create_dir(tree.0.join("日本語")).unwrap();
    symlink("日本語", tree.0.join("別名")).unwrap();
    let root = admit_root(&tree.0).unwrap();
    let unrelated = root
        .candidate_parent_leaf(&PortablePath::new("unrelated.bin").unwrap())
        .unwrap();
    let mut cases = vec![
        "dir/file.bin".to_owned(),
        "dir/missing.bin".to_owned(),
        "missing/dir/file.bin".to_owned(),
        "linked-dir/file.bin".to_owned(),
        "linked-dir/missing/file.bin".to_owned(),
        "dangling-dir/file.bin".to_owned(),
        "dir/file.bin/leaf".to_owned(),
        "dir".to_owned(),
        "cycle-a/file.bin".to_owned(),
        "日本語/数式.bin".to_owned(),
        "別名/未作成/数式.bin".to_owned(),
        format!("linked-dir/{}/leaf.bin", "x".repeat(300)),
    ];
    for length in [255, 256, 257, 1842] {
        let mut name = String::new();
        while length - name.len() > 200 {
            name.push_str(&"d".repeat(180));
            name.push('/');
        }
        name.push_str(&"f".repeat(length - name.len()));
        let file = tree.0.join(&name);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"long").unwrap();
        cases.push(name.clone());
        cases.push(format!("missing/{name}"));
    }
    for name in cases {
        let path = PortablePath::new(name).unwrap();
        let native = root.candidate_parent_leaf(&path);
        let mut prepared = PreparedReadPath::new(&path).unwrap();
        for _ in 0..2 {
            assert_eq!(
                prepared.candidate_identity_matches(&root, &path, &unrelated),
                native
                    .as_ref()
                    .map(|identity| identity == &unrelated)
                    .map_err(Clone::clone),
                "{}",
                path.as_str()
            );
            if let Ok(expected) = &native {
                assert!(prepared
                    .candidate_identity_matches(&root, &path, expected)
                    .unwrap());
                assert!(root.candidate_identity_matches(&path, expected).unwrap());
            }
            assert_eq!(
                prepared.open(&root).map(|file| file.map(|f| f.identity())),
                root.open_candidate(&path)
                    .map(|file| file.map(|f| f.identity())),
                "{}",
                path.as_str()
            );
        }
    }
}

#[test]
fn logical_candidate_checks_preserve_non_utf8_roots_and_long_prefixes() {
    use std::os::unix::ffi::OsStringExt;
    let tree = tests::TempTree::new();
    for component in [
        OsString::from_vec(b"root-\xff".to_vec()),
        OsString::from("d".repeat(180)),
    ] {
        let directory = tree.0.join(component).join("e".repeat(180));
        fs::create_dir_all(directory.join("destination")).unwrap();
        symlink("destination", directory.join("alias")).unwrap();
        let root = admit_root(&directory).unwrap();
        for text in [
            "alias/leaf.bin",
            "alias/missing/数式.bin",
            "alias/destination/leaf.bin",
        ] {
            let path = PortablePath::new(text).unwrap();
            let expected = root.candidate_parent_leaf(&path).unwrap();
            assert!(logical_candidate::matches(&root, &path, &expected).unwrap());
            assert!(root.candidate_identity_matches(&path, &expected).unwrap());
            assert_eq!(
                root.open_candidate(&path).unwrap_err(),
                HostAdmissionError::UnsafeCandidate
            );
        }
    }
}

#[test]
fn logical_candidate_checks_cover_root_and_renamed_admission_paths() {
    let tree = tests::TempTree::new();
    let directory = tree.0.join("admitted");
    fs::create_dir_all(directory.join("destination")).unwrap();
    symlink("destination", directory.join("alias")).unwrap();
    let root = admit_root(&directory).unwrap();
    let path = PortablePath::new("alias/leaf.bin").unwrap();
    let expected = root.candidate_parent_leaf(&path).unwrap();
    fs::rename(&directory, tree.0.join("moved")).unwrap();
    let actual = root.candidate_parent_leaf(&path).unwrap();
    assert_ne!(actual, expected);
    assert!(!logical_candidate::matches(&root, &path, &expected).unwrap());
    assert!(logical_candidate::matches(&root, &path, &actual).unwrap());
    let filesystem_root = admit_root(Path::new("/")).unwrap();
    let relative = PortablePath::new(format!(
        "{}/moved/alias/leaf.bin",
        tree.0.strip_prefix("/").unwrap().display()
    ))
    .unwrap();
    let expected = filesystem_root.candidate_parent_leaf(&relative).unwrap();
    assert!(logical_candidate::matches(&filesystem_root, &relative, &expected).unwrap());
    let missing =
        PortablePath::new(format!("missing-typaxis-{}/leaf.bin", std::process::id())).unwrap();
    let expected = filesystem_root.candidate_parent_leaf(&missing).unwrap();
    assert!(logical_candidate::matches(&filesystem_root, &missing, &expected).unwrap());
}

#[test]
fn borrowed_candidate_checks_reject_changed_missing_suffixes_and_parent_inodes() {
    let tree = tests::TempTree::new();
    let root = admit_root(&tree.0).unwrap();
    let path = PortablePath::new("missing/parent/leaf.bin").unwrap();
    let expected = root.candidate_parent_leaf(&path).unwrap();
    assert!(root.candidate_identity_matches(&path, &expected).unwrap());
    fs::create_dir(tree.0.join("missing")).unwrap();
    assert!(!root.candidate_identity_matches(&path, &expected).unwrap());
    let expected = root.candidate_parent_leaf(&path).unwrap();
    fs::rename(tree.0.join("missing"), tree.0.join("retained")).unwrap();
    fs::create_dir(tree.0.join("missing")).unwrap();
    assert!(!root.candidate_identity_matches(&path, &expected).unwrap());
    // Aliased parents keep the original resolver's identity and remain
    // forbidden for a contained read.
    fs::create_dir(tree.0.join("retained/parent")).unwrap();
    fs::write(tree.0.join("retained/parent/leaf.bin"), b"outside").unwrap();
    fs::remove_dir(tree.0.join("missing")).unwrap();
    symlink("retained", tree.0.join("missing")).unwrap();
    let expected = root.candidate_parent_leaf(&path).unwrap();
    assert!(root.candidate_identity_matches(&path, &expected).unwrap());
    assert_eq!(
        root.open_candidate(&path).unwrap_err(),
        HostAdmissionError::UnsafeCandidate
    );
}

#[cfg(target_os = "linux")]
#[test]
fn logical_candidate_checks_preserve_unlinked_magic_target_errors() {
    use std::os::fd::AsRawFd;
    let tree = tests::TempTree::new();
    let destination = tree.0.join("destination");
    fs::create_dir(&destination).unwrap();
    let directory = File::open(&destination).unwrap();
    let descriptor_path = format!("/proc/self/fd/{}", directory.as_raw_fd());
    if !Path::new(&descriptor_path).exists() {
        return;
    }
    symlink(descriptor_path, tree.0.join("alias")).unwrap();
    let root = admit_root(&tree.0).unwrap();
    let path = PortablePath::new("alias/leaf.bin").unwrap();
    let expected = root.candidate_parent_leaf(&path).unwrap();
    assert!(logical_candidate::matches(&root, &path, &expected).unwrap());
    fs::remove_dir(&destination).unwrap();
    assert!(fs::metadata(tree.0.join("alias")).unwrap().is_dir());
    assert!(fs::canonicalize(tree.0.join("alias")).is_err());
    assert_eq!(
        root.candidate_parent_leaf(&path),
        Err(HostAdmissionError::Read)
    );
    assert_eq!(
        logical_candidate::matches(&root, &path, &expected),
        Err(HostAdmissionError::Read)
    );
    assert_eq!(
        root.candidate_identity_matches(&path, &expected),
        Err(HostAdmissionError::Read)
    );
}
