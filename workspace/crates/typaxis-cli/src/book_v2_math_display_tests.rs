use typaxis_core::{M4EffectiveResourceLimits, M4ResourceLimits, ValidatedResourceLimits};
use typaxis_display_list::book_v2::{
    BookV2MathDisplayBuilder, BookV2MathDisplayError, BookV2MathPaint,
};
use typaxis_display_list::{ProductionBodyDisplayErrorKind, ProductionNativeMathPaint};
use typaxis_pagination::book_v2::{BookV2BodyMathSource, BookV2BodyMathTerminals};
use typaxis_resources::{AdmittedProductionResourceLedgerV3, VectorContentKey};

#[path = "book_v2_text_display_tests.rs"]
mod text_display;
#[path = "book_v2_number_display_tests.rs"]
pub(super) mod number_display;
#[path = "book_v2_marker_display_tests.rs"]
mod marker_display;
#[path = "book_v2_image_anchor_display_tests.rs"]
mod image_anchor_display;
#[path = "book_v2_body_display_tests.rs"]
mod body_display;
pub(super) use body_display::assert_body_display as assert_book_v2_body_resources;

pub(super) fn assert_math_display(
    source: &BookV2BodyMathTerminals<'_, '_, '_, '_, '_, '_, '_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
) -> [u8; 32] {
    let run = |prior, maximum_work| -> Result<(u64, u64, [u8; 32]), BookV2MathDisplayError> {
        let mut builder =
            BookV2MathDisplayBuilder::new(source, admitted, limits, maximum_work, prior, 0)?;
        let first = builder.build()?;
        assert!(std::ptr::eq(first.source(), source));
        assert!(std::ptr::eq(first.admitted(), admitted));
        assert_eq!(first.draws().len(), source.terminals().len());
        for (draw, terminal) in first.draws().iter().zip(source.terminals()) {
            assert!(std::ptr::eq(draw.terminal(), terminal));
            match (draw.paint(), terminal.source()) {
                (BookV2MathPaint::Vector(paint), BookV2BodyMathSource::Vector(binding)) => {
                    let expected = VectorContentKey::from_admitted(
                        admitted.image(binding.resource().image_id()).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(paint.content_key(), expected);
                    assert_eq!(paint.viewport(), terminal.viewport().unwrap());
                    let (scale, color) = match binding.placement() {
                        typaxis_layout::PrecomposedVectorPlacementInput::Inline(p) => {
                            (p.scale().get().raw(), p.paint())
                        }
                        typaxis_layout::PrecomposedVectorPlacementInput::MathVectorBlock(p) => {
                            (p.scale().get().raw(), p.paint())
                        }
                        _ => panic!("ordinary figure received formula paint"),
                    };
                    assert_eq!(paint.scale_raw(), scale);
                    assert_eq!(paint.color(), [color.red(), color.green(), color.blue()]);
                    let matrix = paint.matrix();
                    assert_eq!(
                        (
                            matrix.a.raw(),
                            matrix.b.raw(),
                            matrix.c.raw(),
                            matrix.d.raw()
                        ),
                        (scale, 0, 0, scale)
                    );
                    assert_eq!(
                        (matrix.e, matrix.f),
                        (paint.viewport().x(), paint.viewport().y())
                    );
                }
                (BookV2MathPaint::Native(paint), BookV2BodyMathSource::Native(receipt)) => {
                    let (left, top, right, bottom) = receipt.computation().dimensions().bbox();
                    assert_eq!(paint.bounds().x().raw(), terminal.origin_x().raw() + left);
                    assert_eq!(paint.bounds().y().raw(), terminal.baseline().raw() + top);
                    assert_eq!(paint.bounds().width().get().raw(), right - left);
                    assert_eq!(paint.bounds().height().get().raw(), bottom - top);
                    assert_eq!(paint.paints().len(), receipt.computation().paints().len());
                    for (actual, original) in
                        paint.paints().iter().zip(receipt.computation().paints())
                    {
                        match (actual, original) {
                            (
                                ProductionNativeMathPaint::Glyph {
                                    original_gid,
                                    unicode,
                                    logical_ordinal,
                                    x,
                                    y,
                                    font_size,
                                },
                                typaxis_math::MathPaint::Glyph(g),
                            ) => {
                                assert_eq!(*original_gid, g.original_gid());
                                assert_eq!(*unicode, g.unicode());
                                assert_eq!(*logical_ordinal, g.logical_ordinal());
                                assert_eq!(x.raw(), terminal.origin_x().raw() + g.x());
                                assert_eq!(y.raw(), terminal.baseline().raw() + g.y());
                                assert_eq!(font_size.get().raw(), g.font_size_raw());
                            }
                            (
                                ProductionNativeMathPaint::Rule(rect),
                                typaxis_math::MathPaint::Rule(r),
                            ) => {
                                assert_eq!(rect.x().raw(), terminal.origin_x().raw() + r.x());
                                assert_eq!(rect.y().raw(), terminal.baseline().raw() + r.y());
                                assert_eq!(rect.width().get().raw(), r.width());
                                assert_eq!(rect.height().get().raw(), r.height());
                            }
                            _ => panic!("native paint kind/order changed"),
                        }
                    }
                }
                _ => panic!("formula source was replaced by another paint kind"),
            }
        }
        assert_eq!(first.record_charge(), builder.record_charge());
        assert_eq!(first.work_steps(), builder.work_steps());
        let second = builder.build()?;
        assert_eq!(second.fingerprint(), first.fingerprint());
        assert!(second.record_charge() > first.record_charge());
        assert!(second.work_steps() > first.work_steps());
        Ok((
            builder.record_charge(),
            builder.work_steps(),
            first.fingerprint(),
        ))
    };
    let (records, work, fp) = run(0, 10_000_000).unwrap();
    let mut prior_work = BookV2MathDisplayBuilder::new(
        source,
        admitted,
        limits,
        work + 17,
        0,
        source.work_steps() + 17,
    )
    .unwrap();
    prior_work.build().unwrap();
    assert_eq!(prior_work.build().unwrap().fingerprint(), fp);
    assert_eq!(prior_work.work_steps(), work + 17);
    let mut exhausted =
        BookV2MathDisplayBuilder::new(source, admitted, limits, work - 1, 0, 0).unwrap();
    exhausted.build().unwrap();
    let before = (exhausted.record_charge(), exhausted.work_steps());
    assert!(matches!(
        exhausted.build(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert!(exhausted.record_charge() >= before.0 && exhausted.work_steps() >= before.1);
    assert!(matches!(
        exhausted.build(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    let prior = limits.base().get().max_fragments - (records - source.record_charge());
    assert_eq!(
        run(prior, work).unwrap(),
        (limits.base().get().max_fragments, work, fp)
    );
    assert!(
        matches!(run(prior+1,work),Err(BookV2MathDisplayError::Display(e)) if e.kind==ProductionBodyDisplayErrorKind::RecordLimit)
    );
    assert!(matches!(
        run(0, work - 1),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert!(matches!(
        BookV2MathDisplayBuilder::new(source, admitted, limits, source.work_steps() - 1, 0, 0),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    let mut base = limits.base().get().clone();
    base.max_fragments -= 1;
    let different = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(base).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap();
    assert!(
        matches!(BookV2MathDisplayBuilder::new(source,admitted,&different,10_000_000,0,0),Err(BookV2MathDisplayError::Display(e)) if e.kind==ProductionBodyDisplayErrorKind::ReceiptMismatch)
    );
    let preflight = source.work_steps() + source.terminals().len() as u64;
    let mut early =
        BookV2MathDisplayBuilder::new(source, admitted, limits, preflight, 0, 0).unwrap();
    let required = 1 + source
        .terminals()
        .iter()
        .map(|t| {
            1 + match t.source() {
                BookV2BodyMathSource::Native(n) => n.computation().paints().len() as u64,
                _ => 0,
            }
        })
        .sum::<u64>();
    let expected_records = early.record_charge() + required;
    assert!(matches!(
        early.build(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(early.record_charge(), expected_records);
    assert_eq!(early.work_steps(), preflight);
    text_display::assert_text_display(source, admitted, limits);
    number_display::assert_number_display(source, admitted, limits);
    marker_display::assert_marker_display(source, admitted, limits);
    image_anchor_display::assert_image_anchor_display(source, admitted, limits);
    body_display::assert_body_display(source, admitted, limits);
    fp
}
