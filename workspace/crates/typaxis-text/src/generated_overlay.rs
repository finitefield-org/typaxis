//! A bounded generated namespace for an explicitly selected semantic stage.
//! This is not a complete document GeneratedTextStore or a source authorization.
//! The syntax owner must derive and verify every key/text pair against its source.
use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GeneratedOverlayCountedError<E> {
    Budget(E),
    Overlay(GeneratedTextStoreError),
    Allocation,
}

fn overlay_total<E>(
    records: &[(GeneratedBufferKey, String)],
    limits: &ValidatedResourceLimits,
    retained_text_bytes: u64,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<u64, GeneratedOverlayCountedError<E>> {
    use GeneratedOverlayCountedError as Error;
    if records.len() as u64 > limits.get().max_fragments {
        return Err(Error::Overlay(GeneratedTextStoreError::ResourceLimit));
    }
    let mut total = retained_text_bytes;
    for (_, text) in records {
        charge(0, 0, 1).map_err(Error::Budget)?;
        if text.len() as u64 > u64::from(limits.get().max_text_buffer_bytes) {
            return Err(Error::Overlay(GeneratedTextStoreError::ResourceLimit));
        }
        total = total
            .checked_add(text.len() as u64)
            .ok_or(Error::Overlay(GeneratedTextStoreError::ResourceLimit))?;
    }
    if total > limits.get().max_text_bytes {
        return Err(Error::Overlay(GeneratedTextStoreError::ResourceLimit));
    }
    Ok(total)
}

fn uncounted_error(
    error: GeneratedOverlayCountedError<std::convert::Infallible>,
) -> GeneratedTextStoreError {
    match error {
        GeneratedOverlayCountedError::Overlay(error) => error,
        GeneratedOverlayCountedError::Budget(never) => match never {},
        GeneratedOverlayCountedError::Allocation => GeneratedTextStoreError::ResourceLimit,
    }
}

fn overlay_buffer(
    index: usize,
    key: GeneratedBufferKey,
    utf8: String,
) -> Result<GeneratedTextBuffer, GeneratedTextStoreError> {
    Ok(GeneratedTextBuffer {
        key,
        utf8,
        text_id: GeneratedTextBufferId::new(
            u32::try_from(index).map_err(|_| GeneratedTextStoreError::TooManyBuffers)?,
        ),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedTextOverlay {
    buffers: Vec<GeneratedTextBuffer>,
    reference_fingerprint: ReferenceFingerprint,
    generated_bytes: u64,
}
impl GeneratedTextOverlay {
    pub fn new(
        mut records: Vec<(GeneratedBufferKey, String)>,
        limits: &ValidatedResourceLimits,
        retained_text_bytes: u64,
    ) -> Result<Self, GeneratedTextStoreError> {
        use GeneratedTextStoreError as E;
        let total = overlay_total(
            records.as_slice(),
            limits,
            retained_text_bytes,
            &mut |_, _, _| Ok::<_, std::convert::Infallible>(()),
        )
        .map_err(uncounted_error)?;
        records.sort_unstable_by_key(|r| r.0);
        if records.windows(2).any(|r| r[0].0 == r[1].0) {
            return Err(E::DuplicateKey);
        }
        let reference_fingerprint = generated_text_reference_fingerprint(&records);
        let mut buffers = Vec::new();
        buffers
            .try_reserve_exact(records.len())
            .map_err(|_| E::ResourceLimit)?;
        for (index, (key, utf8)) in records.into_iter().enumerate() {
            buffers.push(overlay_buffer(index, key, utf8)?);
        }
        Ok(Self {
            buffers,
            reference_fingerprint,
            generated_bytes: total - retained_text_bytes,
        })
    }

    /// Builds from strictly increasing canonical keys without copying strings.
    ///
    /// A decreasing key returns `Overlay(UnknownKey)`; an equal key returns
    /// `Overlay(DuplicateKey)`. Unlike [`Self::new`], this entry point never
    /// sorts. The caller owns the input vector and its strings before entry.
    /// The callback pays the full new buffer-vector request (records = slots,
    /// storage = slots times the concrete buffer size), visits, moves, and
    /// streaming canonical SHA-256 work before they occur. Input storage and
    /// string payloads are not charged again. Requests describe requested
    /// storage, not allocator capacity or source authorization.
    pub fn try_from_sorted_records_counted<E>(
        records: Vec<(GeneratedBufferKey, String)>,
        limits: &ValidatedResourceLimits,
        retained_text_bytes: u64,
        charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
    ) -> Result<Self, GeneratedOverlayCountedError<E>> {
        use GeneratedOverlayCountedError as Error;
        let total = overlay_total(&records, limits, retained_text_bytes, charge)?;
        for pair in records.windows(2) {
            charge(0, 0, 1).map_err(Error::Budget)?;
            match pair[0].0.cmp(&pair[1].0) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => {
                    return Err(Error::Overlay(GeneratedTextStoreError::DuplicateKey));
                }
                std::cmp::Ordering::Greater => {
                    return Err(Error::Overlay(GeneratedTextStoreError::UnknownKey));
                }
            }
        }
        let reference_fingerprint = sorted_reference_fingerprint(&records, charge)?;
        let count = u64::try_from(records.len())
            .map_err(|_| Error::Overlay(GeneratedTextStoreError::ResourceLimit))?;
        let bytes = records
            .len()
            .checked_mul(std::mem::size_of::<GeneratedTextBuffer>())
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(Error::Overlay(GeneratedTextStoreError::ResourceLimit))?;
        charge(count, bytes, 1).map_err(Error::Budget)?;
        let mut buffers = Vec::new();
        buffers
            .try_reserve_exact(records.len())
            .map_err(|_| Error::Allocation)?;
        for (index, (key, utf8)) in records.into_iter().enumerate() {
            charge(0, 0, 1).map_err(Error::Budget)?;
            buffers.push(overlay_buffer(index, key, utf8).map_err(Error::Overlay)?);
        }
        Ok(Self {
            buffers,
            reference_fingerprint,
            generated_bytes: total - retained_text_bytes,
        })
    }
    pub fn buffers(&self) -> &[GeneratedTextBuffer] {
        &self.buffers
    }
    pub const fn reference_fingerprint(&self) -> ReferenceFingerprint {
        self.reference_fingerprint
    }
    pub const fn generated_bytes(&self) -> u64 {
        self.generated_bytes
    }
    pub fn buffer(&self, key: GeneratedBufferKey) -> Option<&GeneratedTextBuffer> {
        self.buffers
            .binary_search_by_key(&key, |b| b.key)
            .ok()
            .map(|i| &self.buffers[i])
    }
    pub fn provenance(
        &self,
        key: GeneratedBufferKey,
        start: Utf8ByteOffset,
        end: Utf8ByteOffset,
    ) -> Result<GeneratedProvenance, GeneratedTextStoreError> {
        let buffer = self
            .buffer(key)
            .ok_or(GeneratedTextStoreError::UnknownKey)?;
        if end.get() as usize > buffer.utf8.len()
            || !buffer.utf8.is_char_boundary(start.get() as usize)
            || !buffer.utf8.is_char_boundary(end.get() as usize)
        {
            return Err(GeneratedTextStoreError::SpanOutOfBounds);
        }
        Ok(GeneratedProvenance {
            buffer_key: key,
            text_span: GeneratedTextSpan::new(buffer.text_id, start, end)
                .ok_or(GeneratedTextStoreError::SpanOutOfBounds)?,
        })
    }
    pub fn validates_provenance(&self, value: GeneratedProvenance) -> bool {
        let span = value.text_span();
        let range = span.range();
        self.provenance(value.buffer_key(), range.start_byte(), range.end_byte())
            .ok()
            == Some(value)
    }
}

/// Retains only the SHA state; canonical escaping is shared with the core JCS
/// writer. Work counts each writer call, its bytes, completed SHA blocks, and
/// the final one or two padding blocks. There is no canonical Vec or String.
struct CountedOverlayHash<'c, E, C> {
    state: typaxis_core::Sha256,
    written: u64,
    charge: &'c mut C,
    failure: Option<GeneratedOverlayCountedError<E>>,
}
impl<E, C: FnMut(u64, u64, u64) -> Result<(), E>> std::fmt::Write for CountedOverlayHash<'_, E, C> {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        use GeneratedOverlayCountedError as Error;
        if self.failure.is_some() {
            return Err(std::fmt::Error);
        }
        let Some((next, work)) = u64::try_from(value.len()).ok().and_then(|bytes| {
            let next = self.written.checked_add(bytes)?;
            let blocks = next / 64 - self.written / 64;
            let work = bytes.checked_add(blocks)?.checked_add(1)?;
            Some((next, work))
        }) else {
            self.failure = Some(Error::Overlay(GeneratedTextStoreError::ResourceLimit));
            return Err(std::fmt::Error);
        };
        if let Err(error) = (self.charge)(0, 0, work) {
            self.failure = Some(Error::Budget(error));
            return Err(std::fmt::Error);
        }
        self.state.update(value.as_bytes());
        self.written = next;
        Ok(())
    }
}

fn sorted_reference_fingerprint<E>(
    records: &[(GeneratedBufferKey, String)],
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<ReferenceFingerprint, GeneratedOverlayCountedError<E>> {
    use std::fmt::Write;
    use typaxis_core::{generation_kind_wire_name, write_jcs_string};
    let mut output = CountedOverlayHash {
        state: typaxis_core::Sha256::new(),
        written: 0,
        charge,
        failure: None,
    };
    let result = (|| -> std::fmt::Result {
        output.write_str("{\"algorithm\":")?;
        write_jcs_string(&mut output, ReferenceFingerprint::ALGORITHM_ID)?;
        output.write_str(",\"resolved_generated_text\":[")?;
        for (index, (key, utf8)) in records.iter().enumerate() {
            if index > 0 {
                output.write_char(',')?;
            }
            write!(
                output,
                "{{\"end_byte\":{},\"key\":{{\"generation_kind\":",
                utf8.len()
            )?;
            write_jcs_string(
                &mut output,
                generation_kind_wire_name(key.generation_kind()),
            )?;
            write!(
                output,
                ",\"owner\":{},\"owner_local_ordinal\":{}}},\"start_byte\":0,\"utf8\":",
                key.owner().get(),
                key.owner_local_ordinal()
            )?;
            write_jcs_string(&mut output, utf8)?;
            output.write_char('}')?;
        }
        output.write_str("]}")
    })();
    if result.is_err() {
        return Err(output
            .failure
            .take()
            .unwrap_or(GeneratedOverlayCountedError::Overlay(
                GeneratedTextStoreError::ResourceLimit,
            )));
    }
    let tail_blocks = if output.written % 64 < 56 { 1 } else { 2 };
    (output.charge)(0, 0, tail_blocks).map_err(GeneratedOverlayCountedError::Budget)?;
    Ok(ReferenceFingerprint::from_untrusted_bytes(
        output.state.finish(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{GenerationKind, NodeId, ResourceLimits};
    fn key(owner: u32) -> GeneratedBufferKey {
        GeneratedBufferKey::new(NodeId::new(owner), GenerationKind::ListMarker, 0)
    }
    #[test]
    fn overlay_has_canonical_generated_ids_and_combined_text_boundaries() {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_text_bytes: 10,
            max_text_buffer_bytes: 3,
            max_shaping_context_bytes: 3,
            ..ResourceLimits::default()
        })
        .unwrap();
        let records = vec![(key(4), "•".into()), (key(2), "9.".into())];
        let overlay = GeneratedTextOverlay::new(records.clone(), &limits, 5).unwrap();
        let mut reversed = records.clone();
        reversed.reverse();
        assert_eq!(
            overlay,
            GeneratedTextOverlay::new(reversed, &limits, 5).unwrap()
        );
        assert_eq!(overlay.generated_bytes(), 5);
        assert_eq!(overlay.buffers()[0].key(), key(2));
        let span = overlay
            .provenance(key(4), Utf8ByteOffset::new(0), Utf8ByteOffset::new(3))
            .unwrap();
        assert_eq!(span.text_span().text_id().get(), 1);
        assert!(overlay.validates_provenance(span));
        assert_eq!(
            overlay.provenance(key(4), Utf8ByteOffset::new(0), Utf8ByteOffset::new(1)),
            Err(GeneratedTextStoreError::SpanOutOfBounds)
        );
        assert_eq!(
            GeneratedTextOverlay::new(records, &limits, 6).unwrap_err(),
            GeneratedTextStoreError::ResourceLimit
        );
        assert_eq!(
            GeneratedTextOverlay::new(vec![(key(2), "100.".into())], &limits, 0).unwrap_err(),
            GeneratedTextStoreError::ResourceLimit
        );
        assert_eq!(
            GeneratedTextOverlay::new(
                vec![(key(2), "1.".into()), (key(2), "2.".into())],
                &limits,
                0
            )
            .unwrap_err(),
            GeneratedTextStoreError::DuplicateKey
        );
        let other = GeneratedTextOverlay::new(vec![(key(4), "•".into())], &limits, 0).unwrap();
        assert!(!other.validates_provenance(span));
    }

    fn limits() -> ValidatedResourceLimits {
        ValidatedResourceLimits::new(ResourceLimits::default()).unwrap()
    }

    fn counted_records() -> Vec<(GeneratedBufferKey, String)> {
        vec![
            (key(1), "日本語😀".to_owned()),
            (key(3), "\"\\\n\t\u{0000}\u{001f}".to_owned()),
        ]
    }

    #[test]
    fn counted_overlay_matches_legacy_all_keys_escaping_and_moved_strings() {
        let kinds = [
            GenerationKind::PageReference,
            GenerationKind::Counter,
            GenerationKind::ListMarker,
            GenerationKind::FootnoteMarker,
            GenerationKind::Discretionary,
        ];
        let controls: String = (0..32).map(|n| char::from_u32(n).unwrap()).collect();
        let mut records = vec![];
        for owner in [0, 10, u32::MAX] {
            for kind in kinds {
                for ordinal in [0, u32::MAX] {
                    records.push((
                        GeneratedBufferKey::new(NodeId::new(owner), kind, ordinal),
                        format!("{controls}\"\\日本語😀"),
                    ));
                }
            }
        }
        records.sort_unstable_by_key(|r| r.0);
        let legacy = GeneratedTextOverlay::new(records.clone(), &limits(), 7).unwrap();
        let pointers: Vec<_> = records.iter().map(|(_, s)| s.as_ptr()).collect();
        let mut used = [0; 3];
        let counted = GeneratedTextOverlay::try_from_sorted_records_counted(
            records,
            &limits(),
            7,
            &mut |r, s, w| {
                for (total, value) in used.iter_mut().zip([r, s, w]) {
                    *total += value;
                }
                Ok::<_, ()>(())
            },
        )
        .unwrap();
        assert_eq!(counted, legacy);
        for (index, (buffer, pointer)) in counted.buffers().iter().zip(pointers).enumerate() {
            assert_eq!(buffer.utf8().as_ptr(), pointer);
            assert_eq!(buffer.text_id().get(), index as u32);
        }
        assert_eq!(used[0], 30);
        assert_eq!(
            used[1],
            30 * std::mem::size_of::<GeneratedTextBuffer>() as u64
        );
        assert!(used[2] > counted.generated_bytes());
        let empty = GeneratedTextOverlay::try_from_sorted_records_counted(
            vec![],
            &limits(),
            0,
            &mut |_, _, _| Ok::<_, ()>(()),
        )
        .unwrap();
        assert_eq!(
            empty,
            GeneratedTextOverlay::new(vec![], &limits(), 0).unwrap()
        );
    }

    #[test]
    fn counted_overlay_refuses_decreasing_duplicate_and_text_limits_before_allocation() {
        for (records, expected) in [
            (
                vec![(key(2), "b".into()), (key(1), "a".into())],
                GeneratedTextStoreError::UnknownKey,
            ),
            (
                vec![(key(1), "a".into()), (key(1), "b".into())],
                GeneratedTextStoreError::DuplicateKey,
            ),
        ] {
            let mut records_charged = 0;
            let mut storage_charged = 0;
            let error = GeneratedTextOverlay::try_from_sorted_records_counted(
                records,
                &limits(),
                0,
                &mut |r, s, _| {
                    records_charged += r;
                    storage_charged += s;
                    Ok::<_, ()>(())
                },
            )
            .unwrap_err();
            assert_eq!(error, GeneratedOverlayCountedError::Overlay(expected));
            assert_eq!((records_charged, storage_charged), (0, 0));
        }
        let small = ValidatedResourceLimits::new(ResourceLimits {
            max_text_bytes: 10,
            max_text_buffer_bytes: 3,
            max_shaping_context_bytes: 3,
            ..ResourceLimits::default()
        })
        .unwrap();
        for (records, retained) in [
            (vec![(key(1), "four".into())], 0),
            (vec![(key(1), "三".into()), (key(2), "文".into())], 5),
            (vec![], 11),
            (vec![(key(1), "a".into())], u64::MAX),
        ] {
            let legacy = GeneratedTextOverlay::new(records.clone(), &small, retained).unwrap_err();
            let counted = GeneratedTextOverlay::try_from_sorted_records_counted(
                records,
                &small,
                retained,
                &mut |r, s, _| {
                    assert_eq!((r, s), (0, 0));
                    Ok::<_, ()>(())
                },
            )
            .unwrap_err();
            assert_eq!(counted, GeneratedOverlayCountedError::Overlay(legacy));
        }
    }

    #[test]
    fn counted_overlay_every_callback_refusal_keeps_exact_accepted_prefix() {
        let mut requests = Vec::new();
        GeneratedTextOverlay::try_from_sorted_records_counted(
            counted_records(),
            &limits(),
            5,
            &mut |r, s, w| {
                requests.push([r, s, w]);
                Ok::<_, ()>(())
            },
        )
        .unwrap();
        assert_eq!(requests.iter().filter(|r| r[0] != 0).count(), 1);
        assert_eq!(requests.iter().filter(|r| r[1] != 0).count(), 1);
        for fail_at in 0..requests.len() {
            let mut calls = 0;
            let mut accepted = Vec::new();
            let error = GeneratedTextOverlay::try_from_sorted_records_counted(
                counted_records(),
                &limits(),
                5,
                &mut |r, s, w| {
                    let index = calls;
                    calls += 1;
                    if index == fail_at {
                        return Err(index);
                    }
                    accepted.push([r, s, w]);
                    Ok(())
                },
            )
            .unwrap_err();
            assert_eq!(error, GeneratedOverlayCountedError::Budget(fail_at));
            assert_eq!(calls, fail_at + 1);
            assert_eq!(accepted, requests[..fail_at]);
        }
    }

    #[test]
    fn counted_overlay_exact_and_one_short_caps_keep_reservations_atomic() {
        let mut exact = [0; 3];
        GeneratedTextOverlay::try_from_sorted_records_counted(
            counted_records(),
            &limits(),
            0,
            &mut |r, s, w| {
                for (total, value) in exact.iter_mut().zip([r, s, w]) {
                    *total += value;
                }
                Ok::<_, ()>(())
            },
        )
        .unwrap();
        for axis in 0..3 {
            for short in [false, true] {
                let mut caps = exact;
                caps[axis] -= u64::from(short);
                let mut used = [0; 3];
                let result = GeneratedTextOverlay::try_from_sorted_records_counted(
                    counted_records(),
                    &limits(),
                    0,
                    &mut |r, s, w| {
                        let next = [used[0] + r, used[1] + s, used[2] + w];
                        if next.iter().zip(caps).any(|(n, cap)| *n > cap) {
                            return Err(axis);
                        }
                        used = next;
                        Ok(())
                    },
                );
                if short {
                    assert_eq!(
                        result.unwrap_err(),
                        GeneratedOverlayCountedError::Budget(axis)
                    );
                    assert!(used.iter().zip(caps).all(|(n, cap)| *n <= cap));
                } else {
                    assert!(result.is_ok());
                    assert_eq!(used, exact);
                }
            }
        }
    }
}

#[path = "generated_overlay_work.rs"]
mod work;
pub use work::GeneratedOverlayWorkError;
