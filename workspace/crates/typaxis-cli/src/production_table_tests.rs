fn production_table_fixture() -> serde_json::Value {
    use serde_json::json;
    let mut value = production_body_fixture(12_000_000);
    value["page_masters"]["masters"][0]["body"]["width"] = 8_000_000.into();
    let parts = value["document"]["blocks"][0]["blocks"]
        .as_array()
        .unwrap()
        .clone();
    let span = value["document"]["blocks"][0]["span"].clone();
    let end = parts[2]["span"].clone();
    let point = json!({"source_id":0,"start_byte":0,"end_byte":0});
    let mut header = parts[2].clone();
    header["span"] = point.clone();
    header["children"][0]["span"] = point.clone();
    value["document"]["blocks"][0]["blocks"] = json!([{
        "kind":"table","node_id":0,"span":span,"classes":["common-grid"],
        "columns":[{"kind":"fixed","width":3_000_000},{"kind":"fraction","weight":3}],
        "head":[{"node_id":0,"span":point,"cells":[
            {"node_id":0,"span":point,"colspan":1,"rowspan":1,"blocks":[header]},
            {"node_id":0,"span":point,"colspan":1,"rowspan":1,"blocks":[header]}
        ]}],
        "body":[{"node_id":0,"span":span,"cells":[
            {"node_id":0,"span":parts[0]["span"],"colspan":1,"rowspan":2,"blocks":[parts[0]]},
            {"node_id":0,"span":parts[1]["span"],"colspan":1,"rowspan":1,"blocks":[parts[1]]}
        ]},{"node_id":0,"span":end,"cells":[
            {"node_id":0,"span":end,"colspan":1,"rowspan":1,"blocks":[parts[2]]}
        ]},{"node_id":0,"span":end,"cells":[
            {"node_id":0,"span":end,"colspan":2,"rowspan":1,"blocks":[parts[2]]}
        ]}]
    }]);
    let rules = value["style_sheet"]["rules"].as_array_mut().unwrap();
    let order = rules.len();
    rules.push(json!({"style_id":"common-grid","selector":"table.common-grid","extends":null,"source_order":order,"declarations":[
        {"important":false,"name":"start_indent","value":{"kind":"length","value":131072}},
        {"important":false,"name":"end_indent","value":{"kind":"length","value":196608}}
    ]}));
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_table_cells_shape_body_and_math_in_resolved_column_frames() {
    let value = production_table_fixture();
    with_production_body_inputs(&value, &config(), |lines, blocks, _| {
        let frames = lines.frames().unwrap();
        assert_eq!(frames.tables().len(), 1);
        let table = &frames.tables()[0];
        let source = &lines.source_flow().tables()[table.source_index() as usize];
        assert_eq!(table.owner(), source.owner());
        assert_eq!(table.content().start().raw(), 131072);
        assert_eq!(table.content().width().get().raw(), 7_672_320);
        assert_eq!(
            table
                .columns()
                .iter()
                .map(|c| c.final_width().get().raw())
                .collect::<Vec<_>>(),
            [3_000_000, 4_672_320]
        );
        assert_eq!(table.rounding_residual(), Length::ZERO);
        assert_eq!(table.last_fraction(), Some(1));
        for cell in source.cells() {
            let frame = frames.region(cell.owner()).unwrap();
            assert_eq!(
                frame.start().raw(),
                131072 + if cell.column() == 1 { 3_000_000 } else { 0 }
            );
            let expected = if cell.colspan().get() == 2 {
                7_672_320
            } else if cell.column() == 0 {
                3_000_000
            } else {
                4_672_320
            };
            assert_eq!(frame.width().get().raw(), expected);
        }
        assert_eq!(
            frames.region(blocks.blocks()[0].owner()),
            frames.region(source.cells()[3].owner())
        );
        let paragraphs = frames.paragraphs();
        // Two header cells, inline math cell, second-row cell, spanning cell.
        assert_eq!(paragraphs.len(), 5);
        for (index, width) in [3_000_000, 4_672_320, 3_000_000, 4_672_320, 7_672_320]
            .iter()
            .enumerate()
        {
            let style = lines.source_flow().paragraphs()[index]
                .style()
                .block_style();
            assert_eq!(
                paragraphs[index].width().get().raw(),
                width - style.start_indent().get().raw() - style.end_indent().get().raw()
            );
        }
        assert!(lines.paragraphs()[2]
            .lines()
            .iter()
            .flat_map(|l| l.items())
            .any(|item| matches!(item, typaxis_layout::ProductionPlacedInline::Vector(_))));
    });
}

#[test]
fn production_table_fraction_rounding_keeps_exact_total_and_last_column_residual() {
    for (extra, residual, widths) in [
        (1, 1, [3_000_000, 1_557_440, 1_557_440, 1_557_441]),
        (2, -1, [3_000_000, 1_557_441, 1_557_441, 1_557_440]),
    ] {
        let mut value = production_table_fixture();
        value["page_masters"]["masters"][0]["body"]["width"] = (8_000_000 + extra).into();
        let table = &mut value["document"]["blocks"][0]["blocks"][0];
        table["columns"] = serde_json::json!([{"kind":"fixed","width":3_000_000},{"kind":"fraction","weight":1},{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}]);
        table["head"][0]["cells"][1]["colspan"] = 3.into();
        table["body"][0]["cells"][1]["colspan"] = 3.into();
        table["body"][1]["cells"][0]["colspan"] = 3.into();
        table["body"][2]["cells"][0]["colspan"] = 4.into();
        with_production_body_inputs(&value, &config(), |lines, _, _| {
            let table = &lines.frames().unwrap().tables()[0];
            assert_eq!(
                table
                    .columns()
                    .iter()
                    .map(|c| c.final_width().get().raw())
                    .collect::<Vec<_>>(),
                widths
            );
            assert_eq!(table.rounding_residual().raw(), residual);
            assert_eq!(table.last_fraction(), Some(3));
            assert_eq!(
                table
                    .columns()
                    .iter()
                    .map(|c| c.final_width().get().raw())
                    .sum::<i64>(),
                table.content().width().get().raw()
            );
        });
    }
}

#[test]
fn production_table_nested_and_sibling_tables_restore_the_enclosing_frame() {
    for nested in [false, true] {
        let mut value = production_table_fixture();
        let cell = &value["document"]["blocks"][0]["blocks"][0]["body"][2]["cells"][0];
        let paragraph = cell["blocks"][0].clone();
        let span = cell["span"].clone();
        let row = serde_json::json!({"node_id":0,"span":span,"cells":[
            {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[paragraph]},
            {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[paragraph]}
        ]});
        let child = serde_json::json!({"kind":"table","node_id":0,"span":span,"classes":["common-grid"],
            "columns":[{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}],"head":[row],"body":[row]});
        if nested {
            value["document"]["blocks"][0]["blocks"][0]["body"][2]["cells"][0]["blocks"] =
                serde_json::json!([child, paragraph]);
        } else {
            value["document"]["blocks"][0]["blocks"]
                .as_array_mut()
                .unwrap()
                .extend([child, paragraph]);
        }
        production_body_renumber(&mut value["document"], &mut 0);
        with_production_body_inputs(&value, &config(), |lines, _, _| {
            let frames = lines.frames().unwrap();
            assert_eq!(frames.tables().len(), 2);
            assert_eq!(
                frames.tables()[1].content().start().raw(),
                if nested { 262144 } else { 131072 }
            );
            assert_eq!(
                frames.tables()[1].content().width().get().raw(),
                if nested { 7_344_640 } else { 7_672_320 }
            );
            let paragraphs = frames.paragraphs();
            assert_eq!(paragraphs.len(), if nested { 9 } else { 10 });
            let last = paragraphs.last().unwrap();
            let style = lines
                .source_flow()
                .paragraphs()
                .last()
                .unwrap()
                .style()
                .block_style();
            assert_eq!(
                last.start().raw(),
                if nested { 131072 } else { 0 } + style.start_indent().get().raw()
            );
            assert_eq!(
                last.width().get().raw(),
                if nested { 7_672_320 } else { 8_000_000 }
                    - style.start_indent().get().raw()
                    - style.end_indent().get().raw()
            );
        });
    }
}

#[test]
fn production_table_columns_reject_exhausted_fraction_and_inexact_fixed_total() {
    use serde_json::json;
    for columns in [
        json!([{"kind":"fixed","width":7_672_320},{"kind":"fraction","weight":1}]),
        json!([{"kind":"fixed","width":7_672_321},{"kind":"fraction","weight":1}]),
        json!([{"kind":"fixed","width":3_000_000},{"kind":"fixed","width":4_672_319}]),
        json!([{"kind":"fixed","width":3_000_000},{"kind":"fixed","width":4_672_321}]),
    ] {
        let mut value = production_table_fixture();
        value["document"]["blocks"][0]["blocks"][0]["columns"] = columns;
        with_production_inline_tagged_context(
            &serde_json::to_vec(&value).unwrap(),
            &config(),
            |prepared, _, _, _, _, _, _, _| {
                let body = typaxis_core::Rect::new(
                    Length::ZERO,
                    Length::ZERO,
                    PositiveLength::new(Length::from_raw(8_000_000).unwrap()).unwrap(),
                    PositiveLength::new(Length::from_raw(12_000_000).unwrap()).unwrap(),
                );
                let error =
                    typaxis_layout::layout_production_body_inline_lines(prepared, body, 100_000)
                        .err()
                        .expect("invalid columns must fail before selected lines are issued");
                assert_eq!(error.owner, prepared.source_flow().tables()[0].owner());
                assert_eq!(
                    error.kind,
                    typaxis_layout::ProductionInlinePreparationErrorKind::InvalidTableColumns
                );
            },
        );
    }
    // The same fixed columns exactly filling the table remain valid.
    let mut value = production_table_fixture();
    value["document"]["blocks"][0]["blocks"][0]["columns"] =
        json!([{"kind":"fixed","width":3_000_000},{"kind":"fixed","width":4_672_320}]);
    with_production_body_inputs(&value, &config(), |lines, _, _| {
        let table = &lines.frames().unwrap().tables()[0];
        assert_eq!(table.last_fraction(), None);
        assert_eq!(table.rounding_residual(), Length::ZERO);
        assert_eq!(table.content().width().get().raw(), 7_672_320);
    });
}

fn production_table_spacing_rule(
    value: &mut serde_json::Value,
    selector: &str,
    before: i64,
    after: i64,
) {
    let rules = value["style_sheet"]["rules"].as_array_mut().unwrap();
    let order = rules.len();
    rules.push(serde_json::json!({"style_id":format!("table-spacing-{order}"),"selector":selector,"extends":null,"source_order":order,"declarations":[
        {"important":false,"name":"space_before","value":{"kind":"length","value":before}},
        {"important":false,"name":"space_after","value":{"kind":"length","value":after}}
    ]}));
}

#[test]
fn production_table_measurements_use_actual_cell_heights_and_exact_cumulative_budget() {
    use typaxis_pagination::{
        prepare_production_table_measurements, ProductionBodyPaginationErrorKind as Error,
    };
    let mut value = production_table_fixture();
    value["document"]["blocks"][0]["classes"] = serde_json::json!(["table-wrapper"]);
    production_table_spacing_rule(&mut value, "semantic_container.table-wrapper", 101, 103);
    production_table_spacing_rule(&mut value, "table.common-grid", 17, 19);
    let mut required = 0;
    for delta in [None, Some(0), Some(1)] {
        let cfg = delta.map_or_else(config, |delta| {
            config_with_limits(ResourceLimits {
                max_fragments: required - delta,
                ..ResourceLimits::default()
            })
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let footnotes =
                typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let result = prepare_production_table_measurements(lines, blocks, &footnotes, limits);
            if delta == Some(1) {
                assert_eq!(result.err().unwrap().kind, Error::FragmentLimit);
                return;
            }
            let measurements = result.unwrap();
            measurements.verify(lines, blocks, limits).unwrap();
            required = measurements.record_charge();
            let again =
                prepare_production_table_measurements(lines, blocks, &footnotes, limits).unwrap();
            assert_eq!(measurements.fingerprint(), again.fingerprint());
            assert_eq!(measurements.tables().len(), 1);
            let table = &measurements.tables()[0];
            assert_eq!(
                (table.space_before().raw(), table.space_after().raw()),
                (118, 122)
            );
            let natural = |index: usize| {
                let paragraph = &lines.paragraphs()[index];
                let style = lines.source_flow().paragraphs()[index]
                    .style()
                    .block_style();
                paragraph
                    .selected()
                    .unwrap()
                    .lines()
                    .iter()
                    .map(|l| l.line().metrics().line_height().get().raw())
                    .sum::<i64>()
                    + style.space_before().get().raw()
                    + style.space_after().get().raw()
            };
            let block = &blocks.blocks()[0];
            let block_height = block.content_height().get().raw()
                + block.space_before().get().raw()
                + block.space_after().get().raw();
            assert_eq!(
                table
                    .cells()
                    .iter()
                    .map(|c| c.natural_height().raw())
                    .collect::<Vec<_>>(),
                [
                    natural(0),
                    natural(1),
                    natural(2),
                    block_height,
                    natural(3),
                    natural(4)
                ]
            );
            let expected = [
                natural(0).max(natural(1)),
                block_height,
                natural(2).max(natural(3)),
                natural(4),
            ];
            assert_eq!(
                table
                    .rows()
                    .iter()
                    .map(|r| r.height().raw())
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(table.height().raw(), expected.iter().sum::<i64>());
            assert_eq!(table.rows()[1].top().raw(), expected[0]);
            assert_eq!(table.rows()[2].top().raw(), expected[0] + expected[1]);
            assert_eq!(
                typaxis_pagination::prepare_production_body_flow(lines, blocks, &footnotes, limits)
                    .err()
                    .unwrap()
                    .kind,
                Error::PendingRegion("table")
            );
        });
    }
}

#[test]
fn production_table_nested_measurements_keep_parallel_cells_and_wrapper_spacing() {
    use serde_json::json;
    use typaxis_pagination::{
        prepare_production_table_measurements, ProductionTableContentSource as Source,
    };
    let mut value = production_table_fixture();
    let cell = &value["document"]["blocks"][0]["blocks"][0]["body"][2]["cells"][0];
    let paragraph = cell["blocks"][0].clone();
    let span = cell["span"].clone();
    let row = json!({"node_id":0,"span":span,"cells":[
        {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[paragraph]},
        {"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[paragraph]}
    ]});
    let child = json!({"kind":"table","node_id":0,"span":span,"classes":["common-grid"],
        "columns":[{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}],"head":[row],"body":[row]});
    let wrapper = json!({"kind":"semantic_container","semantic_kind":"result","anchor_id":null,"node_id":0,"span":span,"classes":["table-wrapper"],"blocks":[child]});
    value["document"]["blocks"][0]["blocks"][0]["body"][2]["cells"][0]["blocks"] =
        json!([paragraph, wrapper, paragraph]);
    production_table_spacing_rule(&mut value, "semantic_container.table-wrapper", 101, 103);
    production_table_spacing_rule(&mut value, "table.common-grid", 17, 19);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let footnotes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &footnotes, limits).unwrap();
        let child = &measured.tables()[1];
        let natural = child.cells()[0].natural_height().raw();
        assert!(child
            .cells()
            .iter()
            .all(|c| c.natural_height().raw() == natural));
        assert_eq!(child.height().raw(), 2 * natural);
        assert_eq!(
            (child.space_before().raw(), child.space_after().raw()),
            (118, 122)
        );
        let cell = measured.tables()[0].cells().last().unwrap();
        assert_eq!(cell.content().len(), 3);
        assert_eq!(cell.content()[1].source(), Source::Table(1));
        assert_eq!(cell.content()[1].top().raw(), natural + 118);
        assert_eq!(
            cell.content()[1].end().raw(),
            natural + 118 + child.height().raw() + 122
        );
        assert_eq!(
            cell.natural_height().raw(),
            2 * natural + child.height().raw() + 240
        );
        assert_eq!(
            measured.tables()[0].rows().last().unwrap().height(),
            cell.natural_height()
        );
        with_production_body_inputs(&value, &config(), |other_lines, other_blocks, _| {
            assert!(measured.verify(other_lines, blocks, limits).is_err());
            assert!(measured.verify(lines, other_blocks, limits).is_err());
        });
    });
}

#[test]
fn production_table_cell_measurements_include_real_list_marker_extent() {
    use serde_json::json;
    let mut value = production_table_fixture();
    let cell = &mut value["document"]["blocks"][0]["blocks"][0]["body"][2]["cells"][0];
    let paragraph = cell["blocks"][0].clone();
    let span = cell["span"].clone();
    cell["blocks"] = json!([{"kind":"list","node_id":0,"span":span,"classes":[],"ordered":true,"start":1,
        "items":[{"node_id":0,"span":span,"blocks":[paragraph]}]}]);
    production_body_set_style(&mut value, "list", "font_size", (32 * 65536).into());
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let footnotes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured = typaxis_pagination::prepare_production_table_measurements(
            lines, blocks, &footnotes, limits,
        )
        .unwrap();
        let cell = measured.tables()[0].cells().last().unwrap();
        let typaxis_pagination::ProductionTableContentSource::FlowItem(index) =
            cell.content()[0].source()
        else {
            panic!("list paragraph stays a real leaf");
        };
        let item = measured.item(index).unwrap();
        assert!(item.leading().raw() > 0 || item.trailing().raw() > 0);
        assert_eq!(
            cell.natural_height().raw(),
            item.space_before().raw()
                + item.consumed_height().unwrap().raw()
                + item.space_after().raw()
        );
        assert!(item.consumed_height().unwrap() > item.height());
    });
}

#[test]
fn production_table_measurements_keep_footnote_cells_in_the_definition_frame() {
    use serde_json::json;
    use typaxis_pagination::ProductionTableContentSource as Source;
    let mut value = production_table_fixture();
    let table = value["document"]["blocks"][0]["blocks"][0].clone();
    let mut paragraph = table["head"][0]["cells"][0]["blocks"][0].clone();
    let point = paragraph["span"].clone();
    paragraph["children"].as_array_mut().unwrap().push(json!({
        "kind":"footnote_reference","node_id":0,"span":point,"footnote_id":"table-note"
    }));
    value["document"]["blocks"][0]["blocks"] = json!([paragraph]);
    value["document"]["footnotes"] = json!([{
        "node_id":0,"span":table["span"],"footnote_id":"table-note","blocks":[table]
    }]);
    value["page_masters"]["masters"][0]["footnote"] =
        json!({"x":955360,"y":13000000,"width":16000000,"height":6000000});
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let footnotes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        assert_eq!(footnotes.references().len(), 1);
        let measured = typaxis_pagination::prepare_production_table_measurements(
            lines, blocks, &footnotes, limits,
        )
        .unwrap();
        let table = &measured.tables()[0];
        let Source::FlowItem(first) = table.cells()[0].content()[0].source() else {
            panic!("header paragraph must remain a real leaf");
        };
        assert!(first > 0);
        let item = measured.item(first).unwrap();
        let frame = lines.frames().unwrap().region(item.owner()).unwrap();
        let paragraph_style = lines.source_flow().paragraphs()[1].style().block_style();
        assert_eq!(
            item.x().raw(),
            blocks.page_geometry().body().x().raw()
                + frame.start().raw()
                + paragraph_style.start_indent().get().raw()
        );
        assert_eq!(
            lines.frames().unwrap().tables()[0].content().start().raw(),
            lines.frames().unwrap().footnotes()[0]
                .content()
                .start()
                .raw()
                + 131072
        );
        let marker = &lines.footnote_markers()[0];
        let height = marker
            .font()
            .ascender()
            .checked_sub(marker.font().descender())
            .unwrap();
        assert!(item.consumed_height().unwrap() >= height);
        assert_ne!(measured.item(0).unwrap().owner(), item.owner());
        let mut search =
            typaxis_pagination::prepare_production_table_search(&measured, 0, limits, 100_000)
                .unwrap();
        assert_eq!(
            typaxis_pagination::prepare_production_table_footnote_search(
                &measured, 0, limits, 100_000
            )
            .err()
            .unwrap()
            .kind,
            typaxis_pagination::ProductionBodyPaginationErrorKind::PendingRegion(
                "table_footnote_definition"
            )
        );
        assert_eq!(search.maximum_height().raw(), 6_000_000);
        let cursor = search.begin().unwrap();
        assert_eq!(
            search
                .evaluate(&cursor, Length::from_raw(6_000_001).unwrap())
                .err()
                .unwrap()
                .kind,
            typaxis_pagination::ProductionBodyPaginationErrorKind::InvalidTableCapacity
        );
        assert!(search
            .evaluate(&cursor, search.maximum_height())
            .unwrap()
            .is_some());
    });
}

#[test]
fn production_table_caption_keep_applies_to_the_table_and_caption_siblings() {
    use serde_json::json;
    use typaxis_pagination::ProductionTableContentSource as Source;
    for following in [false, true] {
        let mut value = production_table_fixture();
        let table = value["document"]["blocks"][0]["blocks"][0].clone();
        let paragraph = table["body"][2]["cells"][0]["blocks"][0].clone();
        let math = table["body"][0]["cells"][1]["blocks"][0].clone();
        let caption = if following {
            json!([table, paragraph])
        } else {
            json!([table])
        };
        value["document"]["blocks"][0]["blocks"] = json!([{
            "kind":"vector_figure","node_id":0,"span":table["span"],"classes":[],"image_id":4,
            "viewport":math["metrics"]["viewport"],"alt":math["alt"],"caption":caption
        }]);
        production_body_set_style(&mut value, "vector_figure", "keep_caption", true.into());
        production_body_renumber(&mut value["document"], &mut 0);
        let cfg = config();
        let decoded = wire::StagingSemanticDocumentPackageDecoder::new()
            .decode(
                &serde_json::to_vec(&value).unwrap(),
                &wire::DocumentPackageDecodePolicy::new(cfg.limits()),
            )
            .unwrap();
        let package = typaxis_syntax::StagingSemanticPackageParser::new()
            .parse(decoded, cfg.limits())
            .unwrap();
        let limits = cfg
            .m4_limits()
            .cloned()
            .unwrap_or_else(|| typaxis_core::M4EffectiveResourceLimits::defaults_for(cfg.limits()));
        let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
        let profile = typaxis_machine_profile::preflight_staging_precomposed_vector_profile(
            &package, &limits, &identity,
        )
        .unwrap();
        let figure = &value["document"]["blocks"][0]["blocks"][0];
        let caption_table = &figure["caption"][0];
        assert_eq!(
            profile
                .vector_owners()
                .map(|n| u64::from(n.get()))
                .collect::<Vec<_>>(),
            [
                figure["node_id"].as_u64().unwrap(),
                caption_table["body"][0]["cells"][0]["blocks"][0]["children"][1]["node_id"]
                    .as_u64()
                    .unwrap(),
                caption_table["body"][0]["cells"][1]["blocks"][0]["node_id"]
                    .as_u64()
                    .unwrap(),
            ]
        );
        with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
            let footnotes =
                typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured = typaxis_pagination::prepare_production_table_measurements(
                lines, blocks, &footnotes, limits,
            )
            .unwrap();
            let table = &measured.tables()[0];
            assert!(table.keep_together());
            assert_eq!(table.keep_with_next(), following);
            assert!(measured.item(0).unwrap().keep_with_next());
            for cell in table.cells() {
                for content in cell.content() {
                    let Source::FlowItem(index) = content.source() else {
                        panic!("fixture cell has a direct leaf");
                    };
                    assert!(
                        !measured.item(index).unwrap().keep_with_next(),
                        "caption keep must not join parallel cells"
                    );
                }
            }
        });
    }
}

#[test]
fn production_vector_caption_math_reaches_the_public_pdf_without_owner_omission() {
    use serde_json::json;
    let mut value = production_body_fixture(12_000_000);
    let container = value["document"]["blocks"][0].clone();
    let block = container["blocks"][1].clone();
    value["document"]["blocks"][0]["blocks"] = json!([{
        "kind":"vector_figure","node_id":0,"span":container["span"],"classes":[],"image_id":4,
        "viewport":block["metrics"]["viewport"],"alt":block["alt"],"caption":container["blocks"]
    }]);
    production_body_renumber(&mut value["document"], &mut 0);
    let cfg = EffectiveConfig::new_for_contract(
        DocumentPackageContractId::V1_4,
        false,
        PdfStreamCompression::None,
        vec![ConfigResourceRoot::ProjectRoot],
        ["http", "https", "mailto", "tel"]
            .map(str::to_owned)
            .to_vec(),
        EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
        ResourceLimits::default(),
    )
    .unwrap();
    let (package, navigation, limits, admitted) =
        production_text_fixture(&serde_json::to_vec(&value).unwrap(), &cfg);
    let semantics =
        typaxis_syntax::validate_staging_structure_semantics_v2(&package, &navigation, &limits)
            .unwrap();
    let identity = typaxis_machine_profile::StagingSemanticContainerSessionIdentity::fresh();
    let profile = typaxis_machine_profile::preflight_staging_tagged_pdf_profile_v2(
        &package,
        &navigation,
        &semantics,
        &limits,
        &identity,
    )
    .unwrap();
    let (pdf, _, selected, _, _, _, _) = build_production_book_pdf(
        &package,
        &navigation,
        &semantics,
        &profile,
        &admitted,
        &limits,
        &cfg,
    )
    .unwrap()
    .into_parts();
    assert_eq!(selected, pdf.selected_layout_fingerprint().bytes());
    let text = String::from_utf8_lossy(pdf.bytes());
    assert_eq!(text.matches("/S /Figure").count(), 1);
    assert_eq!(text.matches("/S /Caption").count(), 1);
    assert_eq!(text.matches("/S /Formula").count(), 2);
    assert_eq!(text.matches("/Subtype /Form").count(), 2);
    assert_eq!(text.matches(" Do").count(), 3);
    if let Some(path) = std::env::var_os("VMB_VECTOR_CAPTION_PDF_OUT") {
        std::fs::write(path, pdf.bytes()).unwrap();
    }
}
