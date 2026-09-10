use super::*;
#[path = "book_v2_marked_content_tests.rs"]
mod content;
use typaxis_display_list::book_v2::BookV2ImageSource;
use typaxis_pagination::book_v2::BookV2BodyMathSource;
use typaxis_pdf::book_v2::{
    BookV2ImageCommands, BookV2MarkedArtifact as A, BookV2MarkedRole as R,
    BookV2MarkedScopeBuilder as Scopes,
};
fn utf16(text: &str) -> String {
    let mut value = String::from("<FEFF");
    for c in text.encode_utf16() {
        value.push_str(&format!("{c:04X}"));
    }
    value.push('>');
    value
}
pub(super) fn check(
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
    source: &BookV2ImageCommands<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    const MAX_WORK: u64 = 1_000_000_000;
    let display = source.source().source().source().source().display();
    // Derive the oracle from the actual independent component records, not
    // the scope getters or encoded output. All six paint kinds participate.
    let original = display
        .paints()
        .iter()
        .map(|paint| match *paint {
            Paint::Text(i) => {
                let d = &display.text().draws()[i];
                (
                    Some(d.owner()),
                    d.page_index(),
                    d.fragment_index(),
                    R::Text,
                    d.repeated_header(),
                    None,
                    None,
                )
            }
            Paint::Marker(i) => {
                let d = &display.markers().draws()[i];
                (
                    Some(d.source().owner()),
                    d.fragment().fragment().page_index(),
                    d.fragment_index(),
                    R::Label,
                    d.repeated_header(),
                    None,
                    None,
                )
            }
            Paint::Math(i) => {
                let d = display.math().draws()[i].terminal();
                let (alt, actual) = match d.source() {
                    BookV2BodyMathSource::Native(n) => (
                        n.source().domain().speech.as_str(),
                        n.source().domain().speech.as_str(),
                    ),
                    BookV2BodyMathSource::Vector(v) => (
                        v.source().alternative().alternative(),
                        v.source().alternative().resolved_actual_text().unwrap(),
                    ),
                };
                (
                    Some(d.source().owner()),
                    d.page_index(),
                    d.fragment_index(),
                    R::Formula,
                    d.repeated_header(),
                    Some(alt),
                    Some(actual),
                )
            }
            Paint::Image(i) => {
                let d = &display.images().draws()[i];
                let (alt, actual) = match d.source() {
                    BookV2ImageSource::Figure(f) => (f.source().alternative(), None),
                    BookV2ImageSource::Vector(v) => (
                        v.source().alternative().alternative(),
                        v.source().alternative().authored_actual_text(),
                    ),
                };
                (
                    Some(d.source().owner()),
                    d.fragment().fragment().page_index(),
                    d.fragment_index(),
                    R::Figure,
                    d.repeated_header(),
                    Some(alt),
                    actual,
                )
            }
            Paint::EquationNumber(i) => {
                let d = display.numbers().draws()[i].placement();
                (
                    Some(d.geometry().owner()),
                    d.geometry().page_index(),
                    d.geometry().fragment_index() as usize,
                    R::EquationNumber,
                    d.repeated_header(),
                    None,
                    None,
                )
            }
            Paint::FootnoteSeparator(i) => {
                let d = &display.markers().separators()[i];
                (
                    None,
                    d.page_index(),
                    d.before_fragment_index(),
                    R::Separator,
                    false,
                    None,
                    None,
                )
            }
        })
        .collect::<Vec<_>>();
    let run = |records, spool, output, work, prior_work, capture| -> Result<_, VE> {
        let mut builder = Scopes::new(source, limits, work, records, spool, output, prior_work)?;
        let scopes = builder.build()?;
        assert!(std::ptr::eq(scopes.source(), source));
        assert!(scopes.for_paint(usize::MAX).is_none());
        assert!(scopes.page_groups(u32::MAX).is_none());
        assert!(scopes.begin_bytes(usize::MAX).is_none());
        assert!(scopes.actual_text(usize::MAX).is_none());
        assert!(scopes.alternative(usize::MAX).is_none());
        assert!(scopes.end_bytes(usize::MAX).is_none());
        let mut next = 0;
        let mut per_page = vec![0; display.source().source().geometry().pages().len()];
        let mut encoded_bytes = 0;
        let mut expected = Vec::new();
        let mut begins = Vec::new();
        for (i, g) in scopes.groups().iter().enumerate() {
            assert_eq!(g.paints().start, next);
            let actual = original[next];
            let mut end = next + 1;
            if actual.3 == R::Text {
                while end < original.len() && original[end] == actual {
                    end += 1;
                }
            }
            assert_eq!(g.paints(), next..end);
            assert_eq!(
                (g.owner(), g.page_index(), g.fragment_index(), g.role()),
                (actual.0, actual.1, actual.2, actual.3)
            );
            let artifact = if actual.4 {
                Some(A::RepeatedHeader)
            } else if actual.3 == R::Separator {
                Some(A::FootnoteSeparator)
            } else {
                None
            };
            assert_eq!(g.artifact(), artifact);
            let mcid = if artifact.is_some() {
                None
            } else {
                let n = per_page[actual.1 as usize];
                per_page[actual.1 as usize] += 1;
                Some(n)
            };
            assert_eq!(g.mcid(), mcid);
            let actual_text = if artifact.is_some() { None } else { actual.6 };
            assert_eq!(scopes.actual_text(i), actual_text);
            assert_eq!(
                scopes.alternative(i),
                if artifact.is_some() { None } else { actual.5 }
            );
            let tag = match g.role() {
                R::Text | R::EquationNumber => "Span",
                R::Label => "Lbl",
                R::Formula => "Formula",
                R::Figure => "Figure",
                R::Separator => "Artifact",
            };
            let (wanted, kind, subtype) = match artifact {
                Some(A::RepeatedHeader) => (
                    String::from("/Artifact << /Type /Pagination /Subtype /Header >> BDC\n"),
                    Some("Pagination"),
                    Some("Header"),
                ),
                Some(A::FootnoteSeparator) => (
                    String::from("/Artifact << /Type /Layout >> BDC\n"),
                    Some("Layout"),
                    None,
                ),
                None => (
                    format!(
                        "/{tag} << /MCID {} >> BDC\n{}",
                        mcid.unwrap(),
                        actual_text
                            .map(|a| format!("/Span << /ActualText {} >> BDC\n", utf16(a)))
                            .unwrap_or_default()
                    ),
                    None,
                    None,
                ),
            };
            assert_eq!(scopes.begin_bytes(i).unwrap(), wanted.as_bytes());
            let closing = if actual_text.is_some() {
                "EMC\nEMC\n"
            } else {
                "EMC\n"
            };
            assert_eq!(scopes.end_bytes(i).unwrap(), closing.as_bytes());
            encoded_bytes += wanted.len() + closing.len();
            begins.push(wanted.clone());
            expected.push(serde_json::json!({"page":actual.1,"paint_start":next,"paint_end":end,"owner":actual.0.map(|v|v.get()),"tag":if artifact.is_some(){"Artifact"}else{tag},"mcid":mcid,"type":kind,"subtype":subtype,"actual_text":actual_text,"begin":wanted}));
            for paint in next..end {
                assert!(std::ptr::eq(scopes.for_paint(paint).unwrap(), g));
            }
            next = end;
        }
        assert_eq!(next, original.len());
        let mut page_cursor = 0;
        for page in 0..per_page.len() {
            let groups = scopes.page_groups(page as u32).unwrap();
            assert!(groups.iter().all(|g| g.page_index() == page as u32));
            assert_eq!(
                groups.iter().filter(|g| g.mcid().is_some()).count(),
                per_page[page] as usize
            );
            for g in groups {
                assert!(std::ptr::eq(g, &scopes.groups()[page_cursor]));
                page_cursor += 1;
            }
        }
        assert_eq!(page_cursor, scopes.groups().len());
        assert_eq!(
            scopes.output_charge() - output.max(source.output_charge()),
            encoded_bytes as u64
        );
        if capture {
            content::check(text, &scopes, limits);
            if let Ok(dir) = std::env::var("TYPAXIS_BOOK_MARKED_SCOPES_PROBE") {
                std::fs::create_dir_all(&dir).unwrap();
                let name = scopes
                    .fingerprint()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                std::fs::write(
                    std::path::Path::new(&dir).join(format!("{name}.json")),
                    serde_json::to_vec_pretty(
                        &serde_json::json!({"pages":per_page.len(),"groups":expected}),
                    )
                    .unwrap(),
                )
                .unwrap();
            }
        }
        let result = (
            scopes.record_charge(),
            scopes.spool_charge(),
            scopes.output_charge(),
            scopes.work_steps(),
            scopes.fingerprint(),
            begins,
        );
        if capture {
            let again = builder.build()?;
            assert_eq!(again.fingerprint(), scopes.fingerprint());
            assert_eq!(again.groups(), scopes.groups());
            assert_eq!(
                again.output_charge() - scopes.output_charge(),
                encoded_bytes as u64
            );
        }
        Ok(result)
    };
    let expected = run(0, 0, 0, MAX_WORK, 0, true).unwrap();
    let base = limits.base().get();
    let pr = base.max_fragments - (expected.0 - source.record_charge());
    let ps = base.max_spool_bytes - (expected.1 - source.spool_charge());
    let po = base.max_output_bytes - (expected.2 - source.output_charge());
    let exact = run(pr, ps, po, expected.3, 0, false).unwrap();
    assert_eq!(
        (exact.0, exact.1, exact.2),
        (
            base.max_fragments,
            base.max_spool_bytes,
            base.max_output_bytes
        )
    );
    assert_eq!((&exact.4, &exact.5), (&expected.4, &expected.5));
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
    let shifted = run(0, 0, 0, MAX_WORK, source.work_steps() + 17, false).unwrap();
    assert_eq!(shifted.3, expected.3 + 17);
    assert_eq!(shifted.4, expected.4);
    for max_work in [source.work_steps(), expected.3 - 1] {
        let mut attempt = Scopes::new(source, limits, max_work, 0, 0, 0, 0).unwrap();
        assert!(matches!(attempt.build(), Err(VE::Work)));
        let used = (
            attempt.record_charge(),
            attempt.spool_charge(),
            attempt.output_charge(),
        );
        if max_work == expected.3 - 1 {
            assert_eq!(used, (expected.0, expected.1, expected.2));
        }
        assert!(attempt.build().is_err());
        assert!(attempt.record_charge() >= used.0);
        assert!(attempt.spool_charge() >= used.1);
        assert!(attempt.output_charge() >= used.2);
    }
    let mut changed = base.clone();
    changed.max_output_bytes -= 1;
    let changed = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(changed).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    assert!(matches!(
        Scopes::new(source, &changed, MAX_WORK, 0, 0, 0, 0),
        Err(VE::Identity)
    ));
}
