use super::*;
#[test]
fn native_cff_math_coverage_binary_matches_legacy_for_every_gid() {
    let mut cases = Vec::new();
    let mut one = vec![0, 1, 0, 4];
    for gid in [0u16, 4, 100, 65535] {
        one.extend(gid.to_be_bytes());
    }
    cases.push(one);
    let mut two = vec![0, 2, 0, 3];
    for row in [[0u16, 2, 0], [10, 20, 3], [65530, 65535, 14]] {
        for n in row {
            two.extend(n.to_be_bytes());
        }
    }
    cases.push(two);
    cases.push(vec![0, 1, 0, 0]);
    cases.push(vec![0, 2, 0, 0]);
    for bytes in &cases {
        for gid in 0..=u16::MAX {
            assert_eq!(
                coverage_binary(bytes, 0, gid),
                super::super::coverage_index(bytes, 0, gid)
            );
        }
    }
    eprintln!(
        "CFF bounded coverage lookups={} exact comparisons",
        cases.len() * 65536
    );
}
#[test]
fn native_cff_math_setup_requires_math_and_refuses_before_table_access() {
    let source=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/vmb-book/fixture-fonts/typaxis-cff-fixture.otf"));
    let caps = typaxis_core::M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .unwrap(),
        typaxis_core::M4ResourceLimits::default(),
    )
    .unwrap();
    let admission =
        crate::admit_sfnt_cff1_v2(std::sync::Arc::from(source.as_slice()), 0, &caps).unwrap();
    let mut calls = 0;
    assert_eq!(
        CffMathFontFaceV2::from_admission_with_work(&admission, &mut |_| {
            calls += 1;
            Err(23)
        })
        .unwrap_err(),
        23
    );
    assert_eq!(calls, 1);
    let error =
        CffMathFontFaceV2::from_admission_with_work(&admission, &mut |_| Ok::<_, usize>(()))
            .unwrap()
            .unwrap_err();
    assert_eq!(error, MathFontError::MissingTable("MATH"));
}
