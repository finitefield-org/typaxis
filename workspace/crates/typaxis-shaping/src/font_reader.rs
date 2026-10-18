//! Borrowed read-fonts 0.31.3 setup, prepaid before its directory scans.
//! Callers include every table search they authorize, including provider
//! dependencies. This allowance does not cover later shaping-engine reads.
use super::{shared_shape_storage::Storage, LinkedShaperError};

pub(super) fn read_with_tables<'a, S: Storage>(
    bytes: &'a [u8],
    index: u32,
    table_reads: u8,
    storage: &mut S,
) -> Result<harfrust::FontRef<'a>, LinkedShaperError> {
    if S::OBSERVED {
        storage.work(1)?;
        let count = directory_count(bytes, index).ok_or(LinkedShaperError::InvalidFontOrFace)?;
        // Two directory-length terms cover construction/search bookkeeping;
        // each authorized table read adds one complete directory search. All
        // counts are bounded by u16/u8, so this u64 sum cannot overflow.
        let reads = u64::from(table_reads);
        storage.work((reads + 2) * u64::from(count) + 8 * (reads + 1))?;
    }
    harfrust::FontRef::from_index(bytes, index).map_err(|_| LinkedShaperError::InvalidFontOrFace)
}

fn directory_count(bytes: &[u8], index: u32) -> Option<u16> {
    let u32_at = |at: usize| {
        Some(u32::from_be_bytes(
            bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
        ))
    };
    let directory = if bytes.get(..4)? == b"ttcf" {
        if index >= u32_at(8)? {
            return None;
        }
        let at = usize::try_from(index)
            .ok()?
            .checked_mul(4)?
            .checked_add(12)?;
        usize::try_from(u32_at(at)?).ok()?
    } else {
        if index != 0 {
            return None;
        }
        0
    };
    let at = directory.checked_add(4)?;
    Some(u16::from_be_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared_shape_storage::{NoStorage, Site};
    use read_fonts::TableProvider;
    const FONT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/profiles/production-book-1/combined/job/body.ttf"
    ));
    const COLLECTION: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/collection.bin"));
    #[derive(Default)]
    struct Trace {
        requests: Vec<u64>,
        paid: u64,
        stop: Option<usize>,
    }
    impl Storage for Trace {
        fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
            panic!("borrowed font setup must not reserve")
        }
        fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
            panic!("borrowed font setup must not retain records")
        }
        fn work(&mut self, n: u64) -> Result<(), LinkedShaperError> {
            let at = self.requests.len();
            self.requests.push(n);
            if self.stop == Some(at) {
                return Err(LinkedShaperError::AllocationFailure);
            }
            self.paid += n;
            Ok(())
        }
    }
    #[test]
    fn font_reader_setup_keeps_borrowed_tables_and_stops_before_each_request() {
        for bytes in [FONT, COLLECTION] {
            let original = harfrust::FontRef::from_index(bytes, 0).unwrap();
            for reads in [0, 1, 5, u8::MAX] {
                let mut full = Trace::default();
                let face = read_with_tables(bytes, 0, reads, &mut full).unwrap();
                assert_eq!(
                    face.head().unwrap().units_per_em(),
                    original.head().unwrap().units_per_em()
                );
                assert_eq!(
                    face.hhea().unwrap().ascender(),
                    original.hhea().unwrap().ascender()
                );
                assert_eq!(
                    face.cmap().unwrap().map_codepoint('R'),
                    original.cmap().unwrap().map_codepoint('R')
                );
                assert_eq!(face.data().as_bytes().as_ptr(), bytes.as_ptr());
                assert_eq!(full.requests.len(), 2);
                if reads == 1 {
                    assert_eq!(
                        full.requests,
                        [1, 3 * u64::from(face.table_directory().num_tables()) + 16]
                    );
                }
                for stop in 0..2 {
                    let mut short = Trace {
                        stop: Some(stop),
                        ..Trace::default()
                    };
                    assert_eq!(
                        read_with_tables(bytes, 0, reads, &mut short).err().unwrap(),
                        LinkedShaperError::AllocationFailure
                    );
                    assert_eq!(short.requests, &full.requests[..stop + 1]);
                    assert_eq!(short.paid, full.requests[..stop].iter().sum::<u64>());
                }
                let ordinary = read_with_tables(bytes, 0, reads, &mut NoStorage).unwrap();
                assert_eq!(
                    ordinary.data().as_bytes().as_ptr(),
                    face.data().as_bytes().as_ptr()
                );
            }
        }
    }
    #[test]
    fn font_reader_refusal_precedes_invalid_header_and_ordinary_never_calls_storage() {
        struct Silent;
        impl Storage for Silent {
            const OBSERVED: bool = false;
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
        read_with_tables(FONT, 0, 5, &mut Silent).unwrap();
        for (bytes, index) in [(&[][..], 0), (FONT, 1), (COLLECTION, u32::MAX)] {
            assert_eq!(
                read_with_tables(bytes, index, 1, &mut Silent)
                    .err()
                    .unwrap(),
                LinkedShaperError::InvalidFontOrFace
            );
            let mut refused = Trace {
                stop: Some(0),
                ..Trace::default()
            };
            assert_eq!(
                read_with_tables(bytes, index, 1, &mut refused)
                    .err()
                    .unwrap(),
                LinkedShaperError::AllocationFailure
            );
            assert_eq!(refused.paid, 0);
            let mut accepted = Trace::default();
            assert_eq!(
                read_with_tables(bytes, index, 1, &mut accepted)
                    .err()
                    .unwrap(),
                LinkedShaperError::InvalidFontOrFace
            );
            assert_eq!(accepted.requests, [1]);
        }
    }
}
