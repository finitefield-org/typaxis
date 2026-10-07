use super::*;

#[test]
fn book_v2_number_references_paint_source_bound_digits_and_forward_back_links() {
    check(false);
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_number_references_original_harano_keep_japanese_label_range() {
    check(true);
}

fn check(original_font: bool) {
    let (text, start) = if original_font {
        ("定理1.12", 6)
    } else {
        ("Result 1.12", 7)
    };
    let mut data = source_data(text);
    let group = data["document"]["blocks"][0].clone();
    let reference = |position| {
        json!({
            "kind":"paragraph","node_id":0,"classes":[],
            "span":{"source_id":0,"start_byte":position,"end_byte":position},
            "children":[{"kind":"reference","node_id":0,"target":"result.number","format":"number",
                "span":{"source_id":0,"start_byte":position,"end_byte":position}}]
        })
    };
    data["document"]["blocks"] = json!([reference(0), group, reference(text.len())]);
    data["page_masters"]["masters"][0]["body"] =
        json!({"x":500000,"y":500000,"width":10000000,"height":3000000});
    data["page_masters"]["masters"][0]["width"] = 12_000_000.into();
    data["page_masters"]["masters"][0]["height"] = 12_000_000.into();
    data["page_masters"]["masters"][0]["trim"] =
        json!({"x":0,"y":0,"width":12_000_000,"height":12_000_000});
    renumber(&mut data["document"], &mut 0);
    let group = &data["document"]["blocks"][1];
    data["document"]["number_bindings"] = json!([{
        "anchor_id":"result.number", "owner_node_id":group["node_id"],
        "label_node_id":group["blocks"][0]["children"][0]["node_id"],
        "text_span":{"text_id":0,"start_byte":start,"end_byte":text.len()}
    }]);
    let references: Vec<_> = [0, 2]
        .iter()
        .map(|&i| {
            NodeId::new(
                data["document"]["blocks"][i]["children"][0]["node_id"]
                    .as_u64()
                    .unwrap() as u32,
            )
        })
        .collect();
    let root = Root::new();
    let limits = limits();
    let input = if original_font {
        let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
        let hash = typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(
            hash,
            "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
        );
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = hash.into();
        let body = body_with_source(&root, data, text.as_bytes(), &limits);
        fs::write(root.0.join("body.bin"), font).unwrap();
        prepare_book_v2_resources(
            body,
            &root.context(),
            &config(limits.base().get().clone()),
            &limits,
        )
        .unwrap()
    } else {
        prepared(&root, data, text.as_bytes(), &limits)
    };
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            assert!(pdf.page_references().is_empty());
            let marked = pdf.navigation().source().source().source();
            let display = marked.source().display();
            let flow = display
                .source()
                .source()
                .flow()
                .lines()
                .prepared()
                .source_flow();
            for owner in references {
                assert_eq!(flow.reference_text(owner), Some("1.12"));
                assert!(flow.reference_provenance(owner).is_some());
            }
            assert_eq!(flow.generated_text_bytes(), 8);
            assert_eq!(
                flow.navigation().reference_number("result.number"),
                Some("1.12")
            );
            assert_eq!(pdf.navigation().links().len(), 2);
            assert!(pdf.navigation().rectangles().len() >= 2);
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            if original_font {
                if let Ok(path) = std::env::var("TYPAXIS_BOOK_V2_NUMBER_BINDING_PDF") {
                    use std::io::Write;
                    fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(path)
                        .unwrap()
                        .write_all(pdf.bytes())
                        .unwrap();
                }
            }
        },
    )
    .unwrap();
}
