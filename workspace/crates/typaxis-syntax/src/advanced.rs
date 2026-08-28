use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU16;
use typaxis_core::{
    Length, MasterId, NodeId, NonNegativeLength, PositiveLength, Rect, SourceId, SourceSpan,
    TextBufferId, TextSpan, Utf8ByteOffset,
};
use typaxis_document::{
    AdvancedPageMaster, AdvancedPageMasterSet, ColumnBalance, ColumnFill, ColumnLayout,
    FigurePlacement, HeadingLevel, PageProgression, PageRegion, PageRegionBlock, PageRegionInline,
    PageWritingMode,
};
use typaxis_document_package as wire;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingAdvancedSyntaxFailure {
    MasterExtensionMismatch,
    InvalidLength,
    InvalidNodeOrder,
    InvalidSourceSpan,
    InvalidTextSpan,
    InvalidClass,
    InvalidHeadingLevel,
    InvalidFigurePlacement,
    AstNodeLimit,
    AstDepthLimit,
    ArithmeticOverflow,
}

impl std::fmt::Display for StagingAdvancedSyntaxFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::MasterExtensionMismatch => "advanced page-master extension mismatch",
            Self::InvalidLength => "invalid advanced fixed-point length",
            Self::InvalidNodeOrder => "page-region NodeIds are not globally dense",
            Self::InvalidSourceSpan => "invalid page-region source span",
            Self::InvalidTextSpan => "invalid page-region text span",
            Self::InvalidClass => "invalid or non-canonical page-region class list",
            Self::InvalidHeadingLevel => "invalid page-region heading level",
            Self::InvalidFigurePlacement => "Figure placement registry mismatch",
            Self::AstNodeLimit => "advanced semantic nodes exceed max_ast_nodes",
            Self::AstDepthLimit => "page-region nesting exceeds max_ast_nesting_depth",
            Self::ArithmeticOverflow => "advanced syntax arithmetic overflow",
        })
    }
}

impl std::error::Error for StagingAdvancedSyntaxFailure {}

#[derive(Debug)]
pub enum StagingAdvancedPackageParseError {
    Base(MachineParseFailure),
    Advanced(StagingAdvancedSyntaxFailure),
}

impl std::fmt::Display for StagingAdvancedPackageParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Base(error) => error.fmt(formatter),
            Self::Advanced(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for StagingAdvancedPackageParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Base(error) => Some(error),
            Self::Advanced(error) => Some(error),
        }
    }
}

impl From<MachineParseFailure> for StagingAdvancedPackageParseError {
    fn from(value: MachineParseFailure) -> Self {
        Self::Base(value)
    }
}

impl From<StagingAdvancedSyntaxFailure> for StagingAdvancedPackageParseError {
    fn from(value: StagingAdvancedSyntaxFailure) -> Self {
        Self::Advanced(value)
    }
}

/// Syntax-issued current 1.3 extension receipt. `ValidatedMachinePackage`
/// retains it behind the ordinary public pipeline boundary, so callers cannot
/// forge or silently upgrade an older-contract package into this state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingAdvancedPackage {
    package: ValidatedParsedPackage,
    page_masters: AdvancedPageMasterSet,
    figure_placements: BTreeMap<NodeId, FigurePlacement>,
    raw_sha256: [u8; 32],
    canonical_jcs_sha256: [u8; 32],
}

pub(super) fn validate_current_advanced_extension(
    package: ValidatedParsedPackage,
    extension: wire::WireAdvancedDocumentPackageExtension,
    raw_sha256: [u8; 32],
    canonical_jcs_sha256: [u8; 32],
    limits: &ValidatedResourceLimits,
) -> Result<ValidatedStagingAdvancedPackage, StagingAdvancedSyntaxFailure> {
    let page_masters = lower_advanced_page_masters(&package, extension.page_masters, limits)?;
    let figure_placements = validate_figure_placements(&package, extension.figure_placements)?;
    Ok(ValidatedStagingAdvancedPackage {
        package,
        page_masters,
        figure_placements,
        raw_sha256,
        canonical_jcs_sha256,
    })
}

impl ValidatedStagingAdvancedPackage {
    pub const fn package(&self) -> &ValidatedParsedPackage {
        &self.package
    }

    pub const fn page_masters(&self) -> &AdvancedPageMasterSet {
        &self.page_masters
    }

    pub fn figure_placement(&self, node_id: NodeId) -> Option<FigurePlacement> {
        self.figure_placements.get(&node_id).copied()
    }

    pub fn figure_placements(
        &self,
    ) -> impl ExactSizeIterator<Item = (NodeId, FigurePlacement)> + '_ {
        self.figure_placements
            .iter()
            .map(|(node_id, placement)| (*node_id, *placement))
    }

    pub const fn raw_sha256(&self) -> [u8; 32] {
        self.raw_sha256
    }

    pub const fn canonical_jcs_sha256(&self) -> [u8; 32] {
        self.canonical_jcs_sha256
    }

    pub fn is_neutral_extension(&self) -> bool {
        self.page_masters.masters.iter().all(|extension| {
            self.package
                .package()
                .page_masters
                .masters
                .iter()
                .find(|master| master.master_id == extension.master_id)
                .is_some_and(|master| {
                    extension.trim.x().raw() == 0
                        && extension.trim.y().raw() == 0
                        && extension.trim.width().get() == master.width.get()
                        && extension.trim.height().get() == master.height.get()
                        && extension.header_content.is_none()
                        && extension.footer_content.is_none()
                        && extension.column_layout.is_none()
                })
        }) && self
            .figure_placements
            .values()
            .all(|placement| *placement == FigurePlacement::Block)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StagingAdvancedPackageParser;

impl StagingAdvancedPackageParser {
    pub const fn new() -> Self {
        Self
    }

    pub fn parse(
        &self,
        decoded: wire::DecodedStagingAdvancedDocumentPackage,
        source_utf8: String,
        policy: &PackageValidationPolicy<'_>,
    ) -> Result<ValidatedStagingAdvancedPackage, StagingAdvancedPackageParseError> {
        let (wire, advanced, placements, raw_sha256, canonical_jcs_sha256, locations) =
            decoded.into_parts();
        preflight_wire_semantics(&wire, policy.limits, &locations)?;
        let wire::WireDocumentPackage {
            contract: _,
            coordinate_unit: _,
            advanced: _,
            sources,
            text_buffers,
            document,
            style_sheet,
            page_masters,
            resources,
        } = wire;
        let source_catalog = lower_staging_source(sources, source_utf8, &locations)?;
        let text_store = lower_text_store(text_buffers, policy.limits, &locations)?;
        let package = ParsedPackage {
            sources: source_catalog,
            text_store,
            document: lower_document(document, policy, &locations)?,
            style_sheet: lower_style_sheet(style_sheet, &locations)?,
            page_masters: lower_page_masters(page_masters, &locations)?,
            resources: lower_resources(resources, &locations)?,
        };
        let include_graph = ValidatedIncludeGraph::entry_only(&package.sources, policy.limits)
            .map_err(|_| {
                MachineParseFailure::package(
                    MachineParseFailureKind::PackageValidation(
                        PackageValidationError::IncludeGraphMismatch,
                    ),
                    locations.root_member(wire::DocumentPackageRootMember::Sources),
                )
            })?;
        let package = ValidatedParsedPackage::new_resolved_with_style_contract(
            package,
            policy,
            &include_graph,
            true,
            |package, error| machine_validation_failure(package, error, &locations),
        )?;
        let page_masters = lower_advanced_page_masters(&package, advanced, policy.limits)?;
        let figure_placements = validate_figure_placements(&package, placements)?;
        Ok(ValidatedStagingAdvancedPackage {
            package,
            page_masters,
            figure_placements,
            raw_sha256,
            canonical_jcs_sha256,
        })
    }
}

fn lower_advanced_page_masters(
    package: &ValidatedParsedPackage,
    advanced: wire::WireAdvancedPageMasterSet,
    limits: &ValidatedResourceLimits,
) -> Result<AdvancedPageMasterSet, StagingAdvancedSyntaxFailure> {
    if advanced.masters.len() != package.package().page_masters.masters.len() {
        return Err(StagingAdvancedSyntaxFailure::MasterExtensionMismatch);
    }
    let region_depth = advanced
        .masters
        .iter()
        .flat_map(|master| {
            [
                master.header_content.as_ref(),
                master.footer_content.as_ref(),
            ]
        })
        .flatten()
        .map(|region| {
            if region.blocks.iter().any(|block| match block {
                wire::WirePageRegionBlock::Paragraph { children, .. }
                | wire::WirePageRegionBlock::Heading { children, .. } => !children.is_empty(),
            }) {
                3
            } else if region.blocks.is_empty() {
                1
            } else {
                2
            }
        })
        .max()
        .unwrap_or(0);
    if region_depth > limits.get().max_ast_nesting_depth {
        return Err(StagingAdvancedSyntaxFailure::AstDepthLimit);
    }
    let mut next_node = u32::try_from(package.document_nodes().node_count())
        .map_err(|_| StagingAdvancedSyntaxFailure::ArithmeticOverflow)?;
    let mut masters = Vec::new();
    masters
        .try_reserve_exact(advanced.masters.len())
        .map_err(|_| StagingAdvancedSyntaxFailure::ArithmeticOverflow)?;
    for (base, extension) in package
        .package()
        .page_masters
        .masters
        .iter()
        .zip(advanced.masters)
    {
        if base.master_id.as_str() != extension.master_id {
            return Err(StagingAdvancedSyntaxFailure::MasterExtensionMismatch);
        }
        let master_id = MasterId::new(extension.master_id)
            .map_err(|_| StagingAdvancedSyntaxFailure::MasterExtensionMismatch)?;
        let trim = lower_rect(extension.trim)?;
        let header_content = extension
            .header_content
            .map(|region| lower_region(package, region, &mut next_node))
            .transpose()?;
        let footer_content = extension
            .footer_content
            .map(|region| lower_region(package, region, &mut next_node))
            .transpose()?;
        let column_layout = extension
            .column_layout
            .map(lower_column_layout)
            .transpose()?;
        masters.push(AdvancedPageMaster {
            master_id,
            trim,
            header_content,
            footer_content,
            column_layout,
        });
    }
    let total_nodes = u64::from(next_node);
    if total_nodes > limits.get().max_ast_nodes {
        return Err(StagingAdvancedSyntaxFailure::AstNodeLimit);
    }
    Ok(AdvancedPageMasterSet {
        page_progression: match advanced.page_progression {
            wire::WirePageProgression::LeftToRight => PageProgression::LeftToRight,
        },
        writing_mode: match advanced.writing_mode {
            wire::WirePageWritingMode::HorizontalTopToBottom => {
                PageWritingMode::HorizontalTopToBottom
            }
        },
        masters,
    })
}

fn lower_rect(rect: wire::WireRect) -> Result<Rect, StagingAdvancedSyntaxFailure> {
    let x = Length::from_raw(rect.x).ok_or(StagingAdvancedSyntaxFailure::InvalidLength)?;
    let y = Length::from_raw(rect.y).ok_or(StagingAdvancedSyntaxFailure::InvalidLength)?;
    let width = Length::from_raw(rect.width)
        .and_then(PositiveLength::new)
        .ok_or(StagingAdvancedSyntaxFailure::InvalidLength)?;
    let height = Length::from_raw(rect.height)
        .and_then(PositiveLength::new)
        .ok_or(StagingAdvancedSyntaxFailure::InvalidLength)?;
    Ok(Rect::new(x, y, width, height))
}

fn lower_column_layout(
    layout: wire::WireColumnLayout,
) -> Result<ColumnLayout, StagingAdvancedSyntaxFailure> {
    let count = NonZeroU16::new(layout.count)
        .filter(|count| count.get() >= 2)
        .ok_or(StagingAdvancedSyntaxFailure::InvalidLength)?;
    let gap = Length::from_raw(layout.gap)
        .and_then(NonNegativeLength::new)
        .ok_or(StagingAdvancedSyntaxFailure::InvalidLength)?;
    Ok(ColumnLayout {
        count,
        gap,
        fill: match layout.fill {
            wire::WireColumnFill::Sequential => ColumnFill::Sequential,
        },
        balance: match layout.balance {
            wire::WireColumnBalance::None => ColumnBalance::None,
            wire::WireColumnBalance::LastPage => ColumnBalance::LastPage,
        },
    })
}

fn lower_region(
    package: &ValidatedParsedPackage,
    region: wire::WirePageRegion,
    next_node: &mut u32,
) -> Result<PageRegion, StagingAdvancedSyntaxFailure> {
    require_node(region.node_id, next_node)?;
    let span = lower_source_span(package, region.span)?;
    let mut blocks = Vec::new();
    blocks
        .try_reserve_exact(region.blocks.len())
        .map_err(|_| StagingAdvancedSyntaxFailure::ArithmeticOverflow)?;
    for block in region.blocks {
        blocks.push(lower_region_block(package, block, next_node)?);
    }
    Ok(PageRegion {
        node_id: NodeId::new(region.node_id),
        span,
        blocks,
    })
}

fn lower_region_block(
    package: &ValidatedParsedPackage,
    block: wire::WirePageRegionBlock,
    next_node: &mut u32,
) -> Result<PageRegionBlock, StagingAdvancedSyntaxFailure> {
    match block {
        wire::WirePageRegionBlock::Paragraph {
            node_id,
            span,
            classes,
            children,
        } => {
            require_node(node_id, next_node)?;
            validate_classes(&classes)?;
            Ok(PageRegionBlock::Paragraph {
                node_id: NodeId::new(node_id),
                span: lower_source_span(package, span)?,
                classes,
                children: lower_region_inlines(package, children, next_node)?,
            })
        }
        wire::WirePageRegionBlock::Heading {
            node_id,
            span,
            classes,
            level,
            children,
        } => {
            require_node(node_id, next_node)?;
            validate_classes(&classes)?;
            Ok(PageRegionBlock::Heading {
                node_id: NodeId::new(node_id),
                span: lower_source_span(package, span)?,
                classes,
                level: HeadingLevel::new(level)
                    .ok_or(StagingAdvancedSyntaxFailure::InvalidHeadingLevel)?,
                children: lower_region_inlines(package, children, next_node)?,
            })
        }
    }
}

fn lower_region_inlines(
    package: &ValidatedParsedPackage,
    inlines: Vec<wire::WirePageRegionInline>,
    next_node: &mut u32,
) -> Result<Vec<PageRegionInline>, StagingAdvancedSyntaxFailure> {
    let mut lowered = Vec::new();
    lowered
        .try_reserve_exact(inlines.len())
        .map_err(|_| StagingAdvancedSyntaxFailure::ArithmeticOverflow)?;
    for inline in inlines {
        let value = match inline {
            wire::WirePageRegionInline::Text {
                node_id,
                span,
                text_span,
            } => {
                require_node(node_id, next_node)?;
                PageRegionInline::Text {
                    node_id: NodeId::new(node_id),
                    span: lower_source_span(package, span)?,
                    text_span: lower_text_span(package, text_span)?,
                }
            }
            wire::WirePageRegionInline::SoftBreak { node_id, span } => {
                require_node(node_id, next_node)?;
                PageRegionInline::SoftBreak {
                    node_id: NodeId::new(node_id),
                    span: lower_source_span(package, span)?,
                }
            }
            wire::WirePageRegionInline::HardBreak { node_id, span } => {
                require_node(node_id, next_node)?;
                PageRegionInline::HardBreak {
                    node_id: NodeId::new(node_id),
                    span: lower_source_span(package, span)?,
                }
            }
        };
        lowered.push(value);
    }
    Ok(lowered)
}

fn require_node(observed: u32, next_node: &mut u32) -> Result<(), StagingAdvancedSyntaxFailure> {
    if observed != *next_node {
        return Err(StagingAdvancedSyntaxFailure::InvalidNodeOrder);
    }
    *next_node = next_node
        .checked_add(1)
        .ok_or(StagingAdvancedSyntaxFailure::ArithmeticOverflow)?;
    Ok(())
}

fn lower_source_span(
    package: &ValidatedParsedPackage,
    span: wire::WireSourceSpan,
) -> Result<SourceSpan, StagingAdvancedSyntaxFailure> {
    let source_id = SourceId::new(span.source_id);
    let source = package
        .package()
        .sources
        .get(source_id)
        .ok_or(StagingAdvancedSyntaxFailure::InvalidSourceSpan)?;
    let start = Utf8ByteOffset::new(span.start_byte);
    let end = Utf8ByteOffset::new(span.end_byte);
    if span.end_byte > source.utf8_byte_length()
        || !source.utf8().is_char_boundary(span.start_byte as usize)
        || !source.utf8().is_char_boundary(span.end_byte as usize)
    {
        return Err(StagingAdvancedSyntaxFailure::InvalidSourceSpan);
    }
    SourceSpan::new(source_id, start, end).ok_or(StagingAdvancedSyntaxFailure::InvalidSourceSpan)
}

fn lower_text_span(
    package: &ValidatedParsedPackage,
    span: wire::WireTextSpan,
) -> Result<TextSpan, StagingAdvancedSyntaxFailure> {
    let text_id = TextBufferId::new(span.text_id);
    let buffer = package
        .package()
        .text_store
        .get(text_id)
        .ok_or(StagingAdvancedSyntaxFailure::InvalidTextSpan)?;
    let start = Utf8ByteOffset::new(span.start_byte);
    let end = Utf8ByteOffset::new(span.end_byte);
    if !buffer.is_boundary(start) || !buffer.is_boundary(end) {
        return Err(StagingAdvancedSyntaxFailure::InvalidTextSpan);
    }
    TextSpan::new(text_id, start, end).ok_or(StagingAdvancedSyntaxFailure::InvalidTextSpan)
}

fn validate_classes(classes: &[String]) -> Result<(), StagingAdvancedSyntaxFailure> {
    let mut seen = BTreeSet::new();
    let mut previous: Option<&str> = None;
    for class in classes {
        let mut characters = class.chars();
        let valid_start = characters
            .next()
            .is_some_and(|value| value == '_' || value.is_ascii_alphabetic());
        if !valid_start
            || !characters
                .all(|value| value == '_' || value == '-' || value.is_ascii_alphanumeric())
            || !seen.insert(class)
            || previous.is_some_and(|value| value > class)
        {
            return Err(StagingAdvancedSyntaxFailure::InvalidClass);
        }
        previous = Some(class);
    }
    Ok(())
}

fn validate_figure_placements(
    package: &ValidatedParsedPackage,
    records: Vec<wire::WireFigurePlacementRecord>,
) -> Result<BTreeMap<NodeId, FigurePlacement>, StagingAdvancedSyntaxFailure> {
    let figure_nodes: BTreeSet<_> = package
        .document_nodes()
        .nodes()
        .filter_map(|(node_id, kind)| {
            (kind == typaxis_document::DocumentNodeKind::Figure).then_some(node_id)
        })
        .collect();
    let mut placements = BTreeMap::new();
    for record in records {
        let node_id = NodeId::new(record.node_id);
        if !figure_nodes.contains(&node_id)
            || placements
                .insert(
                    node_id,
                    match record.placement {
                        wire::WireFigurePlacement::Block => FigurePlacement::Block,
                        wire::WireFigurePlacement::Float => FigurePlacement::Float,
                    },
                )
                .is_some()
        {
            return Err(StagingAdvancedSyntaxFailure::InvalidFigurePlacement);
        }
    }
    if placements.len() != figure_nodes.len() {
        return Err(StagingAdvancedSyntaxFailure::InvalidFigurePlacement);
    }
    Ok(placements)
}
