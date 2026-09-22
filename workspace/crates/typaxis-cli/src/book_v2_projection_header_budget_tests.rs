use super::*;
use typaxis_layout::book_v2::{BookV2BodyLineVariantSeed, BookV2RebuiltBodyLineVariants};
use typaxis_pagination::book_v2::{
    prepare_book_v2_table_header_catalog_counted, prepare_book_v2_table_header_variant_counted,
    BookV2TableHeaderVariant, BookV2TableMeasurements,
};
use typaxis_pagination::{ProductionBodyPaginationError, ProductionBodyPaginationErrorKind as P};

type Outcome = Result<(u64, u64, [u8; 32]), ProductionBodyPaginationError>;
fn boundaries(
    initial: u64,
    maximum: u64,
    sampled: bool,
    legacy: impl Fn(u64, u64) -> Outcome,
    counted: impl Fn(u64, u64, &mut u64, &mut u64) -> Outcome,
) {
    let full = legacy(100_000_000, 0).unwrap();
    let (mut records, mut work) = (u64::MAX, u64::MAX);
    assert_eq!(counted(full.1, 0, &mut records, &mut work).unwrap(), full);
    assert_eq!((records, work), (full.0, full.1));
    assert!(full.1 > 1 && full.1 <= 100_000_000);
    let mut caps: Vec<_> = if sampled || full.1 > 2048 {
        (0..64).map(|i| (full.1 - 1) * i / 63).collect()
    } else {
        (0..full.1).collect()
    };
    caps.sort_unstable();
    caps.dedup();
    let mut last = (initial, 0);
    for cap in caps {
        let error = counted(cap, 0, &mut records, &mut work).unwrap_err();
        assert_eq!(error.kind, P::TableSearchLimit);
        assert_eq!(error, legacy(cap, 0).unwrap_err());
        assert!(records >= last.0 && records <= full.0);
        assert!(work >= last.1 && work <= cap);
        last = (records, work);
    }
    assert_eq!(last, (full.0, full.1 - 1));
    counted(0, last.0, &mut records, &mut work).unwrap_err();
    assert_eq!((records, work), (last.0, 0));
    let raised = maximum / 2;
    let additional = legacy(full.1, raised).unwrap().0 - raised;
    let prior = maximum - additional;
    assert_eq!(
        counted(full.1, prior, &mut records, &mut work).unwrap().0,
        maximum
    );
    assert_eq!((records, work), (maximum, full.1));
    let error = counted(full.1, prior + 1, &mut records, &mut work).unwrap_err();
    assert_eq!(error.kind, P::FragmentLimit);
    assert_eq!(error, legacy(full.1, prior + 1).unwrap_err());
    assert!(records >= prior + 1 && records <= maximum);
    assert!(work > 0 && work < full.1);
}

pub(super) fn projections<'b, 'f, 's, 'p, 'a>(
    set: &BookV2RebuiltBodyLineVariants<'_, '_, '_>,
    base: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
    variant: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
    entries: &[&'b BookV2TableHeaderVariant<'b, 'f, 's, 'p, 'a>],
    bad: &'b BookV2TableHeaderVariant<'b, 'f, 's, 'p, 'a>,
    limits: &'b M4EffectiveResourceLimits,
    sampled: bool,
) {
    boundaries(
        base.record_charge()
            .max(variant.record_charge())
            .max(set.record_charge()),
        limits.base().get().max_fragments,
        sampled,
        |cap, prior| {
            prepare_book_v2_table_header_variant(set, base, variant, 0, limits, cap, prior)
                .map(|h| (h.record_charge(), h.work_steps(), h.fingerprint()))
        },
        |cap, prior, records, work| {
            prepare_book_v2_table_header_variant_counted(
                set, base, variant, 0, limits, cap, prior, records, work,
            )
            .map(|h| (h.record_charge(), h.work_steps(), h.fingerprint()))
        },
    );
    boundaries(
        base.record_charge(),
        limits.base().get().max_fragments,
        sampled,
        |cap, prior| {
            prepare_book_v2_table_header_catalog(base, entries, limits, cap, prior)
                .map(|h| (h.record_charge(), h.work_steps(), h.fingerprint()))
        },
        |cap, prior, records, work| {
            prepare_book_v2_table_header_catalog_counted(
                base, entries, limits, cap, prior, records, work,
            )
            .map(|h| (h.record_charge(), h.work_steps(), h.fingerprint()))
        },
    );
    let (mut records, mut work) = (0, 0);
    let initial = base.record_charge();
    assert_eq!(
        prepare_book_v2_table_header_catalog_counted(
            base,
            &[],
            limits,
            0,
            0,
            &mut records,
            &mut work
        )
        .err()
        .unwrap()
        .kind,
        P::TableSearchLimit
    );
    assert_eq!((records, work), (initial, 0));
    assert!(prepare_book_v2_table_header_catalog_counted(
        base,
        &[],
        limits,
        1,
        0,
        &mut records,
        &mut work
    )
    .unwrap()
    .is_empty());
    assert_eq!((records, work), (initial + 1, 1));
    for (headers, expected) in [
        (vec![bad], P::WidthMismatch),
        (vec![entries[1], entries[0]], P::ReceiptMismatch),
        (vec![entries[0], entries[0]], P::ReceiptMismatch),
    ] {
        let error = prepare_book_v2_table_header_catalog_counted(
            base,
            &headers,
            limits,
            100_000_000,
            0,
            &mut records,
            &mut work,
        )
        .err()
        .unwrap();
        assert_eq!(error.kind, expected);
        assert_eq!(
            error,
            prepare_book_v2_table_header_catalog(base, &headers, limits, 100_000_000, 0)
                .err()
                .unwrap()
        );
        assert!(records > initial && work > 0);
    }
    let error = prepare_book_v2_table_header_variant_counted(
        set,
        base,
        variant,
        usize::MAX,
        limits,
        100_000_000,
        0,
        &mut records,
        &mut work,
    )
    .err()
    .unwrap();
    assert_eq!(error.kind, P::ReceiptMismatch);
    assert!(work > 0);
    assert_eq!(
        records,
        initial
            .max(variant.record_charge())
            .max(set.record_charge())
    );
}

pub(super) fn driver(
    seed: &BookV2BodyLineVariantSeed<'_>,
    limits: &M4EffectiveResourceLimits,
    width: PositiveLength,
    owner: typaxis_core::NodeId,
) {
    use crate::book_v2_resources::converged_pdf::header_catalog_driver::{
        with_header_catalog, HeaderCatalogBudget, HeaderWidthRequest,
    };
    use crate::book_v2_resources::BookV2ConvergenceError;
    let requests = [HeaderWidthRequest {
        table_index: 0,
        owner,
        parent_width: width,
    }];
    let mut full = HeaderCatalogBudget::default();
    with_header_catalog(seed, &requests, limits, 100_000_000, &mut full, |_, _| {
        Ok(())
    })
    .unwrap();
    let mut failed = HeaderCatalogBudget::default();
    let cap = full.work - 1;
    let result: Result<(), _> =
        with_header_catalog(seed, &requests, limits, cap, &mut failed, |_, _| {
            panic!("failed catalog reached consumer")
        });
    let BookV2ConvergenceError::Stage { stage, source } = result.unwrap_err() else {
        panic!("expected catalog stage")
    };
    assert_eq!(stage, "header catalog");
    assert_eq!(
        source
            .downcast_ref::<ProductionBodyPaginationError>()
            .unwrap()
            .kind,
        P::TableSearchLimit
    );
    assert_eq!(failed.records, full.records);
    assert_eq!(failed.work, cap);
    assert_eq!(failed.line_passes, full.line_passes);
    assert_eq!(failed.page_passes, 0);
    let retained = failed;
    assert!(with_header_catalog(seed, &requests, limits, cap, &mut failed, |_, _| Ok(())).is_err());
    assert_eq!(failed.work, retained.work);
    assert_eq!(failed.records, retained.records);
}
