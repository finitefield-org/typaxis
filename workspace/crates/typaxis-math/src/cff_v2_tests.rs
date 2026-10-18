use super::*;
use typaxis_core::{
    M4EffectiveResourceLimits, M4ResourceLimits, ResourceLimits, ValidatedResourceLimits,
};
use typaxis_font::{
    admit_sfnt_cff1_v2, Cff1AdmissionV2, Cff1Error, Cff1SubsetSessionV2, CffMathFontFaceV2,
    CffMathGlyphMetricsV2,
};
#[derive(Debug)]
enum Failure {
    Stop(usize),
    Cff(String),
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stop(at) => write!(f, "stop {at}"),
            Self::Cff(message) => f.write_str(message),
        }
    }
}

struct Trace {
    session: Cff1SubsetSessionV2,
    requests: Vec<[u64; 3]>,
    stop: Option<usize>,
}
impl Trace {
    fn new(caps: &M4EffectiveResourceLimits) -> Self {
        Self {
            session: Cff1SubsetSessionV2::new(caps),
            requests: Vec::new(),
            stop: None,
        }
    }
    fn request(&mut self, row: [u64; 3]) -> Result<(), Failure> {
        let at = self.requests.len();
        self.requests.push(row);
        if self.stop == Some(at) {
            Err(Failure::Stop(at))
        } else {
            Ok(())
        }
    }
}
impl CffMathProviderV2 for Trace {
    type Error = Failure;
    fn work(&mut self, n: u64) -> Result<(), Failure> {
        self.request([0, 0, n])
    }
    fn glyph_metrics(
        &mut self,
        admission: &Cff1AdmissionV2,
        gid: OriginalGlyphId,
    ) -> Result<CffMathGlyphMetricsV2, Failure> {
        let Self {
            session,
            requests,
            stop,
        } = self;
        let mut refused = None;
        let result = session.math_glyph_metrics_with_charge(admission, gid, &mut |r, s, w| {
            let at = requests.len();
            requests.push([r as u64, s as u64, w as u64]);
            if *stop == Some(at) {
                refused = Some(at);
                Err(Cff1Error::SubsetByteLimit)
            } else {
                Ok(())
            }
        });
        result.map_err(|e| match refused {
            Some(at) => Failure::Stop(at),
            None => Failure::Cff(format!("{e:?}")),
        })
    }
}
fn caps() -> M4EffectiveResourceLimits {
    M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap()
}
#[test]
#[ignore = "requires unchanged original Latin Modern Math through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn native_cff_math_original_computations_share_selected_programs_and_keep_callback_prefixes() {
    let list =
        std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let source = std::fs::read(list.lines().nth(2).unwrap()).unwrap();
    assert_eq!(source.len(), 733736);
    let caps = caps();
    let admission = admit_sfnt_cff1_v2(source.into(), 0, &caps).unwrap();
    let mut setup = Vec::new();
    let face = CffMathFontFaceV2::from_admission_with_work(&admission, &mut |n| {
        setup.push(n);
        Ok::<_, usize>(())
    })
    .unwrap()
    .unwrap();
    assert_eq!(face.constants().display_operator_min_height(), 1300);
    assert_eq!(face.units_per_em(), 1000);
    let arrow = face
        .glyph_id_with_work('↕', &mut |_| Ok::<_, usize>(()))
        .unwrap()
        .unwrap();
    assert_eq!(arrow.get(), 1894);
    assert_eq!(
        face.vertical_variant_with_work(arrow, 1015, &mut |_| Ok::<_, usize>(()))
            .unwrap()
            .unwrap(),
        (arrow, 1015)
    );
    assert_eq!(
        face.vertical_variant_with_work(arrow, 1016, &mut |_| Ok::<_, usize>(()))
            .unwrap()
            .unwrap_err(),
        MathFontError::MissingVerticalVariant(1894)
    );
    let mut total_cuts = 0;
    let mut shared = Trace::new(&caps);
    for text in [
        "x^{2}+1",
        r"\frac{x+1}{y-2}",
        r"\sqrt{x}",
        r"\sqrt{\frac{x+1}{y}}",
        "∑x",
        "∫x",
    ] {
        let parsed = parse_math_source(text, MathParseLimits::new(1000, 64).unwrap()).unwrap();
        let input = MathComputationInput::new(
            MathNodeKind::Display,
            12 * 65536,
            required_math_layout_units(&parsed).unwrap(),
        )
        .unwrap();
        let mut full = Trace::new(&caps);
        let result = compute_math_cff_v2(&parsed, face, input, &mut full).unwrap();
        result.verify_sealed_cff_v2(&parsed, face).unwrap();
        assert_eq!(result.layout_work(), input.max_layout_units());
        assert_eq!(
            result.math_table_fingerprint(),
            face.math_table_fingerprint()
        );
        assert!(result
            .paints()
            .iter()
            .any(|p| matches!(p, MathPaint::Glyph(_))));
        let mut exact = Trace::new(&caps);
        let mut early = input;
        early.max_layout_units -= 1;
        assert!(matches!(
            compute_math_cff_v2(&parsed, face, early, &mut exact),
            Err(CffMathComputationErrorV2::Computation(
                MathComputationError::LayoutUnitLimit
            ))
        ));
        assert!(exact.requests.is_empty());
        for stop in 0..full.requests.len() {
            let mut short = Trace::new(&caps);
            short.stop = Some(stop);
            let failure = compute_math_cff_v2(&parsed, face, input, &mut short).unwrap_err();
            assert!(
                matches!(failure,CffMathComputationErrorV2::Access(Failure::Stop(at))if at==stop)
            );
            assert_eq!(short.requests, full.requests[..=stop]);
            total_cuts += 1;
        }
        let first = compute_math_cff_v2(&parsed, face, input, &mut shared).unwrap();
        let stats = (
            shared.session.operations_used(),
            shared.session.outline_segments_used(),
            shared.session.cached_glyph_count(),
        );
        let second = compute_math_cff_v2(&parsed, face, input, &mut shared).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            stats,
            (
                shared.session.operations_used(),
                shared.session.outline_segments_used(),
                shared.session.cached_glyph_count()
            )
        );
        eprintln!(
            "original CFF native math {text:?}: dimensions={:?} paints={} callbacks={}",
            result.dimensions(),
            result.paints().len(),
            full.requests.len()
        );
    }
    let six_case_stats = (
        shared.session.cached_glyph_count(),
        shared.session.operations_used(),
        shared.session.outline_segments_used(),
    );
    // A provider cannot substitute another source glyph's sealed metrics.
    struct Wrong<'a>(&'a mut Trace, OriginalGlyphId);
    impl CffMathProviderV2 for Wrong<'_> {
        type Error = Failure;
        fn work(&mut self, n: u64) -> Result<(), Failure> {
            self.0.work(n)
        }
        fn glyph_metrics(
            &mut self,
            a: &Cff1AdmissionV2,
            _: OriginalGlyphId,
        ) -> Result<CffMathGlyphMetricsV2, Failure> {
            self.0.glyph_metrics(a, self.1)
        }
    }
    let parsed = parse_math_source("x", MathParseLimits::new(100, 8).unwrap()).unwrap();
    let target = OriginalGlyphId::new(admission.cmap().glyph_for_sequence('A', None).unwrap());
    let mut wrong = Wrong(&mut shared, target);
    assert!(matches!(
        compute_math_cff_v2(
            &parsed,
            face,
            MathComputationInput::new(MathNodeKind::Inline, 12 * 65536, 100).unwrap(),
            &mut wrong
        ),
        Err(CffMathComputationErrorV2::Computation(
            MathComputationError::Font(MathFontError::MalformedTable("CFF "))
        ))
    ));
    for stop in 0..setup.len() {
        let mut at = 0;
        let error = CffMathFontFaceV2::from_admission_with_work(&admission, &mut |_| {
            let here = at;
            at += 1;
            if here == stop {
                Err(here)
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(error, stop);
        assert_eq!(at, stop + 1);
    }
    assert!(shared.session.cached_glyph_count() < 100);
    eprintln!("original CFF native math cases=6 callback cuts={total_cuts} selected={} ops={} segments={} setup={setup:?}",six_case_stats.0,six_case_stats.1,six_case_stats.2);
}
