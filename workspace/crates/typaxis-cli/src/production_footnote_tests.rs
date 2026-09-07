fn production_footnote_flow_fixture() -> serde_json::Value {
    use serde_json::json;
    let mut value = production_list_fixture();
    let container = value["document"]["blocks"][0].clone();
    let span = container["span"].clone();
    let vector = container["blocks"][1].clone();
    let list = container["blocks"][2].clone();
    let paragraph = list["items"][1]["blocks"][0].clone();
    value["document"]["blocks"][0]["blocks"] = json!([container["blocks"][0]]);
    let paragraph_end = container["blocks"][0]["span"]["end_byte"].clone();
    let reference_span = json!({"source_id":0,"start_byte":paragraph_end,"end_byte":paragraph_end});
    for id in ["first", "second"] {
        value["document"]["blocks"][0]["blocks"][0]["children"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"footnote_reference","node_id":0,"span":reference_span,"footnote_id":id}));
    }
    value["document"]["footnotes"] = json!([
        {"node_id":0,"span":span,"footnote_id":"first","blocks":[vector,list]},
        {"node_id":0,"span":span,"footnote_id":"second","blocks":[paragraph]}
    ]);
    value["page_masters"]["masters"][0]["footnote"] =
        json!({"x":955360,"y":13000000,"width":16000000,"height":6000000});
    production_body_set_style(&mut value, "list", "font_size", (32 * 65_536).into());
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_footnote_flow_collects_real_blocks_and_lists_without_body_splicing() {
    use typaxis_pagination::{
        prepare_production_body_flow, ProductionBodyFragmentSource as Source,
    };
    let value = production_footnote_flow_fixture();
    let bytes = serde_json::to_vec(&value).unwrap();
    let mut required = 0;
    let mut records_before_collection = 0;
    for maximum in [None, Some(0), Some(1)] {
        let cfg = match maximum {
            None => config(),
            Some(delta) => config_with_limits(ResourceLimits {
                max_fragments: required - delta,
                ..ResourceLimits::default()
            }),
        };
        with_production_inline_context(
            &bytes,
            &cfg,
            |prepared, package, profile, limits, admitted, bindings| {
                let math = typaxis_layout::prepare_staging_math_vector_flows(
                    package, profile, limits, admitted, bindings,
                )
                .unwrap();
                let blocks = typaxis_layout::prepare_staging_precomposed_vector_blocks(
                    package, profile, limits, admitted, bindings, &math,
                )
                .unwrap();
                let lines = typaxis_layout::layout_production_body_inline_lines(
                    prepared,
                    profile.page_geometry().body(),
                    100_000,
                )
                .unwrap();
                let footnotes =
                    typaxis_layout::prepare_production_footnote_lines(&lines, limits).unwrap();
                let result = prepare_production_body_flow(&lines, &blocks, &footnotes, limits);
                if maximum == Some(1) {
                    assert_eq!(
                        result.err().unwrap().kind,
                        typaxis_pagination::ProductionBodyPaginationErrorKind::FragmentLimit
                    );
                    return;
                }
                let flow = result.unwrap();
                flow.verify(&lines, &blocks, limits).unwrap();
                assert_eq!(flow.footnotes().definitions().len(), 2);
                assert_eq!(flow.footnotes().references().len(), 2);
                assert_eq!(flow.list_marker_count(), 4);
                assert_eq!(
                    flow.footnote_region(),
                    lines.frames().unwrap().footnote_region()
                );
                assert_eq!(flow.body_items().len(), lines.paragraphs()[0].lines().len());
                assert!(flow.body_items().iter().all(|i| matches!(
                    i.source(),
                    Some(Source::ParagraphLine {
                        paragraph_index: 0,
                        ..
                    })
                )));
                let first = flow.definition_items(0).unwrap();
                let second = flow.definition_items(1).unwrap();
                assert!(flow.definition_items(2).is_none());
                assert!(matches!(
                    first[0].source(),
                    Some(Source::VectorBlock { block_index: 0 })
                ));
                assert!(first[0].viewport_left().is_some());
                assert!(first
                    .iter()
                    .any(|item| item.leading() > Length::ZERO || item.trailing() > Length::ZERO));
                assert_eq!(second.len(), 1);
                assert_eq!(second[0].space_before().raw(), 65_536);
                assert_eq!(second[0].space_after().raw(), 65_536);
                for (definition_index, items) in [first, second].iter().enumerate() {
                    let paragraphs =
                        flow.footnotes().definitions()[definition_index].paragraph_range();
                    assert!(!items.is_empty());
                    for item in *items {
                        assert!(item.x().raw() >= 955_360);
                        assert!(item.width().get() > Length::ZERO);
                        assert!(item.consumed_height().unwrap() >= item.height());
                        if let Some(Source::ParagraphLine {
                            paragraph_index,
                            line_index,
                        }) = item.source()
                        {
                            assert!(paragraphs.contains(&(paragraph_index as usize)));
                            let selected = &lines.paragraphs()[paragraph_index as usize]
                                .selected()
                                .unwrap()
                                .lines()[line_index as usize];
                            assert_eq!(
                                item.owner(),
                                lines.paragraphs()[paragraph_index as usize].owner()
                            );
                            assert_eq!(
                                item.height(),
                                selected.line().metrics().line_height().get()
                            );
                        }
                    }
                }
                let count = flow.body_items().len() + first.len() + second.len();
                records_before_collection =
                    flow.footnotes().record_charge() + blocks.blocks().len() as u64;
                assert_eq!(
                    flow.record_charge(),
                    records_before_collection + 2 + count as u64 + 4
                );
                required = flow.record_charge();
                let other = typaxis_layout::layout_production_body_inline_lines(
                    prepared,
                    profile.page_geometry().body(),
                    100_000,
                )
                .unwrap();
                assert!(flow.verify(&other, &blocks, limits).is_err());
                assert_eq!(
                    prepare_production_body_flow(&other, &blocks, &footnotes, limits)
                        .err()
                        .unwrap()
                        .kind,
                    typaxis_pagination::ProductionBodyPaginationErrorKind::ReceiptMismatch
                );
                let widths = lines
                    .frames()
                    .unwrap()
                    .paragraphs()
                    .iter()
                    .map(|frame| frame.width())
                    .collect::<Vec<_>>();
                let unframed =
                    typaxis_layout::layout_production_inline_lines(prepared, &widths, 100_000)
                        .unwrap();
                let unframed_footnotes =
                    typaxis_layout::prepare_production_footnote_lines(&unframed, limits).unwrap();
                assert_eq!(
                    prepare_production_body_flow(&unframed, &blocks, &unframed_footnotes, limits)
                        .err()
                        .unwrap()
                        .kind,
                    typaxis_pagination::ProductionBodyPaginationErrorKind::PendingRegion(
                        "footnote_frame"
                    )
                );
                let other_blocks = typaxis_layout::prepare_staging_precomposed_vector_blocks(
                    package, profile, limits, admitted, bindings, &math,
                )
                .unwrap();
                assert!(flow.verify(&lines, &other_blocks, limits).is_err());
                assert!(
                    typaxis_pagination::paginate_production_body(&lines, &blocks, limits).is_err()
                );
            },
        );
    }
    assert!(required > records_before_collection);
}

#[test]
fn production_footnote_flow_retains_forced_breaks_and_stable_line_registry() {
    let mut value = production_footnote_flow_fixture();
    let span = value["document"]["footnotes"][0]["blocks"][1]["span"].clone();
    value["document"]["footnotes"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(
            1,
            serde_json::json!({"kind":"page_break","node_id":0,"span":span,"classes":[]}),
        );
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_inline_context(
        &serde_json::to_vec(&value).unwrap(),
        &config(),
        |prepared, package, profile, limits, admitted, bindings| {
            let math = typaxis_layout::prepare_staging_math_vector_flows(
                package, profile, limits, admitted, bindings,
            )
            .unwrap();
            let blocks = typaxis_layout::prepare_staging_precomposed_vector_blocks(
                package, profile, limits, admitted, bindings, &math,
            )
            .unwrap();
            let source = prepared.source_flow();
            let mut calls = 0;
            typaxis_layout::with_converged_production_body_lines(
                package,
                source.navigation(),
                profile,
                limits,
                admitted,
                source,
                bindings,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
                profile.page_geometry().body(),
                100_000,
                |stable| {
                    calls += 1;
                    let flow = typaxis_pagination::prepare_production_body_flow(
                        stable.lines(),
                        &blocks,
                        stable.footnotes(),
                        limits,
                    )
                    .unwrap();
                    assert!(std::ptr::eq(flow.footnotes(), stable.footnotes()));
                    assert!(flow.body_items().iter().all(|item| item.source().is_some()));
                    let first = flow.definition_items(0).unwrap();
                    assert_eq!(
                        first.iter().filter(|item| item.source().is_none()).count(),
                        1
                    );
                    assert!(first[1].source().is_none());
                    assert_eq!(first[1].height(), Length::ZERO);
                    assert!(flow
                        .definition_items(1)
                        .unwrap()
                        .iter()
                        .all(|item| item.source().is_some()));
                },
            )
            .unwrap();
            assert_eq!(calls, 1);
        },
    );
}
