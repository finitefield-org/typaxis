//! Line-local projection of the sealed authored shape and SVG bindings.
//! Pagination, final-line reshaping and PDF paint authorization are later stages.
use super::*;
use typaxis_core::TextSpan;
use typaxis_layout_contract::SelectedPrecomposedVectorInlineGeometry;
use typaxis_linebreak::{
    break_production_inline, AtomicVectorLineOccurrence, ProductionInlineBreak,
    ProductionInlineLogicalUnit as Unit, ProductionInlineSelectedLine, ProductionLineBreakBudget,
};
use typaxis_shaping::{ProductionBodyFont, ProductionBodyTextRun, ShapedGlyph};

pub const PRODUCTION_INLINE_LINE_LAYOUT_ALGORITHM: &str = "typaxis.production-inline-line-layout/1";

/// Original glyph plus a line-local origin in the top-left, Y-down system.
/// The shaper's positive Y offset is subtracted from the shared line baseline.
#[derive(Debug)]
pub struct ProductionPlacedGlyph<'p> {
    glyph_index: u32,
    glyph: &'p ShapedGlyph,
    x: Length,
    y: Length,
}
impl<'p> ProductionPlacedGlyph<'p> {
    pub const fn glyph_index(&self) -> u32 {
        self.glyph_index
    }
    pub const fn glyph(&self) -> &'p ShapedGlyph {
        self.glyph
    }
    pub const fn x(&self) -> Length {
        self.x
    }
    pub const fn y(&self) -> Length {
        self.y
    }
}

/// A complete source cluster, with no Unicode-to-glyph reconstruction. This is
/// the logical extraction unit even when the shaper emitted multiple glyphs.
#[derive(Debug)]
pub struct ProductionPlacedTextCluster<'p, 'a> {
    run_index: u32,
    cluster_index: u32,
    run: &'p ProductionBodyTextRun<'a>,
    source_span: TextSpan,
    utf8: &'a str,
    pen_x: Length,
    glyphs: Vec<ProductionPlacedGlyph<'p>>,
}
impl<'p, 'a> ProductionPlacedTextCluster<'p, 'a> {
    pub const fn run_index(&self) -> u32 {
        self.run_index
    }
    pub const fn cluster_index(&self) -> u32 {
        self.cluster_index
    }
    pub const fn run(&self) -> &'p ProductionBodyTextRun<'a> {
        self.run
    }
    pub const fn source_span(&self) -> TextSpan {
        self.source_span
    }
    pub const fn utf8(&self) -> &'a str {
        self.utf8
    }
    pub const fn pen_x(&self) -> Length {
        self.pen_x
    }
    pub fn glyphs(&self) -> &[ProductionPlacedGlyph<'p>] {
        &self.glyphs
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionPlacedInlineVector {
    occurrence: AtomicVectorLineOccurrence,
    geometry: SelectedPrecomposedVectorInlineGeometry,
}
impl ProductionPlacedInlineVector {
    pub const fn occurrence(self) -> AtomicVectorLineOccurrence {
        self.occurrence
    }
    pub const fn geometry(self) -> SelectedPrecomposedVectorInlineGeometry {
        self.geometry
    }
}

/// Source order within a selected line. A Break retains provenance and consumes
/// no paint or extraction text; markup/anchors remain in the borrowed flow.
#[derive(Debug)]
pub enum ProductionPlacedInline<'p, 'a> {
    Text(ProductionPlacedTextCluster<'p, 'a>),
    Vector(ProductionPlacedInlineVector),
    Break(ProductionExplicitBreak),
}

#[derive(Debug)]
pub struct ProductionPlacedInlineLine<'p, 'a> {
    baseline: Length,
    items: Vec<ProductionPlacedInline<'p, 'a>>,
}
impl<'p, 'a> ProductionPlacedInlineLine<'p, 'a> {
    pub const fn baseline(&self) -> Length {
        self.baseline
    }
    pub fn items(&self) -> &[ProductionPlacedInline<'p, 'a>] {
        &self.items
    }
}

pub struct ProductionInlineParagraphLineLayout<'p, 'a> {
    owner: NodeId,
    inline_size: PositiveLength,
    font: Option<&'p ProductionBodyFont>,
    selected: Option<ProductionInlineBreak>,
    lines: Vec<ProductionPlacedInlineLine<'p, 'a>>,
}
impl<'p, 'a> ProductionInlineParagraphLineLayout<'p, 'a> {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn inline_size(&self) -> PositiveLength {
        self.inline_size
    }
    pub const fn font(&self) -> Option<&'p ProductionBodyFont> {
        self.font
    }
    pub const fn selected(&self) -> Option<&ProductionInlineBreak> {
        self.selected.as_ref()
    }
    pub fn lines(&self) -> &[ProductionPlacedInlineLine<'p, 'a>] {
        &self.lines
    }
}

pub struct ProductionInlineLineLayout<'p, 'a> {
    prepared: &'p ProductionPreparedInlines<'a>,
    paragraphs: Vec<ProductionInlineParagraphLineLayout<'p, 'a>>,
    output_records: u64,
    fingerprint: [u8; 32],
}
impl<'p, 'a> ProductionInlineLineLayout<'p, 'a> {
    pub const fn source_flow(&self) -> &'a ProductionTextFlow<'a> {
        self.prepared.flow
    }
    pub const fn binding_epoch(&self) -> &crate::PrecomposedVectorLayoutEpoch {
        self.prepared.bindings.epoch()
    }
    pub const fn binding_set_fingerprint(&self) -> [u8; 32] {
        self.prepared.bindings.fingerprint()
    }
    pub fn paragraphs(&self) -> &[ProductionInlineParagraphLineLayout<'p, 'a>] {
        &self.paragraphs
    }
    pub const fn output_records(&self) -> u64 {
        self.output_records
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        prepared: &ProductionPreparedInlines<'_>,
    ) -> Result<(), ProductionInlinePreparationError> {
        if !std::ptr::eq(self.prepared, prepared) {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}

/// Select every prepared paragraph exactly once, preserving syntax order. The
/// containing flow supplies the available width for each paragraph; mismatched
/// cardinality cannot silently omit one. Widths are bound into the result.
///
/// Candidate visits share one finite budget. Retained paragraph/line/unit-pen/
/// occurrence/cluster/glyph/control records share the preparation's document
/// fragment ceiling, charged before allocation. No per-paragraph reset occurs.
pub fn layout_production_inline_lines<'p, 'a>(
    prepared: &'p ProductionPreparedInlines<'a>,
    inline_sizes: &[PositiveLength],
    max_candidate_steps: u64,
) -> Result<ProductionInlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let root = NodeId::new(0);
    if inline_sizes.len() != prepared.paragraphs.len() {
        return Err(error(root, E::ReceiptMismatch));
    }
    let mut remaining = prepared.max_fragments;
    charge(&mut remaining, prepared.paragraphs.len(), root)?;
    let mut paragraphs = Vec::new();
    paragraphs
        .try_reserve_exact(prepared.paragraphs.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    let mut budget = ProductionLineBreakBudget::new(max_candidate_steps, remaining);
    let mut digests = Vec::new();
    digests
        .try_reserve_exact(
            prepared
                .paragraphs
                .len()
                .checked_mul(32)
                .and_then(|n| n.checked_add(64))
                .ok_or_else(|| error(root, E::ArithmeticOverflow))?,
        )
        .map_err(|_| error(root, E::AllocationFailure))?;
    digests.extend_from_slice(&sha256(PRODUCTION_INLINE_LINE_LAYOUT_ALGORITHM.as_bytes()));
    digests.extend_from_slice(&prepared.fingerprint());
    for (index, (p, width)) in prepared.paragraphs.iter().zip(inline_sizes).enumerate() {
        let shape = &prepared.shaped.paragraphs()[index];
        let mut placed = ProductionInlineParagraphLineLayout {
            owner: p.owner,
            inline_size: *width,
            font: shape.font(),
            selected: None,
            lines: Vec::new(),
        };
        if let Some(items) = &p.items {
            // Unit pens and kernel SVG occurrences are retained by the selected
            // break result in addition to the projected records below.
            charge(&mut remaining, items.units().len(), p.owner)?;
            charge(
                &mut remaining,
                items
                    .units()
                    .iter()
                    .filter(|u| matches!(u, Unit::Vector(_)))
                    .count(),
                p.owner,
            )?;
            budget.constrain_remaining_lines(remaining / 2);
            let selected = break_production_inline(
                items,
                *width,
                p.line_height
                    .ok_or_else(|| error(p.owner, E::MissingTextStyle))?,
                &mut budget,
            )
            .map_err(|e| error(p.owner, E::Atomic(e)))?;
            charge(
                &mut remaining,
                selected
                    .lines()
                    .len()
                    .checked_mul(2)
                    .ok_or_else(|| error(p.owner, E::ArithmeticOverflow))?,
                p.owner,
            )?;
            placed
                .lines
                .try_reserve_exact(selected.lines().len())
                .map_err(|_| error(p.owner, E::AllocationFailure))?;
            let mut cluster_cursor = 0;
            for line in selected.lines() {
                let baseline = line
                    .line()
                    .metrics()
                    .leading_before()
                    .get()
                    .checked_add(line.line().metrics().content_ascent().get())
                    .ok_or_else(|| error(p.owner, E::ArithmeticOverflow))?;
                let mut output = ProductionPlacedInlineLine {
                    baseline,
                    items: Vec::new(),
                };
                let mut unit = line.line().start_unit();
                let mut vector_cursor = 0;
                while unit < line.line().end_unit() {
                    charge(&mut remaining, 1, p.owner)?;
                    output
                        .items
                        .try_reserve(1)
                        .map_err(|_| error(p.owner, E::AllocationFailure))?;
                    match items.units()[unit as usize] {
                        Unit::Text(_) => {
                            let map = p
                                .glyph_clusters
                                .get(cluster_cursor)
                                .ok_or_else(|| error(p.owner, E::ReceiptMismatch))?;
                            if map.start_unit != unit || map.end_unit > line.line().end_unit() {
                                return Err(error(p.owner, E::ReceiptMismatch));
                            }
                            let run = &shape.runs()[map.run_index as usize];
                            let cluster = &run.glyph_run().clusters[map.cluster_index as usize];
                            let ShapeSourceSpan::Parsed(source_span) = cluster.source_span else {
                                return Err(error(run.owner(), E::ReceiptMismatch));
                            };
                            let site = &prepared.flow.paragraphs()[index].items()
                                [run.site_index() as usize];
                            let ProductionInlineContent::Text { span, utf8 } = site.content()
                            else {
                                return Err(error(run.owner(), E::ReceiptMismatch));
                            };
                            let utf8 = utf8
                                .get(
                                    (source_span.start_byte().get() - span.start_byte().get())
                                        as usize
                                        ..(source_span.end_byte().get() - span.start_byte().get())
                                            as usize,
                                )
                                .ok_or_else(|| error(run.owner(), E::ReceiptMismatch))?;
                            let pen_x = shifted_pen(line, unit, run.owner())?;
                            let mut pen = pen_x;
                            let count = (cluster.glyph_end - cluster.glyph_start) as usize;
                            charge(&mut remaining, count, run.owner())?;
                            let mut glyphs = Vec::new();
                            glyphs
                                .try_reserve_exact(count)
                                .map_err(|_| error(run.owner(), E::AllocationFailure))?;
                            for glyph_index in cluster.glyph_start..cluster.glyph_end {
                                let glyph = &run.glyph_run().glyphs[glyph_index as usize];
                                glyphs.push(ProductionPlacedGlyph {
                                    glyph_index,
                                    glyph,
                                    x: pen
                                        .checked_add(glyph.offset_x)
                                        .ok_or_else(|| error(run.owner(), E::ArithmeticOverflow))?,
                                    y: baseline
                                        .checked_sub(glyph.offset_y)
                                        .ok_or_else(|| error(run.owner(), E::ArithmeticOverflow))?,
                                });
                                pen = pen
                                    .checked_add(glyph.advance_x)
                                    .ok_or_else(|| error(run.owner(), E::ArithmeticOverflow))?;
                            }
                            let Unit::Text(last) = items.units()[map.end_unit as usize - 1] else {
                                return Err(error(run.owner(), E::ReceiptMismatch));
                            };
                            let expected_end = shifted_pen(line, map.end_unit - 1, run.owner())?
                                .checked_add(last.advance().get())
                                .ok_or_else(|| error(run.owner(), E::ArithmeticOverflow))?;
                            if pen != expected_end {
                                return Err(error(run.owner(), E::ReceiptMismatch));
                            }
                            output.items.push(ProductionPlacedInline::Text(
                                ProductionPlacedTextCluster {
                                    run_index: map.run_index,
                                    cluster_index: map.cluster_index,
                                    run,
                                    source_span,
                                    utf8,
                                    pen_x,
                                    glyphs,
                                },
                            ));
                            unit = map.end_unit;
                            cluster_cursor += 1;
                        }
                        Unit::Vector(v) => {
                            let occurrence = *line
                                .line()
                                .occurrences()
                                .get(vector_cursor)
                                .ok_or_else(|| error(v.node_id(), E::ReceiptMismatch))?;
                            let pen = shifted_pen(line, unit, v.node_id())?;
                            if occurrence.unit_index() != unit
                                || occurrence.item() != v
                                || occurrence.pen_x().checked_add(line.origin_shift().get())
                                    != Some(pen)
                            {
                                return Err(error(v.node_id(), E::ReceiptMismatch));
                            }
                            let geometry = v
                                .metrics()
                                .select_inline_geometry(pen, baseline)
                                .map_err(|_| error(v.node_id(), E::ArithmeticOverflow))?;
                            output.items.push(ProductionPlacedInline::Vector(
                                ProductionPlacedInlineVector {
                                    occurrence,
                                    geometry,
                                },
                            ));
                            vector_cursor += 1;
                            unit += 1;
                        }
                        Unit::Break(control) => {
                            output.items.push(ProductionPlacedInline::Break(control));
                            unit += 1;
                        }
                    }
                }
                if vector_cursor != line.line().occurrences().len() {
                    return Err(error(p.owner, E::ReceiptMismatch));
                }
                placed.lines.push(output);
            }
            if cluster_cursor != p.glyph_clusters.len() {
                return Err(error(p.owner, E::ReceiptMismatch));
            }
            placed.selected = Some(selected);
        }
        // Prepared owner binds exact shaped glyphs/source and ordered unit maps;
        // selected owner binds measured pens/width/line metrics. Projection has
        // a distinct algorithm, with width also bound for structure-only records.
        let mut digest = [0u8; 44];
        digest[..4].copy_from_slice(&p.owner.get().to_be_bytes());
        digest[4..12].copy_from_slice(&width.get().raw().to_be_bytes());
        digest[12..].copy_from_slice(
            &placed
                .selected
                .as_ref()
                .map_or([0; 32], ProductionInlineBreak::fingerprint),
        );
        digests.extend_from_slice(&sha256(&digest));
        paragraphs.push(placed);
    }
    Ok(ProductionInlineLineLayout {
        prepared,
        paragraphs,
        output_records: prepared.max_fragments - remaining,
        fingerprint: sha256(&digests),
    })
}

fn shifted_pen(
    line: &ProductionInlineSelectedLine,
    unit: u32,
    owner: NodeId,
) -> Result<Length, ProductionInlinePreparationError> {
    line.unit_pen_x(unit)
        .ok_or_else(|| error(owner, ProductionInlinePreparationErrorKind::ReceiptMismatch))?
        .checked_add(line.origin_shift().get())
        .ok_or_else(|| {
            error(
                owner,
                ProductionInlinePreparationErrorKind::ArithmeticOverflow,
            )
        })
}

fn charge(
    remaining: &mut u64,
    count: usize,
    owner: NodeId,
) -> Result<(), ProductionInlinePreparationError> {
    *remaining = remaining
        .checked_sub(count as u64)
        .ok_or_else(|| error(owner, ProductionInlinePreparationErrorKind::UnitLimit))?;
    Ok(())
}
