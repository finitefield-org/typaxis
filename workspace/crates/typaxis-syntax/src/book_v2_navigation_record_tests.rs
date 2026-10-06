use super::*;

// Inspect actual retained projections, independently of the source census.
// Full-length sort scratch and a worst-case intern slot per explicit spelling
// are deliberate reservations even when these particular inputs share strings.
fn expected(nav: &PreparedBookV2Navigation<'_>) -> u64 {
    use BookV2LanguageNodeKind as K;
    assert!(nav.number_bindings().is_empty());
    2 + nav.metadata().keywords.len() as u64
        + 3 * nav.languages().len() as u64
        + nav
            .languages()
            .iter()
            .filter(|v| v.explicit_language().is_some())
            .count() as u64
        + nav
            .languages()
            .iter()
            .filter(|v| {
                v.page_region().is_none() && matches!(v.kind(), K::Heading | K::SemanticContainer)
            })
            .count() as u64
        + nav
            .languages()
            .iter()
            .filter(|v| v.kind() == K::FootnoteDefinition)
            .count() as u64
        + nav.language_children().len() as u64
        + 2 * nav.anchors().len() as u64
        + 3 * nav.internal_links().len() as u64
        + 2 * nav.references().len() as u64
        + 4 * nav.outline().len() as u64
}

#[test]
fn navigation_record_prefixes_bound_all_registries_and_retain_retry_history() {
    let region =
        typaxis_document_package::staging_book_navigation_page_region_fixture(NAVIGATION).unwrap();
    let combined = root(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json")));
    let mut description = root(FIXTURE);
    let container = description["document"]["blocks"][0].clone();
    let mut term = container["blocks"][0].clone();
    term.as_object_mut().unwrap().remove("kind");
    description["document"]["blocks"] = json!([{
        "kind":"description_list","node_id":0,"span":container["span"],"classes":[],
        "items":[{"node_id":0,"span":container["span"],"term":term,
            "blocks":container["blocks"].as_array().unwrap()[1..]}]
    }]);
    // The domain's authored term precedes definitions in dense source preorder.
    fn description_ids(value: &mut Value, next: &mut u32) {
        if let Some(values) = value.as_array_mut() {
            for value in values {
                description_ids(value, next);
            }
        } else if let Some(object) = value.as_object_mut() {
            if let Some(id) = object.get_mut("node_id") {
                *id = (*next).into();
                *next += 1;
            }
            for name in ["term", "blocks", "children", "items", "footnotes"] {
                if let Some(value) = object.get_mut(name) {
                    description_ids(value, next);
                }
            }
        }
    }
    description_ids(&mut description["document"], &mut 0);
    for data in [
        root(FIXTURE),
        root(VECTOR),
        outline_input(),
        reference_input(),
        root(&region),
        combined,
        description,
    ] {
        let body = styled(&data, &limits());
        let reference = prepare_book_v2_navigation(&body).unwrap();
        let charge = expected(&reference);
        assert_eq!(reference.source_record_charge(), charge);
        let prior = 7;
        let mut previous = prior;
        for available in 0..=charge {
            let maximum = prior + available;
            let mut observed = u64::MAX;
            let result = prepare_book_v2_navigation_counted(&body, prior, maximum, &mut observed);
            assert!(observed >= previous && observed <= maximum);
            previous = observed;
            if available == charge {
                let nav = result.unwrap();
                assert_eq!(observed, maximum);
                assert_eq!(nav.source_record_charge(), charge);
                assert_eq!(nav.metadata(), reference.metadata());
                assert_eq!(nav.anchors(), reference.anchors());
                assert_eq!(nav.internal_links(), reference.internal_links());
                assert_eq!(nav.references(), reference.references());
                assert_eq!(nav.outline(), reference.outline());
                nav.verify_for(&body).unwrap();
                let other = styled(&data, &limits());
                assert_eq!(
                    nav.verify_for(&other).unwrap_err().kind(),
                    BookNavigationSyntaxErrorKind::ReceiptMismatch
                );
            } else {
                assert!(matches!(
                    result,
                    Err(BookV2NavigationPreparationError::RecordLimit { .. })
                ));
                let retry_prior = observed;
                assert!(prepare_book_v2_navigation_counted(
                    &body,
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
            prepare_book_v2_navigation_counted(&body, cap - charge, u64::MAX, &mut observed)
                .unwrap();
        assert_eq!(exact.source_record_charge(), charge);
        assert_eq!(observed, cap);
        assert!(prepare_book_v2_navigation_counted(
            &body,
            cap - charge + 1,
            u64::MAX,
            &mut observed
        )
        .is_err());
        assert!(observed <= cap);
        assert!(
            prepare_book_v2_navigation_counted(&body, u64::MAX, u64::MAX, &mut observed).is_err()
        );
        assert_eq!(observed, u64::MAX);
    }
}

#[test]
fn navigation_reservations_include_number_indexes_and_preserve_late_source_errors() {
    let mut data = outline_input();
    let base = styled(&data, &limits());
    let baseline = prepare_book_v2_navigation(&base)
        .unwrap()
        .source_record_charge();
    for anchor in ["group.root", "virtual.number"] {
        data["document"]["number_bindings"] = json!([{
            "anchor_id":anchor, "owner_node_id":1, "label_node_id":3,
            "text_span":{"text_id":0,"start_byte":0,"end_byte":5}
        }]);
        let body = styled(&data, &limits());
        let mut observed = 0;
        let nav = prepare_book_v2_navigation_counted(&body, 11, u64::MAX, &mut observed).unwrap();
        assert_eq!(nav.source_record_charge(), baseline + 10);
        assert_eq!(observed, 11 + baseline + 10);
        assert_eq!(nav.reference_number(anchor), Some("Resul"));
        assert_eq!(
            nav.anchors().len(),
            2 + usize::from(anchor == "virtual.number")
        );
    }
    data["document"]
        .as_object_mut()
        .unwrap()
        .remove("number_bindings");
    data["outline"]["entries"][1]["destination"] = "absent".into();
    let body = styled(&data, &limits());
    let mut observed = 0;
    for attempt in 1..=2 {
        let prior = observed;
        let error =
            prepare_book_v2_navigation_counted(&body, prior, u64::MAX, &mut observed).unwrap_err();
        let BookV2NavigationPreparationError::Syntax(error) = error else {
            panic!("expected source error")
        };
        assert_eq!(error.kind(), BookNavigationSyntaxErrorKind::InvalidOutline);
        assert_eq!(
            error.pointer().to_string(),
            "/outline/entries/1/destination"
        );
        assert_eq!(observed, attempt * baseline);
    }
}

#[test]
fn default_navigation_enforces_the_exact_body_record_ceiling() {
    let data = reference_input();
    let body = styled(&data, &limits());
    let charge = prepare_book_v2_navigation(&body)
        .unwrap()
        .source_record_charge();
    for short in [false, true] {
        let mut caps = ResourceLimits::default();
        caps.max_fragments = charge - u64::from(short);
        let body = styled(&data, &ValidatedResourceLimits::new(caps).unwrap());
        let result = prepare_book_v2_navigation(&body);
        if short {
            assert_eq!(
                result.unwrap_err().kind(),
                BookNavigationSyntaxErrorKind::NavigationRecordLimit
            );
        } else {
            assert_eq!(result.unwrap().source_record_charge(), charge);
        }
    }
}
