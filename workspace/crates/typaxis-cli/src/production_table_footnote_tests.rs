fn production_table_footnote_fixture(long: bool) -> serde_json::Value {
    use serde_json::json;
    let mut value = production_table_break_fixture(false);
    let table = &mut value["document"]["blocks"][0]["blocks"][0];
    let paragraph = table["head"][0]["cells"][0]["blocks"][0].clone();
    let point = paragraph["span"].clone();
    let reference =
        |id| json!({"kind":"footnote_reference","node_id":0,"span":point,"footnote_id":id});
    table["head"][0]["cells"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(reference("header"));
    // The later source cell is painted on the first fragment; the reference in
    // the earlier cell's last paragraph belongs only to the continuation.
    table["body"][0]["cells"][0]["blocks"][5]["children"]
        .as_array_mut()
        .unwrap()
        .push(reference("late"));
    table["body"][0]["cells"][1]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(reference("early"));
    value["document"]["footnotes"] = json!([
        {"node_id":0,"span":point,"footnote_id":"header","blocks":[paragraph]},
        {"node_id":0,"span":point,"footnote_id":"early","blocks":[paragraph]},
        {"node_id":0,"span":point,"footnote_id":"late","blocks":vec![paragraph; if long {8} else {1}]}
    ]);
    let master = &mut value["page_masters"]["masters"][0];
    master["body"]["height"] = 7_000_000.into();
    master["footnote"] = json!({"x":master["body"]["x"],"y":master["body"]["y"].as_i64().unwrap()+1_000_000,"width":8_000_000,"height":6_000_000});
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_table_footnotes_reserve_selected_parallel_cells_and_retry_without_losing_state() {
    use typaxis_pagination::{
        prepare_production_table_footnote_search, prepare_production_table_measurements,
        ProductionFootnoteDemandStatus as Status,
    };
    let value = production_table_footnote_fixture(false);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let mut search =
            prepare_production_table_footnote_search(&measured, 0, limits, 100_000).unwrap();
        let before = search.begin().unwrap();
        assert!(search
            .evaluate(&before, Length::from_raw(7_000_000).unwrap())
            .unwrap()
            .is_none());
        assert_eq!(before.demand().status(0), Some(Status::Unreferenced));
        let first = search
            .evaluate(&before, Length::from_raw(5_500_000).unwrap())
            .unwrap()
            .unwrap();
        first.verify(&before).unwrap();
        assert_eq!(first.table().unwrap().used_height().raw(), 4_000_000);
        let ids = |f: &typaxis_pagination::ProductionTableFootnoteSelection<'_, '_, '_, '_, '_>| {
            f.footnotes()
                .unwrap()
                .fragments()
                .iter()
                .map(|f| f.fragment().definition_index())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&first), [0, 1]);
        assert_eq!(
            first.next_state().demand().status(2),
            Some(Status::Unreferenced)
        );
        let bounds = first.footnote_bounds().unwrap();
        assert!(
            bounds.y()
                >= blocks
                    .page_geometry()
                    .body()
                    .y()
                    .checked_add(first.table().unwrap().used_height())
                    .unwrap()
        );
        let second = search
            .evaluate(first.next_state(), Length::from_raw(5_500_000).unwrap())
            .unwrap()
            .unwrap();
        assert!(second.table().unwrap().repeats_header());
        assert_eq!(ids(&second), [2]);
        assert!(second.next_state().is_complete());
        assert!(search
            .evaluate(second.next_state(), Length::ZERO)
            .unwrap()
            .is_none());
        assert!(second.verify(&before).is_err());
        let mut foreign =
            prepare_production_table_footnote_search(&measured, 0, limits, 100_000).unwrap();
        assert!(foreign
            .evaluate(&before, Length::from_raw(5_500_000).unwrap())
            .is_err());
    });
}

#[test]
fn production_table_footnotes_continue_after_the_last_table_fragment() {
    use typaxis_pagination::{
        prepare_production_table_footnote_search, prepare_production_table_measurements,
    };
    let value = production_table_footnote_fixture(true);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let mut search =
            prepare_production_table_footnote_search(&measured, 0, limits, 100_000).unwrap();
        let begin = search.begin().unwrap();
        let mut selections = Vec::new();
        loop {
            let state = selections.last().map_or(
                &begin,
                |s: &typaxis_pagination::ProductionTableFootnoteSelection<'_, '_, '_, '_, '_>| {
                    s.next_state()
                },
            );
            if state.is_complete() {
                break;
            }
            assert!(selections.len() < 8);
            let selected = search
                .evaluate(state, Length::from_raw(5_500_000).unwrap())
                .unwrap()
                .unwrap();
            selections.push(selected);
        }
        assert!(selections.len() > 2);
        assert!(selections.iter().skip(2).all(|s| s.table().is_none()));
        let late = selections
            .iter()
            .flat_map(|s| s.footnotes().unwrap().fragments())
            .filter(|f| f.fragment().definition_index() == 2)
            .collect::<Vec<_>>();
        assert_eq!(
            late.iter()
                .map(|f| f.fragment().items().len())
                .sum::<usize>(),
            8
        );
        assert_eq!(
            late.iter()
                .filter(|f| f.fragment().marker().is_some())
                .count(),
            1
        );
    });
}

#[test]
fn production_table_footnotes_keep_one_cumulative_record_and_work_budget() {
    use typaxis_pagination::{
        prepare_production_table_footnote_search, prepare_production_table_measurements,
        ProductionBodyPaginationError, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_table_footnote_fixture(false);
    let mut required = 0;
    for delta in [None, Some(0), Some(1)] {
        let cfg = delta.map_or_else(config, |delta| {
            config_with_limits(ResourceLimits {
                max_fragments: required - delta,
                ..ResourceLimits::default()
            })
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured =
                prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
            let run = |work| -> Result<(u64, u64), ProductionBodyPaginationError> {
                let mut search =
                    prepare_production_table_footnote_search(&measured, 0, limits, work)?;
                let before = search.begin()?;
                assert!(search
                    .evaluate(&before, Length::from_raw(7_000_000).unwrap())?
                    .is_none());
                let first = search
                    .evaluate(&before, Length::from_raw(5_500_000).unwrap())?
                    .unwrap();
                let second = search
                    .evaluate(first.next_state(), Length::from_raw(5_500_000).unwrap())?
                    .unwrap();
                assert!(second.next_state().is_complete());
                Ok((search.record_charge(), search.work_charge()))
            };
            let result = run(100_000);
            if delta == Some(1) {
                assert_eq!(result.unwrap_err().kind, E::FragmentLimit);
                return;
            }
            let (records, work) = result.unwrap();
            required = records;
            assert_eq!(run(work).unwrap(), (records, work));
            assert!(matches!(
                run(work - 1).unwrap_err().kind,
                E::FootnoteSearchLimit | E::TableSearchLimit
            ));
        });
    }
}

#[test]
fn production_table_footnotes_can_finish_incoming_notes_before_advancing_the_table() {
    use typaxis_pagination::{
        prepare_production_table_footnote_search, prepare_production_table_measurements,
    };
    let mut value = production_table_footnote_fixture(false);
    let paragraph = value["document"]["footnotes"][1]["blocks"][0].clone();
    value["document"]["footnotes"][1]["blocks"] = serde_json::json!(vec![paragraph; 5]);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let mut search =
            prepare_production_table_footnote_search(&measured, 0, limits, 100_000).unwrap();
        let begin = search.begin().unwrap();
        let first = search
            .evaluate(&begin, Length::from_raw(5_500_000).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(first.next_state().demand().pending_definitions(), [1]);
        let note_only = search
            .evaluate(first.next_state(), Length::ZERO)
            .unwrap()
            .unwrap();
        assert!(note_only.table().is_none());
        assert_eq!(
            note_only.next_state().table_cursor().offset(),
            first.next_state().table_cursor().offset()
        );
        assert!(note_only
            .next_state()
            .demand()
            .pending_definitions()
            .is_empty());
        assert!(!note_only.next_state().is_complete());
        let end = search
            .evaluate(note_only.next_state(), Length::from_raw(5_500_000).unwrap())
            .unwrap()
            .unwrap();
        assert!(end.next_state().is_complete());
        let early = [&first, &note_only]
            .iter()
            .flat_map(|s| s.footnotes().unwrap().fragments())
            .filter(|f| f.fragment().definition_index() == 1)
            .collect::<Vec<_>>();
        assert_eq!(
            early
                .iter()
                .map(|f| f.fragment().items().len())
                .sum::<usize>(),
            5
        );
        assert_eq!(
            early
                .iter()
                .filter(|f| f.fragment().marker().is_some())
                .count(),
            1
        );
    });
}

#[test]
fn production_footnote_definition_lookup_preserves_authored_unsorted_numbers() {
    let mut value = production_footnote_flow_fixture();
    value["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let definitions = flow.footnotes().definitions();
        assert_eq!(
            definitions
                .iter()
                .map(|d| (d.id(), d.number()))
                .collect::<Vec<_>>(),
            [("second", 1), ("first", 2)]
        );
        assert_eq!(
            flow.references()
                .iter()
                .map(|r| r.source().definition_index())
                .collect::<Vec<_>>(),
            [1, 0]
        );
        let mut search =
            typaxis_pagination::prepare_production_footnote_demand_search(flow, limits, 100_000)
                .unwrap();
        let before = search.begin().unwrap();
        let demanded = search
            .require_body(&before, 0..flow.body_items().len())
            .unwrap();
        assert_eq!(demanded.pending_definitions(), [1, 0]);
    });
}
