//! Full admitted instance tables without a temporary set or canonical String.
//! The caller pays requested rows/bytes and work; allocator overcapacity is not
//! measured here. This callback cannot issue or replace resource admission.
use super::*;
use typaxis_core::Sha256;

/// Private-staging /3 requests; every refusal precedes the requested operation.
pub trait ProductionFontInstanceBudgetV3 {
    type Error;
    fn reserve_faces(&mut self, count: usize) -> Result<(), Self::Error>;
    fn work(&mut self, amount: u64) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub enum ProductionFontInstanceErrorV3<E> {
    Budget(E),
    ResourceLimit,
    AllocationFailure,
    InvalidFaceOrder,
}

impl<'a> AdmittedProductionFontInstancesV3<'a> {
    /// Copies the authentic ledger's full canonical face order. Used subsets
    /// retain the separate from_used_faces entry point. Neither entry point
    /// authorizes a public profile or makes an allocator-capacity guarantee.
    pub fn from_all_faces_with_budget<B: ProductionFontInstanceBudgetV3>(
        ledger: &'a AdmittedProductionResourceLedgerV3,
        budget: &mut B,
    ) -> Result<Self, ProductionFontInstanceErrorV3<B::Error>> {
        use ProductionFontInstanceErrorV3 as E;
        let count = ledger.fonts().len();
        if u64::try_from(count)
            .ok()
            .filter(|n| *n <= ledger.effective_limits().base().get().max_fragments)
            .is_none()
        {
            return Err(E::ResourceLimit);
        }
        budget.reserve_faces(count).map_err(E::Budget)?;
        let mut faces = Vec::new();
        faces
            .try_reserve_exact(count)
            .map_err(|_| E::AllocationFailure)?;
        for font in ledger.fonts() {
            budget.work(1).map_err(E::Budget)?;
            let id = font.font_face_id();
            if faces.last().is_some_and(|previous| *previous >= id) {
                return Err(E::InvalidFaceOrder);
            }
            faces.push(id);
        }
        let mut canonical = Sha256::new();
        hash_bytes(
            &mut canonical,
            budget,
            b"{\"algorithm\":\"typaxis.production-font-instances/3\",\"faces\":[",
        )?;
        for (index, face) in faces.iter().enumerate() {
            if index != 0 {
                hash_bytes(&mut canonical, budget, b",")?;
            }
            hash_face_id(&mut canonical, budget, face.get())?;
        }
        hash_bytes(&mut canonical, budget, b"],\"ledger_fingerprint\":")?;
        let mut hex = [b'"'; 66];
        budget.work(32).map_err(E::Budget)?;
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        for (index, byte) in ledger.fingerprint().into_iter().enumerate() {
            hex[1 + index * 2] = DIGITS[usize::from(byte >> 4)];
            hex[2 + index * 2] = DIGITS[usize::from(byte & 15)];
        }
        hash_bytes(&mut canonical, budget, &hex)?;
        hash_bytes(&mut canonical, budget, b"}")?;
        Ok(Self {
            ledger,
            faces,
            fingerprint: canonical.finish(),
        })
    }
}

fn hash_bytes<B: ProductionFontInstanceBudgetV3>(
    hash: &mut Sha256,
    budget: &mut B,
    bytes: &[u8],
) -> Result<(), ProductionFontInstanceErrorV3<B::Error>> {
    budget
        .work(bytes.len() as u64)
        .map_err(ProductionFontInstanceErrorV3::Budget)?;
    hash.update(bytes);
    Ok(())
}

fn hash_face_id<B: ProductionFontInstanceBudgetV3>(
    hash: &mut Sha256,
    budget: &mut B,
    mut value: u32,
) -> Result<(), ProductionFontInstanceErrorV3<B::Error>> {
    let mut digits = [0u8; 10];
    let mut start = digits.len();
    loop {
        budget
            .work(1)
            .map_err(ProductionFontInstanceErrorV3::Budget)?;
        start -= 1;
        digits[start] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    hash_bytes(hash, budget, &digits[start..])
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Work(u64);
    impl ProductionFontInstanceBudgetV3 for Work {
        type Error = std::convert::Infallible;
        fn reserve_faces(&mut self, _: usize) -> Result<(), Self::Error> {
            panic!("decimal hashing must not reserve storage")
        }
        fn work(&mut self, n: u64) -> Result<(), Self::Error> {
            self.0 += n;
            Ok(())
        }
    }
    #[test]
    fn observed_font_instance_decimal_hash_matches_all_u32_digit_boundaries() {
        for value in [
            0,
            1,
            9,
            10,
            99,
            100,
            999,
            1_000,
            9_999,
            10_000,
            99_999,
            100_000,
            999_999,
            1_000_000,
            9_999_999,
            10_000_000,
            99_999_999,
            100_000_000,
            999_999_999,
            1_000_000_000,
            u32::MAX,
        ] {
            let mut hash = Sha256::new();
            let mut work = Work(0);
            hash_face_id(&mut hash, &mut work, value).unwrap();
            let expected = value.to_string();
            assert_eq!(hash.finish(), sha256(expected.as_bytes()));
            assert_eq!(work.0, 2 * expected.len() as u64);
        }
    }
}
