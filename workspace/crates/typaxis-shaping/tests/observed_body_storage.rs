//! Genuine host-admitted body and running-region controls for requested scratch.
#![cfg(feature = "book-v2-staging")]
use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use typaxis_core::*;
use typaxis_document_package::DocumentPackageDecodePolicy;
use typaxis_machine_input::{book_v2::HostBookV2InputSession, MachineInputHostOptions};
use typaxis_resource_admission::*;
use typaxis_shaping::{book_v2::*, ProductionParagraphLineContext};
use typaxis_syntax::book_v2::*;
const EPOCH: [u8; 32] = [19; 32];
const MAXIMUM: u64 = 100_000_000;
const PRIOR: [u64; 3] = [7, 11, 13];
const FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));
const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"
));
const PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/cover.bin"
));
const COLLECTION: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/collection.bin"));
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "typaxis-book-v2-resources-{}-{}",
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
fn config(limits: ResourceLimits) -> EffectiveConfig {
    config_with_extension(limits, M4ResourceLimits::default())
}
fn config_with_extension(limits: ResourceLimits, extension: M4ResourceLimits) -> EffectiveConfig {
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
        limits,
        extension,
    )
    .unwrap()
}
fn limits() -> M4EffectiveResourceLimits {
    M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap()
}
fn data() -> Value {
    let mut data: Value = serde_json::from_slice(FIXTURE).unwrap();
    data["contract"] = "typaxis.contract/1.5".into();
    data["document"]["blocks"][0]["semantic_kind"] = "solution".into();
    data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"paragraph-text","selector":"paragraph","source_order":2,"extends":null,"declarations":[
        {"name":"font_family","important":false,"value":{"kind":"font_family_list","families":["Body"]}},
        {"name":"font_size","important":false,"value":{"kind":"length","value":786432}},
        {"name":"line_height","important":false,"value":{"kind":"length","value":1048576}}
    ]}));
    data
}
fn body_with_source(
    root: &Root,
    mut data: Value,
    source: &[u8],
    limits: &M4EffectiveResourceLimits,
) -> SourceAdmittedBookV2Body {
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
        limits.base(),
    )
    .unwrap();
    let decoded = session
        .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(limits.base()))
        .unwrap();
    let sources = session.admit_sources(&decoded, limits.base()).unwrap();
    prepare_admitted_book_v2_body(
        session.finish(raw, decoded, sources).unwrap(),
        limits.base(),
    )
    .unwrap()
}
fn source_data(text: &str) -> Value {
    let mut data = data();
    let end = text.len();
    let span = json!({"source_id":0,"start_byte":0,"end_byte":end});
    data["text_buffers"] = json!([{"text_id":0,"utf8":text,"mappings":[{"kind":"identity","source_span":span,"text_range":{"start_byte":0,"end_byte":end}}]}]);
    data["document"]["blocks"] = json!([{"kind":"semantic_container","node_id":1,"classes":[],"semantic_kind":"solution","anchor_id":null,"span":span,"blocks":[
        {"kind":"paragraph","node_id":2,"classes":[],"span":span,"children":[{"kind":"text","node_id":3,"span":span,"text_span":{"text_id":0,"start_byte":0,"end_byte":end}}]}
    ]}]);
    data
}
fn region_data(text: &str) -> Value {
    let mut data = source_data(text);
    let span = data["document"]["blocks"][0]["span"].clone();
    let paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    let mut header = paragraph.clone();
    header["node_id"] = 5.into();
    header["children"][0]["node_id"] = 6.into();
    header["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"soft_break","node_id":7,"span":span}));
    let mut duplicate = header["children"][0].clone();
    duplicate["node_id"] = 8.into();
    header["children"].as_array_mut().unwrap().push(duplicate);
    header["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"hard_break","node_id":9,"span":span}));
    let mut footer = paragraph;
    footer["kind"] = "heading".into();
    footer["level"] = 2.into();
    footer["anchor_id"] = Value::Null;
    footer["node_id"] = 11.into();
    footer["children"][0]["node_id"] = 12.into();
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    let mut heading = rules
        .iter()
        .find(|r| r["selector"] == "paragraph")
        .unwrap()
        .clone();
    heading["style_id"] = "running-heading".into();
    heading["selector"] = "heading".into();
    heading["source_order"] = rules.len().into();
    rules.push(heading);
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = (400 * 65536).into();
    master["height"] = (600 * 65536).into();
    master["trim"] = json!({"x":0,"y":0,"width":400*65536,"height":600*65536});
    master["body"] = json!({"x":20*65536,"y":60*65536,"width":360*65536,"height":400*65536});

    master["header"] = json!({"x":10*65536,"y":0,"width":200*65536,"height":30*65536});
    master["footer"] = json!({"x":10*65536,"y":500*65536,"width":200*65536,"height":30*65536});
    master["header_content"] = json!({"node_id":4,"span":span,"blocks":[header]});
    master["footer_content"] = json!({"node_id":10,"span":span,"blocks":[footer]});
    data
}

struct Input {
    body: SourceAdmittedBookV2Body,
    resources: AdmittedProductionResourceLedgerV3,
}
fn input(root: &Root, data: Value, text: &str, limits: &M4EffectiveResourceLimits) -> Input {
    input_with_font(root, data, text, limits, None)
}
fn input_with_font(
    root: &Root,
    mut data: Value,
    text: &str,
    limits: &M4EffectiveResourceLimits,
    font: Option<&[u8]>,
) -> Input {
    if let Some(font) = font {
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = sha256(font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            .into();
    }
    let body = body_with_source(root, data, text.as_bytes(), limits);
    if let Some(font) = font {
        fs::write(root.0.join("body.bin"), font).unwrap();
    }
    let policy = prepare_book_v2_resource_policy(&body, limits).unwrap();
    let declared = staging_declared_base_catalog(body.styled().body().resources()).unwrap();
    let context = root.context();
    let config = config(limits.base().get().clone());
    let host = HostResourceAdmissionSession::new_with_read_ledger(
        &context,
        &config,
        declared.resource_catalog(),
        body.provenance().read_ledger(),
    )
    .unwrap();
    let mut resolver = StagingProductionResourceResolverV3::new(
        &declared,
        limits,
        policy.fingerprint(),
        host.roots(),
    )
    .unwrap();
    for font in &declared.resource_catalog().font_faces {
        let pending = resolver
            .read_font(host.open_font(font.font_face_id).unwrap())
            .unwrap();
        resolver.parse_and_bind_font(pending).unwrap();
    }
    for image in &declared.resource_catalog().images {
        let pending = resolver
            .read_image(host.open_image(image.image_id).unwrap())
            .unwrap();
        resolver.parse_and_bind_image(pending).unwrap();
    }
    let resources = resolver.finish().unwrap();
    Input { body, resources }
}
fn body_cases() -> Vec<(Value, &'static str)> {
    let mut labels = source_data("Result");
    let span = labels["document"]["blocks"][0]["span"].clone();
    let mut p = labels["document"]["blocks"][0]["blocks"][0].clone();
    p["node_id"] = 4.into();
    p["children"][0]["node_id"] = 5.into();
    p["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"footnote_reference","node_id":6,"span":span,"footnote_id":"note"}));
    labels["document"]["blocks"][0]["blocks"] = json!([{"kind":"list","node_id":2,"classes":[],"span":span,"ordered":true,"start":1,"items":[{"node_id":3,"span":span,"blocks":[p]}]}]);
    p["node_id"] = 8.into();
    p["children"].as_array_mut().unwrap().truncate(1);
    p["children"][0]["node_id"] = 9.into();
    labels["document"]["footnotes"] =
        json!([{"node_id":7,"span":span,"footnote_id":"note","blocks":[p]}]);
    let mut rule = labels["style_sheet"]["rules"][2].clone();
    rule["selector"] = "list".into();
    rule["style_id"] = "list-text".into();
    rule["source_order"] = 3.into();
    labels["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(rule);
    vec![
        (source_data("Result"), "Result"),
        (source_data("Result Result"), "Result Result"),
        (labels, "Result"),
    ]
}
#[test]
fn observed_body_storage_matches_receipts_and_independent_source_records_at_exact_limits() {
    for (data, text) in body_cases() {
        let root = Root::new();
        let limits = limits();
        let input = input(&root, data, text, &limits);
        let body = input.body.styled();
        let nav = prepare_book_v2_navigation(body).unwrap();
        let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(&input.body, &limits).unwrap();
        let boundaries = (3..text.len())
            .step_by(3)
            .chain(std::iter::once(text.len()))
            .map(|end| end as u32)
            .collect::<Vec<_>>();
        let ends = [ProductionParagraphLineContext {
            owner: flow.paragraphs()[0].owner(),
            ends: &boundaries,
        }];
        let line_cases = if flow.paragraphs().len() == 1 {
            vec![None, Some(ends.as_slice())]
        } else {
            vec![None]
        };
        for lines in line_cases {
            let old = shape_book_v2_authored_text(
                &policy,
                &flow,
                &input.resources,
                &limits,
                EPOCH,
                lines,
            )
            .unwrap();
            eprintln!(
                "receipt body text={text:?} lines={} glyph_records={} fp={}",
                lines.is_some(),
                old.output_records(),
                old.fingerprint()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            let mut source = BookV2ObservedSourceFlowObservation::default();
            flow.verify_for_observed_counted(
                body,
                &nav,
                PRIOR[0],
                PRIOR[1],
                PRIOR[2],
                limits.base().get().max_fragments,
                MAXIMUM,
                &mut source,
            )
            .unwrap();
            let expected_records = source.record_charge()
                + input.resources.fonts().len() as u64
                + old.output_records();
            let run = |start: [u64; 3], max_records, max_work| {
                let [mut r, mut s, mut w] = start;
                let mut report = 99;
                let result = with_book_v2_observed_source_accounting(
                    &mut r,
                    &mut s,
                    &mut w,
                    limits.base(),
                    max_records,
                    max_work,
                    |a| {
                        a.with_zero_intrinsic_records(|a, window| {
                            let result = shape_book_v2_authored_text_with_observed_source_counted(
                                &policy,
                                &flow,
                                &input.resources,
                                &limits,
                                EPOCH,
                                lines,
                                a,
                                window,
                                &mut report,
                            );
                            assert_eq!(window.paid_records(), report);
                            result.map(|shape| {
                                shape
                                    .verify(&flow, &input.resources, &limits, EPOCH)
                                    .unwrap();
                                assert_eq!(
                                    shape.font_instances().fingerprint(),
                                    old.font_instances().fingerprint()
                                );
                                assert_eq!(shape.output_records(), old.output_records());
                                assert_eq!(shape.list_markers().len(), old.list_markers().len());
                                assert_eq!(
                                    shape.footnote_markers().len(),
                                    old.footnote_markers().len()
                                );
                                shape.fingerprint()
                            })
                        })
                    },
                );
                (result, [r, s, w], report)
            };
            let full = run(PRIOR, limits.base().get().max_fragments, MAXIMUM);
            assert_eq!(full.0.unwrap(), old.fingerprint());
            assert_eq!(full.1[0], expected_records);
            assert!(full.1[1] > source.spool_charge() && full.1[2] > source.work_steps());
            assert_eq!(run(PRIOR, expected_records, full.1[2]), full);
            let exact_spool = limits.base().get().max_spool_bytes - (full.1[1] - PRIOR[1]);
            let exact = run(
                [PRIOR[0], exact_spool, PRIOR[2]],
                expected_records,
                full.1[2],
            );
            assert_eq!(exact.0.unwrap(), old.fingerprint());
            assert_eq!(exact.1[1], limits.base().get().max_spool_bytes);
            for axis in 0..3 {
                let short = match axis {
                    0 => run(PRIOR, expected_records - 1, MAXIMUM),
                    1 => run(
                        [PRIOR[0], exact_spool + 1, PRIOR[2]],
                        expected_records,
                        MAXIMUM,
                    ),
                    _ => run(PRIOR, expected_records, full.1[2] - 1),
                };
                assert!(short.0.is_err());
                assert!(short.1.iter().zip(PRIOR).all(|(a, b)| *a >= b));
                match (axis, short.0.unwrap_err()) {
                    (
                        0,
                        BookV2ObservedShapeError::Shape(
                            typaxis_shaping::ProductionTextShapeError {
                                kind: typaxis_shaping::ProductionTextShapeErrorKind::OutputLimit,
                                ..
                            },
                        ),
                    ) => (),
                    (
                        1,
                        BookV2ObservedShapeError::Accounting(
                            BookV2ObservedAccountingError::Spool { owner },
                        ),
                    )
                    | (
                        2,
                        BookV2ObservedShapeError::Accounting(BookV2ObservedAccountingError::Work {
                            owner,
                        }),
                    ) => assert_ne!(owner, NodeId::new(0)),
                    other => panic!("unexpected axis/error: {other:?}"),
                }
            }
            eprintln!(
                "body text={text:?} lines={} records={} source={:?} full={:?}",
                lines.is_some(),
                old.output_records(),
                [
                    source.record_charge(),
                    source.spool_charge(),
                    source.work_steps()
                ],
                full.1
            );
        }
    }
}
#[test]
fn observed_region_storage_shares_source_counters_and_preserves_exact_original_runs() {
    let root = Root::new();
    let limits = limits();
    let input = input(&root, region_data("Result"), "Result", &limits);
    let body = input.body.styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let selected = select_book_v2_page_master(body, 0, None, &mut 0, MAXIMUM).unwrap();
    let policy = prepare_book_v2_resource_policy(&input.body, &limits).unwrap();
    for kind in [BookV2PageRegionKind::Header, BookV2PageRegionKind::Footer] {
        let flow = prepare_book_v2_page_region_text_flow(selected, kind, &nav, 0).unwrap();
        let old =
            shape_book_v2_page_region_text(&policy, &flow, &input.resources, &limits, EPOCH, None)
                .unwrap();
        eprintln!(
            "receipt region kind={kind:?} fp={}",
            old.fingerprint()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let run = |start: [u64; 3], max_records, max_work| {
            let [mut r, mut s, mut w] = start;
            let mut report = 99;
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                limits.base(),
                max_records,
                max_work,
                |a| {
                    a.with_zero_intrinsic_records(|a, window| {
                        let result = shape_book_v2_page_region_text_with_observed_source_counted(
                            &policy,
                            &flow,
                            &input.resources,
                            &limits,
                            EPOCH,
                            None,
                            a,
                            window,
                            &mut report,
                        );
                        assert_eq!(window.paid_records(), report);
                        result.map(|shape| {
                            shape
                                .verify(&flow, &input.resources, &limits, EPOCH)
                                .unwrap();
                            assert_eq!(
                                shape.font_instances().fingerprint(),
                                old.font_instances().fingerprint()
                            );
                            for (old, new) in old.paragraphs().iter().zip(shape.paragraphs()) {
                                for (old, new) in old.runs().iter().zip(new.runs()) {
                                    assert_eq!(old.glyph_run(), new.glyph_run());
                                }
                            }
                            shape.fingerprint()
                        })
                    })
                },
            );
            (result, [r, s, w], report)
        };
        let full = run(PRIOR, limits.base().get().max_fragments, MAXIMUM);
        assert_eq!(full.0.unwrap(), old.fingerprint());
        assert_eq!(
            full.1[0],
            PRIOR[0] + input.resources.fonts().len() as u64 + old.output_records()
        );
        assert!(full.1[1] > PRIOR[1] && full.1[2] > PRIOR[2]);
        assert_eq!(run(PRIOR, full.1[0], full.1[2]), full);
        let exact_spool = limits.base().get().max_spool_bytes - (full.1[1] - PRIOR[1]);
        assert_eq!(
            run([PRIOR[0], exact_spool, PRIOR[2]], full.1[0], full.1[2])
                .0
                .unwrap(),
            old.fingerprint()
        );
        let short_records = run(PRIOR, full.1[0] - 1, MAXIMUM);
        assert!(matches!(
            short_records.0,
            Err(BookV2ObservedPageRegionShapeError::Shape(
                typaxis_shaping::ProductionTextShapeError {
                    kind: typaxis_shaping::ProductionTextShapeErrorKind::OutputLimit,
                    ..
                }
            ))
        ));
        for (axis, short) in [
            (
                1,
                run([PRIOR[0], exact_spool + 1, PRIOR[2]], full.1[0], MAXIMUM),
            ),
            (2, run(PRIOR, full.1[0], full.1[2] - 1)),
        ] {
            match (axis, short.0.unwrap_err()) {
                (
                    1,
                    BookV2ObservedPageRegionShapeError::Accounting(
                        BookV2ObservedAccountingError::Spool { owner },
                    ),
                )
                | (
                    2,
                    BookV2ObservedPageRegionShapeError::Accounting(
                        BookV2ObservedAccountingError::Work { owner },
                    ),
                ) => {
                    if axis == 2 {
                        assert_eq!(owner, flow.text_flow().paragraphs().last().unwrap().owner());
                    } else {
                        assert!(flow
                            .text_flow()
                            .paragraphs()
                            .iter()
                            .flat_map(|p| p.items())
                            .any(|s| s.owner() == owner));
                    }
                }
                other => panic!("unexpected region failure: {other:?}"),
            }
            assert!(short.1[1] <= limits.base().get().max_spool_bytes);
        }
        eprintln!("region {kind:?} full={:?}", full.1);
    }
}

#[test]
#[ignore = "requires the retained original-font list in TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_body_storage_original_harano_cff_body_and_running_regions() {
    let paths = fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let font = fs::read(paths.lines().next().unwrap()).unwrap();
    assert_eq!(
        sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let root = Root::new();
    let limits = limits();
    let input = input_with_font(
        &root,
        region_data("本文の柱"),
        "本文の柱",
        &limits,
        Some(&font),
    );
    let body = input.body.styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(&input.body, &limits).unwrap();
    let old = shape_book_v2_authored_text(&policy, &flow, &input.resources, &limits, EPOCH, None)
        .unwrap();
    let selected = select_book_v2_page_master(body, 0, None, &mut 0, MAXIMUM).unwrap();
    eprintln!(
        "receipt original-cff body fp={}",
        old.fingerprint()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let [mut r, mut s, mut w] = PRIOR;
    with_book_v2_observed_source_accounting(
        &mut r,
        &mut s,
        &mut w,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |a| {
            a.with_zero_intrinsic_records(|a, window| {
                let mut report = 0;
                let actual = shape_book_v2_authored_text_with_observed_source_counted(
                    &policy,
                    &flow,
                    &input.resources,
                    &limits,
                    EPOCH,
                    None,
                    a,
                    window,
                    &mut report,
                )
                .unwrap();
                assert_eq!(actual.fingerprint(), old.fingerprint());
                assert_eq!(
                    actual.font_instances().fingerprint(),
                    old.font_instances().fingerprint()
                );
                assert_eq!(window.paid_records(), old.output_records());
            });
            for kind in [BookV2PageRegionKind::Header, BookV2PageRegionKind::Footer] {
                let region =
                    prepare_book_v2_page_region_text_flow(selected, kind, &nav, 0).unwrap();
                let old = shape_book_v2_page_region_text(
                    &policy,
                    &region,
                    &input.resources,
                    &limits,
                    EPOCH,
                    None,
                )
                .unwrap();
                eprintln!(
                    "receipt original-cff region={kind:?} fp={}",
                    old.fingerprint()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                );
                let before = a.counters();
                a.with_zero_intrinsic_records(|a, window| {
                    let mut report = 0;
                    let actual = shape_book_v2_page_region_text_with_observed_source_counted(
                        &policy,
                        &region,
                        &input.resources,
                        &limits,
                        EPOCH,
                        None,
                        a,
                        window,
                        &mut report,
                    )
                    .unwrap();
                    assert_eq!(actual.fingerprint(), old.fingerprint());
                    assert_eq!(
                        actual.font_instances().fingerprint(),
                        old.font_instances().fingerprint()
                    );
                    assert_eq!(
                        a.counters()[0],
                        before[0] + input.resources.fonts().len() as u64 + old.output_records()
                    );
                    assert_eq!(window.paid_records(), report);
                });
                assert!(a.counters()[1] > before[1] && a.counters()[2] > before[2]);
            }
            eprintln!(
                "original Harano body/header/footer counters={:?}",
                a.counters()
            );
        },
    );
}

#[test]
fn observed_font_instance_constructor_refusals_keep_owner_and_atomic_prefix() {
    let root = Root::new();
    let limits = limits();
    let input = input(&root, region_data("Result"), "Result", &limits);
    let body = input.body.styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let selected = select_book_v2_page_master(body, 0, None, &mut 0, MAXIMUM).unwrap();
    let policy = prepare_book_v2_resource_policy(&input.body, &limits).unwrap();
    let flow =
        prepare_book_v2_page_region_text_flow(selected, BookV2PageRegionKind::Header, &nav, 0)
            .unwrap();
    let owner = NodeId::new(flow.source().node_id);
    let count = input.resources.fonts().len() as u64;
    let bytes = count * std::mem::size_of::<FontFaceId>() as u64;
    for axis in 0..3 {
        let mut start = PRIOR;
        if axis == 1 {
            start[1] = limits.base().get().max_spool_bytes - bytes + 1;
        }
        let [mut r, mut s, mut w] = start;
        let max_records = if axis == 0 {
            PRIOR[0] + count - 1
        } else {
            limits.base().get().max_fragments
        };
        let max_work = if axis == 2 {
            PRIOR[2] + count - 1
        } else {
            MAXIMUM
        };
        with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            limits.base(),
            max_records,
            max_work,
            |a| {
                a.with_zero_intrinsic_records(|a, window| {
                    let mut report = 99;
                    let actual = shape_book_v2_page_region_text_with_observed_source_counted(
                        &policy,
                        &flow,
                        &input.resources,
                        &limits,
                        EPOCH,
                        None,
                        a,
                        window,
                        &mut report,
                    )
                    .err()
                    .unwrap();
                    let expected = match axis {
                        0 => BookV2ObservedAccountingError::Records { owner },
                        1 => BookV2ObservedAccountingError::Spool { owner },
                        _ => BookV2ObservedAccountingError::Work { owner },
                    };
                    assert_eq!(
                        actual,
                        BookV2ObservedPageRegionShapeError::Accounting(expected)
                    );
                    assert_eq!(a.counters(), start);
                    assert_eq!(report, 0);
                    assert_eq!(window.paid_records(), 0);
                });
            },
        );
        assert_eq!([r, s, w], start);
    }
    let [mut r, mut s, mut w] = PRIOR;
    with_book_v2_observed_source_accounting(
        &mut r,
        &mut s,
        &mut w,
        limits.base(),
        limits.base().get().max_fragments,
        PRIOR[2] + 2 * count,
        |a| {
            a.with_zero_intrinsic_records(|a, window| {
                let mut report = 99;
                for _ in 0..2 {
                    let actual = shape_book_v2_page_region_text_with_observed_source_counted(
                        &policy,
                        &flow,
                        &input.resources,
                        &limits,
                        EPOCH,
                        None,
                        a,
                        window,
                        &mut report,
                    )
                    .err()
                    .unwrap();
                    assert_eq!(
                        actual,
                        BookV2ObservedPageRegionShapeError::Accounting(
                            BookV2ObservedAccountingError::Work { owner }
                        )
                    );
                    assert_eq!(
                        a.counters(),
                        [PRIOR[0] + count, PRIOR[1] + bytes, PRIOR[2] + 2 * count]
                    );
                    assert_eq!(report, 0);
                    assert_eq!(window.paid_records(), 0);
                }
                window.seal();
                let before = a.counters();
                let actual = shape_book_v2_page_region_text_with_observed_source_counted(
                    &policy,
                    &flow,
                    &input.resources,
                    &limits,
                    EPOCH,
                    None,
                    a,
                    window,
                    &mut report,
                )
                .err()
                .unwrap();
                assert_eq!(
                    actual,
                    BookV2ObservedPageRegionShapeError::Accounting(
                        BookV2ObservedAccountingError::ClosedWindow
                    )
                );
                assert_eq!(a.counters(), before);
            });
        },
    );
}

#[test]
#[ignore = "requires the retained original-font list in TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_original_cff_coverage_refuses_at_text_owner_before_glyph_output() {
    let paths = fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let font = fs::read(paths.lines().next().unwrap()).unwrap();
    assert_eq!(
        sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let root = Root::new();
    let limits = limits();
    let input = input_with_font(
        &root,
        source_data("本文の柱"),
        "本文の柱",
        &limits,
        Some(&font),
    );
    let body = input.body.styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(&input.body, &limits).unwrap();
    let owner = flow.paragraphs()[0].items()[0].owner();
    assert_eq!(owner, NodeId::new(3));
    let run = |maximum_work| {
        let [mut r, mut s, mut w] = PRIOR;
        let mut report = 99;
        let result = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            limits.base(),
            limits.base().get().max_fragments,
            maximum_work,
            |a| {
                a.with_zero_intrinsic_records(|a, window| {
                    let result = shape_book_v2_authored_text_with_observed_source_counted(
                        &policy,
                        &flow,
                        &input.resources,
                        &limits,
                        EPOCH,
                        None,
                        a,
                        window,
                        &mut report,
                    );
                    assert_eq!(window.paid_records(), report);
                    result.map(|s| s.fingerprint())
                })
            },
        );
        (result, [r, s, w], report)
    };
    let full = run(MAXIMUM);
    full.0.unwrap();
    type ShapeAttempt = (Result<[u8; 32], BookV2ObservedShapeError>, [u64; 3], u64);
    let reaches_text = |attempt: &ShapeAttempt| {
        if attempt.2 > 0 {
            return true;
        }
        match &attempt.0 {
            Ok(_) => true,
            Err(BookV2ObservedShapeError::Accounting(BookV2ObservedAccountingError::Work {
                owner: actual,
            })) => *actual == owner,
            _ => false,
        }
    };
    let mut lo = 0;
    let mut hi = full.1[2];
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if reaches_text(&run(mid)) {
            hi = mid
        } else {
            lo = mid + 1
        }
    }
    let first = run(lo);
    let next = run(lo + 1);
    assert_eq!(
        first.0,
        Err(BookV2ObservedShapeError::Accounting(
            BookV2ObservedAccountingError::Work { owner }
        ))
    );
    assert_eq!(next.0, first.0);
    assert_eq!(first.2, 0);
    assert_eq!(next.2, 0);
    assert_eq!(first.1[0], next.1[0]);
    assert_eq!(first.1[1], next.1[1]);
    assert_eq!(next.1[2], first.1[2] + 1);
    assert!(!reaches_text(&run(lo - 1)));
    eprintln!(
        "original CFF coverage first text-owner cutoff={lo}, accepted={:?}, next={:?}",
        first.1, next.1
    );
}

#[test]
fn observed_truetype_coverage_prepays_directory_before_scalar_and_keeps_text_owner() {
    let root = Root::new();
    let limits = limits();
    let input = input(&root, source_data("Result"), "Result", &limits);
    let body = input.body.styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(&input.body, &limits).unwrap();
    let owner = flow.paragraphs()[0].items()[0].owner();
    assert_eq!(owner, NodeId::new(3));
    let run = |maximum_work| {
        let [mut r, mut s, mut w] = PRIOR;
        let mut report = 99;
        let result = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            limits.base(),
            limits.base().get().max_fragments,
            maximum_work,
            |a| {
                a.with_zero_intrinsic_records(|a, window| {
                    let result = shape_book_v2_authored_text_with_observed_source_counted(
                        &policy,
                        &flow,
                        &input.resources,
                        &limits,
                        EPOCH,
                        None,
                        a,
                        window,
                        &mut report,
                    );
                    assert_eq!(window.paid_records(), report);
                    result.map(|s| s.fingerprint())
                })
            },
        );
        (result, [r, s, w], report)
    };
    let full = run(MAXIMUM);
    full.0.unwrap();
    type ShapeAttempt = (Result<[u8; 32], BookV2ObservedShapeError>, [u64; 3], u64);
    let reaches_text = |attempt: &ShapeAttempt| {
        if attempt.2 > 0 {
            return true;
        }
        match &attempt.0 {
            Ok(_) => true,
            Err(BookV2ObservedShapeError::Accounting(BookV2ObservedAccountingError::Work {
                owner: actual,
            })) => *actual == owner,
            _ => false,
        }
    };
    let mut lo = 0;
    let mut hi = full.1[2];
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if reaches_text(&run(mid)) {
            hi = mid
        } else {
            lo = mid + 1
        }
    }
    let first = run(lo);
    let refusal = Err(BookV2ObservedShapeError::Accounting(
        BookV2ObservedAccountingError::Work { owner },
    ));
    assert_eq!(first.0, refusal);
    assert_eq!(first.1[2], lo);
    assert!(!reaches_text(&run(lo - 1)));
    let setup = 3 * u64::from(u16::from_be_bytes(FONT[4..6].try_into().unwrap())) + 16;
    // The same borrowed setup is paid first for paragraph hhea metrics. One
    // paragraph-owned site-dispatch visit now separates it from text coverage.
    let metrics_begin = lo - setup - 2;
    for (cap, accepted) in [
        (metrics_begin, metrics_begin),
        (metrics_begin + 1, metrics_begin + 1),
        (lo - 2, metrics_begin + 1),
        (lo - 1, lo - 1),
    ] {
        let actual = run(cap);
        assert_eq!(
            actual.0,
            Err(BookV2ObservedShapeError::Accounting(
                BookV2ObservedAccountingError::Work {
                    owner: NodeId::new(2)
                }
            ))
        );
        assert_eq!(actual.1, [first.1[0], first.1[1], accepted]);
        assert_eq!(actual.2, 0);
    }
    // A refused directory allowance is atomic; no cmap/scalar work or glyph
    // output can run until its complete request has been accepted.
    for (cap, accepted) in [
        (lo, lo),
        (lo + 1, lo + 1),
        (lo + setup, lo + 1),
        (lo + setup + 1, lo + setup + 1),
        (lo + setup + 2, lo + setup + 2),
    ] {
        let actual = run(cap);
        assert_eq!(actual.0, refusal);
        assert_eq!(actual.1, [first.1[0], first.1[1], accepted]);
        assert_eq!(actual.2, 0);
    }
    eprintln!("TrueType coverage first text-owner cutoff={lo}, setup allowance={setup}, accepted={:?}, full={:?}", first.1, full.1);
}

#[test]
fn observed_truetype_coverage_preserves_ignorable_and_missing_scalar_results() {
    for (text, covered) in [
        ("Result\u{e0100}", true),
        ("Result\u{200d}", true),
        ("Result猫", false),
        ("Re\u{0301}sult", false),
    ] {
        let root = Root::new();
        let limits = limits();
        let input = input(&root, source_data(text), text, &limits);
        let body = input.body.styled();
        let nav = prepare_book_v2_navigation(body).unwrap();
        let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(&input.body, &limits).unwrap();
        let old =
            shape_book_v2_authored_text(&policy, &flow, &input.resources, &limits, EPOCH, None)
                .map(|s| s.fingerprint())
                .map_err(BookV2ObservedShapeError::Shape);
        assert_eq!(old.is_ok(), covered, "ordinary text={text:?}");
        if !covered {
            assert!(matches!(old, Err(BookV2ObservedShapeError::Shape(
                typaxis_shaping::ProductionTextShapeError {
                    owner, kind: typaxis_shaping::ProductionTextShapeErrorKind::MissingDeclaredFontCoverage,
                }
            )) if owner == NodeId::new(3)));
        }
        let [mut r, mut s, mut w] = PRIOR;
        let mut report = 99;
        let actual = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            limits.base(),
            limits.base().get().max_fragments,
            MAXIMUM,
            |a| {
                a.with_zero_intrinsic_records(|a, window| {
                    let result = shape_book_v2_authored_text_with_observed_source_counted(
                        &policy,
                        &flow,
                        &input.resources,
                        &limits,
                        EPOCH,
                        None,
                        a,
                        window,
                        &mut report,
                    );
                    assert_eq!(window.paid_records(), report);
                    result.map(|s| s.fingerprint())
                })
            },
        );
        assert_eq!(actual, old, "observed text={text:?}");
        if !covered {
            assert_eq!(report, 0);
        }
    }
}

#[test]
fn observed_shape_digest_final_line_binding_keeps_last_context_owner_and_paid_outputs() {
    let root = Root::new();
    let limits = limits();
    let mut data = source_data("Result");
    let mut second = data["document"]["blocks"][0]["blocks"][0].clone();
    second["node_id"] = 4.into();
    second["children"][0]["node_id"] = 5.into();
    data["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let input = input(&root, data, "Result", &limits);
    let body = input.body.styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(&input.body, &limits).unwrap();
    let ends = [6];
    let contexts = [
        ProductionParagraphLineContext {
            owner: NodeId::new(2),
            ends: &ends,
        },
        ProductionParagraphLineContext {
            owner: NodeId::new(4),
            ends: &ends,
        },
    ];
    let ordinary = shape_book_v2_authored_text(
        &policy,
        &flow,
        &input.resources,
        &limits,
        EPOCH,
        Some(&contexts),
    )
    .unwrap();
    let run = |maximum_work| {
        let [mut r, mut s, mut w] = PRIOR;
        let mut report = 99;
        let result = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            limits.base(),
            limits.base().get().max_fragments,
            maximum_work,
            |a| {
                a.with_zero_intrinsic_records(|a, window| {
                    let result = shape_book_v2_authored_text_with_observed_source_counted(
                        &policy,
                        &flow,
                        &input.resources,
                        &limits,
                        EPOCH,
                        Some(&contexts),
                        a,
                        window,
                        &mut report,
                    );
                    assert_eq!(window.paid_records(), report);
                    result.map(|s| s.fingerprint())
                })
            },
        );
        (result, [r, s, w], report)
    };
    let full = run(MAXIMUM);
    assert_eq!(full.0.unwrap(), ordinary.fingerprint());
    let short = run(full.1[2] - 1);
    assert_eq!(
        short.0,
        Err(BookV2ObservedShapeError::Accounting(
            BookV2ObservedAccountingError::Work {
                owner: NodeId::new(4)
            }
        ))
    );
    assert_eq!(short.1, [full.1[0], full.1[1], full.1[2] - 128]);
    assert_eq!(short.2, full.2);
    eprintln!(
        "final line-binding hash owner=4, full={:?}, refused-prefix={:?}, paid-output={}",
        full.1, short.1, short.2
    );
}

#[test]
fn observed_text_dispatch_invalid_line_owner_refuses_at_trusted_paragraph_before_error() {
    let root = Root::new();
    let limits = limits();
    let input = input(&root, source_data("Result"), "Result", &limits);
    let body = input.body.styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(&input.body, &limits).unwrap();
    let ends = [6];
    let contexts = [ProductionParagraphLineContext {
        owner: NodeId::new(999),
        ends: &ends,
    }];
    let ordinary = shape_book_v2_authored_text(
        &policy,
        &flow,
        &input.resources,
        &limits,
        EPOCH,
        Some(&contexts),
    )
    .err()
    .unwrap();
    assert_eq!(
        ordinary,
        typaxis_shaping::ProductionTextShapeError {
            owner: NodeId::new(2),
            kind: typaxis_shaping::ProductionTextShapeErrorKind::InvalidLineContext
        }
    );
    let run = |maximum_work| {
        let [mut r, mut s, mut w] = PRIOR;
        let mut report = 99;
        let result = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            limits.base(),
            limits.base().get().max_fragments,
            maximum_work,
            |a| {
                a.with_zero_intrinsic_records(|a, window| {
                    let result = shape_book_v2_authored_text_with_observed_source_counted(
                        &policy,
                        &flow,
                        &input.resources,
                        &limits,
                        EPOCH,
                        Some(&contexts),
                        a,
                        window,
                        &mut report,
                    );
                    assert_eq!(window.paid_records(), 0);
                    result.map(|s| s.fingerprint())
                })
            },
        );
        (result, [r, s, w], report)
    };
    let full = run(MAXIMUM);
    assert_eq!(full.0, Err(BookV2ObservedShapeError::Shape(ordinary)));
    assert_eq!(full.2, 0);
    let short = run(full.1[2] - 1);
    assert_eq!(
        short.0,
        Err(BookV2ObservedShapeError::Accounting(
            BookV2ObservedAccountingError::Work {
                owner: NodeId::new(2)
            }
        ))
    );
    assert_eq!(short.1, [full.1[0], full.1[1], full.1[2] - 1]);
    assert_eq!(short.2, 0);
    eprintln!("invalid line-context claimed owner=999, work owner=2, full={:?}, refused-prefix={:?}, paid-output=0",full.1,short.1);
}

#[test]
fn observed_font_selection_missing_family_keeps_atomic_allowance_and_paragraph_owner() {
    let root = Root::new();
    let limits = limits();
    let mut data = source_data("Result");
    let missing = "未登録".repeat(64);
    let rule = data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["style_id"] == "paragraph-text")
        .unwrap();
    rule["declarations"][0]["value"]["families"] = json!([missing.clone()]);
    let input = input(&root, data, "Result", &limits);
    let body = input.body.styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(&input.body, &limits).unwrap();
    let ordinary =
        shape_book_v2_authored_text(&policy, &flow, &input.resources, &limits, EPOCH, None)
            .err()
            .unwrap();
    assert_eq!(
        ordinary,
        typaxis_shaping::ProductionTextShapeError {
            owner: NodeId::new(2),
            kind: typaxis_shaping::ProductionTextShapeErrorKind::MissingSelectedFont
        }
    );
    let mut fees = Vec::new();
    let lookup = input
        .resources
        .font_families()
        .try_resolve_with_work(&[missing], &mut |n| {
            fees.push(n);
            Ok::<(), u8>(())
        });
    assert_eq!(
        lookup,
        Err(typaxis_font::FontFamilyWorkError::Family(
            typaxis_font::FontFamilyError::UnknownFamily
        ))
    );
    assert_eq!(fees.len(), 2);
    assert_eq!(fees[0], 1);
    let allowance = fees[1];
    let run = |maximum_work| {
        let [mut r, mut s, mut w] = PRIOR;
        let mut report = 99;
        let result = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            limits.base(),
            limits.base().get().max_fragments,
            maximum_work,
            |a| {
                a.with_zero_intrinsic_records(|a, window| {
                    let result = shape_book_v2_authored_text_with_observed_source_counted(
                        &policy,
                        &flow,
                        &input.resources,
                        &limits,
                        EPOCH,
                        None,
                        a,
                        window,
                        &mut report,
                    );
                    assert_eq!(window.paid_records(), 0);
                    result.map(|s| s.fingerprint())
                })
            },
        );
        (result, [r, s, w], report)
    };
    let full = run(MAXIMUM);
    assert_eq!(full.0, Err(BookV2ObservedShapeError::Shape(ordinary)));
    assert_eq!(full.2, 0);
    for (cap, accepted) in [
        (full.1[2] - allowance - 1, full.1[2] - allowance - 1),
        (full.1[2] - 1, full.1[2] - allowance),
    ] {
        let short = run(cap);
        assert_eq!(
            short.0,
            Err(BookV2ObservedShapeError::Accounting(
                BookV2ObservedAccountingError::Work {
                    owner: NodeId::new(2)
                }
            ))
        );
        assert_eq!(short.1, [full.1[0], full.1[1], accepted]);
        assert_eq!(short.2, 0);
    }
    eprintln!("missing family selection owner=2, query allowance={allowance}, full={:?}, one-short accepted={:?}, paid-output=0",full.1,[full.1[0],full.1[1],full.1[2]-allowance]);
}

#[test]
fn observed_generated_lookup_real_flow_preserves_source_owner_and_work_prefix() {
    let root = Root::new();
    let limits = limits();
    let (data, text) = body_cases().pop().unwrap();
    let input = input(&root, data, text, &limits);
    let body = input.body.styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let flow = prepare_book_v2_text_flow(body, &nav).unwrap();
    let paragraph = flow
        .paragraphs()
        .iter()
        .find(|p| p.items().iter().any(|i| i.owner() == NodeId::new(6)))
        .unwrap()
        .owner();
    assert_eq!(paragraph, NodeId::new(4));
    let mut cases = 0;
    let mut caps = 0;
    for (operation, owner) in [
        (0, paragraph),
        (1, NodeId::new(7)),
        (2, NodeId::new(3)),
        (3, paragraph),
    ] {
        let expected = match operation {
            0 => flow
                .footnote_marker_provenance(NodeId::new(6))
                .zip(flow.footnote_marker_text(NodeId::new(6))),
            1 => flow
                .footnote_marker_provenance(NodeId::new(7))
                .zip(flow.footnote_marker_text(NodeId::new(7))),
            2 => flow.list_marker_provenance(0).zip(flow.list_marker_text(0)),
            _ => flow
                .reference_provenance(NodeId::new(u32::MAX))
                .zip(flow.reference_text(NodeId::new(u32::MAX))),
        };
        let apply = |work: &mut dyn FnMut(u64) -> Result<(), BookV2ObservedAccountingError>| {
            match operation {
                0 => flow.footnote_marker_with_work(NodeId::new(6), &mut |n| work(n)),
                1 => flow.footnote_marker_with_work(NodeId::new(7), &mut |n| work(n)),
                2 => flow.list_marker_with_work(0, &mut |n| work(n)),
                _ => flow.reference_with_work(NodeId::new(u32::MAX), &mut |n| work(n)),
            }
        };
        let mut amount = 0;
        assert_eq!(
            apply(&mut |n| {
                assert_eq!(n, 1);
                amount += n;
                Ok(())
            })
            .unwrap(),
            expected
        );
        let full = PRIOR[2] + amount;
        for cap in PRIOR[2]..=full + 1 {
            let [mut r, mut s, mut w] = PRIOR;
            let actual = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                limits.base(),
                limits.base().get().max_fragments,
                cap,
                |a| apply(&mut |n| a.debit_work(owner, n)),
            );
            let wanted = if cap < full {
                Err(BookV2ObservedAccountingError::Work { owner })
            } else {
                Ok(expected)
            };
            assert_eq!(actual, wanted);
            assert_eq!([r, s, w], [PRIOR[0], PRIOR[1], cap.min(full)]);
            caps += 1;
        }
        cases += 1;
    }
    eprintln!("generated lookup Source cases={cases}, work caps={caps}, owners=4/7/3, records/spool unchanged");
}

#[path = "support/observed_cff_base_cmap_admission.rs"]
mod cff_base_cmap_admission;

#[path = "support/observed_sfnt_preflight.rs"]
mod sfnt_preflight;

#[path = "support/observed_cff_metadata.rs"]
mod cff_metadata;

#[path = "support/observed_cff_tables.rs"]
mod cff_tables;

#[path = "support/observed_cff_program.rs"]
mod cff_program;

#[path = "support/observed_cff_local_optional.rs"]
mod cff_local_optional;

#[path = "support/observed_cff_math.rs"]
mod cff_math;

#[path = "support/observed_cff_jstf.rs"]
mod cff_jstf;
