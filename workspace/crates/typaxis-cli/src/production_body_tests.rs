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
                typaxis_layout::layout_production_inline_lines(prepared, &widths, 1000).unwrap();
            check(&lines, &blocks, limits);
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
