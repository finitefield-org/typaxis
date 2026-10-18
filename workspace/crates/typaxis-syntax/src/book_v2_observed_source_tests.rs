//! Genuine source construction and full recollection controls. Fixture setup,
//! traces and fault injection are test operations outside the charged product.
use super::super::super::source_ops::{with_allocation_fault, with_charge_trace};
use super::*;
use crate::book_v2::{prepare_book_v2_body, prepare_book_v2_navigation, style_book_v2_body};
use serde_json::{json, Value};
use typaxis_core::ResourceLimits;
use typaxis_document_package::{
    book_v2::BookV2DocumentPackageDecoder, DocumentPackageDecodePolicy,
};

const COMBINED: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json"));
fn limits(spool: u64) -> ValidatedResourceLimits {
    let mut raw = ResourceLimits::default();
    if spool != u64::MAX {
        raw.max_spool_bytes = spool;
    }
    ValidatedResourceLimits::new(raw).unwrap()
}
fn input() -> Value {
    let mut data: Value = serde_json::from_slice(COMBINED).unwrap();
    data["contract"] = "typaxis.contract/1.5".into();
    data
}
fn styled(data: &Value, limits: &ValidatedResourceLimits) -> StyledBookV2Body {
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(
            &serde_json::to_vec(data).unwrap(),
            &DocumentPackageDecodePolicy::new(limits),
        )
        .unwrap();
    style_book_v2_body(prepare_book_v2_body(decoded, limits).unwrap()).unwrap()
}
// A genuine counted initial-style owner exercises its retained sorted registry
// in the observed source provider; fixture setup is not a source observation.
fn styled_counted(data: &Value, limits: &ValidatedResourceLimits) -> StyledBookV2Body {
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(
            &serde_json::to_vec(data).unwrap(),
            &DocumentPackageDecodePolicy::new(limits),
        )
        .unwrap();
    let mut setup = [0u64; 3];
    let body = crate::book_v2::style_book_v2_body_counted(
        prepare_book_v2_body(decoded, limits).unwrap(),
        &mut |r, s, w| {
            for (i, v) in [r, s, w].into_iter().enumerate() {
                setup[i] += v
            }
            Ok::<_, ()>(())
        },
    )
    .unwrap();
    assert!(setup.iter().all(|n| *n > 0));
    body
}
fn rsw(v: Observation) -> [u64; 3] {
    [v.record_charge(), v.spool_charge(), v.work_steps()]
}
fn values(flow: &PreparedBookV2TextFlow<'_>, page: u32) -> Vec<(NodeId, u32)> {
    let mut result: Vec<_> = flow
        .paragraphs()
        .iter()
        .flat_map(|p| p.items())
        .filter_map(|v| {
            matches!(
                v.reference(),
                Some(ProductionInlineReference::Anchor {
                    format: ProductionReferenceFormat::Page,
                    ..
                })
            )
            .then_some((v.owner(), page))
        })
        .collect();
    result.sort_unstable_by_key(|v| v.0);
    result
}
fn construct<'a>(
    body: &'a StyledBookV2Body,
    nav: &'a PreparedBookV2Navigation<'a>,
    values: Option<&[(NodeId, u32)]>,
    prior: [u64; 3],
    maximum_r: u64,
    maximum_w: u64,
    observation: &mut Observation,
) -> Result<PreparedBookV2TextFlow<'a>, Error> {
    if let Some(values) = values {
        prepare_book_v2_text_flow_with_page_references_observed_counted(
            body,
            nav,
            values,
            prior[0],
            prior[1],
            prior[2],
            maximum_r,
            maximum_w,
            observation,
        )
    } else {
        prepare_book_v2_text_flow_observed_counted(
            body,
            nav,
            prior[0],
            prior[1],
            prior[2],
            maximum_r,
            maximum_w,
            observation,
        )
    }
}
fn same(a: &PreparedBookV2TextFlow<'_>, b: &PreparedBookV2TextFlow<'_>) {
    assert!(std::ptr::eq(a.body(), b.body()));
    assert!(std::ptr::eq(a.navigation(), b.navigation()));
    macro_rules! fields {($($field:ident),*$(,)?)=>{$(assert_eq!(a.$field,b.$field,stringify!($field));)*};}
    fields!(
        events,
        paragraphs,
        figures,
        named_page_breaks,
        tables,
        table_record_charge,
        source_record_charge,
        lists,
        list_items,
        description_lists,
        description_items,
        footnote_definitions,
        footnote_base_style,
        generated,
        page_reference_values,
        text_bytes,
        fingerprint
    );
}
// This existing legacy source census is calculated from actual constructed
// carriers, independently of the new full allocation history.
fn intrinsic(flow: &PreparedBookV2TextFlow<'_>) -> u64 {
    1 + 2 * flow.footnote_definitions().len() as u64
        + flow.events().len() as u64
        + flow.paragraphs().len() as u64
        + flow
            .paragraphs()
            .iter()
            .map(|p| p.items().len() as u64)
            .sum::<u64>()
        + flow.figures().len() as u64
        + flow.named_page_breaks().len() as u64
        + flow.lists().len() as u64
        + flow.list_items().len() as u64
        + flow.description_lists().len() as u64
        + flow.description_items().len() as u64
        + flow.table_record_charge()
        + 2 * flow.generated.buffers().len() as u64
        + flow.page_reference_values().map_or(0, |v| v.len() as u64)
}
fn renumber(value: &mut Value, next: &mut u32) {
    if let Some(values) = value.as_array_mut() {
        for value in values {
            renumber(value, next)
        }
    } else if let Some(object) = value.as_object_mut() {
        if let Some(id) = object.get_mut("node_id") {
            *id = (*next).into();
            *next += 1;
        }
        for key in [
            "term",
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
            if let Some(value) = object.get_mut(key) {
                renumber(value, next)
            }
        }
    }
}
fn rich_input() -> Value {
    let mut data = input();
    let paragraph = data["document"]["blocks"][3]["items"][0]["blocks"][0].clone();
    let mut term = paragraph.clone();
    term.as_object_mut().unwrap().remove("kind");
    data["document"]["blocks"].as_array_mut().unwrap().push(json!({
        "kind":"description_list","node_id":0,"span":paragraph["span"],"classes":[],
        "items":[{"node_id":0,"span":paragraph["span"],"term":term,"blocks":[paragraph.clone()]}]}));
    let nested = data["document"]["blocks"][6].clone();
    data["document"]["blocks"][6]["body"][0]["cells"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(nested);
    data["document"]["blocks"][6]["caption"] = json!([paragraph]);
    let order = data["style_sheet"]["rules"].as_array().unwrap().len();
    let mut description = data["style_sheet"]["rules"][0].clone();
    description["style_id"] = "observed-description".into();
    description["selector"] = "description_list".into();
    description["source_order"] = (order as u64).into();
    data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(description);
    data["document"]["language"] = "ja-jp".into();
    data["document"]["blocks"][0]["language"] = "en-us".into();
    data["document"]["footnotes"][0]["language"] = "de-de".into();
    data["style_sheet"]["rules"][3]["declarations"][0]["value"] =
        json!({"kind":"string","value":"basic-combined"});
    let mut unused = data["document"]["footnotes"][0].clone();
    unused["footnote_id"] = "observed-unused".into();
    unused["blocks"] = json!([]);
    data["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(unused);
    renumber(&mut data["document"], &mut 0);
    fn find(value: &Value, anchor: &str) -> Option<Value> {
        match value {
            Value::Array(v) => v.iter().find_map(|v| find(v, anchor)),
            Value::Object(v) => {
                if v.get("anchor_id").and_then(Value::as_str) == Some(anchor) {
                    v.get("node_id").cloned()
                } else {
                    v.values().find_map(|v| find(v, anchor))
                }
            }
            _ => None,
        }
    }
    let document = data["document"].clone();
    for entry in data["outline"]["entries"].as_array_mut().unwrap() {
        entry["source_node_id"] = find(&document, entry["destination"].as_str().unwrap()).unwrap();
    }
    data
}
fn minimal_input() -> Value {
    let mut data = input();
    let mut p = data["document"]["blocks"][1].clone();
    let mut text = data["document"]["blocks"][0]["children"][0].clone();
    p["node_id"] = 1.into();
    text["node_id"] = 2.into();
    p["children"] = json!([text]);
    data["document"]["blocks"] = json!([p]);
    data["document"]["footnotes"] = json!([]);
    data["outline"]["entries"] = json!([]);
    let mut rule = data["style_sheet"]["rules"][1].clone();
    rule["source_order"] = 0.into();
    data["style_sheet"]["rules"] = json!([rule]);
    data
}

#[test]
fn observed_source_original_and_page10_page100_match_all_legacy_fields_and_intrinsic_records() {
    for (rich, counted_registry) in [(false, false), (true, false), (true, true)] {
        let data = if rich { rich_input() } else { input() };
        let body = if counted_registry {
            styled_counted(&data, &limits(u64::MAX))
        } else {
            styled(&data, &limits(u64::MAX))
        };
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let original = prepare_book_v2_text_flow(&body, &nav).unwrap();
        if rich {
            assert_eq!(original.tables().len(), 2);
            assert_eq!(original.description_lists().len(), 1);
            assert!(original.tables()[0].caption_event_range().is_some());
            assert!(original.tables()[0]
                .cells()
                .iter()
                .any(|c| c.rowspan().get() == 2));
            assert_eq!(original.footnote_definitions().len(), 2);
            assert!(original.footnote_base_style.is_some());
            assert_eq!(original.named_page_breaks().len(), 1);
            assert!(!original.lists().is_empty());
        }
        for page in [None, Some(10), Some(100)] {
            let bound_values = page.map(|page| values(&original, page));
            let values = bound_values.as_deref();
            let legacy = if let Some(values) = values {
                prepare_book_v2_text_flow_with_page_references(&body, &nav, values).unwrap()
            } else {
                prepare_book_v2_text_flow(&body, &nav).unwrap()
            };
            let prior = [11, 13, 17];
            let mut observed = Observation::default();
            let flow = construct(
                &body,
                &nav,
                values,
                prior,
                u64::MAX,
                u64::MAX,
                &mut observed,
            )
            .unwrap();
            same(&flow, &legacy);
            assert_eq!(flow.source_record_charge(), intrinsic(&legacy));
            assert_eq!(observed.logical_source_record_charge(), intrinsic(&legacy));
            assert!(observed.record_charge() > prior[0] + intrinsic(&legacy));
            assert!(observed.spool_charge() > prior[1]);
            assert!(observed.work_steps() > prior[2]);
            let prepared = observed;
            flow.verify_for_observed_counted(
                &body,
                &nav,
                prepared.record_charge(),
                prepared.spool_charge(),
                prepared.work_steps(),
                u64::MAX,
                u64::MAX,
                &mut observed,
            )
            .unwrap();
            assert_eq!(observed.logical_source_record_charge(), intrinsic(&legacy));
            assert_eq!(
                observed.record_charge() - prepared.record_charge(),
                prepared.record_charge() - prior[0]
            );
            assert_eq!(
                observed.spool_charge() - prepared.spool_charge(),
                prepared.spool_charge() - prior[1]
            );
            assert!(
                observed.work_steps() - prepared.work_steps() > prepared.work_steps() - prior[2]
            );
            for paragraph in flow.paragraphs() {
                for site in paragraph.items() {
                    if let ProductionInlineContent::Text { span, utf8 } = site.content() {
                        let source =
                            &body.body().wire().text_buffers()[span.text_id().get() as usize].utf8;
                        assert!((utf8.as_ptr() as usize) >= (source.as_ptr() as usize));
                        assert!(
                            (utf8.as_ptr() as usize) + utf8.len()
                                <= (source.as_ptr() as usize) + source.len()
                        );
                    }
                }
            }
            eprintln!(
                "observed source rich={rich} counted_registry={counted_registry} page={page:?} intrinsic={} prepare={:?} verify={:?}",
                intrinsic(&legacy),
                rsw(prepared),
                rsw(observed)
            );
        }
    }
}

#[test]
fn observed_source_text_and_number_references_preserve_original_counter_text_and_unused_definitions(
) {
    for format in ["text", "number"] {
        let mut data = input();
        data["document"]["blocks"][1]["children"][1]["format"] = format.into();
        if format == "number" {
            data["document"]["number_bindings"] = json!([{
            "anchor_id":"top","owner_node_id":1,"label_node_id":2,
            "text_span":{"text_id":0,"start_byte":0,"end_byte":5}}]);
        }
        let body = styled(&data, &limits(u64::MAX));
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let legacy = prepare_book_v2_text_flow(&body, &nav).unwrap();
        let mut observed = Observation::default();
        let flow = construct(&body, &nav, None, [0; 3], u64::MAX, u64::MAX, &mut observed).unwrap();
        same(&flow, &legacy);
        assert!(flow.reference_text(NodeId::new(7)).is_some());
        assert!(flow.page_reference_text(NodeId::new(7)).is_none());
        assert_eq!(observed.logical_source_record_charge(), intrinsic(&legacy));
    }
}

#[test]
fn observed_source_recollection_rejects_foreign_roots_and_each_tampered_field_without_hash_authority(
) {
    let data = rich_input();
    let body = styled(&data, &limits(u64::MAX));
    let other = styled(&data, &limits(u64::MAX));
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let other_nav = prepare_book_v2_navigation(&body).unwrap();
    let original = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let values = values(&original, 10);
    let prior = [11, 13, 17];
    for (wrong_body, wrong_nav) in [(&other, &nav), (&body, &other_nav)] {
        let mut observed = Observation::default();
        assert!(matches!(
            original.verify_for_observed_counted(
                wrong_body,
                wrong_nav,
                prior[0],
                prior[1],
                prior[2],
                u64::MAX,
                u64::MAX,
                &mut observed
            ),
            Err(Error::Source(ProductionFlowError {
                kind: ProductionFlowErrorKind::ReceiptMismatch,
                ..
            }))
        ));
        assert_eq!(observed.record_charge(), prior[0]);
        assert_eq!(observed.spool_charge(), prior[1]);
        assert!(observed.work_steps() >= prior[2]);
        assert_eq!(observed.logical_source_record_charge(), 0);
    }
    let mut seen = Observation::default();
    assert!(matches!(
        construct(&other, &nav, None, prior, u64::MAX, u64::MAX, &mut seen),
        Err(Error::Source(ProductionFlowError {
            kind: ProductionFlowErrorKind::ReceiptMismatch,
            ..
        }))
    ));
    assert_eq!(seen.record_charge(), prior[0]);
    assert_eq!(seen.spool_charge(), prior[1]);
    assert!(seen.work_steps() >= prior[2]);
    for field in 0..17 {
        let mut flow =
            prepare_book_v2_text_flow_with_page_references(&body, &nav, &values).unwrap();
        let unchanged_hash = flow.fingerprint();
        match field {
            0 => flow.fingerprint[0] ^= 1,
            1 => flow.source_record_charge += 1,
            2 => flow.text_bytes += 1,
            3 => flow.table_record_charge += 1,
            4 => flow.events.clear(),
            5 => flow.paragraphs[0].owner = NodeId::new(99999),
            6 => flow.figures.clear(),
            7 => flow.tables.clear(),
            8 => flow.lists.clear(),
            9 => flow.list_items.clear(),
            10 => flow.description_lists.clear(),
            11 => flow.description_items.clear(),
            12 => flow.footnote_definitions.clear(),
            13 => {
                flow.generated = typaxis_text::GeneratedTextOverlay::new(
                    vec![],
                    body.body().limits(),
                    nav.retained_text_bytes(),
                )
                .unwrap()
            }
            14 => flow.page_reference_values.as_mut().unwrap()[0].1 = 100,
            15 => flow.named_page_breaks.clear(),
            16 => flow.footnote_base_style = None,
            _ => unreachable!(),
        }
        if field != 0 {
            assert_eq!(flow.fingerprint(), unchanged_hash);
        }
        let mut observed = Observation::default();
        assert!(
            matches!(
                flow.verify_for_observed_counted(
                    &body,
                    &nav,
                    prior[0],
                    prior[1],
                    prior[2],
                    u64::MAX,
                    u64::MAX,
                    &mut observed
                ),
                Err(Error::Source(ProductionFlowError {
                    kind: ProductionFlowErrorKind::ReceiptMismatch,
                    ..
                }))
            ),
            "tamper {field}"
        );
        assert!(observed.record_charge() > prior[0]);
        assert!(observed.spool_charge() > prior[1]);
        assert!(observed.work_steps() > prior[2]);
    }
}

#[test]
fn observed_source_constructor_and_full_verify_exact_one_short_caps_and_retry_keep_accepted_history(
) {
    let data = input();
    let body = styled(&data, &limits(u64::MAX));
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let original = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let values = values(&original, 100);
    let flow = prepare_book_v2_text_flow_with_page_references(&body, &nav, &values).unwrap();
    for verify in [false, true] {
        let run = |prior: [u64; 3], mr, mw, observation: &mut Observation| {
            if verify {
                flow.verify_for_observed_counted(
                    &body,
                    &nav,
                    prior[0],
                    prior[1],
                    prior[2],
                    mr,
                    mw,
                    observation,
                )
            } else {
                construct(&body, &nav, Some(&values), prior, mr, mw, observation).map(|_| ())
            }
        };
        let prior = [11, 13, 17];
        let mut full = Observation::default();
        run(prior, u64::MAX, u64::MAX, &mut full).unwrap();
        for axis in 0..3 {
            for short in [false, true] {
                let mut trial_prior = prior;
                let mut mr = u64::MAX;
                let mut mw = u64::MAX;
                match axis {
                    0 => mr = full.record_charge() - u64::from(short),
                    1 => {
                        trial_prior[1] = body.body().limits().get().max_spool_bytes
                            - (full.spool_charge() - prior[1])
                            + u64::from(short)
                    }
                    2 => mw = full.work_steps() - u64::from(short),
                    _ => unreachable!(),
                }
                let mut seen = Observation::default();
                let result = run(trial_prior, mr, mw, &mut seen);
                if short {
                    assert!(matches!(
                        (axis, result),
                        (0, Err(Error::Records { .. }))
                            | (1, Err(Error::Spool { .. }))
                            | (2, Err(Error::Work { .. }))
                    ));
                    let after = rsw(seen);
                    assert!(after.iter().zip(trial_prior).all(|(a, b)| *a >= b));
                    assert!(run(after, mr, mw, &mut seen).is_err());
                    assert!(rsw(seen).iter().zip(after).all(|(a, b)| *a >= b));
                } else {
                    result.unwrap();
                    assert_eq!(seen.record_charge(), full.record_charge());
                    assert_eq!(
                        seen.spool_charge(),
                        trial_prior[1] + full.spool_charge() - prior[1]
                    );
                    assert_eq!(seen.work_steps(), full.work_steps());
                }
            }
        }
    }
}

#[test]
fn observed_source_every_positive_accepted_constructor_and_verify_prefix_is_retained() {
    let data = minimal_input();
    let body = styled(&data, &limits(u64::MAX));
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let prior = [7, 11, 13];
    for verify in [false, true] {
        let run = |p: [u64; 3], mr, mw, o: &mut Observation| {
            if verify {
                flow.verify_for_observed_counted(&body, &nav, p[0], p[1], p[2], mr, mw, o)
            } else {
                construct(&body, &nav, None, p, mr, mw, o).map(|_| ())
            }
        };
        let mut full = Observation::default();
        let (result, trace) = with_charge_trace(|| run(prior, u64::MAX, u64::MAX, &mut full));
        result.unwrap();
        let mut before = prior;
        let mut exercised = 0;
        for (request, after) in &trace {
            if request.iter().all(|v| *v == 0) {
                before = *after;
                continue;
            }
            let axis = if request[2] > 0 {
                2
            } else if request[0] > 0 {
                0
            } else {
                1
            };
            let mut p = prior;
            let mut mr = u64::MAX;
            let mut mw = u64::MAX;
            match axis {
                0 => mr = before[0],
                1 => p[1] = body.body().limits().get().max_spool_bytes - (before[1] - prior[1]),
                2 => mw = before[2],
                _ => unreachable!(),
            }
            let shift = p[1] - prior[1];
            let mut observed = Observation::default();
            let result = run(p, mr, mw, &mut observed);
            assert!(matches!(
                (axis, result),
                (0, Err(Error::Records { .. }))
                    | (1, Err(Error::Spool { .. }))
                    | (2, Err(Error::Work { .. }))
            ));
            assert_eq!(
                rsw(observed),
                [before[0], before[1] + shift, before[2]],
                "verify={verify} prefix={exercised}"
            );
            before = *after;
            exercised += 1;
        }
        assert!(exercised > 100);
        eprintln!(
            "observed source full prefix verify={verify} positive_charges={exercised} total={:?}",
            rsw(full)
        );
    }
}

#[test]
fn observed_source_actual_storage_faults_retain_the_paid_source_and_lowering_prefixes() {
    let data = input();
    let body = styled(&data, &limits(u64::MAX));
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let prior = [7, 11, 13];
    let mut full = Observation::default();
    let ((result, attempts), trace) = with_charge_trace(|| {
        with_allocation_fault(usize::MAX, || {
            construct(&body, &nav, None, prior, u64::MAX, u64::MAX, &mut full)
        })
    });
    result.unwrap();
    assert!(attempts > 10);
    for at in 0..attempts {
        let mut seen = Observation::default();
        let (result, count) = with_allocation_fault(at, || {
            construct(&body, &nav, None, prior, u64::MAX, u64::MAX, &mut seen)
        });
        assert!(
            matches!(result, Err(Error::Allocation { .. })),
            "source fault {at}"
        );
        assert_eq!(count, at + 1);
        assert!(trace.iter().any(|(_, after)| *after == rsw(seen)));
        assert!(seen.spool_charge() > prior[1]);
    }
    // Each positive-byte lowering allocation is preceded by an accepted
    // positive-spool request. The complete successful trace therefore bounds
    // all lowering fault positions, including the final successful attempt.
    let maximum_lowering_attempts = trace.iter().filter(|(request, _)| request[1] > 0).count();
    let mut lowered_failures = 0;
    let mut completed = false;
    for at in 0..=maximum_lowering_attempts {
        let mut seen = Observation::default();
        let result = crate::semantic_container::initial_style::with_allocation_fault(at, || {
            construct(&body, &nav, None, prior, u64::MAX, u64::MAX, &mut seen)
        });
        match result {
            Ok(flow) => {
                same(&flow, &prepare_book_v2_text_flow(&body, &nav).unwrap());
                completed = true;
                break;
            }
            Err(Error::Allocation { .. }) => {
                lowered_failures += 1;
                assert!(trace.iter().any(|(_, after)| *after == rsw(seen)));
            }
            Err(error) => panic!("lowering fault {at}: {error:?}"),
        }
    }
    assert!(completed && lowered_failures > 10);
    eprintln!(
        "observed source allocation faults source={attempts} syntax_lowering={lowered_failures} paid_storage_bound={maximum_lowering_attempts}"
    );
}

#[test]
fn observed_source_preserves_existing_text_and_page_value_limits_with_real_partial_observations() {
    let data = input();
    let body = styled(&data, &limits(u64::MAX));
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let owner = NodeId::new(7);
    for values in [
        vec![],
        vec![(owner, 0)],
        vec![(owner, body.body().limits().get().max_pages + 1)],
        vec![(owner, 1), (owner, 2)],
        vec![(NodeId::new(8), 1)],
    ] {
        let expected = prepare_book_v2_text_flow_with_page_references(&body, &nav, &values)
            .err()
            .unwrap();
        let mut seen = Observation::default();
        let error = construct(
            &body,
            &nav,
            Some(&values),
            [7, 11, 13],
            u64::MAX,
            u64::MAX,
            &mut seen,
        )
        .err()
        .unwrap();
        assert_eq!(error, Error::Source(expected));
        assert!(seen.record_charge() >= 7);
        assert!(seen.spool_charge() >= 11);
        assert!(seen.work_steps() >= 13);
    }
    let generated = prepare_book_v2_text_flow_with_page_references(&body, &nav, &[(owner, 12)])
        .unwrap()
        .generated_text_bytes();
    let total = nav.retained_text_bytes() + generated;
    for short in [false, true] {
        let bound = total - u64::from(short);
        let mut raw = ResourceLimits::default();
        raw.max_text_bytes = bound;
        raw.max_text_buffer_bytes = u32::try_from(bound).unwrap();
        raw.max_shaping_context_bytes = u32::try_from(bound).unwrap();
        let base = ValidatedResourceLimits::new(raw).unwrap();
        let body = styled(&data, &base);
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let expected = prepare_book_v2_text_flow_with_page_references(&body, &nav, &[(owner, 12)]);
        let mut seen = Observation::default();
        let result = construct(
            &body,
            &nav,
            Some(&[(owner, 12)]),
            [0; 3],
            u64::MAX,
            u64::MAX,
            &mut seen,
        );
        if short {
            assert_eq!(
                result.err().unwrap(),
                Error::Source(expected.err().unwrap())
            );
        } else {
            same(&result.unwrap(), &expected.unwrap());
        }
        assert!(seen.record_charge() > 0 && seen.spool_charge() > 0 && seen.work_steps() > 0);
    }
    assert_eq!(flow.page_reference_values(), None);
}

#[test]
fn observed_source_entry_floors_precede_identity_and_preserve_all_prior_axes() {
    let data = minimal_input();
    let base = limits(u64::MAX);
    let body = styled(&data, &base);
    let other = styled(&data, &base);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    for (axis, prior, mr, mw) in [
        (0, [u64::MAX, 11, 13], u64::MAX, u64::MAX),
        (
            1,
            [7, base.get().max_spool_bytes + 1, 13],
            u64::MAX,
            u64::MAX,
        ),
        (2, [7, 11, 13], u64::MAX, 12),
    ] {
        for verify in [false, true] {
            let mut observed = Observation::default();
            let result = if verify {
                flow.verify_for_observed_counted(
                    &other,
                    &nav,
                    prior[0],
                    prior[1],
                    prior[2],
                    mr,
                    mw,
                    &mut observed,
                )
            } else {
                construct(&other, &nav, None, prior, mr, mw, &mut observed).map(|_| ())
            };
            assert!(matches!(
                (axis, result),
                (0, Err(Error::Records { .. }))
                    | (1, Err(Error::Spool { .. }))
                    | (2, Err(Error::Work { .. }))
            ));
            assert_eq!(rsw(observed), prior);
            assert_eq!(observed.logical_source_record_charge(), 0);
        }
    }
}
