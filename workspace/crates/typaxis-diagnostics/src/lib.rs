#![forbid(unsafe_code)]

use typaxis_core::{NodeId, SourceSpan, TextSpan};

pub const DIAGNOSTIC_PREFIXES: &[&str] = &["P1", "T2", "S3", "F4", "L5", "G6", "R7", "D8", "I9"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticCode(String);
impl DiagnosticCode {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.len() == 5
            && DIAGNOSTIC_PREFIXES
                .iter()
                .any(|prefix| bytes[..2] == prefix.as_bytes()[..])
            && bytes[2..].iter().all(|byte| byte.is_ascii_digit())
        {
            Some(Self(value))
        } else {
            None
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Note,
    Warning,
    Error,
    Fatal,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    code: DiagnosticCode,
    severity: Severity,
    message: String,
    source_span: Option<SourceSpan>,
    text_span: Option<TextSpan>,
    node_id: Option<NodeId>,
    notes: Vec<String>,
}
impl Diagnostic {
    pub fn new(
        code: DiagnosticCode,
        severity: Severity,
        message: impl Into<String>,
    ) -> Option<Self> {
        let message = message.into();
        if message.is_empty() {
            return None;
        }
        Some(Self {
            code,
            severity,
            message,
            source_span: None,
            text_span: None,
            node_id: None,
            notes: Vec::new(),
        })
    }
    pub const fn code(&self) -> &DiagnosticCode {
        &self.code
    }
    pub const fn severity(&self) -> Severity {
        self.severity
    }
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A note or warning proven safe to accompany a successful phase value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdvisoryDiagnostic(Diagnostic);
impl AdvisoryDiagnostic {
    pub fn new(diagnostic: Diagnostic) -> Result<Self, Diagnostic> {
        match diagnostic.severity {
            Severity::Note | Severity::Warning => Ok(Self(diagnostic)),
            Severity::Error | Severity::Fatal => Err(diagnostic),
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
/// or fatal, and a fatal diagnostic is necessarily the final diagnostic: no
/// work is permitted to append observations after an immediate abort.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseFailure(Vec<Diagnostic>);
impl ParseFailure {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Result<Self, Vec<Diagnostic>> {
        let has_failure = diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.severity, Severity::Error | Severity::Fatal));
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

/// Result of recording one diagnostic in a phase-owned collector.
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use = "a fatal diagnostic requires the caller to abort immediately"]
pub enum DiagnosticFlow {
    Continue,
    Abort(ParseFailure),
    AlreadyAborted,
}

/// Diagnostic owner for a parser or validation phase.
///
/// Errors may be accumulated until the caller reaches a documented safe phase
/// boundary. A fatal diagnostic seals the collector immediately and returns a
/// complete failure in the same operation. `finish_boundary` can produce
/// advisories only when no error or fatal was ever recorded.
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
            .any(|diagnostic| matches!(diagnostic.severity, Severity::Error | Severity::Fatal))
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

#[cfg(test)]
mod tests {
    use super::*;
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
    fn advisory_wrapper_accepts_only_note_or_warning() {
        let code = DiagnosticCode::new("P1000").unwrap();
        let warning = Diagnostic::new(code.clone(), Severity::Warning, "recoverable").unwrap();
        assert!(AdvisoryDiagnostic::new(warning).is_ok());
        let error = Diagnostic::new(code.clone(), Severity::Error, "failed").unwrap();
        assert!(AdvisoryDiagnostic::new(error).is_err());
        let fatal = Diagnostic::new(code, Severity::Fatal, "stop").unwrap();
        assert!(AdvisoryDiagnostic::new(fatal).is_err());
    }

    #[test]
    fn parse_failure_is_nonempty_and_contains_an_error() {
        let code = DiagnosticCode::new("P1000").unwrap();
        let warning = Diagnostic::new(code.clone(), Severity::Warning, "warning").unwrap();
        assert!(ParseFailure::new(vec![]).is_err());
        assert!(ParseFailure::new(vec![warning.clone()]).is_err());
        let error = Diagnostic::new(code, Severity::Error, "failed").unwrap();
        assert!(ParseFailure::new(vec![warning, error]).is_ok());
    }

    #[test]
    fn fatal_is_terminal_and_seals_the_phase_immediately() {
        let code = DiagnosticCode::new("P1000").unwrap();
        let warning = Diagnostic::new(code.clone(), Severity::Warning, "warning").unwrap();
        let fatal = Diagnostic::new(code.clone(), Severity::Fatal, "fatal").unwrap();
        let after = Diagnostic::new(code, Severity::Note, "too late").unwrap();
        assert!(ParseFailure::new(vec![fatal.clone(), after.clone()]).is_err());

        let mut phase = PhaseDiagnostics::new();
        assert_eq!(phase.emit(warning), DiagnosticFlow::Continue);
        let DiagnosticFlow::Abort(failure) = phase.emit(fatal) else {
            panic!("fatal must request immediate abort");
        };
        assert_eq!(failure.diagnostics().len(), 2);
        assert_eq!(phase.emit(after), DiagnosticFlow::AlreadyAborted);
        assert!(phase.finish_boundary().is_err());
    }

    #[test]
    fn error_fails_at_boundary_and_advisories_can_accompany_success() {
        let code = DiagnosticCode::new("P1000").unwrap();
        let warning = Diagnostic::new(code.clone(), Severity::Warning, "warning").unwrap();
        let error = Diagnostic::new(code, Severity::Error, "error").unwrap();

        let mut successful = PhaseDiagnostics::new();
        assert_eq!(successful.emit(warning.clone()), DiagnosticFlow::Continue);
        assert_eq!(successful.finish_boundary().unwrap().len(), 1);

        let mut failed = PhaseDiagnostics::new();
        assert_eq!(failed.emit(warning), DiagnosticFlow::Continue);
        assert_eq!(failed.emit(error), DiagnosticFlow::Continue);
        assert_eq!(failed.finish_boundary().unwrap_err().diagnostics().len(), 2);
    }
}
