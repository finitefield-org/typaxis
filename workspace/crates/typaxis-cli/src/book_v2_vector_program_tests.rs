use super::*;
use typaxis_pdf::book_v2::{BookV2FontStreamError as VE, BookV2VectorProgramBuilder as Vectors};
use typaxis_resources::{book_v2::BookV2RasterPrograms, AdmittedSafeVector};
pub(super) fn check(
    text: &typaxis_pdf::book_v2::BookV2TextCommands<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    source: &BookV2RasterPrograms<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
    base_output: u64,
) {
    const MAX_WORK: u64 = 1_000_000_000;
    let run = |records, spool, output, work, prior_work, capture| -> Result<_, VE> {
        let mut builder = Vectors::new(source, limits, work, records, spool, output, prior_work)?;
        let programs = builder.build()?;
        assert!(std::ptr::eq(programs.source(), source));
        assert!(programs.program(usize::MAX).is_none());
        let mut payloads = Vec::new();
        for (index, image) in source.source().images().iter().enumerate() {
            let p = programs.program(index);
            let Some(ir) = image.image().admitted_safe_vector() else {
                assert!(p.is_none());
                continue;
            };
            let p = p.unwrap();
            assert!(std::ptr::eq(p.source(), image));
            assert!(std::ptr::eq(p.ir(), ir));
            let (pairs, draws, canonical) = match ir {
                AdmittedSafeVector::V1(v) => (
                    v.draws().iter().map(|_| (65536, 65536)).collect::<Vec<_>>(),
                    v.draws().len(),
                    v.canonical_jcs(),
                ),
                AdmittedSafeVector::V2(v) => (
                    v.draws()
                        .iter()
                        .map(|d| (d.fill().alpha().raw(), d.stroke().paint().alpha().raw()))
                        .collect::<Vec<_>>(),
                    v.draws().len(),
                    v.canonical_jcs(),
                ),
            };
            let expected = pairs.iter().copied().collect::<BTreeSet<_>>();
            assert_eq!(
                p.states()
                    .iter()
                    .map(|s| (s.fill_alpha_raw(), s.stroke_alpha_raw()))
                    .collect::<Vec<_>>(),
                expected.iter().copied().collect::<Vec<_>>()
            );
            assert_eq!(
                p.bbox(),
                [
                    0,
                    0,
                    ir.intrinsic_width().get().raw(),
                    ir.intrinsic_height().get().raw()
                ]
            );
            let content = std::str::from_utf8(p.content()).unwrap();
            assert_eq!(content.lines().filter(|l| *l == "q").count(), draws + 1);
            assert_eq!(content.lines().filter(|l| *l == "Q").count(), draws + 1);
            let actual = content
                .lines()
                .filter(|l| l.ends_with(" gs"))
                .map(|l| {
                    l.strip_prefix("/GS")
                        .unwrap()
                        .strip_suffix(" gs")
                        .unwrap()
                        .parse::<usize>()
                        .unwrap()
                })
                .collect::<Vec<_>>();
            let sorted = expected.iter().copied().collect::<Vec<_>>();
            assert_eq!(
                actual,
                pairs
                    .iter()
                    .map(|pair| sorted.binary_search(pair).unwrap())
                    .collect::<Vec<_>>()
            );
            for forbidden in [
                "/Subtype /Image",
                "/MCID",
                "/Alt",
                "/ActualText",
                "/Lang",
                " BDC",
                " BMC",
                " Do",
            ] {
                assert!(!content.contains(forbidden));
            }
            let dictionaries = p
                .states()
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let bytes = p.state_dictionary(i).unwrap();
                    let text = std::str::from_utf8(bytes).unwrap();
                    let tokens = text.split_whitespace().collect::<Vec<_>>();
                    assert_eq!(&tokens[..4], &["<<", "/Type", "/ExtGState", "/ca"]);
                    assert_eq!(tokens[5], "/CA");
                    assert_eq!(tokens[7], ">>");
                    assert_eq!(
                        tokens[4].parse::<f64>().unwrap() * 65536.0,
                        f64::from(s.fill_alpha_raw())
                    );
                    assert_eq!(
                        tokens[6].parse::<f64>().unwrap() * 65536.0,
                        f64::from(s.stroke_alpha_raw())
                    );
                    text.to_owned()
                })
                .collect::<Vec<_>>();
            assert!(p.state_dictionary(p.states().len()).is_none());
            if capture {
                if let Ok(directory) = std::env::var("TYPAXIS_BOOK_VECTOR_PROGRAMS_PROBE") {
                    let key = p
                        .fingerprint()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>();
                    let path = std::path::Path::new(&directory);
                    std::fs::create_dir_all(path).unwrap();
                    std::fs::write(path.join(format!("{key}.svg")), image.image().bytes()).unwrap();
                    std::fs::write(path.join(format!("{key}.content")), p.content()).unwrap();
                    std::fs::write(path.join(format!("{key}.json")),serde_json::to_vec(&serde_json::json!({"ir":serde_json::from_str::<serde_json::Value>(canonical).unwrap(),"states":dictionaries,"source_sha256":image.image().content_hash().iter().map(|b|format!("{b:02x}")).collect::<String>(),"content_sha256":typaxis_core::sha256(p.content()).iter().map(|b|format!("{b:02x}")).collect::<String>()})).unwrap()).unwrap();
                }
            }
            payloads.push((p.content().to_vec(), dictionaries, p.fingerprint()));
        }
        if capture {
            objects::check(text, &programs, limits);
        }
        Ok((
            programs.record_charge(),
            programs.spool_charge(),
            programs.output_charge(),
            programs.work_steps(),
            programs.fingerprint(),
            payloads,
        ))
    };
    let expected = run(0, 0, base_output, MAX_WORK, 0, true).unwrap();
    assert_eq!(
        run(0, 0, base_output, expected.3, 0, false).unwrap(),
        expected
    );
    let records = limits.base().get().max_fragments - (expected.0 - source.record_charge());
    let spool = limits.base().get().max_spool_bytes - (expected.1 - source.spool_charge());
    let output = limits.base().get().max_output_bytes - (expected.2 - base_output);
    let exact = run(records, spool, output, expected.3, 0, false).unwrap();
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
        run(records + 1, spool, output, expected.3, 0, false),
        Err(VE::Records)
    ));
    if expected.1 > source.spool_charge() {
        assert!(matches!(
            run(records, spool + 1, output, expected.3, 0, false),
            Err(VE::Spool)
        ));
    }
    assert!(matches!(
        run(records, spool, output + 1, expected.3, 0, false),
        Err(VE::Output)
    ));
    assert!(matches!(
        run(0, 0, base_output, expected.3 - 1, 0, false),
        Err(VE::Work)
    ));
    let prior = source.work_steps() + 17;
    let shifted = run(0, 0, base_output, MAX_WORK, prior, false).unwrap();
    assert_eq!(shifted.3, expected.3 + 17);
    assert_eq!((&shifted.4, &shifted.5), (&expected.4, &expected.5));
    let mut failed =
        Vectors::new(source, limits, source.work_steps(), 0, 0, base_output, 0).unwrap();
    let before = (
        failed.record_charge(),
        failed.spool_charge(),
        failed.output_charge(),
    );
    assert!(matches!(failed.build(), Err(VE::Work)));
    let after = (
        failed.record_charge(),
        failed.spool_charge(),
        failed.output_charge(),
    );
    assert!(after.0 > before.0);
    assert!(after.1 >= before.1);
    assert!(failed.build().is_err());
    assert!(failed.record_charge() >= after.0);
    assert!(failed.spool_charge() >= after.1);
    if expected.2 > base_output {
        let mut lo = source.work_steps();
        let mut hi = expected.3;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let mut attempt = Vectors::new(source, limits, mid, 0, 0, base_output, 0).unwrap();
            let _ = attempt.build();
            if attempt.output_charge() > base_output {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        let mut before = Vectors::new(source, limits, lo - 1, 0, 0, base_output, 0).unwrap();
        assert!(matches!(before.build(), Err(VE::Work)));
        assert_eq!(before.output_charge(), base_output);
        let mut after = Vectors::new(source, limits, lo, 0, 0, base_output, 0).unwrap();
        assert!(matches!(after.build(), Err(VE::Work)));
        let reserved = (
            after.record_charge(),
            after.spool_charge(),
            after.output_charge(),
        );
        assert!(reserved.2 > base_output);
        assert!(after.build().is_err());
        assert!(after.record_charge() >= reserved.0);
        assert!(after.spool_charge() >= reserved.1);
        assert_eq!(after.output_charge(), reserved.2);
    }
    let mut other = limits.base().get().clone();
    other.max_output_bytes -= 1;
    let other = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(other).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    assert!(matches!(
        Vectors::new(source, &other, MAX_WORK, 0, 0, base_output, 0),
        Err(VE::Identity)
    ));
}

#[path = "book_v2_image_object_tests.rs"]
mod objects;
