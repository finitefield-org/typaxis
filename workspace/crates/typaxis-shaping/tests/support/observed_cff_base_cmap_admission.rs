use super::*;
use typaxis_font::validate_cff_cmap_v2_with_base_cmap_budget;

#[test]
fn observed_cff_full_cmap_source_caps_stop_before_span_and_owned_storage_and_keep_retry() {
    let mut b = small_cmap();
    b[2..4].copy_from_slice(&2u16.to_be_bytes());
    b[8..12].copy_from_slice(&20u32.to_be_bytes());
    b.splice(12..12, [0, 0, 0, 5, 0, 0, 0, 48]);
    b.extend([0, 14]);
    b.extend(38u32.to_be_bytes());
    b.extend(1u32.to_be_bytes());
    b.extend([0xe, 1, 0]);
    b.extend(21u32.to_be_bytes());
    b.extend(29u32.to_be_bytes());
    b.extend(1u32.to_be_bytes());
    b.extend([0, 0, 65, 2]);
    b.extend(1u32.to_be_bytes());
    b.extend([0, 0, 68, 0, 4]);
    let base = limits();
    let run = |owner, axis: usize, extra: u64| {
        let mut raw = base.base().get().clone();
        if axis == 1 {
            raw.max_spool_bytes = PRIOR[1] + extra;
        }
        let domain = ValidatedResourceLimits::new(raw).unwrap();
        let [mut r, mut s, mut w] = PRIOR;
        let result = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            &domain,
            if axis == 0 { PRIOR[0] + extra } else { MAXIMUM },
            if axis == 2 { PRIOR[2] + extra } else { MAXIMUM },
            |a| {
                typaxis_font::validate_cff_cmap_v2_with_cmap_budget(
                    &b,
                    8,
                    &mut a.cmap_budget(owner),
                )
            },
        );
        (result, [r, s, w])
    };
    let (ready, full) = run(NodeId::new(91), 3, 0);
    assert_eq!(ready.unwrap().unwrap().base_mapping_count(), 3);
    let delta = [full[0] - PRIOR[0], full[1] - PRIOR[1], full[2] - PRIOR[2]];
    assert_eq!(delta[0], 5);
    assert_eq!(
        delta[1],
        24 + 38 + 2 * std::mem::size_of::<(usize, usize, u8)>() as u64
    );
    let mut cases = 0;
    for owner in [NodeId::new(91), NodeId::new(205)] {
        for axis in 0..3 {
            for extra in 0..=delta[axis] + 1 {
                let (result, cells) = run(owner, axis, extra);
                if extra >= delta[axis] {
                    result.unwrap().unwrap();
                    assert_eq!(cells, full);
                } else {
                    let cause = match result.unwrap_err() {
                        typaxis_font::CffCmapBudgetFailureV2::Base(f) => f.error,
                        typaxis_font::CffCmapBudgetFailureV2::Variation(f) => f.error,
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
                cases += 1;
            }
        }
        let mut raw = base.base().get().clone();
        raw.max_spool_bytes = PRIOR[1] + 2 * std::mem::size_of::<(usize, usize, u8)>() as u64 - 1;
        let domain = ValidatedResourceLimits::new(raw).unwrap();
        let [mut r, mut s, mut w] = PRIOR;
        with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            &domain,
            MAXIMUM,
            MAXIMUM,
            |a| {
                for expected_work in [17, 21] {
                    let failure = typaxis_font::validate_cff_cmap_v2_with_cmap_budget(
                        &b,
                        8,
                        &mut a.cmap_budget(owner),
                    )
                    .unwrap_err();
                    assert!(
                        matches!(failure,typaxis_font::CffCmapBudgetFailureV2::Variation(typaxis_font::CffVariationBudgetFailureV2 {error:BookV2ObservedAccountingError::Spool {owner:o},table_offset:54,file_offset:None}) if o==owner)
                    );
                    assert_eq!(a.counters(), [PRIOR[0], PRIOR[1], expected_work]);
                }
            },
        );
    }
    eprintln!("full cmap Source caps={cases}, exact delta={delta:?}, span refusal and same-cell retry atomic, owners=91/205");
}

fn small_cmap() -> Vec<u8> {
    let mut b = vec![0, 0, 0, 1, 0, 3, 0, 10, 0, 0, 0, 12, 0, 12, 0, 0];
    for n in [28u32, 0, 1, 65, 67, 1] {
        b.extend(n.to_be_bytes());
    }
    b
}
#[test]
fn observed_cff_base_cmap_source_all_axes_keep_atomic_prefix_and_retry_history() {
    let b = small_cmap();
    let base = limits();
    let mut total = 0;
    for owner in [NodeId::new(91), NodeId::new(205)] {
        for axis in 0..3 {
            for extra in 0..=[3u64, 24, 13][axis] + 1 {
                let mut raw = base.base().get().clone();
                if axis == 1 {
                    raw.max_spool_bytes = PRIOR[1] + extra;
                }
                let domain = ValidatedResourceLimits::new(raw).unwrap();
                let [mut r, mut s, mut w] = PRIOR;
                let cap_r = if axis == 0 { PRIOR[0] + extra } else { MAXIMUM };
                let cap_w = if axis == 2 { PRIOR[2] + extra } else { MAXIMUM };
                let result = with_book_v2_observed_source_accounting(
                    &mut r,
                    &mut s,
                    &mut w,
                    &domain,
                    cap_r,
                    cap_w,
                    |a| {
                        validate_cff_cmap_v2_with_base_cmap_budget(
                            &b,
                            8,
                            &mut a.base_cmap_budget(owner),
                        )
                    },
                );
                if extra >= [3, 24, 13][axis] {
                    assert_eq!(result.unwrap().unwrap().base_mapping_count(), 3);
                    assert_eq!([r, s, w], [10, 35, 26]);
                } else {
                    let expected = match axis {
                        0 => BookV2ObservedAccountingError::Records { owner },
                        1 => BookV2ObservedAccountingError::Spool { owner },
                        _ => BookV2ObservedAccountingError::Work { owner },
                    };
                    assert_eq!(result.unwrap_err().error, expected);
                    assert!([r, s, w][axis] <= PRIOR[axis] + extra);
                }
                total += 1;
            }
        }
        let mut raw = base.base().get().clone();
        raw.max_spool_bytes = 34;
        let domain = ValidatedResourceLimits::new(raw).unwrap();
        let [mut r, mut s, mut w] = PRIOR;
        with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            &domain,
            MAXIMUM,
            MAXIMUM,
            |a| {
                for expected_work in [20, 27] {
                    let failure = validate_cff_cmap_v2_with_base_cmap_budget(
                        &b,
                        8,
                        &mut a.base_cmap_budget(owner),
                    )
                    .unwrap_err();
                    assert_eq!(
                        failure.error,
                        BookV2ObservedAccountingError::Spool { owner }
                    );
                    assert_eq!(a.counters(), [7, 11, expected_work]);
                }
            },
        );
    }
    eprintln!("base cmap Source cap cases={total}, owners=91/205, atomic reservation and same-cell retry preserved");
}

#[test]
#[ignore = "requires unchanged original CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_cff_base_cmap_real_host_preserves_font_subject_prefix_and_source_continuation() {
    real_host_cmap_admission(false);
}
#[test]
#[ignore = "requires unchanged original CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_cff_full_cmap_real_host_keeps_typed_subject_and_source_continuation() {
    let full = real_host_cmap_admission(true);
    let base_only = real_host_cmap_admission(false);
    assert_eq!(full, base_only);
}
#[test]
#[ignore = "requires unchanged original CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_cff_sfnt_real_host_keeps_subject_prefix_and_cmap_ledger_identity() {
    let sfnt = real_host_font_admission(true, AdmissionPhase::Sfnt);
    let cmap = real_host_cmap_admission(true);
    assert_eq!(sfnt, cmap);
}
fn real_host_cmap_admission(full_cmap: bool) -> [u8; 32] {
    real_host_font_admission(full_cmap, AdmissionPhase::Cmap)
}
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum AdmissionPhase {
    Cmap,
    Sfnt,
    Metadata,
    Tables,
    Program,
    OptionalLocal,
    Math,
    Jstf,
}
fn real_host_font_admission(full_cmap: bool, phase: AdmissionPhase) -> [u8; 32] {
    let sfnt = phase >= AdmissionPhase::Sfnt;
    let metadata = phase >= AdmissionPhase::Metadata;
    let tables = phase >= AdmissionPhase::Tables;
    let program = phase >= AdmissionPhase::Program;
    let optional_local = phase >= AdmissionPhase::OptionalLocal;
    let math = phase >= AdmissionPhase::Math;
    let jstf = phase >= AdmissionPhase::Jstf;
    let font_path =
        fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let font = fs::read(font_path.lines().next().unwrap()).unwrap();
    assert_eq!(
        sha256(&font),
        [
            0x66, 0xef, 0x32, 0x70, 0xe6, 0x86, 0x90, 0x61, 0x2e, 0x8b, 0xf9, 0x82, 0xac, 0xfa,
            0xd0, 0xe8, 0xb4, 0x02, 0x12, 0xce, 0x64, 0x66, 0x1c, 0xce, 0x2b, 0xb6, 0xd3, 0xa9,
            0x8a, 0xc8, 0x47, 0x17,
        ]
    );
    let count = usize::from(u16::from_be_bytes([font[4], font[5]]));
    let cmap_record = font[12..12 + count * 16]
        .chunks_exact(16)
        .find(|record| &record[..4] == b"cmap")
        .unwrap();
    let cmap_start = u64::from(u32::from_be_bytes(cmap_record[8..12].try_into().unwrap()));
    let base = limits();
    let owner = NodeId::new(0);
    let run = |caps: &M4EffectiveResourceLimits, cap_r: u64, cap_w: u64, continue_source: bool| {
        let root = Root::new();
        let mut data = source_data("Result");
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            .into();
        let mut alias = data["resources"]["font_faces"][0].clone();
        alias["font_face_id"] = 2.into();
        alias["family"] = "Alias".into();
        data["resources"]["font_faces"]
            .as_array_mut()
            .unwrap()
            .push(alias);
        let body = body_with_source(&root, data, b"Result", caps);
        fs::write(root.0.join("body.bin"), &font).unwrap();
        let policy = prepare_book_v2_resource_policy(&body, caps).unwrap();
        let declared = staging_declared_base_catalog(body.styled().body().resources()).unwrap();
        let context = root.context();
        let cfg = config_with_extension(caps.base().get().clone(), *caps.extension().get());
        let host = HostResourceAdmissionSession::new_with_read_ledger(
            &context,
            &cfg,
            declared.resource_catalog(),
            body.provenance().read_ledger(),
        )
        .unwrap();
        let mut resolver = StagingProductionResourceResolverV3::new(
            &declared,
            caps,
            policy.fingerprint(),
            host.roots(),
        )
        .unwrap();
        let [mut r, mut s, mut w] = PRIOR;
        let mut after_admission = None;
        let mut fp = None;
        let mut before_source = None;
        let result = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            caps.base(),
            cap_r,
            cap_w,
            |a| {
                for declaration in &declared.resource_catalog().font_faces {
                    let pending = resolver
                        .read_font(host.open_font(declaration.font_face_id).unwrap())
                        .unwrap();
                    let result = if jstf {
                        resolver.parse_and_bind_font_with_admission_budget(
                            pending,
                            &mut a.font_jstf_budget(owner),
                        )
                    } else if math {
                        resolver.parse_and_bind_font_with_admission_budget(
                            pending,
                            &mut a.font_math_budget(owner),
                        )
                    } else if optional_local {
                        resolver.parse_and_bind_font_with_admission_budget(
                            pending,
                            &mut a.font_optional_local_budget(owner),
                        )
                    } else if program {
                        resolver.parse_and_bind_font_with_admission_budget(
                            pending,
                            &mut a.font_program_budget(owner),
                        )
                    } else if tables {
                        resolver.parse_and_bind_font_with_admission_budget(
                            pending,
                            &mut a.font_tables_budget(owner),
                        )
                    } else if metadata {
                        resolver.parse_and_bind_font_with_admission_budget(
                            pending,
                            &mut a.font_metadata_budget(owner),
                        )
                    } else if sfnt {
                        resolver.parse_and_bind_font_with_admission_budget(
                            pending,
                            &mut a.admission_budget(owner),
                        )
                    } else {
                        let cmap_result = if full_cmap {
                            resolver.parse_and_bind_font_with_cmap_budget(
                                pending,
                                &mut a.cmap_budget(owner),
                            )
                        } else {
                            resolver
                                .parse_and_bind_font_with_base_cmap_budget(
                                    pending,
                                    &mut a.base_cmap_budget(owner),
                                )
                                .map_err(|f| {
                                    typaxis_resource_admission::ProductionCmapBudgetFailureV3 {
                                        font_face_id: f.font_face_id,
                                        failure: typaxis_font::CffCmapBudgetFailureV2::Base(
                                            f.failure,
                                        ),
                                    }
                                })
                        };
                        cmap_result.map_err(|f| {
                            typaxis_resource_admission::ProductionAdmissionBudgetFailureV3 {
                                font_face_id: f.font_face_id,
                                failure: typaxis_font::CffAdmissionBudgetFailureV2::Cmap(f.failure),
                            }
                        })
                    };
                    match result {
                        Ok(native) => native.unwrap(),
                        Err(failure) => {
                            assert!(matches!(resolver.finish(),Err(typaxis_resource_admission::ProductionResourceErrorV3::Resource(typaxis_resource_admission::ResourceAdmissionError::MissingLogicalResource))));
                            return Err(failure);
                        }
                    }
                }
                for image in &declared.resource_catalog().images {
                    let pending = resolver
                        .read_image(host.open_image(image.image_id).unwrap())
                        .unwrap();
                    resolver.parse_and_bind_image(pending).unwrap();
                }
                let ledger = resolver.finish().unwrap();
                fp = Some(ledger.fingerprint());
                after_admission = Some(a.counters());
                if continue_source {
                    before_source = Some(a.counters());
                    let nav = prepare_book_v2_navigation(body.styled()).unwrap();
                    let flow = a.prepare_source(body.styled(), &nav).unwrap();
                    flow.verify_for(body.styled(), &nav).unwrap();
                    assert!(a.counters()[0] > before_source.unwrap()[0]);
                    assert!(a.counters()[2] > before_source.unwrap()[2]);
                }
                Ok(())
            },
        );
        (result, [r, s, w], after_admission, fp)
    };
    let (ready, full, admission, fp) = run(&base, MAXIMUM, MAXIMUM, false);
    ready.unwrap();
    assert_eq!(admission, Some(full));
    let (continued, after, admission2, fp2) = run(&base, MAXIMUM, MAXIMUM, true);
    continued.unwrap();
    assert_eq!(admission2, Some(full));
    assert_eq!(fp2, fp);
    assert!(after[0] > full[0] && after[2] > full[2]);
    for axis in 0..3 {
        let mut raw = base.base().get().clone();
        if axis == 1 {
            raw.max_spool_bytes = full[1] - 1;
        }
        let mut extension = *base.extension().get();
        extension.max_font_subset_bytes = extension.max_font_subset_bytes.min(raw.max_spool_bytes);
        let caps =
            M4EffectiveResourceLimits::new(ValidatedResourceLimits::new(raw).unwrap(), extension)
                .unwrap();
        let cap_r = if axis == 0 { full[0] - 1 } else { MAXIMUM };
        let cap_w = if axis == 2 { full[2] - 1 } else { MAXIMUM };
        let (result, cells, admission, _) = run(&caps, cap_r, cap_w, false);
        assert!(admission.is_none());
        let failure = result.unwrap_err();
        assert_eq!(failure.font_face_id.get(), 2);
        let expected = match axis {
            0 => BookV2ObservedAccountingError::Records { owner },
            1 => BookV2ObservedAccountingError::Spool { owner },
            _ => BookV2ObservedAccountingError::Work { owner },
        };
        let cause = match failure.failure {
            typaxis_font::CffAdmissionBudgetFailureV2::Sfnt(f) => {
                if let Some(offset) = f.point.file_offset {
                    assert!(offset <= font.len() as u64);
                }
                f.error
            }
            typaxis_font::CffAdmissionBudgetFailureV2::Program(f) => f.error,
            typaxis_font::CffAdmissionBudgetFailureV2::Tables(f) => f.error,
            typaxis_font::CffAdmissionBudgetFailureV2::Metadata(f) => {
                if let Some(offset) = f.point.file_offset {
                    assert!(offset <= font.len() as u64);
                }
                f.error
            }
            typaxis_font::CffAdmissionBudgetFailureV2::Cmap(failure) => match failure {
                typaxis_font::CffCmapBudgetFailureV2::Base(f) => {
                    assert_eq!(
                        f.file_offset,
                        f.table_offset.map(|offset| cmap_start + offset)
                    );
                    f.error
                }
                typaxis_font::CffCmapBudgetFailureV2::Variation(f) => {
                    assert_eq!(f.file_offset, Some(cmap_start + f.table_offset));
                    f.error
                }
            },
        };
        assert_eq!(cause, expected);
        assert!(cells[axis] < full[axis]);
    }
    if jstf {
        let count = usize::from(u16::from_be_bytes([font[4], font[5]]));
        assert!(!font[12..12 + count * 16]
            .chunks_exact(16)
            .any(|r| r[..4] == *b"JSTF"));
        eprintln!("real original CFF JSTF mode admission full={full:?}, Source continuation={after:?}, absent-JSTF aliases and all-axis one-short font subject=2");
    } else if math {
        eprintln!("real original CFF MATH mode admission full={full:?}, Source continuation={after:?}, absent-MATH aliases and all-axis one-short font subject=2");
    } else if optional_local {
        eprintln!("real original CFF local optional admission full={full:?}, Source continuation={after:?}, two genuine aliases and all-axis one-short font subject=2");
    } else if program {
        eprintln!("real original CFF program admission full={full:?}, Source continuation={after:?}, two genuine aliases and all-axis one-short font subject=2");
    } else if tables {
        eprintln!("real original CFF tables admission full={full:?}, Source continuation={after:?}, two genuine aliases and all-axis one-short font subject=2");
    } else if metadata {
        eprintln!("real original CFF metadata admission full={full:?}, Source continuation={after:?}, two genuine aliases and all-axis one-short font subject=2");
    } else if sfnt {
        eprintln!("real original CFF SFNT/cmap admission full={full:?}, Source continuation={after:?}, two genuine aliases and all-axis one-short font subject=2");
    } else if full_cmap {
        eprintln!("real original CFF full cmap admission full={full:?}, Source continuation={after:?}, two genuine aliases and all-axis one-short font subject=2");
    } else {
        eprintln!("real original CFF base cmap admission full={full:?}, Source continuation={after:?}, two genuine aliases and all-axis one-short font subject=2");
    }
    fp.unwrap()
}

#[test]
#[ignore = "requires unchanged original CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_cff_metadata_real_host_keeps_subject_prefix_and_admission_identity() {
    let metadata = real_host_font_admission(true, AdmissionPhase::Metadata);
    let sfnt = real_host_font_admission(true, AdmissionPhase::Sfnt);
    assert_eq!(metadata, sfnt);
}

#[test]
#[ignore = "requires unchanged original CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_cff_tables_real_host_keeps_subject_prefix_and_admission_identity() {
    let tables = real_host_font_admission(true, AdmissionPhase::Tables);
    let metadata = real_host_font_admission(true, AdmissionPhase::Metadata);
    assert_eq!(tables, metadata);
}

#[test]
#[ignore = "requires unchanged original CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_cff_program_real_host_keeps_subject_prefix_and_admission_identity() {
    let program = real_host_font_admission(true, AdmissionPhase::Program);
    let tables = real_host_font_admission(true, AdmissionPhase::Tables);
    assert_eq!(program, tables);
}

#[test]
#[ignore = "requires unchanged original CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_cff_local_optional_real_host_keeps_subject_prefix_and_admission_identity() {
    let local = real_host_font_admission(true, AdmissionPhase::OptionalLocal);
    let program = real_host_font_admission(true, AdmissionPhase::Program);
    assert_eq!(local, program);
}

#[test]
#[ignore = "requires unchanged original CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_cff_math_real_host_keeps_subject_prefix_and_absent_table_billing() {
    let math = real_host_font_admission(true, AdmissionPhase::Math);
    let local = real_host_font_admission(true, AdmissionPhase::OptionalLocal);
    assert_eq!(math, local);
}

#[test]
#[ignore = "requires unchanged original CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn observed_cff_jstf_real_host_keeps_subject_prefix_and_absent_table_billing() {
    let jstf = real_host_font_admission(true, AdmissionPhase::Jstf);
    let math = real_host_font_admission(true, AdmissionPhase::Math);
    assert_eq!(jstf, math);
}
