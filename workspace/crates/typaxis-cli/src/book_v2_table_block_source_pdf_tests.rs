use super::*;

fn wrap_source_table(data: &mut Value) {
    let notes = !data["document"]["footnotes"].as_array().unwrap().is_empty();
    let content = if notes {
        &mut data["document"]["footnotes"][0]["blocks"]
    } else {
        &mut data["document"]["blocks"]
    };
    let mut caption = content[0]["blocks"][0].clone();
    caption["children"] = json!([caption["children"][0].clone()]);
    assert_eq!(caption["children"][0]["kind"], "text");
    let mut span = caption["span"].clone();
    span["start_byte"] = content
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["span"]["start_byte"].as_u64().unwrap())
        .min()
        .unwrap()
        .into();
    span["end_byte"] = content
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["span"]["end_byte"].as_u64().unwrap())
        .max()
        .unwrap()
        .into();
    let br = json!({"kind":"page_break","node_id":0,"span":span,"classes":[]});
    let table = json!({"kind":"table","node_id":0,"span":span,"classes":[],
    "columns":[{"kind":"fraction","weight":1},{"kind":"fraction","weight":2}],
    "caption":[caption,br,caption],"head":[],
    "body":[{"node_id":0,"span":span,"cells":[
        {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[]},
        {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":content.take()}
    ]}]});
    *content = json!([table]);
    for (i, master) in data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        master["width"] = (400 * 65536).into();
        master["height"] = (800 * 65536).into();
        master["trim"] = json!({"x":0,"y":0,"width":400*65536,"height":800*65536});
        let width = [300, 280, 260, 300][i] * 65536;
        master["body"] = json!({"x":10*65536,"y":10*65536,"width":width,"height":300*65536});
        if notes {
            master["footnote"] = json!({"x":20*65536,"y":400*65536,"width":width,"height":350*65536+typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW});
        }
    }
    for rule in data["style_sheet"]["rules"].as_array_mut().unwrap() {
        if rule["style_id"]
            .as_str()
            .unwrap()
            .starts_with("block-width-")
            && rule["selector"] == "paragraph"
        {
            rule["style_id"] = format!(
                "table-block-source-{}",
                rule["style_id"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("block-width-")
                    .unwrap()
            )
            .into();
        }
    }
    fn number(v: &mut Value, next: &mut u32) {
        if let Some(a) = v.as_array_mut() {
            for v in a {
                number(v, next);
            }
        } else if v.is_object() {
            if v.get("node_id").is_some() {
                v["node_id"] = (*next).into();
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
                "equation_number",
                "footnotes",
            ] {
                if let Some(v) = v.get_mut(key) {
                    number(v, next);
                }
            }
        }
    }
    number(&mut data["document"], &mut 0);
}

fn check_block_source_profiles(font: Option<&[u8]>) {
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        if font.is_some() && kind == "native" {
            continue;
        }
        for notes in [false, true] {
            let root = Root::new();
            let limits = driver_limits();
            let mut data = block_width_data(kind, notes);
            wrap_source_table(&mut data);

            let input = if let Some(font) = font {
                data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
                data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .into();
                let source = data["text_buffers"][0]["utf8"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .to_vec();
                let admitted = body_with_source(&root, data, &source, &limits);
                fs::write(root.0.join("body.bin"), font).unwrap();
                fs::write(
                    root.0.join(if kind == "vector" {
                        "vector.svg"
                    } else {
                        "figure.bin"
                    }),
                    match kind {
                        "vector" | "svg" => SVG,
                        "png" => PNG,
                        "jpeg" => JPEG,
                        _ => unreachable!(),
                    },
                )
                .unwrap();
                prepare_book_v2_resources(
                    admitted,
                    &root.context(),
                    &config(limits.base().get().clone()),
                    &limits,
                )
                .unwrap()
            } else {
                match kind {
                    "vector" => vector_input(&root, data, &limits),
                    "native" => {
                        let source = data["text_buffers"][0]["utf8"]
                            .as_str()
                            .unwrap()
                            .as_bytes()
                            .to_vec();
                        native_input_with_source(&root, data, &source, &limits)
                    }
                    "png" => figure_input(&root, data, PNG, &limits),
                    "jpeg" => figure_input(&root, data, JPEG, &limits),
                    "svg" => figure_input(&root, data, SVG, &limits),
                    _ => unreachable!(),
                }
            };
            let run = |maximum| {
                with_converged_book_v2_pdf(
                    &input,
                    &limits,
                    JapaneseLineBreakMode::Normal,
                    maximum,
                    |pdf, observed| {
                        assert!(observed.width_feedback_passes() >= 2);
                        let closed = pdf
                            .navigation()
                            .source()
                            .source()
                            .source()
                            .source()
                            .display()
                            .source()
                            .source();
                        let frames = closed.flow().lines().frames().unwrap();
                        assert!(frames.uses_table_occurrence_frames());
                        assert!(frames.has_block_source_starts());
                        assert!(closed.geometry().pages().len() >= 2);
                        crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                        if let Some(directory) =
                            std::env::var_os("TYPAXIS_BOOK_V2_TABLE_BLOCK_SOURCE_PROBE")
                        {
                            let directory = std::path::PathBuf::from(directory);
                            fs::create_dir_all(&directory).unwrap();
                            let mode = if notes { "notes" } else { "body" };
                            let family = if font.is_some() {
                                "harano"
                            } else {
                                "controlled"
                            };
                            fs::write(
                                directory.join(format!("{family}-{kind}-{mode}.pdf")),
                                pdf.bytes(),
                            )
                            .unwrap();
                            fs::write(directory.join(format!("{family}-{kind}-{mode}.json")),serde_json::to_vec_pretty(&json!({"pages":closed.geometry().pages().len(),"width_passes":observed.width_feedback_passes(),"work":observed.work_steps(),"line_passes":observed.line_reshape_passes(),"page_passes":observed.page_passes()})).unwrap()).unwrap();
                        }
                        eprintln!("table block source: {kind}/{notes}/harano={},pages={},widths={},work={}",font.is_some(),closed.geometry().pages().len(),observed.width_feedback_passes(),observed.work_steps());
                        (observed, typaxis_core::sha256(pdf.bytes()))
                    },
                )
            };
            let full =
                run(100_000_000).unwrap_or_else(|e| panic!("kind={kind} notes={notes}: {e:?}"));
            assert_eq!(run(full.0.work_steps()).unwrap(), full);
            assert!(run(full.0.work_steps() - 1).is_err());
        }
    }
}

#[test]
fn book_v2_table_block_source_profiles_converge_physical_pdfs() {
    check_block_source_profiles(None);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_block_source_profiles_converge_original_harano_pdfs() {
    check_block_source_profiles(Some(
        &fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap(),
    ));
}

#[test]
fn book_v2_block_source_starts_validate_and_finalize_exact_physical_origins() {
    use typaxis_core::Length;
    use typaxis_layout::book_v2::{
        bind_book_v2_vectors, layout_book_v2_source_width_lines_from_flow,
        prepare_book_v2_vector_blocks, with_converged_book_v2_body_lines_with_source_widths,
        BookV2SourceWidthAssignments,
    };
    use typaxis_shaping::book_v2::shape_book_v2_equation_numbers;
    use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};
    for notes in [false, true] {
        let root = Root::new();
        let limits = driver_limits();
        let mut data = block_width_data("vector", notes);
        wrap_source_table(&mut data);
        for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
            master["body"]["width"] = (300 * 65536).into();
            if notes {
                master["footnote"]["width"] = (300 * 65536).into();
            }
        }
        let input = vector_input(&root, data, &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let foreign = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_for_reflow(
            &flow, &mut 0, 1_000_000, 0, 0,
        )
        .unwrap();
        assert!(!plan.requires_width_reflow());
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let profiles = vec![None; flow.paragraphs().len()];
        let origins = vec![None; flow.paragraphs().len()];
        with_converged_book_v2_body_lines_with_source_widths(&policy, &flow, input.resources(), &bindings, &limits, JapaneseLineBreakMode::Normal, plan.measurement_body(), 100_000_000, None, limits.base().get().max_line_reshape_passes, Some(&plan), None, |initial| {
            let frames = initial.lines().frames().unwrap();
            let widths = flow.events().iter().filter_map(|event| match *event {
                Event::Begin {owner, kind: Region::VectorFigure | Region::MathVectorBlock} => Some((owner, frames.region(owner).unwrap().width())),
                _ => None,
            }).collect::<Vec<_>>();
            let starts = widths.iter().map(|&(owner,_)|frames.region(owner).unwrap().start()).collect::<Vec<_>>();
            let run = |a: &BookV2SourceWidthAssignments<'_, '_>, maximum, valid| {
                with_converged_book_v2_body_lines_with_source_widths(&policy, &flow, input.resources(), &bindings, &limits, JapaneseLineBreakMode::Normal, plan.measurement_body(), maximum, None, limits.base().get().max_line_reshape_passes, Some(&plan), Some(a), |stable| {
                    let lines = stable.lines(); let rebound = lines.frames().unwrap();
                    assert!(rebound.has_block_source_starts());
                    let numbers = shape_book_v2_equation_numbers(lines.prepared().shaped(), &limits, 0).unwrap();
                    let blocks = prepare_book_v2_vector_blocks(lines, numbers.as_ref(), &limits, 0).unwrap().unwrap();
                    for &(owner, width) in &widths {
                        assert_eq!(rebound.region(owner).unwrap().width(), width);
                        assert_eq!(rebound.measurement_region(owner), frames.region(owner));
                    }
                    let envelopes=lines.paragraphs().iter().map(|p|p.inline_size()).collect::<Vec<_>>();
                    assert!(layout_book_v2_source_width_lines_from_flow(lines.prepared(), &envelopes, a, maximum, 0).is_err());
                    let body=prepare_book_v2_body_flow(lines,Some(&blocks),stable.footnotes(),&limits,0).unwrap();
                    let measured=prepare_book_v2_table_measurements(body,&limits).unwrap();
                    if valid && maximum==100_000_000 && !rebound.uses_table_occurrence_frames() {
                        let profiles=|work,prior|->Result<_,typaxis_pagination::ProductionBodyPaginationError>{
                            let mut search=prepare_book_v2_table_body_search(&measured,&limits,work,prior)?;
                            let pages=search.select_stable_mixed_pages(limits.base().get().max_layout_passes)?;
                            let placed=search.place_mixed_pages(pages.sequence())?;
                            let closed=search.close_mixed_page_sources(&pages,&placed)?;
                            let feedback=search.paragraph_frame_feedback(&closed)?;
                            assert_eq!(feedback.block_widths(),widths);
                            assert_eq!(feedback.block_starts(),Some(starts.as_slice()));
                            assert!(feedback.matches_selected_block_widths());
                            Ok((feedback.work_steps(),feedback.record_charge(),feedback.assignment_fingerprint()))
                        };
                        let prior=measured.record_charge();let full=profiles(10_000_000,prior).unwrap();
                        assert_eq!(profiles(full.0,prior).unwrap(),full);assert!(profiles(full.0-1,prior).is_err());
                        let exact_prior=limits.base().get().max_fragments-full.1+prior;
                        let exact=profiles(full.0,exact_prior).unwrap();assert_eq!(exact.1,limits.base().get().max_fragments);assert_eq!(exact.2,full.2);
                        assert!(profiles(full.0,exact_prior+1).is_err());
                        eprintln!("block source feedback: notes={notes},work={},records={}",full.0,full.1);
                    }
                    let mut search=prepare_book_v2_table_body_search(&measured,&limits,10_000_000,measured.record_charge()).unwrap();
                    let pages=search.select_stable_mixed_pages(limits.base().get().max_layout_passes).unwrap();
                    let placed=search.place_mixed_pages(pages.sequence()).unwrap();
                    let closed=search.close_mixed_page_sources(&pages,&placed).unwrap();
                    let result=search.finalize_mixed_page_math(closed,&limits,0);
                    if valid { assert!(result.is_ok(),"{:?}",result.err()); }
                    else {assert!(matches!(result,Err(e) if e.kind==typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch));}
                    (stable.candidate_steps(),lines.fingerprint(),blocks.fingerprint())
                })
            };
            for occurrence in [false,true] {
                for tamper in [false,true] {
                    let mut copied=starts.clone();
                    if tamper {copied[0]=copied[0].checked_sub(Length::from_raw(1).unwrap()).unwrap();}
                    let a=BookV2SourceWidthAssignments::new(&flow,&profiles).unwrap().with_block_widths(&widths).with_block_starts(&copied);
                    let a=if occurrence {a.with_source_unit_starts(&origins).unwrap().with_table_occurrence_frames()}else{a};
                    let full=run(&a,100_000_000,!tamper).unwrap();
                    assert_eq!(run(&a,full.0,!tamper).unwrap(),full);
                    assert!(run(&a,full.0-1,!tamper).is_err());
                    eprintln!("block source origins: notes={notes},occurrence={occurrence},tamper={tamper},work={}",full.0);
                }
            }
            let a=BookV2SourceWidthAssignments::new(&foreign,&profiles).unwrap().with_block_widths(&widths).with_block_starts(&starts);
            assert!(run(&a,100_000_000,true).is_err());
            for mode in ["short","long","before","after","inherit","missing-width","reversed-width"] {
                let mut copied=starts.clone(); let mut w=widths.clone();
                match mode {
                    "short"=>{copied.pop();},
                    "long"=>copied.push(copied[0]),
                    "before"=>copied[0]=Length::from_raw(-1).unwrap(),
                    "after"=>copied[0]=Length::from_raw(1_000_000_000).unwrap(),
                    "missing-width"=>w.clear(),
                    "reversed-width"=>w.reverse(),
                    _=>{},
                }
                let a=BookV2SourceWidthAssignments::new(&flow,&profiles).unwrap().with_block_widths(&w).with_block_starts(&copied);
                let a=if mode=="inherit"{a.with_inherited_table_blocks()}else{a};
                assert!(run(&a,100_000_000,true).is_err(),"{mode}");
            }
        }).unwrap();
    }
}
