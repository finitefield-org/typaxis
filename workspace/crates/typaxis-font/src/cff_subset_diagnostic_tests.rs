use super::*;

fn source_position(bytes: &[u8], token: &[u8]) -> usize {
    let offsets: Vec<_> = bytes
        .windows(token.len())
        .enumerate()
        .filter_map(|(i, b)| (b == token).then_some(i))
        .collect();
    assert_eq!(offsets.len(), 1, "test program must be unique in the SFNT");
    offsets[0]
}

#[test]
fn legacy_cff_detailed_glyph_failures_use_original_sfnt_positions() {
    let original = fixture();
    let base = admit_sfnt_cff1(&original, 0, &limits()).unwrap();
    let root = &base.program.charstrings[0];
    let root_at = source_position(&original, root);
    let (_, cff_base, _) = table_location(&original, b"CFF ");
    for mode in 0..6 {
        let mut bytes = original.clone();
        let program = &mut bytes[root_at..root_at + root.len()];
        program.fill(139);
        let (position, operator, reason, exact) = match mode {
            0 => {
                program[0] = 0;
                (0, Some(0), FontFailureReason::ReservedCffOperator, true)
            }
            1 => {
                program[..2].copy_from_slice(&[12, 10]);
                (
                    0,
                    Some(0x0c0a),
                    FontFailureReason::UnsupportedCffOperator,
                    true,
                )
            }
            2 => {
                program[0] = 5;
                (0, Some(5), FontFailureReason::InvalidCharstring, true)
            }
            3 => (
                root.len(),
                None,
                FontFailureReason::InvalidCharstring,
                false,
            ),
            4 => {
                *program.last_mut().unwrap() = 28;
                (
                    root.len() - 1,
                    None,
                    FontFailureReason::InvalidCharstring,
                    true,
                )
            }
            _ => {
                *program.last_mut().unwrap() = 12;
                (
                    root.len() - 1,
                    None,
                    FontFailureReason::InvalidCharstring,
                    true,
                )
            }
        };
        recompute_sfnt_checksums(&mut bytes);
        let admission = admit_sfnt_cff1_detailed(&bytes, 0, &limits()).unwrap();
        let mut session = Cff1SubsetSession::from_admission(&admission);
        let error = session
            .subset_detailed(
                &admission,
                FontFaceId::new(7),
                FontInstanceId::new(9),
                &BTreeSet::new(),
                10,
            )
            .unwrap_err();
        assert_eq!(error.kind, Cff1Error::InvalidCharstring);
        let c = error.context;
        assert_eq!(
            (c.phase, c.reason, c.table_tag),
            (FontFailurePhase::Charstring, reason, Some(*b"CFF "))
        );
        assert_eq!(
            (c.file_offset, c.table_offset),
            (
                Some((root_at + position) as u64),
                Some((root_at + position - cff_base) as u64)
            )
        );
        assert_eq!((c.gid, c.fd, c.operator), (Some(0), None, operator));
        assert_eq!((c.position_is_exact, c.position_is_end), (exact, !exact));
        assert_eq!(c.embedding, FontEmbeddingStatus::Allowed(0));
        assert_eq!((c.limit, c.observed), (None, None));
        assert!(session.evaluated.is_empty());
        let spent = session.operations_used();
        assert!(spent > 0);
        // Existing broad callers retain the same error; failures are retried
        // rather than cached, and the aggregate work is not reset.
        assert_eq!(
            session.subset(
                &admission,
                FontFaceId::new(7),
                FontInstanceId::new(9),
                &BTreeSet::new(),
                10
            ),
            Err(error.kind)
        );
        assert_eq!(session.operations_used(), spent * 2);
    }
}

#[test]
fn legacy_cff_detailed_budgets_and_selection_do_not_invent_source_fields() {
    let bytes = fixture();
    for operations in [true, false] {
        let mut extension = M4ResourceLimits::default();
        if operations {
            extension.max_cff_charstring_operations = 1;
        } else {
            extension.max_cff_outline_segments = 1;
        }
        let admission = admit_sfnt_cff1(&bytes, 0, &limits_with(extension)).unwrap();
        let start = source_position(&bytes, &admission.program.charstrings[0]);
        let mut session = Cff1SubsetSession::from_admission(&admission);
        let error = session
            .prepare_face_detailed(&admission, FontFaceId::new(1), &BTreeSet::new())
            .unwrap_err();
        assert_eq!(error.context.reason, FontFailureReason::BudgetExceeded);
        assert_eq!(
            (error.context.limit, error.context.observed),
            (Some(1), Some(2))
        );
        assert_eq!(
            error.context.file_offset,
            Some((start + if operations { 2 } else { 10 }) as u64)
        );
        assert_eq!(
            error.context.operator,
            if operations { None } else { Some(6) }
        );
        assert!(error.context.position_is_exact);
    }
    let admission = admit_sfnt_cff1(&bytes, 0, &limits()).unwrap();
    let mut session = Cff1SubsetSession::from_admission(&admission);
    let selected = [OriginalGlyphId::new(999)].into_iter().collect();
    let error = session
        .prepare_face_detailed(&admission, FontFaceId::new(1), &selected)
        .unwrap_err();
    assert_eq!(error.context.gid, Some(999));
    assert_eq!(error.context.file_offset, None);
    assert_eq!(error.context.operator, None);
    let error = session
        .subset_detailed(
            &admission,
            FontFaceId::new(1),
            FontInstanceId::new(1),
            &selected,
            10,
        )
        .unwrap_err();
    assert_eq!(error.context.phase, FontFailurePhase::Subset);
    assert_eq!(error.context.table_tag, None);
    assert_eq!(error.context.file_offset, None);
    assert_eq!(error.context.gid, None);
}

#[test]
fn legacy_cff_detailed_local_and_global_subroutines_keep_original_positions() {
    let hex = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/cff-media/diagnostics/typaxis-cff-subr-diagnostic-fixture.otf.hex"));
    let digits: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    let original: Vec<_> = digits
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect();
    let base = admit_sfnt_cff1(&original, 0, &limits()).unwrap();
    let selected = [OriginalGlyphId::new(1)].into_iter().collect();
    let mut good = Cff1SubsetSession::from_admission(&base);
    assert!(good
        .subset_detailed(
            &base,
            FontFaceId::new(1),
            FontInstanceId::new(1),
            &selected,
            10
        )
        .is_ok());
    for global in [false, true] {
        let program = if global {
            &base.program.global_subrs[0]
        } else {
            &base.program.local_subrs[0]
        };
        let start = source_position(&original, program);
        for end in [false, true] {
            let mut bytes = original.clone();
            if end {
                bytes[start..start + program.len()].fill(139);
            } else {
                bytes[start] = 0;
            }
            recompute_sfnt_checksums(&mut bytes);
            let admission = admit_sfnt_cff1(&bytes, 0, &limits()).unwrap();
            let mut session = Cff1SubsetSession::from_admission(&admission);
            let error = session
                .prepare_face_detailed(&admission, FontFaceId::new(1), &selected)
                .unwrap_err();
            assert_eq!(error.context.gid, Some(1));
            assert_eq!(error.context.fd, None); // name-keyed /1 has no FDSelect
            assert_eq!(
                error.context.file_offset,
                Some((start + if end { program.len() } else { 0 }) as u64)
            );
            assert_eq!(error.context.position_is_end, end);
            assert_eq!(error.context.operator, if end { None } else { Some(0) });
            assert_eq!(
                error.context.reason,
                if end {
                    FontFailureReason::InvalidCharstring
                } else {
                    FontFailureReason::ReservedCffOperator
                }
            );
            assert_eq!(session.evaluated.len(), 1); // successful .notdef remains cached
        }
    }
}

#[test]
fn legacy_cff_subset_byte_diagnostic_matches_actual_output_and_exact_boundary() {
    let bytes = fixture();
    let selected = [OriginalGlyphId::new(1), OriginalGlyphId::new(2)]
        .into_iter()
        .collect();
    let admission = admit_sfnt_cff1(&bytes, 0, &limits()).unwrap();
    let baseline = Cff1SubsetSession::from_admission(&admission)
        .subset_detailed(
            &admission,
            FontFaceId::new(1),
            FontInstanceId::new(1),
            &selected,
            10,
        )
        .unwrap();
    let length = baseline.bytes().len() as u64;
    for maximum in [1, length - 1, length, length + 1] {
        let extension = M4ResourceLimits {
            max_font_subset_bytes: maximum,
            ..M4ResourceLimits::default()
        };
        let admission = admit_sfnt_cff1(&bytes, 0, &limits_with(extension)).unwrap();
        let mut session = Cff1SubsetSession::from_admission(&admission);
        let result = session.subset_detailed(
            &admission,
            FontFaceId::new(1),
            FontInstanceId::new(1),
            &selected,
            10,
        );
        if maximum >= length {
            assert_eq!(result.unwrap(), baseline);
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.kind, Cff1Error::SubsetByteLimit);
            let c = error.context;
            assert_eq!(c.subset_stage, Some(FontSubsetStage::SfntSize));
            assert_eq!((c.limit, c.observed), (Some(maximum), Some(length)));
            assert_eq!(
                (c.table_tag, c.file_offset, c.table_offset, c.gid),
                (None, None, None, None)
            );
            assert_eq!(c.reason, FontFailureReason::BudgetExceeded);
            let work = (session.operations_used(), session.outline_segments_used());
            let again = session.subset_detailed(
                &admission,
                FontFaceId::new(1),
                FontInstanceId::new(1),
                &selected,
                10,
            );
            assert_eq!(again.unwrap_err(), error);
            assert_eq!(
                (session.operations_used(), session.outline_segments_used()),
                work
            );
        }
    }
}

#[test]
fn legacy_cff_subset_stages_do_not_blame_unobserved_source_bytes_or_budgets() {
    let original = fixture();
    let admission = admit_sfnt_cff1(&original, 0, &limits()).unwrap();
    let selected = [OriginalGlyphId::new(1)].into_iter().collect();
    for (instance, selection, stage) in [
        (26u32.pow(6), &selected, FontSubsetStage::Name),
        (1, &BTreeSet::new(), FontSubsetStage::Cmap),
    ] {
        let error = Cff1SubsetSession::from_admission(&admission)
            .subset_detailed(
                &admission,
                FontFaceId::new(1),
                FontInstanceId::new(instance),
                selection,
                10,
            )
            .unwrap_err();
        assert_eq!(error.kind, Cff1Error::InvalidSubset);
        assert_eq!(error.context.subset_stage, Some(stage));
        assert_eq!((error.context.limit, error.context.observed), (None, None));
        assert_eq!(
            (
                error.context.table_tag,
                error.context.file_offset,
                error.context.gid
            ),
            (None, None, None)
        );
    }
    // A valid source Type2 contour reaches x=32767.5, which cannot be rounded
    // outward into the subset's signed 16-bit bounding box. Admission and glyph
    // evaluation succeed; output encoding is the first failing stage.
    let mut bytes = original.clone();
    let start = source_position(&bytes, &admission.program.charstrings[0]);
    bytes[start..start + 12]
        .copy_from_slice(&[248, 236, 255, 127, 255, 128, 0, 22, 138, 139, 5, 14]);
    recompute_sfnt_checksums(&mut bytes);
    let admitted = admit_sfnt_cff1(&bytes, 0, &limits()).unwrap();
    let mut session = Cff1SubsetSession::from_admission(&admitted);
    session
        .prepare_face_detailed(&admitted, FontFaceId::new(1), &selected)
        .unwrap();
    let error = session
        .subset_detailed(
            &admitted,
            FontFaceId::new(1),
            FontInstanceId::new(1),
            &selected,
            10,
        )
        .unwrap_err();
    assert_eq!(error.kind, Cff1Error::InvalidSubset);
    assert_eq!(
        error.context.subset_stage,
        Some(FontSubsetStage::GlyphBounds)
    );
    assert_eq!(error.context.gid, Some(0));
    assert_eq!(
        (
            error.context.table_tag,
            error.context.file_offset,
            error.context.table_offset
        ),
        (None, None, None)
    );
    assert_eq!((error.context.limit, error.context.observed), (None, None));
    assert!(!error.context_note().contains("cff_operator="));
}
