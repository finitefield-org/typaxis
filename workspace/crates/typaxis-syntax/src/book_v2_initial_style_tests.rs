use super::*;

fn owners(value: &Value, output: &mut Vec<NodeId>) {
    if let Some(array) = value.as_array() {
        for value in array {
            owners(value, output)
        }
    }
    if let Some(object) = value.as_object() {
        if let Some(id) = object.get("node_id").and_then(Value::as_u64) {
            output.push(NodeId::new(id as u32))
        }
        for value in object.values() {
            owners(value, output)
        }
    }
}
fn fixture() -> Value {
    let mut value = root(FIXTURE);
    for selector in ["paragraph", "heading"] {
        let rules = value["style_sheet"]["rules"].as_array_mut().unwrap();
        let source_order = rules.len();
        rules.push(json!({"style_id":format!("initial-{selector}"),"source_order":source_order,"selector":selector,"extends":null,"declarations":[
            {"name":"font_family","important":false,"value":{"kind":"font_family_list","families":["Body"]}},
            {"name":"font_size","important":false,"value":{"kind":"length","value":786432}},
            {"name":"line_height","important":false,"value":{"kind":"length","value":1048576}}
        ]}));
    }
    value
}
fn same(input: &Value) -> Vec<[u64; 3]> {
    let legacy = style_book_v2_body(prepare(input).unwrap()).unwrap();
    let mut trace = Vec::new();
    let counted = style_book_v2_body_counted(prepare(input).unwrap(), &mut |r, s, w| {
        trace.push([r, s, w]);
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(
        format!("{:?}", legacy.containers),
        format!("{:?}", counted.containers)
    );
    assert_eq!(
        format!("{:?}", legacy.vectors),
        format!("{:?}", counted.vectors)
    );
    assert_eq!(format!("{:?}", legacy.math), format!("{:?}", counted.math));
    let mut ids = Vec::new();
    owners(input, &mut ids);
    for id in ids {
        assert_eq!(legacy.container_style(id), counted.container_style(id));
        assert_eq!(legacy.vector_style(id), counted.vector_style(id));
        assert_eq!(legacy.math_style(id), counted.math_style(id));
    }
    assert_eq!(
        legacy.body().canonical_jcs_sha256(),
        counted.body().canonical_jcs_sha256()
    );
    let first = prepare_book_v2_navigation(&legacy).unwrap();
    let second = prepare_book_v2_navigation(&counted).unwrap();
    match (
        prepare_book_v2_text_flow(&legacy, &first),
        prepare_book_v2_text_flow(&counted, &second),
    ) {
        (Ok(a), Ok(b)) => assert_eq!(a.fingerprint(), b.fingerprint()),
        (Err(a), Err(b)) => assert_eq!(a, b),
        _ => panic!("legacy/counted full source result differs"),
    }
    assert!(trace.iter().any(|v| v[1] > 0));
    assert!(trace.iter().any(|v| v[2] > 0));
    trace
}

#[test]
fn counted_initial_style_authentic_semantic_native_vector_values_and_source_fingerprints_match() {
    for fixture in [FIXTURE, MATH, VECTOR] {
        same(&root(fixture));
    }
    for kind in BookV2SemanticContainerKind::ALL {
        let mut value = fixture();
        value["document"]["blocks"][0]["semantic_kind"] = kind.as_str().into();
        same(&value);
    }
}

#[test]
fn counted_initial_style_retains_every_accepted_request_and_exact_axis_caps() {
    let input = fixture();
    let trace = same(&input);
    for fail_at in 0..trace.len() {
        let mut accepted = Vec::new();
        let mut index = 0;
        let error = style_book_v2_body_counted(prepare(&input).unwrap(), &mut |r, s, w| {
            let at = index;
            index += 1;
            if at == fail_at {
                return Err(at);
            }
            accepted.push([r, s, w]);
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error, BookV2InitialStyleError::Budget(fail_at));
        assert_eq!(accepted, trace[..fail_at]);
        assert_eq!(index, fail_at + 1);
    }
    let mut total = [0u64; 3];
    for values in &trace {
        for (i, v) in values.iter().enumerate() {
            total[i] += v
        }
    }
    for axis in 0..3 {
        for short in [false, true] {
            let mut seen = [7u64; 3];
            let mut caps = total.map(|v| v + 7);
            caps[axis] -= u64::from(short);
            let result = style_book_v2_body_counted(prepare(&input).unwrap(), &mut |r, s, w| {
                let request = [r, s, w];
                let next =
                    std::array::from_fn::<_, 3, _>(|i| seen[i].checked_add(request[i]).unwrap());
                if (0..3).any(|i| next[i] > caps[i]) {
                    return Err(axis);
                }
                seen = next;
                Ok(())
            });
            if short {
                assert!(matches!(result,Err(BookV2InitialStyleError::Budget(i)) if i==axis));
            } else {
                result.unwrap();
                assert_eq!(seen, caps)
            }
            for i in 0..3 {
                assert!(seen[i] <= caps[i]);
            }
        }
    }
    eprintln!(
        "counted initial style exact trace requests={} R/S/W={total:?}",
        trace.len()
    );
}

#[test]
fn counted_initial_style_actual_allocation_backend_failures_keep_paid_prefix() {
    let input = fixture();
    let trace = same(&input);
    let mut failures = 0;
    for at in 0..1000 {
        let body = prepare(&input).unwrap();
        let mut accepted = Vec::new();
        let result = super::super::super::initial_style::with_allocation_fault(at, || {
            style_book_v2_body_counted(body, &mut |r, s, w| {
                accepted.push([r, s, w]);
                Ok::<_, ()>(())
            })
        });
        match result {
            Ok(_) => {
                assert_eq!(accepted, trace);
                break;
            }
            Err(BookV2InitialStyleError::Allocation) => {
                failures += 1;
                assert!(!accepted.is_empty());
                assert_eq!(accepted, &trace[..accepted.len()]);
                assert!(accepted.last().unwrap()[1] > 0);
            }
            Err(error) => panic!("unexpected failure: {error:?}"),
        }
    }
    assert!(failures > 10);
    assert!(failures < 1000);
    eprintln!("counted initial syntax-owned allocation fault sites={failures}");
}

#[test]
fn counted_initial_style_preserves_first_semantic_errors_and_legacy_default_path() {
    let mut inputs = Vec::new();
    for selector in [
        "unknown.a",
        "paragraph.a.b.a",
        "paragraph.z.a",
        "table_cell.geometry",
    ] {
        let mut value = root(FIXTURE);
        value["style_sheet"]["rules"][0]["selector"] = selector.into();
        inputs.push(value);
    }
    let mut value = root(FIXTURE);
    value["style_sheet"]["rules"][1]["style_id"] =
        value["style_sheet"]["rules"][0]["style_id"].clone();
    inputs.push(value);
    let mut value = root(FIXTURE);
    value["style_sheet"]["rules"][0]["extends"] = "missing-parent".into();
    inputs.push(value);
    for input in inputs {
        let legacy = style_book_v2_body(prepare(&input).unwrap()).unwrap_err();
        let mut accepted = [0; 3];
        let counted = style_book_v2_body_counted(prepare(&input).unwrap(), &mut |r, s, w| {
            for (i, v) in [r, s, w].into_iter().enumerate() {
                accepted[i] += v
            }
            Ok::<_, ()>(())
        })
        .unwrap_err();
        assert_eq!(counted, BookV2InitialStyleError::Syntax(legacy));
        assert!(accepted[2] > 0);
    }
}

#[test]
fn counted_initial_style_registry_sorts_original_owner_keys_without_capacity_authority() {
    use super::super::super::initial_style::{Context, CountedRegistry, Output};
    let mut requests = Vec::new();
    let mut callback = |r, s, w| {
        requests.push([r, s, w]);
        Ok::<_, ()>(())
    };
    let mut context = Context::counted(&mut callback);
    let mut registry = CountedRegistry::new(3, &mut context).unwrap();
    for id in [8, 2, 5] {
        assert!(!registry.insert(NodeId::new(id), id, &mut context).unwrap());
    }
    assert!(registry.insert(NodeId::new(2), 999, &mut context).unwrap());
    let sorted = registry.finish(&mut context).unwrap();
    assert_eq!(
        sorted.iter().map(|(n, _)| n.get()).collect::<Vec<_>>(),
        [2, 5, 8]
    );
}
