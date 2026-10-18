use super::*;
use crate::book_v2::{prepare_book_v2_body, prepare_book_v2_navigation, style_book_v2_body};
use serde_json::{json, Value};
use typaxis_core::ResourceLimits;
use typaxis_document_package::{
    book_v2::BookV2DocumentPackageDecoder, DocumentPackageDecodePolicy,
};

const COMBINED: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json"));
fn input() -> Value {
    let mut data: Value = serde_json::from_slice(COMBINED).unwrap();
    data["contract"] = "typaxis.contract/1.5".into();
    data
}
fn limits(spool: u64) -> ValidatedResourceLimits {
    ValidatedResourceLimits::new(ResourceLimits {
        max_spool_bytes: if spool == u64::MAX {
            ResourceLimits::default().max_spool_bytes
        } else {
            spool
        },
        ..ResourceLimits::default()
    })
    .unwrap()
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
fn page_values(flow: &PreparedBookV2TextFlow<'_>, page: u32) -> Vec<(NodeId, u32)> {
    let mut values: Vec<_> = flow
        .paragraphs()
        .iter()
        .flat_map(|p| p.items())
        .filter_map(|site| {
            matches!(
                site.reference(),
                Some(ProductionInlineReference::Anchor {
                    format: ProductionReferenceFormat::Page,
                    ..
                })
            )
            .then_some((site.owner(), page))
        })
        .collect();
    values.sort_unstable_by_key(|v| v.0);
    values
}
fn assert_same(left: &PreparedBookV2TextFlow<'_>, right: &PreparedBookV2TextFlow<'_>) {
    assert!(std::ptr::eq(left.body(), right.body()));
    assert!(std::ptr::eq(left.navigation(), right.navigation()));
    macro_rules! fields { ($($field:ident),* $(,)?) => { $(assert_eq!(left.$field, right.$field, stringify!($field));)* }; }
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
// This counts original family carriers independently of the copy callbacks and
// the prepaid source permit. Every copied style family slot is additional R.
fn family_records(flow: &PreparedBookV2TextFlow<'_>) -> u64 {
    let families =
        |s: &SemanticContainerInheritanceStyle| s.font_families().map_or(0, |s| s.len() as u64);
    flow.paragraphs()
        .iter()
        .map(|p| families(p.style()))
        .sum::<u64>()
        + flow
            .figures()
            .iter()
            .map(|p| families(p.style()))
            .sum::<u64>()
        + flow
            .tables()
            .iter()
            .map(|p| families(p.style()))
            .sum::<u64>()
        + flow
            .lists()
            .iter()
            .map(|p| families(p.style()))
            .sum::<u64>()
        + flow
            .description_lists()
            .iter()
            .map(|p| families(p.style()))
            .sum::<u64>()
        + flow.footnote_base_style.as_ref().map_or(0, families)
}
fn copy<'a>(
    flow: &PreparedBookV2TextFlow<'a>,
    body: &'a StyledBookV2Body,
    nav: &'a PreparedBookV2Navigation<'a>,
    values: &[(NodeId, u32)],
    prior: [u64; 3],
    maximum_records: u64,
    maximum_work: u64,
    observed: &mut BookV2PageCandidateObservation,
) -> Result<PreparedBookV2TextFlow<'a>> {
    prepare_book_v2_page_reference_candidate_counted(
        flow,
        body,
        nav,
        values,
        prior[0],
        prior[1],
        prior[2],
        maximum_records,
        maximum_work,
        observed,
    )
}

#[test]
fn authentic_page_copy_equals_all_legacy_fields_and_keeps_logical_records_separate() {
    for page in [1, 10, 100] {
        let data = input();
        let body = styled(&data, &limits(u64::MAX));
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let original = prepare_book_v2_text_flow(&body, &nav).unwrap();
        let original_hash = original.fingerprint();
        let values = page_values(&original, page);
        let legacy = prepare_book_v2_text_flow_with_page_references(&body, &nav, &values).unwrap();
        let prior = [11, 13, 17];
        let mut observed = BookV2PageCandidateObservation::default();
        let candidate = copy(
            &original,
            &body,
            &nav,
            &values,
            prior,
            u64::MAX,
            u64::MAX,
            &mut observed,
        )
        .unwrap();
        assert_same(&candidate, &legacy);
        assert_eq!(
            candidate.source_record_charge(),
            original.source_record_charge() + 3 * values.len() as u64
        );
        assert_eq!(
            observed.record_charge(),
            prior[0] + legacy.source_record_charge() + family_records(&original)
        );
        assert!(observed.spool_charge() > prior[1]);
        assert!(observed.work_steps() > prior[2]);
        candidate.verify_for(&body, &nav).unwrap();
        assert_eq!(original.fingerprint(), original_hash);
        assert!(original.page_reference_values().is_none());
        for (source, copied) in original.paragraphs().iter().zip(candidate.paragraphs()) {
            assert_ne!(source.items().as_ptr(), copied.items().as_ptr());
            for (source, copied) in source.items().iter().zip(copied.items()) {
                if let (
                    ProductionInlineContent::Text { utf8: a, .. },
                    ProductionInlineContent::Text { utf8: b, .. },
                ) = (source.content(), copied.content())
                {
                    assert_eq!(a.as_ptr(), b.as_ptr());
                }
            }
        }
        for source in original.generated.buffers() {
            let copied = candidate.generated.buffer(source.key()).unwrap();
            assert_eq!(source.utf8(), copied.utf8());
            if !source.utf8().is_empty() {
                assert_ne!(source.utf8().as_ptr(), copied.utf8().as_ptr());
            }
        }
    }
}

fn renumber(value: &mut Value, next: &mut u32) {
    if let Some(values) = value.as_array_mut() {
        for value in values {
            renumber(value, next);
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
                renumber(value, next);
            }
        }
    }
}
fn refresh_outline(data: &mut Value) {
    fn find(value: &Value, anchor: &str) -> Option<Value> {
        match value {
            Value::Array(values) => values.iter().find_map(|v| find(v, anchor)),
            Value::Object(object) => {
                if object.get("anchor_id").and_then(Value::as_str) == Some(anchor) {
                    return object.get("node_id").cloned();
                }
                object.values().find_map(|v| find(v, anchor))
            }
            _ => None,
        }
    }
    let document = data["document"].clone();
    for entry in data["outline"]["entries"].as_array_mut().unwrap() {
        entry["source_node_id"] = find(&document, entry["destination"].as_str().unwrap()).unwrap();
    }
}
fn rich_input() -> Value {
    let mut data = input();
    let paragraph = data["document"]["blocks"][3]["items"][0]["blocks"][0].clone();
    let mut term = paragraph.clone();
    term.as_object_mut().unwrap().remove("kind");
    data["document"]["blocks"].as_array_mut().unwrap().push(json!({
        "kind":"description_list","node_id":0,"span":paragraph["span"],"classes":[],
        "items":[{"node_id":0,"span":paragraph["span"],"term":term,"blocks":[paragraph.clone()]}]
    }));
    let nested = data["document"]["blocks"][6].clone();
    data["document"]["blocks"][6]["body"][0]["cells"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(nested);
    data["document"]["blocks"][6]["caption"] = json!([paragraph]);
    let order = data["style_sheet"]["rules"].as_array().unwrap().len();
    let mut description = data["style_sheet"]["rules"][0].clone();
    description["style_id"] = "candidate-description".into();
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
    // A genuine unreferenced empty definition exercises the original base style.
    let mut empty = data["document"]["footnotes"][0].clone();
    empty["footnote_id"] = "candidate-unused".into();
    empty["blocks"] = json!([]);
    data["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(empty);
    renumber(&mut data["document"], &mut 0);
    refresh_outline(&mut data);
    data
}

#[test]
fn authentic_page_copy_retains_nested_span_caption_description_language_and_named_breaks() {
    let data = rich_input();
    let body = styled(&data, &limits(u64::MAX));
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let original = prepare_book_v2_text_flow(&body, &nav).unwrap();
    assert_eq!(original.tables().len(), 2);
    assert_eq!(original.description_lists().len(), 1);
    assert!(original.tables()[0].caption_event_range().is_some());
    assert!(original.tables()[0]
        .cells()
        .iter()
        .any(|c| c.rowspan().get() == 2));
    assert!(original.footnote_base_style.is_some());
    assert_eq!(original.named_page_breaks().len(), 1);
    let values = page_values(&original, 100);
    let legacy = prepare_book_v2_text_flow_with_page_references(&body, &nav, &values).unwrap();
    let mut observed = BookV2PageCandidateObservation::default();
    let candidate = copy(
        &original,
        &body,
        &nav,
        &values,
        [0; 3],
        u64::MAX,
        u64::MAX,
        &mut observed,
    )
    .unwrap();
    assert_same(&candidate, &legacy);
    assert_eq!(
        observed.record_charge(),
        legacy.source_record_charge() + family_records(&original)
    );
    candidate.verify_for(&body, &nav).unwrap();
}

#[test]
fn authentic_page_copy_rejects_foreign_roots_labelled_source_and_invalid_complete_maps() {
    let data = input();
    let configured = limits(u64::MAX);
    let body = styled(&data, &configured);
    let other = styled(&data, &configured);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let other_nav = prepare_book_v2_navigation(&body).unwrap();
    let original = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let values = page_values(&original, 10);
    for (body, nav) in [(&other, &nav), (&body, &other_nav)] {
        let mut observed = BookV2PageCandidateObservation::default();
        assert_eq!(
            copy(
                &original,
                body,
                nav,
                &values,
                [11, 13, 17],
                u64::MAX,
                u64::MAX,
                &mut observed
            )
            .err()
            .unwrap(),
            source_error(ProductionFlowErrorKind::ReceiptMismatch, NodeId::new(0))
        );
        assert_eq!(
            [
                observed.record_charge(),
                observed.spool_charge(),
                observed.work_steps()
            ],
            [11, 13, 17]
        );
    }
    let labelled = prepare_book_v2_text_flow_with_page_references(&body, &nav, &values).unwrap();
    let mut observed = BookV2PageCandidateObservation::default();
    assert!(copy(
        &labelled,
        &body,
        &nav,
        &values,
        [0; 3],
        u64::MAX,
        u64::MAX,
        &mut observed
    )
    .is_err());
    for values in [
        vec![],
        vec![(NodeId::new(7), 0)],
        vec![(NodeId::new(7), configured.get().max_pages + 1)],
        vec![(NodeId::new(8), 1)],
        vec![(NodeId::new(7), 1), (NodeId::new(7), 2)],
        vec![(NodeId::new(8), 1), (NodeId::new(7), 1)],
        vec![(NodeId::new(7), 1), (NodeId::new(8), 1)],
    ] {
        let mut observed = BookV2PageCandidateObservation::default();
        let error = copy(
            &original,
            &body,
            &nav,
            &values,
            [0; 3],
            u64::MAX,
            u64::MAX,
            &mut observed,
        )
        .err()
        .unwrap();
        assert!(matches!(
            error,
            BookV2PageCandidateError::Source(ProductionFlowError {
                kind: ProductionFlowErrorKind::ReceiptMismatch,
                ..
            })
        ));
        assert_eq!(original.page_reference_values(), None);
    }
}

#[test]
fn authentic_page_copy_exact_and_one_short_limits_keep_every_accepted_prefix() {
    let data = input();
    let body = styled(&data, &limits(u64::MAX));
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let original = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let values = page_values(&original, 100);
    let prior = [11, 13, 17];
    let mut full = BookV2PageCandidateObservation::default();
    copy(
        &original,
        &body,
        &nav,
        &values,
        prior,
        u64::MAX,
        u64::MAX,
        &mut full,
    )
    .unwrap();
    for (maximum_r, maximum_w, succeeds) in [
        (full.records, full.work, true),
        (full.records - 1, full.work, false),
        (full.records, full.work - 1, false),
    ] {
        let mut observed = BookV2PageCandidateObservation::default();
        let result = copy(
            &original,
            &body,
            &nav,
            &values,
            prior,
            maximum_r,
            maximum_w,
            &mut observed,
        );
        assert_eq!(result.is_ok(), succeeds);
        assert!(observed.records >= prior[0] && observed.records <= maximum_r);
        assert!(observed.spool >= prior[1] && observed.spool <= full.spool);
        assert!(observed.work >= prior[2] && observed.work <= maximum_w);
        if succeeds {
            assert_eq!(observed, full);
        } else {
            let prefix = observed;
            assert!(copy(
                &original,
                &body,
                &nav,
                &values,
                [prefix.records, prefix.spool, prefix.work],
                maximum_r,
                maximum_w,
                &mut observed
            )
            .is_err());
            assert!(
                observed.records >= prefix.records
                    && observed.spool >= prefix.spool
                    && observed.work >= prefix.work
            );
        }
    }
    for (cap, succeeds) in [(full.spool, true), (full.spool - 1, false)] {
        let body = styled(&data, &limits(cap));
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let original = prepare_book_v2_text_flow(&body, &nav).unwrap();
        let mut observed = BookV2PageCandidateObservation::default();
        let result = copy(
            &original,
            &body,
            &nav,
            &values,
            prior,
            full.records,
            full.work,
            &mut observed,
        );
        assert_eq!(result.is_ok(), succeeds);
        assert!(observed.spool <= cap);
        if !succeeds {
            assert!(matches!(
                result.err().unwrap(),
                BookV2PageCandidateError::SpoolLimit { .. }
            ));
        }
    }
}

#[test]
fn authentic_page_copy_per_request_and_overlay_allowances_cannot_pay_family_records() {
    let owner = NodeId::new(3);
    let mut observed = BookV2PageCandidateObservation::default();
    let mut copy = CandidateCopy {
        observed: &mut observed,
        maximum_records: 10,
        maximum_spool: 7,
        maximum_work: 100,
    };
    let (mut structural, mut overlay) = copy.prepay(5, 2).unwrap();
    assert!(copy.vector::<u64>(owner, 1, &mut structural).is_err());
    assert_eq!(copy.observed.records, 5);
    assert_eq!(copy.observed.spool, 0);
    assert!(copy.overlay_request(owner, 1, 0, 0, &mut overlay).is_err());
    copy.overlay_request(owner, 2, 0, 0, &mut overlay).unwrap();
    assert!(copy.overlay_request(owner, 2, 0, 0, &mut overlay).is_err());
    assert_eq!(copy.observed.records, 5);
    copy.maximum_spool = 100;
    copy.maximum_work = 0;
    let mut permit = SourceRecordPermit { remaining: 1 };
    assert!(copy.vector::<u64>(owner, 1, &mut permit).is_err());
    assert_eq!(copy.observed.spool, 8);
    assert_eq!(copy.observed.work, 0);
    // Fresh style scratch records must hit the R ceiling, even though unused
    // structural prepaid slots remain. No internal credit can pay this charge.
    copy.maximum_records = 5;
    copy.maximum_work = 100;
    assert!(copy.charge(owner, 1, 0, 0).is_err());
    assert_eq!(copy.observed.records, 5);
}

#[test]
fn authentic_page_copy_preserves_text_number_and_empty_page_maps_after_original_drop() {
    for format in ["text", "number"] {
        let mut data = input();
        data["document"]["blocks"][1]["children"][1]["format"] = format.into();
        if format == "number" {
            data["document"]["number_bindings"] = json!([{
                "anchor_id":"top","owner_node_id":1,"label_node_id":2,
                "text_span":{"text_id":0,"start_byte":0,"end_byte":5}
            }]);
        }
        let body = styled(&data, &limits(u64::MAX));
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let legacy = prepare_book_v2_text_flow_with_page_references(&body, &nav, &[]).unwrap();
        let candidate = {
            let original = prepare_book_v2_text_flow(&body, &nav).unwrap();
            assert!(page_values(&original, 10).is_empty());
            let mut observed = BookV2PageCandidateObservation::default();
            let candidate = copy(
                &original,
                &body,
                &nav,
                &[],
                [0; 3],
                u64::MAX,
                u64::MAX,
                &mut observed,
            )
            .unwrap();
            assert_eq!(
                observed.record_charge(),
                legacy.source_record_charge() + family_records(&original)
            );
            candidate
        };
        assert_same(&candidate, &legacy);
        assert_eq!(
            candidate.reference_text(NodeId::new(7)),
            legacy.reference_text(NodeId::new(7))
        );
        assert!(candidate.reference_text(NodeId::new(7)).is_some());
        candidate.verify_for(&body, &nav).unwrap();
    }
}
