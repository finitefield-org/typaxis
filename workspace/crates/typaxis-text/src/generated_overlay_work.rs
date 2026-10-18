//! Borrowed overlay lookup with work accepted before each search probe.
use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GeneratedOverlayWorkError<E> {
    Budget(E),
    Text(GeneratedTextStoreError),
}
impl GeneratedTextOverlay {
    /// Borrow a canonical buffer after accepting entry and binary-search probes.
    /// Refusal stops before the next probe; no input or output storage is added.
    pub fn try_buffer_with_work<E>(
        &self,
        key: GeneratedBufferKey,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Option<&GeneratedTextBuffer>, E> {
        work(1)?;
        let (mut left, mut right) = (0, self.buffers.len());
        while left < right {
            work(1)?;
            let middle = left + (right - left) / 2;
            match self.buffers[middle].key.cmp(&key) {
                std::cmp::Ordering::Less => left = middle + 1,
                std::cmp::Ordering::Greater => right = middle,
                std::cmp::Ordering::Equal => return Ok(Some(&self.buffers[middle])),
            }
        }
        Ok(None)
    }
    /// Preserve ordinary key/span errors while paying search and fixed UTF-8
    /// span validation before execution. A budget cause is returned unchanged.
    pub fn try_provenance_with_work<E>(
        &self,
        key: GeneratedBufferKey,
        start: Utf8ByteOffset,
        end: Utf8ByteOffset,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<GeneratedProvenance, GeneratedOverlayWorkError<E>> {
        use GeneratedOverlayWorkError as Error;
        let buffer = self
            .try_buffer_with_work(key, work)
            .map_err(Error::Budget)?
            .ok_or(Error::Text(GeneratedTextStoreError::UnknownKey))?;
        work(1).map_err(Error::Budget)?;
        if end.get() as usize > buffer.utf8.len()
            || !buffer.utf8.is_char_boundary(start.get() as usize)
            || !buffer.utf8.is_char_boundary(end.get() as usize)
        {
            return Err(Error::Text(GeneratedTextStoreError::SpanOutOfBounds));
        }
        Ok(GeneratedProvenance {
            buffer_key: key,
            text_span: GeneratedTextSpan::new(buffer.text_id, start, end)
                .ok_or(Error::Text(GeneratedTextStoreError::SpanOutOfBounds))?,
        })
    }
}

#[cfg(test)]
#[path = "generated_overlay_work_tests.rs"]
mod tests;
