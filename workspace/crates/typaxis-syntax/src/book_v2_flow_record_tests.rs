use super::*;

// Independently count the constructed carriers; include the temporary ordinal
// registry and both generated registries rather than treating retained output
// length as a complete allocation history.
fn expected_records(flow: &PreparedBookV2TextFlow<'_>) -> u64 {
    1 + 2 * flow.footnote_definitions().len() as u64
        + flow.events().len() as u64
        + flow.paragraphs().len() as u64
        + flow
            .paragraphs()
            .iter()
            .map(|p| p.items().len() as u64)
            .sum::<u64>()
        + flow.lists().len() as u64
        + flow.list_items().len() as u64
        + flow.description_lists().len() as u64
        + flow.description_items().len() as u64
        + flow.figures().len() as u64
        + flow.named_page_breaks().len() as u64
        + flow.table_record_charge()
        + 2 * flow.generated.buffers().len() as u64
        + flow.page_reference_values().map_or(0, |v| v.len() as u64)
}

#[test]
fn successor_output_room_excludes_history_and_preserves_prepaid_graphs() {
    let mut budget = BookV2SourceVerificationBudget::new(7, 20);
    assert_eq!(budget.output_record_limit(100), Some(13));
    assert_eq!(budget.output_record_limit(10), Some(3));
    assert_eq!(budget.output_record_limit(6), None);
    budget.include_retained_records(9);
    assert_eq!(budget.output_record_limit(20), Some(13));
    assert_eq!(budget.output_record_limit(15), None);
    let mut prepaid = BookV2SourceVerificationBudget::new_with_prepaid_records(20, 25, 12);
    assert_eq!(prepaid.output_record_limit(100), Some(17));
    prepaid.include_retained_records(17);
    assert_eq!(prepaid.output_record_limit(25), Some(17));
    prepaid.include_retained_records(18);
    assert_eq!(prepaid.output_record_limit(25), None);
    let prepaid = BookV2SourceVerificationBudget::new_with_prepaid_records(7, 20, u64::MAX);
    assert_eq!(prepaid.output_record_limit(100), Some(20));
    let overflow = BookV2SourceVerificationBudget::new_with_prepaid_records(1, u64::MAX, 1);
    assert_eq!(overflow.output_record_limit(u64::MAX), Some(u64::MAX));
    let exhausted = BookV2SourceVerificationBudget::new(u64::MAX, u64::MAX);
    assert_eq!(exhausted.output_record_limit(u64::MAX), Some(0));
    assert_eq!(exhausted.output_record_limit(u64::MAX - 1), None);
}

#[test]
fn successor_source_records_bound_every_prefix_and_retain_all_carriers() {
    for case in ["combined", "pages", "text", "number", "named-break"] {
        let mut data = input();
        match case {
            "text" | "number" => {
                data["document"]["blocks"][1]["children"][1]["format"] = case.into();
                if case == "number" {
                    data["document"]["number_bindings"] = serde_json::json!([{
                        "anchor_id":"top", "owner_node_id":1, "label_node_id":2,
                        "text_span":{"text_id":0,"start_byte":0,"end_byte":5}
                    }]);
                }
            }
            "named-break" => {
                data["style_sheet"]["rules"][3]["declarations"][0]["value"] =
                    serde_json::json!({"kind":"string", "value":"basic-combined"})
            }
            _ => {}
        }
        let body = styled(&data, &limits());
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let values = [(NodeId::new(7), 12)];
        let values = (case == "pages").then_some(values.as_slice());
        let reference = prepare_inner(&body, &nav, values).unwrap();
        let charge = expected_records(&reference);
        assert_eq!(reference.source_record_charge(), charge, "{case}");
        let prior = 7;
        let mut last = prior;
        for available in 0..=charge {
            let maximum = prior + available;
            let mut observed = u64::MAX;
            let result = prepare_inner_counted(&body, &nav, values, prior, maximum, &mut observed);
            assert!(
                observed >= last && observed <= maximum,
                "{case}/{available}"
            );
            last = observed;
            if available == charge {
                let flow = result.unwrap();
                assert_eq!(observed, maximum);
                assert_eq!(flow.source_record_charge(), charge);
                assert_eq!(flow.fingerprint(), reference.fingerprint());
                flow.verify_for(&body, &nav).unwrap();
            } else {
                assert_eq!(
                    result.err().unwrap().kind,
                    ProductionFlowErrorKind::NodeLimit
                );
                let retry_prior = observed;
                assert!(prepare_inner_counted(
                    &body,
                    &nav,
                    values,
                    retry_prior,
                    maximum,
                    &mut observed
                )
                .is_err());
                assert!(observed >= retry_prior && observed <= maximum);
            }
        }
        let cap = body.body().limits().get().max_fragments;
        let mut observed = 0;
        let exact =
            prepare_inner_counted(&body, &nav, values, cap - charge, u64::MAX, &mut observed)
                .unwrap();
        assert_eq!(observed, cap);
        assert_eq!(exact.source_record_charge(), charge);
        assert!(prepare_inner_counted(
            &body,
            &nav,
            values,
            cap - charge + 1,
            u64::MAX,
            &mut observed
        )
        .is_err());
        assert!(observed <= cap);
        assert!(
            prepare_inner_counted(&body, &nav, values, u64::MAX, u64::MAX, &mut observed).is_err()
        );
        assert_eq!(observed, u64::MAX);
    }
}

fn two_paragraphs() -> Value {
    let mut data = input();
    let mut paragraph = data["document"]["blocks"][1].clone();
    paragraph["children"] =
        serde_json::json!([data["document"]["blocks"][0]["children"][0].clone()]);
    let mut second = paragraph.clone();
    paragraph["node_id"] = 1.into();
    paragraph["children"][0]["node_id"] = 2.into();
    second["node_id"] = 3.into();
    second["children"][0]["node_id"] = 4.into();
    second["classes"] = serde_json::json!(["missing"]);
    data["document"]["blocks"] = serde_json::json!([paragraph, second]);
    data["document"]["footnotes"] = serde_json::json!([]);
    data["outline"]["entries"] = serde_json::json!([]);
    data
}

#[test]
fn successor_source_records_keep_history_on_identity_and_late_style_failures() {
    let mut data = two_paragraphs();
    // Keep the style registry/extends targets valid while removing the font
    // family required by the later paragraph's actual text.
    for rule in data["style_sheet"]["rules"].as_array_mut().unwrap() {
        if rule["selector"] == "paragraph" {
            rule["declarations"]
                .as_array_mut()
                .unwrap()
                .retain(|d| d["name"] != "font_family");
        }
    }
    data["document"]["blocks"][0]["kind"] = "heading".into();
    data["document"]["blocks"][0]["level"] = 1.into();
    data["document"]["blocks"][0]["anchor_id"] = Value::Null;
    let body = styled(&data, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let mut observed = u64::MAX;
    let cause = prepare_book_v2_text_flow_counted(&body, &nav, 17, 1000, &mut observed)
        .err()
        .unwrap();
    assert_eq!(
        cause,
        ProductionFlowError {
            owner: NodeId::new(3),
            kind: ProductionFlowErrorKind::MissingTextStyle
        }
    );
    assert_eq!(observed, 17 + 9);
    let cause2 = prepare_book_v2_text_flow_counted(&body, &nav, observed, 1000, &mut observed)
        .err()
        .unwrap();
    assert_eq!(cause2, cause);
    assert_eq!(observed, 17 + 18);
    let other = styled(&data, &limits());
    assert_eq!(
        prepare_book_v2_text_flow_counted(&other, &nav, 29, 0, &mut observed)
            .err()
            .unwrap()
            .kind,
        ProductionFlowErrorKind::ReceiptMismatch
    );
    assert_eq!(observed, 29);
}

#[test]
fn successor_source_record_accounting_is_revalidated_independently_of_fingerprint() {
    let body = styled(&two_paragraphs(), &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let mut flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    assert_eq!(flow.source_record_charge(), 11);
    let original = flow.fingerprint();
    flow.source_record_charge -= 1;
    assert_eq!(flow.fingerprint(), original);
    assert_eq!(
        flow.verify_for(&body, &nav).unwrap_err().kind,
        ProductionFlowErrorKind::ReceiptMismatch
    );
}

#[test]
fn successor_source_verification_keeps_every_reconstruction_prefix_and_retry() {
    let body = styled(&input(), &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    for values in [None, Some([(NodeId::new(7), 12)].as_slice())] {
        let flow = prepare_inner(&body, &nav, values).unwrap();
        let charge = expected_records(&flow);
        let prior = 7;
        let mut last = prior;
        for available in 0..=charge {
            let maximum = prior + available;
            let mut observed = u64::MAX;
            let result = flow.verify_for_counted(&body, &nav, prior, maximum, &mut observed);
            assert!(observed >= last && observed <= maximum);
            last = observed;
            if available == charge {
                result.unwrap();
                assert_eq!(observed, maximum);
            } else {
                assert_eq!(result.unwrap_err().kind, ProductionFlowErrorKind::NodeLimit);
            }
        }
        let mut budget =
            crate::book_v2::BookV2SourceVerificationBudget::new(prior, prior + 2 * charge);
        budget.verify(&flow, &body, &nav).unwrap();
        assert_eq!(budget.record_charge(), prior + charge);
        budget.verify(&flow, &body, &nav).unwrap();
        assert_eq!(budget.record_charge(), prior + 2 * charge);
        assert_eq!(
            budget.verify(&flow, &body, &nav).unwrap_err().kind,
            ProductionFlowErrorKind::NodeLimit
        );
        assert_eq!(budget.record_charge(), prior + 2 * charge);
        let cap = body.body().limits().get().max_fragments;
        flow.verify_for_counted(&body, &nav, cap - charge, u64::MAX, &mut last)
            .unwrap();
        assert_eq!(last, cap);
        assert_eq!(
            flow.verify_for_counted(&body, &nav, cap - charge + 1, u64::MAX, &mut last)
                .unwrap_err()
                .kind,
            ProductionFlowErrorKind::NodeLimit
        );
        assert!(last <= cap);
        assert_eq!(
            flow.verify_for_counted(&body, &nav, u64::MAX, u64::MAX, &mut last)
                .unwrap_err()
                .kind,
            ProductionFlowErrorKind::NodeLimit
        );
        assert_eq!(last, u64::MAX);
    }
}

#[test]
fn successor_source_verification_reconstructs_instead_of_trusting_charge_or_hash() {
    let data = two_paragraphs();
    let body = styled(&data, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let other = styled(&data, &limits());
    let other_nav = prepare_book_v2_navigation(&other).unwrap();
    let mut flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let charge = expected_records(&flow);
    let fingerprint = flow.fingerprint();
    let mut observed = u64::MAX;
    for (source, navigation) in [(&other, &nav), (&body, &other_nav), (&other, &other_nav)] {
        assert_eq!(
            flow.verify_for_counted(source, navigation, 29, 0, &mut observed)
                .unwrap_err()
                .kind,
            ProductionFlowErrorKind::ReceiptMismatch
        );
        assert_eq!(observed, 29);
    }
    flow.source_record_charge = 0;
    assert_eq!(flow.fingerprint(), fingerprint);
    assert_eq!(
        flow.verify_for_counted(&body, &nav, 7, 7 + charge - 1, &mut observed)
            .unwrap_err()
            .kind,
        ProductionFlowErrorKind::NodeLimit
    );
    assert!(observed > 7 && observed < 7 + charge);
    for attempt in 1..=2 {
        let prior = observed;
        assert_eq!(
            flow.verify_for_counted(&body, &nav, prior, 1000, &mut observed)
                .unwrap_err()
                .kind,
            ProductionFlowErrorKind::ReceiptMismatch
        );
        assert_eq!(observed, prior + charge, "attempt {attempt}");
    }
}
