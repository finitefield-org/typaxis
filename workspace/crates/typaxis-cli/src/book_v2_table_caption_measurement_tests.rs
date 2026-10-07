use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
    prepare_book_v2_table_measurements, prepare_book_v2_table_search,
};
use typaxis_pagination::{
    ProductionBodyPaginationErrorKind as Error, ProductionTableContentSource as Source,
};

fn captioned(depth: usize, headers: bool) -> Value {
    let mut data = source_data("Result");
    let paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    fn table(paragraph: &Value, depth: usize, headers: bool) -> Value {
        let span = &paragraph["span"];
        let cell = json!({"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[paragraph]});
        let mut caption = vec![paragraph.clone()];
        if depth > 0 {
            caption.push(table(paragraph, depth - 1, headers));
        }
        let row = json!({"node_id":0,"span":span,"cells":[cell,cell]});
        let head = if headers {
            vec![row.clone()]
        } else {
            Vec::new()
        };
        json!({"kind":"table","node_id":0,"span":span,"classes":[],
            "columns":[{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}],
            "caption":caption,"head":head,"body":[row]})
    }
    data["document"]["blocks"] = json!([table(&paragraph, depth, headers)]);
    renumber(&mut data["document"], &mut 0);
    data
}

fn renumber(v: &mut Value, next: &mut u32) {
    if let Some(a) = v.as_array_mut() {
        for c in a {
            renumber(c, next);
        }
    } else if let Some(o) = v.as_object_mut() {
        if let Some(id) = o.get_mut("node_id") {
            *id = (*next).into();
            *next += 1;
        }
        for key in [
            "blocks",
            "children",
            "caption",
            "head",
            "body",
            "cells",
            "footnotes",
        ] {
            if let Some(c) = o.get_mut(key) {
                renumber(c, next);
            }
        }
    }
}

#[test]
fn book_v2_table_caption_pdf_demands_a_continued_note_before_header_references() {
    let root = Root::new();
    let limits = limits();
    let mut data = continued_caption(true, 5);
    let table = &mut data["document"]["blocks"][0];
    let paragraph = table["caption"][0].clone();
    let span = paragraph["span"].clone();
    let reference =
        json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"note"});
    table["caption"][1]["children"]
        .as_array_mut()
        .unwrap()
        .push(reference.clone());
    table["head"][0]["cells"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(reference);
    data["document"]["footnotes"] = json!([
        {"node_id":0,"span":span,"footnote_id":"note","blocks":vec![paragraph;8]}
    ]);
    renumber(&mut data["document"], &mut 0);
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 12_000_000.into();
    master["height"] = 24_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":12_000_000,"height":24_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":10_000_000,"height":3_000_000});
    master["footnote"] = json!({"x":750_000,"y":21_000_000,"width":9_000_000,"height":2_000_000});
    let input = prepared(&root, data, b"Result", &limits);
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            let registry = pdf.navigation().source().source();
            assert!(registry.source().pages().len() > 4);
            assert_eq!(
                registry
                    .nodes()
                    .iter()
                    .filter(|n| n.source().pdf_role() == "Note")
                    .count(),
                1
            );
            assert_eq!(
                registry
                    .nodes()
                    .iter()
                    .filter(|n| n.source().pdf_role() == "Reference")
                    .count(),
                2
            );
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}

fn continued_caption(headers: bool, paragraphs: usize) -> Value {
    let mut data = captioned(0, headers);
    let table = &mut data["document"]["blocks"][0];
    let paragraph = table["caption"][0].clone();
    table["caption"] = vec![paragraph; paragraphs].into();
    let row = table["body"][0].clone();
    table["body"] = vec![row; 4].into();
    renumber(&mut data["document"], &mut 0);
    data
}

#[test]
fn book_v2_table_caption_measures_its_full_width_and_parallel_nested_tables() {
    for (depth, headers) in (0..=2).flat_map(|depth| [false, true].map(|headers| (depth, headers)))
    {
        let root = Root::new();
        let limits = limits();
        let input = prepared(&root, captioned(depth, headers), b"Result", &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings =
            typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits)
                .unwrap();
        let raw = |n| typaxis_core::Length::from_raw(n).unwrap();
        let body = typaxis_core::Rect::new(
            raw(0),
            raw(0),
            typaxis_core::PositiveLength::new(raw(10_000_000)).unwrap(),
            typaxis_core::PositiveLength::new(raw(10_000_000)).unwrap(),
        );
        typaxis_layout::book_v2::with_converged_book_v2_body_lines(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            typaxis_linebreak::JapaneseLineBreakMode::Normal,
            body,
            1_000_000,
            |stable| {
                let frames = stable.lines().frames().unwrap();
                assert_eq!(frames.tables().len(), depth + 1);
                assert_eq!(
                    frames.paragraphs()[0].width(),
                    frames.tables()[0].content().width()
                );
                let make = |prior| {
                    prepare_book_v2_body_flow(
                        stable.lines(),
                        None,
                        stable.footnotes(),
                        &limits,
                        prior,
                    )
                    .unwrap()
                };
                let initial = make(0);
                let body_records = initial.record_charge() - stable.footnotes().record_charge();
                let measured = prepare_book_v2_table_measurements(initial, &limits).unwrap();
                assert_eq!(measured.tables().len(), depth + 1);
                for (index, table) in measured.tables().iter().enumerate() {
                    let caption = table.caption().unwrap();
                    let unit = table.cells()[0].natural_height();
                    assert_eq!(table.cells().len(), if headers { 4 } else { 2 });
                    assert_eq!(table.rows().len(), if headers { 2 } else { 1 });
                    assert_eq!(table.rows()[0].height(), unit);
                    assert_eq!(table.rows()[0].top(), caption.height());
                    let row_height = table.rows().iter().fold(raw(0), |height, row| {
                        height.checked_add(row.height()).unwrap()
                    });
                    assert_eq!(
                        table.height(),
                        caption.height().checked_add(row_height).unwrap()
                    );
                    if headers {
                        assert_eq!(
                            table.rows()[0].section(),
                            typaxis_syntax::ProductionTableSection::Head
                        );
                        assert_eq!(
                            table.rows()[1].top(),
                            caption.height().checked_add(unit).unwrap()
                        );
                    }
                    assert_eq!(table.cells()[1].natural_height(), unit);
                    assert!(matches!(caption.content()[0].source(), Source::FlowItem(_)));
                    let caption_first = caption.content()[0];
                    assert_eq!(caption_first.top(), raw(0));
                    assert_eq!(caption_first.end(), unit);
                    if index < depth {
                        let nested = &measured.tables()[index + 1];
                        assert_eq!(caption.content().len(), 2);
                        assert_eq!(caption.content()[1].source(), Source::Table(index + 1));
                        assert_eq!(caption.content()[1].top(), unit);
                        assert_eq!(caption.height(), unit.checked_add(nested.height()).unwrap());
                    } else {
                        assert_eq!(caption.content().len(), 1);
                        assert_eq!(caption.height(), unit);
                    }
                    // Cells never acquire the caption's source leaf range.
                    for cell in table.cells() {
                        assert_eq!(cell.content().len(), 1);
                        assert_ne!(cell.content()[0].source(), caption_first.source());
                    }
                    let search =
                        prepare_book_v2_table_search(&measured, index, &limits, 1_000_000, 0);
                    {
                        let mut search = search.unwrap();
                        let cursor = search.begin().unwrap();
                        let fragment = search
                            .evaluate(&cursor, search.maximum_height())
                            .unwrap()
                            .unwrap();
                        assert!(fragment.after().is_terminal());
                        assert_eq!(fragment.used_height(), table.height());
                        assert_eq!(
                            fragment.caption_placement_leaves().count(),
                            depth - index + 1
                        );
                        assert!(!fragment.repeats_header());
                    }
                }
                let body_search =
                    prepare_book_v2_table_body_search(&measured, &limits, 1_000_000, 0);
                {
                    let mut search = body_search.unwrap();
                    let pages = search.select_stable_mixed_pages(2).unwrap();
                    let placed = search.place_mixed_pages(pages.sequence()).unwrap();
                    assert_eq!(placed.pages().len(), 1);
                    assert_eq!(placed.pages()[0].cell_roles()[0], None);
                    search.close_mixed_page_sources(&pages, &placed).unwrap();
                }
                let table_records = measured.record_charge() - measured.flow().record_charge();
                let exact_prior = limits.base().get().max_fragments - body_records - table_records;
                assert_eq!(
                    prepare_book_v2_table_measurements(make(exact_prior), &limits)
                        .unwrap()
                        .record_charge(),
                    limits.base().get().max_fragments
                );
                assert_eq!(
                    prepare_book_v2_table_measurements(make(exact_prior + 1), &limits)
                        .err()
                        .unwrap()
                        .kind,
                    Error::FragmentLimit
                );
                let repeated = prepare_book_v2_table_measurements(make(0), &limits).unwrap();
                assert_eq!(measured.fingerprint(), repeated.fingerprint());
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_table_caption_continues_before_first_header_without_duplicate_cell_roles() {
    let root = Root::new();
    let limits = limits();
    let data = continued_caption(true, 5);
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings =
        typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let raw = |n| typaxis_core::Length::from_raw(n).unwrap();
    let body = typaxis_core::Rect::new(
        raw(0),
        raw(0),
        typaxis_core::PositiveLength::new(raw(10_000_000)).unwrap(),
        typaxis_core::PositiveLength::new(raw(3 * 16 * 65536)).unwrap(),
    );
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        typaxis_linebreak::JapaneseLineBreakMode::Normal,
        body,
        1_000_000,
        |stable| {
            let measured = prepare_book_v2_table_measurements(
                prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0)
                    .unwrap(),
                &limits,
            )
            .unwrap();
            let mut search =
                prepare_book_v2_table_search(&measured, 0, &limits, 1_000_000, 0).unwrap();
            let mut cursor = search.begin().unwrap();
            let mut caption_counts = Vec::new();
            let mut repeated = Vec::new();
            let mut semantics = Vec::new();
            while !cursor.is_terminal() {
                let fragment = search
                    .evaluate(&cursor, search.maximum_height())
                    .unwrap()
                    .unwrap();
                caption_counts.push(fragment.caption_placement_leaves().count());
                repeated.push(fragment.repeats_header());
                semantics.extend(fragment.semantic_leaf_ranges().flatten());
                if caption_counts.len() <= 2 {
                    assert!(fragment.cells().is_empty());
                    assert_eq!(fragment.header_height(), raw(0));
                }
                cursor = fragment.after();
            }
            assert_eq!(caption_counts, [3, 2, 0, 0]);
            assert_eq!(repeated, [false, false, false, true]);
            assert_eq!(semantics, (0..15).collect::<Vec<_>>());
            assert_eq!(cursor.offset(), measured.tables()[0].height());
            let replay =
                |prior, work| -> Result<_, typaxis_pagination::ProductionBodyPaginationError> {
                    let mut search =
                        prepare_book_v2_table_search(&measured, 0, &limits, work, prior)?;
                    let initial = search.begin()?;
                    assert!(search.evaluate(&initial, raw(0))?.is_none());
                    let mut cursor = initial;
                    let mut fingerprints = Vec::new();
                    while !cursor.is_terminal() {
                        let fragment = search.evaluate(&cursor, search.maximum_height())?.unwrap();
                        fingerprints.push(fragment.fingerprint());
                        cursor = fragment.after();
                    }
                    assert!(initial.is_initial());
                    assert_eq!(initial.offset(), raw(0));
                    Ok((search.record_charge(), search.work_charge(), fingerprints))
                };
            let (records, work, fingerprints) = replay(0, 1_000_000).unwrap();
            let prior = limits.base().get().max_fragments - (records - measured.record_charge());
            assert_eq!(
                replay(prior, work).unwrap(),
                (limits.base().get().max_fragments, work, fingerprints)
            );
            assert_eq!(
                replay(prior + 1, work).unwrap_err().kind,
                Error::FragmentLimit
            );
            assert_eq!(
                replay(0, work - 1).unwrap_err().kind,
                Error::TableSearchLimit
            );
            let mut search =
                prepare_book_v2_table_body_search(&measured, &limits, 1_000_000, 0).unwrap();
            let pages = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(pages.sequence()).unwrap();
            assert_eq!(placed.pages().len(), 4);
            assert!(placed.pages()[0]
                .cell_roles()
                .iter()
                .chain(placed.pages()[1].cell_roles())
                .all(Option::is_none));
            assert!(placed.pages()[2]
                .cell_roles()
                .iter()
                .all(|c| c.is_some_and(|c| !c.repeated_header())));
            assert_eq!(
                placed.pages()[3]
                    .cell_roles()
                    .iter()
                    .filter(|c| c.is_some_and(|c| c.repeated_header()))
                    .count(),
                2
            );
            assert_eq!(
                placed.pages().iter().flat_map(|p| p.fragments()).count(),
                17
            );
            search.close_mixed_page_sources(&pages, &placed).unwrap();
        },
    )
    .unwrap();
}

#[test]
fn book_v2_table_caption_pdf_preserves_caption_structure_across_pages() {
    for (headers, captions, expected_pages) in
        [(false, 1, 2), (false, 5, 3), (true, 1, 3), (true, 5, 4)]
    {
        let root = Root::new();
        let limits = limits();
        let mut data = continued_caption(headers, captions);
        // Distinct source text makes accidental caption repetition visible in
        // independently extracted PDF text as well as the structure tree.
        data["text_buffers"] = source_data("ResultProof")["text_buffers"].clone();
        fn row_text(value: &mut Value) {
            if let Some(values) = value.as_array_mut() {
                for value in values {
                    row_text(value);
                }
            } else if let Some(object) = value.as_object_mut() {
                if let Some(span) = object.get_mut("span") {
                    span["start_byte"] = 6.into();
                    span["end_byte"] = 11.into();
                }
                if let Some(span) = object.get_mut("text_span") {
                    span["start_byte"] = 6.into();
                    span["end_byte"] = 11.into();
                }
                for key in ["cells", "blocks", "children"] {
                    if let Some(child) = object.get_mut(key) {
                        row_text(child);
                    }
                }
            }
        }
        let table = &mut data["document"]["blocks"][0];
        table["span"]["end_byte"] = 11.into();
        row_text(&mut table["head"]);
        row_text(&mut table["body"]);
        data["page_masters"]["masters"][0]["body"] = json!({
            "x":500_000,"y":500_000,"width":10_000_000,"height":3 * 16 * 65536
        });
        let input = prepared(&root, data, b"ResultProof", &limits);
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                let registry = pdf.navigation().source().source();
                let marked = registry.source();
                assert_eq!(marked.pages().len(), expected_pages);
                assert_eq!(observation.candidate_passes(), 1);
                let roles = registry
                    .nodes()
                    .iter()
                    .map(|node| node.source().pdf_role())
                    .collect::<Vec<_>>();
                assert_eq!(roles.iter().filter(|&&r| r == "Caption").count(), 1);
                assert_eq!(roles.iter().filter(|&&r| r == "Table").count(), 1);
                assert_eq!(roles.iter().filter(|&&r| r == "TD").count(), 8);
                assert_eq!(
                    roles.iter().filter(|&&r| r == "TH").count(),
                    if headers { 2 } else { 0 }
                );
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap();
    }
}
