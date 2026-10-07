use super::*;
use typaxis_core::{Length, NodeId, PositiveLength, Rect};
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, layout_book_v2_body_inline_lines, prepare_book_v2_footnote_lines,
    prepare_book_v2_text_inlines, with_converged_book_v2_body_lines,
};
use typaxis_layout::{ProductionInlinePreparationErrorKind as E, ProductionPlacedInline};
use typaxis_linebreak::JapaneseLineBreakMode;

fn renumber(value: &mut Value, next: &mut u32) {
    if value.get("node_id").is_some() {
        value["node_id"] = (*next).into();
        *next += 1;
    }
    for key in [
        "blocks",
        "children",
        "items",
        "head",
        "body",
        "cells",
        "footnotes",
    ] {
        if let Some(array) = value.get_mut(key).and_then(Value::as_array_mut) {
            for child in array {
                renumber(child, next);
            }
        }
    }
}
fn input_data() -> Value {
    let mut data = source_data("Result");
    let span = data["document"]["blocks"][0]["span"].clone();
    let paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    let ids = (0..12)
        .map(|i| format!("note-{:02}", 12 - i))
        .collect::<Vec<_>>();
    let reference = |i: usize| json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":ids[i]});
    let children = data["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap();
    for i in 0..12 {
        children.push(reference(i));
    }
    children.push(reference(11));
    let definitions = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let mut p = paragraph.clone();
            if i == 0 {
                p["children"].as_array_mut().unwrap().push(reference(1));
            }
            json!({"node_id":0,"span":span,"footnote_id":id,"blocks":[p]})
        })
        .collect::<Vec<_>>();
    data["document"]["footnotes"] = json!(definitions);
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 32_000_000.into();
    master["height"] = 24_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":32_000_000,"height":24_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":30_000_000,"height":20_000_000});
    master["footnote"] = json!({"x":500_000,"y":21_000_000,"width":30_000_000,"height":2_000_000});
    renumber(&mut data["document"], &mut 0);
    data
}
fn body() -> Rect {
    let length = |n| Length::from_raw(n).unwrap();
    Rect::new(
        length(500_000),
        length(500_000),
        PositiveLength::new(length(30_000_000)).unwrap(),
        PositiveLength::new(length(20_000_000)).unwrap(),
    )
}
#[test]
fn book_v2_footnotes_join_unsorted_definitions_repeated_multidigit_and_nested_references() {
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, input_data(), b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let shaped =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
            .unwrap();
    let prepared = prepare_book_v2_text_inlines(
        &flow,
        &shaped,
        input.resources(),
        &limits,
        EPOCH,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    let lines = layout_book_v2_body_inline_lines(&prepared, body(), 1_000_000).unwrap();
    let footnotes = prepare_book_v2_footnote_lines(&lines, &limits).unwrap();
    footnotes.verify(&lines, &limits).unwrap();
    assert_eq!(footnotes.definitions().len(), 12);
    assert_eq!(footnotes.references().len(), 14);
    // 12 definitions, 12 unsorted-id index entries, 14 references + coverage.
    assert_eq!(
        footnotes.record_charge(),
        lines.output_records() + 12 + 12 + 28
    );
    for (index, definition) in footnotes.definitions().iter().enumerate() {
        let source = &flow.footnote_definitions()[index];
        assert_eq!(definition.owner(), source.owner());
        assert_eq!(definition.id().as_ptr(), source.id().as_ptr());
        assert_eq!(definition.number(), index as u32 + 1);
        assert_eq!(definition.paragraph_range(), index + 1..index + 2);
        let events = definition.event_range();
        assert!(
            matches!(flow.events()[events.start], typaxis_syntax::ProductionFlowEvent::Begin {owner, kind: typaxis_syntax::ProductionFlowRegionKind::Footnote} if owner == source.owner())
        );
        assert!(
            matches!(flow.events()[events.end-1], typaxis_syntax::ProductionFlowEvent::End {owner} if owner == source.owner())
        );
    }
    for (i, reference) in footnotes.references().iter().enumerate() {
        assert_eq!(
            reference.definition_index(),
            if i < 12 {
                i
            } else if i == 12 {
                11
            } else {
                1
            }
        );
        assert_eq!(
            reference.source_definition(),
            if i == 13 { Some(0) } else { None }
        );
        let mut text = String::new();
        for paragraph in lines.paragraphs() {
            for line in paragraph.lines() {
                for item in line.items() {
                    if let ProductionPlacedInline::Text(t) = item {
                        if t.run().owner() == reference.owner() {
                            assert!(matches!(t.source_span(), ShapeSourceSpan::Generated(_)));
                            text.push_str(t.utf8());
                        }
                    }
                }
            }
        }
        assert_eq!(text, (reference.definition_index() + 1).to_string());
        for position in [reference.first(), reference.last()] {
            let ProductionPlacedInline::Text(t) = &lines.paragraphs()[position.paragraph_index()]
                .lines()[position.line_index()]
            .items()[position.item_index()] else {
                panic!("marker")
            };
            assert_eq!(t.run().owner(), reference.owner());
        }
    }
    assert_ne!(
        footnotes.references()[11].first(),
        footnotes.references()[12].first()
    );
    let other = layout_book_v2_body_inline_lines(&prepared, body(), 1_000_000).unwrap();
    assert!(footnotes.verify(&other, &limits).is_err());
    let mut base = limits.base().get().clone();
    base.max_fragments -= 1;
    let wrong = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(base).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    assert!(footnotes.verify(&lines, &wrong).is_err());
    assert!(matches!(
        prepare_book_v2_footnote_lines(&lines, &wrong)
            .err()
            .unwrap()
            .kind,
        E::ReceiptMismatch
    ));
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body(),
        1_000_000,
        |stable| {
            stable.footnotes().verify(stable.lines(), &limits).unwrap();
            let body_flow = typaxis_pagination::book_v2::prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0).unwrap();
            assert_eq!(body_flow.references().len(), 14);
            for (index, definition) in stable.footnotes().definitions().iter().enumerate() {
                let items = body_flow.definition_items(index).unwrap();
                assert!(!items.is_empty());
                let marker = body_flow.definition_marker(index).unwrap();
                assert_eq!(marker.owner(), definition.owner());
                assert_eq!(marker.definition_index(), index);
                assert!(marker.item_index() < items.len());
                assert!(items.iter().all(|i| !body_flow.body_items().iter().any(|b| b.owner() == i.owner())));
            }
            for (actual, source) in body_flow.references().iter().zip(stable.footnotes().references()) {
                assert!(std::ptr::eq(actual.source(), source));
                let items = source.source_definition().map_or(body_flow.body_items(), |i| body_flow.definition_items(i).unwrap());
                assert!(actual.first_item_index() <= actual.last_item_index());
                assert!(actual.last_item_index() < items.len());
                assert!(matches!(items[actual.first_item_index()].source(), Some(typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine {paragraph_index, line_index}) if paragraph_index as usize == source.first().paragraph_index() && line_index as usize == source.first().line_index()));
            }
            use typaxis_pagination::book_v2::{prepare_book_v2_footnote_search, prepare_book_v2_body_flow};
            use typaxis_pagination::ProductionBodyPaginationErrorKind as BreakError;
            let run = |prior, maximum| -> Result<(u64,u64),typaxis_pagination::ProductionBodyPaginationError> {
                let mut search = prepare_book_v2_footnote_search(&body_flow,&limits,maximum,prior)?;
                for (index, definition) in stable.footnotes().definitions().iter().enumerate() {
                    let cursor = search.begin(index)?;
                    assert_eq!(cursor.definition_index(),index);
                    assert_eq!(cursor.next_item(),0);
                    assert!(search.evaluate(&cursor,Length::ZERO)?.is_none());
                    let selected = search.evaluate(&cursor,Length::from_raw(2_000_000).unwrap())?.unwrap();
                    selected.verify(&body_flow)?;
                    assert_eq!(selected.marker().unwrap().owner(),definition.owner());
                    assert_eq!(selected.consumed_range().unwrap(),0..body_flow.definition_items(index).unwrap().len());
                    assert!(selected.continuation().is_none());
                    assert_eq!(selected.references().count(),usize::from(index == 0));
                    assert!(selected.items().unwrap().iter().all(|item| body_flow.definition_items(index).unwrap().iter().any(|actual| std::ptr::eq(item,actual))));
                }
                Ok((search.record_charge(),search.visited_items()))
            };
            let (records,work) = run(0,1_000_000).unwrap();
            let exact_prior = limits.base().get().max_fragments - (records-body_flow.record_charge());
            assert_eq!(run(exact_prior,work).unwrap(),(limits.base().get().max_fragments,work));
            assert!(matches!(run(exact_prior+1,work),Err(e) if e.kind == BreakError::FragmentLimit));
            assert!(matches!(run(0,work-1),Err(e) if e.kind == BreakError::FootnoteSearchLimit));
            let other_flow = prepare_book_v2_body_flow(stable.lines(),None,stable.footnotes(),&limits,0).unwrap();
            let mut search = prepare_book_v2_footnote_search(&body_flow,&limits,1_000_000,0).unwrap();
            let mut other_search = prepare_book_v2_footnote_search(&other_flow,&limits,1_000_000,0).unwrap();
            assert!(matches!(search.evaluate(&other_search.begin(0).unwrap(),Length::from_raw(2_000_000).unwrap()),Err(e) if e.kind == BreakError::ReceiptMismatch));
            let other_footnotes = prepare_book_v2_footnote_lines(stable.lines(), &limits).unwrap();
            assert!(body_flow.verify(stable.lines(), None, &other_footnotes, &limits).is_err());

            assert_eq!(stable.footnotes().definitions(), footnotes.definitions());
            assert_eq!(stable.footnotes().references(), footnotes.references());
        },
    )
    .unwrap();
}

fn registry_budget(maximum: u64) -> Result<u64, typaxis_layout::ProductionInlinePreparationError> {
    let root = Root::new();
    let limits = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(ResourceLimits {
            max_fragments: maximum,
            ..ResourceLimits::default()
        })
        .unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap();
    let input = prepared(&root, input_data(), b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let shaped =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
            .unwrap();
    let prepared = prepare_book_v2_text_inlines(
        &flow,
        &shaped,
        input.resources(),
        &limits,
        EPOCH,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    let lines = layout_book_v2_body_inline_lines(&prepared, body(), 1_000_000).unwrap();
    prepare_book_v2_footnote_lines(&lines, &limits).map(|f| f.record_charge())
}
#[test]
fn book_v2_footnotes_charge_selected_lines_and_definition_reference_indexes_together() {
    let required = registry_budget(ResourceLimits::default().max_fragments).unwrap();
    assert_eq!(registry_budget(required).unwrap(), required);
    let err = registry_budget(required - 1).unwrap_err();
    assert_eq!(err.owner, NodeId::new(0));
    assert!(matches!(err.kind, E::UnitLimit));
}

#[test]
fn book_v2_footnote_fragments_preserve_forced_boundaries_continuations_and_single_labels() {
    for mode in ["continue", "keep", "oversize"] {
        let root = Root::new();
        let limits = limits();
        let mut data = input_data();
        let span = data["document"]["footnotes"][0]["span"].clone();
        let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
        let forced = json!({"kind":"page_break","node_id":0,"classes":[],"span":span});
        data["document"]["footnotes"][0]["blocks"] =
            json!([forced, paragraph, paragraph, forced, paragraph]);
        if mode == "keep" {
            data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"keep-paragraph","selector":"paragraph","source_order":3,"extends":null,"declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
        } else if mode == "oversize" {
            data["page_masters"]["masters"][0]["footnote"]["height"] = 500_000.into();
        }
        renumber(&mut data["document"], &mut 0);
        let input = prepared(&root, data, b"Result", &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        with_converged_book_v2_body_lines(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,body(),1_000_000,|stable| {
            use typaxis_pagination::book_v2::{prepare_book_v2_body_flow,prepare_book_v2_footnote_search};
            use typaxis_pagination::{ProductionBodyPaginationErrorKind as BreakError,ProductionBodyBreakReason};
            let measured = prepare_book_v2_body_flow(stable.lines(),None,stable.footnotes(),&limits,0).unwrap();
            let search = prepare_book_v2_footnote_search(&measured,&limits,1_000_000,0);
            if mode == "keep" {
                assert!(matches!(search,Err(e) if e.kind == BreakError::KeepAcrossForcedBreak));
                return;
            }
            let mut search = search.unwrap();
            let begin = search.begin(0).unwrap();
            assert!(matches!(search.evaluate(&begin,Length::from_raw(-1).unwrap()),Err(e) if e.kind == BreakError::InvalidFootnoteCapacity));
            let leading = search.evaluate(&begin,Length::ZERO).unwrap().unwrap();
            assert_eq!(leading.reason(),ProductionBodyBreakReason::Forced);
            assert_eq!(leading.consumed_range().unwrap(),0..1);
            assert!(leading.items().unwrap().is_empty());
            assert!(leading.marker().is_none());
            assert_eq!(leading.references().count(),0);
            let mut cursor = leading.continuation();
            if mode == "oversize" {
                assert!(matches!(search.evaluate(&cursor.unwrap(),Length::from_raw(500_000).unwrap()),Err(e) if e.kind == BreakError::Oversize));
                return;
            }
            let mut markers = 0;
            let mut references = 0;
            let mut forced_count = 1;
            let mut consumed = 1;
            let mut fragments = 0;
            while let Some(current) = cursor {
                assert!(fragments < 8);
                let fragment = search.evaluate(&current,Length::from_raw(2_000_000).unwrap()).unwrap().unwrap();
                assert_eq!(fragment.consumed_range().unwrap().start,consumed);
                assert!(fragment.consumed_range().unwrap().end > consumed);
                consumed = fragment.consumed_range().unwrap().end;
                markers += usize::from(fragment.marker().is_some());
                references += fragment.references().count();
                forced_count += usize::from(fragment.forced_break_owner().is_some());
                cursor = fragment.continuation();
                fragments += 1;
            }
            assert_eq!(markers,1);
            assert_eq!(references,3);
            assert_eq!(forced_count,2);
            assert_eq!(consumed,measured.definition_items(0).unwrap().len());
            assert!(fragments >= 3);
            let mut demand = typaxis_pagination::book_v2::prepare_book_v2_footnote_demand_search(&measured,&limits,1_000_000,0).unwrap();
            let base = demand.begin().unwrap();
            let demanded = demand.require_body(&base,0..measured.body_items().len()).unwrap();
            assert!(demand.select_required_region(&demanded,Length::from_raw(2_000_000).unwrap()).unwrap().is_none());
            let first_region = demand.select_region(&demanded,Length::ZERO).unwrap().unwrap();
            assert_eq!(first_region.fragments().len(),1);
            assert_eq!(first_region.used_height(),Length::ZERO);
            assert!(first_region.forced_break_owner().is_some());
            assert!(first_region.fragments()[0].fragment().items().unwrap().is_empty());
            assert!(first_region.fragments()[0].fragment().marker().is_none());
            let mut current = first_region;
            let mut regions = 1;
            let mut region_markers = 0;
            let mut region_forced = 1;
            while current.next_state().status(0) != Some(typaxis_pagination::ProductionFootnoteDemandStatus::Complete) {
                assert!(regions < 8);
                current = demand.select_region(current.next_state(),Length::from_raw(2_000_000).unwrap()).unwrap().unwrap();
                region_markers += current.fragments().iter().filter(|f| f.fragment().definition_index()==0 && f.fragment().marker().is_some()).count();
                region_forced += usize::from(current.forced_break_owner().is_some());
                regions += 1;
            }
            assert_eq!(region_markers,1);
            assert_eq!(region_forced,2);
            assert_eq!(demanded.status(0),Some(typaxis_pagination::ProductionFootnoteDemandStatus::Pending));

        }).unwrap();
    }
}

#[path = "book_v2_demand_tests.rs"]
mod demand;
