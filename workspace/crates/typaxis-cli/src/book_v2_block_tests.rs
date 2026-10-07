use super::*;
#[path = "book_v2_block_width_tests.rs"]
mod block_widths;
use typaxis_core::Rect;
use typaxis_layout::book_v2::{
    layout_book_v2_body_inline_lines, prepare_book_v2_vector_blocks, BookV2VectorBlockError,
};
pub(in crate::book_v2_resources::tests::shaping_tests) fn block_data(align: &str, numbered: bool) -> Value {
    let mut d = numbered_data();
    d["style_sheet"]["rules"][0]["declarations"][2]["value"]["value"] = 100_000.into();
    d["style_sheet"]["rules"][0]["declarations"][3]["value"]["value"] = 200_000.into();
    for (order, selector) in [(4, "vector_figure"), (5, "math_vector_block")] {
        d["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":format!("block-{order}"),"selector":selector,"source_order":order,"extends":null,"declarations":[
            {"name":"start_indent","important":false,"value":{"kind":"length","value":131072}},
            {"name":"end_indent","important":false,"value":{"kind":"length","value":196608}},
            {"name":"text_align","important":false,"value":{"kind":"keyword","value":align}}
        ]}));
    }
    if !numbered {
        d["document"]["blocks"][0]["blocks"][2]["equation_number"] = Value::Null;
    }
    d
}
fn body() -> Rect {
    let l = |n| Length::from_raw(n).unwrap();
    Rect::new(
        l(500_000),
        l(500_000),
        PositiveLength::new(l(10_000_000)).unwrap(),
        PositiveLength::new(l(20_000_000)).unwrap(),
    )
}
#[test]
fn book_v2_vector_blocks_bind_actual_nested_frames_and_number_geometry() {
    for align in ["start", "center", "end"] {
        let root = Root::new();
        let limits = limits();
        let input = vector_input(&root, block_data(align, align != "end"), &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let shaped = shape_book_v2_authored_text(
            &policy,
            &flow,
            input.resources(),
            &limits,
            bindings.epoch(),
            None,
        )
        .unwrap();
        let prepared = prepare_book_v2_inline_items(
            &flow,
            &shaped,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();
        let lines = layout_book_v2_body_inline_lines(&prepared, body(), 1_000_000).unwrap();
        let numbers = shape_book_v2_equation_numbers(&shaped, &limits, 0).unwrap();
        let blocks = prepare_book_v2_vector_blocks(&lines, numbers.as_ref(), &limits, 0)
            .unwrap()
            .unwrap();
        blocks.verify(&lines, &limits).unwrap();
        assert_eq!(blocks.blocks().len(), 2);
        let extra = 3 + u64::from(align != "end");
        let retained = numbers.as_ref().map_or(0, |n| n.retained_records());
        assert_eq!(
            blocks.record_charge(),
            lines.output_records() + retained + extra
        );
        for (index, block) in blocks.blocks().iter().enumerate() {
            let owner = NodeId::new(7 + index as u32);
            assert_eq!(block.owner(), owner);
            assert!(std::ptr::eq(
                block.binding(),
                bindings.receipt(owner).unwrap()
            ));
            assert!(
                matches!(flow.events()[block.source_event_index()], typaxis_syntax::ProductionFlowEvent::Begin {owner: actual, ..} if actual == owner)
            );
            assert_eq!(block.inner_frame_left().raw(), 731_072);
            assert_eq!(block.inner_frame_width().get().raw(), 9_372_320);
            assert_eq!(block.viewport_width().get().raw(), 1_966_080);
            assert_eq!(
                block.viewport_left().raw(),
                731_072
                    + match align {
                        "start" => 0,
                        "center" => 3_703_120,
                        _ => 7_406_240,
                    }
            );
            assert_eq!(block.viewport_height().get().raw(), 786_432);
            if index == 1 && align != "end" {
                let number = block.equation_number().unwrap();
                let shape = numbers.as_ref().unwrap().shape(owner).unwrap();
                assert_eq!(number.shape_fingerprint(), shape.fingerprint());
                assert_eq!(number.owner(), NodeId::new(9));
                assert_eq!(number.left().raw(), 10_103_392 - shape.width().get().raw());
                assert_eq!(block.content_height().get().raw(), 1_048_576);
                assert_eq!(block.viewport_top_offset().get().raw(), 131_072);
                assert_eq!(number.top_offset().get(), Length::ZERO);
                assert!(
                    block.viewport_left().raw()
                        + block.viewport_width().get().raw()
                        + number.minimum_gap().get().raw()
                        <= number.left().raw()
                );
            } else {
                assert!(block.equation_number().is_none());
                assert_eq!(block.content_height(), block.viewport_height());
            }
        }
        let prior = limits.base().get().max_fragments - extra - retained;
        assert_eq!(
            prepare_book_v2_vector_blocks(&lines, numbers.as_ref(), &limits, prior)
                .unwrap()
                .unwrap()
                .record_charge(),
            limits.base().get().max_fragments
        );
        assert!(matches!(
            prepare_book_v2_vector_blocks(&lines, numbers.as_ref(), &limits, prior + 1),
            Err(BookV2VectorBlockError::OutputLimit)
        ));
        typaxis_layout::book_v2::with_converged_book_v2_body_lines(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            body(),
            1_000_000,
            |stable| {
                let prior = stable.footnotes().record_charge();
                let numbers = shape_book_v2_equation_numbers(
                    stable.lines().prepared().shaped(),
                    &limits,
                    prior,
                )
                .unwrap();
                let final_blocks =
                    prepare_book_v2_vector_blocks(stable.lines(), numbers.as_ref(), &limits, prior)
                        .unwrap()
                        .unwrap();
                final_blocks.verify(stable.lines(), &limits).unwrap();
                use typaxis_pagination::book_v2::prepare_book_v2_body_flow;
                use typaxis_pagination::{ProductionBodyFragmentSource as Source, ProductionBodyPaginationErrorKind as Error};
                let body_flow = prepare_book_v2_body_flow(stable.lines(), Some(&final_blocks), stable.footnotes(), &limits, 0).unwrap();
                body_flow.verify(stable.lines(), Some(&final_blocks), stable.footnotes(), &limits).unwrap();
                let items = body_flow.body_items();
                let paragraph_lines = stable.lines().paragraphs()[0].lines().len();
                assert_eq!(items.len(), paragraph_lines + 2);
                for (line_index, item) in items[..paragraph_lines].iter().enumerate() {
                    assert_eq!(item.source(), Some(Source::ParagraphLine { paragraph_index: 0, line_index: line_index as u32 }));
                    assert_eq!(item.height(), stable.lines().paragraphs()[0].selected().unwrap().lines()[line_index].line().metrics().line_height().get());
                }
                assert!(matches!(items[0].source(), Some(Source::ParagraphLine { paragraph_index: 0, line_index: 0 })));
                for (i, block) in final_blocks.blocks().iter().enumerate() {
                    assert_eq!(items[i + paragraph_lines].source(), Some(Source::VectorBlock { block_index: i as u32 }));
                    assert_eq!(items[i + paragraph_lines].owner(), block.owner());
                    assert_eq!(items[i + paragraph_lines].x(), block.inner_frame_left());
                    assert_eq!(items[i + paragraph_lines].viewport_left(), Some(block.viewport_left()));
                    assert_eq!(items[i + paragraph_lines].height(), block.content_height().get());
                }
                // Owner set + every actual selected leaf, including wrapped paragraph lines.
                let body_records = 1 + items.len() as u64;
                assert_eq!(body_flow.record_charge(), final_blocks.record_charge() + body_records);
                let exact_prior = limits.base().get().max_fragments - final_blocks.retained_records() - body_records;
                assert_eq!(prepare_book_v2_body_flow(stable.lines(), Some(&final_blocks), stable.footnotes(), &limits, exact_prior).unwrap().record_charge(), limits.base().get().max_fragments);
                assert!(matches!(prepare_book_v2_body_flow(stable.lines(), Some(&final_blocks), stable.footnotes(), &limits, exact_prior + 1), Err(e) if e.kind == Error::FragmentLimit));
                assert!(prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0).is_err());
                assert!(body_flow.verify(stable.lines(), Some(&blocks), stable.footnotes(), &limits).is_err());
                assert!(prepare_book_v2_body_flow(stable.lines(), Some(&blocks), stable.footnotes(), &limits, 0).is_err());

                assert_eq!(
                    final_blocks.record_charge(),
                    prior + numbers.as_ref().map_or(0, |n| n.retained_records()) + extra
                );
                for (initial, final_block) in blocks.blocks().iter().zip(final_blocks.blocks()) {
                    assert!(std::ptr::eq(initial.binding(), final_block.binding()));
                    assert_eq!(initial.viewport_left(), final_block.viewport_left());
                    assert_eq!(initial.content_height(), final_block.content_height());
                }
            },
        )
        .unwrap();
        let widths = lines
            .frames()
            .unwrap()
            .paragraphs()
            .iter()
            .map(|f| f.width())
            .collect::<Vec<_>>();
        let no_frames = layout_book_v2_inline_lines(&prepared, &widths, 1_000_000).unwrap();
        assert!(matches!(
            prepare_book_v2_vector_blocks(&no_frames, numbers.as_ref(), &limits, 0),
            Err(BookV2VectorBlockError::MissingFrames)
        ));
        let other = layout_book_v2_body_inline_lines(&prepared, body(), 1_000_000).unwrap();
        assert!(blocks.verify(&other, &limits).is_err());
        if numbers.is_some() {
            assert!(prepare_book_v2_vector_blocks(&lines, None, &limits, 0).is_err());
        }
    }
}
#[test]
fn book_v2_vector_blocks_reject_number_overlap_without_moving_or_shrinking_formula() {
    let root = Root::new();
    let limits = limits();
    let input = vector_input(&root, block_data("end", true), &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let shaped = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let prepared = prepare_book_v2_inline_items(
        &flow,
        &shaped,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    let lines = layout_book_v2_body_inline_lines(&prepared, body(), 1_000_000).unwrap();
    let numbers = shape_book_v2_equation_numbers(&shaped, &limits, 0).unwrap();
    assert!(
        matches!(prepare_book_v2_vector_blocks(&lines, numbers.as_ref(), &limits, 0), Err(BookV2VectorBlockError::Geometry(typaxis_layout::StagingPrecomposedVectorBlockLayoutError::InvalidGeometry(owner, _))) if owner == NodeId::new(8))
    );
}

#[test]
fn book_v2_combined_body_joins_numbered_table_math_figures_list_and_definition_baselines() {
    let root = Root::new();
    let limits = limits();
    let mut data = block_data("start", true);
    let span = data["document"]["blocks"][0]["span"].clone();
    let vector_figure = data["document"]["blocks"][0]["blocks"][1].clone();
    let numbered_math = data["document"]["blocks"][0]["blocks"][2].clone();
    let mut definition_math = numbered_math.clone();
    definition_math["equation_number"] = Value::Null;
    let paragraph = json!({"kind":"paragraph","node_id":0,"classes":[],"span":{"source_id":0,"start_byte":0,"end_byte":7},"children":[{"kind":"text","node_id":0,"span":{"source_id":0,"start_byte":0,"end_byte":7},"text_span":{"text_id":0,"start_byte":0,"end_byte":7}}]});
    let mut body_paragraph = paragraph.clone();
    body_paragraph["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"footnote_reference","node_id":0,"span":{"source_id":0,"start_byte":0,"end_byte":7},"footnote_id":"note"}));
    let mut ordinary_figure = json!({"kind":"figure","node_id":0,"classes":[],"span":span,"image_id":0,"placement":"block","alt":"ordinary source SVG","caption":[paragraph]});
    ordinary_figure["caption"][0]["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":paragraph["span"],"footnote_id":"note"}));
    data["document"]["blocks"][0]["blocks"] = json!([
        body_paragraph,
        {"kind":"table","node_id":0,"span":span,"classes":[],"columns":[{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}],"head":[],"body":[{"node_id":0,"span":span,"cells":[
            {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[numbered_math]},
            {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[ordinary_figure]}]}]},
        {"kind":"list","node_id":0,"span":span,"classes":[],"ordered":true,"start":1,"items":[{"node_id":0,"span":span,"blocks":[vector_figure,paragraph]}]}
    ]);
    data["document"]["footnotes"] = json!([{"node_id":0,"span":span,"footnote_id":"note","blocks":[definition_math,paragraph]}]);
    let mut list_style = data["style_sheet"]["rules"][2].clone();
    list_style["selector"] = "list".into();
    list_style["style_id"] = "list-text".into();
    list_style["source_order"] = 6.into();
    data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(list_style);
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 22_000_000.into();
    master["height"] = 24_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":22_000_000,"height":24_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":20_000_000,"height":20_000_000});
    master["footnote"] = json!({"x":500_000,"y":21_000_000,"width":20_000_000,"height":1_500_000});
    fn renumber(v: &mut Value, next: &mut u32) {
        if v.get("node_id").is_some() {
            v["node_id"] = (*next).into();
            *next += 1;
        }
        for key in [
            "blocks",
            "children",
            "items",
            "head",
            "body",
            "cells",
            "caption",
            "footnotes",
        ] {
            if let Some(children) = v.get_mut(key).and_then(Value::as_array_mut) {
                for child in children {
                    renumber(child, next);
                }
            }
        }
        if let Some(number) = v.get_mut("equation_number").filter(|v| !v.is_null()) {
            renumber(number, next);
        }
    }
    renumber(&mut data["document"], &mut 0);
    let input = vector_input(&root, data, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let page_body = Rect::new(
        body().x(),
        body().y(),
        PositiveLength::new(Length::from_raw(20_000_000).unwrap()).unwrap(),
        body().height(),
    );
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(&policy, &flow, input.resources(), &bindings, &limits, JapaneseLineBreakMode::Normal, page_body, 1_000_000, |stable| {
        use typaxis_pagination::book_v2::{prepare_book_v2_body_flow, prepare_book_v2_table_measurements};
        use typaxis_pagination::{ProductionBodyFragmentSource as Source, ProductionTableContentSource as CellSource};
        let numbers = shape_book_v2_equation_numbers(stable.lines().prepared().shaped(), &limits, 0).unwrap();
        let blocks = prepare_book_v2_vector_blocks(stable.lines(), numbers.as_ref(), &limits, 0).unwrap().unwrap();
        assert_eq!(blocks.blocks().len(), 3);
        let collected = prepare_book_v2_body_flow(stable.lines(), Some(&blocks), stable.footnotes(), &limits, 0).unwrap();
        assert!(collected.record_charge() > stable.footnotes().record_charge() + blocks.retained_records());
        assert_eq!(collected.table_count(), 1);
        assert_eq!(collected.list_marker_count(), 1);
        assert_eq!(collected.references().len(), 2);
        let definition = collected.definition_items(0).unwrap();
        assert!(matches!(definition[0].source(), Some(Source::VectorBlock { block_index: 2 })));
        let math = &blocks.blocks()[2];
        assert_eq!(collected.definition_marker(0).unwrap().baseline(), math.viewport_top_offset().get().checked_add(math.baseline().unwrap().get()).unwrap());
        assert!(collected.body_items().iter().all(|i| i.owner() != math.owner()));
        let mut note_search = typaxis_pagination::book_v2::prepare_book_v2_footnote_search(&collected,&limits,1_000_000,0).unwrap();
        let note_cursor = note_search.begin(0).unwrap();
        let first_height = collected.definition_items(0).unwrap()[0].consumed_height().unwrap();
        let note_first = note_search.evaluate(&note_cursor,first_height).unwrap().unwrap();
        assert_eq!(note_first.items().unwrap().len(),1);
        assert!(matches!(note_first.items().unwrap()[0].source(),Some(Source::VectorBlock {block_index:2})));
        assert!(note_first.marker().is_some());
        let note_last = note_search.evaluate(&note_first.continuation().unwrap(),Length::from_raw(1_500_000).unwrap()).unwrap().unwrap();
        assert!(note_last.marker().is_none());
        assert!(note_last.continuation().is_none());
        let measured = prepare_book_v2_table_measurements(collected, &limits).unwrap();
        let mut demand = typaxis_pagination::book_v2::prepare_book_v2_footnote_demand_search(measured.flow(),&limits,1_000_000,measured.record_charge()).unwrap();
        let base = demand.begin().unwrap();
        assert!(matches!(demand.require_body(&base,0..measured.flow().body_items().len()),Err(e) if e.kind == typaxis_pagination::ProductionBodyPaginationErrorKind::PendingRegion("table_body_demand")));
        assert!(matches!(demand.evaluate_body_candidate(&base,0..measured.flow().body_items().len()),Err(e) if e.kind == typaxis_pagination::ProductionBodyPaginationErrorKind::PendingRegion("table_body_demand")));
        let joined = demand.evaluate_body_candidate(&base,0..1).unwrap().unwrap();
        joined.verify(&base).unwrap();
        assert_eq!(joined.footnotes().unwrap().fragments()[0].fragment().items().unwrap()[0].source(),Some(Source::VectorBlock { block_index: 2 }));
        assert!(joined.footnotes().unwrap().fragments()[0].fragment().marker().is_some());
        assert_eq!(joined.footnote_bounds().unwrap().height().get().raw(),joined.footnotes().unwrap().used_height().raw()+typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW);
        let mut demanded = demand.require_body(&base,0..1).unwrap();
        assert_eq!(demanded.pending_definitions(),[0]);
        let mut label_count = 0;
        let mut fragment_count = 0;
        while !demanded.pending_definitions().is_empty() {
            assert!(fragment_count < 4);
            let selected = demand.evaluate_next(&demanded,Length::from_raw(1_500_000).unwrap()).unwrap().unwrap();
            if fragment_count == 0 { assert!(matches!(selected.fragment().items().unwrap()[0].source(),Some(Source::VectorBlock {block_index:2}))); }
            label_count += usize::from(selected.fragment().marker().is_some());
            demanded = demand.advance(&demanded,&selected).unwrap();
            fragment_count += 1;
        }
        assert_eq!(label_count,1);
        assert_eq!(demanded.status(0),Some(typaxis_pagination::ProductionFootnoteDemandStatus::Complete));
        assert!(base.pending_definitions().is_empty());

        let table = &measured.tables()[0];
        assert_eq!(table.cells().len(), 2);
        assert!(matches!(table.cells()[0].content()[0].source(), CellSource::FlowItem(i) if measured.item(i).unwrap().source() == Some(Source::VectorBlock {block_index:0})));
        assert!(matches!(table.cells()[1].content()[0].source(), CellSource::FlowItem(i) if matches!(measured.item(i).unwrap().source(), Some(Source::Figure { .. }))));
        assert_eq!(table.height(), table.cells().iter().map(|c| c.natural_height()).max().unwrap());
        assert_eq!(numbers.as_ref().unwrap().shapes().len(), 1);
        let mut search = typaxis_pagination::book_v2::prepare_book_v2_table_search(&measured, 0, &limits, 1_000_000, 0).unwrap();
        let cursor = search.begin().unwrap();
        assert!(search.evaluate(&cursor, blocks.blocks()[0].content_height().get().checked_sub(Length::from_raw(1).unwrap()).unwrap()).unwrap().is_none());
        let selected = search.evaluate(&cursor, table.height()).unwrap().unwrap();
        assert!(selected.after().is_terminal());
        assert_eq!(selected.cells().len(), 2);
        for slice in selected.cells() {
            assert_eq!(slice.content_range(), 0..table.cells()[slice.cell_index()].content().len());
        }
        let mut joined = typaxis_pagination::book_v2::prepare_book_v2_table_footnote_search(&measured,0,&limits,1_000_000,0).unwrap();
        let source = joined.begin().unwrap();
        let table_and_note = joined.evaluate(&source,table.height()).unwrap().unwrap();
        table_and_note.verify(&source).unwrap();
        assert!(table_and_note.table().unwrap().after().is_terminal());
        let first_note = table_and_note.footnotes().unwrap().fragments()[0].fragment();
        assert_eq!(first_note.items().unwrap()[0].source(),Some(Source::VectorBlock { block_index: 2 }));
        assert!(first_note.marker().is_some());
        assert!(!table_and_note.next_state().is_complete());
        let tail = joined.evaluate(table_and_note.next_state(),Length::ZERO).unwrap().unwrap();
        assert!(tail.table().is_none());
        assert!(tail.footnotes().unwrap().fragments()[0].fragment().marker().is_none());
        assert!(tail.next_state().is_complete());
        assert!(source.demand().pending_definitions().is_empty());
        use typaxis_pagination::book_v2::BookV2BodyCandidatePart as Part;
        let mut mixed=typaxis_pagination::book_v2::prepare_book_v2_table_body_search(&measured,&limits,1_000_000,0).unwrap();
        let origin=mixed.begin_body_source().unwrap();
        let table_cursor=mixed.begin_table(0).unwrap();
        let end=measured.flow().body_items().len();
        let page=mixed.evaluate_mixed_candidate(&origin,&[Part::Items {end:1},Part::Table {cursor:table_cursor,capacity:table.height()},Part::Items {end}]).unwrap().unwrap();
        page.verify(&origin).unwrap();
        assert_eq!(page.parts().len(),3);
        assert!(page.parts()[1].table().unwrap().after().is_terminal());
        assert_eq!(page.next_state().next_item(),end);
        assert_eq!(page.next_state().next_table_index(),1);
        assert!(page.next_state().table_continuation().is_none());
        assert!(!page.next_state().is_complete());
        assert_eq!(page.footnotes().unwrap().fragments()[0].fragment().items().unwrap()[0].source(),Some(Source::VectorBlock {block_index:2}));
        let notes_only=mixed.evaluate_mixed_candidate(page.next_state(),&[]).unwrap().unwrap();
        assert!(notes_only.parts().is_empty());
        assert_eq!(notes_only.used_height(),Length::ZERO);
        assert!(notes_only.next_state().is_complete());
        assert!(notes_only.footnotes().unwrap().fragments()[0].fragment().marker().is_none());
        let pages=mixed.select_mixed_pages().unwrap();
        mixed.verify_mixed_sequence(&pages).unwrap();
        assert!(pages.pages().len()>=2);
        let mut body_items=Vec::new();
        let mut note_labels=0;
        for page in pages.pages() {
            for part in page.candidate().parts() {
                if let Some(items)=part.items() { body_items.extend(items); }
                if let Some(table)=part.table() { body_items.extend(table.semantic_leaf_ranges().flatten()); }
            }
            if let Some(region)=page.candidate().footnotes() {
                for fragment in region.fragments() { note_labels+=usize::from(fragment.fragment().marker().is_some()); }
            }
        }
        body_items.sort_unstable();
        assert_eq!(body_items,(0..end).collect::<Vec<_>>());
        assert_eq!(note_labels,1);
        assert!(pages.pages().last().unwrap().next_state().is_complete());
        let placed=mixed.place_mixed_pages(&pages).unwrap();
        assert!(std::ptr::eq(placed.sequence(),&pages));
        let mut labels=0;
        let mut lists=0;
        let mut vectors=0;
        for page in placed.pages() {
            labels+=page.footnote_markers().len();
            lists+=page.list_markers().len();
            for fragment in page.fragments() {
                if let Source::VectorBlock {block_index}=fragment.fragment().source() {
                    vectors+=1;
                    let source=&blocks.blocks()[block_index as usize];
                    assert_eq!(fragment.fragment().viewport().unwrap().width(),source.viewport_width());
                    assert_eq!(fragment.fragment().viewport().unwrap().height(),source.viewport_height());
                }
            }
        }
        assert_eq!(labels,1);
        assert_eq!(lists,1);
        assert_eq!(vectors,3);
        let stable_pages=mixed.select_stable_mixed_pages(2).unwrap();
        let geometry=mixed.place_mixed_pages(stable_pages.sequence()).unwrap();
        let closure=mixed.close_mixed_page_sources(&stable_pages,&geometry).unwrap();
        assert_eq!(closure.unreferenced_definitions(),0);
        // Two formulas; the third vector is an ordinary figure.
        assert_eq!(closure.semantic_math(),2);
        assert_eq!(closure.repeated_math(),0);
        let terminals=mixed.finalize_mixed_page_math(closure,&limits,0).unwrap();
        assert_math_terminals(&terminals);
        assert_math_display(&terminals,input.resources(),&limits);
        assert_eq!(terminals.terminals().iter().filter(|t|t.definition_index().is_some()).count(),1);



    }).unwrap();
}

#[path="book_v2_page_number_tests.rs"]
mod page_numbers;
