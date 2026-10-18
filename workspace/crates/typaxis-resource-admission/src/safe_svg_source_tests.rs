use super::*;
use crate::safe_vector::{decode_v2_with_source_budget, decode_v2_with_work_budget};
#[derive(Default)]
struct Trace {
    requests: Vec<SafeSvgSourceRequest>,
    stop: Option<usize>,
}
impl SafeSvgSourceBudget for Trace {
    type Error = usize;
    fn work(&mut self, r: SafeSvgSourceRequest) -> Result<(), usize> {
        let at = self.requests.len();
        if self.stop == Some(at) {
            return Err(at);
        }
        self.requests.push(r);
        Ok(())
    }
}
fn limits() -> typaxis_core::M4EffectiveResourceLimits {
    typaxis_core::M4EffectiveResourceLimits::defaults_for(
        &typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .unwrap(),
    )
}
fn decode(
    bytes: &[u8],
    trace: &mut Trace,
) -> Result<crate::safe_vector::DecodedSafeVectorV2, SafeSvgSourceError<usize>> {
    let limits = limits();
    decode_v2_with_source_budget(
        bytes,
        &limits,
        100_000,
        1_000_000,
        ImageResourceId::new(7),
        trace,
    )
}
#[test]
fn svg_source_validation_matches_native_and_refuses_every_preflight_chunk() {
    let original=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/fraction-equality.svg"));
    let mut cases = vec![
        original.to_vec(),
        vec![],
        b"<".to_vec(),
        b"\xef\xbb\xbf<svg/>".to_vec(),
        b"<svg>\xff".to_vec(),
        b"<?xml?>".to_vec(),
        b"<!DOCTYPE svg>".to_vec(),
        b"<svg>&bad;</svg>".to_vec(),
        b"<svg>\r</svg>".to_vec(),
    ];
    for body in [
        "<path d=\"M1 1L2 2Z\" fill=\"currentColor\" />",
        "<path d=\"M 0 0 L 1 1\" fill=\"currentColor\" />",
        "<path d=\"M 0 0 L 2 0 L 2 2 Z\" fill=\"none\" />",
        "<g transform=\"rotate(2)\"></g>",
        "<script />",
        "<rect width=\"-1\" height=\"2\" />",
    ] {
        cases.push(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10pt\" height=\"10pt\" viewBox=\"0 0 10 10\">{body}</svg>").into_bytes());
    }
    for length in [0, 1, 63, 255, 256, 257, 511, 512, 1024, 8192] {
        let mut bytes = vec![b' '; length];
        bytes.extend_from_slice(original);
        cases.push(bytes);
    }
    for at in [0, 1, 255, 256, 511] {
        for value in [0, 13, 127, 128, 159, 255] {
            let mut bytes = vec![b' '; 512];
            bytes.extend_from_slice(original);
            bytes[at] = value;
            cases.push(bytes);
        }
    }
    let mut refusals = 0;
    for bytes in &cases {
        let limits = limits();
        let native = decode_v2_with_work_budget(bytes, &limits, 100_000, 1_000_000);
        let mut full = Trace::default();
        let observed = decode(bytes, &mut full);
        match (native, observed) {
            (Ok(a), Ok(z)) => {
                assert_eq!(a, z);
                assert_eq!(a.ir.fingerprint(), z.ir.fingerprint());
            }
            (Err(a), Err(SafeSvgSourceError::Native(z))) => assert_eq!(a, z),
            (a, z) => panic!("native mismatch: {a:?} / {z:?}"),
        }
        assert_eq!(full.requests.len(), bytes.len().max(1).div_ceil(256));
        assert_eq!(
            full.requests.iter().map(|r| r.byte_length).sum::<u64>(),
            bytes.len() as u64
        );
        for (at, request) in full.requests.iter().enumerate() {
            assert_eq!(request.image_id, ImageResourceId::new(7));
            assert_eq!(request.byte_offset, 256 * at as u64);
            assert!(request.byte_length <= 256);
            assert_eq!(request.work, 16 * request.byte_length + 64);
        }
        for stop in 0..full.requests.len() {
            let mut trace = Trace {
                stop: Some(stop),
                ..Trace::default()
            };
            let SafeSvgSourceError::Budget(f) = decode(bytes, &mut trace).unwrap_err() else {
                panic!("native instead of unpaid preflight")
            };
            assert_eq!(f.error, stop);
            assert_eq!(f.request, full.requests[stop]);
            assert_eq!(trace.requests, full.requests[..stop]);
            refusals += 1;
        }
    }
    eprintln!(
        "SVG source raw cases={} callback refusals={refusals}",
        cases.len()
    );
}
#[test]
fn svg_source_invalid_limits_precede_hooks_and_unobserved_decode_has_no_hooks() {
    struct Native(usize);
    impl SafeSvgSourceBudget for Native {
        type Error = usize;
        const OBSERVED: bool = false;
        fn work(&mut self, _: SafeSvgSourceRequest) -> Result<(), usize> {
            self.0 += 1;
            Ok(())
        }
    }
    let caps = limits();
    let mut native = Native(0);
    assert!(decode_v2_with_source_budget(
        b"bad",
        &caps,
        100_000,
        1_000_000,
        ImageResourceId::new(0),
        &mut native
    )
    .is_err());
    assert_eq!(native.0, 0);
    let mut trace = Trace::default();
    assert!(matches!(
        decode_v2_with_source_budget(
            b"bad",
            &caps,
            caps.extension().get().max_vector_nodes + 1,
            1_000_000,
            ImageResourceId::new(0),
            &mut trace
        ),
        Err(SafeSvgSourceError::Native(
            ResourceAdmissionError::ReceiptIdentityMismatch
        ))
    ));
    assert!(trace.requests.is_empty());
}

#[derive(Default)]
struct CountTrace(Trace);
impl SafeSvgSourceBudget for CountTrace {
    type Error = usize;
    const OBSERVE_COUNT: bool = true;
    fn work(&mut self, r: SafeSvgSourceRequest) -> Result<(), usize> {
        self.0.work(r)
    }
}
#[test]
fn svg_count_pass_matches_native_and_refuses_all_requested_work() {
    let mut cases = Vec::new();
    for paths in [1, 2, 8, 64] {
        let mut body = String::new();
        for _ in 0..paths {
            body.push_str("<path d=\"M 1 1 L 4 1 L 4 4 Z\" fill=\"currentColor\" />");
        }
        cases.push(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10pt\" height=\"10pt\" viewBox=\"0 0 10 10\">{body}</svg>").into_bytes());
    }
    for body in [
        "<rect width=\"2\" height=\"3\" />",
        "<ellipse cx=\"4\" cy=\"4\" rx=\"2\" ry=\"1\" />",
        "<path d=\"M 1 1 Q 2 4 4 1 C 4 2 2 4 1 1 Z\" />",
        "<path d=\"M1 1L4 4Z\" />",
        "<rect width=\"-2\" height=\"3\" />",
        "<script />",
    ] {
        cases.push(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10pt\" height=\"10pt\" viewBox=\"0 0 10 10\">{body}</svg>").into_bytes());
    }
    cases.extend([vec![], b"bad".to_vec(), b"\xff".to_vec()]);
    let caps = limits();
    let mut refusals = 0;
    for bytes in &cases {
        let native = decode_v2_with_work_budget(bytes, &caps, 100_000, 1_000_000);
        let mut full = CountTrace::default();
        let observed = decode_v2_with_source_budget(
            bytes,
            &caps,
            100_000,
            1_000_000,
            ImageResourceId::new(8),
            &mut full,
        );
        match (native, observed) {
            (Ok(a), Ok(z)) => assert_eq!(a, z),
            (Err(a), Err(SafeSvgSourceError::Native(z))) => assert_eq!(a, z),
            (a, z) => panic!("count mismatch: {a:?}/{z:?}"),
        }
        for r in &full.0.requests {
            assert_eq!(r.image_id, ImageResourceId::new(8));
            match r.stage {
                SafeSvgSourceStage::Validation => assert_eq!(r.work, 16 * r.byte_length + 64),
                SafeSvgSourceStage::Count => assert_eq!(r.work, 128 * r.byte_length + 64),
                SafeSvgSourceStage::CountFinish => assert_eq!(
                    (r.byte_offset, r.byte_length, r.work),
                    (bytes.len() as u64, 0, 64)
                ),
                _ => panic!("clip stages require a separate opt-in"),
            }
        }
        for stop in 0..full.0.requests.len() {
            let mut short = CountTrace(Trace {
                stop: Some(stop),
                ..Trace::default()
            });
            let SafeSvgSourceError::Budget(f) = decode_v2_with_source_budget(
                bytes,
                &caps,
                100_000,
                1_000_000,
                ImageResourceId::new(8),
                &mut short,
            )
            .unwrap_err() else {
                panic!("unpaid count operation")
            };
            assert_eq!(f.error, stop);
            assert_eq!(f.request, full.0.requests[stop]);
            assert_eq!(short.0.requests, full.0.requests[..stop]);
            refusals += 1;
        }
    }
    eprintln!(
        "SVG count cases={} callback refusals={refusals}",
        cases.len()
    );
}
#[test]
fn svg_count_native_limit_refusals_do_not_reach_finish() {
    let bytes=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/fraction-equality.svg"));
    let caps = limits();
    for (nodes, paths) in [(1, 1_000_000), (100_000, 4)] {
        let native = decode_v2_with_work_budget(bytes, &caps, nodes, paths).unwrap_err();
        let mut trace = CountTrace::default();
        let SafeSvgSourceError::Native(observed) = decode_v2_with_source_budget(
            bytes,
            &caps,
            nodes,
            paths,
            ImageResourceId::new(8),
            &mut trace,
        )
        .unwrap_err() else {
            panic!("budget error")
        };
        assert_eq!(native, observed);
        assert!(trace
            .0
            .requests
            .iter()
            .any(|r| r.stage == SafeSvgSourceStage::Count));
        assert!(trace
            .0
            .requests
            .iter()
            .all(|r| r.stage != SafeSvgSourceStage::CountFinish));
    }
}
