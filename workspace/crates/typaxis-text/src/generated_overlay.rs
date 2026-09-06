//! A bounded generated namespace for an explicitly selected semantic stage.
//! This is not a complete document GeneratedTextStore or a source authorization.
//! The syntax owner must derive and verify every key/text pair against its source.
use super::*;

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
        if records.len() as u64 > limits.get().max_fragments {
            return Err(E::ResourceLimit);
        }
        let total = records
            .iter()
            .try_fold(retained_text_bytes, |used, (_, text)| {
                if text.len() as u64 > u64::from(limits.get().max_text_buffer_bytes) {
                    return None;
                }
                used.checked_add(text.len() as u64)
            })
            .filter(|n| *n <= limits.get().max_text_bytes)
            .ok_or(E::ResourceLimit)?;
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
            buffers.push(GeneratedTextBuffer {
                key,
                utf8,
                text_id: GeneratedTextBufferId::new(
                    u32::try_from(index).map_err(|_| E::TooManyBuffers)?,
                ),
            });
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
}
