use super::*;
#[path = "book_v2_image_resource_tests.rs"]
mod images;
use typaxis_display_list::book_v2::{
    BookV2BodyPaintIndex as Paint, BookV2FontUseGlyphs as Glyphs, BookV2MathPaint,
};
use typaxis_display_list::ProductionNativeMathPaint as Native;
use typaxis_pdf::book_v2::{BookV2FontObjects, BookV2TextCommandBuilder as Text};

fn fixed(token: &str) -> i128 {
    let negative = token.starts_with('-');
    let token = token.strip_prefix('-').unwrap_or(token);
    let (whole, fraction) = token.split_once('.').unwrap_or((token, ""));
    let whole = whole.parse::<i128>().unwrap() * 65536;
    let part = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i128>().unwrap()
    };
    let divisor = 10i128.pow(fraction.len() as u32);
    assert_eq!(part * 65536 % divisor, 0, "inexact PDF position: {token}");
    let value = whole + part * 65536 / divisor;
    if negative {
        -value
    } else {
        value
    }
}
pub(super) fn check(
    source: &BookV2FontObjects<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    const MAX_WORK: u64 = 1_000_000_000;
    let cids = source.source().source();
    let display = cids.source().source().selection().display();
    let mut builder = Text::new(source, limits, MAX_WORK, 0, 0, 0, 0).unwrap();
    let text = builder.build().unwrap();
    images::check(
        &text,
        display,
        limits,
        text.record_charge(),
        text.spool_charge(),
        text.output_charge(),
        text.work_steps(),
    );
    assert!(std::ptr::eq(text.source(), source));
    let mut usages = 0;
    let mut bytes = 0;
    let mut expected = Vec::new();
    for (index, command) in text.commands().iter().enumerate() {
        let encoded = text.command_bytes(index).unwrap();
        bytes += encoded.len();
        let tokens = std::str::from_utf8(encoded)
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>();
        assert!(!tokens
            .iter()
            .any(|t| ["BDC", "BMC", "EMC", "q", "Q"].contains(t)));
        assert_eq!(&tokens[..2], ["0", "g"]);
        let paint = display.paints()[command.paint_index()];
        let page = match paint {
            Paint::Text(i) => display.text().draws()[i].page_index(),
            Paint::Marker(i) => display.markers().draws()[i]
                .fragment()
                .fragment()
                .page_index(),
            Paint::EquationNumber(i) => display.numbers().draws()[i]
                .placement()
                .geometry()
                .page_index(),
            Paint::Math(i) => display.math().draws()[i].terminal().page_index(),
            Paint::FootnoteSeparator(i) => display.markers().separators()[i].page_index(),
            Paint::Image(_) => panic!("image is not text or a rule"),
        };
        assert_eq!(command.page_index(), page);
        if let Some(usage_index) = command.usage_index() {
            assert_eq!(usage_index, usages);
            usages += 1;
            let usage = &cids.uses()[usage_index];
            assert_eq!(usage.source().paint_index(), command.paint_index());
            assert_eq!(Some(usage.source().usage().slot()), command.slot_index());
            let actual = usage.source().usage();
            let glyphs = match actual.glyphs() {
                Glyphs::Cluster(glyphs) => glyphs
                    .iter()
                    .map(|g| (g.x().raw(), g.y().raw()))
                    .collect::<Vec<_>>(),
                Glyphs::Native(_) => {
                    let Paint::Math(i) = paint else { panic!() };
                    let BookV2MathPaint::Native(n) = display.math().draws()[i].paint() else {
                        panic!()
                    };
                    let Native::Glyph { x, y, .. } = n.paints()[command.slot_index().unwrap()]
                    else {
                        panic!()
                    };
                    vec![(x.raw(), y.raw())]
                }
            };
            assert_eq!(tokens[2], "BT");
            assert_eq!(
                tokens[3],
                format!("/PB{}", actual.instance().font_instance_id().get())
            );
            assert_eq!(fixed(tokens[4]), i128::from(actual.size().get().raw()));
            assert_eq!(
                &tokens[5..18],
                ["Tf", "0", "Tr", "0", "Tc", "0", "Tw", "100", "Tz", "0", "TL", "0", "Ts"]
            );
            let draws = &tokens[18..tokens.len() - 1];
            assert_eq!(tokens.last(), Some(&"ET"));
            assert_eq!(draws.len(), 9 * glyphs.len());
            for ((chunk, (x, y)), cid) in draws
                .chunks_exact(9)
                .zip(&glyphs)
                .zip(cids.cids(usage_index).unwrap())
            {
                assert_eq!(&chunk[..4], ["1", "0", "0", "-1"]);
                assert_eq!(fixed(chunk[4]), i128::from(*x));
                assert_eq!(fixed(chunk[5]), i128::from(*y));
                assert_eq!(chunk[6], "Tm");
                assert_eq!(chunk[8], "Tj");
                assert_eq!(chunk[7].len(), 6);
                assert_eq!(u16::from_str_radix(&chunk[7][1..5], 16).unwrap(), cid.get());
            }
            match (
                text.actual_text(index),
                source.source().actual_text(usage_index),
            ) {
                (Some(a), Some(b)) => assert!(std::ptr::eq(a, b)),
                (None, None) => {}
                _ => panic!("wrong occurrence ActualText"),
            }
            expected.push(serde_json::json!({"page":page,"font":actual.instance().font_instance_id().get(),"font_object":source.font_object(usage.font_index()).unwrap().get(),"size":actual.size().get().raw(),"positions":glyphs,"cids":cids.cids(usage_index).unwrap().iter().map(|c|c.get()).collect::<Vec<_>>(),"actual_text":text.actual_text(index),"source_text":original(actual.text())}));
        } else {
            let rect = match paint {
                Paint::FootnoteSeparator(i) => {
                    assert!(command.slot_index().is_none());
                    display.markers().separators()[i].ink()
                }
                Paint::Math(i) => {
                    let BookV2MathPaint::Native(n) = display.math().draws()[i].paint() else {
                        panic!()
                    };
                    let Native::Rule(r) = n.paints()[command.slot_index().unwrap()] else {
                        panic!()
                    };
                    r
                }
                _ => panic!("unexpected rule owner"),
            };
            assert!(text.actual_text(index).is_none());
            assert_eq!(tokens.len(), 8);
            assert_eq!(&tokens[6..], ["re", "f"]);
            let raw = [
                rect.x().raw(),
                rect.y().raw(),
                rect.width().get().raw(),
                rect.height().get().raw(),
            ];
            for (token, value) in tokens[2..6].iter().zip(raw) {
                assert_eq!(fixed(token), i128::from(value));
            }
            expected.push(serde_json::json!({"page":page,"rect":raw}));
        }
    }
    assert_eq!(usages, cids.uses().len());
    assert_eq!(bytes, text.byte_length());
    let mut command_count = 0;
    for (index, paint) in display.paints().iter().enumerate() {
        let commands = text.for_paint(index).unwrap();
        let expected_count = match paint {
            Paint::FootnoteSeparator(_) => 1,
            _ => display.font_slot_count(index).unwrap(),
        };
        assert_eq!(commands.len(), expected_count);
        assert!(commands.iter().all(|c| c.paint_index() == index));
        command_count += commands.len();
    }
    assert_eq!(command_count, text.commands().len());
    assert!(text.for_paint(usize::MAX).is_none());
    assert!(text.command_bytes(usize::MAX).is_none());
    assert!(text.actual_text(usize::MAX).is_none());
    if let Ok(directory) = std::env::var("TYPAXIS_BOOK_TEXT_COMMANDS_PROBE") {
        capture(&text, &expected, &directory);
    }
    let measured = (
        text.record_charge(),
        text.spool_charge(),
        text.output_charge(),
        text.work_steps(),
        text.fingerprint(),
    );
    let run = |records, spool, output, work, prior_work| -> Result<_, Error> {
        let mut b = Text::new(source, limits, work, records, spool, output, prior_work)?;
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
    assert_eq!(run(0, 0, 0, work, 0).unwrap(), measured);
    assert_eq!(output - source.output_charge(), bytes as u64);
    let base = limits.base().get();
    let pr = base.max_fragments - (records - source.record_charge());
    let ps = base.max_spool_bytes - (spool - source.spool_charge());
    let po = base.max_output_bytes - (output - source.output_charge());
    assert_eq!(
        run(pr, ps, po, work, 0).unwrap(),
        (
            base.max_fragments,
            base.max_spool_bytes,
            base.max_output_bytes,
            work,
            fp
        )
    );
    assert!(matches!(run(pr + 1, ps, po, work, 0), Err(Error::Records)));
    assert!(matches!(run(pr, ps + 1, po, work, 0), Err(Error::Spool)));
    assert!(matches!(run(pr, ps, po + 1, work, 0), Err(Error::Output)));
    assert!(matches!(run(0, 0, 0, work - 1, 0), Err(Error::Work)));
    assert_eq!(
        run(0, 0, 0, work + 17, source.work_steps() + 17).unwrap(),
        (records, spool, output, work + 17, fp)
    );
    let second = builder.build().unwrap();
    assert_eq!(second.fingerprint(), fp);
    assert_eq!(second.output_charge() - output, bytes as u64);
    assert_eq!(second.spool_charge() - spool, spool - source.spool_charge());
    let mut failed = Text::new(source, limits, work - 1, 0, 0, 0, 0).unwrap();
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
    assert!(
        failed.record_charge() >= records
            && failed.spool_charge() >= spool
            && failed.output_charge() >= output
    );
    let mut lower = source.work_steps();
    let mut upper = work - 1;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let mut probe = Text::new(source, limits, middle, 0, 0, 0, 0).unwrap();
        assert!(matches!(probe.build(), Err(Error::Work)));
        if probe.record_charge() == records {
            upper = middle;
        } else {
            assert_eq!(probe.record_charge(), source.record_charge() + 1);
            lower = middle + 1;
        }
    }
    let mut reserved = Text::new(source, limits, lower, 0, 0, 0, 0).unwrap();
    assert!(matches!(reserved.build(), Err(Error::Work)));
    assert_eq!(
        (
            reserved.record_charge(),
            reserved.spool_charge(),
            reserved.output_charge()
        ),
        (records, spool, output)
    );
    if lower > source.work_steps() {
        let mut before = Text::new(source, limits, lower - 1, 0, 0, 0, 0).unwrap();
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
    let mut changed = base.clone();
    changed.max_output_bytes -= 1;
    let changed = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(changed).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    assert!(matches!(
        Text::new(source, &changed, MAX_WORK, 0, 0, 0, 0),
        Err(Error::Identity)
    ));
}

fn capture(
    text: &typaxis_pdf::book_v2::BookV2TextCommands<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    expected: &[serde_json::Value],
    directory: &str,
) {
    // Test-only envelope: actual font object bytes and physical text/rules.
    // It omits images, formula replacement scopes and structure deliberately;
    // complete public PDF assembly must not consume this probe generator.
    let source = text.source();
    let cids = source.source().source();
    let mut resources = String::from("<< /Font <<");
    for (i, font) in cids.fonts().iter().enumerate() {
        resources.push_str(&format!(
            " /PB{} {} 0 R",
            font.font_instance_id().get(),
            source.font_object(i).unwrap().get()
        ));
    }
    resources.push_str(" >> >>");
    let content_id = source.next_object();
    let mut content = b"q\n1 0 0 -1 0 792 cm\n".to_vec();
    for i in 0..text.commands().len() {
        if let Some(actual) = text.actual_text(i) {
            content.extend_from_slice(b"/Span << /ActualText ");
            content.extend_from_slice(actual);
            content.extend_from_slice(b" >> BDC\n");
        }
        content.extend_from_slice(text.command_bytes(i).unwrap());
        if text.actual_text(i).is_some() {
            content.extend_from_slice(b"EMC\n");
        }
    }
    content.extend_from_slice(b"Q\n");
    let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize];
    for (id,body) in [(1,"<< /Type /Catalog /Pages 2 0 R >>".to_string()),(2,"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),(3,format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources {resources} /Contents {content_id} 0 R >>"))]{offsets.push(pdf.len());pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());}
    let start = pdf.len();
    offsets.extend(
        source
            .objects()
            .iter()
            .map(|o| start + o.byte_range().start),
    );
    pdf.extend_from_slice(source.bytes());
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "{content_id} 0 obj\n<< /Length {} >>\nstream\n",
            content.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(&content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
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
    let key = text
        .fingerprint()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let path = std::path::Path::new(directory);
    std::fs::create_dir_all(path).unwrap();
    std::fs::write(path.join(format!("{key}.pdf")), pdf).unwrap();
    std::fs::write(
        path.join(format!("{key}.json")),
        serde_json::to_vec(expected).unwrap(),
    )
    .unwrap();
}
