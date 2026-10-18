//! Independent single-graph Native controls on genuine admitted TT MATH data.
//! These tests observe accepted requested records/spool/work. They grant no
//! allocator-capacity, simultaneous graph-set, Header/Footer or pipeline proof.
use super::super::super as driver;
use super::*;
use typaxis_core::{FontFaceId, NodeId};
use typaxis_layout::book_v2::{
    compute_book_v2_native_math, compute_book_v2_native_math_observed_counted,
    compute_preflighted_book_v2_native_math, preflight_book_v2_native_math_counted,
    prepare_book_v2_inline_items_with_native_context_counted,
    prepare_book_v2_inline_items_with_observed_native_source_counted,
    with_budgeted_book_v2_column_lines, with_observed_native_book_v2_column_lines,
    BookV2BodyLineBudget, BookV2NativeMath, BookV2NativeMathBudgetObservation,
    BookV2ObservedBodyLineBudget, BookV2ObservedInlineError, BookV2ObservedNativeMathError as N,
};
use typaxis_layout::{ProductionNativeMathComputationError as NativeError, StagingMathLayoutError};
use typaxis_shaping::book_v2::{
    shape_book_v2_authored_text, shape_book_v2_authored_text_with_observed_source_counted,
    with_book_v2_observed_source_accounting, BookV2ObservedAccountingError as A,
    BookV2ObservedSourceAccounting, BookV2ObservedVerificationError as V,
};
use typaxis_syntax::book_v2::{
    BookV2ObservedSourceFlowError as S, BookV2ObservedSourceFlowObservation as SourceObservation,
};

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
fn rsw(o: BookV2PdfConvergenceObservation) -> [u64; 3] {
    [o.records, o.spool, o.work]
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
fn seed() -> BookV2PdfConvergenceObservation {
    BookV2PdfConvergenceObservation {
        records: PRIOR[0],
        spool: PRIOR[1],
        work: PRIOR[2],
        output: 17,
        candidates: 2,
        balance_candidates: 3,
        width_passes: 4,
        width_refinements: 5,
        line_passes: 6,
        page_passes: 7,
    }
}

#[derive(Debug, PartialEq)]
enum Cause {
    Native(NativeError),
    Accounting(A),
    Cff { owner:NodeId,font:typaxis_core::FontFaceId,diagnostic:String },
}
fn new_cause(error: N) -> Cause {
    match error {
        N::Native(e) => Cause::Native(e),
        N::Accounting(e) => Cause::Accounting(e),
        N::Cff {owner,font_face_id,error}=>Cause::Cff{owner,font:font_face_id,diagnostic:format!("{error:?}")},
    }
}
fn old_cause(error: E) -> Cause {
    match error {
        E::Limit("work") => Cause::Accounting(A::Work {
            owner: NodeId::new(0),
        }),
        E::Limit("records") => Cause::Accounting(A::Records {
            owner: NodeId::new(0),
        }),
        E::Limit("spool") => Cause::Accounting(A::Spool {
            owner: NodeId::new(0),
        }),
        E::Stage {
            stage: "native math",
            source,
        } => Cause::Native(
            source
                .downcast_ref::<NativeError>()
                .expect("exact typed native cause")
                .clone(),
        ),
        other => panic!("unexpected old Native cause: {other:?}"),
    }
}

fn assert_math(
    actual: &BookV2NativeMath<'_>,
    expected: &BookV2NativeMath<'_>,
    input: &PreparedBookV2Resources,
) {
    assert_eq!(actual.fingerprint(), expected.fingerprint());
    assert_eq!(actual.layout_work(), expected.layout_work());
    assert_eq!(actual.record_charge(), expected.record_charge());
    assert_eq!(actual.spool_charge(), expected.spool_charge());
    assert_eq!(
        actual.font_instances().fingerprint(),
        expected.font_instances().fingerprint()
    );
    assert_eq!(
        actual.font_instances().ledger_fingerprint(),
        input.resources().fingerprint()
    );
    assert_eq!(actual.receipts().len(), 2);
    assert_eq!(actual.display_blocks().len(), 1);
    for (a, b) in actual.receipts().iter().zip(expected.receipts()) {
        assert!(std::ptr::eq(a.source(), b.source()));
        assert!(std::ptr::eq(a.style(), b.style()));
        assert_eq!(a.fingerprint(), b.fingerprint());
        assert_eq!(a.font_face_id(), FontFaceId::new(0));
        assert_eq!(a.font_sha256(), typaxis_core::sha256(MATH_FONT));
        assert_eq!(a.face_index(), 0);
        assert_eq!(a.computation().fingerprint(), b.computation().fingerprint());
        assert_eq!(a.computation().paints(), b.computation().paints());
        assert_eq!(
            a.computation().parsed_fingerprint(),
            a.source().parsed().fingerprint()
        );
        assert_eq!(
            a.computation()
                .paints()
                .iter()
                .filter(|p| matches!(p, typaxis_math::MathPaint::Rule(_)))
                .count(),
            1
        );
        let ai = actual
            .font_instances()
            .resolve(a.font_instance_id())
            .unwrap();
        let bi = expected
            .font_instances()
            .resolve(b.font_instance_id())
            .unwrap();
        // Instance tables are distinct genuine allocations; both resolve to
        // the exact same admitted original face, never to a copied font.
        assert!(std::ptr::eq(ai.font(), bi.font()));
        assert!(std::ptr::eq(
            ai.font(),
            input.resources().font(a.font_face_id()).unwrap()
        ));
    }
    for (a, b) in actual
        .display_blocks()
        .iter()
        .zip(expected.display_blocks())
    {
        assert_eq!(a.owner(), b.owner());
        assert_eq!(a.width(), b.width());
        assert_eq!(a.height(), b.height());
        assert_eq!(a.baseline(), b.baseline());
        assert!(a.width().get().raw() > 0 && a.height().get().raw() > 0 && a.baseline().raw() > 0);
    }
}

#[test]
fn book_v2_column_observed_native_genuine_fraction_receipts_and_local_origins() {
    let root = Root::new();
    let limits = caps();
    let input = native_input(&root, native_data(), &limits, MATH_FONT);
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let old = compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0)
        .unwrap()
        .unwrap();
    let inclusive = compute_book_v2_native_math(&bindings, input.resources(), &limits, 19, 23)
        .unwrap()
        .unwrap();
    assert_eq!(inclusive.record_charge(), old.record_charge() + 19);
    assert_eq!(inclusive.spool_charge(), old.spool_charge() + 23);
    let mut observation = BookV2NativeMathBudgetObservation::default();
    let plan = preflight_book_v2_native_math_counted(
        &bindings,
        input.resources(),
        &limits,
        0,
        0,
        &mut observation,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        [
            plan.record_charge(),
            plan.spool_charge(),
            plan.reserved_layout_units()
        ],
        [old.record_charge(), old.spool_charge(), old.layout_work()]
    );
    let exact = compute_preflighted_book_v2_native_math(plan, &limits).unwrap();
    assert_math(&exact, &old, &input);
    let mut cells = PRIOR;
    loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |accounting| {
            let actual = compute_book_v2_native_math_observed_counted(
                &bindings,
                input.resources(),
                &limits,
                accounting,
            )
            .unwrap()
            .unwrap();
            assert_math(actual.math(), &old, &input);
            assert_eq!(
                [
                    actual.reserved_records(),
                    actual.reserved_spool(),
                    actual.reserved_layout_units()
                ],
                [old.record_charge(), old.spool_charge(), old.layout_work()]
            );
            actual
                .math()
                .verify(&bindings, input.resources(), &limits)
                .unwrap();
            assert_eq!(
                accounting.counters(),
                [
                    PRIOR[0] + old.record_charge(),
                    PRIOR[1] + old.spool_charge(),
                    PRIOR[2] + old.layout_work()
                ]
            );
        },
    );
    eprintln!("P1 genuine TT MATH local=[{},{},{}] before={PRIOR:?} after={cells:?}; inline+display fraction Rules=2",old.record_charge(),old.spool_charge(),old.layout_work());
}

#[test]
fn book_v2_column_observed_native_independent_axis_prefixes_match_command_oracle() {
    let limits = caps();
    for has_native in [false, true] {
        let root = Root::new();
        let input = if has_native {
            native_input(&root, native_data(), &limits, MATH_FONT)
        } else {
            input(&root, &data(false), &limits)
        };
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let mut observed = BookV2NativeMathBudgetObservation::default();
        let _ = preflight_book_v2_native_math_counted(
            &bindings,
            input.resources(),
            &limits,
            0,
            0,
            &mut observed,
        )
        .unwrap();
        let local = [
            observed.record_charge(),
            observed.spool_charge(),
            observed.reserved_layout_units(),
        ];
        assert_eq!(local.iter().all(|n| *n > 0), has_native);
        // Every combination of existing bad priors exercises the actual
        // zero request too. Factory construction itself never repairs cells.
        for mask in 0..8u8 {
            let prior = [
                if mask & 1 != 0 {
                    limits.base().get().max_fragments + 1
                } else {
                    PRIOR[0]
                },
                if mask & 2 != 0 {
                    limits.base().get().max_spool_bytes + 1
                } else {
                    PRIOR[1]
                },
                if mask & 4 != 0 { MAXIMUM + 1 } else { PRIOR[2] },
            ];
            let mut history = seed();
            history.records = prior[0];
            history.spool = prior[1];
            history.work = prior[2];
            let expected =
                crate::book_v2_resources::converged_pdf::with_reserved_book_v2_native_math(
                    &bindings,
                    input.resources(),
                    &limits,
                    MAXIMUM,
                    &mut history,
                    |plan| {
                        plan.map(|p| compute_preflighted_book_v2_native_math(p, &limits))
                            .transpose()
                            .map(|n| n.is_some())
                    },
                )
                .map_err(old_cause);
            let mut cells = prior;
            let actual = loan(
                &mut cells,
                limits.base(),
                limits.base().get().max_fragments,
                MAXIMUM,
                |accounting| {
                    compute_book_v2_native_math_observed_counted(
                        &bindings,
                        input.resources(),
                        &limits,
                        accounting,
                    )
                    .map(|n| n.is_some())
                    .map_err(new_cause)
                },
            );
            assert_eq!(actual, expected, "native={has_native} mask={mask}");
            assert_eq!(cells, rsw(history));
        }
        if has_native {
            for axis in 0..3 {
                for short in [false, true] {
                    let mut prior = PRIOR;
                    if axis == 0 {
                        prior[0] = limits.base().get().max_fragments - local[0] + u64::from(short);
                    }
                    if axis == 1 {
                        prior[1] =
                            limits.base().get().max_spool_bytes - local[1] + u64::from(short);
                    }
                    let max_w = if axis == 2 {
                        prior[2] + local[2] - u64::from(short)
                    } else {
                        MAXIMUM
                    };
                    let mut history = seed();
                    history.records = prior[0];
                    history.spool = prior[1];
                    history.work = prior[2];
                    let expected =
                        crate::book_v2_resources::converged_pdf::with_reserved_book_v2_native_math(
                            &bindings,
                            input.resources(),
                            &limits,
                            max_w,
                            &mut history,
                            |plan| {
                                plan.map(|p| compute_preflighted_book_v2_native_math(p, &limits))
                                    .transpose()
                                    .map(|n| n.is_some())
                            },
                        )
                        .map_err(old_cause);
                    let mut cells = prior;
                    let actual = loan(
                        &mut cells,
                        limits.base(),
                        limits.base().get().max_fragments,
                        max_w,
                        |accounting| {
                            compute_book_v2_native_math_observed_counted(
                                &bindings,
                                input.resources(),
                                &limits,
                                accounting,
                            )
                            .map(|n| n.is_some())
                            .map_err(new_cause)
                        },
                    );
                    assert_eq!(actual, expected);
                    assert_eq!(actual.is_err(), short);
                    assert_eq!(cells, rsw(history));
                    // The refused axis leaves its prior; the other two still pay.
                    for i in 0..3 {
                        assert_eq!(
                            cells[i],
                            prior[i] + if short && i == axis { 0 } else { local[i] }
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn book_v2_column_observed_native_preflight_and_compute_error_priority() {
    for low_units in [false, true] {
        let mut limits = caps();
        if low_units {
            let mut extension = limits.extension().get().clone();
            extension.max_math_layout_units = 1;
            limits = M4EffectiveResourceLimits::new(limits.base().clone(), extension).unwrap();
        }
        let root = Root::new();
        let input = native_input(&root, native_data(), &limits, FONT);
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        for mask in 0..8u8 {
            let prior = [
                if mask & 1 != 0 {
                    limits.base().get().max_fragments + 1
                } else {
                    PRIOR[0]
                },
                if mask & 2 != 0 {
                    limits.base().get().max_spool_bytes + 1
                } else {
                    PRIOR[1]
                },
                if mask & 4 != 0 { MAXIMUM + 1 } else { PRIOR[2] },
            ];
            let mut history = seed();
            history.records = prior[0];
            history.spool = prior[1];
            history.work = prior[2];
            let expected =
                crate::book_v2_resources::converged_pdf::with_reserved_book_v2_native_math(
                    &bindings,
                    input.resources(),
                    &limits,
                    MAXIMUM,
                    &mut history,
                    |plan| {
                        plan.map(|p| compute_preflighted_book_v2_native_math(p, &limits))
                            .transpose()
                            .map(|_| ())
                    },
                )
                .map_err(old_cause);
            let mut cells = prior;
            let actual = loan(
                &mut cells,
                limits.base(),
                limits.base().get().max_fragments,
                MAXIMUM,
                |accounting| {
                    compute_book_v2_native_math_observed_counted(
                        &bindings,
                        input.resources(),
                        &limits,
                        accounting,
                    )
                    .map(|_| ())
                    .map_err(new_cause)
                },
            );
            assert_eq!(actual, expected);
            assert_eq!(cells, rsw(history));
            assert!(actual.is_err());
            if mask == 0 && !low_units {
                assert_eq!(
                    actual,
                    Err(Cause::Native(
                        StagingMathLayoutError::InvalidMathFont(NodeId::new(4)).into()
                    ))
                );
            }
            if low_units {
                assert!(
                    matches!(actual, Err(Cause::Native(_))),
                    "local preflight error precedes every command axis: {actual:?}"
                );
            }
        }
    }
}

#[test]
fn book_v2_column_observed_native_inline_reports_zero_base_and_pays_shared_native_once() {
    let root = Root::new();
    let limits = caps();
    let input = native_input(&root, native_data(), &limits, MATH_FONT);
    let body = input.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let raw = compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0)
        .unwrap()
        .unwrap();
    let shape = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let mut legacy_report = 0;
    let legacy = prepare_book_v2_inline_items_with_native_context_counted(
        &flow,
        &shape,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        Some(&raw),
        &mut legacy_report,
    )
    .unwrap();
    assert!(legacy_report >= raw.record_charge());
    let mut cells = PRIOR;
    loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |accounting| {
            let native = compute_book_v2_native_math_observed_counted(
                &bindings,
                input.resources(),
                &limits,
                accounting,
            )
            .unwrap()
            .unwrap();
            let after_native = accounting.counters();
            accounting.with_zero_intrinsic_records(|accounting,window| {
            let mut shape_r=0;
            let (ok,mut expected)=verify_oracle(&flow,body,&nav,after_native,accounting.maximum_records(),MAXIMUM);ok.unwrap();
            let observed_shape=shape_book_v2_authored_text_with_observed_source_counted(&policy,&flow,input.resources(),&limits,bindings.epoch(),None,accounting,window,&mut shape_r).unwrap();
            assert_eq!(observed_shape.fingerprint(),shape.fingerprint());assert_eq!(shape_r,shape.output_records());
            expected[0]+=shape_r+input.resources().fonts().len() as u64;
            let after_shape=accounting.counters();assert_eq!(after_shape[0],expected[0]);
            assert!(after_shape[1]>expected[1] && after_shape[2]>expected[2]);expected=after_shape;
            let (ok,mut expected)=verify_oracle(&flow,body,&nav,expected,accounting.maximum_records(),MAXIMUM);ok.unwrap();
            let mut inline_r=u64::MAX;
            let actual=prepare_book_v2_inline_items_with_observed_native_source_counted(&flow,&observed_shape,input.resources(),&bindings,&limits,MODE,Some(&native),accounting,window,&mut inline_r).unwrap();
            assert_eq!(inline_r,legacy_report-raw.record_charge());assert_eq!(actual.fingerprint(),legacy.fingerprint());
            assert!(std::ptr::eq(actual.native_math().unwrap(),native.math()));
            expected[0]+=inline_r.saturating_sub(shape_r);assert_eq!(accounting.counters(),expected);
            assert_eq!(window.paid_records(),shape_r.max(inline_r));
            let before=accounting.counters();window.seal();
            assert_eq!(accounting.retain_intrinsic_output(window,NodeId::new(4),0),Err(A::ClosedWindow));assert_eq!(accounting.counters(),before);
            eprintln!("P1 nativeN={} shapeI={shape_r} inlineLegacy={legacy_report} inlineLocal={inline_r} final={before:?}",raw.record_charge());
        });
        },
    );
}

fn page_facts(pages: &BookV2ColumnRepeatedPages<'_, '_, '_, '_, '_>) -> Value {
    json!(pages.sequence().pages().iter().map(|p|json!({"index":p.page_index(),"name":p.named_page(),"forced":p.forced_break().map(|o|o.get()),"columns":p.candidate().columns().iter().map(|c|json!({"x":c.bounds().x().raw(),"y":c.bounds().y().raw(),"width":c.bounds().width().get().raw(),"height":c.bounds().height().get().raw(),"used":c.used_height().raw(),"parts":c.parts().iter().map(|part|part.items()).collect::<Vec<_>>()})).collect::<Vec<_>>()})).collect::<Vec<_>>())
}

#[test]
fn book_v2_column_observed_native_real_context_lines_source_columns_then_current_full_verify() {
    let root = Root::new();
    let limits = caps();
    let input = native_input(&root, native_data(), &limits, MATH_FONT);
    // The unchanged Native/line/pagination route is a genuine geometry oracle,
    // not a second implementation of the observed accounting adapter.
    let body = input.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let raw = compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0)
        .unwrap()
        .unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, MAXIMUM, 0, 0).unwrap();
    let mut old_budget =
        BookV2BodyLineBudget::new(MAXIMUM, limits.base().get().max_line_reshape_passes);
    let (old_fp, old_pages) = with_budgeted_book_v2_column_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        Some(&raw),
        &mut old_budget,
        &plan,
        None,
        |stable| {
            assert!(std::ptr::eq(
                stable.lines().prepared().native_math().unwrap(),
                &raw
            ));
            let columns = prepare_book_v2_column_flow_counted(
                &stable,
                None,
                &limits,
                stable.retained_record_charge(),
                &mut 0,
            )
            .unwrap();
            let measured =
                prepare_book_v2_column_table_measurements_counted(columns, &limits, &mut 0)
                    .unwrap();
            let mut search = prepare_book_v2_column_page_search_counted(
                &measured,
                &limits,
                MAXIMUM,
                measured.record_charge(),
                &mut 0,
                &mut 0,
            )
            .unwrap();
            let pages = search
                .select_repeated_pages_counted(limits.base().get().max_layout_passes, &mut 0)
                .unwrap();
            (stable.lines().fingerprint(), page_facts(&pages))
        },
    )
    .unwrap();
    let mut budget = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    budget.inner.observation = seed();
    let result=with_p1_observed_original_column_context(&input,&limits,MODE,&mut budget,|ctx,accounting,rest| {
        assert!(std::ptr::eq(ctx.input,&input));assert!(std::ptr::eq(ctx.limits,&limits));
        assert!(std::ptr::eq(ctx.flow.body(),body));assert!(std::ptr::eq(ctx.flow.navigation(),ctx.navigation));
        assert!(std::ptr::eq(ctx.columns.source(),body));
        assert_eq!(ctx.mode,MODE);assert_eq!(ctx.maximum,MAXIMUM);
        assert_eq!(ctx.prepared_progress,driver::ColumnProgressView{accounting:&mut *accounting,rest:&mut *rest}.snapshot());
        let native=ctx.native.expect("the actual private context owns paid NativeSome");
        native.math().verify(ctx.bindings,input.resources(),&limits).unwrap();
        assert_eq!(native.math().fingerprint(),raw.fingerprint());
        assert!(native.math().receipts().iter().all(|r|r.computation().paints().iter().any(|p|matches!(p,typaxis_math::MathPaint::Rule(_)))));
        let mut local=BookV2ObservedBodyLineBudget::new(MAXIMUM,limits.base().get().max_line_reshape_passes);
        let result=with_observed_native_book_v2_column_lines(ctx.policy,ctx.flow,input.resources(),ctx.bindings,&limits,MODE,ctx.native,&mut local,accounting,ctx.columns,None,|stable,accounting| {
            assert_eq!(stable.lines().fingerprint(),old_fp);
            assert!(std::ptr::eq(stable.lines().prepared().native_math().unwrap(),native.math()));
            assert!(std::ptr::eq(stable.column_plan(),ctx.columns));
            let before=accounting.counters();
            let mut view=driver::ColumnProgressView{accounting:&mut *accounting,rest:&mut *rest};
            let callback=driver::with_p0_observed_column_pagination(&stable,&limits,&mut view,|pages,search,feedback,observed| {
                assert_eq!(page_facts(pages),old_pages);assert!(!pages.sequence().pages().is_empty());
                assert!(std::ptr::eq(search.source_lines(),stable.lines()));
                assert!(feedback.matches_source_flow(ctx.flow));
                assert!(std::ptr::eq(feedback.column_plan(),ctx.columns));
                assert_eq!(observed.output,seed().output);
                assert_eq!(observed.records,search.record_charge());assert_eq!(observed.spool,search.terminal_spool_charge());assert_eq!(observed.work,before[2]+search.work_steps());
                let pre=[search.record_charge(),search.terminal_spool_charge(),before[2]+search.work_steps()];
                let replay=search.paragraph_frame_feedback(pages.sequence()).unwrap();
                assert_eq!(replay.assignment_fingerprint(),feedback.assignment_fingerprint());
                let post=[search.record_charge(),search.terminal_spool_charge(),before[2]+search.work_steps()];
                assert!(post[0]>pre[0]&&post[2]>pre[2]);assert!(post[1]>=pre[1]);
                post
            }).unwrap();
            assert_eq!(view.accounting.counters(),callback);
            assert_eq!(view.snapshot().output,seed().output);
        }).map_err(E::from);
        *rest.line_passes+=local.reshape_passes();result?;
        // Graphs/search have ended; the same caller loan now recollects every
        // source field using the exact latest accepted triple, not logical R.
        let before=accounting.counters();
        let (expected,after)=verify_oracle(ctx.flow,body,ctx.navigation,before,accounting.maximum_records(),MAXIMUM);expected.unwrap();
        accounting.verify_source(ctx.flow,body,ctx.navigation).unwrap();assert_eq!(accounting.counters(),after);
        assert!(after.iter().zip(before).all(|(a,b)|*a>b));
        assert_eq!(*rest.output,seed().output);assert_eq!(*rest.candidates,seed().candidates+1);
        assert_eq!(*rest.width_passes,seed().width_passes);assert_eq!(*rest.width_refinements,seed().width_refinements);assert_eq!(*rest.balance_candidates,seed().balance_candidates);
        eprintln!("P1 genuine NativeSome→lines→SourceColumns before-current-verify={before:?} after={after:?}; nativeN={} lineW={} linepasses={}",native.reserved_records(),local.candidate_steps(),local.reshape_passes());
        Ok(after)
    }).unwrap();
    assert_eq!(rsw(budget.observation()), result);
    assert_eq!(budget.observation().output, 17);
    assert!(
        budget.observation().line_passes > seed().line_passes
            && budget.observation().page_passes > seed().page_passes
    );
}

#[test]
fn book_v2_column_observed_native_all_result_prefixes_retry_and_borrowed_forget() {
    let root = Root::new();
    let limits = caps();
    let input = native_input(&root, native_data(), &limits, MATH_FONT);
    let mut budget = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    budget.inner.observation = seed();
    let mut expected = PRIOR;
    // Each private-context attempt owns a new genuine Native/source/frame set;
    // dropping those owners on a normal application Err never refunds it.
    for attempt in 0..3 {
        let prior = budget.observation();
        let error = with_p1_observed_original_column_context(
            &input,
            &limits,
            MODE,
            &mut budget,
            |ctx, accounting, rest| {
                let entry = accounting.counters();
                assert!(entry.iter().zip(rsw(prior)).all(|(a, b)| *a > b));
                let native = ctx.native.unwrap();
                let n = [
                    native.reserved_records(),
                    native.reserved_spool(),
                    native.reserved_layout_units(),
                ];
                assert!(n.iter().all(|v| *v > 0));
                let mut local = BookV2ObservedBodyLineBudget::new(
                    MAXIMUM,
                    limits.base().get().max_line_reshape_passes,
                );
                let lines = with_observed_native_book_v2_column_lines(
                    ctx.policy,
                    ctx.flow,
                    input.resources(),
                    ctx.bindings,
                    &limits,
                    MODE,
                    ctx.native,
                    &mut local,
                    accounting,
                    ctx.columns,
                    None,
                    |stable, accounting| {
                        assert!(std::ptr::eq(
                            stable.lines().prepared().native_math().unwrap(),
                            native.math()
                        ));
                        let mut view = driver::ColumnProgressView {
                            accounting: &mut *accounting,
                            rest: &mut *rest,
                        };
                        driver::with_p0_observed_column_pagination(
                            &stable,
                            &limits,
                            &mut view,
                            |pages, search, feedback, _| {
                                let before = search.work_steps();
                                let replay =
                                    search.paragraph_frame_feedback(pages.sequence()).unwrap();
                                assert_eq!(
                                    replay.assignment_fingerprint(),
                                    feedback.assignment_fingerprint()
                                );
                                assert!(search.work_steps() > before);
                            },
                        )
                        .unwrap();
                    },
                );
                *rest.line_passes += local.reshape_passes();
                lines.map_err(E::from)?;
                let entry = accounting.counters();
                let (ok, after) = verify_oracle(
                    ctx.flow,
                    input.body().styled(),
                    ctx.navigation,
                    entry,
                    accounting.maximum_records(),
                    MAXIMUM,
                );
                ok.unwrap();
                accounting
                    .verify_source(ctx.flow, input.body().styled(), ctx.navigation)
                    .unwrap();
                assert_eq!(accounting.counters(), after);
                assert_eq!(*rest.output, prior.output);
                expected = after;
                if attempt == 1 {
                    // Forgetting only the borrowed reference cannot move the
                    // driver-owned loan or erase the actual counter cells.
                    #[allow(forgetting_references)]
                    std::mem::forget(&mut *accounting);
                }
                Err::<(), _>(E::Identity)
            },
        )
        .unwrap_err();
        assert!(matches!(error, E::Identity));
        assert_eq!(rsw(budget.observation()), expected);
        assert_eq!(budget.observation().output, prior.output);
    }
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let raw = compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0)
        .unwrap()
        .unwrap();
    let (_, verified) = verify_oracle(
        &flow,
        input.body().styled(),
        &nav,
        PRIOR,
        limits.base().get().max_fragments,
        MAXIMUM,
    );
    let max_w = PRIOR[2] + raw.layout_work() + (verified[2] - PRIOR[2]) - 1;
    let mut cells = PRIOR;
    loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        max_w,
        |accounting| {
            let native = compute_book_v2_native_math_observed_counted(
                &bindings,
                input.resources(),
                &limits,
                accounting,
            )
            .unwrap()
            .unwrap();
            let before = accounting.counters();
            let (expected, after) = verify_oracle(
                &flow,
                input.body().styled(),
                &nav,
                before,
                accounting.maximum_records(),
                max_w,
            );
            assert!(matches!(expected, Err(S::Work { .. })));
            assert_eq!(
                accounting.verify_source(&flow, input.body().styled(), &nav),
                expected.map_err(V::Source)
            );
            assert_eq!(accounting.counters(), after);
            assert_eq!(native.reserved_records(), raw.record_charge());
            let mut first = 0;
            accounting.with_zero_intrinsic_records(|accounting, window| {
                accounting
                    .retain_intrinsic_output(window, NodeId::new(4), 5)
                    .unwrap();
                first = accounting.counters()[0];
                window.seal();
                assert_eq!(
                    accounting.retain_intrinsic_output(window, NodeId::new(4), 6),
                    Err(A::ClosedWindow)
                );
                assert_eq!(accounting.counters()[0], first);
            });
            accounting.with_zero_intrinsic_records(|accounting, window| {
                assert_eq!(window.paid_records(), 0);
                accounting
                    .retain_intrinsic_output(window, NodeId::new(4), 5)
                    .unwrap();
                assert_eq!(accounting.counters()[0], first + 5);
            });
        },
    );
    assert_eq!(cells[2], max_w);
    eprintln!("P1 appErr/borrowed-forget/retry retained={expected:?}; late source Work retained={cells:?}");
}

#[test]
fn book_v2_column_observed_native_frames_require_full_domain_before_allocating() {
    let root = Root::new();
    let limits = caps();
    let input = native_input(&root, native_data(), &limits, MATH_FONT);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let max_r = limits.base().get().max_fragments;
    let mut cells = PRIOR;
    loan(
        &mut cells,
        limits.base(),
        max_r - 1,
        MAXIMUM,
        |accounting| {
            let before = accounting.counters();
            assert!(matches!(
                driver::prepare_p1_observed_column_frame_plan(&flow, &limits, accounting),
                Err(E::ObservedAccounting(A::LimitsMismatch))
            ));
            assert_eq!(accounting.counters(), before);
        },
    );
    assert_eq!(cells, PRIOR);
    let run = |max_w| {
        let mut work = PRIOR[2];
        let mut observed = typaxis_syntax::book_v2::BookV2ColumnFramePlanObservation::default();
        let expected = typaxis_syntax::book_v2::prepare_book_v2_column_frame_plan_counted(
            &flow,
            &mut work,
            max_w,
            PRIOR[0],
            PRIOR[1],
            &mut observed,
        )
        .map(|p| {
            (
                p.measurement_body(),
                p.measurement_footnote(),
                p.record_charge(),
                p.spool_charge(),
            )
        });
        let expected_cells = [
            observed.record_charge().max(PRIOR[0]),
            observed.spool_charge().max(PRIOR[1]),
            work,
        ];
        let mut cells = PRIOR;
        let actual = loan(&mut cells, limits.base(), max_r, max_w, |accounting| {
            driver::prepare_p1_observed_column_frame_plan(&flow, &limits, accounting).map(|p| {
                (
                    p.measurement_body(),
                    p.measurement_footnote(),
                    p.record_charge(),
                    p.spool_charge(),
                )
            })
        });
        match (actual, expected) {
            (Ok(a), Ok(b)) => assert_eq!(a, b),
            (Err(E::ObservedFrames(a)), Err(b)) => assert_eq!(a, b),
            (a, b) => panic!("frame result mismatch {a:?} {b:?}"),
        }
        assert_eq!(cells, expected_cells);
        (cells, work)
    };
    let (_, full_w) = run(MAXIMUM);
    assert!(full_w > PRIOR[2]);
    run(full_w);
    run(full_w - 1);
    // Constructor R/S exactness uses genuine caller-prior shifts against the
    // unchanged body limits, never a lower-cap physical constructor.
    let mut baseline_work = PRIOR[2];
    let mut baseline = typaxis_syntax::book_v2::BookV2ColumnFramePlanObservation::default();
    typaxis_syntax::book_v2::prepare_book_v2_column_frame_plan_counted(
        &flow,
        &mut baseline_work,
        MAXIMUM,
        PRIOR[0],
        PRIOR[1],
        &mut baseline,
    )
    .unwrap();
    for axis in 0..2 {
        for short in [false, true] {
            let mut start = PRIOR;
            start[axis] = if axis == 0 {
                max_r - (baseline.record_charge() - PRIOR[0])
            } else {
                limits.base().get().max_spool_bytes - (baseline.spool_charge() - PRIOR[1])
            } + u64::from(short);
            let mut work = start[2];
            let mut observed = typaxis_syntax::book_v2::BookV2ColumnFramePlanObservation::default();
            let expected = typaxis_syntax::book_v2::prepare_book_v2_column_frame_plan_counted(
                &flow,
                &mut work,
                MAXIMUM,
                start[0],
                start[1],
                &mut observed,
            )
            .map(|_| ());
            let mut cells = start;
            let actual = loan(&mut cells, limits.base(), max_r, MAXIMUM, |accounting| {
                driver::prepare_p1_observed_column_frame_plan(&flow, &limits, accounting)
                    .map(|_| ())
            });
            match (actual, expected) {
                (Ok(()), Ok(())) => assert!(!short),
                (Err(E::ObservedFrames(a)), Err(b)) => {
                    assert!(short);
                    assert_eq!(a, b)
                }
                (a, b) => panic!("frame exact/short mismatch {a:?} {b:?}"),
            }
            assert_eq!(
                cells,
                [
                    observed.record_charge().max(start[0]),
                    observed.spool_charge().max(start[1]),
                    work
                ]
            );
        }
    }
    for axis in 0..3 {
        let mut start = PRIOR;
        if axis == 0 {
            start[0] = max_r + 1;
        }
        if axis == 1 {
            start[1] = limits.base().get().max_spool_bytes + 1;
        }
        if axis == 2 {
            start[2] = MAXIMUM + 1;
        }
        let mut work = start[2];
        let mut observed = typaxis_syntax::book_v2::BookV2ColumnFramePlanObservation::default();
        let expected = typaxis_syntax::book_v2::prepare_book_v2_column_frame_plan_counted(
            &flow,
            &mut work,
            MAXIMUM,
            start[0],
            start[1],
            &mut observed,
        )
        .err()
        .expect("real frame floor refusal");
        let mut cells = start;
        let error = loan(&mut cells, limits.base(), max_r, MAXIMUM, |accounting| {
            driver::prepare_p1_observed_column_frame_plan(&flow, &limits, accounting)
                .err()
                .expect("same genuine refusal")
        });
        assert!(matches!(error,E::ObservedFrames(e) if e==expected));
        assert_eq!(
            cells,
            [
                observed.record_charge().max(start[0]),
                observed.spool_charge().max(start[1]),
                work
            ]
        );
    }
}

#[test]
fn book_v2_column_observed_native_domains_identity_and_current_work_before_measure() {
    let root = Root::new();
    let other_root = Root::new();
    let limits = caps();
    let input = native_input(&root, native_data(), &limits, MATH_FONT);
    let other = native_input(&other_root, native_data(), &limits, MATH_FONT);
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    for change in [-1i64, 1, 2] {
        let mut base = limits.base().get().clone();
        if change < 0 {
            base.max_spool_bytes -= 1;
        } else if change == 1 {
            base.max_spool_bytes += 1;
        } else {
            base.max_layout_passes -= 1;
        }
        let changed = ValidatedResourceLimits::new(base).unwrap();
        for foreign in [false, true] {
            let mut cells = PRIOR;
            let actual = loan(&mut cells, &changed, u64::MAX, MAXIMUM, |accounting| {
                compute_book_v2_native_math_observed_counted(
                    &bindings,
                    if foreign {
                        other.resources()
                    } else {
                        input.resources()
                    },
                    &limits,
                    accounting,
                )
                .err()
                .expect("real earlier identity/domain refusal")
            });
            if foreign {
                assert_eq!(
                    new_cause(actual),
                    Cause::Native(StagingMathLayoutError::ReceiptMismatch.into())
                );
            } else {
                assert_eq!(new_cause(actual), Cause::Accounting(A::LimitsMismatch));
            }
            assert_eq!(cells, PRIOR);
        }
    }
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let foreign_nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, MAXIMUM, 0, 0).unwrap();
    let raw = compute_book_v2_native_math(&bindings, input.resources(), &limits, 0, 0)
        .unwrap()
        .unwrap();
    let shape = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
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
        |accounting| {
            let native = compute_book_v2_native_math_observed_counted(
                &bindings,
                input.resources(),
                &limits,
                accounting,
            )
            .unwrap()
            .unwrap();
            assert!(native
                .math()
                .verify(&bindings, other.resources(), &limits)
                .is_err());
            let before = accounting.counters();
            let (expected, after) = verify_oracle(
                &flow,
                input.body().styled(),
                &foreign_nav,
                before,
                accounting.maximum_records(),
                MAXIMUM,
            );
            assert!(expected.is_err());
            assert_eq!(
                accounting.verify_source(&flow, input.body().styled(), &foreign_nav),
                expected.map_err(V::Source)
            );
            assert_eq!(accounting.counters(), after);
            accounting.with_zero_intrinsic_records(|accounting, window| {
                let before = accounting.counters();
                let mut reported = u64::MAX;
                let foreign = prepare_book_v2_inline_items_with_observed_native_source_counted(
                    &flow, &shape, other.resources(), &bindings, &limits, MODE,
                    Some(&native), accounting, window, &mut reported,
                )
                .err()
                .expect("binding/admission mismatch precedes input/source retention");
                assert!(matches!(foreign, BookV2ObservedInlineError::Inline(e) if e.kind==typaxis_layout::ProductionInlinePreparationErrorKind::ReceiptMismatch));
                assert_eq!(reported,0);assert_eq!(accounting.counters(),before);
                let mut source_before = before;
                source_before[0] += shape.output_records();
                let (ok, expected) = verify_oracle(&flow, input.body().styled(), &nav, source_before, accounting.maximum_records(), MAXIMUM);
                ok.unwrap();
                let missing = prepare_book_v2_inline_items_with_observed_native_source_counted(
                    &flow, &shape, input.resources(), &bindings, &limits, MODE,
                    None, accounting, window, &mut reported,
                )
                .err()
                .expect("missing Native follows the genuine full source check");
                assert!(matches!(missing, BookV2ObservedInlineError::Inline(e) if e.kind==typaxis_layout::ProductionInlinePreparationErrorKind::ReceiptMismatch));
                assert_eq!(accounting.counters(),expected);assert_eq!(window.paid_records(),shape.output_records());
            });
        },
    );
    let run = |max_w| {
        let mut cells = PRIOR;
        let mut local =
            BookV2ObservedBodyLineBudget::new(MAXIMUM, limits.base().get().max_line_reshape_passes);
        let mut called = false;
        let result = loan(
            &mut cells,
            limits.base(),
            limits.base().get().max_fragments,
            max_w,
            |accounting| {
                let native = compute_book_v2_native_math_observed_counted(
                    &bindings,
                    input.resources(),
                    &limits,
                    accounting,
                )
                .unwrap()
                .unwrap();
                with_observed_native_book_v2_column_lines(
                    &policy,
                    &flow,
                    input.resources(),
                    &bindings,
                    &limits,
                    MODE,
                    Some(&native),
                    &mut local,
                    accounting,
                    &plan,
                    None,
                    |stable, _| {
                        called = true;
                        assert!(stable.passes().last().unwrap().is_stable());
                        stable.lines().fingerprint()
                    },
                )
            },
        );
        (result, cells, local, called)
    };
    let (full, after, local, called) = run(MAXIMUM);
    let fp = full.unwrap();
    assert!(called);
    let source_once = verify_oracle(
        &flow,
        input.body().styled(),
        &nav,
        PRIOR,
        limits.base().get().max_fragments,
        MAXIMUM,
    )
    .1;
    let recollections = 2 * (u64::from(local.reshape_passes()) + 1);
    assert_eq!(
        after[0],
        PRIOR[0]
            + raw.record_charge()
            + recollections * (source_once[0] - PRIOR[0])
            + local.record_charge()
    );
    assert_eq!(
        after[1],
        PRIOR[1] + raw.spool_charge() + recollections * (source_once[1] - PRIOR[1])
    );
    assert_eq!(
        after[2],
        PRIOR[2]
            + raw.layout_work()
            + recollections * (source_once[2] - PRIOR[2])
            + local.candidate_steps()
    );
    let (exact, exact_cells, exact_local, called) = run(after[2]);
    assert!(called);
    assert_eq!(exact.unwrap(), fp);
    assert_eq!(exact_cells, after);
    assert_eq!(exact_local.candidate_steps(), local.candidate_steps());
    let (short, short_cells, short_local, called) = run(after[2] - 1);
    assert!(!called && short.is_err());
    assert_eq!(short_cells[2], after[2] - 1);
    assert_eq!(short_local.candidate_steps() + 1, local.candidate_steps());
    assert_eq!(REGISTERED_UNICODE_VERSION, "16.0.0");
    eprintln!("P1 currentW full={after:?} short={short_cells:?}; source-recollections={recollections}, nativeW={}, measureW={}",raw.layout_work(),local.candidate_steps());
}
