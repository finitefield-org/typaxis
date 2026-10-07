fn production_mixed_table_fixture() -> serde_json::Value {
    use serde_json::json;
    let mut value = production_table_footnote_fixture(false);
    let table = value["document"]["blocks"][0]["blocks"][0].clone();
    let paragraph = table["head"][0]["cells"][1]["blocks"][0].clone();
    let mut prefix = paragraph.clone();
    prefix["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":paragraph["span"],"footnote_id":"prefix"}));
    value["document"]["footnotes"].as_array_mut().unwrap().push(
        json!({"node_id":0,"span":paragraph["span"],"footnote_id":"prefix","blocks":[paragraph]}),
    );
    value["document"]["blocks"][0]["blocks"] = json!([prefix, table, paragraph]);
    let master = &mut value["page_masters"]["masters"][0];
    master["body"]["height"] = 9_000_000.into();
    master["footnote"]["y"] = (master["body"]["y"].as_i64().unwrap() + 3_000_000).into();
    production_body_renumber(&mut value["document"], &mut 0);
    value
}
fn production_measured_table_end(table: &typaxis_pagination::ProductionMeasuredTable) -> usize {
    let typaxis_pagination::ProductionTableContentSource::FlowItem(last) = table
        .cells()
        .last()
        .unwrap()
        .content()
        .last()
        .unwrap()
        .source()
    else {
        panic!("flat test table")
    };
    last + 1
}
#[test]
fn production_mixed_body_selects_prefix_table_continuation_suffix_and_actual_footnotes() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyCandidatePart as Part,
    };
    let value = production_mixed_table_fixture();
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let table_end = production_measured_table_end(&measured.tables()[0]);
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let state = search.begin().unwrap();
        let table = search.begin_table(0).unwrap();
        assert!(search
            .evaluate_mixed_candidate(
                &state,
                0,
                &[
                    Part::Items { end: 1 },
                    Part::Table {
                        cursor: table,
                        capacity: Length::from_raw(7_000_000).unwrap()
                    }
                ]
            )
            .unwrap()
            .is_none());
        let first = search
            .evaluate_mixed_candidate(
                &state,
                0,
                &[
                    Part::Items { end: 1 },
                    Part::Table {
                        cursor: table,
                        capacity: Length::from_raw(5_500_000).unwrap(),
                    },
                ],
            )
            .unwrap()
            .unwrap();
        first.verify(&state).unwrap();
        assert_eq!((first.start_item(), first.next_item()), (0, 1));
        assert_eq!(first.parts()[0].items(), Some(0..1));
        assert_eq!(first.parts()[0].top().raw(), 0);
        assert_eq!(first.parts()[1].top().raw(), 1_000_000);
        assert_eq!(first.used_height().raw(), 5_000_000);
        assert_eq!(
            first
                .footnotes()
                .unwrap()
                .fragments()
                .iter()
                .map(|f| f.fragment().definition_index())
                .collect::<Vec<_>>(),
            [3, 0, 1]
        );
        let next = search
            .evaluate_mixed_candidate(
                first.next_state(),
                first.next_item(),
                &[
                    Part::Table {
                        cursor: first.table_continuation().unwrap(),
                        capacity: Length::from_raw(5_500_000).unwrap(),
                    },
                    Part::Items { end: table_end + 1 },
                ],
            )
            .unwrap()
            .unwrap();
        assert_eq!(next.next_item(), table_end + 1);
        assert!(next.table_continuation().is_none());
        assert_eq!(next.parts()[0].top().raw(), 0);
        assert!(next.parts()[0].table().unwrap().repeats_header());
        assert_eq!(next.parts()[1].items(), Some(table_end..table_end + 1));
        assert_eq!(next.parts()[1].top().raw(), 4_000_000);
        assert_eq!(next.used_height().raw(), 5_000_000);
        assert_eq!(
            next.footnotes()
                .unwrap()
                .fragments()
                .iter()
                .map(|f| f.fragment().definition_index())
                .collect::<Vec<_>>(),
            [2]
        );
        assert!(next.next_state().pending_definitions().is_empty());
        assert!(next.verify(&state).is_err());
        // The exact same common demand owner accepts a prefix selected before
        // the table query, including a pending definition from ordinary body.
        let incoming = search.require_body(&state, 0..1).unwrap();
        let table_only = search
            .evaluate_mixed_candidate(
                &incoming,
                1,
                &[Part::Table {
                    cursor: table,
                    capacity: Length::from_raw(5_500_000).unwrap(),
                }],
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            table_only
                .footnotes()
                .unwrap()
                .fragments()
                .iter()
                .map(|f| f.fragment().definition_index())
                .collect::<Vec<_>>(),
            [3, 0, 1]
        );
    });
}
#[test]
fn production_mixed_body_rejects_flattening_and_discontinuous_or_over_capacity_parts() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyCandidatePart as Part, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_mixed_table_fixture();
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let end = production_measured_table_end(&measured.tables()[0]);
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let state = search.begin().unwrap();
        let table = search.begin_table(0).unwrap();
        assert_eq!(
            search.begin_table(99).err().unwrap().kind,
            E::ReceiptMismatch
        );
        assert_eq!(
            search.require_body(&state, 0..end).err().unwrap().kind,
            E::ReceiptMismatch
        );
        assert_eq!(
            search
                .evaluate_body_candidate(&state, 0..end)
                .err()
                .unwrap()
                .kind,
            E::ReceiptMismatch
        );
        let pending = search.begin_pages().err().unwrap();
        assert_eq!(pending.kind, E::PendingRegion("table_page_selection"));
        assert_eq!(pending.owner, measured.tables()[0].owner());
        assert_eq!(
            search
                .evaluate_mixed_candidate(&state, 0, &[Part::Items { end }])
                .err()
                .unwrap()
                .kind,
            E::ReceiptMismatch
        );
        assert_eq!(
            search
                .evaluate_mixed_candidate(
                    &state,
                    0,
                    &[Part::Table {
                        cursor: table,
                        capacity: Length::from_raw(4_000_000).unwrap()
                    }]
                )
                .err()
                .unwrap()
                .kind,
            E::ReceiptMismatch
        );
        assert_eq!(
            search
                .evaluate_mixed_candidate(
                    &state,
                    0,
                    &[
                        Part::Items { end: 1 },
                        Part::Table {
                            cursor: table,
                            capacity: Length::from_raw(8_000_001).unwrap()
                        }
                    ]
                )
                .err()
                .unwrap()
                .kind,
            E::InvalidTableCapacity
        );
        assert_eq!(
            search
                .evaluate_mixed_candidate(
                    &state,
                    1,
                    &[
                        Part::Table {
                            cursor: table,
                            capacity: Length::from_raw(4_000_000).unwrap()
                        },
                        Part::Items { end: end + 1 }
                    ]
                )
                .err()
                .unwrap()
                .kind,
            E::ReceiptMismatch
        );
        let mut other = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        assert_eq!(
            other
                .evaluate_mixed_candidate(&state, 0, &[])
                .err()
                .unwrap()
                .kind,
            E::ReceiptMismatch
        );
    });
}
#[test]
fn production_mixed_body_keeps_multiple_tables_and_outside_spacing_in_one_candidate() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyCandidatePart as Part,
    };
    let mut value = production_table_break_fixture(false);
    let table = value["document"]["blocks"][0]["blocks"][0].clone();
    value["document"]["blocks"][0]["blocks"] = serde_json::json!([table, table]);
    value["page_masters"]["masters"][0]["body"]["height"] = 16_000_000.into();
    let paragraph_rule = value["style_sheet"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["selector"] == "paragraph")
        .unwrap()
        .clone();
    let mut extra = Vec::new();
    for (name, replacement) in [
        ("space_before", 111.into()),
        ("space_after", 222.into()),
        ("keep_with_next", true.into()),
    ] {
        let mut declaration = paragraph_rule["declarations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["name"] == name)
            .unwrap()
            .clone();
        declaration["value"]["value"] = replacement;
        extra.push(declaration);
    }
    value["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["selector"] == "table.common-grid")
        .unwrap()["declarations"]
        .as_array_mut()
        .unwrap()
        .extend(extra);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let first_end = production_measured_table_end(&measured.tables()[0]);
        let end = production_measured_table_end(&measured.tables()[1]);
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let state = search.begin().unwrap();
        let a = search.begin_table(0).unwrap();
        let b = search.begin_table(1).unwrap();
        let capacity = Length::from_raw(7_000_000).unwrap();
        assert!(search
            .evaluate_mixed_candidate(
                &state,
                0,
                &[Part::Table {
                    cursor: a,
                    capacity
                }]
            )
            .unwrap()
            .is_none());
        assert!(search
            .evaluate_mixed_candidate(
                &state,
                first_end,
                &[Part::Table {
                    cursor: b,
                    capacity
                }]
            )
            .unwrap()
            .is_none());
        let joined = search
            .evaluate_mixed_candidate(
                &state,
                0,
                &[
                    Part::Table {
                        cursor: a,
                        capacity,
                    },
                    Part::Table {
                        cursor: b,
                        capacity,
                    },
                ],
            )
            .unwrap()
            .unwrap();
        assert_eq!(joined.next_item(), end);
        assert_eq!(joined.parts()[0].top().raw(), 0);
        assert_eq!(joined.parts()[1].top().raw(), 7_000_333);
        assert_eq!(joined.used_height().raw(), 14_000_333);
        assert!(joined.footnotes().is_none());
    });
}
#[test]
fn production_mixed_body_retains_all_table_preparation_and_candidate_budgets() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyCandidatePart as Part, ProductionBodyPaginationError,
        ProductionBodyPaginationErrorKind as E,
    };
    let value = production_mixed_table_fixture();
    let mut required = 0;
    for delta in [None, Some(0), Some(1)] {
        let cfg = delta.map_or_else(config, |n| {
            config_with_limits(ResourceLimits {
                max_fragments: required - n,
                ..ResourceLimits::default()
            })
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured =
                prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
            let end = production_measured_table_end(&measured.tables()[0]);
            let run = |work| -> Result<(u64, u64), ProductionBodyPaginationError> {
                let mut search = prepare_production_table_body_search(&measured, limits, work)?;
                let state = search.begin()?;
                let table = search.begin_table(0)?;
                assert!(search
                    .evaluate_mixed_candidate(
                        &state,
                        0,
                        &[
                            Part::Items { end: 1 },
                            Part::Table {
                                cursor: table,
                                capacity: Length::from_raw(7_000_000).unwrap()
                            }
                        ]
                    )?
                    .is_none());
                let first = search
                    .evaluate_mixed_candidate(
                        &state,
                        0,
                        &[
                            Part::Items { end: 1 },
                            Part::Table {
                                cursor: table,
                                capacity: Length::from_raw(5_500_000).unwrap(),
                            },
                        ],
                    )?
                    .unwrap();
                let second = search
                    .evaluate_mixed_candidate(
                        first.next_state(),
                        first.next_item(),
                        &[
                            Part::Table {
                                cursor: first.table_continuation().unwrap(),
                                capacity: Length::from_raw(5_500_000).unwrap(),
                            },
                            Part::Items { end: end + 1 },
                        ],
                    )?
                    .unwrap();
                assert_eq!(second.next_item(), end + 1);
                Ok((search.record_charge(), search.work_steps()))
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
                E::TableSearchLimit | E::FootnoteSearchLimit
            ));
        });
    }
}

#[test]
fn production_mixed_body_preserves_table_offset_on_an_incoming_footnote_only_candidate() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyCandidatePart as Part,
    };
    let mut value = production_mixed_table_fixture();
    let paragraph = value["document"]["footnotes"][3]["blocks"][0].clone();
    value["document"]["footnotes"][3]["blocks"] = serde_json::json!(vec![paragraph; 5]);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let end = production_measured_table_end(&measured.tables()[0]);
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let state = search.begin().unwrap();
        let table = search.begin_table(0).unwrap();
        let first = search
            .evaluate_mixed_candidate(
                &state,
                0,
                &[
                    Part::Items { end: 1 },
                    Part::Table {
                        cursor: table,
                        capacity: Length::from_raw(5_500_000).unwrap(),
                    },
                ],
            )
            .unwrap()
            .unwrap();
        let cursor = first.table_continuation().unwrap();
        assert_eq!(first.next_state().pending_definitions(), [3]);
        let only_notes = search
            .evaluate_mixed_candidate(
                first.next_state(),
                first.next_item(),
                &[Part::Table {
                    cursor,
                    capacity: Length::ZERO,
                }],
            )
            .unwrap()
            .unwrap();
        assert!(only_notes.parts().is_empty());
        assert_eq!(only_notes.used_height(), Length::ZERO);
        assert_eq!(only_notes.next_item(), first.next_item());
        assert_eq!(
            only_notes.table_continuation().unwrap().offset(),
            cursor.offset()
        );
        assert_eq!(
            only_notes.table_continuation().unwrap().next_row(),
            cursor.next_row()
        );
        assert!(only_notes.next_state().pending_definitions().is_empty());
        let finish = search
            .evaluate_mixed_candidate(
                only_notes.next_state(),
                only_notes.next_item(),
                &[
                    Part::Table {
                        cursor: only_notes.table_continuation().unwrap(),
                        capacity: Length::from_raw(5_500_000).unwrap(),
                    },
                    Part::Items { end: end + 1 },
                ],
            )
            .unwrap()
            .unwrap();
        assert_eq!(finish.next_item(), end + 1);
        let count = [&first, &only_notes]
            .iter()
            .flat_map(|c| c.footnotes().unwrap().fragments())
            .filter(|f| f.fragment().definition_index() == 3)
            .map(|f| f.fragment().items().len())
            .sum::<usize>();
        assert_eq!(count, 5);
    });
}
#[test]
fn production_mixed_body_prefix_keep_is_satisfied_by_the_first_table_fragment() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyCandidatePart as Part,
    };
    let mut value = production_mixed_table_fixture();
    value["document"]["blocks"][0]["blocks"][0]["classes"] = serde_json::json!(["lead-keep"]);
    let rules = value["style_sheet"]["rules"].as_array_mut().unwrap();
    let mut keep = rules.iter().find(|r| r["selector"] == "paragraph").unwrap()["declarations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "keep_with_next")
        .unwrap()
        .clone();
    keep["value"]["value"] = true.into();
    let order = rules.len();
    rules.push(serde_json::json!({"style_id":"lead-keep","selector":"paragraph.lead-keep","extends":null,"source_order":order,"declarations":[keep]}));
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let end = production_measured_table_end(&measured.tables()[0]);
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let state = search.begin().unwrap();
        let table = search.begin_table(0).unwrap();
        let capacity = Length::from_raw(5_500_000).unwrap();
        assert!(search
            .evaluate_mixed_candidate(
                &state,
                1,
                &[Part::Table {
                    cursor: table,
                    capacity
                }]
            )
            .unwrap()
            .is_none());
        let first = search
            .evaluate_mixed_candidate(
                &state,
                0,
                &[
                    Part::Items { end: 1 },
                    Part::Table {
                        cursor: table,
                        capacity,
                    },
                ],
            )
            .unwrap()
            .unwrap();
        let second = search
            .evaluate_mixed_candidate(
                first.next_state(),
                first.next_item(),
                &[
                    Part::Table {
                        cursor: first.table_continuation().unwrap(),
                        capacity,
                    },
                    Part::Items { end: end + 1 },
                ],
            )
            .unwrap()
            .unwrap();
        assert_eq!(second.next_item(), end + 1);
    });
}

#[test]
fn production_mixed_body_ordinary_suffix_does_not_inherit_a_parallel_cell_keep() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
    };
    let mut value = production_mixed_table_fixture();
    value["document"]["blocks"][0]["blocks"][1]["body"][0]["cells"][1]["blocks"][3]["classes"] =
        serde_json::json!(["cell-end-keep", "tall"]);
    let rules = value["style_sheet"]["rules"].as_array_mut().unwrap();
    let mut keep = rules.iter().find(|r| r["selector"] == "paragraph").unwrap()["declarations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "keep_with_next")
        .unwrap()
        .clone();
    keep["value"]["value"] = true.into();
    let order = rules.len();
    rules.push(serde_json::json!({"style_id":"cell-end-keep","selector":"paragraph.cell-end-keep","extends":null,"source_order":order,"declarations":[keep]}));
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let end = production_measured_table_end(&measured.tables()[0]);
        assert!(measured.item(end - 1).unwrap().keep_with_next());
        assert!(!measured.tables()[0].keep_with_next());
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let state = search.begin().unwrap();
        let suffix = search
            .evaluate_body_candidate(&state, end..end + 1)
            .unwrap()
            .unwrap();
        assert_eq!(suffix.body_range(), end..end + 1);
        assert_eq!(suffix.body_height().raw(), 1_000_000);
    });
}
