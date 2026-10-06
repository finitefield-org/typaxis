//! Freeze complete public vector bindings and their mixed paragraph projections.
use super::*;

#[test]
fn book_v2_vector_fingerprints_preserve_frozen_tt_bindings() {
    check(None);
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_vector_fingerprints_preserve_frozen_original_harano_bindings() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        hex(typaxis_core::sha256(&bytes)),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    check(Some(&bytes));
}

fn hex(value: [u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn renumber(value: &mut Value, next: &mut u32) {
    if value.get("node_id").is_some() {
        value["node_id"] = (*next).into();
        *next += 1;
    }
    for key in ["blocks", "children", "caption"] {
        if let Some(children) = value.get_mut(key).and_then(Value::as_array_mut) {
            for child in children {
                renumber(child, next);
            }
        }
    }
    if let Some(number) = value.get_mut("equation_number") {
        renumber(number, next);
    }
}

fn check(font: Option<&[u8]>) {
    let name = if font.is_some() { "harano" } else { "tt" };
    let mut cases = serde_json::Map::new();
    for case in ["base", "provenance", "geometry", "occurrences"] {
        let root = Root::new();
        let limits = limits();
        let mut data = vector_data();
        match case {
            "provenance" => {
                data["resources"]["images"][0]["vector_provenance"]["engine_version"] =
                    "2026.09.1".into();
            }
            "geometry" => {
                for child in data["document"]["blocks"][0]["blocks"][0]["children"]
                    .as_array_mut()
                    .unwrap()
                {
                    if child.get("metrics").is_some() {
                        child["metrics"]["origin_x"] = (-32768).into();
                        child["metrics"]["advance"] = 2162688.into();
                        child["spacing"]["before"] = 8192.into();
                        child["spacing"]["after"] = 32768.into();
                    }
                }
            }
            "occurrences" => {
                let blocks = data["document"]["blocks"][0]["blocks"]
                    .as_array()
                    .unwrap()
                    .clone();
                data["document"]["blocks"][0]["blocks"] =
                    Value::Array((0..64).flat_map(|_| blocks.clone()).collect());
                renumber(&mut data["document"], &mut 0);
            }
            "base" => (),
            _ => unreachable!(),
        }
        let input = match font {
            Some(bytes) => {
                vector_input_with_font(&root, data.clone(), &limits, bytes, VECTOR_SOURCE)
            }
            None => vector_input(&root, data.clone(), &limits),
        };
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let binding = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        assert_eq!(
            binding.receipts().len(),
            if case == "occurrences" { 256 } else { 4 }
        );
        binding
            .verify(input.body(), input.resources(), &limits)
            .unwrap();
        let navigation = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &navigation).unwrap();
        let shape = shape_book_v2_authored_text(
            &policy,
            &flow,
            input.resources(),
            &limits,
            binding.epoch(),
            None,
        )
        .unwrap();
        let inline = prepare_book_v2_inline_items(
            &flow,
            &shape,
            input.resources(),
            &binding,
            &limits,
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();
        let another_root = Root::new();
        let another = match font {
            Some(bytes) => {
                vector_input_with_font(&another_root, data, &limits, bytes, VECTOR_SOURCE)
            }
            None => vector_input(&another_root, data, &limits),
        };
        let rejection = binding
            .verify(another.body(), input.resources(), &limits)
            .unwrap_err();
        assert_eq!(rejection, PrecomposedVectorBindingError::ReceiptMismatch);
        let value = json!({"epoch":hex(binding.epoch()),"binding":hex(binding.fingerprint()),
            "receipt_count":binding.receipts().len(),
            "receipts":binding.receipts().iter().map(|receipt| json!({"owner":receipt.node_id().get(),
                "kind":format!("{:?}",receipt.kind()),"fingerprint":hex(receipt.fingerprint()),
                "all_fields_sha256":hex(typaxis_core::sha256(format!("{receipt:?}").as_bytes()))})).collect::<Vec<_>>(),
            "shape":hex(shape.fingerprint()),"records":shape.output_records(),"inline":hex(inline.fingerprint()),
            "paragraphs":inline.paragraphs().iter().map(|paragraph|json!({"owner":paragraph.owner().get(),
                "items":paragraph.items().map(|items|json!({"fingerprint":hex(items.fingerprint()),
                    "units":items.units().len(),"clusters":items.clusters().len(),
                    "units_sha256":hex(typaxis_core::sha256(format!("{:?}",items.units()).as_bytes())),
                    "clusters_sha256":hex(typaxis_core::sha256(format!("{:?}",items.clusters()).as_bytes()))}))})).collect::<Vec<_>>(),
            "wrong_owner":format!("{rejection:?}")});
        cases.insert(case.into(), value);
    }
    let actual = Value::Object(cases);
    if let Ok(path) = std::env::var("TYPAXIS_BOOK_V2_VECTOR_FINGERPRINT_CAPTURE") {
        fs::create_dir_all(&path).unwrap();
        fs::write(
            std::path::Path::new(&path).join(format!("{name}.json")),
            serde_json::to_vec_pretty(&actual).unwrap(),
        )
        .unwrap();
    } else {
        let frozen: Value =
            serde_json::from_str(include_str!("book_v2_vector_fingerprint_golden.json")).unwrap();
        assert_eq!(
            frozen["production_commit"],
            "526f08cf7d44ebf8b7005b930456145eab18b5af"
        );
        let expected = frozen["cases"][name].as_object().unwrap();
        assert_eq!(
            actual.as_object().unwrap().keys().collect::<Vec<_>>(),
            expected.keys().collect::<Vec<_>>()
        );
        for (case, value) in actual.as_object().unwrap() {
            assert_eq!(
                hex(typaxis_core::sha256(&serde_json::to_vec(value).unwrap())),
                expected[case],
                "public binding/inline projection changed for {name}/{case}"
            );
        }
    }
}
