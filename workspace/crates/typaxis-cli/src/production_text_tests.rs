// Real host admission + syntax-owned body shaping; no synthetic math font.
fn production_text_fixture(
    bytes: &[u8],
    config: &EffectiveConfig,
) -> (
    typaxis_syntax::ValidatedStagingSemanticPackage,
    typaxis_syntax::ValidatedStagingBookNavigationV2,
    typaxis_core::M4EffectiveResourceLimits,
    AdmittedResourceLedger,
) {
    let job = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../samples/machine-package/profiles/production-book-1/combined/job");
    let decoded = wire::StagingSemanticDocumentPackageDecoder::new()
        .decode(
            bytes,
            &wire::DocumentPackageDecodePolicy::new(config.limits()),
        )
        .unwrap();
    let package = typaxis_syntax::StagingSemanticPackageParser::new()
        .parse(decoded, config.limits())
        .unwrap();
    let limits = typaxis_core::M4EffectiveResourceLimits::defaults_for(config.limits());
    let navigation =
        typaxis_syntax::validate_staging_book_navigation_v2(&package, &limits).unwrap();
    let semantics =
        typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits)
            .unwrap();
    let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
    let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
        &package,
        &navigation,
        &semantics,
        &limits,
        &identity,
    )
    .unwrap();
    let base = typaxis_resources::staging_declared_base_catalog(package.resources()).unwrap();
    let root = MachineFixtureRoot::new("production-body-text");
    let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    for resource in value["resources"]["font_faces"]
        .as_array()
        .unwrap()
        .iter()
        .chain(value["resources"]["images"].as_array().unwrap())
    {
        let uri = resource["uri"].as_str().unwrap();
        let source = match uri {
            "body-no-math.ttf" => job.join("../../../basic-document-1/combined/job/body.ttf"),
            "collection-no-math.ttc" => {
                job.join("../../../basic-document-1/combined/job/collection.ttc")
            }
            _ => job.join(uri),
        };
        let destination = root.path().join(uri);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(source, destination).unwrap();
    }
    fs::write(root.path().join("document-package.json"), bytes).unwrap();
    let context = HostAdmissionContext::new(
        HostPath::new(root.path().join("document-package.json")).unwrap(),
        HostPath::new(root.path().to_path_buf()).unwrap(),
        None,
        Vec::new(),
    );
    let session = HostResourceAdmissionSession::new(&context, config, &base).unwrap();
    let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
        &base,
        &limits,
        profile.base().base().authorization().profile_fingerprint(),
        session.roots(),
    )
    .unwrap();
    for declaration in &package.resources().font_faces {
        let pending = resolver
            .read_font(session.open_font(declaration.font_face_id).unwrap())
            .unwrap();
        resolver.parse_and_bind_declared_sfnt(pending).unwrap();
    }
    for declaration in &package.resources().images {
        let pending = resolver
            .read_image(session.open_image(declaration.image_id).unwrap())
            .unwrap();
        resolver.parse_and_bind_declared_image(pending).unwrap();
    }
    (package, navigation, limits, resolver.finish().unwrap())
}
const PRODUCTION_TEXT_COMBINED: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json"));

fn production_text_without_math_fonts() -> serde_json::Value {
    let mut value: serde_json::Value = serde_json::from_slice(PRODUCTION_TEXT_COMBINED).unwrap();
    for (index, name, hash) in [
        (
            0,
            "body-no-math.ttf",
            "c399cf1de56ffdc143b97547749a128b49b9356ec89a07a86ab0ca8107b3c50e",
        ),
        (
            1,
            "collection-no-math.ttc",
            "3a5f1d1bcbda54c4fa3543f0b886d6fa86cec51ed0328a22e10874426112259c",
        ),
    ] {
        value["resources"]["font_faces"][index]["uri"] = name.into();
        value["resources"]["font_faces"][index]["expected_sha256"] = hash.into();
    }
    value
}

#[test]
fn production_authored_text_shapes_real_body_ttc_and_keeps_pending_objects() {
    let bytes = serde_json::to_vec(&production_text_without_math_fonts()).unwrap();
    let (package, navigation, limits, admitted) = production_text_fixture(&bytes, &config());
    let flow =
        typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
    let epoch = sha256(b"production-body-text-test");
    let shaped = typaxis_shaping::shape_production_authored_text(
        &package,
        &navigation,
        &flow,
        &admitted,
        &limits,
        epoch,
    )
    .unwrap();
    assert_eq!(shaped.paragraphs().len(), 27);
    assert_eq!(shaped.paragraphs()[0].font().unwrap().face_id().get(), 1);
    assert_eq!(
        shaped.paragraphs()[0].font().unwrap().size().get().raw(),
        14 * 65_536
    );
    let body = shaped
        .paragraphs()
        .iter()
        .find(|p| p.owner().get() == 8)
        .unwrap();
    assert_eq!(body.font().unwrap().face_id().get(), 0);
    assert_eq!(body.font().unwrap().size().get().raw(), 12 * 65_536);
    assert_eq!(
        body.pending_references()
            .iter()
            .map(|n| n.get())
            .collect::<Vec<_>>(),
        [11]
    );
    assert!(shaped.paragraphs()[1].font().is_none());
    assert!(shaped.paragraphs()[1].runs().is_empty());
    assert_eq!(shaped.paragraphs()[1].pending_references()[0].get(), 7);
    for owner in [46, 78] {
        let atomic = shaped
            .paragraphs()
            .iter()
            .find(|p| p.owner().get() == owner)
            .unwrap();
        assert!(atomic.runs().is_empty());
        assert!(atomic.font().is_none());
    }
    for (paragraph, input) in shaped.paragraphs().iter().zip(flow.paragraphs()) {
        for run in paragraph.runs() {
            let site = &input.items()[run.site_index() as usize];
            assert_eq!(run.owner(), site.owner());
            assert_eq!(run.language(), site.language());
            assert!(!run.glyph_run().glyphs.is_empty());
            assert!(!run.glyph_run().clusters.is_empty());
            assert!(
                run.glyph_run()
                    .glyphs
                    .iter()
                    .all(|g| g.original_gid.get() != 0),
                "owner {:?}: {:?}",
                run.owner(),
                run.glyph_run()
            );
        }
    }
    shaped.verify(&flow, &admitted, &limits, epoch).unwrap();
    let second = typaxis_shaping::shape_production_authored_text(
        &package,
        &navigation,
        &flow,
        &admitted,
        &limits,
        epoch,
    )
    .unwrap();
    assert_eq!(shaped.fingerprint(), second.fingerprint());
    assert!(shaped
        .verify(&flow, &admitted, &limits, sha256(b"other-epoch"))
        .is_err());
    let other_flow =
        typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
    assert!(shaped
        .verify(&other_flow, &admitted, &limits, epoch)
        .is_err());
}

fn production_text_single_paragraph(texts: &[&str], family: &str) -> Vec<u8> {
    use serde_json::json;
    let mut value = production_text_without_math_fonts();
    let span = json!({"source_id":0,"start_byte":0,"end_byte":0});
    value["document"]["blocks"] = json!([{
        "kind":"semantic_container", "semantic_kind":"result", "node_id":1,
        "classes":[], "anchor_id":null, "span":span, "blocks":[{
        "kind":"paragraph", "node_id":2, "classes":[], "span":span,
        "children":texts.iter().enumerate().map(|(index, text)| json!({
            "kind":"text", "node_id":index + 3, "span":span,
            "text_span":{"text_id":index,"start_byte":0,"end_byte":text.len()}
        })).collect::<Vec<_>>()
    }]}]);
    value["document"]["footnotes"] = json!([]);
    value["outline"]["entries"] = json!([]);
    value["text_buffers"] = json!(texts
        .iter()
        .enumerate()
        .map(|(index, text)| json!({
            "text_id":index,"utf8":text,
            "mappings":[{"kind":"inserted","source_span":null,
                "text_range":{"start_byte":0,"end_byte":text.len()}}]
        }))
        .collect::<Vec<_>>());
    for rule in value["style_sheet"]["rules"].as_array_mut().unwrap() {
        if rule["selector"] == "paragraph" {
            rule["declarations"][0]["value"]["families"] = json!([family]);
        }
    }
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn production_authored_text_cff_without_native_math_uses_font_advances_and_source_clusters() {
    let bytes = production_text_single_paragraph(&["A ", "B"], "Typaxis CFF Fixture");
    let (package, navigation, limits, admitted) = production_text_fixture(&bytes, &config());
    let flow =
        typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
    let shaped = typaxis_shaping::shape_production_authored_text(
        &package,
        &navigation,
        &flow,
        &admitted,
        &limits,
        sha256(b"cff-body-text"),
    )
    .unwrap();
    let paragraph = &shaped.paragraphs()[0];
    let font = paragraph.font().unwrap();
    assert_eq!(font.face_id().get(), 2); // Body is declared first; selection must obey the style.
    assert_eq!(font.ascender().raw(), 629_146); // round-even(800 / 1000 * 12pt)
    assert_eq!(font.descender().raw(), -157_286);
    assert_eq!(font.line_gap().raw(), 0);
    assert!(paragraph.pending_references().is_empty());
    assert_eq!(paragraph.runs().len(), 2);
    let glyphs: Vec<_> = paragraph
        .runs()
        .iter()
        .flat_map(|r| &r.glyph_run().glyphs)
        .collect();
    assert_eq!(
        glyphs
            .iter()
            .map(|g| g.original_gid.get())
            .collect::<Vec<_>>(),
        [1, 3, 2]
    );
    assert_eq!(
        glyphs.iter().map(|g| g.advance_x.raw()).collect::<Vec<_>>(),
        [471_859, 235_930, 471_859]
    );
    let spans = paragraph
        .runs()
        .iter()
        .flat_map(|r| &r.glyph_run().clusters)
        .map(|c| {
            let typaxis_shaping::ShapeSourceSpan::Parsed(span) = c.source_span else {
                panic!("authored source")
            };
            (
                span.text_id().get(),
                span.start_byte().get(),
                span.end_byte().get(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(spans, [(0, 0, 1), (0, 1, 2), (1, 0, 1)]);
    assert_eq!(shaped.output_records(), 8); // 2 runs + 3 glyphs + 3 clusters.
}

#[test]
fn production_authored_text_rejects_notdef_from_conflicting_cmap_subtables() {
    let (package, navigation, limits, admitted) =
        production_text_fixture(PRODUCTION_TEXT_COMBINED, &config());
    let flow =
        typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
    let result = typaxis_shaping::shape_production_authored_text(
        &package,
        &navigation,
        &flow,
        &admitted,
        &limits,
        sha256(b"bad-cmap"),
    );
    let error = match result {
        Ok(_) => panic!("visible .notdef must fail"),
        Err(e) => e,
    };
    assert_eq!(error.owner.get(), 2);
    let typaxis_shaping::ProductionTextShapeErrorKind::MissingShapedGlyph { span } = error.kind
    else {
        panic!("{error}")
    };
    assert_eq!(
        (
            span.text_id().get(),
            span.start_byte().get(),
            span.end_byte().get()
        ),
        (0, 0, 1)
    );
}

#[test]
fn production_authored_text_rejects_split_grapheme_missing_font_and_document_output_limit() {
    use typaxis_shaping::{ItemizationError, ProductionTextShapeErrorKind as E};
    let cases = [
        (
            vec!["A", "\u{301}"],
            "Body",
            ResourceLimits::default(),
            E::Itemization(ItemizationError::SiteBoundarySplitsGrapheme),
        ),
        (
            vec!["A"],
            "Undeclared Body Font",
            ResourceLimits::default(),
            E::MissingSelectedFont,
        ),
        (
            vec!["𠮷"],
            "Body",
            ResourceLimits::default(),
            E::MissingDeclaredFontCoverage,
        ),
        (
            vec!["A B"],
            "Body",
            ResourceLimits {
                max_fragments: 6,
                ..ResourceLimits::default()
            },
            E::OutputLimit,
        ),
        (
            vec!["A B"],
            "Body",
            ResourceLimits {
                max_shaping_context_bytes: 16_383,
                ..ResourceLimits::default()
            },
            E::ContextLimit,
        ),
    ];
    for (texts, family, limits, expected) in cases {
        let bytes = production_text_single_paragraph(&texts, family);
        let (package, navigation, limits, admitted) =
            production_text_fixture(&bytes, &config_with_limits(limits));
        let flow =
            typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
        let result = typaxis_shaping::shape_production_authored_text(
            &package,
            &navigation,
            &flow,
            &admitted,
            &limits,
            sha256(b"body-text-negative"),
        );
        let error = match result {
            Ok(_) => panic!("expected {expected:?}"),
            Err(e) => e,
        };
        assert_eq!(error.kind, expected);
        assert!(error.owner.get() == 2 || error.owner.get() == 3);
    }
}

#[test]
fn production_authored_text_charges_output_across_paragraphs() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A"], "Body")).unwrap();
    let mut second = value["document"]["blocks"][0]["blocks"][0].clone();
    second["node_id"] = 4.into();
    second["children"][0]["node_id"] = 5.into();
    value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let bytes = serde_json::to_vec(&value).unwrap();
    for maximum in [5, 6] {
        let config = config_with_limits(ResourceLimits {
            max_fragments: maximum,
            ..ResourceLimits::default()
        });
        let (package, navigation, limits, admitted) = production_text_fixture(&bytes, &config);
        let flow =
            typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
        let result = typaxis_shaping::shape_production_authored_text(
            &package,
            &navigation,
            &flow,
            &admitted,
            &limits,
            sha256(b"aggregate-body-shape"),
        );
        match (maximum, result) {
            (5, Err(error)) => {
                assert_eq!(error.owner.get(), 5);
                assert_eq!(
                    error.kind,
                    typaxis_shaping::ProductionTextShapeErrorKind::OutputLimit
                );
            }
            (6, Ok(shaped)) => assert_eq!(shaped.output_records(), 6),
            _ => panic!("document output budget was not exact"),
        }
    }
}
