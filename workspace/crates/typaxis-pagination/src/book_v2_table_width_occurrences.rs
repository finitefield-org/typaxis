//! Source-position observations for reflowing a table across different widths.
use super::*;
use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BookV2TableWidthSource {
    Paragraph { owner: NodeId, units: Range<u32> },
    Block { owner: NodeId },
    ForcedBreak { owner: NodeId },
}
impl BookV2TableWidthSource {
    pub fn owner(&self) -> NodeId {
        match *self {
            Self::Paragraph { owner, .. } | Self::Block { owner } | Self::ForcedBreak { owner } => {
                owner
            }
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookV2TableWidthPiece {
    source: BookV2TableWidthSource,
    repeated: bool,
    header_variant: bool,
    measured_frame: Option<(Length, PositiveLength)>,
    frame: Option<typaxis_layout::ProductionInlineFrame>,
}
impl BookV2TableWidthPiece {
    /// Remeasured source frame, when requested. Forced breaks have no frame.
    pub fn frame(&self) -> Option<typaxis_layout::ProductionInlineFrame> {
        self.frame
    }
    pub fn source(&self) -> &BookV2TableWidthSource {
        &self.source
    }
    pub fn repeated(&self) -> bool {
        self.repeated
    }
    /// This observation belongs to a separately measured root-header copy.
    /// It must not overwrite the original semantic source's width assignment.
    pub fn uses_header_variant(&self) -> bool {
        self.header_variant
    }
    /// Actual variant (start, width), retained when occurrence remeasurement is requested.
    pub fn measured_header_frame(&self) -> Option<(Length, PositiveLength)> {
        self.measured_frame
    }
}
pub struct BookV2TableWidthOccurrence {
    owner: NodeId,
    page: u32,
    definition: Option<usize>,
    parent_width: PositiveLength,
    pieces: Vec<BookV2TableWidthPiece>,
}
impl BookV2TableWidthOccurrence {
    pub fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn definition_index(&self) -> Option<usize> {
        self.definition
    }
    pub fn parent_width(&self) -> PositiveLength {
        self.parent_width
    }
    /// Original semantic pieces followed by repeated header/caption pieces.
    /// This order groups source observations; it is not a paint-order receipt.
    pub fn pieces(&self) -> &[BookV2TableWidthPiece] {
        &self.pieces
    }
}
pub struct BookV2TableWidthOccurrences {
    source: [u8; 32],
    remeasured: bool,
    fingerprint: [u8; 32],
    occurrences: Vec<BookV2TableWidthOccurrence>,
    records: u64,
    work: u64,
}
impl BookV2TableWidthOccurrences {
    pub fn has_remeasured_frames(&self) -> bool {
        self.remeasured
    }
    pub fn occurrences(&self) -> &[BookV2TableWidthOccurrence] {
        &self.occurrences
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
    /// Rebinding still requires a fresh exact owner; content identity alone is
    /// not page/PDF authorization. Generated-label changes invalidate the report.
    pub fn matches_source_flow(
        &self,
        flow: &typaxis_syntax::book_v2::PreparedBookV2TextFlow<'_>,
    ) -> bool {
        self.source == flow.fingerprint()
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Preserve each selected root occurrence rather than collapsing different
    /// physical widths into one table-wide value. Source unit positions survive
    /// a fresh shaping pass; line ordinals and current leaf indexes do not.
    pub fn table_width_occurrences(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2TableWidthOccurrences, ProductionBodyPaginationError> {
        self.collect_table_width_occurrences(closed, false)
    }
    /// Resolve the original hierarchy at each observed root parent width and
    /// attach paragraph/block frames to its source positions. These candidates
    /// still require fresh shaping, actual page convergence and final validation.
    pub fn table_width_frames(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2TableWidthOccurrences, ProductionBodyPaginationError> {
        self.collect_table_width_occurrences(closed, true)
    }
    fn collect_table_width_occurrences(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
        remeasured: bool,
    ) -> Result<BookV2TableWidthOccurrences, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        if !std::ptr::eq(closed.flow(), self.content.flow) {
            return Err(error(root, E::ReceiptMismatch));
        }
        self.verify_mixed_sequence(closed.stable().sequence())?;
        // Count before allocation: repeated exact-one reservations could copy
        // all preceding records at each append, outside the work budget.
        let mut count = 0usize;
        for page in closed.geometry().pages() {
            self.content.step(root)?;
            let mut visit = |is_table: bool| -> Result<(), ProductionBodyPaginationError> {
                self.content.step(root)?;
                if is_table {
                    count = count
                        .checked_add(1)
                        .ok_or_else(|| error(root, E::FragmentLimit))?;
                }
                Ok(())
            };
            for part in page.selection().candidate().parts() {
                visit(part.table().is_some())?;
            }
            if let Some(notes) = page.selection().candidate().footnotes() {
                for selected in notes.fragments() {
                    visit(false)?;
                    if let Some(mixed) = selected.fragment().mixed() {
                        for part in mixed.parts() {
                            visit(part.table().is_some())?;
                        }
                    }
                }
            }
        }
        self.content.charge.take(
            count
                .checked_add(1)
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut occurrences = Vec::new();
        occurrences
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let source = self
            .content
            .flow
            .lines()
            .prepared()
            .source_flow()
            .fingerprint();
        let mut result = BookV2TableWidthOccurrences {
            source,
            remeasured,
            fingerprint: source,
            occurrences,
            records: 0,
            work: 0,
        };
        self.fold_table_occurrence(
            &mut result.fingerprint,
            if remeasured {
                0x5441424c455732
            } else {
                0x5441424c455731
            },
            root,
        )?;
        for page in closed.geometry().pages() {
            self.content.step(root)?;
            for part in page.selection().candidate().parts() {
                self.content.step(root)?;
                if let Some(table) = part.table() {
                    self.append_table_occurrence(&mut result, page.selection(), table, None)?;
                }
            }
            if let Some(notes) = page.selection().candidate().footnotes() {
                for selected in notes.fragments() {
                    self.content.step(root)?;
                    let fragment = selected.fragment();
                    if let Some(mixed) = fragment.mixed() {
                        for part in mixed.parts() {
                            self.content.step(root)?;
                            if let Some(table) = part.table() {
                                self.append_table_occurrence(
                                    &mut result,
                                    page.selection(),
                                    table,
                                    Some(fragment.definition_index()),
                                )?;
                            }
                        }
                    }
                }
            }
        }
        self.fold_table_occurrence(
            &mut result.fingerprint,
            result.occurrences.len() as u64,
            root,
        )?;
        result.records = self.record_charge();
        result.work = self.work_steps();
        Ok(result)
    }
    fn fold_table_occurrence(
        &mut self,
        hash: &mut [u8; 32],
        value: u64,
        owner: NodeId,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.content.step(owner)?;
        let mut bytes = [0; 40];
        bytes[..32].copy_from_slice(hash);
        bytes[32..].copy_from_slice(&value.to_be_bytes());
        *hash = typaxis_core::sha256(&bytes);
        Ok(())
    }
    fn append_table_occurrence(
        &mut self,
        result: &mut BookV2TableWidthOccurrences,
        page: &BookV2BodyMixedPageSelection<'b, 'f, 's, 'p, 'a>,
        selected: &crate::production_body::body_flow::book_v2::BookV2TableFragmentSelection<
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        definition: Option<usize>,
    ) -> Result<(), ProductionBodyPaginationError> {
        selected.verify_source_flow(self.content.flow, definition)?;
        let index = selected.before().table_index();
        let table = &self.content.flow.collected.tables.tables[index];
        let owner = table.owner;
        if table.parent.is_some() || table.definition != definition {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let width = self.physical_table_parent_width(page, index)?;
        let mut count = 0usize;
        for range in selected.source_leaf_ranges() {
            self.content.step(owner)?;
            count = count
                .checked_add(range?.len())
                .ok_or_else(|| error(owner, E::FragmentLimit))?;
        }
        if selected.has_header_variants() {
            for leaf in selected.variant_placement_leaves() {
                self.content.step(owner)?;
                if leaf?.repeated() {
                    count = count
                        .checked_add(1)
                        .ok_or_else(|| error(owner, E::FragmentLimit))?;
                }
            }
        } else {
            for leaf in selected.source_placement_leaves() {
                self.content.step(owner)?;
                if leaf?.repeated_header() {
                    count = count
                        .checked_add(1)
                        .ok_or_else(|| error(owner, E::FragmentLimit))?;
                }
            }
        }
        self.content.charge.take(count, owner)?;
        let mut pieces = Vec::new();
        pieces
            .try_reserve_exact(count)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        let mut occurrence = BookV2TableWidthOccurrence {
            owner,
            page: page.page_index(),
            definition,
            parent_width: width,
            pieces,
        };
        let mut source_owners = Vec::new();
        if result.remeasured {
            self.content.charge.take(count, owner)?;
            source_owners
                .try_reserve_exact(count)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            let flow = self.content.flow;
            let items = match definition {
                Some(d) => flow
                    .definition_items(d)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                None => flow.body_items(),
            };
            for range in selected.source_leaf_ranges() {
                self.content.step(owner)?;
                for index in range? {
                    self.content.step(owner)?;
                    source_owners.push(
                        items
                            .get(index)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                            .owner(),
                    );
                }
            }
            if selected.has_header_variants() {
                for leaf in selected.variant_placement_leaves() {
                    self.content.step(owner)?;
                    let leaf = leaf?;
                    if leaf.repeated() {
                        source_owners.push(
                            leaf.measurements()
                                .item(leaf.global_item_index())
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                                .owner(),
                        );
                    }
                }
            } else {
                for leaf in selected.source_placement_leaves() {
                    self.content.step(owner)?;
                    let leaf = leaf?;
                    if leaf.repeated_header() {
                        source_owners.push(
                            items
                                .get(leaf.item_index())
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                                .owner(),
                        );
                    }
                }
            }
            if source_owners.len() != count {
                return Err(error(owner, E::ReceiptMismatch));
            }
            let sorting = (count as u64)
                .checked_mul(u64::from(count.checked_ilog2().unwrap_or(0)) + 2)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            self.content.steps = self
                .content
                .steps
                .checked_add(sorting)
                .filter(|n| *n <= self.content.maximum_steps)
                .ok_or_else(|| error(owner, E::FootnoteSearchLimit))?;
            source_owners.sort_unstable();
            source_owners.dedup();
        }
        let frames = if result.remeasured {
            let original = self
                .content
                .flow
                .lines()
                .frames()
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let (records, work) = original
                .table_source_occurrence_projection_budget()
                .map_err(table_frame_error)?;
            let prior = self.record_charge();
            self.content.charge.take(
                usize::try_from(records).map_err(|_| error(owner, E::FragmentLimit))?,
                owner,
            )?;
            self.content.steps = self
                .content
                .steps
                .checked_add(work)
                .filter(|n| *n <= self.content.maximum_steps)
                .ok_or_else(|| error(owner, E::FootnoteSearchLimit))?;
            Some(
                original
                    .remeasure_table_parent_for_sources(owner, width, &source_owners, work, prior)
                    .map_err(table_frame_error)?,
            )
        } else {
            None
        };
        for range in selected.source_leaf_ranges() {
            self.content.step(owner)?;
            for item in range? {
                self.append_table_width_piece(
                    &mut occurrence,
                    self.content.flow,
                    item,
                    false,
                    false,
                    frames.as_ref(),
                )?;
            }
        }
        // Repeated header/caption pieces are observations, not a second semantic
        // consumption. Keep them separate even if they have identical text.
        if selected.has_header_variants() {
            for leaf in selected.variant_placement_leaves() {
                self.content.step(owner)?;
                let leaf = leaf?;
                if leaf.repeated() {
                    let flow = leaf.measurements().flow();
                    let region = definition
                        .map_or_else(
                            || Some(0..flow.collected.body_end),
                            |d| flow.collected.definitions.get(d).cloned(),
                        )
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    if !region.contains(&leaf.global_item_index()) {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    self.append_table_width_piece(
                        &mut occurrence,
                        flow,
                        leaf.global_item_index() - region.start,
                        true,
                        leaf.uses_header_variant(),
                        frames.as_ref(),
                    )?;
                }
            }
        } else {
            for leaf in selected.source_placement_leaves() {
                self.content.step(owner)?;
                let leaf = leaf?;
                if leaf.repeated_header() {
                    self.append_table_width_piece(
                        &mut occurrence,
                        self.content.flow,
                        leaf.item_index(),
                        true,
                        false,
                        frames.as_ref(),
                    )?;
                }
            }
        }
        for value in [
            u64::from(owner.get()),
            u64::from(page.page_index()),
            definition.map_or(u64::MAX, |n| n as u64),
            width.get().raw() as u64,
            occurrence.pieces.len() as u64,
        ] {
            self.fold_table_occurrence(&mut result.fingerprint, value, owner)?;
        }
        for piece in &occurrence.pieces {
            if piece.header_variant {
                self.fold_table_occurrence(&mut result.fingerprint, 0x4844525631, owner)?;
                self.fold_table_occurrence(
                    &mut result.fingerprint,
                    u64::from(piece.measured_frame.is_some()),
                    owner,
                )?;
                if let Some(frame) = piece.measured_frame {
                    self.fold_table_occurrence(
                        &mut result.fingerprint,
                        frame.0.raw() as u64,
                        owner,
                    )?;
                    self.fold_table_occurrence(
                        &mut result.fingerprint,
                        frame.1.get().raw() as u64,
                        owner,
                    )?;
                }
            }
            let (kind, start, end) = match &piece.source {
                BookV2TableWidthSource::Paragraph { units, .. } => (0, units.start, units.end),
                BookV2TableWidthSource::Block { .. } => (1, 0, 0),
                BookV2TableWidthSource::ForcedBreak { .. } => (2, 0, 0),
            };
            for value in [
                kind,
                u64::from(piece.source.owner().get()),
                u64::from(start),
                u64::from(end),
                u64::from(piece.repeated),
            ] {
                self.fold_table_occurrence(&mut result.fingerprint, value, owner)?;
            }
        }
        if result.remeasured {
            for piece in &occurrence.pieces {
                self.fold_table_occurrence(
                    &mut result.fingerprint,
                    u64::from(piece.frame.is_some()),
                    owner,
                )?;
                if let Some(frame) = piece.frame {
                    self.fold_table_occurrence(
                        &mut result.fingerprint,
                        frame.start().raw() as u64,
                        owner,
                    )?;
                    self.fold_table_occurrence(
                        &mut result.fingerprint,
                        frame.width().get().raw() as u64,
                        owner,
                    )?;
                }
            }
        }
        result.occurrences.push(occurrence);
        Ok(())
    }
    fn append_table_width_piece(
        &mut self,
        occurrence: &mut BookV2TableWidthOccurrence,
        flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
        index: usize,
        repeated: bool,
        header_variant: bool,
        frames: Option<&typaxis_layout::book_v2::BookV2TableOccurrenceFrames<'_, '_, '_>>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = occurrence.owner;
        self.content.step(root)?;
        let items = match occurrence.definition {
            Some(d) => flow
                .definition_items(d)
                .ok_or_else(|| error(root, E::ReceiptMismatch))?,
            None => flow.body_items(),
        };
        let item = items
            .get(index)
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let owner = item.owner();
        let mut frame = None;
        let mut measured_frame = None;
        if header_variant && !repeated {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let source = match item.source() {
            Some(ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            }) => {
                let p = flow
                    .lines()
                    .paragraphs()
                    .get(paragraph_index as usize)
                    .filter(|p| p.owner() == owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let line = p
                    .selected()
                    .and_then(|s| s.lines().get(line_index as usize))
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                if header_variant && frames.is_some() {
                    let actual = flow
                        .lines()
                        .frames()
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let start = actual
                        .source_unit_start(paragraph_index as usize, line.line().start_unit())
                        .unwrap_or(actual.paragraphs()[paragraph_index as usize].start());
                    measured_frame = Some((start, line.inline_size()));
                }
                let line = line.line();
                if let Some(frames) = frames {
                    for _ in 0..frames.source_owner_lookup_work() {
                        self.content.step(owner)?;
                    }
                    frame = Some(
                        frames
                            .paragraph(paragraph_index as usize, owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                    );
                }
                BookV2TableWidthSource::Paragraph {
                    owner,
                    units: line.start_unit()..line.end_unit(),
                }
            }
            Some(_) => {
                if header_variant && frames.is_some() {
                    let actual = flow
                        .lines()
                        .frames()
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    for _ in 0..actual.inherited_block_lookup_work() {
                        self.content.step(owner)?;
                    }
                    let actual = actual
                        .region(owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    measured_frame = Some((actual.start(), actual.width()));
                }
                if let Some(frames) = frames {
                    for _ in 0..frames.region_lookup_work() {
                        self.content.step(owner)?;
                    }
                    frame = Some(
                        frames
                            .region(owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                    );
                }
                BookV2TableWidthSource::Block { owner }
            }
            None => {
                if repeated {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                BookV2TableWidthSource::ForcedBreak { owner }
            }
        };
        occurrence.pieces.push(BookV2TableWidthPiece {
            source,
            repeated,
            header_variant,
            measured_frame,
            frame,
        });
        Ok(())
    }
}

fn table_frame_error(
    cause: typaxis_layout::ProductionInlinePreparationError,
) -> ProductionBodyPaginationError {
    use typaxis_layout::ProductionInlinePreparationErrorKind as L;
    error(
        cause.owner,
        match cause.kind {
            L::ReceiptMismatch => E::ReceiptMismatch,
            L::AllocationFailure => E::AllocationFailure,
            L::UnitLimit => E::FragmentLimit,
            L::ArithmeticOverflow => E::ArithmeticOverflow,
            L::Atomic(_) => E::FootnoteSearchLimit,
            _ => E::WidthMismatch,
        },
    )
}
