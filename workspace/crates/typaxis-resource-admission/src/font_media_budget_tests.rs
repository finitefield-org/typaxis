use super::*;
use typaxis_font::CffSfntStageV2 as S;

fn sfnt(tags: &[[u8; 4]]) -> Vec<u8> {
    let mut bytes = vec![0; 12 + tags.len() * 16];
    bytes[..4].copy_from_slice(b"OTTO");
    bytes[4..6].copy_from_slice(&(tags.len() as u16).to_be_bytes());
    for (row, tag) in tags.iter().enumerate() {
        bytes[12 + row * 16..16 + row * 16].copy_from_slice(tag);
    }
    bytes
}
type MediaOutcome = Result<Result<AdmittedFontMediaKind, ResourceAdmissionError>, usize>;
fn result(
    bytes: &[u8],
    face: u32,
    stop: Option<usize>,
) -> (Vec<(CffSfntWorkPointV2, u64)>, MediaOutcome) {
    let mut trace = Vec::new();
    let mut work = |point, amount| {
        if stop == Some(trace.len()) {
            return Err(trace.len());
        }
        trace.push((point, amount));
        Ok(())
    };
    let outcome = match attest_walk(bytes, face, &mut work) {
        Ok(value) => Ok(Ok(value)),
        Err(WalkError::Budget(error)) => Err(error),
        Err(WalkError::Native(_)) => {
            font_diagnostic::diagnose_font_container_with_work(bytes, face, &mut work)
                .map(|f| Err(ResourceAdmissionError::FontContainerDetailed(f)))
        }
    };
    (trace, outcome)
}
#[test]
fn media_walk_refuses_before_each_header_record_and_error_diagnostic() {
    let good = sfnt(&[*b"head", *b"CFF ", *b"name"]);
    let mut collection = vec![0; 24];
    collection[..4].copy_from_slice(b"ttcf");
    collection[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    collection[8..12].copy_from_slice(&3u32.to_be_bytes());
    for row in 0..3 {
        collection[12 + row * 4..16 + row * 4].copy_from_slice(&24u32.to_be_bytes());
    }
    collection.extend(sfnt(&[*b"CFF2"]));
    let mut cases = vec![
        (good.clone(), 0),
        (good.clone(), 1),
        (sfnt(&[*b"CFF2"]), 0),
        (sfnt(&[*b"CFF ", *b"glyf"]), 0),
        (collection.clone(), 0),
        (collection, 3),
    ];
    cases.extend((0..good.len()).map(|n| (good[..n].to_vec(), 0)));
    let mut cuts = 0;
    for (bytes, face) in cases {
        let (full, native) = result(&bytes, face, None);
        assert_eq!(
            format!("{:?}", native.as_ref().unwrap()),
            format!("{:?}", declared_native(&bytes, face))
        );
        assert_eq!(full[0], (point(S::DeclaredMediaHeader, 0), 64));
        for stop in 0..full.len() {
            let (short, error) = result(&bytes, face, Some(stop));
            assert_eq!(error.unwrap_err(), stop);
            assert_eq!(short, full[..stop]);
            assert_eq!(
                format!("{:?}", result(&bytes, face, None).1),
                format!("{native:?}")
            );
            cuts += 1;
        }
    }
    let (trace, accepted) = result(&good, 0, None);
    assert!(matches!(accepted, Ok(Ok(AdmittedFontMediaKind::SfntCff1))));
    assert_eq!(
        trace
            .iter()
            .map(|(p, _)| p.file_offset.unwrap())
            .collect::<Vec<_>>(),
        [0, 12, 28, 44]
    );
    assert!(trace.iter().all(|(_, amount)| *amount == 64));
    eprintln!("resource media callback refusals={cuts} exact_prefix=true retry=true");
}
#[test]
fn media_walk_handles_maximum_directory_and_unobserved_hooks_stay_closed() {
    let mut tags = vec![*b"head"; u16::MAX as usize];
    *tags.last_mut().unwrap() = *b"CFF ";
    let bytes = sfnt(&tags);
    let (trace, result) = result(&bytes, 0, None);
    assert_eq!(trace.len(), 1 + u16::MAX as usize);
    assert!(result.unwrap().is_ok());
    assert_eq!(
        trace.last().unwrap().0.file_offset,
        Some(12 + (u16::MAX as u64 - 1) * 16)
    );
    // Every ordinary entry point shares this grammar and bypasses the budget.
    assert_eq!(
        attest_native(&bytes, 0).unwrap(),
        AdmittedFontMediaKind::SfntCff1
    );
}

#[derive(Default)]
struct Budget<const MEDIA: bool, const OBSERVED: bool> {
    calls: usize,
    stop: Option<usize>,
}
impl<const M: bool, const O: bool> typaxis_font::CffBaseCmapBudgetV2 for Budget<M, O> {
    type Error = usize;
    const OBSERVED: bool = O;
    fn reserve(
        &mut self,
        _: typaxis_font::CffBaseCmapStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        Ok(())
    }
    fn work(&mut self, _: u64) -> Result<(), usize> {
        Ok(())
    }
}
impl<const M: bool, const O: bool> typaxis_font::CffCmapBudgetV2 for Budget<M, O> {
    const OBSERVE_VARIATIONS: bool = false;
    fn reserve_variation(
        &mut self,
        _: typaxis_font::CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        Ok(())
    }
}
impl<const M: bool, const O: bool> CffAdmissionBudgetV2 for Budget<M, O> {
    const OBSERVE_SFNT: bool = false;
    const OBSERVE_RESOURCE_MEDIA: bool = M;
    fn reserve_sfnt(
        &mut self,
        _: typaxis_font::CffSfntStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        Ok(())
    }
    fn work_sfnt(&mut self, _: u64) -> Result<(), usize> {
        if self.stop == Some(self.calls) {
            return Err(self.calls);
        }
        self.calls += 1;
        Ok(())
    }
}
#[test]
fn media_observation_requires_both_opt_in_and_observed_source() {
    for bytes in [sfnt(&[*b"CFF "]), sfnt(&[*b"CFF2"])] {
        let mut legacy = Budget::<false, true>::default();
        let mut unobserved = Budget::<true, false>::default();
        assert_eq!(
            format!(
                "{:?}",
                declared_with_budget(&bytes, 0, &mut legacy).unwrap()
            ),
            format!("{:?}", declared_native(&bytes, 0))
        );
        assert_eq!(
            format!(
                "{:?}",
                declared_with_budget(&bytes, 0, &mut unobserved).unwrap()
            ),
            format!("{:?}", declared_native(&bytes, 0))
        );
        assert_eq!((legacy.calls, unobserved.calls), (0, 0));
        let mut refused = Budget::<true, true> {
            stop: Some(0),
            ..Default::default()
        };
        let CffAdmissionBudgetFailureV2::Sfnt(f) =
            declared_with_budget(&bytes, 0, &mut refused).unwrap_err()
        else {
            panic!("wrong failure")
        };
        assert_eq!(f.error, 0);
        assert_eq!(f.point, point(S::DeclaredMediaHeader, 0));
    }
}
