use super::*;
#[path = "cff_font_metadata_cases.rs"]
mod cases;
#[derive(Clone, Debug, Eq, PartialEq)]
enum Request {
    Reserve(Site, usize, usize),
    Work(u64),
}
impl Request {
    fn delta(&self) -> [u64; 3] {
        match *self {
            Self::Reserve(_, r, b) => [r as u64, b as u64, 0],
            Self::Work(w) => [0, 0, w],
        }
    }
}
#[derive(Default)]
struct Trace {
    requests: Vec<Request>,
    paid: [u64; 3],
    stop: Option<usize>,
}
impl Trace {
    fn request(&mut self, r: Request) -> Result<(), usize> {
        let index = self.requests.len();
        assert!(self.stop.map_or(true, |stop| index <= stop));
        let delta = r.delta();
        self.requests.push(r);
        if self.stop == Some(index) {
            return Err(91);
        }
        for (v, n) in self.paid.iter_mut().zip(delta) {
            *v += n;
        }
        Ok(())
    }
}
impl CffBaseCmapBudgetV2 for Trace {
    type Error = usize;
    fn reserve(&mut self, _: CffBaseCmapStorageV2, _: usize, _: usize) -> Result<(), usize> {
        panic!("metadata cannot reserve cmap")
    }
    fn work(&mut self, _: u64) -> Result<(), usize> {
        panic!("metadata must use its own work scope")
    }
}
impl CffCmapBudgetV2 for Trace {
    fn reserve_variation(
        &mut self,
        _: CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        panic!("metadata cannot reserve variations")
    }
}
impl CffAdmissionBudgetV2 for Trace {
    const OBSERVE_METADATA: bool = true;
    fn reserve_sfnt(&mut self, _: CffSfntStorageV2, _: usize, _: usize) -> Result<(), usize> {
        panic!("metadata cannot reserve SFNT")
    }
    fn work_sfnt(&mut self, _: u64) -> Result<(), usize> {
        panic!("metadata cannot inspect SFNT")
    }
    fn reserve_metadata(&mut self, site: Site, rows: usize, bytes: usize) -> Result<(), usize> {
        self.request(Request::Reserve(site, rows, bytes))
    }
    fn work_metadata(&mut self, n: u64) -> Result<(), usize> {
        assert!((1..=257).contains(&n));
        self.request(Request::Work(n))
    }
}
fn prefix(requests: &[Request]) -> [u64; 3] {
    let mut paid = [0; 3];
    for r in requests {
        for (v, n) in paid.iter_mut().zip(r.delta()) {
            *v += n;
        }
    }
    paid
}
fn every_cut<T: std::fmt::Debug>(
    full: &Trace,
    mut run: impl FnMut(&mut Trace) -> Result<T, Failure<usize>>,
) -> usize {
    for stop in 0..full.requests.len() {
        let mut short = Trace {
            stop: Some(stop),
            ..Trace::default()
        };
        let Failure::Budget(f) = run(&mut short).unwrap_err() else {
            panic!("expected exact callback refusal")
        };
        assert_eq!(f.error, 91);
        assert_eq!(short.requests, full.requests[..=stop]);
        assert_eq!(short.paid, prefix(&full.requests[..stop]));
        assert_eq!(f.point.file_offset, None);
    }
    full.requests.len()
}
fn native<T>(result: Result<T, Failure<usize>>) -> Result<T, Cff1Error> {
    result.map_err(|e| match e {
        Failure::Native(e) => e,
        other => panic!("unexpected {other:?}"),
    })
}
fn names_tuple(n: NameFacts) -> (String, String, String) {
    (n.family, n.subfamily, n.postscript_name)
}
#[test]
fn font_metadata_names_preserve_native_selection_errors_and_every_callback_prefix() {
    let (mut count, mut cuts) = (0, 0);
    for (_, bytes) in cases::names() {
        let expected = parse_name(&bytes).map(names_tuple);
        let mut trace = Trace::default();
        assert_eq!(native(names(&bytes, &mut trace)).map(names_tuple), expected);
        cuts += every_cut(&trace, |t| names(&bytes, t));
        count += 1;
    }
    eprintln!("metadata name native outcomes={count}, all-request refusal cuts={cuts}");
}
#[test]
fn font_metadata_metrics_preserve_native_values_errors_and_every_callback_prefix() {
    let (mut count, mut cuts) = (0, 0);
    for (g, n, bytes) in cases::metrics() {
        let expected = parse_hmtx(&bytes, g, n);
        let mut trace = Trace::default();
        assert_eq!(native(hmtx(&bytes, g, n, &mut trace)), expected);
        cuts += every_cut(&trace, |t| hmtx(&bytes, g, n, t));
        count += 1;
    }
    // SFNT fixture is deliberately a structural negative after names/metrics.
    for (_, bytes) in cases::cases().into_iter().take(1) {
        let limits = M4EffectiveResourceLimits::new(
            typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
                .unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap();
        let failure = admit_sfnt_cff1_v2(std::sync::Arc::from(bytes), 0, &limits).unwrap_err();
        assert!(
            matches!(failure.kind, Cff1FailureKindV2::Program(_)),
            "{failure:?}"
        );
    }
    eprintln!("metadata metric native outcomes={count}, all-request refusal cuts={cuts}");
}
#[test]
fn font_metadata_hashes_match_original_identity_bytes_and_all_refusal_boundaries() {
    let (mut count, mut cuts) = (0, 0);
    for len in [
        0, 1, 55, 56, 63, 64, 65, 127, 128, 255, 256, 257, 511, 512, 513, 1025, 4097,
    ] {
        let source = (0..len)
            .map(|i| ((i * 73 + 11) % 251) as u8)
            .collect::<Vec<_>>();
        let limits = sha256(b"limits");
        let source_hash = sha256(&source);
        let mut expected = String::from(std::str::from_utf8(PREFIX).unwrap());
        crate::cff::push_hash(&mut expected, limits);
        expected.push_str(std::str::from_utf8(MIDDLE).unwrap());
        crate::cff::push_hash(&mut expected, source_hash);
        expected.push('}');
        assert_eq!(expected.len(), IDENTITY_LEN);
        let mut trace = Trace::default();
        assert_eq!(
            hashes(&source, limits, &mut trace).unwrap(),
            (source_hash, sha256(expected.as_bytes()))
        );
        assert_eq!(trace.paid[0..2], [0, 0]);
        for stop in 0..trace.requests.len() {
            let mut short = Trace {
                stop: Some(stop),
                ..Trace::default()
            };
            let Failure::Budget(f) = hashes(&source, limits, &mut short).unwrap_err() else {
                panic!()
            };
            assert_eq!(f.error, 91);
            assert_eq!(short.requests, trace.requests[..=stop]);
            assert_eq!(short.paid, prefix(&trace.requests[..stop]));
            assert_eq!(f.point.table_tag, None);
            assert_eq!(f.point.table_offset, None);
            if f.point.stage == Stage::SourceDigest {
                assert!(f.point.file_offset.is_some_and(|n| n <= len as u64));
            } else {
                assert_eq!(f.point.file_offset, None);
            }
            cuts += 1;
        }
        count += 1;
    }
    eprintln!("metadata exact identity/hash cases={count}, refusal cuts={cuts}");
}
#[test]
fn font_metadata_aliases_allocate_only_selected_names_with_exact_utf8_requests() {
    let mut records = cases::standard();
    records[0].4 = cases::utf16("日本語é");
    let mut bytes = cases::name(&records);
    // 256 extra records alias the first payload rather than copying it.
    let original_start = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    let payload = bytes.split_off(original_start);
    for _ in 0..256 {
        let record = bytes[6..18].to_vec();
        bytes.extend(record);
    }
    bytes[2..4].copy_from_slice(&259u16.to_be_bytes());
    let start = bytes.len();
    bytes[4..6].copy_from_slice(&(start as u16).to_be_bytes());
    bytes.extend(payload);
    let mut trace = Trace::default();
    let result = names(&bytes, &mut trace).unwrap();
    assert_eq!(result.family, "日本語é");
    assert_eq!(
        names_tuple(result),
        names_tuple(parse_name(&bytes).unwrap())
    );
    let reserves = trace
        .requests
        .iter()
        .filter(|r| matches!(r, Request::Reserve(..)))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        reserves,
        vec![
            Request::Reserve(Site::FamilyName, 0, 11),
            Request::Reserve(Site::SubfamilyName, 0, 7),
            Request::Reserve(Site::PostscriptName, 0, 14)
        ]
    );
    eprintln!("259 eligible name records share payloads; only three exact UTF-8 string requests");
}
struct Overflow {
    at: usize,
    fail: usize,
    overflow_failures: usize,
}
impl Allocator for Overflow {
    fn vector<T>(&mut self, out: &mut Vec<T>, n: usize) -> Result<(), ()> {
        let at = self.at;
        self.at += 1;
        if at == self.fail {
            assert!(out.try_reserve_exact(usize::MAX).is_err());
            self.overflow_failures += 1;
            Err(())
        } else {
            out.try_reserve_exact(n).map_err(|_| ())
        }
    }
    fn string(&mut self, out: &mut String, n: usize) -> Result<(), ()> {
        let at = self.at;
        self.at += 1;
        if at == self.fail {
            assert!(out.try_reserve_exact(usize::MAX).is_err());
            self.overflow_failures += 1;
            Err(())
        } else {
            out.try_reserve_exact(n).map_err(|_| ())
        }
    }
}
#[test]
fn font_metadata_real_capacity_overflow_returns_native_allocation_after_reservation() {
    for fail in 0..2 {
        let mut alloc = Overflow {
            at: 0,
            fail,
            overflow_failures: 0,
        };
        let mut t = Trace::default();
        assert!(matches!(
            hmtx_alloc(&[0; 10], 4, 1, &mut t, &mut alloc),
            Err(Failure::Allocation)
        ));
        assert_eq!(alloc.overflow_failures, 1);
        assert_eq!(t.paid[..2], [4 * (fail as u64 + 1), 8 * (fail as u64 + 1)]);
    }
    let bytes = cases::name(&cases::standard());
    for fail in 0..3 {
        let mut alloc = Overflow {
            at: 0,
            fail,
            overflow_failures: 0,
        };
        let mut t = Trace::default();
        assert!(matches!(
            names_alloc(&bytes, &mut t, &mut alloc),
            Err(Failure::Allocation)
        ));
        assert_eq!(alloc.overflow_failures, 1);
        assert_eq!(
            t.requests
                .iter()
                .filter(|r| matches!(r, Request::Reserve(..)))
                .count(),
            fail + 1
        );
    }
    assert!(std::mem::size_of::<CffAdmissionBudgetFailureV2<usize>>() < 128);
}
