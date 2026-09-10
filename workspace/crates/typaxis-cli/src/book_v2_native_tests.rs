use super::*;
#[path = "book_v2_header_math_terminal_tests.rs"]
mod header_terminals;
#[path = "book_v2_description_math_tests.rs"]
mod descriptions;
use typaxis_core::{Length, NodeId, PositiveLength};
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, compute_book_v2_native_math, layout_book_v2_inline_lines,
    prepare_book_v2_inline_items, prepare_book_v2_inline_items_with_native_context,
};
use typaxis_layout::{
    ProductionNativeMathComputationError, ProductionPlacedInline, StagingMathLayoutError,
};
use typaxis_linebreak::JapaneseLineBreakMode;
const NATIVE_SOURCE: &[u8] = b"Result x^{2} Proofx+1";
const NATIVE_BODY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/profiles/production-book-1/combined/job/body.ttf"
));

#[test]
fn book_v2_native_fraction_display_preserves_rule_and_glyph_paints() {
    check_fraction_display(false);
}
#[test]
fn book_v2_native_table_headers_keep_repeated_fraction_paints() {
    check_fraction_display(true);
}
fn check_fraction_display(repeated: bool) {
    let root=Root::new();
    let limits=limits();
    let mut source=b"Result \\frac{x}{1} Proof\\frac{x}{2}".to_vec();
    let inline_end=7+b"\\frac{x}{1}".len();
    let display_start=inline_end+6;
    let mut data=native_data();
    fn rebase(value: &mut Value, inline_end: usize, display_start: usize, end: usize) {
        match value {
            Value::Object(fields) => for (key,value) in fields {
                if matches!(key.as_str(),"start_byte"|"end_byte") {
                    *value=match value.as_u64().unwrap() {
                        12=>inline_end.into(),18=>display_start.into(),21=>end.into(),other=>other.into(),
                    };
                } else { rebase(value,inline_end,display_start,end); }
            },
            Value::Array(values)=>for value in values {rebase(value,inline_end,display_start,end);},
            _=>(),
        }
    }
    rebase(&mut data,inline_end,display_start,source.len());
    data["text_buffers"][0]["utf8"]=std::str::from_utf8(&source).unwrap().into();
    data["document"]["blocks"][0]["blocks"][0]["children"][1]["speech"]="x over one".into();
    data["document"]["blocks"][0]["blocks"][1]["speech"]="x over two".into();
    if repeated {
        let paragraph=data["document"]["blocks"][0]["blocks"][0].clone();
        let mut display=data["document"]["blocks"][0]["blocks"][1].clone();
        let span=|start,end|json!({"source_id":0,"start_byte":start,"end_byte":end});
        let text_span=|start,end|json!({"text_id":0,"start_byte":start,"end_byte":end});
        let row=|block:Value| {let span=block["span"].clone();json!({"node_id":0,"span":span,"cells":[{"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[block]}]})};
        source=b"\\frac{x}{2}".to_vec();
        display["span"]=span(0,source.len());
        display["math_source"]["text_span"]=text_span(0,source.len());
        let mapping=|start,end|json!({"kind":"identity","source_span":span(start,end),"text_range":{"start_byte":start,"end_byte":end}});
        let mut mappings=vec![mapping(0,source.len())];
        let mut rows=Vec::new();
        for _ in 0..12 {
            let start=source.len();
            source.extend_from_slice(b"Result \\frac{x}{1} Proof");
            let mut block=paragraph.clone();
            block["span"]=span(start,source.len());
            for (index,(left,right)) in [(start,start+7),(start+7,start+18),(start+18,source.len())].into_iter().enumerate() {
                let child=&mut block["children"][index];
                child["span"]=span(left,right);
                if index==1 {child["math_source"]["text_span"]=text_span(left,right);} else {child["text_span"]=text_span(left,right);}
                mappings.push(mapping(left,right));
            }
            rows.push(row(block));
        }
        data["text_buffers"][0]["utf8"]=std::str::from_utf8(&source).unwrap().into();
        data["text_buffers"][0]["mappings"]=mappings.into();
        let span=span(0,source.len());
        data["document"]["blocks"][0]["span"]=span.clone();
        data["document"]["blocks"][0]["blocks"]=json!([{"kind":"table","node_id":0,"span":span,"classes":[],"columns":[{"kind":"fraction","weight":1}],"head":[row(display)],"body":rows}]);
        fn renumber(value:&mut Value,next:&mut u32) {
            if value.get("node_id").is_some() {value["node_id"]=(*next).into();*next+=1;}
            for key in ["blocks","children","head","body","cells"] {
                if let Some(values)=value.get_mut(key).and_then(Value::as_array_mut) {
                    for child in values {renumber(child,next);}
                }
            }
        }
        renumber(&mut data["document"],&mut 0);
    }
    let height=if repeated {6_000_000} else {20_000_000};
    let master=&mut data["page_masters"]["masters"][0];
    master["width"]=22_000_000.into();master["height"]=24_000_000.into();
    master["trim"]=json!({"x":0,"y":0,"width":22_000_000,"height":24_000_000});
    master["body"]=json!({"x":500_000,"y":500_000,"width":20_000_000,"height":height});
    let input=native_input_with_source(&root,data,&source,&limits);
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input, &limits, JapaneseLineBreakMode::Normal, 100_000_000,
        |pdf, observation| {
            assert_eq!(observation.candidate_passes(), 1);
            let terminals = pdf.navigation().source().source().source().source().display().source();
            assert_math_terminals(terminals);
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            assert_eq!(terminals.source().semantic_math(), if repeated { 13 } else { 2 });
        },
    ).unwrap();
    let nav=prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow=prepare_book_v2_text_flow(input.body().styled(),&nav).unwrap();
    let policy=prepare_book_v2_resource_policy(input.body(),&limits).unwrap();
    let bindings=bind_book_v2_vectors(&policy,input.resources(),&limits).unwrap();
    let raw=|n|Length::from_raw(n).unwrap();
    let rect=typaxis_core::Rect::new(raw(500_000),raw(500_000),PositiveLength::new(raw(20_000_000)).unwrap(),PositiveLength::new(raw(height)).unwrap());
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,rect,1_000_000,|lines| {
        let measured=typaxis_pagination::book_v2::prepare_book_v2_table_measurements(
            typaxis_pagination::book_v2::prepare_book_v2_body_flow(lines.lines(),None,lines.footnotes(),&limits,0).unwrap(),&limits).unwrap();
        let mut search=typaxis_pagination::book_v2::prepare_book_v2_table_body_search(&measured,&limits,1_000_000,0).unwrap();
        let stable=search.select_stable_mixed_pages(2).unwrap();
        let placed=search.place_mixed_pages(stable.sequence()).unwrap();
        let closure=search.close_mixed_page_sources(&stable,&placed).unwrap();
        let terminals=search.finalize_mixed_page_math(closure,&limits,0).unwrap();
        assert_math_terminals(&terminals);
        assert_math_display(&terminals,input.resources(),&limits);
        if repeated {
            assert!(placed.pages().len()>2);
            assert_eq!(terminals.source().semantic_math(),13);
            assert_eq!(terminals.source().repeated_math(),placed.pages().len()-1);
        } else {assert_eq!(terminals.terminals().len(),2);}
        for terminal in terminals.terminals() {
            let typaxis_pagination::book_v2::BookV2BodyMathSource::Native(receipt)=terminal.source() else {panic!("not native");};
            assert_eq!(receipt.computation().paints().iter().filter(|p|matches!(p,typaxis_math::MathPaint::Rule(_))).count(),1);
        }
    }).unwrap();
}
pub(in crate::book_v2_resources::tests::shaping_tests) fn native_data() -> Value {
    let mut d = source_data(std::str::from_utf8(NATIVE_SOURCE).unwrap());
    d["text_buffers"][0]["mappings"]=Value::Array([(0,7),(7,12),(12,18),(18,21)].into_iter().map(|(s,e)|json!({"kind":"identity","source_span":{"source_id":0,"start_byte":s,"end_byte":e},"text_range":{"start_byte":s,"end_byte":e}})).collect());
    d["document"]["blocks"][0]["blocks"][0]["children"] = json!([
        {"kind":"text","node_id":3,"span":{"source_id":0,"start_byte":0,"end_byte":7},"text_span":{"text_id":0,"start_byte":0,"end_byte":7}},
        {"kind":"inline_math","node_id":4,"span":{"source_id":0,"start_byte":7,"end_byte":12},"math_source":{"language":"typaxis-math","version":"1","text_span":{"text_id":0,"start_byte":7,"end_byte":12}},"speech":"x squared"},
        {"kind":"text","node_id":5,"span":{"source_id":0,"start_byte":12,"end_byte":18},"text_span":{"text_id":0,"start_byte":12,"end_byte":18}}
    ]);
    d["document"]["blocks"][0]["blocks"].as_array_mut().unwrap().push(json!({"kind":"display_math","node_id":6,"classes":[],"span":{"source_id":0,"start_byte":18,"end_byte":21},"math_source":{"language":"typaxis-math","version":"1","text_span":{"text_id":0,"start_byte":18,"end_byte":21}},"speech":"x plus one"}));
    let mut rule = d["style_sheet"]["rules"][2].clone();
    rule["selector"] = "display_math".into();
    rule["style_id"] = "native-display".into();
    rule["source_order"] = 3.into();
    d["style_sheet"]["rules"].as_array_mut().unwrap().push(rule);
    d["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(NATIVE_BODY)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    d
}
pub(in crate::book_v2_resources::tests::shaping_tests) fn native_input(
    root: &Root,
    data: Value,
    limits: &M4EffectiveResourceLimits,
) -> PreparedBookV2Resources {
    native_input_with_source(root, data, NATIVE_SOURCE, limits)
}
pub(in crate::book_v2_resources::tests::shaping_tests) fn native_input_with_source(root: &Root, data: Value, source: &[u8], limits: &M4EffectiveResourceLimits) -> PreparedBookV2Resources {
    let body = body_with_source(root, data, source, limits);
    fs::write(root.0.join("body.bin"), NATIVE_BODY).unwrap();
    prepare_book_v2_resources(
        body,
        &root.context(),
        &config_with_extension(
            limits.base().get().clone(),
            limits.extension().get().clone(),
        ),
        limits,
    )
    .unwrap()
}
#[test]
fn book_v2_native_math_keeps_actual_font_source_and_reuses_computation_in_lines() {
    let root = Root::new();
    let limits = limits();
    let input = native_input(&root, native_data(), &limits);
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let native = compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0)
        .unwrap()
        .unwrap();
    assert_eq!(native.receipts().len(), 2);
    assert_eq!(native.display_blocks().len(), 1);
    assert!(native.layout_work() > 0);
    assert!(native.record_charge() > 0);
    assert!(native.spool_charge() > 0);
    for (i, receipt) in native.receipts().iter().enumerate() {
        assert!(std::ptr::eq(
            receipt.source(),
            &input.body().styled().body().math()[i]
        ));
        assert_eq!(receipt.font_sha256(), typaxis_core::sha256(NATIVE_BODY));
        assert_eq!(receipt.font_face_id(), FontFaceId::new(0));
        assert_eq!(receipt.font_instance_id().get(), 0);
        assert_eq!(
            receipt.computation().parsed_fingerprint(),
            receipt.source().parsed().fingerprint()
        );
        assert!(receipt.computation().layout_work() > 0);
    }
    let display = &native.display_blocks()[0];
    assert_eq!(display.owner(), NodeId::new(6));
    assert!(display.height().get().raw() > 0);
    assert!(display.width().get().raw() > 0);
    assert!(display.baseline().raw() > 0);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let shaped = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    assert_eq!(
        shaped.font_instances().fingerprint(),
        native.font_instances().fingerprint()
    );
    let prepared = prepare_book_v2_inline_items_with_native_context(
        &flow,
        &shaped,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        Some(&native),
    )
    .unwrap();
    assert!(std::ptr::eq(prepared.native_math().unwrap(), &native));
    let text_width = shaped.paragraphs()[0]
        .runs()
        .iter()
        .flat_map(|r| &r.glyph_run().glyphs)
        .map(|g| g.advance_x.raw())
        .sum::<i64>();
    let math_width = native.receipts()[0].computation().dimensions().advance();
    let width =
        PositiveLength::new(Length::from_raw((text_width + math_width) * 2 / 3).unwrap()).unwrap();
    let selected = layout_book_v2_inline_lines(&prepared, &[width], 1_000_000).unwrap();
    let body = typaxis_core::Rect::new(
        Length::ZERO,
        Length::ZERO,
        PositiveLength::new(
            width
                .get()
                .checked_add(Length::from_raw(9).unwrap())
                .unwrap(),
        )
        .unwrap(),
        width,
    );
    let framed =
        typaxis_layout::book_v2::layout_book_v2_body_inline_lines(&prepared, body, 1_000_000)
            .unwrap();
    let frames = framed.frames().unwrap();
    assert_eq!(frames.paragraphs()[0].width(), width);
    assert_eq!(
        selected.output_records() + frames.record_charge(),
        framed.output_records()
    );
    assert_eq!(selected.candidate_steps(), framed.candidate_steps());
    let selected = framed;
    assert!(selected.paragraphs()[0].lines().len() > 1);
    let mut actual = String::new();
    let mut count = 0;
    for line in selected.paragraphs()[0].lines() {
        for item in line.items() {
            match item {
                ProductionPlacedInline::Text(t) => actual.push_str(t.utf8()),
                ProductionPlacedInline::BookV2Math(m) => {
                    count += 1;
                    assert!(std::ptr::eq(m.receipt(), &native.receipts()[0]));
                    assert_eq!(m.source_span(), native.receipts()[0].source().domain().span);
                    assert_eq!(m.baseline(), line.baseline());
                }
                _ => panic!("wrong native namespace"),
            }
        }
    }
    assert_eq!(actual, "Result  Proof");
    assert_eq!(count, 1);
    assert!(selected.output_records() > native.record_charge());
    typaxis_layout::book_v2::with_converged_book_v2_body_lines_with_native_context(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body,
        1_000_000,
        Some(&native),
        |stable| {
            assert!(stable.passes().last().unwrap().is_stable());
            let body_flow = typaxis_pagination::book_v2::prepare_book_v2_body_flow(
                stable.lines(),
                None,
                stable.footnotes(),
                &limits,
                0,
            )
            .unwrap();
            let block = &stable
                .lines()
                .prepared()
                .native_math()
                .unwrap()
                .display_blocks()[0];
            let item = body_flow
                .body_items()
                .iter()
                .find(|i| {
                    matches!(
                        i.source(),
                        Some(
                            typaxis_pagination::ProductionBodyFragmentSource::NativeMathBlock {
                                block_index: 0
                            }
                        )
                    )
                })
                .unwrap();
            assert_eq!(item.owner(), block.owner());
            assert_eq!(item.height(), block.height().get());
            assert!(item.viewport_left().is_some());
            let measured=typaxis_pagination::book_v2::prepare_book_v2_table_measurements(body_flow,&limits).unwrap();
            let mut pages=typaxis_pagination::book_v2::prepare_book_v2_table_body_search(&measured,&limits,1_000_000,0).unwrap();
            let stable_pages=pages.select_stable_mixed_pages(2).unwrap();
            let placed=pages.place_mixed_pages(stable_pages.sequence()).unwrap();
            let closure=pages.close_mixed_page_sources(&stable_pages,&placed).unwrap();
            assert_eq!(closure.semantic_math(),2);
            assert_eq!(closure.repeated_math(),0);
            let terminals=pages.finalize_mixed_page_math(closure,&limits,0).unwrap();
            assert_math_terminals(&terminals);
            assert_math_display(&terminals,input.resources(),&limits);
            assert_eq!(terminals.terminals().len(),2);
            assert_eq!(terminals.spool_charge(),native.spool_charge()+terminals.canonical_bytes().len() as u64);
            let native_placed=placed.pages().iter().flat_map(|p|p.fragments()).find(|f|f.fragment().owner()==block.owner()).unwrap().fragment();
            assert_eq!(native_placed.viewport().unwrap().width(),block.width());
            assert_eq!(native_placed.viewport().unwrap().height(),block.height());
            assert_eq!(native_placed.baseline(),Some(native_placed.bounds().y().checked_add(block.baseline()).unwrap()));


            assert!(std::ptr::eq(
                stable.lines().prepared().native_math().unwrap(),
                &native
            ));
            let mut found = 0;
            for line in stable.lines().paragraphs()[0].lines() {
                for item in line.items() {
                    if let ProductionPlacedInline::BookV2Math(m) = item {
                        found += 1;
                        assert!(std::ptr::eq(m.receipt(), &native.receipts()[0]));
                        assert_eq!(m.baseline(), line.baseline());
                    }
                }
            }
            assert_eq!(found, 1);
        },
    )
    .unwrap();
    let contexts = selected.selected_line_contexts().unwrap();
    assert_eq!(contexts.paragraphs()[0].ends().last().copied(), Some(16));
    let views = contexts
        .paragraphs()
        .iter()
        .map(|p| ProductionParagraphLineContext {
            owner: p.owner(),
            ends: p.ends(),
        })
        .collect::<Vec<_>>();
    let reshaped = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        Some(&views),
    )
    .unwrap();
    let again = prepare_book_v2_inline_items_with_native_context(
        &flow,
        &reshaped,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        Some(&native),
    )
    .unwrap();
    assert!(std::ptr::eq(again.native_math().unwrap(), &native));
    let owned = prepare_book_v2_inline_items(
        &flow,
        &shaped,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    assert_eq!(
        owned.native_math().unwrap().fingerprint(),
        native.fingerprint()
    );
    assert!(prepare_book_v2_inline_items_with_native_context(
        &flow,
        &shaped,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        None
    )
    .is_err());
    let other_bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    assert!(native
        .verify(&other_bindings, input.resources(), &limits)
        .is_err());
    assert!(prepare_book_v2_inline_items_with_native_context(
        &flow,
        &shaped,
        input.resources(),
        &other_bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        Some(&native)
    )
    .is_err());
}
#[test]
fn book_v2_native_math_preflights_cumulative_records_spool_and_missing_math_font() {
    let root = Root::new();
    let limits = limits();
    let input = native_input(&root, native_data(), &limits);
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let native = compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0)
        .unwrap()
        .unwrap();
    let prior = limits.base().get().max_fragments - native.record_charge();
    assert_eq!(
        compute_book_v2_native_math(&bindings, input.resources(), &limits, prior, 0)
            .unwrap()
            .unwrap()
            .record_charge(),
        limits.base().get().max_fragments
    );
    assert!(matches!(
        compute_book_v2_native_math(&bindings, input.resources(), &limits, prior + 1, 0)
            .unwrap_err(),
        ProductionNativeMathComputationError::RecordLimit
    ));
    let prior = limits.base().get().max_spool_bytes - native.spool_charge();
    assert_eq!(
        compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, prior)
            .unwrap()
            .unwrap()
            .spool_charge(),
        limits.base().get().max_spool_bytes
    );
    assert!(matches!(
        compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, prior + 1)
            .unwrap_err(),
        ProductionNativeMathComputationError::SpoolLimit
    ));
    for (work_limit, success) in [
        (native.layout_work(), true),
        (native.layout_work() - 1, false),
    ] {
        let root = Root::new();
        let mut extension = M4ResourceLimits::default();
        extension.max_math_layout_units = work_limit;
        let bounded = M4EffectiveResourceLimits::new(limits.base().clone(), extension).unwrap();
        let input = native_input(&root, native_data(), &bounded);
        let policy = prepare_book_v2_resource_policy(input.body(), &bounded).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &bounded).unwrap();
        let result = compute_book_v2_native_math(&bindings, input.resources(), &bounded, 0, 0);
        if success {
            assert_eq!(result.unwrap().unwrap().layout_work(), work_limit);
        } else {
            assert!(matches!(
                result.unwrap_err(),
                ProductionNativeMathComputationError::Math(StagingMathLayoutError::LayoutUnitLimit)
            ));
        }
    }
    let r = Root::new();
    let mut data = native_data();
    data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(FONT)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    let body = body_with_source(&r, data, NATIVE_SOURCE, &limits);
    let bad = prepare_book_v2_resources(
        body,
        &r.context(),
        &config_with_extension(
            limits.base().get().clone(),
            limits.extension().get().clone(),
        ),
        &limits,
    )
    .unwrap();
    let policy = prepare_book_v2_resource_policy(bad.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, bad.resources(), &limits).unwrap();
    assert!(
        matches!(compute_book_v2_native_math(&bindings,bad.resources(),&limits,0,0).unwrap_err(),ProductionNativeMathComputationError::Math(StagingMathLayoutError::InvalidMathFont(owner)) if owner==NodeId::new(4))
    );
}

#[path = "book_v2_native_source_width_tests.rs"]
mod source_widths;
