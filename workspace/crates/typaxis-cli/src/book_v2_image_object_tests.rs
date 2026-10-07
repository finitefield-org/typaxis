use super::*;
use typaxis_pdf::book_v2::{
    BookV2ImageObjectBuilder as Objects, BookV2ImageObjectRole as Role, BookV2VectorPrograms,
};
pub(super) fn check(
    text: &typaxis_pdf::book_v2::BookV2TextCommands<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    source: &BookV2VectorPrograms<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    const MAX_WORK: u64 = 1_000_000_000;
    let run = |first, records, spool, output, work, prior_work, capture| -> Result<_, VE> {
        let mut builder = Objects::new(
            source, first, limits, work, records, spool, output, prior_work,
        )?;
        let objects = builder.build()?;
        assert!(std::ptr::eq(objects.source(), source));
        assert_eq!(objects.first_object(), first);
        assert_eq!(
            objects.next_object(),
            first + objects.objects().len() as u32
        );
        assert!(objects.for_image(usize::MAX).is_none());
        assert!(objects.resource_object(usize::MAX).is_none());
        let mut cursor = 0;
        let mut expected = Vec::new();
        let mut credit = 0;
        for (i, image) in source.source().source().images().iter().enumerate() {
            let group = objects.for_image(i).unwrap();
            assert_eq!(objects.resource_object(i), Some(group[0].id()));
            let raster = source.source().program(i);
            let vector = source.program(i);
            assert_eq!(
                group.len(),
                raster.map_or_else(
                    || 1 + vector.unwrap().states().len(),
                    |r| 1 + usize::from(r.alpha_mask().is_some())
                )
            );
            let mut details = Vec::new();
            for (part, o) in group.iter().enumerate() {
                assert_eq!(o.image_index(), i);
                assert_eq!(o.id().get(), first + expected.len() as u32);
                assert_eq!(o.byte_range().start, cursor);
                cursor = o.byte_range().end;
                let bytes = &objects.bytes()[o.byte_range()];
                assert!(bytes.starts_with(format!("{} 0 obj\n", o.id().get()).as_bytes()));
                assert!(bytes.ends_with(b"\nendobj\n"));
                let (role, payload) = if let Some(r) = raster {
                    if part == 0 {
                        (Role::Image, r.bytes())
                    } else {
                        (
                            Role::SoftMask,
                            r.alpha_mask().unwrap().encoded_bytes.as_slice(),
                        )
                    }
                } else {
                    let v = vector.unwrap();
                    if part == 0 {
                        (Role::Form, v.content())
                    } else {
                        (Role::ExtGState, v.state_dictionary(part - 1).unwrap())
                    }
                };
                assert_eq!(o.role(), role);
                assert_eq!(o.state_index(), (role == Role::ExtGState).then(|| part - 1));
                let dict = if role == Role::ExtGState {
                    let prefix = format!("{} 0 obj\n", o.id().get());
                    assert_eq!(&bytes[prefix.len()..bytes.len() - 8], payload);
                    std::str::from_utf8(payload).unwrap()
                } else {
                    let start = bytes.windows(8).position(|w| w == b"\nstream\n").unwrap() + 8;
                    let dict = std::str::from_utf8(&bytes[..start]).unwrap();
                    assert!(dict.contains(&format!("/Length {}", payload.len())));
                    assert_eq!(&bytes[start..start + payload.len()], payload);
                    assert_eq!(&bytes[start + payload.len()..], b"\nendstream\nendobj\n");
                    dict
                };
                match role {
                    Role::Image | Role::SoftMask => {
                        let r = raster.unwrap();
                        assert!(dict.contains(&format!(
                            "/Width {} /Height {}",
                            r.width(),
                            r.height()
                        )));
                        assert!(dict.contains("/BitsPerComponent 8"));
                        if role == Role::Image && r.encoding() == ImageEncoding::Jpeg {
                            assert!(dict.contains("/DCTDecode"));
                            assert!(dict.contains(&format!(
                                "/ColorTransform {}",
                                u8::from(
                                    r.color_space() == typaxis_resources::ImageColorSpace::Rgb
                                )
                            )));
                        } else {
                            assert!(dict.contains("/FlateDecode"));
                            assert!(!dict.contains("/DecodeParms"));
                        }
                        if role == Role::Image && r.alpha_mask().is_some() {
                            assert!(dict.contains(&format!("/SMask {} 0 R", group[1].id().get())));
                        } else {
                            assert!(!dict.contains("/SMask"));
                        }
                    }
                    Role::Form => {
                        let v = vector.unwrap();
                        assert!(dict.contains("/Subtype /Form /FormType 1"));
                        for (s, o) in group[1..].iter().enumerate() {
                            assert!(dict.contains(&format!("/GS{s} {} 0 R", o.id().get())));
                        }
                        credit += v.content().len();
                    }
                    Role::ExtGState => {
                        credit += payload.len();
                        assert!(dict.starts_with("<< /Type /ExtGState"));
                    }
                }
                for token in ["/Alt", "/ActualText", "/MCID", "/Lang"] {
                    assert!(!dict.contains(token));
                }
                let item = serde_json::json!({"id":o.id().get(),"image":i,"role":format!("{:?}",role),"state":o.state_index(),"payload_sha256":typaxis_core::sha256(payload).iter().map(|b|format!("{b:02x}")).collect::<String>()});
                details.push(item.clone());
                expected.push(item);
            }
            if capture {
                if let Ok(directory) = std::env::var("TYPAXIS_BOOK_IMAGE_OBJECTS_PROBE") {
                    let path = std::path::Path::new(&directory);
                    std::fs::create_dir_all(path).unwrap();
                    let key = objects
                        .fingerprint()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>();
                    std::fs::write(
                        path.join(format!("{key}.{i}.source")),
                        image.image().bytes(),
                    )
                    .unwrap();
                    let data = if let Some(r) = raster {
                        serde_json::json!({"kind":"raster","width":r.width(),"height":r.height(),"color":format!("{:?}",r.color_space()),"encoding":format!("{:?}",r.encoding()),"alpha":r.alpha_mask().is_some()})
                    } else {
                        let v = vector.unwrap();
                        let ir = match v.ir() {
                            AdmittedSafeVector::V1(v) => v.canonical_jcs(),
                            AdmittedSafeVector::V2(v) => v.canonical_jcs(),
                        };
                        serde_json::json!({"kind":"vector","bbox":v.bbox(),"ir":serde_json::from_str::<serde_json::Value>(ir).unwrap(),"states":v.states().iter().map(|s|(s.fill_alpha_raw(),s.stroke_alpha_raw())).collect::<Vec<_>>()})
                    };
                    std::fs::write(path.join(format!("{key}.{i}.json")),serde_json::to_vec(&serde_json::json!({"image":i,"objects":details,"source_sha256":image.image().content_hash().iter().map(|b|format!("{b:02x}")).collect::<String>(),"data":data})).unwrap()).unwrap();
                }
            }
        }
        assert_eq!(cursor, objects.bytes().len());
        assert_eq!(objects.objects().len(), expected.len());
        assert_eq!(
            objects.output_charge() - output.max(source.output_charge()),
            (objects.bytes().len() - credit) as u64
        );
        if capture {
            if let Ok(directory) = std::env::var("TYPAXIS_BOOK_IMAGE_OBJECTS_PROBE") {
                assert_eq!(first, 4);
                let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
                let mut offsets = vec![0];
                for (id, body) in [
                    (1, "<< /Type /Catalog /Pages 2 0 R >>"),
                    (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                    (
                        3,
                        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>",
                    ),
                ] {
                    offsets.push(pdf.len());
                    pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
                }
                let start = pdf.len();
                offsets.extend(
                    objects
                        .objects()
                        .iter()
                        .map(|o| start + o.byte_range().start),
                );
                pdf.extend_from_slice(objects.bytes());
                let xref = pdf.len();
                pdf.extend_from_slice(
                    format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes(),
                );
                for offset in &offsets[1..] {
                    pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
                }
                pdf.extend_from_slice(
                    format!(
                        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                        offsets.len()
                    )
                    .as_bytes(),
                );
                let key = objects
                    .fingerprint()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                let path = std::path::Path::new(&directory);
                std::fs::create_dir_all(path).unwrap();
                std::fs::write(path.join(format!("{key}.pdf")), pdf).unwrap();
            }
        }
        if capture {
            commands::check(text, &objects, limits);
        }
        Ok((
            objects.record_charge(),
            objects.spool_charge(),
            objects.output_charge(),
            objects.work_steps(),
            objects.fingerprint(),
            objects.bytes().to_vec(),
            objects.objects().len(),
        ))
    };
    let expected = run(4, 0, 0, 0, MAX_WORK, 0, true).unwrap();
    assert_eq!(run(4, 0, 0, 0, expected.3, 0, false).unwrap(), expected);
    let br = source.record_charge();
    let bs = source.spool_charge();
    let bo = source.output_charge();
    let pr = limits.base().get().max_fragments - (expected.0 - br);
    let ps = limits.base().get().max_spool_bytes - (expected.1 - bs);
    let po = limits.base().get().max_output_bytes - (expected.2 - bo);
    let exact = run(4, pr, ps, po, expected.3, 0, false).unwrap();
    assert_eq!(
        (exact.0, exact.1, exact.2),
        (
            limits.base().get().max_fragments,
            limits.base().get().max_spool_bytes,
            limits.base().get().max_output_bytes
        )
    );
    assert_eq!((&exact.4, &exact.5), (&expected.4, &expected.5));
    assert!(matches!(
        run(4, pr + 1, ps, po, expected.3, 0, false),
        Err(VE::Records)
    ));
    if expected.1 > bs {
        assert!(matches!(
            run(4, pr, ps + 1, po, expected.3, 0, false),
            Err(VE::Spool)
        ));
    }
    assert!(matches!(
        run(4, pr, ps, po + 1, expected.3, 0, false),
        Err(VE::Output)
    ));
    assert!(matches!(
        run(4, 0, 0, 0, expected.3 - 1, 0, false),
        Err(VE::Work)
    ));
    assert!(matches!(
        run(0, 0, 0, 0, MAX_WORK, 0, false),
        Err(VE::Objects)
    ));
    assert!(matches!(
        run(u32::MAX, 0, 0, 0, MAX_WORK, 0, false),
        Err(VE::Objects)
    ));
    let last = limits.base().get().max_pdf_objects - expected.6 as u32 + 1;
    let edge = run(last, 0, 0, 0, MAX_WORK, 0, false).unwrap();
    assert_eq!(edge.6, expected.6);
    assert!(matches!(
        run(last + 1, 0, 0, 0, MAX_WORK, 0, false),
        Err(VE::Objects)
    ));
    assert_ne!(run(5, 0, 0, 0, MAX_WORK, 0, false).unwrap().4, expected.4);
    let shifted = run(4, 0, 0, 0, MAX_WORK, source.work_steps() + 17, false).unwrap();
    assert_eq!(shifted.3, expected.3 + 17);
    assert_eq!((&shifted.4, &shifted.5), (&expected.4, &expected.5));
    let mut builder = Objects::new(source, 4, limits, MAX_WORK, 0, 0, 0, 0).unwrap();
    let first = builder.build().unwrap();
    let second = builder.build().unwrap();
    assert_eq!(first.bytes(), second.bytes());
    assert_eq!(first.fingerprint(), second.fingerprint());
    assert_eq!(
        second.output_charge() - first.output_charge(),
        second.bytes().len() as u64
    );
    if !expected.5.is_empty() {
        let mut lo = Objects::new(source, 4, limits, MAX_WORK, 0, 0, 0, 0)
            .unwrap()
            .work_steps();
        let mut hi = expected.3;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let mut candidate = Objects::new(source, 4, limits, mid, 0, 0, 0, 0).unwrap();
            let _ = candidate.build();
            if candidate.output_charge() > bo {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        let mut before = Objects::new(source, 4, limits, lo - 1, 0, 0, 0, 0).unwrap();
        assert!(matches!(before.build(), Err(VE::Work)));
        assert_eq!(before.output_charge(), bo);
        let mut after = Objects::new(source, 4, limits, lo, 0, 0, 0, 0).unwrap();
        assert!(matches!(after.build(), Err(VE::Work)));
        let reserved = (
            after.record_charge(),
            after.spool_charge(),
            after.output_charge(),
        );
        assert!(reserved.2 > bo);
        assert!(after.build().is_err());
        assert!(after.record_charge() >= reserved.0);
        assert!(after.spool_charge() >= reserved.1);
        assert_eq!(after.output_charge(), reserved.2);
    }
    let mut changed = limits.base().get().clone();
    changed.max_output_bytes -= 1;
    let changed = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(changed).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    assert!(matches!(
        Objects::new(source, 4, &changed, MAX_WORK, 0, 0, 0, 0),
        Err(VE::Identity)
    ));
}

#[path = "book_v2_image_command_tests.rs"]
mod commands;
