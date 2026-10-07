use super::*;
use typaxis_display_list::book_v2::BookV2EquationNumberDisplay;
use typaxis_shaping::ShapeSourceSpan;

pub(super) fn check_number_draws(
    display: &BookV2EquationNumberDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
) {
    let source = display.source();
    let expected = source
        .source()
        .geometry()
        .pages()
        .iter()
        .flat_map(|p| p.equation_numbers())
        .collect::<Vec<_>>();
    assert_eq!(display.draws().len(), expected.len());
    for (draw, placement) in display.draws().iter().zip(expected) {
        assert!(std::ptr::eq(draw.placement(), placement));
        let geometry = placement.geometry();
        let shape = source
            .source()
            .flow()
            .blocks()
            .unwrap()
            .numbers()
            .unwrap()
            .shape(geometry.parent_owner())
            .unwrap();
        assert!(std::ptr::eq(draw.shape(), shape));
        assert_eq!(shape.fingerprint(), geometry.shape_fingerprint());
        let font = draw.font();
        assert_eq!(font.face_id(), shape.font_face_id());
        assert_eq!(font.content_hash(), shape.font_sha256());
        assert_eq!(font.face_index(), shape.face_index());
        assert_eq!(font.size(), shape.font_size());
        assert_eq!(
            display
                .admitted()
                .font(font.face_id())
                .unwrap()
                .content_hash(),
            font.content_hash()
        );
        let rect = geometry.bounds();
        let leading = rect.height().get().raw() - font.ascender().raw() + font.descender().raw();
        let baseline = rect.y().raw() + leading / 2 + font.ascender().raw();
        let runs = shape.runs();
        let mut observed = draw.clusters().iter();
        for (index, run) in runs.iter().enumerate() {
            // Pairwise embedding parity determines which run precedes this one;
            // independent from the kernel's descending-level reversal loops.
            let mut left = rect.x().raw();
            for (other, r) in runs.iter().enumerate() {
                if other == index {
                    continue;
                }
                let minimum = runs[index.min(other)..=index.max(other)]
                    .iter()
                    .map(|r| r.bidi_level().get())
                    .min()
                    .unwrap();
                let before = if minimum % 2 == 0 {
                    other < index
                } else {
                    other > index
                };
                if before {
                    left += r.glyphs().iter().map(|g| g.advance_x.raw()).sum::<i64>();
                }
            }
            for (cluster_index, cluster) in run.clusters().iter().enumerate() {
                let paint = observed.next().expect("authored number cluster missing");
                assert_eq!(paint.run_index(), index as u32);
                assert_eq!(paint.cluster_index(), cluster_index as u32);
                let ShapeSourceSpan::Parsed(span) = cluster.source_span else {
                    panic!("authored number became generated");
                };
                assert_eq!(paint.text_span().text_id().get(), span.text_id().get());
                assert_eq!(paint.text_span().range().start_byte(), span.start_byte());
                assert_eq!(paint.text_span().range().end_byte(), span.end_byte());
                let start = shape
                    .source()
                    .equation_number()
                    .unwrap()
                    .text()
                    .text_span()
                    .start_byte()
                    .get();
                assert_eq!(
                    paint.exact_text(),
                    &shape.text()[(span.start_byte().get() - start) as usize
                        ..(span.end_byte().get() - start) as usize]
                );
                let start = cluster.glyph_start as usize;
                let end = cluster.glyph_end as usize;
                assert_eq!(paint.glyphs().len(), end - start);
                let pen = left
                    + run.glyphs()[..start]
                        .iter()
                        .map(|g| g.advance_x.raw())
                        .sum::<i64>();
                for (i, (g, original)) in paint
                    .glyphs()
                    .iter()
                    .zip(&run.glyphs()[start..end])
                    .enumerate()
                {
                    let x = pen
                        + run.glyphs()[start..start + i]
                            .iter()
                            .map(|g| g.advance_x.raw())
                            .sum::<i64>()
                        + original.offset_x.raw();
                    assert_eq!(g.original_gid(), original.original_gid);
                    assert_eq!(g.x().raw(), x);
                    assert_eq!(g.y().raw(), baseline - original.offset_y.raw());
                }
                let advance = run.glyphs()[start..end]
                    .iter()
                    .map(|g| g.advance_x.raw())
                    .sum::<i64>();
                if start != end && advance > 0 {
                    let bounds = paint.logical_bounds().unwrap();
                    assert_eq!(bounds.x().raw(), pen);
                    assert_eq!(bounds.y(), rect.y());
                    assert_eq!(bounds.width().get().raw(), advance);
                    assert_eq!(bounds.height(), rect.height());
                } else {
                    assert_eq!(paint.logical_bounds(), None);
                }
            }
        }
        assert!(observed.next().is_none());
        assert_eq!(
            draw.clusters()
                .iter()
                .map(|c| c.exact_text())
                .collect::<String>(),
            shape.text()
        );
    }
}

pub(super) fn assert_number_display(
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
        let display = builder.build_equation_numbers()?;
        assert!(std::ptr::eq(display.source(), source));
        assert!(std::ptr::eq(display.admitted(), admitted));
        assert_eq!(display.record_charge(), builder.record_charge());
        assert_eq!(display.work_steps(), builder.work_steps());
        check_number_draws(&display);
        let second = builder.build_equation_numbers()?;
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
    assert!(matches!(
        exhausted.build_equation_numbers(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(
        (exhausted.record_charge(), exhausted.work_steps()),
        (records, work - 1)
    );
    assert!(matches!(
        exhausted.build_equation_numbers(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(
        (exhausted.record_charge(), exhausted.work_steps()),
        (records, work - 1)
    );
    let mut reordered =
        BookV2MathDisplayBuilder::new(source, admitted, limits, work, 0, 0).unwrap();
    assert_eq!(
        reordered.build_equation_numbers().unwrap().fingerprint(),
        fp
    );
    reordered.build().unwrap();
    reordered.build_text().unwrap();
    assert_eq!(
        reordered.build_equation_numbers().unwrap().fingerprint(),
        fp
    );
    assert_eq!(
        (reordered.record_charge(), reordered.work_steps()),
        (records, work)
    );
}
