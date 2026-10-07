use super::*;
use typaxis_core::{Length, PositiveLength};
use typaxis_layout::book_v2::{
    layout_book_v2_inline_lines_with_source_widths, layout_book_v2_source_width_lines_charged,
    prepare_book_v2_text_inlines,
};
use typaxis_linebreak::{
    JapaneseLineBreakMode, ProductionInlineLogicalUnit as Unit, ProductionInlineSourceWidths,
};

fn verify_widths(text: &str, font: Option<&[u8]>) {
    let root = Root::new();
    let limits = limits();
    let mut data = source_data(text);
    data["document"]["footnotes"] = json!([]);
    if let Some(font) = font {
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            .into();
    }
    let input = if let Some(font) = font {
        let body = body_with_source(&root, data, text.as_bytes(), &limits);
        fs::write(root.0.join("body.bin"), font).unwrap();
        prepare_book_v2_resources(
            body,
            &root.context(),
            &config(limits.base().get().clone()),
            &limits,
        )
        .unwrap()
    } else {
        prepared(&root, data, text.as_bytes(), &limits)
    };
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let shape =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
            .unwrap();
    let inlines = prepare_book_v2_text_inlines(
        &flow,
        &shape,
        input.resources(),
        &limits,
        EPOCH,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    assert_eq!(inlines.paragraphs().len(), 1);
    let items = inlines.paragraphs()[0].items().unwrap();
    if text.starts_with('か') {
        assert!(inlines.paragraphs()[0]
            .glyph_clusters()
            .iter()
            .all(|c| c.end_unit() - c.start_unit() == 2));
    }
    let advance = |i| match items.units()[i] {
        Unit::Text(t) => t.advance().get(),
        _ => panic!("text source"),
    };
    let small = advance(0).checked_add(advance(1)).unwrap();
    let large = small.checked_add(small).unwrap();
    let positive = |n| PositiveLength::new(n).unwrap();
    let mut sizes = vec![positive(large); items.units().len()];
    sizes[0] = positive(small);
    let profiles = [Some(
        ProductionInlineSourceWidths::new(items, &sizes).unwrap(),
    )];
    let envelope = [positive(large)];
    let full =
        layout_book_v2_inline_lines_with_source_widths(&inlines, &envelope, &profiles, 1_000_000)
            .unwrap();
    assert!(full.frames().is_none());
    let p = &full.paragraphs()[0];
    assert!(p.lines().len() > 1);
    assert_eq!(p.line_inline_size(0), Some(positive(small)));
    assert_eq!(p.line_inline_size(1), Some(positive(large)));
    let mut actual = String::new();
    for line in p.lines() {
        for item in line.items() {
            let typaxis_layout::ProductionPlacedInline::Text(cluster) = item else {
                panic!("original text")
            };
            actual.push_str(cluster.utf8());
            assert!(matches!(
                cluster.source_span(),
                typaxis_shaping::ShapeSourceSpan::Parsed(_)
            ));
            for glyph in cluster.glyphs() {
                assert!(std::ptr::eq(
                    glyph.glyph(),
                    &cluster.run().glyph_run().glyphs[glyph.glyph_index() as usize]
                ));
                assert!(glyph.glyph().original_gid.get() > 0);
            }
        }
    }
    assert_eq!(actual, text);
    full.verify(&inlines).unwrap();
    // Widths survive a fresh shaping/preparation by binding to the immutable
    // source flow, while the direct item-owner profile above remains exact.
    use typaxis_layout::book_v2::{
        bind_book_v2_vectors, layout_book_v2_source_width_lines_from_flow,
        with_converged_book_v2_body_lines_with_source_widths, BookV2SourceWidthAssignments,
    };
    let assignment_slices = [Some(sizes.as_slice())];
    let assignments = BookV2SourceWidthAssignments::new(&flow, &assignment_slices).unwrap();
    let rebound = layout_book_v2_source_width_lines_from_flow(
        &inlines,
        &envelope,
        &assignments,
        1_000_000,
        0,
    )
    .unwrap();
    assert_eq!(rebound.fingerprint(), full.fingerprint());
    assert_eq!(rebound.output_records(), full.output_records());
    assert!(rebound.candidate_steps() > full.candidate_steps());
    let prior = limits.base().get().max_fragments - rebound.output_records();
    let exact = layout_book_v2_source_width_lines_from_flow(
        &inlines,
        &envelope,
        &assignments,
        rebound.candidate_steps(),
        prior,
    )
    .unwrap();
    assert_eq!(exact.output_records(), limits.base().get().max_fragments);
    assert!(layout_book_v2_source_width_lines_from_flow(
        &inlines,
        &envelope,
        &assignments,
        rebound.candidate_steps(),
        prior + 1,
    )
    .is_err());
    assert!(layout_book_v2_source_width_lines_from_flow(
        &inlines,
        &envelope,
        &assignments,
        rebound.candidate_steps() - 1,
        0,
    )
    .is_err());
    let other_flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let foreign_assignments =
        BookV2SourceWidthAssignments::new(&other_flow, &assignment_slices).unwrap();
    assert!(matches!(layout_book_v2_source_width_lines_from_flow(
        &inlines, &envelope, &foreign_assignments, 1_000_000, 0,
    ), Err(e) if e.kind == typaxis_layout::ProductionInlinePreparationErrorKind::ReceiptMismatch));
    assert!(BookV2SourceWidthAssignments::new(&flow, &[]).is_err());
    let malformed = [Some(&sizes[..sizes.len() - 1])];
    assert!(layout_book_v2_source_width_lines_from_flow(
        &inlines,
        &envelope,
        &BookV2SourceWidthAssignments::new(&flow, &malformed).unwrap(),
        1_000_000,
        0
    )
    .is_err());
    let binding = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let body = typaxis_core::Rect::new(
        Length::ZERO,
        Length::ZERO,
        positive(Length::from_raw(1000 * 65536).unwrap()),
        positive(Length::from_raw(1000 * 65536).unwrap()),
    );
    let run = |work, passes| {
        with_converged_book_v2_body_lines_with_source_widths(
            &policy,
            &flow,
            input.resources(),
            &binding,
            &limits,
            JapaneseLineBreakMode::Normal,
            body,
            work,
            None,
            passes,
            None,
            Some(&assignments),
            |stable| {
                assert!(stable.passes().last().unwrap().is_stable());
                let selected = stable.lines().paragraphs()[0].selected().unwrap();
                let ends = selected
                    .lines()
                    .iter()
                    .map(|l| l.line().end_unit())
                    .collect::<Vec<_>>();
                assert_eq!(
                    ends,
                    p.selected()
                        .unwrap()
                        .lines()
                        .iter()
                        .map(|l| l.line().end_unit())
                        .collect::<Vec<_>>()
                );
                let mut original = String::new();
                for line in stable.lines().paragraphs()[0].lines() {
                    for item in line.items() {
                        let typaxis_layout::ProductionPlacedInline::Text(c) = item else {
                            panic!("text source")
                        };
                        original.push_str(c.utf8());
                        for g in c.glyphs() {
                            assert!(std::ptr::eq(
                                g.glyph(),
                                &c.run().glyph_run().glyphs[g.glyph_index() as usize]
                            ));
                        }
                    }
                }
                assert_eq!(original, text);
                (
                    stable.candidate_steps(),
                    stable.passes().len() as u16,
                    stable.lines().fingerprint(),
                    ends,
                )
            },
        )
    };
    let reshape = run(1_000_000, limits.base().get().max_line_reshape_passes).unwrap();
    let exact_reshape = run(reshape.0, reshape.1).unwrap();
    assert_eq!(exact_reshape, reshape);
    assert!(run(reshape.0 - 1, reshape.1).is_err());
    assert!(run(reshape.0, reshape.1 - 1).is_err());

    let selected = p.selected().unwrap();
    assert_eq!(selected.lines()[0].line().end_unit(), 2);
    assert_eq!(
        selected.lines().last().unwrap().line().end_unit() as usize,
        items.units().len()
    );
    assert_eq!(full.selected_line_contexts().unwrap().paragraphs().len(), 1);
    let fixed =
        typaxis_layout::book_v2::layout_book_v2_text_lines(&inlines, &envelope, 1_000_000).unwrap();
    let ends = |p: &typaxis_layout::ProductionInlineParagraphLineLayout<'_, '_>| {
        p.selected()
            .unwrap()
            .lines()
            .iter()
            .map(|l| l.line().end_unit())
            .collect::<Vec<_>>()
    };
    assert_ne!(ends(p), ends(&fixed.paragraphs()[0]));
    if let Some(directory) = std::env::var_os("TYPAXIS_BOOK_V2_SOURCE_WIDTH_PROBE") {
        let directory = std::path::PathBuf::from(directory);
        fs::create_dir_all(&directory).unwrap();
        let proof = json!({"text":text,"font_sha256":font.map(typaxis_core::sha256),
            "fixed":serde_json::from_str::<Value>(fixed.paragraphs()[0].selected().unwrap().canonical_jcs()).unwrap(),
            "variable":serde_json::from_str::<Value>(selected.canonical_jcs()).unwrap(),
            "records":full.output_records(),"work":full.candidate_steps(),
            "reshape":{"work":reshape.0,"passes":reshape.1,"ends":reshape.3},
            "clusters":p.lines().iter().map(|line|line.items().iter().map(|item|{
                let typaxis_layout::ProductionPlacedInline::Text(cluster)=item else {unreachable!()};
                json!({"text":cluster.utf8(),"pen_x":cluster.pen_x().raw(),"glyphs":cluster.glyphs().iter().map(|g|g.glyph().original_gid.get()).collect::<Vec<_>>()})
            }).collect::<Vec<_>>()).collect::<Vec<_>>()});
        fs::write(
            directory.join(if text.starts_with('か') {
                "harano-graphemes.json"
            } else if font.is_some() {
                "harano.json"
            } else {
                "controlled.json"
            }),
            serde_json::to_vec_pretty(&proof).unwrap(),
        )
        .unwrap();
    }
    let exact = layout_book_v2_source_width_lines_charged(
        &inlines,
        &envelope,
        &profiles,
        full.candidate_steps(),
        limits.base().get().max_fragments - full.output_records(),
    )
    .unwrap();
    assert_eq!(exact.output_records(), limits.base().get().max_fragments);
    assert_eq!(exact.fingerprint(), full.fingerprint());
    assert!(layout_book_v2_source_width_lines_charged(
        &inlines,
        &envelope,
        &profiles,
        full.candidate_steps(),
        limits.base().get().max_fragments - full.output_records() + 1
    )
    .is_err());
    assert!(layout_book_v2_inline_lines_with_source_widths(
        &inlines,
        &envelope,
        &profiles,
        full.candidate_steps() - 1
    )
    .is_err());
    assert!(
        layout_book_v2_inline_lines_with_source_widths(&inlines, &[], &profiles, 1_000_000)
            .is_err()
    );
    assert!(
        layout_book_v2_inline_lines_with_source_widths(&inlines, &envelope, &[], 1_000_000)
            .is_err()
    );
    let foreign = prepare_book_v2_text_inlines(
        &flow,
        &shape,
        input.resources(),
        &limits,
        EPOCH,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    let other = [Some(
        ProductionInlineSourceWidths::new(foreign.paragraphs()[0].items().unwrap(), &sizes)
            .unwrap(),
    )];
    assert!(
        matches!(layout_book_v2_inline_lines_with_source_widths(&inlines,&envelope,&other,1_000_000),Err(e) if e.kind==typaxis_layout::ProductionInlinePreparationErrorKind::ReceiptMismatch)
    );
    let too_narrow = [positive(Length::from_raw(small.raw() - 1).unwrap())];
    assert!(layout_book_v2_inline_lines_with_source_widths(
        &inlines,
        &too_narrow,
        &profiles,
        1_000_000
    )
    .is_err());
}
#[test]
fn book_v2_source_widths_reselect_real_glyphs_and_retain_exact_preparation_and_limits() {
    verify_widths("R R R R R R", None);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_source_widths_reselect_original_harano_clusters() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    verify_widths("左側右側左側右側左側右側", Some(&font));
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_source_widths_rebind_multiscalar_harano_graphemes_through_reshape() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    verify_widths(
        "か\u{3099}か\u{3099}か\u{3099}か\u{3099}か\u{3099}か\u{3099}",
        Some(&font),
    );
}

// Exercise rebinding over complete mixed source paragraphs and the None fallback.
pub(super) fn verify_mixed_source_rebinding(
    input: &PreparedBookV2Resources,
    body: typaxis_core::Rect,
    limits: &M4EffectiveResourceLimits,
) {
    use typaxis_layout::book_v2::*;
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), limits).unwrap();
    let shape = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let prepared = prepare_book_v2_inline_items(
        &flow,
        &shape,
        input.resources(),
        &bindings,
        limits,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    let frames = prepare_book_v2_body_inline_frames(&prepared, body).unwrap();
    let envelopes = frames
        .paragraphs()
        .iter()
        .map(|f| f.width())
        .collect::<Vec<_>>();
    let sizes = prepared
        .paragraphs()
        .iter()
        .zip(&envelopes)
        .map(|(p, w)| vec![*w; p.items().unwrap().units().len().max(1)])
        .collect::<Vec<_>>();
    let expected = layout_book_v2_inline_lines(&prepared, &envelopes, 1_000_000).unwrap();
    for sparse in [false, true] {
        let profiles = sizes
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if sparse && i % 2 == 0 {
                    None
                } else {
                    Some(s.as_slice())
                }
            })
            .collect::<Vec<_>>();
        let assignments = BookV2SourceWidthAssignments::new(&flow, &profiles).unwrap();
        let lines = layout_book_v2_source_width_lines_from_flow(
            &prepared,
            &envelopes,
            &assignments,
            1_000_000,
            0,
        )
        .unwrap();
        for (actual, expected) in lines.paragraphs().iter().zip(expected.paragraphs()) {
            assert_eq!(
                actual
                    .selected()
                    .unwrap()
                    .lines()
                    .iter()
                    .map(|l| (
                        l.line().start_unit(),
                        l.line().end_unit(),
                        l.required_inline_size()
                    ))
                    .collect::<Vec<_>>(),
                expected
                    .selected()
                    .unwrap()
                    .lines()
                    .iter()
                    .map(|l| (
                        l.line().start_unit(),
                        l.line().end_unit(),
                        l.required_inline_size()
                    ))
                    .collect::<Vec<_>>()
            );
        }
        with_converged_book_v2_body_lines_with_source_widths(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            limits,
            JapaneseLineBreakMode::Normal,
            body,
            1_000_000,
            None,
            limits.base().get().max_line_reshape_passes,
            None,
            Some(&assignments),
            |stable| {
                assert!(stable.passes().last().unwrap().is_stable());
                assert_eq!(
                    stable.lines().paragraphs().len(),
                    expected.paragraphs().len()
                );
                for (actual, expected) in stable
                    .lines()
                    .paragraphs()
                    .iter()
                    .zip(expected.paragraphs())
                {
                    assert_eq!(
                        actual
                            .selected()
                            .unwrap()
                            .lines()
                            .iter()
                            .map(|l| l.line().end_unit())
                            .collect::<Vec<_>>(),
                        expected
                            .selected()
                            .unwrap()
                            .lines()
                            .iter()
                            .map(|l| l.line().end_unit())
                            .collect::<Vec<_>>()
                    );
                }
            },
        )
        .unwrap();
    }
}
