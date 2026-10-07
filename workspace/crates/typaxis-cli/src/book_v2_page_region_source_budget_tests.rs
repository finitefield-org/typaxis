use super::*;
use typaxis_syntax::book_v2::prepare_book_v2_page_region_text_flow_counted as counted;
use typaxis_syntax::ProductionFlowErrorKind;

#[test]
fn book_v2_page_region_source_budget_retains_each_accepted_reservation() {
    check(None);
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_page_region_source_budget_retains_original_harano_reservations() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&font));
}

fn check(font: Option<&[u8]>) {
    let limits = limits();
    let cap = limits.base().get().max_fragments;
    let text = if font.is_some() {
        "本文の柱"
    } else {
        "Result"
    };
    for missing_style in [false, true] {
        let root = Root::new();
        let mut data = region_data(text);
        let mut second = data["page_masters"]["masters"][0]["header_content"]["blocks"][0].clone();
        second["node_id"] = 13.into();
        for (index, child) in second["children"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            child["node_id"] = (14 + index).into();
        }
        data["page_masters"]["masters"][0]["header_content"]["blocks"]
            .as_array_mut()
            .unwrap()
            .push(second);
        if missing_style {
            data["style_sheet"]["rules"]
                .as_array_mut()
                .unwrap()
                .retain(|rule| rule["selector"] != "heading");
        }
        let input = if let Some(font) = font {
            vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
        } else {
            prepared(&root, data, text.as_bytes(), &limits)
        };
        let body = input.body().styled();
        let nav = prepare_book_v2_navigation(body).unwrap();
        let selected = select_book_v2_page_master(body, 0, None, &mut 0, 1_000_000).unwrap();
        for (kind, reservations) in [(Kind::Header, vec![1, 8, 8]), (Kind::Footer, vec![1, 5])] {
            let total: u64 = reservations.iter().sum();
            let mut records = u64::MAX;
            let outcome = counted(selected, kind, &nav, 7, &mut records);
            assert_eq!(records, 7 + total);
            let legacy = region_flow(selected, kind, &nav, 7);
            if missing_style && kind == Kind::Footer {
                let cause = outcome.err().unwrap();
                assert_eq!(cause.kind, ProductionFlowErrorKind::MissingTextStyle);
                assert_eq!(cause, legacy.err().unwrap());
                assert_eq!(
                    cause,
                    counted(selected, kind, &nav, records, &mut records)
                        .err()
                        .unwrap()
                );
                assert_eq!(records, 7 + 2 * total);
            } else {
                let flow = outcome.unwrap();
                assert_eq!(flow.record_charge(), records);
                assert_eq!(flow.fingerprint(), legacy.unwrap().fingerprint());
                assert_eq!(
                    flow.text_flow().text_bytes(),
                    (text.len() * if kind == Kind::Header { 4 } else { 1 }) as u64
                );
            }
            // Every position before the next indivisible owner/block reservation.
            for remaining in 0..total {
                let prior = cap - remaining;
                let mut expected = prior;
                for charge in &reservations {
                    if expected + charge > cap {
                        break;
                    }
                    expected += charge;
                }
                let cause = counted(selected, kind, &nav, prior, &mut records)
                    .err()
                    .unwrap();
                assert_eq!(cause.kind, ProductionFlowErrorKind::NodeLimit);
                assert_eq!(
                    cause,
                    region_flow(selected, kind, &nav, prior).err().unwrap()
                );
                assert_eq!(records, expected);
                let retry_prior = records;
                assert!(counted(selected, kind, &nav, retry_prior, &mut records).is_err());
                assert!(records >= retry_prior && records <= cap);
            }
            let exact = counted(selected, kind, &nav, cap - total, &mut records);
            assert_eq!(records, cap);
            if missing_style && kind == Kind::Footer {
                assert_eq!(
                    exact.err().unwrap().kind,
                    ProductionFlowErrorKind::MissingTextStyle
                );
            } else {
                assert_eq!(
                    exact.unwrap().fingerprint(),
                    region_flow(selected, kind, &nav, 0).unwrap().fingerprint()
                );
            }
            assert!(counted(selected, kind, &nav, u64::MAX, &mut records).is_err());
            assert_eq!(records, u64::MAX);
        }
        // Identity and absent-region failures precede any source reservation.
        let other_root = Root::new();
        let other = prepared(&other_root, source_data(text), text.as_bytes(), &limits);
        let other_body = other.body().styled();
        let other_nav = prepare_book_v2_navigation(other_body).unwrap();
        let absent = select_book_v2_page_master(other_body, 0, None, &mut 0, 1_000_000).unwrap();
        for (selection, navigation) in [(selected, &other_nav), (absent, &other_nav)] {
            let mut records = 0;
            let cause = counted(selection, Kind::Header, navigation, 29, &mut records)
                .err()
                .unwrap();
            assert_eq!(cause.kind, ProductionFlowErrorKind::ReceiptMismatch);
            assert_eq!(records, 29);
        }
    }
}
