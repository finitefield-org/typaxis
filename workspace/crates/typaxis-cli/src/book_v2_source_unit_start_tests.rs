use super::*;
use typaxis_core::Length;
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, with_converged_book_v2_body_lines_with_source_widths,
    BookV2SourceWidthAssignments,
};
use typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_for_reflow;

#[test]
fn book_v2_source_unit_starts_validate_binding_and_final_physical_origins() {
    for notes in [false, true] {
        let root = Root::new();
        let limits = driver_limits();
        let text = "Pro Pro Pro";
        let data = crate::book_v2_resources::tests::shaping_tests::table_width_frames::table_data(
            text, notes,
        );
        let input = prepared(&root, data, text.as_bytes(), &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let foreign = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let plan =
            prepare_book_v2_page_frame_plan_for_reflow(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
        assert!(!plan.requires_width_reflow());
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        with_converged_book_v2_body_lines_with_source_widths(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,plan.measurement_body(),100_000_000,None,limits.base().get().max_line_reshape_passes,Some(&plan),None,|initial| {
            let lines=initial.lines();let frames=lines.frames().unwrap();
            let widths=lines.prepared().paragraphs().iter().zip(frames.paragraphs()).map(|(p,f)|vec![f.width();p.items().unwrap().units().len().max(1)]).collect::<Vec<_>>();
            let starts=widths.iter().zip(frames.paragraphs()).map(|(w,f)|vec![f.start();w.len()]).collect::<Vec<_>>();
            let width_views=widths.iter().map(|v|Some(v.as_slice())).collect::<Vec<_>>();
            let first=flow.paragraphs().iter().position(|p|p.owner()>flow.tables()[0].owner()).unwrap();
            let run=|assignment:&BookV2SourceWidthAssignments<'_, '_>,maximum,valid| {
                with_converged_book_v2_body_lines_with_source_widths(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,plan.measurement_body(),maximum,None,limits.base().get().max_line_reshape_passes,Some(&plan),Some(assignment),|next| {
                    let new_lines=next.lines();
                    let body=prepare_book_v2_body_flow(new_lines,None,next.footnotes(),&limits,0).unwrap();
                    let measured=prepare_book_v2_table_measurements(body,&limits).unwrap();
                    let mut search=prepare_book_v2_table_body_search(&measured,&limits,10_000_000,measured.record_charge()).unwrap();
                    let pages=search.select_stable_mixed_pages(limits.base().get().max_layout_passes).unwrap();
                    let placed=search.place_mixed_pages(pages.sequence()).unwrap();
                    let closed=search.close_mixed_page_sources(&pages,&placed).unwrap();
                    let finalization=search.finalize_mixed_page_math(closed,&limits,0);
                    if valid {assert!(finalization.is_ok(),"{:?}",finalization.err());} else {
                        assert!(matches!(finalization,Err(e) if e.kind==typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch));
                    }
                    (next.candidate_steps(),new_lines.fingerprint())
                })
            };
            for occurrence in [false,true] {
            for tamper in [false,true] {
                let mut origins=starts.clone();
                if tamper {for x in &mut origins[first] {*x=x.checked_sub(Length::from_raw(1).unwrap()).unwrap();}}
                let views=origins.iter().map(|v|Some(v.as_slice())).collect::<Vec<_>>();
                let a=BookV2SourceWidthAssignments::new(&flow,&width_views).unwrap().with_source_unit_starts(&views).unwrap();
                let a=if occurrence {a.with_table_occurrence_frames()} else {a};
                let full=run(&a,100_000_000,!tamper).unwrap();
                assert_eq!(run(&a,full.0,!tamper).unwrap(),full);assert!(run(&a,full.0-1,!tamper).is_err());
                eprintln!("unit starts finalization: notes={notes},occurrence={occurrence},tamper={tamper},work={}",full.0);
            }
            }
            let missing=BookV2SourceWidthAssignments::new(&flow,&width_views).unwrap().with_table_occurrence_frames();
            assert!(run(&missing,100_000_000,true).is_err());
            let views=starts.iter().map(|v|Some(v.as_slice())).collect::<Vec<_>>();
            assert!(BookV2SourceWidthAssignments::new(&flow,&width_views).unwrap().with_source_unit_starts(&views[..views.len()-1]).is_err());
            let a=BookV2SourceWidthAssignments::new(&foreign,&width_views).unwrap().with_source_unit_starts(&views).unwrap();
            assert!(run(&a,100_000_000,true).is_err());
            for mode in ["short","long","before","after","missing-width"] {
                let mut origins=starts.clone();let mut w=width_views.clone();
                match mode {
                    "short"=>{origins[first].pop();},
                    "long"=>{let value=origins[first][0]; origins[first].push(value);},
                    "before"=>{origins[first][0]=Length::from_raw(-1_000_000_000).unwrap();},
                    "after"=>{origins[first][0]=Length::from_raw(1_000_000_000).unwrap();},
                    "missing-width"=>{w[first]=None;},
                    _=>unreachable!(),
                }
                let views=origins.iter().map(|v|Some(v.as_slice())).collect::<Vec<_>>();
                let a=BookV2SourceWidthAssignments::new(&flow,&w).unwrap().with_source_unit_starts(&views).unwrap();
                assert!(run(&a,100_000_000,true).is_err(),"{mode}");
            }
        }).unwrap();
    }
}
