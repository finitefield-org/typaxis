use super::*;
use table_caption_breaks::renumber;

fn uniform_data(text: &str, mode: &str) -> Value {
    let (mut data, _) = names_data(text, "table");
    let table = &mut data["document"]["blocks"][0];
    table["classes"] = json!(["appendix", format!("uniform-{mode}")]);
    for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
        for paragraph in cell["blocks"].as_array_mut().unwrap() {
            paragraph["classes"] = json!(["short"]);
        }
    }
    if mode == "nested" {
        let mut child = table.clone();
        child["classes"] = json!(["appendix"]);
        for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
            cell["blocks"] = json!([child.clone()]);
        }
    } else if mode == "caption" {
        table["caption"] = json!([table["body"][0]["cells"][0]["blocks"][0].clone()]);
    }
    let master = &mut data["page_masters"]["masters"][1];
    master["body"]["height"] = (40 * 65536).into();
    if mode == "width" {
        master["body"]["x"] = (18 * 65536).into();
        master["body"]["width"] = (120 * 65536).into();
    }
    renumber(&mut data["document"], &mut 0);
    data
}

fn check_uniform_tables(font: Option<&[u8]>) {
    for mode in ["flat", "nested", "caption", "width"] {
        let text = if mode == "nested" {
            if font.is_some() {
                "左右"
            } else {
                "Res"
            }
        } else {
            match (font.is_some(), mode == "width") {
                (true, true) => "左側右側左側右側",
                (true, false) => "左側右側",
                (false, true) => "Result Result",
                (false, false) => "Result",
            }
        };
        let root = Root::new();
        let limits = driver_limits();
        let mut data = uniform_data(text, mode);
        let input = if let Some(bytes) = font {
            data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
            data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                .into();
            let body = body_with_source(&root, data, text.as_bytes(), &limits);
            fs::write(root.0.join("body.bin"), bytes).unwrap();
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
        let run = |work, capture| {
            with_converged_book_v2_pdf(
                &input,
                &limits,
                JapaneseLineBreakMode::Normal,
                work,
                |pdf, observed| {
                    let body = pdf
                        .navigation()
                        .source()
                        .source()
                        .source()
                        .source()
                        .display()
                        .source()
                        .source();
                    let pages = body.geometry().pages();
                    let lines = body.flow().lines();
                    let plan = lines.frames().unwrap().page_plan().unwrap();
                    let table_owner = lines.prepared().source_flow().tables()[0].owner();
                    assert_eq!(
                        plan.name(plan.source_name_index(table_owner).unwrap()),
                        Some("appendix")
                    );
                    assert!(pages.len() >= 3, "{mode}: {}", pages.len());
                    let (last, table_pages) = pages.split_last().unwrap();
                    assert_eq!(last.selection().named_page(), None, "{mode}");
                    assert!(
                        table_pages
                            .iter()
                            .all(|p| p.selection().named_page() == Some("short")),
                        "{mode}"
                    );
                    if capture {
                        crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                    }
                    (observed, typaxis_core::sha256(pdf.bytes()))
                },
            )
        };
        let full = run(100_000_000, true).unwrap_or_else(|e| panic!("{mode}: {e:?}"));
        assert_eq!(run(full.0.work_steps(), false).unwrap(), full, "{mode}");
        assert!(run(full.0.work_steps() - 1, false).is_err(), "{mode}");
    }
}

#[test]
fn book_v2_uniform_table_pages_keep_content_scope_and_restore_body() {
    check_uniform_tables(None);
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_uniform_table_pages_render_original_harano() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    check_uniform_tables(Some(&bytes));
}

#[test]
fn book_v2_uniform_table_pages_reject_caption_and_empty_child_conflicts() {
    for mode in ["caption", "empty-child"] {
        let mut data = uniform_data("Result", "flat");
        let table = &mut data["document"]["blocks"][0];
        let owner;
        if mode == "caption" {
            let mut paragraph = table["body"][0]["cells"][0]["blocks"][0].clone();
            paragraph["classes"] = json!([]);
            table["caption"] = json!([paragraph]);
            renumber(&mut data["document"], &mut 0);
            // The inherited caption selects appendix, so the first short leaf conflicts.
            owner = data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0]["node_id"]
                .as_u64()
                .unwrap();
        } else {
            let mut child = table.clone();
            for cell in child["body"][0]["cells"].as_array_mut().unwrap() {
                cell["blocks"] = json!([]);
            }
            table["body"][0]["cells"][0]["blocks"]
                .as_array_mut()
                .unwrap()
                .push(child);
            renumber(&mut data["document"], &mut 0);
            owner = data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][4]["node_id"]
                .as_u64()
                .unwrap();
        }
        let root = Root::new();
        let limits = driver_limits();
        let input = prepared(&root, data, b"Result", &limits);
        let error = with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |_, _| panic!("conflicting table reached PDF"),
        )
        .err()
        .unwrap();
        assert!(
            matches!(error, CE::Stage { ref source, .. }
            if source.downcast_ref::<typaxis_pagination::ProductionBodyPaginationError>().is_some_and(|e|
                e.kind == typaxis_pagination::ProductionBodyPaginationErrorKind::PendingNamedPage
                && u64::from(e.owner.get()) == owner)),
            "{mode}: {error:?}"
        );
    }
}
