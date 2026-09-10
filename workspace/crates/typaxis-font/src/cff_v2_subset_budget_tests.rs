use super::*;

pub(super) fn verify(
    session: &Cff1SubsetSessionV2,
    admission: &Cff1AdmissionV2,
    closure: &Cff1GlyphClosureV2,
    expected: &Cff1SubsetV2,
) {
    let run = |maximum: (usize, usize, usize)| {
        let mut spent = (0usize, 0usize, 0usize);
        let result = session.write_prepared_subset_with_charge(
            admission,
            closure.clone(),
            &mut |records, bytes, work| {
                let records = spent
                    .0
                    .checked_add(records)
                    .filter(|n| *n <= maximum.0)
                    .ok_or(Cff1Error::SubsetByteLimit)?;
                let bytes = spent
                    .1
                    .checked_add(bytes)
                    .filter(|n| *n <= maximum.1)
                    .ok_or(Cff1Error::SubsetByteLimit)?;
                spent.0 = records;
                spent.1 = bytes;
                let available = maximum.2 - spent.2;
                if work > available {
                    spent.2 = maximum.2;
                    return Err(Cff1Error::SubsetByteLimit);
                }
                spent.2 += work;
                Ok(())
            },
        );
        (result, spent)
    };
    let before = (
        session.operations_used(),
        session.outline_segments_used(),
        session.cached_glyph_count(),
    );
    let (written, spent) = run((usize::MAX, usize::MAX, usize::MAX));
    let written = written.unwrap();
    assert_eq!(written.bytes(), expected.bytes());
    assert_eq!(written.canonical_jcs(), expected.canonical_jcs());
    assert_eq!(written.fingerprint(), expected.fingerprint());
    assert!(run(spent).0.is_ok());
    for maximum in [
        (spent.0 - 1, spent.1, spent.2),
        (spent.0, spent.1 - 1, spent.2),
        (spent.0, spent.1, spent.2 - 1),
    ] {
        assert_eq!(run(maximum).0.unwrap_err(), Cff1Error::SubsetByteLimit);
    }
    assert_eq!(
        run((spent.0, spent.1, spent.2 - 1)).1,
        (spent.0, spent.1, spent.2 - 1)
    );
    assert_eq!(
        (
            session.operations_used(),
            session.outline_segments_used(),
            session.cached_glyph_count()
        ),
        before
    );
    let empty = Cff1SubsetSessionV2::from_admission(admission);
    assert_eq!(
        empty
            .write_prepared_subset_with_charge(admission, closure.clone(), &mut |_, _, _| Ok(()))
            .unwrap_err(),
        Cff1Error::InvalidGlyphClosure
    );
    assert_eq!(
        (
            empty.operations_used(),
            empty.outline_segments_used(),
            empty.cached_glyph_count()
        ),
        (0, 0, 0)
    );
    // Actual font-byte cap: the exact complete SFNT size succeeds; one short
    // refuses before allocating its final SFNT buffer. Earlier table buffers
    // are still charged and no evaluation is hidden in the writer.
    for (byte_limit, succeeds) in [
        (expected.bytes().len(), true),
        (expected.bytes().len() - 1, false),
    ] {
        let mut extension = *admission.effective_limits().extension().get();
        extension.max_font_subset_bytes = byte_limit as u64;
        let limits =
            M4EffectiveResourceLimits::new(admission.effective_limits().base().clone(), extension)
                .unwrap();
        let source = admit_sfnt_cff1_v2(Arc::from(admission.source()), 0, &limits).unwrap();
        let selected = closure.source_gids().iter().copied().collect();
        let closed = Cff1SubsetSessionV2::close_instance_selection(
            &source,
            closure.font_face_id(),
            closure.font_instance_id(),
            &selected,
            65534,
        )
        .unwrap();
        let mut session = Cff1SubsetSessionV2::from_admission(&source);
        session.prepare_closure(&source, &closed).unwrap();
        let mut final_buffer = 0;
        let result =
            session.write_prepared_subset_with_charge(&source, closed, &mut |records, bytes, _| {
                if records == 1 && bytes == expected.bytes().len() {
                    final_buffer += 1;
                }
                Ok(())
            });
        if succeeds {
            assert_eq!(result.unwrap().bytes(), expected.bytes());
            assert_eq!(final_buffer, 1);
        } else {
            assert_eq!(result.unwrap_err(), Cff1Error::SubsetByteLimit);
            assert_eq!(final_buffer, 0);
        }
    }
}
