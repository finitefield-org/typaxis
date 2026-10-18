extern crate std;
use self::std::{borrow::ToOwned, eprintln, vec::Vec};
use crate::UnicodeSegmentation;
#[path = "../tests/testdata/mod.rs"]
#[allow(dead_code)]
mod testdata;

#[test]
fn grapheme_work_unicode16_goldens_and_every_forward_request_refusal_restore_cursor() {
    let mut cases = 0;
    let mut cutoffs = 0;
    let mut visits = 0;
    let cases_same = testdata::TEST_SAME
        .iter()
        .flat_map(|&(s, clusters)| [(s, true, clusters), (s, false, clusters)]);
    let cases_diff = testdata::TEST_DIFF
        .iter()
        .flat_map(|&(s, extended, legacy)| [(s, true, extended), (s, false, legacy)]);
    for (text, extended, expected) in cases_same.chain(cases_diff) {
        assert_eq!(text.graphemes(extended).collect::<Vec<_>>(), expected);
        let mut iter = text.grapheme_indices(extended);
        let mut offset = 0;
        for cluster in expected
            .iter()
            .copied()
            .map(Some)
            .chain(core::iter::once(None))
        {
            let before = iter.as_str();
            let mut complete = iter.clone();
            let mut calls = 0;
            let actual = complete
                .try_next_with_work(&mut || {
                    calls += 1;
                    Ok::<(), u8>(())
                })
                .unwrap();
            assert_eq!(actual, cluster.map(|s| (offset, s)));
            for stop in 0..calls {
                let mut short = iter.clone();
                let mut attempts = 0;
                let result = short.try_next_with_work(&mut || {
                    let at = attempts;
                    attempts += 1;
                    if at == stop {
                        Err(77u8)
                    } else {
                        Ok(())
                    }
                });
                assert_eq!(result, Err(77));
                assert_eq!(attempts, stop + 1);
                assert_eq!(short.as_str(), before);
                assert_eq!(
                    short.try_next_with_work(&mut || Ok::<(), u8>(())).unwrap(),
                    actual
                );
                assert_eq!(short.as_str(), complete.as_str());
                cutoffs += 1;
            }
            if let Some(cluster) = cluster {
                offset += cluster.len();
            }
            iter = complete;
            visits += calls;
        }
        cases += 1;
    }
    eprintln!("grapheme Unicode16 golden cases={cases}, paid visits={visits}, refusal/retry cutoffs={cutoffs}");
}

#[test]
fn grapheme_work_long_lookback_and_mixed_direction_iteration_match_ordinary() {
    let inputs = [
        "क्".repeat(64) + "क",
        "🇦".repeat(64),
        "👩\u{200d}".repeat(64) + "👩",
        "a".to_owned() + &"\u{0308}".repeat(1024),
    ];
    for text in &inputs {
        let ordinary = text.graphemes(true).collect::<Vec<_>>();
        let mut actual = Vec::new();
        let mut iter = text.graphemes(true);
        let mut calls = 0;
        while let Some(cluster) = iter
            .try_next_with_work(&mut || {
                calls += 1;
                Ok::<(), u8>(())
            })
            .unwrap()
        {
            actual.push(cluster);
        }
        assert_eq!(actual, ordinary);
        let mut forward = text.graphemes(true);
        let mut reference = forward.clone();
        assert_eq!(forward.next_back(), reference.next_back());
        assert_eq!(
            forward
                .try_next_with_work(&mut || Ok::<(), u8>(()))
                .unwrap(),
            reference.next()
        );
        let mut full = text.graphemes(true);
        let mut next_calls = 0;
        let expected = full
            .try_next_with_work(&mut || {
                next_calls += 1;
                Ok::<(), u8>(())
            })
            .unwrap();
        for stop in [0, 1, next_calls / 2, next_calls - 1] {
            let mut short = text.graphemes(true);
            let mut attempts = 0;
            assert_eq!(
                short.try_next_with_work(&mut || {
                    let at = attempts;
                    attempts += 1;
                    if at == stop {
                        Err(9u8)
                    } else {
                        Ok(())
                    }
                }),
                Err(9)
            );
            assert_eq!(short.as_str(), text);
            assert_eq!(attempts, stop + 1);
            assert_eq!(
                short.try_next_with_work(&mut || Ok::<(), u8>(())).unwrap(),
                expected
            );
        }
        eprintln!(
            "grapheme lookback bytes={}, graphemes={}, paid visits={calls}",
            text.len(),
            actual.len()
        );
    }
}
