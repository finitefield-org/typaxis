// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Private full-string forward work kernel, preserving unicode-segmentation 1.12.0 rules.
use super::*;

#[derive(Debug)]
enum StepError<E> {
    Budget(E),
    Incomplete(GraphemeIncomplete),
}
fn visit<E>(work: &mut impl FnMut() -> Result<(), E>) -> Result<(), StepError<E>> {
    work().map_err(StepError::Budget)
}

impl<'a> Graphemes<'a> {
    /// Advance one full-string grapheme, accepting work before scalar and rule visits.
    /// On refusal, the forward cursor is restored; accepted caller work is not refunded.
    /// Other segmentation methods do not acquire work accounting through this API.
    pub fn try_next_with_work<E>(
        &mut self,
        work: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Option<&'a str>, E> {
        work()?;
        let start = self.cursor.cur_cursor();
        if start == self.cursor_back.cur_cursor() {
            return Ok(None);
        }
        let original = self.cursor.clone();
        match self.cursor.next_boundary_work(self.string, 0, work) {
            Ok(Some(next)) => Ok(Some(&self.string[start..next])),
            Err(StepError::Budget(error)) => {
                self.cursor = original;
                Err(error)
            }
            Err(StepError::Incomplete(error)) => {
                panic!("inconsistent full-string grapheme context: {:?}", error)
            }
            Ok(None) => unreachable!("nonempty full-string iterator has a next boundary"),
        }
    }
}
impl<'a> GraphemeIndices<'a> {
    /// Fallible forward grapheme/byte-offset iteration; see Graphemes::try_next_with_work.
    pub fn try_next_with_work<E>(
        &mut self,
        work: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Option<(usize, &'a str)>, E> {
        let origin = self.start_offset;
        self.iter
            .try_next_with_work(work)
            .map(|part| part.map(|s| (s.as_ptr() as usize - origin, s)))
    }
}
impl GraphemeCursor {
    fn handle_incb_work<E>(
        &mut self,
        chunk: &str,
        chunk_start: usize,
        work: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<(), StepError<E>> {
        visit(work)?;
        use crate::tables::{self, grapheme as gr};

        // GB9c only applies to extended grapheme clusters
        if !self.is_extended {
            self.decide(true);
            return Ok(());
        }

        let mut incb_linker_count = self.incb_linker_count.unwrap_or(0);

        let mut chars = chunk.chars().rev();
        loop {
            visit(work)?;
            let Some(ch) = chars.next() else { break };
            if tables::is_incb_linker(ch) {
                // We found an InCB linker
                incb_linker_count += 1;
                self.incb_linker_count = Some(incb_linker_count);
            } else if tables::derived_property::InCB_Extend(ch) {
                // We ignore InCB extends, continue
            } else {
                // Prev character is neither linker nor extend, break suppressed iff it's InCB=Consonant
                let result = !(self.incb_linker_count.unwrap_or(0) > 0
                    && self.grapheme_category(ch) == gr::GC_InCB_Consonant);
                self.decide(result);
                return Ok(());
            }
        }

        if chunk_start == 0 {
            // Start of text and we still haven't found a consonant, so break
            self.decide(true);
        } else {
            // We need more context
            self.pre_context_offset = Some(chunk_start);
            self.state = GraphemeState::InCbConsonant;
        }

        Ok(())
    }
    fn handle_regional_work<E>(
        &mut self,
        chunk: &str,
        chunk_start: usize,
        work: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<(), StepError<E>> {
        visit(work)?;
        use crate::tables::grapheme as gr;
        let mut ris_count = self.ris_count.unwrap_or(0);
        let mut chars = chunk.chars().rev();
        loop {
            visit(work)?;
            let Some(ch) = chars.next() else { break };
            if self.grapheme_category(ch) != gr::GC_Regional_Indicator {
                self.ris_count = Some(ris_count);
                self.decide((ris_count & 1) == 0);
                return Ok(());
            }
            ris_count += 1;
        }
        self.ris_count = Some(ris_count);
        if chunk_start == 0 {
            self.decide((ris_count & 1) == 0);
        } else {
            self.pre_context_offset = Some(chunk_start);
            self.state = GraphemeState::Regional;
        }

        Ok(())
    }
    fn handle_emoji_work<E>(
        &mut self,
        chunk: &str,
        chunk_start: usize,
        work: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<(), StepError<E>> {
        visit(work)?;
        use crate::tables::grapheme as gr;
        let mut iter = chunk.chars().rev();
        visit(work)?;
        if let Some(ch) = iter.next() {
            if self.grapheme_category(ch) != gr::GC_ZWJ {
                self.decide(true);
                return Ok(());
            }
        }
        loop {
            visit(work)?;
            let Some(ch) = iter.next() else { break };
            match self.grapheme_category(ch) {
                gr::GC_Extend => (),
                gr::GC_Extended_Pictographic => {
                    self.decide(false);
                    return Ok(());
                }
                _ => {
                    self.decide(true);
                    return Ok(());
                }
            }
        }
        if chunk_start == 0 {
            self.decide(true);
        } else {
            self.pre_context_offset = Some(chunk_start);
            self.state = GraphemeState::Emoji;
        }

        Ok(())
    }
    fn is_boundary_work<E>(
        &mut self,
        chunk: &str,
        chunk_start: usize,
        work: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<bool, StepError<E>> {
        visit(work)?;
        use crate::tables::grapheme as gr;
        if self.state == GraphemeState::Break {
            return Ok(true);
        }
        if self.state == GraphemeState::NotBreak {
            return Ok(false);
        }
        if (self.offset < chunk_start || self.offset >= chunk_start.saturating_add(chunk.len()))
            && (self.offset > chunk_start.saturating_add(chunk.len()) || self.cat_after.is_none())
        {
            return Err(StepError::Incomplete(GraphemeIncomplete::InvalidOffset));
        }
        if let Some(pre_context_offset) = self.pre_context_offset {
            return Err(StepError::Incomplete(GraphemeIncomplete::PreContext(
                pre_context_offset,
            )));
        }
        let offset_in_chunk = self.offset.saturating_sub(chunk_start);
        if self.cat_after.is_none() {
            visit(work)?;
            let ch = chunk[offset_in_chunk..].chars().next().unwrap();
            self.cat_after = Some(self.grapheme_category(ch));
        }
        if self.offset == chunk_start {
            let mut need_pre_context = true;
            match self.cat_after.unwrap() {
                gr::GC_InCB_Consonant => self.state = GraphemeState::InCbConsonant,
                gr::GC_Regional_Indicator => self.state = GraphemeState::Regional,
                gr::GC_Extended_Pictographic => self.state = GraphemeState::Emoji,
                _ => need_pre_context = self.cat_before.is_none(),
            }
            if need_pre_context {
                self.pre_context_offset = Some(chunk_start);
                return Err(StepError::Incomplete(GraphemeIncomplete::PreContext(
                    chunk_start,
                )));
            }
        }
        if self.cat_before.is_none() {
            visit(work)?;
            let ch = chunk[..offset_in_chunk].chars().next_back().unwrap();
            self.cat_before = Some(self.grapheme_category(ch));
        }
        match check_pair(self.cat_before.unwrap(), self.cat_after.unwrap()) {
            PairResult::NotBreak => self.decision(false).map_err(StepError::Incomplete),
            PairResult::Break => self.decision(true).map_err(StepError::Incomplete),
            PairResult::Extended => {
                let is_extended = self.is_extended;
                self.decision(!is_extended).map_err(StepError::Incomplete)
            }
            PairResult::InCbConsonant => {
                self.handle_incb_work(&chunk[..offset_in_chunk], chunk_start, work)?;
                self.is_boundary_result().map_err(StepError::Incomplete)
            }
            PairResult::Regional => {
                if let Some(ris_count) = self.ris_count {
                    return self
                        .decision((ris_count & 1) == 0)
                        .map_err(StepError::Incomplete);
                }
                self.handle_regional_work(&chunk[..offset_in_chunk], chunk_start, work)?;
                self.is_boundary_result().map_err(StepError::Incomplete)
            }
            PairResult::Emoji => {
                self.handle_emoji_work(&chunk[..offset_in_chunk], chunk_start, work)?;
                self.is_boundary_result().map_err(StepError::Incomplete)
            }
        }
    }
    fn next_boundary_work<E>(
        &mut self,
        chunk: &str,
        chunk_start: usize,
        work: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Option<usize>, StepError<E>> {
        visit(work)?;
        if self.offset == self.len {
            return Ok(None);
        }
        let mut iter = chunk[self.offset.saturating_sub(chunk_start)..].chars();
        visit(work)?;
        let mut ch = match iter.next() {
            Some(ch) => ch,
            None => return Err(StepError::Incomplete(GraphemeIncomplete::NextChunk)),
        };
        loop {
            visit(work)?;
            if self.resuming {
                if self.cat_after.is_none() {
                    self.cat_after = Some(self.grapheme_category(ch));
                }
            } else {
                self.offset = self.offset.saturating_add(ch.len_utf8());
                self.state = GraphemeState::Unknown;
                self.cat_before = self.cat_after.take();
                if self.cat_before.is_none() {
                    self.cat_before = Some(self.grapheme_category(ch));
                }
                if crate::tables::is_incb_linker(ch) {
                    self.incb_linker_count = Some(self.incb_linker_count.map_or(1, |c| c + 1));
                } else if !crate::tables::derived_property::InCB_Extend(ch) {
                    self.incb_linker_count = Some(0);
                }
                if self.cat_before.unwrap() == GraphemeCat::GC_Regional_Indicator {
                    self.ris_count = self.ris_count.map(|c| c + 1);
                } else {
                    self.ris_count = Some(0);
                }
                visit(work)?;
                if let Some(next_ch) = iter.next() {
                    ch = next_ch;
                    self.cat_after = Some(self.grapheme_category(ch));
                } else if self.offset == self.len {
                    self.decide(true);
                } else {
                    self.resuming = true;
                    return Err(StepError::Incomplete(GraphemeIncomplete::NextChunk));
                }
            }
            self.resuming = true;
            if self.is_boundary_work(chunk, chunk_start, work)? {
                self.resuming = false;
                return Ok(Some(self.offset));
            }
            self.resuming = false;
        }
    }
}

#[cfg(test)]
#[path = "grapheme_work_tests.rs"]
mod tests;
