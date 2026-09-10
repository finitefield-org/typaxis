fn production_native_display_fixture() -> serde_json::Value {
    fn find(value: &serde_json::Value) -> Option<serde_json::Value> {
        if value["kind"] == "display_math" {
            return Some(value.clone());
        }
        match value {
            serde_json::Value::Object(values) => values.values().find_map(find),
            serde_json::Value::Array(values) => values.iter().find_map(find),
            _ => None,
        }
    }
    let original: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json"
    ))).unwrap();
    let display = find(&original["document"]).unwrap();
    let mut value = production_native_math_fixture();
    let paragraph = value["document"]["blocks"][0].clone();
    // Following authored text uses an inserted source buffer at the display's
    // end, preserving monotonically ordered source spans.
    let end = display["span"]["end_byte"].clone();
    let point = serde_json::json!({"source_id":0,"start_byte":end,"end_byte":end});
    let mut following = paragraph.clone();
    following["span"] = point.clone();
    following["children"] = serde_json::json!([{"kind":"text","node_id":0,"span":point,
        "text_span":{"text_id":8,"start_byte":0,"end_byte":5}}]);
    value["document"]["blocks"] = serde_json::json!([paragraph, display, following]);
    value["page_masters"]["masters"][0]["footnote"] = serde_json::Value::Null;
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_native_display_uses_real_block_metrics_and_alignment() {
    for alignment in ["start", "center", "end"] {
        let mut value = production_native_display_fixture();
        production_body_set_style(&mut value, "display_math", "text_align", alignment.into());
        production_body_set_style(&mut value, "display_math", "start_indent", 196608.into());
        production_body_set_style(&mut value, "display_math", "end_indent", 327680.into());
        production_body_set_style(&mut value, "display_math", "line_height", 2_000_001.into());
        with_production_body_resources(&value, &config(), |lines, blocks, limits, admitted| {
            assert_eq!(lines.native_math_blocks().len(), 1);
            let math = &lines.native_math_blocks()[0];
            let receipt = lines.native_math_receipt(math.owner()).unwrap();
            assert_eq!(receipt.kind(), typaxis_math::MathNodeKind::Display);
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let fragment = selected
                .fragments()
                .iter()
                .find(|f| f.owner() == math.owner())
                .unwrap();
            assert_eq!(
                fragment.source(),
                typaxis_pagination::ProductionBodyFragmentSource::NativeMathBlock {
                    block_index: 0
                }
            );
            let body = blocks.page_geometry().body();
            assert_eq!(fragment.bounds().x().raw(), body.x().raw() + 196608);
            assert_eq!(
                fragment.bounds().width().get().raw(),
                body.width().get().raw() - 196608 - 327680
            );
            let dimensions = receipt.computation().dimensions();
            let width = dimensions.bbox().2.max(dimensions.advance()) - dimensions.bbox().0.min(0);
            let slack = fragment.bounds().width().get().raw() - width;
            let offset = match alignment {
                "start" => 0,
                "center" => slack / 2,
                _ => slack,
            };
            let viewport = fragment.viewport().unwrap();
            assert_eq!(viewport.x().raw(), fragment.bounds().x().raw() + offset);
            assert_eq!(viewport.width().get().raw(), width);
            let height = 2_000_001.max(dimensions.ascent() + dimensions.descent());
            let extra = height - dimensions.ascent() - dimensions.descent();
            let half_even = extra / 2 + i64::from(extra % 2 == 1 && (extra / 2) % 2 == 1);
            let baseline = viewport.y().raw() + half_even + dimensions.ascent();
            assert_eq!(fragment.bounds().height().get().raw(), height);
            assert_eq!(fragment.baseline().unwrap().raw(), baseline);
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let draw = display
                .draws()
                .iter()
                .find_map(|draw| match draw {
                    typaxis_display_list::ProductionBodyDraw::Math(draw)
                        if draw.owner() == math.owner() =>
                    {
                        Some(draw)
                    }
                    _ => None,
                })
                .unwrap();
            assert!(std::ptr::eq(draw.receipt(), receipt));
            assert_eq!(draw.source_span(), math.source_span());
            assert_eq!(
                draw.origin_x().raw(),
                viewport.x().raw() - dimensions.bbox().0.min(0)
            );
            assert_eq!(draw.baseline().raw(), baseline);
            assert_eq!(draw.paints().len(), receipt.computation().paints().len());
        });
    }
}

#[test]
fn production_native_display_keep_with_next_uses_shared_page_selection() {
    for keep in [false, true] {
        let mut value = production_native_display_fixture();
        for selector in ["paragraph", "display_math"] {
            production_body_set_style(&mut value, selector, "line_height", 2_000_000.into());
            production_body_set_style(&mut value, selector, "space_before", 0.into());
            production_body_set_style(&mut value, selector, "space_after", 0.into());
        }
        production_body_set_style(&mut value, "display_math", "keep_with_next", keep.into());
        value["page_masters"]["masters"][0]["body"]["height"] = 5_000_000.into();
        with_production_body_resources(&value, &config(), |lines, blocks, limits, _| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            assert_eq!(selected.pages().len(), 2);
            let counts: Vec<_> = selected
                .pages()
                .iter()
                .map(|p| p.fragment_count())
                .collect();
            assert_eq!(counts, if keep { vec![1, 2] } else { vec![2, 1] });
            let owner = lines.native_math_blocks()[0].owner();
            let math = selected
                .fragments()
                .iter()
                .find(|f| f.owner() == owner)
                .unwrap();
            assert_eq!(math.page_index(), u32::from(keep));
            if keep {
                assert_eq!(math.effective_space_before(), Length::ZERO);
            }
        });
    }
}

fn production_native_display_fraction_fixture() -> serde_json::Value {
    let mut value = production_native_math_fraction_fixture();
    let mut block = value["document"]["blocks"][0]["children"][0].clone();
    block["kind"] = "display_math".into();
    block["classes"] = serde_json::json!([]);
    value["document"]["blocks"] = serde_json::json!([block]);
    value["page_masters"]["masters"][0]["footnote"] = serde_json::Value::Null;
    // The public machine source profile requires a single source. Append the
    // fraction to the preserved original source instead of adding source 1.
    let original = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/profiles/production-book-1/combined/job/input.tsf"
    ));
    let tex = b"\\frac{x}{2}";
    let mut source = original.to_vec();
    source.extend_from_slice(tex);
    value["sources"].as_array_mut().unwrap().truncate(1);
    value["sources"][0]["utf8_byte_length"] = source.len().into();
    value["sources"][0]["sha256"] = sha256(&source)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    let span =
        serde_json::json!({"source_id":0,"start_byte":original.len(),"end_byte":source.len()});
    value["document"]["blocks"][0]["span"] = span.clone();
    value["text_buffers"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["mappings"][0]["source_span"] = span;
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

fn production_native_display_cases() -> Vec<(&'static str, serde_json::Value, usize)> {
    let mixed = production_native_display_fixture();
    let mut standalone = mixed.clone();
    standalone["document"]["blocks"] = serde_json::json!([mixed["document"]["blocks"][1]]);
    production_body_renumber(&mut standalone["document"], &mut 0);
    let mut listed = mixed.clone();
    let block = listed["document"]["blocks"][1].clone();
    let span = block["span"].clone();
    listed["document"]["blocks"][1] = serde_json::json!({"kind":"list","node_id":0,"span":span,
        "classes":[],"ordered":true,"start":1,"items":[{"node_id":0,"span":span,"blocks":[block]}]});
    production_body_renumber(&mut listed["document"], &mut 0);
    let mut footnote = mixed.clone();
    let block = footnote["document"]["blocks"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    let span = block["span"].clone();
    footnote["document"]["footnotes"] =
        serde_json::json!([{"node_id":0,"span":span,"footnote_id":"math-note","blocks":[block]}]);
    let paragraph = &mut footnote["document"]["blocks"][0];
    let end = paragraph["span"]["end_byte"].clone();
    paragraph["children"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"kind":"footnote_reference","node_id":0,
        "span":{"source_id":0,"start_byte":end,"end_byte":end},"footnote_id":"math-note"}));
    footnote["page_masters"]["masters"][0]["footnote"] =
        serde_json::json!({"x":955360,"y":13000000,"width":16000000,"height":6000000});
    production_body_renumber(&mut footnote["document"], &mut 0);
    vec![
        ("native-display", mixed, 2),
        ("native-display-only", standalone, 1),
        ("native-display-list", listed, 2),
        ("native-display-footnote", footnote, 2),
        (
            "native-display-fraction",
            production_native_display_fraction_fixture(),
            1,
        ),
    ]
}

// Track PDF graphics-state save/restore through the native ActualText scope.
// The font must still be active at EMC so independent extractors see nonzero
// glyph bounds instead of a zero-height replacement word.
fn assert_native_actual_text_font_active(content: &str, marker: &str) {
    let start = content.find(marker).unwrap() + marker.len();
    let mut font_active = false;
    let mut stack = Vec::new();
    for token in content[start..].split_ascii_whitespace() {
        match token {
            "q" => stack.push(font_active),
            "Q" => {
                font_active = stack
                    .pop()
                    .expect("native scope must not restore its outer state before EMC")
            }
            "Tf" => font_active = true,
            "EMC" => {
                assert!(font_active, "native ActualText lost its font at EMC");
                return;
            }
            _ => {}
        }
    }
    panic!("missing native ActualText terminator");
}

#[test]
fn production_native_display_public_pdf_retains_source_order_and_formula_structure() {
    for (name, value, count) in production_native_display_cases() {
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
        let built = build_production_book_pdf(
            &package,
            &navigation,
            &semantics,
            &profile,
            &admitted,
            &limits,
            &cfg,
        )
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let (pdf, _, selected, _, fragments, _, _) = built.into_parts();
        assert_eq!(selected, pdf.selected_layout_fingerprint().bytes());
        assert!(fragments >= count as u64);
        let content = String::from_utf8_lossy(pdf.bytes());
        assert_eq!(content.matches("/S /Formula").count(), count, "{name}");
        let inline: String = "x squared"
            .encode_utf16()
            .map(|c| format!("{c:04X}"))
            .collect();
        let speech = if name == "native-display-fraction" {
            "x divided by two"
        } else {
            "x plus one"
        };
        let block: String = speech.encode_utf16().map(|c| format!("{c:04X}")).collect();
        let inline = format!("/ActualText <FEFF{inline}>");
        let block = format!("/ActualText <FEFF{block}>");
        assert!(content.contains(&block), "{name}");
        assert_native_actual_text_font_active(&content, &block);
        if count > 1 {
            assert_native_actual_text_font_active(&content, &inline);
        }
        if name == "native-display" {
            let first = content.find(&inline).unwrap();
            let middle =
                content[first + inline.len()..].find(&block).unwrap() + first + inline.len();
            assert!(middle > first);
            assert_eq!(fragments, 3);
        }
        if let Ok(directory) = std::env::var("VMB_NATIVE_DISPLAY_OUT") {
            let directory = PathBuf::from(directory);
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join(format!("{name}.pdf")), pdf.bytes()).unwrap();
            fs::write(directory.join(format!("{name}-package.json")), &bytes).unwrap();
        }
    }
}

#[test]
fn production_native_display_rejects_oversize_without_scaling_or_splitting() {
    let mut value = production_native_display_cases().remove(1).1;
    let mut measured = None;
    with_production_body_resources(&value, &config(), |lines, _, _, _| {
        let block = &lines.native_math_blocks()[0];
        measured = Some((block.width().get().raw(), block.height().get().raw()));
    });
    let (width, height) = measured.unwrap();
    for (available_width, available_height, expected) in [
        (width, height, None),
        (
            width - 1,
            height,
            Some(typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch),
        ),
        (
            width,
            height - 1,
            Some(typaxis_pagination::ProductionBodyPaginationErrorKind::Oversize),
        ),
    ] {
        value["page_masters"]["masters"][0]["body"]["width"] = available_width.into();
        value["page_masters"]["masters"][0]["body"]["height"] = available_height.into();
        with_production_body_resources(&value, &config(), |lines, blocks, limits, _| {
            match typaxis_pagination::paginate_production_body(lines, blocks, limits) {
                Ok(selected) => {
                    assert!(expected.is_none());
                    assert_eq!(selected.fragments().len(), 1);
                    let viewport = selected.fragments()[0].viewport().unwrap();
                    assert_eq!(viewport.width().get().raw(), width);
                    assert_eq!(viewport.height().get().raw(), height);
                    assert_eq!(
                        selected.fragments()[0].effective_space_before(),
                        Length::ZERO
                    );
                }
                Err(error) => assert_eq!(Some(error.kind), expected),
            }
        });
    }
}

#[test]
fn production_native_display_fraction_preserves_intrinsic_height_and_rule() {
    let value = production_native_display_fraction_fixture();
    with_production_body_resources(&value, &config(), |lines, blocks, limits, admitted| {
        let block = &lines.native_math_blocks()[0];
        let receipt = lines.native_math_receipt(block.owner()).unwrap();
        let dimensions = receipt.computation().dimensions();
        assert!(dimensions.ascent() + dimensions.descent() > 1_048_576);
        assert_eq!(
            block.height().get().raw(),
            dimensions.ascent() + dimensions.descent()
        );
        assert_eq!(block.baseline().raw(), dimensions.ascent());
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        let display =
            typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                .unwrap();
        let typaxis_display_list::ProductionBodyDraw::Math(draw) = &display.draws()[0] else {
            panic!("native display paint");
        };
        assert_eq!(draw.paints().len(), receipt.computation().paints().len());
        let mut count = 0;
        for (paint, source) in draw.paints().iter().zip(receipt.computation().paints()) {
            if let (
                typaxis_display_list::ProductionNativeMathPaint::Rule(rule),
                typaxis_math::MathPaint::Rule(original),
            ) = (paint, source)
            {
                count += 1;
                assert_eq!(rule.x().raw(), draw.origin_x().raw() + original.x());
                assert_eq!(rule.y().raw(), draw.baseline().raw() + original.y());
                assert_eq!(rule.width().get().raw(), original.width());
                assert_eq!(rule.height().get().raw(), original.height());
            }
        }
        assert_eq!(count, 1);
    });
}
