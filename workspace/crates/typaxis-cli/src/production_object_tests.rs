// The final PDF merger must resolve typed roles. These tests inspect object
// contributions without inventing a successful untagged publication writer.
fn with_production_marked_body(
    value: &serde_json::Value,
    config: &EffectiveConfig,
    check: impl FnOnce(
        &typaxis_pdf::ProductionBodyMarkedContent<'_, '_, '_, '_, '_, '_, '_>,
        &AdmittedResourceLedger,
        &typaxis_core::M4EffectiveResourceLimits,
    ),
) {
    with_production_body_structure_resources(
        value,
        config,
        |lines, blocks, limits, admitted, semantics, profile| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
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
            check(&marked, admitted, limits);
        },
    );
}
fn production_object_bytes(object: &typaxis_pdf::ProductionBodyObject) -> Vec<u8> {
    object
        .chunks()
        .iter()
        .filter_map(|c| match c {
            typaxis_pdf::ProductionBodyObjectChunk::Bytes(b) => Some(b.as_slice()),
            _ => None,
        })
        .flatten()
        .copied()
        .collect()
}
fn production_object_references(
    object: &typaxis_pdf::ProductionBodyObject,
) -> Vec<typaxis_pdf::ProductionBodyObjectRole> {
    object
        .chunks()
        .iter()
        .filter_map(|c| match c {
            typaxis_pdf::ProductionBodyObjectChunk::Reference(r) => Some(*r),
            _ => None,
        })
        .collect()
}

#[test]
fn production_body_objects_preserve_tt_ttc_cff_font_programs_and_unicode() {
    use typaxis_pdf::{ProductionBodyFontObjectPart as P, ProductionBodyObjectRole as R};
    for family in ["Body", "Collection", "Typaxis CFF Fixture"] {
        let value: serde_json::Value =
            serde_json::from_slice(&production_text_single_paragraph(&["A B"], family)).unwrap();
        with_production_marked_body(&value, &config(), |marked, admitted, limits| {
            let objects =
                typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
            objects.verify(marked, admitted, limits).unwrap();
            let fonts = marked.content().plans().fonts().fonts();
            assert_eq!(fonts.len(), 1);
            let font = fonts[0].pdf_font();
            let role = |part| R::Font {
                instance: font.font_instance_id(),
                part,
            };
            let get = |r| objects.objects().iter().find(|o| o.role() == r).unwrap();
            assert_eq!(
                production_object_references(get(role(P::Type0))),
                [role(P::CidFont), role(P::ToUnicode)]
            );
            let cid = String::from_utf8(production_object_bytes(get(role(P::CidFont)))).unwrap();
            let descriptor =
                String::from_utf8(production_object_bytes(get(role(P::Descriptor)))).unwrap();
            let refs = production_object_references(get(role(P::Descriptor)));
            if family == "Typaxis CFF Fixture" {
                assert!(cid.contains("/CIDFontType0") && !cid.contains("/CIDToGIDMap"));
                assert!(descriptor.contains("/FontFile3") && descriptor.contains("/CIDSet"));
                assert_eq!(refs, [role(P::Program), role(P::Auxiliary)]);
            } else {
                assert!(cid.contains("/CIDFontType2") && cid.contains("/CIDToGIDMap"));
                assert!(descriptor.contains("/FontFile2") && !descriptor.contains("/CIDSet"));
                assert_eq!(refs, [role(P::Program)]);
            }
            let program = production_object_bytes(get(role(P::Program)));
            assert!(program
                .windows(font.subset_bytes().len())
                .any(|w| w == font.subset_bytes()));
            let unicode =
                String::from_utf8(production_object_bytes(get(role(P::ToUnicode)))).unwrap();
            assert!(
                unicode.contains("<0041>")
                    && unicode.contains("<0042>")
                    && unicode.contains("<0020>")
            );
            assert_eq!(
                production_object_references(get(R::PageResources(0))),
                [role(P::Type0)]
            );
            let page = production_object_bytes(get(R::PageContent(0)));
            assert!(page
                .windows(marked.pages()[0].content().len())
                .any(|w| w == marked.pages()[0].content()));
            let again =
                typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
            assert_eq!(objects.objects(), again.objects());
        });
    }
}

#[test]
fn production_body_objects_link_real_vmb_forms_pages_and_structure() {
    use typaxis_pdf::ProductionBodyObjectRole as R;
    let value = production_body_fixture(3_000_000);
    with_production_marked_body(&value, &config(), |marked, admitted, limits| {
        let objects = typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
        let get = |r| objects.objects().iter().find(|o| o.role() == r).unwrap();
        let node = |source| {
            marked
                .structure()
                .registry()
                .source_node(typaxis_core::NodeId::new(source))
                .unwrap()
                .structure_node_id()
        };
        let parent_refs = production_object_references(get(R::ParentTree));
        assert_eq!(
            parent_refs,
            [3, 4, 5, 6, 8].map(|n| R::StructureNode(node(n)))
        );
        let parent = String::from_utf8(production_object_bytes(get(R::ParentTree))).unwrap();
        assert!(parent.starts_with("<< /Nums [0 [") && parent.contains("] 1 ["));
        for (owner, page, mcid) in [(3, 0, 0), (4, 0, 1), (5, 0, 2), (6, 0, 3), (8, 1, 0)] {
            let object = get(R::StructureNode(node(owner)));
            let bytes = String::from_utf8(production_object_bytes(object)).unwrap();
            assert!(bytes.contains(&format!("/MCID {mcid} >>")));
            assert!(production_object_references(object).contains(&R::Page(page)));
            assert!(!bytes.contains("/ActualText")); // not duplicated on StructElem
            if [4, 6].contains(&owner) {
                assert!(bytes.contains("/S /Formula") && bytes.contains("/Alt <FEFF"));
            }
        }
        for form in marked.content().vectors().forms() {
            let bytes = production_object_bytes(get(R::Vector(form.relative_object_role())));
            assert!(bytes
                .windows(form.content_stream().len())
                .any(|w| w == form.content_stream()));
            let as_text = std::str::from_utf8(&bytes).unwrap();
            assert!(!as_text.contains("/ActualText") && !as_text.contains("/MCID"));
        }
        let page0_refs = production_object_references(get(R::PageResources(0)));
        for resource in marked.content().vectors().pages()[0].resources() {
            assert!(page0_refs.contains(&R::Vector(resource.form_relative_object_role())));
        }
        assert!(!production_object_references(get(R::PageResources(1)))
            .iter()
            .any(|r| matches!(r, R::Vector(_))));
        // Only real page objects remain for the final document merger to supply.
        let roles = objects
            .objects()
            .iter()
            .map(|o| o.role())
            .collect::<std::collections::BTreeSet<_>>();
        for object in objects.objects() {
            for reference in production_object_references(object) {
                assert!(roles.contains(&reference) || matches!(reference, R::Page(0 | 1)));
            }
        }
        let other_marked = typaxis_pdf::build_production_body_marked_content(
            marked.content(),
            marked.structure(),
            admitted,
            limits,
        )
        .unwrap();
        assert_eq!(
            objects.verify(&other_marked, admitted, limits),
            Err(typaxis_pdf::ProductionBodyObjectError::ReceiptMismatch)
        );
    });
}

#[test]
fn production_body_objects_enforce_cumulative_budgets() {
    let value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A"], "Body")).unwrap();
    let (mut records, mut spool, mut count) = (0, 0, 0);
    with_production_marked_body(&value, &config(), |marked, admitted, limits| {
        let objects = typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
        records = objects.record_charge();
        spool = objects.spool_charge();
        count = objects.objects().len() as u32;
        assert!(records > marked.record_charge() && spool > marked.spool_charge());
    });
    use typaxis_pdf::ProductionBodyObjectError as E;
    for (max_fragments, max_spool_bytes, max_pdf_objects, expected) in [
        (records, spool, count, None),
        (records - 1, spool, count, Some(E::RecordLimit)),
        (records, spool - 1, count, Some(E::SpoolLimit)),
        (records, spool, count - 1, Some(E::ObjectLimit)),
    ] {
        let config = config_with_limits(ResourceLimits {
            max_fragments,
            max_spool_bytes,
            max_pdf_objects,
            ..ResourceLimits::default()
        });
        with_production_marked_body(&value, &config, |marked, admitted, limits| {
            let result = typaxis_pdf::build_production_body_objects(marked, admitted, limits);
            match expected {
                None => {
                    let objects = result.unwrap();
                    assert_eq!(objects.record_charge(), records);
                    assert_eq!(objects.spool_charge(), spool);
                }
                Some(e) => assert_eq!(result.err(), Some(e)),
            }
        });
    }
}

#[test]
fn production_body_objects_keep_blank_page_entries_and_refuse_missing_link_annotations() {
    use typaxis_pdf::ProductionBodyObjectRole as R;
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A"], "Body")).unwrap();
    let paragraph = value["document"]["blocks"][0]["blocks"][0].clone();
    let page_break = serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":{"source_id":0,"start_byte":0,"end_byte":0}});
    value["document"]["blocks"][0]["blocks"] =
        serde_json::json!([page_break, paragraph, page_break]);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_marked_body(&value, &config(), |marked, admitted, limits| {
        assert_eq!(marked.pages().len(), 3);
        let objects = typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
        let get = |role| objects.objects().iter().find(|o| o.role() == role).unwrap();
        let tree = String::from_utf8(production_object_bytes(get(R::ParentTree))).unwrap();
        assert!(tree.contains("0 [] 1 [") && tree.contains("2 []"));
        assert_eq!(production_object_references(get(R::ParentTree)).len(), 1);
        for page in [0, 2] {
            assert!(production_object_references(get(R::PageResources(page))).is_empty());
            let bytes =
                String::from_utf8(production_object_bytes(get(R::PageContent(page)))).unwrap();
            assert!(!bytes.contains("MCID") && !bytes.contains("Tj") && !bytes.contains("Do"));
        }
    });
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A"], "Body")).unwrap();
    let children = &mut value["document"]["blocks"][0]["blocks"][0]["children"];
    let text = children[0].clone();
    let span = text["span"].clone();
    *children = serde_json::json!([
        {"kind":"anchor","node_id":0,"span":span,"anchor_id":"target"},
        {"kind":"link","node_id":0,"span":span,"target":{"kind":"internal","anchor_id":"target"},"children":[text]}
    ]);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_marked_body(&value, &config(), |marked, admitted, limits| {
        assert!(marked
            .structure()
            .registry()
            .nodes()
            .iter()
            .any(|n| n.role() == typaxis_layout::StructureRole::Link));
        assert_eq!(
            typaxis_pdf::build_production_body_objects(marked, admitted, limits).err(),
            Some(typaxis_pdf::ProductionBodyObjectError::PendingNavigation)
        );
    });
}
