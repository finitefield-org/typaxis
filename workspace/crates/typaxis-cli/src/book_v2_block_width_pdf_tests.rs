use super::*;
use crate::book_v2_resources::tests::shaping_tests::vector_tests::{
    equations::blocks::block_data, vector_input,
};

use crate::book_v2_resources::tests::shaping_tests::{
    figures::{figure_data, figure_input, JPEG, SVG},
    native_tests::{native_data, native_input_with_source},
};

fn block_width_data(kind: &str, notes: bool) -> Value {
    let mut data = source_frame(match kind {
        "vector" => block_data("center", true),
        "native" => native_data(),
        "png" => figure_data("png", PNG, Some(32 * 65536)),
        "jpeg" => figure_data("jpeg-baseline", JPEG, Some(32 * 65536)),
        "svg" => figure_data("svg-safe-2", SVG, Some(16 * 65536)),
        _ => unreachable!(),
    });
    let text = data["text_buffers"][0]["utf8"].as_str().unwrap().to_owned();
    let wrapper = &mut data["document"]["blocks"][0];
    if kind == "vector" {
        let mut caption = wrapper["blocks"][0].clone();
        caption["children"] = json!([caption["children"][0].clone()]);
        wrapper["blocks"][1]["caption"] = json!([caption]);
    }
    let wrapper = wrapper.clone();
    if notes {
        let mut p = wrapper["blocks"][0].clone();
        p["span"] = wrapper["span"].clone();
        let reference_span = p["children"].as_array().unwrap().last().unwrap()["span"].clone();
        p["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":reference_span,"footnote_id":"block-width-note"}));
        data["document"]["blocks"] = json!([p]);
        data["document"]["footnotes"] = json!([{"node_id":0,"span":wrapper["span"],"footnote_id":"block-width-note","blocks":[wrapper]}]);
    } else {
        let br = json!({"kind":"page_break","node_id":0,"classes":[],"span":wrapper["span"]});
        data["document"]["blocks"] =
            json!([wrapper.clone(), br.clone(), wrapper.clone(), br, wrapper]);
        data["document"]["footnotes"] = json!([]);
    }
    data["page_masters"] = selected_master_data(&text)["page_masters"].clone();
    for (i, master) in data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        master["width"] = (320 * 65536).into();
        master["height"] = (300 * 65536).into();
        master["trim"] = json!({"x":0,"y":0,"width":320*65536,"height":300*65536});
        let body_width = [180, 140, 100, 160][i] * 65536;
        master["body"] = json!({"x":10*65536,"y":10*65536,"width":body_width,"height":100*65536});
        if notes {
            let note_width = [160, 140, 120, 160][i] * 65536;
            master["footnote"] = json!({"x":20*65536,"y":150*65536,"width":note_width,"height":90*65536+typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW});
        }
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    if kind == "native" {
        rules.push(json!({"style_id":"block-width-native-align","selector":"display_math","source_order":rules.len(),"extends":null,
            "declarations":[{"name":"text_align","important":false,"value":{"kind":"keyword","value":"center"}}]}));
    }
    rules.push(json!({"style_id":format!("block-width-{kind}-{}",if notes {"notes"} else {"body"}),"selector":"paragraph","source_order":rules.len(),"extends":null,"declarations":[]}));
    if kind == "native" {
        // Distinct native math source ranges must remain monotone across the
        // three authored copies, including the intervening explicit breaks.
        let count = text.len() as u64;
        fn offset(value: &mut Value, amount: u64) {
            if let Some(values) = value.as_array_mut() {
                for value in values {
                    offset(value, amount);
                }
            } else if let Some(object) = value.as_object_mut() {
                for value in object.values_mut() {
                    offset(value, amount);
                }
                for key in ["start_byte", "end_byte"] {
                    if let Some(value) = object.get_mut(key) {
                        *value = (value.as_u64().unwrap() + amount).into();
                    }
                }
            }
        }
        if notes {
            offset(&mut data["document"]["footnotes"][0], count);
        } else {
            for (index, block) in data["document"]["blocks"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
            {
                if index % 2 == 0 {
                    offset(block, (index / 2) as u64 * count);
                } else {
                    let at = (index / 2 + 1) as u64 * count;
                    block["span"]["start_byte"] = at.into();
                    block["span"]["end_byte"] = at.into();
                }
            }
        }
        let original = data["text_buffers"][0]["mappings"]
            .as_array()
            .unwrap()
            .clone();
        let mut mappings = Vec::new();
        let copies = if notes { 2 } else { 3 };
        for i in 0..copies {
            for mapping in &original {
                let mut mapping = mapping.clone();
                offset(&mut mapping, i * count);
                mappings.push(mapping);
            }
        }
        data["text_buffers"][0]["mappings"] = mappings.into();
        data["text_buffers"][0]["utf8"] = text.repeat(copies as usize).into();
    }
    super::horizontal_origins::number_nodes(&mut data["document"], &mut 0);
    data
}

#[path = "book_v2_table_block_width_pdf_tests.rs"]
mod table_blocks;

fn check_block_width_pdfs(font: Option<&[u8]>) {
    check_block_width_cases(font, false);
}
fn check_block_width_cases(font: Option<&[u8]>, tables: bool) {
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        if font.is_some() && kind == "native" {
            continue;
        }
        for notes in [false, true] {
            let root = Root::new();
            let limits = driver_limits();
            let mut data = block_width_data(kind, notes);
            if tables {
                table_blocks::wrap_tables(&mut data);
            }

            let input = if let Some(font) = font {
                data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
                data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .into();
                let source = data["text_buffers"][0]["utf8"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .to_vec();
                let admitted = body_with_source(&root, data, &source, &limits);
                fs::write(root.0.join("body.bin"), font).unwrap();
                fs::write(
                    root.0.join(if kind == "vector" {
                        "vector.svg"
                    } else {
                        "figure.bin"
                    }),
                    match kind {
                        "vector" | "svg" => SVG,
                        "png" => PNG,
                        "jpeg" => JPEG,
                        _ => unreachable!(),
                    },
                )
                .unwrap();
                prepare_book_v2_resources(
                    admitted,
                    &root.context(),
                    &config(limits.base().get().clone()),
                    &limits,
                )
                .unwrap()
            } else {
                match kind {
                    "vector" => vector_input(&root, data, &limits),
                    "native" => {
                        let source = data["text_buffers"][0]["utf8"]
                            .as_str()
                            .unwrap()
                            .as_bytes()
                            .to_vec();
                        native_input_with_source(&root, data, &source, &limits)
                    }
                    "png" => figure_input(&root, data, PNG, &limits),
                    "jpeg" => figure_input(&root, data, JPEG, &limits),
                    "svg" => figure_input(&root, data, SVG, &limits),
                    _ => unreachable!(),
                }
            };
            let run = |maximum| {
                with_converged_book_v2_pdf(
                    &input,
                    &limits,
                    JapaneseLineBreakMode::Normal,
                    maximum,
                    |pdf, observed| {
                        assert!(observed.width_feedback_passes() >= 2);
                        crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                        if let Some(directory) = std::env::var_os(if tables {
                            "TYPAXIS_BOOK_V2_TABLE_BLOCK_WIDTH_PROBE"
                        } else {
                            "TYPAXIS_BOOK_V2_BLOCK_WIDTH_PROBE"
                        }) {
                            let directory = std::path::PathBuf::from(directory);
                            fs::create_dir_all(&directory).unwrap();
                            let mode = if notes { "notes" } else { "body" };
                            let family = if font.is_some() {
                                "harano"
                            } else {
                                "controlled"
                            };
                            fs::write(
                                directory.join(format!("{family}-{kind}-{mode}.pdf")),
                                pdf.bytes(),
                            )
                            .unwrap();
                            fs::write(directory.join(format!("{family}-{kind}-{mode}.json")),serde_json::to_vec_pretty(&json!({"width_passes":observed.width_feedback_passes(),"work":observed.work_steps(),"line_passes":observed.line_reshape_passes(),"page_passes":observed.page_passes()})).unwrap()).unwrap();
                        }
                        (observed, typaxis_core::sha256(pdf.bytes()))
                    },
                )
            };
            let full =
                run(100_000_000).unwrap_or_else(|e| panic!("kind={kind} notes={notes}: {e:?}"));
            assert_eq!(run(full.0.work_steps()).unwrap(), full);
            assert!(run(full.0.work_steps() - 1).is_err());
        }
    }
}

#[test]
fn book_v2_block_widths_converge_vector_native_and_raster_pdf() {
    check_block_width_pdfs(None);
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_block_widths_render_original_harano_figures_and_captions() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    check_block_width_pdfs(Some(&font));
}

#[path = "book_v2_table_block_source_pdf_tests.rs"]
mod block_source_profiles;
