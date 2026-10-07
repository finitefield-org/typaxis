use super::*;

// Derive the old logical text charge from the retained public projections and
// exact borrowed source spellings, independently of admission in the builder.
fn charges(body: &StyledBookV2Body, nav: &PreparedBookV2Navigation<'_>) -> Vec<(u64, String)> {
    let metadata = nav.metadata();
    let mut values = Vec::new();
    for value in [
        &metadata.author,
        &metadata.created,
        &metadata.identifier,
        &metadata.modified,
        &metadata.subject,
        &metadata.title,
    ]
    .into_iter()
    .flatten()
    {
        values.push((value.len() as u64, "/metadata".into()));
    }
    for keyword in &metadata.keywords {
        values.push((keyword.len() as u64, "/metadata/keywords".into()));
    }
    let wire = body.body().wire();
    let mut sites = Vec::new();
    collect_document(
        wire.document(),
        wire.advanced_page_masters(),
        LanguageRegistryGeneration::V2,
        &mut sites,
        &mut BTreeMap::new(),
        &mut BTreeMap::new(),
    )
    .unwrap();
    for site in &sites {
        let record = nav.language(NodeId::new(site.node_id)).unwrap();
        let language = record.effective_language();
        let mut bytes = language.len() as u64;
        if let Some(raw) = site.raw.filter(|raw| *raw != language) {
            bytes += raw.len() as u64;
        }
        if let Some(prepaid) = record.vector().and_then(|v| v.language()) {
            bytes -= prepaid.charged_bytes();
        }
        values.push((bytes, site.pointer.clone()));
    }
    for (i, binding) in wire.document().number_bindings.iter().flatten().enumerate() {
        values.push((
            binding.anchor_id.len() as u64,
            format!("/document/number_bindings/{i}/anchor_id"),
        ));
    }
    for (i, entry) in nav.outline().iter().enumerate() {
        values.push((
            entry.label.len() as u64,
            format!("/outline/entries/{i}/label"),
        ));
    }
    values
}

#[test]
fn text_admission_preserves_every_exact_charge_pointer_and_failed_retry_reservation() {
    let region =
        typaxis_document_package::staging_book_navigation_page_region_fixture(NAVIGATION).unwrap();
    let combined = root(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json")));
    let mut numbered = root(FIXTURE);
    numbered["document"]["number_bindings"] = json!([{
        "anchor_id":"virtual.number", "owner_node_id":1, "label_node_id":3,
        "text_span":{"text_id":0,"start_byte":0,"end_byte":5}
    }]);
    for mut data in [
        root(FIXTURE),
        root(VECTOR),
        outline_input(),
        reference_input(),
        root(&region),
        combined,
        numbered,
    ] {
        data["document"]["language"] = "en-u-foo-a-bar".into();
        data["document"]["blocks"][0]["language"] = "JA-jp".into();
        let body = styled(&data, &limits());
        let reference = prepare_book_v2_navigation(&body).unwrap();
        let increments = charges(&body, &reference);
        let baseline = body.body().retained_text_bytes();
        let exact = baseline + increments.iter().map(|v| v.0).sum::<u64>();
        assert_eq!(reference.retained_text_bytes(), exact);
        let mut boundaries = BTreeSet::from([exact]);
        let mut prefix = baseline;
        for (bytes, _) in &increments {
            prefix += bytes;
            if *bytes != 0 {
                boundaries.extend([prefix - 1, prefix]);
            }
        }
        for maximum in boundaries.into_iter().filter(|n| *n >= baseline.max(32)) {
            let mut raw = ResourceLimits::default();
            raw.max_text_bytes = maximum;
            raw.max_text_buffer_bytes = maximum as u32;
            raw.max_shaping_context_bytes = maximum as u32;
            let limited = ValidatedResourceLimits::new(raw).unwrap();
            let body = styled(&data, &limited);
            let mut total = baseline;
            let rejected = increments.iter().find_map(|(bytes, pointer)| {
                total += bytes;
                (total > maximum).then_some(pointer)
            });
            let charge = reference.source_record_charge();
            let prior = 5;
            let mut observed = 0;
            let result =
                prepare_book_v2_navigation_counted(&body, prior, prior + 2 * charge, &mut observed);
            assert_eq!(observed, prior + charge);
            if let Some(pointer) = rejected {
                for result in [
                    result,
                    prepare_book_v2_navigation_counted(
                        &body,
                        observed,
                        prior + 2 * charge,
                        &mut observed,
                    ),
                ] {
                    let BookV2NavigationPreparationError::Syntax(error) = result.unwrap_err()
                    else {
                        panic!("wrong error");
                    };
                    assert_eq!(
                        error.kind(),
                        BookNavigationSyntaxErrorKind::TextAggregateLimit
                    );
                    assert_eq!(error.code(), "T2101");
                    assert_eq!(error.pointer().as_str(), pointer);
                }
                assert_eq!(observed, prior + 2 * charge);
            } else {
                let nav = result.unwrap();
                assert_eq!(nav.retained_text_bytes(), exact);
                assert_eq!(nav.metadata(), reference.metadata());
                assert_eq!(nav.outline(), reference.outline());
                assert_eq!(nav.anchors(), reference.anchors());
                assert_eq!(
                    nav.number_bindings().len(),
                    reference.number_bindings().len()
                );
                nav.verify_for(&body).unwrap();
            }
        }
    }
}

#[test]
fn language_collection_borrows_original_spelling_instead_of_copying_it() {
    let mut data = root(FIXTURE);
    data["document"]["language"] = "en-u-foo-a-bar".into();
    data["document"]["blocks"][0]["language"] = "JA-jp".into();
    let body = styled(&data, &limits());
    let wire = body.body().wire();
    let mut sites = Vec::new();
    collect_document(
        wire.document(),
        wire.advanced_page_masters(),
        LanguageRegistryGeneration::V2,
        &mut sites,
        &mut BTreeMap::new(),
        &mut BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(
        sites[0].raw.unwrap().as_ptr(),
        wire.document().language.as_ptr()
    );
    let typaxis_document_package::book_v2::WireBookV2Block::SemanticContainer { language, .. } =
        &wire.document().blocks[0]
    else {
        panic!("wrong fixture");
    };
    assert_eq!(
        sites[1].raw.unwrap().as_ptr(),
        language.as_ref().unwrap().as_ptr()
    );
}
