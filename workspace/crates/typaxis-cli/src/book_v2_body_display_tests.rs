use super::*;
use typaxis_display_list::book_v2::{BookV2BodyDisplay, BookV2BodyPaintIndex as P};
#[path = "book_v2_font_uses_tests.rs"]
mod font_uses;
#[path = "book_v2_font_selection_tests.rs"]
mod font_selection;

fn check(display: &BookV2BodyDisplay<'_, '_, '_, '_, '_, '_, '_, '_>) {
    font_uses::check(display);
    let source = display.source();
    for component in [
        display.math().source(),
        display.text().source(),
        display.numbers().source(),
        display.markers().source(),
        display.images().source(),
        display.anchors().source(),
    ] {
        assert!(std::ptr::eq(component, source));
    }
    for component in [
        display.math().admitted(),
        display.text().admitted(),
        display.numbers().admitted(),
        display.markers().admitted(),
        display.images().admitted(),
    ] {
        assert!(std::ptr::eq(component, display.admitted()));
    }
    // Independent physical-fragment oracle: scan each owned component and
    // reconstruct the authored inline stream in that particular fragment.
    let mut expected = Vec::new();
    let fragments = source
        .source()
        .geometry()
        .pages()
        .iter()
        .flat_map(|p| p.fragments())
        .count();
    for fragment in 0..fragments {
        for (i, separator) in display.markers().separators().iter().enumerate() {
            if separator.before_fragment_index() == fragment {
                expected.push(P::FootnoteSeparator(i));
            }
        }
        for (i, marker) in display.markers().draws().iter().enumerate() {
            if marker.fragment_index() == fragment {
                expected.push(P::Marker(i));
            }
        }
        let mut inline = Vec::new();
        for (i, text) in display.text().draws().iter().enumerate() {
            if text.fragment_index() == fragment {
                inline.push((text.inline_index(), P::Text(i)));
            }
        }
        for (i, math) in display.math().draws().iter().enumerate() {
            if math.terminal().fragment_index() == fragment {
                inline.push((math.terminal().inline_index().unwrap_or(0), P::Math(i)));
            }
        }
        for (i, image) in display.images().draws().iter().enumerate() {
            if image.fragment_index() == fragment {
                inline.push((image.inline_index().unwrap_or(0), P::Image(i)));
            }
        }
        inline.sort_by_key(|(position, _)| *position);
        assert!(inline.windows(2).all(|w| w[0].0 < w[1].0));
        expected.extend(inline.into_iter().map(|(_, paint)| paint));
        for (i, number) in display.numbers().draws().iter().enumerate() {
            if number.placement().geometry().fragment_index() as usize == fragment {
                let Some(P::Math(parent)) = expected.last() else {
                    panic!("independent number must follow its formula")
                };
                assert_eq!(
                    display.math().draws()[*parent].terminal().fragment_index(),
                    fragment
                );
                expected.push(P::EquationNumber(i));
            }
        }
    }
    assert_eq!(display.paints(), expected);
    assert_eq!(
        display.paints().len(),
        display.math().draws().len()
            + display.text().draws().len()
            + display.numbers().draws().len()
            + display.markers().draws().len()
            + display.markers().separators().len()
            + display.images().draws().len()
    );
    assert_eq!(display.math().draws().len(), source.terminals().len());
}

pub(crate) fn assert_body_display(
    source: &BookV2BodyMathTerminals<'_, '_, '_, '_, '_, '_, '_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
) {
    let maximum = source.work_steps().checked_add(10_000_000).unwrap();
    let run = |prior, maximum, work_before| -> Result<_, BookV2MathDisplayError> {
        let mut builder =
            BookV2MathDisplayBuilder::new(source, admitted, limits, maximum, prior, work_before)?;
        let display = builder.build_body()?;
        display.verify_resources(admitted, limits)?;
        assert!(std::ptr::eq(display.source(), source));
        assert!(std::ptr::eq(display.admitted(), admitted));
        assert_eq!(display.record_charge(), builder.record_charge());
        assert_eq!(display.work_steps(), builder.work_steps());
        check(&display);
        let again = builder.build_body()?;
        assert_eq!(again.fingerprint(), display.fingerprint());
        assert_eq!(again.paints(), display.paints());
        assert!(again.record_charge() > display.record_charge());
        assert!(again.work_steps() > display.work_steps());
        Ok((
            builder.record_charge(),
            builder.work_steps(),
            display.fingerprint(),
        ))
    };
    let (records, work, fp) = run(0, maximum, 0).unwrap();
    let mut font_builder = BookV2MathDisplayBuilder::new(source, admitted, limits, maximum, 0, 0).unwrap();
    let font_display = font_builder.build_body().unwrap();
    font_selection::check(&font_display, limits);
    let prior = limits.base().get().max_fragments - (records - source.record_charge());
    assert_eq!(
        run(prior, work, 0).unwrap(),
        (limits.base().get().max_fragments, work, fp)
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
        (records, work + 17, fp)
    );
    let mut failed =
        BookV2MathDisplayBuilder::new(source, admitted, limits, work - 1, 0, 0).unwrap();
    failed.build_body().unwrap();
    assert!(matches!(
        failed.build_body(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert_eq!(
        (failed.record_charge(), failed.work_steps()),
        (records, work - 1)
    );
    assert!(matches!(
        failed.build_body(),
        Err(BookV2MathDisplayError::WorkLimit(_))
    ));
    assert!(failed.record_charge() >= records);
    assert_eq!(failed.work_steps(), work - 1);
}
