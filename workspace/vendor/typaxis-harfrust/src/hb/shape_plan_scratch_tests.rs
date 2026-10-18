use super::*;
use crate::scratch::{Budget, Error, ObservePlan, PlanScratch, Site};
use alloc::{format, string::String, vec, vec::Vec};

const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/profiles/production-book-1/combined/job/body.ttf"
));

#[derive(Clone, Debug, PartialEq, Eq)]
struct Request {
    site: Site,
    count: usize,
    bytes: usize,
    spool: usize,
    work: usize,
}

#[derive(Default)]
struct Meter {
    spool: usize,
    work: usize,
    requests: Vec<Request>,
    fail_at: Option<usize>,
    maximum_work: Option<usize>,
    failed: bool,
}

impl Budget for Meter {
    fn reserve(&mut self, site: Site, count: usize, bytes: usize) -> Result<(), Error> {
        assert!(!self.failed, "no observer call after the first refusal");
        self.work(count)?;
        self.spool += count.checked_mul(bytes).unwrap();
        self.requests.push(Request {
            site,
            count,
            bytes,
            spool: self.spool,
            work: self.work,
        });
        if self.fail_at == Some(self.requests.len() - 1) {
            self.failed = true;
            return Err(Error::AllocationFailure);
        }
        Ok(())
    }
    fn work(&mut self, count: usize) -> Result<(), Error> {
        assert!(!self.failed, "no observer call after the first refusal");
        let target = self.work.checked_add(count).unwrap();
        if self.maximum_work.is_some_and(|maximum| target > maximum) {
            self.failed = true;
            return Err(Error::Refused);
        }
        self.work = target;
        Ok(())
    }
}

fn properties() -> [(Script, &'static str); 10] {
    [
        (crate::script::LATIN, "en"),
        (crate::script::ARABIC, "ar"),
        (crate::script::DEVANAGARI, "hi"),
        (crate::script::BENGALI, "bn"),
        (crate::script::KHMER, "km"),
        (crate::script::HANGUL, "ko"),
        (crate::script::BALINESE, "ban"),
        (crate::script::HEBREW, "he"),
        (crate::script::THAI, "th"),
        (crate::script::MYANMAR, "my"),
    ]
}

fn features() -> Vec<Feature> {
    [
        "liga=0",
        "liga=1",
        "kern=0",
        "frac=1",
        "locl=1",
        "mark[0:2]=1",
        "liga[1:3]=0",
    ]
    .into_iter()
    .map(|text| text.parse().unwrap())
    .collect()
}

fn snapshot(plan: &hb_ot_shape_plan_t) -> String {
    format!(
        "{:?} {:?} {:?} {:?} {:?}",
        plan.direction,
        plan.script,
        [
            plan.frac_mask,
            plan.numr_mask,
            plan.dnom_mask,
            plan.rtlm_mask,
            plan.kern_mask
        ],
        [
            plan.requested_kerning,
            plan.has_frac,
            plan.has_vert,
            plan.has_gpos_mark,
            plan.zero_marks,
            plan.fallback_glyph_classes,
            plan.fallback_mark_positioning,
            plan.adjust_mark_positioning_when_zeroing,
            plan.apply_gpos,
            plan.apply_fallback_kern,
            plan.apply_kern,
            plan.apply_kerx,
            plan.apply_morx,
            plan.apply_trak
        ],
        plan.ot_map.scratch_test_snapshot()
    )
}

#[test]
fn plan_scratch_maps_and_all_script_data_match_original_constructor() {
    let font = crate::FontRef::new(FONT).unwrap();
    let data = crate::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let features = features();
    let mut data_variants = [false; 5];
    for (script, language) in properties() {
        let language: Language = language.parse().unwrap();
        for direction in [
            Direction::LeftToRight,
            Direction::RightToLeft,
            Direction::TopToBottom,
        ] {
            let original =
                hb_ot_shape_plan_t::new(&face, direction, Some(script), Some(&language), &features);
            let mut meter = Meter::default();
            let plan = hb_ot_shape_plan_t::try_new_with_scratch(
                &face,
                direction,
                Some(script),
                Some(&language),
                &features,
                &mut meter,
            )
            .unwrap();
            assert_eq!(snapshot(&plan), snapshot(&original));
            assert!(core::ptr::eq(plan.shaper, original.shaper));
            assert!(plan.data.is_none());
            assert_eq!(plan.observed_data.is_some(), original.data.is_some());
            if let Some(value) = plan.observed_data.as_ref() {
                assert_eq!(
                    value.as_any().type_id(),
                    original.data.as_ref().unwrap().as_ref().type_id()
                );
                let index = match value {
                    ObservedPlanData::Arabic(v) => {
                        assert_eq!(v.len(), 1);
                        0
                    }
                    ObservedPlanData::Hangul(v) => {
                        assert_eq!(v.len(), 1);
                        1
                    }
                    ObservedPlanData::Indic(v) => {
                        assert_eq!(v.len(), 1);
                        2
                    }
                    ObservedPlanData::Khmer(v) => {
                        assert_eq!(v.len(), 1);
                        3
                    }
                    ObservedPlanData::Universal(v) => {
                        assert_eq!(v.len(), 1);
                        4
                    }
                };
                data_variants[index] = true;
                assert!(meter.requests.iter().any(|r| r.site == Site::PlanData));
            }
        }
    }
    assert_eq!(data_variants, [true; 5]);
}

#[test]
fn plan_scratch_every_paid_allocation_fault_and_work_cutoff_stops_before_publication() {
    let font = crate::FontRef::new(FONT).unwrap();
    let data = crate::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let features = features();
    let mut requests = 0;
    let mut cuts = 0;
    for (script, language) in properties() {
        let language: Language = language.parse().unwrap();
        let build = |meter: &mut Meter| {
            hb_ot_shape_plan_t::try_new_with_scratch(
                &face,
                Direction::LeftToRight,
                Some(script),
                Some(&language),
                &features,
                meter,
            )
        };
        let mut full = Meter::default();
        let expected = snapshot(&build(&mut full).unwrap());
        for index in 0..full.requests.len() {
            let mut meter = Meter {
                fail_at: Some(index),
                ..Meter::default()
            };
            assert!(matches!(build(&mut meter), Err(Error::AllocationFailure)));
            assert_eq!(meter.requests, full.requests[..=index]);
            assert_eq!(
                (meter.spool, meter.work),
                (full.requests[index].spool, full.requests[index].work)
            );
            requests += 1;
        }
        for maximum in 0..full.work {
            let mut meter = Meter {
                maximum_work: Some(maximum),
                ..Meter::default()
            };
            assert!(matches!(build(&mut meter), Err(Error::Refused)));
            assert!(meter.work <= maximum && meter.spool <= full.spool);
            assert_eq!(meter.requests, full.requests[..meter.requests.len()]);
            cuts += 1;
        }
        let mut exact = Meter {
            maximum_work: Some(full.work),
            ..Meter::default()
        };
        assert_eq!(snapshot(&build(&mut exact).unwrap()), expected);
        assert_eq!((exact.spool, exact.work), (full.spool, full.work));
    }
    std::eprintln!("Harfrust plan: paid allocation faults={requests}, work cutoffs={cuts}, script categories=10");
}

#[test]
fn plan_scratch_sort_has_no_storage_request_and_preserves_all_values_on_refusal() {
    for count in [0, 1, 2, 3, 7, 16, 31, 64] {
        let original: Vec<_> = (0..count).map(|n| (n * 37) % 11).collect();
        let mut expected = original.clone();
        expected.sort();
        let mut full = Meter::default();
        let mut values = original.clone();
        {
            let mut observer = ObservePlan(&mut full);
            PlanScratch::new(Some(&mut observer))
                .sort(&mut values)
                .unwrap();
        }
        assert_eq!(values, expected);
        assert!(full.requests.is_empty());
        for maximum in 0..full.work {
            let mut meter = Meter {
                maximum_work: Some(maximum),
                ..Meter::default()
            };
            let mut values = original.clone();
            let mut observer = ObservePlan(&mut meter);
            assert_eq!(
                PlanScratch::new(Some(&mut observer)).sort(&mut values),
                Err(Error::Refused)
            );
            values.sort();
            assert_eq!(values, expected);
            assert!(meter.requests.is_empty() && meter.work <= maximum);
        }
    }
}

#[test]
fn plan_scratch_output_retains_no_loan_and_default_dispatch_never_calls_the_observer() {
    struct Original;
    impl Budget for Original {
        const OBSERVED: bool = false;
        fn reserve(&mut self, _: Site, _: usize, _: usize) -> Result<(), Error> {
            panic!("unobserved reserve")
        }
        fn work(&mut self, _: usize) -> Result<(), Error> {
            panic!("unobserved work")
        }
    }
    fn owned<T: Send + Sync + 'static>(_: &T) {}
    let font = crate::FontRef::new(FONT).unwrap();
    let data = crate::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let script = Some(crate::script::ARABIC);
    let plan = {
        let mut meter = Meter::default();
        hb_ot_shape_plan_t::try_new_with_scratch(
            &face,
            Direction::LeftToRight,
            script,
            None,
            &[],
            &mut meter,
        )
        .unwrap()
    };
    owned(&plan);
    let old = hb_ot_shape_plan_t::try_new_with_scratch(
        &face,
        Direction::LeftToRight,
        script,
        None,
        &[],
        &mut Original,
    )
    .unwrap();
    assert_eq!(snapshot(&plan), snapshot(&old));
    let mut input = crate::UnicodeBuffer::new();
    input.push_str("(123)");
    input.guess_segment_properties();
    owned(
        &face
            .try_shape_with_scratch(input, &[], &mut Original)
            .unwrap(),
    );
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARFRUST_CACHE_FONTS list of unchanged original fonts"]
fn plan_scratch_original_fonts_match_immutable_registry_and_all_reservation_faults() {
    let paths =
        std::fs::read_to_string(std::env::var_os("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let texts = [
        "(123)",
        "日本語123",
        "AV ffi",
        "x\u{0301}",
        "العربية 123",
        "אבג (12)",
        "বাংলা",
        "हिन्दी",
        "ภาษาไทย",
        "한글",
        "កម្ពុជា",
        "မြန်မာ",
    ];
    let mut cases = 0;
    let mut faults = 0;
    let mut seen = Vec::new();
    for path in paths.lines().filter(|p| !p.is_empty()) {
        let bytes = std::fs::read(path).unwrap();
        let font = crate::FontRef::new(&bytes).unwrap();
        let data = crate::ShaperData::new(&font);
        let face = data.shaper(&font).build();
        let reference = harfrust_reference::ShaperData::new(&font);
        for text in texts {
            for feature_text in [None, Some("liga=0"), Some("frac=1")] {
                let mut buffer = crate::UnicodeBuffer::new();
                buffer.push_str(text);
                buffer.set_pre_context("x 前");
                buffer.set_post_context("後 y");
                buffer.set_language("JA-jp".parse().unwrap());
                buffer.guess_segment_properties();
                let direction = buffer.direction();
                let script = Some(buffer.script());
                let language = buffer.language();
                let features = feature_text
                    .map(|t| vec![t.parse::<Feature>().unwrap()])
                    .unwrap_or_default();
                let mut meter = Meter::default();
                let plan = hb_ot_shape_plan_t::try_new_with_scratch(
                    &face,
                    direction,
                    script,
                    language.as_ref(),
                    &features,
                    &mut meter,
                )
                .unwrap();
                let original =
                    hb_ot_shape_plan_t::new(&face, direction, script, language.as_ref(), &features);
                assert_eq!(snapshot(&plan), snapshot(&original));
                for request in &meter.requests {
                    if !seen.contains(&request.site) {
                        seen.push(request.site);
                    }
                }
                for index in 0..meter.requests.len() {
                    let mut failed = Meter {
                        fail_at: Some(index),
                        ..Meter::default()
                    };
                    assert!(matches!(
                        hb_ot_shape_plan_t::try_new_with_scratch(
                            &face,
                            direction,
                            script,
                            language.as_ref(),
                            &features,
                            &mut failed
                        ),
                        Err(Error::AllocationFailure)
                    ));
                    assert_eq!(failed.requests, meter.requests[..=index]);
                    faults += 1;
                }
                let actual = face.shape_with_plan(&plan, buffer, &features);
                let mut buffer = harfrust_reference::UnicodeBuffer::new();
                buffer.push_str(text);
                buffer.set_pre_context("x 前");
                buffer.set_post_context("後 y");
                buffer.set_language("JA-jp".parse().unwrap());
                buffer.guess_segment_properties();
                let features: Vec<harfrust_reference::Feature> = feature_text
                    .map(|t| vec![t.parse().unwrap()])
                    .unwrap_or_default();
                let expected = reference.shaper(&font).build().shape(buffer, &features);
                assert_eq!(
                    format!("{:?}", actual.glyph_infos()),
                    format!("{:?}", expected.glyph_infos()),
                    "{path} {text}"
                );
                assert_eq!(
                    format!("{:?}", actual.glyph_positions()),
                    format!("{:?}", expected.glyph_positions()),
                    "{path} {text}"
                );
                cases += 1;
            }
        }
    }
    for site in [
        Site::PlanFeatureInfos,
        Site::PlanGsubPauses,
        Site::PlanGposPauses,
        Site::PlanFeatures,
        Site::PlanGsubLookups,
        Site::PlanGposLookups,
        Site::PlanGsubStages,
        Site::PlanGposStages,
        Site::PlanData,
        Site::LanguageString,
    ] {
        assert!(seen.contains(&site), "{site:?}");
    }
    assert!(cases >= 144);
    std::eprintln!("Harfrust plan immutable-registry original-font cases={cases}, paid reserve faults={faults}, sites={}",seen.len());
}
