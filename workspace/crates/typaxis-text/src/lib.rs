#![forbid(unsafe_code)]

use typaxis_core::{
    generated_text_reference_fingerprint, sha256, GeneratedBufferKey, GeneratedTextBufferId,
    GeneratedTextSpan, PortablePath, ReferenceFingerprint, SourceId, SourceSpan, TextBufferId,
    Utf8ByteOffset, Utf8ByteRange, ValidatedResourceLimits,
};
use typaxis_document::ValidatedDocumentNodeIndex;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRecord {
    source_id: SourceId,
    uri: PortablePath,
    utf8: String,
    utf8_byte_length: u32,
    sha256: [u8; 32],
}
impl SourceRecord {
    pub fn new(
        source_id: SourceId,
        uri: PortablePath,
        utf8: String,
    ) -> Result<Self, SourceCatalogError> {
        let utf8_byte_length =
            u32::try_from(utf8.len()).map_err(|_| SourceCatalogError::ByteLengthMismatch)?;
        let sha256 = sha256(utf8.as_bytes());
        Ok(Self {
            source_id,
            uri,
            utf8,
            utf8_byte_length,
            sha256,
        })
    }
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub fn utf8(&self) -> &str {
        &self.utf8
    }
    pub const fn utf8_byte_length(&self) -> u32 {
        self.utf8_byte_length
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceCatalogError {
    DuplicateSourceId,
    DuplicateSourceUri,
    ByteLengthMismatch,
    NonDenseSourceId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceCatalog {
    records: Vec<SourceRecord>,
}
impl SourceCatalog {
    pub fn new(records: Vec<SourceRecord>) -> Result<Self, SourceCatalogError> {
        use std::collections::BTreeSet;
        let mut ids = BTreeSet::new();
        let mut uris = BTreeSet::new();
        for (index, record) in records.iter().enumerate() {
            if !ids.insert(record.source_id()) {
                return Err(SourceCatalogError::DuplicateSourceId);
            }
            if record.source_id().get()
                != u32::try_from(index).map_err(|_| SourceCatalogError::NonDenseSourceId)?
            {
                return Err(SourceCatalogError::NonDenseSourceId);
            }
            if u32::try_from(record.utf8().len()).ok() != Some(record.utf8_byte_length())
                || sha256(record.utf8().as_bytes()) != record.content_hash()
            {
                return Err(SourceCatalogError::ByteLengthMismatch);
            }
            if !uris.insert(record.uri().clone()) {
                return Err(SourceCatalogError::DuplicateSourceUri);
            }
        }
        Ok(Self { records })
    }

    pub fn records(&self) -> &[SourceRecord] {
        &self.records
    }
    pub fn get(&self, source_id: SourceId) -> Option<&SourceRecord> {
        self.records
            .iter()
            .find(|record| record.source_id() == source_id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextMapKind {
    Identity,
    Replacement,
    Inserted,
}

/// `text_range` is local to the owning TextBuffer. It deliberately carries no
/// TextBufferId, preventing a segment from naming a different buffer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextMapSegment {
    pub text_range: Utf8ByteRange,
    pub kind: TextMapKind,
    pub source_span: Option<SourceSpan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TextBufferError {
    TooLarge,
    EmptyMappingForNonEmptyBuffer,
    EmptyMappingSegment,
    MappingGapOrOverlap,
    MappingNotUtf8Boundary,
    MissingSourceForMappedSegment,
    InsertedSegmentHasSource,
    IdentityLengthMismatch,
    DuplicateTextBufferId,
    NonDenseTextBufferId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextBuffer {
    text_id: TextBufferId,
    text: String,
    mappings: Vec<TextMapSegment>,
    byte_len: u32,
}
impl TextBuffer {
    pub fn new(
        text_id: TextBufferId,
        text: String,
        mappings: Vec<TextMapSegment>,
        max_bytes: u32,
    ) -> Result<Self, TextBufferError> {
        let byte_len = u32::try_from(text.len()).map_err(|_| TextBufferError::TooLarge)?;
        if byte_len > max_bytes {
            return Err(TextBufferError::TooLarge);
        }
        if byte_len > 0 && mappings.is_empty() {
            return Err(TextBufferError::EmptyMappingForNonEmptyBuffer);
        }
        let mut cursor = 0u32;
        for segment in &mappings {
            if segment.text_range.is_empty() {
                return Err(TextBufferError::EmptyMappingSegment);
            }
            if segment.text_range.start_byte().get() != cursor {
                return Err(TextBufferError::MappingGapOrOverlap);
            }
            let end = segment.text_range.end_byte().get();
            if end > byte_len
                || !text.is_char_boundary(cursor as usize)
                || !text.is_char_boundary(end as usize)
            {
                return Err(TextBufferError::MappingNotUtf8Boundary);
            }
            match (segment.kind, segment.source_span) {
                (TextMapKind::Inserted, Some(_)) => {
                    return Err(TextBufferError::InsertedSegmentHasSource)
                }
                (TextMapKind::Identity | TextMapKind::Replacement, None) => {
                    return Err(TextBufferError::MissingSourceForMappedSegment)
                }
                (TextMapKind::Identity, Some(source_span))
                    if segment.text_range.len() != source_span.range().len() =>
                {
                    return Err(TextBufferError::IdentityLengthMismatch)
                }
                _ => {}
            }
            cursor = end;
        }
        if cursor != byte_len {
            return Err(TextBufferError::MappingGapOrOverlap);
        }
        Ok(Self {
            text_id,
            text,
            mappings,
            byte_len,
        })
    }
    pub const fn text_id(&self) -> TextBufferId {
        self.text_id
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn mappings(&self) -> &[TextMapSegment] {
        &self.mappings
    }
    pub const fn byte_len(&self) -> u32 {
        self.byte_len
    }
    pub fn is_boundary(&self, offset: Utf8ByteOffset) -> bool {
        offset.get() <= self.byte_len && self.text.is_char_boundary(offset.get() as usize)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextStore {
    buffers: Vec<TextBuffer>,
}
impl TextStore {
    pub fn new(buffers: Vec<TextBuffer>) -> Result<Self, TextBufferError> {
        use std::collections::BTreeSet;
        let mut ids = BTreeSet::new();
        for (index, buffer) in buffers.iter().enumerate() {
            if !ids.insert(buffer.text_id()) {
                return Err(TextBufferError::DuplicateTextBufferId);
            }
            if buffer.text_id().get()
                != u32::try_from(index).map_err(|_| TextBufferError::NonDenseTextBufferId)?
            {
                return Err(TextBufferError::NonDenseTextBufferId);
            }
        }
        Ok(Self { buffers })
    }

    pub fn buffers(&self) -> &[TextBuffer] {
        &self.buffers
    }
    pub fn get(&self, text_id: TextBufferId) -> Option<&TextBuffer> {
        self.buffers
            .iter()
            .find(|buffer| buffer.text_id() == text_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedBufferDraft {
    key: GeneratedBufferKey,
    utf8: String,
}
impl GeneratedBufferDraft {
    pub fn new(
        document_nodes: &ValidatedDocumentNodeIndex,
        key: GeneratedBufferKey,
        utf8: String,
    ) -> Result<Self, GeneratedTextStoreError> {
        if document_nodes.generated_site(key).is_none() {
            return Err(GeneratedTextStoreError::UnknownGeneratedSite);
        }
        Ok(Self { key, utf8 })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedTextBuffer {
    text_id: GeneratedTextBufferId,
    key: GeneratedBufferKey,
    utf8: String,
}

/// Store-issued provenance. Its private fields prevent combining a logical key
/// with an allocation-derived buffer ID from another GeneratedTextStore.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GeneratedProvenance {
    buffer_key: GeneratedBufferKey,
    text_span: GeneratedTextSpan,
}
impl GeneratedProvenance {
    pub const fn buffer_key(self) -> GeneratedBufferKey {
        self.buffer_key
    }
    pub const fn text_span(self) -> GeneratedTextSpan {
        self.text_span
    }
}
impl GeneratedTextBuffer {
    pub const fn text_id(&self) -> GeneratedTextBufferId {
        self.text_id
    }
    pub const fn key(&self) -> GeneratedBufferKey {
        self.key
    }
    pub fn utf8(&self) -> &str {
        &self.utf8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneratedTextStoreError {
    DuplicateKey,
    TooManyBuffers,
    UnknownKey,
    SpanOutOfBounds,
    ResourceLimit,
    UnknownGeneratedSite,
    MissingGeneratedSite,
}

/// Finalized generated buffers. IDs are derived from canonical key order and
/// therefore never depend on allocation or insertion order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedTextStore {
    reference_fingerprint: ReferenceFingerprint,
    buffers: Vec<GeneratedTextBuffer>,
    ids: std::collections::BTreeMap<GeneratedBufferKey, GeneratedTextBufferId>,
    document_nodes: ValidatedDocumentNodeIndex,
}
impl GeneratedTextStore {
    pub fn new(
        drafts: Vec<GeneratedBufferDraft>,
        document_nodes: &ValidatedDocumentNodeIndex,
        limits: &ValidatedResourceLimits,
        parsed_store: &TextStore,
    ) -> Result<Self, GeneratedTextStoreError> {
        let mut canonical = std::collections::BTreeMap::new();
        for draft in drafts {
            if document_nodes.generated_site(draft.key).is_none() {
                return Err(GeneratedTextStoreError::UnknownGeneratedSite);
            }
            if canonical.insert(draft.key, draft.utf8).is_some() {
                return Err(GeneratedTextStoreError::DuplicateKey);
            }
        }
        if canonical.len() != document_nodes.generated_sites().len()
            || document_nodes
                .generated_sites()
                .any(|site| !canonical.contains_key(&site.key()))
        {
            return Err(GeneratedTextStoreError::MissingGeneratedSite);
        }
        let limits = limits.get();
        let mut total_bytes = parsed_store
            .buffers()
            .iter()
            .try_fold(0u64, |total, buffer| {
                if buffer.byte_len() > limits.max_text_buffer_bytes {
                    return None;
                }
                total.checked_add(u64::from(buffer.byte_len()))
            })
            .ok_or(GeneratedTextStoreError::ResourceLimit)?;
        for utf8 in canonical.values() {
            let bytes =
                u64::try_from(utf8.len()).map_err(|_| GeneratedTextStoreError::ResourceLimit)?;
            if bytes > u64::from(limits.max_text_buffer_bytes) {
                return Err(GeneratedTextStoreError::ResourceLimit);
            }
            total_bytes = total_bytes
                .checked_add(bytes)
                .ok_or(GeneratedTextStoreError::ResourceLimit)?;
        }
        if total_bytes > limits.max_text_bytes {
            return Err(GeneratedTextStoreError::ResourceLimit);
        }
        let fingerprint_records: Vec<_> = canonical
            .iter()
            .map(|(key, utf8)| (*key, utf8.clone()))
            .collect();
        let reference_fingerprint = generated_text_reference_fingerprint(&fingerprint_records);
        let mut buffers = Vec::with_capacity(canonical.len());
        let mut ids = std::collections::BTreeMap::new();
        for (index, (key, utf8)) in canonical.into_iter().enumerate() {
            let text_id = GeneratedTextBufferId::new(
                u32::try_from(index).map_err(|_| GeneratedTextStoreError::TooManyBuffers)?,
            );
            ids.insert(key, text_id);
            buffers.push(GeneratedTextBuffer { text_id, key, utf8 });
        }
        Ok(Self {
            reference_fingerprint,
            buffers,
            ids,
            document_nodes: document_nodes.clone(),
        })
    }
    pub const fn reference_fingerprint(&self) -> ReferenceFingerprint {
        self.reference_fingerprint
    }
    pub fn buffers(&self) -> &[GeneratedTextBuffer] {
        &self.buffers
    }
    pub const fn document_nodes(&self) -> &ValidatedDocumentNodeIndex {
        &self.document_nodes
    }
    pub fn get(&self, text_id: GeneratedTextBufferId) -> Option<&GeneratedTextBuffer> {
        self.buffers
            .get(text_id.get() as usize)
            .filter(|buffer| buffer.text_id() == text_id)
    }
    pub fn provenance(
        &self,
        key: GeneratedBufferKey,
        start: Utf8ByteOffset,
        end: Utf8ByteOffset,
    ) -> Result<GeneratedProvenance, GeneratedTextStoreError> {
        let text_id = *self
            .ids
            .get(&key)
            .ok_or(GeneratedTextStoreError::UnknownKey)?;
        let buffer = self
            .get(text_id)
            .ok_or(GeneratedTextStoreError::UnknownKey)?;
        if end.get() as usize > buffer.utf8.len()
            || !buffer.utf8.is_char_boundary(start.get() as usize)
            || !buffer.utf8.is_char_boundary(end.get() as usize)
        {
            return Err(GeneratedTextStoreError::SpanOutOfBounds);
        }
        let text_span = GeneratedTextSpan::new(text_id, start, end)
            .ok_or(GeneratedTextStoreError::SpanOutOfBounds)?;
        Ok(GeneratedProvenance {
            buffer_key: key,
            text_span,
        })
    }
    pub fn validates_provenance(&self, provenance: GeneratedProvenance) -> bool {
        let span = provenance.text_span();
        let Some(expected_id) = self.ids.get(&provenance.buffer_key()) else {
            return false;
        };
        let Some(buffer) = self.get(span.text_id()) else {
            return false;
        };
        let start = span.range().start_byte().get() as usize;
        let end = span.range().end_byte().get() as usize;
        *expected_id == span.text_id()
            && end <= buffer.utf8().len()
            && buffer.utf8().is_char_boundary(start)
            && buffer.utf8().is_char_boundary(end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{
        AnchorId, GenerationKind, NodeId, ResourceLimits, SourceId, TextBufferId, Utf8ByteOffset,
    };
    use typaxis_document::{Block, Document, Inline, ReferenceFormat, ValidatedDocumentNodeIndex};

    fn generated_index() -> ValidatedDocumentNodeIndex {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        ValidatedDocumentNodeIndex::new(&Document {
            node_id: NodeId::new(0),
            blocks: vec![Block::Paragraph {
                node_id: NodeId::new(1),
                span,
                classes: vec![],
                children: vec![
                    Inline::Reference {
                        node_id: NodeId::new(2),
                        span,
                        target: AnchorId::new("page").unwrap(),
                        format: ReferenceFormat::Page,
                    },
                    Inline::Reference {
                        node_id: NodeId::new(3),
                        span,
                        target: AnchorId::new("number").unwrap(),
                        format: ReferenceFormat::Number,
                    },
                ],
            }],
            footnotes: vec![],
        })
        .unwrap()
    }

    #[test]
    fn mapping_is_local_and_must_cover_buffer() {
        let range = Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(3)).unwrap();
        let source = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(3),
        )
        .unwrap();
        let buffer = TextBuffer::new(
            TextBufferId::new(0),
            "日".to_owned(),
            vec![TextMapSegment {
                text_range: range,
                kind: TextMapKind::Identity,
                source_span: Some(source),
            }],
            3,
        )
        .unwrap();
        assert_eq!(buffer.byte_len(), 3);
    }

    #[test]
    fn empty_mapping_segments_are_rejected() {
        let range = Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(0)).unwrap();
        let error = TextBuffer::new(
            TextBufferId::new(0),
            String::new(),
            vec![TextMapSegment {
                text_range: range,
                kind: TextMapKind::Inserted,
                source_span: None,
            }],
            0,
        );
        assert_eq!(error, Err(TextBufferError::EmptyMappingSegment));
    }

    #[test]
    fn identity_mapping_requires_equal_range_lengths() {
        let text_range =
            Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(3)).unwrap();
        let source_span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(2),
        )
        .unwrap();
        let error = TextBuffer::new(
            TextBufferId::new(0),
            "日".to_owned(),
            vec![TextMapSegment {
                text_range,
                kind: TextMapKind::Identity,
                source_span: Some(source_span),
            }],
            3,
        );
        assert_eq!(error, Err(TextBufferError::IdentityLengthMismatch));
    }

    #[test]
    fn stores_reject_duplicate_ids() {
        let uri = PortablePath::new("input.tsf").unwrap();
        let source = SourceRecord::new(SourceId::new(0), uri, String::new()).unwrap();
        assert_eq!(
            SourceCatalog::new(vec![source.clone(), source]),
            Err(SourceCatalogError::DuplicateSourceId)
        );

        let first = TextBuffer::new(TextBufferId::new(0), String::new(), vec![], 0).unwrap();
        let second = TextBuffer::new(TextBufferId::new(0), String::new(), vec![], 0).unwrap();
        assert_eq!(
            TextStore::new(vec![first, second]),
            Err(TextBufferError::DuplicateTextBufferId)
        );
    }

    #[test]
    fn generated_ids_are_derived_from_key_order_not_insertion_order() {
        let index = generated_index();
        let first = GeneratedBufferDraft::new(
            &index,
            GeneratedBufferKey::new(NodeId::new(3), GenerationKind::Counter, 0),
            "2".to_owned(),
        )
        .unwrap();
        let second = GeneratedBufferDraft::new(
            &index,
            GeneratedBufferKey::new(NodeId::new(2), GenerationKind::PageReference, 0),
            "1".to_owned(),
        )
        .unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let parsed = TextStore::new(vec![]).unwrap();
        let forward = GeneratedTextStore::new(
            vec![first.clone(), second.clone()],
            &index,
            &limits,
            &parsed,
        )
        .unwrap();
        let reverse =
            GeneratedTextStore::new(vec![second, first], &index, &limits, &parsed).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(
            forward.reference_fingerprint(),
            reverse.reference_fingerprint()
        );
        assert_eq!(forward.buffers()[0].text_id().get(), 0);
        assert_eq!(forward.buffers()[0].key().owner().get(), 2);

        assert_eq!(
            GeneratedBufferDraft::new(
                &index,
                GeneratedBufferKey::new(NodeId::new(1), GenerationKind::Counter, 0),
                "forged".to_owned(),
            ),
            Err(GeneratedTextStoreError::UnknownGeneratedSite)
        );
    }
}
