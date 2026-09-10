fn production_raster_fixture(file: &str, width: i64, height: i64) -> serde_json::Value {
    let mut value = production_body_fixture(height);
    let image_id = if file == "color-2x1.jpg" {
        3
    } else {
        let index: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/vmb-book/raster/fixture-index.json"))).unwrap();
        let case = index["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["file"] == file)
            .unwrap();
        value["resources"]["images"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
            "image_id":5,"uri":file,"media_type":"png","expected_sha256":case["sha256"]}));
        5
    };
    let parts = value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    let span = parts[1]["span"].clone();
    let caption = parts[2].clone();
    // Keep the block formula before the ordinary Figure and a following body
    // paragraph after its actual caption. Empty spans do not duplicate TeX.
    let end = caption["span"].clone();
    parts.insert(2, serde_json::json!({"kind":"figure","node_id":9,"span":end,
        "classes":[],"image_id":image_id,"placement":"block","alt":"VMB diagram","caption":[caption]}));
    assert!(span["end_byte"].as_u64().unwrap() <= end["start_byte"].as_u64().unwrap());
    production_body_renumber(&mut value["document"], &mut 0);
    production_body_set_style(&mut value, "figure", "width", width.into());
    value
}

#[test]
fn production_body_raster_keeps_pixel_aspect_and_real_caption_flow() {
    use typaxis_pagination::ProductionBodyFragmentSource as S;
    for keep in [false, true] {
        let mut value = production_raster_fixture("book-venn.png", 2_000_001, 3_000_000);
        // Use text, figure, caption, following text for a tight keep boundary.
        value["document"]["blocks"][0]["blocks"]
            .as_array_mut()
            .unwrap()
            .remove(1);
        production_body_renumber(&mut value["document"], &mut 0);
        production_body_set_style(&mut value, "figure", "keep_caption", keep.into());
        with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
            assert_eq!(lines.figures().len(), 1);
            let measured = &lines.figures()[0];
            assert_eq!(
                measured.media(),
                typaxis_layout::ProductionFigureMedia::Raster { pixel_width: 1200, pixel_height: 720 }
            );
            assert_eq!(measured.width().get().raw(), 2_000_001);
            assert_eq!(measured.height().get().raw(), 1_200_001);
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let fragments = selected.fragments();
            assert_eq!(fragments.len(), 4);
            assert_eq!(fragments[1].source(), S::Figure { figure_index: 0 });
            assert_eq!(fragments[1].viewport(), Some(fragments[1].bounds()));
            assert_eq!(fragments[2].bounds().height().get().raw(), 917_504);
            assert_eq!(
                fragments.iter().map(|f| f.page_index()).collect::<Vec<_>>(),
                if keep {
                    vec![0, 1, 1, 2]
                } else {
                    vec![0, 0, 1, 1]
                }
            );
        });
    }
    let too_wide = production_raster_fixture("book-venn.png", 4_000_000, 10_000_000);
    with_production_body_inputs(&too_wide, &config(), |lines, blocks, limits| {
        assert_eq!(
            typaxis_pagination::paginate_production_body(lines, blocks, limits)
                .err()
                .unwrap()
                .kind,
            typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch
        );
    });
    for (width, rounded_height) in [(1_000_001, 500_000), (1_000_003, 500_002)] {
        let value = production_raster_fixture("color-2x1.jpg", width, 3_500_000);
        with_production_body_inputs(&value, &config(), |lines, _, _| {
            assert_eq!(lines.figures()[0].height().get().raw(), rounded_height);
        });
    }
    let mut oversize = production_raster_fixture("orientation-alpha.png", 3_000_000, 2_500_000);
    production_body_set_style(&mut oversize, "figure", "keep_caption", false.into());
    with_production_body_inputs(&oversize, &config(), |lines, blocks, limits| {
        let failure = typaxis_pagination::paginate_production_body(lines, blocks, limits)
            .err()
            .unwrap();
        assert_eq!(
            failure.kind,
            typaxis_pagination::ProductionBodyPaginationErrorKind::Oversize
        );
        assert_eq!(failure.owner, lines.figures()[0].owner());
    });
}

#[test]
fn production_body_raster_pdf_preserves_shared_payloads_alpha_tags_and_source_order() {
    use typaxis_pdf::ProductionBodyObjectRole as R;
    for file in ["book-venn.png", "orientation-alpha.png", "color-2x1.jpg"] {
        let mut value = production_raster_fixture(file, 2_000_001, 3_500_000);
        // A distinct logical ID with identical bytes must share its payload.
        let mut alias = value["resources"]["images"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["uri"] == file)
            .unwrap()
            .clone();
        let alias_id = value["resources"]["images"].as_array().unwrap().len();
        alias["image_id"] = alias_id.into();
        value["resources"]["images"]
            .as_array_mut()
            .unwrap()
            .push(alias);
        let mut alias = value["document"]["blocks"][0]["blocks"][2].clone();
        alias["image_id"] = alias_id.into();
        alias["alt"] = "Same pixels, second occurrence".into();
        alias["caption"] = serde_json::json!([]);
        value["document"]["blocks"][0]["blocks"]
            .as_array_mut()
            .unwrap()
            .push(alias);
        production_body_renumber(&mut value["document"], &mut 0);
        with_production_marked_body(&value, &config(), |marked, admitted, limits| {
            let content = marked.content();
            let display = content.plans().fonts().display();
            let rasters = content.rasters();
            assert_eq!(rasters.plans().len(), 1);
            let plan = &rasters.plans()[0];
            assert_eq!(
                plan.encoding(),
                if file.ends_with(".jpg") {
                    typaxis_resources::ImageEncoding::Jpeg
                } else {
                    typaxis_resources::ImageEncoding::Flate
                }
            );
            let paints = display
                .draws()
                .iter()
                .enumerate()
                .filter_map(|(i, d)| {
                    if let typaxis_display_list::ProductionBodyDraw::Raster(r) = d {
                        Some((i, r))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(paints.len(), 2);
            assert_ne!(paints[0].1.owner(), paints[1].1.owner());
            assert_ne!(paints[0].1.alternative(), paints[1].1.alternative());
            for (index, raster) in &paints {
                assert_eq!(rasters.draw_plan(*index), Some(0));
                let group_index = marked
                    .structure()
                    .groups()
                    .iter()
                    .position(|g| g.draws().contains(index))
                    .unwrap();
                let group = &marked.structure().groups()[group_index];
                let node = marked.structure().registry().node(group.node()).unwrap();
                assert!(!group.is_text());
                assert_eq!(group.vector_usage_id(), None);
                assert_eq!(node.role(), typaxis_layout::StructureRole::Figure);
                assert_eq!(node.alternative(), Some(raster.alternative()));
                assert_eq!(marked.structure().group_actual_text(group_index), None);
            }
            assert_eq!(marked.anchors().len(), 2); // inline and block formulas only
            let objects =
                typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
            assert_eq!(
                objects
                    .objects()
                    .iter()
                    .filter(|o| matches!(o.role(), R::Raster(_)))
                    .count(),
                1
            );
            assert_eq!(
                objects
                    .objects()
                    .iter()
                    .filter(|o| matches!(o.role(), R::RasterMask(_)))
                    .count(),
                usize::from(plan.alpha_mask().is_some())
            );
            assert_eq!(plan.alpha_mask().is_some(), file == "orientation-alpha.png");
            let pdf =
                typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
            let again =
                typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
            assert_eq!(pdf.bytes(), again.bytes());
            if let Some(root) = std::env::var_os("TYPAXIS_RASTER_PDF_PROBE_DIR") {
                let root = PathBuf::from(root);
                fs::create_dir_all(&root).unwrap();
                fs::write(root.join(format!("{file}.pdf")), pdf.bytes()).unwrap();
                let expected = serde_json::json!({"algorithm":"typaxis.production-raster-probe/1",
                    "public_build":false,"full_book":false,"source":file,"source_sha256":
                        plan.admitted_sha256().iter().map(|b|format!("{b:02x}")).collect::<String>(),
                    "page_count":pdf.page_count(),"pixel_width":plan.width().get(),"pixel_height":plan.height().get(),
                    "retained_spool":rasters.spool_charge(),"peak_spool":rasters.peak_spool_charge(),
                    "encoded_bytes":plan.encoded_bytes().len(),"draws":paints.iter().map(|(_,r)|{
                        let v=r.viewport();serde_json::json!({"page":r.page_index(),"owner":r.owner().get(),
                            "alt":r.alternative(),"viewport_raw":[v.x().raw(),v.y().raw(),v.width().get().raw(),v.height().get().raw()]})
                    }).collect::<Vec<_>>()});
                fs::write(
                    root.join(format!("{file}.expected.json")),
                    serde_json::to_vec_pretty(&expected).unwrap(),
                )
                .unwrap();
            }
        });
    }
}

#[test]
fn production_body_raster_resource_budget_and_owner_are_not_reset() {
    let value = production_raster_fixture("orientation-alpha.png", 1_000_001, 3_500_000);
    with_production_marked_body(&value, &config(), |marked, admitted, limits| {
        use typaxis_resources::{finalize_production_body_rasters as freeze, ResourceError as E};
        let content = marked.content();
        let fonts = content.plans().fonts();
        let record_base = content.plans().record_charge();
        let spool_base =
            fonts.spool_charge() + content.text().byte_length() + content.vectors().spool_bytes();
        assert_eq!(
            freeze(
                fonts,
                admitted,
                limits,
                fonts.record_charge() - 1,
                spool_base
            )
            .err(),
            Some(E::AdmittedLedgerEpochMismatch)
        );
        assert_eq!(
            freeze(
                fonts,
                admitted,
                limits,
                record_base,
                fonts.spool_charge() - 1
            )
            .err(),
            Some(E::AdmittedLedgerEpochMismatch)
        );
        assert_eq!(
            freeze(
                fonts,
                admitted,
                limits,
                limits.base().get().max_fragments,
                spool_base
            )
            .err(),
            Some(E::ResourceLimit)
        );
        assert_eq!(
            freeze(
                fonts,
                admitted,
                limits,
                record_base,
                limits.base().get().max_spool_bytes
            )
            .err(),
            Some(E::ResourceLimit)
        );
        let frozen = freeze(fonts, admitted, limits, record_base, spool_base).unwrap();
        assert_eq!(frozen.plans(), content.rasters().plans());
        frozen
            .verify(fonts, admitted, limits, record_base, spool_base)
            .unwrap();
        assert_eq!(
            frozen.verify(fonts, admitted, limits, record_base + 1, spool_base),
            Err(E::AdmittedLedgerEpochMismatch)
        );
        assert_eq!(
            frozen.verify(fonts, admitted, limits, record_base, spool_base + 1),
            Err(E::AdmittedLedgerEpochMismatch)
        );
        let other =
            typaxis_resources::finalize_production_body_fonts(fonts.display(), admitted, limits)
                .unwrap();
        assert_eq!(
            frozen.verify(&other, admitted, limits, record_base, spool_base),
            Err(E::AdmittedLedgerEpochMismatch)
        );
        assert!(frozen.peak_spool_charge() > frozen.spool_charge());
        let record_slack = limits.base().get().max_fragments - frozen.record_charge();
        assert_eq!(
            freeze(
                fonts,
                admitted,
                limits,
                record_base + record_slack,
                spool_base
            )
            .unwrap()
            .record_charge(),
            limits.base().get().max_fragments
        );
        assert_eq!(
            freeze(
                fonts,
                admitted,
                limits,
                record_base + record_slack + 1,
                spool_base
            )
            .err(),
            Some(E::ResourceLimit)
        );
        let spool_slack = limits.base().get().max_spool_bytes - frozen.peak_spool_charge();
        assert_eq!(
            freeze(
                fonts,
                admitted,
                limits,
                record_base,
                spool_base + spool_slack
            )
            .unwrap()
            .peak_spool_charge(),
            limits.base().get().max_spool_bytes
        );
        assert_eq!(
            freeze(
                fonts,
                admitted,
                limits,
                record_base,
                spool_base + spool_slack + 1
            )
            .err(),
            Some(E::ResourceLimit)
        );
    });
}
