#![forbid(unsafe_code)]

use core::fmt;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

pub const PRODUCT_NAME: &str = "typaxis";
pub const REGISTERED_UNICODE_VERSION: &str = "16.0.0";
pub const REGISTERED_JAPANESE_LINE_BREAK_VERSION: &str = "typaxis-jlreq-horizontal/1.0.0";
pub const CONTRACT: &str = "typaxis.contract/1.0";
pub const COORDINATE_UNIT: &str = "pdf_point_1_65536";
pub const UNITS_PER_PDF_POINT: i64 = 65_536;
pub const JSON_SAFE_INTEGER_MAX: i64 = 9_007_199_254_740_991;
pub const MAX_BIDI_LEVEL: u8 = 125;
pub const DEFAULT_MAX_URI_BYTES: usize = 8_192;
pub const DEFAULT_ALLOWED_URI_SCHEMES: &[&str] = &["http", "https", "mailto", "tel"];

/// Dependency-free SHA-256 used at admission boundaries.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut hash = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in padded.chunks_exact(64) {
        let mut words = [0u32; 64];
        for (index, word) in words[..16].iter_mut().enumerate() {
            let start = index * 4;
            *word = u32::from_be_bytes(chunk[start..start + 4].try_into().unwrap());
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = hash;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in hash.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
    let mut output = [0u8; 32];
    for (chunk, word) in output.chunks_exact_mut(4).zip(hash) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    output
}

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            pub const fn new(value: u32) -> Self {
                Self(value)
            }
            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

id_type!(SourceId);
id_type!(TextBufferId);
id_type!(NodeId);
id_type!(FontFaceId);
id_type!(FontInstanceId);
id_type!(GlyphRunId);
id_type!(DisplayGlyphRunId);
id_type!(ImageResourceId);
id_type!(GeneratedTextBufferId);
id_type!(DisplayTextBufferId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentifierError {
    Empty,
    InvalidStart,
    InvalidCharacter,
}

macro_rules! string_id_type {
    ($name:ident, $is_valid:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
                let value = value.into();
                validate_string_identifier(&value, $is_valid)?;
                Ok(Self(value))
            }
            pub fn is_valid(value: &str) -> bool {
                validate_string_identifier(value, $is_valid).is_ok()
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

fn validate_string_identifier(
    value: &str,
    valid_tail: fn(u8) -> bool,
) -> Result<(), IdentifierError> {
    let mut bytes = value.bytes();
    let first = bytes.next().ok_or(IdentifierError::Empty)?;
    if !first.is_ascii_alphabetic() && first != b'_' {
        return Err(IdentifierError::InvalidStart);
    }
    if bytes.all(valid_tail) {
        Ok(())
    } else {
        Err(IdentifierError::InvalidCharacter)
    }
}

fn valid_style_tail(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

fn valid_anchor_tail(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'-')
}

string_id_type!(AnchorId, valid_anchor_tail);
string_id_type!(FootnoteId, valid_anchor_tail);
string_id_type!(MasterId, valid_style_tail);
string_id_type!(StyleId, valid_style_tail);
string_id_type!(PageName, valid_style_tail);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Utf8ByteOffset(u32);
impl Utf8ByteOffset {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A half-open UTF-8 byte range local to a single already-selected buffer.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Utf8ByteRange {
    start_byte: Utf8ByteOffset,
    end_byte: Utf8ByteOffset,
}
impl Utf8ByteRange {
    pub const fn new(start_byte: Utf8ByteOffset, end_byte: Utf8ByteOffset) -> Option<Self> {
        if start_byte.get() <= end_byte.get() {
            Some(Self {
                start_byte,
                end_byte,
            })
        } else {
            None
        }
    }
    pub const fn start_byte(self) -> Utf8ByteOffset {
        self.start_byte
    }
    pub const fn end_byte(self) -> Utf8ByteOffset {
        self.end_byte
    }
    pub const fn len(self) -> u32 {
        self.end_byte.get() - self.start_byte.get()
    }
    pub const fn is_empty(self) -> bool {
        self.start_byte.get() == self.end_byte.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    source_id: SourceId,
    range: Utf8ByteRange,
}
impl SourceSpan {
    pub const fn new(
        source_id: SourceId,
        start_byte: Utf8ByteOffset,
        end_byte: Utf8ByteOffset,
    ) -> Option<Self> {
        match Utf8ByteRange::new(start_byte, end_byte) {
            Some(range) => Some(Self { source_id, range }),
            None => None,
        }
    }
    pub const fn source_id(self) -> SourceId {
        self.source_id
    }
    pub const fn range(self) -> Utf8ByteRange {
        self.range
    }
    pub const fn start_byte(self) -> Utf8ByteOffset {
        self.range.start_byte()
    }
    pub const fn end_byte(self) -> Utf8ByteOffset {
        self.range.end_byte()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextSpan {
    text_id: TextBufferId,
    range: Utf8ByteRange,
}

/// A span in generated layout text. It cannot name a parsed TextStore buffer.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GeneratedTextSpan {
    text_id: GeneratedTextBufferId,
    range: Utf8ByteRange,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GenerationKind {
    PageReference,
    Counter,
    ListMarker,
    FootnoteMarker,
    Discretionary,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GeneratedBufferKey {
    owner: NodeId,
    generation_kind: GenerationKind,
    owner_local_ordinal: u32,
}
impl GeneratedBufferKey {
    pub const fn new(
        owner: NodeId,
        generation_kind: GenerationKind,
        owner_local_ordinal: u32,
    ) -> Self {
        Self {
            owner,
            generation_kind,
            owner_local_ordinal,
        }
    }
    pub const fn owner(self) -> NodeId {
        self.owner
    }
    pub const fn generation_kind(self) -> GenerationKind {
        self.generation_kind
    }
    pub const fn owner_local_ordinal(self) -> u32 {
        self.owner_local_ordinal
    }
}

/// A span in the dense text-buffer namespace of one DisplayDocument artifact.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DisplayTextSpan {
    text_id: DisplayTextBufferId,
    range: Utf8ByteRange,
}
impl DisplayTextSpan {
    pub const fn new(
        text_id: DisplayTextBufferId,
        start_byte: Utf8ByteOffset,
        end_byte: Utf8ByteOffset,
    ) -> Option<Self> {
        match Utf8ByteRange::new(start_byte, end_byte) {
            Some(range) => Some(Self { text_id, range }),
            None => None,
        }
    }
    pub const fn text_id(self) -> DisplayTextBufferId {
        self.text_id
    }
    pub const fn range(self) -> Utf8ByteRange {
        self.range
    }
}
impl GeneratedTextSpan {
    pub const fn new(
        text_id: GeneratedTextBufferId,
        start_byte: Utf8ByteOffset,
        end_byte: Utf8ByteOffset,
    ) -> Option<Self> {
        match Utf8ByteRange::new(start_byte, end_byte) {
            Some(range) => Some(Self { text_id, range }),
            None => None,
        }
    }
    pub const fn text_id(self) -> GeneratedTextBufferId {
        self.text_id
    }
    pub const fn range(self) -> Utf8ByteRange {
        self.range
    }
}
impl TextSpan {
    pub const fn new(
        text_id: TextBufferId,
        start_byte: Utf8ByteOffset,
        end_byte: Utf8ByteOffset,
    ) -> Option<Self> {
        match Utf8ByteRange::new(start_byte, end_byte) {
            Some(range) => Some(Self { text_id, range }),
            None => None,
        }
    }
    pub const fn text_id(self) -> TextBufferId {
        self.text_id
    }
    pub const fn range(self) -> Utf8ByteRange {
        self.range
    }
    pub const fn start_byte(self) -> Utf8ByteOffset {
        self.range.start_byte()
    }
    pub const fn end_byte(self) -> Utf8ByteOffset {
        self.range.end_byte()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextOffset {
    pub text_id: TextBufferId,
    pub byte: Utf8ByteOffset,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BidiLevel(u8);
impl BidiLevel {
    pub const fn new(value: u8) -> Option<Self> {
        if value <= MAX_BIDI_LEVEL {
            Some(Self(value))
        } else {
            None
        }
    }
    pub const fn get(self) -> u8 {
        self.0
    }
    pub const fn is_rtl(self) -> bool {
        self.0 % 2 == 1
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OpenTypeTag([u8; 4]);
impl OpenTypeTag {
    pub fn new(bytes: [u8; 4]) -> Option<Self> {
        if bytes.iter().all(|byte| (0x20..=0x7e).contains(byte)) {
            Some(Self(bytes))
        } else {
            None
        }
    }
    pub const fn bytes(self) -> [u8; 4] {
        self.0
    }
}

/// Signed fixed-point length in units of 1/65536 PDF point.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Length(i64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LengthError {
    ZeroDenominator,
    ArithmeticOverflow,
    OutOfRange,
}
impl Length {
    pub const ZERO: Self = Self(0);
    pub const fn from_raw(raw: i64) -> Option<Self> {
        if raw >= -JSON_SAFE_INTEGER_MAX && raw <= JSON_SAFE_INTEGER_MAX {
            Some(Self(raw))
        } else {
            None
        }
    }
    pub const fn raw(self) -> i64 {
        self.0
    }
    pub fn from_rational_pdf_points(
        numerator: i128,
        denominator: i128,
    ) -> Result<Self, LengthError> {
        if denominator == 0 {
            return Err(LengthError::ZeroDenominator);
        }
        let (numerator, denominator) = if denominator < 0 {
            (
                numerator
                    .checked_neg()
                    .ok_or(LengthError::ArithmeticOverflow)?,
                denominator
                    .checked_neg()
                    .ok_or(LengthError::ArithmeticOverflow)?,
            )
        } else {
            (numerator, denominator)
        };
        let scaled = numerator
            .checked_mul(i128::from(UNITS_PER_PDF_POINT))
            .ok_or(LengthError::ArithmeticOverflow)?;
        let rounded = round_ratio_ties_even(scaled, denominator)?;
        let raw = i64::try_from(rounded).map_err(|_| LengthError::OutOfRange)?;
        Self::from_raw(raw).ok_or(LengthError::OutOfRange)
    }
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).and_then(Self::from_raw)
    }
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).and_then(Self::from_raw)
    }
}
fn round_ratio_ties_even(numerator: i128, denominator: i128) -> Result<i128, LengthError> {
    debug_assert!(denominator > 0);
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder == 0 {
        return Ok(quotient);
    }
    let doubled = remainder
        .unsigned_abs()
        .checked_mul(2)
        .ok_or(LengthError::ArithmeticOverflow)?;
    let denominator = u128::try_from(denominator).map_err(|_| LengthError::ArithmeticOverflow)?;
    let step = if remainder.is_positive() { 1 } else { -1 };
    if doubled < denominator {
        Ok(quotient)
    } else if doubled > denominator {
        quotient
            .checked_add(step)
            .ok_or(LengthError::ArithmeticOverflow)
    } else if quotient % 2 == 0 {
        Ok(quotient)
    } else {
        quotient
            .checked_add(step)
            .ok_or(LengthError::ArithmeticOverflow)
    }
}
impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/65536pdfpt", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NonNegativeLength(Length);
impl NonNegativeLength {
    pub const ZERO: Self = Self(Length::ZERO);
    pub const fn new(value: Length) -> Option<Self> {
        if value.raw() >= 0 {
            Some(Self(value))
        } else {
            None
        }
    }
    pub const fn get(self) -> Length {
        self.0
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PositiveLength(Length);
impl PositiveLength {
    pub const fn new(value: Length) -> Option<Self> {
        if value.raw() > 0 {
            Some(Self(value))
        } else {
            None
        }
    }
    pub const fn get(self) -> Length {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Unitless16_16(i32);
impl Unitless16_16 {
    pub const ONE: Self = Self(65_536);
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PositiveUnitless16_16(Unitless16_16);
impl PositiveUnitless16_16 {
    pub const ONE: Self = Self(Unitless16_16::ONE);
    pub const fn new(value: Unitless16_16) -> Option<Self> {
        if value.raw() > 0 {
            Some(Self(value))
        } else {
            None
        }
    }
    pub const fn get(self) -> Unitless16_16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AffineTransform {
    pub a: Unitless16_16,
    pub b: Unitless16_16,
    pub c: Unitless16_16,
    pub d: Unitless16_16,
    pub e: Length,
    pub f: Length,
}
impl AffineTransform {
    pub const IDENTITY: Self = Self {
        a: Unitless16_16::ONE,
        b: Unitless16_16::from_raw(0),
        c: Unitless16_16::from_raw(0),
        d: Unitless16_16::ONE,
        e: Length::ZERO,
        f: Length::ZERO,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Point {
    pub x: Length,
    pub y: Length,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    x: Length,
    y: Length,
    width: PositiveLength,
    height: PositiveLength,
}
impl Rect {
    pub const fn new(x: Length, y: Length, width: PositiveLength, height: PositiveLength) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub const fn x(self) -> Length {
        self.x
    }
    pub const fn y(self) -> Length {
        self.y
    }
    pub const fn width(self) -> PositiveLength {
        self.width
    }
    pub const fn height(self) -> PositiveLength {
        self.height
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortablePathError {
    Empty,
    Absolute,
    EmptyComponent,
    DotComponent,
    ParentComponent,
    Backslash,
    Colon,
    Nul,
    Control,
}
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PortablePath(String);
impl PortablePath {
    pub fn new(value: impl Into<String>) -> Result<Self, PortablePathError> {
        let value = value.into();
        if value.is_empty() {
            return Err(PortablePathError::Empty);
        }
        if value.starts_with('/') {
            return Err(PortablePathError::Absolute);
        }
        if value.contains('\\') {
            return Err(PortablePathError::Backslash);
        }
        if value.contains(':') {
            return Err(PortablePathError::Colon);
        }
        if value.contains('\0') {
            return Err(PortablePathError::Nul);
        }
        if value.bytes().any(|byte| byte <= 0x1f || byte == 0x7f) {
            return Err(PortablePathError::Control);
        }
        for component in value.split('/') {
            if component.is_empty() {
                return Err(PortablePathError::EmptyComponent);
            }
            if component == "." {
                return Err(PortablePathError::DotComponent);
            }
            if component == ".." {
                return Err(PortablePathError::ParentComponent);
            }
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ConfigResourceRoot {
    ProjectRoot,
    Relative(PortablePath),
}
impl ConfigResourceRoot {
    pub fn parse(value: impl Into<String>) -> Result<Self, PortablePathError> {
        let value = value.into();
        if value == "." {
            Ok(Self::ProjectRoot)
        } else {
            PortablePath::new(value).map(Self::Relative)
        }
    }
    pub fn wire_value(&self) -> &str {
        match self {
            Self::ProjectRoot => ".",
            Self::Relative(path) => path.as_str(),
        }
    }
}

/// A platform-native execution path. This type is intentionally distinct from
/// artifact `PortablePath` and must never be serialized into canonical output.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HostPath(PathBuf);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostPathError {
    Empty,
}

impl HostPath {
    pub fn new(value: impl Into<PathBuf>) -> Result<Self, HostPathError> {
        let value = value.into();
        if value.as_os_str().is_empty() {
            Err(HostPathError::Empty)
        } else {
            Ok(Self(value))
        }
    }
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Host-only source and resource admission facts. CLI root order is retained
/// for deterministic inspection and diagnostics, never as lookup precedence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostAdmissionContext {
    entry: HostPath,
    project_root: HostPath,
    config: Option<HostPath>,
    cli_resource_roots: Vec<HostPath>,
}
impl HostAdmissionContext {
    pub fn new(
        entry: HostPath,
        project_root: HostPath,
        config: Option<HostPath>,
        cli_resource_roots: Vec<HostPath>,
    ) -> Self {
        Self {
            entry,
            project_root,
            config,
            cli_resource_roots,
        }
    }
    pub const fn entry(&self) -> &HostPath {
        &self.entry
    }
    pub const fn project_root(&self) -> &HostPath {
        &self.project_root
    }
    pub const fn config(&self) -> Option<&HostPath> {
        self.config.as_ref()
    }
    pub fn cli_resource_roots(&self) -> &[HostPath] {
        &self.cli_resource_roots
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputSink {
    File,
    Stdout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplacePolicy {
    NoReplace,
    Replace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum BuildOutputTarget {
    File(HostPath),
    Stdout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildExecutionError {
    EmptyOutput,
    AliasedWriteTarget,
    CurrentDirectoryUnavailable,
}

/// Host-only write targets. The exact CLI token `-` selects stdout; every
/// other token, including `./-`, remains a normal host file path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildExecutionContext {
    output: BuildOutputTarget,
    trace_target: Option<HostPath>,
    manifest_target: Option<HostPath>,
    replace_policy: ReplacePolicy,
}
impl BuildExecutionContext {
    pub fn from_cli_token(
        output_token: &OsStr,
        trace_target: Option<HostPath>,
        manifest_target: Option<HostPath>,
        replace_policy: ReplacePolicy,
    ) -> Result<Self, BuildExecutionError> {
        if output_token.is_empty() {
            return Err(BuildExecutionError::EmptyOutput);
        }
        let output = if output_token == OsStr::new("-") {
            BuildOutputTarget::Stdout
        } else {
            BuildOutputTarget::File(
                HostPath::new(PathBuf::from(output_token))
                    .map_err(|_| BuildExecutionError::EmptyOutput)?,
            )
        };
        let context = Self {
            output,
            trace_target,
            manifest_target,
            replace_policy,
        };
        context.validate_distinct_targets()?;
        Ok(context)
    }
    pub const fn output_sink(&self) -> OutputSink {
        match self.output {
            BuildOutputTarget::File(_) => OutputSink::File,
            BuildOutputTarget::Stdout => OutputSink::Stdout,
        }
    }
    pub const fn output_path(&self) -> Option<&HostPath> {
        match &self.output {
            BuildOutputTarget::File(path) => Some(path),
            BuildOutputTarget::Stdout => None,
        }
    }
    pub const fn trace_target(&self) -> Option<&HostPath> {
        self.trace_target.as_ref()
    }
    pub const fn manifest_target(&self) -> Option<&HostPath> {
        self.manifest_target.as_ref()
    }
    pub const fn replace_policy(&self) -> ReplacePolicy {
        self.replace_policy
    }

    /// Re-resolves every configured write target and rejects aliases using
    /// the current filesystem state. A sink owner must call this immediately
    /// before each write because paths that were distinct when CLI arguments
    /// were admitted can later become symlink or hard-link aliases.
    pub fn revalidate_write_targets(&self) -> Result<(), BuildExecutionError> {
        self.validate_distinct_targets()
    }

    fn validate_distinct_targets(&self) -> Result<(), BuildExecutionError> {
        let mut identities = Vec::new();
        if let Some(path) = self.output_path() {
            identities.push(write_target_identity(path)?);
        }
        if let Some(path) = self.trace_target() {
            identities.push(write_target_identity(path)?);
        }
        if let Some(path) = self.manifest_target() {
            identities.push(write_target_identity(path)?);
        }
        let unique: std::collections::BTreeSet<_> = identities.iter().collect();
        if unique.len() != identities.len() {
            return Err(BuildExecutionError::AliasedWriteTarget);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum WriteTargetIdentity {
    #[cfg(unix)]
    ExistingFile {
        device: u64,
        inode: u64,
    },
    ResolvedPath(PathBuf),
}

fn write_target_identity(path: &HostPath) -> Result<WriteTargetIdentity, BuildExecutionError> {
    if let Ok(canonical) = std::fs::canonicalize(path.as_path()) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = std::fs::metadata(&canonical)
                .map_err(|_| BuildExecutionError::CurrentDirectoryUnavailable)?;
            return Ok(WriteTargetIdentity::ExistingFile {
                device: metadata.dev(),
                inode: metadata.ino(),
            });
        }
        #[cfg(not(unix))]
        return Ok(WriteTargetIdentity::ResolvedPath(canonical));
    }
    let absolute = if path.as_path().is_absolute() {
        path.as_path().to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| BuildExecutionError::CurrentDirectoryUnavailable)?
            .join(path.as_path())
    };
    let normalized = normalize_host_path(&absolute);
    let mut existing_ancestor = normalized.as_path();
    while !existing_ancestor.exists() {
        existing_ancestor = existing_ancestor
            .parent()
            .ok_or(BuildExecutionError::EmptyOutput)?;
    }
    let canonical_ancestor = std::fs::canonicalize(existing_ancestor)
        .map_err(|_| BuildExecutionError::CurrentDirectoryUnavailable)?;
    let suffix = normalized
        .strip_prefix(existing_ancestor)
        .map_err(|_| BuildExecutionError::CurrentDirectoryUnavailable)?;
    Ok(WriteTargetIdentity::ResolvedPath(
        canonical_ancestor.join(suffix),
    ))
}

fn normalize_host_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SafeUriError {
    Empty,
    TooLong,
    MissingScheme,
    InvalidScheme,
    SchemeNotAllowed,
    ControlOrWhitespace,
    MissingSchemeSpecificPart,
    MissingHttpAuthority,
    InvalidMailtoAddress,
    InvalidTelephoneNumber,
    InvalidAllowedScheme,
    NonCanonicalAllowedSchemes,
}

/// A URI admitted at the syntax boundary under an explicit scheme and size policy.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SafeUri(String);
impl SafeUri {
    pub fn new(value: impl Into<String>) -> Result<Self, SafeUriError> {
        Self::with_policy(value, DEFAULT_ALLOWED_URI_SCHEMES, DEFAULT_MAX_URI_BYTES)
    }

    pub fn with_policy(
        value: impl Into<String>,
        allowed_schemes: &[&str],
        max_bytes: usize,
    ) -> Result<Self, SafeUriError> {
        let unique_schemes: std::collections::BTreeSet<_> =
            allowed_schemes.iter().copied().collect();
        if unique_schemes.len() != allowed_schemes.len()
            || allowed_schemes
                .iter()
                .any(|scheme| !DEFAULT_ALLOWED_URI_SCHEMES.contains(scheme))
        {
            return Err(SafeUriError::InvalidAllowedScheme);
        }
        if allowed_schemes.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(SafeUriError::NonCanonicalAllowedSchemes);
        }
        let value = value.into();
        if value.is_empty() {
            return Err(SafeUriError::Empty);
        }
        if value.len() > max_bytes {
            return Err(SafeUriError::TooLong);
        }
        if value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        {
            return Err(SafeUriError::ControlOrWhitespace);
        }

        let colon = value.find(':').ok_or(SafeUriError::MissingScheme)?;
        let raw_scheme = &value[..colon];
        if raw_scheme.is_empty()
            || !raw_scheme.as_bytes()[0].is_ascii_alphabetic()
            || !raw_scheme
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
        {
            return Err(SafeUriError::InvalidScheme);
        }
        let scheme = raw_scheme.to_ascii_lowercase();
        if !allowed_schemes.iter().any(|allowed| *allowed == scheme) {
            return Err(SafeUriError::SchemeNotAllowed);
        }

        let remainder = &value[colon + 1..];
        if remainder.is_empty() {
            return Err(SafeUriError::MissingSchemeSpecificPart);
        }
        if matches!(scheme.as_str(), "http" | "https") {
            let authority = remainder
                .strip_prefix("//")
                .ok_or(SafeUriError::MissingHttpAuthority)?;
            let authority = authority.split(['/', '?', '#']).next().unwrap_or_default();
            if authority.is_empty() {
                return Err(SafeUriError::MissingHttpAuthority);
            }
        } else if scheme == "mailto" {
            let address = remainder
                .split_once('?')
                .map_or(remainder, |(address, _)| address);
            let (local, domain) = address
                .split_once('@')
                .ok_or(SafeUriError::InvalidMailtoAddress)?;
            if local.is_empty() || domain.is_empty() || local.contains('@') || domain.contains('@')
            {
                return Err(SafeUriError::InvalidMailtoAddress);
            }
        } else if scheme == "tel" {
            let number = remainder.strip_prefix('+').unwrap_or(remainder);
            if number.is_empty()
                || !number.as_bytes()[0].is_ascii_digit()
                || !number
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'(' | b')' | b'.' | b'-'))
            {
                return Err(SafeUriError::InvalidTelephoneNumber);
            }
        }

        let mut normalized = scheme;
        normalized.push(':');
        normalized.push_str(remainder);
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate_policy(
        &self,
        allowed_schemes: &[&str],
        max_bytes: usize,
    ) -> Result<(), SafeUriError> {
        Self::with_policy(self.0.clone(), allowed_schemes, max_bytes).map(|_| ())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Fingerprint(pub [u8; 32]);

macro_rules! fingerprint_type {
    ($name:ident, $algorithm:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 32]);
        impl $name {
            pub const ALGORITHM_ID: &'static str = $algorithm;
            const fn new(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }
            /// Decode an untrusted wire claim. Trusted phase APIs must
            /// recompute and compare this value before accepting the artifact.
            pub const fn from_untrusted_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }
            pub const fn bytes(self) -> [u8; 32] {
                self.0
            }
        }
        impl From<$name> for Fingerprint {
            fn from(value: $name) -> Self {
                Self(value.bytes())
            }
        }
    };
}

fingerprint_type!(DocumentFingerprint, "typaxis.document-state.sha256/1");
fingerprint_type!(StyleFingerprint, "typaxis.style-state.sha256/1");
fingerprint_type!(
    AdmittedResourceFingerprint,
    "typaxis.admitted-resources.jcs-sha256/1"
);
fingerprint_type!(ReferenceFingerprint, "typaxis.reference-state.jcs-sha256/1");
fingerprint_type!(
    EffectiveConfigFingerprint,
    "typaxis.effective-config.jcs-sha256/1"
);

/// Fingerprint value shared by the two closed pagination-state record
/// variants. It deliberately has no single `ALGORITHM_ID`: state 0 and
/// materialized states use distinct canonical JCS domains.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LayoutStateFingerprint([u8; 32]);
impl LayoutStateFingerprint {
    pub const INITIAL_ALGORITHM_ID: &'static str = "typaxis.initial-pagination-state/1";
    pub const MATERIALIZED_ALGORITHM_ID: &'static str = "typaxis.pagination-fingerprint/1";
    const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub const fn from_untrusted_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}
impl From<LayoutStateFingerprint> for Fingerprint {
    fn from(value: LayoutStateFingerprint) -> Self {
        Self(value.bytes())
    }
}

pub fn document_fingerprint_from_jcs(canonical_jcs: &str) -> DocumentFingerprint {
    DocumentFingerprint::new(sha256(canonical_jcs.as_bytes()))
}
pub fn style_fingerprint_from_jcs(canonical_jcs: &str) -> StyleFingerprint {
    StyleFingerprint::new(sha256(canonical_jcs.as_bytes()))
}
pub fn admitted_resource_fingerprint_from_jcs(canonical_jcs: &str) -> AdmittedResourceFingerprint {
    AdmittedResourceFingerprint::new(sha256(canonical_jcs.as_bytes()))
}
pub fn initial_pagination_state_fingerprint_from_jcs(
    canonical_jcs: &str,
) -> LayoutStateFingerprint {
    debug_assert!(canonical_jcs.contains(LayoutStateFingerprint::INITIAL_ALGORITHM_ID));
    LayoutStateFingerprint::new(sha256(canonical_jcs.as_bytes()))
}
pub fn materialized_pagination_state_fingerprint_from_jcs(
    canonical_jcs: &str,
) -> LayoutStateFingerprint {
    debug_assert!(canonical_jcs.contains(LayoutStateFingerprint::MATERIALIZED_ALGORITHM_ID));
    LayoutStateFingerprint::new(sha256(canonical_jcs.as_bytes()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfStreamCompression {
    Flate,
    None,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveDataVersions {
    unicode: String,
    japanese_line_break: String,
}
impl EffectiveDataVersions {
    pub fn new(unicode: impl Into<String>, japanese_line_break: impl Into<String>) -> Option<Self> {
        let unicode = unicode.into();
        let japanese_line_break = japanese_line_break.into();
        if unicode != REGISTERED_UNICODE_VERSION
            || japanese_line_break != REGISTERED_JAPANESE_LINE_BREAK_VERSION
        {
            return None;
        }
        Some(Self {
            unicode,
            japanese_line_break,
        })
    }
    pub fn unicode(&self) -> &str {
        &self.unicode
    }
    pub fn japanese_line_break(&self) -> &str {
        &self.japanese_line_break
    }
}

/// Token proving the configured selectors resolved to the Profile 1.0 tables
/// actually linked into this engine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedDataTables(EffectiveDataVersions);
impl ResolvedDataTables {
    pub fn resolve(unicode: &str, japanese_line_break: &str) -> Option<Self> {
        EffectiveDataVersions::new(unicode, japanese_line_break).map(Self)
    }
    pub const fn versions(&self) -> &EffectiveDataVersions {
        &self.0
    }
}

/// Compile-time engine identity; callers cannot replace product/version facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineIdentity {
    name: &'static str,
    version: &'static str,
    rust_version: &'static str,
    git_commit: Option<&'static str>,
}
impl EngineIdentity {
    pub const fn compiled() -> Self {
        Self {
            name: PRODUCT_NAME,
            version: env!("CARGO_PKG_VERSION"),
            rust_version: env!("TYPAXIS_RUST_VERSION"),
            git_commit: option_env!("TYPAXIS_GIT_COMMIT"),
        }
    }
    pub const fn name(&self) -> &'static str {
        self.name
    }
    pub const fn version(&self) -> &'static str {
        self.version
    }
    pub const fn rust_version(&self) -> &'static str {
        self.rust_version
    }
    pub const fn git_commit(&self) -> Option<&'static str> {
        self.git_commit
    }
}

/// Closed identity fact for the shaping backend selected by this engine
/// build. This value prevents arbitrary identity strings; by itself it is not
/// a capability proving that a particular implementation executed shaping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShaperIdentity {
    backend: &'static str,
    version: &'static str,
}
impl ShaperIdentity {
    pub const fn linked_reference() -> Self {
        Self {
            backend: "typaxis-reference-shaper",
            version: env!("CARGO_PKG_VERSION"),
        }
    }
    pub const fn backend(self) -> &'static str {
        self.backend
    }
    pub const fn version(self) -> &'static str {
        self.version
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    pub max_input_bytes: u64,
    pub max_source_bytes: u32,
    pub max_include_depth: u32,
    pub max_include_files: u32,
    pub max_ast_nesting_depth: u32,
    pub max_ast_nodes: u64,
    pub max_style_rules: u64,
    pub max_text_bytes: u64,
    pub max_text_buffer_bytes: u32,
    pub max_shaping_context_bytes: u32,
    pub max_font_bytes: u64,
    pub max_fonts: u32,
    pub max_image_bytes: u64,
    pub max_images: u32,
    pub max_resource_bytes: u64,
    pub max_image_pixels: u64,
    pub max_decoded_image_bytes: u64,
    pub max_pages: u32,
    pub max_layout_passes: u16,
    pub max_uri_bytes: u32,
    pub max_line_reshape_passes: u16,
    pub max_page_break_lookback: u16,
    pub max_footnote_reflows_per_page: u16,
    pub max_column_balance_candidates: u16,
    pub max_float_queue: u32,
    pub max_float_carry_pages: u16,
    pub max_cids_per_font: u16,
    pub max_fragments: u64,
    pub max_spool_bytes: u64,
    pub max_pdf_objects: u32,
    pub max_output_bytes: u64,
}
pub const MAX_AST_NESTING_DEPTH: u32 = 64;
/// Number of collision-free six-uppercase-letter PDF subset tags.
pub const MAX_FONT_SUBSET_TAGS: u32 = 26u32.pow(6);
impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 16 * 1024 * 1024,
            max_source_bytes: 16 * 1024 * 1024,
            max_include_depth: 16,
            max_include_files: 1024,
            max_ast_nesting_depth: MAX_AST_NESTING_DEPTH,
            max_ast_nodes: 1_000_000,
            max_style_rules: 100_000,
            max_text_bytes: 64 * 1024 * 1024,
            max_text_buffer_bytes: 16 * 1024 * 1024,
            max_shaping_context_bytes: 64 * 1024,
            max_font_bytes: 128 * 1024 * 1024,
            max_fonts: 256,
            max_image_bytes: 128 * 1024 * 1024,
            max_images: 1024,
            max_resource_bytes: 1024 * 1024 * 1024,
            max_image_pixels: 100_000_000,
            max_decoded_image_bytes: 512 * 1024 * 1024,
            max_pages: 10_000,
            max_layout_passes: 8,
            max_uri_bytes: DEFAULT_MAX_URI_BYTES as u32,
            max_line_reshape_passes: 4,
            max_page_break_lookback: 32,
            max_footnote_reflows_per_page: 8,
            max_column_balance_candidates: 16,
            max_float_queue: 1_024,
            max_float_carry_pages: 16,
            max_cids_per_font: u16::MAX,
            max_fragments: 10_000_000,
            max_spool_bytes: 4 * 1024 * 1024 * 1024,
            max_pdf_objects: 5_000_000,
            max_output_bytes: 8 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceLimitsError {
    ZeroLimit,
    IntegerNotJsonSafe,
    AstNestingDepthExceedsProfile,
    FontCountExceedsSubsetNamespace,
    SourceExceedsInput,
    TextBufferExceedsText,
    ShapingContextExceedsTextBuffer,
    FontExceedsResources,
    ImageExceedsResources,
    OutputExceedsClassicXref,
}

impl ResourceLimits {
    pub fn validate(&self) -> Result<(), ResourceLimitsError> {
        let positive = [
            self.max_input_bytes,
            u64::from(self.max_source_bytes),
            u64::from(self.max_include_depth),
            u64::from(self.max_include_files),
            u64::from(self.max_ast_nesting_depth),
            self.max_ast_nodes,
            self.max_style_rules,
            self.max_text_bytes,
            u64::from(self.max_text_buffer_bytes),
            u64::from(self.max_shaping_context_bytes),
            self.max_font_bytes,
            u64::from(self.max_fonts),
            self.max_image_bytes,
            u64::from(self.max_images),
            self.max_resource_bytes,
            self.max_image_pixels,
            self.max_decoded_image_bytes,
            u64::from(self.max_pages),
            u64::from(self.max_layout_passes),
            u64::from(self.max_uri_bytes),
            u64::from(self.max_line_reshape_passes),
            u64::from(self.max_page_break_lookback),
            u64::from(self.max_footnote_reflows_per_page),
            u64::from(self.max_column_balance_candidates),
            u64::from(self.max_float_queue),
            u64::from(self.max_float_carry_pages),
            u64::from(self.max_cids_per_font),
            self.max_fragments,
            self.max_spool_bytes,
            u64::from(self.max_pdf_objects),
            self.max_output_bytes,
        ];
        if positive.contains(&0) {
            return Err(ResourceLimitsError::ZeroLimit);
        }
        let json_safe = [
            self.max_input_bytes,
            self.max_ast_nodes,
            self.max_style_rules,
            self.max_text_bytes,
            self.max_font_bytes,
            self.max_image_bytes,
            self.max_resource_bytes,
            self.max_image_pixels,
            self.max_decoded_image_bytes,
            self.max_fragments,
            self.max_spool_bytes,
            self.max_output_bytes,
        ];
        if json_safe
            .iter()
            .any(|value| *value > JSON_SAFE_INTEGER_MAX as u64)
        {
            return Err(ResourceLimitsError::IntegerNotJsonSafe);
        }
        if self.max_ast_nesting_depth > MAX_AST_NESTING_DEPTH {
            return Err(ResourceLimitsError::AstNestingDepthExceedsProfile);
        }
        if self.max_fonts > MAX_FONT_SUBSET_TAGS {
            return Err(ResourceLimitsError::FontCountExceedsSubsetNamespace);
        }
        if u64::from(self.max_source_bytes) > self.max_input_bytes {
            return Err(ResourceLimitsError::SourceExceedsInput);
        }
        if u64::from(self.max_text_buffer_bytes) > self.max_text_bytes {
            return Err(ResourceLimitsError::TextBufferExceedsText);
        }
        if self.max_shaping_context_bytes > self.max_text_buffer_bytes {
            return Err(ResourceLimitsError::ShapingContextExceedsTextBuffer);
        }
        if self.max_font_bytes > self.max_resource_bytes {
            return Err(ResourceLimitsError::FontExceedsResources);
        }
        if self.max_image_bytes > self.max_resource_bytes {
            return Err(ResourceLimitsError::ImageExceedsResources);
        }
        if self.max_output_bytes > 9_999_999_999 {
            return Err(ResourceLimitsError::OutputExceedsClassicXref);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedResourceLimits(ResourceLimits);
impl ValidatedResourceLimits {
    pub fn new(limits: ResourceLimits) -> Result<Self, ResourceLimitsError> {
        limits.validate()?;
        Ok(Self(limits))
    }
    pub const fn get(&self) -> &ResourceLimits {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectiveConfigError {
    ResourceLimits(ResourceLimitsError),
    NonCanonicalResourceRoots,
    InvalidAllowedUriSchemes,
}

/// Canonical, fully validated configuration facts. Its fingerprint is always
/// computed from this value's contract-defined JCS representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveConfig {
    strict: bool,
    stream_compression: PdfStreamCompression,
    resource_roots: Vec<ConfigResourceRoot>,
    allowed_uri_schemes: Vec<String>,
    data_versions: EffectiveDataVersions,
    limits: ValidatedResourceLimits,
    canonical_jcs: String,
    fingerprint: EffectiveConfigFingerprint,
}
impl EffectiveConfig {
    pub fn new(
        strict: bool,
        stream_compression: PdfStreamCompression,
        resource_roots: Vec<ConfigResourceRoot>,
        allowed_uri_schemes: Vec<String>,
        data_versions: EffectiveDataVersions,
        limits: ResourceLimits,
    ) -> Result<Self, EffectiveConfigError> {
        if resource_roots
            .windows(2)
            .any(|pair| pair[0].wire_value() >= pair[1].wire_value())
        {
            return Err(EffectiveConfigError::NonCanonicalResourceRoots);
        }
        let schemes: Vec<&str> = allowed_uri_schemes.iter().map(String::as_str).collect();
        if schemes.windows(2).any(|pair| pair[0] >= pair[1])
            || schemes
                .iter()
                .any(|scheme| !DEFAULT_ALLOWED_URI_SCHEMES.contains(scheme))
        {
            return Err(EffectiveConfigError::InvalidAllowedUriSchemes);
        }
        let limits =
            ValidatedResourceLimits::new(limits).map_err(EffectiveConfigError::ResourceLimits)?;
        let mut config = Self {
            strict,
            stream_compression,
            resource_roots,
            allowed_uri_schemes,
            data_versions,
            limits,
            canonical_jcs: String::new(),
            fingerprint: EffectiveConfigFingerprint::new([0; 32]),
        };
        config.canonical_jcs = config.encode_jcs();
        config.fingerprint =
            EffectiveConfigFingerprint::new(sha256(config.canonical_jcs.as_bytes()));
        Ok(config)
    }
    pub const fn deterministic(&self) -> bool {
        true
    }
    pub const fn strict(&self) -> bool {
        self.strict
    }
    pub const fn stream_compression(&self) -> PdfStreamCompression {
        self.stream_compression
    }
    pub fn resource_roots(&self) -> &[ConfigResourceRoot] {
        &self.resource_roots
    }
    pub fn allowed_uri_schemes(&self) -> &[String] {
        &self.allowed_uri_schemes
    }
    pub const fn data_versions(&self) -> &EffectiveDataVersions {
        &self.data_versions
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> EffectiveConfigFingerprint {
        self.fingerprint
    }

    fn encode_jcs(&self) -> String {
        let mut output = String::from("{\"allowed_uri_schemes\":[");
        push_jcs_string_array(&mut output, &self.allowed_uri_schemes);
        output.push_str("],\"contract\":");
        push_jcs_string(&mut output, CONTRACT);
        output.push_str(",\"data_versions\":{\"japanese_line_break\":");
        push_jcs_string(&mut output, self.data_versions.japanese_line_break());
        output.push_str(",\"unicode\":");
        push_jcs_string(&mut output, self.data_versions.unicode());
        output.push_str("},\"deterministic\":true,\"limits\":{");
        push_limits_jcs(&mut output, self.limits.get());
        output.push_str("},\"pdf_stream_compression\":");
        push_jcs_string(
            &mut output,
            match self.stream_compression {
                PdfStreamCompression::Flate => "flate",
                PdfStreamCompression::None => "none",
            },
        );
        output.push_str(",\"resource_roots\":[");
        for (index, root) in self.resource_roots.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            push_jcs_string(&mut output, root.wire_value());
        }
        output.push_str("],\"strict\":");
        output.push_str(if self.strict { "true" } else { "false" });
        output.push('}');
        output
    }
}

fn push_jcs_string_array(output: &mut String, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_jcs_string(output, value);
    }
}

pub fn push_jcs_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{09}' => output.push_str("\\t"),
            '\u{0a}' => output.push_str("\\n"),
            '\u{0c}' => output.push_str("\\f"),
            '\u{0d}' => output.push_str("\\r"),
            character if character <= '\u{1f}' => {
                output.push_str(&format!("\\u{:04x}", u32::from(character)));
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

pub const fn generation_kind_wire_name(kind: GenerationKind) -> &'static str {
    match kind {
        GenerationKind::PageReference => "page_reference",
        GenerationKind::Counter => "counter",
        GenerationKind::ListMarker => "list_marker",
        GenerationKind::FootnoteMarker => "footnote_marker",
        GenerationKind::Discretionary => "discretionary",
    }
}

pub fn push_generated_buffer_key_jcs(output: &mut String, key: GeneratedBufferKey) {
    output.push_str("{\"generation_kind\":");
    push_jcs_string(output, generation_kind_wire_name(key.generation_kind()));
    output.push_str(",\"owner\":");
    output.push_str(&key.owner().get().to_string());
    output.push_str(",\"owner_local_ordinal\":");
    output.push_str(&key.owner_local_ordinal().to_string());
    output.push('}');
}

pub fn generated_text_reference_fingerprint(
    records: &[(GeneratedBufferKey, String)],
) -> ReferenceFingerprint {
    let mut canonical: Vec<_> = records.iter().collect();
    canonical.sort_by_key(|(key, _)| *key);
    let mut jcs = String::from("{\"algorithm\":");
    push_jcs_string(&mut jcs, ReferenceFingerprint::ALGORITHM_ID);
    jcs.push_str(",\"resolved_generated_text\":[");
    for (index, (key, utf8)) in canonical.into_iter().enumerate() {
        if index > 0 {
            jcs.push(',');
        }
        jcs.push_str("{\"end_byte\":");
        jcs.push_str(&utf8.len().to_string());
        jcs.push_str(",\"key\":");
        push_generated_buffer_key_jcs(&mut jcs, *key);
        jcs.push_str(",\"start_byte\":0,\"utf8\":");
        push_jcs_string(&mut jcs, utf8);
        jcs.push('}');
    }
    jcs.push_str("]}");
    ReferenceFingerprint::new(sha256(jcs.as_bytes()))
}

fn push_limits_jcs(output: &mut String, limits: &ResourceLimits) {
    macro_rules! fields {
        ($(($name:literal, $value:expr)),+ $(,)?) => {{
            let mut first = true;
            $(
                if !first { output.push(','); }
                first = false;
                output.push_str(concat!("\"", $name, "\":"));
                output.push_str(&$value.to_string());
            )+
            let _ = first;
        }};
    }
    fields!(
        ("max_ast_nesting_depth", limits.max_ast_nesting_depth),
        ("max_ast_nodes", limits.max_ast_nodes),
        ("max_cids_per_font", limits.max_cids_per_font),
        (
            "max_column_balance_candidates",
            limits.max_column_balance_candidates
        ),
        ("max_decoded_image_bytes", limits.max_decoded_image_bytes),
        ("max_float_carry_pages", limits.max_float_carry_pages),
        ("max_float_queue", limits.max_float_queue),
        ("max_font_bytes", limits.max_font_bytes),
        ("max_fonts", limits.max_fonts),
        (
            "max_footnote_reflows_per_page",
            limits.max_footnote_reflows_per_page
        ),
        ("max_fragments", limits.max_fragments),
        ("max_image_bytes", limits.max_image_bytes),
        ("max_image_pixels", limits.max_image_pixels),
        ("max_images", limits.max_images),
        ("max_include_depth", limits.max_include_depth),
        ("max_include_files", limits.max_include_files),
        ("max_input_bytes", limits.max_input_bytes),
        ("max_layout_passes", limits.max_layout_passes),
        ("max_line_reshape_passes", limits.max_line_reshape_passes),
        ("max_output_bytes", limits.max_output_bytes),
        ("max_page_break_lookback", limits.max_page_break_lookback),
        ("max_pages", limits.max_pages),
        ("max_pdf_objects", limits.max_pdf_objects),
        ("max_resource_bytes", limits.max_resource_bytes),
        (
            "max_shaping_context_bytes",
            limits.max_shaping_context_bytes
        ),
        ("max_source_bytes", limits.max_source_bytes),
        ("max_spool_bytes", limits.max_spool_bytes),
        ("max_style_rules", limits.max_style_rules),
        ("max_text_buffer_bytes", limits.max_text_buffer_bytes),
        ("max_text_bytes", limits.max_text_bytes),
        ("max_uri_bytes", limits.max_uri_bytes),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spans_are_ordered_and_distinct() {
        assert!(Utf8ByteRange::new(Utf8ByteOffset::new(2), Utf8ByteOffset::new(1)).is_none());
        assert!(SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1)
        )
        .is_some());
        assert!(TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1)
        )
        .is_some());
    }
    #[test]
    fn rational_conversion_matches_a4_fixture() {
        assert_eq!(
            Length::from_rational_pdf_points(210 * 720, 254)
                .unwrap()
                .raw(),
            39_011_981
        );
        assert_eq!(
            Length::from_rational_pdf_points(297 * 720, 254)
                .unwrap()
                .raw(),
            55_174_088
        );
        assert_eq!(
            Length::from_raw(JSON_SAFE_INTEGER_MAX).unwrap().raw(),
            JSON_SAFE_INTEGER_MAX
        );
        assert_eq!(
            Length::from_raw(-JSON_SAFE_INTEGER_MAX).unwrap().raw(),
            -JSON_SAFE_INTEGER_MAX
        );
        assert!(Length::from_raw(JSON_SAFE_INTEGER_MAX + 1).is_none());
        assert!(Length::from_raw(-JSON_SAFE_INTEGER_MAX - 1).is_none());
        assert!(Length::from_raw(JSON_SAFE_INTEGER_MAX)
            .unwrap()
            .checked_add(Length::from_raw(1).unwrap())
            .is_none());
    }
    #[test]
    fn portable_path_rejects_escape_forms() {
        assert!(PortablePath::new("fonts/body.ttf").is_ok());
        assert!(PortablePath::new("../secret").is_err());
        assert!(PortablePath::new("C:/secret").is_err());
        assert!(PortablePath::new("/absolute").is_err());
        assert_eq!(
            PortablePath::new("line\nbreak"),
            Err(PortablePathError::Control)
        );
        assert_eq!(
            PortablePath::new("delete\u{7f}"),
            Err(PortablePathError::Control)
        );
        assert_eq!(
            ConfigResourceRoot::parse(".").unwrap(),
            ConfigResourceRoot::ProjectRoot
        );
        assert!(HostPath::new("/absolute/host/path").is_ok());
        assert!(HostPath::new(r"C:\host\path").is_ok());
    }
    #[test]
    fn bidi_level_parity_is_direction() {
        assert!(!BidiLevel::new(0).unwrap().is_rtl());
        assert!(BidiLevel::new(1).unwrap().is_rtl());
        assert!(BidiLevel::new(126).is_none());
    }
    #[test]
    fn safe_uri_normalizes_and_enforces_policy() {
        assert_eq!(
            SafeUri::new("HTTPS://example.test/a").unwrap().as_str(),
            "https://example.test/a"
        );
        assert!(matches!(
            SafeUri::new("javascript:alert(1)"),
            Err(SafeUriError::SchemeNotAllowed)
        ));
        assert!(matches!(
            SafeUri::new("https://example.test/a b"),
            Err(SafeUriError::ControlOrWhitespace)
        ));
        assert!(matches!(
            SafeUri::new("https:///missing-host"),
            Err(SafeUriError::MissingHttpAuthority)
        ));
        assert!(SafeUri::new("mailto:a@example.test").is_ok());
        assert!(matches!(
            SafeUri::new("mailto:not-an-address"),
            Err(SafeUriError::InvalidMailtoAddress)
        ));
        assert!(SafeUri::new("tel:+81312345678").is_ok());
        assert!(matches!(
            SafeUri::new("tel:call-me"),
            Err(SafeUriError::InvalidTelephoneNumber)
        ));
        assert!(matches!(
            SafeUri::with_policy("https://example.test", &["mailto"], DEFAULT_MAX_URI_BYTES),
            Err(SafeUriError::SchemeNotAllowed)
        ));
        assert!(matches!(
            SafeUri::with_policy("mailto:a@example.test", DEFAULT_ALLOWED_URI_SCHEMES, 4),
            Err(SafeUriError::TooLong)
        ));
        assert_eq!(
            SafeUri::with_policy("javascript:alert(1)", &["javascript"], 100),
            Err(SafeUriError::InvalidAllowedScheme)
        );
        assert_eq!(
            SafeUri::with_policy("https://example.test", &[], 100),
            Err(SafeUriError::SchemeNotAllowed)
        );
        assert_eq!(
            SafeUri::with_policy("https://example.test", &["https", "http"], 100),
            Err(SafeUriError::NonCanonicalAllowedSchemes)
        );
    }
    #[test]
    fn resource_limit_defaults_cover_bounded_feedback_loops() {
        let limits = ResourceLimits::default();
        assert!(limits.max_uri_bytes > 0);
        assert!(limits.max_line_reshape_passes > 0);
        assert!(limits.max_page_break_lookback > 0);
        assert!(limits.max_footnote_reflows_per_page > 0);
        assert!(limits.max_column_balance_candidates > 0);
        assert!(limits.max_float_queue > 0);
        assert!(limits.max_float_carry_pages > 0);
        assert!(limits.max_cids_per_font > 0);
        assert_eq!(limits.max_ast_nesting_depth, MAX_AST_NESTING_DEPTH);
        assert!(limits.validate().is_ok());
        let mut invalid = limits.clone();
        invalid.max_resource_bytes = invalid.max_font_bytes - 1;
        assert_eq!(
            invalid.validate(),
            Err(ResourceLimitsError::FontExceedsResources)
        );
        let mut invalid = limits;
        invalid.max_ast_nesting_depth = MAX_AST_NESTING_DEPTH + 1;
        assert_eq!(
            invalid.validate(),
            Err(ResourceLimitsError::AstNestingDepthExceedsProfile)
        );

        let mut exact_subset_namespace = ResourceLimits {
            max_fonts: MAX_FONT_SUBSET_TAGS,
            ..ResourceLimits::default()
        };
        assert!(exact_subset_namespace.validate().is_ok());
        exact_subset_namespace.max_fonts = MAX_FONT_SUBSET_TAGS + 1;
        assert_eq!(
            exact_subset_namespace.validate(),
            Err(ResourceLimitsError::FontCountExceedsSubsetNamespace)
        );
    }
    #[test]
    fn cross_phase_string_ids_use_schema_grammars() {
        assert!(AnchorId::new("chapter.1:lead").is_ok());
        assert!(FootnoteId::new("note_1").is_ok());
        assert!(MasterId::new("body-master").is_ok());
        assert!(StyleId::new("lead_text").is_ok());
        assert!(AnchorId::new("1chapter").is_err());
        assert!(StyleId::new("chapter.lead").is_err());
        assert!(MasterId::new("body master").is_err());
    }

    #[test]
    fn sha256_matches_standard_vector() {
        assert_eq!(
            sha256(b"abc"),
            [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad,
            ]
        );
    }

    #[test]
    fn reference_fingerprint_golden_includes_its_algorithm_domain() {
        let fingerprint = generated_text_reference_fingerprint(&[(
            GeneratedBufferKey::new(NodeId::new(7), GenerationKind::ListMarker, 0),
            "\u{2022}".to_owned(),
        )]);
        let hex: String = fingerprint
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            hex,
            "e5661ab14986c87bc6616fc41e7ffdadca9afc3bd3f1b61b6feb91383d703c86"
        );
    }

    #[test]
    fn build_execution_classifies_exact_dash_and_rejects_target_aliases() {
        let stdout = BuildExecutionContext::from_cli_token(
            OsStr::new("-"),
            None,
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        assert_eq!(stdout.output_sink(), OutputSink::Stdout);
        assert!(stdout.output_path().is_none());

        let file = BuildExecutionContext::from_cli_token(
            OsStr::new("./-"),
            None,
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        assert_eq!(file.output_sink(), OutputSink::File);

        let same = HostPath::new("target/out.pdf").unwrap();
        assert_eq!(
            BuildExecutionContext::from_cli_token(
                OsStr::new("target/out.pdf"),
                Some(same),
                None,
                ReplacePolicy::Replace,
            ),
            Err(BuildExecutionError::AliasedWriteTarget)
        );
    }

    #[cfg(unix)]
    #[test]
    fn build_execution_rejects_existing_symlink_aliases() {
        use std::os::unix::fs::symlink;
        let directory =
            std::env::temp_dir().join(format!("typaxis-core-target-alias-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let target = directory.join("target.pdf");
        let alias = directory.join("alias.pdf");
        std::fs::write(&target, b"existing").unwrap();
        if alias.exists() || alias.is_symlink() {
            std::fs::remove_file(&alias).unwrap();
        }
        symlink(&target, &alias).unwrap();
        let result = BuildExecutionContext::from_cli_token(
            target.as_os_str(),
            Some(HostPath::new(alias.clone()).unwrap()),
            None,
            ReplacePolicy::NoReplace,
        );
        assert_eq!(result, Err(BuildExecutionError::AliasedWriteTarget));
        std::fs::remove_file(alias).unwrap();
        std::fs::remove_file(target).unwrap();

        let hard_target = directory.join("hard-target.pdf");
        let hard_alias = directory.join("hard-alias.pdf");
        std::fs::write(&hard_target, b"same inode").unwrap();
        std::fs::hard_link(&hard_target, &hard_alias).unwrap();
        assert_eq!(
            BuildExecutionContext::from_cli_token(
                hard_target.as_os_str(),
                Some(HostPath::new(hard_alias.clone()).unwrap()),
                None,
                ReplacePolicy::NoReplace,
            ),
            Err(BuildExecutionError::AliasedWriteTarget)
        );
        std::fs::remove_file(hard_alias).unwrap();
        std::fs::remove_file(hard_target).unwrap();

        let ancestor_alias = directory.join("ancestor-alias");
        symlink(&directory, &ancestor_alias).unwrap();
        let direct_nested = directory.join("missing/nested/out.pdf");
        let alias_nested = ancestor_alias.join("missing/nested/out.pdf");
        assert_eq!(
            BuildExecutionContext::from_cli_token(
                direct_nested.as_os_str(),
                Some(HostPath::new(alias_nested).unwrap()),
                None,
                ReplacePolicy::NoReplace,
            ),
            Err(BuildExecutionError::AliasedWriteTarget)
        );
        std::fs::remove_file(ancestor_alias).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn effective_config_requires_canonical_sets_and_computes_its_fingerprint() {
        let versions =
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap();
        let config = EffectiveConfig::new(
            false,
            PdfStreamCompression::Flate,
            vec![ConfigResourceRoot::ProjectRoot],
            DEFAULT_ALLOWED_URI_SCHEMES
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            versions.clone(),
            ResourceLimits::default(),
        )
        .unwrap();
        assert!(config.deterministic());
        assert_eq!(
            config.fingerprint().bytes(),
            sha256(config.canonical_jcs().as_bytes())
        );
        let sample = EffectiveConfig::new(
            true,
            PdfStreamCompression::Flate,
            vec![ConfigResourceRoot::ProjectRoot],
            ["http", "https", "mailto"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            versions.clone(),
            ResourceLimits::default(),
        )
        .unwrap();
        let sample_hash: String = sample
            .fingerprint()
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            sample_hash,
            "583021260f14cb709bd0f1eec0c41eb458cb670ff28e8ae948f1b87cb1224d0d"
        );
        assert_eq!(
            EffectiveConfig::new(
                false,
                PdfStreamCompression::Flate,
                vec![
                    ConfigResourceRoot::Relative(PortablePath::new("fonts").unwrap()),
                    ConfigResourceRoot::ProjectRoot,
                ],
                DEFAULT_ALLOWED_URI_SCHEMES
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
                versions,
                ResourceLimits::default(),
            ),
            Err(EffectiveConfigError::NonCanonicalResourceRoots)
        );
    }
}
