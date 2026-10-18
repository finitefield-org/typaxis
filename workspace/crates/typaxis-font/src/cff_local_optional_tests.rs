use super::*;
#[path = "cff_local_optional_cases.rs"]
mod cases;
struct Budget<const ON: bool> {
    stop: Option<usize>,
    attempts: usize,
    accepted: usize,
    work: u64,
}
impl<const ON: bool> Budget<ON> {
    fn new(stop: Option<usize>) -> Self {
        Self {
            stop,
            attempts: 0,
            accepted: 0,
            work: 0,
        }
    }
}
impl<const ON: bool> CffBaseCmapBudgetV2 for Budget<ON> {
    type Error = usize;
    const OBSERVED: bool = false;
    fn reserve(
        &mut self,
        _: CffBaseCmapStorageV2,
        _: usize,
        _: usize,
    ) -> std::result::Result<(), usize> {
        panic!()
    }
    fn work(&mut self, _: u64) -> std::result::Result<(), usize> {
        panic!()
    }
}
impl<const ON: bool> CffCmapBudgetV2 for Budget<ON> {
    const OBSERVE_VARIATIONS: bool = false;
    fn reserve_variation(
        &mut self,
        _: CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> std::result::Result<(), usize> {
        panic!()
    }
}
impl<const ON: bool> CffAdmissionBudgetV2 for Budget<ON> {
    const OBSERVE_SFNT: bool = false;
    const OBSERVE_OPTIONAL_LOCAL: bool = ON;
    fn reserve_sfnt(
        &mut self,
        _: CffSfntStorageV2,
        _: usize,
        _: usize,
    ) -> std::result::Result<(), usize> {
        panic!()
    }
    fn work_sfnt(&mut self, _: u64) -> std::result::Result<(), usize> {
        panic!()
    }
    fn work_optional_local(&mut self, amount: u64) -> std::result::Result<(), usize> {
        assert!(ON);
        assert_eq!(amount, STEP_WORK);
        let at = self.attempts;
        self.attempts += 1;
        if self.stop == Some(at) {
            return Err(at);
        }
        self.accepted += 1;
        self.work += amount;
        Ok(())
    }
}
#[test]
fn local_optional_native_corpus_matches_saved_prechange_stream_and_ordinary_calls_no_hooks() {
    let mut stream = Vec::new();
    let mut accepted = 0;
    let rows = cases::cases();
    assert_eq!(rows.len(), 3539);
    for c in &rows {
        let mut off = Budget::<false>::new(Some(0));
        let ordinary = finish(validate(&c.bytes, c.glyphs, c.tag, None, &mut off)).unwrap();
        assert_eq!((off.attempts, off.work), (0, 0));
        let mut on = Budget::<true>::new(None);
        let observed = finish(validate(&c.bytes, c.glyphs, c.tag, None, &mut on)).unwrap();
        assert_eq!(observed, ordinary, "{}", c.label);
        assert!(on.accepted > 0);
        if ordinary.is_ok() {
            accepted += 1;
        }
        stream.extend(format!("{}\t{ordinary:?}\n", c.label).bytes());
    }
    stream.extend(format!("total={} accepted={accepted}\n", rows.len()).bytes());
    assert_eq!(accepted, 883);
    assert_eq!(
        typaxis_core::sha256(&stream),
        [
            160, 108, 152, 29, 196, 135, 61, 155, 188, 164, 149, 183, 210, 73, 189, 224, 5, 6, 84,
            237, 92, 17, 219, 242, 229, 214, 186, 110, 230, 230, 147, 199,
        ]
    );
    eprintln!(
        "local optional saved native cases={}, accepted={accepted}, ordinary hooks=0",
        rows.len()
    );
}
#[test]
fn local_optional_every_small_callback_prefix_stops_with_exact_tag_position_and_paid_work() {
    let (mut cutoffs, mut whole_cases) = (0, 0);
    for c in cases::cases() {
        let mut ready = Budget::<true>::new(None);
        let expected = finish(validate(&c.bytes, c.glyphs, c.tag, Some(4096), &mut ready)).unwrap();
        let all = !c.label.contains("-byte-");
        if all {
            whole_cases += 1;
        }
        let cuts = if all { ready.accepted } else { 1 };
        for stop in 0..cuts {
            let mut b = Budget::<true>::new(Some(stop));
            let failure =
                finish(validate(&c.bytes, c.glyphs, c.tag, Some(4096), &mut b)).unwrap_err();
            assert_eq!(failure.error, stop);
            assert_eq!(
                (b.attempts, b.accepted, b.work),
                (stop + 1, stop, stop as u64 * STEP_WORK)
            );
            assert_eq!(failure.point.table_tag, Some(c.tag));
            assert!(failure
                .point
                .table_offset
                .map_or(true, |n| n <= c.bytes.len() as u64));
            assert_eq!(
                failure.point.file_offset,
                failure.point.table_offset.map(|n| 4096 + n)
            );
            cutoffs += 1;
        }
        let mut enough = Budget::<true>::new(Some(ready.accepted));
        assert_eq!(
            finish(validate(&c.bytes, c.glyphs, c.tag, Some(4096), &mut enough)).unwrap(),
            expected
        );
        assert_eq!(enough.work, ready.work);
    }
    eprintln!(
        "local optional all-small cases={whole_cases}, sampled/all-small refusal cuts={cutoffs}"
    );
}
#[test]
fn local_optional_primitives_refuse_before_native_range_errors_and_keep_unknown_positions_absent() {
    let mut checked = 0;
    for which in 0..5 {
        let mut budget = Budget::<true>::new(Some(0));
        let mut state = State {
            budget: &mut budget,
            tag: Some(*b"BASE"),
            length: 0,
            file_base: Some(4096),
        };
        let failure = match which {
            0 => state
                .u16(&[], usize::MAX, Cff1Error::InvalidOptionalTable)
                .unwrap_err(),
            1 => state
                .u32(&[], usize::MAX, Cff1Error::InvalidOptionalTable)
                .unwrap_err(),
            2 => state.range(&[], usize::MAX, usize::MAX).unwrap_err(),
            3 => state
                .relative(&[], usize::MAX, u16::MAX, false)
                .unwrap_err(),
            _ => state
                .relative32(&[], usize::MAX, u32::MAX, false)
                .unwrap_err(),
        };
        let Failure::Budget(f) = failure else {
            panic!("native operation ran after refusal")
        };
        assert_eq!(f.error, 0);
        assert_eq!(f.point.table_offset, None);
        assert_eq!(f.point.file_offset, None);
        assert_eq!(budget.work, 0);
        checked += 1;
    }
    let mut budget = Budget::<true>::new(Some(0));
    let mut state = State {
        budget: &mut budget,
        tag: Some(*b"GDEF"),
        length: 2,
        file_base: Some(usize::MAX),
    };
    let Failure::Budget(f) = state
        .u16(&[0; 2], 1, Cff1Error::InvalidOptionalTable)
        .unwrap_err()
    else {
        panic!()
    };
    assert_eq!(f.point.table_offset, Some(1));
    assert_eq!(f.point.file_offset, None);
    eprintln!(
        "local optional refused primitive boundaries={checked}, absolute-offset overflow absent"
    );
}
