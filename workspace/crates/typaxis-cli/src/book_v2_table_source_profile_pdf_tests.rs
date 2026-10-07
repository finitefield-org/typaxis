use super::*;

fn check_source_profiles(font: Option<&[u8]>) {
    for notes in [false, true] {
        for mode in ["natural", "split", "forced"] {
            let root = Root::new();
            let base = driver_limits();
            let mut caps = base.base().get().clone();
            caps.max_layout_passes = 64;
            caps.max_line_reshape_passes = 64;
            let limits = M4EffectiveResourceLimits::new(
                typaxis_core::ValidatedResourceLimits::new(caps).unwrap(),
                base.extension().get().clone(),
            )
            .unwrap();
            let text = match (font.is_some(), mode) {
                (true, "split") => "本文を続けて組み直す本文を続けて組み直す",
                (true, _) => "本文の段落",
                (false, "split") => "Pro Pro Pro Pro Pro Pro Pro",
                _ => "Result",
            };
            let mut data = super::table_width_occurrences::occurrence_data(notes, mode);
            fn expand(v: &mut Value, end: usize) {
                if let Some(a) = v.as_array_mut() {
                    for v in a {
                        expand(v, end);
                    }
                } else if let Some(o) = v.as_object_mut() {
                    for (key, v) in o {
                        if key == "end_byte" && v.as_u64() == Some(6) {
                            *v = end.into();
                        } else {
                            expand(v, end);
                        }
                    }
                }
            }
            expand(&mut data, text.len());
            data["text_buffers"][0]["utf8"] = text.into();
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            rules.push(json!({"style_id":format!("table-source-profile-{mode}-{}",if notes{"notes"}else{"body"}),"selector":"paragraph","source_order":rules.len(),"extends":null,"declarations":[{"name":"text_align","important":false,"value":{"kind":"keyword","value":"start"}}]}));
            for (class, align) in [("left", "center"), ("right", "end")] {
                rules.push(json!({"style_id":format!("source-profile-{class}"),"selector":format!("paragraph.{class}"),"source_order":rules.len(),"extends":null,"declarations":[{"name":"text_align","important":false,"value":{"kind":"keyword","value":align}}]}));
            }
            let input = if let Some(font) = font {
                data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
                data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .into();
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
            let run = |maximum| {
                with_converged_book_v2_pdf(
                    &input,
                    &limits,
                    JapaneseLineBreakMode::Normal,
                    maximum,
                    |pdf, observed| {
                        let closed = pdf
                            .navigation()
                            .source()
                            .source()
                            .source()
                            .source()
                            .display()
                            .source()
                            .source();
                        let frames = closed.flow().lines().frames().unwrap();
                        assert!(frames.uses_table_occurrence_frames());
                        assert!(observed.width_feedback_passes() >= 2);
                        let mut widths = std::collections::BTreeSet::new();
                        for page in closed.geometry().pages() {
                            let bounds = if notes {
                                page.selection().declared_footnote_region().unwrap()
                            } else {
                                page.selection().body_bounds()
                            };
                            widths.insert(bounds.width().get().raw());
                            for placed in page.fragments() {
                                let bounds = if placed.definition_index().is_some() {
                                    page.selection().declared_footnote_region().unwrap()
                                } else {
                                    page.selection().body_bounds()
                                };
                                let b = placed.fragment().bounds();
                                assert!(b.x() >= bounds.x());
                                assert!(
                                    b.x().checked_add(b.width().get()).unwrap()
                                        <= bounds.x().checked_add(bounds.width().get()).unwrap()
                                );
                            }
                        }
                        assert!(widths.len() > 1);
                        crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                        if let Some(directory) =
                            std::env::var_os("TYPAXIS_BOOK_V2_TABLE_SOURCE_PROFILE_PROBE")
                        {
                            let directory = std::path::PathBuf::from(directory);
                            fs::create_dir_all(&directory).unwrap();
                            let name = format!(
                                "{}-{mode}-{}",
                                if font.is_some() {
                                    "harano"
                                } else {
                                    "controlled"
                                },
                                if notes { "notes" } else { "body" }
                            );
                            fs::write(directory.join(format!("{name}.pdf")), pdf.bytes()).unwrap();
                            fs::write(directory.join(format!("{name}.json")),serde_json::to_vec_pretty(&json!({"width_passes":observed.width_feedback_passes(),"refinements":observed.width_refinement_passes(),"line_passes":observed.line_reshape_passes(),"page_passes":observed.page_passes(),"pages":closed.geometry().pages().len(),"work":observed.work_steps()})).unwrap()).unwrap();
                        }
                        eprintln!("table source profiles: notes={notes},mode={mode},harano={},pages={},widths={},work={}",font.is_some(),closed.geometry().pages().len(),observed.width_feedback_passes(),observed.work_steps());
                        (observed, typaxis_core::sha256(pdf.bytes()))
                    },
                )
            };
            let full = run(100_000_000)
                .unwrap_or_else(|e| panic!("{mode}/{notes}/harano={} {e:?}", font.is_some()));
            assert_eq!(run(full.0.work_steps()).unwrap(), full);
            assert!(run(full.0.work_steps() - 1).is_err());
        }
    }
}
#[test]
fn book_v2_table_source_profiles_converge_physical_pdfs() {
    check_source_profiles(None);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_source_profiles_converge_original_harano_pdfs() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_source_profiles(Some(&font));
}
