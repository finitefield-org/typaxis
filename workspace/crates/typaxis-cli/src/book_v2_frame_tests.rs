use super::*;
use typaxis_core::{Length, NodeId, PositiveLength, Rect};
use typaxis_layout::book_v2::{
    layout_book_v2_body_inline_lines, layout_book_v2_text_lines,
    prepare_book_v2_body_inline_frames, prepare_book_v2_text_inlines,
};
use typaxis_layout::ProductionInlinePreparationErrorKind as E;
use typaxis_linebreak::JapaneseLineBreakMode;

fn rect(x: i64, y: i64, width: i64, height: i64) -> Rect {
    Rect::new(
        Length::from_raw(x).unwrap(),
        Length::from_raw(y).unwrap(),
        PositiveLength::new(Length::from_raw(width).unwrap()).unwrap(),
        PositiveLength::new(Length::from_raw(height).unwrap()).unwrap(),
    )
}
#[test]
fn book_v2_frames_preserve_all_container_kinds_nested_indents_and_exact_owner() {
    for kind in [
        "result",
        "proof",
        "exercise",
        "solution",
        "example",
        "counterexample",
        "remark",
        "note",
        "warning",
        "common_error",
        "formalization_note",
        "quote",
        "exercise_part",
        "choice",
        "hint",
        "assumption",
    ] {
        let root = Root::new();
        let limits = limits();
        let mut data = source_data("Result Proof Result");
        let mut inner = data["document"]["blocks"][0].clone();
        inner["node_id"] = 2.into();
        inner["semantic_kind"] = kind.into();
        inner["blocks"][0]["node_id"] = 3.into();
        inner["blocks"][0]["children"][0]["node_id"] = 4.into();
        data["document"]["blocks"][0]["blocks"] = json!([inner]);
        data["style_sheet"]["rules"][0]["declarations"][2]["value"]["value"] = 100_000.into();
        data["style_sheet"]["rules"][0]["declarations"][3]["value"]["value"] = 200_000.into();
        let input = prepared(&root, data, b"Result Proof Result", &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let shaped =
            shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
                .unwrap();
        let inline = prepare_book_v2_text_inlines(
            &flow,
            &shaped,
            input.resources(),
            &limits,
            EPOCH,
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();
        let body = rect(500_000, 500_000, 10_000_000, 20_000_000);
        let lines = layout_book_v2_body_inline_lines(&inline, body, 1_000_000).unwrap();
        let frames = lines.frames().unwrap();
        frames.verify(&inline, body).unwrap();
        assert_eq!(frames.paragraphs()[0].start().raw(), 200_000);
        assert_eq!(frames.paragraphs()[0].width().get().raw(), 9_400_000);
        assert_eq!(
            frames.region(NodeId::new(2)).unwrap().start().raw(),
            100_000
        );
        assert!(frames.footnote_region().is_none());
        let widths = frames
            .paragraphs()
            .iter()
            .map(|f| f.width())
            .collect::<Vec<_>>();
        let plain = layout_book_v2_text_lines(&inline, &widths, 1_000_000).unwrap();
        assert_eq!(plain.candidate_steps(), lines.candidate_steps());
        assert_eq!(
            plain.output_records() + frames.record_charge(),
            lines.output_records()
        );
        assert_ne!(plain.fingerprint(), lines.fingerprint());
        assert_eq!(
            lines.selected_line_contexts().unwrap().source_fingerprint(),
            lines.fingerprint()
        );
        let other = prepare_book_v2_text_inlines(
            &flow,
            &shaped,
            input.resources(),
            &limits,
            EPOCH,
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();
        assert!(frames.verify(&other, body).is_err());
        assert!(frames
            .verify(&inline, rect(0, 0, 10_000_000, 20_000_000))
            .is_err());
        let err = prepare_book_v2_body_inline_frames(&inline, rect(0, 0, 300_000, 20_000_000))
            .err()
            .unwrap();
        assert!(matches!(err.kind, E::ContainerFrameExhausted));
    }
}
fn framed_data() -> Value {
    let mut data = source_data("Result");
    let span = data["document"]["blocks"][0]["span"].clone();
    let paragraph = |id| json!({"kind":"paragraph","node_id":id,"classes":[],"span":span,"children":[{"kind":"text","node_id":id+1,"span":span,"text_span":{"text_id":0,"start_byte":0,"end_byte":6}}]});
    let mut list_p = paragraph(12);
    list_p["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"footnote_reference","node_id":14,"span":span,"footnote_id":"note"}));
    data["document"]["blocks"][0]["blocks"] = json!([
        {"kind":"table","node_id":2,"span":span,"classes":[],"columns":[{"kind":"fixed","width":3_000_000},{"kind":"fraction","weight":1}],"head":[],"body":[{"node_id":3,"span":span,"cells":[
            {"node_id":4,"span":span,"colspan":1,"rowspan":1,"blocks":[paragraph(5)]},
            {"node_id":7,"span":span,"colspan":1,"rowspan":1,"blocks":[paragraph(8)]}]}]},
        {"kind":"list","node_id":10,"classes":[],"span":span,"ordered":true,"start":1,"items":[{"node_id":11,"span":span,"blocks":[list_p]}]}
    ]);
    data["document"]["footnotes"] =
        json!([{"node_id":15,"span":span,"footnote_id":"note","blocks":[paragraph(16)]}]);
    let mut rule = data["style_sheet"]["rules"][2].clone();
    rule["selector"] = "list".into();
    rule["style_id"] = "list-text".into();
    rule["source_order"] = 3.into();
    data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(rule);
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 12_000_000.into();
    master["height"] = 24_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":12_000_000,"height":24_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":10_000_000,"height":20_000_000});
    master["footnote"] = json!({"x":750_000,"y":21_000_000,"width":9_000_000,"height":2_000_000});
    data
}
#[test]
fn book_v2_frames_measure_table_columns_and_actual_list_footnote_markers() {
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, framed_data(), b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let shaped =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
            .unwrap();
    let inline = prepare_book_v2_text_inlines(
        &flow,
        &shaped,
        input.resources(),
        &limits,
        EPOCH,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    let body = rect(500_000, 500_000, 10_000_000, 20_000_000);
    let lines = layout_book_v2_body_inline_lines(&inline, body, 1_000_000).unwrap();
    let bindings =
        typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body,
        1_000_000,
        |stable| {
            assert!(stable.passes().last().unwrap().is_stable());
            assert_eq!(stable.lines().paragraphs().len(), 4);
            let body_flow = typaxis_pagination::book_v2::prepare_book_v2_body_flow(
                stable.lines(),
                None,
                stable.footnotes(),
                &limits,
                0,
            )
            .unwrap();
            assert_eq!(body_flow.table_count(), 1);
            assert_eq!(body_flow.list_marker_count(), 1);
            assert!(body_flow.definition_items(0).is_some());
            let measured = typaxis_pagination::book_v2::prepare_book_v2_table_measurements(body_flow, &limits).unwrap();
            let table = &measured.tables()[0];
            assert_eq!(table.cells().len(), 2);
            assert_eq!(table.rows().len(), 1);
            assert_eq!(table.height(), table.cells().iter().map(|c| c.natural_height()).max().unwrap());
            assert_eq!(table.rows()[0].height(), table.height());
            for cell in table.cells() {
                assert!(!cell.content().is_empty());
                assert!(cell.content().iter().all(|c| matches!(c.source(), typaxis_pagination::ProductionTableContentSource::FlowItem(i) if measured.item(i).is_some())));
            }


            let final_frames = stable.lines().frames().unwrap();
            assert_eq!(
                final_frames.paragraphs(),
                lines.frames().unwrap().paragraphs()
            );
            assert_eq!(final_frames.lists(), lines.frames().unwrap().lists());
            assert_eq!(
                final_frames.footnotes(),
                lines.frames().unwrap().footnotes()
            );
        },
    )
    .unwrap();
    let frames = lines.frames().unwrap();
    assert_eq!(frames.tables().len(), 1);
    let table = &frames.tables()[0];
    assert_eq!(
        table
            .columns()
            .iter()
            .map(|c| c.final_width().get().raw())
            .collect::<Vec<_>>(),
        [3_000_000, 6_999_991]
    );
    assert_eq!(frames.paragraphs()[0].width().get().raw(), 3_000_000);
    assert_eq!(frames.paragraphs()[1].start().raw(), 3_000_004);
    let list = &frames.lists()[0];
    assert_eq!(list.marker_width(), shaped.list_markers()[0].advance());
    assert_eq!(list.content(), frames.paragraphs()[2]);
    assert_eq!(
        list.content().width().get().raw(),
        9_999_991 - list.marker_width().get().raw() - list.marker_gap().get().raw()
    );
    let footnote = &frames.footnotes()[0];
    assert_eq!(footnote.marker_start().raw(), 250_000);
    assert_eq!(
        footnote.marker_width(),
        shaped.footnote_markers()[0].advance()
    );
    assert_eq!(footnote.content(), frames.paragraphs()[3]);
    assert_eq!(
        footnote.content().width().get().raw(),
        9_000_000 - footnote.marker_width().get().raw() - footnote.marker_gap().get().raw()
    );
    assert_eq!(
        frames.footnote_region(),
        Some(rect(750_000, 21_000_000, 9_000_000, 2_000_000))
    );
    assert!(matches!(
        prepare_book_v2_body_inline_frames(&inline, rect(0, 0, 10_000_000, 20_000_000))
            .err()
            .unwrap()
            .kind,
        E::ReceiptMismatch
    ));
}

#[test]
fn book_v2_frames_reject_missing_and_exhausted_declared_footnote_regions() {
    for missing in [true, false] {
        let root = Root::new();
        let limits = limits();
        let mut data = framed_data();
        if missing {
            data["page_masters"]["masters"][0]["footnote"] = Value::Null;
        } else {
            data["page_masters"]["masters"][0]["footnote"]["width"] = 1.into();
        }
        let input = prepared(&root, data, b"Result", &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let shaped =
            shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
                .unwrap();
        let inline = prepare_book_v2_text_inlines(
            &flow,
            &shaped,
            input.resources(),
            &limits,
            EPOCH,
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();
        let err = prepare_book_v2_body_inline_frames(
            &inline,
            rect(500_000, 500_000, 10_000_000, 20_000_000),
        )
        .err()
        .unwrap();
        assert_eq!(err.owner, NodeId::new(15));
        assert!(matches!(
            (missing, err.kind),
            (true, E::MissingFootnoteRegion) | (false, E::FootnoteFrameExhausted)
        ));
    }
}

#[test]
fn book_v2_body_measures_nested_table_once_and_preserves_rowspan_deficits() {
    let root = Root::new();
    let limits = limits();
    let mut data = framed_data();
    let span = data["document"]["blocks"][0]["span"].clone();
    let paragraph =
        data["document"]["blocks"][0]["blocks"][0]["body"][0]["cells"][0]["blocks"][0].clone();
    let inner = json!({"kind":"table","node_id":0,"span":span,"classes":[],"columns":[{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}],"head":[],"body":[{"node_id":0,"span":span,"cells":[
        {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[paragraph]},
        {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[paragraph,paragraph]}]}]});
    let outer = &mut data["document"]["blocks"][0]["blocks"][0];
    outer["columns"][0]["width"] = 6_000_000.into();
    outer["body"][0]["cells"][0]["rowspan"] = 2.into();
    outer["body"][0]["cells"][0]["blocks"] = json!([inner]);
    outer["body"].as_array_mut().unwrap().push(json!({"node_id":0,"span":span,"cells":[{"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[paragraph]}]}));
    fn renumber(v: &mut Value, next: &mut u32) {
        if v.get("node_id").is_some() {
            v["node_id"] = (*next).into();
            *next += 1;
        }
        for key in [
            "blocks",
            "children",
            "items",
            "head",
            "body",
            "cells",
            "footnotes",
        ] {
            if let Some(children) = v.get_mut(key).and_then(Value::as_array_mut) {
                for child in children {
                    renumber(child, next);
                }
            }
        }
    }
    renumber(&mut data["document"], &mut 0);
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings =
        typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(
        &policy, &flow, input.resources(), &bindings, &limits, JapaneseLineBreakMode::Normal,
        rect(500_000, 500_000, 10_000_000, 20_000_000), 1_000_000,
        |stable| {
            use typaxis_pagination::book_v2::{prepare_book_v2_body_flow, prepare_book_v2_table_measurements};
            use typaxis_pagination::ProductionTableContentSource as Source;
            let make = |prior| prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, prior).unwrap();
            let initial = make(0);
            let body_records = initial.record_charge() - stable.footnotes().record_charge();
            let measured = prepare_book_v2_table_measurements(initial, &limits).unwrap();
            assert_eq!(measured.tables().len(), 2);
            let outer = &measured.tables()[0];
            let inner = &measured.tables()[1];
            assert_eq!(outer.rows().len(), 2);
            assert_eq!(inner.rows().len(), 1);
            assert_eq!(inner.height(), inner.cells().iter().map(|c| c.natural_height()).max().unwrap());
            assert!(inner.height().raw() < inner.cells().iter().map(|c| c.natural_height().raw()).sum::<i64>());
            assert_eq!(outer.cells()[0].content().len(), 1);
            assert_eq!(outer.cells()[0].content()[0].source(), Source::Table(1));
            assert_eq!(outer.cells()[0].natural_height(), inner.height().checked_add(inner.space_before()).unwrap().checked_add(inner.space_after()).unwrap());
            assert_eq!(outer.height(), outer.rows()[0].height().checked_add(outer.rows()[1].height()).unwrap());
            assert!(outer.height() >= outer.cells()[0].natural_height());
            assert_eq!(measured.flow().references().len(), 1);
            let table_records = measured.record_charge() - measured.flow().record_charge();
            let exact_prior = limits.base().get().max_fragments - body_records - table_records;
            assert_eq!(prepare_book_v2_table_measurements(make(exact_prior), &limits).unwrap().record_charge(), limits.base().get().max_fragments);
            assert!(matches!(prepare_book_v2_table_measurements(make(exact_prior + 1), &limits), Err(e) if e.kind == typaxis_pagination::ProductionBodyPaginationErrorKind::FragmentLimit));
            let repeated = prepare_book_v2_table_measurements(make(0), &limits).unwrap();
            assert_eq!(measured.fingerprint(), repeated.fingerprint());
        },
    ).unwrap();
}

#[path = "book_v2_table_break_tests.rs"]
mod table_breaks;

#[test]
fn book_v2_source_widths_rebind_table_list_and_generated_note_sources() {
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, framed_data(), b"Result", &limits);
    super::source_widths::verify_mixed_source_rebinding(
        &input,
        rect(500_000, 500_000, 10_000_000, 20_000_000),
        &limits,
    );
}

#[test]
fn book_v2_page_width_feedback_preserves_repeated_headers_notes_and_unreferenced_source() {
    let root = Root::new();
    let limits = limits();
    let mut data = framed_data();
    let table = &mut data["document"]["blocks"][0]["blocks"][0];
    let row = table["body"][0].clone();
    table["head"] = json!([row.clone()]);
    table["body"] = json!(vec![row; 4]);
    let mut unused = data["document"]["footnotes"][0].clone();
    unused["footnote_id"] = "unused-width-source".into();
    data["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(unused);
    fn renumber(value: &mut Value, next: &mut u32) {
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
            "footnotes",
        ] {
            if let Some(children) = value.get_mut(key).and_then(Value::as_array_mut) {
                for child in children {
                    renumber(child, next);
                }
            }
        }
    }
    renumber(&mut data["document"], &mut 0);
    data["page_masters"]["masters"][0]["body"]["height"] = (40 * 65536).into();
    let input = prepared(&root, data, b"Result", &limits);
    super::page_width_feedback::verify_closed_widths(
        &input,
        rect(500_000, 500_000, 10_000_000, 40 * 65536),
        &limits,
        true,
        1,
    );
}
