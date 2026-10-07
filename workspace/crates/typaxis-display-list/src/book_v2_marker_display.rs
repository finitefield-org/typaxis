//! Generated list/note markers and separator ink over the actual stable pages.
pub use super::super::marker_geometry::ProductionGeneratedMarkerCluster;
use super::*;
use typaxis_pagination::{
    ProductionBodyFootnotePlacedFragment, ProductionBodyFootnotePlacedMarker,
    ProductionBodyListMarker, ProductionTablePlacedCellRole,
};
use typaxis_shaping::{
    GlyphRun, ProductionBodyFont, ProductionFootnoteMarkerShape, ProductionListMarkerShape,
};
use typaxis_text::GeneratedProvenance;

pub const BOOK_V2_MARKER_DISPLAY_ALGORITHM: &str = "typaxis.book-2-marker-display/1";
#[derive(Clone, Copy)]
pub enum BookV2MarkerSource<'s, 'a> {
    List(&'s ProductionListMarkerShape<'a>),
    Footnote(&'s ProductionFootnoteMarkerShape<'a>),
}
impl<'s, 'a> BookV2MarkerSource<'s, 'a> {
    pub fn owner(self) -> NodeId {
        match self {
            Self::List(v) => v.source().owner(),
            Self::Footnote(v) => v.source().owner(),
        }
    }
    pub fn font(self) -> &'s ProductionBodyFont {
        match self {
            Self::List(v) => v.font(),
            Self::Footnote(v) => v.font(),
        }
    }
    pub fn glyph_run(self) -> &'s GlyphRun {
        match self {
            Self::List(v) => v.glyph_run(),
            Self::Footnote(v) => v.glyph_run(),
        }
    }
    pub fn utf8(self) -> &'a str {
        match self {
            Self::List(v) => v.utf8(),
            Self::Footnote(v) => v.utf8(),
        }
    }
    pub fn provenance(self) -> GeneratedProvenance {
        match self {
            Self::List(v) => v.provenance(),
            Self::Footnote(v) => v.provenance(),
        }
    }
    pub fn advance(self) -> PositiveLength {
        match self {
            Self::List(v) => v.advance(),
            Self::Footnote(v) => v.advance(),
        }
    }
    pub fn fingerprint(self) -> [u8; 32] {
        match self {
            Self::List(v) => v.fingerprint(),
            Self::Footnote(v) => v.fingerprint(),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum BookV2MarkerPlacement<'g> {
    List(&'g ProductionBodyListMarker),
    Footnote(&'g ProductionBodyFootnotePlacedMarker),
}
impl BookV2MarkerPlacement<'_> {
    pub fn bounds(self) -> Rect {
        match self {
            Self::List(v) => v.bounds(),
            Self::Footnote(v) => v.bounds(),
        }
    }
    pub fn baseline(self) -> Length {
        match self {
            Self::List(v) => v.baseline(),
            Self::Footnote(v) => v.baseline(),
        }
    }
    pub fn local_fragment_index(self) -> u32 {
        match self {
            Self::List(v) => v.fragment_index(),
            Self::Footnote(v) => v.fragment_index(),
        }
    }
}
fn placements<'g>(
    lists: &'g [ProductionBodyListMarker],
    notes: &'g [ProductionBodyFootnotePlacedMarker],
) -> impl Iterator<Item = BookV2MarkerPlacement<'g>> {
    let mut lists = lists.iter().peekable();
    let mut notes = notes.iter().peekable();
    std::iter::from_fn(move || match (lists.peek(), notes.peek()) {
        (Some(l), Some(n)) if n.fragment_index() <= l.fragment_index() => {
            notes.next().map(BookV2MarkerPlacement::Footnote)
        }
        (Some(_), _) => lists.next().map(BookV2MarkerPlacement::List),
        (_, Some(_)) => notes.next().map(BookV2MarkerPlacement::Footnote),
        _ => None,
    })
}
pub struct BookV2MarkerDraw<'g, 's, 'a> {
    source: BookV2MarkerSource<'s, 'a>,
    placement: BookV2MarkerPlacement<'g>,
    fragment: &'g ProductionBodyFootnotePlacedFragment,
    fragment_index: usize,
    cell: Option<ProductionTablePlacedCellRole>,
    repeated: bool,
    clusters: Vec<ProductionGeneratedMarkerCluster<'a>>,
}
impl<'g, 's, 'a> BookV2MarkerDraw<'g, 's, 'a> {
    pub fn source(&self) -> BookV2MarkerSource<'s, 'a> {
        self.source
    }
    pub fn placement(&self) -> BookV2MarkerPlacement<'g> {
        self.placement
    }
    pub fn fragment(&self) -> &'g ProductionBodyFootnotePlacedFragment {
        self.fragment
    }
    pub fn fragment_index(&self) -> usize {
        self.fragment_index
    }
    pub fn cell_role(&self) -> Option<ProductionTablePlacedCellRole> {
        self.cell
    }
    pub fn repeated_header(&self) -> bool {
        self.repeated
    }
    pub fn clusters(&self) -> &[ProductionGeneratedMarkerCluster<'a>] {
        &self.clusters
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2FootnoteSeparatorDraw {
    page: u32,
    before_fragment: usize,
    ink: Rect,
}
impl BookV2FootnoteSeparatorDraw {
    pub fn page_index(self) -> u32 {
        self.page
    }
    pub fn before_fragment_index(self) -> usize {
        self.before_fragment
    }
    pub fn ink(self) -> Rect {
        self.ink
    }
}
pub struct BookV2MarkerDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &'d AdmittedProductionResourceLedgerV3,
    draws: Vec<BookV2MarkerDraw<'g, 's, 'a>>,
    separators: Vec<BookV2FootnoteSeparatorDraw>,
    fingerprint: [u8; 32],
    records: u64,
    work: u64,
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2MarkerDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn source(&self) -> &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn admitted(&self) -> &'d AdmittedProductionResourceLedgerV3 {
        self.admitted
    }
    pub fn draws(&self) -> &[BookV2MarkerDraw<'g, 's, 'a>] {
        &self.draws
    }
    pub fn separators(&self) -> &[BookV2FootnoteSeparatorDraw] {
        &self.separators
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
    fn marker_source(
        &mut self,
        placement: BookV2MarkerPlacement<'_>,
        offset: usize,
    ) -> Result<BookV2MarkerSource<'s, 'a>, BookV2MathDisplayError> {
        let index = offset
            .checked_add(placement.local_fragment_index() as usize)
            .ok_or_else(|| error(NodeId::new(0), E::RecordLimit))?;
        let shaped = self
            .source
            .fragment_flow(index)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?
            .lines()
            .prepared()
            .shaped();
        let source = match placement {
            BookV2MarkerPlacement::List(p) => {
                self.step(p.owner())?;
                let m = shaped
                    .list_markers()
                    .get(p.marker_index() as usize)
                    .filter(|m| {
                        m.marker_index() == p.marker_index() && m.source().owner() == p.owner()
                    })
                    .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?;
                BookV2MarkerSource::List(m)
            }
            BookV2MarkerPlacement::Footnote(p) => {
                self.step(NodeId::new(0))?;
                let m = shaped
                    .footnote_markers()
                    .get(p.definition_index())
                    .filter(|m| m.definition_index() as usize == p.definition_index())
                    .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
                BookV2MarkerSource::Footnote(m)
            }
        };
        Ok(source)
    }
    fn marker_records(
        &mut self,
        source: BookV2MarkerSource<'_, '_>,
    ) -> Result<usize, BookV2MathDisplayError> {
        let mut records = 0usize;
        for cluster in &source.glyph_run().clusters {
            self.step(source.owner())?;
            let glyphs = cluster
                .glyph_end
                .checked_sub(cluster.glyph_start)
                .ok_or_else(|| error(source.owner(), E::ReceiptMismatch))?;
            records = records
                .checked_add(1)
                .and_then(|n| n.checked_add(glyphs as usize))
                .ok_or_else(|| error(source.owner(), E::RecordLimit))?;
        }
        Ok(records)
    }
    pub fn build_markers(
        &mut self,
    ) -> Result<BookV2MarkerDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2MathDisplayError> {
        let root = NodeId::new(0);
        let source = self.source;
        let pages = source.source().geometry().pages();
        let mut required = 1usize;
        let mut count = 0usize;
        let mut separator_count = 0usize;
        let mut offset = 0usize;
        for page in pages {
            self.step(root)?;
            for placement in placements(page.list_markers(), page.footnote_markers()) {
                let marker = self.marker_source(placement, offset)?;
                let records = self.marker_records(marker)?;
                required = required
                    .checked_add(1)
                    .and_then(|n| n.checked_add(records))
                    .ok_or_else(|| error(root, E::RecordLimit))?;
                count = count
                    .checked_add(1)
                    .ok_or_else(|| error(root, E::RecordLimit))?;
            }
            offset = offset
                .checked_add(page.fragments().len())
                .ok_or_else(|| error(root, E::RecordLimit))?;
            if page.separator_ink().is_some() {
                required = required
                    .checked_add(1)
                    .ok_or_else(|| error(root, E::RecordLimit))?;
                separator_count = separator_count
                    .checked_add(1)
                    .ok_or_else(|| error(root, E::RecordLimit))?;
            }
        }
        take(&mut self.remaining, required, root)?;
        let mut draws = Vec::new();
        draws
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let mut separators = Vec::new();
        separators
            .try_reserve_exact(separator_count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let parsed = u32::try_from(
            source
                .source()
                .flow()
                .lines()
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
        let mut fingerprint = sha256(BOOK_V2_MARKER_DISPLAY_ALGORITHM.as_bytes());
        fingerprint = self.fold(fingerprint, &source.fingerprint(), root)?;
        fingerprint = self.fold(fingerprint, &self.admitted.fingerprint(), root)?;
        let mut offset = 0usize;
        for page in pages {
            self.step(root)?;
            let mut role_flags = page.fragments_with_roles().map(|(_, _, repeated)| repeated);
            let mut next_role = 0usize;
            let mut repeated = false;
            for placement in placements(page.list_markers(), page.footnote_markers()) {
                let marker = self.marker_source(placement, offset)?;
                let owner = marker.owner();
                let local = placement.local_fragment_index() as usize;
                while next_role <= local {
                    repeated = role_flags
                        .next()
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    next_role += 1;
                }
                let fragment = page
                    .fragments()
                    .get(local)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let cell = *page
                    .cell_roles()
                    .get(local)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let page_index = fragment.fragment().page_index();
                if page_index != page.selection().page_index() {
                    return Err(error(owner, E::ReceiptMismatch).into());
                }
                match placement {
                    BookV2MarkerPlacement::List(p) => {
                        if p.page_index() != page_index
                            || fragment
                                .fragment()
                                .baseline()
                                .is_some_and(|b| b != p.baseline())
                        {
                            return Err(error(owner, E::ReceiptMismatch).into());
                        }
                    }
                    BookV2MarkerPlacement::Footnote(p) => {
                        if fragment.definition_index() != Some(p.definition_index()) {
                            return Err(error(owner, E::ReceiptMismatch).into());
                        }
                    }
                }
                let font = marker.font();
                let face = self
                    .admitted
                    .font(font.face_id())
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                if face.content_hash() != font.content_hash()
                    || face.face_index() != font.face_index()
                    || placement.bounds().width() != marker.advance()
                {
                    return Err(error(owner, E::ReceiptMismatch).into());
                }
                let mut reserved = self.marker_records(marker)? as u64;
                let mut clusters = Vec::new();
                clusters
                    .try_reserve_exact(marker.glyph_run().clusters.len())
                    .map_err(|_| error(owner, E::AllocationFailure))?;
                marker_geometry::project_marker(
                    marker_geometry::MarkerSource {
                        owner,
                        run: marker.glyph_run(),
                        key: marker.provenance().buffer_key(),
                        utf8: marker.utf8(),
                        provenance: marker.provenance(),
                        advance: marker.advance(),
                    },
                    placement.bounds(),
                    placement.baseline(),
                    parsed,
                    &mut reserved,
                    |n| self.step(n),
                    |cluster| {
                        clusters.push(cluster);
                        Ok(())
                    },
                )?;
                if reserved != 0 || clusters.len() != marker.glyph_run().clusters.len() {
                    return Err(error(owner, E::ReceiptMismatch).into());
                }
                let fragment_index = offset
                    .checked_add(local)
                    .ok_or_else(|| error(owner, E::RecordLimit))?;
                let mut bytes = [0u8; 80];
                bytes[0] = u8::from(matches!(placement, BookV2MarkerPlacement::Footnote(_)));
                bytes[1..5].copy_from_slice(&owner.get().to_be_bytes());
                bytes[5..9].copy_from_slice(&page_index.to_be_bytes());
                bytes[9..17].copy_from_slice(&(fragment_index as u64).to_be_bytes());
                bytes[17] = u8::from(fragment.definition_index().is_some());
                bytes[18..26].copy_from_slice(
                    &(fragment.definition_index().unwrap_or(0) as u64).to_be_bytes(),
                );
                bytes[26..34].copy_from_slice(&(fragment.item_index() as u64).to_be_bytes());
                bytes[34] = u8::from(cell.is_some());
                bytes[35..39].copy_from_slice(&cell.map_or(0, |r| r.owner().get()).to_be_bytes());
                bytes[39] = u8::from(repeated);
                put_rect(&mut bytes[40..72], placement.bounds());
                bytes[72..80].copy_from_slice(&placement.baseline().raw().to_be_bytes());
                fingerprint = self.fold(fingerprint, &bytes, owner)?;
                fingerprint = self.fold(fingerprint, &marker.fingerprint(), owner)?;
                for cluster in &clusters {
                    let span = cluster.text_span();
                    let mut bytes = [0u8; 33];
                    bytes[..4].copy_from_slice(&cluster.cluster_index().to_be_bytes());
                    bytes[4..8].copy_from_slice(&span.text_id().get().to_be_bytes());
                    bytes[8..12].copy_from_slice(&span.range().start_byte().get().to_be_bytes());
                    bytes[12..16].copy_from_slice(&span.range().end_byte().get().to_be_bytes());
                    bytes[16..24]
                        .copy_from_slice(&(cluster.exact_text().len() as u64).to_be_bytes());
                    bytes[24..32].copy_from_slice(&(cluster.glyphs().len() as u64).to_be_bytes());
                    bytes[32] = u8::from(cluster.logical_bounds().is_some());
                    fingerprint = self.fold(fingerprint, &bytes, owner)?;
                    if let Some(bounds) = cluster.logical_bounds() {
                        let mut bytes = [0u8; 32];
                        put_rect(&mut bytes, bounds);
                        fingerprint = self.fold(fingerprint, &bytes, owner)?;
                    }
                    for chunk in cluster.exact_text().as_bytes().chunks(104) {
                        fingerprint = self.fold(fingerprint, chunk, owner)?;
                    }
                    for glyph in cluster.glyphs() {
                        let mut bytes = [0u8; 18];
                        bytes[..2].copy_from_slice(&glyph.original_gid().get().to_be_bytes());
                        bytes[2..10].copy_from_slice(&glyph.x().raw().to_be_bytes());
                        bytes[10..18].copy_from_slice(&glyph.y().raw().to_be_bytes());
                        fingerprint = self.fold(fingerprint, &bytes, owner)?;
                    }
                }
                draws.push(BookV2MarkerDraw {
                    source: marker,
                    placement,
                    fragment,
                    fragment_index,
                    cell,
                    repeated,
                    clusters,
                });
            }
            if let Some(ink) = page.separator_ink() {
                let mut first = None;
                for (index, fragment) in page.fragments().iter().enumerate() {
                    self.step(root)?;
                    if fragment.definition_index().is_some() {
                        first = Some(index);
                        break;
                    }
                }
                let before_fragment = offset
                    .checked_add(first.ok_or_else(|| error(root, E::ReceiptMismatch))?)
                    .ok_or_else(|| error(root, E::RecordLimit))?;
                let page = page.selection().page_index();
                let mut bytes = [0u8; 44];
                bytes[..4].copy_from_slice(&page.to_be_bytes());
                bytes[4..12].copy_from_slice(&(before_fragment as u64).to_be_bytes());
                put_rect(&mut bytes[12..], ink);
                fingerprint = self.fold(fingerprint, &bytes, root)?;
                separators.push(BookV2FootnoteSeparatorDraw {
                    page,
                    before_fragment,
                    ink,
                });
            }
            offset = offset
                .checked_add(page.fragments().len())
                .ok_or_else(|| error(root, E::RecordLimit))?;
        }
        Ok(BookV2MarkerDisplay {
            source,
            admitted: self.admitted,
            draws,
            separators,
            fingerprint,
            records: self.record_charge(),
            work: self.work_steps(),
        })
    }
}
