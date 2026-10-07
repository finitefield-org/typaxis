//! Successor math placement records over the exact verified source geometry.
use super::*;
use typaxis_layout::book_v2::{BookV2BoundVector, BookV2MathReceipt};
use typaxis_layout::{PrecomposedVectorPlacementInput, ProductionPlacedInline};
use typaxis_syntax::PrecomposedVectorKind;

pub const BOOK_V2_BODY_MATH_TERMINAL_ALGORITHM: &str = "typaxis.book-2-body-math-terminals/1";
const HEADER_BYTES: usize = 7 * 32 + 5 * 8;
const TERMINAL_BYTES: usize = 126;
const NUMBER_BYTES: usize = 81;
const VARIANT_HEADER_BYTES: usize = 32 + 8;
const VARIANT_BYTES: usize = 4 + 8 + 8 + 5 * 32;

#[derive(Clone, Copy, Debug)]
pub enum BookV2BodyMathSource<'r, 'a> {
    Vector(&'r BookV2BoundVector<'a>),
    Native(&'r BookV2MathReceipt<'a>),
}
impl BookV2BodyMathSource<'_, '_> {
    pub fn owner(self) -> NodeId {
        match self {
            Self::Vector(v) => v.node_id(),
            Self::Native(v) => v.node_id(),
        }
    }
    pub fn fingerprint(self) -> [u8; 32] {
        match self {
            Self::Vector(v) => v.fingerprint(),
            Self::Native(v) => v.fingerprint(),
        }
    }
}

/// All positions are physical page coordinates. Native glyph/rule positions
/// remain local to origin_x/baseline; their actual receipt supplies dimensions.
#[derive(Debug)]
pub struct BookV2BodyMathTerminal<'r, 'a> {
    source: BookV2BodyMathSource<'r, 'a>,
    page: u32,
    fragment: usize,
    definition: Option<usize>,
    item: usize,
    inline: Option<u32>,
    cell: Option<NodeId>,
    repeated: bool,
    origin_x: Length,
    baseline: Length,
    viewport: Option<Rect>,
}
impl<'r, 'a> BookV2BodyMathTerminal<'r, 'a> {
    pub fn source(&self) -> BookV2BodyMathSource<'r, 'a> {
        self.source
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
    pub fn inline_index(&self) -> Option<u32> {
        self.inline
    }
    pub fn cell_owner(&self) -> Option<NodeId> {
        self.cell
    }
    pub fn repeated_header(&self) -> bool {
        self.repeated
    }
    pub fn origin_x(&self) -> Length {
        self.origin_x
    }
    pub fn baseline(&self) -> Length {
        self.baseline
    }
    pub fn viewport(&self) -> Option<Rect> {
        self.viewport
    }
}

pub struct BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: BookV2BodySourceClosure<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    terminals: Vec<BookV2BodyMathTerminal<'s, 'p>>,
    fragment_count: usize,
    fragment_flows: Vec<&'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>>,
    canonical: Vec<u8>,
    fingerprint: [u8; 32],
    records: u64,
    work: u64,
    spool: u64,
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn source(&self) -> &BookV2BodySourceClosure<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.source
    }
    pub fn terminals(&self) -> &[BookV2BodyMathTerminal<'s, 'p>] {
        &self.terminals
    }
    /// Resolve a global physical fragment index without rescanning prior pages.
    pub fn fragment_flow(
        &self,
        index: usize,
    ) -> Option<&'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>> {
        if index >= self.fragment_count {
            return None;
        }
        if self.fragment_flows.is_empty() {
            Some(self.source.flow())
        } else {
            self.fragment_flows.get(index).copied()
        }
    }
    pub fn fragment_count(&self) -> usize {
        self.fragment_count
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
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
    pub fn spool_charge(&self) -> u64 {
        self.spool
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn terminal_spool_charge(&self) -> u64 {
        self.terminal_spool
    }

    /// Serialize physical formula occurrences, including explicit header copies.
    /// prior_spool includes preceding caller-owned spool; native computation
    /// storage is a mandatory lower bound. Repeated calls never refund storage.
    pub fn finalize_mixed_page_math<'g, 'q>(
        &mut self,
        source: BookV2BodySourceClosure<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        prior_spool: u64,
    ) -> Result<BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        let root = NodeId::new(0);
        self.verify_mixed_sequence(source.stable().sequence())?;
        let flow = self.content.flow;
        if !std::ptr::eq(flow, source.flow()) {
            return Err(error(root, E::ReceiptMismatch));
        }
        flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)?;
        if source.has_header_variants()
            || flow
                .lines()
                .frames()
                .is_some_and(|f| f.uses_table_occurrence_frames())
        {
            self.verify_table_source_frames(&source)?;
        } else if flow.lines().frames().is_some_and(|f| {
            f.has_table_width_candidates()
                || (flow.table_count() != 0
                    && f.page_plan().is_some_and(|p| p.requires_width_reflow()))
        }) {
            let (_, matches) = self.collect_root_table_width_feedback(&source)?;
            if !matches {
                return Err(error(root, E::WidthMismatch));
            }
        }
        if source.has_header_variants()
            || flow.lines().frames().is_some_and(|f| {
                f.has_table_width_candidates()
                    || f.has_block_width_candidates()
                    || f.has_source_unit_starts()
                    || f.page_plan().is_some_and(|p| p.requires_width_reflow())
            })
        {
            self.verify_physical_paragraph_widths(&source)?;
        }

        let prepared = flow.lines().prepared();
        let count = source
            .semantic_math()
            .checked_add(source.repeated_math())
            .ok_or_else(|| error(root, E::FragmentLimit))?;
        let mut numbers = 0usize;
        let mut fragment_count = 0usize;
        for page in source.geometry().pages() {
            self.content.step(root)?;
            fragment_count = fragment_count
                .checked_add(page.fragments().len())
                .ok_or_else(|| error(root, E::FragmentLimit))?;
            numbers = numbers
                .checked_add(page.equation_numbers().len())
                .ok_or_else(|| error(root, E::FragmentLimit))?;
        }
        let variant_bytes = if source.has_header_variants() {
            source
                .header_variant_fragments()
                .checked_mul(VARIANT_BYTES)
                .and_then(|n| n.checked_add(VARIANT_HEADER_BYTES))
                .ok_or_else(|| error(root, E::SpoolLimit))?
        } else {
            0
        };
        let bytes = count
            .checked_mul(TERMINAL_BYTES)
            .and_then(|n| {
                numbers
                    .checked_mul(NUMBER_BYTES)
                    .and_then(|m| n.checked_add(m))
            })
            .and_then(|n| n.checked_add(HEADER_BYTES))
            .and_then(|n| n.checked_add(variant_bytes))
            .ok_or_else(|| error(root, E::SpoolLimit))?;
        self.terminal_spool = self
            .terminal_spool
            .max(prior_spool)
            .max(prepared.native_math().map_or(0, |n| n.spool_charge()));
        self.terminal_spool = self
            .terminal_spool
            .checked_add(bytes as u64)
            .filter(|n| *n <= limits.base().get().max_spool_bytes)
            .ok_or_else(|| error(root, E::SpoolLimit))?;
        self.content.charge.take(
            count
                .checked_add(2)
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut fragment_flows = Vec::new();
        if source.has_header_variants() {
            self.content.charge.take(
                fragment_count
                    .checked_add(1)
                    .ok_or_else(|| error(root, E::FragmentLimit))?,
                root,
            )?;
            fragment_flows
                .try_reserve_exact(fragment_count)
                .map_err(|_| error(root, E::AllocationFailure))?;
        }
        let mut terminals = Vec::new();
        terminals
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let mut canonical = Vec::new();
        canonical
            .try_reserve_exact(bytes)
            .map_err(|_| error(root, E::AllocationFailure))?;
        for hash in [
            sha256(BOOK_V2_BODY_MATH_TERMINAL_ALGORITHM.as_bytes()),
            limits.fingerprint(),
            flow.lines().fingerprint(),
            flow.blocks().map_or([0; 32], |b| b.fingerprint()),
            source.stable().sequence().measurements_fingerprint(),
            prepared
                .vector_bindings()
                .map_or([0; 32], |b| b.fingerprint()),
            prepared.native_math().map_or([0; 32], |n| n.fingerprint()),
        ] {
            canonical.extend_from_slice(&hash);
        }
        for n in [
            source.geometry().pages().len(),
            source.semantic_math(),
            source.repeated_math(),
            source.unreferenced_definitions(),
            numbers,
        ] {
            canonical.extend_from_slice(&(n as u64).to_be_bytes());
        }
        let mut fragment_index = 0usize;
        let mut semantic = 0usize;
        let mut repeated = 0usize;
        for (page_index, page) in source.geometry().pages().iter().enumerate() {
            self.content.step(root)?;
            for (local_fragment, (placed, role, is_repeated)) in
                page.fragments_with_roles().enumerate()
            {
                let fragment = placed.fragment();
                let owner = fragment.owner();
                self.content.step(owner)?;
                let flow = if page.header_variants().is_empty() {
                    flow
                } else {
                    self.source_lookup_work(page.header_variants().len(), owner)?;
                    source
                        .fragment_flow(page_index, local_fragment)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                };
                if source.has_header_variants() {
                    fragment_flows.push(flow);
                }
                let prepared = flow.lines().prepared();
                let mut push = |this: &mut Self,
                                binding,
                                inline,
                                origin_x,
                                baseline,
                                viewport|
                 -> Result<(), ProductionBodyPaginationError> {
                    this.content.step(owner)?;
                    if terminals.len() >= count {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    if is_repeated {
                        repeated += 1;
                    } else {
                        semantic += 1;
                    }
                    terminals.push(BookV2BodyMathTerminal {
                        source: binding,
                        page: fragment.page_index(),
                        fragment: fragment_index,
                        definition: placed.definition_index(),
                        item: placed.item_index(),
                        inline,
                        cell: role.map(|r| r.owner()),
                        repeated: is_repeated,
                        origin_x,
                        baseline,
                        viewport,
                    });
                    Ok(())
                };
                match fragment.source() {
                    ProductionBodyFragmentSource::ParagraphLine {
                        paragraph_index,
                        line_index,
                    } => {
                        let line = &flow.lines().paragraphs()[paragraph_index as usize].lines()
                            [line_index as usize];
                        for (index, item) in line.items().iter().enumerate() {
                            self.content.step(owner)?;
                            let index = Some(
                                u32::try_from(index).map_err(|_| error(owner, E::FragmentLimit))?,
                            );
                            match item {
                                ProductionPlacedInline::Vector(selected) => {
                                    let bindings = prepared
                                        .vector_bindings()
                                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                                    self.source_lookup_work(bindings.receipts().len(), owner)?;
                                    let binding = bindings
                                        .receipt(selected.occurrence().item().node_id())
                                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                                    if binding.kind() != PrecomposedVectorKind::MathVector {
                                        continue;
                                    }
                                    let local = selected.geometry();
                                    let viewport = translated(
                                        local.viewport(),
                                        fragment.bounds().x(),
                                        fragment.bounds().y(),
                                        owner,
                                    )?;
                                    push(
                                        self,
                                        BookV2BodyMathSource::Vector(binding),
                                        index,
                                        add(fragment.bounds().x(), local.pen_origin_x(), owner)?,
                                        add(fragment.bounds().y(), local.line_baseline_y(), owner)?,
                                        Some(viewport),
                                    )?;
                                }
                                ProductionPlacedInline::BookV2Math(selected) => {
                                    push(
                                        self,
                                        BookV2BodyMathSource::Native(selected.receipt()),
                                        index,
                                        add(fragment.bounds().x(), selected.pen_x(), owner)?,
                                        add(fragment.bounds().y(), selected.baseline(), owner)?,
                                        None,
                                    )?;
                                }
                                ProductionPlacedInline::Math(_) => {
                                    return Err(error(owner, E::ReceiptMismatch))
                                }
                                _ => (),
                            }
                        }
                    }
                    ProductionBodyFragmentSource::VectorBlock { block_index } => {
                        let block = &flow
                            .blocks()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                            .blocks()[block_index as usize];
                        let binding = block.binding();
                        if let PrecomposedVectorPlacementInput::MathVectorBlock(input) =
                            binding.placement()
                        {
                            let viewport = fragment
                                .viewport()
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                            push(
                                self,
                                BookV2BodyMathSource::Vector(binding),
                                None,
                                viewport
                                    .x()
                                    .checked_sub(input.metrics().origin_x())
                                    .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
                                fragment
                                    .baseline()
                                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                                Some(viewport),
                            )?;
                        }
                    }
                    ProductionBodyFragmentSource::NativeMathBlock { block_index } => {
                        let native = prepared
                            .native_math()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        let block = &native.display_blocks()[block_index as usize];
                        self.source_lookup_work(native.receipts().len(), owner)?;
                        let receipt = native
                            .receipt(owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        let viewport = fragment
                            .viewport()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        push(
                            self,
                            BookV2BodyMathSource::Native(receipt),
                            None,
                            viewport
                                .x()
                                .checked_sub(block.left())
                                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
                            fragment
                                .baseline()
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                            None,
                        )?;
                    }
                    ProductionBodyFragmentSource::Figure { .. } => (),
                }
                fragment_index = fragment_index
                    .checked_add(1)
                    .ok_or_else(|| error(owner, E::FragmentLimit))?;
            }
        }
        if semantic != source.semantic_math()
            || repeated != source.repeated_math()
            || terminals.len() != count
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        for terminal in &terminals {
            self.content.step(terminal.source.owner())?;
            encode_terminal(terminal, &mut canonical);
        }
        for page in source.geometry().pages() {
            self.content.step(root)?;
            for number in page.equation_numbers() {
                self.content.step(number.geometry().owner())?;
                encode_number(number, &mut canonical);
            }
        }
        if source.has_header_variants() {
            canonical.extend_from_slice(&sha256(b"typaxis.book-2-math-terminal-header-owners/1"));
            canonical.extend_from_slice(&(source.header_variant_fragments() as u64).to_be_bytes());
            let mut written = 0usize;
            for page in source.geometry().pages() {
                self.content.step(root)?;
                for variant in page.header_variants() {
                    self.content.step(root)?;
                    let flow = variant.measurements().flow();
                    let prepared = flow.lines().prepared();
                    canonical.extend_from_slice(&page.selection().page_index().to_be_bytes());
                    canonical.extend_from_slice(&(variant.fragment_index() as u64).to_be_bytes());
                    canonical
                        .extend_from_slice(&(variant.global_item_index() as u64).to_be_bytes());
                    for hash in [
                        variant.header().fingerprint(),
                        flow.lines().fingerprint(),
                        flow.blocks().map_or([0; 32], |b| b.fingerprint()),
                        prepared
                            .vector_bindings()
                            .map_or([0; 32], |b| b.fingerprint()),
                        prepared.native_math().map_or([0; 32], |n| n.fingerprint()),
                    ] {
                        canonical.extend_from_slice(&hash);
                    }
                    written = written
                        .checked_add(1)
                        .ok_or_else(|| error(root, E::FragmentLimit))?;
                }
            }
            if written != source.header_variant_fragments() {
                return Err(error(root, E::ReceiptMismatch));
            }
        }
        if canonical.len() != bytes {
            return Err(error(root, E::ReceiptMismatch));
        }
        // Hashing the fixed-size encoding is charged in 64-byte blocks.
        for _ in 0..canonical.len().div_ceil(64) {
            self.content.step(root)?;
        }
        let fingerprint = sha256(&canonical);
        Ok(BookV2BodyMathTerminals {
            source,
            terminals,
            fragment_count,
            fragment_flows,
            canonical,
            fingerprint,
            records: self.record_charge(),
            work: self.work_steps(),
            spool: self.terminal_spool,
        })
    }
}

fn translated(
    rect: Rect,
    x: Length,
    y: Length,
    owner: NodeId,
) -> Result<Rect, ProductionBodyPaginationError> {
    Ok(Rect::new(
        add(rect.x(), x, owner)?,
        add(rect.y(), y, owner)?,
        rect.width(),
        rect.height(),
    ))
}
fn encode_rect(rect: Rect, out: &mut Vec<u8>) {
    for value in [rect.x(), rect.y(), rect.width().get(), rect.height().get()] {
        out.extend_from_slice(&value.raw().to_be_bytes());
    }
}
fn encode_terminal(t: &BookV2BodyMathTerminal<'_, '_>, out: &mut Vec<u8>) {
    out.push(match t.source {
        BookV2BodyMathSource::Vector(_) => 0,
        BookV2BodyMathSource::Native(_) => 1,
    });
    out.extend_from_slice(&t.source.owner().get().to_be_bytes());
    out.extend_from_slice(&t.page.to_be_bytes());
    out.extend_from_slice(&(t.fragment as u64).to_be_bytes());
    out.push(u8::from(t.definition.is_some()));
    out.extend_from_slice(&(t.definition.unwrap_or(0) as u64).to_be_bytes());
    out.extend_from_slice(&(t.item as u64).to_be_bytes());
    out.push(u8::from(t.inline.is_some()));
    out.extend_from_slice(&t.inline.unwrap_or(0).to_be_bytes());
    out.push(u8::from(t.cell.is_some()));
    out.extend_from_slice(&t.cell.map_or(0, |c| c.get()).to_be_bytes());
    out.push(u8::from(t.repeated));
    out.extend_from_slice(&t.origin_x.raw().to_be_bytes());
    out.extend_from_slice(&t.baseline.raw().to_be_bytes());
    out.push(u8::from(t.viewport.is_some()));
    if let Some(rect) = t.viewport {
        encode_rect(rect, out);
    } else {
        out.extend_from_slice(&[0; 32]);
    }
    out.extend_from_slice(&t.source.fingerprint());
}
fn encode_number(number: &BookV2BodyPlacedEquationNumber, out: &mut Vec<u8>) {
    let n = number.geometry();
    for value in [
        n.owner().get(),
        n.parent_owner().get(),
        n.fragment_index(),
        n.page_index(),
    ] {
        out.extend_from_slice(&value.to_be_bytes());
    }
    encode_rect(n.bounds(), out);
    out.extend_from_slice(&n.shape_fingerprint());
    out.push(u8::from(number.repeated_header()));
}
