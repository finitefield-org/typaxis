#[cfg(test)]
mod ir_admission_tests {
    use super::*;
    use crate::{
        SafeSvgSourceBudget, SafeSvgSourceError, SafeSvgSourceRequest, SafeSvgSourceStage as S,
    };
    #[derive(Default)]
    struct Trace {
        requests: Vec<SafeSvgSourceRequest>,
        stop: Option<usize>,
    }
    impl SafeSvgSourceBudget for Trace {
        type Error = usize;
        const OBSERVE_IR: bool = true;
        fn work(&mut self, r: SafeSvgSourceRequest) -> Result<(), usize> {
            let at = self.requests.len();
            if self.stop == Some(at) {
                return Err(at);
            }
            self.requests.push(r);
            Ok(())
        }
    }
    fn caps() -> M4EffectiveResourceLimits {
        M4EffectiveResourceLimits::defaults_for(
            &typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
                .unwrap(),
        )
    }
    fn svg(body: &str) -> Vec<u8> {
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10pt\" height=\"10pt\" viewBox=\"0 0 10 10\">{body}</svg>").into_bytes()
    }
    #[test]
    fn svg_ir_all_phases_preserve_native_and_refuse_each_small_request() {
        let mut cases=vec![include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/fraction-equality.svg")).to_vec()];
        for body in [
            "<rect x=\"-1\" y=\"-2\" width=\"5\" height=\"6\" />",
            "<circle cx=\"4\" cy=\"4\" r=\"2\" />",
            "<ellipse cx=\"4\" cy=\"4\" rx=\"2\" ry=\"1\" />",
            "<polygon points=\"1 1 4 1 4 4\" fill-rule=\"evenodd\" />",
            "<polyline points=\"1 1 4 1 4 4\" fill=\"none\" stroke=\"currentColor\" />",
            "<line x1=\"1\" y1=\"1\" x2=\"4\" y2=\"4\" stroke=\"#112233\" stroke-linecap=\"round\" />",
            "<path d=\"M 1 1 h 2 v 2 q 1 0 1 -1 c 0 -1 -1 -1 -1 0 Z\" fill-opacity=\"0.5\" stroke=\"#123456\" stroke-opacity=\"0.25\" stroke-linejoin=\"bevel\" />",
            "<defs><clipPath id=\"clip\"><path d=\"M 0 0 L 8 0 L 8 8 Z\" /></clipPath></defs><g clip-path=\"url(#clip)\"><rect width=\"4\" height=\"4\" clip-path=\"url(#clip)\" /></g>",
            "<defs><clipPath id=\"unused\"><rect width=\"5\" height=\"5\" /></clipPath></defs><rect width=\"4\" height=\"4\" />",
            "<rect width=\"4\" height=\"4\" clip-path=\"url(#missing)\" />",
            "<path d=\"M1 1L2 2Z\" />",
            "<rect width=\"-1\" height=\"2\" />",
            "<script />",
        ] {cases.push(svg(body));}
        for n in [64, 512] {
            cases.push(svg(&"<path d=\"M 1 1 L 4 1 L 4 4 Z\" />".repeat(n)));
        }
        let mut refusals = 0;
        let mut positives = 0;
        for (at, bytes) in cases.iter().enumerate() {
            let native = decode_v2_with_work_budget(bytes, &caps(), 100_000, 1_000_000);
            let mut full = Trace::default();
            let observed = decode_v2_with_source_budget(
                bytes,
                &caps(),
                100_000,
                1_000_000,
                typaxis_core::ImageResourceId::new(12),
                &mut full,
            );
            match (native, observed) {
                (Ok(a), Ok(z)) => {
                    assert_eq!(a, z);
                    positives += 1;
                    let text = z.ir.canonical_jcs();
                    assert_eq!(
                        z.ir.fingerprint(),
                        sha256(
                            format!(
                                "{{\"algorithm\":\"{}\",\"ir\":{text}}}",
                                SAFE_VECTOR_IR_FINGERPRINT_ID_V2
                            )
                            .as_bytes()
                        )
                    );
                    let measure = full
                        .requests
                        .iter()
                        .filter(|r| r.stage == S::CanonicalMeasure)
                        .collect::<Vec<_>>();
                    assert_eq!(measure.len(), text.len().div_ceil(256));
                    assert!(measure.iter().all(|r| r.work == 4160));
                    let storage = full
                        .requests
                        .iter()
                        .find(|r| r.stage == S::CanonicalStorage)
                        .unwrap();
                    assert_eq!(
                        (storage.records, storage.storage_bytes),
                        (1, text.len() as u64)
                    );
                    let writes = full
                        .requests
                        .iter()
                        .filter(|r| r.stage == S::CanonicalWrite)
                        .map(|r| r.work)
                        .collect::<Vec<_>>();
                    assert_eq!(
                        writes,
                        (0..text.len())
                            .step_by(256)
                            .map(|offset| 16 * (text.len() - offset).min(256) as u64 + 64)
                            .collect::<Vec<_>>()
                    );
                    let expected = [
                        b"{\"algorithm\":\"".as_slice(),
                        SAFE_VECTOR_IR_FINGERPRINT_ID_V2.as_bytes(),
                        b"\",\"ir\":",
                        text.as_bytes(),
                        b"}",
                    ]
                    .into_iter()
                    .flat_map(|part| part.chunks(256).map(|c| 2 * c.len() as u64 + 64))
                    .collect::<Vec<_>>();
                    assert_eq!(
                        full.requests
                            .iter()
                            .filter(|r| r.stage == S::IrHash)
                            .map(|r| r.work)
                            .collect::<Vec<_>>(),
                        expected
                    );
                    assert_eq!(full.requests.last().unwrap().stage, S::IrFinish);
                }
                (Err(a), Err(SafeSvgSourceError::Native(z))) => assert_eq!(a, z),
                (a, z) => panic!("IR mismatch {a:?}/{z:?}"),
            }
            assert!(full.requests.iter().all(|r| r.image_id.get() == 12));
            if at + 2 >= cases.len() {
                continue;
            }
            for stop in 0..full.requests.len() {
                let mut trace = Trace {
                    stop: Some(stop),
                    ..Trace::default()
                };
                let SafeSvgSourceError::Budget(f) = decode_v2_with_source_budget(
                    bytes,
                    &caps(),
                    100_000,
                    1_000_000,
                    typaxis_core::ImageResourceId::new(12),
                    &mut trace,
                )
                .unwrap_err() else {
                    panic!("unpaid IR operation")
                };
                assert_eq!(f.error, stop);
                assert_eq!(f.request, full.requests[stop]);
                assert_eq!(trace.requests, full.requests[..stop]);
                refusals += 1;
            }
        }
        eprintln!(
            "SVG IR cases={} positives={positives} callback refusals={refusals} scale_paths=512",
            cases.len()
        );
    }
    #[test]
    fn svg_ir_decimal_and_measured_bounds_are_exact() {
        for n in [
            i128::MIN,
            i64::MIN as i128,
            -65536,
            -1,
            0,
            1,
            65536,
            u64::MAX as i128,
            i128::MAX,
        ] {
            let mut text = String::new();
            SvgOutput::number(&mut text, n).unwrap();
            assert_eq!(text, n.to_string());
        }
        for n in [0, 1, 255, 256, 257, 512, 513] {
            let value = "x".repeat(n);
            let mut trace = Trace::default();
            let mut measure = Measure {
                budget: &mut trace,
                image: typaxis_core::ImageResourceId::new(1),
                length: 0,
                funded: 0,
            };
            measure.text(&value).unwrap();
            assert_eq!(measure.length, n);
            assert_eq!(trace.requests.len(), n.div_ceil(256));
        }
        let mut text = ReservedText {
            text: String::new(),
            bound: 0,
        };
        assert_eq!(
            text.text("x"),
            Err(ResourceAdmissionError::ReceiptIdentityMismatch)
        );
        assert!(text.text.is_empty());
        let mut trace = Trace::default();
        let mut measure = Measure {
            budget: &mut trace,
            image: typaxis_core::ImageResourceId::new(1),
            length: usize::MAX,
            funded: usize::MAX,
        };
        assert!(matches!(
            measure.text("x"),
            Err(SafeSvgSourceError::Native(
                ResourceAdmissionError::ResourceLimit
            ))
        ));
        assert!(trace.requests.is_empty());
    }
    #[test]
    fn svg_ir_rejects_wrong_private_census_and_capacity_overflow() {
        let bytes=svg("<defs><clipPath id=\"c\"><rect width=\"5\" height=\"5\" /></clipPath></defs><rect width=\"4\" height=\"4\" clip-path=\"url(#c)\" />");
        let source = ValidatedSvgSource(scanner(&bytes).unwrap().source);
        let count = scan_v2(&bytes, ScanMode::Count, None, Some(source)).unwrap();
        for axis in 0..3 {
            for down in [false, true] {
                let mut sizes = count.storage_counts;
                let n = match axis {
                    0 => &mut sizes.definitions,
                    1 => &mut sizes.references,
                    _ => &mut sizes.draws,
                };
                if down {
                    *n -= 1;
                } else {
                    *n += 1;
                }
                for mode in [ScanMode::Analyze, ScanMode::Build] {
                    assert!(matches!(
                        scan_v2_budget(
                            &bytes,
                            mode,
                            None,
                            Some(source),
                            Some(sizes),
                            &mut Trace::default(),
                            typaxis_core::ImageResourceId::new(1)
                        ),
                        Err(SafeSvgSourceError::Native(
                            ResourceAdmissionError::ReceiptIdentityMismatch
                        ))
                    ));
                }
            }
        }
        let mut trace = Trace::default();
        assert!(matches!(
            crate::safe_svg_source::reserve_vec::<u64, _>(
                &mut trace,
                typaxis_core::ImageResourceId::new(1),
                S::BuildPath,
                usize::MAX
            ),
            Err(SafeSvgSourceError::Native(
                ResourceAdmissionError::ResourceLimit
            ))
        ));
        assert!(trace.requests.is_empty());
        let mut data = Vec::<u8>::new();
        assert_eq!(
            crate::safe_svg_source::push_within(&mut data, 1, Some(0)),
            Err(ResourceAdmissionError::ReceiptIdentityMismatch)
        );
        assert_eq!(data.capacity(), 0);
    }
}
