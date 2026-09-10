//! Width candidates bound to exact prepared source units, before page acceptance.
use super::*;

pub const PRODUCTION_SOURCE_WIDTH_BREAK_ALGORITHM: &str = "typaxis.source-width-inline-break/1";
pub const PRODUCTION_REFINED_WIDTH_BREAK_ALGORITHM: &str = "typaxis.refined-width-inline-break/1";

/// One width per possible original logical-unit start (one slot for an empty
/// paragraph). This is a provisional source-width assignment, not a page receipt.
/// Re-itemizing or reshaping a paragraph requires binding a new assignment.
pub struct ProductionInlineSourceWidths<'a> {
    paragraph: &'a ProductionInlineParagraph,
    sizes: &'a [PositiveLength],
    retained_ends: Option<&'a [u32]>,
}
impl<'a> ProductionInlineSourceWidths<'a> {
    pub fn new(
        paragraph: &'a ProductionInlineParagraph,
        sizes: &'a [PositiveLength],
    ) -> Result<Self, AtomicVectorInlineError> {
        if sizes.len() != paragraph.units().len().max(1) {
            return Err(AtomicVectorInlineError::InvalidBinding);
        }
        Ok(Self {
            paragraph,
            sizes,
            retained_ends: None,
        })
    }
    /// Retain legal source boundaries while allowing further subdivision. These
    /// are layout constraints, never additional authored mandatory breaks.
    pub fn with_retained_line_ends(
        paragraph: &'a ProductionInlineParagraph,
        sizes: &'a [PositiveLength],
        ends: &'a [u32],
    ) -> Result<Self, AtomicVectorInlineError> {
        let mut value = Self::new(paragraph, sizes)?;
        value.retained_ends = Some(ends);
        Ok(value)
    }
    pub fn retained_line_ends(&self) -> Option<&'a [u32]> {
        self.retained_ends
    }
    pub fn source(&self) -> &'a ProductionInlineParagraph {
        self.paragraph
    }
    pub fn sizes(&self) -> &'a [PositiveLength] {
        self.sizes
    }
}

pub(super) enum InlineWidths<'a> {
    Fixed(PositiveLength),
    Source(&'a [PositiveLength], Option<&'a [u32]>),
}
impl InlineWidths<'_> {
    pub(super) fn at(&self, start: usize) -> PositiveLength {
        match self {
            Self::Fixed(size) => *size,
            Self::Source(sizes, _) => sizes[start],
        }
    }
    pub(super) fn retained_ends(&self) -> Option<&[u32]> {
        match self {
            Self::Source(_, ends) => *ends,
            Self::Fixed(_) => None,
        }
    }
}

/// The source start determines candidate width, rather than an unstable line
/// ordinal. Every legal candidate still uses the common minimum-demerit kernel.
/// Source-width visits and selected lines consume the caller's existing budget.
pub fn break_production_inline_with_source_widths(
    widths: &ProductionInlineSourceWidths<'_>,
    line_height: PositiveLength,
    budget: &mut ProductionLineBreakBudget,
) -> Result<ProductionInlineBreak, AtomicVectorInlineError> {
    // Pay for the complete assignment, including unreachable starts, before
    // allocating a search arena. The complete assignment enters its fingerprint.
    for _ in widths.sizes {
        budget.step()?;
    }
    if let Some(ends) = widths.retained_ends {
        let count = widths.paragraph.units.len();
        let mut previous = 0;
        for &end in ends {
            budget.step()?;
            let end = end as usize;
            if end > count
                || (count != 0 && end <= previous)
                || (end < count
                    && widths.paragraph.boundaries[end - 1].kind() == BreakKind::Prohibited)
            {
                return Err(AtomicVectorInlineError::InvalidBinding);
            }
            previous = end;
        }
        if ends.last().copied().map(|n| n as usize) != Some(count) || (count == 0 && ends != [0]) {
            return Err(AtomicVectorInlineError::InvalidBinding);
        }
    }
    break_with_widths(
        widths.paragraph,
        InlineWidths::Source(widths.sizes, widths.retained_ends),
        line_height,
        budget,
    )
}

#[cfg(test)]
#[path = "production_inline_width_tests.rs"]
mod tests;
