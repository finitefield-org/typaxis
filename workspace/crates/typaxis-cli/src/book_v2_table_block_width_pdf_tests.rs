use super::*;
use typaxis_core::{Length, PositiveLength};
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, prepare_book_v2_vector_blocks,
    with_converged_book_v2_body_lines_with_source_widths, BookV2SourceWidthAssignments,
};
use typaxis_shaping::book_v2::shape_book_v2_equation_numbers;

#[test]
fn book_v2_table_blocks_bind_inherited_frames_and_reject_explicit_mismatch() {
    let root = Root::new();
    let limits = driver_limits();
    let mut data = block_width_data("vector", false);
    wrap_tables(&mut data);
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        master["body"]["width"] = (300 * 65536).into();
    }
    let input = vector_input(&root, data, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_for_reflow(
        &flow, &mut 0, 1_000_000, 0, 0,
    )
    .unwrap();
    assert!(!plan.requires_width_reflow());
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};
    let widths = flow
        .events()
        .iter()
        .filter_map(|event| match *event {
            Event::Begin {
                owner,
                kind: Region::VectorFigure | Region::MathVectorBlock,
            } => Some((
                owner,
                PositiveLength::new(Length::from_raw(200 * 65536).unwrap()).unwrap(),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    let profiles = vec![None; flow.paragraphs().len()];
    let run = |inherit, work| {
        let assignments = BookV2SourceWidthAssignments::new(&flow, &profiles)
            .unwrap()
            .with_block_widths(&widths);
        let assignments = if inherit {
            assignments.with_inherited_table_blocks()
        } else {
            assignments
        };
        with_converged_book_v2_body_lines_with_source_widths(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            plan.measurement_body(),
            work,
            None,
            limits.base().get().max_line_reshape_passes,
            Some(&plan),
            Some(&assignments),
            |stable| {
                let lines = stable.lines();
                let frames = lines.frames().unwrap();
                for &(owner, width) in &widths {
                    let actual = frames.region(owner).unwrap();
                    let inherited = frames.inherited_block_region(owner).unwrap();
                    if inherit {
                        assert_eq!(actual, inherited);
                        assert!(actual.width().get() > width.get());
                    } else {
                        assert_eq!(actual.width(), width);
                        assert_ne!(actual, inherited);
                    }
                }
                let numbers =
                    shape_book_v2_equation_numbers(lines.prepared().shaped(), &limits, 0).unwrap();
                let blocks = prepare_book_v2_vector_blocks(lines, numbers.as_ref(), &limits, 0)
                    .unwrap()
                    .unwrap();
                let body =
                    prepare_book_v2_body_flow(lines, Some(&blocks), stable.footnotes(), &limits, 0)
                        .unwrap();
                let measured = prepare_book_v2_table_measurements(body, &limits).unwrap();
                let mut search = prepare_book_v2_table_body_search(
                    &measured,
                    &limits,
                    10_000_000,
                    measured.record_charge(),
                )
                .unwrap();
                let pages = search
                    .select_stable_mixed_pages(limits.base().get().max_layout_passes)
                    .unwrap();
                let placed = search.place_mixed_pages(pages.sequence()).unwrap();
                let closed = search.close_mixed_page_sources(&pages, &placed).unwrap();
                let occurrences = search.table_width_frames(&closed).unwrap();
                let observed_blocks = occurrences
                    .occurrences()
                    .iter()
                    .flat_map(|table| table.pieces())
                    .filter_map(|piece| {
                        assert!(!piece.repeated());
                        match piece.source() {
                            typaxis_pagination::book_v2::BookV2TableWidthSource::Block {
                                owner,
                            } => {
                                assert_eq!(piece.frame(), frames.inherited_block_region(*owner));
                                Some(*owner)
                            }
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    observed_blocks,
                    widths.iter().map(|&(owner, _)| owner).collect::<Vec<_>>()
                );
                assert_eq!(
                    search
                        .paragraph_width_feedback(&closed)
                        .unwrap()
                        .matches_selected_block_widths(),
                    inherit
                );
                let result = search.finalize_mixed_page_math(closed, &limits, 0);
                if inherit {
                    assert!(result.is_ok());
                } else {
                    assert!(
                        matches!(result,Err(e) if e.kind==typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch)
                    );
                }
                (stable.candidate_steps(), lines.fingerprint())
            },
        )
    };
    for inherit in [false, true] {
        let full = run(inherit, 100_000_000).unwrap();
        assert_eq!(run(inherit, full.0).unwrap(), full);
        assert!(run(inherit, full.0 - 1).is_err());
    }
}

pub(super) fn wrap_tables(data: &mut Value) {
    fn wrap(wrapper: Value) -> Value {
        let span = wrapper["span"].clone();
        let cell = |blocks: Value, span_count| json!({"node_id":0,"span":span,"colspan":span_count,"rowspan":1,"blocks":blocks});
        let nested = json!({"kind":"table","node_id":0,"span":span,"classes":[],
            "columns":[{"kind":"fixed","width":20*65536},{"kind":"fraction","weight":1}],
            "head":[],"body":[{"node_id":0,"span":span,"cells":[cell(json!([]),1),cell(json!([wrapper]),1)]}]});
        json!({"kind":"table","node_id":0,"span":span,"classes":[],
            "columns":[{"kind":"fixed","width":20*65536},{"kind":"fraction","weight":1},{"kind":"fraction","weight":2}],
            "head":[],"body":[{"node_id":0,"span":span,"cells":[cell(json!([]),1),cell(json!([nested]),2)]}]})
    }
    let notes = !data["document"]["footnotes"].as_array().unwrap().is_empty();
    let blocks = if notes {
        &mut data["document"]["footnotes"][0]["blocks"]
    } else {
        &mut data["document"]["blocks"]
    };
    for block in blocks.as_array_mut().unwrap() {
        if block["kind"] != "page_break" {
            *block = wrap(block.take());
        }
    }
    for (i, master) in data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        master["width"] = (400 * 65536).into();
        master["height"] = (800 * 65536).into();
        master["trim"] = json!({"x":0,"y":0,"width":400*65536,"height":800*65536});
        let width = [300, 280, 260, 300][i] * 65536;
        master["body"] = json!({"x":10*65536,"y":10*65536,"width":width,"height":300*65536});
        if notes {
            master["footnote"] = json!({"x":20*65536,"y":400*65536,"width":width,"height":350*65536+typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW});
        }
    }
    for rule in data["style_sheet"]["rules"].as_array_mut().unwrap() {
        if rule["selector"] == "paragraph"
            && rule["style_id"]
                .as_str()
                .unwrap()
                .starts_with("block-width-")
        {
            rule["style_id"] = format!("table-{}", rule["style_id"].as_str().unwrap()).into();
        }
    }
    fn number(v: &mut Value, next: &mut u32) {
        if let Some(values) = v.as_array_mut() {
            for value in values {
                number(value, next);
            }
        } else if v.is_object() {
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
                "equation_number",
                "footnotes",
            ] {
                if let Some(child) = v.get_mut(key) {
                    number(child, next);
                }
            }
        }
    }
    number(&mut data["document"], &mut 0);
}

#[test]
fn book_v2_table_blocks_reflow_vector_native_and_raster_pdfs() {
    check_block_width_cases(None, true);
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_blocks_reflow_original_harano_pdfs() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    check_block_width_cases(Some(&font), true);
}
