//! Share an authenticated owned buffer without copying it into an Arc slice.
//! External slice-based APIs retain their original Arc and pointer affiliation.
use super::*;

#[derive(Clone)]
pub(super) enum SourceBytesV2 {
    Slice(Arc<[u8]>),
    Owned(Arc<Vec<u8>>),
}
impl std::fmt::Debug for SourceBytesV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceBytesV2")
            .field("byte_length", &self.len())
            .finish_non_exhaustive()
    }
}
impl std::ops::Deref for SourceBytesV2 {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match self {
            Self::Slice(bytes) => bytes,
            Self::Owned(bytes) => bytes.as_slice(),
        }
    }
}
impl From<Arc<[u8]>> for SourceBytesV2 {
    fn from(bytes: Arc<[u8]>) -> Self {
        Self::Slice(bytes)
    }
}
#[cfg(test)]
impl From<Vec<u8>> for SourceBytesV2 {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Owned(Arc::new(bytes))
    }
}

/// Logical Vec header and two reference counters. This excludes the caller's
/// already owned byte allocation, allocator overhead and live-graph storage.
pub(super) const OWNER_BYTES: usize =
    std::mem::size_of::<Vec<u8>>() + 2 * std::mem::size_of::<usize>();
pub(super) const OWNER_WORK: u64 = 64;

pub(super) fn from_owned<B: CffAdmissionBudgetV2>(
    bytes: Vec<u8>,
    budget: &mut B,
) -> Result<SourceBytesV2, CffAdmissionBudgetFailureV2<B::Error>> {
    if B::OBSERVE_SOURCE_OWNER {
        let point = CffMetadataWorkPointV2 {
            stage: CffMetadataStageV2::SourceOwner,
            table_tag: None,
            table_offset: None,
            file_offset: None,
        };
        let failure = |error| {
            CffAdmissionBudgetFailureV2::Metadata(CffMetadataBudgetFailureV2 { error, point })
        };
        budget.work_metadata(OWNER_WORK).map_err(failure)?;
        budget
            .reserve_metadata(CffMetadataStorageV2::SharedSourceOwner, 1, OWNER_BYTES)
            .map_err(failure)?;
    }
    Ok(SourceBytesV2::Owned(Arc::new(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Request {
        Work(u64),
        Reserve(CffMetadataStorageV2, usize, usize),
    }
    #[derive(Default)]
    struct Trace {
        accepted: Vec<Request>,
        stop: Option<usize>,
    }
    impl Trace {
        fn request(&mut self, request: Request) -> Result<(), usize> {
            if self.stop == Some(self.accepted.len()) {
                return Err(self.accepted.len());
            }
            self.accepted.push(request);
            Ok(())
        }
    }
    impl CffBaseCmapBudgetV2 for Trace {
        type Error = usize;
        const OBSERVED: bool = false;
        fn reserve(&mut self, _: CffBaseCmapStorageV2, _: usize, _: usize) -> Result<(), usize> {
            unreachable!()
        }
        fn work(&mut self, _: u64) -> Result<(), usize> {
            unreachable!()
        }
    }
    impl CffCmapBudgetV2 for Trace {
        const OBSERVE_VARIATIONS: bool = false;
        fn reserve_variation(
            &mut self,
            _: CffVariationStorageV2,
            _: usize,
            _: usize,
        ) -> Result<(), usize> {
            unreachable!()
        }
    }
    impl CffAdmissionBudgetV2 for Trace {
        const OBSERVE_SFNT: bool = false;
        const OBSERVE_SOURCE_OWNER: bool = true;
        fn reserve_sfnt(&mut self, _: CffSfntStorageV2, _: usize, _: usize) -> Result<(), usize> {
            unreachable!()
        }
        fn work_sfnt(&mut self, _: u64) -> Result<(), usize> {
            unreachable!()
        }
        fn work_metadata(&mut self, n: u64) -> Result<(), usize> {
            self.request(Request::Work(n))
        }
        fn reserve_metadata(
            &mut self,
            s: CffMetadataStorageV2,
            r: usize,
            b: usize,
        ) -> Result<(), usize> {
            self.request(Request::Reserve(s, r, b))
        }
    }
    #[test]
    fn source_owner_moves_the_buffer_and_keeps_shared_views_alive() {
        for length in [0, 1, 63, 64, 8192, 65536] {
            let mut bytes = Vec::with_capacity(length + 1024);
            bytes.extend((0..length).map(|n| (n % 251) as u8));
            let pointer = bytes.as_ptr();
            let capacity = bytes.capacity();
            let mut trace = Trace::default();
            let source = from_owned(bytes, &mut trace).unwrap();
            assert_eq!(
                trace.accepted,
                [
                    Request::Work(64),
                    Request::Reserve(CffMetadataStorageV2::SharedSourceOwner, 1, OWNER_BYTES)
                ]
            );
            assert_eq!(source.as_ptr(), pointer);
            let SourceBytesV2::Owned(shared) = &source else {
                panic!()
            };
            assert_eq!(shared.capacity(), capacity);
            let clone = source.clone();
            let SourceBytesV2::Owned(other) = &clone else {
                panic!()
            };
            assert!(Arc::ptr_eq(shared, other));
            drop(source);
            assert_eq!(clone.as_ptr(), pointer);
            assert_eq!(clone.len(), length);
            assert!(clone.iter().enumerate().all(|(n, v)| *v == (n % 251) as u8));
        }
    }
    #[test]
    fn source_owner_refusal_keeps_typed_point_and_paid_prefix() {
        let expected = [
            Request::Work(64),
            Request::Reserve(CffMetadataStorageV2::SharedSourceOwner, 1, OWNER_BYTES),
        ];
        for stop in 0..2 {
            let mut trace = Trace {
                stop: Some(stop),
                ..Trace::default()
            };
            let CffAdmissionBudgetFailureV2::Metadata(f) =
                from_owned(vec![7; 65536], &mut trace).unwrap_err()
            else {
                panic!()
            };
            assert_eq!(f.error, stop);
            assert_eq!(f.point.stage, CffMetadataStageV2::SourceOwner);
            assert_eq!(
                (f.point.table_tag, f.point.table_offset, f.point.file_offset),
                (None, None, None)
            );
            assert_eq!(trace.accepted, expected[..stop]);
        }
    }
    #[test]
    fn owned_source_native_preflight_precedes_owner_requests() {
        let limits = M4EffectiveResourceLimits::defaults_for(
            &typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits {
                max_font_bytes: 64,
                ..typaxis_core::ResourceLimits::default()
            })
            .unwrap(),
        );
        for (length, face) in [(12, 1), (65, 0)] {
            let bytes = vec![0; length];
            let native =
                admit_sfnt_cff1_v2(Arc::from(bytes.as_slice()), face, &limits).unwrap_err();
            let mut trace = Trace {
                stop: Some(0),
                ..Trace::default()
            };
            assert_eq!(
                admit_sfnt_cff1_v2_owned_with_admission_budget(bytes, face, &limits, &mut trace)
                    .unwrap()
                    .unwrap_err(),
                native
            );
            assert!(trace.accepted.is_empty());
        }
    }
    #[test]
    #[ignore = "requires unchanged original Harano through TYPAXIS_HARANO_FONT"]
    fn owned_source_original_harano_shares_admission_and_program_buffer() {
        let bytes = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
        let hash = typaxis_core::sha256(&bytes);
        let pointer = bytes.as_ptr();
        let limits = M4EffectiveResourceLimits::defaults_for(
            &typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
                .unwrap(),
        );
        let sliced = admit_sfnt_cff1_v2(Arc::from(bytes.as_slice()), 0, &limits).unwrap();
        let owned = admit_sfnt_cff1_v2_owned(bytes, 0, &limits).unwrap();
        assert_eq!(owned.source().as_ptr(), pointer);
        assert_eq!(owned.source_sha256(), hash);
        assert_eq!(owned.fingerprint(), sliced.fingerprint());
        let SourceBytesV2::Owned(p) = &owned.program().program.source else {
            panic!()
        };
        assert_eq!(p.as_ref().as_ptr(), pointer);
        assert_eq!(Arc::strong_count(p), 2);
        assert_eq!(
            owned.program().program.bytes(),
            sliced.program().program.bytes()
        );
        assert_eq!(owned.program().glyph_count(), 23060);
        assert_eq!(owned.program().font_dict_count(), 18);
        let mut a = CffProgramEvaluationSessionV2::new(&limits);
        let mut b = CffProgramEvaluationSessionV2::new(&limits);
        let mut state = typaxis_core::sha256(b"typaxis.cff-v2-outline-records/1");
        for gid in 0..owned.glyph_count() {
            let advance = owned.horizontal_metric(gid).unwrap().0;
            let x = a.evaluate(owned.program(), gid, advance).unwrap();
            let y = b.evaluate(sliced.program(), gid, advance).unwrap();
            assert!(x.commands().eq(y.commands()));
            assert_eq!(x.source_width_fixed(), y.source_width_fixed());
            let mut record = Vec::new();
            for command in x.commands() {
                let (symbol, values) = match command {
                    CffOutlineCommandV2::Move(x, y) => (b'M', vec![x, y]),
                    CffOutlineCommandV2::Line(x, y) => (b'L', vec![x, y]),
                    CffOutlineCommandV2::Cubic(a, b, c, d, e, f) => (b'C', vec![a, b, c, d, e, f]),
                    CffOutlineCommandV2::Close => (b'Z', vec![]),
                };
                record.push(symbol);
                for value in values {
                    record.extend(value.to_be_bytes());
                }
            }
            let mut next = state.to_vec();
            next.extend(gid.to_be_bytes());
            next.extend(advance.to_be_bytes());
            next.extend(typaxis_core::sha256(&record));
            state = typaxis_core::sha256(&next);
        }
        let hex = state.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(
            hex,
            "f3a7b806eb38a37c56ec76eac5b80dd21a1bfe1f17a71650c971b3c399bace38"
        );
        assert_eq!(a.operations_used(), b.operations_used());
        assert_eq!(a.outline_segments_used(), b.outline_segments_used());
        eprintln!(
            "owned source glyphs=23060 FDs=18 operations={} segments={} outline_sha256={hex}",
            a.operations_used(),
            a.outline_segments_used()
        );
    }
}
