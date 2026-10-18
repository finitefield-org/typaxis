//! Fallible owned style copies for an observed book-2 source candidate.
use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BookV2StyleCopyError<E> {
    Budget(E),
    Allocation,
}

impl SemanticContainerInheritanceStyle {
    /// Copies owned font-family storage after the caller accepts each request.
    ///
    /// The callback receives additional records, requested storage bytes, and
    /// work. Records count family carrier slots; storage counts the full new
    /// `Vec<String>` request and each new UTF-8 request while the source stays
    /// live. Work counts scalar visits, allocation requests, and copied bytes.
    /// Allocator capacity and process memory use are not measured here.
    pub fn try_copy_book_v2_counted<E>(
        &self,
        charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
    ) -> Result<Self, BookV2StyleCopyError<E>> {
        use BookV2StyleCopyError as Error;
        charge(0, 0, 1).map_err(Error::Budget)?;
        let font_families = match &self.font_families {
            None => None,
            Some(source) => {
                let records = u64::try_from(source.len()).map_err(|_| Error::Allocation)?;
                let bytes = source
                    .len()
                    .checked_mul(std::mem::size_of::<String>())
                    .and_then(|n| u64::try_from(n).ok())
                    .ok_or(Error::Allocation)?;
                charge(records, bytes, 1).map_err(Error::Budget)?;
                super::super::initial_style::allocation_attempt::<E>(bytes)
                    .map_err(|_| Error::Allocation)?;
                let mut families = Vec::new();
                families
                    .try_reserve_exact(source.len())
                    .map_err(|_| Error::Allocation)?;
                for family in source {
                    let bytes = u64::try_from(family.len()).map_err(|_| Error::Allocation)?;
                    charge(0, bytes, 1).map_err(Error::Budget)?;
                    super::super::initial_style::allocation_attempt::<E>(bytes)
                        .map_err(|_| Error::Allocation)?;
                    let mut copied = String::new();
                    copied
                        .try_reserve_exact(family.len())
                        .map_err(|_| Error::Allocation)?;
                    charge(0, 0, bytes).map_err(Error::Budget)?;
                    copied.push_str(family);
                    charge(0, 0, 1).map_err(Error::Budget)?;
                    families.push(copied);
                }
                Some(families)
            }
        };
        Ok(Self {
            block_style: self.block_style,
            font_families,
            font_size: self.font_size,
            line_height: self.line_height,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(families: Option<Vec<String>>) -> SemanticContainerInheritanceStyle {
        let mut value =
            cascade_book_v2_descendant_style("paragraph", &[], &StyleSheet { rules: vec![] }, None)
                .unwrap();
        value.font_families = families;
        value.font_size = Some(PositiveLength::new(Length::from_raw(17).unwrap()).unwrap());
        value.line_height = Some(PositiveLength::new(Length::from_raw(23).unwrap()).unwrap());
        value
    }

    #[test]
    fn counted_style_copy_preserves_none_empty_unicode_and_owned_storage() {
        for families in [
            None,
            Some(vec![]),
            Some(vec![String::new()]),
            Some(vec!["日本語".to_owned(), "A\"\\\nB".to_owned()]),
        ] {
            let source = style(families);
            let mut charges = Vec::new();
            let copied = source
                .try_copy_book_v2_counted(&mut |r, s, w| {
                    charges.push([r, s, w]);
                    Ok::<_, ()>(())
                })
                .unwrap();
            assert_eq!(source, copied);
            let source_families = source.font_families();
            let count = source_families.map_or(0, |v| v.len() as u64);
            let payload =
                source_families.map_or(0, |v| v.iter().map(|s| s.len() as u64).sum::<u64>());
            assert_eq!(charges.iter().map(|c| c[0]).sum::<u64>(), count);
            assert_eq!(
                charges.iter().map(|c| c[1]).sum::<u64>(),
                count * std::mem::size_of::<String>() as u64 + payload
            );
            if let (Some(original), Some(copy)) = (source_families, copied.font_families()) {
                for (a, b) in original.iter().zip(copy) {
                    if !a.is_empty() {
                        assert_ne!(a.as_ptr(), b.as_ptr());
                    }
                }
            }
        }
    }

    #[test]
    fn counted_style_copy_retains_every_accepted_prefix_before_refusal() {
        let source = style(Some(vec!["αβ".to_owned(), "second".to_owned()]));
        let mut requests = Vec::new();
        source
            .try_copy_book_v2_counted(&mut |r, s, w| {
                requests.push([r, s, w]);
                Ok::<_, ()>(())
            })
            .unwrap();
        for fail_at in 0..requests.len() {
            let mut accepted = Vec::new();
            let mut calls = 0;
            let error = source
                .try_copy_book_v2_counted(&mut |r, s, w| {
                    let index = calls;
                    calls += 1;
                    if index == fail_at {
                        return Err(index);
                    }
                    accepted.push([r, s, w]);
                    Ok(())
                })
                .unwrap_err();
            assert_eq!(error, BookV2StyleCopyError::Budget(fail_at));
            assert_eq!(calls, fail_at + 1);
            assert_eq!(accepted, requests[..fail_at]);
        }
    }

    #[test]
    fn counted_style_copy_exact_and_one_short_caps_are_atomic() {
        let source = style(Some(vec!["one".to_owned(), "三".to_owned()]));
        let mut exact = [0; 3];
        source
            .try_copy_book_v2_counted(&mut |r, s, w| {
                for (total, value) in exact.iter_mut().zip([r, s, w]) {
                    *total += value;
                }
                Ok::<_, ()>(())
            })
            .unwrap();
        for axis in 0..3 {
            for short in [false, true] {
                let mut caps = exact;
                caps[axis] -= u64::from(short);
                let mut used = [0; 3];
                let result = source.try_copy_book_v2_counted(&mut |r, s, w| {
                    let next = [used[0] + r, used[1] + s, used[2] + w];
                    if next.iter().zip(caps).any(|(n, cap)| *n > cap) {
                        return Err(axis);
                    }
                    used = next;
                    Ok(())
                });
                if short {
                    assert_eq!(result.unwrap_err(), BookV2StyleCopyError::Budget(axis));
                    assert!(used.iter().zip(caps).all(|(n, cap)| *n <= cap));
                } else {
                    assert_eq!(result.unwrap(), source);
                    assert_eq!(used, exact);
                }
            }
        }
    }
}
