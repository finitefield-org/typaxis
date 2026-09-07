// Synthetic cardinality corpus derived from the unchanged engine outline.
// Each fixed fill produces a distinct IR and PDF Form, not just different XML.
fn production_distinct_vmb_svg(index: u32) -> Vec<u8> {
    let svg = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/vmb-book/engine-v2/fraction-inline-720896.svg"));
    assert!(index < 0x1000000);
    assert_eq!(svg.matches("fill=\"currentColor\"").count(), 1);
    svg.replacen(
        "fill=\"currentColor\"",
        &format!("fill=\"#{index:06x}\""),
        1,
    )
    .into_bytes()
}

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
    let limits = config.m4_limits().cloned().unwrap_or_else(|| typaxis_core::M4EffectiveResourceLimits::defaults_for(config.limits()));
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
    let mut copied_paths = std::collections::BTreeSet::new();
    for resource in value["resources"]["font_faces"]
        .as_array()
        .unwrap()
        .iter()
        .chain(value["resources"]["images"].as_array().unwrap())
    {
        let uri = resource["uri"].as_str().unwrap();
        // Alias declarations share a staged file; admission still opens and
        // validates every declaration below, including all 5,000 aliases.
        if !copied_paths.insert(uri) {
            continue;
        }
        if let Some(index) = uri.strip_prefix("vmb-distinct-").and_then(|s| s.strip_suffix(".svg")) {
            let index: u32 = index.parse().unwrap();
            fs::write(root.path().join(uri), production_distinct_vmb_svg(index)).unwrap();
            continue;
        }
        let source = match uri {
            "body-list-no-math.ttf" | "collection-list-no-math.ttc" | "body-list-visible.ttf" => job.join("../../../../staging/production-book-1/vmb-book/list-fonts").join(uri),
            "book-venn.png" | "orientation-alpha.png" => job.join("../../../../staging/production-book-1/vmb-book/raster").join(uri),
            "vmb-block-fraction.svg" => job.join("../../../../staging/production-book-1/vmb-book/engine-v2/fraction-block-720896.svg"),
            "body-context.ttf" => job.join("../../../../staging/production-book-1/vmb-book/reshape/context.ttf"),
            "vmb-fraction.svg" => job.join("../../../../staging/production-book-1/vmb-book/engine-v2/fraction-inline-720896.svg"),
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
        profile
            .base()
            .base()
            .authorization()
            .profile_receipt_fingerprint(),
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

fn with_prepared_production_inlines(
    bytes: &[u8],
    check: impl FnOnce(&typaxis_layout::ProductionPreparedInlines<'_>),
) {
    with_prepared_production_inlines_config(bytes, &config(), check);
}

fn with_prepared_production_inlines_config(
    bytes: &[u8],
    config: &EffectiveConfig,
    check: impl FnOnce(&typaxis_layout::ProductionPreparedInlines<'_>),
) {
    with_production_inline_context(bytes, config, |prepared, _, _, _, _, _| check(prepared));
}

fn with_production_inline_context(
    bytes: &[u8],
    config: &EffectiveConfig,
    check: impl FnOnce(
        &typaxis_layout::ProductionPreparedInlines<'_>,
        &typaxis_syntax::ValidatedStagingSemanticPackage,
        &typaxis_syntax::StagingPrecomposedVectorProfileAuthorization,
        &typaxis_core::M4EffectiveResourceLimits,
        &AdmittedResourceLedger,
        &typaxis_layout::ValidatedPrecomposedVectorBindings,
    ),
) {
    with_production_inline_tagged_context(
        bytes,
        config,
        |prepared, package, profile, limits, admitted, bindings, _, _| {
            check(prepared, package, profile, limits, admitted, bindings)
        },
    );
}

fn with_production_inline_tagged_context(
    bytes: &[u8],
    config: &EffectiveConfig,
    check: impl FnOnce(
        &typaxis_layout::ProductionPreparedInlines<'_>,
        &typaxis_syntax::ValidatedStagingSemanticPackage,
        &typaxis_syntax::StagingPrecomposedVectorProfileAuthorization,
        &typaxis_core::M4EffectiveResourceLimits,
        &AdmittedResourceLedger,
        &typaxis_layout::ValidatedPrecomposedVectorBindings,
        &typaxis_syntax::ValidatedStagingStructureSemanticsV2,
        &typaxis_machine_profile::StagingTaggedPdfProfileReceiptV2,
    ),
) {
    let (package, navigation, limits, admitted) = production_text_fixture(bytes, config);
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
    let bindings = typaxis_layout::bind_staging_precomposed_vectors(
        &package,
        profile.base().base().authorization(),
        &limits,
        &admitted,
    )
    .unwrap();
    let flow =
        typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
    let shaped = typaxis_shaping::shape_production_authored_text(
        &package,
        &navigation,
        &flow,
        &admitted,
        &limits,
        bindings.epoch().fingerprint(),
    )
    .unwrap();
    let prepared = typaxis_layout::prepare_production_inline_items(
        &package,
        &navigation,
        profile.base().base().authorization(),
        &limits,
        &admitted,
        &flow,
        &shaped,
        &bindings,
        typaxis_linebreak::JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    prepared.verify(&flow, &shaped, &bindings).unwrap();
    check(
        &prepared,
        &package,
        profile.base().base().authorization(),
        &limits,
        &admitted,
        &bindings,
        &semantics,
        &profile,
    );
}

fn production_inline_vmb_fixture(surrounding_text: bool) -> Vec<u8> {
    use serde_json::json;
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A ", "B"], "Body")).unwrap();
    let index: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/vmb-book/engine-v2/fixture-index.json"))).unwrap();
    let case = &index["cases"][0];
    assert_eq!(case["derived_svg"], "fraction-inline-720896.svg");
    value["resources"]["images"][2]["uri"] = "vmb-fraction.svg".into();
    value["resources"]["images"][2]["expected_sha256"] = case["derived_sha256"].clone();
    value["resources"]["images"][2]["vector_provenance"] = json!({
        "engine_id":case["engine_artifact"]["EngineID"],
        "engine_version":case["engine_artifact"]["EngineVersion"],
        "rules_version":index["rules_version"]
    });
    let tex_len = case["tex"].as_str().unwrap().len();
    let source_span = json!({"source_id":0,"start_byte":0,"end_byte":tex_len});
    value["sources"][0]["utf8_byte_length"] = tex_len.into();
    value["sources"][0]["sha256"] = sha256(case["tex"].as_str().unwrap().as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    value["text_buffers"].as_array_mut().unwrap().push(json!({
        "text_id":2,"utf8":case["tex"],
        "mappings":[{"kind":"identity","source_span":source_span,
            "text_range":{"start_byte":0,"end_byte":tex_len}}]
    }));
    let formula = json!({"kind":"math_vector", "node_id":if surrounding_text {4} else {3},
        "span":source_span, "image_id":2,
        "metrics":case["metrics"], "spacing":{"before":0,"after":0},
        "source_tex":{"text_span":{"text_id":2,"start_byte":0,"end_byte":tex_len}},
        "alt":case["speech"], "actual_text":null});
    let children = value["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap();
    if surrounding_text {
        children[1]["node_id"] = 5.into();
        children[1]["span"] = json!({"source_id":0,"start_byte":tex_len,"end_byte":tex_len});
        children.insert(1, formula);
    } else {
        *children = vec![formula];
    }
    value["document"]["blocks"][0]["span"] = source_span.clone();
    value["document"]["blocks"][0]["blocks"][0]["span"] = source_span;
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn production_line_projection_keeps_real_vmb_body_glyphs_and_formula_on_one_baseline() {
    use typaxis_layout::ProductionPlacedInline as P;
    let width = PositiveLength::new(Length::from_raw(3_000_000).unwrap()).unwrap();
    with_prepared_production_inlines(&production_inline_vmb_fixture(true), |prepared| {
        let layout =
            typaxis_layout::layout_production_inline_lines(prepared, &[width], 100).unwrap();
        layout.verify(prepared).unwrap();
        assert_eq!(layout.output_records(), 15);
        let p = &layout.paragraphs()[0];
        assert_eq!(p.font().unwrap().face_id().get(), 0);
        assert_eq!(p.lines().len(), 1);
        let line = &p.lines()[0];
        assert_eq!(line.baseline().raw(), 665_170);
        assert_eq!(line.items().len(), 4);
        let mut text = String::new();
        for (index, (item, expected_pen)) in line
            .items()
            .iter()
            .zip([0, 471_859, 707_789, 2_173_770])
            .enumerate()
        {
            match item {
                P::Text(cluster) => {
                    text.push_str(cluster.utf8());
                    assert_eq!(cluster.pen_x().raw(), expected_pen);
                    assert_eq!(cluster.glyphs().len(), 1);
                    let glyph = &cluster.glyphs()[0];
                    assert_eq!(glyph.x().raw(), expected_pen);
                    assert_eq!(glyph.y(), line.baseline());
                    assert!(std::ptr::eq(
                        glyph.glyph(),
                        &cluster.run().glyph_run().glyphs[glyph.glyph_index() as usize]
                    ));
                    assert_eq!(
                        cluster.source_span().text_id().get(),
                        if index == 3 { 1 } else { 0 }
                    );
                }
                P::Vector(vector) => {
                    assert_eq!(index, 2);
                    let g = vector.geometry();
                    assert_eq!(g.pen_origin_x().raw(), expected_pen);
                    assert_eq!(g.line_baseline_y(), line.baseline());
                    assert_eq!(g.viewport().x().raw(), 662_733);
                    assert_eq!(g.viewport().y().raw(), 0);
                    assert_eq!(g.viewport().width().get().raw(), 1_556_093);
                    assert_eq!(vector.occurrence().item().node_id().get(), 4);
                }
                P::Break(_) => panic!("no break node in fixture"),
            }
        }
        assert_eq!(text, "A B");
        let again =
            typaxis_layout::layout_production_inline_lines(prepared, &[width], 100).unwrap();
        assert_eq!(layout.fingerprint(), again.fingerprint());
        let wider = PositiveLength::new(Length::from_raw(3_000_001).unwrap()).unwrap();
        assert_ne!(
            layout.fingerprint(),
            typaxis_layout::layout_production_inline_lines(prepared, &[wider], 100)
                .unwrap()
                .fingerprint()
        );
        with_prepared_production_inlines(&production_inline_vmb_fixture(true), |other| {
            assert!(layout.verify(other).is_err());
        });
    });
}

#[test]
fn production_line_projection_shifts_formula_and_following_glyph_together() {
    use typaxis_layout::ProductionPlacedInline as P;
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_inline_vmb_fixture(true)).unwrap();
    let children = value["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap();
    children.remove(0);
    children[0]["node_id"] = 3.into();
    children[1]["node_id"] = 4.into();
    with_prepared_production_inlines(&serde_json::to_vec(&value).unwrap(), |prepared| {
        let width = PositiveLength::new(Length::from_raw(1_982_896).unwrap()).unwrap();
        let layout =
            typaxis_layout::layout_production_inline_lines(prepared, &[width], 100).unwrap();
        let p = &layout.paragraphs()[0];
        assert_eq!(
            p.selected().unwrap().lines()[0].origin_shift().get().raw(),
            45_056
        );
        let line = &p.lines()[0];
        let P::Vector(vector) = line.items()[0] else {
            panic!("vector first")
        };
        let P::Text(ref cluster) = line.items()[1] else {
            panic!("body second")
        };
        assert_eq!(vector.geometry().viewport().x().raw(), 0);
        assert_eq!(vector.geometry().pen_origin_x().raw(), 45_056);
        assert_eq!(cluster.pen_x().raw(), 1_511_037);
        assert_eq!(cluster.glyphs()[0].x().raw(), 1_511_037);
        assert_eq!(cluster.glyphs()[0].y(), vector.geometry().line_baseline_y());
        assert_eq!(cluster.utf8(), "B");
    });
}

#[test]
fn production_line_projection_applies_svg_spacing_only_before_same_line_body() {
    use typaxis_layout::ProductionPlacedInline as P;
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_inline_vmb_fixture(true)).unwrap();
    let children = value["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap();
    children.remove(0);
    children[0]["node_id"] = 3.into();
    children[0]["spacing"]["after"] = 200.into();
    children[1]["node_id"] = 5.into();
    let span = children[1]["span"].clone();
    children.insert(
        1,
        serde_json::json!({"kind":"soft_break","node_id":4,"span":span}),
    );
    with_prepared_production_inlines(&serde_json::to_vec(&value).unwrap(), |prepared| {
        for (raw_width, count) in [(2_000_000, 1), (1_556_093, 2)] {
            let width = PositiveLength::new(Length::from_raw(raw_width).unwrap()).unwrap();
            let layout =
                typaxis_layout::layout_production_inline_lines(prepared, &[width], 100).unwrap();
            let p = &layout.paragraphs()[0];
            assert_eq!(p.lines().len(), count);
            let P::Vector(vector) = p.lines()[0].items()[0] else {
                panic!("first formula")
            };
            assert_eq!(
                vector.occurrence().spacing_after().get().raw(),
                if count == 1 { 200 } else { 0 }
            );
            let cluster = p
                .lines()
                .iter()
                .flat_map(|line| line.items())
                .find_map(|item| match item {
                    P::Text(c) => Some(c),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                cluster.glyphs()[0].x().raw(),
                if count == 1 { 1_511_237 } else { 0 }
            );
            assert_eq!(cluster.utf8(), "B");
        }
    });
}

#[test]
fn production_line_projection_keeps_cff_body_font_and_needs_no_font_for_formula_only() {
    use typaxis_layout::ProductionPlacedInline as P;
    with_prepared_production_inlines(
        &production_text_single_paragraph(&["A B"], "Typaxis CFF Fixture"),
        |prepared| {
            let width = PositiveLength::new(Length::from_raw(1_179_648).unwrap()).unwrap();
            let layout =
                typaxis_layout::layout_production_inline_lines(prepared, &[width], 100).unwrap();
            let p = &layout.paragraphs()[0];
            assert_eq!(p.font().unwrap().face_id().get(), 2);
            let mut projected = Vec::new();
            for item in p.lines()[0].items() {
                let P::Text(cluster) = item else {
                    panic!("body text")
                };
                projected.push((
                    cluster.utf8(),
                    cluster.glyphs()[0].glyph().original_gid.get(),
                    cluster.pen_x().raw(),
                ));
            }
            assert_eq!(
                projected,
                [("A", 1, 0), (" ", 3, 471_859), ("B", 2, 707_789)]
            );
        },
    );
    with_prepared_production_inlines(&production_inline_vmb_fixture(false), |prepared| {
        let width = PositiveLength::new(Length::from_raw(1_556_093).unwrap()).unwrap();
        let layout =
            typaxis_layout::layout_production_inline_lines(prepared, &[width], 100).unwrap();
        let p = &layout.paragraphs()[0];
        assert!(p.font().is_none());
        assert_eq!(p.lines()[0].items().len(), 1);
        let P::Vector(vector) = p.lines()[0].items()[0] else {
            panic!("formula only")
        };
        assert_eq!(vector.geometry().viewport().x(), Length::ZERO);
        assert_eq!(vector.geometry().viewport().y(), Length::ZERO);
    });
}

#[test]
fn production_line_projection_preserves_source_clusters_across_explicit_breaks() {
    use typaxis_layout::ProductionPlacedInline as P;
    for kind in ["soft_break", "hard_break"] {
        with_prepared_production_inlines(
            &production_explicit_break_fixture(&["A", kind, "B"]),
            |prepared| {
                let width = PositiveLength::new(Length::from_raw(471_859).unwrap()).unwrap();
                let layout =
                    typaxis_layout::layout_production_inline_lines(prepared, &[width], 100)
                        .unwrap();
                let p = &layout.paragraphs()[0];
                assert_eq!(p.lines().len(), 2);
                let mut text = String::new();
                let mut controls = 0;
                for line in p.lines() {
                    for item in line.items() {
                        match item {
                            P::Text(cluster) => {
                                text.push_str(cluster.utf8());
                                assert_eq!(cluster.pen_x(), Length::ZERO);
                                assert_eq!(cluster.glyphs()[0].x(), Length::ZERO);
                                assert_eq!(cluster.glyphs()[0].y(), line.baseline());
                            }
                            P::Break(control) => {
                                controls += 1;
                                assert_eq!(control.owner().get(), 4);
                            }
                            _ => panic!("unexpected image"),
                        }
                    }
                }
                assert_eq!(text, "AB");
                assert_eq!(controls, 1);
            },
        );
    }
}

#[test]
fn production_line_projection_requires_complete_paragraph_widths_and_document_budgets() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["AB"], "Body")).unwrap();
    let mut second = value["document"]["blocks"][0]["blocks"][0].clone();
    second["node_id"] = 4.into();
    second["children"][0]["node_id"] = 5.into();
    value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let bytes = serde_json::to_vec(&value).unwrap();
    let width = PositiveLength::new(Length::from_raw(943_718).unwrap()).unwrap();
    with_prepared_production_inlines(&bytes, |prepared| {
        assert!(typaxis_layout::layout_production_inline_lines(prepared, &[width], 100).is_err());
        let layout =
            typaxis_layout::layout_production_inline_lines(prepared, &[width; 2], 4).unwrap();
        assert_eq!(layout.output_records(), 18);
        assert_eq!(
            layout
                .paragraphs()
                .iter()
                .map(|p| p.owner().get())
                .collect::<Vec<_>>(),
            [2, 4]
        );
        let err = match typaxis_layout::layout_production_inline_lines(prepared, &[width; 2], 3) {
            Err(e) => e,
            Ok(_) => panic!("candidate budget reset"),
        };
        assert_eq!(err.owner.get(), 4);
        assert_eq!(
            err.kind,
            typaxis_layout::ProductionInlinePreparationErrorKind::Atomic(
                typaxis_linebreak::AtomicVectorInlineError::CandidateLimit
            )
        );
    });
    for maximum in [18, 17] {
        let configured = config_with_limits(ResourceLimits {
            max_fragments: maximum,
            ..ResourceLimits::default()
        });
        with_prepared_production_inlines_config(&bytes, &configured, |prepared| {
            let result = typaxis_layout::layout_production_inline_lines(prepared, &[width; 2], 100);
            if maximum == 18 {
                assert_eq!(result.unwrap().output_records(), 18);
            } else {
                let err = match result {
                    Err(e) => e,
                    Ok(_) => panic!("retained record budget reset"),
                };
                assert_eq!(err.owner.get(), 5);
                assert_eq!(
                    err.kind,
                    typaxis_layout::ProductionInlinePreparationErrorKind::UnitLimit
                );
            }
        });
    }
}

#[test]
fn production_inline_joins_shaped_body_and_real_vmb_svg_in_one_candidate() {
    with_prepared_production_inlines(&production_inline_vmb_fixture(true), |prepared| {
        let paragraph = &prepared.paragraphs()[0];
        let items = paragraph.items().unwrap();
        assert_eq!(items.units().len(), 4); // A, space, atomic formula, B.
        assert_eq!(
            paragraph
                .glyph_clusters()
                .iter()
                .map(|c| (c.start_unit(), c.end_unit()))
                .collect::<Vec<_>>(),
            [(0, 1), (1, 2), (3, 4)]
        );
        let selected = typaxis_linebreak::break_production_inline(
            items,
            PositiveLength::new(Length::from_raw(3_000_000).unwrap()).unwrap(),
            paragraph.line_height().unwrap(),
            &mut typaxis_linebreak::ProductionLineBreakBudget::new(100, 10),
        )
        .unwrap();
        assert_eq!(selected.lines().len(), 1);
        let line = &selected.lines()[0];
        assert_eq!(line.origin_shift().get().raw(), 0);
        assert_eq!(line.line().logical_advance().get().raw(), 2_645_629);
        assert_eq!(line.line().occurrences().len(), 1);
        assert_eq!(line.line().occurrences()[0].unit_index(), 2);
        assert_eq!(line.line().occurrences()[0].pen_x().raw(), 707_789);
        assert_eq!(line.line().metrics().line_height().get().raw(), 958_936);
    });
}

#[test]
fn production_inline_real_vmb_formula_alone_fits_after_origin_compensation() {
    with_prepared_production_inlines(&production_inline_vmb_fixture(false), |prepared| {
        let paragraph = &prepared.paragraphs()[0];
        assert!(paragraph.glyph_clusters().is_empty());
        let items = paragraph.items().unwrap();
        let selected = typaxis_linebreak::break_production_inline(
            items,
            PositiveLength::new(Length::from_raw(1_556_093).unwrap()).unwrap(),
            paragraph.line_height().unwrap(),
            &mut typaxis_linebreak::ProductionLineBreakBudget::new(100, 10),
        )
        .unwrap();
        let line = &selected.lines()[0];
        assert_eq!(line.origin_shift().get().raw(), 45_056);
        assert_eq!(line.required_inline_size().get().raw(), 1_556_093);
        assert_eq!(line.line().logical_advance().get().raw(), 1_465_981);
        let occurrence = line.line().occurrences()[0];
        assert_eq!(occurrence.pen_x().raw(), 0);
        assert_eq!(occurrence.item().metrics().origin_x().raw(), -45_056);
        assert!(matches!(
            typaxis_linebreak::break_production_inline(
                items,
                PositiveLength::new(Length::from_raw(1_556_092).unwrap()).unwrap(),
                paragraph.line_height().unwrap(),
                &mut typaxis_linebreak::ProductionLineBreakBudget::new(100, 10)
            ),
            Err(typaxis_linebreak::AtomicVectorInlineError::NoFeasibleLine)
        ));
    });
}

#[test]
fn production_inline_body_only_cff_uses_the_same_candidate_kernel() {
    with_prepared_production_inlines(
        &production_text_single_paragraph(&["A B"], "Typaxis CFF Fixture"),
        |prepared| {
            let paragraph = &prepared.paragraphs()[0];
            let selected = typaxis_linebreak::break_production_inline(
                paragraph.items().unwrap(),
                PositiveLength::new(Length::from_raw(1_179_648).unwrap()).unwrap(),
                paragraph.line_height().unwrap(),
                &mut typaxis_linebreak::ProductionLineBreakBudget::new(100, 10),
            )
            .unwrap();
            assert_eq!(paragraph.glyph_clusters().len(), 3);
            assert_eq!(selected.lines().len(), 1);
            assert!(selected.lines()[0].line().occurrences().is_empty());
            assert_eq!(
                selected.lines()[0].line().logical_advance().get().raw(),
                1_179_648
            );
        },
    );
}

fn production_explicit_break_fixture(kinds: &[&str]) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A", "B"], "Body")).unwrap();
    let templates = value["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array()
        .unwrap()
        .clone();
    let mut children = Vec::new();
    for (index, kind) in kinds.iter().enumerate() {
        let mut item = match *kind {
            "A" => templates[0].clone(),
            "B" => templates[1].clone(),
            "soft_break" | "hard_break" => serde_json::json!({
                "kind":kind, "span":{"source_id":0,"start_byte":0,"end_byte":0}
            }),
            _ => panic!("unknown test kind"),
        };
        item["node_id"] = (index + 3).into();
        children.push(item);
    }
    value["document"]["blocks"][0]["blocks"][0]["children"] = children.into();
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn production_inline_soft_break_is_zero_width_and_owns_the_optional_boundary() {
    with_prepared_production_inlines(
        &production_explicit_break_fixture(&["A", "soft_break", "B"]),
        |prepared| {
            let paragraph = &prepared.paragraphs()[0];
            let items = paragraph.items().unwrap();
            assert_eq!(paragraph.glyph_clusters().len(), 2);
            let typaxis_linebreak::ProductionInlineLogicalUnit::Break(control) = items.units()[1]
            else {
                panic!("typed break")
            };
            assert_eq!(control.owner().get(), 4);
            assert_eq!(control.kind(), typaxis_linebreak::BreakKind::Allowed);
            for (width, expected_lines) in [(943_718, 1), (471_859, 2)] {
                let selected = typaxis_linebreak::break_production_inline(
                    items,
                    PositiveLength::new(Length::from_raw(width).unwrap()).unwrap(),
                    paragraph.line_height().unwrap(),
                    &mut typaxis_linebreak::ProductionLineBreakBudget::new(100, 10),
                )
                .unwrap();
                assert_eq!(selected.lines().len(), expected_lines);
                assert_eq!(
                    selected
                        .lines()
                        .iter()
                        .map(|l| l.line().logical_advance().get().raw())
                        .sum::<i64>(),
                    943_718
                );
                if expected_lines == 2 {
                    assert_eq!(selected.lines()[0].line().end_unit(), 2);
                    assert_eq!(
                        selected.lines()[0].line().break_kind(),
                        typaxis_linebreak::BreakKind::Allowed
                    );
                }
            }
        },
    );
}

/// Insert two independent source anchors at every gap without adding text or
/// control units. Equal-gap markers exercise stable source ordering.
fn production_anchor_gaps_fixture(bytes: &[u8]) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let children = &mut value["document"]["blocks"][0]["blocks"][0]["children"];
    let original = children.as_array().unwrap().clone();
    let mut marked = Vec::new();
    for gap in 0..=original.len() {
        for ordinal in 0..2 {
            marked.push(serde_json::json!({
                "kind":"anchor", "node_id":0,
                "span":{"source_id":0,"start_byte":0,"end_byte":0},
                "anchor_id":format!("gap-{gap}-{ordinal}")
            }));
        }
        if let Some(child) = original.get(gap) {
            marked.push(child.clone());
        }
    }
    *children = marked.into();
    production_body_renumber(&mut value["document"], &mut 0);
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn production_line_anchors_preserve_break_affinity_source_order_and_text_geometry() {
    for (kinds, width, positions) in [
        (
            vec!["A", "hard_break", "B"],
            4_000_000,
            vec![(0, 0), (0, 471_859), (1, 0), (1, 471_859)],
        ),
        (
            vec!["A", "soft_break", "B"],
            471_859,
            vec![(0, 0), (0, 471_859), (1, 0), (1, 471_859)],
        ),
        (
            vec!["A", "soft_break", "B"],
            943_718,
            vec![(0, 0), (0, 471_859), (0, 471_859), (0, 943_718)],
        ),
        (
            vec!["A", "hard_break", "hard_break", "B"],
            4_000_000,
            vec![(0, 0), (0, 471_859), (1, 0), (2, 0), (2, 471_859)],
        ),
        (
            vec!["A", "hard_break"],
            4_000_000,
            vec![(0, 0), (0, 471_859), (0, 471_859)],
        ),
    ] {
        let base = production_explicit_break_fixture(&kinds);
        let bytes = production_anchor_gaps_fixture(&base);
        let width = PositiveLength::new(Length::from_raw(width).unwrap()).unwrap();
        with_prepared_production_inlines(&base, |base_prepared| {
            let base_layout =
                typaxis_layout::layout_production_inline_lines(base_prepared, &[width], 100)
                    .unwrap();
            with_prepared_production_inlines(&bytes, |prepared| {
                let layout =
                    typaxis_layout::layout_production_inline_lines(prepared, &[width], 100)
                        .unwrap();
                let p = &layout.paragraphs()[0];
                let original = &base_layout.paragraphs()[0];
                assert_eq!(
                    prepared.paragraphs()[0].items().unwrap().units().len(),
                    kinds.len()
                );
                assert_eq!(p.anchors().len(), positions.len() * 2);
                assert_eq!(
                    layout.output_records(),
                    base_layout.output_records() + p.anchors().len() as u64
                );
                for (index, marker) in p.anchors().iter().enumerate() {
                    assert_eq!(marker.source(), &prepared.paragraphs()[0].anchors()[index]);
                    assert_eq!(marker.source().boundary_unit() as usize, index / 2);
                    let position = marker.position().unwrap();
                    assert_eq!(
                        (position.line_index(), position.x().raw()),
                        positions[index / 2]
                    );
                    assert_eq!(
                        position.baseline(),
                        p.lines()[position.line_index() as usize].baseline()
                    );
                    if index > 0 {
                        assert!(p.anchors()[index - 1].source().owner() < marker.source().owner());
                    }
                }
                assert_eq!(p.lines().len(), original.lines().len());
                for (a, b) in p.lines().iter().zip(original.lines()) {
                    assert_eq!(a.baseline(), b.baseline());
                    assert_eq!(a.items().len(), b.items().len());
                    for (a, b) in a.items().iter().zip(b.items()) {
                        if let (
                            typaxis_layout::ProductionPlacedInline::Text(a),
                            typaxis_layout::ProductionPlacedInline::Text(b),
                        ) = (a, b)
                        {
                            assert_eq!((a.utf8(), a.pen_x()), (b.utf8(), b.pen_x()));
                            assert_eq!(
                                a.glyphs()
                                    .iter()
                                    .map(|g| (g.glyph().original_gid, g.x(), g.y()))
                                    .collect::<Vec<_>>(),
                                b.glyphs()
                                    .iter()
                                    .map(|g| (g.glyph().original_gid, g.x(), g.y()))
                                    .collect::<Vec<_>>()
                            );
                        }
                    }
                }
            });
        });
    }
}

#[test]
fn production_line_anchors_apply_vector_origin_shift_and_keep_empty_profile_gate() {
    let bytes = production_anchor_gaps_fixture(&production_inline_vmb_fixture(false));
    with_prepared_production_inlines(&bytes, |prepared| {
        let width = PositiveLength::new(Length::from_raw(2_000_000).unwrap()).unwrap();
        let layout =
            typaxis_layout::layout_production_inline_lines(prepared, &[width], 100).unwrap();
        let p = &layout.paragraphs()[0];
        let selected = &p.selected().unwrap().lines()[0];
        assert!(selected.origin_shift().get().raw() > 0);
        assert_eq!(
            p.anchors()[0].position().unwrap().x(),
            selected.origin_shift().get()
        );
        assert_eq!(
            p.anchors()[2].position().unwrap().x(),
            selected
                .line()
                .logical_advance()
                .get()
                .checked_add(selected.origin_shift().get())
                .unwrap()
        );
        assert_eq!(p.lines()[0].items().len(), 1);
    });
    let bytes = production_anchor_gaps_fixture(&production_explicit_break_fixture(&[]));
    // A wholly empty semantic container is rejected by existing profile rules.
    // Retain the empty anchor paragraph beside a real paragraph in that flow.
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A"], "Body")).unwrap();
    value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(body["document"]["blocks"][0]["blocks"][0].clone());
    production_body_renumber(&mut value["document"], &mut 0);
    let bytes = serde_json::to_vec(&value).unwrap();
    let configured = config();
    let decoded = wire::StagingSemanticDocumentPackageDecoder::new()
        .decode(
            &bytes,
            &wire::DocumentPackageDecodePolicy::new(configured.limits()),
        )
        .unwrap();
    let package = typaxis_syntax::StagingSemanticPackageParser::new()
        .parse(decoded, configured.limits())
        .unwrap();
    let limits = typaxis_core::M4EffectiveResourceLimits::defaults_for(configured.limits());
    let navigation =
        typaxis_syntax::validate_staging_book_navigation_v2(&package, &limits).unwrap();
    let semantics =
        typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits)
            .unwrap();
    let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
    assert_eq!(
        typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
            &package,
            &navigation,
            &semantics,
            &limits,
            &identity
        )
        .unwrap_err(),
        typaxis_machine_profile::StagingTaggedPdfProfileError::UnsupportedSemantic
    );
    let flow =
        typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
    assert_eq!(
        flow.paragraphs()[0]
            .items()
            .iter()
            .filter(|s| matches!(s.content(), typaxis_syntax::ProductionInlineContent::Anchor))
            .count(),
        2
    );
}

#[test]
fn production_inline_hard_breaks_preserve_empty_lines_and_terminal_ownership() {
    for (kinds, ranges) in [
        (vec!["A", "hard_break", "B"], vec![(0, 2), (2, 3)]),
        (
            vec!["A", "hard_break", "hard_break", "B"],
            vec![(0, 2), (2, 3), (3, 4)],
        ),
        (vec!["hard_break", "A"], vec![(0, 1), (1, 2)]),
        (vec!["A", "hard_break"], vec![(0, 2)]),
        (vec!["A", "soft_break"], vec![(0, 2)]),
    ] {
        with_prepared_production_inlines(&production_explicit_break_fixture(&kinds), |prepared| {
            let paragraph = &prepared.paragraphs()[0];
            let selected = typaxis_linebreak::break_production_inline(
                paragraph.items().unwrap(),
                PositiveLength::new(Length::from_raw(4_000_000).unwrap()).unwrap(),
                paragraph.line_height().unwrap(),
                &mut typaxis_linebreak::ProductionLineBreakBudget::new(100, 10),
            )
            .unwrap();
            assert_eq!(
                selected
                    .lines()
                    .iter()
                    .map(|l| (l.line().start_unit(), l.line().end_unit()))
                    .collect::<Vec<_>>(),
                ranges
            );
            for line in selected.lines() {
                assert_eq!(line.line().metrics().line_height().get().raw(), 917_504);
                assert_eq!(
                    line.line().break_kind(),
                    typaxis_linebreak::BreakKind::Mandatory
                );
                assert!(line.line().occurrences().is_empty());
            }
        });
    }
}

#[test]
fn production_line_anchors_share_document_record_budget_and_do_not_create_breaks() {
    let bytes = production_anchor_gaps_fixture(&production_explicit_break_fixture(&["A", "B"]));
    let width = PositiveLength::new(Length::from_raw(943_718).unwrap()).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let mut second = value["document"]["blocks"][0]["blocks"][0].clone();
    for child in second["children"].as_array_mut().unwrap() {
        if let Some(id) = child.get_mut("anchor_id") {
            *id = format!("second-{}", id.as_str().unwrap()).into();
        }
    }
    value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(second);
    production_body_renumber(&mut value["document"], &mut 0);
    let bytes = serde_json::to_vec(&value).unwrap();
    let mut required = 0;
    with_prepared_production_inlines(&bytes, |prepared| {
        let layout =
            typaxis_layout::layout_production_inline_lines(prepared, &[width; 2], 100).unwrap();
        required = layout.output_records();
        assert_eq!(required, 30);
        let narrow = PositiveLength::new(Length::from_raw(471_859).unwrap()).unwrap();
        assert!(
            typaxis_layout::layout_production_inline_lines(prepared, &[narrow; 2], 100).is_err(),
            "an anchor must not authorize an otherwise prohibited A/B break"
        );
    });
    for maximum in [required, required - 1] {
        let configured = config_with_limits(ResourceLimits {
            max_fragments: maximum,
            ..ResourceLimits::default()
        });
        with_prepared_production_inlines_config(&bytes, &configured, |prepared| {
            let result = typaxis_layout::layout_production_inline_lines(prepared, &[width; 2], 100);
            if maximum == required {
                assert_eq!(result.unwrap().output_records(), required);
            } else {
                let error = result.err().expect("document-wide marker budget");
                assert_eq!(
                    error.kind,
                    typaxis_layout::ProductionInlinePreparationErrorKind::UnitLimit
                );
            }
        });
    }
}
const PRODUCTION_TEXT_COMBINED: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json"));

fn production_text_without_math_fonts() -> serde_json::Value {
    let mut value: serde_json::Value = serde_json::from_slice(PRODUCTION_TEXT_COMBINED).unwrap();
    for (index, name, hash) in [
        (
            0,
            "body-list-no-math.ttf",
            "271832ab63e0f7cba2ca918d6746e931b00a7746f9c2aeb306ee55c0bc53a26d",
        ),
        (
            1,
            "collection-list-no-math.ttc",
            "143f1f32b9690558bc405fb62bd0b33f7dea25f937b67fc40a4d74ced463eb12",
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

include!("production_body_tests.rs");

include!("production_object_tests.rs");
include!("production_raster_tests.rs");
include!("production_list_tests.rs");

include!("production_container_tests.rs");

include!("production_terminal_tests.rs");

include!("production_reshape_tests.rs");
