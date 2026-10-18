//! Fallible receipt hashing with the shared Source work loan; no owned bytes.
use super::*;

pub(super) struct Digest<'call, S> {
    state: Sha256,
    storage: &'call mut S,
    owner: NodeId,
}
impl<'call, S: Storage> Digest<'call, S> {
    pub(super) fn new(
        owner: NodeId,
        storage: &'call mut S,
    ) -> Result<Self, ProductionTextShapeError> {
        if S::OBSERVED {
            storage.owner(owner);
            storage
                .work(1)
                .map_err(|e| error(owner, ProductionTextShapeErrorKind::Backend(e)))?;
        }
        Ok(Self {
            state: Sha256::new(),
            storage,
            owner,
        })
    }
    pub(super) fn owner(&mut self, owner: NodeId) {
        self.owner = owner;
        if S::OBSERVED {
            self.storage.owner(owner);
        }
    }
    fn pay(&mut self, amount: u64) -> Result<(), ProductionTextShapeError> {
        if S::OBSERVED {
            self.storage
                .work(amount)
                .map_err(|e| error(self.owner, ProductionTextShapeErrorKind::Backend(e)))?;
        }
        Ok(())
    }
    fn input(&mut self, bytes: &[u8]) -> Result<(), ProductionTextShapeError> {
        let amount = u64::try_from(bytes.len())
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| error(self.owner, ProductionTextShapeErrorKind::ArithmeticOverflow))?;
        self.pay(amount)
    }
    pub(super) fn update(&mut self, bytes: &[u8]) -> Result<(), ProductionTextShapeError> {
        self.input(bytes)?;
        self.state.update(bytes);
        Ok(())
    }
    pub(super) fn hash_into(&mut self, bytes: &[u8]) -> Result<(), ProductionTextShapeError> {
        self.pay(1)?;
        let mut nested = Sha256::new();
        self.input(bytes)?;
        nested.update(bytes);
        // SHA-256 finalization uses at most two fixed 64-byte blocks.
        self.pay(128)?;
        let hash = nested.finish();
        self.update(&hash)
    }
    pub(super) fn finish(mut self) -> Result<[u8; 32], ProductionTextShapeError> {
        self.pay(128)?;
        Ok(self.state.finish())
    }
}

pub(super) fn hash<S: Storage>(
    owner: NodeId,
    bytes: &[u8],
    storage: &mut S,
) -> Result<[u8; 32], ProductionTextShapeError> {
    let mut digest = Digest::new(owner, storage)?;
    digest.update(bytes)?;
    digest.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Trace {
        owner: Option<NodeId>,
        requests: Vec<(NodeId, u64)>,
        paid: u64,
        stop: Option<usize>,
    }
    impl Storage for Trace {
        fn owner(&mut self, owner: NodeId) {
            self.owner = Some(owner);
        }
        fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
            panic!("receipt hashing must not reserve storage")
        }
        fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
            panic!("receipt hashing must not retain records")
        }
        fn work(&mut self, amount: u64) -> Result<(), LinkedShaperError> {
            let at = self.requests.len();
            self.requests.push((self.owner.unwrap(), amount));
            if self.stop == Some(at) {
                return Err(LinkedShaperError::AllocationFailure);
            }
            self.paid += amount;
            Ok(())
        }
    }
    fn encode<S: Storage>(
        data: &[u8],
        split: usize,
        storage: &mut S,
    ) -> Result<[u8; 32], ProductionTextShapeError> {
        let mut digest = Digest::new(NodeId::new(7), storage)?;
        digest.update(&[0, 1, 2])?;
        digest.owner(NodeId::new(11));
        digest.hash_into(data)?;
        digest.update(&data[..split])?;
        digest.update(&data[split..])?;
        digest.finish()
    }
    #[test]
    fn shape_digest_chunk_boundaries_nested_hashes_and_every_work_refusal_keep_bytes_owner_prefix()
    {
        let mut cases = 0;
        let mut cutoffs = 0;
        for len in [0, 1, 55, 56, 63, 64, 65, 127, 128, 129, 1024, 4096] {
            let data = (0..len).map(|i| (i % 251) as u8).collect::<Vec<_>>();
            let mut expected = Sha256::new();
            expected.update(&[0, 1, 2]);
            expected.update(&sha256(&data));
            expected.update(&data);
            let expected = expected.finish();
            for split in [0, len / 2, len] {
                let mut full = Trace::default();
                assert_eq!(encode(&data, split, &mut full).unwrap(), expected);
                for stop in 0..full.requests.len() {
                    let mut short = Trace {
                        stop: Some(stop),
                        ..Trace::default()
                    };
                    assert_eq!(
                        encode(&data, split, &mut short),
                        Err(error(
                            full.requests[stop].0,
                            ProductionTextShapeErrorKind::Backend(
                                LinkedShaperError::AllocationFailure
                            )
                        ))
                    );
                    assert_eq!(short.requests, &full.requests[..stop + 1]);
                    assert_eq!(
                        short.paid,
                        full.requests[..stop].iter().map(|(_, n)| n).sum::<u64>()
                    );
                    cutoffs += 1;
                }
                cases += 1;
            }
        }
        eprintln!("shape digest byte-stream cases={cases}, work refusal cutoffs={cutoffs}");
    }
    #[test]
    fn shape_digest_empty_abc_golden_and_ordinary_hooks_are_unchanged() {
        struct Silent;
        impl Storage for Silent {
            const OBSERVED: bool = false;
            fn owner(&mut self, _: NodeId) {
                panic!()
            }
            fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
                panic!()
            }
            fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
                panic!()
            }
            fn work(&mut self, _: u64) -> Result<(), LinkedShaperError> {
                panic!()
            }
        }
        for (input, expected) in [
            (
                &b""[..],
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                &b"abc"[..],
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
        ] {
            let digest = hash(NodeId::new(9), input, &mut Silent).unwrap();
            assert_eq!(
                digest
                    .iter()
                    .map(|v| format!("{v:02x}"))
                    .collect::<String>(),
                expected
            );
        }
    }
}
