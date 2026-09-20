use super::*;

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn cff_v2_subset_detailed_limits_and_caller_errors_preserve_output_and_work() {
    let source: Arc<[u8]> = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap())
        .unwrap()
        .into();
    assert_eq!(
        sha256(&source)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let limits = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap();
    let admission = admit_sfnt_cff1_v2(source.clone(), 0, &limits).unwrap();
    let selected = "本文"
        .chars()
        .map(|c| OriginalGlyphId::new(admission.cmap().glyph_for_sequence(c, None).unwrap()))
        .collect();
    let close = |a: &Cff1AdmissionV2| {
        Cff1SubsetSessionV2::close_instance_selection(
            a,
            FontFaceId::new(7),
            FontInstanceId::new(9),
            &selected,
            65534,
        )
        .unwrap()
    };
    let closure = close(&admission);
    let mut session = Cff1SubsetSessionV2::from_admission(&admission);
    session.prepare_closure(&admission, &closure).unwrap();
    let work = (
        session.operations_used(),
        session.outline_segments_used(),
        session.cached_glyph_count(),
    );
    let mut charges = Vec::new();
    let expected = session
        .write_prepared_subset_detailed_with_charge(&admission, closure.clone(), &mut |r, b, w| {
            charges.push((r, b, w));
            Ok(())
        })
        .unwrap();
    let mut old_charges = Vec::new();
    let broad = session
        .write_prepared_subset_with_charge(&admission, closure.clone(), &mut |r, b, w| {
            old_charges.push((r, b, w));
            Ok(())
        })
        .unwrap();
    assert_eq!(charges, old_charges);
    assert_eq!(expected.bytes(), broad.bytes());
    assert_eq!(expected.fingerprint(), broad.fingerprint());
    assert_eq!(expected.canonical_jcs(), broad.canonical_jcs());
    let size = expected.bytes().len() as u64;
    let prefix = session
        .evaluated_glyph(&admission, OriginalGlyphId::new(0))
        .unwrap()
        .unwrap()
        .canonical_charstring_len()
        .unwrap() as u64;
    for limit in [1, size - 1, size, size + 1] {
        let mut extension = *limits.extension().get();
        extension.max_font_subset_bytes = limit;
        let capped = M4EffectiveResourceLimits::new(limits.base().clone(), extension).unwrap();
        let a = admit_sfnt_cff1_v2(source.clone(), 0, &capped).unwrap();
        let mut s = Cff1SubsetSessionV2::from_admission(&a);
        let closed = close(&a);
        s.prepare_closure(&a, &closed).unwrap();
        let result =
            s.write_prepared_subset_detailed_with_charge(&a, closed.clone(), &mut |_, _, _| Ok(()));
        if limit >= size {
            assert_eq!(result.unwrap().bytes(), expected.bytes());
            continue;
        }
        let failure = result.unwrap_err();
        assert_eq!(failure.kind, Cff1Error::SubsetByteLimit);
        let c = failure.context;
        assert_eq!(
            (c.phase, c.reason, c.embedding),
            (
                FontFailurePhase::Subset,
                FontFailureReason::BudgetExceeded,
                FontEmbeddingStatus::Allowed(0)
            )
        );
        assert_eq!(
            (c.limit, c.observed),
            (Some(limit), Some(if limit == 1 { prefix } else { size }))
        );
        assert_eq!(
            c.subset_stage,
            Some(if limit == 1 {
                FontSubsetStage::CharstringSize
            } else {
                FontSubsetStage::SfntSize
            })
        );
        assert_eq!(c.gid, if limit == 1 { Some(0) } else { None });
        assert_eq!(
            c.fd,
            if limit == 1 {
                a.program().fd_for_gid(0).map(u16::from)
            } else {
                None
            }
        );
        assert_eq!(
            (c.table_tag, c.file_offset, c.table_offset, c.operator),
            (None, None, None, None)
        );
        assert_eq!(
            s.write_prepared_subset_with_charge(&a, closed, &mut |_, _, _| Ok(()))
                .unwrap_err(),
            failure.kind
        );
        assert_eq!(
            (
                s.operations_used(),
                s.outline_segments_used(),
                s.cached_glyph_count()
            ),
            work
        );
    }
    // Real callback rejections at both pre-allocation and late receipt stages
    // must not be reinterpreted as the internal per-font byte cap.
    for (at, stage) in [
        (0, FontSubsetStage::GlyphStorage),
        (1, FontSubsetStage::GlyphLookup),
        (2, FontSubsetStage::CharstringSize),
        (3, FontSubsetStage::Charstring),
        (charges.len() - 2, FontSubsetStage::SfntWrite),
        (charges.len() - 1, FontSubsetStage::Receipt),
    ] {
        for kind in [
            Cff1Error::SubsetByteLimit,
            Cff1Error::OutlineSegmentLimit,
            Cff1Error::InvalidSubset,
        ] {
            let mut seen = Vec::new();
            let failure = session
                .write_prepared_subset_detailed_with_charge(
                    &admission,
                    closure.clone(),
                    &mut |r, b, w| {
                        let fail = seen.len() == at;
                        seen.push((r, b, w));
                        if fail {
                            Err(kind)
                        } else {
                            Ok(())
                        }
                    },
                )
                .unwrap_err();
            assert_eq!(failure.kind, kind);
            assert_eq!(seen, charges[..=at]);
            assert_eq!(failure.context.subset_stage, Some(stage));
            assert_eq!(
                (failure.context.limit, failure.context.observed),
                (None, None)
            );
            assert_eq!(
                (failure.context.file_offset, failure.context.table_offset),
                (None, None)
            );
        }
    }
    assert_eq!(
        (
            session.operations_used(),
            session.outline_segments_used(),
            session.cached_glyph_count()
        ),
        work
    );
    let empty = Cff1SubsetSessionV2::from_admission(&admission);
    let missing = empty
        .write_prepared_subset_detailed_with_charge(&admission, closure.clone(), &mut |_, _, _| {
            Ok(())
        })
        .unwrap_err();
    assert_eq!(missing.kind, Cff1Error::InvalidGlyphClosure);
    assert_eq!(
        missing.context.subset_stage,
        Some(FontSubsetStage::GlyphLookup)
    );
    assert_eq!(missing.context.reason, FontFailureReason::Invariant);
    let other_limits = M4EffectiveResourceLimits::new(
        limits.base().clone(),
        M4ResourceLimits {
            max_font_subset_bytes: size,
            ..M4ResourceLimits::default()
        },
    )
    .unwrap();
    let other = Cff1SubsetSessionV2::new(&other_limits);
    let mismatch = other
        .write_prepared_subset_detailed_with_charge(&admission, closure, &mut |_, _, _| {
            panic!("mismatched closure reached charge")
        })
        .unwrap_err();
    assert_eq!(mismatch.kind, Cff1Error::ReceiptMismatch);
    assert_eq!(mismatch.context.subset_stage, None);
    assert_eq!(
        (
            mismatch.context.gid,
            mismatch.context.fd,
            mismatch.context.limit,
            mismatch.context.observed
        ),
        (None, None, None, None)
    );
}
