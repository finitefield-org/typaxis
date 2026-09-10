use super::*;
use typaxis_core::NodeId;
use typaxis_shaping::book_v2::shape_book_v2_page_region_text as shape_region;
use typaxis_syntax::book_v2::{
    prepare_book_v2_page_region_text_flow as region_flow, select_book_v2_page_master,
    BookV2PageRegionKind as Kind,
};

fn region_data(text: &str) -> Value {
    let mut data = source_data(text);
    let span = data["document"]["blocks"][0]["span"].clone();
    let paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    let mut header = paragraph.clone();
    header["node_id"] = 5.into();
    header["children"][0]["node_id"] = 6.into();
    header["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"soft_break","node_id":7,"span":span}));
    let mut duplicate = header["children"][0].clone();
    duplicate["node_id"] = 8.into();
    header["children"].as_array_mut().unwrap().push(duplicate);
    header["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"hard_break","node_id":9,"span":span}));
    let mut footer = paragraph;
    footer["kind"] = "heading".into();
    footer["level"] = 2.into();
    footer["anchor_id"] = Value::Null;
    footer["node_id"] = 11.into();
    footer["children"][0]["node_id"] = 12.into();
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    let mut heading = rules
        .iter()
        .find(|r| r["selector"] == "paragraph")
        .unwrap()
        .clone();
    heading["style_id"] = "running-heading".into();
    heading["selector"] = "heading".into();
    heading["source_order"] = rules.len().into();
    rules.push(heading);
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = (400 * 65536).into();
    master["height"] = (600 * 65536).into();
    master["trim"] = json!({"x":0,"y":0,"width":400*65536,"height":600*65536});
    master["body"] = json!({"x":20*65536,"y":60*65536,"width":360*65536,"height":400*65536});

    master["header"] = json!({"x":10*65536,"y":0,"width":200*65536,"height":30*65536});
    master["footer"] = json!({"x":10*65536,"y":500*65536,"width":200*65536,"height":30*65536});
    master["header_content"] = json!({"node_id":4,"span":span,"blocks":[header]});
    master["footer_content"] = json!({"node_id":10,"span":span,"blocks":[footer]});
    data
}

#[test]
fn book_v2_page_region_flow_preserves_originals_and_separates_body_authority() {
    check(None, "Result");
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_page_region_flow_shapes_original_harano_japanese() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&font), "本文の柱");
}
fn check(font: Option<&[u8]>, text: &str) {
    let root = Root::new();
    let limits = limits();
    let data = region_data(text);
    let input = if let Some(font) = font {
        vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
    } else {
        prepared(&root, data, text.as_bytes(), &limits)
    };
    let body = input.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let mut work = 0;
    let selected = select_book_v2_page_master(body, 0, None, &mut work, 1_000_000).unwrap();
    let header = region_flow(selected, Kind::Header, &nav, 0).unwrap();
    let footer = region_flow(selected, Kind::Footer, &nav, 0).unwrap();
    assert!(std::ptr::eq(
        header.source(),
        selected.advanced().header_content.as_ref().unwrap()
    ));
    assert_eq!(header.kind(), Kind::Header);
    assert_eq!(footer.kind(), Kind::Footer);
    assert_eq!(header.master_id(), selected.master().master_id);
    assert_ne!(header.fingerprint(), footer.fingerprint());
    assert_eq!(header.text_flow().paragraphs().len(), 1);
    assert_eq!(header.text_flow().paragraphs()[0].owner().get(), 5);
    assert_eq!(header.text_flow().paragraphs()[0].items().len(), 4);
    assert_eq!(header.text_flow().text_bytes(), (text.len() * 2) as u64);
    assert_eq!(footer.text_flow().text_bytes(), text.len() as u64);
    assert_eq!(header.record_charge(), 9);
    assert_eq!(footer.record_charge(), 6);
    let body_flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    assert_eq!(body_flow.paragraphs().len(), 1);
    assert_eq!(body_flow.paragraphs()[0].owner().get(), 2);
    for flow in [&header, &footer] {
        flow.verify_for(body, &nav).unwrap();
        assert!(flow.text_flow().footnote_definitions().is_empty());
        assert!(flow.text_flow().lists().is_empty());
        for site in flow.text_flow().paragraphs()[0].items() {
            assert_eq!(site.language(), "und");
            if let typaxis_syntax::ProductionInlineContent::Text { utf8, span } = site.content() {
                let original =
                    &body.body().wire().text_buffers()[span.text_id().get() as usize].utf8;
                assert_eq!(utf8.as_ptr(), original.as_ptr());
                assert_eq!(utf8, text);
                assert_eq!(site.source_span().end_byte().get() as usize, text.len());
            }
        }
        let exact = limits.base().get().max_fragments - flow.record_charge();
        assert_eq!(
            region_flow(selected, flow.kind(), &nav, exact)
                .unwrap()
                .record_charge(),
            limits.base().get().max_fragments
        );
        assert!(region_flow(selected, flow.kind(), &nav, exact + 1).is_err());
        assert!(region_flow(selected, flow.kind(), &nav, u64::MAX).is_err());
        let second = region_flow(selected, flow.kind(), &nav, 0).unwrap();
        assert_eq!(flow.fingerprint(), second.fingerprint());
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let shaped = shape_region(&policy, flow, input.resources(), &limits, EPOCH, None).unwrap();
        assert_eq!(shaped.paragraphs().len(), 1);
        assert_eq!(
            shaped.paragraphs()[0].font().unwrap().size().get().raw(),
            12 * 65536
        );
        assert!(shaped.paragraphs()[0]
            .runs()
            .iter()
            .all(|r| r.language() == "und"));
        assert_eq!(
            shaped.paragraphs()[0].font().unwrap().content_hash(),
            typaxis_core::sha256(font.unwrap_or(FONT))
        );
        for run in shaped.paragraphs()[0].runs() {
            assert!(flow.text_flow().paragraphs()[0]
                .items()
                .iter()
                .any(|site| site.owner() == run.owner()));
        }

        assert!(shaped.paragraphs()[0]
            .runs()
            .iter()
            .flat_map(|r| &r.glyph_run().glyphs)
            .any(|g| g.original_gid.get() != 0));
        assert!(shaped.paragraphs()[0]
            .runs()
            .iter()
            .flat_map(|r| &r.glyph_run().glyphs)
            .all(|g| g.original_gid.get() != 0));
        shaped
            .verify(flow, input.resources(), &limits, EPOCH)
            .unwrap();
        assert!(shaped
            .verify(&second, input.resources(), &limits, EPOCH)
            .is_err());
        assert!(shaped
            .verify(flow, input.resources(), &limits, [0; 32])
            .is_err());
        assert!(shape_region(&policy, flow, input.resources(), &limits, [0; 32], None).is_err());
        let invalid = [ProductionParagraphLineContext {
            owner: NodeId::new(999),
            ends: &[1],
        }];
        assert!(shape_region(
            &policy,
            flow,
            input.resources(),
            &limits,
            EPOCH,
            Some(&invalid)
        )
        .is_err());
    }
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let ends = [text.len() as u32 / 2, text.len() as u32];
    let contexts = [ProductionParagraphLineContext {
        owner: NodeId::new(11),
        ends: &ends,
    }];
    let reshaped = shape_region(
        &policy,
        &footer,
        input.resources(),
        &limits,
        EPOCH,
        Some(&contexts),
    )
    .unwrap();
    assert!(reshaped.line_context_fingerprint().is_some());
    let mut covered = 0;
    let mut line_boundary = false;
    for run in reshaped.paragraphs()[0].runs() {
        let ShapeSourceSpan::Parsed(span) = run.glyph_run().source_span else {
            panic!("page text must retain original source");
        };
        let start = span.start_byte().get();
        let end = span.end_byte().get();
        assert_eq!(start, covered);
        assert!(
            end <= ends[0] || start >= ends[0],
            "run crosses selected line boundary"
        );
        line_boundary |= end == ends[0];
        covered = end;
    }
    assert!(line_boundary);
    assert_eq!(covered, text.len() as u32);
    if font.is_some() {
        let invalid = [ProductionParagraphLineContext {
            owner: NodeId::new(11),
            ends: &[1, 12],
        }];
        assert!(shape_region(
            &policy,
            &footer,
            input.resources(),
            &limits,
            EPOCH,
            Some(&invalid)
        )
        .is_err());
    }
    let other_nav = prepare_book_v2_navigation(body).unwrap();
    assert!(header.verify_for(body, &other_nav).is_err());
    // Selecting the same master on another physical page preserves source identity.
    let next = select_book_v2_page_master(body, 1, None, &mut work, 1_000_000).unwrap();
    assert_eq!(
        header.fingerprint(),
        region_flow(next, Kind::Header, &nav, 0)
            .unwrap()
            .fingerprint()
    );
    // Preparation alone must not turn on a driver which would drop the region.
    assert!(
        typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan(body, &mut work, 1_000_000)
            .is_err()
    );
    eprintln!(
        "page-region original font={}, header_records={}, footer_records={}, source_bytes={}",
        if font.is_some() {
            "Harano"
        } else {
            "controlled TT"
        },
        header.record_charge(),
        footer.record_charge(),
        text.len()
    );
}

#[test]
fn book_v2_page_region_flow_uses_selected_master_and_requires_present_region() {
    let root = Root::new();
    let limits = limits();
    let mut data = region_data("Result");
    let mut master = data["page_masters"]["masters"][0].clone();
    master["master_id"] = "unused".into();
    master["header_content"] = Value::Null;
    master["footer_content"] = Value::Null;
    data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .push(master);
    let input = prepared(&root, data, b"Result", &limits);
    let body = input.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let mut work = 0;
    let selected = select_book_v2_page_master(body, 0, None, &mut work, 1_000_000).unwrap();
    region_flow(selected, Kind::Header, &nav, 0).unwrap();
    let mut data = region_data("Result");
    data["page_masters"]["masters"][0]["header_content"] = Value::Null;
    let other_root = Root::new();
    let other = prepared(&other_root, data, b"Result", &limits);
    let other_body = other.body().styled();
    let other_nav = prepare_book_v2_navigation(other_body).unwrap();
    let missing = select_book_v2_page_master(other_body, 0, None, &mut work, 1_000_000).unwrap();
    assert!(region_flow(missing, Kind::Header, &other_nav, 0).is_err());
    assert!(region_flow(missing, Kind::Footer, &nav, 0).is_err());
}
