use super::*;
use typaxis_text::GeneratedProvenance;
#[derive(Default)]
struct Visits {
    calls: usize,
    stop: Option<usize>,
}
impl Visits {
    fn work(&mut self, n: u64) -> Result<(), usize> {
        assert_eq!(n, 1);
        let at = self.calls;
        self.calls += 1;
        if self.stop == Some(at) {
            Err(91)
        } else {
            assert!(self.stop.map_or(true, |stop| at < stop));
            Ok(())
        }
    }
}
fn check<'a>(
    expected: Option<(GeneratedProvenance, &'a str)>,
    mut apply: impl FnMut(&mut Visits) -> Result<Option<(GeneratedProvenance, &'a str)>, usize>,
) -> usize {
    let mut full = Visits::default();
    assert_eq!(apply(&mut full).unwrap(), expected);
    for stop in 0..full.calls {
        let mut short = Visits {
            stop: Some(stop),
            ..Visits::default()
        };
        assert_eq!(apply(&mut short), Err(91));
        assert_eq!(short.calls, stop + 1);
    }
    full.calls
}
#[test]
fn generated_lookup_work_real_flows_keep_page_counter_list_footnote_and_missing_results() {
    let mut cases = 0;
    let mut cuts = 0;
    for mode in 0..3 {
        let mut source = input();
        if mode == 2 {
            source["document"]["blocks"][1]["children"][1]["format"] = "text".into();
        }
        let body = styled(&source, &limits());
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let flow = if mode == 1 {
            prepare_book_v2_text_flow_with_page_references(&body, &nav, &[(NodeId::new(7), 12)])
                .unwrap()
        } else {
            prepare_book_v2_text_flow(&body, &nav).unwrap()
        };
        for owner in [
            NodeId::new(0),
            NodeId::new(7),
            NodeId::new(11),
            NodeId::new(92),
            NodeId::new(u32::MAX),
        ] {
            let expected = flow
                .reference_provenance(owner)
                .zip(flow.reference_text(owner));
            cuts += check(expected, |v| {
                flow.reference_with_work(owner, &mut |n| v.work(n))
            });
            cases += 1;
            let expected = flow
                .footnote_marker_provenance(owner)
                .zip(flow.footnote_marker_text(owner));
            cuts += check(expected, |v| {
                flow.footnote_marker_with_work(owner, &mut |n| v.work(n))
            });
            cases += 1;
        }
        for index in 0..flow.list_items().len() + 2 {
            let expected = flow
                .list_marker_provenance(index)
                .zip(flow.list_marker_text(index));
            cuts += check(expected, |v| {
                flow.list_marker_with_work(index, &mut |n| v.work(n))
            });
            cases += 1;
        }
        flow.verify_for(&body, &nav).unwrap();
    }
    eprintln!("generated flow lookup cases={cases}, work refusal cutoffs={cuts}");
}
#[test]
fn generated_lookup_work_empty_page_keeps_priority_over_counter() {
    let body = styled(&input(), &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let owner = NodeId::new(7);
    let mut flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    flow.generated = typaxis_text::GeneratedTextOverlay::new(
        vec![
            (
                typaxis_core::GeneratedBufferKey::new(
                    owner,
                    typaxis_core::GenerationKind::PageReference,
                    0,
                ),
                String::new(),
            ),
            (
                typaxis_core::GeneratedBufferKey::new(
                    owner,
                    typaxis_core::GenerationKind::Counter,
                    0,
                ),
                "counter fallback".into(),
            ),
        ],
        body.body().limits(),
        0,
    )
    .unwrap();
    assert_eq!(flow.reference_text(owner), Some(""));
    let provenance = flow.reference_provenance(owner).unwrap();
    assert_eq!(
        provenance.buffer_key().generation_kind(),
        typaxis_core::GenerationKind::PageReference
    );
    let cuts = check(Some((provenance, "")), |v| {
        flow.reference_with_work(owner, &mut |n| v.work(n))
    });
    assert_eq!(
        flow.verify_for(&body, &nav).unwrap_err().kind,
        ProductionFlowErrorKind::ReceiptMismatch
    );
    eprintln!("empty page priority refusal cutoffs={cuts}; corrupted flow remains rejected");
}
