//! Closed, in-tree Safe-SVG decoder adopted by ADR-0033.
//!
//! This is deliberately not an XML parser. It accepts one exact, iterative
//! grammar and never resolves a namespace URI, entity, stylesheet, external
//! reference, file, or network resource.

use crate::{ResourceAdmissionError, SafeVectorFailureReason};
use std::collections::{BTreeMap, BTreeSet};
use typaxis_core::{push_jcs_string, sha256, Length, M4EffectiveResourceLimits, PositiveLength};

pub const SAFE_SVG_PARSER_ID: &str = "typaxis.safe-svg-parser/1";
pub const SAFE_VECTOR_IR_ID: &str = "typaxis.safe-vector-ir/1";
pub const SAFE_VECTOR_IR_FINGERPRINT_ID: &str = "typaxis.safe-vector-ir-fingerprint/1";
pub const SAFE_VECTOR_ALLOCATION_CHARGE_ID: &str = "typaxis.safe-vector-allocation-charge/1";
pub const SAFE_SVG_PARSER_ID_V2: &str = "typaxis.safe-svg-parser/2";
pub const SAFE_VECTOR_IR_ID_V2: &str = "typaxis.safe-vector-ir/2";
pub const SAFE_VECTOR_IR_FINGERPRINT_ID_V2: &str = "typaxis.safe-vector-ir-fingerprint/2";
pub const SAFE_VECTOR_ALLOCATION_CHARGE_ID_V2: &str = "typaxis.safe-vector-allocation-charge/2";

/// Nominal parser selection made at the declared-media boundary. Downstream
/// code never reinterprets a media string or a boolean feature switch.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SafeVectorParserProfile {
    SafeSvg1,
    SafeSvg2,
}

impl SafeVectorParserProfile {
    pub const fn parser_id(self) -> &'static str {
        match self {
            Self::SafeSvg1 => SAFE_SVG_PARSER_ID,
            Self::SafeSvg2 => SAFE_SVG_PARSER_ID_V2,
        }
    }

    pub const fn ir_id(self) -> &'static str {
        match self {
            Self::SafeSvg1 => SAFE_VECTOR_IR_ID,
            Self::SafeSvg2 => SAFE_VECTOR_IR_ID_V2,
        }
    }

    pub const fn ir_fingerprint_id(self) -> &'static str {
        match self {
            Self::SafeSvg1 => SAFE_VECTOR_IR_FINGERPRINT_ID,
            Self::SafeSvg2 => SAFE_VECTOR_IR_FINGERPRINT_ID_V2,
        }
    }

    pub const fn allocation_charge_id(self) -> &'static str {
        match self {
            Self::SafeSvg1 => SAFE_VECTOR_ALLOCATION_CHARGE_ID,
            Self::SafeSvg2 => SAFE_VECTOR_ALLOCATION_CHARGE_ID_V2,
        }
    }
}

const FIXED_ONE: i64 = 65_536;
const MAX_COORDINATE: i64 = 1_000_000 * FIXED_ONE;
const MAX_ATTRIBUTES: usize = 16;
const HARD_STACK_DEPTH: usize = 64;
const CIRCLE_CONTROL_RATIO: i64 = 36_195;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SafeVectorPoint {
    x: i64,
    y: i64,
}

impl SafeVectorPoint {
    pub const fn x_raw(self) -> i64 {
        self.x
    }

    pub const fn y_raw(self) -> i64 {
        self.y
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SafeVectorSegment {
    Move(SafeVectorPoint),
    Line(SafeVectorPoint),
    Quadratic(SafeVectorPoint, SafeVectorPoint),
    Cubic(SafeVectorPoint, SafeVectorPoint, SafeVectorPoint),
    Close,
}

impl SafeVectorSegment {
    pub const fn kind_str(&self) -> &'static str {
        match self {
            Self::Move(_) => "move",
            Self::Line(_) => "line",
            Self::Quadratic(_, _) => "quadratic",
            Self::Cubic(_, _, _) => "cubic",
            Self::Close => "close",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeVectorPath {
    segments: Vec<SafeVectorSegment>,
}

impl SafeVectorPath {
    pub fn segments(&self) -> &[SafeVectorSegment] {
        &self.segments
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SafeVectorTransform {
    a: i32,
    d: i32,
    e: i64,
    f: i64,
}

impl SafeVectorTransform {
    pub const IDENTITY: Self = Self {
        a: FIXED_ONE as i32,
        d: FIXED_ONE as i32,
        e: 0,
        f: 0,
    };

    pub const fn a_raw(self) -> i32 {
        self.a
    }

    pub const fn d_raw(self) -> i32 {
        self.d
    }

    pub const fn e_raw(self) -> i64 {
        self.e
    }

    pub const fn f_raw(self) -> i64 {
        self.f
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafeVectorFillRule {
    NonZero,
    EvenOdd,
}

impl SafeVectorFillRule {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NonZero => "nonzero",
            Self::EvenOdd => "evenodd",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafeVectorLineCap {
    Butt,
    Round,
    Square,
}

impl SafeVectorLineCap {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Butt => "butt",
            Self::Round => "round",
            Self::Square => "square",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafeVectorLineJoin {
    Miter,
    Round,
    Bevel,
}

impl SafeVectorLineJoin {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Miter => "miter",
            Self::Round => "round",
            Self::Bevel => "bevel",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SafeVectorStroke {
    color: [u8; 3],
    width: i64,
    line_cap: SafeVectorLineCap,
    line_join: SafeVectorLineJoin,
    miter_limit: i64,
}

impl SafeVectorStroke {
    pub const fn color(self) -> [u8; 3] {
        self.color
    }
    pub const fn width_raw(self) -> i64 {
        self.width
    }
    pub const fn line_cap(self) -> SafeVectorLineCap {
        self.line_cap
    }
    pub const fn line_join(self) -> SafeVectorLineJoin {
        self.line_join
    }
    pub const fn miter_limit_raw(self) -> i64 {
        self.miter_limit
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SafeVectorClipUse {
    clip_id: u32,
    transform: SafeVectorTransform,
}

impl SafeVectorClipUse {
    pub const fn clip_id(self) -> u32 {
        self.clip_id
    }
    pub const fn transform(self) -> SafeVectorTransform {
        self.transform
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeVectorClipDefinition {
    clip_id: u32,
    transform: SafeVectorTransform,
    fill_rule: SafeVectorFillRule,
    path: SafeVectorPath,
}

impl SafeVectorClipDefinition {
    pub const fn clip_id(&self) -> u32 {
        self.clip_id
    }
    pub const fn transform(&self) -> SafeVectorTransform {
        self.transform
    }
    pub const fn fill_rule(&self) -> SafeVectorFillRule {
        self.fill_rule
    }
    pub const fn path(&self) -> &SafeVectorPath {
        &self.path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeVectorDraw {
    transform: SafeVectorTransform,
    clips: Vec<SafeVectorClipUse>,
    path: SafeVectorPath,
    fill: Option<[u8; 3]>,
    stroke: Option<SafeVectorStroke>,
    fill_rule: SafeVectorFillRule,
}

impl SafeVectorDraw {
    pub const fn transform(&self) -> SafeVectorTransform {
        self.transform
    }
    pub fn clips(&self) -> &[SafeVectorClipUse] {
        &self.clips
    }
    pub const fn path(&self) -> &SafeVectorPath {
        &self.path
    }
    pub const fn fill(&self) -> Option<[u8; 3]> {
        self.fill
    }
    pub const fn stroke(&self) -> Option<SafeVectorStroke> {
        self.stroke
    }
    pub const fn fill_rule(&self) -> SafeVectorFillRule {
        self.fill_rule
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeVectorIr {
    intrinsic_width: PositiveLength,
    intrinsic_height: PositiveLength,
    view_box: [i64; 4],
    root_scale: i32,
    clips: Vec<SafeVectorClipDefinition>,
    draws: Vec<SafeVectorDraw>,
    node_count: u64,
    stored_segment_count: u64,
    path_work: u64,
    allocation_charge: u64,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl SafeVectorIr {
    pub const fn intrinsic_width(&self) -> PositiveLength {
        self.intrinsic_width
    }
    pub const fn intrinsic_height(&self) -> PositiveLength {
        self.intrinsic_height
    }
    pub const fn view_box(&self) -> [i64; 4] {
        self.view_box
    }
    pub const fn root_scale_raw(&self) -> i32 {
        self.root_scale
    }
    pub fn clips(&self) -> &[SafeVectorClipDefinition] {
        &self.clips
    }
    pub fn draws(&self) -> &[SafeVectorDraw] {
        &self.draws
    }
    pub const fn node_count(&self) -> u64 {
        self.node_count
    }
    pub const fn stored_segment_count(&self) -> u64 {
        self.stored_segment_count
    }
    pub const fn path_work(&self) -> u64 {
        self.path_work
    }
    pub const fn allocation_charge(&self) -> u64 {
        self.allocation_charge
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SafeVectorWork {
    pub nodes: u64,
    pub path_work: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct DecodedSafeVector {
    pub ir: SafeVectorIr,
    pub work: SafeVectorWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Counts {
    nodes: u64,
    stored_segments: u64,
    commands: u64,
    source_clip_id_bytes: u64,
    max_depth: u32,
}

impl Counts {
    const fn new() -> Self {
        Self {
            nodes: 0,
            stored_segments: 5, // synthetic outer clip
            commands: 2,        // synthetic outer clip push/pop
            source_clip_id_bytes: 0,
            max_depth: 0,
        }
    }

    fn allocation_charge(self) -> Result<u64, ResourceAdmissionError> {
        self.nodes
            .checked_mul(64)
            .and_then(|value| {
                self.stored_segments
                    .checked_mul(80)
                    .and_then(|part| value.checked_add(part))
            })
            .and_then(|value| {
                self.commands
                    .checked_mul(32)
                    .and_then(|part| value.checked_add(part))
            })
            .and_then(|value| value.checked_add(self.source_clip_id_bytes))
            .ok_or(ResourceAdmissionError::InvalidSafeVector)
    }
}

#[derive(Clone, Copy)]
struct Attr<'a> {
    name: &'a str,
    value: &'a str,
}

#[derive(Clone, Copy)]
struct Attrs<'a> {
    values: [Option<Attr<'a>>; MAX_ATTRIBUTES],
    len: usize,
}

impl<'a> Attrs<'a> {
    const fn new() -> Self {
        Self {
            values: [None; MAX_ATTRIBUTES],
            len: 0,
        }
    }

    fn push(&mut self, attr: Attr<'a>) -> Result<(), ResourceAdmissionError> {
        if self.len == MAX_ATTRIBUTES
            || self.values[..self.len]
                .iter()
                .flatten()
                .any(|existing| existing.name == attr.name)
        {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        self.values[self.len] = Some(attr);
        self.len += 1;
        Ok(())
    }

    fn get(self, name: &str) -> Option<&'a str> {
        self.values[..self.len]
            .iter()
            .flatten()
            .find(|attr| attr.name == name)
            .map(|attr| attr.value)
    }

    fn names(self) -> impl Iterator<Item = &'a str> {
        self.values
            .into_iter()
            .take(self.len)
            .flatten()
            .map(|attr| attr.name)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TagKind {
    Start,
    End,
    Empty,
}

#[derive(Clone, Copy)]
struct Tag<'a> {
    kind: TagKind,
    name: &'a str,
    attrs: Attrs<'a>,
}

struct MarkupScanner<'a> {
    source: &'a str,
    cursor: usize,
}

impl<'a> MarkupScanner<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, ResourceAdmissionError> {
        let source =
            std::str::from_utf8(bytes).map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
        if source.starts_with('\u{feff}')
            || bytes.iter().any(|byte| {
                *byte == 0
                    || *byte == b'\r'
                    || (*byte < 0x20 && !matches!(*byte, b' ' | b'\t' | b'\n'))
                    || (0x7f..=0x9f).contains(byte)
            })
            || source.contains("<!")
            || source.contains("<?")
            || source.contains('&')
        {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        Ok(Self { source, cursor: 0 })
    }

    fn next(&mut self) -> Result<Option<Tag<'a>>, ResourceAdmissionError> {
        let bytes = self.source.as_bytes();
        while self.cursor < bytes.len() && is_wsp(bytes[self.cursor]) {
            self.cursor += 1;
        }
        if self.cursor == bytes.len() {
            return Ok(None);
        }
        if bytes[self.cursor] != b'<' {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        self.cursor += 1;
        if bytes.get(self.cursor) == Some(&b'/') {
            self.cursor += 1;
            let name = self.read_name()?;
            if bytes.get(self.cursor) != Some(&b'>') {
                return Err(ResourceAdmissionError::InvalidSafeVector);
            }
            self.cursor += 1;
            return Ok(Some(Tag {
                kind: TagKind::End,
                name,
                attrs: Attrs::new(),
            }));
        }
        let name = self.read_name()?;
        let mut attrs = Attrs::new();
        loop {
            match bytes.get(self.cursor) {
                Some(b'>') => {
                    self.cursor += 1;
                    return Ok(Some(Tag {
                        kind: TagKind::Start,
                        name,
                        attrs,
                    }));
                }
                Some(b'/') if bytes.get(self.cursor + 1) == Some(&b'>') => {
                    self.cursor += 2;
                    return Ok(Some(Tag {
                        kind: TagKind::Empty,
                        name,
                        attrs,
                    }));
                }
                Some(byte) if is_wsp(*byte) => {
                    self.consume_sep();
                    // Whitespace before a closing delimiter is not admitted.
                    if matches!(bytes.get(self.cursor), Some(b'>') | Some(b'/')) {
                        return Err(ResourceAdmissionError::InvalidSafeVector);
                    }
                    let attr_name = self.read_name()?;
                    if bytes.get(self.cursor) != Some(&b'=') {
                        return Err(ResourceAdmissionError::InvalidSafeVector);
                    }
                    self.cursor += 1;
                    let quote = *bytes
                        .get(self.cursor)
                        .filter(|quote| matches!(quote, b'\'' | b'"'))
                        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                    self.cursor += 1;
                    let start = self.cursor;
                    while bytes.get(self.cursor).is_some_and(|byte| *byte != quote) {
                        if matches!(bytes[self.cursor], b'<' | b'>') {
                            return Err(ResourceAdmissionError::InvalidSafeVector);
                        }
                        self.cursor += 1;
                    }
                    if bytes.get(self.cursor) != Some(&quote) {
                        return Err(ResourceAdmissionError::InvalidSafeVector);
                    }
                    let value = &self.source[start..self.cursor];
                    self.cursor += 1;
                    attrs.push(Attr {
                        name: attr_name,
                        value,
                    })?;
                }
                _ => return Err(ResourceAdmissionError::InvalidSafeVector),
            }
        }
    }

    fn read_name(&mut self) -> Result<&'a str, ResourceAdmissionError> {
        let bytes = self.source.as_bytes();
        let start = self.cursor;
        while bytes
            .get(self.cursor)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            self.cursor += 1;
        }
        if self.cursor == start {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        Ok(&self.source[start..self.cursor])
    }

    fn consume_sep(&mut self) {
        while self
            .source
            .as_bytes()
            .get(self.cursor)
            .is_some_and(|byte| is_wsp(*byte))
        {
            self.cursor += 1;
        }
    }
}

const fn is_wsp(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n')
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Decimal {
    numerator: i128,
    denominator: i128,
}

fn decimal(value: &str) -> Result<Decimal, ResourceAdmissionError> {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.iter().any(|byte| is_wsp(*byte)) {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let mut cursor = 0usize;
    let negative = bytes.first() == Some(&b'-');
    if negative {
        cursor = 1;
    }
    let integer_start = cursor;
    if bytes.get(cursor) == Some(&b'0') {
        cursor += 1;
        if bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
    } else if bytes
        .get(cursor)
        .is_some_and(|byte| matches!(byte, b'1'..=b'9'))
    {
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
    } else {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    if cursor - integer_start > 12 {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let mut scale = 0usize;
    if bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        let fraction_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        scale = cursor - fraction_start;
        if !(1..=6).contains(&scale) {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
    }
    if cursor != bytes.len() {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let digits: i128 = value
        .trim_start_matches('-')
        .chars()
        .filter(|character| *character != '.')
        .try_fold(0i128, |result, character| {
            result
                .checked_mul(10)?
                .checked_add(i128::from(character.to_digit(10)?))
        })
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let denominator = 10i128
        .checked_pow(u32::try_from(scale).map_err(|_| ResourceAdmissionError::InvalidSafeVector)?)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let numerator = if negative { -digits } else { digits };
    if numerator.unsigned_abs()
        > u128::try_from(1_000_000i128 * denominator)
            .map_err(|_| ResourceAdmissionError::InvalidSafeVector)?
    {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(Decimal {
        numerator,
        denominator,
    })
}

fn decimal_fixed(value: &str) -> Result<i64, ResourceAdmissionError> {
    let value = decimal(value)?;
    let scaled = value
        .numerator
        .checked_mul(i128::from(FIXED_ONE))
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let result = round_ties_even(scaled, value.denominator)?;
    let result = i64::try_from(result).map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
    if result.abs() > MAX_COORDINATE {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(result)
}

fn round_ties_even(numerator: i128, denominator: i128) -> Result<i128, ResourceAdmissionError> {
    if denominator <= 0 {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder == 0 {
        return Ok(quotient);
    }
    let twice = remainder
        .unsigned_abs()
        .checked_mul(2)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let denominator =
        u128::try_from(denominator).map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
    let step = if remainder > 0 { 1 } else { -1 };
    if twice < denominator || (twice == denominator && quotient % 2 == 0) {
        Ok(quotient)
    } else {
        quotient
            .checked_add(step)
            .ok_or(ResourceAdmissionError::InvalidSafeVector)
    }
}

fn fixed_mul(left: i64, right: i64) -> Result<i64, ResourceAdmissionError> {
    let product = i128::from(left)
        .checked_mul(i128::from(right))
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let value = i64::try_from(round_ties_even(product, i128::from(FIXED_ONE))?)
        .map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
    if value.abs() > MAX_COORDINATE {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(value)
}

fn fixed_ratio(numerator: i64, denominator: i64) -> Result<i32, ResourceAdmissionError> {
    if denominator == 0 {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let value = round_ties_even(
        i128::from(numerator)
            .checked_mul(i128::from(FIXED_ONE))
            .ok_or(ResourceAdmissionError::InvalidSafeVector)?,
        i128::from(denominator),
    )?;
    i32::try_from(value).map_err(|_| ResourceAdmissionError::InvalidSafeVector)
}

fn transform_point(
    transform: SafeVectorTransform,
    point: SafeVectorPoint,
) -> Result<SafeVectorPoint, ResourceAdmissionError> {
    let x = fixed_mul(i64::from(transform.a), point.x)?
        .checked_add(transform.e)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let y = fixed_mul(i64::from(transform.d), point.y)?
        .checked_add(transform.f)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    if x.abs() > MAX_COORDINATE || y.abs() > MAX_COORDINATE {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(SafeVectorPoint { x, y })
}

fn compose(
    left: SafeVectorTransform,
    right: SafeVectorTransform,
) -> Result<SafeVectorTransform, ResourceAdmissionError> {
    let a = fixed_mul(i64::from(left.a), i64::from(right.a))?;
    let d = fixed_mul(i64::from(left.d), i64::from(right.d))?;
    let e = fixed_mul(i64::from(left.a), right.e)?
        .checked_add(left.e)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let f = fixed_mul(i64::from(left.d), right.f)?
        .checked_add(left.f)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let a = i32::try_from(a).map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
    let d = i32::try_from(d).map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
    if a == 0 || d == 0 || e.abs() > MAX_COORDINATE || f.abs() > MAX_COORDINATE {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(SafeVectorTransform { a, d, e, f })
}

fn parse_transform(value: Option<&str>) -> Result<SafeVectorTransform, ResourceAdmissionError> {
    let Some(value) = value else {
        return Ok(SafeVectorTransform::IDENTITY);
    };
    if value.is_empty()
        || value.as_bytes().first().is_some_and(|byte| is_wsp(*byte))
        || value.as_bytes().last().is_some_and(|byte| is_wsp(*byte))
    {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let mut result = SafeVectorTransform::IDENTITY;
    let mut rest = value;
    loop {
        let close = rest
            .find(')')
            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
        let function = &rest[..=close];
        let (name, args) = function[..function.len() - 1]
            .split_once('(')
            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
        if name.is_empty() || args.is_empty() {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        let values = parse_fixed_list(args, 6)?;
        if name == "matrix" && !matrix_off_diagonals_are_exact_zero(args)? {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        let next = match (name, values.as_slice()) {
            ("translate", [tx]) => SafeVectorTransform {
                e: *tx,
                ..SafeVectorTransform::IDENTITY
            },
            ("translate", [tx, ty]) => SafeVectorTransform {
                e: *tx,
                f: *ty,
                ..SafeVectorTransform::IDENTITY
            },
            ("scale", [sx]) => scale_transform(*sx, *sx)?,
            ("scale", [sx, sy]) => scale_transform(*sx, *sy)?,
            ("matrix", [a, b, c, d, e, f]) if *b == 0 && *c == 0 => {
                let a = i32::try_from(*a).map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
                let d = i32::try_from(*d).map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
                if a == 0 || d == 0 {
                    return Err(ResourceAdmissionError::InvalidSafeVector);
                }
                SafeVectorTransform { a, d, e: *e, f: *f }
            }
            _ => return Err(ResourceAdmissionError::InvalidSafeVector),
        };
        result = compose(result, next)?;
        if close + 1 == rest.len() {
            break;
        }
        if !rest
            .as_bytes()
            .get(close + 1)
            .is_some_and(|byte| is_wsp(*byte))
        {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        let mut next_start = close + 1;
        while rest
            .as_bytes()
            .get(next_start)
            .is_some_and(|byte| is_wsp(*byte))
        {
            next_start += 1;
        }
        if next_start == rest.len() {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        rest = &rest[next_start..];
    }
    Ok(result)
}

fn matrix_off_diagonals_are_exact_zero(args: &str) -> Result<bool, ResourceAdmissionError> {
    let mut values = args.split_ascii_whitespace();
    let _a = values
        .next()
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let b = values
        .next()
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let c = values
        .next()
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    for _ in 0..3 {
        values
            .next()
            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    }
    if values.next().is_some() {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(decimal(b)?.numerator == 0 && decimal(c)?.numerator == 0)
}

fn scale_transform(sx: i64, sy: i64) -> Result<SafeVectorTransform, ResourceAdmissionError> {
    let a = i32::try_from(sx).map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
    let d = i32::try_from(sy).map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
    if a == 0 || d == 0 {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(SafeVectorTransform { a, d, e: 0, f: 0 })
}

#[derive(Clone, Copy)]
struct FixedList {
    values: [i64; 6],
    len: usize,
}

impl FixedList {
    fn as_slice(&self) -> &[i64] {
        &self.values[..self.len]
    }
}

fn parse_fixed_list(value: &str, maximum: usize) -> Result<FixedList, ResourceAdmissionError> {
    if value.is_empty()
        || value.as_bytes().first().is_some_and(|byte| is_wsp(*byte))
        || value.as_bytes().last().is_some_and(|byte| is_wsp(*byte))
        || value.contains(',')
    {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    if maximum > 6 {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let mut output = FixedList {
        values: [0; 6],
        len: 0,
    };
    for token in value.split_ascii_whitespace() {
        if token.is_empty() || output.len == maximum {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        output.values[output.len] = decimal_fixed(token)?;
        output.len += 1;
    }
    Ok(output)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PaintState {
    fill: Option<[u8; 3]>,
    stroke: Option<[u8; 3]>,
    stroke_width: i64,
    fill_rule: SafeVectorFillRule,
    line_cap: SafeVectorLineCap,
    line_join: SafeVectorLineJoin,
    miter_limit: i64,
}

impl Default for PaintState {
    fn default() -> Self {
        Self {
            fill: Some([0, 0, 0]),
            stroke: None,
            stroke_width: FIXED_ONE,
            fill_rule: SafeVectorFillRule::NonZero,
            line_cap: SafeVectorLineCap::Butt,
            line_join: SafeVectorLineJoin::Miter,
            miter_limit: 4 * FIXED_ONE,
        }
    }
}

fn inherit_paint(
    mut state: PaintState,
    attrs: Attrs<'_>,
) -> Result<PaintState, ResourceAdmissionError> {
    if let Some(value) = attrs.get("fill") {
        state.fill = parse_color(value)?;
    }
    if let Some(value) = attrs.get("stroke") {
        state.stroke = parse_color(value)?;
    }
    if let Some(value) = attrs.get("stroke-width") {
        state.stroke_width = positive_fixed(value)?;
    }
    if let Some(value) = attrs.get("fill-rule") {
        state.fill_rule = match value {
            "nonzero" => SafeVectorFillRule::NonZero,
            "evenodd" => SafeVectorFillRule::EvenOdd,
            _ => return Err(ResourceAdmissionError::InvalidSafeVector),
        };
    }
    if let Some(value) = attrs.get("stroke-linecap") {
        state.line_cap = match value {
            "butt" => SafeVectorLineCap::Butt,
            "round" => SafeVectorLineCap::Round,
            "square" => SafeVectorLineCap::Square,
            _ => return Err(ResourceAdmissionError::InvalidSafeVector),
        };
    }
    if let Some(value) = attrs.get("stroke-linejoin") {
        state.line_join = match value {
            "miter" => SafeVectorLineJoin::Miter,
            "round" => SafeVectorLineJoin::Round,
            "bevel" => SafeVectorLineJoin::Bevel,
            _ => return Err(ResourceAdmissionError::InvalidSafeVector),
        };
    }
    if let Some(value) = attrs.get("stroke-miterlimit") {
        state.miter_limit = decimal_fixed(value)?;
        if state.miter_limit < FIXED_ONE {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
    }
    Ok(state)
}

fn parse_color(value: &str) -> Result<Option<[u8; 3]>, ResourceAdmissionError> {
    if value == "none" {
        return Ok(None);
    }
    let bytes = value.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let mut color = [0u8; 3];
    for (index, pair) in bytes[1..].chunks_exact(2).enumerate() {
        color[index] = hex(pair[0])?
            .checked_mul(16)
            .and_then(|high| high.checked_add(hex(pair[1]).ok()?))
            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    }
    Ok(Some(color))
}

fn hex(byte: u8) -> Result<u8, ResourceAdmissionError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ResourceAdmissionError::InvalidSafeVector),
    }
}

fn positive_fixed(value: &str) -> Result<i64, ResourceAdmissionError> {
    let value = decimal_fixed(value)?;
    (value > 0)
        .then_some(value)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Bounds {
    min_x: i64,
    min_y: i64,
    max_x: i64,
    max_y: i64,
    initialized: bool,
    drawable: bool,
}

impl Bounds {
    const fn new() -> Self {
        Self {
            min_x: 0,
            min_y: 0,
            max_x: 0,
            max_y: 0,
            initialized: false,
            drawable: false,
        }
    }

    fn point(&mut self, point: SafeVectorPoint) {
        if !self.initialized {
            self.min_x = point.x;
            self.max_x = point.x;
            self.min_y = point.y;
            self.max_y = point.y;
            self.initialized = true;
        } else {
            self.min_x = self.min_x.min(point.x);
            self.max_x = self.max_x.max(point.x);
            self.min_y = self.min_y.min(point.y);
            self.max_y = self.max_y.max(point.y);
        }
    }

    fn positive_area(self) -> bool {
        self.initialized && self.min_x < self.max_x && self.min_y < self.max_y
    }
}

fn segment_points(
    segment: &SafeVectorSegment,
    transform: SafeVectorTransform,
    bounds: &mut Bounds,
    current: &mut Option<SafeVectorPoint>,
) -> Result<(), ResourceAdmissionError> {
    match segment {
        SafeVectorSegment::Move(point) => {
            let point = transform_point(transform, *point)?;
            bounds.point(point);
            *current = Some(point);
        }
        SafeVectorSegment::Line(point) => {
            let point = transform_point(transform, *point)?;
            bounds.drawable |= current.is_some_and(|current| current != point);
            bounds.point(point);
            *current = Some(point);
        }
        SafeVectorSegment::Quadratic(control, point) => {
            let control = transform_point(transform, *control)?;
            let point = transform_point(transform, *point)?;
            bounds.drawable |=
                current.is_some_and(|current| current != control || current != point);
            bounds.point(control);
            bounds.point(point);
            *current = Some(point);
        }
        SafeVectorSegment::Cubic(first, second, point) => {
            let first = transform_point(transform, *first)?;
            let second = transform_point(transform, *second)?;
            let point = transform_point(transform, *point)?;
            bounds.drawable |= current
                .is_some_and(|current| current != first || current != second || current != point);
            bounds.point(first);
            bounds.point(second);
            bounds.point(point);
            *current = Some(point);
        }
        SafeVectorSegment::Close => {}
    }
    Ok(())
}

fn validate_path_geometry(
    path: &SafeVectorPath,
    transform: SafeVectorTransform,
    require_area: bool,
) -> Result<(), ResourceAdmissionError> {
    let mut bounds = Bounds::new();
    let mut current = None;
    for segment in path.segments() {
        segment_points(segment, transform, &mut bounds, &mut current)?;
    }
    if !bounds.drawable || (require_area && !bounds.positive_area()) {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct PathTokenCursor<'a> {
    value: &'a str,
    cursor: usize,
}

impl<'a> PathTokenCursor<'a> {
    fn new(value: &'a str) -> Result<Self, ResourceAdmissionError> {
        if value.is_empty()
            || value.as_bytes().first().is_some_and(|byte| is_wsp(*byte))
            || value.as_bytes().last().is_some_and(|byte| is_wsp(*byte))
            || value.contains(',')
        {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        Ok(Self { value, cursor: 0 })
    }

    fn peek(&self) -> Option<&'a str> {
        if self.cursor == self.value.len() {
            return None;
        }
        let tail = &self.value[self.cursor..];
        let end = tail
            .as_bytes()
            .iter()
            .position(|byte| is_wsp(*byte))
            .unwrap_or(tail.len());
        Some(&tail[..end])
    }

    fn next(&mut self) -> Option<&'a str> {
        let token = self.peek()?;
        self.cursor += token.len();
        while self
            .value
            .as_bytes()
            .get(self.cursor)
            .is_some_and(|byte| is_wsp(*byte))
        {
            self.cursor += 1;
        }
        Some(token)
    }
}

fn is_path_command(token: &str) -> bool {
    token.len() == 1
        && matches!(
            token.as_bytes()[0],
            b'M' | b'm'
                | b'L'
                | b'l'
                | b'H'
                | b'h'
                | b'V'
                | b'v'
                | b'Q'
                | b'q'
                | b'C'
                | b'c'
                | b'Z'
                | b'z'
        )
}

fn record_segment(
    emit: &mut impl FnMut(SafeVectorSegment) -> Result<(), ResourceAdmissionError>,
    segment: SafeVectorSegment,
    count: &mut u64,
    has_drawable: &mut bool,
) -> Result<(), ResourceAdmissionError> {
    *count = count
        .checked_add(1)
        .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?;
    *has_drawable |= !matches!(
        segment,
        SafeVectorSegment::Move(_) | SafeVectorSegment::Close
    );
    emit(segment)
}

fn visit_path_data(
    value: &str,
    mut emit: impl FnMut(SafeVectorSegment) -> Result<(), ResourceAdmissionError>,
) -> Result<u64, ResourceAdmissionError> {
    let mut cursor = PathTokenCursor::new(value)?;
    let mut segment_count = 0u64;
    let mut has_drawable = false;
    let mut current = SafeVectorPoint { x: 0, y: 0 };
    let mut subpath = current;
    let mut first_command = true;
    let mut after_close = false;
    while let Some(command_token) = cursor.next() {
        if !is_path_command(command_token) {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        let command = command_token.as_bytes()[0];
        if first_command && !matches!(command, b'M' | b'm') {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        if after_close && !matches!(command, b'M' | b'm') {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        first_command = false;
        after_close = false;
        if matches!(command, b'Z' | b'z') {
            record_segment(
                &mut emit,
                SafeVectorSegment::Close,
                &mut segment_count,
                &mut has_drawable,
            )?;
            current = subpath;
            after_close = true;
            continue;
        }
        let arity = match command.to_ascii_uppercase() {
            b'M' | b'L' => 2,
            b'H' | b'V' => 1,
            b'Q' => 4,
            b'C' => 6,
            _ => return Err(ResourceAdmissionError::InvalidSafeVector),
        };
        let relative = command.is_ascii_lowercase();
        let mut group = 0usize;
        while cursor.peek().is_some_and(|token| !is_path_command(token)) {
            let mut values = [0i64; 6];
            for slot in values.iter_mut().take(arity) {
                let token = cursor
                    .next()
                    .filter(|token| !is_path_command(token))
                    .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                *slot = decimal_fixed(token)?;
            }
            let resolve = |x: i64, y: i64, base: SafeVectorPoint| {
                if relative {
                    Ok(SafeVectorPoint {
                        x: base
                            .x
                            .checked_add(x)
                            .filter(|value| value.abs() <= MAX_COORDINATE)
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?,
                        y: base
                            .y
                            .checked_add(y)
                            .filter(|value| value.abs() <= MAX_COORDINATE)
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?,
                    })
                } else {
                    Ok(SafeVectorPoint { x, y })
                }
            };
            match command.to_ascii_uppercase() {
                b'M' => {
                    let point = resolve(values[0], values[1], current)?;
                    if group == 0 {
                        record_segment(
                            &mut emit,
                            SafeVectorSegment::Move(point),
                            &mut segment_count,
                            &mut has_drawable,
                        )?;
                        subpath = point;
                    } else {
                        record_segment(
                            &mut emit,
                            SafeVectorSegment::Line(point),
                            &mut segment_count,
                            &mut has_drawable,
                        )?;
                    }
                    current = point;
                }
                b'L' => {
                    let point = resolve(values[0], values[1], current)?;
                    record_segment(
                        &mut emit,
                        SafeVectorSegment::Line(point),
                        &mut segment_count,
                        &mut has_drawable,
                    )?;
                    current = point;
                }
                b'H' => {
                    let x = if relative {
                        current
                            .x
                            .checked_add(values[0])
                            .filter(|value| value.abs() <= MAX_COORDINATE)
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?
                    } else {
                        values[0]
                    };
                    current = SafeVectorPoint { x, y: current.y };
                    record_segment(
                        &mut emit,
                        SafeVectorSegment::Line(current),
                        &mut segment_count,
                        &mut has_drawable,
                    )?;
                }
                b'V' => {
                    let y = if relative {
                        current
                            .y
                            .checked_add(values[0])
                            .filter(|value| value.abs() <= MAX_COORDINATE)
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?
                    } else {
                        values[0]
                    };
                    current = SafeVectorPoint { x: current.x, y };
                    record_segment(
                        &mut emit,
                        SafeVectorSegment::Line(current),
                        &mut segment_count,
                        &mut has_drawable,
                    )?;
                }
                b'Q' => {
                    let control = resolve(values[0], values[1], current)?;
                    let point = resolve(values[2], values[3], current)?;
                    record_segment(
                        &mut emit,
                        SafeVectorSegment::Quadratic(control, point),
                        &mut segment_count,
                        &mut has_drawable,
                    )?;
                    current = point;
                }
                b'C' => {
                    let first = resolve(values[0], values[1], current)?;
                    let second = resolve(values[2], values[3], current)?;
                    let point = resolve(values[4], values[5], current)?;
                    record_segment(
                        &mut emit,
                        SafeVectorSegment::Cubic(first, second, point),
                        &mut segment_count,
                        &mut has_drawable,
                    )?;
                    current = point;
                }
                _ => unreachable!(),
            }
            group += 1;
        }
        if group == 0 {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
    }
    if segment_count == 0 || !has_drawable {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(segment_count)
}

fn parse_path(value: &str) -> Result<SafeVectorPath, ResourceAdmissionError> {
    let mut segments = Vec::new();
    let count = visit_path_data(value, |segment| {
        if segments.len() == segments.capacity() {
            segments
                .try_reserve(1)
                .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        }
        segments.push(segment);
        Ok(())
    })?;
    if u64::try_from(segments.len()) != Ok(count) {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(SafeVectorPath { segments })
}

fn shape_path(name: &str, attrs: Attrs<'_>) -> Result<SafeVectorPath, ResourceAdmissionError> {
    let point = |x: i64, y: i64| SafeVectorPoint { x, y };
    let required = |name| {
        attrs
            .get(name)
            .ok_or(ResourceAdmissionError::InvalidSafeVector)
    };
    let optional = |name| attrs.get(name).map_or(Ok(0), decimal_fixed);
    let path = match name {
        "path" => parse_path(required("d")?)?,
        "rect" => {
            let x = optional("x")?;
            let y = optional("y")?;
            let width = positive_fixed(required("width")?)?;
            let height = positive_fixed(required("height")?)?;
            SafeVectorPath {
                segments: vec![
                    SafeVectorSegment::Move(point(x, y)),
                    SafeVectorSegment::Line(point(x + width, y)),
                    SafeVectorSegment::Line(point(x + width, y + height)),
                    SafeVectorSegment::Line(point(x, y + height)),
                    SafeVectorSegment::Close,
                ],
            }
        }
        "circle" => {
            let cx = optional("cx")?;
            let cy = optional("cy")?;
            let radius = positive_fixed(required("r")?)?;
            ellipse_path(cx, cy, radius, radius)?
        }
        "ellipse" => {
            let cx = optional("cx")?;
            let cy = optional("cy")?;
            let rx = positive_fixed(required("rx")?)?;
            let ry = positive_fixed(required("ry")?)?;
            ellipse_path(cx, cy, rx, ry)?
        }
        "line" => {
            let x1 = decimal_fixed(required("x1")?)?;
            let y1 = decimal_fixed(required("y1")?)?;
            let x2 = decimal_fixed(required("x2")?)?;
            let y2 = decimal_fixed(required("y2")?)?;
            SafeVectorPath {
                segments: vec![
                    SafeVectorSegment::Move(point(x1, y1)),
                    SafeVectorSegment::Line(point(x2, y2)),
                ],
            }
        }
        "polyline" | "polygon" => {
            let values = parse_unbounded_fixed_list(required("points")?)?;
            let minimum = if name == "polygon" { 6 } else { 4 };
            if values.len() < minimum || values.len() % 2 != 0 {
                return Err(ResourceAdmissionError::InvalidSafeVector);
            }
            let mut segments = Vec::new();
            segments
                .try_reserve_exact(values.len() / 2 + usize::from(name == "polygon"))
                .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
            for (index, pair) in values.chunks_exact(2).enumerate() {
                let point = point(pair[0], pair[1]);
                segments.push(if index == 0 {
                    SafeVectorSegment::Move(point)
                } else {
                    SafeVectorSegment::Line(point)
                });
            }
            if name == "polygon" {
                segments.push(SafeVectorSegment::Close);
            }
            SafeVectorPath { segments }
        }
        _ => return Err(ResourceAdmissionError::InvalidSafeVector),
    };
    Ok(path)
}

fn parse_unbounded_fixed_list(value: &str) -> Result<Vec<i64>, ResourceAdmissionError> {
    if value.is_empty()
        || value.as_bytes().first().is_some_and(|byte| is_wsp(*byte))
        || value.as_bytes().last().is_some_and(|byte| is_wsp(*byte))
        || value.contains(',')
    {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    value
        .split_ascii_whitespace()
        .map(|token| {
            if token.is_empty() {
                Err(ResourceAdmissionError::InvalidSafeVector)
            } else {
                decimal_fixed(token)
            }
        })
        .collect()
}

fn ellipse_segments(
    cx: i64,
    cy: i64,
    rx: i64,
    ry: i64,
) -> Result<[SafeVectorSegment; 6], ResourceAdmissionError> {
    let kx = fixed_mul(rx, CIRCLE_CONTROL_RATIO)?;
    let ky = fixed_mul(ry, CIRCLE_CONTROL_RATIO)?;
    let add = |a: i64, b: i64| {
        a.checked_add(b)
            .filter(|value| value.abs() <= MAX_COORDINATE)
            .ok_or(ResourceAdmissionError::InvalidSafeVector)
    };
    let sub = |a: i64, b: i64| {
        a.checked_sub(b)
            .filter(|value| value.abs() <= MAX_COORDINATE)
            .ok_or(ResourceAdmissionError::InvalidSafeVector)
    };
    let right = add(cx, rx)?;
    let left = sub(cx, rx)?;
    let bottom = add(cy, ry)?;
    let top = sub(cy, ry)?;
    Ok([
        SafeVectorSegment::Move(SafeVectorPoint { x: right, y: cy }),
        SafeVectorSegment::Cubic(
            SafeVectorPoint {
                x: right,
                y: add(cy, ky)?,
            },
            SafeVectorPoint {
                x: add(cx, kx)?,
                y: bottom,
            },
            SafeVectorPoint { x: cx, y: bottom },
        ),
        SafeVectorSegment::Cubic(
            SafeVectorPoint {
                x: sub(cx, kx)?,
                y: bottom,
            },
            SafeVectorPoint {
                x: left,
                y: add(cy, ky)?,
            },
            SafeVectorPoint { x: left, y: cy },
        ),
        SafeVectorSegment::Cubic(
            SafeVectorPoint {
                x: left,
                y: sub(cy, ky)?,
            },
            SafeVectorPoint {
                x: sub(cx, kx)?,
                y: top,
            },
            SafeVectorPoint { x: cx, y: top },
        ),
        SafeVectorSegment::Cubic(
            SafeVectorPoint {
                x: add(cx, kx)?,
                y: top,
            },
            SafeVectorPoint {
                x: right,
                y: sub(cy, ky)?,
            },
            SafeVectorPoint { x: right, y: cy },
        ),
        SafeVectorSegment::Close,
    ])
}

fn ellipse_path(
    cx: i64,
    cy: i64,
    rx: i64,
    ry: i64,
) -> Result<SafeVectorPath, ResourceAdmissionError> {
    Ok(SafeVectorPath {
        segments: Vec::from(ellipse_segments(cx, cy, rx, ry)?),
    })
}

/// First-pass geometry validation and exact segment counting. This deliberately
/// visits one stack-owned segment at a time so an untrusted path or point list
/// cannot allocate before its node/segment/allocation permits are known.
fn validate_shape_without_allocation(
    name: &str,
    attrs: Attrs<'_>,
    transform: SafeVectorTransform,
    physical_transform: SafeVectorTransform,
    require_area: bool,
    require_closed: bool,
    segment_limit: Option<u64>,
) -> Result<u64, ResourceAdmissionError> {
    let required = |name| {
        attrs
            .get(name)
            .ok_or(ResourceAdmissionError::InvalidSafeVector)
    };
    let optional = |name| attrs.get(name).map_or(Ok(0), decimal_fixed);
    let mut bounds = Bounds::new();
    let mut current = None;
    let mut subpath = None;
    let mut physical_bounds = Bounds::new();
    let mut physical_current = None;
    let mut physical_subpath = None;
    let mut open = false;
    let mut observed_segments = 0u64;
    let mut observe = |segment: SafeVectorSegment| {
        observed_segments = observed_segments
            .checked_add(1)
            .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?;
        if segment_limit.is_some_and(|maximum| observed_segments > maximum) {
            return Err(ResourceAdmissionError::VectorPathSegmentLimit);
        }
        match &segment {
            SafeVectorSegment::Move(point) => {
                if require_closed && open {
                    return Err(ResourceAdmissionError::InvalidSafeVector);
                }
                open = true;
                subpath = Some(transform_point(transform, *point)?);
                physical_subpath = Some(transform_point(physical_transform, *point)?);
                segment_points(&segment, transform, &mut bounds, &mut current)?;
                segment_points(
                    &segment,
                    physical_transform,
                    &mut physical_bounds,
                    &mut physical_current,
                )?;
            }
            SafeVectorSegment::Close => {
                if !open {
                    return Err(ResourceAdmissionError::InvalidSafeVector);
                }
                open = false;
                current = subpath;
                physical_current = physical_subpath;
            }
            _ => {
                if !open {
                    return Err(ResourceAdmissionError::InvalidSafeVector);
                }
                segment_points(&segment, transform, &mut bounds, &mut current)?;
                segment_points(
                    &segment,
                    physical_transform,
                    &mut physical_bounds,
                    &mut physical_current,
                )?;
            }
        }
        Ok(())
    };

    let count = match name {
        "path" => visit_path_data(required("d")?, &mut observe)?,
        "rect" => {
            let x = optional("x")?;
            let y = optional("y")?;
            let width = positive_fixed(required("width")?)?;
            let height = positive_fixed(required("height")?)?;
            let right = x
                .checked_add(width)
                .filter(|value| value.abs() <= MAX_COORDINATE)
                .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
            let bottom = y
                .checked_add(height)
                .filter(|value| value.abs() <= MAX_COORDINATE)
                .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
            for segment in [
                SafeVectorSegment::Move(SafeVectorPoint { x, y }),
                SafeVectorSegment::Line(SafeVectorPoint { x: right, y }),
                SafeVectorSegment::Line(SafeVectorPoint {
                    x: right,
                    y: bottom,
                }),
                SafeVectorSegment::Line(SafeVectorPoint { x, y: bottom }),
                SafeVectorSegment::Close,
            ] {
                observe(segment)?;
            }
            5
        }
        "circle" | "ellipse" => {
            let cx = optional("cx")?;
            let cy = optional("cy")?;
            let (rx, ry) = if name == "circle" {
                let radius = positive_fixed(required("r")?)?;
                (radius, radius)
            } else {
                (
                    positive_fixed(required("rx")?)?,
                    positive_fixed(required("ry")?)?,
                )
            };
            for segment in ellipse_segments(cx, cy, rx, ry)? {
                observe(segment)?;
            }
            6
        }
        "line" => {
            let x1 = decimal_fixed(required("x1")?)?;
            let y1 = decimal_fixed(required("y1")?)?;
            let x2 = decimal_fixed(required("x2")?)?;
            let y2 = decimal_fixed(required("y2")?)?;
            observe(SafeVectorSegment::Move(SafeVectorPoint { x: x1, y: y1 }))?;
            observe(SafeVectorSegment::Line(SafeVectorPoint { x: x2, y: y2 }))?;
            2
        }
        "polyline" | "polygon" => {
            let value = required("points")?;
            if value.is_empty()
                || value.as_bytes().first().is_some_and(|byte| is_wsp(*byte))
                || value.as_bytes().last().is_some_and(|byte| is_wsp(*byte))
                || value.contains(',')
            {
                return Err(ResourceAdmissionError::InvalidSafeVector);
            }
            let mut tokens = value.split_ascii_whitespace();
            let mut point_count = 0u64;
            while let Some(x) = tokens.next() {
                if x.is_empty() {
                    return Err(ResourceAdmissionError::InvalidSafeVector);
                }
                let y = tokens
                    .next()
                    .filter(|token| !token.is_empty())
                    .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                let point = SafeVectorPoint {
                    x: decimal_fixed(x)?,
                    y: decimal_fixed(y)?,
                };
                observe(if point_count == 0 {
                    SafeVectorSegment::Move(point)
                } else {
                    SafeVectorSegment::Line(point)
                })?;
                point_count = point_count
                    .checked_add(1)
                    .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?;
            }
            let minimum = if name == "polygon" { 3 } else { 2 };
            if point_count < minimum {
                return Err(ResourceAdmissionError::InvalidSafeVector);
            }
            if name == "polygon" {
                observe(SafeVectorSegment::Close)?;
                point_count
                    .checked_add(1)
                    .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?
            } else {
                point_count
            }
        }
        _ => return Err(ResourceAdmissionError::InvalidSafeVector),
    };
    if observed_segments != count
        || (require_closed && open)
        || !bounds.drawable
        || (require_area && !bounds.positive_area())
        || !physical_bounds.drawable
        || (require_area && !physical_bounds.positive_area())
    {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(count)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContainerKind {
    Svg,
    Defs,
    ClipPath,
    Group,
}

#[derive(Clone, Copy)]
struct Frame<'a> {
    kind: ContainerKind,
    child_count: u32,
    has_paint: bool,
    transform: SafeVectorTransform,
    paint: PaintState,
    clip_ref: Option<&'a str>,
    clip_id: Option<&'a str>,
}

impl<'a> Frame<'a> {
    const EMPTY: Self = Self {
        kind: ContainerKind::Svg,
        child_count: 0,
        has_paint: false,
        transform: SafeVectorTransform::IDENTITY,
        paint: PaintState {
            fill: Some([0, 0, 0]),
            stroke: None,
            stroke_width: FIXED_ONE,
            fill_rule: SafeVectorFillRule::NonZero,
            line_cap: SafeVectorLineCap::Butt,
            line_join: SafeVectorLineJoin::Miter,
            miter_limit: 4 * FIXED_ONE,
        },
        clip_ref: None,
        clip_id: None,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RootGeometry {
    width: PositiveLength,
    height: PositiveLength,
    view_box: [i64; 4],
    root_scale: i32,
}

struct ScanResult<'a> {
    counts: Counts,
    root: RootGeometry,
    definitions: Vec<(&'a str, SafeVectorClipDefinition)>,
    draws: Vec<SafeVectorDraw>,
    references: Vec<&'a str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScanMode {
    Count,
    Analyze,
    Build,
}

#[derive(Clone, Copy)]
struct ScanLimits {
    nodes: u64,
    stored_segments: u64,
    depth: u32,
}

fn scan<'a>(
    bytes: &'a [u8],
    mode: ScanMode,
    scan_limits: Option<ScanLimits>,
) -> Result<ScanResult<'a>, ResourceAdmissionError> {
    let mut scanner = MarkupScanner::new(bytes)?;
    let mut stack = [Frame::EMPTY; HARD_STACK_DEPTH];
    let mut depth = 0usize;
    let mut counts = Counts::new();
    if scan_limits.is_some_and(|limits| counts.stored_segments > limits.stored_segments) {
        return Err(ResourceAdmissionError::VectorPathSegmentLimit);
    }
    let mut root = None;
    let mut definitions = Vec::new();
    let mut draws = Vec::new();
    let mut references = Vec::new();
    let mut root_closed = false;

    while let Some(tag) = scanner.next()? {
        if root_closed {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        match tag.kind {
            TagKind::Start => {
                let kind = match tag.name {
                    "svg" => ContainerKind::Svg,
                    "defs" => ContainerKind::Defs,
                    "clipPath" => ContainerKind::ClipPath,
                    "g" => ContainerKind::Group,
                    _ => return Err(ResourceAdmissionError::InvalidSafeVector),
                };
                if depth == HARD_STACK_DEPTH {
                    return Err(ResourceAdmissionError::VectorNestingLimit);
                }
                let next_depth = u32::try_from(depth + 1)
                    .map_err(|_| ResourceAdmissionError::VectorNestingLimit)?;
                counts.max_depth = counts.max_depth.max(next_depth);
                if scan_limits.is_some_and(|limits| counts.max_depth > limits.depth) {
                    return Err(ResourceAdmissionError::VectorNestingLimit);
                }
                counts.nodes = counts
                    .nodes
                    .checked_add(1)
                    .ok_or(ResourceAdmissionError::VectorNodeLimit)?;
                if scan_limits.is_some_and(|limits| counts.nodes > limits.nodes) {
                    return Err(ResourceAdmissionError::VectorNodeLimit);
                }
                let parent = depth.checked_sub(1).map(|index| stack[index]);
                let frame = match kind {
                    ContainerKind::Svg => {
                        if depth != 0 || root.is_some() || tag.name != "svg" {
                            return Err(ResourceAdmissionError::InvalidSafeVector);
                        }
                        let geometry = parse_root(tag.attrs)?;
                        root = Some(geometry);
                        Frame {
                            kind,
                            transform: SafeVectorTransform::IDENTITY,
                            paint: PaintState::default(),
                            ..Frame::EMPTY
                        }
                    }
                    ContainerKind::Defs => {
                        let parent = parent
                            .filter(|parent| {
                                parent.kind == ContainerKind::Svg && parent.child_count == 0
                            })
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                        require_names(tag.attrs, &[])?;
                        Frame {
                            kind,
                            transform: parent.transform,
                            paint: parent.paint,
                            ..Frame::EMPTY
                        }
                    }
                    ContainerKind::ClipPath => {
                        let parent = parent
                            .filter(|parent| parent.kind == ContainerKind::Defs)
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                        require_names(tag.attrs, &["id"])?;
                        let id = tag
                            .attrs
                            .get("id")
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                        validate_id(id)?;
                        counts.source_clip_id_bytes = counts
                            .source_clip_id_bytes
                            .checked_add(id.len() as u64)
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                        Frame {
                            kind,
                            transform: parent.transform,
                            paint: parent.paint,
                            clip_id: Some(id),
                            ..Frame::EMPTY
                        }
                    }
                    ContainerKind::Group => {
                        let parent = parent
                            .filter(|parent| {
                                matches!(parent.kind, ContainerKind::Svg | ContainerKind::Group)
                            })
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                        validate_shared_attrs(tag.attrs, false, false)?;
                        let local = parse_transform(tag.attrs.get("transform"))?;
                        let transform = compose(parent.transform, local)?;
                        let paint = inherit_paint(parent.paint, tag.attrs)?;
                        let clip_ref = parse_clip_ref(tag.attrs.get("clip-path"))?;
                        if let Some(id) = clip_ref {
                            counts.source_clip_id_bytes = counts
                                .source_clip_id_bytes
                                .checked_add(id.len() as u64)
                                .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                            counts.commands = counts
                                .commands
                                .checked_add(2)
                                .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                            if mode != ScanMode::Count {
                                references.push(id);
                            }
                            if mode == ScanMode::Analyze {
                                resolve_clip_use(
                                    &definitions,
                                    id,
                                    transform,
                                    root.ok_or(ResourceAdmissionError::InvalidSafeVector)?,
                                )?;
                            }
                        }
                        Frame {
                            kind,
                            transform,
                            paint,
                            clip_ref,
                            ..Frame::EMPTY
                        }
                    }
                };
                if let Some(parent) = depth.checked_sub(1).map(|index| &mut stack[index]) {
                    parent.child_count = parent
                        .child_count
                        .checked_add(1)
                        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                }
                stack[depth] = frame;
                depth += 1;
            }
            TagKind::End => {
                let index = depth
                    .checked_sub(1)
                    .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                let frame = stack[index];
                let expected = match frame.kind {
                    ContainerKind::Svg => "svg",
                    ContainerKind::Defs => "defs",
                    ContainerKind::ClipPath => "clipPath",
                    ContainerKind::Group => "g",
                };
                if tag.name != expected || tag.attrs.len != 0 {
                    return Err(ResourceAdmissionError::InvalidSafeVector);
                }
                match frame.kind {
                    ContainerKind::Svg if frame.child_count == 0 || !frame.has_paint => {
                        return Err(ResourceAdmissionError::InvalidSafeVector)
                    }
                    ContainerKind::Defs if frame.child_count == 0 => {
                        return Err(ResourceAdmissionError::InvalidSafeVector)
                    }
                    ContainerKind::ClipPath if frame.child_count != 1 => {
                        return Err(ResourceAdmissionError::InvalidSafeVector)
                    }
                    ContainerKind::Group if frame.child_count == 0 || !frame.has_paint => {
                        return Err(ResourceAdmissionError::InvalidSafeVector)
                    }
                    _ => {}
                }
                depth -= 1;
                if depth == 0 {
                    if frame.kind != ContainerKind::Svg {
                        return Err(ResourceAdmissionError::InvalidSafeVector);
                    }
                    root_closed = true;
                } else if frame.has_paint {
                    stack[depth - 1].has_paint = true;
                }
            }
            TagKind::Empty => {
                if !is_geometry(tag.name) || depth == 0 {
                    return Err(ResourceAdmissionError::InvalidSafeVector);
                }
                counts.nodes = counts
                    .nodes
                    .checked_add(1)
                    .ok_or(ResourceAdmissionError::VectorNodeLimit)?;
                if scan_limits.is_some_and(|limits| counts.nodes > limits.nodes) {
                    return Err(ResourceAdmissionError::VectorNodeLimit);
                }
                counts.max_depth = counts.max_depth.max(
                    u32::try_from(depth + 1)
                        .map_err(|_| ResourceAdmissionError::VectorNestingLimit)?,
                );
                if scan_limits.is_some_and(|limits| counts.max_depth > limits.depth) {
                    return Err(ResourceAdmissionError::VectorNestingLimit);
                }
                let parent = stack[depth - 1];
                let in_clip = parent.kind == ContainerKind::ClipPath;
                if in_clip
                    && !matches!(tag.name, "path" | "rect" | "circle" | "ellipse" | "polygon")
                {
                    return Err(ResourceAdmissionError::InvalidSafeVector);
                }
                if !in_clip && !matches!(parent.kind, ContainerKind::Svg | ContainerKind::Group) {
                    return Err(ResourceAdmissionError::InvalidSafeVector);
                }
                validate_geometry_attrs(tag.name, tag.attrs, in_clip)?;
                let local = parse_transform(tag.attrs.get("transform"))?;
                let transform = compose(parent.transform, local)?;
                // A clip definition has no physical CTM until a concrete
                // use site supplies `element_ctm`; every use is checked by
                // `resolve_clip_use` below. Paint geometry is rooted here.
                let physical_transform = if in_clip {
                    transform
                } else {
                    compose(
                        root_transform(root.ok_or(ResourceAdmissionError::InvalidSafeVector)?)?,
                        transform,
                    )?
                };
                let paint = if in_clip {
                    None
                } else {
                    let paint = inherit_paint(parent.paint, tag.attrs)?;
                    if paint.fill.is_none() && paint.stroke.is_none() {
                        return Err(ResourceAdmissionError::InvalidSafeVector);
                    }
                    if tag.name == "line" && paint.stroke.is_none() {
                        return Err(ResourceAdmissionError::InvalidSafeVector);
                    }
                    Some(paint)
                };
                if let Some(paint) = paint {
                    if paint.stroke.is_some() {
                        validate_transformed_stroke_width(paint.stroke_width, transform)?;
                        validate_transformed_stroke_width(paint.stroke_width, physical_transform)?;
                    }
                }
                let require_area = in_clip
                    || paint.is_some_and(|paint| paint.fill.is_some() && paint.stroke.is_none());
                let (path, segment_count) = if mode == ScanMode::Count {
                    let remaining_segments = scan_limits
                        .map(|limits| {
                            limits
                                .stored_segments
                                .checked_sub(counts.stored_segments)
                                .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)
                        })
                        .transpose()?;
                    (
                        None,
                        validate_shape_without_allocation(
                            tag.name,
                            tag.attrs,
                            transform,
                            physical_transform,
                            require_area,
                            in_clip,
                            remaining_segments,
                        )?,
                    )
                } else {
                    let path = shape_path(tag.name, tag.attrs)?;
                    if in_clip {
                        ensure_closed_clip(&path)?;
                    }
                    validate_path_geometry(&path, transform, require_area)?;
                    validate_path_geometry(&path, physical_transform, require_area)?;
                    let segment_count = u64::try_from(path.segments.len())
                        .map_err(|_| ResourceAdmissionError::VectorPathSegmentLimit)?;
                    (Some(path), segment_count)
                };
                counts.stored_segments = counts
                    .stored_segments
                    .checked_add(segment_count)
                    .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?;
                if scan_limits.is_some_and(|limits| counts.stored_segments > limits.stored_segments)
                {
                    return Err(ResourceAdmissionError::VectorPathSegmentLimit);
                }
                if in_clip {
                    let fill_rule = match tag.attrs.get("fill-rule") {
                        None | Some("nonzero") => SafeVectorFillRule::NonZero,
                        Some("evenodd") => SafeVectorFillRule::EvenOdd,
                        _ => return Err(ResourceAdmissionError::InvalidSafeVector),
                    };
                    if mode != ScanMode::Count {
                        let path = path.ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                        let id = parent
                            .clip_id
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                        definitions.push((
                            id,
                            SafeVectorClipDefinition {
                                clip_id: u32::try_from(definitions.len())
                                    .map_err(|_| ResourceAdmissionError::VectorNodeLimit)?,
                                transform: local,
                                fill_rule,
                                path,
                            },
                        ));
                    }
                } else {
                    let paint = paint.ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                    let clip_ref = parse_clip_ref(tag.attrs.get("clip-path"))?;
                    if let Some(id) = clip_ref {
                        counts.source_clip_id_bytes = counts
                            .source_clip_id_bytes
                            .checked_add(id.len() as u64)
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                        counts.commands = counts
                            .commands
                            .checked_add(2)
                            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                        if mode != ScanMode::Count {
                            references.push(id);
                        }
                        if mode == ScanMode::Analyze {
                            resolve_clip_use(
                                &definitions,
                                id,
                                transform,
                                root.ok_or(ResourceAdmissionError::InvalidSafeVector)?,
                            )?;
                        }
                    }
                    counts.commands = counts
                        .commands
                        .checked_add(1)
                        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                    stack[depth - 1].has_paint = true;
                    if mode == ScanMode::Build {
                        let path = path.ok_or(ResourceAdmissionError::InvalidSafeVector)?;
                        let mut clips = Vec::new();
                        for frame in &stack[..depth] {
                            if let Some(id) = frame.clip_ref {
                                clips.push(resolve_clip_use(
                                    &definitions,
                                    id,
                                    frame.transform,
                                    root.ok_or(ResourceAdmissionError::InvalidSafeVector)?,
                                )?);
                            }
                        }
                        if let Some(id) = clip_ref {
                            clips.push(resolve_clip_use(
                                &definitions,
                                id,
                                transform,
                                root.ok_or(ResourceAdmissionError::InvalidSafeVector)?,
                            )?);
                        }
                        draws.push(SafeVectorDraw {
                            transform,
                            clips,
                            path,
                            fill: paint.fill,
                            stroke: paint.stroke.map(|color| SafeVectorStroke {
                                color,
                                width: paint.stroke_width,
                                line_cap: paint.line_cap,
                                line_join: paint.line_join,
                                miter_limit: paint.miter_limit,
                            }),
                            fill_rule: paint.fill_rule,
                        });
                    }
                }
                stack[depth - 1].child_count = stack[depth - 1]
                    .child_count
                    .checked_add(1)
                    .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
            }
        }
    }
    if depth != 0 || !root_closed {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(ScanResult {
        counts,
        root: root.ok_or(ResourceAdmissionError::InvalidSafeVector)?,
        definitions,
        draws,
        references,
    })
}

fn parse_root(attrs: Attrs<'_>) -> Result<RootGeometry, ResourceAdmissionError> {
    require_names(attrs, &["height", "viewBox", "width", "xmlns"])?;
    if attrs.get("xmlns") != Some("http://www.w3.org/2000/svg") {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let (width, width_ratio) = physical_dimension(
        attrs
            .get("width")
            .ok_or(ResourceAdmissionError::InvalidSafeVector)?,
    )?;
    let (height, height_ratio) = physical_dimension(
        attrs
            .get("height")
            .ok_or(ResourceAdmissionError::InvalidSafeVector)?,
    )?;
    let view_box = attrs
        .get("viewBox")
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let values = parse_fixed_list(view_box, 4)?;
    let exact = parse_decimal_quartet(view_box)?;
    let [min_x, min_y, view_width, view_height]: [i64; 4] = values
        .as_slice()
        .try_into()
        .map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
    if view_width <= 0 || view_height <= 0 {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    if exact[2].numerator <= 0 || exact[3].numerator <= 0 {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let physical_cross = checked_product([
        width_ratio.0,
        height_ratio.1,
        exact[2].denominator,
        exact[3].numerator,
    ])?;
    let view_cross = checked_product([
        width_ratio.1,
        height_ratio.0,
        exact[2].numerator,
        exact[3].denominator,
    ])?;
    if physical_cross != view_cross {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let horizontal = fixed_ratio(width.get().raw(), view_width)?;
    let vertical = fixed_ratio(height.get().raw(), view_height)?;
    if horizontal <= 0 || horizontal != vertical {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let max_x = min_x
        .checked_add(view_width)
        .filter(|value| value.abs() <= MAX_COORDINATE)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let max_y = min_y
        .checked_add(view_height)
        .filter(|value| value.abs() <= MAX_COORDINATE)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let translate_x = fixed_mul(i64::from(horizontal), min_x)?
        .checked_neg()
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let translate_y = fixed_mul(i64::from(horizontal), min_y)?
        .checked_neg()
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let root_transform = SafeVectorTransform {
        a: horizontal,
        d: horizontal,
        e: translate_x,
        f: translate_y,
    };
    for point in [
        SafeVectorPoint { x: min_x, y: min_y },
        SafeVectorPoint { x: max_x, y: max_y },
    ] {
        transform_point(root_transform, point)?;
    }
    Ok(RootGeometry {
        width,
        height,
        view_box: [min_x, min_y, view_width, view_height],
        root_scale: horizontal,
    })
}

fn root_transform(root: RootGeometry) -> Result<SafeVectorTransform, ResourceAdmissionError> {
    let scale = i64::from(root.root_scale);
    let translate_x = fixed_mul(scale, root.view_box[0])?
        .checked_neg()
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let translate_y = fixed_mul(scale, root.view_box[1])?
        .checked_neg()
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    Ok(SafeVectorTransform {
        a: root.root_scale,
        d: root.root_scale,
        e: translate_x,
        f: translate_y,
    })
}

fn parse_decimal_quartet(value: &str) -> Result<[Decimal; 4], ResourceAdmissionError> {
    let mut output = [Decimal {
        numerator: 0,
        denominator: 1,
    }; 4];
    let mut count = 0usize;
    for token in value.split_ascii_whitespace() {
        if token.is_empty() || count == output.len() {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        output[count] = decimal(token)?;
        count += 1;
    }
    if count != output.len() {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(output)
}

fn checked_product(values: [i128; 4]) -> Result<i128, ResourceAdmissionError> {
    values.into_iter().try_fold(1i128, |product, value| {
        product
            .checked_mul(value)
            .ok_or(ResourceAdmissionError::InvalidSafeVector)
    })
}

fn physical_dimension(
    value: &str,
) -> Result<(PositiveLength, (i128, i128)), ResourceAdmissionError> {
    let (number, multiplier_num, multiplier_den) = if let Some(number) = value.strip_suffix("pt") {
        (number, 1i128, 1i128)
    } else if let Some(number) = value.strip_suffix("px") {
        (number, 3i128, 4i128)
    } else {
        (value, 3i128, 4i128)
    };
    let parsed = decimal(number)?;
    if parsed.numerator <= 0 {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let numerator = parsed
        .numerator
        .checked_mul(multiplier_num)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let denominator = parsed
        .denominator
        .checked_mul(multiplier_den)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let length = Length::from_rational_pdf_points(numerator, denominator)
        .map_err(|_| ResourceAdmissionError::InvalidSafeVector)?;
    let positive = PositiveLength::new(length).ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    Ok((positive, (numerator, denominator)))
}

fn require_names(attrs: Attrs<'_>, names: &[&str]) -> Result<(), ResourceAdmissionError> {
    if attrs.len != names.len()
        || attrs.names().any(|name| !names.contains(&name))
        || names.iter().any(|name| attrs.get(name).is_none())
    {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(())
}

fn validate_shared_attrs(
    attrs: Attrs<'_>,
    geometry: bool,
    clip_geometry: bool,
) -> Result<(), ResourceAdmissionError> {
    const PAINT: &[&str] = &[
        "fill",
        "stroke",
        "stroke-width",
        "fill-rule",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-miterlimit",
    ];
    for name in attrs.names() {
        let allowed = name == "transform"
            || (!clip_geometry && name == "clip-path")
            || (clip_geometry && name == "fill-rule")
            || (!clip_geometry && PAINT.contains(&name));
        if !allowed && !geometry {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
    }
    Ok(())
}

fn validate_geometry_attrs(
    name: &str,
    attrs: Attrs<'_>,
    clip: bool,
) -> Result<(), ResourceAdmissionError> {
    let geometry_names: &[&str] = match name {
        "path" => &["d"],
        "rect" => &["x", "y", "width", "height"],
        "circle" => &["cx", "cy", "r"],
        "ellipse" => &["cx", "cy", "rx", "ry"],
        "line" => &["x1", "y1", "x2", "y2"],
        "polyline" | "polygon" => &["points"],
        _ => return Err(ResourceAdmissionError::InvalidSafeVector),
    };
    const SHARED: &[&str] = &[
        "transform",
        "clip-path",
        "fill",
        "stroke",
        "stroke-width",
        "fill-rule",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-miterlimit",
    ];
    for attr in attrs.names() {
        if !geometry_names.contains(&attr) && !SHARED.contains(&attr)
            || (clip
                && !matches!(attr, "transform" | "fill-rule")
                && !geometry_names.contains(&attr))
        {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
    }
    let required: &[&str] = match name {
        "path" => &["d"],
        "rect" => &["width", "height"],
        "circle" => &["r"],
        "ellipse" => &["rx", "ry"],
        "line" => &["x1", "y1", "x2", "y2"],
        "polyline" | "polygon" => &["points"],
        _ => unreachable!(),
    };
    if required.iter().any(|name| attrs.get(name).is_none()) {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(())
}

fn parse_clip_ref(value: Option<&str>) -> Result<Option<&str>, ResourceAdmissionError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let id = value
        .strip_prefix("url(#")
        .and_then(|value| value.strip_suffix(')'))
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    validate_id(id)?;
    Ok(Some(id))
}

fn validate_id(id: &str) -> Result<(), ResourceAdmissionError> {
    let bytes = id.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 64
        || !matches!(bytes[0], b'A'..=b'Z' | b'a'..=b'z' | b'_')
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(())
}

fn is_geometry(name: &str) -> bool {
    matches!(
        name,
        "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon"
    )
}

fn ensure_closed_clip(path: &SafeVectorPath) -> Result<(), ResourceAdmissionError> {
    let mut open = false;
    for segment in path.segments() {
        match segment {
            SafeVectorSegment::Move(_) => {
                if open {
                    return Err(ResourceAdmissionError::InvalidSafeVector);
                }
                open = true;
            }
            SafeVectorSegment::Close => open = false,
            _ if !open => return Err(ResourceAdmissionError::InvalidSafeVector),
            _ => {}
        }
    }
    if open {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(())
}

fn resolve_clip_use(
    definitions: &[(&str, SafeVectorClipDefinition)],
    id: &str,
    use_transform: SafeVectorTransform,
    root: RootGeometry,
) -> Result<SafeVectorClipUse, ResourceAdmissionError> {
    let definition = definitions
        .iter()
        .find(|(candidate, _)| *candidate == id)
        .map(|(_, definition)| definition)
        .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
    let transform = compose(use_transform, definition.transform())?;
    validate_path_geometry(definition.path(), transform, true)?;
    validate_path_geometry(
        definition.path(),
        compose(root_transform(root)?, transform)?,
        true,
    )?;
    Ok(SafeVectorClipUse {
        clip_id: definition.clip_id(),
        transform: use_transform,
    })
}

fn validate_transformed_stroke_width(
    width: i64,
    transform: SafeVectorTransform,
) -> Result<(), ResourceAdmissionError> {
    if fixed_mul(i64::from(transform.a), width)? == 0
        || fixed_mul(i64::from(transform.d), width)? == 0
    {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn decode(
    bytes: &[u8],
    limits: &M4EffectiveResourceLimits,
) -> Result<DecodedSafeVector, ResourceAdmissionError> {
    let extension = limits.extension().get();
    decode_with_work_budget(
        bytes,
        limits,
        extension.max_vector_nodes,
        extension.max_vector_path_segments,
    )
}

pub(crate) fn decode_with_work_budget(
    bytes: &[u8],
    limits: &M4EffectiveResourceLimits,
    node_budget: u64,
    path_work_budget: u64,
) -> Result<DecodedSafeVector, ResourceAdmissionError> {
    let extension = limits.extension().get();
    if node_budget > extension.max_vector_nodes
        || path_work_budget > extension.max_vector_path_segments
    {
        return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
    }
    // Pass 1: lexical/structural/count validation. The scanner and element
    // stack are fixed-capacity; no IR survives this pass.
    let counted = scan(
        bytes,
        ScanMode::Count,
        Some(ScanLimits {
            nodes: node_budget,
            stored_segments: path_work_budget,
            depth: extension.max_vector_nesting_depth,
        }),
    )?;
    if counted.counts.nodes > node_budget {
        return Err(ResourceAdmissionError::VectorNodeLimit);
    }
    if counted.counts.max_depth > extension.max_vector_nesting_depth {
        return Err(ResourceAdmissionError::VectorNestingLimit);
    }
    if counted.counts.stored_segments > path_work_budget {
        return Err(ResourceAdmissionError::VectorPathSegmentLimit);
    }
    let allocation_charge = counted.counts.allocation_charge()?;
    if allocation_charge > limits.base().get().max_decoded_image_bytes {
        return Err(ResourceAdmissionError::DecodedImageLimit);
    }

    // Pass 2: bounded definition/reference closure and replay accounting.
    let analyzed = scan(bytes, ScanMode::Analyze, None)?;
    if analyzed.counts != counted.counts || analyzed.root != counted.root {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let mut id_map = BTreeMap::new();
    let mut used = BTreeSet::new();
    for (id, definition) in &analyzed.definitions {
        if id_map.insert(*id, definition).is_some() {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
    }
    let mut replay = 0u64;
    for reference in &analyzed.references {
        let definition = id_map
            .get(reference)
            .ok_or(ResourceAdmissionError::InvalidSafeVector)?;
        used.insert(*reference);
        replay = replay
            .checked_add(definition.path.segments.len() as u64)
            .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?;
        if counted
            .counts
            .stored_segments
            .checked_add(replay)
            .map_or(true, |work| work > path_work_budget)
        {
            return Err(ResourceAdmissionError::VectorPathSegmentLimit);
        }
    }
    if used.len() != id_map.len() {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let path_work = counted
        .counts
        .stored_segments
        .checked_add(replay)
        .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?;
    if path_work > path_work_budget {
        return Err(ResourceAdmissionError::VectorPathSegmentLimit);
    }
    drop(id_map);
    drop(used);
    drop(analyzed);

    // Pass 3: exact canonical IR construction.
    let built = scan(bytes, ScanMode::Build, None)?;
    if built.counts != counted.counts || built.root != counted.root || built.draws.is_empty() {
        return Err(ResourceAdmissionError::InvalidSafeVector);
    }
    let definitions: Vec<_> = built
        .definitions
        .into_iter()
        .map(|(_, definition)| definition)
        .collect();
    let canonical_jcs = encode_ir(
        built.root,
        &definitions,
        &built.draws,
        built.counts,
        path_work,
        allocation_charge,
    );
    let fingerprint_jcs = format!(
        "{{\"algorithm\":\"{}\",\"ir\":{}}}",
        SAFE_VECTOR_IR_FINGERPRINT_ID, canonical_jcs
    );
    let ir = SafeVectorIr {
        intrinsic_width: built.root.width,
        intrinsic_height: built.root.height,
        view_box: built.root.view_box,
        root_scale: built.root.root_scale,
        clips: definitions,
        draws: built.draws,
        node_count: built.counts.nodes,
        stored_segment_count: built.counts.stored_segments,
        path_work,
        allocation_charge,
        fingerprint: sha256(fingerprint_jcs.as_bytes()),
        canonical_jcs,
    };
    Ok(DecodedSafeVector {
        work: SafeVectorWork {
            nodes: ir.node_count,
            path_work: ir.path_work,
        },
        ir,
    })
}

fn encode_ir(
    root: RootGeometry,
    clips: &[SafeVectorClipDefinition],
    draws: &[SafeVectorDraw],
    counts: Counts,
    path_work: u64,
    allocation_charge: u64,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, SAFE_VECTOR_IR_ID);
    output.push_str(",\"allocation\":{\"algorithm\":");
    push_jcs_string(&mut output, SAFE_VECTOR_ALLOCATION_CHARGE_ID);
    output.push_str(",\"charge\":");
    output.push_str(&allocation_charge.to_string());
    output.push_str(",\"nodes\":");
    output.push_str(&counts.nodes.to_string());
    output.push_str(",\"paint_or_clip_commands\":");
    output.push_str(&counts.commands.to_string());
    output.push_str(",\"source_clip_id_bytes\":");
    output.push_str(&counts.source_clip_id_bytes.to_string());
    output.push_str(",\"stored_segments\":");
    output.push_str(&counts.stored_segments.to_string());
    output.push_str("},\"clips\":[");
    for (index, clip) in clips.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"clip_id\":");
        output.push_str(&clip.clip_id.to_string());
        output.push_str(",\"fill_rule\":");
        push_jcs_string(&mut output, clip.fill_rule.as_str());
        output.push_str(",\"path\":");
        encode_path(&mut output, &clip.path);
        output.push_str(",\"transform\":");
        encode_transform(&mut output, clip.transform);
        output.push('}');
    }
    output.push_str("],\"draws\":[");
    for (index, draw) in draws.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"clips\":[");
        for (clip_index, clip) in draw.clips.iter().enumerate() {
            if clip_index > 0 {
                output.push(',');
            }
            output.push_str("{\"clip_id\":");
            output.push_str(&clip.clip_id.to_string());
            output.push_str(",\"transform\":");
            encode_transform(&mut output, clip.transform);
            output.push('}');
        }
        output.push_str("],\"fill\":");
        encode_color(&mut output, draw.fill);
        output.push_str(",\"fill_rule\":");
        push_jcs_string(&mut output, draw.fill_rule.as_str());
        output.push_str(",\"path\":");
        encode_path(&mut output, &draw.path);
        output.push_str(",\"stroke\":");
        if let Some(stroke) = draw.stroke {
            output.push_str("{\"color\":");
            encode_color(&mut output, Some(stroke.color));
            output.push_str(",\"line_cap\":");
            push_jcs_string(&mut output, stroke.line_cap.as_str());
            output.push_str(",\"line_join\":");
            push_jcs_string(&mut output, stroke.line_join.as_str());
            output.push_str(",\"miter_limit\":");
            output.push_str(&stroke.miter_limit.to_string());
            output.push_str(",\"width\":");
            output.push_str(&stroke.width.to_string());
            output.push('}');
        } else {
            output.push_str("null");
        }
        output.push_str(",\"transform\":");
        encode_transform(&mut output, draw.transform);
        output.push('}');
    }
    output.push_str("],\"intrinsic_height\":");
    output.push_str(&root.height.get().raw().to_string());
    output.push_str(",\"intrinsic_width\":");
    output.push_str(&root.width.get().raw().to_string());
    output.push_str(",\"parser\":");
    push_jcs_string(&mut output, SAFE_SVG_PARSER_ID);
    output.push_str(",\"path_work\":");
    output.push_str(&path_work.to_string());
    output.push_str(",\"root_scale\":");
    output.push_str(&root.root_scale.to_string());
    output.push_str(",\"view_box\":[");
    for (index, value) in root.view_box.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&value.to_string());
    }
    output.push_str("]}");
    output
}

fn encode_transform(output: &mut String, value: SafeVectorTransform) {
    output.push_str("{\"a\":");
    output.push_str(&value.a.to_string());
    output.push_str(",\"d\":");
    output.push_str(&value.d.to_string());
    output.push_str(",\"e\":");
    output.push_str(&value.e.to_string());
    output.push_str(",\"f\":");
    output.push_str(&value.f.to_string());
    output.push('}');
}

fn encode_color(output: &mut String, color: Option<[u8; 3]>) {
    if let Some(color) = color {
        output.push('[');
        output.push_str(&color[0].to_string());
        output.push(',');
        output.push_str(&color[1].to_string());
        output.push(',');
        output.push_str(&color[2].to_string());
        output.push(']');
    } else {
        output.push_str("null");
    }
}

fn encode_path(output: &mut String, path: &SafeVectorPath) {
    output.push('[');
    for (index, segment) in path.segments.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"kind\":");
        push_jcs_string(output, segment.kind_str());
        match segment {
            SafeVectorSegment::Move(point) | SafeVectorSegment::Line(point) => {
                output.push_str(",\"points\":[");
                encode_point(output, *point);
                output.push(']');
            }
            SafeVectorSegment::Quadratic(first, second) => {
                output.push_str(",\"points\":[");
                encode_point(output, *first);
                output.push(',');
                encode_point(output, *second);
                output.push(']');
            }
            SafeVectorSegment::Cubic(first, second, third) => {
                output.push_str(",\"points\":[");
                encode_point(output, *first);
                output.push(',');
                encode_point(output, *second);
                output.push(',');
                encode_point(output, *third);
                output.push(']');
            }
            SafeVectorSegment::Close => output.push_str(",\"points\":[]"),
        }
        output.push('}');
    }
    output.push(']');
}

fn encode_point(output: &mut String, point: SafeVectorPoint) {
    output.push('[');
    output.push_str(&point.x.to_string());
    output.push(',');
    output.push_str(&point.y.to_string());
    output.push(']');
}

mod v2 {
    use super::*;

    const OPAQUE_ALPHA: u32 = 65_536;

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct SafeVectorAlpha(u32);

    impl SafeVectorAlpha {
        pub const OPAQUE: Self = Self(OPAQUE_ALPHA);

        pub const fn raw(self) -> u32 {
            self.0
        }

        pub const fn is_positive(self) -> bool {
            self.0 != 0
        }
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub enum SafeVectorPaint {
        None,
        FixedRgb8([u8; 3]),
        CurrentColor,
    }

    impl SafeVectorPaint {
        pub const fn as_str(self) -> &'static str {
            match self {
                Self::None => "none",
                Self::FixedRgb8(_) => "fixed-rgb8",
                Self::CurrentColor => "current-color",
            }
        }

        pub const fn enabled(self) -> bool {
            !matches!(self, Self::None)
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct SafeVectorPaintLayer {
        paint: SafeVectorPaint,
        alpha: SafeVectorAlpha,
    }

    impl SafeVectorPaintLayer {
        pub const fn paint(self) -> SafeVectorPaint {
            self.paint
        }

        pub const fn alpha(self) -> SafeVectorAlpha {
            self.alpha
        }

        pub const fn is_visible(self) -> bool {
            self.paint.enabled() && self.alpha.is_positive()
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct SafeVectorStrokeV2 {
        paint: SafeVectorPaintLayer,
        width: i64,
        line_cap: SafeVectorLineCap,
        line_join: SafeVectorLineJoin,
        miter_limit: i64,
    }

    impl SafeVectorStrokeV2 {
        pub const fn paint(self) -> SafeVectorPaintLayer {
            self.paint
        }
        pub const fn width_raw(self) -> i64 {
            self.width
        }
        pub const fn line_cap(self) -> SafeVectorLineCap {
            self.line_cap
        }
        pub const fn line_join(self) -> SafeVectorLineJoin {
            self.line_join
        }
        pub const fn miter_limit_raw(self) -> i64 {
            self.miter_limit
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct SafeVectorDrawV2 {
        transform: SafeVectorTransform,
        clips: Vec<SafeVectorClipUse>,
        path: SafeVectorPath,
        fill: SafeVectorPaintLayer,
        stroke: SafeVectorStrokeV2,
        fill_rule: SafeVectorFillRule,
    }

    impl SafeVectorDrawV2 {
        pub const fn transform(&self) -> SafeVectorTransform {
            self.transform
        }
        pub fn clips(&self) -> &[SafeVectorClipUse] {
            &self.clips
        }
        pub const fn path(&self) -> &SafeVectorPath {
            &self.path
        }
        pub const fn fill(&self) -> SafeVectorPaintLayer {
            self.fill
        }
        pub const fn stroke(&self) -> SafeVectorStrokeV2 {
            self.stroke
        }
        pub const fn fill_rule(&self) -> SafeVectorFillRule {
            self.fill_rule
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct SafeVectorIrV2 {
        intrinsic_width: PositiveLength,
        intrinsic_height: PositiveLength,
        view_box: [i64; 4],
        root_scale: i32,
        clips: Vec<SafeVectorClipDefinition>,
        draws: Vec<SafeVectorDrawV2>,
        node_count: u64,
        stored_segment_count: u64,
        path_work: u64,
        allocation_charge: u64,
        canonical_jcs: String,
        fingerprint: [u8; 32],
    }

    impl SafeVectorIrV2 {
        pub const fn intrinsic_width(&self) -> PositiveLength {
            self.intrinsic_width
        }
        pub const fn intrinsic_height(&self) -> PositiveLength {
            self.intrinsic_height
        }
        pub const fn view_box(&self) -> [i64; 4] {
            self.view_box
        }
        pub const fn root_scale_raw(&self) -> i32 {
            self.root_scale
        }
        pub fn clips(&self) -> &[SafeVectorClipDefinition] {
            &self.clips
        }
        pub fn draws(&self) -> &[SafeVectorDrawV2] {
            &self.draws
        }
        pub const fn node_count(&self) -> u64 {
            self.node_count
        }
        pub const fn stored_segment_count(&self) -> u64 {
            self.stored_segment_count
        }
        pub const fn path_work(&self) -> u64 {
            self.path_work
        }
        pub const fn allocation_charge(&self) -> u64 {
            self.allocation_charge
        }
        pub fn canonical_jcs(&self) -> &str {
            &self.canonical_jcs
        }
        pub const fn fingerprint(&self) -> [u8; 32] {
            self.fingerprint
        }
        pub const fn parser_profile(&self) -> SafeVectorParserProfile {
            SafeVectorParserProfile::SafeSvg2
        }
        pub const fn root_viewport_clip_is_outermost(&self) -> bool {
            true
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    pub(crate) struct DecodedSafeVectorV2 {
        pub ir: SafeVectorIrV2,
        pub work: SafeVectorWork,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct PaintStateV2 {
        fill: SafeVectorPaint,
        fill_alpha: SafeVectorAlpha,
        stroke: SafeVectorPaint,
        stroke_alpha: SafeVectorAlpha,
        stroke_width: i64,
        fill_rule: SafeVectorFillRule,
        line_cap: SafeVectorLineCap,
        line_join: SafeVectorLineJoin,
        miter_limit: i64,
    }

    impl Default for PaintStateV2 {
        fn default() -> Self {
            Self {
                fill: SafeVectorPaint::FixedRgb8([0, 0, 0]),
                fill_alpha: SafeVectorAlpha::OPAQUE,
                stroke: SafeVectorPaint::None,
                stroke_alpha: SafeVectorAlpha::OPAQUE,
                stroke_width: FIXED_ONE,
                fill_rule: SafeVectorFillRule::NonZero,
                line_cap: SafeVectorLineCap::Butt,
                line_join: SafeVectorLineJoin::Miter,
                miter_limit: 4 * FIXED_ONE,
            }
        }
    }

    impl PaintStateV2 {
        const fn fill_layer(self) -> SafeVectorPaintLayer {
            SafeVectorPaintLayer {
                paint: self.fill,
                alpha: self.fill_alpha,
            }
        }

        const fn stroke_value(self) -> SafeVectorStrokeV2 {
            SafeVectorStrokeV2 {
                paint: SafeVectorPaintLayer {
                    paint: self.stroke,
                    alpha: self.stroke_alpha,
                },
                width: self.stroke_width,
                line_cap: self.line_cap,
                line_join: self.line_join,
                miter_limit: self.miter_limit,
            }
        }
    }

    #[derive(Clone, Copy)]
    struct FrameV2<'a> {
        kind: ContainerKind,
        child_count: u32,
        has_visible_draw: bool,
        transform: SafeVectorTransform,
        paint: PaintStateV2,
        clip_ref: Option<&'a str>,
        clip_id: Option<&'a str>,
    }

    impl<'a> FrameV2<'a> {
        const EMPTY: Self = Self {
            kind: ContainerKind::Svg,
            child_count: 0,
            has_visible_draw: false,
            transform: SafeVectorTransform::IDENTITY,
            paint: PaintStateV2 {
                fill: SafeVectorPaint::FixedRgb8([0, 0, 0]),
                fill_alpha: SafeVectorAlpha::OPAQUE,
                stroke: SafeVectorPaint::None,
                stroke_alpha: SafeVectorAlpha::OPAQUE,
                stroke_width: FIXED_ONE,
                fill_rule: SafeVectorFillRule::NonZero,
                line_cap: SafeVectorLineCap::Butt,
                line_join: SafeVectorLineJoin::Miter,
                miter_limit: 4 * FIXED_ONE,
            },
            clip_ref: None,
            clip_id: None,
        };
    }

    struct ScanResultV2<'a> {
        counts: Counts,
        root: RootGeometry,
        definitions: Vec<(&'a str, SafeVectorClipDefinition)>,
        draws: Vec<SafeVectorDrawV2>,
        references: Vec<&'a str>,
    }

    fn error(reason: SafeVectorFailureReason) -> ResourceAdmissionError {
        ResourceAdmissionError::InvalidSafeVectorV2(reason)
    }

    fn preserve_limit_or_malformed(source: ResourceAdmissionError) -> ResourceAdmissionError {
        match source {
            ResourceAdmissionError::VectorNodeLimit
            | ResourceAdmissionError::VectorPathSegmentLimit
            | ResourceAdmissionError::VectorNestingLimit
            | ResourceAdmissionError::DecodedImageLimit => source,
            _ => error(SafeVectorFailureReason::MalformedSvg),
        }
    }

    fn checked<T>(result: Result<T, ResourceAdmissionError>) -> Result<T, ResourceAdmissionError> {
        result.map_err(preserve_limit_or_malformed)
    }

    fn scanner(bytes: &[u8]) -> Result<MarkupScanner<'_>, ResourceAdmissionError> {
        let source =
            std::str::from_utf8(bytes).map_err(|_| error(SafeVectorFailureReason::MalformedSvg))?;
        if source.starts_with('\u{feff}')
            || bytes.iter().any(|byte| {
                *byte == 0
                    || *byte == b'\r'
                    || (*byte < 0x20 && !matches!(*byte, b' ' | b'\t' | b'\n'))
                    || (0x7f..=0x9f).contains(byte)
            })
        {
            return Err(error(SafeVectorFailureReason::MalformedSvg));
        }
        if source.contains("<!") || source.contains("<?") || source.contains('&') {
            return Err(error(SafeVectorFailureReason::ForbiddenFeature));
        }
        MarkupScanner::new(bytes).map_err(preserve_limit_or_malformed)
    }

    fn parse_paint(value: &str) -> Result<SafeVectorPaint, ResourceAdmissionError> {
        match value {
            "none" => Ok(SafeVectorPaint::None),
            "currentColor" => Ok(SafeVectorPaint::CurrentColor),
            value if value.starts_with("url(#") => {
                Err(error(SafeVectorFailureReason::UnsupportedFeature))
            }
            value if value.starts_with("url(") => {
                Err(error(SafeVectorFailureReason::ExternalReference))
            }
            _ => parse_color(value)
                .map(|color| color.map_or(SafeVectorPaint::None, SafeVectorPaint::FixedRgb8))
                .map_err(|_| error(SafeVectorFailureReason::ForbiddenFeature)),
        }
    }

    fn parse_alpha(value: &str) -> Result<SafeVectorAlpha, ResourceAdmissionError> {
        if value == "0" {
            return Ok(SafeVectorAlpha(0));
        }
        if value == "1" {
            return Ok(SafeVectorAlpha::OPAQUE);
        }
        let (whole, fraction) = value
            .split_once('.')
            .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
        if !(1..=6).contains(&fraction.len())
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
            || !matches!(whole, "0" | "1")
            || (whole == "1" && !fraction.bytes().all(|byte| byte == b'0'))
        {
            return Err(error(SafeVectorFailureReason::MalformedSvg));
        }
        let denominator = 10i128
            .checked_pow(fraction.len() as u32)
            .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
        let fraction = fraction
            .parse::<i128>()
            .map_err(|_| error(SafeVectorFailureReason::MalformedSvg))?;
        let numerator = if whole == "1" { denominator } else { fraction };
        let raw = round_ties_even(
            numerator
                .checked_mul(i128::from(OPAQUE_ALPHA))
                .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?,
            denominator,
        )
        .map_err(preserve_limit_or_malformed)?;
        let raw = u32::try_from(raw).map_err(|_| error(SafeVectorFailureReason::MalformedSvg))?;
        if raw > OPAQUE_ALPHA {
            return Err(error(SafeVectorFailureReason::MalformedSvg));
        }
        Ok(SafeVectorAlpha(raw))
    }

    fn inherit_paint_v2(
        mut state: PaintStateV2,
        attrs: Attrs<'_>,
    ) -> Result<PaintStateV2, ResourceAdmissionError> {
        if let Some(value) = attrs.get("fill") {
            state.fill = parse_paint(value)?;
        }
        if let Some(value) = attrs.get("fill-opacity") {
            state.fill_alpha = parse_alpha(value)?;
        }
        if let Some(value) = attrs.get("stroke") {
            state.stroke = parse_paint(value)?;
        }
        if let Some(value) = attrs.get("stroke-opacity") {
            state.stroke_alpha = parse_alpha(value)?;
        }
        if let Some(value) = attrs.get("stroke-width") {
            state.stroke_width = checked(positive_fixed(value))?;
        }
        if let Some(value) = attrs.get("fill-rule") {
            state.fill_rule = match value {
                "nonzero" => SafeVectorFillRule::NonZero,
                "evenodd" => SafeVectorFillRule::EvenOdd,
                _ => return Err(error(SafeVectorFailureReason::MalformedSvg)),
            };
        }
        if let Some(value) = attrs.get("stroke-linecap") {
            state.line_cap = match value {
                "butt" => SafeVectorLineCap::Butt,
                "round" => SafeVectorLineCap::Round,
                "square" => SafeVectorLineCap::Square,
                _ => return Err(error(SafeVectorFailureReason::MalformedSvg)),
            };
        }
        if let Some(value) = attrs.get("stroke-linejoin") {
            state.line_join = match value {
                "miter" => SafeVectorLineJoin::Miter,
                "round" => SafeVectorLineJoin::Round,
                "bevel" => SafeVectorLineJoin::Bevel,
                _ => return Err(error(SafeVectorFailureReason::MalformedSvg)),
            };
        }
        if let Some(value) = attrs.get("stroke-miterlimit") {
            state.miter_limit = checked(decimal_fixed(value))?;
            if state.miter_limit < FIXED_ONE {
                return Err(error(SafeVectorFailureReason::MalformedSvg));
            }
        }
        Ok(state)
    }

    fn forbidden_attribute_reason(name: &str, value: &str) -> SafeVectorFailureReason {
        if matches!(name, "href" | "xlink-href")
            || name == "clip-path" && (value.starts_with("url(") && !value.starts_with("url(#"))
        {
            return SafeVectorFailureReason::ExternalReference;
        }
        if matches!(
            name,
            "opacity" | "mask" | "filter" | "isolation" | "mix-blend-mode"
        ) {
            return SafeVectorFailureReason::UnsupportedFeature;
        }
        if name.starts_with("on")
            || matches!(name, "style" | "class" | "color" | "font" | "font-family")
        {
            return SafeVectorFailureReason::ForbiddenFeature;
        }
        SafeVectorFailureReason::UnsupportedFeature
    }

    fn validate_attrs(
        name: &str,
        attrs: Attrs<'_>,
        clip_geometry: bool,
    ) -> Result<(), ResourceAdmissionError> {
        let geometry_names: &[&str] = match name {
            "path" => &["d"],
            "rect" => &["x", "y", "width", "height"],
            "circle" => &["cx", "cy", "r"],
            "ellipse" => &["cx", "cy", "rx", "ry"],
            "line" => &["x1", "y1", "x2", "y2"],
            "polyline" | "polygon" => &["points"],
            "g" => &[],
            _ => return Err(error(SafeVectorFailureReason::UnsupportedFeature)),
        };
        const PAINT: &[&str] = &[
            "fill",
            "fill-opacity",
            "stroke",
            "stroke-opacity",
            "stroke-width",
            "fill-rule",
            "stroke-linecap",
            "stroke-linejoin",
            "stroke-miterlimit",
        ];
        for attr in attrs.values.iter().flatten() {
            let allowed = geometry_names.contains(&attr.name)
                || attr.name == "transform"
                || (!clip_geometry && attr.name == "clip-path")
                || (clip_geometry && attr.name == "fill-rule")
                || (!clip_geometry && PAINT.contains(&attr.name));
            if !allowed {
                return Err(error(forbidden_attribute_reason(attr.name, attr.value)));
            }
        }
        let required: &[&str] = match name {
            "path" => &["d"],
            "rect" => &["width", "height"],
            "circle" => &["r"],
            "ellipse" => &["rx", "ry"],
            "line" => &["x1", "y1", "x2", "y2"],
            "polyline" | "polygon" => &["points"],
            "g" => &[],
            _ => unreachable!(),
        };
        if required.iter().any(|name| attrs.get(name).is_none()) {
            return Err(error(SafeVectorFailureReason::MalformedSvg));
        }
        if let Some(path) = attrs.get("d") {
            for byte in path.bytes().filter(u8::is_ascii_alphabetic) {
                if !matches!(
                    byte,
                    b'M' | b'm'
                        | b'L'
                        | b'l'
                        | b'H'
                        | b'h'
                        | b'V'
                        | b'v'
                        | b'C'
                        | b'c'
                        | b'Q'
                        | b'q'
                        | b'Z'
                        | b'z'
                ) {
                    return Err(error(SafeVectorFailureReason::UnsupportedFeature));
                }
            }
        }
        Ok(())
    }

    fn parse_clip_ref_v2(value: Option<&str>) -> Result<Option<&str>, ResourceAdmissionError> {
        let Some(value) = value else {
            return Ok(None);
        };
        let Some(id) = value
            .strip_prefix("url(#")
            .and_then(|value| value.strip_suffix(')'))
        else {
            return Err(error(SafeVectorFailureReason::ExternalReference));
        };
        checked(validate_id(id))?;
        Ok(Some(id))
    }

    fn classify_element(tag: Tag<'_>) -> ResourceAdmissionError {
        if tag.attrs.values.iter().flatten().any(|attr| {
            matches!(attr.name, "href" | "xlink-href")
                && (attr.value.contains("://") || !attr.value.starts_with('#'))
        }) {
            return error(SafeVectorFailureReason::ExternalReference);
        }
        if matches!(
            tag.name,
            "script" | "style" | "animate" | "animateTransform" | "set"
        ) {
            error(SafeVectorFailureReason::ForbiddenFeature)
        } else {
            error(SafeVectorFailureReason::UnsupportedFeature)
        }
    }

    fn allocation_charge(counts: Counts) -> Result<u64, ResourceAdmissionError> {
        counts
            .nodes
            .checked_mul(64)
            .and_then(|value| {
                counts
                    .stored_segments
                    .checked_mul(80)
                    .and_then(|part| value.checked_add(part))
            })
            .and_then(|value| {
                counts
                    .commands
                    .checked_mul(48)
                    .and_then(|part| value.checked_add(part))
            })
            .and_then(|value| value.checked_add(counts.source_clip_id_bytes))
            .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))
    }

    fn parse_transform_v2(
        value: Option<&str>,
    ) -> Result<SafeVectorTransform, ResourceAdmissionError> {
        if value.is_some_and(|value| {
            value.contains("rotate")
                || value.contains("skewX")
                || value.contains("skewY")
                || value.contains("perspective")
        }) {
            return Err(error(SafeVectorFailureReason::UnsupportedFeature));
        }
        checked(parse_transform(value))
    }

    fn resolve_clip_use_v2(
        definitions: &[(&str, SafeVectorClipDefinition)],
        id: &str,
        use_transform: SafeVectorTransform,
        root: RootGeometry,
    ) -> Result<SafeVectorClipUse, ResourceAdmissionError> {
        if !definitions.iter().any(|(candidate, _)| *candidate == id) {
            return Err(error(SafeVectorFailureReason::ForbiddenFeature));
        }
        checked(resolve_clip_use(definitions, id, use_transform, root))
    }

    fn require_exact_attrs(attrs: Attrs<'_>, names: &[&str]) -> Result<(), ResourceAdmissionError> {
        for attr in attrs.values.iter().flatten() {
            if !names.contains(&attr.name) {
                return Err(error(forbidden_attribute_reason(attr.name, attr.value)));
            }
        }
        if attrs.len != names.len() || names.iter().any(|name| attrs.get(name).is_none()) {
            return Err(error(SafeVectorFailureReason::MalformedSvg));
        }
        Ok(())
    }

    fn scan_v2<'a>(
        bytes: &'a [u8],
        mode: ScanMode,
        scan_limits: Option<ScanLimits>,
    ) -> Result<ScanResultV2<'a>, ResourceAdmissionError> {
        let mut scanner = scanner(bytes)?;
        let mut stack = [FrameV2::EMPTY; HARD_STACK_DEPTH];
        let mut depth = 0usize;
        let mut counts = Counts::new();
        if scan_limits.is_some_and(|limits| counts.stored_segments > limits.stored_segments) {
            return Err(ResourceAdmissionError::VectorPathSegmentLimit);
        }
        let mut root = None;
        let mut definitions = Vec::new();
        let mut draws = Vec::new();
        let mut references = Vec::new();
        let mut root_closed = false;

        while let Some(tag) = scanner.next().map_err(preserve_limit_or_malformed)? {
            if root_closed {
                return Err(error(SafeVectorFailureReason::MalformedSvg));
            }
            match tag.kind {
                TagKind::Start => {
                    let kind = match tag.name {
                        "svg" => ContainerKind::Svg,
                        "defs" => ContainerKind::Defs,
                        "clipPath" => ContainerKind::ClipPath,
                        "g" => ContainerKind::Group,
                        _ => return Err(classify_element(tag)),
                    };
                    if depth == HARD_STACK_DEPTH {
                        return Err(ResourceAdmissionError::VectorNestingLimit);
                    }
                    let next_depth = u32::try_from(depth + 1)
                        .map_err(|_| ResourceAdmissionError::VectorNestingLimit)?;
                    counts.max_depth = counts.max_depth.max(next_depth);
                    if scan_limits.is_some_and(|limits| counts.max_depth > limits.depth) {
                        return Err(ResourceAdmissionError::VectorNestingLimit);
                    }
                    counts.nodes = counts
                        .nodes
                        .checked_add(1)
                        .ok_or(ResourceAdmissionError::VectorNodeLimit)?;
                    if scan_limits.is_some_and(|limits| counts.nodes > limits.nodes) {
                        return Err(ResourceAdmissionError::VectorNodeLimit);
                    }
                    let parent = depth.checked_sub(1).map(|index| stack[index]);
                    let frame = match kind {
                        ContainerKind::Svg => {
                            if depth != 0 || root.is_some() {
                                return Err(error(SafeVectorFailureReason::MalformedSvg));
                            }
                            require_exact_attrs(
                                tag.attrs,
                                &["height", "viewBox", "width", "xmlns"],
                            )?;
                            let geometry = checked(parse_root(tag.attrs))?;
                            root = Some(geometry);
                            FrameV2 {
                                kind,
                                transform: SafeVectorTransform::IDENTITY,
                                paint: PaintStateV2::default(),
                                ..FrameV2::EMPTY
                            }
                        }
                        ContainerKind::Defs => {
                            let parent = parent
                                .filter(|parent| {
                                    parent.kind == ContainerKind::Svg && parent.child_count == 0
                                })
                                .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                            require_exact_attrs(tag.attrs, &[])?;
                            FrameV2 {
                                kind,
                                transform: parent.transform,
                                paint: parent.paint,
                                ..FrameV2::EMPTY
                            }
                        }
                        ContainerKind::ClipPath => {
                            let parent = parent
                                .filter(|parent| parent.kind == ContainerKind::Defs)
                                .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                            require_exact_attrs(tag.attrs, &["id"])?;
                            let id = tag
                                .attrs
                                .get("id")
                                .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                            checked(validate_id(id))?;
                            counts.source_clip_id_bytes = counts
                                .source_clip_id_bytes
                                .checked_add(id.len() as u64)
                                .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                            FrameV2 {
                                kind,
                                transform: parent.transform,
                                paint: parent.paint,
                                clip_id: Some(id),
                                ..FrameV2::EMPTY
                            }
                        }
                        ContainerKind::Group => {
                            let parent = parent
                                .filter(|parent| {
                                    matches!(parent.kind, ContainerKind::Svg | ContainerKind::Group)
                                })
                                .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                            validate_attrs("g", tag.attrs, false)?;
                            let local = parse_transform_v2(tag.attrs.get("transform"))?;
                            let transform = checked(compose(parent.transform, local))?;
                            let paint = inherit_paint_v2(parent.paint, tag.attrs)?;
                            let clip_ref = parse_clip_ref_v2(tag.attrs.get("clip-path"))?;
                            if let Some(id) = clip_ref {
                                counts.source_clip_id_bytes = counts
                                    .source_clip_id_bytes
                                    .checked_add(id.len() as u64)
                                    .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                                counts.commands = counts
                                    .commands
                                    .checked_add(2)
                                    .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                                if mode != ScanMode::Count {
                                    references.push(id);
                                }
                                if mode == ScanMode::Analyze {
                                    resolve_clip_use_v2(
                                        &definitions,
                                        id,
                                        transform,
                                        root.ok_or_else(|| {
                                            error(SafeVectorFailureReason::MalformedSvg)
                                        })?,
                                    )?;
                                }
                            }
                            FrameV2 {
                                kind,
                                transform,
                                paint,
                                clip_ref,
                                ..FrameV2::EMPTY
                            }
                        }
                    };
                    if let Some(parent) = depth.checked_sub(1).map(|index| &mut stack[index]) {
                        parent.child_count = parent
                            .child_count
                            .checked_add(1)
                            .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                    }
                    stack[depth] = frame;
                    depth += 1;
                }
                TagKind::End => {
                    let index = depth
                        .checked_sub(1)
                        .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                    let frame = stack[index];
                    let expected = match frame.kind {
                        ContainerKind::Svg => "svg",
                        ContainerKind::Defs => "defs",
                        ContainerKind::ClipPath => "clipPath",
                        ContainerKind::Group => "g",
                    };
                    if tag.name != expected || tag.attrs.len != 0 {
                        return Err(error(SafeVectorFailureReason::MalformedSvg));
                    }
                    match frame.kind {
                        ContainerKind::Svg if frame.child_count == 0 => {
                            return Err(error(SafeVectorFailureReason::MalformedSvg));
                        }
                        ContainerKind::Svg if !frame.has_visible_draw => {
                            return Err(error(SafeVectorFailureReason::ForbiddenFeature));
                        }
                        ContainerKind::Defs if frame.child_count == 0 => {
                            return Err(error(SafeVectorFailureReason::MalformedSvg));
                        }
                        ContainerKind::ClipPath if frame.child_count != 1 => {
                            return Err(error(SafeVectorFailureReason::MalformedSvg));
                        }
                        ContainerKind::Group if frame.child_count == 0 => {
                            return Err(error(SafeVectorFailureReason::MalformedSvg));
                        }
                        _ => {}
                    }
                    depth -= 1;
                    if depth == 0 {
                        if frame.kind != ContainerKind::Svg {
                            return Err(error(SafeVectorFailureReason::MalformedSvg));
                        }
                        root_closed = true;
                    } else if frame.has_visible_draw {
                        stack[depth - 1].has_visible_draw = true;
                    }
                }
                TagKind::Empty => {
                    if !is_geometry(tag.name) || depth == 0 {
                        if matches!(tag.name, "svg" | "defs" | "clipPath" | "g") {
                            return Err(error(SafeVectorFailureReason::MalformedSvg));
                        }
                        return Err(classify_element(tag));
                    }
                    counts.nodes = counts
                        .nodes
                        .checked_add(1)
                        .ok_or(ResourceAdmissionError::VectorNodeLimit)?;
                    if scan_limits.is_some_and(|limits| counts.nodes > limits.nodes) {
                        return Err(ResourceAdmissionError::VectorNodeLimit);
                    }
                    counts.max_depth = counts.max_depth.max(
                        u32::try_from(depth + 1)
                            .map_err(|_| ResourceAdmissionError::VectorNestingLimit)?,
                    );
                    if scan_limits.is_some_and(|limits| counts.max_depth > limits.depth) {
                        return Err(ResourceAdmissionError::VectorNestingLimit);
                    }
                    let parent = stack[depth - 1];
                    let in_clip = parent.kind == ContainerKind::ClipPath;
                    if in_clip
                        && !matches!(tag.name, "path" | "rect" | "circle" | "ellipse" | "polygon")
                    {
                        return Err(error(SafeVectorFailureReason::UnsupportedFeature));
                    }
                    if !in_clip && !matches!(parent.kind, ContainerKind::Svg | ContainerKind::Group)
                    {
                        return Err(error(SafeVectorFailureReason::MalformedSvg));
                    }
                    validate_attrs(tag.name, tag.attrs, in_clip)?;
                    let local = parse_transform_v2(tag.attrs.get("transform"))?;
                    let transform = checked(compose(parent.transform, local))?;
                    let root_geometry =
                        root.ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                    let physical_transform = if in_clip {
                        transform
                    } else {
                        checked(compose(checked(root_transform(root_geometry))?, transform))?
                    };
                    let paint = if in_clip {
                        None
                    } else {
                        Some(inherit_paint_v2(parent.paint, tag.attrs)?)
                    };
                    if let Some(paint) = paint {
                        if paint.stroke.enabled() {
                            checked(validate_transformed_stroke_width(
                                paint.stroke_width,
                                transform,
                            ))?;
                            checked(validate_transformed_stroke_width(
                                paint.stroke_width,
                                physical_transform,
                            ))?;
                        }
                    }
                    let fill_visible = paint
                        .is_some_and(|paint| tag.name != "line" && paint.fill_layer().is_visible());
                    let stroke_visible =
                        paint.is_some_and(|paint| paint.stroke_value().paint().is_visible());
                    let visible = fill_visible || stroke_visible;
                    let require_area = in_clip || (fill_visible && !stroke_visible);
                    let (path, segment_count) = if mode == ScanMode::Count {
                        let remaining_segments = scan_limits
                            .map(|limits| {
                                limits
                                    .stored_segments
                                    .checked_sub(counts.stored_segments)
                                    .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)
                            })
                            .transpose()?;
                        (
                            None,
                            checked(validate_shape_without_allocation(
                                tag.name,
                                tag.attrs,
                                transform,
                                physical_transform,
                                require_area,
                                in_clip,
                                remaining_segments,
                            ))?,
                        )
                    } else {
                        let path = checked(shape_path(tag.name, tag.attrs))?;
                        if in_clip {
                            checked(ensure_closed_clip(&path))?;
                        }
                        checked(validate_path_geometry(&path, transform, require_area))?;
                        checked(validate_path_geometry(
                            &path,
                            physical_transform,
                            require_area,
                        ))?;
                        let segment_count = u64::try_from(path.segments.len())
                            .map_err(|_| ResourceAdmissionError::VectorPathSegmentLimit)?;
                        (Some(path), segment_count)
                    };
                    counts.stored_segments = counts
                        .stored_segments
                        .checked_add(segment_count)
                        .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?;
                    if scan_limits
                        .is_some_and(|limits| counts.stored_segments > limits.stored_segments)
                    {
                        return Err(ResourceAdmissionError::VectorPathSegmentLimit);
                    }
                    if in_clip {
                        let fill_rule = match tag.attrs.get("fill-rule") {
                            None | Some("nonzero") => SafeVectorFillRule::NonZero,
                            Some("evenodd") => SafeVectorFillRule::EvenOdd,
                            _ => return Err(error(SafeVectorFailureReason::MalformedSvg)),
                        };
                        if mode != ScanMode::Count {
                            let path =
                                path.ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                            let id = parent
                                .clip_id
                                .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                            definitions.push((
                                id,
                                SafeVectorClipDefinition {
                                    clip_id: u32::try_from(definitions.len())
                                        .map_err(|_| ResourceAdmissionError::VectorNodeLimit)?,
                                    transform: local,
                                    fill_rule,
                                    path,
                                },
                            ));
                        }
                    } else {
                        let paint =
                            paint.ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                        let clip_ref = parse_clip_ref_v2(tag.attrs.get("clip-path"))?;
                        if let Some(id) = clip_ref {
                            counts.source_clip_id_bytes = counts
                                .source_clip_id_bytes
                                .checked_add(id.len() as u64)
                                .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                            counts.commands = counts
                                .commands
                                .checked_add(2)
                                .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                            if mode != ScanMode::Count {
                                references.push(id);
                            }
                            if mode == ScanMode::Analyze {
                                resolve_clip_use_v2(&definitions, id, transform, root_geometry)?;
                            }
                        }
                        counts.commands = counts
                            .commands
                            .checked_add(1)
                            .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                        if visible {
                            stack[depth - 1].has_visible_draw = true;
                        }
                        if mode == ScanMode::Build {
                            let path =
                                path.ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                            let mut clips = Vec::new();
                            for frame in &stack[..depth] {
                                if let Some(id) = frame.clip_ref {
                                    clips.push(resolve_clip_use_v2(
                                        &definitions,
                                        id,
                                        frame.transform,
                                        root_geometry,
                                    )?);
                                }
                            }
                            if let Some(id) = clip_ref {
                                clips.push(resolve_clip_use_v2(
                                    &definitions,
                                    id,
                                    transform,
                                    root_geometry,
                                )?);
                            }
                            draws.push(SafeVectorDrawV2 {
                                transform,
                                clips,
                                path,
                                fill: paint.fill_layer(),
                                stroke: paint.stroke_value(),
                                fill_rule: paint.fill_rule,
                            });
                        }
                    }
                    stack[depth - 1].child_count = stack[depth - 1]
                        .child_count
                        .checked_add(1)
                        .ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?;
                }
            }
        }
        if depth != 0 || !root_closed {
            return Err(error(SafeVectorFailureReason::MalformedSvg));
        }
        Ok(ScanResultV2 {
            counts,
            root: root.ok_or_else(|| error(SafeVectorFailureReason::MalformedSvg))?,
            definitions,
            draws,
            references,
        })
    }

    #[cfg(test)]
    pub(crate) fn decode_v2(
        bytes: &[u8],
        limits: &M4EffectiveResourceLimits,
    ) -> Result<DecodedSafeVectorV2, ResourceAdmissionError> {
        let extension = limits.extension().get();
        decode_v2_with_work_budget(
            bytes,
            limits,
            extension.max_vector_nodes,
            extension.max_vector_path_segments,
        )
    }

    pub(crate) fn decode_v2_with_work_budget(
        bytes: &[u8],
        limits: &M4EffectiveResourceLimits,
        node_budget: u64,
        path_work_budget: u64,
    ) -> Result<DecodedSafeVectorV2, ResourceAdmissionError> {
        let extension = limits.extension().get();
        if node_budget > extension.max_vector_nodes
            || path_work_budget > extension.max_vector_path_segments
        {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
        }

        // Pass 1 is allocation-free apart from the fixed scanner/stack. It
        // proves all cardinalities and the /2 charge before IR vectors exist.
        let counted = scan_v2(
            bytes,
            ScanMode::Count,
            Some(ScanLimits {
                nodes: node_budget,
                stored_segments: path_work_budget,
                depth: extension.max_vector_nesting_depth,
            }),
        )?;
        if counted.counts.nodes > node_budget {
            return Err(ResourceAdmissionError::VectorNodeLimit);
        }
        if counted.counts.max_depth > extension.max_vector_nesting_depth {
            return Err(ResourceAdmissionError::VectorNestingLimit);
        }
        if counted.counts.stored_segments > path_work_budget {
            return Err(ResourceAdmissionError::VectorPathSegmentLimit);
        }
        let allocation_charge = allocation_charge(counted.counts)?;
        if allocation_charge > limits.base().get().max_decoded_image_bytes {
            return Err(ResourceAdmissionError::DecodedImageLimit);
        }

        // Pass 2 closes local clip definitions/references and charges replay.
        let analyzed = scan_v2(bytes, ScanMode::Analyze, None)?;
        if analyzed.counts != counted.counts || analyzed.root != counted.root {
            return Err(error(SafeVectorFailureReason::MalformedSvg));
        }
        let mut id_map = BTreeMap::new();
        let mut used = BTreeSet::new();
        for (id, definition) in &analyzed.definitions {
            if id_map.insert(*id, definition).is_some() {
                return Err(error(SafeVectorFailureReason::ForbiddenFeature));
            }
        }
        let mut replay = 0u64;
        for reference in &analyzed.references {
            let definition = id_map
                .get(reference)
                .ok_or_else(|| error(SafeVectorFailureReason::ForbiddenFeature))?;
            used.insert(*reference);
            replay = replay
                .checked_add(definition.path.segments.len() as u64)
                .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?;
            if counted
                .counts
                .stored_segments
                .checked_add(replay)
                .map_or(true, |work| work > path_work_budget)
            {
                return Err(ResourceAdmissionError::VectorPathSegmentLimit);
            }
        }
        if used.len() != id_map.len() {
            return Err(error(SafeVectorFailureReason::ForbiddenFeature));
        }
        let path_work = counted
            .counts
            .stored_segments
            .checked_add(replay)
            .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?;
        if path_work > path_work_budget {
            return Err(ResourceAdmissionError::VectorPathSegmentLimit);
        }
        drop(id_map);
        drop(used);
        drop(analyzed);

        // Pass 3 builds the exact canonical /2 IR only after all limits close.
        let built = scan_v2(bytes, ScanMode::Build, None)?;
        if built.counts != counted.counts || built.root != counted.root || built.draws.is_empty() {
            return Err(error(SafeVectorFailureReason::MalformedSvg));
        }
        if !built
            .draws
            .iter()
            .any(|draw| draw.fill.is_visible() || draw.stroke.paint().is_visible())
        {
            return Err(error(SafeVectorFailureReason::ForbiddenFeature));
        }
        let definitions: Vec<_> = built
            .definitions
            .into_iter()
            .map(|(_, definition)| definition)
            .collect();
        let canonical_jcs = encode_ir_v2(
            built.root,
            &definitions,
            &built.draws,
            built.counts,
            path_work,
            allocation_charge,
        );
        let fingerprint_jcs = format!(
            "{{\"algorithm\":\"{}\",\"ir\":{}}}",
            SAFE_VECTOR_IR_FINGERPRINT_ID_V2, canonical_jcs
        );
        let ir = SafeVectorIrV2 {
            intrinsic_width: built.root.width,
            intrinsic_height: built.root.height,
            view_box: built.root.view_box,
            root_scale: built.root.root_scale,
            clips: definitions,
            draws: built.draws,
            node_count: built.counts.nodes,
            stored_segment_count: built.counts.stored_segments,
            path_work,
            allocation_charge,
            fingerprint: sha256(fingerprint_jcs.as_bytes()),
            canonical_jcs,
        };
        Ok(DecodedSafeVectorV2 {
            work: SafeVectorWork {
                nodes: ir.node_count,
                path_work: ir.path_work,
            },
            ir,
        })
    }

    fn encode_ir_v2(
        root: RootGeometry,
        clips: &[SafeVectorClipDefinition],
        draws: &[SafeVectorDrawV2],
        counts: Counts,
        path_work: u64,
        allocation_charge: u64,
    ) -> String {
        let mut output = String::from("{\"algorithm\":");
        push_jcs_string(&mut output, SAFE_VECTOR_IR_ID_V2);
        output.push_str(",\"allocation\":{\"algorithm\":");
        push_jcs_string(&mut output, SAFE_VECTOR_ALLOCATION_CHARGE_ID_V2);
        output.push_str(",\"charge\":");
        output.push_str(&allocation_charge.to_string());
        output.push_str(",\"nodes\":");
        output.push_str(&counts.nodes.to_string());
        output.push_str(",\"paint_or_clip_commands\":");
        output.push_str(&counts.commands.to_string());
        output.push_str(",\"source_clip_id_bytes\":");
        output.push_str(&counts.source_clip_id_bytes.to_string());
        output.push_str(",\"stored_segments\":");
        output.push_str(&counts.stored_segments.to_string());
        output.push_str("},\"clips\":[");
        for (index, clip) in clips.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str("{\"clip_id\":");
            output.push_str(&clip.clip_id.to_string());
            output.push_str(",\"fill_rule\":");
            push_jcs_string(&mut output, clip.fill_rule.as_str());
            output.push_str(",\"path\":");
            encode_path(&mut output, &clip.path);
            output.push_str(",\"transform\":");
            encode_transform(&mut output, clip.transform);
            output.push('}');
        }
        output.push_str("],\"draws\":[");
        for (index, draw) in draws.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str("{\"clips\":[");
            for (clip_index, clip) in draw.clips.iter().enumerate() {
                if clip_index > 0 {
                    output.push(',');
                }
                output.push_str("{\"clip_id\":");
                output.push_str(&clip.clip_id.to_string());
                output.push_str(",\"transform\":");
                encode_transform(&mut output, clip.transform);
                output.push('}');
            }
            output.push_str("],\"fill\":");
            encode_paint_layer(&mut output, draw.fill);
            output.push_str(",\"fill_rule\":");
            push_jcs_string(&mut output, draw.fill_rule.as_str());
            output.push_str(",\"path\":");
            encode_path(&mut output, &draw.path);
            output.push_str(",\"stroke\":{\"alpha\":");
            output.push_str(&draw.stroke.paint.alpha.raw().to_string());
            output.push_str(",\"line_cap\":");
            push_jcs_string(&mut output, draw.stroke.line_cap.as_str());
            output.push_str(",\"line_join\":");
            push_jcs_string(&mut output, draw.stroke.line_join.as_str());
            output.push_str(",\"miter_limit\":");
            output.push_str(&draw.stroke.miter_limit.to_string());
            output.push_str(",\"paint\":");
            encode_paint(&mut output, draw.stroke.paint.paint);
            output.push_str(",\"width\":");
            output.push_str(&draw.stroke.width.to_string());
            output.push_str("},\"transform\":");
            encode_transform(&mut output, draw.transform);
            output.push('}');
        }
        output.push_str("],\"intrinsic_height\":");
        output.push_str(&root.height.get().raw().to_string());
        output.push_str(",\"intrinsic_width\":");
        output.push_str(&root.width.get().raw().to_string());
        output.push_str(",\"parser\":");
        push_jcs_string(&mut output, SAFE_SVG_PARSER_ID_V2);
        output.push_str(",\"path_work\":");
        output.push_str(&path_work.to_string());
        output.push_str(",\"root_scale\":");
        output.push_str(&root.root_scale.to_string());
        output.push_str(",\"root_viewport_clip\":\"outermost\",\"view_box\":[");
        for (index, value) in root.view_box.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str(&value.to_string());
        }
        output.push_str("]}");
        output
    }

    fn encode_paint_layer(output: &mut String, layer: SafeVectorPaintLayer) {
        output.push_str("{\"alpha\":");
        output.push_str(&layer.alpha.raw().to_string());
        output.push_str(",\"paint\":");
        encode_paint(output, layer.paint);
        output.push('}');
    }

    fn encode_paint(output: &mut String, paint: SafeVectorPaint) {
        output.push_str("{\"kind\":");
        push_jcs_string(output, paint.as_str());
        if let SafeVectorPaint::FixedRgb8(rgb) = paint {
            output.push_str(",\"rgb\":[");
            output.push_str(&rgb[0].to_string());
            output.push(',');
            output.push_str(&rgb[1].to_string());
            output.push(',');
            output.push_str(&rgb[2].to_string());
            output.push(']');
        }
        output.push('}');
    }
}

pub(crate) use v2::decode_v2_with_work_budget;
pub use v2::{
    SafeVectorAlpha, SafeVectorDrawV2, SafeVectorIrV2, SafeVectorPaint, SafeVectorPaintLayer,
    SafeVectorStrokeV2,
};

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{M4ResourceLimits, ResourceLimits, ValidatedResourceLimits};

    fn limits(extension: M4ResourceLimits) -> M4EffectiveResourceLimits {
        M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
            extension,
        )
        .unwrap()
    }

    const ALLOWED: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="80pt" height="40pt" viewBox="0 0 80 40"><defs><clipPath id="frame"><rect x="1" y="1" width="78" height="38"/></clipPath></defs><g fill="#1256Aa" stroke="#000000" stroke-width="1" clip-path="url(#frame)" transform="translate(1 1) scale(0.95)"><path d="M 2 2 L 20 2 Q 25 2 25 7 C 25 10 20 12 15 12 Z"/><circle cx="40" cy="12" r="5"/><ellipse cx="55" cy="12" rx="7" ry="4"/><line x1="2" y1="25" x2="20" y2="25" fill="none"/><polyline points="25 25 30 30 35 25" fill="none"/><polygon points="42 25 48 32 54 25"/></g></svg>"##;

    const ALLOWED_V2: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10pt" height="10pt" viewBox="0 0 10 10"><defs><clipPath id="c"><rect width="10" height="10"/></clipPath></defs><g fill="currentColor" fill-opacity="0.25" stroke="#1234Ab" stroke-opacity="0.5" stroke-width="1" clip-path="url(#c)"><rect x="1" y="1" width="3" height="3"/><rect x="5" y="5" width="3" height="3" fill-opacity="1.000000" stroke="none"/></g></svg>"##;

    #[test]
    fn safe_svg_2_profile_is_nominal_and_ir_is_canonical() {
        assert_eq!(
            SafeVectorParserProfile::SafeSvg1.parser_id(),
            SAFE_SVG_PARSER_ID
        );
        assert_eq!(
            SafeVectorParserProfile::SafeSvg2.parser_id(),
            SAFE_SVG_PARSER_ID_V2
        );
        assert_eq!(SafeVectorParserProfile::SafeSvg1.ir_id(), SAFE_VECTOR_IR_ID);
        assert_eq!(
            SafeVectorParserProfile::SafeSvg2.ir_id(),
            SAFE_VECTOR_IR_ID_V2
        );
        assert_eq!(
            SafeVectorParserProfile::SafeSvg1.ir_fingerprint_id(),
            SAFE_VECTOR_IR_FINGERPRINT_ID
        );
        assert_eq!(
            SafeVectorParserProfile::SafeSvg2.ir_fingerprint_id(),
            SAFE_VECTOR_IR_FINGERPRINT_ID_V2
        );
        assert_eq!(
            SafeVectorParserProfile::SafeSvg1.allocation_charge_id(),
            SAFE_VECTOR_ALLOCATION_CHARGE_ID
        );
        assert_eq!(
            SafeVectorParserProfile::SafeSvg2.allocation_charge_id(),
            SAFE_VECTOR_ALLOCATION_CHARGE_ID_V2
        );

        let limits = limits(M4ResourceLimits::default());
        let first = v2::decode_v2(ALLOWED_V2, &limits).unwrap();
        let second = v2::decode_v2(ALLOWED_V2, &limits).unwrap();
        assert_eq!(first.ir, second.ir);
        assert_eq!(first.ir.parser_profile(), SafeVectorParserProfile::SafeSvg2);
        assert!(first.ir.root_viewport_clip_is_outermost());
        assert_eq!(first.ir.clips().len(), 1);
        assert_eq!(first.ir.draws().len(), 2);
        assert_eq!(
            first.ir.draws()[0].fill().paint(),
            v2::SafeVectorPaint::CurrentColor
        );
        assert_eq!(first.ir.draws()[0].fill().alpha().raw(), 16_384);
        assert_eq!(
            first.ir.draws()[0].stroke().paint().paint(),
            v2::SafeVectorPaint::FixedRgb8([0x12, 0x34, 0xab])
        );
        assert_eq!(first.ir.draws()[0].stroke().paint().alpha().raw(), 32_768);
        assert_eq!(first.ir.draws()[1].fill().alpha().raw(), 65_536);
        assert_eq!(
            first.ir.draws()[1].stroke().paint().paint(),
            v2::SafeVectorPaint::None
        );
        assert!(first.ir.canonical_jcs().contains(SAFE_SVG_PARSER_ID_V2));
        assert!(first.ir.canonical_jcs().contains(SAFE_VECTOR_IR_ID_V2));
        assert!(first
            .ir
            .canonical_jcs()
            .contains(SAFE_VECTOR_ALLOCATION_CHARGE_ID_V2));
        assert!(first
            .ir
            .canonical_jcs()
            .contains("\"root_viewport_clip\":\"outermost\""));
        let expected_charge =
            64 * first.ir.node_count() + 80 * first.ir.stored_segment_count() + 48 * 6 + 2;
        assert_eq!(first.ir.allocation_charge(), expected_charge);

        let v1_for_v1 = decode(ALLOWED, &limits).unwrap();
        let v2_for_same_bytes = v2::decode_v2(ALLOWED, &limits).unwrap();
        assert_ne!(
            v1_for_v1.ir.fingerprint(),
            v2_for_same_bytes.ir.fingerprint()
        );
        assert!(v1_for_v1.ir.canonical_jcs().contains(SAFE_VECTOR_IR_ID));
        assert!(v2_for_same_bytes
            .ir
            .canonical_jcs()
            .contains(SAFE_VECTOR_IR_ID_V2));

        assert_eq!(
            decode(ALLOWED_V2, &limits),
            Err(ResourceAdmissionError::InvalidSafeVector)
        );
    }

    #[test]
    fn safe_svg_2_vmb_positive_corpus_is_admitted_without_fallback() {
        let limits = limits(M4ResourceLimits::default());
        for (name, bytes) in [
            (
                "aligned",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/aligned.svg"
                ))
                .as_slice(),
            ),
            (
                "fraction-equality",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/fraction-equality.svg"
                ))
                .as_slice(),
            ),
            (
                "integral",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/integral.svg"
                ))
                .as_slice(),
            ),
            (
                "large-brackets",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/large-brackets.svg"
                ))
                .as_slice(),
            ),
            (
                "long-block",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/long-block.svg"
                ))
                .as_slice(),
            ),
            (
                "matrix",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/matrix.svg"
                ))
                .as_slice(),
            ),
            (
                "not-divides",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/not-divides.svg"
                ))
                .as_slice(),
            ),
            (
                "ordered-pair",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/ordered-pair.svg"
                ))
                .as_slice(),
            ),
            (
                "scripts",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/scripts.svg"
                ))
                .as_slice(),
            ),
            (
                "similar",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/similar.svg"
                ))
                .as_slice(),
            ),
            (
                "sum",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/sum.svg"
                ))
                .as_slice(),
            ),
            (
                "x-plus-y",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/x-plus-y.svg"
                ))
                .as_slice(),
            ),
        ] {
            let first = v2::decode_v2(bytes, &limits)
                .unwrap_or_else(|failure| panic!("{name} was rejected: {failure:?}"));
            let second = v2::decode_v2(bytes, &limits).unwrap();
            assert_eq!(first.ir, second.ir, "{name}");
            assert!(!first.ir.draws().is_empty(), "{name}");
        }
    }

    #[test]
    fn safe_svg_2_checked_in_negative_corpus_has_exact_typed_reasons() {
        let limits = limits(M4ResourceLimits::default());
        let manifest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/negative.tsv"
        ));
        let mut rows = manifest.lines();
        assert_eq!(rows.next(), Some("case_id\texpected_reason\tsvg_path"));
        for (name, path, bytes, reason) in [
            (
                "clip-alpha",
                "negative-svg/clip-alpha.svg",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/negative-svg/clip-alpha.svg"
                ))
                .as_slice(),
                SafeVectorFailureReason::UnsupportedFeature,
            ),
            (
                "external-image",
                "negative-svg/external-image.svg",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/negative-svg/external-image.svg"
                ))
                .as_slice(),
                SafeVectorFailureReason::ExternalReference,
            ),
            (
                "forbidden-script",
                "negative-svg/forbidden-script.svg",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/negative-svg/forbidden-script.svg"
                ))
                .as_slice(),
                SafeVectorFailureReason::ForbiddenFeature,
            ),
            (
                "invalid-alpha",
                "negative-svg/invalid-alpha.svg",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/negative-svg/invalid-alpha.svg"
                ))
                .as_slice(),
                SafeVectorFailureReason::MalformedSvg,
            ),
            (
                "malformed-unclosed",
                "negative-svg/malformed-unclosed.svg",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/negative-svg/malformed-unclosed.svg"
                ))
                .as_slice(),
                SafeVectorFailureReason::MalformedSvg,
            ),
            (
                "unsupported-text",
                "negative-svg/unsupported-text.svg",
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/negative-svg/unsupported-text.svg"
                ))
                .as_slice(),
                SafeVectorFailureReason::UnsupportedFeature,
            ),
        ] {
            let row = rows.next().expect("negative corpus row must exist");
            let columns: Vec<_> = row.split('\t').collect();
            assert_eq!(columns.as_slice(), [name, reason.as_str(), path]);
            assert_eq!(
                v2::decode_v2(bytes, &limits),
                Err(ResourceAdmissionError::InvalidSafeVectorV2(reason)),
                "{name}"
            );
        }
        assert_eq!(rows.next(), None);
    }

    #[test]
    fn safe_svg_2_opacity_lexical_and_replacement_rules_are_closed() {
        let limits = limits(M4ResourceLimits::default());
        let wrap = |attribute: &str| {
            format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1pt\" height=\"1pt\" viewBox=\"0 0 1 1\"><rect width=\"1\" height=\"1\" stroke=\"#000000\" {attribute}/></svg>"
            )
        };
        for value in ["0", "1", "0.0", "0.000001", "0.999999", "1.0", "1.000000"] {
            assert!(v2::decode_v2(
                wrap(&format!("fill-opacity=\"{value}\"")).as_bytes(),
                &limits
            )
            .is_ok());
        }
        for value in [
            "",
            ".5",
            "+0.5",
            "-0",
            "00",
            "01",
            "0.",
            "0.0000000",
            "1.",
            "1.000001",
            "2",
            "5e-1",
            " 0.5",
            "0.5 ",
        ] {
            assert_eq!(
                v2::decode_v2(
                    wrap(&format!("fill-opacity=\"{value}\"")).as_bytes(),
                    &limits
                ),
                Err(ResourceAdmissionError::InvalidSafeVectorV2(
                    SafeVectorFailureReason::MalformedSvg
                )),
                "value {value:?}"
            );
        }

        let replacement = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2pt" height="1pt" viewBox="0 0 2 1"><g fill-opacity="0.25"><rect width="1" height="1"/><g fill-opacity="0.5"><rect x="1" width="1" height="1"/></g></g></svg>"#;
        let ir = v2::decode_v2(replacement, &limits).unwrap().ir;
        assert_eq!(ir.draws()[0].fill().alpha().raw(), 16_384);
        assert_eq!(ir.draws()[1].fill().alpha().raw(), 32_768);
    }

    #[test]
    fn safe_svg_2_paint_and_compositing_features_fail_closed() {
        let limits = limits(M4ResourceLimits::default());
        let attribute = |attribute: &str| {
            format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1pt\" height=\"1pt\" viewBox=\"0 0 1 1\"><rect width=\"1\" height=\"1\" {attribute}/></svg>"
            )
        };
        for paint in [
            "CurrentColor",
            "currentcolor",
            " currentColor",
            "currentColor ",
            "inherit",
            "var(--math)",
            "red",
            "#abcd",
        ] {
            assert_eq!(
                v2::decode_v2(attribute(&format!("fill=\"{paint}\"")).as_bytes(), &limits),
                Err(ResourceAdmissionError::InvalidSafeVectorV2(
                    SafeVectorFailureReason::ForbiddenFeature
                )),
                "paint {paint:?}"
            );
        }
        for forbidden in [
            "color=\"#000000\"",
            "style=\"fill:#000000\"",
            "class=\"math\"",
            "onclick=\"paint()\"",
        ] {
            assert_eq!(
                v2::decode_v2(attribute(forbidden).as_bytes(), &limits),
                Err(ResourceAdmissionError::InvalidSafeVectorV2(
                    SafeVectorFailureReason::ForbiddenFeature
                )),
                "attribute {forbidden:?}"
            );
        }
        for unsupported in [
            "opacity=\"0.5\"",
            "mask=\"url(#m)\"",
            "filter=\"url(#f)\"",
            "isolation=\"isolate\"",
            "mix-blend-mode=\"multiply\"",
            "data-unknown=\"value\"",
        ] {
            assert_eq!(
                v2::decode_v2(attribute(unsupported).as_bytes(), &limits),
                Err(ResourceAdmissionError::InvalidSafeVectorV2(
                    SafeVectorFailureReason::UnsupportedFeature
                )),
                "attribute {unsupported:?}"
            );
        }
        assert_eq!(
            v2::decode_v2(
                attribute("fill=\"url(https://example.invalid/p)\"").as_bytes(),
                &limits
            ),
            Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::ExternalReference
            ))
        );
        let clip_alpha = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><defs><clipPath id="c"><rect width="1" height="1" fill-opacity="0.5"/></clipPath></defs><rect width="1" height="1" clip-path="url(#c)"/></svg>"#;
        assert_eq!(
            v2::decode_v2(clip_alpha, &limits),
            Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::UnsupportedFeature
            ))
        );
    }

    #[test]
    fn safe_svg_2_elements_references_and_empty_paint_are_typed_failures() {
        let limits = limits(M4ResourceLimits::default());
        for body in [
            "<script></script>",
            "<style></style>",
            "<animate></animate>",
            "<animateTransform></animateTransform>",
            "<set></set>",
        ] {
            let svg = format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1pt\" height=\"1pt\" viewBox=\"0 0 1 1\">{body}</svg>"
            );
            assert_eq!(
                v2::decode_v2(svg.as_bytes(), &limits),
                Err(ResourceAdmissionError::InvalidSafeVectorV2(
                    SafeVectorFailureReason::ForbiddenFeature
                ))
            );
        }
        for body in [
            "<foreignObject></foreignObject>",
            "<text></text>",
            "<tspan></tspan>",
            "<font></font>",
            "<use></use>",
            "<symbol></symbol>",
            "<linearGradient></linearGradient>",
            "<radialGradient></radialGradient>",
            "<pattern></pattern>",
            "<marker></marker>",
            "<mask></mask>",
            "<filter></filter>",
            "<image></image>",
            "<unknown></unknown>",
        ] {
            let svg = format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1pt\" height=\"1pt\" viewBox=\"0 0 1 1\">{body}</svg>"
            );
            assert_eq!(
                v2::decode_v2(svg.as_bytes(), &limits),
                Err(ResourceAdmissionError::InvalidSafeVectorV2(
                    SafeVectorFailureReason::UnsupportedFeature
                )),
                "body {body}"
            );
        }
        let external = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><image href="https://example.invalid/math.svg"></image></svg>"#;
        assert_eq!(
            v2::decode_v2(external, &limits),
            Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::ExternalReference
            ))
        );
        for forbidden_document in [
            br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1"/></svg>"#.as_slice(),
            br#"<!DOCTYPE svg><svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1" data-x="&entity;"/></svg>"#,
        ] {
            assert_eq!(
                v2::decode_v2(forbidden_document, &limits),
                Err(ResourceAdmissionError::InvalidSafeVectorV2(
                    SafeVectorFailureReason::ForbiddenFeature
                ))
            );
        }
        for svg in [
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1" fill="none"/></svg>"#.as_slice(),
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1" fill-opacity="0"/></svg>"#,
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1" fill="none" stroke="#000000" stroke-opacity="0"/></svg>"##,
        ] {
            assert_eq!(
                v2::decode_v2(svg, &limits),
                Err(ResourceAdmissionError::InvalidSafeVectorV2(
                    SafeVectorFailureReason::ForbiddenFeature
                ))
            );
        }
    }

    #[test]
    fn safe_svg_2_clip_closure_and_limits_are_bounded_and_inclusive() {
        let limits_default = limits(M4ResourceLimits::default());
        let baseline = v2::decode_v2(ALLOWED_V2, &limits_default).unwrap();
        let exact_extension = M4ResourceLimits {
            max_vector_nodes: baseline.work.nodes,
            max_vector_path_segments: baseline.work.path_work,
            max_vector_nesting_depth: 4,
            ..M4ResourceLimits::default()
        };
        assert!(v2::decode_v2(ALLOWED_V2, &limits(exact_extension)).is_ok());
        assert_eq!(
            v2::decode_v2(
                ALLOWED_V2,
                &limits(M4ResourceLimits {
                    max_vector_nodes: baseline.work.nodes - 1,
                    ..exact_extension
                })
            ),
            Err(ResourceAdmissionError::VectorNodeLimit)
        );
        assert_eq!(
            v2::decode_v2(
                ALLOWED_V2,
                &limits(M4ResourceLimits {
                    max_vector_path_segments: baseline.work.path_work - 1,
                    ..exact_extension
                })
            ),
            Err(ResourceAdmissionError::VectorPathSegmentLimit)
        );
        assert_eq!(
            v2::decode_v2(
                ALLOWED_V2,
                &limits(M4ResourceLimits {
                    max_vector_nesting_depth: exact_extension.max_vector_nesting_depth - 1,
                    ..exact_extension
                })
            ),
            Err(ResourceAdmissionError::VectorNestingLimit)
        );
        let exact_base = ResourceLimits {
            max_decoded_image_bytes: baseline.ir.allocation_charge(),
            ..ResourceLimits::default()
        };
        let exact = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(exact_base).unwrap(),
            exact_extension,
        )
        .unwrap();
        assert!(v2::decode_v2(ALLOWED_V2, &exact).is_ok());
        let too_small = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(ResourceLimits {
                max_decoded_image_bytes: baseline.ir.allocation_charge() - 1,
                ..ResourceLimits::default()
            })
            .unwrap(),
            exact_extension,
        )
        .unwrap();
        assert_eq!(
            v2::decode_v2(ALLOWED_V2, &too_small),
            Err(ResourceAdmissionError::DecodedImageLimit)
        );

        for svg in [
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><g clip-path="url(#later)"><rect width="1" height="1"/></g><defs><clipPath id="later"><rect width="1" height="1"/></clipPath></defs></svg>"#.as_slice(),
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><defs><clipPath id="unused"><rect width="1" height="1"/></clipPath></defs><rect width="1" height="1"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><defs><clipPath id="cycle" clip-path="url(#cycle)"><rect width="1" height="1"/></clipPath></defs><rect width="1" height="1" clip-path="url(#cycle)"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1" clip-path="url(#missing)"/></svg>"#,
        ] {
            assert!(matches!(
                v2::decode_v2(svg, &limits_default),
                Err(ResourceAdmissionError::InvalidSafeVectorV2(_))
            ));
        }
        let external_clip = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1" clip-path="url(https://example.invalid/c.svg#x)"/></svg>"#;
        assert_eq!(
            v2::decode_v2(external_clip, &limits_default),
            Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::ExternalReference
            ))
        );

        let malformed_path = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><path d="M 0"/></svg>"#;
        assert_eq!(
            v2::decode_v2(malformed_path, &limits_default),
            Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::MalformedSvg
            ))
        );
        let unsupported_path = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><path d="M 0 0 A 1 1 0 0 0 1 1"/></svg>"#;
        assert_eq!(
            v2::decode_v2(unsupported_path, &limits_default),
            Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::UnsupportedFeature
            ))
        );
    }

    #[test]
    fn safe_svg_2_invalid_inputs_are_total_and_nesting_is_bounded() {
        let limits = limits(M4ResourceLimits::default());
        for end in 0..ALLOWED_V2.len() {
            let outcome = std::panic::catch_unwind(|| v2::decode_v2(&ALLOWED_V2[..end], &limits));
            assert!(
                matches!(outcome, Ok(Err(_))),
                "parser accepted or panicked for prefix ending at {end}"
            );
        }
        for bytes in [
            b"\xff\xfe\xfd".as_slice(),
            b"<".as_slice(),
            b"<svg".as_slice(),
            b"<svg>&entity;</svg>".as_slice(),
        ] {
            assert!(matches!(
                std::panic::catch_unwind(|| v2::decode_v2(bytes, &limits)),
                Ok(Err(_))
            ));
        }

        let mut nested = String::from(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1pt\" height=\"1pt\" viewBox=\"0 0 1 1\">",
        );
        for _ in 0..HARD_STACK_DEPTH {
            nested.push_str("<g>");
        }
        nested.push_str("<rect width=\"1\" height=\"1\"/>");
        for _ in 0..HARD_STACK_DEPTH {
            nested.push_str("</g>");
        }
        nested.push_str("</svg>");
        let outcome = std::panic::catch_unwind(|| v2::decode_v2(nested.as_bytes(), &limits));
        assert_eq!(
            outcome.unwrap(),
            Err(ResourceAdmissionError::VectorNestingLimit)
        );
    }

    #[test]
    fn safe_svg_1_frozen_canonical_ir_fingerprint_and_charge() {
        let decoded = decode(ALLOWED, &limits(M4ResourceLimits::default())).unwrap();
        assert_eq!(decoded.ir.allocation_charge(), 3_914);
        assert_eq!(
            sha256(decoded.ir.canonical_jcs().as_bytes()),
            [
                138, 41, 162, 93, 49, 147, 170, 84, 68, 90, 60, 19, 85, 96, 130, 53, 8, 29, 226,
                172, 225, 59, 28, 65, 243, 183, 170, 71, 118, 188, 66, 133,
            ]
        );
        assert_eq!(
            decoded.ir.fingerprint(),
            [
                211, 39, 112, 2, 226, 176, 202, 14, 56, 220, 118, 155, 32, 49, 0, 163, 192, 144,
                194, 216, 10, 68, 243, 229, 50, 40, 160, 67, 33, 235, 171, 82,
            ]
        );

        let fixture = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/vector-media/job/art.vector"
        ));
        assert_eq!(
            sha256(fixture),
            [
                65, 44, 171, 109, 60, 154, 149, 180, 170, 146, 12, 216, 217, 198, 3, 11, 239, 100,
                37, 123, 243, 254, 167, 236, 134, 76, 70, 12, 193, 55, 251, 53,
            ]
        );
        let fixture_ir = decode(fixture, &limits(M4ResourceLimits::default()))
            .unwrap()
            .ir;
        assert_eq!(fixture_ir.allocation_charge(), 2_330);
        assert_eq!(
            fixture_ir.fingerprint(),
            [
                108, 45, 194, 188, 209, 250, 54, 136, 208, 91, 249, 254, 75, 84, 172, 225, 194, 13,
                90, 32, 116, 130, 140, 30, 141, 174, 161, 140, 150, 7, 175, 211,
            ]
        );
    }

    #[test]
    fn vector_allowed_subset_is_deterministic_and_canonical() {
        let limits = limits(M4ResourceLimits::default());
        let first = decode(ALLOWED, &limits).unwrap();
        let second = decode(ALLOWED, &limits).unwrap();
        assert_eq!(first.ir, second.ir);
        assert_eq!(first.ir.clips().len(), 1);
        assert_eq!(first.ir.draws().len(), 6);
        assert_eq!(first.ir.intrinsic_width().get().raw(), 80 * FIXED_ONE);
        assert_eq!(first.ir.intrinsic_height().get().raw(), 40 * FIXED_ONE);
        assert!(first.ir.canonical_jcs().contains(SAFE_VECTOR_IR_ID));

        let repeated_separators = std::str::from_utf8(ALLOWED)
            .unwrap()
            .replace("viewBox=\"0 0 80 40\"", "viewBox=\"0  0\t80\n40\"")
            .replace(
                "translate(1 1) scale(0.95)",
                "translate(1  \t1)\n scale(0.95)",
            )
            .replace(
                "points=\"25 25 30 30 35 25\"",
                "points=\"25  25\t30\n30 35 25\"",
            );
        assert_eq!(
            decode(repeated_separators.as_bytes(), &limits).unwrap().ir,
            first.ir
        );

        let translated_clip = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><defs><clipPath id="c"><rect x="999999" y="999999" width="1" height="1"/></clipPath></defs><g transform="translate(-999999 -999999)" clip-path="url(#c)"><rect x="999999" y="999999" width="1" height="1"/></g></svg>"#;
        assert!(decode(translated_clip, &limits).is_ok());
    }

    #[test]
    fn vector_forbidden_and_unknown_features_fail_closed() {
        let limits = limits(M4ResourceLimits::default());
        for bytes in [
            br#"<?xml version="1.0"?><svg/>"#.as_slice(),
            br#"<!DOCTYPE svg [<!ENTITY bomb "boom">]><svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><script></script></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><image href="https://example.test/a"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1" style="fill:red"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:x="urn:x" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" width="1" height="1"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><text>x</text></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><path d="M 0 0 A 1 1 0 0 0 1 1"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1" onclick="x"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1" fill="url(#x)"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><g clip-path="url(#missing)"><rect width="1" height="1"/></g></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><defs><clipPath id="unused"><rect width="1" height="1"/></clipPath></defs><rect width="1" height="1"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><defs><clipPath id="same"><rect width="1" height="1"/></clipPath><clipPath id="same"><rect width="1" height="1"/></clipPath></defs><rect width="1" height="1" clip-path="url(#same)"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1" height="1" transform="matrix(1 0.000001 0 1 0 0)"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1.000001 1"><rect width="1" height="1"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1000pt" height="1000pt" viewBox="1000000 0 1000 1000"><rect x="1000000" width="1" height="1"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="32767pt" height="32767pt" viewBox="0 0 1 1"><rect x="100" y="100" width="1" height="1"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="32767pt" height="32767pt" viewBox="0 0 1 1"><defs><clipPath id="c"><rect x="100" y="100" width="1" height="1"/></clipPath></defs><rect width="1" height="1" clip-path="url(#c)"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><defs><clipPath id="c"><rect x="999999" y="999999" width="1" height="1"/></clipPath></defs><g clip-path="url(#c)" transform="scale(32767)"><rect width="0.0001" height="0.0001"/></g></svg>"#,
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><g stroke="#000000" stroke-width="1000000" transform="scale(32767)"><rect width="0.0001" height="0.0001"/></g></svg>"##,
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><rect width="1000000" height="1000000" fill="none" stroke="#000000" stroke-width="0.000015" transform="scale(0.000015)"/></svg>"##,
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="0.000015pt" height="0.000015pt" viewBox="0 0 1 1"><rect width="1" height="1" fill="none" stroke="#000000" stroke-width="0.000015"/></svg>"##,
        ] {
            assert_eq!(decode(bytes, &limits), Err(ResourceAdmissionError::InvalidSafeVector));
        }
    }

    #[test]
    fn vector_limits_are_inclusive_and_report_the_owning_code() {
        let baseline = decode(ALLOWED, &limits(M4ResourceLimits::default())).unwrap();
        let exact = M4ResourceLimits {
            max_vector_nodes: baseline.work.nodes,
            max_vector_path_segments: baseline.work.path_work,
            max_vector_nesting_depth: 4,
            ..M4ResourceLimits::default()
        };
        assert!(decode(ALLOWED, &limits(exact)).is_ok());
        assert_eq!(
            decode(
                ALLOWED,
                &limits(M4ResourceLimits {
                    max_vector_nodes: baseline.work.nodes - 1,
                    ..exact
                })
            ),
            Err(ResourceAdmissionError::VectorNodeLimit)
        );
        assert_eq!(
            decode(
                ALLOWED,
                &limits(M4ResourceLimits {
                    max_vector_path_segments: baseline.work.path_work - 1,
                    ..exact
                })
            ),
            Err(ResourceAdmissionError::VectorPathSegmentLimit)
        );
        assert_eq!(
            decode(
                ALLOWED,
                &limits(M4ResourceLimits {
                    max_vector_nesting_depth: 3,
                    ..exact
                })
            ),
            Err(ResourceAdmissionError::VectorNestingLimit)
        );

        let exact_base = ResourceLimits {
            max_decoded_image_bytes: baseline.ir.allocation_charge(),
            ..ResourceLimits::default()
        };
        let exact_allocation = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(exact_base.clone()).unwrap(),
            exact,
        )
        .unwrap();
        assert!(decode(ALLOWED, &exact_allocation).is_ok());
        let max_plus_one_base = ResourceLimits {
            max_decoded_image_bytes: baseline.ir.allocation_charge() - 1,
            ..ResourceLimits::default()
        };
        let max_plus_one_allocation = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(max_plus_one_base).unwrap(),
            exact,
        )
        .unwrap();
        assert_eq!(
            decode(ALLOWED, &max_plus_one_allocation),
            Err(ResourceAdmissionError::DecodedImageLimit)
        );
    }

    #[test]
    fn vector_clip_reference_replay_is_charged_per_reference() {
        const REPLAY: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10pt" height="10pt" viewBox="0 0 10 10"><defs><clipPath id="c"><rect width="10" height="10"/></clipPath></defs><rect width="4" height="4" clip-path="url(#c)"/><rect x="5" y="5" width="4" height="4" clip-path="url(#c)"/></svg>"#;
        let baseline = decode(REPLAY, &limits(M4ResourceLimits::default())).unwrap();
        assert_eq!(
            baseline.ir.path_work(),
            baseline.ir.stored_segment_count() + 2 * 5
        );
        let exact = M4ResourceLimits {
            max_vector_nodes: baseline.work.nodes,
            max_vector_path_segments: baseline.work.path_work,
            ..M4ResourceLimits::default()
        };
        assert!(decode(REPLAY, &limits(exact)).is_ok());
        assert_eq!(
            decode(
                REPLAY,
                &limits(M4ResourceLimits {
                    max_vector_path_segments: baseline.work.path_work - 1,
                    ..exact
                })
            ),
            Err(ResourceAdmissionError::VectorPathSegmentLimit)
        );
    }

    #[test]
    fn vector_clip_use_transform_is_rejected_before_ir_build() {
        const INVALID_USE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><defs><clipPath id="c"><rect x="999999" y="999999" width="1" height="1"/></clipPath></defs><g clip-path="url(#c)" transform="scale(32767)"><rect width="0.0001" height="0.0001"/></g></svg>"#;
        let extension = M4ResourceLimits::default();
        assert!(scan(
            INVALID_USE,
            ScanMode::Count,
            Some(ScanLimits {
                nodes: extension.max_vector_nodes,
                stored_segments: extension.max_vector_path_segments,
                depth: extension.max_vector_nesting_depth,
            }),
        )
        .is_ok());
        assert!(matches!(
            scan(INVALID_USE, ScanMode::Analyze, None),
            Err(ResourceAdmissionError::InvalidSafeVector)
        ));
    }
}
