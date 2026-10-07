use super::*;
use typaxis_display_list::book_v2::{
    BookV2MarkerDisplay, BookV2MarkerPlacement as Placement, BookV2MarkerSource as Source,
};
use typaxis_shaping::ShapeSourceSpan;

fn check(display: &BookV2MarkerDisplay<'_, '_, '_, '_, '_, '_, '_, '_>) {
    let source = display.source();
    let flow = source.source().flow().lines().prepared().source_flow();
    let shaped = source.source().flow().lines().prepared().shaped();
    let parsed = flow.body().body().wire().text_buffers().len() as u32;
    let mut expected = Vec::new();
    let mut expected_separators = Vec::new();
    let mut offset = 0usize;
    for page in source.source().geometry().pages() {
        for (index, m) in page.list_markers().iter().enumerate() {
            expected.push((
                offset + m.fragment_index() as usize,
                1,
                index,
                Placement::List(m),
                page,
            ));
        }
        for (index, m) in page.footnote_markers().iter().enumerate() {
            expected.push((
                offset + m.fragment_index() as usize,
                0,
                index,
                Placement::Footnote(m),
                page,
            ));
        }
        if let Some(ink) = page.separator_ink() {
            let first = page
                .fragments()
                .iter()
                .position(|f| f.definition_index().is_some())
                .unwrap();
            expected_separators.push((page.selection().page_index(), offset + first, ink));
        }
        offset += page.fragments().len();
    }
    expected.sort_by_key(|(fragment, priority, index, _, _)| (*fragment, *priority, *index));
    assert_eq!(display.draws().len(), expected.len());
    for (draw, (index, _, _, placement, page)) in display.draws().iter().zip(expected) {
        assert_eq!(draw.fragment_index(), index);
        let local = placement.local_fragment_index() as usize;
        assert!(std::ptr::eq(draw.fragment(), &page.fragments()[local]));
        assert_eq!(draw.cell_role(), page.cell_roles()[local]);
        assert_eq!(
            draw.repeated_header(),
            page.fragments_with_roles().nth(local).unwrap().2
        );
        let marker = draw.source();
        match (draw.placement(), placement, marker) {
            (Placement::List(actual), Placement::List(expected), Source::List(shape)) => {
                assert!(std::ptr::eq(actual, expected));
                assert!(std::ptr::eq(
                    shape,
                    &shaped.list_markers()[expected.marker_index() as usize]
                ));
                assert_eq!(
                    marker.provenance(),
                    flow.list_marker_provenance(expected.marker_index() as usize)
                        .unwrap()
                );
                assert_eq!(
                    marker.utf8(),
                    flow.list_marker_text(expected.marker_index() as usize)
                        .unwrap()
                );
            }
            (
                Placement::Footnote(actual),
                Placement::Footnote(expected),
                Source::Footnote(shape),
            ) => {
                assert!(std::ptr::eq(actual, expected));
                assert!(std::ptr::eq(
                    shape,
                    &shaped.footnote_markers()[expected.definition_index()]
                ));
                assert_eq!(
                    marker.provenance(),
                    flow.footnote_marker_provenance(shape.source().owner())
                        .unwrap()
                );
                assert_eq!(
                    marker.utf8(),
                    flow.footnote_marker_text(shape.source().owner()).unwrap()
                );
            }
            _ => panic!("marker kind/order/source changed"),
        }
        let font = marker.font();
        let face = display.admitted().font(font.face_id()).unwrap();
        assert_eq!(font.content_hash(), face.content_hash());
        assert_eq!(font.face_index(), face.face_index());
        assert_eq!(placement.bounds().width(), marker.advance());
        let run = marker.glyph_run();
        assert_eq!(draw.clusters().len(), run.clusters.len());
        for (index, (paint, cluster)) in draw.clusters().iter().zip(&run.clusters).enumerate() {
            assert_eq!(paint.cluster_index(), index as u32);
            let ShapeSourceSpan::Generated(provenance) = cluster.source_span else {
                panic!("generated marker became authored");
            };
            assert_eq!(paint.provenance(), provenance);
            let span = provenance.text_span();
            let full = marker.provenance().text_span().range();
            assert_eq!(
                marker
                    .provenance()
                    .subspan(span.range().start_byte(), span.range().end_byte()),
                Some(provenance)
            );
            assert_eq!(
                paint.text_span().text_id().get(),
                parsed + span.text_id().get()
            );
            assert_eq!(paint.text_span().range(), span.range());
            assert_eq!(
                paint.exact_text(),
                &marker.utf8()[(span.range().start_byte().get() - full.start_byte().get()) as usize
                    ..(span.range().end_byte().get() - full.start_byte().get()) as usize]
            );
            let start = cluster.glyph_start as usize;
            let end = cluster.glyph_end as usize;
            assert_eq!(paint.glyphs().len(), end - start);
            for (i, (g, original)) in paint
                .glyphs()
                .iter()
                .zip(&run.glyphs[start..end])
                .enumerate()
            {
                assert_eq!(g.original_gid(), original.original_gid);
                let x = placement.bounds().x().raw()
                    + run.glyphs[..start + i]
                        .iter()
                        .map(|g| g.advance_x.raw())
                        .sum::<i64>()
                    + original.offset_x.raw();
                let y = placement.baseline().raw()
                    - run.glyphs[..start + i]
                        .iter()
                        .map(|g| g.advance_y.raw())
                        .sum::<i64>()
                    - original.offset_y.raw();
                assert_eq!(g.x().raw(), x);
                assert_eq!(g.y().raw(), y);
            }
            let width = run.glyphs[start..end]
                .iter()
                .map(|g| g.advance_x.raw())
                .sum::<i64>();
            if width > 0 {
                let bounds = paint.logical_bounds().unwrap();
                assert_eq!(
                    bounds.x().raw(),
                    placement.bounds().x().raw()
                        + run.glyphs[..start]
                            .iter()
                            .map(|g| g.advance_x.raw())
                            .sum::<i64>()
                );
                assert_eq!(bounds.y(), placement.bounds().y());
                assert_eq!(bounds.width().get().raw(), width);
                assert_eq!(bounds.height(), placement.bounds().height());
            } else {
                assert_eq!(paint.logical_bounds(), None);
            }
        }
        assert_eq!(
            draw.clusters()
                .iter()
                .map(|c| c.exact_text())
                .collect::<String>(),
            marker.utf8()
        );
    }
    assert_eq!(display.separators().len(), expected_separators.len());
    for (draw, (page, before, ink)) in display.separators().iter().zip(expected_separators) {
        assert_eq!(draw.page_index(), page);
        assert_eq!(draw.before_fragment_index(), before);
        assert_eq!(draw.ink(), ink);
    }
}
pub(super) fn assert_marker_display(
    source: &BookV2BodyMathTerminals<'_, '_, '_, '_, '_, '_, '_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
) {
    let run = |prior,
               maximum,
               work_before|
     -> Result<(u64, u64, [u8; 32]), BookV2MathDisplayError> {
        let mut builder =
            BookV2MathDisplayBuilder::new(source, admitted, limits, maximum, prior, work_before)?;
        builder.build()?;
        builder.build_text()?;
        builder.build_equation_numbers()?;
        let display = builder.build_markers()?;
        assert!(std::ptr::eq(display.source(), source));
        assert!(std::ptr::eq(display.admitted(), admitted));
        check(&display);
        assert_eq!(display.record_charge(), builder.record_charge());
        assert_eq!(display.work_steps(), builder.work_steps());
        let second = builder.build_markers()?;
        assert_eq!(second.fingerprint(), display.fingerprint());
        assert!(second.record_charge() > display.record_charge());
        assert!(second.work_steps() > display.work_steps());
        Ok((
            builder.record_charge(),
            builder.work_steps(),
            display.fingerprint(),
        ))
    };
    let (records, work, fp) = run(0, 10_000_000, 0).unwrap();
    let prior = limits.base().get().max_fragments - (records - source.record_charge());
    assert_eq!(
        run(prior, work, 0).unwrap(),
        (limits.base().get().max_fragments, work, fp)
    );
    assert!(
        matches!(run(prior+1,work,0),Err(BookV2MathDisplayError::Display(e)) if e.kind==ProductionBodyDisplayErrorKind::RecordLimit)
    );
    assert!(matches!(
        run(0, work - 1, 0),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(
        run(0, work + 17, source.work_steps() + 17).unwrap(),
        (records, work + 17, fp)
    );
    let mut exhausted =
        BookV2MathDisplayBuilder::new(source, admitted, limits, work - 1, 0, 0).unwrap();
    exhausted.build().unwrap();
    exhausted.build_text().unwrap();
    exhausted.build_equation_numbers().unwrap();
    exhausted.build_markers().unwrap();
    assert!(matches!(
        exhausted.build_markers(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(
        (exhausted.record_charge(), exhausted.work_steps()),
        (records, work - 1)
    );
    assert!(matches!(
        exhausted.build_markers(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(
        (exhausted.record_charge(), exhausted.work_steps()),
        (records, work - 1)
    );
    let mut reverse = BookV2MathDisplayBuilder::new(source, admitted, limits, work, 0, 0).unwrap();
    let first = reverse.build_markers().unwrap();
    assert_eq!(first.fingerprint(), fp);
    let shaped = source.source().flow().lines().prepared().shaped();
    let mut preflight = source.work_steps();
    for page in source.source().geometry().pages() {
        preflight += 1;
        for marker in page.list_markers() {
            preflight += 1 + shaped.list_markers()[marker.marker_index() as usize]
                .glyph_run()
                .clusters
                .len() as u64;
        }
        for marker in page.footnote_markers() {
            preflight += 1 + shaped.footnote_markers()[marker.definition_index()]
                .glyph_run()
                .clusters
                .len() as u64;
        }
    }
    let mut early =
        BookV2MathDisplayBuilder::new(source, admitted, limits, preflight, 0, 0).unwrap();
    assert!(matches!(
        early.build_markers(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(early.record_charge(), first.record_charge());
    assert_eq!(early.work_steps(), preflight);
    assert!(matches!(
        early.build_markers(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(early.record_charge(), first.record_charge());
    assert_eq!(early.work_steps(), preflight);
    reverse.build_equation_numbers().unwrap();
    reverse.build_text().unwrap();
    reverse.build().unwrap();
    assert_eq!(reverse.build_markers().unwrap().fingerprint(), fp);
    assert_eq!(
        (reverse.record_charge(), reverse.work_steps()),
        (records, work)
    );
}
