#[test]
fn production_body_terminals_bind_actual_block_placement_and_downstream_budgets() {
    let value = production_body_fixture(3_000_000);
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, semantics, profile, math| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let placement = selected.fingerprint();
            let before_records = selected.record_charge();
            assert!(selected.math_terminals().is_none());
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            let terminal = selected.math_terminals().unwrap();
            terminal.terminals().verify(math).unwrap();
            assert_eq!(terminal.placement_fingerprint(), placement);
            assert_ne!(selected.fingerprint(), placement);
            assert_eq!(selected.fingerprint(), terminal.fingerprint());
            assert_eq!(terminal.terminals().receipts().len(), 1);
            assert_eq!(
                terminal.terminals().receipts()[0].owner(),
                blocks.blocks()[0].owner()
            );
            assert_eq!(terminal.terminals().receipts()[0].terminal().get(), 1);
            assert_eq!(selected.record_charge(), before_records + 6);
            assert!(terminal.spool_charge() > 0);
            assert!(terminal.peak_spool_charge() >= terminal.spool_charge());
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
            assert!(fonts.spool_charge() >= terminal.spool_charge());
            let content =
                typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
            let structure = typaxis_display_list::build_production_body_structure(
                &display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .unwrap();
            let marked = typaxis_pdf::build_production_body_marked_content(
                &content, &structure, admitted, limits,
            )
            .unwrap();
            let objects =
                typaxis_pdf::build_production_body_objects(&marked, admitted, limits).unwrap();
            let pdf =
                typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
            assert!(pdf.bytes().starts_with(b"%PDF-"));
            drop(pdf);
            drop(objects);
            drop(marked);
            drop(structure);
            drop(content);
            drop(fonts);
            drop(display);
            assert_eq!(
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .err()
                    .unwrap()
                    .kind,
                typaxis_pagination::ProductionBodyPaginationErrorKind::ReceiptMismatch
            );
        },
    );
}

#[test]
fn production_body_terminals_preserve_forced_pages_without_double_consumption() {
    let mut value = production_body_fixture(3_000_000);
    let span = value["document"]["blocks"][0]["blocks"][1]["span"].clone();
    let zero = serde_json::json!({"source_id":0,"start_byte":span["start_byte"],"end_byte":span["start_byte"]});
    value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(
            1,
            serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":zero}),
        );
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, _, _, _, math| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            assert_eq!(selected.page_breaks().len(), 1);
            let block = selected
                .fragments()
                .iter()
                .find(|f| {
                    matches!(
                        f.source(),
                        typaxis_pagination::ProductionBodyFragmentSource::VectorBlock { .. }
                    )
                })
                .unwrap();
            assert!(block.page_index() >= 1);
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            assert_eq!(
                selected
                    .math_terminals()
                    .unwrap()
                    .terminals()
                    .receipts()
                    .len(),
                1
            );
        },
    );
    let empty: serde_json::Value =
        serde_json::from_slice(&production_text_single_paragraph(&["A"], "Body")).unwrap();
    with_production_body_math_resources(
        &empty,
        &config(),
        |lines, blocks, limits, _, _, _, math| {
            assert!(math.flows().is_empty());
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            assert!(selected
                .math_terminals()
                .unwrap()
                .terminals()
                .receipts()
                .is_empty());
        },
    );
}

#[test]
fn production_body_terminals_reject_another_preparation_registry() {
    let first = production_body_fixture(3_000_000);
    let other = production_body_fixture(3_100_000);
    with_production_body_math_resources(&first, &config(), |lines, blocks, limits, _, _, _, _| {
        let selected = typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
        with_production_body_math_resources(
            &other,
            &config(),
            |_, _, _, _, _, _, other_registry| {
                assert_eq!(
                    typaxis_pagination::finalize_production_body_math_terminals(
                        selected,
                        other_registry,
                        limits
                    )
                    .err()
                    .unwrap()
                    .kind,
                    typaxis_pagination::ProductionBodyPaginationErrorKind::ReceiptMismatch
                );
            },
        );
    });
}

#[test]
fn production_body_terminals_obey_exact_cumulative_record_and_spool_limits() {
    let value = production_body_fixture(3_000_000);
    let (mut records, mut spool) = (0, 0);
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, _, _, _, math| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            records = selected.record_charge();
            spool = selected.math_terminals().unwrap().peak_spool_charge();
        },
    );
    for (record_limit, spool_limit, expected) in [
        (records, spool, None),
        (
            records - 1,
            spool,
            Some(typaxis_pagination::ProductionBodyPaginationErrorKind::FragmentLimit),
        ),
        (
            records,
            spool - 1,
            Some(typaxis_pagination::ProductionBodyPaginationErrorKind::SpoolLimit),
        ),
    ] {
        let config = config_with_limits(ResourceLimits {
            max_fragments: record_limit,
            max_spool_bytes: spool_limit,
            ..ResourceLimits::default()
        });
        with_production_body_math_resources(
            &value,
            &config,
            |lines, blocks, limits, _, _, _, math| {
                let selected =
                    typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
                let result = typaxis_pagination::finalize_production_body_math_terminals(
                    selected, math, limits,
                );
                assert_eq!(result.err().map(|e| e.kind), expected);
            },
        );
    }
}

fn production_numbered_body_fixture(height: i64) -> serde_json::Value {
    production_numbered_body_text_fixture(height, "B")
}

fn production_numbered_body_text_fixture(height: i64, label: &str) -> serde_json::Value {
    let mut value = production_body_fixture(height);
    let end = value["sources"][0]["utf8_byte_length"].as_u64().unwrap();
    let span =
        serde_json::json!({"source_id":0,"start_byte":end,"end_byte":end+label.len() as u64});
    value["sources"][0]["utf8_byte_length"] = (end + label.len() as u64).into();
    let tex = value["text_buffers"][3]["utf8"].as_str().unwrap();
    value["sources"][0]["sha256"] = sha256(format!("{tex}{tex}{label}").as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    value["text_buffers"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "text_id":4,"utf8":label,"mappings":[{"kind":"identity","source_span":span,
            "text_range":{"start_byte":0,"end_byte":label.len()}}]
        }));
    value["document"]["blocks"][0]["span"]["end_byte"] = (end + label.len() as u64).into();
    value["document"]["blocks"][0]["blocks"][1]["span"]["end_byte"] =
        (end + label.len() as u64).into();
    let trailing = serde_json::json!({"source_id":0,"start_byte":end+label.len() as u64,"end_byte":end+label.len() as u64});
    value["document"]["blocks"][0]["blocks"][2]["span"] = trailing.clone();
    value["document"]["blocks"][0]["blocks"][2]["children"][0]["span"] = trailing;

    value["document"]["blocks"][0]["blocks"][1]["equation_number"] = serde_json::json!({
        "node_id":0,"span":span,
        "minimum_gap":65_536,"text_span":{"text_id":4,"start_byte":0,"end_byte":label.len()}
    });
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_body_equation_numbers_select_and_paint_the_complete_atom() {
    let value = production_numbered_body_fixture(3_000_000);
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, semantics, profile, math| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            assert_eq!(
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .err()
                    .unwrap()
                    .kind,
                typaxis_display_list::ProductionBodyDisplayErrorKind::PendingEquationNumber
            );
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            let number = &selected.equation_numbers()[0];
            let formula = &selected.fragments()[number.fragment_index() as usize];
            assert_eq!(number.page_index(), formula.page_index());
            assert_eq!(number.parent_owner(), formula.owner());
            assert_eq!(
                number
                    .bounds()
                    .x()
                    .checked_add(number.bounds().width().get()),
                formula
                    .bounds()
                    .x()
                    .checked_add(formula.bounds().width().get())
            );
            assert!(
                number.bounds().x()
                    >= formula
                        .viewport()
                        .unwrap()
                        .x()
                        .checked_add(formula.viewport().unwrap().width().get())
                        .unwrap()
                        .checked_add(
                            blocks.blocks()[0]
                                .equation_number()
                                .unwrap()
                                .minimum_gap()
                                .get()
                        )
                        .unwrap()
            );
            selected
                .math_terminals()
                .unwrap()
                .terminals()
                .verify(math)
                .unwrap();
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let number_draws = display
                .draws()
                .iter()
                .filter_map(|d| match d {
                    typaxis_display_list::ProductionBodyDraw::Text(t)
                        if t.equation_number().is_some() =>
                    {
                        Some(t)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                number_draws
                    .iter()
                    .map(|t| t.exact_text())
                    .collect::<String>(),
                "B"
            );
            assert_eq!(number_draws[0].owner(), number.owner());
            assert_eq!(number_draws[0].page_index(), number.page_index());
            assert_eq!(
                number_draws[0].font_face_id(),
                math.equation_number_shape(number.parent_owner())
                    .unwrap()
                    .font_face_id()
            );
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
            let content =
                typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
            let structure = typaxis_display_list::build_production_body_structure(
                &display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .unwrap();
            let node = structure
                .registry()
                .nodes()
                .iter()
                .find(|n| n.equation_number_binding_v2().is_some())
                .unwrap();
            assert_eq!(node.role(), typaxis_layout::StructureRole::Span);
            assert_eq!(node.actual_text(), None);
            assert_eq!(node.equation_number_binding_v2().unwrap().exact_text(), "B");
            assert_eq!(
                structure
                    .node_groups(node.structure_node_id())
                    .unwrap()
                    .len(),
                1
            );
            let marked = typaxis_pdf::build_production_body_marked_content(
                &content, &structure, admitted, limits,
            )
            .unwrap();
            let objects =
                typaxis_pdf::build_production_body_objects(&marked, admitted, limits).unwrap();
            let pdf =
                typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
            assert!(pdf.bytes().starts_with(b"%PDF-"));
        },
    );
}

#[test]
fn production_body_equation_numbers_recompute_container_gap_and_keep_forced_page() {
    let mut value = production_numbered_body_fixture(4_000_000);
    production_container_indent(&mut value, 131_072, 65_536);
    let start = value["document"]["blocks"][0]["blocks"][1]["span"]["start_byte"].clone();
    value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(
            1,
            serde_json::json!({"kind":"page_break","node_id":0,"classes":[],
        "span":{"source_id":0,"start_byte":start,"end_byte":start}}),
        );
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, _, _, math| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            let number = &selected.equation_numbers()[0];
            let fragment = selected.fragments()[number.fragment_index() as usize];
            assert_eq!(number.page_index(), 1);
            assert_eq!(
                fragment.bounds().width().get().raw(),
                4_000_000 - 131_072 - 65_536
            );
            assert_eq!(
                number.bounds().x().raw(),
                blocks.blocks()[0].equation_number().unwrap().left().raw() - 65_536
            );
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let text = display
                .draws()
                .iter()
                .find_map(|d| match d {
                    typaxis_display_list::ProductionBodyDraw::Text(t)
                        if t.equation_number().is_some() =>
                    {
                        Some(t)
                    }
                    _ => None,
                })
                .unwrap();
            assert_eq!(text.page_index(), 1);
            let shape = text.equation_number().unwrap();
            let font = typaxis_shaping::production_equation_number_font(shape, admitted).unwrap();
            let leading = shape
                .height()
                .get()
                .checked_sub(font.ascender())
                .unwrap()
                .checked_add(font.descender())
                .unwrap();
            let baseline = number.bounds().y().raw() + leading.raw() / 2 + font.ascender().raw();
            assert_eq!(
                text.glyphs()[0].y().raw(),
                baseline - shape.runs()[0].glyphs()[0].offset_y.raw()
            );
            assert_eq!(text.logical_bounds().unwrap(), number.bounds());
            assert_eq!(
                selected
                    .math_terminals()
                    .unwrap()
                    .terminals()
                    .receipts()
                    .len(),
                1
            );
        },
    );
    let mut collision = production_numbered_body_fixture(4_000_000);
    production_container_indent(&mut collision, 1_000_000, 1_000_000);
    collision["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    production_body_renumber(&mut collision["document"], &mut 0);
    with_production_body_math_resources(
        &collision,
        &config(),
        |lines, blocks, limits, _, _, _, _| {
            assert_eq!(
                typaxis_pagination::paginate_production_body(lines, blocks, limits)
                    .err()
                    .unwrap()
                    .kind,
                typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch
            );
        },
    );
}

#[test]
fn production_body_equation_numbers_share_cids_and_preserve_number_text() {
    let mut value = production_numbered_body_text_fixture(4_000_000, "ABAB");
    value["page_masters"]["masters"][0]["body"]["width"] = 8_000_000.into();
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, semantics, profile, math| {
            let selected =
                typaxis_pagination::paginate_production_body(lines, blocks, limits).unwrap();
            let selected =
                typaxis_pagination::finalize_production_body_math_terminals(selected, math, limits)
                    .unwrap();
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .unwrap();
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .unwrap();
            let number_cids = display
                .draws()
                .iter()
                .enumerate()
                .filter_map(|(i, d)| match d {
                    typaxis_display_list::ProductionBodyDraw::Text(t)
                        if t.equation_number().is_some() =>
                    {
                        Some(fonts.text_plan(i).unwrap().1.cids().to_vec())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(number_cids.len(), 4);
            assert_eq!(number_cids[0], number_cids[2]);
            assert_eq!(number_cids[1], number_cids[3]);
            assert_ne!(number_cids[0], number_cids[1]);
            let content =
                typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits).unwrap();
            let structure = typaxis_display_list::build_production_body_structure(
                &display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .unwrap();
            let node = structure
                .registry()
                .nodes()
                .iter()
                .find(|n| n.equation_number_binding_v2().is_some())
                .unwrap();
            let indices = structure.node_groups(node.structure_node_id()).unwrap();
            assert_eq!(indices.len(), 1);
            let group = &structure.groups()[indices[0]];
            let text = display.draws()[group.draws()]
                .iter()
                .map(|d| match d {
                    typaxis_display_list::ProductionBodyDraw::Text(t) => t.exact_text(),
                    _ => panic!(),
                })
                .collect::<String>();
            assert_eq!(text, "ABAB");
            let marked = typaxis_pdf::build_production_body_marked_content(
                &content, &structure, admitted, limits,
            )
            .unwrap();
            let objects =
                typaxis_pdf::build_production_body_objects(&marked, admitted, limits).unwrap();
            let pdf =
                typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits).unwrap();
            if let Some(root) = std::env::var_os("TYPAXIS_NUMBER_PDF_PROBE_DIR") {
                let root = PathBuf::from(root);
                fs::create_dir_all(&root).unwrap();
                fs::write(root.join("number.pdf"), pdf.bytes()).unwrap();
            }
        },
    );
}
