fn production_table_break_fixture(rowspan: bool) -> serde_json::Value {
    use serde_json::json;
    let mut value = production_table_fixture();
    let table = &mut value["document"]["blocks"][0]["blocks"][0];
    let paragraph = table["head"][0]["cells"][0]["blocks"][0].clone();
    let span = paragraph["span"].clone();
    let cell = |count: usize, tall: bool, rowspan: u16, colspan: u16| {
        let mut paragraph = paragraph.clone();
        paragraph["classes"] = if tall { json!(["tall"]) } else { json!([]) };
        json!({"node_id":0,"span":span,"colspan":colspan,"rowspan":rowspan,"blocks":vec![paragraph;count]})
    };
    table["body"] = if rowspan {
        json!([
            {"node_id":0,"span":span,"cells":[cell(3,false,2,1),cell(2,false,1,1)]},
            {"node_id":0,"span":span,"cells":[cell(1,false,1,1)]},
            {"node_id":0,"span":span,"cells":[cell(1,false,1,2)]}
        ])
    } else {
        json!([{"node_id":0,"span":span,"cells":[cell(6,false,1,1),cell(4,true,1,1)]}])
    };
    for (name, number) in [
        ("line_height", 1_000_000),
        ("space_before", 0),
        ("space_after", 0),
    ] {
        production_body_set_style(&mut value, "paragraph", name, number.into());
    }
    let rules = value["style_sheet"]["rules"].as_array_mut().unwrap();
    let order = rules.len();
    rules.push(json!({"style_id":"table-tall-lines","selector":"paragraph.tall","extends":null,"source_order":order,
        "declarations":[{"important":false,"name":"line_height","value":{"kind":"length","value":1_500_000}}]}));
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_table_breaks_intersect_cell_endpoints_and_reserve_every_header() {
    use typaxis_pagination::{
        prepare_production_table_measurements, prepare_production_table_search,
    };
    let value = production_table_break_fixture(false);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let footnotes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &footnotes, limits).unwrap();
        let mut search = prepare_production_table_search(&measured, 0, limits, 100_000).unwrap();
        assert_eq!(search.header_height().raw(), 1_000_000);
        let begin = search.begin().unwrap();
        for raw in [0, 999_999, 1_000_000, 2_499_999] {
            assert!(search
                .evaluate(&begin, Length::from_raw(raw).unwrap())
                .unwrap()
                .is_none());
        }
        let first = search
            .evaluate(&begin, Length::from_raw(5_500_000).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(first.used_height().raw(), 4_000_000);
        assert_eq!(first.available_height().raw(), 5_500_000);
        assert!(!first.repeats_header());
        assert_eq!(
            first
                .cells()
                .iter()
                .map(|c| (c.cell_index(), c.content_range()))
                .collect::<Vec<_>>(),
            [(2, 0..3), (3, 0..2)]
        );
        assert!(first
            .cells()
            .iter()
            .all(|c| c.top().raw() == 1_000_000 && c.height().raw() == 3_000_000));
        let repeated = search
            .evaluate(&begin, Length::from_raw(5_500_000).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(first.fingerprint(), repeated.fingerprint());
        let narrower = search
            .evaluate(&begin, Length::from_raw(4_500_000).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(first.after().offset(), narrower.after().offset());
        assert_ne!(first.fingerprint(), narrower.fingerprint());
        let second = search
            .evaluate(&first.after(), Length::from_raw(5_500_000).unwrap())
            .unwrap()
            .unwrap();
        assert!(second.repeats_header());
        assert_eq!(second.used_height().raw(), 4_000_000);
        assert_eq!(
            second
                .cells()
                .iter()
                .map(|c| (c.cell_index(), c.content_range()))
                .collect::<Vec<_>>(),
            [(2, 3..6), (3, 2..4)]
        );
        assert!(second.after().is_terminal());
        assert!(search
            .evaluate(&second.after(), search.maximum_height())
            .is_err());
        assert_eq!(
            search
                .evaluate(&begin, Length::from_raw(12_000_001).unwrap())
                .err()
                .unwrap()
                .kind,
            typaxis_pagination::ProductionBodyPaginationErrorKind::InvalidTableCapacity
        );
        with_production_body_inputs(
            &value,
            &config(),
            |other_lines, other_blocks, other_limits| {
                let other_notes =
                    typaxis_layout::prepare_production_footnote_lines(other_lines, other_limits)
                        .unwrap();
                let other_measured = prepare_production_table_measurements(
                    other_lines,
                    other_blocks,
                    &other_notes,
                    other_limits,
                )
                .unwrap();
                let mut other_search =
                    prepare_production_table_search(&other_measured, 0, other_limits, 100_000)
                        .unwrap();
                assert_eq!(
                    other_search
                        .evaluate(&begin, Length::from_raw(5_500_000).unwrap())
                        .err()
                        .unwrap()
                        .kind,
                    typaxis_pagination::ProductionBodyPaginationErrorKind::ReceiptMismatch
                );
            },
        );
    });
}

#[test]
fn production_table_breaks_advance_rowspan_cells_through_content_and_padding() {
    use typaxis_pagination::{
        prepare_production_table_measurements, prepare_production_table_search,
    };
    let value = production_table_break_fixture(true);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        assert_eq!(
            measured.tables()[0]
                .rows()
                .iter()
                .map(|r| r.height().raw())
                .collect::<Vec<_>>(),
            [1_000_000, 2_000_000, 3_000_000, 1_000_000]
        );
        let mut search = prepare_production_table_search(&measured, 0, limits, 100_000).unwrap();
        let begin = search.begin().unwrap();
        let first = search
            .evaluate(&begin, Length::from_raw(3_000_000).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(first.after().next_row(), 2);
        let second = search
            .evaluate(&first.after(), Length::from_raw(2_500_000).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(second.cells()[0].content_range(), 2..3);
        assert_eq!(
            (
                second.cells()[0].offset_before().raw(),
                second.cells()[0].offset_after().raw()
            ),
            (2_000_000, 3_500_000)
        );
        let third = search
            .evaluate(&second.after(), Length::from_raw(3_000_000).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(third.after().next_row(), 3);
        assert_eq!(third.used_height().raw(), 2_500_000);
        assert!(third.cells().iter().all(|c| c.content_range().is_empty()));
        let fourth = search
            .evaluate(&third.after(), Length::from_raw(3_000_000).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(fourth.used_height().raw(), 2_000_000);
        assert_eq!(fourth.cells()[0].cell_index(), 5);
        assert_eq!(fourth.cells()[0].content_range(), 0..1);
        assert!(fourth.after().is_terminal());
    });
}

#[test]
fn production_table_breaks_never_cut_a_real_formula_at_an_unsafe_row_boundary() {
    use serde_json::json;
    use typaxis_pagination::{
        prepare_production_table_measurements, prepare_production_table_search,
        ProductionBodyFragmentSource,
    };
    let mut value = production_table_fixture();
    let table = &mut value["document"]["blocks"][0]["blocks"][0];
    let math = table["body"][0]["cells"][1]["blocks"][0].clone();
    let paragraph = table["body"][2]["cells"][0]["blocks"][0].clone();
    table["body"][0]["cells"][0]["blocks"] = json!([math]);
    table["body"][0]["cells"][0]["span"] = math["span"].clone();
    table["body"][0]["cells"][1]["blocks"] = json!([paragraph]);
    table["body"][0]["cells"][1]["span"] = paragraph["span"].clone();
    for (name, number) in [
        ("line_height", 1_000_000),
        ("space_before", 0),
        ("space_after", 0),
    ] {
        production_body_set_style(&mut value, "paragraph", name, number.into());
    }
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let mut search = prepare_production_table_search(&measured, 0, limits, 100_000).unwrap();
        let begin = search.begin().unwrap();
        assert_eq!(measured.tables()[0].rows()[1].height().raw(), 1_000_000);
        assert!(measured.tables()[0].cells()[2].natural_height().raw() > 1_000_000);
        assert!(search
            .evaluate(&begin, Length::from_raw(2_000_000).unwrap())
            .unwrap()
            .is_none());
        let selected = search
            .evaluate(&begin, Length::from_raw(3_000_000).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(selected.cells()[0].content_range(), 0..1);
        let typaxis_pagination::ProductionTableContentSource::FlowItem(index) =
            measured.tables()[0].cells()[2].content()[0].source()
        else {
            panic!("real formula must retain its leaf");
        };
        assert!(matches!(
            measured.item(index).unwrap().source(),
            Some(ProductionBodyFragmentSource::VectorBlock { .. })
        ));
    });
}

#[test]
fn production_table_break_search_retains_exact_record_and_work_budgets() {
    use typaxis_pagination::{
        prepare_production_table_measurements, prepare_production_table_search,
        ProductionBodyPaginationError, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_table_break_fixture(false);
    let mut required_records = 0;
    for delta in [None, Some(0), Some(1)] {
        let cfg = delta.map_or_else(config, |delta| {
            config_with_limits(ResourceLimits {
                max_fragments: required_records - delta,
                ..ResourceLimits::default()
            })
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured =
                prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
            let run = |work| -> Result<(u64, u64), ProductionBodyPaginationError> {
                let mut search = prepare_production_table_search(&measured, 0, limits, work)?;
                let begin = search.begin()?;
                let first = search
                    .evaluate(&begin, Length::from_raw(5_500_000).unwrap())?
                    .unwrap();
                let second = search
                    .evaluate(&first.after(), Length::from_raw(5_500_000).unwrap())?
                    .unwrap();
                assert!(second.after().is_terminal());
                Ok((search.record_charge(), search.work_charge()))
            };
            let result = run(100_000);
            if delta == Some(1) {
                assert_eq!(result.unwrap_err().kind, E::FragmentLimit);
                return;
            }
            let (records, work) = result.unwrap();
            required_records = records;
            assert_eq!(run(work).unwrap(), (records, work));
            assert_eq!(run(work - 1).unwrap_err().kind, E::TableSearchLimit);
            assert_eq!(run(0).unwrap_err().kind, E::TableSearchLimit);
        });
    }
}

#[test]
fn production_table_breaks_complete_zero_height_rows_structurally() {
    let mut value = production_table_break_fixture(false);
    for cell in value["document"]["blocks"][0]["blocks"][0]["body"][0]["cells"]
        .as_array_mut()
        .unwrap()
    {
        cell["blocks"] = serde_json::json!([]);
    }
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured = typaxis_pagination::prepare_production_table_measurements(
            lines, blocks, &notes, limits,
        )
        .unwrap();
        assert_eq!(measured.tables()[0].rows()[1].height(), Length::ZERO);
        let mut search =
            typaxis_pagination::prepare_production_table_search(&measured, 0, limits, 100_000)
                .unwrap();
        let before = search.begin().unwrap();
        let selected = search
            .evaluate(&before, search.header_height())
            .unwrap()
            .unwrap();
        assert_eq!(selected.before().offset(), selected.after().offset());
        assert_eq!(selected.before().next_row(), 1);
        assert_eq!(selected.after().next_row(), 2);
        assert!(selected.after().is_terminal());
        assert_eq!(selected.used_height().raw(), 1_000_000);
        assert!(selected.cells().is_empty());
    });
}

#[test]
fn production_table_breaks_reject_oversize_headers_and_unbreakable_cell_keeps() {
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for header in [true, false] {
        let mut value = production_table_break_fixture(false);
        value["page_masters"]["masters"][0]["body"]["height"] =
            if header { 999_999 } else { 5_000_000 }.into();
        if !header {
            production_body_set_style(&mut value, "paragraph", "keep_with_next", true.into());
        }
        with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured = typaxis_pagination::prepare_production_table_measurements(
                lines, blocks, &notes, limits,
            )
            .unwrap();
            let result =
                typaxis_pagination::prepare_production_table_search(&measured, 0, limits, 100_000);
            if header {
                assert_eq!(result.err().unwrap().kind, E::TableHeaderOversize);
                return;
            }
            let mut search = result.unwrap();
            let before = search.begin().unwrap();
            assert!(search
                .evaluate(&before, Length::from_raw(4_000_000).unwrap())
                .unwrap()
                .is_none());
            assert_eq!(
                search
                    .evaluate(&before, search.maximum_height())
                    .err()
                    .unwrap()
                    .kind,
                E::Oversize
            );
        });
    }
}

#[test]
fn production_table_breaks_preserve_pending_nested_and_forced_boundaries() {
    use serde_json::json;
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for nested in [false, true] {
        let mut value = production_table_break_fixture(true);
        let table = &mut value["document"]["blocks"][0]["blocks"][0];
        let point = table["head"][0]["span"].clone();
        if nested {
            let mut child = table.clone();
            child["span"] = point;
            table["body"][2]["cells"][0]["blocks"] = json!([child]);
        } else {
            table["body"][0]["cells"][0]["blocks"]
                .as_array_mut()
                .unwrap()
                .insert(
                    1,
                    json!({"kind":"page_break","node_id":0,"span":point,"classes":[]}),
                );
        }
        production_body_renumber(&mut value["document"], &mut 0);
        with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured = typaxis_pagination::prepare_production_table_measurements(
                lines, blocks, &notes, limits,
            )
            .unwrap();
            let error =
                typaxis_pagination::prepare_production_table_search(&measured, 0, limits, 100_000)
                    .err()
                    .unwrap();
            assert_eq!(
                error.kind,
                E::PendingRegion(if nested {
                    "nested_table_breaks"
                } else {
                    "table_forced_break"
                })
            );
            if nested {
                assert_eq!(error.owner, measured.tables()[1].owner());
                let mut inner = typaxis_pagination::prepare_production_table_search(
                    &measured, 1, limits, 100_000,
                )
                .unwrap();
                let before = inner.begin().unwrap();
                assert!(inner
                    .evaluate(&before, inner.maximum_height())
                    .unwrap()
                    .unwrap()
                    .after()
                    .is_terminal());
            }
        });
    }
}

#[test]
fn production_table_break_search_indexes_a_long_rowspan_without_scanning_all_prior_rows() {
    use serde_json::json;
    let mut value = production_table_break_fixture(true);
    let table = &mut value["document"]["blocks"][0]["blocks"][0];
    let mut cell = table["head"][0]["cells"][0].clone();
    cell["rowspan"] = 1.into();
    let mut spanning = cell.clone();
    spanning["rowspan"] = 1000.into();
    let span = cell["span"].clone();
    let mut rows = vec![json!({"node_id":0,"span":span,"cells":[spanning,cell]})];
    rows.extend((1..1000).map(|_| json!({"node_id":0,"span":span,"cells":[cell]})));
    table["body"] = json!(rows);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured = typaxis_pagination::prepare_production_table_measurements(
            lines, blocks, &notes, limits,
        )
        .unwrap();
        let mut search =
            typaxis_pagination::prepare_production_table_search(&measured, 0, limits, 100_000)
                .unwrap();
        let mut cursor = search.begin().unwrap();
        let mut content = 0usize;
        for ordinal in 0..1000 {
            let selected = search
                .evaluate(&cursor, Length::from_raw(2_000_000).unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(selected.cells().len(), 2);
            assert_eq!(selected.after().next_row(), ordinal + 2);
            content += selected
                .cells()
                .iter()
                .map(|c| c.content_range().len())
                .sum::<usize>();
            cursor = selected.after();
        }
        assert!(cursor.is_terminal());
        assert_eq!(content, 1001);
        // A retained span must not force a scan of the ever-growing prefix of
        // completed rows. This checks charged search work, not host wall time.
        assert!(search.work_charge() < 100_000);
    });
}
