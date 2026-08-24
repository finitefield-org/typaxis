use std::char;

use typaxis_linebreak::unicode_line_breaks;

const LINE_BREAK_TEST: &str =
    include_str!("../../../../third_party/unicode/16.0.0/LineBreakTest.txt");

#[test]
fn default_uax14_conformance_unicode_16() {
    let mut case_count = 0usize;
    for (line_number, raw_line) in LINE_BREAK_TEST.lines().enumerate() {
        let test = raw_line
            .split('#')
            .next()
            .expect("split always yields one field")
            .trim();
        if test.is_empty() {
            continue;
        }
        case_count += 1;
        let mut tokens = test.split_whitespace();
        assert_eq!(
            tokens.next(),
            Some("×"),
            "line {} start marker",
            line_number + 1
        );

        let mut text = String::new();
        let mut expected = Vec::new();
        while let Some(codepoint) = tokens.next() {
            let codepoint = u32::from_str_radix(codepoint, 16)
                .unwrap_or_else(|_| panic!("line {} code point", line_number + 1));
            let scalar = char::from_u32(codepoint)
                .unwrap_or_else(|| panic!("line {} Unicode scalar", line_number + 1));
            text.push(scalar);
            match tokens.next() {
                Some("÷") => expected.push(text.len()),
                Some("×") => {}
                marker => panic!("line {} invalid marker {marker:?}", line_number + 1),
            }
        }

        let actual = unicode_line_breaks(&text)
            .unwrap_or_else(|_| panic!("line {} allocation", line_number + 1))
            .into_iter()
            .map(|boundary| boundary.byte_offset())
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "line {}: {raw_line}", line_number + 1);
    }
    assert_eq!(case_count, 16_672);
}
