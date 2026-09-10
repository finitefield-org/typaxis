use super::*;
use typaxis_pdf::book_v2::{BookV2PdfPipeline as Pipeline, BookV2PdfPipelineError as P};
use typaxis_resources::book_v2::BookV2FontSelectionError as S;

pub(super) fn check(
    display: &typaxis_display_list::book_v2::BookV2BodyDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    first: u32,
    limits: &M4EffectiveResourceLimits,
    expected: &[u8],
) {
    // Fund verification after the input display's already consumed work.
    let work_limit = display.work_steps().checked_add(100_000_000).unwrap();
    let run = |records, spool, output, work, prior_work| -> Result<_, P> {
        let mut pipeline = Pipeline::new(
            display, first, limits, work, records, spool, output, prior_work,
        )?;
        let mut called = false;
        let bytes = pipeline.with_pdf(|pdf| {
            called = true;
            assert_eq!(pdf.bytes(), expected);
            pdf.bytes().len()
        })?;
        assert!(called);
        assert!(pipeline.output_charge() >= bytes as u64);
        Ok((
            pipeline.record_charge(),
            pipeline.spool_charge(),
            pipeline.output_charge(),
            pipeline.work_steps(),
        ))
    };
    let full = run(0, 0, 0, work_limit, 0).unwrap();
    let base = limits.base().get();
    let r = base.max_fragments - (full.0 - display.record_charge());
    let s = base.max_spool_bytes - (full.1 - display.source().spool_charge());
    let o = base.max_output_bytes - full.2;
    assert_eq!(
        run(r, s, o, full.3, 0).unwrap(),
        (
            base.max_fragments,
            base.max_spool_bytes,
            base.max_output_bytes,
            full.3
        )
    );
    assert!(run(r + 1, 0, 0, work_limit, 0).is_err());
    assert!(run(0, s + 1, 0, work_limit, 0).is_err());
    assert!(run(0, 0, o + 1, work_limit, 0).is_err());
    let prior = run(0, 0, 0, work_limit, display.work_steps() + 17).unwrap();
    assert_eq!(prior.3, full.3 + 17);
    let mut pipeline = Pipeline::new(display, first, limits, full.3 - 1, 0, 0, 0, 0).unwrap();
    assert!(matches!(
        pipeline.with_pdf(|_| panic!("incomplete pipeline reached callback")),
        Err(P::Assembly(AE::Resource(PE::Work)))
    ));
    let retained = (
        pipeline.record_charge(),
        pipeline.spool_charge(),
        pipeline.output_charge(),
        pipeline.work_steps(),
    );
    assert_eq!(retained.3, full.3 - 1);
    assert!(retained.0 > display.record_charge());
    assert!(retained.1 > display.source().spool_charge());
    assert!(retained.2 > 0);
    assert!(matches!(
        pipeline.with_pdf(|_| panic!("failed pipeline reached callback")),
        Err(P::Selection(S::Work))
    ));
    assert!(pipeline.record_charge() >= retained.0);
    assert!(pipeline.spool_charge() >= retained.1);
    assert_eq!(pipeline.output_charge(), retained.2);
    assert_eq!(pipeline.work_steps(), retained.3);
    let mut changed = base.clone();
    changed.max_output_bytes -= 1;
    let changed = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(changed).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    assert!(matches!(
        Pipeline::new(display, first, &changed, work_limit, 0, 0, 0, 0),
        Err(P::Pdf(PE::Identity))
    ));
}
