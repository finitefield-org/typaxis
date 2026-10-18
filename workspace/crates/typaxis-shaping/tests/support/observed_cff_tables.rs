use super::*;
#[path = "../../../typaxis-font/src/cff_font_tables_cases.rs"]
pub(super) mod cases;
#[test]
fn observed_font_tables_source_axes_preserve_owners_native_diagnostics_and_prefixes() {
    let bytes = cases::replace(
        &cases::standard(),
        *b"VORG",
        Some(vec![0, 1, 0, 0, 0, 0, 0, 1, 0, 2, 0, 0]),
    );
    // Exercise the common fixture generator so unused helpers do not hide drift.
    assert!(!cases::vertical_cases().is_empty() && !cases::cases().is_empty());
    let base = limits();
    let expected =
        typaxis_font::admit_sfnt_cff1_v2(std::sync::Arc::from(bytes.as_slice()), 0, &base)
            .unwrap_err();
    assert!(matches!(
        expected.kind,
        typaxis_font::Cff1FailureKindV2::Program(_)
    ));
    let run = |owner, axis: usize, extra: u64| {
        let mut raw = base.base().get().clone();
        if axis == 1 {
            raw.max_spool_bytes = PRIOR[1] + extra;
        }
        let domain = ValidatedResourceLimits::new(raw).unwrap();
        let mut ext = *base.extension().get();
        ext.max_font_subset_bytes = ext.max_font_subset_bytes.min(domain.get().max_spool_bytes);
        let caps = M4EffectiveResourceLimits::new(domain.clone(), ext).unwrap();
        let [mut r, mut s, mut w] = PRIOR;
        let result = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            &domain,
            if axis == 0 { PRIOR[0] + extra } else { MAXIMUM },
            if axis == 2 { PRIOR[2] + extra } else { MAXIMUM },
            |a| {
                typaxis_font::admit_sfnt_cff1_v2_with_admission_budget(
                    std::sync::Arc::from(bytes.as_slice()),
                    0,
                    &caps,
                    &mut a.font_tables_budget(owner),
                )
            },
        );
        (result, [r, s, w])
    };
    let (ready, full) = run(NodeId::new(91), 3, 0);
    assert_eq!(ready.unwrap().unwrap_err(), expected);
    let delta = [full[0] - PRIOR[0], full[1] - PRIOR[1], full[2] - PRIOR[2]];
    let mut count = 0;
    let mut metadata_refusals = 0;
    let mut table_refusals = 0;
    for owner in [NodeId::new(91), NodeId::new(205)] {
        for axis in 0..3 {
            for extra in 0..=delta[axis] + 1 {
                let (result, cells) = run(owner, axis, extra);
                if extra >= delta[axis] {
                    assert_eq!(result.unwrap().unwrap_err(), expected);
                    assert_eq!(cells, full);
                } else {
                    let cause = match result.unwrap_err() {
                        typaxis_font::CffAdmissionBudgetFailureV2::Sfnt(f) => f.error,
                        typaxis_font::CffAdmissionBudgetFailureV2::Cmap(
                            typaxis_font::CffCmapBudgetFailureV2::Base(f),
                        ) => f.error,
                        typaxis_font::CffAdmissionBudgetFailureV2::Cmap(
                            typaxis_font::CffCmapBudgetFailureV2::Variation(f),
                        ) => f.error,
                        typaxis_font::CffAdmissionBudgetFailureV2::Program(f) => f.error,
                        typaxis_font::CffAdmissionBudgetFailureV2::Tables(f) => {
                            table_refusals += 1;
                            if let Some(offset) = f.point.table_offset {
                                let tag = f.point.table_tag.unwrap();
                                let count = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
                                let record = bytes[12..12 + count * 16]
                                    .chunks_exact(16)
                                    .find(|r| r[..4] == tag)
                                    .unwrap();
                                let base = u64::from(u32::from_be_bytes(
                                    record[8..12].try_into().unwrap(),
                                ));
                                assert_eq!(f.point.file_offset, Some(base + offset));
                            }
                            f.error
                        }
                        typaxis_font::CffAdmissionBudgetFailureV2::Metadata(f) => {
                            metadata_refusals += 1;
                            let tag = f.point.table_tag.unwrap();
                            let tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
                            let record = bytes[12..12 + tables * 16]
                                .chunks_exact(16)
                                .find(|r| r[..4] == tag)
                                .unwrap();
                            let base =
                                u64::from(u32::from_be_bytes(record[8..12].try_into().unwrap()));
                            assert_eq!(f.point.file_offset, f.point.table_offset.map(|n| base + n));
                            f.error
                        }
                    };
                    assert_eq!(
                        cause,
                        match axis {
                            0 => BookV2ObservedAccountingError::Records { owner },
                            1 => BookV2ObservedAccountingError::Spool { owner },
                            _ => BookV2ObservedAccountingError::Work { owner },
                        }
                    );
                    assert!(cells[axis] <= PRIOR[axis] + extra);
                }
                count += 1;
            }
        }
    }
    assert!(metadata_refusals > 0 && table_refusals > 0);
    eprintln!("font tables Source cap cases={count}, metadata refusals={metadata_refusals}, table refusals={table_refusals}, owners=91/205, delta={delta:?}, structural negative retained");
}

#[test]
fn observed_font_tables_spool_refusal_is_atomic_and_same_cells_keep_the_prefix() {
    let bytes = cases::standard();
    let mut raw = limits().base().get().clone();
    raw.max_spool_bytes = PRIOR[1] + 576 + 7;
    let domain = ValidatedResourceLimits::new(raw).unwrap();
    let ext = M4ResourceLimits {
        max_font_subset_bytes: domain.get().max_spool_bytes,
        ..M4ResourceLimits::default()
    };
    let caps = M4EffectiveResourceLimits::new(domain.clone(), ext).unwrap();
    let [mut r, mut s, mut w] = PRIOR;
    let owner = NodeId::new(91);
    with_book_v2_observed_source_accounting(
        &mut r,
        &mut s,
        &mut w,
        &domain,
        MAXIMUM,
        MAXIMUM,
        |a| {
            let failure = typaxis_font::admit_sfnt_cff1_v2_with_admission_budget(
                std::sync::Arc::from(bytes.as_slice()),
                0,
                &caps,
                &mut a.font_tables_budget(owner),
            )
            .unwrap_err();
            let typaxis_font::CffAdmissionBudgetFailureV2::Metadata(f) = failure else {
                panic!("must refuse the first hmtx reservation")
            };
            assert_eq!(
                f.point.stage,
                typaxis_font::CffMetadataStageV2::HorizontalHeader
            );
            assert_eq!(f.point.table_tag, Some(*b"hmtx"));
            assert_eq!(f.error, BookV2ObservedAccountingError::Spool { owner });
            let first = a.counters();
            assert_eq!(first[..2], [PRIOR[0] + 18, PRIOR[1] + 576]);
            let error = typaxis_font::admit_sfnt_cff1_v2_with_admission_budget(
                std::sync::Arc::from(bytes.as_slice()),
                0,
                &caps,
                &mut a.font_tables_budget(owner),
            )
            .unwrap_err();
            let typaxis_font::CffAdmissionBudgetFailureV2::Sfnt(f) = error else {
                panic!("retry must use the same exhausted spool cells")
            };
            assert_eq!(f.error, BookV2ObservedAccountingError::Spool { owner });
            assert_eq!(a.counters()[..2], first[..2]);
            assert!(a.counters()[2] > first[2]);
        },
    );
}
