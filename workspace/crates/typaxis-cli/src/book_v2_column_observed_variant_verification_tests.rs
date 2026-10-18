//! Independent P2 controls on actual Native roots and simultaneously owned
//! variants. These measure accepted requested storage and bounded stage work;
//! they do not assert allocator capacities or full shaping-engine accounting.
use super::super::super as driver;
use super::*;
use typaxis_core::{Length, NodeId, PositiveLength};
use typaxis_layout::book_v2::{
    compute_book_v2_native_math_observed_counted,
    prepare_book_v2_inline_items_with_observed_native_source_counted,
    prepare_budgeted_book_v2_column_line_variant_seed,
    prepare_observed_book_v2_column_line_variant_seed,
    with_budgeted_rebuilt_book_v2_column_line_variants,
    with_observed_rebuilt_book_v2_column_line_variant,
    with_observed_rebuilt_book_v2_column_line_variants, BookV2InlineLineLayout,
    BookV2LineVariantBudget, BookV2ObservedLineVariantBudget, BookV2ObservedVariantObservation,
    BookV2ObservedVariantPhase as Phase, BookV2SourceWidthAssignments,
};
use typaxis_shaping::book_v2::shape_book_v2_authored_text_with_observed_source_counted;
use typaxis_shaping::book_v2::{
    shape_book_v2_authored_text, shape_book_v2_authored_text_for_observed_request_counted,
    with_book_v2_observed_source_accounting, BookV2ObservedAccountingError as A,
    BookV2ObservedShapeError, BookV2ObservedSourceAccounting,
};
use typaxis_syntax::book_v2::{
    BookV2ObservedSourceFlowError as S, BookV2ObservedSourceFlowObservation as SourceObservation,
};
#[path = "book_v2_column_observed_adoption_tests.rs"]
mod observed_adoption;
const PRIOR: [u64; 3] = [7, 11, 13];
const NATIVE_SOURCE: &str = "Result \\frac{x}{1} Proof\\frac{x}{2}";
const MATH_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/profiles/production-book-1/combined/job/body.ttf"
));

fn native_data() -> Value {
    let mut d = data(false);
    let ie = 7 + "\\frac{x}{1}".len();
    let ds = ie + 6;
    let end = NATIVE_SOURCE.len();
    let span = |a, b| json!({"source_id":0,"start_byte":a,"end_byte":b});
    let text = |a, b| json!({"text_id":0,"start_byte":a,"end_byte":b});
    d["text_buffers"] = json!([{"text_id":0,"utf8":NATIVE_SOURCE,"mappings":
        ([(0,7),(7,ie),(ie,ds),(ds,end)].map(|(a,b)|json!({"kind":"identity","source_span":span(a,b),"text_range":{"start_byte":a,"end_byte":b}})))}]);
    d["document"]["blocks"] = json!([{"kind":"semantic_container","node_id":1,"classes":[],"semantic_kind":"solution","anchor_id":null,"span":span(0,end),"blocks":[
        {"kind":"paragraph","node_id":2,"classes":[],"span":span(0,ds),"children":[
            {"kind":"text","node_id":3,"span":span(0,7),"text_span":text(0,7)},
            {"kind":"inline_math","node_id":4,"span":span(7,ie),"math_source":{"language":"typaxis-math","version":"1","text_span":text(7,ie)},"speech":"x over one"},
            {"kind":"text","node_id":5,"span":span(ie,ds),"text_span":text(ie,ds)}]},
        {"kind":"display_math","node_id":6,"classes":[],"span":span(ds,end),"math_source":{"language":"typaxis-math","version":"1","text_span":text(ds,end)},"speech":"x over two"}]}]);
    let mut display = d["style_sheet"]["rules"][2].clone();
    display["selector"] = "display_math".into();
    display["style_id"] = "p1-native-display".into();
    display["source_order"] = 4.into();
    d["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(display);
    // This is the original genuine fraction fixture's tall body, split into
    // actual columns. The source and original authored resource caps are kept.
    d["page_masters"]["masters"][0]["body"]["height"] = (240 * 65536).into();
    d
}

fn native_input(
    root: &Root,
    mut d: Value,
    limits: &M4EffectiveResourceLimits,
    font: &[u8],
) -> PreparedBookV2Resources {
    let source = d["text_buffers"][0]["utf8"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let hex = |bytes: &[u8]| {
        typaxis_core::sha256(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    d["sources"][0]["sha256"] = hex(&source).into();
    d["sources"][0]["utf8_byte_length"] = source.len().into();
    d["resources"]["font_faces"][0]["expected_sha256"] = hex(font).into();
    fs::write(
        root.0.join("document-package.json"),
        serde_json::to_vec(&d).unwrap(),
    )
    .unwrap();
    fs::write(root.0.join("input.tsf"), &source).unwrap();
    fs::write(root.0.join("body.bin"), font).unwrap();
    fs::write(root.0.join("collection.bin"), COLLECTION).unwrap();
    fs::write(root.0.join("cover.bin"), PNG).unwrap();
    let (session, raw) = HostBookV2InputSession::open(
        MachineInputHostOptions::new(
            HostPath::new(root.0.join("document-package.json")).unwrap(),
            None,
        ),
        limits.base(),
    )
    .unwrap();
    let decoded = session
        .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(limits.base()))
        .unwrap();
    let sources = session.admit_sources(&decoded, limits.base()).unwrap();
    let body = prepare_admitted_book_v2_body(
        session.finish(raw, decoded, sources).unwrap(),
        limits.base(),
    )
    .unwrap();
    let prepared = crate::book_v2_resources::prepare_book_v2_resources(
        body,
        &root.context(),
        &config(limits),
        limits,
    )
    .unwrap();
    assert_eq!(prepared.body().styled().body().limits(), limits.base());
    assert_eq!(
        prepared.body().styled().body().wire().text_buffers()[0]
            .utf8
            .as_bytes(),
        source
    );
    assert_eq!(prepared.resources().fonts()[0].bytes(), font);
    assert_eq!(prepared.resources().fonts()[1].bytes(), COLLECTION);
    assert_eq!(prepared.resources().images()[0].bytes(), PNG);
    let canonical = typaxis_document_package::book_v2::BookV2DocumentPackageEncoder::new()
        .encode(prepared.body().styled().body().wire())
        .unwrap();
    assert_eq!(
        prepared.body().styled().body().canonical_jcs_sha256(),
        typaxis_core::sha256(canonical.as_bytes())
    );
    prepared
}

fn loan<'cells, R>(
    cells: &'cells mut [u64; 3],
    limits: &'cells ValidatedResourceLimits,
    max_r: u64,
    max_w: u64,
    inspect: impl for<'id> FnOnce(&mut BookV2ObservedSourceAccounting<'id, 'cells>) -> R,
) -> R {
    let [r, s, w] = cells;
    with_book_v2_observed_source_accounting(r, s, w, limits, max_r, max_w, inspect)
}
fn triple(o: SourceObservation) -> [u64; 3] {
    [o.record_charge(), o.spool_charge(), o.work_steps()]
}
fn verify_oracle(
    flow: &PreparedBookV2TextFlow<'_>,
    body: &StyledBookV2Body,
    nav: &PreparedBookV2Navigation<'_>,
    prior: [u64; 3],
    max_r: u64,
    max_w: u64,
) -> (Result<(), S>, [u64; 3]) {
    let mut observed = SourceObservation::default();
    let result = flow.verify_for_observed_counted(
        body,
        nav,
        prior[0],
        prior[1],
        prior[2],
        max_r,
        max_w,
        &mut observed,
    );
    (result, triple(observed))
}

fn variant_data(native: bool, rich: bool) -> Value {
    let mut d = native_data();
    let (start, end) = if native {
        // Added paragraphs follow the original display math in source order.
        // Keep the original 35-byte math source and its four mappings intact.
        let start = NATIVE_SOURCE.len();
        let end = start + 7;
        d["text_buffers"][0]["utf8"] = format!("{NATIVE_SOURCE}Result ").into();
        d["text_buffers"][0]["mappings"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"identity","source_span":{"source_id":0,"start_byte":start,"end_byte":end},"text_range":{"start_byte":start,"end_byte":end}}));
        d["document"]["blocks"][0]["span"]["end_byte"] = end.into();
        (start, end)
    } else {
        (0, 7)
    };
    let span = json!({"source_id":0,"start_byte":start,"end_byte":end});
    let text = json!({"kind":"text","node_id":0,"span":span,"text_span":{"text_id":0,"start_byte":start,"end_byte":end}});
    let paragraph = |count| {
        json!({"kind":"paragraph","node_id":0,"classes":[],"span":span,
        "children":vec![text.clone();count]})
    };
    if !native {
        d["document"]["blocks"][0]["blocks"] = json!([paragraph(16)]);
    } else {
        d["document"]["blocks"][0]["blocks"]
            .as_array_mut()
            .unwrap()
            .push(paragraph(16));
    }
    if rich {
        let mut p = paragraph(4);
        p["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"variant-note"}));
        let row = |count| json!({"node_id":0,"span":span,"cells":[{"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[paragraph(count)]}]});
        let table = json!({"kind":"table","node_id":0,"classes":[],"span":span,
            "columns":[{"kind":"fraction","weight":1}],"caption":[paragraph(2)],
            "head":[row(5)],"body":[row(10),row(8)]});
        d["document"]["blocks"][0]["blocks"]
            .as_array_mut()
            .unwrap()
            .extend([p, table]);
        d["document"]["footnotes"] =
            json!([{"node_id":0,"span":span,"footnote_id":"variant-note","blocks":[paragraph(8)]}]);
        d["page_masters"]["masters"][0]["footnote"] =
            json!({"x":20*65536,"y":320*65536,"width":360*65536,"height":120*65536});
    }
    let mut next = 0;
    renumber(&mut d["document"], &mut next);
    d
}
fn width(pt: i64) -> PositiveLength {
    PositiveLength::new(Length::from_raw(pt * 65536).unwrap()).unwrap()
}
fn source_profiles(
    flow: &PreparedBookV2TextFlow<'_>,
    w: PositiveLength,
) -> Vec<Vec<PositiveLength>> {
    flow.paragraphs()
        .iter()
        .map(|p| {
            let count = p
                .items()
                .iter()
                .map(|s| match s.content() {
                    typaxis_syntax::ProductionInlineContent::Text { utf8, .. } => {
                        utf8.chars().count()
                    }
                    _ => 1,
                })
                .sum::<usize>()
                .max(1);
            vec![w; count]
        })
        .collect()
}
fn selected_facts(selected: Option<&typaxis_linebreak::ProductionInlineBreak>) -> Option<Value> {
    selected.map(|selected| {
        json!({
            "canonical": selected.canonical_jcs(),
            "fingerprint": selected.fingerprint(),
            "paragraph": selected.paragraph_fingerprint(),
            "lines": selected.lines().iter().map(|line| {
                let actual = line.line();
                let metrics = actual.metrics();
                json!({
                    "width": line.inline_size().get().raw(),
                    "shift": line.origin_shift().get().raw(),
                    "required": line.required_inline_size().get().raw(),
                    "index": actual.line_index(),
                    "start": actual.start_unit(), "end": actual.end_unit(),
                    "advance": actual.logical_advance().get().raw(),
                    "left": actual.visual_left().map(|x| x.raw()),
                    "right": actual.visual_right().map(|x| x.raw()),
                    "ascent": metrics.content_ascent().get().raw(),
                    "descent": metrics.content_descent().get().raw(),
                    "leading_before": metrics.leading_before().get().raw(),
                    "leading_after": metrics.leading_after().get().raw(),
                    "height": metrics.line_height().get().raw(),
                    "break_kind": format!("{:?}", actual.break_kind()),
                    "penalty": actual.break_penalty(), "demerits": actual.break_demerits(),
                    "pens": (actual.start_unit()..actual.end_unit())
                        .map(|unit| line.unit_pen_x(unit).map(|x| x.raw())).collect::<Vec<_>>(),
                    "occurrences": actual.occurrences().iter().map(|item| json!({
                        "unit": item.unit_index(), "item": format!("{:?}", item.item()),
                        "pen": item.pen_x().raw(), "before": item.spacing_before().get().raw(),
                        "after": item.spacing_after().get().raw()
                    })).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        })
    })
}

fn line_facts(lines: &BookV2InlineLineLayout<'_, '_>) -> Value {
    json!({"fingerprint":lines.fingerprint(),"shape":lines.prepared().shaped().fingerprint(),
        "inline":lines.prepared().fingerprint(),"native":lines.prepared().native_math().map(|n|n.fingerprint()),
        "paragraphs":lines.paragraphs().iter().map(|p|json!({"owner":p.owner().get(),
            "width":p.inline_size().get().raw(),"selected":selected_facts(p.selected()),
            "font":format!("{:?}",p.font()),"lines":format!("{:?}",p.lines()),
            "anchors":format!("{:?}",p.anchors())})).collect::<Vec<_>>()})
}

#[test]
fn book_v2_column_observed_variants_actual_seed_and_sibling_match_legacy_distinct_widths() {
    for has_native in [false, true] {
        let limits = caps();
        let root = Root::new();
        let prepared = native_input(&root, variant_data(has_native, false), &limits, MATH_FONT);
        let mut history = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
        with_p1_observed_original_column_context(&prepared,&limits,MODE,&mut history,|ctx,a,_|{
            assert_eq!(ctx.native.is_some(),has_native);
            let wide=source_profiles(ctx.flow,width(140));
            let narrow=source_profiles(ctx.flow,width(70));
            let wide_views=wide.iter().map(|p|Some(p.as_slice())).collect::<Vec<_>>();
            let narrow_views=narrow.iter().map(|p|Some(p.as_slice())).collect::<Vec<_>>();
            let wide=BookV2SourceWidthAssignments::new(ctx.flow,&wide_views).unwrap();
            let narrow=BookV2SourceWidthAssignments::new(ctx.flow,&narrow_views).unwrap();
            let foreign=prepare_book_v2_text_flow(ctx.flow.body(),ctx.navigation).unwrap();
            let foreign_width=BookV2SourceWidthAssignments::new(&foreign,&narrow_views).unwrap();
            let mut old_b=BookV2LineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
            let old=prepare_budgeted_book_v2_column_line_variant_seed(ctx.policy,ctx.flow,prepared.resources(),ctx.bindings,
                &limits,MODE,ctx.columns,&mut old_b,0,ctx.native.map(|n|n.math()),Some(&wide)).unwrap();
            let mut first_b=BookV2ObservedLineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
            let first=prepare_observed_book_v2_column_line_variant_seed(ctx.policy,ctx.flow,prepared.resources(),ctx.bindings,
                &limits,MODE,ctx.columns,ctx.native,Some(&wide),&mut first_b,a).unwrap();
            assert_eq!(first.fingerprint(),old.fingerprint());assert!(std::ptr::eq(first.source_flow(),ctx.flow));
            assert!(std::ptr::eq(first.column_plan(),ctx.columns));assert_eq!(first.prepared_counters(),a.counters());
            let context_records=1+first.contexts().paragraphs().iter().map(|p|2+p.ends().len() as u64).sum::<u64>();
            assert_eq!(first.captured_context_records(),context_records);
            assert_eq!(first_b.captured_context_records(),context_records);
            assert!(first_b.staged_record_highwater()>0);
            let before=a.counters();let mut b=BookV2ObservedLineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
            assert!(first.prepare_observed_with_source_widths(&foreign_width,&mut b,a).is_err());
            assert_eq!(a.counters(),before);assert_eq!(b.reshape_passes(),0);
            let mut sibling_b=BookV2ObservedLineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
            let sibling=first.prepare_observed_with_source_widths(&narrow,&mut sibling_b,a).unwrap();
            let mut old_b=BookV2LineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
            let old_sibling=old.prepare_budgeted_with_source_widths(&narrow,&mut old_b,0).unwrap();
            assert_eq!(sibling.fingerprint(),old_sibling.fingerprint());assert_ne!(first.fingerprint(),sibling.fingerprint());
            assert!(a.counters().iter().zip(before).all(|(x,y)|*x>y));
            assert!(std::ptr::eq(sibling.source_flow(),ctx.flow));assert!(std::ptr::eq(sibling.column_plan(),ctx.columns));
            assert_eq!(sibling.prepared_counters(),a.counters());
            eprintln!("P2 genuine seed native={has_native} widths=140/70 first-q={} sibling-q={} current={:?}",first.replay_record_bound(),sibling.replay_record_bound(),a.counters());
            Ok(())
        }).unwrap();
    }
}

#[test]
fn book_v2_column_observed_variants_one_shot_shapes_have_distinct_actual_payments_and_current_caps()
{
    let limits = caps();
    let root = Root::new();
    let prepared = native_input(&root, native_data(), &limits, MATH_FONT);
    let body = prepared.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(prepared.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, prepared.resources(), &limits).unwrap();
    let expected = shape_book_v2_authored_text(
        &policy,
        &flow,
        prepared.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let mut cells = PRIOR;
    loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |a| {
            let one = a.reserve_graph_request(NodeId::new(0), 0).unwrap();
            assert_eq!(a.counters(), [PRIOR[0], PRIOR[1], PRIOR[2] + 1]);
            let before = a.counters();
            let (ok, oracle) = verify_oracle(
                &flow,
                body,
                &nav,
                before,
                a.maximum_records(),
                a.maximum_work(),
            );
            ok.unwrap();
            let mut one_r = 0;
            let (one, mut q1) = shape_book_v2_authored_text_for_observed_request_counted(
                one,
                &policy,
                &flow,
                prepared.resources(),
                &limits,
                bindings.epoch(),
                None,
                a,
                &mut one_r,
            )
            .unwrap();
            assert_eq!(one_r, expected.output_records());
            assert_eq!(one.shape().fingerprint(), expected.fingerprint());
            one.shape()
                .verify(&flow, prepared.resources(), &limits, bindings.epoch())
                .unwrap();
            assert_eq!(one.occurrence_key(), q1.occurrence_key());
            assert_eq!(q1.paid_records(), one_r);
            assert_eq!(a.counters(), [oracle[0] + one_r, oracle[1], oracle[2]]);
            let two = a.reserve_graph_request(NodeId::new(0), 0).unwrap();
            let before = a.counters();
            let (ok, oracle) = verify_oracle(
                &flow,
                body,
                &nav,
                before,
                a.maximum_records(),
                a.maximum_work(),
            );
            ok.unwrap();
            let mut two_r = 0;
            let (two, mut q2) = shape_book_v2_authored_text_for_observed_request_counted(
                two,
                &policy,
                &flow,
                prepared.resources(),
                &limits,
                bindings.epoch(),
                None,
                a,
                &mut two_r,
            )
            .unwrap();
            assert_ne!(one.occurrence_key(), two.occurrence_key());
            assert_ne!(q1.occurrence_key(), q2.occurrence_key());
            assert_eq!(two_r, one_r);
            assert_eq!(two.shape().fingerprint(), one.shape().fingerprint());
            assert!(!std::ptr::eq(one.shape(), two.shape()));
            assert_eq!(a.counters(), [oracle[0] + two_r, oracle[1], oracle[2]]);
            let current = a.counters();
            assert_eq!(
                q1.output_record_limit(a).unwrap(),
                (a.maximum_records() - current[0] + one_r).min(limits.base().get().max_fragments)
            );
            assert_eq!(
                q2.output_record_limit(a).unwrap(),
                (a.maximum_records() - current[0] + two_r).min(limits.base().get().max_fragments)
            );
            // Only this graph's genuine q is included. The other live shape's q
            // is already in currentR and cannot become this graph's credit.
            assert!(
                q1.output_record_limit(a).unwrap()
                    < a.maximum_records() - current[0] + one_r + two_r
            );
            let before = a.counters();
            let mut observed = SourceObservation::default();
            flow.verify_for_observed_counted(
                body,
                &nav,
                before[0],
                before[1],
                before[2],
                a.maximum_records(),
                a.maximum_work(),
                &mut observed,
            )
            .unwrap();
            a.verify_source(&flow, body, &nav).unwrap();
            assert_eq!(a.counters(), triple(observed));
            assert!(q1.output_record_limit(a).unwrap() < a.maximum_records() - current[0] + one_r);
            let before = a.counters();
            q1.retain_actual_output(a, NodeId::new(0), one_r - 1)
                .unwrap();
            assert_eq!(a.counters(), before);
            q1.seal();
            q2.seal();
            assert_eq!(q1.output_record_limit(a), Err(A::ClosedWindow));
            assert_eq!(
                q2.retain_actual_output(a, NodeId::new(0), two_r),
                Err(A::ClosedWindow)
            );
        },
    );
}

#[test]
fn book_v2_column_observed_variants_fresh_shape_cap_is_applied_after_real_source_verification() {
    use typaxis_shaping::{ProductionTextShapeError, ProductionTextShapeErrorKind};
    let limits = caps();
    let root = Root::new();
    let prepared = native_input(&root, native_data(), &limits, MATH_FONT);
    let body = prepared.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(prepared.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, prepared.resources(), &limits).unwrap();
    let shaped = shape_book_v2_authored_text(
        &policy,
        &flow,
        prepared.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let after_request = [PRIOR[0], PRIOR[1], PRIOR[2] + 1];
    let (ok, source) = verify_oracle(
        &flow,
        body,
        &nav,
        after_request,
        limits.base().get().max_fragments,
        MAXIMUM,
    );
    ok.unwrap();
    let exact = source[0] + shaped.output_records();
    for short in [false, true] {
        let maximum = exact - u64::from(short);
        let (ok, actual_source) = verify_oracle(&flow, body, &nav, after_request, maximum, MAXIMUM);
        ok.unwrap();
        assert_eq!(
            actual_source, source,
            "both caps genuinely pass the same full Source prefix"
        );
        let mut cells = PRIOR;
        loan(&mut cells, limits.base(), maximum, MAXIMUM, |a| {
            let request = a.reserve_graph_request(NodeId::new(0), 0).unwrap();
            let mut report = 0;
            let result = shape_book_v2_authored_text_for_observed_request_counted(
                request,
                &policy,
                &flow,
                prepared.resources(),
                &limits,
                bindings.epoch(),
                None,
                a,
                &mut report,
            );
            assert_eq!(a.counters(), [source[0] + report, source[1], source[2]]);
            if short {
                assert!(matches!(
                    result,
                    Err(BookV2ObservedShapeError::Shape(ProductionTextShapeError {
                        kind: ProductionTextShapeErrorKind::OutputLimit,
                        ..
                    }))
                ));
                assert!(report < shaped.output_records());
            } else {
                let (actual, payment) = result.unwrap();
                assert_eq!(actual.shape().fingerprint(), shaped.fingerprint());
                assert_eq!(payment.paid_records(), shaped.output_records());
                assert_eq!(a.counters()[0], maximum);
            }
        });
        eprintln!(
            "P2 post-Source shape short={short} Source={source:?} J={} accepted={cells:?}",
            shaped.output_records()
        );
    }
    let after_sibling = [
        source[0] + shaped.output_records(),
        source[1],
        source[2] + 1,
    ];
    let (ok, target_source) = verify_oracle(
        &flow,
        body,
        &nav,
        after_sibling,
        limits.base().get().max_fragments,
        MAXIMUM,
    );
    ok.unwrap();
    for short in [false, true] {
        let maximum = target_source[0] + shaped.output_records() - u64::from(short);
        let mut cells = PRIOR;
        loan(&mut cells, limits.base(), maximum, MAXIMUM, |a| {
            let sibling_request = a.reserve_graph_request(NodeId::new(0), 0).unwrap();
            let mut sibling_r = 0;
            let (sibling, sibling_q) = shape_book_v2_authored_text_for_observed_request_counted(
                sibling_request,
                &policy,
                &flow,
                prepared.resources(),
                &limits,
                bindings.epoch(),
                None,
                a,
                &mut sibling_r,
            )
            .unwrap();
            assert_eq!(sibling_q.paid_records(), shaped.output_records());
            let request = a.reserve_graph_request(NodeId::new(0), 0).unwrap();
            assert_eq!(a.counters(), after_sibling);
            let (ok, oracle) = verify_oracle(&flow, body, &nav, a.counters(), maximum, MAXIMUM);
            ok.unwrap();
            assert_eq!(oracle, target_source);
            let mut r = 0;
            let result = shape_book_v2_authored_text_for_observed_request_counted(
                request,
                &policy,
                &flow,
                prepared.resources(),
                &limits,
                bindings.epoch(),
                None,
                a,
                &mut r,
            );
            assert_eq!(
                a.counters(),
                [target_source[0] + r, target_source[1], target_source[2]]
            );
            if short {
                assert!(matches!(
                    result,
                    Err(BookV2ObservedShapeError::Shape(ProductionTextShapeError {
                        kind: ProductionTextShapeErrorKind::OutputLimit,
                        ..
                    }))
                ));
                assert_eq!(maximum - target_source[0], shaped.output_records() - 1);
                assert!(
                    maximum - target_source[0] + sibling_q.paid_records()
                        >= shaped.output_records(),
                    "forbidden sibling credit would incorrectly let the target fit"
                );
            } else {
                let (target, q) = result.unwrap();
                assert_ne!(target.occurrence_key(), sibling.occurrence_key());
                assert_eq!(q.paid_records(), shaped.output_records());
            }
            sibling
                .shape()
                .verify(&flow, prepared.resources(), &limits, bindings.epoch())
                .unwrap();
        });
    }
}

#[test]
fn book_v2_column_observed_variants_request_axes_retain_prefixes_without_issuing_positive_credit() {
    let limits = caps();
    let maximum = limits.base().get().max_fragments;
    let max_s = limits.base().get().max_spool_bytes;
    let owner = NodeId::new(23);
    for amount in [0, 3] {
        for mask in 1..8u8 {
            let prior = [
                if mask & 1 != 0 { maximum + 1 } else { PRIOR[0] },
                if mask & 2 != 0 { max_s + 1 } else { PRIOR[1] },
                if mask & 4 != 0 { MAXIMUM + 1 } else { PRIOR[2] },
            ];
            let mut cells = prior;
            loan(&mut cells, limits.base(), maximum, MAXIMUM, |a| {
                let error = a.reserve_graph_request(owner, amount).err().unwrap();
                let expected = if mask & 1 != 0 {
                    A::Records { owner }
                } else if mask & 2 != 0 {
                    A::Spool { owner }
                } else {
                    A::Work { owner }
                };
                assert_eq!(error, expected);
                assert_eq!(a.counters(), prior);
            });
            assert_eq!(cells, prior);
        }
        for short in [false, true] {
            let mut cells = PRIOR;
            let cap = PRIOR[0] + amount - u64::from(short && amount > 0);
            loan(&mut cells, limits.base(), cap, PRIOR[2] + 1, |a| {
                let result = a.reserve_graph_request(owner, amount);
                if short && amount > 0 {
                    assert!(matches!(result,Err(A::Records{owner:o}) if o==owner));
                    assert_eq!(a.counters(), PRIOR);
                } else {
                    assert!(result.is_ok());
                    assert_eq!(a.counters(), [PRIOR[0] + amount, PRIOR[1], PRIOR[2] + 1]);
                }
            });
        }
    }
    let mut cells = PRIOR;
    loan(&mut cells, limits.base(), maximum, PRIOR[2], |a| {
        assert!(matches!(a.reserve_graph_request(owner,0),Err(A::Work{owner:o}) if o==owner));
        assert_eq!(a.counters(), PRIOR);
    });
}

#[test]
fn book_v2_column_observed_variants_set_keeps_duplicate_occurrences_distinct_and_matches_real_legacy(
) {
    for has_native in [false, true] {
        let limits = caps();
        let root = Root::new();
        let prepared = native_input(&root, variant_data(has_native, true), &limits, MATH_FONT);
        let mut history = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
        with_p1_observed_original_column_context(&prepared,&limits,MODE,&mut history,|ctx,a,rest|{
            let wide=source_profiles(ctx.flow,width(140));let narrow=source_profiles(ctx.flow,width(70));
            let wa=wide.iter().map(|v|Some(v.as_slice())).collect::<Vec<_>>();
            let wb=narrow.iter().map(|v|Some(v.as_slice())).collect::<Vec<_>>();
            let aw=BookV2SourceWidthAssignments::new(ctx.flow,&wa).unwrap();
            let bw=BookV2SourceWidthAssignments::new(ctx.flow,&wb).unwrap();
            let mut capture=BookV2ObservedLineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
            let first=prepare_observed_book_v2_column_line_variant_seed(ctx.policy,ctx.flow,prepared.resources(),ctx.bindings,
                &limits,MODE,ctx.columns,ctx.native,Some(&aw),&mut capture,a).unwrap();
            let second=first.prepare_observed_with_source_widths(&bw,&mut capture,a).unwrap();
            let mut old_capture=BookV2LineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
            let old_a=prepare_budgeted_book_v2_column_line_variant_seed(ctx.policy,ctx.flow,prepared.resources(),ctx.bindings,
                &limits,MODE,ctx.columns,&mut old_capture,0,ctx.native.map(|n|n.math()),Some(&aw)).unwrap();
            let old_b=old_a.prepare_budgeted_with_source_widths(&bw,&mut old_capture,0).unwrap();
            let mut old_replay=BookV2LineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
            let old_facts=with_budgeted_rebuilt_book_v2_column_line_variants(&[&old_a,&old_b,&old_a],&mut old_replay,0,|set|
                set.variants().map(|v|(line_facts(v.lines()),v.lines().output_records(),v.footnotes().record_charge())).collect::<Vec<_>>()).unwrap();
            let expected=old_facts.iter().map(|v|v.0.clone()).collect::<Vec<_>>();
            let before=a.counters();let rest_before=(*rest.candidates,*rest.balance_candidates,*rest.width_passes,
                *rest.width_refinements,*rest.line_passes,*rest.page_passes,*rest.output);
            let seeds=[&first,&second,&first];
            let mut budget=BookV2ObservedLineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
            let keys=with_observed_rebuilt_book_v2_column_line_variants(&seeds,&mut budget,a,|set,a|{
                assert_eq!(set.variants().len(),3);assert_eq!(set.counters(),a.counters());
                let mut keys=Vec::new();let mut actual=Vec::new();
                for (i,v) in set.variants().enumerate() {
                    assert_eq!(v.occurrence_ordinal(),i);keys.push(v.occurrence_key());
                    assert!(std::ptr::eq(v.lines().prepared().source_flow(),ctx.flow));
                    assert!(std::ptr::eq(v.column_plan(),ctx.columns));
                    v.footnotes().verify(v.lines(),&limits).unwrap();
                    assert!(v.paid_records()>=v.footnotes().record_charge());
                    assert!(seeds[i].replay_record_bound()>=v.footnotes().record_charge(),
                        "this successful replay's exact footnote J is dominated by its own genuine seed bound");
                    assert_eq!(v.lines().prepared().native_math().is_some(),has_native);
                    eprintln!("P2 origin reports native={has_native} occurrence={i} N={} old-selected={} old-footnotes={} observed-selected={} observed-footnotes={} initial-ownq={}",
                        ctx.native.map_or(0,|n|n.reserved_records()),old_facts[i].1,old_facts[i].2,
                        v.lines().output_records(),v.footnotes().record_charge(),seeds[i].replay_record_bound());
                    if let Some(native)=ctx.native {assert!(std::ptr::eq(v.lines().prepared().native_math().unwrap(),native.math()));}
                    for j in 0..i {
                        let other=set.variant(j).unwrap();assert_ne!(v.occurrence_key(),other.occurrence_key());
                        assert!(!std::ptr::eq(v.lines().prepared(),other.lines().prepared()));
                        assert!(!std::ptr::eq(v.lines().prepared().shaped(),other.lines().prepared().shaped()));
                        assert!(v.footnotes().verify(other.lines(),&limits).is_err());
                    }
                    actual.push(line_facts(v.lines()));
                }
                assert_eq!(actual,expected);assert_eq!(actual[0],actual[2]);assert_ne!(actual[0],actual[1]);keys
            }).unwrap();
            let paid=first.replay_record_bound()*2+second.replay_record_bound();
            assert!(budget.replay_record_payments()>=paid);assert!(a.counters()[0]>=before[0]+paid);
            assert!(a.counters().iter().zip(before).all(|(x,y)|*x>y));
            let after_set=a.counters();let mut single=BookV2ObservedLineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
            with_observed_rebuilt_book_v2_column_line_variant(&first,&mut single,a,|v,a|{
                assert_eq!(line_facts(v.lines()),expected[0]);assert!(!keys.contains(&v.occurrence_key()));
                assert!(a.counters().iter().zip(after_set).all(|(x,y)|*x>y));
            }).unwrap();
            assert_eq!(rest_before,(*rest.candidates,*rest.balance_candidates,*rest.width_passes,
                *rest.width_refinements,*rest.line_passes,*rest.page_passes,*rest.output));
            eprintln!("P2 live-set native={has_native} duplicate=[A,B,A] bounds={paid} paid={} before={before:?} after={:?}",budget.replay_record_payments(),a.counters());
            Ok(())
        }).unwrap();
    }
}

#[test]
fn book_v2_column_observed_variants_callbacks_keep_full_descendant_source_on_all_results_and_retry()
{
    let limits = caps();
    let root = Root::new();
    let prepared = native_input(&root, variant_data(true, false), &limits, MATH_FONT);
    let mut history = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    with_p1_observed_original_column_context(&prepared,&limits,MODE,&mut history,|ctx,a,_|{
        let profiles=source_profiles(ctx.flow,width(100));let views=profiles.iter().map(|p|Some(p.as_slice())).collect::<Vec<_>>();
        let widths=BookV2SourceWidthAssignments::new(ctx.flow,&views).unwrap();
        let mut capture=BookV2ObservedLineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
        let seed=prepare_observed_book_v2_column_line_variant_seed(ctx.policy,ctx.flow,prepared.resources(),ctx.bindings,
            &limits,MODE,ctx.columns,ctx.native,Some(&widths),&mut capture,a).unwrap();
        let mut keys=Vec::new();
        for mode in 0..3 {
            let start=a.counters();let mut expected=None;
            let mut budget=BookV2ObservedLineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
            let outcome=with_observed_rebuilt_book_v2_column_line_variant(&seed,&mut budget,a,|v,a|{
                assert!(!keys.contains(&v.occurrence_key()));keys.push(v.occurrence_key());
                assert_eq!(v.lines().fingerprint(),seed.fingerprint());
                let before=a.counters();let (ok,after)=verify_oracle(ctx.flow,ctx.flow.body(),ctx.navigation,before,a.maximum_records(),a.maximum_work());ok.unwrap();
                a.verify_source(ctx.flow,ctx.flow.body(),ctx.navigation).unwrap();assert_eq!(a.counters(),after);expected=Some(after);
                if mode==2 {let borrowed=&mut *a;std::mem::forget(borrowed);}
                if mode==1 {Err("P2 application sentinel")}else{Ok(())}
            }).unwrap();
            assert_eq!(outcome.is_ok(),mode!=1);assert_eq!(a.counters(),expected.unwrap());
            assert!(a.counters().iter().zip(start).all(|(x,y)|*x>y));
            let before=a.counters();let (ok,after)=verify_oracle(ctx.flow,ctx.flow.body(),ctx.navigation,before,a.maximum_records(),a.maximum_work());ok.unwrap();
            a.verify_source(ctx.flow,ctx.flow.body(),ctx.navigation).unwrap();assert_eq!(a.counters(),after);
        }
        eprintln!("P2 current-prefix callback Ok/Err/borrowedforget plus retry and later full Source={:?}",a.counters());
        Ok(())
    }).unwrap();
}

#[test]
fn book_v2_column_observed_variants_first_real_protocol_backing_has_atomic_exact_short_prefixes() {
    for axis in 0..3 {
        for short in [false, true] {
            let limits = caps();
            let root = Root::new();
            let prepared = native_input(&root, variant_data(false, false), &limits, MATH_FONT);
            let mut history = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
            with_p1_observed_original_column_context(&prepared,&limits,MODE,&mut history,|ctx,a,_|{
                let mut capture=BookV2ObservedLineVariantBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
                let seed=prepare_observed_book_v2_column_line_variant_seed(ctx.policy,ctx.flow,prepared.resources(),ctx.bindings,
                    &limits,MODE,ctx.columns,ctx.native,None,&mut capture,a).unwrap();
                let n=3;let bytes=n*std::mem::size_of_val(&&seed) as u64;
                let request=[n,bytes,n];let scan=9*n+1;
                let maximum=[a.maximum_records(),a.maximum_spool(),a.maximum_work()];
                let available=request[axis]+(if axis==2{scan}else{0})-u64::from(short);
                let mut pressure=[0;3];pressure[axis]=maximum[axis]-a.counters()[axis]-available;
                // This genuine debit is allowance arithmetic only. It does not
                // pretend that the pressure represents a historical allocation.
                a.debit(NodeId::new(0),pressure).unwrap();let before=a.counters();
                // The actual family scan precedes metadata. The exact local
                // allowance then permits just the first real backing request.
                let mut replay=BookV2ObservedLineVariantBudget::new(scan+n,limits.base().get().max_line_reshape_passes);
                let result: Result<(),_>=with_observed_rebuilt_book_v2_column_line_variants(&[&seed,&seed,&seed],&mut replay,a,|_,_|panic!("protocol boundary must not issue graphs"));
                let error=result.err().unwrap();
                if short {
                    let expected=match axis {0=>A::Records{owner:NodeId::new(0)},1=>A::Spool{owner:NodeId::new(0)},_=>A::Work{owner:NodeId::new(0)}};
                    assert!(matches!(error,typaxis_layout::book_v2::BookV2ObservedBodyReshapeError::Accounting(e) if e==expected));
                    assert_eq!(a.counters(),[before[0],before[1],before[2]+scan]);assert_eq!(replay.work_steps(),scan);
                } else {
                    assert!(matches!(error,typaxis_layout::book_v2::BookV2ObservedBodyReshapeError::Accounting(A::Work{..})));
                    assert_eq!(a.counters(),[before[0]+request[0],before[1]+request[1],before[2]+scan+request[2]]);
                    assert_eq!(replay.work_steps(),scan+n);
                }
                assert_eq!(replay.replay_record_payments(),0);assert_eq!(replay.reshape_passes(),0);
                assert!(matches!(replay.last_attempted().unwrap().phase(),typaxis_layout::book_v2::BookV2ObservedVariantPhase::ProtocolStorage));
                eprintln!("P2 actual first view backing axis={axis} short={short} request={request:?} before={before:?} after={:?}",a.counters());
                Ok(())
            }).unwrap();
        }
    }
}

#[test]
fn book_v2_column_observed_variants_mixed_authentic_families_refuse_before_any_replay_payment() {
    let limits = caps();
    let root = Root::new();
    let other_root = Root::new();
    let prepared = native_input(&root, variant_data(false, false), &limits, MATH_FONT);
    let other = native_input(&other_root, variant_data(false, false), &limits, MATH_FONT);
    let other_nav = prepare_book_v2_navigation(other.body().styled()).unwrap();
    let other_policy = prepare_book_v2_resource_policy(other.body(), &limits).unwrap();
    let other_bindings = bind_book_v2_vectors(&other_policy, other.resources(), &limits).unwrap();
    let mut history = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    with_p1_observed_original_column_context(
        &prepared,
        &limits,
        MODE,
        &mut history,
        |ctx, a, _| {
            let mut capture = BookV2ObservedLineVariantBudget::new(
                MAXIMUM,
                limits.base().get().max_line_reshape_passes,
            );
            let first = prepare_observed_book_v2_column_line_variant_seed(
                ctx.policy,
                ctx.flow,
                prepared.resources(),
                ctx.bindings,
                &limits,
                MODE,
                ctx.columns,
                None,
                None,
                &mut capture,
                a,
            )
            .unwrap();
            let other_flow = a.prepare_source(other.body().styled(), &other_nav).unwrap();
            let other_columns =
                driver::prepare_p1_observed_column_frame_plan(&other_flow, &limits, a).unwrap();
            let second = prepare_observed_book_v2_column_line_variant_seed(
                &other_policy,
                &other_flow,
                other.resources(),
                &other_bindings,
                &limits,
                MODE,
                &other_columns,
                None,
                None,
                &mut capture,
                a,
            )
            .unwrap();
            assert_eq!(first.fingerprint(), second.fingerprint());
            assert!(!std::ptr::eq(first.source_flow(), second.source_flow()));
            for scarcity in 0..3 {
                // Scarcity setup is actual allowance arithmetic. Both authenticated
                // roots already exist; no storage charge or fake positive q is minted.
                if scarcity == 1 {
                    a.debit_records(NodeId::new(0), a.maximum_records() - a.counters()[0])
                        .unwrap();
                }
                if scarcity == 2 {
                    a.debit_spool(NodeId::new(0), a.maximum_spool() - a.counters()[1])
                        .unwrap();
                }
                let before = a.counters();
                let mut replay = BookV2ObservedLineVariantBudget::new(
                    MAXIMUM,
                    limits.base().get().max_line_reshape_passes,
                );
                let result: Result<(), _> = with_observed_rebuilt_book_v2_column_line_variants(
                    &[&first, &second],
                    &mut replay,
                    a,
                    |_, _| panic!("mixed authentic families cannot issue a set"),
                );
                assert!(format!("{:?}", result.err().unwrap()).contains("ReceiptMismatch"));
                let n = 2;
                assert_eq!(a.counters(), [before[0], before[1], before[2] + 9 * n + 1]);
                assert_eq!(replay.replay_record_payments(), 0);
                assert_eq!(replay.reshape_passes(), 0);
            }
            let before = a.counters();
            let mut empty = BookV2ObservedLineVariantBudget::new(
                MAXIMUM,
                limits.base().get().max_line_reshape_passes,
            );
            let result: Result<(), _> =
                with_observed_rebuilt_book_v2_column_line_variants(&[], &mut empty, a, |_, _| {
                    panic!("empty set cannot issue")
                });
            assert!(format!("{:?}", result.err().unwrap()).contains("ReceiptMismatch"));
            assert_eq!(a.counters(), before);
            Ok(())
        },
    )
    .unwrap();
}

#[derive(Debug)]
struct CaptureProbe {
    ok: bool,
    error: Option<String>,
    before: [u64; 3],
    after: [u64; 3],
    passes: u16,
    attempted: Option<BookV2ObservedVariantObservation>,
    completed: Option<BookV2ObservedVariantObservation>,
    initial_frame_r: u64,
    initial_frame_q: u64,
}
// Immutable roots are actually prepared once for this call, on an observed
// full-domain allowance. The lower-M trial pays its own authentic Native owner
// and every full Source verification, with a genuinely zero staged graph.
// No positive q, graph receipt or frame authority is constructed from numbers.
fn capture_probe(maximum: Option<u64>, manual_frame_oracle: bool) -> CaptureProbe {
    let limits = caps();
    let root = Root::new();
    let prepared = native_input(&root, variant_data(true, true), &limits, MATH_FONT);
    let body = prepared.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let policy = prepare_book_v2_resource_policy(prepared.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, prepared.resources(), &limits).unwrap();
    let maximum = maximum.unwrap_or(limits.base().get().max_fragments);
    let mut setup = PRIOR;
    let flow = loan(
        &mut setup,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |a| a.prepare_source(body, &nav),
    )
    .unwrap();
    let columns = loan(
        &mut setup,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |a| driver::prepare_p1_observed_column_frame_plan(&flow, &limits, a),
    )
    .unwrap();
    assert_eq!(flow.tables().len(), 1);
    assert_eq!(flow.footnote_definitions().len(), 1);
    let mut cells = setup;
    let mut budget =
        BookV2ObservedLineVariantBudget::new(MAXIMUM, limits.base().get().max_line_reshape_passes);
    let mut before = [0; 3];
    let mut frame_r = 0;
    let mut frame_q = 0;
    let result = loan(&mut cells, limits.base(), maximum, MAXIMUM, |a| {
        let native = compute_book_v2_native_math_observed_counted(
            &bindings,
            prepared.resources(),
            &limits,
            a,
        )
        .unwrap()
        .unwrap();
        before = a.counters();
        if manual_frame_oracle {
            a.with_zero_intrinsic_records(|a, w| {
                let mut r = 0;
                let shaped = shape_book_v2_authored_text_with_observed_source_counted(
                    &policy,
                    &flow,
                    prepared.resources(),
                    &limits,
                    bindings.epoch(),
                    None,
                    a,
                    w,
                    &mut r,
                )
                .unwrap();
                prepare_book_v2_inline_items_with_observed_native_source_counted(
                    &flow,
                    &shaped,
                    prepared.resources(),
                    &bindings,
                    &limits,
                    MODE,
                    Some(&native),
                    a,
                    w,
                    &mut r,
                )
                .unwrap();
                frame_r = a.counters()[0];
                frame_q = w.paid_records();
            });
            Ok(())
        } else {
            prepare_observed_book_v2_column_line_variant_seed(
                &policy,
                &flow,
                prepared.resources(),
                &bindings,
                &limits,
                MODE,
                &columns,
                Some(&native),
                None,
                &mut budget,
                a,
            )
            .map(|_| ())
        }
    });
    CaptureProbe {
        ok: result.is_ok(),
        error: result.err().map(|e| format!("{e:?}")),
        before,
        after: cells,
        passes: budget.reshape_passes(),
        attempted: budget.last_attempted(),
        completed: budget.last_completed(),
        initial_frame_r: frame_r,
        initial_frame_q: frame_q,
    }
}
fn first_frame_passed(probe: &CaptureProbe) -> bool {
    if probe.ok || probe.passes > 0 {
        return true;
    }
    match probe.attempted.map(|o| o.phase()) {
        Some(Phase::Frames)
        | Some(Phase::Shape)
        | Some(Phase::Inline)
        | Some(Phase::SourceVerify)
        | None => false,
        _ => true,
    }
}

#[test]
fn book_v2_column_observed_variants_actual_table_frame_cap_is_consumed_after_current_source() {
    let manual = capture_probe(None, true);
    assert!(manual.ok);
    let full = capture_probe(None, false);
    assert!(full.ok, "real unchanged-rich-input baseline: {full:?}");
    let mut lower = manual.initial_frame_r;
    let baseline = capture_probe(Some(lower), false);
    let first = baseline.attempted.expect("actual attempted phase");
    // If this exact authentic fixture is dominated, that fact must be reported
    // rather than calling an earlier Source refusal a late-cap demonstration.
    assert!(
        !baseline.ok && first.phase() == Phase::Frames && baseline.passes == 0,
        "calibration did not reach the intended first real table/frame allocation: {baseline:?}"
    );
    assert!(first.entered_allocating_kernel());
    assert_eq!(first.before_rsw()[0], manual.initial_frame_r);
    assert_eq!(first.own_q_before(), manual.initial_frame_q);
    assert_eq!(first.effective_limit(), Some(manual.initial_frame_q));
    assert!(baseline.error.as_ref().unwrap().contains("UnitLimit"));
    let mut upper = caps().base().get().max_fragments;
    while upper - lower > 1 {
        let middle = lower + (upper - lower) / 2;
        let probe = capture_probe(Some(middle), false);
        eprintln!("P2 genuine frame calibration M={middle} {probe:?}");
        if first_frame_passed(&probe) {
            upper = middle;
        } else {
            let actual = probe.attempted.unwrap();
            assert_eq!(
                actual.phase(),
                Phase::Frames,
                "a Source/inline refusal is not the requested late Frame proof: {probe:?}"
            );
            assert!(actual.entered_allocating_kernel());
            lower = middle;
        }
    }
    let short = capture_probe(Some(lower), false);
    let exact = capture_probe(Some(upper), false);
    assert_eq!(upper, lower + 1);
    assert!(!first_frame_passed(&short));
    assert!(first_frame_passed(&exact));
    let refused = short.attempted.unwrap();
    assert_eq!(refused.phase(), Phase::Frames);
    assert!(refused.entered_allocating_kernel());
    assert!(short.error.as_ref().unwrap().contains("UnitLimit"));
    assert_eq!(refused.before_rsw()[0], manual.initial_frame_r);
    let cap = refused.effective_limit().unwrap();
    assert!(cap >= refused.own_q_before());
    assert_eq!(
        cap,
        lower - refused.before_rsw()[0] + refused.own_q_before()
    );
    assert_eq!(exact.before, short.before);
    assert!(exact.completed.is_some());
    assert!(exact.after[0] > short.after[0]);
    // The real capped kernel refused at cap>=ownq, and the same first frame
    // succeeds at cap+1. Thus the genuine required local request exceeds ownq.
    // The exact companion may subsequently refuse ContextCapture or Source;
    // that later result is preserved, never promoted to whole-seed success.
    eprintln!("P2 actual consumed table/frame bound J_required={} > ownq={}, exact-companion={exact:?}; short={short:?}",cap+1,refused.own_q_before());
}
