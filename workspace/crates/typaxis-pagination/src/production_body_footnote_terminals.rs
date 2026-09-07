//! Atomic math completion bound to stable joint physical geometry.
use super::*;
use typaxis_layout::{StagingMathVectorFlowRegistry, StagingMathVectorTerminalReceiptSet};

pub struct ProductionBodyFootnoteMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    geometry: &'g ProductionBodyFootnotePlacedSequence<'q, 'b, 'f, 's, 'p, 'a>,
    terminals: StagingMathVectorTerminalReceiptSet,
    numbers: Vec<ProductionBodyEquationNumber>,
    spool_bytes: u64,
    flow: &'b ProductionPreparedBodyFlow<'f, 's, 'p, 'a>,
    registry: &'g StagingMathVectorFlowRegistry,
    record_charge: u64,
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> ProductionBodyFootnoteMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn line_layout(&self) -> &'s typaxis_layout::ProductionInlineLineLayout<'p, 'a> {
        self.flow.lines
    }
    pub fn block_layout(&self) -> &'s typaxis_layout::StagingPrecomposedVectorBlockLayout {
        self.flow.blocks
    }
    pub fn footnote_lines(&self) -> &typaxis_layout::ProductionFootnoteLines<'s, 'p, 'a> {
        self.flow.footnotes()
    }
    pub fn registry(&self) -> &'g StagingMathVectorFlowRegistry {
        self.registry
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn geometry(&self) -> &'g ProductionBodyFootnotePlacedSequence<'q, 'b, 'f, 's, 'p, 'a> {
        self.geometry
    }
    pub fn terminals(&self) -> &StagingMathVectorTerminalReceiptSet {
        &self.terminals
    }
    /// Fragment indices address the flattened page/content sequence.
    pub fn equation_numbers(&self) -> &[ProductionBodyEquationNumber] {
        &self.numbers
    }
    pub fn spool_bytes(&self) -> u64 {
        self.spool_bytes
    }
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn terminal_spool_charge(&self) -> u64 {
        self.terminal_spool
    }
    pub fn finalize_page_math<'g, 'q>(
        &mut self,
        stable: &ProductionBodyFootnoteStablePages<'b, 'f, 's, 'p, 'a>,
        geometry: &'g ProductionBodyFootnotePlacedSequence<'q, 'b, 'f, 's, 'p, 'a>,
        registry: &'g StagingMathVectorFlowRegistry,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<
        ProductionBodyFootnoteMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        ProductionBodyPaginationError,
    > {
        let root = NodeId::new(0);
        self.verify_sequence(stable.sequence())?;
        let flow = self.content.flow;
        if !std::ptr::eq(geometry.sequence(), stable.sequence())
            || flow.limits_fingerprint != limits.fingerprint()
            || flow.blocks.receipt().math_flow_registry_fingerprint()
                != registry.receipt().fingerprint()
            || flow.blocks.receipt().layout_epoch_fingerprint()
                != registry.receipt().layout_epoch_fingerprint()
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        let count = registry.flows().len();
        let records = count
            .checked_mul(4)
            .and_then(|n| n.checked_add(2))
            .and_then(|n| n.checked_add(registry.equation_number_shapes().len()))
            .ok_or_else(|| error(root, E::FragmentLimit))?;
        self.content.charge.take(records, root)?;
        let mut integrity = registry.receipt().canonical_jcs().len() as u64;
        for shape in registry.equation_number_shapes() {
            self.step(root)?;
            integrity = integrity
                .checked_add(shape.canonical_jcs().len() as u64)
                .ok_or_else(|| error(root, E::SpoolLimit))?;
        }
        let peak = (count as u64)
            .checked_mul(4096)
            .and_then(|n| n.checked_add(4096))
            .and_then(|n| integrity.checked_mul(4).and_then(|m| n.checked_add(m)))
            .ok_or_else(|| error(root, E::SpoolLimit))?;
        self.terminal_spool = self
            .terminal_spool
            .checked_add(peak)
            .filter(|n| *n <= limits.base().get().max_spool_bytes)
            .ok_or_else(|| error(root, E::SpoolLimit))?;
        // Ledger construction and final verification each visit all flow records.
        for _ in 0..count {
            self.step(root)?;
            self.step(root)?;
        }
        let mut ledger = registry
            .terminal_ledger()
            .map_err(|e| error(root, E::MathTerminal(e)))?;
        let mut numbers = Vec::new();
        numbers
            .try_reserve_exact(registry.equation_number_shapes().len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        let mut index = 0usize;
        for page in geometry.pages() {
            self.step(root)?;
            for placed in page.fragments() {
                let fragment = placed.fragment();
                self.step(fragment.owner())?;
                terminals::consume_selected_fragment(
                    index,
                    &fragment,
                    flow.blocks,
                    geometry.pages().len(),
                    registry,
                    &mut ledger,
                    &mut numbers,
                )?;
                index = index
                    .checked_add(1)
                    .ok_or_else(|| error(root, E::FragmentLimit))?;
            }
        }
        if numbers.len() != registry.equation_number_shapes().len() {
            return Err(error(root, E::ReceiptMismatch));
        }
        let terminals = ledger
            .finish()
            .map_err(|e| error(root, E::MathTerminal(e)))?;
        terminals
            .verify(registry)
            .map_err(|e| error(root, E::MathTerminal(e)))?;
        let mut spool_bytes = terminals.canonical_jcs().len() as u64;
        for receipt in terminals.receipts() {
            self.step(root)?;
            spool_bytes = spool_bytes
                .checked_add(receipt.canonical_jcs().len() as u64)
                .filter(|n| *n <= peak)
                .ok_or_else(|| error(root, E::SpoolLimit))?;
        }
        if spool_bytes > peak {
            return Err(error(root, E::SpoolLimit));
        }
        Ok(ProductionBodyFootnoteMathTerminals {
            geometry,
            flow,
            registry,
            record_charge: self.record_charge(),
            terminals,
            numbers,
            spool_bytes,
        })
    }
}
