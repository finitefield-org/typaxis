use super::*;
use crate::{BidiInfo, Level};
use alloc::string::String;
use std::{eprintln, fs};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Request {
    site: Site,
    count: usize,
    element_bytes: usize,
    spool: usize,
    work: usize,
}
#[derive(Default)]
struct Meter {
    spool: usize,
    work: usize,
    requests: Vec<Request>,
    fail_at: Option<usize>,
}
impl Budget for Meter {
    fn reserve(&mut self, site: Site, count: usize, element_bytes: usize) -> Result<(), Error> {
        let bytes = count
            .checked_mul(element_bytes)
            .ok_or(Error::ArithmeticOverflow)?;
        self.spool = self
            .spool
            .checked_add(bytes)
            .ok_or(Error::ArithmeticOverflow)?;
        self.work += bytes;
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
        self.work += amount;
        Ok(())
    }
}
fn compare<'text>(text: &'text str, base: Option<Level>, meter: &mut Meter) -> BidiInfo<'text> {
    let actual = BidiInfo::try_new_with_scratch(text, base, meter).unwrap();
    let legacy = BidiInfo::new(text, base);
    assert_eq!(actual.levels, legacy.levels);
    assert_eq!(actual.original_classes, legacy.original_classes);
    assert_eq!(actual.paragraphs, legacy.paragraphs);
    let expected = unicode_bidi_reference::BidiInfo::new(
        text,
        base.map(|l| unicode_bidi_reference::Level::new(l.number()).unwrap()),
    );
    assert_eq!(
        actual.levels.iter().map(|l| l.number()).collect::<Vec<_>>(),
        expected
            .levels
            .iter()
            .map(|l| l.number())
            .collect::<Vec<_>>(),
        "{text:?}"
    );
    assert_eq!(
        actual
            .original_classes
            .iter()
            .map(|c| *c as u8)
            .collect::<Vec<_>>(),
        expected
            .original_classes
            .iter()
            .map(|c| *c as u8)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        actual
            .paragraphs
            .iter()
            .map(|p| (p.range.clone(), p.level.number()))
            .collect::<Vec<_>>(),
        expected
            .paragraphs
            .iter()
            .map(|p| (p.range.clone(), p.level.number()))
            .collect::<Vec<_>>()
    );
    actual
}
#[test]
fn scratch_matches_original_unicode_bidi_and_faults_every_request() {
    let inputs = [
        "א\u{00ad}$$a",
        "",
        "(123)",
        "日本語123",
        "abc א(123) العربية",
        "a\u{2068}א\u{2066}(4)\u{2069}b\u{2069}z",
        "\u{202e}a((ב))\u{202c} ",
        "a\u{2029}א\n123",
    ];
    let mut seen = Vec::new();
    let mut faults = 0;
    for text in inputs {
        let mut full = Meter::default();
        compare(text, None, &mut full);
        for request in &full.requests {
            if !seen.contains(&request.site) {
                seen.push(request.site);
            }
        }
        for index in 0..full.requests.len() {
            let mut meter = Meter {
                fail_at: Some(index),
                ..Meter::default()
            };
            let result = BidiInfo::try_new_with_scratch(text, None, &mut meter);
            assert_eq!(result.unwrap_err(), Error::AllocationFailure);
            assert_eq!(meter.requests, full.requests[..=index]);
            assert_eq!(
                (meter.spool, meter.work),
                (full.requests[index].spool, full.requests[index].work)
            );
            faults += 1;
        }
    }
    eprintln!("bidi scratch sites={seen:?}, post-payment allocation failures={faults}");
    assert_eq!(seen.len(), 17);
}
fn official_case(
    text: &str,
    base: Option<Level>,
    expected: &[Option<u8>],
    visual: &[usize],
    resolved: Option<u8>,
) {
    let mut meter = Meter::default();
    let actual = compare(text, base, &mut meter);
    assert_eq!(actual.paragraphs.len(), 1, "{text:?}");
    let para = &actual.paragraphs[0];
    if let Some(level) = resolved {
        assert_eq!(para.level.number(), level);
    }
    let levels = actual.reordered_levels(para, para.range.clone());
    let starts = text.char_indices().map(|(i, _)| i).collect::<Vec<_>>();
    assert_eq!(starts.len(), expected.len());
    for (i, level) in expected.iter().enumerate() {
        if let Some(level) = level {
            assert_eq!(levels[starts[i]].number(), *level, "char={i} text={text:?}");
        }
    }
    // Rule X9 removes these indexes before L2. The constructor itself retains
    // their resolved levels for caller affiliation; do not compare x to zero.
    let retained = expected
        .iter()
        .enumerate()
        .filter(|(_, l)| l.is_some())
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let retained_levels = retained
        .iter()
        .map(|i| levels[starts[*i]])
        .collect::<Vec<_>>();
    let order = BidiInfo::reorder_visual(&retained_levels)
        .iter()
        .map(|i| retained[*i])
        .collect::<Vec<_>>();
    assert_eq!(order, visual, "{text:?}");
}
fn levels(text: &str) -> Vec<Option<u8>> {
    text.split_whitespace()
        .map(|x| {
            if x == "x" {
                None
            } else {
                Some(x.parse().unwrap())
            }
        })
        .collect()
}
fn visual(text: &str) -> Vec<usize> {
    text.split_whitespace()
        .map(|x| x.parse().unwrap())
        .collect()
}
#[test]
#[ignore = "requires TYPAXIS_UCD_TEST_DIR with official Unicode 16 conformance files"]
fn official_unicode16_character_conformance() {
    let path = std::path::PathBuf::from(std::env::var_os("TYPAXIS_UCD_TEST_DIR").unwrap())
        .join("BidiCharacterTest.txt");
    let contents = fs::read_to_string(path).unwrap();
    assert!(contents.starts_with("# BidiCharacterTest-16.0.0.txt"));
    let mut cases = 0;
    for (line_number, line) in contents.lines().enumerate() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let fields = line.split(';').map(str::trim).collect::<Vec<_>>();
        assert_eq!(fields.len(), 5, "line={line_number}");
        let text = fields[0]
            .split_whitespace()
            .map(|v| char::from_u32(u32::from_str_radix(v, 16).unwrap()).unwrap())
            .collect::<String>();
        let base = match fields[1] {
            "0" => Some(Level::ltr()),
            "1" => Some(Level::rtl()),
            "2" => None,
            _ => panic!("direction"),
        };
        official_case(
            &text,
            base,
            &levels(fields[3]),
            &visual(fields[4]),
            Some(fields[2].parse().unwrap()),
        );
        cases += 1;
    }
    assert!(cases > 90_000);
    eprintln!("official Unicode16 BidiCharacterTest: {cases} cases");
}
fn class_character(class: &str) -> char {
    match class {
        "L" => 'a',
        "R" => 'א',
        "AL" => 'ا',
        "EN" => '1',
        "ES" => '+',
        "ET" => '$',
        "AN" => '١',
        "CS" => ',',
        "NSM" => '\u{0300}',
        "BN" => '\u{00ad}',
        "B" => '\u{2029}',
        "S" => '\t',
        "WS" => ' ',
        "ON" => '\u{fffc}',
        "LRE" => '\u{202a}',
        "RLE" => '\u{202b}',
        "PDF" => '\u{202c}',
        "LRO" => '\u{202d}',
        "RLO" => '\u{202e}',
        "LRI" => '\u{2066}',
        "RLI" => '\u{2067}',
        "FSI" => '\u{2068}',
        "PDI" => '\u{2069}',
        _ => panic!("{}", class),
    }
}
#[test]
#[ignore = "requires TYPAXIS_UCD_TEST_DIR with official Unicode 16 conformance files"]
fn official_unicode16_class_conformance() {
    let path = std::path::PathBuf::from(std::env::var_os("TYPAXIS_UCD_TEST_DIR").unwrap())
        .join("BidiTest.txt");
    let contents = fs::read_to_string(path).unwrap();
    assert!(contents.starts_with("# BidiTest-16.0.0.txt"));
    let (mut expected, mut order, mut cases) = (Vec::new(), Vec::new(), 0);
    for line in contents.lines() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        if let Some(v) = line.strip_prefix("@Levels:") {
            expected = levels(v);
            continue;
        }
        if let Some(v) = line.strip_prefix("@Reorder:") {
            order = visual(v);
            continue;
        }
        if line.starts_with('@') {
            continue;
        }
        let fields = line.split(';').map(str::trim).collect::<Vec<_>>();
        let text = fields[0]
            .split_whitespace()
            .map(class_character)
            .collect::<String>();
        let modes = u8::from_str_radix(fields[1], 16).unwrap();
        for (mask, base) in [(1, None), (2, Some(Level::ltr())), (4, Some(Level::rtl()))] {
            if modes & mask != 0 {
                official_case(&text, base, &expected, &order, None);
                cases += 1;
            }
        }
    }
    assert!(cases > 700_000);
    eprintln!("official Unicode16 BidiTest: {cases} direction cases");
}
