use super::*;
use crate::shared_shape_storage::Site;

use crate::shared_grapheme_storage::reference_testdata as unicode_cases;

#[derive(Default)]
struct Visits {
    attempts: usize,
    accepted: usize,
    stop: Option<usize>,
    stopped: bool,
}
impl Storage for Visits {
    fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
        panic!("borrowed work helper must not reserve")
    }
    fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
        panic!("borrowed work helper must not retain")
    }
    fn work(&mut self, amount: u64) -> Result<(), LinkedShaperError> {
        assert!(!self.stopped);
        assert_eq!(amount, 1);
        let at = self.attempts;
        self.attempts += 1;
        if self.stop == Some(at) {
            self.stopped = true;
            Err(LinkedShaperError::AllocationFailure)
        } else {
            self.accepted += 1;
            Ok(())
        }
    }
}
struct Silent;
impl Storage for Silent {
    const OBSERVED: bool = false;
    fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
        panic!("ordinary reserve hook")
    }
    fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
        panic!("ordinary record hook")
    }
    fn work(&mut self, _: u64) -> Result<(), LinkedShaperError> {
        panic!("ordinary work hook")
    }
}
fn all_cutoffs<T: std::fmt::Debug>(
    calls: usize,
    mut apply: impl FnMut(&mut Visits) -> Result<T, LinkedShaperError>,
) -> usize {
    for stop in 0..calls {
        let mut short = Visits {
            stop: Some(stop),
            ..Visits::default()
        };
        assert!(matches!(
            apply(&mut short),
            Err(LinkedShaperError::AllocationFailure)
        ));
        assert_eq!(
            (short.attempts, short.accepted, short.stopped),
            (stop + 1, stop, true)
        );
    }
    calls
}

#[test]
fn text_work_script_intersections_match_pinned_for_str_and_refuse_each_scalar_and_terminal() {
    let stress = "a".to_owned() + &"\u{0308}".repeat(1024);
    let mut cases = 0;
    let mut cuts = 0;
    for text in unicode_cases::TEST_SAME
        .iter()
        .map(|r| r.0)
        .chain(unicode_cases::TEST_DIFF.iter().map(|r| r.0))
        .chain(["", stress.as_str(), "a日", "العربية हिन्दी 한글 កម្ពុជា ᬩ"])
    {
        let expected = ScriptExtension::for_str(text);
        let mut full = Visits::default();
        assert_eq!(script_candidates(text, &mut full).unwrap(), expected);
        assert_eq!(full.accepted, text.chars().count() + 1);
        assert_eq!(script_candidates(text, &mut Silent).unwrap(), expected);
        cuts += all_cutoffs(full.attempts, |storage| script_candidates(text, storage));
        cases += 1;
    }
    eprintln!("text work script cases={cases}, scalar/terminal refusal cutoffs={cuts}");
}

#[test]
fn text_work_line_partition_and_bidi_levels_keep_standard_results_at_every_visit() {
    let mut line_cases = 0;
    let mut line_cuts = 0;
    for len in 0..=96 {
        let ends = (0..len)
            .map(|i| (i / 3 * 7) as u32)
            .chain([u32::MAX])
            .collect::<Vec<_>>();
        for start in (0..=len * 3 + 2).chain([u32::MAX as usize, usize::MAX]) {
            let expected = ends.partition_point(|end| *end as usize <= start);
            let mut full = Visits::default();
            assert_eq!(line_index(&ends, start, &mut full).unwrap(), expected);
            assert_eq!(line_index(&ends, start, &mut Silent).unwrap(), expected);
            line_cuts += all_cutoffs(full.attempts, |storage| line_index(&ends, start, storage));
            line_cases += 1;
        }
    }
    let mut empty = Visits::default();
    assert_eq!(line_index(&[], 0, &mut empty), Ok(0));
    assert_eq!(empty.accepted, 1);
    let mut bidi_cases = 0;
    let mut bidi_cuts = 0;
    for len in 0..=32 {
        for mismatch in 0..=len {
            let levels = (0..len)
                .map(|i| {
                    if i == mismatch {
                        unicode_bidi::Level::rtl()
                    } else {
                        unicode_bidi::Level::ltr()
                    }
                })
                .collect::<Vec<_>>();
            let expected = levels
                .iter()
                .any(|level| *level != unicode_bidi::Level::ltr());
            let mut full = Visits::default();
            assert_eq!(
                mixed_levels(&levels, unicode_bidi::Level::ltr(), &mut full).unwrap(),
                expected
            );
            assert_eq!(
                mixed_levels(&levels, unicode_bidi::Level::ltr(), &mut Silent).unwrap(),
                expected
            );
            bidi_cuts += all_cutoffs(full.attempts, |storage| {
                mixed_levels(&levels, unicode_bidi::Level::ltr(), storage)
            });
            bidi_cases += 1;
        }
    }
    eprintln!("text work line cases={line_cases}, refusal cutoffs={line_cuts}; bidi cases={bidi_cases}, refusal cutoffs={bidi_cuts}");
}

fn run(advances: &[i64]) -> StagingEquationNumberGlyphRun {
    let source_span = TextSpan::new(
        typaxis_core::TextBufferId::new(42),
        Utf8ByteOffset::new(17),
        Utf8ByteOffset::new(20),
    )
    .unwrap();
    StagingEquationNumberGlyphRun {
        run_id: GlyphRunId::new(0),
        bidi_level: BidiLevel::LTR,
        script: OpenTypeTag::new(*b"latn").unwrap(),
        source_span,
        glyphs: advances
            .iter()
            .map(|&raw| ShapedGlyph {
                original_gid: OriginalGlyphId::new(1),
                advance_x: Length::from_raw(raw).unwrap(),
                advance_y: Length::ZERO,
                offset_x: Length::ZERO,
                offset_y: Length::ZERO,
            })
            .collect(),
        clusters: Vec::new(),
    }
}
#[test]
fn text_work_number_width_preserves_nonpositive_range_and_sum_overflow_errors() {
    let max = typaxis_core::JSON_SAFE_INTEGER_MAX;
    let cases = [
        vec![],
        vec![run(&[])],
        vec![run(&[0])],
        vec![run(&[-1])],
        vec![run(&[2, 3]), run(&[-1, 7])],
        vec![run(&[max, max])],
        vec![run(&vec![max; 1025])],
        vec![run(&vec![-max; 1025])],
    ];
    let mut cutoffs = 0;
    for runs in &cases {
        let expected = equation_number_runs_width(runs);
        let mut full = Visits::default();
        assert_eq!(number_width(runs, &mut full).unwrap(), expected);
        assert_eq!(number_width(runs, &mut Silent).unwrap(), expected);
        cutoffs += all_cutoffs(full.attempts, |storage| number_width(runs, storage));
    }
    eprintln!(
        "text work width cases={}, refusal cutoffs={cutoffs}",
        cases.len()
    );
}

#[test]
fn text_work_itemization_matches_ordinary_for_multilingual_and_invalid_clusters() {
    use crate::shared_shape_storage::{itemize_run_specs_with_storage, NoStorage};
    #[derive(Default)]
    struct Loan {
        work: u64,
        visits: usize,
    }
    impl Storage for Loan {
        fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
            Ok(())
        }
        fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
            panic!("itemization must not retain output records")
        }
        fn work(&mut self, n: u64) -> Result<(), LinkedShaperError> {
            self.work += n;
            self.visits += 1;
            Ok(())
        }
    }
    let mut cases = 0;
    for text in unicode_cases::TEST_SAME
        .iter()
        .map(|r| r.0)
        .chain(unicode_cases::TEST_DIFF.iter().map(|r| r.0))
        .chain([
            "",
            "Result Result",
            "العربية हिन्दी 한글 កម្ពុជា ᬩ",
            "日a\u{0308}",
            "a\nb",
            "a\u{2028}b",
        ])
    {
        let mut loan = Loan::default();
        let ordinary = itemize_run_specs_with_storage(text, &mut NoStorage).map(|(level, runs)| {
            (
                level,
                runs.into_iter()
                    .map(|r| (r.start, r.end, r.bidi_level, r.script))
                    .collect::<Vec<_>>(),
            )
        });
        let observed = itemize_run_specs_with_storage(text, &mut loan).map(|(level, runs)| {
            (
                level,
                runs.into_iter()
                    .map(|r| (r.start, r.end, r.bidi_level, r.script))
                    .collect::<Vec<_>>(),
            )
        });
        assert_eq!(observed, ordinary, "{text:?}");
        cases += 1;
    }
    eprintln!("text work itemization ordinary/observed cases={cases}");
}
