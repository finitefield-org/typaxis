//! Diagnostic-only inspection after declared font media attestation fails.
//! This never issues a receipt or changes the admission decision.
use super::{read_be_u16, read_be_u32};

pub const SUPPORTED_FONT_OUTLINES_NOTE: &str = "supported outlines: standalone TrueType glyf; TTC TrueType glyf with an explicit face index; standalone name-keyed CFF1. CID-keyed CFF1 is unsupported by sfnt-cff1/1. Inspect available faces with: typaxis inspect-font FONT";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontContainerFailureReason {
    MalformedContainer,
    InvalidFaceIndex,
    UnsupportedOutline,
    InvalidOutlineTables,
    InspectionLimit,
}

impl FontContainerFailureReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MalformedContainer => "malformed_font_container",
            Self::InvalidFaceIndex => "invalid_face_index",
            Self::UnsupportedOutline => "unsupported_outline",
            Self::InvalidOutlineTables => "invalid_outline_tables",
            Self::InspectionLimit => "font_diagnostic_inspection_limit",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontContainerFailure {
    pub reason: FontContainerFailureReason,
    pub requested_face_index: u32,
    /// Present directories, not faces proven to pass outline admission.
    pub face_count: Option<u32>,
    pub font_byte: Option<u64>,
    pub table_tag: Option<[u8; 4]>,
    pub selected_outline: Option<&'static str>,
}

impl FontContainerFailure {
    pub fn context_note(self) -> String {
        use std::fmt::Write;
        let phase = if self.reason == FontContainerFailureReason::InvalidFaceIndex {
            "face-selection"
        } else {
            "sfnt-directory"
        };
        let mut note = format!(
            "phase={phase}; reason={}; requested_face_index={}; embedding=not-checked",
            self.reason.as_str(),
            self.requested_face_index,
        );
        if let Some(count) = self.face_count {
            let _ = write!(note, "; face_count={count}; present_face_indexes=[");
            for index in 0..count.min(32) {
                if index != 0 {
                    note.push(',');
                }
                let _ = write!(note, "{index}");
            }
            let _ = write!(
                note,
                "]; truncated={}; face_admission=not-checked",
                count > 32
            );
        }
        if let Some(offset) = self.font_byte {
            let _ = write!(note, "; font_byte={offset}");
        }
        if let Some(tag) = self.table_tag {
            // The diagnostic constructor assigns only the literal tags below.
            let _ = write!(note, "; table={}", String::from_utf8_lossy(&tag));
        }
        if let Some(outline) = self.selected_outline {
            let _ = write!(note, "; selected_outline={outline}");
        }
        note
    }
}

fn directory_end(bytes: &[u8], start: usize) -> Option<usize> {
    let count = usize::from(read_be_u16(bytes, start.checked_add(4)?).ok()?);
    if count == 0 {
        return None;
    }
    let end = start.checked_add(12)?.checked_add(count.checked_mul(16)?)?;
    bytes.get(start..end)?;
    Some(end)
}

pub(super) fn diagnose_font_container(bytes: &[u8], face: u32) -> FontContainerFailure {
    use FontContainerFailureReason::*;
    let mut result = FontContainerFailure {
        reason: MalformedContainer,
        requested_face_index: face,
        face_count: None,
        font_byte: Some(0),
        table_tag: None,
        selected_outline: None,
    };
    let collection = bytes.get(..4) == Some(b"ttcf");
    let start = if collection {
        result.font_byte = Some(4);
        if !matches!(read_be_u32(bytes, 4), Ok(0x0001_0000 | 0x0002_0000)) {
            return result;
        }
        result.font_byte = Some(8);
        let Ok(count) = read_be_u32(bytes, 8) else {
            return result;
        };
        if count == 0 {
            return result;
        }
        if count > 4096 {
            result.reason = InspectionLimit;
            return result;
        }
        let end = 12 + count as usize * 4;
        if bytes.get(12..end).is_none() {
            return result;
        }
        // Prove that every listed index has a bounded directory. The 4,096
        // ceiling is diagnostic work only; it cannot reject an accepted font.
        let mut selected = None;
        for index in 0..count {
            let position = 12 + index as usize * 4;
            let Ok(offset) = read_be_u32(bytes, position) else {
                return result;
            };
            let Ok(offset) = usize::try_from(offset) else {
                return result;
            };
            result.font_byte = Some(position as u64);
            if directory_end(bytes, offset).is_none() {
                return result;
            }
            if index == face {
                selected = Some(offset);
            }
        }
        result.face_count = Some(count);
        let Some(selected) = selected else {
            result.reason = InvalidFaceIndex;
            result.font_byte = None;
            return result;
        };
        selected
    } else {
        if !matches!(bytes.get(..4), Some(b"OTTO" | b"\0\x01\0\0")) {
            return result;
        }
        if directory_end(bytes, 0).is_none() {
            return result;
        }
        result.face_count = Some(1);
        if face != 0 {
            result.reason = InvalidFaceIndex;
            result.font_byte = None;
            return result;
        }
        0
    };
    result.font_byte = Some(start as u64);
    match bytes.get(start..start + 4) {
        Some(b"OTTO") => {
            result.selected_outline = Some("cff");
            if collection {
                result.reason = UnsupportedOutline;
                return result;
            }
        }
        Some(b"\0\x01\0\0") => result.selected_outline = Some("truetype"),
        _ => {
            result.reason = UnsupportedOutline;
            return result;
        }
    }
    result.reason = InvalidOutlineTables;
    result.font_byte = None;
    if let Some(end) = directory_end(bytes, start) {
        for record in (start + 12..end).step_by(16) {
            if bytes.get(record..record + 4) == Some(b"CFF2") {
                result.reason = UnsupportedOutline;
                result.table_tag = Some(*b"CFF2");
                result.font_byte = Some(record as u64);
                result.selected_outline = Some("cff2");
                break;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collection(count: u32, cff: bool) -> Vec<u8> {
        let directory = 12 + count as usize * 4;
        let mut bytes = vec![0; directory + 28];
        bytes[..4].copy_from_slice(b"ttcf");
        bytes[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        bytes[8..12].copy_from_slice(&count.to_be_bytes());
        for i in 0..count as usize {
            bytes[12 + i * 4..16 + i * 4].copy_from_slice(&(directory as u32).to_be_bytes());
        }
        bytes[directory..directory + 4].copy_from_slice(if cff { b"OTTO" } else { b"\0\x01\0\0" });
        bytes[directory + 4..directory + 6].copy_from_slice(&1u16.to_be_bytes());
        bytes
    }

    #[test]
    fn index_notes_are_bounded_and_do_not_claim_admission() {
        let bytes = collection(40, false);
        let f = diagnose_font_container(&bytes, 40);
        assert_eq!(f.reason, FontContainerFailureReason::InvalidFaceIndex);
        assert_eq!(f.face_count, Some(40));
        let note = f.context_note();
        assert!(note.contains(",30,31]; truncated=true; face_admission=not-checked"));
        assert!(!note.contains(",32,"));
        assert_eq!(f.font_byte, None);
    }

    #[test]
    fn present_cff_face_is_unsupported_and_truncated_header_has_no_list() {
        let bytes = collection(2, true);
        let f = diagnose_font_container(&bytes, 1);
        assert_eq!(f.reason, FontContainerFailureReason::UnsupportedOutline);
        assert_eq!(f.face_count, Some(2));
        assert_eq!(f.selected_outline, Some("cff"));
        for end in 0..bytes.len() {
            assert_eq!(diagnose_font_container(&bytes[..end], 2).face_count, None);
        }
    }
}
