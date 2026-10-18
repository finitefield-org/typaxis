use super::*;
use crate::cff::font_tables::tests::cases as sfnt_cases;
use crate::math::validation::tests::cases as math_cases;
const TINY:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/vmb-book/fixture-fonts/typaxis-cff-fixture.otf"));
fn tied_math() -> Vec<u8> {
    let mut table = math_cases::controls()[1].bytes.clone();
    let variants = usize::from(u16::from_be_bytes(table[8..10].try_into().unwrap()));
    let first = variants
        + usize::from(u16::from_be_bytes(
            table[variants + 10..variants + 12].try_into().unwrap(),
        ));
    let advance: [u8; 2] = table[first + 6..first + 8].try_into().unwrap();
    table[first + 10..first + 12].copy_from_slice(&advance);
    table
}
#[derive(Default)]
struct MathTrace {
    calls: usize,
    stop: Option<usize>,
}
impl CffBaseCmapBudgetV2 for MathTrace {
    type Error = usize;
    fn reserve(&mut self, _: CffBaseCmapStorageV2, _: usize, _: usize) -> Result<(), usize> {
        Ok(())
    }
    fn work(&mut self, _: u64) -> Result<(), usize> {
        Ok(())
    }
}
impl CffCmapBudgetV2 for MathTrace {
    const OBSERVE_VARIATIONS: bool = false;
    fn reserve_variation(
        &mut self,
        _: CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        Ok(())
    }
}
impl CffAdmissionBudgetV2 for MathTrace {
    const OBSERVE_SFNT: bool = false;
    const OBSERVE_MATH: bool = true;
    fn reserve_sfnt(&mut self, _: CffSfntStorageV2, _: usize, _: usize) -> Result<(), usize> {
        Ok(())
    }
    fn work_sfnt(&mut self, _: u64) -> Result<(), usize> {
        Ok(())
    }
    fn work_math(&mut self, n: u64) -> Result<(), usize> {
        assert_eq!(n, 64);
        if self.stop == Some(self.calls) {
            return Err(self.calls);
        }
        self.calls += 1;
        Ok(())
    }
}
#[test]
fn cff_v2_math_variant_policy_is_selected_by_admission_version_not_budget_flags() {
    let caps = limits();
    let good = sfnt_cases::replace(
        TINY,
        *b"MATH",
        Some(math_cases::controls()[1].bytes.clone()),
    );
    assert!(admit_sfnt_cff1_detailed(&good, 0, &caps).is_ok());
    let tied = sfnt_cases::replace(TINY, *b"MATH", Some(tied_math()));
    let old = admit_sfnt_cff1_detailed(&tied, 0, &caps).unwrap_err();
    assert_eq!(old.kind, Cff1Error::InvalidOptionalTable);
    assert_eq!(old.context.table_tag, Some(*b"MATH"));
    let current = admit_sfnt_cff1_v2(tied.clone().into(), 0, &caps).unwrap();
    let mut trace = MathTrace::default();
    let observed =
        admit_sfnt_cff1_v2_owned_with_admission_budget(tied.clone(), 0, &caps, &mut trace)
            .unwrap()
            .unwrap();
    assert_eq!(observed.fingerprint(), current.fingerprint());
    assert!(trace.calls > 0);
    for stop in [0, 1, trace.calls / 2, trace.calls - 1] {
        let mut short = MathTrace {
            calls: 0,
            stop: Some(stop),
        };
        let error =
            admit_sfnt_cff1_v2_owned_with_admission_budget(tied.clone(), 0, &caps, &mut short)
                .unwrap_err();
        let CffAdmissionBudgetFailureV2::Tables(f) = error else {
            panic!("wrong failure")
        };
        assert_eq!(f.error, stop);
        assert_eq!(f.point.table_tag, Some(*b"MATH"));
        assert!(f.point.file_offset.is_some() || f.point.table_offset.is_none());
        assert_eq!(short.calls, stop);
    }
    eprintln!(
        "CFF /1 strict table negative and /2 name-keyed MATH admitted; callbacks={} cuts=4",
        trace.calls
    );
}
#[test]
#[ignore = "requires unchanged original fonts through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn cff_v2_math_variant_original_table_in_derived_cid_font_is_admitted() {
    let list =
        std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let paths = list.lines().collect::<Vec<_>>();
    let mincho = std::fs::read(paths[0]).unwrap();
    let math = std::fs::read(paths[2]).unwrap();
    assert_eq!(
        sha256(&mincho)
            .iter()
            .map(|n| format!("{n:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    assert_eq!(
        sha256(&math)
            .iter()
            .map(|n| format!("{n:02x}"))
            .collect::<String>(),
        "6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f"
    );
    let table = math_cases::font_case(&math).bytes;
    let source = sfnt_cases::replace(&mincho, *b"MATH", Some(table));
    let caps = limits();
    let native = admit_sfnt_cff1_v2(source.clone().into(), 0, &caps).unwrap();
    let mut trace = MathTrace::default();
    let observed =
        admit_sfnt_cff1_v2_owned_with_admission_budget(source.clone(), 0, &caps, &mut trace)
            .unwrap()
            .unwrap();
    assert_eq!(native.fingerprint(), observed.fingerprint());
    assert_eq!(native.source_sha256(), sha256(&source));
    assert_eq!(trace.calls, 9622);
    for stop in [0, 1, 5618, 5619, 9621] {
        let mut trace = MathTrace {
            calls: 0,
            stop: Some(stop),
        };
        let error =
            admit_sfnt_cff1_v2_owned_with_admission_budget(source.clone(), 0, &caps, &mut trace)
                .unwrap_err();
        let CffAdmissionBudgetFailureV2::Tables(f) = error else {
            panic!("wrong failure")
        };
        assert_eq!(f.error, stop);
        assert_eq!(f.point.table_tag, Some(*b"MATH"));
        assert_eq!(trace.calls, stop);
    }
    // Both original files remain unchanged. The joined font is structural fixture
    // evidence only; glyph IDs do not assert semantic math/Mincho correspondence.
    assert_eq!(sha256(&std::fs::read(paths[0]).unwrap()), sha256(&mincho));
    assert_eq!(sha256(&std::fs::read(paths[2]).unwrap()), sha256(&math));
    eprintln!("derived CID + original MATH positive source_sha256={} fingerprint={} MATH callbacks=9622 sampled cuts=5",sha256(&source).iter().map(|n|format!("{n:02x}")).collect::<String>(),native.fingerprint().iter().map(|n|format!("{n:02x}")).collect::<String>());
}
