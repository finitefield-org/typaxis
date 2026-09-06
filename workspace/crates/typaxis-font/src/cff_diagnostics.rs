//! Bounded error context captured by the same CFF admission implementation.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontFailurePhase {
    FaceSelection,
    SfntDirectory,
    TableDecode,
    Cmap,
    CffIndex,
    CffTopDict,
    CffPrivateDict,
    EmbeddingPermission,
    Charstring,
    Subset,
}
impl FontFailurePhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FaceSelection => "face-selection",
            Self::SfntDirectory => "sfnt-directory",
            Self::TableDecode => "table-decode",
            Self::Cmap => "cmap",
            Self::CffIndex => "cff-index",
            Self::CffTopDict => "cff-top-dict",
            Self::CffPrivateDict => "cff-private-dict",
            Self::EmbeddingPermission => "embedding-permission",
            Self::Charstring => "charstring",
            Self::Subset => "subset",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontFailureReason {
    MalformedFont,
    InvalidFaceIndex,
    UnsupportedTable,
    MissingTable,
    InvalidTableOrder,
    InvalidTableRange,
    ChecksumMismatch,
    InvalidTableCount,
    InvalidTable,
    UnsupportedCmapFormat,
    UnsupportedCffOperator,
    RestrictedEmbedding,
    BudgetExceeded,
    Invariant,
}
impl FontFailureReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MalformedFont => "malformed_font",
            Self::InvalidFaceIndex => "invalid_face_index",
            Self::UnsupportedTable => "unsupported_table",
            Self::MissingTable => "missing_table",
            Self::InvalidTableOrder => "duplicate_or_unsorted_table",
            Self::InvalidTableRange => "invalid_table_range",
            Self::ChecksumMismatch => "checksum_mismatch",
            Self::InvalidTableCount => "invalid_table_count",
            Self::InvalidTable => "invalid_table",
            Self::UnsupportedCmapFormat => "unsupported_cmap_format",
            Self::UnsupportedCffOperator => "unsupported_cff_operator",
            Self::RestrictedEmbedding => "restricted_embedding",
            Self::BudgetExceeded => "budget_exceeded",
            Self::Invariant => "receipt_invariant",
        }
    }
    pub const fn class(self) -> &'static str {
        match self {
            Self::UnsupportedTable | Self::UnsupportedCmapFormat | Self::UnsupportedCffOperator => {
                "unsupported"
            }
            Self::RestrictedEmbedding => "embedding-permission",
            Self::BudgetExceeded => "resource-budget",
            Self::Invariant => "internal",
            _ => "malformed-or-invalid-input",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontEmbeddingStatus {
    NotChecked,
    Allowed(u16),
    Denied(u16),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontFailureContext {
    pub phase: FontFailurePhase,
    pub reason: FontFailureReason,
    pub table_tag: Option<[u8; 4]>,
    pub file_offset: Option<u64>,
    pub table_offset: Option<u64>,
    /// True only when the position names a specific validated field/token.
    pub position_is_exact: bool,
    pub requested_face_index: u32,
    pub embedding: FontEmbeddingStatus,
    pub operator: Option<u16>,
    pub cmap_format: Option<u16>,
    pub gid: Option<u32>,
    pub fd: Option<u16>,
    pub limit: Option<u64>,
    pub observed: Option<u64>,
    // Base belongs to the validated table currently being parsed. This is not
    // itself an offending-byte position and is never emitted as one.
    table_base: Option<u64>,
}
impl FontFailureContext {
    pub(super) const fn new(face_index: u32) -> Self {
        Self {
            phase: FontFailurePhase::FaceSelection,
            reason: FontFailureReason::InvalidFaceIndex,
            table_tag: None,
            file_offset: None,
            table_offset: None,
            position_is_exact: false,
            requested_face_index: face_index,
            embedding: FontEmbeddingStatus::NotChecked,
            operator: None,
            cmap_format: None,
            gid: None,
            fd: None,
            limit: None,
            observed: None,
            table_base: None,
        }
    }
    pub(super) fn table(&mut self, tag: [u8; 4], records: &[TableRecord], phase: FontFailurePhase) {
        self.phase = phase;
        self.reason = FontFailureReason::InvalidTable;
        self.table_tag = Some(tag);
        self.table_base = records
            .iter()
            .find(|r| r.tag == tag)
            .map(|r| r.offset as u64);
        self.clear_position();
        self.operator = None;
        self.cmap_format = None;
        self.limit = None;
        self.observed = None;
    }
    pub(super) fn clear_position(&mut self) {
        self.file_offset = None;
        self.table_offset = None;
        self.position_is_exact = false;
    }
    pub(super) fn field(&mut self, relative: usize) {
        self.at(relative);
        self.position_is_exact = true;
    }
    pub(super) fn at(&mut self, relative: usize) {
        self.position_is_exact = false;
        self.table_offset = Some(relative as u64);
        self.file_offset = self
            .table_base
            .and_then(|base| base.checked_add(relative as u64));
    }
    pub(super) fn directory(
        &mut self,
        tag: Option<[u8; 4]>,
        offset: Option<usize>,
        reason: FontFailureReason,
    ) {
        self.phase = FontFailurePhase::SfntDirectory;
        self.table_tag = tag;
        self.file_offset = offset.map(|v| v as u64);
        self.table_offset = None;
        self.table_base = None;
        self.reason = reason;
        self.position_is_exact = offset.is_some();
    }
    pub(super) fn permission(&mut self, bytes: &[u8]) {
        self.embedding = match parse_os2(bytes) {
            Ok(os2) => {
                if embedding_permission(os2.fs_type).is_ok() {
                    FontEmbeddingStatus::Allowed(os2.fs_type)
                } else {
                    FontEmbeddingStatus::Denied(os2.fs_type)
                }
            }
            Err(_) => FontEmbeddingStatus::NotChecked,
        };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cff1Failure {
    pub kind: Cff1Error,
    pub context: FontFailureContext,
}
impl Cff1Failure {
    pub fn context_note(self) -> String {
        use std::fmt::Write;
        let c = self.context;
        let mut note = format!(
            "phase={}; reason={}; class={}; requested_face_index={}",
            c.phase.as_str(),
            c.reason.as_str(),
            c.reason.class(),
            c.requested_face_index
        );
        if let Some(tag) = c.table_tag {
            note.push_str("; table=");
            for byte in tag {
                if byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'/') {
                    note.push(char::from(byte));
                } else {
                    let _ = write!(note, "\\x{byte:02X}");
                }
            }
        }
        if c.file_offset.is_some() || c.table_offset.is_some() {
            note.push_str(if c.position_is_exact {
                "; offset_kind=field"
            } else {
                "; offset_kind=context-start"
            });
        }
        if let Some(offset) = c.file_offset {
            let _ = write!(note, "; font_byte={offset}");
        }
        if let Some(offset) = c.table_offset {
            let _ = write!(note, "; table_byte={offset}");
        }
        if let Some(op) = c.operator {
            let _ = write!(note, "; cff_operator=0x{op:04X}");
        }
        if let Some(format) = c.cmap_format {
            let _ = write!(note, "; cmap_format={format}");
        }
        if let Some(gid) = c.gid {
            let _ = write!(note, "; gid={gid}");
        }
        if let Some(fd) = c.fd {
            let _ = write!(note, "; fd={fd}");
        }
        if let Some(limit) = c.limit {
            let _ = write!(note, "; limit={limit}");
        }
        if let Some(observed) = c.observed {
            let _ = write!(note, "; observed={observed}");
        }
        match c.embedding {
            FontEmbeddingStatus::NotChecked => note.push_str("; embedding=not-checked"),
            FontEmbeddingStatus::Allowed(fs) => {
                let _ = write!(note, "; embedding=allowed; fs_type=0x{fs:04X}");
            }
            FontEmbeddingStatus::Denied(fs) => {
                let _ = write!(note, "; embedding=denied; fs_type=0x{fs:04X}");
            }
        }
        note
    }
}
impl std::fmt::Display for Cff1Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cff1 {}: {}",
            self.context.reason.as_str(),
            self.context_note()
        )
    }
}
impl std::error::Error for Cff1Failure {}

pub fn admit_sfnt_cff1_detailed(
    source: &[u8],
    face_index: u32,
    limits: &M4EffectiveResourceLimits,
) -> Result<Cff1Admission, Cff1Failure> {
    let mut context = FontFailureContext::new(face_index);
    admit_sfnt_cff1_inner(source, face_index, limits, &mut context).map_err(|kind| {
        if matches!(
            kind,
            Cff1Error::TableLimit
                | Cff1Error::GlyphLimit
                | Cff1Error::SubroutineLimit
                | Cff1Error::CharstringOperationLimit
                | Cff1Error::OutlineSegmentLimit
                | Cff1Error::SubsetByteLimit
        ) && context.reason != FontFailureReason::InvalidTableCount
        {
            context.reason = FontFailureReason::BudgetExceeded;
        }
        Cff1Failure { kind, context }
    })
}
