//! Source-to-selected-line footnote closure, before page assignment or region fit.
use super::*;
use typaxis_syntax::{
    ProductionFlowEvent as Event, ProductionFlowRegionKind as Region, ProductionInlineReference,
};

#[derive(Debug, Eq, PartialEq)]
pub struct ProductionFootnoteDefinitionLines<'a> {
    owner: NodeId,
    id: &'a str,
    number: u32,
    events: std::ops::Range<usize>,
    paragraphs: std::ops::Range<usize>,
}
impl<'a> ProductionFootnoteDefinitionLines<'a> {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn id(&self) -> &'a str {
        self.id
    }
    pub const fn number(&self) -> u32 {
        self.number
    }
    /// Includes the definition's Begin and End; never a body-flow splice.
    pub fn event_range(&self) -> std::ops::Range<usize> {
        self.events.clone()
    }
    pub fn paragraph_range(&self) -> std::ops::Range<usize> {
        self.paragraphs.clone()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionFootnoteReferencePosition {
    paragraph_index: usize,
    line_index: usize,
    item_index: usize,
}
impl ProductionFootnoteReferencePosition {
    pub const fn paragraph_index(self) -> usize {
        self.paragraph_index
    }
    pub const fn line_index(self) -> usize {
        self.line_index
    }
    pub const fn item_index(self) -> usize {
        self.item_index
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct ProductionFootnoteLineReference {
    owner: NodeId,
    definition_index: usize,
    source_definition: Option<usize>,
    first: Option<ProductionFootnoteReferencePosition>,
    last: Option<ProductionFootnoteReferencePosition>,
}
impl ProductionFootnoteLineReference {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn definition_index(&self) -> usize {
        self.definition_index
    }
    /// Some for a reference originating inside a definition, not body content.
    pub const fn source_definition(&self) -> Option<usize> {
        self.source_definition
    }
    pub fn first(&self) -> ProductionFootnoteReferencePosition {
        self.first.expect("validated marker coverage")
    }
    pub fn last(&self) -> ProductionFootnoteReferencePosition {
        self.last.expect("validated marker coverage")
    }
}
/// Borrows the actual selected lines. It does not authorize page/footnote paint.
pub struct ProductionFootnoteLines<'s, 'p, 'a> {
    lines: &'s ProductionInlineLineLayout<'p, 'a>,
    definitions: Vec<ProductionFootnoteDefinitionLines<'a>>,
    references: Vec<ProductionFootnoteLineReference>,
    record_charge: u64,
    limits_fingerprint: [u8; 32],
}
impl<'s, 'p, 'a> ProductionFootnoteLines<'s, 'p, 'a> {
    pub fn definitions(&self) -> &[ProductionFootnoteDefinitionLines<'a>] {
        &self.definitions
    }
    pub fn references(&self) -> &[ProductionFootnoteLineReference] {
        &self.references
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn verify(
        &self,
        lines: &ProductionInlineLineLayout<'_, '_>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionInlinePreparationError> {
        if !std::ptr::eq(self.lines, lines) || self.limits_fingerprint != limits.fingerprint() {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}

pub fn prepare_production_footnote_lines<'s, 'p, 'a>(
    lines: &'s ProductionInlineLineLayout<'p, 'a>,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionFootnoteLines<'s, 'p, 'a>, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let root = NodeId::new(0);
    if lines.binding_epoch().limits_fingerprint() != limits.fingerprint() {
        return Err(error(root, E::ReceiptMismatch));
    }
    let flow = lines.source_flow();
    let wire = flow
        .package()
        .checked_wire()
        .map_err(|_| error(root, E::ReceiptMismatch))?;
    let sources = &wire.document().footnotes;
    let reference_count = flow
        .paragraphs()
        .iter()
        .flat_map(|p| p.items())
        .filter(|site| {
            matches!(
                site.reference(),
                Some(ProductionInlineReference::Footnote { .. })
            )
        })
        .count();
    let record_charge = lines
        .output_records()
        .checked_add(sources.len() as u64)
        .and_then(|n| n.checked_add((reference_count as u64).checked_mul(2)?))
        .filter(|n| *n <= limits.base().get().max_fragments)
        .ok_or_else(|| error(root, E::UnitLimit))?;
    let mut result = ProductionFootnoteLines {
        lines,
        definitions: Vec::new(),
        references: Vec::new(),
        record_charge,
        limits_fingerprint: limits.fingerprint(),
    };
    result
        .definitions
        .try_reserve_exact(sources.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    result
        .references
        .try_reserve_exact(reference_count)
        .map_err(|_| error(root, E::AllocationFailure))?;
    let mut covered = Vec::new();
    covered
        .try_reserve_exact(reference_count)
        .map_err(|_| error(root, E::AllocationFailure))?;
    covered.resize(reference_count, 0u32);
    let mut depth = 0usize;
    let mut active = None;
    let mut paragraph_cursor = 0usize;
    for (event_index, event) in flow.events().iter().enumerate() {
        match *event {
            Event::Begin { owner, kind } => {
                if kind == Region::Footnote {
                    if depth != 0 || active.is_some() {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    let index = result.definitions.len();
                    let source = sources
                        .get(index)
                        .filter(|s| s.node_id == owner.get())
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    result.definitions.push(ProductionFootnoteDefinitionLines {
                        owner,
                        id: &source.footnote_id,
                        number: u32::try_from(index)
                            .ok()
                            .and_then(|n| n.checked_add(1))
                            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
                        events: event_index..event_index,
                        paragraphs: paragraph_cursor..paragraph_cursor,
                    });
                    active = Some(index);
                }
                depth = depth
                    .checked_add(1)
                    .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            }
            Event::Paragraph { index } => {
                if index as usize != paragraph_cursor {
                    return Err(error(root, E::ReceiptMismatch));
                }
                paragraph_cursor += 1;
            }
            Event::End { owner } => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                if depth == 0 {
                    if let Some(index) = active.take() {
                        let definition = &mut result.definitions[index];
                        if definition.owner != owner {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                        definition.events.end = event_index + 1;
                        definition.paragraphs.end = paragraph_cursor;
                    }
                }
            }
        }
    }
    if depth != 0
        || active.is_some()
        || result.definitions.len() != sources.len()
        || paragraph_cursor != flow.paragraphs().len()
    {
        return Err(error(root, E::ReceiptMismatch));
    }
    let mut scope_index = 0;
    for (paragraph_index, paragraph) in flow.paragraphs().iter().enumerate() {
        while result
            .definitions
            .get(scope_index)
            .is_some_and(|d| d.paragraphs.end <= paragraph_index)
        {
            scope_index += 1;
        }
        let source_definition = result
            .definitions
            .get(scope_index)
            .filter(|d| d.paragraphs.contains(&paragraph_index))
            .map(|_| scope_index);
        for site in paragraph.items() {
            if let Some(ProductionInlineReference::Footnote { footnote_id }) = site.reference() {
                let definition_index = sources
                    .binary_search_by(|d| d.footnote_id.as_str().cmp(footnote_id))
                    .map_err(|_| error(site.owner(), E::ReceiptMismatch))?;
                if result
                    .references
                    .last()
                    .is_some_and(|r| r.owner >= site.owner())
                {
                    return Err(error(site.owner(), E::ReceiptMismatch));
                }
                result.references.push(ProductionFootnoteLineReference {
                    owner: site.owner(),
                    definition_index,
                    source_definition,
                    first: None,
                    last: None,
                });
            }
        }
    }
    for (paragraph_index, paragraph) in lines.paragraphs().iter().enumerate() {
        for (line_index, line) in paragraph.lines().iter().enumerate() {
            for (item_index, item) in line.items().iter().enumerate() {
                let ProductionPlacedInline::Text(cluster) = item else {
                    continue;
                };
                let ShapeSourceSpan::Generated(part) = cluster.source_span() else {
                    continue;
                };
                let owner = cluster.run().owner();
                match part.buffer_key().generation_kind() {
                    typaxis_core::GenerationKind::FootnoteMarker => {}
                    // Page candidates are already checked by line selection;
                    // they do not create a footnote demand or definition edge.
                    typaxis_core::GenerationKind::PageReference => continue,
                    _ => return Err(error(owner, E::ReceiptMismatch)),
                }
                let reference_index = result
                    .references
                    .binary_search_by_key(&owner, |r| r.owner)
                    .map_err(|_| error(owner, E::ReceiptMismatch))?;
                let whole = flow
                    .footnote_marker_provenance(owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let text = flow
                    .footnote_marker_text(owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let (start, end) = relative_shape_range(
                    ShapeSourceSpan::Generated(part),
                    ShapeSourceSpan::Generated(whole),
                )
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                if start != covered[reference_index]
                    || start == end
                    || text.get(start as usize..end as usize) != Some(cluster.utf8())
                {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                covered[reference_index] = end;
                let position = ProductionFootnoteReferencePosition {
                    paragraph_index,
                    line_index,
                    item_index,
                };
                let reference = &mut result.references[reference_index];
                if reference.first.is_none() {
                    reference.first = Some(position);
                }
                reference.last = Some(position);
            }
        }
    }
    for (reference, covered) in result.references.iter().zip(covered) {
        let text = flow
            .footnote_marker_text(reference.owner)
            .ok_or_else(|| error(reference.owner, E::ReceiptMismatch))?;
        if covered as usize != text.len() || reference.first.is_none() || reference.last.is_none() {
            return Err(error(reference.owner, E::ReceiptMismatch));
        }
    }
    Ok(result)
}
