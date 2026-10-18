//! Actual full-PDF Page-label convergence controls. Test-host traces borrow only
//! authentic attempt diagnostics; they do not supply layout or budget authority.
use super::*;
// BEGIN PINNED FIXTURE-ONLY HARNESS
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
            "typaxis-column-convergence-{}-{}",
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

// END PINNED FIXTURE-ONLY HARNESS

#[derive(Clone, Debug, PartialEq, Eq)]
struct Attempt {
    values: Vec<(NodeId, u32)>,
    references: Vec<(NodeId, u32, Option<u32>, bool)>,
    display: [u8; 32],
    pdf: [u8; 32],
    bytes: Vec<u8>,
    constructors: [[u64; 4]; 14],
    observation: BookV2PdfConvergenceObservation,
    decision: ColumnLabelDecision,
}
impl Attempt {
    fn from_actual(actual: ColumnPdfAttempt<'_>) -> Self {
        Self {
            values: actual.values.to_vec(),
            references: actual
                .references
                .iter()
                .map(|r| {
                    (
                        r.owner(),
                        r.candidate_page(),
                        r.target_page(),
                        r.is_placed(),
                    )
                })
                .collect(),
            display: actual.display_fingerprint,
            pdf: actual.pdf_fingerprint,
            bytes: actual.pdf_bytes.to_vec(),
            constructors: actual.constructors,
            observation: actual.observation,
            decision: actual.decision,
        }
    }
}

fn quartet(v: BookV2PdfConvergenceObservation) -> [u64; 4] {
    [v.records, v.spool, v.output, v.work]
}

fn forward_data(forced: usize) -> (Value, NodeId, NodeId) {
    let mut value = data(false);
    let mut blocks = vec![paragraph(vec![page()])];
    blocks.extend(
        (0..forced).map(|_| json!({"kind":"page_break","node_id":0,"classes":[],"span":span()})),
    );
    blocks.push(json!({"kind":"heading","node_id":0,"classes":[],"span":span(),"level":1,"anchor_id":"page-target","children":[text()]}));
    value["document"]["blocks"] = blocks.into();
    renumber(&mut value["document"], &mut 0);
    let paragraph = NodeId::new(value["document"]["blocks"][0]["node_id"].as_u64().unwrap() as u32);
    let reference = NodeId::new(
        value["document"]["blocks"][0]["children"][0]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    (value, paragraph, reference)
}

#[derive(Debug, PartialEq, Eq)]
struct LabelMetrics {
    text: String,
    advance: i64,
    gids: Vec<u16>,
    lines: usize,
    required: Vec<i64>,
    line_heights: Vec<i64>,
}
fn label_metrics(
    display: &typaxis_display_list::book_v2::BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    paragraph: NodeId,
    reference: NodeId,
) -> LabelMetrics {
    let mut text = String::new();
    let mut advance = 0i64;
    let mut gids = Vec::new();
    for draw in display
        .text()
        .draws()
        .iter()
        .filter(|d| d.owner() == reference && !d.repeated_header())
    {
        assert!(draw.generated_provenance().is_some());
        text.push_str(draw.exact_text());
        for glyph in draw.cluster().glyphs() {
            advance += glyph.glyph().advance_x.raw();
            gids.push(glyph.glyph().original_gid.get());
        }
    }
    let graph = display.source().original_graph();
    let selected = graph
        .lines()
        .paragraphs()
        .iter()
        .find(|p| p.owner() == paragraph)
        .unwrap()
        .selected()
        .unwrap();
    LabelMetrics {
        text,
        advance,
        gids,
        lines: selected.lines().len(),
        required: selected
            .lines()
            .iter()
            .map(|line| line.required_inline_size().get().raw())
            .collect(),
        line_heights: selected
            .lines()
            .iter()
            .map(|line| line.line().metrics().line_height().get().raw())
            .collect(),
    }
}
fn source_label_metrics(
    prepared: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
    paragraph: NodeId,
    reference: NodeId,
    number: u32,
) -> LabelMetrics {
    let mut budget = BookV2ColumnPageBudget::new(caps, MAXIMUM);
    source_context::with_original_column_context(
        prepared,
        caps,
        MODE,
        &mut budget,
        |original, budget| {
            let census = original.census_counted(budget)?;
            let values: Vec<_> = census
                .owners()
                .iter()
                .map(|owner| (*owner, number))
                .collect();
            original.with_page_references(&census, &values, budget, |candidate, budget| {
                with_column_candidate_display(candidate, budget, |display, _, _| {
                    label_metrics(display, paragraph, reference)
                })
            })
        },
    )
    .unwrap()
}

#[test]
fn book_v2_column_converged_pdf_fixture_harness_is_the_pinned_original_admission_prefix() {
    let original = include_str!("book_v2_column_source_context_tests.rs");
    let original_start = original.find("use serde_json::{json, Value};").unwrap();
    let original_end = original.find("fn authored_owners(").unwrap();
    // The unchanged fixture prefix is pinned to the archived b55bc265 source.
    // Later candidate accounting controls may evolve outside this harness.
    assert_eq!(
        typaxis_core::sha256(original[original_start..original_end].as_bytes()),
        [
            0xa5, 0x9f, 0xa9, 0x2f, 0x4e, 0x0a, 0x2e, 0x4c, 0xa7, 0x60, 0x6d, 0xae, 0x23, 0x05,
            0xe5, 0x04, 0xac, 0x34, 0xbd, 0xae, 0xa5, 0x80, 0xae, 0xd6, 0xb2, 0x5b, 0x4c, 0x67,
            0x26, 0xdc, 0x3e, 0x00
        ],
    );
    let expected = original[original_start..original_end].replace(
        "typaxis-column-source-context-",
        "typaxis-column-convergence-",
    );
    let current = include_str!("book_v2_column_converged_pdf_tests.rs");
    let start = current
        .find("// BEGIN PINNED FIXTURE-ONLY HARNESS\n")
        .unwrap()
        + "// BEGIN PINNED FIXTURE-ONLY HARNESS\n".len();
    let end = current.find("// END PINNED FIXTURE-ONLY HARNESS").unwrap();
    assert_eq!(current[start..end].trim(), expected.trim());
}

#[test]
fn book_v2_column_converged_pdf_without_page_sites_delivers_one_actual_pdf() {
    let root = Root::new();
    let caps = caps();
    let prepared = input(&root, &data(false), &caps);
    let mut old = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let expected = super::super::pdf_driver::with_budgeted_book_v2_column_pdf(
        &prepared,
        &caps,
        MODE,
        &mut old,
        4,
        |bundle, _, _| {
            (
                bundle.pdf().bytes().to_vec(),
                bundle.display().fingerprint(),
                bundle.pdf().fingerprint(),
            )
        },
    )
    .unwrap();
    let mut budget = BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
    let mut attempts = Vec::new();
    let (actual, delivered) = with_converged_column_pdf_observed(
        &prepared,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, observed| {
            assert!(std::ptr::eq(
                bundle.display().admitted(),
                prepared.resources()
            ));
            assert!(bundle.pdf().page_references().is_empty());
            (
                (
                    bundle.pdf().bytes().to_vec(),
                    bundle.display().fingerprint(),
                    bundle.pdf().fingerprint(),
                ),
                observed,
            )
        },
        |attempt| attempts.push(Attempt::from_actual(attempt)),
    )
    .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(attempts.len(), 1);
    assert!(attempts[0].values.is_empty() && attempts[0].references.is_empty());
    assert_eq!(attempts[0].decision, ColumnLabelDecision::Ready);
    assert_eq!(delivered, budget.observation());
    assert_eq!(delivered.candidate_passes(), 1);
    assert!(delivered.page_passes() >= 2);
    assert_eq!(budget.cff_program_statistics().operations_used, 0);
    assert_eq!(budget.cff_program_statistics().cached_glyphs, 0);
    eprintln!(
        "Column full convergence noPage: {:?}; attempts={}",
        quartet(delivered),
        attempts.len()
    );
}

#[test]
fn book_v2_column_converged_pdf_forward_page_ten_replaces_real_glyphs_and_confirms_two_complete_states(
) {
    let root = Root::new();
    let caps = caps();
    let (authored, paragraph, reference) = forward_data(9);
    let prepared = input(&root, &authored, &caps);
    let one = source_label_metrics(&prepared, &caps, paragraph, reference, 1);
    let ten = source_label_metrics(&prepared, &caps, paragraph, reference, 10);
    assert_eq!(one.text, "1");
    assert_eq!(ten.text, "10");
    assert!(one.advance > 0 && ten.advance > one.advance);
    assert!(one.gids.iter().chain(&ten.gids).all(|gid| *gid > 0));
    let mut budget = BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
    let mut attempts = Vec::new();
    let observed = with_converged_column_pdf_observed(
        &prepared,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, observed| {
            assert!(std::ptr::eq(
                bundle.display().admitted(),
                prepared.resources()
            ));
            assert_eq!(label_metrics(bundle.display(), paragraph, reference), ten);
            assert!(bundle.pdf().page_reference_labels_match());
            assert_eq!(
                bundle.display().source().source().geometry().pages().len(),
                10
            );
            observed
        },
        |attempt| attempts.push(Attempt::from_actual(attempt)),
    )
    .unwrap();
    assert_eq!(attempts.len(), 3);
    assert_eq!(
        attempts
            .iter()
            .map(|a| a.values.clone())
            .collect::<Vec<_>>(),
        vec![
            vec![(reference, 1)],
            vec![(reference, 10)],
            vec![(reference, 10)]
        ]
    );
    assert_eq!(
        attempts.iter().map(|a| a.decision).collect::<Vec<_>>(),
        vec![
            ColumnLabelDecision::Retry,
            ColumnLabelDecision::Retry,
            ColumnLabelDecision::Ready
        ]
    );
    for attempt in &attempts {
        assert_eq!(attempt.references.len(), 1);
        assert_eq!(
            attempt.references[0],
            (reference, attempt.values[0].1, Some(10), true)
        );
        assert!(attempt.bytes.starts_with(b"%PDF-"));
    }
    assert_ne!(attempts[0].display, attempts[1].display);
    assert_ne!(attempts[0].pdf, attempts[1].pdf);
    assert_eq!(
        (attempts[1].display, attempts[1].pdf),
        (attempts[2].display, attempts[2].pdf)
    );
    assert_eq!(attempts[1].bytes, attempts[2].bytes);
    assert_eq!(observed.candidate_passes(), 3);
    assert_eq!(observed, budget.observation());
    for pair in attempts.windows(2) {
        let before = pair[0].observation;
        let after = pair[1].observation;
        assert!(
            after.records > before.records
                && after.spool > before.spool
                && after.work > before.work
        );
        assert!(after.line_passes > before.line_passes && after.page_passes > before.page_passes);
    }
    eprintln!(
        "Column actual forwardPage10 convergence: {:?}; labeladvance={}->{}; attempts={}",
        quartet(observed),
        one.advance,
        ten.advance,
        attempts.len()
    );
}

fn first_character() -> Value {
    let mut atom = text();
    atom["text_span"]["end_byte"] = 1.into();
    atom
}
fn wrapping_data(forced: usize, filler: bool) -> (Value, NodeId, NodeId) {
    let (mut authored, _, _) = forward_data(forced);
    let mut prefix = text();
    prefix["text_span"]["end_byte"] = 2.into();
    authored["document"]["blocks"][0]["children"] = json!([
        prefix,
        {"kind":"soft_break","node_id":0,"span":span()},
        page(),
    ]);
    // The target is a deliberately authored one-character heading, so the
    // longest ordinary atom also fits the measured narrow-column interval.
    authored["document"]["blocks"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["children"] = json!([first_character()]);
    if filler {
        authored["document"]["blocks"]
            .as_array_mut()
            .unwrap()
            .insert(1, paragraph(vec![first_character()]));
    }
    renumber(&mut authored["document"], &mut 0);
    let paragraph = NodeId::new(
        authored["document"]["blocks"][0]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    let reference = NodeId::new(
        authored["document"]["blocks"][0]["children"][2]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    (authored, paragraph, reference)
}
fn calibrated_wrap_fixture(
    forced: usize,
    filler: bool,
    line_capacity: i64,
    caps: &M4EffectiveResourceLimits,
) -> (Value, NodeId, NodeId, i64) {
    let (mut authored, paragraph, reference) = wrapping_data(forced, filler);
    let wide_root = Root::new();
    let wide = input(&wide_root, &authored, caps);
    let one = source_label_metrics(&wide, caps, paragraph, reference, 1);
    let ninety_nine = source_label_metrics(&wide, caps, paragraph, reference, 99);
    let hundred = source_label_metrics(&wide, caps, paragraph, reference, 100);
    assert_eq!((one.lines, ninety_nine.lines, hundred.lines), (1, 1, 1));
    let lower = one.required[0].max(hundred.advance);
    let upper = ninety_nine.required[0];
    assert!(
        upper > lower + 1,
        "no legal original-font interval: {one:?} {ninety_nine:?} {hundred:?}"
    );
    let width = lower + (upper - lower) / 2;
    assert!(
        one.required[0] <= width && hundred.advance <= width && width < ninety_nine.required[0]
    );
    assert_eq!(one.line_heights, vec![16 * 65536]);
    let frame = &mut authored["page_masters"]["masters"][0]["body"];
    frame["width"] = (2 * width + 10 * 65536).into();
    // Preserve one raw unit below the next line fit, allowing the zero-height
    // authored PageBreak to end the current nonempty page at a painted boundary.
    frame["height"] = (line_capacity * one.line_heights[0] + 1).into();
    eprintln!("Actual original-font wrap interval: forced={forced}; filler={filler}; capacity={line_capacity}; one={}; two={}; standalone100={}; selectedwidth={width}", one.required[0], ninety_nine.required[0], hundred.advance);
    (authored, paragraph, reference, width)
}

#[test]
fn book_v2_column_converged_pdf_one_to_ten_reflows_original_source_lines_at_measured_digit_width() {
    let caps = caps();
    let (authored, paragraph, reference, width) = calibrated_wrap_fixture(9, false, 2, &caps);
    let root = Root::new();
    let prepared = input(&root, &authored, &caps);
    let one = source_label_metrics(&prepared, &caps, paragraph, reference, 1);
    let ten = source_label_metrics(&prepared, &caps, paragraph, reference, 10);
    assert_eq!((one.lines, ten.lines), (1, 2));
    assert!(one
        .required
        .iter()
        .chain(&ten.required)
        .all(|required| *required <= width));
    let mut budget = BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
    let mut attempts = Vec::new();
    let observed = with_converged_column_pdf_observed(
        &prepared,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, observed| {
            assert_eq!(label_metrics(bundle.display(), paragraph, reference), ten);
            assert_eq!(
                bundle.display().source().source().geometry().pages().len(),
                10
            );
            observed
        },
        |attempt| attempts.push(Attempt::from_actual(attempt)),
    )
    .unwrap();
    assert_eq!(
        attempts.iter().map(|a| a.values[0].1).collect::<Vec<_>>(),
        vec![1, 10, 10]
    );
    assert!(attempts
        .iter()
        .all(|a| a.references == vec![(reference, a.values[0].1, Some(10), true)]));
    assert_eq!(
        (attempts[1].display, attempts[1].pdf),
        (attempts[2].display, attempts[2].pdf)
    );
    assert_eq!(attempts[1].bytes, attempts[2].bytes);
    assert_eq!(attempts[2].decision, ColumnLabelDecision::Ready);
    assert_eq!(observed, budget.observation());
    eprintln!(
        "Column actual1→10 line-reflow convergence: {:?}; width={width}; lines1={} lines10={}",
        quartet(observed),
        one.lines,
        ten.lines
    );
}

#[test]
fn book_v2_column_converged_pdf_ninety_nine_to_hundred_moves_the_actual_forward_target_after_digit_reflow(
) {
    let caps = caps();
    let (authored, paragraph, reference, width) = calibrated_wrap_fixture(98, true, 1, &caps);
    let root = Root::new();
    let prepared = input(&root, &authored, &caps);
    let one = source_label_metrics(&prepared, &caps, paragraph, reference, 1);
    let ninety_nine = source_label_metrics(&prepared, &caps, paragraph, reference, 99);
    let hundred = source_label_metrics(&prepared, &caps, paragraph, reference, 100);
    assert_eq!((one.lines, ninety_nine.lines, hundred.lines), (1, 2, 2));
    assert!(one
        .required
        .iter()
        .chain(&ninety_nine.required)
        .chain(&hundred.required)
        .all(|required| *required <= width));
    let mut budget = BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
    let mut attempts = Vec::new();
    let observed = with_converged_column_pdf_observed(
        &prepared,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, observed| {
            assert_eq!(
                label_metrics(bundle.display(), paragraph, reference),
                hundred
            );
            assert_eq!(
                bundle.display().source().source().geometry().pages().len(),
                100
            );
            observed
        },
        |attempt| attempts.push(Attempt::from_actual(attempt)),
    )
    .unwrap();
    assert_eq!(
        attempts.iter().map(|a| a.values[0].1).collect::<Vec<_>>(),
        vec![1, 99, 100, 100]
    );
    assert_eq!(
        attempts
            .iter()
            .map(|a| a.references[0].2)
            .collect::<Vec<_>>(),
        vec![Some(99), Some(100), Some(100), Some(100)]
    );
    assert_eq!(
        (attempts[2].display, attempts[2].pdf),
        (attempts[3].display, attempts[3].pdf)
    );
    assert_eq!(attempts[2].bytes, attempts[3].bytes);
    assert_eq!(
        attempts.iter().map(|a| a.decision).collect::<Vec<_>>(),
        vec![
            ColumnLabelDecision::Retry,
            ColumnLabelDecision::Retry,
            ColumnLabelDecision::Retry,
            ColumnLabelDecision::Ready
        ]
    );
    assert_eq!(observed.candidate_passes(), 4);
    assert_eq!(observed, budget.observation());
    eprintln!(
        "Column actual99→100 target shift: {:?}; width={width}; attempts={}",
        quartet(observed),
        attempts.len()
    );
}
// Actual outer-governor controls. These TrueType fixtures observe a real zero
// Type2 state; they make no claim about a positive CFF execution quota.
fn assert_outer_history_monotone(
    before: BookV2PdfConvergenceObservation,
    after: BookV2PdfConvergenceObservation,
) {
    assert!(after.record_charge() >= before.record_charge());
    assert!(after.spool_charge() >= before.spool_charge());
    assert!(after.work_steps() >= before.work_steps());
    assert!(after.candidate_passes() >= before.candidate_passes());
    assert!(after.balance_candidates() >= before.balance_candidates());
    assert!(after.width_feedback_passes() >= before.width_feedback_passes());
    assert!(after.width_refinement_passes() >= before.width_refinement_passes());
    assert!(after.line_reshape_passes() >= before.line_reshape_passes());
    assert!(after.page_passes() >= before.page_passes());
    // O is checked against the actual typed settlement below, not made monotone
    // by this helper: its contract permits signed replacement after real builds.
}
fn assert_outer_tt_statistics_zero(
    statistics: typaxis_resources::book_v2::BookV2CffProgramStatistics,
) {
    assert_eq!(statistics.operations_used, 0);
    assert_eq!(statistics.outline_segments_used, 0);
    assert_eq!(statistics.cached_glyphs, 0);
}

#[test]
fn book_v2_column_converged_pdf_exact_and_one_short_work_keep_actual_settlement_and_retry_history()
{
    let root = Root::new();
    let caps = caps();
    let (authored, _, _) = forward_data(9);
    let prepared = input(&root, &authored, &caps);
    let mut complete_budget = BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
    let mut complete_attempts = Vec::new();
    let (expected, delivered) = with_converged_column_pdf_observed(
        &prepared,
        &caps,
        MODE,
        &mut complete_budget,
        4,
        |bundle, observed| {
            (
                (
                    bundle.pdf().bytes().to_vec(),
                    bundle.display().fingerprint(),
                    bundle.pdf().fingerprint(),
                ),
                observed,
            )
        },
        |attempt| complete_attempts.push(Attempt::from_actual(attempt)),
    )
    .unwrap();
    let complete = complete_budget.observation();
    assert_eq!(delivered, complete);
    assert!(complete.work_steps() > 1 && !complete_attempts.is_empty());
    assert_outer_tt_statistics_zero(complete_budget.cff_program_statistics());

    let mut exact_budget = BookV2ColumnPdfConvergenceBudget::new(&caps, complete.work_steps());
    let mut exact_attempts = Vec::new();
    let (actual, delivered) = with_converged_column_pdf_observed(
        &prepared,
        &caps,
        MODE,
        &mut exact_budget,
        4,
        |bundle, observed| {
            (
                (
                    bundle.pdf().bytes().to_vec(),
                    bundle.display().fingerprint(),
                    bundle.pdf().fingerprint(),
                ),
                observed,
            )
        },
        |attempt| exact_attempts.push(Attempt::from_actual(attempt)),
    )
    .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(exact_attempts, complete_attempts);
    assert_eq!(delivered, complete);
    assert_eq!(exact_budget.observation(), complete);
    assert_outer_tt_statistics_zero(exact_budget.cff_program_statistics());

    let maximum = complete.work_steps() - 1;
    let mut short_budget = BookV2ColumnPdfConvergenceBudget::new(&caps, maximum);
    let statistics = short_budget.cff_program_statistics();
    let mut short_attempts = Vec::new();
    let mut called = false;
    let error = with_converged_column_pdf_observed(
        &prepared,
        &caps,
        MODE,
        &mut short_budget,
        4,
        |_, _| {
            called = true;
        },
        |attempt| short_attempts.push(Attempt::from_actual(attempt)),
    )
    .unwrap_err();
    assert!(!called);
    let failed = short_budget.observation();
    // These are actual complete-run prefix amounts, not a guessed error stage.
    assert_eq!(failed.record_charge(), complete.record_charge());
    assert_eq!(failed.spool_charge(), complete.spool_charge());
    assert_eq!(failed.output_charge(), complete.output_charge());
    assert_eq!(failed.work_steps(), maximum);
    assert_eq!(failed.candidate_passes(), complete.candidate_passes());
    assert_eq!(failed.line_reshape_passes(), complete.line_reshape_passes());
    assert_eq!(failed.page_passes(), complete.page_passes());
    assert!(short_attempts.len() < complete_attempts.len());
    assert_eq!(short_attempts, complete_attempts[..short_attempts.len()]);
    assert_eq!(short_budget.cff_program_statistics(), statistics);
    assert_outer_tt_statistics_zero(statistics);

    let mut called = false;
    let retry_error = with_budgeted_converged_book_v2_column_pdf(
        &prepared,
        &caps,
        MODE,
        &mut short_budget,
        4,
        |_, _| {
            called = true;
        },
    )
    .unwrap_err();
    assert!(!called);
    let retry = short_budget.observation();
    assert_outer_history_monotone(failed, retry);
    assert!(retry.work_steps() <= maximum);
    assert_eq!(retry.output_charge(), failed.output_charge());
    assert_eq!(short_budget.cff_program_statistics(), statistics);
    eprintln!("Column actual outer work-bound refusal/retry: {error:?}; {failed:?}; retry={retry_error:?}; {retry:?}; Type2={statistics:?}");
}

#[test]
fn book_v2_column_converged_pdf_rejects_foreign_admission_and_changed_limits_without_losing_history(
) {
    let root = Root::new();
    let foreign_root = Root::new();
    let caps = caps();
    let (authored, _, _) = forward_data(9);
    let prepared = input(&root, &authored, &caps);
    let foreign = input(&foreign_root, &authored, &caps);
    assert_eq!(
        prepared.body().styled().body().canonical_jcs_sha256(),
        foreign.body().styled().body().canonical_jcs_sha256()
    );
    assert!(!std::ptr::eq(prepared.resources(), foreign.resources()));
    let mut budget = BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
    let expected = with_budgeted_converged_book_v2_column_pdf(
        &prepared,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _| {
            (
                bundle.pdf().bytes().to_vec(),
                bundle.display().fingerprint(),
                bundle.pdf().fingerprint(),
            )
        },
    )
    .unwrap();
    let before = budget.observation();
    let statistics = budget.cff_program_statistics();
    assert_outer_tt_statistics_zero(statistics);
    let mut called = false;
    let foreign_error = with_budgeted_converged_book_v2_column_pdf(
        &foreign,
        &caps,
        MODE,
        &mut budget,
        4,
        |_, _| {
            called = true;
        },
    )
    .unwrap_err();
    assert!(!called);
    let rejected = budget.observation();
    assert_outer_history_monotone(before, rejected);
    assert_eq!(rejected.output_charge(), before.output_charge());
    assert_eq!(budget.cff_program_statistics(), statistics);

    let different = changed(&caps, |base| base.max_pages -= 1);
    let mut called = false;
    let limits_error = with_budgeted_converged_book_v2_column_pdf(
        &prepared,
        &different,
        MODE,
        &mut budget,
        4,
        |_, _| {
            called = true;
        },
    )
    .unwrap_err();
    assert!(!called);
    assert!(matches!(limits_error, E::Identity));
    assert_eq!(budget.observation(), rejected);
    assert_eq!(budget.cff_program_statistics(), statistics);

    // The original authentic owner still works with the very same carrier and
    // accepted history after the foreign-owner refusal; no new budget is made.
    let recovered = with_budgeted_converged_book_v2_column_pdf(
        &prepared,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _| {
            (
                bundle.pdf().bytes().to_vec(),
                bundle.display().fingerprint(),
                bundle.pdf().fingerprint(),
            )
        },
    )
    .unwrap();
    assert_eq!(recovered, expected);
    assert_outer_history_monotone(rejected, budget.observation());
    assert!(budget.observation().output_charge() >= rejected.output_charge());
    assert_eq!(budget.cff_program_statistics(), statistics);
    eprintln!("Column actual outer identity refusals: foreign={foreign_error:?}; limits={limits_error:?}; before={before:?}; rejected={rejected:?}; recovered={:?}; Type2={statistics:?}", budget.observation());
}
#[test]
fn book_v2_column_converged_pdf_retains_all_five_original_body_table_and_note_page_owners() {
    let root = Root::new();
    let caps = caps();
    let authored = data(true);
    let blocks = &authored["document"]["blocks"];
    let footnotes = &authored["document"]["footnotes"];
    let owner = |value: &Value| NodeId::new(value["node_id"].as_u64().unwrap() as u32);
    let mut owners = vec![
        owner(&blocks[1]["children"][0]),
        owner(&blocks[2]["head"][0]["cells"][0]["blocks"][0]["children"][0]),
        owner(&blocks[2]["body"][0]["cells"][0]["blocks"][0]["children"][0]),
        owner(&footnotes[0]["blocks"][0]["children"][0]),
        owner(&footnotes[1]["blocks"][0]["children"][0]),
    ];
    owners.sort_unstable();
    assert_eq!(owners.len(), 5);
    assert!(owners.windows(2).all(|pair| pair[0] < pair[1]));
    let unused = owner(&footnotes[1]["blocks"][0]["children"][0]);
    let prepared = input(&root, &authored, &caps);
    let mut budget = BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
    let mut attempts = Vec::new();
    let mut called = 0;
    let delivered = with_converged_column_pdf_observed(
        &prepared,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, observed| {
            called += 1;
            assert!(bundle.pdf().page_reference_labels_match());
            let references = bundle.pdf().page_references();
            assert_eq!(
                references.iter().map(|r| r.owner()).collect::<Vec<_>>(),
                owners
            );
            let unpainted = references.iter().find(|r| r.owner() == unused).unwrap();
            assert!(!unpainted.is_placed());
            // Preserve the real optional target, including None if genuinely
            // unplaced; candidate1 alone is never asserted as a destination.
            if let Some(target) = unpainted.target_page() {
                assert_eq!(unpainted.candidate_page(), target);
            }
            observed
        },
        |attempt| attempts.push(Attempt::from_actual(attempt)),
    )
    .unwrap();
    assert_eq!(called, 1);
    assert_eq!(delivered, budget.observation());
    assert!(attempts.len() >= 2);
    for attempt in &attempts {
        assert_eq!(
            attempt
                .values
                .iter()
                .map(|(owner, _)| *owner)
                .collect::<Vec<_>>(),
            owners
        );
        assert_eq!(
            attempt.references.iter().map(|r| r.0).collect::<Vec<_>>(),
            owners
        );
        assert_eq!(attempt.values.len(), 5);
        assert!(attempt
            .values
            .iter()
            .zip(&attempt.references)
            .all(|((owner, value), reference)| *owner == reference.0 && *value == reference.1));
        assert!(!attempt.references.iter().find(|r| r.0 == unused).unwrap().3);
    }
    let last = &attempts[attempts.len() - 2..];
    assert_eq!(last[0].display, last[1].display);
    assert_eq!(last[0].pdf, last[1].pdf);
    assert_eq!(last[0].bytes, last[1].bytes);
    assert_eq!(last[1].decision, ColumnLabelDecision::Ready);
    assert!(attempts[..attempts.len() - 1]
        .iter()
        .all(|a| a.decision == ColumnLabelDecision::Retry));
    eprintln!("Column actual original five-owner convergence: owners={owners:?}; unused={unused:?}; refs={:?}; attempts={}; after={:?}",
        last[1].references, attempts.len(), budget.observation());
}

#[path = "book_v2_column_attempt_observer_tests.rs"]
mod attempt_observer;

#[path = "book_v2_column_unplaced_page_target_tests.rs"]
mod unplaced_page_target;

// This scoped test-host bridge reuses the pinned admission harness and the exact
// original calibrated fixtures; it grants no candidate or builder authority.
pub(in crate::book_v2_resources) fn with_convergence_probe_fixture<R>(
    case: &str,
    inspect: impl FnOnce(
        &PreparedBookV2Resources,
        &M4EffectiveResourceLimits,
        u64,
        typaxis_linebreak::JapaneseLineBreakMode,
    ) -> R,
) -> R {
    let caps = caps();
    let authored = match case {
        "no-page" => data(false),
        "forward-ten" => forward_data(9).0,
        "wrap-ten" => calibrated_wrap_fixture(9, false, 2, &caps).0,
        "wrap-hundred" => calibrated_wrap_fixture(98, true, 1, &caps).0,
        "rich-five" => data(true),
        "unplaced-target" => {
            let mut authored = data(true);
            let mut unplaced_heading = authored["document"]["blocks"][0].clone();
            unplaced_heading["anchor_id"] = "unplaced-page-target".into();
            let blocks = authored["document"]["footnotes"][1]["blocks"]
                .as_array_mut()
                .unwrap();
            blocks[0]["children"][0]["target"] = "unplaced-page-target".into();
            blocks.insert(0, unplaced_heading);
            let mut next = 0;
            renumber(&mut authored["document"], &mut next);
            for master in authored["page_masters"]["masters"].as_array_mut().unwrap() {
                for role in ["header_content", "footer_content"] {
                    renumber(&mut master[role], &mut next);
                }
            }
            authored
        }
        _ => panic!("unknown original convergence fixture: {case}"),
    };
    let root = Root::new();
    let prepared = input(&root, &authored, &caps);
    inspect(&prepared, &caps, MAXIMUM, MODE)
}
