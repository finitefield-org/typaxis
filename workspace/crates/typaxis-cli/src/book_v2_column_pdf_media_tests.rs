use super::*;

#[derive(Debug)]
struct MediaFacts {
    pages: usize,
    regions: usize,
    native: usize,
    rules: usize,
    vector: usize,
    images: usize,
    raster: usize,
    numbers: usize,
    notes: usize,
    repeated: usize,
    balanced: usize,
    anchor_font: bool,
    columns: BTreeSet<usize>,
    header_gids: BTreeSet<u16>,
    footer_gids: BTreeSet<u16>,
    region_faces: BTreeSet<u32>,
}

// New running paragraphs borrow an actual original text slice. Every original
// document node and every original body/note frame is retained byte for byte.
fn with_running_text(mut data: Value) -> Value {
    fn paragraph(value: &Value) -> Option<&Value> {
        if matches!(value["kind"].as_str(), Some("paragraph" | "heading"))
            && value["children"]
                .as_array()
                .is_some_and(|children| children.iter().any(|c| c["kind"] == "text"))
        {
            return Some(value);
        }
        match value {
            Value::Array(items) => items.iter().find_map(paragraph),
            Value::Object(items) => items.values().find_map(paragraph),
            _ => None,
        }
    }
    fn maximum(value: &Value) -> u32 {
        let own = value.get("node_id").and_then(Value::as_u64).unwrap_or(0) as u32;
        match value {
            Value::Array(items) => own.max(items.iter().map(maximum).max().unwrap_or(0)),
            Value::Object(items) => own.max(items.values().map(maximum).max().unwrap_or(0)),
            _ => own,
        }
    }
    let original = data["document"].clone();
    let mut p = paragraph(&original)
        .expect("original authored text paragraph")
        .clone();
    let mut text = p["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "text")
        .unwrap()
        .clone();
    let span = &mut text["text_span"];
    let buffer = data["text_buffers"][span["text_id"].as_u64().unwrap() as usize]["utf8"]
        .as_str()
        .unwrap();
    let start = span["start_byte"].as_u64().unwrap() as usize;
    let end = span["end_byte"].as_u64().unwrap() as usize;
    let authored = &buffer[start..end];
    let count = authored
        .char_indices()
        .nth(8)
        .map_or(authored.len(), |(i, _)| i);
    span["end_byte"] = (start + count).into();
    p["kind"] = "paragraph".into();
    p.as_object_mut().unwrap().remove("level");
    p.as_object_mut().unwrap().remove("anchor_id");
    p["classes"] = json!([]);
    p["children"] = json!([text]);
    let span = p["span"].clone();
    let mut next = maximum(&original) + 1;
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        let body_right =
            master["body"]["x"].as_i64().unwrap() + master["body"]["width"].as_i64().unwrap();
        let note_right = master["footnote"].as_object().map_or(0, |_| {
            master["footnote"]["x"].as_i64().unwrap()
                + master["footnote"]["width"].as_i64().unwrap()
        });
        let x = body_right.max(note_right) + 10 * 65536;
        let width = 160 * 65536;
        if master["width"].as_i64().unwrap() < x + width + 10 * 65536 {
            master["width"] = (x + width + 10 * 65536).into();
            master["trim"]["width"] = master["width"].clone();
        }
        let height = master["height"].as_i64().unwrap();
        let region_height = ((height - 10 * 65536) / 2).min(80 * 65536);
        assert!(region_height > 0);
        master["header"] = json!({"x":x,"y":5*65536,"width":width,"height":region_height});
        master["footer"] =
            json!({"x":x,"y":height-5*65536-region_height,"width":width,"height":region_height});
        for role in ["header_content", "footer_content"] {
            master[role] = json!({"node_id":0,"span":span,"blocks":[p.clone()]});
            semantic_renumber(&mut master[role], &mut next);
        }
    }
    assert_eq!(data["document"], original);
    data
}

fn actual_pdf(
    case: &str,
    input: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
) -> MediaFacts {
    let mut budget = BookV2ColumnPageBudget::new(caps, MAXIMUM);
    let (facts, pending) = pdf_drive(
        input,
        caps,
        MODE,
        &mut budget,
        4,
        |bundle, builder, observed| {
            let display = bundle.display();
            let pages = display.source().source().geometry().pages();
            let mut native = 0;
            let mut rules = 0;
            let mut vector = 0;
            for draw in display.math().draws() {
                match draw.paint() {
                    BookV2MathPaint::Native(n) => {
                        native += 1;
                        rules += n
                            .paints()
                            .iter()
                            .filter(|p| matches!(p, ProductionNativeMathPaint::Rule(_)))
                            .count();
                    }
                    BookV2MathPaint::Vector(_) => vector += 1,
                }
            }
            let marked = bundle.navigation().source().source().source();
            let facts = MediaFacts {
                pages: pages.len(),
                regions: display.page_regions().len(),
                native,
                rules,
                vector,
                images: display.images().draws().len(),
                raster: display
                    .images()
                    .draws()
                    .iter()
                    .filter(|i| matches!(i.paint(), BookV2ImagePaint::Raster { .. }))
                    .count(),
                numbers: display.numbers().draws().len(),
                notes: display
                    .text()
                    .draws()
                    .iter()
                    .filter(|t| t.definition_index().is_some())
                    .count(),
                repeated: display
                    .text()
                    .draws()
                    .iter()
                    .filter(|t| t.repeated_header())
                    .count()
                    + display
                        .math()
                        .draws()
                        .iter()
                        .filter(|t| t.terminal().repeated_header())
                        .count(),
                balanced: pages
                    .iter()
                    .filter(|p| p.selection().balance().is_some())
                    .count(),
                anchor_font: marked.anchor_font_object().is_some(),
                columns: pages.iter().map(|p| p.columns().len()).collect(),
                header_gids: display
                    .page_regions()
                    .iter()
                    .filter(|r| r.kind() == RegionKind::Header)
                    .flat_map(|r| r.draws())
                    .flat_map(|d| d.glyphs())
                    .map(|g| g.original_gid().get())
                    .collect(),
                footer_gids: display
                    .page_regions()
                    .iter()
                    .filter(|r| r.kind() == RegionKind::Footer)
                    .flat_map(|r| r.draws())
                    .flat_map(|d| d.glyphs())
                    .map(|g| g.original_gid().get())
                    .collect(),
                region_faces: display
                    .page_regions()
                    .iter()
                    .flat_map(|r| r.draws())
                    .map(|d| d.font().face_id().get())
                    .collect(),
            };
            assert!(facts.pages > 0);
            assert_eq!(facts.regions, facts.pages * 2);
            assert!(display.page_regions().iter().all(|r| !r.draws().is_empty()));
            assert!(
                bundle.pdf().bytes().starts_with(b"%PDF-1.7\n")
                    && bundle.pdf().bytes().ends_with(b"%%EOF\n")
            );
            assert_eq!(bundle.pdf().page_references().len(), 0);
            let names: Vec<_> = pages
                .iter()
                .enumerate()
                .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
                .collect();
            let epoch = display
                .source()
                .original_graph()
                .lines()
                .prepared()
                .shaped()
                .binding_epoch();
            let (_, local) = original_regions(input, caps, epoch, &names);
            for (region, proof) in display.page_regions().iter().zip(&local) {
                assert_projection(region, proof);
            }
            let pending = pdf_probes::emit(
                case,
                bundle,
                caps,
                observed,
                &local,
                builder.reservation_sizes(),
            );
            (facts, pending)
        },
    )
    .unwrap_or_else(|error| panic!("{case}: {error:?}"));
    pdf_probes::finish(pending, budget.observation());
    facts
}

#[test]
fn book_v2_column_pdf_media_running_regions_keep_native_vector_raster_numbers_and_original_notes() {
    let caps = display_caps();
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        for balanced in [false, true] {
            let mut data = crate::book_v2_resources::tests::shaping_tests::frames::table_breaks::mixed::pages::stability::image_anchor_roles::converged_pdf::block_widths::column_block_data(kind, false, false);
            anchors(&mut data);
            balance(&mut data, balanced);
            let data = with_running_text(data);
            let root = Root::new();
            let input = match kind {
                "native" => {
                    let text = data["text_buffers"][0]["utf8"]
                        .as_str()
                        .unwrap()
                        .as_bytes()
                        .to_vec();
                    native_input_with_source(&root, data, &text, &caps)
                }
                "vector" => vector_input(&root, data, &caps),
                "png" => figure_input(&root, data, PNG, &caps),
                "jpeg" => figure_input(&root, data, JPEG, &caps),
                "svg" => figure_input(&root, data, SVG, &caps),
                _ => unreachable!(),
            };
            let facts = actual_pdf(
                &format!("column-pdf-running-media-{kind}-{balanced}"),
                &input,
                &caps,
            );
            assert_eq!(facts.balanced > 0, balanced);
            match kind {
                "native" => assert!(facts.native > 0),
                "vector" => assert!(facts.vector > 0),
                "png" | "jpeg" => assert!(facts.raster > 0),
                _ => assert!(facts.images > 0),
            }
            assert_eq!(facts.anchor_font, kind == "vector");
        }
    }
    for balanced in [false, true] {
        let mut data = balanced_six_stream_data();
        balance(&mut data, balanced);
        let data = with_running_text(data);
        let root = Root::new();
        let input = vector_input(&root, data, &caps);
        let facts = actual_pdf(
            &format!("column-pdf-running-six-stream-notes-{balanced}"),
            &input,
            &caps,
        );
        assert!(
            facts.vector > 0
                && facts.images > 0
                && facts.numbers > 0
                && facts.notes > 0
                && facts.anchor_font
        );
        assert_eq!(facts.balanced > 0, balanced);
    }
}

#[test]
fn book_v2_column_pdf_media_running_regions_keep_actual_fraction_rules_repeated_headers_and_named_columns(
) {
    let caps = display_caps();
    for repeated in [false, true] {
        let data = with_running_text(fraction_data(repeated));
        let text = data["text_buffers"][0]["utf8"]
            .as_str()
            .unwrap()
            .as_bytes()
            .to_vec();
        let root = Root::new();
        let input = native_input_with_source(&root, data, &text, &caps);
        let facts = actual_pdf(
            &format!("column-pdf-running-native-fraction-{repeated}"),
            &input,
            &caps,
        );
        assert!(facts.native > 0 && facts.rules > 0);
        assert_eq!(facts.repeated > 0, repeated);
    }
    for vector in [false, true] {
        for notes in [false, true] {
            let data = with_running_text(header_data(vector, notes));
            let root = Root::new();
            let input = if vector {
                vector_input(&root, data, &caps)
            } else {
                let text = data["text_buffers"][0]["utf8"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .to_vec();
                native_input_with_source(&root, data, &text, &caps)
            };
            let facts = actual_pdf(
                &format!("column-pdf-running-header-{vector}-{notes}"),
                &input,
                &caps,
            );
            assert!(facts.repeated > 0 && facts.pages > 1);
            assert_eq!(facts.notes > 0, notes);
        }
    }
    for blanks in [1, 2] {
        for balanced in [false, true] {
            let data = with_running_text(named_display_data(
                "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro",
                blanks,
                balanced,
            ));
            let root = Root::new();
            let input = display_input(&root, data, None, &caps);
            let facts = actual_pdf(
                &format!("column-pdf-running-named-{blanks}-{balanced}"),
                &input,
                &caps,
            );
            assert!(facts.regions > 0);
            assert!(facts.columns.contains(&(if blanks == 1 { 3 } else { 2 })));
            assert_eq!(facts.balanced > 0, balanced);
        }
    }
}

#[test]
fn book_v2_column_pdf_media_running_context_and_collection_faces_use_original_selected_programs() {
    const CONTEXT_FONT:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/vmb-book/header-context-font/header-context.ttf"));
    let caps = display_caps();
    let mut data = crate::book_v2_resources::tests::shaping_tests::page_regions::region_data("f i");
    let p = data["document"]["blocks"][0]["blocks"][0].clone();
    data["document"]["blocks"][0]["blocks"] = vec![p; 8].into();
    let m = &mut data["page_masters"]["masters"][0];
    m["body"]["y"] = (120 * 65536).into();
    m["body"]["height"] = (48 * 65536).into();
    m["column_layout"] = json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"none"});
    m["header"]["width"] = (14 * 65536).into();
    m["header"]["height"] = (96 * 65536).into();
    m["footer"]["width"] = (80 * 65536).into();
    m["footer"]["height"] = (60 * 65536).into();
    let mut next = 0;
    semantic_renumber(&mut data["document"], &mut next);
    for role in ["header_content", "footer_content"] {
        semantic_renumber(&mut data["page_masters"]["masters"][0][role], &mut next);
    }
    let root = Root::new();
    let input = display_input(&root, data, Some(CONTEXT_FONT), &caps);
    let facts = actual_pdf("column-pdf-running-context-original-gid71", &input, &caps);
    assert!(facts.header_gids.contains(&71) && !facts.header_gids.contains(&59));
    assert!(facts.footer_gids.contains(&59) && !facts.footer_gids.contains(&71));
    assert_eq!(facts.region_faces, [0].into_iter().collect());

    let mut data = region_data(2, true, false);
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        for role in ["header_content", "footer_content"] {
            for p in master[role]["blocks"].as_array_mut().unwrap() {
                p["classes"] = json!(["running-collection"]);
            }
        }
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    for kind in ["paragraph", "heading"] {
        rules.push(json!({"style_id":format!("region-font-{kind}"),"selector":format!("{kind}.running-collection"),"source_order":rules.len(),"extends":null,"declarations":[{"name":"font_family","important":false,"value":{"kind":"font_family_list","families":["Collection"]}}]}));
    }
    let root = Root::new();
    let input = display_input(&root, data, None, &caps);
    let facts = actual_pdf("column-pdf-running-original-collection-face", &input, &caps);
    assert_eq!(facts.region_faces, [1].into_iter().collect());
}

#[test]
fn book_v2_column_pdf_media_semantic_refusals_and_invalid_first_objects_never_deliver_or_refund_work(
) {
    let caps = display_caps();
    for conflict in [false, true] {
        let data = with_running_text(semantic_refusal_data(conflict));
        let root = Root::new();
        let input = display_input(&root, data, Some(DISPLAY_LIST_FONT), &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let mut delivered = false;
        let error = pdf_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| {
            delivered = true;
        })
        .unwrap_err();
        assert!(!delivered);
        let RegionError::Stage { stage, source } = error else {
            panic!("semantic stage refusal")
        };
        assert_eq!(stage, "column navigation geometry");
        let error = source.downcast_ref::<NavigationError>().unwrap();
        assert!(matches!(
            (conflict, error),
            (true, NavigationError::ConflictingLinks(_))
                | (false, NavigationError::UnplacedDestination(_))
        ));
        let prefix = budget.observation();
        assert!(prefix.record_charge() > 0 && prefix.spool_charge() > 0 && prefix.work_steps() > 0);
        assert!(pdf_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| {
            delivered = true;
        })
        .is_err());
        assert!(!delivered);
        let retry = budget.observation();
        assert!(
            retry.record_charge() > prefix.record_charge()
                && retry.spool_charge() > prefix.spool_charge()
                && retry.work_steps() > prefix.work_steps()
        );
        assert!(retry.output_charge() <= caps.base().get().max_output_bytes);
    }
    let root = Root::new();
    let input = display_input(&root, region_data(2, false, false), None, &caps);
    for first in [0, u32::MAX] {
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        assert!(
            pdf_drive(&input, &caps, MODE, &mut budget, first, |_, _, _| panic!(
                "invalid object origin delivered"
            ))
            .is_err()
        );
        assert!(budget.observation().record_charge() > 0 && budget.observation().work_steps() > 0);
    }
}

#[test]
fn book_v2_column_pdf_media_default_and_authored_metadata_keep_xml_escapes_utf16_surrogates_and_dates(
) {
    let caps = display_caps();
    for authored in [false, true] {
        let mut data = region_data(3, true, false);
        if authored {
            data["metadata"] = json!({"title":"Column <&> \"' 😀","author":"A & B <編集> 🧪","subject":"Exact <source> & geometry 😀","keywords":["<three>","one & two","😀"],"identifier":"urn:typaxis:column:<&>:😀","created":"2026-10-15T01:02:03Z","modified":"2026-10-16T04:05:06Z"});
        }
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let pending = pdf_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, builder, observed| {
                let bytes = bundle.pdf().bytes();
                let contains = |needle: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);
                assert_eq!(contains(b" /Title "), authored);
                assert_eq!(contains(b" /Author "), authored);
                assert_eq!(contains(b" /CreationDate "), authored);
                if authored {
                    assert!(contains(b"D83DDE00") && contains(b"D83EDDEA"));
                    assert!(contains(b"(D:20261015010203Z)") && contains(b"(D:20261016040506Z)"));
                    assert!(contains(b"&lt;") && contains(b"&amp;") && contains("😀".as_bytes()));
                }
                let d = bundle.display();
                let pages = d.source().source().geometry().pages();
                let names: Vec<_> = pages
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
                    .collect();
                let (_, local) = original_regions(
                    &input,
                    &caps,
                    d.source()
                        .original_graph()
                        .lines()
                        .prepared()
                        .shaped()
                        .binding_epoch(),
                    &names,
                );
                pdf_probes::emit(
                    &format!("column-pdf-running-metadata-{authored}"),
                    bundle,
                    &caps,
                    observed,
                    &local,
                    builder.reservation_sizes(),
                )
            },
        )
        .unwrap();
        pdf_probes::finish(pending, budget.observation());
    }
}
