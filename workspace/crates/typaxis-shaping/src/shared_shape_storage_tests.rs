use super::*;
use typaxis_core::{TextBufferId, Utf8ByteOffset};

const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/profiles/production-book-1/combined/job/body.ttf"
));
mod multiple_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor/typaxis-harfrust/test-support/multiple_substitution_font.rs"
    ));
}
pub(crate) fn glyph_growth_font() -> Vec<u8> {
    let runs = shape_equation_number_runs(input(FONT, "R")).unwrap();
    assert_eq!(runs.1.len(), 1);
    assert_eq!(runs.1[0].glyphs.len(), 1);
    let glyph = u16::try_from(runs.1[0].glyphs[0].original_gid.get()).unwrap();
    multiple_fixture::multiple_substitution_font(FONT, glyph, 128)
}
#[path = "../../../vendor/typaxis-harfrust/test-support/aat_font.rs"]
mod aat_fixture;
pub(crate) fn aat_font() -> Vec<u8> {
    multiple_fixture::with_replaced_tables(FONT, &aat_fixture::aat_tables(3, false))
}
#[allow(dead_code)]
#[path = "../../../vendor/typaxis-harfrust/test-support/contextual_font.rs"]
mod context_fixture;
pub(crate) fn context_font() -> Vec<u8> {
    multiple_fixture::with_replaced_tables(
        FONT,
        &context_fixture::contextual_tables(&context_fixture::Context::sequence(
            3, false, &[18; 5], 8,
        )),
    )
}
#[path = "../../../vendor/typaxis-harfrust/test-support/indic_font.rs"]
mod indic_fixture;
pub(crate) fn indic_font() -> Vec<u8> {
    multiple_fixture::with_replaced_tables(FONT, &indic_fixture::indic_tables(FONT, false))
}
pub(crate) fn normalization_font() -> Vec<u8> {
    multiple_fixture::with_replaced_tables(
        FONT,
        &indic_fixture::script_tables(FONT, 0x0300..=0x036f, &[*b"latn"]),
    )
}
pub(crate) fn myanmar_font() -> Vec<u8> {
    multiple_fixture::with_replaced_tables(
        FONT,
        &indic_fixture::script_tables(FONT, 0x1000..=0x109f, &[*b"mym2"]),
    )
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Request {
    site: Site,
    count: usize,
    element_bytes: usize,
    records: u64,
    spool: u64,
    work: u64,
}
#[derive(Default)]
struct Trace {
    requests: Vec<Request>,
    fail_at: Option<usize>,
    records: u64,
    spool: u64,
    work: u64,
}
impl Storage for Trace {
    fn reserve<T>(&mut self, site: Site, count: usize) -> Result<(), LinkedShaperError> {
        self.spool += (count * std::mem::size_of::<T>()) as u64;
        self.work += count as u64;
        self.requests.push(Request {
            site,
            count,
            element_bytes: std::mem::size_of::<T>(),
            records: self.records,
            spool: self.spool,
            work: self.work,
        });
        if self.fail_at == Some(self.requests.len() - 1) {
            return Err(LinkedShaperError::AllocationFailure);
        }
        Ok(())
    }
    fn records(&mut self, count: u64) -> Result<(), LinkedShaperError> {
        self.records += count;
        Ok(())
    }
    fn work(&mut self, count: u64) -> Result<(), LinkedShaperError> {
        self.work += count;
        Ok(())
    }
}
pub(crate) fn input<'a>(font: &'a [u8], text: &'a str) -> EquationNumberRunInput<'a> {
    let face = harfrust::FontRef::from_index(font, 0).unwrap();
    EquationNumberRunInput {
        exact_text: text,
        text_span: TextSpan::new(
            TextBufferId::new(42),
            Utf8ByteOffset::new(100),
            Utf8ByteOffset::new(100 + text.len() as u32),
        )
        .unwrap(),
        font_bytes: font,
        face_index: 0,
        units_per_em: face.head().unwrap().units_per_em(),
        glyph_count: u32::from(face.maxp().unwrap().num_glyphs()),
        font_instance: FontInstanceId::new(1),
        font_size: PositiveLength::new(Length::from_raw(12 * 65536).unwrap()).unwrap(),
        language: Some("ja"),
        maximum_records: 65_536,
        total_records: Some(100_000),
    }
}

#[test]
fn shape_output_borrowed_clusters_keep_error_order_and_refuse_every_visit() {
    struct Visits {
        calls: usize,
        stop: Option<usize>,
    }
    impl Storage for Visits {
        fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
            panic!("borrowed validation must not allocate")
        }
        fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
            panic!("borrowed validation must not retain output")
        }
        fn work(&mut self, n: u64) -> Result<(), LinkedShaperError> {
            assert_eq!(n, 1);
            let at = self.calls;
            self.calls += 1;
            if self.stop == Some(at) {
                Err(LinkedShaperError::AllocationFailure)
            } else {
                Ok(())
            }
        }
    }
    type ClusterCase<'a> = (&'a str, &'a [u32], bool, Result<u32, LinkedShaperError>);
    let cases: &[ClusterCase<'_>] = &[
        ("abc", &[0, 0, 1, 2], false, Ok(3)),
        ("abc", &[2, 1, 1, 0], true, Ok(3)),
        ("日a", &[0, 3], false, Ok(2)),
        (
            "éa",
            &[0, 1],
            false,
            Err(LinkedShaperError::InvalidBackendCluster),
        ),
        (
            "abc",
            &[0, 3],
            false,
            Err(LinkedShaperError::InvalidBackendCluster),
        ),
        (
            "abc",
            &[0, 2, 1],
            false,
            Err(LinkedShaperError::NonMonotoneBackendClusters),
        ),
        (
            "abc",
            &[2, 0, 1],
            true,
            Err(LinkedShaperError::NonMonotoneBackendClusters),
        ),
        (
            "abc",
            &[1, 2],
            false,
            Err(LinkedShaperError::InvalidBackendCluster),
        ),
        (
            "abc",
            &[2, 1],
            true,
            Err(LinkedShaperError::InvalidBackendCluster),
        ),
    ];
    let mut cutoffs = 0;
    for &(text, clusters, rtl, expected) in cases {
        let infos = clusters
            .iter()
            .map(|&cluster| {
                let mut info = harfrust::GlyphInfo::default();
                info.cluster = cluster;
                info
            })
            .collect::<Vec<_>>();
        let level = if rtl {
            BidiLevel::new(1).unwrap()
        } else {
            BidiLevel::LTR
        };
        let mut full = Visits {
            calls: 0,
            stop: None,
        };
        assert_eq!(
            validate_backend_clusters_with_storage(&infos, text, level, &mut full),
            expected
        );
        for stop in 0..full.calls {
            let mut short = Visits {
                calls: 0,
                stop: Some(stop),
            };
            assert_eq!(
                validate_backend_clusters_with_storage(&infos, text, level, &mut short),
                Err(LinkedShaperError::AllocationFailure)
            );
            assert_eq!(short.calls, stop + 1);
            cutoffs += 1;
        }
    }
    eprintln!(
        "borrowed-output cluster cases={}, refusal cutoffs={cutoffs}",
        cases.len()
    );
}

#[test]
fn shape_output_real_number_all_work_refusals_preserve_accepted_request_prefix() {
    #[derive(Default)]
    struct Fees {
        events: Vec<[u64; 3]>,
        cells: [u64; 3],
        work_calls: usize,
        stop: Option<usize>,
        stopped: bool,
    }
    impl Fees {
        fn accept(&mut self, fee: [u64; 3]) {
            assert!(!self.stopped);
            self.events.push(fee);
            for (total, delta) in self.cells.iter_mut().zip(fee) {
                *total += delta;
            }
        }
    }
    impl Storage for Fees {
        fn reserve<T>(&mut self, _: Site, count: usize) -> Result<(), LinkedShaperError> {
            self.accept([0, (count * std::mem::size_of::<T>()) as u64, count as u64]);
            Ok(())
        }
        fn records(&mut self, n: u64) -> Result<(), LinkedShaperError> {
            self.accept([n, 0, 0]);
            Ok(())
        }
        fn work(&mut self, n: u64) -> Result<(), LinkedShaperError> {
            assert!(!self.stopped);
            let at = self.work_calls;
            self.work_calls += 1;
            if self.stop == Some(at) {
                self.stopped = true;
                return Err(LinkedShaperError::AllocationFailure);
            }
            self.accept([0, 0, n]);
            Ok(())
        }
    }
    let ordinary = shape_equation_number_runs(input(FONT, "Result")).unwrap();
    let mut full = Fees::default();
    assert_eq!(
        shape_equation_number_runs_with_storage(input(FONT, "Result"), &mut full).unwrap(),
        ordinary
    );
    for stop in 0..full.work_calls {
        let mut short = Fees {
            stop: Some(stop),
            ..Fees::default()
        };
        assert!(
            shape_equation_number_runs_with_storage(input(FONT, "Result"), &mut short).is_err()
        );
        assert!(short.stopped);
        assert_eq!(short.work_calls, stop + 1);
        assert_eq!(short.events, &full.events[..short.events.len()]);
        let mut expected = [0; 3];
        for fee in &short.events {
            for (total, delta) in expected.iter_mut().zip(fee) {
                *total += delta;
            }
        }
        assert_eq!(short.cells, expected);
    }
    eprintln!(
        "real number work refusal cutoffs={}, final={:?}",
        full.work_calls, full.cells
    );
}
#[test]
fn shared_shape_storage_real_number_runs_match_unobserved_for_scripts_and_nested_bidi() {
    for text in [
        "(1)",
        "(123) abc",
        "日本語(1)",
        "abc א(1)",
        "a\u{202b}א(2)\u{202c}z",
        "العربية (123)",
        "हिन्दी (123)",
        "한글 (123)",
        "កម្ពុជា (123)",
        "ᬩ (123)",
    ] {
        let legacy = shape_equation_number_runs(input(FONT, text)).unwrap();
        let mut trace = Trace::default();
        let observed =
            shape_equation_number_runs_with_storage(input(FONT, text), &mut trace).unwrap();
        assert_eq!(observed, legacy);
        let sites = trace.requests.iter().map(|r| r.site).collect::<Vec<_>>();
        for site in [
            Site::Graphemes,
            Site::ItemizedRuns,
            Site::NumberRuns,
            Site::VisualClusters,
            Site::LogicalClusters,
            Site::Glyphs,
            Site::Clusters,
            Site::Utf8Boundaries,
            Site::ValidationRanges,
            Site::Harfrust(harfrust::scratch::Site::UnicodeInfos),
            Site::Harfrust(harfrust::scratch::Site::UnicodePositions),
            Site::Harfrust(harfrust::scratch::Site::LanguageString),
            Site::Harfrust(harfrust::scratch::Site::PlanFeatureInfos),
            Site::Harfrust(harfrust::scratch::Site::PlanGsubPauses),
            Site::Harfrust(harfrust::scratch::Site::PlanGposPauses),
            Site::Harfrust(harfrust::scratch::Site::PlanFeatures),
            Site::Harfrust(harfrust::scratch::Site::PlanGsubStages),
            Site::Harfrust(harfrust::scratch::Site::PlanGposStages),
        ] {
            assert!(sites.contains(&site), "{text:?}: {site:?}");
        }
        let retained = observed
            .1
            .iter()
            .map(|r| 1 + r.glyphs.len() as u64 + r.clusters.len() as u64)
            .sum::<u64>();
        assert_eq!(trace.records, retained);
        assert_eq!(
            trace.spool,
            trace
                .requests
                .iter()
                .map(|r| (r.count * r.element_bytes) as u64)
                .sum::<u64>()
        );
        assert!(trace.work > trace.records);
    }
}

#[test]
fn shared_shape_font_preflight_pays_hmtx_dependency_before_metadata_or_allocation() {
    struct Preflight {
        maximum: u64,
        paid: u64,
        requests: Vec<u64>,
    }
    impl Storage for Preflight {
        fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
            panic!("metadata preflight must precede allocation");
        }
        fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
            panic!("metadata preflight must precede retained output");
        }
        fn work(&mut self, amount: u64) -> Result<(), LinkedShaperError> {
            self.requests.push(amount);
            let next = self.paid + amount;
            if next > self.maximum {
                return Err(LinkedShaperError::AllocationFailure);
            }
            self.paid = next;
            Ok(())
        }
    }
    let request = input(FONT, "Result");
    let n = u64::from(
        harfrust::FontRef::from_index(FONT, 0)
            .unwrap()
            .table_directory()
            .num_tables(),
    );
    let allowance = 7 * n + 48;
    for (maximum, paid, expected) in [
        (0, 0, LinkedShaperError::AllocationFailure),
        (1, 1, LinkedShaperError::AllocationFailure),
        (allowance, 1, LinkedShaperError::AllocationFailure),
        (
            allowance + 1,
            allowance + 1,
            LinkedShaperError::FontMetadataMismatch,
        ),
    ] {
        let mut storage = Preflight {
            maximum,
            paid: 0,
            requests: Vec::new(),
        };
        let actual = shape_linked_with_storage(
            LinkedBackendInput {
                run_id: GlyphRunId::new(0),
                font: request.font_instance,
                source: ShapeSourceSpan::Parsed(request.text_span),
                utf8: request.exact_text,
                font_bytes: request.font_bytes,
                face_index: request.face_index,
                admitted_units_per_em: request.units_per_em + 1,
                admitted_glyph_count: request.glyph_count,
                font_size: request.font_size.get(),
                bidi_level: BidiLevel::LTR,
                script: OpenTypeTag::new(*b"Latn").unwrap(),
                language: Some("ja"),
                pre_context: None,
                post_context: None,
                max_output_records: request.maximum_records,
            },
            &mut ShapeOutputBudget::new(request.maximum_records),
            |_, _| panic!("metadata preflight must precede output callback"),
            &mut storage,
        )
        .unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(storage.paid, paid);
        assert_eq!(
            storage.requests,
            if maximum == 0 {
                vec![1]
            } else {
                vec![1, allowance]
            }
        );
    }
}
#[test]
fn shared_shape_storage_every_real_reservation_failure_keeps_paid_prefix() {
    let text = "abc א(123) 日本語 العربية हिन्दी 한글 កម្ពុជា ᬩ";
    let mut full = Trace::default();
    shape_equation_number_runs_with_storage(input(FONT, text), &mut full).unwrap();
    assert!(full
        .requests
        .iter()
        .any(|request| request.site == Site::Harfrust(harfrust::scratch::Site::PlanData)));
    assert!(full.requests.len() > 9);
    for index in 0..full.requests.len() {
        let mut trace = Trace {
            fail_at: Some(index),
            ..Trace::default()
        };
        let error =
            shape_equation_number_runs_with_storage(input(FONT, text), &mut trace).unwrap_err();
        assert!(
            matches!(
                error,
                StagingEquationNumberShapeError::AllocationFailure
                    | StagingEquationNumberShapeError::Backend(
                        LinkedShaperError::AllocationFailure
                    )
            ),
            "site={:?}: {error:?}",
            full.requests[index].site
        );
        assert_eq!(trace.requests, full.requests[..=index]);
        let paid = &full.requests[index];
        assert_eq!(
            [trace.records, trace.spool, trace.work],
            [paid.records, paid.spool, paid.work]
        );
    }
}

#[test]
fn shared_shape_storage_gsub_growth_matches_legacy_and_refuses_each_actual_buffer_reservation() {
    let font = glyph_growth_font();
    let legacy = shape_equation_number_runs(input(&font, "RRR")).unwrap();
    assert_eq!(legacy.1[0].glyphs.len(), 384);
    let mut full = Trace::default();
    let actual = shape_equation_number_runs_with_storage(input(&font, "RRR"), &mut full).unwrap();
    assert_eq!(actual, legacy);
    let mut faults = 0;
    for (index, request) in full.requests.iter().enumerate() {
        if !matches!(
            request.site,
            Site::Harfrust(
                harfrust::scratch::Site::GlyphInfos | harfrust::scratch::Site::GlyphPositions
            )
        ) {
            continue;
        }
        let mut trace = Trace {
            fail_at: Some(index),
            ..Trace::default()
        };
        assert!(matches!(
            shape_equation_number_runs_with_storage(input(&font, "RRR"), &mut trace),
            Err(StagingEquationNumberShapeError::Backend(
                LinkedShaperError::AllocationFailure
            ))
        ));
        assert_eq!(trace.requests, full.requests[..=index]);
        assert_eq!(
            [trace.records, trace.spool, trace.work],
            [request.records, request.spool, request.work]
        );
        faults += 1;
    }
    assert_eq!(faults, 14);
    eprintln!("number glyph growth storage: glyphs=384, paid allocation faults={faults}");
}

#[test]
fn shared_shape_storage_aat_map_matches_legacy_and_refuses_every_actual_aat_reservation() {
    let font = aat_font();
    let legacy = shape_equation_number_runs(input(&font, "111")).unwrap();
    assert!(legacy.1[0]
        .glyphs
        .iter()
        .all(|g| g.original_gid.get() == 19));
    let mut full = Trace::default();
    let actual = shape_equation_number_runs_with_storage(input(&font, "111"), &mut full).unwrap();
    assert_eq!(actual, legacy);
    let mut faults = 0;
    for (index, request) in full.requests.iter().enumerate() {
        if !matches!(
            request.site,
            Site::Harfrust(
                harfrust::scratch::Site::AatFeatureEvents
                    | harfrust::scratch::Site::AatChainFlags
                    | harfrust::scratch::Site::AatRangeFlags
            )
        ) {
            continue;
        }
        let mut trace = Trace {
            fail_at: Some(index),
            ..Trace::default()
        };
        assert!(matches!(
            shape_equation_number_runs_with_storage(input(&font, "111"), &mut trace),
            Err(StagingEquationNumberShapeError::Backend(
                LinkedShaperError::AllocationFailure
            ))
        ));
        assert_eq!(trace.requests, full.requests[..=index]);
        assert_eq!(
            [trace.records, trace.spool, trace.work],
            [request.records, request.spool, request.work]
        );
        faults += 1;
    }
    assert_eq!(faults, 5);
    eprintln!("number AAT storage: glyphs=3, paid allocation faults={faults}");
}
#[test]
fn shared_shape_storage_counted_sort_matches_standard_keys_and_refuses_before_next_step() {
    for count in 0..80 {
        let mut values = (0..count).map(|n| (n * 37) % 23).collect::<Vec<_>>();
        let mut expected = values.clone();
        expected.sort_unstable();
        let mut trace = Trace::default();
        sort_by_key(&mut values, |n| *n, &mut trace).unwrap();
        assert_eq!(values, expected);
        struct Work {
            used: u64,
            maximum: u64,
        }
        impl Storage for Work {
            fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
                Ok(())
            }
            fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
                Ok(())
            }
            fn work(&mut self, n: u64) -> Result<(), LinkedShaperError> {
                if self.used + n > self.maximum {
                    return Err(LinkedShaperError::AllocationFailure);
                }
                self.used += n;
                Ok(())
            }
        }
        if trace.work > 0 {
            let mut values = (0..count).map(|n| (n * 37) % 23).collect::<Vec<_>>();
            let mut w = Work {
                used: 0,
                maximum: trace.work - 1,
            };
            assert!(sort_by_key(&mut values, |n| *n, &mut w).is_err());
            assert_eq!(w.used, w.maximum);
        }
    }
}

#[test]
fn shared_shape_storage_match_positions_matches_legacy_and_refuses_every_actual_match_reservation()
{
    let font = context_font();
    let legacy = shape_equation_number_runs(input(&font, "11111")).unwrap();
    assert_eq!(legacy.1[0].glyphs.len(), 12);
    assert_eq!(
        legacy.1[0]
            .glyphs
            .iter()
            .filter(|g| g.original_gid.get() == 19)
            .count(),
        7
    );
    assert_eq!(
        legacy.1[0]
            .glyphs
            .iter()
            .filter(|g| g.original_gid.get() == 20)
            .count(),
        1
    );
    let mut full = Trace::default();
    let actual = shape_equation_number_runs_with_storage(input(&font, "11111"), &mut full).unwrap();
    assert_eq!(actual, legacy);
    let mut faults = 0;
    for (index, request) in full.requests.iter().enumerate() {
        if !matches!(
            request.site,
            Site::Harfrust(harfrust::scratch::Site::MatchPositions)
        ) {
            continue;
        }
        let mut trace = Trace {
            fail_at: Some(index),
            ..Trace::default()
        };
        assert!(matches!(
            shape_equation_number_runs_with_storage(input(&font, "11111"), &mut trace),
            Err(StagingEquationNumberShapeError::Backend(
                LinkedShaperError::AllocationFailure
            ))
        ));
        assert_eq!(trace.requests, full.requests[..=index]);
        assert_eq!(
            [trace.records, trace.spool, trace.work],
            [request.records, request.spool, request.work]
        );
        faults += 1;
    }
    assert_eq!(faults, 2);
    eprintln!("number match storage: glyphs=12, paid allocation faults={faults}");
}

#[test]
fn shared_shape_storage_indic_sort_matches_legacy_and_refuses_every_actual_match_reservation() {
    let font = indic_font();
    let legacy = shape_equation_number_runs(input(&font, "किकि")).unwrap();
    assert_eq!(
        legacy.1[0]
            .glyphs
            .iter()
            .map(|g| g.original_gid.get())
            .collect::<Vec<_>>(),
        [65, 23, 65, 23]
    );
    let mut full = Trace::default();
    let actual = shape_equation_number_runs_with_storage(input(&font, "किकि"), &mut full).unwrap();
    assert_eq!(actual, legacy);
    let mut faults = 0;
    for (index, request) in full.requests.iter().enumerate() {
        if !matches!(
            request.site,
            Site::Harfrust(harfrust::scratch::Site::IndicSortRecords)
        ) {
            continue;
        }
        let mut trace = Trace {
            fail_at: Some(index),
            ..Trace::default()
        };
        assert!(matches!(
            shape_equation_number_runs_with_storage(input(&font, "किकि"), &mut trace),
            Err(StagingEquationNumberShapeError::Backend(
                LinkedShaperError::AllocationFailure
            ))
        ));
        assert_eq!(trace.requests, full.requests[..=index]);
        assert_eq!(
            [trace.records, trace.spool, trace.work],
            [request.records, request.spool, request.work]
        );
        faults += 1;
    }
    assert_eq!(faults, 2);
    eprintln!("number Indic storage: glyphs=4, paid allocation faults={faults}");
}
