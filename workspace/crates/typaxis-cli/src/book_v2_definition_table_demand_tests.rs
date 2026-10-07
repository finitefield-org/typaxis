use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

#[test]
fn book_v2_definition_table_demands_follow_only_retained_source() {
    use typaxis_core::{Length, PositiveLength, Rect};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_definition_table_demand_search,
        prepare_book_v2_table_body_search, prepare_book_v2_table_measurements,
    };
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for mode in [
        "early",
        "late",
        "parallel",
        "repeat",
        "self",
        "prefix",
        "suffix",
        "empty-tail",
    ] {
        let root = Root::new();
        let limits = limits();
        let mut data = if mode == "repeat" {
            super::nested_spans::spanning("LeftRight", 4, "header-caption")
        } else {
            super::nested_tables::nested("LeftRight", 4, "forced")
        };
        let mut table = data["document"]["blocks"][0].clone();
        let paragraph =
            if mode == "repeat" {
                &mut table["head"][0]["cells"][0]["blocks"][0]["caption"][0]
            } else if mode == "parallel" {
                &mut table["body"][0]["cells"][1]["blocks"][0]
            } else {
                &mut table["body"][0]["cells"][0]["blocks"][0]["body"][0]["cells"][0]["blocks"]
                    [if mode == "late" || mode == "self" {
                        2
                    } else {
                        0
                    }]
            };
        let span = paragraph["span"].clone();
        paragraph["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":if mode=="self"{"table"}else{"before"}}));
        let paragraph = super::nested_tables::nested("LeftRight", 4, "forced")["document"]
            ["blocks"][0]["body"][0]["cells"][1]["blocks"][0]
            .clone();
        let mut reference = paragraph.clone();
        reference["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":paragraph["span"],"footnote_id":"table"}));
        data["document"]["blocks"] = json!([reference, paragraph.clone()]);
        data["document"]["footnotes"] = json!([
            {"node_id":0,"span":table["span"],"footnote_id":"before","blocks":[paragraph.clone(),paragraph.clone(),paragraph.clone()]},
            {"node_id":0,"span":table["span"],"footnote_id":"table","blocks":[table]}
        ]);
        if mode == "prefix" || mode == "suffix" {
            let blocks = data["document"]["footnotes"][1]["blocks"]
                .as_array_mut()
                .unwrap();
            blocks.insert(if mode == "prefix" { 0 } else { 1 }, paragraph.clone());
        }
        if mode == "empty-tail" {
            let mut empty = data["document"]["footnotes"][1]["blocks"][0].clone();
            empty["head"] = json!([]);
            empty.as_object_mut().unwrap().remove("caption");
            empty["body"].as_array_mut().unwrap().truncate(1);
            for cell in empty["body"][0]["cells"].as_array_mut().unwrap() {
                cell["blocks"] = json!([]);
                cell["rowspan"] = 1.into();
            }
            data["document"]["footnotes"][1]["blocks"]
                .as_array_mut()
                .unwrap()
                .push(empty);
        }
        let frame_width = data["page_masters"]["masters"][0]["body"]["width"]
            .as_i64()
            .unwrap();
        data["page_masters"]["masters"][0]["width"] = (frame_width + 20 * 65536).into();
        data["page_masters"]["masters"][0]["height"] = (320 * 65536).into();
        data["page_masters"]["masters"][0]["trim"]["height"] = (320 * 65536).into();
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":10*65536,"y":150*65536,"width":frame_width,"height":128*65536});
        super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
        let frame_height = data["page_masters"]["masters"][0]["body"]["height"]
            .as_i64()
            .unwrap();
        let input = prepared(&root, data, b"LeftRight", &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings =
            typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits)
                .unwrap();
        let raw = |v| Length::from_raw(v).unwrap();
        let rect = Rect::new(
            raw(10 * 65536),
            raw(10 * 65536),
            PositiveLength::new(raw(frame_width)).unwrap(),
            PositiveLength::new(raw(frame_height)).unwrap(),
        );
        typaxis_layout::book_v2::with_converged_book_v2_body_lines(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            rect,
            1_000_000,
            |stable| {
                let measured = prepare_book_v2_table_measurements(
                    prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0)
                        .unwrap(),
                    &limits,
                )
                .unwrap();
                use typaxis_pagination::ProductionFootnoteDemandStatus as Status;
                if mode=="prefix" {
                    let mut search=prepare_book_v2_definition_table_demand_search(&measured,0,&limits,1_000_000,0).unwrap();
                    let before=search.begin(0..measured.flow().body_items().len()).unwrap();
                    assert!(matches!(search.evaluate(&before,search.maximum_height()),Err(e) if e.kind==E::ReceiptMismatch));
                    assert_eq!(before.demand().status(1),Some(Status::Pending));
                    assert_eq!(before.demand().status(0),Some(Status::Unreferenced));
                    return;
                }
                let replay = |prior, work| -> Result<_,typaxis_pagination::ProductionBodyPaginationError> {
                    let mut search=prepare_book_v2_definition_table_demand_search(&measured,0,&limits,work,prior)?;
                    let mut state=search.begin(0..measured.flow().body_items().len())?;
                    let mut hashes=Vec::new();let mut original=Vec::new();
                    while !state.table_complete(){
                        assert!(hashes.len()<10,"{mode}");
                        let before=state.demand().status(0);
                        let selected=search.evaluate(&state,if mode=="repeat"{raw(64*65536)}else{search.maximum_height()})?.unwrap();
                        selected.verify(&state)?;
                        assert_eq!(state.demand().status(0),before);
                        let expected=if mode=="self" || mode=="late"&&hashes.is_empty(){Status::Unreferenced}else{Status::Pending};
                        assert_eq!(selected.next_state().demand().status(0),Some(expected),"{mode}/{}",hashes.len());
                        assert_eq!(selected.next_state().demand().status(1),Some(if selected.next_state().table_complete()&&mode!="suffix"&&mode!="empty-tail"{Status::Complete}else{Status::Pending}));
                        original.extend(selected.table().source_leaf_ranges().collect::<Result<Vec<_>,_>>()?.into_iter().flatten());
                        let marker=measured.flow().definition_marker(1).unwrap().item_index();
                        assert_eq!(selected.next_state().demand().definition_started(1),original.contains(&marker),"table marker {mode}");
                        if !selected.next_state().table_complete() {
                            let cursor=selected.next_state().demand().definition_cursor(1).unwrap();
                            assert_eq!(cursor.table_continuation().unwrap().cell_progress_fingerprint(),selected.table().after().cell_progress_fingerprint());
                        }
                        hashes.push(selected.table().fingerprint());
                        state=selected.into_next_state();
                    }
                    original.sort_unstable();
                    assert_eq!(original,(0..measured.flow().definition_items(1).unwrap().len()-usize::from(mode=="suffix")).collect::<Vec<_>>());
                    assert_eq!(state.demand().pending_definitions(),if mode=="self"{&[][..]}else if mode=="suffix"||mode=="empty-tail"{&[1,0][..]}else{&[0][..]});
                    Ok((search.record_charge(),search.work_charge(),hashes))
                };
                let (records, work, hashes) = replay(0, 1_000_000).unwrap();
                let prior =
                    limits.base().get().max_fragments - (records - measured.record_charge());
                assert_eq!(
                    replay(prior, work).unwrap(),
                    (limits.base().get().max_fragments, work, hashes)
                );
                assert_eq!(replay(prior + 1, work).unwrap_err().kind, E::FragmentLimit);
                assert!(matches!(replay(0, work - 1).unwrap_err().kind,E::TableSearchLimit|E::FootnoteSearchLimit));
                if mode=="early" {
                    let mut search=prepare_book_v2_definition_table_demand_search(&measured,0,&limits,1_000_000,0).unwrap();
                    assert!(matches!(search.begin(1..measured.flow().body_items().len()),Err(e) if e.kind==E::ReceiptMismatch));
                    let before=search.begin(0..measured.flow().body_items().len()).unwrap();
                    let work=search.work_charge();
                    assert!(search.evaluate(&before,Length::ZERO).unwrap().is_none());
                    assert!(search.work_charge()>work);
                    assert_eq!(before.demand().status(0),Some(Status::Unreferenced));
                    let selected=search.evaluate(&before,search.maximum_height()).unwrap().unwrap();
                    let branch=search.begin(0..measured.flow().body_items().len()).unwrap();
                    assert_eq!(selected.verify(&branch).unwrap_err().kind,E::ReceiptMismatch);
                    assert_eq!(selected.verify(selected.next_state()).unwrap_err().kind,E::ReceiptMismatch);
                    let mut other=prepare_book_v2_definition_table_demand_search(&measured,0,&limits,1_000_000,0).unwrap();
                    assert!(matches!(other.evaluate(&before,other.maximum_height()),Err(e) if e.kind==E::ReceiptMismatch));
                    assert!(matches!(prepare_book_v2_definition_table_demand_search(&measured,1,&limits,1_000_000,0),Err(e) if e.kind==E::ReceiptMismatch));
                }
                assert_eq!(prepare_book_v2_table_body_search(&measured,&limits,1_000_000,0).unwrap().body_table_count(), (0..measured.flow().table_count()).filter(|&index|measured.flow().table_source_definition(index)==Some(None)).count());
            },
        )
        .unwrap();
    }
}
