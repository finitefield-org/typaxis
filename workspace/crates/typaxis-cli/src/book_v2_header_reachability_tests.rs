use super::*;
use crate::book_v2_resources::with_converged_book_v2_pdf;

#[test]
fn book_v2_header_rebuild_does_not_restrict_first_page_body_figure_to_repeat_width() {
    for mode in ["body", "caption", "nested-body", "nested-caption"] {
        check(mode, false);
    }
}

#[test]
fn book_v2_header_scope_does_not_authorize_body_on_an_infeasible_first_page() {
    check("body", true);
}

fn check(mode: &str, infeasible: bool) {
    for notes in [false, true] {
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
        let (mut data, _) = header_fixture(true, false, true, notes);
        let table = if notes {
            &mut data["document"]["footnotes"][0]["blocks"][0]
        } else {
            &mut data["document"]["blocks"][0]
        };
        let mut figure = table["head"][0]["cells"][0]["blocks"][3].clone();
        assert_eq!(figure["kind"], "figure");
        figure["classes"] = json!(["wide-body"]);
        figure["alt"] = "wide first-page body figure".into();
        let content = if mode.starts_with("nested-") {
            let mut label = table["body"][0]["cells"][0]["blocks"][0].clone();
            label["span"]["end_byte"] = 1.into();
            label["children"][0]["span"]["end_byte"] = 1.into();
            label["children"][0]["text_span"]["end_byte"] = 1.into();
            let span = table["span"].clone();
            json!({"kind":"table","node_id":0,"span":span,"classes":[],
            "columns":[{"kind":"fixed","width":170*65536},{"kind":"fraction","weight":1}],"head":[],
            "body":[{"node_id":0,"span":span,"cells":[
                {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[figure]},
                {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[label]}
            ]}]})
        } else {
            figure
        };
        if mode.ends_with("caption") {
            table["caption"] = json!([content]);
        } else {
            table["body"][0]["cells"][0]["blocks"]
                .as_array_mut()
                .unwrap()
                .push(content);
        }
        let order = data["style_sheet"]["rules"].as_array().unwrap().len();
        data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({
            "style_id":"wide-body-figure", "selector":"figure.wide-body", "source_order":order,
            "extends":null, "declarations":[{"name":"width","important":false,"value":{"kind":"length","value":160*65536}}]
        }));
        renumber(&mut data["document"], &mut 0);
        let original = data["page_masters"]["masters"][0].clone();
        let mut masters = Vec::new();
        for (id, width) in [("a-wide", 180), ("b-even", 140), ("c-first", 220)] {
            let mut master = original.clone();
            let width = if infeasible {
                match id {
                    "c-first" => 140,
                    "b-even" => 220,
                    _ => width,
                }
            } else {
                width
            };
            master["master_id"] = id.into();
            master["body"]["width"] = (width * 65536).into();
            master["body"]["height"] = (300 * 65536).into();
            if notes {
                master["footnote"]["width"] = (width * 65536).into();
                master["footnote"]["height"] = (300 * 65536).into();
            }
            masters.push(master);
        }
        data["page_masters"]["masters"] = masters.into();
        data["page_masters"]["default_master_id"] = "a-wide".into();
        data["page_masters"]["selection_rules"] = json!([
            {"master_id":"b-even","parity":"even","first":null,"named_page":null,"source_order":0},
            {"master_id":"c-first","parity":"any","first":true,"named_page":null,"source_order":1}
        ]);
        let input = super::super::super::vector_tests::vector_input(&root, data, &limits);
        let pages = with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            1_000_000_000,
            |pdf, observed| {
                assert!(
                    !infeasible,
                    "a header candidate cannot authorize a too-wide original body figure"
                );
                let display = pdf
                    .navigation()
                    .source()
                    .source()
                    .source()
                    .source()
                    .display();
                let closed = display.source().source();
                assert!(closed.has_header_variants());
                let pages = closed.geometry().pages();
                assert!(pages.len() >= 3);
                let raster = display
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
                assert_eq!(raster.len(), pages.len() + 1);
                assert_eq!(raster.iter().filter(|d| !d.repeated_header()).count(), 2);
                let wide = raster
                    .iter()
                    .filter(|d| d.source().alternative() == "wide first-page body figure")
                    .collect::<Vec<_>>();
                assert_eq!(wide.len(), 1);
                assert_eq!(wide[0].paint().viewport().width().get().raw(), 160 * 65536);
                assert_eq!(wide[0].fragment().fragment().page_index(), 0);
                assert!(!wide[0].repeated_header());
                let actual_widths = pages
                    .iter()
                    .map(|p| {
                        if notes {
                            p.selection().declared_footnote_region().unwrap().width()
                        } else {
                            p.selection().body_bounds().width()
                        }
                    })
                    .map(|w| w.get().raw())
                    .collect::<std::collections::BTreeSet<_>>();
                assert_eq!(
                    actual_widths,
                    [140 * 65536, 180 * 65536, 220 * 65536].into()
                );
                if !notes {
                    let frames = closed.flow().lines().frames().unwrap();
                    let source = closed.flow().lines().prepared().source_flow();
                    let owner = source.tables()[0].owner();
                    if mode == "body" {
                        verify_scoped_frames(
                            frames,
                            owner,
                            source.paragraphs()[0].owner(),
                            wide[0].source().owner(),
                            &limits,
                        );
                    } else if mode.starts_with("nested-") {
                        let (_, work) = frames.table_source_occurrence_projection_budget().unwrap();
                        let width = typaxis_core::PositiveLength::new(
                            typaxis_core::Length::from_raw(140 * 65536).unwrap(),
                        )
                        .unwrap();
                        // This time the wide nested table is actually requested:
                        // its fixed column must fail at the narrow parent.
                        assert!(frames
                            .remeasure_table_parent_for_sources(
                                owner,
                                width,
                                &[wide[0].source().owner()],
                                work,
                                frames.record_charge()
                            )
                            .is_err());
                    }
                }
                super::display::verify_driver_resources(display, &limits);
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                pages.len()
            },
        );
        if infeasible {
            let error = pages.unwrap_err();
            assert!(format!("{error:?}").contains("WidthMismatch"), "{error:?}");
            eprintln!("infeasible first-page body rejected: notes={notes},error={error:?}");
        } else {
            let pages = pages.unwrap();
            eprintln!(
                "fixed body figure header reachability: mode={mode},notes={notes},pages={pages}"
            );
        }
    }
}

fn renumber(v: &mut Value, next: &mut u32) {
    if v.get("node_id").is_some() {
        v["node_id"] = (*next).into();
        *next += 1;
    }
    for key in [
        "blocks",
        "children",
        "items",
        "caption",
        "head",
        "body",
        "cells",
        "footnotes",
    ] {
        if let Some(values) = v.get_mut(key).and_then(Value::as_array_mut) {
            for value in values {
                renumber(value, next);
            }
        }
    }
    if let Some(value) = v.get_mut("equation_number").filter(|v| !v.is_null()) {
        renumber(value, next);
    }
}

fn verify_scoped_frames(
    frames: &typaxis_layout::book_v2::BookV2BodyInlineFrames<'_, '_>,
    root: typaxis_core::NodeId,
    paragraph: typaxis_core::NodeId,
    absent: typaxis_core::NodeId,
    limits: &M4EffectiveResourceLimits,
) {
    use typaxis_core::{Length, NodeId, PositiveLength};
    let width = PositiveLength::new(Length::from_raw(140 * 65536).unwrap()).unwrap();
    let owners = [paragraph];
    let (additional, work) = frames.table_source_occurrence_projection_budget().unwrap();
    let make = |owners, work, prior| {
        frames.remeasure_table_parent_for_sources(root, width, owners, work, prior)
    };
    let projected = make(&owners, work, frames.record_charge()).unwrap();
    projected.verify(frames).unwrap();
    assert!(projected.covers_owner(paragraph));
    assert!(projected.paragraph(0, paragraph).is_some());
    assert!(projected.paragraph(1, paragraph).is_none());
    assert!(projected.paragraph(0, absent).is_none());
    assert!(projected.region(absent).is_none());
    assert!(!projected.covers_owner(absent));
    assert_eq!(projected.work_steps(), work);
    assert_eq!(
        projected.record_charge(),
        frames.record_charge() + additional
    );
    assert!(make(&owners, work - 1, frames.record_charge()).is_err());
    let prior = limits.base().get().max_fragments - additional;
    assert_eq!(
        make(&owners, work, prior).unwrap().record_charge(),
        limits.base().get().max_fragments
    );
    assert!(make(&owners, work, prior + 1).is_err());
    for invalid in [
        vec![paragraph, paragraph],
        vec![absent, paragraph],
        vec![root],
        vec![NodeId::new(u32::MAX)],
    ] {
        assert!(frames
            .remeasure_table_parent_for_sources(root, width, &invalid, work, frames.record_charge())
            .is_err());
    }
}
