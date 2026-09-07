fn production_body_fixture(height: i64) -> serde_json::Value {
    use serde_json::json;
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_inline_vmb_fixture(true)).unwrap();
    let index: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/vmb-book/engine-v2/fixture-index.json"))).unwrap();
    let case = &index["cases"][2];
    assert_eq!(case["derived_svg"], "fraction-block-720896.svg");
    let mut resource = value["resources"]["images"][2].clone();
    resource["image_id"] = 4.into();
    resource["uri"] = "vmb-block-fraction.svg".into();
    resource["expected_sha256"] = case["derived_sha256"].clone();
    value["resources"]["images"]
        .as_array_mut()
        .unwrap()
        .push(resource);
    let tex = case["tex"].as_str().unwrap();
    let n = tex.len();
    let span = json!({"source_id":0,"start_byte":n,"end_byte":2*n});
    let end = json!({"source_id":0,"start_byte":2*n,"end_byte":2*n});
    value["sources"][0]["utf8_byte_length"] = (2 * n).into();
    value["sources"][0]["sha256"] = sha256(format!("{tex}{tex}").as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    value["text_buffers"].as_array_mut().unwrap().push(json!({
        "text_id":3,"utf8":tex,"mappings":[{"kind":"identity","source_span":span,
        "text_range":{"start_byte":0,"end_byte":n}}]
    }));
    value["document"]["blocks"][0]["span"]["end_byte"] = (2 * n).into();
    let children = value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    children.push(json!({"kind":"math_vector_block","node_id":6,"span":span,"classes":[],
        "image_id":4,"metrics":case["metrics"],"source_tex":{"text_span":{"text_id":3,"start_byte":0,"end_byte":n}},
        "alt":case["speech"],"actual_text":null,"equation_number":null}));
    children.push(json!({"kind":"paragraph","node_id":7,"span":end,"classes":[],"children":[{
        "kind":"text","node_id":8,"span":end,"text_span":{"text_id":1,"start_byte":0,"end_byte":1}}]}));
    let master = &mut value["page_masters"]["masters"][0];
    master["body"]["width"] = 4_000_000.into();
    master["body"]["height"] = height.into();
    master["footnote"] = serde_json::Value::Null;
    value
}

fn with_production_body_inputs(
    value: &serde_json::Value,
    config: &EffectiveConfig,
    check: impl FnOnce(
        &typaxis_layout::ProductionInlineLineLayout<'_, '_>,
        &typaxis_layout::StagingPrecomposedVectorBlockLayout,
        &typaxis_core::M4EffectiveResourceLimits,
    ),
) {
    with_production_body_resources(value, config, |lines, blocks, limits, _| {
        check(lines, blocks, limits)
    });
}

fn with_production_body_resources(
    value: &serde_json::Value,
    config: &EffectiveConfig,
    check: impl FnOnce(
        &typaxis_layout::ProductionInlineLineLayout<'_, '_>,
        &typaxis_layout::StagingPrecomposedVectorBlockLayout,
        &typaxis_core::M4EffectiveResourceLimits,
        &typaxis_resources::AdmittedResourceLedger,
    ),
) {
    with_production_body_structure_resources(value, config,
        |lines, blocks, limits, admitted, _, _| check(lines, blocks, limits, admitted));
}

fn with_production_body_structure_resources(
    value: &serde_json::Value,
    config: &EffectiveConfig,
    check: impl FnOnce(
        &typaxis_layout::ProductionInlineLineLayout<'_, '_>,
        &typaxis_layout::StagingPrecomposedVectorBlockLayout,
        &typaxis_core::M4EffectiveResourceLimits,
        &typaxis_resources::AdmittedResourceLedger,
        &typaxis_syntax::ValidatedStagingStructureSemanticsV2,
        &typaxis_machine_profile::StagingTaggedPdfProfileReceiptV2,
    ),
) {
    with_production_body_math_resources(
        value,
        config,
        |lines, blocks, limits, admitted, semantics, profile, _| {
            check(lines, blocks, limits, admitted, semantics, profile)
        },
    );
}

fn with_production_body_math_resources(
    value: &serde_json::Value,
    config: &EffectiveConfig,
    check: impl FnOnce(
        &typaxis_layout::ProductionInlineLineLayout<'_, '_>,
        &typaxis_layout::StagingPrecomposedVectorBlockLayout,
        &typaxis_core::M4EffectiveResourceLimits,
        &typaxis_resources::AdmittedResourceLedger,
        &typaxis_syntax::ValidatedStagingStructureSemanticsV2,
        &typaxis_machine_profile::StagingTaggedPdfProfileReceiptV2,
        &typaxis_layout::StagingMathVectorFlowRegistry,
    ),
) {
    with_production_inline_tagged_context(
        &serde_json::to_vec(value).unwrap(),
        config,
        |prepared, package, profile, limits, admitted, bindings, semantics, tagged| {
            let math = typaxis_layout::prepare_staging_math_vector_flows(
                package, profile, limits, admitted, bindings,
            )
            .unwrap();
            let blocks = typaxis_layout::prepare_staging_precomposed_vector_blocks(
                package, profile, limits, admitted, bindings, &math,
            )
            .unwrap();
            let widths = prepared
                .source_flow()
                .paragraphs()
                .iter()
                .map(|p| {
                    let style = p.style().block_style();
                    PositiveLength::new(
                        blocks
                            .page_geometry()
                            .body()
                            .width()
                            .get()
                            .checked_sub(style.start_indent().get())
                            .unwrap()
                            .checked_sub(style.end_indent().get())
                            .unwrap(),
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();
            let needs_container_frames = prepared.source_flow().events().iter().any(|event| {
                let typaxis_syntax::ProductionFlowEvent::Begin { owner, kind: typaxis_syntax::ProductionFlowRegionKind::SemanticContainer } = *event else { return false; };
                let style = prepared.source_flow().semantic_container_style(owner).unwrap().block_style();
                style.start_indent().get() != Length::ZERO || style.end_indent().get() != Length::ZERO
            });
            let lines = if prepared.source_flow().lists().is_empty() && !needs_container_frames {
                typaxis_layout::layout_production_inline_lines(prepared, &widths, 100_000).unwrap()
            } else {
                typaxis_layout::layout_production_body_inline_lines(
                    prepared, blocks.page_geometry().body(), 100_000,
                ).unwrap()
            };
            check(&lines, &blocks, limits, admitted, semantics, tagged, &math);
        },
    );
}

fn production_body_set_style(
    value: &mut serde_json::Value,
    selector: &str,
    name: &str,
    replacement: serde_json::Value,
) {
    let rule = value["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["selector"] == selector)
        .unwrap();
    let declaration = rule["declarations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["name"] == name)
        .unwrap();
    declaration["value"]["value"] = replacement;
}

fn production_body_renumber(value: &mut serde_json::Value, next: &mut u32) {
    match value {
        serde_json::Value::Object(fields) => {
            if let Some(id) = fields.get_mut("node_id") {
                *id = (*next).into();
                *next += 1;
            }
            for child in fields.values_mut() {
                production_body_renumber(child, next);
            }
        }
        serde_json::Value::Array(values) => {
            for v in values {
                production_body_renumber(v, next);
            }
        }
        _ => (),
    }
}

#[test]
fn production_body_places_authored_inline_block_and_following_body_with_one_cursor() {
    let value = production_body_fixture(3_000_000);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        selected.verify(lines, blocks, limits).unwrap();
        assert_eq!(selected.pages().len(), 2);
        assert_eq!(
            selected
                .fragments()
                .iter()
                .map(|f| (f.owner().get(), f.page_index(), f.bounds().y().raw()))
                .collect::<Vec<_>>(),
            [(2, 0, 655_360), (6, 0, 1_745_368), (7, 1, 655_360)]
        );
        assert_eq!(selected.pages()[0].used_height().raw(), 2_642_823);
        let first = selected.fragments()[0];
        assert_eq!(first.bounds().x().raw(), 720_896);
        assert_eq!(first.baseline().unwrap().raw(), 1_320_530);
        let block = selected.fragments()[1];
        assert_eq!(block.viewport().unwrap().x().raw(), 1_769_183);
        assert_eq!(block.viewport().unwrap().height().get().raw(), 1_552_815);
        assert_eq!(block.baseline().unwrap().raw(), 2_758_589);
        assert_eq!(block.effective_space_before().raw(), 131_072);
        assert_eq!(
            selected.fragments()[2].effective_space_before(),
            Length::ZERO
        );
        let again = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        assert_eq!(selected.fingerprint(), again.fingerprint());
        with_production_body_inputs(&value, &config(), |other_lines, other_blocks, _| {
            assert!(selected.verify(other_lines, blocks, limits).is_err());
            assert!(selected.verify(lines, other_blocks, limits).is_err());
        });
    });
}

#[test]
fn production_body_keep_uses_actual_following_paragraph_height() {
    let mut value = production_body_fixture(3_000_000);
    production_body_set_style(
        &mut value,
        "math_vector_block",
        "keep_with_next",
        true.into(),
    );
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        assert_eq!(
            selected
                .fragments()
                .iter()
                .map(|f| (f.owner().get(), f.page_index()))
                .collect::<Vec<_>>(),
            [(2, 0), (6, 1), (7, 1)]
        );
        assert_eq!(selected.pages()[1].used_height().raw(), 2_601_391);
    });
    value["page_masters"]["masters"][0]["body"]["height"] = 2_500_000.into();
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let err = match typaxis_pagination::paginate_production_body(lines, blocks, limits) {
            Err(e) => e,
            Ok(_) => panic!("kept extent must overflow"),
        };
        assert_eq!(err.owner.get(), 6);
        assert_eq!(
            err.kind,
            typaxis_pagination::ProductionBodyPaginationErrorKind::Oversize
        );
    });
}

#[test]
fn production_body_vector_caption_consumes_real_lines_and_keep_caption_policy() {
    let mut value = production_body_fixture(3_000_000);
    let parts = value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    let math = parts[1].clone();
    let caption = parts[2].clone();
    parts[1] = serde_json::json!({"kind":"vector_figure","node_id":6,"span":math["span"],"classes":[],"image_id":4,
        "viewport":math["metrics"]["viewport"],"alt":math["alt"],"caption":[caption]});
    production_body_renumber(&mut value["document"], &mut 0);
    for keep in [true, false] {
        production_body_set_style(&mut value, "vector_figure", "keep_caption", keep.into());
        with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            assert_eq!(
                selected
                    .fragments()
                    .iter()
                    .map(|f| f.owner().get())
                    .collect::<Vec<_>>(),
                [2, 6, 7, 9]
            );
            assert_eq!(
                selected
                    .fragments()
                    .iter()
                    .map(|f| f.page_index())
                    .collect::<Vec<_>>(),
                if keep {
                    vec![0, 1, 1, 2]
                } else {
                    vec![0, 0, 1, 1]
                }
            );
            assert_eq!(
                selected.fragments()[2].bounds().height().get().raw(),
                917_504
            );
        });
    }
}

#[test]
fn production_body_preserves_explicit_blank_pages_and_exact_page_limit() {
    let mut value = production_body_fixture(10_000_000);
    let parts = value["document"]["blocks"][0]["blocks"]
        .as_array()
        .unwrap()
        .clone();
    let start = parts[0]["span"]["start_byte"].clone();
    let end = parts[2]["span"]["end_byte"].clone();
    let page_break = |at: serde_json::Value| serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":{"source_id":0,"start_byte":at,"end_byte":at}});
    value["document"]["blocks"][0]["blocks"] = serde_json::json!([
        page_break(start),
        parts[0],
        parts[1],
        page_break(end.clone()),
        page_break(end.clone()),
        parts[2],
        page_break(end)
    ]);
    production_body_renumber(&mut value["document"], &mut 0);
    for maximum in [5, 4] {
        let cfg = config_with_limits(ResourceLimits {
            max_pages: maximum,
            ..ResourceLimits::default()
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let result = typaxis_pagination::paginate_production_body(lines, blocks, limits);
            if maximum == 5 {
                let selected = result.unwrap();
                assert_eq!(
                    selected
                        .pages()
                        .iter()
                        .map(|p| p.fragment_count())
                        .collect::<Vec<_>>(),
                    [0, 2, 0, 1, 0]
                );
                assert_eq!(
                    selected
                        .page_breaks()
                        .iter()
                        .map(|b| b.produced_page_index())
                        .collect::<Vec<_>>(),
                    [1, 2, 3, 4]
                );
            } else {
                let err = match result {
                    Err(e) => e,
                    Ok(_) => panic!("page budget exceeded"),
                };
                assert_eq!(
                    err.kind,
                    typaxis_pagination::ProductionBodyPaginationErrorKind::PageLimit
                );
            }
        });
    }
}

#[test]
fn production_body_fragment_budget_is_shared_with_line_and_block_preparation() {
    let value = production_body_fixture(3_000_000);
    for maximum in [40, 39] {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: maximum,
            ..ResourceLimits::default()
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let result = typaxis_pagination::paginate_production_body(lines, blocks, limits);
            if maximum == 40 {
                assert_eq!(result.unwrap().record_charge(), maximum);
            } else {
                let err = match result {
                    Err(e) => e,
                    Ok(_) => panic!("fragment budget exceeded"),
                };
                assert_eq!(
                    err.kind,
                    typaxis_pagination::ProductionBodyPaginationErrorKind::FragmentLimit
                );
                assert_eq!(err.owner.get(), 7);
            }
        });
    }
}

#[test]
fn production_body_empty_hard_break_line_stays_within_an_end_aligned_frame() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_explicit_break_fixture(&[
            "A",
            "hard_break",
            "hard_break",
            "B",
        ]))
        .unwrap();
    production_body_set_style(&mut value, "paragraph", "text_align", "end".into());
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        assert_eq!(selected.fragments().len(), 3);
        let body = selected.page_geometry().body();
        let blank = selected.fragments()[1];
        assert_eq!(blank.bounds().x().raw(), body.x().raw() + 65_536);
        assert_eq!(blank.bounds().height().get().raw(), 917_504);
        assert!(
            blank.bounds().x().raw() + blank.bounds().width().get().raw()
                <= body.x().raw() + body.width().get().raw()
        );
        assert_eq!(
            selected
                .fragments()
                .iter()
                .map(|f| f.bounds().y().raw())
                .collect::<Vec<_>>(),
            [655_360, 1_572_864, 2_490_368]
        );
    });
}

#[test]
fn production_body_retains_named_paragraph_page_for_its_pending_policy() {
    let mut value = production_body_fixture(3_000_000);
    let rule = value["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["selector"] == "paragraph")
        .unwrap();
    let declaration = rule["declarations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["name"] == "page")
        .unwrap();
    declaration["value"] = serde_json::json!({"kind":"string","value":"basic-combined"});
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        assert_eq!(
            lines.source_flow().paragraphs()[0]
                .page_name()
                .unwrap()
                .as_str(),
            "basic-combined"
        );
        let err = match typaxis_pagination::paginate_production_body(lines, blocks, limits) {
            Err(e) => e,
            Ok(_) => panic!("named page silently discarded"),
        };
        assert_eq!(
            err.kind,
            typaxis_pagination::ProductionBodyPaginationErrorKind::PendingNamedPage
        );
        assert_eq!(err.owner.get(), 2);
    });
}

#[test]
fn production_body_list_marker_uses_first_real_item_fragment() {
    let mut value = production_body_fixture(3_000_000);
    let p = value["document"]["blocks"][0]["blocks"][2].clone();
    value["document"]["blocks"][0]["blocks"][2] = serde_json::json!({"kind":"list","node_id":7,"span":p["span"],"classes":[],"ordered":true,"start":1,
        "items":[{"node_id":8,"span":p["span"],"blocks":[p]}]});
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        assert_eq!(selected.list_markers().len(), 1);
        let marker = &selected.list_markers()[0];
        assert_eq!(marker.owner().get(), 8);
        assert_eq!(marker.fragment_index(), 2);
        assert_eq!(
            marker.baseline(),
            selected.fragments()[2].baseline().unwrap()
        );
        assert_eq!(lines.list_markers()[0].utf8(), "1.");
    });
}

#[test]
fn production_body_display_and_pdf_text_use_selected_page_coordinates_and_fonts() {
    use typaxis_display_list::ProductionBodyDraw as D;
    let value = production_body_fixture(3_000_000);
    with_production_body_resources(&value, &config(), |lines, blocks, limits, admitted| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let display =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        display.verify(&selected, admitted, limits).unwrap();
        let text = display
            .draws()
            .iter()
            .filter_map(|d| match d {
                D::Text(t) => Some(t),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            text.iter().map(|t| t.exact_text()).collect::<String>(),
            "A BB"
        );
        assert_eq!(
            text.iter().map(|t| t.page_index()).collect::<Vec<_>>(),
            [0, 0, 0, 1]
        );
        assert_eq!(text[0].glyphs()[0].x().raw(), 720_896);
        assert_eq!(text[0].glyphs()[0].y().raw(), 1_320_530);
        assert_eq!(text[1].glyphs()[0].x().raw(), 1_192_755);
        assert_eq!(text[2].glyphs()[0].x().raw(), 2_894_666);
        assert_eq!(
            text[3].glyphs()[0].y(),
            selected.fragments()[2].baseline().unwrap()
        );
        assert!(text.iter().all(|t| t.font_size().get().raw() == 786_432));
        for draw in &text {
            let fragment = &selected.fragments()[draw.fragment_index() as usize];
            let typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } = fragment.source()
            else {
                panic!("text line")
            };
            let paragraph = &lines.paragraphs()[paragraph_index as usize];
            let font = paragraph.font().unwrap();
            let cluster = paragraph.lines()[line_index as usize]
                .items()
                .iter()
                .find_map(|item| match item {
                    typaxis_layout::ProductionPlacedInline::Text(c)
                        if c.run().owner() == draw.owner()
                            && matches!(c.source_span(), typaxis_shaping::ShapeSourceSpan::Parsed(span)
                                if span.start_byte() == draw.text_span().range().start_byte()) =>
                    {
                        Some(c)
                    }
                    _ => None,
                })
                .unwrap();
            let bounds = draw.logical_bounds().unwrap();
            assert_eq!(
                bounds.x(),
                fragment.bounds().x().checked_add(cluster.pen_x()).unwrap()
            );
            assert_eq!(
                bounds.y(),
                fragment
                    .baseline()
                    .unwrap()
                    .checked_sub(font.ascender())
                    .unwrap()
            );
            assert_eq!(
                bounds.width().get().raw(),
                cluster
                    .glyphs()
                    .iter()
                    .map(|g| g.glyph().advance_x.raw())
                    .sum::<i64>()
            );
            assert_eq!(
                bounds.height().get(),
                font.ascender().checked_sub(font.descender()).unwrap()
            );
        }
        // The explicit space is interactive logical area despite having no ink.
        assert!(text[1].logical_bounds().unwrap().width().get().raw() > 0);
        let vectors = display
            .draws()
            .iter()
            .filter_map(|d| match d {
                D::Vector(v) => Some(v),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(vectors.len(), 2);
        assert_eq!(vectors[0].viewport().y().raw(), 655_360);
        assert_eq!(
            vectors[1].viewport(),
            selected.fragments()[1].viewport().unwrap()
        );
        assert!(vectors.iter().all(|v| v.math_binding().is_some()));
        let fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        fonts.verify(&display, admitted, limits).unwrap();
        assert_eq!(fonts.fonts().len(), 1);
        assert_eq!(fonts.fonts()[0].pdf_font().subset_plan().cids.len(), 3);
        let contribution =
            typaxis_pdf::encode_production_body_text(&fonts, admitted, limits).unwrap();
        contribution.verify(&fonts, admitted, limits).unwrap();
        assert_eq!(contribution.paints().len(), 4);
        for (index, paint) in contribution.paints().iter().enumerate() {
            let D::Text(draw) = &display.draws()[paint.draw_index()] else {
                panic!("text paint owner")
            };
            let (font, cluster) = fonts.text_plan(paint.draw_index()).unwrap();
            assert_eq!(paint.page_index(), draw.page_index());
            assert_eq!(paint.font_instance_id(), font.pdf_font().font_instance_id());
            let bytes = std::str::from_utf8(contribution.paint_bytes(index).unwrap()).unwrap();
            assert!(bytes.contains(" 12 Tf 0 Tr\n"));
            assert_eq!(bytes.matches(" Tj\n").count(), draw.glyphs().len());
            assert!(bytes.contains(&format!("<{:04X}> Tj", cluster.cids()[0].get())));
            assert!(!bytes.contains("/MCID")); // Assigned later by actual structure ownership.
        }
        let bytes = std::str::from_utf8(contribution.paint_bytes(0).unwrap()).unwrap();
        assert!(
            bytes.contains("1 0 0 -1 11 20.149688720703125 Tm"),
            "{bytes}"
        );
        let other =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        assert_eq!(other.fingerprint(), display.fingerprint());
        assert_eq!(
            fonts.verify(&other, admitted, limits),
            Err(typaxis_resources::ResourceError::AdmittedLedgerEpochMismatch)
        );
        let other_fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        assert_eq!(
            contribution.verify(&other_fonts, admitted, limits),
            Err(typaxis_pdf::ProductionBodyTextError::ReceiptMismatch)
        );
        assert_eq!(fonts.fonts(), other_fonts.fonts());
    });
}

#[test]
fn production_body_display_anchors_follow_selected_fragments_across_pages_without_paint() {
    let bytes = production_anchor_gaps_fixture(&production_explicit_break_fixture(
        &["A", "hard_break", "B", "hard_break", "A"],
    ));
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["page_masters"]["masters"][0]["body"]["height"] = 1_000_000.into();
    value["page_masters"]["masters"][0]["footnote"] = serde_json::Value::Null;
    with_production_body_resources(&value, &config(), |lines, blocks, limits, admitted| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let display = typaxis_display_list::build_production_body_display(&selected, admitted, limits).unwrap();
        assert_eq!(selected.pages().len(), 3);
        assert_eq!(display.draws().len(), 3);
        assert_eq!(display.inline_anchors().len(), 12);
        assert_eq!(display.inline_anchors().iter().map(|a| a.page_index()).collect::<Vec<_>>(),
            [0,0,0,0,1,1,1,1,2,2,2,2]);
        for anchor in display.inline_anchors() {
            let fragment = &selected.fragments()[anchor.fragment_index() as usize];
            let position = anchor.source().position().unwrap();
            assert_eq!(anchor.page_index(), fragment.page_index());
            assert_eq!(anchor.x(), fragment.bounds().x().checked_add(position.x()).unwrap());
            assert_eq!(anchor.baseline(), fragment.baseline().unwrap());
            assert_eq!(fragment.source(), typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index: 0, line_index: position.line_index(),
            });
        }
        // Three text draws and three glyphs plus twelve nonpainting markers.
        assert_eq!(display.record_charge(), selected.record_charge() + 18);
    });
}

#[test]
fn production_body_fonts_share_repeated_glyphs_across_distinct_source_spans() {
    for family in ["Body", "Collection", "Typaxis CFF Fixture"] {
        let value =
            serde_json::from_slice(&production_text_single_paragraph(&["A", "A", "A"], family))
                .unwrap();
        let config = config_with_limits(typaxis_core::ResourceLimits {
            max_cids_per_font: 1,
            ..typaxis_core::ResourceLimits::default()
        });
        with_production_body_resources(&value, &config, |lines, blocks, limits, admitted| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
            assert_eq!(fonts.fonts().len(), 1);
            let font = &fonts.fonts()[0];
            assert_eq!(font.pdf_font().subset_plan().cids.len(), 1);
            assert_eq!(font.clusters().len(), 3);
            assert!(font
                .clusters()
                .windows(2)
                .all(|w| w[0].text_span() != w[1].text_span()));
            assert!(font.clusters().iter().all(|c| c.cids()[0].get() == 1
                && c.exact_text() == "A"
                && !c.requires_actual_text()));
            for cluster in font.clusters() {
                assert_eq!(
                    font.cluster(cluster.text_span(), "A", cluster.glyphs()),
                    Some(cluster)
                );
                assert!(font
                    .cluster(cluster.text_span(), "B", cluster.glyphs())
                    .is_none());
            }
            let output =
                typaxis_pdf::encode_production_body_text(&fonts, admitted, limits).unwrap();
            assert_eq!(output.paints().len(), 3);
        });
    }
}

#[test]
fn production_body_formula_only_has_no_text_font_or_dummy_glyph() {
    let value = serde_json::from_slice(&production_inline_vmb_fixture(false)).unwrap();
    with_production_body_resources(&value, &config(), |lines, blocks, limits, admitted| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let display =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        assert_eq!(display.draws().len(), 1);
        let fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        assert!(fonts.fonts().is_empty());
        assert!(fonts.text_plan(0).is_none());
        let output = typaxis_pdf::encode_production_body_text(&fonts, admitted, limits).unwrap();
        assert!(output.paints().is_empty());
        assert!(output.paint_bytes(0).is_none());
    });
}

#[test]
fn production_body_more_than_65535_selected_glyphs_use_one_cid_without_losing_occurrences() {
    use serde_json::json;
    let text = "A".repeat(32);
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&[&text], "Body")).unwrap();
    let paragraph = value["document"]["blocks"][0]["blocks"][0].clone();
    let buffer = value["text_buffers"][0].clone();
    let mut paragraphs = Vec::new();
    let mut buffers = Vec::new();
    for index in 0..2048 {
        let mut p = paragraph.clone();
        p["children"][0]["text_span"]["text_id"] = json!(index);
        paragraphs.push(p);
        let mut b = buffer.clone();
        b["text_id"] = json!(index);
        buffers.push(b);
    }
    value["document"]["blocks"][0]["blocks"] = json!(paragraphs);
    value["text_buffers"] = json!(buffers);
    production_body_renumber(&mut value["document"], &mut 0);
    let config = config_with_limits(typaxis_core::ResourceLimits {
        max_cids_per_font: 1,
        ..typaxis_core::ResourceLimits::default()
    });
    with_production_body_resources(&value, &config, |lines, blocks, limits, admitted| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        assert!(selected.pages().len() > 1);
        let display =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        assert_eq!(display.draws().len(), 65_536);
        let fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        assert_eq!(fonts.fonts().len(), 1);
        assert_eq!(fonts.fonts()[0].clusters().len(), 65_536);
        assert_eq!(fonts.fonts()[0].pdf_font().subset_plan().cids.len(), 1);
        let output = typaxis_pdf::encode_production_body_text(&fonts, admitted, limits).unwrap();
        assert_eq!(output.paints().len(), 65_536);
        assert!(output
            .paints()
            .windows(2)
            .all(|w| w[0].draw_index() + 1 == w[1].draw_index()
                && w[0].page_index() <= w[1].page_index()));
        assert_eq!(
            output
                .paints()
                .iter()
                .enumerate()
                .map(|(i, _)| std::str::from_utf8(output.paint_bytes(i).unwrap())
                    .unwrap()
                    .matches("<0001> Tj")
                    .count())
                .sum::<usize>(),
            65_536
        );
    });
}

#[test]
fn production_body_font_and_text_contributions_preserve_cumulative_budget_and_owner_checks() {
    let value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A"], "Body")).unwrap();
    let mut font_charge = 0;
    let mut paint_charge = 0;
    with_production_body_resources(&value, &config(), |lines, blocks, limits, admitted| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let display =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        let fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        font_charge = fonts.record_charge();
        let paint = typaxis_pdf::encode_production_body_text(&fonts, admitted, limits).unwrap();
        paint_charge = paint.record_charge();
        let mut changed_base = limits.base().get().clone();
        changed_base.max_images += 1;
        let changed = typaxis_core::M4EffectiveResourceLimits::new(
            typaxis_core::ValidatedResourceLimits::new(changed_base).unwrap(),
            limits.extension().get().clone(),
        )
        .unwrap();
        assert!(display.verify_resources(admitted, &changed).is_err());
        assert!(fonts.verify(&display, admitted, &changed).is_err());
        assert!(
            typaxis_resources::finalize_production_body_fonts(&display, admitted, &changed)
                .is_err()
        );
        assert!(typaxis_pdf::encode_production_body_text(&fonts, admitted, &changed).is_err());
        with_production_body_resources(&value, &config(), |_, _, other_limits, other_admitted| {
            assert_eq!(other_admitted.fingerprint(), admitted.fingerprint());
            assert!(fonts
                .verify(&display, other_admitted, other_limits)
                .is_err());
        });
    });
    for limit in [font_charge - 1, font_charge, paint_charge] {
        let config = config_with_limits(typaxis_core::ResourceLimits {
            max_fragments: limit,
            ..typaxis_core::ResourceLimits::default()
        });
        with_production_body_resources(&value, &config, |lines, blocks, limits, admitted| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let result =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits);
            if limit < font_charge {
                assert!(matches!(
                    result,
                    Err(typaxis_resources::ResourceError::ResourceLimit)
                ));
                return;
            }
            let fonts = result.unwrap();
            assert_eq!(fonts.record_charge(), font_charge);
            let result = typaxis_pdf::encode_production_body_text(&fonts, admitted, limits);
            if limit < paint_charge {
                assert!(matches!(
                    result,
                    Err(typaxis_pdf::ProductionBodyTextError::RecordLimit)
                ));
            } else {
                assert_eq!(result.unwrap().record_charge(), paint_charge);
            }
        });
    }
    let config = config_with_limits(typaxis_core::ResourceLimits {
        max_output_bytes: 1,
        ..typaxis_core::ResourceLimits::default()
    });
    with_production_body_resources(&value, &config, |lines, blocks, limits, admitted| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let display =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        let fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        assert!(matches!(
            typaxis_pdf::encode_production_body_text(&fonts, admitted, limits),
            Err(typaxis_pdf::ProductionBodyTextError::OutputLimit)
        ));
    });
}

#[test]
fn production_body_page_content_interleaves_real_vmb_forms_and_text_at_selected_positions() {
    use typaxis_pdf::ProductionBodyPageDrawSource as S;
    let value = production_body_fixture(3_000_000);
    with_production_body_resources(&value, &config(), |lines, blocks, limits, admitted| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let display =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        let fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        let content =
            typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
        content.verify(&fonts, admitted, limits).unwrap();
        assert_eq!(content.pages().len(), 2);
        assert_eq!(content.vectors().forms().len(), 2);
        assert_eq!(content.vectors().usages().len(), 2);
        assert_eq!(
            content.pages()[0]
                .draws()
                .iter()
                .map(|d| d.source())
                .collect::<Vec<_>>(),
            [
                S::Text { paint_index: 0 },
                S::Text { paint_index: 1 },
                S::Vector { usage_index: 0 },
                S::Text { paint_index: 2 },
                S::Vector { usage_index: 1 }
            ]
        );
        assert_eq!(
            content.pages()[1].draws()[0].source(),
            S::Text { paint_index: 3 }
        );
        for page in content.pages() {
            let bytes = std::str::from_utf8(page.content()).unwrap();
            assert_eq!(bytes.matches("1 0 0 -1 0 ").count(), 1);
            assert_eq!(
                bytes.matches(" Do").count(),
                page.draws()
                    .iter()
                    .filter(|d| matches!(d.source(), S::Vector { .. }))
                    .count()
            );
            assert!(!bytes.contains("/MCID"));
            for (ordinal, draw) in page.draws().iter().enumerate() {
                let original = match draw.source() {
                    typaxis_pdf::ProductionBodyPageDrawSource::Raster { .. } => panic!("this fixture contains only text and vectors"),
                    S::Text { paint_index } => content.text().paint_bytes(paint_index).unwrap(),
                    S::Vector { usage_index } => {
                        let vector = &content.vectors().usages()[usage_index];
                        let typaxis_display_list::ProductionBodyDraw::Vector(v) =
                            &display.draws()[draw.draw_index()]
                        else {
                            panic!("vector source")
                        };
                        assert_eq!(vector.matrix(), v.matrix());
                        assert_eq!(vector.semantic_hook().owner(), v.binding().node_id());
                        assert_eq!(
                            vector.semantic_hook().display_command_fingerprint(),
                            v.fingerprint()
                        );
                        vector.content()
                    }
                };
                assert_eq!(page.draw_content(ordinal).unwrap(), original);
            }
        }
        for form in content.vectors().forms() {
            let bytes = std::str::from_utf8(form.content_stream()).unwrap();
            assert!(bytes.contains(" re W n\n"));
            assert!(
                !bytes.contains("/ActualText")
                    && !bytes.contains("/Alt")
                    && !bytes.contains("/MCID")
            );
            assert!(bytes.contains(" c\n")); // Actual font outlines, not a placeholder rectangle.
        }
        assert!(content.vectors().pages()[1].resources().is_empty());
        let again =
            typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
        assert_eq!(content.vectors(), again.vectors());
        assert_eq!(
            content
                .pages()
                .iter()
                .map(|p| p.content())
                .collect::<Vec<_>>(),
            again
                .pages()
                .iter()
                .map(|p| p.content())
                .collect::<Vec<_>>()
        );
        let other_fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        assert!(content.verify(&other_fonts, admitted, limits).is_err());
    });
}

#[test]
fn production_body_shared_form_keeps_alias_counts_and_per_occurrence_math_semantics() {
    let mut value = production_body_fixture(3_000_000);
    let mut alias = value["resources"]["images"][2].clone();
    alias["image_id"] = 4.into();
    value["resources"]["images"][4] = alias.clone();
    alias["image_id"] = 5.into();
    value["resources"]["images"]
        .as_array_mut()
        .unwrap()
        .push(alias); // admitted, never placed
    let inline_metrics =
        value["document"]["blocks"][0]["blocks"][0]["children"][1]["metrics"].clone();
    let block = &mut value["document"]["blocks"][0]["blocks"][1];
    block["metrics"] = inline_metrics;
    block["alt"] = "ブロック分数".into();
    with_production_body_structure_resources(&value, &config(), |lines, blocks, limits, admitted, semantics, profile| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let display =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        let fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        let content =
            typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
        let structure = typaxis_display_list::build_production_body_structure(&display, semantics, profile.authorization(), profile.base().authorization(), admitted, limits).unwrap();
        let marked = typaxis_pdf::build_production_body_marked_content(&content, &structure, admitted, limits).unwrap();
        let vector_groups = structure.groups().iter().enumerate().filter(|(_, g)| g.vector_usage_id().is_some()).collect::<Vec<_>>();
        assert_eq!(vector_groups.len(), 2);
        assert_ne!(vector_groups[0].1.node(), vector_groups[1].1.node());
        assert_ne!(structure.group_actual_text(vector_groups[0].0), structure.group_actual_text(vector_groups[1].0));
        assert_eq!(structure.group_actual_text(vector_groups[1].0), Some("ブロック分数"));
        for (index, group) in vector_groups {
            let expected_hex = structure.group_actual_text(index).unwrap().encode_utf16().map(|u| format!("{u:04X}")).collect::<String>();
            let bytes = std::str::from_utf8(marked.pages()[group.page_index() as usize].content()).unwrap();
            assert_eq!(bytes.matches(&format!("/ActualText <FEFF{expected_hex}>")).count(), 1);
        }
        for form in content.vectors().forms() {
            let bytes = std::str::from_utf8(form.content_stream()).unwrap();
            assert!(!bytes.contains("/MCID") && !bytes.contains("/ActualText") && !bytes.contains("/Alt"));
        }
        assert_eq!(content.vectors().forms().len(), 1);
        assert_eq!(content.vectors().usages().len(), 2);
        let plan = &content.plans().forms().plans()[0];
        assert_eq!(
            plan.alias_usage_counts()
                .iter()
                .map(|a| (a.image_id().get(), a.usage_count()))
                .collect::<Vec<_>>(),
            [(2, 1), (4, 1), (5, 0)]
        );
        assert_eq!(plan.total_usage_count(), 2);
        let vectors = display
            .draws()
            .iter()
            .filter_map(|d| match d {
                typaxis_display_list::ProductionBodyDraw::Vector(v) => Some(v),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(vectors[0].content_key(), vectors[1].content_key());
        assert_ne!(vectors[0].fingerprint(), vectors[1].fingerprint());
        assert_ne!(
            vectors[0].math_binding().unwrap().resolved_actual_text(),
            vectors[1].math_binding().unwrap().resolved_actual_text()
        );
        assert_eq!(
            vectors[1].math_binding().unwrap().resolved_actual_text(),
            "ブロック分数"
        );
        assert_eq!(
            content.vectors().usages()[0].form_relative_object_role(),
            content.vectors().usages()[1].form_relative_object_role()
        );
        assert_eq!(
            content
                .pages()
                .iter()
                .map(|p| std::str::from_utf8(p.content())
                    .unwrap()
                    .matches(" Do")
                    .count())
                .sum::<usize>(),
            2
        );
    });
}

#[test]
fn production_body_page_content_preserves_blank_pages_and_cumulative_limits() {
    let mut value = production_body_fixture(10_000_000);
    let parts = value["document"]["blocks"][0]["blocks"]
        .as_array()
        .unwrap()
        .clone();
    let page_break = |at: serde_json::Value| serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":{"source_id":0,"start_byte":at,"end_byte":at}});
    let end = parts[2]["span"]["end_byte"].clone();
    value["document"]["blocks"][0]["blocks"] = serde_json::json!([
        page_break(0.into()),
        parts[0],
        parts[1],
        page_break(end.clone()),
        page_break(end.clone()),
        parts[2],
        page_break(end)
    ]);
    production_body_renumber(&mut value["document"], &mut 0);
    let mut records = 0;
    let mut output_bytes = 0;
    with_production_body_resources(&value, &config(), |lines, blocks, limits, admitted| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let display =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        let fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        let content =
            typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
        records = content.record_charge();
        output_bytes = content
            .pages()
            .iter()
            .map(|p| p.content().len() as u64)
            .sum();
        assert_eq!(
            content
                .pages()
                .iter()
                .map(|p| p.draws().len())
                .collect::<Vec<_>>(),
            [0, 5, 0, 1, 0]
        );
        assert_eq!(content.vectors().pages().len(), 5);
        for index in [0, 2, 4] {
            assert!(content.vectors().pages()[index].resources().is_empty());
            assert!(content.vectors().pages()[index].usage_ids().is_empty());
            let bytes = std::str::from_utf8(content.pages()[index].content()).unwrap();
            assert!(!bytes.contains("Do") && !bytes.contains("Tj") && !bytes.contains("MCID"));
        }
    });
    for (record_limit, output_limit, success) in [
        (
            records - 1,
            ResourceLimits::default().max_output_bytes,
            false,
        ),
        (records, output_bytes, true),
        (records, output_bytes - 1, false),
    ] {
        let config = config_with_limits(ResourceLimits {
            max_fragments: record_limit,
            max_output_bytes: output_limit,
            ..ResourceLimits::default()
        });
        with_production_body_resources(&value, &config, |lines, blocks, limits, admitted| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
            let result = typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits);
            assert_eq!(result.is_ok(), success);
            if !success {
                assert!(matches!(
                    result,
                    Err(typaxis_pdf::ProductionBodyPageError::OutputLimit)
                        | Err(typaxis_pdf::ProductionBodyPageError::Rasters(
                            typaxis_resources::ResourceError::ResourceLimit
                        ))
                        | Err(typaxis_pdf::ProductionBodyPageError::Forms(
                            typaxis_resources::StagingSafeVectorResourceV2Error::RecordLimit
                        ))
                ));
            }
        });
    }
}

#[test]
fn production_body_page_content_places_5000_real_svg_aliases_with_one_shared_form() {
    production_body_large_svg_content(false);
}

#[test]
fn production_body_page_content_places_5000_distinct_svg_forms_with_profile_defaults() {
    production_body_large_svg_content(true);
}

fn production_body_large_svg_content(distinct: bool) {
    use serde_json::json;
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_inline_vmb_fixture(false)).unwrap();
    let image = value["resources"]["images"][2].clone();
    let paragraph = value["document"]["blocks"][0]["blocks"][0].clone();
    let formula = paragraph["children"][0].clone();
    let text_id = formula["source_tex"]["text_span"]["text_id"]
        .as_u64()
        .unwrap() as usize;
    let tex = value["text_buffers"][text_id]["utf8"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut images = Vec::new();
    let mut paragraphs = Vec::new();
    let mut buffers = Vec::new();
    for index in 0..5000 {
        let mut im = image.clone();
        im["image_id"] = json!(index);
        if distinct {
            im["uri"] = json!(format!("vmb-distinct-{index}.svg"));
            im["expected_sha256"] = json!(sha256(&production_distinct_vmb_svg(index as u32))
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>());
        }
        images.push(im);
        let span =
            json!({"source_id":0,"start_byte":index*tex.len(),"end_byte":(index+1)*tex.len()});
        let mut p = paragraph.clone();
        p["span"] = span.clone();
        p["children"][0]["span"] = span.clone();
        p["children"][0]["image_id"] = json!(index);
        p["children"][0]["source_tex"]["text_span"]["text_id"] = json!(index);
        paragraphs.push(p);
        buffers.push(
            json!({"text_id":index,"utf8":tex,"mappings":[{"kind":"identity","source_span":span,
            "text_range":{"start_byte":0,"end_byte":tex.len()}}]}),
        );
    }
    value["resources"]["images"] = json!(images);
    value["document"]["blocks"][0]["blocks"] = json!(paragraphs);
    value["document"]["blocks"][0]["span"]["end_byte"] = json!(5000 * tex.len());
    value["text_buffers"] = json!(buffers);
    value["sources"][0]["utf8_byte_length"] = json!(5000 * tex.len());
    value["sources"][0]["sha256"] = json!(sha256(tex.repeat(5000).as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>());
    production_body_renumber(&mut value["document"], &mut 0);
    let config = crate::config::load_for_profile(
        typaxis_core::MachinePdfProfileId::ProductionBook1,
        None,
        Vec::<(&str, &str)>::new(),
        &crate::config::ConfigOverrides::default(),
    )
    .unwrap();
    assert_eq!(config.limits().get().max_images, 8192);
    assert_eq!(
        config
            .m4_limits()
            .unwrap()
            .extension()
            .get()
            .max_vector_path_segments,
        4_000_000
    );
    with_production_body_structure_resources(
        &value,
        &config,
        |lines, blocks, limits, admitted, semantics, profile| {
            assert_eq!(admitted.images().len(), 5000);
            assert_eq!(
                admitted
                    .images()
                    .iter()
                    .map(|i| i.content_hash())
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                if distinct { 5000 } else { 1 }
            );
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
            assert!(fonts.fonts().is_empty());
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
            assert_eq!(structure.groups().len(), 5000);
            let mut observed = 0;
            for page in marked.pages() {
                let groups = structure.page_groups(page.page_index()).unwrap();
                for (mcid, group) in groups.iter().enumerate() {
                    assert_eq!(group.mcid(), mcid as u32);
                    assert_eq!(group.vector_usage_id(), Some(observed));
                    assert_eq!(group.semantic_fragment_ordinal(), 0);
                    let node = structure.registry().node(group.node()).unwrap();
                    assert_eq!(node.role(), typaxis_layout::StructureRole::Formula);
                    assert_eq!(structure.node_groups(group.node()).unwrap().len(), 1);
                    observed += 1;
                }
                let bytes = std::str::from_utf8(page.content()).unwrap();
                assert_eq!(bytes.matches("/MCID ").count(), groups.len());
                assert_eq!(bytes.matches("/ActualText ").count(), groups.len());
                assert_eq!(bytes.matches(" Do").count(), groups.len());
                assert_eq!(
                    bytes.matches(" BDC\n").count(),
                    bytes.matches("EMC\n").count()
                );
            }
            assert_eq!(observed, 5000);
            let objects =
                typaxis_pdf::build_production_body_objects(&marked, admitted, limits).unwrap();
            use typaxis_pdf::ProductionBodyObjectRole as R;
            assert_eq!(
                objects
                    .objects()
                    .iter()
                    .filter(|o| matches!(o.role(), R::Vector(_)))
                    .count(),
                content.vectors().relative_objects().len()
            );
            assert_eq!(
                objects
                    .objects()
                    .iter()
                    .filter(|o| matches!(o.role(), R::StructureNode(_)))
                    .count(),
                structure.registry().nodes().len()
            );
            assert!(!objects
                .objects()
                .iter()
                .any(|o| matches!(o.role(), R::Font { .. })));
            let parent = objects
                .objects()
                .iter()
                .find(|o| o.role() == R::ParentTree)
                .unwrap();
            assert_eq!(production_object_references(parent).len(), 5000);
            for (reference, group) in production_object_references(parent)
                .into_iter()
                .zip(structure.groups())
            {
                assert_eq!(reference, R::StructureNode(group.node()));
            }
            assert_eq!(
                objects
                    .objects()
                    .iter()
                    .filter(|o| matches!(o.role(), R::PageContent(_)))
                    .count(),
                content.pages().len()
            );
            assert_eq!(
                content.vectors().forms().len(),
                if distinct { 5000 } else { 1 }
            );
            assert_eq!(content.vectors().usages().len(), 5000);
            assert_eq!(
                content.plans().forms().plans().len(),
                if distinct { 5000 } else { 1 }
            );
            assert_eq!(
                content
                    .plans()
                    .forms()
                    .plans()
                    .iter()
                    .map(|p| p.alias_usage_counts().len())
                    .sum::<usize>(),
                5000
            );
            assert!(content
                .plans()
                .forms()
                .plans()
                .iter()
                .all(|p| p.alias_usage_counts().iter().all(|a| a.usage_count() == 1)));
            assert!(content.pages().len() > 1);
            assert_eq!(
                content
                    .pages()
                    .iter()
                    .map(|p| p.draws().len())
                    .sum::<usize>(),
                5000
            );
            assert_eq!(
                content
                    .pages()
                    .iter()
                    .map(|p| std::str::from_utf8(p.content())
                        .unwrap()
                        .matches(" Do")
                        .count())
                    .sum::<usize>(),
                5000
            );
            assert!(content
                .pages()
                .iter()
                .all(|p| !std::str::from_utf8(p.content()).unwrap().contains(" Tj")));
            assert!(content
                .vectors()
                .usages()
                .windows(2)
                .all(|w| w[0].paint_ordinal() + 1 == w[1].paint_ordinal()
                    && w[0].page_index() <= w[1].page_index()));
        },
    );
}

#[test]
fn production_body_structure_marks_actual_text_and_vmb_vectors_in_source_order() {
    use typaxis_display_list::{build_production_body_structure, ProductionBodyStructureError};
    use typaxis_layout::{StructureOwner, StructureRole};
    let value = production_body_fixture(3_000_000);
    with_production_body_structure_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, semantics, profile| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
            let content =
                typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
            let structure = build_production_body_structure(
                &display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .unwrap();
            structure.verify(&display, admitted, limits).unwrap();
            assert_eq!(structure.groups().len(), 5);
            assert_eq!(
                structure
                    .groups()
                    .iter()
                    .map(|g| (g.page_index(), g.mcid(), g.draws()))
                    .collect::<Vec<_>>(),
                [
                    (0, 0, 0..2),
                    (0, 1, 2..3),
                    (0, 2, 3..4),
                    (0, 3, 4..5),
                    (1, 0, 5..6)
                ]
            );
            let owners = structure
                .groups()
                .iter()
                .map(|g| structure.registry().node(g.node()).unwrap().owner())
                .collect::<Vec<_>>();
            assert_eq!(
                owners,
                [3, 4, 5, 6, 8].map(|n| StructureOwner::Source(typaxis_core::NodeId::new(n)))
            );
            for (index, group) in structure.groups().iter().enumerate() {
                assert_eq!(structure.node_groups(group.node()).unwrap(), [index]);
                let node = structure.registry().node(group.node()).unwrap();
                if group.vector_usage_id().is_some() {
                    assert_eq!(node.role(), StructureRole::Formula);
                    assert_eq!(structure.group_actual_text(index), node.actual_text());
                    assert!(node.alternative().is_some());
                } else {
                    assert_eq!(node.role(), StructureRole::Span);
                    assert_eq!(structure.group_actual_text(index), None);
                }
            }
            let marked = typaxis_pdf::build_production_body_marked_content(
                &content, &structure, admitted, limits,
            )
            .unwrap();
            marked
                .verify(&content, &structure, admitted, limits)
                .unwrap();
            for (page, expected_mcids) in marked.pages().iter().zip([4, 1]) {
                let bytes = std::str::from_utf8(page.content()).unwrap();
                assert_eq!(bytes.matches("/MCID ").count(), expected_mcids);
                assert_eq!(bytes.matches("1 0 0 -1 0 ").count(), 1);
                assert_eq!(
                    bytes.matches(" BDC\n").count(),
                    bytes.matches("EMC\n").count()
                );
                for mcid in 0..expected_mcids {
                    assert!(bytes.contains(&format!("/MCID {mcid} ")));
                }
            }
            let page0 = std::str::from_utf8(marked.pages()[0].content()).unwrap();
            assert_eq!(page0.matches("/ActualText").count(), 4);
            assert_eq!(page0.matches(" Do").count(), 2);
            assert!(
                page0.find("/Span << /MCID 0").unwrap()
                    < page0.find("/Formula << /MCID 1").unwrap()
            );
            assert!(
                page0.find("/Formula << /MCID 1").unwrap()
                    < page0.find("/Span << /MCID 2").unwrap()
            );
            assert!(
                page0.find("/Span << /MCID 2").unwrap()
                    < page0.find("/Formula << /MCID 3").unwrap()
            );
            assert_eq!(
                std::str::from_utf8(marked.pages()[1].content())
                    .unwrap()
                    .matches("/ActualText")
                    .count(),
                1
            );
            let again = typaxis_pdf::build_production_body_marked_content(
                &content, &structure, admitted, limits,
            )
            .unwrap();
            assert_eq!(
                marked
                    .pages()
                    .iter()
                    .map(|p| p.content())
                    .collect::<Vec<_>>(),
                again
                    .pages()
                    .iter()
                    .map(|p| p.content())
                    .collect::<Vec<_>>()
            );
            let other_display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            assert_eq!(display.fingerprint(), other_display.fingerprint());
            assert_eq!(
                structure.verify(&other_display, admitted, limits),
                Err(ProductionBodyStructureError::ReceiptMismatch)
            );
            let other_structure = build_production_body_structure(
                &other_display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .unwrap();
            assert!(matches!(
                typaxis_pdf::build_production_body_marked_content(
                    &content,
                    &other_structure,
                    admitted,
                    limits
                ),
                Err(typaxis_pdf::ProductionBodyMarkedError::ReceiptMismatch)
            ));
            with_production_body_structure_resources(
                &value,
                &config(),
                |_, _, other_limits, other_admitted, other_semantics, other_profile| {
                    assert_eq!(other_admitted.fingerprint(), admitted.fingerprint());
                    assert!(structure
                        .verify(&display, other_admitted, other_limits)
                        .is_err());
                    // Profile/semantic authorizations are deterministic values. Equal
                    // inputs can reproduce them; display and ledger borrows still cannot
                    // be swapped, even when their fingerprints match.
                    let repeated = build_production_body_structure(
                        &display,
                        other_semantics,
                        other_profile.authorization(),
                        other_profile.base().authorization(),
                        admitted,
                        limits,
                    )
                    .unwrap();
                    assert_eq!(structure.fingerprint(), repeated.fingerprint());
                },
            );
            let mut different = value.clone();
            different["document"]["blocks"][0]["blocks"][1]["actual_text"] = "別の分数".into();
            with_production_body_structure_resources(
                &different,
                &config(),
                |_, _, _, _, other_semantics, other_profile| {
                    assert!(build_production_body_structure(
                        &display,
                        other_semantics,
                        other_profile.authorization(),
                        other_profile.base().authorization(),
                        admitted,
                        limits
                    )
                    .is_err());
                },
            );
        },
    );
}

#[test]
fn production_body_structure_splits_one_source_text_across_pages_without_repeated_actual_text() {
    let text = "A A A A A A A A A A A A";
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&[text], "Body")).unwrap();
    value["page_masters"]["masters"][0]["body"]["width"] = 1_600_000.into();
    value["page_masters"]["masters"][0]["body"]["height"] = 1_300_000.into();
    with_production_body_structure_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, semantics, profile| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
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
            assert!(marked.pages().len() > 1);
            let source = structure
                .registry()
                .source_node(typaxis_core::NodeId::new(3))
                .unwrap();
            assert_eq!(source.actual_text(), Some(text));
            let groups = structure.node_groups(source.structure_node_id()).unwrap();
            assert_eq!(groups.len(), selected.fragments().len());
            for (ordinal, index) in groups.iter().copied().enumerate() {
                let group = &structure.groups()[index];
                assert_eq!(group.semantic_fragment_ordinal(), ordinal as u32);
                assert_eq!(group.node(), source.structure_node_id());
                assert_eq!(group.mcid(), 0);
                assert!(structure.group_actual_text(index).is_none());
            }
            // Each selected fragment has its own replacement text. The full
            // source node must never be repeated on every page/line.
            for page in marked.pages() {
                let bytes = std::str::from_utf8(page.content()).unwrap();
                assert_eq!(bytes.matches("/MCID 0 ").count(), 1);
                assert_eq!(bytes.matches("/ActualText").count(), 1);
                let group = &structure.page_groups(page.page_index()).unwrap()[0];
                let selected_text: String = display.draws()[group.draws()]
                    .iter()
                    .map(|d| match d {
                        typaxis_display_list::ProductionBodyDraw::Text(t) => t.exact_text(),
                        _ => panic!("text-only group"),
                    })
                    .collect();
                assert!(selected_text.len() < text.len());
                let hex: String = selected_text
                    .encode_utf16()
                    .map(|u| format!("{u:04X}"))
                    .collect();
                assert!(bytes.contains(&format!("/ActualText <FEFF{hex}>")));
            }
            let painted: String = display
                .draws()
                .iter()
                .map(|d| match d {
                    typaxis_display_list::ProductionBodyDraw::Text(t) => t.exact_text(),
                    _ => panic!("text-only fixture"),
                })
                .collect();
            assert_eq!(painted, text);
        },
    );
}

#[test]
fn production_body_structure_blank_pages_and_marked_budget_boundaries() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A"], "Body")).unwrap();
    let paragraph = value["document"]["blocks"][0]["blocks"][0].clone();
    let page_break = serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":{"source_id":0,"start_byte":0,"end_byte":0}});
    value["document"]["blocks"][0]["blocks"] =
        serde_json::json!([page_break, paragraph, page_break, page_break]);
    production_body_renumber(&mut value["document"], &mut 0);
    let (mut records, mut output, mut spool) = (0, 0, 0);
    with_production_body_structure_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, semantics, profile| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
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
            assert_eq!(marked.pages().len(), 4);
            assert_eq!(
                (0..4)
                    .map(|p| structure.page_groups(p).unwrap().len())
                    .collect::<Vec<_>>(),
                [0, 1, 0, 0]
            );
            for page in [0, 2, 3] {
                let bytes = std::str::from_utf8(marked.pages()[page].content()).unwrap();
                assert!(!bytes.contains("MCID") && !bytes.contains("Tj") && !bytes.contains("Do"));
            }
            records = marked.record_charge();
            spool = marked.spool_charge();
            output = marked
                .pages()
                .iter()
                .map(|p| p.content().len() as u64)
                .sum();
            assert_eq!(
                records,
                content.record_charge() + structure.record_charge()
                    - display.record_charge()
                    + 4
            );
            assert_eq!(
                spool,
                content.spool_charge() + structure.spool_charge() + output
            );
        },
    );
    // The exact merged budget succeeds. Each branch must not silently restart
    // the record/spool budget when fonts, vectors and structure are combined.
    for (record_limit, output_limit, spool_limit, expected) in [
        (records, output, spool, None),
        (
            records - 1,
            output,
            spool,
            Some(typaxis_pdf::ProductionBodyMarkedError::RecordLimit),
        ),
        (
            records,
            output - 1,
            spool,
            Some(typaxis_pdf::ProductionBodyMarkedError::OutputLimit),
        ),
        (
            records,
            output,
            spool - 1,
            Some(typaxis_pdf::ProductionBodyMarkedError::OutputLimit),
        ),
    ] {
        let config = config_with_limits(ResourceLimits {
            max_fragments: record_limit,
            max_output_bytes: output_limit,
            max_spool_bytes: spool_limit,
            ..ResourceLimits::default()
        });
        with_production_body_structure_resources(
            &value,
            &config,
            |lines, blocks, limits, admitted, semantics, profile| {
                let selected =
                    typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
                let display = typaxis_display_list::build_production_body_display(
                    &selected, admitted, limits,
                )
                .unwrap();
                let fonts =
                    typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                        .unwrap();
                let content =
                    typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits)
                        .unwrap();
                let structure = typaxis_display_list::build_production_body_structure(
                    &display,
                    semantics,
                    profile.authorization(),
                    profile.base().authorization(),
                    admitted,
                    limits,
                )
                .unwrap();
                let result = typaxis_pdf::build_production_body_marked_content(
                    &content, &structure, admitted, limits,
                );
                match expected {
                    None => {
                        let marked = result.unwrap();
                        assert_eq!(marked.record_charge(), records);
                        assert_eq!(marked.spool_charge(), spool);
                    }
                    Some(error) => assert_eq!(result.err(), Some(error)),
                }
            },
        );
    }
}

#[test]
fn production_body_structure_keeps_figure_caption_as_child_after_its_vector_paint() {
    let mut value = production_body_fixture(3_000_000);
    let parts = value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    let math = parts[1].clone();
    let caption = parts[2].clone();
    parts[1] = serde_json::json!({"kind":"vector_figure","node_id":6,"span":math["span"],"classes":[],"image_id":4,
        "viewport":math["metrics"]["viewport"],"alt":math["alt"],"caption":[caption]});
    production_body_renumber(&mut value["document"], &mut 0);
    production_body_set_style(&mut value, "vector_figure", "keep_caption", true.into());
    with_production_body_structure_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, semantics, profile| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
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
            let figure = structure
                .registry()
                .source_node(typaxis_core::NodeId::new(6))
                .unwrap();
            assert_eq!(figure.role(), typaxis_layout::StructureRole::Figure);
            assert!(figure.alternative().is_some());
            assert_eq!(figure.children().len(), 1);
            let caption = structure.registry().node(figure.children()[0]).unwrap();
            assert_eq!(caption.role(), typaxis_layout::StructureRole::Caption);
            assert_eq!(caption.parent(), Some(figure.structure_node_id()));
            assert_eq!(caption.children().len(), 1);
            let paragraph = structure.registry().node(caption.children()[0]).unwrap();
            assert_eq!(paragraph.role(), typaxis_layout::StructureRole::Paragraph);
            let text = structure.registry().node(paragraph.children()[0]).unwrap();
            let figure_groups = structure.node_groups(figure.structure_node_id()).unwrap();
            let text_groups = structure.node_groups(text.structure_node_id()).unwrap();
            assert_eq!(figure_groups.len(), 1);
            assert_eq!(text_groups.len(), 1);
            assert!(structure
                .node_groups(caption.structure_node_id())
                .unwrap()
                .is_empty());
            assert!(figure_groups[0] < text_groups[0]);
            let figure_group = &structure.groups()[figure_groups[0]];
            let text_group = &structure.groups()[text_groups[0]];
            assert_eq!(figure_group.page_index(), text_group.page_index());
            assert_eq!(figure_group.mcid() + 1, text_group.mcid());
            assert_eq!(structure.group_actual_text(figure_groups[0]), None);
            let bytes =
                std::str::from_utf8(marked.pages()[figure_group.page_index() as usize].content())
                    .unwrap();
            assert!(
                bytes.find("/Figure << /MCID 0").unwrap() < bytes.find("/Span << /MCID 1").unwrap()
            );
        },
    );
}


#[test]
fn production_body_page_choices_avoid_a_single_continuation_line_and_enforce_search_budget() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_explicit_break_fixture(&[
            "A",
            "hard_break",
            "A",
            "hard_break",
            "B",
            "hard_break",
            "B",
        ]))
        .unwrap();
    value["page_masters"]["masters"][0]["body"]["height"] = (3 * 917_504).into();
    value["page_masters"]["masters"][0]["footnote"] = serde_json::Value::Null;
    for maximum in [3, 2] {
        let cfg = config_with_limits(ResourceLimits {
            max_page_break_lookback: maximum,
            ..ResourceLimits::default()
        });
        with_production_body_resources(&value, &cfg, |lines, blocks, limits, admitted| {
            assert_eq!(lines.paragraphs()[0].lines().len(), 4);
            let result = typaxis_pagination::paginate_production_body(lines, blocks, limits);
            if maximum == 2 {
                let err = match result {
                    Err(e) => e,
                    Ok(_) => panic!("search cannot truncate candidates"),
                };
                assert_eq!(
                    err.kind,
                    typaxis_pagination::ProductionBodyPaginationErrorKind::PageBreakLookbackLimit {
                        limit: 2,
                        observed: 3
                    }
                );
                assert_eq!(err.owner.get(), 2);
                return;
            }
            let selected = result.unwrap();
            assert_eq!(
                selected
                    .pages()
                    .iter()
                    .map(|p| p.fragment_count())
                    .collect::<Vec<_>>(),
                [2, 2]
            );
            assert_eq!(
                selected
                    .fragments()
                    .iter()
                    .map(|f| f.page_index())
                    .collect::<Vec<_>>(),
                [0, 0, 1, 1]
            );
            assert_eq!(
                selected
                    .fragments()
                    .iter()
                    .map(|f| f.bounds().y().raw())
                    .collect::<Vec<_>>(),
                [655_360, 1_572_864, 655_360, 1_572_864]
            );
            assert_eq!(selected.page_break_decisions()[0].selected().end_item(), 2);
            assert_eq!(selected.page_break_decisions()[0].candidates().len(), 3);
            assert_eq!(
                selected.page_break_decisions()[1].reason(),
                typaxis_pagination::ProductionBodyBreakReason::End
            );
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let painted: String = display
                .draws()
                .iter()
                .map(|d| match d {
                    typaxis_display_list::ProductionBodyDraw::Text(t) => t.exact_text(),
                    _ => panic!("body-only case"),
                })
                .collect();
            assert_eq!(painted, "AABB");
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
            let text = typaxis_pdf::encode_production_body_text(&fonts, admitted, limits).unwrap();
            text.verify(&fonts, admitted, limits).unwrap();
            assert_eq!(
                text.paints()
                    .iter()
                    .map(|p| p.page_index())
                    .collect::<Vec<_>>(),
                [0, 0, 1, 1]
            );
            let again =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            assert_eq!(selected.fingerprint(), again.fingerprint());
        });
    }
}

#[test]
fn production_body_book_sized_page_requires_explicit_sufficient_lookback() {
    let mut kinds = Vec::new();
    for index in 0..70 {
        if index > 0 {
            kinds.push("hard_break");
        }
        kinds.push("A");
    }
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_explicit_break_fixture(&kinds)).unwrap();
    value["page_masters"]["masters"][0]["body"]["height"] = (33 * 917_504).into();
    value["page_masters"]["masters"][0]["footnote"] = serde_json::Value::Null;
    value["page_masters"]["masters"][0]["height"] = (33 * 917_504 + 2 * 655_360).into();
    value["page_masters"]["masters"][0]["trim"]["height"] = (33 * 917_504 + 2 * 655_360).into();
    for maximum in [32, 128] {
        let cfg = config_with_limits(ResourceLimits {
            max_page_break_lookback: maximum,
            ..ResourceLimits::default()
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            assert_eq!(lines.paragraphs()[0].lines().len(), 70);
            let result = typaxis_pagination::paginate_production_body(lines, blocks, limits);
            if maximum == 32 {
                let e = match result {
                    Err(e) => e,
                    Ok(_) => panic!("33 candidates exceed 32"),
                };
                assert_eq!(
                    e.kind,
                    typaxis_pagination::ProductionBodyPaginationErrorKind::PageBreakLookbackLimit {
                        limit: 32,
                        observed: 33
                    }
                );
            } else {
                let selected = result.unwrap();
                assert_eq!(
                    selected
                        .pages()
                        .iter()
                        .map(|p| p.fragment_count())
                        .collect::<Vec<_>>(),
                    [33, 33, 4]
                );
                assert_eq!(selected.fragments().len(), 70);
                for (index, f) in selected.fragments().iter().enumerate() {
                    assert_eq!(
                        f.source(),
                        typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine {
                            paragraph_index: 0,
                            line_index: index as u32
                        }
                    );
                }
                assert_eq!(
                    selected
                        .page_break_decisions()
                        .iter()
                        .map(|d| d.candidates().len())
                        .collect::<Vec<_>>(),
                    [33, 33, 1]
                );
            }
        });
    }
}

#[test]
fn production_body_cost_selected_break_keeps_four_real_vmb_formulas_and_semantics() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_inline_vmb_fixture(true)).unwrap();
    let template = value["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array()
        .unwrap()
        .clone();
    let tex = value["text_buffers"][2]["utf8"]
        .as_str()
        .unwrap()
        .to_owned();
    let size = tex.len();
    let buffer = value["text_buffers"][2].clone();
    let mut children = Vec::new();
    for index in 0..4 {
        let begin = index * size;
        let end = begin + size;
        if index > 0 {
            children.push(serde_json::json!({"kind":"hard_break","node_id":0,"span":{"source_id":0,"start_byte":begin,"end_byte":begin}}));
        }
        let mut line = template.clone();
        line[0]["span"] = serde_json::json!({"source_id":0,"start_byte":begin,"end_byte":begin});
        line[1]["span"] = serde_json::json!({"source_id":0,"start_byte":begin,"end_byte":end});
        line[2]["span"] = serde_json::json!({"source_id":0,"start_byte":end,"end_byte":end});
        line[1]["source_tex"]["text_span"]["text_id"] = (2 + index).into();
        if index > 0 {
            let mut b = buffer.clone();
            b["text_id"] = (2 + index).into();
            b["mappings"][0]["source_span"] = line[1]["span"].clone();
            value["text_buffers"].as_array_mut().unwrap().push(b);
        }
        children.extend(line);
    }
    value["sources"][0]["utf8_byte_length"] = (4 * size).into();
    value["sources"][0]["sha256"] = sha256(tex.repeat(4).as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    value["document"]["blocks"][0]["span"]["end_byte"] = (4 * size).into();
    value["document"]["blocks"][0]["blocks"][0]["span"]["end_byte"] = (4 * size).into();
    value["document"]["blocks"][0]["blocks"][0]["children"] = children.into();
    production_body_renumber(&mut value["document"], &mut 0);
    value["page_masters"]["masters"][0]["body"]["height"] = (3 * 958_936).into();
    value["page_masters"]["masters"][0]["footnote"] = serde_json::Value::Null;
    with_production_body_structure_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, semantics, profile| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            assert_eq!(
                selected
                    .pages()
                    .iter()
                    .map(|p| p.fragment_count())
                    .collect::<Vec<_>>(),
                [2, 2]
            );
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
            let content =
                typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
            assert_eq!(content.vectors().forms().len(), 1);
            assert_eq!(content.vectors().usages().len(), 4);
            let structure = typaxis_display_list::build_production_body_structure(
                &display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .unwrap();
            let formulas = structure
                .groups()
                .iter()
                .filter(|g| g.vector_usage_id().is_some())
                .collect::<Vec<_>>();
            assert_eq!(
                formulas.iter().map(|g| g.page_index()).collect::<Vec<_>>(),
                [0, 0, 1, 1]
            );
            for group in formulas {
                let node = structure.registry().node(group.node()).unwrap();
                assert_eq!(node.role(), typaxis_layout::StructureRole::Formula);
                assert!(node.alternative().is_some());
                assert!(node.actual_text().is_some());
            }
            let marked = typaxis_pdf::build_production_body_marked_content(
                &content, &structure, admitted, limits,
            )
            .unwrap();
            for page in marked.pages() {
                let stream = std::str::from_utf8(page.content()).unwrap();
                assert_eq!(stream.matches(" Do").count(), 2);
                assert_eq!(stream.matches("/Formula <<").count(), 2);
            }
            let painted: String = display
                .draws()
                .iter()
                .filter_map(|d| match d {
                    typaxis_display_list::ProductionBodyDraw::Text(t) => Some(t.exact_text()),
                    _ => None,
                })
                .collect();
            assert_eq!(painted, "A BA BA BA B");
        },
    );
}
