use super::*;
use std::collections::BTreeSet;
use typaxis_display_list::book_v2::{BookV2BodyDisplay, BookV2FontUseText};
#[path = "book_v2_header_resource_tests.rs"]
mod nested_resources;
use typaxis_layout::book_v2::with_rebuilt_book_v2_body_line_variants;
use typaxis_pagination::book_v2::{
    prepare_book_v2_table_body_search, prepare_book_v2_table_header_catalog,
    prepare_book_v2_table_header_variant,
};

pub(super) fn check(font: Option<&[u8]>, placement: bool) {
    check_base(font, placement, false);
    if placement {
        check_base(font, placement, true);
    }
}
fn check_base(font: Option<&[u8]>, placement: bool, wide_base: bool) {
    for mode in ["common", "nested-header"] {
        for notes in [false, true] {
            let root = Root::new();
            let original_limits = driver_limits();
            let mut caps = original_limits.base().get().clone();
            caps.max_page_break_lookback = 256;
            let limits = M4EffectiveResourceLimits::new(
                typaxis_core::ValidatedResourceLimits::new(caps).unwrap(),
                original_limits.extension().get().clone(),
            )
            .unwrap();
            let text = if font.is_some() {
                "本文を続けて組み直す本文を続けて組み直す"
            } else {
                "Pro Pro Pro Pro Pro Pro Pro"
            };
            let mut data = super::header_selection::fixture(notes, mode, text);
            if wide_base {
                let table = if notes {
                    &mut data["document"]["footnotes"][0]["blocks"][0]
                } else {
                    &mut data["document"]["blocks"][0]
                };
                for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
                    cell["blocks"] = vec![
                        cell["blocks"][0].clone();
                        if mode == "nested-header" { 64 } else { 24 }
                    ]
                    .into();
                }
                crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
                    &mut data["document"],
                    &mut 0,
                );
            }
            if placement && mode == "common" {
                let table = if notes {
                    &mut data["document"]["footnotes"][0]["blocks"][0]
                } else {
                    &mut data["document"]["blocks"][0]
                };
                let paragraph = table["head"][0]["cells"][0]["blocks"][0].clone();
                table["head"][0]["cells"][0]["blocks"][0] = json!({"kind":"list","node_id":0,"span":paragraph["span"],"classes":[],"ordered":true,"start":1,"items":[{"node_id":0,"span":paragraph["span"],"blocks":[paragraph]}]});
                let mut rule = data["style_sheet"]["rules"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["selector"] == "paragraph")
                    .unwrap()
                    .clone();
                rule["selector"] = "list".into();
                rule["style_id"] = "header-list".into();
                rule["source_order"] =
                    (data["style_sheet"]["rules"].as_array().unwrap().len() as u64).into();
                data["style_sheet"]["rules"]
                    .as_array_mut()
                    .unwrap()
                    .push(rule);
                crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
                    &mut data["document"],
                    &mut 0,
                );
            }
            for (index, master) in data["page_masters"]["masters"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
            {
                let width = if index == 0 || (index == 2 && !wide_base) {
                    140
                } else {
                    220
                };
                let height = if mode == "nested-header" {
                    900
                } else if placement {
                    300
                } else {
                    200
                };
                if placement {
                    master["width"] = (600 * 65536).into();
                    master["trim"]["width"] = (600 * 65536).into();
                    master["body"]["x"] = ((20 + index as i64 * 7) * 65536).into();
                }
                master["body"]["width"] = (width * 65536).into();
                master["body"]["height"] = (height * 65536).into();
                if notes {
                    if placement {
                        master["footnote"]["x"] = ((30 + index as i64 * 11) * 65536).into();
                    }
                    master["footnote"]["width"] = (width * 65536).into();
                    master["footnote"]["height"] = (height * 65536).into();
                }
            }
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
                    assignment,
                )
                .unwrap()
            };
            let (original_parent, mismatched_widths) = {
                let seed = make(None);
                with_rebuilt_book_v2_body_line_variant(&seed, 10_000_000, 0, |v| {
                    let widths = v
                        .lines()
                        .prepared()
                        .paragraphs()
                        .iter()
                        .zip(v.lines().frames().unwrap().paragraphs())
                        .map(|(p, f)| {
                            let width = PositiveLength::new(
                                f.width()
                                    .get()
                                    .checked_sub(Length::from_raw(1).unwrap())
                                    .unwrap(),
                            )
                            .unwrap();
                            vec![width; p.items().unwrap().units().len().max(1)]
                        })
                        .collect::<Vec<_>>();
                    (
                        v.lines()
                            .frames()
                            .unwrap()
                            .measurement_region(flow.tables()[0].owner())
                            .unwrap()
                            .width(),
                        widths,
                    )
                })
                .unwrap()
            };
            let physical_narrow =
                PositiveLength::new(Length::from_raw(140 * 65536).unwrap()).unwrap();
            let physical_wide =
                PositiveLength::new(Length::from_raw(220 * 65536).unwrap()).unwrap();
            let narrow_parent = PositiveLength::new(
                original_parent
                    .get()
                    .checked_sub(Length::from_raw(80 * 65536).unwrap())
                    .unwrap(),
            )
            .unwrap();
            let a = [(flow.tables()[0].owner(), narrow_parent)];
            let b = [(flow.tables()[0].owner(), original_parent)];
            let profiles = vec![None; flow.paragraphs().len()];
            let narrow = BookV2SourceWidthAssignments::new(&flow, &profiles)
                .unwrap()
                .with_root_table_widths(&a);
            let wide = BookV2SourceWidthAssignments::new(&flow, &profiles)
                .unwrap()
                .with_root_table_widths(&b);
            let mismatched_views = mismatched_widths
                .iter()
                .map(|w| Some(w.as_slice()))
                .collect::<Vec<_>>();
            let mismatch = BookV2SourceWidthAssignments::new(&flow, &mismatched_views)
                .unwrap()
                .with_root_table_widths(&b);
            let bad_seed = make(Some(&mismatch));
            let first = make(Some(&narrow));
            let second = make(Some(&wide));
            if !placement {
                check_automatic_catalog(if wide_base { &second } else { &first }, &limits, physical_narrow, physical_wide);
            }
            with_rebuilt_book_v2_body_line_variants(&[&first,&second,&bad_seed],10_000_000,0,|set| {
                let measured=set.variants().iter().map(|v|prepare_book_v2_table_measurements(prepare_book_v2_body_flow(v.lines(),None,v.footnotes(),&limits,0).unwrap(),&limits).unwrap()).collect::<Vec<_>>();
                let base=&measured[usize::from(wide_base)];
                let h0=prepare_book_v2_table_header_variant(&set,base,&measured[0],0,&limits,1_000_000,0).unwrap();
                let h1=prepare_book_v2_table_header_variant(&set,base,&measured[1],0,&limits,1_000_000,0).unwrap();
                let bad_header=prepare_book_v2_table_header_variant(&set,base,&measured[2],0,&limits,1_000_000,0).unwrap();
                assert!(matches!(prepare_book_v2_table_header_catalog(base,&[&bad_header],&limits,10_000_000,0),Err(e) if e.kind==typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch));
                let entries=[&h0,&h1];
                let build=|work,prior|prepare_book_v2_table_header_catalog(base,&entries,&limits,work,prior);
                let catalog=build(10_000_000,0).unwrap();
                assert_eq!(catalog.len(),2);assert!(!catalog.is_empty());
                assert!(std::ptr::eq(catalog.for_region_width(0,Some(physical_narrow)).unwrap(),&h0));
                assert!(std::ptr::eq(catalog.for_region_width(0,Some(physical_wide)).unwrap(),&h1));
                assert_eq!(build(catalog.work_steps(),0).unwrap().fingerprint(),catalog.fingerprint());
                assert!(build(catalog.work_steps()-1,0).is_err());
                let raised=limits.base().get().max_fragments/2;
                let extra=build(catalog.work_steps(),raised).unwrap().record_charge()-raised;
                let exact=limits.base().get().max_fragments-extra;
                assert_eq!(build(catalog.work_steps(),exact).unwrap().record_charge(),limits.base().get().max_fragments);
                assert!(build(catalog.work_steps(),exact+1).is_err());
                for bad in [vec![&h1,&h0],vec![&h0,&h0]] {assert!(prepare_book_v2_table_header_catalog(base,&bad,&limits,10_000_000,0).is_err());}
                assert!(prepare_book_v2_table_header_catalog(&measured[usize::from(!wide_base)],&entries,&limits,10_000_000,0).is_err());
                assert!(catalog.for_region_width(0,Some(PositiveLength::new(Length::from_raw(141*65536).unwrap()).unwrap())).is_err());
                let empty=prepare_book_v2_table_header_catalog(base,&[],&limits,1,0).unwrap();
                assert!(empty.is_empty());
                assert_eq!(empty.record_charge(),base.record_charge()+1);
                assert!(prepare_book_v2_table_header_catalog(base,&[],&limits,0,0).is_err());
                for (physical,parent) in [(physical_narrow,narrow_parent),(physical_wide,original_parent)] {
                    let missing=empty.for_region_width(0,Some(physical)).err().unwrap();
                    assert_eq!(missing.owner,flow.tables()[0].owner());
                    assert_eq!(missing.kind,typaxis_pagination::ProductionBodyPaginationErrorKind::TableHeaderWidthRequired{table_index:0,parent_width:parent});
                }
                let mut discovery=prepare_book_v2_table_body_search(base,&limits,100_000_000,0).unwrap();
                discovery.set_table_header_catalog(&empty).unwrap();
                let missing=discovery.select_mixed_pages().err().unwrap();
                assert_eq!(missing.owner,flow.tables()[0].owner());
                assert!(matches!(missing.kind,typaxis_pagination::ProductionBodyPaginationErrorKind::TableHeaderWidthRequired{table_index:0,parent_width} if parent_width==narrow_parent || parent_width==original_parent));
                let mut late=prepare_book_v2_table_body_search(base,&limits,100_000_000,0).unwrap();
                let _begun=late.begin_mixed_pages().unwrap();
                assert!(late.set_table_header_catalog(&catalog).is_err());
                drop(late);
                let mut expected=BTreeSet::new();
                for table in base.tables() {
                    for contents in table.caption().map(|c|c.content()).into_iter().chain(table.cells().iter().map(|c|c.content())) {
                        for content in contents { if let typaxis_pagination::ProductionTableContentSource::FlowItem(i)=content.source() {assert!(expected.insert(i));} }
                    }
                }
                let run=|work,prior|->Result<(u64,u64,Vec<[u8;32]>,usize,usize),typaxis_pagination::ProductionBodyPaginationError>{
                    let mut search=prepare_book_v2_table_body_search(base,&limits,work,prior)?;
                    search.set_table_header_catalog(&catalog)?;
                    let stable=if placement {Some(search.select_stable_mixed_pages(limits.base().get().max_layout_passes)?)}else{None};
                    let ordinary=if stable.is_none(){Some(search.select_mixed_pages()?)}else{None};
                    let sequence=stable.as_ref().map(|s|s.sequence()).unwrap_or_else(||ordinary.as_ref().unwrap());
                    let mut fingerprints=Vec::new();
                    let mut variants=0;
                    let mut coverage=BTreeSet::new();
                    let mut widths=BTreeSet::new();
                    search.verify_mixed_sequence(&sequence)?;
                    for page in sequence.pages() {
                        let mut inspect=|table:&typaxis_pagination::book_v2::BookV2TableFragmentSelection<'_, '_, '_, '_, '_>,width: PositiveLength| {
                            fingerprints.push(table.fingerprint());
                            for range in table.semantic_leaf_ranges() {for i in range{assert!(coverage.insert(i),"duplicate source {i}");}}
                            for range in table.source_leaf_ranges() {assert!(range.is_ok());}
                            if let Some(header)=table.header_variant() {
                                assert!(table.before().has_started_rows());
                                widths.insert(width.get().raw());
                                assert!(std::ptr::eq(header,catalog.for_region_width(table.before().table_index(),Some(width)).unwrap()));
                                assert_eq!(table.header_height(),header.height());
                                let mut count=0;
                                for leaf in table.variant_placement_leaves() {let leaf=leaf.unwrap();if leaf.uses_header_variant(){assert!(std::ptr::eq(leaf.measurements(),header.variant()));count+=1;}}
                                assert_eq!(count,header.leaves().len());
                                assert!(table.source_placement_leaves().next().unwrap().is_err());
                                variants+=1;
                            }
                        };
                        for part in page.candidate().parts() {if let Some(table)=part.table(){inspect(table,page.body_bounds().width());}}
                        if let Some(region)=page.candidate().footnotes(){for fragment in region.fragments(){if let Some(mixed)=fragment.fragment().mixed(){for part in mixed.parts(){if let Some(table)=part.table(){inspect(table,page.declared_footnote_region().unwrap().width());}}}}}
                    }
                    assert!(variants>0,"{mode}/{notes}");
                    assert_eq!(coverage,expected);
                    assert_eq!(widths.len(),2,"both continuation widths: {mode}/{notes}");
                    assert!(search.set_table_header_catalog(&catalog).is_err());
                    if let Some(stable)=&stable {
                        let geometry=search.place_mixed_pages(sequence)?;
                        super::header_placement::verify_geometry(&geometry, base);
                        assert!(stable.passes()>=2);
                        if work==100_000_000 {eprintln!("placed header geometry: wide_base={wide_base},{mode},notes={notes},harano={},passes={},fragments={},header_leaves={},list_markers={}",font.is_some(),stable.passes(),geometry.pages().iter().map(|p|p.fragments().len()).sum::<usize>(),geometry.pages().iter().map(|p|p.header_variants().len()).sum::<usize>(),geometry.pages().iter().map(|p|p.list_markers().len()).sum::<usize>());}
                        let closed=search.close_mixed_page_sources(stable,&geometry)?;
                        assert!(closed.has_header_variants());
                        for (pi,page) in geometry.pages().iter().enumerate(){
                            for fi in 0..page.fragments().len(){assert!(std::ptr::eq(closed.fragment_flow(pi,fi).unwrap(),page.header_variant(fi).map_or(base.flow(),|v|v.measurements().flow())));}
                            assert!(closed.fragment_flow(pi,page.fragments().len()).is_none());
                        }
                        assert!(closed.fragment_flow(geometry.pages().len(),0).is_none());
                        assert_eq!(closed.header_variant_fragments(),geometry.pages().iter().map(|p|p.header_variants().len()).sum::<usize>());
                        let total=geometry.pages().iter().map(|p|p.fragments().len()).sum::<usize>();
                        let copies=geometry.pages().iter().flat_map(|p|p.fragments_with_roles()).filter(|(_,_,r)|*r).count();
                        assert_eq!(closed.repeated_fragments(),copies);
                        assert_eq!(closed.semantic_fragments()+closed.repeated_fragments(),total);
                        assert_eq!(closed.unreferenced_definitions(),0);
                        assert_eq!(closed.semantic_math(),0);assert_eq!(closed.repeated_math(),0);
                        let raw=search.table_width_occurrences(&closed)?;
                        assert!(!raw.has_remeasured_frames());
                        assert!(raw.occurrences().iter().flat_map(|o|o.pieces()).all(|p|p.frame().is_none() && p.measured_header_frame().is_none()));
                        let report=search.table_width_frames(&closed)?;
                        assert!(report.has_remeasured_frames());
                        assert_ne!(raw.fingerprint(),report.fingerprint());
                        assert_eq!(raw.occurrences().len(),report.occurrences().len());
                        for (raw,projected) in raw.occurrences().iter().zip(report.occurrences()) {
                            assert_eq!(raw.owner(),projected.owner());assert_eq!(raw.parent_width(),projected.parent_width());
                            assert_eq!(raw.pieces().len(),projected.pieces().len());
                            for (a,b) in raw.pieces().iter().zip(projected.pieces()) {assert_eq!(a.source(),b.source());assert_eq!(a.repeated(),b.repeated());assert_eq!(a.uses_header_variant(),b.uses_header_variant());}
                        }
                        let variant_pieces=report.occurrences().iter().flat_map(|o|o.pieces()).filter(|p|p.uses_header_variant()).collect::<Vec<_>>();
                        assert_eq!(variant_pieces.len(),closed.header_variant_fragments());
                        for piece in variant_pieces {
                            assert!(piece.repeated());
                            let frame=piece.frame().unwrap();
                            assert_eq!(piece.measured_header_frame(),Some((frame.start(),frame.width())));
                        }
                        let mut feedback=search.paragraph_frame_feedback(&closed)?;
                        assert!(feedback.uses_table_occurrence_frames());
                        assert!(feedback.root_table_widths().is_empty());
                        assert_eq!(feedback.paragraphs().len(),base.flow().lines().paragraphs().len());
                        super::header_placement::verify_width_feedback(&geometry,base,&report,&feedback);
                        search.retain_paragraph_line_boundaries(&closed,&mut feedback)?;
                        for (p,candidate) in base.flow().lines().paragraphs().iter().zip(feedback.paragraphs()) {
                            if !candidate.uses_table_frame() {assert_eq!(candidate.retained_line_ends().unwrap(),p.selected().unwrap().lines().iter().map(|l|l.line().end_unit()).collect::<Vec<_>>());}
                        }
                        if work==100_000_000 {eprintln!("header width feedback: wide_base={wide_base},{mode},notes={notes},harano={},pieces={},matches={},work={},records={}",font.is_some(),report.occurrences().iter().map(|o|o.pieces().len()).sum::<usize>(),feedback.matches_selected_line_widths(),search.work_steps(),search.record_charge());}
                        assert!(matches!(search.paragraph_width_feedback(&closed),Err(e) if e.kind==typaxis_pagination::ProductionBodyPaginationErrorKind::PendingRegion("table_header_variant_width_feedback")));
                        if work==100_000_000 {eprintln!("header source closure: wide_base={wide_base},{mode},notes={notes},harano={},semantic={},repeated={},variants={},work={},records={}",font.is_some(),closed.semantic_fragments(),closed.repeated_fragments(),closed.header_variant_fragments(),closed.work_steps(),closed.record_charge());}
                        match search.finalize_mixed_page_math(closed,&limits,0) {
                            Err(e) if e.kind==typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch=>{},
                            Err(e)=>return Err(e),Ok(_)=>panic!("variant resources still require integration"),
                        }
                        if mode == "common" {assert!(geometry.pages().iter().any(|p|p.list_markers().iter().any(|m|p.header_variant(m.fragment_index() as usize).is_some())));}
                    }
                    let selected_work=search.work_steps();
                    let selected_records=search.record_charge();
                    // Mixed placement resolves variant owners; legacy leaf queries above stay guarded.
                    if !placement && work > selected_work {
                        search.place_mixed_pages(sequence).unwrap();
                    }
                    Ok((selected_work,selected_records,fingerprints,sequence.pages().len(),variants))
                };
                let full=run(100_000_000,0).unwrap_or_else(|e|panic!("{mode}/{notes}: {e:?}"));
                assert_eq!(run(full.0,0).unwrap(),full);
                assert!(run(full.0-1,0).is_err());
                let raised=limits.base().get().max_fragments/2;
                let delta=run(full.0,raised).unwrap().1-raised;
                let exact=limits.base().get().max_fragments-delta;
                let boundary=run(full.0,exact).unwrap();
                assert_eq!(boundary.1,limits.base().get().max_fragments);
                assert_eq!(boundary.2,full.2);
                assert!(run(full.0,exact+1).is_err());
                eprintln!("header catalog pages: placement={placement},wide_base={wide_base},{mode},notes={notes},harano={},catalog_work={},catalog_records={},work={},records={},pages={},variants={}",font.is_some(),catalog.work_steps(),catalog.record_charge(),full.0,full.1,full.3,full.4);
            }).unwrap();
        }
    }
}
#[test]
fn book_v2_table_header_catalog_selects_body_and_note_page_frames() {
    check(None, false);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_header_catalog_selects_original_harano_page_frames() {
    check(
        Some(&fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap()),
        false,
    );
}


fn check_automatic_catalog(
    seed: &typaxis_layout::book_v2::BookV2BodyLineVariantSeed<'_>,
    limits: &M4EffectiveResourceLimits,
    narrow: PositiveLength,
    wide: PositiveLength,
) {
    use crate::book_v2_resources::converged_pdf::header_catalog_driver::{
        with_discovered_header_catalog, with_header_catalog, HeaderCatalogBudget,
        HeaderWidthRequest,
    };
    let mut budget = HeaderCatalogBudget::default();
    let requests = with_discovered_header_catalog(
        seed,
        limits,
        1_000_000_000,
        &mut budget,
        |catalog, budget| {
            assert_eq!(
                catalog.base().flow().lines().fingerprint(),
                seed.fingerprint()
            );
            assert_eq!(catalog.len(), 2);
            assert_eq!(budget.page_passes, 3);
            let mut requests = Vec::new();
            for physical in [narrow, wide] {
                let header = catalog.for_region_width(0, Some(physical)).unwrap();
                let owner = catalog.base().tables()[0].owner();
                let parent_width = header
                    .variant()
                    .flow()
                    .lines()
                    .frames()
                    .unwrap()
                    .region(owner)
                    .unwrap()
                    .width();
                requests.push(HeaderWidthRequest {
                    table_index: 0,
                    owner,
                    parent_width,
                });
            }
            Ok(requests)
        },
    )
    .unwrap();
    let attempts = budget.page_passes;
    let build = |work, prior, passes| {
        let mut budget = HeaderCatalogBudget {
            work: 0,
            records: prior,
            line_passes: passes,
            page_passes: 0,
        };
        let fingerprint =
            with_header_catalog(seed, &requests, limits, work, &mut budget, |catalog, _| {
                Ok(catalog.fingerprint())
            })?;
        Ok::<_, crate::book_v2_resources::BookV2ConvergenceError>((budget, fingerprint))
    };
    let built = build(1_000_000_000, 0, 0).unwrap();
    assert_eq!(build(built.0.work, 0, 0).unwrap(), built);
    assert!(build(built.0.work - 1, 0, 0).is_err());
    let max_passes = limits.base().get().max_line_reshape_passes;
    assert_eq!(
        build(built.0.work, 0, max_passes - built.0.line_passes)
            .unwrap()
            .0
            .line_passes,
        max_passes
    );
    assert!(build(built.0.work, 0, max_passes - built.0.line_passes + 1).is_err());
    let raised = limits.base().get().max_fragments / 2;
    let added = build(built.0.work, raised, 0).unwrap().0.records - raised;
    let exact = limits.base().get().max_fragments - added;
    assert_eq!(
        build(built.0.work, exact, 0).unwrap().0.records,
        limits.base().get().max_fragments
    );
    assert!(build(built.0.work, exact + 1, 0).is_err());
    assert!(build(built.0.work, 17, 0).unwrap().0.records >= built.0.records);
    let mut invalid = requests.clone();
    invalid[0].owner = NodeId::new(u32::MAX);
    assert!(with_header_catalog(
        seed,
        &invalid,
        limits,
        1_000_000_000,
        &mut HeaderCatalogBudget::default(),
        |_, _| Ok(())
    )
    .is_err());
    invalid = requests.clone();
    invalid.reverse();
    assert!(with_header_catalog(
        seed,
        &invalid,
        limits,
        1_000_000_000,
        &mut HeaderCatalogBudget::default(),
        |_, _| Ok(())
    )
    .is_err());
    eprintln!("automatic header catalog: attempts={attempts}, headers={}, work={}, records={}, line_passes={}",requests.len(),budget.work,budget.records,budget.line_passes);
}

#[test]
fn book_v2_driver_converges_automatic_repeated_header_widths() {
    for mode in ["common", "nested-header"] {
        check_automatic_driver(None, mode);
    }
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_driver_converges_automatic_original_harano_headers() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    for mode in ["common", "nested-header"] {
        check_automatic_driver(Some(&bytes), mode);
    }
}
#[test]
fn book_v2_driver_converges_independent_nested_body_headers() {
    check_automatic_driver(None, "nested-body");
}
#[test]
fn book_v2_driver_converges_nested_headers_without_parent_head() {
    check_automatic_driver(None, "nested-body-no-head");
}
#[test]
fn book_v2_driver_converges_nested_caption_headers() {
    check_automatic_driver(None, "nested-body-caption");
}
#[test]
fn book_v2_driver_converges_parallel_nested_headers() {
    check_automatic_driver(None, "nested-body-parallel");
}
#[test]
fn book_v2_driver_converges_deep_nested_headers() {
    check_automatic_driver(None, "nested-body-deep");
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_driver_converges_original_harano_nested_headers() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_automatic_driver(Some(&bytes), "nested-body-no-head");
}
#[test]
fn book_v2_driver_shares_nested_headers_for_long_body_and_notes() {
    check_automatic_driver(None, "nested-body-stress");
}
fn check_automatic_driver(font: Option<&[u8]>, mode: &str) {
    for notes in [false, true] {
        let root = Root::new();
        let original = driver_limits();
        let mut caps = original.base().get().clone();
        caps.max_layout_passes = 128;
        caps.max_line_reshape_passes = if mode.starts_with("nested-body") {
            512
        } else {
            128
        };
        caps.max_page_break_lookback = 256;
        let limits = M4EffectiveResourceLimits::new(
            typaxis_core::ValidatedResourceLimits::new(caps).unwrap(),
            original.extension().get().clone(),
        )
        .unwrap();
        let text = if font.is_some() {
            "本文を続けて組み直す本文を続けて組み直す"
        } else {
            "Pro Pro Pro Pro Pro Pro Pro"
        };
        let height = if mode == "nested-header" { 900 } else { 300 };
        let mut data = super::header_selection::fixture(
            notes,
            if mode.starts_with("nested-body") {
                "nested-body"
            } else {
                mode
            },
            text,
        );
        if mode.starts_with("nested-body") {
            let table = if notes {
                &mut data["document"]["footnotes"][0]["blocks"][0]
            } else {
                &mut data["document"]["blocks"][0]
            };
            for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
                cell["blocks"]
                    .as_array_mut()
                    .unwrap()
                    .truncate(if mode == "nested-body-stress" { 16 } else { 8 });
            }
            let child = &mut table["body"][0]["cells"][0]["blocks"][0];
            for cell in child["body"][0]["cells"].as_array_mut().unwrap() {
                cell["blocks"]
                    .as_array_mut()
                    .unwrap()
                    .truncate(if mode == "nested-body-stress" { 16 } else { 8 });
            }
            if !matches!(mode, "nested-body" | "nested-body-stress") {
                for cell in child["body"][0]["cells"].as_array_mut().unwrap() {
                    cell["blocks"].as_array_mut().unwrap().truncate(4);
                }
            }
            if mode == "nested-body-deep" {
                let child = &mut table["body"][0]["cells"][0]["blocks"][0];
                child["columns"].as_array_mut().unwrap().truncate(1);
                for section in ["head", "body"] {
                    for row in child[section].as_array_mut().unwrap() {
                        row["cells"].as_array_mut().unwrap().truncate(1);
                    }
                }
                let grandchild = child.clone();
                child["body"][0]["cells"][0]["blocks"] = json!([grandchild]);
            }
            let child = table["body"][0]["cells"][0]["blocks"][0].clone();
            if mode == "nested-body-no-head" {
                table["head"] = json!([]);
            }
            if mode == "nested-body-parallel" {
                table["body"][0]["cells"][1]["blocks"] = json!([child]);
            } else if mode == "nested-body-caption" {
                table["caption"] = json!([child]);
                table["body"][0]["cells"][0]["blocks"] =
                    table["body"][0]["cells"][1]["blocks"].clone();
            }
            crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
                &mut data["document"],
                &mut 0,
            );
        }

        for (index, master) in data["page_masters"]["masters"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            let width = if index == 1 { 220 } else { 140 };
            master["body"]["width"] = (width * 65536).into();
            master["body"]["height"] = (height * 65536).into();
            if notes {
                master["footnote"]["width"] = (width * 65536).into();
                master["footnote"]["height"] = (height * 65536).into();
            }
        }
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
        let run = |maximum, record| {
            with_converged_book_v2_pdf(
                &input,
                &limits,
                JapaneseLineBreakMode::Normal,
                maximum,
                |pdf, observed| {
                    let display = pdf
                        .navigation()
                        .source()
                        .source()
                        .source()
                        .source()
                        .display();
                    let closed = display.source().source();
                    assert!(closed.has_header_variants());
                    if mode.starts_with("nested-body") {
                        let mut owners = BTreeSet::new();
                        for page in closed.geometry().pages() {
                            for variant in page.header_variants() {
                                owners.insert(variant.header().table_index());
                            }
                        }
                        let expected = match mode {
                            "nested-body-no-head" => BTreeSet::from([1]),
                            "nested-body-parallel" | "nested-body-deep" => {
                                BTreeSet::from([0, 1, 2])
                            }
                            _ => BTreeSet::from([0, 1]),
                        };
                        assert_eq!(owners, expected);
                        if matches!(
                            mode,
                            "nested-body"
                                | "nested-body-stress"
                                | "nested-body-parallel"
                                | "nested-body-deep"
                        ) {
                            let mut shared = false;
                            for page in closed.geometry().pages() {
                                let Some(first) = page.header_variants().first() else {
                                    continue;
                                };
                                for variant in page.header_variants() {
                                    assert!(std::ptr::eq(
                                        first.measurements(),
                                        variant.measurements()
                                    ));
                                    shared |= first.header().table_index()
                                        != variant.header().table_index();
                                }
                            }
                            assert!(shared, "independent headers must use the same measured graph at this root width");
                        }
                    }
                    assert!(closed.geometry().pages().len() > 1);
                    assert!(observed.width_feedback_passes() >= 2);
                    if record {
                        if mode.starts_with("nested-body") {
                            nested_resources::verify(display, &limits, None);
                        }
                        crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                    }
                    (
                        observed,
                        closed.geometry().pages().len(),
                        typaxis_core::sha256(pdf.bytes()),
                    )
                },
            )
        };
        let value = run(1_000_000_000, true).unwrap();
        if !notes
            && font.is_none()
            && matches!(mode, "common" | "nested-body-no-head" | "nested-body-deep")
        {
            assert_eq!(run(value.0.work_steps(), false).unwrap(), value);
            assert!(run(value.0.work_steps() - 1, false).is_err());
        }
        eprintln!(
            "automatic header driver: mode={mode},harano={},notes={notes},pages={},observed={:?}",
            font.is_some(),
            value.1,
            value.0
        );
    }
}
