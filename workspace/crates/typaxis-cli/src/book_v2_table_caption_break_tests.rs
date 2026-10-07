use super::*;
use typaxis_core::{Length, PositiveLength, Rect};
use typaxis_linebreak::JapaneseLineBreakMode;
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
    prepare_book_v2_table_measurements, prepare_book_v2_table_search, BookV2BodyCandidatePart,
};
use typaxis_pagination::ProductionBodyPaginationErrorKind as Error;

pub(super) fn renumber(v: &mut Value, next: &mut u32) {
    if let Some(a) = v.as_array_mut() {
        for c in a {
            renumber(c, next)
        }
    } else if let Some(o) = v.as_object_mut() {
        if let Some(id) = o.get_mut("node_id") {
            *id = (*next).into();
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
            if let Some(c) = o.get_mut(key) {
                renumber(c, next)
            }
        }
    }
}
fn caption_breaks(headers: bool, only_breaks: bool) -> Value {
    caption_breaks_with_text(headers, only_breaks, "Result")
}
pub(super) fn caption_breaks_with_text(headers: bool, only_breaks: bool, text: &str) -> Value {
    let mut data = source_data(text);
    let para = data["document"]["blocks"][0]["blocks"][0].clone();
    let span = para["span"].clone();
    let br = json!({"kind":"page_break","node_id":0,"span":span,"classes":[]});
    let caption = if only_breaks {
        vec![br.clone(), br.clone()]
    } else {
        vec![
            br.clone(),
            para.clone(),
            br.clone(),
            br.clone(),
            para.clone(),
            br.clone(),
        ]
    };
    let cell = json!({"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":if only_breaks{vec![]}else{vec![para.clone()]}});
    let row = json!({"node_id":0,"span":span,"cells":[cell]});
    data["document"]["blocks"] = json!([{"kind":"table","node_id":0,"span":span,"classes":[],
        "columns":[{"kind":"fraction","weight":1}],"caption":caption,
        "head":if headers{vec![row.clone()]}else{vec![]},"body":if only_breaks{vec![row.clone()]}else{vec![row.clone();3]}}]);
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = (200 * 65536).into();
    master["height"] = (150 * 65536).into();
    master["trim"] = json!({"x":0,"y":0,"width":200*65536,"height":150*65536});
    master["body"] = json!({"x":10*65536,"y":10*65536,"width":180*65536,"height":48*65536});
    renumber(&mut data["document"], &mut 0);
    data
}

#[test]
fn book_v2_table_caption_forced_breaks_keep_leading_consecutive_trailing_pdf_pages() {
    for (headers, only_breaks, pages) in [(false, false, 5), (true, false, 6), (false, true, 3)] {
        let root = Root::new();
        let limits = limits();
        let data = caption_breaks(headers, only_breaks);
        let input = prepared(&root, data, b"Result", &limits);
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                assert_eq!(
                    pdf.navigation().source().source().source().pages().len(),
                    pages
                );
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_table_caption_forced_cursor_counts_semantics_and_exact_budgets() {
    let root = Root::new();
    let limits = limits();
    let data = caption_breaks(true, false);
    let expected_breaks = data["document"]["blocks"][0]["caption"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["kind"] == "page_break")
        .map(|b| b["node_id"].as_u64().unwrap() as u32)
        .collect::<Vec<_>>();
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings =
        typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let raw = |v| Length::from_raw(v).unwrap();
    let body = Rect::new(
        raw(10 * 65536),
        raw(10 * 65536),
        PositiveLength::new(raw(180 * 65536)).unwrap(),
        PositiveLength::new(raw(48 * 65536)).unwrap(),
    );
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,body,1_000_000,|stable|{
        let measured=prepare_book_v2_table_measurements(prepare_book_v2_body_flow(stable.lines(),None,stable.footnotes(),&limits,0).unwrap(),&limits).unwrap();
        let replay=|prior,work|->Result<_,typaxis_pagination::ProductionBodyPaginationError>{
            let mut search=prepare_book_v2_table_search(&measured,0,&limits,work,prior)?;
            let mut cursor=search.begin()?;let mut breaks=Vec::new();let mut semantics=Vec::new();let mut hashes=Vec::new();
            while !cursor.is_terminal(){
                assert!(hashes.len()<20);
                let part=search.evaluate(&cursor,search.maximum_height())?.unwrap();
                if let Some(owner)=part.forced_break_owner(){
                    breaks.push(owner.get());assert!(!part.after().has_started_rows());assert!(part.cells().is_empty());
                    assert!(part.after().next_caption_item()>cursor.next_caption_item());
                }
                semantics.extend(part.semantic_leaf_ranges().flatten());hashes.push(part.fingerprint());cursor=part.after();
            }
            assert_eq!(breaks,expected_breaks);assert_eq!(semantics,(0..10).collect::<Vec<_>>());
            Ok((search.record_charge(),search.work_charge(),hashes))
        };
        let (records,work,hashes)=replay(0,1_000_000).unwrap();
        let prior=limits.base().get().max_fragments-(records-measured.record_charge());
        assert_eq!(replay(prior,work).unwrap(),(limits.base().get().max_fragments,work,hashes));
        assert_eq!(replay(prior+1,work).unwrap_err().kind,Error::FragmentLimit);
        assert_eq!(replay(0,work-1).unwrap_err().kind,Error::TableSearchLimit);
        let mut search=prepare_book_v2_table_body_search(&measured,&limits,1_000_000,0).unwrap();
        let pages=search.select_stable_mixed_pages(2).unwrap();
        assert_eq!(pages.sequence().pages().iter().filter_map(|p|p.forced_break().map(|o|o.get())).collect::<Vec<_>>(),expected_breaks);
        let placed=search.place_mixed_pages(pages.sequence()).unwrap();
        assert_eq!(placed.pages().iter().map(|p|p.fragments().len()).collect::<Vec<_>>(),[0,1,0,1,3,2]);
        search.close_mixed_page_sources(&pages,&placed).unwrap();
        // The two consecutive source breaks share a height, but are distinct cursors.
        let first=&pages.sequence().pages()[1];let second=&pages.sequence().pages()[2];
        let stale=first.next_state().source_state().table_continuation().unwrap();
        let state=second.next_state().source_state();
        assert_eq!(stale.offset(),state.table_continuation().unwrap().offset());
        assert_eq!(stale.next_row(),state.table_continuation().unwrap().next_row());
        assert!(matches!(search.evaluate_mixed_candidate(state,&[BookV2BodyCandidatePart::Table{cursor:stale,capacity:raw(48*65536)}]),Err(e) if e.kind==Error::ReceiptMismatch));
    }).unwrap();
}

#[test]
fn book_v2_table_caption_forced_breaks_preserve_pending_footnotes() {
    let root = Root::new();
    let limits = limits();
    let mut data = caption_breaks(true, false);
    let paragraph = data["document"]["blocks"][0]["caption"][1].clone();
    let span = paragraph["span"].clone();
    data["document"]["blocks"][0]["caption"][1]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({
        "kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"note"}));
    data["document"]["footnotes"] =
        json!([{"node_id":0,"span":span,"footnote_id":"note","blocks":vec![paragraph;8]}]);
    data["page_masters"]["masters"][0]["footnote"] =
        json!({"x":10*65536,"y":80*65536,"width":180*65536,"height":48*65536});
    renumber(&mut data["document"], &mut 0);
    let input = prepared(&root, data, b"Result", &limits);
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            let registry = pdf.navigation().source().source();
            assert!(registry.source().pages().len() >= 6);
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
                1
            );
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_table_caption_forced_breaks_reject_keep_conflicts() {
    for region in ["keep", "table_keep"] {
        let root = Root::new();
        let limits = limits();
        let mut data = caption_breaks(true, false);
        let owner;
        if region == "keep" {
            data["document"]["blocks"][0]["caption"][1]["classes"] = json!(["keep"]);
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            rules.push(json!({"style_id":"caption-keep","selector":"paragraph.keep","source_order":rules.len(),"extends":null,
                "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
            owner = data["document"]["blocks"][0]["caption"][1]["node_id"]
                .as_u64()
                .unwrap();
        } else {
            // A figure's keep_caption holds its enclosed table together.
            let table = data["document"]["blocks"][0].clone();
            data["document"]["blocks"] = json!([{"kind":"figure","node_id":0,"span":table["span"],"classes":[],
                "image_id":0,"placement":"block","alt":"Original diagram","caption":[table]}]);
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            rules.push(json!({"style_id":"figure-keep","selector":"figure","source_order":rules.len(),"extends":null,
                "declarations":[{"name":"keep_caption","important":false,"value":{"kind":"boolean","value":true}},
                    {"name":"width","important":false,"value":{"kind":"length","value":32*65536}}]}));
            renumber(&mut data["document"], &mut 0);
            owner = data["document"]["blocks"][0]["caption"][0]["node_id"]
                .as_u64()
                .unwrap();
        }
        let input = prepared(&root, data, b"Result", &limits);
        let result = crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |_, _| panic!("unsupported source must not reach PDF callback"),
        );
        let error = result.unwrap_err();
        let text = format!("{error:?}");
        assert!(text.contains("KeepAcrossForcedBreak"), "{text}");
        assert!(text.contains(&format!("NodeId({owner})")), "{text}");
    }
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_caption_forced_breaks_render_original_harano() {
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
    let text = "表の説明";
    let mut data = caption_breaks_with_text(true, false, text);
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
            // Original font metrics allow one body row beside each repeated header.
            assert_eq!(pdf.navigation().source().source().source().pages().len(), 7);
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}
