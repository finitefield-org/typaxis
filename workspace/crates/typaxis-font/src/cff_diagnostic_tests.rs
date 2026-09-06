use super::*;

fn failure(bytes: &[u8]) -> Cff1Failure {
    let detailed = admit_sfnt_cff1_detailed(bytes, 0, &limits()).unwrap_err();
    assert_eq!(admit_sfnt_cff1(bytes, 0, &limits()), Err(detailed.kind));
    detailed
}

#[test]
fn detailed_admission_preserves_receipt_and_legacy_failures() {
    let bytes = fixture();
    let legacy = admit_sfnt_cff1(&bytes, 0, &limits()).unwrap();
    let detailed = admit_sfnt_cff1_detailed(&bytes, 0, &limits()).unwrap();
    assert_eq!(legacy, detailed);
    for end in 0..bytes.len() {
        failure(&bytes[..end]);
    }
    let f = admit_sfnt_cff1_detailed(&bytes, 3, &limits()).unwrap_err();
    assert_eq!(f.kind, Cff1Error::InvalidFaceIndex);
    assert_eq!(f.context.requested_face_index, 3);
    assert_eq!(f.context.phase, FontFailurePhase::FaceSelection);
    assert_eq!(f.context.embedding, FontEmbeddingStatus::NotChecked);
}

#[test]
fn unsupported_vorg_is_separate_from_embedding_permission() {
    let bytes = with_optional_tables(vec![RewriteTable {
        tag: *b"VORG",
        bytes: vec![0, 1, 0, 0, 0, 0, 0, 0],
    }]);
    let f = failure(&bytes);
    assert_eq!(f.kind, Cff1Error::UnsupportedTable);
    assert_eq!(f.context.reason, FontFailureReason::UnsupportedTable);
    assert_eq!(f.context.table_tag, Some(*b"VORG"));
    assert_eq!(f.context.embedding, FontEmbeddingStatus::Allowed(0));
    let (record, _, _) = table_location(&bytes, b"VORG");
    assert_eq!(f.context.file_offset, Some(record as u64));
    assert_eq!(f.context.table_offset, None);
    assert!(f.context.position_is_exact);
    let note = f.context_note();
    assert!(note.contains("class=unsupported"));
    assert!(note.contains("table=VORG"));
    assert!(note.contains("embedding=allowed; fs_type=0x0000"));

    let bytes = with_fs_type(2);
    let f = failure(&bytes);
    let (_, offset, _) = table_location(&bytes, b"OS/2");
    assert_eq!(f.context.reason, FontFailureReason::RestrictedEmbedding);
    assert_eq!(f.context.phase, FontFailurePhase::EmbeddingPermission);
    assert_eq!(f.context.table_tag, Some(*b"OS/2"));
    assert_eq!(f.context.embedding, FontEmbeddingStatus::Denied(2));
    assert_eq!(f.context.file_offset, Some((offset + 8) as u64));
    assert_eq!(f.context.table_offset, Some(8));
    assert!(f.context.position_is_exact);
}

#[test]
fn invalid_checksum_never_becomes_permission_failure() {
    let mut bytes = with_fs_type(2);
    let (record, offset, _) = table_location(&bytes, b"OS/2");
    bytes[offset + 8] ^= 1;
    let f = failure(&bytes);
    assert_eq!(f.context.reason, FontFailureReason::ChecksumMismatch);
    assert_eq!(f.context.table_tag, Some(*b"OS/2"));
    assert_eq!(f.context.file_offset, Some((record + 4) as u64));
    assert_eq!(f.context.embedding, FontEmbeddingStatus::NotChecked);
}

#[test]
fn font_budget_details_preserve_kind_limit_and_observation() {
    let bytes = fixture();
    for (extension, kind, tag, limit, observed) in [
        (
            M4ResourceLimits {
                max_font_tables: 8,
                ..M4ResourceLimits::default()
            },
            Cff1Error::TableLimit,
            None,
            8,
            9,
        ),
        (
            M4ResourceLimits {
                max_font_glyphs: 3,
                ..M4ResourceLimits::default()
            },
            Cff1Error::GlyphLimit,
            Some(*b"maxp"),
            3,
            4,
        ),
    ] {
        let limits = limits_with(extension);
        let f = admit_sfnt_cff1_detailed(&bytes, 0, &limits).unwrap_err();
        assert_eq!(admit_sfnt_cff1(&bytes, 0, &limits), Err(kind));
        assert_eq!(f.kind, kind);
        assert_eq!(f.context.reason, FontFailureReason::BudgetExceeded);
        assert_eq!(f.context.table_tag, tag);
        assert_eq!(f.context.limit, Some(limit));
        assert_eq!(f.context.observed, Some(observed));
    }
}

#[test]
fn cmap_and_optional_table_failures_keep_their_owner() {
    let mut bytes = fixture();
    let (_, cmap, _) = table_location(&bytes, b"cmap");
    let offset = read_u32(&bytes, cmap + 8, Cff1Error::InvalidCmap).unwrap() as usize;
    bytes[cmap + offset..cmap + offset + 2].copy_from_slice(&14u16.to_be_bytes());
    recompute_sfnt_checksums(&mut bytes);
    let f = failure(&bytes);
    assert_eq!(f.context.table_tag, Some(*b"cmap"));
    assert_eq!(f.context.reason, FontFailureReason::UnsupportedCmapFormat);
    assert_eq!(f.context.cmap_format, Some(14));
    assert_eq!(f.context.table_offset, Some(offset as u64));
    assert!(f.context.position_is_exact);

    let bytes = with_optional_tables(vec![RewriteTable {
        tag: *b"GDEF",
        bytes: vec![0, 1, 0, 0],
    }]);
    let f = failure(&bytes);
    assert_eq!(f.kind, Cff1Error::InvalidOptionalTable);
    assert_eq!(f.context.table_tag, Some(*b"GDEF"));
    assert_eq!(f.context.phase, FontFailurePhase::TableDecode);
    assert_eq!(f.context.file_offset, None);
}

#[test]
fn cff_operator_and_subroutine_budget_report_cff_positions() {
    let mut bytes = fixture();
    let (_, cff, length) = table_location(&bytes, b"CFF ");
    let table = &bytes[cff..cff + length];
    let names = parse_cff_index(table, 4, None).unwrap();
    let top = parse_cff_index(table, names.end, None).unwrap();
    let entries = parse_dict(&top.objects[0]).unwrap();
    let entry = entries.iter().find(|e| e.operator == 0x0C06).unwrap();
    let operator = top.data_start + entry.operator_offset;
    // Substitute unsupported ROS without moving the DICT or its offsets.
    bytes[cff + operator + 1] = 30;
    recompute_sfnt_checksums(&mut bytes);
    let f = failure(&bytes);
    assert_eq!(f.context.reason, FontFailureReason::UnsupportedCffOperator);
    assert_eq!(f.context.operator, Some(0x0C1E));
    assert_eq!(f.context.phase, FontFailurePhase::CffTopDict);
    assert_eq!(f.context.table_offset, Some(operator as u64));
    assert_eq!(f.context.file_offset, Some((cff + operator) as u64));
    assert!(f.context.position_is_exact);

    // Exercise the global INDEX budget before parsing objects or later offsets.
    // The shipped small font has no subroutines, so do not assume it provides
    // a positive subroutine corpus.
    let bytes = fixture();
    let (_, cff, length) = table_location(&bytes, b"CFF ");
    let mut table = bytes[cff..cff + length].to_vec();
    let names = parse_cff_index(&table, 4, None).unwrap();
    let top = parse_cff_index(&table, names.end, None).unwrap();
    let strings = parse_cff_index(&table, top.end, None).unwrap();
    table[strings.end..strings.end + 2].copy_from_slice(&2u16.to_be_bytes());
    let limits = limits_with(M4ResourceLimits {
        max_cff_subroutines: 1,
        ..M4ResourceLimits::default()
    });
    let mut context = FontFailureContext::new(0);
    context.table(*b"CFF ", &[], FontFailurePhase::CffIndex);
    let kind = parse_cff(
        &table,
        4,
        std::str::from_utf8(&names.objects[0]).unwrap(),
        [0, 0, 1, 1],
        &limits,
        &mut context,
    )
    .unwrap_err();
    assert_eq!(kind, Cff1Error::SubroutineLimit);
    assert_eq!(context.phase, FontFailurePhase::CffIndex);
    assert_eq!(context.table_tag, Some(*b"CFF "));
    assert_eq!(context.limit, Some(1));
    assert_eq!(context.observed, Some(2));
    assert_eq!(context.table_offset, Some(strings.end as u64));
    assert!(context.position_is_exact);
}
