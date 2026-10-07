use super::*;
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_footnote_demand_search,
};
use typaxis_pagination::{
    ProductionBodyPaginationError, ProductionBodyPaginationErrorKind as Error,
    ProductionFootnoteDemandStatus as Status,
};

#[test]
fn book_v2_demand_keeps_immutable_branches_first_references_and_nested_continuations() {
    let root = Root::new();
    let limits = limits();
    let mut data = input_data();
    let original = data["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array()
        .unwrap()
        .clone();
    data["document"]["blocks"][0]["blocks"][0]["children"] =
        json!([original[0], original[1], original[1]]);
    let nested = data["document"]["footnotes"][0]["blocks"][0].clone();
    data["document"]["footnotes"][0]["blocks"] = json!([nested, nested]);
    let master = &mut data["page_masters"]["masters"][0];
    master["height"] = 28_000_000.into();
    master["trim"]["height"] = 28_000_000.into();
    master["footnote"]["height"] = 6_000_000.into();
    data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"region-spacing","selector":"paragraph","source_order":3,"extends":null,"declarations":[
        {"name":"space_before","important":false,"value":{"kind":"length","value":65_536}},
        {"name":"space_after","important":false,"value":{"kind":"length","value":65_536}}
    ]}));
    renumber(&mut data["document"], &mut 0);
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    with_converged_book_v2_body_lines(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,body(),1_000_000,|stable| {
        let measured = prepare_book_v2_body_flow(stable.lines(),None,stable.footnotes(),&limits,0).unwrap();
        let first_body_reference = measured.references().iter().find(|r| r.source().source_definition().is_none()).unwrap().source().owner();
        let first_nested_reference = measured.references().iter().find(|r| r.source().source_definition()==Some(0)).unwrap().source().owner();
        let run = |prior,work| -> Result<(u64,u64),ProductionBodyPaginationError> {
            let mut search = prepare_book_v2_footnote_demand_search(&measured,&limits,work,prior)?;
            let base = search.begin()?;
            assert!(base.pending_definitions().is_empty());
            assert!(search.evaluate_next(&base,Length::from_raw(2_000_000).unwrap())?.is_none());
            let demanded = search.require_body(&base,0..measured.body_items().len())?;
            assert_eq!(base.status(0),Some(Status::Unreferenced));
            assert_eq!(demanded.pending_definitions(),[0]);
            assert_eq!(demanded.first_reference(0),Some(first_body_reference));
            assert_eq!(demanded.status(1),Some(Status::Unreferenced));
            let sibling = search.require_body(&base,0..measured.body_items().len())?;
            assert!(search.evaluate_next(&demanded,Length::ZERO)?.is_none());
            let first = search.evaluate_next(&demanded,Length::from_raw(2_000_000).unwrap())?.unwrap();
            assert!(first.fragment().marker().is_some());
            assert_eq!(first.fragment().items().unwrap().len(),1);
            assert!(matches!(search.advance(&sibling,&first),Err(e) if e.kind==Error::ReceiptMismatch));
            let continued = search.advance(&demanded,&first)?;
            assert_eq!(continued.pending_definitions(),[0,1]);
            assert_eq!(continued.first_reference(1),Some(first_nested_reference));
            assert_eq!(demanded.pending_definitions(),[0]);
            assert_eq!(demanded.status(1),Some(Status::Unreferenced));
            let last = search.evaluate_next(&continued,Length::from_raw(2_000_000).unwrap())?.unwrap();
            assert!(last.fragment().marker().is_none());
            let completed = search.advance(&continued,&last)?;
            assert_eq!(completed.pending_definitions(),[1]);
            assert_eq!(completed.status(0),Some(Status::Complete));
            assert_eq!(completed.first_reference(0),Some(first_body_reference));
            assert_eq!(completed.first_reference(1),Some(first_nested_reference));
            assert!(matches!(search.advance(&completed,&first),Err(e) if e.kind==Error::ReceiptMismatch));
            let child = search.evaluate_next(&completed,Length::from_raw(2_000_000).unwrap())?.unwrap();
            assert_eq!(child.fragment().definition_index(),1);
            let end = search.advance(&completed,&child)?;
            assert!(end.pending_definitions().is_empty());
            assert_eq!(end.status(1),Some(Status::Complete));
            for index in 2..12 { assert_eq!(end.status(index),Some(Status::Unreferenced)); }
            let repeated = search.require_body(&end,0..measured.body_items().len())?;
            assert!(repeated.pending_definitions().is_empty());
            assert!(search.evaluate_next(&repeated,Length::ZERO)?.is_none());
            assert!(matches!(search.require_body(&base,0..measured.body_items().len()+1),Err(e) if e.kind==Error::ReceiptMismatch));
            // A fresh branch fits both paragraphs of definition 0 and its nested
            // demand in one region. Inter-definition spacing is charged once.
            let capacity = Length::from_raw(6_000_000).unwrap();
            assert!(search.select_region(&base,capacity)?.is_none());
            assert!(search.select_region(&demanded,Length::ZERO)?.is_none());
            assert!(matches!(search.select_region(&base,Length::from_raw(-1).unwrap()),Err(e) if e.kind==Error::InvalidFootnoteCapacity));
            assert!(matches!(search.select_region(&demanded,Length::from_raw(6_000_001).unwrap()),Err(e) if e.kind==Error::InvalidFootnoteCapacity));
            let entry_only = search.select_required_region(&demanded,capacity)?.unwrap();
            assert_eq!(entry_only.fragments().len(),1);
            assert_eq!(entry_only.fragments()[0].fragment().definition_index(),0);
            assert_eq!(entry_only.next_state().pending_definitions(),[1]);
            assert_eq!(entry_only.next_state().first_reference(1),Some(first_nested_reference));
            let region = search.select_region(&demanded,capacity)?.unwrap();
            region.verify(&demanded)?;
            assert!(matches!(region.verify(&sibling),Err(e) if e.kind==Error::ReceiptMismatch));
            assert_eq!(region.fragments().len(),2);
            assert_eq!(region.fragments()[0].fragment().definition_index(),0);
            assert_eq!(region.fragments()[0].fragment().items().unwrap().len(),2);
            assert_eq!(region.fragments()[1].fragment().definition_index(),1);
            assert!(region.fragments().iter().all(|f| f.fragment().marker().is_some()));
            let first = region.fragments()[0].fragment();
            let second = region.fragments()[1].fragment();
            let gap = first.items().unwrap().last().unwrap().space_after().checked_add(second.items().unwrap()[0].space_before()).unwrap();
            assert!(gap > Length::ZERO);
            let second_offset = first.used_height().checked_add(gap).unwrap();
            assert_eq!(region.fragments()[0].offset(),Length::ZERO);
            assert_eq!(region.fragments()[1].offset(),second_offset);
            let exact = second_offset.checked_add(second.used_height()).unwrap();
            assert_eq!(region.used_height(),exact);
            assert_eq!(region.available_height(),capacity);
            assert!(region.forced_break_owner().is_none());
            assert!(region.next_state().pending_definitions().is_empty());
            assert_eq!(region.next_state().first_reference(1),Some(first_nested_reference));
            assert_eq!(demanded.pending_definitions(),[0]);
            assert_eq!(search.select_region(&demanded,exact)?.unwrap().fragments().len(),2);
            let short = search.select_region(&demanded,exact.checked_sub(Length::from_raw(1).unwrap()).unwrap())?.unwrap();
            assert_eq!(short.fragments().len(),1);
            assert_eq!(short.next_state().pending_definitions(),[1]);
            let child_region = search.select_region(short.next_state(),capacity)?.unwrap();
            assert_eq!(child_region.fragments().len(),1);
            assert_eq!(child_region.fragments()[0].offset(),Length::ZERO);
            assert!(child_region.next_state().pending_definitions().is_empty());
            Ok((search.record_charge(),search.work_steps()))
        };
        let (records,work) = run(0,1_000_000).unwrap();
        let exact_prior = limits.base().get().max_fragments-(records-measured.record_charge());
        assert_eq!(run(exact_prior,work).unwrap(),(limits.base().get().max_fragments,work));
        assert!(matches!(run(exact_prior+1,work),Err(e) if e.kind==Error::FragmentLimit));
        assert!(matches!(run(0,work-1),Err(e) if e.kind==Error::FootnoteSearchLimit));
        let mut first_search = prepare_book_v2_footnote_demand_search(&measured,&limits,1_000_000,0).unwrap();
        let mut other_search = prepare_book_v2_footnote_demand_search(&measured,&limits,1_000_000,0).unwrap();
        let foreign = other_search.begin().unwrap();
        assert!(matches!(first_search.require_body(&foreign,0..measured.body_items().len()),Err(e) if e.kind==Error::ReceiptMismatch));
    }).unwrap();
}

#[test]
fn book_v2_required_region_reserves_each_entry_demand_before_extra_content() {
    let root = Root::new();
    let limits = limits();
    let mut data = input_data();
    data["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .truncate(3);
    let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
    data["document"]["footnotes"][0]["blocks"] = json!([paragraph, paragraph, paragraph]);
    let master = &mut data["page_masters"]["masters"][0];
    master["height"] = 28_000_000.into();
    master["trim"]["height"] = 28_000_000.into();
    master["footnote"]["height"] = 6_000_000.into();
    data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"required-spacing","selector":"paragraph","source_order":3,"extends":null,"declarations":[
        {"name":"space_before","important":false,"value":{"kind":"length","value":65_536}},
        {"name":"space_after","important":false,"value":{"kind":"length","value":65_536}}
    ]}));
    renumber(&mut data["document"], &mut 0);
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    with_converged_book_v2_body_lines(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,body(),1_000_000,|stable| {
        let measured = prepare_book_v2_body_flow(stable.lines(),None,stable.footnotes(),&limits,0).unwrap();
        let first = &measured.definition_items(0).unwrap()[0];
        let second = &measured.definition_items(1).unwrap()[0];
        let gap = first.space_after().checked_add(second.space_before()).unwrap();
        assert!(gap > Length::ZERO);
        let second_offset = first.consumed_height().unwrap().checked_add(gap).unwrap();
        let minimum = second_offset.checked_add(second.consumed_height().unwrap()).unwrap();
        let run = |prior,work| -> Result<(u64,u64),ProductionBodyPaginationError> {
            let mut search = prepare_book_v2_footnote_demand_search(&measured,&limits,work,prior)?;
            let base = search.begin()?;
            assert!(search.select_required_region(&base,minimum)?.is_none());
            let demanded = search.require_body(&base,0..measured.body_items().len())?;
            assert_eq!(demanded.pending_definitions(),[0,1]);
            assert!(search.select_required_region(&demanded,Length::ZERO)?.is_none());
            assert!(search.select_required_region(&demanded,minimum.checked_sub(Length::from_raw(1).unwrap()).unwrap())?.is_none());
            let ordered = search.select_region(&demanded,minimum)?.unwrap();
            assert_eq!(ordered.fragments().len(),1);
            let required = search.select_required_region(&demanded,minimum)?.unwrap();
            required.verify(&demanded)?;
            assert!(matches!(required.verify(&base),Err(e) if e.kind==Error::ReceiptMismatch));
            assert_eq!(required.fragments().len(),2);
            assert_eq!(required.fragments()[0].offset(),Length::ZERO);
            assert_eq!(required.fragments()[1].offset(),second_offset);
            assert_eq!(required.used_height(),minimum);
            assert_eq!(required.available_height(),minimum);
            assert!(required.forced_break_owner().is_none());
            for (i,part) in required.fragments().iter().enumerate() {
                assert_eq!(part.fragment().definition_index(),i);
                assert_eq!(part.fragment().items().unwrap().len(),1);
                assert!(part.fragment().marker().is_some());
                assert_eq!(part.fragment().consumed_range().unwrap(),0..1);
                let selected = part.fragment().candidates()[part.fragment().selected_candidate_index().unwrap() as usize];
                assert_eq!(selected.used_height(),part.fragment().used_height());
            }
            assert!(required.fragments()[0].fragment().continuation().is_some());
            assert!(required.fragments()[1].fragment().continuation().is_none());
            assert_eq!(required.next_state().pending_definitions(),[0]);
            assert_eq!(required.next_state().status(1),Some(Status::Complete));
            assert_eq!(required.next_state().first_reference(1),demanded.first_reference(1));
            assert_eq!(demanded.pending_definitions(),[0,1]);
            let end = search.select_required_region(required.next_state(),Length::from_raw(6_000_000).unwrap())?.unwrap();
            assert_eq!(end.fragments().len(),1);
            assert_eq!(end.fragments()[0].fragment().items().unwrap().len(),2);
            assert!(end.fragments()[0].fragment().marker().is_none());
            assert!(end.next_state().pending_definitions().is_empty());
            assert!(matches!(search.select_required_region(&demanded,Length::from_raw(-1).unwrap()),Err(e) if e.kind==Error::InvalidFootnoteCapacity));
            assert!(matches!(search.select_required_region(&demanded,Length::from_raw(6_000_001).unwrap()),Err(e) if e.kind==Error::InvalidFootnoteCapacity));
            Ok((search.record_charge(),search.work_steps()))
        };
        let (records,work) = run(0,1_000_000).unwrap();
        let exact_prior = limits.base().get().max_fragments-(records-measured.record_charge());
        assert_eq!(run(exact_prior,work).unwrap(),(limits.base().get().max_fragments,work));
        assert!(matches!(run(exact_prior+1,work),Err(e) if e.kind==Error::FragmentLimit));
        assert!(matches!(run(0,work-1),Err(e) if e.kind==Error::FootnoteSearchLimit));
    }).unwrap();
}

#[test]
fn book_v2_required_region_stops_at_the_last_demand_forced_boundary() {
    let root = Root::new();
    let limits = limits();
    let mut data = input_data();
    data["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .truncate(3);
    let span = data["document"]["footnotes"][1]["span"].clone();
    let paragraph = data["document"]["footnotes"][1]["blocks"][0].clone();
    data["document"]["footnotes"][1]["blocks"] = json!([
        {"kind":"page_break","node_id":0,"classes":[],"span":span},paragraph
    ]);
    renumber(&mut data["document"], &mut 0);
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
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
            let measured =
                prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0)
                    .unwrap();
            let mut search =
                prepare_book_v2_footnote_demand_search(&measured, &limits, 1_000_000, 0).unwrap();
            let base = search.begin().unwrap();
            let demanded = search
                .require_body(&base, 0..measured.body_items().len())
                .unwrap();
            assert_eq!(demanded.pending_definitions(), [0, 1]);
            let region = search
                .select_required_region(&demanded, Length::from_raw(2_000_000).unwrap())
                .unwrap()
                .unwrap();
            region.verify(&demanded).unwrap();
            assert_eq!(region.fragments().len(), 2);
            let content = region.fragments()[0].fragment();
            let forced = region.fragments()[1].fragment();
            assert_eq!(content.definition_index(), 0);
            assert!(content.marker().is_some());
            assert_eq!(forced.definition_index(), 1);
            assert!(forced.items().unwrap().is_empty());
            assert!(forced.marker().is_none());
            assert_eq!(forced.consumed_range().unwrap(), 0..1);
            assert_eq!(forced.used_height(), Length::ZERO);
            assert_eq!(
                region.forced_break_owner(),
                Some(measured.definition_items(1).unwrap()[0].owner())
            );
            assert_eq!(region.fragments()[1].offset(), content.used_height());
            assert_eq!(region.used_height(), content.used_height());
            assert_eq!(region.next_state().pending_definitions(), [1]);
            let continued = search
                .select_required_region(region.next_state(), Length::from_raw(2_000_000).unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(continued.fragments().len(), 1);
            assert_eq!(continued.fragments()[0].fragment().definition_index(), 1);
            assert!(continued.fragments()[0].fragment().marker().is_some());
            assert!(continued.forced_break_owner().is_none());
            assert!(continued.next_state().pending_definitions().is_empty());
            assert_eq!(demanded.pending_definitions(), [0, 1]);
        },
    )
    .unwrap();
}

#[path = "book_v2_joint_fit_tests.rs"]
mod joint_fit;
