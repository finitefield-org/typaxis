use super::*;
use typaxis_core::{Length, PositiveLength};
use typaxis_layout::book_v2::{layout_book_v2_text_lines, prepare_book_v2_text_inlines};
use typaxis_layout::ProductionPlacedInline;
use typaxis_linebreak::JapaneseLineBreakMode;

pub(super) fn check_actual_lines(
    flow: &typaxis_syntax::book_v2::PreparedBookV2TextFlow<'_>,
    shaped: &typaxis_shaping::book_v2::BookV2AuthoredTextShape<'_>,
    admitted: &typaxis_resources::AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    expected: &str,
    width: i64,
) -> typaxis_layout::ProductionSelectedLineContexts {
    let prepared = prepare_book_v2_text_inlines(
        flow,
        shaped,
        admitted,
        limits,
        EPOCH,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    let width = PositiveLength::new(Length::from_raw(width).unwrap()).unwrap();
    let selected = layout_book_v2_text_lines(&prepared, &[width], 1_000_000).unwrap();
    assert!(selected.paragraphs()[0].lines().len() > 1);
    assert!(selected.candidate_steps() > 0);
    assert!(selected.output_records() > 0);
    assert!(layout_book_v2_text_lines(&prepared, &[width], selected.candidate_steps()).is_ok());
    assert!(
        layout_book_v2_text_lines(&prepared, &[width], selected.candidate_steps() - 1).is_err()
    );

    selected.verify(&prepared).unwrap();
    prepared.verify(flow, shaped).unwrap();
    let mut actual = String::new();
    for p in selected.paragraphs() {
        for line in p.lines() {
            for item in line.items() {
                if let ProductionPlacedInline::Text(cluster) = item {
                    actual.push_str(cluster.utf8());
                    assert!(matches!(cluster.source_span(), ShapeSourceSpan::Parsed(_)));
                    for placed in cluster.glyphs() {
                        let original =
                            &cluster.run().glyph_run().glyphs[placed.glyph_index() as usize];
                        assert!(std::ptr::eq(original, placed.glyph()));
                        assert!(placed.glyph().original_gid.get() > 0);
                    }
                }
            }
        }
    }
    assert_eq!(actual, expected);
    let contexts = selected.selected_line_contexts().unwrap();
    assert_eq!(
        contexts.paragraphs()[0].ends().last().copied(),
        Some(expected.len() as u32)
    );
    assert_eq!(contexts.source_fingerprint(), selected.fingerprint());
    assert!(contexts.record_charge() > selected.output_records());
    assert!(layout_book_v2_text_lines(&prepared, &[], 1_000_000).is_err());
    assert!(layout_book_v2_text_lines(&prepared, &[width], 0).is_err());
    let other = prepare_book_v2_text_inlines(
        flow,
        shaped,
        admitted,
        limits,
        EPOCH,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    assert!(selected.verify(&other).is_err());
    assert!(prepare_book_v2_text_inlines(
        flow,
        shaped,
        admitted,
        limits,
        [20; 32],
        JapaneseLineBreakMode::Normal
    )
    .is_err());
    contexts
}

#[test]
fn book_v2_body_selects_source_glyph_lines_and_reshapes_actual_boundaries() {
    let root = Root::new();
    let limits = limits();
    let text = "Result Proof Result";
    let input = prepared(&root, source_data(text), text.as_bytes(), &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let initial =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
            .unwrap();
    let width = initial.paragraphs()[0]
        .runs()
        .iter()
        .flat_map(|r| &r.glyph_run().glyphs)
        .map(|g| g.advance_x.raw())
        .sum::<i64>()
        * 2
        / 3;
    let contexts = check_actual_lines(&flow, &initial, input.resources(), &limits, text, width);
    let ends = contexts
        .paragraphs()
        .iter()
        .map(|p| ProductionParagraphLineContext {
            owner: p.owner(),
            ends: p.ends(),
        })
        .collect::<Vec<_>>();
    let reshaped = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        EPOCH,
        Some(&ends),
    )
    .unwrap();
    let again = check_actual_lines(&flow, &reshaped, input.resources(), &limits, text, width);
    assert_eq!(
        contexts.paragraphs()[0].ends(),
        again.paragraphs()[0].ends()
    );
}
