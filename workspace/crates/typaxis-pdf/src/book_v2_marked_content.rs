//! Complete actual marked body commands in source-selected page order.
//! MediaBox/global page transform, resources and the structure tree are bound
//! by the final page/object owner; no new layout coordinates are invented here.
use super::*;
use crate::{semantic_anchor_encoding, ObjectId};
use typaxis_core::{Length, Rect};
use typaxis_display_list::book_v2::{BookV2BodyDisplay, BookV2BodyPaintIndex as Paint};

pub const BOOK_V2_MARKED_CONTENT_ALGORITHM: &str = "typaxis.book-2-marked-content/1";
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookV2MarkedPage {
    page: u32,
    groups: Range<usize>,
    bytes: Range<usize>,
}
impl BookV2MarkedPage {
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn groups(&self) -> Range<usize> {
        self.groups.clone()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2SemanticAnchor {
    page: u32,
    group: usize,
    paint: usize,
    viewport: Rect,
    baseline: Length,
}
impl BookV2SemanticAnchor {
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn group_index(&self) -> usize {
        self.group
    }
    pub fn paint_index(&self) -> usize {
        self.paint
    }
    pub fn viewport(&self) -> Rect {
        self.viewport
    }
    pub fn baseline(&self) -> Length {
        self.baseline
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2SemanticAnchorRole {
    Font,
    Glyph,
    ToUnicode,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookV2SemanticAnchorObject {
    id: ObjectId,
    role: BookV2SemanticAnchorRole,
    bytes: Range<usize>,
}
impl BookV2SemanticAnchorObject {
    pub fn id(&self) -> ObjectId {
        self.id
    }
    pub fn role(&self) -> BookV2SemanticAnchorRole {
        self.role
    }
    pub fn byte_range(&self) -> Range<usize> {
        self.bytes.clone()
    }
}
fn encode_anchor_objects(
    out: &mut Encoder<'_>,
    first: u32,
    needed: bool,
    mut objects: Option<&mut Vec<BookV2SemanticAnchorObject>>,
) -> Result<(), E> {
    if !needed {
        return Ok(());
    }
    let base = out.length;
    for (offset, role) in [
        BookV2SemanticAnchorRole::Font,
        BookV2SemanticAnchorRole::Glyph,
        BookV2SemanticAnchorRole::ToUnicode,
    ]
    .into_iter()
    .enumerate()
    {
        out.budget.step(1)?;
        let id =
            ObjectId::new(first.checked_add(offset as u32).ok_or(E::Objects)?).ok_or(E::Objects)?;
        let start = out.length - base;
        out.unsigned(u64::from(id.get()))?;
        out.extend(b" 0 obj\n")?;
        if role == BookV2SemanticAnchorRole::Font {
            semantic_anchor_encoding::font(out, b"BMA", |out, r| {
                let offset = match r {
                    semantic_anchor_encoding::Reference::Glyph => 1,
                    semantic_anchor_encoding::Reference::ToUnicode => 2,
                };
                out.unsigned(u64::from(first.checked_add(offset).ok_or(E::Objects)?))?;
                out.extend(b" 0 R")
            })?;
        } else {
            let payload = if role == BookV2SemanticAnchorRole::Glyph {
                semantic_anchor_encoding::GLYPH
            } else {
                semantic_anchor_encoding::TO_UNICODE
            };
            out.extend(b"<< /Length ")?;
            out.unsigned(payload.len() as u64)?;
            out.extend(b" >>\nstream\n")?;
            out.extend(payload)?;
            out.extend(b"\nendstream")?;
        }
        out.extend(b"\nendobj\n")?;
        if let Some(objects) = &mut objects {
            objects.push(BookV2SemanticAnchorObject {
                id,
                role,
                bytes: start..out.length - base,
            });
        }
    }
    Ok(())
}
fn anchor(
    display: &BookV2BodyDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    group: usize,
    g: &BookV2MarkedScope,
    replaced: bool,
) -> Result<Option<BookV2SemanticAnchor>, E> {
    if !replaced {
        return Ok(None);
    }
    if g.paints().len() != 1 {
        return Err(E::Identity);
    }
    let paint = g.paints().start;
    let Some(usage) = display.image_use(paint).map_err(|_| E::Identity)? else {
        return Ok(None);
    };
    let baseline = match display.paints()[paint] {
        Paint::Math(i) => display.math().draws()[i].terminal().baseline(),
        Paint::Image(i) => display.images().draws()[i]
            .fragment()
            .fragment()
            .baseline()
            .ok_or(E::Identity)?,
        _ => return Err(E::Identity),
    };
    Ok(Some(BookV2SemanticAnchor {
        page: g.page_index(),
        group,
        paint,
        viewport: usage.geometry().viewport(),
        baseline,
    }))
}
fn replacement<S: Sink<Error = E>>(
    display: &BookV2BodyDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    g: &BookV2MarkedScope,
    out: &mut S,
) -> Result<(), E> {
    out.extend(b"/Span << /ActualText ")?;
    match g.role() {
        BookV2MarkedRole::Text => font_encoding::utf16(
            out,
            g.paints().flat_map(|paint| {
                let Paint::Text(i) = display.paints()[paint] else {
                    unreachable!("sealed text group")
                };
                display.text().draws()[i].exact_text().chars()
            }),
            true,
        )?,
        BookV2MarkedRole::Label => {
            let Paint::Marker(i) = display.paints()[g.paints().start] else {
                return Err(E::Identity);
            };
            font_encoding::utf16(
                out,
                display.markers().draws()[i].source().utf8().chars(),
                true,
            )?;
        }
        BookV2MarkedRole::EquationNumber => {
            let Paint::EquationNumber(i) = display.paints()[g.paints().start] else {
                return Err(E::Identity);
            };
            font_encoding::utf16(
                out,
                display.numbers().draws()[i].shape().text().chars(),
                true,
            )?;
        }
        _ => return Err(E::Identity),
    }
    out.extend(b" >> BDC\n")
}
fn encode(
    scopes: &BookV2MarkedScopes<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    text: &BookV2TextCommands<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    out: &mut Encoder<'_>,
    mut pages: Option<&mut Vec<BookV2MarkedPage>>,
    mut groups: Option<&mut Vec<Range<usize>>>,
    mut anchors: Option<&mut Vec<BookV2SemanticAnchor>>,
) -> Result<usize, E> {
    let display = scopes.display();
    let images = scopes.source();
    let mut text_cursor = 0;
    let mut image_cursor = 0;
    let mut group_cursor = 0;
    let mut paint_cursor = 0;
    let mut anchor_count = 0;
    for page in 0..display.source().source().geometry().pages().len() {
        out.budget.step(1)?;
        let page_start = out.length;
        let group_start = group_cursor;
        out.extend(b"q\n")?;
        for g in scopes.page_groups(page as u32).ok_or(E::Identity)? {
            out.budget.step(1)?;
            if g.paints().start != paint_cursor || g.page_index() != page as u32 {
                return Err(E::Identity);
            }
            let start = out.length;
            out.extend(b"q\n")?;
            out.extend(scopes.begin_bytes(group_cursor).ok_or(E::Identity)?)?;
            let replaced = scopes.actual_text(group_cursor).is_some();
            let inline_replacement = g.artifact().is_none()
                && matches!(
                    g.role(),
                    BookV2MarkedRole::Text
                        | BookV2MarkedRole::Label
                        | BookV2MarkedRole::EquationNumber
                );
            if inline_replacement {
                if replaced {
                    return Err(E::Identity);
                }
                out.budget.step(g.paints().len())?;
                replacement(display, g, out)?;
            }
            if let Some(a) = anchor(display, group_cursor, g, replaced)? {
                out.budget.step(1)?;
                semantic_anchor_encoding::command(out, b"BMA", a.viewport, a.baseline)?;
                anchor_count += 1;
                if let Some(anchors) = &mut anchors {
                    anchors.push(a);
                }
            }
            for paint in g.paints() {
                out.budget.step(1)?;
                let commands = text.for_paint(paint).ok_or(E::Identity)?;
                let mut has_text = false;
                for c in commands {
                    out.budget.step(1)?;
                    if !std::ptr::eq(text.commands().get(text_cursor).ok_or(E::Identity)?, c)
                        || c.paint_index() != paint
                        || c.page_index() != page as u32
                    {
                        return Err(E::Identity);
                    }
                    // Whole selected text/label/number or parent Formula owns
                    // extraction; CID-local replacements must not nest in it.
                    if c.usage_index().is_some()
                        && !(inline_replacement || replaced || g.artifact().is_some())
                    {
                        return Err(E::Identity);
                    }
                    out.extend(text.command_bytes(text_cursor).ok_or(E::Identity)?)?;
                    text_cursor += 1;
                    has_text = true;
                }
                let c = images.commands().get(image_cursor);
                if c.is_some_and(|c| c.paint_index() < paint) {
                    return Err(E::Identity);
                }
                if let Some(c) = c.filter(|c| c.paint_index() == paint) {
                    if has_text || c.page_index() != page as u32 || Some(c.owner()) != g.owner() {
                        return Err(E::Identity);
                    }
                    out.extend(images.command_bytes(image_cursor).ok_or(E::Identity)?)?;
                    // The SVG placement kernel intentionally has no trailing LF.
                    out.extend(b"\n")?;
                    image_cursor += 1;
                }
                paint_cursor += 1;
            }
            if inline_replacement {
                out.extend(b"EMC\n")?;
            }
            out.extend(scopes.end_bytes(group_cursor).ok_or(E::Identity)?)?;
            out.extend(b"Q\n")?;
            if let Some(groups) = &mut groups {
                groups.push(start..out.length);
            }
            group_cursor += 1;
        }
        out.extend(b"Q\n")?;
        if let Some(pages) = &mut pages {
            pages.push(BookV2MarkedPage {
                page: page as u32,
                groups: group_start..group_cursor,
                bytes: page_start..out.length,
            });
        }
    }
    if group_cursor != scopes.groups().len()
        || paint_cursor != display.paints().len()
        || text_cursor != text.commands().len()
        || image_cursor != images.commands().len()
    {
        return Err(E::Identity);
    }
    Ok(anchor_count)
}

pub struct BookV2MarkedContent<
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> {
    source: &'n BookV2MarkedScopes<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    text: &'n BookV2TextCommands<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    pages: Vec<BookV2MarkedPage>,
    groups: Vec<Range<usize>>,
    anchors: Vec<BookV2SemanticAnchor>,
    anchor_objects: Vec<BookV2SemanticAnchorObject>,
    body_length: usize,
    bytes: Vec<u8>,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2MarkedContent<
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >
{
    pub fn source(
        &self,
    ) -> &'n BookV2MarkedScopes<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn text(
        &self,
    ) -> &'n BookV2TextCommands<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.text
    }
    pub fn pages(&self) -> &[BookV2MarkedPage] {
        &self.pages
    }
    pub fn page_bytes(&self, page: usize) -> Option<&[u8]> {
        self.bytes.get(self.pages.get(page)?.bytes.clone())
    }
    pub fn group_bytes(&self, group: usize) -> Option<&[u8]> {
        self.bytes.get(self.groups.get(group)?.clone())
    }
    pub fn anchors(&self) -> &[BookV2SemanticAnchor] {
        &self.anchors
    }
    pub fn anchor_objects(&self) -> &[BookV2SemanticAnchorObject] {
        &self.anchor_objects
    }
    pub fn anchor_object_bytes(&self) -> &[u8] {
        &self.bytes[self.body_length..]
    }
    pub fn anchor_font_object(&self) -> Option<ObjectId> {
        self.anchor_objects.first().map(|o| o.id)
    }
    pub fn next_object(&self) -> u32 {
        self.text
            .source()
            .next_object()
            .max(self.source.source().source().next_object())
            + self.anchor_objects.len() as u32
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
    pub fn byte_length(&self) -> usize {
        self.body_length
    }
}
pub struct BookV2MarkedContentBuilder<
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> {
    source: &'n BookV2MarkedScopes<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    text: &'n BookV2TextCommands<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    budget: Budget,
    credit_available: bool,
    max_objects: u32,
}
impl<'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2MarkedContentBuilder<
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >
{
    pub fn new(
        source: &'n BookV2MarkedScopes<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        text: &'n BookV2TextCommands<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        let display = source.display();
        if !std::ptr::eq(display, text.display()) {
            return Err(E::Identity);
        }
        let images = source.source().source();
        let programs = images.source();
        let prefix = programs.source().source().prior_charges();
        if prefix[0] < text.record_charge()
            || prefix[1] < text.spool_charge()
            || prefix[2] < text.work_steps()
            || programs.prior_output_charge() < text.output_charge()
        {
            return Err(E::Identity);
        }
        let fonts = text.source();
        if !fonts.objects().is_empty()
            && !images.objects().is_empty()
            && fonts.first_object() < images.next_object()
            && images.first_object() < fonts.next_object()
        {
            return Err(E::Objects);
        }
        let inherited = BookV2MarkedScopeBuilder::new(
            source.source(),
            limits,
            max_work,
            prior_records.max(source.record_charge()),
            prior_spool.max(source.spool_charge()),
            prior_output.max(source.output_charge()),
            prior_work.max(source.work_steps()),
        )?;
        Ok(Self {
            source,
            text,
            budget: inherited.budget,
            credit_available: true,
            max_objects: limits.base().get().max_pdf_objects,
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
    ) -> Result<
        BookV2MarkedContent<
            'n,
            'm,
            'k,
            'j,
            'r,
            'i,
            't,
            'o,
            'z,
            'y,
            'x,
            'c,
            'v,
            'd,
            'g,
            'q,
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        E,
    > {
        let mut credit = self
            .text
            .byte_length()
            .checked_add(self.source.source().bytes().len())
            .ok_or(E::Output)?;
        let mut anchor_count = 0usize;
        for (i, g) in self.source.groups().iter().enumerate() {
            self.budget.step(1)?;
            credit = credit
                .checked_add(self.source.begin_bytes(i).ok_or(E::Identity)?.len())
                .and_then(|n| n.checked_add(self.source.end_bytes(i)?.len()))
                .ok_or(E::Output)?;
            if anchor(
                self.source.display(),
                i,
                g,
                self.source.actual_text(i).is_some(),
            )?
            .is_some()
            {
                anchor_count = anchor_count.checked_add(1).ok_or(E::Records)?;
            }
        }
        for i in 0..self.text.commands().len() {
            self.budget.step(1)?;
            credit = credit
                .checked_add(self.text.actual_text(i).map_or(0, |b| b.len()))
                .ok_or(E::Output)?;
        }
        let credit = if self.credit_available { credit } else { 0 };
        let charged = self
            .budget
            .output
            .checked_sub(credit as u64)
            .ok_or(E::Identity)?;
        let npages = self
            .source
            .display()
            .source()
            .source()
            .geometry()
            .pages()
            .len();
        let ngroups = self.source.groups().len();
        let first_anchor = self
            .text
            .source()
            .next_object()
            .max(self.source.source().source().next_object());
        let object_count = if anchor_count == 0 { 0usize } else { 3usize };
        let next_object = first_anchor
            .checked_add(object_count as u32)
            .ok_or(E::Objects)?;
        if object_count != 0 && next_object - 1 > self.max_objects {
            return Err(E::Objects);
        }

        let records = npages
            .checked_add(ngroups)
            .and_then(|n| n.checked_add(object_count))
            .and_then(|n| n.checked_add(anchor_count))
            .and_then(|n| n.checked_add(1))
            .ok_or(E::Records)?;
        let metadata = object_count
            .checked_mul(std::mem::size_of::<BookV2SemanticAnchorObject>())
            .ok_or(E::Spool)?;
        let metadata = npages
            .checked_mul(std::mem::size_of::<BookV2MarkedPage>())
            .and_then(|n| n.checked_add(metadata))
            .and_then(|n| n.checked_add(ngroups.checked_mul(std::mem::size_of::<Range<usize>>())?))
            .and_then(|n| {
                n.checked_add(
                    anchor_count.checked_mul(std::mem::size_of::<BookV2SemanticAnchor>())?,
                )
            })
            .ok_or(E::Spool)?;
        let mut out = Encoder {
            budget: &mut self.budget,
            bytes: None,
            length: 0,
            output_limit: 0,
            spool_limit: 0,
        };
        // Remaining final-output capacity includes exactly the contributions
        // replaced here; copied spool storage never receives a credit.
        out.output_limit = out.budget.max_output - charged;
        out.spool_limit = (out.budget.max_spool - out.budget.spool)
            .checked_sub(metadata as u64)
            .ok_or(E::Spool)?;
        if encode(self.source, self.text, &mut out, None, None, None)? != anchor_count {
            return Err(E::Identity);
        }
        let body_length = out.length;
        encode_anchor_objects(&mut out, first_anchor, object_count != 0, None)?;
        let length = out.length;
        let mut reserved = self.budget;
        reserved.output = charged;
        reserved.reserve(
            records,
            metadata.checked_add(length).ok_or(E::Spool)?,
            length,
        )?;
        self.budget = reserved;
        self.credit_available = false;
        self.budget.step(1)?;
        let mut pages = Vec::new();
        let mut groups = Vec::new();
        let mut anchors = Vec::new();
        let mut anchor_objects = Vec::new();
        anchor_objects
            .try_reserve_exact(object_count)
            .map_err(|_| E::Allocation)?;
        let mut bytes = Vec::new();
        pages.try_reserve_exact(npages).map_err(|_| E::Allocation)?;
        groups
            .try_reserve_exact(ngroups)
            .map_err(|_| E::Allocation)?;
        anchors
            .try_reserve_exact(anchor_count)
            .map_err(|_| E::Allocation)?;
        bytes.try_reserve_exact(length).map_err(|_| E::Allocation)?;
        let mut out = Encoder {
            budget: &mut self.budget,
            bytes: Some(&mut bytes),
            length: 0,
            output_limit: length as u64,
            spool_limit: length as u64,
        };
        if encode(
            self.source,
            self.text,
            &mut out,
            Some(&mut pages),
            Some(&mut groups),
            Some(&mut anchors),
        )? != anchor_count
            || out.length != body_length
            || pages.len() != npages
            || groups.len() != ngroups
            || anchors.len() != anchor_count
        {
            return Err(E::Identity);
        }
        encode_anchor_objects(
            &mut out,
            first_anchor,
            object_count != 0,
            Some(&mut anchor_objects),
        )?;
        if out.length != length || anchor_objects.len() != object_count {
            return Err(E::Identity);
        }
        self.budget.step(length.div_ceil(64) + 1)?;
        let mut fingerprint = sha256(BOOK_V2_MARKED_CONTENT_ALGORITHM.as_bytes());
        fingerprint = self.budget.fold(fingerprint, &self.source.fingerprint())?;
        fingerprint = self.budget.fold(fingerprint, &self.text.fingerprint())?;
        fingerprint = self.budget.fold(fingerprint, &sha256(&bytes))?;
        for p in &pages {
            let mut b = [0; 36];
            b[..4].copy_from_slice(&p.page.to_be_bytes());
            for (offset, value) in [
                (4, p.groups.start),
                (12, p.groups.end),
                (20, p.bytes.start),
                (28, p.bytes.end),
            ] {
                b[offset..offset + 8].copy_from_slice(&(value as u64).to_be_bytes());
            }
            fingerprint = self.budget.fold(fingerprint, &b)?;
        }
        for g in &groups {
            let mut b = [0; 16];
            b[..8].copy_from_slice(&(g.start as u64).to_be_bytes());
            b[8..].copy_from_slice(&(g.end as u64).to_be_bytes());
            fingerprint = self.budget.fold(fingerprint, &b)?;
        }
        for a in &anchors {
            let mut b = [0; 60];
            b[..4].copy_from_slice(&a.page.to_be_bytes());
            b[4..12].copy_from_slice(&(a.group as u64).to_be_bytes());
            b[12..20].copy_from_slice(&(a.paint as u64).to_be_bytes());
            for (i, value) in [
                a.viewport.x().raw(),
                a.viewport.y().raw(),
                a.viewport.width().get().raw(),
                a.viewport.height().get().raw(),
                a.baseline.raw(),
            ]
            .into_iter()
            .enumerate()
            {
                b[20 + i * 8..28 + i * 8].copy_from_slice(&value.to_be_bytes());
            }
            fingerprint = self.budget.fold(fingerprint, &b)?;
        }
        Ok(BookV2MarkedContent {
            source: self.source,
            text: self.text,
            pages,
            groups,
            anchors,
            anchor_objects,
            body_length,
            bytes,
            fingerprint,
            budget: self.budget,
        })
    }
}
