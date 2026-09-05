//! Fixed-size, resource-local SVG failure context. Never holds input borrows.
use crate::{ResourceAdmissionError, SafeVectorFailureReason};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceByteSpan {
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticToken {
    bytes: [u8; 80],
    len: u8,
    pub truncated: bool,
}

impl DiagnosticToken {
    pub fn new(value: &str) -> Self {
        let mut length = value.len().min(80);
        while !value.is_char_boundary(length) {
            length -= 1;
        }
        let mut bytes = [0; 80];
        bytes[..length].copy_from_slice(&value.as_bytes()[..length]);
        Self {
            bytes,
            len: length as u8,
            truncated: length < value.len(),
        }
    }
    pub fn escaped(self) -> String {
        // The bytes always end on a UTF-8 boundary. Escape even ASCII controls;
        // diagnostic text cannot inject a second note or terminal sequence.
        let text = std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("");
        text.chars().flat_map(char::escape_default).collect()
    }
    fn percent_encoded(self) -> String {
        self.bytes[..usize::from(self.len)]
            .iter()
            .map(|byte| format!("%{byte:02X}"))
            .collect()
    }
    fn note_name(self) -> String {
        let name = std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("");
        if matches!(
            name,
            "svg"
                | "g"
                | "defs"
                | "clipPath"
                | "path"
                | "rect"
                | "circle"
                | "ellipse"
                | "line"
                | "polyline"
                | "polygon"
                | "d"
                | "x"
                | "y"
                | "x1"
                | "y1"
                | "x2"
                | "y2"
                | "cx"
                | "cy"
                | "r"
                | "rx"
                | "ry"
                | "width"
                | "height"
                | "viewBox"
                | "xmlns"
                | "id"
                | "fill"
                | "stroke"
                | "fill-rule"
                | "fill-opacity"
                | "stroke-opacity"
                | "stroke-width"
                | "stroke-linecap"
                | "stroke-linejoin"
                | "stroke-miterlimit"
                | "clip-path"
                | "points"
                | "transform"
        ) {
            name.to_owned()
        } else {
            self.percent_encoded()
        }
    }
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafeSvg2DetailReason {
    UnexpectedToken,
    UnexpectedEndOfInput,
    InvalidUtf8,
    MissingAttribute,
    DuplicateAttribute,
    UnsupportedAttribute,
    UnsupportedCommand,
    UnsupportedPathSyntax,
    InvalidNumber,
    CoordinateOutOfRange,
    WrongParameterCount,
    AspectRatioMismatch,
    UnrepresentableScale,
    InvalidGeometry,
    BudgetExceeded,
    Category(SafeVectorFailureReason),
}
impl SafeSvg2DetailReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnexpectedToken => "unexpected_token",
            Self::UnexpectedEndOfInput => "unexpected_end_of_input",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::MissingAttribute => "missing_attribute",
            Self::DuplicateAttribute => "duplicate_attribute",
            Self::UnsupportedAttribute => "unsupported_attribute",
            Self::UnsupportedCommand => "unsupported_command",
            Self::UnsupportedPathSyntax => "unsupported_path_syntax",
            Self::InvalidNumber => "invalid_number",
            Self::CoordinateOutOfRange => "coordinate_out_of_range",
            Self::WrongParameterCount => "wrong_parameter_count",
            Self::AspectRatioMismatch => "aspect_ratio_mismatch",
            Self::UnrepresentableScale => "unrepresentable_scale",
            Self::InvalidGeometry => "invalid_geometry",
            Self::BudgetExceeded => "budget_exceeded",
            Self::Category(reason) => reason.as_str(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VectorBudgetKind {
    Nodes,
    StoredSegments,
    ClipReplay,
    Nesting,
    Allocation,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetScope {
    ResourceLocal,
    DocumentTotal,
}
impl VectorBudgetKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nodes => "nodes",
            Self::StoredSegments => "stored_segment",
            Self::ClipReplay => "clip_replay",
            Self::Nesting => "nesting",
            Self::Allocation => "allocation",
        }
    }
}
impl BudgetScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ResourceLocal => "resource-local",
            Self::DocumentTotal => "document-total",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VectorBudgetFailure {
    pub kind: VectorBudgetKind,
    pub scope: BudgetScope,
    pub limit: u64,
    pub observed: u64,
    pub used_before_resource: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SafeSvg2Failure {
    pub reason: SafeSvg2DetailReason,
    pub span: Option<ResourceByteSpan>,
    pub line: Option<u32>,
    pub byte_column: Option<u64>,
    pub element_index: Option<u32>,
    pub element: DiagnosticToken,
    pub path_index: Option<u32>,
    pub subpath_index: Option<u32>,
    pub segment_index: Option<u64>,
    pub attribute: DiagnosticToken,
    pub token: DiagnosticToken,
    pub budget: Option<VectorBudgetFailure>,
}
impl SafeSvg2Failure {
    pub fn new(reason: SafeSvg2DetailReason) -> Self {
        Self {
            reason,
            span: None,
            line: None,
            byte_column: None,
            element_index: None,
            element: DiagnosticToken::new(""),
            path_index: None,
            subpath_index: None,
            segment_index: None,
            attribute: DiagnosticToken::new(""),
            token: DiagnosticToken::new(""),
            budget: None,
        }
    }
    pub(crate) fn budget(
        kind: VectorBudgetKind,
        limit: u64,
        observed: u64,
    ) -> ResourceAdmissionError {
        let mut failure = Self::new(SafeSvg2DetailReason::BudgetExceeded);
        failure.budget = Some(VectorBudgetFailure {
            kind,
            scope: BudgetScope::ResourceLocal,
            limit,
            observed,
            used_before_resource: 0,
        });
        ResourceAdmissionError::SafeSvg2Detailed(failure)
    }
    pub fn at(mut self, start: usize, end: usize) -> Self {
        self.span = Some(ResourceByteSpan {
            start: start as u64,
            end: end as u64,
        });
        self
    }
    pub fn attribute(mut self, name: &str, value: &str) -> Self {
        self.attribute = DiagnosticToken::new(name);
        self.token = DiagnosticToken::new(value);
        self
    }
    pub(crate) fn locate(mut self, bytes: &[u8]) -> Self {
        if self.reason != SafeSvg2DetailReason::InvalidUtf8 {
            if let Some(span) = self.span {
                if let Some(prefix) = bytes.get(..span.start as usize) {
                    self.line =
                        u32::try_from(prefix.iter().filter(|byte| **byte == b'\n').count() + 1)
                            .ok();
                    self.byte_column = Some(
                        (prefix.len()
                            - prefix
                                .iter()
                                .rposition(|byte| *byte == b'\n')
                                .map_or(0, |p| p + 1)
                            + 1) as u64,
                    );
                }
            }
        }
        self
    }
    pub fn context_note(self) -> String {
        let mut values = Vec::new();
        if !self.element.is_empty() {
            values.push(format!("element={}", self.element.note_name()));
        }
        if let Some(index) = self.element_index {
            values.push(format!("element_number={}", u64::from(index) + 1));
        }
        if let Some(index) = self.path_index {
            values.push(format!("path={}", u64::from(index) + 1));
        }
        if let Some(index) = self.subpath_index {
            values.push(format!("subpath={}", u64::from(index) + 1));
        }
        if let Some(index) = self.segment_index {
            values.push(format!("segment={}", index + 1));
        }
        if !self.attribute.is_empty() {
            values.push(format!("attribute={}", self.attribute.note_name()));
        }
        if !self.token.is_empty() {
            // The canonical diagnostic note owner forbids raw input snippets.
            // Percent-encode token bytes to preserve exact bounded evidence
            // without allowing quotes, control bytes, paths or OS-like text.
            let encoded = self.token.percent_encoded();
            values.push(format!(
                "token_percent={encoded}; token_truncated={}",
                self.token.truncated
            ));
        }
        if let Some(span) = self.span {
            values.push(format!("svg_byte={}..{}", span.start, span.end));
        }
        if let Some(line) = self.line {
            values.push(format!("line={line}"));
        }
        if let Some(column) = self.byte_column {
            values.push(format!("byte_column={column}"));
        }
        values.join("; ")
    }
    pub(crate) fn from_error(error: ResourceAdmissionError) -> Self {
        match error {
            ResourceAdmissionError::SafeSvg2Detailed(failure) => failure,
            ResourceAdmissionError::InvalidSafeVectorV2(SafeVectorFailureReason::MalformedSvg)
            | ResourceAdmissionError::InvalidSafeVector => {
                Self::new(SafeSvg2DetailReason::UnexpectedToken)
            }
            ResourceAdmissionError::InvalidSafeVectorV2(reason) => {
                Self::new(SafeSvg2DetailReason::Category(reason))
            }
            _ => Self::new(SafeSvg2DetailReason::BudgetExceeded),
        }
    }
    #[cfg(test)]
    pub(crate) fn legacy(self) -> ResourceAdmissionError {
        if let Some(budget) = self.budget {
            return match budget.kind {
                VectorBudgetKind::Nodes => ResourceAdmissionError::VectorNodeLimit,
                VectorBudgetKind::StoredSegments | VectorBudgetKind::ClipReplay => {
                    ResourceAdmissionError::VectorPathSegmentLimit
                }
                VectorBudgetKind::Nesting => ResourceAdmissionError::VectorNestingLimit,
                VectorBudgetKind::Allocation => ResourceAdmissionError::DecodedImageLimit,
            };
        }
        ResourceAdmissionError::InvalidSafeVectorV2(match self.reason {
            SafeSvg2DetailReason::Category(reason) => reason,
            SafeSvg2DetailReason::UnsupportedAttribute
            | SafeSvg2DetailReason::UnsupportedCommand => {
                SafeVectorFailureReason::UnsupportedFeature
            }
            _ => SafeVectorFailureReason::MalformedSvg,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_are_bounded_utf8_and_safe_for_canonical_notes() {
        let value = format!("{}\" /private/tmp/file raw_os_error\n", "数".repeat(40));
        let token = DiagnosticToken::new(&value);
        assert!(token.truncated);
        assert!(token.escaped().len() <= 480);
        let failure = SafeSvg2Failure::new(SafeSvg2DetailReason::InvalidNumber)
            .attribute("d", &value)
            .at(5, 8);
        let note = failure.context_note();
        typaxis_diagnostics::DiagnosticNote::new(note.clone()).unwrap();
        assert!(note.contains("token_percent="));
        assert!(!note.contains("/private/tmp"));
        for value in ["1.25", "A", "errno", "\"source:\"", "\n\t\u{1b}"] {
            let failure =
                SafeSvg2Failure::new(SafeSvg2DetailReason::InvalidNumber).attribute("d", value);
            typaxis_diagnostics::DiagnosticNote::new(failure.context_note()).unwrap();
        }
    }
}
