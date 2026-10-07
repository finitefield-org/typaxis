#[test]
fn production_table_topology_keeps_sections_spans_and_source_owner() {
    let package = parse(COMBINED);
    let (limits, navigation) = navigation(&package);
    let mut flow = prepare_production_text_flow(&package, &navigation, &limits).unwrap();
    assert_eq!(flow.tables().len(), 1);
    let table = &flow.tables()[0];
    assert_eq!(table.owner().get(), 23);
    assert_eq!(table.columns().len(), 2);
    assert_eq!(table.rows().len(), 4);
    assert_eq!(table.cells().len(), 6);
    assert_eq!(table.rows()[0].section(), ProductionTableSection::Head);
    assert!(table.rows()[1..]
        .iter()
        .all(|r| r.section() == ProductionTableSection::Body));
    assert_eq!(
        table
            .cells()
            .iter()
            .map(|c| (
                c.owner().get(),
                c.row(),
                c.column(),
                c.rowspan().get(),
                c.colspan().get()
            ))
            .collect::<Vec<_>>(),
        [
            (25, 0, 0, 1, 1),
            (28, 0, 1, 1, 1),
            (32, 1, 0, 2, 1),
            (35, 1, 1, 1, 1),
            (39, 2, 1, 1, 1),
            (43, 3, 0, 1, 2)
        ]
    );
    assert_eq!(flow.table_record_charge(), 23);
    flow.verify(&package, &navigation, &limits).unwrap();
    flow.tables.pop();
    assert_eq!(
        flow.verify(&package, &navigation, &limits)
            .unwrap_err()
            .kind,
        ProductionFlowErrorKind::ReceiptMismatch
    );
}

#[test]
fn production_table_grid_rejects_section_crossing_holes_and_overlaps() {
    for (row, cell, field, number) in [
        (0, 0, "rowspan", 4),
        (0, 1, "colspan", 2),
        (1, 0, "colspan", 2),
        (2, 0, "colspan", 1),
    ] {
        let original = parse(COMBINED);
        let mut wire = original.checked_wire().unwrap().clone();
        let mut document = wire.document().clone();
        let table = document
            .blocks
            .iter_mut()
            .find(|b| matches!(b, WireStagingM4Block::Table { .. }))
            .unwrap();
        let WireStagingM4Block::Table { body, .. } = table else {
            unreachable!()
        };
        if field == "rowspan" {
            body[row].cells[cell].rowspan = number;
        } else {
            body[row].cells[cell].colspan = number;
        }
        wire.replace_typed_regions(document, wire.resources().clone());
        let package = parse(
            StagingSemanticDocumentPackageEncoder::new()
                .encode(&wire)
                .unwrap()
                .as_bytes(),
        );
        let (limits, navigation) = navigation(&package);
        let error = prepare_production_text_flow(&package, &navigation, &limits)
            .err()
            .unwrap();
        assert_eq!(error.kind, ProductionFlowErrorKind::InvalidTableGrid);
    }
}

#[test]
fn production_table_grid_precharges_occupancy_and_retained_topology() {
    for maximum in [23, 22] {
        let base = ValidatedResourceLimits::new(ResourceLimits {
            max_fragments: maximum,
            ..ResourceLimits::default()
        })
        .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(COMBINED, &DocumentPackageDecodePolicy::new(&base))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &base)
            .unwrap();
        let (limits, navigation) = navigation(&package);
        let flow = prepare_production_text_flow(&package, &navigation, &limits);
        if maximum == 23 {
            assert_eq!(flow.unwrap().table_record_charge(), maximum);
        } else {
            assert_eq!(
                flow.err().unwrap(),
                ProductionFlowError {
                    owner: NodeId::new(23),
                    kind: ProductionFlowErrorKind::NodeLimit
                }
            );
        }
    }
}
