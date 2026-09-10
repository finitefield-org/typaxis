fn production_container_indent(value: &mut serde_json::Value, start: i64, end: i64) {
    production_body_set_style(value, "semantic_container", "start_indent", start.into());
    production_body_set_style(value, "semantic_container", "end_indent", end.into());
}

#[test]
fn production_body_nested_container_frames_reflow_and_restore_source_order() {
    let mut value = production_body_fixture(3_000_000);
    production_container_indent(&mut value, 131_072, 65_536);
    let outer = value["document"]["blocks"][0].clone();
    let mut inner = outer.clone();
    inner["blocks"].as_array_mut().unwrap().pop();
    value["document"]["blocks"][0]["blocks"] = serde_json::json!([inner, outer["blocks"][2]]);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_structure_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, semantics, profile| {
            let body = blocks.page_geometry().body();
            let frames = lines.frames().unwrap();
            let nested = frames.paragraphs()[0];
            let following = frames.paragraphs()[1];
            assert_eq!(nested.start().raw(), 2 * 131_072 + 65_536);
            assert_eq!(following.start().raw(), 131_072 + 65_536);
            assert_eq!(
                nested.width().get().raw(),
                4_000_000 - 2 * (131_072 + 65_536) - 2 * 65_536
            );
            assert_eq!(
                following.width().get().raw(),
                4_000_000 - (131_072 + 65_536) - 2 * 65_536
            );
            for (p, frame) in lines.paragraphs().iter().zip(frames.paragraphs()) {
                assert_eq!(p.inline_size(), frame.width());
            }
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            assert_eq!(selected.fragments().len(), 3);
            let formula = selected.fragments()[1];
            assert_eq!(formula.bounds().x().raw(), body.x().raw() + 2 * 131_072);
            assert_eq!(
                formula.bounds().width().get().raw(),
                4_000_000 - 2 * (131_072 + 65_536)
            );
            let viewport = formula.viewport().unwrap();
            assert_eq!(
                viewport.x().raw(),
                formula.bounds().x().raw()
                    + (formula.bounds().width().get().raw() - viewport.width().get().raw()) / 2
            );
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
            let content =
                typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
            let structure = typaxis_display_list::build_production_body_structure(
                &display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .unwrap();
            let marked = typaxis_pdf::build_production_body_marked_content(
                &content, &structure, admitted, limits,
            )
            .unwrap();
            let objects =
                typaxis_pdf::build_production_body_objects(&marked, admitted, limits).unwrap();
            let pdf =
                typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
            assert!(pdf.bytes().starts_with(b"%PDF-"));
            assert_eq!(content.vectors().usages().len(), 2);
        },
    );
}

#[test]
fn production_body_container_raster_caption_and_list_share_the_parent_frame() {
    let mut value = production_raster_fixture("book-venn.png", 1_000_000, 3_500_000);
    production_container_indent(&mut value, 262_144, 131_072);
    let paragraph = value["document"]["blocks"][0]["blocks"][3].clone();
    value["document"]["blocks"][0]["blocks"][3] = serde_json::json!({
        "kind":"list", "node_id":0,"span":paragraph["span"],"classes":[],"ordered":true,"start":1,
        "items":[{"node_id":0,"span":paragraph["span"],"blocks":[paragraph]}]
    });
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let body = blocks.page_geometry().body();
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let raster = selected
            .fragments()
            .iter()
            .find(|f| {
                matches!(
                    f.source(),
                    typaxis_pagination::ProductionBodyFragmentSource::Figure { .. }
                )
            })
            .unwrap();
        let figure_style = lines.figures()[0].source().style().block_style();
        assert_eq!(
            raster.bounds().x().raw(),
            body.x().raw() + 262_144 + figure_style.start_indent().get().raw()
        );
        let frame = lines.frames().unwrap();
        let list = &frame.lists()[0];
        assert!(list.marker_start().raw() >= 262_144);
        assert!(list.content().width().get().raw() < 4_000_000 - 262_144 - 131_072);
        assert_eq!(selected.list_markers().len(), 1);
        let caption = &lines.source_flow().paragraphs()[1];
        assert_eq!(
            frame.region(caption.owner()).unwrap().start().raw(),
            262_144
        );
    });
    // The figure fits the body but cannot fit inside the indented container.
    production_body_set_style(&mut value, "figure", "width", 3_700_000.into());
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let error = typaxis_pagination::paginate_production_body(lines, blocks, limits)
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch
        );
        assert_eq!(error.owner, lines.figures()[0].owner());
    });
}

#[test]
fn production_body_container_indents_require_measured_frames_and_bounded_width() {
    for (start, exhausted) in [(131_072, false), (4_000_000, true)] {
        let mut value = production_body_fixture(3_000_000);
        production_container_indent(&mut value, start, 0);
        with_production_inline_tagged_context(
            &serde_json::to_vec(&value).unwrap(),
            &config(),
            |prepared, package, profile, limits, admitted, bindings, _, _| {
                let math = typaxis_layout::prepare_staging_math_vector_flows(
                    package, profile, limits, admitted, bindings,
                )
                .unwrap();
                let blocks = typaxis_layout::prepare_staging_precomposed_vector_blocks(
                    package, profile, limits, admitted, bindings, &math,
                )
                .unwrap();
                let body = blocks.page_geometry().body();
                let result =
                    typaxis_layout::layout_production_body_inline_lines(prepared, body, 100_000);
                if exhausted {
                    let error = result.err().unwrap();
                    assert_eq!(error.kind, typaxis_layout::ProductionInlinePreparationErrorKind::ContainerFrameExhausted);
                    assert_eq!(error.owner.get(), 1);
                } else {
                    let measured = result.unwrap();
                    assert!(typaxis_pagination::paginate_production_body(
                        &measured, &blocks, limits
                    )
                    .is_ok());
                    let widths = vec![body.width(); prepared.source_flow().paragraphs().len()];
                    let unframed =
                        typaxis_layout::layout_production_inline_lines(prepared, &widths, 100_000)
                            .unwrap();
                    let error =
                        typaxis_pagination::paginate_production_body(&unframed, &blocks, limits)
                            .err()
                            .unwrap();
                    assert_eq!(error.kind, typaxis_pagination::ProductionBodyPaginationErrorKind::PendingContainerIndent);
                }
            },
        );
    }
}

#[test]
fn production_body_container_width_changes_actual_line_breaks() {
    let mut value: serde_json::Value = serde_json::from_slice(&production_text_single_paragraph(
        &["A A A A A A A A"],
        "Body",
    ))
    .unwrap();
    value["page_masters"]["masters"][0]["body"]["width"] = 4_000_000.into();
    value["page_masters"]["masters"][0]["body"]["height"] = 3_000_000.into();
    value["page_masters"]["masters"][0]["footnote"] = serde_json::Value::Null;
    let mut line_counts = Vec::new();
    for indent in [0, 1_000_000] {
        production_container_indent(&mut value, indent, indent);
        with_production_body_resources(&value, &config(), |lines, blocks, limits, admitted| {
            line_counts.push(lines.paragraphs()[0].lines().len());
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let text = display
                .draws()
                .iter()
                .filter_map(|draw| match draw {
                    typaxis_display_list::ProductionBodyDraw::Text(text) => Some(text.exact_text()),
                    _ => None,
                })
                .collect::<String>();
            assert_eq!(text, "A A A A A A A A");
            for fragment in selected.fragments() {
                assert!(
                    fragment.bounds().x().raw() >= blocks.page_geometry().body().x().raw() + indent
                );
            }
        });
    }
    assert!(
        line_counts[1] > line_counts[0],
        "narrower actual frame must reflow: {line_counts:?}"
    );
}
