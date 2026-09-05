//! Production candidate selection. Source/glyph authorization belongs to the
//! layout bridge; this kernel never turns scalar metrics into PDF text.
use super::*;

pub const PRODUCTION_INLINE_BREAK_ALGORITHM: &str = "typaxis.production-inline-break/3";

/// Explicit zero-width opportunity, with syntax-owned provenance. It is not a
/// space, object-replacement character, or fabricated TextSpan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionExplicitBreak {
    owner: NodeId,
    source_span: SourceSpan,
    kind: BreakKind,
}
impl ProductionExplicitBreak {
    pub fn new(
        owner: NodeId,
        source_span: SourceSpan,
        kind: BreakKind,
    ) -> Result<Self, AtomicVectorInlineError> {
        if kind == BreakKind::Prohibited {
            return Err(AtomicVectorInlineError::InvalidBinding);
        }
        Ok(Self {
            owner,
            source_span,
            kind,
        })
    }
    pub const fn owner(self) -> NodeId {
        self.owner
    }
    pub const fn source_span(self) -> SourceSpan {
        self.source_span
    }
    pub const fn kind(self) -> BreakKind {
        self.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionInlineLogicalUnit {
    Text(AtomicVectorTextUnit),
    Vector(AtomicVectorInlineItem),
    Break(ProductionExplicitBreak),
}
impl From<AtomicVectorInlineLogicalUnit> for ProductionInlineLogicalUnit {
    fn from(value: AtomicVectorInlineLogicalUnit) -> Self {
        match value {
            AtomicVectorInlineLogicalUnit::Text(t) => Self::Text(t),
            AtomicVectorInlineLogicalUnit::Vector(v) => Self::Vector(v),
        }
    }
}

/// Scalar-unit range of one already-shaped cluster. This prevents the Unicode
/// classifier from proposing a break inside a ligature or extended grapheme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionTextClusterRange {
    pub start_unit: u32,
    pub end_unit: u32,
}

pub struct ProductionInlineParagraph {
    owner: NodeId,
    units: Vec<ProductionInlineLogicalUnit>,
    boundaries: Vec<AtomicVectorInlineBoundary>,
    previous_content: Vec<Option<usize>>,
    clusters: Vec<ProductionTextClusterRange>,
    fingerprint: [u8; 32],
}
impl ProductionInlineParagraph {
    /// All text units must be covered exactly once, in order, by cluster ranges.
    /// Vectors remain individual typed AL units. Empty paragraphs are handled by
    /// the containing flow, not by fabricating a zero-width text scalar here.
    pub fn itemize(
        owner: NodeId,
        units: Vec<AtomicVectorInlineLogicalUnit>,
        clusters: Vec<ProductionTextClusterRange>,
        japanese_mode: JapaneseLineBreakMode,
    ) -> Result<Self, AtomicVectorInlineError> {
        let mut typed = Vec::new();
        typed
            .try_reserve_exact(units.len())
            .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
        typed.extend(units.into_iter().map(ProductionInlineLogicalUnit::from));
        Self::itemize_with_breaks(owner, typed, clusters, japanese_mode)
    }

    pub fn itemize_with_breaks(
        owner: NodeId,
        units: Vec<ProductionInlineLogicalUnit>,
        clusters: Vec<ProductionTextClusterRange>,
        japanese_mode: JapaneseLineBreakMode,
    ) -> Result<Self, AtomicVectorInlineError> {
        use ProductionInlineLogicalUnit as U;
        if units.is_empty() {
            return Err(AtomicVectorInlineError::EmptyParagraph);
        }
        u32::try_from(units.len()).map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)?;
        let mut unicode = Vec::new();
        unicode
            .try_reserve_exact(units.len())
            .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
        let mut unicode_positions = Vec::new();
        unicode_positions
            .try_reserve_exact(units.len())
            .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
        let mut previous_content = Vec::new();
        previous_content
            .try_reserve_exact(units.len())
            .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
        let mut previous = None;
        let mut break_nodes = std::collections::BTreeSet::new();
        let mut vector_nodes = std::collections::BTreeSet::new();
        for (index, unit) in units.iter().enumerate() {
            previous_content.push(previous);
            unicode_positions.push(unicode.len());
            match unit {
                U::Text(t) => unicode.push(UnicodeLineBreakUnit::Scalar(t.scalar())),
                U::Vector(v) => {
                    if v.paragraph_node() != owner
                        || !vector_nodes.insert(v.node_id())
                        || !v.matches_bound_placement(v.binding_fingerprint(), v.placement())
                    {
                        return Err(AtomicVectorInlineError::InvalidBinding);
                    }
                    unicode.push(UnicodeLineBreakUnit::SyntheticAl);
                }
                U::Break(b) => {
                    if b.owner == owner || !break_nodes.insert(b.owner) {
                        return Err(AtomicVectorInlineError::InvalidBinding);
                    }
                    if b.kind == BreakKind::Mandatory {
                        unicode.push(UnicodeLineBreakUnit::MandatoryBreak);
                    }
                    continue;
                }
            }
            previous = Some(index);
        }
        if break_nodes.iter().any(|n| vector_nodes.contains(n)) {
            return Err(AtomicVectorInlineError::InvalidBinding);
        }
        let mut unicode_kinds = Vec::new();
        unicode_kinds
            .try_reserve_exact(unicode.len() + 1)
            .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
        unicode_kinds.resize(unicode.len() + 1, None);
        for b in unicode_line_breaks_for_units(&unicode)
            .map_err(|_| AtomicVectorInlineError::UnicodeLineBreak)?
        {
            unicode_kinds[b.unit_offset()] = Some(b.kind());
        }
        let mut boundaries = Vec::new();
        boundaries
            .try_reserve_exact(units.len() - 1)
            .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
        for index in 0..units.len() - 1 {
            let (kind, penalty) = match (units[index], units[index + 1]) {
                (U::Break(b), _) => (b.kind, 0),
                (_, U::Break(_)) => (BreakKind::Prohibited, 0),
                _ => {
                    let scalar = |u| match u {
                        U::Text(t) => Some(t.scalar()),
                        U::Vector(_) => Some('A'),
                        U::Break(_) => None,
                    };
                    let pair = japanese_pair_rule(
                        scalar(units[index]),
                        scalar(units[index + 1]),
                        japanese_mode,
                    );
                    let kind = match unicode_kinds[unicode_positions[index + 1]] {
                        Some(UnicodeBreakKind::Mandatory) => BreakKind::Mandatory,
                        Some(UnicodeBreakKind::Allowed)
                            if pair.permission() == JapanesePairPermission::Preserve =>
                        {
                            BreakKind::Allowed
                        }
                        _ => BreakKind::Prohibited,
                    };
                    (kind, pair.penalty())
                }
            };
            boundaries.push(AtomicVectorInlineBoundary::Text { kind, penalty });
        }
        let mut cursor = 0usize;
        for cluster in &clusters {
            while matches!(units.get(cursor), Some(U::Vector(_) | U::Break(_))) {
                cursor += 1;
            }
            let start = cluster.start_unit as usize;
            let end = cluster.end_unit as usize;
            if start != cursor
                || start >= end
                || end > units.len()
                || units[start..end]
                    .iter()
                    .any(|unit| !matches!(unit, U::Text(_)))
            {
                return Err(AtomicVectorInlineError::InvalidBinding);
            }
            for index in start..end - 1 {
                let boundary = &mut boundaries[index];
                if boundary.kind() == BreakKind::Mandatory {
                    return Err(AtomicVectorInlineError::InvalidBinding);
                }
                *boundary = AtomicVectorInlineBoundary::Text {
                    kind: BreakKind::Prohibited,
                    penalty: boundary.penalty(),
                };
            }
            cursor = end;
        }
        if units[cursor..]
            .iter()
            .any(|unit| !matches!(unit, U::Vector(_) | U::Break(_)))
        {
            return Err(AtomicVectorInlineError::InvalidBinding);
        }
        let mut canonical = String::from(PRODUCTION_INLINE_BREAK_ALGORITHM);
        canonical.push_str(match japanese_mode {
            JapaneseLineBreakMode::Loose => "/loose/",
            JapaneseLineBreakMode::Normal => "/normal/",
            JapaneseLineBreakMode::Strict => "/strict/",
        });
        canonical.push_str(&format!("/{}/", owner.get()));
        for (index, unit) in units.iter().enumerate() {
            match unit {
                U::Text(t) => canonical.push_str(&format!(
                    "/text/{}/{}/{}/{}",
                    t.scalar() as u32,
                    t.advance().get().raw(),
                    t.ascent().get().raw(),
                    t.descent().get().raw()
                )),
                U::Vector(v) => {
                    canonical.push_str("/vector/");
                    push_hash(&mut canonical, v.fingerprint());
                }
                U::Break(b) => {
                    canonical.push_str(&format!(
                        "/break/{}/{}/{}/",
                        index,
                        b.owner.get(),
                        break_kind_str(b.kind)
                    ));
                    push_source_span(&mut canonical, b.source_span);
                }
            }
        }
        for b in &boundaries {
            canonical.push_str(&format!(
                "/boundary/{}/{}",
                break_kind_str(b.kind()),
                b.penalty()
            ));
        }
        for range in &clusters {
            canonical.push_str(&format!("/{},{}", range.start_unit, range.end_unit));
        }
        Ok(Self {
            owner,
            units,
            boundaries,
            previous_content,
            clusters,
            fingerprint: sha256(canonical.as_bytes()),
        })
    }
    pub const fn paragraph_node(&self) -> NodeId {
        self.owner
    }
    pub fn units(&self) -> &[ProductionInlineLogicalUnit] {
        &self.units
    }
    pub fn clusters(&self) -> &[ProductionTextClusterRange] {
        &self.clusters
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    fn gap_before(
        &self,
        index: usize,
        line_start: usize,
    ) -> Result<Length, AtomicVectorInlineError> {
        if matches!(self.units[index], ProductionInlineLogicalUnit::Break(_)) {
            return Ok(Length::ZERO);
        }
        let Some(previous) = self.previous_content[index].filter(|i| *i >= line_start) else {
            return Ok(Length::ZERO);
        };
        let after = match self.units[previous] {
            ProductionInlineLogicalUnit::Vector(v) => v.placement().spacing_after(),
            _ => NonNegativeLength::ZERO,
        };
        let before = match self.units[index] {
            ProductionInlineLogicalUnit::Vector(v) => v.placement().spacing_before(),
            _ => NonNegativeLength::ZERO,
        };
        after
            .get()
            .checked_add(before.get())
            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)
    }
}

/// A containing document stage retains one budget across paragraph calls.
/// Work is charged per visited candidate unit, before its metrics are evaluated.
#[derive(Debug)]
pub struct ProductionLineBreakBudget {
    remaining_steps: u64,
    remaining_lines: u64,
}
impl ProductionLineBreakBudget {
    pub const fn new(max_candidate_steps: u64, max_selected_lines: u64) -> Self {
        Self {
            remaining_steps: max_candidate_steps,
            remaining_lines: max_selected_lines,
        }
    }
    pub const fn remaining_steps(&self) -> u64 {
        self.remaining_steps
    }
    pub const fn remaining_lines(&self) -> u64 {
        self.remaining_lines
    }
    /// Tighten the remaining line allowance to fit a downstream document
    /// allocation budget. This never restores visits or previously used lines.
    pub fn constrain_remaining_lines(&mut self, maximum: u64) {
        self.remaining_lines = self.remaining_lines.min(maximum);
    }
    fn step(&mut self) -> Result<(), AtomicVectorInlineError> {
        self.remaining_steps = self
            .remaining_steps
            .checked_sub(1)
            .ok_or(AtomicVectorInlineError::CandidateLimit)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ProductionInlineSelectedLine {
    unit_pens: Vec<Length>,
    line: AtomicVectorSelectedLine,
    origin_shift: NonNegativeLength,
    required_inline_size: NonNegativeLength,
}
impl ProductionInlineSelectedLine {
    /// Pen after same-line spacing and before this logical unit. Coordinates
    /// exclude origin_shift; controls and zero-advance scalars keep their slots.
    pub fn unit_pen_x(&self, unit_index: u32) -> Option<Length> {
        let offset = unit_index.checked_sub(self.line.start_unit())?;
        self.unit_pens.get(offset as usize).copied()
    }

    /// Pens in `line` are relative to this shifted line origin. Apply the same
    /// shift to body glyphs and vector viewports; do not alter producer metrics.
    pub const fn line(&self) -> &AtomicVectorSelectedLine {
        &self.line
    }
    pub const fn origin_shift(&self) -> NonNegativeLength {
        self.origin_shift
    }
    pub const fn required_inline_size(&self) -> NonNegativeLength {
        self.required_inline_size
    }
}

pub struct ProductionInlineBreak {
    paragraph_fingerprint: [u8; 32],
    lines: Vec<ProductionInlineSelectedLine>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}
impl ProductionInlineBreak {
    pub const fn paragraph_fingerprint(&self) -> [u8; 32] {
        self.paragraph_fingerprint
    }
    pub fn lines(&self) -> &[ProductionInlineSelectedLine] {
        &self.lines
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// Minimum-demerit LTR candidates with cluster-atomic breaks and compensated
/// vector viewport overhang. Candidate extension is O(1), rather than remeasuring
/// every prefix; selected lines alone collect occurrences and vertical metrics.
pub fn break_production_inline(
    paragraph: &ProductionInlineParagraph,
    inline_size: PositiveLength,
    line_height: PositiveLength,
    budget: &mut ProductionLineBreakBudget,
) -> Result<ProductionInlineBreak, AtomicVectorInlineError> {
    let p = paragraph;
    let count = p
        .units
        .len()
        .checked_add(1)
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
    let mut costs = Vec::new();
    costs
        .try_reserve_exact(count)
        .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
    costs.resize(count, None::<i64>);
    let mut previous = Vec::new();
    previous
        .try_reserve_exact(count)
        .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
    previous.resize(count, None::<usize>);
    costs[0] = Some(0);
    for start in 0..p.units.len() {
        let Some(base_cost) = costs[start] else {
            continue;
        };
        let mut cursor = Length::ZERO;
        let mut left = Length::ZERO;
        let mut right = Length::ZERO;
        for index in start..p.units.len() {
            budget.step()?;
            if index > start {
                cursor = cursor
                    .checked_add(p.gap_before(index, start)?)
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
            }
            let advance = match p.units[index] {
                ProductionInlineLogicalUnit::Text(t) => t.advance().get(),
                ProductionInlineLogicalUnit::Break(_) => Length::ZERO,
                ProductionInlineLogicalUnit::Vector(v) => {
                    left = left.min(
                        cursor
                            .checked_add(v.metrics().origin_x())
                            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?,
                    );
                    right = right.max(
                        cursor
                            .checked_add(v.metrics().viewport_right_from_pen())
                            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?,
                    );
                    v.metrics().advance().get()
                }
            };
            cursor = cursor
                .checked_add(advance)
                .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
            right = right.max(cursor);
            let required = right
                .checked_sub(left)
                .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
            // All logical advances/spacing are nonnegative; visual extrema only
            // expand, so a longer candidate cannot repair this overflow.
            if required > inline_size.get() {
                break;
            }
            let end = index + 1;
            let kind = if end == p.units.len() {
                BreakKind::Mandatory
            } else {
                p.boundaries[index].kind()
            };
            if kind != BreakKind::Prohibited {
                let penalty = if end == p.units.len() {
                    0
                } else {
                    p.boundaries[index].penalty()
                };
                let width = NonNegativeLength::new(required)
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
                let cost = base_cost
                    .checked_add(atomic_line_demerits(inline_size, width, penalty)?)
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
                if costs[end].map_or(true, |old| cost < old) {
                    costs[end] = Some(cost);
                    previous[end] = Some(start);
                }
            }
            if kind == BreakKind::Mandatory {
                break;
            }
        }
    }
    if costs[p.units.len()].is_none() {
        return Err(AtomicVectorInlineError::NoFeasibleLine);
    }
    let mut line_count = 0u64;
    let mut cursor = p.units.len();
    while cursor > 0 {
        line_count = line_count
            .checked_add(1)
            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
        if line_count > budget.remaining_lines {
            return Err(AtomicVectorInlineError::SelectionLimit);
        }
        cursor = previous[cursor].ok_or(AtomicVectorInlineError::NoFeasibleLine)?;
    }
    budget.remaining_lines -= line_count;
    let mut lines = Vec::new();
    lines
        .try_reserve_exact(line_count as usize)
        .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
    let mut end = p.units.len();
    while end > 0 {
        let start = previous[end].ok_or(AtomicVectorInlineError::NoFeasibleLine)?;
        let measured = measure_production_line(p, start, end, line_height)?;
        let left = measured
            .visual_left
            .unwrap_or(Length::ZERO)
            .min(Length::ZERO);
        let right = measured
            .visual_right
            .unwrap_or(Length::ZERO)
            .max(measured.logical_advance.get());
        let required = right
            .checked_sub(left)
            .and_then(NonNegativeLength::new)
            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
        if required.get() > inline_size.get() {
            return Err(AtomicVectorInlineError::InvalidBinding);
        }
        lines.push(ProductionInlineSelectedLine {
            unit_pens: measured.unit_pens,
            origin_shift: Length::ZERO
                .checked_sub(left)
                .and_then(NonNegativeLength::new)
                .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?,
            required_inline_size: required,
            line: AtomicVectorSelectedLine {
                line_index: u32::try_from(line_count - 1 - lines.len() as u64)
                    .map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)?,
                start_unit: start as u32,
                end_unit: end as u32,
                logical_advance: measured.logical_advance,
                visual_left: measured.visual_left,
                visual_right: measured.visual_right,
                metrics: measured.metrics,
                occurrences: measured.occurrences,
                break_kind: if end == p.units.len() {
                    BreakKind::Mandatory
                } else {
                    p.boundaries[end - 1].kind()
                },
                break_penalty: if end == p.units.len() {
                    0
                } else {
                    p.boundaries[end - 1].penalty()
                },
                break_demerits: costs[end].ok_or(AtomicVectorInlineError::InvalidBinding)?,
            },
        });
        end = start;
    }
    lines.reverse();
    let mut canonical = String::from("{\"algorithm\":");
    push_jcs_string(&mut canonical, PRODUCTION_INLINE_BREAK_ALGORITHM);
    canonical.push_str(&format!(
        ",\"inline_size\":{},\"line_height\":{},\"lines\":[",
        inline_size.get().raw(),
        line_height.get().raw()
    ));
    for (index, value) in lines.iter().enumerate() {
        if index != 0 {
            canonical.push(',');
        }
        canonical.push_str(&format!(
            "{{\"end\":{},\"origin_shift\":{},\"required\":{},\"start\":{},\"unit_pens\":[",
            value.line.end_unit,
            value.origin_shift.get().raw(),
            value.required_inline_size.get().raw(),
            value.line.start_unit
        ));
        for (unit, pen) in value.unit_pens.iter().enumerate() {
            if unit != 0 {
                canonical.push(',');
            }
            canonical.push_str(&pen.raw().to_string());
        }
        canonical.push_str("]}");
    }
    canonical.push_str("],\"paragraph_fingerprint\":");
    push_hash(&mut canonical, paragraph.fingerprint);
    canonical.push('}');
    Ok(ProductionInlineBreak {
        paragraph_fingerprint: paragraph.fingerprint,
        lines,
        fingerprint: sha256(canonical.as_bytes()),
        canonical_jcs: canonical,
    })
}

struct ProductionMeasuredLine {
    unit_pens: Vec<Length>,
    logical_advance: NonNegativeLength,
    visual_left: Option<Length>,
    visual_right: Option<Length>,
    metrics: AtomicVectorLineMetrics,
    occurrences: Vec<AtomicVectorLineOccurrence>,
}

fn measure_production_line(
    p: &ProductionInlineParagraph,
    start: usize,
    end: usize,
    line_height: PositiveLength,
) -> Result<ProductionMeasuredLine, AtomicVectorInlineError> {
    use ProductionInlineLogicalUnit as U;
    let mut cursor = Length::ZERO;
    let mut ascent = Length::ZERO;
    let mut descent = Length::ZERO;
    let mut visual_left: Option<Length> = None;
    let mut visual_right: Option<Length> = None;
    let mut unit_pens = Vec::new();
    unit_pens
        .try_reserve_exact(end - start)
        .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
    let mut occurrences = Vec::new();
    occurrences
        .try_reserve_exact(
            p.units[start..end]
                .iter()
                .filter(|u| matches!(u, U::Vector(_)))
                .count(),
        )
        .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
    // The last painted content item owns after-spacing suppression even when
    // zero-width explicit breaks follow it on this same selected line.
    let last_content = (start..end)
        .rev()
        .find(|i| !matches!(p.units[*i], U::Break(_)));
    for index in start..end {
        cursor = cursor
            .checked_add(p.gap_before(index, start)?)
            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
        unit_pens.push(cursor);
        match p.units[index] {
            U::Text(t) => {
                ascent = ascent.max(t.ascent().get());
                descent = descent.max(t.descent().get());
                cursor = cursor
                    .checked_add(t.advance().get())
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
            }
            U::Vector(v) => {
                let m = v.metrics();
                ascent = ascent.max(m.ascent().get());
                descent = descent.max(m.descent().get());
                let left = cursor
                    .checked_add(m.origin_x())
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
                let right = cursor
                    .checked_add(m.viewport_right_from_pen())
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
                visual_left = Some(visual_left.map_or(left, |old| old.min(left)));
                visual_right = Some(visual_right.map_or(right, |old| old.max(right)));
                occurrences.push(AtomicVectorLineOccurrence {
                    unit_index: index as u32,
                    item: v,
                    pen_x: cursor,
                    spacing_before: if p.previous_content[index].is_some_and(|i| i >= start) {
                        v.placement().spacing_before()
                    } else {
                        NonNegativeLength::ZERO
                    },
                    spacing_after: if last_content == Some(index) {
                        NonNegativeLength::ZERO
                    } else {
                        v.placement().spacing_after()
                    },
                });
                cursor = cursor
                    .checked_add(m.advance().get())
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
            }
            U::Break(_) => (),
        }
    }
    Ok(ProductionMeasuredLine {
        unit_pens,
        logical_advance: NonNegativeLength::new(cursor)
            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?,
        visual_left,
        visual_right,
        metrics: compute_inline_line_metrics(ascent, descent, line_height)?,
        occurrences,
    })
}
