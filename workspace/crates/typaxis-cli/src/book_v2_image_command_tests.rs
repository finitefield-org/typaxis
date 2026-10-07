use super::*;
#[path = "book_v2_marked_scope_tests.rs"]
mod scopes;
use typaxis_display_list::book_v2::BookV2ImagePaint;
use typaxis_pdf::book_v2::{BookV2ImageCommandBuilder as Commands, BookV2ImageObjects};
pub(super) fn check(
    text: &typaxis_pdf::book_v2::BookV2TextCommands<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    source: &BookV2ImageObjects<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    const MAX_WORK: u64 = 1_000_000_000;
    let selection = source.source().source().source();
    let run = |records, spool, output, work, prior_work, capture| -> Result<_, VE> {
        let mut builder = Commands::new(source, limits, work, records, spool, output, prior_work)?;
        let commands = builder.build()?;
        assert!(std::ptr::eq(commands.source(), source));
        assert_eq!(commands.commands().len(), selection.uses().len());
        assert!(commands.for_paint(usize::MAX).is_none());
        assert!(commands.command_bytes(usize::MAX).is_none());
        let mut cursor = 0;
        let mut expected = Vec::new();
        for (index, (c, u)) in commands.commands().iter().zip(selection.uses()).enumerate() {
            assert_eq!(c.paint_index(), u.paint_index());
            assert_eq!(c.resource_index(), u.resource_index());
            assert_eq!(
                c.resource_object(),
                source.resource_object(u.resource_index()).unwrap()
            );
            assert_eq!(c.page_index(), u.usage().page_index());
            assert_eq!(c.owner(), u.usage().owner());
            assert!(std::ptr::eq(
                commands.for_paint(u.paint_index()).unwrap(),
                c
            ));
            assert_eq!(c.byte_range().start, cursor);
            cursor = c.byte_range().end;
            let bytes = commands.command_bytes(index).unwrap();
            assert_eq!(bytes, &commands.bytes()[c.byte_range()]);
            let content = std::str::from_utf8(bytes).unwrap();
            let mut lines = content.lines();
            assert_eq!(lines.next(), Some("q"));
            let (matrix, color) = match u.usage().geometry() {
                BookV2ImagePaint::Raster { viewport } => (
                    [
                        viewport.width().get().raw(),
                        0,
                        0,
                        -viewport.height().get().raw(),
                        viewport.x().raw(),
                        viewport.y().raw() + viewport.height().get().raw(),
                    ],
                    None,
                ),
                BookV2ImagePaint::Vector { matrix, color, .. } => {
                    for op in ["rg", "RG"] {
                        let tokens = lines.next().unwrap().split_whitespace().collect::<Vec<_>>();
                        assert_eq!(tokens.len(), 4);
                        assert_eq!(tokens[3], op);
                        for (token, c) in tokens[..3].iter().zip(color) {
                            assert_eq!(
                                fixed(token),
                                i128::from((u64::from(c) * 65536 + 127) / 255)
                            );
                        }
                    }
                    (
                        [
                            i64::from(matrix.a.raw()),
                            i64::from(matrix.b.raw()),
                            i64::from(matrix.c.raw()),
                            i64::from(matrix.d.raw()),
                            matrix.e.raw(),
                            matrix.f.raw(),
                        ],
                        Some(color),
                    )
                }
            };
            let tokens = lines.next().unwrap().split_whitespace().collect::<Vec<_>>();
            assert_eq!(tokens.len(), 7);
            assert_eq!(tokens[6], "cm");
            for (token, raw) in tokens[..6].iter().zip(matrix) {
                assert_eq!(fixed(token), i128::from(raw));
            }
            assert_eq!(
                lines.next().unwrap(),
                format!("/BI{} Do", u.resource_index())
            );
            assert_eq!(lines.next(), Some("Q"));
            assert_eq!(lines.next(), None);
            for bad in ["BDC", "BMC", "/MCID", "/Alt", "/ActualText", "BT"] {
                assert!(!content.contains(bad));
            }
            expected.push(serde_json::json!({"paint":u.paint_index(),"resource":u.resource_index(),"object":c.resource_object().get(),"page":u.usage().page_index(),"owner":u.usage().owner().get(),"matrix":matrix,"color":color}));
        }
        assert_eq!(cursor, commands.bytes().len());
        for paint in 0..selection.display().paints().len() {
            assert_eq!(
                commands.for_paint(paint).is_some(),
                selection.for_paint(paint).is_some()
            );
        }
        assert_eq!(
            commands.output_charge() - output.max(source.output_charge()),
            commands.bytes().len() as u64
        );
        if capture {
            scopes::check(text, &commands, limits);
            if let Ok(directory) = std::env::var("TYPAXIS_BOOK_IMAGE_COMMANDS_PROBE") {
                assert_eq!(source.first_object(), 4);
                let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
                let mut offsets = vec![0];
                let names = (0..selection.images().len())
                    .map(|i| format!(" /BI{i} {} 0 R", source.resource_object(i).unwrap().get()))
                    .collect::<String>();
                let page=format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /XObject <<{names} >> >> /Contents {} 0 R >>",source.next_object());
                for (id, body) in [
                    (1, "<< /Type /Catalog /Pages 2 0 R >>"),
                    (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                    (3, page.as_str()),
                ] {
                    offsets.push(pdf.len());
                    pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
                }
                let start = pdf.len();
                offsets.extend(
                    source
                        .objects()
                        .iter()
                        .map(|o| start + o.byte_range().start),
                );
                pdf.extend_from_slice(source.bytes());
                let mut content = b"q\n1 0 0 -1 0 792 cm\n".to_vec();
                content.extend_from_slice(commands.bytes());
                content.extend_from_slice(b"Q\n");
                offsets.push(pdf.len());
                pdf.extend_from_slice(
                    format!(
                        "{} 0 obj\n<< /Length {} >>\nstream\n",
                        source.next_object(),
                        content.len()
                    )
                    .as_bytes(),
                );
                pdf.extend_from_slice(&content);
                pdf.extend_from_slice(b"\nendstream\nendobj\n");
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
                let key = commands
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
        }
        Ok((
            commands.record_charge(),
            commands.spool_charge(),
            commands.output_charge(),
            commands.work_steps(),
            commands.fingerprint(),
            commands.bytes().to_vec(),
        ))
    };
    let expected = run(0, 0, 0, MAX_WORK, 0, true).unwrap();
    assert_eq!(run(0, 0, 0, expected.3, 0, false).unwrap(), expected);
    let br = source.record_charge();
    let bs = source.spool_charge();
    let bo = source.output_charge();
    let pr = limits.base().get().max_fragments - (expected.0 - br);
    let ps = limits.base().get().max_spool_bytes - (expected.1 - bs);
    let po = limits.base().get().max_output_bytes - (expected.2 - bo);
    let exact = run(pr, ps, po, expected.3, 0, false).unwrap();
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
        run(pr + 1, ps, po, expected.3, 0, false),
        Err(VE::Records)
    ));
    if expected.1 > bs {
        assert!(matches!(
            run(pr, ps + 1, po, expected.3, 0, false),
            Err(VE::Spool)
        ));
    }
    assert!(matches!(
        run(pr, ps, po + 1, expected.3, 0, false),
        Err(VE::Output)
    ));
    assert!(matches!(
        run(0, 0, 0, expected.3 - 1, 0, false),
        Err(VE::Work)
    ));
    let shifted = run(0, 0, 0, MAX_WORK, source.work_steps() + 17, false).unwrap();
    assert_eq!(shifted.3, expected.3 + 17);
    assert_eq!((&shifted.4, &shifted.5), (&expected.4, &expected.5));
    let mut attempt = Commands::new(source, limits, source.work_steps(), 0, 0, 0, 0).unwrap();
    assert!(matches!(attempt.build(), Err(VE::Work)));
    let used = (
        attempt.record_charge(),
        attempt.spool_charge(),
        attempt.output_charge(),
    );
    assert!(attempt.build().is_err());
    assert!(attempt.record_charge() >= used.0);
    assert!(attempt.spool_charge() >= used.1);
    assert!(attempt.output_charge() >= used.2);
    let mut late = Commands::new(source, limits, expected.3 - 1, 0, 0, 0, 0).unwrap();
    assert!(matches!(late.build(), Err(VE::Work)));
    assert_eq!(
        (
            late.record_charge(),
            late.spool_charge(),
            late.output_charge()
        ),
        (expected.0, expected.1, expected.2)
    );
    assert!(late.build().is_err());
    assert!(late.record_charge() >= expected.0);
    assert!(late.spool_charge() >= expected.1);
    assert!(late.output_charge() >= expected.2);
    let mut changed = limits.base().get().clone();
    changed.max_output_bytes -= 1;
    let changed = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(changed).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    assert!(matches!(
        Commands::new(source, &changed, MAX_WORK, 0, 0, 0, 0),
        Err(VE::Identity)
    ));
}
