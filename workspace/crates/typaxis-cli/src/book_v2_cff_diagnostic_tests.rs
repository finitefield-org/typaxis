use super::*;

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn book_v2_cff_diagnostics_reach_driver_with_original_font_positions_and_budgets() {
    use std::error::Error;
    use typaxis_font::{
        Cff1Error, CffGlyphFailureV2, FontEmbeddingStatus, FontFailurePhase, FontFailureReason,
    };
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hash = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hash,
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let table_count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    let record = bytes[12..12 + 16 * table_count]
        .chunks_exact(16)
        .find(|r| &r[..4] == b"CFF ")
        .unwrap();
    let cff_base = u32::from_be_bytes(record[8..12].try_into().unwrap()) as u64;
    let cff_len = u32::from_be_bytes(record[12..16].try_into().unwrap()) as u64;
    for axis in ["operations", "segments"] {
        let root = Root::new();
        let defaults = limits();
        let mut extension = *defaults.extension().get();
        if axis == "operations" {
            extension.max_cff_charstring_operations = 1;
        } else {
            extension.max_cff_outline_segments = 1;
        }
        let limits = M4EffectiveResourceLimits::new(defaults.base().clone(), extension).unwrap();
        let mut data = source_data("本文");
        let master = &mut data["page_masters"]["masters"][0];
        master["width"] = 12_000_000.into();
        master["height"] = 12_000_000.into();
        master["trim"] = json!({"x":0,"y":0,"width":12_000_000,"height":12_000_000});
        master["body"] = json!({"x":500_000,"y":500_000,"width":10_000_000,"height":10_000_000});
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = hash.clone().into();
        let body = body_with_source(&root, data, "本文".as_bytes(), &limits);
        fs::write(root.0.join("body.bin"), &bytes).unwrap();
        let input = prepare_book_v2_resources(
            body,
            &root.context(),
            &config_with_extension(limits.base().get().clone(), extension),
            &limits,
        )
        .unwrap();
        let error = crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            typaxis_linebreak::JapaneseLineBreakMode::Normal,
            100_000_000,
            |_, _| panic!("exhausted CFF budget reached PDF"),
        )
        .err()
        .unwrap();
        let mut source: &(dyn Error + 'static) = &error;
        let mut depth = 0;
        let failure = loop {
            if let Some(failure) = source.downcast_ref::<CffGlyphFailureV2>() {
                break failure;
            }
            source = source
                .source()
                .unwrap_or_else(|| panic!("{axis}: lost CFF source at {source:?}; root={error:?}"));
            depth += 1;
            assert!(depth <= 8);
        };
        assert!(depth >= 3);
        let expected = if axis == "operations" {
            Cff1Error::CharstringOperationLimit
        } else {
            Cff1Error::OutlineSegmentLimit
        };
        assert_eq!(failure.kind, expected);
        let context = failure.font_context.unwrap();
        assert_eq!(
            (context.phase, context.reason),
            (
                FontFailurePhase::Charstring,
                FontFailureReason::BudgetExceeded
            )
        );
        assert_eq!(context.table_tag, Some(*b"CFF "));
        assert_eq!(context.requested_face_index, 0);
        assert_eq!(context.embedding, FontEmbeddingStatus::Allowed(0));
        assert_eq!((context.limit, context.observed), (Some(1), Some(2)));
        assert_eq!(context.gid, Some(u32::from(failure.gid)));
        assert_eq!(context.fd, failure.fd.map(u16::from));
        let offset = context.table_offset.unwrap();
        assert!(offset < cff_len);
        assert_eq!(context.file_offset, Some(cff_base + offset));
        assert!(context.position_is_exact);
        assert!(!context.position_is_end);
        assert_eq!(context.operator, failure.operator);
        let byte = bytes[(cff_base + offset) as usize];
        if byte == 28 || byte >= 32 {
            assert_eq!(context.operator, None);
        } else if byte == 12 {
            assert_eq!(
                context.operator,
                Some(0x0c00 | u16::from(bytes[(cff_base + offset + 1) as usize]))
            );
        } else {
            assert_eq!(context.operator, Some(u16::from(byte)));
        }
        let note = typaxis_font::Cff1Failure {
            kind: failure.kind,
            context,
        }
        .context_note();
        assert!(note.contains("phase=charstring; reason=budget_exceeded; class=resource-budget"));
        assert!(note.contains(&format!("font_byte={}", cff_base + offset)));
        assert!(note.len() < 1024);
        if let Ok(path) = std::env::var("TYPAXIS_BOOK_V2_FONT_FAILURE_PROBE") {
            let path = PathBuf::from(path);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join(format!("{axis}.json")), serde_json::to_vec_pretty(&json!({
                "font_sha256": hash, "axis":axis, "note":note, "gid":failure.gid,"fd":failure.fd,
                "font_byte":context.file_offset,"table_byte":context.table_offset,"operator":context.operator,
                "limit":context.limit,"observed":context.observed,"source_chain_depth":depth
            })).unwrap()).unwrap();
        }
    }
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn book_v2_cff_subset_diagnostics_reach_driver_with_resource_and_measured_sizes() {
    use std::error::Error;
    use typaxis_font::{
        Cff1Error, Cff1Failure, FontEmbeddingStatus, FontFailurePhase, FontSubsetStage,
    };
    use typaxis_resources::book_v2::BookV2CffSubsetError;
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hash = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hash,
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let mut data = source_data("本文");
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 12_000_000.into();
    master["height"] = 12_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":12_000_000,"height":12_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":10_000_000,"height":10_000_000});
    data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
    data["resources"]["font_faces"][0]["expected_sha256"] = hash.clone().into();
    let run = |byte_limit: Option<u64>| {
        let root = Root::new();
        let defaults = limits();
        let mut extension = *defaults.extension().get();
        if let Some(limit) = byte_limit {
            extension.max_font_subset_bytes = limit;
        }
        let limits = M4EffectiveResourceLimits::new(defaults.base().clone(), extension).unwrap();
        let body = body_with_source(&root, data.clone(), "本文".as_bytes(), &limits);
        fs::write(root.0.join("body.bin"), &bytes).unwrap();
        let input = prepare_book_v2_resources(
            body,
            &root.context(),
            &config_with_extension(limits.base().get().clone(), extension),
            &limits,
        )
        .unwrap();
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            typaxis_linebreak::JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, _| pdf.bytes().to_vec(),
        )
    };
    let pdf = run(None).unwrap();
    let offsets: Vec<_> = pdf
        .windows(4)
        .enumerate()
        .filter_map(|(i, b)| (b == b"OTTO").then_some(i))
        .collect();
    assert_eq!(offsets.len(), 1);
    let start = offsets[0];
    let count = u16::from_be_bytes(pdf[start + 4..start + 6].try_into().unwrap()) as usize;
    let size = (12..12 + count * 16)
        .step_by(16)
        .map(|record| {
            let p = start + record;
            let offset = u32::from_be_bytes(pdf[p + 8..p + 12].try_into().unwrap()) as usize;
            let length = u32::from_be_bytes(pdf[p + 12..p + 16].try_into().unwrap()) as usize;
            (offset + length + 3) & !3
        })
        .max()
        .unwrap();
    let subset = &pdf[start..start + size];
    assert!(pdf[start + size..].starts_with(b"\nendstream"));
    let probe = std::env::var("TYPAXIS_BOOK_V2_SUBSET_FAILURE_PROBE")
        .ok()
        .map(PathBuf::from);
    if let Some(path) = &probe {
        fs::create_dir_all(path).unwrap();
        fs::write(path.join("reference.pdf"), &pdf).unwrap();
        fs::write(path.join("reference.otf"), subset).unwrap();
        fs::write(
            path.join("source.json"),
            serde_json::to_vec_pretty(&data).unwrap(),
        )
        .unwrap();
    }
    for (case, limit, stage) in [
        ("charstrings", 1, FontSubsetStage::CharstringSize),
        ("sfnt", size as u64 - 1, FontSubsetStage::SfntSize),
    ] {
        let error = run(Some(limit)).unwrap_err();
        let mut source: &(dyn Error + 'static) = &error;
        let mut depth = 0;
        let mut face = None;
        let failure = loop {
            if let Some(BookV2CffSubsetError::FontDetailed { font_face_id, .. }) =
                source.downcast_ref::<BookV2CffSubsetError>()
            {
                face = Some(font_face_id.get());
            }
            if let Some(failure) = source.downcast_ref::<Cff1Failure>() {
                break failure;
            }
            source = source
                .source()
                .unwrap_or_else(|| panic!("lost subset source: {error:?}"));
            depth += 1;
            assert!(depth <= 8);
        };
        assert_eq!(face, Some(0));
        assert!(depth >= 3);
        assert_eq!(failure.kind, Cff1Error::SubsetByteLimit);
        let c = failure.context;
        assert_eq!(c.phase, FontFailurePhase::Subset);
        assert_eq!(c.subset_stage, Some(stage));
        assert_eq!(c.embedding, FontEmbeddingStatus::Allowed(0));
        assert_eq!(c.limit, Some(limit));
        assert!(c.observed.unwrap() > limit);
        assert_eq!(
            (c.table_tag, c.file_offset, c.table_offset, c.operator),
            (None, None, None, None)
        );
        if case == "sfnt" {
            assert_eq!(c.observed, Some(size as u64));
            assert_eq!((c.gid, c.fd), (None, None));
        } else {
            assert_eq!((c.gid, c.fd), (Some(0), Some(5)));
        }
        if let Some(path) = &probe {
            fs::write(path.join(format!("{case}.json")),serde_json::to_vec_pretty(&json!({
                "font_sha256":hash,"font_face_id":face,"case":case,"note":failure.context_note(),
                "code":failure.kind.diagnostic_code(),"gid":c.gid,"fd":c.fd,"limit":c.limit,"observed":c.observed,
                "source_chain_depth":depth
            })).unwrap()).unwrap();
        }
    }
    // Exact final size still permits publication with the same font bytes.
    let exact = run(Some(size as u64)).unwrap();
    assert!(exact.windows(subset.len()).any(|bytes| bytes == subset));
}
