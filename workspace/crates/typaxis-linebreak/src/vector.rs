use crate::{
    japanese_pair_rule, unicode_line_breaks_for_units, BreakKind, JapaneseLineBreakMode,
    JapanesePairPermission, UnicodeBreakKind, UnicodeLineBreakUnit,
};
use typaxis_core::{
    push_jcs_string, sha256, Length, NodeId, NonNegativeLength, PositiveLength, SourceSpan,
};
use typaxis_layout_contract::{
    BoundPrecomposedVectorMetrics, PrecomposedVectorBindingFingerprint,
    PrecomposedVectorInlinePlacementInput,
};

pub const ATOMIC_VECTOR_INLINE_ALGORITHM: &str = "typaxis.atomic-vector-inline/1";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AtomicVectorInlineKind {
    InlineVector,
    MathVector,
}

impl AtomicVectorInlineKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InlineVector => "inline_vector",
            Self::MathVector => "math_vector",
        }
    }
}

/// Source-owned identity paired with the synthetic `AL` unit. It deliberately
/// contains no substituted Unicode scalar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicVectorSyntheticAlUnit {
    node_id: NodeId,
    paragraph_node: NodeId,
    source_span: SourceSpan,
    binding_fingerprint: PrecomposedVectorBindingFingerprint,
}

impl AtomicVectorSyntheticAlUnit {
    pub const fn node_id(self) -> NodeId {
        self.node_id
    }

    pub const fn paragraph_node(self) -> NodeId {
        self.paragraph_node
    }

    pub const fn source_span(self) -> SourceSpan {
        self.source_span
    }

    pub const fn binding_fingerprint(self) -> PrecomposedVectorBindingFingerprint {
        self.binding_fingerprint
    }

    pub const fn line_break_unit(self) -> UnicodeLineBreakUnit {
        UnicodeLineBreakUnit::SyntheticAl
    }

    /// The first horizontal profile treats each vector as one atomic LTR
    /// isolate. Reordering inside the resource is never exposed to UAX #9.
    pub const fn is_atomic_ltr_isolate(self) -> bool {
        true
    }
}

/// One producer-composed vector in the line breaker's atomic namespace.
/// Metrics and spacing are copied from a resource-bound placement receipt;
/// no SVG path or internal outline is represented here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicVectorInlineItem {
    node_id: NodeId,
    paragraph_node: NodeId,
    source_span: SourceSpan,
    kind: AtomicVectorInlineKind,
    binding_fingerprint: PrecomposedVectorBindingFingerprint,
    placement: PrecomposedVectorInlinePlacementInput,
    fingerprint: [u8; 32],
}

impl AtomicVectorInlineItem {
    #[doc(hidden)]
    pub fn from_bound_placement(
        node_id: NodeId,
        paragraph_node: NodeId,
        source_span: SourceSpan,
        kind: AtomicVectorInlineKind,
        binding_fingerprint: PrecomposedVectorBindingFingerprint,
        placement: PrecomposedVectorInlinePlacementInput,
    ) -> Result<Self, AtomicVectorInlineError> {
        if node_id == paragraph_node {
            return Err(AtomicVectorInlineError::InvalidBinding);
        }
        let mut item = Self {
            node_id,
            paragraph_node,
            source_span,
            kind,
            binding_fingerprint,
            placement,
            fingerprint: [0; 32],
        };
        item.fingerprint = sha256(encode_atomic_item(&item).as_bytes());
        Ok(item)
    }

    pub const fn node_id(self) -> NodeId {
        self.node_id
    }

    pub const fn paragraph_node(self) -> NodeId {
        self.paragraph_node
    }

    pub const fn source_span(self) -> SourceSpan {
        self.source_span
    }

    pub const fn kind(self) -> AtomicVectorInlineKind {
        self.kind
    }

    pub const fn binding_fingerprint(self) -> PrecomposedVectorBindingFingerprint {
        self.binding_fingerprint
    }

    pub const fn placement(self) -> PrecomposedVectorInlinePlacementInput {
        self.placement
    }

    pub const fn metrics(self) -> BoundPrecomposedVectorMetrics {
        self.placement.metrics()
    }

    pub const fn fingerprint(self) -> [u8; 32] {
        self.fingerprint
    }

    pub const fn synthetic_al_unit(self) -> AtomicVectorSyntheticAlUnit {
        AtomicVectorSyntheticAlUnit {
            node_id: self.node_id,
            paragraph_node: self.paragraph_node,
            source_span: self.source_span,
            binding_fingerprint: self.binding_fingerprint,
        }
    }

    pub fn matches_bound_placement(
        self,
        binding_fingerprint: PrecomposedVectorBindingFingerprint,
        placement: PrecomposedVectorInlinePlacementInput,
    ) -> bool {
        self.binding_fingerprint == binding_fingerprint
            && self.placement == placement
            && self.fingerprint == sha256(encode_atomic_item(&self).as_bytes())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicVectorTextUnit {
    scalar: char,
    advance: NonNegativeLength,
    ascent: NonNegativeLength,
    descent: NonNegativeLength,
}

impl AtomicVectorTextUnit {
    pub const fn new(
        scalar: char,
        advance: NonNegativeLength,
        ascent: NonNegativeLength,
        descent: NonNegativeLength,
    ) -> Self {
        Self {
            scalar,
            advance,
            ascent,
            descent,
        }
    }

    pub const fn scalar(self) -> char {
        self.scalar
    }

    pub const fn advance(self) -> NonNegativeLength {
        self.advance
    }

    pub const fn ascent(self) -> NonNegativeLength {
        self.ascent
    }

    pub const fn descent(self) -> NonNegativeLength {
        self.descent
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtomicVectorInlineLogicalUnit {
    Text(AtomicVectorTextUnit),
    Vector(AtomicVectorInlineItem),
}

impl AtomicVectorInlineLogicalUnit {
    fn line_break_unit(self) -> UnicodeLineBreakUnit {
        match self {
            Self::Text(value) => UnicodeLineBreakUnit::Scalar(value.scalar()),
            Self::Vector(value) => value.synthetic_al_unit().line_break_unit(),
        }
    }

    /// Synthetic AL participates in Japanese pair tailoring as the existing
    /// Latin class. Natural/stretch/shrink values are deliberately discarded
    /// at vector boundaries in favor of producer-specified exact spacing.
    fn japanese_pair_scalar(self) -> char {
        match self {
            Self::Text(value) => value.scalar(),
            Self::Vector(_) => 'A',
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VectorBoundaryBranch {
    SameLine,
    Break,
}

/// Exactly one conditional spacing and break record for a logical boundary
/// adjacent to at least one atomic vector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VectorBoundaryItem {
    logical_boundary: u32,
    kind: BreakKind,
    penalty: i32,
    left_after: NonNegativeLength,
    right_before: NonNegativeLength,
    same_line_width: NonNegativeLength,
}

impl VectorBoundaryItem {
    pub const fn logical_boundary(self) -> u32 {
        self.logical_boundary
    }

    pub const fn kind(self) -> BreakKind {
        self.kind
    }

    pub const fn penalty(self) -> i32 {
        self.penalty
    }

    pub const fn left_after(self) -> NonNegativeLength {
        self.left_after
    }

    pub const fn right_before(self) -> NonNegativeLength {
        self.right_before
    }

    pub const fn same_line_width(self) -> NonNegativeLength {
        self.same_line_width
    }

    pub const fn width_for(self, branch: VectorBoundaryBranch) -> NonNegativeLength {
        match branch {
            VectorBoundaryBranch::SameLine => self.same_line_width,
            VectorBoundaryBranch::Break => NonNegativeLength::ZERO,
        }
    }

    pub const fn pre_break_width(self) -> NonNegativeLength {
        NonNegativeLength::ZERO
    }

    pub const fn post_break_width(self) -> NonNegativeLength {
        NonNegativeLength::ZERO
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AtomicVectorInlineBoundary {
    Text { kind: BreakKind, penalty: i32 },
    Vector(VectorBoundaryItem),
}

impl AtomicVectorInlineBoundary {
    const fn kind(self) -> BreakKind {
        match self {
            Self::Text { kind, .. } | Self::Vector(VectorBoundaryItem { kind, .. }) => kind,
        }
    }

    const fn penalty(self) -> i32 {
        match self {
            Self::Text { penalty, .. } | Self::Vector(VectorBoundaryItem { penalty, .. }) => {
                penalty
            }
        }
    }

    const fn same_line_width(self) -> NonNegativeLength {
        match self {
            Self::Text { .. } => NonNegativeLength::ZERO,
            Self::Vector(value) => value.same_line_width(),
        }
    }

    const fn vector(self) -> Option<VectorBoundaryItem> {
        match self {
            Self::Text { .. } => None,
            Self::Vector(value) => Some(value),
        }
    }
}

/// Complete typed logical sequence and its single boundary record per
/// adjacency. Atomic vectors remain one unit and one item throughout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtomicVectorInlineParagraph {
    paragraph_node: NodeId,
    units: Vec<AtomicVectorInlineLogicalUnit>,
    boundaries: Vec<AtomicVectorInlineBoundary>,
    vector_count: u32,
    fingerprint: [u8; 32],
}

impl AtomicVectorInlineParagraph {
    pub fn itemize(
        units: Vec<AtomicVectorInlineLogicalUnit>,
        japanese_mode: JapaneseLineBreakMode,
    ) -> Result<Self, AtomicVectorInlineError> {
        if units.is_empty() {
            return Err(AtomicVectorInlineError::EmptyParagraph);
        }
        let Some(paragraph_node) = units.iter().find_map(|unit| match unit {
            AtomicVectorInlineLogicalUnit::Text(_) => None,
            AtomicVectorInlineLogicalUnit::Vector(value) => Some(value.paragraph_node()),
        }) else {
            return Err(AtomicVectorInlineError::MissingVector);
        };
        let mut vector_nodes = std::collections::BTreeSet::new();
        for unit in &units {
            if let AtomicVectorInlineLogicalUnit::Vector(value) = unit {
                if value.paragraph_node() != paragraph_node
                    || !vector_nodes.insert(value.node_id())
                    || !value
                        .matches_bound_placement(value.binding_fingerprint(), value.placement())
                {
                    return Err(AtomicVectorInlineError::InvalidBinding);
                }
            }
        }

        let mut unicode_units = Vec::new();
        unicode_units
            .try_reserve_exact(units.len())
            .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
        unicode_units.extend(
            units
                .iter()
                .copied()
                .map(AtomicVectorInlineLogicalUnit::line_break_unit),
        );
        let unicode_breaks = unicode_line_breaks_for_units(&unicode_units)
            .map_err(|_| AtomicVectorInlineError::UnicodeLineBreak)?;
        let unicode_boundary_count = units
            .len()
            .checked_add(1)
            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
        let mut unicode_kinds = Vec::new();
        unicode_kinds
            .try_reserve_exact(unicode_boundary_count)
            .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
        unicode_kinds.resize(unicode_boundary_count, None);
        for boundary in unicode_breaks {
            let slot = unicode_kinds
                .get_mut(boundary.unit_offset())
                .ok_or(AtomicVectorInlineError::UnicodeLineBreak)?;
            if slot.replace(boundary.kind()).is_some() {
                return Err(AtomicVectorInlineError::UnicodeLineBreak);
            }
        }

        let boundary_count = units.len() - 1;
        let mut boundaries = Vec::new();
        boundaries
            .try_reserve_exact(boundary_count)
            .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
        for logical_boundary in 1..units.len() {
            let pair = japanese_pair_rule(
                Some(units[logical_boundary - 1].japanese_pair_scalar()),
                Some(units[logical_boundary].japanese_pair_scalar()),
                japanese_mode,
            );
            let kind = match unicode_kinds[logical_boundary] {
                Some(UnicodeBreakKind::Mandatory) => BreakKind::Mandatory,
                Some(UnicodeBreakKind::Allowed)
                    if pair.permission() == JapanesePairPermission::Preserve =>
                {
                    BreakKind::Allowed
                }
                Some(UnicodeBreakKind::Allowed) | None => BreakKind::Prohibited,
            };
            let left_after = match units[logical_boundary - 1] {
                AtomicVectorInlineLogicalUnit::Vector(value) => value.placement().spacing_after(),
                AtomicVectorInlineLogicalUnit::Text(_) => NonNegativeLength::ZERO,
            };
            let right_before = match units[logical_boundary] {
                AtomicVectorInlineLogicalUnit::Vector(value) => value.placement().spacing_before(),
                AtomicVectorInlineLogicalUnit::Text(_) => NonNegativeLength::ZERO,
            };
            if left_after != NonNegativeLength::ZERO || right_before != NonNegativeLength::ZERO {
                let same_line_width = checked_nonnegative_add(left_after, right_before)?;
                boundaries.push(AtomicVectorInlineBoundary::Vector(VectorBoundaryItem {
                    logical_boundary: u32::try_from(logical_boundary)
                        .map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)?,
                    kind,
                    penalty: pair.penalty(),
                    left_after,
                    right_before,
                    same_line_width,
                }));
            } else if matches!(
                (units[logical_boundary - 1], units[logical_boundary]),
                (
                    AtomicVectorInlineLogicalUnit::Vector(_),
                    AtomicVectorInlineLogicalUnit::Text(_)
                ) | (
                    AtomicVectorInlineLogicalUnit::Text(_),
                    AtomicVectorInlineLogicalUnit::Vector(_)
                ) | (
                    AtomicVectorInlineLogicalUnit::Vector(_),
                    AtomicVectorInlineLogicalUnit::Vector(_)
                )
            ) {
                boundaries.push(AtomicVectorInlineBoundary::Vector(VectorBoundaryItem {
                    logical_boundary: u32::try_from(logical_boundary)
                        .map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)?,
                    kind,
                    penalty: pair.penalty(),
                    left_after,
                    right_before,
                    same_line_width: NonNegativeLength::ZERO,
                }));
            } else {
                boundaries.push(AtomicVectorInlineBoundary::Text {
                    kind,
                    penalty: pair.penalty(),
                });
            }
        }
        let vector_count = u32::try_from(vector_nodes.len())
            .map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)?;
        let mut paragraph = Self {
            paragraph_node,
            units,
            boundaries,
            vector_count,
            fingerprint: [0; 32],
        };
        paragraph.fingerprint = sha256(encode_itemization(&paragraph).as_bytes());
        Ok(paragraph)
    }

    pub const fn paragraph_node(&self) -> NodeId {
        self.paragraph_node
    }

    pub fn units(&self) -> &[AtomicVectorInlineLogicalUnit] {
        &self.units
    }

    pub const fn vector_count(&self) -> u32 {
        self.vector_count
    }

    pub fn vector_boundaries(&self) -> impl Iterator<Item = VectorBoundaryItem> + '_ {
        self.boundaries
            .iter()
            .copied()
            .filter_map(AtomicVectorInlineBoundary::vector)
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicVectorLineMetrics {
    content_ascent: NonNegativeLength,
    content_descent: NonNegativeLength,
    leading_before: NonNegativeLength,
    leading_after: NonNegativeLength,
    line_height: PositiveLength,
}

impl AtomicVectorLineMetrics {
    pub const fn content_ascent(self) -> NonNegativeLength {
        self.content_ascent
    }

    pub const fn content_descent(self) -> NonNegativeLength {
        self.content_descent
    }

    pub const fn leading_before(self) -> NonNegativeLength {
        self.leading_before
    }

    pub const fn leading_after(self) -> NonNegativeLength {
        self.leading_after
    }

    pub const fn line_height(self) -> PositiveLength {
        self.line_height
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicVectorLineOccurrence {
    unit_index: u32,
    item: AtomicVectorInlineItem,
    pen_x: Length,
    spacing_before: NonNegativeLength,
    spacing_after: NonNegativeLength,
}

impl AtomicVectorLineOccurrence {
    pub const fn unit_index(self) -> u32 {
        self.unit_index
    }

    pub const fn item(self) -> AtomicVectorInlineItem {
        self.item
    }

    pub const fn pen_x(self) -> Length {
        self.pen_x
    }

    pub const fn spacing_before(self) -> NonNegativeLength {
        self.spacing_before
    }

    pub const fn spacing_after(self) -> NonNegativeLength {
        self.spacing_after
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtomicVectorSelectedLine {
    line_index: u32,
    start_unit: u32,
    end_unit: u32,
    logical_advance: NonNegativeLength,
    visual_left: Option<Length>,
    visual_right: Option<Length>,
    metrics: AtomicVectorLineMetrics,
    break_kind: BreakKind,
    break_penalty: i32,
    break_demerits: i64,
    occurrences: Vec<AtomicVectorLineOccurrence>,
}

impl AtomicVectorSelectedLine {
    pub const fn line_index(&self) -> u32 {
        self.line_index
    }

    pub const fn start_unit(&self) -> u32 {
        self.start_unit
    }

    pub const fn end_unit(&self) -> u32 {
        self.end_unit
    }

    pub const fn logical_advance(&self) -> NonNegativeLength {
        self.logical_advance
    }

    pub const fn visual_left(&self) -> Option<Length> {
        self.visual_left
    }

    pub const fn visual_right(&self) -> Option<Length> {
        self.visual_right
    }

    pub const fn metrics(&self) -> AtomicVectorLineMetrics {
        self.metrics
    }

    pub const fn break_kind(&self) -> BreakKind {
        self.break_kind
    }

    pub const fn break_penalty(&self) -> i32 {
        self.break_penalty
    }

    /// Cumulative demerits through this selected line.
    pub const fn break_demerits(&self) -> i64 {
        self.break_demerits
    }

    pub fn occurrences(&self) -> &[AtomicVectorLineOccurrence] {
        &self.occurrences
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtomicVectorInlineBreak {
    itemization_fingerprint: [u8; 32],
    inline_size: PositiveLength,
    computed_line_height: PositiveLength,
    lines: Vec<AtomicVectorSelectedLine>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl AtomicVectorInlineBreak {
    pub const fn itemization_fingerprint(&self) -> [u8; 32] {
        self.itemization_fingerprint
    }

    pub const fn inline_size(&self) -> PositiveLength {
        self.inline_size
    }

    pub const fn computed_line_height(&self) -> PositiveLength {
        self.computed_line_height
    }

    pub fn lines(&self) -> &[AtomicVectorSelectedLine] {
        &self.lines
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtomicVectorInlineError {
    InvalidBinding,
    EmptyParagraph,
    MissingVector,
    UnicodeLineBreak,
    InvalidLineSize,
    NoFeasibleLine,
    Oversize(NodeId),
    SelectionLimit,
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for AtomicVectorInlineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBinding => formatter.write_str("I9190: atomic vector binding mismatch"),
            Self::EmptyParagraph => formatter.write_str("L5100: empty atomic vector paragraph"),
            Self::MissingVector => {
                formatter.write_str("L5100: atomic vector paragraph has no vector")
            }
            Self::UnicodeLineBreak => {
                formatter.write_str("L5100: atomic vector Unicode line-break failure")
            }
            Self::InvalidLineSize => formatter.write_str("L5100: invalid atomic vector line size"),
            Self::NoFeasibleLine => formatter.write_str("L5100: no feasible atomic vector line"),
            Self::Oversize(owner) => write!(
                formatter,
                "L5100: inline vector {} exceeds an empty line",
                owner.get()
            ),
            Self::SelectionLimit => {
                formatter.write_str("L5110: atomic vector selected line limit exceeded")
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("L5100: atomic vector line arithmetic overflow")
            }
            Self::AllocationFailure => {
                formatter.write_str("L5100: atomic vector line allocation failed")
            }
        }
    }
}

impl std::error::Error for AtomicVectorInlineError {}

/// Selects the minimum-demerit sequence of legal boundaries. Candidate cost
/// and logical fit use each vector's `advance`; visual fit independently
/// checks its viewport interval. Equal-cost candidates retain the first
/// source-order predecessor, and an overfull atomic unit is never emitted as
/// a fallback line. `max_selected_lines` is the caller-owned remaining
/// containing-fragment budget; it is checked before line and occurrence
/// records are allocated.
pub fn break_atomic_vector_inline(
    paragraph: &AtomicVectorInlineParagraph,
    inline_size: PositiveLength,
    computed_line_height: PositiveLength,
    max_selected_lines: u64,
) -> Result<AtomicVectorInlineBreak, AtomicVectorInlineError> {
    let state_count = paragraph
        .units
        .len()
        .checked_add(1)
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
    let mut cumulative_costs = Vec::new();
    cumulative_costs
        .try_reserve_exact(state_count)
        .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
    cumulative_costs.resize(state_count, None);
    cumulative_costs[0] = Some(0i64);
    let mut predecessors = Vec::new();
    predecessors
        .try_reserve_exact(state_count)
        .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
    predecessors.resize(state_count, None);

    for start in 0..paragraph.units.len() {
        let Some(predecessor_cost) = cumulative_costs[start] else {
            continue;
        };
        for end in start + 1..=paragraph.units.len() {
            if end > start + 1 && paragraph.boundaries[end - 2].kind() == BreakKind::Mandatory {
                break;
            }
            let (legal, mandatory) = if end == paragraph.units.len() {
                (true, true)
            } else {
                let kind = paragraph.boundaries[end - 1].kind();
                (kind != BreakKind::Prohibited, kind == BreakKind::Mandatory)
            };
            if legal {
                let measured = measure_line(
                    paragraph,
                    start,
                    end,
                    inline_size,
                    computed_line_height,
                    false,
                )?;
                if measured.fits {
                    let penalty = if end == paragraph.units.len() {
                        0
                    } else {
                        paragraph.boundaries[end - 1].penalty()
                    };
                    let edge =
                        atomic_line_demerits(inline_size, measured.logical_advance, penalty)?;
                    let cumulative = predecessor_cost
                        .checked_add(edge)
                        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
                    if cumulative_costs[end].map_or(true, |current| cumulative < current) {
                        cumulative_costs[end] = Some(cumulative);
                        predecessors[end] = Some(start);
                    }
                }
            }
            if mandatory {
                break;
            }
        }
    }

    if cumulative_costs[paragraph.units.len()].is_none() {
        for (index, unit) in paragraph.units.iter().copied().enumerate() {
            if let AtomicVectorInlineLogicalUnit::Vector(item) = unit {
                let measured = measure_line(
                    paragraph,
                    index,
                    index + 1,
                    inline_size,
                    computed_line_height,
                    false,
                )?;
                if !measured.fits {
                    return Err(AtomicVectorInlineError::Oversize(item.node_id()));
                }
            }
        }
        return Err(AtomicVectorInlineError::NoFeasibleLine);
    }

    let mut selected_line_count = 0u64;
    let mut cursor = paragraph.units.len();
    while cursor > 0 {
        selected_line_count = selected_line_count
            .checked_add(1)
            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
        if selected_line_count > max_selected_lines {
            return Err(AtomicVectorInlineError::SelectionLimit);
        }
        cursor = predecessors[cursor].ok_or(AtomicVectorInlineError::NoFeasibleLine)?;
    }

    let mut selected_ends = Vec::new();
    selected_ends
        .try_reserve_exact(
            usize::try_from(selected_line_count)
                .map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)?,
        )
        .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
    cursor = paragraph.units.len();
    while cursor > 0 {
        selected_ends.push(cursor);
        cursor = predecessors[cursor].ok_or(AtomicVectorInlineError::NoFeasibleLine)?;
    }
    selected_ends.reverse();

    let mut lines = Vec::new();
    lines
        .try_reserve_exact(selected_ends.len())
        .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
    let mut start = 0usize;
    for end in selected_ends {
        let measured = measure_line(
            paragraph,
            start,
            end,
            inline_size,
            computed_line_height,
            true,
        )?;
        let (break_kind, break_penalty) = if end == paragraph.units.len() {
            (BreakKind::Mandatory, 0)
        } else {
            (
                paragraph.boundaries[end - 1].kind(),
                paragraph.boundaries[end - 1].penalty(),
            )
        };
        let break_demerits =
            cumulative_costs[end].ok_or(AtomicVectorInlineError::NoFeasibleLine)?;
        lines.push(AtomicVectorSelectedLine {
            line_index: u32::try_from(lines.len())
                .map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)?,
            start_unit: u32::try_from(start)
                .map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)?,
            end_unit: u32::try_from(end)
                .map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)?,
            logical_advance: measured.logical_advance,
            visual_left: measured.visual_left,
            visual_right: measured.visual_right,
            metrics: measured.metrics,
            break_kind,
            break_penalty,
            break_demerits,
            occurrences: measured.occurrences,
        });
        start = end;
    }
    let canonical_jcs = encode_selected_break(
        paragraph.fingerprint,
        inline_size,
        computed_line_height,
        &lines,
    );
    Ok(AtomicVectorInlineBreak {
        itemization_fingerprint: paragraph.fingerprint,
        inline_size,
        computed_line_height,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
        lines,
    })
}

fn atomic_line_demerits(
    target: PositiveLength,
    logical_advance: NonNegativeLength,
    penalty: i32,
) -> Result<i64, AtomicVectorInlineError> {
    let remaining = i128::from(target.get().raw())
        .checked_sub(i128::from(logical_advance.get().raw()))
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
    if remaining < 0 {
        return Err(AtomicVectorInlineError::InvalidLineSize);
    }
    let ratio_milli = remaining
        .checked_mul(1_000)
        .and_then(|value| value.checked_div(i128::from(target.get().raw())))
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
    let badness = ratio_milli
        .checked_mul(ratio_milli)
        .and_then(|value| value.checked_mul(ratio_milli))
        .and_then(|value| value.checked_div(1_000_000))
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
    let penalty = i128::from(penalty);
    let penalty_cost = penalty
        .checked_mul(penalty)
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
    let total = if penalty >= 0 {
        badness.checked_add(penalty_cost)
    } else {
        badness.checked_sub(penalty_cost)
    }
    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?
    .max(0);
    i64::try_from(total).map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)
}

struct MeasuredLine {
    fits: bool,
    logical_advance: NonNegativeLength,
    visual_left: Option<Length>,
    visual_right: Option<Length>,
    metrics: AtomicVectorLineMetrics,
    occurrences: Vec<AtomicVectorLineOccurrence>,
}

fn measure_line(
    paragraph: &AtomicVectorInlineParagraph,
    start: usize,
    end: usize,
    inline_size: PositiveLength,
    computed_line_height: PositiveLength,
    collect_occurrences: bool,
) -> Result<MeasuredLine, AtomicVectorInlineError> {
    if start >= end || end > paragraph.units.len() {
        return Err(AtomicVectorInlineError::InvalidLineSize);
    }
    let mut cursor = Length::ZERO;
    let mut content_ascent = Length::ZERO;
    let mut content_descent = Length::ZERO;
    let mut visual_left: Option<Length> = None;
    let mut visual_right: Option<Length> = None;
    let vector_count = if collect_occurrences {
        paragraph.units[start..end]
            .iter()
            .filter(|unit| matches!(unit, AtomicVectorInlineLogicalUnit::Vector(_)))
            .count()
    } else {
        0
    };
    let mut occurrences = Vec::new();
    occurrences
        .try_reserve_exact(vector_count)
        .map_err(|_| AtomicVectorInlineError::AllocationFailure)?;
    for index in start..end {
        if index > start {
            cursor = cursor
                .checked_add(paragraph.boundaries[index - 1].same_line_width().get())
                .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
        }
        match paragraph.units[index] {
            AtomicVectorInlineLogicalUnit::Text(value) => {
                content_ascent = content_ascent.max(value.ascent().get());
                content_descent = content_descent.max(value.descent().get());
                cursor = cursor
                    .checked_add(value.advance().get())
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
            }
            AtomicVectorInlineLogicalUnit::Vector(item) => {
                let metrics = item.metrics();
                content_ascent = content_ascent.max(metrics.ascent().get());
                content_descent = content_descent.max(metrics.descent().get());
                let left = cursor
                    .checked_add(metrics.origin_x())
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
                let right = cursor
                    .checked_add(metrics.viewport_right_from_pen())
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
                visual_left = Some(visual_left.map_or(left, |current| current.min(left)));
                visual_right = Some(visual_right.map_or(right, |current| current.max(right)));
                if collect_occurrences {
                    occurrences.push(AtomicVectorLineOccurrence {
                        unit_index: u32::try_from(index)
                            .map_err(|_| AtomicVectorInlineError::ArithmeticOverflow)?,
                        item,
                        pen_x: cursor,
                        spacing_before: if index == start {
                            NonNegativeLength::ZERO
                        } else {
                            item.placement().spacing_before()
                        },
                        spacing_after: if index + 1 == end {
                            NonNegativeLength::ZERO
                        } else {
                            item.placement().spacing_after()
                        },
                    });
                }
                cursor = cursor
                    .checked_add(metrics.advance().get())
                    .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
            }
        }
    }
    let logical_advance =
        NonNegativeLength::new(cursor).ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
    let content_height = content_ascent
        .checked_add(content_descent)
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
    let extra = computed_line_height
        .get()
        .raw()
        .checked_sub(content_height.raw())
        .unwrap_or(0)
        .max(0);
    let leading_before_raw = round_nonnegative_half_ties_even(extra)?;
    let leading_after_raw = extra
        .checked_sub(leading_before_raw)
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
    let line_height_raw = leading_before_raw
        .checked_add(content_height.raw())
        .and_then(|value| value.checked_add(leading_after_raw))
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
    let metrics = AtomicVectorLineMetrics {
        content_ascent: NonNegativeLength::new(content_ascent)
            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?,
        content_descent: NonNegativeLength::new(content_descent)
            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?,
        leading_before: nonnegative_from_raw(leading_before_raw)?,
        leading_after: nonnegative_from_raw(leading_after_raw)?,
        line_height: positive_from_raw(line_height_raw)?,
    };
    let fits = logical_advance.get().raw() <= inline_size.get().raw()
        && visual_left.map_or(true, |left| left.raw() >= 0)
        && visual_right.map_or(true, |right| right.raw() <= inline_size.get().raw());
    Ok(MeasuredLine {
        fits,
        logical_advance,
        visual_left,
        visual_right,
        metrics,
        occurrences,
    })
}

fn round_nonnegative_half_ties_even(value: i64) -> Result<i64, AtomicVectorInlineError> {
    if value < 0 {
        return Err(AtomicVectorInlineError::ArithmeticOverflow);
    }
    let quotient = value / 2;
    if value % 2 == 0 || quotient % 2 == 0 {
        Ok(quotient)
    } else {
        quotient
            .checked_add(1)
            .ok_or(AtomicVectorInlineError::ArithmeticOverflow)
    }
}

fn checked_nonnegative_add(
    left: NonNegativeLength,
    right: NonNegativeLength,
) -> Result<NonNegativeLength, AtomicVectorInlineError> {
    let sum = left
        .get()
        .checked_add(right.get())
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)?;
    NonNegativeLength::new(sum).ok_or(AtomicVectorInlineError::ArithmeticOverflow)
}

fn nonnegative_from_raw(value: i64) -> Result<NonNegativeLength, AtomicVectorInlineError> {
    Length::from_raw(value)
        .and_then(NonNegativeLength::new)
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)
}

fn positive_from_raw(value: i64) -> Result<PositiveLength, AtomicVectorInlineError> {
    Length::from_raw(value)
        .and_then(PositiveLength::new)
        .ok_or(AtomicVectorInlineError::ArithmeticOverflow)
}

fn encode_atomic_item(value: &AtomicVectorInlineItem) -> String {
    let metrics = value.metrics();
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ATOMIC_VECTOR_INLINE_ALGORITHM);
    output.push_str(",\"bidi\":\"ltr_isolate\",\"binding_fingerprint\":");
    push_hash(&mut output, value.binding_fingerprint.bytes());
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, value.kind.as_str());
    output.push_str(",\"line_break_class\":\"AL\",\"metrics\":");
    push_metrics(&mut output, metrics);
    output.push_str(",\"node_id\":");
    output.push_str(&value.node_id.get().to_string());
    output.push_str(",\"paint\":{\"blue\":");
    output.push_str(&value.placement.paint().blue().to_string());
    output.push_str(",\"green\":");
    output.push_str(&value.placement.paint().green().to_string());
    output.push_str(",\"red\":");
    output.push_str(&value.placement.paint().red().to_string());
    output.push_str("},\"paragraph_node\":");
    output.push_str(&value.paragraph_node.get().to_string());
    output.push_str(",\"record\":\"item\",\"scale\":");
    output.push_str(&value.placement.scale().get().raw().to_string());
    output.push_str(",\"source_span\":");
    push_source_span(&mut output, value.source_span);
    output.push_str(",\"spacing\":{\"after\":");
    output.push_str(&value.placement.spacing_after().get().raw().to_string());
    output.push_str(",\"before\":");
    output.push_str(&value.placement.spacing_before().get().raw().to_string());
    output.push_str("}}");
    output
}

fn encode_itemization(value: &AtomicVectorInlineParagraph) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ATOMIC_VECTOR_INLINE_ALGORITHM);
    output.push_str(",\"boundaries\":[");
    for (index, boundary) in value.boundaries.iter().copied().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"kind\":");
        push_jcs_string(&mut output, break_kind_str(boundary.kind()));
        output.push_str(",\"logical_boundary\":");
        output.push_str(&(index + 1).to_string());
        output.push_str(",\"penalty\":");
        output.push_str(&boundary.penalty().to_string());
        output.push_str(",\"same_line_width\":");
        output.push_str(&boundary.same_line_width().get().raw().to_string());
        output.push('}');
    }
    output.push_str("],\"paragraph_node\":");
    output.push_str(&value.paragraph_node.get().to_string());
    output.push_str(",\"record\":\"itemization\",\"units\":[");
    for (index, unit) in value.units.iter().copied().enumerate() {
        if index > 0 {
            output.push(',');
        }
        match unit {
            AtomicVectorInlineLogicalUnit::Text(text) => {
                output.push_str("{\"advance\":");
                output.push_str(&text.advance().get().raw().to_string());
                output.push_str(",\"ascent\":");
                output.push_str(&text.ascent().get().raw().to_string());
                output.push_str(",\"descent\":");
                output.push_str(&text.descent().get().raw().to_string());
                output.push_str(",\"kind\":\"text\",\"scalar\":");
                push_jcs_string(&mut output, &text.scalar().to_string());
                output.push('}');
            }
            AtomicVectorInlineLogicalUnit::Vector(item) => {
                output.push_str("{\"atomic_fingerprint\":");
                push_hash(&mut output, item.fingerprint());
                output.push_str(",\"kind\":\"vector\",\"node_id\":");
                output.push_str(&item.node_id().get().to_string());
                output.push('}');
            }
        }
    }
    output.push_str("],\"vector_count\":");
    output.push_str(&value.vector_count.to_string());
    output.push('}');
    output
}

fn encode_selected_break(
    itemization_fingerprint: [u8; 32],
    inline_size: PositiveLength,
    computed_line_height: PositiveLength,
    lines: &[AtomicVectorSelectedLine],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ATOMIC_VECTOR_INLINE_ALGORITHM);
    output.push_str(",\"computed_line_height\":");
    output.push_str(&computed_line_height.get().raw().to_string());
    output.push_str(",\"inline_size\":");
    output.push_str(&inline_size.get().raw().to_string());
    output.push_str(",\"itemization_fingerprint\":");
    push_hash(&mut output, itemization_fingerprint);
    output.push_str(",\"lines\":[");
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"break_kind\":");
        push_jcs_string(&mut output, break_kind_str(line.break_kind));
        output.push_str(",\"break_penalty\":");
        output.push_str(&line.break_penalty.to_string());
        output.push_str(",\"demerits\":");
        output.push_str(&line.break_demerits.to_string());
        output.push_str(",\"end_unit\":");
        output.push_str(&line.end_unit.to_string());
        output.push_str(",\"line_height\":");
        output.push_str(&line.metrics.line_height().get().raw().to_string());
        output.push_str(",\"line_index\":");
        output.push_str(&line.line_index.to_string());
        output.push_str(",\"logical_advance\":");
        output.push_str(&line.logical_advance.get().raw().to_string());
        output.push_str(",\"start_unit\":");
        output.push_str(&line.start_unit.to_string());
        output.push_str(",\"vector_count\":");
        output.push_str(&line.occurrences.len().to_string());
        output.push('}');
    }
    output.push_str("],\"record\":\"line_selection\"}");
    output
}

fn push_metrics(output: &mut String, value: BoundPrecomposedVectorMetrics) {
    output.push_str("{\"advance\":");
    output.push_str(&value.advance().get().raw().to_string());
    output.push_str(",\"ascent\":");
    output.push_str(&value.ascent().get().raw().to_string());
    output.push_str(",\"baseline\":");
    output.push_str(&value.baseline().get().raw().to_string());
    output.push_str(",\"descent\":");
    output.push_str(&value.descent().get().raw().to_string());
    output.push_str(",\"origin_x\":");
    output.push_str(&value.origin_x().raw().to_string());
    output.push_str(",\"viewport\":{\"height\":");
    output.push_str(&value.viewport_height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&value.viewport_width().get().raw().to_string());
    output.push_str("}}");
}

fn push_source_span(output: &mut String, value: SourceSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&value.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&value.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&value.start_byte().get().to_string());
    output.push('}');
}

fn push_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

const fn break_kind_str(value: BreakKind) -> &'static str {
    match value {
        BreakKind::Allowed => "allowed",
        BreakKind::Mandatory => "mandatory",
        BreakKind::Prohibited => "prohibited",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{SourceId, Utf8ByteOffset};
    use typaxis_document::{
        PrecomposedVectorMetrics, PrecomposedVectorSpacing, PrecomposedVectorViewport,
    };
    use typaxis_layout_contract::ResolvedRgb8;

    fn length(value: i64) -> Length {
        Length::from_raw(value).unwrap()
    }

    fn nonnegative(value: i64) -> NonNegativeLength {
        NonNegativeLength::new(length(value)).unwrap()
    }

    fn positive(value: i64) -> PositiveLength {
        PositiveLength::new(length(value)).unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn vector(
        node: u32,
        advance: i64,
        ascent: i64,
        descent: i64,
        origin_x: i64,
        baseline: i64,
        viewport_width: i64,
        viewport_height: i64,
        before: i64,
        after: i64,
    ) -> AtomicVectorInlineItem {
        let placement = PrecomposedVectorInlinePlacementInput::from_validated_metrics(
            PrecomposedVectorMetrics {
                advance: positive(advance),
                ascent: positive(ascent),
                baseline: nonnegative(baseline),
                descent: nonnegative(descent),
                origin_x: length(origin_x),
                viewport: PrecomposedVectorViewport {
                    width: positive(viewport_width),
                    height: positive(viewport_height),
                },
            },
            PrecomposedVectorSpacing {
                before: nonnegative(before),
                after: nonnegative(after),
            },
            positive(viewport_width),
            positive(viewport_height),
            ResolvedRgb8::BLACK,
        )
        .unwrap();
        AtomicVectorInlineItem::from_bound_placement(
            NodeId::new(node),
            NodeId::new(1),
            SourceSpan::new(
                SourceId::new(0),
                Utf8ByteOffset::new(node),
                Utf8ByteOffset::new(node + 1),
            )
            .unwrap(),
            AtomicVectorInlineKind::MathVector,
            PrecomposedVectorBindingFingerprint::from_receipt([node as u8; 32]),
            placement,
        )
        .unwrap()
    }

    fn text(scalar: char, advance: i64) -> AtomicVectorInlineLogicalUnit {
        AtomicVectorInlineLogicalUnit::Text(AtomicVectorTextUnit::new(
            scalar,
            nonnegative(advance),
            nonnegative(700),
            nonnegative(300),
        ))
    }

    #[test]
    fn atomic_vector_inline_is_one_synthetic_al_unit_without_source_substitution() {
        let item = vector(2, 1_900, 900, 300, 0, 800, 1_835, 1_100, 17, 19);
        let paragraph = AtomicVectorInlineParagraph::itemize(
            vec![
                text('日', 1_000),
                AtomicVectorInlineLogicalUnit::Vector(item),
                text('語', 1_000),
            ],
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();

        assert_eq!(paragraph.vector_count(), 1);
        assert_eq!(paragraph.units().len(), 3);
        assert_eq!(paragraph.vector_boundaries().count(), 2);
        assert!(paragraph.units().iter().all(|unit| !matches!(
            unit,
            AtomicVectorInlineLogicalUnit::Text(value) if value.scalar() == '\u{fffc}'
        )));
        let synthetic = item.synthetic_al_unit();
        assert_eq!(synthetic.node_id(), NodeId::new(2));
        assert_eq!(
            synthetic.line_break_unit(),
            UnicodeLineBreakUnit::SyntheticAl
        );
        assert!(synthetic.is_atomic_ltr_isolate());
        assert!(item.matches_bound_placement(item.binding_fingerprint(), item.placement()));
        assert_ne!(
            ATOMIC_VECTOR_INLINE_ALGORITHM,
            typaxis_math::MATH_COMPUTATION_ID
        );
    }

    #[test]
    fn atomic_vector_inline_preserves_empty_source_provenance() {
        let original = vector(2, 1_900, 900, 300, 0, 800, 1_835, 1_100, 17, 19);
        let empty_span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(2),
            Utf8ByteOffset::new(2),
        )
        .unwrap();
        let item = AtomicVectorInlineItem::from_bound_placement(
            NodeId::new(3),
            NodeId::new(1),
            empty_span,
            AtomicVectorInlineKind::InlineVector,
            original.binding_fingerprint(),
            original.placement(),
        )
        .unwrap();

        assert!(item.source_span().range().is_empty());
        assert_eq!(item.synthetic_al_unit().source_span(), empty_span);
    }

    #[test]
    fn atomic_vector_inline_uses_advance_and_dynamic_max_metrics() {
        let cases = [
            // fraction-equality
            (
                5, 4_653_056, 1_179_648, 458_752, 1_114_112, 4_587_520, 1_572_864,
            ),
            // sum
            (
                6, 2_818_048, 1_179_648, 458_752, 1_114_112, 2_752_512, 1_572_864,
            ),
            // integral
            (
                7, 2_424_832, 1_376_256, 524_288, 1_310_720, 2_359_296, 1_835_008,
            ),
            // scripts
            (
                8, 2_162_688, 983_040, 393_216, 917_504, 2_097_152, 1_310_720,
            ),
            // large-brackets
            (
                9, 2_686_976, 1_179_648, 458_752, 1_114_112, 2_621_440, 1_572_864,
            ),
            // matrix
            (
                10, 4_259_840, 1_376_256, 524_288, 1_310_720, 4_194_304, 1_835_008,
            ),
        ];
        let units = cases
            .iter()
            .map(
                |(node, advance, ascent, descent, baseline, width, height)| {
                    AtomicVectorInlineLogicalUnit::Vector(vector(
                        *node, *advance, *ascent, *descent, 0, *baseline, *width, *height, 16_384,
                        16_384,
                    ))
                },
            )
            .collect();
        let paragraph =
            AtomicVectorInlineParagraph::itemize(units, JapaneseLineBreakMode::Normal).unwrap();
        let selected = break_atomic_vector_inline(
            &paragraph,
            positive(30_000_000),
            positive(2_097_153),
            u64::MAX,
        )
        .unwrap();
        assert_eq!(selected.lines().len(), 1);
        let line = &selected.lines()[0];
        let expected_advance: i64 = cases.iter().map(|case| case.1).sum::<i64>() + 32_768 * 5;
        assert_eq!(line.logical_advance().get().raw(), expected_advance);
        assert_ne!(
            line.logical_advance().get().raw(),
            cases.iter().map(|case| case.5).sum::<i64>() + 32_768 * 5
        );
        assert_eq!(line.metrics().content_ascent().get().raw(), 1_376_256);
        assert_eq!(line.metrics().content_descent().get().raw(), 524_288);
        assert_eq!(line.metrics().leading_before().get().raw(), 98_304);
        assert_eq!(line.metrics().leading_after().get().raw(), 98_305);
        assert_eq!(line.metrics().line_height().get().raw(), 2_097_153);
        assert_eq!(line.occurrences().len(), 6);
    }

    #[test]
    fn atomic_vector_inline_moves_whole_item_and_rejects_visual_overhang() {
        let item = vector(2, 40, 9, 3, 0, 8, 35, 11, 5, 7);
        let paragraph = AtomicVectorInlineParagraph::itemize(
            vec![text('日', 70), AtomicVectorInlineLogicalUnit::Vector(item)],
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();
        let selected =
            break_atomic_vector_inline(&paragraph, positive(100), positive(14), u64::MAX).unwrap();
        assert_eq!(selected.lines().len(), 2);
        assert!(selected.lines()[0].occurrences().is_empty());
        let moved = selected.lines()[1].occurrences()[0];
        assert_eq!(moved.unit_index(), 1);
        assert_eq!(moved.pen_x(), Length::ZERO);
        assert_eq!(moved.spacing_before(), NonNegativeLength::ZERO);
        assert_eq!(moved.spacing_after(), NonNegativeLength::ZERO);

        for overhang in [
            vector(3, 90, 9, 3, -1, 8, 90, 11, 0, 0),
            vector(4, 90, 9, 3, 11, 8, 90, 11, 0, 0),
        ] {
            let paragraph = AtomicVectorInlineParagraph::itemize(
                vec![AtomicVectorInlineLogicalUnit::Vector(overhang)],
                JapaneseLineBreakMode::Normal,
            )
            .unwrap();
            assert_eq!(
                break_atomic_vector_inline(&paragraph, positive(100), positive(14), u64::MAX),
                Err(AtomicVectorInlineError::Oversize(overhang.node_id()))
            );
        }
    }

    #[test]
    fn atomic_vector_inline_uses_advance_in_optimal_cost_selection() {
        let item = vector(2, 40, 9, 3, 0, 8, 35, 11, 0, 0);
        let paragraph = AtomicVectorInlineParagraph::itemize(
            vec![
                text('日', 40),
                AtomicVectorInlineLogicalUnit::Vector(item),
                text('語', 40),
            ],
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();
        let selected =
            break_atomic_vector_inline(&paragraph, positive(100), positive(14), u64::MAX).unwrap();

        assert_eq!(selected.lines().len(), 2);
        assert_eq!(selected.lines()[0].end_unit(), 1);
        assert_eq!(selected.lines()[1].end_unit(), 3);
        assert_eq!(selected.lines()[0].logical_advance().get().raw(), 40);
        assert_eq!(selected.lines()[1].logical_advance().get().raw(), 80);
        assert_eq!(selected.lines()[0].break_demerits(), 2_716);
        assert_eq!(selected.lines()[1].break_demerits(), 2_724);
        assert_eq!(
            break_atomic_vector_inline(&paragraph, positive(100), positive(14), 1),
            Err(AtomicVectorInlineError::SelectionLimit)
        );
    }

    #[test]
    fn vector_japanese_boundaries_preserve_prohibitions_and_exact_spacing() {
        let item = vector(2, 40, 9, 3, 0, 8, 35, 11, 7, 11);
        let paragraph = AtomicVectorInlineParagraph::itemize(
            vec![
                text('（', 20),
                AtomicVectorInlineLogicalUnit::Vector(item),
                text('）', 20),
            ],
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();
        let boundaries: Vec<_> = paragraph.vector_boundaries().collect();
        assert_eq!(boundaries.len(), 2);
        assert_eq!(boundaries[0].kind(), BreakKind::Prohibited);
        assert_eq!(boundaries[1].kind(), BreakKind::Prohibited);
        assert_eq!(boundaries[0].same_line_width().get().raw(), 7);
        assert_eq!(boundaries[1].same_line_width().get().raw(), 11);
        assert_eq!(
            boundaries[0].width_for(VectorBoundaryBranch::Break),
            NonNegativeLength::ZERO
        );
        assert_eq!(boundaries[0].pre_break_width(), NonNegativeLength::ZERO);
        assert_eq!(boundaries[0].post_break_width(), NonNegativeLength::ZERO);

        let japanese_latin =
            japanese_pair_rule(Some('日'), Some('A'), JapaneseLineBreakMode::Normal);
        assert!(japanese_latin.natural_gap_per_1024_em() > 0);
        let mixed = AtomicVectorInlineParagraph::itemize(
            vec![text('日', 20), AtomicVectorInlineLogicalUnit::Vector(item)],
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();
        let boundary = mixed.vector_boundaries().next().unwrap();
        assert_eq!(boundary.kind(), BreakKind::Allowed);
        assert_eq!(boundary.penalty(), japanese_latin.penalty());
        assert_eq!(boundary.same_line_width().get().raw(), 7);

        let right = vector(3, 50, 10, 4, 0, 9, 45, 13, 13, 17);
        let adjacent = AtomicVectorInlineParagraph::itemize(
            vec![
                AtomicVectorInlineLogicalUnit::Vector(item),
                AtomicVectorInlineLogicalUnit::Vector(right),
            ],
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();
        let boundaries: Vec<_> = adjacent.vector_boundaries().collect();
        assert_eq!(boundaries.len(), 1);
        assert_eq!(boundaries[0].same_line_width().get().raw(), 11 + 13);
        assert_eq!(boundaries[0].left_after().get().raw(), 11);
        assert_eq!(boundaries[0].right_before().get().raw(), 13);
    }
}
