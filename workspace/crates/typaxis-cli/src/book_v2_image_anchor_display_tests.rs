use super::*;
use typaxis_display_list::book_v2::{
    BookV2AnchorDisplay, BookV2ImageDisplay, BookV2ImagePaint, BookV2ImageSource,
};
use typaxis_layout::{
    PrecomposedVectorPlacementInput, ProductionFigureMedia, ProductionPlacedInline,
};
use typaxis_pagination::ProductionBodyFragmentSource;
use typaxis_syntax::PrecomposedVectorKind;

fn check_images(display: &BookV2ImageDisplay<'_, '_, '_, '_, '_, '_, '_, '_>) {
    let source = display.source().source();
    let flow = source.flow();
    let prepared = flow.lines().prepared();
    let mut expected = Vec::new();
    let mut index = 0usize;
    for page in source.geometry().pages() {
        for (fragment, role, repeated) in page.fragments_with_roles() {
            match fragment.fragment().source() {
                ProductionBodyFragmentSource::Figure { figure_index } => {
                    expected.push((
                        index,
                        None,
                        fragment,
                        role,
                        repeated,
                        BookV2ImageSource::Figure(&prepared.figures()[figure_index as usize]),
                    ));
                }
                ProductionBodyFragmentSource::VectorBlock { block_index } => {
                    let binding = flow.blocks().unwrap().blocks()[block_index as usize].binding();
                    if binding.kind() == PrecomposedVectorKind::VectorFigure {
                        expected.push((
                            index,
                            None,
                            fragment,
                            role,
                            repeated,
                            BookV2ImageSource::Vector(binding),
                        ));
                    }
                }
                ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                } => {
                    for (inline, item) in flow.lines().paragraphs()[paragraph_index as usize]
                        .lines()[line_index as usize]
                        .items()
                        .iter()
                        .enumerate()
                    {
                        if let ProductionPlacedInline::Vector(selected) = item {
                            let binding = prepared
                                .vector_bindings()
                                .unwrap()
                                .receipt(selected.occurrence().item().node_id())
                                .unwrap();
                            if binding.kind() == PrecomposedVectorKind::InlineVector {
                                expected.push((
                                    index,
                                    Some(inline as u32),
                                    fragment,
                                    role,
                                    repeated,
                                    BookV2ImageSource::Vector(binding),
                                ));
                            }
                        }
                    }
                }
                _ => (),
            }
            index += 1;
        }
    }
    assert_eq!(display.draws().len(), expected.len());
    for (draw, (index, inline, fragment, role, repeated, source)) in
        display.draws().iter().zip(expected)
    {
        assert_eq!(draw.fragment_index(), index);
        assert_eq!(draw.inline_index(), inline);
        assert_eq!(draw.cell_role(), role);
        assert_eq!(draw.repeated_header(), repeated);
        assert!(std::ptr::eq(draw.fragment(), fragment));
        assert_eq!(draw.source().alternative(), source.alternative());
        assert_eq!(draw.source().owner(), source.owner());
        let (id, vector) = match (draw.source(), source) {
            (BookV2ImageSource::Figure(actual), BookV2ImageSource::Figure(f)) => {
                assert!(std::ptr::eq(actual, f));
                assert_eq!(draw.paint().viewport().width(), f.width());
                assert_eq!(draw.paint().viewport().height(), f.height());
                let vector = match f.media() {
                    ProductionFigureMedia::Raster {
                        pixel_width,
                        pixel_height,
                    } => {
                        assert!(matches!(draw.paint(), BookV2ImagePaint::Raster { .. }));
                        assert_eq!(
                            (draw.image().width().get(), draw.image().height().get()),
                            (pixel_width, pixel_height)
                        );
                        assert!(draw.image().admitted_safe_vector().is_none());
                        None
                    }
                    ProductionFigureMedia::Svg {
                        content_key,
                        scale_raw,
                    } => Some((content_key, scale_raw, [0, 0, 0])),
                };
                assert_eq!(draw.image().content_hash(), f.admitted_sha256());
                (f.image_id(), vector)
            }
            (BookV2ImageSource::Vector(actual), BookV2ImageSource::Vector(v)) => {
                assert!(std::ptr::eq(actual, v));
                let (scale, paint) = match v.placement() {
                    PrecomposedVectorPlacementInput::Inline(p) => {
                        (p.scale().get().raw(), p.paint())
                    }
                    PrecomposedVectorPlacementInput::VectorFigure(p) => {
                        (p.scale().get().raw(), p.paint())
                    }
                    _ => panic!("formula received ordinary-image paint"),
                };
                assert!(display
                    .source()
                    .terminals()
                    .iter()
                    .all(|t| t.source().owner() != v.node_id()));
                (
                    v.resource().image_id(),
                    Some((
                        VectorContentKey::from_admitted(draw.image()).unwrap(),
                        scale,
                        [paint.red(), paint.green(), paint.blue()],
                    )),
                )
            }
            _ => panic!("image source changed"),
        };
        assert!(std::ptr::eq(
            draw.image(),
            display.admitted().image(id).unwrap()
        ));
        let bounds = fragment.fragment().bounds();
        let viewport = draw.paint().viewport();
        if let Some(inline) = inline {
            let ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } = fragment.fragment().source()
            else {
                panic!()
            };
            let ProductionPlacedInline::Vector(selected) =
                flow.lines().paragraphs()[paragraph_index as usize].lines()[line_index as usize]
                    .items()[inline as usize]
            else {
                panic!()
            };
            let local = selected.geometry().viewport();
            assert_eq!(viewport.x().raw(), bounds.x().raw() + local.x().raw());
            assert_eq!(viewport.y().raw(), bounds.y().raw() + local.y().raw());
            assert_eq!(
                (viewport.width(), viewport.height()),
                (local.width(), local.height())
            );
        } else {
            assert_eq!(Some(viewport), fragment.fragment().viewport());
        }
        if let Some((key, scale, rgb)) = vector {
            let BookV2ImagePaint::Vector {
                content_key,
                scale_raw,
                matrix,
                color,
                ..
            } = draw.paint()
            else {
                panic!()
            };
            assert_eq!((content_key, scale_raw, color), (key, scale, rgb));
            assert_eq!(
                (
                    matrix.a.raw(),
                    matrix.b.raw(),
                    matrix.c.raw(),
                    matrix.d.raw()
                ),
                (scale, 0, 0, scale)
            );
            assert_eq!((matrix.e, matrix.f), (viewport.x(), viewport.y()));
        }
    }
}
fn check_anchors(display: &BookV2AnchorDisplay<'_, '_, '_, '_, '_, '_, '_, '_>) {
    let source = display.source().source();
    let lines = source.flow().lines();
    let expected_unpositioned = lines
        .paragraphs()
        .iter()
        .flat_map(|p| p.anchors())
        .filter(|a| a.position().is_none())
        .collect::<Vec<_>>();
    assert_eq!(display.unpositioned().len(), expected_unpositioned.len());
    for (actual, original) in display.unpositioned().iter().zip(expected_unpositioned) {
        assert!(std::ptr::eq(*actual, original));
    }
    let mut expected = Vec::new();
    let mut expected_empty = Vec::new();
    let mut index = 0;
    for page in source.geometry().pages() {
        for (placed, role, repeated) in page.fragments_with_roles() {
            if let ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } = placed.fragment().source()
            {
                if lines.paragraphs()[paragraph_index as usize].lines()[line_index as usize]
                    .items()
                    .iter()
                    .all(|item| matches!(item, ProductionPlacedInline::Break(_)))
                {
                    expected_empty.push((index, placed, role, repeated));
                }
                // Full linear oracle, independent of production binary lookup.
                for anchor in lines.paragraphs()[paragraph_index as usize].anchors() {
                    if anchor
                        .position()
                        .is_some_and(|p| p.line_index() == line_index)
                    {
                        expected.push((index, anchor, placed, role, repeated));
                    }
                }
            }
            index += 1;
        }
    }
    assert_eq!(display.positions().len(), expected.len());
    assert_eq!(display.nonpainting_lines().len(), expected_empty.len());
    for (actual, (index, placed, role, repeated)) in
        display.nonpainting_lines().iter().zip(expected_empty)
    {
        assert!(std::ptr::eq(actual.fragment(), placed));
        assert_eq!(actual.fragment_index(), index);
        assert_eq!(actual.cell_role(), role);
        assert_eq!(actual.repeated_header(), repeated);
    }
    for (actual, (index, anchor, fragment, role, repeated)) in
        display.positions().iter().zip(expected)
    {
        assert!(std::ptr::eq(actual.anchor(), anchor));
        assert!(std::ptr::eq(actual.fragment(), fragment));
        assert_eq!(actual.fragment_index(), index);
        assert_eq!(actual.cell_role(), role);
        assert_eq!(actual.repeated_header(), repeated);
        let local = anchor.position().unwrap();
        assert_eq!(
            actual.x().raw(),
            fragment.fragment().bounds().x().raw() + local.x().raw()
        );
        assert_eq!(
            actual.baseline().raw(),
            fragment.fragment().bounds().y().raw() + local.baseline().raw()
        );
        assert_eq!(Some(actual.baseline()), fragment.fragment().baseline());
    }
}

pub(super) fn assert_image_anchor_display(
    source: &BookV2BodyMathTerminals<'_, '_, '_, '_, '_, '_, '_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
) {
    let run = |prior, maximum, work_before| -> Result<_, BookV2MathDisplayError> {
        let mut builder =
            BookV2MathDisplayBuilder::new(source, admitted, limits, maximum, prior, work_before)?;
        builder.build()?;
        builder.build_text()?;
        builder.build_equation_numbers()?;
        builder.build_markers()?;
        let images = builder.build_images()?;
        assert!(std::ptr::eq(images.source(), source));
        assert!(std::ptr::eq(images.admitted(), admitted));
        assert_eq!(images.record_charge(), builder.record_charge());
        assert_eq!(images.work_steps(), builder.work_steps());
        check_images(&images);
        let anchors = builder.build_anchors()?;
        assert!(std::ptr::eq(anchors.source(), source));
        assert_eq!(anchors.record_charge(), builder.record_charge());
        assert_eq!(anchors.work_steps(), builder.work_steps());
        check_anchors(&anchors);
        let again = builder.build_images()?;
        assert_eq!(again.fingerprint(), images.fingerprint());
        assert!(again.record_charge() > images.record_charge());
        let again = builder.build_anchors()?;
        assert_eq!(again.fingerprint(), anchors.fingerprint());
        assert!(again.record_charge() > anchors.record_charge());
        Ok((
            builder.record_charge(),
            builder.work_steps(),
            images.fingerprint(),
            anchors.fingerprint(),
        ))
    };
    let (records, work, image_fp, anchor_fp) = run(0, 10_000_000, 0).unwrap();
    // Locate the first post-reservation failure from observable accounting,
    // without copying the traversal's work formula into this oracle.
    for images in [true, false] {
        let project = |builder: &mut BookV2MathDisplayBuilder<'_, '_, '_, '_, '_, '_, '_, '_>| {
            if images {
                builder.build_images().map(|_| ())
            } else {
                builder.build_anchors().map(|_| ())
            }
        };
        let mut complete =
            BookV2MathDisplayBuilder::new(source, admitted, limits, work, 0, 0).unwrap();
        let before = complete.record_charge();
        project(&mut complete).unwrap();
        let reserved = complete.record_charge();
        assert!(reserved > before);
        let mut lower = source.work_steps();
        let mut upper = complete.work_steps() - 1;
        while lower < upper {
            let middle = lower + (upper - lower) / 2;
            let mut probe =
                BookV2MathDisplayBuilder::new(source, admitted, limits, middle, 0, 0).unwrap();
            assert!(matches!(
                project(&mut probe),
                Err(BookV2MathDisplayError::WorkLimit(_))
            ));
            if probe.record_charge() == reserved {
                upper = middle;
            } else {
                assert_eq!(probe.record_charge(), before);
                lower = middle + 1;
            }
        }
        let mut earliest =
            BookV2MathDisplayBuilder::new(source, admitted, limits, upper, 0, 0).unwrap();
        assert!(matches!(
            project(&mut earliest),
            Err(BookV2MathDisplayError::WorkLimit(_))
        ));
        assert_eq!(
            (earliest.record_charge(), earliest.work_steps()),
            (reserved, upper)
        );
        assert!(matches!(
            project(&mut earliest),
            Err(BookV2MathDisplayError::WorkLimit(_))
        ));
        assert_eq!(
            (earliest.record_charge(), earliest.work_steps()),
            (reserved, upper)
        );
    }
    let prior = limits.base().get().max_fragments - (records - source.record_charge());
    assert_eq!(
        run(prior, work, 0).unwrap(),
        (limits.base().get().max_fragments, work, image_fp, anchor_fp)
    );
    assert!(
        matches!(run(prior + 1, work, 0), Err(BookV2MathDisplayError::Display(e)) if e.kind == ProductionBodyDisplayErrorKind::RecordLimit)
    );
    assert!(matches!(
        run(0, work - 1, 0),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(
        run(0, work + 17, source.work_steps() + 17).unwrap(),
        (records, work + 17, image_fp, anchor_fp)
    );
    let mut reverse = BookV2MathDisplayBuilder::new(source, admitted, limits, work, 0, 0).unwrap();
    assert_eq!(reverse.build_anchors().unwrap().fingerprint(), anchor_fp);
    assert_eq!(reverse.build_images().unwrap().fingerprint(), image_fp);
    let mut failed =
        BookV2MathDisplayBuilder::new(source, admitted, limits, work - 1, 0, 0).unwrap();
    failed.build().unwrap();
    failed.build_text().unwrap();
    failed.build_equation_numbers().unwrap();
    failed.build_markers().unwrap();
    failed.build_images().unwrap();
    failed.build_anchors().unwrap();
    failed.build_images().unwrap();
    assert!(matches!(
        failed.build_anchors(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(
        (failed.record_charge(), failed.work_steps()),
        (records, work - 1)
    );
    assert!(matches!(
        failed.build_anchors(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(
        (failed.record_charge(), failed.work_steps()),
        (records, work - 1)
    );
}
