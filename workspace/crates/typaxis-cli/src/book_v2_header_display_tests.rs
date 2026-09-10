use super::*;
#[path = "book_v2_header_resource_tests.rs"]
mod resources;
use typaxis_display_list::book_v2::*;
use typaxis_resources::AdmittedProductionResourceLedgerV3;

pub(super) fn verify_driver_resources(
    display: &BookV2BodyDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    resources::verify(display, limits, None);
}

pub(super) fn verify(
    source: &BookV2BodyMathTerminals<'_, '_, '_, '_, '_, '_, '_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    repeated_only_gid: Option<u16>,
) {
    let run = |work, prior| -> Result<_, BookV2MathDisplayError> {
        let mut builder = BookV2MathDisplayBuilder::new(source, admitted, limits, work, prior, 0)?;
        let body = builder.build_body()?;
        let mut text = body.text().draws().iter();
        let mut index = 0;
        for (pi, page) in source.source().geometry().pages().iter().enumerate() {
            for (fi, (placed, role, repeated)) in page.fragments_with_roles().enumerate() {
                let flow = source.fragment_flow(index).unwrap();
                assert!(std::ptr::eq(
                    flow,
                    source.source().fragment_flow(pi, fi).unwrap()
                ));
                if let typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                } = placed.fragment().source()
                {
                    let p = &flow.lines().paragraphs()[paragraph_index as usize];
                    for (inline, item) in p.lines()[line_index as usize].items().iter().enumerate()
                    {
                        if let ProductionPlacedInline::Text(cluster) = item {
                            let draw = text.next().unwrap();
                            assert!(std::ptr::eq(draw.cluster(), cluster));
                            assert!(std::ptr::eq(draw.font(), p.font().unwrap()));
                            assert_eq!(draw.fragment_index(), index);
                            assert_eq!(draw.inline_index() as usize, inline);
                            assert_eq!(draw.cell_owner(), role.map(|r| r.owner()));
                            assert_eq!(draw.repeated_header(), repeated);
                            assert_eq!(draw.glyphs().len(), cluster.glyphs().len());
                            for (paint, glyph) in draw.glyphs().iter().zip(cluster.glyphs()) {
                                assert_eq!(paint.original_gid(), glyph.glyph().original_gid);
                                assert_eq!(
                                    paint.x().raw(),
                                    placed.fragment().bounds().x().raw() + glyph.x().raw()
                                );
                                assert_eq!(
                                    paint.y().raw(),
                                    placed.fragment().bounds().y().raw() + glyph.y().raw()
                                );
                            }
                            assert_eq!(draw.exact_text(), cluster.utf8());
                        }
                    }
                }
                index += 1;
            }
        }
        assert!(text.next().is_none());
        assert_eq!(index, source.fragment_count());
        assert!(source.fragment_flow(index).is_none());
        assert!(source.fragment_flow(usize::MAX).is_none());
        for draw in body.numbers().draws() {
            let g = draw.placement().geometry();
            let numbers = source
                .fragment_flow(g.fragment_index() as usize)
                .unwrap()
                .blocks()
                .unwrap()
                .numbers()
                .unwrap();
            assert!(std::ptr::eq(
                draw.shape(),
                numbers.shape(g.parent_owner()).unwrap()
            ));
        }
        for draw in body.markers().draws() {
            let shaped = source
                .fragment_flow(draw.fragment_index())
                .unwrap()
                .lines()
                .prepared()
                .shaped();
            match (draw.source(), draw.placement()) {
                (BookV2MarkerSource::List(s), BookV2MarkerPlacement::List(p)) => assert!(
                    std::ptr::eq(s, &shaped.list_markers()[p.marker_index() as usize])
                ),
                (BookV2MarkerSource::Footnote(s), BookV2MarkerPlacement::Footnote(p)) => assert!(
                    std::ptr::eq(s, &shaped.footnote_markers()[p.definition_index()])
                ),
                _ => panic!("marker kind"),
            }
        }
        for draw in body.images().draws() {
            let prepared = source
                .fragment_flow(draw.fragment_index())
                .unwrap()
                .lines()
                .prepared();
            match draw.source() {
                BookV2ImageSource::Vector(v) => assert!(std::ptr::eq(
                    v,
                    prepared
                        .vector_bindings()
                        .unwrap()
                        .receipt(v.node_id())
                        .unwrap()
                )),
                BookV2ImageSource::Figure(v) => {
                    assert!(prepared.figures().iter().any(|p| std::ptr::eq(p, v)))
                }
            }
        }
        let mut repeated_anchors = 0;
        for position in body.anchors().positions() {
            let flow = source.fragment_flow(position.fragment_index()).unwrap();
            let typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } = position.fragment().fragment().source()
            else {
                panic!("anchor paragraph");
            };
            let p = &flow.lines().paragraphs()[paragraph_index as usize];
            assert!(p
                .anchors()
                .iter()
                .any(|a| std::ptr::eq(a, position.anchor())));
            assert_eq!(
                position.anchor().position().unwrap().line_index(),
                line_index
            );
            repeated_anchors += usize::from(position.repeated_header());
        }
        assert!(repeated_anchors > 0);
        let mut glyphs = 0;
        for i in 0..body.paints().len() {
            for slot in 0..body.font_slot_count(i).unwrap() {
                if let Some(font) = body.font_use(i, slot)? {
                    assert!(std::ptr::eq(font.instance().ledger(), admitted));
                    let (fragment, id, native) = match body.paints()[i] {
                        BookV2BodyPaintIndex::Text(n) => {
                            let d = &body.text().draws()[n];
                            (
                                d.fragment_index(),
                                d.cluster().run().glyph_run().font,
                                false,
                            )
                        }
                        BookV2BodyPaintIndex::Marker(n) => {
                            let d = &body.markers().draws()[n];
                            (d.fragment_index(), d.source().glyph_run().font, false)
                        }
                        BookV2BodyPaintIndex::EquationNumber(n) => {
                            let d = &body.numbers().draws()[n];
                            (
                                d.placement().geometry().fragment_index() as usize,
                                d.shape().font_instance_id(),
                                false,
                            )
                        }
                        BookV2BodyPaintIndex::Math(n) => {
                            let d = &body.math().draws()[n];
                            let BookV2BodyMathSource::Native(r) = d.terminal().source() else {
                                panic!("vector font");
                            };
                            (d.terminal().fragment_index(), r.font_instance_id(), true)
                        }
                        _ => panic!("nonfont paint"),
                    };
                    let prepared = source.fragment_flow(fragment).unwrap().lines().prepared();
                    let expected = if native {
                        prepared
                            .native_math()
                            .unwrap()
                            .font_instances()
                            .resolve(id)
                            .unwrap()
                    } else {
                        prepared.shaped().font_instances().resolve(id).unwrap()
                    };
                    assert_eq!(
                        font.instance().font_instance_id(),
                        expected.font_instance_id()
                    );
                    assert_eq!(
                        font.instance().table_fingerprint(),
                        expected.table_fingerprint()
                    );
                    assert!(std::ptr::eq(font.instance().font(), expected.font()));
                    glyphs += font.glyphs().len();
                }
            }
        }
        if source
            .source()
            .flow()
            .lines()
            .prepared()
            .source_flow()
            .navigation()
            .anchors()
            .iter()
            .any(|(a, _)| a.as_str() == "empty.begin")
        {
            let pages = source.source().geometry().pages().len();
            let empty = body.anchors().nonpainting_lines();
            assert_eq!(empty.len(), pages);
            assert_eq!(empty.iter().filter(|l| !l.repeated_header()).count(), 1);
            let raster = body
                .images()
                .draws()
                .iter()
                .filter(|d| matches!(d.paint(), BookV2ImagePaint::Raster { .. }))
                .collect::<Vec<_>>();
            assert_eq!(raster.len(), pages);
            assert_eq!(raster.iter().filter(|d| !d.repeated_header()).count(), 1);
        }
        assert!(glyphs > 0);
        if work == 100_000_000 && prior == 0 {
            resources::verify(&body, limits, repeated_only_gid);
        }
        body.verify_resources(admitted, limits)?;
        if work == 100_000_000 && prior == 0 {
            assert_book_v2_body_resources(source, admitted, limits);
        }
        Ok((
            body.work_steps(),
            body.record_charge(),
            body.fingerprint(),
            body.text().draws().len(),
            body.markers().draws().len(),
            body.images().draws().len(),
            body.anchors().positions().len(),
            glyphs,
        ))
    };
    let full = run(100_000_000, 0).unwrap();
    assert_eq!(run(full.0, 0).unwrap(), full);
    assert!(run(full.0 - 1, 0).is_err());
    let delta = full.1 - source.record_charge();
    let prior = limits.base().get().max_fragments - delta;
    assert_eq!(
        run(full.0, prior).unwrap().1,
        limits.base().get().max_fragments
    );
    assert!(run(full.0, prior + 1).is_err());
    eprintln!(
        "header display: work={},records={},text={},markers={},images={},anchors={},glyphs={}",
        full.0, full.1, full.3, full.4, full.5, full.6, full.7
    );
}
