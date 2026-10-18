use super::*;
#[path = "cff_jstf_cases.rs"]
mod cases;
struct Budget<const ON: bool> {
    stop: Option<usize>,
    attempts: usize,
    paid: Vec<u64>,
}
impl<const ON: bool> Budget<ON> {
    fn new(stop: Option<usize>) -> Self {
        Self {
            stop,
            attempts: 0,
            paid: Vec::new(),
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
    const OBSERVE_JSTF: bool = ON;
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
    fn work_jstf(&mut self, n: u64) -> std::result::Result<(), usize> {
        assert!(ON);
        assert!([STEP_WORK, READER_WORK, 1024 * STEP_WORK].contains(&n));
        let at = self.attempts;
        self.attempts += 1;
        if self.stop == Some(at) {
            return Err(at);
        }
        self.paid.push(n);
        Ok(())
    }
}
fn raw_lookup<const ON: bool>(
    c: &cases::Case,
    b: &mut Budget<ON>,
) -> std::result::Result<
    std::result::Result<bool, read_fonts::ReadError>,
    CffTableAdmissionBudgetFailureV2<usize>,
> {
    let mut s = State {
        budget: b,
        start: c.bytes.as_ptr() as usize,
        length: c.bytes.len(),
        base: Some(4096),
    };
    match intersection::lookup(&c.bytes, c.glyphs, &mut s) {
        Ok(v) => Ok(Ok(v)),
        Err(intersection::Failure::Native(e)) => Ok(Err(e)),
        Err(intersection::Failure::Budget(e)) => Err(e),
    }
}
fn run<const ON: bool>(
    kind: &str,
    c: &cases::Case,
    b: &mut Budget<ON>,
) -> std::result::Result<String, CffTableAdmissionBudgetFailureV2<usize>> {
    if kind == "lookup" {
        if ON {
            return raw_lookup(c, b).map(|r| format!("{r:?}"));
        }
        let mut glyphs = IntSet::empty();
        for g in 0..c.glyphs {
            glyphs.insert(GlyphId::new(g as u32));
        }
        Ok(format!(
            "{:?}",
            PositionLookup::read(FontData::new(&c.bytes))
                .and_then(|l| l.subtables())
                .and_then(|s| s.intersects(&glyphs))
        ))
    } else {
        local_optional::finish(validate(&c.bytes, c.glyphs, 8, 9, Some(4096), b))
            .map(|r| format!("{r:?}"))
    }
}
fn caught<const ON: bool>(kind: &str, c: &cases::Case, b: &mut Budget<ON>) -> String {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(kind, c, b).unwrap()))
        .unwrap_or_else(|_| "Panic".into())
}
#[test]
fn jstf_native_corpus_matches_saved_stream_and_ordinary_calls_no_hooks() {
    let mut stream = Vec::new();
    let (mut total, mut accepted, mut panics) = (0, 0, 0);
    for (kind, rows) in [
        ("lookup", cases::lookup_cases()),
        ("jstf", cases::table_cases()),
    ] {
        for c in rows {
            let mut off = Budget::<false>::new(Some(0));
            let expected = caught(kind, &c, &mut off);
            assert_eq!((off.attempts, off.paid.len()), (0, 0));
            let mut on = Budget::<true>::new(None);
            assert_eq!(caught(kind, &c, &mut on), expected, "{}", c.label);
            assert!(on.attempts > 0);
            accepted += usize::from(expected.starts_with("Ok("));
            panics += usize::from(expected == "Panic");
            total += 1;
            stream.extend(format!("{kind}\t{}\t{expected}\n", c.label).bytes());
        }
    }
    stream.extend(format!("total={total} accepted={accepted} panics={panics}\n").bytes());
    assert_eq!((total, accepted, panics), (36027, 10787, 32));
    assert_eq!(typaxis_core::sha256(&stream), SAVED_STREAM_SHA);
    eprintln!("JSTF saved native cases={total}, accepted={accepted}, existing panics={panics}, ordinary hooks=0");
}
const SAVED_STREAM_SHA: [u8; 32] = [
    61, 48, 20, 49, 119, 77, 230, 47, 146, 156, 110, 37, 51, 230, 168, 161, 98, 196, 170, 96, 18,
    31, 123, 193, 169, 196, 180, 159, 198, 241, 57, 151,
];
#[test]
fn jstf_callback_cuts_preserve_paid_prefix_and_authenticated_positions() {
    let (mut cuts, mut all_cases) = (0, 0);
    for (kind, rows) in [
        ("lookup", cases::lookup_cases()),
        ("jstf", cases::table_cases()),
    ] {
        for c in rows {
            let mut ready = Budget::<true>::new(None);
            let expected = caught(kind, &c, &mut ready);
            let all =
                !c.label.contains("-byte-") && !c.label.contains("-prefix-") && expected != "Panic";
            let count = if all {
                all_cases += 1;
                ready.paid.len()
            } else {
                1
            };
            for stop in 0..count {
                let mut b = Budget::<true>::new(Some(stop));
                let failure = run(kind, &c, &mut b).unwrap_err();
                assert_eq!(failure.error, stop);
                assert_eq!(b.attempts, stop + 1);
                assert_eq!(b.paid, ready.paid[..stop]);
                assert_eq!(failure.point.table_tag, Some(*b"JSTF"));
                assert!(failure
                    .point
                    .table_offset
                    .map_or(true, |at| at <= c.bytes.len() as u64));
                assert_eq!(
                    failure.point.file_offset,
                    failure.point.table_offset.map(|at| 4096 + at)
                );
                if matches!(failure.point.stage, Stage::JstfVisit | Stage::JstfSetInit) {
                    assert_eq!(failure.point.table_offset, None);
                }
                cuts += 1;
            }
        }
    }
    eprintln!("JSTF all-whole cases={all_cases}, sampled/all-whole callback cuts={cuts}");
}
#[test]
fn jstf_primitives_refuse_before_native_reads_and_unknown_offsets_stay_absent() {
    for primitive in 0..4 {
        let mut b = Budget::<true>::new(Some(0));
        let mut s = State {
            budget: &mut b,
            start: 0,
            length: 0,
            base: Some(usize::MAX),
        };
        let r = match primitive {
            0 => s
                .u16(&[], usize::MAX, Cff1Error::InvalidOptionalTable)
                .map(|_| ()),
            1 => s
                .u32(&[], usize::MAX, Cff1Error::InvalidOptionalTable)
                .map(|_| ()),
            2 => s.range(&[], usize::MAX, usize::MAX).map(|_| ()),
            _ => s.relative(&[], usize::MAX, 65535, false).map(|_| ()),
        };
        let e = local_optional::finish(r).unwrap_err();
        assert_eq!(e.point.table_offset, None);
        assert_eq!(e.point.file_offset, None);
        assert!(b.paid.is_empty());
    }
    let c = cases::Case {
        label: "empty".into(),
        bytes: Vec::new(),
        glyphs: 1,
    };
    let mut b = Budget::<true>::new(Some(0));
    assert_eq!(
        raw_lookup(&c, &mut b).unwrap_err().point.stage,
        Stage::JstfReader
    );
    assert!(b.paid.is_empty());
    eprintln!("JSTF four local primitives and borrowed reader stop before native failure");
}
#[test]
fn jstf_observed_budget_stops_locked_native_zero_class_cursor() {
    let (_, v) = cases::lookup_seeds()
        .into_iter()
        .find(|(n, _)| n == "pair2-1")
        .unwrap();
    let mut sub = v[8..].to_vec();
    let at = sub.len() as u16;
    sub[10..12].copy_from_slice(&at.to_be_bytes());
    sub.extend(cases::words(&[1, 0, 2, 0, 0]));
    let c = cases::Case {
        label: "native-constant-cursor".into(),
        bytes: cases::lookup(2, &sub),
        glyphs: 3,
    };
    for stop in [32, 128, 1024] {
        let mut b = Budget::<true>::new(Some(stop));
        let failure = raw_lookup(&c, &mut b).unwrap_err();
        assert_eq!(failure.error, stop);
        assert_eq!(failure.point.stage, Stage::JstfVisit);
        assert_eq!(b.paid.len(), stop);
    }
    eprintln!("JSTF locked native zero-class cursor stops at three finite-budget cutoffs; ordinary hanging walk not launched");
}
