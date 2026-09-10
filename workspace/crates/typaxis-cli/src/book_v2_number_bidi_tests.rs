use super::*;

#[test]
#[ignore = "requires explicit TYPAXIS_NUMBER_BIDI_FONT pointing to original Arial Unicode.ttf"]
fn book_v2_equation_number_draws_preserve_rtl_marks_and_signed_leading() {
    let root = Root::new();
    let limits = limits();
    let bytes = fs::read(std::env::var("TYPAXIS_NUMBER_BIDI_FONT").unwrap()).unwrap();
    let hash = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hash,
        "876af2cd4854644e7f3e7feb2f688997fdb3343c6df6693611209c9dfb47ccec"
    );
    let label = "(אב\u{05b0} 12 ג)";
    let source = format!(
        "{}{}",
        std::str::from_utf8(&VECTOR_SOURCE[..16]).unwrap(),
        label
    );
    let mut data = numbered_data();
    fn resize(v: &mut Value, end: usize) {
        match v {
            Value::Object(m) => {
                for (key, value) in m {
                    if key == "end_byte" && value.as_u64() == Some(19) {
                        *value = end.into();
                    } else {
                        resize(value, end);
                    }
                }
            }
            Value::Array(v) => {
                for value in v {
                    resize(value, end);
                }
            }
            _ => (),
        }
    }
    resize(&mut data, source.len());
    data["text_buffers"][0]["utf8"] = source.clone().into();
    data["resources"]["font_faces"][0]["expected_sha256"] = hash.into();
    let last = data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap();
    for declaration in last["declarations"].as_array_mut().unwrap() {
        if declaration["name"] == "line_height" {
            declaration["value"]["value"] = 500_000.into();
        }
    }
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 22_000_000.into();
    master["height"] = 24_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":22_000_000,"height":24_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":20_000_000,"height":20_000_000});
    let source_body = body_with_source(&root, data, source.as_bytes(), &limits);
    fs::write(root.0.join("body.bin"), &bytes).unwrap();
    fs::write(root.0.join("vector.svg"), VECTOR).unwrap();
    let input = prepare_book_v2_resources(
        source_body,
        &root.context(),
        &config(limits.base().get().clone()),
        &limits,
    )
    .unwrap();
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let raw = |n| Length::from_raw(n).unwrap();
    let body = typaxis_core::Rect::new(
        raw(500_000),
        raw(500_000),
        PositiveLength::new(raw(20_000_000)).unwrap(),
        PositiveLength::new(raw(20_000_000)).unwrap(),
    );
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body,
        1_000_000,
        |lines| {
            let shapes =
                shape_book_v2_equation_numbers(lines.lines().prepared().shaped(), &limits, 0)
                    .unwrap()
                    .unwrap();
            let shape = &shapes.shapes()[0];
            assert_eq!(shape.text(), label);
            assert!(shape.runs().iter().any(|r| r.bidi_level().get() % 2 == 1));
            assert!(shape.runs().iter().any(|r| r.bidi_level().get() % 2 == 0));
            assert!(shape
                .runs()
                .iter()
                .flat_map(|r| r.clusters())
                .any(|c| c.glyph_end - c.glyph_start > 1));
            let blocks = typaxis_layout::book_v2::prepare_book_v2_vector_blocks(
                lines.lines(),
                Some(&shapes),
                &limits,
                0,
            )
            .unwrap()
            .unwrap();
            let measured = typaxis_pagination::book_v2::prepare_book_v2_table_measurements(
                typaxis_pagination::book_v2::prepare_book_v2_body_flow(
                    lines.lines(),
                    Some(&blocks),
                    lines.footnotes(),
                    &limits,
                    0,
                )
                .unwrap(),
                &limits,
            )
            .unwrap();
            let mut search = typaxis_pagination::book_v2::prepare_book_v2_table_body_search(
                &measured, &limits, 1_000_000, 0,
            )
            .unwrap();
            let stable = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(stable.sequence()).unwrap();
            let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
            let terminals = search
                .finalize_mixed_page_math(closure, &limits, 0)
                .unwrap();
            assert_math_display(&terminals, input.resources(), &limits);
            let mut builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                &terminals,
                input.resources(),
                &limits,
                1_000_000,
                0,
                0,
            )
            .unwrap();
            let display = builder.build_equation_numbers().unwrap();
            assert_eq!(display.draws().len(), 1);
            let draw = &display.draws()[0];
            let font = draw.font();
            assert!(
                shape.height().get().raw() - font.ascender().raw() + font.descender().raw() < 0
            );
            assert_eq!(
                draw.clusters()
                    .iter()
                    .map(|c| c.exact_text())
                    .collect::<String>(),
                label
            );
            assert_eq!(draw.font().content_hash(), typaxis_core::sha256(&bytes));
            let body = builder.build_text().unwrap();
            assert!(body
                .draws()
                .iter()
                .all(|d| !d.exact_text().chars().any(|c| matches!(c, 'א' | 'ב' | 'ג'))));
        },
    )
    .unwrap();
}
