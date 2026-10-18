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
fn main() {
    let root = PathBuf::from(std::env::args_os().nth(1).unwrap());
    let label = std::env::args().nth(2).unwrap();
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
    for row in registered.iter().take(3) {
        let _ = host
            .roots()
            .open_registered_with_budget(row, &mut Native)
            .unwrap();
    }
    let (token, issued) = measured(|| host.read_ledger().token().unwrap());
    assert_eq!(token.stored_opened_identity_count(), 2);
    assert_eq!(issued, [0, 0]);
    let (ok, valid) = measured(|| token.revalidate());
    ok.unwrap();
    let absent = HostPath::new(root.join("out.pdf")).unwrap();
    let (alias, write) = measured(|| token.revalidate_write_target(&absent));
    assert!(!alias.unwrap());
    let existing = HostPath::new(root.join("alias.bin")).unwrap();
    let (alias, conflict) = measured(|| token.revalidate_write_target(&existing));
    assert!(alias.unwrap());
    host.roots()
        .register_candidates(std::iter::once(&PortablePath::new("new.bin").unwrap()))
        .unwrap();
    let (error, stale) = measured(|| token.revalidate());
    assert_eq!(error, Err(HostAdmissionError::ReadIdentityMismatch));
    assert_eq!(stale, [0, 0]);
    println!(
        "{}",
        serde_json::json!({"passed":true,"epoch":label,"candidate_attempts":5000,"opened_identities":2,"valid_revalidate_allocation_count_bytes":valid,"valid_write_guard_allocation_count_bytes":write,"valid_conflict_guard_allocation_count_bytes":conflict,"token_allocation_count_bytes":issued,"stale_allocation_count_bytes":stale,"conflict_result":true,"non_conflict_result":false})
    );
}
