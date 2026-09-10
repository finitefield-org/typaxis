use super::*;
use typaxis_core::{ImageResourceId, Length, NodeId, PositiveLength};
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, layout_book_v2_inline_lines, prepare_book_v2_inline_items,
    BookV2VectorBindingError,
};
use typaxis_layout::{PrecomposedVectorBindingError, ProductionPlacedInline};
use typaxis_linebreak::JapaneseLineBreakMode;
use typaxis_syntax::PrecomposedVectorKind;
const VECTOR_SOURCE: &[u8] = b"Result x+y Proof(1)";
const VECTOR: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/x-plus-y.svg"));
fn vector_data() -> Value {
    let mut d = source_data(std::str::from_utf8(VECTOR_SOURCE).unwrap());
    d["text_buffers"][0]["mappings"] = Value::Array([ (0,7), (7,10), (10,16), (16,19) ].into_iter().map(|(start,end)|json!({"kind":"identity","source_span":{"source_id":0,"start_byte":start,"end_byte":end},"text_range":{"start_byte":start,"end_byte":end}})).collect::<Vec<_>>());
    let all = json!({"source_id":0,"start_byte":0,"end_byte":VECTOR_SOURCE.len()});
    let tex = json!({"source_id":0,"start_byte":7,"end_byte":10});
    let metrics = json!({"advance":2031616,"ascent":655360,"baseline":589824,"descent":196608,"origin_x":0,"viewport":{"height":786432,"width":1966080}});
    let inline = json!({"kind":"inline_vector","node_id":4,"image_id":0,"span":tex,"metrics":metrics,"spacing":{"before":16384,"after":16384},"alt":"diagram","actual_text":null});
    let mut math = inline.clone();
    math["kind"] = "math_vector".into();
    math["node_id"] = 5.into();
    math["alt"] = "x plus y".into();
    math["source_tex"] = json!({"text_span":{"text_id":0,"start_byte":7,"end_byte":10}});
    math["language"] = "ja".into();
    let paragraph = &mut d["document"]["blocks"][0]["blocks"][0];
    paragraph["children"] = json!([
        {"kind":"text","node_id":3,"span":{"source_id":0,"start_byte":0,"end_byte":7},"text_span":{"text_id":0,"start_byte":0,"end_byte":7}}, inline, math,
        {"kind":"text","node_id":6,"span":{"source_id":0,"start_byte":10,"end_byte":16},"text_span":{"text_id":0,"start_byte":10,"end_byte":16}}
    ]);
    let figure = json!({"kind":"vector_figure","node_id":7,"classes":[],"span":all,"image_id":0,"alt":"figure","caption":[],"viewport":{"height":786432,"width":1966080}});
    let block = json!({"kind":"math_vector_block","node_id":8,"classes":[],"span":all,"image_id":0,"alt":"x plus y numbered","actual_text":"x+y","source_tex":{"text_span":{"text_id":0,"start_byte":7,"end_byte":10}},"metrics":metrics,"equation_number":{"node_id":9,"span":{"source_id":0,"start_byte":16,"end_byte":19},"text_span":{"text_id":0,"start_byte":16,"end_byte":19},"minimum_gap":65536}});
    d["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .extend([figure, block]);
    d["resources"]["images"] = json!([{"image_id":0,"uri":"vector.svg","media_type":"svg-safe-2","expected_sha256":typaxis_core::sha256(VECTOR).iter().map(|b|format!("{b:02x}")).collect::<String>(),"vector_provenance":{"engine_id":"vmb.texToSvg","engine_version":"2026.09.0","rules_version":"vmb.math-safe-svg/1"}}]);
    d
}
pub(in crate::book_v2_resources::tests::shaping_tests) fn vector_input(
    root: &Root,
    data: Value,
    limits: &M4EffectiveResourceLimits,
) -> PreparedBookV2Resources {
    vector_input_bytes(root, data, limits, VECTOR)
}
pub(in crate::book_v2_resources::tests::shaping_tests) fn vector_input_with_font(
    root: &Root,
    mut data: Value,
    limits: &M4EffectiveResourceLimits,
    font: &[u8],
    source: &[u8],
) -> PreparedBookV2Resources {
    data["resources"]["font_faces"][0]["media_type"] = if font.starts_with(b"OTTO") {
        "sfnt-cff1"
    } else {
        "sfnt-truetype-glyf"
    }
    .into();
    data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    let body = body_with_source(root, data, source, limits);
    fs::write(root.0.join("body.bin"), font).unwrap();
    fs::write(root.0.join("vector.svg"), VECTOR).unwrap();
    prepare_book_v2_resources(
        body,
        &root.context(),
        &config(limits.base().get().clone()),
        limits,
    )
    .unwrap()
}
fn vector_input_bytes(
    root: &Root,
    data: Value,
    limits: &M4EffectiveResourceLimits,
    bytes: &[u8],
) -> PreparedBookV2Resources {
    let body = body_with_source(root, data, VECTOR_SOURCE, limits);
    fs::write(root.0.join("vector.svg"), bytes).unwrap();
    prepare_book_v2_resources(
        body,
        &root.context(),
        &config(limits.base().get().clone()),
        limits,
    )
    .unwrap()
}
#[test]
fn book_v2_vectors_bind_all_four_kinds_and_select_mixed_source_lines() {
    let root = Root::new();
    let limits = limits();
    let input = vector_input(&root, vector_data(), &limits);
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    assert_eq!(bindings.receipts().len(), 4);
    assert_eq!(
        bindings
            .receipts()
            .iter()
            .map(|r| r.kind())
            .collect::<Vec<_>>(),
        vec![
            PrecomposedVectorKind::InlineVector,
            PrecomposedVectorKind::MathVector,
            PrecomposedVectorKind::VectorFigure,
            PrecomposedVectorKind::MathVectorBlock
        ]
    );
    for (i, r) in bindings.receipts().iter().enumerate() {
        assert!(std::ptr::eq(
            r.source(),
            &input.body().styled().body().vectors()[i]
        ));
        assert_eq!(r.resource().image_id(), ImageResourceId::new(0));
        assert_eq!(r.resource().source_sha256(), typaxis_core::sha256(VECTOR));
        assert_eq!(r.resource().profile_fingerprint(), policy.fingerprint());
        assert_eq!(r.provenance().unwrap().engine_id, "vmb.texToSvg");
    }
    let math = bindings.receipt(NodeId::new(5)).unwrap();
    assert_eq!(math.source().alternative().alternative(), "x plus y");
    assert_eq!(
        math.source().alternative().resolved_actual_text(),
        Some("x plus y")
    );
    assert_eq!(
        math.source().source_tex().unwrap().exact_text_sha256(),
        typaxis_core::sha256(b"x+y")
    );
    assert_eq!(math.source().language().unwrap().canonical(), "ja");
    assert!(bindings
        .receipt(NodeId::new(8))
        .unwrap()
        .source()
        .equation_number()
        .is_some());
    assert_ne!(
        bindings.receipts()[0].fingerprint(),
        bindings.receipts()[1].fingerprint()
    );
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let shaped = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let prepared = prepare_book_v2_inline_items(
        &flow,
        &shaped,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    assert!(std::ptr::eq(prepared.vector_bindings().unwrap(), &bindings));
    let text_width = shaped.paragraphs()[0]
        .runs()
        .iter()
        .flat_map(|r| &r.glyph_run().glyphs)
        .map(|g| g.advance_x.raw())
        .sum::<i64>();
    let width =
        PositiveLength::new(Length::from_raw((text_width + 4_128_768) * 2 / 3).unwrap()).unwrap();
    let selected = layout_book_v2_inline_lines(&prepared, &[width], 1_000_000).unwrap();
    let body = typaxis_core::Rect::new(
        Length::ZERO,
        Length::ZERO,
        PositiveLength::new(
            width
                .get()
                .checked_add(Length::from_raw(9).unwrap())
                .unwrap(),
        )
        .unwrap(),
        width,
    );
    let framed =
        typaxis_layout::book_v2::layout_book_v2_body_inline_lines(&prepared, body, 1_000_000)
            .unwrap();
    let frames = framed.frames().unwrap();
    assert_eq!(frames.paragraphs()[0].width(), width);
    assert_eq!(
        selected.output_records() + frames.record_charge(),
        framed.output_records()
    );
    assert_eq!(selected.candidate_steps(), framed.candidate_steps());
    let selected = framed;
    assert!(selected.paragraphs()[0].lines().len() > 1);
    let mut text = String::new();
    let mut vectors = Vec::new();
    for line in selected.paragraphs()[0].lines() {
        for item in line.items() {
            match item {
                ProductionPlacedInline::Text(t) => text.push_str(t.utf8()),
                ProductionPlacedInline::Vector(v) => {
                    let actual = v.occurrence().item();
                    let binding = bindings.receipt(actual.node_id()).unwrap();
                    assert_eq!(actual.binding_fingerprint(), binding.binding_fingerprint());
                    vectors.push(actual.node_id());
                }
                _ => panic!("unexpected inline"),
            }
        }
    }
    assert_eq!(text, "Result  Proof");
    assert_eq!(vectors, vec![NodeId::new(4), NodeId::new(5)]);
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body,
        1_000_000,
        |stable| {
            assert!(stable.passes().last().unwrap().is_stable());
            assert!(stable.lines().prepared().native_math().is_none());
            let owners = stable.lines().paragraphs()[0]
                .lines()
                .iter()
                .flat_map(|l| l.items())
                .filter_map(|i| match i {
                    ProductionPlacedInline::Vector(v) => Some(v.occurrence().item().node_id()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(owners, vectors);
        },
    )
    .unwrap();
    let contexts = selected.selected_line_contexts().unwrap();
    assert_eq!(contexts.paragraphs()[0].ends().last().copied(), Some(19));
    let inputs = contexts
        .paragraphs()
        .iter()
        .map(|p| ProductionParagraphLineContext {
            owner: p.owner(),
            ends: p.ends(),
        })
        .collect::<Vec<_>>();
    let reshaped = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        Some(&inputs),
    )
    .unwrap();
    prepare_book_v2_inline_items(
        &flow,
        &reshaped,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    let r2 = Root::new();
    let other = vector_input(&r2, vector_data(), &limits);
    assert!(bindings
        .verify(other.body(), input.resources(), &limits)
        .is_err());
    assert!(bindings
        .verify(input.body(), other.resources(), &limits)
        .is_err());
    let wrong = prepare_book_v2_resource_policy(other.body(), &limits).unwrap();
    let wrong_bindings = bind_book_v2_vectors(&wrong, other.resources(), &limits).unwrap();
    assert!(prepare_book_v2_inline_items(
        &flow,
        &shaped,
        input.resources(),
        &wrong_bindings,
        &limits,
        JapaneseLineBreakMode::Normal
    )
    .is_err());
}
#[test]
fn book_v2_vector_geometry_rejects_nonuniform_source_viewport() {
    let root = Root::new();
    let limits = limits();
    let mut data = vector_data();
    data["document"]["blocks"][0]["blocks"][0]["children"][1]["metrics"]["viewport"]["width"] =
        2_097_152.into();
    let input = vector_input(&root, data, &limits);
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    assert!(matches!(
        bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap_err(),
        BookV2VectorBindingError::Binding(PrecomposedVectorBindingError::InvalidScale(owner)) if owner == NodeId::new(4)
    ));
}

#[test]
fn book_v2_vector_output_limit_is_cumulative_and_exact() {
    for (maximum, succeeds) in [(4, true), (3, false)] {
        let root = Root::new();
        let mut base = ResourceLimits::default();
        base.max_fragments = maximum;
        let limits = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(base).unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap();
        let input = vector_input(&root, vector_data(), &limits);
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let result = bind_book_v2_vectors(&policy, input.resources(), &limits);
        if succeeds {
            assert_eq!(result.unwrap().receipts().len(), 4);
        } else {
            assert!(matches!(
                result.unwrap_err(),
                BookV2VectorBindingError::OutputLimit
            ));
        }
    }
}
#[test]
fn book_v2_math_vectors_require_actual_safe_svg_two() {
    const SVG1:&[u8]=br##"<svg xmlns="http://www.w3.org/2000/svg" width="30pt" height="12pt" viewBox="0 0 30 12"><path d="M 0 0 L 30 0 L 30 12 Z" fill="#000000"/></svg>"##;
    let root = Root::new();
    let limits = limits();
    let mut data = vector_data();
    let declaration = &mut data["resources"]["images"][0];
    declaration["media_type"] = "svg-safe-1".into();
    declaration["expected_sha256"] = typaxis_core::sha256(SVG1)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    declaration
        .as_object_mut()
        .unwrap()
        .remove("vector_provenance");
    let input = vector_input_bytes(&root, data, &limits, SVG1);
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    assert!(
        matches!(bind_book_v2_vectors(&policy,input.resources(),&limits).unwrap_err(),
        BookV2VectorBindingError::Binding(PrecomposedVectorBindingError::ResourceMismatch(owner)) if owner == NodeId::new(5))
    );
}
#[path = "book_v2_equation_tests.rs"]
pub(in crate::book_v2_resources::tests::shaping_tests) mod equations;

#[test]
fn book_v2_source_widths_rebind_original_vector_and_math_vector_atoms() {
    for mode in ["original", "breaks", "empty"] {
        let root = Root::new();
        let limits = limits();
        let mut data = vector_data();
        if mode != "original" {
            let p = &mut data["document"]["blocks"][0]["blocks"][0];
            let span = p["span"].clone();
            let anchor_span = if mode == "empty" {
                span.clone()
            } else {
                json!({"source_id":0,"start_byte":10,"end_byte":16})
            };
            let anchor =
                json!({"kind":"anchor","node_id":0,"span":anchor_span,"anchor_id":"width-source-anchor"});
            if mode == "empty" {
                p["children"] = json!([anchor]);
            } else {
                let children = p["children"].as_array_mut().unwrap();
                children.insert(1, json!({"kind":"soft_break","node_id":0,"span":{"source_id":0,"start_byte":7,"end_byte":10}}));
                children.insert(4, json!({"kind":"hard_break","node_id":0,"span":{"source_id":0,"start_byte":10,"end_byte":16}}));
                children.insert(4, anchor);
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
            renumber(&mut data["document"], &mut 0);
        }
        let input = vector_input(&root, data, &limits);
        let width = |n| PositiveLength::new(Length::from_raw(n).unwrap()).unwrap();
        let body = typaxis_core::Rect::new(
            Length::ZERO,
            Length::ZERO,
            width(20_000_000),
            width(20_000_000),
        );
        super::source_widths::verify_mixed_source_rebinding(&input, body, &limits);
    }
}
