use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use typaxis_core::{
    BuildExecutionContext, BuildExecutionError, DiagnosticsExecutionContext, HostPath,
    ReplacePolicy,
};
use typaxis_manifest::PublicationReadLedgerToken;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[allow(dead_code)] // Diagnostics is wired into the private machine runner by MI1-15.
pub enum SidecarArtifact {
    Trace,
    Diagnostics,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[allow(dead_code)] // MI1-15 consumes the complete terminal-artifact vocabulary.
pub enum TerminalArtifact {
    Trace,
    Pdf,
    Diagnostics,
    BuiltManifest,
    FailedManifest,
    StdoutPdfPrefix,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[allow(dead_code)] // MI1-15 carries this set through the private machine runner.
pub struct VisibleArtifactSet(BTreeSet<TerminalArtifact>);

#[allow(dead_code)]
impl VisibleArtifactSet {
    pub fn insert(&mut self, artifact: TerminalArtifact) {
        self.0.insert(artifact);
    }

    pub fn contains(&self, artifact: TerminalArtifact) -> bool {
        self.0.contains(&artifact)
    }

    pub fn artifacts(&self) -> impl Iterator<Item = TerminalArtifact> + '_ {
        self.0.iter().copied()
    }
}

/// Terminal failures explicitly distinguish rollback-safe pre-publication
/// errors, a rollback-impossible stdout prefix, file durability uncertainty,
/// and a later failure after earlier artifacts became visible.
#[derive(Debug)]
#[allow(dead_code)] // Public machine command wiring is intentionally deferred to MI1-15.
pub enum TerminalPublicationError<E> {
    PrePublication {
        failed: TerminalArtifact,
        source: E,
    },
    StdoutPartial {
        bytes_written: u64,
        visible: VisibleArtifactSet,
        source: E,
    },
    FileDurabilityUncertain {
        artifact: TerminalArtifact,
        visible: VisibleArtifactSet,
        source: E,
    },
    AlreadyVisible {
        failed: TerminalArtifact,
        visible: VisibleArtifactSet,
        source: E,
    },
}

/// Processing failure plus both terminal sidecar attempts. The diagnostics
/// result never short-circuits the failed-manifest closure.
#[derive(Debug)]
#[allow(dead_code)] // Public machine command wiring is intentionally deferred to MI1-15.
pub struct CombinedFailurePublication<P, M, ME> {
    primary: P,
    diagnostics: Result<CommitReceipt, CommitError>,
    failed_manifest: Result<M, ME>,
}

#[allow(dead_code)]
impl<P, M, ME> CombinedFailurePublication<P, M, ME> {
    pub const fn primary(&self) -> &P {
        &self.primary
    }

    pub const fn diagnostics(&self) -> &Result<CommitReceipt, CommitError> {
        &self.diagnostics
    }

    pub const fn failed_manifest(&self) -> &Result<M, ME> {
        &self.failed_manifest
    }

    pub fn into_parts(self) -> (P, Result<CommitReceipt, CommitError>, Result<M, ME>) {
        (self.primary, self.diagnostics, self.failed_manifest)
    }
}

#[allow(dead_code)] // MI1-15 supplies the command-owned diagnostics/manifest closures.
pub fn publish_processing_failure<P, M, ME>(
    primary: P,
    publish_diagnostics: impl FnOnce() -> Result<CommitReceipt, CommitError>,
    publish_failed_manifest: impl FnOnce() -> Result<M, ME>,
) -> CombinedFailurePublication<P, M, ME> {
    let diagnostics = publish_diagnostics();
    let failed_manifest = publish_failed_manifest();
    CombinedFailurePublication {
        primary,
        diagnostics,
        failed_manifest,
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CommitReceipt {
    artifact: SidecarArtifact,
    bytes: u64,
}

#[allow(dead_code)]
impl CommitReceipt {
    pub const fn artifact(&self) -> SidecarArtifact {
        self.artifact
    }

    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
}

#[derive(Debug)]
pub enum CommitError {
    PrePublication {
        artifact: SidecarArtifact,
        source: io::Error,
    },
    PublishedButDurabilityUncertain {
        receipt: Box<CommitReceipt>,
        source: io::Error,
    },
}

impl CommitError {
    pub const fn was_published(&self) -> bool {
        matches!(self, Self::PublishedButDurabilityUncertain { .. })
    }

    pub const fn artifact(&self) -> SidecarArtifact {
        match self {
            Self::PrePublication { artifact, .. } => *artifact,
            Self::PublishedButDurabilityUncertain { receipt, .. } => receipt.artifact,
        }
    }

    #[allow(dead_code)] // Consumed by the MI1-15 terminal error mapper.
    pub fn receipt(&self) -> Option<&CommitReceipt> {
        match self {
            Self::PrePublication { .. } => None,
            Self::PublishedButDurabilityUncertain { receipt, .. } => Some(receipt),
        }
    }

    #[cfg(test)]
    fn kind(&self) -> io::ErrorKind {
        match self {
            Self::PrePublication { source, .. }
            | Self::PublishedButDurabilityUncertain { source, .. } => source.kind(),
        }
    }

    fn pre_publication(artifact: SidecarArtifact, source: io::Error) -> Self {
        Self::PrePublication { artifact, source }
    }
}

impl std::fmt::Display for CommitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PrePublication { source, .. } => source.fmt(formatter),
            Self::PublishedButDurabilityUncertain { source, .. } => {
                write!(
                    formatter,
                    "sidecar is visible but directory synchronization failed: {source}"
                )
            }
        }
    }
}

impl std::error::Error for CommitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PrePublication { source, .. }
            | Self::PublishedButDurabilityUncertain { source, .. } => Some(source),
        }
    }
}

/// A complete sidecar temporary. Encoding, allocation, write, and file sync
/// have already succeeded, but the target is not visible until `publish_*`.
/// Dropping the value removes the private temporary on a best-effort basis.
#[derive(Debug)]
pub struct PreparedSidecar {
    artifact: SidecarArtifact,
    target: HostPath,
    parent: PathBuf,
    temporary: Option<PathBuf>,
    replace_policy: ReplacePolicy,
    bytes: u64,
}

#[allow(dead_code)]
impl PreparedSidecar {
    pub const fn artifact(&self) -> SidecarArtifact {
        self.artifact
    }

    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
}

impl Drop for PreparedSidecar {
    fn drop(&mut self) {
        if let Some(temporary) = self.temporary.take() {
            let _ = fs::remove_file(temporary);
        }
    }
}

#[allow(dead_code)] // Compatibility wrapper retained while callers migrate to prepare/publish.
pub fn commit(
    execution: &BuildExecutionContext,
    target: &Path,
    bytes: &[u8],
) -> Result<CommitReceipt, CommitError> {
    let artifact = if execution
        .trace_target()
        .is_some_and(|configured| configured.as_path() == target)
    {
        SidecarArtifact::Trace
    } else if execution
        .diagnostics_target()
        .is_some_and(|configured| configured.as_path() == target)
    {
        SidecarArtifact::Diagnostics
    } else {
        return Err(CommitError::pre_publication(
            SidecarArtifact::Trace,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "sidecar target is not owned by this build execution",
            ),
        ));
    };
    let prepared = prepare_build(execution, artifact, bytes, None)?;
    publish_build(execution, prepared, None)
}

pub fn prepare_build(
    execution: &BuildExecutionContext,
    artifact: SidecarArtifact,
    bytes: &[u8],
    read_ledger: Option<&PublicationReadLedgerToken>,
) -> Result<PreparedSidecar, CommitError> {
    let target = match artifact {
        SidecarArtifact::Trace => execution.trace_target(),
        SidecarArtifact::Diagnostics => execution.diagnostics_target(),
    }
    .ok_or_else(|| {
        CommitError::pre_publication(
            artifact,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "sidecar target was not requested",
            ),
        )
    })?;
    revalidate_build_targets(execution, read_ledger)
        .map_err(|source| execution_error(artifact, source))?;
    prepare_platform(artifact, target, bytes, execution.replace_policy())
}

#[allow(dead_code)] // Diagnostics-only command wiring is introduced by MI1-15.
pub fn prepare_diagnostics(
    execution: &DiagnosticsExecutionContext,
    bytes: &[u8],
    read_ledger: Option<&PublicationReadLedgerToken>,
) -> Result<PreparedSidecar, CommitError> {
    revalidate_diagnostics_target(execution, read_ledger)
        .map_err(|source| execution_error(SidecarArtifact::Diagnostics, source))?;
    prepare_platform(
        SidecarArtifact::Diagnostics,
        execution.diagnostics_target(),
        bytes,
        execution.replace_policy(),
    )
}

pub fn publish_build(
    execution: &BuildExecutionContext,
    prepared: PreparedSidecar,
    read_ledger: Option<&PublicationReadLedgerToken>,
) -> Result<CommitReceipt, CommitError> {
    let configured = match prepared.artifact {
        SidecarArtifact::Trace => execution.trace_target(),
        SidecarArtifact::Diagnostics => execution.diagnostics_target(),
    };
    if configured != Some(&prepared.target) || execution.replace_policy() != prepared.replace_policy
    {
        return Err(CommitError::pre_publication(
            prepared.artifact,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "prepared sidecar does not belong to this build execution",
            ),
        ));
    }
    revalidate_build_targets(execution, read_ledger)
        .map_err(|source| execution_error(prepared.artifact, source))?;
    publish_platform(prepared)
}

#[allow(dead_code)] // Diagnostics-only command wiring is introduced by MI1-15.
pub fn publish_diagnostics(
    execution: &DiagnosticsExecutionContext,
    prepared: PreparedSidecar,
    read_ledger: Option<&PublicationReadLedgerToken>,
) -> Result<CommitReceipt, CommitError> {
    if prepared.artifact != SidecarArtifact::Diagnostics
        || prepared.target != *execution.diagnostics_target()
        || prepared.replace_policy != execution.replace_policy()
    {
        return Err(CommitError::pre_publication(
            SidecarArtifact::Diagnostics,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "prepared diagnostics do not belong to this execution",
            ),
        ));
    }
    revalidate_diagnostics_target(execution, read_ledger)
        .map_err(|source| execution_error(SidecarArtifact::Diagnostics, source))?;
    publish_platform(prepared)
}

fn execution_error(artifact: SidecarArtifact, error: BuildExecutionError) -> CommitError {
    CommitError::pre_publication(
        artifact,
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("publication targets failed revalidation: {error:?}"),
        ),
    )
}

fn revalidate_build_targets(
    execution: &BuildExecutionContext,
    read_ledger: Option<&PublicationReadLedgerToken>,
) -> Result<(), BuildExecutionError> {
    execution.revalidate_write_targets()?;
    let Some(read_ledger) = read_ledger else {
        return Ok(());
    };
    for target in [
        execution.output_path(),
        execution.trace_target(),
        execution.manifest_target(),
        execution.diagnostics_target(),
    ]
    .into_iter()
    .flatten()
    {
        revalidate_read_write_target(read_ledger, target)?;
    }
    Ok(())
}

#[allow(dead_code)] // Reachable through the MI1-15 diagnostics-only publisher.
fn revalidate_diagnostics_target(
    execution: &DiagnosticsExecutionContext,
    read_ledger: Option<&PublicationReadLedgerToken>,
) -> Result<(), BuildExecutionError> {
    execution.revalidate_write_target()?;
    if let Some(read_ledger) = read_ledger {
        revalidate_read_write_target(read_ledger, execution.diagnostics_target())?;
    }
    Ok(())
}

fn revalidate_read_write_target(
    read_ledger: &PublicationReadLedgerToken,
    target: &HostPath,
) -> Result<(), BuildExecutionError> {
    match read_ledger.revalidate_write_target(target) {
        Ok(false) => Ok(()),
        Ok(true) => Err(BuildExecutionError::AliasedReadWriteTarget),
        Err(_) => Err(BuildExecutionError::ReadTargetChanged),
    }
}

#[cfg(unix)]
fn prepare_platform(
    artifact: SidecarArtifact,
    target: &HostPath,
    bytes: &[u8],
    replace_policy: ReplacePolicy,
) -> Result<PreparedSidecar, CommitError> {
    let target_path = target.as_path();
    if replace_policy == ReplacePolicy::NoReplace {
        match fs::symlink_metadata(target_path) {
            Ok(_) => {
                return Err(CommitError::pre_publication(
                    artifact,
                    io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "sidecar target already exists",
                    ),
                ))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(CommitError::pre_publication(artifact, source)),
        }
    }
    let parent = target_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let leaf = target_path.file_name().ok_or_else(|| {
        CommitError::pre_publication(
            artifact,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "sidecar target has no file name",
            ),
        )
    })?;
    let (temporary, mut file) = create_temporary(parent, leaf)
        .map_err(|source| CommitError::pre_publication(artifact, source))?;
    if let Err(source) = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })() {
        let _ = fs::remove_file(&temporary);
        return Err(CommitError::pre_publication(artifact, source));
    }
    Ok(PreparedSidecar {
        artifact,
        target: target.clone(),
        parent: parent.to_path_buf(),
        temporary: Some(temporary),
        replace_policy,
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
    })
}

#[cfg(unix)]
fn create_temporary(parent: &Path, leaf: &OsStr) -> io::Result<(PathBuf, File)> {
    for _ in 0..128 {
        let ordinal = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut name = OsString::from(".");
        name.push(leaf);
        name.push(format!(
            ".typaxis-sidecar-{}-{ordinal}.tmp",
            std::process::id()
        ));
        let path = parent.join(name);
        match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique sidecar temporary",
    ))
}

#[cfg(not(unix))]
fn prepare_platform(
    artifact: SidecarArtifact,
    _target: &HostPath,
    _bytes: &[u8],
    _replace_policy: ReplacePolicy,
) -> Result<PreparedSidecar, CommitError> {
    Err(CommitError::pre_publication(
        artifact,
        io::Error::new(
            io::ErrorKind::Unsupported,
            "no atomic sidecar committer is registered for this platform",
        ),
    ))
}

#[cfg(unix)]
fn publish_platform(mut prepared: PreparedSidecar) -> Result<CommitReceipt, CommitError> {
    let temporary = prepared.temporary.as_ref().ok_or_else(|| {
        CommitError::pre_publication(
            prepared.artifact,
            io::Error::new(io::ErrorKind::InvalidInput, "sidecar temporary is missing"),
        )
    })?;
    let result = match prepared.replace_policy {
        ReplacePolicy::NoReplace => fs::hard_link(temporary, prepared.target.as_path()),
        ReplacePolicy::Replace => fs::rename(temporary, prepared.target.as_path()),
    };
    if let Err(source) = result {
        return Err(CommitError::pre_publication(prepared.artifact, source));
    }
    if prepared.replace_policy == ReplacePolicy::NoReplace {
        if let Some(temporary) = prepared.temporary.take() {
            let _ = fs::remove_file(temporary);
        }
    } else {
        prepared.temporary = None;
    }
    let receipt = CommitReceipt {
        artifact: prepared.artifact,
        bytes: prepared.bytes,
    };
    match File::open(&prepared.parent).and_then(|directory| directory.sync_all()) {
        Ok(()) => Ok(receipt),
        Err(source) => Err(CommitError::PublishedButDurabilityUncertain {
            receipt: Box::new(receipt),
            source,
        }),
    }
}

#[cfg(not(unix))]
fn publish_platform(prepared: PreparedSidecar) -> Result<CommitReceipt, CommitError> {
    Err(CommitError::pre_publication(
        prepared.artifact,
        io::Error::new(
            io::ErrorKind::Unsupported,
            "no atomic sidecar publisher is registered for this platform",
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::ffi::OsStr;
    use std::time::{SystemTime, UNIX_EPOCH};
    use typaxis_core::{
        BuildExecutionContext, DiagnosticsExecutionContext, HostPath, ResourceLimits,
        ValidatedResourceLimits,
    };
    use typaxis_machine_input::{HostMachineInputSession, MachineInputHostOptions};

    struct TempTree(PathBuf);

    impl TempTree {
        fn new(label: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "typaxis-sidecar-{label}-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(unix)]
    #[derive(Clone, Copy, Debug)]
    enum WriteTargetSlot {
        Output,
        Trace,
        Manifest,
        Diagnostics,
    }

    #[cfg(unix)]
    fn execution_with_selected_target(
        root: &Path,
        slot: WriteTargetSlot,
        selected: HostPath,
        tag: &str,
    ) -> BuildExecutionContext {
        let mut output = root.join(format!("{tag}-output.pdf"));
        let mut trace = HostPath::new(root.join(format!("{tag}-trace.json"))).unwrap();
        let mut manifest = HostPath::new(root.join(format!("{tag}-manifest.json"))).unwrap();
        let mut diagnostics = HostPath::new(root.join(format!("{tag}-diagnostics.json"))).unwrap();
        match slot {
            WriteTargetSlot::Output => output = selected.as_path().to_path_buf(),
            WriteTargetSlot::Trace => trace = selected,
            WriteTargetSlot::Manifest => manifest = selected,
            WriteTargetSlot::Diagnostics => diagnostics = selected,
        }
        BuildExecutionContext::from_cli_token(
            output.as_os_str(),
            Some(trace),
            Some(manifest),
            Some(diagnostics),
            ReplacePolicy::Replace,
        )
        .unwrap()
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn read_token_for_path(path: &Path) -> PublicationReadLedgerToken {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let options =
            MachineInputHostOptions::new(HostPath::new(path.to_path_buf()).unwrap(), None);
        let (session, _raw) = HostMachineInputSession::open(options, &limits).unwrap();
        session.read_ledger_token().unwrap()
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn missing_read_token_for_path(path: &Path) -> PublicationReadLedgerToken {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let options =
            MachineInputHostOptions::new(HostPath::new(path.to_path_buf()).unwrap(), None);
        match HostMachineInputSession::open(options, &limits) {
            Ok(_) => panic!("missing read candidate unexpectedly opened"),
            Err(error) => error.read_ledger_token().unwrap(),
        }
    }

    #[test]
    fn post_publication_sync_failure_retains_visible_state() {
        let error = CommitError::PublishedButDurabilityUncertain {
            receipt: Box::new(CommitReceipt {
                artifact: SidecarArtifact::Diagnostics,
                bytes: 2,
            }),
            source: io::Error::other("sync failed"),
        };
        assert!(error.was_published());
        assert!(error.to_string().contains("visible"));
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(error.artifact(), SidecarArtifact::Diagnostics);
        assert_eq!(error.receipt().unwrap().bytes(), 2);
    }

    #[test]
    fn processing_failure_attempts_manifest_after_diagnostics_failure() {
        let order = RefCell::new(Vec::new());
        let outcome = publish_processing_failure(
            "primary processing failure",
            || {
                order.borrow_mut().push(TerminalArtifact::Diagnostics);
                Err(CommitError::pre_publication(
                    SidecarArtifact::Diagnostics,
                    io::Error::other("diagnostics failed"),
                ))
            },
            || {
                order.borrow_mut().push(TerminalArtifact::FailedManifest);
                Err::<(), _>("manifest failed")
            },
        );
        assert_eq!(
            *order.borrow(),
            [
                TerminalArtifact::Diagnostics,
                TerminalArtifact::FailedManifest
            ]
        );
        assert_eq!(outcome.primary(), &"primary processing failure");
        assert!(outcome.diagnostics().is_err());
        assert_eq!(outcome.failed_manifest(), &Err("manifest failed"));
        let (primary, diagnostics, manifest) = outcome.into_parts();
        assert_eq!(primary, "primary processing failure");
        assert!(diagnostics.is_err());
        assert_eq!(manifest, Err("manifest failed"));
    }

    #[test]
    fn partial_publication_variants_keep_visible_artifacts_separate() {
        let pre = TerminalPublicationError::PrePublication {
            failed: TerminalArtifact::Trace,
            source: "pre",
        };
        let mut visible = VisibleArtifactSet::default();
        visible.insert(TerminalArtifact::Trace);
        let partial = TerminalPublicationError::AlreadyVisible {
            failed: TerminalArtifact::Pdf,
            visible: visible.clone(),
            source: "partial",
        };
        let mut stdout_visible = visible.clone();
        stdout_visible.insert(TerminalArtifact::StdoutPdfPrefix);
        let stdout = TerminalPublicationError::StdoutPartial {
            bytes_written: 17,
            visible: stdout_visible,
            source: "stdout",
        };
        let mut durable_visible = visible;
        durable_visible.insert(TerminalArtifact::Pdf);
        let durability = TerminalPublicationError::FileDurabilityUncertain {
            artifact: TerminalArtifact::Pdf,
            visible: durable_visible,
            source: "sync",
        };

        assert!(matches!(
            pre,
            TerminalPublicationError::PrePublication { .. }
        ));
        let TerminalPublicationError::AlreadyVisible { visible, .. } = partial else {
            unreachable!()
        };
        assert!(visible.contains(TerminalArtifact::Trace));
        assert_eq!(
            visible.artifacts().collect::<Vec<_>>(),
            [TerminalArtifact::Trace]
        );
        assert!(matches!(
            stdout,
            TerminalPublicationError::StdoutPartial {
                bytes_written: 17,
                ..
            }
        ));
        assert!(matches!(
            durability,
            TerminalPublicationError::FileDurabilityUncertain { .. }
        ));
    }

    #[test]
    #[cfg(unix)]
    fn sidecar_commit_honors_no_replace() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("typaxis-sidecar-{unique}"));
        fs::create_dir(&directory).unwrap();
        let output = directory.join("output.pdf");
        let trace = directory.join("trace.json");
        let execution = BuildExecutionContext::from_cli_token(
            output.as_os_str(),
            Some(HostPath::new(trace.clone()).unwrap()),
            None,
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let receipt = commit(&execution, &trace, b"first").unwrap();
        assert_eq!(receipt.artifact(), SidecarArtifact::Trace);
        assert_eq!(receipt.bytes(), 5);
        assert_eq!(fs::read(&trace).unwrap(), b"first");
        assert_eq!(
            commit(&execution, &trace, b"second").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        fs::remove_file(trace).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn sidecar_revalidates_aliases_created_after_cli_admission() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("typaxis-sidecar-alias-{unique}"));
        fs::create_dir(&directory).unwrap();
        let output = directory.join("output.pdf");
        let trace = directory.join("trace.json");
        let execution = BuildExecutionContext::from_cli_token(
            output.as_os_str(),
            Some(HostPath::new(trace.clone()).unwrap()),
            None,
            None,
            ReplacePolicy::Replace,
        )
        .unwrap();

        fs::write(&output, b"existing output").unwrap();
        fs::hard_link(&output, &trace).unwrap();
        let error = commit(&execution, &trace, b"must not be written").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(fs::read(&output).unwrap(), b"existing output");
        assert_eq!(fs::read(&trace).unwrap(), b"existing output");

        fs::remove_file(trace).unwrap();
        fs::remove_file(output).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn stdout_output_does_not_turn_a_trace_into_stdout() {
        let trace = HostPath::new("trace.json").unwrap();
        let execution = BuildExecutionContext::from_cli_token(
            OsStr::new("-"),
            Some(trace),
            None,
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        assert!(execution.output_path().is_none());
        assert!(execution.trace_target().is_some());
    }

    #[test]
    #[cfg(unix)]
    fn diagnostics_only_context_prepares_and_publishes_without_pdf() {
        let tree = TempTree::new("diagnostics-only");
        let target = tree.path().join("diagnostics.json");
        let execution = DiagnosticsExecutionContext::new(
            HostPath::new(target.clone()).unwrap(),
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let prepared = prepare_diagnostics(&execution, b"{\"diagnostics\":[]}", None).unwrap();
        assert_eq!(prepared.artifact(), SidecarArtifact::Diagnostics);
        let receipt = publish_diagnostics(&execution, prepared, None).unwrap();
        assert_eq!(receipt.artifact(), SidecarArtifact::Diagnostics);
        assert_eq!(fs::read(target).unwrap(), b"{\"diagnostics\":[]}");
    }

    #[test]
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn every_build_write_target_is_checked_against_the_sealed_read_set() {
        let tree = TempTree::new("read-write-pairs");
        for name in ["package.json", "source.json", "config.toml", "font.ttf"] {
            fs::write(tree.path().join(name), name.as_bytes()).unwrap();
        }
        let safe_trace = || HostPath::new(tree.path().join("trace.json")).unwrap();
        let safe_output = tree.path().join("output.pdf");

        let cases = [
            (
                tree.path().join("package.json"),
                BuildExecutionContext::from_cli_token(
                    tree.path().join("package.json").as_os_str(),
                    Some(safe_trace()),
                    None,
                    None,
                    ReplacePolicy::Replace,
                )
                .unwrap(),
            ),
            (
                tree.path().join("source.json"),
                BuildExecutionContext::from_cli_token(
                    safe_output.as_os_str(),
                    Some(HostPath::new(tree.path().join("source.json")).unwrap()),
                    None,
                    None,
                    ReplacePolicy::Replace,
                )
                .unwrap(),
            ),
            (
                tree.path().join("config.toml"),
                BuildExecutionContext::from_cli_token(
                    safe_output.as_os_str(),
                    Some(safe_trace()),
                    Some(HostPath::new(tree.path().join("config.toml")).unwrap()),
                    None,
                    ReplacePolicy::Replace,
                )
                .unwrap(),
            ),
            (
                tree.path().join("font.ttf"),
                BuildExecutionContext::from_cli_token(
                    safe_output.as_os_str(),
                    Some(safe_trace()),
                    None,
                    Some(HostPath::new(tree.path().join("font.ttf")).unwrap()),
                    ReplacePolicy::Replace,
                )
                .unwrap(),
            ),
        ];
        for (input, execution) in &cases {
            let token = read_token_for_path(input);
            let error = prepare_build(execution, SidecarArtifact::Trace, b"trace", Some(&token))
                .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert!(error.to_string().contains("AliasedReadWriteTarget"));
        }
    }

    #[test]
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn every_write_target_rejects_every_input_alias_kind_and_publish_race() {
        use std::os::unix::fs::symlink;

        let tree = TempTree::new("complete-read-write-matrix");
        let input_names = ["package.json", "source.tsf", "config.toml", "font.ttf"];
        for name in input_names {
            fs::write(tree.path().join(name), name.as_bytes()).unwrap();
        }
        let slots = [
            WriteTargetSlot::Output,
            WriteTargetSlot::Trace,
            WriteTargetSlot::Manifest,
            WriteTargetSlot::Diagnostics,
        ];

        for (input_index, input_name) in input_names.into_iter().enumerate() {
            let input = tree.path().join(input_name);
            let token = read_token_for_path(&input);
            let symlink_alias = tree.path().join(format!("input-{input_index}-symlink"));
            let hard_link_alias = tree.path().join(format!("input-{input_index}-hard-link"));
            symlink(&input, &symlink_alias).unwrap();
            fs::hard_link(&input, &hard_link_alias).unwrap();
            let aliases = [
                tree.path().join(".").join(input_name),
                symlink_alias,
                hard_link_alias,
            ];
            for (alias_index, alias) in aliases.into_iter().enumerate() {
                for (slot_index, slot) in slots.into_iter().enumerate() {
                    let tag = format!("alias-{input_index}-{alias_index}-{slot_index}");
                    let execution = execution_with_selected_target(
                        tree.path(),
                        slot,
                        HostPath::new(alias.clone()).unwrap(),
                        &tag,
                    );
                    let error =
                        prepare_build(&execution, SidecarArtifact::Trace, b"trace", Some(&token))
                            .unwrap_err();
                    assert!(
                        error.to_string().contains("AliasedReadWriteTarget"),
                        "{slot:?} accepted alias `{}` for {input_name}: {error}",
                        alias.display()
                    );
                }
            }
        }

        for (input_index, input_name) in input_names.into_iter().enumerate() {
            let input = tree.path().join(input_name);
            let token = read_token_for_path(&input);
            for (slot_index, slot) in slots.into_iter().enumerate() {
                let tag = format!("race-{input_index}-{slot_index}");
                let raced_target = tree.path().join(format!("{tag}-target"));
                let execution = execution_with_selected_target(
                    tree.path(),
                    slot,
                    HostPath::new(raced_target.clone()).unwrap(),
                    &tag,
                );
                let prepared =
                    prepare_build(&execution, SidecarArtifact::Trace, b"trace", Some(&token))
                        .unwrap();
                fs::hard_link(&input, &raced_target).unwrap();
                let error = publish_build(&execution, prepared, Some(&token)).unwrap_err();
                assert!(
                    error.to_string().contains("AliasedReadWriteTarget"),
                    "{slot:?} missed publish race against {input_name}: {error}"
                );
                assert_eq!(fs::read(&raced_target).unwrap(), fs::read(&input).unwrap());
                fs::remove_file(raced_target).unwrap();
            }
        }
    }

    #[test]
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn missing_read_candidate_and_post_temp_race_cannot_be_created_by_force() {
        let tree = TempTree::new("missing-read-race");
        let target = tree.path().join("missing-resource.bin");
        let token = missing_read_token_for_path(&target);
        let execution = DiagnosticsExecutionContext::new(
            HostPath::new(target.clone()).unwrap(),
            ReplacePolicy::Replace,
        )
        .unwrap();
        assert!(prepare_diagnostics(&execution, b"diagnostics", Some(&token)).is_err());
        assert!(!target.exists());

        let input = tree.path().join("input.json");
        fs::write(&input, b"first identity").unwrap();
        let race_token = read_token_for_path(&input);
        let diagnostics = tree.path().join("race-diagnostics.json");
        let race_execution = DiagnosticsExecutionContext::new(
            HostPath::new(diagnostics.clone()).unwrap(),
            ReplacePolicy::Replace,
        )
        .unwrap();
        let prepared =
            prepare_diagnostics(&race_execution, b"diagnostics", Some(&race_token)).unwrap();
        let old = tree.path().join("old-input.json");
        fs::rename(&input, &old).unwrap();
        fs::write(&input, b"replacement identity").unwrap();
        let error = publish_diagnostics(&race_execution, prepared, Some(&race_token)).unwrap_err();
        assert!(error.to_string().contains("ReadTargetChanged"));
        assert!(!diagnostics.exists());
    }

    #[test]
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn read_write_alias_guard_catches_lexical_symlink_and_hard_link_targets() {
        use std::os::unix::fs::symlink;

        let tree = TempTree::new("read-write-alias-kinds");
        let input = tree.path().join("input.json");
        fs::write(&input, b"input").unwrap();
        let token = read_token_for_path(&input);

        let lexical = tree.path().join(".").join("input.json");
        let symlink_path = tree.path().join("input-symlink.json");
        let hard_link_path = tree.path().join("input-hard-link.json");
        symlink(&input, &symlink_path).unwrap();
        fs::hard_link(&input, &hard_link_path).unwrap();
        for target in [lexical, symlink_path, hard_link_path] {
            let execution = DiagnosticsExecutionContext::new(
                HostPath::new(target).unwrap(),
                ReplacePolicy::Replace,
            )
            .unwrap();
            let error =
                prepare_diagnostics(&execution, b"must not publish", Some(&token)).unwrap_err();
            assert!(error.to_string().contains("AliasedReadWriteTarget"));
        }
        assert_eq!(fs::read(input).unwrap(), b"input");
    }

    #[test]
    #[cfg(unix)]
    fn publish_rechecks_write_aliases_after_the_temporary_is_complete() {
        let tree = TempTree::new("post-temp-write-alias");
        let output = tree.path().join("output.pdf");
        let trace = tree.path().join("trace.json");
        let execution = BuildExecutionContext::from_cli_token(
            output.as_os_str(),
            Some(HostPath::new(trace.clone()).unwrap()),
            None,
            None,
            ReplacePolicy::Replace,
        )
        .unwrap();
        let prepared =
            prepare_build(&execution, SidecarArtifact::Trace, b"new trace", None).unwrap();
        fs::write(&output, b"existing output").unwrap();
        fs::hard_link(&output, &trace).unwrap();
        let error = publish_build(&execution, prepared, None).unwrap_err();
        assert!(error.to_string().contains("AliasedWriteTarget"));
        assert_eq!(fs::read(&trace).unwrap(), b"existing output");
    }
}
