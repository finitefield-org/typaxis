//! Development controls for source candidates, with no final page/PDF receipt.
//! The small host harness follows book_v2_resources_tests::{body_with_source,
//! config_with_extension} and book_v2_shaping_tests::{source_data,prepared}.
//! Assets are those helpers' exact checked-in bytes, without font substitution.
use super::*;
#[path = "book_v2_column_font_seam_tests.rs"]
mod font_seam;
#[path = "book_v2_column_lower_stages_tests.rs"]
mod lower_stages;
use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use typaxis_core::{
    ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions, HostAdmissionContext, HostPath,
    M4ResourceLimits, PdfStreamCompression, ResourceLimits, ValidatedResourceLimits,
    DEFAULT_ALLOWED_URI_SCHEMES, REGISTERED_JAPANESE_LINE_BREAK_VERSION,
    REGISTERED_UNICODE_VERSION,
};
use typaxis_document_package::DocumentPackageDecodePolicy;
use typaxis_machine_input::{book_v2::HostBookV2InputSession, MachineInputHostOptions};

const FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));
const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"
));
const COLLECTION: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/collection.bin"));
const PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/cover.bin"
));
const TEXT: &str = "Result";
const MAXIMUM: u64 = 100_000_000;
const MODE: typaxis_linebreak::JapaneseLineBreakMode =
    typaxis_linebreak::JapaneseLineBreakMode::Normal;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "typaxis-column-source-context-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn context(&self) -> HostAdmissionContext {
        HostAdmissionContext::new(
            HostPath::new(self.0.join("document-package.json")).unwrap(),
            HostPath::new(self.0.clone()).unwrap(),
            None,
            vec![],
        )
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn caps() -> M4EffectiveResourceLimits {
    let mut base = ResourceLimits::default();
    base.max_line_reshape_passes = 512;
    base.max_layout_passes = 512;
    base.max_footnote_reflows_per_page = 512;
    M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(base).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap()
}
fn changed(
    caps: &M4EffectiveResourceLimits,
    f: impl FnOnce(&mut ResourceLimits),
) -> M4EffectiveResourceLimits {
    let mut base = caps.base().get().clone();
    f(&mut base);
    M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(base).unwrap(),
        caps.extension().get().clone(),
    )
    .unwrap()
}
fn config(caps: &M4EffectiveResourceLimits) -> EffectiveConfig {
    EffectiveConfig::new_for_contract_with_m4_limits(
        typaxis_core::DocumentPackageContractId::V1_4,
        false,
        PdfStreamCompression::Flate,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        EffectiveDataVersions::new(
            REGISTERED_UNICODE_VERSION,
            REGISTERED_JAPANESE_LINE_BREAK_VERSION,
        )
        .unwrap(),
        caps.base().get().clone(),
        caps.extension().get().clone(),
    )
    .unwrap()
}
fn input(root: &Root, data: &Value, caps: &M4EffectiveResourceLimits) -> PreparedBookV2Resources {
    let source = data["text_buffers"][0]["utf8"].as_str().unwrap().as_bytes();
    let mut data = data.clone();
    data["sources"][0]["sha256"] = typaxis_core::sha256(source)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    data["sources"][0]["utf8_byte_length"] = source.len().into();
    fs::write(
        root.0.join("document-package.json"),
        serde_json::to_vec(&data).unwrap(),
    )
    .unwrap();
    fs::write(root.0.join("input.tsf"), source).unwrap();
    fs::write(root.0.join("body.bin"), FONT).unwrap();
    fs::write(root.0.join("collection.bin"), COLLECTION).unwrap();
    fs::write(root.0.join("cover.bin"), PNG).unwrap();
    let (session, raw) = HostBookV2InputSession::open(
        MachineInputHostOptions::new(
            HostPath::new(root.0.join("document-package.json")).unwrap(),
            None,
        ),
        caps.base(),
    )
    .unwrap();
    let decoded = session
        .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(caps.base()))
        .unwrap();
    let sources = session.admit_sources(&decoded, caps.base()).unwrap();
    let body =
        prepare_admitted_book_v2_body(session.finish(raw, decoded, sources).unwrap(), caps.base())
            .unwrap();
    let prepared = crate::book_v2_resources::prepare_book_v2_resources(
        body,
        &root.context(),
        &config(caps),
        caps,
    )
    .unwrap();
    let original = prepared.body().styled().body();
    assert_eq!(original.limits(), caps.base());
    assert_eq!(original.wire().text_buffers()[0].utf8.as_bytes(), source);
    let canonical = typaxis_document_package::book_v2::BookV2DocumentPackageEncoder::new()
        .encode(original.wire())
        .unwrap();
    assert_eq!(
        original.canonical_jcs_sha256(),
        typaxis_core::sha256(canonical.as_bytes())
    );
    assert_eq!(prepared.resources().fonts()[0].bytes(), FONT);
    assert_eq!(prepared.resources().fonts()[1].bytes(), COLLECTION);
    assert_eq!(prepared.resources().images()[0].bytes(), PNG);
    prepared
}
fn span() -> Value {
    json!({"source_id":0,"start_byte":0,"end_byte":TEXT.len()})
}
fn text() -> Value {
    json!({"kind":"text","node_id":0,"span":span(),"text_span":{"text_id":0,"start_byte":0,"end_byte":TEXT.len()}})
}
fn paragraph(children: Vec<Value>) -> Value {
    json!({"kind":"paragraph","node_id":0,"classes":[],"span":span(),"children":children})
}
fn page() -> Value {
    json!({"kind":"reference","node_id":0,"span":span(),"target":"page-target","format":"page"})
}
fn renumber(v: &mut Value, next: &mut u32) {
    if let Some(items) = v.as_array_mut() {
        for item in items {
            renumber(item, next);
        }
    } else if let Some(object) = v.as_object_mut() {
        if let Some(id) = object.get_mut("node_id") {
            *id = (*next).into();
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
            "footnotes",
        ] {
            if let Some(child) = object.get_mut(key) {
                renumber(child, next);
            }
        }
    }
}
fn data(rich: bool) -> Value {
    let mut data: Value = serde_json::from_slice(FIXTURE).unwrap();
    data["contract"] = "typaxis.contract/1.5".into();
    data["text_buffers"] = json!([{"text_id":0,"utf8":TEXT,"mappings":[{"kind":"identity","source_span":span(),"text_range":{"start_byte":0,"end_byte":TEXT.len()}}]}]);
    let declarations = json!([
        {"name":"font_family","important":false,"value":{"kind":"font_family_list","families":["Body"]}},
        {"name":"font_size","important":false,"value":{"kind":"length","value":12*65536}},
        {"name":"line_height","important":false,"value":{"kind":"length","value":16*65536}}
    ]);
    for (index, selector) in ["paragraph", "heading"].iter().enumerate() {
        data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":format!("context-{selector}"),"selector":selector,"source_order":index+2,"extends":null,"declarations":declarations}));
    }
    data["document"]["blocks"] = vec![paragraph(vec![text()]); 6].into();
    data["document"]["footnotes"] = json!([]);
    let m = &mut data["page_masters"]["masters"][0];
    m["width"] = (400 * 65536).into();
    m["height"] = (600 * 65536).into();
    m["trim"] = json!({"x":0,"y":0,"width":400*65536,"height":600*65536});
    m["body"] = json!({"x":20*65536,"y":60*65536,"width":360*65536,"height":48*65536});
    m["column_layout"] = json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"none"});
    if rich {
        let heading = json!({"kind":"heading","node_id":0,"classes":[],"span":span(),"level":1,"anchor_id":"page-target","children":[text()]});
        let p = paragraph(vec![
            page(),
            json!({"kind":"footnote_reference","node_id":0,"span":span(),"footnote_id":"demanded-context-note"}),
        ]);
        let row = |p: Value| json!({"node_id":0,"span":span(),"cells":[{"node_id":0,"span":span(),"colspan":1,"rowspan":1,"blocks":[p]}]});
        let table = json!({"kind":"table","node_id":0,"span":span(),"classes":[],"columns":[{"kind":"fraction","weight":1}],"caption":[paragraph(vec![text()])],"head":[row(paragraph(vec![page()]))],"body":[row(paragraph(vec![page()]))]});
        data["document"]["blocks"] = json!([heading, p, table]);
        data["document"]["footnotes"] = json!([
            {"node_id":0,"span":span(),"footnote_id":"demanded-context-note","blocks":[paragraph(vec![page()])]},
            {"node_id":0,"span":span(),"footnote_id":"unreferenced-context-note","blocks":[paragraph(vec![page()])]}
        ]);
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":20*65536,"y":300*65536,"width":360*65536,"height":120*65536});
        let mut base = data["page_masters"]["masters"][0].clone();
        base["header"] = json!({"x":10*65536,"y":0,"width":200*65536,"height":30*65536});
        base["footer"] = json!({"x":10*65536,"y":500*65536,"width":200*65536,"height":30*65536});
        let running = json!({"node_id":0,"span":span(),"blocks":[paragraph(vec![text(),json!({"kind":"soft_break","node_id":0,"span":span()}),text(),json!({"kind":"hard_break","node_id":0,"span":span()})])]});
        let mut absent = base.clone();
        absent["master_id"] = "unreachable-absent".into();
        absent["header_content"] = Value::Null;
        absent["footer_content"] = Value::Null;
        let mut unreachable = base.clone();
        unreachable["master_id"] = "unreachable-both".into();
        unreachable["header_content"] = running.clone();
        unreachable["footer_content"] = running.clone();
        base["header_content"] = running;
        base["footer_content"] = Value::Null;
        data["page_masters"]["masters"] = json!([base, absent, unreachable]);
    }
    let mut next = 0;
    renumber(&mut data["document"], &mut next);
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        for role in ["header_content", "footer_content"] {
            renumber(&mut master[role], &mut next);
        }
    }
    data
}
fn authored_owners(value: &Value) -> Vec<NodeId> {
    fn visit(value: &Value, result: &mut Vec<NodeId>) {
        if value["kind"] == "reference" && value["format"] == "page" {
            result.push(NodeId::new(value["node_id"].as_u64().unwrap() as u32));
        }
        match value {
            Value::Array(a) => {
                for v in a {
                    visit(v, result)
                }
            }
            Value::Object(o) => {
                for v in o.values() {
                    visit(v, result)
                }
            }
            _ => (),
        }
    }
    let mut owners = Vec::new();
    visit(&value["document"], &mut owners);
    owners.sort_unstable();
    owners
}
fn identity(error: E) {
    assert!(matches!(error, E::Identity), "{error:?}");
}
fn values(census: &ColumnPageReferenceCensus<'_>, page: u32) -> Vec<(NodeId, u32)> {
    census.owners().iter().map(|owner| (*owner, page)).collect()
}

#[test]
fn book_v2_column_source_context_default_path_keeps_original_roots_and_layout_counts() {
    let root = Root::new();
    let caps = caps();
    let data = data(false);
    let input = input(&root, &data, &caps);
    let mut public = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let expected = super::super::with_budgeted_book_v2_column_pages(
        &input,
        &caps,
        MODE,
        &mut public,
        |pages, search, feedback, observed| {
            assert!(feedback.matches_selected_line_widths());
            (
                pages.sequence().pages().len(),
                search.source_lines().fingerprint(),
                observed,
            )
        },
    )
    .unwrap();
    let mut private = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let actual =
        with_original_column_context(&input, &caps, MODE, &mut private, |original, budget| {
            assert!(std::ptr::eq(original.input, &input));
            assert!(std::ptr::eq(original.flow.body(), input.body().styled()));
            assert!(std::ptr::eq(
                original.flow.navigation(),
                original.navigation
            ));
            assert!(original.flow.page_reference_values().is_none());
            original.columns.verify(input.body().styled()).unwrap();
            let source = original.original_candidate();
            assert!(std::ptr::eq(source.flow, original.flow));
            super::super::with_column_candidate_pages(
                &source,
                budget,
                super::super::SourceColumns(
                    |pages: &BookV2ColumnRepeatedPages<'_, '_, '_, '_, '_>,
                     search: &mut BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
                     feedback: &BookV2ColumnWidthFeedback<'_, '_>,
                     observed| {
                        assert!(feedback.matches_selected_line_widths());
                        (
                            pages.sequence().pages().len(),
                            search.source_lines().fingerprint(),
                            observed,
                        )
                    },
                ),
            )
        })
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(public.observation(), private.observation());
    assert!(
        public.observation().line_reshape_passes() > 0 && public.observation().page_passes() > 0
    );
    eprintln!(
        "Column source-context default observation: {:?}; pages={}",
        public.observation(),
        actual.0
    );
}

#[test]
fn book_v2_column_source_context_census_covers_all_original_sites_and_all_master_rows() {
    let root = Root::new();
    let caps = caps();
    let data = data(true);
    let input = input(&root, &data, &caps);
    let expected = authored_owners(&data);
    assert_eq!(expected.len(), 5);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    with_original_column_context(&input,&caps,MODE,&mut budget,|original,budget| {
        let before=budget.observation(); let census=original.census_counted(budget)?;
        assert_eq!(census.owners(),expected); assert!(census.owners().windows(2).all(|p|p[0]<p[1]));
        assert_eq!(original.flow.footnote_definitions().len(),2); assert_eq!(original.flow.tables().len(),1);
        assert_eq!(census.regions().len(),6);
        for (index,row) in census.regions().iter().enumerate() {
            assert_eq!(row.master_index,index/2); assert_eq!(row.master_id,data["page_masters"]["masters"][index/2]["master_id"]);
            assert_eq!(row.kind,if index%2==0{BookV2PageRegionKind::Header}else{BookV2PageRegionKind::Footer});
            assert_eq!(row.page_references(),0);
            assert_eq!(row.owner.is_some(),matches!(index,0|4|5));
            let role = if index%2==0 {"header_content"} else {"footer_content"};
            let source = &data["page_masters"]["masters"][index/2][role];
            assert_eq!(row.owner,source["node_id"].as_u64().map(|id|NodeId::new(id as u32)));
            assert_eq!((row.blocks,row.inlines),if row.owner.is_some(){(1,4)}else{(0,0)});
        }
        assert_eq!(census.prefix(),prefix(&budget.observation()));
        assert_eq!(budget.observation().records-before.records,12);
        let spool=budget.observation().spool-before.spool;
        let [owner,row]=census.reservation_sizes().map(|n|n as u64);
        assert!(spool>=5*owner+6*row && spool<=15*owner+21*row);
        let original_fingerprint=original.flow.fingerprint();
        for number in [10,100] {
            let labels=values(&census,number); let before=budget.observation();
            let (result, expected) = direct_candidate_prefix(original, &census, &labels, before);
            result.unwrap();
            original.with_page_references(&census,&labels,budget,|candidate,budget| {
                assert!(std::ptr::eq(candidate.original,original));
                assert!(std::ptr::eq(candidate.original.columns,original.columns));
                assert!(std::ptr::eq(candidate.flow.body(),original.flow.body()));
                assert!(std::ptr::eq(candidate.flow.navigation(),original.navigation));
                assert_eq!(candidate.flow.page_reference_values(),Some(labels.as_slice()));
                candidate.flow.verify_for(original.flow.body(),original.navigation).unwrap();
                original.columns.verify(candidate.flow.body()).unwrap();
                for owner in census.owners() { assert_eq!(candidate.flow.page_reference_text(*owner),Some(number.to_string().as_str())); }
                assert_ne!(candidate.flow.fingerprint(),original_fingerprint);
                assert!(budget.observation().records>before.records);
                assert!(budget.observation().spool > before.spool);
                assert!(budget.observation().work > before.work + census.owners().len() as u64);
                assert_eq!(prefix(&budget.observation()), prefix(&expected));
                Ok(())
            })?;
            assert_eq!(original.flow.fingerprint(),original_fingerprint);
            assert!(original.flow.page_reference_values().is_none());
        }
        eprintln!("Column source-context census/source observation: {:?}; owners={expected:?}; region_rows={}",budget.observation(),census.regions().len());
        Ok(())
    }).unwrap();
}

#[test]
fn book_v2_column_source_context_rejects_incomplete_duplicate_reordered_wrong_and_foreign_owners() {
    let root = Root::new();
    let foreign_root = Root::new();
    let caps = caps();
    let data = data(true);
    let foreign = input(&foreign_root, &data, &caps);
    let input = input(&root, &data, &caps);
    assert_eq!(
        input.body().styled().body().canonical_jcs_sha256(),
        foreign.body().styled().body().canonical_jcs_sha256()
    );
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    with_original_column_context(&input, &caps, MODE, &mut budget, |original, budget| {
        let census = original.census_counted(budget)?;
        let correct = values(&census, 10);
        for variant in 0..4 {
            let mut supplied = correct.clone();
            match variant {
                0 => {
                    supplied.pop();
                }
                1 => supplied[1] = supplied[0],
                2 => supplied.swap(0, 1),
                3 => supplied[0].0 = NodeId::new(0),
                _ => unreachable!(),
            }
            let before = budget.observation();
            identity(
                original
                    .with_page_references(&census, &supplied, budget, |_, _| -> Result<(), E> {
                        panic!("invalid owner map delivered")
                    })
                    .unwrap_err(),
            );
            assert_eq!(budget.observation().records, before.records);
            assert_eq!(budget.observation().spool, before.spool);
            assert_eq!(
                budget.observation().work - before.work,
                if variant == 0 {
                    0
                } else if variant == 1 {
                    2
                } else {
                    1
                }
            );
        }
        with_original_column_context(&foreign, &caps, MODE, budget, |other, budget| {
            let foreign = other.census_counted(budget)?;
            let before = budget.observation();
            identity(
                original
                    .with_page_references(&foreign, &correct, budget, |_, _| -> Result<(), E> {
                        panic!("foreign source delivered")
                    })
                    .unwrap_err(),
            );
            assert_eq!(budget.observation(), before);
            Ok(())
        })?;
        Ok(())
    })
    .unwrap();
}

#[test]
fn book_v2_column_source_context_rejects_reset_history_changed_limits_and_changed_work_bound() {
    let root = Root::new();
    let caps = caps();
    let data = data(true);
    let input = input(&root, &data, &caps);
    let changed_caps = changed(&caps, |c| c.max_pages -= 1);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    with_original_column_context(&input, &caps, MODE, &mut budget, |original, budget| {
        let census = original.census_counted(budget)?;
        let labels = values(&census, 100);
        for (case, mut wrong) in [
            BookV2ColumnPageBudget::new(&caps, MAXIMUM),
            BookV2ColumnPageBudget::new(&changed_caps, MAXIMUM),
            BookV2ColumnPageBudget::new(&caps, MAXIMUM - 1),
        ]
        .into_iter()
        .enumerate()
        {
            if case > 0 {
                // Retain the authentic accepted source history, so these two
                // refusals isolate changed limits and maximum-work identity.
                wrong.inner.observation = budget.observation();
            }
            let before = wrong.observation();
            identity(
                original
                    .with_page_references(&census, &labels, &mut wrong, |_, _| -> Result<(), E> {
                        panic!("foreign budget delivered")
                    })
                    .unwrap_err(),
            );
            assert_eq!(wrong.observation(), before);
        }
        let retained = budget.observation();
        for field in 0..3 {
            budget.inner.observation = retained;
            match field {
                0 => budget.inner.observation.records = census.progress.records - 1,
                1 => budget.inner.observation.spool = census.progress.spool - 1,
                2 => budget.inner.observation.work = census.progress.work - 1,
                _ => unreachable!(),
            };
            let before = budget.observation();
            identity(
                original
                    .with_page_references(&census, &labels, budget, |_, _| -> Result<(), E> {
                        panic!("reset source history delivered")
                    })
                    .unwrap_err(),
            );
            assert_eq!(budget.observation(), before);
        }
        budget.inner.observation = retained;
        for field in 0..6 {
            let mut earlier = retained;
            match field {
                0 => earlier.candidates = 1,
                1 => earlier.balance_candidates = 1,
                2 => earlier.width_passes = 1,
                3 => earlier.width_refinements = 1,
                4 => earlier.line_passes = 1,
                5 => earlier.page_passes = 1,
                _ => unreachable!(),
            }
            assert!(!retains(&retained, &earlier));
        }
        Ok(())
    })
    .unwrap();
    let before = budget.observation();
    identity(
        with_original_column_context(
            &input,
            &changed_caps,
            MODE,
            &mut budget,
            |_, _| -> Result<(), E> { panic!("changed limits delivered") },
        )
        .unwrap_err(),
    );
    assert_eq!(budget.observation(), before);
}

#[test]
fn book_v2_column_source_context_source_failure_and_callback_failure_retain_original_prefixes() {
    let root = Root::new();
    let caps = caps();
    let data = data(true);
    let input = input(&root, &data, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    with_original_column_context(&input, &caps, MODE, &mut budget, |original, budget| {
        let census = original.census_counted(budget)?;
        for number in [0, caps.base().get().max_pages + 1] {
            let labels = values(&census, number);
            for _ in 0..2 {
                let before = budget.observation();
                let (result, expected) =
                    direct_candidate_prefix(original, &census, &labels, before);
                assert!(matches!(
                    result,
                    Err(typaxis_syntax::book_v2::BookV2PageCandidateError::Source(
                        typaxis_syntax::ProductionFlowError {
                            kind: typaxis_syntax::ProductionFlowErrorKind::ReceiptMismatch,
                            ..
                        }
                    ))
                ));
                let error = original
                    .with_page_references(&census, &labels, budget, |_, _| -> Result<(), E> {
                        panic!("invalid Page source delivered")
                    })
                    .unwrap_err();
                let E::Stage { stage, source } = error else {
                    panic!("{error:?}")
                };
                assert_eq!(stage, "column Page source");
                let error = source
                    .downcast_ref::<typaxis_syntax::ProductionFlowError>()
                    .unwrap();
                assert_eq!(
                    error.kind,
                    typaxis_syntax::ProductionFlowErrorKind::ReceiptMismatch
                );
                assert_eq!(error.owner, census.owners()[0]);
                assert!(budget.observation().records > before.records);
                assert!(budget.observation().work > before.work + census.owners().len() as u64);
                assert!(budget.observation().spool > before.spool);
                assert_eq!(prefix(&budget.observation()), prefix(&expected));
            }
        }
        let labels = values(&census, 100);
        let before = budget.observation();
        let (result, expected) = direct_candidate_prefix(original, &census, &labels, before);
        result.unwrap();
        assert!(matches!(
            original.with_page_references(&census, &labels, budget, |_, _| Err::<(), _>(E::Limit(
                "intentional candidate callback"
            ))),
            Err(E::Limit("intentional candidate callback"))
        ));
        assert!(budget.observation().records > before.records);
        assert!(budget.observation().spool > before.spool);
        assert!(budget.observation().work > before.work + census.owners().len() as u64);
        assert_eq!(prefix(&budget.observation()), prefix(&expected));
        original.with_page_references(&census, &labels, budget, |_, _| Ok(()))?;
        eprintln!(
            "Column source-context failed-source/callback retained observation: {:?}",
            budget.observation()
        );
        Ok(())
    })
    .unwrap();
}

fn budget_data() -> Value {
    let mut data = data(true);
    let blank = data["page_masters"]["masters"][1].clone();
    for index in 3..64 {
        let mut master = blank.clone();
        master["master_id"] = format!("unreachable-budget-{index}").into();
        data["page_masters"]["masters"]
            .as_array_mut()
            .unwrap()
            .push(master);
    }
    data
}
fn bounded(caps: &M4EffectiveResourceLimits, axis: usize, value: u64) -> M4EffectiveResourceLimits {
    let mut base = caps.base().get().clone();
    let mut extension = caps.extension().get().clone();
    match axis {
        0 => base.max_fragments = value,
        1 => {
            base.max_spool_bytes = value;
            extension.max_font_subset_bytes = extension.max_font_subset_bytes.min(value);
        }
        _ => unreachable!(),
    }
    M4EffectiveResourceLimits::new(ValidatedResourceLimits::new(base).unwrap(), extension).unwrap()
}
fn census_run(
    data: &Value,
    caps: &M4EffectiveResourceLimits,
    maximum: u64,
    candidate: bool,
) -> (Result<(), E>, BookV2PdfConvergenceObservation) {
    let root = Root::new();
    let input = input(&root, data, caps);
    let mut budget = BookV2ColumnPageBudget::new(caps, maximum);
    let result =
        with_original_column_context(&input, caps, MODE, &mut budget, |original, budget| {
            let census = original.census_counted(budget)?;
            if candidate {
                let labels = values(&census, 100);
                original.with_page_references(&census, &labels, budget, |_, _| Ok(()))?;
            }
            Ok(())
        });
    (result, budget.observation())
}
#[test]
fn book_v2_column_source_context_exact_and_one_short_census_budgets_keep_accepted_prefixes() {
    let caps = caps();
    let data = budget_data();
    let (result, expected) = census_run(&data, &caps, MAXIMUM, false);
    result.unwrap();
    assert!(expected.spool > 0);
    eprintln!("Column source-context actual64master census budget: {expected:?}; size scope=census requested arrays only");
    for axis in 0..3 {
        let exact = if axis == 2 {
            caps.clone()
        } else {
            bounded(
                &caps,
                axis,
                if axis == 0 {
                    expected.records
                } else {
                    expected.spool
                },
            )
        };
        let maximum = if axis == 2 { expected.work } else { MAXIMUM };
        let (result, actual) = census_run(&data, &exact, maximum, false);
        result.unwrap();
        assert_eq!(actual, expected, "exact axis{axis}");
        let short = if axis == 2 {
            caps.clone()
        } else {
            bounded(
                &caps,
                axis,
                if axis == 0 {
                    expected.records - 1
                } else {
                    expected.spool - 1
                },
            )
        };
        let maximum = if axis == 2 {
            expected.work - 1
        } else {
            MAXIMUM
        };
        let (result, actual) = census_run(&data, &short, maximum, false);
        let error = result.unwrap_err();
        assert!(matches!(error, E::Limit(_)), "axis{axis}: {error:?}");
        assert!(actual.records > 0 && actual.work > 0);
        assert!(
            actual.records <= short.base().get().max_fragments
                && actual.spool <= short.base().get().max_spool_bytes
                && actual.work <= maximum
        );
        match axis {
            0 => assert_eq!(actual.records, expected.records - 1),
            1 => {
                assert_eq!(actual.records, expected.records);
                assert!(actual.spool < expected.spool);
            }
            2 => {
                assert_eq!(actual.records, expected.records);
                assert_eq!(actual.spool, expected.spool);
                assert_eq!(actual.work, expected.work - 1);
            }
            _ => unreachable!(),
        }
        eprintln!("Column source-context census short axis{axis}: {error:?}; retained {actual:?}");
    }
}
#[test]
fn book_v2_column_source_context_exact_and_one_short_candidate_source_records_spool_and_work() {
    let caps = caps();
    let data = budget_data();
    let (result, expected) = census_run(&data, &caps, MAXIMUM, true);
    result.unwrap();
    for axis in 0..3 {
        let exact = match axis {
            0 => bounded(&caps, 0, expected.records),
            1 => bounded(&caps, 1, expected.spool),
            _ => caps.clone(),
        };
        // Changing a limit also changes canonical bytes and their charged hash
        // work. Derive the actual full run for those exact immutable roots.
        let (result, exact_expected) = census_run(&data, &exact, MAXIMUM, true);
        result.unwrap();
        assert_eq!(exact_expected.records, expected.records);
        assert_eq!(exact_expected.spool, expected.spool);
        let maximum = if axis == 2 {
            exact_expected.work
        } else {
            MAXIMUM
        };
        let (result, actual) = census_run(&data, &exact, maximum, true);
        result.unwrap();
        assert_eq!(actual, exact_expected);
        let short = match axis {
            0 => bounded(&caps, 0, expected.records - 1),
            1 => bounded(&caps, 1, expected.spool - 1),
            _ => caps.clone(),
        };
        let maximum = if axis == 2 {
            exact_expected.work - 1
        } else {
            MAXIMUM
        };
        let (result, actual) = census_run(&data, &short, maximum, true);
        let error = result.unwrap_err();
        let E::Stage { stage, source } = &error else {
            panic!("{error:?}")
        };
        assert_eq!(*stage, "column Page source");
        match axis {
            0 => {
                assert_eq!(
                    source
                        .downcast_ref::<typaxis_syntax::ProductionFlowError>()
                        .unwrap()
                        .kind,
                    typaxis_syntax::ProductionFlowErrorKind::NodeLimit
                );
                assert!(actual.records < expected.records && actual.records > 0);
            }
            1 => assert!(matches!(
                source.downcast_ref::<typaxis_syntax::book_v2::BookV2PageCandidateError>(),
                Some(typaxis_syntax::book_v2::BookV2PageCandidateError::SpoolLimit { .. })
            )),
            2 => {
                assert!(matches!(
                    source.downcast_ref::<typaxis_syntax::book_v2::BookV2PageCandidateError>(),
                    Some(typaxis_syntax::book_v2::BookV2PageCandidateError::WorkLimit { .. })
                ));
                assert_eq!(actual.records, exact_expected.records);
                assert_eq!(actual.spool, exact_expected.spool);
                assert_eq!(actual.work, exact_expected.work - 1);
            }
            _ => unreachable!(),
        }
        assert!(
            actual.records <= short.base().get().max_fragments
                && actual.spool <= short.base().get().max_spool_bytes
                && actual.work <= maximum
        );
        eprintln!("Column source-context counted candidate short axis{axis}: {error:?}; retained {actual:?}");
    }
}

// Test-host replay of the actual primitive binds the wrapper's finally merge to
// every genuine accepted R/S/W prefix. It grants no production budget credit.
fn direct_candidate_prefix(
    original: &ColumnOriginalContext<'_>,
    census: &ColumnPageReferenceCensus<'_>,
    values: &[(NodeId, u32)],
    mut total: BookV2PdfConvergenceObservation,
) -> (
    Result<(), typaxis_syntax::book_v2::BookV2PageCandidateError>,
    BookV2PdfConvergenceObservation,
) {
    for _ in census.owners() {
        step(&mut total, original.maximum, "column Page owners").unwrap();
    }
    let mut source = typaxis_syntax::book_v2::BookV2PageCandidateObservation::default();
    let result = typaxis_syntax::book_v2::prepare_book_v2_page_reference_candidate_counted(
        original.flow,
        original.flow.body(),
        original.navigation,
        values,
        total.records,
        total.spool,
        total.work,
        original.limits.base().get().max_fragments,
        original.maximum,
        &mut source,
    )
    .map(|_| ());
    total.records = total.records.max(source.record_charge());
    total.spool = total.spool.max(source.spool_charge());
    total.work = total.work.max(source.work_steps());
    (result, total)
}

#[test]
fn book_v2_column_source_context_running_region_grammar_has_no_page_reference_variant() {
    let caps = caps();
    let valid = data(true);
    use typaxis_document_package::book_v2::BookV2DocumentPackageDecoder;
    for role in ["header_content", "footer_content"] {
        let mut data = valid.clone();
        data["page_masters"]["masters"][2][role]["blocks"][0]["children"][0] = page();
        let error = BookV2DocumentPackageDecoder::new()
            .decode(
                &serde_json::to_vec(&data).unwrap(),
                &DocumentPackageDecodePolicy::new(caps.base()),
            )
            .unwrap_err();
        eprintln!("Column source-context all-master {role} Page grammar refusal: {error:?}");
    }
}

#[test]
fn book_v2_column_source_context_census_prepays_complete_new_requests_before_allocation() {
    let caps = caps();
    let data = data(true);
    let owners = authored_owners(&data);
    let mut target = Vec::<NodeId>::new();
    let mut observed = BookV2PdfConvergenceObservation::default();
    let mut requested = 0;
    for owner in owners {
        let old = observed;
        let request = if target.capacity() == target.len() {
            ((target.len() + 1) * std::mem::size_of::<NodeId>()) as u64
        } else {
            0
        };
        reserve_one(&mut target, &mut observed, &caps, MAXIMUM).unwrap();
        assert_eq!(observed.records, old.records + 1);
        assert_eq!(observed.work, old.work + 1);
        assert_eq!(observed.spool, old.spool + request);
        target.push(owner);
        requested += request;
        assert_eq!(observed.spool, requested);
    }
    assert_eq!(target, authored_owners(&data));
    let mut target = Vec::<NodeId>::new();
    let mut observed = BookV2PdfConvergenceObservation::default();
    let ceiling = std::mem::size_of::<NodeId>() as u64 - 1;
    let short = bounded(&caps, 1, ceiling);
    assert!(matches!(
        reserve_one(&mut target, &mut observed, &short, MAXIMUM),
        Err(E::Limit("column Page census spool"))
    ));
    assert_eq!((observed.records, observed.spool, observed.work), (1, 0, 0));
    assert_eq!((target.len(), target.capacity()), (0, 0));
    let mut observed = BookV2PdfConvergenceObservation::default();
    assert!(matches!(
        reserve_one(&mut target, &mut observed, &caps, 0),
        Err(E::Limit("column Page census write"))
    ));
    assert_eq!(
        (observed.records, observed.spool, observed.work),
        (1, std::mem::size_of::<NodeId>() as u64, 0)
    );
    assert_eq!((target.len(), target.capacity()), (0, 0));
}

#[derive(Debug, PartialEq, Eq)]
struct CandidateDisplayFacts {
    fingerprint: [u8; 32],
    source: [u8; 32],
    components: [[u8; 32]; 6],
    text: Vec<(u32, u32, usize, String, bool, Vec<(u16, i64, i64)>)>,
    regions: Vec<([u8; 32], u32)>,
    paints: usize,
    observation: BookV2PdfConvergenceObservation,
}
fn display_facts(
    display: &typaxis_display_list::book_v2::BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    observation: BookV2PdfConvergenceObservation,
) -> CandidateDisplayFacts {
    CandidateDisplayFacts {
        fingerprint: display.fingerprint(),
        source: display
            .source()
            .original_graph()
            .lines()
            .prepared()
            .source_flow()
            .fingerprint(),
        components: [
            display.math().fingerprint(),
            display.text().fingerprint(),
            display.numbers().fingerprint(),
            display.markers().fingerprint(),
            display.images().fingerprint(),
            display.anchors().fingerprint(),
        ],
        text: display
            .text()
            .draws()
            .iter()
            .map(|draw| {
                (
                    draw.owner().get(),
                    draw.page_index(),
                    draw.fragment_index(),
                    draw.exact_text().to_owned(),
                    draw.repeated_header(),
                    draw.glyphs()
                        .iter()
                        .map(|g| (g.original_gid().get(), g.x().raw(), g.y().raw()))
                        .collect(),
                )
            })
            .collect(),
        regions: display
            .page_regions()
            .iter()
            .map(|r| (r.fingerprint(), r.page_index()))
            .collect(),
        paints: display.paints().len(),
        observation,
    }
}
#[test]
fn book_v2_column_source_context_candidate_display_reuses_default_kernel_without_new_charges() {
    let caps = caps();
    let root = Root::new();
    let data = data(false);
    let input = input(&root, &data, &caps);
    let mut public = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let expected = super::super::with_budgeted_book_v2_column_display(
        &input,
        &caps,
        MODE,
        &mut public,
        |display, _builder, observed| display_facts(display, observed),
    )
    .unwrap();
    let mut private = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let actual =
        with_original_column_context(&input, &caps, MODE, &mut private, |original, budget| {
            let candidate = original.original_candidate();
            super::super::with_column_candidate_display(
                &candidate,
                budget,
                |display, _builder, observed| display_facts(display, observed),
            )
        })
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(private.observation(), public.observation());
    eprintln!(
        "Column candidate/default Display equality: {:?}; paints={}",
        private.observation(),
        actual.paints
    );
}
#[test]
fn book_v2_column_source_context_candidate_display_places_real_ten_and_hundred_with_original_regions(
) {
    let caps = caps();
    let root = Root::new();
    let data = data(true);
    let input = input(&root, &data, &caps);
    let expected = authored_owners(&data);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    with_original_column_context(&input, &caps, MODE, &mut budget, |original, budget| {
        let census = original.census_counted(budget)?;
        let mut previous = None;
        for number in [10, 100] {
            let labels = values(&census, number);
            let before = budget.observation();
            let facts =
                original.with_page_references(&census, &labels, budget, |candidate, budget| {
                    super::super::with_column_candidate_display(
                        candidate,
                        budget,
                        |display, _builder, observed| {
                            let actual = display
                                .source()
                                .original_graph()
                                .lines()
                                .prepared()
                                .source_flow();
                            assert!(std::ptr::eq(actual, candidate.flow));
                            assert!(std::ptr::eq(actual.body(), original.flow.body()));
                            assert!(std::ptr::eq(actual.navigation(), original.navigation));
                            assert!(std::ptr::eq(display.admitted(), input.resources()));
                            assert_eq!(actual.page_reference_values(), Some(labels.as_slice()));
                            assert!(!display.page_regions().is_empty());
                            let mut placed = std::collections::BTreeMap::<NodeId, String>::new();
                            for draw in
                                display.text().draws().iter().filter(|d| {
                                    !d.repeated_header() && expected.contains(&d.owner())
                                })
                            {
                                assert!(draw.generated_provenance().is_some());
                                assert!(draw.glyphs().iter().all(|g| g.original_gid().get() > 0));
                                placed
                                    .entry(draw.owner())
                                    .or_default()
                                    .push_str(draw.exact_text());
                            }
                            for owner in &expected[..expected.len() - 1] {
                                assert_eq!(
                                    placed.get(owner),
                                    Some(&number.to_string()),
                                    "owner{owner:?}"
                                );
                            }
                            assert!(
                                !placed.contains_key(expected.last().unwrap()),
                                "unreferenced definition remains source-only"
                            );
                            display_facts(display, observed)
                        },
                    )
                })?;
            assert!(
                facts.observation.line_passes > before.line_passes
                    && facts.observation.page_passes > before.page_passes
            );
            assert!(facts.text.iter().all(|t| t.5.iter().all(|g| g.0 > 0)));
            if let Some((source, display)) = previous {
                assert_ne!(source, facts.source);
                assert_ne!(display, facts.fingerprint);
            }
            previous = Some((facts.source, facts.fingerprint));
            eprintln!(
                "Column candidate Display actual Page={number}: {:?}; paints={}; regions={}",
                budget.observation(),
                facts.paints,
                facts.regions.len()
            );
        }
        Ok(())
    })
    .unwrap();
}
#[test]
fn book_v2_column_source_context_candidate_display_callback_errors_keep_extra_build_and_retry_prefix(
) {
    let caps = caps();
    let root = Root::new();
    let data = data(false);
    let input = input(&root, &data, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    with_original_column_context(&input, &caps, MODE, &mut budget, |original, budget| {
        let candidate = original.original_candidate();
        let before = budget.observation();
        let mut during = None;
        let result = super::super::with_column_candidate_display(
            &candidate,
            budget,
            |_display, builder, observed| {
                let _extra = builder.build().unwrap();
                assert!(
                    builder.record_charge() > observed.records
                        && builder.work_steps() > observed.work
                );
                during = Some((builder.record_charge(), builder.work_steps()));
                Err::<(), _>(E::Limit("intentional candidate Display callback"))
            },
        )?;
        assert!(matches!(
            result,
            Err(E::Limit("intentional candidate Display callback"))
        ));
        let failed = budget.observation();
        let (records, work) = during.unwrap();
        assert!(
            failed.records >= records
                && failed.work >= work
                && failed.records > before.records
                && failed.work > before.work
        );
        let facts = super::super::with_column_candidate_display(
            &candidate,
            budget,
            |display, _builder, observed| display_facts(display, observed),
        )?;
        assert!(
            budget.observation().records > failed.records
                && budget.observation().work > failed.work
        );
        assert!(
            budget.observation().line_passes > failed.line_passes
                && budget.observation().page_passes > failed.page_passes
        );
        eprintln!(
            "Column candidate Display callback failure/retry: {failed:?} -> {:?}; paints={}",
            budget.observation(),
            facts.paints
        );
        Ok(())
    })
    .unwrap();
}

#[path = "book_v2_column_cff_session_controls_tests.rs"]
mod synthetic_cid_cff_controls;

#[path = "book_v2_column_observed_original_context_tests.rs"]
mod observed_original_context;

#[path = "book_v2_column_observed_source_verification_tests.rs"]
mod observed_source_verification;

#[path = "book_v2_column_observed_native_verification_tests.rs"]
mod observed_native_verification;

#[path = "book_v2_column_observed_variant_verification_tests.rs"]
mod observed_variant_verification;

#[path = "book_v2_column_observed_region_delivery_tests.rs"]
mod observed_region_delivery;
