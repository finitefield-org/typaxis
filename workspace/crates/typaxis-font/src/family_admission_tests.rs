use super::*;
#[derive(Clone, Debug, Eq, PartialEq)]
enum Request {
    Reserve(FontFamilyStorage, Option<FontFaceId>, usize, usize),
    Work(FontFamilyAdmissionStage, FontFaceId, u64),
}
#[derive(Default)]
struct Trace {
    requests: Vec<Request>,
    stop: Option<usize>,
}
impl Trace {
    fn request(&mut self, r: Request) -> Result<(), usize> {
        let at = self.requests.len();
        if self.stop == Some(at) {
            return Err(at);
        }
        self.requests.push(r);
        Ok(())
    }
}
impl FontFamilyAdmissionBudget for Trace {
    type Error = usize;
    fn reserve(
        &mut self,
        s: FontFamilyStorage,
        f: Option<FontFaceId>,
        r: usize,
        b: usize,
    ) -> Result<(), usize> {
        self.request(Request::Reserve(s, f, r, b))
    }
    fn work(&mut self, s: FontFamilyAdmissionStage, f: FontFaceId, n: u64) -> Result<(), usize> {
        self.request(Request::Work(s, f, n))
    }
}
#[test]
fn borrowed_family_index_matches_native_lookup_equality_and_refuses_each_small_request() {
    let mut total = 0;
    for count in [0, 1, 2, 16, 64, 512] {
        let names = (0..count)
            .map(|i| format!("Family-{i:04}-和\"😀"))
            .collect::<Vec<_>>();
        for order in 0..3 {
            let mut input = names
                .iter()
                .enumerate()
                .map(|(i, n)| (n.as_str(), FontFaceId::new(i as u32)))
                .collect::<Vec<_>>();
            if order == 1 {
                input.reverse();
            }
            if order == 2 && !input.is_empty() {
                input.rotate_left(count / 2);
            }
            let native =
                FontFamilyTable::new(input.iter().map(|(n, f)| ((*n).to_owned(), *f)).collect())
                    .unwrap();
            let mut full = Trace::default();
            let sorted =
                FontFamilyTable::try_from_borrowed_with_budget(input.iter().copied(), &mut full)
                    .unwrap();
            assert_eq!(native, sorted);
            assert_eq!(sorted, native);
            assert_eq!(sorted, sorted.clone());
            for query in names
                .iter()
                .map(|n| vec!["missing".to_owned(), n.clone()])
                .chain([
                    vec![],
                    vec!["missing".to_owned()],
                    names.iter().rev().take(2).cloned().collect(),
                ])
            {
                assert_eq!(native.resolve(&query), sorted.resolve(&query));
                let mut native_work = Vec::new();
                let mut sorted_work = Vec::new();
                assert_eq!(
                    native.try_resolve_with_work(&query, &mut |n| {
                        native_work.push(n);
                        Ok::<_, ()>(())
                    }),
                    sorted.try_resolve_with_work(&query, &mut |n| {
                        sorted_work.push(n);
                        Ok::<_, ()>(())
                    })
                );
                assert_eq!(native_work, sorted_work);
            }
            assert_eq!(full.requests.len(), 5 * count + usize::from(count > 0));
            let mut refusals = 0;
            for stop in 0..full.requests.len() {
                if count > 16 && stop != 0 && stop + 1 != full.requests.len() {
                    continue;
                }
                let mut short = Trace {
                    stop: Some(stop),
                    ..Trace::default()
                };
                let FontFamilyAdmissionError::Refused { error, face, stage } =
                    FontFamilyTable::try_from_borrowed_with_budget(
                        input.iter().copied(),
                        &mut short,
                    )
                    .unwrap_err()
                else {
                    panic!("wrong refusal")
                };
                let (expected_face, expected_stage) = match full.requests[stop] {
                    Request::Work(s, f, _) => (Some(f), s),
                    Request::Reserve(s, f, _, _) => (
                        f,
                        match s {
                            FontFamilyStorage::Entries => FontFamilyAdmissionStage::Entries,
                            FontFamilyStorage::NameBytes => FontFamilyAdmissionStage::NameCopy,
                        },
                    ),
                };
                assert_eq!((error, face, stage), (stop, expected_face, expected_stage));
                assert_eq!(short.requests, full.requests[..stop]);
                short.stop = None;
                let retry = FontFamilyTable::try_from_borrowed_with_budget(
                    input.iter().copied(),
                    &mut short,
                )
                .unwrap();
                assert_eq!(retry, native);
                assert_eq!(&short.requests[stop..], full.requests);
                refusals += 1;
            }
            total += refusals;
            eprintln!(
                "font families count={count} order={order} requests={} refusals={refusals}",
                full.requests.len()
            );
        }
    }
    eprintln!("font family callback refusals={total}");
}
#[test]
fn borrowed_family_index_preserves_native_validation_and_duplicate_priority() {
    let cases = [
        vec![""],
        vec!["  \t"],
        vec!["\u{2000}"],
        vec!["A\0"],
        vec!["A\u{0085}"],
        vec!["A", "A"],
        vec!["A", "", "A"],
        vec!["A", "A", ""],
        vec!["A", "a", "和", "é", "e\u{301}", "\u{2000}A\u{2000}"],
    ];
    for names in cases {
        let input = names
            .iter()
            .enumerate()
            .map(|(i, n)| (*n, FontFaceId::new(i as u32)))
            .collect::<Vec<_>>();
        let native =
            FontFamilyTable::new(input.iter().map(|(n, f)| ((*n).to_owned(), *f)).collect());
        let mut trace = Trace::default();
        let observed =
            FontFamilyTable::try_from_borrowed_with_budget(input.into_iter(), &mut trace);
        match (native, observed) {
            (Ok(a), Ok(z)) => assert_eq!(a, z),
            (Err(a), Err(FontFamilyAdmissionError::Family(z))) => assert_eq!(a, z),
            (a, z) => panic!("native mismatch: {a:?} / {z:?}"),
        }
    }
}

#[test]
fn borrowed_family_index_rejects_inconsistent_counts_and_capacity_overflow() {
    struct Counted {
        remaining: usize,
        reported: usize,
    }
    impl Iterator for Counted {
        type Item = (&'static str, FontFaceId);
        fn next(&mut self) -> Option<Self::Item> {
            if self.remaining == 0 {
                None
            } else {
                self.remaining -= 1;
                Some(("A", FontFaceId::new(0)))
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.reported, Some(self.reported))
        }
    }
    impl ExactSizeIterator for Counted {}
    for (remaining, reported) in [(1, 0), (0, 1), (1, 2)] {
        assert!(matches!(
            FontFamilyTable::try_from_borrowed_with_budget(
                Counted {
                    remaining,
                    reported
                },
                &mut Trace::default()
            ),
            Err(FontFamilyAdmissionError::IteratorLength)
        ));
    }
    let mut trace = Trace::default();
    assert!(matches!(
        FontFamilyTable::try_from_borrowed_with_budget(
            Counted {
                remaining: 0,
                reported: usize::MAX
            },
            &mut trace
        ),
        Err(FontFamilyAdmissionError::ArithmeticOverflow)
    ));
    assert!(trace.requests.is_empty());
}
