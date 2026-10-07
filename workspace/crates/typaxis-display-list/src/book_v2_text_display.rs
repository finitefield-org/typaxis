//! Page-space text retains the exact selected cluster and successor owner.
use super::*;
use typaxis_layout::ProductionPlacedTextCluster;
use typaxis_shaping::{ProductionBodyFont, ShapeSourceSpan};

pub const BOOK_V2_TEXT_DISPLAY_ALGORITHM: &str = "typaxis.book-2-text-display/1";

/// A cluster is the extraction unit, even when it contains several glyphs.
/// Repeated table headers retain their explicit display-only role.
#[derive(Debug)]
pub struct BookV2TextDraw<'s, 'p, 'a> {
    cluster: &'s ProductionPlacedTextCluster<'p, 'a>,
    font: &'p ProductionBodyFont,
    page: u32,
    fragment: usize,
    definition: Option<usize>,
    item: usize,
    inline: u32,
    cell: Option<NodeId>,
    repeated: bool,
    text_span: DisplayTextSpan,
    bounds: Option<Rect>,
    glyphs: Vec<ProductionBodyGlyph>,
}
impl<'s, 'p, 'a> BookV2TextDraw<'s, 'p, 'a> {
    pub fn cluster(&self) -> &'s ProductionPlacedTextCluster<'p, 'a> {
        self.cluster
    }
    pub fn font(&self) -> &'p ProductionBodyFont {
        self.font
    }
    pub fn owner(&self) -> NodeId {
        self.cluster.run().owner()
    }
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn fragment_index(&self) -> usize {
        self.fragment
    }
    pub fn definition_index(&self) -> Option<usize> {
        self.definition
    }
    pub fn item_index(&self) -> usize {
        self.item
    }
    pub fn inline_index(&self) -> u32 {
        self.inline
    }
    pub fn cell_owner(&self) -> Option<NodeId> {
        self.cell
    }
    pub fn repeated_header(&self) -> bool {
        self.repeated
    }
    pub fn text_span(&self) -> DisplayTextSpan {
        self.text_span
    }
    pub fn exact_text(&self) -> &'a str {
        self.cluster.utf8()
    }
    pub fn generated_provenance(&self) -> Option<typaxis_text::GeneratedProvenance> {
        match self.cluster.source_span() {
            ShapeSourceSpan::Generated(p) => Some(p),
            ShapeSourceSpan::Parsed(_) => None,
        }
    }
    pub fn logical_bounds(&self) -> Option<Rect> {
        self.bounds
    }
    pub fn glyphs(&self) -> &[ProductionBodyGlyph] {
        &self.glyphs
    }
}

pub struct BookV2TextDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &'d AdmittedProductionResourceLedgerV3,
    draws: Vec<BookV2TextDraw<'s, 'p, 'a>>,
    fingerprint: [u8; 32],
    records: u64,
    work: u64,
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2TextDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn source(&self) -> &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn admitted(&self) -> &'d AdmittedProductionResourceLedgerV3 {
        self.admitted
    }
    pub fn draws(&self) -> &[BookV2TextDraw<'s, 'p, 'a>] {
        &self.draws
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
}

impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2MathDisplayBuilder<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    /// Uses the same source/font admission and cumulative counters as math
    /// projection. Building either component never refunds the other component.
    pub fn build_text(
        &mut self,
    ) -> Result<BookV2TextDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2MathDisplayError> {
        let root = NodeId::new(0);
        let source = self.source;
        let lines = source.source().flow().lines();
        let pages = source.source().geometry().pages();
        let mut required = 1usize;
        let mut count = 0usize;
        // Preflight every retained cluster/glyph before allocating the draw list.
        let mut fragment_index = 0usize;
        for page in pages {
            self.step(root)?;
            for placed in page.fragments() {
                let fragment = placed.fragment();
                self.step(fragment.owner())?;
                let lines = source
                    .fragment_flow(fragment_index)
                    .ok_or_else(|| error(fragment.owner(), E::ReceiptMismatch))?
                    .lines();
                fragment_index = fragment_index
                    .checked_add(1)
                    .ok_or_else(|| error(fragment.owner(), E::RecordLimit))?;
                if let ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                } = fragment.source()
                {
                    let paragraph = lines
                        .paragraphs()
                        .get(paragraph_index as usize)
                        .ok_or_else(|| error(fragment.owner(), E::ReceiptMismatch))?;
                    let line = paragraph
                        .lines()
                        .get(line_index as usize)
                        .ok_or_else(|| error(fragment.owner(), E::ReceiptMismatch))?;
                    for item in line.items() {
                        self.step(fragment.owner())?;
                        if let ProductionPlacedInline::Text(cluster) = item {
                            count = count
                                .checked_add(1)
                                .ok_or_else(|| error(root, E::RecordLimit))?;
                            required = required
                                .checked_add(1)
                                .and_then(|n| n.checked_add(cluster.glyphs().len()))
                                .ok_or_else(|| error(root, E::RecordLimit))?;
                        }
                    }
                }
            }
        }
        if required as u64 > self.remaining {
            return Err(error(root, E::RecordLimit).into());
        }
        // Commit the entire allocation before reserving it; a later failure
        // retains even the slots whose glyph projection was never reached.
        take(&mut self.remaining, required, root)?;
        let mut draws = Vec::new();
        draws
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let parsed_count = u32::try_from(
            lines
                .prepared()
                .source_flow()
                .body()
                .body()
                .wire()
                .text_buffers()
                .len(),
        )
        .map_err(|_| error(root, E::RecordLimit))?;
        self.step(root)?;
        let mut fingerprint = sha256(BOOK_V2_TEXT_DISPLAY_ALGORITHM.as_bytes());
        fingerprint = self.fold(fingerprint, &source.fingerprint(), root)?;
        fingerprint = self.fold(fingerprint, &self.admitted.fingerprint(), root)?;
        let mut index = 0usize;
        for page in pages {
            self.step(root)?;
            for (placed, role, repeated) in page.fragments_with_roles() {
                let fragment = placed.fragment();
                self.step(fragment.owner())?;
                let lines = source
                    .fragment_flow(index)
                    .ok_or_else(|| error(fragment.owner(), E::ReceiptMismatch))?
                    .lines();
                if let ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                } = fragment.source()
                {
                    let paragraph = &lines.paragraphs()[paragraph_index as usize];
                    for (inline, item) in paragraph.lines()[line_index as usize]
                        .items()
                        .iter()
                        .enumerate()
                    {
                        self.step(fragment.owner())?;
                        let ProductionPlacedInline::Text(cluster) = item else {
                            continue;
                        };
                        let owner = cluster.run().owner();
                        let font = paragraph
                            .font()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        let face = self
                            .admitted
                            .font(font.face_id())
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        if face.content_hash() != font.content_hash()
                            || face.face_index() != font.face_index()
                        {
                            return Err(error(owner, E::ReceiptMismatch).into());
                        }
                        // Projection and its fingerprint each walk the actual glyphs.
                        for _ in cluster.glyphs() {
                            self.step(owner)?;
                        }
                        let mut reserved = u64::try_from(cluster.glyphs().len())
                            .ok()
                            .and_then(|n| n.checked_add(1))
                            .ok_or_else(|| error(owner, E::RecordLimit))?;
                        let (glyphs, bounds) =
                            text_geometry::project_cluster(cluster, font, fragment, &mut reserved)?;
                        let (buffer, start, end) = match cluster.source_span() {
                            ShapeSourceSpan::Parsed(span) => {
                                (span.text_id().get(), span.start_byte(), span.end_byte())
                            }
                            ShapeSourceSpan::Generated(p) => {
                                let span = p.text_span();
                                (
                                    parsed_count
                                        .checked_add(span.text_id().get())
                                        .ok_or_else(|| error(owner, E::RecordLimit))?,
                                    span.range().start_byte(),
                                    span.range().end_byte(),
                                )
                            }
                        };
                        let draw = BookV2TextDraw {
                            cluster,
                            font,
                            page: fragment.page_index(),
                            fragment: index,
                            definition: placed.definition_index(),
                            item: placed.item_index(),
                            inline: u32::try_from(inline)
                                .map_err(|_| error(owner, E::RecordLimit))?,
                            cell: role.map(|r| r.owner()),
                            repeated,
                            text_span: DisplayTextSpan::new(
                                DisplayTextBufferId::new(buffer),
                                start,
                                end,
                            )
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                            bounds,
                            glyphs,
                        };
                        // Fixed-size records are length-delimited by the closed schema;
                        // exact UTF-8 is folded in chunks after its byte length.
                        let mut bytes = [0u8; 85];
                        bytes[..4].copy_from_slice(&owner.get().to_be_bytes());
                        bytes[4..8].copy_from_slice(&draw.page.to_be_bytes());
                        bytes[8..16].copy_from_slice(&(index as u64).to_be_bytes());
                        bytes[16] = u8::from(draw.definition.is_some());
                        bytes[17..25]
                            .copy_from_slice(&(draw.definition.unwrap_or(0) as u64).to_be_bytes());
                        bytes[25..33].copy_from_slice(&(draw.item as u64).to_be_bytes());
                        bytes[33..37].copy_from_slice(&draw.inline.to_be_bytes());
                        bytes[37] = u8::from(draw.cell.is_some());
                        bytes[38..42]
                            .copy_from_slice(&draw.cell.map_or(0, |n| n.get()).to_be_bytes());
                        bytes[42] = u8::from(draw.repeated);
                        bytes[43..47].copy_from_slice(&cluster.run_index().to_be_bytes());
                        bytes[47..51].copy_from_slice(&cluster.cluster_index().to_be_bytes());
                        bytes[51] = u8::from(draw.generated_provenance().is_some());
                        bytes[52..56].copy_from_slice(&buffer.to_be_bytes());
                        bytes[56..60].copy_from_slice(&start.get().to_be_bytes());
                        bytes[60..64].copy_from_slice(&end.get().to_be_bytes());
                        bytes[64..72].copy_from_slice(&(cluster.utf8().len() as u64).to_be_bytes());
                        bytes[72..80].copy_from_slice(&(draw.glyphs.len() as u64).to_be_bytes());
                        bytes[80..84].copy_from_slice(&font.face_id().get().to_be_bytes());
                        bytes[84] = u8::from(bounds.is_some());
                        fingerprint = self.fold(fingerprint, &bytes, owner)?;
                        let mut font_bytes = [0u8; 44];
                        font_bytes[..32].copy_from_slice(&font.content_hash());
                        font_bytes[32..36].copy_from_slice(&font.face_index().to_be_bytes());
                        font_bytes[36..44].copy_from_slice(&font.size().get().raw().to_be_bytes());
                        fingerprint = self.fold(fingerprint, &font_bytes, owner)?;
                        if let Some(bounds) = bounds {
                            let mut bytes = [0u8; 32];
                            put_rect(&mut bytes, bounds);
                            fingerprint = self.fold(fingerprint, &bytes, owner)?;
                        }
                        for chunk in cluster.utf8().as_bytes().chunks(104) {
                            fingerprint = self.fold(fingerprint, chunk, owner)?;
                        }
                        for glyph in &draw.glyphs {
                            let mut bytes = [0u8; 18];
                            bytes[..2].copy_from_slice(&glyph.original_gid().get().to_be_bytes());
                            bytes[2..10].copy_from_slice(&glyph.x().raw().to_be_bytes());
                            bytes[10..18].copy_from_slice(&glyph.y().raw().to_be_bytes());
                            fingerprint = self.fold(fingerprint, &bytes, owner)?;
                        }
                        draws.push(draw);
                    }
                }
                index = index
                    .checked_add(1)
                    .ok_or_else(|| error(root, E::RecordLimit))?;
            }
        }
        Ok(BookV2TextDisplay {
            source,
            admitted: self.admitted,
            draws,
            fingerprint,
            records: self.record_charge(),
            work: self.work_steps(),
        })
    }
}
