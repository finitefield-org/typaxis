use super::*;
use crate::SafeSvgSourceRequest;
#[derive(Default)]
struct Trace {
    requests: Vec<SafeSvgSourceRequest>,
    stop: Option<usize>,
}
impl SafeSvgSourceBudget for Trace {
    type Error = usize;
    const OBSERVE_COUNT: bool = true;
    const OBSERVE_CLIP_CLOSURE: bool = true;
    fn work(&mut self, request: SafeSvgSourceRequest) -> Result<(), usize> {
        let at = self.requests.len();
        if self.stop == Some(at) {
            return Err(at);
        }
        self.requests.push(request);
        Ok(())
    }
}
const IMAGE: ImageResourceId = ImageResourceId::new(11);
fn run(keys: &[String], trace: &mut Trace) -> Result<(), SafeSvgSourceError<usize>> {
    let mut index = Index::new(
        keys.iter()
            .enumerate()
            .map(|(i, k)| (k.as_str(), i as u64 + 5)),
        trace,
        IMAGE,
    )?;
    for (i, key) in keys.iter().enumerate().rev() {
        assert_eq!(index.lookup(key, trace, IMAGE)?, Some(i as u64 + 5));
    }
    assert_eq!(index.lookup("missing", trace, IMAGE)?, None);
    index.finish(trace, IMAGE)
}
#[test]
fn svg_clip_index_orders_borrowed_ids_and_refuses_every_small_operation() {
    let mut refusals = 0;
    for count in [0, 1, 2, 17, 512] {
        for order in 0..3 {
            let mut keys = (0..count)
                .map(|i| format!("shared_prefix_{}_{i:05}", "x".repeat(32)))
                .collect::<Vec<_>>();
            if order == 1 {
                keys.reverse();
            }
            if order == 2 && count > 0 {
                keys.rotate_left(count / 3);
            }
            let mut full = Trace::default();
            run(&keys, &mut full).unwrap();
            let reserve = full.requests[0];
            assert_eq!(
                (reserve.stage, reserve.records, reserve.storage_bytes),
                (
                    Stage::ClipStorage,
                    count as u64,
                    (count * std::mem::size_of::<Row<'_>>()) as u64
                )
            );
            assert!(full
                .requests
                .iter()
                .all(|r| r.image_id == IMAGE && r.byte_length == 0 && r.byte_offset == 0));
            if count <= 17 {
                for stop in 0..full.requests.len() {
                    let mut trace = Trace {
                        stop: Some(stop),
                        ..Trace::default()
                    };
                    let SafeSvgSourceError::Budget(f) = run(&keys, &mut trace).unwrap_err() else {
                        panic!("unpaid clip operation")
                    };
                    assert_eq!(f.error, stop);
                    assert_eq!(f.request, full.requests[stop]);
                    assert_eq!(trace.requests, full.requests[..stop]);
                    refusals += 1;
                }
            }
        }
    }
    eprintln!(
        "SVG clip index scale=512 orders=3 callback refusals={refusals} row_bytes={}",
        std::mem::size_of::<Row<'_>>()
    );
}
#[test]
fn svg_clip_index_preserves_duplicate_unused_and_missing_failures() {
    let mut trace = Trace::default();
    assert!(matches!(
        Index::new([("same", 5), ("same", 6)].into_iter(), &mut trace, IMAGE)
            .err()
            .unwrap(),
        SafeSvgSourceError::Native(_)
    ));
    let mut index = Index::new([("a", 5), ("b", 6)].into_iter(), &mut trace, IMAGE).unwrap();
    assert_eq!(index.lookup("absent", &mut trace, IMAGE).unwrap(), None);
    assert_eq!(index.lookup("a", &mut trace, IMAGE).unwrap(), Some(5));
    assert!(matches!(
        index.finish(&mut trace, IMAGE),
        Err(SafeSvgSourceError::Native(_))
    ));
}
struct Liar {
    left: usize,
    advertised: usize,
}
impl Iterator for Liar {
    type Item = (&'static str, u64);
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            None
        } else {
            self.left -= 1;
            Some(("x", 5))
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.advertised, Some(self.advertised))
    }
}
impl ExactSizeIterator for Liar {
    fn len(&self) -> usize {
        self.advertised
    }
}
#[test]
fn svg_clip_index_rejects_false_exact_counts_before_growth() {
    for (left, advertised) in [(1, 0), (2, 1), (0, 1), (1, 2)] {
        assert!(matches!(
            Index::new(Liar { left, advertised }, &mut Trace::default(), IMAGE)
                .err()
                .unwrap(),
            SafeSvgSourceError::Native(ResourceAdmissionError::ReceiptIdentityMismatch)
        ));
    }
    let mut trace = Trace::default();
    assert!(matches!(
        Index::new(
            Liar {
                left: 0,
                advertised: usize::MAX
            },
            &mut trace,
            IMAGE
        )
        .err()
        .unwrap(),
        SafeSvgSourceError::Native(ResourceAdmissionError::ResourceLimit)
    ));
    assert!(trace.requests.is_empty());
}
fn svg(body: &str) -> Vec<u8> {
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10pt\" height=\"10pt\" viewBox=\"0 0 10 10\">{body}</svg>").into_bytes()
}
#[test]
fn svg_clip_closure_matches_native_ir_errors_and_all_callback_refusals() {
    use crate::safe_vector::{decode_v2_with_source_budget, decode_v2_with_work_budget};
    let caps = typaxis_core::M4EffectiveResourceLimits::defaults_for(
        &typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .unwrap(),
    );
    let mut cases = Vec::new();
    for count in [0, 1, 2, 17, 64] {
        let mut body = String::new();
        if count > 0 {
            body.push_str("<defs>");
            for i in (0..count).rev() {
                body.push_str(&format!(
                    "<clipPath id=\"shared_{i}\"><rect width=\"5\" height=\"5\" /></clipPath>"
                ));
            }
            body.push_str("</defs>");
        }
        if count == 0 {
            body.push_str("<rect width=\"4\" height=\"4\" />");
        }
        for i in 0..count {
            body.push_str(&format!("<g clip-path=\"url(#shared_{i})\"><path d=\"M 1 1 L 4 1 L 4 4 Z\" clip-path=\"url(#shared_{i})\" /></g>"));
        }
        cases.push(svg(&body));
    }
    for body in [
        "<defs><clipPath id=\"c\"><rect width=\"5\" height=\"5\" /></clipPath><clipPath id=\"c\"><rect width=\"5\" height=\"5\" /></clipPath></defs><rect width=\"4\" height=\"4\" clip-path=\"url(#c)\" />",
        "<defs><clipPath id=\"unused\"><rect width=\"5\" height=\"5\" /></clipPath></defs><rect width=\"4\" height=\"4\" />",
        "<rect width=\"4\" height=\"4\" clip-path=\"url(#missing)\" />",
        "<defs><clipPath id=\"c\"><rect width=\"5\" height=\"5\" /></clipPath></defs><rect width=\"4\" height=\"4\" clip-path=\"url(#c)\" transform=\"translate(1 1)\" />",
    ]{cases.push(svg(body));}
    let mut refusals = 0;
    for (at, bytes) in cases.iter().enumerate() {
        let native = decode_v2_with_work_budget(bytes, &caps, 100_000, 1_000_000);
        let mut full = Trace::default();
        let observed =
            decode_v2_with_source_budget(bytes, &caps, 100_000, 1_000_000, IMAGE, &mut full);
        match (native, observed) {
            (Ok(a), Ok(z)) => assert_eq!(a, z),
            (Err(a), Err(SafeSvgSourceError::Native(z))) => assert_eq!(a, z),
            (a, z) => panic!("native clip mismatch {a:?}/{z:?}"),
        }
        if at == 4 {
            continue;
        }
        for stop in 0..full.requests.len() {
            let mut trace = Trace {
                stop: Some(stop),
                ..Trace::default()
            };
            let SafeSvgSourceError::Budget(f) =
                decode_v2_with_source_budget(bytes, &caps, 100_000, 1_000_000, IMAGE, &mut trace)
                    .unwrap_err()
            else {
                panic!("unpaid native clip closure")
            };
            assert_eq!(f.error, stop);
            assert_eq!(f.request, full.requests[stop]);
            assert_eq!(trace.requests, full.requests[..stop]);
            refusals += 1;
        }
    }
    let bytes=svg("<defs><clipPath id=\"c\"><rect width=\"5\" height=\"5\" /></clipPath></defs><rect width=\"4\" height=\"4\" clip-path=\"url(#c)\" />");
    for cap in [14, 19, 20] {
        let native = decode_v2_with_work_budget(&bytes, &caps, 100_000, cap);
        let observed =
            decode_v2_with_source_budget(&bytes, &caps, 100_000, cap, IMAGE, &mut Trace::default());
        match (native, observed) {
            (Ok(a), Ok(z)) => assert_eq!(a, z),
            (Err(a), Err(SafeSvgSourceError::Native(z))) => assert_eq!(a, z),
            (a, z) => panic!("native replay mismatch {a:?}/{z:?}"),
        }
    }
    eprintln!(
        "SVG clip native cases={} callback refusals={refusals} replay_caps=3",
        cases.len()
    );
}
