//! Production candidate selection. Source/glyph authorization belongs to the
//! layout bridge; this kernel never turns scalar metrics into PDF text.
use super::*;

pub const PRODUCTION_INLINE_BREAK_ALGORITHM: &str = "typaxis.production-inline-break/1";

/// Scalar-unit range of one already-shaped cluster. This prevents the Unicode
/// classifier from proposing a break inside a ligature or extended grapheme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionTextClusterRange {
    pub start_unit: u32,
    pub end_unit: u32,
}

pub struct ProductionInlineParagraph {
    inner: AtomicVectorInlineParagraph,
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
        u32::try_from(units.len()).map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)?;
        let mut inner =
            AtomicVectorInlineParagraph::itemize_for_owner(units, japanese_mode, Some(owner))?;
        let mut cursor = 0usize;
        for cluster in &clusters {
            while matches!(
                inner.units.get(cursor),
                Some(AtomicVectorInlineLogicalUnit::Vector(_))
            ) {
                cursor += 1;
            }
            let start = cluster.start_unit as usize;
            let end = cluster.end_unit as usize;
            if start != cursor
                || start >= end
                || end > inner.units.len()
                || inner.units[start..end]
                    .iter()
                    .any(|unit| !matches!(unit, AtomicVectorInlineLogicalUnit::Text(_)))
            {
                return Err(AtomicVectorInlineError::InvalidBinding);
            }
            for index in start..end - 1 {
                let boundary = &mut inner.boundaries[index];
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
        if inner.units[cursor..]
            .iter()
            .any(|unit| !matches!(unit, AtomicVectorInlineLogicalUnit::Vector(_)))
        {
            return Err(AtomicVectorInlineError::InvalidBinding);
        }
        let mut canonical = String::from(PRODUCTION_INLINE_BREAK_ALGORITHM);
        canonical.push_str(match japanese_mode {
            JapaneseLineBreakMode::Loose => "/loose/",
            JapaneseLineBreakMode::Normal => "/normal/",
            JapaneseLineBreakMode::Strict => "/strict/",
        });
        let inner_jcs = encode_itemization(&inner);
        inner.fingerprint = sha256(inner_jcs.as_bytes());
        canonical.push_str(&inner_jcs);
        for range in &clusters {
            canonical.push_str(&format!("/{},{}", range.start_unit, range.end_unit));
        }
        Ok(Self {
            inner,
            clusters,
            fingerprint: sha256(canonical.as_bytes()),
        })
    }
    pub const fn paragraph_node(&self) -> NodeId {
        self.inner.paragraph_node
    }
    pub fn units(&self) -> &[AtomicVectorInlineLogicalUnit] {
        &self.inner.units
    }
    pub fn clusters(&self) -> &[ProductionTextClusterRange] {
        &self.clusters
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
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
    line: AtomicVectorSelectedLine,
    origin_shift: NonNegativeLength,
    required_inline_size: NonNegativeLength,
}
impl ProductionInlineSelectedLine {
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
    let p = &paragraph.inner;
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
                    .checked_add(p.boundaries[index - 1].same_line_width().get())
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
            }
            let advance = match p.units[index] {
                AtomicVectorInlineLogicalUnit::Text(t) => t.advance().get(),
                AtomicVectorInlineLogicalUnit::Vector(v) => {
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
        let measured = measure_line(p, start, end, inline_size, line_height, true)?;
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
            "{{\"end\":{},\"origin_shift\":{},\"required\":{},\"start\":{}}}",
            value.line.end_unit,
            value.origin_shift.get().raw(),
            value.required_inline_size.get().raw(),
            value.line.start_unit
        ));
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
