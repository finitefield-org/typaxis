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
