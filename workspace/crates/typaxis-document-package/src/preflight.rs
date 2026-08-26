use crate::{
    JsonContainerContext, JsonContainerKind, JsonNumberKind, JsonPreflightError,
    JsonPreflightErrorKind, JsonPreflightLimitError, JsonPreflightLocation, JsonTokenKind,
    JsonTokenMetadata,
};
use std::collections::BTreeSet;
use typaxis_core::{JsonPointer, MachineInputLimitBounds, ValidatedResourceLimits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentPackageByteLimit(u64);

impl DocumentPackageByteLimit {
    pub const MINIMUM: u64 = 1;

    pub const fn new(value: u64) -> Result<Self, JsonPreflightLimitError> {
        if value < Self::MINIMUM || value > MachineInputLimitBounds::HARD_MAX_DOCUMENT_PACKAGE_BYTES
        {
            Err(JsonPreflightLimitError::DocumentPackageBytesOutOfRange {
                requested: value,
                minimum: Self::MINIMUM,
                maximum: MachineInputLimitBounds::HARD_MAX_DOCUMENT_PACKAGE_BYTES,
            })
        } else {
            Ok(Self(value))
        }
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JsonNestingDepthLimit(u16);

impl JsonNestingDepthLimit {
    pub const MINIMUM: u16 = 1;

    pub const fn new(value: u16) -> Result<Self, JsonPreflightLimitError> {
        if value < Self::MINIMUM || value > MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH {
            Err(JsonPreflightLimitError::JsonNestingDepthOutOfRange {
                requested: value,
                minimum: Self::MINIMUM,
                maximum: MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH,
            })
        } else {
            Ok(Self(value))
        }
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentPackagePreflightLimits {
    max_bytes: DocumentPackageByteLimit,
    max_depth: JsonNestingDepthLimit,
}

impl DocumentPackagePreflightLimits {
    pub const fn new(max_bytes: u64, max_depth: u16) -> Result<Self, JsonPreflightLimitError> {
        let max_bytes = match DocumentPackageByteLimit::new(max_bytes) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        let max_depth = match JsonNestingDepthLimit::new(max_depth) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        Ok(Self {
            max_bytes,
            max_depth,
        })
    }

    pub const fn max_bytes(self) -> DocumentPackageByteLimit {
        self.max_bytes
    }

    pub const fn max_depth(self) -> JsonNestingDepthLimit {
        self.max_depth
    }

    /// Project the package bounds from the validated effective configuration.
    /// Validation guarantees these values are inside the same hard bounds
    /// enforced by the staged scanner and capability descriptor.
    pub fn from_resource_limits(limits: &ValidatedResourceLimits) -> Self {
        let limits = limits.get();
        Self::new(
            limits.max_document_package_bytes,
            limits.max_json_nesting_depth,
        )
        .expect("validated ResourceLimits must satisfy machine-package hard bounds")
    }
}

impl Default for DocumentPackagePreflightLimits {
    fn default() -> Self {
        Self {
            max_bytes: DocumentPackageByteLimit(
                MachineInputLimitBounds::DEFAULT_MAX_DOCUMENT_PACKAGE_BYTES,
            ),
            max_depth: JsonNestingDepthLimit(
                MachineInputLimitBounds::DEFAULT_MAX_JSON_NESTING_DEPTH,
            ),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct JsonPreflightReport<'a> {
    input: &'a str,
    byte_length: u64,
    maximum_depth: u16,
}

impl std::fmt::Debug for JsonPreflightReport<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("JsonPreflightReport")
            .field("byte_length", &self.byte_length)
            .field("maximum_depth", &self.maximum_depth)
            .finish()
    }
}

impl JsonPreflightReport<'_> {
    pub const fn byte_length(self) -> u64 {
        self.byte_length
    }

    pub const fn maximum_depth(self) -> u16 {
        self.maximum_depth
    }

    /// Re-inspects one known token start without retaining a token-offset table.
    /// This lets the typed decoder distinguish a valid fraction/exponent token
    /// from malformed number grammar when mapping a later field-type error.
    pub fn token_metadata_at(self, start_byte: u64) -> Option<JsonTokenMetadata> {
        let start = usize::try_from(start_byte).ok()?;
        let byte = *self.input.as_bytes().get(start)?;
        if !self.input.is_char_boundary(start) {
            return None;
        }
        let mut scanner = Scanner::new(self.input, JsonNestingDepthLimit(1));
        scanner.position = start;
        match byte {
            b'{' => Some(JsonTokenMetadata::incomplete(
                JsonTokenKind::Object,
                start_byte,
            )),
            b'[' => Some(JsonTokenMetadata::incomplete(
                JsonTokenKind::Array,
                start_byte,
            )),
            b'"' => scanner.parse_string(false).ok().map(|(_, token)| token),
            b'-' | b'0'..=b'9' => scanner.parse_number().ok(),
            b't' => scanner.parse_literal(b"true", JsonTokenKind::True).ok(),
            b'f' => scanner.parse_literal(b"false", JsonTokenKind::False).ok(),
            b'n' => scanner.parse_literal(b"null", JsonTokenKind::Null).ok(),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StrictJsonPreflight {
    limits: DocumentPackagePreflightLimits,
}

impl StrictJsonPreflight {
    pub const fn new(limits: DocumentPackagePreflightLimits) -> Self {
        Self { limits }
    }

    pub const fn with_limits(
        max_bytes: u64,
        max_depth: u16,
    ) -> Result<Self, JsonPreflightLimitError> {
        match DocumentPackagePreflightLimits::new(max_bytes, max_depth) {
            Ok(limits) => Ok(Self::new(limits)),
            Err(error) => Err(error),
        }
    }

    pub const fn limits(self) -> DocumentPackagePreflightLimits {
        self.limits
    }

    /// Checks byte admission before UTF-8 scanning or scanner allocation.
    pub fn check<'a>(
        &self,
        input: &'a [u8],
    ) -> Result<JsonPreflightReport<'a>, JsonPreflightError> {
        let actual = u64::try_from(input.len()).unwrap_or(u64::MAX);
        let byte_limit = self.limits.max_bytes().get();
        if actual > byte_limit {
            return Err(envelope_error(
                JsonPreflightErrorKind::PackageBytesExceeded {
                    limit: byte_limit,
                    actual,
                },
                byte_limit,
            ));
        }

        if input.starts_with(&[0xef, 0xbb, 0xbf]) {
            return Err(envelope_error(JsonPreflightErrorKind::Utf8Bom, 0));
        }

        let raw_nul = input.iter().position(|byte| *byte == 0);
        let utf8 = std::str::from_utf8(input);
        if let Err(error) = utf8 {
            let invalid = error.valid_up_to();
            if raw_nul.map_or(true, |nul| invalid < nul) {
                return Err(envelope_error(
                    JsonPreflightErrorKind::InvalidUtf8,
                    offset(invalid),
                ));
            }
        }
        if let Some(raw_nul) = raw_nul {
            return Err(envelope_error(
                JsonPreflightErrorKind::RawNul,
                offset(raw_nul),
            ));
        }
        let input = utf8.expect("UTF-8 error was returned above");
        Scanner::new(input, self.limits.max_depth()).scan()
    }
}

impl Default for StrictJsonPreflight {
    fn default() -> Self {
        Self::new(DocumentPackagePreflightLimits::default())
    }
}

fn envelope_error(kind: JsonPreflightErrorKind, byte_offset: u64) -> JsonPreflightError {
    JsonPreflightError::new(
        kind,
        JsonPreflightLocation::new(byte_offset, JsonPointer::root(), None, None, None, None),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObjectState {
    MemberOrEnd,
    Member,
    Colon,
    Value,
    CommaOrEnd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArrayState {
    ValueOrEnd,
    Value,
    CommaOrEnd,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MemberContext {
    start_byte: usize,
    name: String,
}

#[derive(Debug)]
enum FrameKind {
    Object {
        state: ObjectState,
        members: BTreeSet<String>,
        current_member: Option<MemberContext>,
    },
    Array {
        state: ArrayState,
        index: u64,
    },
}

#[derive(Debug)]
struct Frame {
    start_byte: usize,
    kind: FrameKind,
}

impl Frame {
    const fn container_kind(&self) -> JsonContainerKind {
        match &self.kind {
            FrameKind::Object { .. } => JsonContainerKind::Object,
            FrameKind::Array { .. } => JsonContainerKind::Array,
        }
    }

    const fn token_kind(&self) -> JsonTokenKind {
        match &self.kind {
            FrameKind::Object { .. } => JsonTokenKind::Object,
            FrameKind::Array { .. } => JsonTokenKind::Array,
        }
    }
}

struct Scanner<'a> {
    input: &'a str,
    bytes: &'a [u8],
    position: usize,
    depth_limit: JsonNestingDepthLimit,
    maximum_depth: u16,
    frames: Vec<Frame>,
    last_token: Option<JsonTokenMetadata>,
}

impl<'a> Scanner<'a> {
    fn new(input: &'a str, depth_limit: JsonNestingDepthLimit) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            position: 0,
            depth_limit,
            maximum_depth: 0,
            frames: Vec::new(),
            last_token: None,
        }
    }

    fn scan(mut self) -> Result<JsonPreflightReport<'a>, JsonPreflightError> {
        self.skip_whitespace();
        let root_start = self.position;
        if self.bytes.get(self.position) != Some(&b'{') {
            return Err(self.fail_at(
                JsonPreflightErrorKind::RootMustBeObject,
                self.position,
                None,
            ));
        }
        self.push_container(JsonContainerKind::Object, root_start)?;
        self.position += 1;

        while !self.frames.is_empty() {
            self.skip_whitespace();
            self.step()?;
        }

        self.skip_whitespace();
        if self.position != self.bytes.len() {
            return Err(self.fail_at(
                JsonPreflightErrorKind::TrailingToken,
                self.position,
                self.last_token,
            ));
        }
        Ok(JsonPreflightReport {
            input: self.input,
            byte_length: offset(self.bytes.len()),
            maximum_depth: self.maximum_depth,
        })
    }

    fn step(&mut self) -> Result<(), JsonPreflightError> {
        enum State {
            Object(ObjectState),
            Array(ArrayState),
        }
        let state = match &self.frames.last().expect("scanner has a frame").kind {
            FrameKind::Object { state, .. } => State::Object(*state),
            FrameKind::Array { state, .. } => State::Array(*state),
        };
        match state {
            State::Object(ObjectState::MemberOrEnd) => self.object_member(true),
            State::Object(ObjectState::Member) => self.object_member(false),
            State::Object(ObjectState::Colon) => self.object_colon(),
            State::Object(ObjectState::Value) => self.consume_value(),
            State::Object(ObjectState::CommaOrEnd) => self.object_comma_or_end(),
            State::Array(ArrayState::ValueOrEnd) => self.array_value(true),
            State::Array(ArrayState::Value) => self.array_value(false),
            State::Array(ArrayState::CommaOrEnd) => self.array_comma_or_end(),
        }
    }

    fn object_member(&mut self, allow_end: bool) -> Result<(), JsonPreflightError> {
        match self.bytes.get(self.position).copied() {
            Some(b'}') if allow_end => self.close_container(JsonContainerKind::Object),
            Some(b'}') => Err(self.fail_at(
                JsonPreflightErrorKind::UnexpectedToken,
                self.position,
                self.most_recent_token(),
            )),
            Some(b'"') => {
                let start = self.position;
                let (member, token) = self.parse_string(true)?;
                let member = member.expect("member strings are decoded");
                let duplicate = match &self.frames.last().expect("object frame").kind {
                    FrameKind::Object { members, .. } => members.contains(&member),
                    FrameKind::Array { .. } => unreachable!("object state has object frame"),
                };
                if duplicate {
                    return Err(self.fail_with_member(
                        JsonPreflightErrorKind::DuplicateObjectMember,
                        start,
                        Some(token),
                        start,
                        member,
                    ));
                }
                match &mut self.frames.last_mut().expect("object frame").kind {
                    FrameKind::Object {
                        state,
                        members,
                        current_member,
                    } => {
                        members.insert(member.clone());
                        *current_member = Some(MemberContext {
                            start_byte: start,
                            name: member,
                        });
                        *state = ObjectState::Colon;
                    }
                    FrameKind::Array { .. } => unreachable!("object state has object frame"),
                }
                self.last_token = Some(token);
                Ok(())
            }
            Some(_) => Err(self.fail_at(
                JsonPreflightErrorKind::UnexpectedToken,
                self.position,
                self.most_recent_token(),
            )),
            None => Err(self.fail_at(
                JsonPreflightErrorKind::UnexpectedEnd,
                self.position,
                self.most_recent_token(),
            )),
        }
    }

    fn object_colon(&mut self) -> Result<(), JsonPreflightError> {
        match self.bytes.get(self.position) {
            Some(b':') => {
                self.position += 1;
                match &mut self.frames.last_mut().expect("object frame").kind {
                    FrameKind::Object { state, .. } => *state = ObjectState::Value,
                    FrameKind::Array { .. } => unreachable!("object state has object frame"),
                }
                Ok(())
            }
            Some(_) => Err(self.fail_at(
                JsonPreflightErrorKind::UnexpectedToken,
                self.position,
                self.most_recent_token(),
            )),
            None => Err(self.fail_at(
                JsonPreflightErrorKind::UnexpectedEnd,
                self.position,
                self.most_recent_token(),
            )),
        }
    }

    fn object_comma_or_end(&mut self) -> Result<(), JsonPreflightError> {
        match self.bytes.get(self.position) {
            Some(b',') => {
                self.position += 1;
                match &mut self.frames.last_mut().expect("object frame").kind {
                    FrameKind::Object {
                        state,
                        current_member,
                        ..
                    } => {
                        *state = ObjectState::Member;
                        *current_member = None;
                    }
                    FrameKind::Array { .. } => unreachable!("object state has object frame"),
                }
                Ok(())
            }
            Some(b'}') => self.close_container(JsonContainerKind::Object),
            Some(_) => Err(self.fail_at(
                JsonPreflightErrorKind::UnexpectedToken,
                self.position,
                self.most_recent_token(),
            )),
            None => Err(self.fail_at(
                JsonPreflightErrorKind::UnexpectedEnd,
                self.position,
                self.most_recent_token(),
            )),
        }
    }

    fn array_value(&mut self, allow_end: bool) -> Result<(), JsonPreflightError> {
        match self.bytes.get(self.position) {
            Some(b']') if allow_end => self.close_container(JsonContainerKind::Array),
            Some(b']') => Err(self.fail_at(
                JsonPreflightErrorKind::UnexpectedToken,
                self.position,
                self.most_recent_token(),
            )),
            Some(_) => self.consume_value(),
            None => Err(self.fail_at(
                JsonPreflightErrorKind::UnexpectedEnd,
                self.position,
                self.most_recent_token(),
            )),
        }
    }

    fn array_comma_or_end(&mut self) -> Result<(), JsonPreflightError> {
        match self.bytes.get(self.position) {
            Some(b',') => {
                self.position += 1;
                match &mut self.frames.last_mut().expect("array frame").kind {
                    FrameKind::Array { state, index } => {
                        *state = ArrayState::Value;
                        *index = index.checked_add(1).expect("array index is byte-bounded");
                    }
                    FrameKind::Object { .. } => unreachable!("array state has array frame"),
                }
                Ok(())
            }
            Some(b']') => self.close_container(JsonContainerKind::Array),
            Some(_) => Err(self.fail_at(
                JsonPreflightErrorKind::UnexpectedToken,
                self.position,
                self.most_recent_token(),
            )),
            None => Err(self.fail_at(
                JsonPreflightErrorKind::UnexpectedEnd,
                self.position,
                self.most_recent_token(),
            )),
        }
    }

    fn consume_value(&mut self) -> Result<(), JsonPreflightError> {
        let start = self.position;
        let byte = match self.bytes.get(start).copied() {
            Some(byte) => byte,
            None => {
                return Err(self.fail_at(
                    JsonPreflightErrorKind::UnexpectedEnd,
                    start,
                    self.most_recent_token(),
                ))
            }
        };
        match byte {
            b'{' => self.open_child_container(JsonContainerKind::Object, start),
            b'[' => self.open_child_container(JsonContainerKind::Array, start),
            b'"' => {
                let (_, token) = self.parse_string(false)?;
                self.finish_scalar(token);
                Ok(())
            }
            b'-' | b'0'..=b'9' => {
                let token = self.parse_number()?;
                self.finish_scalar(token);
                Ok(())
            }
            b't' => {
                let token = self.parse_literal(b"true", JsonTokenKind::True)?;
                self.finish_scalar(token);
                Ok(())
            }
            b'f' => {
                let token = self.parse_literal(b"false", JsonTokenKind::False)?;
                self.finish_scalar(token);
                Ok(())
            }
            b'n' => {
                let token = self.parse_literal(b"null", JsonTokenKind::Null)?;
                self.finish_scalar(token);
                Ok(())
            }
            _ => Err(self.fail_at(JsonPreflightErrorKind::UnexpectedToken, start, None)),
        }
    }

    fn open_child_container(
        &mut self,
        kind: JsonContainerKind,
        start: usize,
    ) -> Result<(), JsonPreflightError> {
        self.check_container_depth(kind, start)?;
        self.mark_parent_value_complete();
        self.push_container_unchecked(kind, start);
        self.position += 1;
        Ok(())
    }

    fn finish_scalar(&mut self, token: JsonTokenMetadata) {
        self.mark_parent_value_complete();
        self.last_token = Some(token);
    }

    fn mark_parent_value_complete(&mut self) {
        match &mut self.frames.last_mut().expect("value has a parent").kind {
            FrameKind::Object { state, .. } => {
                debug_assert_eq!(*state, ObjectState::Value);
                *state = ObjectState::CommaOrEnd;
            }
            FrameKind::Array { state, .. } => {
                debug_assert!(matches!(*state, ArrayState::Value | ArrayState::ValueOrEnd));
                *state = ArrayState::CommaOrEnd;
            }
        }
    }

    fn push_container(
        &mut self,
        kind: JsonContainerKind,
        start: usize,
    ) -> Result<(), JsonPreflightError> {
        self.check_container_depth(kind, start)?;
        self.push_container_unchecked(kind, start);
        Ok(())
    }

    fn check_container_depth(
        &self,
        kind: JsonContainerKind,
        start: usize,
    ) -> Result<(), JsonPreflightError> {
        let attempted = self.frames.len() + 1;
        if attempted > usize::from(self.depth_limit.get()) {
            let attempted = u16::try_from(attempted).unwrap_or(u16::MAX);
            return Err(self.fail_at(
                JsonPreflightErrorKind::JsonNestingDepthExceeded {
                    limit: self.depth_limit.get(),
                    attempted,
                },
                start,
                Some(JsonTokenMetadata::incomplete(
                    container_token_kind(kind),
                    offset(start),
                )),
            ));
        }
        Ok(())
    }

    fn push_container_unchecked(&mut self, kind: JsonContainerKind, start: usize) {
        let frame_kind = match kind {
            JsonContainerKind::Object => FrameKind::Object {
                state: ObjectState::MemberOrEnd,
                members: BTreeSet::new(),
                current_member: None,
            },
            JsonContainerKind::Array => FrameKind::Array {
                state: ArrayState::ValueOrEnd,
                index: 0,
            },
        };
        self.frames.push(Frame {
            start_byte: start,
            kind: frame_kind,
        });
        self.maximum_depth = self
            .maximum_depth
            .max(u16::try_from(self.frames.len()).expect("depth was bounded by u16"));
    }

    fn close_container(&mut self, expected: JsonContainerKind) -> Result<(), JsonPreflightError> {
        let frame = self.frames.pop().expect("close has a frame");
        debug_assert_eq!(frame.container_kind(), expected);
        self.position += 1;
        self.last_token = Some(JsonTokenMetadata::complete(
            frame.token_kind(),
            offset(frame.start_byte),
            offset(self.position),
        ));
        Ok(())
    }

    fn parse_string(
        &mut self,
        decode: bool,
    ) -> Result<(Option<String>, JsonTokenMetadata), JsonPreflightError> {
        let start = self.position;
        debug_assert_eq!(self.bytes[start], b'"');
        self.position += 1;
        let mut decoded = decode.then(String::new);
        loop {
            let byte = match self.bytes.get(self.position).copied() {
                Some(byte) => byte,
                None => {
                    return Err(self.fail_at(
                        JsonPreflightErrorKind::UnterminatedString,
                        self.position,
                        Some(JsonTokenMetadata::incomplete(
                            JsonTokenKind::String,
                            offset(start),
                        )),
                    ))
                }
            };
            match byte {
                b'"' => {
                    self.position += 1;
                    return Ok((
                        decoded,
                        JsonTokenMetadata::complete(
                            JsonTokenKind::String,
                            offset(start),
                            offset(self.position),
                        ),
                    ));
                }
                b'\\' => self.parse_escape(start, decoded.as_mut())?,
                0x00..=0x1f => {
                    return Err(self.fail_at(
                        JsonPreflightErrorKind::UnescapedControlCharacter,
                        self.position,
                        Some(JsonTokenMetadata::incomplete(
                            JsonTokenKind::String,
                            offset(start),
                        )),
                    ))
                }
                0x20..=0x7f => {
                    if let Some(decoded) = decoded.as_mut() {
                        decoded.push(char::from(byte));
                    }
                    self.position += 1;
                }
                _ => {
                    let character = self.input[self.position..]
                        .chars()
                        .next()
                        .expect("position is inside valid UTF-8");
                    if let Some(decoded) = decoded.as_mut() {
                        decoded.push(character);
                    }
                    self.position += character.len_utf8();
                }
            }
        }
    }

    fn parse_escape(
        &mut self,
        string_start: usize,
        decoded: Option<&mut String>,
    ) -> Result<(), JsonPreflightError> {
        let escape_start = self.position;
        let escaped = match self.bytes.get(escape_start + 1).copied() {
            Some(value) => value,
            None => {
                return Err(self.fail_at(
                    JsonPreflightErrorKind::InvalidStringEscape,
                    self.bytes.len(),
                    Some(JsonTokenMetadata::incomplete(
                        JsonTokenKind::String,
                        offset(string_start),
                    )),
                ))
            }
        };
        let simple = match escaped {
            b'"' => Some('"'),
            b'\\' => Some('\\'),
            b'/' => Some('/'),
            b'b' => Some('\u{0008}'),
            b'f' => Some('\u{000c}'),
            b'n' => Some('\n'),
            b'r' => Some('\r'),
            b't' => Some('\t'),
            b'u' => None,
            _ => {
                return Err(self.fail_at(
                    JsonPreflightErrorKind::InvalidStringEscape,
                    escape_start + 1,
                    Some(JsonTokenMetadata::incomplete(
                        JsonTokenKind::String,
                        offset(string_start),
                    )),
                ))
            }
        };
        if let Some(character) = simple {
            if let Some(decoded) = decoded {
                decoded.push(character);
            }
            self.position += 2;
            return Ok(());
        }

        let first = self.parse_hex_quad(escape_start + 2, string_start)?;
        let character = if (0xd800..=0xdbff).contains(&first) {
            let second_escape = escape_start + 6;
            if self.bytes.get(second_escape) != Some(&b'\\')
                || self.bytes.get(second_escape + 1) != Some(&b'u')
            {
                return Err(self.fail_at(
                    JsonPreflightErrorKind::InvalidUnicodeSurrogate,
                    second_escape.min(self.bytes.len()),
                    Some(JsonTokenMetadata::incomplete(
                        JsonTokenKind::String,
                        offset(string_start),
                    )),
                ));
            }
            let second = self.parse_hex_quad(second_escape + 2, string_start)?;
            if !(0xdc00..=0xdfff).contains(&second) {
                return Err(self.fail_at(
                    JsonPreflightErrorKind::InvalidUnicodeSurrogate,
                    second_escape,
                    Some(JsonTokenMetadata::incomplete(
                        JsonTokenKind::String,
                        offset(string_start),
                    )),
                ));
            }
            self.position = escape_start + 12;
            let scalar =
                0x1_0000 + ((u32::from(first) - 0xd800) << 10) + (u32::from(second) - 0xdc00);
            char::from_u32(scalar).expect("valid surrogate pair is a scalar")
        } else if (0xdc00..=0xdfff).contains(&first) {
            return Err(self.fail_at(
                JsonPreflightErrorKind::InvalidUnicodeSurrogate,
                escape_start,
                Some(JsonTokenMetadata::incomplete(
                    JsonTokenKind::String,
                    offset(string_start),
                )),
            ));
        } else {
            self.position = escape_start + 6;
            char::from_u32(u32::from(first)).expect("non-surrogate u16 is a scalar")
        };
        if let Some(decoded) = decoded {
            decoded.push(character);
        }
        Ok(())
    }

    fn parse_hex_quad(&self, start: usize, string_start: usize) -> Result<u16, JsonPreflightError> {
        let mut value = 0u16;
        for index in 0..4 {
            let position = start + index;
            let digit = match self.bytes.get(position).copied().and_then(hex_digit) {
                Some(digit) => digit,
                None => {
                    return Err(self.fail_at(
                        JsonPreflightErrorKind::InvalidUnicodeEscape,
                        position.min(self.bytes.len()),
                        Some(JsonTokenMetadata::incomplete(
                            JsonTokenKind::String,
                            offset(string_start),
                        )),
                    ))
                }
            };
            value = (value << 4) | u16::from(digit);
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<JsonTokenMetadata, JsonPreflightError> {
        let start = self.position;
        if self.bytes[self.position] == b'-' {
            self.position += 1;
        }
        match self.bytes.get(self.position).copied() {
            Some(b'0') => {
                self.position += 1;
                if matches!(self.bytes.get(self.position), Some(b'0'..=b'9')) {
                    return Err(self.invalid_number(start, self.position, false, false));
                }
            }
            Some(b'1'..=b'9') => {
                self.position += 1;
                while matches!(self.bytes.get(self.position), Some(b'0'..=b'9')) {
                    self.position += 1;
                }
            }
            _ => return Err(self.invalid_number(start, self.position, false, false)),
        }

        let mut fraction = false;
        if self.bytes.get(self.position) == Some(&b'.') {
            fraction = true;
            self.position += 1;
            if !matches!(self.bytes.get(self.position), Some(b'0'..=b'9')) {
                return Err(self.invalid_number(start, self.position, true, false));
            }
            while matches!(self.bytes.get(self.position), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }

        let mut exponent = false;
        if matches!(self.bytes.get(self.position), Some(b'e' | b'E')) {
            exponent = true;
            self.position += 1;
            if matches!(self.bytes.get(self.position), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.bytes.get(self.position), Some(b'0'..=b'9')) {
                return Err(self.invalid_number(start, self.position, fraction, true));
            }
            while matches!(self.bytes.get(self.position), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }

        let kind = number_kind(fraction, exponent);
        if self
            .bytes
            .get(self.position)
            .is_some_and(|byte| !is_value_delimiter(*byte))
        {
            return Err(self.invalid_number(start, self.position, fraction, exponent));
        }
        Ok(JsonTokenMetadata::complete(
            JsonTokenKind::Number(kind),
            offset(start),
            offset(self.position),
        ))
    }

    fn invalid_number(
        &self,
        start: usize,
        error_at: usize,
        fraction: bool,
        exponent: bool,
    ) -> JsonPreflightError {
        self.fail_at(
            JsonPreflightErrorKind::InvalidNumber,
            error_at.min(self.bytes.len()),
            Some(JsonTokenMetadata::incomplete(
                JsonTokenKind::Number(number_kind(fraction, exponent)),
                offset(start),
            )),
        )
    }

    fn parse_literal(
        &mut self,
        expected: &[u8],
        kind: JsonTokenKind,
    ) -> Result<JsonTokenMetadata, JsonPreflightError> {
        let start = self.position;
        for (index, expected_byte) in expected.iter().enumerate() {
            let position = start + index;
            if self.bytes.get(position) != Some(expected_byte) {
                return Err(self.fail_at(
                    JsonPreflightErrorKind::InvalidLiteral,
                    position.min(self.bytes.len()),
                    Some(JsonTokenMetadata::incomplete(kind, offset(start))),
                ));
            }
        }
        self.position += expected.len();
        if self
            .bytes
            .get(self.position)
            .is_some_and(|byte| !is_value_delimiter(*byte))
        {
            return Err(self.fail_at(
                JsonPreflightErrorKind::InvalidLiteral,
                self.position,
                Some(JsonTokenMetadata::incomplete(kind, offset(start))),
            ));
        }
        Ok(JsonTokenMetadata::complete(
            kind,
            offset(start),
            offset(self.position),
        ))
    }

    fn skip_whitespace(&mut self) {
        while matches!(
            self.bytes.get(self.position),
            Some(b' ' | b'\t' | b'\n' | b'\r')
        ) {
            self.position += 1;
        }
    }

    fn most_recent_token(&self) -> Option<JsonTokenMetadata> {
        let open = self.frames.last().map(|frame| {
            JsonTokenMetadata::incomplete(frame.token_kind(), offset(frame.start_byte))
        });
        match (self.last_token, open) {
            (Some(last), Some(open)) if open.start_byte() > last.start_byte() => Some(open),
            (Some(last), _) => Some(last),
            (None, open) => open,
        }
    }

    fn fail_at(
        &self,
        kind: JsonPreflightErrorKind,
        byte_offset: usize,
        token: Option<JsonTokenMetadata>,
    ) -> JsonPreflightError {
        self.fail_with_optional_member(kind, byte_offset, token, None)
    }

    fn fail_with_member(
        &self,
        kind: JsonPreflightErrorKind,
        byte_offset: usize,
        token: Option<JsonTokenMetadata>,
        member_start: usize,
        member_name: String,
    ) -> JsonPreflightError {
        self.fail_with_optional_member(
            kind,
            byte_offset,
            token,
            Some(MemberContext {
                start_byte: member_start,
                name: member_name,
            }),
        )
    }

    fn fail_with_optional_member(
        &self,
        kind: JsonPreflightErrorKind,
        byte_offset: usize,
        token: Option<JsonTokenMetadata>,
        member_override: Option<MemberContext>,
    ) -> JsonPreflightError {
        let json_pointer = self.error_pointer(member_override.as_ref());
        let container = self.frames.last().map(|frame| {
            JsonContainerContext::new(frame.container_kind(), offset(frame.start_byte))
        });
        let member = member_override.or_else(|| {
            self.frames
                .iter()
                .rev()
                .find_map(|frame| match &frame.kind {
                    FrameKind::Object { current_member, .. } => current_member.clone(),
                    FrameKind::Array { .. } => None,
                })
        });
        let (member_start_byte, member_name) = match member {
            Some(member) => (Some(offset(member.start_byte)), Some(member.name)),
            None => (None, None),
        };
        JsonPreflightError::new(
            kind,
            JsonPreflightLocation::new(
                offset(byte_offset),
                json_pointer,
                container,
                member_start_byte,
                member_name,
                token.or_else(|| self.most_recent_token()),
            ),
        )
    }

    fn error_pointer(&self, member_override: Option<&MemberContext>) -> JsonPointer {
        let mut pointer = JsonPointer::root();
        let parent_count = self.frames.len().saturating_sub(1);
        for frame in self.frames.iter().take(parent_count) {
            append_current_path_segment(&mut pointer, frame);
        }
        if let Some(member) = member_override {
            pointer.push_segment(&member.name);
        } else if let Some(frame) = self.frames.last() {
            append_current_path_segment(&mut pointer, frame);
        }
        pointer
    }
}

fn append_current_path_segment(pointer: &mut JsonPointer, frame: &Frame) {
    match &frame.kind {
        FrameKind::Object {
            current_member: Some(member),
            ..
        } => pointer.push_segment(&member.name),
        FrameKind::Object {
            current_member: None,
            ..
        } => {}
        FrameKind::Array { index, .. } => pointer.push_segment(&index.to_string()),
    }
}

const fn container_token_kind(kind: JsonContainerKind) -> JsonTokenKind {
    match kind {
        JsonContainerKind::Object => JsonTokenKind::Object,
        JsonContainerKind::Array => JsonTokenKind::Array,
    }
}

const fn number_kind(fraction: bool, exponent: bool) -> JsonNumberKind {
    match (fraction, exponent) {
        (false, false) => JsonNumberKind::Integer,
        (true, false) => JsonNumberKind::Fraction,
        (false, true) => JsonNumberKind::Exponent,
        (true, true) => JsonNumberKind::FractionAndExponent,
    }
}

const fn is_value_delimiter(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b',' | b']' | b'}')
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn offset(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JsonPreflightErrorClass;

    fn check(input: &[u8]) -> Result<JsonPreflightReport<'_>, JsonPreflightError> {
        StrictJsonPreflight::default().check(input)
    }

    fn syntax_error(input: &[u8]) -> JsonPreflightError {
        let error = check(input).expect_err("fixture must fail preflight");
        assert_eq!(error.class(), JsonPreflightErrorClass::JsonSyntax);
        error
    }

    fn nested_document(depth: u16) -> Vec<u8> {
        assert!(depth >= 1);
        let mut input = b"{\"value\":".to_vec();
        input.resize(input.len() + usize::from(depth - 1), b'[');
        input.extend_from_slice(b"null");
        input.resize(input.len() + usize::from(depth - 1), b']');
        input.push(b'}');
        input
    }

    #[test]
    fn preflight_runtime_limits_are_closed_over_machine_bounds() {
        assert!(matches!(
            DocumentPackageByteLimit::new(0),
            Err(JsonPreflightLimitError::DocumentPackageBytesOutOfRange { .. })
        ));
        assert_eq!(
            DocumentPackageByteLimit::new(MachineInputLimitBounds::HARD_MAX_DOCUMENT_PACKAGE_BYTES)
                .unwrap()
                .get(),
            MachineInputLimitBounds::HARD_MAX_DOCUMENT_PACKAGE_BYTES
        );
        assert!(matches!(
            DocumentPackageByteLimit::new(
                MachineInputLimitBounds::HARD_MAX_DOCUMENT_PACKAGE_BYTES + 1
            ),
            Err(JsonPreflightLimitError::DocumentPackageBytesOutOfRange { .. })
        ));
        assert!(matches!(
            JsonNestingDepthLimit::new(0),
            Err(JsonPreflightLimitError::JsonNestingDepthOutOfRange { .. })
        ));
        assert_eq!(
            JsonNestingDepthLimit::new(MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH)
                .unwrap()
                .get(),
            MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH
        );
        assert!(matches!(
            JsonNestingDepthLimit::new(MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH + 1),
            Err(JsonPreflightLimitError::JsonNestingDepthOutOfRange { .. })
        ));
    }

    #[test]
    fn effective_resource_limits_project_without_staging_defaults() {
        let limits = typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits {
            max_document_package_bytes: 12_345,
            max_json_nesting_depth: 123,
            ..typaxis_core::ResourceLimits::default()
        })
        .unwrap();
        let projected = DocumentPackagePreflightLimits::from_resource_limits(&limits);
        assert_eq!(projected.max_bytes().get(), 12_345);
        assert_eq!(projected.max_depth().get(), 123);
    }

    #[test]
    fn preflight_byte_limit_is_checked_before_lexical_work() {
        let exact = StrictJsonPreflight::with_limits(2, 1).unwrap();
        assert_eq!(exact.check(b"{}").unwrap().byte_length(), 2);

        let too_small = StrictJsonPreflight::with_limits(1, 1).unwrap();
        let error = too_small.check(&[0xff, 0xff]).unwrap_err();
        assert_eq!(error.class(), JsonPreflightErrorClass::PackageByteLimit);
        assert_eq!(
            error.kind(),
            JsonPreflightErrorKind::PackageBytesExceeded {
                limit: 1,
                actual: 2
            }
        );
        assert_eq!(error.location().byte_offset(), 1);
    }

    #[test]
    fn preflight_enforces_utf8_bom_nul_root_and_single_value() {
        let cases: &[(&[u8], JsonPreflightErrorKind, u64)] = &[
            (
                &[0xef, 0xbb, 0xbf, b'{', b'}'],
                JsonPreflightErrorKind::Utf8Bom,
                0,
            ),
            (&[b'{', 0xff, b'}'], JsonPreflightErrorKind::InvalidUtf8, 1),
            (b"{\"a\":\0}", JsonPreflightErrorKind::RawNul, 5),
            (b"", JsonPreflightErrorKind::RootMustBeObject, 0),
            (b" \n\t", JsonPreflightErrorKind::RootMustBeObject, 3),
            (b"[]", JsonPreflightErrorKind::RootMustBeObject, 0),
            (b"null", JsonPreflightErrorKind::RootMustBeObject, 0),
            (b"{} []", JsonPreflightErrorKind::TrailingToken, 3),
            (b"{}{}", JsonPreflightErrorKind::TrailingToken, 2),
        ];
        for (input, expected_kind, expected_offset) in cases {
            let error = check(input).unwrap_err();
            assert_eq!(error.class(), JsonPreflightErrorClass::PackageEnvelope);
            assert_eq!(error.kind(), *expected_kind);
            assert_eq!(error.location().byte_offset(), *expected_offset);
        }
        assert_eq!(check(b" \n { } \r\t").unwrap().maximum_depth(), 1);
    }

    #[test]
    fn preflight_depth_is_inclusive_and_stops_before_max_plus_one_push() {
        let exact = StrictJsonPreflight::with_limits(1024, 3).unwrap();
        assert_eq!(exact.check(br#"{"a":[{}]}"#).unwrap().maximum_depth(), 3);

        let too_shallow = StrictJsonPreflight::with_limits(1024, 2).unwrap();
        let error = too_shallow.check(br#"{"a":[{}]}"#).unwrap_err();
        assert_eq!(
            error.kind(),
            JsonPreflightErrorKind::JsonNestingDepthExceeded {
                limit: 2,
                attempted: 3
            }
        );
        assert_eq!(
            error.class(),
            JsonPreflightErrorClass::JsonNestingDepthLimit
        );
        assert_eq!(error.location().byte_offset(), 6);
        assert_eq!(error.location().json_pointer().as_str(), "/a/0");
        assert_eq!(
            error.location().token().unwrap().kind(),
            JsonTokenKind::Object
        );
        assert!(!error.location().token().unwrap().is_complete());

        let exact_hard = nested_document(MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH);
        assert_eq!(
            check(&exact_hard).unwrap().maximum_depth(),
            MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH
        );
        let too_deep = nested_document(MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH + 1);
        let error = check(&too_deep).unwrap_err();
        assert!(matches!(
            error.kind(),
            JsonPreflightErrorKind::JsonNestingDepthExceeded {
                limit: MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH,
                attempted: 257
            }
        ));
    }

    #[test]
    fn preflight_accepts_every_json_scalar_grammar_inside_an_object() {
        let input = r#"{
            "array":[0,-0,17,-23,1.25,1e3,1E-3,-1.25e+3],
            "false":false,
            "null":null,
            "object":{},
            "string":"plain\/\"\\\b\f\n\r\t\u96ea雪",
            "true":true
        }"#
        .as_bytes();
        let report = check(input).unwrap();
        assert_eq!(report.byte_length(), input.len() as u64);
        assert_eq!(report.maximum_depth(), 2);
        assert!(check(br#"{"escaped_nul":"\u0000"}"#).is_ok());
    }

    #[test]
    fn preflight_rejects_structural_and_literal_grammar_errors() {
        let cases: &[&[u8]] = &[
            b"{",
            br#"{"a"}"#,
            br#"{"a" 1}"#,
            br#"{"a":}"#,
            br#"{"a":1,}"#,
            br#"{"a":1 "b":2}"#,
            br#"{"a":[,1]}"#,
            br#"{"a":[1,]}"#,
            br#"{"a":[1}"#,
            br#"{"a":tru}"#,
            br#"{"a":truth}"#,
            br#"{"a":nul}"#,
            br#"{"a":falsee}"#,
        ];
        for input in cases {
            syntax_error(input);
        }
    }

    #[test]
    fn preflight_decodes_unicode_keys_for_object_local_duplicates() {
        let escaped = syntax_error(br#"{"a":1,"\u0061":2}"#);
        assert_eq!(
            escaped.kind(),
            JsonPreflightErrorKind::DuplicateObjectMember
        );
        assert_eq!(escaped.location().member_name(), Some("a"));
        assert_eq!(escaped.location().member_start_byte(), Some(7));
        assert_eq!(escaped.location().json_pointer().as_str(), "/a");
        assert_eq!(
            escaped.location().container().unwrap().kind(),
            JsonContainerKind::Object
        );

        let scalar = syntax_error(r#"{"\uD83D\uDE00":1,"😀":2}"#.as_bytes());
        assert_eq!(scalar.kind(), JsonPreflightErrorKind::DuplicateObjectMember);
        assert_eq!(scalar.location().member_name(), Some("😀"));

        let nested = syntax_error(br#"{"outer":[{"x":1,"\u0078":2}]}"#);
        assert_eq!(nested.kind(), JsonPreflightErrorKind::DuplicateObjectMember);
        assert_eq!(nested.location().member_name(), Some("x"));
        assert_eq!(nested.location().json_pointer().as_str(), "/outer/0/x");
        assert!(check(br#"{"left":{"x":1},"right":{"x":2}}"#).is_ok());
        assert!(check("{\"é\":1,\"e\\u0301\":2}".as_bytes()).is_ok());
        assert!(check(br#"{"A":1,"a":2}"#).is_ok());

        let escaped_pointer = syntax_error(br#"{"a/b~c":1,"a\/b~c":2}"#);
        assert_eq!(
            escaped_pointer.location().json_pointer().as_str(),
            "/a~1b~0c"
        );
    }

    #[test]
    fn preflight_validates_unicode_escape_and_surrogate_grammar() {
        assert!(check(br#"{"value":"\uD834\uDD1E"}"#).is_ok());
        let cases: &[(&[u8], JsonPreflightErrorKind)] = &[
            (
                br#"{"value":"\uD800"}"#,
                JsonPreflightErrorKind::InvalidUnicodeSurrogate,
            ),
            (
                br#"{"value":"\uDC00"}"#,
                JsonPreflightErrorKind::InvalidUnicodeSurrogate,
            ),
            (
                br#"{"value":"\uD800\uD800"}"#,
                JsonPreflightErrorKind::InvalidUnicodeSurrogate,
            ),
            (
                br#"{"value":"\uD800x"}"#,
                JsonPreflightErrorKind::InvalidUnicodeSurrogate,
            ),
            (
                br#"{"value":"\u12xz"}"#,
                JsonPreflightErrorKind::InvalidUnicodeEscape,
            ),
            (
                br#"{"value":"\v"}"#,
                JsonPreflightErrorKind::InvalidStringEscape,
            ),
            (
                b"{\"value\":\"line\nfeed\"}",
                JsonPreflightErrorKind::UnescapedControlCharacter,
            ),
            (
                br#"{"value":"unterminated}"#,
                JsonPreflightErrorKind::UnterminatedString,
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(syntax_error(input).kind(), *expected);
        }
    }

    #[test]
    fn preflight_number_metadata_distinguishes_forms_from_grammar_errors() {
        for (input, expected) in [
            ("0", JsonNumberKind::Integer),
            ("-17", JsonNumberKind::Integer),
            ("1.25", JsonNumberKind::Fraction),
            ("1e3", JsonNumberKind::Exponent),
            ("-1.25E+3", JsonNumberKind::FractionAndExponent),
        ] {
            let document = format!(r#"{{"n":{input}}}"#);
            let report = check(document.as_bytes()).unwrap();
            let token = report.token_metadata_at(5).unwrap();
            assert_eq!(token.kind(), JsonTokenKind::Number(expected));
            assert_eq!(token.end_byte(), Some(5 + input.len() as u64));
        }

        for (input, expected) in [
            (br#"{"n":01}"#.as_slice(), JsonNumberKind::Integer),
            (br#"{"n":-}"#.as_slice(), JsonNumberKind::Integer),
            (br#"{"n":1.}"#.as_slice(), JsonNumberKind::Fraction),
            (br#"{"n":1e+}"#.as_slice(), JsonNumberKind::Exponent),
            (
                br#"{"n":1.2e-}"#.as_slice(),
                JsonNumberKind::FractionAndExponent,
            ),
            (br#"{"n":1x}"#.as_slice(), JsonNumberKind::Integer),
        ] {
            let error = syntax_error(input);
            assert_eq!(error.kind(), JsonPreflightErrorKind::InvalidNumber);
            let token = error.location().token().unwrap();
            assert_eq!(token.kind(), JsonTokenKind::Number(expected));
            assert!(!token.is_complete());
        }
        assert_eq!(
            syntax_error(br#"{"n":+1}"#).kind(),
            JsonPreflightErrorKind::UnexpectedToken
        );
        assert_eq!(
            syntax_error(br#"{"n":.1}"#).kind(),
            JsonPreflightErrorKind::UnexpectedToken
        );
    }

    #[test]
    fn preflight_truncation_reports_current_container_member_and_last_token() {
        let input = br#"{"outer":{"value":[1,"#;
        let error = syntax_error(input);
        assert_eq!(error.kind(), JsonPreflightErrorKind::UnexpectedEnd);
        assert_eq!(error.location().byte_offset(), input.len() as u64);
        assert_eq!(error.location().member_name(), Some("value"));
        assert_eq!(error.location().json_pointer().as_str(), "/outer/value/1");
        let container = error.location().container().unwrap();
        assert_eq!(container.kind(), JsonContainerKind::Array);
        assert_eq!(
            error.location().token().unwrap().kind(),
            JsonTokenKind::Number(JsonNumberKind::Integer)
        );
        assert!(error.location().token().unwrap().is_complete());
    }

    #[test]
    fn preflight_deep_malformed_input_is_iterative() {
        let mut input = b"{\"value\":".to_vec();
        input.extend(std::iter::repeat(b'[').take(100_000));
        let error = check(&input).unwrap_err();
        assert_eq!(
            error.kind(),
            JsonPreflightErrorKind::JsonNestingDepthExceeded {
                limit: MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH,
                attempted: 257,
            }
        );
    }

    #[test]
    fn preflight_arbitrary_bytes_never_panics_or_recurses() {
        let preflight = StrictJsonPreflight::with_limits(4096, 32).unwrap();
        let mut state = 0x243f_6a88_85a3_08d3u64;
        for case in 0..8_192usize {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let length = (state as usize ^ case) % 257;
            let mut input = Vec::with_capacity(length);
            for _ in 0..length {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                input.push(state as u8);
            }
            let _ = preflight.check(&input);
        }

        for first in 0u8..=u8::MAX {
            for second in 0u8..=u8::MAX {
                let _ = preflight.check(&[first, second]);
            }
        }
    }
}
