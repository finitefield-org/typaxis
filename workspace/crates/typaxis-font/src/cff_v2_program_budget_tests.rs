use super::*;
#[path = "cff_v2_program_cases.rs"]
mod cases;
#[derive(Clone, Debug, Eq, PartialEq)]
enum Request {
    Reserve(CffProgramStorageV2, usize, usize),
    Work(u64),
}
impl Request {
    fn delta(&self) -> [u64; 3] {
        match *self {
            Self::Reserve(_, r, b) => [r as u64, b as u64, 0],
            Self::Work(n) => [0, 0, n],
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
    fn request(&mut self, r: Request) -> std::result::Result<(), usize> {
        let at = self.requests.len();
        assert!(self.stop.map_or(true, |stop| at <= stop));
        let delta = r.delta();
        self.requests.push(r);
        if self.stop == Some(at) {
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
    fn reserve(
        &mut self,
        _: CffBaseCmapStorageV2,
        _: usize,
        _: usize,
    ) -> std::result::Result<(), usize> {
        panic!("program cannot debit cmap storage")
    }
    fn work(&mut self, _: u64) -> std::result::Result<(), usize> {
        panic!("program must use its own work loan")
    }
}
impl CffCmapBudgetV2 for Trace {
    fn reserve_variation(
        &mut self,
        _: CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> std::result::Result<(), usize> {
        panic!()
    }
}
impl CffAdmissionBudgetV2 for Trace {
    const OBSERVE_PROGRAM: bool = true;
    fn reserve_sfnt(
        &mut self,
        _: CffSfntStorageV2,
        _: usize,
        _: usize,
    ) -> std::result::Result<(), usize> {
        panic!()
    }
    fn work_sfnt(&mut self, _: u64) -> std::result::Result<(), usize> {
        panic!()
    }
    fn reserve_program(
        &mut self,
        site: CffProgramStorageV2,
        r: usize,
        b: usize,
    ) -> std::result::Result<(), usize> {
        self.request(Request::Reserve(site, r, b))
    }
    fn work_program(&mut self, n: u64) -> std::result::Result<(), usize> {
        assert!(n > 0);
        self.request(Request::Work(n))
    }
}
fn prefix(r: &[Request]) -> [u64; 3] {
    let mut paid = [0; 3];
    for r in r {
        for (v, n) in paid.iter_mut().zip(r.delta()) {
            *v += n;
        }
    }
    paid
}
type GlyphFacts = (Option<u8>, Option<CffSourceGlyphV2>, Vec<u8>);
type FdFacts = (Option<usize>, Option<(i32, i32)>, Vec<u8>);
#[derive(Debug, Eq, PartialEq)]
struct Facts {
    glyphs: Vec<GlyphFacts>,
    fds: Vec<FdFacts>,
    globals: usize,
}
fn facts(p: CffProgramInspectionV2) -> Facts {
    Facts {
        glyphs: (0..p.glyph_count())
            .map(|g| {
                (
                    p.fd_for_gid(g),
                    p.source_glyph(g),
                    p.charstring(g).unwrap().to_vec(),
                )
            })
            .collect(),
        fds: (0..p.font_dict_count())
            .map(|f| {
                (
                    p.local_subroutine_count(f),
                    p.width_defaults(f),
                    p.private_dict_bytes(f).unwrap().to_vec(),
                )
            })
            .collect(),
        globals: p.global_subroutine_count(),
    }
}
fn fixture(shared: bool, real: bool) -> Vec<u8> {
    let mut b = cases::cid_fixture(shared);
    if real {
        let offsets = b
            .windows(5)
            .enumerate()
            .filter_map(|(i, b)| (b == [29, 0, 0, 2, 88]).then_some(i))
            .collect::<Vec<_>>();
        assert!(!offsets.is_empty());
        for at in offsets {
            b[at..at + 5].copy_from_slice(&[30, 0, 0, 0x60, 0x0f]);
        }
    }
    b
}
#[test]
fn program_budget_native_outcomes_and_every_small_callback_prefix_are_exact() {
    let (mut count, mut cutoffs) = (0, 0);
    let mut input = cases::cases();
    for shared in [false, true] {
        input.push((format!("real-{shared}"), fixture(shared, true), 3, 5));
    }
    for (label, b, g, limit) in input {
        let expected = inspect_cff1_program_v2(Arc::from(b.as_slice()), g, 1000, limit).map(facts);
        let mut full = Trace::default();
        let actual =
            inspect_cff1_program_v2_with_budget(Arc::from(b.as_slice()), g, 1000, limit, &mut full)
                .unwrap()
                .map(facts);
        assert_eq!(actual, expected);
        // All requests for the intact multi-FD controls and every prefix. Mutations
        // also compare native fields; their first refusal proves the entry boundary.
        let all = label.contains("prefix")
            || label == "shared-false"
            || label == "shared-true"
            || label.starts_with("real-");
        let stops = if all {
            full.requests.len()
        } else {
            full.requests.len().min(1)
        };
        for stop in 0..stops {
            let mut short = Trace {
                stop: Some(stop),
                ..Trace::default()
            };
            let f = inspect_cff1_program_v2_with_budget(
                Arc::from(b.as_slice()),
                g,
                1000,
                limit,
                &mut short,
            )
            .unwrap_err();
            assert_eq!(f.error, 91);
            assert_eq!(short.requests, full.requests[..=stop]);
            assert_eq!(short.paid, prefix(&full.requests[..stop]));
            assert_eq!(f.point.file_offset, None);
            if let Some(at) = f.point.table_offset {
                assert!(at <= b.len() as u64);
            }
            cutoffs += 1;
        }
        count += 1;
    }
    eprintln!("program native cases={count}, sampled/all-small callback cuts={cutoffs}");
}
#[test]
fn program_budget_every_successful_multi_fd_allocation_is_fallible_without_refunds() {
    let (mut faults, mut allocations) = (0, 0);
    for shared in [false, true] {
        for real in [false, true] {
            let b = fixture(shared, real);
            let mut full = Trace::default();
            let mut s = State::new(&mut full);
            let expected = facts(
                super::super::program_kernel(
                    SourceBytesV2::Slice(Arc::from(b.as_slice())),
                    None,
                    3,
                    1000,
                    5,
                    None,
                    &mut s,
                )
                .unwrap(),
            );
            let total = s.allocation_count;
            allocations += total;
            for fail_at in 0..total {
                let mut trace = Trace::default();
                let mut state = State::new(&mut trace);
                state.allocation_failure = Some(fail_at);
                let result = super::super::program_kernel(
                    SourceBytesV2::Slice(Arc::from(b.as_slice())),
                    None,
                    3,
                    1000,
                    5,
                    None,
                    &mut state,
                );
                let Failure::Native(error) = result.unwrap_err() else {
                    panic!("expected actual capacity overflow")
                };
                assert_eq!(error.kind, K::AllocationFailure);
                assert_eq!(state.allocation_count, fail_at + 1);
                assert_eq!(trace.paid, prefix(&trace.requests));
                assert!(matches!(trace.requests.last(), Some(Request::Reserve(..))));
                faults += 1;
            }
            let actual = inspect_cff1_program_v2_with_budget(
                Arc::from(b.as_slice()),
                3,
                1000,
                5,
                &mut Trace::default(),
            )
            .unwrap()
            .unwrap();
            assert_eq!(facts(actual), expected);
        }
    }
    assert!(std::mem::size_of::<CffAdmissionBudgetFailureV2<usize>>() < 128);
    eprintln!("program real capacity-overflow failures={faults}, successful allocation sites={allocations}");
}
#[test]
fn program_budget_span_heapsort_matches_native_order_and_refuses_every_comparison_and_swap() {
    let (mut count, mut cuts) = (0, 0);
    for n in [0, 1, 2, 3, 7, 16, 31, 64] {
        for pattern in 0..4 {
            let input = (0..n)
                .map(|i| {
                    let k = match pattern {
                        0 => i,
                        1 => n - i,
                        2 => i % 3,
                        _ => (i * 73 + 11) % 17,
                    };
                    (
                        ProgramSpan {
                            start: k,
                            end: k + 1,
                        },
                        (i % 3) as u8,
                    )
                })
                .collect::<Vec<_>>();
            let mut expected = input.clone();
            expected.sort_unstable();
            let mut t = Trace::default();
            let mut state = State::new(&mut t);
            let mut actual = input.clone();
            sort_spans(&mut actual, &mut state).unwrap();
            assert_eq!(actual, expected);
            for stop in 0..t.requests.len() {
                let mut short = Trace {
                    stop: Some(stop),
                    ..Trace::default()
                };
                let mut state = State::new(&mut short);
                let Failure::Budget(f) = sort_spans(&mut input.clone(), &mut state).unwrap_err()
                else {
                    panic!()
                };
                assert_eq!(f.point.table_offset, None);
                assert_eq!(f.point.fd, None);
                assert_eq!(short.requests, t.requests[..=stop]);
                assert_eq!(short.paid, prefix(&t.requests[..stop]));
                cuts += 1;
            }
            count += 1;
        }
    }
    eprintln!("program independent span orders={count}, comparison/swap refusal cuts={cuts}");
}

#[test]
#[ignore = "requires unchanged original Harano through TYPAXIS_HARFRUST_CACHE_FONTS"]
fn program_admission_keeps_original_source_allocation_and_exact_cff_views() {
    let paths =
        std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let bytes = std::fs::read(paths.lines().next().unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let source: Arc<[u8]> = bytes.into();
    let limits = typaxis_core::M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .unwrap(),
        typaxis_core::M4ResourceLimits::default(),
    )
    .unwrap();
    let admitted = admit_sfnt_cff1_v2(source.clone(), 0, &limits).unwrap();
    let SourceBytesV2::Slice(shared) = &admitted.program().program.source else {
        panic!("slice owner changed")
    };
    assert!(Arc::ptr_eq(&source, shared));
    let span = admitted.program().program.source_span.unwrap();
    assert_eq!(
        admitted.program().program.bytes(),
        &source[span.start..span.end]
    );
    let standalone = inspect_cff1_program_v2(
        Arc::from(admitted.program().program.bytes()),
        admitted.glyph_count(),
        admitted.units_per_em(),
        limits.extension().get().max_cff_subroutines,
    )
    .unwrap();
    for gid in 0..admitted.program().glyph_count() {
        assert_eq!(
            admitted.program().charstring(gid),
            standalone.charstring(gid)
        );
        assert_eq!(
            admitted.program().fd_for_gid(gid),
            standalone.fd_for_gid(gid)
        );
    }
    let mut subroutines = 0;
    for (actual, expected) in admitted
        .program()
        .program
        .global_subrs
        .iter()
        .zip(&standalone.program.global_subrs)
    {
        assert_eq!(
            actual.bytes(admitted.program().program.bytes()),
            expected.bytes(standalone.program.bytes())
        );
        subroutines += 1;
    }
    for (actual, expected) in admitted
        .program()
        .program
        .font_dicts
        .iter()
        .zip(&standalone.program.font_dicts)
    {
        for (a, e) in actual.local_subrs.iter().zip(&expected.local_subrs) {
            assert_eq!(
                a.bytes(admitted.program().program.bytes()),
                e.bytes(standalone.program.bytes())
            );
            subroutines += 1;
        }
    }
    let mut fds = std::collections::BTreeSet::new();
    let mut sliced = CffProgramEvaluationSessionV2::new(&limits);
    let mut direct = CffProgramEvaluationSessionV2::new(&limits);
    for gid in 0..admitted.program().glyph_count() {
        let fd = admitted.program().fd_for_gid(gid).unwrap();
        if !fds.insert(fd) {
            continue;
        }
        let advance = admitted.horizontal_metric(gid as u16).unwrap().0;
        let a = sliced
            .evaluate(admitted.program(), gid as u16, advance)
            .unwrap();
        let b = direct.evaluate(&standalone, gid as u16, advance).unwrap();
        assert_eq!(a.source_width_fixed(), b.source_width_fixed());
        assert_eq!(
            a.canonical_charstring().unwrap(),
            b.canonical_charstring().unwrap()
        );
    }
    // This unchanged font declares 18 FDs; its glyph map selects 12 of them.
    assert_eq!(fds.len(), 12);
    assert_eq!(admitted.program().font_dict_count(), 18);
    eprintln!(
        "borrowed source subroutine views={subroutines}, representative FD evaluations={}",
        fds.len()
    );
    eprintln!(
        "original program source Arc shared, CFF range={span:?}, glyph views={}",
        admitted.program().glyph_count()
    );
}

#[test]
fn program_budget_name_keyed_all_native_allocations_fail_without_refunds() {
    use read_fonts::{types::Tag, FontRef, TableProvider};
    let source=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/vmb-book/fixture-fonts/typaxis-cff-fixture.otf"));
    let font = FontRef::new(source).unwrap();
    let b = font.data_for_tag(Tag::new(b"CFF ")).unwrap();
    let b = b.as_bytes();
    let mut full = Trace::default();
    let mut state = State::new(&mut full);
    let expected = facts(
        super::super::program_kernel(
            SourceBytesV2::Slice(Arc::from(b)),
            None,
            4,
            1000,
            20,
            None,
            &mut state,
        )
        .unwrap(),
    );
    let total = state.allocation_count;
    for at in 0..total {
        let mut trace = Trace::default();
        let mut state = State::new(&mut trace);
        state.allocation_failure = Some(at);
        let error = super::super::program_kernel(
            SourceBytesV2::Slice(Arc::from(b)),
            None,
            4,
            1000,
            20,
            None,
            &mut state,
        )
        .unwrap_err();
        let Failure::Native(error) = error else {
            panic!("wrong failure")
        };
        assert_eq!(error.kind, K::AllocationFailure);
        assert_eq!(state.allocation_count, at + 1);
        assert_eq!(trace.requests, full.requests[..trace.requests.len()]);
        assert_eq!(trace.paid, prefix(&trace.requests));
    }
    let actual = inspect_cff1_program_v2(Arc::from(b), 4, 1000, 20).unwrap();
    assert_eq!(facts(actual), expected);
    eprintln!("name-keyed native allocation failures={total}");
}
