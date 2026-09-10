//! Joint page projection authenticated by completed stable math terminals.
use super::*;
use typaxis_pagination::ProductionBodyFootnoteMathTerminals;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionFootnoteSeparatorDraw {
    page_index: u32,
    before_draw: usize,
    ink: Rect,
}
impl ProductionFootnoteSeparatorDraw {
    pub fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn before_draw(&self) -> usize {
        self.before_draw
    }
    pub fn ink(&self) -> Rect {
        self.ink
    }
}
/// Complete projection; structure, tagging and PDF publication are separate.
pub struct ProductionBodyFootnoteDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'d ProductionBodyFootnoteMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &'d AdmittedResourceLedger,
    draws: Vec<ProductionBodyDraw<'d>>,
    anchors: Vec<ProductionBodyInlineAnchor<'d>>,
    separators: Vec<ProductionFootnoteSeparatorDraw>,
    table_draws: Vec<(usize, typaxis_pagination::ProductionTablePlacedCellRole)>,
    record_charge: u64,
    fingerprint: [u8; 32],
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> ProductionBodyFootnoteDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn source(&self) -> &'d ProductionBodyFootnoteMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn resource_declarations(&self) -> &typaxis_document::StagingM4ResourceCatalog {
        self.source
            .line_layout()
            .source_flow()
            .resource_declarations()
    }
    pub fn table_draw_role(
        &self,
        index: usize,
    ) -> Option<typaxis_pagination::ProductionTablePlacedCellRole> {
        self.table_draws
            .binary_search_by_key(&index, |(i, _)| *i)
            .ok()
            .map(|i| self.table_draws[i].1)
    }
    pub fn repeated_header_draws(&self) -> impl Iterator<Item = usize> + '_ {
        self.table_draws
            .iter()
            .filter_map(|(index, role)| role.repeated_header().then_some(*index))
    }
    pub fn draws(&self) -> &[ProductionBodyDraw<'d>] {
        &self.draws
    }
    pub fn inline_anchors(&self) -> &[ProductionBodyInlineAnchor<'d>] {
        &self.anchors
    }
    pub fn separators(&self) -> &[ProductionFootnoteSeparatorDraw] {
        &self.separators
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify_resources(
        &self,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyDisplayError> {
        if !std::ptr::eq(admitted, self.admitted)
            || self
                .source
                .line_layout()
                .binding_epoch()
                .limits_fingerprint()
                != limits.fingerprint()
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}
fn fragment_index(draw: &ProductionBodyDraw<'_>) -> u32 {
    match draw {
        ProductionBodyDraw::Text(d) => d.fragment_index,
        ProductionBodyDraw::Vector(d) => d.fragment_index,
        ProductionBodyDraw::SvgFigure(d) => d.fragment_index,
        ProductionBodyDraw::Raster(d) => d.fragment_index,
        ProductionBodyDraw::Math(d) => d.fragment_index(),
    }
}
// Fixed-size incremental encoding avoids an uncharged document-sized buffer.
fn feed(digest: &mut [u8; 32], tag: u8, values: &[i64]) {
    let mut bytes = [0u8; 128];
    bytes[..32].copy_from_slice(digest);
    bytes[32] = tag;
    bytes[33] = values.len() as u8;
    for (slot, n) in bytes[40..].chunks_exact_mut(8).zip(values) {
        slot.copy_from_slice(&n.to_be_bytes());
    }
    *digest = sha256(&bytes);
}
fn rect(digest: &mut [u8; 32], tag: u8, r: Rect) {
    feed(
        digest,
        tag,
        &[
            r.x().raw(),
            r.y().raw(),
            r.width().get().raw(),
            r.height().get().raw(),
        ],
    );
}

pub fn build_production_footnote_display<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    source: &'d ProductionBodyFootnoteMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &'d AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodyFootnoteDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, ProductionBodyDisplayError>
{
    let root = NodeId::new(0);
    let lines = source.line_layout();
    if lines.binding_epoch().admitted_fingerprint() != admitted.fingerprint().bytes()
        || lines.binding_epoch().limits_fingerprint() != limits.fingerprint()
    {
        return Err(error(root, E::ReceiptMismatch));
    }
    let maximum = limits.base().get().max_fragments;
    let mut remaining = maximum
        .checked_sub(source.record_charge())
        .ok_or_else(|| error(root, E::RecordLimit))?;
    take(&mut remaining, 1, root)?;
    let mut fingerprint = sha256(b"typaxis.production-footnote-display/1");
    for hash in [
        lines.fingerprint(),
        source.block_layout().receipt().fingerprint(),
        source.terminals().fingerprint(),
    ] {
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(&fingerprint);
        bytes[32..].copy_from_slice(&hash);
        fingerprint = sha256(&bytes);
    }
    for page in source.geometry().pages() {
        feed(
            &mut fingerprint,
            0,
            &[i64::from(page.page_index()), page.fragments().len() as i64],
        );
        if let Some(ink) = page.separator_ink() {
            rect(&mut fingerprint, 1, ink);
        }
        for (local, placed) in page.fragments().iter().enumerate() {
            if let Some(role) = page.cell_role(local) {
                feed(
                    &mut fingerprint,
                    15,
                    &[role.owner().get().into(), i64::from(role.repeated_header())],
                );
            }
            let f = placed.fragment();
            feed(
                &mut fingerprint,
                2,
                &[
                    i64::from(f.owner().get()),
                    i64::from(f.page_index()),
                    placed.definition_index().map_or(-1, |n| n as i64),
                    placed.item_index() as i64,
                    f.effective_space_before().raw(),
                ],
            );
            match f.source() {
                ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                } => feed(
                    &mut fingerprint,
                    3,
                    &[paragraph_index.into(), line_index.into()],
                ),
                ProductionBodyFragmentSource::VectorBlock { block_index } => {
                    feed(&mut fingerprint, 4, &[block_index.into()])
                }
                ProductionBodyFragmentSource::NativeMathBlock { block_index } => {
                    feed(&mut fingerprint, 14, &[block_index.into()])
                }
                ProductionBodyFragmentSource::Figure { figure_index } => {
                    feed(&mut fingerprint, 5, &[figure_index.into()])
                }
            }
            rect(&mut fingerprint, 6, f.bounds());
            if let Some(b) = f.baseline() {
                feed(&mut fingerprint, 7, &[b.raw()]);
            }
            if let Some(v) = f.viewport() {
                rect(&mut fingerprint, 8, v);
            }
        }
        for m in page.list_markers() {
            feed(
                &mut fingerprint,
                9,
                &[
                    m.marker_index().into(),
                    m.fragment_index().into(),
                    m.baseline().raw(),
                ],
            );
            rect(&mut fingerprint, 10, m.bounds());
        }
        for m in page.footnote_markers() {
            feed(
                &mut fingerprint,
                11,
                &[
                    m.definition_index() as i64,
                    m.fragment_index().into(),
                    m.baseline().raw(),
                ],
            );
            rect(&mut fingerprint, 12, m.bounds());
        }
    }
    let mut draws = Vec::new();
    let mut anchors = Vec::new();
    let mut separators = Vec::new();
    let mut table_draws = Vec::new();
    let mut offset = 0u32;
    for page in source.geometry().pages() {
        take(
            &mut remaining,
            page.fragments()
                .len()
                .checked_add(1)
                .ok_or_else(|| error(root, E::RecordLimit))?,
            root,
        )?;
        let mut fragments = Vec::new();
        fragments
            .try_reserve_exact(page.fragments().len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        fragments.extend(page.fragments().iter().map(|f| f.fragment()));
        let input = DisplayInput {
            lines,
            blocks: source.block_layout(),
            fragments: &fragments,
            markers: page.list_markers(),
            numbers: source.equation_numbers(),
            registry: Some(source.registry()),
            fingerprint,
            fragment_offset: offset,
            footnote_markers: page.footnote_markers(),
        };
        let projection = project_display(&input, admitted, limits, maximum - remaining)?;
        remaining = maximum
            .checked_sub(projection.record_charge)
            .ok_or_else(|| error(root, E::RecordLimit))?;
        if let Some(ink) = page.separator_ink() {
            take(&mut remaining, 1, root)?;
            let first = page
                .fragments()
                .iter()
                .position(|f| f.definition_index().is_some())
                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
            let first = offset
                .checked_add(u32::try_from(first).map_err(|_| error(root, E::RecordLimit))?)
                .ok_or_else(|| error(root, E::RecordLimit))?;
            let before = projection
                .draws
                .partition_point(|d| fragment_index(d) < first);
            separators
                .try_reserve(1)
                .map_err(|_| error(root, E::AllocationFailure))?;
            separators.push(ProductionFootnoteSeparatorDraw {
                page_index: page.page_index(),
                before_draw: draws
                    .len()
                    .checked_add(before)
                    .ok_or_else(|| error(root, E::RecordLimit))?,
                ink,
            });
        }
        // Preserve both temporary projection and retained output allocation charges.
        take(
            &mut remaining,
            projection
                .draws
                .len()
                .checked_add(projection.inline_anchors.len())
                .ok_or_else(|| error(root, E::RecordLimit))?,
            root,
        )?;
        draws
            .try_reserve(projection.draws.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        anchors
            .try_reserve(projection.inline_anchors.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        for (index, draw) in projection.draws.iter().enumerate() {
            let local = fragment_index(draw)
                .checked_sub(offset)
                .ok_or_else(|| error(root, E::ReceiptMismatch))? as usize;
            if let Some(role) = page.cell_role(local) {
                take(&mut remaining, 1, role.owner())?;
                table_draws
                    .try_reserve(1)
                    .map_err(|_| error(role.owner(), E::AllocationFailure))?;
                table_draws.push((
                    draws
                        .len()
                        .checked_add(index)
                        .ok_or_else(|| error(root, E::RecordLimit))?,
                    role,
                ));
            }
        }
        draws.extend(projection.draws);
        // Repeated header copies do not create another destination occurrence.
        anchors.extend(projection.inline_anchors.into_iter().filter(|a| {
            !page
                .cell_role((a.fragment_index() - offset) as usize)
                .is_some_and(|r| r.repeated_header())
        }));
        offset = offset
            .checked_add(u32::try_from(fragments.len()).map_err(|_| error(root, E::RecordLimit))?)
            .ok_or_else(|| error(root, E::RecordLimit))?;
    }
    Ok(ProductionBodyFootnoteDisplay {
        source,
        admitted,
        draws,
        anchors,
        separators,
        table_draws,
        record_charge: maximum - remaining,
        fingerprint,
    })
}
