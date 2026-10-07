use super::*;
use crate::semantic_container::semantic_wire_ast_node_count;
use crate::{
    StagingSemanticDocumentPackageDecoder, StagingSemanticDocumentPackageEncoder,
    StrictDocumentPackageDecoder, WireStagingSemanticContainerKind,
};
use serde_json::{json, Value};
use typaxis_core::{sha256, DocumentPackageContractId, ResourceLimits, ValidatedResourceLimits};

#[path = "book_v2_number_binding_tests.rs"]
mod number_bindings;

#[path = "book_v2_table_caption_tests.rs"]
mod table_captions;

const FIXTURE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"
));
fn root() -> Value {
    let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
    root["contract"] = BOOK_V2_DOCUMENT_PACKAGE_CONTRACT.into();
    root
}
fn limits() -> ValidatedResourceLimits {
    ValidatedResourceLimits::new(ResourceLimits::default()).unwrap()
}
fn wire(root: &Value) -> Vec<u8> {
    serde_json::to_vec(root).unwrap()
}

#[test]
fn successor_kind_round_trips_without_relabeling_or_publication() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    for kind in WireBookV2SemanticContainerKind::ALL {
        let mut input = root();
        input["document"]["blocks"][0]["semantic_kind"] = kind.as_str().into();
        input["document"]["blocks"][0]["anchor_id"] = "authored.group".into();
        input["document"]["blocks"][0]["language"] = "ja".into();
        let bytes = wire(&input);
        let decoded = BookV2DocumentPackageDecoder::new()
            .decode(&bytes, &policy)
            .unwrap();
        let WireBookV2Block::SemanticContainer {
            semantic_kind,
            anchor_id,
            blocks,
            ..
        } = &decoded.wire().document().blocks[0]
        else {
            panic!("wrapper lost")
        };
        assert_eq!(*semantic_kind, kind);
        assert_eq!(anchor_id.as_deref(), Some("authored.group"));
        assert_eq!(blocks.len(), 3);
        assert_eq!(
            serde_json::to_value(decoded.wire().document()).unwrap(),
            input["document"]
        );
        assert_eq!(decoded.raw_sha256(), sha256(&bytes));
        assert_eq!(decoded.contract(), "typaxis.contract/1.5");
        let canonical = BookV2DocumentPackageEncoder::new()
            .encode(decoded.wire())
            .unwrap();
        assert_eq!(canonical, decoded.canonical_jcs());
        assert_eq!(decoded.canonical_jcs_sha256(), sha256(canonical.as_bytes()));
        assert_eq!(
            BookV2DocumentPackageDecoder::new()
                .decode(canonical.as_bytes(), &policy)
                .unwrap()
                .canonical_jcs(),
            canonical
        );
        assert!(StrictDocumentPackageDecoder::new()
            .decode(&bytes, &policy)
            .is_err());
        assert!(matches!(
            StagingSemanticDocumentPackageDecoder::new().decode(&bytes, &policy),
            Err(StagingSemanticDecodeError::Contract)
        ));
        input["contract"] = "typaxis.contract/1.4".into();
        let old_result =
            StagingSemanticDocumentPackageDecoder::new().decode(&wire(&input), &policy);
        assert_eq!(
            old_result.is_ok(),
            matches!(
                kind,
                WireBookV2SemanticContainerKind::Result
                    | WireBookV2SemanticContainerKind::Proof
                    | WireBookV2SemanticContainerKind::Exercise
            )
        );
    }
    assert_eq!(
        DocumentPackageContractId::CURRENT.as_str(),
        "typaxis.contract/1.4"
    );
    assert!("typaxis.contract/1.5"
        .parse::<DocumentPackageContractId>()
        .is_err());
}

#[test]
fn frozen_carrier_still_has_exact_old_canonical_bytes_and_closed_kind() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let old = StagingSemanticDocumentPackageDecoder::new()
        .decode(FIXTURE, &policy)
        .unwrap();
    let bytes = StagingSemanticDocumentPackageEncoder::new()
        .encode(old.wire())
        .unwrap();
    assert_eq!(old.canonical_jcs(), bytes);
    assert_eq!(old.contract(), "typaxis.contract/1.4");
    assert!(format!("{old:?}").starts_with("DecodedStagingSemanticDocumentPackage {"));
    assert_eq!(
        serde_json::from_str::<Value>(&bytes).unwrap(),
        serde_json::from_slice::<Value>(FIXTURE).unwrap()
    );
    for name in [
        "solution",
        "example",
        "admonition",
        "custom",
        "note",
        "quote",
        "",
    ] {
        assert!(serde_json::from_value::<WireStagingSemanticContainerKind>(name.into()).is_err());
    }
    let error = BookV2DocumentPackageDecoder::new()
        .decode(FIXTURE, &policy)
        .unwrap_err();
    assert_eq!(error.to_string(), "expected typaxis.contract/1.5");
}

// Carrier shape tests intentionally precede the separate dense-ID/span syntax
// admission. Every general recursive slot must retain the same typed child.
#[test]
fn successor_containers_survive_every_general_block_slot() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    for kind in WireBookV2SemanticContainerKind::ALL {
        for slot in [
            "body",
            "container",
            "list",
            "table-head",
            "table-body",
            "figure",
            "vector-figure",
            "footnote",
        ] {
            let mut input = root();
            let mut child = input["document"]["blocks"][0].clone();
            child["semantic_kind"] = kind.as_str().into();
            child["blocks"][1]["semantic_kind"] = "common_error".into();
            child["blocks"][2]["semantic_kind"] = "formalization_note".into();
            let span = child["span"].clone();
            let wrapped = match slot {
                "body" | "footnote" => child.clone(),
                "container" => {
                    json!({"kind":"semantic_container", "semantic_kind":"example", "anchor_id":null, "node_id":20,"span":span,"classes":[],"blocks":[child]})
                }
                "list" => {
                    json!({"kind":"list","ordered":false,"start":null,"node_id":20,"span":span,"classes":[],"items":[{"node_id":21,"span":span,"blocks":[child]}]})
                }
                "table-head" | "table-body" => {
                    let row = json!({"node_id":21,"span":span,"cells":[{"node_id":22,"span":span,"colspan":1,"rowspan":1,"blocks":[child]}]});
                    let mut table = json!({"kind":"table","node_id":20,"span":span,"classes":[],"columns":[{"kind":"fraction","weight":1}],"head":[],"body":[]});
                    table[if slot == "table-head" { "head" } else { "body" }] = json!([row]);
                    table
                }
                "figure" => {
                    json!({"kind":"figure","node_id":20,"span":span,"classes":[],"image_id":0,"placement":"block","alt":"diagram","caption":[child]})
                }
                "vector-figure" => {
                    json!({"kind":"vector_figure","node_id":20,"span":span,"classes":[],"image_id":0,"viewport":{"width":65536,"height":65536},"alt":"diagram","caption":[child]})
                }
                _ => unreachable!(),
            };
            if slot == "footnote" {
                input["document"]["blocks"] = json!([]);
                input["document"]["footnotes"] =
                    json!([{"footnote_id":"note.one","node_id":20,"span":span,"blocks":[wrapped]}]);
            } else {
                input["document"]["blocks"] = json!([wrapped]);
            }
            let decoded = BookV2DocumentPackageDecoder::new()
                .decode(&wire(&input), &policy)
                .unwrap_or_else(|e| panic!("{slot}: {e}"));
            assert_eq!(
                serde_json::to_value(decoded.wire().document()).unwrap(),
                input["document"],
                "{slot}"
            );
            let encoded = BookV2DocumentPackageEncoder::new()
                .encode(decoded.wire())
                .unwrap();
            assert_eq!(
                serde_json::from_str::<Value>(&encoded).unwrap(),
                input,
                "{slot}"
            );
        }
    }
}

#[test]
fn successor_input_is_closed_and_rejects_duplicate_fields() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    for case in [
        "unknown-kind",
        "null-kind",
        "missing-kind",
        "extra-child-field",
        "extra-root",
        "empty-container",
        "wrong-unit",
        "page-region-container",
    ] {
        let mut input = root();
        match case {
            "unknown-kind" => input["document"]["blocks"][0]["semantic_kind"] = "admonition".into(),
            "null-kind" => input["document"]["blocks"][0]["semantic_kind"] = Value::Null,
            "missing-kind" => {
                input["document"]["blocks"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("semantic_kind");
            }
            "extra-child-field" => {
                input["document"]["blocks"][0]["blocks"][1]["custom_role"] = "anything".into()
            }
            "extra-root" => input["extension"] = json!({}),
            "empty-container" => input["document"]["blocks"][0]["blocks"] = json!([]),
            "wrong-unit" => input["coordinate_unit"] = "pixel".into(),
            "page-region-container" => {
                input["page_masters"]["masters"][0]["header"] =
                    json!({"height":65536,"blocks":[input["document"]["blocks"][0].clone()]});
            }
            _ => unreachable!(),
        }
        assert!(
            BookV2DocumentPackageDecoder::new()
                .decode(&wire(&input), &policy)
                .is_err(),
            "{case}"
        );
    }
    let text = String::from_utf8(wire(&root())).unwrap();
    let duplicate = text.replacen(
        "\"semantic_kind\":\"result\"",
        "\"semantic_kind\":\"result\",\"semantic_kind\":\"note\"",
        1,
    );
    assert_ne!(duplicate, text);
    let error = BookV2DocumentPackageDecoder::new()
        .decode(duplicate.as_bytes(), &policy)
        .unwrap_err();
    assert!(
        matches!(error.kind(), StagingSemanticDecodeError::Preflight(error) if error.kind() == crate::JsonPreflightErrorKind::DuplicateObjectMember)
    );
}

#[test]
fn successor_encoder_rechecks_mutated_typed_regions() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&root()), &policy)
        .unwrap();
    let mut package = decoded.into_wire();
    let mut document = package.document().clone();
    let WireBookV2Block::SemanticContainer { blocks, .. } = &mut document.blocks[0] else {
        unreachable!()
    };
    blocks.clear();
    package.replace_typed_regions(document, package.resources().clone());
    assert!(BookV2DocumentPackageEncoder::new()
        .encode(&package)
        .is_err());
}

#[test]
fn successor_uses_same_inclusive_ast_and_resource_limits() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let input = wire(&root());
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(&input, &policy)
        .unwrap();
    let count =
        semantic_wire_ast_node_count(decoded.wire(), limits.get().max_ast_nesting_depth).unwrap();
    for delta in [0, 1] {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_ast_nodes: count - delta,
            ..ResourceLimits::default()
        })
        .unwrap();
        let result = BookV2DocumentPackageDecoder::new()
            .decode(&input, &DocumentPackageDecodePolicy::new(&limits));
        assert_eq!(result.is_ok(), delta == 0);
        if delta == 1 {
            assert!(matches!(
                result.unwrap_err().kind(),
                StagingSemanticDecodeError::Limit
            ));
        }
    }
    let mut bad = root();
    bad["resources"]["images"] = json!([null, null]);
    let limits = ValidatedResourceLimits::new(ResourceLimits {
        max_images: 1,
        ..ResourceLimits::default()
    })
    .unwrap();
    let error = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&bad), &DocumentPackageDecodePolicy::new(&limits))
        .unwrap_err();
    assert_eq!(error.pointer(), Some("/resources/images/1"));
}

#[test]
fn successor_preserves_unchanged_native_math_vector_and_media_carriers() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let cases: &[&[u8]] = &[
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/math/job/document-package.json")),
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/vector-media/job/document-package.json")),
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json")),
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/jpeg-media/job/document-package.json")),
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/cff-media/job/document-package.json")),
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/book-navigation/job/document-package.json")),
    ];
    for bytes in cases {
        let old = StagingSemanticDocumentPackageDecoder::new()
            .decode(bytes, &policy)
            .unwrap();
        let old_canonical = StagingSemanticDocumentPackageEncoder::new()
            .encode(old.wire())
            .unwrap();
        let mut input: Value = serde_json::from_str(&old_canonical).unwrap();
        input["contract"] = BOOK_V2_DOCUMENT_PACKAGE_CONTRACT.into();
        let decoded = BookV2DocumentPackageDecoder::new()
            .decode(&wire(&input), &policy)
            .unwrap();
        let canonical = BookV2DocumentPackageEncoder::new()
            .encode(decoded.wire())
            .unwrap();
        assert_eq!(serde_json::from_str::<Value>(&canonical).unwrap(), input);
        let mut retained: Value = serde_json::from_str(&canonical).unwrap();
        retained["contract"] = "typaxis.contract/1.4".into();
        let restored = StagingSemanticDocumentPackageDecoder::new()
            .decode(&wire(&retained), &policy)
            .unwrap();
        assert_eq!(restored.canonical_jcs(), old_canonical);
    }
}

#[path = "book_v2_description_tests.rs"]
mod descriptions;

#[path = "book_v2_table_cell_tests.rs"]
mod table_cells;
