use super::*;

#[test]
fn family_work_lookup_matches_existing_order_and_refuses_each_atomic_request() {
    let mut cases = 0;
    let mut cuts = 0;
    let mut atomic_cuts = 0;
    for count in 0..=64usize {
        let mut declarations = (0..count)
            .map(|n| (format!("Font{n:04}"), FontFaceId::new(n as u32)))
            .collect::<Vec<_>>();
        declarations.extend([
            ("Body".into(), FontFaceId::new(101)),
            ("body".into(), FontFaceId::new(102)),
            ("é".into(), FontFaceId::new(103)),
            ("e\u{0301}".into(), FontFaceId::new(104)),
        ]);
        let declared = declarations.len();
        let table = FontFamilyTable::new(declarations).unwrap();
        let last = format!("Font{:04}", count.saturating_sub(1));
        let queries = [
            vec![],
            vec![String::new()],
            vec!["Body".into(), "body".into()],
            vec!["body".into(), "Body".into()],
            vec!["Missing".into(), last, "Body".into()],
            vec!["é".into()],
            vec!["e\u{0301}".into()],
            vec!["未登録".repeat(256), "Body".into()],
            vec!["\n".into()],
            vec!["Missing".repeat(1024)],
        ];
        for fallback in queries {
            let mut expected_requests = vec![1u64];
            for name in &fallback {
                expected_requests.push(1 + (declared as u64 + 1) * (name.len() as u64 + 2));
                if table.resolve(std::slice::from_ref(name)).is_ok() {
                    break;
                }
            }
            let ordinary = table
                .resolve(&fallback)
                .map_err(FontFamilyWorkError::Family::<u8>);
            let mut requests = Vec::new();
            let result = table.try_resolve_with_work(&fallback, &mut |n| {
                requests.push(n);
                Ok::<(), u8>(())
            });
            assert_eq!(result, ordinary);
            assert_eq!(requests, expected_requests);
            for stop in 0..requests.len() {
                let mut attempts = 0;
                let mut accepted = Vec::new();
                let refused = table.try_resolve_with_work(&fallback, &mut |n| {
                    let at = attempts;
                    attempts += 1;
                    assert!(at <= stop);
                    if at == stop {
                        Err(77u8)
                    } else {
                        accepted.push(n);
                        Ok(())
                    }
                });
                assert_eq!(refused, Err(FontFamilyWorkError::Refused(77)));
                assert_eq!(attempts, stop + 1);
                assert_eq!(accepted, requests[..stop]);
                cuts += 1;
                let prefix = 17 + requests[..stop].iter().sum::<u64>();
                let cap = prefix + requests[stop] - 1;
                let mut total = 17u64;
                let mut attempts = 0;
                let refused = table.try_resolve_with_work(&fallback, &mut |n| {
                    attempts += 1;
                    let next = total + n;
                    if next > cap {
                        Err(91u8)
                    } else {
                        total = next;
                        Ok(())
                    }
                });
                assert_eq!(refused, Err(FontFamilyWorkError::Refused(91)));
                assert_eq!(attempts, stop + 1);
                assert_eq!(total, prefix);
                atomic_cuts += 1;
            }
            cases += 1;
        }
    }
    let empty = FontFamilyTable::new(vec![]).unwrap();
    let mut fees = Vec::new();
    assert_eq!(
        empty.try_resolve_with_work(&["Body".into()], &mut |n| {
            fees.push(n);
            Ok::<(), u8>(())
        }),
        Err(FontFamilyWorkError::Family(FontFamilyError::UnknownFamily))
    );
    assert_eq!(fees, [1, 7]);
    eprintln!("family work fallback cases={cases}, request refusal cutoffs={cuts}, atomic one-short cutoffs={atomic_cuts}");
}

#[test]
fn family_work_checked_allowance_covers_query_bytes_and_rejects_overflow() {
    for families in [0usize, 1, 2, 11, 64, 1024] {
        for bytes in [0usize, 1, 4, 7, 32, 4096, 65536] {
            assert_eq!(
                lookup_allowance(families, bytes),
                Some(1 + (families as u64 + 1) * (bytes as u64 + 2))
            );
        }
    }
    assert_eq!(lookup_allowance(usize::MAX, usize::MAX), None);
    assert_eq!(lookup_allowance(0, 0), Some(3));
}
