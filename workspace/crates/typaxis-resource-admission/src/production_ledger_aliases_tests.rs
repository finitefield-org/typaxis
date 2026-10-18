use super::*;

#[derive(Default)]
struct Trace {
    requests: Vec<(ProductionResourceLedgerPointV3, u64)>,
    stop: Option<usize>,
}
impl ProductionResourceLedgerBudgetV3 for Trace {
    type Error = usize;
    fn reserve(
        &mut self,
        _: ProductionResourceLedgerStorageV3,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        unreachable!()
    }
    fn work(&mut self, p: ProductionResourceLedgerPointV3, n: u64) -> Result<(), usize> {
        let at = self.requests.len();
        if self.stop == Some(at) {
            return Err(at);
        }
        self.requests.push((p, n));
        Ok(())
    }
}
fn result(v: Result<(), ProductionResourceLedgerErrorV3<usize>>) -> bool {
    match v {
        Ok(()) => true,
        Err(ProductionResourceLedgerErrorV3::Native(ProductionResourceErrorV3::Resource(
            ResourceAdmissionError::InvalidSafeVectorV2(SafeVectorFailureReason::ResourceConflict),
        ))) => false,
        Err(error) => panic!("unexpected {error:?}"),
    }
}

#[test]
fn observed_alias_index_matches_native_at_book_scale_and_refuses_before_each_small_operation() {
    let payloads = (0u8..31)
        .map(|i| vec![i; 513 + usize::from(i)])
        .collect::<Vec<_>>();
    let mut refusals = 0;
    for count in [0, 1, 2, 7, 33, 1024, 8192] {
        let input = (0..count)
            .rev()
            .map(|i| {
                let bytes = payloads[(i * 13) % payloads.len()].as_slice();
                (sha256(bytes), bytes, ImageResourceId::new(i as u32))
            })
            .collect::<Vec<_>>();
        let native = input.iter().map(|v| (v.0, v.1)).collect::<Vec<_>>();
        assert!(validate_safe_vector_digest_aliases(&native).is_ok());
        let mut aliases = input.clone();
        let pointer = aliases.as_ptr();
        let mut trace = Trace::default();
        assert!(result(validate(&mut aliases, &mut trace)));
        assert_eq!(pointer, aliases.as_ptr());
        let mut sorted = input.clone();
        sorted.sort_unstable_by_key(|a| (a.0, a.2));
        assert_eq!(aliases, sorted);
        for stop in 0..trace.requests.len() {
            if count > 7 && stop != 0 && stop + 1 != trace.requests.len() {
                continue;
            }
            let mut short = Trace {
                stop: Some(stop),
                ..Trace::default()
            };
            let ProductionResourceLedgerErrorV3::Budget(error) =
                validate(&mut input.clone(), &mut short).unwrap_err()
            else {
                panic!("native error")
            };
            assert_eq!(error.error, stop);
            assert_eq!(error.point, trace.requests[stop].0);
            assert_eq!(short.requests, trace.requests[..stop]);
            short.stop = None;
            assert!(result(validate(&mut input.clone(), &mut short)));
            assert_eq!(&short.requests[stop..], trace.requests);
            refusals += 1;
        }
        eprintln!(
            "vector aliases count={count} requests={} work={}",
            trace.requests.len(),
            trace.requests.iter().map(|r| r.1).sum::<u64>()
        );
    }
    eprintln!("vector alias callback refusals={refusals}");
}

#[test]
fn observed_alias_conflicts_preserve_native_reason_and_bound_each_payload_comparison() {
    // Explicit collision controls at the private validator seam, never forged
    // host receipts or admitted resources. Positive records use real SHA-256.
    let original = vec![71; 1025];
    for mutation in [None, Some(0), Some(255), Some(256), Some(512), Some(1024)] {
        let mut changed = original.clone();
        if let Some(index) = mutation {
            changed[index] ^= 1;
        } else {
            changed.pop();
        }
        let mut aliases = [
            (
                sha256(&original),
                original.as_slice(),
                ImageResourceId::new(0),
            ),
            (
                sha256(&original),
                changed.as_slice(),
                ImageResourceId::new(1),
            ),
        ];
        assert!(matches!(
            validate_safe_vector_digest_aliases(
                &aliases.iter().map(|a| (a.0, a.1)).collect::<Vec<_>>()
            ),
            Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::ResourceConflict
            ))
        ));
        let mut trace = Trace::default();
        assert!(!result(validate(&mut aliases, &mut trace)));
        let chunks = trace
            .requests
            .iter()
            .filter(|(p, n)| p.stage == Stage::AliasCompare && *n > 64)
            .collect::<Vec<_>>();
        assert_eq!(chunks.len(), mutation.map_or(0, |i| i / 256 + 1));
        assert!(chunks
            .iter()
            .all(|(p, n)| p.image_id == Some(ImageResourceId::new(1)) && *n <= 576));
    }
}
