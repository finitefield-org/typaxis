use super::*;
use crate::book_v2_resources::with_converged_book_v2_pdf;

#[test]
fn book_v2_driver_converges_native_vector_and_raster_headers() {
    check(false, false, None, false, false);
    check(true, false, None, false, false);
    check(true, true, None, false, false);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_driver_converges_original_harano_vector_headers() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(true, false, Some(&font), false, false);
}

#[test]
fn book_v2_driver_rejects_fixed_vector_header_that_cannot_fit_note_insets() {
    check(true, false, None, true, false);
}

#[test]
fn book_v2_driver_converges_independent_child_math_and_image_headers() {
    check(false, false, None, false, true);
    check(true, false, None, false, true);
    check(true, true, None, false, true);
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_driver_converges_original_harano_child_vector_headers() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(true, false, Some(&font), false, true);
}

#[test]
fn book_v2_driver_rejects_fixed_child_vector_header_that_cannot_fit_note_insets() {
    check(true, false, None, true, true);
}

fn check(vector: bool, raster: bool, font: Option<&[u8]>, undersized: bool, nested: bool) {
    for notes in [false, true] {
        if undersized && !notes {
            continue;
        }
        // The two adjacent fixed vectors need room in addition to the list and
        // footnote insets. Preserve their original authored metrics.
        let extra = if vector && notes && !undersized {
            20
        } else {
            0
        };
        let root = Root::new();
        let original = limits();
        let mut caps = original.base().get().clone();
        caps.max_line_reshape_passes = 128;
        caps.max_layout_passes = 128;
        caps.max_page_break_lookback = 256;
        let limits = M4EffectiveResourceLimits::new(
            typaxis_core::ValidatedResourceLimits::new(caps).unwrap(),
            original.extension().get().clone(),
        )
        .unwrap();
        let (mut data, text) = header_fixture(vector, false, raster, notes);
        if nested {
            let table = if notes {
                &mut data["document"]["footnotes"][0]["blocks"][0]
            } else {
                &mut data["document"]["blocks"][0]
            };
            let child = table.clone();
            let mut parent = child.clone();
            // The child is in the body, so its header must repeat independently
            // of the parent header rather than being copied as a whole table.
            parent["head"][0]["cells"][0]["blocks"] =
                child["body"][0]["cells"][0]["blocks"].clone();
            parent["body"].as_array_mut().unwrap().truncate(1);
            parent["body"][0]["cells"][0]["blocks"] = json!([child]);
            *table = parent;
            super::renumber(&mut data["document"], &mut 0);
        }
        let original = data["page_masters"]["masters"][0].clone();
        let mut masters = Vec::new();
        for (index, (id, width)) in [
            ("header-wide", 180),
            ("header-first", 100),
            ("header-even", 140),
        ]
        .into_iter()
        .enumerate()
        {
            let mut master = original.clone();
            master["master_id"] = id.into();
            if nested {
                // Explicitly fund the added parent header's vertical band;
                // all child math and image metrics remain authored values.
                for region in if notes {
                    &["body", "footnote"][..]
                } else {
                    &["body"][..]
                } {
                    let height = master[region]["height"].as_i64().unwrap();
                    master[region]["height"] = (height + 32 * 65536).into();
                }
            }
            master["body"]["width"] = ((width + extra) * 65536).into();
            master["body"]["x"] = ((20 + index as i64 * 7) * 65536).into();
            if notes {
                master["footnote"]["width"] = ((width + extra) * 65536).into();
                master["footnote"]["x"] = ((30 + index as i64 * 11) * 65536).into();
            }
            masters.push(master);
        }
        masters.sort_by(|a, b| {
            a["master_id"]
                .as_str()
                .unwrap()
                .cmp(b["master_id"].as_str().unwrap())
        });
        data["page_masters"]["masters"] = masters.into();
        data["page_masters"]["default_master_id"] = "header-wide".into();
        data["page_masters"]["selection_rules"] = json!([
            {"master_id":"header-even","parity":"even","first":null,"named_page":null,"source_order":0},
            {"master_id":"header-first","parity":"any","first":true,"named_page":null,"source_order":1}
        ]);
        let input = if let Some(font) = font {
            super::super::super::vector_tests::vector_input_with_font(
                &root, data, &limits, font, text,
            )
        } else if vector {
            super::super::super::vector_tests::vector_input(&root, data, &limits)
        } else {
            native_input(&root, data, &limits)
        };
        let result = with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            1_000_000_000,
            |pdf, observed| {
                assert!(
                    !undersized,
                    "an infeasible fixed header must not return a PDF"
                );
                let display = pdf
                    .navigation()
                    .source()
                    .source()
                    .source()
                    .source()
                    .display();
                let terminals = display.source();
                let closed = terminals.source();
                let pages = closed.geometry().pages();
                assert!(closed.has_header_variants());
                assert!(pages.len() >= 3);
                if nested {
                    let mut repeated = std::collections::BTreeSet::new();
                    for page in pages {
                        for variant in page.header_variants() {
                            repeated.insert(variant.header().table_index());
                        }
                    }
                    assert_eq!(repeated, [0, 1].into());
                }
                assert_eq!(closed.semantic_math(), 2);
                assert_eq!(closed.repeated_math(), 2 * (pages.len() - 1));
                assert_math_terminals(terminals);
                let widths = pages
                    .iter()
                    .map(|p| {
                        let selection = p.selection();
                        if notes {
                            selection.declared_footnote_region().unwrap().width()
                        } else {
                            selection.body_bounds().width()
                        }
                    })
                    .map(|w| w.get().raw())
                    .collect::<std::collections::BTreeSet<_>>();
                assert_eq!(
                    widths,
                    [100 + extra, 140 + extra, 180 + extra]
                        .map(|w| w * 65536)
                        .into()
                );
                assert_eq!(
                    pages
                        .iter()
                        .map(|p| p.equation_numbers().len())
                        .sum::<usize>(),
                    if vector { pages.len() } else { 0 }
                );
                for t in terminals.terminals() {
                    assert_eq!(
                        matches!(t.source(), BookV2BodyMathSource::Vector(_)),
                        vector
                    );
                }
                if raster {
                    let draws = display
                        .images()
                        .draws()
                        .iter()
                        .filter(|d| {
                            matches!(
                                d.paint(),
                                typaxis_display_list::book_v2::BookV2ImagePaint::Raster { .. }
                            )
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(draws.len(), pages.len());
                    assert_eq!(draws.iter().filter(|d| !d.repeated_header()).count(), 1);
                    let empty = display.anchors().nonpainting_lines();
                    assert_eq!(empty.len(), pages.len());
                    assert_eq!(empty.iter().filter(|l| !l.repeated_header()).count(), 1);
                }
                super::display::verify_driver_resources(display, &limits);
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                (pages.len(), observed)
            },
        );
        if undersized {
            let error = result.unwrap_err();
            assert!(
                matches!(
                    &error,
                    crate::book_v2_resources::converged_pdf::BookV2ConvergenceError::Stage {
                        stage: "header base convergence",
                        ..
                    }
                ),
                "{error:?}"
            );
            assert!(format!("{error:?}").contains("Atomic(NoFeasibleLine)"));
            continue;
        }
        let result = result.unwrap();
        eprintln!("automatic header math driver: nested={nested},vector={vector},raster={raster},harano={},notes={notes},pages={},observed={:?}",font.is_some(),result.0,result.1);
    }
}
