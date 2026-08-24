#![cfg(unix)]

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "typaxis-font-command-test-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create unique test directory");
        Self(path)
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(directory: &Path, arguments: &[&OsStr]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_typaxis"));
    command.current_dir(directory).args(arguments);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("TYPAXIS_") {
            command.env_remove(key);
        }
    }
    command.output().expect("CLI process must start")
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn utf16_be(value: &str) -> Vec<u8> {
    value.encode_utf16().flat_map(u16::to_be_bytes).collect()
}

fn make_name_table(family: &str, full_name: &str, postscript_name: &str) -> Vec<u8> {
    let values = [
        (1u16, utf16_be(family)),
        (4u16, utf16_be(full_name)),
        (6u16, utf16_be(postscript_name)),
    ];
    let storage_offset = 6 + values.len() * 12;
    let mut table = Vec::new();
    push_u16(&mut table, 0);
    push_u16(&mut table, values.len() as u16);
    push_u16(&mut table, storage_offset as u16);
    let mut string_offset = 0usize;
    for (name_id, value) in &values {
        push_u16(&mut table, 3);
        push_u16(&mut table, 1);
        push_u16(&mut table, 0x0409);
        push_u16(&mut table, *name_id);
        push_u16(&mut table, value.len() as u16);
        push_u16(&mut table, string_offset as u16);
        string_offset += value.len();
    }
    for (_, value) in values {
        table.extend_from_slice(&value);
    }
    table
}

fn make_sfnt(family: &str, full_name: &str, postscript_name: &str, glyph_count: u16) -> Vec<u8> {
    let name = make_name_table(family, full_name, postscript_name);
    let directory_length = 12 + 3 * 16;
    let head_offset = directory_length;
    let maxp_offset = head_offset + 54;
    let name_offset = maxp_offset + 6;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"\0\x01\0\0");
    push_u16(&mut bytes, 3);
    bytes.extend_from_slice(&[0; 6]);
    for (tag, offset, length) in [
        (*b"head", head_offset, 54usize),
        (*b"maxp", maxp_offset, 6usize),
        (*b"name", name_offset, name.len()),
    ] {
        bytes.extend_from_slice(&tag);
        push_u32(&mut bytes, 0);
        push_u32(&mut bytes, offset as u32);
        push_u32(&mut bytes, length as u32);
    }
    let mut head = [0u8; 54];
    head[12..16].copy_from_slice(&0x5f0f_3cf5u32.to_be_bytes());
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    bytes.extend_from_slice(&head);
    push_u32(&mut bytes, 0x0001_0000);
    push_u16(&mut bytes, glyph_count);
    bytes.extend_from_slice(&name);
    bytes
}

#[test]
fn inspect_and_list_fonts_emit_deterministic_json() {
    let directory = TempDirectory::new();
    fs::write(
        directory.0.join("b.ttf"),
        make_sfnt("Beta", "Beta Regular", "Beta-Regular", 9),
    )
    .unwrap();
    fs::write(
        directory.0.join("a.otf"),
        make_sfnt("Alpha", "Alpha Regular", "Alpha-Regular", 7),
    )
    .unwrap();
    fs::write(directory.0.join("notes.txt"), b"ignored").unwrap();

    let inspect = run(
        &directory.0,
        &[
            OsStr::new("inspect-font"),
            directory.0.join("a.otf").as_os_str(),
        ],
    );
    assert!(inspect.status.success());
    let inspection = String::from_utf8(inspect.stdout).unwrap();
    assert!(inspection.ends_with('\n'));
    assert!(inspection.contains("\"family\":\"Alpha\""));
    assert!(inspection.contains("\"glyph_count\":7"));
    assert!(!inspection.contains(&*directory.0.to_string_lossy()));
    assert!(inspect.stderr.is_empty());

    let listing = run(
        &directory.0,
        &[
            OsStr::new("list-fonts"),
            OsStr::new("--font-dir"),
            directory.0.as_os_str(),
        ],
    );
    assert!(listing.status.success());
    let listing = String::from_utf8(listing.stdout).unwrap();
    assert!(listing.find("a.otf").unwrap() < listing.find("b.ttf").unwrap());
    assert!(!listing.contains("notes.txt"));
    assert!(!listing.contains(&*directory.0.to_string_lossy()));
}

#[test]
fn font_commands_preserve_input_and_io_exit_classes() {
    let directory = TempDirectory::new();
    fs::write(directory.0.join("broken.ttf"), b"not a font").unwrap();

    let invalid = run(
        &directory.0,
        &[OsStr::new("inspect-font"), OsStr::new("broken.ttf")],
    );
    assert_eq!(invalid.status.code(), Some(1));
    assert!(invalid.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("F4000:"));

    let oversized_path = directory.0.join("oversized.ttf");
    fs::File::create(&oversized_path)
        .unwrap()
        .set_len(128 * 1024 * 1024 + 1)
        .unwrap();
    let oversized = run(
        &directory.0,
        &[OsStr::new("inspect-font"), OsStr::new("oversized.ttf")],
    );
    assert_eq!(oversized.status.code(), Some(5));
    assert!(oversized.stdout.is_empty());
    assert!(String::from_utf8_lossy(&oversized.stderr).contains("I9000:"));

    let missing = run(
        &directory.0,
        &[OsStr::new("inspect-font"), OsStr::new("missing.ttf")],
    );
    assert_eq!(missing.status.code(), Some(3));
    assert!(missing.stdout.is_empty());
}

#[test]
fn font_positional_path_is_platform_native() {
    use std::os::unix::ffi::OsStringExt;

    let directory = TempDirectory::new();
    let file_name = OsString::from_vec(b"native-\xff.ttf".to_vec());
    fs::write(
        directory.0.join(&file_name),
        make_sfnt("Native", "Native Regular", "Native-Regular", 4),
    )
    .unwrap();

    let output = run(
        &directory.0,
        &[OsStr::new("inspect-font"), file_name.as_os_str()],
    );
    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).unwrap();
    assert!(output.contains("\"family\":\"Native\""));
    assert!(output.contains("6e61746976652dff2e747466"));
}
