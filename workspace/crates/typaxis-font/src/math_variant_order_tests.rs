use super::*;
fn order_facts<B: Budget>(
    c: &cases::Case,
    order: VariantOrder,
    b: &mut B,
) -> BudgetResult<Facts, B::Error> {
    table_with_variant_order(&c.bytes, c.glyphs, order, b)
        .map(|r| r.map(|v| (v.constants, v.glyph_info_offset, v.variants_offset)))
}
#[test]
fn math_variant_order_private_cff_v2_allows_ties_and_preserves_legacy_and_bounds() {
    let mut cases_count = 0;
    let mut cutoffs = 0;
    for base in cases::controls().into_iter().skip(1) {
        let variants = usize::from(read_u16(&base.bytes, 8).unwrap());
        let count = usize::from(read_u16(&base.bytes, variants + 6).unwrap())
            + usize::from(read_u16(&base.bytes, variants + 8).unwrap());
        for index in 0..count {
            let construction =
                variants + usize::from(read_u16(&base.bytes, variants + 10 + 2 * index).unwrap());
            let first = read_u16(&base.bytes, construction + 6).unwrap();
            let advance_at = construction + 10;
            let mut tied = base.clone();
            tied.bytes[advance_at..advance_at + 2].copy_from_slice(&first.to_be_bytes());
            let mut legacy = Counter::<false>::new(Some(0), tied.bytes.len());
            assert_eq!(
                facts(&tied, &mut legacy).unwrap(),
                Err(MathFontError::MalformedTable("MATH"))
            );
            assert_eq!(legacy.attempts, 0);
            let mut trace = Counter::<true>::new(None, tied.bytes.len());
            let expected = order_facts(&tied, VariantOrder::CffV2Nondecreasing, &mut trace)
                .unwrap()
                .unwrap();
            assert_eq!(
                expected,
                facts(&base, &mut Counter::<false>::new(Some(0), base.bytes.len()))
                    .unwrap()
                    .unwrap()
            );
            for stop in 0..trace.accepted {
                let mut b = Counter::<true>::new(Some(stop), tied.bytes.len());
                let f = order_facts(&tied, VariantOrder::CffV2Nondecreasing, &mut b).unwrap_err();
                assert_eq!(f.at, stop);
                assert_eq!(b.paid, stop as u64 * STEP_WORK);
                cutoffs += 1;
            }
            for bad in [0, first - 1] {
                let mut c = tied.clone();
                c.bytes[advance_at..advance_at + 2].copy_from_slice(&bad.to_be_bytes());
                assert_eq!(
                    order_facts(
                        &c,
                        VariantOrder::CffV2Nondecreasing,
                        &mut Counter::<false>::new(Some(0), c.bytes.len())
                    )
                    .unwrap(),
                    Err(MathFontError::MalformedTable("MATH"))
                );
            }
            let mut c = tied.clone();
            c.bytes[construction + 8..construction + 10].copy_from_slice(&c.glyphs.to_be_bytes());
            assert_eq!(
                order_facts(
                    &c,
                    VariantOrder::CffV2Nondecreasing,
                    &mut Counter::<false>::new(Some(0), c.bytes.len())
                )
                .unwrap(),
                Err(MathFontError::MalformedTable("MATH"))
            );
            cases_count += 1;
        }
    }
    assert_eq!(cases_count, 8);
    eprintln!("MATH CFF /2 tied constructions={cases_count}, typed refusal cuts={cutoffs}, legacy strict preserved");
}
#[test]
#[ignore = "requires unchanged original MATH CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn math_variant_order_original_latin_modern_table_validates_under_cff_v2() {
    let list =
        std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let font = std::fs::read(list.lines().nth(2).unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>(),
        "6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f"
    );
    let c = cases::font_case(&font);
    let mut trace = Counter::<true>::new(None, c.bytes.len());
    let good = order_facts(&c, VariantOrder::CffV2Nondecreasing, &mut trace)
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            good.1,
            good.2,
            good.0.axis_height,
            good.0.fraction_rule_thickness
        ),
        (224, 17034, 250, 40)
    );
    assert_eq!((trace.accepted, trace.paid), (9622, 615808));
    assert_eq!(
        facts(&c, &mut Counter::<false>::new(Some(0), c.bytes.len())).unwrap(),
        Err(MathFontError::MalformedTable("MATH"))
    );
    for stop in [0, 1, 57, 5618, 5619, 6000, 9621] {
        let mut trace = Counter::<true>::new(Some(stop), c.bytes.len());
        let refused = order_facts(&c, VariantOrder::CffV2Nondecreasing, &mut trace).unwrap_err();
        assert_eq!(refused.at, stop);
        assert_eq!(trace.paid, stop as u64 * 64);
    }
    let mut descending = c.clone();
    descending.bytes[17798..17800].copy_from_slice(&1014u16.to_be_bytes());
    assert_eq!(
        order_facts(
            &descending,
            VariantOrder::CffV2Nondecreasing,
            &mut Counter::<false>::new(Some(0), descending.bytes.len())
        )
        .unwrap(),
        Err(MathFontError::MalformedTable("MATH"))
    );
    eprintln!("original Latin Modern MATH CFF /2 table positive requests=9622 work=615808 sampled cuts=7; whole font admission excluded");
}
