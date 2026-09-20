use super::*;

#[test]
fn cff_v2_execution_charges_growth_before_allocation_and_preserves_failed_work() {
    let mut p = program();
    let mut source = p.program.source.to_vec();
    // Three path operators consume operands; their shared fixed-capacity
    // stack is retained. Five outline commands grow the outline from 4 to 8.
    let root = append(&mut source, &[139, 139, 21, 32, 29, 32, 29, 32, 29, 14]);
    p.program.source = source.into();
    p.program.charstrings[0] = root;
    let limits = limits(M4ResourceLimits::default());
    let run = |maximum: (usize, usize, usize)| {
        let mut session = CffProgramEvaluationSessionV2::new(&limits);
        let mut spent = (0usize, 0usize, 0usize);
        let mut allocations = Vec::new();
        let result = session.evaluate_with_charge(&p, 0, 500, &mut |records, bytes, work| {
            let records_after = spent
                .0
                .checked_add(records)
                .filter(|n| *n <= maximum.0)
                .ok_or(Cff1Error::SubsetByteLimit)?;
            let bytes_after = spent
                .1
                .checked_add(bytes)
                .filter(|n| *n <= maximum.1)
                .ok_or(Cff1Error::SubsetByteLimit)?;
            spent.0 = records_after;
            spent.1 = bytes_after;
            if records > 0 {
                allocations.push((records, bytes));
            }
            for _ in 0..work {
                spent.2 = spent
                    .2
                    .checked_add(1)
                    .filter(|n| *n <= maximum.2)
                    .ok_or(Cff1Error::SubsetByteLimit)?;
            }
            Ok(())
        });
        (result, spent, session, allocations)
    };
    let (actual, spent, mut failed_session, allocations) =
        run((usize::MAX, usize::MAX, usize::MAX));
    let actual = actual.unwrap();
    assert_eq!(actual.commands().len(), 5);
    assert_eq!(allocations.len(), 3);
    assert_eq!(
        allocations[0].0,
        TYPE2_OPERAND_STACK_LIMIT + TYPE2_CALL_DEPTH_LIMIT + 1
    );
    assert_eq!(
        &allocations[1..],
        &[(4, 4 * std::mem::size_of::<OutlineSegment>()); 2]
    );
    let mut legacy = CffProgramEvaluationSessionV2::new(&limits);
    let expected = legacy.evaluate(&p, 0, 500).unwrap();
    assert_eq!(
        actual.commands().collect::<Vec<_>>(),
        expected.commands().collect::<Vec<_>>()
    );
    assert_eq!(actual.control_bounds(), expected.control_bounds());
    assert_eq!(
        actual.canonical_charstring().unwrap(),
        expected.canonical_charstring().unwrap()
    );
    assert!(run(spent).0.is_ok());
    for maximum in [
        (spent.0 - 1, spent.1, spent.2),
        (spent.0, spent.1 - 1, spent.2),
        (spent.0, spent.1, spent.2 - 1),
    ] {
        let (result, _, session, _) = run(maximum);
        assert_eq!(result.unwrap_err().kind, Cff1Error::SubsetByteLimit);
        assert!(session.operations_used() > 0);
    }
    let (result, refused, session, _) = run((spent.0, spent.1, spent.2 - 1));
    assert_eq!(result.unwrap_err().kind, Cff1Error::SubsetByteLimit);
    assert_eq!(refused, (spent.0, spent.1, spent.2 - 1));
    assert_eq!(session.operations_used(), legacy.operations_used());
    assert_eq!(
        session.outline_segments_used(),
        legacy.outline_segments_used()
    );
    // A subsequent allocation rejection does not erase the successful first
    // glyph's operation/segment counters or execute a second program.
    let before = (
        failed_session.operations_used(),
        failed_session.outline_segments_used(),
    );
    let error = failed_session
        .evaluate_with_charge(&p, 0, 500, &mut |_, _, _| Err(Cff1Error::SubsetByteLimit))
        .unwrap_err();
    assert_eq!(error.kind, Cff1Error::SubsetByteLimit);
    assert_eq!(
        (
            failed_session.operations_used(),
            failed_session.outline_segments_used()
        ),
        before
    );
}
fn limits(extension: M4ResourceLimits) -> M4EffectiveResourceLimits {
    M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .unwrap(),
        extension,
    )
    .unwrap()
}
fn append(source: &mut Vec<u8>, bytes: &[u8]) -> ProgramSpan {
    let start = source.len();
    source.extend(bytes);
    ProgramSpan {
        start,
        end: source.len(),
    }
}
// Unit fixture for the execution provider: each root calls the same global
// subroutine, which calls local subroutine 0 in that root glyph's FD.
fn program() -> CffProgramInspectionV2 {
    let mut source = Vec::new();
    let root = append(&mut source, &[139, 139, 21, 32, 29, 14]); // moveto, callgsubr -107, endchar
    let global = append(&mut source, &[32, 10, 11]); // callsubr -107, return
    let left = append(&mut source, &[149, 139, 5, 11]); // line by (10,0), return
    let right = append(&mut source, &[159, 139, 5, 11]); // line by (20,0), return
    let empty = ProgramSpan { start: 0, end: 0 };
    CffProgramInspectionV2 {
        program: CffProgramV2 {
            source: source.into(),
            charstrings: vec![root, root],
            global_subrs: vec![global],
            font_dicts: vec![
                CffFontDictV2 {
                    private_span: empty,
                    local_subrs: vec![left],
                    default_width_x: 500 * 65536,
                    nominal_width_x: 0,
                },
                CffFontDictV2 {
                    private_span: empty,
                    local_subrs: vec![right],
                    default_width_x: 600 * 65536,
                    nominal_width_x: 0,
                },
            ],
            fd_by_gid: vec![0, 1],
            cid_by_gid: vec![0, 10],
        },
    }
}
#[test]
fn cff_v2_execution_global_to_local_preserves_each_glyph_fd() {
    let p = program();
    let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
    for (gid, advance, distance) in [(0, 500, 10), (1, 600, 20)] {
        let glyph = session.evaluate(&p, gid, advance).unwrap();
        assert_eq!(glyph.fd(), gid as u8);
        assert_eq!(
            glyph.commands().collect::<Vec<_>>(),
            [
                CffOutlineCommandV2::Move(0, 0),
                CffOutlineCommandV2::Line(distance * 65536, 0),
                CffOutlineCommandV2::Close
            ]
        );
    }
}
#[test]
fn cff_v2_execution_preserves_distinct_postscript_and_opentype_widths() {
    let p = program();
    let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
    let glyph = session.evaluate(&p, 1, 500).unwrap();
    assert_eq!(glyph.advance(), 500);
    assert_eq!(glyph.source_width_fixed(), 600 * 65536);
}
#[test]
fn cff_v2_execution_invalid_width_is_located_at_the_actual_operator() {
    let mut p = program();
    p.program.font_dicts[1].nominal_width_x = i32::MAX;
    let mut source = p.program.source.to_vec();
    let root = append(&mut source, &[140, 139, 139, 21, 14]);
    p.program.source = source.into();
    p.program.charstrings[1] = root;
    let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
    let e = session.evaluate(&p, 1, 500).unwrap_err();
    assert_eq!(e.reason, CffGlyphFailureReasonV2::InvalidWidth);
    assert_eq!(
        (e.gid, e.fd, e.table_offset, e.operator),
        (1, Some(1), Some(root.start + 3), Some(21))
    );
}
#[test]
fn cff_v2_execution_work_budgets_are_inclusive_and_shared_across_glyphs() {
    let p = program();
    let mut measured = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
    measured.evaluate(&p, 0, 500).unwrap();
    measured.evaluate(&p, 1, 600).unwrap();
    for (operations, segments, succeeds) in [
        (
            measured.operations_used(),
            measured.outline_segments_used(),
            true,
        ),
        (
            measured.operations_used() - 1,
            measured.outline_segments_used(),
            false,
        ),
        (
            measured.operations_used(),
            measured.outline_segments_used() - 1,
            false,
        ),
    ] {
        let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits {
            max_cff_charstring_operations: operations,
            max_cff_outline_segments: segments,
            ..M4ResourceLimits::default()
        }));
        session.evaluate(&p, 0, 500).unwrap();
        assert_eq!(session.evaluate(&p, 1, 600).is_ok(), succeeds);
    }
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn cff_v2_execution_original_harano_all_glyphs() {
    let bytes = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hex = sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hex,
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let font = FontRef::new(&bytes).unwrap();
    let table = font
        .data_for_tag(read_fonts::types::Tag::new(b"CFF "))
        .unwrap();
    let p = inspect_cff1_program_v2(
        Arc::from(table.as_bytes()),
        font.maxp().unwrap().num_glyphs(),
        font.head().unwrap().units_per_em(),
        100_000,
    )
    .unwrap();
    let hmtx = font.hmtx().unwrap();
    let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
    let mut state = sha256(b"typaxis.cff-v2-outline-records/1");
    let mut widths = Vec::new();
    let mut mismatches = 0;
    for gid in 0..p.glyph_count() {
        let advance = hmtx.advance(GlyphId::new(gid as u32)).unwrap();
        let glyph = session.evaluate(&p, gid as u16, advance).unwrap();
        widths.extend(glyph.source_width_fixed().to_be_bytes());
        if i64::from(glyph.source_width_fixed()) != i64::from(advance) * 65536 {
            mismatches += 1;
        }
        let mut record = Vec::new();
        for command in glyph.commands() {
            let (symbol, values) = match command {
                CffOutlineCommandV2::Move(x, y) => (b'M', vec![x, y]),
                CffOutlineCommandV2::Line(x, y) => (b'L', vec![x, y]),
                CffOutlineCommandV2::Cubic(a, b, c, d, e, f) => (b'C', vec![a, b, c, d, e, f]),
                CffOutlineCommandV2::Close => (b'Z', vec![]),
            };
            record.push(symbol);
            for value in values {
                record.extend(value.to_be_bytes());
            }
        }
        let mut next = state.to_vec();
        next.extend((gid as u16).to_be_bytes());
        next.extend(advance.to_be_bytes());
        next.extend(sha256(&record));
        state = sha256(&next);
    }
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    assert_eq!(
        hex(&state),
        "f3a7b806eb38a37c56ec76eac5b80dd21a1bfe1f17a71650c971b3c399bace38"
    );
    assert_eq!(mismatches, 310);
    assert_eq!(
        hex(&sha256(&widths)),
        "feb4b1cdc05ab3b6ef6b3a8ead1167be85068ecac3cdfda03cc7f8d97f39113d"
    );
    eprintln!(
        "original Harano glyphs={} operations={} segments={}",
        p.glyph_count(),
        session.operations_used(),
        session.outline_segments_used()
    );
}

fn replace_root(p: &mut CffProgramInspectionV2, bytes: &[u8]) -> ProgramSpan {
    let mut source = p.program.source.to_vec();
    let span = append(&mut source, bytes);
    p.program.source = source.into();
    p.program.charstrings[0] = span;
    span
}
#[test]
fn cff_v2_execution_endchar_in_nested_subroutine_finishes_the_glyph() {
    let mut p = program();
    let mut source = p.program.source.to_vec();
    let root = append(&mut source, &[32, 29]);
    let global = append(&mut source, &[32, 10]);
    let local = append(&mut source, &[14]);
    p.program.source = source.into();
    p.program.charstrings[0] = root;
    p.program.global_subrs[0] = global;
    p.program.font_dicts[0].local_subrs[0] = local;
    let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
    let glyph = session.evaluate(&p, 0, 500).unwrap();
    assert_eq!(glyph.commands().len(), 0);
    assert_eq!(glyph.source_width_fixed(), 500 * 65536);
}
#[test]
fn cff_v2_execution_rejects_bad_calls_masks_stacks_and_locates_escape_operator() {
    for (bytes, position, operator) in [
        (vec![251, 0, 10, 14], 2, Some(10)),  // local index -1 after bias
        (vec![139, 140, 1, 19], 3, Some(19)), // truncated hintmask
        (vec![12, 255], 0, Some(0x0cff)),
        (vec![139; 49], 48, None),
    ] {
        let mut p = program();
        let span = replace_root(&mut p, &bytes);
        let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
        let e = session.evaluate(&p, 0, 500).unwrap_err();
        assert_eq!(
            (e.kind, e.fd, e.table_offset, e.operator),
            (
                Cff1Error::InvalidCharstring,
                Some(0),
                Some(span.start + position),
                operator
            )
        );
    }
}
#[test]
fn cff_v2_execution_diagnostics_distinguish_operands_escapes_and_program_ends() {
    for (bytes, offset, exact) in [
        (vec![], 0, false),
        (vec![139], 1, false),
        (vec![28], 0, true),
        (vec![255, 0, 0], 0, true),
        (vec![12], 0, true),
        (vec![139; 49], 48, true),
    ] {
        let mut p = program();
        let span = replace_root(&mut p, &bytes);
        let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
        let e = session.evaluate(&p, 0, 500).unwrap_err();
        assert_eq!(e.kind, Cff1Error::InvalidCharstring);
        assert_eq!(
            (e.table_offset, e.operator, e.position_is_exact),
            (Some(span.start + offset), None, exact)
        );
        assert_eq!((e.gid, e.fd), (0, Some(0)));
        assert!(e.font_context.is_none(), "inspection has no SFNT authority");
    }
    let mut p = program();
    let mut source = p.program.source.to_vec();
    let local = append(&mut source, &[149, 139, 5]); // valid line, missing return
    p.program.source = source.into();
    p.program.font_dicts[0].local_subrs[0] = local;
    let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
    let e = session.evaluate(&p, 0, 500).unwrap_err();
    assert_eq!(
        (e.table_offset, e.operator, e.position_is_exact),
        (Some(local.end), None, false)
    );
    assert_eq!((e.gid, e.fd), (0, Some(0)));
    assert!(session.operations_used() > 0);
    // The failed local cursor is not reused by a different FD or by preflight.
    session.evaluate(&p, 1, 600).unwrap();
    let e = session.evaluate(&p, 2, 600).unwrap_err();
    assert_eq!((e.fd, e.table_offset, e.operator), (None, None, None));
    let e = session
        .evaluate_with_charge(&p, 1, 600, &mut |_, _, _| Err(Cff1Error::SubsetByteLimit))
        .unwrap_err();
    assert_eq!(
        (e.table_offset, e.operator, e.position_is_exact),
        (None, None, false)
    );
}
#[test]
fn cff_v2_execution_recursive_global_subroutine_hits_call_depth_limit() {
    let mut p = program();
    let span = replace_root(&mut p, &[32, 29, 14]);
    p.program.global_subrs[0] = span;
    let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
    let e = session.evaluate(&p, 0, 500).unwrap_err();
    assert_eq!(e.kind, Cff1Error::InvalidCharstring);
    assert!(session.operations_used() < 100);
}

#[test]
fn cff_v2_execution_classifies_all_operator_encodings() {
    // Independent lists from Adobe #5177 Appendix A/C. Flex operators remain
    // implemented: their empty-stack failure is not an unsupported opcode.
    let known = [
        0, 3, 4, 5, 9, 10, 11, 12, 14, 15, 18, 20, 21, 22, 23, 24, 26, 27, 28, 29, 30,
    ];
    for escaped in 0..=255u8 {
        let mut p = program();
        let mut source = p.program.source.to_vec();
        let span = append(&mut source, &[12, escaped, 14]);
        p.program.source = source.into();
        p.program.charstrings[1] = span;
        let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
        let failure = session.evaluate(&p, 1, 500).unwrap_err();
        let expected = if known.contains(&escaped) {
            CffGlyphFailureReasonV2::UnsupportedOperator
        } else if [34, 35, 36, 37].contains(&escaped) {
            CffGlyphFailureReasonV2::Execution
        } else {
            CffGlyphFailureReasonV2::ReservedOperator
        };
        assert_eq!(failure.reason, expected, "escape {escaped}");
        assert_eq!(failure.kind, Cff1Error::InvalidCharstring);
        assert_eq!(
            (
                failure.gid,
                failure.fd,
                failure.table_offset,
                failure.operator
            ),
            (
                1,
                Some(1),
                Some(span.start),
                Some(0x0c00 | u16::from(escaped))
            )
        );
        assert!(failure.position_is_exact);
        assert!(failure.font_context.is_none());
        assert_eq!(
            (session.operations_used(), session.outline_segments_used()),
            (1, 0)
        );
    }
    for byte in [0, 2, 9, 13, 15, 16, 17] {
        let mut p = program();
        let mut source = p.program.source.to_vec();
        let span = append(&mut source, &[byte]);
        p.program.source = source.into();
        p.program.charstrings[0] = span;
        let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
        let failure = session.evaluate(&p, 0, 500).unwrap_err();
        assert_eq!(failure.reason, CffGlyphFailureReasonV2::ReservedOperator);
        assert_eq!(
            (failure.operator, failure.table_offset),
            (Some(u16::from(byte)), Some(span.start))
        );
    }
}

#[test]
fn cff_v2_execution_operator_rejection_preserves_fd_and_prior_errors() {
    let mut p = program();
    let mut source = p.program.source.to_vec();
    let span = append(&mut source, &[12, 10]); // unsupported add in FD 1's local subroutine
    p.program.source = source.into();
    p.program.font_dicts[1].local_subrs[0] = span;
    let defaults = limits(M4ResourceLimits::default());
    let mut session = CffProgramEvaluationSessionV2::new(&defaults);
    let error = session.evaluate(&p, 1, 500).unwrap_err();
    assert_eq!(error.reason, CffGlyphFailureReasonV2::UnsupportedOperator);
    assert_eq!(
        (error.fd, error.table_offset, error.operator),
        (Some(1), Some(span.start), Some(0x0c0a))
    );
    assert!(session.evaluate(&p, 0, 500).is_ok());

    let mut p = program();
    let mut source = p.program.source.to_vec();
    let span = append(&mut source, &[139, 12, 10]);
    p.program.source = source.into();
    p.program.charstrings[0] = span;
    let bounded = limits(M4ResourceLimits {
        max_cff_charstring_operations: 1,
        ..M4ResourceLimits::default()
    });
    let mut session = CffProgramEvaluationSessionV2::new(&bounded);
    let error = session.evaluate(&p, 0, 500).unwrap_err();
    assert_eq!(error.kind, Cff1Error::CharstringOperationLimit);
    assert_eq!(error.reason, CffGlyphFailureReasonV2::Execution);
    // The escape's second byte has not been decoded when charging fails.
    assert_eq!(error.operator, Some(12));
    assert_eq!(error.table_offset, Some(span.start + 1));
    let mut session = CffProgramEvaluationSessionV2::new(&defaults);
    let mut operations = 0;
    let error = session
        .evaluate_with_charge(&p, 0, 500, &mut |_, _, work| {
            if work == 64 {
                operations += 1;
            }
            if operations == 2 {
                Err(Cff1Error::InvalidCharstring)
            } else {
                Ok(())
            }
        })
        .unwrap_err();
    assert_eq!(error.reason, CffGlyphFailureReasonV2::Execution);
    assert_eq!(error.operator, Some(12));
    assert_eq!(session.operations_used(), 2);
}

#[test]
fn cff_v2_execution_hflex_rejects_unrepresentable_return_delta_without_panicking() {
    for dy in [i32::MIN, i32::MIN + 1, -65536, 0, 65536, i32::MAX] {
        let mut p = program();
        let mut source = p.program.source.to_vec();
        let mut bytes = vec![139, 139, 21, 139, 139, 255]; // move, dx1, dx2, dy2
        bytes.extend(dy.to_be_bytes());
        bytes.extend([139, 139, 139, 139, 12, 34, 14]);
        let span = append(&mut source, &bytes);
        p.program.source = source.into();
        p.program.charstrings[0] = span;
        let mut session = CffProgramEvaluationSessionV2::new(&limits(M4ResourceLimits::default()));
        let result = session.evaluate(&p, 0, 500);
        if dy == i32::MIN {
            let error = result.unwrap_err();
            assert_eq!(error.kind, Cff1Error::InvalidCharstring);
            assert_eq!(error.reason, CffGlyphFailureReasonV2::Execution);
            assert_eq!(
                (error.table_offset, error.operator),
                (Some(span.end - 3), Some(0x0c22))
            );
            assert_eq!(session.outline_segments_used(), 1); // the preceding move only
        } else {
            let glyph = result.unwrap();
            assert_eq!(glyph.commands().last(), Some(CffOutlineCommandV2::Close));
            let commands = glyph.commands().collect::<Vec<_>>();
            assert_eq!(commands.len(), 4);
            assert!(matches!(
                commands[2],
                CffOutlineCommandV2::Cubic(_, _, _, _, 0, 0)
            ));
        }
    }
}
