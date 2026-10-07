use super::*;

fn table(bytes: &[u8], tag: &[u8; 4]) -> (usize, usize) {
    let count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    for record in (12..12 + count * 16).step_by(16) {
        if &bytes[record..record + 4] == tag {
            return (
                record,
                u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap()) as usize,
            );
        }
    }
    panic!("fixture table missing: {tag:?}");
}

fn checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn repair_checksums(bytes: &mut [u8]) {
    let (_, head) = table(bytes, b"head");
    bytes[head + 8..head + 12].fill(0);
    let count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    for record in (12..12 + count * 16).step_by(16) {
        let offset =
            u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap()) as usize;
        let length =
            u32::from_be_bytes(bytes[record + 12..record + 16].try_into().unwrap()) as usize;
        let sum = checksum(&bytes[offset..offset + length]);
        bytes[record + 4..record + 8].copy_from_slice(&sum.to_be_bytes());
    }
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(bytes));
    bytes[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn machine_book_font_diagnostics_preserve_check_build_pointer_and_cause() {
    for (case, id, message, markers) in [
        (
            "vorg",
            2,
            "cff1 unsupported_table",
            vec![
                "table=VORG",
                "phase=sfnt-directory",
                "embedding=allowed",
                "fs_type=0x0000",
            ],
        ),
        (
            "permission",
            2,
            "cff1 restricted_embedding",
            vec![
                "table=OS/2",
                "phase=embedding-permission",
                "embedding=denied",
                "fs_type=0x0002",
                "table_byte=8",
            ],
        ),
        (
            "checksum",
            2,
            "cff1 checksum_mismatch",
            vec!["table=OS/2", "embedding=not-checked"],
        ),
        (
            "ttc-index",
            1,
            "font invalid_face_index",
            vec![
                "requested_face_index=999",
                "face_count=1",
                "present_face_indexes=[0]",
                "face_admission=not-checked",
            ],
        ),
        (
            "cff2",
            2,
            "font unsupported_outline",
            vec![
                "table=CFF2",
                "selected_outline=cff2",
                "embedding=not-checked",
            ],
        ),
    ] {
        let (_tree, job, artifacts, expected) =
            copy_fixture("profiles/production-book-1/combined", case);
        let path = job.join("document-package.json");
        let mut package: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let font = &mut package["resources"]["font_faces"][id];
        let uri = font["uri"].as_str().unwrap().to_owned();
        if case == "ttc-index" {
            font["face_index"] = 999.into();
        } else {
            let mut bytes = fs::read(job.join(&uri)).unwrap();
            match case {
                "vorg" => {
                    let (record, _) = table(&bytes, b"cmap");
                    bytes[record..record + 4].copy_from_slice(b"VORG");
                    repair_checksums(&mut bytes);
                }
                "permission" | "checksum" => {
                    let (_, offset) = table(&bytes, b"OS/2");
                    bytes[offset + 8..offset + 10].copy_from_slice(&2u16.to_be_bytes());
                    if case == "permission" {
                        repair_checksums(&mut bytes);
                    }
                }
                "cff2" => {
                    let (record, _) = table(&bytes, b"CFF ");
                    bytes[record..record + 4].copy_from_slice(b"CFF2");
                    repair_checksums(&mut bytes);
                }
                _ => unreachable!(),
            }
            fs::write(job.join(&uri), &bytes).unwrap();
            font["expected_sha256"] = typaxis_core::sha256(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
                .into();
        }
        fs::write(&path, serde_json::to_vec(&package).unwrap()).unwrap();
        let checked = run_check_package(CheckPackageOptions {
            package: path,
            package_root: Some(job.clone()),
            profile: MachinePdfProfileId::ProductionBook1,
            diagnostics: Some(artifacts.join("check-diagnostics.json")),
            common: build_options(&job, &artifacts, &expected).common,
        });
        let built = run_build_package(build_options(&job, &artifacts, &expected));
        for outcome in [checked, built] {
            let error = outcome.expect_err(case);
            assert_eq!(error.kind.exit_code(), 1, "{case}: {}", error.message);
            assert_eq!(
                error.message.matches("R7100").count(),
                1,
                "{case}: {}",
                error.message
            );
        }
        assert!(!artifacts.join("output.pdf").exists());
        let manifest = read_json(&artifacts.join("manifest.json"));
        assert_eq!(manifest["status"], "failed");
        assert!(manifest["output"].is_null());
        let fonts = manifest["fonts"].as_array().unwrap();
        assert_eq!(fonts.len(), id);
        for font in fonts {
            assert_eq!(font["media_declaration"]["kind"], "declared");
            assert_eq!(
                font["attested_media_kind"],
                font["media_declaration"]["media_type"]
            );
        }
        assert!(manifest["images"].as_array().unwrap().is_empty());
        let checked = read_json(&artifacts.join("check-diagnostics.json"));
        let built = read_json(&artifacts.join("diagnostics.json"));
        for diagnostics in [&checked, &built] {
            let f = &diagnostics["diagnostics"][0];
            assert_eq!(f["code"], "R7100");
            assert_eq!(f["message"], message);
            assert_eq!(
                f["location"]["json_pointer"].as_str(),
                Some(format!("/resources/font_faces/{id}").as_str())
            );
            assert!(f["location"]["byte_offset"].is_null());
            let notes = f["notes"].as_array().unwrap();
            assert_eq!(notes.len(), 3);
            assert_eq!(
                notes[0]["message"].as_str(),
                Some(format!("resource={uri}").as_str())
            );
            let context = notes[1]["message"].as_str().unwrap();
            for marker in &markers {
                assert!(
                    context.contains(marker),
                    "{case}: {context}; missing {marker}"
                );
            }
            assert!(notes[2]["message"]
                .as_str()
                .unwrap()
                .contains("inspect-font FONT"));
        }
        assert_eq!(
            checked["diagnostics"][0]["notes"],
            built["diagnostics"][0]["notes"]
        );
    }
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn machine_book_cff_finalization_diagnostics_preserve_resource_and_cause() {
    for (case, limit, code, reason, exit_code) in [
        ("operations", Some("max-cff-charstring-operations"), "R7133", "budget_exceeded", 5),
        ("segments", Some("max-cff-outline-segments"), "R7134", "budget_exceeded", 5),
        ("subset", Some("max-font-subset-bytes"), "R7135", "budget_exceeded", 5),
        ("reserved", None, "R7100", "reserved_cff_operator", 1),
        ("unsupported", None, "R7100", "unsupported_cff_operator", 1),
        ("end", None, "R7100", "invalid_charstring", 1),
        ("bounds", None, "R7100", "invalid_table", 1),
    ] {
        let (_tree, job, artifacts, expected) = copy_fixture("profiles/production-book-1/combined", case);
        let path = job.join("document-package.json");
        let mut package: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let font = &mut package["resources"]["font_faces"][2];
        let uri = font["uri"].as_str().unwrap().to_owned();
        let mut bytes = fs::read(job.join(&uri)).unwrap();
        // The fixed synthetic .notdef is independent of the selected A/B text.
        let program = [248, 236, 189, 22, 248, 136, 249, 80, 252, 136, 6, 14];
        let starts: Vec<_> = bytes.windows(program.len()).enumerate()
            .filter_map(|(i, p)| (p == program).then_some(i)).collect();
        assert_eq!(starts.len(), 1);
        let start = starts[0];
        match case {
            "reserved" => bytes[start] = 0,
            "unsupported" => bytes[start..start + 2].copy_from_slice(&[12, 10]),
            "end" => bytes[start..start + program.len()].fill(139),
            "bounds" => bytes[start..start + program.len()].copy_from_slice(&[248, 236, 255, 127, 255, 128, 0, 22, 138, 139, 5, 14]),
            _ => {},
        }
        if limit.is_none() {
            repair_checksums(&mut bytes);
            fs::write(job.join(&uri), &bytes).unwrap();
            font["expected_sha256"] = typaxis_core::sha256(&bytes).iter()
                .map(|v| format!("{v:02x}")).collect::<String>().into();
            fs::write(&path, serde_json::to_vec(&package).unwrap()).unwrap();
        }
        // Independent evidence for the byte budget comes from an actual
        // successful build of the same package with the default limit.
        let reference = if case == "subset" {
            let reference_artifacts = artifacts.join("reference");
            let mut reference_options = build_options(&job, &reference_artifacts, &expected);
            reference_options.common.no_compress = true;
            run_build_package(reference_options).unwrap();
            let pdf = fs::read(reference_artifacts.join("output.pdf")).unwrap();
            let offsets: Vec<_> = pdf.windows(4).enumerate()
                .filter_map(|(i, b)| (b == b"OTTO").then_some(i)).collect();
            assert_eq!(offsets.len(), 1);
            let start = offsets[0];
            let count = u16::from_be_bytes(pdf[start + 4..start + 6].try_into().unwrap()) as usize;
            let length = (12..12 + count * 16).step_by(16).map(|record| {
                let p = start + record;
                let offset = u32::from_be_bytes(pdf[p + 8..p + 12].try_into().unwrap()) as usize;
                let size = u32::from_be_bytes(pdf[p + 12..p + 16].try_into().unwrap()) as usize;
                (offset + size + 3) & !3
            }).max().unwrap();
            assert!(pdf[start + length..].starts_with(b"\nendstream"));
            Some((pdf[start..start + length].to_vec(), pdf))
        } else { None };
        let mut options = build_options(&job, &artifacts, &expected);
        if let Some(name) = limit { options.common.limits.push((name.to_owned(), 1)); }
        // Check performs admission; selected-glyph evaluation belongs to build.
        let checked = run_check_package(CheckPackageOptions {
            package: path, package_root: Some(job.clone()), profile: MachinePdfProfileId::ProductionBook1,
            diagnostics: Some(artifacts.join("check-diagnostics.json")), common: options.common.clone(),
        });
        assert!(checked.is_ok(), "{case}: {checked:?}");
        assert!(read_json(&artifacts.join("check-diagnostics.json"))["diagnostics"].as_array().unwrap().is_empty());
        if case == "unsupported" {
            fs::write(artifacts.join("output.pdf"), b"existing PDF target").unwrap();
            options.force = true;
        }
        let error = run_build_package(options).expect_err(case);
        assert_eq!(error.kind.exit_code(), exit_code, "{case}: {}", error.message);
        assert!(error.message.starts_with(code), "{case}: {}", error.message);
        if case == "unsupported" { assert_eq!(fs::read(artifacts.join("output.pdf")).unwrap(), b"existing PDF target"); }
        else { assert!(!artifacts.join("output.pdf").exists()); }
        let manifest = read_json(&artifacts.join("manifest.json"));
        assert_eq!(manifest["status"], "failed");
        assert!(manifest["output"].is_null());
        assert_eq!(manifest["fonts"].as_array().unwrap().len(), 3);
        let diagnostics = read_json(&artifacts.join("diagnostics.json"));
        assert_eq!(diagnostics["contract"], "typaxis.contract/1.4");
        assert_eq!(diagnostics["diagnostics"].as_array().unwrap().len(), 1);
        let diagnostic = &diagnostics["diagnostics"][0];
        assert_eq!(diagnostic["code"], code);
        assert_eq!(diagnostic["message"].as_str(), Some(format!("cff1 {reason}").as_str()));
        assert_eq!(diagnostic["location"]["json_pointer"], "/resources/font_faces/2");
        assert!(diagnostic["location"]["byte_offset"].is_null());
        let notes = diagnostic["notes"].as_array().unwrap();
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0]["message"].as_str(), Some(format!("resource={uri}").as_str()));
        let context = notes[1]["message"].as_str().unwrap();
        assert!(context.contains("embedding=allowed; fs_type=0x0000"));
        assert!(!context.contains("; fd="));
        if case == "subset" {
            assert!(context.starts_with("phase=subset;"));
            assert!(!context.contains("font_byte="));
            assert!(!context.contains("gid="));
            let length = reference.as_ref().unwrap().0.len();
            assert!(context.contains(&format!("subset_stage=sfnt-size; limit=1; observed={length};")), "{context}");
        } else if case == "bounds" {
            assert!(context.starts_with("phase=subset;"));
            assert!(context.contains("gid=0; subset_stage=glyph-bounds;"));
            for absent in ["font_byte=", "table_byte=", "cff_operator=", "limit=", "observed="] {
                assert!(!context.contains(absent), "{context}");
            }
        } else {
            assert!(context.starts_with("phase=charstring;"));
            assert!(context.contains("gid=0"));
            let position = start + match case { "operations" => 2, "segments" => 10, "end" => program.len(), _ => 0 };
            assert!(context.contains(&format!("font_byte={position};")), "{case}: {context}");
            assert!(context.contains(if case == "end" { "offset_kind=program-end" } else { "offset_kind=field" }));
        }
        assert!(notes[2]["message"].as_str().unwrap().contains("inspect-font FONT"));
        if let Ok(probes) = std::env::var("VMB_PUBLIC_CFF_DIAGNOSTIC_PROBE") {
            let destination = PathBuf::from(probes).join(case);
            fs::create_dir_all(&destination).unwrap();
            for file in ["diagnostics.json", "check-diagnostics.json", "manifest.json"] {
                fs::copy(artifacts.join(file), destination.join(file)).unwrap();
            }
            fs::write(destination.join("font.otf"), &bytes).unwrap();
            if let Some((font, pdf)) = &reference {
                fs::write(destination.join("reference-subset.otf"), font).unwrap();
                fs::write(destination.join("reference.pdf"), pdf).unwrap();
            }
            fs::write(destination.join("outcome.json"), serde_json::to_vec_pretty(&serde_json::json!({
                "case":case,"exit_code":error.kind.exit_code(),"message":error.message,
                "output_preserved":case == "unsupported","font_sha256":typaxis_core::sha256(&bytes).iter().map(|v| format!("{v:02x}")).collect::<String>(),
                "resource_uri":uri,
            })).unwrap()).unwrap();
        }
    }
}
