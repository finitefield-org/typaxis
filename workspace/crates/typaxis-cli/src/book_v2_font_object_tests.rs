use super::*;
#[path = "book_v2_text_command_tests.rs"]
mod text;
use typaxis_pdf::book_v2::{
    BookV2FontObjectBuilder as Objects, BookV2FontObjectRole as Role, BookV2FontStreams,
};

pub(super) fn check(
    source: &BookV2FontStreams<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    const MAX_WORK: u64 = 1_000_000_000;
    let mut original = Objects::new(source, 4, limits, MAX_WORK, 0, 0, 0, 0).unwrap();
    let objects = original.build().unwrap();
    text::check(&objects, limits);
    assert!(std::ptr::eq(objects.source(), source));
    assert_eq!(objects.objects().len(), 6 * source.fonts().len());
    assert_eq!(objects.next_object(), 4 + objects.objects().len() as u32);
    assert!(objects.font_object(usize::MAX).is_none());
    let mut cursor = 0;
    let mut credited = 0;
    let mut expected = Vec::new();
    for (i, object) in objects.objects().iter().enumerate() {
        assert_eq!(object.id().get(), 4 + i as u32);
        assert_eq!(object.font_index(), i / 6);
        assert_eq!(
            object.role(),
            [
                Role::Type0,
                Role::CidFont,
                Role::Descriptor,
                Role::Program,
                Role::ToUnicode,
                Role::Auxiliary
            ][i % 6]
        );
        let range = object.byte_range();
        assert_eq!(range.start, cursor);
        cursor = range.end;
        let bytes = &objects.bytes()[range];
        assert!(bytes.starts_with(format!("{} 0 obj\n", object.id().get()).as_bytes()));
        assert!(bytes.ends_with(b"\nendobj\n"));
        let font = source.source().fonts()[i / 6].source();
        let cff = font.kind() == K::Cff1V2;
        match object.role() {
            Role::Type0 | Role::CidFont | Role::Descriptor => {
                let text = std::str::from_utf8(bytes).unwrap();
                assert!(text.contains(font.postscript_name()));
                match object.role() {
                    Role::Type0 => {
                        assert!(text.contains("/Subtype /Type0"));
                        assert!(text.contains(&format!(
                            "/DescendantFonts [{} 0 R]",
                            object.id().get() + 1
                        )));
                    }
                    Role::CidFont => {
                        assert!(text.contains(if cff {
                            "/Subtype /CIDFontType0"
                        } else {
                            "/Subtype /CIDFontType2"
                        }));
                        assert_eq!(text.contains("/CIDToGIDMap"), !cff);
                    }
                    Role::Descriptor => {
                        assert_eq!(text.contains("/CIDSet"), cff);
                        assert_eq!(text.contains("/FontFile3"), cff);
                        assert_eq!(text.contains("/FontFile2"), !cff);
                        assert!(text.contains(&format!("/Ascent {}", font.metrics().ascent_1000)));
                    }
                    _ => unreachable!(),
                }
            }
            role => {
                let payload = match role {
                    Role::Program => font.bytes(),
                    Role::ToUnicode => source.to_unicode(i / 6).unwrap(),
                    Role::Auxiliary => source.auxiliary(i / 6).unwrap(),
                    _ => unreachable!(),
                };
                let start = bytes.windows(8).position(|s| s == b"\nstream\n").unwrap() + 8;
                let dict = std::str::from_utf8(&bytes[..start]).unwrap();
                assert!(dict.contains(&format!("/Length {}", payload.len())));
                assert_eq!(&bytes[start..start + payload.len()], payload);
                assert_eq!(&bytes[start + payload.len()..], b"\nendstream\nendobj\n");
                if role == Role::Program {
                    assert_eq!(dict.contains("/Subtype /OpenType"), cff);
                    assert_eq!(dict.contains("/Length1"), !cff);
                } else {
                    credited += payload.len();
                }
            }
        }
        if i % 6 == 0 {
            let m = font.metrics();
            expected.push(serde_json::json!({
                "id":object.id().get(),"cff":cff,"name":font.postscript_name(),
                "program_sha256":font.sha256().iter().map(|b|format!("{b:02x}")).collect::<String>(),
                "notdef_width":if cff {font.advance(typaxis_font::OriginalGlyphId::new(0)).map(u32::from)} else {None},
                "widths":source.source().bindings(i/6).unwrap().iter().map(|b|(b.cid().get(),b.width_1000())).collect::<Vec<_>>(),
                "bbox":m.bbox_1000,"ascent":m.ascent_1000,"descent":m.descent_1000,"flags":m.flags,
                "to_unicode":source.to_unicode(i/6).unwrap(),"auxiliary":source.auxiliary(i/6).unwrap()
            }));
        }
    }
    assert_eq!(cursor, objects.bytes().len());
    assert_eq!(
        objects.output_charge() - source.output_charge(),
        (objects.bytes().len() - credited) as u64
    );
    // Optional independently parsed probes are test artifacts, not production
    // book-2 PDFs. They contain font objects and an empty untagged page only.
    if let Ok(directory) = std::env::var("TYPAXIS_BOOK_FONT_OBJECTS_PROBE") {
        let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = vec![0usize];
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
        std::fs::write(
            path.join(format!("{key}.json")),
            serde_json::to_vec(&expected).unwrap(),
        )
        .unwrap();
    }
    let measured = (
        objects.record_charge(),
        objects.spool_charge(),
        objects.output_charge(),
        objects.work_steps(),
        objects.fingerprint(),
    );
    let run = |first, records, spool, output, work, prior_work| -> Result<_, Error> {
        let mut b = Objects::new(
            source, first, limits, work, records, spool, output, prior_work,
        )?;
        let p = b.build()?;
        assert_eq!(
            (
                p.record_charge(),
                p.spool_charge(),
                p.output_charge(),
                p.work_steps()
            ),
            (
                b.record_charge(),
                b.spool_charge(),
                b.output_charge(),
                b.work_steps()
            )
        );
        Ok((
            p.record_charge(),
            p.spool_charge(),
            p.output_charge(),
            p.work_steps(),
            p.fingerprint(),
        ))
    };
    let (records, spool, output, work, fp) = measured;
    assert_eq!(run(4, 0, 0, 0, work, 0).unwrap(), measured);
    let base = limits.base().get();
    let pr = base.max_fragments - (records - source.record_charge());
    let ps = base.max_spool_bytes - (spool - source.spool_charge());
    let po = base.max_output_bytes - (output - source.output_charge());
    assert_eq!(
        run(4, pr, ps, po, work, 0).unwrap(),
        (
            base.max_fragments,
            base.max_spool_bytes,
            base.max_output_bytes,
            work,
            fp
        )
    );
    assert!(matches!(
        run(4, pr + 1, ps, po, work, 0),
        Err(Error::Records)
    ));
    assert!(matches!(run(4, pr, ps + 1, po, work, 0), Err(Error::Spool)));
    assert!(matches!(
        run(4, pr, ps, po + 1, work, 0),
        Err(Error::Output)
    ));
    assert!(matches!(run(4, 0, 0, 0, work - 1, 0), Err(Error::Work)));
    assert_eq!(
        run(4, 0, 0, 0, work + 17, source.work_steps() + 17).unwrap(),
        (records, spool, output, work + 17, fp)
    );
    assert_ne!(run(5, 0, 0, 0, MAX_WORK, 0).unwrap().4, fp);
    assert!(matches!(run(0, 0, 0, 0, MAX_WORK, 0), Err(Error::Objects)));
    let last = base.max_pdf_objects + 1 - objects.objects().len() as u32;
    assert!(run(last, 0, 0, 0, MAX_WORK, 0).is_ok());
    assert!(matches!(
        run(last + 1, 0, 0, 0, MAX_WORK, 0),
        Err(Error::Objects)
    ));
    assert!(
        matches!(run(u32::MAX, 0, 0, 0, MAX_WORK, 0), Err(Error::Objects))
            || objects.objects().is_empty()
    );
    let second = original.build().unwrap();
    assert_eq!(second.fingerprint(), fp);
    assert_eq!(second.bytes(), objects.bytes());
    assert_eq!(
        second.output_charge() - output,
        objects.bytes().len() as u64
    );
    assert_eq!(second.spool_charge() - spool, spool - source.spool_charge());
    let mut failed = Objects::new(source, 4, limits, work - 1, 0, 0, 0, 0).unwrap();
    assert!(matches!(failed.build(), Err(Error::Work)));
    assert_eq!(
        (
            failed.record_charge(),
            failed.spool_charge(),
            failed.output_charge(),
            failed.work_steps()
        ),
        (records, spool, output, work - 1)
    );
    assert!(matches!(failed.build(), Err(Error::Work)));
    // The second attempt cannot regain the consumed source-output credit.
    assert!(failed.output_charge() >= output && failed.spool_charge() >= spool);
    assert_eq!(failed.work_steps(), work - 1);
    let mut lower = source.work_steps();
    let mut upper = work - 1;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let mut probe = Objects::new(source, 4, limits, middle, 0, 0, 0, 0).unwrap();
        assert!(matches!(probe.build(), Err(Error::Work)));
        if probe.record_charge() == records {
            upper = middle;
        } else {
            assert_eq!(probe.record_charge(), source.record_charge() + 1);
            lower = middle + 1;
        }
    }
    let mut reserved = Objects::new(source, 4, limits, lower, 0, 0, 0, 0).unwrap();
    assert!(matches!(reserved.build(), Err(Error::Work)));
    assert_eq!(
        (
            reserved.record_charge(),
            reserved.spool_charge(),
            reserved.output_charge()
        ),
        (records, spool, output)
    );
    assert!(matches!(reserved.build(), Err(Error::Work)));
    assert!(
        reserved.record_charge() >= records
            && reserved.spool_charge() >= spool
            && reserved.output_charge() >= output
    );
    if lower > source.work_steps() {
        let mut before = Objects::new(source, 4, limits, lower - 1, 0, 0, 0, 0).unwrap();
        assert!(matches!(before.build(), Err(Error::Work)));
        assert_eq!(
            (
                before.record_charge(),
                before.spool_charge(),
                before.output_charge()
            ),
            (
                source.record_charge() + 1,
                source.spool_charge(),
                source.output_charge()
            )
        );
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
        Err(Error::Identity)
    ));
}
