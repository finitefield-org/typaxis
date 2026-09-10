fn production_svg_figure_fixture() -> serde_json::Value {
    let mut value = production_raster_fixture("color-2x1.jpg", 2_097_152, 8_000_000);
    let mut alias = value["resources"]["images"][1].clone();
    alias["image_id"] = 5.into();
    value["resources"]["images"]
        .as_array_mut()
        .unwrap()
        .push(alias);
    let parts = value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    parts[2]["image_id"] = 1.into();
    let mut alias = parts[2].clone();
    alias["image_id"] = 5.into();
    alias["alt"] = "Shared SVG, second figure".into();
    alias["caption"] = serde_json::json!([]);
    parts.push(alias);
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_svg_figure_shares_forms_with_source_bound_occurrences() {
    let value = production_svg_figure_fixture();
    with_production_marked_body(&value, &config(), |marked, admitted, limits| {
        let content = marked.content();
        let display = content.plans().fonts().display();
        let figures: Vec<_> = display
            .draws()
            .iter()
            .enumerate()
            .filter_map(|(i, d)| {
                matches!(d, typaxis_display_list::ProductionBodyDraw::SvgFigure(_))
                    .then(|| (i, d.vector_paint().unwrap()))
            })
            .collect();
        assert_eq!(figures.len(), 2);
        assert!(content.rasters().plans().is_empty());
        assert_eq!(figures[0].1.content_key(), figures[1].1.content_key());
        assert_ne!(figures[0].1.image_id(), figures[1].1.image_id());
        assert_ne!(figures[0].1.fingerprint(), figures[1].1.fingerprint());
        // Intrinsic 80 x 40 pt at floor(32/80 * 65536), uniformly applied.
        for (index, figure) in figures {
            assert_eq!(figure.scale_raw(), 26214);
            assert_eq!(figure.viewport().width().get().raw(), 2_097_120);
            assert_eq!(figure.viewport().height().get().raw(), 1_048_560);
            let group_index = marked
                .structure()
                .groups()
                .iter()
                .position(|g| g.draws().contains(&index))
                .unwrap();
            let group = &marked.structure().groups()[group_index];
            let node = marked.structure().registry().node(group.node()).unwrap();
            assert_eq!(node.role(), typaxis_layout::StructureRole::Figure);
            assert!(node.vector_binding_v2().is_none());
            assert_eq!(marked.structure().group_actual_text(group_index), None);
            let usage = &content.vectors().usages()[group.vector_usage_id().unwrap() as usize];
            assert_eq!(usage.paint_ordinal() as usize, index);
            assert_eq!(
                usage.semantic_hook().kind(),
                typaxis_display_list::StagingCombinedVectorKindV2::Figure
            );
            assert_eq!(usage.semantic_hook().owner(), figure.owner());
            assert_eq!(
                usage.semantic_hook().display_command_fingerprint(),
                figure.fingerprint()
            );
        }
        let shared = content
            .plans()
            .forms()
            .plans()
            .iter()
            .find(|p| p.total_usage_count() == 2)
            .unwrap();
        assert_eq!(shared.usages().len(), 2);
        let objects = typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
        let pdf = typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
        let text = String::from_utf8_lossy(pdf.bytes());
        assert_eq!(text.matches("/S /Figure").count(), 2);
        assert!(!text.contains("/Subtype /Image"));
    });
}

fn production_svg_figure_cases() -> Vec<(&'static str, serde_json::Value, usize)> {
    let value = production_svg_figure_fixture();
    let mut only = value.clone();
    let mut figure = only["document"]["blocks"][0]["blocks"][2].clone();
    figure["caption"] = serde_json::json!([]);
    only["document"]["blocks"][0]["blocks"] = serde_json::json!([figure]);
    production_body_renumber(&mut only["document"], &mut 0);
    let mut note = value.clone();
    let figure = note["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .pop()
        .unwrap();
    let span = figure["span"].clone();
    note["document"]["footnotes"] =
        serde_json::json!([{"node_id":0,"span":span,"footnote_id":"svg-note","blocks":[figure]}]);
    let paragraph = &mut note["document"]["blocks"][0]["blocks"][0];
    let end = paragraph["span"]["end_byte"].clone();
    paragraph["children"].as_array_mut().unwrap().push(serde_json::json!({"kind":"footnote_reference","node_id":0,"footnote_id":"svg-note","span":{"source_id":0,"start_byte":end,"end_byte":end}}));
    let body = note["page_masters"]["masters"][0]["body"].clone();
    note["page_masters"]["masters"][0]["footnote"] =
        serde_json::json!({"x":body["x"],"y":13000000,"width":16000000,"height":6000000});
    production_body_renumber(&mut note["document"], &mut 0);
    vec![
        ("svg-figure", value, 2),
        ("svg-figure-footnote", note, 2),
        ("svg-figure-container-only", only, 1),
    ]
}

#[test]
fn production_svg_figure_public_pdf_and_manifest_keep_ordinary_figure_identity() {
    for (name, value, count) in production_svg_figure_cases() {
        let bytes = serde_json::to_vec(&value).unwrap();
        let cfg = EffectiveConfig::new_for_contract(
            DocumentPackageContractId::V1_4,
            false,
            PdfStreamCompression::None,
            vec![ConfigResourceRoot::ProjectRoot],
            ["http", "https", "mailto", "tel"]
                .map(str::to_owned)
                .to_vec(),
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
            ResourceLimits::default(),
        )
        .unwrap();
        let (package, navigation, limits, admitted) = production_text_fixture(&bytes, &cfg);
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
        let result = build_production_book_pdf(
            &package,
            &navigation,
            &semantics,
            &profile,
            &admitted,
            &limits,
            &cfg,
        )
        .unwrap();
        let (pdf, _, selected, _, _, _, _) = result.into_parts();
        assert_eq!(selected, pdf.selected_layout_fingerprint().bytes());
        let text = String::from_utf8_lossy(pdf.bytes());
        assert_eq!(text.matches("/S /Figure").count(), count);
        assert!(!text.contains("/Subtype /Image"));
        if let Some(dir) = std::env::var_os("VMB_SVG_FIGURE_OUT") {
            let dir = PathBuf::from(dir);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(format!("{name}.pdf")), pdf.bytes()).unwrap();
            fs::write(dir.join(format!("{name}-package.json")), bytes).unwrap();
        }
    }
}

#[test]
fn production_svg_figure_uses_intrinsic_dimensions_and_selected_indents() {
    for start_indent in [0, 196608, 262144] {
        let mut value = production_svg_figure_fixture();
        production_body_set_style(&mut value, "figure", "start_indent", start_indent.into());
        production_body_set_style(&mut value, "figure", "end_indent", 327680.into());
        with_production_body_resources(&value, &config(), |lines, blocks, limits, admitted| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            for draw in display.draws() {
                let typaxis_display_list::ProductionBodyDraw::SvgFigure(figure) = draw else {
                    continue;
                };
                let viewport = figure.viewport();
                let body = blocks.page_geometry().body();
                assert_eq!(viewport.x().raw(), body.x().raw() + start_indent);
                assert_eq!(
                    selected.fragments()[figure.fragment_index() as usize].viewport(),
                    Some(viewport)
                );
            }
        });
    }
    let mut value = production_svg_figure_fixture();
    let rule = value["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["selector"] == "figure")
        .unwrap();
    let width = rule["declarations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["name"] == "width")
        .unwrap();
    width["value"] = serde_json::json!({"kind":"keyword","value":"auto"});
    with_production_body_inputs(&value, &config(), |lines, _, _| {
        for figure in lines.figures() {
            assert_eq!(figure.width().get().raw(), 80 * 65536);
            assert_eq!(figure.height().get().raw(), 40 * 65536);
            let typaxis_layout::ProductionFigureMedia::Svg { scale_raw, .. } = figure.media()
            else {
                panic!("wrong media");
            };
            assert_eq!(scale_raw, 65536);
        }
    });
}
