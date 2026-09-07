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
                    records_before_collection + 2 + count as u64 + 4 + 2
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

fn production_footnote_break_fixture() -> serde_json::Value {
    use serde_json::json;
    let text = "A B ".repeat(8);
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A ", &text], "Body")).unwrap();
    let span = json!({"source_id":0,"start_byte":0,"end_byte":0});
    value["document"]["blocks"][0]["blocks"][0]["children"][1] =
        json!({"kind":"footnote_reference","node_id":4,"span":span,"footnote_id":"note"});
    value["document"]["footnotes"] = json!([{"node_id":5,"span":span,"footnote_id":"note",
    "blocks":[{"kind":"paragraph","node_id":6,"span":span,"classes":[],"children":[{
        "kind":"text","node_id":7,"span":span,"text_span":{"text_id":1,"start_byte":0,"end_byte":text.len()}
    }]}]}]);
    value["page_masters"]["masters"][0]["footnote"] =
        json!({"x":655360,"y":13000000,"width":3000000,"height":6000000});
    value
}

fn with_production_footnote_prepared(
    value: &serde_json::Value,
    cfg: &EffectiveConfig,
    check: impl FnOnce(
        &typaxis_pagination::ProductionPreparedBodyFlow<'_, '_, '_, '_>,
        &typaxis_core::M4EffectiveResourceLimits,
    ),
) {
    with_production_inline_context(
        &serde_json::to_vec(value).unwrap(),
        cfg,
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
            let flow = typaxis_pagination::prepare_production_body_flow(
                &lines, &blocks, &footnotes, limits,
            )
            .unwrap();
            check(&flow, limits);
        },
    );
}

#[test]
fn production_footnote_breaks_preserve_continuation_and_reject_foreign_cursors() {
    use typaxis_pagination::{
        prepare_production_footnote_search, ProductionBodyBreakReason as Reason,
    };
    let value = production_footnote_break_fixture();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let items = flow.definition_items(0).unwrap();
        assert!(items.len() >= 6);
        let height = items[0].consumed_height().unwrap();
        let capacity = height.checked_add(height).unwrap();
        let mut search = prepare_production_footnote_search(flow, limits, 100_000).unwrap();
        let mut cursor = search.begin(0).unwrap();
        assert_eq!(cursor.definition_index(), 0);
        assert_eq!(cursor.next_item(), 0);
        let before = search.record_charge();
        assert!(search.evaluate(&cursor, Length::ZERO).unwrap().is_none());
        assert!(search.record_charge() > before);
        let mut end = 0;
        let mut source_items = Vec::new();
        loop {
            let selected = search.evaluate(&cursor, capacity).unwrap().unwrap();
            selected.verify(flow).unwrap();
            assert_eq!(selected.consumed_range().start, end);
            assert_eq!(
                selected.marker().is_some(),
                selected
                    .consumed_range()
                    .contains(&flow.definition_marker(0).unwrap().item_index())
            );
            assert!(!selected.items().is_empty());
            assert!(selected.used_height() <= capacity);
            assert_eq!(selected.available_height(), capacity);
            assert!(selected.forced_break_owner().is_none());
            assert!(selected.selected_candidate_index().is_some());
            let expected_height = selected
                .items()
                .iter()
                .enumerate()
                .try_fold(Length::ZERO, |total, (index, item)| {
                    let space = if index == 0 {
                        Length::ZERO
                    } else {
                        selected.items()[index - 1]
                            .space_after()
                            .checked_add(item.space_before())
                            .unwrap()
                    };
                    total
                        .checked_add(space)?
                        .checked_add(item.consumed_height().unwrap())
                })
                .unwrap();
            assert_eq!(selected.used_height(), expected_height);
            source_items.extend(selected.items().iter().map(|i| i.source()));
            end = selected.consumed_range().end;
            let Some(next) = selected.continuation() else {
                assert_eq!(selected.reason(), Reason::End);
                break;
            };
            assert_eq!(selected.reason(), Reason::Overflow);
            assert_eq!(next.next_item(), end);
            cursor = next;
        }
        assert_eq!(end, items.len());
        assert_eq!(
            source_items,
            items.iter().map(|i| i.source()).collect::<Vec<_>>()
        );
        with_production_footnote_prepared(&value, &config(), |other, other_limits| {
            let mut other_search =
                prepare_production_footnote_search(other, other_limits, 100_000).unwrap();
            let foreign = other_search.begin(0).unwrap();
            assert_eq!(
                search.evaluate(&foreign, capacity).err().unwrap().kind,
                typaxis_pagination::ProductionBodyPaginationErrorKind::ReceiptMismatch
            );
            let selected = other_search.evaluate(&foreign, capacity).unwrap().unwrap();
            assert!(selected.verify(flow).is_err());
        });
    });
}

#[test]
fn production_footnote_breaks_accumulate_candidate_records_and_visited_work() {
    use typaxis_pagination::{
        prepare_production_footnote_search, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_footnote_break_fixture();
    let mut required_records = 0;
    let mut required_work = 0;
    for (record_delta, work_delta) in [
        (None, None),
        (Some(0), None),
        (Some(1), None),
        (None, Some(0)),
        (None, Some(1)),
    ] {
        let cfg = record_delta.map_or_else(config, |delta| {
            config_with_limits(ResourceLimits {
                max_fragments: required_records - delta,
                ..ResourceLimits::default()
            })
        });
        with_production_footnote_prepared(&value, &cfg, |flow, limits| {
            let capacity =
                Length::from_raw(2 * flow.definition_items(0).unwrap()[0].height().raw()).unwrap();
            let work_limit = work_delta.map_or(100_000, |delta| required_work - delta);
            let mut search = prepare_production_footnote_search(flow, limits, work_limit).unwrap();
            let cursor = search.begin(0).unwrap();
            let result = search.evaluate(&cursor, capacity);
            if record_delta == Some(1) {
                assert_eq!(result.err().unwrap().kind, E::FragmentLimit);
            } else if work_delta == Some(1) {
                assert_eq!(result.err().unwrap().kind, E::FootnoteSearchLimit);
            } else {
                let selected = result.unwrap().unwrap();
                assert_eq!(selected.items().len(), 2);
                required_records = search.record_charge();
                required_work = search.visited_items();
                drop(selected);
                if work_delta == Some(0) {
                    assert_eq!(
                        search.evaluate(&cursor, capacity).err().unwrap().kind,
                        E::FootnoteSearchLimit
                    );
                } else if record_delta == Some(0) {
                    assert_eq!(
                        search.evaluate(&cursor, capacity).err().unwrap().kind,
                        E::FragmentLimit
                    );
                }
            }
        });
    }
    let cfg = config_with_limits(ResourceLimits {
        max_page_break_lookback: 1,
        ..ResourceLimits::default()
    });
    with_production_footnote_prepared(&value, &cfg, |flow, limits| {
        let capacity =
            Length::from_raw(2 * flow.definition_items(0).unwrap()[0].height().raw()).unwrap();
        let mut search = prepare_production_footnote_search(flow, limits, 100_000).unwrap();
        let cursor = search.begin(0).unwrap();
        assert_eq!(
            search.evaluate(&cursor, capacity).err().unwrap().kind,
            E::PageBreakLookbackLimit {
                limit: 1,
                observed: 2
            }
        );
    });
}

#[test]
fn production_footnote_breaks_consume_leading_consecutive_and_trailing_forced_breaks_once() {
    use typaxis_pagination::{
        prepare_production_footnote_search, ProductionBodyBreakReason as Reason,
    };
    let mut value = production_footnote_break_fixture();
    let paragraph = value["document"]["footnotes"][0]["blocks"][0].clone();
    let forced =
        serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":paragraph["span"]});
    value["document"]["footnotes"][0]["blocks"] =
        serde_json::json!([forced, forced, paragraph, forced]);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_search(flow, limits, 100_000).unwrap();
        let mut cursor = search.begin(0).unwrap();
        let mut end = 0;
        let mut breaks = Vec::new();
        let mut content = Vec::new();
        loop {
            let capacity = if end < 2 {
                Length::ZERO
            } else {
                Length::from_raw(2 * 917_504).unwrap()
            };
            let selected = search.evaluate(&cursor, capacity).unwrap().unwrap();
            assert_eq!(selected.consumed_range().start, end);
            assert_eq!(
                selected.marker().is_some(),
                selected
                    .consumed_range()
                    .contains(&flow.definition_marker(0).unwrap().item_index())
            );
            assert!(selected.consumed_range().end > end);
            if end < 2 {
                assert!(selected.items().is_empty());
                assert!(selected.candidates().is_empty());
                assert_eq!(selected.used_height(), Length::ZERO);
            }
            if let Some(owner) = selected.forced_break_owner() {
                assert_eq!(selected.reason(), Reason::Forced);
                breaks.push(owner);
            }
            content.extend(selected.items().iter().map(|i| i.source()));
            end = selected.consumed_range().end;
            let Some(next) = selected.continuation() else {
                assert_eq!(selected.reason(), Reason::Forced);
                break;
            };
            cursor = next;
        }
        let items = flow.definition_items(0).unwrap();
        assert_eq!(end, items.len());
        assert_eq!(
            breaks,
            items
                .iter()
                .filter(|i| i.source().is_none())
                .map(|i| i.owner())
                .collect::<Vec<_>>()
        );
        assert_eq!(breaks.len(), 3);
        assert_eq!(
            content,
            items
                .iter()
                .filter(|i| i.source().is_some())
                .map(|i| i.source())
                .collect::<Vec<_>>()
        );
    });
}

#[test]
fn production_footnote_breaks_respect_keep_spacing_and_maximum_region_height() {
    use typaxis_pagination::{
        prepare_production_footnote_search, ProductionBodyPaginationErrorKind as E,
    };
    let mut value = production_footnote_break_fixture();
    let mut paragraph = value["document"]["footnotes"][0]["blocks"][0].clone();
    paragraph["children"][0]["text_span"]["end_byte"] = 1.into();
    value["document"]["footnotes"][0]["blocks"] =
        serde_json::json!([paragraph, paragraph, paragraph, paragraph]);
    production_body_renumber(&mut value["document"], &mut 0);
    for keep in [false, true] {
        production_body_set_style(&mut value, "paragraph", "keep_with_next", keep.into());
        with_production_footnote_prepared(&value, &config(), |flow, limits| {
            let mut search = prepare_production_footnote_search(flow, limits, 100_000).unwrap();
            let cursor = search.begin(0).unwrap();
            let items = flow.definition_items(0).unwrap();
            assert_eq!(items.len(), 4);
            let two_height = items[0]
                .consumed_height()
                .unwrap()
                .checked_add(items[0].space_after())
                .unwrap()
                .checked_add(items[1].space_before())
                .unwrap()
                .checked_add(items[1].consumed_height().unwrap())
                .unwrap();
            let partial = search.evaluate(&cursor, two_height).unwrap();
            if keep {
                assert!(partial.is_none());
            } else {
                let partial = partial.unwrap();
                assert_eq!(partial.items().len(), 2);
                assert_eq!(partial.used_height(), two_height);
            }
            let maximum = flow.footnote_region().unwrap().height().get();
            let complete = search.evaluate(&cursor, maximum).unwrap().unwrap();
            assert_eq!(complete.items().len(), 4);
            assert!(complete.continuation().is_none());
            assert_eq!(complete.used_height().raw(), 4 * 917_504 + 6 * 65_536);
            for invalid in [
                Length::from_raw(-1).unwrap(),
                maximum.checked_add(Length::from_raw(1).unwrap()).unwrap(),
            ] {
                assert_eq!(
                    search.evaluate(&cursor, invalid).err().unwrap().kind,
                    E::InvalidFootnoteCapacity
                );
            }
        });
    }
    let forced =
        serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":paragraph["span"]});
    value["document"]["footnotes"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(1, forced);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        assert_eq!(
            prepare_production_footnote_search(flow, limits, 100_000)
                .err()
                .unwrap()
                .kind,
            E::KeepAcrossForcedBreak
        );
    });
    let mut oversize = production_footnote_flow_fixture();
    oversize["page_masters"]["masters"][0]["footnote"]["height"] = 1.into();
    with_production_footnote_prepared(&oversize, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_search(flow, limits, 100_000).unwrap();
        let cursor = search.begin(0).unwrap();
        assert!(search.evaluate(&cursor, Length::ZERO).unwrap().is_none());
        assert_eq!(
            search
                .evaluate(&cursor, Length::from_raw(1).unwrap())
                .err()
                .unwrap()
                .kind,
            E::Oversize
        );
    });
}

fn production_footnote_marker_style(value: &mut serde_json::Value, family: &str, size: i64) {
    let rules = value["style_sheet"]["rules"].as_array_mut().unwrap();
    let mut rule = rules
        .iter()
        .find(|r| r["selector"] == "paragraph")
        .unwrap()
        .clone();
    rule["selector"] = "paragraph.note".into();
    rule["style_id"] = "footnote-marker-test".into();
    rule["source_order"] = (rules
        .iter()
        .map(|r| r["source_order"].as_u64().unwrap())
        .max()
        .unwrap()
        + 1)
    .into();
    for declaration in rule["declarations"].as_array_mut().unwrap() {
        if declaration["name"] == "font_family" {
            declaration["value"]["families"] = serde_json::json!([family]);
        }
        if declaration["name"] == "font_size" {
            declaration["value"]["value"] = size.into();
        }
    }
    rules.push(rule);
    value["document"]["footnotes"][0]["blocks"][0]["classes"] = serde_json::json!(["note"]);
}

#[test]
fn production_footnote_marker_glyphs_use_definition_style_and_generated_owner() {
    let mut value = production_footnote_break_fixture();
    production_footnote_marker_style(&mut value, "Body", 1_048_576);
    with_production_inline_context(
        &serde_json::to_vec(&value).unwrap(),
        &config(),
        |prepared, _, _, _, admitted, _| {
            let flow = prepared.source_flow();
            let markers = prepared.footnote_markers();
            assert_eq!(markers.len(), 1);
            let marker = &markers[0];
            assert!(std::ptr::eq(
                marker.source(),
                &flow.footnote_definitions()[0]
            ));
            assert_eq!(marker.source().owner().get(), 5);
            assert_eq!(marker.source().id(), "note");
            assert_eq!(marker.source().style_paragraph_index(), Some(1));
            assert_eq!(marker.definition_index(), 0);
            assert_eq!(marker.utf8(), "1");
            assert_eq!(marker.font().size().get().raw(), 1_048_576);
            assert_ne!(
                marker.font().size(),
                flow.paragraphs()[0].style().font_size().unwrap()
            );
            assert_eq!(
                marker.font().content_hash(),
                admitted
                    .font(marker.font().face_id())
                    .unwrap()
                    .content_hash()
            );
            assert_eq!(
                marker.provenance(),
                flow.footnote_marker_provenance(marker.source().owner())
                    .unwrap()
            );
            assert_eq!(
                marker.provenance().buffer_key().generation_kind(),
                typaxis_core::GenerationKind::FootnoteMarker
            );
            assert_eq!(
                marker.glyph_run().source_span,
                typaxis_shaping::ShapeSourceSpan::Generated(marker.provenance())
            );
            assert!(marker
                .glyph_run()
                .glyphs
                .iter()
                .all(|g| g.original_gid.get() != 0));
            assert_eq!(
                marker.advance().get().raw(),
                marker
                    .glyph_run()
                    .glyphs
                    .iter()
                    .map(|g| g.advance_x.raw())
                    .sum::<i64>()
            );
            assert_ne!(
                marker.provenance().buffer_key(),
                flow.footnote_marker_provenance(typaxis_core::NodeId::new(4))
                    .unwrap()
                    .buffer_key()
            );
        },
    );
}

#[test]
fn production_footnote_marker_vector_only_definition_uses_declared_base_style() {
    let mut value = production_footnote_flow_fixture();
    value["document"]["footnotes"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_inline_context(
        &serde_json::to_vec(&value).unwrap(),
        &config(),
        |prepared, _, _, _, _, _| {
            let marker = &prepared.footnote_markers()[0];
            assert_eq!(marker.source().style_paragraph_index(), None);
            assert_eq!(marker.utf8(), "1");
            assert_eq!(marker.font().size().get().raw(), 786_432);
            assert_eq!(prepared.footnote_markers()[1].utf8(), "2");
            assert!(marker.advance().get() > Length::ZERO);
        },
    );
}

#[test]
fn production_footnote_marker_shape_budget_is_shared_and_missing_coverage_reports_definition() {
    let value = production_footnote_break_fixture();
    let bytes = serde_json::to_vec(&value).unwrap();
    let mut required = 0;
    for delta in [None, Some(0), Some(1)] {
        let cfg = delta.map_or_else(config, |d| {
            config_with_limits(ResourceLimits {
                max_fragments: required - d,
                ..ResourceLimits::default()
            })
        });
        let (package, navigation, limits, admitted) = production_text_fixture(&bytes, &cfg);
        let flow =
            typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
        let result = typaxis_shaping::shape_production_authored_text(
            &package,
            &navigation,
            &flow,
            &admitted,
            &limits,
            sha256(b"footnote-markers"),
        );
        if delta == Some(1) {
            let failure = result.err().unwrap();
            assert_eq!(failure.owner.get(), 5);
            assert_eq!(
                failure.kind,
                typaxis_shaping::ProductionTextShapeErrorKind::OutputLimit
            );
        } else {
            let shape = result.unwrap();
            required = shape.output_records();
            let again = typaxis_shaping::shape_production_authored_text(
                &package,
                &navigation,
                &flow,
                &admitted,
                &limits,
                sha256(b"footnote-markers"),
            )
            .unwrap();
            assert_eq!(shape.fingerprint(), again.fingerprint());
            assert_eq!(
                shape.footnote_markers()[0].fingerprint(),
                again.footnote_markers()[0].fingerprint()
            );
        }
    }
    let mut value = value;
    value["document"]["footnotes"][0]["blocks"][0]["children"][0]["text_span"]["end_byte"] =
        1.into();
    production_footnote_marker_style(&mut value, "Typaxis CFF Fixture", 786_432);
    let (package, navigation, limits, admitted) =
        production_text_fixture(&serde_json::to_vec(&value).unwrap(), &config());
    let flow =
        typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
    let failure = typaxis_shaping::shape_production_authored_text(
        &package,
        &navigation,
        &flow,
        &admitted,
        &limits,
        sha256(b"footnote-markers"),
    )
    .err()
    .unwrap();
    assert_eq!(failure.owner.get(), 5);
    assert_eq!(
        failure.kind,
        typaxis_shaping::ProductionTextShapeErrorKind::MissingDeclaredFontCoverage
    );
}

#[test]
fn production_footnote_columns_share_actual_marker_extents_and_reject_exhaustion() {
    let mut value = production_footnote_flow_fixture();
    production_footnote_marker_style(&mut value, "Body", 32 * 65_536);
    value["document"]["footnotes"][1]["blocks"][0]["classes"] = serde_json::json!(["note"]);
    let mut reserved = 0;
    with_production_inline_context(
        &serde_json::to_vec(&value).unwrap(),
        &config(),
        |prepared, _, profile, _, _, _| {
            let markers = prepared.footnote_markers();
            assert_ne!(markers[0].font().size(), markers[1].font().size());
            let width = markers.iter().map(|m| m.advance().get()).max().unwrap();
            let gap = markers.iter().map(|m| m.font().size().get()).max().unwrap();
            reserved = width.raw() + gap.raw();
            let lines = typaxis_layout::layout_production_body_inline_lines(
                prepared,
                profile.page_geometry().body(),
                100_000,
            )
            .unwrap();
            let frames = lines.frames().unwrap();
            let region = frames.footnote_region().unwrap();
            for (index, column) in frames.footnotes().iter().enumerate() {
                assert_eq!(column.owner(), markers[index].source().owner());
                assert_eq!(column.marker_width().get(), width);
                assert_eq!(column.marker_gap().get(), gap);
                assert_eq!(
                    column.content().width().get().raw(),
                    region.width().get().raw() - reserved
                );
                assert_eq!(
                    column.content().start().raw(),
                    region.x().raw() - frames.body().x().raw() + reserved
                );
                assert_eq!(frames.region(column.owner()), Some(column.content()));
            }
            assert_eq!(
                frames.footnotes()[0].content(),
                frames.footnotes()[1].content()
            );
        },
    );
    for width in [reserved, reserved - 1] {
        value["page_masters"]["masters"][0]["footnote"]["width"] = width.into();
        with_production_inline_context(
            &serde_json::to_vec(&value).unwrap(),
            &config(),
            |prepared, _, profile, _, _, _| {
                let failure = typaxis_layout::layout_production_body_inline_lines(
                    prepared,
                    profile.page_geometry().body(),
                    100_000,
                )
                .err()
                .unwrap();
                assert_eq!(
                    failure.owner,
                    prepared.footnote_markers()[0].source().owner()
                );
                assert_eq!(
                    failure.kind,
                    typaxis_layout::ProductionInlinePreparationErrorKind::FootnoteFrameExhausted
                );
            },
        );
    }
}

#[test]
fn production_footnote_marker_metrics_join_real_baselines_and_merge_list_extents() {
    use typaxis_pagination::{
        prepare_production_body_flow, prepare_production_footnote_search,
        ProductionBodyFragmentSource as Source,
    };
    for kind in ["paragraph", "vector", "list", "raster"] {
        let mut value = production_footnote_flow_fixture();
        let paragraph = value["document"]["footnotes"][1]["blocks"][0].clone();
        let first = match kind {
            "vector" => value["document"]["footnotes"][0]["blocks"][0].clone(),
            "list" => {
                let mut list = value["document"]["footnotes"][0]["blocks"][1].clone();
                list["span"] = value["document"]["footnotes"][0]["span"].clone();
                list["items"].as_array_mut().unwrap().truncate(1);
                list["items"][0]["span"] = list["span"].clone();
                list["items"][0]["blocks"] =
                    serde_json::json!([value["document"]["footnotes"][0]["blocks"][0], paragraph]);
                list
            }
            "raster" => {
                let raster =
                    production_raster_fixture("orientation-alpha.png", 1_500_000, 10_000_000);
                value["resources"]["images"] = raster["resources"]["images"].clone();
                let mut figure = raster["document"]["blocks"][0]["blocks"][2].clone();
                figure["caption"] = serde_json::json!([]);
                production_body_set_style(&mut value, "figure", "width", 1_500_000.into());
                production_body_set_style(&mut value, "figure", "keep_caption", false.into());
                figure
            }
            _ => paragraph.clone(),
        };
        value["document"]["footnotes"][0]["blocks"] =
            serde_json::json!([first, paragraph, paragraph]);
        production_footnote_marker_style(&mut value, "Body", 64 * 65_536);
        fn mark_paragraphs(node: &mut serde_json::Value) {
            if node["kind"] == "paragraph" {
                node["classes"] = serde_json::json!(["note"]);
            }
            match node {
                serde_json::Value::Object(object) => {
                    for child in object.values_mut() {
                        mark_paragraphs(child);
                    }
                }
                serde_json::Value::Array(array) => {
                    for child in array {
                        mark_paragraphs(child);
                    }
                }
                _ => {}
            }
        }
        mark_paragraphs(&mut value["document"]["footnotes"]);
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
                let lines = typaxis_layout::layout_production_body_inline_lines(
                    prepared,
                    profile.page_geometry().body(),
                    100_000,
                )
                .unwrap();
                let registry =
                    typaxis_layout::prepare_production_footnote_lines(&lines, limits).unwrap();
                let flow =
                    prepare_production_body_flow(&lines, &blocks, &registry, limits).unwrap();
                let binding = flow.definition_marker(0).unwrap();
                let marker = &lines.footnote_markers()[0];
                assert_eq!(binding.owner(), marker.source().owner());
                assert_eq!(binding.definition_index(), 0);
                assert_eq!(binding.item_index(), 0);
                assert!(flow.definition_marker(2).is_none());
                let item = &flow.definition_items(0).unwrap()[0];
                let baseline = match item.source().unwrap() {
                    Source::ParagraphLine {
                        paragraph_index,
                        line_index,
                    } => lines.paragraphs()[paragraph_index as usize].lines()[line_index as usize]
                        .baseline(),
                    Source::VectorBlock { block_index } => {
                        let block = &blocks.blocks()[block_index as usize];
                        block
                            .viewport_top_offset()
                            .get()
                            .checked_add(block.baseline().unwrap().get())
                            .unwrap()
                    }
                    Source::RasterFigure { .. } => marker.font().ascender(),
                };
                assert_eq!(binding.baseline(), baseline);
                let font = marker.font();
                let mut leading = font
                    .ascender()
                    .checked_sub(baseline)
                    .unwrap()
                    .max(Length::ZERO);
                let mut trailing = baseline
                    .checked_sub(font.descender())
                    .unwrap()
                    .checked_sub(item.height())
                    .unwrap()
                    .max(Length::ZERO);
                if kind == "list" {
                    let list_font = lines.list_markers()[0].font();
                    leading = leading.max(
                        list_font
                            .ascender()
                            .checked_sub(baseline)
                            .unwrap()
                            .max(Length::ZERO),
                    );
                    trailing = trailing.max(
                        baseline
                            .checked_sub(list_font.descender())
                            .unwrap()
                            .checked_sub(item.height())
                            .unwrap()
                            .max(Length::ZERO),
                    );
                }
                // Both definition and list labels share the same first vector.
                // Taking maxima prevents counting the same vertical area twice.
                assert_eq!(item.leading(), leading, "{kind}");
                assert_eq!(item.trailing(), trailing, "{kind}");
                let height = item
                    .height()
                    .checked_add(leading)
                    .unwrap()
                    .checked_add(trailing)
                    .unwrap();
                assert_eq!(item.consumed_height().unwrap(), height);
                if kind != "paragraph" {
                    assert!(leading > Length::ZERO || trailing > Length::ZERO, "{kind}");
                }
                let mut search =
                    prepare_production_footnote_search(&flow, limits, 100_000).unwrap();
                let cursor = search.begin(0).unwrap();
                assert!(search
                    .evaluate(
                        &cursor,
                        height.checked_sub(Length::from_raw(1).unwrap()).unwrap()
                    )
                    .unwrap()
                    .is_none());
                let selected = search.evaluate(&cursor, height).unwrap().unwrap();
                assert_eq!(selected.items().len(), 1);
                assert_eq!(selected.used_height(), height);
                assert!(std::ptr::eq(selected.marker().unwrap(), binding));
                let next = selected.continuation().unwrap();
                let continuation = search
                    .evaluate(&next, flow.footnote_region().unwrap().height().get())
                    .unwrap()
                    .unwrap();
                assert!(continuation.marker().is_none());
            },
        );
    }
}

#[test]
fn production_footnote_marker_rejects_definition_without_a_paint_anchor() {
    let mut value = production_footnote_break_fixture();
    let span = value["document"]["footnotes"][0]["span"].clone();
    value["document"]["footnotes"][0]["blocks"] = serde_json::json!([
        {"kind":"page_break","node_id":0,"span":span,"classes":[]}
    ]);
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
            let lines = typaxis_layout::layout_production_body_inline_lines(
                prepared,
                profile.page_geometry().body(),
                100_000,
            )
            .unwrap();
            let registry =
                typaxis_layout::prepare_production_footnote_lines(&lines, limits).unwrap();
            let failure = typaxis_pagination::prepare_production_body_flow(
                &lines, &blocks, &registry, limits,
            )
            .err()
            .unwrap();
            assert_eq!(failure.owner, registry.definitions()[0].owner());
            assert_eq!(
                failure.kind,
                typaxis_pagination::ProductionBodyPaginationErrorKind::EmptyFootnote
            );
        },
    );
}
