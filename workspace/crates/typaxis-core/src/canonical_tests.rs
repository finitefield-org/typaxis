use super::{sha256, write_jcs_string, Sha256};
use std::fmt::{self, Write};

#[test]
fn sha256_borrowed_blocks_preserve_padding_boundary_vectors() {
    // Digests independently produced with Python hashlib for this byte pattern.
    for (length, expected) in [
        (
            55,
            "3a147922e2d3204c31b109973b2389929ee5b762d859a16f0dce6f06756e709a",
        ),
        (
            56,
            "cf551a6e572e7b84f9a567c61ef7223c8b5c432def73a56aa95cf2ac866ae719",
        ),
        (
            63,
            "14127f3d3ff5154261ffa3567a84e9d00a8a8bc89c2510e15865f6b5bdaa7820",
        ),
        (
            64,
            "175640ac3c7c1ee18bb1a4118a67fcae33b52b7fe6271de135c418d624334448",
        ),
        (
            65,
            "2f89f166abc1c0987bd4a79c33a628229d7bda6cc55caadcff6f3b63d2bd2889",
        ),
        (
            119,
            "96c398b286e1dcd43934ad8242be22f254d695b40f6851c591577ea4e13d736c",
        ),
        (
            120,
            "e0d12fd0e637149316902fa3e6831cfffba3ced8693dc132b9e2f77174951768",
        ),
        (
            127,
            "60c78ee2ec2fc239db386d45b742602804efdc8b0ac994f00b0828bb2efe200e",
        ),
        (
            128,
            "debadb6c5173013451666e3678db5ea719449d5696d7343b41821b9c02a5b4a7",
        ),
    ] {
        let bytes: Vec<u8> = (0..length)
            .map(|i| ((i * 73 + i / 11 + 19) & 255) as u8)
            .collect();
        let actual: String = sha256(&bytes).iter().map(|n| format!("{n:02x}")).collect();
        assert_eq!(actual, expected, "input length {length}");
        for split in 0..=length {
            let mut state = Sha256::new();
            state.update(&bytes[..split]);
            state.update(&[]);
            state.update(&bytes[split..]);
            state.update(&[]);
            let actual: String = state.finish().iter().map(|n| format!("{n:02x}")).collect();
            assert_eq!(actual, expected, "input length {length}, split {split}");
        }
    }
}

#[test]
fn sha256_streamed_writer_preserves_known_empty_and_text_vectors() {
    for (text, expected) in [
        (
            "",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            "abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        ),
    ] {
        let mut state = Sha256::default();
        state.write_str("").unwrap();
        for character in text.chars() {
            state.write_char(character).unwrap();
            state.write_str("").unwrap();
        }
        let actual: String = state.finish().iter().map(|n| format!("{n:02x}")).collect();
        assert_eq!(actual, expected);
    }
}

struct FixedWriter {
    bytes: [u8; 256],
    used: usize,
    limit: usize,
}
impl Write for FixedWriter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self
            .used
            .checked_add(value.len())
            .filter(|n| *n <= self.limit)
            .ok_or(fmt::Error)?;
        self.bytes[self.used..end].copy_from_slice(value.as_bytes());
        self.used = end;
        Ok(())
    }
}

#[test]
fn canonical_string_writer_preserves_escapes_and_propagates_bounded_write_failure() {
    let value = "\0\u{8}\t\n\u{c}\r\u{1f}\"\\α 📸";
    let expected = "\"\\u0000\\b\\t\\n\\f\\r\\u001f\\\"\\\\α 📸\"";
    for limit in 0..=expected.len() {
        let mut writer = FixedWriter {
            bytes: [0; 256],
            used: 0,
            limit,
        };
        let result = write_jcs_string(&mut writer, value);
        if limit == expected.len() {
            result.unwrap();
            assert_eq!(&writer.bytes[..writer.used], expected.as_bytes());
        } else {
            assert_eq!(result, Err(fmt::Error), "capacity {limit}");
            assert!(writer.used <= limit);
        }
    }
}
