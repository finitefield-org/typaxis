use crate::HostAdmissionError;
use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};
use typaxis_core::{HostPath, PortablePath};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct FileIdentity {
    pub(crate) device: u128,
    pub(crate) inode: u128,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CandidatePathIdentity {
    ancestor: FileIdentity,
    suffix: Vec<OsString>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WriteTargetIdentity {
    pub(crate) candidate: CandidatePathIdentity,
    pub(crate) existing: Option<FileIdentity>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FileKind {
    Directory,
    Regular,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FileSnapshot {
    pub(crate) identity: FileIdentity,
    pub(crate) length: u64,
    modified_seconds: i128,
    modified_nanoseconds: u128,
    changed_seconds: i128,
    changed_nanoseconds: u128,
    pub(crate) kind: FileKind,
}

impl FileSnapshot {
    pub(crate) fn from_file(file: &File) -> Result<Self, HostAdmissionError> {
        let stat = rustix::fs::fstat(file).map_err(|_| HostAdmissionError::Read)?;
        let length = u64::try_from(stat.st_size).map_err(|_| HostAdmissionError::LengthMismatch)?;
        let kind = match rustix::fs::FileType::from_raw_mode(stat.st_mode) {
            rustix::fs::FileType::Directory => FileKind::Directory,
            rustix::fs::FileType::RegularFile => FileKind::Regular,
            _ => FileKind::Other,
        };
        #[cfg(any(target_os = "android", target_os = "linux"))]
        let device = u128::from(stat.st_dev);
        #[cfg(target_os = "macos")]
        let device = u128::try_from(stat.st_dev).map_err(|_| HostAdmissionError::Read)?;
        let inode = u128::from(stat.st_ino);
        #[cfg(target_os = "linux")]
        let modified_nanoseconds = u128::from(stat.st_mtime_nsec);
        #[cfg(any(target_os = "android", target_os = "macos"))]
        let modified_nanoseconds =
            u128::try_from(stat.st_mtime_nsec).map_err(|_| HostAdmissionError::Read)?;
        #[cfg(target_os = "linux")]
        let changed_nanoseconds = u128::from(stat.st_ctime_nsec);
        #[cfg(any(target_os = "android", target_os = "macos"))]
        let changed_nanoseconds =
            u128::try_from(stat.st_ctime_nsec).map_err(|_| HostAdmissionError::Read)?;
        Ok(Self {
            identity: FileIdentity { device, inode },
            length,
            modified_seconds: i128::from(stat.st_mtime),
            modified_nanoseconds,
            changed_seconds: i128::from(stat.st_ctime),
            changed_nanoseconds,
            kind,
        })
    }
}

pub(crate) struct AdmittedRoot {
    directory: File,
    canonical_path: PathBuf,
    identity: FileIdentity,
}

impl fmt::Debug for AdmittedRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AdmittedRoot(..)")
    }
}

pub(crate) fn admit_root(path: &Path) -> Result<AdmittedRoot, HostAdmissionError> {
    use rustix::fs::{openat, Mode, OFlags, CWD};

    // A root symlink is intentionally allowed and collapsed once. Every path
    // below this admitted handle is opened without following symlinks.
    let canonical = std::fs::canonicalize(path).map_err(|_| HostAdmissionError::RootUnavailable)?;
    let descriptor = openat(
        CWD,
        &canonical,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::DIRECTORY | OFlags::NOFOLLOW,
        Mode::empty(),
    )
    .map_err(|error| {
        if error == rustix::io::Errno::NOTDIR {
            HostAdmissionError::RootNotDirectory
        } else {
            HostAdmissionError::RootUnavailable
        }
    })?;
    let directory: File = descriptor.into();
    let snapshot = FileSnapshot::from_file(&directory)?;
    if snapshot.kind != FileKind::Directory {
        return Err(HostAdmissionError::RootNotDirectory);
    }
    Ok(AdmittedRoot {
        directory,
        canonical_path: canonical,
        identity: snapshot.identity,
    })
}

impl AdmittedRoot {
    pub(crate) const fn identity(&self) -> FileIdentity {
        self.identity
    }

    pub(crate) fn snapshot(&self) -> Result<FileSnapshot, HostAdmissionError> {
        FileSnapshot::from_file(&self.directory)
    }

    pub(crate) fn candidate_parent_leaf(
        &self,
        path: &PortablePath,
    ) -> Result<CandidatePathIdentity, HostAdmissionError> {
        let components: Vec<&str> = path.as_str().split('/').collect();
        let parent_count = components.len().saturating_sub(1);
        let mut current = None;

        for (index, component) in components[..parent_count].iter().enumerate() {
            match open_directory_component(self, current.as_ref(), component) {
                Err(HostAdmissionError::UnsafeCandidate) => {
                    return resolve_parent_leaf_path(&portable_host_path(
                        &self.canonical_path,
                        path,
                    ));
                }
                Err(error) => return Err(error),
                Ok(Some(directory)) => current = Some(directory),
                Ok(None) => {
                    let ancestor = snapshot_directory(self, current.as_ref())?.identity;
                    return Ok(CandidatePathIdentity {
                        ancestor,
                        suffix: components[index..].iter().map(OsString::from).collect(),
                    });
                }
            }
        }

        let ancestor = snapshot_directory(self, current.as_ref())?.identity;
        Ok(CandidatePathIdentity {
            ancestor,
            suffix: vec![OsString::from(components[components.len() - 1])],
        })
    }

    pub(crate) fn open_candidate(
        &self,
        path: &PortablePath,
    ) -> Result<Option<OpenedCandidate>, HostAdmissionError> {
        #[cfg(any(target_os = "android", target_os = "linux"))]
        {
            match open_candidate_with_openat2(self, path) {
                Err(Openat2Failure::Unavailable) => open_candidate_component_walker(self, path),
                Err(Openat2Failure::Admission(error)) => Err(error),
                Ok(opened) => Ok(opened),
            }
        }
        #[cfg(target_os = "macos")]
        {
            open_candidate_component_walker(self, path)
        }
    }
}

fn portable_host_path(root: &Path, path: &PortablePath) -> PathBuf {
    let mut host = root.to_path_buf();
    host.extend(path.as_str().split('/'));
    host
}

fn snapshot_directory(
    root: &AdmittedRoot,
    current: Option<&File>,
) -> Result<FileSnapshot, HostAdmissionError> {
    let snapshot = match current {
        Some(directory) => FileSnapshot::from_file(directory)?,
        None => root.snapshot()?,
    };
    if snapshot.kind != FileKind::Directory {
        return Err(HostAdmissionError::UnsafeCandidate);
    }
    Ok(snapshot)
}

fn open_directory_component(
    root: &AdmittedRoot,
    current: Option<&File>,
    component: &str,
) -> Result<Option<File>, HostAdmissionError> {
    use rustix::fs::{openat, Mode, OFlags};

    let flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::DIRECTORY | OFlags::NOFOLLOW;
    let opened = match current {
        Some(directory) => openat(directory, component, flags, Mode::empty()),
        None => openat(&root.directory, component, flags, Mode::empty()),
    };
    let descriptor = match opened {
        Ok(descriptor) => descriptor,
        Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
        Err(_) => return Err(HostAdmissionError::UnsafeCandidate),
    };
    let file: File = descriptor.into();
    if FileSnapshot::from_file(&file)?.kind != FileKind::Directory {
        return Err(HostAdmissionError::UnsafeCandidate);
    }
    Ok(Some(file))
}

fn open_candidate_component_walker(
    root: &AdmittedRoot,
    path: &PortablePath,
) -> Result<Option<OpenedCandidate>, HostAdmissionError> {
    use rustix::fs::{openat, Mode, OFlags};

    let mut components = path.as_str().split('/').peekable();
    let mut current = None;
    while let Some(component) = components.next() {
        if components.peek().is_some() {
            match open_directory_component(root, current.as_ref(), component)? {
                Some(directory) => current = Some(directory),
                None => return Ok(None),
            }
            continue;
        }

        let flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK;
        let opened = match current.as_ref() {
            Some(directory) => openat(directory, component, flags, Mode::empty()),
            None => openat(&root.directory, component, flags, Mode::empty()),
        };
        let descriptor = match opened {
            Ok(descriptor) => descriptor,
            Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
            Err(_) => return Err(HostAdmissionError::UnsafeCandidate),
        };
        return opened_regular_file(descriptor.into());
    }
    Err(HostAdmissionError::UnsafeCandidate)
}

#[cfg(any(target_os = "android", target_os = "linux"))]
enum Openat2Failure {
    Unavailable,
    Admission(HostAdmissionError),
}

#[cfg(any(target_os = "android", target_os = "linux"))]
fn open_candidate_with_openat2(
    root: &AdmittedRoot,
    path: &PortablePath,
) -> Result<Option<OpenedCandidate>, Openat2Failure> {
    use rustix::fs::{openat2, Mode, OFlags, ResolveFlags};

    let descriptor = match openat2(
        &root.directory,
        path.as_str(),
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_SYMLINKS,
    ) {
        Ok(descriptor) => descriptor,
        Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
        Err(error) if error == rustix::io::Errno::NOSYS => return Err(Openat2Failure::Unavailable),
        Err(_) => {
            return Err(Openat2Failure::Admission(
                HostAdmissionError::UnsafeCandidate,
            ))
        }
    };
    opened_regular_file(descriptor.into()).map_err(Openat2Failure::Admission)
}

fn opened_regular_file(file: File) -> Result<Option<OpenedCandidate>, HostAdmissionError> {
    let before_lock = FileSnapshot::from_file(&file)?;
    if before_lock.kind != FileKind::Regular {
        return Err(HostAdmissionError::NotRegularFile);
    }
    Ok(Some(OpenedCandidate { file, before_lock }))
}

#[derive(Debug)]
pub(crate) struct OpenedCandidate {
    pub(crate) file: File,
    before_lock: FileSnapshot,
}

impl OpenedCandidate {
    pub(crate) const fn identity(&self) -> FileIdentity {
        self.before_lock.identity
    }

    pub(crate) fn lock(self) -> Result<(File, FileSnapshot), HostAdmissionError> {
        rustix::fs::flock(
            &self.file,
            rustix::fs::FlockOperation::NonBlockingLockShared,
        )
        .map_err(|_| HostAdmissionError::LockUnavailable)?;
        let snapshot = FileSnapshot::from_file(&self.file)?;
        if snapshot != self.before_lock {
            return Err(HostAdmissionError::LengthMismatch);
        }
        Ok((self.file, snapshot))
    }
}

pub(crate) fn resolve_write_target(
    target: &HostPath,
) -> Result<WriteTargetIdentity, HostAdmissionError> {
    use std::os::unix::fs::MetadataExt;

    let absolute = if target.as_path().is_absolute() {
        target.as_path().to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| HostAdmissionError::Read)?
            .join(target.as_path())
    };
    let candidate = resolve_parent_leaf_path(&absolute)?;
    let existing = std::fs::metadata(&absolute)
        .ok()
        .map(|metadata| FileIdentity {
            device: u128::from(metadata.dev()),
            inode: u128::from(metadata.ino()),
        });
    Ok(WriteTargetIdentity {
        candidate,
        existing,
    })
}

fn resolve_parent_leaf_path(path: &Path) -> Result<CandidatePathIdentity, HostAdmissionError> {
    use std::os::unix::fs::MetadataExt;

    let leaf = path
        .file_name()
        .ok_or(HostAdmissionError::UnsafeCandidate)?
        .to_os_string();
    let mut ancestor = path.parent().ok_or(HostAdmissionError::UnsafeCandidate)?;
    let mut suffix = vec![leaf];
    while !std::fs::metadata(ancestor).is_ok_and(|metadata| metadata.is_dir()) {
        let component = ancestor
            .file_name()
            .ok_or(HostAdmissionError::UnsafeCandidate)?
            .to_os_string();
        suffix.push(component);
        ancestor = ancestor
            .parent()
            .ok_or(HostAdmissionError::UnsafeCandidate)?;
    }
    suffix.reverse();
    let canonical_ancestor =
        std::fs::canonicalize(ancestor).map_err(|_| HostAdmissionError::Read)?;
    let ancestor_metadata =
        std::fs::metadata(&canonical_ancestor).map_err(|_| HostAdmissionError::Read)?;
    if !ancestor_metadata.is_dir() {
        return Err(HostAdmissionError::UnsafeCandidate);
    }
    Ok(CandidatePathIdentity {
        ancestor: FileIdentity {
            device: u128::from(ancestor_metadata.dev()),
            inode: u128::from(ancestor_metadata.ino()),
        },
        suffix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TempTree(PathBuf);

    impl TempTree {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "typaxis-host-platform-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn component_walker_opens_nested_regular_file() {
        let tree = TempTree::new();
        fs::create_dir(tree.0.join("nested")).unwrap();
        fs::write(tree.0.join("nested/file.bin"), b"bytes").unwrap();
        let root = admit_root(&tree.0).unwrap();
        let opened =
            open_candidate_component_walker(&root, &PortablePath::new("nested/file.bin").unwrap())
                .unwrap()
                .unwrap();
        assert_eq!(opened.before_lock.length, 5);
    }

    #[test]
    fn component_walker_rejects_intermediate_and_terminal_symlinks() {
        use std::os::unix::fs::symlink;

        let tree = TempTree::new();
        fs::create_dir(tree.0.join("actual")).unwrap();
        fs::write(tree.0.join("actual/file.bin"), b"bytes").unwrap();
        symlink("actual", tree.0.join("linked-dir")).unwrap();
        symlink("actual/file.bin", tree.0.join("linked-file")).unwrap();
        let root = admit_root(&tree.0).unwrap();
        assert_eq!(
            open_candidate_component_walker(
                &root,
                &PortablePath::new("linked-dir/file.bin").unwrap(),
            )
            .unwrap_err(),
            HostAdmissionError::UnsafeCandidate
        );
        assert_eq!(
            open_candidate_component_walker(&root, &PortablePath::new("linked-file").unwrap(),)
                .unwrap_err(),
            HostAdmissionError::UnsafeCandidate
        );
    }
}
