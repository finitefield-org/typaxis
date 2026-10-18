use super::*;
#[path = "cff_font_tables_cases.rs"]
pub(crate) mod cases;
#[derive(Default)]
struct Trace {
    requests: Vec<u64>,
    paid: u64,
    stop: Option<usize>,
}
impl CffBaseCmapBudgetV2 for Trace {
    type Error = usize;
    fn reserve(&mut self, _: CffBaseCmapStorageV2, _: usize, _: usize) -> Result<(), usize> {
        panic!("table index/headers/vertical validation must not allocate")
    }
    fn work(&mut self, _: u64) -> Result<(), usize> {
        panic!("table work has its own scope")
    }
}
impl CffCmapBudgetV2 for Trace {
    fn reserve_variation(
        &mut self,
        _: CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        panic!()
    }
}
impl CffAdmissionBudgetV2 for Trace {
    const OBSERVE_TABLES: bool = true;
    fn reserve_sfnt(&mut self, _: CffSfntStorageV2, _: usize, _: usize) -> Result<(), usize> {
        panic!()
    }
    fn work_sfnt(&mut self, _: u64) -> Result<(), usize> {
        panic!()
    }
    fn work_tables(&mut self, n: u64) -> Result<(), usize> {
        let at = self.requests.len();
        assert!(self.stop.map_or(true, |stop| at <= stop));
        assert!((1..=1024).contains(&n));
        self.requests.push(n);
        if self.stop == Some(at) {
            return Err(91);
        }
        self.paid += n;
        Ok(())
    }
}
fn cuts<T: std::fmt::Debug>(
    full: &Trace,
    mut run: impl FnMut(&mut Trace) -> Result<T, CffTableAdmissionBudgetFailureV2<usize>>,
) -> usize {
    for stop in 0..full.requests.len() {
        let mut short = Trace {
            stop: Some(stop),
            ..Trace::default()
        };
        let f = run(&mut short).unwrap_err();
        assert_eq!(f.error, 91);
        assert_eq!(short.requests, full.requests[..=stop]);
        assert_eq!(short.paid, full.requests[..stop].iter().sum::<u64>());
    }
    full.requests.len()
}
#[test]
fn font_tables_stack_index_matches_native_bytes_and_stops_before_every_bounded_step() {
    let limits = M4ResourceLimits::default();
    let (mut count, mut refusals) = (0, 0);
    for (_, bytes) in cases::cases() {
        let Ok(records) =
            preflight_sfnt_with_vertical(&bytes, limits, &mut FontFailureContext::new(0), true)
        else {
            continue;
        };
        let expected = table_map(&bytes, &records).unwrap();
        let mut full = Trace::default();
        let actual = Tables::new(&bytes, &records, &mut full).unwrap().unwrap();
        assert!(actual.ordinary.is_none());
        for tag in TAGS.iter().chain([*b"xxxx"].iter()) {
            assert_eq!(
                actual.get(tag).map(|t| t.bytes),
                expected.get(tag).map(|t| t.bytes)
            );
        }
        refusals += cuts(&full, |t| Tables::new(&bytes, &records, t));
        count += 1;
    }
    assert_eq!(
        TAGS.len(),
        REQUIRED_TABLES.len() + OPTIONAL_TABLES.len() + 3
    );
    for tag in REQUIRED_TABLES
        .iter()
        .chain(OPTIONAL_TABLES.iter())
        .chain([*b"VORG", *b"vhea", *b"vmtx"].iter())
    {
        assert!(TAGS.binary_search(tag).is_ok());
    }
    eprintln!(
        "stack table index native cases={count}, all-request cuts={refusals}, heap reserve hooks=0"
    );
}
#[test]
fn font_tables_fixed_decoder_refusal_precedes_native_call_and_ordinary_has_zero_hooks() {
    for tag in [*b"head", *b"maxp", *b"hhea", *b"OS/2", *b"post"] {
        let called = std::cell::Cell::new(0);
        let mut short = Trace {
            stop: Some(0),
            ..Trace::default()
        };
        let f = fixed(&mut short, tag, || called.set(called.get() + 1)).unwrap_err();
        assert_eq!(called.get(), 0);
        assert_eq!(f.point.stage, Stage::FixedDecoder);
        assert_eq!(f.point.table_tag, Some(tag));
        assert_eq!(f.point.file_offset, None);
        assert_eq!(f.point.table_offset, None);
        let mut full = Trace::default();
        assert_eq!(
            fixed(&mut full, tag, || {
                called.set(called.get() + 1);
                17
            })
            .unwrap(),
            17
        );
        assert_eq!(called.get(), 1);
        assert_eq!(full.requests, [64]);
        assert_eq!(
            fixed(
                &mut super::super::v2::admission_budget::UnobservedAdmission,
                tag,
                || 19
            )
            .unwrap(),
            19
        );
    }
    assert!(std::mem::size_of::<CffAdmissionBudgetFailureV2<usize>>() < 128);
}
type VerticalFacts = (u16, u16, Vec<GlyphVertical>);
type GlyphVertical = (Option<i16>, Option<(u16, i16)>);
fn vertical_facts(v: super::super::v2::vertical::CffVerticalMetricsV2<'_>) -> VerticalFacts {
    (
        v.long_metric_count(),
        v.origin_record_count(),
        (0..5)
            .map(|gid| (v.vertical_origin(gid), v.glyph_metric(gid)))
            .collect(),
    )
}
#[test]
fn font_tables_vertical_shared_grammar_preserves_all_fields_and_every_callback_prefix() {
    let (mut count, mut refusals) = (0, 0);
    for (g, v, h, m) in cases::vertical_cases() {
        let (v, h, m) = (v.as_deref(), h.as_deref(), m.as_deref());
        let expected = validate_cff_vertical_metrics_v2(g, v, h, m).map(vertical_facts);
        let mut full = Trace::default();
        let actual = super::super::v2::vertical::with_budget(g, v, h, m, &mut full)
            .unwrap()
            .map(vertical_facts);
        assert_eq!(actual, expected);
        refusals += cuts(&full, |t| {
            super::super::v2::vertical::with_budget(g, v, h, m, t)
        });
        count += 1;
    }
    eprintln!("vertical exact native cases={count}, all-request refusal cuts={refusals}, heap reserve hooks=0");
}
#[test]
fn font_tables_optional_dispatch_uses_the_shared_lookup_interface() {
    struct Counting<'a> {
        map: BTreeMap<[u8; 4], TableRef<'a>>,
        calls: std::cell::Cell<usize>,
    }
    impl<'a> Lookup<'a> for Counting<'a> {
        fn get(&self, tag: &[u8; 4]) -> Option<&TableRef<'a>> {
            self.calls.set(self.calls.get() + 1);
            self.map.get(tag)
        }
    }
    let bytes = cases::standard();
    let records = preflight_sfnt_with_vertical(
        &bytes,
        M4ResourceLimits::default(),
        &mut FontFailureContext::new(0),
        true,
    )
    .unwrap();
    let tables = Counting {
        map: table_map(&bytes, &records).unwrap(),
        calls: std::cell::Cell::new(0),
    };
    validate_optional_tables(
        &bytes,
        &tables,
        4,
        &records,
        &mut FontFailureContext::new(0),
    )
    .unwrap();
    assert!(tables.calls.get() > 0 && tables.calls.get() <= 24);
}
