//! Logical aliases use native directory metadata, never contained-read authority.
use super::*;
use std::ffi::{CStr, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;

fn stat_prefix(
    path: &mut PreparedReadPath,
    length: usize,
) -> Result<rustix::fs::Stat, HostAdmissionError> {
    let saved = path.bytes()[length];
    path.bytes_mut()[length] = 0;
    let result = CStr::from_bytes_with_nul(&path.bytes()[..=length])
        .map_err(|_| HostAdmissionError::UnsafeCandidate)
        .and_then(|name| rustix::fs::stat(name).map_err(|_| HostAdmissionError::Read));
    path.bytes_mut()[length] = saved;
    result
}

#[cfg(test)]
pub(super) fn matches(
    root: &AdmittedRoot,
    relative: &PortablePath,
    expected: &CandidatePathIdentity,
) -> Result<bool, HostAdmissionError> {
    match matches_with_budget(
        root,
        relative,
        expected,
        &mut crate::read_budget::UnobservedRead,
    ) {
        Ok(value) => Ok(value),
        Err(crate::HostReadError::Native(error)) => Err(error),
        Err(crate::HostReadError::Budget(failure)) => match failure.error {},
    }
}

pub(super) fn matches_with_budget<B: crate::HostReadBudget>(
    root: &AdmittedRoot,
    relative: &PortablePath,
    expected: &CandidatePathIdentity,
    budget: &mut B,
) -> Result<bool, crate::HostReadError<B::Error>> {
    use crate::{read_budget, HostReadRequest, HostReadStage};
    let observed = B::OBSERVED && B::OBSERVE_LEDGER;
    if observed {
        let prefix = root.canonical_path.as_os_str().as_bytes();
        let length = prefix
            .len()
            .checked_add(usize::from(!prefix.ends_with(b"/")))
            .and_then(|n| n.checked_add(relative.as_str().len()))
            .and_then(|n| n.checked_add(1))
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(HostAdmissionError::ReadCapacity)?;
        read_budget::charge(
            budget,
            HostReadRequest {
                stage: HostReadStage::LedgerAliasPath,
                file_offset: 0,
                buffer_bytes: if length > 256 { length } else { 0 },
                work: length
                    .checked_mul(16)
                    .and_then(|n| n.checked_add(320))
                    .ok_or(HostAdmissionError::ReadCapacity)?,
            },
        )?;
    }
    let mut path =
        PreparedReadPath::prefixed(root.canonical_path.as_os_str().as_bytes(), relative)?;
    let length = path.bytes().len() - 1;
    let slash = path.bytes()[..length]
        .iter()
        .rposition(|b| *b == b'/')
        .ok_or(HostAdmissionError::UnsafeCandidate)?;
    let mut parent = slash.max(1);
    let mut suffix = slash + 1;
    loop {
        if observed {
            read_budget::charge(
                budget,
                HostReadRequest {
                    stage: HostReadStage::LedgerAliasProbe,
                    file_offset: 0,
                    buffer_bytes: 0,
                    work: (parent as u64)
                        .checked_mul(16)
                        .and_then(|n| n.checked_add(128))
                        .ok_or(HostAdmissionError::ReadCapacity)?,
                },
            )?;
        }
        // As in the owned resolver, unavailable or nondirectory parents become
        // logical suffix components. A successful stat follows native aliases.
        if stat_prefix(&mut path, parent).is_ok_and(|stat| {
            rustix::fs::FileType::from_raw_mode(stat.st_mode) == rustix::fs::FileType::Directory
        }) {
            if observed {
                read_budget::charge(
                    budget,
                    HostReadRequest {
                        stage: HostReadStage::LedgerAliasCanonicalize,
                        file_offset: 0,
                        buffer_bytes: 0,
                        work: (parent as u64)
                            .checked_mul(16)
                            .and_then(|n| n.checked_add(256))
                            .ok_or(HostAdmissionError::ReadCapacity)?,
                    },
                )?;
            }
            // Keep canonicalization's native error semantics. In particular,
            // metadata can still succeed for a magic link to an unlinked
            // directory, while canonicalization correctly rejects that path.
            let ancestor = Path::new(OsStr::from_bytes(&path.bytes()[..parent]));
            let canonical =
                std::fs::canonicalize(ancestor).map_err(|_| HostAdmissionError::Read)?;
            let metadata = std::fs::metadata(&canonical).map_err(|_| HostAdmissionError::Read)?;
            if !metadata.is_dir() {
                return Err(HostAdmissionError::UnsafeCandidate.into());
            }
            let actual = FileIdentity {
                device: u128::from(metadata.dev()),
                inode: u128::from(metadata.ino()),
            };
            return Ok(actual == expected.ancestor
                && expected
                    .suffix
                    .iter()
                    .map(|s| s.as_bytes())
                    .eq(path.bytes()[suffix..length].split(|b| *b == b'/')));
        }
        if parent == 1 {
            return Err(HostAdmissionError::UnsafeCandidate.into());
        }
        let slash = path.bytes()[..parent]
            .iter()
            .rposition(|b| *b == b'/')
            .ok_or(HostAdmissionError::UnsafeCandidate)?;
        suffix = slash + 1;
        parent = slash.max(1);
    }
}
