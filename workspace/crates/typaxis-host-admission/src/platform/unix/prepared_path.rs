//! One reusable NUL-terminated buffer; secure open helpers receive borrowed CStr.
use super::*;
use std::ffi::CStr;
const STACK_BYTES: usize = 256;
enum Storage {
    Stack([u8; STACK_BYTES]),
    Heap(Vec<u8>),
}
pub(crate) struct PreparedReadPath {
    storage: Storage,
    length: usize,
    components: usize,
}
impl PreparedReadPath {
    pub(crate) fn storage_bytes(path: &PortablePath) -> Result<u64, HostAdmissionError> {
        if path.as_str().len() < STACK_BYTES {
            return Ok(0);
        }
        path.as_str()
            .len()
            .checked_add(1)
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(HostAdmissionError::ReadCapacity)
    }
    pub(crate) fn new(path: &PortablePath) -> Result<Self, HostAdmissionError> {
        Self::prefixed(&[], path)
    }
    pub(super) fn prefixed(prefix: &[u8], path: &PortablePath) -> Result<Self, HostAdmissionError> {
        let separator = usize::from(!prefix.is_empty() && !prefix.ends_with(b"/"));
        let start = prefix
            .len()
            .checked_add(separator)
            .ok_or(HostAdmissionError::ReadCapacity)?;
        let length = path
            .as_str()
            .len()
            .checked_add(start)
            .and_then(|n| n.checked_add(1))
            .ok_or(HostAdmissionError::ReadCapacity)?;
        let storage = if length <= STACK_BYTES {
            Storage::Stack([0; STACK_BYTES])
        } else {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(length)
                .map_err(|_| HostAdmissionError::ReadCapacity)?;
            bytes.resize(length, 0);
            Storage::Heap(bytes)
        };
        let mut result = Self {
            storage,
            length,
            components: 0,
        };
        let bytes = result.bytes_mut();
        bytes[..prefix.len()].copy_from_slice(prefix);
        if separator != 0 {
            bytes[prefix.len()] = b'/';
        }
        bytes[start..length - 1].copy_from_slice(path.as_str().as_bytes());
        result.components = result.bytes()[..length - 1]
            .iter()
            .filter(|b| **b == b'/')
            .count()
            + 1;
        Ok(result)
    }
    pub(super) fn bytes(&self) -> &[u8] {
        match &self.storage {
            Storage::Stack(b) => &b[..self.length],
            Storage::Heap(b) => b,
        }
    }
    pub(super) fn bytes_mut(&mut self) -> &mut [u8] {
        match &mut self.storage {
            Storage::Stack(b) => &mut b[..self.length],
            Storage::Heap(b) => b,
        }
    }
    pub(crate) fn root_work(&self) -> Result<u64, HostAdmissionError> {
        let count = u64::try_from(self.components).map_err(|_| HostAdmissionError::ReadCapacity)?;
        let length =
            u64::try_from(self.length - 1).map_err(|_| HostAdmissionError::ReadCapacity)?;
        count
            .checked_mul(128)
            .and_then(|n| length.checked_mul(16).and_then(|b| n.checked_add(b)))
            .and_then(|n| n.checked_add(64))
            .ok_or(HostAdmissionError::ReadCapacity)
    }
    pub(super) fn candidate_identity_matches(
        &mut self,
        root: &AdmittedRoot,
        path: &PortablePath,
        expected: &CandidatePathIdentity,
    ) -> Result<bool, HostAdmissionError> {
        match self.candidate_identity_matches_with_budget(
            root,
            path,
            expected,
            &mut crate::read_budget::UnobservedRead,
        ) {
            Ok(value) => Ok(value),
            Err(crate::HostReadError::Native(error)) => Err(error),
            Err(crate::HostReadError::Budget(failure)) => match failure.error {},
        }
    }
    pub(crate) fn candidate_identity_matches_with_budget<B: crate::HostReadBudget>(
        &mut self,
        root: &AdmittedRoot,
        path: &PortablePath,
        expected: &CandidatePathIdentity,
        budget: &mut B,
    ) -> Result<bool, crate::HostReadError<B::Error>> {
        use std::os::unix::ffi::OsStrExt;
        let mut current = None;
        let mut start = 0;
        loop {
            let end = self.bytes()[start..self.length - 1]
                .iter()
                .position(|b| *b == b'/')
                .map_or(self.length - 1, |n| start + n);
            let opened = if end == self.length - 1 {
                None
            } else {
                self.bytes_mut()[end] = 0;
                let result = {
                    let component = CStr::from_bytes_with_nul(&self.bytes()[start..=end])
                        .map_err(|_| HostAdmissionError::UnsafeCandidate);
                    component.and_then(|c| open_directory_component(root, current.as_ref(), c))
                };
                self.bytes_mut()[end] = b'/';
                match result {
                    // Logical alias identity is separate from contained-open
                    // authority. No owned candidate identity is needed here.
                    Err(HostAdmissionError::UnsafeCandidate) => {
                        return super::logical_candidate::matches_with_budget(
                            root, path, expected, budget,
                        );
                    }
                    Err(error) => return Err(error.into()),
                    Ok(opened) => opened,
                }
            };
            match opened {
                Some(directory) => current = Some(directory),
                None => {
                    let ancestor = snapshot_directory(root, current.as_ref())?.identity;
                    return Ok(ancestor == expected.ancestor
                        && expected
                            .suffix
                            .iter()
                            .map(|s| s.as_bytes())
                            .eq(path.as_str()[start..].split('/').map(str::as_bytes)));
                }
            }
            start = end + 1;
        }
    }
    pub(crate) fn open(
        &mut self,
        root: &AdmittedRoot,
    ) -> Result<Option<OpenedCandidate>, HostAdmissionError> {
        #[cfg(any(target_os = "android", target_os = "linux"))]
        {
            let path = CStr::from_bytes_with_nul(self.bytes())
                .map_err(|_| HostAdmissionError::UnsafeCandidate)?;
            match open_candidate_with_openat2(root, path) {
                Err(Openat2Failure::Unavailable) => (),
                Err(Openat2Failure::Admission(e)) => return Err(e),
                Ok(opened) => return Ok(opened),
            }
        }
        self.walk(root)
    }
    fn walk(&mut self, root: &AdmittedRoot) -> Result<Option<OpenedCandidate>, HostAdmissionError> {
        let mut current = None;
        let mut start = 0;
        loop {
            let end = self.bytes()[start..self.length - 1]
                .iter()
                .position(|b| *b == b'/')
                .map_or(self.length - 1, |n| start + n);
            if end == self.length - 1 {
                let component = CStr::from_bytes_with_nul(&self.bytes()[start..=end])
                    .map_err(|_| HostAdmissionError::UnsafeCandidate)?;
                return open_regular_component(root, current.as_ref(), component);
            }
            self.bytes_mut()[end] = 0;
            let opened = {
                let component = CStr::from_bytes_with_nul(&self.bytes()[start..=end])
                    .map_err(|_| HostAdmissionError::UnsafeCandidate);
                component.and_then(|c| open_directory_component(root, current.as_ref(), c))
            };
            self.bytes_mut()[end] = b'/';
            match opened? {
                Some(directory) => current = Some(directory),
                None => return Ok(None),
            }
            start = end + 1;
        }
    }
    #[cfg(test)]
    pub(crate) fn open_component_walker(
        &mut self,
        root: &AdmittedRoot,
    ) -> Result<Option<OpenedCandidate>, HostAdmissionError> {
        self.walk(root)
    }
}
