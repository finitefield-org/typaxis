use super::*;
use typaxis_layout::ProductionPlacedInline;
use typaxis_pagination::ProductionBodyFragmentSource;
use typaxis_shaping::ShapeSourceSpan;

pub(super) fn assert_text_display(
    source: &BookV2BodyMathTerminals<'_, '_, '_, '_, '_, '_, '_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
) {
    let run = |prior,
               maximum_work,
               work_before|
     -> Result<(u64, u64, [u8; 32]), BookV2MathDisplayError> {
        let mut builder = BookV2MathDisplayBuilder::new(
            source,
            admitted,
            limits,
            maximum_work,
            prior,
            work_before,
        )?;
        let math = builder.build()?;
        let display = builder.build_text()?;
        assert!(std::ptr::eq(display.source(), source));
        assert!(std::ptr::eq(display.admitted(), admitted));
        assert!(display.record_charge() > math.record_charge());
        assert!(display.work_steps() > math.work_steps());
        assert_eq!(display.record_charge(), builder.record_charge());
        assert_eq!(display.work_steps(), builder.work_steps());
        let lines = source.source().flow().lines();
        let parsed = lines
            .prepared()
            .source_flow()
            .body()
            .body()
            .wire()
            .text_buffers()
            .len();
        let mut observed = display.draws().iter();
        let mut fragment_index = 0;
        for page in source.source().geometry().pages() {
            for (placed, role, repeated) in page.fragments_with_roles() {
                let fragment = placed.fragment();
                if let ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                } = fragment.source()
                {
                    let paragraph = &lines.paragraphs()[paragraph_index as usize];
                    for (inline, original) in paragraph.lines()[line_index as usize]
                        .items()
                        .iter()
                        .enumerate()
                    {
                        if let ProductionPlacedInline::Text(cluster) = original {
                            let draw = observed.next().expect("selected source cluster was lost");
                            assert!(std::ptr::eq(draw.cluster(), cluster));
                            let font = paragraph.font().unwrap();
                            assert!(std::ptr::eq(draw.font(), font));
                            let face = admitted.font(font.face_id()).unwrap();
                            assert_eq!(font.content_hash(), face.content_hash());
                            assert_eq!(font.face_index(), face.face_index());
                            assert_eq!(draw.owner(), cluster.run().owner());
                            assert_eq!(draw.page_index(), fragment.page_index());
                            assert_eq!(draw.fragment_index(), fragment_index);
                            assert_eq!(draw.inline_index(), inline as u32);
                            assert_eq!(draw.definition_index(), placed.definition_index());
                            assert_eq!(draw.item_index(), placed.item_index());
                            assert_eq!(draw.cell_owner(), role.map(|r| r.owner()));
                            assert_eq!(draw.repeated_header(), repeated);
                            assert_eq!(draw.exact_text(), cluster.utf8());
                            let (id, start, end) = match cluster.source_span() {
                                ShapeSourceSpan::Parsed(span) => {
                                    assert_eq!(draw.generated_provenance(), None);
                                    (span.text_id().get(), span.start_byte(), span.end_byte())
                                }
                                ShapeSourceSpan::Generated(p) => {
                                    assert_eq!(draw.generated_provenance(), Some(p));
                                    let span = p.text_span();
                                    (
                                        parsed as u32 + span.text_id().get(),
                                        span.range().start_byte(),
                                        span.range().end_byte(),
                                    )
                                }
                            };
                            assert_eq!(draw.text_span().text_id().get(), id);
                            assert_eq!(draw.text_span().range().start_byte(), start);
                            assert_eq!(draw.text_span().range().end_byte(), end);
                            assert_eq!(draw.glyphs().len(), cluster.glyphs().len());
                            for (paint, glyph) in draw.glyphs().iter().zip(cluster.glyphs()) {
                                assert_eq!(paint.original_gid(), glyph.glyph().original_gid);
                                assert_eq!(
                                    paint.x().raw(),
                                    fragment.bounds().x().raw() + glyph.x().raw()
                                );
                                assert_eq!(
                                    paint.y().raw(),
                                    fragment.bounds().y().raw() + glyph.y().raw()
                                );
                            }
                            let width: i64 = cluster
                                .glyphs()
                                .iter()
                                .map(|g| g.glyph().advance_x.raw())
                                .sum();
                            let height = font.ascender().raw() - font.descender().raw();
                            if width > 0 && height > 0 {
                                let bounds = draw.logical_bounds().unwrap();
                                assert_eq!(
                                    bounds.x().raw(),
                                    fragment.bounds().x().raw() + cluster.pen_x().raw()
                                );
                                assert_eq!(
                                    bounds.y().raw(),
                                    fragment.baseline().unwrap().raw() - font.ascender().raw()
                                );
                                assert_eq!(bounds.width().get().raw(), width);
                                assert_eq!(bounds.height().get().raw(), height);
                            } else {
                                assert_eq!(draw.logical_bounds(), None);
                            }
                        }
                    }
                }
                fragment_index += 1;
            }
        }
        assert!(
            observed.next().is_none(),
            "a non-text item received a text draw"
        );
        let second = builder.build_text()?;
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
    assert!(matches!(
        exhausted.build_text(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    let spent = (exhausted.record_charge(), exhausted.work_steps());
    assert_eq!(spent, (records, work - 1));
    assert!(matches!(
        exhausted.build_text(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert!(exhausted.record_charge() >= spent.0 && exhausted.work_steps() >= spent.1);
    // Component order affects accounting only, never actual source/paint facts.
    let mut reverse = BookV2MathDisplayBuilder::new(source, admitted, limits, work, 0, 0).unwrap();
    let first_text = reverse.build_text().unwrap();
    assert_eq!(first_text.fingerprint(), fp);
    // Stop immediately after allocation, before the first draw is projected.
    // All reserved draw/glyph slots must still remain charged.
    let lines = source.source().flow().lines();
    let mut preflight = source.work_steps();
    for page in source.source().geometry().pages() {
        preflight += 1;
        for placed in page.fragments() {
            preflight += 1;
            if let ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } = placed.fragment().source()
            {
                preflight += lines.paragraphs()[paragraph_index as usize].lines()
                    [line_index as usize]
                    .items()
                    .len() as u64;
            }
        }
    }
    let mut early =
        BookV2MathDisplayBuilder::new(source, admitted, limits, preflight, 0, 0).unwrap();
    assert!(matches!(
        early.build_text(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(early.record_charge(), first_text.record_charge());
    assert_eq!(early.work_steps(), preflight);
    assert!(matches!(
        early.build_text(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(early.record_charge(), first_text.record_charge());
    assert_eq!(early.work_steps(), preflight);
    reverse.build().unwrap();
    assert_eq!(reverse.build_text().unwrap().fingerprint(), fp);
    assert_eq!(
        (reverse.record_charge(), reverse.work_steps()),
        (records, work)
    );
}
