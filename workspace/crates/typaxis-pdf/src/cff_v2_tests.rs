use super::*;
use typaxis_core::{
    BidiLevel, FontFaceId, FontInstanceId, GlyphRunId, Length, M4ResourceLimits, OpenTypeTag,
    PositiveLength, ResourceLimits, TextBufferId, TextSpan, Utf8ByteOffset,
};
use typaxis_font::admit_sfnt_cff1_v2;
use typaxis_resources::{freeze_cff1_pdf_fonts_v2, Cff1PdfFontInputV2};
use typaxis_shaping::{shape_cff1_run_v2, Cff1ShapeInputV2, ShapeSourceSpan};

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn cff_v2_original_pdf_font_objects_and_exact_text() {
    let bytes: std::sync::Arc<[u8]> = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap())
        .unwrap()
        .into();
    assert_eq!(
        sha256(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let limits = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap();
    let admission = admit_sfnt_cff1_v2(bytes, 0, &limits).unwrap();
    let (base, vs) = (0x4e00..=0x9fff)
        .filter_map(char::from_u32)
        .find_map(|base| {
            let gid = admission.cmap().glyph_for_sequence(base, None)?;
            (0xe0100..=0xe010f)
                .filter_map(char::from_u32)
                .find(|&vs| admission.cmap().glyph_for_sequence(base, Some(vs)) == Some(gid))
                .map(|vs| (base, vs))
        })
        .unwrap();
    let text = format!("A{base}{base}{vs}日本語");
    let shaped = shape_cff1_run_v2(
        &admission,
        Cff1ShapeInputV2 {
            run_id: GlyphRunId::new(1),
            font: FontInstanceId::new(1),
            source: ShapeSourceSpan::Parsed(
                TextSpan::new(
                    TextBufferId::new(1),
                    Utf8ByteOffset::new(0),
                    Utf8ByteOffset::new(text.len() as u32),
                )
                .unwrap(),
            ),
            utf8: &text,
            font_size: PositiveLength::new(Length::from_raw(11 * 65536).unwrap()).unwrap(),
            bidi_level: BidiLevel::LTR,
            script: OpenTypeTag::new(*b"Hani").unwrap(),
            language: Some("ja"),
            pre_context: None,
            post_context: None,
        },
    )
    .unwrap();
    let runs = [&shaped];
    let plans = freeze_cff1_pdf_fonts_v2(&[Cff1PdfFontInputV2 {
        font_face_id: FontFaceId::new(1),
        font_instance_id: FontInstanceId::new(1),
        admission: &admission,
        runs: &runs,
    }])
    .unwrap();
    let plan = &plans[0];
    let objects = encode_cff1_pdf_objects_v2(plan, ObjectId::new(5).unwrap(), &limits).unwrap();
    assert_eq!(objects.plan_fingerprint(), plan.fingerprint());
    for (i, &offset) in objects.offsets().iter().enumerate() {
        assert!(
            objects.bytes()[offset as usize..].starts_with(format!("{} 0 obj\n", i + 5).as_bytes())
        );
    }
    let ascii = String::from_utf8_lossy(objects.bytes());
    assert!(ascii.contains("/Subtype /CIDFontType0"));
    assert!(ascii.contains("/FontFile3 8 0 R /CIDSet 10 0 R"));
    assert!(ascii.contains("/Subtype /OpenType"));
    assert!(!ascii.contains("/CIDToGIDMap"));
    assert!(objects
        .bytes()
        .windows(plan.subset().bytes().len())
        .any(|b| b == plan.subset().bytes()));
    assert_eq!(
        objects.bytes(),
        encode_cff1_pdf_objects_v2(plan, ObjectId::new(5).unwrap(), &limits)
            .unwrap()
            .bytes()
    );
    assert!(matches!(
        encode_cff1_pdf_objects_v2(plan, ObjectId::new(u32::MAX).unwrap(), &limits),
        Err(PdfError::ObjectCountOverflow)
    ));
    assert!(matches!(
        encode_cff1_pdf_objects_v2(
            plan,
            ObjectId::new(limits.base().get().max_pdf_objects - 4).unwrap(),
            &limits
        ),
        Err(PdfError::ObjectLimit)
    ));
    let mut other_base = ResourceLimits::default();
    other_base.max_pdf_objects -= 1;
    let other = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(other_base).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        encode_cff1_pdf_objects_v2(plan, ObjectId::new(5).unwrap(), &other),
        Err(PdfError::ResourcePlanMismatch)
    ));
    // Diagnostic one-page assembly to allow an independent PDF reader to
    // inspect these exact objects. This is not a VerifiedPdfBytesReceipt.
    let mut content = LimitedPdfBuffer::new(100_000);
    content.extend(b"BT /F1 11 Tf 20 100 Td\n").unwrap();
    for cluster in plan.clusters() {
        if cluster.requires_actual_text() {
            content.extend(b"/Span << /ActualText ").unwrap();
            write_utf16be_hex(&mut content, cluster.exact_text().chars(), true).unwrap();
            content.extend(b" >> BDC\n").unwrap();
        }
        let codes: Vec<_> = cluster
            .cids()
            .iter()
            .flat_map(|c| c.get().to_be_bytes())
            .collect();
        write_hex_string(&mut content, &codes).unwrap();
        content.extend(b" Tj\n").unwrap();
        if cluster.requires_actual_text() {
            content.extend(b"EMC\n").unwrap();
        }
    }
    content.extend(b"ET\n").unwrap();
    let content = content.into_bytes();
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0u64];
    let bodies=[b"<< /Type /Catalog /Pages 2 0 R >>".as_slice(),b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>",b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 150] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"];
    for (i, body) in bodies.iter().enumerate() {
        offsets.push(pdf.len() as u64);
        pdf.extend(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend(*body);
        pdf.extend(b"\nendobj\n");
    }
    offsets.push(pdf.len() as u64);
    pdf.extend(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend(content);
    pdf.extend(b"\nendstream\nendobj\n");
    let start = pdf.len() as u64;
    offsets.extend(objects.offsets().iter().map(|o| o + start));
    pdf.extend(objects.bytes());
    let xref = pdf.len();
    pdf.extend(b"xref\n0 11\n0000000000 65535 f \n");
    for &offset in &offsets[1..] {
        pdf.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend(
        format!("trailer\n<< /Size 11 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    if let Ok(path) = std::env::var("TYPAXIS_CFF_V2_PDF_OUTPUT") {
        std::fs::write(&path, &pdf).unwrap();
        std::fs::write(format!("{path}.txt"), &text).unwrap();
    }
}
