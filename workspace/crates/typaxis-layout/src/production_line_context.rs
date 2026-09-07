//! Derive shaping context boundaries from real selected clusters and atoms.
//! This is a reshape input owner, not evidence that the rebreak loop converged.
use super::*;

pub struct ProductionSelectedParagraphContext {
    owner: NodeId,
    ends: Vec<u32>,
}
impl ProductionSelectedParagraphContext {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn ends(&self) -> &[u32] {
        &self.ends
    }
}
pub struct ProductionSelectedLineContexts {
    paragraphs: Vec<ProductionSelectedParagraphContext>,
    source_fingerprint: [u8; 32],
    record_charge: u64,
}
impl ProductionSelectedLineContexts {
    pub fn paragraphs(&self) -> &[ProductionSelectedParagraphContext] {
        &self.paragraphs
    }
    pub const fn source_fingerprint(&self) -> [u8; 32] {
        self.source_fingerprint
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
}

pub fn production_selected_line_contexts(
    selected: &ProductionInlineLineLayout<'_, '_>,
) -> Result<ProductionSelectedLineContexts, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let root = NodeId::new(0);
    let mut record_charge = selected.output_records();
    // An owned paragraph record, a borrowed shaper input view, and each line end.
    for paragraph in selected.paragraphs() {
        record_charge = record_charge
            .checked_add(paragraph.lines().len() as u64)
            .and_then(|n| n.checked_add(2))
            .filter(|n| *n <= selected.prepared_limit())
            .ok_or_else(|| error(paragraph.owner(), E::UnitLimit))?;
    }
    let mut paragraphs = Vec::new();
    paragraphs
        .try_reserve_exact(selected.paragraphs().len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    for paragraph in selected.paragraphs() {
        let mut ends = Vec::new();
        ends.try_reserve_exact(paragraph.lines().len())
            .map_err(|_| error(paragraph.owner(), E::AllocationFailure))?;
        let mut offset = 0u32;
        for line in paragraph.lines() {
            for item in line.items() {
                let count = match item {
                    ProductionPlacedInline::Text(t) => u32::try_from(t.utf8().len())
                        .map_err(|_| error(paragraph.owner(), E::ArithmeticOverflow))?,
                    ProductionPlacedInline::Vector(_) => 3, // U+FFFC
                    ProductionPlacedInline::Break(b) if b.kind() == BreakKind::Mandatory => 3, // U+2028
                    ProductionPlacedInline::Break(_) => 0,
                };
                offset = offset
                    .checked_add(count)
                    .ok_or_else(|| error(paragraph.owner(), E::ArithmeticOverflow))?;
            }
            ends.push(offset);
        }
        paragraphs.push(ProductionSelectedParagraphContext {
            owner: paragraph.owner(),
            ends,
        });
    }
    Ok(ProductionSelectedLineContexts {
        paragraphs,
        source_fingerprint: selected.fingerprint(),
        record_charge,
    })
}
