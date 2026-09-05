#![forbid(unsafe_code)]

mod advanced;
mod book_navigation;
mod semantic_container;
mod tagged_structure;

pub use advanced::{
    StagingAdvancedPackageParseError, StagingAdvancedPackageParser, StagingAdvancedSyntaxFailure,
    ValidatedStagingAdvancedPackage,
};
pub use book_navigation::{
    canonicalize_bcp47_language, validate_staging_book_navigation,
    validate_staging_book_navigation_v2, BookNavigationSyntaxError, BookNavigationSyntaxErrorKind,
    ComputedLanguageRegistryReceipt, ComputedLanguageRegistryReceiptV2, DocumentMetadataReceipt,
    StagingBookNavigationProfileAuthorization, StagingBookNavigationProfileAuthorizationV2,
    StagingBookNavigationProfileView, StagingBookNavigationProfileViewV2,
    ValidatedOutlineRegistryReceipt, ValidatedStagingBookNavigation,
    ValidatedStagingBookNavigationV2, BCP47_LANGUAGE_ALGORITHM,
    BOOK_NAVIGATION_PROFILE_VIEW_ALGORITHM, BOOK_NAVIGATION_PROFILE_VIEW_ALGORITHM_V2,
    COMPUTED_LANGUAGE_REGISTRY_ALGORITHM, COMPUTED_LANGUAGE_REGISTRY_ALGORITHM_V2,
    DOCUMENT_METADATA_ALGORITHM, OUTLINE_REGISTRY_ALGORITHM,
};
pub use semantic_container::{
    PrecomposedVectorActualTextResolution, PrecomposedVectorField, PrecomposedVectorKind,
    PrecomposedVectorMetricPayload, ProductionMachineParseOutcome, StagingCffProfileView,
    StagingJpegFigureProfileUse, StagingJpegProfileView, StagingM4PageGeometry,
    StagingMathLayoutBudgetGuard, StagingMathProfileAuthorization, StagingMathProfileProgressToken,
    StagingMathProfileSessionIdentity, StagingMathProfileView,
    StagingPrecomposedVectorProfileAuthorization, StagingPrecomposedVectorProfileProgressToken,
    StagingPrecomposedVectorProfileSessionIdentity, StagingSafeVectorProfileView,
    StagingSemanticContainerProfileView, StagingSemanticPackageParser, StagingSemanticSyntaxError,
    UnresolvedPrecomposedVectorResourceBinding, ValidatedPrecomposedVectorAlternative,
    ValidatedPrecomposedVectorEffectiveLanguage, ValidatedPrecomposedVectorEquationNumber,
    ValidatedPrecomposedVectorLanguageOverride, ValidatedPrecomposedVectorMetrics,
    ValidatedPrecomposedVectorTextBinding, ValidatedProductionMachinePackage,
    ValidatedStagingMathNode, ValidatedStagingSemanticPackage,
    PRECOMPOSED_VECTOR_EFFECTIVE_LANGUAGE_ALGORITHM, PRECOMPOSED_VECTOR_METRICS_ALGORITHM,
};
pub use tagged_structure::{
    validate_staging_structure_semantics, validate_staging_structure_semantics_v2,
    StagingAccessibilityProfileAuthorization, StagingAccessibilityProfileAuthorizationV2,
    StagingAccessibilityProfileView, StagingAccessibilityProfileViewV2,
    StagingStructureEquationNumberV2, StagingStructureLanguageBindingV2,
    StagingStructureSemanticError, StagingStructureSemanticKind, StagingStructureSemanticRecord,
    StagingStructureTableSection, ValidatedStagingStructureSemantics,
    ValidatedStagingStructureSemanticsV2, STAGING_ACCESSIBILITY_AUTHORIZATION_ALGORITHM,
    STAGING_ACCESSIBILITY_AUTHORIZATION_ALGORITHM_V2, STAGING_ACCESSIBILITY_PROFILE_VIEW_ALGORITHM,
    STAGING_STRUCTURE_SEMANTIC_INPUT_ALGORITHM,
};

use core::num::{NonZeroU16, NonZeroU64};
use std::collections::{BTreeMap, BTreeSet};
use typaxis_core::{
    document_fingerprint_from_jcs, push_generated_buffer_key_jcs, push_jcs_string, sha256,
    style_fingerprint_from_jcs, AnchorId, DocumentFingerprint, FontFaceId, FootnoteId,
    GeneratedBufferKey, GenerationKind, ImageResourceId, JsonPointer, Length,
    M4EffectiveResourceLimits, MasterId, NodeId, NonNegativeLength, PageName, PortablePath,
    PositiveLength, Rect, ReferenceFingerprint, SafeUri, SafeUriError, SourceId, SourceSpan,
    StyleFingerprint, StyleId, TextBufferId, TextSpan, Utf8ByteOffset, Utf8ByteRange,
    ValidatedResourceLimits, COORDINATE_UNIT, DEFAULT_ALLOWED_URI_SCHEMES,
};
use typaxis_diagnostics::{
    AdvisoryDiagnostic, Diagnostic, DiagnosticBuilder, DiagnosticCode, DiagnosticFlow,
    DiagnosticLocation, DiagnosticSubject, MasterErrorSubject, ParseFailure, PhaseDiagnostics,
    PublicMachineError, ResourceErrorSubject, Severity, SourceDiagnosticLocation,
    StyleErrorSubject, StylePropertyName,
};
use typaxis_document::{
    Block, ColumnSizing, Document, DocumentNodeKind, FontFaceDeclaration, FootnoteDefinition,
    GeneratedSiteTarget, HeadingLevel, ImageDeclaration, Inline, LinkTarget, ListItem,
    ReferenceFormat, ResourceCatalog, TableCell, TableColumn, TableRow, ValidatedDocumentNodeIndex,
};
use typaxis_document_package::{
    self as wire, CanonicalDocumentPackageJcsSha256, DocumentPackageRootMember, JsonLocationIndex,
    RawDocumentPackageSha256, WireDocumentPackage,
};
use typaxis_machine_input::{
    AdmittedMachinePackage, AdmittedMachineSource, AdmittedSemanticMachinePackage,
    MachineInputAdmissionProvenance, MachineInputFingerprint, MachineInputProgress,
    MachineInputSessionIdentity, MachineInputStage,
};
use typaxis_style::{
    is_style_identifier, BasicStyleBlockKind, BasicStyleProperty, ComputedMachineBlockStyle,
    ComputedMachineListStyle, ComputedStyle, Declaration, PageMaster, PageMasterRule,
    PageMasterSet, PageMasterValidationError, PageParity, StyleRule, StyleSheet,
    StyleValidationError, StyleValue, BASIC_BLOCK_STYLE_REGISTRY_VERSION,
    TABLE_BLOCK_STYLE_REGISTRY_VERSION,
};
use typaxis_text::{
    GeneratedBufferDraft, GeneratedProvenance, GeneratedTextStore, SourceCatalog, SourceRecord,
    TextBuffer, TextMapKind, TextMapSegment, TextStore,
};

/// Narrow dependency-inversion facade used by `typaxis-machine-profile`.
///
/// The profile crate is intentionally limited to `core + syntax +
/// diagnostics`. These are already-public domain/admission types needed to
/// inspect a sealed [`ValidatedMachinePackage`] and compose target facts; this
/// module issues no trusted value and exposes no DTO-to-trusted promotion path.
#[doc(hidden)]
pub mod machine_profile_boundary {
    pub use typaxis_document::{
        Block, ColumnBalance, ColumnFill, ColumnLayout, FigurePlacement, FloatPlacementClass,
        FontMediaDeclaration, FontMediaType, FootnoteDefinition, ImageMediaDeclaration,
        ImageMediaType, Inline, PageRegionBlock, PageRegionInline, ReferenceFormat,
        SemanticContainerKind, StagingComputedLanguageOwnerKindV2, StagingLanguageNodeKind,
        StagingM4Block, StagingM4InlineVectorKind, StagingM4ResourceCatalog,
        StagingOutlineSourceKind,
    };
    pub use typaxis_document_package as wire;
    pub use typaxis_machine_input::{
        AtomicFilePublicationCapabilityToken, HostMachineInputSession,
        HostResourceCapabilityToken as ResourceAdmissionCapabilityToken,
        MachineInputCapabilityToken, MachineInputHostOptions, MachineInputSessionIdentity,
        MAX_HOST_READ_CANDIDATES, MAX_RESOURCE_ROOTS,
    };
    pub use typaxis_style::{
        require_precomposed_vector_style_registry, BasicBlockStylePropertyDescriptor,
        BasicStyleBlockKind, BasicStyleProperty, MachineFigureWidth, MachineTextAlign, PageMaster,
        PageMasterRule, PageParity, PrecomposedVectorComputedStyleReceipt,
        PrecomposedVectorStyleConsumer, PrecomposedVectorStyleKind, PrecomposedVectorStyleProperty,
        PrecomposedVectorStylePropertyDescriptor, SemanticContainerStyleKind, StyleRule,
        StyleValue, BASIC_BLOCK_STYLE_PROPERTIES, BASIC_BLOCK_STYLE_REGISTRY_VERSION,
        PRECOMPOSED_VECTOR_STYLE_PROPERTIES, PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION,
    };

    pub use crate::{
        MachineBlockComputedStyleReceipt, MachineListComputedStyleReceipt,
        StagingListMarkerPreflightError, StagingStyleReceiptMismatch,
        ValidatedStagingListMarkerUsageReceipt, ValidatedStagingSemanticPackage,
        ValidatedStagingStylePackage,
    };
}

/// Narrow DTO facade used by `typaxis-layout-contract` without reversing the
/// workspace's document-to-layout dependency direction.
#[doc(hidden)]
pub mod layout_contract_boundary {
    pub use typaxis_document::{
        PrecomposedVectorMetrics, PrecomposedVectorSpacing, PrecomposedVectorViewport,
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    pub source_id: SourceId,
    pub uri: PortablePath,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedPackage {
    pub sources: SourceCatalog,
    pub text_store: TextStore,
    pub document: Document,
    pub style_sheet: StyleSheet,
    pub page_masters: PageMasterSet,
    pub resources: ResourceCatalog,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentPackageConversionError {
    UnknownStyleDeclarationName(String),
}

impl std::fmt::Display for DocumentPackageConversionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownStyleDeclarationName(name) => write!(
                formatter,
                "style declaration `{name}` has no current DocumentPackage wire representation"
            ),
        }
    }
}

impl std::error::Error for DocumentPackageConversionError {}

fn parsed_package_to_wire(
    package: &ParsedPackage,
) -> Result<WireDocumentPackage, DocumentPackageConversionError> {
    let document = wire_document(&package.document);
    let page_masters = wire_page_masters(&package.page_masters);
    let advanced = neutral_wire_advanced_extension(&document, &page_masters);
    Ok(WireDocumentPackage {
        contract: typaxis_core::DocumentPackageContractId::V1_3,
        coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
        advanced: Some(advanced),
        sources: package
            .sources
            .records()
            .iter()
            .map(|source| wire::WireSource {
                source_id: source.source_id().get(),
                uri: source.uri().as_str().to_owned(),
                utf8_byte_length: source.utf8_byte_length(),
                sha256: source.content_hash(),
            })
            .collect(),
        text_buffers: package
            .text_store
            .buffers()
            .iter()
            .map(|buffer| wire::WireTextBuffer {
                text_id: buffer.text_id().get(),
                utf8: buffer.text().to_owned(),
                mappings: buffer
                    .mappings()
                    .iter()
                    .map(|mapping| wire::WireTextMapSegment {
                        text_range: wire_byte_range(mapping.text_range),
                        kind: match mapping.kind {
                            TextMapKind::Identity => wire::WireTextMapKind::Identity,
                            TextMapKind::Replacement => wire::WireTextMapKind::Replacement,
                            TextMapKind::Inserted => wire::WireTextMapKind::Inserted,
                        },
                        source_span: mapping.source_span.map(wire_source_span),
                    })
                    .collect(),
            })
            .collect(),
        document,
        style_sheet: wire_style_sheet(&package.style_sheet)?,
        page_masters,
        resources: wire_resources(&package.resources),
    })
}

fn neutral_wire_advanced_extension(
    document: &wire::WireDocument,
    page_masters: &wire::WirePageMasterSet,
) -> wire::WireAdvancedDocumentPackageExtension {
    let mut figure_placements = Vec::new();
    for block in &document.blocks {
        collect_neutral_figure_placements(block, &mut figure_placements);
    }
    for footnote in &document.footnotes {
        for block in &footnote.blocks {
            collect_neutral_figure_placements(block, &mut figure_placements);
        }
    }
    figure_placements.sort_by_key(|record| record.node_id);
    wire::WireAdvancedDocumentPackageExtension {
        page_masters: wire::WireAdvancedPageMasterSet {
            page_progression: wire::WirePageProgression::LeftToRight,
            writing_mode: wire::WirePageWritingMode::HorizontalTopToBottom,
            masters: page_masters
                .masters
                .iter()
                .map(|master| wire::WireAdvancedPageMaster {
                    master_id: master.master_id.clone(),
                    trim: wire::WireRect {
                        x: 0,
                        y: 0,
                        width: master.width,
                        height: master.height,
                    },
                    header_content: None,
                    footer_content: None,
                    column_layout: None,
                })
                .collect(),
        },
        figure_placements,
    }
}

fn collect_neutral_figure_placements(
    block: &wire::WireBlock,
    output: &mut Vec<wire::WireFigurePlacementRecord>,
) {
    match block {
        wire::WireBlock::Figure {
            node_id, caption, ..
        } => {
            output.push(wire::WireFigurePlacementRecord {
                node_id: *node_id,
                placement: wire::WireFigurePlacement::Block,
            });
            for block in caption {
                collect_neutral_figure_placements(block, output);
            }
        }
        wire::WireBlock::List { items, .. } => {
            for block in items.iter().flat_map(|item| &item.blocks) {
                collect_neutral_figure_placements(block, output);
            }
        }
        wire::WireBlock::Table { head, body, .. } => {
            for block in head
                .iter()
                .chain(body)
                .flat_map(|row| &row.cells)
                .flat_map(|cell| &cell.blocks)
            {
                collect_neutral_figure_placements(block, output);
            }
        }
        wire::WireBlock::Paragraph { .. }
        | wire::WireBlock::Heading { .. }
        | wire::WireBlock::PageBreak { .. } => {}
    }
}

fn wire_document(document: &Document) -> wire::WireDocument {
    wire::WireDocument {
        node_id: document.node_id.get(),
        blocks: document.blocks.iter().map(wire_block).collect(),
        footnotes: document
            .footnotes
            .iter()
            .map(|footnote| wire::WireFootnote {
                footnote_id: footnote.footnote_id.as_str().to_owned(),
                node_id: footnote.node_id.get(),
                span: wire_source_span(footnote.span),
                blocks: footnote.blocks.iter().map(wire_block).collect(),
            })
            .collect(),
    }
}

fn wire_block(block: &Block) -> wire::WireBlock {
    match block {
        Block::Paragraph {
            node_id,
            span,
            classes,
            children,
        } => wire::WireBlock::Paragraph {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            classes: classes.clone(),
            children: children.iter().map(wire_inline).collect(),
        },
        Block::Heading {
            node_id,
            span,
            classes,
            level,
            anchor_id,
            children,
        } => wire::WireBlock::Heading {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            classes: classes.clone(),
            level: level.get(),
            anchor_id: anchor_id.as_ref().map(|value| value.as_str().to_owned()),
            children: children.iter().map(wire_inline).collect(),
        },
        Block::List {
            node_id,
            span,
            classes,
            ordered,
            start,
            items,
        } => wire::WireBlock::List {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            classes: classes.clone(),
            ordered: *ordered,
            start: *start,
            items: items
                .iter()
                .map(|item| wire::WireListItem {
                    node_id: item.node_id.get(),
                    span: wire_source_span(item.span),
                    blocks: item.blocks.iter().map(wire_block).collect(),
                })
                .collect(),
        },
        Block::Table {
            node_id,
            span,
            classes,
            columns,
            head,
            body,
        } => wire::WireBlock::Table {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            classes: classes.clone(),
            columns: columns
                .iter()
                .map(|column| match column.sizing {
                    ColumnSizing::Fixed(width) => wire::WireTableColumn::Fixed {
                        width: width.get().raw(),
                    },
                    ColumnSizing::Fraction(weight) => wire::WireTableColumn::Fraction {
                        weight: weight.get(),
                    },
                })
                .collect(),
            head: head.iter().map(wire_table_row).collect(),
            body: body.iter().map(wire_table_row).collect(),
        },
        Block::Figure {
            node_id,
            span,
            classes,
            image_id,
            alt,
            caption,
        } => wire::WireBlock::Figure {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            classes: classes.clone(),
            image_id: image_id.get(),
            alt: alt.clone(),
            caption: caption.iter().map(wire_block).collect(),
        },
        Block::PageBreak {
            node_id,
            span,
            classes,
        } => wire::WireBlock::PageBreak {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            classes: classes.clone(),
        },
    }
}

fn wire_table_row(row: &TableRow) -> wire::WireTableRow {
    wire::WireTableRow {
        node_id: row.node_id.get(),
        span: wire_source_span(row.span),
        cells: row
            .cells
            .iter()
            .map(|cell| wire::WireTableCell {
                node_id: cell.node_id.get(),
                span: wire_source_span(cell.span),
                colspan: cell.colspan.get(),
                rowspan: cell.rowspan.get(),
                blocks: cell.blocks.iter().map(wire_block).collect(),
            })
            .collect(),
    }
}

fn wire_inline(inline: &Inline) -> wire::WireInline {
    match inline {
        Inline::Text {
            node_id,
            span,
            text_span,
        } => wire::WireInline::Text {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            text_span: wire_text_span(*text_span),
        },
        Inline::Emphasis {
            node_id,
            span,
            children,
        } => wire::WireInline::Emphasis {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            children: children.iter().map(wire_inline).collect(),
        },
        Inline::Strong {
            node_id,
            span,
            children,
        } => wire::WireInline::Strong {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            children: children.iter().map(wire_inline).collect(),
        },
        Inline::Link {
            node_id,
            span,
            target,
            children,
        } => wire::WireInline::Link {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            target: match target {
                LinkTarget::Internal(anchor_id) => wire::WireLinkTarget::Internal {
                    anchor_id: anchor_id.as_str().to_owned(),
                },
                LinkTarget::Uri(uri) => wire::WireLinkTarget::Uri {
                    uri: uri.as_str().to_owned(),
                },
            },
            children: children.iter().map(wire_inline).collect(),
        },
        Inline::Anchor {
            node_id,
            span,
            anchor_id,
        } => wire::WireInline::Anchor {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            anchor_id: anchor_id.as_str().to_owned(),
        },
        Inline::Reference {
            node_id,
            span,
            target,
            format,
        } => wire::WireInline::Reference {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            target: target.as_str().to_owned(),
            format: match format {
                ReferenceFormat::Text => wire::WireReferenceFormat::Text,
                ReferenceFormat::Page => wire::WireReferenceFormat::Page,
                ReferenceFormat::Number => wire::WireReferenceFormat::Number,
            },
        },
        Inline::FootnoteReference {
            node_id,
            span,
            footnote_id,
        } => wire::WireInline::FootnoteReference {
            node_id: node_id.get(),
            span: wire_source_span(*span),
            footnote_id: footnote_id.as_str().to_owned(),
        },
        Inline::SoftBreak { node_id, span } => wire::WireInline::SoftBreak {
            node_id: node_id.get(),
            span: wire_source_span(*span),
        },
        Inline::HardBreak { node_id, span } => wire::WireInline::HardBreak {
            node_id: node_id.get(),
            span: wire_source_span(*span),
        },
    }
}

fn wire_style_sheet(
    style_sheet: &StyleSheet,
) -> Result<wire::WireStyleSheet, DocumentPackageConversionError> {
    let rules = style_sheet
        .rules
        .iter()
        .map(|rule| {
            Ok(wire::WireStyleRule {
                style_id: rule.style_id.as_str().to_owned(),
                extends: rule.extends.as_ref().map(|value| value.as_str().to_owned()),
                selector: rule.selector.clone(),
                source_order: rule.source_order,
                declarations: rule
                    .declarations
                    .iter()
                    .map(wire_declaration)
                    .collect::<Result<_, _>>()?,
            })
        })
        .collect::<Result<_, DocumentPackageConversionError>>()?;
    Ok(wire::WireStyleSheet { rules })
}

fn wire_declaration(
    declaration: &Declaration,
) -> Result<wire::WireDeclaration, DocumentPackageConversionError> {
    let name = match declaration.name.as_str() {
        "font_family" => wire::WireDeclarationName::FontFamily,
        "font_size" => wire::WireDeclarationName::FontSize,
        "line_height" => wire::WireDeclarationName::LineHeight,
        "page" => wire::WireDeclarationName::Page,
        name => {
            return Err(DocumentPackageConversionError::UnknownStyleDeclarationName(
                name.to_owned(),
            ))
        }
    };
    let value = match &declaration.value {
        StyleValue::Keyword(value) => wire::WireStyleValue::Keyword {
            value: value.clone(),
        },
        StyleValue::Text(value) => wire::WireStyleValue::String {
            value: value.clone(),
        },
        StyleValue::Integer(value) => wire::WireStyleValue::Integer { value: *value },
        StyleValue::Length(value) => wire::WireStyleValue::Length { value: value.raw() },
        StyleValue::Boolean(value) => wire::WireStyleValue::Boolean { value: *value },
        StyleValue::FontFamilyList(families) => wire::WireStyleValue::FontFamilyList {
            families: families.clone(),
        },
        StyleValue::Ratio {
            numerator,
            denominator,
        } => wire::WireStyleValue::Ratio {
            numerator: *numerator,
            denominator: denominator.get(),
        },
    };
    Ok(wire::WireDeclaration {
        name,
        value,
        important: declaration.important,
    })
}

fn wire_page_masters(page_masters: &PageMasterSet) -> wire::WirePageMasterSet {
    wire::WirePageMasterSet {
        default_master_id: page_masters.default_master_id.as_str().to_owned(),
        masters: page_masters
            .masters
            .iter()
            .map(|master| wire::WirePageMaster {
                master_id: master.master_id.as_str().to_owned(),
                width: master.width.get().raw(),
                height: master.height.get().raw(),
                body: wire_rect(master.body),
                header: master.header.map(wire_rect),
                footer: master.footer.map(wire_rect),
                footnote: master.footnote.map(wire_rect),
            })
            .collect(),
        selection_rules: page_masters
            .selection_rules
            .iter()
            .map(|rule| wire::WirePageMasterRule {
                master_id: rule.master_id.as_str().to_owned(),
                parity: match rule.parity {
                    PageParity::Any => wire::WirePageParity::Any,
                    PageParity::Odd => wire::WirePageParity::Odd,
                    PageParity::Even => wire::WirePageParity::Even,
                },
                first: rule.first,
                named_page: rule
                    .named_page
                    .as_ref()
                    .map(|value| value.as_str().to_owned()),
                source_order: rule.source_order,
            })
            .collect(),
    }
}

fn wire_resources(resources: &ResourceCatalog) -> wire::WireResourceCatalog {
    wire::WireResourceCatalog {
        font_faces: resources
            .font_faces
            .iter()
            .map(|font| wire::WireFontFace {
                font_face_id: font.font_face_id.get(),
                family: font.family.clone(),
                uri: font.uri.as_str().to_owned(),
                face_index: font.face_index,
                expected_sha256: font.expected_sha256,
            })
            .collect(),
        images: resources
            .images
            .iter()
            .map(|image| wire::WireImage {
                image_id: image.image_id.get(),
                uri: image.uri.as_str().to_owned(),
                expected_sha256: image.expected_sha256,
            })
            .collect(),
    }
}

fn wire_rect(rect: Rect) -> wire::WireRect {
    wire::WireRect {
        x: rect.x().raw(),
        y: rect.y().raw(),
        width: rect.width().get().raw(),
        height: rect.height().get().raw(),
    }
}

fn wire_source_span(span: SourceSpan) -> wire::WireSourceSpan {
    wire::WireSourceSpan {
        source_id: span.source_id().get(),
        start_byte: span.start_byte().get(),
        end_byte: span.end_byte().get(),
    }
}

fn wire_text_span(span: TextSpan) -> wire::WireTextSpan {
    wire::WireTextSpan {
        text_id: span.text_id().get(),
        start_byte: span.range().start_byte().get(),
        end_byte: span.range().end_byte().get(),
    }
}

fn wire_byte_range(range: Utf8ByteRange) -> wire::WireByteRange {
    wire::WireByteRange {
        start_byte: range.start_byte().get(),
        end_byte: range.end_byte().get(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageValidationError {
    UnknownSource,
    SourceSpanOutOfBounds,
    SourceSpanNotUtf8Boundary,
    IdentityBytesMismatch,
    UnknownTextBuffer,
    TextSpanOutOfBounds,
    TextSpanNotUtf8Boundary,
    DuplicateNodeId,
    NonCanonicalNodeId,
    DuplicateAnchorId,
    DuplicateFootnoteId,
    UnknownInternalTarget,
    UnknownFootnoteTarget,
    DuplicateFontFaceId,
    NonCanonicalFontFaceId,
    DuplicateFontFamily,
    InvalidFontFamily,
    DuplicateImageId,
    NonCanonicalImageId,
    UnknownImageTarget,
    InvalidBlockClass,
    DuplicateBlockClass,
    NonCanonicalBlockClasses,
    InvalidStyle(StyleValidationError),
    InvalidPageMasters(PageMasterValidationError),
    InvalidUri(SafeUriError),
    InvalidListStart,
    EmptyListItems,
    ListMarkerOverflow,
    EmptyTableColumns,
    EmptyTableRows,
    InvalidTableGrid,
    TableHeadBodyCross,
    SourceByteLimit,
    InputByteLimit,
    IncludeFileLimit,
    AstNestingDepthLimit,
    AstNodeLimit,
    StyleRuleLimit,
    TextBufferByteLimit,
    TextByteLimit,
    NonCanonicalFootnoteOrder,
    MissingEntrySource,
    IncludeGraphMismatch,
    UnresolvedIncludeDirective,
}

#[derive(Clone, Debug)]
pub struct PackageValidationPolicy<'a> {
    limits: &'a ValidatedResourceLimits,
    allowed_uri_schemes: &'a [String],
}
impl<'a> PackageValidationPolicy<'a> {
    pub fn new(
        limits: &'a ValidatedResourceLimits,
        allowed_uri_schemes: &'a [String],
    ) -> Result<Self, SafeUriError> {
        let unique: BTreeSet<&str> = allowed_uri_schemes.iter().map(String::as_str).collect();
        if unique.len() != allowed_uri_schemes.len()
            || allowed_uri_schemes
                .iter()
                .any(|scheme| !DEFAULT_ALLOWED_URI_SCHEMES.contains(&scheme.as_str()))
        {
            return Err(SafeUriError::InvalidAllowedScheme);
        }
        if allowed_uri_schemes
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(SafeUriError::NonCanonicalAllowedSchemes);
        }
        Ok(Self {
            limits,
            allowed_uri_schemes,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedIncludeEdge {
    parent: SourceId,
    child: SourceId,
}
impl ResolvedIncludeEdge {
    #[allow(dead_code)] // reserved for the sealed in-crate resolver
    const fn new(parent: SourceId, child: SourceId) -> Self {
        Self { parent, child }
    }
    pub const fn parent(self) -> SourceId {
        self.parent
    }
    pub const fn child(self) -> SourceId {
        self.child
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncludeGraphError {
    MissingEntrySource,
    NonCanonicalEdgeOrder,
    MissingOrDuplicateParent,
    ParentNotPreviouslyResolved,
    IncludeDepthLimit,
    IncludeFileLimit,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IncludeSourceIdentity {
    source_id: SourceId,
    uri: PortablePath,
    sha256: [u8; 32],
}

/// Resolver-issued proof of entry/include closure. Every non-entry SourceId
/// has exactly one parent earlier in canonical resolver order, and its checked
/// depth is bound by the same immutable limits used for package validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedIncludeGraph {
    sources: Vec<IncludeSourceIdentity>,
    edges: Vec<ResolvedIncludeEdge>,
    max_observed_depth: u32,
}
/// In-process include resolver session owned by this crate's parser. It is not
/// public: untrusted callers cannot turn an arbitrary parent vector into a
/// trusted include-closure receipt.
///
/// ```compile_fail
/// use typaxis_syntax::IncludeResolverSession;
/// ```
#[allow(dead_code)] // production parser implementation owns this session
struct IncludeResolverSession<'a> {
    sources: &'a SourceCatalog,
    limits: &'a ValidatedResourceLimits,
    edges: Vec<ResolvedIncludeEdge>,
    depths: Vec<u32>,
    next_child: usize,
    max_observed_depth: u32,
}
impl<'a> IncludeResolverSession<'a> {
    #[allow(dead_code)] // reserved for the sealed in-crate parser owner
    fn new(
        sources: &'a SourceCatalog,
        limits: &'a ValidatedResourceLimits,
    ) -> Result<Self, IncludeGraphError> {
        if sources.records().is_empty() || sources.records()[0].source_id() != SourceId::new(0) {
            return Err(IncludeGraphError::MissingEntrySource);
        }
        let include_count = sources
            .records()
            .len()
            .checked_sub(1)
            .ok_or(IncludeGraphError::MissingEntrySource)?;
        if include_count > limits.get().max_include_files as usize {
            return Err(IncludeGraphError::IncludeFileLimit);
        }
        Ok(Self {
            sources,
            limits,
            edges: Vec::with_capacity(include_count),
            depths: vec![0u32; sources.records().len()],
            next_child: 1,
            max_observed_depth: 0,
        })
    }
    #[allow(dead_code)] // production parser implementation calls this per resolved directive
    fn admit_next_include(&mut self, parent: SourceId) -> Result<SourceId, IncludeGraphError> {
        if self.next_child >= self.sources.records().len() {
            return Err(IncludeGraphError::MissingOrDuplicateParent);
        }
        let child_value =
            u32::try_from(self.next_child).map_err(|_| IncludeGraphError::ArithmeticOverflow)?;
        let child = SourceId::new(child_value);
        if parent.get() >= child.get() {
            return Err(IncludeGraphError::ParentNotPreviouslyResolved);
        }
        let parent_depth = *self
            .depths
            .get(parent.get() as usize)
            .ok_or(IncludeGraphError::ParentNotPreviouslyResolved)?;
        let depth = parent_depth
            .checked_add(1)
            .ok_or(IncludeGraphError::ArithmeticOverflow)?;
        if depth > self.limits.get().max_include_depth {
            return Err(IncludeGraphError::IncludeDepthLimit);
        }
        self.depths[self.next_child] = depth;
        self.edges.push(ResolvedIncludeEdge::new(parent, child));
        self.next_child = self
            .next_child
            .checked_add(1)
            .ok_or(IncludeGraphError::ArithmeticOverflow)?;
        self.max_observed_depth = self.max_observed_depth.max(depth);
        Ok(child)
    }
    #[allow(dead_code)] // reserved for the sealed in-crate parser owner
    fn finish(self) -> Result<ValidatedIncludeGraph, IncludeGraphError> {
        if self.next_child != self.sources.records().len() {
            return Err(IncludeGraphError::MissingOrDuplicateParent);
        }
        let sources = self
            .sources
            .records()
            .iter()
            .map(|source| IncludeSourceIdentity {
                source_id: source.source_id(),
                uri: source.uri().clone(),
                sha256: source.content_hash(),
            })
            .collect();
        Ok(ValidatedIncludeGraph {
            sources,
            edges: self.edges,
            max_observed_depth: self.max_observed_depth,
        })
    }
}
impl ValidatedIncludeGraph {
    #[allow(dead_code)] // entry-only issuance is exposed only to fixture builds
    fn entry_only(
        sources: &SourceCatalog,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, IncludeGraphError> {
        IncludeResolverSession::new(sources, limits)?.finish()
    }
    pub const fn max_observed_depth(&self) -> u32 {
        self.max_observed_depth
    }
    pub fn edges(&self) -> &[ResolvedIncludeEdge] {
        &self.edges
    }
    fn matches(&self, sources: &SourceCatalog) -> bool {
        self.sources.len() == sources.records().len()
            && self
                .sources
                .iter()
                .zip(sources.records())
                .all(|(left, right)| {
                    left.source_id == right.source_id()
                        && left.uri == *right.uri()
                        && left.sha256 == right.content_hash()
                })
    }
}

/// Canonical document/style identities derived from the exact package
/// projections used by the portable cross-artifact validator. No API accepts
/// caller-provided digest bytes for these issued values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageEpochIdentity {
    document_jcs: String,
    style_jcs: String,
    document: DocumentFingerprint,
    style: StyleFingerprint,
}

/// Package-issued pagination inputs. The private identity fields prevent a
/// valid page-master set from being paired with another package's layout
/// epoch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagePaginationContext {
    page_masters: PageMasterSet,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    document_node_id: NodeId,
}
impl PackagePaginationContext {
    pub const fn page_masters(&self) -> &PageMasterSet {
        &self.page_masters
    }
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }
    pub const fn document_node_id(&self) -> NodeId {
        self.document_node_id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageGeneratedTextBinding<'a> {
    package: &'a ValidatedParsedPackage,
    generated_text: &'a GeneratedTextStore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageShapeTextSource {
    Parsed(TextSpan),
    Generated(GeneratedProvenance),
}

/// Canonical logical text-site identity for one paragraph. The sequence is
/// derived from the validated inline tree and is used by whole-paragraph
/// itemization to prevent callers from reordering or omitting shaping sites.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageParagraphTextSite {
    Parsed(TextSpan),
    Generated(GeneratedBufferKey),
}

/// Package-issued proof that shaping text belongs to one exact parsed or
/// selected-generated buffer and has a deterministic style context. Private
/// fields prevent a caller from pairing arbitrary bytes with another owner or
/// package fingerprint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageShapeTextReceipt<'a> {
    source: PackageShapeTextSource,
    site_owner: NodeId,
    style_owner: NodeId,
    utf8: &'a str,
    document: DocumentFingerprint,
    reference: Option<ReferenceFingerprint>,
    complete_site: bool,
    standalone_logical_text: bool,
}
impl<'a> PackageShapeTextReceipt<'a> {
    pub const fn source(&self) -> PackageShapeTextSource {
        self.source
    }
    pub const fn site_owner(&self) -> NodeId {
        self.site_owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
    pub const fn utf8(&self) -> &'a str {
        self.utf8
    }
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn reference_fingerprint(&self) -> Option<ReferenceFingerprint> {
        self.reference
    }
    /// Returns whether this receipt covers the complete package-declared text
    /// site rather than a caller-selected subspan of that site.
    pub const fn covers_complete_site(&self) -> bool {
        self.complete_site
    }
    /// Returns whether package structure proves that no adjacent inline text
    /// site can contribute bidi or shaping context to this receipt.
    pub const fn is_standalone_logical_text(&self) -> bool {
        self.standalone_logical_text
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageShapeTextError {
    UnknownParsedBuffer,
    InvalidSpanBoundary,
    UnownedParsedSpan,
    AmbiguousParsedSpan,
    UnknownGeneratedProvenance,
    UnknownGeneratedSite,
    MissingStyleOwner,
}

impl<'a> PackageGeneratedTextBinding<'a> {
    pub const fn package(&self) -> &'a ValidatedParsedPackage {
        self.package
    }
    pub const fn generated_text(&self) -> &'a GeneratedTextStore {
        self.generated_text
    }
    pub fn bind_generated_shape_text(
        &self,
        provenance: GeneratedProvenance,
    ) -> Result<PackageShapeTextReceipt<'a>, PackageShapeTextError> {
        if !self.generated_text.validates_provenance(provenance) {
            return Err(PackageShapeTextError::UnknownGeneratedProvenance);
        }
        let key = provenance.buffer_key();
        if self.package.document_nodes.generated_site(key).is_none() {
            return Err(PackageShapeTextError::UnknownGeneratedSite);
        }
        let style_owner = shape_style_owner(
            &self.package.package.document,
            self.package.document_nodes(),
            key.owner(),
        )
        .ok_or(PackageShapeTextError::MissingStyleOwner)?;
        let span = provenance.text_span();
        let buffer = self
            .generated_text
            .get(span.text_id())
            .ok_or(PackageShapeTextError::UnknownGeneratedProvenance)?;
        let start = span.range().start_byte().get() as usize;
        let end = span.range().end_byte().get() as usize;
        let utf8 = buffer
            .utf8()
            .get(start..end)
            .ok_or(PackageShapeTextError::InvalidSpanBoundary)?;
        Ok(PackageShapeTextReceipt {
            source: PackageShapeTextSource::Generated(provenance),
            site_owner: key.owner(),
            style_owner,
            utf8,
            document: self.package.epoch_identity.document(),
            reference: Some(self.generated_text.reference_fingerprint()),
            complete_site: start == 0 && end == buffer.utf8().len(),
            // List markers are separate layout text with spacing represented
            // by Glue. Inline-generated text is standalone only when package
            // structure proves it is the paragraph's sole logical site.
            standalone_logical_text: key.generation_kind() == GenerationKind::ListMarker
                || (key.generation_kind() != GenerationKind::Discretionary
                    && generated_inline_site_is_standalone(
                        &self.package.package.document,
                        key.owner(),
                    )),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageGeneratedTextError {
    DocumentMismatch,
    UnknownListMarkerSite,
    ListMarkerMismatch,
    FootnoteMarkerMismatch,
    ListMarkerOverflow,
    TextBufferLimit,
    TextTotalLimit,
    ArithmeticOverflow,
    GeneratedStoreRejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageComputedStyle {
    owner: NodeId,
    style_owner: NodeId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    computed: ComputedStyle,
}
impl PackageComputedStyle {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }
    pub const fn computed(&self) -> &ComputedStyle {
        &self.computed
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPageSelectionName {
    owner: NodeId,
    style_owner: NodeId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    page_name: Option<PageName>,
}
impl ResolvedPageSelectionName {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }
    pub const fn page_name(&self) -> Option<&PageName> {
        self.page_name.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageStyleError {
    UnknownStyleOwner,
    NonEmptyDocument,
    InvalidStyle(StyleValidationError),
}
impl PackageEpochIdentity {
    fn from_package(package: &ParsedPackage) -> Self {
        let document_jcs = encode_document_fingerprint_record(package);
        let style_jcs = encode_style_fingerprint_record(package);
        Self {
            document: document_fingerprint_from_jcs(&document_jcs),
            style: style_fingerprint_from_jcs(&style_jcs),
            document_jcs,
            style_jcs,
        }
    }
    pub const fn document(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style(&self) -> StyleFingerprint {
        self.style
    }
    pub fn document_jcs(&self) -> &str {
        &self.document_jcs
    }
    pub fn style_jcs(&self) -> &str {
        &self.style_jcs
    }
}

fn encode_document_fingerprint_record(package: &ParsedPackage) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, DocumentFingerprint::ALGORITHM_ID);
    output.push_str(",\"contract\":");
    // This fingerprint algorithm was published with the 1.2 normalized
    // document domain. Contract 1.3 adds syntax-owned pagination facts that
    // are bound by their own receipts; changing this discriminator would
    // silently rewrite every frozen 1.0--1.2 document identity.
    push_jcs_string(&mut output, "typaxis.contract/1.2");
    output.push_str(",\"coordinate_unit\":");
    push_jcs_string(&mut output, COORDINATE_UNIT);
    output.push_str(",\"document\":");
    push_document_jcs(&mut output, &package.document);
    output.push_str(",\"resources\":");
    push_resource_catalog_jcs(&mut output, &package.resources);
    output.push_str(",\"sources\":[");
    for (index, source) in package.sources.records().iter().enumerate() {
        push_separator(&mut output, index);
        output.push_str("{\"sha256\":");
        push_hash_hex(&mut output, source.content_hash());
        output.push_str(",\"source_id\":");
        output.push_str(&source.source_id().get().to_string());
        output.push_str(",\"uri\":");
        push_jcs_string(&mut output, source.uri().as_str());
        output.push_str(",\"utf8_byte_length\":");
        output.push_str(&source.utf8_byte_length().to_string());
        output.push('}');
    }
    output.push_str("],\"text_buffers\":[");
    for (index, buffer) in package.text_store.buffers().iter().enumerate() {
        push_separator(&mut output, index);
        output.push_str("{\"mappings\":[");
        for (mapping_index, mapping) in buffer.mappings().iter().enumerate() {
            push_separator(&mut output, mapping_index);
            output.push_str("{\"kind\":");
            push_jcs_string(
                &mut output,
                match mapping.kind {
                    TextMapKind::Identity => "identity",
                    TextMapKind::Replacement => "replacement",
                    TextMapKind::Inserted => "inserted",
                },
            );
            output.push_str(",\"source_span\":");
            push_optional_source_span_jcs(&mut output, mapping.source_span);
            output.push_str(",\"text_range\":{\"end_byte\":");
            output.push_str(&mapping.text_range.end_byte().get().to_string());
            output.push_str(",\"start_byte\":");
            output.push_str(&mapping.text_range.start_byte().get().to_string());
            output.push_str("}}");
        }
        output.push_str("],\"text_id\":");
        output.push_str(&buffer.text_id().get().to_string());
        output.push_str(",\"utf8\":");
        push_jcs_string(&mut output, buffer.text());
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn encode_style_fingerprint_record(package: &ParsedPackage) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, StyleFingerprint::ALGORITHM_ID);
    output.push_str(",\"page_masters\":");
    push_page_masters_jcs(&mut output, &package.page_masters);
    output.push_str(",\"style_sheet\":");
    push_style_sheet_jcs(&mut output, &package.style_sheet);
    output.push('}');
    output
}

fn push_resource_catalog_jcs(output: &mut String, resources: &ResourceCatalog) {
    output.push_str("{\"font_faces\":[");
    for (index, font) in resources.font_faces.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"expected_sha256\":");
        push_optional_hash_hex(output, font.expected_sha256);
        output.push_str(",\"face_index\":");
        output.push_str(&font.face_index.to_string());
        output.push_str(",\"family\":");
        push_jcs_string(output, &font.family);
        output.push_str(",\"font_face_id\":");
        output.push_str(&font.font_face_id.get().to_string());
        output.push_str(",\"uri\":");
        push_jcs_string(output, font.uri.as_str());
        output.push('}');
    }
    output.push_str("],\"images\":[");
    for (index, image) in resources.images.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"expected_sha256\":");
        push_optional_hash_hex(output, image.expected_sha256);
        output.push_str(",\"image_id\":");
        output.push_str(&image.image_id.get().to_string());
        output.push_str(",\"uri\":");
        push_jcs_string(output, image.uri.as_str());
        output.push('}');
    }
    output.push_str("]}");
}

fn push_optional_hash_hex(output: &mut String, bytes: Option<[u8; 32]>) {
    match bytes {
        Some(bytes) => push_hash_hex(output, bytes),
        None => output.push_str("null"),
    }
}

fn push_document_jcs(output: &mut String, document: &Document) {
    output.push_str("{\"blocks\":[");
    for (index, block) in document.blocks.iter().enumerate() {
        push_separator(output, index);
        push_block_jcs(output, block);
    }
    output.push_str("],\"footnotes\":[");
    for (index, footnote) in document.footnotes.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"blocks\":[");
        for (block_index, block) in footnote.blocks.iter().enumerate() {
            push_separator(output, block_index);
            push_block_jcs(output, block);
        }
        output.push_str("],\"footnote_id\":");
        push_jcs_string(output, footnote.footnote_id.as_str());
        output.push_str(",\"node_id\":");
        output.push_str(&footnote.node_id.get().to_string());
        output.push_str(",\"span\":");
        push_source_span_jcs(output, footnote.span);
        output.push('}');
    }
    output.push_str("],\"node_id\":");
    output.push_str(&document.node_id.get().to_string());
    output.push('}');
}

fn push_block_jcs(output: &mut String, block: &Block) {
    match block {
        Block::Paragraph {
            node_id,
            span,
            classes,
            children,
        } => {
            output.push_str("{\"children\":");
            push_inlines_jcs(output, children);
            output.push_str(",\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"kind\":\"paragraph\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Block::Heading {
            node_id,
            span,
            classes,
            level,
            anchor_id,
            children,
        } => {
            output.push_str("{\"anchor_id\":");
            push_optional_string_jcs(output, anchor_id.as_ref().map(AnchorId::as_str));
            output.push_str(",\"children\":");
            push_inlines_jcs(output, children);
            output.push_str(",\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"kind\":\"heading\",\"level\":");
            output.push_str(&level.get().to_string());
            output.push_str(",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Block::List {
            node_id,
            span,
            classes,
            ordered,
            start,
            items,
        } => {
            output.push_str("{\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"items\":[");
            for (index, item) in items.iter().enumerate() {
                push_separator(output, index);
                push_list_item_jcs(output, item);
            }
            output.push_str("],\"kind\":\"list\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"ordered\":");
            output.push_str(if *ordered { "true" } else { "false" });
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push_str(",\"start\":");
            push_optional_u32_jcs(output, *start);
            output.push('}');
        }
        Block::Table {
            node_id,
            span,
            classes,
            columns,
            head,
            body,
        } => {
            output.push_str("{\"body\":");
            push_table_rows_jcs(output, body);
            output.push_str(",\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"columns\":[");
            for (index, column) in columns.iter().enumerate() {
                push_separator(output, index);
                push_table_column_jcs(output, column);
            }
            output.push_str("],\"head\":");
            push_table_rows_jcs(output, head);
            output.push_str(",\"kind\":\"table\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Block::Figure {
            node_id,
            span,
            classes,
            image_id,
            alt,
            caption,
        } => {
            output.push_str("{\"alt\":");
            push_jcs_string(output, alt);
            output.push_str(",\"caption\":[");
            for (index, block) in caption.iter().enumerate() {
                push_separator(output, index);
                push_block_jcs(output, block);
            }
            output.push_str("],\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"image_id\":");
            output.push_str(&image_id.get().to_string());
            output.push_str(",\"kind\":\"figure\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Block::PageBreak {
            node_id,
            span,
            classes,
        } => {
            output.push_str("{\"classes\":");
            push_strings_jcs(output, classes);
            output.push_str(",\"kind\":\"page_break\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
    }
}

fn push_list_item_jcs(output: &mut String, item: &ListItem) {
    output.push_str("{\"blocks\":[");
    for (index, block) in item.blocks.iter().enumerate() {
        push_separator(output, index);
        push_block_jcs(output, block);
    }
    output.push_str("],\"node_id\":");
    output.push_str(&item.node_id.get().to_string());
    output.push_str(",\"span\":");
    push_source_span_jcs(output, item.span);
    output.push('}');
}

fn push_table_column_jcs(output: &mut String, column: &TableColumn) {
    match column.sizing {
        ColumnSizing::Fixed(width) => {
            output.push_str("{\"kind\":\"fixed\",\"width\":");
            output.push_str(&width.get().raw().to_string());
        }
        ColumnSizing::Fraction(weight) => {
            output.push_str("{\"kind\":\"fraction\",\"weight\":");
            output.push_str(&weight.get().to_string());
        }
    }
    output.push('}');
}

fn push_table_rows_jcs(output: &mut String, rows: &[TableRow]) {
    output.push('[');
    for (index, row) in rows.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"cells\":[");
        for (cell_index, cell) in row.cells.iter().enumerate() {
            push_separator(output, cell_index);
            output.push_str("{\"blocks\":[");
            for (block_index, block) in cell.blocks.iter().enumerate() {
                push_separator(output, block_index);
                push_block_jcs(output, block);
            }
            output.push_str("],\"colspan\":");
            output.push_str(&cell.colspan.get().to_string());
            output.push_str(",\"node_id\":");
            output.push_str(&cell.node_id.get().to_string());
            output.push_str(",\"rowspan\":");
            output.push_str(&cell.rowspan.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, cell.span);
            output.push('}');
        }
        output.push_str("],\"node_id\":");
        output.push_str(&row.node_id.get().to_string());
        output.push_str(",\"span\":");
        push_source_span_jcs(output, row.span);
        output.push('}');
    }
    output.push(']');
}

fn push_inlines_jcs(output: &mut String, inlines: &[Inline]) {
    output.push('[');
    for (index, inline) in inlines.iter().enumerate() {
        push_separator(output, index);
        push_inline_jcs(output, inline);
    }
    output.push(']');
}

fn push_inline_jcs(output: &mut String, inline: &Inline) {
    match inline {
        Inline::Text {
            node_id,
            span,
            text_span,
        } => {
            output.push_str("{\"kind\":\"text\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push_str(",\"text_span\":");
            push_text_span_jcs(output, *text_span);
            output.push('}');
        }
        Inline::Emphasis {
            node_id,
            span,
            children,
        }
        | Inline::Strong {
            node_id,
            span,
            children,
        } => {
            output.push_str("{\"children\":");
            push_inlines_jcs(output, children);
            output.push_str(",\"kind\":");
            push_jcs_string(
                output,
                if matches!(inline, Inline::Emphasis { .. }) {
                    "emphasis"
                } else {
                    "strong"
                },
            );
            output.push_str(",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Inline::Link {
            node_id,
            span,
            target,
            children,
        } => {
            output.push_str("{\"children\":");
            push_inlines_jcs(output, children);
            output.push_str(",\"kind\":\"link\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push_str(",\"target\":");
            match target {
                LinkTarget::Internal(anchor) => {
                    output.push_str("{\"anchor_id\":");
                    push_jcs_string(output, anchor.as_str());
                    output.push_str(",\"kind\":\"internal\"}");
                }
                LinkTarget::Uri(uri) => {
                    output.push_str("{\"kind\":\"uri\",\"uri\":");
                    push_jcs_string(output, uri.as_str());
                    output.push('}');
                }
            }
            output.push('}');
        }
        Inline::Anchor {
            node_id,
            span,
            anchor_id,
        } => {
            output.push_str("{\"anchor_id\":");
            push_jcs_string(output, anchor_id.as_str());
            output.push_str(",\"kind\":\"anchor\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Inline::Reference {
            node_id,
            span,
            target,
            format,
        } => {
            output.push_str("{\"format\":");
            push_jcs_string(
                output,
                match format {
                    ReferenceFormat::Text => "text",
                    ReferenceFormat::Page => "page",
                    ReferenceFormat::Number => "number",
                },
            );
            output.push_str(",\"kind\":\"reference\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push_str(",\"target\":");
            push_jcs_string(output, target.as_str());
            output.push('}');
        }
        Inline::FootnoteReference {
            node_id,
            span,
            footnote_id,
        } => {
            output.push_str("{\"footnote_id\":");
            push_jcs_string(output, footnote_id.as_str());
            output.push_str(",\"kind\":\"footnote_reference\",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
        Inline::SoftBreak { node_id, span } | Inline::HardBreak { node_id, span } => {
            output.push_str("{\"kind\":");
            push_jcs_string(
                output,
                if matches!(inline, Inline::SoftBreak { .. }) {
                    "soft_break"
                } else {
                    "hard_break"
                },
            );
            output.push_str(",\"node_id\":");
            output.push_str(&node_id.get().to_string());
            output.push_str(",\"span\":");
            push_source_span_jcs(output, *span);
            output.push('}');
        }
    }
}

fn push_style_sheet_jcs(output: &mut String, style_sheet: &StyleSheet) {
    output.push_str("{\"rules\":[");
    for (index, rule) in style_sheet.rules.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"declarations\":[");
        for (declaration_index, declaration) in rule.declarations.iter().enumerate() {
            push_separator(output, declaration_index);
            output.push_str("{\"important\":");
            output.push_str(if declaration.important {
                "true"
            } else {
                "false"
            });
            output.push_str(",\"name\":");
            push_jcs_string(output, &declaration.name);
            output.push_str(",\"value\":");
            push_style_value_jcs(output, &declaration.value);
            output.push('}');
        }
        output.push_str("],\"extends\":");
        push_optional_string_jcs(output, rule.extends.as_ref().map(|value| value.as_str()));
        output.push_str(",\"selector\":");
        push_jcs_string(output, &rule.selector);
        output.push_str(",\"source_order\":");
        output.push_str(&rule.source_order.to_string());
        output.push_str(",\"style_id\":");
        push_jcs_string(output, rule.style_id.as_str());
        output.push('}');
    }
    output.push_str("]}");
}

fn push_style_value_jcs(output: &mut String, value: &StyleValue) {
    match value {
        StyleValue::Keyword(value) => push_kind_value_jcs(output, "keyword", value),
        StyleValue::Text(value) => push_kind_value_jcs(output, "string", value),
        StyleValue::Integer(value) => {
            output.push_str("{\"kind\":\"integer\",\"value\":");
            output.push_str(&value.to_string());
            output.push('}');
        }
        StyleValue::Length(value) => {
            output.push_str("{\"kind\":\"length\",\"value\":");
            output.push_str(&value.raw().to_string());
            output.push('}');
        }
        StyleValue::Boolean(value) => {
            output.push_str("{\"kind\":\"boolean\",\"value\":");
            output.push_str(if *value { "true" } else { "false" });
            output.push('}');
        }
        StyleValue::FontFamilyList(families) => {
            output.push_str("{\"families\":");
            push_strings_jcs(output, families);
            output.push_str(",\"kind\":\"font_family_list\"}");
        }
        StyleValue::Ratio {
            numerator,
            denominator,
        } => {
            output.push_str("{\"denominator\":");
            output.push_str(&denominator.get().to_string());
            output.push_str(",\"kind\":\"ratio\",\"numerator\":");
            output.push_str(&numerator.to_string());
            output.push('}');
        }
    }
}

fn push_kind_value_jcs(output: &mut String, kind: &str, value: &str) {
    output.push_str("{\"kind\":");
    push_jcs_string(output, kind);
    output.push_str(",\"value\":");
    push_jcs_string(output, value);
    output.push('}');
}

fn push_page_masters_jcs(output: &mut String, page_masters: &PageMasterSet) {
    output.push_str("{\"default_master_id\":");
    push_jcs_string(output, page_masters.default_master_id.as_str());
    output.push_str(",\"masters\":[");
    for (index, master) in page_masters.masters.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"body\":");
        push_rect_jcs(output, master.body);
        output.push_str(",\"footer\":");
        push_optional_rect_jcs(output, master.footer);
        output.push_str(",\"footnote\":");
        push_optional_rect_jcs(output, master.footnote);
        output.push_str(",\"header\":");
        push_optional_rect_jcs(output, master.header);
        output.push_str(",\"height\":");
        output.push_str(&master.height.get().raw().to_string());
        output.push_str(",\"master_id\":");
        push_jcs_string(output, master.master_id.as_str());
        output.push_str(",\"width\":");
        output.push_str(&master.width.get().raw().to_string());
        output.push('}');
    }
    output.push_str("],\"selection_rules\":[");
    for (index, rule) in page_masters.selection_rules.iter().enumerate() {
        push_separator(output, index);
        output.push_str("{\"first\":");
        match rule.first {
            Some(value) => output.push_str(if value { "true" } else { "false" }),
            None => output.push_str("null"),
        }
        output.push_str(",\"master_id\":");
        push_jcs_string(output, rule.master_id.as_str());
        output.push_str(",\"named_page\":");
        push_optional_string_jcs(output, rule.named_page.as_ref().map(|value| value.as_str()));
        output.push_str(",\"parity\":");
        push_jcs_string(
            output,
            match rule.parity {
                PageParity::Any => "any",
                PageParity::Odd => "odd",
                PageParity::Even => "even",
            },
        );
        output.push_str(",\"source_order\":");
        output.push_str(&rule.source_order.to_string());
        output.push('}');
    }
    output.push_str("]}");
}

fn push_source_span_jcs(output: &mut String, span: SourceSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&span.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&span.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&span.start_byte().get().to_string());
    output.push('}');
}

fn push_optional_source_span_jcs(output: &mut String, span: Option<SourceSpan>) {
    match span {
        Some(span) => push_source_span_jcs(output, span),
        None => output.push_str("null"),
    }
}

fn push_text_span_jcs(output: &mut String, span: TextSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&span.range().end_byte().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&span.range().start_byte().get().to_string());
    output.push_str(",\"text_id\":");
    output.push_str(&span.text_id().get().to_string());
    output.push('}');
}

fn push_rect_jcs(output: &mut String, rect: Rect) {
    output.push_str("{\"height\":");
    output.push_str(&rect.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&rect.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&rect.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&rect.y().raw().to_string());
    output.push('}');
}

fn push_optional_rect_jcs(output: &mut String, rect: Option<Rect>) {
    match rect {
        Some(rect) => push_rect_jcs(output, rect),
        None => output.push_str("null"),
    }
}

fn push_strings_jcs(output: &mut String, values: &[String]) {
    output.push('[');
    for (index, value) in values.iter().enumerate() {
        push_separator(output, index);
        push_jcs_string(output, value);
    }
    output.push(']');
}

fn push_optional_string_jcs(output: &mut String, value: Option<&str>) {
    match value {
        Some(value) => push_jcs_string(output, value),
        None => output.push_str("null"),
    }
}

fn push_optional_u32_jcs(output: &mut String, value: Option<u32>) {
    match value {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push_str("null"),
    }
}

fn push_separator(output: &mut String, index: usize) {
    if index > 0 {
        output.push(',');
    }
}

fn push_hash_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

/// A parsed package that has crossed the syntax phase's validation boundary.
/// Arbitrary `ParsedPackage` values cannot be promoted through a feature flag:
///
/// ```compile_fail
/// use typaxis_syntax::ValidatedParsedPackage;
/// let _ = ValidatedParsedPackage::new_entry_only;
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedParsedPackage {
    package: ParsedPackage,
    document_nodes: ValidatedDocumentNodeIndex,
    include_graph: ValidatedIncludeGraph,
    epoch_identity: PackageEpochIdentity,
    extended_style_contract: bool,
}
impl ValidatedParsedPackage {
    /// Validates an entry-only parser result. The syntax keyword scan prevents
    /// a caller from claiming entry-only closure while the admitted entry still
    /// contains an unresolved include directive. Multi-source issuance remains
    /// owned by the in-crate include resolver.
    #[cfg(test)]
    fn new_entry_only(
        package: ParsedPackage,
        policy: &PackageValidationPolicy<'_>,
    ) -> Result<Self, PackageValidationError> {
        if package.sources.records().len() != 1 {
            return Err(PackageValidationError::IncludeGraphMismatch);
        }
        if contains_include_directive(package.sources.records()[0].utf8()) {
            return Err(PackageValidationError::UnresolvedIncludeDirective);
        }
        let include_graph = ValidatedIncludeGraph::entry_only(&package.sources, policy.limits)
            .map_err(|_| PackageValidationError::IncludeGraphMismatch)?;
        Self::new_resolved(package, policy, &include_graph, |_, error| error)
    }

    fn new_resolved<E>(
        package: ParsedPackage,
        policy: &PackageValidationPolicy<'_>,
        include_graph: &ValidatedIncludeGraph,
        map_error: impl FnOnce(&ParsedPackage, PackageValidationError) -> E,
    ) -> Result<Self, E> {
        Self::new_resolved_with_style_contract(package, policy, include_graph, false, map_error)
    }

    fn new_resolved_with_style_contract<E>(
        package: ParsedPackage,
        policy: &PackageValidationPolicy<'_>,
        include_graph: &ValidatedIncludeGraph,
        staging_style_1_2: bool,
        map_error: impl FnOnce(&ParsedPackage, PackageValidationError) -> E,
    ) -> Result<Self, E> {
        let validation = (|| -> Result<_, PackageValidationError> {
            if !include_graph.matches(&package.sources)
                || include_graph.max_observed_depth() > policy.limits.get().max_include_depth
            {
                return Err(PackageValidationError::IncludeGraphMismatch);
            }
            let non_document_ast_nodes = validate_package_limits(&package, policy)?;
            validate_document_ast_limits(&package.document, policy.limits, non_document_ast_nodes)?;
            for buffer in package.text_store.buffers() {
                for mapping in buffer.mappings() {
                    if let Some(source_span) = mapping.source_span {
                        let source = validate_source_span(&package, source_span)?;
                        if mapping.kind == TextMapKind::Identity {
                            let text_start = mapping.text_range.start_byte().get() as usize;
                            let text_end = mapping.text_range.end_byte().get() as usize;
                            let source_start = source_span.start_byte().get() as usize;
                            let source_end = source_span.end_byte().get() as usize;
                            if buffer.text()[text_start..text_end]
                                != source.utf8()[source_start..source_end]
                            {
                                return Err(PackageValidationError::IdentityBytesMismatch);
                            }
                        }
                    }
                }
            }
            validate_style_inheritance_depth(&package.style_sheet, policy.limits)?;
            if staging_style_1_2 {
                package
                    .style_sheet
                    .validate_basic_document_style_shape()
                    .map_err(PackageValidationError::InvalidStyle)?;
            } else {
                package
                    .style_sheet
                    .validate()
                    .map_err(PackageValidationError::InvalidStyle)?;
            }
            package
                .page_masters
                .validate()
                .map_err(PackageValidationError::InvalidPageMasters)?;
            let image_ids = validate_resource_catalog(&package.resources)?;
            validate_document(
                &package,
                &image_ids,
                policy,
                non_document_ast_nodes,
                staging_style_1_2,
            )?;
            let document_nodes = ValidatedDocumentNodeIndex::new(&package.document)
                .map_err(|_| PackageValidationError::NonCanonicalNodeId)?;
            let epoch_identity = PackageEpochIdentity::from_package(&package);
            Ok((document_nodes, epoch_identity))
        })();
        let (document_nodes, epoch_identity) = match validation {
            Ok(validated) => validated,
            Err(error) => return Err(map_error(&package, error)),
        };
        Ok(Self {
            package,
            document_nodes,
            include_graph: include_graph.clone(),
            epoch_identity,
            extended_style_contract: staging_style_1_2,
        })
    }

    pub fn package(&self) -> &ParsedPackage {
        &self.package
    }
    /// Converts the complete validated domain package to its untrusted wire DTO.
    ///
    /// This conversion is intentionally owned by the syntax/domain boundary so
    /// downstream callers do not duplicate variant spelling or field ordering.
    pub fn to_wire_document_package(
        &self,
    ) -> Result<WireDocumentPackage, DocumentPackageConversionError> {
        parsed_package_to_wire(&self.package)
    }
    pub const fn document_nodes(&self) -> &ValidatedDocumentNodeIndex {
        &self.document_nodes
    }
    pub const fn include_graph(&self) -> &ValidatedIncludeGraph {
        &self.include_graph
    }
    pub const fn epoch_identity(&self) -> &PackageEpochIdentity {
        &self.epoch_identity
    }
    pub fn pagination_context(&self) -> PackagePaginationContext {
        PackagePaginationContext {
            page_masters: self.package.page_masters.clone(),
            document: self.epoch_identity.document(),
            style: self.epoch_identity.style(),
            document_node_id: self.package.document.node_id,
        }
    }
    /// Materializes the canonical Profile 1.0 bytes for one registered list
    /// marker. Ordered markers are ASCII decimal plus `.`; unordered markers
    /// are U+2022. Marker-adjacent spacing remains layout Glue.
    pub fn materialize_list_marker(
        &self,
        key: GeneratedBufferKey,
    ) -> Result<GeneratedBufferDraft, PackageGeneratedTextError> {
        if key.generation_kind() != GenerationKind::ListMarker || key.owner_local_ordinal() != 0 {
            return Err(PackageGeneratedTextError::UnknownListMarkerSite);
        }
        let markers = canonical_list_marker_texts(&self.package.document)?;
        let utf8 = markers
            .get(&key.owner())
            .ok_or(PackageGeneratedTextError::UnknownListMarkerSite)?
            .clone();
        GeneratedBufferDraft::new(&self.document_nodes, key, utf8)
            .map_err(|_| PackageGeneratedTextError::UnknownListMarkerSite)
    }
    /// Builds the deterministic state-0 generated-text overlay solely from
    /// validated package facts. State-dependent references begin empty;
    /// list and footnote markers are canonical package-derived text, and
    /// explicit soft/hard-break discretionary sites begin empty.
    pub fn materialize_initial_generated_text(
        &self,
        limits: &ValidatedResourceLimits,
    ) -> Result<GeneratedTextStore, PackageGeneratedTextError> {
        let footnote_markers =
            canonical_footnote_marker_texts(&self.package.document, &self.document_nodes)?;
        let mut drafts = Vec::new();
        drafts
            .try_reserve_exact(self.document_nodes.generated_sites().len())
            .map_err(|_| PackageGeneratedTextError::GeneratedStoreRejected)?;
        for site in self.document_nodes.generated_sites() {
            let key = site.key();
            let utf8 = match key.generation_kind() {
                GenerationKind::ListMarker => {
                    drafts.push(self.materialize_list_marker(key)?);
                    continue;
                }
                GenerationKind::FootnoteMarker => footnote_markers
                    .get(&key)
                    .cloned()
                    .ok_or(PackageGeneratedTextError::GeneratedStoreRejected)?,
                GenerationKind::PageReference
                | GenerationKind::Counter
                | GenerationKind::Discretionary => String::new(),
            };
            drafts.push(
                GeneratedBufferDraft::new(&self.document_nodes, key, utf8)
                    .map_err(|_| PackageGeneratedTextError::GeneratedStoreRejected)?,
            );
        }
        GeneratedTextStore::new(
            drafts,
            &self.document_nodes,
            limits,
            &self.package.text_store,
        )
        .map_err(|_| PackageGeneratedTextError::GeneratedStoreRejected)
    }
    pub fn bind_generated_text<'a>(
        &'a self,
        generated_text: &'a GeneratedTextStore,
        limits: &ValidatedResourceLimits,
    ) -> Result<PackageGeneratedTextBinding<'a>, PackageGeneratedTextError> {
        if generated_text.document_nodes() != self.document_nodes() {
            return Err(PackageGeneratedTextError::DocumentMismatch);
        }
        let list_markers = canonical_list_marker_texts(&self.package.document)?;
        let footnote_markers =
            canonical_footnote_marker_texts(&self.package.document, &self.document_nodes)?;
        let limits = limits.get();
        let mut total = 0u64;
        for buffer in self.package.text_store.buffers() {
            let bytes = u64::from(buffer.byte_len());
            if bytes > u64::from(limits.max_text_buffer_bytes) {
                return Err(PackageGeneratedTextError::TextBufferLimit);
            }
            total = total
                .checked_add(bytes)
                .ok_or(PackageGeneratedTextError::ArithmeticOverflow)?;
        }
        for buffer in generated_text.buffers() {
            if buffer.key().generation_kind() == GenerationKind::ListMarker
                && list_markers.get(&buffer.key().owner()).map(String::as_str)
                    != Some(buffer.utf8())
            {
                return Err(PackageGeneratedTextError::ListMarkerMismatch);
            }
            if buffer.key().generation_kind() == GenerationKind::FootnoteMarker
                && footnote_markers.get(&buffer.key()).map(String::as_str) != Some(buffer.utf8())
            {
                return Err(PackageGeneratedTextError::FootnoteMarkerMismatch);
            }
            let bytes = u64::try_from(buffer.utf8().len())
                .map_err(|_| PackageGeneratedTextError::ArithmeticOverflow)?;
            if bytes > u64::from(limits.max_text_buffer_bytes) {
                return Err(PackageGeneratedTextError::TextBufferLimit);
            }
            total = total
                .checked_add(bytes)
                .ok_or(PackageGeneratedTextError::ArithmeticOverflow)?;
        }
        if total > limits.max_text_bytes {
            return Err(PackageGeneratedTextError::TextTotalLimit);
        }
        Ok(PackageGeneratedTextBinding {
            package: self,
            generated_text,
        })
    }
    pub fn bind_parsed_shape_text(
        &self,
        span: TextSpan,
    ) -> Result<PackageShapeTextReceipt<'_>, PackageShapeTextError> {
        let buffer = self
            .package
            .text_store
            .get(span.text_id())
            .ok_or(PackageShapeTextError::UnknownParsedBuffer)?;
        let start = span.start_byte().get() as usize;
        let end = span.end_byte().get() as usize;
        let utf8 = buffer
            .text()
            .get(start..end)
            .ok_or(PackageShapeTextError::InvalidSpanBoundary)?;
        let (site_owner, style_owner, declared_span, standalone_logical_text) =
            parsed_shape_owners(&self.package.document, span)?;
        Ok(PackageShapeTextReceipt {
            source: PackageShapeTextSource::Parsed(span),
            site_owner,
            style_owner,
            utf8,
            document: self.epoch_identity.document(),
            reference: None,
            complete_site: span == declared_span,
            standalone_logical_text,
        })
    }
    pub fn cascade_style(&self, owner: NodeId) -> Result<PackageComputedStyle, PackageStyleError> {
        self.cascade_style_for_owner(owner, owner)
    }

    /// Resolves the style of the first text-producing definition block while
    /// retaining the definition NodeId as the generated marker's site owner.
    /// Ordinary style lookup deliberately continues to reject definition
    /// containers.
    pub fn cascade_footnote_marker_style(
        &self,
        definition_owner: NodeId,
    ) -> Result<PackageComputedStyle, PackageStyleError> {
        if self.document_nodes.node_kind(definition_owner)
            != Some(DocumentNodeKind::FootnoteDefinition)
        {
            return Err(PackageStyleError::UnknownStyleOwner);
        }
        let style_owner = shape_style_owner(
            &self.package.document,
            &self.document_nodes,
            definition_owner,
        )
        .ok_or(PackageStyleError::UnknownStyleOwner)?;
        self.cascade_style_for_owner(definition_owner, style_owner)
    }

    fn cascade_style_for_owner(
        &self,
        site_owner: NodeId,
        lookup_owner: NodeId,
    ) -> Result<PackageComputedStyle, PackageStyleError> {
        let (style_owner, block_type, classes) =
            find_styleable_block(&self.package.document, lookup_owner)
                .ok_or(PackageStyleError::UnknownStyleOwner)?;
        let computed = if self.extended_style_contract {
            self.package
                .style_sheet
                .cascade_basic_document(block_type, classes)
        } else {
            self.package.style_sheet.cascade(block_type, classes)
        }
        .map_err(PackageStyleError::InvalidStyle)?;
        Ok(PackageComputedStyle {
            owner: site_owner,
            style_owner,
            document: self.epoch_identity.document(),
            style: self.epoch_identity.style(),
            computed,
        })
    }

    pub fn paragraph_shape_text_sites(
        &self,
        paragraph_owner: NodeId,
    ) -> Option<Vec<PackageParagraphTextSite>> {
        paragraph_inline_children(&self.package.document, paragraph_owner).map(|children| {
            let mut sites = Vec::new();
            sites.extend(self.document_nodes.generated_sites().filter_map(|site| {
                let key = site.key();
                (key.generation_kind() == GenerationKind::FootnoteMarker
                    && self.document_nodes.node_kind(key.owner())
                        == Some(DocumentNodeKind::FootnoteDefinition)
                    && shape_style_owner(&self.package.document, &self.document_nodes, key.owner())
                        == Some(paragraph_owner))
                .then_some(PackageParagraphTextSite::Generated(key))
            }));
            collect_shape_text_site_identities(children, &mut sites);
            sites
        })
    }
    pub fn resolve_page_selection(
        &self,
        owner: NodeId,
    ) -> Result<ResolvedPageSelectionName, PackageStyleError> {
        let computed = self.cascade_style(owner)?;
        let page_name = computed
            .computed
            .page_name()
            .map_err(PackageStyleError::InvalidStyle)?;
        Ok(ResolvedPageSelectionName {
            owner,
            style_owner: computed.style_owner,
            document: computed.document,
            style: computed.style,
            page_name,
        })
    }
    /// Issues the `auto` page selection only for the canonical blank-document
    /// case. Non-empty flow must resolve the `page` property for its owner.
    pub fn resolve_blank_page_selection(
        &self,
    ) -> Result<ResolvedPageSelectionName, PackageStyleError> {
        if !self.package.document.blocks.is_empty() || !self.package.document.footnotes.is_empty() {
            return Err(PackageStyleError::NonEmptyDocument);
        }
        Ok(ResolvedPageSelectionName {
            owner: self.package.document.node_id,
            style_owner: self.package.document.node_id,
            document: self.epoch_identity.document(),
            style: self.epoch_identity.style(),
            page_name: None,
        })
    }
    pub fn into_package(self) -> ParsedPackage {
        self.package
    }
}

/// Typed reason a decoder-admitted package did not cross the syntax trust
/// boundary. Capability/PDF support is intentionally not represented here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachineParseFailureKind {
    AdmissionSessionMismatch,
    AdmissionProgressMismatch,
    AdmissionFingerprintMismatch,
    PackageIdentityMismatch,
    ContractMismatch,
    CoordinateUnitMismatch,
    SourceCountMismatch,
    SourceDeclarationMismatch,
    SourceBytesMismatch,
    InvalidSourceCatalog,
    NonCanonicalTextBufferId,
    InvalidTextBuffer,
    InvalidSourceSpan,
    InvalidTextSpan,
    NonCanonicalNodeId,
    AstNestingDepthLimit,
    AstNodeLimit,
    NonCanonicalStyleOrder,
    NonCanonicalPageMasterOrder,
    NonCanonicalFontFaceId,
    NonCanonicalImageId,
    InvalidIdentifier,
    InvalidHeadingLevel,
    InvalidLength,
    InvalidTableShape,
    InvalidUri,
    AdvancedSyntax(StagingAdvancedSyntaxFailure),
    PackageValidation(PackageValidationError),
}

impl MachineParseFailureKind {
    /// Stable public class assignment. This match is the only syntax-to-public
    /// code mapper and never inspects formatted error text.
    pub fn public_error(&self) -> PublicMachineError {
        match self {
            Self::AdmissionSessionMismatch
            | Self::AdmissionProgressMismatch
            | Self::AdmissionFingerprintMismatch
            | Self::PackageIdentityMismatch => PublicMachineError::CapabilityDomainMismatch,
            Self::ContractMismatch | Self::CoordinateUnitMismatch => {
                PublicMachineError::PackageContract
            }
            Self::SourceCountMismatch | Self::SourceDeclarationMismatch => {
                PublicMachineError::SourceProfile
            }
            Self::SourceBytesMismatch => PublicMachineError::SourceIdentity,
            Self::PackageValidation(
                PackageValidationError::IdentityBytesMismatch
                | PackageValidationError::SourceSpanOutOfBounds
                | PackageValidationError::SourceSpanNotUtf8Boundary
                | PackageValidationError::SourceByteLimit
                | PackageValidationError::InputByteLimit,
            ) => PublicMachineError::SourceIdentity,
            Self::PackageValidation(
                PackageValidationError::MissingEntrySource
                | PackageValidationError::IncludeFileLimit
                | PackageValidationError::IncludeGraphMismatch
                | PackageValidationError::UnresolvedIncludeDirective,
            ) => PublicMachineError::SourceProfile,
            Self::AdvancedSyntax(_) => PublicMachineError::PackageMember,
            _ => PublicMachineError::PackageMember,
        }
    }

    pub const fn canonical_message(&self) -> &'static str {
        match self {
            Self::AdmissionSessionMismatch => "machine admission session does not match",
            Self::AdmissionProgressMismatch => "machine admission progress does not match",
            Self::AdmissionFingerprintMismatch => "machine admission fingerprint does not match",
            Self::PackageIdentityMismatch => "package identity does not match admission",
            Self::ContractMismatch => "DocumentPackage contract is unsupported",
            Self::CoordinateUnitMismatch => "coordinate unit is unsupported",
            Self::SourceCountMismatch => "source profile requires exactly one source",
            Self::SourceDeclarationMismatch => "source declaration does not match admission",
            Self::SourceBytesMismatch => "source bytes do not match admitted identity",
            Self::InvalidSourceCatalog => "source catalog is invalid",
            Self::NonCanonicalTextBufferId => "text buffer ID is not canonical",
            Self::InvalidTextBuffer => "text buffer is invalid",
            Self::InvalidSourceSpan => "source span is invalid",
            Self::InvalidTextSpan => "text span is invalid",
            Self::NonCanonicalNodeId => "node ID is not canonical",
            Self::AstNestingDepthLimit => "AST nesting depth limit was exceeded",
            Self::AstNodeLimit => "AST node limit was exceeded",
            Self::NonCanonicalStyleOrder => "style order is not canonical",
            Self::NonCanonicalPageMasterOrder => "page master order is not canonical",
            Self::NonCanonicalFontFaceId => "font face ID is not canonical",
            Self::NonCanonicalImageId => "image ID is not canonical",
            Self::InvalidIdentifier => "identifier is invalid",
            Self::InvalidHeadingLevel => "heading level is invalid",
            Self::InvalidLength => "length is invalid",
            Self::InvalidTableShape => "table shape is invalid",
            Self::InvalidUri => "URI is invalid",
            Self::AdvancedSyntax(error) => match error {
                StagingAdvancedSyntaxFailure::MasterExtensionMismatch => {
                    "advanced page-master extension does not match"
                }
                StagingAdvancedSyntaxFailure::InvalidLength => {
                    "advanced fixed-point length is invalid"
                }
                StagingAdvancedSyntaxFailure::InvalidNodeOrder => {
                    "advanced node ID order is not canonical"
                }
                StagingAdvancedSyntaxFailure::InvalidSourceSpan => {
                    "advanced source span is invalid"
                }
                StagingAdvancedSyntaxFailure::InvalidTextSpan => "advanced text span is invalid",
                StagingAdvancedSyntaxFailure::InvalidClass => "advanced block class is invalid",
                StagingAdvancedSyntaxFailure::InvalidHeadingLevel => {
                    "advanced heading level is invalid"
                }
                StagingAdvancedSyntaxFailure::InvalidFigurePlacement => {
                    "advanced Figure placement registry does not match"
                }
                StagingAdvancedSyntaxFailure::AstNodeLimit => {
                    "advanced AST node limit was exceeded"
                }
                StagingAdvancedSyntaxFailure::AstDepthLimit => {
                    "advanced AST nesting depth limit was exceeded"
                }
                StagingAdvancedSyntaxFailure::ArithmeticOverflow => {
                    "advanced syntax arithmetic overflowed"
                }
            },
            Self::PackageValidation(error) => package_validation_canonical_message(error),
        }
    }
}

const fn package_validation_canonical_message(error: &PackageValidationError) -> &'static str {
    match error {
        PackageValidationError::UnknownSource => "source ID is unknown",
        PackageValidationError::SourceSpanOutOfBounds => "source span is out of bounds",
        PackageValidationError::SourceSpanNotUtf8Boundary => {
            "source span is not on UTF-8 boundaries"
        }
        PackageValidationError::IdentityBytesMismatch => {
            "identity text mapping does not match source bytes"
        }
        PackageValidationError::UnknownTextBuffer => "text buffer ID is unknown",
        PackageValidationError::TextSpanOutOfBounds => "text span is out of bounds",
        PackageValidationError::TextSpanNotUtf8Boundary => "text span is not on UTF-8 boundaries",
        PackageValidationError::DuplicateNodeId => "node ID is duplicated",
        PackageValidationError::NonCanonicalNodeId => "node ID is not canonical",
        PackageValidationError::DuplicateAnchorId => "anchor ID is duplicated",
        PackageValidationError::DuplicateFootnoteId => "footnote ID is duplicated",
        PackageValidationError::UnknownInternalTarget => "internal target is unknown",
        PackageValidationError::UnknownFootnoteTarget => "footnote target is unknown",
        PackageValidationError::DuplicateFontFaceId => "font face ID is duplicated",
        PackageValidationError::NonCanonicalFontFaceId => "font face ID is not canonical",
        PackageValidationError::DuplicateFontFamily => "font family is duplicated",
        PackageValidationError::InvalidFontFamily => "font family is invalid",
        PackageValidationError::DuplicateImageId => "image ID is duplicated",
        PackageValidationError::NonCanonicalImageId => "image ID is not canonical",
        PackageValidationError::UnknownImageTarget => "image target is unknown",
        PackageValidationError::InvalidBlockClass => "block class is invalid",
        PackageValidationError::DuplicateBlockClass => "block class is duplicated",
        PackageValidationError::NonCanonicalBlockClasses => "block class order is not canonical",
        PackageValidationError::InvalidStyle(_) => "style sheet is invalid",
        PackageValidationError::InvalidPageMasters(_) => "page master set is invalid",
        PackageValidationError::InvalidUri(_) => "URI policy was rejected",
        PackageValidationError::InvalidListStart => "list start is invalid",
        PackageValidationError::EmptyListItems => "list has no items",
        PackageValidationError::ListMarkerOverflow => "list marker exceeds the supported range",
        PackageValidationError::EmptyTableColumns => "table has no columns",
        PackageValidationError::EmptyTableRows => "table has no rows",
        PackageValidationError::InvalidTableGrid => "table grid is invalid",
        PackageValidationError::TableHeadBodyCross => "table row span crosses head and body",
        PackageValidationError::SourceByteLimit => "source byte limit was exceeded",
        PackageValidationError::InputByteLimit => "aggregate input byte limit was exceeded",
        PackageValidationError::IncludeFileLimit => "source file count limit was exceeded",
        PackageValidationError::AstNestingDepthLimit => "AST nesting depth limit was exceeded",
        PackageValidationError::AstNodeLimit => "AST node limit was exceeded",
        PackageValidationError::StyleRuleLimit => "style rule limit was exceeded",
        PackageValidationError::TextBufferByteLimit => "text buffer byte limit was exceeded",
        PackageValidationError::TextByteLimit => "aggregate text byte limit was exceeded",
        PackageValidationError::NonCanonicalFootnoteOrder => "footnote order is not canonical",
        PackageValidationError::MissingEntrySource => "entry source is missing",
        PackageValidationError::IncludeGraphMismatch => "include graph does not match sources",
        PackageValidationError::UnresolvedIncludeDirective => {
            "source contains an unresolved include directive"
        }
    }
}

impl std::fmt::Display for MachineParseFailureKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.canonical_message())
    }
}

/// Exactly one primary location for a machine syntax failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachineParsePrimaryLocation {
    PackageJson(JsonPointer),
    Source(SourceSpan),
}

/// Pointer-aware semantic failure. A source primary always carries exactly one
/// package JSON note; package-primary failures never carry a second primary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineParseFailure {
    kind: MachineParseFailureKind,
    primary: MachineParsePrimaryLocation,
    package_note: Option<JsonPointer>,
    subject: Option<DiagnosticSubject>,
}

impl MachineParseFailure {
    fn package(kind: MachineParseFailureKind, pointer: JsonPointer) -> Self {
        Self {
            kind,
            primary: MachineParsePrimaryLocation::PackageJson(pointer),
            package_note: None,
            subject: None,
        }
    }

    fn package_with_subject(
        kind: MachineParseFailureKind,
        pointer: JsonPointer,
        subject: DiagnosticSubject,
    ) -> Self {
        Self {
            kind,
            primary: MachineParsePrimaryLocation::PackageJson(pointer),
            package_note: None,
            subject: Some(subject),
        }
    }

    fn with_subject(mut self, subject: DiagnosticSubject) -> Self {
        self.subject = Some(subject);
        self
    }

    fn source(kind: MachineParseFailureKind, span: SourceSpan, package: JsonPointer) -> Self {
        Self {
            kind,
            primary: MachineParsePrimaryLocation::Source(span),
            package_note: Some(package),
            subject: None,
        }
    }

    pub const fn kind(&self) -> &MachineParseFailureKind {
        &self.kind
    }

    pub const fn primary(&self) -> &MachineParsePrimaryLocation {
        &self.primary
    }

    pub const fn package_note(&self) -> Option<&JsonPointer> {
        self.package_note.as_ref()
    }

    pub const fn subject(&self) -> Option<&DiagnosticSubject> {
        self.subject.as_ref()
    }

    /// Project this typed syntax failure into the structured public model.
    /// The package URI is already a portable logical path; byte offsets remain
    /// absent when the semantic location index only has an RFC 6901 pointer.
    pub fn to_diagnostic(&self, package_uri: &PortablePath) -> Diagnostic {
        let public_error = self.kind.public_error();
        let location = match &self.primary {
            MachineParsePrimaryLocation::PackageJson(pointer) => {
                DiagnosticLocation::package_json(package_uri.clone(), pointer.clone(), None)
            }
            MachineParsePrimaryLocation::Source(span) => DiagnosticLocation::source(
                SourceDiagnosticLocation::new(Some(*span), None, None)
                    .expect("a source primary always supplies a source span"),
            ),
        };
        let mut builder = DiagnosticBuilder::located(
            public_error.code(),
            Severity::Error,
            self.kind.canonical_message(),
            location,
        )
        .expect("static canonical syntax messages are valid");
        if let Some(subject) = self.subject.clone().or_else(|| public_error.subject()) {
            builder = builder.subject(subject);
        }
        if let Some(pointer) = &self.package_note {
            builder = builder
                .located_note(
                    "related package member",
                    DiagnosticLocation::package_json(package_uri.clone(), pointer.clone(), None),
                )
                .expect("static canonical syntax notes are valid");
        }
        builder.build()
    }
}

impl std::fmt::Display for MachineParseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.primary {
            MachineParsePrimaryLocation::PackageJson(pointer) => {
                write!(formatter, "{} at {pointer}", self.kind)
            }
            MachineParsePrimaryLocation::Source(span) => write!(
                formatter,
                "{} at source {} bytes {}..{} (package note {})",
                self.kind,
                span.source_id().get(),
                span.start_byte().get(),
                span.end_byte().get(),
                self.package_note
                    .as_ref()
                    .expect("source failures always have one package note")
            ),
        }
    }
}

impl std::error::Error for MachineParseFailure {}

/// Decoder/admission provenance kept with a syntax-validated package.
#[derive(Debug)]
pub struct ValidatedMachineProvenance {
    admission: MachineInputAdmissionProvenance,
    raw_sha256: RawDocumentPackageSha256,
    canonical_jcs_sha256: CanonicalDocumentPackageJcsSha256,
    locations: JsonLocationIndex,
}

impl ValidatedMachineProvenance {
    pub const fn session_identity(&self) -> &MachineInputSessionIdentity {
        self.admission.session_identity()
    }

    pub const fn admission(&self) -> &MachineInputAdmissionProvenance {
        &self.admission
    }

    pub const fn progress(&self) -> &MachineInputProgress {
        self.admission.progress()
    }

    pub const fn fingerprint(&self) -> MachineInputFingerprint {
        self.admission.fingerprint()
    }

    pub const fn raw_sha256(&self) -> RawDocumentPackageSha256 {
        self.raw_sha256
    }

    pub const fn canonical_jcs_sha256(&self) -> CanonicalDocumentPackageJcsSha256 {
        self.canonical_jcs_sha256
    }

    pub const fn locations(&self) -> &JsonLocationIndex {
        &self.locations
    }
}

/// The only successful machine syntax result. It wraps the existing trusted
/// package rather than introducing a weaker parallel AST.
///
/// Raw wire DTOs and caller-built parsed packages have no promotion API:
///
/// ```compile_fail
/// use typaxis_syntax::ValidatedMachinePackage;
/// let _ = ValidatedMachinePackage::from_wire;
/// ```
///
/// ```compile_fail
/// use typaxis_syntax::ValidatedMachinePackage;
/// let _ = ValidatedMachinePackage::from_parsed_package;
/// ```
#[derive(Debug)]
pub struct ValidatedMachinePackage {
    package: ValidatedParsedPackage,
    advanced: Option<ValidatedStagingAdvancedPackage>,
    provenance: ValidatedMachineProvenance,
}

impl ValidatedMachinePackage {
    pub const fn package(&self) -> &ValidatedParsedPackage {
        &self.package
    }

    pub const fn provenance(&self) -> &ValidatedMachineProvenance {
        &self.provenance
    }

    /// Contract-1.3 facts issued by the ordinary syntax owner. Older raw
    /// contracts have no advanced view and are never upgraded implicitly.
    pub const fn advanced_view(&self) -> Option<&ValidatedStagingAdvancedPackage> {
        self.advanced.as_ref()
    }

    /// Raw contract selected by the strict decoder. Compatibility input keeps
    /// this identity even though every newly generated artifact uses 1.2.
    pub fn contract(&self) -> typaxis_core::DocumentPackageContractId {
        self.provenance
            .progress()
            .decoded()
            .expect("validated machine packages retain decoded PACKAGE facts")
            .contract()
    }

    /// Issue the syntax-owned view consumed by the immutable basic-document
    /// slices. Raw 1.3 can cross only after its extension has been validated;
    /// profile preflight separately requires its exact neutral encoding.
    pub fn basic_document_view(&self) -> Option<ValidatedBasicDocumentPackage> {
        matches!(
            self.contract(),
            typaxis_core::DocumentPackageContractId::V1_2
                | typaxis_core::DocumentPackageContractId::V1_3
        )
        .then(|| ValidatedStagingStylePackage {
            package: self.package.clone(),
            raw_sha256: self.provenance.raw_sha256,
            canonical_jcs_sha256: self.provenance.canonical_jcs_sha256,
            locations: self.provenance.locations.clone(),
        })
    }
}

#[derive(Debug)]
pub enum MachineParseOutcome {
    Parsed {
        package: Box<ValidatedMachinePackage>,
    },
    Failed {
        progress: Box<MachineInputProgress>,
        failure: MachineParseFailure,
    },
}

/// Syntax-owner parser for decoder- and host-admission-issued machine input.
///
/// The entry-only graph constructor remains private to this crate:
///
/// ```compile_fail
/// use typaxis_syntax::ValidatedIncludeGraph;
/// let _ = ValidatedIncludeGraph::entry_only;
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DocumentPackageParser;

impl DocumentPackageParser {
    pub const fn new() -> Self {
        Self
    }

    pub fn parse(
        &self,
        input: AdmittedMachinePackage,
        policy: &PackageValidationPolicy<'_>,
    ) -> MachineParseOutcome {
        let (decoded, sources, admission) = input.into_parts();
        match lower_machine_package(decoded, sources, &admission, policy) {
            Ok((package, advanced, raw_sha256, canonical_jcs_sha256, locations)) => {
                MachineParseOutcome::Parsed {
                    package: Box::new(ValidatedMachinePackage {
                        package,
                        advanced,
                        provenance: ValidatedMachineProvenance {
                            admission,
                            raw_sha256,
                            canonical_jcs_sha256,
                            locations,
                        },
                    }),
                }
            }
            Err(failure) => MachineParseOutcome::Failed {
                progress: Box::new(admission.into_failure_progress()),
                failure,
            },
        }
    }
}

/// Historical algorithm names retained for the focused contract 1.2 slice
/// receipts. Public orchestration obtains the same sealed facts from the
/// validated basic-document view of a normal machine-package receipt.
pub const STAGING_BASIC_LIST_POLICY_VERSION: &str = "typaxis.basic-list-policy/1";
pub const STAGING_LIST_MARKER_USAGE_ALGORITHM: &str = "typaxis.basic-list-marker-usage/1";
pub const STAGING_FORCED_PAGE_BREAK_POLICY_VERSION: &str =
    "typaxis.basic-forced-page-break-policy/1";
pub const STAGING_FORCED_PAGE_BREAK_USAGE_ALGORITHM: &str =
    "typaxis.basic-forced-page-break-usage/1";
pub const STAGING_BASIC_FIGURE_POLICY_VERSION: &str = "typaxis.basic-png-figure-policy/1";
pub const STAGING_FIGURE_USAGE_ALGORITHM: &str = "typaxis.basic-png-figure-usage/1";
pub const STAGING_BASIC_LINK_POLICY_VERSION: &str = "typaxis.basic-link-policy/1";
pub const STAGING_LINK_USAGE_ALGORITHM: &str = "typaxis.basic-link-usage/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingLinkPreflightError {
    UnsupportedContainer(NodeId),
    UnsupportedChild(NodeId),
    NestedLink(NodeId),
    EmptyChildren(NodeId),
    UnpaintedChildren(NodeId),
    UnknownInternalTarget(NodeId),
    ArithmeticOverflow,
    AllocationFailure,
}

/// Syntax-admitted target for the private link slice. External values have
/// already crossed `SafeUri`; internal values are bound to the package's exact
/// anchor owner rather than carrying an unresolved identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidatedStagingLinkTarget {
    Internal {
        anchor_id: AnchorId,
        anchor_owner: NodeId,
    },
    External(SafeUri),
}

impl ValidatedStagingLinkTarget {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Internal { .. } => "internal",
            Self::External(_) => "external",
        }
    }

    pub const fn internal_anchor_id(&self) -> Option<&AnchorId> {
        match self {
            Self::Internal { anchor_id, .. } => Some(anchor_id),
            Self::External(_) => None,
        }
    }

    pub const fn internal_anchor_owner(&self) -> Option<NodeId> {
        match self {
            Self::Internal { anchor_owner, .. } => Some(*anchor_owner),
            Self::External(_) => None,
        }
    }

    pub const fn external_uri(&self) -> Option<&SafeUri> {
        match self {
            Self::Internal { .. } => None,
            Self::External(uri) => Some(uri),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingAnchor {
    anchor_id: AnchorId,
    owner: NodeId,
}

impl ValidatedStagingAnchor {
    pub const fn anchor_id(&self) -> &AnchorId {
        &self.anchor_id
    }

    pub const fn owner(&self) -> NodeId {
        self.owner
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingLink {
    owner: NodeId,
    paragraph_owner: NodeId,
    target: ValidatedStagingLinkTarget,
    painted_site_owners: Vec<NodeId>,
}

impl ValidatedStagingLink {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn paragraph_owner(&self) -> NodeId {
        self.paragraph_owner
    }

    pub const fn target(&self) -> &ValidatedStagingLinkTarget {
        &self.target
    }

    pub fn painted_site_owners(&self) -> &[NodeId] {
        &self.painted_site_owners
    }
}

#[derive(Debug)]
struct StagingLinkUsageBinding;

/// Syntax-issued closure over every accepted link and every package anchor.
/// The receipt is intentionally package-bound so an otherwise-valid anchor
/// registry from another package cannot be substituted after preflight.
#[derive(Debug)]
pub struct ValidatedStagingLinkUsageReceipt {
    package: CanonicalDocumentPackageJcsSha256,
    anchors: Vec<ValidatedStagingAnchor>,
    links: Vec<ValidatedStagingLink>,
    usage_sha256: [u8; 32],
    _binding: StagingLinkUsageBinding,
}

impl ValidatedStagingLinkUsageReceipt {
    pub const fn package_fingerprint(&self) -> CanonicalDocumentPackageJcsSha256 {
        self.package
    }

    pub const fn policy_version(&self) -> &'static str {
        STAGING_BASIC_LINK_POLICY_VERSION
    }

    pub fn anchors(&self) -> &[ValidatedStagingAnchor] {
        &self.anchors
    }

    pub fn links(&self) -> &[ValidatedStagingLink] {
        &self.links
    }

    pub const fn usage_sha256(&self) -> [u8; 32] {
        self.usage_sha256
    }

    pub fn verifies(&self, package: &ValidatedStagingStylePackage) -> bool {
        self.package == package.package_fingerprint()
            && self.policy_version() == STAGING_BASIC_LINK_POLICY_VERSION
    }
}

fn staging_link_usage_fingerprint(
    anchors: &[ValidatedStagingAnchor],
    links: &[ValidatedStagingLink],
) -> [u8; 32] {
    let mut jcs = String::from("{\"algorithm\":");
    push_jcs_string(&mut jcs, STAGING_LINK_USAGE_ALGORITHM);
    jcs.push_str(",\"anchors\":[");
    for (index, anchor) in anchors.iter().enumerate() {
        if index > 0 {
            jcs.push(',');
        }
        jcs.push_str("{\"anchor_id\":");
        push_jcs_string(&mut jcs, anchor.anchor_id.as_str());
        jcs.push_str(",\"owner_node_id\":");
        jcs.push_str(&anchor.owner.get().to_string());
        jcs.push('}');
    }
    jcs.push_str("],\"links\":[");
    for (index, link) in links.iter().enumerate() {
        if index > 0 {
            jcs.push(',');
        }
        jcs.push_str("{\"link_node_id\":");
        jcs.push_str(&link.owner.get().to_string());
        jcs.push_str(",\"painted_site_owners\":[");
        for (site_index, owner) in link.painted_site_owners.iter().enumerate() {
            if site_index > 0 {
                jcs.push(',');
            }
            jcs.push_str(&owner.get().to_string());
        }
        jcs.push_str("],\"paragraph_node_id\":");
        jcs.push_str(&link.paragraph_owner.get().to_string());
        jcs.push_str(",\"target\":");
        match &link.target {
            ValidatedStagingLinkTarget::Internal {
                anchor_id,
                anchor_owner,
            } => {
                jcs.push_str("{\"anchor_id\":");
                push_jcs_string(&mut jcs, anchor_id.as_str());
                jcs.push_str(",\"anchor_owner_node_id\":");
                jcs.push_str(&anchor_owner.get().to_string());
                jcs.push_str(",\"kind\":\"internal\"}");
            }
            ValidatedStagingLinkTarget::External(uri) => {
                jcs.push_str("{\"kind\":\"external\",\"uri\":");
                push_jcs_string(&mut jcs, uri.as_str());
                jcs.push('}');
            }
        }
        jcs.push('}');
    }
    jcs.push_str("]}");
    sha256(jcs.as_bytes())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingFigurePreflightError {
    UnsupportedContainer(NodeId),
    UnsupportedCaptionBlock(NodeId),
    ArithmeticOverflow,
    AllocationFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingFigure {
    owner: NodeId,
    document_ordinal: u32,
    image_id: ImageResourceId,
    alt: String,
    caption_owners: Vec<NodeId>,
}

impl ValidatedStagingFigure {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn document_ordinal(&self) -> u32 {
        self.document_ordinal
    }

    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }

    pub fn alt(&self) -> &str {
        &self.alt
    }

    pub fn caption_owners(&self) -> &[NodeId] {
        &self.caption_owners
    }
}

#[derive(Debug)]
struct StagingFigureUsageBinding;

/// Syntax-issued proof of the complete Figure set accepted by the private
/// PNG slice. The current closed policy admits non-floating figures only in
/// the document body and paragraph/heading blocks only in caption subflows.
#[derive(Debug)]
pub struct ValidatedStagingFigureUsageReceipt {
    package: CanonicalDocumentPackageJcsSha256,
    figures: Vec<ValidatedStagingFigure>,
    usage_sha256: [u8; 32],
    _binding: StagingFigureUsageBinding,
}

impl ValidatedStagingFigureUsageReceipt {
    pub const fn package_fingerprint(&self) -> CanonicalDocumentPackageJcsSha256 {
        self.package
    }

    pub const fn policy_version(&self) -> &'static str {
        STAGING_BASIC_FIGURE_POLICY_VERSION
    }

    pub fn figures(&self) -> &[ValidatedStagingFigure] {
        &self.figures
    }

    pub const fn usage_sha256(&self) -> [u8; 32] {
        self.usage_sha256
    }

    pub fn verifies(&self, package: &ValidatedStagingStylePackage) -> bool {
        self.package == package.package_fingerprint()
            && self.policy_version() == STAGING_BASIC_FIGURE_POLICY_VERSION
    }
}

fn staging_figure_usage_fingerprint(figures: &[ValidatedStagingFigure]) -> [u8; 32] {
    let mut jcs = String::from("{\"algorithm\":");
    push_jcs_string(&mut jcs, STAGING_FIGURE_USAGE_ALGORITHM);
    jcs.push_str(",\"figures\":[");
    for (index, figure) in figures.iter().enumerate() {
        if index > 0 {
            jcs.push(',');
        }
        jcs.push_str("{\"alt\":");
        push_jcs_string(&mut jcs, &figure.alt);
        jcs.push_str(",\"caption_owners\":[");
        for (caption_index, owner) in figure.caption_owners.iter().enumerate() {
            if caption_index > 0 {
                jcs.push(',');
            }
            jcs.push_str(&owner.get().to_string());
        }
        jcs.push_str("],\"document_ordinal\":");
        jcs.push_str(&figure.document_ordinal.to_string());
        jcs.push_str(",\"figure_node_id\":");
        jcs.push_str(&figure.owner.get().to_string());
        jcs.push_str(",\"image_id\":");
        jcs.push_str(&figure.image_id.get().to_string());
        jcs.push('}');
    }
    jcs.push_str("]}");
    sha256(jcs.as_bytes())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingForcedPageBreakPreflightError {
    UnsupportedContainer(NodeId),
    ArithmeticOverflow,
    AllocationFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingForcedPageBreak {
    owner: NodeId,
    document_ordinal: u32,
}

impl ValidatedStagingForcedPageBreak {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn document_ordinal(&self) -> u32 {
        self.document_ordinal
    }
}

#[derive(Debug)]
struct StagingForcedPageBreakUsageBinding;

/// Syntax-issued proof of the complete, canonical forced-boundary set for a
/// staging package. A page break is represented only as a typed owner; it has
/// no size, fragment, or paint payload that could be confused with content.
#[derive(Debug)]
pub struct ValidatedStagingForcedPageBreakUsageReceipt {
    package: CanonicalDocumentPackageJcsSha256,
    breaks: Vec<ValidatedStagingForcedPageBreak>,
    usage_sha256: [u8; 32],
    _binding: StagingForcedPageBreakUsageBinding,
}

impl ValidatedStagingForcedPageBreakUsageReceipt {
    pub const fn package_fingerprint(&self) -> CanonicalDocumentPackageJcsSha256 {
        self.package
    }

    pub const fn policy_version(&self) -> &'static str {
        STAGING_FORCED_PAGE_BREAK_POLICY_VERSION
    }

    pub fn breaks(&self) -> &[ValidatedStagingForcedPageBreak] {
        &self.breaks
    }

    pub const fn usage_sha256(&self) -> [u8; 32] {
        self.usage_sha256
    }

    pub fn verifies(&self, package: &ValidatedStagingStylePackage) -> bool {
        self.package == package.package_fingerprint()
            && self.policy_version() == STAGING_FORCED_PAGE_BREAK_POLICY_VERSION
    }
}

fn staging_forced_page_break_usage_fingerprint(
    breaks: &[ValidatedStagingForcedPageBreak],
) -> [u8; 32] {
    let mut jcs = String::from("{\"algorithm\":");
    push_jcs_string(&mut jcs, STAGING_FORCED_PAGE_BREAK_USAGE_ALGORITHM);
    jcs.push_str(",\"breaks\":[");
    for (index, boundary) in breaks.iter().enumerate() {
        if index > 0 {
            jcs.push(',');
        }
        jcs.push_str("{\"document_ordinal\":");
        jcs.push_str(&boundary.document_ordinal.to_string());
        jcs.push_str(",\"owner_node_id\":");
        jcs.push_str(&boundary.owner.get().to_string());
        jcs.push('}');
    }
    jcs.push_str("]}");
    sha256(jcs.as_bytes())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingListMarker {
    list_owner: NodeId,
    item_owner: NodeId,
    item_index: u32,
    ordered: bool,
    ordered_value: Option<u32>,
    key: GeneratedBufferKey,
    utf8: String,
}

impl ValidatedStagingListMarker {
    pub const fn list_owner(&self) -> NodeId {
        self.list_owner
    }
    pub const fn item_owner(&self) -> NodeId {
        self.item_owner
    }
    pub const fn item_index(&self) -> u32 {
        self.item_index
    }
    pub const fn is_ordered(&self) -> bool {
        self.ordered
    }
    pub const fn ordered_value(&self) -> Option<u32> {
        self.ordered_value
    }
    pub const fn key(&self) -> GeneratedBufferKey {
        self.key
    }
    pub fn utf8(&self) -> &str {
        &self.utf8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingListMarkerPreflightError {
    MarkerOverflow { list_owner: NodeId },
    MissingMarkerTextStyle { list_owner: NodeId },
    TextBufferLimit { item_owner: NodeId },
    TextTotalLimit,
    ArithmeticOverflow,
    AllocationFailure,
}

#[derive(Debug)]
struct StagingListMarkerUsageBinding;

/// Syntax-issued proof that canonical list-marker bytes and their complete
/// parsed-plus-generated budget were checked before marker string allocation.
#[derive(Debug)]
pub struct ValidatedStagingListMarkerUsageReceipt {
    package: CanonicalDocumentPackageJcsSha256,
    markers: Vec<ValidatedStagingListMarker>,
    marker_usage_sha256: [u8; 32],
    parsed_text_bytes: u64,
    generated_marker_bytes: u64,
    max_text_buffer_bytes: u32,
    max_text_bytes: u64,
    _binding: StagingListMarkerUsageBinding,
}

impl ValidatedStagingListMarkerUsageReceipt {
    pub const fn package_fingerprint(&self) -> CanonicalDocumentPackageJcsSha256 {
        self.package
    }
    pub const fn policy_version(&self) -> &'static str {
        STAGING_BASIC_LIST_POLICY_VERSION
    }
    pub fn markers(&self) -> &[ValidatedStagingListMarker] {
        &self.markers
    }
    pub const fn marker_usage_sha256(&self) -> [u8; 32] {
        self.marker_usage_sha256
    }
    pub const fn parsed_text_bytes(&self) -> u64 {
        self.parsed_text_bytes
    }
    pub const fn generated_marker_bytes(&self) -> u64 {
        self.generated_marker_bytes
    }
    pub const fn max_text_buffer_bytes(&self) -> u32 {
        self.max_text_buffer_bytes
    }
    pub const fn max_text_bytes(&self) -> u64 {
        self.max_text_bytes
    }
    pub fn verifies(&self, package: &ValidatedStagingStylePackage) -> bool {
        self.package == package.package_fingerprint()
            && self.policy_version() == STAGING_BASIC_LIST_POLICY_VERSION
    }
    pub fn verifies_generated_text(&self, generated: PackageGeneratedTextBinding<'_>) -> bool {
        if generated.generated_text().document_nodes() != generated.package().document_nodes() {
            return false;
        }
        let observed: Vec<_> = generated
            .generated_text()
            .buffers()
            .iter()
            .filter(|buffer| buffer.key().generation_kind() == GenerationKind::ListMarker)
            .map(|buffer| (buffer.key(), buffer.utf8()))
            .collect();
        observed.len() == self.markers.len()
            && self
                .markers
                .iter()
                .zip(observed)
                .all(|(expected, actual)| expected.key == actual.0 && expected.utf8 == actual.1)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingStagingListMarker {
    list_owner: NodeId,
    item_owner: NodeId,
    item_index: u32,
    ordered: bool,
    ordered_value: Option<u32>,
}

const fn staging_decimal_digits(mut value: u32) -> u32 {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}

fn staging_list_marker_usage_fingerprint(markers: &[ValidatedStagingListMarker]) -> [u8; 32] {
    let mut jcs = String::from("{\"algorithm\":");
    push_jcs_string(&mut jcs, STAGING_LIST_MARKER_USAGE_ALGORITHM);
    jcs.push_str(",\"markers\":[");
    for (index, marker) in markers.iter().enumerate() {
        if index > 0 {
            jcs.push(',');
        }
        jcs.push_str("{\"key\":");
        push_generated_buffer_key_jcs(&mut jcs, marker.key);
        jcs.push_str(",\"utf8\":");
        push_jcs_string(&mut jcs, &marker.utf8);
        jcs.push('}');
    }
    jcs.push_str("]}");
    sha256(jcs.as_bytes())
}

#[derive(Debug)]
pub struct ValidatedStagingStylePackage {
    package: ValidatedParsedPackage,
    raw_sha256: RawDocumentPackageSha256,
    canonical_jcs_sha256: CanonicalDocumentPackageJcsSha256,
    locations: JsonLocationIndex,
}

/// Public 1.2 name for the syntax-owned basic-document view. The staging name
/// remains as a source-compatible alias for the frozen MI2-02..07 tests.
pub type ValidatedBasicDocumentPackage = ValidatedStagingStylePackage;

impl ValidatedStagingStylePackage {
    pub const fn package(&self) -> &ValidatedParsedPackage {
        &self.package
    }

    pub const fn raw_sha256(&self) -> RawDocumentPackageSha256 {
        self.raw_sha256
    }

    pub const fn package_fingerprint(&self) -> CanonicalDocumentPackageJcsSha256 {
        self.canonical_jcs_sha256
    }

    pub const fn locations(&self) -> &JsonLocationIndex {
        &self.locations
    }

    /// Preflights the private basic-document link domain before itemization or
    /// layout begins. The resulting target values are either package-local
    /// anchor bindings or syntax-admitted `SafeUri` values; no raw URI string
    /// survives this boundary.
    pub fn preflight_link_usage(
        &self,
    ) -> Result<ValidatedStagingLinkUsageReceipt, StagingLinkPreflightError> {
        self.preflight_link_usage_internal(false)
    }

    /// Footnote-profile variant: definition paragraphs/headings are an
    /// admitted painted container, while table-cell links remain outside this
    /// profile's composition boundary.
    pub fn preflight_footnote_link_usage(
        &self,
    ) -> Result<ValidatedStagingLinkUsageReceipt, StagingLinkPreflightError> {
        self.preflight_link_usage_internal(true)
    }

    fn preflight_link_usage_internal(
        &self,
        accept_footnote_links: bool,
    ) -> Result<ValidatedStagingLinkUsageReceipt, StagingLinkPreflightError> {
        let nodes = self.package.document_nodes();
        let mut anchors = Vec::new();
        anchors
            .try_reserve_exact(nodes.anchors().len())
            .map_err(|_| StagingLinkPreflightError::AllocationFailure)?;
        anchors.extend(
            nodes
                .anchors()
                .map(|(anchor_id, owner)| ValidatedStagingAnchor {
                    anchor_id: anchor_id.clone(),
                    owner,
                }),
        );

        let mut links = Vec::new();
        collect_staging_links_from_blocks(
            &self.package.package().document.blocks,
            true,
            nodes,
            &mut links,
        )?;
        for footnote in &self.package.package().document.footnotes {
            collect_staging_links_from_blocks(
                &footnote.blocks,
                accept_footnote_links,
                nodes,
                &mut links,
            )?;
        }
        links.sort_by_key(ValidatedStagingLink::owner);
        if links.windows(2).any(|pair| pair[0].owner == pair[1].owner) {
            return Err(StagingLinkPreflightError::ArithmeticOverflow);
        }
        let usage_sha256 = staging_link_usage_fingerprint(&anchors, &links);
        Ok(ValidatedStagingLinkUsageReceipt {
            package: self.canonical_jcs_sha256,
            anchors,
            links,
            usage_sha256,
            _binding: StagingLinkUsageBinding,
        })
    }

    /// Preflights the closed non-floating Figure domain. Resource bytes do not
    /// participate here: media kind is attested later by resource admission,
    /// never inferred from the declaration URI or another caller string.
    pub fn preflight_figure_usage(
        &self,
    ) -> Result<ValidatedStagingFigureUsageReceipt, StagingFigurePreflightError> {
        let mut pending: Vec<(&Block, bool)> = self
            .package
            .package()
            .document
            .blocks
            .iter()
            .rev()
            .map(|block| (block, true))
            .collect();
        let mut figures = Vec::new();
        while let Some((block, document_body)) = pending.pop() {
            match block {
                Block::Figure {
                    node_id,
                    image_id,
                    alt,
                    caption,
                    ..
                } => {
                    if !document_body {
                        return Err(StagingFigurePreflightError::UnsupportedContainer(*node_id));
                    }
                    let document_ordinal = u32::try_from(figures.len())
                        .map_err(|_| StagingFigurePreflightError::ArithmeticOverflow)?;
                    let mut caption_owners = Vec::new();
                    caption_owners
                        .try_reserve_exact(caption.len())
                        .map_err(|_| StagingFigurePreflightError::AllocationFailure)?;
                    for caption_block in caption {
                        let owner = match caption_block {
                            Block::Paragraph { node_id, .. } | Block::Heading { node_id, .. } => {
                                *node_id
                            }
                            Block::List { node_id, .. }
                            | Block::Table { node_id, .. }
                            | Block::Figure { node_id, .. }
                            | Block::PageBreak { node_id, .. } => {
                                return Err(StagingFigurePreflightError::UnsupportedCaptionBlock(
                                    *node_id,
                                ))
                            }
                        };
                        caption_owners.push(owner);
                    }
                    figures
                        .try_reserve(1)
                        .map_err(|_| StagingFigurePreflightError::AllocationFailure)?;
                    figures.push(ValidatedStagingFigure {
                        owner: *node_id,
                        document_ordinal,
                        image_id: *image_id,
                        alt: alt.clone(),
                        caption_owners,
                    });
                }
                Block::List { items, .. } => {
                    let additional = items.iter().try_fold(0usize, |total, item| {
                        total
                            .checked_add(item.blocks.len())
                            .ok_or(StagingFigurePreflightError::ArithmeticOverflow)
                    })?;
                    pending
                        .try_reserve(additional)
                        .map_err(|_| StagingFigurePreflightError::AllocationFailure)?;
                    pending.extend(
                        items
                            .iter()
                            .rev()
                            .flat_map(|item| item.blocks.iter().rev())
                            .map(|block| (block, false)),
                    );
                }
                Block::Table { head, body, .. } => {
                    let additional = head
                        .iter()
                        .chain(body)
                        .flat_map(|row| &row.cells)
                        .flat_map(|cell| &cell.blocks)
                        .try_fold(0usize, |total, _| {
                            total
                                .checked_add(1)
                                .ok_or(StagingFigurePreflightError::ArithmeticOverflow)
                        })?;
                    pending
                        .try_reserve(additional)
                        .map_err(|_| StagingFigurePreflightError::AllocationFailure)?;
                    pending.extend(
                        body.iter()
                            .rev()
                            .chain(head.iter().rev())
                            .flat_map(|row| row.cells.iter().rev())
                            .flat_map(|cell| cell.blocks.iter().rev())
                            .map(|block| (block, false)),
                    );
                }
                Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
            }
        }
        let usage_sha256 = staging_figure_usage_fingerprint(&figures);
        Ok(ValidatedStagingFigureUsageReceipt {
            package: self.canonical_jcs_sha256,
            figures,
            usage_sha256,
            _binding: StagingFigureUsageBinding,
        })
    }

    /// Preflight the complete forced-break domain accepted by the private M2
    /// profile. Breaks are accepted in the document body and list-item flows;
    /// figure captions, tables, and other later domains remain closed.
    pub fn preflight_forced_page_break_usage(
        &self,
    ) -> Result<ValidatedStagingForcedPageBreakUsageReceipt, StagingForcedPageBreakPreflightError>
    {
        let mut pending: Vec<(&Block, bool)> = self
            .package
            .package()
            .document
            .blocks
            .iter()
            .rev()
            .map(|block| (block, true))
            .collect();
        let mut breaks = Vec::new();
        while let Some((block, accepted_container)) = pending.pop() {
            match block {
                Block::PageBreak { node_id, .. } => {
                    if !accepted_container {
                        return Err(StagingForcedPageBreakPreflightError::UnsupportedContainer(
                            *node_id,
                        ));
                    }
                    let document_ordinal = u32::try_from(breaks.len())
                        .map_err(|_| StagingForcedPageBreakPreflightError::ArithmeticOverflow)?;
                    breaks
                        .try_reserve(1)
                        .map_err(|_| StagingForcedPageBreakPreflightError::AllocationFailure)?;
                    breaks.push(ValidatedStagingForcedPageBreak {
                        owner: *node_id,
                        document_ordinal,
                    });
                }
                Block::List { items, .. } => {
                    let additional = items.iter().try_fold(0usize, |total, item| {
                        total
                            .checked_add(item.blocks.len())
                            .ok_or(StagingForcedPageBreakPreflightError::ArithmeticOverflow)
                    })?;
                    pending
                        .try_reserve(additional)
                        .map_err(|_| StagingForcedPageBreakPreflightError::AllocationFailure)?;
                    pending.extend(
                        items
                            .iter()
                            .rev()
                            .flat_map(|item| item.blocks.iter().rev())
                            .map(|block| (block, accepted_container)),
                    );
                }
                Block::Figure { caption, .. } => {
                    pending
                        .try_reserve(caption.len())
                        .map_err(|_| StagingForcedPageBreakPreflightError::AllocationFailure)?;
                    pending.extend(caption.iter().rev().map(|block| (block, false)));
                }
                Block::Table { head, body, .. } => {
                    let nested = body
                        .iter()
                        .chain(head)
                        .flat_map(|row| &row.cells)
                        .flat_map(|cell| &cell.blocks);
                    pending.extend(nested.map(|block| (block, false)));
                }
                Block::Paragraph { .. } | Block::Heading { .. } => {}
            }
        }
        let usage_sha256 = staging_forced_page_break_usage_fingerprint(&breaks);
        Ok(ValidatedStagingForcedPageBreakUsageReceipt {
            package: self.canonical_jcs_sha256,
            breaks,
            usage_sha256,
            _binding: StagingForcedPageBreakUsageBinding,
        })
    }

    pub fn compute_block_style(
        &self,
        owner: NodeId,
        flow_parent: Option<&MachineBlockComputedStyleReceipt>,
    ) -> Result<MachineBlockComputedStyleReceipt, StagingStyleReceiptMismatch> {
        if let Some(parent) = flow_parent {
            if parent.package != self.canonical_jcs_sha256
                || parent.document != self.package.epoch_identity.document()
                || parent.style != self.package.epoch_identity.style()
                || parent.registry_version != BASIC_BLOCK_STYLE_REGISTRY_VERSION
            {
                return Err(StagingStyleReceiptMismatch::ParentReceiptMismatch);
            }
        }
        let (style_owner, block_type, classes, expected_parent) =
            find_basic_styleable_block(&self.package.package.document, owner)
                .ok_or(StagingStyleReceiptMismatch::UnknownStyleOwner)?;
        if expected_parent != flow_parent.map(MachineBlockComputedStyleReceipt::owner) {
            return Err(StagingStyleReceiptMismatch::ParentReceiptMismatch);
        }
        let block = BasicStyleBlockKind::from_str(block_type)
            .ok_or(StagingStyleReceiptMismatch::UnsupportedBlockKind)?;
        let computed = self
            .package
            .package
            .style_sheet
            .cascade_basic_document_style(block, classes, flow_parent.map(|value| &value.computed))
            .map_err(StagingStyleReceiptMismatch::InvalidStyle)?;
        Ok(MachineBlockComputedStyleReceipt {
            owner,
            style_owner,
            package: self.canonical_jcs_sha256,
            document: self.package.epoch_identity.document(),
            style: self.package.epoch_identity.style(),
            registry_version: BASIC_BLOCK_STYLE_REGISTRY_VERSION,
            block,
            computed,
            _binding: StagingStyleBinding,
        })
    }

    /// Issues the private `table-1` typed placement style for an actual,
    /// direct table owner. This does not make `table` a basic-document block
    /// kind and exposes no raw declaration map to layout.
    pub fn compute_table_style(
        &self,
        owner: NodeId,
    ) -> Result<MachineTableComputedStyleReceipt, StagingStyleReceiptMismatch> {
        let (style_owner, block_type, classes, expected_parent) =
            find_basic_styleable_block(&self.package.package.document, owner)
                .ok_or(StagingStyleReceiptMismatch::UnknownStyleOwner)?;
        if style_owner != owner || block_type != "table" || expected_parent.is_some() {
            return Err(StagingStyleReceiptMismatch::UnsupportedBlockKind);
        }
        let computed = self
            .package
            .package
            .style_sheet
            .cascade_table_document_style(classes)
            .map_err(StagingStyleReceiptMismatch::InvalidStyle)?;
        Ok(MachineTableComputedStyleReceipt {
            owner,
            package: self.canonical_jcs_sha256,
            document: self.package.epoch_identity.document(),
            style: self.package.epoch_identity.style(),
            registry_version: TABLE_BLOCK_STYLE_REGISTRY_VERSION,
            computed,
            _binding: StagingStyleBinding,
        })
    }

    /// Issues the ordinary M2 paragraph receipt for a site inside a validated
    /// table cell, with inheritance bound to the sealed table parent rather
    /// than a caller-selected block receipt.
    pub fn compute_table_cell_block_style(
        &self,
        owner: NodeId,
        table_parent: &MachineTableComputedStyleReceipt,
    ) -> Result<MachineBlockComputedStyleReceipt, StagingStyleReceiptMismatch> {
        if table_parent.package != self.canonical_jcs_sha256
            || table_parent.document != self.package.epoch_identity.document()
            || table_parent.style != self.package.epoch_identity.style()
            || table_parent.registry_version != TABLE_BLOCK_STYLE_REGISTRY_VERSION
        {
            return Err(StagingStyleReceiptMismatch::ParentReceiptMismatch);
        }
        let (style_owner, block_type, classes, expected_parent) =
            find_basic_styleable_block(&self.package.package.document, owner)
                .ok_or(StagingStyleReceiptMismatch::UnknownStyleOwner)?;
        if expected_parent != Some(table_parent.owner) || block_type != "paragraph" {
            return Err(StagingStyleReceiptMismatch::ParentReceiptMismatch);
        }
        let computed = self
            .package
            .package
            .style_sheet
            .cascade_table_cell_paragraph_style(classes, &table_parent.computed)
            .map_err(StagingStyleReceiptMismatch::InvalidStyle)?;
        Ok(MachineBlockComputedStyleReceipt {
            owner,
            style_owner,
            package: self.canonical_jcs_sha256,
            document: self.package.epoch_identity.document(),
            style: self.package.epoch_identity.style(),
            registry_version: BASIC_BLOCK_STYLE_REGISTRY_VERSION,
            block: BasicStyleBlockKind::Paragraph,
            computed,
            _binding: StagingStyleBinding,
        })
    }

    /// Issues the complete list-marker style only for an actual list block.
    /// List-item owners cannot borrow their parent's style receipt as a
    /// caller-selected marker context.
    pub fn compute_list_style(
        &self,
        owner: NodeId,
    ) -> Result<MachineListComputedStyleReceipt, StagingStyleReceiptMismatch> {
        let (style_owner, block_type, classes, _) =
            find_basic_styleable_block(&self.package.package.document, owner)
                .ok_or(StagingStyleReceiptMismatch::UnknownStyleOwner)?;
        if style_owner != owner
            || BasicStyleBlockKind::from_str(block_type) != Some(BasicStyleBlockKind::List)
        {
            return Err(StagingStyleReceiptMismatch::UnsupportedBlockKind);
        }
        let computed = self
            .package
            .package
            .style_sheet
            .cascade_basic_document_list_style(classes)
            .map_err(StagingStyleReceiptMismatch::InvalidStyle)?;
        Ok(MachineListComputedStyleReceipt {
            owner,
            package: self.canonical_jcs_sha256,
            document: self.package.epoch_identity.document(),
            style: self.package.epoch_identity.style(),
            registry_version: BASIC_BLOCK_STYLE_REGISTRY_VERSION,
            computed,
            _binding: StagingStyleBinding,
        })
    }

    /// Preflights the exact canonical marker overlay for the basic-document list
    /// policy. The byte budget is complete before decimal/bullet strings are
    /// allocated, and the resulting receipt is package-bound and sealed.
    pub fn preflight_list_marker_usage(
        &self,
        limits: &ValidatedResourceLimits,
    ) -> Result<ValidatedStagingListMarkerUsageReceipt, StagingListMarkerPreflightError> {
        let parsed = self.package.package();
        let parsed_text_bytes = parsed
            .text_store
            .buffers()
            .iter()
            .try_fold(0u64, |total, buffer| {
                total.checked_add(u64::from(buffer.byte_len()))
            })
            .ok_or(StagingListMarkerPreflightError::ArithmeticOverflow)?;

        let mut pending_blocks: Vec<&Block> = parsed.document.blocks.iter().rev().collect();
        let mut pending_markers = Vec::new();
        let mut generated_marker_bytes = 0u64;
        while let Some(block) = pending_blocks.pop() {
            match block {
                Block::List {
                    node_id,
                    ordered,
                    start,
                    items,
                    ..
                } => {
                    if self.compute_list_style(*node_id).is_err() {
                        return Err(StagingListMarkerPreflightError::MissingMarkerTextStyle {
                            list_owner: *node_id,
                        });
                    }
                    for (index, item) in items.iter().enumerate() {
                        let item_index = u32::try_from(index).map_err(|_| {
                            StagingListMarkerPreflightError::MarkerOverflow {
                                list_owner: *node_id,
                            }
                        })?;
                        let (ordered_value, byte_len) = if *ordered {
                            let value = start.and_then(|value| value.checked_add(item_index));
                            let Some(value) = value else {
                                return Err(StagingListMarkerPreflightError::MarkerOverflow {
                                    list_owner: *node_id,
                                });
                            };
                            (Some(value), u64::from(staging_decimal_digits(value) + 1))
                        } else {
                            (None, 3)
                        };
                        if byte_len > u64::from(limits.get().max_text_buffer_bytes) {
                            return Err(StagingListMarkerPreflightError::TextBufferLimit {
                                item_owner: item.node_id,
                            });
                        }
                        generated_marker_bytes = generated_marker_bytes
                            .checked_add(byte_len)
                            .ok_or(StagingListMarkerPreflightError::ArithmeticOverflow)?;
                        pending_markers.push(PendingStagingListMarker {
                            list_owner: *node_id,
                            item_owner: item.node_id,
                            item_index,
                            ordered: *ordered,
                            ordered_value,
                        });
                    }
                    pending_blocks
                        .extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
                }
                Block::Figure { caption, .. } => pending_blocks.extend(caption.iter().rev()),
                Block::Table { head, body, .. } => pending_blocks.extend(
                    body.iter()
                        .rev()
                        .chain(head.iter().rev())
                        .flat_map(|row| row.cells.iter().rev())
                        .flat_map(|cell| cell.blocks.iter().rev()),
                ),
                Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
            }
        }

        if parsed_text_bytes
            .checked_add(generated_marker_bytes)
            .ok_or(StagingListMarkerPreflightError::ArithmeticOverflow)?
            > limits.get().max_text_bytes
        {
            return Err(StagingListMarkerPreflightError::TextTotalLimit);
        }

        pending_markers.sort_by_key(|marker| marker.item_owner);
        let mut markers = Vec::new();
        markers
            .try_reserve_exact(pending_markers.len())
            .map_err(|_| StagingListMarkerPreflightError::AllocationFailure)?;
        for marker in pending_markers {
            let utf8 = match marker.ordered_value {
                Some(value) => format!("{value}."),
                None => "\u{2022}".to_owned(),
            };
            markers.push(ValidatedStagingListMarker {
                list_owner: marker.list_owner,
                item_owner: marker.item_owner,
                item_index: marker.item_index,
                ordered: marker.ordered,
                ordered_value: marker.ordered_value,
                key: GeneratedBufferKey::new(marker.item_owner, GenerationKind::ListMarker, 0),
                utf8,
            });
        }
        let marker_usage_sha256 = staging_list_marker_usage_fingerprint(&markers);
        Ok(ValidatedStagingListMarkerUsageReceipt {
            package: self.canonical_jcs_sha256,
            markers,
            marker_usage_sha256,
            parsed_text_bytes,
            generated_marker_bytes,
            max_text_buffer_bytes: limits.get().max_text_buffer_bytes,
            max_text_bytes: limits.get().max_text_bytes,
            _binding: StagingListMarkerUsageBinding,
        })
    }

    pub fn figure_has_required_width(
        &self,
        owner: NodeId,
    ) -> Result<bool, StagingStyleReceiptMismatch> {
        let (_, block_type, classes, _) =
            find_basic_styleable_block(&self.package.package.document, owner)
                .ok_or(StagingStyleReceiptMismatch::UnknownStyleOwner)?;
        let block = BasicStyleBlockKind::from_str(block_type)
            .ok_or(StagingStyleReceiptMismatch::UnsupportedBlockKind)?;
        if block != BasicStyleBlockKind::Figure {
            return Err(StagingStyleReceiptMismatch::UnsupportedBlockKind);
        }
        let computed = self
            .package
            .package
            .style_sheet
            .cascade_basic_document_style(block, classes, None)
            .map_err(StagingStyleReceiptMismatch::InvalidStyle)?;
        Ok(!matches!(
            computed.width(),
            typaxis_style::MachineFigureWidth::Auto
        ))
    }
}

#[derive(Debug, Eq, PartialEq)]
struct StagingStyleBinding;

/// Sealed computed-value receipt. Layout sees this typed value, never a raw
/// declaration name, JSON scalar, or unchecked fixed-point integer.
#[derive(Debug, Eq, PartialEq)]
pub struct MachineBlockComputedStyleReceipt {
    owner: NodeId,
    style_owner: NodeId,
    package: CanonicalDocumentPackageJcsSha256,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    registry_version: &'static str,
    block: BasicStyleBlockKind,
    computed: ComputedMachineBlockStyle,
    _binding: StagingStyleBinding,
}

/// Sealed `table-1` receipt containing only the M2 typed block-placement
/// values admitted for table selectors.
#[derive(Debug, Eq, PartialEq)]
pub struct MachineTableComputedStyleReceipt {
    owner: NodeId,
    package: CanonicalDocumentPackageJcsSha256,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    registry_version: &'static str,
    computed: ComputedMachineBlockStyle,
    _binding: StagingStyleBinding,
}

/// Sealed package/list-owner binding for generated marker text and placement.
#[derive(Debug, Eq, PartialEq)]
pub struct MachineListComputedStyleReceipt {
    owner: NodeId,
    package: CanonicalDocumentPackageJcsSha256,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    registry_version: &'static str,
    computed: ComputedMachineListStyle,
    _binding: StagingStyleBinding,
}

impl MachineListComputedStyleReceipt {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn package_fingerprint(&self) -> CanonicalDocumentPackageJcsSha256 {
        self.package
    }

    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }

    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }

    pub const fn registry_version(&self) -> &'static str {
        self.registry_version
    }

    pub const fn computed(&self) -> &ComputedMachineListStyle {
        &self.computed
    }
}

impl MachineBlockComputedStyleReceipt {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
    pub const fn package_fingerprint(&self) -> CanonicalDocumentPackageJcsSha256 {
        self.package
    }
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }
    pub const fn registry_version(&self) -> &'static str {
        self.registry_version
    }
    pub const fn block_kind(&self) -> BasicStyleBlockKind {
        self.block
    }
    pub const fn computed(&self) -> ComputedMachineBlockStyle {
        self.computed
    }

    pub const fn has_required_figure_width(&self) -> bool {
        !matches!(
            (self.block, self.computed.width()),
            (
                BasicStyleBlockKind::Figure,
                typaxis_style::MachineFigureWidth::Auto
            )
        )
    }
}

impl MachineTableComputedStyleReceipt {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn package_fingerprint(&self) -> CanonicalDocumentPackageJcsSha256 {
        self.package
    }

    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }

    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }

    pub const fn registry_version(&self) -> &'static str {
        self.registry_version
    }

    pub const fn computed(&self) -> ComputedMachineBlockStyle {
        self.computed
    }
}

fn collect_staging_links_from_blocks(
    blocks: &[Block],
    accepted_container: bool,
    nodes: &ValidatedDocumentNodeIndex,
    links: &mut Vec<ValidatedStagingLink>,
) -> Result<(), StagingLinkPreflightError> {
    for block in blocks {
        match block {
            Block::Paragraph {
                node_id, children, ..
            }
            | Block::Heading {
                node_id, children, ..
            } => collect_staging_links_from_inlines(
                children,
                *node_id,
                accepted_container,
                nodes,
                links,
            )?,
            Block::List { items, .. } => {
                for item in items {
                    collect_staging_links_from_blocks(
                        &item.blocks,
                        accepted_container,
                        nodes,
                        links,
                    )?;
                }
            }
            Block::Figure { caption, .. } => {
                collect_staging_links_from_blocks(caption, accepted_container, nodes, links)?
            }
            Block::Table { head, body, .. } => {
                for row in head.iter().chain(body) {
                    for cell in &row.cells {
                        collect_staging_links_from_blocks(&cell.blocks, false, nodes, links)?;
                    }
                }
            }
            Block::PageBreak { .. } => {}
        }
    }
    Ok(())
}

fn collect_staging_links_from_inlines(
    inlines: &[Inline],
    paragraph_owner: NodeId,
    accepted_container: bool,
    nodes: &ValidatedDocumentNodeIndex,
    links: &mut Vec<ValidatedStagingLink>,
) -> Result<(), StagingLinkPreflightError> {
    for inline in inlines {
        match inline {
            Inline::Link {
                node_id,
                target,
                children,
                ..
            } => {
                if !accepted_container {
                    return Err(StagingLinkPreflightError::UnsupportedContainer(*node_id));
                }
                if children.is_empty() {
                    return Err(StagingLinkPreflightError::EmptyChildren(*node_id));
                }
                let mut painted_site_owners = Vec::new();
                painted_site_owners
                    .try_reserve_exact(children.len())
                    .map_err(|_| StagingLinkPreflightError::AllocationFailure)?;
                for child in children {
                    match child {
                        Inline::Text {
                            node_id, text_span, ..
                        } => {
                            if !text_span.range().is_empty() {
                                painted_site_owners.push(*node_id);
                            }
                        }
                        Inline::Reference {
                            node_id,
                            format: ReferenceFormat::Page,
                            ..
                        } => painted_site_owners.push(*node_id),
                        Inline::Anchor { .. }
                        | Inline::SoftBreak { .. }
                        | Inline::HardBreak { .. } => {}
                        Inline::Link { node_id, .. } => {
                            return Err(StagingLinkPreflightError::NestedLink(*node_id))
                        }
                        Inline::Emphasis { node_id, .. }
                        | Inline::Strong { node_id, .. }
                        | Inline::Reference { node_id, .. }
                        | Inline::FootnoteReference { node_id, .. } => {
                            return Err(StagingLinkPreflightError::UnsupportedChild(*node_id))
                        }
                    }
                }
                if painted_site_owners.is_empty() {
                    return Err(StagingLinkPreflightError::UnpaintedChildren(*node_id));
                }
                let target = match target {
                    LinkTarget::Internal(anchor_id) => {
                        let anchor_owner = nodes
                            .anchor_owner(anchor_id)
                            .ok_or(StagingLinkPreflightError::UnknownInternalTarget(*node_id))?;
                        ValidatedStagingLinkTarget::Internal {
                            anchor_id: anchor_id.clone(),
                            anchor_owner,
                        }
                    }
                    LinkTarget::Uri(uri) => ValidatedStagingLinkTarget::External(uri.clone()),
                };
                links
                    .try_reserve(1)
                    .map_err(|_| StagingLinkPreflightError::AllocationFailure)?;
                links.push(ValidatedStagingLink {
                    owner: *node_id,
                    paragraph_owner,
                    target,
                    painted_site_owners,
                });
            }
            Inline::Emphasis { children, .. } | Inline::Strong { children, .. } => {
                collect_staging_links_from_inlines(children, paragraph_owner, false, nodes, links)?;
            }
            Inline::Text { .. }
            | Inline::Anchor { .. }
            | Inline::Reference { .. }
            | Inline::FootnoteReference { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => {}
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingStyleReceiptMismatch {
    UnknownStyleOwner,
    UnsupportedBlockKind,
    ParentReceiptMismatch,
    InvalidStyle(StyleValidationError),
}

/// Compatibility parser for deterministic in-repository M2 slice fixtures.
/// Host admission and `DocumentPackageParser` remain mandatory for public input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StagingStylePackageParser;

impl StagingStylePackageParser {
    pub const fn new() -> Self {
        Self
    }

    pub fn parse(
        &self,
        decoded: wire::DecodedStagingStyleDocumentPackage,
        source_utf8: String,
        policy: &PackageValidationPolicy<'_>,
    ) -> Result<ValidatedStagingStylePackage, MachineParseFailure> {
        let (wire, raw_sha256, canonical_jcs_sha256, locations) = decoded.into_parts();
        preflight_wire_semantics(&wire, policy.limits, &locations)?;
        let WireDocumentPackage {
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
                    locations.root_member(DocumentPackageRootMember::Sources),
                )
            })?;
        let package = ValidatedParsedPackage::new_resolved_with_style_contract(
            package,
            policy,
            &include_graph,
            true,
            |package, error| machine_validation_failure(package, error, &locations),
        )?;
        Ok(ValidatedStagingStylePackage {
            package,
            raw_sha256,
            canonical_jcs_sha256,
            locations,
        })
    }
}

fn lower_staging_source(
    declarations: Vec<wire::WireSource>,
    source_utf8: String,
    locations: &JsonLocationIndex,
) -> Result<SourceCatalog, MachineParseFailure> {
    if declarations.len() != 1 {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::SourceDeclarationMismatch,
            locations.root_member(DocumentPackageRootMember::Sources),
        ));
    }
    let declaration = declarations.into_iter().next().expect("length was checked");
    let pointer = source_pointer(locations, declaration.source_id, 0);
    let uri = PortablePath::new(declaration.uri).map_err(|_| {
        MachineParseFailure::package(
            MachineParseFailureKind::InvalidSourceCatalog,
            pointer.child("uri"),
        )
    })?;
    let record = SourceRecord::new(SourceId::new(declaration.source_id), uri, source_utf8)
        .map_err(|_| {
            MachineParseFailure::package(
                MachineParseFailureKind::InvalidSourceCatalog,
                pointer.clone(),
            )
        })?;
    if record.utf8_byte_length() != declaration.utf8_byte_length
        || record.content_hash() != declaration.sha256
    {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::SourceBytesMismatch,
            pointer,
        ));
    }
    SourceCatalog::new(vec![record]).map_err(|_| {
        MachineParseFailure::package(
            MachineParseFailureKind::InvalidSourceCatalog,
            locations.root_member(DocumentPackageRootMember::Sources),
        )
    })
}

fn lower_machine_package(
    decoded: wire::DecodedDocumentPackage,
    admitted_sources: Vec<AdmittedMachineSource>,
    admission: &MachineInputAdmissionProvenance,
    policy: &PackageValidationPolicy<'_>,
) -> Result<
    (
        ValidatedParsedPackage,
        Option<ValidatedStagingAdvancedPackage>,
        RawDocumentPackageSha256,
        CanonicalDocumentPackageJcsSha256,
        JsonLocationIndex,
    ),
    MachineParseFailure,
> {
    let (wire, raw_sha256, canonical_jcs_sha256, locations) = decoded.into_parts();
    recheck_machine_admission(
        &wire,
        raw_sha256,
        canonical_jcs_sha256,
        &admitted_sources,
        admission,
        &locations,
    )?;
    preflight_wire_semantics(&wire, policy.limits, &locations)?;

    let extended_style_contract = matches!(
        wire.contract,
        typaxis_core::DocumentPackageContractId::V1_2
            | typaxis_core::DocumentPackageContractId::V1_3
    );
    let WireDocumentPackage {
        contract: _,
        coordinate_unit: _,
        advanced,
        sources,
        text_buffers,
        document,
        style_sheet,
        page_masters,
        resources,
    } = wire;
    let source_catalog = lower_sources(sources, admitted_sources, &locations)?;
    let text_store = lower_text_store(text_buffers, policy.limits, &locations)?;
    let package = ParsedPackage {
        sources: source_catalog,
        text_store,
        document: lower_document(document, policy, &locations)?,
        style_sheet: lower_style_sheet(style_sheet, &locations)?,
        page_masters: lower_page_masters(page_masters, &locations)?,
        resources: lower_resources(resources, &locations)?,
    };

    // Machine profile M1 has exactly one admitted entry source. No producer
    // source keyword scan participates in this closure proof.
    let include_graph = ValidatedIncludeGraph::entry_only(&package.sources, policy.limits)
        .map_err(|_| {
            MachineParseFailure::package(
                MachineParseFailureKind::PackageValidation(
                    PackageValidationError::IncludeGraphMismatch,
                ),
                locations.root_member(DocumentPackageRootMember::Sources),
            )
        })?;
    let package = ValidatedParsedPackage::new_resolved_with_style_contract(
        package,
        policy,
        &include_graph,
        extended_style_contract,
        |package, error| machine_validation_failure(package, error, &locations),
    )?;
    let advanced = advanced
        .map(|extension| {
            advanced::validate_current_advanced_extension(
                package.clone(),
                extension,
                raw_sha256.into_bytes(),
                canonical_jcs_sha256.into_bytes(),
                policy.limits,
            )
            .map_err(|error| {
                MachineParseFailure::package(
                    MachineParseFailureKind::AdvancedSyntax(error),
                    JsonPointer::root(),
                )
            })
        })
        .transpose()?;
    Ok((
        package,
        advanced,
        raw_sha256,
        canonical_jcs_sha256,
        locations,
    ))
}

fn recheck_machine_admission(
    package: &WireDocumentPackage,
    raw_sha256: RawDocumentPackageSha256,
    canonical_jcs_sha256: CanonicalDocumentPackageJcsSha256,
    sources: &[AdmittedMachineSource],
    admission: &MachineInputAdmissionProvenance,
    locations: &JsonLocationIndex,
) -> Result<(), MachineParseFailure> {
    let progress = admission.progress();
    if progress.stage() != MachineInputStage::SourcesAdmitted {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::AdmissionProgressMismatch,
            JsonPointer::root(),
        ));
    }
    if progress.session_identity() != Some(admission.session_identity()) {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::AdmissionSessionMismatch,
            JsonPointer::root(),
        ));
    }
    if progress.fingerprint() != Some(admission.fingerprint()) {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::AdmissionFingerprintMismatch,
            JsonPointer::root(),
        ));
    }
    let Some(package_facts) = progress.package() else {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::AdmissionProgressMismatch,
            JsonPointer::root(),
        ));
    };
    let Some(decoded_facts) = progress.decoded() else {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::AdmissionProgressMismatch,
            JsonPointer::root(),
        ));
    };
    if package_facts.sha256() != raw_sha256.into_bytes()
        || decoded_facts.canonical_sha256() != canonical_jcs_sha256.into_bytes()
    {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::PackageIdentityMismatch,
            JsonPointer::root(),
        ));
    }
    if decoded_facts.contract() != package.contract {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::ContractMismatch,
            locations.root_member(DocumentPackageRootMember::Contract),
        ));
    }
    if package.coordinate_unit != wire::WireCoordinateUnit::PdfPoint1_65536 {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::CoordinateUnitMismatch,
            locations.root_member(DocumentPackageRootMember::CoordinateUnit),
        ));
    }
    if package.sources.len() != 1 || sources.len() != 1 || progress.sources().len() != 1 {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::SourceCountMismatch,
            locations.root_member(DocumentPackageRootMember::Sources),
        ));
    }
    if progress.sources()[0] != *sources[0].facts() {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::SourceDeclarationMismatch,
            source_pointer(locations, package.sources[0].source_id, 0),
        ));
    }
    let declaration = &package.sources[0];
    let facts = sources[0].facts();
    let pointer = source_pointer(locations, declaration.source_id, 0);
    if declaration.source_id != 0 || declaration.source_id != facts.source_id().get() {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::SourceDeclarationMismatch,
            pointer.child("source_id"),
        ));
    }
    if declaration.uri != facts.uri().as_str() {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::SourceDeclarationMismatch,
            pointer.child("uri"),
        ));
    }
    if u64::from(declaration.utf8_byte_length) != facts.bytes() {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::SourceDeclarationMismatch,
            pointer.child("utf8_byte_length"),
        ));
    }
    if declaration.sha256 != facts.sha256() {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::SourceDeclarationMismatch,
            pointer.child("sha256"),
        ));
    }
    let actual = sources[0].text().as_bytes();
    if u64::try_from(actual.len()).ok() != Some(facts.bytes()) || sha256(actual) != facts.sha256() {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::SourceBytesMismatch,
            source_pointer(locations, declaration.source_id, 0),
        ));
    }
    Ok(())
}

fn source_pointer(locations: &JsonLocationIndex, source_id: u32, occurrence: usize) -> JsonPointer {
    locations
        .source(source_id, occurrence)
        .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::Sources))
}

fn text_pointer(locations: &JsonLocationIndex, text_id: u32, occurrence: usize) -> JsonPointer {
    locations
        .text_buffer(text_id, occurrence)
        .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::TextBuffers))
}

fn node_pointer(locations: &JsonLocationIndex, node_id: u32, occurrence: usize) -> JsonPointer {
    locations
        .node(node_id, occurrence)
        .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::Document))
}

#[derive(Clone, Copy)]
enum WirePreorderNode<'a> {
    Document(&'a wire::WireDocument),
    Block(&'a wire::WireBlock),
    Inline(&'a wire::WireInline),
    Footnote(&'a wire::WireFootnote),
    ListItem(&'a wire::WireListItem),
    TableRow(&'a wire::WireTableRow),
    TableCell(&'a wire::WireTableCell),
}

impl WirePreorderNode<'_> {
    fn node_id(self) -> u32 {
        match self {
            Self::Document(document) => document.node_id,
            Self::Footnote(footnote) => footnote.node_id,
            Self::ListItem(item) => item.node_id,
            Self::TableRow(row) => row.node_id,
            Self::TableCell(cell) => cell.node_id,
            Self::Block(block) => match block {
                wire::WireBlock::Paragraph { node_id, .. }
                | wire::WireBlock::Heading { node_id, .. }
                | wire::WireBlock::List { node_id, .. }
                | wire::WireBlock::Table { node_id, .. }
                | wire::WireBlock::Figure { node_id, .. }
                | wire::WireBlock::PageBreak { node_id, .. } => *node_id,
            },
            Self::Inline(inline) => match inline {
                wire::WireInline::Text { node_id, .. }
                | wire::WireInline::Emphasis { node_id, .. }
                | wire::WireInline::Strong { node_id, .. }
                | wire::WireInline::Link { node_id, .. }
                | wire::WireInline::Anchor { node_id, .. }
                | wire::WireInline::Reference { node_id, .. }
                | wire::WireInline::FootnoteReference { node_id, .. }
                | wire::WireInline::SoftBreak { node_id, .. }
                | wire::WireInline::HardBreak { node_id, .. } => *node_id,
            },
        }
    }
}

fn preflight_wire_semantics(
    package: &WireDocumentPackage,
    limits: &ValidatedResourceLimits,
    locations: &JsonLocationIndex,
) -> Result<(), MachineParseFailure> {
    for (index, source) in package.sources.iter().enumerate() {
        let expected = u32::try_from(index).unwrap_or(u32::MAX);
        if source.source_id != expected {
            return Err(MachineParseFailure::package(
                MachineParseFailureKind::SourceDeclarationMismatch,
                source_pointer(locations, source.source_id, 0).child("source_id"),
            ));
        }
    }
    let mut text_occurrences = BTreeMap::new();
    for (index, buffer) in package.text_buffers.iter().enumerate() {
        let occurrence = next_occurrence(&mut text_occurrences, buffer.text_id);
        let expected = u32::try_from(index).unwrap_or(u32::MAX);
        if buffer.text_id != expected {
            return Err(MachineParseFailure::package(
                MachineParseFailureKind::NonCanonicalTextBufferId,
                text_pointer(locations, buffer.text_id, occurrence).child("text_id"),
            ));
        }
    }
    let style_count = u64::try_from(package.style_sheet.rules.len()).unwrap_or(u64::MAX);
    if style_count > limits.get().max_style_rules {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::PackageValidation(PackageValidationError::StyleRuleLimit),
            locations.root_member(DocumentPackageRootMember::StyleSheet),
        ));
    }
    let mut non_document_nodes = 0u64;
    let mut style_order_occurrences = BTreeMap::new();
    for (index, rule) in package.style_sheet.rules.iter().enumerate() {
        let occurrence = next_occurrence(&mut style_order_occurrences, rule.source_order);
        let expected = u32::try_from(index).unwrap_or(u32::MAX);
        if rule.source_order != expected {
            return Err(MachineParseFailure::package(
                MachineParseFailureKind::NonCanonicalStyleOrder,
                locations
                    .style_rule_by_source_order(rule.source_order, occurrence)
                    .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::StyleSheet))
                    .child("source_order"),
            ));
        }
        let declarations = u64::try_from(rule.declarations.len()).unwrap_or(u64::MAX);
        non_document_nodes = non_document_nodes.saturating_add(declarations.saturating_mul(2));
    }
    if non_document_nodes > limits.get().max_ast_nodes {
        return Err(MachineParseFailure::package(
            MachineParseFailureKind::AstNodeLimit,
            locations.root_member(DocumentPackageRootMember::StyleSheet),
        ));
    }

    let mut previous_master: Option<&str> = None;
    let mut master_occurrences = BTreeMap::new();
    for master in &package.page_masters.masters {
        let occurrence = next_occurrence(&mut master_occurrences, master.master_id.as_str());
        if previous_master.is_some_and(|previous| previous >= master.master_id.as_str()) {
            return Err(MachineParseFailure::package(
                MachineParseFailureKind::NonCanonicalPageMasterOrder,
                locations
                    .page_master(&master.master_id, occurrence)
                    .unwrap_or_else(|| {
                        locations.root_member(DocumentPackageRootMember::PageMasters)
                    })
                    .child("master_id"),
            ));
        }
        previous_master = Some(&master.master_id);
    }
    let mut master_rule_occurrences = BTreeMap::new();
    for (index, rule) in package.page_masters.selection_rules.iter().enumerate() {
        let occurrence = next_occurrence(&mut master_rule_occurrences, rule.source_order);
        let expected = u32::try_from(index).unwrap_or(u32::MAX);
        if rule.source_order != expected {
            return Err(MachineParseFailure::package(
                MachineParseFailureKind::NonCanonicalPageMasterOrder,
                locations
                    .page_master_rule_by_source_order(rule.source_order, occurrence)
                    .unwrap_or_else(|| {
                        locations.root_member(DocumentPackageRootMember::PageMasters)
                    })
                    .child("source_order"),
            ));
        }
    }
    let mut font_occurrences = BTreeMap::new();
    for (index, font) in package.resources.font_faces.iter().enumerate() {
        let occurrence = next_occurrence(&mut font_occurrences, font.font_face_id);
        let expected = u32::try_from(index).unwrap_or(u32::MAX);
        if font.font_face_id != expected {
            return Err(MachineParseFailure::package_with_subject(
                MachineParseFailureKind::NonCanonicalFontFaceId,
                locations
                    .font_face(font.font_face_id, occurrence)
                    .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::Resources))
                    .child("font_face_id"),
                DiagnosticSubject::Resource(ResourceErrorSubject::FontFace(FontFaceId::new(
                    font.font_face_id,
                ))),
            ));
        }
    }
    let mut image_occurrences = BTreeMap::new();
    for (index, image) in package.resources.images.iter().enumerate() {
        let occurrence = next_occurrence(&mut image_occurrences, image.image_id);
        let expected = u32::try_from(index).unwrap_or(u32::MAX);
        if image.image_id != expected {
            return Err(MachineParseFailure::package_with_subject(
                MachineParseFailureKind::NonCanonicalImageId,
                locations
                    .image(image.image_id, occurrence)
                    .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::Resources))
                    .child("image_id"),
                DiagnosticSubject::Resource(ResourceErrorSubject::Image(ImageResourceId::new(
                    image.image_id,
                ))),
            ));
        }
    }
    let mut previous_footnote: Option<&str> = None;
    for footnote in &package.document.footnotes {
        if previous_footnote.is_some_and(|previous| previous >= footnote.footnote_id.as_str()) {
            return Err(MachineParseFailure::package(
                MachineParseFailureKind::PackageValidation(
                    PackageValidationError::NonCanonicalFootnoteOrder,
                ),
                node_pointer(locations, footnote.node_id, 0).child("footnote_id"),
            ));
        }
        previous_footnote = Some(&footnote.footnote_id);
    }

    let mut stack = vec![(WirePreorderNode::Document(&package.document), 1u32)];
    let mut expected_node_id = 0u32;
    let mut observed_nodes = non_document_nodes;
    let mut occurrences = BTreeMap::<u32, usize>::new();
    while let Some((node, depth)) = stack.pop() {
        let node_id = node.node_id();
        let occurrence = occurrences.entry(node_id).or_insert(0);
        let pointer = node_pointer(locations, node_id, *occurrence);
        *occurrence = occurrence.saturating_add(1);
        if depth > limits.get().max_ast_nesting_depth {
            return Err(MachineParseFailure::package(
                MachineParseFailureKind::AstNestingDepthLimit,
                pointer,
            ));
        }
        observed_nodes = observed_nodes.saturating_add(1);
        if observed_nodes > limits.get().max_ast_nodes {
            return Err(MachineParseFailure::package(
                MachineParseFailureKind::AstNodeLimit,
                pointer,
            ));
        }
        if node_id != expected_node_id {
            return Err(MachineParseFailure::package_with_subject(
                MachineParseFailureKind::NonCanonicalNodeId,
                pointer.child("node_id"),
                DiagnosticSubject::Node(NodeId::new(node_id)),
            ));
        }
        expected_node_id = expected_node_id.checked_add(1).ok_or_else(|| {
            MachineParseFailure::package(MachineParseFailureKind::AstNodeLimit, pointer.clone())
        })?;
        let child_depth = depth.checked_add(1).ok_or_else(|| {
            MachineParseFailure::package(
                MachineParseFailureKind::AstNestingDepthLimit,
                pointer.clone(),
            )
        })?;
        push_wire_children(&mut stack, node, child_depth);
    }
    Ok(())
}

fn next_occurrence<K: Ord>(occurrences: &mut BTreeMap<K, usize>, key: K) -> usize {
    let next = occurrences.entry(key).or_insert(0);
    let occurrence = *next;
    *next = next.saturating_add(1);
    occurrence
}

fn push_wire_children<'a>(
    stack: &mut Vec<(WirePreorderNode<'a>, u32)>,
    node: WirePreorderNode<'a>,
    depth: u32,
) {
    match node {
        WirePreorderNode::Document(document) => {
            stack.extend(
                document
                    .footnotes
                    .iter()
                    .rev()
                    .map(|value| (WirePreorderNode::Footnote(value), depth)),
            );
            stack.extend(
                document
                    .blocks
                    .iter()
                    .rev()
                    .map(|value| (WirePreorderNode::Block(value), depth)),
            );
        }
        WirePreorderNode::Block(block) => match block {
            wire::WireBlock::Paragraph { children, .. }
            | wire::WireBlock::Heading { children, .. } => stack.extend(
                children
                    .iter()
                    .rev()
                    .map(|value| (WirePreorderNode::Inline(value), depth)),
            ),
            wire::WireBlock::List { items, .. } => stack.extend(
                items
                    .iter()
                    .rev()
                    .map(|value| (WirePreorderNode::ListItem(value), depth)),
            ),
            wire::WireBlock::Table { head, body, .. } => {
                stack.extend(
                    body.iter()
                        .rev()
                        .map(|value| (WirePreorderNode::TableRow(value), depth)),
                );
                stack.extend(
                    head.iter()
                        .rev()
                        .map(|value| (WirePreorderNode::TableRow(value), depth)),
                );
            }
            wire::WireBlock::Figure { caption, .. } => stack.extend(
                caption
                    .iter()
                    .rev()
                    .map(|value| (WirePreorderNode::Block(value), depth)),
            ),
            wire::WireBlock::PageBreak { .. } => {}
        },
        WirePreorderNode::Inline(inline) => match inline {
            wire::WireInline::Emphasis { children, .. }
            | wire::WireInline::Strong { children, .. }
            | wire::WireInline::Link { children, .. } => stack.extend(
                children
                    .iter()
                    .rev()
                    .map(|value| (WirePreorderNode::Inline(value), depth)),
            ),
            wire::WireInline::Text { .. }
            | wire::WireInline::Anchor { .. }
            | wire::WireInline::Reference { .. }
            | wire::WireInline::FootnoteReference { .. }
            | wire::WireInline::SoftBreak { .. }
            | wire::WireInline::HardBreak { .. } => {}
        },
        WirePreorderNode::Footnote(footnote) => stack.extend(
            footnote
                .blocks
                .iter()
                .rev()
                .map(|value| (WirePreorderNode::Block(value), depth)),
        ),
        WirePreorderNode::ListItem(item) => stack.extend(
            item.blocks
                .iter()
                .rev()
                .map(|value| (WirePreorderNode::Block(value), depth)),
        ),
        WirePreorderNode::TableRow(row) => stack.extend(
            row.cells
                .iter()
                .rev()
                .map(|value| (WirePreorderNode::TableCell(value), depth)),
        ),
        WirePreorderNode::TableCell(cell) => stack.extend(
            cell.blocks
                .iter()
                .rev()
                .map(|value| (WirePreorderNode::Block(value), depth)),
        ),
    }
}

fn lower_sources(
    declarations: Vec<wire::WireSource>,
    admitted: Vec<AdmittedMachineSource>,
    locations: &JsonLocationIndex,
) -> Result<SourceCatalog, MachineParseFailure> {
    let mut records = Vec::with_capacity(admitted.len());
    for (declaration, source) in declarations.into_iter().zip(admitted) {
        let pointer = source_pointer(locations, declaration.source_id, 0);
        let (facts, text) = source.into_parts();
        let record =
            SourceRecord::new(facts.source_id(), facts.uri().clone(), text).map_err(|_| {
                MachineParseFailure::package(
                    MachineParseFailureKind::InvalidSourceCatalog,
                    pointer.clone(),
                )
            })?;
        if record.utf8_byte_length() != declaration.utf8_byte_length
            || record.content_hash() != declaration.sha256
        {
            return Err(MachineParseFailure::package(
                MachineParseFailureKind::SourceBytesMismatch,
                pointer,
            ));
        }
        records.push(record);
    }
    SourceCatalog::new(records).map_err(|_| {
        MachineParseFailure::package(
            MachineParseFailureKind::InvalidSourceCatalog,
            locations.root_member(DocumentPackageRootMember::Sources),
        )
    })
}

fn lower_text_store(
    buffers: Vec<wire::WireTextBuffer>,
    limits: &ValidatedResourceLimits,
    locations: &JsonLocationIndex,
) -> Result<TextStore, MachineParseFailure> {
    let mut lowered = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let pointer = text_pointer(locations, buffer.text_id, 0);
        let text_id = TextBufferId::new(buffer.text_id);
        let mut mappings = Vec::with_capacity(buffer.mappings.len());
        for (ordinal, mapping) in buffer.mappings.into_iter().enumerate() {
            let mapping_pointer = locations
                .text_mapping(buffer.text_id, 0, ordinal)
                .unwrap_or_else(|| pointer.clone());
            mappings.push(TextMapSegment {
                text_range: lower_byte_range(
                    mapping.text_range,
                    mapping_pointer.child("text_range"),
                )?,
                kind: match mapping.kind {
                    wire::WireTextMapKind::Identity => TextMapKind::Identity,
                    wire::WireTextMapKind::Replacement => TextMapKind::Replacement,
                    wire::WireTextMapKind::Inserted => TextMapKind::Inserted,
                },
                source_span: mapping
                    .source_span
                    .map(|span| lower_source_span(span, mapping_pointer.child("source_span")))
                    .transpose()?,
            });
        }
        lowered.push(
            TextBuffer::new(
                text_id,
                buffer.utf8,
                mappings,
                limits.get().max_text_buffer_bytes,
            )
            .map_err(|_| {
                MachineParseFailure::package(MachineParseFailureKind::InvalidTextBuffer, pointer)
            })?,
        );
    }
    TextStore::new(lowered).map_err(|_| {
        MachineParseFailure::package(
            MachineParseFailureKind::NonCanonicalTextBufferId,
            locations.root_member(DocumentPackageRootMember::TextBuffers),
        )
    })
}

fn lower_byte_range(
    range: wire::WireByteRange,
    pointer: JsonPointer,
) -> Result<Utf8ByteRange, MachineParseFailure> {
    Utf8ByteRange::new(
        Utf8ByteOffset::new(range.start_byte),
        Utf8ByteOffset::new(range.end_byte),
    )
    .ok_or_else(|| MachineParseFailure::package(MachineParseFailureKind::InvalidTextSpan, pointer))
}

fn lower_source_span(
    span: wire::WireSourceSpan,
    pointer: JsonPointer,
) -> Result<SourceSpan, MachineParseFailure> {
    SourceSpan::new(
        SourceId::new(span.source_id),
        Utf8ByteOffset::new(span.start_byte),
        Utf8ByteOffset::new(span.end_byte),
    )
    .ok_or_else(|| {
        MachineParseFailure::package(MachineParseFailureKind::InvalidSourceSpan, pointer)
    })
}

fn lower_text_span(
    span: wire::WireTextSpan,
    pointer: JsonPointer,
) -> Result<TextSpan, MachineParseFailure> {
    TextSpan::new(
        TextBufferId::new(span.text_id),
        Utf8ByteOffset::new(span.start_byte),
        Utf8ByteOffset::new(span.end_byte),
    )
    .ok_or_else(|| MachineParseFailure::package(MachineParseFailureKind::InvalidTextSpan, pointer))
}

fn lower_document(
    document: wire::WireDocument,
    policy: &PackageValidationPolicy<'_>,
    locations: &JsonLocationIndex,
) -> Result<Document, MachineParseFailure> {
    Ok(Document {
        node_id: NodeId::new(document.node_id),
        blocks: document
            .blocks
            .into_iter()
            .map(|block| lower_block(block, policy, locations))
            .collect::<Result<_, _>>()?,
        footnotes: document
            .footnotes
            .into_iter()
            .map(|footnote| {
                let pointer = node_pointer(locations, footnote.node_id, 0);
                Ok(FootnoteDefinition {
                    footnote_id: FootnoteId::new(footnote.footnote_id).map_err(|_| {
                        MachineParseFailure::package(
                            MachineParseFailureKind::InvalidIdentifier,
                            pointer.child("footnote_id"),
                        )
                    })?,
                    node_id: NodeId::new(footnote.node_id),
                    span: lower_source_span(footnote.span, pointer.child("span"))?,
                    blocks: footnote
                        .blocks
                        .into_iter()
                        .map(|block| lower_block(block, policy, locations))
                        .collect::<Result<_, _>>()?,
                })
            })
            .collect::<Result<_, MachineParseFailure>>()?,
    })
}

fn lower_block(
    block: wire::WireBlock,
    policy: &PackageValidationPolicy<'_>,
    locations: &JsonLocationIndex,
) -> Result<Block, MachineParseFailure> {
    match block {
        wire::WireBlock::Paragraph {
            node_id,
            span,
            classes,
            children,
        } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Block::Paragraph {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
                classes,
                children: lower_inlines(children, policy, locations)?,
            })
        }
        wire::WireBlock::Heading {
            node_id,
            span,
            classes,
            level,
            anchor_id,
            children,
        } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Block::Heading {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
                classes,
                level: HeadingLevel::new(level).ok_or_else(|| {
                    MachineParseFailure::package(
                        MachineParseFailureKind::InvalidHeadingLevel,
                        pointer.child("level"),
                    )
                })?,
                anchor_id: anchor_id
                    .map(|value| {
                        AnchorId::new(value).map_err(|_| {
                            MachineParseFailure::package(
                                MachineParseFailureKind::InvalidIdentifier,
                                pointer.child("anchor_id"),
                            )
                        })
                    })
                    .transpose()?,
                children: lower_inlines(children, policy, locations)?,
            })
        }
        wire::WireBlock::List {
            node_id,
            span,
            classes,
            ordered,
            start,
            items,
        } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Block::List {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
                classes,
                ordered,
                start,
                items: items
                    .into_iter()
                    .map(|item| lower_list_item(item, policy, locations))
                    .collect::<Result<_, _>>()?,
            })
        }
        wire::WireBlock::Table {
            node_id,
            span,
            classes,
            columns,
            head,
            body,
        } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Block::Table {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
                classes,
                columns: columns
                    .into_iter()
                    .enumerate()
                    .map(|(index, column)| {
                        let column_pointer = pointer.child("columns").child(&index.to_string());
                        let sizing = match column {
                            wire::WireTableColumn::Fixed { width } => {
                                ColumnSizing::Fixed(positive_length(width, column_pointer)?)
                            }
                            wire::WireTableColumn::Fraction { weight } => ColumnSizing::Fraction(
                                NonZeroU16::new(weight).ok_or_else(|| {
                                    MachineParseFailure::package(
                                        MachineParseFailureKind::InvalidTableShape,
                                        column_pointer,
                                    )
                                })?,
                            ),
                        };
                        Ok(TableColumn { sizing })
                    })
                    .collect::<Result<_, MachineParseFailure>>()?,
                head: head
                    .into_iter()
                    .map(|row| lower_table_row(row, policy, locations))
                    .collect::<Result<_, _>>()?,
                body: body
                    .into_iter()
                    .map(|row| lower_table_row(row, policy, locations))
                    .collect::<Result<_, _>>()?,
            })
        }
        wire::WireBlock::Figure {
            node_id,
            span,
            classes,
            image_id,
            alt,
            caption,
        } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Block::Figure {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
                classes,
                image_id: ImageResourceId::new(image_id),
                alt,
                caption: caption
                    .into_iter()
                    .map(|block| lower_block(block, policy, locations))
                    .collect::<Result<_, _>>()?,
            })
        }
        wire::WireBlock::PageBreak {
            node_id,
            span,
            classes,
        } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Block::PageBreak {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
                classes,
            })
        }
    }
}

fn lower_list_item(
    item: wire::WireListItem,
    policy: &PackageValidationPolicy<'_>,
    locations: &JsonLocationIndex,
) -> Result<ListItem, MachineParseFailure> {
    let pointer = node_pointer(locations, item.node_id, 0);
    Ok(ListItem {
        node_id: NodeId::new(item.node_id),
        span: lower_source_span(item.span, pointer.child("span"))?,
        blocks: item
            .blocks
            .into_iter()
            .map(|block| lower_block(block, policy, locations))
            .collect::<Result<_, _>>()?,
    })
}

fn lower_table_row(
    row: wire::WireTableRow,
    policy: &PackageValidationPolicy<'_>,
    locations: &JsonLocationIndex,
) -> Result<TableRow, MachineParseFailure> {
    let pointer = node_pointer(locations, row.node_id, 0);
    Ok(TableRow {
        node_id: NodeId::new(row.node_id),
        span: lower_source_span(row.span, pointer.child("span"))?,
        cells: row
            .cells
            .into_iter()
            .map(|cell| lower_table_cell(cell, policy, locations))
            .collect::<Result<_, _>>()?,
    })
}

fn lower_table_cell(
    cell: wire::WireTableCell,
    policy: &PackageValidationPolicy<'_>,
    locations: &JsonLocationIndex,
) -> Result<TableCell, MachineParseFailure> {
    let pointer = node_pointer(locations, cell.node_id, 0);
    Ok(TableCell {
        node_id: NodeId::new(cell.node_id),
        span: lower_source_span(cell.span, pointer.child("span"))?,
        colspan: NonZeroU16::new(cell.colspan).ok_or_else(|| {
            MachineParseFailure::package(
                MachineParseFailureKind::InvalidTableShape,
                pointer.child("colspan"),
            )
        })?,
        rowspan: NonZeroU16::new(cell.rowspan).ok_or_else(|| {
            MachineParseFailure::package(
                MachineParseFailureKind::InvalidTableShape,
                pointer.child("rowspan"),
            )
        })?,
        blocks: cell
            .blocks
            .into_iter()
            .map(|block| lower_block(block, policy, locations))
            .collect::<Result<_, _>>()?,
    })
}

fn lower_inlines(
    inlines: Vec<wire::WireInline>,
    policy: &PackageValidationPolicy<'_>,
    locations: &JsonLocationIndex,
) -> Result<Vec<Inline>, MachineParseFailure> {
    inlines
        .into_iter()
        .map(|inline| lower_inline(inline, policy, locations))
        .collect()
}

fn lower_inline(
    inline: wire::WireInline,
    policy: &PackageValidationPolicy<'_>,
    locations: &JsonLocationIndex,
) -> Result<Inline, MachineParseFailure> {
    match inline {
        wire::WireInline::Text {
            node_id,
            span,
            text_span,
        } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Inline::Text {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
                text_span: lower_text_span(text_span, pointer.child("text_span"))?,
            })
        }
        wire::WireInline::Emphasis {
            node_id,
            span,
            children,
        } => lower_inline_container(node_id, span, children, policy, locations, true),
        wire::WireInline::Strong {
            node_id,
            span,
            children,
        } => lower_inline_container(node_id, span, children, policy, locations, false),
        wire::WireInline::Link {
            node_id,
            span,
            target,
            children,
        } => {
            let pointer = node_pointer(locations, node_id, 0);
            let target = match target {
                wire::WireLinkTarget::Internal { anchor_id } => {
                    LinkTarget::Internal(AnchorId::new(anchor_id).map_err(|_| {
                        MachineParseFailure::package(
                            MachineParseFailureKind::InvalidIdentifier,
                            pointer.child("target").child("anchor_id"),
                        )
                    })?)
                }
                wire::WireLinkTarget::Uri { uri } => {
                    let schemes: Vec<&str> = policy
                        .allowed_uri_schemes
                        .iter()
                        .map(String::as_str)
                        .collect();
                    LinkTarget::Uri(
                        SafeUri::with_policy(
                            uri,
                            &schemes,
                            policy.limits.get().max_uri_bytes as usize,
                        )
                        .map_err(|_| {
                            MachineParseFailure::package(
                                MachineParseFailureKind::InvalidUri,
                                pointer.child("target").child("uri"),
                            )
                        })?,
                    )
                }
            };
            Ok(Inline::Link {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
                target,
                children: lower_inlines(children, policy, locations)?,
            })
        }
        wire::WireInline::Anchor {
            node_id,
            span,
            anchor_id,
        } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Inline::Anchor {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
                anchor_id: AnchorId::new(anchor_id).map_err(|_| {
                    MachineParseFailure::package(
                        MachineParseFailureKind::InvalidIdentifier,
                        pointer.child("anchor_id"),
                    )
                })?,
            })
        }
        wire::WireInline::Reference {
            node_id,
            span,
            target,
            format,
        } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Inline::Reference {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
                target: AnchorId::new(target).map_err(|_| {
                    MachineParseFailure::package(
                        MachineParseFailureKind::InvalidIdentifier,
                        pointer.child("target"),
                    )
                })?,
                format: match format {
                    wire::WireReferenceFormat::Text => ReferenceFormat::Text,
                    wire::WireReferenceFormat::Page => ReferenceFormat::Page,
                    wire::WireReferenceFormat::Number => ReferenceFormat::Number,
                },
            })
        }
        wire::WireInline::FootnoteReference {
            node_id,
            span,
            footnote_id,
        } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Inline::FootnoteReference {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
                footnote_id: FootnoteId::new(footnote_id).map_err(|_| {
                    MachineParseFailure::package(
                        MachineParseFailureKind::InvalidIdentifier,
                        pointer.child("footnote_id"),
                    )
                })?,
            })
        }
        wire::WireInline::SoftBreak { node_id, span } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Inline::SoftBreak {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
            })
        }
        wire::WireInline::HardBreak { node_id, span } => {
            let pointer = node_pointer(locations, node_id, 0);
            Ok(Inline::HardBreak {
                node_id: NodeId::new(node_id),
                span: lower_source_span(span, pointer.child("span"))?,
            })
        }
    }
}

fn lower_inline_container(
    node_id: u32,
    span: wire::WireSourceSpan,
    children: Vec<wire::WireInline>,
    policy: &PackageValidationPolicy<'_>,
    locations: &JsonLocationIndex,
    emphasis: bool,
) -> Result<Inline, MachineParseFailure> {
    let pointer = node_pointer(locations, node_id, 0);
    let span = lower_source_span(span, pointer.child("span"))?;
    let children = lower_inlines(children, policy, locations)?;
    if emphasis {
        Ok(Inline::Emphasis {
            node_id: NodeId::new(node_id),
            span,
            children,
        })
    } else {
        Ok(Inline::Strong {
            node_id: NodeId::new(node_id),
            span,
            children,
        })
    }
}

fn positive_length(
    value: i64,
    pointer: JsonPointer,
) -> Result<PositiveLength, MachineParseFailure> {
    Length::from_raw(value)
        .and_then(PositiveLength::new)
        .ok_or_else(|| {
            MachineParseFailure::package(MachineParseFailureKind::InvalidLength, pointer)
        })
}

fn lower_style_sheet(
    style_sheet: wire::WireStyleSheet,
    locations: &JsonLocationIndex,
) -> Result<StyleSheet, MachineParseFailure> {
    let mut rules = Vec::with_capacity(style_sheet.rules.len());
    for rule in style_sheet.rules {
        let pointer = locations
            .style_rule_by_source_order(rule.source_order, 0)
            .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::StyleSheet));
        let style_id = StyleId::new(rule.style_id).map_err(|_| {
            MachineParseFailure::package(
                MachineParseFailureKind::InvalidIdentifier,
                pointer.child("style_id"),
            )
        })?;
        let extends = rule
            .extends
            .map(|value| {
                StyleId::new(value).map_err(|_| {
                    MachineParseFailure::package(
                        MachineParseFailureKind::InvalidIdentifier,
                        pointer.child("extends"),
                    )
                })
            })
            .transpose()?;
        let mut declarations = Vec::with_capacity(rule.declarations.len());
        for (ordinal, declaration) in rule.declarations.into_iter().enumerate() {
            let declaration_pointer = pointer.child("declarations").child(&ordinal.to_string());
            let name = match declaration.name {
                wire::WireDeclarationName::FontFamily => "font_family",
                wire::WireDeclarationName::FontSize => "font_size",
                wire::WireDeclarationName::LineHeight => "line_height",
                wire::WireDeclarationName::Page => "page",
                wire::WireDeclarationName::SpaceBefore => "space_before",
                wire::WireDeclarationName::SpaceAfter => "space_after",
                wire::WireDeclarationName::StartIndent => "start_indent",
                wire::WireDeclarationName::EndIndent => "end_indent",
                wire::WireDeclarationName::TextAlign => "text_align",
                wire::WireDeclarationName::Width => "width",
                wire::WireDeclarationName::KeepWithNext => "keep_with_next",
                wire::WireDeclarationName::KeepCaption => "keep_caption",
            };
            let subject = DiagnosticSubject::Style(StyleErrorSubject::new(
                NodeId::new(0),
                Some(style_id.clone()),
                Some(
                    StylePropertyName::new(name)
                        .expect("closed wire declaration names are canonical"),
                ),
            ));
            declarations.push(Declaration {
                name: name.to_owned(),
                value: lower_style_value(declaration.value, declaration_pointer.child("value"))
                    .map_err(|failure| failure.with_subject(subject))?,
                important: declaration.important,
            });
        }
        rules.push(StyleRule {
            style_id,
            extends,
            selector: rule.selector,
            source_order: rule.source_order,
            declarations,
        });
    }
    Ok(StyleSheet { rules })
}

fn lower_style_value(
    value: wire::WireStyleValue,
    pointer: JsonPointer,
) -> Result<StyleValue, MachineParseFailure> {
    match value {
        wire::WireStyleValue::Keyword { value } => Ok(StyleValue::Keyword(value)),
        wire::WireStyleValue::String { value } => Ok(StyleValue::Text(value)),
        wire::WireStyleValue::Integer { value } => Ok(StyleValue::Integer(value)),
        wire::WireStyleValue::Length { value } => Length::from_raw(value)
            .map(StyleValue::Length)
            .ok_or_else(|| {
                MachineParseFailure::package(MachineParseFailureKind::InvalidLength, pointer)
            }),
        wire::WireStyleValue::Boolean { value } => Ok(StyleValue::Boolean(value)),
        wire::WireStyleValue::FontFamilyList { families } => {
            Ok(StyleValue::FontFamilyList(families))
        }
        wire::WireStyleValue::Ratio {
            numerator,
            denominator,
        } => Ok(StyleValue::Ratio {
            numerator,
            denominator: NonZeroU64::new(denominator).ok_or_else(|| {
                MachineParseFailure::package(MachineParseFailureKind::InvalidLength, pointer)
            })?,
        }),
    }
}

fn lower_page_masters(
    page_masters: wire::WirePageMasterSet,
    locations: &JsonLocationIndex,
) -> Result<PageMasterSet, MachineParseFailure> {
    let default_master_id = MasterId::new(page_masters.default_master_id).map_err(|_| {
        MachineParseFailure::package(
            MachineParseFailureKind::InvalidIdentifier,
            locations
                .root_member(DocumentPackageRootMember::PageMasters)
                .child("default_master_id"),
        )
    })?;
    let mut masters = Vec::with_capacity(page_masters.masters.len());
    for master in page_masters.masters {
        let pointer = locations
            .page_master(&master.master_id, 0)
            .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::PageMasters));
        let master_id = MasterId::new(master.master_id).map_err(|_| {
            MachineParseFailure::package(
                MachineParseFailureKind::InvalidIdentifier,
                pointer.child("master_id"),
            )
        })?;
        let subject = DiagnosticSubject::Master(MasterErrorSubject::new(master_id.clone(), None));
        masters.push(PageMaster {
            master_id,
            width: positive_length(master.width, pointer.child("width"))
                .map_err(|failure| failure.with_subject(subject.clone()))?,
            height: positive_length(master.height, pointer.child("height"))
                .map_err(|failure| failure.with_subject(subject.clone()))?,
            body: lower_rect(master.body, pointer.child("body"))
                .map_err(|failure| failure.with_subject(subject.clone()))?,
            header: master
                .header
                .map(|rect| lower_rect(rect, pointer.child("header")))
                .transpose()
                .map_err(|failure| failure.with_subject(subject.clone()))?,
            footer: master
                .footer
                .map(|rect| lower_rect(rect, pointer.child("footer")))
                .transpose()
                .map_err(|failure| failure.with_subject(subject.clone()))?,
            footnote: master
                .footnote
                .map(|rect| lower_rect(rect, pointer.child("footnote")))
                .transpose()
                .map_err(|failure| failure.with_subject(subject))?,
        });
    }
    let mut selection_rules = Vec::with_capacity(page_masters.selection_rules.len());
    for (ordinal, rule) in page_masters.selection_rules.into_iter().enumerate() {
        let pointer = locations
            .page_master_selection_rule(ordinal)
            .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::PageMasters));
        let master_id = MasterId::new(rule.master_id).map_err(|_| {
            MachineParseFailure::package(
                MachineParseFailureKind::InvalidIdentifier,
                pointer.child("master_id"),
            )
        })?;
        let subject = DiagnosticSubject::Master(MasterErrorSubject::new(
            master_id.clone(),
            u32::try_from(ordinal).ok(),
        ));
        selection_rules.push(PageMasterRule {
            master_id,
            parity: match rule.parity {
                wire::WirePageParity::Any => PageParity::Any,
                wire::WirePageParity::Odd => PageParity::Odd,
                wire::WirePageParity::Even => PageParity::Even,
            },
            first: rule.first,
            named_page: rule
                .named_page
                .map(|value| {
                    PageName::new(value).map_err(|_| {
                        MachineParseFailure::package(
                            MachineParseFailureKind::InvalidIdentifier,
                            pointer.child("named_page"),
                        )
                        .with_subject(subject.clone())
                    })
                })
                .transpose()?,
            source_order: rule.source_order,
        });
    }
    Ok(PageMasterSet {
        default_master_id,
        masters,
        selection_rules,
    })
}

fn lower_rect(rect: wire::WireRect, pointer: JsonPointer) -> Result<Rect, MachineParseFailure> {
    let x = Length::from_raw(rect.x).ok_or_else(|| {
        MachineParseFailure::package(MachineParseFailureKind::InvalidLength, pointer.child("x"))
    })?;
    let y = Length::from_raw(rect.y).ok_or_else(|| {
        MachineParseFailure::package(MachineParseFailureKind::InvalidLength, pointer.child("y"))
    })?;
    let width = positive_length(rect.width, pointer.child("width"))?;
    let height = positive_length(rect.height, pointer.child("height"))?;
    Ok(Rect::new(x, y, width, height))
}

fn lower_resources(
    resources: wire::WireResourceCatalog,
    locations: &JsonLocationIndex,
) -> Result<ResourceCatalog, MachineParseFailure> {
    let mut font_faces = Vec::with_capacity(resources.font_faces.len());
    for font in resources.font_faces {
        let pointer = locations
            .font_face(font.font_face_id, 0)
            .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::Resources));
        font_faces.push(FontFaceDeclaration {
            font_face_id: FontFaceId::new(font.font_face_id),
            family: font.family,
            uri: PortablePath::new(font.uri).map_err(|_| {
                MachineParseFailure::package_with_subject(
                    MachineParseFailureKind::InvalidUri,
                    pointer.child("uri"),
                    DiagnosticSubject::Resource(ResourceErrorSubject::FontFace(FontFaceId::new(
                        font.font_face_id,
                    ))),
                )
            })?,
            face_index: font.face_index,
            expected_sha256: font.expected_sha256,
        });
    }
    let mut images = Vec::with_capacity(resources.images.len());
    for image in resources.images {
        let pointer = locations
            .image(image.image_id, 0)
            .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::Resources));
        images.push(ImageDeclaration {
            image_id: ImageResourceId::new(image.image_id),
            uri: PortablePath::new(image.uri).map_err(|_| {
                MachineParseFailure::package_with_subject(
                    MachineParseFailureKind::InvalidUri,
                    pointer.child("uri"),
                    DiagnosticSubject::Resource(ResourceErrorSubject::Image(ImageResourceId::new(
                        image.image_id,
                    ))),
                )
            })?,
            expected_sha256: image.expected_sha256,
        });
    }
    Ok(ResourceCatalog { font_faces, images })
}

fn machine_validation_failure(
    package: &ParsedPackage,
    error: PackageValidationError,
    locations: &JsonLocationIndex,
) -> MachineParseFailure {
    if matches!(
        error,
        PackageValidationError::SourceSpanOutOfBounds
            | PackageValidationError::SourceSpanNotUtf8Boundary
            | PackageValidationError::IdentityBytesMismatch
    ) {
        if let Some((span, pointer)) = locate_mapping_source_failure(package, &error, locations)
            .or_else(|| locate_document_source_failure(package, &error, locations))
        {
            return MachineParseFailure::source(
                MachineParseFailureKind::PackageValidation(error),
                span,
                pointer,
            );
        }
    }
    if matches!(
        error,
        PackageValidationError::UnknownTextBuffer
            | PackageValidationError::TextSpanOutOfBounds
            | PackageValidationError::TextSpanNotUtf8Boundary
    ) {
        if let Some(pointer) = locate_document_text_failure(package, &error, locations) {
            return MachineParseFailure::package(
                MachineParseFailureKind::PackageValidation(error),
                pointer,
            );
        }
    }

    let pointer = match &error {
        PackageValidationError::UnknownSource
        | PackageValidationError::SourceSpanOutOfBounds
        | PackageValidationError::SourceSpanNotUtf8Boundary
        | PackageValidationError::IdentityBytesMismatch
        | PackageValidationError::SourceByteLimit
        | PackageValidationError::InputByteLimit
        | PackageValidationError::IncludeFileLimit
        | PackageValidationError::MissingEntrySource
        | PackageValidationError::IncludeGraphMismatch
        | PackageValidationError::UnresolvedIncludeDirective => {
            locations.root_member(DocumentPackageRootMember::Sources)
        }
        PackageValidationError::UnknownTextBuffer
        | PackageValidationError::TextSpanOutOfBounds
        | PackageValidationError::TextSpanNotUtf8Boundary
        | PackageValidationError::TextBufferByteLimit
        | PackageValidationError::TextByteLimit => {
            locations.root_member(DocumentPackageRootMember::TextBuffers)
        }
        PackageValidationError::InvalidStyle(_) | PackageValidationError::StyleRuleLimit => {
            locations
                .style_rule_by_source_order(0, 0)
                .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::StyleSheet))
        }
        PackageValidationError::InvalidPageMasters(_) => locations
            .page_master(&package.page_masters.default_master_id.to_string(), 0)
            .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::PageMasters)),
        PackageValidationError::DuplicateFontFaceId
        | PackageValidationError::NonCanonicalFontFaceId
        | PackageValidationError::DuplicateFontFamily
        | PackageValidationError::InvalidFontFamily => package
            .resources
            .font_faces
            .first()
            .and_then(|font| locations.font_face(font.font_face_id.get(), 0))
            .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::Resources)),
        PackageValidationError::DuplicateImageId | PackageValidationError::NonCanonicalImageId => {
            package
                .resources
                .images
                .first()
                .and_then(|image| locations.image(image.image_id.get(), 0))
                .unwrap_or_else(|| locations.root_member(DocumentPackageRootMember::Resources))
        }
        PackageValidationError::InvalidUri(_) => {
            locations.root_member(DocumentPackageRootMember::Document)
        }
        PackageValidationError::AstNestingDepthLimit
        | PackageValidationError::AstNodeLimit
        | PackageValidationError::DuplicateNodeId
        | PackageValidationError::NonCanonicalNodeId
        | PackageValidationError::DuplicateAnchorId
        | PackageValidationError::DuplicateFootnoteId
        | PackageValidationError::UnknownInternalTarget
        | PackageValidationError::UnknownFootnoteTarget
        | PackageValidationError::UnknownImageTarget
        | PackageValidationError::InvalidBlockClass
        | PackageValidationError::DuplicateBlockClass
        | PackageValidationError::NonCanonicalBlockClasses
        | PackageValidationError::InvalidListStart
        | PackageValidationError::EmptyListItems
        | PackageValidationError::ListMarkerOverflow
        | PackageValidationError::EmptyTableColumns
        | PackageValidationError::EmptyTableRows
        | PackageValidationError::InvalidTableGrid
        | PackageValidationError::TableHeadBodyCross
        | PackageValidationError::NonCanonicalFootnoteOrder => {
            locations.root_member(DocumentPackageRootMember::Document)
        }
    };
    let subject = match &error {
        PackageValidationError::InvalidStyle(_) | PackageValidationError::StyleRuleLimit => {
            package.style_sheet.rules.first().map(|rule| {
                DiagnosticSubject::Style(StyleErrorSubject::new(
                    package.document.node_id,
                    Some(rule.style_id.clone()),
                    rule.declarations
                        .first()
                        .and_then(|declaration| StylePropertyName::new(declaration.name.clone())),
                ))
            })
        }
        PackageValidationError::InvalidPageMasters(_) => Some(DiagnosticSubject::Master(
            MasterErrorSubject::new(package.page_masters.default_master_id.clone(), None),
        )),
        PackageValidationError::DuplicateFontFaceId
        | PackageValidationError::NonCanonicalFontFaceId
        | PackageValidationError::DuplicateFontFamily
        | PackageValidationError::InvalidFontFamily => {
            package.resources.font_faces.first().map(|font| {
                DiagnosticSubject::Resource(ResourceErrorSubject::FontFace(font.font_face_id))
            })
        }
        PackageValidationError::DuplicateImageId | PackageValidationError::NonCanonicalImageId => {
            package.resources.images.first().map(|image| {
                DiagnosticSubject::Resource(ResourceErrorSubject::Image(image.image_id))
            })
        }
        PackageValidationError::InvalidUri(_)
        | PackageValidationError::AstNestingDepthLimit
        | PackageValidationError::AstNodeLimit
        | PackageValidationError::DuplicateNodeId
        | PackageValidationError::NonCanonicalNodeId
        | PackageValidationError::DuplicateAnchorId
        | PackageValidationError::DuplicateFootnoteId
        | PackageValidationError::UnknownInternalTarget
        | PackageValidationError::UnknownFootnoteTarget
        | PackageValidationError::UnknownImageTarget
        | PackageValidationError::InvalidBlockClass
        | PackageValidationError::DuplicateBlockClass
        | PackageValidationError::NonCanonicalBlockClasses
        | PackageValidationError::InvalidListStart
        | PackageValidationError::EmptyListItems
        | PackageValidationError::ListMarkerOverflow
        | PackageValidationError::EmptyTableColumns
        | PackageValidationError::EmptyTableRows
        | PackageValidationError::InvalidTableGrid
        | PackageValidationError::TableHeadBodyCross
        | PackageValidationError::NonCanonicalFootnoteOrder => {
            Some(DiagnosticSubject::Node(package.document.node_id))
        }
        _ => None,
    };
    let failure =
        MachineParseFailure::package(MachineParseFailureKind::PackageValidation(error), pointer);
    match subject {
        Some(subject) => failure.with_subject(subject),
        None => failure,
    }
}

fn locate_mapping_source_failure(
    package: &ParsedPackage,
    expected: &PackageValidationError,
    locations: &JsonLocationIndex,
) -> Option<(SourceSpan, JsonPointer)> {
    for buffer in package.text_store.buffers() {
        for (ordinal, mapping) in buffer.mappings().iter().enumerate() {
            let Some(span) = mapping.source_span else {
                continue;
            };
            let pointer = locations
                .text_mapping(buffer.text_id().get(), 0, ordinal)
                .unwrap_or_else(|| text_pointer(locations, buffer.text_id().get(), 0));
            match validate_source_span(package, span) {
                Err(error) if &error == expected => return Some((span, pointer)),
                Err(_) => continue,
                Ok(source) if *expected == PackageValidationError::IdentityBytesMismatch => {
                    if mapping.kind != TextMapKind::Identity {
                        continue;
                    }
                    let text_start = mapping.text_range.start_byte().get() as usize;
                    let text_end = mapping.text_range.end_byte().get() as usize;
                    let source_start = span.start_byte().get() as usize;
                    let source_end = span.end_byte().get() as usize;
                    if buffer.text()[text_start..text_end]
                        != source.utf8()[source_start..source_end]
                    {
                        return Some((span, pointer));
                    }
                }
                Ok(_) => {}
            }
        }
    }
    None
}

#[derive(Clone, Copy)]
enum LocatedDomainNode<'a> {
    Block(&'a Block),
    Inline(&'a Inline),
    Footnote(&'a FootnoteDefinition),
    ListItem(&'a ListItem),
    TableRow(&'a TableRow),
    TableCell(&'a TableCell),
}

impl LocatedDomainNode<'_> {
    fn node_id(self) -> NodeId {
        match self {
            Self::Footnote(value) => value.node_id,
            Self::ListItem(value) => value.node_id,
            Self::TableRow(value) => value.node_id,
            Self::TableCell(value) => value.node_id,
            Self::Block(value) => match value {
                Block::Paragraph { node_id, .. }
                | Block::Heading { node_id, .. }
                | Block::List { node_id, .. }
                | Block::Table { node_id, .. }
                | Block::Figure { node_id, .. }
                | Block::PageBreak { node_id, .. } => *node_id,
            },
            Self::Inline(value) => match value {
                Inline::Text { node_id, .. }
                | Inline::Emphasis { node_id, .. }
                | Inline::Strong { node_id, .. }
                | Inline::Link { node_id, .. }
                | Inline::Anchor { node_id, .. }
                | Inline::Reference { node_id, .. }
                | Inline::FootnoteReference { node_id, .. }
                | Inline::SoftBreak { node_id, .. }
                | Inline::HardBreak { node_id, .. } => *node_id,
            },
        }
    }

    fn source_span(self) -> SourceSpan {
        match self {
            Self::Footnote(value) => value.span,
            Self::ListItem(value) => value.span,
            Self::TableRow(value) => value.span,
            Self::TableCell(value) => value.span,
            Self::Block(value) => match value {
                Block::Paragraph { span, .. }
                | Block::Heading { span, .. }
                | Block::List { span, .. }
                | Block::Table { span, .. }
                | Block::Figure { span, .. }
                | Block::PageBreak { span, .. } => *span,
            },
            Self::Inline(value) => match value {
                Inline::Text { span, .. }
                | Inline::Emphasis { span, .. }
                | Inline::Strong { span, .. }
                | Inline::Link { span, .. }
                | Inline::Anchor { span, .. }
                | Inline::Reference { span, .. }
                | Inline::FootnoteReference { span, .. }
                | Inline::SoftBreak { span, .. }
                | Inline::HardBreak { span, .. } => *span,
            },
        }
    }
}

fn initial_domain_stack(document: &Document) -> Vec<LocatedDomainNode<'_>> {
    let mut stack = Vec::new();
    stack.extend(
        document
            .footnotes
            .iter()
            .rev()
            .map(LocatedDomainNode::Footnote),
    );
    stack.extend(document.blocks.iter().rev().map(LocatedDomainNode::Block));
    stack
}

fn push_domain_children<'a>(stack: &mut Vec<LocatedDomainNode<'a>>, node: LocatedDomainNode<'a>) {
    match node {
        LocatedDomainNode::Block(block) => match block {
            Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                stack.extend(children.iter().rev().map(LocatedDomainNode::Inline));
            }
            Block::List { items, .. } => {
                stack.extend(items.iter().rev().map(LocatedDomainNode::ListItem));
            }
            Block::Table { head, body, .. } => {
                stack.extend(body.iter().rev().map(LocatedDomainNode::TableRow));
                stack.extend(head.iter().rev().map(LocatedDomainNode::TableRow));
            }
            Block::Figure { caption, .. } => {
                stack.extend(caption.iter().rev().map(LocatedDomainNode::Block));
            }
            Block::PageBreak { .. } => {}
        },
        LocatedDomainNode::Inline(inline) => match inline {
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => {
                stack.extend(children.iter().rev().map(LocatedDomainNode::Inline));
            }
            Inline::Text { .. }
            | Inline::Anchor { .. }
            | Inline::Reference { .. }
            | Inline::FootnoteReference { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => {}
        },
        LocatedDomainNode::Footnote(footnote) => {
            stack.extend(footnote.blocks.iter().rev().map(LocatedDomainNode::Block));
        }
        LocatedDomainNode::ListItem(item) => {
            stack.extend(item.blocks.iter().rev().map(LocatedDomainNode::Block));
        }
        LocatedDomainNode::TableRow(row) => {
            stack.extend(row.cells.iter().rev().map(LocatedDomainNode::TableCell));
        }
        LocatedDomainNode::TableCell(cell) => {
            stack.extend(cell.blocks.iter().rev().map(LocatedDomainNode::Block));
        }
    }
}

fn locate_document_source_failure(
    package: &ParsedPackage,
    expected: &PackageValidationError,
    locations: &JsonLocationIndex,
) -> Option<(SourceSpan, JsonPointer)> {
    let mut stack = initial_domain_stack(&package.document);
    while let Some(node) = stack.pop() {
        let span = node.source_span();
        if validate_source_span(package, span).is_err_and(|error| &error == expected) {
            return Some((span, node_pointer(locations, node.node_id().get(), 0)));
        }
        push_domain_children(&mut stack, node);
    }
    None
}

fn locate_document_text_failure(
    package: &ParsedPackage,
    expected: &PackageValidationError,
    locations: &JsonLocationIndex,
) -> Option<JsonPointer> {
    let mut stack = initial_domain_stack(&package.document);
    while let Some(node) = stack.pop() {
        if let LocatedDomainNode::Inline(Inline::Text { text_span, .. }) = node {
            if validate_text_span(package, *text_span).is_err_and(|error| &error == expected) {
                return Some(node_pointer(locations, node.node_id().get(), 0).child("text_span"));
            }
        }
        push_domain_children(&mut stack, node);
    }
    None
}

fn canonical_list_marker_texts(
    document: &Document,
) -> Result<BTreeMap<NodeId, String>, PackageGeneratedTextError> {
    let mut markers = BTreeMap::new();
    let mut pending: Vec<&Block> = document
        .footnotes
        .iter()
        .flat_map(|footnote| footnote.blocks.iter())
        .chain(document.blocks.iter())
        .collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::List {
                ordered,
                start,
                items,
                ..
            } => {
                for (index, item) in items.iter().enumerate() {
                    let marker = if *ordered {
                        let start = start.ok_or(PackageGeneratedTextError::ListMarkerOverflow)?;
                        let offset = u32::try_from(index)
                            .map_err(|_| PackageGeneratedTextError::ListMarkerOverflow)?;
                        let value = start
                            .checked_add(offset)
                            .ok_or(PackageGeneratedTextError::ListMarkerOverflow)?;
                        format!("{value}.")
                    } else {
                        "\u{2022}".to_owned()
                    };
                    markers.insert(item.node_id, marker);
                    pending.extend(item.blocks.iter());
                }
            }
            Block::Table { head, body, .. } => {
                pending.extend(
                    head.iter()
                        .chain(body)
                        .flat_map(|row| row.cells.iter())
                        .flat_map(|cell| cell.blocks.iter()),
                );
            }
            Block::Figure { caption, .. } => pending.extend(caption),
            Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
        }
    }
    Ok(markers)
}

fn canonical_footnote_marker_texts(
    document: &Document,
    nodes: &ValidatedDocumentNodeIndex,
) -> Result<BTreeMap<GeneratedBufferKey, String>, PackageGeneratedTextError> {
    let catalog: BTreeMap<_, _> = document
        .footnotes
        .iter()
        .enumerate()
        .map(|(index, definition)| {
            let ordinal = index
                .checked_add(1)
                .ok_or(PackageGeneratedTextError::ArithmeticOverflow)?;
            Ok((definition.footnote_id.clone(), ordinal.to_string()))
        })
        .collect::<Result<_, PackageGeneratedTextError>>()?;
    let definitions: BTreeMap<_, _> = document
        .footnotes
        .iter()
        .map(|definition| (definition.node_id, &definition.footnote_id))
        .collect();
    nodes
        .generated_sites()
        .filter(|site| site.key().generation_kind() == GenerationKind::FootnoteMarker)
        .map(|site| {
            let footnote_id = match site.target() {
                GeneratedSiteTarget::Footnote(footnote_id) => footnote_id,
                GeneratedSiteTarget::None => definitions
                    .get(&site.key().owner())
                    .copied()
                    .ok_or(PackageGeneratedTextError::GeneratedStoreRejected)?,
                GeneratedSiteTarget::Anchor(_) => {
                    return Err(PackageGeneratedTextError::GeneratedStoreRejected)
                }
            };
            let marker = catalog
                .get(footnote_id)
                .cloned()
                .ok_or(PackageGeneratedTextError::GeneratedStoreRejected)?;
            Ok((site.key(), marker))
        })
        .collect()
}

fn find_styleable_block(
    document: &Document,
    owner: NodeId,
) -> Option<(NodeId, &'static str, &[String])> {
    let mut pending: Vec<&Block> = document
        .footnotes
        .iter()
        .rev()
        .flat_map(|footnote| footnote.blocks.iter().rev())
        .chain(document.blocks.iter().rev())
        .collect();
    while let Some(block) = pending.pop() {
        let (node_id, block_type) = match block {
            Block::Paragraph { node_id, .. } => (*node_id, "paragraph"),
            Block::Heading { node_id, .. } => (*node_id, "heading"),
            Block::List { node_id, .. } => (*node_id, "list"),
            Block::Table { node_id, .. } => (*node_id, "table"),
            Block::Figure { node_id, .. } => (*node_id, "figure"),
            Block::PageBreak { node_id, .. } => (*node_id, "page_break"),
        };
        if node_id == owner {
            return Some((node_id, block_type, block.classes()));
        }
        match block {
            Block::Paragraph { children, .. } | Block::Heading { children, .. }
                if inline_tree_contains_owner(children, owner) =>
            {
                return Some((node_id, block_type, block.classes()));
            }
            Block::List { items, .. } => {
                if items.iter().any(|item| item.node_id == owner) {
                    return Some((node_id, block_type, block.classes()));
                }
                for nested in items.iter().rev().flat_map(|item| item.blocks.iter().rev()) {
                    pending.push(nested);
                }
            }
            Block::Table { head, body, .. } => {
                if head.iter().chain(body).any(|row| row.node_id == owner) {
                    return Some((node_id, block_type, block.classes()));
                }
                for nested in body
                    .iter()
                    .rev()
                    .chain(head.iter().rev())
                    .flat_map(|row| row.cells.iter().rev())
                    .flat_map(|cell| cell.blocks.iter().rev())
                {
                    pending.push(nested);
                }
            }
            Block::Figure { caption, .. } => {
                pending.extend(caption.iter().rev());
            }
            Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
        }
    }
    None
}

fn find_basic_styleable_block(
    document: &Document,
    owner: NodeId,
) -> Option<(NodeId, &'static str, &[String], Option<NodeId>)> {
    let mut pending: Vec<(&Block, Option<NodeId>)> = document
        .blocks
        .iter()
        .rev()
        .map(|block| (block, None))
        .collect();
    while let Some((block, flow_parent)) = pending.pop() {
        let (node_id, block_type) = match block {
            Block::Paragraph { node_id, .. } => (*node_id, "paragraph"),
            Block::Heading { node_id, .. } => (*node_id, "heading"),
            Block::List { node_id, .. } => (*node_id, "list"),
            Block::Table { node_id, .. } => (*node_id, "table"),
            Block::Figure { node_id, .. } => (*node_id, "figure"),
            Block::PageBreak { node_id, .. } => (*node_id, "page_break"),
        };
        if node_id == owner {
            return Some((node_id, block_type, block.classes(), flow_parent));
        }
        match block {
            Block::Paragraph { children, .. } | Block::Heading { children, .. }
                if inline_tree_contains_owner(children, owner) =>
            {
                return Some((node_id, block_type, block.classes(), flow_parent));
            }
            Block::List { items, .. } => {
                for nested in items.iter().rev().flat_map(|item| item.blocks.iter().rev()) {
                    pending.push((nested, Some(node_id)));
                }
            }
            Block::Figure { caption, .. } => {
                pending.extend(caption.iter().rev().map(|nested| (nested, Some(node_id))));
            }
            Block::Table { head, body, .. } => {
                for nested in body
                    .iter()
                    .rev()
                    .chain(head.iter().rev())
                    .flat_map(|row| row.cells.iter().rev())
                    .flat_map(|cell| cell.blocks.iter().rev())
                {
                    pending.push((nested, Some(node_id)));
                }
            }
            Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
        }
    }
    None
}

fn inline_tree_contains_owner(inlines: &[Inline], owner: NodeId) -> bool {
    let mut pending: Vec<&Inline> = inlines.iter().rev().collect();
    while let Some(inline) = pending.pop() {
        let (node_id, children) = match inline {
            Inline::Text { node_id, .. }
            | Inline::Anchor { node_id, .. }
            | Inline::Reference { node_id, .. }
            | Inline::FootnoteReference { node_id, .. }
            | Inline::SoftBreak { node_id, .. }
            | Inline::HardBreak { node_id, .. } => (*node_id, None),
            Inline::Emphasis {
                node_id, children, ..
            }
            | Inline::Strong {
                node_id, children, ..
            }
            | Inline::Link {
                node_id, children, ..
            } => (*node_id, Some(children.as_slice())),
        };
        if node_id == owner {
            return true;
        }
        if let Some(children) = children {
            pending.extend(children.iter().rev());
        }
    }
    false
}

fn parsed_shape_owners(
    document: &Document,
    requested: TextSpan,
) -> Result<(NodeId, NodeId, TextSpan, bool), PackageShapeTextError> {
    let mut matched = None;
    let mut pending: Vec<&Block> = document
        .footnotes
        .iter()
        .rev()
        .flat_map(|footnote| footnote.blocks.iter().rev())
        .chain(document.blocks.iter().rev())
        .collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::Paragraph {
                node_id: style_owner,
                children,
                ..
            }
            | Block::Heading {
                node_id: style_owner,
                children,
                ..
            } => {
                let mut inlines: Vec<&Inline> = children.iter().rev().collect();
                while let Some(inline) = inlines.pop() {
                    match inline {
                        Inline::Text {
                            node_id: site_owner,
                            text_span,
                            ..
                        } if text_span_contains(*text_span, requested) => {
                            if matched.is_some() {
                                return Err(PackageShapeTextError::AmbiguousParsedSpan);
                            }
                            matched = Some((
                                *site_owner,
                                *style_owner,
                                *text_span,
                                inline_logical_site_count(children) == 1,
                            ));
                        }
                        Inline::Emphasis { children, .. }
                        | Inline::Strong { children, .. }
                        | Inline::Link { children, .. } => {
                            inlines.extend(children.iter().rev());
                        }
                        Inline::Text { .. }
                        | Inline::Anchor { .. }
                        | Inline::Reference { .. }
                        | Inline::FootnoteReference { .. }
                        | Inline::SoftBreak { .. }
                        | Inline::HardBreak { .. } => {}
                    }
                }
            }
            Block::List { items, .. } => {
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Table { head, body, .. } => {
                pending.extend(
                    body.iter()
                        .rev()
                        .chain(head.iter().rev())
                        .flat_map(|row| row.cells.iter().rev())
                        .flat_map(|cell| cell.blocks.iter().rev()),
                );
            }
            Block::Figure { caption, .. } => pending.extend(caption.iter().rev()),
            Block::PageBreak { .. } => {}
        }
    }
    matched.ok_or(PackageShapeTextError::UnownedParsedSpan)
}

fn inline_logical_site_count(inlines: &[Inline]) -> usize {
    let mut count = 0usize;
    let mut pending: Vec<&Inline> = inlines.iter().rev().collect();
    while let Some(inline) = pending.pop() {
        match inline {
            Inline::Text { .. }
            | Inline::Reference { .. }
            | Inline::FootnoteReference { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => {
                if count == 1 {
                    return 2;
                }
                count = 1;
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => pending.extend(children.iter().rev()),
            Inline::Anchor { .. } => {}
        }
    }
    count
}

fn paragraph_inline_children(document: &Document, owner: NodeId) -> Option<&[Inline]> {
    let mut pending: Vec<&Block> = document
        .footnotes
        .iter()
        .rev()
        .flat_map(|footnote| footnote.blocks.iter().rev())
        .chain(document.blocks.iter().rev())
        .collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::Paragraph {
                node_id, children, ..
            }
            | Block::Heading {
                node_id, children, ..
            } if *node_id == owner => return Some(children),
            Block::Paragraph { .. } | Block::Heading { .. } => {}
            Block::List { items, .. } => {
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Table { head, body, .. } => {
                pending.extend(
                    body.iter()
                        .rev()
                        .chain(head.iter().rev())
                        .flat_map(|row| row.cells.iter().rev())
                        .flat_map(|cell| cell.blocks.iter().rev()),
                );
            }
            Block::Figure { caption, .. } => pending.extend(caption.iter().rev()),
            Block::PageBreak { .. } => {}
        }
    }
    None
}

fn collect_shape_text_site_identities(
    inlines: &[Inline],
    output: &mut Vec<PackageParagraphTextSite>,
) {
    for inline in inlines {
        match inline {
            Inline::Text { text_span, .. } => {
                output.push(PackageParagraphTextSite::Parsed(*text_span));
            }
            Inline::Reference {
                node_id, format, ..
            } => {
                let generation_kind = match format {
                    ReferenceFormat::Page => GenerationKind::PageReference,
                    ReferenceFormat::Text | ReferenceFormat::Number => GenerationKind::Counter,
                };
                output.push(PackageParagraphTextSite::Generated(
                    GeneratedBufferKey::new(*node_id, generation_kind, 0),
                ));
            }
            Inline::FootnoteReference { node_id, .. } => {
                output.push(PackageParagraphTextSite::Generated(
                    GeneratedBufferKey::new(*node_id, GenerationKind::FootnoteMarker, 0),
                ));
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => {
                collect_shape_text_site_identities(children, output);
            }
            Inline::Anchor { .. } | Inline::SoftBreak { .. } | Inline::HardBreak { .. } => {}
        }
    }
}

fn generated_inline_site_is_standalone(document: &Document, owner: NodeId) -> bool {
    let mut pending: Vec<&Block> = document
        .footnotes
        .iter()
        .rev()
        .flat_map(|footnote| footnote.blocks.iter().rev())
        .chain(document.blocks.iter().rev())
        .collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                if inline_tree_contains_owner(children, owner) {
                    return inline_logical_site_count(children) == 1;
                }
            }
            Block::List { items, .. } => {
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Table { head, body, .. } => {
                pending.extend(
                    body.iter()
                        .rev()
                        .chain(head.iter().rev())
                        .flat_map(|row| row.cells.iter().rev())
                        .flat_map(|cell| cell.blocks.iter().rev()),
                );
            }
            Block::Figure { caption, .. } => pending.extend(caption.iter().rev()),
            Block::PageBreak { .. } => {}
        }
    }
    false
}

fn text_span_contains(container: TextSpan, requested: TextSpan) -> bool {
    container.text_id() == requested.text_id()
        && container.start_byte().get() <= requested.start_byte().get()
        && requested.end_byte().get() <= container.end_byte().get()
}

fn shape_style_owner(
    document: &Document,
    index: &ValidatedDocumentNodeIndex,
    site_owner: NodeId,
) -> Option<NodeId> {
    let site_path = index.node_path(site_owner)?;
    if index.node_kind(site_owner) == Some(DocumentNodeKind::FootnoteDefinition) {
        return document
            .footnotes
            .iter()
            .find(|definition| definition.node_id == site_owner)
            .and_then(|definition| {
                definition.blocks.iter().find_map(|block| match block {
                    Block::Paragraph {
                        node_id, children, ..
                    }
                    | Block::Heading {
                        node_id, children, ..
                    } if definition_inlines_produce_text(children) => Some(*node_id),
                    _ => None,
                })
            });
    }
    index
        .nodes()
        .filter(|(candidate, kind)| {
            matches!(
                kind,
                DocumentNodeKind::Paragraph
                    | DocumentNodeKind::Heading
                    | DocumentNodeKind::List
                    | DocumentNodeKind::Table
                    | DocumentNodeKind::Figure
                    | DocumentNodeKind::PageBreak
            ) && index
                .node_path(*candidate)
                .is_some_and(|path| site_path.starts_with(path))
        })
        .max_by_key(|(candidate, _)| index.node_path(*candidate).map(<[u32]>::len))
        .map(|(owner, _)| owner)
}

fn definition_inlines_produce_text(inlines: &[Inline]) -> bool {
    inlines.iter().any(|inline| match inline {
        Inline::Text { text_span, .. } => text_span.start_byte() < text_span.end_byte(),
        Inline::Reference { .. } => true,
        Inline::Emphasis { children, .. }
        | Inline::Strong { children, .. }
        | Inline::Link { children, .. } => definition_inlines_produce_text(children),
        Inline::Anchor { .. }
        | Inline::FootnoteReference { .. }
        | Inline::SoftBreak { .. }
        | Inline::HardBreak { .. } => false,
    })
}

#[cfg(test)]
fn contains_include_directive(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'@' {
            let mut keyword = index + 1;
            while keyword < bytes.len() && bytes[keyword].is_ascii_whitespace() {
                keyword += 1;
            }
            let Some(end) = keyword.checked_add(b"include".len()) else {
                return false;
            };
            let keyword_boundary = match bytes.get(end) {
                Some(byte) => !byte.is_ascii_alphanumeric() && *byte != b'_',
                None => true,
            };
            if bytes.get(keyword..end) == Some(b"include") && keyword_boundary {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn validate_package_limits(
    package: &ParsedPackage,
    policy: &PackageValidationPolicy<'_>,
) -> Result<u64, PackageValidationError> {
    let limits = policy.limits.get();
    let include_count = package
        .sources
        .records()
        .len()
        .checked_sub(1)
        .ok_or(PackageValidationError::MissingEntrySource)?;
    if include_count > limits.max_include_files as usize {
        return Err(PackageValidationError::IncludeFileLimit);
    }
    let mut input_bytes = 0u64;
    for source in package.sources.records() {
        if source.utf8_byte_length() > limits.max_source_bytes {
            return Err(PackageValidationError::SourceByteLimit);
        }
        input_bytes = input_bytes
            .checked_add(u64::from(source.utf8_byte_length()))
            .ok_or(PackageValidationError::InputByteLimit)?;
    }
    if input_bytes > limits.max_input_bytes {
        return Err(PackageValidationError::InputByteLimit);
    }
    let style_rule_count = u64::try_from(package.style_sheet.rules.len())
        .map_err(|_| PackageValidationError::StyleRuleLimit)?;
    if style_rule_count > limits.max_style_rules {
        return Err(PackageValidationError::StyleRuleLimit);
    }
    let mut text_bytes = 0u64;
    for buffer in package.text_store.buffers() {
        if buffer.byte_len() > limits.max_text_buffer_bytes {
            return Err(PackageValidationError::TextBufferByteLimit);
        }
        text_bytes = text_bytes
            .checked_add(u64::from(buffer.byte_len()))
            .ok_or(PackageValidationError::TextByteLimit)?;
    }
    if text_bytes > limits.max_text_bytes {
        return Err(PackageValidationError::TextByteLimit);
    }
    let declaration_count = package
        .style_sheet
        .rules
        .iter()
        .try_fold(0u64, |count, rule| {
            let declarations = u64::try_from(rule.declarations.len()).ok()?;
            count.checked_add(declarations)
        })
        .ok_or(PackageValidationError::AstNodeLimit)?;
    let non_document_ast_nodes = declaration_count
        .checked_mul(2)
        .ok_or(PackageValidationError::AstNodeLimit)?;
    if non_document_ast_nodes > limits.max_ast_nodes {
        return Err(PackageValidationError::AstNodeLimit);
    }
    Ok(non_document_ast_nodes)
}

#[derive(Clone, Copy)]
enum AstPrecheckNode<'a> {
    Document(&'a Document),
    Block(&'a Block),
    Inline(&'a Inline),
    Footnote(&'a FootnoteDefinition),
    ListItem(&'a ListItem),
    TableRow(&'a TableRow),
    TableCell(&'a TableCell),
}

fn push_ast_precheck_node<'a>(
    stack: &mut Vec<(AstPrecheckNode<'a>, u32)>,
    observed_nodes: &mut u64,
    limits: &ValidatedResourceLimits,
    node: AstPrecheckNode<'a>,
    depth: u32,
) -> Result<(), PackageValidationError> {
    if depth > limits.get().max_ast_nesting_depth {
        return Err(PackageValidationError::AstNestingDepthLimit);
    }
    let next_observed = observed_nodes
        .checked_add(1)
        .ok_or(PackageValidationError::AstNodeLimit)?;
    if next_observed > limits.get().max_ast_nodes {
        return Err(PackageValidationError::AstNodeLimit);
    }
    *observed_nodes = next_observed;
    stack.push((node, depth));
    Ok(())
}

/// Performs the depth and node-count checks iteratively before any recursive
/// validation, indexing, or fingerprint traversal can observe the document.
fn validate_document_ast_limits(
    document: &Document,
    limits: &ValidatedResourceLimits,
    non_document_ast_nodes: u64,
) -> Result<(), PackageValidationError> {
    let mut observed_nodes = non_document_ast_nodes;
    let mut stack = Vec::new();
    push_ast_precheck_node(
        &mut stack,
        &mut observed_nodes,
        limits,
        AstPrecheckNode::Document(document),
        1,
    )?;

    while let Some((node, depth)) = stack.pop() {
        let child_depth = depth
            .checked_add(1)
            .ok_or(PackageValidationError::AstNestingDepthLimit)?;
        let mut push = |node| {
            push_ast_precheck_node(&mut stack, &mut observed_nodes, limits, node, child_depth)
        };
        match node {
            AstPrecheckNode::Document(document) => {
                for footnote in document.footnotes.iter().rev() {
                    push(AstPrecheckNode::Footnote(footnote))?;
                }
                for block in document.blocks.iter().rev() {
                    push(AstPrecheckNode::Block(block))?;
                }
            }
            AstPrecheckNode::Block(block) => match block {
                Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                    for inline in children.iter().rev() {
                        push(AstPrecheckNode::Inline(inline))?;
                    }
                }
                Block::List { items, .. } => {
                    for item in items.iter().rev() {
                        push(AstPrecheckNode::ListItem(item))?;
                    }
                }
                Block::Table { head, body, .. } => {
                    for row in body.iter().rev() {
                        push(AstPrecheckNode::TableRow(row))?;
                    }
                    for row in head.iter().rev() {
                        push(AstPrecheckNode::TableRow(row))?;
                    }
                }
                Block::Figure { caption, .. } => {
                    for block in caption.iter().rev() {
                        push(AstPrecheckNode::Block(block))?;
                    }
                }
                Block::PageBreak { .. } => {}
            },
            AstPrecheckNode::Inline(inline) => match inline {
                Inline::Emphasis { children, .. }
                | Inline::Strong { children, .. }
                | Inline::Link { children, .. } => {
                    for inline in children.iter().rev() {
                        push(AstPrecheckNode::Inline(inline))?;
                    }
                }
                Inline::Text { .. }
                | Inline::Anchor { .. }
                | Inline::Reference { .. }
                | Inline::FootnoteReference { .. }
                | Inline::SoftBreak { .. }
                | Inline::HardBreak { .. } => {}
            },
            AstPrecheckNode::Footnote(footnote) => {
                for block in footnote.blocks.iter().rev() {
                    push(AstPrecheckNode::Block(block))?;
                }
            }
            AstPrecheckNode::ListItem(item) => {
                for block in item.blocks.iter().rev() {
                    push(AstPrecheckNode::Block(block))?;
                }
            }
            AstPrecheckNode::TableRow(row) => {
                for cell in row.cells.iter().rev() {
                    push(AstPrecheckNode::TableCell(cell))?;
                }
            }
            AstPrecheckNode::TableCell(cell) => {
                for block in cell.blocks.iter().rev() {
                    push(AstPrecheckNode::Block(block))?;
                }
            }
        }
    }
    Ok(())
}

/// Bounds the otherwise potentially quadratic parent-chain walks performed by
/// style validation and cascade. Duplicate, unknown, and cyclic graphs are
/// deliberately left to `StyleSheet::validate` so their specific errors win.
fn validate_style_inheritance_depth(
    style_sheet: &StyleSheet,
    limits: &ValidatedResourceLimits,
) -> Result<(), PackageValidationError> {
    let mut by_id = BTreeMap::new();
    for (index, rule) in style_sheet.rules.iter().enumerate() {
        if by_id.insert(&rule.style_id, index).is_some() {
            return Ok(());
        }
    }

    let mut state = vec![0u8; style_sheet.rules.len()];
    let mut depths = vec![0u32; style_sheet.rules.len()];
    for start in 0..style_sheet.rules.len() {
        if state[start] == 2 {
            continue;
        }
        let mut path = Vec::new();
        let mut current = Some(start);
        let base_depth = loop {
            let Some(index) = current else { break 0 };
            match state[index] {
                0 => {
                    state[index] = 1;
                    path.push(index);
                    current = match style_sheet.rules[index].extends.as_ref() {
                        Some(parent) => match by_id.get(parent) {
                            Some(parent_index) => Some(*parent_index),
                            None => return Ok(()),
                        },
                        None => None,
                    };
                }
                1 => return Ok(()),
                2 => break depths[index],
                _ => unreachable!("private style traversal state"),
            }
        };

        let mut depth = base_depth;
        for index in path.into_iter().rev() {
            depth = depth
                .checked_add(1)
                .ok_or(PackageValidationError::AstNestingDepthLimit)?;
            if depth > limits.get().max_ast_nesting_depth {
                return Err(PackageValidationError::AstNestingDepthLimit);
            }
            depths[index] = depth;
            state[index] = 2;
        }
    }
    Ok(())
}

fn validate_source_span(
    package: &ParsedPackage,
    span: SourceSpan,
) -> Result<&typaxis_text::SourceRecord, PackageValidationError> {
    let source = package
        .sources
        .get(span.source_id())
        .ok_or(PackageValidationError::UnknownSource)?;
    if span.end_byte().get() > source.utf8_byte_length() {
        return Err(PackageValidationError::SourceSpanOutOfBounds);
    }
    let start = span.start_byte().get() as usize;
    let end = span.end_byte().get() as usize;
    if !source.utf8().is_char_boundary(start) || !source.utf8().is_char_boundary(end) {
        return Err(PackageValidationError::SourceSpanNotUtf8Boundary);
    }
    Ok(source)
}

fn validate_text_span(
    package: &ParsedPackage,
    span: TextSpan,
) -> Result<(), PackageValidationError> {
    let buffer = package
        .text_store
        .get(span.text_id())
        .ok_or(PackageValidationError::UnknownTextBuffer)?;
    if span.end_byte().get() > buffer.byte_len() {
        return Err(PackageValidationError::TextSpanOutOfBounds);
    }
    if !buffer.is_boundary(span.start_byte()) || !buffer.is_boundary(span.end_byte()) {
        return Err(PackageValidationError::TextSpanNotUtf8Boundary);
    }
    Ok(())
}

fn validate_resource_catalog(
    resources: &ResourceCatalog,
) -> Result<BTreeSet<ImageResourceId>, PackageValidationError> {
    let mut font_ids = BTreeSet::<FontFaceId>::new();
    let mut families = BTreeSet::new();
    for (index, font) in resources.font_faces.iter().enumerate() {
        if font.font_face_id.get()
            != u32::try_from(index).map_err(|_| PackageValidationError::NonCanonicalFontFaceId)?
        {
            return Err(PackageValidationError::NonCanonicalFontFaceId);
        }
        if !font_ids.insert(font.font_face_id) {
            return Err(PackageValidationError::DuplicateFontFaceId);
        }
        if font.family.trim().is_empty() || font.family.chars().any(char::is_control) {
            return Err(PackageValidationError::InvalidFontFamily);
        }
        if !families.insert(font.family.as_str()) {
            return Err(PackageValidationError::DuplicateFontFamily);
        }
    }
    let mut image_ids = BTreeSet::new();
    for (index, image) in resources.images.iter().enumerate() {
        if image.image_id.get()
            != u32::try_from(index).map_err(|_| PackageValidationError::NonCanonicalImageId)?
        {
            return Err(PackageValidationError::NonCanonicalImageId);
        }
        if !image_ids.insert(image.image_id) {
            return Err(PackageValidationError::DuplicateImageId);
        }
    }
    Ok(image_ids)
}

struct DocumentValidator<'a> {
    package: &'a ParsedPackage,
    known_images: &'a BTreeSet<ImageResourceId>,
    node_ids: BTreeSet<NodeId>,
    anchors: BTreeSet<AnchorId>,
    footnote_ids: BTreeSet<FootnoteId>,
    internal_targets: Vec<AnchorId>,
    footnote_targets: Vec<FootnoteId>,
    image_targets: Vec<ImageResourceId>,
    policy: &'a PackageValidationPolicy<'a>,
    next_node_id: u32,
    non_document_ast_nodes: u64,
    defer_list_marker_overflow: bool,
}

fn validate_document(
    package: &ParsedPackage,
    known_images: &BTreeSet<ImageResourceId>,
    policy: &PackageValidationPolicy<'_>,
    non_document_ast_nodes: u64,
    defer_list_marker_overflow: bool,
) -> Result<(), PackageValidationError> {
    let mut validator = DocumentValidator {
        package,
        known_images,
        node_ids: BTreeSet::new(),
        anchors: BTreeSet::new(),
        footnote_ids: BTreeSet::new(),
        internal_targets: vec![],
        footnote_targets: vec![],
        image_targets: vec![],
        policy,
        next_node_id: 0,
        non_document_ast_nodes,
        defer_list_marker_overflow,
    };
    validator.node(package.document.node_id)?;
    for block in &package.document.blocks {
        validator.block(block)?;
    }
    let mut previous_footnote: Option<&FootnoteId> = None;
    for footnote in &package.document.footnotes {
        if previous_footnote.is_some_and(|previous| previous >= &footnote.footnote_id) {
            return Err(PackageValidationError::NonCanonicalFootnoteOrder);
        }
        validator.footnote(footnote)?;
        previous_footnote = Some(&footnote.footnote_id);
    }
    if validator
        .internal_targets
        .iter()
        .any(|target| !validator.anchors.contains(target))
    {
        return Err(PackageValidationError::UnknownInternalTarget);
    }
    if validator
        .footnote_targets
        .iter()
        .any(|target| !validator.footnote_ids.contains(target))
    {
        return Err(PackageValidationError::UnknownFootnoteTarget);
    }
    if validator
        .image_targets
        .iter()
        .any(|target| !validator.known_images.contains(target))
    {
        return Err(PackageValidationError::UnknownImageTarget);
    }
    Ok(())
}

impl DocumentValidator<'_> {
    fn node(&mut self, node_id: NodeId) -> Result<(), PackageValidationError> {
        if !self.node_ids.insert(node_id) {
            return Err(PackageValidationError::DuplicateNodeId);
        }
        if node_id.get() != self.next_node_id {
            return Err(PackageValidationError::NonCanonicalNodeId);
        }
        let total_after_insert = self
            .non_document_ast_nodes
            .checked_add(u64::from(self.next_node_id))
            .and_then(|value| value.checked_add(1))
            .ok_or(PackageValidationError::AstNodeLimit)?;
        if total_after_insert > self.policy.limits.get().max_ast_nodes {
            return Err(PackageValidationError::AstNodeLimit);
        }
        self.next_node_id = self
            .next_node_id
            .checked_add(1)
            .ok_or(PackageValidationError::AstNodeLimit)?;
        Ok(())
    }

    fn source_node(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
    ) -> Result<(), PackageValidationError> {
        self.node(node_id)?;
        validate_source_span(self.package, span)?;
        Ok(())
    }

    fn classes(&self, classes: &[String]) -> Result<(), PackageValidationError> {
        let mut previous: Option<&str> = None;
        for class in classes {
            if !is_style_identifier(class) {
                return Err(PackageValidationError::InvalidBlockClass);
            }
            if previous == Some(class) {
                return Err(PackageValidationError::DuplicateBlockClass);
            }
            if previous.is_some_and(|value| value > class.as_str()) {
                return Err(PackageValidationError::NonCanonicalBlockClasses);
            }
            previous = Some(class);
        }
        Ok(())
    }

    fn anchor(&mut self, anchor_id: &AnchorId) -> Result<(), PackageValidationError> {
        if !self.anchors.insert(anchor_id.clone()) {
            return Err(PackageValidationError::DuplicateAnchorId);
        }
        Ok(())
    }

    fn block(&mut self, block: &Block) -> Result<(), PackageValidationError> {
        match block {
            Block::Paragraph {
                node_id,
                span,
                classes,
                children,
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)?;
                self.inlines(children)
            }
            Block::Heading {
                node_id,
                span,
                classes,
                anchor_id,
                children,
                ..
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)?;
                if let Some(anchor_id) = anchor_id {
                    self.anchor(anchor_id)?;
                }
                self.inlines(children)
            }
            Block::List {
                node_id,
                span,
                classes,
                ordered,
                start,
                items,
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)?;
                // The parser resolves an omitted ordered-list source value to
                // `Some(1)` before constructing the canonical package.
                if (*ordered && start.map_or(true, |value| value == 0))
                    || (!*ordered && start.is_some())
                {
                    return Err(PackageValidationError::InvalidListStart);
                }
                if items.is_empty() {
                    return Err(PackageValidationError::EmptyListItems);
                }
                if *ordered && !self.defer_list_marker_overflow {
                    let last_offset = u32::try_from(items.len() - 1)
                        .map_err(|_| PackageValidationError::ListMarkerOverflow)?;
                    start
                        .and_then(|start| start.checked_add(last_offset))
                        .ok_or(PackageValidationError::ListMarkerOverflow)?;
                }
                for item in items {
                    self.list_item(item)?;
                }
                Ok(())
            }
            Block::Table {
                node_id,
                span,
                classes,
                columns,
                head,
                body,
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)?;
                if columns.is_empty() {
                    return Err(PackageValidationError::EmptyTableColumns);
                }
                if head.is_empty() && body.is_empty() {
                    return Err(PackageValidationError::EmptyTableRows);
                }
                validate_table_grid(columns.len(), head, body)?;
                for row in head.iter().chain(body) {
                    self.table_row(row)?;
                }
                Ok(())
            }
            Block::Figure {
                node_id,
                span,
                classes,
                image_id,
                caption,
                ..
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)?;
                self.image_targets.push(*image_id);
                for block in caption {
                    self.block(block)?;
                }
                Ok(())
            }
            Block::PageBreak {
                node_id,
                span,
                classes,
            } => {
                self.source_node(*node_id, *span)?;
                self.classes(classes)
            }
        }
    }

    fn inlines(&mut self, inlines: &[Inline]) -> Result<(), PackageValidationError> {
        for inline in inlines {
            self.inline(inline)?;
        }
        Ok(())
    }

    fn inline(&mut self, inline: &Inline) -> Result<(), PackageValidationError> {
        match inline {
            Inline::Text {
                node_id,
                span,
                text_span,
            } => {
                self.source_node(*node_id, *span)?;
                validate_text_span(self.package, *text_span)
            }
            Inline::Emphasis {
                node_id,
                span,
                children,
            }
            | Inline::Strong {
                node_id,
                span,
                children,
            } => {
                self.source_node(*node_id, *span)?;
                self.inlines(children)
            }
            Inline::Link {
                node_id,
                span,
                target,
                children,
            } => {
                self.source_node(*node_id, *span)?;
                match target {
                    LinkTarget::Internal(target) => self.internal_targets.push(target.clone()),
                    LinkTarget::Uri(uri) => {
                        let schemes: Vec<&str> = self
                            .policy
                            .allowed_uri_schemes
                            .iter()
                            .map(String::as_str)
                            .collect();
                        uri.validate_policy(
                            &schemes,
                            self.policy.limits.get().max_uri_bytes as usize,
                        )
                        .map_err(PackageValidationError::InvalidUri)?;
                    }
                }
                self.inlines(children)
            }
            Inline::Anchor {
                node_id,
                span,
                anchor_id,
            } => {
                self.source_node(*node_id, *span)?;
                self.anchor(anchor_id)
            }
            Inline::Reference {
                node_id,
                span,
                target,
                ..
            } => {
                self.source_node(*node_id, *span)?;
                self.internal_targets.push(target.clone());
                Ok(())
            }
            Inline::FootnoteReference {
                node_id,
                span,
                footnote_id,
            } => {
                self.source_node(*node_id, *span)?;
                self.footnote_targets.push(footnote_id.clone());
                Ok(())
            }
            Inline::SoftBreak { node_id, span } | Inline::HardBreak { node_id, span } => {
                self.source_node(*node_id, *span)
            }
        }
    }

    fn list_item(&mut self, item: &ListItem) -> Result<(), PackageValidationError> {
        self.source_node(item.node_id, item.span)?;
        for block in &item.blocks {
            self.block(block)?;
        }
        Ok(())
    }

    fn table_row(&mut self, row: &TableRow) -> Result<(), PackageValidationError> {
        self.source_node(row.node_id, row.span)?;
        for cell in &row.cells {
            self.table_cell(cell)?;
        }
        Ok(())
    }

    fn table_cell(&mut self, cell: &TableCell) -> Result<(), PackageValidationError> {
        self.source_node(cell.node_id, cell.span)?;
        for block in &cell.blocks {
            self.block(block)?;
        }
        Ok(())
    }

    fn footnote(&mut self, footnote: &FootnoteDefinition) -> Result<(), PackageValidationError> {
        if !self.footnote_ids.insert(footnote.footnote_id.clone()) {
            return Err(PackageValidationError::DuplicateFootnoteId);
        }
        self.source_node(footnote.node_id, footnote.span)?;
        for block in &footnote.blocks {
            self.block(block)?;
        }
        Ok(())
    }
}

fn validate_table_grid(
    column_count: usize,
    head: &[TableRow],
    body: &[TableRow],
) -> Result<(), PackageValidationError> {
    let row_count = head
        .len()
        .checked_add(body.len())
        .ok_or(PackageValidationError::InvalidTableGrid)?;
    let mut occupied_rows = vec![0usize; column_count];
    for (row_index, row) in head.iter().chain(body).enumerate() {
        for cell in &row.cells {
            let column_index = occupied_rows
                .iter()
                .position(|remaining| *remaining == 0)
                .ok_or(PackageValidationError::InvalidTableGrid)?;
            let colspan = usize::from(cell.colspan.get());
            let rowspan = usize::from(cell.rowspan.get());
            let column_end = column_index
                .checked_add(colspan)
                .ok_or(PackageValidationError::InvalidTableGrid)?;
            let row_end = row_index
                .checked_add(rowspan)
                .ok_or(PackageValidationError::InvalidTableGrid)?;
            if column_end > column_count
                || row_end > row_count
                || occupied_rows[column_index..column_end]
                    .iter()
                    .any(|remaining| *remaining != 0)
            {
                return Err(PackageValidationError::InvalidTableGrid);
            }
            if row_index < head.len() && row_end > head.len() {
                return Err(PackageValidationError::TableHeadBodyCross);
            }
            occupied_rows[column_index..column_end].fill(rowspan);
        }
        if occupied_rows.contains(&0) {
            return Err(PackageValidationError::InvalidTableGrid);
        }
        for remaining in &mut occupied_rows {
            *remaining -= 1;
        }
    }
    if occupied_rows.iter().any(|remaining| *remaining != 0) {
        return Err(PackageValidationError::InvalidTableGrid);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseOutcome {
    Parsed {
        package: Box<ValidatedParsedPackage>,
        diagnostics: Vec<AdvisoryDiagnostic>,
    },
    Failed {
        failure: ParseFailure,
    },
}
mod parser_seal {
    pub trait Sealed {}
}

/// Trusted parsers are implemented inside this crate; downstream code cannot
/// implement the trait and inject a caller-built AST into `ParseOutcome`.
pub trait Parser: parser_seal::Sealed {
    fn parse(&self, source: &SourceFile, policy: &PackageValidationPolicy<'_>) -> ParseOutcome;
}

/// Small source-driven parser used by the reference workspace to exercise
/// downstream trust boundaries. It accepts only empty lines, `paragraph`,
/// `font:<family>:<portable-path>`, `anchor:<id>`, `reference:<id>`,
/// `soft_break`, `hard_break`, `text:<utf8>`, and
/// `inlines:text=<utf8>|reference=<id>|anchor=<id>` records. The resulting
/// AST, node IDs, spans, text maps, style/resource tables, and default page
/// master are all derived inside this crate; callers never supply a
/// `ParsedPackage`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReferenceParser;
impl ReferenceParser {
    pub const fn new() -> Self {
        Self
    }
}
impl parser_seal::Sealed for ReferenceParser {}
impl Parser for ReferenceParser {
    fn parse(&self, source: &SourceFile, policy: &PackageValidationPolicy<'_>) -> ParseOutcome {
        let package = match parse_reference_entry(source) {
            Ok(package) => package,
            Err(message) => return reference_parse_failure(source, message),
        };
        let include_graph = match ValidatedIncludeGraph::entry_only(&package.sources, policy.limits)
        {
            Ok(include_graph) => include_graph,
            Err(_) => return reference_parse_failure(source, "entry include closure was rejected"),
        };
        match ValidatedParsedPackage::new_resolved(package, policy, &include_graph, |_, error| {
            error
        }) {
            Ok(package) => ParseOutcome::Parsed {
                package: Box::new(package),
                diagnostics: vec![],
            },
            Err(_) => reference_parse_failure(source, "reference source failed package validation"),
        }
    }
}

fn parse_reference_entry(source: &SourceFile) -> Result<ParsedPackage, &'static str> {
    if source.source_id != SourceId::new(0) {
        return Err("entry SourceId must be zero");
    }
    let source_record =
        SourceRecord::new(source.source_id, source.uri.clone(), source.text.clone())
            .map_err(|_| "entry source is too large")?;
    let sources = SourceCatalog::new(vec![source_record])
        .map_err(|_| "entry source catalog is not canonical")?;
    let mut blocks = Vec::new();
    let mut text_buffers = Vec::new();
    let mut font_faces = Vec::new();
    let mut next_node = 1u32;
    let mut start = 0usize;
    for raw_line in source.text.split_inclusive('\n') {
        let without_lf = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let line = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        let end = start
            .checked_add(line.len())
            .ok_or("source span overflow")?;
        if !line.is_empty() {
            if let Some(declaration) = line.strip_prefix("font:") {
                let (family, uri) = declaration
                    .split_once(':')
                    .ok_or("font record must contain family and portable path")?;
                if family.trim().is_empty() || family.chars().any(char::is_control) {
                    return Err("font family is invalid");
                }
                font_faces.push(FontFaceDeclaration {
                    font_face_id: FontFaceId::new(
                        u32::try_from(font_faces.len()).map_err(|_| "font ID overflow")?,
                    ),
                    family: family.to_owned(),
                    uri: PortablePath::new(uri).map_err(|_| "font path is invalid")?,
                    face_index: 0,
                    expected_sha256: None,
                });
                start = start
                    .checked_add(raw_line.len())
                    .ok_or("source span overflow")?;
                continue;
            }
            let start_byte = u32::try_from(start).map_err(|_| "source span overflow")?;
            let end_byte = u32::try_from(end).map_err(|_| "source span overflow")?;
            let span = SourceSpan::new(
                SourceId::new(0),
                Utf8ByteOffset::new(start_byte),
                Utf8ByteOffset::new(end_byte),
            )
            .ok_or("source span is invalid")?;
            let paragraph_id = NodeId::new(next_node);
            next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
            let children = if let Some(sequence) = line.strip_prefix("inlines:") {
                parse_reference_inline_sequence(sequence, start, &mut next_node, &mut text_buffers)?
            } else if let Some(anchor) = line.strip_prefix("anchor:") {
                let anchor_id = AnchorId::new(anchor).map_err(|_| "anchor ID is invalid")?;
                let anchor_node = NodeId::new(next_node);
                next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
                vec![Inline::Anchor {
                    node_id: anchor_node,
                    span,
                    anchor_id,
                }]
            } else if let Some(target) = line.strip_prefix("reference:") {
                let target = AnchorId::new(target).map_err(|_| "reference target is invalid")?;
                let reference_node = NodeId::new(next_node);
                next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
                vec![Inline::Reference {
                    node_id: reference_node,
                    span,
                    target,
                    format: ReferenceFormat::Page,
                }]
            } else if let Some(text) = line.strip_prefix("text:") {
                if text.is_empty() {
                    return Err("text record must not be empty");
                }
                let text_start = start
                    .checked_add("text:".len())
                    .ok_or("source span overflow")?;
                let text_end = text_start
                    .checked_add(text.len())
                    .ok_or("source span overflow")?;
                let source_start = u32::try_from(text_start).map_err(|_| "source span overflow")?;
                let source_end = u32::try_from(text_end).map_err(|_| "source span overflow")?;
                let text_len = u32::try_from(text.len()).map_err(|_| "text buffer overflow")?;
                let text_id = TextBufferId::new(
                    u32::try_from(text_buffers.len()).map_err(|_| "text buffer ID overflow")?,
                );
                let text_source_span = SourceSpan::new(
                    SourceId::new(0),
                    Utf8ByteOffset::new(source_start),
                    Utf8ByteOffset::new(source_end),
                )
                .ok_or("text source span is invalid")?;
                let text_range =
                    Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(text_len))
                        .ok_or("text range is invalid")?;
                text_buffers.push(
                    TextBuffer::new(
                        text_id,
                        text.to_owned(),
                        vec![TextMapSegment {
                            text_range,
                            kind: TextMapKind::Identity,
                            source_span: Some(text_source_span),
                        }],
                        text_len,
                    )
                    .map_err(|_| "text buffer was rejected")?,
                );
                let text_node = NodeId::new(next_node);
                next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
                vec![Inline::Text {
                    node_id: text_node,
                    span: text_source_span,
                    text_span: TextSpan::new(
                        text_id,
                        Utf8ByteOffset::new(0),
                        Utf8ByteOffset::new(text_len),
                    )
                    .ok_or("text span is invalid")?,
                }]
            } else if line == "paragraph" {
                vec![]
            } else if line == "soft_break" || line == "hard_break" {
                let break_node = NodeId::new(next_node);
                next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
                if line == "soft_break" {
                    vec![Inline::SoftBreak {
                        node_id: break_node,
                        span,
                    }]
                } else {
                    vec![Inline::HardBreak {
                        node_id: break_node,
                        span,
                    }]
                }
            } else {
                return Err("unsupported reference source record");
            };
            blocks.push(Block::Paragraph {
                node_id: paragraph_id,
                span,
                classes: vec![],
                children,
            });
        }
        start = start
            .checked_add(raw_line.len())
            .ok_or("source span overflow")?;
    }
    // The reference grammar has no page/style declarations of its own, so it
    // supplies one deterministic, physically meaningful default: A4 with a
    // 20 mm body margin, 10.5 pt text, and a 17 pt line height. Keep all unit
    // conversion on the canonical rational-PDF-point path.
    let page_width = PositiveLength::new(
        Length::from_rational_pdf_points(210 * 720, 254)
            .map_err(|_| "invalid default page width")?,
    )
    .ok_or("invalid default page width")?;
    let page_height = PositiveLength::new(
        Length::from_rational_pdf_points(297 * 720, 254)
            .map_err(|_| "invalid default page height")?,
    )
    .ok_or("invalid default page height")?;
    let body_margin = Length::from_rational_pdf_points(20 * 720, 254)
        .map_err(|_| "invalid default page margin")?;
    let body_width = PositiveLength::new(
        page_width
            .get()
            .checked_sub(body_margin)
            .and_then(|value| value.checked_sub(body_margin))
            .ok_or("invalid default body width")?,
    )
    .ok_or("invalid default body width")?;
    let body_height = PositiveLength::new(
        page_height
            .get()
            .checked_sub(body_margin)
            .and_then(|value| value.checked_sub(body_margin))
            .ok_or("invalid default body height")?,
    )
    .ok_or("invalid default body height")?;
    let default_font_size =
        Length::from_rational_pdf_points(21, 2).map_err(|_| "invalid default font size")?;
    let default_line_height =
        Length::from_rational_pdf_points(17, 1).map_err(|_| "invalid default line height")?;
    Ok(ParsedPackage {
        sources,
        text_store: TextStore::new(text_buffers).map_err(|_| "text store was rejected")?,
        document: Document {
            node_id: NodeId::new(0),
            blocks,
            footnotes: vec![],
        },
        style_sheet: StyleSheet {
            rules: if font_faces.is_empty() {
                vec![]
            } else {
                vec![StyleRule {
                    style_id: StyleId::new("reference_text")
                        .map_err(|_| "default style ID is invalid")?,
                    extends: None,
                    selector: "paragraph".to_owned(),
                    source_order: 0,
                    declarations: vec![
                        Declaration {
                            name: "font_family".to_owned(),
                            value: StyleValue::FontFamilyList(
                                font_faces.iter().map(|font| font.family.clone()).collect(),
                            ),
                            important: false,
                        },
                        Declaration {
                            name: "font_size".to_owned(),
                            value: StyleValue::Length(default_font_size),
                            important: false,
                        },
                        Declaration {
                            name: "line_height".to_owned(),
                            value: StyleValue::Length(default_line_height),
                            important: false,
                        },
                    ],
                }]
            },
        },
        page_masters: PageMasterSet {
            default_master_id: MasterId::new("default").map_err(|_| "invalid master ID")?,
            masters: vec![PageMaster {
                master_id: MasterId::new("default").map_err(|_| "invalid master ID")?,
                width: page_width,
                height: page_height,
                body: Rect::new(body_margin, body_margin, body_width, body_height),
                header: None,
                footer: None,
                footnote: None,
            }],
            selection_rules: vec![],
        },
        resources: ResourceCatalog {
            font_faces,
            images: vec![],
        },
    })
}

fn parse_reference_inline_sequence(
    sequence: &str,
    line_start: usize,
    next_node: &mut u32,
    text_buffers: &mut Vec<TextBuffer>,
) -> Result<Vec<Inline>, &'static str> {
    if sequence.is_empty() || sequence.ends_with('|') {
        return Err("inline sequence is empty or has an empty final component");
    }
    let prefix_len = "inlines:".len();
    let mut local_start = 0usize;
    let mut children = Vec::new();
    for raw_component in sequence.split_inclusive('|') {
        let component = raw_component.strip_suffix('|').unwrap_or(raw_component);
        if component.is_empty() {
            return Err("inline sequence has an empty component");
        }
        let component_source_start = line_start
            .checked_add(prefix_len)
            .and_then(|value| value.checked_add(local_start))
            .ok_or("source span overflow")?;
        let component_source_end = component_source_start
            .checked_add(component.len())
            .ok_or("source span overflow")?;
        let component_span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(
                u32::try_from(component_source_start).map_err(|_| "source span overflow")?,
            ),
            Utf8ByteOffset::new(
                u32::try_from(component_source_end).map_err(|_| "source span overflow")?,
            ),
        )
        .ok_or("inline component span is invalid")?;
        let node_id = NodeId::new(*next_node);
        *next_node = next_node.checked_add(1).ok_or("node ID overflow")?;
        if let Some(text) = component.strip_prefix("text=") {
            if text.is_empty() {
                return Err("inline text component must not be empty");
            }
            let text_source_start = component_source_start
                .checked_add("text=".len())
                .ok_or("source span overflow")?;
            let text_source_end = text_source_start
                .checked_add(text.len())
                .ok_or("source span overflow")?;
            let text_len = u32::try_from(text.len()).map_err(|_| "text buffer overflow")?;
            let text_id = TextBufferId::new(
                u32::try_from(text_buffers.len()).map_err(|_| "text buffer ID overflow")?,
            );
            let source_span = SourceSpan::new(
                SourceId::new(0),
                Utf8ByteOffset::new(
                    u32::try_from(text_source_start).map_err(|_| "source span overflow")?,
                ),
                Utf8ByteOffset::new(
                    u32::try_from(text_source_end).map_err(|_| "source span overflow")?,
                ),
            )
            .ok_or("inline text source span is invalid")?;
            let text_range =
                Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(text_len))
                    .ok_or("text range is invalid")?;
            text_buffers.push(
                TextBuffer::new(
                    text_id,
                    text.to_owned(),
                    vec![TextMapSegment {
                        text_range,
                        kind: TextMapKind::Identity,
                        source_span: Some(source_span),
                    }],
                    text_len,
                )
                .map_err(|_| "text buffer was rejected")?,
            );
            children.push(Inline::Text {
                node_id,
                span: source_span,
                text_span: TextSpan::new(
                    text_id,
                    Utf8ByteOffset::new(0),
                    Utf8ByteOffset::new(text_len),
                )
                .ok_or("text span is invalid")?,
            });
        } else if let Some(target) = component.strip_prefix("reference=") {
            children.push(Inline::Reference {
                node_id,
                span: component_span,
                target: AnchorId::new(target).map_err(|_| "reference target is invalid")?,
                format: ReferenceFormat::Page,
            });
        } else if let Some(anchor) = component.strip_prefix("anchor=") {
            children.push(Inline::Anchor {
                node_id,
                span: component_span,
                anchor_id: AnchorId::new(anchor).map_err(|_| "anchor ID is invalid")?,
            });
        } else if component == "soft_break" || component == "hard_break" {
            children.push(if component == "soft_break" {
                Inline::SoftBreak {
                    node_id,
                    span: component_span,
                }
            } else {
                Inline::HardBreak {
                    node_id,
                    span: component_span,
                }
            });
        } else {
            return Err("unsupported inline sequence component");
        }
        local_start = local_start
            .checked_add(raw_component.len())
            .ok_or("source span overflow")?;
    }
    Ok(children)
}

fn reference_parse_failure(source: &SourceFile, message: &'static str) -> ParseOutcome {
    let source_span = SourceSpan::new(
        source.source_id,
        Utf8ByteOffset::new(0),
        Utf8ByteOffset::new(0),
    )
    .expect("an empty source span is valid");
    let diagnostic = Diagnostic::located(
        DiagnosticCode::new("P1000").expect("static diagnostic code is valid"),
        Severity::Error,
        message,
        DiagnosticLocation::source(
            SourceDiagnosticLocation::new(Some(source_span), None, None)
                .expect("a source span forms a diagnostic location"),
        ),
    )
    .expect("static reference diagnostic content is canonical");
    let mut phase = PhaseDiagnostics::new();
    let flow = phase.emit(diagnostic);
    debug_assert_eq!(flow, DiagnosticFlow::Continue);
    ParseOutcome::Failed {
        failure: phase
            .finish_boundary()
            .expect_err("the safe phase boundary contains an error"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::num::{NonZeroU16, NonZeroU64};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use typaxis_core::{
        FootnoteId, GeneratedBufferKey, GenerationKind, ImageResourceId, Length, MasterId,
        PageName, PositiveLength, Rect, ResourceLimits, SafeUri, SourceSpan, StyleId, TextBufferId,
        Utf8ByteOffset, Utf8ByteRange, ValidatedResourceLimits,
    };
    use typaxis_document::{
        ColumnSizing, FontFaceDeclaration, FootnoteDefinition, HeadingLevel, ImageDeclaration,
        LinkTarget, ListItem, TableCell, TableColumn, TableRow,
    };
    use typaxis_style::{
        Declaration, PageMaster, PageMasterRule, PageParity, StyleRule, StyleValue,
    };
    use typaxis_text::{
        GeneratedBufferDraft, SourceRecord, TextBuffer, TextMapKind, TextMapSegment,
    };

    static NEXT_MACHINE_TEST_ROOT: AtomicU64 = AtomicU64::new(0);

    struct MachineTestRoot(PathBuf);

    impl MachineTestRoot {
        fn new(label: &str) -> Self {
            let ordinal = NEXT_MACHINE_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "typaxis-syntax-{label}-{}-{ordinal}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for MachineTestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn machine_wire(
        source: &str,
        text: Option<(String, wire::WireTextMapKind, Option<wire::WireSourceSpan>)>,
    ) -> WireDocumentPackage {
        let text_buffers = text
            .map(|(utf8, kind, source_span)| {
                let end = u32::try_from(utf8.len()).unwrap();
                vec![wire::WireTextBuffer {
                    text_id: 0,
                    utf8,
                    mappings: if end == 0 {
                        vec![]
                    } else {
                        vec![wire::WireTextMapSegment {
                            text_range: wire::WireByteRange {
                                start_byte: 0,
                                end_byte: end,
                            },
                            kind,
                            source_span,
                        }]
                    },
                }]
            })
            .unwrap_or_default();
        WireDocumentPackage {
            contract: typaxis_core::DocumentPackageContractId::CONTRACT_1_0,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            advanced: None,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: "input.tsf".to_owned(),
                utf8_byte_length: u32::try_from(source.len()).unwrap(),
                sha256: sha256(source.as_bytes()),
            }],
            text_buffers,
            document: wire::WireDocument {
                node_id: 0,
                blocks: vec![],
                footnotes: vec![],
            },
            style_sheet: wire::WireStyleSheet { rules: vec![] },
            page_masters: wire::WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![wire::WirePageMaster {
                    master_id: "default".to_owned(),
                    width: 100,
                    height: 100,
                    body: wire::WireRect {
                        x: 0,
                        y: 0,
                        width: 100,
                        height: 100,
                    },
                    header: None,
                    footer: None,
                    footnote: None,
                }],
                selection_rules: vec![],
            },
            resources: wire::WireResourceCatalog {
                font_faces: vec![],
                images: vec![],
            },
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn admit_machine_wire(
        root: &MachineTestRoot,
        package: &WireDocumentPackage,
        source: &str,
        limits: &ValidatedResourceLimits,
    ) -> AdmittedMachinePackage {
        let bytes = wire::DocumentPackageEncoder::default()
            .to_jcs_vec(package)
            .unwrap();
        let package_path = root.path().join("document-package.json");
        fs::write(&package_path, bytes).unwrap();
        fs::write(root.path().join("input.tsf"), source).unwrap();
        let options = typaxis_machine_input::MachineInputHostOptions::new(
            typaxis_core::HostPath::new(package_path).unwrap(),
            None,
        );
        let (session, raw) =
            typaxis_machine_input::HostMachineInputSession::open(options, limits).unwrap();
        let decode_policy = wire::DocumentPackageDecodePolicy::new(limits);
        let decoded = session
            .decode_and_bind(
                &raw,
                &wire::StrictDocumentPackageDecoder::new(),
                &decode_policy,
            )
            .unwrap();
        let sources = session.admit_sources(&decoded, limits).unwrap();
        session.finish(raw, decoded, sources).unwrap()
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_parser_moves_large_source_and_text_buffers_without_cloning() {
        let root = MachineTestRoot::new("machine-move");
        let source = "include is ordinary producer text here\n".repeat(32_768);
        let text = "x".repeat(1 << 20);
        let package = machine_wire(&source, Some((text, wire::WireTextMapKind::Inserted, None)));
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let admitted = admit_machine_wire(&root, &package, &source, &limits);
        let source_pointer = admitted.sources()[0].text().as_ptr();
        let text_pointer = admitted.decoded().wire().text_buffers[0].utf8.as_ptr();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let policy = PackageValidationPolicy::new(&limits, &schemes).unwrap();

        let MachineParseOutcome::Parsed { package } =
            DocumentPackageParser::new().parse(admitted, &policy)
        else {
            panic!("admitted package must cross syntax validation");
        };
        assert_eq!(
            package.package().package().sources.records()[0]
                .utf8()
                .as_ptr(),
            source_pointer
        );
        assert_eq!(
            package.package().package().text_store.buffers()[0]
                .text()
                .as_ptr(),
            text_pointer
        );
        assert!(package.package().include_graph().edges().is_empty());
        assert_eq!(
            package.provenance().progress().stage(),
            MachineInputStage::SourcesAdmitted
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_parser_rejects_identity_mismatch_with_source_primary_and_package_note() {
        let root = MachineTestRoot::new("identity-mismatch");
        let source = "actual";
        let package = machine_wire(
            source,
            Some((
                "xxxxxx".to_owned(),
                wire::WireTextMapKind::Identity,
                Some(wire::WireSourceSpan {
                    source_id: 0,
                    start_byte: 0,
                    end_byte: 6,
                }),
            )),
        );
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let admitted = admit_machine_wire(&root, &package, source, &limits);
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let policy = PackageValidationPolicy::new(&limits, &schemes).unwrap();

        let MachineParseOutcome::Failed { progress, failure } =
            DocumentPackageParser::new().parse(admitted, &policy)
        else {
            panic!("identity bytes must be rechecked against actual source bytes");
        };
        assert_eq!(progress.stage(), MachineInputStage::SourcesAdmitted);
        assert_eq!(
            failure.kind(),
            &MachineParseFailureKind::PackageValidation(
                PackageValidationError::IdentityBytesMismatch
            )
        );
        let MachineParsePrimaryLocation::Source(span) = failure.primary() else {
            panic!("identity mismatch must use its actual source span as primary");
        };
        assert_eq!(span.source_id(), SourceId::new(0));
        assert_eq!(span.start_byte(), Utf8ByteOffset::new(0));
        assert_eq!(span.end_byte(), Utf8ByteOffset::new(6));
        assert_eq!(
            failure.package_note().map(JsonPointer::as_str),
            Some("/text_buffers/0/mappings/0")
        );
        let diagnostic = failure.to_diagnostic(
            &PortablePath::new("document-package.json").expect("static package URI is portable"),
        );
        assert_eq!(*diagnostic.code(), typaxis_diagnostics::P1112);
        assert!(matches!(
            diagnostic.location(),
            Some(DiagnosticLocation::Source(_))
        ));
        assert!(matches!(
            diagnostic.notes().first().and_then(|note| note.location()),
            Some(DiagnosticLocation::PackageJson { json_pointer, .. })
                if json_pointer.as_str() == "/text_buffers/0/mappings/0"
        ));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_parser_maps_noncanonical_node_id_through_location_index() {
        let root = MachineTestRoot::new("node-location");
        let source = "\n";
        let mut package = machine_wire(source, None);
        package.document.node_id = 7;
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let admitted = admit_machine_wire(&root, &package, source, &limits);
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let policy = PackageValidationPolicy::new(&limits, &schemes).unwrap();

        let MachineParseOutcome::Failed { progress, failure } =
            DocumentPackageParser::new().parse(admitted, &policy)
        else {
            panic!("non-canonical node ID must fail before recursive lowering");
        };
        assert_eq!(progress.stage(), MachineInputStage::SourcesAdmitted);
        assert_eq!(failure.kind(), &MachineParseFailureKind::NonCanonicalNodeId);
        assert_eq!(
            failure.primary(),
            &MachineParsePrimaryLocation::PackageJson(JsonPointer::from_segments([
                "document", "node_id",
            ]))
        );
        assert_eq!(failure.package_note(), None);
        assert_eq!(
            failure.subject(),
            Some(&DiagnosticSubject::Node(NodeId::new(7)))
        );
        let diagnostic = failure.to_diagnostic(
            &PortablePath::new("document-package.json").expect("static package URI is portable"),
        );
        assert_eq!(*diagnostic.code(), typaxis_diagnostics::P1102);
        assert_eq!(
            diagnostic.subject(),
            Some(&DiagnosticSubject::Node(NodeId::new(7)))
        );
    }

    fn empty_package(sources: SourceCatalog, text_store: TextStore) -> ParsedPackage {
        let size = PositiveLength::new(Length::from_raw(100).unwrap()).unwrap();
        ParsedPackage {
            sources,
            text_store,
            document: Document {
                node_id: typaxis_core::NodeId::new(0),
                blocks: vec![],
                footnotes: vec![],
            },
            style_sheet: StyleSheet { rules: vec![] },
            page_masters: PageMasterSet {
                default_master_id: MasterId::new("default").unwrap(),
                masters: vec![PageMaster {
                    master_id: MasterId::new("default").unwrap(),
                    width: size,
                    height: size,
                    body: Rect::new(Length::ZERO, Length::ZERO, size, size),
                    header: None,
                    footer: None,
                    footnote: None,
                }],
                selection_rules: vec![],
            },
            resources: ResourceCatalog {
                font_faces: vec![],
                images: vec![],
            },
        }
    }

    fn validate(package: ParsedPackage) -> Result<ValidatedParsedPackage, PackageValidationError> {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = vec![
            "http".to_owned(),
            "https".to_owned(),
            "mailto".to_owned(),
            "tel".to_owned(),
        ];
        ValidatedParsedPackage::new_entry_only(
            package,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        )
    }

    fn validate_with_limits(
        package: ParsedPackage,
        limits: ResourceLimits,
    ) -> Result<ValidatedParsedPackage, PackageValidationError> {
        let limits = ValidatedResourceLimits::new(limits).unwrap();
        let schemes = vec![
            "http".to_owned(),
            "https".to_owned(),
            "mailto".to_owned(),
            "tel".to_owned(),
        ];
        ValidatedParsedPackage::new_entry_only(
            package,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        )
    }

    fn empty_package_with_source() -> ParsedPackage {
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            String::new(),
        )
        .unwrap();
        empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![]).unwrap(),
        )
    }

    fn test_source_span() -> SourceSpan {
        SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1),
        )
        .unwrap()
    }

    #[test]
    fn full_domain_surface_converts_and_encodes_through_the_shared_wire_tree() {
        let source_span = test_source_span();
        let text_range =
            Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(1)).unwrap();
        let sources = SourceCatalog::new(vec![SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            "abc".to_owned(),
        )
        .unwrap()])
        .unwrap();
        let text_store = TextStore::new(vec![
            TextBuffer::new(
                TextBufferId::new(0),
                "a".to_owned(),
                vec![TextMapSegment {
                    text_range,
                    kind: TextMapKind::Identity,
                    source_span: Some(source_span),
                }],
                1,
            )
            .unwrap(),
            TextBuffer::new(
                TextBufferId::new(1),
                "x".to_owned(),
                vec![TextMapSegment {
                    text_range,
                    kind: TextMapKind::Replacement,
                    source_span: Some(source_span),
                }],
                1,
            )
            .unwrap(),
            TextBuffer::new(
                TextBufferId::new(2),
                "y".to_owned(),
                vec![TextMapSegment {
                    text_range,
                    kind: TextMapKind::Inserted,
                    source_span: None,
                }],
                1,
            )
            .unwrap(),
        ])
        .unwrap();
        let mut package = empty_package(sources, text_store);
        let empty_children = Vec::new();
        let inline_variants = vec![
            Inline::Text {
                node_id: NodeId::new(2),
                span: source_span,
                text_span: TextSpan::new(
                    TextBufferId::new(0),
                    Utf8ByteOffset::new(0),
                    Utf8ByteOffset::new(1),
                )
                .unwrap(),
            },
            Inline::Emphasis {
                node_id: NodeId::new(3),
                span: source_span,
                children: empty_children.clone(),
            },
            Inline::Strong {
                node_id: NodeId::new(4),
                span: source_span,
                children: empty_children.clone(),
            },
            Inline::Link {
                node_id: NodeId::new(5),
                span: source_span,
                target: LinkTarget::Internal(AnchorId::new("target").unwrap()),
                children: empty_children.clone(),
            },
            Inline::Link {
                node_id: NodeId::new(6),
                span: source_span,
                target: LinkTarget::Uri(SafeUri::new("https://example.test/").unwrap()),
                children: empty_children.clone(),
            },
            Inline::Anchor {
                node_id: NodeId::new(7),
                span: source_span,
                anchor_id: AnchorId::new("target").unwrap(),
            },
            Inline::Reference {
                node_id: NodeId::new(8),
                span: source_span,
                target: AnchorId::new("target").unwrap(),
                format: ReferenceFormat::Text,
            },
            Inline::Reference {
                node_id: NodeId::new(9),
                span: source_span,
                target: AnchorId::new("target").unwrap(),
                format: ReferenceFormat::Page,
            },
            Inline::Reference {
                node_id: NodeId::new(10),
                span: source_span,
                target: AnchorId::new("target").unwrap(),
                format: ReferenceFormat::Number,
            },
            Inline::FootnoteReference {
                node_id: NodeId::new(11),
                span: source_span,
                footnote_id: FootnoteId::new("note").unwrap(),
            },
            Inline::SoftBreak {
                node_id: NodeId::new(12),
                span: source_span,
            },
            Inline::HardBreak {
                node_id: NodeId::new(13),
                span: source_span,
            },
        ];
        let size = PositiveLength::new(Length::from_raw(100).unwrap()).unwrap();
        let table_head_row = TableRow {
            node_id: NodeId::new(19),
            span: source_span,
            cells: vec![TableCell {
                node_id: NodeId::new(20),
                span: source_span,
                colspan: NonZeroU16::new(2).unwrap(),
                rowspan: NonZeroU16::new(1).unwrap(),
                blocks: vec![],
            }],
        };
        let table_body_row = TableRow {
            node_id: NodeId::new(21),
            span: source_span,
            cells: vec![TableCell {
                node_id: NodeId::new(22),
                span: source_span,
                colspan: NonZeroU16::new(2).unwrap(),
                rowspan: NonZeroU16::new(1).unwrap(),
                blocks: vec![],
            }],
        };
        package.document = Document {
            node_id: NodeId::new(0),
            blocks: vec![
                Block::Paragraph {
                    node_id: NodeId::new(1),
                    span: source_span,
                    classes: vec!["body".to_owned()],
                    children: inline_variants,
                },
                Block::Heading {
                    node_id: NodeId::new(14),
                    span: source_span,
                    classes: vec!["title".to_owned()],
                    level: HeadingLevel::new(2).unwrap(),
                    anchor_id: Some(AnchorId::new("heading").unwrap()),
                    children: vec![],
                },
                Block::List {
                    node_id: NodeId::new(15),
                    span: source_span,
                    classes: vec![],
                    ordered: true,
                    start: Some(1),
                    items: vec![ListItem {
                        node_id: NodeId::new(16),
                        span: source_span,
                        blocks: vec![Block::PageBreak {
                            node_id: NodeId::new(17),
                            span: source_span,
                            classes: vec![],
                        }],
                    }],
                },
                Block::Table {
                    node_id: NodeId::new(18),
                    span: source_span,
                    classes: vec!["grid".to_owned()],
                    columns: vec![
                        TableColumn {
                            sizing: ColumnSizing::Fixed(size),
                        },
                        TableColumn {
                            sizing: ColumnSizing::Fraction(NonZeroU16::new(2).unwrap()),
                        },
                    ],
                    head: vec![table_head_row],
                    body: vec![table_body_row],
                },
                Block::Figure {
                    node_id: NodeId::new(23),
                    span: source_span,
                    classes: vec!["figure".to_owned()],
                    image_id: ImageResourceId::new(0),
                    alt: "diagram".to_owned(),
                    caption: vec![Block::Paragraph {
                        node_id: NodeId::new(24),
                        span: source_span,
                        classes: vec![],
                        children: vec![],
                    }],
                },
                Block::PageBreak {
                    node_id: NodeId::new(25),
                    span: source_span,
                    classes: vec![],
                },
            ],
            footnotes: vec![FootnoteDefinition {
                footnote_id: FootnoteId::new("note").unwrap(),
                node_id: NodeId::new(26),
                span: source_span,
                blocks: vec![Block::Paragraph {
                    node_id: NodeId::new(27),
                    span: source_span,
                    classes: vec![],
                    children: vec![],
                }],
            }],
        };
        package.style_sheet = StyleSheet {
            rules: vec![StyleRule {
                style_id: StyleId::new("style").unwrap(),
                extends: Some(StyleId::new("base").unwrap()),
                selector: "paragraph.body".to_owned(),
                source_order: 0,
                declarations: vec![
                    Declaration {
                        name: "font_family".to_owned(),
                        value: StyleValue::FontFamilyList(vec!["Body".to_owned()]),
                        important: false,
                    },
                    Declaration {
                        name: "font_size".to_owned(),
                        value: StyleValue::Length(Length::from_raw(10).unwrap()),
                        important: true,
                    },
                    Declaration {
                        name: "line_height".to_owned(),
                        value: StyleValue::Integer(-2),
                        important: false,
                    },
                    Declaration {
                        name: "page".to_owned(),
                        value: StyleValue::Keyword("auto".to_owned()),
                        important: false,
                    },
                    Declaration {
                        name: "page".to_owned(),
                        value: StyleValue::Text("chapter".to_owned()),
                        important: false,
                    },
                    Declaration {
                        name: "page".to_owned(),
                        value: StyleValue::Boolean(true),
                        important: false,
                    },
                    Declaration {
                        name: "page".to_owned(),
                        value: StyleValue::Ratio {
                            numerator: -3,
                            denominator: NonZeroU64::new(4).unwrap(),
                        },
                        important: false,
                    },
                ],
            }],
        };
        let frame = Rect::new(Length::ZERO, Length::ZERO, size, size);
        package.page_masters = PageMasterSet {
            default_master_id: MasterId::new("default").unwrap(),
            masters: vec![PageMaster {
                master_id: MasterId::new("default").unwrap(),
                width: size,
                height: size,
                body: frame,
                header: Some(frame),
                footer: Some(frame),
                footnote: Some(frame),
            }],
            selection_rules: vec![
                PageMasterRule {
                    master_id: MasterId::new("default").unwrap(),
                    parity: PageParity::Any,
                    first: Some(true),
                    named_page: Some(PageName::new("chapter").unwrap()),
                    source_order: 0,
                },
                PageMasterRule {
                    master_id: MasterId::new("default").unwrap(),
                    parity: PageParity::Odd,
                    first: Some(false),
                    named_page: None,
                    source_order: 1,
                },
                PageMasterRule {
                    master_id: MasterId::new("default").unwrap(),
                    parity: PageParity::Even,
                    first: None,
                    named_page: None,
                    source_order: 2,
                },
            ],
        };
        package.resources = ResourceCatalog {
            font_faces: vec![FontFaceDeclaration {
                font_face_id: FontFaceId::new(0),
                family: "Body".to_owned(),
                uri: PortablePath::new("body.ttf").unwrap(),
                face_index: 2,
                expected_sha256: Some([0x11; 32]),
            }],
            images: vec![ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("diagram.png").unwrap(),
                expected_sha256: Some([0x22; 32]),
            }],
        };

        let wire_package = parsed_package_to_wire(&package).unwrap();
        assert_eq!(wire_package.text_buffers.len(), 3);
        assert_eq!(wire_package.document.blocks.len(), 6);
        assert_eq!(wire_package.document.footnotes.len(), 1);
        assert_eq!(wire_package.style_sheet.rules[0].declarations.len(), 7);
        assert_eq!(wire_package.page_masters.selection_rules.len(), 3);
        assert_eq!(wire_package.resources.font_faces.len(), 1);
        assert_eq!(wire_package.resources.images.len(), 1);

        let json = wire::DocumentPackageEncoder::default()
            .to_jcs_string(&wire_package)
            .unwrap();
        for expected in [
            "\"kind\":\"identity\"",
            "\"kind\":\"replacement\"",
            "\"kind\":\"inserted\"",
            "\"kind\":\"emphasis\"",
            "\"kind\":\"strong\"",
            "\"kind\":\"link\"",
            "\"kind\":\"footnote_reference\"",
            "\"kind\":\"soft_break\"",
            "\"kind\":\"hard_break\"",
            "\"kind\":\"heading\"",
            "\"kind\":\"list\"",
            "\"kind\":\"table\"",
            "\"kind\":\"figure\"",
            "\"kind\":\"page_break\"",
            "\"kind\":\"font_family_list\"",
            "\"kind\":\"ratio\"",
        ] {
            assert!(
                json.contains(expected),
                "missing canonical wire value {expected}"
            );
        }

        #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
        {
            // Integer/boolean/ratio are representable wire values but are not
            // valid for any current style property. Keep the encoder coverage
            // above, then select the semantically valid subset for issuance.
            let mut trusted_wire = wire_package;
            let rule = &mut trusted_wire.style_sheet.rules[0];
            rule.extends = None;
            rule.declarations[2].value = wire::WireStyleValue::Length { value: 12 };
            rule.declarations.truncate(5);
            let root = MachineTestRoot::new("full-wire-lowering");
            let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
            let admitted = admit_machine_wire(&root, &trusted_wire, "abc", &limits);
            let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
            let policy = PackageValidationPolicy::new(&limits, &schemes).unwrap();
            let MachineParseOutcome::Parsed { package } =
                DocumentPackageParser::new().parse(admitted, &policy)
            else {
                panic!("the complete semantically valid wire surface must be trusted");
            };
            assert_eq!(package.package().document_nodes().node_count(), 28);
            assert_eq!(package.package().package().document.blocks.len(), 6);
            assert_eq!(package.package().package().resources.font_faces.len(), 1);
            assert_eq!(package.package().package().resources.images.len(), 1);
        }
    }

    #[test]
    fn unknown_style_declaration_requires_an_explicit_contract_migration() {
        let mut package = empty_package_with_source();
        package.style_sheet.rules.push(StyleRule {
            style_id: StyleId::new("future").unwrap(),
            extends: None,
            selector: "paragraph".to_owned(),
            source_order: 0,
            declarations: vec![Declaration {
                name: "future_property".to_owned(),
                value: StyleValue::Boolean(true),
                important: false,
            }],
        });
        assert_eq!(
            parsed_package_to_wire(&package),
            Err(DocumentPackageConversionError::UnknownStyleDeclarationName(
                "future_property".to_owned()
            ))
        );
    }

    #[test]
    fn successful_outcome_requires_validated_package() {
        let package = empty_package_with_source();
        let package = validate(package).unwrap();
        let outcome = ParseOutcome::Parsed {
            package: Box::new(package),
            diagnostics: vec![],
        };
        assert!(matches!(outcome, ParseOutcome::Parsed { .. }));
    }

    #[test]
    fn reference_parser_derives_trusted_facts_from_source_records() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("reference.tsf").unwrap(),
            text: "anchor:chapter\ntext:actual".to_owned(),
        };
        let outcome = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        );
        let ParseOutcome::Parsed { package, .. } = outcome else {
            panic!("reference source must parse");
        };
        assert_eq!(package.package().document.blocks.len(), 2);
        assert_eq!(package.package().text_store.buffers()[0].text(), "actual");
        assert_eq!(
            package
                .document_nodes()
                .anchor_owner(&AnchorId::new("chapter").unwrap()),
            Some(NodeId::new(2))
        );
        let requested = TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(1),
            Utf8ByteOffset::new(4),
        )
        .unwrap();
        let receipt = package.bind_parsed_shape_text(requested).unwrap();
        assert_eq!(receipt.source(), PackageShapeTextSource::Parsed(requested));
        assert_eq!(receipt.site_owner(), NodeId::new(4));
        assert_eq!(receipt.style_owner(), NodeId::new(3));
        assert_eq!(receipt.utf8(), "ctu");
        assert_eq!(receipt.reference_fingerprint(), None);
        assert!(!receipt.covers_complete_site());
        assert!(receipt.is_standalone_logical_text());
        assert_eq!(
            receipt.document_fingerprint(),
            package.epoch_identity().document()
        );

        let complete = TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(6),
        )
        .unwrap();
        assert!(package
            .bind_parsed_shape_text(complete)
            .unwrap()
            .covers_complete_site());
    }

    #[test]
    fn reference_parser_uses_physical_a4_text_defaults() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("reference.tsf").unwrap(),
            text: "font:Reference:Reference.ttf\ntext:actual".to_owned(),
        };
        let ParseOutcome::Parsed { package, .. } = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        ) else {
            panic!("reference source must parse");
        };
        let master = &package.package().page_masters.masters[0];
        assert_eq!(master.width.get().raw(), 39_011_981);
        assert_eq!(master.height.get().raw(), 55_174_088);
        assert_eq!(master.body.x().raw(), 3_715_427);
        assert_eq!(master.body.y().raw(), 3_715_427);
        assert_eq!(master.body.width().get().raw(), 31_581_127);
        assert_eq!(master.body.height().get().raw(), 47_743_234);

        let computed = package.cascade_style(NodeId::new(2)).unwrap();
        assert_eq!(
            computed.computed().properties().get("font_size"),
            Some(&StyleValue::Length(Length::from_raw(688_128).unwrap()))
        );
        assert_eq!(
            computed.computed().properties().get("line_height"),
            Some(&StyleValue::Length(Length::from_raw(1_114_112).unwrap()))
        );
    }

    #[test]
    fn reference_parser_derives_canonical_adjacent_inline_sites() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("reference.tsf").unwrap(),
            text: "anchor:chapter\ninlines:text=See |reference=chapter|text= now".to_owned(),
        };
        let outcome = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        );
        let ParseOutcome::Parsed { package, .. } = outcome else {
            panic!("reference source must parse");
        };
        let Block::Paragraph {
            node_id: paragraph, ..
        } = package.package().document.blocks[1]
        else {
            panic!("inline record must derive a paragraph")
        };
        let sites = package.paragraph_shape_text_sites(paragraph).unwrap();
        assert_eq!(sites.len(), 3);
        assert!(matches!(sites[0], PackageParagraphTextSite::Parsed(_)));
        assert!(matches!(
            sites[1],
            PackageParagraphTextSite::Generated(key)
                if key.generation_kind() == GenerationKind::PageReference
        ));
        assert!(matches!(sites[2], PackageParagraphTextSite::Parsed(_)));
    }

    #[test]
    fn parsed_shape_receipt_marks_adjacent_inline_text_as_non_standalone() {
        let source_span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let text_range =
            Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(2)).unwrap();
        let text_store = TextStore::new(vec![TextBuffer::new(
            TextBufferId::new(0),
            "ab".to_owned(),
            vec![TextMapSegment {
                text_range,
                kind: TextMapKind::Inserted,
                source_span: None,
            }],
            2,
        )
        .unwrap()])
        .unwrap();
        let mut parsed = empty_package(
            SourceCatalog::new(vec![SourceRecord::new(
                SourceId::new(0),
                PortablePath::new("input.tsf").unwrap(),
                String::new(),
            )
            .unwrap()])
            .unwrap(),
            text_store,
        );
        let first = TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1),
        )
        .unwrap();
        let second = TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(1),
            Utf8ByteOffset::new(2),
        )
        .unwrap();
        parsed.document.blocks.push(Block::Paragraph {
            node_id: NodeId::new(1),
            span: source_span,
            classes: vec![],
            children: vec![
                Inline::Text {
                    node_id: NodeId::new(2),
                    span: source_span,
                    text_span: first,
                },
                Inline::Text {
                    node_id: NodeId::new(3),
                    span: source_span,
                    text_span: second,
                },
            ],
        });
        let package = validate(parsed).unwrap();
        let receipt = package.bind_parsed_shape_text(first).unwrap();
        assert!(receipt.covers_complete_site());
        assert!(!receipt.is_standalone_logical_text());
        assert_eq!(
            package.paragraph_shape_text_sites(NodeId::new(1)).unwrap(),
            [
                PackageParagraphTextSite::Parsed(first),
                PackageParagraphTextSite::Parsed(second)
            ]
        );
    }

    #[test]
    fn reference_parser_rejects_non_grammar_source() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("reference.tsf").unwrap(),
            text: "caller-authored AST marker".to_owned(),
        };
        assert!(matches!(
            ReferenceParser::new().parse(
                &source,
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            ),
            ParseOutcome::Failed { .. }
        ));
    }

    #[test]
    fn generated_text_binding_rechecks_the_actual_package_text_limits() {
        let buffer = |text_id| {
            let range =
                Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(16)).unwrap();
            TextBuffer::new(
                TextBufferId::new(text_id),
                "x".repeat(16),
                vec![TextMapSegment {
                    text_range: range,
                    kind: TextMapKind::Inserted,
                    source_span: None,
                }],
                16,
            )
            .unwrap()
        };
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            String::new(),
        )
        .unwrap();
        let package = empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![buffer(0), buffer(1)]).unwrap(),
        );
        let package = validate(package).unwrap();

        let limits = |max_text_bytes| {
            ValidatedResourceLimits::new(ResourceLimits {
                max_text_bytes,
                max_text_buffer_bytes: 16,
                max_shaping_context_bytes: 16,
                ..ResourceLimits::default()
            })
            .unwrap()
        };
        let exact = limits(32);
        // Construct the generated store against an unrelated empty parsed
        // store; package binding must still recompute the actual package's
        // parsed-plus-generated totals before accepting it.
        let generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &exact,
            &TextStore::new(vec![]).unwrap(),
        )
        .unwrap();
        assert!(package.bind_generated_text(&generated, &exact).is_ok());

        let below = limits(31);
        assert_eq!(
            package.bind_generated_text(&generated, &below),
            Err(PackageGeneratedTextError::TextTotalLimit)
        );
    }

    #[test]
    fn generated_shape_text_binds_site_style_and_selected_overlay() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut package = empty_package_with_source();
        package.document.blocks.push(Block::Paragraph {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            children: vec![Inline::SoftBreak {
                node_id: NodeId::new(2),
                span,
            }],
        });
        let package = validate(package).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let key = GeneratedBufferKey::new(NodeId::new(2), GenerationKind::Discretionary, 0);
        let generated = GeneratedTextStore::new(
            vec![
                GeneratedBufferDraft::new(package.document_nodes(), key, "xy".to_owned()).unwrap(),
            ],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let provenance = generated
            .provenance(key, Utf8ByteOffset::new(0), Utf8ByteOffset::new(2))
            .unwrap();
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let receipt = binding.bind_generated_shape_text(provenance).unwrap();
        assert_eq!(
            receipt.source(),
            PackageShapeTextSource::Generated(provenance)
        );
        assert_eq!(receipt.site_owner(), NodeId::new(2));
        assert_eq!(receipt.style_owner(), NodeId::new(1));
        assert_eq!(receipt.utf8(), "xy");
        assert!(receipt.covers_complete_site());
        assert!(!receipt.is_standalone_logical_text());
        assert_eq!(
            receipt.reference_fingerprint(),
            Some(generated.reference_fingerprint())
        );
    }

    #[test]
    fn initial_generated_overlay_materializes_explicit_break_sites() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut package = empty_package_with_source();
        package.document.blocks.push(Block::Paragraph {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            children: vec![
                Inline::SoftBreak {
                    node_id: NodeId::new(2),
                    span,
                },
                Inline::HardBreak {
                    node_id: NodeId::new(3),
                    span,
                },
            ],
        });
        let package = validate(package).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = package.materialize_initial_generated_text(&limits).unwrap();
        assert_eq!(generated.buffers().len(), 2);
        assert!(generated.buffers().iter().all(|buffer| {
            buffer.key().generation_kind() == GenerationKind::Discretionary
                && buffer.utf8().is_empty()
        }));
        package.bind_generated_text(&generated, &limits).unwrap();
    }

    #[test]
    fn footnote_marker_uses_first_text_producing_descendant_style() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let text_range =
            Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(1)).unwrap();
        let text_store = TextStore::new(vec![TextBuffer::new(
            TextBufferId::new(0),
            "x".to_owned(),
            vec![TextMapSegment {
                text_range,
                kind: TextMapKind::Inserted,
                source_span: None,
            }],
            1,
        )
        .unwrap()])
        .unwrap();
        let mut package = empty_package(
            SourceCatalog::new(vec![SourceRecord::new(
                SourceId::new(0),
                PortablePath::new("input.tsf").unwrap(),
                String::new(),
            )
            .unwrap()])
            .unwrap(),
            text_store,
        );
        package.document.footnotes.push(FootnoteDefinition {
            footnote_id: FootnoteId::new("note").unwrap(),
            node_id: NodeId::new(1),
            span,
            blocks: vec![
                Block::Paragraph {
                    node_id: NodeId::new(2),
                    span,
                    classes: vec![],
                    children: vec![
                        Inline::SoftBreak {
                            node_id: NodeId::new(3),
                            span,
                        },
                        Inline::Text {
                            node_id: NodeId::new(4),
                            span,
                            text_span: TextSpan::new(
                                TextBufferId::new(0),
                                Utf8ByteOffset::new(0),
                                Utf8ByteOffset::new(0),
                            )
                            .unwrap(),
                        },
                    ],
                },
                Block::Heading {
                    node_id: NodeId::new(5),
                    span,
                    classes: vec![],
                    level: HeadingLevel::new(2).unwrap(),
                    anchor_id: None,
                    children: vec![Inline::Text {
                        node_id: NodeId::new(6),
                        span,
                        text_span: TextSpan::new(
                            TextBufferId::new(0),
                            Utf8ByteOffset::new(0),
                            Utf8ByteOffset::new(1),
                        )
                        .unwrap(),
                    }],
                },
            ],
        });
        let package = validate(package).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let marker = GeneratedBufferKey::new(NodeId::new(1), GenerationKind::FootnoteMarker, 0);
        let discretionary =
            GeneratedBufferKey::new(NodeId::new(3), GenerationKind::Discretionary, 0);
        let generated = GeneratedTextStore::new(
            vec![
                GeneratedBufferDraft::new(package.document_nodes(), marker, "1".to_owned())
                    .unwrap(),
                GeneratedBufferDraft::new(package.document_nodes(), discretionary, " ".to_owned())
                    .unwrap(),
            ],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let provenance = generated
            .provenance(marker, Utf8ByteOffset::new(0), Utf8ByteOffset::new(1))
            .unwrap();
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let receipt = binding.bind_generated_shape_text(provenance).unwrap();
        assert_eq!(receipt.site_owner(), NodeId::new(1));
        assert_eq!(receipt.style_owner(), NodeId::new(5));
        assert!(package
            .paragraph_shape_text_sites(NodeId::new(2))
            .unwrap()
            .iter()
            .all(|site| !matches!(
                site,
                PackageParagraphTextSite::Generated(key)
                    if key.generation_kind() == GenerationKind::FootnoteMarker
            )));
        assert!(matches!(
            package
                .paragraph_shape_text_sites(NodeId::new(5))
                .unwrap()
                .first(),
            Some(PackageParagraphTextSite::Generated(key))
                if key.generation_kind() == GenerationKind::FootnoteMarker
        ));

        let wrong = GeneratedTextStore::new(
            vec![
                GeneratedBufferDraft::new(package.document_nodes(), marker, "2".to_owned())
                    .unwrap(),
                GeneratedBufferDraft::new(package.document_nodes(), discretionary, " ".to_owned())
                    .unwrap(),
            ],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        assert_eq!(
            package.bind_generated_text(&wrong, &limits),
            Err(PackageGeneratedTextError::FootnoteMarkerMismatch)
        );
    }

    #[test]
    fn page_selection_is_issued_from_the_package_style_and_owner() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut package = empty_package_with_source();
        package.document.blocks.push(Block::Paragraph {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            children: vec![],
        });
        package.style_sheet.rules.push(StyleRule {
            style_id: StyleId::new("paragraph-page").unwrap(),
            extends: None,
            selector: "paragraph".to_owned(),
            source_order: 0,
            declarations: vec![Declaration {
                name: "page".to_owned(),
                value: StyleValue::Text("chapter".to_owned()),
                important: false,
            }],
        });
        let package = validate(package).unwrap();
        let selection = package.resolve_page_selection(NodeId::new(1)).unwrap();
        assert_eq!(selection.owner(), NodeId::new(1));
        assert_eq!(selection.page_name().map(PageName::as_str), Some("chapter"));
        assert_eq!(
            package.resolve_page_selection(NodeId::new(0)),
            Err(PackageStyleError::UnknownStyleOwner)
        );
        assert_eq!(
            package.resolve_blank_page_selection(),
            Err(PackageStyleError::NonEmptyDocument)
        );
    }

    #[test]
    fn list_item_flow_owner_resolves_its_nearest_styleable_list() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut package = empty_package_with_source();
        package.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec!["chapter".to_owned()],
            ordered: false,
            start: None,
            items: vec![ListItem {
                node_id: NodeId::new(2),
                span,
                blocks: vec![],
            }],
        });
        package.style_sheet.rules.push(StyleRule {
            style_id: StyleId::new("list-page").unwrap(),
            extends: None,
            selector: "list.chapter".to_owned(),
            source_order: 0,
            declarations: vec![Declaration {
                name: "page".to_owned(),
                value: StyleValue::Text("chapter".to_owned()),
                important: false,
            }],
        });
        let package = validate(package).unwrap();
        let selection = package.resolve_page_selection(NodeId::new(2)).unwrap();
        assert_eq!(selection.owner(), NodeId::new(2));
        assert_eq!(selection.style_owner(), NodeId::new(1));
        assert_eq!(selection.page_name().map(PageName::as_str), Some("chapter"));
    }

    #[test]
    fn package_prechecks_ast_nesting_before_recursive_validation() {
        let nested_package = || {
            let span = SourceSpan::new(
                SourceId::new(0),
                Utf8ByteOffset::new(0),
                Utf8ByteOffset::new(0),
            )
            .unwrap();
            let mut inline = Inline::SoftBreak {
                node_id: NodeId::new(4),
                span,
            };
            for node_id in (2..=3).rev() {
                inline = Inline::Strong {
                    node_id: NodeId::new(node_id),
                    span,
                    children: vec![inline],
                };
            }
            let mut package = empty_package_with_source();
            package.document.blocks.push(Block::Paragraph {
                node_id: NodeId::new(1),
                span,
                classes: vec![],
                children: vec![inline],
            });
            package
        };

        assert!(validate_with_limits(
            nested_package(),
            ResourceLimits {
                max_ast_nesting_depth: 5,
                ..ResourceLimits::default()
            },
        )
        .is_ok());
        assert_eq!(
            validate_with_limits(
                nested_package(),
                ResourceLimits {
                    max_ast_nesting_depth: 4,
                    ..ResourceLimits::default()
                },
            ),
            Err(PackageValidationError::AstNestingDepthLimit)
        );
    }

    #[test]
    fn package_bounds_style_inheritance_and_preserves_graph_errors() {
        let style_chain = |length: u32| {
            let mut package = empty_package_with_source();
            package.style_sheet.rules = (0..length)
                .map(|index| StyleRule {
                    style_id: typaxis_core::StyleId::new(format!("s{index}")).unwrap(),
                    extends: index
                        .checked_sub(1)
                        .map(|parent| typaxis_core::StyleId::new(format!("s{parent}")).unwrap()),
                    selector: "paragraph".to_owned(),
                    source_order: index,
                    declarations: vec![],
                })
                .collect();
            package
        };

        assert!(validate_with_limits(
            style_chain(4),
            ResourceLimits {
                max_ast_nesting_depth: 4,
                ..ResourceLimits::default()
            },
        )
        .is_ok());
        assert_eq!(
            validate_with_limits(
                style_chain(4),
                ResourceLimits {
                    max_ast_nesting_depth: 3,
                    ..ResourceLimits::default()
                },
            ),
            Err(PackageValidationError::AstNestingDepthLimit)
        );

        let mut unknown = style_chain(1);
        unknown.style_sheet.rules[0].extends = Some(typaxis_core::StyleId::new("missing").unwrap());
        assert_eq!(
            validate_with_limits(
                unknown,
                ResourceLimits {
                    max_ast_nesting_depth: 1,
                    ..ResourceLimits::default()
                },
            ),
            Err(PackageValidationError::InvalidStyle(
                StyleValidationError::UnknownParent
            ))
        );

        let mut cycle = style_chain(2);
        cycle.style_sheet.rules[0].extends = Some(typaxis_core::StyleId::new("s1").unwrap());
        assert_eq!(
            validate_with_limits(
                cycle,
                ResourceLimits {
                    max_ast_nesting_depth: 1,
                    ..ResourceLimits::default()
                },
            ),
            Err(PackageValidationError::InvalidStyle(
                StyleValidationError::InheritanceCycle
            ))
        );
    }

    #[test]
    fn package_rejects_unknown_mapped_source() {
        let text_range =
            Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(1)).unwrap();
        let source_span = SourceSpan::new(
            SourceId::new(7),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1),
        )
        .unwrap();
        let buffer = TextBuffer::new(
            TextBufferId::new(0),
            "x".to_owned(),
            vec![TextMapSegment {
                text_range,
                kind: TextMapKind::Replacement,
                source_span: Some(source_span),
            }],
            1,
        )
        .unwrap();
        let entry = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            String::new(),
        )
        .unwrap();
        let package = empty_package(
            SourceCatalog::new(vec![entry]).unwrap(),
            TextStore::new(vec![buffer]).unwrap(),
        );
        assert_eq!(
            validate(package),
            Err(PackageValidationError::UnknownSource)
        );
    }

    #[test]
    fn package_rejects_out_of_bounds_source_span() {
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            "x".to_owned(),
        )
        .unwrap();
        let text_range =
            Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(2)).unwrap();
        let source_span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(2),
        )
        .unwrap();
        let buffer = TextBuffer::new(
            TextBufferId::new(0),
            "xx".to_owned(),
            vec![TextMapSegment {
                text_range,
                kind: TextMapKind::Identity,
                source_span: Some(source_span),
            }],
            2,
        )
        .unwrap();
        let package = empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![buffer]).unwrap(),
        );
        assert_eq!(
            validate(package),
            Err(PackageValidationError::SourceSpanOutOfBounds)
        );
    }

    #[test]
    fn package_rejects_identity_bytes_that_only_match_in_length() {
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            "a".to_owned(),
        )
        .unwrap();
        let range = Utf8ByteRange::new(Utf8ByteOffset::new(0), Utf8ByteOffset::new(1)).unwrap();
        let source_span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1),
        )
        .unwrap();
        let buffer = TextBuffer::new(
            TextBufferId::new(0),
            "b".to_owned(),
            vec![TextMapSegment {
                text_range: range,
                kind: TextMapKind::Identity,
                source_span: Some(source_span),
            }],
            1,
        )
        .unwrap();
        let package = empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![buffer]).unwrap(),
        );
        assert_eq!(
            validate(package),
            Err(PackageValidationError::IdentityBytesMismatch)
        );
    }

    #[test]
    fn package_enforces_list_start_and_table_column_semantics() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut unordered = empty_package_with_source();
        unordered.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            ordered: false,
            start: Some(1),
            items: vec![],
        });
        assert_eq!(
            validate(unordered),
            Err(PackageValidationError::InvalidListStart)
        );

        let mut ordered = empty_package_with_source();
        ordered.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            ordered: true,
            start: None,
            items: vec![],
        });
        assert_eq!(
            validate(ordered),
            Err(PackageValidationError::InvalidListStart)
        );

        let mut zero = empty_package_with_source();
        zero.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            ordered: true,
            start: Some(0),
            items: vec![],
        });
        assert_eq!(
            validate(zero),
            Err(PackageValidationError::InvalidListStart)
        );

        let mut empty_list = empty_package_with_source();
        empty_list.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            ordered: false,
            start: None,
            items: vec![],
        });
        assert_eq!(
            validate(empty_list),
            Err(PackageValidationError::EmptyListItems)
        );

        let empty_item = |node_id| ListItem {
            node_id: NodeId::new(node_id),
            span,
            blocks: vec![],
        };
        let mut overflowing = empty_package_with_source();
        overflowing.document.blocks.push(Block::List {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            ordered: true,
            start: Some(u32::MAX),
            items: vec![empty_item(2), empty_item(3)],
        });
        assert_eq!(
            validate(overflowing),
            Err(PackageValidationError::ListMarkerOverflow)
        );

        let mut table = empty_package_with_source();
        table.document.blocks.push(Block::Table {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            columns: vec![],
            head: vec![],
            body: vec![],
        });
        assert_eq!(
            validate(table),
            Err(PackageValidationError::EmptyTableColumns)
        );

        let width = PositiveLength::new(Length::from_raw(1).unwrap()).unwrap();
        let mut empty_table = empty_package_with_source();
        empty_table.document.blocks.push(Block::Table {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            columns: vec![TableColumn {
                sizing: ColumnSizing::Fixed(width),
            }],
            head: vec![],
            body: vec![],
        });
        assert_eq!(
            validate(empty_table),
            Err(PackageValidationError::EmptyTableRows)
        );

        let mut incomplete_grid = empty_package_with_source();
        incomplete_grid.document.blocks.push(Block::Table {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            columns: vec![TableColumn {
                sizing: ColumnSizing::Fixed(width),
            }],
            head: vec![],
            body: vec![TableRow {
                node_id: NodeId::new(2),
                span,
                cells: vec![],
            }],
        });
        assert_eq!(
            validate(incomplete_grid),
            Err(PackageValidationError::InvalidTableGrid)
        );

        let mut crossing = empty_package_with_source();
        crossing.document.blocks.push(Block::Table {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            columns: vec![TableColumn {
                sizing: ColumnSizing::Fixed(width),
            }],
            head: vec![TableRow {
                node_id: NodeId::new(2),
                span,
                cells: vec![TableCell {
                    node_id: NodeId::new(3),
                    span,
                    colspan: NonZeroU16::new(1).unwrap(),
                    rowspan: NonZeroU16::new(2).unwrap(),
                    blocks: vec![],
                }],
            }],
            body: vec![TableRow {
                node_id: NodeId::new(4),
                span,
                cells: vec![],
            }],
        });
        assert_eq!(
            validate(crossing),
            Err(PackageValidationError::TableHeadBodyCross)
        );
    }

    #[test]
    fn every_list_item_has_one_canonical_generated_marker() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let item = |node_id| ListItem {
            node_id: NodeId::new(node_id),
            span,
            blocks: vec![],
        };
        let mut package = empty_package_with_source();
        package.document.blocks.extend([
            Block::List {
                node_id: NodeId::new(1),
                span,
                classes: vec![],
                ordered: true,
                start: Some(9),
                items: vec![item(2), item(3)],
            },
            Block::List {
                node_id: NodeId::new(4),
                span,
                classes: vec![],
                ordered: false,
                start: None,
                items: vec![item(5)],
            },
        ]);
        let package = validate(package).unwrap();
        let key =
            |owner| GeneratedBufferKey::new(NodeId::new(owner), GenerationKind::ListMarker, 0);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            vec![
                package.materialize_list_marker(key(2)).unwrap(),
                package.materialize_list_marker(key(3)).unwrap(),
                package.materialize_list_marker(key(5)).unwrap(),
            ],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let bytes: Vec<_> = generated
            .buffers()
            .iter()
            .map(|buffer| buffer.utf8())
            .collect();
        assert_eq!(bytes, ["9.", "10.", "\u{2022}"]);
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let marker = generated
            .provenance(key(2), Utf8ByteOffset::new(0), Utf8ByteOffset::new(2))
            .unwrap();
        let receipt = binding.bind_generated_shape_text(marker).unwrap();
        assert!(receipt.covers_complete_site());
        assert!(receipt.is_standalone_logical_text());

        let wrong = GeneratedTextStore::new(
            vec![
                GeneratedBufferDraft::new(package.document_nodes(), key(2), "9. ".to_owned())
                    .unwrap(),
                package.materialize_list_marker(key(3)).unwrap(),
                package.materialize_list_marker(key(5)).unwrap(),
            ],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        assert_eq!(
            package.bind_generated_text(&wrong, &limits),
            Err(PackageGeneratedTextError::ListMarkerMismatch)
        );
    }

    #[test]
    fn package_revalidates_uri_against_effective_policy() {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut package = empty_package_with_source();
        package.document.blocks.push(Block::Paragraph {
            node_id: NodeId::new(1),
            span,
            classes: vec![],
            children: vec![Inline::Link {
                node_id: NodeId::new(2),
                span,
                target: LinkTarget::Uri(
                    typaxis_core::SafeUri::new("https://example.test").unwrap(),
                ),
                children: vec![],
            }],
        });
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = vec!["mailto".to_owned()];
        assert!(matches!(
            ValidatedParsedPackage::new_entry_only(
                package,
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            ),
            Err(PackageValidationError::InvalidUri(_))
        ));
    }

    #[test]
    fn package_requires_unique_font_family_names() {
        let mut package = empty_package_with_source();
        package.resources.font_faces = vec![
            FontFaceDeclaration {
                font_face_id: FontFaceId::new(0),
                family: "Body".to_owned(),
                uri: PortablePath::new("body-a.ttf").unwrap(),
                face_index: 0,
                expected_sha256: None,
            },
            FontFaceDeclaration {
                font_face_id: FontFaceId::new(1),
                family: "Body".to_owned(),
                uri: PortablePath::new("body-b.ttf").unwrap(),
                face_index: 0,
                expected_sha256: None,
            },
        ];
        assert_eq!(
            validate(package),
            Err(PackageValidationError::DuplicateFontFamily)
        );
    }

    #[test]
    fn include_graph_checks_exact_depth_and_catalog_identity() {
        let catalog = |count: u32, suffix: &str| {
            SourceCatalog::new(
                (0..count)
                    .map(|id| {
                        SourceRecord::new(
                            SourceId::new(id),
                            PortablePath::new(format!("source-{id}-{suffix}.tsf")).unwrap(),
                            String::new(),
                        )
                        .unwrap()
                    })
                    .collect(),
            )
            .unwrap()
        };
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_include_depth: 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        let exact_catalog = catalog(2, "exact");
        let mut resolver = IncludeResolverSession::new(&exact_catalog, &limits).unwrap();
        assert_eq!(
            resolver.admit_next_include(SourceId::new(0)),
            Ok(SourceId::new(1))
        );
        let exact = resolver.finish().unwrap();
        assert_eq!(exact.max_observed_depth(), 1);

        let too_deep = catalog(3, "deep");
        let mut resolver = IncludeResolverSession::new(&too_deep, &limits).unwrap();
        assert_eq!(
            resolver.admit_next_include(SourceId::new(0)),
            Ok(SourceId::new(1))
        );
        assert_eq!(
            resolver.admit_next_include(SourceId::new(1)),
            Err(IncludeGraphError::IncludeDepthLimit)
        );
        assert!(!exact.matches(&catalog(2, "other")));
    }

    #[test]
    fn entry_only_validation_rejects_unresolved_include_syntax() {
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("input.tsf").unwrap(),
            "@ include \"child.tsf\";".to_owned(),
        )
        .unwrap();
        let package = empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![]).unwrap(),
        );
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        assert_eq!(
            ValidatedParsedPackage::new_entry_only(
                package,
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            ),
            Err(PackageValidationError::UnresolvedIncludeDirective)
        );
    }

    #[test]
    fn package_epoch_identity_matches_portable_minimal_golden() {
        let source = SourceRecord::new(
            SourceId::new(0),
            PortablePath::new("empty.tsf").unwrap(),
            "\n".to_owned(),
        )
        .unwrap();
        let mut package = empty_package(
            SourceCatalog::new(vec![source]).unwrap(),
            TextStore::new(vec![]).unwrap(),
        );
        let width = PositiveLength::new(Length::from_raw(39_011_981).unwrap()).unwrap();
        let height = PositiveLength::new(Length::from_raw(55_174_088).unwrap()).unwrap();
        let body_width = PositiveLength::new(Length::from_raw(31_581_127).unwrap()).unwrap();
        let body_height = PositiveLength::new(Length::from_raw(47_743_234).unwrap()).unwrap();
        package.page_masters = PageMasterSet {
            default_master_id: MasterId::new("a4").unwrap(),
            masters: vec![PageMaster {
                master_id: MasterId::new("a4").unwrap(),
                width,
                height,
                body: Rect::new(
                    Length::from_raw(3_715_427).unwrap(),
                    Length::from_raw(3_715_427).unwrap(),
                    body_width,
                    body_height,
                ),
                header: None,
                footer: None,
                footnote: None,
            }],
            selection_rules: vec![],
        };
        let identity = PackageEpochIdentity::from_package(&package);
        let hex = |bytes: [u8; 32]| {
            let mut value = String::new();
            push_hash_hex(&mut value, bytes);
            value.trim_matches('"').to_owned()
        };
        assert_eq!(
            hex(identity.document().bytes()),
            "0f274e09eca8f12c46be9b2398c24efcb46cb91190c6a7c6d6fc86dac044e6af"
        );
        assert_eq!(
            hex(identity.style().bytes()),
            "40d9810b810455a25f773560a743860c1d04e59b7c72273c161da2136b09b12d"
        );

        package.document.node_id = NodeId::new(1);
        let changed = PackageEpochIdentity::from_package(&package);
        assert_ne!(identity.document(), changed.document());
        assert_eq!(identity.style(), changed.style());
    }

    #[test]
    fn document_epoch_binds_every_resource_declaration_field() {
        let base = empty_package_with_source();
        let base_identity = PackageEpochIdentity::from_package(&base);
        let font = FontFaceDeclaration {
            font_face_id: FontFaceId::new(0),
            family: "Body".to_owned(),
            uri: PortablePath::new("body.ttf").unwrap(),
            face_index: 2,
            expected_sha256: Some([1; 32]),
        };
        let image = ImageDeclaration {
            image_id: ImageResourceId::new(0),
            uri: PortablePath::new("cover.png").unwrap(),
            expected_sha256: Some([2; 32]),
        };
        let with_resources = |font: FontFaceDeclaration, image: ImageDeclaration| {
            let mut package = base.clone();
            package.resources.font_faces.push(font);
            package.resources.images.push(image);
            PackageEpochIdentity::from_package(&package)
        };
        let identity = with_resources(font.clone(), image.clone());
        assert_ne!(base_identity.document(), identity.document());
        assert_eq!(base_identity.style(), identity.style());

        let mut variants = Vec::new();
        let mut value = font.clone();
        value.font_face_id = FontFaceId::new(1);
        variants.push(with_resources(value, image.clone()));
        let mut value = font.clone();
        value.family = "Heading".to_owned();
        variants.push(with_resources(value, image.clone()));
        let mut value = font.clone();
        value.uri = PortablePath::new("other.ttf").unwrap();
        variants.push(with_resources(value, image.clone()));
        let mut value = font.clone();
        value.face_index = 3;
        variants.push(with_resources(value, image.clone()));
        let mut value = font.clone();
        value.expected_sha256 = None;
        variants.push(with_resources(value, image.clone()));
        let mut value = image.clone();
        value.image_id = ImageResourceId::new(1);
        variants.push(with_resources(font.clone(), value));
        let mut value = image.clone();
        value.uri = PortablePath::new("other.png").unwrap();
        variants.push(with_resources(font.clone(), value));
        let mut value = image;
        value.expected_sha256 = None;
        variants.push(with_resources(font, value));

        assert!(variants
            .iter()
            .all(|variant| variant.document() != identity.document()));
        assert!(variants
            .iter()
            .all(|variant| variant.style() == identity.style()));
    }

    #[test]
    fn machine_properties_lower_to_package_bound_typed_receipt_under_the_current_contract() {
        let source = "p";
        let mut package = machine_wire(source, None);
        package.document.blocks.push(wire::WireBlock::Paragraph {
            node_id: 1,
            span: wire::WireSourceSpan {
                source_id: 0,
                start_byte: 0,
                end_byte: 1,
            },
            classes: vec!["lead".to_owned()],
            children: vec![],
        });
        package.style_sheet.rules = vec![
            wire::WireStyleRule {
                style_id: "paragraph-style".to_owned(),
                extends: None,
                selector: "paragraph.lead".to_owned(),
                source_order: 0,
                declarations: vec![
                    wire::WireDeclaration {
                        name: wire::WireDeclarationName::SpaceBefore,
                        value: wire::WireStyleValue::Length { value: 1 },
                        important: false,
                    },
                    wire::WireDeclaration {
                        name: wire::WireDeclarationName::SpaceAfter,
                        value: wire::WireStyleValue::Length { value: 2 },
                        important: false,
                    },
                    wire::WireDeclaration {
                        name: wire::WireDeclarationName::StartIndent,
                        value: wire::WireStyleValue::Length { value: 3 },
                        important: false,
                    },
                    wire::WireDeclaration {
                        name: wire::WireDeclarationName::EndIndent,
                        value: wire::WireStyleValue::Length { value: 4 },
                        important: false,
                    },
                    wire::WireDeclaration {
                        name: wire::WireDeclarationName::TextAlign,
                        value: wire::WireStyleValue::Keyword {
                            value: "center".to_owned(),
                        },
                        important: false,
                    },
                    wire::WireDeclaration {
                        name: wire::WireDeclarationName::KeepWithNext,
                        value: wire::WireStyleValue::Boolean { value: true },
                        important: false,
                    },
                ],
            },
            wire::WireStyleRule {
                style_id: "figure-style".to_owned(),
                extends: None,
                selector: "figure".to_owned(),
                source_order: 1,
                declarations: vec![
                    wire::WireDeclaration {
                        name: wire::WireDeclarationName::Width,
                        value: wire::WireStyleValue::Length { value: 20 },
                        important: false,
                    },
                    wire::WireDeclaration {
                        name: wire::WireDeclarationName::KeepCaption,
                        value: wire::WireStyleValue::Boolean { value: false },
                        important: false,
                    },
                ],
            },
        ];
        let bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&package)
            .unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("typaxis.contract/1.2"));

        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decode_policy = wire::DocumentPackageDecodePolicy::new(&limits);
        let current = wire::StrictDocumentPackageDecoder::new()
            .decode(&bytes, &decode_policy)
            .unwrap();
        assert_eq!(
            current.wire().contract,
            typaxis_core::DocumentPackageContractId::V1_2
        );
        let decoded = wire::StagingStyleDocumentPackageDecoder::new()
            .decode(&bytes, &decode_policy)
            .unwrap();
        assert_eq!(
            current.canonical_jcs_sha256(),
            decoded.canonical_jcs_sha256()
        );
        let package_hash = decoded.canonical_jcs_sha256();
        let schemes = vec!["http".to_owned(), "https".to_owned()];
        let policy = PackageValidationPolicy::new(&limits, &schemes).unwrap();
        let validated = StagingStylePackageParser::new()
            .parse(decoded, source.to_owned(), &policy)
            .unwrap();
        let receipt = validated.compute_block_style(NodeId::new(1), None).unwrap();
        assert_eq!(receipt.owner(), NodeId::new(1));
        assert_eq!(receipt.style_owner(), NodeId::new(1));
        assert_eq!(receipt.package_fingerprint(), package_hash);
        assert_eq!(
            receipt.registry_version(),
            BASIC_BLOCK_STYLE_REGISTRY_VERSION
        );
        assert_eq!(receipt.computed().space_before().get().raw(), 1);
        assert_eq!(receipt.computed().space_after().get().raw(), 2);
        assert_eq!(receipt.computed().start_indent().get().raw(), 3);
        assert_eq!(receipt.computed().end_indent().get().raw(), 4);
        assert_eq!(
            receipt.computed().text_align(),
            typaxis_style::MachineTextAlign::Center
        );
        assert!(receipt.computed().keep_with_next());

        let wrong_tag = String::from_utf8(bytes).unwrap().replacen(
            "\"kind\":\"length\",\"value\":1",
            "\"kind\":\"integer\",\"value\":1",
            1,
        );
        let error = wire::StagingStyleDocumentPackageDecoder::new()
            .decode(wrong_tag.as_bytes(), &decode_policy)
            .unwrap_err();
        assert_eq!(
            error
                .typed_error()
                .unwrap()
                .location()
                .json_pointer()
                .as_str(),
            "/style_sheet/rules/0/declarations/0/value"
        );
    }

    #[test]
    fn machine_properties_inheritance_requires_the_typed_flow_owner_receipt() {
        let mut wire_package = machine_wire("", None);
        let span = wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        wire_package.document.blocks.push(wire::WireBlock::List {
            node_id: 1,
            span,
            classes: vec![],
            ordered: false,
            start: None,
            items: vec![wire::WireListItem {
                node_id: 2,
                span,
                blocks: vec![wire::WireBlock::Paragraph {
                    node_id: 3,
                    span,
                    classes: vec![],
                    children: vec![],
                }],
            }],
        });
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&wire_package)
            .unwrap();
        let decoded = wire::StagingStyleDocumentPackageDecoder::new()
            .decode(&bytes, &wire::DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let package = StagingStylePackageParser::new()
            .parse(
                decoded,
                String::new(),
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            )
            .unwrap();
        assert_eq!(
            package.compute_block_style(NodeId::new(3), None),
            Err(StagingStyleReceiptMismatch::ParentReceiptMismatch)
        );
        let list = package.compute_block_style(NodeId::new(1), None).unwrap();
        let paragraph = package
            .compute_block_style(NodeId::new(3), Some(&list))
            .unwrap();
        assert_eq!(paragraph.style_owner(), NodeId::new(3));
        assert_eq!(
            paragraph.computed().text_align(),
            typaxis_style::MachineTextAlign::Start
        );
    }
}
