#[test]
fn production_body_terminals_bind_actual_block_placement_and_downstream_budgets() {
    let value = production_body_fixture(3_000_000);
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, semantics, profile, math| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let placement = selected.fingerprint();
            let before_records = selected.record_charge();
            assert!(selected.math_terminals().is_none());
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            let terminal = selected.math_terminals().unwrap();
            terminal.terminals().verify(math).unwrap();
            assert_eq!(terminal.placement_fingerprint(), placement);
            assert_ne!(selected.fingerprint(), placement);
            assert_eq!(selected.fingerprint(), terminal.fingerprint());
            assert_eq!(terminal.terminals().receipts().len(), 1);
            assert_eq!(
                terminal.terminals().receipts()[0].owner(),
                blocks.blocks()[0].owner()
            );
            assert_eq!(terminal.terminals().receipts()[0].terminal().get(), 1);
            assert_eq!(selected.record_charge(), before_records + 6);
            assert!(terminal.spool_charge() > 0);
            assert!(terminal.peak_spool_charge() >= terminal.spool_charge());
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
            assert!(fonts.spool_charge() >= terminal.spool_charge());
            let content =
                typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
            let structure = typaxis_display_list::build_production_body_structure(
                &display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .unwrap();
            let marked = typaxis_pdf::build_production_body_marked_content(
                &content, &structure, admitted, limits,
            )
            .unwrap();
            let objects =
                typaxis_pdf::build_production_body_objects(&marked, admitted, limits).unwrap();
            let pdf =
                typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
            assert!(pdf.bytes().starts_with(b"%PDF-"));
            drop(pdf);
            drop(objects);
            drop(marked);
            drop(structure);
            drop(content);
            drop(fonts);
            drop(display);
            assert_eq!(
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .err()
                    .unwrap()
                    .kind,
                typaxis_pagination::ProductionBodyPaginationErrorKind::ReceiptMismatch
            );
        },
    );
}

#[test]
fn production_body_terminals_preserve_forced_pages_without_double_consumption() {
    let mut value = production_body_fixture(3_000_000);
    let span = value["document"]["blocks"][0]["blocks"][1]["span"].clone();
    let zero = serde_json::json!({"source_id":0,"start_byte":span["start_byte"],"end_byte":span["start_byte"]});
    value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(
            1,
            serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":zero}),
        );
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, _, _, _, math| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            assert_eq!(selected.page_breaks().len(), 1);
            let block = selected
                .fragments()
                .iter()
                .find(|f| {
                    matches!(
                        f.source(),
                        typaxis_pagination::ProductionBodyFragmentSource::VectorBlock { .. }
                    )
                })
                .unwrap();
            assert!(block.page_index() >= 1);
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            assert_eq!(
                selected
                    .math_terminals()
                    .unwrap()
                    .terminals()
                    .receipts()
                    .len(),
                1
            );
        },
    );
    let empty: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A"], "Body")).unwrap();
    with_production_body_math_resources(
        &empty,
        &config(),
        |lines, blocks, limits, _, _, _, math| {
            assert!(math.flows().is_empty());
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            assert!(selected
                .math_terminals()
                .unwrap()
                .terminals()
                .receipts()
                .is_empty());
        },
    );
}

#[test]
fn production_body_terminals_reject_another_preparation_registry() {
    let first = production_body_fixture(3_000_000);
    let other = production_body_fixture(3_100_000);
    with_production_body_math_resources(&first, &config(), |lines, blocks, limits, _, _, _, _| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        with_production_body_math_resources(
            &other,
            &config(),
            |_, _, _, _, _, _, other_registry| {
                assert_eq!(
                    typaxis_pagination::finalize_production_body_math_terminals(
                        selected,
                        other_registry,
                        limits
                    )
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
fn production_body_terminals_obey_exact_cumulative_record_and_spool_limits() {
    let value = production_body_fixture(3_000_000);
    let (mut records, mut spool) = (0, 0);
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, _, _, _, math| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            records = selected.record_charge();
            spool = selected.math_terminals().unwrap().peak_spool_charge();
        },
    );
    for (record_limit, spool_limit, expected) in [
        (records, spool, None),
        (
            records - 1,
            spool,
            Some(typaxis_pagination::ProductionBodyPaginationErrorKind::FragmentLimit),
        ),
        (
            records,
            spool - 1,
            Some(typaxis_pagination::ProductionBodyPaginationErrorKind::SpoolLimit),
        ),
    ] {
        let config = config_with_limits(ResourceLimits {
            max_fragments: record_limit,
            max_spool_bytes: spool_limit,
            ..ResourceLimits::default()
        });
        with_production_body_math_resources(
            &value,
            &config,
            |lines, blocks, limits, _, _, _, math| {
                let selected =
                    typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
                let result = typaxis_pagination::finalize_production_body_math_terminals(
                    selected, math, limits,
                );
                assert_eq!(result.err().map(|e| e.kind), expected);
            },
        );
    }
}

#[test]
fn production_body_terminals_do_not_complete_unselected_equation_numbers() {
    let mut value = production_body_fixture(3_000_000);
    let end = value["sources"][0]["utf8_byte_length"].as_u64().unwrap();
    let span = serde_json::json!({"source_id":0,"start_byte":end,"end_byte":end+1});
    value["sources"][0]["utf8_byte_length"] = (end + 1).into();
    let tex = value["text_buffers"][3]["utf8"].as_str().unwrap();
    value["sources"][0]["sha256"] = sha256(format!("{tex}{tex}B").as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    value["text_buffers"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "text_id":4,"utf8":"B","mappings":[{"kind":"identity","source_span":span,
            "text_range":{"start_byte":0,"end_byte":1}}]
        }));
    value["document"]["blocks"][0]["span"]["end_byte"] = (end + 1).into();
    value["document"]["blocks"][0]["blocks"][1]["span"]["end_byte"] = (end + 1).into();
    let trailing = serde_json::json!({"source_id":0,"start_byte":end+1,"end_byte":end+1});
    value["document"]["blocks"][0]["blocks"][2]["span"] = trailing.clone();
    value["document"]["blocks"][0]["blocks"][2]["children"][0]["span"] = trailing;

    value["document"]["blocks"][0]["blocks"][1]["equation_number"] = serde_json::json!({
        "node_id":0,"span":span,
        "minimum_gap":65_536,"text_span":{"text_id":4,"start_byte":0,"end_byte":1}
    });
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, _, _, _, math| {
            assert!(blocks.blocks()[0].equation_number().is_some());
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            assert_eq!(
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .err()
                    .unwrap()
                    .kind,
                typaxis_pagination::ProductionBodyPaginationErrorKind::PendingEquationNumber
            );
        },
    );
}
