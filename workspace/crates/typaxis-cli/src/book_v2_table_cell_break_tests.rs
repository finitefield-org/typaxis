use super::table_caption_breaks::{caption_breaks_with_text, renumber};
use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

pub(super) fn cells(
    text: &str,
    split: usize,
    simultaneous: bool,
    caption: bool,
    empty: bool,
) -> Value {
    let mut data = caption_breaks_with_text(false, false, text);
    let table = &mut data["document"]["blocks"][0];
    let mut left = table["caption"][1].clone();
    left["children"][0]["text_span"]["end_byte"] = split.into();
    let mut right = table["caption"][1].clone();
    right["children"][0]["text_span"]["start_byte"] = split.into();
    if !simultaneous {
        right["classes"] = json!(["tall"]);
    }
    let br = table["caption"][0].clone();
    let mut cell = table["body"][0]["cells"][0].clone();
    cell["blocks"] = if empty {
        json!([br.clone(), br.clone()])
    } else {
        json!([left.clone(), br.clone(), left])
    };
    let mut second = cell.clone();
    second["blocks"] = if empty {
        json!([])
    } else if simultaneous {
        json!([right.clone(), br, right])
    } else {
        json!([right.clone(), right])
    };
    table["columns"] = json!([{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}]);
    table["body"][0]["cells"] = json!([cell, second]);
    table["body"].as_array_mut().unwrap().truncate(1);
    if !caption {
        table.as_object_mut().unwrap().remove("caption");
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"tall","selector":"paragraph.tall","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"line_height","important":false,"value":{"kind":"length","value":32*65536}}]}));
    renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_table_cell_breaks_preserve_parallel_source_and_blank_pages() {
    for (simultaneous, caption, empty, expected) in [
        (false, false, false, 3),
        (true, false, false, 2),
        (false, true, false, 7),
        (false, false, true, 3),
    ] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(
            &root,
            cells("LeftRight", 4, simultaneous, caption, empty),
            b"LeftRight",
            &limits,
        );
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                assert_eq!(
                    pdf.navigation().source().source().source().pages().len(),
                    expected,
                    "{simultaneous}/{caption}/{empty}"
                );
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap();
    }
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_cell_breaks_render_original_harano() {
    let root = Root::new();
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
    let mut data = cells(text, 6, false, false, false);
    data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
    data["resources"]["font_faces"][0]["expected_sha256"] = hash.into();
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
            assert_eq!(pdf.navigation().source().source().source().pages().len(), 3);
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_table_cell_breaks_repeat_headers_keep_caption_prefix_and_notes() {
    for mode in [
        "header",
        "caption",
        "caption-keep",
        "caption-keep-long",
        "notes",
    ] {
        let root = Root::new();
        let limits = limits();
        let mut data = cells("LeftRight", 4, false, false, false);
        if mode == "header" {
            let mut header = data["document"]["blocks"][0]["body"][0].clone();
            for cell in header["cells"].as_array_mut().unwrap() {
                cell["blocks"].as_array_mut().unwrap().truncate(1);
            }
            data["document"]["blocks"][0]["head"] = json!([header]);
            data["page_masters"]["masters"][0]["body"]["height"] = (64 * 65536).into();
        } else if mode.starts_with("caption") {
            let para = caption_breaks_with_text(false, false, "LeftRight")["document"]["blocks"][0]
                ["caption"][1]
                .clone();
            data["document"]["blocks"][0]["caption"] = json!([para]);
            if mode.starts_with("caption-keep") {
                data["document"]["blocks"][0]["caption"][0]["classes"] = json!(["caption-keep"]);
                let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
                rules.push(json!({"style_id":"caption-keep","selector":"paragraph.caption-keep","source_order":rules.len(),"extends":null,
                    "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
            }
        } else {
            let para = data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0].clone();
            data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0]["children"].as_array_mut().unwrap()
                .push(json!({"kind":"footnote_reference","node_id":0,"span":para["span"],"footnote_id":"note"}));
            data["document"]["footnotes"] = json!([{"node_id":0,"span":para["span"],"footnote_id":"note","blocks":vec![para;8]}]);
            data["page_masters"]["masters"][0]["footnote"] =
                json!({"x":10*65536,"y":80*65536,"width":180*65536,"height":48*65536});
        }
        if mode == "caption-keep-long" {
            let kept = data["document"]["blocks"][0]["caption"][0].clone();
            let mut loose = kept.clone();
            loose["classes"] = json!([]);
            data["document"]["blocks"][0]["caption"] = json!([loose.clone(), loose, kept]);
        }
        renumber(&mut data["document"], &mut 0);
        let input = prepared(&root, data, b"LeftRight", &limits);
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                let pages = pdf.navigation().source().source().source().pages().len();
                if mode == "notes" {
                    assert!(pages >= 3);
                } else {
                    assert_eq!(
                        pages,
                        if mode == "caption-keep-long" { 4 } else { 3 },
                        "{mode}"
                    );
                }
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_table_cell_breaks_bind_cursors_and_exact_shared_budgets() {
    use typaxis_core::{Length, PositiveLength, Rect};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
        prepare_book_v2_table_measurements, prepare_book_v2_table_search, BookV2BodyCandidatePart,
    };
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    let root = Root::new();
    let limits = limits();
    let input = prepared(
        &root,
        cells("LeftRight", 4, false, false, true),
        b"LeftRight",
        &limits,
    );
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings =
        typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let raw = |v| Length::from_raw(v).unwrap();
    let rect = Rect::new(
        raw(10 * 65536),
        raw(10 * 65536),
        PositiveLength::new(raw(180 * 65536)).unwrap(),
        PositiveLength::new(raw(48 * 65536)).unwrap(),
    );
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,rect,1_000_000,|stable| {
        let measured=prepare_book_v2_table_measurements(prepare_book_v2_body_flow(stable.lines(),None,stable.footnotes(),&limits,0).unwrap(),&limits).unwrap();
        let replay=|prior,work|->Result<_,typaxis_pagination::ProductionBodyPaginationError>{
            let mut search=prepare_book_v2_table_search(&measured,0,&limits,work,prior)?;
            let mut cursor=search.begin()?;let mut hashes=Vec::new();let mut leaves=Vec::new();
            while !cursor.is_terminal() {
                assert!(hashes.len()<10);
                let selected=search.evaluate(&cursor,search.maximum_height())?.unwrap();
                leaves.extend(selected.semantic_leaf_ranges().flatten());hashes.push(selected.fingerprint());cursor=selected.after();
            }
            assert_eq!(leaves,[0,1]); assert_eq!(hashes.len(),3);
            Ok((search.record_charge(),search.work_charge(),hashes))
        };
        let (records,work,hashes)=replay(0,1_000_000).unwrap();
        let prior=limits.base().get().max_fragments-(records-measured.record_charge());
        assert_eq!(replay(prior,work).unwrap(),(limits.base().get().max_fragments,work,hashes));
        assert_eq!(replay(prior+1,work).unwrap_err().kind,E::FragmentLimit);
        assert_eq!(replay(0,work-1).unwrap_err().kind,E::TableSearchLimit);
        let mut search=prepare_book_v2_table_body_search(&measured,&limits,1_000_000,0).unwrap();
        let pages=search.select_stable_mixed_pages(2).unwrap();
        let placed=search.place_mixed_pages(pages.sequence()).unwrap();
        search.close_mixed_page_sources(&pages,&placed).unwrap();
        let stale=pages.sequence().pages()[0].next_state().source_state().table_continuation().unwrap();
        let state=pages.sequence().pages()[1].next_state().source_state();
        let current=state.table_continuation().unwrap();
        assert_eq!(stale.offset(),current.offset()); assert_eq!(stale.next_row(),current.next_row());
        assert_ne!(stale.cell_progress_fingerprint(),current.cell_progress_fingerprint());
        assert!(matches!(search.evaluate_mixed_candidate(state,&[BookV2BodyCandidatePart::Table{cursor:stale,capacity:raw(48*65536)}]),Err(e) if e.kind==E::ReceiptMismatch));
    }).unwrap();
}

#[test]
fn book_v2_table_cell_breaks_reject_keep_conflicts() {
    let root = Root::new();
    let limits = limits();
    let mut data = cells("LeftRight", 4, false, false, false);
    data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0]["classes"] = json!(["keep"]);
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"keep","selector":"paragraph.keep","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
    renumber(&mut data["document"], &mut 0);
    let owner = data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0]["node_id"]
        .as_u64()
        .unwrap();
    let input = prepared(&root, data, b"LeftRight", &limits);
    let error = crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |_, _| panic!("unsupported source reached PDF"),
    )
    .unwrap_err();
    let message = format!("{error:?}");
    assert!(message.contains("KeepAcrossForcedBreak"), "{message}");
    assert!(message.contains(&format!("NodeId({owner})")), "{message}");
}

#[test]
fn book_v2_table_cell_breaks_continue_into_following_rows_and_colspans() {
    for colspan in [false, true] {
        let root = Root::new();
        let limits = limits();
        let mut data = cells("LeftRight", 4, false, false, false);
        let mut row = data["document"]["blocks"][0]["body"][0].clone();
        for cell in row["cells"].as_array_mut().unwrap() {
            cell["blocks"].as_array_mut().unwrap().truncate(1);
        }
        if colspan {
            row["cells"].as_array_mut().unwrap().truncate(1);
            row["cells"][0]["colspan"] = 2.into();
        }
        data["document"]["blocks"][0]["body"]
            .as_array_mut()
            .unwrap()
            .push(row);
        renumber(&mut data["document"], &mut 0);
        let input = prepared(&root, data, b"LeftRight", &limits);
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                assert_eq!(
                    pdf.navigation().source().source().source().pages().len(),
                    if colspan { 3 } else { 4 }
                );
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap();
    }
}
