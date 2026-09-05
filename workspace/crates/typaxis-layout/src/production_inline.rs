//! Joins authored shaping and bound inline SVGs in syntax order. This is input
//! to line/page selection, not PDF paint authorization or a source-text painter.
use crate::ValidatedPrecomposedVectorBindings;
use typaxis_core::{
    sha256, Length, M4EffectiveResourceLimits, NodeId, NonNegativeLength, PositiveLength,
};
use typaxis_layout_contract::PrecomposedVectorPlacementInput;
use typaxis_linebreak::{
    AtomicVectorInlineError, AtomicVectorInlineItem, AtomicVectorInlineKind,
    AtomicVectorInlineLogicalUnit, AtomicVectorTextUnit, JapaneseLineBreakMode,
    ProductionInlineParagraph, ProductionTextClusterRange,
};
use typaxis_resource_admission::AdmittedResourceLedger;
use typaxis_shaping::{ProductionAuthoredTextShape, ShapeSourceSpan};
use typaxis_syntax::{
    PrecomposedVectorKind, ProductionInlineContent, ProductionTextFlow,
    StagingPrecomposedVectorProfileAuthorization, ValidatedStagingBookNavigationV2,
    ValidatedStagingSemanticPackage,
};

pub const PRODUCTION_INLINE_PREPARATION_ALGORITHM: &str = "typaxis.production-inline-preparation/1";
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductionInlinePreparationErrorKind {
    ReceiptMismatch,
    MissingTextStyle,
    InvalidHorizontalMetrics,
    UnitLimit,
    PendingInline(&'static str),
    PendingBidiLineSelection,
    AllocationFailure,
    ArithmeticOverflow,
    Atomic(AtomicVectorInlineError),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionInlinePreparationError {
    pub owner: NodeId,
    pub kind: ProductionInlinePreparationErrorKind,
}
impl std::fmt::Display for ProductionInlinePreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "production_inline_preparation {:?}: node {}",
            self.kind,
            self.owner.get()
        )
    }
}
impl std::error::Error for ProductionInlinePreparationError {}
fn error(
    owner: NodeId,
    kind: ProductionInlinePreparationErrorKind,
) -> ProductionInlinePreparationError {
    ProductionInlinePreparationError { owner, kind }
}

/// Indices refer to the matching sealed paragraph shape. The scalar range is
/// only for UAX #14; glyphs and their advances are never recreated from it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionShapedClusterItem {
    run_index: u32,
    cluster_index: u32,
    start_unit: u32,
    end_unit: u32,
}
impl ProductionShapedClusterItem {
    pub const fn run_index(&self) -> u32 {
        self.run_index
    }
    pub const fn cluster_index(&self) -> u32 {
        self.cluster_index
    }
    pub const fn start_unit(&self) -> u32 {
        self.start_unit
    }
    pub const fn end_unit(&self) -> u32 {
        self.end_unit
    }
}
pub struct ProductionPreparedInlineParagraph {
    owner: NodeId,
    line_height: Option<PositiveLength>,
    items: Option<ProductionInlineParagraph>,
    glyph_clusters: Vec<ProductionShapedClusterItem>,
}
impl ProductionPreparedInlineParagraph {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn line_height(&self) -> Option<PositiveLength> {
        self.line_height
    }
    /// None preserves an empty/structural-only paragraph for its containing flow;
    /// it does not authorize dropping the paragraph or its anchors.
    pub const fn items(&self) -> Option<&ProductionInlineParagraph> {
        self.items.as_ref()
    }
    pub fn glyph_clusters(&self) -> &[ProductionShapedClusterItem] {
        &self.glyph_clusters
    }
}
pub struct ProductionPreparedInlines<'a> {
    flow: &'a ProductionTextFlow<'a>,
    shaped: &'a ProductionAuthoredTextShape<'a>,
    bindings: &'a ValidatedPrecomposedVectorBindings,
    paragraphs: Vec<ProductionPreparedInlineParagraph>,
    fingerprint: [u8; 32],
}
impl<'a> ProductionPreparedInlines<'a> {
    pub fn paragraphs(&self) -> &[ProductionPreparedInlineParagraph] {
        &self.paragraphs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        flow: &ProductionTextFlow<'_>,
        shaped: &ProductionAuthoredTextShape<'_>,
        bindings: &ValidatedPrecomposedVectorBindings,
    ) -> Result<(), ProductionInlinePreparationError> {
        if !std::ptr::eq(self.flow, flow)
            || !std::ptr::eq(self.shaped, shaped)
            || !std::ptr::eq(self.bindings, bindings)
        {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_production_inline_items<'a>(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    flow: &'a ProductionTextFlow<'a>,
    shaped: &'a ProductionAuthoredTextShape<'a>,
    bindings: &'a ValidatedPrecomposedVectorBindings,
    japanese_mode: JapaneseLineBreakMode,
) -> Result<ProductionPreparedInlines<'a>, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let root = NodeId::new(0);
    flow.verify(package, navigation, limits)
        .map_err(|e| error(e.owner, E::ReceiptMismatch))?;
    bindings
        .verify(package, profile, limits, admitted)
        .map_err(|_| error(root, E::ReceiptMismatch))?;
    shaped
        .verify(flow, admitted, limits, bindings.epoch().fingerprint())
        .map_err(|e| error(e.owner, E::ReceiptMismatch))?;
    if flow.paragraphs().len() != shaped.paragraphs().len() {
        return Err(error(root, E::ReceiptMismatch));
    }
    let mut paragraphs = Vec::new();
    paragraphs
        .try_reserve_exact(flow.paragraphs().len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    let mut charge = 0u64;
    for (p, shape) in flow.paragraphs().iter().zip(shaped.paragraphs()) {
        if p.owner() != shape.owner() {
            return Err(error(p.owner(), E::ReceiptMismatch));
        }
        if shape.paragraph_level().get() != 0 {
            return Err(error(p.owner(), E::PendingBidiLineSelection));
        }
        let mut units = Vec::new();
        let mut cluster_ranges = Vec::new();
        let mut glyph_clusters = Vec::new();
        let mut run_cursor = 0;
        for (site_index, site) in p.items().iter().enumerate() {
            let owner = site.owner();
            match site.content() {
                ProductionInlineContent::Text { span, utf8 } => {
                    if utf8.is_empty() {
                        continue;
                    }
                    let font = shape
                        .font()
                        .ok_or_else(|| error(owner, E::MissingTextStyle))?;
                    let ascent = NonNegativeLength::new(font.ascender())
                        .ok_or_else(|| error(owner, E::InvalidHorizontalMetrics))?;
                    let descent = Length::ZERO
                        .checked_sub(font.descender())
                        .and_then(NonNegativeLength::new)
                        .ok_or_else(|| error(owner, E::InvalidHorizontalMetrics))?;
                    let mut covered = span.start_byte().get();
                    while let Some(run) = shape
                        .runs()
                        .get(run_cursor)
                        .filter(|r| r.site_index() as usize == site_index)
                    {
                        let raw = run.glyph_run();
                        if raw.bidi_level.get() != 0 {
                            return Err(error(owner, E::PendingBidiLineSelection));
                        }
                        for (cluster_index, cluster) in raw.clusters.iter().enumerate() {
                            let ShapeSourceSpan::Parsed(source) = cluster.source_span else {
                                return Err(error(owner, E::ReceiptMismatch));
                            };
                            if source.text_id() != span.text_id()
                                || source.start_byte().get() != covered
                                || source.end_byte() > span.end_byte()
                            {
                                return Err(error(owner, E::ReceiptMismatch));
                            }
                            let text = utf8
                                .get(
                                    (source.start_byte().get() - span.start_byte().get()) as usize
                                        ..(source.end_byte().get() - span.start_byte().get())
                                            as usize,
                                )
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                            let mut advance = Length::ZERO;
                            for glyph in &raw.glyphs
                                [cluster.glyph_start as usize..cluster.glyph_end as usize]
                            {
                                if glyph.advance_y.raw() != 0 {
                                    return Err(error(owner, E::InvalidHorizontalMetrics));
                                }
                                advance = advance
                                    .checked_add(glyph.advance_x)
                                    .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                            }
                            let advance = NonNegativeLength::new(advance)
                                .ok_or_else(|| error(owner, E::InvalidHorizontalMetrics))?;
                            let scalar_count = text.chars().count();
                            charge = charge
                                .checked_add(scalar_count as u64 + 1)
                                .filter(|n| *n <= limits.base().get().max_fragments)
                                .ok_or_else(|| error(owner, E::UnitLimit))?;
                            units
                                .try_reserve(scalar_count)
                                .map_err(|_| error(owner, E::AllocationFailure))?;
                            cluster_ranges
                                .try_reserve(1)
                                .map_err(|_| error(owner, E::AllocationFailure))?;
                            glyph_clusters
                                .try_reserve(1)
                                .map_err(|_| error(owner, E::AllocationFailure))?;
                            let start = u32::try_from(units.len())
                                .map_err(|_| error(owner, E::ArithmeticOverflow))?;
                            for (index, scalar) in text.chars().enumerate() {
                                units.push(AtomicVectorInlineLogicalUnit::Text(
                                    AtomicVectorTextUnit::new(
                                        scalar,
                                        if index + 1 == scalar_count {
                                            advance
                                        } else {
                                            NonNegativeLength::ZERO
                                        },
                                        ascent,
                                        descent,
                                    ),
                                ));
                            }
                            let end = u32::try_from(units.len())
                                .map_err(|_| error(owner, E::ArithmeticOverflow))?;
                            cluster_ranges.push(ProductionTextClusterRange {
                                start_unit: start,
                                end_unit: end,
                            });
                            glyph_clusters.push(ProductionShapedClusterItem {
                                run_index: run_cursor as u32,
                                cluster_index: cluster_index as u32,
                                start_unit: start,
                                end_unit: end,
                            });
                            covered = source.end_byte().get();
                        }
                        run_cursor += 1;
                    }
                    if covered != span.end_byte().get() {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                }
                ProductionInlineContent::InlineVector | ProductionInlineContent::MathVector => {
                    let receipt = bindings
                        .receipt(owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let (kind, expected) =
                        if matches!(site.content(), ProductionInlineContent::InlineVector) {
                            (
                                AtomicVectorInlineKind::InlineVector,
                                PrecomposedVectorKind::InlineVector,
                            )
                        } else {
                            (
                                AtomicVectorInlineKind::MathVector,
                                PrecomposedVectorKind::MathVector,
                            )
                        };
                    if receipt.kind() != expected
                        || receipt.owner_source_span() != site.source_span()
                    {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    let PrecomposedVectorPlacementInput::Inline(placement) = receipt.placement()
                    else {
                        return Err(error(owner, E::ReceiptMismatch));
                    };
                    let item = AtomicVectorInlineItem::from_bound_placement(
                        owner,
                        p.owner(),
                        site.source_span(),
                        kind,
                        receipt.binding_fingerprint(),
                        *placement,
                    )
                    .map_err(|e| error(owner, E::Atomic(e)))?;
                    charge = charge
                        .checked_add(1)
                        .filter(|n| *n <= limits.base().get().max_fragments)
                        .ok_or_else(|| error(owner, E::UnitLimit))?;
                    units
                        .try_reserve(1)
                        .map_err(|_| error(owner, E::AllocationFailure))?;
                    units.push(AtomicVectorInlineLogicalUnit::Vector(item));
                }
                ProductionInlineContent::Anchor
                | ProductionInlineContent::BeginEmphasis
                | ProductionInlineContent::BeginStrong
                | ProductionInlineContent::BeginLink
                | ProductionInlineContent::EndContainer => (), // Retained by the borrowed syntax flow.
                other => return Err(error(owner, E::PendingInline(other.as_str()))),
            }
        }
        if run_cursor != shape.runs().len() {
            return Err(error(p.owner(), E::ReceiptMismatch));
        }
        let items = if units.is_empty() {
            None
        } else {
            if p.style().line_height().is_none() {
                return Err(error(p.owner(), E::MissingTextStyle));
            }
            Some(
                ProductionInlineParagraph::itemize(p.owner(), units, cluster_ranges, japanese_mode)
                    .map_err(|e| error(p.owner(), E::Atomic(e)))?,
            )
        };
        paragraphs.push(ProductionPreparedInlineParagraph {
            owner: p.owner(),
            line_height: p.style().line_height(),
            items,
            glyph_clusters,
        });
    }
    let mut b = Vec::new();
    b.try_reserve_exact(
        paragraphs
            .len()
            .checked_mul(32)
            .and_then(|n| n.checked_add(128))
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?,
    )
    .map_err(|_| error(root, E::AllocationFailure))?;
    b.extend_from_slice(&sha256(PRODUCTION_INLINE_PREPARATION_ALGORITHM.as_bytes()));
    b.extend_from_slice(&flow.fingerprint());
    b.extend_from_slice(&shaped.fingerprint());
    b.extend_from_slice(&bindings.fingerprint());
    for p in &paragraphs {
        b.extend_from_slice(
            &p.items
                .as_ref()
                .map_or([0; 32], ProductionInlineParagraph::fingerprint),
        );
    }
    Ok(ProductionPreparedInlines {
        flow,
        shaped,
        bindings,
        paragraphs,
        fingerprint: sha256(&b),
    })
}
