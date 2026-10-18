use super::*;
use crate::book_v2_resources::converged_pdf::column_page_driver::column_page_region_driver::attach;
use crate::book_v2_resources::{
    with_budgeted_book_v2_column_display as display_drive, BookV2ConvergenceError as RegionError,
};
use typaxis_display_list::book_v2::{
    BookV2CommandDisplay, BookV2SemanticArtifact, BookV2SemanticDisplay,
};
use typaxis_display_list::book_v2::{BookV2PageRegionDisplay, BookV2PageRegionDisplayBuilder};
use typaxis_syntax::book_v2::BookV2PageRegionKind as RegionKind;

#[path = "book_v2_column_pdf_tests.rs"]
mod pdf_tests;
#[path = "book_v2_column_region_probes.rs"]
mod region_probes;

fn region_data(columns: u64, blank: bool, empty: bool) -> Value {
    let mut data =
        crate::book_v2_resources::tests::shaping_tests::page_regions::region_data("Result");
    let mut p = data["document"]["blocks"][0]["blocks"][0].clone();
    p["children"][0]["text_span"]["end_byte"] = 1.into();
    let span = p["span"].clone();
    let mut blocks = vec![p; 14];
    if blank {
        blocks.insert(
            0,
            json!({"kind":"page_break","node_id":0,"classes":[],"span":span}),
        );
    }
    data["document"]["blocks"][0]["blocks"] = blocks.into();
    let master = &mut data["page_masters"]["masters"][0];
    master["body"]["height"] = (48 * 65536).into();
    master["body"]["width"] = (360 * 65536 + 1).into();
    master["column_layout"] =
        json!({"count":columns,"gap":10*65536,"fill":"sequential","balance":"none"});
    if empty {
        for role in ["header_content", "footer_content"] {
            for p in master[role]["blocks"].as_array_mut().unwrap() {
                p["children"] = json!([]);
            }
        }
    }
    let mut next = 0;
    semantic_renumber(&mut data["document"], &mut next);
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        for role in ["header_content", "footer_content"] {
            if let Some(v) = m.get_mut(role) {
                semantic_renumber(v, &mut next);
            }
        }
    }
    data
}

// This companion uses the original region source/shape/line graph. It records
// local coordinates before the owned projection, without a Body display.
fn original_regions<'a>(
    input: &'a PreparedBookV2Resources,
    caps: &'a M4EffectiveResourceLimits,
    epoch: [u8; 32],
    pages: &[(u32, Option<String>)],
) -> (Vec<BookV2PageRegionDisplay<'a>>, Vec<Value>) {
    use typaxis_layout::book_v2::with_converged_book_v2_page_region_lines;
    use typaxis_syntax::book_v2::{
        prepare_book_v2_navigation, prepare_book_v2_page_region_text_flow,
        select_book_v2_page_master,
    };
    let source = input.body().styled();
    let navigation = prepare_book_v2_navigation(source).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), caps).unwrap();
    let mut builder =
        BookV2PageRegionDisplayBuilder::new(source, input.resources(), caps, epoch, 0, 0, MAXIMUM)
            .unwrap();
    let mut regions = Vec::new();
    let mut proofs = Vec::new();
    for (page, name) in pages {
        let selected =
            select_book_v2_page_master(source, *page, name.as_deref(), &mut 0, MAXIMUM).unwrap();
        for (kind, present) in [
            (
                RegionKind::Header,
                selected.advanced().header_content.is_some(),
            ),
            (
                RegionKind::Footer,
                selected.advanced().footer_content.is_some(),
            ),
        ] {
            if !present {
                continue;
            }
            let flow =
                prepare_book_v2_page_region_text_flow(selected, kind, &navigation, 0).unwrap();
            let display = with_converged_book_v2_page_region_lines(&policy, &flow, input.resources(), caps, epoch, MODE, selected, MAXIMUM, caps.base().get().max_line_reshape_passes, 0, |stable| {
                let lines = stable.lines();
                let mut clusters = Vec::new();
                let mut origins = Vec::new();
                for origin in lines.origins() {
                    let paragraph = &lines.paragraphs()[origin.paragraph_index() as usize];
                    let line = &paragraph.lines()[origin.line_index() as usize];
                    let font=paragraph.font();
                    let mut items=Vec::new();
                    for (inline,item) in line.items().iter().enumerate() {
                        if let ProductionPlacedInline::Text(c) = item {
                            let ShapeSourceSpan::Parsed(span) = c.source_span() else { panic!("authored region cluster") };
                            let cluster=json!({"kind":"text","inline":inline,"owner":c.run().owner().get(),"paragraph":paragraph.owner().get(),"line":origin.line_index(),"source":[span.text_id().get(),span.start_byte().get(),span.end_byte().get()],"text":c.utf8(),"origin":[origin.x().raw(),origin.y().raw()],"baseline":line.baseline().raw(),"font_instance":c.run().glyph_run().font.get(),"pen_x":c.pen_x().raw(),"advance":c.glyphs().iter().map(|g|g.glyph().advance_x.raw()).sum::<i64>(),"glyphs":c.glyphs().iter().map(|g|json!([g.glyph().original_gid.get(),g.x().raw(),g.y().raw()])).collect::<Vec<_>>(),"original_glyphs":c.glyphs().iter().map(|g|json!({"gid":g.glyph().original_gid.get(),"advance_x":g.glyph().advance_x.raw(),"advance_y":g.glyph().advance_y.raw(),"offset_x":g.glyph().offset_x.raw(),"offset_y":g.glyph().offset_y.raw()})).collect::<Vec<_>>()});
                            items.push(cluster.clone());clusters.push(cluster);
                        } else if let ProductionPlacedInline::Break(b)=item {
                            let span=b.source_span();
                            items.push(json!({"kind":"break","inline":inline,"owner":b.owner().get(),"break_kind":format!("{:?}",b.kind()),"source":[span.source_id().get(),span.start_byte().get(),span.end_byte().get()]}));
                        } else {panic!("region source permits authored text and breaks only");}
                    }
                    origins.push(json!({"paragraph_index":origin.paragraph_index(),"paragraph":paragraph.owner().get(),"line":origin.line_index(),"x":origin.x().raw(),"y":origin.y().raw(),"baseline":line.baseline().raw(),"font":font.map(|font|json!({"face":font.face_id().get(),"sha256":probes::hex(&font.content_hash()),"index":font.face_index(),"size":font.size().get().raw(),"ascent":font.ascender().raw(),"descent":font.descender().raw(),"line_gap":font.line_gap().raw()})),"items":items}));
                }
                let before=[builder.record_charge(),builder.work_steps()];
                let display=builder.build(&stable).unwrap();
                proofs.push(json!({"scope":"local_source_companion","owner":flow.source().node_id,"page":page,"kind":kind.as_str(),"frame":raw_rect(lines.frame()),"layout_fingerprint":probes::hex(&lines.fingerprint()),"shape_fingerprint":probes::hex(&lines.prepared().shaped().fingerprint()),"line_context_fingerprint":lines.prepared().shaped().line_context_fingerprint().map(|f|probes::hex(&f)),"reshape_passes":stable.passes().len(),"candidate_steps":stable.candidate_steps(),"builder_before":before,"builder_after":[builder.record_charge(),builder.work_steps()],"clusters":clusters,"origins":origins}));
                display
            }).unwrap();
            regions.push(display);
        }
    }
    (regions, proofs)
}

fn assert_projection(region: &BookV2PageRegionDisplay<'_>, proof: &Value) {
    assert_eq!(region.page_index(), proof["page"].as_u64().unwrap() as u32);
    assert_eq!(region.kind().as_str(), proof["kind"]);
    assert_eq!(json!(raw_rect(region.frame())), proof["frame"]);
    assert_eq!(
        probes::hex(&region.layout_fingerprint()),
        proof["layout_fingerprint"]
    );
    let clusters = proof["clusters"].as_array().unwrap();
    assert_eq!(region.draws().len(), clusters.len());
    for (draw, c) in region.draws().iter().zip(clusters) {
        assert_eq!(draw.owner().get(), c["owner"].as_u64().unwrap() as u32);
        assert_eq!(
            draw.paragraph_owner().get(),
            c["paragraph"].as_u64().unwrap() as u32
        );
        assert_eq!(draw.line_index(), c["line"].as_u64().unwrap() as u32);
        assert_eq!(draw.exact_text(), c["text"]);
        let span = draw.source_span();
        assert_eq!(
            json!([
                span.text_id().get(),
                span.start_byte().get(),
                span.end_byte().get()
            ]),
            c["source"]
        );
        assert_eq!(
            draw.font_instance().font_instance_id().get(),
            c["font_instance"].as_u64().unwrap() as u32
        );
        assert_eq!(draw.glyphs().len(), c["glyphs"].as_array().unwrap().len());
        for (glyph, original) in draw.glyphs().iter().zip(c["glyphs"].as_array().unwrap()) {
            assert_eq!(
                glyph.original_gid().get(),
                original[0].as_u64().unwrap() as u16
            );
            assert_eq!(
                glyph.x().raw(),
                c["origin"][0].as_i64().unwrap() + original[1].as_i64().unwrap()
            );
            assert_eq!(
                glyph.y().raw(),
                c["origin"][1].as_i64().unwrap() + original[2].as_i64().unwrap()
            );
        }
    }
}

#[test]
fn book_v2_column_regions_join_reserves_exact_records_and_work_and_retains_failed_retry_prefixes() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, region_data(3, true, false), None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    math_drive(&input, &caps, MODE, &mut budget, |source, _, _| {
        let epoch = source
            .original_graph()
            .lines()
            .prepared()
            .shaped()
            .binding_epoch();
        let pages: Vec<_> = source
            .source()
            .geometry()
            .pages()
            .iter()
            .enumerate()
            .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
            .collect();
        let fresh = || {
            BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                .unwrap()
                .build()
                .unwrap()
        };
        let make = || original_regions(&input, &caps, epoch, &pages).0;
        let start = fresh();
        let regions = make();
        let additional = regions.iter().map(|r| r.draws().len()).sum::<usize>();
        // Both replacement paint storage and its compact private locators coexist.
        let allocation =
            (start.paints().len() + additional + regions.len() + 1 + additional + 1) as u64;
        let joined = start
            .with_page_regions(regions, &caps, MAXIMUM, 0, 0)
            .unwrap();
        let (end_records, end_work) = (joined.record_charge(), joined.work_steps());
        let exact = fresh()
            .with_page_regions(make(), &caps, end_work, 0, 0)
            .unwrap();
        assert_eq!(
            (exact.record_charge(), exact.work_steps()),
            (end_records, end_work)
        );
        assert_eq!(exact.fingerprint(), joined.fingerprint());
        let cap = caps.base().get().max_fragments;
        let record_exact = fresh()
            .with_page_regions(make(), &caps, MAXIMUM, cap - allocation, 0)
            .unwrap();
        assert_eq!(record_exact.record_charge(), cap);
        assert_eq!(record_exact.fingerprint(), joined.fingerprint());
        for (prior_records, maximum) in [(cap - allocation + 1, MAXIMUM), (0, end_work - 1)] {
            let (mut records, mut work) = (0, 0);
            let error = fresh()
                .with_page_regions_counted(
                    make(),
                    &caps,
                    maximum,
                    prior_records,
                    0,
                    &mut records,
                    &mut work,
                )
                .err()
                .unwrap();
            assert!(format!("{error:?}").contains(if prior_records > 0 {
                "RecordLimit"
            } else {
                "WorkLimit"
            }));
            assert!(records <= cap && work <= maximum);
            let floor = (records, work);
            assert!(fresh()
                .with_page_regions_counted(
                    make(),
                    &caps,
                    maximum,
                    records,
                    work,
                    &mut records,
                    &mut work
                )
                .is_err());
            assert!(records >= floor.0 && work >= floor.1 && records <= cap && work <= maximum);
        }
    })
    .unwrap();
}

#[test]
fn book_v2_column_regions_attachment_failures_keep_source_records_begun_line_passes_and_join_work()
{
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, region_data(2, true, false), None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    math_drive(&input, &caps, MODE, &mut budget, |source, _, _| {
        let policy = prepare_book_v2_resource_policy(input.body(), &caps).unwrap();
        let fresh = || {
            BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                .unwrap()
                .build()
                .unwrap()
        };
        let start = (fresh().record_charge(), fresh().work_steps());
        let (mut records, mut work, mut passes) = (0, 0, 0);
        let joined = attach(
            fresh(),
            &policy,
            &caps,
            MODE,
            MAXIMUM,
            &mut passes,
            &mut records,
            &mut work,
        )
        .unwrap();
        assert_eq!(
            (records, work),
            (joined.record_charge(), joined.work_steps())
        );
        assert!(passes > 2);
        let end = work;
        let (mut exact_records, mut exact_work, mut exact_passes) = (0, 0, 0);
        let exact = attach(
            fresh(),
            &policy,
            &caps,
            MODE,
            end,
            &mut exact_passes,
            &mut exact_records,
            &mut exact_work,
        )
        .unwrap();
        assert_eq!(exact.fingerprint(), joined.fingerprint());
        assert_eq!(
            (exact_records, exact_work, exact_passes),
            (records, work, passes)
        );
        let mut stages = BTreeSet::new();
        for maximum in (1..24)
            .map(|i| start.1 + (end - start.1) * i / 24)
            .chain([start.1, end - 1])
        {
            let (mut r, mut w, mut p) = (0, 0, 0);
            let error = attach(
                fresh(),
                &policy,
                &caps,
                MODE,
                maximum,
                &mut p,
                &mut r,
                &mut w,
            )
            .err()
            .unwrap();
            let message = format!("{error:?}");
            assert!(r >= start.0 && w >= start.1 && w <= maximum, "{message}");
            assert!(p <= passes);
            if message.contains("column page-region join") {
                stages.insert("join");
                assert_eq!(p, passes);
            }
            if message.contains("column page-region display") {
                stages.insert("display");
            }
            if message.contains("column page-region lines") {
                stages.insert("lines");
            }
        }
        assert_eq!(stages, ["display", "join", "lines"].into_iter().collect());
        // Reproduce the first source boundary from the authentic navigation.
        let original = source.original_graph().lines().prepared();
        let body = original.source_flow().body();
        let nav = original.source_flow().navigation();
        let selected =
            typaxis_syntax::book_v2::select_book_v2_page_master(body, 0, None, &mut 0, MAXIMUM)
                .unwrap();
        let epoch = original.shaped().binding_epoch();
        let slots = input.resources().fonts().len() as u64 * 2 + 1;
        let source_flow = typaxis_syntax::book_v2::prepare_book_v2_page_region_text_flow(
            selected,
            RegionKind::Header,
            nav,
            0,
        )
        .unwrap();
        let source_slots = source_flow.record_charge();
        let display_delta = start.0 - source.record_charge();
        let cap = caps.base().get().max_fragments;
        for remaining in 0..source_slots {
            let prior = cap - remaining;
            let (mut expected_records, mut r, mut w, mut p) = (0, 0, 0, 0);
            let expected = typaxis_syntax::book_v2::prepare_book_v2_page_region_text_flow_counted(
                selected,
                RegionKind::Header,
                nav,
                prior,
                &mut expected_records,
            )
            .err()
            .unwrap();
            let near = BookV2ColumnDisplayBuilder::new(
                source,
                input.resources(),
                &caps,
                MAXIMUM,
                prior - slots - 1 - display_delta,
                0,
            )
            .unwrap()
            .build()
            .unwrap();
            assert_eq!(near.record_charge(), prior - slots - 1);
            let error = attach(near, &policy, &caps, MODE, MAXIMUM, &mut p, &mut r, &mut w)
                .err()
                .unwrap();
            let RegionError::Stage {
                stage,
                source: actual,
            } = error
            else {
                panic!("source error precedence")
            };
            assert_eq!(stage, "column page-region source");
            assert_eq!(
                actual.downcast_ref::<typaxis_syntax::ProductionFlowError>(),
                Some(&expected)
            );
            assert_eq!(r, expected_records);
            assert_eq!(p, 0);
        }
        assert_ne!(epoch, [0; 32]);
    })
    .unwrap();
}

#[test]
fn book_v2_column_regions_public_driver_exact_work_callback_build_and_retries_keep_shared_history()
{
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, region_data(2, false, false), None, &caps);
    let mut baseline = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let (fingerprint, total) = display_drive(
        &input,
        &caps,
        MODE,
        &mut baseline,
        |display, builder, observed| {
            assert_eq!(
                (builder.record_charge(), builder.work_steps()),
                (display.record_charge(), display.work_steps())
            );
            assert_eq!(
                (observed.record_charge(), observed.work_steps()),
                (display.record_charge(), display.work_steps())
            );
            (display.fingerprint(), observed)
        },
    )
    .unwrap();
    let mut exact = BookV2ColumnPageBudget::new(&caps, total.work_steps());
    let observed = display_drive(&input, &caps, MODE, &mut exact, |display, _, observed| {
        assert_eq!(display.fingerprint(), fingerprint);
        observed
    })
    .unwrap();
    assert_eq!(
        (
            observed.record_charge(),
            observed.work_steps(),
            observed.line_reshape_passes()
        ),
        (
            total.record_charge(),
            total.work_steps(),
            total.line_reshape_passes()
        )
    );
    let mut short = BookV2ColumnPageBudget::new(&caps, total.work_steps() - 1);
    assert!(
        display_drive(&input, &caps, MODE, &mut short, |_, _, _| panic!(
            "short work delivered"
        ))
        .is_err()
    );
    let failed = short.observation();
    assert!(failed.work_steps() <= total.work_steps() - 1);
    assert_eq!(failed.line_reshape_passes(), total.line_reshape_passes());
    assert!(
        display_drive(&input, &caps, MODE, &mut short, |_, _, _| panic!(
            "failed retry delivered"
        ))
        .is_err()
    );
    let retried = short.observation();
    assert!(
        retried.work_steps() >= failed.work_steps()
            && retried.record_charge() >= failed.record_charge()
            && retried.line_reshape_passes() >= failed.line_reshape_passes()
    );
    let mut callback = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut after = (0, 0);
    let result = display_drive(
        &input,
        &caps,
        MODE,
        &mut callback,
        |display, builder, observed| {
            assert!(std::ptr::eq(
                display.source(),
                builder.build().unwrap().source()
            ));
            after = (builder.record_charge(), builder.work_steps());
            assert!(after.0 > observed.record_charge() && after.1 > observed.work_steps());
            Err::<(), RegionError>(RegionError::Identity)
        },
    )
    .unwrap();
    assert!(result.is_err());
    assert_eq!(
        (
            callback.observation().record_charge(),
            callback.observation().work_steps()
        ),
        after
    );
    let prior = callback.observation();
    display_drive(&input, &caps, MODE, &mut callback, |_, _, _| ()).unwrap();
    assert!(callback.observation().record_charge() > prior.record_charge());
    assert!(callback.observation().work_steps() > prior.work_steps());
    assert!(callback.observation().line_reshape_passes() > prior.line_reshape_passes());
}

#[test]
fn book_v2_column_regions_public_driver_records_are_inclusive_and_refused_retry_never_releases_reservations(
) {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, region_data(3, false, false), None, &caps);
    let mut baseline = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let required = display_drive(&input, &caps, MODE, &mut baseline, |_, _, o| {
        o.record_charge()
    })
    .unwrap();
    for short in [false, true] {
        let limited = changed_caps(&caps, |c| c.max_fragments = required - u64::from(short));
        let root = Root::new();
        let input = display_input(&root, region_data(3, false, false), None, &limited);
        let mut budget = BookV2ColumnPageBudget::new(&limited, MAXIMUM);
        let result = display_drive(&input, &limited, MODE, &mut budget, |_, _, o| o);
        if !short {
            assert_eq!(result.unwrap().record_charge(), required);
        } else {
            assert!(result.is_err());
            let prefix = budget.observation();
            assert!(prefix.record_charge() <= required - 1);
            assert!(
                display_drive(&input, &limited, MODE, &mut budget, |_, _, _| panic!(
                    "one-short retry delivered"
                ))
                .is_err()
            );
            let retry = budget.observation();
            assert!(
                retry.record_charge() >= prefix.record_charge()
                    && retry.record_charge() <= required - 1
            );
            assert!(retry.work_steps() >= prefix.work_steps());
            assert!(retry.line_reshape_passes() >= prefix.line_reshape_passes());
        }
    }
}

fn named_region_data(blanks: usize, balanced: bool) -> Value {
    let mut data = region_data(2, false, false);
    let span = data["document"]["blocks"][0]["span"].clone();
    let blocks = data["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    for paragraph in blocks.iter_mut() {
        paragraph["classes"] = json!(["region-appendix"]);
    }
    for _ in 0..blanks {
        blocks.insert(
            0,
            json!({"kind":"page_break","node_id":0,"classes":[],"span":span}),
        );
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"region-appendix","selector":"paragraph.region-appendix","source_order":rules.len(),"extends":null,"declarations":[{"name":"page","important":false,"value":{"kind":"string","value":"region-appendix"}}]}));
    let mut template = data["page_masters"]["masters"][0].clone();
    template["body"]["y"] = (100 * 65536).into();
    template["header"]["height"] = (80 * 65536).into();
    data["page_masters"]["masters"][0] = template.clone();
    for (parity, count, x, width) in [("left", 3, 13, 70), ("right", 2, 19, 110)] {
        let mut master = template.clone();
        master["master_id"] = format!("region-appendix-{parity}").into();
        master["body"]["width"] = ((count * 100 + (count - 1) * 10) * 65536 + 1).into();
        master["body"]["height"] = (64 * 65536).into();
        master["column_layout"]["count"] = count.into();
        master["column_layout"]["balance"] = if balanced { "last_page" } else { "none" }.into();
        master["header"]["x"] = (x * 65536).into();
        master["header"]["width"] = (width * 65536 + 1).into();
        master["footer"]["x"] = ((x + 4) * 65536).into();
        master["footer"]["width"] = ((width + 30) * 65536).into();
        data["page_masters"]["masters"]
            .as_array_mut()
            .unwrap()
            .push(master);
    }
    data["page_masters"]["selection_rules"] = json!([
        {"master_id":"region-appendix-left","parity":"even","first":null,"named_page":"region-appendix","source_order":0},
        {"master_id":"region-appendix-right","parity":"odd","first":null,"named_page":"region-appendix","source_order":1}
    ]);
    let mut next = 0;
    semantic_renumber(&mut data["document"], &mut next);
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        for role in ["header_content", "footer_content"] {
            semantic_renumber(&mut master[role], &mut next);
        }
    }
    data
}

#[test]
fn book_v2_column_regions_absent_fastpath_and_unselected_authored_masters_have_distinct_accounting()
{
    let caps = display_caps();
    for authored in [false, true] {
        let root = Root::new();
        let mut data = region_data(2, false, false);
        let mut unused = data["page_masters"]["masters"][0].clone();
        unused["master_id"] = "never-selected-regions".into();
        let selected = &mut data["page_masters"]["masters"][0];
        selected["header_content"] = Value::Null;
        selected["footer_content"] = Value::Null;
        if authored {
            data["page_masters"]["masters"]
                .as_array_mut()
                .unwrap()
                .push(unused);
        }
        let input = display_input(&root, data, None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        math_drive(&input, &caps, MODE, &mut budget, |source, _, _| {
            let policy = prepare_book_v2_resource_policy(input.body(), &caps).unwrap();
            let base =
                BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                    .unwrap()
                    .build()
                    .unwrap();
            let before = (
                base.fingerprint(),
                base.record_charge(),
                base.work_steps(),
                base.paints().to_vec(),
            );
            let (mut records, mut work, mut passes) = (0, 0, 7);
            let attached = attach(
                base,
                &policy,
                &caps,
                MODE,
                MAXIMUM,
                &mut passes,
                &mut records,
                &mut work,
            )
            .unwrap();
            assert!(attached.page_regions().is_empty());
            assert_eq!(attached.fingerprint(), before.0);
            assert_eq!(attached.paints(), before.3);
            assert_eq!(passes, 7);
            if authored {
                assert_eq!(
                    records,
                    before.1 + input.resources().fonts().len() as u64 * 2 + 1
                );
                assert!(work > before.2);
            } else {
                assert_eq!((records, work), (before.1, before.2));
            }
        })
        .unwrap();
    }
}

#[test]
fn book_v2_column_regions_cover_empty_zero_and_trailing_blank_selected_pages() {
    let caps = display_caps();
    let mut failures = Vec::new();
    for body in ["empty-paragraph", "anchor-only", "forced", "trailing"] {
        let root = Root::new();
        let mut data = region_data(3, false, false);
        let mut p = data["document"]["blocks"][0]["blocks"][0].clone();
        let command = json!({"kind":"page_break","node_id":0,"classes":[],"span":p["span"]});
        if body == "empty-paragraph" {
            p["children"] = json!([]);
        }
        if body == "anchor-only" {
            p["children"] = json!([{"kind":"anchor","node_id":0,"span":p["span"],"anchor_id":"region-empty-body"}]);
        }
        data["document"]["blocks"][0]["blocks"] = match body {
            "empty-paragraph" | "anchor-only" => json!([p]),
            "forced" => json!([command.clone(), command]),
            _ => json!([p, command.clone(), command]),
        };
        let mut next = 0;
        semantic_renumber(&mut data["document"], &mut next);
        for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
            for role in ["header_content", "footer_content"] {
                semantic_renumber(&mut m[role], &mut next);
            }
        }
        let authored = data.clone();
        let input = display_input(&root, data, None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let result = navigation_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, _, observed| {
                let display = bundle.display();
                let pages = display.source().source().geometry().pages();
                assert_eq!(
                    pages.len(),
                    if matches!(body, "empty-paragraph" | "anchor-only") {
                        1
                    } else {
                        3
                    }
                );
                if body == "forced" {
                    assert!(pages.iter().all(|p| p.fragments().is_empty()));
                } else if matches!(body, "empty-paragraph" | "anchor-only") {
                    assert!(display.text().draws().is_empty());
                } else {
                    assert!(!pages[0].fragments().is_empty());
                    assert!(pages[1..].iter().all(|p| p.fragments().is_empty()));
                }
                assert_eq!(display.page_regions().len(), 2 * pages.len());
                let selected: Vec<_> = pages
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
                    .collect();
                let (_, proofs) = original_regions(
                    &input,
                    &caps,
                    display
                        .source()
                        .original_graph()
                        .lines()
                        .prepared()
                        .shaped()
                        .binding_epoch(),
                    &selected,
                );
                for (r, p) in display.page_regions().iter().zip(&proofs) {
                    assert_projection(r, p);
                }
                assert!(display
                    .paints()
                    .iter()
                    .any(|p| matches!(p, Paint::PageRegionText { .. })));
                region_probes::emit_nav(
                    &format!("regions-{body}"),
                    bundle,
                    &caps,
                    observed,
                    &proofs,
                );
            },
        );
        if let Err(error) = result {
            let o = budget.observation();
            let value = json!({"case":body,"error":format!("{error:?}"),"source_package":authored,"records":o.record_charge(),"spool":o.spool_charge(),"output":o.output_charge(),"work":o.work_steps(),"line_passes":o.line_reshape_passes()});
            println!("REGION_ZERO_DIAGNOSTIC {value}");
            if let Some(dir) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
                let dir = std::path::PathBuf::from(dir).join("region-refusals");
                fs::create_dir_all(&dir).unwrap();
                fs::write(
                    dir.join(format!("{body}.json")),
                    serde_json::to_vec_pretty(&value).unwrap(),
                )
                .unwrap();
            }
            failures.push((body, error));
        }
    }
    assert!(
        failures.is_empty(),
        "genuine zero source cases: {failures:?}"
    );
}

#[test]
fn book_v2_column_regions_nested_styled_kept_forced_only_original_containers_preserve_blank_debt() {
    let caps = display_caps();
    for columns in [2, 3] {
        for depth in [1, 2, 3] {
            let mut data = region_data(columns, false, false);
            let mut container = data["document"]["blocks"][0].clone();
            let span = container["span"].clone();
            let command = json!({"kind":"page_break","node_id":0,"classes":[],"span":span});
            container["classes"] = json!(["forced-regions"]);
            container["blocks"] = json!([command.clone(), command]);
            for _ in 1..depth {
                let mut parent = container.clone();
                parent["blocks"] = json!([container]);
                container = parent;
            }
            data["document"]["blocks"] = json!([container]);
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            rules.push(json!({"style_id":"forced-regions","selector":"semantic_container.forced-regions","source_order":rules.len(),"extends":null,"declarations":[
            {"name":"space_before","important":false,"value":{"kind":"length","value":2*65536}},
            {"name":"space_after","important":false,"value":{"kind":"length","value":3*65536}},
            {"name":"start_indent","important":false,"value":{"kind":"length","value":65536}},
            {"name":"end_indent","important":false,"value":{"kind":"length","value":2*65536}},
            {"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}
        ]}));
            let mut next = 0;
            semantic_renumber(&mut data["document"], &mut next);
            for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
                for role in ["header_content", "footer_content"] {
                    semantic_renumber(&mut m[role], &mut next);
                }
            }
            fn forced_owners(v: &Value, out: &mut Vec<u32>) {
                if v["kind"] == "page_break" {
                    out.push(v["node_id"].as_u64().unwrap() as u32);
                }
                if let Some(blocks) = v.get("blocks").and_then(Value::as_array) {
                    for b in blocks {
                        forced_owners(b, out);
                    }
                }
            }
            let mut expected = Vec::new();
            forced_owners(&data["document"], &mut expected);
            assert_eq!(expected.len(), 2);
            let root = Root::new();
            let input = display_input(&root, data, None, &caps);
            let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
            navigation_drive(
                &input,
                &caps,
                MODE,
                &mut budget,
                4,
                |bundle, _, observed| {
                    let display = bundle.display();
                    let pages = display.source().source().geometry().pages();
                    assert_eq!(pages.len(), 3);
                    assert_eq!(
                        pages
                            .iter()
                            .filter_map(|p| p.selection().forced_break().map(|n| n.get()))
                            .collect::<Vec<_>>(),
                        expected
                    );
                    assert!(
                        pages
                            .iter()
                            .all(|p| p.fragments().is_empty()
                                && p.columns().len() == columns as usize)
                    );
                    assert!(
                        display.text().draws().is_empty()
                            && display.math().draws().is_empty()
                            && display.images().draws().is_empty()
                            && display.numbers().draws().is_empty()
                            && display.markers().draws().is_empty()
                    );
                    assert_eq!(display.page_regions().len(), 6);
                    let scopes = bundle.marked().content().source();
                    assert!(scopes
                        .groups()
                        .iter()
                        .all(|g| g.mcid().is_none() && g.artifact().is_some()));
                    let tree = bundle.relations().source();
                    assert!(tree.bindings().is_empty());
                    let requests: Vec<_> = pages
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
                        .collect();
                    let (_, proofs) = original_regions(
                        &input,
                        &caps,
                        display
                            .source()
                            .original_graph()
                            .lines()
                            .prepared()
                            .shaped()
                            .binding_epoch(),
                        &requests,
                    );
                    for (r, p) in display.page_regions().iter().zip(&proofs) {
                        assert_projection(r, p);
                    }
                    region_probes::emit_nav(
                        &format!("regions-forced-kept-{columns}-{depth}"),
                        bundle,
                        &caps,
                        observed,
                        &proofs,
                    );
                },
            )
            .unwrap();
        }
    }
}

#[test]
fn book_v2_column_regions_failed_layout_and_shared_pass_limit_survive_whole_driver_retry() {
    let caps = display_caps();
    let root = Root::new();
    let mut data = region_data(2, false, false);
    data["page_masters"]["masters"][0]["header"]["height"] = 1.into();
    let input = display_input(&root, data, None, &caps);
    let mut source_budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let source_prefix = math_drive(&input, &caps, MODE, &mut source_budget, |_, _, o| o).unwrap();
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let error = display_drive(&input, &caps, MODE, &mut budget, |_, _, _| {
        panic!("bad header height delivered")
    })
    .unwrap_err();
    assert!(format!("{error:?}").contains("column page-region lines"));
    let first = budget.observation();
    assert!(first.record_charge() > source_prefix.record_charge());
    assert!(first.line_reshape_passes() > source_prefix.line_reshape_passes());
    assert!(first.work_steps() > source_prefix.work_steps());
    assert_eq!(first.output_charge(), 0);
    assert!(
        display_drive(&input, &caps, MODE, &mut budget, |_, _, _| panic!(
            "failed retry delivered"
        ))
        .is_err()
    );
    let second = budget.observation();
    assert!(
        second.record_charge() >= first.record_charge()
            && second.work_steps() >= first.work_steps()
            && second.line_reshape_passes() > first.line_reshape_passes()
    );
    let valid = display_input(&root, region_data(2, false, false), None, &caps);
    let mut ample = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let passes = display_drive(&valid, &caps, MODE, &mut ample, |_, _, o| {
        o.line_reshape_passes()
    })
    .unwrap();
    for extra in [0, 1] {
        let limited = changed_caps(&caps, |c| c.max_line_reshape_passes = passes - extra);
        let limited_root = Root::new();
        let limited_input =
            display_input(&limited_root, region_data(2, false, false), None, &limited);
        let mut budget = BookV2ColumnPageBudget::new(&limited, MAXIMUM);
        let result = display_drive(&limited_input, &limited, MODE, &mut budget, |_, _, o| o);
        if extra == 0 {
            assert_eq!(result.unwrap().line_reshape_passes(), passes);
        } else {
            assert!(result.is_err());
            assert_eq!(budget.observation().line_reshape_passes(), passes - 1);
            let prefix = budget.observation();
            assert!(display_drive(
                &limited_input,
                &limited,
                MODE,
                &mut budget,
                |_, _, _| panic!("exhausted passes delivered")
            )
            .is_err());
            assert!(budget.observation().record_charge() >= prefix.record_charge());
            assert!(budget.observation().work_steps() >= prefix.work_steps());
            assert_eq!(budget.observation().line_reshape_passes(), passes - 1);
        }
    }
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT pinned to 66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"]
fn book_v2_column_regions_original_harano_running_glyphs_keep_pinned_source() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        probes::hex(&typaxis_core::sha256(&font)),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, region_data(2, true, false), Some(&font), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    navigation_drive(&input, &caps, MODE, &mut budget, 4, |bundle, _, _| {
        assert!(bundle
            .display()
            .page_regions()
            .iter()
            .flat_map(|r| r.draws())
            .all(|d| d.font().content_hash() == typaxis_core::sha256(&font)));
    })
    .unwrap();
}

#[test]
fn book_v2_column_regions_follow_named_parity_blank_frames_and_exact_original_local_glyphs() {
    let caps = display_caps();
    for blanks in [1, 2] {
        for balanced in [false, true] {
            let root = Root::new();
            let input = display_input(&root, named_region_data(blanks, balanced), None, &caps);
            let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
            navigation_drive(
                &input,
                &caps,
                MODE,
                &mut budget,
                4,
                |bundle, _, observed| {
                    let display = bundle.display();
                    let pages = display.source().source().geometry().pages();
                    assert!(pages[..blanks].iter().all(|p| p.fragments().is_empty()));
                    let positive: Vec<_> = pages.iter().skip(blanks).collect();
                    assert!(positive.len() >= 2);
                    assert!(positive
                        .iter()
                        .all(|p| p.selection().named_page() == Some("region-appendix")));
                    let counts: BTreeSet<_> = positive.iter().map(|p| p.columns().len()).collect();
                    assert_eq!(counts, [2, 3].into_iter().collect());
                    for page in positive {
                        let count = page.columns().len();
                        let widths: Vec<_> = page
                            .columns()
                            .iter()
                            .map(|c| c.bounds().width().get().raw())
                            .collect();
                        assert_eq!(
                            widths,
                            (0..count)
                                .map(|i| 100 * 65536 + i64::from(i == count - 1))
                                .collect::<Vec<_>>()
                        );
                    }
                    assert_eq!(
                        pages
                            .iter()
                            .filter(|p| p.selection().balance().is_some())
                            .count(),
                        usize::from(balanced)
                    );
                    let selected: Vec<_> = pages
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
                        .collect();
                    let epoch = display
                        .source()
                        .original_graph()
                        .lines()
                        .prepared()
                        .shaped()
                        .binding_epoch();
                    let (original, proofs) = original_regions(&input, &caps, epoch, &selected);
                    assert_eq!(original.len(), display.page_regions().len());
                    for ((region, source), proof) in
                        display.page_regions().iter().zip(&original).zip(&proofs)
                    {
                        assert_projection(region, proof);
                        assert_eq!(region.fingerprint(), source.fingerprint());
                    }
                    // A real default master on this same page has another authored
                    // region owner/frame. It cannot stand in for the selected named one.
                    let mut wrong = selected.clone();
                    wrong[blanks].1 = None;
                    let receipts = original_regions(&input, &caps, epoch, &wrong).0;
                    let unjoined = BookV2ColumnDisplayBuilder::new(
                        display.source(),
                        input.resources(),
                        &caps,
                        MAXIMUM,
                        0,
                        0,
                    )
                    .unwrap()
                    .build()
                    .unwrap();
                    assert!(unjoined
                        .with_page_regions(receipts, &caps, MAXIMUM, 0, 0)
                        .is_err());
                    let cids = bundle.marked().paints().cids();
                    let commands = bundle.marked().paints().text();
                    let mut region_uses = 0;
                    for (paint, p) in display.paints().iter().copied().enumerate() {
                        if matches!(p, Paint::PageRegionText { .. }) {
                            let expected = display.font_use(paint, 0).unwrap().unwrap();
                            let usage = cids.usage_index(paint, 0).unwrap();
                            assert_eq!(cids.cids(usage).unwrap().len(), expected.glyphs().len());
                            let for_paint = commands.for_paint(paint).unwrap();
                            assert_eq!(for_paint.len(), 1);
                            let command = &for_paint[0];
                            assert_eq!(
                                command.page_index(),
                                display.text_paint(paint).unwrap().page_index()
                            );
                            let text = std::str::from_utf8(
                                commands
                                    .command_bytes(
                                        commands
                                            .commands()
                                            .iter()
                                            .position(|c| std::ptr::eq(c, &for_paint[0]))
                                            .unwrap(),
                                    )
                                    .unwrap(),
                            )
                            .unwrap();
                            assert!(
                                text.contains(" Tf")
                                    && text.contains(" Tm")
                                    && text.contains(" Tj")
                            );
                            region_uses += 1;
                        }
                    }
                    assert!(region_uses > 0);
                    region_probes::emit_nav(
                        &format!("regions-named-{blanks}-{balanced}"),
                        bundle,
                        &caps,
                        observed,
                        &proofs,
                    );
                },
            )
            .unwrap();
        }
    }
}

#[test]
fn book_v2_column_regions_actual_narrow_context_reshapes_original_font_glyphs_before_selection() {
    const FONT:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/vmb-book/header-context-font/header-context.ttf"));
    let caps = display_caps();
    let root = Root::new();
    let mut data = crate::book_v2_resources::tests::shaping_tests::page_regions::region_data("f i");
    let p = data["document"]["blocks"][0]["blocks"][0].clone();
    data["document"]["blocks"][0]["blocks"] = vec![p; 8].into();
    let m = &mut data["page_masters"]["masters"][0];
    m["body"]["y"] = (120 * 65536).into();
    m["body"]["height"] = (48 * 65536).into();
    m["column_layout"] = json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"none"});
    m["header"]["width"] = (14 * 65536).into();
    m["header"]["height"] = (96 * 65536).into();
    m["footer"]["width"] = (80 * 65536).into();
    m["footer"]["height"] = (60 * 65536).into();
    let mut next = 0;
    semantic_renumber(&mut data["document"], &mut next);
    for role in ["header_content", "footer_content"] {
        semantic_renumber(&mut data["page_masters"]["masters"][0][role], &mut next);
    }
    let input = display_input(&root, data, Some(FONT), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    navigation_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            let display = bundle.display();
            let pages = display.source().source().geometry().pages();
            let header: BTreeSet<_> = display
                .page_regions()
                .iter()
                .filter(|r| r.kind() == RegionKind::Header)
                .flat_map(|r| r.draws())
                .flat_map(|d| d.glyphs())
                .map(|g| g.original_gid().get())
                .collect();
            let footer: BTreeSet<_> = display
                .page_regions()
                .iter()
                .filter(|r| r.kind() == RegionKind::Footer)
                .flat_map(|r| r.draws())
                .flat_map(|d| d.glyphs())
                .map(|g| g.original_gid().get())
                .collect();
            assert!(header.contains(&71) && !header.contains(&59));
            assert!(footer.contains(&59) && !footer.contains(&71));
            assert!(display
                .page_regions()
                .iter()
                .flat_map(|r| r.draws())
                .all(|d| d.font().content_hash() == typaxis_core::sha256(FONT)));
            let requests: Vec<_> = pages
                .iter()
                .enumerate()
                .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
                .collect();
            let (_, proofs) = original_regions(
                &input,
                &caps,
                display
                    .source()
                    .original_graph()
                    .lines()
                    .prepared()
                    .shaped()
                    .binding_epoch(),
                &requests,
            );
            for (r, p) in display.page_regions().iter().zip(&proofs) {
                assert_projection(r, p);
                assert!(p["line_context_fingerprint"].is_string());
                assert!(p["reshape_passes"].as_u64().unwrap() >= 2);
            }
            region_probes::emit_nav(
                "regions-contextual-original-gid71",
                bundle,
                &caps,
                observed,
                &proofs,
            );
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_regions_select_a_declared_original_collection_face_used_only_by_running_text() {
    let caps = display_caps();
    let root = Root::new();
    let mut data = region_data(2, true, false);
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        for role in ["header_content", "footer_content"] {
            for p in m[role]["blocks"].as_array_mut().unwrap() {
                p["classes"] = json!(["running-collection"]);
            }
        }
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    for kind in ["paragraph", "heading"] {
        rules.push(json!({"style_id":format!("region-font-{kind}"),"selector":format!("{kind}.running-collection"),"source_order":rules.len(),"extends":null,"declarations":[{"name":"font_family","important":false,"value":{"kind":"font_family_list","families":["Collection"]}}]}));
    }
    let input = display_input(&root, data, None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    navigation_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            let display = bundle.display();
            assert!(display
                .text()
                .draws()
                .iter()
                .all(|t| t.font().face_id().get() == 0));
            assert!(display
                .page_regions()
                .iter()
                .flat_map(|r| r.draws())
                .all(|t| t.font().face_id().get() == 1));
            let selected = bundle
                .marked()
                .paints()
                .cids()
                .source()
                .source()
                .selection();
            assert_eq!(
                selected
                    .fonts()
                    .iter()
                    .map(|f| f.instance().font().font_face_id().get())
                    .collect::<BTreeSet<_>>(),
                [0, 1].into_iter().collect()
            );
            for usage in selected.uses() {
                let region = matches!(usage.usage().paint(), Paint::PageRegionText { .. });
                assert_eq!(
                    usage.usage().instance().font().font_face_id().get(),
                    u32::from(region)
                );
            }
            let pages = display.source().source().geometry().pages();
            let requests: Vec<_> = pages
                .iter()
                .enumerate()
                .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
                .collect();
            let (_, proofs) = original_regions(
                &input,
                &caps,
                display
                    .source()
                    .original_graph()
                    .lines()
                    .prepared()
                    .shaped()
                    .binding_epoch(),
                &requests,
            );
            for (r, p) in display.page_regions().iter().zip(&proofs) {
                assert_projection(r, p);
            }
            region_probes::emit_nav(
                "regions-original-collection-only-font",
                bundle,
                &caps,
                observed,
                &proofs,
            );
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_regions_merge_after_body_formula_images_numbers_and_full_page_notes() {
    let caps = display_caps();
    for balanced in [false, true] {
        let mut data = balanced_six_stream_data();
        balance(&mut data, balanced);
        let mut p = data["document"]["blocks"][0]["blocks"][0].clone();
        p["children"] = p["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["kind"] == "text")
            .cloned()
            .collect::<Vec<_>>()
            .into();
        assert!(!p["children"].as_array().unwrap().is_empty());
        let span = p["span"].clone();
        for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
            m["header"] = json!({"x":400*65536,"y":5*65536,"width":180*65536,"height":60*65536});
            m["footer"] = json!({"x":400*65536,"y":220*65536,"width":180*65536,"height":60*65536});
            for role in ["header_content", "footer_content"] {
                m[role] = json!({"node_id":0,"span":span,"blocks":[p.clone()]});
            }
        }
        let mut next = 0;
        semantic_renumber(&mut data["document"], &mut next);
        for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
            for role in ["header_content", "footer_content"] {
                semantic_renumber(&mut m[role], &mut next);
            }
        }
        let root = Root::new();
        let input = vector_input(&root, data, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        navigation_drive(&input, &caps, MODE, &mut budget, 4, |bundle, _, _| {
            let d = bundle.display();
            assert!(
                !d.text().draws().is_empty()
                    && !d.math().draws().is_empty()
                    && !d.images().draws().is_empty()
                    && !d.numbers().draws().is_empty()
                    && !d.markers().draws().is_empty()
                    && !d.markers().separators().is_empty()
            );
            assert!(d
                .text()
                .draws()
                .iter()
                .any(|t| t.definition_index().is_some()));
            assert_eq!(
                d.source()
                    .source()
                    .geometry()
                    .pages()
                    .iter()
                    .any(|p| p.selection().balance().is_some()),
                balanced
            );
            let mut page = 0;
            let mut regions_started = false;
            for (i, paint) in d.paints().iter().enumerate() {
                let current = if let Paint::Image(index) = paint {
                    d.images().draws()[*index]
                        .fragment()
                        .fragment()
                        .page_index()
                } else {
                    d.text_paint(i).unwrap().page_index()
                };
                if current != page {
                    assert!(current > page);
                    page = current;
                    regions_started = false;
                }
                if matches!(paint, Paint::PageRegionText { .. }) {
                    regions_started = true;
                } else {
                    assert!(
                        !regions_started,
                        "body paints must precede the same page's running glyphs"
                    );
                }
            }
            assert!(regions_started);
        })
        .unwrap();
    }
}

#[test]
fn book_v2_column_regions_join_authentic_receipts_and_reject_missing_duplicate_foreign_order_page_epoch_limits(
) {
    let caps = display_caps();
    let root = Root::new();
    let data = region_data(2, true, false);
    let input = display_input(&root, data.clone(), None, &caps);
    let foreign_root = Root::new();
    let foreign = display_input(&foreign_root, data, None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    math_drive(&input, &caps, MODE, &mut budget, |source, _, _| {
        let epoch = source
            .original_graph()
            .lines()
            .prepared()
            .shaped()
            .binding_epoch();
        let pages: Vec<_> = source
            .source()
            .geometry()
            .pages()
            .iter()
            .enumerate()
            .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
            .collect();
        let fresh = || {
            BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                .unwrap()
                .build()
                .unwrap()
        };
        let (regions, proofs) = original_regions(&input, &caps, epoch, &pages);
        for (r, p) in regions.iter().zip(&proofs) {
            assert_projection(r, p);
        }
        let joined = fresh()
            .with_page_regions(regions, &caps, MAXIMUM, 0, 0)
            .unwrap();
        assert_eq!(joined.page_regions().len(), proofs.len());
        for (r, p) in joined.page_regions().iter().zip(&proofs) {
            assert_projection(r, p);
        }
        for mode in [
            "missing",
            "duplicate",
            "extra",
            "order",
            "page",
            "foreign",
            "epoch",
            "limits",
            "rejoin",
        ] {
            let (mut regions, _) = original_regions(
                if mode == "foreign" { &foreign } else { &input },
                &caps,
                if mode == "epoch" { [29; 32] } else { epoch },
                &pages,
            );
            match mode {
                "missing" => {
                    regions.remove(0);
                }
                "duplicate" => {
                    let (mut more, _) = original_regions(&input, &caps, epoch, &pages);
                    regions[1] = more.remove(0);
                }
                "extra" => {
                    let (mut more, _) = original_regions(&input, &caps, epoch, &pages);
                    regions.push(more.remove(0));
                }
                "order" => regions.swap(0, 1),
                "page" => {
                    let mut other = pages.clone();
                    other[0].0 = 1;
                    regions = original_regions(&input, &caps, epoch, &other).0;
                }
                _ => {}
            }
            let mut records = 0;
            let mut work = 0;
            let changed = changed_caps(&caps, |c| c.max_layout_passes -= 1);
            let display = if mode == "rejoin" {
                fresh()
                    .with_page_regions(
                        original_regions(&input, &caps, epoch, &pages).0,
                        &caps,
                        MAXIMUM,
                        0,
                        0,
                    )
                    .unwrap()
            } else {
                fresh()
            };
            let floor = (display.record_charge(), display.work_steps());
            let error = display
                .with_page_regions_counted(
                    regions,
                    if mode == "limits" { &changed } else { &caps },
                    MAXIMUM,
                    floor.0 + 7,
                    floor.1 + 11,
                    &mut records,
                    &mut work,
                )
                .err()
                .unwrap();
            assert!(
                format!("{error:?}").contains("ReceiptMismatch"),
                "{mode}: {error:?}"
            );
            assert!(records >= floor.0 + 7 && work >= floor.1 + 11, "{mode}");
        }
    })
    .unwrap();
}

#[test]
fn book_v2_column_regions_keep_compact_paints_source_glyphs_and_page_artifact_ownership() {
    assert_eq!(std::mem::size_of::<Paint>(), 16);
    let caps = display_caps();
    for columns in [2, 3] {
        for blank in [false, true] {
            for empty in [false, true] {
                let root = Root::new();
                let input = display_input(&root, region_data(columns, blank, empty), None, &caps);
                let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
                navigation_drive(&input,&caps,MODE,&mut budget,4,|bundle,_,observed|{
            let display=bundle.display();let pages=display.source().source().geometry().pages();
            assert!(pages.len()>1);assert_eq!(display.page_regions().len(),2*pages.len());
            if blank {assert!(pages[0].fragments().is_empty());}
            let mut expected=Vec::new();let mut region_owners=BTreeSet::new();
            for (index,r) in display.page_regions().iter().enumerate(){
                assert_eq!(r.page_index(),(index/2)as u32);assert_eq!(r.kind(),if index%2==0{RegionKind::Header}else{RegionKind::Footer});
                for d in r.draws(){region_owners.insert(d.owner());expected.push((index,r,d));}
                if empty {assert!(r.draws().is_empty());}
            }
            let mut actual=0;let mut region_gids=BTreeSet::new();let mut body_gids=BTreeSet::new();
            for (paint,p) in display.paints().iter().copied().enumerate(){
                if let Paint::PageRegionText{region_draw}=p {
                    assert_eq!(region_draw,actual);let(index,r,d)=expected[actual];actual+=1;
                    let semantic=display.semantic_paint(paint).unwrap().unwrap();assert_eq!(semantic.fragment_index(),index);assert_eq!(semantic.owner(),Some(d.owner()));assert_eq!(semantic.artifact(),Some(if r.kind()==RegionKind::Header{BookV2SemanticArtifact::RunningHeader}else{BookV2SemanticArtifact::RunningFooter}));assert!(semantic.alternative().is_none()&&semantic.actual_text().is_none());
                    assert_eq!(display.text_paint(paint).unwrap().page_index(),r.page_index());assert!(display.navigation_paint_bounds(paint).is_err());assert!(display.image_use(paint).unwrap().is_none());
                    let u=display.font_use(paint,0).unwrap().unwrap();assert_eq!(u.paint(),p);assert_eq!(u.size(),d.font().size());assert!(matches!(u.text(),typaxis_display_list::book_v2::BookV2FontUseText::Text(s)if s==d.exact_text()));assert_eq!((0..u.glyphs().len()).map(|i|u.glyphs().get(i).unwrap()).collect::<Vec<_>>(),d.glyphs().iter().map(|g|g.original_gid()).collect::<Vec<_>>());
                    for glyph in d.glyphs(){region_gids.insert(glyph.original_gid());assert!(glyph.x().raw()>=r.frame().x().raw());assert!(glyph.y().raw()>=r.frame().y().raw());}
                }else if let Some(u)=display.font_use(paint,0).unwrap(){for i in 0..u.glyphs().len(){body_gids.insert(u.glyphs().get(i).unwrap());}}
            }
            assert_eq!(actual,expected.len());if !empty {assert!(region_gids.difference(&body_gids).next().is_some());}
            let tree=bundle.relations();let source=tree.source();assert!(source.nodes().iter().all(|n|!region_owners.contains(&n.source().key().owner())));
            let scopes=bundle.marked().content().source();for(index,g)in scopes.groups().iter().enumerate(){if matches!(g.artifact(),Some(typaxis_pdf::book_v2::BookV2MarkedArtifact::RunningHeader|typaxis_pdf::book_v2::BookV2MarkedArtifact::RunningFooter)){assert!(g.mcid().is_none());assert!(source.for_group(index).is_none());}}
            assert!(observed.line_reshape_passes()>0);assert_eq!(observed.output_charge(),bundle.relations_prefix().output_charge());
            let requests:Vec<_>=pages.iter().enumerate().map(|(i,p)|(i as u32,p.selection().named_page().map(str::to_owned))).collect();
            let (_,proofs)=original_regions(&input,&caps,display.source().original_graph().lines().prepared().shaped().binding_epoch(),&requests);
            for (r,p) in display.page_regions().iter().zip(&proofs){assert_projection(r,p);}
            region_probes::emit_nav(&format!("regions-{columns}-{blank}-{empty}"),bundle,&caps,observed,&proofs);
        }).unwrap();
                assert!(budget.observation().line_reshape_passes() > 0);
            }
        }
    }
}
