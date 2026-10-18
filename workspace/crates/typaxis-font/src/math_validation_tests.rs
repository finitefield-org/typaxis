use super::*;
#[path = "math_validation_cases.rs"]
pub(crate) mod cases;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Refused {
    at: usize,
    point: Point,
}
struct Counter<const ON: bool> {
    stop: Option<usize>,
    attempts: usize,
    accepted: usize,
    paid: u64,
    length: usize,
}
impl<const ON: bool> Counter<ON> {
    fn new(stop: Option<usize>, length: usize) -> Self {
        Self {
            stop,
            attempts: 0,
            accepted: 0,
            paid: 0,
            length,
        }
    }
}
impl<const ON: bool> Budget for Counter<ON> {
    type Error = Refused;
    const OBSERVED: bool = ON;
    fn work(&mut self, point: Point, amount: u64) -> std::result::Result<(), Refused> {
        assert!(ON);
        assert_eq!(amount, STEP_WORK);
        assert!(point.table_offset.map_or(true, |n| n <= self.length as u64));
        let at = self.attempts;
        self.attempts += 1;
        if self.stop == Some(at) {
            return Err(Refused { at, point });
        }
        self.accepted += 1;
        self.paid += amount;
        Ok(())
    }
}
type Facts = (MathFontConstants, usize, usize);
fn facts<B: Budget>(c: &cases::Case, budget: &mut B) -> BudgetResult<Facts, B::Error> {
    table(&c.bytes, c.glyphs, budget)
        .map(|native| native.map(|v| (v.constants, v.glyph_info_offset, v.variants_offset)))
}
const FIXTURE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/math/job/math.ttf"
));
#[test]
fn math_validation_native_facts_and_errors_match_the_saved_prechange_stream_without_ordinary_hooks()
{
    let rows = cases::cases(FIXTURE);
    assert_eq!(rows.len(), 7125);
    let mut output = Vec::new();
    let mut accepted = 0;
    for c in &rows {
        let mut off = Counter::<false>::new(Some(0), c.bytes.len());
        let ordinary = facts(c, &mut off).unwrap();
        assert_eq!((off.attempts, off.paid), (0, 0));
        let mut on = Counter::<true>::new(None, c.bytes.len());
        assert_eq!(facts(c, &mut on).unwrap(), ordinary, "{}", c.label);
        assert!(on.accepted > 0);
        accepted += usize::from(ordinary.is_ok());
        output.extend(format!("{}\t{ordinary:?}\n", c.label).bytes());
    }
    output.extend(format!("total={} accepted={accepted}\n", rows.len()).bytes());
    assert_eq!(accepted, 2279);
    let hash: String = typaxis_core::sha256(&output)
        .iter()
        .map(|n| format!("{n:02x}"))
        .collect();
    assert_eq!(
        hash,
        "2dd1a71f131385b96ce8ec2e64bb02be7e9b802ea0b8d88f16c1dcde33a78559"
    );
    eprintln!(
        "MATH saved native cases={}, accepted={accepted}, ordinary hooks=0",
        rows.len()
    );
}
#[test]
fn math_validation_every_small_callback_prefix_keeps_typed_point_and_paid_work() {
    let (mut cutoffs, mut whole) = (0, 0);
    for c in cases::cases(FIXTURE) {
        let mut ready = Counter::<true>::new(None, c.bytes.len());
        let expected = facts(&c, &mut ready).unwrap();
        let all = !c.label.contains("-byte-");
        if all {
            whole += 1;
        }
        let cuts = if all { ready.accepted } else { 1 };
        for stop in 0..cuts {
            let mut counter = Counter::<true>::new(Some(stop), c.bytes.len());
            let refused = facts(&c, &mut counter).unwrap_err();
            assert_eq!(refused.at, stop);
            assert_eq!(
                (counter.attempts, counter.accepted, counter.paid),
                (stop + 1, stop, stop as u64 * STEP_WORK)
            );
            assert!(refused
                .point
                .table_offset
                .map_or(true, |n| n <= c.bytes.len() as u64));
            cutoffs += 1;
        }
        let mut enough = Counter::<true>::new(Some(ready.accepted), c.bytes.len());
        assert_eq!(facts(&c, &mut enough).unwrap(), expected);
        assert_eq!(enough.paid, ready.paid);
    }
    eprintln!("MATH all-small cases={whole}, sampled/all-small refusal cuts={cutoffs}");
}
#[test]
fn math_validation_primitives_refuse_before_native_reads_and_relative_errors_without_inventing_offsets(
) {
    for which in 0..4 {
        let mut budget = Counter::<true>::new(Some(0), 0);
        let mut state = State {
            budget: &mut budget,
            length: 0,
            variant_order: VariantOrder::LegacyStrict,
        };
        let refusal = match which {
            0 => state.u16(&[], usize::MAX).unwrap_err(),
            1 => state.i16(&[], usize::MAX).unwrap_err(),
            2 => state.u32(&[], usize::MAX).unwrap_err(),
            _ => state.relative(usize::MAX, usize::MAX, 0).unwrap_err(),
        };
        let Failure::Budget(refused) = refusal else {
            panic!("native operation ran after refusal")
        };
        assert_eq!(refused.at, 0);
        assert_eq!(refused.point.table_offset, None);
        assert_eq!(budget.paid, 0);
    }
    eprintln!("MATH refused primitive boundaries=4, unknown offsets absent");
}

#[test]
#[ignore = "requires unchanged original MATH CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn math_validation_original_math_table_preserves_prior_rejection_and_nonzero_work() {
    let list =
        std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let font = std::fs::read(list.lines().nth(2).unwrap()).unwrap();
    let hash: String = typaxis_core::sha256(&font)
        .iter()
        .map(|n| format!("{n:02x}"))
        .collect();
    assert_eq!(
        hash,
        "6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f"
    );
    let c = cases::font_case(&font);
    assert_eq!((c.glyphs, c.bytes.len()), (4802, 25280));
    let mut off = Counter::<false>::new(Some(0), c.bytes.len());
    let expected = facts(&c, &mut off).unwrap();
    assert_eq!(expected, Err(MathFontError::MalformedTable("MATH")));
    assert_eq!(off.paid, 0);
    let mut on = Counter::<true>::new(None, c.bytes.len());
    assert_eq!(facts(&c, &mut on).unwrap(), expected);
    assert_eq!((on.accepted, on.paid), (5619, 359616));
    for stop in [0, 1, 10, 2048, 5618] {
        let mut b = Counter::<true>::new(Some(stop), c.bytes.len());
        let refused = facts(&c, &mut b).unwrap_err();
        assert_eq!(refused.at, stop);
        assert_eq!(b.paid, stop as u64 * STEP_WORK);
    }
    let mut enough = Counter::<true>::new(Some(5619), c.bytes.len());
    assert_eq!(facts(&c, &mut enough).unwrap(), expected);
    // Direct table validation preserves the prior policy; it creates no font resource.
    eprintln!("original MATH raw table prior negative preserved, requests=5619, work=359616, five refusal cutoffs");
}

mod variant_order {
    include!("math_variant_order_tests.rs");
}
