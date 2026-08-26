//! Platform-owned contained-open and filesystem identity operations.

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
mod unix;

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
pub(crate) use unix::{
    admit_root, resolve_write_target, AdmittedRoot, CandidatePathIdentity, FileIdentity,
    FileSnapshot,
};

pub(crate) const CONTAINED_OPEN_AVAILABLE: bool = cfg!(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos"
));

#[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct FileIdentity;

#[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CandidatePathIdentity;
