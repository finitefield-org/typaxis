//! Public shape/inline projections captured before the streaming codec change.
use super::*;
use typaxis_layout::book_v2::prepare_book_v2_text_inlines;
use typaxis_linebreak::JapaneseLineBreakMode;
use typaxis_shaping::{
    book_v2::{shape_book_v2_authored_text_counted, BookV2AuthoredTextShape},
    GlyphRun,
};
use typaxis_syntax::book_v2::prepare_book_v2_text_flow_with_page_references;

#[test]
fn book_v2_text_fingerprints_preserve_frozen_tt_projections() {
    check(None);
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_text_fingerprints_preserve_frozen_original_harano_projections() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        hex(typaxis_core::sha256(&bytes)),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    check(Some(&bytes));
}

fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn run_snapshot(run: &GlyphRun) -> Value {
    json!({"run_id":run.run_id.get(),"font":run.font.get(),
        "level":run.bidi_level.get(),"source":format!("{:?}", run.source_span),
        "glyph_count":run.glyphs.len(),"cluster_count":run.clusters.len(),
        "all_fields_sha256":hex(typaxis_core::sha256(format!("{run:?}").as_bytes()))})
}

fn shape_snapshot(shape: &BookV2AuthoredTextShape<'_>) -> Value {
    json!({
        "fingerprint":hex(shape.fingerprint()),"records":shape.output_records(),
        "context":shape.line_context_fingerprint().map(hex),
        "paragraphs":shape.paragraphs().iter().map(|p| json!({
            "owner":p.owner().get(),"font":format!("{:?}",p.font()),
            "level":p.paragraph_level().get(),"fingerprint":hex(p.fingerprint()),
            "pending":p.pending_references().iter().map(|n|n.get()).collect::<Vec<_>>(),
            "runs":p.runs().iter().map(|r|json!({"owner":r.owner().get(),
                "site":r.site_index(),"language":r.language(),"script":r.script().bytes(),
                "run":run_snapshot(r.glyph_run())})).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "lists":shape.list_markers().iter().map(|m|json!({
            "owner":m.source().owner().get(),"index":m.marker_index(),"utf8":m.utf8(),
            "provenance":format!("{:?}",m.provenance()),"font":format!("{:?}",m.font()),
            "advance":m.advance().get().raw(),"fingerprint":hex(m.fingerprint()),
            "run":run_snapshot(m.glyph_run())})).collect::<Vec<_>>(),
        "notes":shape.footnote_markers().iter().map(|m|json!({
            "owner":m.source().owner().get(),"utf8":m.utf8(),
            "provenance":format!("{:?}",m.provenance()),"font":format!("{:?}",m.font()),
            "advance":m.advance().get().raw(),"fingerprint":hex(m.fingerprint()),
            "run":run_snapshot(m.glyph_run())})).collect::<Vec<_>>()
    })
}

fn fixture(case: &str, text: &str) -> Value {
    let mut data = super::body_line_budget::wide_data(text);
    let span = data["document"]["blocks"][0]["span"].clone();
    let mut paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    match case {
        "empty" => paragraph["children"] = json!([]),
        "controls" => {
            paragraph["children"].as_array_mut().unwrap().extend([
                json!({"kind":"soft_break","node_id":0,"span":span}),
                json!({"kind":"hard_break","node_id":0,"span":span}),
            ]);
        }
        "references" => {
            let mut heading = paragraph.clone();
            heading["kind"] = "heading".into();
            heading["level"] = 2.into();
            heading["anchor_id"] = "chapter".into();
            paragraph["children"].as_array_mut().unwrap().extend([
                json!({"kind":"reference","node_id":0,"span":span,"target":"chapter","format":"text"}),
                json!({"kind":"reference","node_id":0,"span":span,"target":"chapter","format":"page"}),
            ]);
            data["document"]["blocks"][0]["blocks"] = json!([heading, paragraph]);
            let mut rule = data["style_sheet"]["rules"][2].clone();
            rule["selector"] = "heading".into();
            rule["style_id"] = "heading-text".into();
            rule["source_order"] = 3.into();
            data["style_sheet"]["rules"]
                .as_array_mut()
                .unwrap()
                .push(rule);
            super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
            return data;
        }
        "generated" => {
            let definition = paragraph.clone();
            paragraph["children"].as_array_mut().unwrap().push(json!({
                "kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"note"
            }));
            data["document"]["blocks"][0]["blocks"] = json!([{
                "kind":"list","node_id":0,"classes":[],"span":span,"ordered":true,"start":1,
                "items":[{"node_id":0,"span":span,"blocks":[paragraph]}]
            }]);
            data["document"]["footnotes"] = json!([{
                "node_id":0,"span":span,"footnote_id":"note","blocks":[definition]
            }]);
            let mut rule = data["style_sheet"]["rules"][2].clone();
            rule["selector"] = "list".into();
            rule["style_id"] = "list-text".into();
            rule["source_order"] = 3.into();
            data["style_sheet"]["rules"]
                .as_array_mut()
                .unwrap()
                .push(rule);
            super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
            return data;
        }
        "body" | "large" | "backend-bound" => (),
        _ => panic!("unknown fixture"),
    }
    data["document"]["blocks"][0]["blocks"] = json!([paragraph]);
    super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
    data
}

fn check(font: Option<&[u8]>) {
    let name = if font.is_some() { "harano" } else { "tt" };
    let word = if font.is_some() { "本文" } else { "Result" };
    let limits = limits();
    let mut cases = serde_json::Map::new();
    for case in [
        "body",
        "controls",
        "empty",
        "generated",
        "references",
        "large",
        "backend-bound",
    ] {
        let text = match case {
            "large" => word.repeat(128),
            "backend-bound" => word.repeat(
                limits.base().get().max_shaping_context_bytes as usize / 64 / word.chars().count()
                    + 1,
            ),
            _ => word.into(),
        };
        let root = Root::new();
        let data = fixture(case, &text);
        let page_owner = (case == "references").then(|| {
            typaxis_core::NodeId::new(
                data["document"]["blocks"][0]["blocks"][1]["children"][2]["node_id"]
                    .as_u64()
                    .unwrap() as u32,
            )
        });
        let input = if let Some(font) = font {
            vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
        } else {
            prepared(&root, data, text.as_bytes(), &limits)
        };
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let values = page_owner.map(|n| [(n, 12)]);
        let flow = match &values {
            Some(values) => {
                prepare_book_v2_text_flow_with_page_references(input.body().styled(), &nav, values)
            }
            None => prepare_book_v2_text_flow(input.body().styled(), &nav),
        }
        .unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let ends = text
            .char_indices()
            .map(|(n, c)| (n + c.len_utf8()) as u32)
            .collect::<Vec<_>>();
        // Split only the body/large parsed text. Generated/reference and explicit
        // controls retain their actual initial context in this frozen comparison.
        for contextual in [false, true]
            .into_iter()
            .filter(|v| !*v || matches!(case, "body" | "large"))
        {
            let contexts = [ProductionParagraphLineContext {
                owner: flow.paragraphs()[0].owner(),
                ends: &ends,
            }];
            let mut observed = u64::MAX;
            let result = shape_book_v2_authored_text_counted(
                &policy,
                &flow,
                input.resources(),
                &limits,
                EPOCH,
                contextual.then_some(contexts.as_slice()),
                &mut observed,
            );
            if case == "backend-bound" {
                let cause = result.unwrap_err();
                assert_eq!(cause.kind, ProductionTextShapeErrorKind::ContextLimit);
                cases.insert(case.into(),json!({"owner":cause.owner.get(),"kind":format!("{:?}",cause.kind),"records":observed}));
                continue;
            }
            let shape = result.unwrap();
            assert_eq!(observed, shape.output_records());
            let inline = prepare_book_v2_text_inlines(
                &flow,
                &shape,
                input.resources(),
                &limits,
                EPOCH,
                JapaneseLineBreakMode::Normal,
            )
            .unwrap();
            let prepared = inline.paragraphs().iter().map(|p| json!({"owner":p.owner().get(),
                "items":p.items().map(|items| json!({"fingerprint":hex(items.fingerprint()),
                    "unit_count":items.units().len(),"cluster_count":items.clusters().len(),
                    "units_sha256":hex(typaxis_core::sha256(format!("{:?}",items.units()).as_bytes())),
                    "clusters_sha256":hex(typaxis_core::sha256(format!("{:?}",items.clusters()).as_bytes()))}))})).collect::<Vec<_>>();
            cases.insert(format!("{case}-{}",if contextual {"context"} else {"initial"}),
                json!({"shape":shape_snapshot(&shape),"inline":hex(inline.fingerprint()),"prepared":prepared}));
        }
    }
    let actual = Value::Object(cases);
    if let Ok(path) = std::env::var("TYPAXIS_BOOK_V2_TEXT_FINGERPRINT_CAPTURE") {
        fs::create_dir_all(&path).unwrap();
        fs::write(
            std::path::Path::new(&path).join(format!("{name}.json")),
            serde_json::to_vec_pretty(&actual).unwrap(),
        )
        .unwrap();
    } else {
        let frozen: Value =
            serde_json::from_str(include_str!("book_v2_text_fingerprint_golden.json")).unwrap();
        assert_eq!(
            frozen["production_commit"],
            "83fedd490801029a3a08e972a8d982fcc6943ad9"
        );
        let expected = frozen["cases"][name].as_object().unwrap();
        assert_eq!(
            actual.as_object().unwrap().keys().collect::<Vec<_>>(),
            expected.keys().collect::<Vec<_>>()
        );
        for (case, value) in actual.as_object().unwrap() {
            assert_eq!(
                hex(typaxis_core::sha256(&serde_json::to_vec(value).unwrap())),
                expected[case],
                "complete public shape/inline projection changed for {name}/{case}"
            );
        }
    }
}
