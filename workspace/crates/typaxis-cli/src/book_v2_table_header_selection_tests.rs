use super::*;
use std::collections::BTreeSet;
use typaxis_layout::book_v2::with_rebuilt_book_v2_body_line_variants;
use typaxis_pagination::book_v2::{
    prepare_book_v2_table_header_variant, prepare_book_v2_table_search,
};
use typaxis_pagination::ProductionTableContentSource;

pub(super) fn fixture(notes: bool, mode: &str, text: &str) -> Value {
    let mut data = super::super::table_width_occurrences::occurrence_data(notes, "header");
    if mode == "nested-header" {
        super::header_variants::nest_header(&mut data, notes);
    }
    let table = if notes {
        &mut data["document"]["footnotes"][0]["blocks"][0]
    } else {
        &mut data["document"]["blocks"][0]
    };
    for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
        let p = cell["blocks"][0].clone();
        cell["blocks"] = vec![p; 16].into();
    }
    if mode == "forced" {
        let p = table["body"][0]["cells"][0]["blocks"][0].clone();
        table["body"][0]["cells"][0]["blocks"]
            .as_array_mut()
            .unwrap()
            .insert(
                2,
                json!({"kind":"page_break","node_id":0,"span":p["span"],"classes":[]}),
            );
    }
    if mode == "span" {
        let mut row = table["body"][0].clone();
        row["cells"].as_array_mut().unwrap().remove(0);
        table["body"][0]["cells"][0]["rowspan"] = 2.into();
        table["body"].as_array_mut().unwrap().push(row);
    }
    if mode == "nested-body" {
        let child = table.clone();
        table["body"][0]["cells"][0]["blocks"] = json!([child]);
    }
    for (index, master) in data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        master["height"] = (4500i64 * 65536).into();
        master["trim"]["height"] = (4500i64 * 65536).into();
        if index == 0 {
            master["body"]["height"] = (2048i64 * 65536).into();
        } else if mode == "common" {
            master["body"]["height"] = (256i64 * 65536).into();
        }
        if notes {
            master["footnote"]["y"] = (2200i64 * 65536).into();
            if index == 0 {
                master["footnote"]["height"] = (2048i64 * 65536).into();
            } else if mode == "common" {
                master["footnote"]["height"] = (256i64 * 65536).into();
            }
        }
    }
    if mode == "common" {
        for rule in data["style_sheet"]["rules"].as_array_mut().unwrap() {
            if rule["style_id"] == "left" {
                rule["declarations"][0]["value"]["value"] = (23i64 * 65536).into();
            }
        }
    }
    fn expand(v: &mut Value, length: usize) {
        if let Some(a) = v.as_array_mut() {
            for v in a {
                expand(v, length);
            }
        } else if let Some(o) = v.as_object_mut() {
            for (k, v) in o {
                if k == "end_byte" && v.as_u64() == Some(6) {
                    *v = length.into();
                } else {
                    expand(v, length);
                }
            }
        }
    }
    expand(&mut data, text.len());
    data["text_buffers"][0]["utf8"] = text.into();
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
    data
}

fn check(font: Option<&[u8]>) {
    for mode in ["common", "forced", "span", "nested-header", "nested-body"] {
        for notes in [false, true] {
            let root = Root::new();
            let limits = driver_limits();
            let text = if font.is_some() {
                "本文を続けて組み直す本文を続けて組み直す"
            } else {
                "Pro Pro Pro Pro Pro Pro Pro"
            };
            let mut data = fixture(notes, mode, text);
            let input = if let Some(font) = font {
                data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
                data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .into();
                let body = body_with_source(&root, data, text.as_bytes(), &limits);
                fs::write(root.0.join("body.bin"), font).unwrap();
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
            let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
            let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
            let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_for_reflow(
                &flow, &mut 0, 1_000_000, 0, 0,
            )
            .unwrap();
            let profiles = vec![None; flow.paragraphs().len()];
            let width =
                |pt: i64| PositiveLength::new(Length::from_raw(pt * 65536).unwrap()).unwrap();
            let narrow = [(flow.tables()[0].owner(), width(140))];
            let wide = [(
                flow.tables()[0].owner(),
                width(if notes { 200 } else { 220 }),
            )];
            let a = BookV2SourceWidthAssignments::new(&flow, &profiles)
                .unwrap()
                .with_root_table_widths(&narrow);
            let b = BookV2SourceWidthAssignments::new(&flow, &profiles)
                .unwrap()
                .with_root_table_widths(&wide);
            let make = |assignment| {
                prepare_book_v2_body_line_variant_seed(
                    &policy,
                    &flow,
                    input.resources(),
                    &bindings,
                    &limits,
                    JapaneseLineBreakMode::Normal,
                    plan.measurement_body(),
                    10_000_000,
                    0,
                    None,
                    limits.base().get().max_line_reshape_passes,
                    Some(&plan),
                    Some(assignment),
                )
                .unwrap()
            };
            let first = make(&a);
            let second = make(&b);
            with_rebuilt_book_v2_body_line_variants(&[&first,&second],10_000_000,0,|set| {
                let mut measured=set.variants().iter().map(|v|prepare_book_v2_table_measurements(prepare_book_v2_body_flow(v.lines(),None,v.footnotes(),&limits,0).unwrap(),&limits).unwrap()).collect::<Vec<_>>();
                let v=&set.variants()[1];
                measured.push(prepare_book_v2_table_measurements(prepare_book_v2_body_flow(v.lines(),None,v.footnotes(),&limits,0).unwrap(),&limits).unwrap());
                assert!(!std::ptr::eq(&measured[1],&measured[2]));
                assert_eq!(measured[1].fingerprint(),measured[2].fingerprint());
                for base_index in 0..2 {
                let base=&measured[base_index];
                let headers=[prepare_book_v2_table_header_variant(&set,base,&measured[0],0,&limits,1_000_000,0).unwrap(),prepare_book_v2_table_header_variant(&set,base,&measured[1],0,&limits,1_000_000,0).unwrap(),prepare_book_v2_table_header_variant(&set,base,&measured[2],0,&limits,1_000_000,0).unwrap()];
                assert!(headers[0].height()>headers[1].height());
                let mut expected=BTreeSet::new();
                for table in base.tables() {
                    for contents in table.caption().map(|c|c.content()).into_iter().chain(table.cells().iter().map(|c|c.content())) {
                        for content in contents { if let ProductionTableContentSource::FlowItem(i)=content.source() {assert!(expected.insert(i));} }
                    }
                }
                let run=|maximum,prior| -> Result<(u64,u64,[u8;32],usize,usize),typaxis_pagination::ProductionBodyPaginationError> {
                    let mut search=prepare_book_v2_table_search(base,0,&limits,maximum,prior)?;
                    let mut cursor=search.begin()?;
                    let mut coverage=BTreeSet::new();
                    let step=Length::from_raw(23*65536).unwrap();
                    let mut capacity=headers[0].height().checked_add(step).unwrap();
                    let mut pages=0;
                    let mut digest=[0u8;32];
                    while !cursor.has_started_rows() {
                        let Some(selected)=search.evaluate(&cursor,capacity)? else {capacity=capacity.checked_add(step).unwrap();continue;};
                        for range in selected.semantic_leaf_ranges() {for i in range {assert!(coverage.insert(i));}}
                        cursor=selected.after();pages+=1;
                    }
                    assert!(!cursor.is_terminal(),"{mode}/{notes}");
                    // Two choices from the same actual source cursor must use
                    // different reservations; neither trial consumes the cursor.
                    let small=search.evaluate_with_header(&cursor,capacity,&headers[1],&limits)?;
                    let before_duplicate=search.record_charge();
                    let duplicate=search.evaluate_with_header(&cursor,capacity,&headers[2],&limits)?;
                    assert_eq!(small.as_ref().map(|s|s.fingerprint()),duplicate.as_ref().map(|s|s.fingerprint()));
                    // Equal geometry still owns an independent collected table
                    // projection; its line graph is already held by the exact set.
                    // A max-only ledger would hide this independent allocation.
                    assert!(search.record_charge()>=before_duplicate+measured[2].retained_records());
                    let large=search.evaluate_with_header(&cursor,capacity,&headers[0],&limits)?;
                    if mode=="common" {
                        assert!(small.as_ref().unwrap().semantic_leaf_ranges().map(|r|r.len()).sum::<usize>()>large.as_ref().unwrap().semantic_leaf_ranges().map(|r|r.len()).sum::<usize>());
                    }
                    // A no-fit trial must restore ordinary same-owner selection.
                    assert!(search.evaluate_with_header(&cursor,headers[1].height(),&headers[1],&limits)?.is_none());
                    let ordinary=search.evaluate(&cursor,capacity)?;
                    if let Some(ordinary)=ordinary {assert_eq!(ordinary.header_height(),headers[base_index].height());}
                    let mut base_repeated=0;
                    while !cursor.is_terminal() {
                        assert!(pages<500);
                        let header=&headers[pages%2];
                        let Some(selected)=search.evaluate_with_header(&cursor,capacity,header,&limits)? else {
                            capacity=capacity.checked_add(step).unwrap();continue;
                        };
                        assert_eq!(selected.header_height(),header.height());
                        assert_eq!(selected.available_height(),capacity);
                        assert!(selected.used_height()<=capacity);
                        let mut variant_leaves=0;
                        for leaf in selected.placement_leaves() {
                            let leaf=leaf?;
                            let item=leaf.measurements().item(leaf.global_item_index()).unwrap();
                            assert!(leaf.top()>=Length::ZERO);
                            assert!(leaf.top().checked_add(item.consumed_height()?).unwrap()<=selected.used_height());
                            if leaf.uses_header_variant() {
                                assert!(std::ptr::eq(leaf.measurements(),header.variant()));assert!(leaf.repeated());variant_leaves+=1;
                            } else {
                                assert!(std::ptr::eq(leaf.measurements(),base));assert!(leaf.top()>=header.height());
                                base_repeated+=usize::from(leaf.repeated());
                            }
                        }
                        assert_eq!(variant_leaves,header.leaves().len());
                        for range in selected.semantic_leaf_ranges() {for i in range {assert!(coverage.insert(i),"repeated semantic item {mode}/{notes}/{i}");}}
                        let mut bytes=[0u8;64];bytes[..32].copy_from_slice(&digest);bytes[32..].copy_from_slice(&selected.fingerprint());digest=typaxis_core::sha256(&bytes);
                        cursor=selected.after();pages+=1;
                    }
                    assert_eq!(coverage,expected,"{mode}/{notes}");
                    assert_eq!(base_repeated>0,mode=="nested-body");
                    Ok((search.work_charge(),search.record_charge(),digest,pages,base_repeated))
                };
                let full=run(10_000_000,0).unwrap_or_else(|e|panic!("{mode}/{notes}: {e:?}"));
                assert_eq!(run(full.0,0).unwrap(),full);
                assert!(run(full.0-1,0).is_err());
                let raised=limits.base().get().max_fragments/2;
                let extra=run(full.0,raised).unwrap().1-raised;
                let exact=limits.base().get().max_fragments-extra;
                assert_eq!(run(full.0,exact).unwrap().1,limits.base().get().max_fragments);
                assert!(run(full.0,exact+1).is_err());
                let mut search=prepare_book_v2_table_search(base,0,&limits,10_000_000,0).unwrap();
                let initial=search.begin().unwrap();
                assert!(search.evaluate_with_header(&initial,headers[0].height(),&headers[1],&limits).is_err());
                eprintln!("table header selection: {mode},base={base_index},notes={notes},harano={},work={},records={},fragments={},base_repeats={}",font.is_some(),full.0,full.1,full.3,full.4);
                }
            }).unwrap();
        }
    }
}

#[test]
fn book_v2_table_header_selection_uses_actual_variants_and_source_cursors() {
    check(None);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_header_selection_uses_original_harano_variants() {
    check(Some(
        &fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap(),
    ));
}
