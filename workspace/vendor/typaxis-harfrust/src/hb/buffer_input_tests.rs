use super::*;
use crate::scratch::{Budget, Error, Site};
use alloc::{format, string::String, vec::Vec};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Request {
    site: Site,
    count: usize,
    element_bytes: usize,
    spool: usize,
    work: usize,
}

#[derive(Default)]
struct Meter {
    requests: Vec<Request>,
    spool: usize,
    work: usize,
    fail_at: Option<usize>,
    work_limit: Option<usize>,
}
impl Budget for Meter {
    fn reserve(&mut self, site: Site, count: usize, element_bytes: usize) -> Result<(), Error> {
        let bytes = count.checked_mul(element_bytes).unwrap();
        self.work(bytes)?;
        self.spool = self.spool.checked_add(bytes).unwrap();
        self.requests.push(Request {
            site,
            count,
            element_bytes,
            spool: self.spool,
            work: self.work,
        });
        if self.fail_at == Some(self.requests.len() - 1) {
            return Err(Error::AllocationFailure);
        }
        Ok(())
    }
    fn work(&mut self, amount: usize) -> Result<(), Error> {
        let next = self.work.checked_add(amount).unwrap();
        if self.work_limit.is_some_and(|limit| next > limit) {
            return Err(Error::Refused);
        }
        self.work = next;
        Ok(())
    }
}

fn snapshot(buffer: &UnicodeBuffer) -> String {
    let b = &buffer.0;
    format!(
        "{}/{}/{:?}/{:?}/{:?}/{:?}/{:?}/{:?}/{:?}",
        b.len,
        b.successful,
        b.info,
        b.pos,
        b.context,
        b.context_len,
        b.direction,
        b.script,
        b.language
    )
}

fn prepared() -> UnicodeBuffer {
    let mut b = UnicodeBuffer::new();
    b.push_str("X");
    b.set_direction(Direction::RightToLeft);
    b.set_script(script::ARABIC);
    b.set_pre_context("before日本");
    b.set_post_context("afterภาษา");
    b
}

#[test]
fn input_scratch_matches_original_unicode_and_context_for_append_and_reuse() {
    let cases = [
        "",
        "abc",
        "é\0a\u{301}",
        "本文前後",
        "אב(12)",
        "١٢٣",
        "क्\u{200d}ष",
        "👩\u{200d}🚀",
    ];
    for text in cases {
        let mut original = prepared();
        let mut observed = prepared();
        let mut meter = Meter::default();
        original.push_str(text);
        observed
            .try_push_str_with_scratch(text, &mut meter)
            .unwrap();
        assert_eq!(snapshot(&observed), snapshot(&original));
        original.clear();
        observed.clear();
        original.push_str(text);
        observed
            .try_push_str_with_scratch(text, &mut meter)
            .unwrap();
        assert_eq!(snapshot(&observed), snapshot(&original));
        assert_eq!(observed.len(), text.chars().count());
    }
}

#[test]
fn input_scratch_faults_preserve_paired_lengths_and_original_logical_input() {
    let text = "本文é١(12)👩\u{200d}🚀";
    let mut full = Meter::default();
    let mut success = prepared();
    success.try_push_str_with_scratch(text, &mut full).unwrap();
    assert_eq!(
        full.requests.iter().map(|r| r.site).collect::<Vec<_>>(),
        [Site::UnicodeInfos, Site::UnicodePositions]
    );
    for index in 0..full.requests.len() {
        let mut buffer = prepared();
        let before = snapshot(&buffer);
        let mut meter = Meter {
            fail_at: Some(index),
            ..Meter::default()
        };
        assert_eq!(
            buffer.try_push_str_with_scratch(text, &mut meter),
            Err(Error::AllocationFailure)
        );
        assert_eq!(snapshot(&buffer), before);
        assert_eq!(meter.requests, full.requests[..=index]);
        assert_eq!(
            (meter.spool, meter.work),
            (full.requests[index].spool, full.requests[index].work)
        );
        assert_eq!(buffer.0.info.len(), buffer.0.pos.len());
        buffer.try_push_str_with_scratch(text, &mut meter).unwrap();
        assert_eq!(snapshot(&buffer), snapshot(&success));
    }
    for limit in 0..full.work {
        let mut buffer = prepared();
        let before = snapshot(&buffer);
        let mut meter = Meter {
            work_limit: Some(limit),
            ..Meter::default()
        };
        assert_eq!(
            buffer.try_push_str_with_scratch(text, &mut meter),
            Err(Error::Refused)
        );
        assert_eq!(snapshot(&buffer), before);
        assert_eq!(meter.requests, full.requests[..meter.requests.len()]);
        assert!(meter.work <= limit && meter.spool <= full.spool);
        assert_eq!(buffer.0.info.len(), buffer.0.pos.len());
    }
    std::eprintln!(
        "Harfrust input: allocation sites=2, faults={}, work cuts={}",
        full.requests.len(),
        full.work
    );
}

#[test]
fn input_scratch_reallocation_pays_the_whole_new_target_and_keeps_prior_charges() {
    let mut buffer = UnicodeBuffer::new();
    let mut meter = Meter::default();
    buffer.try_push_str_with_scratch("abc", &mut meter).unwrap();
    let old_spool = meter.spool;
    let extra = buffer.0.info.capacity().max(buffer.0.pos.capacity()) + 3;
    let text = "x".repeat(extra);
    buffer.try_push_str_with_scratch(&text, &mut meter).unwrap();
    assert_eq!(meter.requests.len(), 4);
    let target = 3 + extra;
    assert_eq!(meter.requests[2].count, target);
    assert_eq!(meter.requests[3].count, target);
    assert_eq!(
        meter.spool,
        old_spool
            + target
                * (core::mem::size_of::<hb_glyph_info_t>() + core::mem::size_of::<GlyphPosition>())
    );
    let charged = (meter.spool, meter.requests.len());
    buffer.clear();
    buffer.try_push_str_with_scratch("a", &mut meter).unwrap();
    assert_eq!((meter.spool, meter.requests.len()), charged);
}

#[test]
fn input_scratch_length_limit_refuses_before_allocation_as_the_original_does() {
    let mut original = UnicodeBuffer::new();
    let mut observed = UnicodeBuffer::new();
    original.0.max_len = 2;
    observed.0.max_len = 2;
    let mut meter = Meter::default();
    original.push_str("日本語");
    assert_eq!(
        observed.try_push_str_with_scratch("日本語", &mut meter),
        Err(Error::InputLimit)
    );
    assert_eq!(snapshot(&observed), snapshot(&original));
    assert!(meter.requests.is_empty());
    assert_eq!(meter.work, "日本語".len());
}

#[test]
fn input_scratch_real_capacity_failure_retains_payment_and_overflow_does_not_pay() {
    let mut values = Vec::<u8>::new();
    let mut meter = Meter::default();
    assert_eq!(
        crate::scratch::reserve(&mut values, usize::MAX, Site::UnicodeInfos, &mut meter),
        Err(Error::AllocationFailure)
    );
    assert!(values.is_empty());
    assert_eq!(meter.spool, usize::MAX);
    assert_eq!(meter.requests.len(), 1);
    let mut values = Vec::<u64>::new();
    let mut meter = Meter::default();
    assert_eq!(
        crate::scratch::reserve(&mut values, usize::MAX, Site::UnicodeInfos, &mut meter),
        Err(Error::ArithmeticOverflow)
    );
    assert!(meter.requests.is_empty());
    assert_eq!((meter.spool, meter.work), (0, 0));
}

#[test]
fn input_scratch_unobserved_path_never_calls_budget_or_retains_its_borrow() {
    struct Never;
    impl Budget for Never {
        const OBSERVED: bool = false;
        fn reserve(&mut self, _: Site, _: usize, _: usize) -> Result<(), Error> {
            panic!("unobserved reservation")
        }
        fn work(&mut self, _: usize) -> Result<(), Error> {
            panic!("unobserved work")
        }
    }
    fn send_sync_static<T: Send + Sync + 'static>() {}
    send_sync_static::<UnicodeBuffer>();
    let mut buffer = UnicodeBuffer::new();
    {
        let mut meter = Never;
        buffer
            .try_push_str_with_scratch("日本(12)", &mut meter)
            .unwrap();
    }
    let mut original = UnicodeBuffer::new();
    original.push_str("日本(12)");
    assert_eq!(snapshot(&buffer), snapshot(&original));
}

#[test]
fn input_language_scratch_preserves_original_parser_and_paid_failure_prefixes() {
    for text in ["ja-JP", "EN-us", "x-İ-É-ABC", "العربية", "\0"] {
        let mut full = Meter::default();
        let actual = Language::try_from_str_with_scratch(text, &mut full).unwrap();
        assert_eq!(actual, text.parse::<Language>().unwrap());
        assert_eq!(full.requests.len(), 1);
        assert_eq!(full.requests[0].site, Site::LanguageString);
        assert_eq!(full.spool, text.len());
        assert_eq!(full.work, 3 * text.len());
        let mut failed = Meter {
            fail_at: Some(0),
            ..Meter::default()
        };
        assert_eq!(
            Language::try_from_str_with_scratch(text, &mut failed),
            Err(Error::AllocationFailure)
        );
        assert_eq!(failed.requests, full.requests);
        assert_eq!(
            (failed.spool, failed.work),
            (full.requests[0].spool, full.requests[0].work)
        );
        for limit in 0..full.work {
            let mut failed = Meter {
                work_limit: Some(limit),
                ..Meter::default()
            };
            assert_eq!(
                Language::try_from_str_with_scratch(text, &mut failed),
                Err(Error::Refused)
            );
            assert!(failed.work <= limit && failed.spool <= full.spool);
            assert_eq!(failed.requests, full.requests[..failed.requests.len()]);
        }
    }
    let mut meter = Meter::default();
    assert_eq!(
        Language::try_from_str_with_scratch("", &mut meter),
        Err(Error::InvalidLanguage)
    );
    assert!("".parse::<Language>().is_err());
    assert_eq!((meter.spool, meter.work), (0, 0));
    assert!(meter.requests.is_empty());
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARFRUST_CACHE_FONTS list of original font paths"]
fn input_scratch_original_fonts_match_immutable_registry_shaping() {
    let paths =
        std::fs::read_to_string(std::env::var_os("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let mut cases = 0;
    for path in paths.lines().filter(|line| !line.is_empty()) {
        let bytes = std::fs::read(path).unwrap();
        let face = crate::FontRef::new(&bytes).unwrap();
        let reference = harfrust_reference::ShaperData::new(&face);
        let mut meter = Meter::default();
        let actual = crate::ShaperData::try_new_with_scratch(&face, &mut meter).unwrap();
        for text in [
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
        ] {
            let mut buffer = UnicodeBuffer::new();
            buffer.try_push_str_with_scratch(text, &mut meter).unwrap();
            buffer.set_pre_context("x 前");
            buffer.set_post_context("後 y");
            buffer.set_language(Language::try_from_str_with_scratch("JA-jp", &mut meter).unwrap());
            buffer.guess_segment_properties();
            let shaped = actual.shaper(&face).build().shape(buffer, &[]);
            let mut old_buffer = harfrust_reference::UnicodeBuffer::new();
            old_buffer.push_str(text);
            old_buffer.set_pre_context("x 前");
            old_buffer.set_post_context("後 y");
            old_buffer.set_language("JA-jp".parse().unwrap());
            old_buffer.guess_segment_properties();
            let expected = reference.shaper(&face).build().shape(old_buffer, &[]);
            assert_eq!(
                format!("{:?}", shaped.glyph_infos()),
                format!("{:?}", expected.glyph_infos()),
                "{path} {text:?}"
            );
            assert_eq!(
                format!("{:?}", shaped.glyph_positions()),
                format!("{:?}", expected.glyph_positions()),
                "{path} {text:?}"
            );
            cases += 1;
        }
    }
    assert!(cases >= 48);
    std::eprintln!("Harfrust observed input/language immutable-registry shaping cases={cases}");
}
