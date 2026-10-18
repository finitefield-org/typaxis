//! Actual numbered table headers, including the original temporary-spool gap.
use super::*;
use typaxis_layout::book_v2::{
    prepare_book_v2_vector_blocks_counted, prepare_book_v2_vector_blocks_with_observed_storage,
};
use typaxis_shaping::book_v2::{
    shape_book_v2_equation_numbers_counted, shape_book_v2_equation_numbers_with_observed_storage,
    with_book_v2_observed_source_accounting,
};
#[path = "../../../vendor/typaxis-harfrust/test-support/aat_font.rs"]
mod aat_fixture;
#[allow(dead_code)]
#[path = "../../../vendor/typaxis-harfrust/test-support/contextual_font.rs"]
mod context_fixture;
#[path = "../../../vendor/typaxis-harfrust/test-support/indic_font.rs"]
mod indic_fixture;
#[allow(dead_code)]
#[path = "../../../vendor/typaxis-harfrust/test-support/multiple_substitution_font.rs"]
mod sfnt_fixture;

#[test]
fn book_v2_column_number_storage_admitted_aat_font_reaches_observed_number_shapes() {
    use typaxis_shaping::book_v2::{BookV2EquationNumberErrorKind, BookV2ObservedAccountingError};
    let limits = changed_caps(&caps(), |c| {
        c.max_line_reshape_passes = 512;
        c.max_layout_passes = 512;
    });
    let root = Root::new();
    const BASE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"));
    let font = sfnt_fixture::with_replaced_tables(BASE, &aat_fixture::aat_tables(3, false));
    let hash = typaxis_core::sha256(&font);
    let hash_hex = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let mut data = header_data(true, true, true);
    data["resources"]["font_faces"][0]["expected_sha256"] = hash_hex.clone().into();
    let source = data["text_buffers"][0]["utf8"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let body = body_with_source(&root, data, &source, &limits);
    fs::write(root.0.join("body.bin"), &font).unwrap();
    fs::write(root.0.join("vector.svg"),include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/x-plus-y.svg"))).unwrap();
    let input = prepare_book_v2_resources(
        body,
        &root.context(),
        &config_with_extension(
            limits.base().get().clone(),
            limits.extension().get().clone(),
        ),
        &limits,
    )
    .unwrap();
    let admitted = input
        .resources()
        .font(typaxis_core::FontFaceId::new(0))
        .unwrap();
    assert_eq!(admitted.bytes(), font);
    assert_eq!(admitted.content_hash(), hash);
    let mut budget = BookV2ColumnPageBudget::new(&limits, 1_000_000_000);
    crate::book_v2_resources::with_budgeted_book_v2_column_pages(&input,&limits,MODE,&mut budget,
        |_,search,_,prior|{
            let shaped=search.source_lines().prepared().shaped();
            let mut legacy_records=prior.record_charge();
            let legacy=shape_book_v2_equation_numbers_counted(shaped,&limits,legacy_records,&mut legacy_records).unwrap().unwrap();
            assert!(legacy.shapes().iter().any(|shape|shape.text().contains('1')));
            let mut receipts=Vec::new();
            let mut owners=Vec::new();
            for shape in legacy.shapes() {
                assert_eq!(shape.font_sha256(),hash);
                let owner=shape.source().node_id();owners.push(owner);
                let label_owner=shape.source().equation_number().unwrap().node_id();
                let glyphs=shape.runs().iter().flat_map(|r|r.glyphs()).map(|g|json!({
                    "original_gid":g.original_gid.get(),"advance_x":g.advance_x.raw(),"advance_y":g.advance_y.raw(),
                    "offset_x":g.offset_x.raw(),"offset_y":g.offset_y.raw()})).collect::<Vec<_>>();
                assert!(glyphs.iter().all(|glyph|glyph["original_gid"]!=18));
                receipts.push(json!({"owner":owner.get(),"label_owner":label_owner.get(),"text":shape.text(),"font_sha256":hash_hex,
                    "glyphs":glyphs}));
            }
            let initial=[prior.record_charge(),11,13];
            let run=|start:[u64;3],max_r,max_w|{
                let [mut r,mut s,mut w]=start;
                let result=with_book_v2_observed_source_accounting(&mut r,&mut s,&mut w,limits.base(),max_r,max_w,
                    |a|shape_book_v2_equation_numbers_with_observed_storage(shaped,&limits,a).map(|numbers|numbers.unwrap().fingerprint()));
                (result,[r,s,w])
            };
            let full=run(initial,limits.base().get().max_fragments,1_000_000_000);
            assert_eq!(full.0,Ok(legacy.fingerprint()));assert_eq!(full.1[0],legacy_records);
            assert_eq!(run(initial,full.1[0],full.1[2]),full);
            let mut short_start=initial;
            short_start[1]=limits.base().get().max_spool_bytes-(full.1[1]-initial[1])+1;
            let spool=run(short_start,full.1[0],full.1[2]);let error=spool.0.unwrap_err();
            assert!(owners.contains(&error.owner),"{error:?}, owners={owners:?}");
            assert_eq!(error.kind,BookV2EquationNumberErrorKind::ObservedAccounting(BookV2ObservedAccountingError::Spool{owner:error.owner}));
            let work=run(initial,full.1[0],full.1[2]-1);let error=work.0.unwrap_err();
            assert!(owners.contains(&error.owner),"{error:?}, owners={owners:?}");
            assert_eq!(error.kind,BookV2EquationNumberErrorKind::ObservedAccounting(BookV2ObservedAccountingError::Work{owner:error.owner}));
            if let Some(directory)=std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
                let output=PathBuf::from(directory).join("aat-number-route");let job=output.join("job");
                fs::create_dir_all(&job).unwrap();
                for entry in fs::read_dir(&root.0).unwrap(){let entry=entry.unwrap();assert!(entry.file_type().unwrap().is_file());fs::copy(entry.path(),job.join(entry.file_name())).unwrap();}
                fs::write(output.join("number-shapes.json"),serde_json::to_vec_pretty(&json!({
                    "schema":"typaxis.admitted-aat-number-route/1","fixture_kind":"derived-test-font",
                    "admitted_font_sha256":hash_hex,"initial":initial,"full":full.1,"shapes":receipts})).unwrap()).unwrap();
            }
            eprintln!("admitted AAT numbered route: shapes={}, full={:?}",legacy.shapes().len(),full.1);
        }).unwrap();
}

#[test]
fn book_v2_column_number_storage_admitted_contextual_font_reaches_observed_number_shapes() {
    use typaxis_shaping::book_v2::{BookV2EquationNumberErrorKind, BookV2ObservedAccountingError};
    let limits = changed_caps(&caps(), |c| {
        c.max_line_reshape_passes = 512;
        c.max_layout_passes = 512;
    });
    let root = Root::new();
    const BASE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"));
    let mut config = context_fixture::Context::sequence(3, false, &[9, 18, 10], 8);
    config.nested_index = 1;
    let font =
        sfnt_fixture::with_replaced_tables(BASE, &context_fixture::contextual_tables(&config));
    let hash = typaxis_core::sha256(&font);
    let hash_hex = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let mut data = header_data(true, true, true);
    // The derived substitution expands three label characters to ten glyphs.
    // Keep its width inside the original repeated-header frame.
    for rule in data["style_sheet"]["rules"].as_array_mut().unwrap() {
        if rule["selector"] == "math_vector_block" {
            for declaration in rule["declarations"].as_array_mut().unwrap() {
                if declaration["name"] == "font_size" {
                    declaration["value"]["value"] = (3 * 65536).into();
                }
            }
        }
    }

    data["resources"]["font_faces"][0]["expected_sha256"] = hash_hex.clone().into();
    let source = data["text_buffers"][0]["utf8"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let body = body_with_source(&root, data, &source, &limits);
    fs::write(root.0.join("body.bin"), &font).unwrap();
    fs::write(root.0.join("vector.svg"),include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/x-plus-y.svg"))).unwrap();
    let input = prepare_book_v2_resources(
        body,
        &root.context(),
        &config_with_extension(
            limits.base().get().clone(),
            limits.extension().get().clone(),
        ),
        &limits,
    )
    .unwrap();
    let admitted = input
        .resources()
        .font(typaxis_core::FontFaceId::new(0))
        .unwrap();
    assert_eq!(admitted.bytes(), font);
    assert_eq!(admitted.content_hash(), hash);
    let mut budget = BookV2ColumnPageBudget::new(&limits, 1_000_000_000);
    crate::book_v2_resources::with_budgeted_book_v2_column_pages(&input,&limits,MODE,&mut budget,
        |_,search,_,prior|{
            let shaped=search.source_lines().prepared().shaped();
            let mut legacy_records=prior.record_charge();
            let legacy=shape_book_v2_equation_numbers_counted(shaped,&limits,legacy_records,&mut legacy_records).unwrap().unwrap();
            assert!(legacy.shapes().iter().any(|shape|shape.text().contains('1')));
            let mut receipts=Vec::new();
            let mut owners=Vec::new();
            for shape in legacy.shapes() {
                assert_eq!(shape.font_sha256(),hash);
                let owner=shape.source().node_id();owners.push(owner);
                let label_owner=shape.source().equation_number().unwrap().node_id();
                let glyphs=shape.runs().iter().flat_map(|r|r.glyphs()).map(|g|json!({
                    "original_gid":g.original_gid.get(),"advance_x":g.advance_x.raw(),"advance_y":g.advance_y.raw(),
                    "offset_x":g.offset_x.raw(),"offset_y":g.offset_y.raw()})).collect::<Vec<_>>();
                assert!(glyphs.iter().all(|glyph|glyph["original_gid"]!=18));
                receipts.push(json!({"owner":owner.get(),"label_owner":label_owner.get(),"text":shape.text(),"font_sha256":hash_hex,
                    "glyphs":glyphs}));
            }
            let initial=[prior.record_charge(),11,13];
            let run=|start:[u64;3],max_r,max_w|{
                let [mut r,mut s,mut w]=start;
                let result=with_book_v2_observed_source_accounting(&mut r,&mut s,&mut w,limits.base(),max_r,max_w,
                    |a|shape_book_v2_equation_numbers_with_observed_storage(shaped,&limits,a).map(|numbers|numbers.unwrap().fingerprint()));
                (result,[r,s,w])
            };
            let full=run(initial,limits.base().get().max_fragments,1_000_000_000);
            assert_eq!(full.0,Ok(legacy.fingerprint()));assert_eq!(full.1[0],legacy_records);
            assert_eq!(run(initial,full.1[0],full.1[2]),full);
            let mut short_start=initial;
            short_start[1]=limits.base().get().max_spool_bytes-(full.1[1]-initial[1])+1;
            let spool=run(short_start,full.1[0],full.1[2]);let error=spool.0.unwrap_err();
            assert!(owners.contains(&error.owner),"{error:?}, owners={owners:?}");
            assert_eq!(error.kind,BookV2EquationNumberErrorKind::ObservedAccounting(BookV2ObservedAccountingError::Spool{owner:error.owner}));
            let work=run(initial,full.1[0],full.1[2]-1);let error=work.0.unwrap_err();
            assert!(owners.contains(&error.owner),"{error:?}, owners={owners:?}");
            assert_eq!(error.kind,BookV2EquationNumberErrorKind::ObservedAccounting(BookV2ObservedAccountingError::Work{owner:error.owner}));
            if let Some(directory)=std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
                let output=PathBuf::from(directory).join("match-number-route");let job=output.join("job");
                fs::create_dir_all(&job).unwrap();
                for entry in fs::read_dir(&root.0).unwrap(){let entry=entry.unwrap();assert!(entry.file_type().unwrap().is_file());fs::copy(entry.path(),job.join(entry.file_name())).unwrap();}
                fs::write(output.join("number-shapes.json"),serde_json::to_vec_pretty(&json!({
                    "schema":"typaxis.admitted-match-number-route/1","fixture_kind":"derived-test-font",
                    "admitted_font_sha256":hash_hex,"initial":initial,"full":full.1,"shapes":receipts})).unwrap()).unwrap();
            }
            eprintln!("admitted contextual numbered route: shapes={}, full={:?}",legacy.shapes().len(),full.1);
        }).unwrap();
}

#[test]
fn book_v2_column_number_storage_admitted_indic_font_reaches_observed_number_shapes() {
    use typaxis_shaping::book_v2::{BookV2EquationNumberErrorKind, BookV2ObservedAccountingError};
    let limits = changed_caps(&caps(), |c| {
        c.max_line_reshape_passes = 512;
        c.max_layout_passes = 512;
    });
    let root = Root::new();
    const BASE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"));
    let font = sfnt_fixture::with_replaced_tables(BASE, &indic_fixture::indic_tables(BASE, false));
    let hash = typaxis_core::sha256(&font);
    let hash_hex = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let mut data = header_data(true, true, true);
    // Replace the final ASCII number label with a six-byte Indic label and
    // extend only its source/text-buffer-zero terminal spans.
    let original = data["text_buffers"][0]["utf8"].as_str().unwrap();
    assert!(original.ends_with("(1)"));
    let mut number_source = original[..original.len() - 3].to_owned();
    number_source.push_str("कि");
    data["text_buffers"][0]["utf8"] = number_source.into();
    fn extend_number_spans(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                if (fields.get("source_id") == Some(&json!(0))
                    || fields.get("text_id") == Some(&json!(0)))
                    && fields.get("end_byte") == Some(&json!(19))
                {
                    fields.insert("end_byte".into(), json!(22));
                }
                for child in fields.values_mut() {
                    extend_number_spans(child);
                }
            }
            Value::Array(children) => {
                for child in children {
                    extend_number_spans(child);
                }
            }
            _ => {}
        }
    }
    extend_number_spans(&mut data);
    for mapping in data["text_buffers"][0]["mappings"].as_array_mut().unwrap() {
        if mapping["text_range"]["end_byte"] == 19 {
            mapping["text_range"]["end_byte"] = 22.into();
        }
    }
    data["resources"]["font_faces"][0]["expected_sha256"] = hash_hex.clone().into();
    let source = data["text_buffers"][0]["utf8"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let body = body_with_source(&root, data, &source, &limits);
    fs::write(root.0.join("body.bin"), &font).unwrap();
    fs::write(root.0.join("vector.svg"),include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/x-plus-y.svg"))).unwrap();
    let input = prepare_book_v2_resources(
        body,
        &root.context(),
        &config_with_extension(
            limits.base().get().clone(),
            limits.extension().get().clone(),
        ),
        &limits,
    )
    .unwrap();
    let admitted = input
        .resources()
        .font(typaxis_core::FontFaceId::new(0))
        .unwrap();
    assert_eq!(admitted.bytes(), font);
    assert_eq!(admitted.content_hash(), hash);
    let mut budget = BookV2ColumnPageBudget::new(&limits, 1_000_000_000);
    crate::book_v2_resources::with_budgeted_book_v2_column_pages(&input,&limits,MODE,&mut budget,
        |_,search,_,prior|{
            let shaped=search.source_lines().prepared().shaped();
            let mut legacy_records=prior.record_charge();
            let legacy=shape_book_v2_equation_numbers_counted(shaped,&limits,legacy_records,&mut legacy_records).unwrap().unwrap();
            assert!(legacy.shapes().iter().all(|shape|shape.text()=="कि"));
            let mut receipts=Vec::new();
            let mut owners=Vec::new();
            for shape in legacy.shapes() {
                assert_eq!(shape.font_sha256(),hash);
                let owner=shape.source().node_id();owners.push(owner);
                let label_owner=shape.source().equation_number().unwrap().node_id();
                let glyphs=shape.runs().iter().flat_map(|r|r.glyphs()).map(|g|json!({
                    "original_gid":g.original_gid.get(),"advance_x":g.advance_x.raw(),"advance_y":g.advance_y.raw(),
                    "offset_x":g.offset_x.raw(),"offset_y":g.offset_y.raw()})).collect::<Vec<_>>();
                assert_eq!(glyphs.iter().map(|glyph|glyph["original_gid"].as_u64().unwrap()).collect::<Vec<_>>(),[65,23]);
                receipts.push(json!({"owner":owner.get(),"label_owner":label_owner.get(),"text":shape.text(),"font_sha256":hash_hex,
                    "glyphs":glyphs}));
            }
            let initial=[prior.record_charge(),11,13];
            let run=|start:[u64;3],max_r,max_w|{
                let [mut r,mut s,mut w]=start;
                let result=with_book_v2_observed_source_accounting(&mut r,&mut s,&mut w,limits.base(),max_r,max_w,
                    |a|shape_book_v2_equation_numbers_with_observed_storage(shaped,&limits,a).map(|numbers|numbers.unwrap().fingerprint()));
                (result,[r,s,w])
            };
            let full=run(initial,limits.base().get().max_fragments,1_000_000_000);
            assert_eq!(full.0,Ok(legacy.fingerprint()));assert_eq!(full.1[0],legacy_records);
            assert_eq!(run(initial,full.1[0],full.1[2]),full);
            let mut short_start=initial;
            short_start[1]=limits.base().get().max_spool_bytes-(full.1[1]-initial[1])+1;
            let spool=run(short_start,full.1[0],full.1[2]);let error=spool.0.unwrap_err();
            assert!(owners.contains(&error.owner),"{error:?}, owners={owners:?}");
            assert_eq!(error.kind,BookV2EquationNumberErrorKind::ObservedAccounting(BookV2ObservedAccountingError::Spool{owner:error.owner}));
            let work=run(initial,full.1[0],full.1[2]-1);let error=work.0.unwrap_err();
            assert!(owners.contains(&error.owner),"{error:?}, owners={owners:?}");
            assert_eq!(error.kind,BookV2EquationNumberErrorKind::ObservedAccounting(BookV2ObservedAccountingError::Work{owner:error.owner}));
            if let Some(directory)=std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
                let output=PathBuf::from(directory).join("indic-number-route");let job=output.join("job");
                fs::create_dir_all(&job).unwrap();
                for entry in fs::read_dir(&root.0).unwrap(){let entry=entry.unwrap();assert!(entry.file_type().unwrap().is_file());fs::copy(entry.path(),job.join(entry.file_name())).unwrap();}
                fs::write(output.join("number-shapes.json"),serde_json::to_vec_pretty(&json!({
                    "schema":"typaxis.admitted-indic-number-route/1","fixture_kind":"derived-test-font",
                    "admitted_font_sha256":hash_hex,"initial":initial,"full":full.1,"shapes":receipts})).unwrap()).unwrap();
            }
            eprintln!("admitted Indic numbered route: shapes={}, full={:?}",legacy.shapes().len(),full.1);
        }).unwrap();
}

#[test]
fn book_v2_column_number_storage_actual_shapes_and_blocks_keep_bytes_and_budget_prefixes() {
    let limits = changed_caps(&caps(), |c| {
        c.max_line_reshape_passes = 512;
        c.max_layout_passes = 512;
    });
    let root = Root::new();
    let input = spool_vector_input(&root, header_data(true, true, true), &limits);
    let mut budget = BookV2ColumnPageBudget::new(&limits, 100_000_000);
    crate::book_v2_resources::with_budgeted_book_v2_column_pages(
        &input,
        &limits,
        MODE,
        &mut budget,
        |_, search, _, prior| {
            let lines = search.source_lines();
            let mut legacy_records = prior.record_charge();
            let numbers = shape_book_v2_equation_numbers_counted(
                lines.prepared().shaped(),
                &limits,
                legacy_records,
                &mut legacy_records,
            )
            .unwrap()
            .unwrap();
            assert!(!numbers.shapes().is_empty());
            let blocks = prepare_book_v2_vector_blocks_counted(
                lines,
                Some(&numbers),
                &limits,
                legacy_records,
                &mut legacy_records,
            )
            .unwrap()
            .unwrap();
            let expected = (numbers.fingerprint(), blocks.fingerprint(), legacy_records);
            let initial = [prior.record_charge(), 11, 13];
            let foreign = changed_caps(&limits, |c| c.max_layout_passes += 1);
            let [mut r, mut s, mut w] = initial;
            with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                foreign.base(),
                limits.base().get().max_fragments,
                100_000_000,
                |a| {
                    assert!(shape_book_v2_equation_numbers_with_observed_storage(
                        lines.prepared().shaped(),
                        &limits,
                        a,
                    )
                    .is_err());
                    assert!(prepare_book_v2_vector_blocks_with_observed_storage(
                        lines,
                        Some(&numbers),
                        &limits,
                        a,
                    )
                    .is_err());
                },
            );
            assert_eq!([r, s, w], initial);
            let run = |start: [u64; 3], max_records, max_work| {
                let [mut r, mut s, mut w] = start;
                let result = with_book_v2_observed_source_accounting(
                    &mut r,
                    &mut s,
                    &mut w,
                    limits.base(),
                    max_records,
                    max_work,
                    |a| {
                        let numbers = shape_book_v2_equation_numbers_with_observed_storage(
                            lines.prepared().shaped(),
                            &limits,
                            a,
                        )
                        .map_err(|e| format!("{e:?}"))?
                        .unwrap();
                        let blocks = prepare_book_v2_vector_blocks_with_observed_storage(
                            lines,
                            Some(&numbers),
                            &limits,
                            a,
                        )
                        .map_err(|e| format!("{e:?}"))?
                        .unwrap();
                        Ok::<_, String>((
                            numbers.fingerprint(),
                            blocks.fingerprint(),
                            a.counters()[0],
                        ))
                    },
                );
                (result, [r, s, w])
            };
            let full = run(initial, limits.base().get().max_fragments, 100_000_000);
            assert_eq!(full.0, Ok(expected));
            assert!(full.1[1] > initial[1] && full.1[2] > initial[2]);
            assert_eq!(run(initial, full.1[0], full.1[2]), full);
            for axis in 0..3 {
                let mut start = initial;
                let mut max_r = full.1[0];
                let mut max_w = full.1[2];
                if axis == 0 {
                    max_r -= 1;
                }
                if axis == 1 {
                    start[1] = limits.base().get().max_spool_bytes - (full.1[1] - initial[1]) + 1;
                }
                if axis == 2 {
                    max_w -= 1;
                }
                let failed = run(start, max_r, max_w);
                assert!(failed.0.is_err(), "axis={axis}: {failed:?}");
                assert!(failed
                    .1
                    .iter()
                    .zip(start)
                    .all(|(after, before)| *after >= before));
                assert!(
                    failed.1[0] <= max_r
                        && failed.1[1] <= limits.base().get().max_spool_bytes
                        && failed.1[2] <= max_w
                );
            }
            let mut exact = initial;
            exact[1] = limits.base().get().max_spool_bytes - (full.1[1] - initial[1]);
            let exact = run(exact, full.1[0], full.1[2]);
            assert_eq!(exact.0, Ok(expected));
            assert_eq!(exact.1[1], limits.base().get().max_spool_bytes);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_number_storage_numbered_headers_keep_pdf_and_exact_command_spool() {
    use crate::book_v2_resources::{
        with_budgeted_converged_book_v2_column_pdf, BookV2ColumnPdfConvergenceBudget,
    };
    let baseline = changed_caps(&caps(), |c| {
        c.max_line_reshape_passes = 512;
        c.max_layout_passes = 512;
    });
    let data = header_data(true, true, true);
    let evaluate = |limits: &M4EffectiveResourceLimits, maximum| {
        let root = Root::new();
        let input = spool_vector_input(&root, data.clone(), limits);
        let mut budget = BookV2ColumnPdfConvergenceBudget::new(limits, maximum);
        let mut called = false;
        let result = with_budgeted_converged_book_v2_column_pdf(
            &input,
            limits,
            MODE,
            &mut budget,
            4,
            |roots, _| {
                called = true;
                roots.pdf().bytes().to_vec()
            },
        )
        .map_err(|e| format!("{e:?}"));
        (result, budget.observation(), called)
    };
    let full = evaluate(&baseline, 1_000_000_000);
    assert!(full.0.is_ok() && full.2, "{full:?}");
    let root = Root::new();
    let input = spool_vector_input(&root, data.clone(), &baseline);
    let mut legacy = BookV2ColumnPageBudget::new(&baseline, 1_000_000_000);
    let original = crate::book_v2_resources::with_budgeted_book_v2_column_pdf(
        &input,
        &baseline,
        MODE,
        &mut legacy,
        4,
        |roots, _, _| roots.pdf().bytes().to_vec(),
    )
    .unwrap();
    assert_eq!(full.0.as_ref().unwrap(), &original);
    if let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
        // Retain this actual numbered input/PDF pair separately from the
        // geometry corpus. The raw 1.5 input remains a private staging fixture.
        let output = PathBuf::from(directory).join("number-input-storage");
        let job = output.join("job");
        fs::create_dir_all(&job).unwrap();
        for entry in fs::read_dir(&root.0).unwrap() {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            fs::copy(entry.path(), job.join(entry.file_name())).unwrap();
        }
        fs::write(output.join("observed.pdf"), full.0.as_ref().unwrap()).unwrap();
        fs::write(output.join("legacy.pdf"), &original).unwrap();
    }
    assert_eq!(evaluate(&baseline, full.1.work_steps()), full);
    let mut retained = BookV2ColumnPdfConvergenceBudget::new(&baseline, full.1.work_steps());
    let app_result = with_budgeted_converged_book_v2_column_pdf(
        &input,
        &baseline,
        MODE,
        &mut retained,
        4,
        |_, _| Err::<(), _>("numbered-header application result"),
    )
    .unwrap();
    assert_eq!(app_result, Err("numbered-header application result"));
    assert_eq!(retained.observation(), full.1);
    let mut called_again = false;
    assert!(with_budgeted_converged_book_v2_column_pdf(
        &input,
        &baseline,
        MODE,
        &mut retained,
        4,
        |_, _| {
            called_again = true;
        },
    )
    .is_err());
    assert!(!called_again);
    assert!(retained.observation().spool_charge() >= full.1.spool_charge());
    assert!(retained.observation().record_charge() >= full.1.record_charge());
    assert_eq!(retained.observation().work_steps(), full.1.work_steps());
    let work_short = evaluate(&baseline, full.1.work_steps() - 1);
    assert!(work_short.0.is_err() && !work_short.2);
    assert!(work_short.1.spool_charge() > 0);
    let exact = evaluate(
        &spool_limits(&baseline, full.1.spool_charge()),
        1_000_000_000,
    );
    assert!(exact.0.is_ok() && exact.2, "{exact:?}");
    let short = evaluate(
        &spool_limits(&baseline, full.1.spool_charge() - 1),
        1_000_000_000,
    );
    assert!(short.0.is_err() && !short.2, "{short:?}");
    assert!(short.1.spool_charge() > 0 && short.1.spool_charge() < full.1.spool_charge());
    let historical = evaluate(&spool_limits(&baseline, 5_619), 1_000_000_000);
    assert!(historical.0.is_err() && !historical.2);
    assert!(historical.1.spool_charge() > 0 && historical.1.spool_charge() <= 5_619);
    eprintln!(
        "numbered header command: full={:?}, short={:?}, historical={historical:?}",
        full.1, short.1
    );
}
