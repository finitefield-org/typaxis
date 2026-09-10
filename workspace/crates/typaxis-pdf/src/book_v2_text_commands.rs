//! Actual selected glyph/rule command bytes. This contribution intentionally
//! leaves marked-content ownership and the page coordinate transform to the
//! later complete PDF owner; source ActualText tokens stay separately bound.
use super::*;
use crate::text_encoding;
use typaxis_display_list::book_v2::{
    BookV2BodyDisplay, BookV2BodyPaintIndex as Paint, BookV2FontUseGlyphs, BookV2MathPaint,
};
use typaxis_display_list::ProductionNativeMathPaint as Native;

pub const BOOK_V2_TEXT_COMMANDS_ALGORITHM: &str = "typaxis.book-2-text-commands/1";
#[derive(Clone, Debug)]
pub struct BookV2TextCommand {
    paint: usize,
    slot: Option<usize>,
    usage: Option<usize>,
    page: u32,
    range: Range<usize>,
}
impl BookV2TextCommand {
    pub fn paint_index(&self) -> usize {
        self.paint
    }
    pub fn slot_index(&self) -> Option<usize> {
        self.slot
    }
    pub fn usage_index(&self) -> Option<usize> {
        self.usage
    }
    pub fn page_index(&self) -> u32 {
        self.page
    }
}
pub struct BookV2TextCommands<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'t BookV2FontObjects<'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    commands: Vec<BookV2TextCommand>,
    paints: Vec<Range<usize>>,
    bytes: Vec<u8>,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2TextCommands<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn source(
        &self,
    ) -> &'t BookV2FontObjects<'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn display(&self) -> &'v BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
            .source()
            .source()
            .source()
            .source()
            .selection()
            .display()
    }
    pub fn commands(&self) -> &[BookV2TextCommand] {
        &self.commands
    }
    pub fn for_paint(&self, paint: usize) -> Option<&[BookV2TextCommand]> {
        self.commands.get(self.paints.get(paint)?.clone())
    }
    pub fn command_bytes(&self, command: usize) -> Option<&[u8]> {
        self.bytes.get(self.commands.get(command)?.range.clone())
    }
    /// Borrowed extraction token for this glyph occurrence; no wrapper is
    /// emitted here, so a parent Formula/paragraph can own its replacement.
    pub fn actual_text(&self, command: usize) -> Option<&[u8]> {
        self.source
            .source()
            .actual_text(self.commands.get(command)?.usage?)
    }
    pub fn byte_length(&self) -> usize {
        self.bytes.len()
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn record_charge(&self) -> u64 {
        self.budget.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.budget.spool
    }
    pub fn output_charge(&self) -> u64 {
        self.budget.output
    }
    pub fn work_steps(&self) -> u64 {
        self.budget.work
    }
}
fn count_commands(
    display: &BookV2BodyDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    budget: &mut Budget,
) -> Result<usize, E> {
    let mut count = 0usize;
    for paint in display.paints() {
        budget.step(1)?;
        let n = match *paint {
            Paint::Text(_) | Paint::FootnoteSeparator(_) => 1,
            Paint::Marker(i) => display.markers().draws()[i].clusters().len(),
            Paint::EquationNumber(i) => display.numbers().draws()[i].clusters().len(),
            Paint::Math(i) => match display.math().draws()[i].paint() {
                BookV2MathPaint::Native(n) => n.paints().len(),
                BookV2MathPaint::Vector(_) => 0,
            },
            Paint::Image(_) => 0,
        };
        count = count.checked_add(n).ok_or(E::Records)?;
    }
    Ok(count)
}
fn encode(
    source: &BookV2FontObjects<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    output: &mut Encoder<'_>,
    mut commands: Option<&mut Vec<BookV2TextCommand>>,
    mut paints: Option<&mut Vec<Range<usize>>>,
) -> Result<(), E> {
    let cids = source.source().source();
    let display = cids.source().source().selection().display();
    let mut usage_cursor = 0usize;
    let mut command_cursor = 0usize;
    for (paint_index, paint) in display.paints().iter().enumerate() {
        output.budget.step(1)?;
        let before = command_cursor;
        let (page, slots) = match *paint {
            Paint::Text(i) => (display.text().draws()[i].page_index(), 1),
            Paint::Marker(i) => {
                let d = &display.markers().draws()[i];
                (d.fragment().fragment().page_index(), d.clusters().len())
            }
            Paint::EquationNumber(i) => {
                let d = &display.numbers().draws()[i];
                (d.placement().geometry().page_index(), d.clusters().len())
            }
            Paint::Math(i) => {
                let d = &display.math().draws()[i];
                (
                    d.terminal().page_index(),
                    match d.paint() {
                        BookV2MathPaint::Native(n) => n.paints().len(),
                        BookV2MathPaint::Vector(_) => 0,
                    },
                )
            }
            Paint::FootnoteSeparator(i) => (display.markers().separators()[i].page_index(), 1),
            Paint::Image(_) => (0, 0),
        };
        for slot in 0..slots {
            output.budget.step(1)?;
            let start = output.length;
            let rule = match *paint {
                Paint::FootnoteSeparator(i) => Some(display.markers().separators()[i].ink()),
                Paint::Math(i) => match display.math().draws()[i].paint() {
                    BookV2MathPaint::Native(n) => match n.paints()[slot] {
                        Native::Rule(r) => Some(r),
                        _ => None,
                    },
                    _ => return Err(E::Identity),
                },
                _ => None,
            };
            let usage = if let Some(rect) = rule {
                if cids.uses().get(usage_cursor).is_some_and(|u| {
                    u.source().paint_index() == paint_index && u.source().usage().slot() == slot
                }) {
                    return Err(E::Identity);
                }
                output.extend(b"0 g\n")?;
                text_encoding::rule(output, rect)?;
                None
            } else {
                let cid_use = cids.uses().get(usage_cursor).ok_or(E::Identity)?;
                let selected = cid_use.source();
                let usage = selected.usage();
                if selected.paint_index() != paint_index
                    || usage.slot() != slot
                    || usage.paint() != *paint
                    || source.font_object(cid_use.font_index()).is_none()
                {
                    return Err(E::Identity);
                }
                let actual_cids = cids.cids(usage_cursor).ok_or(E::Identity)?;
                if actual_cids.len() != usage.glyphs().len() {
                    return Err(E::Identity);
                }
                text_encoding::begin(
                    output,
                    usage.instance().font_instance_id().get(),
                    usage.size().get().raw(),
                    true,
                )?;
                match usage.glyphs() {
                    BookV2FontUseGlyphs::Cluster(glyphs) => {
                        for (glyph, cid) in glyphs.iter().zip(actual_cids) {
                            text_encoding::glyph(
                                output,
                                glyph.x().raw(),
                                glyph.y().raw(),
                                cid.get(),
                            )?;
                        }
                    }
                    BookV2FontUseGlyphs::Native(original) => {
                        let Paint::Math(i) = *paint else {
                            return Err(E::Identity);
                        };
                        let BookV2MathPaint::Native(n) = display.math().draws()[i].paint() else {
                            return Err(E::Identity);
                        };
                        let Native::Glyph {
                            original_gid,
                            x,
                            y,
                            font_size,
                            ..
                        } = n.paints()[slot]
                        else {
                            return Err(E::Identity);
                        };
                        if original_gid != original
                            || font_size != usage.size()
                            || actual_cids.len() != 1
                        {
                            return Err(E::Identity);
                        }
                        text_encoding::glyph(output, x.raw(), y.raw(), actual_cids[0].get())?;
                    }
                }
                text_encoding::end(output)?;
                let index = usage_cursor;
                usage_cursor += 1;
                Some(index)
            };
            if let Some(commands) = &mut commands {
                commands.push(BookV2TextCommand {
                    paint: paint_index,
                    slot: (!matches!(paint, Paint::FootnoteSeparator(_))).then_some(slot),
                    usage,
                    page,
                    range: start..output.length,
                });
            }
            command_cursor += 1;
        }
        if let Some(paints) = &mut paints {
            paints.push(before..command_cursor);
        }
    }
    if usage_cursor != cids.uses().len() {
        return Err(E::Identity);
    }
    Ok(())
}
pub struct BookV2TextCommandBuilder<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'t BookV2FontObjects<'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    budget: Budget,
}
impl<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2TextCommandBuilder<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn new(
        source: &'t BookV2FontObjects<'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        let inherited = BookV2FontStreamBuilder::new(
            source.source().source(),
            limits,
            max_work,
            prior_records.max(source.record_charge()),
            prior_spool.max(source.spool_charge()),
            prior_output.max(source.output_charge()),
            prior_work.max(source.work_steps()),
        )?;
        Ok(Self {
            source,
            budget: inherited.budget,
        })
    }
    pub fn record_charge(&self) -> u64 {
        self.budget.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.budget.spool
    }
    pub fn output_charge(&self) -> u64 {
        self.budget.output
    }
    pub fn work_steps(&self) -> u64 {
        self.budget.work
    }
    pub fn build(
        &mut self,
    ) -> Result<BookV2TextCommands<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E>
    {
        let display = self
            .source
            .source()
            .source()
            .source()
            .source()
            .selection()
            .display();
        let count = count_commands(display, &mut self.budget)?;
        let npaints = display.paints().len();
        let records = count
            .checked_add(npaints)
            .and_then(|n| n.checked_add(1))
            .ok_or(E::Records)?;
        let metadata = count
            .checked_mul(std::mem::size_of::<BookV2TextCommand>())
            .and_then(|n| n.checked_add(npaints.checked_mul(std::mem::size_of::<Range<usize>>())?))
            .ok_or(E::Spool)?;
        let output_limit = self.budget.max_output - self.budget.output;
        let spool_limit = (self.budget.max_spool - self.budget.spool)
            .checked_sub(metadata as u64)
            .ok_or(E::Spool)?;
        let mut counter = Encoder {
            budget: &mut self.budget,
            bytes: None,
            length: 0,
            output_limit,
            spool_limit,
        };
        encode(self.source, &mut counter, None, None)?;
        let length = counter.length;
        self.budget.reserve(
            records,
            metadata.checked_add(length).ok_or(E::Spool)?,
            length,
        )?;
        self.budget.step(1)?;
        let mut bytes = Vec::new();
        let mut commands = Vec::new();
        let mut paints = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| E::Allocation)?;
        commands
            .try_reserve_exact(count)
            .map_err(|_| E::Allocation)?;
        paints
            .try_reserve_exact(npaints)
            .map_err(|_| E::Allocation)?;
        let mut encoder = Encoder {
            budget: &mut self.budget,
            bytes: Some(&mut bytes),
            length: 0,
            output_limit: length as u64,
            spool_limit: length as u64,
        };
        encode(
            self.source,
            &mut encoder,
            Some(&mut commands),
            Some(&mut paints),
        )?;
        if encoder.length != length || commands.len() != count || paints.len() != npaints {
            return Err(E::Identity);
        }
        self.budget.step(length.div_ceil(64) + 1)?;
        let mut fingerprint = sha256(BOOK_V2_TEXT_COMMANDS_ALGORITHM.as_bytes());
        fingerprint = self.budget.fold(fingerprint, &self.source.fingerprint())?;
        fingerprint = self.budget.fold(fingerprint, &sha256(&bytes))?;
        for c in &commands {
            let mut b = [0; 46];
            b[..8].copy_from_slice(&(c.paint as u64).to_be_bytes());
            b[8] = u8::from(c.slot.is_some());
            b[9..17].copy_from_slice(&(c.slot.unwrap_or(0) as u64).to_be_bytes());
            b[17] = u8::from(c.usage.is_some());
            b[18..26].copy_from_slice(&(c.usage.unwrap_or(0) as u64).to_be_bytes());
            b[26..30].copy_from_slice(&c.page.to_be_bytes());
            b[30..38].copy_from_slice(&(c.range.start as u64).to_be_bytes());
            b[38..46].copy_from_slice(&(c.range.end as u64).to_be_bytes());
            fingerprint = self.budget.fold(fingerprint, &b)?;
        }
        for range in &paints {
            let mut b = [0; 16];
            b[..8].copy_from_slice(&(range.start as u64).to_be_bytes());
            b[8..].copy_from_slice(&(range.end as u64).to_be_bytes());
            fingerprint = self.budget.fold(fingerprint, &b)?;
        }
        Ok(BookV2TextCommands {
            source: self.source,
            commands,
            paints,
            bytes,
            fingerprint,
            budget: self.budget,
        })
    }
}
