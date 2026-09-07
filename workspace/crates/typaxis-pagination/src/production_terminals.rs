//! Close the existing atomic math-flow ledger from actual common-body fragments.
//! This is placement completion, not a convergence or PDF publication permit.
use super::*;
use typaxis_layout::{StagingMathVectorFlowRegistry, StagingMathVectorTerminalReceiptSet};

pub const PRODUCTION_BODY_TERMINAL_ALGORITHM: &str = "typaxis.production-body-terminal/1";

pub struct ProductionBodyMathTerminals {
    terminals: StagingMathVectorTerminalReceiptSet,
    placement_fingerprint: [u8; 32],
    fingerprint: [u8; 32],
    spool_charge: u64,
    peak_spool_charge: u64,
}
impl ProductionBodyMathTerminals {
    pub const fn terminals(&self) -> &StagingMathVectorTerminalReceiptSet {
        &self.terminals
    }
    pub const fn placement_fingerprint(&self) -> [u8; 32] {
        self.placement_fingerprint
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub const fn peak_spool_charge(&self) -> u64 {
        self.peak_spool_charge
    }
}

/// The registry must be the same sealed registry used to prepare these blocks.
/// Every block terminal is consumed once, after a complete selected fragment;
/// explicit blank pages, captions and list labels never consume a terminal.
pub fn finalize_production_body_math_terminals<'s, 'p, 'a>(
    mut selected: ProductionBodySelectedLayout<'s, 'p, 'a>,
    registry: &StagingMathVectorFlowRegistry,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodySelectedLayout<'s, 'p, 'a>, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    if selected.math_terminals.is_some()
        || selected.limits_fingerprint != limits.fingerprint()
        || selected.blocks.receipt().math_flow_registry_fingerprint()
            != registry.receipt().fingerprint()
        || selected.blocks.receipt().layout_epoch_fingerprint()
            != registry.receipt().layout_epoch_fingerprint()
    {
        return Err(error(root, E::ReceiptMismatch));
    }
    // The ledger copies flow records and keeps pending/final receipts; the
    // final set and this binding share the same cumulative document budget.
    let count = registry.flows().len() as u64;
    let record_charge = count
        .checked_mul(4)
        .and_then(|n| n.checked_add(2))
        .and_then(|n| n.checked_add(selected.record_charge))
        .filter(|n| *n <= limits.base().get().max_fragments)
        .ok_or_else(|| error(root, E::FragmentLimit))?;
    // Terminal /1 has fixed field names, two SHA-256 values and bounded u32
    // decimal IDs: each record is <512 bytes; the set repeats it once. Reserve
    // allocator growth plus the registry/shape integrity encoders before the
    // ledger performs any allocation. Retained bytes are measured separately.
    let integrity_bytes = registry
        .equation_number_shapes()
        .iter()
        .try_fold(
            registry.receipt().canonical_jcs().len() as u64,
            |n, shape| n.checked_add(shape.canonical_jcs().len() as u64),
        )
        .ok_or_else(|| error(root, E::SpoolLimit))?;
    let peak_spool_charge = count
        .checked_mul(4096)
        .and_then(|n| n.checked_add(4096))
        .and_then(|n| {
            integrity_bytes
                .checked_mul(4)
                .and_then(|m| n.checked_add(m))
        })
        .filter(|n| *n <= limits.base().get().max_spool_bytes)
        .ok_or_else(|| error(root, E::SpoolLimit))?;
    let mut ledger = registry
        .terminal_ledger()
        .map_err(|cause| error(root, E::MathTerminal(cause)))?;
    for fragment in &selected.fragments {
        let ProductionBodyFragmentSource::VectorBlock { block_index } = fragment.source else {
            continue;
        };
        let block = selected
            .blocks
            .blocks()
            .get(block_index as usize)
            .filter(|b| b.owner() == fragment.owner)
            .ok_or_else(|| error(fragment.owner, E::ReceiptMismatch))?;
        let Some(flow) = block.math_flow() else {
            continue;
        };
        if block.equation_number().is_some() {
            // A prepared number is not a selected/painted number. Its reduced-
            // frame placement must be connected before this gate can close it.
            return Err(error(block.owner(), E::PendingEquationNumber));
        }
        let source = registry
            .flow(flow.flow_id())
            .filter(|s| s.owner() == block.owner() && s.fingerprint() == flow.flow_fingerprint())
            .ok_or_else(|| error(block.owner(), E::ReceiptMismatch))?;
        let viewport = fragment
            .viewport
            .ok_or_else(|| error(block.owner(), E::ReceiptMismatch))?;
        let expected_y = add(
            fragment.bounds.y(),
            block.viewport_top_offset().get(),
            block.owner(),
        )?;
        let expected_baseline = block
            .baseline()
            .map(|b| add(expected_y, b.get(), block.owner()))
            .transpose()?;
        if fragment.page_index as usize >= selected.pages.len()
            || viewport.width() != block.viewport_width()
            || viewport.height() != block.viewport_height()
            || viewport.y() != expected_y
            || fragment.baseline != expected_baseline
            || fragment.bounds.height() != block.content_height()
        {
            return Err(error(block.owner(), E::ReceiptMismatch));
        }
        ledger
            .consume_selected(source.flow_id(), block.owner())
            .map_err(|cause| error(block.owner(), E::MathTerminal(cause)))?;
    }
    let terminals = ledger
        .finish()
        .map_err(|cause| error(root, E::MathTerminal(cause)))?;
    terminals
        .verify(registry)
        .map_err(|cause| error(root, E::MathTerminal(cause)))?;
    let spool_charge = terminals
        .receipts()
        .iter()
        .try_fold(terminals.canonical_jcs().len() as u64, |n, r| {
            n.checked_add(r.canonical_jcs().len() as u64)
        })
        .filter(|n| *n <= peak_spool_charge)
        .ok_or_else(|| error(root, E::ReceiptMismatch))?;
    let placement_fingerprint = selected.fingerprint;
    let mut digest = [0u8; 96];
    digest[..32].copy_from_slice(&sha256(PRODUCTION_BODY_TERMINAL_ALGORITHM.as_bytes()));
    digest[32..64].copy_from_slice(&placement_fingerprint);
    digest[64..].copy_from_slice(&terminals.fingerprint());
    let fingerprint = sha256(&digest);
    selected.math_terminals = Some(ProductionBodyMathTerminals {
        terminals,
        placement_fingerprint,
        fingerprint,
        spool_charge,
        peak_spool_charge,
    });
    selected.record_charge = record_charge;
    selected.fingerprint = fingerprint;
    Ok(selected)
}
