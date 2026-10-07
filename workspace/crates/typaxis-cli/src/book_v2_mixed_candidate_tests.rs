use super::*;
use typaxis_core::NodeId;
use typaxis_pagination::book_v2::{
    prepare_book_v2_table_body_search, BookV2BodyCandidatePart as Part, BookV2TableMeasurements,
};
use typaxis_pagination::ProductionFootnoteDemandStatus as Status;

fn with_measured(
    data: Value,
    check: impl FnMut(&BookV2TableMeasurements<'_, '_, '_, '_>, &M4EffectiveResourceLimits),
) {
    with_measured_options(
        data,
        limits(),
        rect(500_000, 500_000, 10_000_000, 20_000_000),
        check,
    )
}
fn with_measured_options(
    data: Value,
    limits: M4EffectiveResourceLimits,
    body: typaxis_core::Rect,
    mut check: impl FnMut(&BookV2TableMeasurements<'_, '_, '_, '_>, &M4EffectiveResourceLimits),
) {
    with_measured_resources_options(data, limits, body, |measured, limits, _| {
        check(measured, limits)
    });
}
fn with_measured_resources_options(
    data: Value,
    limits: M4EffectiveResourceLimits,
    body: typaxis_core::Rect,
    check: impl FnMut(
        &BookV2TableMeasurements<'_, '_, '_, '_>,
        &M4EffectiveResourceLimits,
        &typaxis_resources::AdmittedProductionResourceLedgerV3,
    ),
) {
    with_measured_resources_candidates(data, limits, body, None, check);
}
fn with_measured_resources_candidates(
    mut data: Value,
    limits: M4EffectiveResourceLimits,
    body: typaxis_core::Rect,
    candidates: Option<&[(NodeId, u32)]>,
    mut check: impl FnMut(
        &BookV2TableMeasurements<'_, '_, '_, '_>,
        &M4EffectiveResourceLimits,
        &typaxis_resources::AdmittedProductionResourceLedgerV3,
    ),
) {
    data["page_masters"]["masters"][0]["body"] = json!({"x":body.x().raw(),"y":body.y().raw(),"width":body.width().get().raw(),"height":body.height().get().raw()});
    renumber(&mut data["document"], &mut 0);
    let root = Root::new();
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = match candidates {
        None => prepare_book_v2_text_flow(input.body().styled(), &nav),
        Some(values) => typaxis_syntax::book_v2::prepare_book_v2_text_flow_with_page_references(
            input.body().styled(),
            &nav,
            values,
        ),
    }
    .unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings =
        typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body,
        1_000_000,
        |stable| {
            let measured = prepare_book_v2_table_measurements(
                prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0)
                    .unwrap(),
                &limits,
            )
            .unwrap();
            check(&measured, &limits, input.resources());
        },
    )
    .unwrap();
}
fn empty_tables_data() -> Value {
    let mut data = table_data();
    let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
    let mut table = data["document"]["blocks"][0]["blocks"][0].clone();
    table["head"] = json!([]);
    for row in table["body"].as_array_mut().unwrap() {
        for cell in row["cells"].as_array_mut().unwrap() {
            cell["blocks"] = json!([]);
        }
    }
    data["document"]["blocks"][0]["blocks"] = json!([paragraph, table, table, paragraph, table]);
    data
}
#[test]
fn book_v2_mixed_source_advances_adjacent_empty_tables_without_losing_source_order() {
    with_measured(empty_tables_data(), |measured, limits| {
        let flow = measured.flow();
        assert_eq!(flow.body_items().len(), 2);
        assert_eq!(flow.table_count(), 3);
        for index in 0..3 {
            assert_eq!(flow.table_source_definition(index), Some(None));
            assert_eq!(flow.table_parent(index), Some(None));
            assert_eq!(measured.tables()[index].height(), Length::ZERO);
            let search =
                prepare_book_v2_table_search(measured, index, limits, 1_000_000, 0).unwrap();
            assert_eq!(search.maximum_height().raw(), 20_000_000);
        }
        assert_eq!(flow.table_source_definition(3), None);
        let run = |prior, work| -> Result<(u64, u64), ProductionBodyPaginationError> {
            let mut search = prepare_book_v2_table_body_search(measured, limits, work, prior)?;
            assert_eq!(search.body_table_count(), 3);
            assert_eq!(search.body_table_range(0), Some(1..1));
            assert_eq!(search.body_table_range(1), Some(1..1));
            assert_eq!(search.body_table_range(2), Some(2..2));
            let base = search.begin_body_source()?;
            assert!(!base.is_complete());
            let tables = [
                search.begin_table(0)?,
                search.begin_table(1)?,
                search.begin_table(2)?,
            ];
            assert!(
                matches!(search.evaluate_mixed_candidate(&base,&[Part::Items {end:2}]),Err(e) if e.kind==Error::ReceiptMismatch)
            );
            let first = search
                .evaluate_mixed_candidate(&base, &[Part::Items { end: 1 }])?
                .unwrap();
            assert_eq!(first.next_state().next_item(), 1);
            assert_eq!(first.next_state().next_table_index(), 0);
            assert!(
                matches!(search.evaluate_mixed_candidate(first.next_state(),&[Part::Table {cursor:tables[1],capacity:Length::ZERO}]),Err(e) if e.kind==Error::ReceiptMismatch)
            );
            let zero = search
                .evaluate_mixed_candidate(
                    first.next_state(),
                    &[Part::Table {
                        cursor: tables[0],
                        capacity: Length::ZERO,
                    }],
                )?
                .unwrap();
            assert_eq!(zero.used_height(), Length::ZERO);
            assert_eq!(zero.next_state().next_item(), 1);
            assert_eq!(zero.next_state().next_table_index(), 1);
            assert!(!zero.next_state().is_complete());
            assert!(
                matches!(search.evaluate_mixed_candidate(zero.next_state(),&[Part::Table {cursor:tables[0],capacity:Length::ZERO}]),Err(e) if e.kind==Error::ReceiptMismatch)
            );
            let rest = search
                .evaluate_mixed_candidate(
                    zero.next_state(),
                    &[
                        Part::Table {
                            cursor: tables[1],
                            capacity: Length::ZERO,
                        },
                        Part::Items { end: 2 },
                        Part::Table {
                            cursor: tables[2],
                            capacity: Length::ZERO,
                        },
                    ],
                )?
                .unwrap();
            assert!(rest.next_state().is_complete());
            assert_eq!(rest.next_state().next_table_index(), 3);
            assert_eq!(rest.next_state().next_item(), 2);
            let all = search
                .evaluate_mixed_candidate(
                    &base,
                    &[
                        Part::Items { end: 1 },
                        Part::Table {
                            cursor: tables[0],
                            capacity: Length::ZERO,
                        },
                        Part::Table {
                            cursor: tables[1],
                            capacity: Length::ZERO,
                        },
                        Part::Items { end: 2 },
                        Part::Table {
                            cursor: tables[2],
                            capacity: Length::ZERO,
                        },
                    ],
                )?
                .unwrap();
            all.verify(&base)?;
            assert_eq!(all.parts().len(), 5);
            assert_eq!(
                all.parts()
                    .iter()
                    .filter_map(|p| p.items())
                    .flatten()
                    .collect::<Vec<_>>(),
                [0, 1]
            );
            assert!(all.next_state().is_complete());
            assert_eq!(base.next_item(), 0);
            assert_eq!(base.next_table_index(), 0);
            let sibling = search.begin_body_source()?;
            assert!(matches!(all.verify(&sibling),Err(e) if e.kind==Error::ReceiptMismatch));
            Ok((search.record_charge(), search.work_steps()))
        };
        let (records, work) = run(0, 1_000_000).unwrap();
        let exact_prior = limits.base().get().max_fragments - (records - measured.record_charge());
        assert_eq!(
            run(exact_prior, work).unwrap(),
            (limits.base().get().max_fragments, work)
        );
        assert!(matches!(run(exact_prior+1,work),Err(e) if e.kind==Error::FragmentLimit));
        assert!(matches!(run(0,work-1),Err(e) if e.kind==Error::FootnoteSearchLimit));
    });
}
#[test]
fn book_v2_table_membership_uses_source_definition_and_parent_instead_of_leaf_position() {
    let mut data = empty_tables_data();
    let mut table = data["document"]["blocks"][0]["blocks"][1].clone();
    table["columns"] = json!([{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}]);
    data["document"]["footnotes"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(0, table.clone());
    with_measured(data, |measured, limits| {
        let flow = measured.flow();
        assert_eq!(flow.table_source_definition(2), Some(None));
        assert_eq!(flow.table_source_definition(3), Some(Some(0)));
        assert_eq!(
            prepare_book_v2_table_search(measured, 2, limits, 1_000_000, 0)
                .unwrap()
                .maximum_height()
                .raw(),
            20_000_000
        );
        assert_eq!(
            prepare_book_v2_table_search(measured, 3, limits, 1_000_000, 0)
                .unwrap()
                .maximum_height()
                .raw(),
            2_000_000
        );
        let mut search = prepare_book_v2_table_body_search(measured, limits, 1_000_000, 0).unwrap();
        assert_eq!(search.body_table_count(), 3);
        assert!(search.body_table_range(3).is_none());
        assert!(matches!(search.begin_table(3),Err(e) if e.kind==Error::ReceiptMismatch));
    });
    let mut data = empty_tables_data();
    data["document"]["blocks"][0]["blocks"][1]["body"][0]["cells"][0]["blocks"] =
        json!([table, table]);
    with_measured(data, |measured, _| {
        assert_eq!(measured.flow().table_parent(0), Some(None));
        assert_eq!(measured.flow().table_parent(1), Some(Some(0)));
        assert_eq!(measured.flow().table_source_definition(1), Some(None));
        assert_eq!(measured.flow().table_parent(2), Some(Some(0)));
        assert_eq!(measured.flow().table_parent(3), Some(None));
        let content = measured.tables()[0].cells()[0].content();
        assert_eq!(content.len(), 2);
        assert_eq!(
            content[0].source(),
            typaxis_pagination::ProductionTableContentSource::Table(1)
        );
        assert_eq!(
            content[1].source(),
            typaxis_pagination::ProductionTableContentSource::Table(2)
        );
        assert_eq!(
            measured.tables()[0].cells()[0].natural_height(),
            Length::ZERO
        );
    });
}
#[test]
fn book_v2_mixed_candidate_combines_body_table_and_notes_and_checks_exact_continuations() {
    let mut data = table_data();
    let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
    let span = paragraph["span"].clone();
    data["document"]["blocks"][0]["blocks"][0]["head"][0]["cells"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"note"}));
    data["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(0, paragraph);
    with_measured(data, |measured, limits| {
        let flow = measured.flow();
        let mut search = prepare_book_v2_table_body_search(measured, limits, 1_000_000, 0).unwrap();
        let base = search.begin_body_source().unwrap();
        let cursor = search.begin_table(0).unwrap();
        let table = &measured.tables()[0];
        let end = flow.body_items().len();
        let requests = [
            Part::Items { end: 1 },
            Part::Table {
                cursor,
                capacity: table.height(),
            },
            Part::Items { end },
        ];
        let all = search
            .evaluate_mixed_candidate(&base, &requests)
            .unwrap()
            .unwrap();
        assert_eq!(all.parts().len(), 3);
        assert!(all.next_state().is_complete());
        assert_eq!(all.next_state().demand().status(0), Some(Status::Complete));
        assert_eq!(
            all.next_state().demand().first_reference(0),
            Some(flow.references()[0].source().owner())
        );
        assert_eq!(all.footnotes().unwrap().fragments().len(), 1);
        assert!(all.footnotes().unwrap().fragments()[0]
            .fragment()
            .marker()
            .is_some());
        assert_eq!(
            all.parts()[1].top(),
            all.parts()[0]
                .height()
                .checked_add(flow.body_items()[0].space_after())
                .unwrap()
                .checked_add(table.space_before())
                .unwrap()
        );
        let short = table.rows()[0]
            .height()
            .checked_add(table.rows()[1].height())
            .unwrap();
        assert!(
            matches!(search.evaluate_mixed_candidate(&base,&[Part::Items {end:1},Part::Table {cursor,capacity:short},Part::Items {end}]),Err(e) if e.kind==Error::ReceiptMismatch)
        );
        let partial = search
            .evaluate_mixed_candidate(
                &base,
                &[
                    Part::Items { end: 1 },
                    Part::Table {
                        cursor,
                        capacity: short,
                    },
                ],
            )
            .unwrap()
            .unwrap();
        let continuation = partial.next_state().table_continuation().unwrap();
        assert_eq!(partial.next_state().next_item(), 1);
        assert_eq!(partial.next_state().next_table_index(), 0);
        assert!(
            matches!(search.evaluate_mixed_candidate(partial.next_state(),&[Part::Table {cursor,capacity:table.height()}]),Err(e) if e.kind==Error::ReceiptMismatch)
        );
        assert!(
            matches!(search.evaluate_mixed_candidate(&base,&[Part::Table {cursor:continuation,capacity:table.height()}]),Err(e) if e.kind==Error::ReceiptMismatch)
        );
        let tail = search
            .evaluate_mixed_candidate(
                partial.next_state(),
                &[
                    Part::Table {
                        cursor: continuation,
                        capacity: table.height(),
                    },
                    Part::Items { end },
                ],
            )
            .unwrap()
            .unwrap();
        assert!(tail.next_state().is_complete());
        assert!(tail.footnotes().is_none());
        assert_eq!(base.demand().status(0), Some(Status::Unreferenced));
        let mut other = prepare_book_v2_table_body_search(measured, limits, 1_000_000, 0).unwrap();
        let foreign = other.begin_body_source().unwrap();
        assert!(
            matches!(search.evaluate_mixed_candidate(&foreign,&requests),Err(e) if e.kind==Error::ReceiptMismatch)
        );
    });
}

#[path = "book_v2_mixed_page_tests.rs"]
mod pages;
