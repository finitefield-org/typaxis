use super::*;
use typaxis_shaping::{
    book_v2::shape_book_v2_authored_text, ProductionParagraphLineContext,
    ProductionTextShapeErrorKind, ShapeSourceSpan,
};
const EPOCH: [u8; 32] = [19; 32];
fn source_data(text: &str) -> Value {
    let mut data = data();
    let end = text.len();
    let span = json!({"source_id":0,"start_byte":0,"end_byte":end});
    data["text_buffers"] = json!([{"text_id":0,"utf8":text,"mappings":[{"kind":"identity","source_span":span,"text_range":{"start_byte":0,"end_byte":end}}]}]);
    data["document"]["blocks"] = json!([{"kind":"semantic_container","node_id":1,"classes":[],"semantic_kind":"solution","anchor_id":null,"span":span,"blocks":[
        {"kind":"paragraph","node_id":2,"classes":[],"span":span,"children":[{"kind":"text","node_id":3,"span":span,"text_span":{"text_id":0,"start_byte":0,"end_byte":end}}]}
    ]}]);
    data
}
fn prepared(
    root: &Root,
    data: Value,
    source: &[u8],
    limits: &M4EffectiveResourceLimits,
) -> PreparedBookV2Resources {
    let body = body_with_source(root, data, source, limits);
    prepare_book_v2_resources(
        body,
        &root.context(),
        &config(limits.base().get().clone()),
        limits,
    )
    .unwrap()
}
#[test]
fn book_v2_body_shaping_uses_real_source_fonts_and_exact_owners() {
    let root = Root::new();
    let limits = limits();
    let prepared = prepared(&root, data(), SOURCE, &limits);
    let body = prepared.body();
    let admitted = prepared.resources();
    let nav = prepare_book_v2_navigation(body.styled()).unwrap();
    let flow = prepare_book_v2_text_flow(body.styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(body, &limits).unwrap();
    let shaped =
        shape_book_v2_authored_text(&policy, &flow, admitted, &limits, EPOCH, None).unwrap();
    assert_eq!(shaped.paragraphs().len(), 3);
    assert_eq!(shaped.font_instances().len(), 2);
    assert!(admitted.matches_declared_resources(flow.resource_declarations()));
    let mut wrong_media = flow.resource_declarations().clone();
    wrong_media.font_faces[0].media =
        typaxis_document::FontMediaDeclaration::Declared(typaxis_document::FontMediaType::SfntCff1);
    assert!(!admitted.matches_declared_resources(&wrong_media));
    let mut wrong_family = flow.resource_declarations().clone();
    wrong_family.font_faces[0].family = "Other".into();
    assert!(!admitted.matches_declared_resources(&wrong_family));

    let mut text = String::new();
    for paragraph in shaped.paragraphs() {
        assert_eq!(paragraph.font().unwrap().face_id(), FontFaceId::new(0));
        for run in paragraph.runs() {
            let ShapeSourceSpan::Parsed(span) = run.glyph_run().source_span else {
                panic!("source namespace")
            };
            text.push_str(
                &body.styled().body().wire().text_buffers()[span.text_id().get() as usize].utf8
                    [span.start_byte().get() as usize..span.end_byte().get() as usize],
            );
            assert!(run
                .glyph_run()
                .glyphs
                .iter()
                .all(|g| g.original_gid.get() != 0));
        }
    }
    assert_eq!(text, "ResultProofExercise");
    shaped.verify(&flow, admitted, &limits, EPOCH).unwrap();
    assert!(shaped.verify(&flow, admitted, &limits, [20; 32]).is_err());
    let nav2 = prepare_book_v2_navigation(body.styled()).unwrap();
    let flow2 = prepare_book_v2_text_flow(body.styled(), &nav2).unwrap();
    assert!(shaped.verify(&flow2, admitted, &limits, EPOCH).is_err());
    let other_root = Root::new();
    let other = prepared_fn(&other_root, data(), SOURCE, &limits);
    assert!(shaped
        .verify(&flow, other.resources(), &limits, EPOCH)
        .is_err());
    let other_policy = prepare_book_v2_resource_policy(other.body(), &limits).unwrap();
    assert!(
        shape_book_v2_authored_text(&other_policy, &flow, admitted, &limits, EPOCH, None).is_err()
    );
}
// Keep the constructor available when a test binds a local named `prepared`.
fn prepared_fn(
    root: &Root,
    data: Value,
    source: &[u8],
    limits: &M4EffectiveResourceLimits,
) -> PreparedBookV2Resources {
    prepared(root, data, source, limits)
}
#[test]
fn book_v2_body_reshape_checks_grapheme_boundaries_and_shared_record_limits() {
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, source_data("Result"), b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let initial =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
            .unwrap();
    let lines = [ProductionParagraphLineContext {
        owner: flow.paragraphs()[0].owner(),
        ends: &[3, 6],
    }];
    let shaped = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        EPOCH,
        Some(&lines),
    )
    .unwrap();
    assert_eq!(shaped.paragraphs()[0].runs().len(), 2);
    assert!(shaped.line_context_fingerprint().is_some());
    assert_ne!(shaped.fingerprint(), initial.fingerprint());
    let bad = [ProductionParagraphLineContext {
        owner: flow.paragraphs()[0].owner(),
        ends: &[7],
    }];
    assert_eq!(
        shape_book_v2_authored_text(
            &policy,
            &flow,
            input.resources(),
            &limits,
            EPOCH,
            Some(&bad)
        )
        .unwrap_err()
        .kind,
        ProductionTextShapeErrorKind::InvalidLineContext
    );
    for (max, success) in [
        (initial.output_records(), true),
        (initial.output_records() - 1, false),
    ] {
        let mut base = ResourceLimits::default();
        base.max_fragments = max;
        let limited = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(base).unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap();
        let r = Root::new();
        let i = prepared(&r, source_data("Result"), b"Result", &limited);
        let n = prepare_book_v2_navigation(i.body().styled()).unwrap();
        let f = prepare_book_v2_text_flow(i.body().styled(), &n).unwrap();
        let p = prepare_book_v2_resource_policy(i.body(), &limited).unwrap();
        let result = shape_book_v2_authored_text(&p, &f, i.resources(), &limited, EPOCH, None);
        assert_eq!(result.is_ok(), success);
        if !success {
            assert_eq!(
                result.unwrap_err().kind,
                ProductionTextShapeErrorKind::OutputLimit
            );
        }
    }
}
#[test]
fn book_v2_body_shapes_generated_list_and_footnote_labels() {
    let root = Root::new();
    let limits = limits();
    let mut d = source_data("Result");
    let span = d["document"]["blocks"][0]["span"].clone();
    let mut paragraph = d["document"]["blocks"][0]["blocks"][0].clone();
    paragraph["node_id"] = 4.into();
    paragraph["children"][0]["node_id"] = 5.into();
    paragraph["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"footnote_reference","node_id":6,"span":span,"footnote_id":"note"}));
    d["document"]["blocks"][0]["blocks"] = json!([{"kind":"list","node_id":2,"classes":[],"span":span,"ordered":true,"start":1,"items":[{"node_id":3,"span":span,"blocks":[paragraph]}]}]);
    paragraph["node_id"] = 8.into();
    paragraph["children"].as_array_mut().unwrap().truncate(1);
    paragraph["children"][0]["node_id"] = 9.into();
    d["document"]["footnotes"] =
        json!([{"node_id":7,"span":span,"footnote_id":"note","blocks":[paragraph]}]);
    let mut rule = d["style_sheet"]["rules"][2].clone();
    rule["selector"] = "list".into();
    rule["style_id"] = "list-text".into();
    rule["source_order"] = 3.into();
    d["style_sheet"]["rules"].as_array_mut().unwrap().push(rule);
    let input = prepared(&root, d, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let shaped =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
            .unwrap();
    assert_eq!(shaped.list_markers().len(), 1);
    assert_eq!(shaped.footnote_markers().len(), 1);
    assert_eq!(shaped.list_markers()[0].utf8(), "1.");
    assert_eq!(shaped.footnote_markers()[0].utf8(), "1");
    assert!(matches!(
        shaped.list_markers()[0].glyph_run().source_span,
        ShapeSourceSpan::Generated(_)
    ));
    assert!(matches!(
        shaped.footnote_markers()[0].glyph_run().source_span,
        ShapeSourceSpan::Generated(_)
    ));
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_body_shapes_original_harano_japanese_ivs_and_rejects_missing_sequences() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hash = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hash,
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    for (text, success) in [("日本語 日\u{e0100}", true), ("日\u{e01ef}", false)] {
        let root = Root::new();
        let limits = limits();
        let mut d = source_data(text);
        d["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        d["resources"]["font_faces"][0]["expected_sha256"] = hash.clone().into();
        let body = body_with_source(&root, d, text.as_bytes(), &limits);
        fs::write(root.0.join("body.bin"), &bytes).unwrap();
        let input = prepare_book_v2_resources(
            body,
            &root.context(),
            &config(ResourceLimits::default()),
            &limits,
        )
        .unwrap();
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let result =
            shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None);
        if !success {
            assert!(matches!(
                result.unwrap_err().kind,
                ProductionTextShapeErrorKind::CffV2(
                    typaxis_shaping::Cff1ShapeErrorV2::MissingCoverage { .. }
                )
            ));
            continue;
        }
        let shaped = result.unwrap();
        let font = input.resources().font(FontFaceId::new(0)).unwrap();
        let typaxis_resources::AdmittedProductionFontV3::Cff1V2(font) = font else {
            panic!("CFF /2 required")
        };
        let gid = font
            .admission()
            .cmap()
            .glyph_for_sequence('日', Some('\u{e0100}'))
            .unwrap();
        let mut saw_ivs = false;
        for run in shaped.paragraphs()[0].runs() {
            let glyphs = run.glyph_run();
            assert!(glyphs.glyphs.iter().all(|g| g.original_gid.get() != 0));
            for cluster in &glyphs.clusters {
                let ShapeSourceSpan::Parsed(span) = cluster.source_span else {
                    panic!("source namespace")
                };
                let slice = &text[span.start_byte().get() as usize..span.end_byte().get() as usize];
                if slice == "日\u{e0100}" {
                    assert!(glyphs.glyphs
                        [cluster.glyph_start as usize..cluster.glyph_end as usize]
                        .iter()
                        .any(|g| g.original_gid.get() == gid));
                    saw_ivs = true;
                }
            }
        }
        assert!(saw_ivs);
        let ends = [ProductionParagraphLineContext {
            owner: flow.paragraphs()[0].owner(),
            ends: &[13, 17],
        }];
        assert!(shape_book_v2_authored_text(
            &policy,
            &flow,
            input.resources(),
            &limits,
            EPOCH,
            Some(&ends)
        )
        .is_err());
    }
}
