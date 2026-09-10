//! External-consumer checks run with the library's real feature set (not cfg(test)).
use serde_json::{json, Value};
use typaxis_document_package::{WireStagingM4Block, WireStagingSourceSpan};

const FIXTURE: &[u8] = include_bytes!(
    "../../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"
);

#[test]
fn frozen_wire_rejects_description_lists_independently_of_dependency_features() {
    let package: Value = serde_json::from_slice(FIXTURE).unwrap();
    let source = &package["document"]["blocks"][0];
    let paragraph = source["blocks"][0].clone();
    let mut term = paragraph.clone();
    term.as_object_mut().unwrap().remove("kind");
    let item = json!({"node_id":20,"span":source["span"],"term":term,"blocks":[paragraph]});
    for items in [json!([]), json!([item])] {
        let description = json!({"kind":"description_list","node_id":1,"span":source["span"],"classes":[],"items":items});
        let error = serde_json::from_value::<WireStagingM4Block>(description).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("description_list requires contract 1.5"),
            "{error}"
        );
    }
}

#[test]
fn a_shared_typed_carrier_cannot_serialize_a_description_into_the_frozen_contract() {
    let package: Value = serde_json::from_slice(FIXTURE).unwrap();
    let span: WireStagingSourceSpan =
        serde_json::from_value(package["document"]["blocks"][0]["span"].clone()).unwrap();
    let block = WireStagingM4Block::DescriptionList {
        node_id: 1,
        span,
        classes: Vec::new(),
        items: Vec::new(),
        language: None,
    };
    let error = serde_json::to_value(block).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("description_list requires contract 1.5"),
        "{error}"
    );
}
