use super::*;
use typaxis_core::Length;
use typaxis_layout::book_v2::{
    layout_book_v2_page_region_lines as layout_region,
    prepare_book_v2_page_region_inlines as inlines,
    with_converged_book_v2_page_region_lines as converge, BookV2PageRegionLayoutError as Error,
};
use typaxis_layout::ProductionPlacedInline;
use typaxis_linebreak::JapaneseLineBreakMode;
const MODE: JapaneseLineBreakMode = JapaneseLineBreakMode::Normal;

fn layout_data(text: &str, width: i64, height: i64, align: &str, multiple: bool) -> Value {
    let mut data = region_data(text);
    let master = &mut data["page_masters"]["masters"][0];
    master["body"]["y"] = (120 * 65536).into();
    master["body"]["height"] = (300 * 65536).into();
    for region in ["header", "footer"] {
        master[region]["width"] = width.into();
        master[region]["height"] = height.into();
    }
    if multiple {
        let mut block = master["footer_content"]["blocks"][0].clone();
        block["node_id"] = 13.into();
        block["children"][0]["node_id"] = 14.into();
        master["footer_content"]["blocks"]
            .as_array_mut()
            .unwrap()
            .push(block);
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    for selector in ["paragraph", "heading"] {
        rules.push(
            json!({"style_id":format!("region-layout-{selector}"),"selector":selector,
            "extends":null,"source_order":rules.len(),"declarations":[
                {"name":"start_indent","important":false,"value":{"kind":"length","value":2*65536}},
                {"name":"end_indent","important":false,"value":{"kind":"length","value":3*65536}},
                {"name":"space_before","important":false,"value":{"kind":"length","value":4*65536}},
                {"name":"space_after","important":false,"value":{"kind":"length","value":5*65536}},
                {"name":"text_align","important":false,"value":{"kind":"keyword","value":align}}
            ]}),
        );
    }
    data
}

#[test]
fn book_v2_page_region_lines_use_actual_width_spacing_and_original_glyphs() {
    for align in ["start", "center", "end"] {
        check_lines(None, "Result Result", align);
    }
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_page_region_lines_converge_original_harano_japanese() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_lines(Some(&font), "本文の柱", "center");
}
fn check_lines(font: Option<&[u8]>, text: &str, align: &str) {
    let mut counts = Vec::new();
    for width in [200, if font.is_some() { 35 } else { 55 }] {
        let root = Root::new();
        let limits = limits();
        let data = layout_data(text, width * 65536, 95 * 65536, align, true);
        let input = if let Some(font) = font {
            vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
        } else {
            prepared(&root, data, text.as_bytes(), &limits)
        };
        let body = input.body().styled();
        let nav = prepare_book_v2_navigation(body).unwrap();
        let selected = select_book_v2_page_master(body, 0, None, &mut 0, 1_000_000).unwrap();
        let flow = region_flow(selected, Kind::Footer, &nav, 0).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let shape = shape_region(&policy, &flow, input.resources(), &limits, EPOCH, None).unwrap();
        let prepared = inlines(&flow, &shape, input.resources(), &limits, EPOCH, MODE, 0).unwrap();
        let initial = layout_region(&prepared, selected, 1_000_000, 0).unwrap();
        initial.verify(&prepared, selected).unwrap();
        prepared.verify(&flow, &shape).unwrap();
        assert!(inlines(&flow, &shape, input.resources(), &limits, [0; 32], MODE, 0).is_err());
        let other = inlines(&flow, &shape, input.resources(), &limits, EPOCH, MODE, 0).unwrap();
        assert!(initial.verify(&other, selected).is_err());
        let next = select_book_v2_page_master(body, 1, None, &mut 0, 1_000_000).unwrap();
        assert!(initial.verify(&prepared, next).is_err());
        let next_lines = layout_region(&prepared, next, 1_000_000, 0).unwrap();
        assert_ne!(initial.fingerprint(), next_lines.fingerprint());
        assert_eq!(initial.origins(), next_lines.origins());
        assert!(layout_region(&prepared, selected, initial.candidate_steps(), 0).is_ok());
        assert!(layout_region(&prepared, selected, initial.candidate_steps() - 1, 0).is_err());
        let exact = limits.base().get().max_fragments - initial.output_records();
        assert_eq!(
            layout_region(&prepared, selected, 1_000_000, exact)
                .unwrap()
                .output_records(),
            limits.base().get().max_fragments
        );
        assert!(layout_region(&prepared, selected, 1_000_000, exact + 1).is_err());
        assert!(layout_region(&prepared, selected, 1_000_000, u64::MAX).is_err());
        let exact_inline = limits.base().get().max_fragments - prepared.record_charge();
        assert!(inlines(
            &flow,
            &shape,
            input.resources(),
            &limits,
            EPOCH,
            MODE,
            exact_inline
        )
        .is_ok());
        assert!(inlines(
            &flow,
            &shape,
            input.resources(),
            &limits,
            EPOCH,
            MODE,
            exact_inline + 1
        )
        .is_err());
        let (steps, passes, records) = converge(
            &policy,
            &flow,
            input.resources(),
            &limits,
            EPOCH,
            MODE,
            selected,
            1_000_000,
            limits.base().get().max_line_reshape_passes,
            0,
            |stable| {
                let lines = stable.lines();
                assert!(lines
                    .prepared()
                    .shaped()
                    .line_context_fingerprint()
                    .is_some());
                assert!(!stable.passes().is_empty());
                let mut actual = String::new();
                for p in lines.paragraphs() {
                    assert_eq!(p.inline_size().get().raw(), (width - 5) * 65536);
                    for line in p.lines() {
                        for item in line.items() {
                            if let ProductionPlacedInline::Text(c) = item {
                                actual.push_str(c.utf8());
                                assert!(matches!(c.source_span(), ShapeSourceSpan::Parsed(_)));
                                assert_eq!(c.run().language(), "und");
                                for g in c.glyphs() {
                                    assert!(std::ptr::eq(
                                        g.glyph(),
                                        &c.run().glyph_run().glyphs[g.glyph_index() as usize]
                                    ));
                                    assert_ne!(g.glyph().original_gid.get(), 0);
                                }
                            }
                        }
                    }
                }
                assert_eq!(actual, text.repeat(2));
                assert_eq!(
                    lines.origins().len(),
                    lines
                        .paragraphs()
                        .iter()
                        .map(|p| p.lines().len())
                        .sum::<usize>()
                );
                let mut cursor = 500 * 65536;
                let mut previous = 0;
                for origin in lines.origins() {
                    if origin.paragraph_index() != previous {
                        cursor += 9 * 65536;
                    }
                    previous = origin.paragraph_index();
                    assert_eq!(origin.y().raw(), cursor);
                    let p = &lines.paragraphs()[origin.paragraph_index() as usize];
                    let line = &p.selected().unwrap().lines()[origin.line_index() as usize];
                    let slack =
                        p.inline_size().get().raw() - line.required_inline_size().get().raw();
                    let offset = match align {
                        "start" => 0,
                        "end" => slack,
                        _ => slack / 2,
                    };
                    assert_eq!(origin.x().raw(), 12 * 65536 + offset);
                    assert_eq!(origin.height(), line.line().metrics().line_height());
                    cursor += origin.height().get().raw();
                }
                assert_eq!(lines.content_height().get().raw(), cursor - 500 * 65536);
                assert!(lines.content_height().get() <= lines.frame().height().get());
                counts.push(lines.origins().len());
                (
                    stable.candidate_steps(),
                    stable.passes().len() as u16,
                    lines.output_records(),
                )
            },
        )
        .unwrap();
        assert!(converge(
            &policy,
            &flow,
            input.resources(),
            &limits,
            EPOCH,
            MODE,
            selected,
            steps,
            passes,
            0,
            |_| ()
        )
        .is_ok());
        assert!(converge(
            &policy,
            &flow,
            input.resources(),
            &limits,
            EPOCH,
            MODE,
            selected,
            steps - 1,
            passes,
            0,
            |_| panic!("budget must reject")
        )
        .is_err());
        assert!(converge(
            &policy,
            &flow,
            input.resources(),
            &limits,
            EPOCH,
            MODE,
            selected,
            steps,
            passes - 1,
            0,
            |_| panic!("pass limit must reject")
        )
        .is_err());
        eprintln!("region lines: font={} width={width} align={align} candidates={steps} passes={passes} records={records}", if font.is_some(){"Harano"}else{"TT"});
    }
    assert!(
        counts[1] > counts[0],
        "physical width must alter actual line selection"
    );
}

#[test]
fn book_v2_page_region_lines_reject_overflow_without_clipping_or_callback() {
    let mut needed = 0;
    for height in [95 * 65536, 16 * 65536, 16 * 65536 - 1] {
        let root = Root::new();
        let limits = limits();
        let data = layout_data("Result", 200 * 65536, height, "start", false);
        let input = prepared(&root, data, b"Result", &limits);
        let body = input.body().styled();
        let nav = prepare_book_v2_navigation(body).unwrap();
        let selected = select_book_v2_page_master(body, 0, None, &mut 0, 1_000_000).unwrap();
        let flow = region_flow(selected, Kind::Footer, &nav, 0).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let result = converge(
            &policy,
            &flow,
            input.resources(),
            &limits,
            EPOCH,
            MODE,
            selected,
            1_000_000,
            limits.base().get().max_line_reshape_passes,
            0,
            |stable| {
                assert!(height >= 16 * 65536);
                stable.lines().content_height().get().raw()
            },
        );
        if height >= 16 * 65536 {
            needed = result.unwrap();
            assert_eq!(needed, 16 * 65536);
        } else {
            assert!(
                matches!(result, Err(Error::Overflow { owner, required, available })
                if owner == NodeId::new(10) && required == Length::from_raw(needed).unwrap() && available.get().raw() == height)
            );
        }
    }
}

#[test]
fn book_v2_page_region_lines_preserve_hard_breaks_and_empty_paragraphs() {
    for empty in [false, true] {
        let root = Root::new();
        let limits = limits();
        let mut data = layout_data("Result", 55 * 65536, 95 * 65536, "end", false);
        let header = &mut data["page_masters"]["masters"][0]["header_content"]["blocks"][0];
        if empty {
            header["children"] = json!([]);
        } else {
            header["children"][1]["kind"] = "hard_break".into();
        }
        let input = prepared(&root, data, b"Result", &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let selected =
            select_book_v2_page_master(input.body().styled(), 0, None, &mut 0, 1_000_000).unwrap();
        let flow = region_flow(selected, Kind::Header, &nav, 0).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        converge(
            &policy,
            &flow,
            input.resources(),
            &limits,
            EPOCH,
            MODE,
            selected,
            1_000_000,
            limits.base().get().max_line_reshape_passes,
            0,
            |stable| {
                let lines = stable.lines();
                assert_eq!(lines.paragraphs()[0].owner(), NodeId::new(5));
                assert_eq!(lines.origins().len(), if empty { 1 } else { 2 });
                assert_eq!(
                    lines.content_height().get().raw(),
                    if empty { 16 * 65536 } else { 32 * 65536 }
                );
                assert_eq!(lines.origins()[0].y(), Length::ZERO);
                if empty {
                    assert_eq!(lines.origins()[0].x().raw(), 12 * 65536);
                    assert!(lines.paragraphs()[0].lines()[0].items().is_empty());
                } else {
                    let breaks: Vec<_> = lines.paragraphs()[0]
                        .lines()
                        .iter()
                        .flat_map(|l| l.items())
                        .filter_map(|i| {
                            if let ProductionPlacedInline::Break(b) = i {
                                Some(b.owner().get())
                            } else {
                                None
                            }
                        })
                        .collect();
                    assert_eq!(breaks, vec![7, 9]);
                }
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_page_region_lines_reject_exhausted_indents_and_unbreakable_text() {
    for width in [5 * 65536, 6 * 65536] {
        let root = Root::new();
        let limits = limits();
        let data = layout_data("Result", width, 95 * 65536, "start", false);
        let input = prepared(&root, data, b"Result", &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let selected =
            select_book_v2_page_master(input.body().styled(), 0, None, &mut 0, 1_000_000).unwrap();
        let flow = region_flow(selected, Kind::Footer, &nav, 0).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let result = converge(
            &policy,
            &flow,
            input.resources(),
            &limits,
            EPOCH,
            MODE,
            selected,
            1_000_000,
            limits.base().get().max_line_reshape_passes,
            0,
            |_| panic!("invalid frame must reject"),
        );
        if width == 5 * 65536 {
            assert!(matches!(result,Err(Error::Geometry{owner}) if owner==NodeId::new(11)));
        } else {
            assert!(
                matches!(result,Err(Error::Inline(typaxis_layout::ProductionInlinePreparationError {
            owner,kind:typaxis_layout::ProductionInlinePreparationErrorKind::Atomic(typaxis_linebreak::AtomicVectorInlineError::NoFeasibleLine)
        })) if owner==NodeId::new(11))
            );
        }
    }
}
