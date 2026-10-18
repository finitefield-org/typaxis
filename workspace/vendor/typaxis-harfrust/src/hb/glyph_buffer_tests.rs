use super::*;
use crate::scratch::{Budget, Error, ObservePlan, PlanScratch, Site};
use alloc::{format, string::String, vec::Vec};

use crate::test_font as fixture;
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
        assert!(!self.failed);
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
        assert!(!self.failed);
        let target = self.work.checked_add(count).unwrap();
        if self.maximum_work.is_some_and(|n| target > n) {
            self.failed = true;
            return Err(Error::Refused);
        }
        self.work = target;
        Ok(())
    }
}

fn input(text: &str) -> UnicodeBuffer {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_language("en".parse().unwrap());
    buffer.guess_segment_properties();
    buffer
}

fn fixture_font() -> Vec<u8> {
    let font = crate::FontRef::new(FONT).unwrap();
    let data = crate::ShaperData::new(&font);
    let shaped = data.shaper(&font).build().shape(input("R"), &[]);
    assert_eq!(shaped.len(), 1);
    fixture::multiple_substitution_font(
        FONT,
        u16::try_from(shaped.glyph_infos()[0].glyph_id).unwrap(),
        128,
    )
}

fn content(buffer: &hb_buffer_t<'_>) -> String {
    format!(
        "{:?} {:?} {} {} {}",
        buffer.info, buffer.pos, buffer.len, buffer.idx, buffer.out_len
    )
}

#[test]
fn glyph_buffer_growth_is_geometric_and_paired_on_every_refusal() {
    let make = || {
        let mut buffer = UnicodeBuffer::new();
        buffer
            .try_push_str_with_scratch("RRR", &mut Meter::default())
            .unwrap();
        buffer.0
    };
    let run = |meter: &mut Meter| {
        let mut observer = ObservePlan(meter);
        let mut buffer = make().into_working(PlanScratch::new(Some(&mut observer)));
        for size in [5, 7, 8, 20] {
            let before = content(&buffer);
            if !buffer.ensure(size) {
                assert_eq!(content(&buffer), before);
                assert_eq!(buffer.info.len(), buffer.pos.len());
                assert!(!buffer.successful);
                let error = buffer.scratch.status().unwrap_err();
                assert!(!buffer.ensure(size));
                return Err(error);
            }
            assert_eq!(buffer.info.len(), size);
            assert_eq!(buffer.pos.len(), size);
        }
        Ok((
            content(&buffer),
            buffer.info.capacity(),
            buffer.pos.capacity(),
        ))
    };
    let mut full = Meter::default();
    let expected = run(&mut full).unwrap();
    assert_eq!(
        full.requests.iter().map(|r| r.count).collect::<Vec<_>>(),
        [6, 6, 12, 12, 24, 24]
    );
    assert_eq!((expected.1, expected.2), (24, 24));
    for index in 0..full.requests.len() {
        let mut meter = Meter {
            fail_at: Some(index),
            ..Meter::default()
        };
        assert_eq!(run(&mut meter), Err(Error::AllocationFailure));
        assert_eq!(meter.requests, full.requests[..=index]);
        assert_eq!(
            (meter.spool, meter.work),
            (full.requests[index].spool, full.requests[index].work)
        );
    }
    for maximum in 0..full.work {
        let mut meter = Meter {
            maximum_work: Some(maximum),
            ..Meter::default()
        };
        assert_eq!(run(&mut meter), Err(Error::Refused));
        assert!(meter.work <= maximum && meter.spool <= full.spool);
        assert_eq!(meter.requests, full.requests[..meter.requests.len()]);
    }
    let mut exact = Meter {
        maximum_work: Some(full.work),
        ..Meter::default()
    };
    assert_eq!(run(&mut exact).unwrap(), expected);
    std::eprintln!(
        "Harfrust glyph paired growth: requests={}, work cutoffs={}, full spool={}",
        full.requests.len(),
        full.work,
        full.spool
    );
}

#[test]
fn glyph_buffer_multiple_substitution_matches_registry_and_refuses_every_growth_prefix() {
    let bytes = fixture_font();
    let font = crate::FontRef::new(&bytes).unwrap();
    let data = crate::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let props = input("RRR");
    let language = props.language();
    let plan = crate::ShapePlan::new(
        &face,
        props.direction(),
        Some(props.script()),
        language.as_ref(),
        &[],
    );
    let build =
        |meter: &mut Meter| face.try_shape_with_plan_and_scratch(&plan, input("RRR"), &[], meter);
    let mut full = Meter::default();
    let actual = build(&mut full).unwrap();
    assert_eq!(actual.len(), 384);
    assert_eq!(
        full.requests.iter().map(|r| r.count).collect::<Vec<_>>(),
        [8, 8, 16, 16, 32, 32, 64, 64, 128, 128, 256, 256, 512, 512]
    );
    for pair in full.requests.chunks(2) {
        assert_eq!(pair[0].site, Site::GlyphInfos);
        assert_eq!(pair[1].site, Site::GlyphPositions);
    }
    let reference = harfrust_reference::ShaperData::new(&font);
    let mut buffer = harfrust_reference::UnicodeBuffer::new();
    buffer.push_str("RRR");
    buffer.set_language("en".parse().unwrap());
    buffer.guess_segment_properties();
    let expected = reference.shaper(&font).build().shape(buffer, &[]);
    assert_eq!(
        format!("{:?}", actual.glyph_infos()),
        format!("{:?}", expected.glyph_infos())
    );
    assert_eq!(
        format!("{:?}", actual.glyph_positions()),
        format!("{:?}", expected.glyph_positions())
    );
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
    }
    for maximum in 0..full.work {
        let mut meter = Meter {
            maximum_work: Some(maximum),
            ..Meter::default()
        };
        assert!(matches!(build(&mut meter), Err(Error::Refused)));
        assert!(meter.work <= maximum);
        assert_eq!(meter.requests, full.requests[..meter.requests.len()]);
    }
    let mut exact = Meter {
        maximum_work: Some(full.work),
        ..Meter::default()
    };
    let exact = build(&mut exact).unwrap();
    assert_eq!(
        format!("{:?}", exact.glyph_infos()),
        format!("{:?}", actual.glyph_infos())
    );
    std::eprintln!(
        "Harfrust GSUB glyph growth: glyphs={}, paid faults={}, work cutoffs={}, spool={}",
        actual.len(),
        full.requests.len(),
        full.work,
        full.spool
    );
}

#[test]
fn glyph_buffer_owned_state_and_reuse_do_not_retain_a_non_send_budget() {
    use std::{cell::Cell, rc::Rc};
    struct Local(Rc<Cell<usize>>);
    impl Budget for Local {
        fn reserve(&mut self, _: Site, count: usize, bytes: usize) -> Result<(), Error> {
            self.0.set(self.0.get() + count * bytes);
            Ok(())
        }
        fn work(&mut self, _: usize) -> Result<(), Error> {
            Ok(())
        }
    }
    fn owned<T: Send + Sync + 'static>(_: &T) {}
    assert_eq!(
        core::mem::size_of::<UnicodeBuffer>(),
        core::mem::size_of::<harfrust_reference::UnicodeBuffer>()
    );
    assert_eq!(
        core::mem::size_of::<GlyphBuffer>(),
        core::mem::size_of::<harfrust_reference::GlyphBuffer>()
    );
    let bytes = fixture_font();
    let font = crate::FontRef::new(&bytes).unwrap();
    let data = crate::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    let count = Rc::new(Cell::new(0));
    let glyphs = {
        let mut local = Local(count.clone());
        face.try_shape_with_scratch(input("RRR"), &[], &mut local)
            .unwrap()
    };
    assert_eq!(glyphs.len(), 384);
    assert!(count.get() > 0);
    owned(&glyphs);
    let capacities = (glyphs.0.info.capacity(), glyphs.0.pos.capacity());
    let mut buffer = glyphs.clear();
    owned(&buffer);
    assert_eq!(
        (buffer.0.info.capacity(), buffer.0.pos.capacity()),
        capacities
    );
    buffer.push_str("RRR");
    buffer.set_language("en".parse().unwrap());
    buffer.guess_segment_properties();
    let language = buffer.language();
    let plan = crate::ShapePlan::new(
        &face,
        buffer.direction(),
        Some(buffer.script()),
        language.as_ref(),
        &[],
    );
    let mut reused = Meter::default();
    let glyphs = face
        .try_shape_with_plan_and_scratch(&plan, buffer, &[], &mut reused)
        .unwrap();
    owned(&glyphs);
    assert_eq!(glyphs.len(), 384);
    assert_eq!(reused.spool, 0);
    assert!(reused.requests.is_empty());
}

#[test]
fn glyph_buffer_unobserved_dispatch_and_native_length_limit_keep_original_rules() {
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
    let bytes = fixture_font();
    let font = crate::FontRef::new(&bytes).unwrap();
    let data = crate::ShaperData::new(&font);
    let face = data.shaper(&font).build();
    assert_eq!(
        face.try_shape_with_scratch(input("RRR"), &[], &mut Original)
            .unwrap()
            .len(),
        384
    );
    let mut meter = Meter::default();
    let mut observer = ObservePlan(&mut meter);
    let mut buffer = input("R")
        .0
        .into_working(PlanScratch::new(Some(&mut observer)));
    buffer.max_len = 2;
    let before = content(&buffer);
    assert!(!buffer.ensure(3));
    assert_eq!(buffer.scratch.status(), Err(Error::InputLimit));
    assert_eq!(content(&buffer), before);
    assert!(!buffer.successful);
    drop(buffer);
    assert!(meter.requests.is_empty());
    assert_eq!((meter.spool, meter.work), (0, 0));
}

#[test]
fn glyph_buffer_public_input_keeps_its_state_if_a_caller_budget_panics() {
    struct Panics(Meter);
    impl Budget for Panics {
        fn reserve(&mut self, site: Site, count: usize, bytes: usize) -> Result<(), Error> {
            self.0.reserve(site, count, bytes)?;
            if self.0.requests.len() == 2 {
                panic!("caller budget panic after second payment");
            }
            Ok(())
        }
        fn work(&mut self, count: usize) -> Result<(), Error> {
            self.0.work(count)
        }
    }
    let mut buffer = input("RRR");
    buffer.set_pre_context("before");
    buffer.set_post_context("after");
    let before = buffer.0.modify(|working| content(working));
    let properties = (
        buffer.direction(),
        buffer.script(),
        buffer.language(),
        buffer.flags().bits(),
    );
    let mut budget = Panics(Meter::default());
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = buffer.try_push_str_with_scratch("XYZ", &mut budget);
    }));
    assert!(panic.is_err());
    assert_eq!(budget.0.requests.len(), 2);
    assert_eq!(buffer.0.modify(|working| content(working)), before);
    assert_eq!(
        (
            buffer.direction(),
            buffer.script(),
            buffer.language(),
            buffer.flags().bits()
        ),
        properties
    );
    assert_eq!(buffer.0.info.len(), buffer.0.pos.len());
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARFRUST_CACHE_FONTS list of unchanged original fonts"]
fn glyph_buffer_original_fonts_match_registry_with_live_growth() {
    let paths =
        std::fs::read_to_string(std::env::var_os("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let mut cases = 0;
    let mut growth = 0;
    for path in paths.lines().filter(|p| !p.is_empty()) {
        let bytes = std::fs::read(path).unwrap();
        let font = crate::FontRef::new(&bytes).unwrap();
        let data = crate::ShaperData::new(&font);
        let reference = harfrust_reference::ShaperData::new(&font);
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
            "\u{0344}",
            "\u{0e33}",
            "\u{0f73}",
        ] {
            let mut meter = Meter::default();
            let mut buffer = UnicodeBuffer::new();
            buffer.try_push_str_with_scratch(text, &mut meter).unwrap();
            buffer.set_language(Language::try_from_str_with_scratch("en", &mut meter).unwrap());
            buffer.guess_segment_properties();
            let actual = data
                .shaper(&font)
                .build()
                .try_shape_with_scratch(buffer, &[], &mut meter)
                .unwrap();
            let mut buffer = harfrust_reference::UnicodeBuffer::new();
            buffer.push_str(text);
            buffer.set_language("en".parse().unwrap());
            buffer.guess_segment_properties();
            let expected = reference.shaper(&font).build().shape(buffer, &[]);
            assert_eq!(
                format!("{:?}", actual.glyph_infos()),
                format!("{:?}", expected.glyph_infos()),
                "{path} {text:?}"
            );
            assert_eq!(
                format!("{:?}", actual.glyph_positions()),
                format!("{:?}", expected.glyph_positions()),
                "{path} {text:?}"
            );
            growth += meter
                .requests
                .iter()
                .filter(|r| r.site == Site::GlyphInfos || r.site == Site::GlyphPositions)
                .count();
            cases += 1;
        }
    }
    assert!(cases >= 60 && growth > 0);
    std::eprintln!(
        "Harfrust original-font live glyph buffers: cases={cases}, growth reservations={growth}"
    );
}
