use super::*;
use typaxis_core::{Length, PositiveLength, Rect};
use typaxis_layout::book_v2::*;
use typaxis_linebreak::JapaneseLineBreakMode;
use typaxis_pagination::book_v2::*;

fn check_feedback(align: &str, font: Option<&[u8]>, empty: bool) {
    let root = Root::new();
    let limits = limits();
    let text = if font.is_some() {
        "左側右側左側右側左側右側"
    } else {
        "R R R R R R"
    };
    let mut data = source_data(text);
    data["document"]["footnotes"] = json!([]);
    if empty {
        let p = &mut data["document"]["blocks"][0]["blocks"][0];
        p["children"] = json!([{"kind":"anchor","node_id":3,"span":p["span"],"anchor_id":"empty-width-feedback"}]);
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"width-feedback-alignment","selector":"paragraph","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"text_align","important":false,"value":{"kind":"keyword","value":align}}]}));
    let raw = |n| Length::from_raw(n).unwrap();
    let width = |n| PositiveLength::new(raw(n)).unwrap();
    let body = Rect::new(
        raw(10 * 65536),
        raw(10 * 65536),
        width(120 * 65536),
        width(24 * 65536),
    );
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = (200 * 65536).into();
    master["height"] = (100 * 65536).into();
    master["trim"] = json!({"x":0,"y":0,"width":200*65536,"height":100*65536});
    master["body"] = json!({"x":10*65536,"y":10*65536,"width":120*65536,"height":24*65536});
    let input = if let Some(font) = font {
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            .into();
        let admitted = body_with_source(&root, data, text.as_bytes(), &limits);
        fs::write(root.0.join("body.bin"), font).unwrap();
        prepare_book_v2_resources(
            admitted,
            &root.context(),
            &config(limits.base().get().clone()),
            &limits,
        )
        .unwrap()
    } else {
        prepared(&root, data, text.as_bytes(), &limits)
    };
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let shape = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let inlines = prepare_book_v2_inline_items(
        &flow,
        &shape,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    let items = inlines.paragraphs()[0].items().unwrap();
    let small = if empty {
        12 * 65536
    } else {
        items.units()[..2]
            .iter()
            .map(|u| {
                let typaxis_linebreak::ProductionInlineLogicalUnit::Text(t) = u else {
                    panic!("text")
                };
                t.advance().get().raw()
            })
            .sum::<i64>()
    };
    let mut seed = vec![width(small * 2); items.units().len().max(1)];
    seed[0] = width(small);
    let run = |flow: &typaxis_syntax::book_v2::PreparedBookV2TextFlow<'_>,
               profiles: &[Option<&[PositiveLength]>],
               work: u64| {
        let assignments = BookV2SourceWidthAssignments::new(flow, profiles).unwrap();
        with_converged_book_v2_body_lines_with_source_widths(
            &policy,
            flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            body,
            work,
            None,
            limits.base().get().max_line_reshape_passes,
            None,
            Some(&assignments),
            |stable| -> Result<_, typaxis_pagination::ProductionBodyPaginationError> {
                let body_flow = prepare_book_v2_body_flow(
                    stable.lines(),
                    None,
                    stable.footnotes(),
                    &limits,
                    0,
                )?;
                let measured = prepare_book_v2_table_measurements(body_flow, &limits)?;
                let remaining = work - stable.candidate_steps();
                let mut search = prepare_book_v2_table_body_search(
                    &measured,
                    &limits,
                    remaining,
                    measured.record_charge(),
                )?;
                let pages =
                    search.select_stable_mixed_pages(limits.base().get().max_layout_passes)?;
                let placed = search.place_mixed_pages(pages.sequence())?;
                let closed = search.close_mixed_page_sources(&pages, &placed)?;
                let p = &stable.lines().paragraphs()[0];
                let frame = stable.lines().frames().unwrap().paragraphs()[0];
                let mut observed = 0;
                for page in placed.pages() {
                    for placed in page.fragments() {
                        let fragment = placed.fragment();
                        let typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine {
                            paragraph_index,
                            line_index,
                        } = fragment.source()
                        else {
                            panic!("text fragment")
                        };
                        assert_eq!(paragraph_index, 0);
                        let line = &p.selected().unwrap().lines()[line_index as usize];
                        let slack = line.inline_size().get().raw()
                            - line.required_inline_size().get().raw();
                        let offset = if empty {
                            0
                        } else {
                            match align {
                                "start" => 0,
                                "end" => slack,
                                "center" => slack / 2,
                                _ => unreachable!(),
                            }
                        };
                        assert_eq!(
                            fragment.bounds().x().raw(),
                            body.x().raw() + frame.start().raw() + offset
                        );
                        assert_eq!(
                            fragment.bounds().width().get(),
                            if empty {
                                line.inline_size().get()
                            } else {
                                line.required_inline_size().get()
                            }
                        );
                        observed += 1;
                    }
                }
                assert_eq!(observed, p.lines().len());
                let mut foreign = prepare_book_v2_table_body_search(
                    &measured,
                    &limits,
                    remaining,
                    measured.record_charge(),
                )?;
                assert!(
                    matches!(foreign.paragraph_width_feedback(&closed), Err(e) if e.kind==typaxis_pagination::ProductionBodyPaginationErrorKind::ReceiptMismatch)
                );
                let feedback = search.paragraph_width_feedback(&closed)?;
                assert!(feedback.matches_source_flow(flow));
                assert_eq!(feedback.paragraphs().len(), 1);
                assert_eq!(feedback.paragraphs()[0].owner(), p.owner());
                assert!(feedback.paragraphs()[0]
                    .widths()
                    .iter()
                    .all(|w| *w == p.inline_size()));
                let total_work = stable.candidate_steps() + feedback.work_steps();
                Ok((feedback, placed.pages().len(), total_work))
            },
        )
    };
    let profiles = [Some(seed.as_slice())];
    let first = run(&flow, &profiles, 1_000_000).unwrap().unwrap();
    assert!(!first.0.matches_selected_line_widths());
    assert!(empty || first.1 > 1);
    assert!(run(&flow, &profiles, first.2).unwrap().is_ok());
    let short = run(&flow, &profiles, first.2 - 1);
    assert!(short.is_err() || short.unwrap().is_err());
    let replay = first
        .0
        .paragraphs()
        .iter()
        .map(|p| Some(p.widths()))
        .collect::<Vec<_>>();
    let next_flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    assert!(first.0.matches_source_flow(&next_flow));
    let second = run(&next_flow, &replay, 1_000_000).unwrap().unwrap();
    assert!(second.0.matches_selected_line_widths());
    assert!(if empty {
        second.1 == 1
    } else {
        second.1 < first.1
    });
    if let Some(directory) = std::env::var_os("TYPAXIS_BOOK_V2_PAGE_WIDTH_FEEDBACK_PROBE") {
        let directory = std::path::PathBuf::from(directory);
        fs::create_dir_all(&directory).unwrap();
        let case = if font.is_some() {
            "harano"
        } else if empty {
            "controlled-empty"
        } else {
            "controlled"
        };
        fs::write(directory.join(format!("{case}-{align}.json")),
            serde_json::to_vec_pretty(&json!({"text":text,"empty":empty,"align":align,"first_pages":first.1,"next_pages":second.1,
                "first_work":first.2,"next_work":second.2,"records":second.0.record_charge(),
                "widths":second.0.paragraphs()[0].widths().iter().map(|w|w.get().raw()).collect::<Vec<_>>()})).unwrap()).unwrap();
    }
}
#[test]
fn book_v2_page_width_feedback_restores_physical_width_and_aligns_each_selected_line() {
    for align in ["start", "end", "center"] {
        check_feedback(align, None, false);
        check_feedback(align, None, true);
    }
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_page_width_feedback_replays_original_harano_source() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    for align in ["start", "end", "center"] {
        check_feedback(align, Some(&font), false);
    }
}

pub(super) fn verify_closed_widths(
    input: &PreparedBookV2Resources,
    body: Rect,
    limits: &M4EffectiveResourceLimits,
    repeated: bool,
    unreferenced: usize,
) {
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), limits).unwrap();
    with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        limits,
        JapaneseLineBreakMode::Normal,
        body,
        1_000_000,
        |stable| {
            let body_flow =
                prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), limits, 0)
                    .unwrap();
            let measured = prepare_book_v2_table_measurements(body_flow, limits).unwrap();
            let run =
                |prior, maximum| -> Result<_, typaxis_pagination::ProductionBodyPaginationError> {
                    let mut search =
                        prepare_book_v2_table_body_search(&measured, limits, maximum, prior)?;
                    let pages =
                        search.select_stable_mixed_pages(limits.base().get().max_layout_passes)?;
                    let placed = search.place_mixed_pages(pages.sequence())?;
                    let closed = search.close_mixed_page_sources(&pages, &placed)?;
                    assert_eq!(closed.repeated_fragments() > 0, repeated);
                    assert_eq!(closed.unreferenced_definitions(), unreferenced);
                    let feedback = search.paragraph_width_feedback(&closed)?;
                    assert!(feedback.matches_selected_line_widths());
                    assert!(feedback.matches_source_flow(&flow));
                    for (p, expected) in feedback
                        .paragraphs()
                        .iter()
                        .zip(stable.lines().paragraphs())
                    {
                        assert_eq!(p.owner(), expected.owner());
                        assert!(p.widths().iter().all(|w| *w == expected.inline_size()));
                    }
                    Ok(feedback)
                };
            let baseline = measured.record_charge();
            let full = run(baseline, 1_000_000).unwrap();
            let prior = baseline + limits.base().get().max_fragments - full.record_charge();
            assert_eq!(
                run(prior, full.work_steps()).unwrap().record_charge(),
                limits.base().get().max_fragments
            );
            assert!(run(prior + 1, full.work_steps()).is_err());
            assert!(run(baseline, full.work_steps() - 1).is_err());
        },
    )
    .unwrap();
}
