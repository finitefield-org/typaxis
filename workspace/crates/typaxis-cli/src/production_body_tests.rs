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
    with_production_inline_context(
        &serde_json::to_vec(value).unwrap(),
        config,
        |prepared, package, profile, limits, admitted, bindings| {
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
            let lines =
                typaxis_layout::layout_production_inline_lines(prepared, &widths, 100_000).unwrap();
            check(&lines, &blocks, limits, admitted);
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
    for maximum in [33, 32] {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: maximum,
            ..ResourceLimits::default()
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let result = typaxis_pagination::paginate_production_body(lines, blocks, limits);
            if maximum == 33 {
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
fn production_body_does_not_flatten_unconnected_list_flow() {
    let mut value = production_body_fixture(3_000_000);
    let p = value["document"]["blocks"][0]["blocks"][2].clone();
    value["document"]["blocks"][0]["blocks"][2] = serde_json::json!({"kind":"list","node_id":7,"span":p["span"],"classes":[],"ordered":true,"start":1,
        "items":[{"node_id":8,"span":p["span"],"blocks":[p]}]});
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let err = match typaxis_pagination::paginate_production_body(lines, blocks, limits) {
            Err(e) => e,
            Ok(_) => panic!("list needs marker and layout owner"),
        };
        assert_eq!(err.owner.get(), 7);
        assert_eq!(
            err.kind,
            typaxis_pagination::ProductionBodyPaginationErrorKind::PendingRegion("list")
        );
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
    with_production_body_resources(&value, &config(), |lines, blocks, limits, admitted| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let display =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        let fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        let content =
            typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
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
        records = content.plans().record_charge();
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
    let config = config_with_limits(ResourceLimits {
        max_images: 8192,
        ..ResourceLimits::default()
    });
    with_production_body_resources(&value, &config, |lines, blocks, limits, admitted| {
        assert_eq!(admitted.images().len(), 5000);
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let display =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        let fonts =
            typaxis_resources::finalize_production_body_fonts(&display, admitted, limits).unwrap();
        assert!(fonts.fonts().is_empty());
        let content =
            typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
        assert_eq!(content.vectors().forms().len(), 1);
        assert_eq!(content.vectors().usages().len(), 5000);
        let plan = &content.plans().forms().plans()[0];
        assert_eq!(plan.alias_usage_counts().len(), 5000);
        assert!(plan
            .alias_usage_counts()
            .iter()
            .all(|a| a.usage_count() == 1));
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
    });
}
