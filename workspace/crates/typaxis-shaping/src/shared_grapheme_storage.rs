//! Ordinary graphemes retain the registry reader; observed scans pay inside
//! the pinned local engine, including backward lookups and terminal visits.
use super::*;
use crate::shared_shape_storage::Storage;

#[derive(Clone)]
enum Reader<'a> {
    Ordinary(unicode_segmentation::GraphemeIndices<'a>),
    Observed(typaxis_unicode_segmentation::GraphemeIndices<'a>),
}
#[derive(Clone)]
pub(super) struct Graphemes<'a>(Reader<'a>);
impl<'a> Graphemes<'a> {
    pub(super) fn new<S: Storage>(
        text: &'a str,
        storage: &mut S,
    ) -> Result<Self, LinkedShaperError> {
        if S::OBSERVED {
            storage.work(1)?;
            Ok(Self(Reader::Observed(
                typaxis_unicode_segmentation::UnicodeSegmentation::grapheme_indices(text, true),
            )))
        } else {
            Ok(Self(Reader::Ordinary(
                UnicodeSegmentation::grapheme_indices(text, true),
            )))
        }
    }
    pub(super) fn next<S: Storage>(
        &mut self,
        storage: &mut S,
    ) -> Result<Option<(usize, &'a str)>, LinkedShaperError> {
        match &mut self.0 {
            Reader::Ordinary(iter) => Ok(iter.next()),
            Reader::Observed(iter) => iter.try_next_with_work(&mut || storage.work(1)),
        }
    }
}

pub(super) struct Boundaries<'a> {
    reader: Graphemes<'a>,
    len: usize,
    current: Option<usize>,
    loaded: bool,
    terminal: bool,
}
impl<'a> Boundaries<'a> {
    pub(super) fn new<S: Storage>(
        text: &'a str,
        storage: &mut S,
    ) -> Result<Self, LinkedShaperError> {
        Ok(Self {
            reader: Graphemes::new(text, storage)?,
            len: text.len(),
            current: None,
            loaded: false,
            terminal: false,
        })
    }
    pub(super) fn peek<S: Storage>(
        &mut self,
        storage: &mut S,
    ) -> Result<Option<usize>, LinkedShaperError> {
        if !self.loaded {
            self.current = if self.terminal {
                None
            } else if let Some((at, _)) = self.reader.next(storage)? {
                Some(at)
            } else {
                self.terminal = true;
                Some(self.len)
            };
            self.loaded = true;
        }
        Ok(self.current)
    }
    pub(super) fn advance<S: Storage>(&mut self, storage: &mut S) -> Result<(), LinkedShaperError> {
        self.peek(storage)?;
        self.loaded = false;
        Ok(())
    }
}

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../../vendor/typaxis-unicode-segmentation/tests/testdata/mod.rs"]
pub(super) mod reference_testdata;

#[cfg(test)]
mod tests {
    use super::reference_testdata as testdata;
    use super::*;
    use crate::shared_shape_storage::Site;
    #[derive(Default)]
    struct Trace {
        calls: usize,
        stop: Option<usize>,
    }
    impl Storage for Trace {
        fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
            panic!("borrowed grapheme reader must not reserve")
        }
        fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
            panic!("borrowed grapheme reader must not retain")
        }
        fn work(&mut self, n: u64) -> Result<(), LinkedShaperError> {
            assert_eq!(n, 1);
            let at = self.calls;
            self.calls += 1;
            if self.stop == Some(at) {
                Err(LinkedShaperError::AllocationFailure)
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn grapheme_storage_matches_independent_registry_unicode16_cases_and_work_refusal() {
        let mut cases = 0;
        let mut cutoffs = 0;
        for text in testdata::TEST_SAME
            .iter()
            .map(|r| r.0)
            .chain(testdata::TEST_DIFF.iter().map(|r| r.0))
        {
            let expected = UnicodeSegmentation::grapheme_indices(text, true).collect::<Vec<_>>();
            let mut storage = Trace::default();
            let mut iter = Graphemes::new(text, &mut storage).unwrap();
            let mut actual = Vec::new();
            while let Some(item) = iter.next(&mut storage).unwrap() {
                actual.push(item);
            }
            assert_eq!(actual, expected);
            let mut ordinary =
                Graphemes::new(text, &mut crate::shared_shape_storage::NoStorage).unwrap();
            let mut unobserved = Vec::new();
            while let Some(item) = ordinary
                .next(&mut crate::shared_shape_storage::NoStorage)
                .unwrap()
            {
                unobserved.push(item);
            }
            assert_eq!(unobserved, expected);
            for stop in 0..storage.calls {
                let mut short = Trace {
                    stop: Some(stop),
                    ..Trace::default()
                };
                let result = (|| {
                    let mut iter = Graphemes::new(text, &mut short)?;
                    while iter.next(&mut short)?.is_some() {}
                    Ok::<(), LinkedShaperError>(())
                })();
                assert_eq!(result, Err(LinkedShaperError::AllocationFailure));
                assert_eq!(short.calls, stop + 1);
                cutoffs += 1;
            }
            cases += 1;
        }
        eprintln!("independent registry grapheme cases={cases}, work refusal cutoffs={cutoffs}");
    }
    #[test]
    fn grapheme_storage_boundary_peeks_and_terminal_offset_keep_merge_semantics() {
        for text in [
            "",
            "Result",
            "a\u{0308}b",
            "👩\u{200d}👩",
            "🇦🇧🇨",
            "क्\u{0937}",
        ] {
            let expected = UnicodeSegmentation::grapheme_indices(text, true)
                .map(|(at, _)| at)
                .chain(core::iter::once(text.len()))
                .collect::<Vec<_>>();
            let mut storage = Trace::default();
            let mut reader = Boundaries::new(text, &mut storage).unwrap();
            let mut actual = Vec::new();
            while let Some(at) = reader.peek(&mut storage).unwrap() {
                let paid = storage.calls;
                assert_eq!(reader.peek(&mut storage).unwrap(), Some(at));
                assert_eq!(storage.calls, paid);
                actual.push(at);
                reader.advance(&mut storage).unwrap();
            }
            assert_eq!(actual, expected);
            assert_eq!(reader.peek(&mut storage).unwrap(), None);
        }
    }
}
