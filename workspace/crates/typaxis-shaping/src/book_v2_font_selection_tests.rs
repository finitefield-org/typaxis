use super::*;
use crate::font_selection_fixture::{admit, FAMILIES, FONT};
use crate::production_text::inputs::BodyFont;

type SelectionSignature = (u32, u32, [u8; 32]);
type SelectionResult = Result<SelectionSignature, ProductionTextShapeErrorKind>;

struct Silent;
impl shared::Storage for Silent {
    const OBSERVED: bool = false;
    fn owner(&mut self, _: NodeId) {
        panic!("ordinary owner hook")
    }
    fn reserve<T>(&mut self, _: shared::Site, _: usize) -> Result<(), LinkedShaperError> {
        panic!("selection allocation")
    }
    fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
        panic!("selection retained output")
    }
    fn work(&mut self, _: u64) -> Result<(), LinkedShaperError> {
        panic!("ordinary work hook")
    }
}
#[derive(Default)]
struct Fees {
    requests: Vec<u64>,
    stop: Option<usize>,
    attempts: usize,
}
impl shared::Storage for Fees {
    fn reserve<T>(&mut self, _: shared::Site, _: usize) -> Result<(), LinkedShaperError> {
        panic!("selection allocation")
    }
    fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
        panic!("selection retained output")
    }
    fn work(&mut self, n: u64) -> Result<(), LinkedShaperError> {
        let at = self.attempts;
        self.attempts += 1;
        if self.stop == Some(at) {
            Err(LinkedShaperError::AllocationFailure)
        } else {
            assert!(self.stop.map_or(true, |stop| at < stop));
            self.requests.push(n);
            Ok(())
        }
    }
}
fn signature(value: (FontFaceId, BodyFont<'_>)) -> SelectionSignature {
    (
        value.0.get(),
        value.1.instance_id().get(),
        value.1.content_hash(),
    )
}
fn queries() -> Vec<Vec<String>> {
    [
        vec!["Zeta".to_owned()],
        vec!["和文".to_owned()],
        vec!["Missing".to_owned(), "Alpha".to_owned()],
        vec!["é".to_owned(), "e\u{0301}".to_owned()],
        vec!["Never".to_owned()],
        vec![],
        vec!["未登録".repeat(64), "和文".to_owned()],
        vec!["Alpha".to_owned(), "Zeta".to_owned()],
    ]
    .into_iter()
    .collect()
}
fn expected(families: &[String], legacy: bool) -> (SelectionResult, Vec<u64>) {
    let mut fees = vec![1];
    for family in families {
        fees.push(1 + (FAMILIES.len() as u64 + 1) * (family.len() as u64 + 2));
        if let Some(index) = FAMILIES.iter().position(|name| *name == family) {
            fees.extend(std::iter::repeat(1).take(if legacy { index + 1 } else { 1 }));
            return (Ok((index as u32, index as u32, sha256(FONT))), fees);
        }
    }
    (Err(ProductionTextShapeErrorKind::MissingSelectedFont), fees)
}
#[test]
fn font_selection_real_ledgers_preserve_fallback_order_and_legacy_dense_visits() {
    let (legacy, modern, _) = admit();
    let instances = AdmittedProductionFontInstancesV3::from_used_faces(
        &modern,
        modern.fonts().iter().map(|font| font.font_face_id()),
    )
    .unwrap();
    let mut cases = 0;
    let mut cuts = 0;
    for (fonts, is_legacy) in [
        (BodyFonts::Legacy(&legacy), true),
        (BodyFonts::BookV2(&instances, &modern), false),
    ] {
        for families in queries() {
            let (wanted, requests) = expected(&families, is_legacy);
            assert_eq!(
                fonts
                    .select_with_storage(&families, &mut Silent)
                    .map(signature),
                wanted
            );
            let mut full = Fees::default();
            assert_eq!(
                fonts
                    .select_with_storage(&families, &mut full)
                    .map(signature),
                wanted
            );
            assert_eq!(full.requests, requests);
            for stop in 0..requests.len() {
                let mut short = Fees {
                    stop: Some(stop),
                    ..Fees::default()
                };
                assert!(matches!(
                    fonts.select_with_storage(&families, &mut short),
                    Err(ProductionTextShapeErrorKind::Backend(
                        LinkedShaperError::AllocationFailure
                    ))
                ));
                assert_eq!(short.attempts, stop + 1);
                assert_eq!(short.requests, requests[..stop]);
                cuts += 1;
            }
            cases += 1;
        }
    }
    eprintln!("real admitted selector cases={cases}, every-request refusal cutoffs={cuts}, ordinary hooks=0");
}
#[test]
fn font_selection_number_source_restores_owner_and_prefix_at_every_work_cap() {
    let (legacy, modern, limits) = admit();
    let instances = AdmittedProductionFontInstancesV3::from_used_faces(
        &modern,
        modern.fonts().iter().map(|font| font.font_face_id()),
    )
    .unwrap();
    let mut cases = 0;
    let mut caps = 0;
    for (fonts, is_legacy) in [
        (BodyFonts::Legacy(&legacy), true),
        (BodyFonts::BookV2(&instances, &modern), false),
    ] {
        for families in queries() {
            let (wanted, fees) = expected(&families, is_legacy);
            let mut prefixes = vec![13u64];
            for fee in fees {
                prefixes.push(prefixes.last().unwrap() + fee);
            }
            let full = *prefixes.last().unwrap();
            for owner in [NodeId::new(91), NodeId::new(205)] {
                for cap in 13..=full + 1 {
                    let [mut r, mut s, mut w] = [7, 11, 13];
                    let actual = with_book_v2_observed_source_accounting(
                        &mut r,
                        &mut s,
                        &mut w,
                        limits.base(),
                        limits.base().get().max_fragments,
                        cap,
                        |accounting| {
                            select_font_observed(fonts, &families, owner, accounting).map(signature)
                        },
                    );
                    let expected = if cap < full {
                        Err(failure(
                            owner,
                            BookV2EquationNumberErrorKind::ObservedAccounting(
                                BookV2ObservedAccountingError::Work { owner },
                            ),
                        ))
                    } else {
                        wanted.map_err(|_| {
                            failure(owner, BookV2EquationNumberErrorKind::MissingSelectedFont)
                        })
                    };
                    assert_eq!(
                        actual,
                        expected,
                        "legacy={is_legacy}, owner={}, cap={cap}",
                        owner.get()
                    );
                    let accepted = *prefixes
                        .iter()
                        .take_while(|prefix| **prefix <= cap)
                        .last()
                        .unwrap();
                    assert_eq!([r, s, w], [7, 11, accepted]);
                    caps += 1;
                }
                cases += 1;
            }
        }
    }
    eprintln!("number selector Source cases={cases}, work caps={caps}, owners=91/205, records/spool unchanged");
}
