//! Formula occurrences over the same closed physical column geometry.
use super::math_terminals::{
    NUMBER_BYTES, TERMINAL_BYTES, VARIANT_BYTES, VARIANT_HEADER_BYTES, encode_number, encode_rect,
    encode_terminal,
};
use super::*;

pub const BOOK_V2_COLUMN_MATH_TERMINAL_ALGORITHM: &str = "typaxis.book-2-column-math-terminals/1";
const HEADER_BYTES: usize = 9 * 32 + 6 * 8;
const PAGE_BYTES: usize = 152;
const COLUMN_BYTES: usize = 48;
const COLUMN_TERMINAL_BYTES: usize = TERMINAL_BYTES + 3;
const BALANCE_HEADER_BYTES: usize = 40;
const BALANCE_PAGE_BYTES: usize = 44;

/// A physical column occurrence is distinct from a single-body terminal.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnMathTerminal, BookV2BodyMathTerminal};
/// fn single<'r, 'a>(v: BookV2ColumnMathTerminal<'r, 'a>) -> BookV2BodyMathTerminal<'r, 'a> { v }
/// ```
pub struct BookV2ColumnMathTerminal<'r, 'a> {
    inner: BookV2BodyMathTerminal<'r, 'a>,
    column: Option<u16>,
}
impl<'r, 'a> BookV2ColumnMathTerminal<'r, 'a> {
    pub fn source(&self) -> BookV2BodyMathSource<'r, 'a> {
        self.inner.source()
    }
    pub fn page_index(&self) -> u32 {
        self.inner.page_index()
    }
    pub fn column_index(&self) -> Option<u16> {
        self.column
    }
    pub fn fragment_index(&self) -> usize {
        self.inner.fragment_index()
    }
    pub fn definition_index(&self) -> Option<usize> {
        self.inner.definition_index()
    }
    pub fn item_index(&self) -> usize {
        self.inner.item_index()
    }
    pub fn inline_index(&self) -> Option<u32> {
        self.inner.inline_index()
    }
    pub fn cell_owner(&self) -> Option<NodeId> {
        self.inner.cell_owner()
    }
    pub fn repeated_header(&self) -> bool {
        self.inner.repeated_header()
    }
    pub fn origin_x(&self) -> Length {
        self.inner.origin_x()
    }
    pub fn baseline(&self) -> Length {
        self.inner.baseline()
    }
    pub fn viewport(&self) -> Option<Rect> {
        self.inner.viewport()
    }
}

/// Binds source coverage, original formula receipts and actual column regions.
/// This result does not authorize a mixed display or PDF consumer.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnMathTerminals, BookV2BodyMathTerminals};
/// fn single<'g, 'q, 'b, 'f, 's, 'p, 'a>(v: BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>)
///     -> BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: BookV2ColumnSourceClosure<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    terminals: Vec<BookV2ColumnMathTerminal<'s, 'p>>,
    fragment_count: usize,
    fragment_flows: Vec<&'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>>,
    canonical: Vec<u8>,
    fingerprint: [u8; 32],
    records: u64,
    work: u64,
    spool: u64,
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn source(&self) -> &BookV2ColumnSourceClosure<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.source
    }
    pub fn terminals(&self) -> &[BookV2ColumnMathTerminal<'s, 'p>] {
        &self.terminals
    }
    /// Borrow original data even when the closed document has no fragments.
    pub fn original_graph(&self) -> BookV2SourceGraph<'b, 'f, 's, 'p, 'a> {
        self.source.original_graph()
    }
    /// Resolve the real header graph using the retained occurrence cache.
    pub fn fragment_graph(&self, index: usize) -> Option<BookV2SourceGraph<'b, 'f, 's, 'p, 'a>> {
        if index >= self.fragment_count { return None; }
        if self.fragment_flows.is_empty() {
            Some(self.original_graph())
        } else {
            self.fragment_flows.get(index).copied().map(BookV2SourceGraph::new)
        }
    }
    pub fn fragment_lines(
        &self,
        index: usize,
    ) -> Option<&'s typaxis_layout::book_v2::BookV2InlineLineLayout<'p, 'a>> {
        self.fragment_graph(index).map(|graph| graph.lines())
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

impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a> {
    pub fn terminal_spool_charge(&self) -> u64 {
        self.inner.terminal_spool_charge()
    }

    /// Consume the same closed stable columns. Selected-width feedback is
    /// checked again before reserving terminal storage; retries retain charges.
    pub fn finalize_column_page_math<'g, 'q>(
        &mut self,
        source: BookV2ColumnSourceClosure<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        prior_spool: u64,
    ) -> Result<BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        let root = NodeId::new(0);
        self.verify_sequence(source.stable().sequence())?;
        self.verify_sequence(source.geometry().sequence())?;
        let flow = self.inner.content.flow;
        if !std::ptr::eq(source.geometry().sequence(), source.stable().sequence())
            || !std::ptr::eq(flow, source.flow())
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)?;
        let widths = self.paragraph_frame_feedback(source.stable().sequence())?;
        if !widths.matches_selected_line_widths() || !widths.matches_selected_block_widths() {
            return Err(error(root, E::WidthMismatch));
        }
        // The immutable plan derives from this exact original styled package.
        // Domain separation binds authored masters, rules and column requests.
        self.inner.content.step(root)?;
        let mut plan_bytes = [0u8; 64];
        plan_bytes[..32].copy_from_slice(&sha256(b"typaxis.book-2-column-source-plan/1"));
        plan_bytes[32..].copy_from_slice(&self.plan.source().body().canonical_jcs_sha256());
        let plan_fingerprint = sha256(&plan_bytes);
        let prepared = flow.lines().prepared();
        let count = source
            .semantic_math()
            .checked_add(source.repeated_math())
            .ok_or_else(|| error(root, E::FragmentLimit))?;
        let mut numbers = 0usize;
        let mut fragment_count = 0usize;
        let mut columns = 0usize;
        let mut balanced_pages = 0usize;
        for page in source.geometry().pages() {
            self.inner.content.step(root)?;
            fragment_count = fragment_count
                .checked_add(page.fragments().len())
                .ok_or_else(|| error(root, E::FragmentLimit))?;
            numbers = numbers
                .checked_add(page.equation_numbers().len())
                .ok_or_else(|| error(root, E::FragmentLimit))?;
            columns = columns
                .checked_add(page.columns().len())
                .ok_or_else(|| error(root, E::FragmentLimit))?;
            if page.selection().balance().is_some() {
                balanced_pages = balanced_pages.checked_add(1)
                    .ok_or_else(|| error(root, E::FragmentLimit))?;
            }
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
        let balance_bytes = if balanced_pages == 0 { 0 } else {
            balanced_pages.checked_mul(BALANCE_PAGE_BYTES)
                .and_then(|n| n.checked_add(BALANCE_HEADER_BYTES))
                .ok_or_else(|| error(root, E::SpoolLimit))?
        };
        let bytes = count
            .checked_mul(COLUMN_TERMINAL_BYTES)
            .and_then(|n| {
                numbers
                    .checked_mul(NUMBER_BYTES)
                    .and_then(|m| n.checked_add(m))
            })
            .and_then(|n| {
                source
                    .geometry()
                    .pages()
                    .len()
                    .checked_mul(PAGE_BYTES)
                    .and_then(|m| n.checked_add(m))
            })
            .and_then(|n| {
                columns
                    .checked_mul(COLUMN_BYTES)
                    .and_then(|m| n.checked_add(m))
            })
            .and_then(|n| n.checked_add(HEADER_BYTES))
            .and_then(|n| n.checked_add(variant_bytes))
            .and_then(|n| n.checked_add(balance_bytes))
            .ok_or_else(|| error(root, E::SpoolLimit))?;
        self.inner.reserve_math_terminal_spool(
            limits,
            prior_spool,
            prepared.native_math().map_or(0, |n| n.spool_charge()),
            bytes,
        )?;
        self.inner.content.charge.take(
            count
                .checked_add(2)
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut fragment_flows = Vec::new();
        if source.has_header_variants() {
            self.inner.content.charge.take(
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
            sha256(BOOK_V2_COLUMN_MATH_TERMINAL_ALGORITHM.as_bytes()),
            limits.fingerprint(),
            flow.lines().fingerprint(),
            flow.blocks().map_or([0; 32], |b| b.fingerprint()),
            source.stable().sequence().measurements_fingerprint(),
            prepared
                .vector_bindings()
                .map_or([0; 32], |b| b.fingerprint()),
            prepared.native_math().map_or([0; 32], |n| n.fingerprint()),
            plan_fingerprint,
            widths.assignment_fingerprint(),
        ] {
            canonical.extend_from_slice(&hash);
        }
        for n in [
            source.geometry().pages().len(),
            source.semantic_math(),
            source.repeated_math(),
            source.unreferenced_definitions(),
            numbers,
            fragment_count,
        ] {
            canonical.extend_from_slice(&(n as u64).to_be_bytes());
        }
        let mut fragment_index = 0usize;
        let mut semantic = 0usize;
        let mut repeated = 0usize;
        for (page_index, page) in source.geometry().pages().iter().enumerate() {
            self.inner.content.step(root)?;
            for (local_fragment, (placed, role, is_repeated)) in
                page.fragments_with_roles().enumerate()
            {
                let owner = placed.fragment().owner();
                self.inner.content.step(owner)?;
                self.inner.source_lookup_work(page.columns().len(), owner)?;
                let column = page.fragment_column(local_fragment);
                if column.is_none() != placed.definition_index().is_some() {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                let actual_flow = if page.source_header_variants().is_empty() {
                    flow
                } else {
                    self.inner
                        .source_lookup_work(page.source_header_variants().len(), owner)?;
                    source
                        .fragment_flow(page_index, local_fragment)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                };
                if source.has_header_variants() {
                    fragment_flows.push(actual_flow);
                }
                self.inner.collect_fragment_math(
                    actual_flow,
                    placed,
                    role,
                    is_repeated,
                    fragment_index,
                    |terminal| {
                        if terminals.len() >= count {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                        if is_repeated {
                            repeated += 1;
                        } else {
                            semantic += 1;
                        }
                        terminals.push(BookV2ColumnMathTerminal {
                            inner: terminal,
                            column,
                        });
                        Ok(())
                    },
                )?;
                fragment_index = fragment_index
                    .checked_add(1)
                    .ok_or_else(|| error(owner, E::FragmentLimit))?;
            }
        }
        if semantic != source.semantic_math()
            || repeated != source.repeated_math()
            || terminals.len() != count
            || fragment_index != fragment_count
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        for terminal in &terminals {
            self.inner.content.step(terminal.source().owner())?;
            encode_terminal(&terminal.inner, &mut canonical);
            canonical.push(u8::from(terminal.column.is_some()));
            canonical.extend_from_slice(&terminal.column.unwrap_or(0).to_be_bytes());
        }
        let mut base_fragment = 0usize;
        for page in source.geometry().pages() {
            self.inner.content.step(root)?;
            for number in page.equation_numbers() {
                let n = number.geometry();
                self.inner.content.step(n.owner())?;
                let local = (n.fragment_index() as usize)
                    .checked_sub(base_fragment)
                    .ok_or_else(|| error(n.owner(), E::ReceiptMismatch))?;
                let fragment = page
                    .fragments()
                    .get(local)
                    .filter(|p| {
                        p.fragment().owner() == n.parent_owner()
                            && p.fragment().page_index() == n.page_index()
                    })
                    .ok_or_else(|| error(n.owner(), E::ReceiptMismatch))?;
                self.inner.content.step(fragment.fragment().owner())?;
                encode_number(number, &mut canonical);
            }
            base_fragment = base_fragment
                .checked_add(page.fragments().len())
                .ok_or_else(|| error(root, E::FragmentLimit))?;
        }
        if source.has_header_variants() {
            canonical.extend_from_slice(&sha256(b"typaxis.book-2-math-terminal-header-owners/1"));
            canonical.extend_from_slice(&(source.header_variant_fragments() as u64).to_be_bytes());
            let mut written = 0usize;
            for page in source.geometry().pages() {
                self.inner.content.step(root)?;
                for variant in page.source_header_variants() {
                    self.inner.content.step(root)?;
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
        for page in source.geometry().pages() {
            self.inner.content.step(root)?;
            let frames = page.selection().candidate().frames();
            canonical.extend_from_slice(&frames.page_index().to_be_bytes());
            canonical.push(u8::from(frames.named_page_index().is_some()));
            canonical
                .extend_from_slice(&(frames.named_page_index().unwrap_or(0) as u64).to_be_bytes());
            encode_rect(frames.body(), &mut canonical);
            encode_optional_rect(frames.footnote(), &mut canonical);
            encode_optional_rect(
                page.selection().candidate().footnote_bounds(),
                &mut canonical,
            );
            canonical.extend_from_slice(&(page.columns().len() as u64).to_be_bytes());
            encode_optional_rect(page.separator_ink(), &mut canonical);
            for column in page.columns() {
                self.inner.content.step(root)?;
                encode_rect(column.bounds(), &mut canonical);
                canonical.extend_from_slice(&(column.fragments().start as u64).to_be_bytes());
                canonical.extend_from_slice(&(column.fragments().end as u64).to_be_bytes());
            }
        }
        if balanced_pages > 0 {
            canonical.extend_from_slice(&sha256(b"typaxis.book-2-column-math-balance/1"));
            canonical.extend_from_slice(&(balanced_pages as u64).to_be_bytes());
            let mut written = 0usize;
            for page in source.geometry().pages() {
                self.inner.content.step(root)?;
                if let Some(balance) = page.selection().balance() {
                    canonical.extend_from_slice(&page.selection().page_index().to_be_bytes());
                    canonical.extend_from_slice(&balance.fingerprint());
                    canonical.extend_from_slice(&balance.selected_height().get().raw().to_be_bytes());
                    written = written.checked_add(1)
                        .ok_or_else(|| error(root, E::FragmentLimit))?;
                }
            }
            if written != balanced_pages { return Err(error(root, E::ReceiptMismatch)); }
        }
        if canonical.len() != bytes {
            return Err(error(root, E::ReceiptMismatch));
        }
        for _ in 0..canonical.len().div_ceil(64) {
            self.inner.content.step(root)?;
        }
        let fingerprint = sha256(&canonical);
        Ok(BookV2ColumnMathTerminals {
            source,
            terminals,
            fragment_count,
            fragment_flows,
            canonical,
            fingerprint,
            records: self.record_charge(),
            work: self.work_steps(),
            spool: self.terminal_spool_charge(),
        })
    }
}
fn encode_optional_rect(rect: Option<Rect>, out: &mut Vec<u8>) {
    out.push(u8::from(rect.is_some()));
    if let Some(rect) = rect {
        encode_rect(rect, out);
    } else {
        out.extend_from_slice(&[0; 32]);
    }
}
