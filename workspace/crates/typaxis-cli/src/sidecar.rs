use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use typaxis_core::{BuildExecutionContext, ReplacePolicy};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum CommitError {
    PrePublication(io::Error),
    PublishedButDurabilityUncertain(io::Error),
}

impl CommitError {
    pub const fn was_published(&self) -> bool {
        matches!(self, Self::PublishedButDurabilityUncertain(_))
    }

    #[cfg(test)]
    fn kind(&self) -> io::ErrorKind {
        match self {
            Self::PrePublication(error) | Self::PublishedButDurabilityUncertain(error) => {
                error.kind()
            }
        }
    }
}

impl std::fmt::Display for CommitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PrePublication(error) => error.fmt(formatter),
            Self::PublishedButDurabilityUncertain(error) => {
                write!(
                    formatter,
                    "sidecar is visible but directory synchronization failed: {error}"
                )
            }
        }
    }
}

impl std::error::Error for CommitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PrePublication(error) | Self::PublishedButDurabilityUncertain(error) => {
                Some(error)
            }
        }
    }
}

pub fn commit(
    execution: &BuildExecutionContext,
    target: &Path,
    bytes: &[u8],
) -> Result<(), CommitError> {
    execution.revalidate_write_targets().map_err(|error| {
        CommitError::PrePublication(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("write targets are no longer distinct: {error:?}"),
        ))
    })?;
    commit_platform(execution, target, bytes, execution.replace_policy())
}

#[cfg(unix)]
fn commit_platform(
    execution: &BuildExecutionContext,
    target: &Path,
    bytes: &[u8],
    replace_policy: ReplacePolicy,
) -> Result<(), CommitError> {
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let leaf = target.file_name().ok_or_else(|| {
        CommitError::PrePublication(io::Error::new(
            io::ErrorKind::InvalidInput,
            "sidecar target has no file name",
        ))
    })?;
    let (temporary, mut file) =
        create_temporary(parent, leaf).map_err(CommitError::PrePublication)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        execution.revalidate_write_targets().map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("write targets are no longer distinct: {error:?}"),
            )
        })?;
        match replace_policy {
            ReplacePolicy::NoReplace => fs::hard_link(&temporary, target)?,
            ReplacePolicy::Replace => fs::rename(&temporary, target)?,
        }
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(CommitError::PrePublication(error));
    }
    if replace_policy == ReplacePolicy::NoReplace {
        let _ = fs::remove_file(&temporary);
    }
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(CommitError::PublishedButDurabilityUncertain)
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
fn commit_platform(
    _execution: &BuildExecutionContext,
    _target: &Path,
    _bytes: &[u8],
    _replace_policy: ReplacePolicy,
) -> Result<(), CommitError> {
    Err(CommitError::PrePublication(io::Error::new(
        io::ErrorKind::Unsupported,
        "no atomic sidecar committer is registered for this platform",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::time::{SystemTime, UNIX_EPOCH};
    use typaxis_core::{BuildExecutionContext, HostPath};

    #[test]
    fn post_publication_sync_failure_retains_visible_state() {
        let error = CommitError::PublishedButDurabilityUncertain(io::Error::other("sync failed"));
        assert!(error.was_published());
        assert!(error.to_string().contains("visible"));
        assert_eq!(error.kind(), io::ErrorKind::Other);
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
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        commit(&execution, &trace, b"first").unwrap();
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
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        assert!(execution.output_path().is_none());
        assert!(execution.trace_target().is_some());
    }
}
