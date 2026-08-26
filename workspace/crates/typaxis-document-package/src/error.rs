use std::fmt;
use typaxis_core::JsonPointer;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonPreflightLimitError {
    DocumentPackageBytesOutOfRange {
        requested: u64,
        minimum: u64,
        maximum: u64,
    },
    JsonNestingDepthOutOfRange {
        requested: u16,
        minimum: u16,
        maximum: u16,
    },
}

impl fmt::Display for JsonPreflightLimitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DocumentPackageBytesOutOfRange {
                requested,
                minimum,
                maximum,
            } => write!(
                formatter,
                "DocumentPackage byte limit {requested} is outside {minimum}..={maximum}"
            ),
            Self::JsonNestingDepthOutOfRange {
                requested,
                minimum,
                maximum,
            } => write!(
                formatter,
                "JSON nesting limit {requested} is outside {minimum}..={maximum}"
            ),
        }
    }
}

impl std::error::Error for JsonPreflightLimitError {}

/// Stable internal classes later mapped to the public diagnostic namespace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonPreflightErrorClass {
    PackageEnvelope,
    JsonSyntax,
    PackageByteLimit,
    JsonNestingDepthLimit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonContainerKind {
    Object,
    Array,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JsonContainerContext {
    kind: JsonContainerKind,
    start_byte: u64,
}

impl JsonContainerContext {
    pub(crate) const fn new(kind: JsonContainerKind, start_byte: u64) -> Self {
        Self { kind, start_byte }
    }

    pub const fn kind(self) -> JsonContainerKind {
        self.kind
    }

    pub const fn start_byte(self) -> u64 {
        self.start_byte
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonNumberKind {
    Integer,
    Fraction,
    Exponent,
    FractionAndExponent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonTokenKind {
    Object,
    Array,
    String,
    Number(JsonNumberKind),
    True,
    False,
    Null,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JsonTokenMetadata {
    kind: JsonTokenKind,
    start_byte: u64,
    end_byte: Option<u64>,
}

impl JsonTokenMetadata {
    pub(crate) const fn incomplete(kind: JsonTokenKind, start_byte: u64) -> Self {
        Self {
            kind,
            start_byte,
            end_byte: None,
        }
    }

    pub(crate) const fn complete(kind: JsonTokenKind, start_byte: u64, end_byte: u64) -> Self {
        Self {
            kind,
            start_byte,
            end_byte: Some(end_byte),
        }
    }

    pub const fn kind(self) -> JsonTokenKind {
        self.kind
    }

    pub const fn start_byte(self) -> u64 {
        self.start_byte
    }

    pub const fn end_byte(self) -> Option<u64> {
        self.end_byte
    }

    pub const fn is_complete(self) -> bool {
        self.end_byte.is_some()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonPreflightLocation {
    byte_offset: u64,
    json_pointer: JsonPointer,
    container: Option<JsonContainerContext>,
    member_start_byte: Option<u64>,
    member_name: Option<String>,
    token: Option<JsonTokenMetadata>,
}

impl JsonPreflightLocation {
    pub(crate) const fn new(
        byte_offset: u64,
        json_pointer: JsonPointer,
        container: Option<JsonContainerContext>,
        member_start_byte: Option<u64>,
        member_name: Option<String>,
        token: Option<JsonTokenMetadata>,
    ) -> Self {
        Self {
            byte_offset,
            json_pointer,
            container,
            member_start_byte,
            member_name,
            token,
        }
    }

    pub const fn byte_offset(&self) -> u64 {
        self.byte_offset
    }

    pub const fn json_pointer(&self) -> &JsonPointer {
        &self.json_pointer
    }

    pub const fn container(&self) -> Option<JsonContainerContext> {
        self.container
    }

    pub const fn member_start_byte(&self) -> Option<u64> {
        self.member_start_byte
    }

    pub fn member_name(&self) -> Option<&str> {
        self.member_name.as_deref()
    }

    pub const fn token(&self) -> Option<JsonTokenMetadata> {
        self.token
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonPreflightErrorKind {
    PackageBytesExceeded { limit: u64, actual: u64 },
    JsonNestingDepthExceeded { limit: u16, attempted: u16 },
    InvalidUtf8,
    Utf8Bom,
    RawNul,
    RootMustBeObject,
    TrailingToken,
    UnexpectedEnd,
    UnexpectedToken,
    UnescapedControlCharacter,
    UnterminatedString,
    InvalidStringEscape,
    InvalidUnicodeEscape,
    InvalidUnicodeSurrogate,
    InvalidNumber,
    InvalidLiteral,
    DuplicateObjectMember,
}

impl JsonPreflightErrorKind {
    pub const fn class(self) -> JsonPreflightErrorClass {
        match self {
            Self::PackageBytesExceeded { .. } => JsonPreflightErrorClass::PackageByteLimit,
            Self::JsonNestingDepthExceeded { .. } => JsonPreflightErrorClass::JsonNestingDepthLimit,
            Self::InvalidUtf8
            | Self::Utf8Bom
            | Self::RawNul
            | Self::RootMustBeObject
            | Self::TrailingToken => JsonPreflightErrorClass::PackageEnvelope,
            Self::UnexpectedEnd
            | Self::UnexpectedToken
            | Self::UnescapedControlCharacter
            | Self::UnterminatedString
            | Self::InvalidStringEscape
            | Self::InvalidUnicodeEscape
            | Self::InvalidUnicodeSurrogate
            | Self::InvalidNumber
            | Self::InvalidLiteral
            | Self::DuplicateObjectMember => JsonPreflightErrorClass::JsonSyntax,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonPreflightError {
    kind: JsonPreflightErrorKind,
    location: Box<JsonPreflightLocation>,
}

impl JsonPreflightError {
    pub(crate) fn new(kind: JsonPreflightErrorKind, location: JsonPreflightLocation) -> Self {
        Self {
            kind,
            location: Box::new(location),
        }
    }

    pub const fn kind(&self) -> JsonPreflightErrorKind {
        self.kind
    }

    pub const fn class(&self) -> JsonPreflightErrorClass {
        self.kind.class()
    }

    pub fn location(&self) -> &JsonPreflightLocation {
        &self.location
    }
}

impl fmt::Display for JsonPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let offset = self.location.byte_offset();
        match self.kind {
            JsonPreflightErrorKind::PackageBytesExceeded { limit, actual } => write!(
                formatter,
                "DocumentPackage has {actual} bytes, exceeding limit {limit}"
            ),
            JsonPreflightErrorKind::JsonNestingDepthExceeded { limit, attempted } => write!(
                formatter,
                "JSON container depth {attempted} exceeds limit {limit} at byte {offset}"
            ),
            JsonPreflightErrorKind::InvalidUtf8 => {
                write!(formatter, "DocumentPackage is not UTF-8 at byte {offset}")
            }
            JsonPreflightErrorKind::Utf8Bom => {
                formatter.write_str("DocumentPackage starts with a UTF-8 BOM")
            }
            JsonPreflightErrorKind::RawNul => {
                write!(
                    formatter,
                    "DocumentPackage contains raw NUL at byte {offset}"
                )
            }
            JsonPreflightErrorKind::RootMustBeObject => {
                write!(
                    formatter,
                    "DocumentPackage root is not an object at byte {offset}"
                )
            }
            JsonPreflightErrorKind::TrailingToken => {
                write!(
                    formatter,
                    "DocumentPackage has a trailing token at byte {offset}"
                )
            }
            JsonPreflightErrorKind::DuplicateObjectMember => {
                write!(
                    formatter,
                    "JSON object member is duplicated at byte {offset}"
                )
            }
            _ => write!(formatter, "JSON grammar is invalid at byte {offset}"),
        }
    }
}

impl std::error::Error for JsonPreflightError {}
