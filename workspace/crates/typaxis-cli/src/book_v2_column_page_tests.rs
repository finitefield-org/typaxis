use super::*;
#[path = "book_v2_column_auto_tests.rs"]
mod automatic;
use typaxis_pagination::book_v2::*;
use typaxis_pagination::{
    ProductionBodyPaginationErrorKind as E, ProductionFootnoteDemandStatus as S,
};

fn page_data(
    text: &str,
    note_height: i64,
    note_x: i64,
    note_y: i64,
    note_width: i64,
    long_body: bool,
    long_notes: bool,
) -> Value {
    let mut data = super::super::table_width_frames::table_data(text, false);
    let mut short = data["document"]["blocks"][1].clone();
    if long_body {
        short["children"][0]["text_span"]["end_byte"] = 6.into();
        short["children"][0]["span"]["end_byte"] = 6.into();
    }
    let mut first = short.clone();
    let mut second = if long_body {
        data["document"]["blocks"][1].clone()
    } else {
        short.clone()
    };
    for (paragraph, id) in [(&mut first, "a"), (&mut second, "b")] {
        let span = paragraph["span"].clone();
        paragraph["children"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"footnote_reference","node_id":0,
            "span":span,"footnote_id":id}));
    }
    data["document"]["blocks"] = json!([first, second]);
    data["document"]["footnotes"] = json!([
        {"node_id":0,"span":short["span"],"footnote_id":"a","blocks":if long_notes {vec![short.clone();8]} else {vec![short.clone()]}},
        {"node_id":0,"span":short["span"],"footnote_id":"b","blocks":if long_notes {vec![short.clone();8]} else {vec![short.clone()]}}
    ]);
    let master = &mut data["page_masters"]["masters"][0];
    master["body"] = json!({"x":10*65536,"y":10*65536,"width":210*65536,"height":240*65536});
    master["footnote"] = json!({"x":note_x*65536,"y":note_y*65536,"width":note_width*65536,"height":note_height*65536});
    super::super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
    data
}

fn check_joint(font: Option<&[u8]>) {
    let text = if font.is_some() { "本文" } else { "Result" };
    let root = Root::new();
    let limits = limits();
    let input = input(
        &root,
        page_data(text, 100, 10, 300, 210, false, false),
        text,
        font,
        &limits,
    );
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let mut budget =
        BookV2BodyLineBudget::new(1_000_000, limits.base().get().max_line_reshape_passes);
    with_budgeted_book_v2_column_lines(&policy,&flow,input.resources(),&bindings,&limits,MODE,None,&mut budget,&plan,None,|stable| {
        let mut records=0;
        let columns=prepare_book_v2_column_flow_counted(&stable,None,&limits,stable.retained_record_charge(),&mut records).unwrap();
        assert_eq!(columns.body_items().len(),2);
        assert_eq!(columns.definition_items(0).unwrap().len(),1);
        assert_eq!(columns.record_charge(),records);
        assert!(prepare_book_v2_body_flow(stable.lines(),None,stable.footnotes(),&limits,0).is_err());
        let measured=prepare_book_v2_column_table_measurements_counted(columns,&limits,&mut records).unwrap();
        assert_eq!(measured.record_charge(),records);
        let run=|work| -> Result<_,typaxis_pagination::ProductionBodyPaginationError> {
            let mut search=prepare_book_v2_column_page_search_counted(&measured,&limits,work,records,&mut 0,&mut 0).unwrap();
            let state=search.begin().unwrap();
            let a=[BookV2BodyCandidatePart::Items {end:1}];
            let b=[BookV2BodyCandidatePart::Items {end:2}];
            let selected=search.evaluate_page(&state,None,&[&a,&b])?;
            if let Some(selected)=selected {
                selected.verify(&state).unwrap();
                assert_eq!(selected.frames().page_index(),0);
                assert_eq!(selected.columns().len(),2);
                assert_eq!(selected.columns()[0].bounds().x().raw(),10*65536);
                assert_eq!(selected.columns()[1].bounds().x().raw(),120*65536);
                assert_eq!(selected.columns()[0].parts()[0].items(),Some(0..1));
                assert_eq!(selected.columns()[1].parts()[0].items(),Some(1..2));
                let notes=selected.footnotes().unwrap();
                assert_eq!(notes.fragments().len(),2);
                assert_eq!(notes.fragments()[0].fragment().definition_index(),0);
                assert_eq!(notes.fragments()[1].fragment().definition_index(),1);
                assert_eq!(selected.footnote_bounds().unwrap().width().get().raw(),210*65536);
                assert_eq!(selected.next_state().page_index(),1);
                assert_eq!(selected.next_state().definition_status(0),Some(S::Complete));
                assert_eq!(selected.next_state().definition_status(1),Some(S::Complete));
                assert!(selected.next_state().is_complete());
                let mut foreign=prepare_book_v2_column_page_search_counted(&measured,&limits,1_000_000,records,&mut 0,&mut 0).unwrap();
                let foreign_state=foreign.begin().unwrap();
                assert!(selected.verify(&foreign_state).is_err());
                assert!(matches!(foreign.evaluate_page(&state,None,&[&a,&b]),Err(e) if e.kind==E::ReceiptMismatch));
                Ok(Some((search.work_steps(),search.record_charge(),notes.used_height().raw())))
            } else {Ok(None)}
        };
        let expected=run(1_000_000).unwrap().unwrap();
        assert_eq!(run(expected.0).unwrap().unwrap(),expected);
        assert!(run(expected.0-1).is_err());
        let mut search=prepare_book_v2_column_page_search_counted(&measured,&limits,1_000_000,records,&mut 0,&mut 0).unwrap();
        let state=search.begin().unwrap();
        let a=[BookV2BodyCandidatePart::Items {end:1}]; let b=[BookV2BodyCandidatePart::Items {end:2}];
        assert!(search.evaluate_page(&state,None,&[&a]).is_err());
        assert!(search.evaluate_page(&state,None,&[&[],&b]).is_err());
        let before=(search.work_steps(),search.record_charge());
        assert!(search.evaluate_page(&state,None,&[&a,&a]).is_err());
        assert!(search.work_steps()>before.0 && search.record_charge()>before.1);
        assert!(search.evaluate_page(&state,None,&[&a,&b]).unwrap().is_some());
        expected
    }).unwrap();
}

#[test]
fn book_v2_column_page_candidates_join_two_columns_and_one_note_region() {
    check_joint(None);
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_column_page_candidates_preserve_original_harano() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    check_joint(Some(&font));
    check_tables(Some(&font));
}

fn check_reservation(height: i64, x: i64, width: i64, long: bool, carry: bool, expected: bool) {
    let text = if long {
        "Result Result Result Result Result Result Result Result Result Result Result Result"
    } else {
        "Result"
    };
    let root = Root::new();
    let limits = limits();
    let data = page_data(
        text,
        height,
        x,
        if long { 10 } else { 300 },
        width,
        long,
        carry,
    );
    let input = input(&root, data, text, None, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let mut budget =
        BookV2BodyLineBudget::new(1_000_000, limits.base().get().max_line_reshape_passes);
    with_budgeted_book_v2_column_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        None,
        &mut budget,
        &plan,
        None,
        |stable| {
            let flow =
                prepare_book_v2_column_flow_counted(&stable, None, &limits, 0, &mut 0).unwrap();
            let split = 1;
            let end = flow.body_items().len();
            let measured =
                prepare_book_v2_column_table_measurements_counted(flow, &limits, &mut 0).unwrap();
            let mut search = prepare_book_v2_column_page_search_counted(
                &measured,
                &limits,
                1_000_000,
                measured.record_charge(),
                &mut 0,
                &mut 0,
            )
            .unwrap();
            let state = search.begin().unwrap();
            let a = [BookV2BodyCandidatePart::Items { end: split }];
            let b = [BookV2BodyCandidatePart::Items { end }];
            let selected = search.evaluate_page(&state, None, &[&a, &b]).unwrap();
            assert_eq!(
                selected.is_some(),
                expected,
                "height={height},x={x},width={width},long={long}"
            );
            if let Some(selected) = selected {
                if long {
                    assert!(
                        selected.columns()[1].used_height()
                            > Length::from_raw(height * 65536).unwrap()
                    );
                }
                if carry {
                    let mut state = selected.into_next_state();
                    let mut pages = 1;
                    assert!(!state.is_complete());
                    while !state.is_complete() {
                        let selected = search
                            .evaluate_page(&state, None, &[&[], &[]])
                            .unwrap()
                            .unwrap();
                        assert!(selected
                            .columns()
                            .iter()
                            .all(|column| column.parts().is_empty()));
                        assert!(!selected.footnotes().unwrap().fragments().is_empty());
                        state = selected.into_next_state();
                        pages += 1;
                        assert!(pages < 30);
                    }
                    assert!(pages > 1);
                }
            }
        },
    )
    .unwrap();
}
#[test]
fn book_v2_column_page_candidates_require_all_new_definitions_on_the_same_page() {
    check_reservation(24, 10, 210, false, false, false);
    check_reservation(60, 10, 210, false, true, true);
}
#[test]
fn book_v2_column_page_candidates_reserve_only_overlapping_body_columns() {
    check_reservation(60, 10, 100, true, false, true);
    check_reservation(60, 10, 210, true, false, false);
}

fn check_tables(font: Option<&[u8]>) {
    let text = if font.is_some() { "表" } else { "Pro" };
    let root = Root::new();
    let limits = limits();
    let input = input(
        &root,
        super::super::table_width_frames::table_data(text, false),
        text,
        font,
        &limits,
    );
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let mut budget =
        BookV2BodyLineBudget::new(10_000_000, limits.base().get().max_line_reshape_passes);
    with_budgeted_book_v2_column_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        None,
        &mut budget,
        &plan,
        None,
        |stable| {
            let flow =
                prepare_book_v2_column_flow_counted(&stable, None, &limits, 0, &mut 0).unwrap();
            let measured =
                prepare_book_v2_column_table_measurements_counted(flow, &limits, &mut 0).unwrap();
            assert_eq!(measured.tables().len(), 4);
            let middle = measured.body_table_range(2).unwrap().start;
            let mut search = prepare_book_v2_column_page_search_counted(
                &measured,
                &limits,
                10_000_000,
                measured.record_charge(),
                &mut 0,
                &mut 0,
            )
            .unwrap();
            let state = search.begin().unwrap();
            let first = search.begin_table(0).unwrap();
            let last = search.begin_table(2).unwrap();
            let capacity = plan.page(0).unwrap().body().height().get();
            let a = [BookV2BodyCandidatePart::Table {
                cursor: first,
                capacity,
            }];
            let remaining = Length::from_raw(capacity.raw() / 2).unwrap();
            let b = [
                BookV2BodyCandidatePart::Items { end: middle },
                BookV2BodyCandidatePart::Table {
                    cursor: last,
                    capacity: remaining,
                },
            ];
            let selected = search
                .evaluate_page(&state, None, &[&a, &b])
                .unwrap()
                .unwrap();
            assert!(selected.next_state().is_complete());
            assert_eq!(selected.next_state().page_index(), 1);
            assert!(selected.columns()[0].parts()[0]
                .table()
                .unwrap()
                .after()
                .is_terminal());
            assert!(selected.columns()[1].parts()[1]
                .table()
                .unwrap()
                .after()
                .is_terminal());
            let mut seen = Vec::new();
            for column in selected.columns() {
                for part in column.parts() {
                    if let Some(range) = part.items() {
                        seen.extend(range);
                    }
                    if let Some(table) = part.table() {
                        for range in table.semantic_leaf_ranges() {
                            seen.extend(range);
                        }
                    }
                }
            }
            seen.sort_unstable();
            assert_eq!(seen, (0..measured.body_items().len()).collect::<Vec<_>>());
            // Reuse the exact table continuation within the second column of the
            // same physical page; the preliminary branch is never published.
            let mut partial = None;
            for height in (16..160).step_by(8) {
                let capacity = Length::from_raw(height * 65536).unwrap();
                let a = [BookV2BodyCandidatePart::Table {
                    cursor: first,
                    capacity,
                }];
                if let Some(candidate) = search.evaluate_page(&state, None, &[&a, &[]]).unwrap() {
                    if let Some(cursor) = candidate.next_state().table_continuation() {
                        partial = Some((capacity, cursor));
                        break;
                    }
                }
            }
            let (small, next) = partial.expect("actual caption/cell continuation");
            let a = [BookV2BodyCandidatePart::Table {
                cursor: first,
                capacity: small,
            }];
            let b = [BookV2BodyCandidatePart::Table {
                cursor: next,
                capacity,
            }];
            let selected = search
                .evaluate_page(&state, None, &[&a, &b])
                .unwrap()
                .unwrap();
            assert_eq!(selected.next_state().page_index(), 1);
            assert_eq!(
                selected.next_state().next_item(),
                measured.body_table_range(0).unwrap().end
            );
            assert_eq!(
                selected.columns()[1].parts()[0]
                    .table()
                    .unwrap()
                    .before()
                    .cell_progress_fingerprint(),
                next.cell_progress_fingerprint()
            );
        },
    )
    .unwrap();
}
#[test]
fn book_v2_column_page_candidates_keep_nested_tables_and_continuations() {
    check_tables(None);
}

#[test]
fn book_v2_column_page_candidates_follow_original_named_page_requests() {
    let root = Root::new();
    let limits = limits();
    let text = "Result";
    let mut data = page_data(text, 100, 10, 300, 210, false, false);
    for paragraph in data["document"]["blocks"].as_array_mut().unwrap() {
        paragraph["classes"] = json!(["column_named"]);
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"column_named","selector":"paragraph.column_named","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"page","important":false,"value":{"kind":"string","value":"appendix"}}]}));
    let mut master = data["page_masters"]["masters"][0].clone();
    master["master_id"] = "z-appendix".into();
    master["column_layout"] =
        json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"last_page"});
    data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .push(master);
    data["page_masters"]["selection_rules"] = json!([{"master_id":"z-appendix","parity":"any","first":null,"named_page":"appendix","source_order":0}]);
    let input = input(&root, data, text, None, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
    let name = plan
        .source_name_index(flow.paragraphs()[0].owner())
        .unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let mut budget =
        BookV2BodyLineBudget::new(1_000_000, limits.base().get().max_line_reshape_passes);
    with_budgeted_book_v2_column_lines(&policy,&flow,input.resources(),&bindings,&limits,MODE,None,&mut budget,&plan,None,|stable| {
        let flow=prepare_book_v2_column_flow_counted(&stable,None,&limits,0,&mut 0).unwrap();
        assert_eq!(flow.body_page_name_index(0),Some(name));
        let measured=prepare_book_v2_column_table_measurements_counted(flow,&limits,&mut 0).unwrap();
        let mut search=prepare_book_v2_column_page_search_counted(&measured,&limits,1_000_000,measured.record_charge(),&mut 0,&mut 0).unwrap();
        let state=search.begin().unwrap();let a=[BookV2BodyCandidatePart::Items{end:1}];let b=[BookV2BodyCandidatePart::Items{end:2}];
        assert!(matches!(search.evaluate_page(&state,None,&[&a,&b]),Err(e) if e.kind==E::PendingNamedPage));
        let candidate=search.evaluate_page(&state,Some(name),&[&a,&b]).unwrap().unwrap();
        assert_eq!(candidate.frames().named_page_index(),Some(name));
        assert!(candidate.next_state().is_complete());
    }).unwrap();
}
