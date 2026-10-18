use super::*;
#[path = "../../../typaxis-font/src/cff_sfnt_preflight_cases.rs"]
mod cases;
#[test]
fn observed_sfnt_preflight_all_source_axes_and_atomic_directory_retry() {
    let bytes = cases::cases()
        .into_iter()
        .find(|(label, _)| label == "length-4-reverse-false-gap-0-vertical-false")
        .unwrap()
        .1;
    let base = limits();
    let run = |owner, axis: usize, extra: u64| {
        let mut raw = base.base().get().clone();
        if axis == 1 {
            raw.max_spool_bytes = PRIOR[1] + extra;
        }
        let domain = ValidatedResourceLimits::new(raw).unwrap();
        let mut extension = *base.extension().get();
        extension.max_font_subset_bytes = extension
            .max_font_subset_bytes
            .min(domain.get().max_spool_bytes);
        let caps = M4EffectiveResourceLimits::new(domain.clone(), extension).unwrap();
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
                    &mut a.admission_budget(owner),
                )
            },
        );
        (result, [r, s, w])
    };
    let (ready, full) = run(NodeId::new(91), 3, 0);
    let native = ready.unwrap().unwrap_err();
    assert_eq!(
        native.kind,
        typaxis_font::Cff1FailureKindV2::Sfnt(typaxis_font::Cff1Error::InvalidHead)
    );
    let delta = [full[0] - PRIOR[0], full[1] - PRIOR[1], full[2] - PRIOR[2]];
    assert_eq!(delta[0], 18);
    assert_eq!(
        delta[1],
        18 * std::mem::size_of::<(usize, usize, usize, [u8; 4])>() as u64
    );
    let mut count = 0;
    for owner in [NodeId::new(91), NodeId::new(205)] {
        for axis in 0..3 {
            for extra in 0..=delta[axis] + 1 {
                let (result, cells) = run(owner, axis, extra);
                if extra >= delta[axis] {
                    assert_eq!(result.unwrap().unwrap_err(), native);
                    assert_eq!(cells, full);
                } else {
                    let typaxis_font::CffAdmissionBudgetFailureV2::Sfnt(f) = result.unwrap_err()
                    else {
                        panic!("no cmap phase exists in this preflight control")
                    };
                    assert_eq!(
                        f.error,
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
        let mut raw = base.base().get().clone();
        raw.max_spool_bytes = PRIOR[1] + delta[1] / 2 - 1;
        let domain = ValidatedResourceLimits::new(raw).unwrap();
        let mut ext = *base.extension().get();
        ext.max_font_subset_bytes = ext.max_font_subset_bytes.min(domain.get().max_spool_bytes);
        let caps = M4EffectiveResourceLimits::new(domain.clone(), ext).unwrap();
        let [mut r, mut s, mut w] = PRIOR;
        with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            &domain,
            MAXIMUM,
            MAXIMUM,
            |a| {
                for expected in [14, 15] {
                    let typaxis_font::CffAdmissionBudgetFailureV2::Sfnt(f) =
                        typaxis_font::admit_sfnt_cff1_v2_with_admission_budget(
                            std::sync::Arc::from(bytes.as_slice()),
                            0,
                            &caps,
                            &mut a.admission_budget(owner),
                        )
                        .unwrap_err()
                    else {
                        panic!("expected SFNT reserve refusal")
                    };
                    assert_eq!(f.error, BookV2ObservedAccountingError::Spool { owner });
                    assert_eq!(f.point.stage, typaxis_font::CffSfntStageV2::Directory);
                    assert_eq!(f.point.file_offset, Some(4));
                    assert_eq!(a.counters(), [7, 11, expected]);
                }
            },
        );
    }
    eprintln!("SFNT preflight Source caps={count}, delta={delta:?}, owners=91/205, atomic reserve/retry preserved; structural negative is not a font-admission positive");
}
