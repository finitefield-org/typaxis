use super::*;
use typaxis_core::{GenerationKind, NodeId, ResourceLimits};
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
            Err(77)
        } else {
            assert!(self.stop.map_or(true, |stop| at < stop));
            Ok(())
        }
    }
}
fn key(index: usize) -> GeneratedBufferKey {
    GeneratedBufferKey::new(
        NodeId::new((2 * index + 1) as u32),
        if index % 2 == 0 {
            GenerationKind::ListMarker
        } else {
            GenerationKind::FootnoteMarker
        },
        (index % 3) as u32,
    )
}
#[test]
fn overlay_lookup_work_matches_native_search_and_refuses_every_entry_and_probe() {
    let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
    let mut cases = 0;
    let mut cuts = 0;
    for len in [0, 1, 2, 3, 4, 7, 8, 16, 31, 64, 127, 255] {
        let overlay = GeneratedTextOverlay::new(
            (0..len).map(|i| (key(i), format!("value-{i}"))).collect(),
            &limits,
            0,
        )
        .unwrap();
        for owner in 0..2 * len + 3 {
            for kind in [
                GenerationKind::PageReference,
                GenerationKind::Counter,
                GenerationKind::ListMarker,
                GenerationKind::FootnoteMarker,
                GenerationKind::Discretionary,
            ] {
                for ordinal in 0..4 {
                    let query = GeneratedBufferKey::new(NodeId::new(owner as u32), kind, ordinal);
                    let expected = overlay.buffer(query);
                    let mut full = Visits::default();
                    let actual = overlay
                        .try_buffer_with_work(query, &mut |n| full.work(n))
                        .unwrap();
                    assert_eq!(actual, expected);
                    if let (Some(a), Some(b)) = (actual, expected) {
                        assert!(std::ptr::eq(a, b));
                    }
                    for stop in 0..full.calls {
                        let mut short = Visits {
                            stop: Some(stop),
                            ..Visits::default()
                        };
                        assert_eq!(
                            overlay.try_buffer_with_work(query, &mut |n| short.work(n)),
                            Err(77)
                        );
                        assert_eq!(short.calls, stop + 1);
                        cuts += 1;
                    }
                    cases += 1;
                }
            }
        }
    }
    eprintln!("overlay binary lookup cases={cases}, work refusal cutoffs={cuts}");
}
#[test]
fn overlay_lookup_work_provenance_keeps_unknown_utf8_empty_and_reversed_span_errors() {
    let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
    let overlay = GeneratedTextOverlay::new(
        vec![(key(0), "aé日".into()), (key(1), String::new())],
        &limits,
        0,
    )
    .unwrap();
    let mut cases = 0;
    let mut cuts = 0;
    for query in [key(0), key(1), key(2)] {
        for (start, end) in [
            (0, 6),
            (0, 1),
            (1, 3),
            (2, 3),
            (3, 6),
            (6, 6),
            (6, 0),
            (0, 9),
            (u32::MAX, u32::MAX),
        ] {
            let start = Utf8ByteOffset::new(start);
            let end = Utf8ByteOffset::new(end);
            let expected = overlay
                .provenance(query, start, end)
                .map_err(GeneratedOverlayWorkError::Text::<usize>);
            let mut full = Visits::default();
            assert_eq!(
                overlay.try_provenance_with_work(query, start, end, &mut |n| full.work(n)),
                expected
            );
            for stop in 0..full.calls {
                let mut short = Visits {
                    stop: Some(stop),
                    ..Visits::default()
                };
                assert_eq!(
                    overlay.try_provenance_with_work(query, start, end, &mut |n| short.work(n)),
                    Err(GeneratedOverlayWorkError::Budget(77))
                );
                assert_eq!(short.calls, stop + 1);
                cuts += 1;
            }
            cases += 1;
        }
    }
    eprintln!("overlay provenance cases={cases}, work refusal cutoffs={cuts}");
}
