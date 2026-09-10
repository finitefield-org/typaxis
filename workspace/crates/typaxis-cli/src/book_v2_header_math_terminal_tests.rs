use super::*;
#[path = "book_v2_header_driver_math_tests.rs"]
mod driver;
#[path = "book_v2_header_reachability_tests.rs"]
mod reachability;
#[path = "book_v2_header_display_tests.rs"]
mod display;
use typaxis_layout::book_v2::{
    prepare_book_v2_body_line_variant_seed, with_rebuilt_book_v2_body_line_variants,
};
use typaxis_pagination::book_v2::*;

#[test]
fn book_v2_header_math_terminals_resolve_native_inline_and_blocks() {
    check(false, None, false, false);
}
#[test]
fn book_v2_header_math_terminals_resolve_vectors_and_equation_numbers() {
    check(true, None, false, false);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_header_resources_preserve_original_harano() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(true, Some(&font), false, false);
}
#[test]
fn book_v2_header_resources_union_contextual_glyphs() {
    const FONT: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/vmb-book/header-context-font/header-context.ttf"));
    check(true, Some(FONT), true, false);
}
#[test]
fn book_v2_header_pdfs_preserve_raster_and_nonpainting_lines() {
    check(true, None, false, true);
}
fn check(vector: bool, font: Option<&[u8]>, contextual: bool, raster_empty: bool) {
    for notes in [false, true] {
        let root = Root::new();
        let limits = limits();
        let (data,text) = header_fixture(vector,contextual,raster_empty,notes);
        let input = if let Some(font) = font {
            assert!(vector);
            super::super::vector_tests::vector_input_with_font(&root, data, &limits, font, text)
        } else if vector {
            super::super::vector_tests::vector_input(&root, data, &limits)
        } else {
            native_input(&root, data, &limits)
        };
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let native =
            compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0).unwrap();
        assert_eq!(native.is_some(), !vector);
        let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_for_reflow(
            &flow, &mut 0, 1_000_000, 0, 0,
        )
        .unwrap();
        let seed = prepare_book_v2_body_line_variant_seed(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            plan.measurement_body(),
            10_000_000,
            0,
            native.as_ref(),
            limits.base().get().max_line_reshape_passes,
            Some(&plan),
            None,
        )
        .unwrap();
        let (widths, header_index, ends, parent_width) =
            typaxis_layout::book_v2::with_rebuilt_book_v2_body_line_variant(
                &seed,
                10_000_000,
                0,
                |v| {
                    let index = v
                        .lines()
                        .paragraphs()
                        .iter()
                        .position(|p| {
                            p.lines().iter().flat_map(|l| l.items()).any(|i| {
                                matches!(
                                    i,
                                    ProductionPlacedInline::BookV2Math(_)
                                        | ProductionPlacedInline::Vector(_)
                                )
                            })
                        })
                        .unwrap();
                    let widths = v
                        .lines()
                        .prepared()
                        .paragraphs()
                        .iter()
                        .zip(v.lines().paragraphs())
                        .map(|(p, l)| {
                            vec![l.inline_size(); p.items().unwrap().units().len().max(1)]
                        })
                        .collect::<Vec<_>>();
                    let ends = vec![
                        if contextual { 2 } else { 7 },
                        u32::try_from(widths[index].len()).unwrap(),
                    ];
                    (widths, index, ends, v.lines().frames().unwrap().measurement_region(flow.tables()[0].owner()).unwrap().width())
                },
            )
            .unwrap();
        let views = widths
            .iter()
            .map(|w| Some(w.as_slice()))
            .collect::<Vec<_>>();
        let mut retained = vec![None; views.len()];
        retained[header_index] = Some(ends.as_slice());
        let assignment =
            typaxis_layout::book_v2::BookV2SourceWidthAssignments::with_retained_line_ends(
                &flow, &views, &retained,
            )
            .unwrap();
        let split_seed = prepare_book_v2_body_line_variant_seed(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            plan.measurement_body(),
            10_000_000,
            0,
            native.as_ref(),
            limits.base().get().max_line_reshape_passes,
            Some(&plan),
            Some(&assignment),
        )
        .unwrap();
        {
            use crate::book_v2_resources::converged_pdf::header_catalog_driver::{with_header_catalog, HeaderWidthRequest, HeaderCatalogBudget};
            let requests=[HeaderWidthRequest{table_index:0,owner:flow.tables()[0].owner(),parent_width}];
            let mut budget=HeaderCatalogBudget::default();
            with_header_catalog(&split_seed,&requests,&limits,100_000_000,&mut budget,|catalog,budget| {
                assert_eq!(catalog.base().flow().lines().fingerprint(),split_seed.fingerprint());
                let header=catalog.for_region_width(0,None).unwrap();
                assert_eq!(header.variant().flow().lines().paragraphs()[header_index].lines().len(),1);
                let mut search=prepare_book_v2_table_body_search(catalog.base(),&limits,100_000_000-budget.work,budget.records).unwrap();
                search.set_table_header_catalog(catalog).unwrap();
                let stable=search.select_stable_mixed_pages(4).unwrap();
                let placed=search.place_mixed_pages(stable.sequence()).unwrap();
                let closed=search.close_mixed_page_sources(&stable,&placed).unwrap();
                assert!(closed.has_header_variants());
                assert_eq!(closed.semantic_math(),2);
                assert_eq!(closed.repeated_math(),2*(placed.pages().len()-1));
                let terminals=search.finalize_mixed_page_math(closed,&limits,0).unwrap();
                assert_math_terminals(&terminals);
                for terminal in terminals.terminals() {
                    match terminal.source() {
                        BookV2BodyMathSource::Native(receipt)=>assert!(std::ptr::eq(receipt,native.as_ref().unwrap().receipt(receipt.node_id()).unwrap())),
                        BookV2BodyMathSource::Vector(binding)=>assert!(std::ptr::eq(binding,bindings.receipt(binding.node_id()).unwrap())),
                    }
                }
                eprintln!("automatic header math: vector={vector},notes={notes},contextual={contextual},raster={raster_empty},pages={},work={},records={}",placed.pages().len(),budget.work+terminals.work_steps(),terminals.record_charge());
                Ok(())
            }).unwrap();
        }
        with_rebuilt_book_v2_body_line_variants(&[&seed,&split_seed],10_000_000,0,|set| {
            let numbers=set.variants().iter().map(|v|typaxis_shaping::book_v2::shape_book_v2_equation_numbers(v.lines().prepared().shaped(),&limits,0).unwrap()).collect::<Vec<_>>();
            let blocks=set.variants().iter().zip(&numbers).map(|(v,n)|typaxis_layout::book_v2::prepare_book_v2_vector_blocks(v.lines(),n.as_ref(),&limits,0).unwrap()).collect::<Vec<_>>();
            let measured=set.variants().iter().zip(&blocks).map(|(v,b)|prepare_book_v2_table_measurements(prepare_book_v2_body_flow(v.lines(),b.as_ref(),v.footnotes(),&limits,0).unwrap(),&limits).unwrap()).collect::<Vec<_>>();
            assert!(measured[0].flow().lines().paragraphs()[header_index].lines().len()<measured[1].flow().lines().paragraphs()[header_index].lines().len());
            if contextual {
                let first_gid = |i:usize| measured[i].flow().lines().paragraphs()[header_index].lines()[0].items().iter().find_map(|item| match item {ProductionPlacedInline::Text(c) => c.glyphs().first().map(|g| g.glyph().original_gid.get()), _ => None}).unwrap();
                assert_eq!((first_gid(0), first_gid(1)), (59, 71), "line context must change f to Z only before the retained boundary");
            }
            for base_index in 0..2 {
            let base=&measured[base_index];
            let variant=&measured[1-base_index];
            assert!(!std::ptr::eq(base.flow().lines(),variant.flow().lines()));
            let header=prepare_book_v2_table_header_variant(&set,base,variant,0,&limits,10_000_000,0).unwrap();
            let catalog=prepare_book_v2_table_header_catalog(base,&[&header],&limits,10_000_000,0).unwrap();
            let run=|work,prior,spool| -> Result<_,typaxis_pagination::ProductionBodyPaginationError> {
                let mut search=prepare_book_v2_table_body_search(base,&limits,work,prior)?;
                search.set_table_header_catalog(&catalog)?;
                let stable=search.select_stable_mixed_pages(4)?;
                let placed=search.place_mixed_pages(stable.sequence())?;
                let closed=search.close_mixed_page_sources(&stable,&placed)?;
                assert!(closed.has_header_variants());
                assert_eq!(closed.semantic_math(),2);
                assert_eq!(closed.repeated_math(),2*(placed.pages().len()-1));
                let terminals=search.finalize_mixed_page_math(closed,&limits,spool)?;
                assert_math_terminals(&terminals);
                assert_eq!(terminals.terminals().len(),2*placed.pages().len());
                assert_eq!(placed.pages().iter().map(|p|p.equation_numbers().len()).sum::<usize>(),if vector {placed.pages().len()}else{0});
                assert_eq!(terminals.spool_charge(),spool.max(native.as_ref().map_or(0,|n|n.spool_charge()))+terminals.canonical_bytes().len() as u64);
                for t in terminals.terminals() {
                    match t.source() {
                        BookV2BodyMathSource::Native(receipt)=>assert!(std::ptr::eq(receipt,native.as_ref().unwrap().receipt(receipt.node_id()).unwrap())),
                        BookV2BodyMathSource::Vector(binding)=>{assert!(vector);assert!(std::ptr::eq(binding,bindings.receipt(binding.node_id()).unwrap()));}
                    }
                }
                if work==100_000_000 && prior==0 && spool==0 {display::verify(&terminals,input.resources(),&limits, (contextual && base_index == 0).then_some(71)); }
                Ok((terminals.work_steps(),terminals.record_charge(),terminals.spool_charge(),terminals.fingerprint(),terminals.canonical_bytes().len(),placed.pages().len()))
            };
            let full=run(100_000_000,0,0).unwrap();
            assert_eq!(run(full.0,0,0).unwrap(),full);
            assert!(run(full.0-1,0,0).is_err());
            let raised=limits.base().get().max_fragments/2;
            let delta=run(full.0,raised,0).unwrap().1-raised;
            let exact=limits.base().get().max_fragments-delta;
            assert_eq!(run(full.0,exact,0).unwrap().1,limits.base().get().max_fragments);
            assert!(run(full.0,exact+1,0).is_err());
            let exact=limits.base().get().max_spool_bytes-full.4 as u64;
            assert_eq!(run(full.0,0,exact).unwrap().2,limits.base().get().max_spool_bytes);
            assert!(run(full.0,0,exact+1).is_err());
            eprintln!("header terminals: vector={vector},notes={notes},base={base_index},pages={},semantic=2,repeated={},work={},records={},spool={},bytes={}",full.5,2*(full.5-1),full.0,full.1,full.2,full.4);
            }
        }).unwrap();
    }
}

fn header_fixture(vector: bool, contextual: bool, raster_empty: bool, notes: bool) -> (Value, &'static [u8]) {
    let mut data = if vector {
        super::super::vector_tests::equations::blocks::block_data("center", true)
    } else {
        native_data()
    };
    let text = if contextual {
        b"f i    x+y ABCDE(1)".as_slice()
    } else {
        b"Result x+y Proof(1)".as_slice()
    };
    if contextual {
        data["text_buffers"][0]["utf8"] = std::str::from_utf8(text).unwrap().into();
    }
    let mut header = data["document"]["blocks"][0]["blocks"].clone();
    let span = data["document"]["blocks"][0]["span"].clone();
    if raster_empty {
        let mut raster = super::super::super::data()["resources"]["images"][0].clone();
        raster["image_id"] = 1.into();
        data["resources"]["images"]
            .as_array_mut()
            .unwrap()
            .push(raster);
        let order = data["style_sheet"]["rules"].as_array().unwrap().len();
        data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"header-raster","selector":"figure","source_order":order,"extends":null,"declarations":[{"name":"width","important":false,"value":{"kind":"length","value":12*65536}}]}));
        header.as_array_mut().unwrap().push(json!({"kind":"figure","node_id":0,"span":span,"classes":[],"image_id":1,"placement":"block","alt":"header raster","caption":[]}));
        header.as_array_mut().unwrap().push(json!({"kind":"paragraph","node_id":0,"span":span,"classes":[],"children":[{"kind":"anchor","node_id":0,"span":span,"anchor_id":"empty.begin"}]}));
    }
    let mut paragraph = header[0].clone();
    paragraph["children"].as_array_mut().unwrap().truncate(1);
    paragraph["span"] = paragraph["children"][0]["span"].clone();
    header[0]["children"].as_array_mut().unwrap().insert(
        0,
        json!({"kind":"anchor","node_id":0,"span":span,"anchor_id":"header.begin"}),
    );
    let original = header[0].clone();
    header[0] = json!({"kind":"list","node_id":0,"span":span,"classes":[],"ordered":true,"start":1,"items":[{"node_id":0,"span":span,"blocks":[original]}]});
    let mut rule = data["style_sheet"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["selector"] == "paragraph")
        .unwrap()
        .clone();
    rule["selector"] = "list".into();
    rule["style_id"] = "header-list".into();
    rule["source_order"] = data["style_sheet"]["rules"]
        .as_array()
        .unwrap()
        .len()
        .into();
    data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(rule);
    let row = |blocks: Value| json!({"node_id":0,"span":span,"cells":[{"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":blocks}]});
    let table = json!({"kind":"table","node_id":0,"span":span,"classes":[],"columns":[{"kind":"fraction","weight":1}],"head":[row(header)],"body":(0..24).map(|_|row(json!([paragraph]))).collect::<Vec<_>>()});
    if notes {
        let mut reference = paragraph.clone();
        reference["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":paragraph["span"],"footnote_id":"note"}));
        data["document"]["blocks"] = json!([reference]);
        data["document"]["footnotes"] =
            json!([{"node_id":0,"span":span,"footnote_id":"note","blocks":[table]}]);
    } else {
        data["document"]["blocks"] = json!([table]);
    }
    renumber(&mut data["document"], &mut 0);
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = (400 * 65536).into();
    master["height"] = (600 * 65536).into();
    master["trim"] = json!({"x":0,"y":0,"width":400*65536,"height":600*65536});
    let region_height = if raster_empty { 160 } else { 100 };
    master["body"] =
        json!({"x":20*65536,"y":20*65536,"width":240*65536,"height":region_height*65536});
    if notes {
        master["footnote"] =
            json!({"x":30*65536,"y":200*65536,"width":240*65536,"height":region_height*65536});
    }
    (data,text)
}

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
        if let Some(a) = v.get_mut(key).and_then(Value::as_array_mut) {
            for c in a {
                renumber(c, next);
            }
        }
    }
    if let Some(number) = v.get_mut("equation_number") {
        if !number.is_null() {
            renumber(number, next);
        }
    }
}
