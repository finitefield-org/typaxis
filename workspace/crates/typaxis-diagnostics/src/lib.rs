#![forbid(unsafe_code)]

use typaxis_core::{
    push_jcs_string, FontFaceId, ImageResourceId, JsonPointer, MasterId, NodeId, PortablePath,
    SourceSpan, StyleId, TextSpan, CONTRACT,
};

pub const DIAGNOSTIC_PREFIXES: &[&str] = &["P1", "T2", "S3", "F4", "L5", "G6", "R7", "D8", "I9"];
pub const MAX_MACHINE_DIAGNOSTICS: usize = 256;
const MAX_CANONICAL_DIAGNOSTIC_TEXT_BYTES: usize = 4_096;

/// A public diagnostic code. This is deliberately a different nominal type
/// from [`SchemaConformanceRuleId`], even where their lexical languages happen
/// to overlap.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DiagnosticCode([u8; 5]);

impl DiagnosticCode {
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        let bytes: [u8; 5] = value.as_ref().as_bytes().try_into().ok()?;
        if DIAGNOSTIC_PREFIXES
            .iter()
            .any(|prefix| bytes[..2] == prefix.as_bytes()[..])
            && bytes[2..].iter().all(u8::is_ascii_digit)
        {
            Some(Self(bytes))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.0).expect("diagnostic codes contain only static ASCII")
    }
}

impl core::fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A Schema/cross-artifact conformance rule identifier, not a public
/// diagnostic code.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SchemaConformanceRuleId(String);

impl SchemaConformanceRuleId {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        let mut bytes = value.bytes();
        if !bytes.next().is_some_and(|byte| byte.is_ascii_uppercase())
            || !bytes.all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return None;
        }
        Some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub const P1100: DiagnosticCode = DiagnosticCode(*b"P1100");
pub const P1101: DiagnosticCode = DiagnosticCode(*b"P1101");
pub const P1102: DiagnosticCode = DiagnosticCode(*b"P1102");
pub const P1103: DiagnosticCode = DiagnosticCode(*b"P1103");
pub const P1110: DiagnosticCode = DiagnosticCode(*b"P1110");
pub const P1111: DiagnosticCode = DiagnosticCode(*b"P1111");
pub const P1112: DiagnosticCode = DiagnosticCode(*b"P1112");
pub const T2100: DiagnosticCode = DiagnosticCode(*b"T2100");
pub const T2101: DiagnosticCode = DiagnosticCode(*b"T2101");
pub const L5100: DiagnosticCode = DiagnosticCode(*b"L5100");
pub const L5101: DiagnosticCode = DiagnosticCode(*b"L5101");
pub const L5110: DiagnosticCode = DiagnosticCode(*b"L5110");
pub const G6002: DiagnosticCode = DiagnosticCode(*b"G6002");
pub const G6003: DiagnosticCode = DiagnosticCode(*b"G6003");
pub const G6004: DiagnosticCode = DiagnosticCode(*b"G6004");
pub const R7100: DiagnosticCode = DiagnosticCode(*b"R7100");
pub const I9100: DiagnosticCode = DiagnosticCode(*b"I9100");
pub const I9101: DiagnosticCode = DiagnosticCode(*b"I9101");
pub const I9102: DiagnosticCode = DiagnosticCode(*b"I9102");
pub const I9110: DiagnosticCode = DiagnosticCode(*b"I9110");
pub const I9111: DiagnosticCode = DiagnosticCode(*b"I9111");
pub const I9112: DiagnosticCode = DiagnosticCode(*b"I9112");
pub const I9113: DiagnosticCode = DiagnosticCode(*b"I9113");
pub const I9190: DiagnosticCode = DiagnosticCode(*b"I9190");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Note,
    Warning,
    Error,
    Fatal,
}

impl Severity {
    pub const fn is_advisory(self) -> bool {
        matches!(self, Self::Note | Self::Warning)
    }

    pub const fn is_failure(self) -> bool {
        matches!(self, Self::Error | Self::Fatal)
    }
}

/// A source location that cannot be empty. At least one of source span, text
/// span, or node ID must be supplied to the constructor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceDiagnosticLocation {
    source_span: Option<SourceSpan>,
    text_span: Option<TextSpan>,
    node_id: Option<NodeId>,
}

impl SourceDiagnosticLocation {
    pub const fn new(
        source_span: Option<SourceSpan>,
        text_span: Option<TextSpan>,
        node_id: Option<NodeId>,
    ) -> Option<Self> {
        if source_span.is_none() && text_span.is_none() && node_id.is_none() {
            None
        } else {
            Some(Self {
                source_span,
                text_span,
                node_id,
            })
        }
    }

    pub const fn source_span(&self) -> Option<SourceSpan> {
        self.source_span
    }

    pub const fn text_span(&self) -> Option<TextSpan> {
        self.text_span
    }

    pub const fn node_id(&self) -> Option<NodeId> {
        self.node_id
    }
}

/// Structured portable diagnostic location. Package JSON locations use an
/// RFC 6901 pointer and may retain the decoder's raw byte offset. Source
/// locations are constructor-validated by [`SourceDiagnosticLocation`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticLocation {
    PackageJson {
        uri: PortablePath,
        json_pointer: JsonPointer,
        byte_offset: Option<u64>,
    },
    Source(SourceDiagnosticLocation),
}

impl DiagnosticLocation {
    pub fn package_json(
        uri: PortablePath,
        json_pointer: JsonPointer,
        byte_offset: Option<u64>,
    ) -> Self {
        Self::PackageJson {
            uri,
            json_pointer,
            byte_offset,
        }
    }

    pub const fn source(location: SourceDiagnosticLocation) -> Self {
        Self::Source(location)
    }
}

/// The only categories allowed to omit a primary location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlobalDiagnosticScope {
    Config,
    Io,
    Publication,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanonicalDiagnosticTextError {
    Empty,
    TooLong,
    ControlCharacter,
    AbsolutePath,
    RawOsDetail,
    InputSnippet,
}

/// Text admitted to canonical diagnostics. Location details, host errors, and
/// input excerpts belong in typed/private progress, never in this value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalDiagnosticText(String);

impl CanonicalDiagnosticText {
    pub fn new(value: impl Into<String>) -> Result<Self, CanonicalDiagnosticTextError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CanonicalDiagnosticTextError::Empty);
        }
        if value.len() > MAX_CANONICAL_DIAGNOSTIC_TEXT_BYTES {
            return Err(CanonicalDiagnosticTextError::TooLong);
        }
        if value
            .chars()
            .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
        {
            return Err(CanonicalDiagnosticTextError::ControlCharacter);
        }
        if contains_absolute_path(&value) {
            return Err(CanonicalDiagnosticTextError::AbsolutePath);
        }
        let lowercase = value.to_ascii_lowercase();
        if contains_raw_os_detail(&lowercase) {
            return Err(CanonicalDiagnosticTextError::RawOsDetail);
        }
        if contains_input_snippet(&lowercase) {
            return Err(CanonicalDiagnosticTextError::InputSnippet);
        }
        Ok(Self(value))
    }

    fn omission_count(count: u64) -> Self {
        Self(format!(
            "{count} diagnostic{} omitted by the command-wide limit",
            if count == 1 { "" } else { "s" }
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn contains_absolute_path(value: &str) -> bool {
    value.split_ascii_whitespace().any(|token| {
        let token = token.trim_matches(|character: char| {
            matches!(
                character,
                '"' | '\'' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
            )
        });
        let bytes = token.as_bytes();
        token.starts_with('/')
            || token.starts_with("\\\\")
            || token.starts_with("~/")
            || token.starts_with("file://")
            || (bytes.len() >= 3
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && matches!(bytes[2], b'/' | b'\\'))
    })
}

fn contains_raw_os_detail(lowercase: &str) -> bool {
    [
        "os error",
        "raw os error",
        "raw_os_error",
        "errno",
        "winerror",
        "i/o error:",
        "io error:",
        "kind: notfound",
        "kind: permissiondenied",
        "permission denied",
        "operation not permitted",
        "no such file or directory",
        "access is denied",
        "the system cannot find",
        "error { kind:",
    ]
    .iter()
    .any(|marker| lowercase.contains(marker))
}

fn contains_input_snippet(lowercase: &str) -> bool {
    if lowercase
        .chars()
        .any(|character| matches!(character, '"' | '`' | '{' | '}'))
    {
        return true;
    }
    [
        "input:",
        "source:",
        "package:",
        "token:",
        "value:",
        "input snippet",
        "source snippet",
        "package snippet",
        "input excerpt",
        "source excerpt",
        "package excerpt",
        "input was:",
        "source text:",
        "near token:",
        "offending input",
        "offending source",
        "offending package",
    ]
    .iter()
    .any(|marker| lowercase.contains(marker))
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DiagnosticNoteKind {
    Canonical,
    Omitted(u64),
}

/// A validated canonical note, optionally pointing at a secondary structured
/// location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticNote {
    message: CanonicalDiagnosticText,
    location: Option<DiagnosticLocation>,
    kind: DiagnosticNoteKind,
}

impl DiagnosticNote {
    pub fn new(message: impl Into<String>) -> Result<Self, CanonicalDiagnosticTextError> {
        Ok(Self {
            message: CanonicalDiagnosticText::new(message)?,
            location: None,
            kind: DiagnosticNoteKind::Canonical,
        })
    }

    pub fn located(
        message: impl Into<String>,
        location: DiagnosticLocation,
    ) -> Result<Self, CanonicalDiagnosticTextError> {
        Ok(Self {
            message: CanonicalDiagnosticText::new(message)?,
            location: Some(location),
            kind: DiagnosticNoteKind::Canonical,
        })
    }

    fn omission_count(count: u64) -> Self {
        Self {
            message: CanonicalDiagnosticText::omission_count(count),
            location: None,
            kind: DiagnosticNoteKind::Omitted(count),
        }
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    pub const fn location(&self) -> Option<&DiagnosticLocation> {
        self.location.as_ref()
    }

    pub const fn omitted_count(&self) -> Option<u64> {
        match self.kind {
            DiagnosticNoteKind::Canonical => None,
            DiagnosticNoteKind::Omitted(count) => Some(count),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceErrorSubject {
    FontFace(FontFaceId),
    Image(ImageResourceId),
    Uri(PortablePath),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StylePropertyName(String);

impl StylePropertyName {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        let mut bytes = value.bytes();
        if value.len() > 64
            || !bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
            || !bytes.all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
            })
        {
            return None;
        }
        Some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleErrorSubject {
    node_id: NodeId,
    style_id: Option<StyleId>,
    property: Option<StylePropertyName>,
}

impl StyleErrorSubject {
    pub const fn new(
        node_id: NodeId,
        style_id: Option<StyleId>,
        property: Option<StylePropertyName>,
    ) -> Self {
        Self {
            node_id,
            style_id,
            property,
        }
    }

    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub const fn style_id(&self) -> Option<&StyleId> {
        self.style_id.as_ref()
    }

    pub const fn property(&self) -> Option<&StylePropertyName> {
        self.property.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayoutErrorSubject {
    node_id: NodeId,
    text_span: Option<TextSpan>,
}

impl LayoutErrorSubject {
    pub const fn new(node_id: NodeId, text_span: Option<TextSpan>) -> Self {
        Self { node_id, text_span }
    }

    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub const fn text_span(&self) -> Option<TextSpan> {
        self.text_span
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MasterErrorSubject {
    master_id: MasterId,
    rule_index: Option<u32>,
}

impl MasterErrorSubject {
    pub const fn new(master_id: MasterId, rule_index: Option<u32>) -> Self {
        Self {
            master_id,
            rule_index,
        }
    }

    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }

    pub const fn rule_index(&self) -> Option<u32> {
        self.rule_index
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticSubject {
    Node(NodeId),
    Resource(ResourceErrorSubject),
    Style(StyleErrorSubject),
    Layout(LayoutErrorSubject),
    Master(MasterErrorSubject),
}

/// Stable public error classes. Code assignment and logical subjects are
/// exhaustive typed matches; no mapper needs to parse a `Debug` string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicMachineError {
    PackageEnvelope,
    PackageJsonGrammar,
    PackageMember,
    PackageContract,
    SourceProfile,
    SourcePath,
    SourceIdentity,
    UnsupportedContent(LayoutErrorSubject),
    UnsupportedStyle(StyleErrorSubject),
    UnsupportedMaster(MasterErrorSubject),
    UnsupportedResource(ResourceErrorSubject),
    PackageByteLimit,
    JsonNestingDepthLimit,
    HostReadCandidateLimit,
    CompiledHostUnavailable,
    PackageOpen,
    CompanionSourceOpen,
    StableReadMutation,
    CapabilityDomainMismatch,
}

impl PublicMachineError {
    pub const fn code(&self) -> DiagnosticCode {
        match self {
            Self::PackageEnvelope => P1100,
            Self::PackageJsonGrammar => P1101,
            Self::PackageMember => P1102,
            Self::PackageContract => P1103,
            Self::SourceProfile => P1110,
            Self::SourcePath => P1111,
            Self::SourceIdentity => P1112,
            Self::UnsupportedContent(_) => L5100,
            Self::UnsupportedStyle(_) | Self::UnsupportedMaster(_) => L5101,
            Self::UnsupportedResource(_) => R7100,
            Self::PackageByteLimit => I9100,
            Self::JsonNestingDepthLimit => I9101,
            Self::HostReadCandidateLimit => I9102,
            Self::CompiledHostUnavailable => I9110,
            Self::PackageOpen => I9111,
            Self::CompanionSourceOpen => I9112,
            Self::StableReadMutation => I9113,
            Self::CapabilityDomainMismatch => I9190,
        }
    }

    pub fn subject(&self) -> Option<DiagnosticSubject> {
        match self {
            Self::UnsupportedContent(subject) => Some(DiagnosticSubject::Layout(*subject)),
            Self::UnsupportedStyle(subject) => Some(DiagnosticSubject::Style(subject.clone())),
            Self::UnsupportedMaster(subject) => Some(DiagnosticSubject::Master(subject.clone())),
            Self::UnsupportedResource(subject) => {
                Some(DiagnosticSubject::Resource(subject.clone()))
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    code: DiagnosticCode,
    severity: Severity,
    message: CanonicalDiagnosticText,
    location: Option<DiagnosticLocation>,
    global_scope: Option<GlobalDiagnosticScope>,
    subject: Option<DiagnosticSubject>,
    notes: Vec<DiagnosticNote>,
}

impl Diagnostic {
    pub fn located(
        code: DiagnosticCode,
        severity: Severity,
        message: impl Into<String>,
        location: DiagnosticLocation,
    ) -> Result<Self, CanonicalDiagnosticTextError> {
        Ok(DiagnosticBuilder::located(code, severity, message, location)?.build())
    }

    pub fn global(
        code: DiagnosticCode,
        severity: Severity,
        message: impl Into<String>,
        scope: GlobalDiagnosticScope,
    ) -> Result<Self, CanonicalDiagnosticTextError> {
        Ok(DiagnosticBuilder::global(code, severity, message, scope)?.build())
    }

    pub const fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    pub const fn severity(&self) -> Severity {
        self.severity
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    pub const fn location(&self) -> Option<&DiagnosticLocation> {
        self.location.as_ref()
    }

    pub const fn global_scope(&self) -> Option<GlobalDiagnosticScope> {
        self.global_scope
    }

    pub const fn subject(&self) -> Option<&DiagnosticSubject> {
        self.subject.as_ref()
    }

    pub fn notes(&self) -> &[DiagnosticNote] {
        &self.notes
    }

    pub const fn source_span(&self) -> Option<SourceSpan> {
        match &self.location {
            Some(DiagnosticLocation::Source(location)) => location.source_span(),
            _ => None,
        }
    }

    pub const fn text_span(&self) -> Option<TextSpan> {
        match &self.location {
            Some(DiagnosticLocation::Source(location)) => location.text_span(),
            _ => None,
        }
    }

    pub const fn node_id(&self) -> Option<NodeId> {
        match &self.location {
            Some(DiagnosticLocation::Source(location)) => location.node_id(),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticBuilder {
    diagnostic: Diagnostic,
}

impl DiagnosticBuilder {
    pub fn located(
        code: DiagnosticCode,
        severity: Severity,
        message: impl Into<String>,
        location: DiagnosticLocation,
    ) -> Result<Self, CanonicalDiagnosticTextError> {
        Ok(Self {
            diagnostic: Diagnostic {
                code,
                severity,
                message: CanonicalDiagnosticText::new(message)?,
                location: Some(location),
                global_scope: None,
                subject: None,
                notes: Vec::new(),
            },
        })
    }

    pub fn global(
        code: DiagnosticCode,
        severity: Severity,
        message: impl Into<String>,
        scope: GlobalDiagnosticScope,
    ) -> Result<Self, CanonicalDiagnosticTextError> {
        Ok(Self {
            diagnostic: Diagnostic {
                code,
                severity,
                message: CanonicalDiagnosticText::new(message)?,
                location: None,
                global_scope: Some(scope),
                subject: None,
                notes: Vec::new(),
            },
        })
    }

    pub fn subject(mut self, subject: DiagnosticSubject) -> Self {
        self.diagnostic.subject = Some(subject);
        self
    }

    pub fn note(
        mut self,
        message: impl Into<String>,
    ) -> Result<Self, CanonicalDiagnosticTextError> {
        self.diagnostic.notes.push(DiagnosticNote::new(message)?);
        Ok(self)
    }

    pub fn located_note(
        mut self,
        message: impl Into<String>,
        location: DiagnosticLocation,
    ) -> Result<Self, CanonicalDiagnosticTextError> {
        self.diagnostic
            .notes
            .push(DiagnosticNote::located(message, location)?);
        Ok(self)
    }

    pub fn build(self) -> Diagnostic {
        self.diagnostic
    }
}

/// A note or warning proven safe to accompany a successful phase value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdvisoryDiagnostic(Diagnostic);

impl AdvisoryDiagnostic {
    #[allow(clippy::result_large_err)] // Return the owned diagnostic unchanged on severity mismatch.
    pub fn new(diagnostic: Diagnostic) -> Result<Self, Diagnostic> {
        if diagnostic.severity.is_advisory() {
            Ok(Self(diagnostic))
        } else {
            Err(diagnostic)
        }
    }

    pub fn as_diagnostic(&self) -> &Diagnostic {
        &self.0
    }

    pub fn into_inner(self) -> Diagnostic {
        self.0
    }
}

/// Non-empty diagnostics for a failed phase. At least one member is an error
/// or fatal, and a fatal diagnostic is necessarily the final diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseFailure(Vec<Diagnostic>);

impl ParseFailure {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Result<Self, Vec<Diagnostic>> {
        let has_failure = diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_failure());
        let fatal_is_terminal = diagnostics
            .iter()
            .position(|diagnostic| diagnostic.severity == Severity::Fatal)
            .map_or(true, |index| index + 1 == diagnostics.len());
        if !diagnostics.is_empty() && has_failure && fatal_is_terminal {
            Ok(Self(diagnostics))
        } else {
            Err(diagnostics)
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.0
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use = "a fatal diagnostic requires the caller to abort immediately"]
pub enum DiagnosticFlow {
    Continue,
    Abort(ParseFailure),
    AlreadyAborted,
}

/// Diagnostic owner for legacy single-phase parsers. Machine commands use
/// [`MachineDiagnosticBudget`] so all phases share one aggregate cap.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhaseDiagnostics {
    diagnostics: Vec<Diagnostic>,
    aborted: bool,
}

impl PhaseDiagnostics {
    pub const fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            aborted: false,
        }
    }

    pub fn emit(&mut self, diagnostic: Diagnostic) -> DiagnosticFlow {
        if self.aborted {
            return DiagnosticFlow::AlreadyAborted;
        }
        let fatal = diagnostic.severity == Severity::Fatal;
        self.diagnostics.push(diagnostic);
        if fatal {
            self.aborted = true;
            DiagnosticFlow::Abort(
                ParseFailure::new(self.diagnostics.clone())
                    .expect("a terminal fatal diagnostic forms a parse failure"),
            )
        } else {
            DiagnosticFlow::Continue
        }
    }

    pub fn finish_boundary(self) -> Result<Vec<AdvisoryDiagnostic>, ParseFailure> {
        if self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_failure())
        {
            return Err(ParseFailure::new(self.diagnostics)
                .expect("a phase containing an error forms a parse failure"));
        }
        Ok(self
            .diagnostics
            .into_iter()
            .map(|diagnostic| {
                AdvisoryDiagnostic::new(diagnostic)
                    .expect("a successful phase contains only advisory diagnostics")
            })
            .collect())
    }
}

/// Fixed command phase order. A budget may skip a phase, but it can never
/// reopen an already lent phase or move backwards.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum MachineDiagnosticPhase {
    Config,
    Host,
    Package,
    Decode,
    Source,
    Syntax,
    Capability,
    Resource,
    Style,
    Layout,
    Pdf,
    Publication,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineDiagnosticBudgetError {
    PhaseAlreadyLent,
    Terminal,
    GlobalScopePhaseMismatch,
    ExpectedErrorDiagnostic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use]
pub enum MachineDiagnosticEmit {
    Retained,
    Omitted,
    RetainedAfterAdvisoryEviction,
    Terminal,
}

/// The sole owner of a command's fixed 256-record diagnostic materialization
/// budget. It is intentionally neither `Clone` nor `Default` and exposes no
/// reset operation.
#[derive(Debug)]
pub struct MachineDiagnosticBudget {
    diagnostics: Vec<Diagnostic>,
    omitted: u64,
    next_phase: u8,
    terminal: bool,
}

impl MachineDiagnosticBudget {
    #[allow(clippy::new_without_default)] // Command ownership must remain explicit at orchestration.
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            omitted: 0,
            next_phase: 0,
            terminal: false,
        }
    }

    pub fn lend(
        &mut self,
        phase: MachineDiagnosticPhase,
    ) -> Result<MachineDiagnosticLender<'_>, MachineDiagnosticBudgetError> {
        if self.terminal {
            return Err(MachineDiagnosticBudgetError::Terminal);
        }
        let ordinal = phase as u8;
        if ordinal < self.next_phase {
            return Err(MachineDiagnosticBudgetError::PhaseAlreadyLent);
        }
        self.next_phase = ordinal + 1;
        Ok(MachineDiagnosticLender {
            phase,
            budget: self,
        })
    }

    pub fn finish(mut self) -> MachineDiagnostics {
        self.refresh_omission_note();
        MachineDiagnostics {
            diagnostics: self.diagnostics,
            omitted: self.omitted,
        }
    }

    fn record(&mut self, diagnostic: Diagnostic) -> MachineDiagnosticEmit {
        let fatal = diagnostic.severity == Severity::Fatal;
        let emit = if self.diagnostics.len() < MAX_MACHINE_DIAGNOSTICS {
            self.diagnostics.push(diagnostic);
            MachineDiagnosticEmit::Retained
        } else if diagnostic.severity.is_advisory() {
            self.omitted = self.omitted.saturating_add(1);
            MachineDiagnosticEmit::Omitted
        } else if let Some(index) = self
            .diagnostics
            .iter()
            .rposition(|existing| existing.severity.is_advisory())
        {
            self.diagnostics.remove(index);
            self.diagnostics.push(diagnostic);
            self.omitted = self.omitted.saturating_add(1);
            MachineDiagnosticEmit::RetainedAfterAdvisoryEviction
        } else if fatal {
            self.diagnostics.pop();
            self.diagnostics.push(diagnostic);
            self.omitted = self.omitted.saturating_add(1);
            MachineDiagnosticEmit::Terminal
        } else {
            self.omitted = self.omitted.saturating_add(1);
            MachineDiagnosticEmit::Omitted
        };

        if fatal {
            self.terminal = true;
        }
        self.refresh_omission_note();
        if fatal {
            MachineDiagnosticEmit::Terminal
        } else {
            emit
        }
    }

    fn omit_unmaterialized(&mut self) -> MachineDiagnosticEmit {
        self.omitted = self.omitted.saturating_add(1);
        self.refresh_omission_note();
        MachineDiagnosticEmit::Omitted
    }

    fn refresh_omission_note(&mut self) {
        for diagnostic in &mut self.diagnostics {
            diagnostic
                .notes
                .retain(|note| !matches!(note.kind, DiagnosticNoteKind::Omitted(_)));
        }
        if self.omitted != 0 {
            if let Some(last) = self.diagnostics.last_mut() {
                last.notes
                    .push(DiagnosticNote::omission_count(self.omitted));
            }
        }
    }
}

pub struct MachineDiagnosticLender<'budget> {
    phase: MachineDiagnosticPhase,
    budget: &'budget mut MachineDiagnosticBudget,
}

impl MachineDiagnosticLender<'_> {
    pub const fn phase(&self) -> MachineDiagnosticPhase {
        self.phase
    }

    pub fn emit(
        &mut self,
        diagnostic: Diagnostic,
    ) -> Result<MachineDiagnosticEmit, MachineDiagnosticBudgetError> {
        if self.budget.terminal {
            return Err(MachineDiagnosticBudgetError::Terminal);
        }
        if let Some(scope) = diagnostic.global_scope {
            let allowed = match scope {
                GlobalDiagnosticScope::Config => self.phase == MachineDiagnosticPhase::Config,
                GlobalDiagnosticScope::Publication => {
                    self.phase == MachineDiagnosticPhase::Publication
                }
                GlobalDiagnosticScope::Io => matches!(
                    self.phase,
                    MachineDiagnosticPhase::Host
                        | MachineDiagnosticPhase::Package
                        | MachineDiagnosticPhase::Decode
                        | MachineDiagnosticPhase::Source
                        | MachineDiagnosticPhase::Resource
                        | MachineDiagnosticPhase::Pdf
                ),
            };
            if !allowed {
                return Err(MachineDiagnosticBudgetError::GlobalScopePhaseMismatch);
            }
        }
        Ok(self.budget.record(diagnostic))
    }

    /// Materialize an error only when it can be retained by the fixed command
    /// budget. Once the budget contains only 256 failures, `build` is not
    /// called; the omitted count and final omission note are updated directly.
    /// This lets bounded collectors finish scanning without allocating
    /// locations/messages for records that cannot become observable.
    pub fn emit_error_with(
        &mut self,
        build: impl FnOnce() -> Diagnostic,
    ) -> Result<MachineDiagnosticEmit, MachineDiagnosticBudgetError> {
        if self.budget.terminal {
            return Err(MachineDiagnosticBudgetError::Terminal);
        }
        let can_retain = self.budget.diagnostics.len() < MAX_MACHINE_DIAGNOSTICS
            || self
                .budget
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity.is_advisory());
        if !can_retain {
            return Ok(self.budget.omit_unmaterialized());
        }
        let diagnostic = build();
        if diagnostic.severity != Severity::Error {
            return Err(MachineDiagnosticBudgetError::ExpectedErrorDiagnostic);
        }
        self.emit(diagnostic)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineDiagnostics {
    diagnostics: Vec<Diagnostic>,
    omitted: u64,
}

impl MachineDiagnostics {
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub const fn omitted_count(&self) -> u64 {
        self.omitted
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

/// Encode the contract-1.1 diagnostics artifact as canonical JSON.
pub fn encode_diagnostics_canonical(diagnostics: &[Diagnostic]) -> String {
    let mut output = String::from("{\"contract\":");
    push_jcs_string(&mut output, CONTRACT);
    output.push_str(",\"diagnostics\":[");
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_diagnostic_jcs(&mut output, diagnostic);
    }
    output.push_str("]}");
    output
}

fn push_diagnostic_jcs(output: &mut String, diagnostic: &Diagnostic) {
    output.push_str("{\"code\":");
    push_jcs_string(output, diagnostic.code.as_str());
    output.push_str(",\"location\":");
    push_optional_location_jcs(output, diagnostic.location.as_ref());
    output.push_str(",\"message\":");
    push_jcs_string(output, diagnostic.message.as_str());
    output.push_str(",\"notes\":[");
    for (index, note) in diagnostic.notes.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"location\":");
        push_optional_location_jcs(output, note.location.as_ref());
        output.push_str(",\"message\":");
        push_jcs_string(output, note.message.as_str());
        output.push('}');
    }
    output.push_str("],\"severity\":");
    push_jcs_string(
        output,
        match diagnostic.severity {
            Severity::Note => "note",
            Severity::Warning => "warning",
            Severity::Error => "error",
            Severity::Fatal => "fatal",
        },
    );
    output.push('}');
}

fn push_optional_location_jcs(output: &mut String, location: Option<&DiagnosticLocation>) {
    match location {
        None => output.push_str("null"),
        Some(DiagnosticLocation::PackageJson {
            uri,
            json_pointer,
            byte_offset,
        }) => {
            output.push_str("{\"byte_offset\":");
            match byte_offset {
                Some(offset) => output.push_str(&offset.to_string()),
                None => output.push_str("null"),
            }
            output.push_str(",\"json_pointer\":");
            push_jcs_string(output, json_pointer.as_str());
            output.push_str(",\"kind\":\"package_json\",\"uri\":");
            push_jcs_string(output, uri.as_str());
            output.push('}');
        }
        Some(DiagnosticLocation::Source(location)) => {
            output.push_str("{\"kind\":\"source\",\"node_id\":");
            match location.node_id {
                Some(node_id) => output.push_str(&node_id.get().to_string()),
                None => output.push_str("null"),
            }
            output.push_str(",\"source_span\":");
            push_optional_source_span_jcs(output, location.source_span);
            output.push_str(",\"text_span\":");
            push_optional_text_span_jcs(output, location.text_span);
            output.push('}');
        }
    }
}

fn push_optional_source_span_jcs(output: &mut String, span: Option<SourceSpan>) {
    match span {
        None => output.push_str("null"),
        Some(span) => {
            output.push_str("{\"end_byte\":");
            output.push_str(&span.end_byte().get().to_string());
            output.push_str(",\"source_id\":");
            output.push_str(&span.source_id().get().to_string());
            output.push_str(",\"start_byte\":");
            output.push_str(&span.start_byte().get().to_string());
            output.push('}');
        }
    }
}

fn push_optional_text_span_jcs(output: &mut String, span: Option<TextSpan>) {
    match span {
        None => output.push_str("null"),
        Some(span) => {
            output.push_str("{\"end_byte\":");
            output.push_str(&span.end_byte().get().to_string());
            output.push_str(",\"start_byte\":");
            output.push_str(&span.start_byte().get().to_string());
            output.push_str(",\"text_id\":");
            output.push_str(&span.text_id().get().to_string());
            output.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{SourceId, Utf8ByteOffset};

    fn source_location() -> DiagnosticLocation {
        DiagnosticLocation::source(
            SourceDiagnosticLocation::new(None, None, Some(NodeId::new(7))).unwrap(),
        )
    }

    fn diagnostic(severity: Severity) -> Diagnostic {
        Diagnostic::located(
            P1102,
            severity,
            "package member is invalid",
            source_location(),
        )
        .unwrap()
    }

    #[test]
    fn diagnostic_code_uses_wire_pattern() {
        for prefix in DIAGNOSTIC_PREFIXES {
            assert!(DiagnosticCode::new(format!("{prefix}000")).is_some());
            assert!(DiagnosticCode::new(format!("{prefix}999")).is_some());
        }
        for invalid in [
            "P2000", "T1000", "S4000", "F3000", "L6000", "G5000", "R8000", "D7000", "I1000",
            "X1000", "P100", "P10000", "P1A00",
        ] {
            assert!(
                DiagnosticCode::new(invalid).is_none(),
                "{invalid} must be rejected"
            );
        }
    }

    #[test]
    fn public_error_mapping_is_typed_and_exhaustive() {
        let cases = [
            (PublicMachineError::PackageEnvelope, P1100),
            (PublicMachineError::PackageJsonGrammar, P1101),
            (PublicMachineError::PackageMember, P1102),
            (PublicMachineError::PackageContract, P1103),
            (PublicMachineError::SourceProfile, P1110),
            (PublicMachineError::SourcePath, P1111),
            (PublicMachineError::SourceIdentity, P1112),
            (PublicMachineError::PackageByteLimit, I9100),
            (PublicMachineError::JsonNestingDepthLimit, I9101),
            (PublicMachineError::HostReadCandidateLimit, I9102),
            (PublicMachineError::CompiledHostUnavailable, I9110),
            (PublicMachineError::PackageOpen, I9111),
            (PublicMachineError::CompanionSourceOpen, I9112),
            (PublicMachineError::StableReadMutation, I9113),
            (PublicMachineError::CapabilityDomainMismatch, I9190),
        ];
        for (error, expected) in cases {
            assert_eq!(error.code(), expected);
            assert_eq!(error.subject(), None);
        }

        let layout = LayoutErrorSubject::new(NodeId::new(2), None);
        let content = PublicMachineError::UnsupportedContent(layout);
        assert_eq!(content.code(), L5100);
        assert_eq!(content.subject(), Some(DiagnosticSubject::Layout(layout)));

        let style = StyleErrorSubject::new(
            NodeId::new(2),
            Some(StyleId::new("body").unwrap()),
            Some(StylePropertyName::new("font_size").unwrap()),
        );
        let style_error = PublicMachineError::UnsupportedStyle(style.clone());
        assert_eq!(style_error.code(), L5101);
        assert_eq!(style_error.subject(), Some(DiagnosticSubject::Style(style)));

        let master = MasterErrorSubject::new(MasterId::new("default").unwrap(), Some(1));
        let master_error = PublicMachineError::UnsupportedMaster(master.clone());
        assert_eq!(master_error.code(), L5101);
        assert_eq!(
            master_error.subject(),
            Some(DiagnosticSubject::Master(master))
        );

        let resource = ResourceErrorSubject::Image(ImageResourceId::new(3));
        let error = PublicMachineError::UnsupportedResource(resource.clone());
        assert_eq!(error.code(), R7100);
        assert_eq!(error.subject(), Some(DiagnosticSubject::Resource(resource)));
    }

    #[test]
    fn public_error_mapping_and_schema_rule_ids_are_separate_namespaces() {
        let public_code = PublicMachineError::PackageMember.code();
        let rule_id = SchemaConformanceRuleId::new("DOCUMENT_MEMBER_TYPE").unwrap();
        assert_eq!(public_code.as_str(), "P1102");
        assert_eq!(rule_id.as_str(), "DOCUMENT_MEMBER_TYPE");
        assert!(DiagnosticCode::new(rule_id.as_str()).is_none());
    }

    #[test]
    fn source_location_requires_at_least_one_typed_coordinate() {
        assert_eq!(SourceDiagnosticLocation::new(None, None, None), None);
        let span = SourceSpan::new(
            SourceId::new(2),
            Utf8ByteOffset::new(4),
            Utf8ByteOffset::new(8),
        )
        .unwrap();
        let location = SourceDiagnosticLocation::new(Some(span), None, None).unwrap();
        assert_eq!(location.source_span(), Some(span));
    }

    #[test]
    fn package_json_location_is_portable_and_pointer_aware() {
        let location = DiagnosticLocation::package_json(
            PortablePath::new("document-package.json").unwrap(),
            JsonPointer::from_segments(["document", "blocks", "3"]),
            Some(1_942),
        );
        assert!(matches!(
            location,
            DiagnosticLocation::PackageJson {
                byte_offset: Some(1_942),
                ..
            }
        ));
        assert!(PortablePath::new("/checkout/document-package.json").is_err());
    }

    #[test]
    fn diagnostics_encoder_uses_the_current_tagged_location_union() {
        let package_location = DiagnosticLocation::package_json(
            PortablePath::new("document-package.json").unwrap(),
            JsonPointer::from_segments(["document", "blocks", "3"]),
            Some(1942),
        );
        let package = DiagnosticBuilder::located(
            P1102,
            Severity::Error,
            "package member is invalid",
            package_location.clone(),
        )
        .unwrap()
        .located_note("related package member", package_location)
        .unwrap()
        .build();
        let global = Diagnostic::global(
            I9111,
            Severity::Fatal,
            "package could not be opened",
            GlobalDiagnosticScope::Io,
        )
        .unwrap();

        assert_eq!(
            encode_diagnostics_canonical(&[package, global]),
            concat!(
                "{\"contract\":\"typaxis.contract/1.3\",\"diagnostics\":[",
                "{\"code\":\"P1102\",\"location\":{\"byte_offset\":1942,",
                "\"json_pointer\":\"/document/blocks/3\",\"kind\":\"package_json\",",
                "\"uri\":\"document-package.json\"},\"message\":\"package member is invalid\",",
                "\"notes\":[{\"location\":{\"byte_offset\":1942,",
                "\"json_pointer\":\"/document/blocks/3\",\"kind\":\"package_json\",",
                "\"uri\":\"document-package.json\"},\"message\":\"related package member\"}],",
                "\"severity\":\"error\"},",
                "{\"code\":\"I9111\",\"location\":null,",
                "\"message\":\"package could not be opened\",\"notes\":[],",
                "\"severity\":\"fatal\"}]}"
            )
        );
    }

    #[test]
    fn builder_rejects_noncanonical_message_and_note_content() {
        for unsafe_message in [
            "failed at /home/alice/project/input.json",
            "failed at C:\\Users\\alice\\input.json",
            "read failed: OS error 13: Permission denied",
            "input snippet: { secret: true }",
        ] {
            assert!(DiagnosticBuilder::located(
                P1102,
                Severity::Error,
                unsafe_message,
                source_location(),
            )
            .is_err());
        }

        let builder = DiagnosticBuilder::located(
            P1102,
            Severity::Error,
            "package member is invalid",
            source_location(),
        )
        .unwrap();
        assert!(builder
            .clone()
            .note("source excerpt: private package bytes")
            .is_err());
        assert!(builder.note("consult the package member location").is_ok());
    }

    #[test]
    fn advisory_wrapper_accepts_only_note_or_warning() {
        assert!(AdvisoryDiagnostic::new(diagnostic(Severity::Warning)).is_ok());
        assert!(AdvisoryDiagnostic::new(diagnostic(Severity::Error)).is_err());
        assert!(AdvisoryDiagnostic::new(diagnostic(Severity::Fatal)).is_err());
    }

    #[test]
    fn parse_failure_requires_failure_and_terminal_fatal() {
        let warning = diagnostic(Severity::Warning);
        assert!(ParseFailure::new(vec![]).is_err());
        assert!(ParseFailure::new(vec![warning.clone()]).is_err());
        assert!(ParseFailure::new(vec![warning, diagnostic(Severity::Error)]).is_ok());
        assert!(ParseFailure::new(vec![
            diagnostic(Severity::Fatal),
            diagnostic(Severity::Note),
        ])
        .is_err());
    }

    #[test]
    fn phase_owner_aborts_immediately_on_fatal() {
        let fatal = diagnostic(Severity::Fatal);
        let after = diagnostic(Severity::Note);
        let mut phase = PhaseDiagnostics::new();
        assert!(matches!(phase.emit(fatal), DiagnosticFlow::Abort(_)));
        assert_eq!(phase.emit(after), DiagnosticFlow::AlreadyAborted);
    }

    #[test]
    fn phase_boundary_separates_success_from_failure() {
        let mut successful = PhaseDiagnostics::new();
        assert_eq!(
            successful.emit(diagnostic(Severity::Warning)),
            DiagnosticFlow::Continue
        );
        assert_eq!(successful.finish_boundary().unwrap().len(), 1);

        let mut failed = PhaseDiagnostics::new();
        assert_eq!(
            failed.emit(diagnostic(Severity::Error)),
            DiagnosticFlow::Continue
        );
        assert!(failed.finish_boundary().is_err());
    }

    #[test]
    fn command_budget_retains_at_most_fixed_limit() {
        let mut budget = MachineDiagnosticBudget::new();
        {
            let mut syntax = budget.lend(MachineDiagnosticPhase::Syntax).unwrap();
            for _ in 0..MAX_MACHINE_DIAGNOSTICS + 20 {
                let _ = syntax.emit(diagnostic(Severity::Warning)).unwrap();
            }
        }
        let diagnostics = budget.finish();
        assert_eq!(diagnostics.diagnostics().len(), MAX_MACHINE_DIAGNOSTICS);
        assert_eq!(diagnostics.omitted_count(), 20);
        assert_eq!(
            diagnostics
                .diagnostics()
                .last()
                .unwrap()
                .notes()
                .last()
                .unwrap()
                .omitted_count(),
            Some(20)
        );
    }

    #[test]
    fn full_budget_evicts_tail_advisory_for_primary_failure() {
        let mut budget = MachineDiagnosticBudget::new();
        {
            let mut syntax = budget.lend(MachineDiagnosticPhase::Syntax).unwrap();
            for _ in 0..MAX_MACHINE_DIAGNOSTICS {
                assert_eq!(
                    syntax.emit(diagnostic(Severity::Warning)).unwrap(),
                    MachineDiagnosticEmit::Retained
                );
            }
            assert_eq!(
                syntax.emit(diagnostic(Severity::Error)).unwrap(),
                MachineDiagnosticEmit::RetainedAfterAdvisoryEviction
            );
        }
        let diagnostics = budget.finish();
        assert_eq!(diagnostics.diagnostics().len(), MAX_MACHINE_DIAGNOSTICS);
        assert_eq!(diagnostics.omitted_count(), 1);
        let last = diagnostics.diagnostics().last().unwrap();
        assert_eq!(last.severity(), Severity::Error);
        assert_eq!(last.notes().last().unwrap().omitted_count(), Some(1));
    }

    #[test]
    fn full_failure_budget_omits_without_materializing_lazy_error() {
        let mut budget = MachineDiagnosticBudget::new();
        let mut materialized = 0;
        {
            let mut syntax = budget.lend(MachineDiagnosticPhase::Syntax).unwrap();
            for _ in 0..MAX_MACHINE_DIAGNOSTICS {
                assert_eq!(
                    syntax
                        .emit_error_with(|| {
                            materialized += 1;
                            diagnostic(Severity::Error)
                        })
                        .unwrap(),
                    MachineDiagnosticEmit::Retained
                );
            }
            assert_eq!(materialized, MAX_MACHINE_DIAGNOSTICS);
            assert_eq!(
                syntax
                    .emit_error_with(|| {
                        materialized += 1;
                        diagnostic(Severity::Error)
                    })
                    .unwrap(),
                MachineDiagnosticEmit::Omitted
            );
            assert_eq!(materialized, MAX_MACHINE_DIAGNOSTICS);
        }
        let diagnostics = budget.finish();
        assert_eq!(diagnostics.omitted_count(), 1);
        assert_eq!(
            diagnostics
                .diagnostics()
                .last()
                .unwrap()
                .notes()
                .last()
                .unwrap()
                .omitted_count(),
            Some(1)
        );
    }

    #[test]
    fn fatal_is_retained_and_terminal_across_phases() {
        let mut budget = MachineDiagnosticBudget::new();
        {
            let mut syntax = budget.lend(MachineDiagnosticPhase::Syntax).unwrap();
            for _ in 0..MAX_MACHINE_DIAGNOSTICS {
                let _ = syntax.emit(diagnostic(Severity::Warning)).unwrap();
            }
            assert_eq!(
                syntax.emit(diagnostic(Severity::Fatal)).unwrap(),
                MachineDiagnosticEmit::Terminal
            );
            assert_eq!(
                syntax.emit(diagnostic(Severity::Note)),
                Err(MachineDiagnosticBudgetError::Terminal)
            );
        }
        assert!(matches!(
            budget.lend(MachineDiagnosticPhase::Layout),
            Err(MachineDiagnosticBudgetError::Terminal)
        ));
        let diagnostics = budget.finish();
        assert_eq!(diagnostics.diagnostics().len(), MAX_MACHINE_DIAGNOSTICS);
        assert_eq!(
            diagnostics.diagnostics().last().unwrap().severity(),
            Severity::Fatal
        );
    }

    #[test]
    fn scoped_lenders_cannot_duplicate_or_reset_phases() {
        let mut budget = MachineDiagnosticBudget::new();
        {
            let _package = budget.lend(MachineDiagnosticPhase::Package).unwrap();
        }
        assert!(matches!(
            budget.lend(MachineDiagnosticPhase::Package),
            Err(MachineDiagnosticBudgetError::PhaseAlreadyLent)
        ));
        assert!(matches!(
            budget.lend(MachineDiagnosticPhase::Host),
            Err(MachineDiagnosticBudgetError::PhaseAlreadyLent)
        ));
        {
            let _syntax = budget.lend(MachineDiagnosticPhase::Syntax).unwrap();
        }
    }

    #[test]
    fn null_location_scope_must_match_its_command_phase() {
        let config = Diagnostic::global(
            I9102,
            Severity::Error,
            "host candidate limit was exceeded",
            GlobalDiagnosticScope::Config,
        )
        .unwrap();
        let mut budget = MachineDiagnosticBudget::new();
        let mut host = budget.lend(MachineDiagnosticPhase::Host).unwrap();
        assert_eq!(
            host.emit(config),
            Err(MachineDiagnosticBudgetError::GlobalScopePhaseMismatch)
        );
    }
}
