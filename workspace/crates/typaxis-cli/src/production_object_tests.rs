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
fn production_body_objects_keep_blank_page_entries_and_connect_link_annotations() {
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
        let objects = typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
        let navigation = objects.navigation();
        assert_eq!(navigation.destinations().len(), 1);
        assert_eq!(navigation.links().len(), 1);
        let link = &navigation.links()[0];
        let get = |role| objects.objects().iter().find(|o| o.role() == role).unwrap();
        assert!(
            production_object_references(get(R::StructureNode(link.node())))
                .contains(&R::LinkAnnotation(0))
        );
        assert_eq!(
            production_object_references(get(R::LinkAnnotation(0))),
            [R::Page(0)]
        );
        let root = String::from_utf8(production_object_bytes(get(R::StructureRoot))).unwrap();
        assert!(root.contains("/ParentTreeNextKey 2"));
        let annotation =
            String::from_utf8(production_object_bytes(get(R::LinkAnnotation(0)))).unwrap();
        assert!(annotation.contains("/StructParent 1"));
        assert!(annotation.contains("/Border [0 0 0]"));
        let pdf = typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
        assert!(pdf.object_number(R::Destinations).is_some());
        assert!(String::from_utf8_lossy(pdf.bytes()).contains("/Annots ["));
    });
}

#[test]
fn production_body_assembly_graph_and_opt_in_pdf_probes() {
    use typaxis_pdf::{ProductionBodyAssemblyRole as A, ProductionBodyObjectRole as R};
    let mut cases = vec![
        ("vmb-body", production_body_fixture(3_000_000)),
        (
            "vmb-formula-only",
            serde_json::from_slice(&production_inline_vmb_fixture(false)).unwrap(),
        ),
    ];
    let mut visible = production_body_fixture(3_000_000);
    for rule in visible["style_sheet"]["rules"].as_array_mut().unwrap() {
        if rule["selector"] == "paragraph" {
            rule["declarations"][0]["value"]["families"] =
                serde_json::json!(["Typaxis CFF Fixture"]);
        }
    }
    let mut spaced = visible.clone();
    let id = spaced["text_buffers"].as_array().unwrap().len();
    spaced["text_buffers"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "text_id":id, "utf8":" B", "mappings":[{"kind":"inserted",
            "source_span":null,"text_range":{"start_byte":0,"end_byte":2}}]
        }));
    spaced["document"]["blocks"][0]["blocks"][0]["children"][2]["text_span"] =
        serde_json::json!({"text_id":id,"start_byte":0,"end_byte":2});
    let mut doubled = spaced.clone();
    doubled["text_buffers"][0]["utf8"] = "A  ".into();
    doubled["text_buffers"][0]["mappings"][0]["text_range"]["end_byte"] = 3.into();
    doubled["document"]["blocks"][0]["blocks"][0]["children"][0]["text_span"]["end_byte"] =
        3.into();
    doubled["text_buffers"][id]["utf8"] = "  B".into();
    doubled["text_buffers"][id]["mappings"][0]["text_range"]["end_byte"] = 3.into();
    doubled["document"]["blocks"][0]["blocks"][0]["children"][2]["text_span"]["end_byte"] =
        3.into();
    cases.push(("vmb-body-visible-cff", visible));
    cases.push(("vmb-body-spaced-cff", spaced));
    cases.push(("vmb-body-double-spaced-cff", doubled));
    for (name, family) in [
        ("body-tt", "Body"),
        ("body-ttc", "Collection"),
        ("body-cff", "Typaxis CFF Fixture"),
    ] {
        cases.push((
            name,
            serde_json::from_slice(&production_text_single_paragraph(&["A B"], family)).unwrap(),
        ));
    }
    for (name, value) in cases {
        with_production_marked_body(&value, &config(), |marked, admitted, limits| {
            let objects =
                typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
            let pdf =
                typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
            pdf.verify(&objects, admitted, limits).unwrap();
            let anchor_count = marked
                .structure()
                .groups()
                .iter()
                .enumerate()
                .filter(|(i, _)| marked.structure().group_actual_text(*i).is_some())
                .count();
            assert_eq!(marked.anchors().len(), anchor_count);
            for anchor in marked.anchors() {
                let group = &marked.structure().groups()[anchor.group_index()];
                assert_eq!(anchor.page_index(), group.page_index());
                let typaxis_display_list::ProductionBodyDraw::Vector(vector) =
                    &marked.structure().display().draws()[group.draws().start]
                else {
                    panic!()
                };
                assert_eq!(anchor.viewport(), vector.viewport());
                assert_eq!(Some(anchor.baseline()), vector.baseline());
            }
            for page in marked.pages() {
                let content = std::str::from_utf8(page.content()).unwrap();
                let page_anchor_count = marked
                    .anchors()
                    .iter()
                    .filter(|a| a.page_index() == page.page_index())
                    .count();
                assert_eq!(content.matches("/PBA 1 Tf 3 Tr").count(), page_anchor_count);
                assert_eq!(content.matches("EMC\nQ\nEMC").count(), page_anchor_count);
                let resources = objects
                    .objects()
                    .iter()
                    .find(|o| o.role() == R::PageResources(page.page_index()))
                    .unwrap();
                assert_eq!(
                    production_object_references(resources).contains(&R::SemanticAnchorFont),
                    page_anchor_count > 0
                );
            }
            for role in [
                R::SemanticAnchorFont,
                R::SemanticAnchorGlyph,
                R::SemanticAnchorToUnicode,
            ] {
                assert_eq!(
                    objects
                        .objects()
                        .iter()
                        .filter(|o| o.role() == role)
                        .count(),
                    usize::from(anchor_count > 0)
                );
            }
            assert_eq!(pdf.content_hash(), sha256(pdf.bytes()));
            assert_eq!(pdf.page_count() as usize, marked.pages().len());
            assert_eq!(
                pdf.objects().len(),
                objects.objects().len() + marked.pages().len() + 4
            );
            assert!(pdf.bytes().starts_with(b"%PDF-1.7\n"));
            assert!(pdf.bytes().ends_with(b"%%EOF\n"));
            let mut expected_xref = b"0000000000 65535 f \n".to_vec();
            for (index, object) in pdf.objects().iter().enumerate() {
                assert_eq!(object.number(), index as u32 + 1);
                let header = format!("{} 0 obj\n", object.number());
                let at = object.offset() as usize;
                assert_eq!(&pdf.bytes()[at..at + header.len()], header.as_bytes());
                let begin = at + header.len();
                let end = begin + object.byte_length() as usize;
                assert_eq!(sha256(&pdf.bytes()[begin..end]), object.sha256());
                assert_eq!(&pdf.bytes()[end..end + 8], b"\nendobj\n");
                expected_xref
                    .extend_from_slice(format!("{:010} 00000 n \n", object.offset()).as_bytes());
                if let A::Body(role) = object.role() {
                    assert_eq!(pdf.object_number(role), Some(object.number()));
                }
                if let A::Body(R::Page(page)) = object.role() {
                    let bytes = std::str::from_utf8(&pdf.bytes()[begin..end]).unwrap();
                    assert!(bytes.contains(&format!("/StructParents {page} /Tabs /S")));
                    assert!(bytes.contains(&format!(
                        "/Contents {} 0 R",
                        pdf.object_number(R::PageContent(page)).unwrap()
                    )));
                    assert!(bytes.contains(&format!(
                        "/Resources {} 0 R",
                        pdf.object_number(R::PageResources(page)).unwrap()
                    )));
                }
            }
            assert!(pdf
                .bytes()
                .windows(expected_xref.len())
                .any(|w| w == expected_xref));
            assert!(!pdf
                .bytes()
                .windows(b"<pdfuaid:part>".len())
                .any(|w| w == b"<pdfuaid:part>"));
            let again =
                typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
            assert_eq!(pdf.bytes(), again.bytes());
            let other_objects =
                typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
            assert_eq!(
                pdf.verify(&other_objects, admitted, limits),
                Err(typaxis_pdf::ProductionBodyAssemblyError::ReceiptMismatch)
            );
            if let Some(root) = std::env::var_os("TYPAXIS_BODY_PDF_PROBE_DIR") {
                let root = PathBuf::from(root);
                fs::create_dir_all(&root).unwrap();
                fs::write(root.join(format!("{name}.pdf")), pdf.bytes()).unwrap();
                fs::write(
                    root.join(format!("{name}.package.json")),
                    serde_json::to_vec_pretty(&value).unwrap(),
                )
                .unwrap();
            }
        });
    }
}

#[test]
fn production_body_assembly_checks_complete_object_output_and_spool_budgets() {
    use typaxis_pdf::ProductionBodyAssemblyError as E;
    let value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A"], "Body")).unwrap();
    let (mut records, mut spool, mut count, mut output) = (0, 0, 0, 0);
    with_production_marked_body(&value, &config(), |marked, admitted, limits| {
        let objects = typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
        let pdf = typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
        records = pdf.record_charge();
        spool = pdf.spool_charge();
        count = pdf.objects().len() as u32;
        output = pdf.bytes().len() as u64;
    });
    for (max_fragments, max_spool_bytes, max_pdf_objects, max_output_bytes, error) in [
        (records, spool, count, output, None),
        (records - 1, spool, count, output, Some(E::RecordLimit)),
        (records, spool - 1, count, output, Some(E::SpoolLimit)),
        (records, spool, count - 1, output, Some(E::ObjectLimit)),
        (records, spool, count, output - 1, Some(E::OutputLimit)),
    ] {
        let config = config_with_limits(ResourceLimits {
            max_fragments,
            max_spool_bytes,
            max_pdf_objects,
            max_output_bytes,
            ..ResourceLimits::default()
        });
        with_production_marked_body(&value, &config, |marked, admitted, limits| {
            let objects =
                typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
            let result = typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits);
            match error {
                None => {
                    let pdf = result.unwrap();
                    assert_eq!(pdf.record_charge(), records);
                    assert_eq!(pdf.spool_charge(), spool);
                    assert_eq!(pdf.bytes().len() as u64, output);
                }
                Some(e) => assert_eq!(result.err(), Some(e)),
            }
        });
    }
}

fn production_body_navigation_vmb_fixture() -> serde_json::Value {
    use serde_json::json;
    let mut value = production_body_fixture(3_000_000);
    let root = &mut value["document"]["blocks"][0];
    root["anchor_id"] = "book".into();
    let first = &mut root["blocks"][0];
    first["kind"] = "heading".into();
    first["level"] = 1.into();
    first["anchor_id"] = "intro".into();
    let children = first["children"].clone();
    first["children"] = json!([{"kind":"link","node_id":0,"span":first["span"],
        "target":{"kind":"internal","anchor_id":"target"},"children":children}]);
    let mut last = root["blocks"][2].clone();
    let text = last["children"][0].clone();
    last["children"] =
        json!([{"kind":"anchor","node_id":0,"span":last["span"],"anchor_id":"target"},text]);
    root["blocks"][2] = json!({"kind":"semantic_container","semantic_kind":"result","node_id":0,
        "classes":[],"anchor_id":"ending","span":last["span"],"blocks":[last]});
    production_body_renumber(&mut value["document"], &mut 0);
    let root = &value["document"]["blocks"][0];
    value["outline"]["entries"] = json!([
        {"outline_id":0,"parent_outline_id":null,"level":1,"destination":"book","label":"Book","source_kind":"semantic_container","source_node_id":root["node_id"]},
        {"outline_id":1,"parent_outline_id":null,"level":1,"destination":"intro","label":"Introduction","source_kind":"heading","source_node_id":root["blocks"][0]["node_id"]},
        {"outline_id":2,"parent_outline_id":1,"level":2,"destination":"ending","label":"Ending","source_kind":"semantic_container","source_node_id":root["blocks"][2]["node_id"]}
    ]);
    value
}
fn production_body_navigation_multiline_fixture() -> serde_json::Value {
    use serde_json::json;
    let bytes = production_explicit_break_fixture(&["A", "hard_break", "B", "hard_break", "A"]);
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let paragraph = &mut value["document"]["blocks"][0]["blocks"][0];
    let children = paragraph["children"].clone();
    paragraph["children"] = json!([{"kind":"link","node_id":0,"span":paragraph["span"],
        "target":{"kind":"internal","anchor_id":"target"},"children":children}]);
    let span = paragraph["span"].clone();
    paragraph["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .insert(
            4,
            json!({
        "kind":"anchor","node_id":0,"span":span,"anchor_id":"target"}),
        );
    value["page_masters"]["masters"][0]["body"]["height"] = 1_000_000.into();
    value["page_masters"]["masters"][0]["footnote"] = serde_json::Value::Null;
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_body_navigation_resolves_real_vmb_and_multiline_links_to_selected_pages() {
    use typaxis_pdf::ProductionBodyObjectRole as R;
    for (name, value, expected_links, expected_pages) in [
        (
            "vmb-navigation",
            production_body_navigation_vmb_fixture(),
            1,
            2,
        ),
        (
            "multiline-navigation",
            production_body_navigation_multiline_fixture(),
            3,
            3,
        ),
    ] {
        with_production_marked_body(&value, &config(), |marked, admitted, limits| {
            let objects =
                typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
            let nav = objects.navigation();
            nav.verify(marked.structure()).unwrap();
            assert_eq!(nav.links().len(), expected_links);
            assert_eq!(marked.pages().len(), expected_pages);
            let target = nav
                .destinations()
                .iter()
                .find(|d| nav.destination_name(d.anchor_index()).unwrap().as_str() == "target")
                .unwrap();
            assert_eq!(target.page_index() as usize, expected_pages - 1);
            let selected = marked.structure().display().selected();
            let fragment = &selected.fragments()[target.fragment_index() as usize];
            assert_eq!(target.y(), fragment.baseline().unwrap());
            assert!(nav
                .links()
                .iter()
                .all(|l| l.destination_index() == Some(target.anchor_index())));
            if name == "vmb-navigation" {
                assert_eq!(nav.outline().len(), 3);
                assert_eq!(nav.outline_root().descendants(), 3);
                assert_eq!(
                    (
                        nav.outline()[1].first(),
                        nav.outline()[1].last(),
                        nav.outline()[1].descendants()
                    ),
                    (Some(2), Some(2), 1)
                );
                assert_eq!(nav.outline()[0].next(), Some(1));
                assert_eq!(nav.outline()[1].previous(), Some(0));
                for anchor in ["book", "intro", "ending"] {
                    let d = nav
                        .destinations()
                        .iter()
                        .find(|d| {
                            nav.destination_name(d.anchor_index()).unwrap().as_str() == anchor
                        })
                        .unwrap();
                    let f = &selected.fragments()[d.fragment_index() as usize];
                    assert_eq!((d.x(), d.y()), (f.bounds().x(), f.bounds().y()));
                    assert_eq!(d.page_index(), if anchor == "ending" { 1 } else { 0 });
                }
                let link = &nav.links()[0];
                // Real VMB formula ink viewport must be inside the clickable
                // box along with both surrounding body text runs.
                for draw in marked.structure().display().draws().iter().take(4) {
                    let rect = match draw {
                        typaxis_display_list::ProductionBodyDraw::Text(t) => {
                            t.logical_bounds().unwrap()
                        }
                        typaxis_display_list::ProductionBodyDraw::Vector(v) => v.viewport(),
                        typaxis_display_list::ProductionBodyDraw::SvgFigure(r) => r.viewport(),
                        typaxis_display_list::ProductionBodyDraw::Raster(r) => r.viewport(),
                        typaxis_display_list::ProductionBodyDraw::Math(m) => m.bounds(),
                    };
                    assert!(link.bounds().x() <= rect.x());
                    assert!(link.bounds().y() <= rect.y());
                    assert!(
                        link.bounds()
                            .x()
                            .checked_add(link.bounds().width().get())
                            .unwrap()
                            >= rect.x().checked_add(rect.width().get()).unwrap()
                    );
                    assert!(
                        link.bounds()
                            .y()
                            .checked_add(link.bounds().height().get())
                            .unwrap()
                            >= rect.y().checked_add(rect.height().get()).unwrap()
                    );
                }
            } else {
                assert_eq!(
                    nav.links()
                        .iter()
                        .map(|l| l.page_index())
                        .collect::<Vec<_>>(),
                    [0, 1, 2]
                );
                assert!(nav
                    .links()
                    .iter()
                    .all(|l| l.bounds().width().get().raw() == 471_859));
                assert!(nav
                    .links()
                    .iter()
                    .all(|l| l.bounds().height().get().raw() == 786_432));
            }
            let pdf =
                typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
            for index in 0..expected_links {
                assert!(pdf.object_number(R::LinkAnnotation(index as u32)).is_some());
            }
            // Export exact selected navigation receipts as an independent PDF
            // serialization oracle. Source/page/count assertions above prevent
            // accepting an arbitrary self-consistent replacement layout.
            if let Some(root) = std::env::var_os("TYPAXIS_NAVIGATION_PDF_PROBE_DIR") {
                let root = PathBuf::from(root);
                fs::create_dir_all(&root).unwrap();
                fs::write(root.join(format!("{name}.pdf")), pdf.bytes()).unwrap();
                let expected = serde_json::json!({
                    "name":name,"pages":expected_pages,"page_height_raw":selected.page_geometry().page_height().get().raw(),
                    "destinations":nav.destinations().iter().map(|d|serde_json::json!({"name":nav.destination_name(d.anchor_index()).unwrap().as_str(),"page":d.page_index(),"x_raw":d.x().raw(),"y_raw":d.y().raw()})).collect::<Vec<_>>(),
                    "links":nav.links().iter().enumerate().map(|(i,l)|serde_json::json!({"page":l.page_index(),"destination":nav.destination_name(l.destination_index().unwrap()).unwrap().as_str(),"rect_raw":[l.bounds().x().raw(),l.bounds().y().raw(),l.bounds().width().get().raw(),l.bounds().height().get().raw()],"contents":nav.structure().registry().node(l.node()).unwrap().accessible_name().unwrap(),"struct_parent":expected_pages+i,"structure_object":pdf.object_number(R::StructureNode(l.node())).unwrap()})).collect::<Vec<_>>(),
                    "outline":nav.outline_entries().iter().map(|e|serde_json::json!({"id":e.outline_id,"parent":e.parent_outline_id,"title":e.label,"destination":e.destination.as_str()})).collect::<Vec<_>>()
                });
                fs::write(
                    root.join(format!("{name}.expected.json")),
                    serde_json::to_vec_pretty(&expected).unwrap(),
                )
                .unwrap();
            }
        });
    }
}

#[test]
fn production_body_navigation_owner_and_combined_object_budget_are_enforced() {
    let value = production_body_navigation_multiline_fixture();
    let mut required = 0;
    with_production_marked_body(&value, &config(), |marked, admitted, limits| {
        let objects = typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
        required = objects.record_charge();
        assert!(objects.navigation().additional_records() > 0);
        assert_eq!(objects.navigation().record_base(), marked.record_charge());
        for (base, kind) in [
            (
                marked.structure().record_charge() - 1,
                typaxis_display_list::ProductionBodyNavigationErrorKind::ReceiptMismatch,
            ),
            (
                limits.base().get().max_fragments,
                typaxis_display_list::ProductionBodyNavigationErrorKind::RecordLimit,
            ),
        ] {
            assert_eq!(
                typaxis_display_list::build_production_body_navigation(
                    marked.structure(),
                    admitted,
                    limits,
                    base
                )
                .err()
                .unwrap()
                .kind,
                kind
            );
        }
        with_production_marked_body(&value, &config(), |other, _, _| {
            assert_eq!(
                objects
                    .navigation()
                    .verify(other.structure())
                    .unwrap_err()
                    .kind,
                typaxis_display_list::ProductionBodyNavigationErrorKind::ReceiptMismatch
            );
        });
    });
    for maximum in [required, required - 1] {
        let configured = config_with_limits(ResourceLimits {
            max_fragments: maximum,
            ..ResourceLimits::default()
        });
        with_production_marked_body(&value, &configured, |marked, admitted, limits| {
            let result = typaxis_pdf::build_production_body_objects(marked, admitted, limits);
            if maximum == required {
                assert_eq!(result.unwrap().record_charge(), required);
            } else {
                assert_eq!(
                    result.err(),
                    Some(typaxis_pdf::ProductionBodyObjectError::RecordLimit)
                );
            }
        });
    }
}

#[test]
fn production_body_navigation_preserves_external_uri_bytes_and_tag_owner() {
    let mut value = production_body_navigation_multiline_fixture();
    let link = &mut value["document"]["blocks"][0]["blocks"][0]["children"][0];
    let owner = link["node_id"].as_u64().unwrap() as u32;
    link["target"] = serde_json::json!({"kind":"uri","uri":"https://example.com/"});
    link["children"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c["kind"] != "anchor");
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_marked_body(&value, &config(), |marked, admitted, limits| {
        let objects = typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
        assert_eq!(objects.navigation().links().len(), 3);
        for (index, link) in objects.navigation().links().iter().enumerate() {
            assert_eq!(link.owner(), NodeId::new(owner));
            assert_eq!(
                link.target(),
                typaxis_display_list::ProductionBodyLinkTarget::Uri("https://example.com/")
            );
            assert_eq!(link.destination_index(), None);
            let annotation = objects
                .objects()
                .iter()
                .find(|o| {
                    o.role() == typaxis_pdf::ProductionBodyObjectRole::LinkAnnotation(index as u32)
                })
                .unwrap();
            let bytes = String::from_utf8(production_object_bytes(annotation)).unwrap();
            assert!(bytes.contains("/A << /S /URI /URI <68747470733A2F2F6578616D706C652E636F6D2F>"));
            assert!(!bytes.contains("/Dest"));
        }
        let pdf = typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
        assert!(objects.navigation().destinations().is_empty());
        assert!(pdf
            .object_number(typaxis_pdf::ProductionBodyObjectRole::Destinations)
            .is_none());
        if let Some(root) = std::env::var_os("TYPAXIS_NAVIGATION_PDF_PROBE_DIR") {
            let root = PathBuf::from(root);
            fs::create_dir_all(&root).unwrap();
            fs::write(root.join("external-navigation.pdf"), pdf.bytes()).unwrap();
            let nav = objects.navigation();
            let expected = serde_json::json!({"name":"external-navigation","pages":3,
                "page_height_raw":nav.structure().display().selected().page_geometry().page_height().get().raw(),
                "destinations":[],"outline":[],
                "links":nav.links().iter().enumerate().map(|(i,l)|serde_json::json!({"page":l.page_index(),"uri":"https://example.com/",
                    "rect_raw":[l.bounds().x().raw(),l.bounds().y().raw(),l.bounds().width().get().raw(),l.bounds().height().get().raw()],
                    "contents":nav.structure().registry().node(l.node()).unwrap().accessible_name().unwrap(),"struct_parent":3+i,"structure_object":pdf.object_number(typaxis_pdf::ProductionBodyObjectRole::StructureNode(l.node())).unwrap()})).collect::<Vec<_>>()});
            fs::write(
                root.join("external-navigation.expected.json"),
                serde_json::to_vec_pretty(&expected).unwrap(),
            )
            .unwrap();
        }
    });
}
