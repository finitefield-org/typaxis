use super::*;
use typaxis_core::NodeId;
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, with_converged_book_v2_body_lines_with_source_widths,
};
use typaxis_pagination::book_v2::BookV2TableWidthSource;
use typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_for_reflow;

pub(super) fn occurrence_data(notes: bool, mode: &str) -> Value {
    let mut data = varying_table_frames_data(notes);
    let table = if notes {
        &mut data["document"]["footnotes"][0]["blocks"][0]
    } else {
        &mut data["document"]["blocks"][0]
    };
    let padding =
        (notes && mode == "empty").then(|| table["body"][0]["cells"][0]["blocks"][0].clone());
    if mode == "empty" {
        for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
            cell["blocks"] = json!([]);
        }
    }
    if mode == "header" {
        table["head"] = json!([table["body"][0].clone()]);
        for cell in table["head"][0]["cells"].as_array_mut().unwrap() {
            cell["blocks"].as_array_mut().unwrap().truncate(1);
        }
    }
    if mode == "forced" {
        let p = table["body"][0]["cells"][0]["blocks"][0].clone();
        let br = json!({"kind":"page_break","node_id":0,"span":p["span"],"classes":[]});
        table["caption"] = json!([p, br, p]);
    }
    if let Some(padding) = padding {
        data["document"]["footnotes"][0]["blocks"]
            .as_array_mut()
            .unwrap()
            .push(padding);
    }
    for (i, master) in data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let w = [220, 180, 160, 200][i] * 65536;
        master["body"]["width"] = w.into();
        if notes {
            master["footnote"]["width"] = w.into();
        }
        if mode == "header" {
            master["body"]["height"] = (48 * 65536).into();
            if notes {
                master["footnote"]["height"] =
                    (48 * 65536 + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW).into();
            }
        }
    }
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
    data
}

#[test]
fn book_v2_table_width_occurrences_retain_original_units_repeats_and_breaks() {
    check_occurrences(None, false, false);
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_width_occurrences_retain_original_harano_units() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_occurrences(Some(&font), false, false);
}

#[test]
fn book_v2_table_occurrence_frames_follow_original_cells_on_each_page() {
    check_occurrences(None, true, false);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_occurrence_frames_follow_original_harano_cells() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_occurrences(Some(&font), true, false);
}

#[test]
fn book_v2_source_unit_starts_rebind_table_occurrences_through_shaping() {
    check_occurrences(None, true, true);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_source_unit_starts_rebind_original_harano_occurrences() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_occurrences(Some(&font), true, true);
}
fn check_occurrences(font: Option<&[u8]>, remeasure: bool, origins: bool) {
    for notes in [false, true] {
        for mode in ["natural", "header", "forced", "split", "empty"] {
            if origins && mode != "split" {
                continue;
            }
            let root = Root::new();
            let limits = driver_limits();
            let text = match (font.is_some(), mode == "split") {
                (true, true) => "本文を続けて組み直す本文を続けて組み直す",
                (true, false) => "本文の段落",
                (false, true) => "Pro Pro Pro Pro Pro Pro Pro",
                (false, false) => "Result",
            };
            let unit_count = text.chars().count() as u32;
            let mut data = occurrence_data(notes, mode);
            if text != "Result" {
                fn expand(v: &mut Value, end: usize) {
                    if let Some(values) = v.as_array_mut() {
                        for v in values {
                            expand(v, end);
                        }
                    } else if let Some(object) = v.as_object_mut() {
                        for (key, value) in object.iter_mut() {
                            if key == "end_byte" && value.as_u64() == Some(6) {
                                *value = end.into();
                            } else {
                                expand(value, end);
                            }
                        }
                    }
                }
                expand(&mut data, text.len());
                data["text_buffers"][0]["utf8"] = text.into();
            }
            if origins {
                let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
                for (class, align) in [
                    ("left", if font.is_some() { "start" } else { "center" }),
                    ("right", "end"),
                ] {
                    rules.push(json!({"style_id":format!("origins-{class}"),"selector":format!("paragraph.{class}"),"source_order":rules.len(),"extends":null,
                        "declarations":[{"name":"text_align","important":false,"value":{"kind":"keyword","value":align}}]}));
                }
            }
            // Expected positions come from the original two equal-fraction
            // columns, not from a second call to the production frame resolver.
            let table = if notes {
                &data["document"]["footnotes"][0]["blocks"][0]
            } else {
                &data["document"]["blocks"][0]
            };
            assert_eq!(
                table["columns"],
                json!([{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}])
            );
            let mut positions = std::collections::BTreeMap::new();
            for group in ["head", "body"] {
                for row in table[group].as_array().unwrap() {
                    for (column, cell) in row["cells"].as_array().unwrap().iter().enumerate() {
                        for p in cell["blocks"].as_array().unwrap() {
                            positions.insert(
                                NodeId::new(p["node_id"].as_u64().unwrap() as u32),
                                Some(column),
                            );
                        }
                    }
                }
            }
            if let Some(caption) = table.get("caption") {
                for p in caption.as_array().unwrap() {
                    positions.insert(NodeId::new(p["node_id"].as_u64().unwrap() as u32), None);
                }
            }
            let input = if let Some(bytes) = font {
                data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
                data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .into();
                let body = body_with_source(&root, data, text.as_bytes(), &limits);
                fs::write(root.0.join("body.bin"), bytes).unwrap();
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
            let plan =
                prepare_book_v2_page_frame_plan_for_reflow(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
            let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
            let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
            with_converged_book_v2_body_lines_with_source_widths(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,plan.measurement_body(),100_000_000,None,limits.base().get().max_line_reshape_passes,Some(&plan),None,|stable| {
            let lines=stable.lines();
            let body=prepare_book_v2_body_flow(lines,None,stable.footnotes(),&limits,0).unwrap();
            let measured=prepare_book_v2_table_measurements(body,&limits).unwrap();
            let run=|maximum,prior|->Result<_,typaxis_pagination::ProductionBodyPaginationError>{
                let mut search=prepare_book_v2_table_body_search(&measured,&limits,maximum,prior)?;
                let pages=search.select_stable_mixed_pages(limits.base().get().max_layout_passes)?;
                let placed=search.place_mixed_pages(pages.sequence())?;
                let closed=search.close_mixed_page_sources(&pages,&placed)?;
                let report=if remeasure {search.table_width_frames(&closed)?} else {search.table_width_occurrences(&closed)?};
                assert_eq!(report.has_remeasured_frames(),remeasure);
                assert!(report.matches_source_flow(&flow));
                if mode=="empty" { assert_eq!(report.occurrences().len(),1); } else { assert!(report.occurrences().len()>=2,"{mode}/{notes}"); }
                let frames=lines.frames().unwrap();
                let mut seen=std::collections::BTreeMap::<NodeId,Vec<u32>>::new();
                let mut repeated_ranges=Vec::new();let mut breaks=0;let mut widths=std::collections::BTreeSet::new();
                for occurrence in report.occurrences() {
                    assert_eq!(occurrence.definition_index().is_some(),notes);
                    let page=placed.pages().iter().find(|p|p.selection().page_index()==occurrence.page_index()).unwrap();
                    let actual=if notes {page.selection().declared_footnote_region().unwrap()}else{page.selection().body_bounds()};
                    let original=frames.measurement_region(occurrence.owner()).unwrap();
                    let root_width=if notes {frames.footnote_region().unwrap().width()}else{frames.body().width()};
                    assert_eq!(occurrence.parent_width().get(),original.width().get().checked_add(actual.width().get().checked_sub(root_width.get()).unwrap()).unwrap());
                    widths.insert(occurrence.parent_width().get().raw());
                    for piece in occurrence.pieces() {
                        if remeasure && matches!(piece.source(),BookV2TableWidthSource::Paragraph{..}) {
                            let w=occurrence.parent_width().get().raw();
                            let half=w/2;
                            let left=half+i64::from(w%2!=0 && half%2!=0);
                            let (offset,width)=match positions[&piece.source().owner()] {None=>(0,w),Some(0)=>(0,left),Some(1)=>(left,w-left),_=>unreachable!()};
                            let frame=piece.frame().unwrap();
                            assert_eq!(frame.start().raw(),original.start().raw()+offset);
                            assert_eq!(frame.width().get().raw(),width);
                        } else { assert!(piece.frame().is_none()); }
                        if piece.repeated(){
                            match piece.source() {
                                BookV2TableWidthSource::Paragraph{owner,units}=>repeated_ranges.push((*owner,units.clone())),
                                _=>panic!("text header fixture"),
                            }
                            continue;
                        }
                        match piece.source() {
                            BookV2TableWidthSource::Paragraph{owner,units}=>{assert!(units.start<units.end && units.end<=unit_count);seen.entry(*owner).or_default().extend(units.clone());}
                            BookV2TableWidthSource::ForcedBreak{..}=>{breaks+=1;}
                            BookV2TableWidthSource::Block{..}=>panic!("text fixture")
                        }
                    }
                }
                if mode=="empty" { assert_eq!(widths.len(),1);assert!(report.occurrences()[0].pieces().is_empty()); } else { assert!(widths.len()>1); }
                for (owner,units) in &repeated_ranges {
                    assert!(units.start<units.end);
                    assert!(units.clone().all(|unit|seen.get(owner).unwrap().contains(&unit)));
                }
                assert_eq!(!repeated_ranges.is_empty(),mode=="header");assert_eq!(breaks>0,mode=="forced");
                assert!(seen.values().all(|units|units==&(0..unit_count).collect::<Vec<_>>()));
                let expected=flow.paragraphs().len()-usize::from(notes)-usize::from(notes&&mode=="empty");
                assert_eq!(seen.len(),expected);
                if origins && maximum==10_000_000 && prior==measured.record_charge() {
                    use typaxis_layout::book_v2::BookV2SourceWidthAssignments;
                    let mut targets=vec![None;flow.paragraphs().len()];
                    for occurrence in report.occurrences() {
                        for piece in occurrence.pieces().iter().filter(|p|!p.repeated()) {
                            if let BookV2TableWidthSource::Paragraph{owner,units}=piece.source() {
                                let i=flow.paragraphs().iter().position(|p|p.owner()==*owner).unwrap();
                                let target=targets[i].get_or_insert_with(||vec![None;lines.prepared().paragraphs()[i].items().unwrap().units().len().max(1)]);
                                for unit in units.clone() { assert!(target[unit as usize].replace(piece.frame().unwrap()).is_none()); }
                            }
                        }
                    }
                    let source_widths=targets.iter().map(|p|p.as_ref().map(|p|p.iter().map(|f|f.unwrap().width()).collect::<Vec<_>>())).collect::<Vec<_>>();
                    let source_starts=targets.iter().map(|p|p.as_ref().map(|p|p.iter().map(|f|f.unwrap().start()).collect::<Vec<_>>())).collect::<Vec<_>>();
                    let width_views=source_widths.iter().map(|p|p.as_deref()).collect::<Vec<_>>();
                    let start_views=source_starts.iter().map(|p|p.as_deref()).collect::<Vec<_>>();
                    let assignments=BookV2SourceWidthAssignments::new(&flow,&width_views).unwrap().with_source_unit_starts(&start_views).unwrap();
                    let reshape=|budget|with_converged_book_v2_body_lines_with_source_widths(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,plan.measurement_body(),budget,None,limits.base().get().max_line_reshape_passes,Some(&plan),Some(&assignments),|next| {
                        let fresh=next.lines();let new_frames=fresh.frames().unwrap();assert!(new_frames.has_source_unit_starts());
                        assert_eq!(new_frames.record_charge(),frames.record_charge()+source_starts.len() as u64+1+source_starts.iter().flatten().map(|s|s.len() as u64).sum::<u64>());
                        let new_body=prepare_book_v2_body_flow(fresh,None,next.footnotes(),&limits,0).unwrap();
                        let items=if notes {new_body.definition_items(0).unwrap()} else {new_body.body_items()};
                        let mut moved=0;let mut changed=0;let mut narrowed=0;let mut expected_positions=std::collections::BTreeMap::new();
                        for (p,old) in fresh.paragraphs().iter().zip(lines.paragraphs()) { changed+=usize::from(p.lines().len()!=old.lines().len()); }
                        for item in items {
                            let Some(typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine{paragraph_index,line_index})=item.source() else {continue};
                            let i=paragraph_index as usize;
                            let Some(starts)=&source_starts[i] else {continue};
                            let p=&fresh.paragraphs()[i];let selected=&p.selected().unwrap().lines()[line_index as usize];let unit=selected.line().start_unit() as usize;
                            assert_eq!(selected.inline_size(),source_widths[i].as_ref().unwrap()[unit]);
                            narrowed+=usize::from(selected.inline_size()!=lines.paragraphs()[i].inline_size());
                            assert_eq!(new_frames.source_unit_start(i,unit as u32),Some(starts[unit]));
                            let slack=selected.inline_size().get().raw()-selected.required_inline_size().get().raw();
                            let offset=match positions[&item.owner()] {Some(1)=>slack,Some(0) if font.is_none()=>slack/2,_=>0};
                            assert_eq!(item.x().raw(),plan.measurement_body().x().raw()+starts[unit].raw()+offset);
                            expected_positions.insert((paragraph_index,line_index),item.x().raw());
                            moved+=usize::from(starts[unit]!=frames.paragraphs()[i].start());
                        }
                        for (p, source) in fresh.paragraphs().iter().zip(flow.paragraphs()) {
                            assert_eq!(p.owner(),source.owner());
                            for line in p.lines() { for atom in line.items() { if let typaxis_layout::ProductionPlacedInline::Text(c)=atom { for glyph in c.glyphs() {
                                assert!(std::ptr::eq(glyph.glyph(),&c.run().glyph_run().glyphs[glyph.glyph_index() as usize]));
                            } } } }
                        }
                        assert!(moved>0);assert!(narrowed>0);if font.is_none(){assert!(changed>0);}
                        let measured=prepare_book_v2_table_measurements(new_body,&limits).unwrap();
                        let mut search=prepare_book_v2_table_body_search(&measured,&limits,10_000_000,measured.record_charge()).unwrap();
                        let pages=search.select_stable_mixed_pages(limits.base().get().max_layout_passes).unwrap();
                        let placed=search.place_mixed_pages(pages.sequence()).unwrap();
                        let closed=search.close_mixed_page_sources(&pages,&placed).unwrap();
                        let mut placed_count=0;
                        for page in closed.geometry().pages() {
                            for placed in page.fragments() {
                                let fragment=placed.fragment();
                                let typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine{paragraph_index,line_index}=fragment.source() else {continue};
                                if let Some(&expected)=expected_positions.get(&(paragraph_index,line_index)) {
                                    let delta=if notes {page.selection().declared_footnote_region().unwrap().x().raw()-new_frames.footnote_region().unwrap().x().raw()} else {page.selection().body_bounds().x().raw()-new_frames.body().x().raw()};
                                    assert_eq!(fragment.bounds().x().raw(),expected+delta);placed_count+=1;
                                }
                            }
                        }
                        assert_eq!(placed_count,expected_positions.len());
                        assert!(matches!(search.finalize_mixed_page_math(closed,&limits,0),Err(e) if matches!(e.kind,typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch|typaxis_pagination::ProductionBodyPaginationErrorKind::PendingRegion("table_continuation_width_reflow"))));
                        (next.candidate_steps(),fresh.fingerprint(),moved,changed,narrowed)
                    });
                    let full=reshape(100_000_000).unwrap();assert_eq!(reshape(full.0).unwrap(),full);assert!(reshape(full.0-1).is_err());
                    assert!(typaxis_layout::book_v2::layout_book_v2_source_width_lines_from_flow(lines.prepared(),&frames.paragraphs().iter().map(|f|f.width()).collect::<Vec<_>>(),&assignments,100_000_000,0).is_err());
                    eprintln!("source origins: notes={notes},harano={},work={},moved={},changed={},narrowed={}",font.is_some(),full.0,full.2,full.3,full.4);
                }
                let work=report.work_steps();let records=report.record_charge();
                // A report from another issuing search cannot consume this
                // closure, even though both searches have identical source.
                let mut other=prepare_book_v2_table_body_search(&measured,&limits,10_000_000,measured.record_charge()).unwrap();
                assert!(other.table_width_occurrences(&closed).is_err());
                assert!(other.table_width_frames(&closed).is_err());
                if mode!="empty" && maximum>work && prior==measured.record_charge() {
                    assert!(matches!(search.finalize_mixed_page_math(closed,&limits,0),Err(e) if e.kind==typaxis_pagination::ProductionBodyPaginationErrorKind::PendingRegion("table_continuation_width_reflow")));
                }
                Ok((work,records,report.fingerprint(),report.occurrences().len()))
            };
            let prior=measured.record_charge();let full=run(10_000_000,prior).unwrap();
            assert_eq!(run(full.0,prior).unwrap(),full);assert!(run(full.0-1,prior).is_err());
            let exact_prior=limits.base().get().max_fragments-full.1+prior;
            let exact=run(full.0,exact_prior).unwrap();assert_eq!(exact.1,limits.base().get().max_fragments);assert_eq!(exact.2,full.2);
            assert!(run(full.0,exact_prior+1).is_err());
            eprintln!("frames={remeasure} {mode}/{notes}: work={} records={} occurrences={}",full.0,full.1,full.3);
            if remeasure && font.is_none() && !origins {
                let profiles=|maximum,prior|->Result<_,typaxis_pagination::ProductionBodyPaginationError>{
                    let mut search=prepare_book_v2_table_body_search(&measured,&limits,maximum,prior)?;
                    let pages=search.select_stable_mixed_pages(limits.base().get().max_layout_passes)?;
                    let placed=search.place_mixed_pages(pages.sequence())?;
                    let closed=search.close_mixed_page_sources(&pages,&placed)?;
                    let feedback=search.paragraph_frame_feedback(&closed)?;
                    assert!(feedback.uses_table_occurrence_frames());assert!(feedback.root_table_widths().is_empty());assert!(feedback.matches_source_flow(&flow));
                    for (p,source) in feedback.paragraphs().iter().zip(flow.paragraphs()) {
                        assert_eq!(p.owner(),source.owner());assert!(!p.uses_table_frame());
                        if let Some(starts)=p.source_unit_starts(){assert_eq!(starts.len(),p.widths().len());}
                    }
                    let mut foreign=prepare_book_v2_table_body_search(&measured,&limits,10_000_000,measured.record_charge()).unwrap();
                    assert!(foreign.paragraph_frame_feedback(&closed).is_err());
                    Ok((feedback.work_steps(),feedback.record_charge(),feedback.assignment_fingerprint()))
                };
                if mode=="header" {
                    assert!(matches!(profiles(10_000_000,prior),Err(e) if e.kind==typaxis_pagination::ProductionBodyPaginationErrorKind::PendingRegion("table_repeated_frame_reflow")));
                } else {
                    let full=profiles(10_000_000,prior).unwrap();assert_eq!(profiles(full.0,prior).unwrap(),full);assert!(profiles(full.0-1,prior).is_err());
                    let exact_prior=limits.base().get().max_fragments-full.1+prior;
                    let exact=profiles(full.0,exact_prior).unwrap();assert_eq!(exact.1,limits.base().get().max_fragments);assert_eq!(exact.2,full.2);assert!(profiles(full.0,exact_prior+1).is_err());
                    eprintln!("table profiles: {mode}/{notes},work={},records={}",full.0,full.1);
                }
            }

        }).unwrap();
        }
    }
}
