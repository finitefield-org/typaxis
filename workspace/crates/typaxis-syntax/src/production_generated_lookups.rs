//! Observed borrowed generated text. These lookups confer no source authorization.
use super::*;
use typaxis_text::{GeneratedOverlayWorkError, GeneratedProvenance, GeneratedTextBuffer};

impl<'a, P, N> SourceTextFlow<'a, P, N> {
    fn generated_pair_with_work<'s, E>(
        &'s self,
        buffer: Option<&'s GeneratedTextBuffer>,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Option<(GeneratedProvenance, &'s str)>, E> {
        let Some(buffer) = buffer else {
            return Ok(None);
        };
        work(1)?;
        let text = buffer.utf8();
        let Ok(end) = u32::try_from(text.len()) else {
            return Ok(None);
        };
        match self.generated.try_provenance_with_work(
            buffer.key(),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(end),
            work,
        ) {
            Ok(provenance) => Ok(Some((provenance, text))),
            Err(GeneratedOverlayWorkError::Budget(error)) => Err(error),
            Err(GeneratedOverlayWorkError::Text(_)) => Ok(None),
        }
    }
    pub fn reference_with_work<E>(
        &self,
        owner: NodeId,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Option<(GeneratedProvenance, &str)>, E> {
        let buffer = match self
            .generated
            .try_buffer_with_work(page_reference_key(owner), work)?
        {
            Some(buffer) => Some(buffer),
            None => self
                .generated
                .try_buffer_with_work(text_reference_key(owner), work)?,
        };
        // A present page key keeps priority even if its full span is invalid.
        self.generated_pair_with_work(buffer, work)
    }
    pub fn footnote_marker_with_work<E>(
        &self,
        owner: NodeId,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Option<(GeneratedProvenance, &str)>, E> {
        let buffer = self
            .generated
            .try_buffer_with_work(footnote_marker_key(owner), work)?;
        self.generated_pair_with_work(buffer, work)
    }
    pub fn list_marker_with_work<E>(
        &self,
        index: usize,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Option<(GeneratedProvenance, &str)>, E> {
        work(1)?;
        let Some(item) = self.list_items.get(index) else {
            return Ok(None);
        };
        let buffer = self.generated.try_buffer_with_work(item.key(), work)?;
        self.generated_pair_with_work(buffer, work)
    }
}
