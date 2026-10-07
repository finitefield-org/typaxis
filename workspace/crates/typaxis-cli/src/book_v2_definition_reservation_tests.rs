use super::*;
use typaxis_core::{Length, PositiveLength, Rect};
use typaxis_linebreak::JapaneseLineBreakMode;
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_mixed_footnote_demand_search,
    prepare_book_v2_table_body_search, prepare_book_v2_table_measurements, BookV2TableMeasurements,
};
use typaxis_pagination::{
    ProductionBodyPaginationError, ProductionBodyPaginationErrorKind as E,
    ProductionFootnoteDemandStatus as Status,
};

pub(super) fn with_tables(
    mut data: Value,
    mut check: impl FnMut(&BookV2TableMeasurements<'_, '_, '_, '_>, &M4EffectiveResourceLimits),
) {
    let rect = &data["page_masters"]["masters"][0]["body"];
    let n = |key: &str| Length::from_raw(rect[key].as_i64().unwrap()).unwrap();
    let body = Rect::new(
        n("x"),
        n("y"),
        PositiveLength::new(n("width")).unwrap(),
        PositiveLength::new(n("height")).unwrap(),
    );
    super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, data, b"LeftRight", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
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
            let measured = prepare_book_v2_table_measurements(
                prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0)
                    .unwrap(),
                &limits,
            )
            .unwrap();
            check(&measured, &limits);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_definition_reservations_backtrack_tables_and_close_retained_dependencies() {
    for mode in [
        "late",
        "early",
        "repeat",
        "cycle",
        "two-body",
        "forced-child",
    ] {
        let mut data = super::nested_tables::nested(
            "LeftRight",
            4,
            if mode == "repeat" {
                "headers"
            } else {
                "natural"
            },
        );
        let mut table = data["document"]["blocks"][0].clone();
        let plain = super::nested_tables::nested("LeftRight", 4, "natural")["document"]["blocks"]
            [0]["body"][0]["cells"][1]["blocks"][0]
            .clone();
        let reference = |id: &str| json!({"kind":"footnote_reference","node_id":0,"span":plain["span"],"footnote_id":id});
        let mut body = plain.clone();
        body["children"]
            .as_array_mut()
            .unwrap()
            .push(reference("a"));
        if mode == "two-body" {
            body["children"]
                .as_array_mut()
                .unwrap()
                .push(reference("b"));
        }
        let mut tail = plain.clone();
        tail["children"]
            .as_array_mut()
            .unwrap()
            .push(reference("b"));
        if mode != "late" {
            let child = &mut table["body"][0]["cells"][0]["blocks"][0];
            let first = if mode == "repeat" {
                &mut child["head"][0]["cells"][0]["blocks"][0]
            } else {
                &mut child["body"][0]["cells"][0]["blocks"][0]
            };
            first["children"]
                .as_array_mut()
                .unwrap()
                .push(reference("b"));
        }
        let mut child = plain.clone();
        if mode == "cycle" {
            child["children"]
                .as_array_mut()
                .unwrap()
                .push(reference("a"));
        }
        let child_blocks = if mode == "forced-child" {
            let br = super::table_cell_breaks::cells("LeftRight", 4, false, false, false)
                ["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][1]
                .clone();
            json!([br, child])
        } else {
            json!([child])
        };
        data["document"]["blocks"] = json!([body]);
        data["document"]["footnotes"] = json!([
            {"node_id":0,"span":plain["span"],"footnote_id":"a","blocks":if mode=="late"{json!([table,tail])}else{json!([table])}},
            {"node_id":0,"span":plain["span"],"footnote_id":"b","blocks":child_blocks}
        ]);
        let master = &mut data["page_masters"]["masters"][0];
        master["height"] = (320 * 65536).into();
        master["trim"]["height"] = (320 * 65536).into();
        master["footnote"] = json!({"x":master["body"]["x"],"y":150*65536,"width":master["body"]["width"],"height":128*65536});
        let mut capacity = Length::ZERO;
        with_tables(data.clone(), |measured, limits| {
            let mut search =
                prepare_book_v2_mixed_footnote_demand_search(measured, limits, 1_000_000, 0)
                    .unwrap();
            let initial = search.begin().unwrap();
            let state = search
                .require_body(&initial, 0..measured.flow().body_items().len())
                .unwrap();
            let full = search
                .select_definition(&state, 0, Length::from_raw(128 * 65536).unwrap())
                .unwrap()
                .unwrap();
            assert!(full.next_state().is_complete());
            capacity = full.used_height();
        });
        data["page_masters"]["masters"][0]["footnote"]["height"] =
            (capacity.raw() + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW).into();
        with_tables(data.clone(), |measured, limits| {
            let run = |prior, work| -> Result<_, ProductionBodyPaginationError> {
                let mut search =
                    prepare_book_v2_mixed_footnote_demand_search(measured, limits, work, prior)?;
                let initial = search.begin()?;
                let demanded =
                    search.require_body(&initial, 0..measured.flow().body_items().len())?;
                let greedy = search.select_required_region(&demanded, capacity)?.unwrap();
                if mode == "two-body" {
                    assert_eq!(greedy.fragments().len(), 2);
                    assert_eq!(greedy.next_state().status(0), Some(Status::Pending));
                    assert_eq!(greedy.next_state().status(1), Some(Status::Complete));
                } else {
                    assert_eq!(greedy.fragments().len(), 1);
                    assert_eq!(greedy.next_state().status(0), Some(Status::Complete));
                    assert!(!greedy.next_state().definition_started(1));
                }
                let before = search.work_steps();
                let fit = search
                    .evaluate_body_candidate(&initial, 0..measured.flow().body_items().len())?;
                assert!(search.work_steps() > before);
                assert_eq!(initial.status(0), Some(Status::Unreferenced));
                assert_eq!(initial.status(1), Some(Status::Unreferenced));
                if mode == "forced-child" {
                    assert!(
                        fit.is_none(),
                        "a forced-only child cannot satisfy its retained reference"
                    );
                    return Ok((search.record_charge(), search.work_steps(), Vec::new()));
                }
                let mut fit = fit.expect(mode);
                fit.verify(&initial)?;
                assert!(fit.footnotes().unwrap().fragments()[0]
                    .fragment()
                    .marker()
                    .is_some());
                assert_eq!(fit.next_state().status(0), Some(Status::Pending));
                if mode == "late" {
                    assert_eq!(fit.next_state().status(1), Some(Status::Unreferenced));
                    assert_eq!(fit.next_state().first_reference(1), None);
                } else {
                    assert_eq!(fit.next_state().status(1), Some(Status::Complete));
                    assert!(fit.footnotes().unwrap().fragments()[0]
                        .fragment()
                        .continuation()
                        .unwrap()
                        .table_continuation()
                        .is_some());
                }
                let mut seen = [Vec::new(), Vec::new()];
                let mut markers = [0, 0];
                let mut outcomes = Vec::new();
                loop {
                    assert!(outcomes.len() < 10);
                    let region = fit.footnotes().unwrap();
                    assert!(region.used_height() <= capacity);
                    assert_eq!(
                        fit.footnote_bounds().unwrap().height().get().raw(),
                        region.used_height().raw() + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW
                    );
                    for placed in region.fragments() {
                        let fragment = placed.fragment();
                        let definition = fragment.definition_index();
                        markers[definition] += usize::from(fragment.marker().is_some());
                        for reference in fragment.references() {
                            assert!(fit
                                .next_state()
                                .definition_started(reference.source().definition_index()));
                        }
                        for part in fragment.mixed().unwrap().parts() {
                            if let Some(range) = part.items() {
                                seen[definition].extend(range);
                            }
                            if let Some(table) = part.table() {
                                seen[definition].extend(
                                    table
                                        .source_leaf_ranges()
                                        .collect::<Result<Vec<_>, _>>()?
                                        .into_iter()
                                        .flatten(),
                                );
                            }
                        }
                    }
                    outcomes.push((region.used_height(), region.fragments().len()));
                    if fit.next_state().pending_definitions().is_empty() {
                        break;
                    }
                    fit = search
                        .evaluate_body_candidate(fit.next_state(), 0..0)?
                        .unwrap();
                }
                for definition in 0..2 {
                    assert_eq!(markers[definition], 1);
                    seen[definition].sort_unstable();
                    assert_eq!(
                        seen[definition],
                        (0..measured.flow().definition_items(definition).unwrap().len())
                            .collect::<Vec<_>>(),
                        "source once {mode}"
                    );
                }
                Ok((search.record_charge(), search.work_steps(), outcomes))
            };
            let physical = |prior, work| -> Result<_, ProductionBodyPaginationError> {
                let mut pages = prepare_book_v2_table_body_search(measured, limits, work, prior)?;
                assert_eq!(pages.body_table_count(), 0);
                let selected =
                    pages.select_stable_mixed_pages(limits.base().get().max_layout_passes);
                if mode == "forced-child" {
                    match selected {
                        Err(e) if e.kind == E::JointPageNoFit => (),
                        Err(e) => return Err(e),
                        Ok(_) => panic!("invalid dependency reached pages"),
                    }
                    return Ok((pages.record_charge(), pages.work_steps(), 0));
                }
                let stable = selected?;
                let placed = pages.place_mixed_pages(stable.sequence())?;
                let closure = pages.close_mixed_page_sources(&stable, &placed)?;
                assert_eq!(closure.unreferenced_definitions(), 0);
                assert!(placed.pages().len() >= 2);
                let mut labels = [0, 0];
                for page in placed.pages() {
                    for marker in page.footnote_markers() {
                        labels[marker.definition_index()] += 1;
                    }
                }
                assert_eq!(labels, [1, 1]);
                Ok((
                    pages.record_charge(),
                    pages.work_steps(),
                    placed.pages().len(),
                ))
            };
            let (records, work, count) = physical(0, 1_000_000).unwrap();
            let prior = limits.base().get().max_fragments - (records - measured.record_charge());
            assert_eq!(
                physical(prior, work).unwrap(),
                (limits.base().get().max_fragments, work, count)
            );
            assert_eq!(
                physical(prior + 1, work).unwrap_err().kind,
                E::FragmentLimit
            );
            assert!(matches!(
                physical(prior, work - 1).unwrap_err().kind,
                E::TableSearchLimit | E::FootnoteSearchLimit
            ));
            let (records, work, outcomes) = run(0, 1_000_000).unwrap();
            let prior = limits.base().get().max_fragments - (records - measured.record_charge());
            assert_eq!(
                run(prior, work).unwrap(),
                (limits.base().get().max_fragments, work, outcomes)
            );
            assert_eq!(run(prior + 1, work).unwrap_err().kind, E::FragmentLimit);
            assert!(matches!(
                run(prior, work - 1).unwrap_err().kind,
                E::TableSearchLimit | E::FootnoteSearchLimit
            ));
        });
        if mode != "forced-child" {
            super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
            let root = Root::new();
            let limits = limits();
            let input = prepared(&root, data, b"LeftRight", &limits);
            crate::book_v2_resources::with_converged_book_v2_pdf(
                &input,
                &limits,
                JapaneseLineBreakMode::Normal,
                100_000_000,
                |pdf, observation| {
                    assert!(pdf.navigation().source().source().source().pages().len() >= 2);
                    crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
                },
            )
            .unwrap_or_else(|e| panic!("definition driver {mode}: {e:?}"));
        }
    }
}

fn combined_tables(text: &str, split: usize) -> Value {
    let mut data = super::nested_tables::nested(text, split, "natural");
    let note =
        super::nested_tables::nested(text, split, "headers")["document"]["blocks"][0].clone();
    let paragraph = &mut data["document"]["blocks"][0]["body"][0]["cells"][1]["blocks"][0];
    let reference = json!({"kind":"footnote_reference","node_id":0,"span":paragraph["span"],"footnote_id":"note"});
    paragraph["children"]
        .as_array_mut()
        .unwrap()
        .push(reference);
    data["document"]["footnotes"] =
        json!([{"node_id":0,"span":note["span"],"footnote_id":"note","blocks":[note]}]);
    let master = &mut data["page_masters"]["masters"][0];
    master["height"] = (320 * 65536).into();
    master["trim"]["height"] = (320 * 65536).into();
    master["footnote"] = json!({"x":master["body"]["x"],"y":150*65536,"width":master["body"]["width"],"height":48*65536+typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW});
    super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_definition_tables_share_pages_with_original_body_tables() {
    let data = combined_tables("LeftRight", 4);
    with_tables(data.clone(), |measured, limits| {
        let run = |prior, work| -> Result<_, ProductionBodyPaginationError> {
            let mut search = prepare_book_v2_table_body_search(measured, limits, work, prior)?;
            assert_eq!(search.body_table_count(), 2);
            assert_eq!(measured.flow().table_count(), 4);
            assert!(search.body_table_range(2).is_none());
            assert!(matches!(search.begin_table(2),Err(e) if e.kind==E::ReceiptMismatch));
            let stable = search.select_stable_mixed_pages(limits.base().get().max_layout_passes)?;
            let placed = search.place_mixed_pages(stable.sequence())?;
            let closed = search.close_mixed_page_sources(&stable, &placed)?;
            assert_eq!(closed.unreferenced_definitions(), 0);
            let labels: usize = placed
                .pages()
                .iter()
                .map(|page| page.footnote_markers().len())
                .sum();
            assert_eq!(labels, 1);
            assert!(placed
                .pages()
                .iter()
                .flat_map(|page| page.fragments_with_roles())
                .any(|(fragment, _, repeat)| repeat && fragment.definition_index() == Some(0)));
            assert!(stable
                .sequence()
                .pages()
                .last()
                .unwrap()
                .next_state()
                .is_complete());
            Ok((
                search.record_charge(),
                search.work_steps(),
                placed.pages().len(),
            ))
        };
        let (records, work, count) = run(0, 1_000_000).unwrap();
        let prior = limits.base().get().max_fragments - (records - measured.record_charge());
        assert_eq!(
            run(prior, work).unwrap(),
            (limits.base().get().max_fragments, work, count)
        );
        assert_eq!(run(prior + 1, work).unwrap_err().kind, E::FragmentLimit);
        assert!(matches!(
            run(prior, work - 1).unwrap_err().kind,
            E::TableSearchLimit | E::FootnoteSearchLimit
        ));
    });
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, data, b"LeftRight", &limits);
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_definition_tables_render_original_harano() {
    let limits = limits();
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hash = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hash,
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let text = "左側右側";
    // Keep the preceding 80 pt fixture and prove the originally failing 48 pt
    // geometry with the same unmodified font and unequal cell line heights.
    for body_height in [80, 48] {
        let root = Root::new();
        let mut data = combined_tables(text, 6);
        data["page_masters"]["masters"][0]["body"]["height"] = (body_height * 65536).into();
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = hash.clone().into();
        let body = body_with_source(&root, data, text.as_bytes(), &limits);
        fs::write(root.0.join("body.bin"), &bytes).unwrap();
        let input = prepare_book_v2_resources(
            body,
            &root.context(),
            &config(limits.base().get().clone()),
            &limits,
        )
        .unwrap();
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                assert!(pdf.navigation().source().source().source().pages().len() >= 2);
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap();
    }
}
