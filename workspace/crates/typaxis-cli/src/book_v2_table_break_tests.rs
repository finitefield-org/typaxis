use super::*;
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_table_measurements, prepare_book_v2_table_search,
};
use typaxis_pagination::{
    ProductionBodyPaginationError, ProductionBodyPaginationErrorKind as Error,
};

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
        "caption",
        "footnotes",
    ] {
        if let Some(children) = v.get_mut(key).and_then(Value::as_array_mut) {
            for child in children {
                renumber(child, next);
            }
        }
    }
}
fn table_data() -> Value {
    let mut data = framed_data();
    let span = data["document"]["blocks"][0]["span"].clone();
    let paragraph =
        data["document"]["blocks"][0]["blocks"][0]["body"][0]["cells"][0]["blocks"][0].clone();
    let cell = |rowspan, count| json!({"node_id":0,"span":span,"colspan":1,"rowspan":rowspan,"blocks":vec![paragraph.clone();count]});
    let row = |cells| json!({"node_id":0,"span":span,"cells":cells});
    let table = &mut data["document"]["blocks"][0]["blocks"][0];
    table["columns"][0]["width"] = 6_000_000.into();
    table["head"] = json!([row(vec![cell(1, 1), cell(1, 1)])]);
    table["body"] = json!([
        row(vec![cell(3, 3), cell(1, 1)]),
        row(vec![cell(1, 1)]),
        row(vec![cell(1, 1)])
    ]);
    renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_table_fragments_reserve_headers_keep_rowspan_content_and_exact_budgets() {
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, table_data(), b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings =
        typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(&policy, &flow, input.resources(), &bindings, &limits, JapaneseLineBreakMode::Normal,
        rect(500_000,500_000,10_000_000,20_000_000),1_000_000, |stable| {
        let make = || prepare_book_v2_table_measurements(prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0).unwrap(), &limits).unwrap();
        let measured = make();
        let table = &measured.tables()[0];
        assert_eq!(table.rows().len(), 4);
        let header = table.rows()[0].height();
        let capacity = header.checked_add(table.rows()[1].height()).unwrap();
        let replay = |prior, work| -> Result<(u64,u64,Vec<[u8;32]>), ProductionBodyPaginationError> {
            let mut search = prepare_book_v2_table_search(&measured,0,&limits,work,prior)?;
            assert_eq!(search.header_height(), header);
            let mut cursor = search.begin()?;
            assert!(cursor.is_initial());
            assert_eq!(cursor.offset(), header);
            assert_eq!(cursor.next_row(),1);
            assert!(search.evaluate(&cursor, header.checked_sub(Length::from_raw(1).unwrap()).unwrap())?.is_none());
            let mut fingerprints = Vec::new();
            let mut selected = vec![Vec::new(); table.cells().len()];
            let mut semantic_items = Vec::new();
            while !cursor.is_terminal() {
                assert!(fingerprints.len() < 32);
                let fragment = search.evaluate(&cursor,capacity)?.expect("capacity holds an actual cell line plus the complete header");
                assert_eq!(fragment.before().offset(),cursor.offset());
                assert_eq!(fragment.repeats_header(), !fingerprints.is_empty());
                assert_eq!(fragment.header_height(),header);
                assert!(fragment.used_height() <= capacity);
                assert!(fragment.after().offset() > cursor.offset());
                assert!(fragment.after().next_row() >= cursor.next_row());
                for slice in fragment.cells() {
                    assert!(slice.cell_index() >= 2, "header cells are reserved separately");
                    assert!(slice.top() >= header);
                    assert!(slice.offset_after() >= slice.offset_before());
                    selected[slice.cell_index()].extend(slice.content_range());
                }
                semantic_items.extend(fragment.semantic_leaf_ranges().flatten());
                let paint = fragment.placement_leaves().collect::<Result<Vec<_>,_>>()?;
                assert!(paint.iter().all(|(_,index,top,_)| measured.item(*index).is_some() && *top >= Length::ZERO));
                assert_eq!(paint.iter().filter(|(_,_,_,repeated)| *repeated).count(), if fingerprints.is_empty() {0} else {2});
                fingerprints.push(fragment.fingerprint());
                cursor = fragment.after();
            }
            let expected_items = table.cells().iter().flat_map(|c| c.content()).map(|c| match c.source() {
                typaxis_pagination::ProductionTableContentSource::FlowItem(i) => i,
                _ => panic!("fixture has no nested table"),
            }).collect::<std::collections::BTreeSet<_>>();
            assert_eq!(semantic_items.len(), expected_items.len());
            assert_eq!(semantic_items.into_iter().collect::<std::collections::BTreeSet<_>>(), expected_items);
            assert_eq!(cursor.offset(),table.height());
            assert_eq!(cursor.next_row(),table.rows().len());
            assert!(fingerprints.len() > 2);
            for (index, cell) in table.cells().iter().enumerate().skip(2) {
                assert_eq!(selected[index], (0..cell.content().len()).collect::<Vec<_>>());
            }
            assert!(matches!(search.evaluate(&cursor,capacity), Err(e) if e.kind == Error::ReceiptMismatch));
            Ok((search.record_charge(),search.work_charge(),fingerprints))
        };
        let (records, work, fingerprints) = replay(0,1_000_000).unwrap();
        let exact_prior = limits.base().get().max_fragments - (records - measured.record_charge());
        let exact = replay(exact_prior,work).unwrap();
        assert_eq!(exact.0,limits.base().get().max_fragments);
        assert_eq!(exact.1,work);
        assert_eq!(exact.2,fingerprints);
        assert!(matches!(replay(exact_prior+1,work),Err(e) if e.kind == Error::FragmentLimit));
        assert!(matches!(replay(0,work-1),Err(e) if e.kind == Error::TableSearchLimit));
        let other = make();
        let mut other_search = prepare_book_v2_table_search(&other,0,&limits,1_000_000,0).unwrap();
        let other_cursor = other_search.begin().unwrap();
        let mut search = prepare_book_v2_table_search(&measured,0,&limits,1_000_000,0).unwrap();
        assert!(matches!(search.evaluate(&other_cursor,capacity),Err(e) if e.kind == Error::ReceiptMismatch));
        let cursor = search.begin().unwrap();
        assert!(matches!(search.evaluate(&cursor,Length::from_raw(-1).unwrap()),Err(e) if e.kind == Error::InvalidTableCapacity));
        assert!(matches!(search.evaluate(&cursor,search.maximum_height().checked_add(Length::from_raw(1).unwrap()).unwrap()),Err(e) if e.kind == Error::InvalidTableCapacity));
    }).unwrap();
}

#[test]
fn book_v2_table_search_advances_zero_height_rows_and_uses_definition_region_capacity() {
    for mode in ["zero-height", "definition", "oversize-header"] {
        let root = Root::new();
        let limits = limits();
        let mut data = table_data();
        if mode == "zero-height" {
            for row in data["document"]["blocks"][0]["blocks"][0]["body"]
                .as_array_mut()
                .unwrap()
            {
                for cell in row["cells"].as_array_mut().unwrap() {
                    cell["blocks"] = json!([]);
                }
            }
        } else {
            let mut table = data["document"]["blocks"][0]["blocks"]
                .as_array_mut()
                .unwrap()
                .remove(0);
            // The actual definition label/gap narrows its containing region.
            // Resolve both columns from that remaining width for this positive fixture.
            table["columns"] =
                json!([{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}]);
            data["document"]["footnotes"][0]["blocks"]
                .as_array_mut()
                .unwrap()
                .insert(0, table);
            if mode == "oversize-header" {
                data["page_masters"]["masters"][0]["footnote"]["height"] = 500_000.into();
            }
        }
        renumber(&mut data["document"], &mut 0);
        let input = prepared(&root, data, b"Result", &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings =
            typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits)
                .unwrap();
        typaxis_layout::book_v2::with_converged_book_v2_body_lines(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,
            rect(500_000,500_000,10_000_000,20_000_000),1_000_000,|stable| {
            let measured = prepare_book_v2_table_measurements(prepare_book_v2_body_flow(stable.lines(),None,stable.footnotes(),&limits,0).unwrap(),&limits).unwrap();
            if mode != "zero-height" {
                assert!(matches!(typaxis_pagination::book_v2::prepare_book_v2_footnote_search(measured.flow(),&limits,1_000_000,0),Err(e) if e.kind == Error::PendingRegion("table_footnote_definition")));
            }
            let search = prepare_book_v2_table_search(&measured,0,&limits,1_000_000,0);
            if mode == "oversize-header" {
                assert!(matches!(search,Err(e) if e.kind == Error::TableHeaderOversize));
                return;
            }
            let mut search = search.unwrap();
            let before = search.begin().unwrap();
            if mode == "zero-height" {
                let fragment = search.evaluate(&before,search.header_height()).unwrap().unwrap();
                assert_eq!(fragment.before().offset(),fragment.after().offset());
                assert_eq!(fragment.before().next_row(),1);
                assert_eq!(fragment.after().next_row(),4);
                assert!(fragment.after().is_terminal());
                assert!(fragment.cells().is_empty());
            } else {
                assert_eq!(search.maximum_height().raw(),2_000_000);
                assert!(matches!(search.evaluate(&before,Length::from_raw(20_000_000).unwrap()),Err(e) if e.kind == Error::InvalidTableCapacity));
                assert!(measured.flow().definition_items(0).unwrap().len() > 1);
            }
        }).unwrap();
    }
}

#[path = "book_v2_table_footnote_tests.rs"]
mod joint_footnotes;

#[path = "book_v2_mixed_candidate_tests.rs"]
mod mixed;
