use super::*;
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
    prepare_book_v2_table_measurements,
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
    if let Some(number) = v.get_mut("equation_number").filter(|v| !v.is_null()) {
        renumber(number, next);
    }
}
#[test]
fn book_v2_page_numbers_follow_actual_repeated_headers_and_stable_geometry() {
    check_page_numbers(false, 0);
}
#[test]
fn book_v2_source_closure_keeps_unreferenced_numbered_math_explicitly_unplaced() {
    check_page_numbers(true, 0);
}
#[test]
fn book_v2_math_terminals_keep_signed_source_pen_origins() {
    assert_ne!(
        check_page_numbers(false, 0),
        check_page_numbers(false, -65_536)
    );
}
fn check_page_numbers(unreferenced: bool, origin_x: i64) -> [u8; 32] {
    let root = Root::new();
    let limits = limits();
    let mut data = block_data("start", true);
    data["document"]["blocks"][0]["blocks"][0]["children"][2]["metrics"]["origin_x"] =
        origin_x.into();
    data["document"]["blocks"][0]["blocks"][2]["metrics"]["origin_x"] = origin_x.into();
    let span = data["document"]["blocks"][0]["span"].clone();
    let paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    let math = data["document"]["blocks"][0]["blocks"][2].clone();
    let row = |block| json!({"node_id":0,"span":span,"cells":[{"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[block]}]});
    data["document"]["blocks"][0]["blocks"] = json!([{"kind":"table","node_id":0,"span":span,"classes":[],"columns":[{"kind":"fraction","weight":1}],"head":[row(math.clone())],"body":vec![row(paragraph);12]}]);
    if unreferenced {
        data["document"]["footnotes"] =
            json!([{"node_id":0,"span":span,"footnote_id":"unused","blocks":[math]}]);
    }
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 22_000_000.into();
    master["height"] = 24_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":22_000_000,"height":24_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":20_000_000,"height":3_000_000});
    if unreferenced {
        master["footnote"] =
            json!({"x":500_000,"y":21_000_000,"width":20_000_000,"height":1_500_000});
    }
    renumber(&mut data["document"], &mut 0);
    let input = vector_input(&root, data, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let rect = Rect::new(
        Length::from_raw(500_000).unwrap(),
        Length::from_raw(500_000).unwrap(),
        PositiveLength::new(Length::from_raw(20_000_000).unwrap()).unwrap(),
        PositiveLength::new(Length::from_raw(3_000_000).unwrap()).unwrap(),
    );
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        rect,
        1_000_000,
        |lines| {
            let shapes =
                shape_book_v2_equation_numbers(lines.lines().prepared().shaped(), &limits, 0)
                    .unwrap()
                    .unwrap();
            let blocks = prepare_book_v2_vector_blocks(lines.lines(), Some(&shapes), &limits, 0)
                .unwrap()
                .unwrap();
            let measured = prepare_book_v2_table_measurements(
                prepare_book_v2_body_flow(
                    lines.lines(),
                    Some(&blocks),
                    lines.footnotes(),
                    &limits,
                    0,
                )
                .unwrap(),
                &limits,
            )
            .unwrap();
            let number = blocks.blocks()[0].equation_number().unwrap();
            let run = |prior, work, prior_spool| -> Result<(u64, u64, u64, [u8;32]), ProductionBodyPaginationError> {
                let mut search =
                    prepare_book_v2_table_body_search(&measured, &limits, work, prior)?;
                let stable = search.select_stable_mixed_pages(2)?;
                assert_eq!(stable.passes(), 2);
                let placed = search.place_mixed_pages(stable.sequence())?;
                assert!(placed.pages().len() > 2);
                let mut offset = 0;
                let mut semantic_numbers = 0;
                for (index, page) in placed.pages().iter().enumerate() {
                    assert_eq!(page.equation_numbers().len(), 1);
                    let copy = &page.equation_numbers()[0];
                    assert_eq!(copy.repeated_header(), index > 0);
                    semantic_numbers += usize::from(!copy.repeated_header());
                    let geometry = copy.geometry();
                    assert_eq!(geometry.page_index() as usize, index);
                    let local = geometry.fragment_index() as usize - offset;
                    let parent = page.fragments()[local].fragment();
                    assert_eq!(geometry.owner(), number.owner());
                    assert_eq!(geometry.parent_owner(), parent.owner());
                    assert_eq!(
                        geometry.shape_fingerprint(),
                        shapes.shapes()[0].fingerprint()
                    );
                    assert_eq!(geometry.bounds().width(), shapes.shapes()[0].width());
                    assert_eq!(geometry.bounds().height(), shapes.shapes()[0].height());
                    assert_eq!(
                        geometry.bounds().y(),
                        parent
                            .bounds()
                            .y()
                            .checked_add(number.top_offset().get())
                            .unwrap()
                    );
                    assert_eq!(
                        geometry
                            .bounds()
                            .x()
                            .checked_add(geometry.bounds().width().get()),
                        parent
                            .bounds()
                            .x()
                            .checked_add(parent.bounds().width().get())
                    );
                    let formula = parent.viewport().unwrap();
                    assert!(
                        geometry.bounds().x()
                            >= formula
                                .x()
                                .checked_add(formula.width().get())
                                .unwrap()
                                .checked_add(number.minimum_gap().get())
                                .unwrap()
                    );
                    assert_eq!(
                        page.cell_roles()[local].unwrap().repeated_header(),
                        copy.repeated_header()
                    );
                    offset += page.fragments().len();
                }
                assert_eq!(semantic_numbers, 1);
                let closure = search.close_mixed_page_sources(&stable, &placed)?;
                assert_eq!(
                    closure.semantic_fragments(),
                    measured.flow().body_items().len()
                );
                assert_eq!(closure.repeated_fragments(), placed.pages().len() - 1);
                assert_eq!(
                    closure.unreferenced_definitions(),
                    usize::from(unreferenced)
                );
                assert_eq!(closure.semantic_math(), 13);
                assert_eq!(closure.repeated_math(), placed.pages().len() - 1);
                let terminal = search.finalize_mixed_page_math(closure, &limits, prior_spool)?;
                assert_math_terminals(&terminal);
                if prior == 0 && work == 10_000_000 && prior_spool == 0 {
                    assert_math_display(&terminal,input.resources(),&limits);
                    let other_root=Root::new();
                    let other=vector_input(&other_root,block_data("start",true),&limits);
                    assert_eq!(other.resources().image(typaxis_core::ImageResourceId::new(0)).unwrap().content_hash(),input.resources().image(typaxis_core::ImageResourceId::new(0)).unwrap().content_hash());
                    assert!(matches!(typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(&terminal,other.resources(),&limits,10_000_000,0,0),Err(typaxis_display_list::book_v2::BookV2MathDisplayError::Display(e)) if e.kind==typaxis_display_list::ProductionBodyDisplayErrorKind::ReceiptMismatch));
                }
                assert_eq!(terminal.terminals().len(), 13 + placed.pages().len() - 1);
                assert_eq!(terminal.record_charge(), search.record_charge());
                assert_eq!(terminal.work_steps(), search.work_steps());
                assert_eq!(terminal.spool_charge(), prior_spool + terminal.canonical_bytes().len() as u64);
                let again = search.close_mixed_page_sources(&stable, &placed)?;
                let again = search.finalize_mixed_page_math(again, &limits, 0)?;
                assert_eq!(again.fingerprint(), terminal.fingerprint());
                assert_eq!(again.canonical_bytes(), terminal.canonical_bytes());
                assert_eq!(again.spool_charge(), terminal.spool_charge() + terminal.canonical_bytes().len() as u64);
                Ok((search.record_charge(), search.work_steps(), search.terminal_spool_charge(), terminal.fingerprint()))
            };
            let (records, work, spool, fingerprint) = run(0, 10_000_000, 0).unwrap();
            let prior = limits.base().get().max_fragments - (records - measured.record_charge());
            assert_eq!(
                run(prior, work, 0).unwrap(),
                (limits.base().get().max_fragments, work, spool, fingerprint)
            );
            assert!(matches!(run(prior+1,work,0),Err(e) if e.kind==Error::FragmentLimit));
            assert!(matches!(run(0,work-1,0),Err(e) if e.kind==Error::FootnoteSearchLimit));
            let prior_spool = limits.base().get().max_spool_bytes - spool;
            assert_eq!(run(0,work,prior_spool).unwrap(), (records, work, limits.base().get().max_spool_bytes, fingerprint));
            assert!(matches!(run(0,work,prior_spool+1),Err(e) if e.kind==Error::SpoolLimit));
            fingerprint
        },
    )
    .unwrap()
}
