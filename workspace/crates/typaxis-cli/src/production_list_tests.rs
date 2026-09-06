fn production_list_fixture() -> serde_json::Value {
    let mut value = production_body_fixture(10_000_000);
    value["page_masters"]["masters"][0]["body"]["width"] = 8_000_000.into();
    let paragraph = value["document"]["blocks"][0]["blocks"][2].clone();
    let span = paragraph["span"].clone();
    value["document"]["blocks"][0]["blocks"][2] = serde_json::json!({"kind":"list","node_id":0,"span":span,
        "classes":[],"ordered":true,"start":9,"items":[{"node_id":0,"span":span,"blocks":[paragraph,
            {"kind":"list","node_id":0,"span":span,"classes":[],"ordered":false,"start":null,"items":[
                {"node_id":0,"span":span,"blocks":[paragraph]},{"node_id":0,"span":span,"blocks":[paragraph]}]}]},
            {"node_id":0,"span":span,"blocks":[paragraph]}]});
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_list_frames_use_maximum_marker_width_and_nested_item_frames() {
    let value = production_list_fixture();
    with_production_inline_tagged_context(
        &serde_json::to_vec(&value).unwrap(),
        &config(),
        |prepared, package, profile, limits, admitted, bindings, _, _| {
            let math = typaxis_layout::prepare_staging_math_vector_flows(
                package, profile, limits, admitted, bindings,
            )
            .unwrap();
            let blocks = typaxis_layout::prepare_staging_precomposed_vector_blocks(
                package, profile, limits, admitted, bindings, &math,
            )
            .unwrap();
            let body = blocks.page_geometry().body();
            let lines =
                typaxis_layout::layout_production_body_inline_lines(prepared, body, 100_000)
                    .unwrap();
            let frames = lines.frames().unwrap();
            frames.verify(prepared, body).unwrap();
            assert_eq!(frames.lists().len(), 2);
            let outer = &frames.lists()[0];
            let inner = &frames.lists()[1];
            assert_eq!(outer.marker_width(), prepared.list_markers()[3].advance());
            assert_eq!(inner.marker_width(), prepared.list_markers()[1].advance());
            assert_eq!(outer.marker_gap(), prepared.list_markers()[0].font().size());
            assert_eq!(inner.marker_gap(), prepared.list_markers()[1].font().size());
            let style = prepared.source_flow().lists()[0].style().block_style();
            assert_eq!(outer.marker_start(), style.start_indent().get());
            let consumed = style.start_indent().get().raw()
                + style.end_indent().get().raw()
                + outer.marker_width().get().raw()
                + outer.marker_gap().get().raw();
            assert_eq!(
                outer.content().width().get().raw(),
                body.width().get().raw() - consumed
            );
            assert_eq!(
                frames.region(prepared.source_flow().lists()[1].owner()),
                Some(outer.content())
            );
            assert_eq!(
                inner.marker_start().raw(),
                outer.content().start().raw() + style.start_indent().get().raw()
            );
            for (index, (paragraph, line)) in prepared
                .source_flow()
                .paragraphs()
                .iter()
                .zip(lines.paragraphs())
                .enumerate()
            {
                let region = frames.region(paragraph.owner()).unwrap();
                let pstyle = paragraph.style().block_style();
                assert_eq!(line.inline_size(), frames.paragraphs()[index].width());
                assert_eq!(
                    frames.paragraphs()[index].width().get().raw(),
                    region.width().get().raw()
                        - pstyle.start_indent().get().raw()
                        - pstyle.end_indent().get().raw()
                );
                assert_eq!(
                    frames.paragraphs()[index].start().raw(),
                    region.start().raw() + pstyle.start_indent().get().raw()
                );
            }
            assert!(lines.output_records() > frames.record_charge());
            let narrow = typaxis_core::Rect::new(
                body.x(),
                body.y(),
                PositiveLength::new(Length::from_raw(consumed).unwrap()).unwrap(),
                body.height(),
            );
            assert_eq!(
                frames.verify(prepared, narrow).unwrap_err().kind,
                typaxis_layout::ProductionInlinePreparationErrorKind::ReceiptMismatch
            );
            let error =
                typaxis_layout::layout_production_body_inline_lines(prepared, narrow, 100_000)
                    .err()
                    .unwrap();
            assert_eq!(error.owner, outer.owner());
            assert_eq!(
                error.kind,
                typaxis_layout::ProductionInlinePreparationErrorKind::ListFrameExhausted
            );
        },
    );
}

#[test]
fn production_list_markers_keep_nested_source_keys_and_real_font_metrics() {
    use typaxis_shaping::ShapeSourceSpan;
    for (family, size, face_id) in [("Body", 12 * 65_536, 0), ("Collection", 14 * 65_536, 1)] {
        let mut value = production_list_fixture();
        let style = value["style_sheet"]["rules"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["selector"] == "list")
            .unwrap();
        style["declarations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|d| d["name"] == "font_family")
            .unwrap()["value"]["families"] = serde_json::json!([family]);
        production_body_set_style(&mut value, "list", "font_size", size.into());
        let (package, navigation, limits, admitted) =
            production_text_fixture(&serde_json::to_vec(&value).unwrap(), &config());
        let flow =
            typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
        assert_eq!(flow.lists().len(), 2);
        assert_eq!(flow.list_items().len(), 4);
        assert_eq!(
            (0..4)
                .map(|i| flow.list_marker_text(i).unwrap())
                .collect::<Vec<_>>(),
            ["9.", "•", "•", "10."]
        );
        assert_eq!(
            flow.list_items()
                .iter()
                .map(|i| i.list_index())
                .collect::<Vec<_>>(),
            [0, 1, 1, 0]
        );
        assert_eq!(
            flow.list_items()
                .iter()
                .map(|i| i.item_index())
                .collect::<Vec<_>>(),
            [0, 0, 1, 1]
        );
        assert_eq!(flow.generated_text_bytes(), 11);
        flow.verify(&package, &navigation, &limits).unwrap();
        let epoch = sha256(b"production-list-markers-test");
        let shape = typaxis_shaping::shape_production_authored_text(
            &package,
            &navigation,
            &flow,
            &admitted,
            &limits,
            epoch,
        )
        .unwrap();
        assert_eq!(shape.list_markers().len(), 4);
        let semantics =
            typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits)
                .unwrap();
        for (index, marker) in shape.list_markers().iter().enumerate() {
            assert!(std::ptr::eq(marker.source(), &flow.list_items()[index]));
            assert_eq!(marker.provenance().buffer_key(), marker.source().key());
            assert_eq!(
                marker.glyph_run().source_span,
                ShapeSourceSpan::Generated(marker.provenance())
            );
            assert_eq!(marker.font().face_id().get(), face_id);
            assert_eq!(marker.font().size().get().raw(), size);
            assert_eq!(
                marker.font().content_hash(),
                admitted
                    .font(marker.font().face_id())
                    .unwrap()
                    .content_hash()
            );
            assert_eq!(
                marker.advance().get().raw(),
                marker
                    .glyph_run()
                    .glyphs
                    .iter()
                    .map(|g| g.advance_x.raw())
                    .sum::<i64>()
            );
            assert!(marker.advance().get().raw() > 0);
            assert!(marker
                .glyph_run()
                .glyphs
                .iter()
                .all(|g| g.original_gid.get() != 0));
            for cluster in &marker.glyph_run().clusters {
                let ShapeSourceSpan::Generated(provenance) = cluster.source_span else {
                    panic!("label is not parsed body text");
                };
                assert_eq!(provenance.buffer_key(), marker.source().key());
            }
            let semantic = semantics
                .records()
                .iter()
                .find(|r| r.node_id() == marker.source().owner())
                .unwrap();
            let typaxis_syntax::StagingStructureSemanticKind::ListItem { marker: expected } =
                semantic.kind()
            else {
                panic!("LI");
            };
            assert_eq!(marker.utf8(), expected);
        }
        assert!(shape.list_markers()[3].advance().get() > shape.list_markers()[0].advance().get());
        let again = typaxis_shaping::shape_production_authored_text(
            &package,
            &navigation,
            &flow,
            &admitted,
            &limits,
            epoch,
        )
        .unwrap();
        assert_eq!(shape.fingerprint(), again.fingerprint());
        assert_eq!(
            shape
                .list_markers()
                .iter()
                .map(|m| m.fingerprint())
                .collect::<Vec<_>>(),
            again
                .list_markers()
                .iter()
                .map(|m| m.fingerprint())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn production_list_missing_bullet_reports_source_item() {
    let mut value = production_list_fixture();
    value["resources"]["font_faces"][0]["uri"] = "body-no-math.ttf".into();
    value["resources"]["font_faces"][0]["expected_sha256"] =
        "c399cf1de56ffdc143b97547749a128b49b9356ec89a07a86ab0ca8107b3c50e".into();
    let (package, navigation, limits, admitted) =
        production_text_fixture(&serde_json::to_vec(&value).unwrap(), &config());
    let flow =
        typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits).unwrap();
    let failure = typaxis_shaping::shape_production_authored_text(
        &package,
        &navigation,
        &flow,
        &admitted,
        &limits,
        sha256(b"list-coverage"),
    )
    .err()
    .expect("missing U+2022 must not turn into whitespace");
    assert_eq!(failure.owner, flow.list_items()[1].owner());
    assert_eq!(
        failure.kind,
        typaxis_shaping::ProductionTextShapeErrorKind::MissingDeclaredFontCoverage
    );
}

#[test]
fn production_list_selected_labels_and_pdf_structure_share_actual_fragments() {
    let mut value = production_list_fixture();
    value["page_masters"]["masters"][0]["body"]["height"] = 3_000_000.into();
    with_production_marked_body(&value, &config(), |marked, admitted, limits| {
        use typaxis_display_list::ProductionBodyDraw as D;
        let structure = marked.structure();
        let display = structure.display();
        let selected = display.selected();
        assert_eq!(selected.list_markers().len(), 4);
        assert!(selected.pages().len() > 1);
        let parsed_count = value["text_buffers"].as_array().unwrap().len() as u32;
        for marker in selected.list_markers() {
            let fragment = &selected.fragments()[marker.fragment_index() as usize];
            assert_eq!(marker.page_index(), fragment.page_index());
            assert_eq!(Some(marker.baseline()), fragment.baseline());
            let draws = display
                .draws()
                .iter()
                .filter_map(|d| match d {
                    D::Text(t) if t.owner() == marker.owner() => Some(t),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                draws.iter().map(|d| d.exact_text()).collect::<String>(),
                selected.line_layout().list_markers()[marker.marker_index() as usize].utf8()
            );
            for draw in &draws {
                assert!(draw.generated_provenance().is_some());
                assert!(draw.text_span().text_id().get() >= parsed_count);
                assert_eq!(draw.fragment_index(), marker.fragment_index());
            }
        }
        let labels = structure
            .registry()
            .nodes()
            .iter()
            .filter(|n| n.role() == typaxis_layout::StructureRole::Label)
            .collect::<Vec<_>>();
        assert_eq!(labels.len(), 4);
        for label in labels {
            let groups = structure.node_groups(label.structure_node_id()).unwrap();
            assert_eq!(groups.len(), 1);
            assert!(structure.groups()[groups[0]].is_text());
        }
        let objects = typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
        let pdf = typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
        pdf.verify(&objects, admitted, limits).unwrap();
    });
}

fn production_list_wrap(value: &mut serde_json::Value, start: usize, end: usize) {
    let parts = value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    let mut span = parts[start]["span"].clone();
    span["end_byte"] = parts[end - 1]["span"]["end_byte"].clone();
    let contents = parts.drain(start..end).collect::<Vec<_>>();
    parts.insert(start,serde_json::json!({"kind":"list","node_id":0,"span":span,"classes":[],"ordered":false,"start":null,
        "items":[{"node_id":0,"span":span,"blocks":contents}]}));
    production_body_renumber(&mut value["document"], &mut 0);
    value["page_masters"]["masters"][0]["body"]["width"] = 8_000_000.into();
}

#[test]
fn production_list_first_formula_and_raster_use_actual_item_width_and_baseline() {
    for kind in ["inline", "block", "raster"] {
        let mut value = if kind == "raster" {
            production_raster_fixture("orientation-alpha.png", 1_500_000, 4_000_000)
        } else {
            production_body_fixture(3_000_000)
        };
        let start = match kind {
            "inline" => 0,
            "block" => 1,
            _ => 2,
        };
        let end = value["document"]["blocks"][0]["blocks"]
            .as_array()
            .unwrap()
            .len();
        production_list_wrap(&mut value, start, end);
        with_production_marked_body(&value, &config(), |marked, admitted, limits| {
            let selected = marked.structure().display().selected();
            let lines = selected.line_layout();
            assert_eq!(selected.list_markers().len(), 1);
            let label = &selected.list_markers()[0];
            let fragment = &selected.fragments()[label.fragment_index() as usize];
            let frame = lines.frames().unwrap().lists()[0].content();
            let body = selected.page_geometry().body();
            if kind == "raster" {
                assert_eq!(label.bounds().y(), fragment.bounds().y());
                assert_eq!(
                    label.baseline(),
                    fragment
                        .bounds()
                        .y()
                        .checked_add(lines.list_markers()[0].font().ascender())
                        .unwrap()
                );
            } else {
                assert_eq!(Some(label.baseline()), fragment.baseline());
            }
            assert!(fragment.bounds().x().raw() >= body.x().raw() + frame.start().raw());
            assert!(
                fragment.bounds().x().raw() + fragment.bounds().width().get().raw()
                    <= body.x().raw() + frame.start().raw() + frame.width().get().raw()
            );
            if kind == "block" {
                let b = &selected.block_layout().blocks()[0];
                let width = frame.width().get().raw()
                    - b.start_indent().get().raw()
                    - b.end_indent().get().raw();
                assert_eq!(fragment.bounds().width().get().raw(), width);
                assert_ne!(fragment.bounds().width(), b.inner_frame_width());
                let slack = width - b.viewport_width().get().raw();
                let offset = match b.text_align().as_str() {
                    "start" => 0,
                    "end" => slack,
                    "center" => slack / 2,
                    _ => panic!("alignment"),
                };
                assert_eq!(
                    fragment.viewport().unwrap().x().raw(),
                    fragment.bounds().x().raw() + offset
                );
            }
            let objects =
                typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
            typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
        });
    }
}

#[test]
fn production_list_large_marker_consumes_height_once_across_continuation_pages() {
    let mut value = production_body_fixture(3_000_000);
    let paragraph = value["document"]["blocks"][0]["blocks"][2].clone();
    value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .extend(std::iter::repeat(paragraph).take(8));
    production_list_wrap(&mut value, 2, 11);
    production_body_set_style(&mut value, "list", "font_size", (32 * 65_536).into());
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        assert_eq!(selected.list_markers().len(), 1);
        let marker = &selected.list_markers()[0];
        let first = &selected.fragments()[marker.fragment_index() as usize];
        let font = lines.list_markers()[0].font();
        assert_eq!(marker.baseline(), first.baseline().unwrap());
        assert!(marker.bounds().y() >= blocks.page_geometry().body().y());
        assert!(marker.bounds().y() < first.bounds().y());
        assert_eq!(
            marker.bounds().height().get(),
            font.ascender().checked_sub(font.descender()).unwrap()
        );
        assert!(selected.fragments().last().unwrap().page_index() > marker.page_index());
        let page = &selected.pages()[marker.page_index() as usize];
        assert!(page.used_height() >= marker.bounds().height().get());
        assert_eq!(first.bounds().height().get().raw(), 917_504);
    });
    value["page_masters"]["masters"][0]["body"]["height"] = 1_900_000.into();
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let failure = typaxis_pagination::paginate_production_body(lines, blocks, limits)
            .err()
            .unwrap();
        assert_eq!(
            failure.kind,
            typaxis_pagination::ProductionBodyPaginationErrorKind::Oversize
        );
    });
}

#[test]
fn production_list_independent_pdf_probes() {
    use typaxis_display_list::ProductionBodyDraw as D;
    use typaxis_pdf::ProductionBodyObjectRole as R;
    let mut nested = production_list_fixture();
    nested["page_masters"]["masters"][0]["body"]["height"] = 3_000_000.into();
    let mut block = production_body_fixture(3_000_000);
    production_list_wrap(&mut block, 1, 3);
    let mut inline = production_body_fixture(3_000_000);
    production_list_wrap(&mut inline, 0, 3);
    let mut raster = production_raster_fixture("orientation-alpha.png", 1_500_000, 4_000_000);
    production_list_wrap(&mut raster, 2, 4);
    for (name, mut value) in [
        ("nested", nested),
        ("block-first", block),
        ("inline-first", inline),
        ("raster-first", raster),
    ] {
        value["resources"]["font_faces"][0]["uri"] = "body-list-visible.ttf".into();
        value["resources"]["font_faces"][0]["expected_sha256"] =
            "17857592837017395c9f22614b712f41d2ad6175a5b3a39c4d8aa879c99044c6".into();
        for rule in value["style_sheet"]["rules"].as_array_mut().unwrap() {
            if rule["selector"] == "paragraph" {
                rule["declarations"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|d| d["name"] == "font_family")
                    .unwrap()["value"]["families"] = serde_json::json!(["Typaxis CFF Fixture"]);
            }
        }
        with_production_marked_body(&value, &config(), |marked, admitted, limits| {
            let objects =
                typaxis_pdf::build_production_body_objects(marked, admitted, limits).unwrap();
            let pdf =
                typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
            let structure = marked.structure();
            let display = structure.display();
            let selected = display.selected();
            let labels=selected.list_markers().iter().map(|marker| {
                let shape=&selected.line_layout().list_markers()[marker.marker_index() as usize];
                let node=structure.registry().nodes().iter().find(|n|n.role()==typaxis_layout::StructureRole::Label
                    && matches!(n.owner(),typaxis_layout::StructureOwner::Generated(k) if k.owner_node_id()==marker.owner())).unwrap();
                let group=&structure.groups()[structure.node_groups(node.structure_node_id()).unwrap()[0]];
                let glyphs=display.draws()[group.draws()].iter().flat_map(|d|match d {D::Text(t)=>t.glyphs(),_=>panic!()}).map(|g|serde_json::json!([g.x().raw(),g.y().raw()])).collect::<Vec<_>>();
                serde_json::json!({"owner":marker.owner().get(),"text":shape.utf8(),"page":marker.page_index(),"mcid":group.mcid(),
                    "bounds_raw":[marker.bounds().x().raw(),marker.bounds().y().raw(),marker.bounds().width().get().raw(),marker.bounds().height().get().raw()],
                    "font_size_raw":shape.font().size().get().raw(),"baseline_raw":marker.baseline().raw(),"glyphs_raw":glyphs,
                    "structure_object":pdf.object_number(R::StructureNode(node.structure_node_id())).unwrap(),
                    "item_object":pdf.object_number(R::StructureNode(node.parent().unwrap())).unwrap()})
            }).collect::<Vec<_>>();
            let groups=structure.groups().iter().enumerate().map(|(i,g)| {
                let node=structure.registry().node(g.node()).unwrap();
                let text=if g.is_text() {display.draws()[g.draws()].iter().map(|d|match d {D::Text(t)=>t.exact_text(),_=>panic!()}).collect::<String>()}
                    else {structure.group_actual_text(i).unwrap_or("").to_owned()};
                serde_json::json!({"page":g.page_index(),"mcid":g.mcid(),"role":node.role().pdf_name(),"text":text})
            }).collect::<Vec<_>>();
            let expected = serde_json::json!({"algorithm":"typaxis.production-list-probe/1","public_build":false,"full_book":false,"case":name,"pages":selected.pages().len(),"labels":labels,"groups":groups});
            if let Some(root) = std::env::var_os("TYPAXIS_LIST_PDF_PROBE_DIR") {
                let root = PathBuf::from(root);
                fs::create_dir_all(&root).unwrap();
                fs::write(root.join(format!("{name}.pdf")), pdf.bytes()).unwrap();
                fs::write(
                    root.join(format!("{name}.expected.json")),
                    serde_json::to_vec_pretty(&expected).unwrap(),
                )
                .unwrap();
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
fn production_list_marker_skips_nonpainting_break_lines_and_forced_blank_page() {
    let mut value = production_body_fixture(3_000_000);
    production_list_wrap(&mut value, 2, 3);
    let list = &mut value["document"]["blocks"][0]["blocks"][2];
    let span = list["span"].clone();
    let parts = list["items"][0]["blocks"].as_array_mut().unwrap();
    parts[0]["children"].as_array_mut().unwrap().insert(
        0,
        serde_json::json!({"kind":"hard_break","node_id":0,"span":span}),
    );
    parts.insert(
        0,
        serde_json::json!({"kind":"page_break","node_id":0,"span":span,"classes":[]}),
    );
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_marked_body(&value, &config(), |marked, _, _| {
        let selected = marked.structure().display().selected();
        assert_eq!(selected.list_markers().len(), 1);
        let marker = &selected.list_markers()[0];
        assert_eq!(marker.page_index(), 1);
        assert_eq!(
            selected.fragments()[marker.fragment_index() as usize].source(),
            typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index: 1,
                line_index: 1
            }
        );
        assert_eq!(marker.fragment_index(), 3);
        assert_eq!(
            selected.fragments()[2].source(),
            typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index: 1,
                line_index: 0
            }
        );
    });
}

#[test]
fn production_list_number_overflow_identifies_the_source_item() {
    let mut value = production_list_fixture();
    let list = &mut value["document"]["blocks"][0]["blocks"][2];
    list["start"] = u32::MAX.into();
    let owner = list["items"][1]["node_id"].as_u64().unwrap() as u32;
    let cfg = config();
    let bytes = serde_json::to_vec(&value).unwrap();
    let decoded = wire::StagingSemanticDocumentPackageDecoder::new()
        .decode(
            &bytes,
            &wire::DocumentPackageDecodePolicy::new(cfg.limits()),
        )
        .unwrap();
    let package = typaxis_syntax::StagingSemanticPackageParser::new()
        .parse(decoded, cfg.limits())
        .unwrap();
    let limits = typaxis_core::M4EffectiveResourceLimits::defaults_for(cfg.limits());
    let navigation =
        typaxis_syntax::validate_staging_book_navigation_v2(&package, &limits).unwrap();
    let failure = typaxis_syntax::prepare_production_text_flow(&package, &navigation, &limits)
        .err()
        .unwrap();
    assert_eq!(failure.owner.get(), owner);
    assert_eq!(
        failure.kind,
        typaxis_syntax::ProductionFlowErrorKind::MarkerOverflow
    );
}

#[test]
fn production_list_pagination_does_not_reset_frame_or_generated_glyph_budget() {
    let value = production_list_fixture();
    let mut required = 0;
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        required = selected.record_charge();
        assert!(required > lines.output_records());
        assert!(lines.output_records() > lines.frames().unwrap().record_charge());
    });
    for maximum in [required, required - 1] {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: maximum,
            ..ResourceLimits::default()
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits);
            if maximum == required {
                assert_eq!(selected.unwrap().record_charge(), required);
            } else {
                assert_eq!(
                    selected.err().unwrap().kind,
                    typaxis_pagination::ProductionBodyPaginationErrorKind::FragmentLimit
                );
            }
        });
    }
}

#[test]
fn production_list_nested_labels_share_one_first_fragment_without_double_height() {
    let mut value = production_list_fixture();
    value["document"]["blocks"][0]["blocks"][2]["items"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    production_body_renumber(&mut value["document"], &mut 0);
    value["page_masters"]["masters"][0]["body"]["width"] = 16_000_000.into();
    value["page_masters"]["masters"][0]["body"]["height"] = 3_000_000.into();
    production_body_set_style(&mut value, "list", "font_size", (32 * 65_536).into());
    with_production_marked_body(&value, &config(), |marked, _, _| {
        let selected = marked.structure().display().selected();
        let outer = &selected.list_markers()[0];
        let inner = &selected.list_markers()[1];
        assert_eq!(outer.fragment_index(), inner.fragment_index());
        assert_eq!(outer.baseline(), inner.baseline());
        assert!(outer.bounds().x() < inner.bounds().x());
        assert_eq!(outer.bounds().height(), inner.bounds().height());
        let page = &selected.pages()[outer.page_index() as usize];
        assert_eq!(page.fragment_count(), 1);
        assert_eq!(page.used_height(), outer.bounds().height().get());
    });
}
