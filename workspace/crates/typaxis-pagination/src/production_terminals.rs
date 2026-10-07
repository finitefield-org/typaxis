//! Close the existing atomic math-flow ledger from actual common-body fragments.
//! This is placement completion, not a convergence or PDF publication permit.
use super::*;
use typaxis_layout::{StagingMathVectorFlowRegistry, StagingMathVectorTerminalReceiptSet};

pub const PRODUCTION_BODY_TERMINAL_ALGORITHM: &str = "typaxis.production-body-terminal/2";

/// The producer-authored number stays separate from formula replacement text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyEquationNumber {
    owner: NodeId,
    parent_owner: NodeId,
    fragment_index: u32,
    page_index: u32,
    bounds: Rect,
    shape_fingerprint: [u8; 32],
}
impl ProductionBodyEquationNumber {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn parent_owner(&self) -> NodeId {
        self.parent_owner
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
    pub const fn shape_fingerprint(&self) -> [u8; 32] {
        self.shape_fingerprint
    }
}

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
    registry: &'s StagingMathVectorFlowRegistry,
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
        .and_then(|n| n.checked_add(registry.equation_number_shapes().len() as u64))
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
    let mut numbers = Vec::new();
    numbers
        .try_reserve_exact(registry.equation_number_shapes().len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    for (index, fragment) in selected.fragments.iter().enumerate() {
        consume_selected_fragment(
            index,
            fragment,
            selected.blocks,
            selected.pages.len(),
            registry,
            &mut ledger,
            &mut numbers,
        )?;
    }
    if numbers.len() != registry.equation_number_shapes().len() {
        return Err(error(root, E::ReceiptMismatch));
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
    let mut fingerprint = sha256(&digest);
    // Fixed-size incremental encoding adds no uncharged document-sized buffer.
    for number in &numbers {
        let mut bytes = [0u8; 112];
        bytes[..32].copy_from_slice(&fingerprint);
        bytes[32..64].copy_from_slice(&number.shape_fingerprint);
        for (slot, value) in bytes[64..80].chunks_exact_mut(4).zip([
            number.owner.get(),
            number.parent_owner.get(),
            number.fragment_index,
            number.page_index,
        ]) {
            slot.copy_from_slice(&value.to_be_bytes());
        }
        for (slot, value) in bytes[80..].chunks_exact_mut(8).zip([
            number.bounds.x().raw(),
            number.bounds.y().raw(),
            number.bounds.width().get().raw(),
            number.bounds.height().get().raw(),
        ]) {
            slot.copy_from_slice(&value.to_be_bytes());
        }
        fingerprint = sha256(&bytes);
    }
    selected.math_terminals = Some(ProductionBodyMathTerminals {
        terminals,
        placement_fingerprint,
        fingerprint,
        spool_charge,
        peak_spool_charge,
    });
    selected.math_registry = Some(registry);
    selected.equation_numbers = numbers;
    selected.record_charge = record_charge;
    selected.fingerprint = fingerprint;
    Ok(selected)
}

/// Common atomic block/number validation for ordinary and joint pages.
pub(super) fn consume_selected_fragment(
    fragment_index: usize,
    fragment: &ProductionBodyFragment,
    blocks: &StagingPrecomposedVectorBlockLayout,
    page_count: usize,
    registry: &StagingMathVectorFlowRegistry,
    ledger: &mut typaxis_layout::StagingMathVectorTerminalLedger,
    numbers: &mut Vec<ProductionBodyEquationNumber>,
) -> Result<(), ProductionBodyPaginationError> {
    place_selected_math_fragment(
        fragment_index,
        fragment,
        blocks,
        page_count,
        registry,
        ledger,
        numbers,
        true,
    )
}
pub(super) fn place_selected_math_fragment(
    fragment_index: usize,
    fragment: &ProductionBodyFragment,
    blocks: &StagingPrecomposedVectorBlockLayout,
    page_count: usize,
    registry: &StagingMathVectorFlowRegistry,
    ledger: &mut typaxis_layout::StagingMathVectorTerminalLedger,
    numbers: &mut Vec<ProductionBodyEquationNumber>,
    consume: bool,
) -> Result<(), ProductionBodyPaginationError> {
    let ProductionBodyFragmentSource::VectorBlock { block_index } = fragment.source else {
        return Ok(());
    };
    let block = blocks
        .blocks()
        .get(block_index as usize)
        .filter(|b| b.owner() == fragment.owner)
        .ok_or_else(|| error(fragment.owner, E::ReceiptMismatch))?;
    let Some(flow) = block.math_flow() else {
        return Ok(());
    };
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
    if fragment.page_index as usize >= page_count
        || viewport.width() != block.viewport_width()
        || viewport.height() != block.viewport_height()
        || viewport.y() != expected_y
        || fragment.baseline != expected_baseline
        || fragment.bounds.height() != block.content_height()
    {
        return Err(error(block.owner(), E::ReceiptMismatch));
    }
    match (
        block.equation_number(),
        registry.equation_number_shape(block.owner()),
    ) {
        (None, None) => (),
        (Some(number), Some(shape)) => {
            if shape.node_id() != number.owner()
                || shape.owner() != block.owner()
                || shape.fingerprint() != number.shape_fingerprint()
                || shape.width() != number.width()
                || shape.height() != number.height()
                || shape.source_span() != number.source_span()
            {
                return Err(error(block.owner(), E::ReceiptMismatch));
            }
            numbers.push(place_equation_number(fragment_index, fragment, number)?);
        }
        _ => return Err(error(block.owner(), E::ReceiptMismatch)),
    }
    if consume {
        ledger
            .consume_selected(source.flow_id(), block.owner())
            .map_err(|cause| error(block.owner(), E::MathTerminal(cause)))?;
    }
    Ok(())
}

/// Shared geometry only; versioned callers verify their actual source/shape.
pub(super) fn place_equation_number(
    fragment_index: usize,
    fragment: &ProductionBodyFragment,
    number: &typaxis_layout::StagingPreparedVectorEquationNumber,
) -> Result<ProductionBodyEquationNumber, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    let viewport = fragment
        .viewport
        .ok_or_else(|| error(fragment.owner, E::ReceiptMismatch))?;
    let left = add(
        fragment.bounds.x(),
        fragment.bounds.width().get(),
        fragment.owner,
    )?
    .checked_sub(number.width().get())
    .ok_or_else(|| error(fragment.owner, E::ArithmeticOverflow))?;
    let required = add(
        add(viewport.x(), viewport.width().get(), fragment.owner)?,
        number.minimum_gap().get(),
        fragment.owner,
    )?;
    if left < required {
        return Err(error(fragment.owner, E::WidthMismatch));
    }
    let top = add(
        fragment.bounds.y(),
        number.top_offset().get(),
        fragment.owner,
    )?;
    Ok(ProductionBodyEquationNumber {
        owner: number.owner(),
        parent_owner: fragment.owner,
        fragment_index: u32::try_from(fragment_index).map_err(|_| error(root, E::FragmentLimit))?,
        page_index: fragment.page_index,
        bounds: Rect::new(left, top, number.width(), number.height()),
        shape_fingerprint: number.shape_fingerprint(),
    })
}
