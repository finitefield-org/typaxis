use std::collections::BTreeSet;

use typaxis_core::{
    push_jcs_string, sha256, AffineTransform, ImageResourceId, Length, NodeId, Rect, Unitless16_16,
};
use typaxis_document::{StagingM4Block, StagingM4FigurePlacement};
use typaxis_resource_admission::{
    AdmittedImageMediaKind, AdmittedResourceLedger, VectorContentKey,
};
use typaxis_syntax::{PrecomposedVectorKind, ValidatedStagingSemanticPackage};

use crate::{
    SelectedStructureBindingReceiptV2, SelectedStructurePaintBindingV2,
    SelectedStructurePaintOwner, StagingDrawVectorV2Relation, StagingPrecomposedVectorDisplay,
    StagingSafeVectorDisplay, StructureOwner, StructureRegistryReceiptV2, StructureRole,
};

pub const STAGING_COMBINED_VECTOR_DISPLAY_ALGORITHM_V2: &str = "typaxis.combined-vector-display/2";

/// The common semantic kind used after the legacy Figure and precomposed
/// vector Display streams have been joined. This keeps Figure representable
/// without adding it to the four-kind precomposed-vector syntax enum.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingCombinedVectorKindV2 {
    Figure,
    InlineVector,
    MathVector,
    VectorFigure,
    MathVectorBlock,
}

impl StagingCombinedVectorKindV2 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Figure => "figure",
            Self::InlineVector => "inline_vector",
            Self::MathVector => "math_vector",
            Self::VectorFigure => "vector_figure",
            Self::MathVectorBlock => "math_vector_block",
        }
    }

    pub const fn precomposed(self) -> Option<PrecomposedVectorKind> {
        match self {
            Self::Figure => None,
            Self::InlineVector => Some(PrecomposedVectorKind::InlineVector),
            Self::MathVector => Some(PrecomposedVectorKind::MathVector),
            Self::VectorFigure => Some(PrecomposedVectorKind::VectorFigure),
            Self::MathVectorBlock => Some(PrecomposedVectorKind::MathVectorBlock),
        }
    }
}

impl From<PrecomposedVectorKind> for StagingCombinedVectorKindV2 {
    fn from(value: PrecomposedVectorKind) -> Self {
        match value {
            PrecomposedVectorKind::InlineVector => Self::InlineVector,
            PrecomposedVectorKind::MathVector => Self::MathVector,
            PrecomposedVectorKind::VectorFigure => Self::VectorFigure,
            PrecomposedVectorKind::MathVectorBlock => Self::MathVectorBlock,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingCombinedVectorUsageRelationV2 {
    Figure {
        occurrence: u32,
        placement: StagingM4FigurePlacement,
    },
    Precomposed(StagingDrawVectorV2Relation),
}

impl StagingCombinedVectorUsageRelationV2 {
    pub const fn figure_occurrence(&self) -> Option<u32> {
        match self {
            Self::Figure { occurrence, .. } => Some(*occurrence),
            Self::Precomposed(_) => None,
        }
    }

    pub const fn figure_placement(&self) -> Option<StagingM4FigurePlacement> {
        match self {
            Self::Figure { placement, .. } => Some(*placement),
            Self::Precomposed(_) => None,
        }
    }

    pub const fn precomposed(&self) -> Option<&StagingDrawVectorV2Relation> {
        match self {
            Self::Figure { .. } => None,
            Self::Precomposed(value) => Some(value),
        }
    }
}

/// One normalized vector paint. Usage IDs remain the dense precomposed IDs
/// followed by dense Figure occurrences; physical paint ordering is carried
/// independently by `(page_index, paint_ordinal)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingCombinedVectorUsageV2 {
    usage_id: u32,
    owner: NodeId,
    kind: StagingCombinedVectorKindV2,
    image_id: ImageResourceId,
    content_key: VectorContentKey,
    ir_fingerprint: [u8; 32],
    binding_fingerprint: Option<[u8; 32]>,
    selected_placement_fingerprint: [u8; 32],
    page_index: u32,
    frame_index: u32,
    fragment_ordinal: u32,
    paint_ordinal: u32,
    viewport: Rect,
    scale: i32,
    matrix: AffineTransform,
    resolved_current_color: [u8; 3],
    display_command_fingerprint: [u8; 32],
    relation: StagingCombinedVectorUsageRelationV2,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingCombinedVectorUsageV2 {
    pub const fn usage_id(&self) -> u32 {
        self.usage_id
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn kind(&self) -> StagingCombinedVectorKindV2 {
        self.kind
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn content_key(&self) -> &VectorContentKey {
        &self.content_key
    }
    pub const fn ir_fingerprint(&self) -> [u8; 32] {
        self.ir_fingerprint
    }
    pub const fn binding_fingerprint(&self) -> Option<[u8; 32]> {
        self.binding_fingerprint
    }
    pub const fn selected_placement_fingerprint(&self) -> [u8; 32] {
        self.selected_placement_fingerprint
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn frame_index(&self) -> u32 {
        self.frame_index
    }
    pub const fn fragment_ordinal(&self) -> u32 {
        self.fragment_ordinal
    }
    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }
    pub const fn viewport(&self) -> Rect {
        self.viewport
    }
    pub const fn scale_raw(&self) -> i32 {
        self.scale
    }
    pub const fn matrix(&self) -> AffineTransform {
        self.matrix
    }
    pub const fn resolved_current_color(&self) -> [u8; 3] {
        self.resolved_current_color
    }
    pub const fn display_command_fingerprint(&self) -> [u8; 32] {
        self.display_command_fingerprint
    }
    pub const fn relation(&self) -> &StagingCombinedVectorUsageRelationV2 {
        &self.relation
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingCombinedVectorDisplayReceiptV2 {
    package_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    admitted_sha256: [u8; 32],
    profile_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    binding_set_sha256: [u8; 32],
    precomposed_display_sha256: [u8; 32],
    figure_display_sha256: Option<[u8; 32]>,
    structure_registry_sha256: [u8; 32],
    selected_binding_sha256: [u8; 32],
    page_count: u32,
    usage_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingCombinedVectorDisplayReceiptV2 {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn semantic_sha256(&self) -> [u8; 32] {
        self.semantic_sha256
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub const fn profile_sha256(&self) -> [u8; 32] {
        self.profile_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub const fn binding_set_sha256(&self) -> [u8; 32] {
        self.binding_set_sha256
    }
    pub const fn precomposed_display_sha256(&self) -> [u8; 32] {
        self.precomposed_display_sha256
    }
    pub const fn figure_display_sha256(&self) -> Option<[u8; 32]> {
        self.figure_display_sha256
    }
    pub const fn structure_registry_sha256(&self) -> [u8; 32] {
        self.structure_registry_sha256
    }
    pub const fn selected_binding_sha256(&self) -> [u8; 32] {
        self.selected_binding_sha256
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn usage_count(&self) -> u32 {
        self.usage_count
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingCombinedVectorDisplayV2 {
    usages: Vec<StagingCombinedVectorUsageV2>,
    receipt: StagingCombinedVectorDisplayReceiptV2,
}

impl StagingCombinedVectorDisplayV2 {
    pub fn usages(&self) -> &[StagingCombinedVectorUsageV2] {
        &self.usages
    }
    pub const fn receipt(&self) -> &StagingCombinedVectorDisplayReceiptV2 {
        &self.receipt
    }

    pub fn verify_resource_closure(&self) -> Result<(), StagingCombinedVectorDisplayErrorV2> {
        if usize::try_from(self.receipt.usage_count) != Ok(self.usages.len())
            || self.receipt.page_count == 0
            || self.usages.iter().enumerate().any(|(index, usage)| {
                usize::try_from(usage.usage_id) != Ok(index)
                    || usage.page_index >= self.receipt.page_count
                    || usage.canonical_jcs != encode_usage(usage)
                    || usage.fingerprint != sha256(usage.canonical_jcs.as_bytes())
            })
            || self
                .usages
                .iter()
                .map(|usage| (usage.page_index, usage.paint_ordinal))
                .collect::<BTreeSet<_>>()
                .len()
                != self.usages.len()
        {
            return Err(StagingCombinedVectorDisplayErrorV2::ReceiptMismatch);
        }
        let canonical = encode_receipt(&self.receipt, &self.usages);
        if self.receipt.canonical_jcs != canonical
            || self.receipt.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingCombinedVectorDisplayErrorV2::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingCombinedVectorDisplayErrorV2 {
    PrecomposedDisplayMismatch,
    FigureDisplayMismatch,
    AdmissionMismatch(ImageResourceId),
    StructureMismatch(NodeId),
    SelectedPaintMismatch(NodeId),
    CountOverflow,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingCombinedVectorDisplayErrorV2 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "I9190: combined vector Display /2 {self:?}")
    }
}

impl std::error::Error for StagingCombinedVectorDisplayErrorV2 {}

pub fn build_staging_combined_vector_display_v2(
    package: &ValidatedStagingSemanticPackage,
    precomposed: &StagingPrecomposedVectorDisplay,
    figures: Option<&StagingSafeVectorDisplay>,
    admitted: &AdmittedResourceLedger,
    registry: &StructureRegistryReceiptV2,
    selected: &SelectedStructureBindingReceiptV2,
) -> Result<StagingCombinedVectorDisplayV2, StagingCombinedVectorDisplayErrorV2> {
    precomposed
        .verify_resource_closure()
        .map_err(|_| StagingCombinedVectorDisplayErrorV2::PrecomposedDisplayMismatch)?;
    let page_count = u32::try_from(selected.pages().len())
        .map_err(|_| StagingCombinedVectorDisplayErrorV2::CountOverflow)?;
    if precomposed.receipt().package_sha256() != package.canonical_jcs_sha256()
        || precomposed.receipt().admitted_fingerprint() != admitted.fingerprint().bytes()
        || selected.structure_registry_sha256() != registry.fingerprint()
        || selected.limits_sha256() != precomposed.receipt().limits_fingerprint()
        || precomposed.receipt().page_count() != page_count
    {
        return Err(StagingCombinedVectorDisplayErrorV2::ReceiptMismatch);
    }
    if let Some(figures) = figures {
        figures
            .verify_resource_closure()
            .map_err(|_| StagingCombinedVectorDisplayErrorV2::FigureDisplayMismatch)?;
        if figures.receipt().package_fingerprint() != package.semantic_fingerprint() {
            return Err(StagingCombinedVectorDisplayErrorV2::FigureDisplayMismatch);
        }
        if figures.receipt().limits_fingerprint() != precomposed.receipt().limits_fingerprint()
            || figures.receipt().page_geometry_fingerprint()
                != precomposed.receipt().page_geometry_fingerprint()
            || usize::try_from(page_count) != Ok(figures.pages().len())
        {
            return Err(StagingCombinedVectorDisplayErrorV2::FigureDisplayMismatch);
        }
    }
    validate_figure_coverage(package, admitted, figures)?;

    let precomposed_count = precomposed.receipt().command_count();
    let figure_count = figures.map_or(0, |value| value.receipt().command_count());
    let usage_count = precomposed_count
        .checked_add(figure_count)
        .ok_or(StagingCombinedVectorDisplayErrorV2::CountOverflow)?;
    let mut usages = Vec::new();
    usages
        .try_reserve_exact(
            usize::try_from(usage_count)
                .map_err(|_| StagingCombinedVectorDisplayErrorV2::CountOverflow)?,
        )
        .map_err(|_| StagingCombinedVectorDisplayErrorV2::AllocationFailure)?;

    for command in precomposed.commands() {
        let node = registry.source_node(command.owner()).ok_or(
            StagingCombinedVectorDisplayErrorV2::StructureMismatch(command.owner()),
        )?;
        let paint = selected
            .paints()
            .iter()
            .find(|paint| {
                paint.page_index() == command.page_index()
                    && paint.paint_ordinal() == command.paint_ordinal()
                    && paint.owner()
                        == SelectedStructurePaintOwner::Structure(node.structure_node_id())
                    && matches!(
                        paint.binding(),
                        SelectedStructurePaintBindingV2::Vector(binding)
                            if binding.usage_id == command.usage_id()
                                && binding.kind == command.kind()
                                && binding.display_command_fingerprint == command.fingerprint()
                    )
            })
            .ok_or(StagingCombinedVectorDisplayErrorV2::SelectedPaintMismatch(
                command.owner(),
            ))?;
        let color = command.resolved_current_color();
        let mut usage = StagingCombinedVectorUsageV2 {
            usage_id: command.usage_id(),
            owner: command.owner(),
            kind: command.kind().into(),
            image_id: command.image_id(),
            content_key: command.content_key(),
            ir_fingerprint: command.ir_fingerprint(),
            binding_fingerprint: Some(command.binding_fingerprint()),
            selected_placement_fingerprint: command.selected_placement_fingerprint(),
            page_index: command.page_index(),
            frame_index: command.frame_index(),
            fragment_ordinal: paint.semantic_fragment_ordinal(),
            paint_ordinal: command.paint_ordinal(),
            viewport: command.viewport(),
            scale: command.scale_raw(),
            matrix: command.matrix(),
            resolved_current_color: [color.red(), color.green(), color.blue()],
            display_command_fingerprint: command.fingerprint(),
            relation: StagingCombinedVectorUsageRelationV2::Precomposed(command.relation().clone()),
            canonical_jcs: String::new(),
            fingerprint: [0; 32],
        };
        seal_usage(&mut usage);
        usages.push(usage);
    }

    if let Some(figures) = figures {
        for command in figures.commands() {
            let image = admitted.image(command.image_id()).ok_or(
                StagingCombinedVectorDisplayErrorV2::AdmissionMismatch(command.image_id()),
            )?;
            if image.media_kind() != AdmittedImageMediaKind::SafeVector
                || image.content_hash() != command.admitted_sha256()
                || image
                    .admitted_safe_vector()
                    .map(|value| value.fingerprint())
                    != Some(command.ir_fingerprint())
            {
                return Err(StagingCombinedVectorDisplayErrorV2::AdmissionMismatch(
                    command.image_id(),
                ));
            }
            let content_key = VectorContentKey::from_admitted(image).map_err(|_| {
                StagingCombinedVectorDisplayErrorV2::AdmissionMismatch(command.image_id())
            })?;
            let node = registry.source_node(command.owner()).filter(|node| {
                node.owner() == StructureOwner::Source(command.owner())
                    && node.role() == StructureRole::Figure
                    && node.vector_binding_v2().is_none()
            });
            let node = node.ok_or(StagingCombinedVectorDisplayErrorV2::StructureMismatch(
                command.owner(),
            ))?;
            let mut matches = selected.paints().iter().filter(|paint| {
                paint.page_index() == command.page_index()
                    && paint.owner()
                        == SelectedStructurePaintOwner::Structure(node.structure_node_id())
                    && matches!(paint.binding(), SelectedStructurePaintBindingV2::Standard)
            });
            let Some(paint) = matches.next() else {
                return Err(StagingCombinedVectorDisplayErrorV2::SelectedPaintMismatch(
                    command.owner(),
                ));
            };
            if matches.next().is_some() {
                return Err(StagingCombinedVectorDisplayErrorV2::SelectedPaintMismatch(
                    command.owner(),
                ));
            }
            let usage_id = precomposed_count
                .checked_add(command.occurrence())
                .ok_or(StagingCombinedVectorDisplayErrorV2::CountOverflow)?;
            let scale = command.scale_raw();
            let viewport = command.bounds();
            let unitless_scale = Unitless16_16::from_raw(scale);
            let mut usage = StagingCombinedVectorUsageV2 {
                usage_id,
                owner: command.owner(),
                kind: StagingCombinedVectorKindV2::Figure,
                image_id: command.image_id(),
                content_key,
                ir_fingerprint: command.ir_fingerprint(),
                binding_fingerprint: None,
                selected_placement_fingerprint: command.selected_placement_fingerprint(),
                page_index: command.page_index(),
                frame_index: command.frame_index(),
                fragment_ordinal: paint.semantic_fragment_ordinal(),
                paint_ordinal: paint.paint_ordinal(),
                viewport,
                scale,
                matrix: AffineTransform {
                    a: unitless_scale,
                    b: Unitless16_16::from_raw(0),
                    c: Unitless16_16::from_raw(0),
                    d: unitless_scale,
                    e: Length::from_raw(viewport.x().raw())
                        .ok_or(StagingCombinedVectorDisplayErrorV2::ReceiptMismatch)?,
                    f: Length::from_raw(viewport.y().raw())
                        .ok_or(StagingCombinedVectorDisplayErrorV2::ReceiptMismatch)?,
                },
                resolved_current_color: [0, 0, 0],
                display_command_fingerprint: command.fingerprint(),
                relation: StagingCombinedVectorUsageRelationV2::Figure {
                    occurrence: command.occurrence(),
                    placement: command.placement(),
                },
                canonical_jcs: String::new(),
                fingerprint: [0; 32],
            };
            seal_usage(&mut usage);
            usages.push(usage);
        }
    }

    let mut receipt = StagingCombinedVectorDisplayReceiptV2 {
        package_sha256: package.canonical_jcs_sha256(),
        semantic_sha256: package.semantic_fingerprint(),
        admitted_sha256: admitted.fingerprint().bytes(),
        profile_sha256: precomposed.receipt().profile_fingerprint(),
        limits_sha256: precomposed.receipt().limits_fingerprint(),
        binding_set_sha256: precomposed.receipt().binding_set_fingerprint(),
        precomposed_display_sha256: precomposed.receipt().fingerprint(),
        figure_display_sha256: figures.map(|value| value.receipt().fingerprint()),
        structure_registry_sha256: registry.fingerprint(),
        selected_binding_sha256: selected.fingerprint(),
        page_count,
        usage_count,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs = encode_receipt(&receipt, &usages);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    let display = StagingCombinedVectorDisplayV2 { usages, receipt };
    display.verify_resource_closure()?;
    Ok(display)
}

fn validate_figure_coverage(
    package: &ValidatedStagingSemanticPackage,
    admitted: &AdmittedResourceLedger,
    figures: Option<&StagingSafeVectorDisplay>,
) -> Result<(), StagingCombinedVectorDisplayErrorV2> {
    let mut expected = BTreeSet::new();
    collect_safe_vector_figures(&package.document().blocks, admitted, &mut expected)?;
    for footnote in &package.document().footnotes {
        collect_safe_vector_figures(&footnote.blocks, admitted, &mut expected)?;
    }

    let mut observed = BTreeSet::new();
    if let Some(figures) = figures {
        for command in figures.commands() {
            if !observed.insert((command.owner(), command.image_id())) {
                return Err(StagingCombinedVectorDisplayErrorV2::FigureDisplayMismatch);
            }
        }
    }
    if observed != expected {
        return Err(StagingCombinedVectorDisplayErrorV2::FigureDisplayMismatch);
    }
    Ok(())
}

fn collect_safe_vector_figures(
    blocks: &[StagingM4Block],
    admitted: &AdmittedResourceLedger,
    output: &mut BTreeSet<(NodeId, ImageResourceId)>,
) -> Result<(), StagingCombinedVectorDisplayErrorV2> {
    for block in blocks {
        match block {
            StagingM4Block::Figure {
                common,
                image_id,
                caption,
                ..
            } => {
                let image = admitted.image(*image_id).ok_or(
                    StagingCombinedVectorDisplayErrorV2::AdmissionMismatch(*image_id),
                )?;
                if image.media_kind() == AdmittedImageMediaKind::SafeVector
                    && !output.insert((common.node_id, *image_id))
                {
                    return Err(StagingCombinedVectorDisplayErrorV2::FigureDisplayMismatch);
                }
                collect_safe_vector_figures(caption, admitted, output)?;
            }
            StagingM4Block::VectorFigure { caption, .. }
            | StagingM4Block::SemanticContainer {
                blocks: caption, ..
            } => collect_safe_vector_figures(caption, admitted, output)?,
            StagingM4Block::List { items, .. } => {
                for item in items {
                    collect_safe_vector_figures(&item.blocks, admitted, output)?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_safe_vector_figures(&cell.blocks, admitted, output)?;
                }
            }
            StagingM4Block::Paragraph { .. }
            | StagingM4Block::Heading { .. }
            | StagingM4Block::PageBreak { .. }
            | StagingM4Block::DisplayMath { .. }
            | StagingM4Block::MathVectorBlock { .. } => {}
        }
    }
    Ok(())
}

/// Figure-only fixture for the `/1` Figure to production SafeVector `/2`
/// compatibility path. It intentionally carries an empty, sealed precomposed
/// Display so downstream resource and PDF crates can exercise the common
/// vector path without fabricating a producer-composed placement.
#[cfg(any(test, feature = "staging-fixtures"))]
pub struct StagingCombinedVectorFigureFixture {
    pub figure: crate::StagingSafeVectorDisplayFixture,
    pub profile: typaxis_syntax::StagingPrecomposedVectorProfileAuthorization,
    pub bindings: typaxis_layout::ValidatedPrecomposedVectorBindings,
    pub math_flows: typaxis_layout::StagingMathVectorFlowRegistry,
    pub block_layout: typaxis_layout::StagingPrecomposedVectorBlockLayout,
    pub block_input: typaxis_pagination::StagingAtomicVectorBlockPaginationInput,
    pub block_selected: typaxis_pagination::StagingAtomicVectorBlockSelectedLayout,
    pub precomposed: StagingPrecomposedVectorDisplay,
    pub navigation: typaxis_syntax::ValidatedStagingBookNavigationV2,
    pub semantics: typaxis_syntax::ValidatedStagingStructureSemanticsV2,
    pub book_profile: typaxis_syntax::StagingBookNavigationProfileAuthorizationV2,
    pub accessibility: typaxis_syntax::StagingAccessibilityProfileAuthorizationV2,
    pub registry: StructureRegistryReceiptV2,
    pub selected: SelectedStructureBindingReceiptV2,
    pub display: StagingCombinedVectorDisplayV2,
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_combined_vector_figure_fixture(
) -> Result<StagingCombinedVectorFigureFixture, Box<dyn std::error::Error>> {
    use typaxis_core::NonNegativeLength;
    use typaxis_layout::{
        bind_staging_precomposed_vectors, prepare_staging_math_vector_flows,
        prepare_staging_precomposed_vector_blocks, select_structure_bindings_v2,
        SelectedStructurePaintInputV2,
    };
    use typaxis_pagination::{
        paginate_staging_atomic_vector_blocks, StagingAtomicVectorBlockPaginationInput,
    };
    use typaxis_syntax::{
        validate_staging_book_navigation_v2, validate_staging_structure_semantics_v2,
        StagingAccessibilityProfileAuthorizationV2, StagingAccessibilityProfileViewV2,
        StagingBookNavigationProfileAuthorizationV2, StagingBookNavigationProfileViewV2,
        StagingPrecomposedVectorProfileAuthorization,
        StagingPrecomposedVectorProfileSessionIdentity,
    };

    let figure = crate::staging_safe_vector_display_fixture()?;
    let package = &figure.layout.package;
    let limits = &figure.layout.limits;
    let profile = StagingPrecomposedVectorProfileAuthorization::bind_profile_receipt(
        sha256(b"typaxis.fixture/figure-precomposed-vector-profile"),
        package,
        limits,
        &StagingPrecomposedVectorProfileSessionIdentity::fresh(),
    )?;
    let bindings =
        bind_staging_precomposed_vectors(package, &profile, limits, &figure.layout.admitted)?;
    let math_flows = prepare_staging_math_vector_flows(
        package,
        &profile,
        limits,
        &figure.layout.admitted,
        &bindings,
    )?;
    let block_layout = prepare_staging_precomposed_vector_blocks(
        package,
        &profile,
        limits,
        &figure.layout.admitted,
        &bindings,
        &math_flows,
    )?;
    let block_input = StagingAtomicVectorBlockPaginationInput::new(
        &block_layout,
        NonNegativeLength::ZERO,
        0,
        Vec::new(),
        Vec::new(),
    )?;
    let block_selected =
        paginate_staging_atomic_vector_blocks(&block_layout, &math_flows, &block_input, limits)?;
    let navigation = validate_staging_book_navigation_v2(package, limits)?;
    let semantics = validate_staging_structure_semantics_v2(package, &navigation, limits)?;
    let book_profile = StagingBookNavigationProfileAuthorizationV2::bind_profile_receipt(
        StagingBookNavigationProfileViewV2::new(package, &navigation, limits)?,
        sha256(b"typaxis.fixture/figure-book-profile"),
        profile.profile_receipt_fingerprint(),
        profile.profile_fingerprint(),
        package,
        &navigation,
        limits,
    )?;
    let accessibility = StagingAccessibilityProfileAuthorizationV2::bind_profile_receipt(
        StagingAccessibilityProfileViewV2::new(package, &navigation, &semantics, limits)?,
        sha256(b"typaxis.fixture/figure-accessibility-profile"),
        book_profile.profile_receipt_fingerprint(),
        package,
        &navigation,
        &semantics,
        limits,
    )?;
    let registry = crate::build_structure_registry_v2(
        package,
        &navigation,
        &semantics,
        &accessibility,
        limits,
    )?;
    let page_count = u32::try_from(figure.display.pages().len())?;
    if usize::try_from(page_count) != Ok(block_selected.pages().len()) {
        return Err(Box::new(
            StagingCombinedVectorDisplayErrorV2::ReceiptMismatch,
        ));
    }
    let precomposed =
        crate::precomposed_vector::staging_empty_precomposed_vector_display_for_combined_fixture(
            package,
            &figure.layout.admitted,
            &profile,
            &bindings,
            limits,
            page_count,
            block_selected.receipt().fingerprint(),
        )?;
    let geometry = figure.display.page_geometry();
    let pages = (0..page_count)
        .map(|page_index| crate::SelectedStructurePage {
            page_index,
            width_raw: geometry.page_width().get().raw(),
            height_raw: geometry.page_height().get().raw(),
        })
        .collect::<Vec<_>>();
    let paints = figure
        .display
        .commands()
        .enumerate()
        .map(|(index, command)| {
            let node = registry.source_node(command.owner()).ok_or(
                StagingCombinedVectorDisplayErrorV2::StructureMismatch(command.owner()),
            )?;
            Ok(SelectedStructurePaintInputV2 {
                selected_paint_id: u32::try_from(index)
                    .map_err(|_| StagingCombinedVectorDisplayErrorV2::CountOverflow)?,
                page_index: command.page_index(),
                paint_ordinal: command.occurrence(),
                semantic_fragment_ordinal: 0,
                owner: SelectedStructurePaintOwner::Structure(node.structure_node_id()),
                binding: SelectedStructurePaintBindingV2::Standard,
            })
        })
        .collect::<Result<Vec<_>, StagingCombinedVectorDisplayErrorV2>>()?;
    let selected = select_structure_bindings_v2(
        &registry,
        &accessibility,
        limits,
        figure.display.receipt().selected_layout_fingerprint(),
        u64::from(figure.display.receipt().command_count()),
        &pages,
        &paints,
        &[],
    )?;
    let display = build_staging_combined_vector_display_v2(
        package,
        &precomposed,
        Some(&figure.display),
        &figure.layout.admitted,
        &registry,
        &selected,
    )?;
    Ok(StagingCombinedVectorFigureFixture {
        figure,
        profile,
        bindings,
        math_flows,
        block_layout,
        block_input,
        block_selected,
        precomposed,
        navigation,
        semantics,
        book_profile,
        accessibility,
        registry,
        selected,
        display,
    })
}

fn seal_usage(usage: &mut StagingCombinedVectorUsageV2) {
    usage.canonical_jcs = encode_usage(usage);
    usage.fingerprint = sha256(usage.canonical_jcs.as_bytes());
}

fn encode_receipt(
    receipt: &StagingCombinedVectorDisplayReceiptV2,
    usages: &[StagingCombinedVectorUsageV2],
) -> String {
    let mut output = String::from("{\"admitted_sha256\":");
    push_hash(&mut output, receipt.admitted_sha256);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_COMBINED_VECTOR_DISPLAY_ALGORITHM_V2);
    output.push_str(",\"binding_set_sha256\":");
    push_hash(&mut output, receipt.binding_set_sha256);
    output.push_str(",\"figure_display_sha256\":");
    push_optional_hash(&mut output, receipt.figure_display_sha256);
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, receipt.limits_sha256);
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, receipt.package_sha256);
    output.push_str(",\"page_count\":");
    output.push_str(&receipt.page_count.to_string());
    output.push_str(",\"precomposed_display_sha256\":");
    push_hash(&mut output, receipt.precomposed_display_sha256);
    output.push_str(",\"profile_sha256\":");
    push_hash(&mut output, receipt.profile_sha256);
    output.push_str(",\"selected_binding_sha256\":");
    push_hash(&mut output, receipt.selected_binding_sha256);
    output.push_str(",\"semantic_sha256\":");
    push_hash(&mut output, receipt.semantic_sha256);
    output.push_str(",\"structure_registry_sha256\":");
    push_hash(&mut output, receipt.structure_registry_sha256);
    output.push_str(",\"usage_count\":");
    output.push_str(&receipt.usage_count.to_string());
    output.push_str(",\"usages\":[");
    for (index, usage) in usages.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(usage.canonical_jcs());
    }
    output.push_str("]}");
    output
}

fn encode_usage(usage: &StagingCombinedVectorUsageV2) -> String {
    let mut output = String::from("{\"binding_fingerprint\":");
    push_optional_hash(&mut output, usage.binding_fingerprint);
    output.push_str(",\"content_key\":");
    push_content_key(&mut output, &usage.content_key);
    output.push_str(",\"display_command_fingerprint\":");
    push_hash(&mut output, usage.display_command_fingerprint);
    output.push_str(",\"fragment_ordinal\":");
    output.push_str(&usage.fragment_ordinal.to_string());
    output.push_str(",\"frame_index\":");
    output.push_str(&usage.frame_index.to_string());
    output.push_str(",\"image_id\":");
    output.push_str(&usage.image_id.get().to_string());
    output.push_str(",\"ir_fingerprint\":");
    push_hash(&mut output, usage.ir_fingerprint);
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, usage.kind.as_str());
    output.push_str(",\"matrix\":");
    push_matrix(&mut output, usage.matrix);
    output.push_str(",\"owner\":");
    output.push_str(&usage.owner.get().to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&usage.page_index.to_string());
    output.push_str(",\"paint_ordinal\":");
    output.push_str(&usage.paint_ordinal.to_string());
    output.push_str(",\"relation\":");
    match &usage.relation {
        StagingCombinedVectorUsageRelationV2::Figure {
            occurrence,
            placement,
        } => {
            output.push_str("{\"occurrence\":");
            output.push_str(&occurrence.to_string());
            output.push_str(",\"placement\":");
            push_jcs_string(&mut output, placement.as_str());
            output.push('}');
        }
        StagingCombinedVectorUsageRelationV2::Precomposed(value) => {
            push_precomposed_relation(&mut output, value);
        }
    }
    output.push_str(",\"resolved_current_color\":[");
    for (index, color) in usage.resolved_current_color.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&color.to_string());
    }
    output.push_str("],\"scale\":");
    output.push_str(&usage.scale.to_string());
    output.push_str(",\"selected_placement_fingerprint\":");
    push_hash(&mut output, usage.selected_placement_fingerprint);
    output.push_str(",\"usage_id\":");
    output.push_str(&usage.usage_id.to_string());
    output.push_str(",\"viewport\":{");
    output.push_str("\"height\":");
    output.push_str(&usage.viewport.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&usage.viewport.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&usage.viewport.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&usage.viewport.y().raw().to_string());
    output.push_str("}}");
    output
}

fn push_precomposed_relation(output: &mut String, value: &StagingDrawVectorV2Relation) {
    match value {
        StagingDrawVectorV2Relation::Inline { baseline_metrics } => {
            output.push_str("{\"baseline_metrics\":");
            push_baseline_metrics(output, *baseline_metrics);
            output.push_str(",\"kind\":\"inline\"}");
        }
        StagingDrawVectorV2Relation::VectorFigure { figure_caption } => {
            output.push_str("{\"caption_flow_id\":");
            output.push_str(&figure_caption.caption_flow_id().get().to_string());
            output.push_str(",\"caption_owners\":[");
            for (index, owner) in figure_caption.caption_owners().iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                output.push_str(&owner.get().to_string());
            }
            output.push_str("],\"keep_caption\":");
            output.push_str(if figure_caption.keep_caption() {
                "true"
            } else {
                "false"
            });
            output.push_str(",\"kind\":\"vector_figure\"}");
        }
        StagingDrawVectorV2Relation::MathVectorBlock {
            baseline_metrics,
            math_flow,
        } => {
            output.push_str("{\"baseline_metrics\":");
            push_baseline_metrics(output, *baseline_metrics);
            output.push_str(",\"kind\":\"math_vector_block\",\"math_flow\":{");
            output.push_str("\"flow_fingerprint\":");
            push_hash(output, math_flow.flow_fingerprint());
            output.push_str(",\"flow_id\":");
            output.push_str(&math_flow.flow_id().get().to_string());
            output.push_str(",\"parent_flow_id\":");
            output.push_str(&math_flow.parent_flow_id().get().to_string());
            output.push_str(",\"parent_position\":");
            output.push_str(&math_flow.parent_position().to_string());
            output.push_str(",\"terminal\":");
            output.push_str(&math_flow.terminal().get().to_string());
            output.push_str(",\"terminal_receipt_fingerprint\":");
            push_hash(output, math_flow.terminal_receipt_fingerprint());
            output.push_str("}}");
        }
    }
}

fn push_baseline_metrics(output: &mut String, value: crate::StagingDrawVectorBaselineMetrics) {
    output.push_str("{\"baseline\":");
    output.push_str(&value.baseline().get().raw().to_string());
    output.push_str(",\"baseline_y\":");
    output.push_str(&value.baseline_y().raw().to_string());
    output.push_str(",\"metric_receipt_fingerprint\":");
    push_hash(output, value.metric_receipt_fingerprint());
    output.push_str(",\"pen_origin_x\":");
    output.push_str(&value.pen_origin_x().raw().to_string());
    output.push('}');
}

fn push_content_key(output: &mut String, key: &VectorContentKey) {
    output.push_str("{\"ir_fingerprint\":");
    push_hash(output, key.ir_fingerprint());
    output.push_str(",\"ir_id\":");
    push_jcs_string(output, key.ir_id());
    output.push_str(",\"media_type\":");
    push_jcs_string(output, key.media_type().as_str());
    output.push_str(",\"parser_id\":");
    push_jcs_string(output, key.parser_id());
    output.push_str(",\"source_sha256\":");
    push_hash(output, key.source_sha256());
    output.push('}');
}

fn push_matrix(output: &mut String, value: AffineTransform) {
    output.push_str("{\"a_16_16\":");
    output.push_str(&value.a.raw().to_string());
    output.push_str(",\"b_16_16\":");
    output.push_str(&value.b.raw().to_string());
    output.push_str(",\"c_16_16\":");
    output.push_str(&value.c.raw().to_string());
    output.push_str(",\"d_16_16\":");
    output.push_str(&value.d.raw().to_string());
    output.push_str(",\"e\":");
    output.push_str(&value.e.raw().to_string());
    output.push_str(",\"f\":");
    output.push_str(&value.f.raw().to_string());
    output.push('}');
}

fn push_optional_hash(output: &mut String, value: Option<[u8; 32]>) {
    match value {
        Some(value) => push_hash(output, value),
        None => output.push_str("null"),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combined_vector_display_projects_existing_figure_as_safe_vector_v2() {
        let fixture = staging_combined_vector_figure_fixture().unwrap();
        let [usage] = fixture.display.usages() else {
            panic!("Figure fixture must project exactly one vector usage");
        };
        assert_eq!(usage.kind(), StagingCombinedVectorKindV2::Figure);
        assert_eq!(usage.binding_fingerprint(), None);
        assert_eq!(usage.content_key().media_type().as_str(), "svg-safe-1");
        assert_eq!(usage.relation().figure_occurrence(), Some(0));
        assert_eq!(
            fixture.display.receipt().figure_display_sha256(),
            Some(fixture.figure.display.receipt().fingerprint())
        );
        fixture.display.verify_resource_closure().unwrap();
    }

    #[test]
    fn combined_vector_display_rejects_relation_tamper() {
        let fixture = staging_combined_vector_figure_fixture().unwrap();
        let mut altered = fixture.display;
        let StagingCombinedVectorUsageRelationV2::Figure { occurrence, .. } =
            &mut altered.usages[0].relation
        else {
            panic!("Figure fixture must remain a Figure usage");
        };
        *occurrence = 1;
        assert_eq!(
            altered.verify_resource_closure(),
            Err(StagingCombinedVectorDisplayErrorV2::ReceiptMismatch)
        );
    }

    #[test]
    fn combined_vector_display_rejects_omitted_figure_stream() {
        let fixture = staging_combined_vector_figure_fixture().unwrap();
        assert_eq!(
            build_staging_combined_vector_display_v2(
                &fixture.figure.layout.package,
                &fixture.precomposed,
                None,
                &fixture.figure.layout.admitted,
                &fixture.registry,
                &fixture.selected,
            ),
            Err(StagingCombinedVectorDisplayErrorV2::FigureDisplayMismatch)
        );
    }

    #[test]
    fn combined_vector_display_rejects_foreign_limits_receipt() {
        let fixture = staging_combined_vector_figure_fixture().unwrap();
        let page_count = u32::try_from(fixture.figure.display.pages().len()).unwrap();
        let foreign = crate::precomposed_vector::staging_empty_precomposed_vector_display_with_foreign_limits_for_test(
            &fixture.figure.layout.package,
            &fixture.figure.layout.admitted,
            &fixture.profile,
            &fixture.bindings,
            &fixture.figure.layout.limits,
            page_count,
            fixture.block_selected.receipt().fingerprint(),
            [0xa5; 32],
        )
        .unwrap();
        assert_eq!(
            build_staging_combined_vector_display_v2(
                &fixture.figure.layout.package,
                &foreign,
                Some(&fixture.figure.display),
                &fixture.figure.layout.admitted,
                &fixture.registry,
                &fixture.selected,
            ),
            Err(StagingCombinedVectorDisplayErrorV2::ReceiptMismatch)
        );
    }
}
