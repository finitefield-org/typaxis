//! Allocation counter for generic host tokens used by the real Book command.
//! It also runs against a frozen parent compiler with the identical public API.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
use typaxis_core::*;
use typaxis_host_admission::*;
static TRACK: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
struct Meter;
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if TRACK.load(Ordering::SeqCst) {
            COUNT.fetch_add(1, Ordering::SeqCst);
            BYTES.fetch_add(l.size() as u64, Ordering::SeqCst);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if TRACK.load(Ordering::SeqCst) {
            COUNT.fetch_add(1, Ordering::SeqCst);
            BYTES.fetch_add(l.size() as u64, Ordering::SeqCst);
        }
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if TRACK.load(Ordering::SeqCst) {
            COUNT.fetch_add(1, Ordering::SeqCst);
            BYTES.fetch_add(n as u64, Ordering::SeqCst);
        }
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOCATOR: Meter = Meter;
fn measured<T>(f: impl FnOnce() -> T) -> (T, [u64; 2]) {
    COUNT.store(0, Ordering::SeqCst);
    BYTES.store(0, Ordering::SeqCst);
    TRACK.store(true, Ordering::SeqCst);
    let value = f();
    TRACK.store(false, Ordering::SeqCst);
    (
        value,
        [COUNT.load(Ordering::SeqCst), BYTES.load(Ordering::SeqCst)],
    )
}
struct Native;
impl HostReadBudget for Native {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn charge(&mut self, _: HostReadRequest) -> Result<(), Self::Error> {
        unreachable!()
    }
}
#[cfg(token_current)]
fn observed_index(host: &HostAdmissionSession, root: &std::path::Path) -> serde_json::Value {
    struct Budget {
        storage_calls: usize,
        storage_bytes: u64,
        lookup_calls: usize,
    }
    impl HostReadBudget for Budget {
        type Error = std::convert::Infallible;
        const OBSERVE_OPEN: bool = true;
        const OBSERVE_OPEN_IDENTITIES: bool = true;
        fn charge(&mut self, r: HostReadRequest) -> Result<(), Self::Error> {
            if r.stage == HostReadStage::OpenIdentityStorage {
                self.storage_calls += 1;
                self.storage_bytes += r.buffer_bytes;
            }
            if r.stage == HostReadStage::OpenIdentityLookup {
                self.lookup_calls += 1;
                COUNT.store(0, Ordering::SeqCst);
                BYTES.store(0, Ordering::SeqCst);
                TRACK.store(true, Ordering::SeqCst);
            }
            Ok(())
        }
    }
    let names = (0..512)
        .map(|i| PortablePath::new(format!("indexed-{i:04}.bin")).unwrap())
        .collect::<Vec<_>>();
    for name in &names {
        std::fs::write(root.join(name.as_str()), b"i").unwrap();
    }
    let registered = host.roots().register_candidates(&names).unwrap();
    let mut budget = Budget {
        storage_calls: 0,
        storage_bytes: 0,
        lookup_calls: 0,
    };
    for candidate in &registered {
        let opened = host
            .roots()
            .open_registered_with_budget(candidate, &mut budget)
            .unwrap();
        TRACK.store(false, Ordering::SeqCst);
        assert_eq!(
            [COUNT.load(Ordering::SeqCst), BYTES.load(Ordering::SeqCst)],
            [1, 16]
        );
        drop(opened);
    }
    assert_eq!(budget.storage_calls, 1);
    assert_eq!(budget.lookup_calls, 512);
    assert_eq!(
        host.read_ledger()
            .token()
            .unwrap()
            .stored_opened_identity_count(),
        514
    );
    serde_json::json!({"distinct_opens":512,"node_bytes":HOST_OPEN_IDENTITY_NODE_BYTES,"storage_calls":budget.storage_calls,"storage_bytes":budget.storage_bytes,"lookup_calls":budget.lookup_calls,"allocations_after_each_lookup_permission":1,"bytes_after_each_lookup_permission":16,"read_identity_owner_bytes":16,"index_allocations_after_lookup":0,"opened_identities":514})
}
fn main() {
    let root = PathBuf::from(std::env::args_os().nth(1).unwrap());
    let expected = std::env::args().nth(2).unwrap();
    assert!(expected == "parent" || expected == "current");
    assert!(!root.exists());
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("a.bin"), b"a").unwrap();
    std::fs::write(root.join("b.bin"), b"b").unwrap();
    std::fs::hard_link(root.join("a.bin"), root.join("alias.bin")).unwrap();
    let config = EffectiveConfig::new(
        false,
        PdfStreamCompression::Flate,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|s| s.to_string())
            .collect(),
        EffectiveDataVersions::new(
            REGISTERED_UNICODE_VERSION,
            REGISTERED_JAPANESE_LINE_BREAK_VERSION,
        )
        .unwrap(),
        ResourceLimits::default(),
    )
    .unwrap();
    let context = HostAdmissionContext::new(
        HostPath::new(root.join("input.typ")).unwrap(),
        HostPath::new(root.clone()).unwrap(),
        None,
        vec![],
    );
    let host = HostAdmissionSession::new(&context, &config).unwrap();
    let mut names = ["a.bin", "alias.bin", "b.bin"]
        .into_iter()
        .map(|s| PortablePath::new(s).unwrap())
        .collect::<Vec<_>>();
    names
        .extend((3..5000).map(|i| PortablePath::new(format!("missing/entry-{i:04}.bin")).unwrap()));
    let registered = host.roots().register_candidates(&names).unwrap();
    let stale = host.read_ledger().token().unwrap();
    for row in registered.iter().take(3) {
        let _ = host
            .roots()
            .open_registered_with_budget(row, &mut Native)
            .unwrap();
    }
    let (token, issued) = measured(|| host.read_ledger().token().unwrap());
    assert_eq!(token.candidate_attempt_count(), 5000);
    assert_eq!(token.stored_candidate_identity_count(), 5000);
    assert_eq!(token.stored_opened_identity_count(), 2);
    let (negative, rejected) = measured(|| stale.revalidate());
    assert_eq!(negative, Err(HostAdmissionError::ReadIdentityMismatch));
    let target = HostPath::new(root.join("out.pdf")).unwrap();
    let (negative, write_rejected) = measured(|| stale.revalidate_write_target(&target));
    assert_eq!(negative, Err(HostAdmissionError::ReadIdentityMismatch));
    let repeats = if expected == "current" { 256 } else { 1 };
    let (_, repeated) = measured(|| {
        for _ in 0..repeats {
            std::hint::black_box(host.read_ledger().token().unwrap());
        }
    });
    if expected == "current" {
        assert_eq!(issued, [0; 2]);
        assert_eq!(rejected, [0; 2]);
        assert_eq!(write_rejected, [0; 2]);
        assert_eq!(repeated, [0; 2]);
    } else {
        assert!(issued[0] > 5000 && issued[1] > 500000);
        assert!(rejected[0] > 5000 && write_rejected[0] > 5000);
    }
    #[cfg(token_current)]
    let indexed = observed_index(&host, &root);
    #[cfg(not(token_current))]
    let indexed = serde_json::Value::Null;
    println!(
        "{}",
        serde_json::json!({"passed":true,"epoch":expected,"candidate_attempts":5000,"candidate_identities":5000,"opened_identities":2,"token_allocation_count_bytes":issued,"stale_revalidate_allocation_count_bytes":rejected,"stale_write_guard_allocation_count_bytes":write_rejected,"repeat_count":repeats,"repeated_token_allocation_count_bytes":repeated,"stale_typed_error_preserved":true,"hard_link_dedup_preserved":true,"observed_index":indexed})
    );
}
