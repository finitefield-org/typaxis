//! Iteratively rebuild a bounded set of independent line graphs from one source.
use super::*;

pub struct BookV2RebuiltBodyLineVariants<'s, 'p, 'a> {
    variants: Vec<BookV2RebuiltBodyLineVariant<'s, 'p, 'a>>,
    records: u64,
    work: u64,
    fingerprint: [u8; 32],
}
impl<'s, 'p, 'a> BookV2RebuiltBodyLineVariants<'s, 'p, 'a> {
    pub fn variants(&self) -> &[BookV2RebuiltBodyLineVariant<'s, 'p, 'a>] {
        &self.variants
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// All graphs live in one callback. Build each ownership layer in a separate
/// preallocated vector, without recursively nesting one callback per variant.
/// Source/policy/resource/native identity and the full set budget are checked
/// before allocating any replay storage. The result grants no paint permission.
pub fn with_rebuilt_book_v2_body_line_variants<R>(
    seeds: &[&BookV2BodyLineVariantSeed<'_>],
    maximum_work: u64,
    prior_records: u64,
    use_variants: impl FnOnce(BookV2RebuiltBodyLineVariants<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    use ProductionInlinePreparationErrorKind as E;
    let root = NodeId::new(0);
    let first = seeds
        .first()
        .ok_or_else(|| error(root, E::ReceiptMismatch))?;
    let mut work = 0u64;
    let mut captured = 0u64;
    let mut base = 0u64;
    let mut graphs = 0u64;
    let mut views = 0u64;
    let mut fingerprint = sha256(b"typaxis.book-2-line-variant-set/1");
    take_work(&mut work, 1, maximum_work)?;
    for (index, seed) in seeds.iter().enumerate() {
        take_work(&mut work, 9, maximum_work)?;
        let same_native = match (first.native, seed.native) {
            (None, None) => true,
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            _ => false,
        };
        if !std::ptr::eq(first.flow, seed.flow)
            || !std::ptr::eq(first.policy, seed.policy)
            || !std::ptr::eq(first.admitted, seed.admitted)
            || !std::ptr::eq(first.bindings, seed.bindings)
            || first.limits.fingerprint() != seed.limits.fingerprint()
            || first.japanese_mode != seed.japanese_mode
            || !same_native
        {
            return Err(error(root, E::ReceiptMismatch).into());
        }
        base = base.max(seed.records - seed.captured_records);
        captured = captured
            .checked_add(seed.captured_records)
            .ok_or_else(|| error(root, E::UnitLimit))?;
        graphs = graphs
            .checked_add(seed.rebuild_records)
            .ok_or_else(|| error(root, E::UnitLimit))?;
        views = views
            .checked_add(seed.contexts.paragraphs().len() as u64)
            .ok_or_else(|| error(root, E::UnitLimit))?;
        let mut digest = [0u8; 72];
        digest[..32].copy_from_slice(&fingerprint);
        digest[32..64].copy_from_slice(&seed.fingerprint());
        digest[64..].copy_from_slice(&(index as u64).to_be_bytes());
        fingerprint = sha256(&digest);
    }
    // Independent seeds may have overlapping prior charges. Count every owned
    // context once above the largest prior base; preserve a larger caller ledger.
    let records = base
        .checked_add(captured)
        .map(|n| n.max(prior_records))
        .and_then(|n| n.checked_add(graphs))
        .and_then(|n| n.checked_add(views))
        .and_then(|n| {
            (seeds.len() as u64)
                .checked_mul(5)
                .and_then(|v| n.checked_add(v))
        })
        .and_then(|n| n.checked_add(1))
        .filter(|n| *n <= first.limits.base().get().max_fragments)
        .ok_or_else(|| error(root, E::UnitLimit))?;
    let mut inputs = Vec::new();
    inputs
        .try_reserve_exact(seeds.len())
        .map_err(|_| BreakError::AllocationFailure)?;
    for seed in seeds {
        take_work(
            &mut work,
            seed.contexts.paragraphs().len() as u64,
            maximum_work,
        )?;
        let mut input = Vec::new();
        input
            .try_reserve_exact(seed.contexts.paragraphs().len())
            .map_err(|_| BreakError::AllocationFailure)?;
        input.extend(
            seed.contexts
                .paragraphs()
                .iter()
                .map(|p| ProductionParagraphLineContext {
                    owner: p.owner(),
                    ends: p.ends(),
                }),
        );
        inputs.push(input);
    }
    let mut shapes = Vec::new();
    shapes
        .try_reserve_exact(seeds.len())
        .map_err(|_| BreakError::AllocationFailure)?;
    for (seed, inputs) in seeds.iter().zip(&inputs) {
        take_work(&mut work, 1, maximum_work)?;
        shapes.push(shape_book_v2_authored_text(
            seed.policy,
            seed.flow,
            seed.admitted,
            seed.limits,
            seed.bindings.epoch(),
            Some(inputs),
        )?);
    }
    let mut prepared = Vec::new();
    prepared
        .try_reserve_exact(seeds.len())
        .map_err(|_| BreakError::AllocationFailure)?;
    for (seed, shape) in seeds.iter().zip(&shapes) {
        take_work(&mut work, 1, maximum_work)?;
        prepared.push(prepare_book_v2_inline_items_with_native_context(
            seed.flow,
            shape,
            seed.admitted,
            seed.bindings,
            seed.limits,
            seed.japanese_mode,
            seed.native,
        )?);
    }
    let mut selected = Vec::new();
    selected
        .try_reserve_exact(seeds.len())
        .map_err(|_| BreakError::AllocationFailure)?;
    for (seed, prepared) in seeds.iter().zip(&prepared) {
        let lines = super::super::frames::layout_body_lines_with_source_widths(
            prepared,
            seed.body,
            maximum_work - work,
            seed.page_plan,
            seed.source_widths,
        )?;
        take_work(&mut work, lines.candidate_steps(), maximum_work)?;
        take_work(&mut work, 1, maximum_work)?;
        if lines.fingerprint() != seed.fingerprint() {
            return Err(error(root, E::ReceiptMismatch).into());
        }
        selected.push(lines);
    }
    let mut variants = Vec::new();
    variants
        .try_reserve_exact(seeds.len())
        .map_err(|_| BreakError::AllocationFailure)?;
    for (seed, lines) in seeds.iter().zip(&selected) {
        take_work(&mut work, 1, maximum_work)?;
        let footnotes = prepare_book_v2_footnote_lines(lines, seed.limits)?;
        if footnotes.record_charge() > seed.rebuild_records {
            return Err(error(root, E::ReceiptMismatch).into());
        }
        let variant_work = (seed.contexts.paragraphs().len() as u64)
            .checked_add(lines.candidate_steps())
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
        variants.push(BookV2RebuiltBodyLineVariant {
            lines,
            footnotes,
            records,
            work: variant_work,
        });
    }
    Ok(use_variants(BookV2RebuiltBodyLineVariants {
        variants,
        records,
        work,
        fingerprint,
    }))
}
