use super::super::super::base_cmap_cases as cases;
use super::*;

fn context() -> FontFailureContext {
    let mut c = FontFailureContext::new(0);
    c.phase = FontFailurePhase::Cmap;
    c.reason = FontFailureReason::InvalidTable;
    c.table_tag = Some(*b"cmap");
    c
}

#[test]
fn cff_v2_base_cmap_format12_reserved_field_fails_before_allocation() {
    for groups in [vec![], vec![(65, 67, 1)]] {
        for at in [14, 15] {
            for value in [1, 255] {
                let mut bytes = cases::cmap(&[(3, 10, cases::format12(&groups))]);
                bytes[at] = value;
                let mut context = context();
                assert!(matches!(
                    parse_with_reserve(&bytes, 8, &mut context, true, &mut |_, _| panic!(
                        "invalid reserved field precedes all allocation"
                    )),
                    Err(CffCmapFailureV2::Base {
                        kind: Cff1Error::InvalidCmap,
                        ..
                    })
                ));
                assert_eq!(context.cmap_format, Some(12));
                assert_eq!(context.table_offset, Some(12));
            }
        }
    }
}
fn each_case(mut apply: impl FnMut(&[u8], u16)) {
    for (_, bytes, glyphs) in cases::cases() {
        apply(&bytes, glyphs);
        for at in 0..bytes.len() {
            for value in [0u8, 1, 255] {
                if bytes[at] == value {
                    continue;
                }
                let mut changed = bytes.clone();
                changed[at] = value;
                apply(&changed, glyphs);
            }
        }
        for end in 0..bytes.len() {
            apply(&bytes[..end], glyphs);
        }
    }
}
#[test]
fn cff_v2_base_cmap_ordered_rows_match_legacy_tree_and_exact_semantic_contexts() {
    let (mut compared, mut variation_rejected) = (0, 0);
    each_case(|bytes, glyphs| {
        let Ok(variations) = validate_cff_variation_sequences_v2(bytes, glyphs) else {
            variation_rejected += 1;
            return;
        };
        let mut old = context();
        let mut new = context();
        let expected = parse_base_cmap(bytes, glyphs, &mut old, variations.is_some())
            .map(|map| map.into_iter().collect::<Vec<_>>())
            .map_err(|kind| CffCmapFailureV2::Base { kind, context: old });
        let actual = parse(bytes, glyphs, &mut new, variations.is_some());
        assert_eq!(actual, expected);
        assert_eq!(new, old);
        compared += 1;
    });
    eprintln!(
        "direct cmap semantic cases={compared}, prior variation rejections={variation_rejected}"
    );
}
#[test]
fn cff_v2_base_cmap_allocation_failures_are_typed_at_every_exact_row_request() {
    let (mut total, mut cuts, mut requests) = (0, 0, 0);
    for (_, bytes, glyphs) in cases::cases() {
        let Ok(variations) = validate_cff_variation_sequences_v2(&bytes, glyphs) else {
            continue;
        };
        let mut full = Vec::new();
        let expected = parse_with_reserve(
            &bytes,
            glyphs,
            &mut context(),
            variations.is_some(),
            &mut |rows, n| {
                assert!(rows.is_empty() && rows.capacity() == 0 && n > 0);
                full.push(n);
                reserve_rows(rows, n)
            },
        );
        for stop in 0..full.len() {
            let mut prefix = Vec::new();
            let actual = parse_with_reserve(
                &bytes,
                glyphs,
                &mut context(),
                variations.is_some(),
                &mut |rows, n| {
                    assert!(rows.is_empty() && rows.capacity() == 0 && n > 0);
                    let index = prefix.len();
                    assert!(index <= stop);
                    prefix.push(n);
                    if index == stop {
                        // A real capacity-overflow refusal, without asking the OS
                        // to exhaust memory or changing the global allocator.
                        reserve_rows(rows, usize::MAX)
                    } else {
                        reserve_rows(rows, n)
                    }
                },
            );
            assert_eq!(prefix, full[..=stop]);
            assert_eq!(
                actual,
                Err(CffCmapFailureV2::Storage(CffTableFailureV2 {
                    table: *b"cmap",
                    table_offset: 0,
                    kind: CffTableFailureKindV2::AllocationFailure,
                    limit: None,
                    observed: None,
                }))
            );
            cuts += 1;
        }
        assert_eq!(
            parse(&bytes, glyphs, &mut context(), variations.is_some()),
            expected
        );
        requests += full.len();
        total += 1;
    }
    eprintln!(
        "direct cmap allocation cases={total}, requests={requests}, real capacity failures={cuts}"
    );
}
#[test]
fn cff_v2_base_cmap_aliases_subsets_and_conflicts_do_not_allocate_a_merge_vector() {
    for (left, right) in [
        (vec![(1, 1), (2, 2)], vec![(1, 1), (2, 2)]),
        (vec![(1, 1), (2, 2), (3, 3)], vec![(2, 2)]),
        (vec![(2, 2)], vec![(1, 1), (2, 2), (3, 3)]),
        (vec![], vec![(1, 1)]),
        (vec![(1, 1)], vec![]),
    ] {
        let expected = left
            .iter()
            .chain(&right)
            .copied()
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect::<Vec<_>>();
        let left_pointer = left.as_ptr();
        let right_pointer = right.as_ptr();
        let actual = merge(left, right, &mut |_, _| {
            panic!("an existing complete vector must be kept")
        })
        .unwrap();
        assert_eq!(actual, expected);
        assert!(actual.as_ptr() == left_pointer || actual.as_ptr() == right_pointer);
    }
    assert!(matches!(
        merge(vec![(1, 1)], vec![(1, 2)], &mut |_, _| panic!(
            "conflict precedes allocation"
        )),
        Err(Failure::Base(Cff1Error::InvalidCmap))
    ));
    let mut requested = Vec::new();
    let actual = merge(
        vec![(1, 1), (3, 3)],
        vec![(2, 2), (3, 3), (4, 4)],
        &mut |rows, n| {
            requested.push(n);
            reserve_rows(rows, n)
        },
    )
    .unwrap();
    assert_eq!(requested, [4]);
    assert_eq!(actual, [(1, 1), (2, 2), (3, 3), (4, 4)]);
}
