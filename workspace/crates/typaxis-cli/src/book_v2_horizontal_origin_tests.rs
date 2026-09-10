use super::*;
use crate::book_v2_resources::tests::shaping_tests::vector_tests::{
    equations::blocks::block_data, vector_input,
};

pub(super) fn number_nodes(value: &mut Value, next: &mut u32) {
    match value {
        Value::Array(values) => {
            for value in values {
                number_nodes(value, next);
            }
        }
        Value::Object(_) => {
            if value.get("node_id").is_some() {
                value["node_id"] = (*next).into();
                *next += 1;
            }
            for key in [
                "blocks",
                "children",
                "items",
                "head",
                "body",
                "cells",
                "caption",
                "equation_number",
                "footnotes",
            ] {
                if let Some(child) = value.get_mut(key) {
                    number_nodes(child, next);
                }
            }
        }
        _ => {}
    }
}
fn data(text: &str, mode: &str) -> Value {
    let mut data = match mode {
        "body" | "notes" => varying_page_frames_data(text, mode == "notes"),
        "table" | "note-table" => varying_table_frames_data(mode == "note-table"),
        "references" => named_pages::names_data(text, "references").0,
        "nested" => named_pages::names_data(text, "nested").0,
        "list" | "vectors" => {
            let mut data = if mode == "list" {
                source_frame(notes_table_data())
            } else {
                source_frame(block_data("center", true))
            };
            let original = data["page_masters"]["masters"][0].clone();
            let template = selected_master_data(text)["page_masters"].clone();
            data["page_masters"] = template;
            for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
                master["width"] = (600 * 65536).into();
                master["height"] = (600 * 65536).into();
                master["trim"] = json!({"x":0,"y":0,"width":600*65536,"height":600*65536});
                master["body"] = original["body"].clone();
                master["footnote"] = original["footnote"].clone();
            }
            if mode == "vectors" {
                let wrapper = data["document"]["blocks"][0].clone();
                let br =
                    json!({"kind":"page_break","node_id":0,"classes":[],"span":wrapper["span"]});
                data["document"]["blocks"] =
                    json!([wrapper.clone(), br.clone(), wrapper.clone(), br, wrapper]);
            }
            number_nodes(&mut data["document"], &mut 0);
            data
        }
        _ => panic!("mode"),
    };
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":format!("horizontal-{mode}"),"selector":"paragraph","source_order":rules.len(),"extends":null,"declarations":[]}));
    if text == "左側右側" && mode == "notes" {
        let rule = data["style_sheet"]["rules"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["style_id"] == "varying")
            .unwrap();
        rule["declarations"][0]["value"]["value"] = (20 * 65536).into();
        for (i, h) in [(0, 60), (1, 40), (2, 20)] {
            let master = &mut data["page_masters"]["masters"][i];
            master["body"]["height"] = (h * 65536).into();
            master["footnote"]["height"] =
                (h * 65536 + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW).into();
        }
    }
    data
}
fn move_origins(data: &mut Value) {
    for (i, master) in data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        master["body"]["x"] = ([40, 15, 25, 55][i] * 65536).into();
        if !master["footnote"].is_null() {
            master["footnote"]["x"] = ([10, 45, 20, 60][i] * 65536).into();
        }
    }
}
fn run(text: &str, mode: &str, original_font: Option<&[u8]>) {
    let mut expected = None;
    for moved in [false, true] {
        let root = Root::new();
        let limits = driver_limits();
        let mut data = data(text, mode);
        if moved {
            move_origins(&mut data);
        }
        if let Some(font) = original_font {
            data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
            data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                .into();
        }
        let masters = data["page_masters"]["masters"].clone();
        let input = if mode == "vectors" {
            vector_input(&root, data, &limits)
        } else if let Some(font) = original_font {
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
        let run = |work, capture| {
            with_converged_book_v2_pdf(
                &input,
                &limits,
                JapaneseLineBreakMode::Normal,
                work,
                |pdf, observed| {
                    let pages = pdf
                        .navigation()
                        .source()
                        .source()
                        .source()
                        .source()
                        .display()
                        .source()
                        .source()
                        .geometry()
                        .pages();
                    let mut snapshot = Vec::new();
                    let mut lists = 0;
                    let mut numbers = 0;
                    for (page, p) in pages.iter().enumerate() {
                        let i = match p.selection().named_page() {
                            Some("appendix") => 3,
                            Some("short") => 1,
                            None if page == 0 => 2,
                            None if page % 2 == 1 => 1,
                            None => 0,
                            _ => panic!("name"),
                        };
                        let master = &masters[i];
                        let x = |note| {
                            master[if note { "footnote" } else { "body" }]["x"]
                                .as_i64()
                                .unwrap()
                        };
                        let rect = |r: typaxis_core::Rect, note| {
                            json!([
                                r.x().raw() - x(note),
                                r.y().raw(),
                                r.width().get().raw(),
                                r.height().get().raw()
                            ])
                        };
                        let fragments = p
                            .fragments()
                            .iter()
                            .map(|f| {
                                let v = f.fragment();
                                let note = f.definition_index().is_some();
                                json!([
                                    v.owner().get(),
                                    f.definition_index(),
                                    f.item_index(),
                                    format!("{:?}", v.source()),
                                    rect(v.bounds(), note),
                                    v.viewport().map(|r| rect(r, note)),
                                    v.baseline().map(|n| n.raw())
                                ])
                            })
                            .collect::<Vec<_>>();
                        let list = p
                            .list_markers()
                            .iter()
                            .map(|m| {
                                json!([
                                    m.owner().get(),
                                    rect(
                                        m.bounds(),
                                        p.fragments()[m.fragment_index() as usize]
                                            .definition_index()
                                            .is_some()
                                    )
                                ])
                            })
                            .collect::<Vec<_>>();
                        let notes = p
                            .footnote_markers()
                            .iter()
                            .map(|m| json!([m.definition_index(), rect(m.bounds(), true)]))
                            .collect::<Vec<_>>();
                        let equations = p
                            .equation_numbers()
                            .iter()
                            .map(|m| {
                                let v = m.geometry();
                                let f = p
                                    .fragments()
                                    .iter()
                                    .find(|f| f.fragment().owner() == v.parent_owner())
                                    .unwrap();
                                json!([
                                    v.owner().get(),
                                    rect(v.bounds(), f.definition_index().is_some()),
                                    m.repeated_header()
                                ])
                            })
                            .collect::<Vec<_>>();
                        lists += list.len();
                        numbers += equations.len();
                        snapshot.push(json!([
                            p.selection().named_page(),
                            fragments,
                            list,
                            notes,
                            equations,
                            p.separator_ink().map(|r| rect(r, true))
                        ]));
                    }
                    if mode == "list" {
                        assert!(lists > 0);
                    }
                    if mode == "vectors" {
                        assert_eq!(numbers, 3);
                    }
                    if let Some(expected) = &expected {
                        assert_eq!(&snapshot, expected, "{mode}");
                    }
                    if capture {
                        crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                    }
                    (snapshot, observed, typaxis_core::sha256(pdf.bytes()))
                },
            )
        };
        let full = run(100_000_000, true).unwrap_or_else(|e| panic!("{mode},moved={moved}: {e:?}"));
        if moved {
            assert_eq!(run(full.1.work_steps(), false).unwrap(), full);
            assert!(run(full.1.work_steps() - 1, false).is_err());
        }
        expected = Some(full.0);
    }
}
#[test]
fn book_v2_horizontal_origins_preserve_content_and_translate_markers_viewports_and_names() {
    for mode in [
        "body",
        "notes",
        "table",
        "note-table",
        "references",
        "nested",
        "list",
        "vectors",
    ] {
        run("Result", mode, None);
    }
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_horizontal_origins_render_original_harano() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    for mode in ["notes", "references"] {
        run("左側右側", mode, Some(&font));
    }
}
