use super::*;
#[path = "book_v2_source_structure_tests.rs"]
mod structure;
use typaxis_pdf::book_v2::{
    BookV2ImageCommandBuilder, BookV2ImageObjectBuilder, BookV2MarkedContentBuilder as Content,
    BookV2MarkedScopes, BookV2SemanticAnchorRole, BookV2TextCommands,
};
pub(super) fn check(
    text: &BookV2TextCommands<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    prior_scopes: &BookV2MarkedScopes<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    const WORK: u64 = 1_000_000_000;
    let fonts = text.source();
    let old_images = prior_scopes.source().source();
    // Independently produced object fragments deliberately start at 4 in their
    // own tests. Real joined content must reject overlapping actual IDs.
    if !fonts.objects().is_empty() && !old_images.objects().is_empty() {
        assert!(matches!(
            Content::new(prior_scopes, text, limits, WORK, 0, 0, 0, 0),
            Err(VE::Objects)
        ));
    }
    let mut objects = BookV2ImageObjectBuilder::new(
        old_images.source(),
        fonts.next_object(),
        limits,
        WORK,
        0,
        0,
        0,
        0,
    )
    .unwrap();
    let images = objects.build().unwrap();
    let mut command_builder =
        BookV2ImageCommandBuilder::new(&images, limits, WORK, 0, 0, 0, 0).unwrap();
    let commands = command_builder.build().unwrap();
    let mut scope_builder = Scopes::new(&commands, limits, WORK, 0, 0, 0, 0).unwrap();
    let scopes = scope_builder.build().unwrap();
    let display = text.display();
    assert!(std::ptr::eq(display, scopes.display()));
    let mut credit = text.byte_length() + commands.bytes().len();
    for i in 0..text.commands().len() {
        credit += text.actual_text(i).map_or(0, |b| b.len());
    }
    for i in 0..scopes.groups().len() {
        credit += scopes.begin_bytes(i).unwrap().len() + scopes.end_bytes(i).unwrap().len();
    }
    let run = |records, spool, output, work, prior_work, capture| -> Result<_, VE> {
        let mut builder = Content::new(
            &scopes, text, limits, work, records, spool, output, prior_work,
        )?;
        let content = builder.build()?;
        assert!(std::ptr::eq(content.source(), &scopes));
        assert!(std::ptr::eq(content.text(), text));
        assert!(content.page_bytes(usize::MAX).is_none());
        assert!(content.group_bytes(usize::MAX).is_none());
        assert_eq!(
            content.pages().len(),
            display.source().source().geometry().pages().len()
        );
        let mut group_cursor = 0;
        let mut total_bytes = 0;
        let mut text_cursor = 0;
        let mut image_cursor = 0;
        let mut oracle = Vec::new();
        for (page, p) in content.pages().iter().enumerate() {
            assert_eq!(p.page_index(), page as u32);
            assert_eq!(p.groups().start, group_cursor);
            let mut wanted = b"q\n".to_vec();
            for group in p.groups() {
                assert_eq!(group, group_cursor);
                group_cursor += 1;
                let g = &scopes.groups()[group];
                assert_eq!(g.page_index(), page as u32);
                let encoded = content.group_bytes(group).unwrap();
                assert!(encoded.starts_with(b"q\n"));
                assert!(encoded.ends_with(b"EMC\nQ\n"));
                let source_begin = scopes.begin_bytes(group).unwrap();
                assert_eq!(&encoded[2..2 + source_begin.len()], source_begin);
                let mut raw = Vec::new();
                let mut original = String::new();
                for paint in g.paints() {
                    match display.paints()[paint] {
                        Paint::Text(i) => original.push_str(display.text().draws()[i].exact_text()),
                        Paint::Marker(i) => {
                            original.push_str(display.markers().draws()[i].source().utf8())
                        }
                        Paint::EquationNumber(i) => {
                            original.push_str(display.numbers().draws()[i].shape().text())
                        }
                        _ => (),
                    }
                    for c in text.for_paint(paint).unwrap() {
                        assert!(std::ptr::eq(c, &text.commands()[text_cursor]));
                        raw.extend_from_slice(text.command_bytes(text_cursor).unwrap());
                        text_cursor += 1;
                    }
                    if commands
                        .commands()
                        .get(image_cursor)
                        .is_some_and(|c| c.paint_index() == paint)
                    {
                        raw.extend_from_slice(commands.command_bytes(image_cursor).unwrap());
                        raw.push(b'\n');
                        image_cursor += 1;
                    }
                }
                let replacement = if g.artifact().is_none()
                    && matches!(g.role(), R::Text | R::Label | R::EquationNumber)
                {
                    Some(original)
                } else {
                    None
                };
                let occurrence = content.anchors().iter().find(|a| a.group_index() == group);
                let expected_anchor = if scopes.actual_text(group).is_some() {
                    match display.paints()[g.paints().start] {
                        Paint::Math(i) => {
                            if matches!(
                                display.math().draws()[i].paint(),
                                BookV2MathPaint::Vector(_)
                            ) {
                                let t = display.math().draws()[i].terminal();
                                Some((t.viewport().unwrap(), t.baseline()))
                            } else {
                                None
                            }
                        }
                        Paint::Image(i) => {
                            let d = &display.images().draws()[i];
                            Some((
                                d.paint().viewport(),
                                d.fragment().fragment().baseline().unwrap(),
                            ))
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                assert_eq!(
                    occurrence.map(|a| (a.viewport(), a.baseline())),
                    expected_anchor
                );
                if let Some(a) = occurrence {
                    assert_eq!(a.page_index(), page as u32);
                    assert_eq!(a.paint_index(), g.paints().start);
                }
                // The only replacements are one whole selected text/label/
                // number or the parent Formula/vector. Never per native glyph.
                let count = encoded
                    .windows(b"/ActualText".len())
                    .filter(|b| *b == b"/ActualText")
                    .count();
                assert_eq!(
                    count,
                    usize::from(replacement.is_some() || scopes.actual_text(group).is_some())
                );
                assert_eq!(
                    encoded
                        .windows(b"/MCID".len())
                        .filter(|b| *b == b"/MCID")
                        .count(),
                    usize::from(g.mcid().is_some())
                );
                let anchor = expected_anchor.map(|(v, b)| {
                    [
                        v.width().get().raw(),
                        0,
                        0,
                        -v.height().get().raw(),
                        v.x().raw(),
                        b.raw(),
                    ]
                });
                oracle.push(serde_json::json!({"page":page,"group":group,"mcid":g.mcid(),"artifact":g.artifact().is_some(),"begin":String::from_utf8(source_begin.to_vec()).unwrap(),"replacement":replacement,"parent_actual_text":scopes.actual_text(group),"raw_commands":String::from_utf8(raw).unwrap(),"anchor_matrix":anchor}));
                wanted.extend_from_slice(encoded);
            }
            wanted.extend_from_slice(b"Q\n");
            assert_eq!(content.page_bytes(page).unwrap(), wanted);
            total_bytes += wanted.len();
        }
        assert_eq!(total_bytes, content.byte_length());
        assert_eq!(group_cursor, scopes.groups().len());
        assert_eq!(text_cursor, text.commands().len());
        assert_eq!(image_cursor, commands.commands().len());
        assert_eq!(
            content.anchor_objects().len(),
            if content.anchors().is_empty() { 0 } else { 3 }
        );
        let mut offset = 0;
        let first = fonts.next_object().max(images.next_object());
        for (i, o) in content.anchor_objects().iter().enumerate() {
            assert_eq!(o.id().get(), first + i as u32);
            assert_eq!(o.byte_range().start, offset);
            offset = o.byte_range().end;
            assert_eq!(
                o.role(),
                [
                    BookV2SemanticAnchorRole::Font,
                    BookV2SemanticAnchorRole::Glyph,
                    BookV2SemanticAnchorRole::ToUnicode
                ][i]
            );
            let b = &content.anchor_object_bytes()[o.byte_range()];
            assert!(b.starts_with(format!("{} 0 obj\n", o.id().get()).as_bytes()));
            assert!(b.ends_with(b"\nendobj\n"));
        }
        assert_eq!(offset, content.anchor_object_bytes().len());
        assert_eq!(
            content.next_object(),
            first + content.anchor_objects().len() as u32
        );
        assert_eq!(
            content.anchor_font_object(),
            content.anchor_objects().first().map(|o| o.id())
        );
        let size = content.byte_length() + content.anchor_object_bytes().len();
        assert_eq!(
            content.output_charge() + credit as u64 - output.max(scopes.output_charge()),
            size as u64
        );
        if capture {
            structure::check(&content, limits);
            if let Ok(dir) = std::env::var("TYPAXIS_BOOK_MARKED_CONTENT_PROBE") {
                std::fs::create_dir_all(&dir).unwrap();
                let name = content
                    .fingerprint()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                let mut objects = BTreeMap::<u32, Vec<u8>>::new();
                for o in fonts.objects() {
                    objects.insert(o.id().get(), fonts.bytes()[o.byte_range()].to_vec());
                }
                for o in images.objects() {
                    objects.insert(o.id().get(), images.bytes()[o.byte_range()].to_vec());
                }
                for o in content.anchor_objects() {
                    objects.insert(
                        o.id().get(),
                        content.anchor_object_bytes()[o.byte_range()].to_vec(),
                    );
                }
                let page_ids = (0..content.pages().len())
                    .map(|i| {
                        if i == 0 {
                            3
                        } else {
                            content.next_object() + i as u32 - 1
                        }
                    })
                    .collect::<Vec<_>>();
                let first_content = content.next_object() + page_ids.len() as u32 - 1;
                let selection = text
                    .source()
                    .source()
                    .source()
                    .source()
                    .source()
                    .selection();
                let font_names = selection
                    .fonts()
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        format!(
                            " /PB{} {} 0 R",
                            f.instance().font_instance_id().get(),
                            fonts.font_object(i).unwrap().get()
                        )
                    })
                    .collect::<String>();
                let image_names = (0..images.source().source().source().images().len())
                    .map(|i| format!(" /BI{i} {} 0 R", images.resource_object(i).unwrap().get()))
                    .collect::<String>();
                let anchor_name = content
                    .anchor_font_object()
                    .map(|o| format!(" /BMA {} 0 R", o.get()))
                    .unwrap_or_default();
                let resources = format!(
                    "<< /Font <<{font_names}{anchor_name} >> /XObject <<{image_names} >> >>"
                );
                let mut put = |id: u32, body: Vec<u8>| {
                    let mut bytes = format!("{id} 0 obj\n").into_bytes();
                    bytes.extend(body);
                    bytes.extend_from_slice(b"\nendobj\n");
                    assert!(objects.insert(id, bytes).is_none());
                };
                put(1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
                put(
                    2,
                    format!(
                        "<< /Type /Pages /Count {} /Kids [{}] >>",
                        page_ids.len(),
                        page_ids
                            .iter()
                            .map(|i| format!("{i} 0 R "))
                            .collect::<String>()
                    )
                    .into_bytes(),
                );
                for (i, id) in page_ids.iter().enumerate() {
                    put(*id,format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources {resources} /Contents {} 0 R >>",first_content+i as u32).into_bytes());
                    // Explicit inspection envelope: preserves actual page
                    // assignment, but is not source MediaBox/structure proof.
                    let mut page = b"q\n1 0 0 -1 0 792 cm\n".to_vec();
                    page.extend_from_slice(content.page_bytes(i).unwrap());
                    page.extend_from_slice(b"Q\n");
                    let mut stream = format!("<< /Length {} >>\nstream\n", page.len()).into_bytes();
                    stream.extend(page);
                    stream.extend_from_slice(b"\nendstream");
                    put(first_content + i as u32, stream);
                }
                let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
                let mut offsets = vec![0];
                for (id, bytes) in objects {
                    assert_eq!(id as usize, offsets.len());
                    offsets.push(pdf.len());
                    pdf.extend(bytes);
                }
                let xref = pdf.len();
                pdf.extend_from_slice(
                    format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes(),
                );
                for o in &offsets[1..] {
                    pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
                }
                pdf.extend_from_slice(
                    format!(
                        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                        offsets.len()
                    )
                    .as_bytes(),
                );
                let path = std::path::Path::new(&dir);
                std::fs::write(path.join(format!("{name}.pdf")), pdf).unwrap();
                std::fs::write(path.join(format!("{name}.json")),serde_json::to_vec_pretty(&serde_json::json!({"pages":page_ids.len(),"anchor_font":content.anchor_font_object().map(|o|o.get()),"groups":oracle})).unwrap()).unwrap();
            }
        }
        let result = (
            content.record_charge(),
            content.spool_charge(),
            content.output_charge(),
            content.work_steps(),
            content.fingerprint(),
            size,
        );
        if capture {
            let repeated = builder.build()?;
            assert_eq!(repeated.fingerprint(), content.fingerprint());
            assert_eq!(
                repeated.output_charge() - content.output_charge(),
                size as u64
            );
        }
        Ok(result)
    };
    let expected = run(0, 0, 0, WORK, 0, true).unwrap();
    let base = limits.base().get();
    let pr = base.max_fragments - (expected.0 - scopes.record_charge());
    let ps = base.max_spool_bytes - (expected.1 - scopes.spool_charge());
    let net = expected.2 as i128 - scopes.output_charge() as i128;
    let po = (base.max_output_bytes as i128 - net) as u64;
    let exact = run(pr, ps, po, expected.3, 0, false).unwrap();
    assert_eq!(
        (exact.0, exact.1, exact.2),
        (
            base.max_fragments,
            base.max_spool_bytes,
            base.max_output_bytes
        )
    );
    assert_eq!(exact.4, expected.4);
    assert!(matches!(
        run(pr + 1, ps, po, expected.3, 0, false),
        Err(VE::Records)
    ));
    assert!(matches!(
        run(pr, ps + 1, po, expected.3, 0, false),
        Err(VE::Spool)
    ));
    assert!(matches!(
        run(pr, ps, po + 1, expected.3, 0, false),
        Err(VE::Output)
    ));
    assert!(matches!(
        run(0, 0, 0, expected.3 - 1, 0, false),
        Err(VE::Work)
    ));
    let shifted = run(0, 0, 0, WORK, scopes.work_steps() + 17, false).unwrap();
    assert_eq!(shifted.3, expected.3 + 17);
    assert_eq!(shifted.4, expected.4);
    let mut failed = Content::new(&scopes, text, limits, expected.3 - 1, 0, 0, 0, 0).unwrap();
    assert!(matches!(failed.build(), Err(VE::Work)));
    assert_eq!(
        (
            failed.record_charge(),
            failed.spool_charge(),
            failed.output_charge()
        ),
        (expected.0, expected.1, expected.2)
    );
    assert!(failed.build().is_err());
    assert!(failed.output_charge() >= expected.2);
    // A fresh image branch cannot hide the retained text contribution by
    // presenting only the larger of two independently consumed totals.
    let omitted = |target: &typaxis_display_list::book_v2::BookV2BodyDisplay<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
                   prefix: [u64; 4]| {
        let mut selection_builder = typaxis_resources::book_v2::BookV2FontSelectionBuilder::new(
            target, limits, WORK, prefix[0], prefix[1], prefix[3],
        )
        .unwrap();
        let selection = selection_builder.select_images().unwrap();
        let rasters = selection_builder.write_raster_programs(&selection).unwrap();
        let mut vector_builder = typaxis_pdf::book_v2::BookV2VectorProgramBuilder::new(
            &rasters, limits, WORK, 0, 0, prefix[2], 0,
        )
        .unwrap();
        let vectors = vector_builder.build().unwrap();
        let mut object_builder =
            BookV2ImageObjectBuilder::new(&vectors, fonts.next_object(), limits, WORK, 0, 0, 0, 0)
                .unwrap();
        let objects = object_builder.build().unwrap();
        let mut command_builder =
            BookV2ImageCommandBuilder::new(&objects, limits, WORK, 0, 0, 0, 0).unwrap();
        let commands = command_builder.build().unwrap();
        let mut scope_builder = Scopes::new(&commands, limits, WORK, 0, 0, 0, 0).unwrap();
        let candidate = scope_builder.build().unwrap();
        assert!(matches!(
            Content::new(&candidate, text, limits, WORK, 0, 0, 0, 0),
            Err(VE::Identity)
        ));
    };
    let prefix = [
        text.record_charge(),
        text.spool_charge(),
        text.output_charge(),
        text.work_steps(),
    ];
    let floors = [
        display.record_charge(),
        display.source().spool_charge(),
        0,
        display.work_steps(),
    ];
    for i in 0..4 {
        if prefix[i] > floors[i] {
            let mut short = prefix;
            short[i] -= 1;
            omitted(display, short);
        }
    }
    let mut other_builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
        display.source(),
        display.admitted(),
        limits,
        WORK,
        0,
        0,
    )
    .unwrap();
    let other = other_builder.build_body().unwrap();
    assert_eq!(other.fingerprint(), display.fingerprint());
    omitted(&other, prefix);
    let mut changed = limits.base().get().clone();
    changed.max_output_bytes -= 1;
    let changed = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(changed).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    assert!(matches!(
        Content::new(&scopes, text, &changed, WORK, 0, 0, 0, 0),
        Err(VE::Identity)
    ));
    if scopes.groups().iter().enumerate().any(|(i, g)| {
        scopes.actual_text(i).is_some() && commands.for_paint(g.paints().start).is_some()
    }) {
        for spare in [3u32, 2u32] {
            let first =
                limits.base().get().max_pdf_objects - images.objects().len() as u32 + 1 - spare;
            let mut ob =
                BookV2ImageObjectBuilder::new(images.source(), first, limits, WORK, 0, 0, 0, 0)
                    .unwrap();
            let io = ob.build().unwrap();
            let mut cb = BookV2ImageCommandBuilder::new(&io, limits, WORK, 0, 0, 0, 0).unwrap();
            let ic = cb.build().unwrap();
            let mut sb = Scopes::new(&ic, limits, WORK, 0, 0, 0, 0).unwrap();
            let ss = sb.build().unwrap();
            let mut b = Content::new(&ss, text, limits, WORK, 0, 0, 0, 0).unwrap();
            if spare == 3 {
                let c = b.build().unwrap();
                assert_eq!(c.next_object(), limits.base().get().max_pdf_objects + 1);
                assert_eq!(
                    c.anchor_objects().last().unwrap().id().get(),
                    limits.base().get().max_pdf_objects
                );
            } else {
                let before = (b.record_charge(), b.spool_charge(), b.output_charge());
                assert!(matches!(b.build(), Err(VE::Objects)));
                assert_eq!(
                    (b.record_charge(), b.spool_charge(), b.output_charge()),
                    before
                );
            }
        }
    }
}
