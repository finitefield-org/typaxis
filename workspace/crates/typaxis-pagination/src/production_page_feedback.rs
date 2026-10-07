//! Compare actual static-body page selections before consuming any math terminal.
//! This is not generated-reference, named-page or full multi-flow convergence.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyPagePass {
    index: u16,
    selected_fingerprint: [u8; 32],
    page_count: u32,
    cumulative_record_charge: u64,
}
impl ProductionBodyPagePass {
    pub const fn index(&self) -> u16 {
        self.index
    }
    pub const fn selected_fingerprint(&self) -> [u8; 32] {
        self.selected_fingerprint
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn cumulative_record_charge(&self) -> u64 {
        self.cumulative_record_charge
    }
}

/// An immutable, process-owned static page stability proof. It is bound to the
/// exact borrowed line and block preparations and can be checked after the
/// selected layout acquires its block-math terminal fingerprint.
pub struct ProductionBodyPageStability<'s, 'p, 'a> {
    lines: &'s ProductionInlineLineLayout<'p, 'a>,
    blocks: &'s StagingPrecomposedVectorBlockLayout,
    passes: Vec<ProductionBodyPagePass>,
}
impl ProductionBodyPageStability<'_, '_, '_> {
    pub fn passes(&self) -> &[ProductionBodyPagePass] {
        &self.passes
    }
    pub fn verify(
        &self,
        selected: &ProductionBodySelectedLayout<'_, '_, '_>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyPaginationError> {
        selected.verify(self.lines, self.blocks, limits)?;
        let root = NodeId::new(0);
        let last = self
            .passes
            .last()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let placement = selected.math_terminals().map_or_else(
            || selected.fingerprint(),
            |terminal| terminal.placement_fingerprint(),
        );
        if self.passes.len() < 2
            || self.passes.len() > usize::from(limits.base().get().max_layout_passes)
            || self.passes[self.passes.len() - 2].selected_fingerprint != last.selected_fingerprint
            || placement != last.selected_fingerprint
            || selected.record_charge() < last.cumulative_record_charge
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        Ok(())
    }
}

/// Only the owner below creates this value, after matching complete selections.
/// The retained selection includes work charges from every attempted pass.
pub struct ProductionStableBodyPages<'s, 'p, 'a> {
    selected: ProductionBodySelectedLayout<'s, 'p, 'a>,
    stability: ProductionBodyPageStability<'s, 'p, 'a>,
}
impl<'s, 'p, 'a> ProductionStableBodyPages<'s, 'p, 'a> {
    pub fn selected(&self) -> &ProductionBodySelectedLayout<'s, 'p, 'a> {
        &self.selected
    }
    pub fn passes(&self) -> &[ProductionBodyPagePass] {
        self.stability.passes()
    }
    pub fn into_parts(
        self,
    ) -> (
        ProductionBodySelectedLayout<'s, 'p, 'a>,
        ProductionBodyPageStability<'s, 'p, 'a>,
    ) {
        (self.selected, self.stability)
    }
}

/// Run at least two real page searches from the same immutable selected lines
/// and blocks. Compare full placement/decision fingerprints, not page counts.
/// Record charges are conservative: each pass retains its normal input charge
/// plus all preceding pass charges and one pass observation. No failure retries
/// with a fresh budget, and block terminals are consumed only by the next owner.
/// Dynamic reference/footnote sites are explicitly outside this static stage.
pub fn paginate_stable_production_body<'s, 'p, 'a>(
    lines: &'s ProductionInlineLineLayout<'p, 'a>,
    blocks: &'s StagingPrecomposedVectorBlockLayout,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionStableBodyPages<'s, 'p, 'a>, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    let max_passes = limits.base().get().max_layout_passes;
    if max_passes < 2 {
        return Err(error(root, E::PagePassLimit));
    }
    for paragraph in lines.source_flow().paragraphs() {
        for item in paragraph.items() {
            if matches!(
                item.content(),
                typaxis_syntax::ProductionInlineContent::Reference
                    | typaxis_syntax::ProductionInlineContent::FootnoteReference
            ) {
                return Err(error(
                    item.owner(),
                    E::PendingRegion("generated_page_feedback"),
                ));
            }
        }
    }
    let mut previous = None;
    let mut cumulative = 0u64;
    let mut passes = Vec::new();
    for index in 0..max_passes {
        // Reserve one observation before either the page search or its record.
        cumulative = cumulative
            .checked_add(1)
            .filter(|n| *n <= limits.base().get().max_fragments)
            .ok_or_else(|| error(root, E::FragmentLimit))?;
        let selected =
            paginate_production_body_with_prior_charge(lines, blocks, limits, cumulative)?;
        selected.verify(lines, blocks, limits)?;
        cumulative = selected.record_charge();
        let fingerprint = selected.fingerprint();
        passes
            .try_reserve(1)
            .map_err(|_| error(root, E::AllocationFailure))?;
        passes.push(ProductionBodyPagePass {
            index,
            selected_fingerprint: fingerprint,
            page_count: u32::try_from(selected.pages().len())
                .map_err(|_| error(root, E::PageLimit))?,
            cumulative_record_charge: cumulative,
        });
        if previous == Some(fingerprint) {
            let stability = ProductionBodyPageStability {
                lines,
                blocks,
                passes,
            };
            stability.verify(&selected, limits)?;
            return Ok(ProductionStableBodyPages {
                selected,
                stability,
            });
        }
        previous = Some(fingerprint);
    }
    Err(error(root, E::PagePassLimit))
}
