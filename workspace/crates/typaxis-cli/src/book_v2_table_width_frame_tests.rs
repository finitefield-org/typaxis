use super::*;
use typaxis_core::{Length, NodeId, PositiveLength, Rect};
use typaxis_layout::book_v2::*;
use typaxis_linebreak::JapaneseLineBreakMode;
use typaxis_pagination::book_v2::*;

fn width(raw: i64) -> PositiveLength {
    PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap()
}

pub(super) fn table_data(text: &str, notes: bool) -> Value {
    let mut data = source_data(text);
    let para = data["document"]["blocks"][0]["blocks"][0].clone();
    let span = para["span"].clone();
    let cell = |blocks: Value, colspan, rowspan| {
        json!({
            "node_id":0,"span":span,"colspan":colspan,"rowspan":rowspan,"blocks":blocks
        })
    };
    let nested = json!({"kind":"table","node_id":0,"span":span,"classes":[],
        "columns":[{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}],
        "caption":[para],"head":[],"body":[{"node_id":0,"span":span,
        "cells":[cell(json!([para]),1,1),cell(json!([para]),1,1)]}]});
    let table = json!({"kind":"table","node_id":0,"span":span,"classes":[],
        "columns":[{"kind":"fixed","width":40*65536},{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}],
        "caption":[para],"head":[{"node_id":0,"span":span,
        "cells":[cell(json!([para]),1,1),cell(json!([para]),1,1),cell(json!([para]),1,1)]}],
        "body":[{"node_id":0,"span":span,"cells":[cell(json!([para]),1,2),cell(json!([nested]),2,1)]},
        {"node_id":0,"span":span,"cells":[cell(json!([para]),1,1),cell(json!([para]),1,1)]}]});
    if notes {
        let mut reference = para.clone();
        reference["children"].as_array_mut().unwrap().push(json!({
            "kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"note"
        }));
        data["document"]["blocks"] = json!([reference]);
        data["document"]["footnotes"] =
            json!([{"node_id":0,"span":span,"footnote_id":"note","blocks":[table,table]}]);
    } else {
        data["document"]["blocks"] = json!([table, para, table]);
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    for (selector, start, end) in [("table", 2, 3), ("paragraph", 1, 2)] {
        rules.push(json!({"style_id":format!("width-{selector}"),"selector":selector,"source_order":rules.len(),"extends":null,
            "declarations":[{"name":"start_indent","important":false,"value":{"kind":"length","value":start*65536}},
            {"name":"end_indent","important":false,"value":{"kind":"length","value":end*65536}}]}));
    }
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = (300 * 65536).into();
    master["height"] = (2100 * 65536).into();
    master["trim"] = json!({"x":0,"y":0,"width":300*65536,"height":2100*65536});
    master["body"] = json!({"x":10*65536,"y":10*65536,"width":240*65536,"height":1000*65536});
    if notes {
        master["footnote"] =
            json!({"x":20*65536,"y":1100*65536,"width":230*65536,"height":900*65536});
    }
    super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
    data
}

fn verify_table_frames(font: Option<&[u8]>) {
    for notes in [false, true] {
        let root = Root::new();
        let limits = limits();
        let text = if font.is_some() {
            "表の本文を元の字形で組む"
        } else {
            "Pro Pro Pro"
        };
        let mut data = table_data(text, notes);
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
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let foreign = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        assert_eq!(flow.tables().len(), 4);
        let owners = [flow.tables()[0].owner(), flow.tables()[2].owner()];
        let widths = [
            (owners[0], width(180 * 65536 + 1)),
            (owners[1], width(160 * 65536 + 3)),
        ];
        let profiles = vec![None; flow.paragraphs().len()];
        let body = Rect::new(
            Length::from_raw(10 * 65536).unwrap(),
            Length::from_raw(10 * 65536).unwrap(),
            width(240 * 65536),
            width(1000 * 65536),
        );
        let run = |assignments: &BookV2SourceWidthAssignments<'_, '_>, work, passes| {
            with_converged_book_v2_body_lines_with_source_widths(
                &policy,
                &flow,
                input.resources(),
                &bindings,
                &limits,
                JapaneseLineBreakMode::Normal,
                body,
                work,
                None,
                passes,
                None,
                Some(assignments),
                |stable| {
                    let lines = stable.lines();
                    let frames = lines.frames().unwrap();
                    assert!(frames.has_table_width_candidates());
                    assert_eq!(frames.body(), body);
                    assert_eq!(frames.tables().len(), frames.measurement_tables().len());
                    for root_index in 0..2 {
                        let table = &frames.tables()[root_index * 2];
                        let original = &frames.measurement_tables()[root_index * 2];
                        let parent = widths[root_index].1.get().raw();
                        let available = parent - 5 * 65536;
                        let remaining = available - 40 * 65536;
                        // The two fixtures exercise both signs of the residual.
                        let (left, right, residual) = if root_index == 0 {
                            (67 * 65536 + 32768, 67 * 65536 + 32769, 1)
                        } else {
                            (57 * 65536 + 32770, 57 * 65536 + 32769, -1)
                        };
                        assert_eq!(table.owner(), owners[root_index]);
                        assert_eq!(table.content().start(), original.content().start());
                        assert_eq!(table.content().width().get().raw(), available);
                        assert!(original.content().width().get() > table.content().width().get());
                        assert_eq!(
                            table
                                .columns()
                                .iter()
                                .map(|c| c.final_width().get().raw())
                                .collect::<Vec<_>>(),
                            vec![40 * 65536, left, right]
                        );
                        assert_eq!(table.rounding_residual().raw(), residual);
                        assert_eq!(table.last_fraction(), Some(2));
                        let nested = &frames.tables()[root_index * 2 + 1];
                        assert_eq!(
                            nested.content().start().raw(),
                            table.content().start().raw() + 42 * 65536
                        );
                        assert_eq!(nested.content().width().get().raw(), remaining - 5 * 65536);
                        assert_eq!(
                            nested.columns()[0].final_width().get().raw(),
                            if root_index == 0 {
                                65 * 65536
                            } else {
                                55 * 65536 + 2
                            }
                        );
                        assert_eq!(
                            nested.columns()[1].final_width().get().raw(),
                            if root_index == 0 {
                                65 * 65536 + 1
                            } else {
                                55 * 65536 + 1
                            }
                        );
                        assert_eq!(
                            frames
                                .measurement_region(table.owner())
                                .unwrap()
                                .width()
                                .get()
                                .raw(),
                            original.content().width().get().raw() + 5 * 65536
                        );
                    }
                    let baseline =
                        layout_book_v2_body_inline_lines(lines.prepared(), body, 100_000_000)
                            .unwrap();
                    assert!(
                        frames.record_charge() >= 2 * baseline.frames().unwrap().record_charge()
                    );
                    check_occurrence_projection(
                        frames,
                        baseline.frames().unwrap(),
                        &widths,
                        limits.base().get().max_fragments,
                    );
                    assert!(lines
                        .paragraphs()
                        .iter()
                        .zip(baseline.paragraphs())
                        .any(|(p, old)| p.lines().len() > old.lines().len()));
                    let mut changed = 0;
                    for ((p, current), original) in lines
                        .paragraphs()
                        .iter()
                        .zip(frames.paragraphs())
                        .zip(frames.measurement_paragraphs())
                    {
                        assert_eq!(p.inline_size(), current.width());
                        changed += usize::from(current.width() != original.width());
                        let mut actual = String::new();
                        for line in p.lines() {
                            for atom in line.items() {
                                if let typaxis_layout::ProductionPlacedInline::Text(c) = atom {
                                    actual.push_str(c.utf8());
                                    for glyph in c.glyphs() {
                                        assert!(std::ptr::eq(
                                            glyph.glyph(),
                                            &c.run().glyph_run().glyphs
                                                [glyph.glyph_index() as usize]
                                        ));
                                    }
                                }
                            }
                        }
                        assert!(actual.starts_with(text), "{actual}");
                    }
                    assert!(changed > 10);
                    let envelopes = frames
                        .paragraphs()
                        .iter()
                        .map(|p| p.width())
                        .collect::<Vec<_>>();
                    assert!(layout_book_v2_source_width_lines_from_flow(
                        lines.prepared(),
                        &envelopes,
                        assignments,
                        work,
                        0
                    )
                    .is_err());
                    let body_flow =
                        prepare_book_v2_body_flow(lines, None, stable.footnotes(), &limits, 0)
                            .unwrap();
                    let measured = prepare_book_v2_table_measurements(body_flow, &limits).unwrap();
                    let mut search = prepare_book_v2_table_body_search(
                        &measured,
                        &limits,
                        100_000_000,
                        measured.record_charge(),
                    )
                    .unwrap();
                    let pages = search
                        .select_stable_mixed_pages(limits.base().get().max_layout_passes)
                        .unwrap();
                    let placed = search.place_mixed_pages(pages.sequence()).unwrap();
                    let closed = search.close_mixed_page_sources(&pages, &placed).unwrap();
                    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
                    let feedback = search.paragraph_width_feedback(&closed).unwrap();
                    assert!(!feedback.matches_selected_table_widths());
                    assert_eq!(feedback.root_table_widths().len(), 2);
                    assert!(
                        matches!(search.finalize_mixed_page_math(closed,&limits,0), Err(e) if e.kind == E::WidthMismatch)
                    );
                    (
                        stable.candidate_steps(),
                        stable.passes().len() as u16,
                        lines.fingerprint(),
                        changed,
                    )
                },
            )
        };
        let assignment = BookV2SourceWidthAssignments::new(&flow, &profiles)
            .unwrap()
            .with_root_table_widths(&widths);
        let full = run(
            &assignment,
            100_000_000,
            limits.base().get().max_line_reshape_passes,
        )
        .unwrap();
        assert_eq!(run(&assignment, full.0, full.1).unwrap(), full);
        assert!(run(&assignment, full.0 - 1, full.1).is_err());
        assert!(run(&assignment, full.0, full.1 - 1).is_err());
        let foreign = BookV2SourceWidthAssignments::new(&foreign, &profiles)
            .unwrap()
            .with_root_table_widths(&widths);
        assert!(run(&foreign, 100_000_000, full.1).is_err());
        for invalid in [
            vec![widths[1], widths[0]],
            vec![widths[0]],
            vec![widths[0], widths[0]],
            vec![widths[0], widths[1], widths[1]],
            vec![(flow.tables()[1].owner(), widths[0].1), widths[1]],
            vec![(NodeId::new(0), widths[0].1), widths[1]],
            vec![(owners[0], width(250 * 65536)), widths[1]],
            vec![(owners[0], width(45 * 65536)), widths[1]],
        ] {
            let invalid = BookV2SourceWidthAssignments::new(&flow, &profiles)
                .unwrap()
                .with_root_table_widths(&invalid);
            assert!(run(&invalid, 100_000_000, full.1).is_err());
        }
        eprintln!(
            "table width frames: notes={notes}, harano={}, work={}, reshape={}, changed={}",
            font.is_some(),
            full.0,
            full.1,
            full.3
        );
    }
}

#[test]
fn book_v2_root_table_widths_reproject_nested_cells_and_original_shaping() {
    verify_table_frames(None);
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_root_table_widths_reproject_original_harano() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    verify_table_frames(Some(&bytes));
}

fn check_occurrence_projection(
    frames: &BookV2BodyInlineFrames<'_, '_>,
    baseline: &BookV2BodyInlineFrames<'_, '_>,
    widths: &[(NodeId, PositiveLength)],
    maximum_records: u64,
) {
    let original_hash = frames.fingerprint();
    let (records, work) = frames.table_occurrence_projection_budget().unwrap();
    for (root_index, &(owner, width)) in widths.iter().enumerate() {
        let selected = frames
            .remeasure_table_parent(owner, width, work, frames.record_charge())
            .unwrap();
        selected.verify(frames).unwrap();
        assert!(selected.verify(baseline).is_err());
        assert_eq!(selected.owner(), owner);
        assert_eq!(selected.parent_width(), width);
        assert_eq!(selected.work_steps(), work);
        assert_eq!(selected.record_charge(), frames.record_charge() + records);
        for (index, actual) in selected.tables().iter().enumerate() {
            // The other source root goes back to its maximum measurement;
            // only the requested root and its nested table use the candidate.
            let expected = if index / 2 == root_index {
                &frames.tables()[index]
            } else {
                &frames.measurement_tables()[index]
            };
            assert_eq!(actual.owner(), expected.owner());
            assert_eq!(actual.content(), expected.content());
            assert_eq!(actual.rounding_residual(), expected.rounding_residual());
            assert_eq!(actual.last_fraction(), expected.last_fraction());
            assert_eq!(
                actual
                    .columns()
                    .iter()
                    .map(|c| c.final_width())
                    .collect::<Vec<_>>(),
                expected
                    .columns()
                    .iter()
                    .map(|c| c.final_width())
                    .collect::<Vec<_>>()
            );
        }
        let exact = frames
            .remeasure_table_parent(owner, width, work, maximum_records - records)
            .unwrap();
        assert_eq!(exact.record_charge(), maximum_records);
        assert_eq!(exact.fingerprint(), selected.fingerprint());
        assert!(frames
            .remeasure_table_parent(owner, width, work, maximum_records - records + 1)
            .is_err());
        assert!(frames
            .remeasure_table_parent(owner, width, work - 1, frames.record_charge())
            .is_err());
        let too_wide = PositiveLength::new(
            frames
                .measurement_region(owner)
                .unwrap()
                .width()
                .get()
                .checked_add(Length::from_raw(1).unwrap())
                .unwrap(),
        )
        .unwrap();
        for (bad_owner, bad_width) in [
            (NodeId::new(0), width),
            (frames.tables()[root_index * 2 + 1].owner(), width),
            (owner, too_wide),
            (
                owner,
                PositiveLength::new(Length::from_raw(45 * 65536).unwrap()).unwrap(),
            ),
        ] {
            assert!(frames
                .remeasure_table_parent(bad_owner, bad_width, work, frames.record_charge())
                .is_err());
        }
    }
    assert_eq!(frames.fingerprint(), original_hash);
}
