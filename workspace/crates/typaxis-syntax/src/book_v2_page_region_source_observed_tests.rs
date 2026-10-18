//! Direct Syntax source controls. Fixture construction, traces and allocation
//! fault hooks run outside the observed constructor. No shaping or page/PDF
//! authorization is inferred from a selected master or a source observation.
use super::super::super::source_ops::{with_allocation_fault, with_charge_trace};
use super::*;
use crate::book_v2::{
    prepare_book_v2_body, prepare_book_v2_navigation, select_book_v2_page_master,
    style_book_v2_body, style_book_v2_body_counted,
};
use serde_json::{json, Value};
use typaxis_core::ResourceLimits;
use typaxis_document_package::{
    book_v2::BookV2DocumentPackageDecoder, DocumentPackageDecodePolicy,
};

type Observation = BookV2ObservedSourceFlowObservation;
type Error = BookV2ObservedSourceFlowError;
const PRIOR: [u64; 3] = [7, 11, 13];
const NAVIGATION: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/book-navigation/job/document-package.json"));

fn limits() -> ValidatedResourceLimits {
    ValidatedResourceLimits::new(ResourceLimits::default()).unwrap()
}
fn fixture() -> Value {
    let bytes =
        typaxis_document_package::staging_book_navigation_page_region_fixture(NAVIGATION).unwrap();
    let mut data: Value = serde_json::from_slice(&bytes).unwrap();
    data["contract"] = "typaxis.contract/1.5".into();
    for selector in ["paragraph", "heading"] {
        let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
        let source_order = rules.len();
        rules.push(json!({"style_id":format!("direct-region-{selector}"),"selector":selector,
            "source_order":source_order,"extends":null,"declarations":[
                {"name":"font_family","important":false,"value":{"kind":"font_family_list","families":["Body"]}},
                {"name":"font_size","important":false,"value":{"kind":"length","value":786432}},
                {"name":"line_height","important":false,"value":{"kind":"length","value":1048576}}
            ]}));
    }
    let master = &mut data["page_masters"]["masters"][0];
    master["footer"] = json!({"height":1638400,"width":52428800,"x":6553600,"y":49152000});
    master["footer_content"] = master["header_content"].clone();
    region_ids(&mut data);
    data
}
fn maximum_id(value: &Value) -> u32 {
    match value {
        Value::Object(v) => v
            .values()
            .map(maximum_id)
            .max()
            .unwrap_or(0)
            .max(v.get("node_id").and_then(Value::as_u64).unwrap_or(0) as u32),
        Value::Array(v) => v.iter().map(maximum_id).max().unwrap_or(0),
        _ => 0,
    }
}
fn region_ids(data: &mut Value) {
    fn assign(v: &mut Value, next: &mut u32) {
        if let Some(values) = v.as_array_mut() {
            for value in values {
                assign(value, next);
            }
        } else if let Some(object) = v.as_object_mut() {
            if let Some(id) = object.get_mut("node_id") {
                *id = (*next).into();
                *next += 1;
            }
            for key in ["blocks", "children"] {
                if let Some(child) = object.get_mut(key) {
                    assign(child, next);
                }
            }
        }
    }
    let mut next = maximum_id(&data["document"]) + 1;
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        for name in ["header_content", "footer_content"] {
            assign(&mut master[name], &mut next);
        }
    }
}
fn diverse() -> Value {
    let mut data = fixture();
    let utf8 = "É日本🙂";
    let end = utf8.len();
    let hash: String = sha256(utf8.as_bytes())
        .iter()
        .map(|n| format!("{n:02x}"))
        .collect();
    data["sources"].as_array_mut().unwrap().push(
        json!({"source_id":1,"uri":"region-unicode.tsf","sha256":hash,"utf8_byte_length":end}),
    );
    let span = json!({"source_id":1,"start_byte":0,"end_byte":end});
    data["text_buffers"].as_array_mut().unwrap().push(json!({"text_id":1,"utf8":utf8,
        "mappings":[{"kind":"identity","source_span":span,"text_range":{"start_byte":0,"end_byte":end}}]}));
    let text = json!({"kind":"text","node_id":0,"span":span,"text_span":{"text_id":1,"start_byte":0,"end_byte":end}});
    let p = |children: Vec<Value>| json!({"kind":"paragraph","node_id":0,"classes":[],"span":span,"children":children});
    data["page_masters"]["masters"][0]["header_content"] = json!({"node_id":0,"span":span,"blocks":[
        p(vec![text.clone(),json!({"kind":"soft_break","node_id":0,"span":span}),text.clone(),json!({"kind":"hard_break","node_id":0,"span":span})]),
        {"kind":"heading","node_id":0,"classes":[],"span":span,"level":2,"anchor_id":null,"children":[text]},
        p(vec![])
    ]});
    let mut named = data["page_masters"]["masters"][0].clone();
    named["master_id"] = "named-region".into();
    data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .push(named);
    data["page_masters"]["selection_rules"] = json!([
        {"master_id":"named-region","parity":"any","first":null,"named_page":"region-named","source_order":0}
    ]);
    region_ids(&mut data);
    data
}
fn styled(data: &Value, limits: &ValidatedResourceLimits, counted: bool) -> StyledBookV2Body {
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(
            &serde_json::to_vec(data).unwrap(),
            &DocumentPackageDecodePolicy::new(limits),
        )
        .unwrap();
    let body = prepare_book_v2_body(decoded, limits).unwrap();
    if counted {
        let mut setup = [0; 3];
        let result = style_book_v2_body_counted(body, &mut |r, s, w| {
            for (i, n) in [r, s, w].into_iter().enumerate() {
                setup[i] += n;
            }
            Ok::<_, ()>(())
        })
        .unwrap();
        assert!(setup.iter().all(|n| *n > 0));
        result
    } else {
        style_book_v2_body(body).unwrap()
    }
}
fn run<'a>(
    selected: BookV2SelectedPageMaster<'a>,
    kind: BookV2PageRegionKind,
    navigation: &'a PreparedBookV2Navigation<'a>,
    prior: [u64; 3],
    maximum_r: u64,
    maximum_w: u64,
    observed: &mut Observation,
) -> Result<BookV2PageRegionTextFlow<'a>, Error> {
    prepare_book_v2_page_region_text_flow_observed_counted(
        selected, kind, navigation, prior[0], prior[1], prior[2], maximum_r, maximum_w, observed,
    )
}
fn rsw(o: Observation) -> [u64; 3] {
    [o.record_charge(), o.spool_charge(), o.work_steps()]
}
fn authored<'a>(
    selected: BookV2SelectedPageMaster<'a>,
    kind: BookV2PageRegionKind,
) -> &'a WirePageRegion {
    match kind {
        BookV2PageRegionKind::Header => selected.advanced().header_content.as_ref(),
        BookV2PageRegionKind::Footer => selected.advanced().footer_content.as_ref(),
    }
    .unwrap()
}
fn logical(region: &WirePageRegion) -> u64 {
    1 + region
        .blocks
        .iter()
        .map(|block| match block {
            WirePageRegionBlock::Paragraph { children, .. }
            | WirePageRegionBlock::Heading { children, .. } => 4 + children.len() as u64,
        })
        .sum::<u64>()
}
fn owner_set(region: &WirePageRegion) -> Vec<NodeId> {
    let mut result = vec![NodeId::new(0), NodeId::new(region.node_id)];
    for block in &region.blocks {
        let (owner, children) = match block {
            WirePageRegionBlock::Paragraph {
                node_id, children, ..
            }
            | WirePageRegionBlock::Heading {
                node_id, children, ..
            } => (*node_id, children),
        };
        result.push(NodeId::new(owner));
        for child in children {
            result.push(NodeId::new(match child {
                WirePageRegionInline::Text { node_id, .. }
                | WirePageRegionInline::SoftBreak { node_id, .. }
                | WirePageRegionInline::HardBreak { node_id, .. } => *node_id,
            }));
        }
    }
    result
}
fn error_owner(error: Error) -> NodeId {
    match error {
        Error::Source(e) => e.owner,
        Error::Records { owner }
        | Error::Spool { owner }
        | Error::Work { owner }
        | Error::Allocation { owner } => owner,
    }
}
fn same(a: &BookV2PageRegionTextFlow<'_>, b: &BookV2PageRegionTextFlow<'_>) {
    assert!(std::ptr::eq(a.body(), b.body()));
    assert!(std::ptr::eq(a.navigation(), b.navigation()));
    assert!(std::ptr::eq(a.source(), b.source()));
    assert!(std::ptr::eq(a.master_id().as_ptr(), b.master_id().as_ptr()));
    assert_eq!(a.kind(), b.kind());
    assert_eq!(a.master_id(), b.master_id());
    assert_eq!(a.record_charge(), b.record_charge());
    let (a, b) = (a.text_flow(), b.text_flow());
    macro_rules! fields { ($($name:ident),*$(,)?)=>{$(assert_eq!(a.$name,b.$name,stringify!($name));)*}; }
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
    assert!(std::ptr::eq(a.package, b.package));
    assert_eq!(*a.navigation, ());
    assert_eq!(*b.navigation, ());
}
fn identity_fingerprint(
    selected: BookV2SelectedPageMaster<'_>,
    kind: BookV2PageRegionKind,
) -> [u8; 32] {
    // Independently reproduce the unchanged legacy101-byte source identity.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&sha256(BOOK_V2_PAGE_REGION_FLOW_ALGORITHM.as_bytes()));
    bytes.extend_from_slice(&selected.source().body().canonical_jcs_sha256());
    bytes.extend_from_slice(&sha256(selected.master().master_id.as_bytes()));
    bytes.push(if kind == BookV2PageRegionKind::Header {
        0
    } else {
        1
    });
    bytes.extend_from_slice(&authored(selected, kind).node_id.to_be_bytes());
    assert_eq!(bytes.len(), 101);
    sha256(&bytes)
}

#[test]
fn observed_region_source_selected_fields_identity_unicode_and_101_hash_match_legacy() {
    let base = fixture();
    let rich = diverse();
    let mut empty = fixture();
    empty["page_masters"]["masters"][0]["header_content"]["blocks"] = json!([]);
    region_ids(&mut empty);
    let mut blank = fixture();
    blank["page_masters"]["masters"][0]["header_content"]["blocks"][0]["children"] = json!([]);
    region_ids(&mut blank);
    for (case, data) in [
        ("base", base),
        ("unicode", rich),
        ("empty-region", empty),
        ("empty-paragraph", blank),
    ] {
        for counted in [false, true] {
            let body = styled(&data, &limits(), counted);
            let nav = prepare_book_v2_navigation(&body).unwrap();
            for (page, name) in [(0, None), (3, Some("region-named"))] {
                if name.is_some() && case != "unicode" {
                    continue;
                }
                let selected =
                    select_book_v2_page_master(&body, page, name, &mut 0, u64::MAX).unwrap();
                assert_eq!(selected.page_index(), page);
                assert_eq!(
                    selected.master().master_id,
                    if name.is_some() {
                        "named-region"
                    } else {
                        "default"
                    }
                );
                for kind in [BookV2PageRegionKind::Header, BookV2PageRegionKind::Footer] {
                    let source = authored(selected, kind);
                    let charge = logical(source);
                    let mut seen = Observation::default();
                    let current =
                        run(selected, kind, &nav, PRIOR, u64::MAX, u64::MAX, &mut seen).unwrap();
                    let legacy =
                        prepare_book_v2_page_region_text_flow(selected, kind, &nav, PRIOR[0])
                            .unwrap();
                    same(&current, &legacy);
                    assert_eq!(current.fingerprint(), identity_fingerprint(selected, kind));
                    assert_eq!(current.record_charge(), PRIOR[0] + charge);
                    assert_eq!(seen.logical_source_record_charge(), charge);
                    assert!(seen.record_charge() > current.record_charge());
                    assert!(seen.spool_charge() > PRIOR[1] && seen.work_steps() > PRIOR[2]);
                    assert_eq!(current.text_flow().source_record_charge, 0);
                    assert!(
                        current.text_flow().tables.is_empty()
                            && current.text_flow().footnote_definitions.is_empty()
                    );
                    assert!(
                        current.text_flow().figures.is_empty()
                            && current.text_flow().lists.is_empty()
                    );
                    assert!(current.text_flow().page_reference_values.is_none());
                    assert_eq!(current.text_flow().events.len(), source.blocks.len() * 3);
                    let mut bytes = 0;
                    for paragraph in current.text_flow().paragraphs() {
                        assert!(paragraph.page_name.is_none());
                        for site in paragraph.items() {
                            assert!(site.reference().is_none() && site.link_target().is_none());
                            if let ProductionInlineContent::Text { span, utf8 } = site.content() {
                                let buffer = &body.body().wire().text_buffers()
                                    [span.text_id().get() as usize]
                                    .utf8;
                                let expected = &buffer[span.start_byte().get() as usize
                                    ..span.end_byte().get() as usize];
                                assert_eq!(utf8, expected);
                                assert_eq!(utf8.as_ptr(), expected.as_ptr());
                                bytes += utf8.len() as u64;
                            }
                        }
                    }
                    assert_eq!(current.text_flow().text_bytes, bytes);
                    eprintln!("direct region case={case} counted={counted} master={} kind={kind:?} logical={charge} full={:?}",current.master_id(),rsw(seen));
                }
            }
        }
    }
}

#[test]
fn observed_region_source_actual_exact_one_short_axes_preserve_all_three_observations() {
    let data = diverse();
    let body = styled(&data, &limits(), false);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let selected =
        select_book_v2_page_master(&body, 3, Some("region-named"), &mut 0, u64::MAX).unwrap();
    for kind in [BookV2PageRegionKind::Header, BookV2PageRegionKind::Footer] {
        let mut complete = Observation::default();
        let good = run(
            selected,
            kind,
            &nav,
            PRIOR,
            u64::MAX,
            u64::MAX,
            &mut complete,
        )
        .unwrap();
        let full = rsw(complete);
        let owners = owner_set(authored(selected, kind));
        for axis in 0..3 {
            for short in [false, true] {
                let mut p = PRIOR;
                let mr = if axis == 0 {
                    full[0] - u64::from(short)
                } else {
                    u64::MAX
                };
                let mw = if axis == 2 {
                    full[2] - u64::from(short)
                } else {
                    u64::MAX
                };
                if axis == 1 {
                    p[1] = body.body().limits().get().max_spool_bytes - (full[1] - PRIOR[1])
                        + u64::from(short);
                }
                let mut seen = Observation::default();
                let result = run(selected, kind, &nav, p, mr, mw, &mut seen);
                if short {
                    let error = result.err().unwrap();
                    assert!(owners.contains(&error_owner(error)));
                    assert!(matches!(
                        (axis, error),
                        (0, Error::Records { .. })
                            | (1, Error::Spool { .. })
                            | (2, Error::Work { .. })
                    ));
                    assert!(rsw(seen).iter().zip(p).all(|(a, b)| *a >= b));
                } else {
                    same(&good, &result.unwrap());
                    assert_eq!(seen.record_charge(), full[0]);
                    assert_eq!(seen.work_steps(), full[2]);
                    assert_eq!(seen.spool_charge(), full[1] + p[1] - PRIOR[1]);
                }
            }
        }
    }
}

#[test]
fn observed_region_source_every_positive_request_refusal_keeps_exact_prefix_and_owner() {
    let data = diverse();
    let body = styled(&data, &limits(), false);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let selected = select_book_v2_page_master(&body, 0, None, &mut 0, u64::MAX).unwrap();
    let kind = BookV2PageRegionKind::Header;
    let owners = owner_set(authored(selected, kind));
    let mut full = Observation::default();
    let (ok, trace) =
        with_charge_trace(|| run(selected, kind, &nav, PRIOR, u64::MAX, u64::MAX, &mut full));
    ok.unwrap();
    let mut before = PRIOR;
    let mut exercised = [0usize; 3];
    for (request, after) in &trace {
        for axis in 0..3 {
            if request[axis] == 0 {
                continue;
            }
            let mut p = PRIOR;
            let mut mr = u64::MAX;
            let mut mw = u64::MAX;
            match axis {
                0 => mr = before[0],
                1 => p[1] = body.body().limits().get().max_spool_bytes - (before[1] - PRIOR[1]),
                2 => mw = before[2],
                _ => unreachable!(),
            }
            let shift = p[1] - PRIOR[1];
            let mut seen = Observation::default();
            let error = run(selected, kind, &nav, p, mr, mw, &mut seen)
                .err()
                .unwrap();
            assert!(matches!(
                (axis, error),
                (0, Error::Records { .. }) | (1, Error::Spool { .. }) | (2, Error::Work { .. })
            ));
            assert!(
                owners.contains(&error_owner(error)),
                "axis={axis} error={error:?}"
            );
            assert_eq!(
                rsw(seen),
                [before[0], before[1] + shift, before[2]],
                "axis={axis} request={request:?} before={before:?}"
            );
            exercised[axis] += 1;
        }
        before = *after;
    }
    assert_eq!(before, rsw(full));
    assert!(exercised.iter().all(|n| *n > 0));
    eprintln!(
        "direct region every request axes={exercised:?}, full={:?}, requests={}",
        rsw(full),
        trace.len()
    );
}

#[test]
fn observed_region_source_actual_storage_and_lowering_faults_keep_paid_prefixes() {
    let data = diverse();
    let body = styled(&data, &limits(), false);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let selected = select_book_v2_page_master(&body, 0, None, &mut 0, u64::MAX).unwrap();
    let kind = BookV2PageRegionKind::Header;
    let owners = owner_set(authored(selected, kind));
    let mut full = Observation::default();
    let ((ok, attempts), trace) = with_charge_trace(|| {
        with_allocation_fault(usize::MAX, || {
            run(selected, kind, &nav, PRIOR, u64::MAX, u64::MAX, &mut full)
        })
    });
    ok.unwrap();
    assert!(attempts > 0);
    for at in 0..attempts {
        let mut seen = Observation::default();
        let ((result, count), prefix) = with_charge_trace(|| {
            with_allocation_fault(at, || {
                run(selected, kind, &nav, PRIOR, u64::MAX, u64::MAX, &mut seen)
            })
        });
        let error = result.err().unwrap();
        assert!(
            matches!(error, Error::Allocation { .. }),
            "source fault {at}: {error:?}"
        );
        assert!(owners.contains(&error_owner(error)));
        assert_eq!(count, at + 1);
        assert!(trace.starts_with(&prefix));
        assert_eq!(prefix.last().unwrap().1, rsw(seen));
        assert!(seen.spool_charge() > PRIOR[1]);
    }
    let bound = trace.iter().filter(|(request, _)| request[1] > 0).count();
    let mut failures = 0;
    let mut completed = false;
    for at in 0..=bound {
        let mut seen = Observation::default();
        let (result, prefix) = with_charge_trace(|| {
            crate::semantic_container::initial_style::with_allocation_fault(at, || {
                run(selected, kind, &nav, PRIOR, u64::MAX, u64::MAX, &mut seen)
            })
        });
        match result {
            Ok(flow) => {
                same(
                    &flow,
                    &prepare_book_v2_page_region_text_flow(selected, kind, &nav, PRIOR[0]).unwrap(),
                );
                assert_eq!(seen, full);
                completed = true;
                break;
            }
            Err(error) => {
                assert!(
                    matches!(error, Error::Allocation { .. }),
                    "lower fault {at}: {error:?}"
                );
                assert!(owners.contains(&error_owner(error)));
                assert!(trace.starts_with(&prefix));
                assert_eq!(prefix.last().unwrap().1, rsw(seen));
                failures += 1;
            }
        }
    }
    assert!(completed && failures > 0);
    eprintln!("direct region actual faults source={attempts} syntax-initial-style={failures} positive-S-bound={bound}");
}

#[test]
fn observed_region_source_entry_floors_foreign_navigation_and_missing_kind_preserve_order() {
    let data = fixture();
    let body = styled(&data, &limits(), false);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let foreign = styled(&data, &limits(), false);
    let foreign_nav = prepare_book_v2_navigation(&foreign).unwrap();
    let selected = select_book_v2_page_master(&body, 0, None, &mut 0, u64::MAX).unwrap();
    let mut missing_data = fixture();
    missing_data["page_masters"]["masters"][0]["footer_content"] = Value::Null;
    region_ids(&mut missing_data);
    let missing = styled(&missing_data, &limits(), false);
    let missing_nav = prepare_book_v2_navigation(&missing).unwrap();
    let absent = select_book_v2_page_master(&missing, 0, None, &mut 0, u64::MAX).unwrap();
    for (selected, kind, nav) in [
        (selected, BookV2PageRegionKind::Header, &foreign_nav),
        (absent, BookV2PageRegionKind::Footer, &missing_nav),
    ] {
        let expected = prepare_book_v2_page_region_text_flow(selected, kind, nav, PRIOR[0])
            .err()
            .unwrap();
        assert_eq!(expected.kind, ProductionFlowErrorKind::ReceiptMismatch);
        assert_eq!(expected.owner, NodeId::new(0));
        let mut seen = Observation::default();
        let error = run(selected, kind, nav, PRIOR, u64::MAX, u64::MAX, &mut seen)
            .err()
            .unwrap();
        assert_eq!(error, Error::Source(expected));
        assert_eq!(rsw(seen), [PRIOR[0], PRIOR[1], PRIOR[2] + 1]);
        assert_eq!(seen.logical_source_record_charge(), 0);
        for axis in 0..3 {
            let mut p = PRIOR;
            let mut mr = u64::MAX;
            let mut mw = u64::MAX;
            match axis {
                0 => mr = p[0] - 1,
                1 => p[1] = selected.source().body().limits().get().max_spool_bytes + 1,
                2 => mw = p[2] - 1,
                _ => unreachable!(),
            }
            let mut seen = Observation::default();
            let error = run(selected, kind, nav, p, mr, mw, &mut seen)
                .err()
                .unwrap();
            assert_eq!(
                error,
                match axis {
                    0 => Error::Records {
                        owner: NodeId::new(0)
                    },
                    1 => Error::Spool {
                        owner: NodeId::new(0)
                    },
                    _ => Error::Work {
                        owner: NodeId::new(0)
                    },
                }
            );
            assert_eq!(rsw(seen), p);
            assert_eq!(seen.logical_source_record_charge(), 0);
        }
    }
    let other_nav = prepare_book_v2_navigation(&body).unwrap();
    let mut seen = Observation::default();
    let flow = run(
        selected,
        BookV2PageRegionKind::Header,
        &nav,
        PRIOR,
        u64::MAX,
        u64::MAX,
        &mut seen,
    )
    .unwrap();
    flow.verify_for(&body, &nav).unwrap();
    assert!(flow.verify_for(&foreign, &nav).is_err());
    assert!(flow.verify_for(&body, &other_nav).is_err());
    // A new authentic navigation for this same body is a legitimate constructor
    // input; it cannot certify the previously issued flow's pointer identity.
    let mut seen = Observation::default();
    run(
        selected,
        BookV2PageRegionKind::Header,
        &other_nav,
        PRIOR,
        u64::MAX,
        u64::MAX,
        &mut seen,
    )
    .unwrap();
}

#[test]
fn observed_region_source_real_source_error_and_retry_keep_full_not_logical_history() {
    let mut data = fixture();
    data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .retain(|v| {
            !v["style_id"]
                .as_str()
                .unwrap()
                .starts_with("direct-region-")
        });
    let body = styled(&data, &limits(), false);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let selected = select_book_v2_page_master(&body, 0, None, &mut 0, u64::MAX).unwrap();
    let kind = BookV2PageRegionKind::Header;
    let owner = match &authored(selected, kind).blocks[0] {
        WirePageRegionBlock::Paragraph { node_id, .. }
        | WirePageRegionBlock::Heading { node_id, .. } => NodeId::new(*node_id),
    };
    let legacy = prepare_book_v2_page_region_text_flow(selected, kind, &nav, PRIOR[0])
        .err()
        .unwrap();
    assert_eq!(legacy.kind, ProductionFlowErrorKind::MissingTextStyle);
    assert_eq!(legacy.owner, owner);
    let mut seen = Observation::default();
    let error = run(selected, kind, &nav, PRIOR, u64::MAX, u64::MAX, &mut seen)
        .err()
        .unwrap();
    assert_eq!(error, Error::Source(legacy));
    assert!(rsw(seen).iter().zip(PRIOR).all(|(a, b)| *a > b));
    assert_eq!(
        seen.logical_source_record_charge(),
        logical(authored(selected, kind))
    );
    let before = rsw(seen);
    let mut again = Observation::default();
    let error2 = run(selected, kind, &nav, before, u64::MAX, u64::MAX, &mut again)
        .err()
        .unwrap();
    assert_eq!(error2, error);
    assert_eq!(
        rsw(again),
        [
            2 * before[0] - PRIOR[0],
            2 * before[1] - PRIOR[1],
            2 * before[2] - PRIOR[2]
        ]
    );
    assert!(again.record_charge() > before[0] + again.logical_source_record_charge());
    let good = fixture();
    let body = styled(&good, &limits(), false);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let selected = select_book_v2_page_master(&body, 0, None, &mut 0, u64::MAX).unwrap();
    let mut first = Observation::default();
    let a = run(selected, kind, &nav, PRIOR, u64::MAX, u64::MAX, &mut first).unwrap();
    let mut next = Observation::default();
    let b = run(
        selected,
        kind,
        &nav,
        rsw(first),
        u64::MAX,
        u64::MAX,
        &mut next,
    )
    .unwrap();
    assert_eq!(a.fingerprint(), b.fingerprint());
    assert_eq!(
        b.record_charge(),
        first.record_charge() + logical(authored(selected, kind))
    );
    assert_eq!(
        rsw(next),
        [
            2 * first.record_charge() - PRIOR[0],
            2 * first.spool_charge() - PRIOR[1],
            2 * first.work_steps() - PRIOR[2]
        ]
    );
    eprintln!(
        "direct region actual repeated full history first={:?} second={:?}; logical={}",
        rsw(first),
        rsw(next),
        next.logical_source_record_charge()
    );
    // The original retained buffer has only21 bytes. Reusing its genuine
    // seven-byte text span can exceed the region collector's text allowance
    // without a decoder/source-buffer limit being the reason for refusal.
    let mut repeated = fixture();
    let child = repeated["page_masters"]["masters"][0]["header_content"]["blocks"][0]["children"]
        [0]
    .clone();
    repeated["page_masters"]["masters"][0]["header_content"]["blocks"][0]["children"] =
        vec![child; 1000].into();
    region_ids(&mut repeated);
    // Calibrate only the setup lower bound from the exact same authored
    // package. All1000 region occurrences remain. This is a legal source-text
    // domain under which navigation succeeds and collected repeated text does
    // not; it is unrelated to source-storage/allocator accounting.
    let calibration_body = styled(&repeated, &limits(), false);
    let calibration_navigation = prepare_book_v2_navigation(&calibration_body).unwrap();
    let legal_text_limit = calibration_navigation
        .retained_text_bytes()
        .max(calibration_body.body().retained_text_bytes());
    assert!(legal_text_limit >= 21 && legal_text_limit < 7 * 1000);
    let mut raw = ResourceLimits::default();
    raw.max_text_bytes = legal_text_limit;
    raw.max_text_buffer_bytes = u32::try_from(legal_text_limit).unwrap();
    raw.max_shaping_context_bytes = raw.max_shaping_context_bytes.min(raw.max_text_buffer_bytes);
    let bound = ValidatedResourceLimits::new(raw).unwrap();
    let body = styled(&repeated, &bound, false);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let selected = select_book_v2_page_master(&body, 0, None, &mut 0, u64::MAX).unwrap();
    let expected_owner = match &authored(selected, kind).blocks[0] {
        WirePageRegionBlock::Paragraph { children, .. }
        | WirePageRegionBlock::Heading { children, .. } => {
            match children[(legal_text_limit / 7) as usize] {
                WirePageRegionInline::Text { node_id, .. } => NodeId::new(node_id),
                _ => unreachable!(),
            }
        }
    };
    let legacy = prepare_book_v2_page_region_text_flow(selected, kind, &nav, PRIOR[0])
        .err()
        .unwrap();
    assert_eq!(legacy.kind, ProductionFlowErrorKind::TextLimit);
    assert_eq!(legacy.owner, expected_owner);
    let mut seen = Observation::default();
    let error = run(selected, kind, &nav, PRIOR, u64::MAX, u64::MAX, &mut seen)
        .err()
        .unwrap();
    assert_eq!(error, Error::Source(legacy));
    assert!(rsw(seen).iter().zip(PRIOR).all(|(a, b)| *a > b));
    assert_eq!(
        seen.logical_source_record_charge(),
        logical(authored(selected, kind))
    );
}
