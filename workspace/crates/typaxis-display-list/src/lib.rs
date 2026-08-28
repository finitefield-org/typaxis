#![forbid(unsafe_code)]

mod advanced_columns;
mod advanced_content;
mod advanced_float;
mod advanced_header_footer;
mod book_navigation;
mod math;
mod safe_vector;
mod semantic_container;

#[cfg(any(test, feature = "staging-fixtures"))]
pub use advanced_columns::staging_columns_display_fixture;
pub use advanced_columns::{
    build_staging_columns_display, StagingColumnPaintCommand, StagingColumnsDisplay,
    StagingColumnsDisplayError, StagingColumnsDisplayPage, StagingColumnsDisplayReceipt,
};
pub use advanced_content::{
    StagingAdvancedAnchorUse, StagingAdvancedContentBinding, StagingAdvancedContentError,
    StagingAdvancedImageUse, StagingAdvancedLinkTarget, StagingAdvancedLinkUse,
    StagingAdvancedPageContent, ADVANCED_CONTENT_BINDING_ALGORITHM,
};
#[cfg(any(test, feature = "staging-fixtures"))]
pub use advanced_float::staging_float_display_fixture;
pub use advanced_float::{
    build_staging_float_display, StagingFloatDisplay, StagingFloatDisplayError,
    StagingFloatDisplayPage, StagingFloatDisplayReceipt, StagingFloatPaintCommand,
    StagingFloatPaintCommandKind,
};
pub use advanced_header_footer::{
    build_staging_header_footer_display, StagingHeaderFooterDisplay,
    StagingHeaderFooterDisplayError, StagingHeaderFooterDisplayPage,
    StagingHeaderFooterDisplayReceipt, StagingHeaderFooterPaintCommand, StagingPdfPageBox,
    StagingSelectedPageBoxes, ADVANCED_PAINT_CLOSURE_ALGORITHM,
};
pub use book_navigation::{
    select_staging_book_navigation, BookInternalLink, BookInternalLinkInput, BookLanguagePaint,
    BookLanguagePaintInput, BookNavigationDestinationBinding, BookNavigationSelectedEntry,
    BookNavigationSelectedError, BookNavigationSelectedPage, BookNavigationSelectedReceipt,
    BOOK_DESTINATION_REGISTRY_ALGORITHM, BOOK_NAVIGATION_SELECTED_ALGORITHM,
};
pub use math::{
    build_staging_math_display, StagingMathDisplay, StagingMathDisplayError, StagingMathDraw,
    MATH_DISPLAY_ALGORITHM,
};
#[cfg(any(test, feature = "staging-fixtures"))]
pub use math::{staging_math_display_fixture, StagingMathDisplayFixture};
pub use safe_vector::{
    build_staging_safe_vector_display, StagingDrawVector, StagingSafeVectorDisplay,
    StagingSafeVectorDisplayError, StagingSafeVectorDisplayPage, StagingSafeVectorDisplayReceipt,
    STAGING_DRAW_VECTOR_ALGORITHM,
};
#[cfg(any(test, feature = "staging-fixtures"))]
pub use safe_vector::{staging_safe_vector_display_fixture, StagingSafeVectorDisplayFixture};
#[cfg(feature = "staging-fixtures")]
pub use semantic_container::build_staging_semantic_container_display_fixture;
pub use semantic_container::{
    build_staging_semantic_container_display, StagingSemanticChildPaint,
    StagingSemanticContainerDisplay, StagingSemanticContainerDisplayError,
    StagingSemanticContainerDisplayPage, StagingSemanticContainerDisplayReceipt,
    StagingSemanticContainerPaint, StagingSemanticRasterObservation, StagingSemanticStructureRole,
};

use typaxis_core::{
    push_generated_buffer_key_jcs, push_jcs_string, sha256, AffineTransform, AnchorId, BidiLevel,
    DisplayGlyphRunId, DisplayTextBufferId, DisplayTextSpan, EffectiveConfig, FontFaceId,
    FontInstanceId, FootnoteId, GeneratedBufferKey, GeneratedTextBufferId, GeneratedTextSpan,
    GenerationKind, ImageResourceId, LayoutStateFingerprint, Length, MasterId, NodeId,
    NonNegativeLength, Point, PositiveLength, PositiveUnitless16_16, Rect, ReferenceFingerprint,
    SafeUri, TextBufferId, TextSpan, Unitless16_16, JSON_SAFE_INTEGER_MAX,
};
use typaxis_document::{Block, Inline};
use typaxis_font::OriginalGlyphId;
use typaxis_layout::{
    FlowId, FlowTree, FootnoteFlowId, LayoutEpoch, SelectedTypedBlockStyle,
    StagingFootnoteFlowRegistry, TableRowBandLayoutReceipt, ValidatedTableGridReceipt,
    FOOTNOTE_SEPARATOR_BAND_RAW,
};
use typaxis_linebreak::{
    reorder_line_l2, reset_line_bidi_levels, LineBidiClass, LineLevelsAfterL1, ParagraphItem,
    ShapedSlice, StagingMachineLinkClusterKey, ValidatedParagraphItemRegistry,
    ValidatedStagingMachineLinkClusterRange, ValidatedStagingMachineLinkClusters,
};
use typaxis_pagination::{
    PageFrameKind, PaginationResult, SelectedTableLayoutReceipt,
    StagingForcedPageBreakConsumeReceipt, StagingForcedPageBreakSelectedPage,
    StagingForcedPageBreakSelectedState, StagingMachineFigureCaptionFragment,
    StagingMachineFigurePlacement, StagingMachineFigureSelectedPage,
    StagingMachineFigureSelectedState, StagingMachineListSelectedState,
    ValidatedFootnoteSelectedLayout,
};
use typaxis_shaping::{ShapeSourceSpan, ValidatedGlyphRun};
use typaxis_style::{BasicStyleBlockKind, StyleValue};
use typaxis_syntax::{
    ValidatedParsedPackage, ValidatedStagingLinkTarget, ValidatedStagingStylePackage,
    STAGING_BASIC_LINK_POLICY_VERSION,
};
use typaxis_text::{GeneratedTextStore, TextStore};

pub const CONTRACT_VERSION: &str = typaxis_core::CONTRACT;
pub const COORDINATE_UNIT: &str = typaxis_core::COORDINATE_UNIT;
pub const DISPLAY_COMMAND_OPS: &[&str] = &[
    "save",
    "restore",
    "concat_transform",
    "clip_path",
    "fill_path",
    "stroke_path",
    "draw_glyph_run",
    "draw_image",
];

pub const STAGING_MACHINE_BLOCK_STYLE_DISPLAY_ALGORITHM: &str =
    "typaxis.machine-block-style-display/1";

/// Crate-level 1.2 paint observation derived only from layout's selected
/// typed-style receipt. The public 1.1 display-list format remains unchanged.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingMachineBlockStyleDisplay {
    owner: u32,
    package_sha256: [u8; 32],
    registry_version: &'static str,
    block_kind: BasicStyleBlockKind,
    frame_inline_size: i64,
    available_inline_size: i64,
    paint_inline_size: i64,
    start_indent: i64,
    end_indent: i64,
    logical_start_alignment_space: i64,
    logical_end_alignment_space: i64,
    paint_left_inset: i64,
    effective_space_before: i64,
    effective_space_after: i64,
    page_break_before: bool,
    keep_with_next: bool,
    keep_caption: bool,
    canonical_jcs: String,
}

impl StagingMachineBlockStyleDisplay {
    pub fn from_selected(selected: &SelectedTypedBlockStyle) -> Self {
        let mut value = Self {
            owner: selected.owner().get(),
            package_sha256: selected.package_sha256(),
            registry_version: selected.registry_version(),
            block_kind: selected.block_kind(),
            frame_inline_size: selected.frame_inline_size().get().raw(),
            available_inline_size: selected.available_inline_size().get().raw(),
            paint_inline_size: selected.content_inline_size().get().raw(),
            start_indent: selected.start_indent().get().raw(),
            end_indent: selected.end_indent().get().raw(),
            logical_start_alignment_space: selected.logical_start_alignment_space().get().raw(),
            logical_end_alignment_space: selected.logical_end_alignment_space().get().raw(),
            paint_left_inset: selected.physical_left_inset().get().raw(),
            effective_space_before: selected.effective_space_before().get().raw(),
            effective_space_after: selected.effective_space_after().get().raw(),
            page_break_before: selected.page_break_before(),
            keep_with_next: selected.keep_with_next(),
            keep_caption: selected.keep_caption(),
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_block_style_display(&value);
        value
    }

    pub const fn owner_node_id(&self) -> u32 {
        self.owner
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn registry_version(&self) -> &'static str {
        self.registry_version
    }
    pub const fn block_kind(&self) -> BasicStyleBlockKind {
        self.block_kind
    }
    pub const fn frame_inline_size(&self) -> i64 {
        self.frame_inline_size
    }
    pub const fn available_inline_size(&self) -> i64 {
        self.available_inline_size
    }
    pub const fn paint_inline_size(&self) -> i64 {
        self.paint_inline_size
    }
    pub const fn start_indent(&self) -> i64 {
        self.start_indent
    }
    pub const fn end_indent(&self) -> i64 {
        self.end_indent
    }
    pub const fn logical_start_alignment_space(&self) -> i64 {
        self.logical_start_alignment_space
    }
    pub const fn logical_end_alignment_space(&self) -> i64 {
        self.logical_end_alignment_space
    }
    pub const fn paint_left_inset(&self) -> i64 {
        self.paint_left_inset
    }
    pub const fn effective_space_before(&self) -> i64 {
        self.effective_space_before
    }
    pub const fn effective_space_after(&self) -> i64 {
        self.effective_space_after
    }
    pub const fn page_break_before(&self) -> bool {
        self.page_break_before
    }
    pub const fn keep_with_next(&self) -> bool {
        self.keep_with_next
    }
    pub const fn keep_caption(&self) -> bool {
        self.keep_caption
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    /// Fixed cross-crate fixture; unavailable in production builds.
    #[cfg(any(test, feature = "staging-fixtures"))]
    #[doc(hidden)]
    pub fn paragraph_pdf_test_fixture() -> Self {
        let mut value = Self {
            owner: 1,
            package_sha256: [0xaa; 32],
            registry_version: "typaxis.basic-block-style-registry/1",
            block_kind: BasicStyleBlockKind::Paragraph,
            frame_inline_size: 101,
            available_inline_size: 81,
            paint_inline_size: 20,
            start_indent: 10,
            end_indent: 10,
            logical_start_alignment_space: 30,
            logical_end_alignment_space: 31,
            paint_left_inset: 40,
            effective_space_before: 0,
            effective_space_after: 6,
            page_break_before: true,
            keep_with_next: true,
            keep_caption: true,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_block_style_display(&value);
        value
    }

    /// Fixed cross-crate fixture; unavailable in production builds.
    #[cfg(feature = "staging-fixtures")]
    #[doc(hidden)]
    pub fn figure_pdf_test_fixture() -> Self {
        let mut value = Self {
            owner: 2,
            package_sha256: [0xbb; 32],
            registry_version: "typaxis.basic-block-style-registry/1",
            block_kind: BasicStyleBlockKind::Figure,
            frame_inline_size: 100,
            available_inline_size: 100,
            paint_inline_size: 30,
            start_indent: 0,
            end_indent: 0,
            logical_start_alignment_space: 0,
            logical_end_alignment_space: 70,
            paint_left_inset: 0,
            effective_space_before: 0,
            effective_space_after: 0,
            page_break_before: false,
            keep_with_next: false,
            keep_caption: false,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_block_style_display(&value);
        value
    }
}

fn encode_staging_machine_block_style_display(value: &StagingMachineBlockStyleDisplay) -> String {
    let mut output = String::from("{\"algorithm\":\"");
    output.push_str(STAGING_MACHINE_BLOCK_STYLE_DISPLAY_ALGORITHM);
    output.push_str("\",\"available_inline_size\":");
    output.push_str(&value.available_inline_size.to_string());
    output.push_str(",\"block_kind\":\"");
    output.push_str(value.block_kind.as_str());
    output.push_str("\",\"effective_space_after\":");
    output.push_str(&value.effective_space_after.to_string());
    output.push_str(",\"effective_space_before\":");
    output.push_str(&value.effective_space_before.to_string());
    output.push_str(",\"end_indent\":");
    output.push_str(&value.end_indent.to_string());
    output.push_str(",\"frame_inline_size\":");
    output.push_str(&value.frame_inline_size.to_string());
    output.push_str(",\"keep_caption\":");
    output.push_str(if value.keep_caption { "true" } else { "false" });
    output.push_str(",\"keep_with_next\":");
    output.push_str(if value.keep_with_next {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"logical_end_alignment_space\":");
    output.push_str(&value.logical_end_alignment_space.to_string());
    output.push_str(",\"logical_start_alignment_space\":");
    output.push_str(&value.logical_start_alignment_space.to_string());
    output.push_str(",\"owner_node_id\":");
    output.push_str(&value.owner.to_string());
    output.push_str(",\"package_sha256\":\"");
    push_staging_hex(&mut output, value.package_sha256);
    output.push_str("\",\"page_break_before\":");
    output.push_str(if value.page_break_before {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"paint_inline_size\":");
    output.push_str(&value.paint_inline_size.to_string());
    output.push_str(",\"paint_left_inset\":");
    output.push_str(&value.paint_left_inset.to_string());
    output.push_str(",\"registry_version\":\"");
    output.push_str(value.registry_version);
    output.push_str("\",\"start_indent\":");
    output.push_str(&value.start_indent.to_string());
    output.push('}');
    output
}

fn push_staging_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
}

pub const STAGING_MACHINE_FIGURE_DISPLAY_ALGORITHM: &str = "typaxis.machine-figure-display/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMachineFigureDisplayError {
    EmptyFigureSet,
    MissingDrawImage(ImageResourceId),
    ExtraDrawImage(ImageResourceId),
    WrongDrawImage {
        expected: ImageResourceId,
        actual: ImageResourceId,
    },
    PageClosure,
    PlacementOutOfBounds(NodeId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingMachineFigureEpochFacts {
    admitted_resources_sha256: [u8; 32],
    document_sha256: [u8; 32],
    resolved_input_sha256: [u8; 32],
    style_page_master_sha256: [u8; 32],
}

impl StagingMachineFigureEpochFacts {
    fn from_layout_epoch(epoch: LayoutEpoch) -> Self {
        Self {
            admitted_resources_sha256: epoch.admitted_resources().bytes(),
            document_sha256: epoch.document().bytes(),
            resolved_input_sha256: epoch.references().bytes(),
            style_page_master_sha256: epoch.style().bytes(),
        }
    }

    pub const fn admitted_resources_sha256(self) -> [u8; 32] {
        self.admitted_resources_sha256
    }
    pub const fn document_sha256(self) -> [u8; 32] {
        self.document_sha256
    }
    pub const fn resolved_input_sha256(self) -> [u8; 32] {
        self.resolved_input_sha256
    }
    pub const fn style_page_master_sha256(self) -> [u8; 32] {
        self.style_page_master_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigureDisplayCaption {
    caption_node_id: u32,
    caption_flow_id: u32,
    page_index: u32,
    rect: Rect,
}

impl StagingMachineFigureDisplayCaption {
    pub const fn caption_node_id(&self) -> u32 {
        self.caption_node_id
    }
    pub const fn caption_flow_id(&self) -> u32 {
        self.caption_flow_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn rect(&self) -> Rect {
        self.rect
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigureDisplayFigure {
    figure_node_id: u32,
    document_ordinal: u32,
    figure_flow_id: u32,
    caption_flow_id: u32,
    image_id: ImageResourceId,
    alt: String,
    attested_media_kind: &'static str,
    admitted_sha256: [u8; 32],
    admitted_byte_length: u64,
    pixel_width: u32,
    pixel_height: u32,
    decoded_bytes: u64,
    page_index: u32,
    rect: Rect,
    effective_space_before: i64,
    keep_policy: &'static str,
    oversize_policy: &'static str,
    moved_to_fresh_page: bool,
    caption_fragments: Vec<StagingMachineFigureDisplayCaption>,
}

impl StagingMachineFigureDisplayFigure {
    pub const fn figure_node_id(&self) -> u32 {
        self.figure_node_id
    }
    pub const fn document_ordinal(&self) -> u32 {
        self.document_ordinal
    }
    pub const fn figure_flow_id(&self) -> u32 {
        self.figure_flow_id
    }
    pub const fn caption_flow_id(&self) -> u32 {
        self.caption_flow_id
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub fn alt(&self) -> &str {
        &self.alt
    }
    pub const fn attested_media_kind(&self) -> &'static str {
        self.attested_media_kind
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub const fn admitted_byte_length(&self) -> u64 {
        self.admitted_byte_length
    }
    pub const fn pixel_width(&self) -> u32 {
        self.pixel_width
    }
    pub const fn pixel_height(&self) -> u32 {
        self.pixel_height
    }
    pub const fn decoded_bytes(&self) -> u64 {
        self.decoded_bytes
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn rect(&self) -> Rect {
        self.rect
    }
    pub const fn effective_space_before(&self) -> i64 {
        self.effective_space_before
    }
    pub const fn keep_policy(&self) -> &'static str {
        self.keep_policy
    }
    pub const fn oversize_policy(&self) -> &'static str {
        self.oversize_policy
    }
    pub const fn moved_to_fresh_page(&self) -> bool {
        self.moved_to_fresh_page
    }
    pub fn caption_fragments(&self) -> &[StagingMachineFigureDisplayCaption] {
        &self.caption_fragments
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigureDisplayPage {
    page_index: u32,
    figure_count: u32,
    caption_block_count: u32,
}

impl StagingMachineFigureDisplayPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn figure_count(&self) -> u32 {
        self.figure_count
    }
    pub const fn caption_block_count(&self) -> u32 {
        self.caption_block_count
    }
}

/// Copyable projection retained after the trusted Display document is moved
/// into resource finalization/PDF construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigureDisplayFacts {
    package_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    figure_usage_sha256: [u8; 32],
    policy_version: &'static str,
    epoch: StagingMachineFigureEpochFacts,
    selected_state_sha256: [u8; 32],
    layout_state_sha256: [u8; 32],
    master_id: MasterId,
    page_width: PositiveLength,
    page_height: PositiveLength,
    body: Rect,
    pages: Vec<StagingMachineFigureDisplayPage>,
    figures: Vec<StagingMachineFigureDisplayFigure>,
    canonical_jcs: String,
}

impl StagingMachineFigureDisplayFacts {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }
    pub const fn figure_usage_sha256(&self) -> [u8; 32] {
        self.figure_usage_sha256
    }
    pub const fn policy_version(&self) -> &'static str {
        self.policy_version
    }
    pub const fn epoch(&self) -> StagingMachineFigureEpochFacts {
        self.epoch
    }
    pub const fn selected_state_sha256(&self) -> [u8; 32] {
        self.selected_state_sha256
    }
    pub const fn layout_state_sha256(&self) -> [u8; 32] {
        self.layout_state_sha256
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn page_width(&self) -> PositiveLength {
        self.page_width
    }
    pub const fn page_height(&self) -> PositiveLength {
        self.page_height
    }
    pub const fn body(&self) -> Rect {
        self.body
    }
    pub fn pages(&self) -> &[StagingMachineFigureDisplayPage] {
        &self.pages
    }
    pub fn figures(&self) -> &[StagingMachineFigureDisplayFigure] {
        &self.figures
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

pub struct StagingMachineFigureDisplay {
    trusted: ValidatedDisplayDocument,
    facts: StagingMachineFigureDisplayFacts,
}

impl StagingMachineFigureDisplay {
    pub fn from_selected(
        selected: &StagingMachineFigureSelectedState,
    ) -> Result<Self, StagingMachineFigureDisplayError> {
        let image_ids = selected
            .figures()
            .iter()
            .map(StagingMachineFigurePlacement::image_id)
            .collect();
        Self::from_selected_with_draw_image_ids(selected, image_ids)
    }

    /// Exact paint-worker closure seam used by tamper tests. Every Figure must
    /// contribute one DrawImage in canonical order and no other image command.
    #[doc(hidden)]
    pub fn from_selected_with_draw_image_ids(
        selected: &StagingMachineFigureSelectedState,
        draw_image_ids: Vec<ImageResourceId>,
    ) -> Result<Self, StagingMachineFigureDisplayError> {
        if selected.figures().is_empty() {
            return Err(StagingMachineFigureDisplayError::EmptyFigureSet);
        }
        for (index, figure) in selected.figures().iter().enumerate() {
            let Some(actual) = draw_image_ids.get(index).copied() else {
                return Err(StagingMachineFigureDisplayError::MissingDrawImage(
                    figure.image_id(),
                ));
            };
            if actual != figure.image_id() {
                return Err(StagingMachineFigureDisplayError::WrongDrawImage {
                    expected: figure.image_id(),
                    actual,
                });
            }
        }
        if let Some(extra) = draw_image_ids.get(selected.figures().len()).copied() {
            return Err(StagingMachineFigureDisplayError::ExtraDrawImage(extra));
        }
        if selected
            .pages()
            .iter()
            .enumerate()
            .any(|(index, page)| page.page_index() != u32::try_from(index).unwrap_or(u32::MAX))
        {
            return Err(StagingMachineFigureDisplayError::PageClosure);
        }

        let mut pages: Vec<_> = selected
            .pages()
            .iter()
            .map(|page| DisplayPage {
                page_index: page.page_index(),
                width: selected.page_width(),
                height: selected.page_height(),
                commands: Vec::new(),
                annotations: Vec::new(),
            })
            .collect();
        for figure in selected.figures() {
            let page = pages
                .get_mut(figure.page_index() as usize)
                .ok_or(StagingMachineFigureDisplayError::PageClosure)?;
            if !rect_within_page(figure.rect(), page) {
                return Err(StagingMachineFigureDisplayError::PlacementOutOfBounds(
                    figure.figure_owner(),
                ));
            }
            page.commands.push(DisplayCommand::DrawImage {
                image_id: figure.image_id(),
                rect: figure.rect(),
            });
        }
        let selected_page_geometry = selected
            .pages()
            .iter()
            .map(|page| ValidatedDisplayPageGeometry {
                page_index: page.page_index(),
                master_id: selected.master_id().clone(),
                width: selected.page_width(),
                height: selected.page_height(),
            })
            .collect();
        let document = DisplayDocument {
            source_layout: DisplaySourceLayout {
                layout_epoch: selected.epoch(),
                state_fingerprint: selected.state_fingerprint(),
            },
            text_buffers: Vec::new(),
            font_instances: Vec::new(),
            destinations: Vec::new(),
            pages,
        };
        let trusted = ValidatedDisplayDocument {
            structural: StructurallyValidatedDisplayDocument {
                document,
                selected_page_geometry,
            },
        };
        let figures = selected
            .figures()
            .iter()
            .map(staging_machine_figure_display_figure)
            .collect();
        let pages = selected
            .pages()
            .iter()
            .map(staging_machine_figure_display_page)
            .collect();
        let mut facts = StagingMachineFigureDisplayFacts {
            package_sha256: selected.package_sha256(),
            flow_registry_sha256: selected.flow_registry_fingerprint().bytes(),
            figure_usage_sha256: selected.figure_usage_sha256(),
            policy_version: selected.policy_version(),
            epoch: StagingMachineFigureEpochFacts::from_layout_epoch(selected.epoch()),
            selected_state_sha256: sha256(selected.canonical_jcs().as_bytes()),
            layout_state_sha256: selected.state_fingerprint().bytes(),
            master_id: selected.master_id().clone(),
            page_width: selected.page_width(),
            page_height: selected.page_height(),
            body: selected.body(),
            pages,
            figures,
            canonical_jcs: String::new(),
        };
        facts.canonical_jcs = encode_staging_machine_figure_display(&facts);
        Ok(Self { trusted, facts })
    }

    pub const fn validated_document(&self) -> &ValidatedDisplayDocument {
        &self.trusted
    }

    pub const fn facts(&self) -> &StagingMachineFigureDisplayFacts {
        &self.facts
    }

    pub fn canonical_jcs(&self) -> &str {
        self.facts.canonical_jcs()
    }

    pub fn into_parts(self) -> (ValidatedDisplayDocument, StagingMachineFigureDisplayFacts) {
        (self.trusted, self.facts)
    }
}

fn staging_machine_figure_display_page(
    page: &StagingMachineFigureSelectedPage,
) -> StagingMachineFigureDisplayPage {
    StagingMachineFigureDisplayPage {
        page_index: page.page_index(),
        figure_count: page.figure_count(),
        caption_block_count: page.caption_block_count(),
    }
}

fn staging_machine_figure_display_figure(
    figure: &StagingMachineFigurePlacement,
) -> StagingMachineFigureDisplayFigure {
    StagingMachineFigureDisplayFigure {
        figure_node_id: figure.figure_owner().get(),
        document_ordinal: figure.document_ordinal(),
        figure_flow_id: figure.figure_flow_id().get(),
        caption_flow_id: figure.caption_flow_id().get(),
        image_id: figure.image_id(),
        alt: figure.alt().to_owned(),
        attested_media_kind: figure.admitted_media_kind(),
        admitted_sha256: figure.admitted_sha256(),
        admitted_byte_length: figure.admitted_byte_length(),
        pixel_width: figure.pixel_width(),
        pixel_height: figure.pixel_height(),
        decoded_bytes: figure.decoded_bytes(),
        page_index: figure.page_index(),
        rect: figure.rect(),
        effective_space_before: figure.effective_space_before().get().raw(),
        keep_policy: figure.keep_policy().as_str(),
        oversize_policy: figure.oversize_policy().as_str(),
        moved_to_fresh_page: figure.moved_to_fresh_page(),
        caption_fragments: figure
            .caption_fragments()
            .iter()
            .map(staging_machine_figure_display_caption)
            .collect(),
    }
}

fn staging_machine_figure_display_caption(
    caption: &StagingMachineFigureCaptionFragment,
) -> StagingMachineFigureDisplayCaption {
    StagingMachineFigureDisplayCaption {
        caption_node_id: caption.caption_owner().get(),
        caption_flow_id: caption.caption_flow_id().get(),
        page_index: caption.page_index(),
        rect: caption.rect(),
    }
}

fn encode_staging_machine_figure_display(value: &StagingMachineFigureDisplayFacts) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_FIGURE_DISPLAY_ALGORITHM);
    output.push_str(",\"body\":");
    encode_staging_machine_figure_rect(&mut output, value.body);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"figure_usage_sha256\":");
    push_quoted_staging_hex(&mut output, value.figure_usage_sha256);
    output.push_str(",\"figures\":[");
    for (index, figure) in value.figures.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_staging_machine_figure_display_figure(&mut output, figure);
    }
    output.push_str("],\"flow_registry_sha256\":");
    push_quoted_staging_hex(&mut output, value.flow_registry_sha256);
    output.push_str(",\"layout_epoch\":");
    encode_staging_machine_figure_epoch(&mut output, value.epoch);
    output.push_str(",\"layout_state_sha256\":");
    push_quoted_staging_hex(&mut output, value.layout_state_sha256);
    output.push_str(",\"master_id\":");
    push_jcs_string(&mut output, value.master_id.as_str());
    output.push_str(",\"package_sha256\":");
    push_quoted_staging_hex(&mut output, value.package_sha256);
    output.push_str(",\"page_count\":");
    output.push_str(&value.pages.len().to_string());
    output.push_str(",\"page_height\":");
    output.push_str(&value.page_height.get().raw().to_string());
    output.push_str(",\"page_width\":");
    output.push_str(&value.page_width.get().raw().to_string());
    output.push_str(",\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"caption_block_count\":");
        output.push_str(&page.caption_block_count.to_string());
        output.push_str(",\"figure_count\":");
        output.push_str(&page.figure_count.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push('}');
    }
    output.push_str("],\"policy_version\":");
    push_jcs_string(&mut output, value.policy_version);
    output.push_str(",\"selected_state_sha256\":");
    push_quoted_staging_hex(&mut output, value.selected_state_sha256);
    output.push('}');
    output
}

fn encode_staging_machine_figure_display_figure(
    output: &mut String,
    figure: &StagingMachineFigureDisplayFigure,
) {
    output.push_str("{\"admitted_byte_length\":");
    output.push_str(&figure.admitted_byte_length.to_string());
    output.push_str(",\"admitted_sha256\":");
    push_quoted_staging_hex(output, figure.admitted_sha256);
    output.push_str(",\"alt\":");
    push_jcs_string(output, &figure.alt);
    output.push_str(",\"attested_media_kind\":");
    push_jcs_string(output, figure.attested_media_kind);
    output.push_str(",\"caption_flow_id\":");
    output.push_str(&figure.caption_flow_id.to_string());
    output.push_str(",\"caption_fragments\":[");
    for (index, caption) in figure.caption_fragments.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"caption_flow_id\":");
        output.push_str(&caption.caption_flow_id.to_string());
        output.push_str(",\"caption_node_id\":");
        output.push_str(&caption.caption_node_id.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&caption.page_index.to_string());
        output.push_str(",\"rect\":");
        encode_staging_machine_figure_rect(output, caption.rect);
        output.push('}');
    }
    output.push_str("],\"decoded_bytes\":");
    output.push_str(&figure.decoded_bytes.to_string());
    output.push_str(",\"document_ordinal\":");
    output.push_str(&figure.document_ordinal.to_string());
    output.push_str(",\"draw_image_count\":1,\"effective_space_before\":");
    output.push_str(&figure.effective_space_before.to_string());
    output.push_str(",\"figure_flow_id\":");
    output.push_str(&figure.figure_flow_id.to_string());
    output.push_str(",\"figure_node_id\":");
    output.push_str(&figure.figure_node_id.to_string());
    output.push_str(",\"image_id\":");
    output.push_str(&figure.image_id.get().to_string());
    output.push_str(",\"keep_policy\":");
    push_jcs_string(output, figure.keep_policy);
    output.push_str(",\"moved_to_fresh_page\":");
    output.push_str(if figure.moved_to_fresh_page {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"oversize_policy\":");
    push_jcs_string(output, figure.oversize_policy);
    output.push_str(",\"page_index\":");
    output.push_str(&figure.page_index.to_string());
    output.push_str(",\"pixel_height\":");
    output.push_str(&figure.pixel_height.to_string());
    output.push_str(",\"pixel_width\":");
    output.push_str(&figure.pixel_width.to_string());
    output.push_str(",\"rect\":");
    encode_staging_machine_figure_rect(output, figure.rect);
    output.push('}');
}

fn encode_staging_machine_figure_epoch(output: &mut String, epoch: StagingMachineFigureEpochFacts) {
    output.push_str("{\"admitted_resources_sha256\":");
    push_quoted_staging_hex(output, epoch.admitted_resources_sha256);
    output.push_str(",\"document_sha256\":");
    push_quoted_staging_hex(output, epoch.document_sha256);
    output.push_str(",\"resolved_input_sha256\":");
    push_quoted_staging_hex(output, epoch.resolved_input_sha256);
    output.push_str(",\"style_page_master_sha256\":");
    push_quoted_staging_hex(output, epoch.style_page_master_sha256);
    output.push('}');
}

fn encode_staging_machine_figure_rect(output: &mut String, rect: Rect) {
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

fn push_quoted_staging_hex(output: &mut String, bytes: [u8; 32]) {
    output.push('"');
    push_staging_hex(output, bytes);
    output.push('"');
}

pub const STAGING_MACHINE_LINK_DISPLAY_ALGORITHM: &str = "typaxis.machine-link-display/1";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StagingMachineLinkAnnotationTamper {
    #[default]
    None,
    MissingFirst,
    ExtraFirst,
    WrongPageFirst,
    WrongTargetFirst,
    RectangleFirst,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMachineLinkDisplayError {
    ReceiptMismatch,
    EmptyLinkSet,
    MissingPaintedCluster(NodeId),
    DuplicatePaintedCluster(NodeId),
    ZeroAreaPaintedCluster(NodeId),
    RectangleLimit,
    MissingAnnotation(NodeId),
    ExtraAnnotation(NodeId),
    WrongPage(NodeId),
    WrongTarget(NodeId),
    RectangleMismatch(NodeId),
    DestinationClosure,
    NumericOverflow,
    DisplayValidation(DisplayValidationError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingMachineLinkDisplayTarget {
    Internal {
        anchor_id: AnchorId,
        anchor_owner_node_id: u32,
    },
    External {
        uri: SafeUri,
    },
}

impl StagingMachineLinkDisplayTarget {
    fn from_validated(target: &ValidatedStagingLinkTarget) -> Self {
        match target {
            ValidatedStagingLinkTarget::Internal {
                anchor_id,
                anchor_owner,
            } => Self::Internal {
                anchor_id: anchor_id.clone(),
                anchor_owner_node_id: anchor_owner.get(),
            },
            ValidatedStagingLinkTarget::External(uri) => Self::External { uri: uri.clone() },
        }
    }

    fn to_display_target(&self) -> LinkTarget {
        match self {
            Self::Internal { anchor_id, .. } => LinkTarget::Internal(anchor_id.clone()),
            Self::External { uri } => LinkTarget::Uri(uri.clone()),
        }
    }

    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Internal { .. } => "internal",
            Self::External { .. } => "external",
        }
    }

    pub const fn anchor_id(&self) -> Option<&AnchorId> {
        match self {
            Self::Internal { anchor_id, .. } => Some(anchor_id),
            Self::External { .. } => None,
        }
    }

    pub const fn anchor_owner_node_id(&self) -> Option<u32> {
        match self {
            Self::Internal {
                anchor_owner_node_id,
                ..
            } => Some(*anchor_owner_node_id),
            Self::External { .. } => None,
        }
    }

    pub const fn uri(&self) -> Option<&SafeUri> {
        match self {
            Self::Internal { .. } => None,
            Self::External { uri } => Some(uri),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineLinkDisplayRectangle {
    link_node_id: u32,
    paragraph_node_id: u32,
    page_index: u32,
    line_ordinal: u32,
    rect: Rect,
    target: StagingMachineLinkDisplayTarget,
}

impl StagingMachineLinkDisplayRectangle {
    pub const fn link_node_id(&self) -> u32 {
        self.link_node_id
    }
    pub const fn paragraph_node_id(&self) -> u32 {
        self.paragraph_node_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn line_ordinal(&self) -> u32 {
        self.line_ordinal
    }
    pub const fn rect(&self) -> Rect {
        self.rect
    }
    pub const fn target(&self) -> &StagingMachineLinkDisplayTarget {
        &self.target
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineLinkDisplayLink {
    link_node_id: u32,
    paragraph_node_id: u32,
    logical_cluster_start: u32,
    logical_cluster_end: u32,
    logical_cluster_count: u32,
    target: StagingMachineLinkDisplayTarget,
    rectangles: Vec<StagingMachineLinkDisplayRectangle>,
}

impl StagingMachineLinkDisplayLink {
    pub const fn link_node_id(&self) -> u32 {
        self.link_node_id
    }
    pub const fn paragraph_node_id(&self) -> u32 {
        self.paragraph_node_id
    }
    pub const fn logical_cluster_start(&self) -> u32 {
        self.logical_cluster_start
    }
    pub const fn logical_cluster_end(&self) -> u32 {
        self.logical_cluster_end
    }
    pub const fn logical_cluster_count(&self) -> u32 {
        self.logical_cluster_count
    }
    pub const fn target(&self) -> &StagingMachineLinkDisplayTarget {
        &self.target
    }
    pub fn rectangles(&self) -> &[StagingMachineLinkDisplayRectangle] {
        &self.rectangles
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineLinkDisplayDestination {
    anchor_id: AnchorId,
    owner_node_id: u32,
    page_index: u32,
    point: Point,
}

impl StagingMachineLinkDisplayDestination {
    pub const fn anchor_id(&self) -> &AnchorId {
        &self.anchor_id
    }
    pub const fn owner_node_id(&self) -> u32 {
        self.owner_node_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn point(&self) -> Point {
        self.point
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineLinkDisplayPage {
    page_index: u32,
    width: PositiveLength,
    height: PositiveLength,
    annotation_count: u32,
}

impl StagingMachineLinkDisplayPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn width(&self) -> PositiveLength {
        self.width
    }
    pub const fn height(&self) -> PositiveLength {
        self.height
    }
    pub const fn annotation_count(&self) -> u32 {
        self.annotation_count
    }
}

/// Cloneable link facts retained after the publication-trusted Display value
/// moves into the PDF backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineLinkDisplayFacts {
    package_sha256: [u8; 32],
    usage_sha256: [u8; 32],
    cluster_receipt_sha256: [u8; 32],
    selected_state_sha256: [u8; 32],
    layout_state_sha256: [u8; 32],
    epoch: LayoutEpoch,
    policy_version: &'static str,
    pages: Vec<StagingMachineLinkDisplayPage>,
    destinations: Vec<StagingMachineLinkDisplayDestination>,
    links: Vec<StagingMachineLinkDisplayLink>,
    annotation_count: u32,
    canonical_jcs: String,
}

impl StagingMachineLinkDisplayFacts {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn usage_sha256(&self) -> [u8; 32] {
        self.usage_sha256
    }
    pub const fn cluster_receipt_sha256(&self) -> [u8; 32] {
        self.cluster_receipt_sha256
    }
    pub const fn selected_state_sha256(&self) -> [u8; 32] {
        self.selected_state_sha256
    }
    pub const fn layout_state_sha256(&self) -> [u8; 32] {
        self.layout_state_sha256
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn policy_version(&self) -> &'static str {
        self.policy_version
    }
    pub fn pages(&self) -> &[StagingMachineLinkDisplayPage] {
        &self.pages
    }
    pub fn destinations(&self) -> &[StagingMachineLinkDisplayDestination] {
        &self.destinations
    }
    pub fn links(&self) -> &[StagingMachineLinkDisplayLink] {
        &self.links
    }
    pub const fn annotation_count(&self) -> u32 {
        self.annotation_count
    }
    pub fn annotations(&self) -> impl Iterator<Item = &StagingMachineLinkDisplayRectangle> {
        self.links.iter().flat_map(|link| link.rectangles.iter())
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

/// MI2-07 Display owner. Glyph commands are first produced by the existing
/// selected-paragraph painter; this owner derives only annotations from the
/// same selected cluster/line receipts and then revalidates the complete
/// Display document.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingMachineLinkDisplay {
    validated: ValidatedDisplayDocument,
    facts: StagingMachineLinkDisplayFacts,
}

impl StagingMachineLinkDisplay {
    pub fn from_selected(
        package: &ValidatedStagingStylePackage,
        selected: &PaginationResult,
        flow: &FlowTree,
        links: &ValidatedStagingMachineLinkClusters,
        config: &EffectiveConfig,
    ) -> Result<Self, StagingMachineLinkDisplayError> {
        Self::from_selected_with_tamper(
            package,
            selected,
            flow,
            links,
            config,
            StagingMachineLinkAnnotationTamper::None,
        )
    }

    #[doc(hidden)]
    pub fn from_selected_with_tamper(
        package: &ValidatedStagingStylePackage,
        selected: &PaginationResult,
        flow: &FlowTree,
        links: &ValidatedStagingMachineLinkClusters,
        config: &EffectiveConfig,
        tamper: StagingMachineLinkAnnotationTamper,
    ) -> Result<Self, StagingMachineLinkDisplayError> {
        let registry = flow
            .paragraph_items()
            .ok_or(StagingMachineLinkDisplayError::ReceiptMismatch)?;
        if flow != selected.selected_flow()
            || !links.verifies(package, registry)
            || links.ranges().is_empty()
        {
            return Err(if links.ranges().is_empty() {
                StagingMachineLinkDisplayError::EmptyLinkSet
            } else {
                StagingMachineLinkDisplayError::ReceiptMismatch
            });
        }

        let base = ValidatedDisplayDocument::paint_reference_paragraphs(
            package.package(),
            selected,
            flow,
            config,
        )
        .map_err(StagingMachineLinkDisplayError::DisplayValidation)?;
        let expected = derive_staging_machine_link_rectangles(
            selected,
            registry,
            links,
            config.limits().get().max_fragments,
        )?;
        let mut observed = expected.clone();
        apply_staging_machine_link_tamper(&mut observed, tamper)?;
        close_staging_machine_link_annotations(&expected, &observed)?;

        let (mut document, _) = base.into_parts();
        for page in &mut document.pages {
            page.annotations.clear();
        }
        for annotation in &observed {
            let page = document
                .pages
                .get_mut(annotation.page_index as usize)
                .ok_or(StagingMachineLinkDisplayError::WrongPage(NodeId::new(
                    annotation.link_node_id,
                )))?;
            page.annotations.push(LinkAnnotation {
                target: annotation.target.to_display_target(),
                rect: annotation.rect,
            });
        }
        let structural = StructurallyValidatedDisplayDocument::new(
            document,
            package.package(),
            selected,
            config,
        )
        .map_err(StagingMachineLinkDisplayError::DisplayValidation)?;
        let validated = ValidatedDisplayDocument { structural };
        let destinations = staging_machine_link_destinations(package, &validated)?;
        let link_facts = staging_machine_link_facts(links, &expected)?;
        let pages = validated
            .document()
            .pages
            .iter()
            .map(|page| {
                Ok(StagingMachineLinkDisplayPage {
                    page_index: page.page_index,
                    width: page.width,
                    height: page.height,
                    annotation_count: u32::try_from(page.annotations.len())
                        .map_err(|_| StagingMachineLinkDisplayError::NumericOverflow)?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let annotation_count = u32::try_from(expected.len())
            .map_err(|_| StagingMachineLinkDisplayError::NumericOverflow)?;
        let layout_state_sha256 = selected.final_fingerprint().bytes();
        let cluster_receipt_sha256 = sha256(links.canonical_jcs().as_bytes());
        let mut selected_state_bytes = Vec::new();
        selected_state_bytes.extend_from_slice(&layout_state_sha256);
        selected_state_bytes.extend_from_slice(&cluster_receipt_sha256);
        let selected_state_sha256 = sha256(&selected_state_bytes);
        let mut facts = StagingMachineLinkDisplayFacts {
            package_sha256: links.package_sha256(),
            usage_sha256: links.usage_sha256(),
            cluster_receipt_sha256,
            selected_state_sha256,
            layout_state_sha256,
            epoch: links.epoch(),
            policy_version: STAGING_BASIC_LINK_POLICY_VERSION,
            pages,
            destinations,
            links: link_facts,
            annotation_count,
            canonical_jcs: String::new(),
        };
        facts.canonical_jcs = encode_staging_machine_link_display(&facts);
        Ok(Self { validated, facts })
    }

    pub const fn validated_document(&self) -> &ValidatedDisplayDocument {
        &self.validated
    }
    pub const fn facts(&self) -> &StagingMachineLinkDisplayFacts {
        &self.facts
    }
    pub fn canonical_jcs(&self) -> &str {
        self.facts.canonical_jcs()
    }
    pub fn into_parts(self) -> (ValidatedDisplayDocument, StagingMachineLinkDisplayFacts) {
        (self.validated, self.facts)
    }
}

fn derive_staging_machine_link_rectangles(
    selected: &PaginationResult,
    registry: &ValidatedParagraphItemRegistry,
    links: &ValidatedStagingMachineLinkClusters,
    max_rectangles: u64,
) -> Result<Vec<StagingMachineLinkDisplayRectangle>, StagingMachineLinkDisplayError> {
    let expected_clusters: std::collections::BTreeSet<_> = links
        .ranges()
        .iter()
        .flat_map(|range| range.clusters().iter().copied())
        .collect();
    let mut seen_clusters = std::collections::BTreeSet::new();
    let mut unions: std::collections::BTreeMap<(NodeId, u32, u32), Rect> =
        std::collections::BTreeMap::new();
    for page in selected.selected_pages() {
        for fragment in &page.fragments {
            if registry.item_count(fragment.owner).is_none() {
                continue;
            }
            let logical = fragment_shaped_slices(registry, fragment)
                .map_err(StagingMachineLinkDisplayError::DisplayValidation)?;
            if logical.is_empty() {
                continue;
            }
            let levels: Vec<_> = logical
                .iter()
                .map(|slice| slice.shaped.bidi_level())
                .collect();
            let classes: Vec<_> = logical.iter().map(|slice| slice.class).collect();
            let paragraph_level = registry
                .paragraph_level(fragment.owner)
                .ok_or(StagingMachineLinkDisplayError::ReceiptMismatch)?;
            let after_l1 = reset_line_bidi_levels(paragraph_level, &levels, &classes)
                .map_err(|_| StagingMachineLinkDisplayError::ReceiptMismatch)?;
            let mut logical =
                reference_final_line_reshape(registry, fragment.owner, logical, &after_l1)
                    .map_err(StagingMachineLinkDisplayError::DisplayValidation)?;
            justify_reference_line(registry, fragment, &mut logical)
                .map_err(StagingMachineLinkDisplayError::DisplayValidation)?;
            let order = reorder_line_l2(&after_l1)
                .map_err(|_| StagingMachineLinkDisplayError::ReceiptMismatch)?;
            let mut x = fragment.bounds.x();
            for logical_index in order.visual_to_logical() {
                let slice = logical
                    .get(*logical_index as usize)
                    .ok_or(StagingMachineLinkDisplayError::ReceiptMismatch)?;
                if let Some((range, cluster)) = links.range_for_shaped(fragment.owner, slice.shaped)
                {
                    if !seen_clusters.insert(cluster) {
                        return Err(StagingMachineLinkDisplayError::DuplicatePaintedCluster(
                            range.link_node(),
                        ));
                    }
                    let width = PositiveLength::new(slice.advance).ok_or(
                        StagingMachineLinkDisplayError::ZeroAreaPaintedCluster(range.link_node()),
                    )?;
                    let cluster_rect =
                        Rect::new(x, fragment.bounds.y(), width, fragment.bounds.height());
                    let union_key = (
                        range.link_node(),
                        page.page_index,
                        fragment.owner_local_ordinal,
                    );
                    if !unions.contains_key(&union_key)
                        && u64::try_from(unions.len()).unwrap_or(u64::MAX) >= max_rectangles
                    {
                        return Err(StagingMachineLinkDisplayError::RectangleLimit);
                    }
                    match unions.entry(union_key) {
                        std::collections::btree_map::Entry::Occupied(mut entry) => {
                            let union = union_staging_machine_link_rect(*entry.get(), cluster_rect)
                                .ok_or(StagingMachineLinkDisplayError::NumericOverflow)?;
                            entry.insert(union);
                        }
                        std::collections::btree_map::Entry::Vacant(entry) => {
                            entry.insert(cluster_rect);
                        }
                    }
                }
                x = x
                    .checked_add(slice.advance)
                    .ok_or(StagingMachineLinkDisplayError::NumericOverflow)?;
            }
        }
    }
    if seen_clusters != expected_clusters {
        let owner = links
            .ranges()
            .iter()
            .find(|range| {
                range
                    .clusters()
                    .iter()
                    .any(|cluster| !seen_clusters.contains(cluster))
            })
            .map(ValidatedStagingMachineLinkClusterRange::link_node)
            .unwrap_or(NodeId::new(0));
        return Err(StagingMachineLinkDisplayError::MissingPaintedCluster(owner));
    }
    let mut rectangles = Vec::new();
    rectangles
        .try_reserve_exact(unions.len())
        .map_err(|_| StagingMachineLinkDisplayError::NumericOverflow)?;
    for ((link_node, page_index, line_ordinal), rect) in unions {
        let range = links
            .ranges()
            .iter()
            .find(|range| range.link_node() == link_node)
            .ok_or(StagingMachineLinkDisplayError::ReceiptMismatch)?;
        rectangles.push(StagingMachineLinkDisplayRectangle {
            link_node_id: link_node.get(),
            paragraph_node_id: range.paragraph_node().get(),
            page_index,
            line_ordinal,
            rect,
            target: StagingMachineLinkDisplayTarget::from_validated(range.target()),
        });
    }
    for range in links.ranges() {
        if rectangles
            .iter()
            .all(|rectangle| rectangle.link_node_id != range.link_node().get())
        {
            return Err(StagingMachineLinkDisplayError::MissingPaintedCluster(
                range.link_node(),
            ));
        }
    }
    Ok(rectangles)
}

struct FootnoteMachineLinkCollector<'a> {
    links: &'a ValidatedStagingMachineLinkClusters,
    seen_clusters: std::collections::BTreeSet<StagingMachineLinkClusterKey>,
    unions: std::collections::BTreeMap<(NodeId, u32, u32), Rect>,
    max_rectangles: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FootnoteMachineLinkClusterObservation {
    paragraph_owner: NodeId,
    shaped: ShapedSlice,
    page_index: u32,
    line_ordinal: u32,
    x: Length,
    line_bounds: Rect,
    advance: Length,
}

impl<'a> FootnoteMachineLinkCollector<'a> {
    fn new(links: &'a ValidatedStagingMachineLinkClusters, max_rectangles: u64) -> Self {
        Self {
            links,
            seen_clusters: std::collections::BTreeSet::new(),
            unions: std::collections::BTreeMap::new(),
            max_rectangles,
        }
    }

    fn observe(
        &mut self,
        observation: FootnoteMachineLinkClusterObservation,
    ) -> Result<(), StagingMachineLinkDisplayError> {
        let FootnoteMachineLinkClusterObservation {
            paragraph_owner,
            shaped,
            page_index,
            line_ordinal,
            x,
            line_bounds,
            advance,
        } = observation;
        let Some((range, cluster)) = self.links.range_for_shaped(paragraph_owner, shaped) else {
            return Ok(());
        };
        if !self.seen_clusters.insert(cluster) {
            return Err(StagingMachineLinkDisplayError::DuplicatePaintedCluster(
                range.link_node(),
            ));
        }
        let width = PositiveLength::new(advance).ok_or(
            StagingMachineLinkDisplayError::ZeroAreaPaintedCluster(range.link_node()),
        )?;
        let rect = Rect::new(x, line_bounds.y(), width, line_bounds.height());
        let key = (range.link_node(), page_index, line_ordinal);
        if !self.unions.contains_key(&key)
            && u64::try_from(self.unions.len()).unwrap_or(u64::MAX) >= self.max_rectangles
        {
            return Err(StagingMachineLinkDisplayError::RectangleLimit);
        }
        match self.unions.entry(key) {
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let union = union_staging_machine_link_rect(*entry.get(), rect)
                    .ok_or(StagingMachineLinkDisplayError::NumericOverflow)?;
                entry.insert(union);
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(rect);
            }
        }
        Ok(())
    }

    fn finish(
        self,
    ) -> Result<Vec<StagingMachineLinkDisplayRectangle>, StagingMachineLinkDisplayError> {
        let expected: std::collections::BTreeSet<_> = self
            .links
            .ranges()
            .iter()
            .flat_map(|range| range.clusters().iter().copied())
            .collect();
        if self.seen_clusters != expected {
            let owner = self
                .links
                .ranges()
                .iter()
                .find(|range| {
                    range
                        .clusters()
                        .iter()
                        .any(|cluster| !self.seen_clusters.contains(cluster))
                })
                .map(ValidatedStagingMachineLinkClusterRange::link_node)
                .unwrap_or(NodeId::new(0));
            return Err(StagingMachineLinkDisplayError::MissingPaintedCluster(owner));
        }
        let mut rectangles = Vec::new();
        rectangles
            .try_reserve_exact(self.unions.len())
            .map_err(|_| StagingMachineLinkDisplayError::NumericOverflow)?;
        for ((link_node, page_index, line_ordinal), rect) in self.unions {
            let range = self
                .links
                .ranges()
                .iter()
                .find(|range| range.link_node() == link_node)
                .ok_or(StagingMachineLinkDisplayError::ReceiptMismatch)?;
            rectangles.push(StagingMachineLinkDisplayRectangle {
                link_node_id: link_node.get(),
                paragraph_node_id: range.paragraph_node().get(),
                page_index,
                line_ordinal,
                rect,
                target: StagingMachineLinkDisplayTarget::from_validated(range.target()),
            });
        }
        for range in self.links.ranges() {
            if rectangles
                .iter()
                .all(|rectangle| rectangle.link_node_id != range.link_node().get())
            {
                return Err(StagingMachineLinkDisplayError::MissingPaintedCluster(
                    range.link_node(),
                ));
            }
        }
        Ok(rectangles)
    }
}

fn union_staging_machine_link_rect(left: Rect, right: Rect) -> Option<Rect> {
    let min_x = left.x().raw().min(right.x().raw());
    let min_y = left.y().raw().min(right.y().raw());
    let max_x = left
        .x()
        .raw()
        .checked_add(left.width().get().raw())?
        .max(right.x().raw().checked_add(right.width().get().raw())?);
    let max_y = left
        .y()
        .raw()
        .checked_add(left.height().get().raw())?
        .max(right.y().raw().checked_add(right.height().get().raw())?);
    Some(Rect::new(
        Length::from_raw(min_x)?,
        Length::from_raw(min_y)?,
        PositiveLength::new(Length::from_raw(max_x.checked_sub(min_x)?)?)?,
        PositiveLength::new(Length::from_raw(max_y.checked_sub(min_y)?)?)?,
    ))
}

fn apply_staging_machine_link_tamper(
    annotations: &mut Vec<StagingMachineLinkDisplayRectangle>,
    tamper: StagingMachineLinkAnnotationTamper,
) -> Result<(), StagingMachineLinkDisplayError> {
    let Some(first) = annotations.first().cloned() else {
        return Err(StagingMachineLinkDisplayError::EmptyLinkSet);
    };
    match tamper {
        StagingMachineLinkAnnotationTamper::None => {}
        StagingMachineLinkAnnotationTamper::MissingFirst => {
            annotations.remove(0);
        }
        StagingMachineLinkAnnotationTamper::ExtraFirst => annotations.push(first),
        StagingMachineLinkAnnotationTamper::WrongPageFirst => {
            annotations[0].page_index = annotations[0]
                .page_index
                .checked_add(1)
                .ok_or(StagingMachineLinkDisplayError::NumericOverflow)?;
        }
        StagingMachineLinkAnnotationTamper::WrongTargetFirst => {
            annotations[0].target = StagingMachineLinkDisplayTarget::External {
                uri: SafeUri::new("https://tamper.invalid/").map_err(|_| {
                    StagingMachineLinkDisplayError::WrongTarget(NodeId::new(first.link_node_id))
                })?,
            };
        }
        StagingMachineLinkAnnotationTamper::RectangleFirst => {
            let rect = annotations[0].rect;
            let x = rect
                .x()
                .checked_add(Length::from_raw(1).expect("one is a valid fixed-point length"))
                .ok_or(StagingMachineLinkDisplayError::NumericOverflow)?;
            annotations[0].rect = Rect::new(x, rect.y(), rect.width(), rect.height());
        }
    }
    Ok(())
}

fn close_staging_machine_link_annotations(
    expected: &[StagingMachineLinkDisplayRectangle],
    observed: &[StagingMachineLinkDisplayRectangle],
) -> Result<(), StagingMachineLinkDisplayError> {
    if observed.len() < expected.len() {
        let missing = expected
            .iter()
            .find(|candidate| !observed.contains(candidate))
            .unwrap_or(&expected[0]);
        return Err(StagingMachineLinkDisplayError::MissingAnnotation(
            NodeId::new(missing.link_node_id),
        ));
    }
    if observed.len() > expected.len() {
        let extra = observed
            .iter()
            .find(|candidate| !expected.contains(candidate))
            .unwrap_or(&observed[observed.len() - 1]);
        return Err(StagingMachineLinkDisplayError::ExtraAnnotation(
            NodeId::new(extra.link_node_id),
        ));
    }
    for (expected, observed) in expected.iter().zip(observed) {
        let owner = NodeId::new(expected.link_node_id);
        if expected.link_node_id != observed.link_node_id
            || expected.paragraph_node_id != observed.paragraph_node_id
            || expected.line_ordinal != observed.line_ordinal
            || expected.page_index != observed.page_index
        {
            return Err(StagingMachineLinkDisplayError::WrongPage(owner));
        }
        if expected.target != observed.target {
            return Err(StagingMachineLinkDisplayError::WrongTarget(owner));
        }
        if expected.rect != observed.rect {
            return Err(StagingMachineLinkDisplayError::RectangleMismatch(owner));
        }
    }
    Ok(())
}

fn staging_machine_link_destinations(
    package: &ValidatedStagingStylePackage,
    display: &ValidatedDisplayDocument,
) -> Result<Vec<StagingMachineLinkDisplayDestination>, StagingMachineLinkDisplayError> {
    let destinations = &display.document().destinations;
    if destinations.len() != package.package().document_nodes().anchors().len() {
        return Err(StagingMachineLinkDisplayError::DestinationClosure);
    }
    destinations
        .iter()
        .map(|destination| {
            let DestinationView::Xyz { point } = destination.view else {
                return Err(StagingMachineLinkDisplayError::DestinationClosure);
            };
            let owner = package
                .package()
                .document_nodes()
                .anchor_owner(&destination.anchor_id)
                .ok_or(StagingMachineLinkDisplayError::DestinationClosure)?;
            Ok(StagingMachineLinkDisplayDestination {
                anchor_id: destination.anchor_id.clone(),
                owner_node_id: owner.get(),
                page_index: destination.page_index,
                point,
            })
        })
        .collect()
}

fn staging_machine_link_facts(
    clusters: &ValidatedStagingMachineLinkClusters,
    rectangles: &[StagingMachineLinkDisplayRectangle],
) -> Result<Vec<StagingMachineLinkDisplayLink>, StagingMachineLinkDisplayError> {
    clusters
        .ranges()
        .iter()
        .map(|range| {
            let link_rectangles: Vec<_> = rectangles
                .iter()
                .filter(|rectangle| rectangle.link_node_id == range.link_node().get())
                .cloned()
                .collect();
            if link_rectangles.is_empty() {
                return Err(StagingMachineLinkDisplayError::MissingPaintedCluster(
                    range.link_node(),
                ));
            }
            Ok(StagingMachineLinkDisplayLink {
                link_node_id: range.link_node().get(),
                paragraph_node_id: range.paragraph_node().get(),
                logical_cluster_start: range.logical_start(),
                logical_cluster_end: range.logical_end(),
                logical_cluster_count: u32::try_from(range.clusters().len())
                    .map_err(|_| StagingMachineLinkDisplayError::NumericOverflow)?,
                target: StagingMachineLinkDisplayTarget::from_validated(range.target()),
                rectangles: link_rectangles,
            })
        })
        .collect()
}

fn encode_staging_machine_link_display(value: &StagingMachineLinkDisplayFacts) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_LINK_DISPLAY_ALGORITHM);
    output.push_str(",\"annotation_count\":");
    output.push_str(&value.annotation_count.to_string());
    output.push_str(",\"cluster_receipt_sha256\":");
    push_quoted_staging_hex(&mut output, value.cluster_receipt_sha256);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"destinations\":[");
    for (index, destination) in value.destinations.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"anchor_id\":");
        push_jcs_string(&mut output, destination.anchor_id.as_str());
        output.push_str(",\"owner_node_id\":");
        output.push_str(&destination.owner_node_id.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&destination.page_index.to_string());
        output.push_str(",\"point\":");
        encode_staging_machine_link_point(&mut output, destination.point);
        output.push('}');
    }
    output.push_str("],\"layout_epoch\":");
    encode_staging_machine_figure_epoch(
        &mut output,
        StagingMachineFigureEpochFacts::from_layout_epoch(value.epoch),
    );
    output.push_str(",\"layout_state_sha256\":");
    push_quoted_staging_hex(&mut output, value.layout_state_sha256);
    output.push_str(",\"links\":[");
    for (index, link) in value.links.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"link_node_id\":");
        output.push_str(&link.link_node_id.to_string());
        output.push_str(",\"logical_cluster_count\":");
        output.push_str(&link.logical_cluster_count.to_string());
        output.push_str(",\"logical_cluster_end\":");
        output.push_str(&link.logical_cluster_end.to_string());
        output.push_str(",\"logical_cluster_start\":");
        output.push_str(&link.logical_cluster_start.to_string());
        output.push_str(",\"paragraph_node_id\":");
        output.push_str(&link.paragraph_node_id.to_string());
        output.push_str(",\"rectangles\":[");
        for (rectangle_index, rectangle) in link.rectangles.iter().enumerate() {
            if rectangle_index > 0 {
                output.push(',');
            }
            output.push_str("{\"line_ordinal\":");
            output.push_str(&rectangle.line_ordinal.to_string());
            output.push_str(",\"page_index\":");
            output.push_str(&rectangle.page_index.to_string());
            output.push_str(",\"rect\":");
            encode_staging_machine_figure_rect(&mut output, rectangle.rect);
            output.push('}');
        }
        output.push_str("],\"target\":");
        encode_staging_machine_link_target(&mut output, &link.target);
        output.push('}');
    }
    output.push_str("],\"package_sha256\":");
    push_quoted_staging_hex(&mut output, value.package_sha256);
    output.push_str(",\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"annotation_count\":");
        output.push_str(&page.annotation_count.to_string());
        output.push_str(",\"height\":");
        output.push_str(&page.height.get().raw().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"width\":");
        output.push_str(&page.width.get().raw().to_string());
        output.push('}');
    }
    output.push_str("],\"policy_version\":");
    push_jcs_string(&mut output, value.policy_version);
    output.push_str(
        ",\"profile\":\"typaxis.machine-pdf/basic-document-1\",\"selected_state_sha256\":",
    );
    push_quoted_staging_hex(&mut output, value.selected_state_sha256);
    output.push_str(",\"usage_sha256\":");
    push_quoted_staging_hex(&mut output, value.usage_sha256);
    output.push('}');
    output
}

fn encode_staging_machine_link_target(
    output: &mut String,
    target: &StagingMachineLinkDisplayTarget,
) {
    match target {
        StagingMachineLinkDisplayTarget::Internal {
            anchor_id,
            anchor_owner_node_id,
        } => {
            output.push_str("{\"anchor_id\":");
            push_jcs_string(output, anchor_id.as_str());
            output.push_str(",\"anchor_owner_node_id\":");
            output.push_str(&anchor_owner_node_id.to_string());
            output.push_str(",\"kind\":\"internal\"}");
        }
        StagingMachineLinkDisplayTarget::External { uri } => {
            output.push_str("{\"kind\":\"external\",\"uri\":");
            push_jcs_string(output, uri.as_str());
            output.push('}');
        }
    }
}

fn encode_staging_machine_link_point(output: &mut String, point: Point) {
    output.push_str("{\"x\":");
    output.push_str(&point.x.raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&point.y.raw().to_string());
    output.push('}');
}

pub const STAGING_FORCED_PAGE_BREAK_DISPLAY_ALGORITHM: &str = "typaxis.forced-page-break-display/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingForcedPageBreakDisplayError {
    BreakClosure,
    ExtraBreakPaint(NodeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakDisplayPage {
    page_index: u32,
    painted_content_count: u32,
}

impl StagingForcedPageBreakDisplayPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn painted_content_count(&self) -> u32 {
        self.painted_content_count
    }

    pub const fn is_blank(&self) -> bool {
        self.painted_content_count == 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakDisplayBoundary {
    break_node_id: u32,
    document_ordinal: u32,
    flow_id: u32,
    before_flow_local_ordinal: u32,
    after_flow_local_ordinal: u32,
    produced_page_index: u32,
}

impl StagingForcedPageBreakDisplayBoundary {
    pub const fn break_node_id(&self) -> u32 {
        self.break_node_id
    }

    pub const fn document_ordinal(&self) -> u32 {
        self.document_ordinal
    }

    pub const fn flow_id(&self) -> u32 {
        self.flow_id
    }

    pub const fn before_flow_local_ordinal(&self) -> u32 {
        self.before_flow_local_ordinal
    }

    pub const fn after_flow_local_ordinal(&self) -> u32 {
        self.after_flow_local_ordinal
    }

    pub const fn produced_page_index(&self) -> u32 {
        self.produced_page_index
    }
}

/// Display-stage forced-break observation. It intentionally owns no paint
/// operations; attempting to close a break-derived paint is a terminal error.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakDisplay {
    package_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    usage_sha256: [u8; 32],
    policy_version: &'static str,
    page_count: u32,
    pages: Vec<StagingForcedPageBreakDisplayPage>,
    breaks: Vec<StagingForcedPageBreakDisplayBoundary>,
    canonical_jcs: String,
}

impl StagingForcedPageBreakDisplay {
    pub fn from_selected(
        selected: &StagingForcedPageBreakSelectedState,
    ) -> Result<Self, StagingForcedPageBreakDisplayError> {
        Self::from_selected_break_paint_owners(selected, &[])
    }

    /// Closure seam for paint workers. Since a forced boundary has no Display
    /// operation, the only exact accepted set is empty.
    #[doc(hidden)]
    pub fn from_selected_break_paint_owners(
        selected: &StagingForcedPageBreakSelectedState,
        break_paint_owners: &[NodeId],
    ) -> Result<Self, StagingForcedPageBreakDisplayError> {
        selected
            .validate_break_closure()
            .map_err(|_| StagingForcedPageBreakDisplayError::BreakClosure)?;
        validate_staging_forced_page_break_paint_owners(break_paint_owners)?;
        let pages = selected
            .pages()
            .iter()
            .map(staging_forced_page_break_display_page)
            .collect();
        let breaks = selected
            .breaks()
            .iter()
            .map(staging_forced_page_break_display_boundary)
            .collect();
        let mut value = Self {
            package_sha256: selected.package_sha256(),
            flow_registry_sha256: selected.flow_registry_fingerprint().bytes(),
            usage_sha256: selected.usage_sha256(),
            policy_version: selected.policy_version(),
            page_count: selected.page_count(),
            pages,
            breaks,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_forced_page_break_display(&value);
        Ok(value)
    }

    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }

    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }

    pub const fn usage_sha256(&self) -> [u8; 32] {
        self.usage_sha256
    }

    pub const fn policy_version(&self) -> &'static str {
        self.policy_version
    }

    pub const fn page_count(&self) -> u32 {
        self.page_count
    }

    pub fn pages(&self) -> &[StagingForcedPageBreakDisplayPage] {
        &self.pages
    }

    pub fn breaks(&self) -> &[StagingForcedPageBreakDisplayBoundary] {
        &self.breaks
    }

    pub const fn paint_operation_count(&self) -> usize {
        0
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    #[cfg(any(test, feature = "staging-fixtures"))]
    #[doc(hidden)]
    pub fn forced_page_break_pdf_test_fixture() -> Self {
        let mut value = Self {
            package_sha256: [0x11; 32],
            flow_registry_sha256: [0x22; 32],
            usage_sha256: [0x33; 32],
            policy_version: "typaxis.basic-forced-page-break-policy/1",
            page_count: 2,
            pages: vec![
                StagingForcedPageBreakDisplayPage {
                    page_index: 0,
                    painted_content_count: 0,
                },
                StagingForcedPageBreakDisplayPage {
                    page_index: 1,
                    painted_content_count: 0,
                },
            ],
            breaks: vec![StagingForcedPageBreakDisplayBoundary {
                break_node_id: 1,
                document_ordinal: 0,
                flow_id: 0,
                before_flow_local_ordinal: 0,
                after_flow_local_ordinal: 1,
                produced_page_index: 1,
            }],
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_forced_page_break_display(&value);
        value
    }
}

fn validate_staging_forced_page_break_paint_owners(
    break_paint_owners: &[NodeId],
) -> Result<(), StagingForcedPageBreakDisplayError> {
    match break_paint_owners.first().copied() {
        Some(owner) => Err(StagingForcedPageBreakDisplayError::ExtraBreakPaint(owner)),
        None => Ok(()),
    }
}

fn staging_forced_page_break_display_page(
    page: &StagingForcedPageBreakSelectedPage,
) -> StagingForcedPageBreakDisplayPage {
    StagingForcedPageBreakDisplayPage {
        page_index: page.page_index(),
        painted_content_count: page.painted_content_count(),
    }
}

fn staging_forced_page_break_display_boundary(
    boundary: &StagingForcedPageBreakConsumeReceipt,
) -> StagingForcedPageBreakDisplayBoundary {
    debug_assert_eq!(
        boundary.before_cursor().flow_id(),
        boundary.after_cursor().flow_id()
    );
    StagingForcedPageBreakDisplayBoundary {
        break_node_id: boundary.break_owner().get(),
        document_ordinal: boundary.document_ordinal(),
        flow_id: boundary.before_cursor().flow_id().get(),
        before_flow_local_ordinal: boundary.before_cursor().flow_local_ordinal(),
        after_flow_local_ordinal: boundary.after_cursor().flow_local_ordinal(),
        produced_page_index: boundary.produced_page_index(),
    }
}

fn encode_staging_forced_page_break_display(value: &StagingForcedPageBreakDisplay) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_FORCED_PAGE_BREAK_DISPLAY_ALGORITHM);
    output.push_str(",\"break_usage_sha256\":\"");
    push_staging_hex(&mut output, value.usage_sha256);
    output.push_str("\",\"contract\":\"typaxis.contract/1.2\",\"flow_registry_sha256\":\"");
    push_staging_hex(&mut output, value.flow_registry_sha256);
    output.push_str("\",\"forced_page_breaks\":[");
    for (index, boundary) in value.breaks.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_staging_forced_page_break_display_boundary(&mut output, boundary);
    }
    output.push_str("],\"package_sha256\":\"");
    push_staging_hex(&mut output, value.package_sha256);
    output.push_str("\",\"page_count\":");
    output.push_str(&value.page_count.to_string());
    output.push_str(",\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"is_blank\":");
        output.push_str(if page.is_blank() { "true" } else { "false" });
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"painted_content_count\":");
        output.push_str(&page.painted_content_count.to_string());
        output.push('}');
    }
    output.push_str("],\"paint_operations\":[],\"policy_version\":");
    push_jcs_string(&mut output, value.policy_version);
    output.push('}');
    output
}

fn encode_staging_forced_page_break_display_boundary(
    output: &mut String,
    boundary: &StagingForcedPageBreakDisplayBoundary,
) {
    output.push_str("{\"after_cursor\":{\"flow_id\":");
    output.push_str(&boundary.flow_id.to_string());
    output.push_str(",\"flow_local_ordinal\":");
    output.push_str(&boundary.after_flow_local_ordinal.to_string());
    output.push_str("},\"before_cursor\":{\"flow_id\":");
    output.push_str(&boundary.flow_id.to_string());
    output.push_str(",\"flow_local_ordinal\":");
    output.push_str(&boundary.before_flow_local_ordinal.to_string());
    output.push_str("},\"break_node_id\":");
    output.push_str(&boundary.break_node_id.to_string());
    output.push_str(",\"document_ordinal\":");
    output.push_str(&boundary.document_ordinal.to_string());
    output.push_str(",\"produced_page_index\":");
    output.push_str(&boundary.produced_page_index.to_string());
    output.push('}');
}

pub const STAGING_MACHINE_LIST_DISPLAY_ALGORITHM: &str = "typaxis.machine-list-display/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMachineListDisplayError {
    MarkerClosure,
    MissingItem(u32),
    ExtraItem(u32),
    WrongItem { expected: u32, actual: u32 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListDisplayList {
    list_node_id: u32,
    list_flow_id: u32,
    marker_column_width: i64,
    marker_gap: i64,
    start_indent: i64,
    end_indent: i64,
    item_frame_inline_size: i64,
}

impl StagingMachineListDisplayList {
    pub const fn list_node_id(&self) -> u32 {
        self.list_node_id
    }
    pub const fn list_flow_id(&self) -> u32 {
        self.list_flow_id
    }
    pub const fn marker_column_width(&self) -> i64 {
        self.marker_column_width
    }
    pub const fn marker_gap(&self) -> i64 {
        self.marker_gap
    }
    pub const fn start_indent(&self) -> i64 {
        self.start_indent
    }
    pub const fn end_indent(&self) -> i64 {
        self.end_indent
    }
    pub const fn item_frame_inline_size(&self) -> i64 {
        self.item_frame_inline_size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListDisplayItem {
    list_node_id: u32,
    item_node_id: u32,
    item_index: u32,
    list_flow_id: u32,
    item_flow_id: u32,
    marker_key: GeneratedBufferKey,
    marker_utf8: String,
    marker_fragment_id: u64,
    first_line_fragment_id: u64,
    page_index: u32,
    fragment_ids: Vec<u64>,
    marker_inline_size: i64,
    marker_column_width: i64,
    marker_physical_left: i64,
    content_physical_left: i64,
    content_inline_size: i64,
    first_line_inline_size: i64,
    first_line_block_size: i64,
    block_offset: i64,
}

impl StagingMachineListDisplayItem {
    pub const fn list_node_id(&self) -> u32 {
        self.list_node_id
    }
    pub const fn item_node_id(&self) -> u32 {
        self.item_node_id
    }
    pub const fn item_index(&self) -> u32 {
        self.item_index
    }
    pub const fn list_flow_id(&self) -> u32 {
        self.list_flow_id
    }
    pub const fn item_flow_id(&self) -> u32 {
        self.item_flow_id
    }
    pub const fn marker_key(&self) -> GeneratedBufferKey {
        self.marker_key
    }
    pub fn marker_utf8(&self) -> &str {
        &self.marker_utf8
    }
    pub const fn marker_fragment_id(&self) -> u64 {
        self.marker_fragment_id
    }
    pub const fn first_line_fragment_id(&self) -> u64 {
        self.first_line_fragment_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn fragment_ids(&self) -> &[u64] {
        &self.fragment_ids
    }
    pub const fn marker_inline_size(&self) -> i64 {
        self.marker_inline_size
    }
    pub const fn marker_column_width(&self) -> i64 {
        self.marker_column_width
    }
    pub const fn marker_physical_left(&self) -> i64 {
        self.marker_physical_left
    }
    pub const fn content_physical_left(&self) -> i64 {
        self.content_physical_left
    }
    pub const fn content_inline_size(&self) -> i64 {
        self.content_inline_size
    }
    pub const fn first_line_inline_size(&self) -> i64 {
        self.first_line_inline_size
    }
    pub const fn first_line_block_size(&self) -> i64 {
        self.first_line_block_size
    }
    pub const fn block_offset(&self) -> i64 {
        self.block_offset
    }
}

/// Staging Display observation issued only from a selected list state. Marker
/// bytes and item identities are copied; no Display caller supplies labels.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingMachineListDisplay {
    package_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    marker_usage_sha256: [u8; 32],
    policy_version: &'static str,
    page_count: u32,
    lists: Vec<StagingMachineListDisplayList>,
    items: Vec<StagingMachineListDisplayItem>,
    canonical_jcs: String,
}

fn validate_staging_machine_list_item_order(
    expected_owners: &[u32],
    item_owners: &[u32],
) -> Result<(), StagingMachineListDisplayError> {
    for (index, expected) in expected_owners.iter().copied().enumerate() {
        let Some(actual) = item_owners.get(index).copied() else {
            return Err(StagingMachineListDisplayError::MissingItem(expected));
        };
        if actual != expected {
            return Err(StagingMachineListDisplayError::WrongItem { expected, actual });
        }
    }
    if let Some(extra) = item_owners.get(expected_owners.len()).copied() {
        return Err(StagingMachineListDisplayError::ExtraItem(extra));
    }
    Ok(())
}

impl StagingMachineListDisplay {
    pub fn from_selected(
        selected: &StagingMachineListSelectedState,
    ) -> Result<Self, StagingMachineListDisplayError> {
        let owners: Vec<_> = selected
            .items()
            .iter()
            .map(|item| item.item_owner().get())
            .collect();
        Self::from_selected_item_order(selected, &owners)
    }

    /// Explicit closure seam used by tests and parallel painters. The order
    /// must cover the selected item registry exactly once.
    #[doc(hidden)]
    pub fn from_selected_item_order(
        selected: &StagingMachineListSelectedState,
        item_owners: &[u32],
    ) -> Result<Self, StagingMachineListDisplayError> {
        selected
            .validate_marker_closure()
            .map_err(|_| StagingMachineListDisplayError::MarkerClosure)?;
        let expected_owners: Vec<_> = selected
            .items()
            .iter()
            .map(|item| item.item_owner().get())
            .collect();
        validate_staging_machine_list_item_order(&expected_owners, item_owners)?;
        let lists = selected
            .lists()
            .iter()
            .map(|list| StagingMachineListDisplayList {
                list_node_id: list.list_owner().get(),
                list_flow_id: list.list_flow_id().get(),
                marker_column_width: list.marker_column_width(),
                marker_gap: list.marker_gap(),
                start_indent: list.start_indent(),
                end_indent: list.end_indent(),
                item_frame_inline_size: list.item_frame_inline_size(),
            })
            .collect();
        let items = selected
            .items()
            .iter()
            .map(|item| StagingMachineListDisplayItem {
                list_node_id: item.list_owner().get(),
                item_node_id: item.item_owner().get(),
                item_index: item.item_index(),
                list_flow_id: item.list_flow_id().get(),
                item_flow_id: item.item_flow_id().get(),
                marker_key: item.marker_key(),
                marker_utf8: item.marker_utf8().to_owned(),
                marker_fragment_id: item.marker_fragment_id(),
                first_line_fragment_id: item.first_line_fragment_id(),
                page_index: item.page_index(),
                fragment_ids: item.fragment_ids().to_vec(),
                marker_inline_size: item.marker_inline_size(),
                marker_column_width: item.marker_column_width(),
                marker_physical_left: item.marker_physical_left(),
                content_physical_left: item.content_physical_left(),
                content_inline_size: item.content_inline_size(),
                first_line_inline_size: item.first_line_inline_size(),
                first_line_block_size: item.first_line_block_size(),
                block_offset: item.block_offset(),
            })
            .collect();
        let mut value = Self {
            package_sha256: selected.package_sha256(),
            flow_registry_sha256: selected.flow_registry_fingerprint().bytes(),
            marker_usage_sha256: selected.marker_usage_sha256(),
            policy_version: selected.policy_version(),
            page_count: selected.page_count(),
            lists,
            items,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_list_display(&value);
        Ok(value)
    }

    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }
    pub const fn marker_usage_sha256(&self) -> [u8; 32] {
        self.marker_usage_sha256
    }
    pub const fn policy_version(&self) -> &'static str {
        self.policy_version
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub fn lists(&self) -> &[StagingMachineListDisplayList] {
        &self.lists
    }
    pub fn items(&self) -> &[StagingMachineListDisplayItem] {
        &self.items
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    #[cfg(any(test, feature = "staging-fixtures"))]
    #[doc(hidden)]
    pub fn list_pdf_test_fixture() -> Self {
        let key = GeneratedBufferKey::new(
            typaxis_core::NodeId::new(2),
            typaxis_core::GenerationKind::ListMarker,
            0,
        );
        let mut value = Self {
            package_sha256: [0xcc; 32],
            flow_registry_sha256: [0xdd; 32],
            marker_usage_sha256: [0xee; 32],
            policy_version: "typaxis.basic-list-policy/1",
            page_count: 1,
            lists: vec![StagingMachineListDisplayList {
                list_node_id: 1,
                list_flow_id: 0,
                marker_column_width: 8,
                marker_gap: 10,
                start_indent: 5,
                end_indent: 3,
                item_frame_inline_size: 74,
            }],
            items: vec![StagingMachineListDisplayItem {
                list_node_id: 1,
                item_node_id: 2,
                item_index: 0,
                list_flow_id: 0,
                item_flow_id: 1,
                marker_key: key,
                marker_utf8: "1.".to_owned(),
                marker_fragment_id: 0,
                first_line_fragment_id: 0,
                page_index: 0,
                fragment_ids: vec![0],
                marker_inline_size: 8,
                marker_column_width: 8,
                marker_physical_left: 5,
                content_physical_left: 23,
                content_inline_size: 74,
                first_line_inline_size: 20,
                first_line_block_size: 12,
                block_offset: 0,
            }],
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_list_display(&value);
        value
    }
}

fn encode_staging_machine_list_display(value: &StagingMachineListDisplay) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_LIST_DISPLAY_ALGORITHM);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"flow_registry_sha256\":\"");
    push_staging_hex(&mut output, value.flow_registry_sha256);
    output.push_str("\",\"items\":[");
    for (index, item) in value.items.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_staging_machine_list_display_item(&mut output, item);
    }
    output.push_str("],\"list_flows\":[");
    for (index, list) in value.lists.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"end_indent\":");
        output.push_str(&list.end_indent.to_string());
        output.push_str(",\"item_frame_inline_size\":");
        output.push_str(&list.item_frame_inline_size.to_string());
        output.push_str(",\"list_flow_id\":");
        output.push_str(&list.list_flow_id.to_string());
        output.push_str(",\"list_node_id\":");
        output.push_str(&list.list_node_id.to_string());
        output.push_str(",\"marker_column_width\":");
        output.push_str(&list.marker_column_width.to_string());
        output.push_str(",\"marker_gap\":");
        output.push_str(&list.marker_gap.to_string());
        output.push_str(",\"start_indent\":");
        output.push_str(&list.start_indent.to_string());
        output.push('}');
    }
    output.push_str("],\"marker_usage_sha256\":\"");
    push_staging_hex(&mut output, value.marker_usage_sha256);
    output.push_str("\",\"package_sha256\":\"");
    push_staging_hex(&mut output, value.package_sha256);
    output.push_str("\",\"page_count\":");
    output.push_str(&value.page_count.to_string());
    output.push_str(",\"policy_version\":");
    push_jcs_string(&mut output, value.policy_version);
    output.push('}');
    output
}

fn encode_staging_machine_list_display_item(
    output: &mut String,
    item: &StagingMachineListDisplayItem,
) {
    output.push_str("{\"block_offset\":");
    output.push_str(&item.block_offset.to_string());
    output.push_str(",\"content_inline_size\":");
    output.push_str(&item.content_inline_size.to_string());
    output.push_str(",\"content_physical_left\":");
    output.push_str(&item.content_physical_left.to_string());
    output.push_str(",\"first_line_block_size\":");
    output.push_str(&item.first_line_block_size.to_string());
    output.push_str(",\"first_line_fragment_id\":");
    output.push_str(&item.first_line_fragment_id.to_string());
    output.push_str(",\"first_line_inline_size\":");
    output.push_str(&item.first_line_inline_size.to_string());
    output.push_str(",\"fragment_ids\":[");
    for (index, fragment) in item.fragment_ids.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&fragment.to_string());
    }
    output.push_str("],\"item_flow_id\":");
    output.push_str(&item.item_flow_id.to_string());
    output.push_str(",\"item_index\":");
    output.push_str(&item.item_index.to_string());
    output.push_str(",\"item_node_id\":");
    output.push_str(&item.item_node_id.to_string());
    output.push_str(",\"list_flow_id\":");
    output.push_str(&item.list_flow_id.to_string());
    output.push_str(",\"list_node_id\":");
    output.push_str(&item.list_node_id.to_string());
    output.push_str(",\"marker_column_width\":");
    output.push_str(&item.marker_column_width.to_string());
    output.push_str(",\"marker_fragment_id\":");
    output.push_str(&item.marker_fragment_id.to_string());
    output.push_str(",\"marker_inline_size\":");
    output.push_str(&item.marker_inline_size.to_string());
    output.push_str(",\"marker_key\":");
    push_generated_buffer_key_jcs(output, item.marker_key);
    output.push_str(",\"marker_physical_left\":");
    output.push_str(&item.marker_physical_left.to_string());
    output.push_str(",\"marker_utf8\":");
    push_jcs_string(output, &item.marker_utf8);
    output.push_str(",\"page_index\":");
    output.push_str(&item.page_index.to_string());
    output.push('}');
}

pub const TABLE_PAINT_CLOSURE_ALGORITHM: &str = "typaxis.table-paint-closure/1";

/// The first table profile has no authored or inferred table decoration. Any
/// observation in this enum is an extra paint operation and is rejected before
/// a publication-trusted Display document can be issued.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableDecorationObservation {
    Background,
    Border,
    BorderSpacing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TablePaintOccurrenceKind {
    Header,
    Body,
}

impl TablePaintOccurrenceKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Body => "body",
        }
    }
}

/// Exact selected cell-slice rectangle. A structural zero-height row remains
/// representable even though it cannot become a `DisplayCommand` rectangle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TablePaintRect {
    x: i64,
    y: i64,
    width: i64,
    height: i64,
}

impl TablePaintRect {
    pub const fn from_untrusted_parts(x: i64, y: i64, width: i64, height: i64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub const fn x(self) -> i64 {
        self.x
    }
    pub const fn y(self) -> i64 {
        self.y
    }
    pub const fn width(self) -> i64 {
        self.width
    }
    pub const fn height(self) -> i64 {
        self.height
    }
}

/// Untrusted worker observation for one selected cell slice. Every field is
/// compared with a receipt-derived record; construction never implies trust.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TablePaintCellObservation {
    pub kind: TablePaintOccurrenceKind,
    pub page_index: u32,
    pub fragment_id: u64,
    pub source_fragment_id: Option<u64>,
    pub repetition_index: Option<u32>,
    pub row_node_id: NodeId,
    pub logical_row_ordinal: u32,
    pub row_fragment_ordinal: u32,
    pub cell_node_id: NodeId,
    pub flow_id: FlowId,
    pub column_ordinal: u32,
    pub colspan: u16,
    pub rowspan: u16,
    pub rect: TablePaintRect,
    pub content_fragment_start: u32,
    pub content_fragment_end: u32,
    pub vertical_offset_before: i64,
    pub vertical_offset_after: i64,
}

/// Exact table-owned command and its position in the final page command
/// stream. Retaining the command lets the PDF closure reopen the frozen graph
/// instead of trusting a caller-reported operation count.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TablePaintCommandObservation {
    pub page_index: u32,
    pub page_command_index: u32,
    pub fragment_id: u64,
    pub repetition_index: Option<u32>,
    pub cell_node_id: NodeId,
    pub command: DisplayCommand,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableDisplayClosureError {
    SelectedStateMismatch,
    PageClosure,
    MissingCell,
    ExtraCell,
    WrongCell,
    WrongPage,
    WrongRepetition,
    WrongRectangle,
    WrongContentRange,
    NonCanonicalOrder,
    DecorationForbidden,
    ArithmeticOverflow,
    AllocationFailure,
}

/// Dense selected-page body geometry supplied by the page-layout owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TablePaintPageBody {
    selected_page_index: u32,
    target_page_index: u32,
    body: Rect,
    table_block_offset: NonNegativeLength,
}

impl TablePaintPageBody {
    pub const fn new(page_index: u32, body: Rect) -> Self {
        Self {
            selected_page_index: page_index,
            target_page_index: page_index,
            body,
            table_block_offset: NonNegativeLength::ZERO,
        }
    }
    pub const fn new_at(selected_page_index: u32, target_page_index: u32, body: Rect) -> Self {
        Self::new_at_offset(
            selected_page_index,
            target_page_index,
            body,
            NonNegativeLength::ZERO,
        )
    }
    pub const fn new_at_offset(
        selected_page_index: u32,
        target_page_index: u32,
        body: Rect,
        table_block_offset: NonNegativeLength,
    ) -> Self {
        Self {
            selected_page_index,
            target_page_index,
            body,
            table_block_offset,
        }
    }
    pub const fn selected_page_index(self) -> u32 {
        self.selected_page_index
    }
    pub const fn target_page_index(self) -> u32 {
        self.target_page_index
    }
    pub const fn body(self) -> Rect {
        self.body
    }
    pub const fn table_block_offset(self) -> NonNegativeLength {
        self.table_block_offset
    }
}

/// Complete Display-stage closure for one selected table. Records are ordered
/// by page, header-before-body occurrence, logical row/fragment, cell origin,
/// and then the child flow's own fragment order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableDisplayClosureReceipt {
    layout_state_sha256: [u8; 32],
    package_sha256: [u8; 32],
    grid_sha256: [u8; 32],
    row_band_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    table_node_id: NodeId,
    page_bodies: Vec<TablePaintPageBody>,
    records: Vec<TablePaintCellObservation>,
    commands: Vec<TablePaintCommandObservation>,
    decoration_op_count: u32,
    fingerprint: [u8; 32],
    canonical_jcs: String,
}

impl TableDisplayClosureReceipt {
    pub fn from_selected(
        layout_state_sha256: [u8; 32],
        grid: &ValidatedTableGridReceipt,
        layout: &TableRowBandLayoutReceipt,
        selected: &SelectedTableLayoutReceipt,
        page_bodies: Vec<TablePaintPageBody>,
    ) -> Result<Self, TableDisplayClosureError> {
        let observations = derive_table_paint_observations(grid, layout, selected, &page_bodies)?;
        Self::from_observed(
            layout_state_sha256,
            grid,
            layout,
            selected,
            page_bodies,
            observations,
            &[],
        )
    }

    pub fn from_observed(
        layout_state_sha256: [u8; 32],
        grid: &ValidatedTableGridReceipt,
        layout: &TableRowBandLayoutReceipt,
        selected: &SelectedTableLayoutReceipt,
        page_bodies: Vec<TablePaintPageBody>,
        observed: Vec<TablePaintCellObservation>,
        decorations: &[TableDecorationObservation],
    ) -> Result<Self, TableDisplayClosureError> {
        reject_table_decorations(decorations)?;
        let expected = derive_table_paint_observations(grid, layout, selected, &page_bodies)?;
        validate_table_display_records(&expected, &observed)?;

        let mut value = Self {
            layout_state_sha256,
            package_sha256: layout.package_sha256(),
            grid_sha256: grid.fingerprint().bytes(),
            row_band_sha256: layout.fingerprint(),
            selected_layout_sha256: selected.fingerprint().bytes(),
            table_node_id: selected.table_owner(),
            page_bodies,
            records: observed,
            commands: Vec::new(),
            decoration_op_count: 0,
            fingerprint: [0; 32],
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_table_display_closure(&value);
        value.fingerprint = sha256(value.canonical_jcs.as_bytes());
        Ok(value)
    }

    fn bind_painted_commands(
        &mut self,
        commands: Vec<TablePaintCommandObservation>,
        decorations: &[TableDecorationObservation],
    ) -> Result<(), TableDisplayClosureError> {
        if !decorations.is_empty() {
            return Err(TableDisplayClosureError::DecorationForbidden);
        }
        let mut previous = None;
        for observation in &commands {
            if !matches!(observation.command, DisplayCommand::DrawGlyphRun { .. }) {
                return Err(TableDisplayClosureError::DecorationForbidden);
            }
            let key = (observation.page_index, observation.page_command_index);
            if previous.is_some_and(|value| value >= key) {
                return Err(TableDisplayClosureError::NonCanonicalOrder);
            }
            previous = Some(key);
            if !self.records.iter().any(|record| {
                record.page_index == observation.page_index
                    && record.fragment_id == observation.fragment_id
                    && record.repetition_index == observation.repetition_index
                    && record.cell_node_id == observation.cell_node_id
            }) {
                return Err(TableDisplayClosureError::WrongCell);
            }
        }
        self.commands = commands;
        self.canonical_jcs = encode_table_display_closure(self);
        self.fingerprint = sha256(self.canonical_jcs.as_bytes());
        Ok(())
    }

    pub const fn layout_state_sha256(&self) -> [u8; 32] {
        self.layout_state_sha256
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn grid_sha256(&self) -> [u8; 32] {
        self.grid_sha256
    }
    pub const fn row_band_sha256(&self) -> [u8; 32] {
        self.row_band_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub const fn table_node_id(&self) -> NodeId {
        self.table_node_id
    }
    pub fn page_bodies(&self) -> &[TablePaintPageBody] {
        &self.page_bodies
    }
    pub fn records(&self) -> &[TablePaintCellObservation] {
        &self.records
    }
    pub fn commands(&self) -> &[TablePaintCommandObservation] {
        &self.commands
    }
    pub const fn decoration_op_count(&self) -> u32 {
        self.decoration_op_count
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn reject_table_decorations(
    decorations: &[TableDecorationObservation],
) -> Result<(), TableDisplayClosureError> {
    if decorations.is_empty() {
        Ok(())
    } else {
        Err(TableDisplayClosureError::DecorationForbidden)
    }
}

fn validate_table_display_records(
    expected: &[TablePaintCellObservation],
    observed: &[TablePaintCellObservation],
) -> Result<(), TableDisplayClosureError> {
    if observed.len() < expected.len() {
        return Err(TableDisplayClosureError::MissingCell);
    }
    if observed.len() > expected.len() {
        return Err(TableDisplayClosureError::ExtraCell);
    }
    for (actual, expected) in observed.iter().zip(expected) {
        if actual == expected {
            continue;
        }
        if actual.page_index != expected.page_index {
            return Err(TableDisplayClosureError::WrongPage);
        }
        if actual.kind != expected.kind
            || actual.repetition_index != expected.repetition_index
            || actual.source_fragment_id != expected.source_fragment_id
        {
            return Err(TableDisplayClosureError::WrongRepetition);
        }
        if actual.row_node_id != expected.row_node_id
            || actual.logical_row_ordinal != expected.logical_row_ordinal
            || actual.row_fragment_ordinal != expected.row_fragment_ordinal
            || actual.cell_node_id != expected.cell_node_id
            || actual.flow_id != expected.flow_id
            || actual.column_ordinal != expected.column_ordinal
            || actual.colspan != expected.colspan
            || actual.rowspan != expected.rowspan
        {
            return Err(TableDisplayClosureError::WrongCell);
        }
        if actual.rect != expected.rect {
            return Err(TableDisplayClosureError::WrongRectangle);
        }
        if actual.content_fragment_start != expected.content_fragment_start
            || actual.content_fragment_end != expected.content_fragment_end
            || actual.vertical_offset_before != expected.vertical_offset_before
            || actual.vertical_offset_after != expected.vertical_offset_after
        {
            return Err(TableDisplayClosureError::WrongContentRange);
        }
        return Err(TableDisplayClosureError::NonCanonicalOrder);
    }
    Ok(())
}

/// Receipt-only inputs for one table in the public `table-1` painter. Every
/// reference is reclosed before paint; the constructor itself grants no trust.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FootnotePaintCommandKind {
    ReferenceMarker,
    Separator,
    Definition,
}

/// Exact Display command retained for PDF-side footnote observation. Body
/// commands remain covered by the ordinary selected-layout fingerprint; every
/// separator and definition command is additionally bound here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FootnotePaintCommandObservation {
    page_index: u32,
    page_command_index: u32,
    kind: FootnotePaintCommandKind,
    assignment_ordinal: Option<u32>,
    flow_id: Option<FootnoteFlowId>,
    footnote_id: Option<FootnoteId>,
    fragment_ordinal: Option<u32>,
    reference_owner: Option<NodeId>,
    command: DisplayCommand,
}

impl FootnotePaintCommandObservation {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn page_command_index(&self) -> u32 {
        self.page_command_index
    }
    pub const fn kind(&self) -> FootnotePaintCommandKind {
        self.kind
    }
    pub const fn assignment_ordinal(&self) -> Option<u32> {
        self.assignment_ordinal
    }
    pub const fn flow_id(&self) -> Option<FootnoteFlowId> {
        self.flow_id
    }
    pub const fn footnote_id(&self) -> Option<&FootnoteId> {
        self.footnote_id.as_ref()
    }
    pub const fn fragment_ordinal(&self) -> Option<u32> {
        self.fragment_ordinal
    }
    pub const fn reference_owner(&self) -> Option<NodeId> {
        self.reference_owner
    }
    pub const fn command(&self) -> &DisplayCommand {
        &self.command
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FootnotePaintReferenceObservation {
    footnote_id: FootnoteId,
    reference_owner: NodeId,
}

impl FootnotePaintReferenceObservation {
    pub const fn footnote_id(&self) -> &FootnoteId {
        &self.footnote_id
    }
    pub const fn reference_owner(&self) -> NodeId {
        self.reference_owner
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FootnotePaintFlowObservation {
    footnote_id: FootnoteId,
    assignment_ordinal: u32,
    flow_id: FootnoteFlowId,
    before_fragment: u32,
    after_fragment: u32,
    incoming_source_page: Option<u32>,
    carries_out: bool,
}

impl FootnotePaintFlowObservation {
    pub const fn footnote_id(&self) -> &FootnoteId {
        &self.footnote_id
    }
    pub const fn assignment_ordinal(&self) -> u32 {
        self.assignment_ordinal
    }
    pub const fn flow_id(&self) -> FootnoteFlowId {
        self.flow_id
    }
    pub const fn before_fragment(&self) -> u32 {
        self.before_fragment
    }
    pub const fn after_fragment(&self) -> u32 {
        self.after_fragment
    }
    pub const fn incoming_source_page(&self) -> Option<u32> {
        self.incoming_source_page
    }
    pub const fn carries_out(&self) -> bool {
        self.carries_out
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FootnotePaintPageObservation {
    page_index: u32,
    body_continuation_position: u32,
    body_continuation_terminal: bool,
    body_fingerprint: LayoutStateFingerprint,
    body_command_count: u32,
    evaluation_count: u32,
    reservation: NonNegativeLength,
    ordered_footnote_ids: Vec<FootnoteId>,
    references: Vec<FootnotePaintReferenceObservation>,
    flows: Vec<FootnotePaintFlowObservation>,
}

impl FootnotePaintPageObservation {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn body_continuation_position(&self) -> u32 {
        self.body_continuation_position
    }
    pub const fn body_continuation_terminal(&self) -> bool {
        self.body_continuation_terminal
    }
    pub const fn body_fingerprint(&self) -> LayoutStateFingerprint {
        self.body_fingerprint
    }
    pub const fn body_command_count(&self) -> u32 {
        self.body_command_count
    }
    pub const fn evaluation_count(&self) -> u32 {
        self.evaluation_count
    }
    pub const fn reservation(&self) -> NonNegativeLength {
        self.reservation
    }
    pub fn ordered_footnote_ids(&self) -> &[FootnoteId] {
        &self.ordered_footnote_ids
    }
    pub fn references(&self) -> &[FootnotePaintReferenceObservation] {
        &self.references
    }
    pub fn flows(&self) -> &[FootnotePaintFlowObservation] {
        &self.flows
    }
}

/// MI3-07 paint closure derived from the selected body and dedicated
/// definition-flow receipts. It is intentionally retained by the PDF graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FootnoteDisplayClosureReceipt {
    profile_sha256: [u8; 32],
    registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    body_layout_sha256: [u8; 32],
    pages: Vec<FootnotePaintPageObservation>,
    commands: Vec<FootnotePaintCommandObservation>,
    canonical_jcs: String,
}

impl FootnoteDisplayClosureReceipt {
    pub const fn profile_sha256(&self) -> [u8; 32] {
        self.profile_sha256
    }
    pub const fn registry_sha256(&self) -> [u8; 32] {
        self.registry_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub const fn body_layout_sha256(&self) -> [u8; 32] {
        self.body_layout_sha256
    }
    pub fn pages(&self) -> &[FootnotePaintPageObservation] {
        &self.pages
    }
    pub fn commands(&self) -> &[FootnotePaintCommandObservation] {
        &self.commands
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        sha256(self.canonical_jcs.as_bytes())
    }

    /// Fixed serializer-bound closure fixture; unavailable in production
    /// builds. Its commands are opaque because PDF receipt tests exercise the
    /// already-validated Display-to-byte binding rather than Display paint
    /// validation itself.
    #[cfg(feature = "staging-fixtures")]
    #[doc(hidden)]
    pub fn serializer_pdf_test_fixture() -> Self {
        let footnote_id = FootnoteId::new("a").unwrap();
        let flow_id = FootnoteFlowId::new(0);
        let reference_owner = NodeId::new(1);
        let mut value = Self {
            profile_sha256: [1; 32],
            registry_sha256: [2; 32],
            selected_layout_sha256: [3; 32],
            body_layout_sha256: [4; 32],
            pages: vec![FootnotePaintPageObservation {
                page_index: 0,
                body_continuation_position: 1,
                body_continuation_terminal: true,
                body_fingerprint: LayoutStateFingerprint::from_untrusted_bytes([5; 32]),
                body_command_count: 1,
                evaluation_count: 2,
                reservation: NonNegativeLength::new(Length::from_raw(1).unwrap()).unwrap(),
                ordered_footnote_ids: vec![footnote_id.clone()],
                references: vec![FootnotePaintReferenceObservation {
                    footnote_id: footnote_id.clone(),
                    reference_owner,
                }],
                flows: vec![FootnotePaintFlowObservation {
                    footnote_id: footnote_id.clone(),
                    assignment_ordinal: 0,
                    flow_id,
                    before_fragment: 0,
                    after_fragment: 1,
                    incoming_source_page: None,
                    carries_out: false,
                }],
            }],
            commands: vec![
                FootnotePaintCommandObservation {
                    page_index: 0,
                    page_command_index: 0,
                    kind: FootnotePaintCommandKind::ReferenceMarker,
                    assignment_ordinal: None,
                    flow_id: None,
                    footnote_id: Some(footnote_id.clone()),
                    fragment_ordinal: None,
                    reference_owner: Some(reference_owner),
                    command: DisplayCommand::Save,
                },
                FootnotePaintCommandObservation {
                    page_index: 0,
                    page_command_index: 1,
                    kind: FootnotePaintCommandKind::Separator,
                    assignment_ordinal: None,
                    flow_id: None,
                    footnote_id: None,
                    fragment_ordinal: None,
                    reference_owner: None,
                    command: DisplayCommand::Restore,
                },
                FootnotePaintCommandObservation {
                    page_index: 0,
                    page_command_index: 2,
                    kind: FootnotePaintCommandKind::Definition,
                    assignment_ordinal: Some(0),
                    flow_id: Some(flow_id),
                    footnote_id: Some(footnote_id),
                    fragment_ordinal: Some(0),
                    reference_owner: None,
                    command: DisplayCommand::Save,
                },
            ],
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_footnote_display_closure(&value);
        value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FootnoteProfileDisplayError {
    SelectedLayoutMismatch,
    RegistryMismatch,
    DefinitionFragmentMismatch,
    DefinitionPaintOrder,
    SeparatorMismatch,
    Link(StagingMachineLinkDisplayError),
    NumericOverflow,
    Display(DisplayValidationError),
}

pub struct FootnoteProfileDisplay {
    trusted: ValidatedDisplayDocument,
    closure: FootnoteDisplayClosureReceipt,
}

impl FootnoteProfileDisplay {
    pub const fn validated_document(&self) -> &ValidatedDisplayDocument {
        &self.trusted
    }
    pub const fn closure(&self) -> &FootnoteDisplayClosureReceipt {
        &self.closure
    }
    pub fn into_parts(self) -> (ValidatedDisplayDocument, FootnoteDisplayClosureReceipt) {
        (self.trusted, self.closure)
    }
}

pub struct TableProfilePaintInput<'a> {
    grid: &'a ValidatedTableGridReceipt,
    layout: &'a TableRowBandLayoutReceipt,
    selected: &'a SelectedTableLayoutReceipt,
    page_bodies: &'a [TablePaintPageBody],
    paragraph_items: &'a ValidatedParagraphItemRegistry,
}

impl<'a> TableProfilePaintInput<'a> {
    pub const fn new(
        grid: &'a ValidatedTableGridReceipt,
        layout: &'a TableRowBandLayoutReceipt,
        selected: &'a SelectedTableLayoutReceipt,
        page_bodies: &'a [TablePaintPageBody],
        paragraph_items: &'a ValidatedParagraphItemRegistry,
    ) -> Self {
        Self {
            grid,
            layout,
            selected,
            page_bodies,
            paragraph_items,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableProfileDisplayError {
    Closure(TableDisplayClosureError),
    TableSetMismatch,
    ParagraphRegistryMismatch,
    CellContentMismatch,
    CellInlineOverflow,
    CellBidiMismatch,
    CellPaintBoundsMismatch,
    NumericOverflow,
    Link(StagingMachineLinkDisplayError),
    Display(DisplayValidationError),
}

/// Publication-trusted table Display document plus the exact per-table paint
/// receipts consumed by the PDF closure.
pub struct TableProfileDisplay {
    trusted: ValidatedDisplayDocument,
    tables: Vec<TableDisplayClosureReceipt>,
}

impl TableProfileDisplay {
    pub const fn validated_document(&self) -> &ValidatedDisplayDocument {
        &self.trusted
    }
    pub fn table_closures(&self) -> &[TableDisplayClosureReceipt] {
        &self.tables
    }
    pub fn into_parts(self) -> (ValidatedDisplayDocument, Vec<TableDisplayClosureReceipt>) {
        (self.trusted, self.tables)
    }
}

fn validate_table_profile_page_mapping(
    package: &ValidatedParsedPackage,
    pagination: &PaginationResult,
    tables: &[TableProfilePaintInput<'_>],
    body: Rect,
) -> Result<(), TableDisplayClosureError> {
    let body_height = body.height().get().raw();
    let mut next_target_page = 0u32;
    for input in tables {
        if input.grid.frame_inline_size() != body.width() {
            return Err(TableDisplayClosureError::PageClosure);
        }
        let first_row_owner = input
            .grid
            .rows()
            .first()
            .map(|row| row.row_owner())
            .ok_or(TableDisplayClosureError::PageClosure)?;
        let (source_page, source_y) = pagination
            .selected_pages()
            .iter()
            .find_map(|page| {
                page.fragments
                    .iter()
                    .find(|fragment| fragment.owner == first_row_owner)
                    .map(|fragment| (page.page_index, fragment.bounds.y().raw()))
            })
            .ok_or(TableDisplayClosureError::PageClosure)?;
        let mut target_page = source_page.max(next_target_page);
        let mut first_offset = if target_page == source_page {
            source_y
                .checked_sub(body.y().raw())
                .and_then(|value| value.checked_add(input.grid.space_before().get().raw()))
                .ok_or(TableDisplayClosureError::ArithmeticOverflow)?
        } else {
            0
        };
        if first_offset < 0 {
            return Err(TableDisplayClosureError::PageClosure);
        }
        if first_offset >= body_height {
            target_page = target_page
                .checked_add(1)
                .ok_or(TableDisplayClosureError::ArithmeticOverflow)?;
            first_offset = 0;
        }
        let first_remaining = body_height
            .checked_sub(first_offset)
            .ok_or(TableDisplayClosureError::ArithmeticOverflow)?;
        if input.selected.body_block_size() != body_height
            || input.selected.first_page_remaining_block_size() != first_remaining
            || input.page_bodies.len()
                != usize::try_from(input.selected.page_count())
                    .map_err(|_| TableDisplayClosureError::ArithmeticOverflow)?
            || input.page_bodies.iter().enumerate().any(|(index, page)| {
                let Ok(index) = u32::try_from(index) else {
                    return true;
                };
                let Some(expected_target_page) = target_page.checked_add(index) else {
                    return true;
                };
                page.selected_page_index() != index
                    || page.target_page_index() != expected_target_page
                    || page.body() != body
                    || page.table_block_offset() != NonNegativeLength::ZERO
            })
        {
            return Err(TableDisplayClosureError::PageClosure);
        }
        next_target_page = target_page
            .checked_add(input.selected.page_count())
            .ok_or(TableDisplayClosureError::ArithmeticOverflow)?;
    }
    if package.package().page_masters.masters.len() != 1 {
        return Err(TableDisplayClosureError::PageClosure);
    }
    Ok(())
}

fn validate_table_display_inputs(
    grid: &ValidatedTableGridReceipt,
    layout: &TableRowBandLayoutReceipt,
    selected: &SelectedTableLayoutReceipt,
    page_bodies: &[TablePaintPageBody],
) -> Result<(), TableDisplayClosureError> {
    if grid.package_sha256() != layout.package_sha256()
        || grid.epoch() != layout.epoch()
        || grid.flow_registry() != layout.flow_registry_fingerprint()
        || grid.fingerprint() != layout.grid_fingerprint()
        || grid.table_owner() != layout.table_owner()
        || selected.package_sha256() != layout.package_sha256()
        || selected.epoch() != layout.epoch()
        || selected.flow_registry_fingerprint() != layout.flow_registry_fingerprint()
        || selected.grid_sha256() != grid.fingerprint().bytes()
        || selected.row_band_sha256() != layout.fingerprint()
        || selected.table_owner() != layout.table_owner()
    {
        return Err(TableDisplayClosureError::SelectedStateMismatch);
    }
    let page_count = usize::try_from(selected.page_count())
        .map_err(|_| TableDisplayClosureError::ArithmeticOverflow)?;
    if page_bodies.len() != page_count
        || page_bodies.iter().enumerate().any(|(index, page)| {
            u32::try_from(index) != Ok(page.selected_page_index)
                || page.body.height().get().raw() != selected.body_block_size()
        })
        || page_bodies
            .windows(2)
            .any(|pair| pair[0].target_page_index.checked_add(1) != Some(pair[1].target_page_index))
    {
        return Err(TableDisplayClosureError::PageClosure);
    }
    // Selected header/body offsets are already relative to the complete body
    // frame; the first-page consumed prefix is sealed by the selected receipt
    // and must not be added a second time by Display page mapping.
    if page_bodies
        .iter()
        .any(|page| page.table_block_offset != NonNegativeLength::ZERO)
    {
        return Err(TableDisplayClosureError::PageClosure);
    }
    Ok(())
}

fn derive_table_paint_observations(
    grid: &ValidatedTableGridReceipt,
    layout: &TableRowBandLayoutReceipt,
    selected: &SelectedTableLayoutReceipt,
    page_bodies: &[TablePaintPageBody],
) -> Result<Vec<TablePaintCellObservation>, TableDisplayClosureError> {
    validate_table_display_inputs(grid, layout, selected, page_bodies)?;
    let mut records = Vec::new();
    for page in selected.pages() {
        let body = page_bodies
            .get(
                usize::try_from(page.page_index())
                    .map_err(|_| TableDisplayClosureError::ArithmeticOverflow)?,
            )
            .ok_or(TableDisplayClosureError::PageClosure)?
            .to_owned();
        let target_page_index = body.target_page_index;
        let table_block_offset = body.table_block_offset.get().raw();
        let body = body.body;
        if let Some(repetition_index) = page.header_repetition_index() {
            let repetition = selected
                .header_repetitions()
                .iter()
                .find(|receipt| {
                    receipt.repetition_index() == repetition_index
                        && receipt.target_page_index() == page.page_index()
                })
                .ok_or(TableDisplayClosureError::WrongRepetition)?;
            for occurrence in repetition.rows() {
                let source = selected
                    .header_sources()
                    .iter()
                    .find(|source| {
                        source.source_fragment_id() == occurrence.source_fragment_id()
                            && source.row_owner() == occurrence.row_owner()
                    })
                    .ok_or(TableDisplayClosureError::WrongRepetition)?;
                for cell in source.cells() {
                    records
                        .try_reserve(1)
                        .map_err(|_| TableDisplayClosureError::AllocationFailure)?;
                    records.push(table_paint_record(
                        grid,
                        layout,
                        body,
                        TablePaintOccurrenceKind::Header,
                        target_page_index,
                        occurrence.fragment_id(),
                        Some(occurrence.source_fragment_id()),
                        Some(repetition_index),
                        occurrence.row_owner(),
                        source.row_ordinal(),
                        0,
                        occurrence
                            .target_block_offset()
                            .checked_add(table_block_offset)
                            .ok_or(TableDisplayClosureError::ArithmeticOverflow)?,
                        cell,
                    )?);
                }
            }
        } else if !page.header_fragment_ids().is_empty() {
            return Err(TableDisplayClosureError::WrongRepetition);
        }
        for fragment_id in page.row_fragment_ids() {
            let row = selected
                .row_fragments()
                .iter()
                .find(|row| {
                    row.fragment_id() == *fragment_id && row.page_index() == page.page_index()
                })
                .ok_or(TableDisplayClosureError::WrongPage)?;
            for cell in row.cells() {
                records
                    .try_reserve(1)
                    .map_err(|_| TableDisplayClosureError::AllocationFailure)?;
                records.push(table_paint_record(
                    grid,
                    layout,
                    body,
                    TablePaintOccurrenceKind::Body,
                    target_page_index,
                    row.fragment_id(),
                    None,
                    None,
                    row.row_owner(),
                    row.logical_row_ordinal(),
                    row.row_fragment_ordinal(),
                    row.page_block_offset()
                        .checked_add(table_block_offset)
                        .ok_or(TableDisplayClosureError::ArithmeticOverflow)?,
                    cell,
                )?);
            }
        }
    }
    Ok(records)
}

#[allow(clippy::too_many_arguments)]
fn table_paint_record(
    grid: &ValidatedTableGridReceipt,
    layout: &TableRowBandLayoutReceipt,
    body: Rect,
    kind: TablePaintOccurrenceKind,
    page_index: u32,
    fragment_id: u64,
    source_fragment_id: Option<u64>,
    repetition_index: Option<u32>,
    row_node_id: NodeId,
    logical_row_ordinal: u32,
    row_fragment_ordinal: u32,
    block_offset: i64,
    cell: &typaxis_pagination::StagingTableCellFragmentReceipt,
) -> Result<TablePaintCellObservation, TableDisplayClosureError> {
    let measured = layout
        .cell(cell.cell_owner())
        .ok_or(TableDisplayClosureError::WrongCell)?;
    let binding = grid
        .cells()
        .iter()
        .find(|binding| binding.cell_owner() == cell.cell_owner())
        .ok_or(TableDisplayClosureError::WrongCell)?;
    if measured.flow_id() != cell.flow_id()
        || binding.flow_id() != cell.flow_id()
        || binding.column_ordinal() != measured.column_ordinal()
        || binding.colspan() != measured.colspan()
        || binding.rowspan() != measured.rowspan()
        || cell.before_cursor().flow_id() != cell.flow_id()
        || cell.after_cursor().flow_id() != cell.flow_id()
    {
        return Err(TableDisplayClosureError::WrongCell);
    }
    let x = body
        .x()
        .raw()
        .checked_add(grid.start_indent().get().raw())
        .and_then(|value| value.checked_add(measured.frame_inline_start().get().raw()))
        .ok_or(TableDisplayClosureError::ArithmeticOverflow)?;
    let y = body
        .y()
        .raw()
        .checked_add(block_offset)
        .ok_or(TableDisplayClosureError::ArithmeticOverflow)?;
    let width = measured.frame_inline_size().get().raw();
    let height = cell.selected_block_extent();
    let right = x
        .checked_add(width)
        .ok_or(TableDisplayClosureError::ArithmeticOverflow)?;
    let bottom = y
        .checked_add(height)
        .ok_or(TableDisplayClosureError::ArithmeticOverflow)?;
    let body_right = body
        .x()
        .raw()
        .checked_add(body.width().get().raw())
        .ok_or(TableDisplayClosureError::ArithmeticOverflow)?;
    let body_bottom = body
        .y()
        .raw()
        .checked_add(body.height().get().raw())
        .ok_or(TableDisplayClosureError::ArithmeticOverflow)?;
    if width <= 0
        || height < 0
        || x < body.x().raw()
        || right > body_right
        || y < body.y().raw()
        || bottom > body_bottom
    {
        return Err(TableDisplayClosureError::WrongRectangle);
    }
    Ok(TablePaintCellObservation {
        kind,
        page_index,
        fragment_id,
        source_fragment_id,
        repetition_index,
        row_node_id,
        logical_row_ordinal,
        row_fragment_ordinal,
        cell_node_id: cell.cell_owner(),
        flow_id: cell.flow_id(),
        column_ordinal: measured.column_ordinal(),
        colspan: measured.colspan().get(),
        rowspan: measured.rowspan().get(),
        rect: TablePaintRect {
            x,
            y,
            width,
            height,
        },
        content_fragment_start: cell.before_cursor().next_fragment_ordinal(),
        content_fragment_end: cell.after_cursor().next_fragment_ordinal(),
        vertical_offset_before: cell.vertical_offset_before(),
        vertical_offset_after: cell.vertical_offset_after(),
    })
}

fn encode_table_display_closure(value: &TableDisplayClosureReceipt) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, TABLE_PAINT_CLOSURE_ALGORITHM);
    output.push_str(",\"commands\":[");
    for (index, observation) in value.commands.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"cell_node_id\":");
        output.push_str(&observation.cell_node_id.get().to_string());
        output.push_str(",\"command_sha256\":");
        push_table_display_hex(
            &mut output,
            table_display_command_sha256(&observation.command),
        );
        output.push_str(",\"fragment_id\":");
        output.push_str(&observation.fragment_id.to_string());
        output.push_str(",\"page_command_index\":");
        output.push_str(&observation.page_command_index.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&observation.page_index.to_string());
        output.push_str(",\"repetition_index\":");
        match observation.repetition_index {
            Some(value) => output.push_str(&value.to_string()),
            None => output.push_str("null"),
        }
        output.push('}');
    }
    output.push(']');
    output.push_str(",\"decoration_op_count\":");
    output.push_str(&value.decoration_op_count.to_string());
    output.push_str(",\"grid_sha256\":");
    push_table_display_hex(&mut output, value.grid_sha256);
    output.push_str(",\"layout_state_sha256\":");
    push_table_display_hex(&mut output, value.layout_state_sha256);
    output.push_str(",\"page_bodies\":[");
    for (index, page) in value.page_bodies.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"body\":{\"height\":");
        output.push_str(&page.body.height().get().raw().to_string());
        output.push_str(",\"width\":");
        output.push_str(&page.body.width().get().raw().to_string());
        output.push_str(",\"x\":");
        output.push_str(&page.body.x().raw().to_string());
        output.push_str(",\"y\":");
        output.push_str(&page.body.y().raw().to_string());
        output.push_str("},\"selected_page_index\":");
        output.push_str(&page.selected_page_index.to_string());
        output.push_str(",\"table_block_offset\":");
        output.push_str(&page.table_block_offset.get().raw().to_string());
        output.push_str(",\"target_page_index\":");
        output.push_str(&page.target_page_index.to_string());
        output.push('}');
    }
    output.push_str("],\"package_sha256\":");
    push_table_display_hex(&mut output, value.package_sha256);
    output.push_str(",\"records\":[");
    for (index, record) in value.records.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"cell_node_id\":");
        output.push_str(&record.cell_node_id.get().to_string());
        output.push_str(",\"column_ordinal\":");
        output.push_str(&record.column_ordinal.to_string());
        output.push_str(",\"colspan\":");
        output.push_str(&record.colspan.to_string());
        output.push_str(",\"content_fragment_end\":");
        output.push_str(&record.content_fragment_end.to_string());
        output.push_str(",\"content_fragment_start\":");
        output.push_str(&record.content_fragment_start.to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&record.flow_id.get().to_string());
        output.push_str(",\"fragment_id\":");
        output.push_str(&record.fragment_id.to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(&mut output, record.kind.as_str());
        output.push_str(",\"logical_row_ordinal\":");
        output.push_str(&record.logical_row_ordinal.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&record.page_index.to_string());
        output.push_str(",\"rect\":{\"height\":");
        output.push_str(&record.rect.height.to_string());
        output.push_str(",\"width\":");
        output.push_str(&record.rect.width.to_string());
        output.push_str(",\"x\":");
        output.push_str(&record.rect.x.to_string());
        output.push_str(",\"y\":");
        output.push_str(&record.rect.y.to_string());
        output.push_str("},\"repetition_index\":");
        match record.repetition_index {
            Some(value) => output.push_str(&value.to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"row_fragment_ordinal\":");
        output.push_str(&record.row_fragment_ordinal.to_string());
        output.push_str(",\"row_node_id\":");
        output.push_str(&record.row_node_id.get().to_string());
        output.push_str(",\"rowspan\":");
        output.push_str(&record.rowspan.to_string());
        output.push_str(",\"source_fragment_id\":");
        match record.source_fragment_id {
            Some(value) => output.push_str(&value.to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"vertical_offset_after\":");
        output.push_str(&record.vertical_offset_after.to_string());
        output.push_str(",\"vertical_offset_before\":");
        output.push_str(&record.vertical_offset_before.to_string());
        output.push('}');
    }
    output.push_str("],\"row_band_sha256\":");
    push_table_display_hex(&mut output, value.row_band_sha256);
    output.push_str(",\"selected_layout_sha256\":");
    push_table_display_hex(&mut output, value.selected_layout_sha256);
    output.push_str(",\"table_node_id\":");
    output.push_str(&value.table_node_id.get().to_string());
    output.push('}');
    output
}

fn table_display_command_sha256(command: &DisplayCommand) -> [u8; 32] {
    let mut output = String::new();
    match command {
        DisplayCommand::DrawGlyphRun {
            run_id,
            font_instance_id,
            text_span,
            origin,
            font_size,
            bidi_level,
            fill,
            glyphs,
            clusters,
        } => {
            output.push_str("{\"bidi_level\":");
            output.push_str(&bidi_level.get().to_string());
            output.push_str(",\"clusters\":[");
            for (index, cluster) in clusters.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str("{\"extraction\":");
                match cluster.extraction {
                    ClusterExtraction::Artifact => output.push_str("{\"kind\":\"artifact\"}"),
                    ClusterExtraction::Unicode { text_span } => {
                        output.push_str("{\"end_byte\":");
                        output.push_str(&text_span.range().end_byte().get().to_string());
                        output.push_str(",\"kind\":\"unicode\",\"start_byte\":");
                        output.push_str(&text_span.range().start_byte().get().to_string());
                        output.push_str(",\"text_id\":");
                        output.push_str(&text_span.text_id().get().to_string());
                        output.push('}');
                    }
                }
                output.push_str(",\"glyph_end\":");
                output.push_str(&cluster.glyph_end.to_string());
                output.push_str(",\"glyph_start\":");
                output.push_str(&cluster.glyph_start.to_string());
                output.push_str(",\"logical_ordinal\":");
                output.push_str(&cluster.logical_ordinal.to_string());
                output.push('}');
            }
            output.push_str("],\"fill\":");
            encode_table_display_paint(&mut output, *fill);
            output.push_str(",\"font_instance_id\":");
            output.push_str(&font_instance_id.get().to_string());
            output.push_str(",\"font_size\":");
            output.push_str(&font_size.get().raw().to_string());
            output.push_str(",\"glyphs\":[");
            for (index, glyph) in glyphs.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str("{\"advance_x\":");
                output.push_str(&glyph.advance_x.raw().to_string());
                output.push_str(",\"advance_y\":");
                output.push_str(&glyph.advance_y.raw().to_string());
                output.push_str(",\"offset_x\":");
                output.push_str(&glyph.offset_x.raw().to_string());
                output.push_str(",\"offset_y\":");
                output.push_str(&glyph.offset_y.raw().to_string());
                output.push_str(",\"original_gid\":");
                output.push_str(&glyph.original_gid.get().to_string());
                output.push('}');
            }
            output.push_str("],\"operation\":\"draw_glyph_run\",\"origin\":{\"x\":");
            output.push_str(&origin.x.raw().to_string());
            output.push_str(",\"y\":");
            output.push_str(&origin.y.raw().to_string());
            output.push_str("},\"run_id\":");
            output.push_str(&run_id.get().to_string());
            output.push_str(",\"text_span\":{\"end_byte\":");
            output.push_str(&text_span.range().end_byte().get().to_string());
            output.push_str(",\"start_byte\":");
            output.push_str(&text_span.range().start_byte().get().to_string());
            output.push_str(",\"text_id\":");
            output.push_str(&text_span.text_id().get().to_string());
            output.push_str("}}");
        }
        _ => output.push_str("{\"operation\":\"forbidden\"}"),
    }
    sha256(output.as_bytes())
}

fn footnote_display_command_sha256(command: &DisplayCommand) -> [u8; 32] {
    if matches!(command, DisplayCommand::DrawGlyphRun { .. }) {
        return table_display_command_sha256(command);
    }
    let mut output = String::new();
    match command {
        DisplayCommand::StrokePath {
            path,
            paint,
            stroke,
        } => {
            output.push_str("{\"dash\":{");
            output.push_str("\"array\":[");
            for (index, value) in stroke.dash.array().iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&value.get().raw().to_string());
            }
            output.push_str("],\"phase\":");
            output.push_str(&stroke.dash.phase().get().raw().to_string());
            output.push_str("},\"line_cap\":");
            push_jcs_string(
                &mut output,
                match stroke.line_cap {
                    LineCap::Butt => "butt",
                    LineCap::Round => "round",
                    LineCap::Square => "square",
                },
            );
            output.push_str(",\"line_join\":");
            push_jcs_string(
                &mut output,
                match stroke.line_join {
                    LineJoin::Miter => "miter",
                    LineJoin::Round => "round",
                    LineJoin::Bevel => "bevel",
                },
            );
            output.push_str(",\"miter_limit\":");
            output.push_str(&stroke.miter_limit.get().raw().to_string());
            output.push_str(",\"operation\":\"stroke_path\",\"paint\":");
            encode_table_display_paint(&mut output, *paint);
            output.push_str(",\"path\":[");
            for (index, verb) in path.verbs().iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                match verb {
                    PathVerb::MoveTo(point) => {
                        output.push_str("{\"operation\":\"move_to\",\"x\":");
                        output.push_str(&point.x.raw().to_string());
                        output.push_str(",\"y\":");
                        output.push_str(&point.y.raw().to_string());
                        output.push('}');
                    }
                    PathVerb::LineTo(point) => {
                        output.push_str("{\"operation\":\"line_to\",\"x\":");
                        output.push_str(&point.x.raw().to_string());
                        output.push_str(",\"y\":");
                        output.push_str(&point.y.raw().to_string());
                        output.push('}');
                    }
                    PathVerb::CurveTo(first, second, third) => {
                        output.push_str("{\"operation\":\"curve_to\",\"x1\":");
                        output.push_str(&first.x.raw().to_string());
                        output.push_str(",\"x2\":");
                        output.push_str(&second.x.raw().to_string());
                        output.push_str(",\"x3\":");
                        output.push_str(&third.x.raw().to_string());
                        output.push_str(",\"y1\":");
                        output.push_str(&first.y.raw().to_string());
                        output.push_str(",\"y2\":");
                        output.push_str(&second.y.raw().to_string());
                        output.push_str(",\"y3\":");
                        output.push_str(&third.y.raw().to_string());
                        output.push('}');
                    }
                    PathVerb::Close => output.push_str("{\"operation\":\"close\"}"),
                }
            }
            output.push_str("],\"width\":");
            output.push_str(&stroke.width.get().raw().to_string());
            output.push('}');
        }
        _ => output.push_str("{\"operation\":\"forbidden\"}"),
    }
    sha256(output.as_bytes())
}

fn encode_footnote_display_closure(value: &FootnoteDisplayClosureReceipt) -> String {
    let mut output = String::from("{\"algorithm\":\"typaxis.footnote-paint-closure/1\"");
    output.push_str(",\"body_layout_sha256\":");
    push_table_display_hex(&mut output, value.body_layout_sha256);
    output.push_str(",\"commands\":[");
    for (index, command) in value.commands.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"assignment_ordinal\":");
        match command.assignment_ordinal {
            Some(value) => output.push_str(&value.to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"command_sha256\":");
        push_table_display_hex(
            &mut output,
            footnote_display_command_sha256(&command.command),
        );
        output.push_str(",\"flow_id\":");
        match command.flow_id {
            Some(value) => output.push_str(&value.get().to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"footnote_id\":");
        match &command.footnote_id {
            Some(value) => push_jcs_string(&mut output, value.as_str()),
            None => output.push_str("null"),
        }
        output.push_str(",\"fragment_ordinal\":");
        match command.fragment_ordinal {
            Some(value) => output.push_str(&value.to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"kind\":");
        push_jcs_string(
            &mut output,
            match command.kind {
                FootnotePaintCommandKind::ReferenceMarker => "reference_marker",
                FootnotePaintCommandKind::Separator => "separator",
                FootnotePaintCommandKind::Definition => "definition",
            },
        );
        output.push_str(",\"page_command_index\":");
        output.push_str(&command.page_command_index.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&command.page_index.to_string());
        output.push_str(",\"reference_owner\":");
        match command.reference_owner {
            Some(value) => output.push_str(&value.get().to_string()),
            None => output.push_str("null"),
        }
        output.push('}');
    }
    output.push_str("],\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"body_command_count\":");
        output.push_str(&page.body_command_count.to_string());
        output.push_str(",\"body_continuation_position\":");
        output.push_str(&page.body_continuation_position.to_string());
        output.push_str(",\"body_continuation_terminal\":");
        output.push_str(if page.body_continuation_terminal {
            "true"
        } else {
            "false"
        });
        output.push_str(",\"body_fingerprint\":");
        push_table_display_hex(&mut output, page.body_fingerprint.bytes());
        output.push_str(",\"evaluation_count\":");
        output.push_str(&page.evaluation_count.to_string());
        output.push_str(",\"flows\":[");
        for (flow_index, flow) in page.flows.iter().enumerate() {
            if flow_index > 0 {
                output.push(',');
            }
            output.push_str("{\"after_fragment\":");
            output.push_str(&flow.after_fragment.to_string());
            output.push_str(",\"assignment_ordinal\":");
            output.push_str(&flow.assignment_ordinal.to_string());
            output.push_str(",\"before_fragment\":");
            output.push_str(&flow.before_fragment.to_string());
            output.push_str(",\"carries_out\":");
            output.push_str(if flow.carries_out { "true" } else { "false" });
            output.push_str(",\"flow_id\":");
            output.push_str(&flow.flow_id.get().to_string());
            output.push_str(",\"footnote_id\":");
            push_jcs_string(&mut output, flow.footnote_id.as_str());
            output.push_str(",\"incoming_source_page\":");
            match flow.incoming_source_page {
                Some(value) => output.push_str(&value.to_string()),
                None => output.push_str("null"),
            }
            output.push('}');
        }
        output.push_str("],\"ordered_footnote_ids\":[");
        for (footnote_index, footnote_id) in page.ordered_footnote_ids.iter().enumerate() {
            if footnote_index > 0 {
                output.push(',');
            }
            push_jcs_string(&mut output, footnote_id.as_str());
        }
        output.push_str("],\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"references\":[");
        for (reference_index, reference) in page.references.iter().enumerate() {
            if reference_index > 0 {
                output.push(',');
            }
            output.push_str("{\"footnote_id\":");
            push_jcs_string(&mut output, reference.footnote_id.as_str());
            output.push_str(",\"reference_owner\":");
            output.push_str(&reference.reference_owner.get().to_string());
            output.push('}');
        }
        output.push(']');
        output.push_str(",\"reservation\":");
        output.push_str(&page.reservation.get().raw().to_string());
        output.push('}');
    }
    output.push_str("],\"profile_sha256\":");
    push_table_display_hex(&mut output, value.profile_sha256);
    output.push_str(",\"registry_sha256\":");
    push_table_display_hex(&mut output, value.registry_sha256);
    output.push_str(",\"selected_layout_sha256\":");
    push_table_display_hex(&mut output, value.selected_layout_sha256);
    output.push('}');
    output
}

fn validate_footnote_display_closure(
    display: &ValidatedDisplayDocument,
    closure: &FootnoteDisplayClosureReceipt,
) -> Result<(), FootnoteProfileDisplayError> {
    if closure.pages.len() != display.document().pages.len()
        || closure.canonical_jcs != encode_footnote_display_closure(closure)
    {
        return Err(FootnoteProfileDisplayError::SelectedLayoutMismatch);
    }
    let mut claimed = std::collections::BTreeSet::new();
    let mut previous = None;
    for observation in &closure.commands {
        let key = (observation.page_index, observation.page_command_index);
        if previous.is_some_and(|previous| previous >= key) || !claimed.insert(key) {
            return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
        }
        previous = Some(key);
        let command = display
            .document()
            .pages
            .get(observation.page_index as usize)
            .and_then(|page| page.commands.get(observation.page_command_index as usize))
            .ok_or(FootnoteProfileDisplayError::DefinitionPaintOrder)?;
        if command != &observation.command {
            return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
        }
        match observation.kind {
            FootnotePaintCommandKind::ReferenceMarker
                if observation.assignment_ordinal.is_none()
                    && observation.flow_id.is_none()
                    && observation.footnote_id.is_some()
                    && observation.fragment_ordinal.is_none()
                    && observation.reference_owner.is_some()
                    && matches!(command, DisplayCommand::DrawGlyphRun { .. }) => {}
            FootnotePaintCommandKind::Separator
                if observation.assignment_ordinal.is_none()
                    && observation.flow_id.is_none()
                    && observation.footnote_id.is_none()
                    && observation.fragment_ordinal.is_none()
                    && observation.reference_owner.is_none()
                    && matches!(command, DisplayCommand::StrokePath { .. }) => {}
            FootnotePaintCommandKind::Definition
                if observation.assignment_ordinal.is_some()
                    && observation.flow_id.is_some()
                    && observation.footnote_id.is_some()
                    && observation.fragment_ordinal.is_some()
                    && observation.reference_owner.is_none()
                    && matches!(command, DisplayCommand::DrawGlyphRun { .. }) => {}
            _ => return Err(FootnoteProfileDisplayError::DefinitionPaintOrder),
        }
    }
    for page in &display.document().pages {
        for (index, command) in page.commands.iter().enumerate() {
            if matches!(command, DisplayCommand::StrokePath { .. })
                && !claimed.contains(&(
                    page.page_index,
                    u32::try_from(index)
                        .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?,
                ))
            {
                return Err(FootnoteProfileDisplayError::SeparatorMismatch);
            }
        }
    }
    for (index, page) in closure.pages.iter().enumerate() {
        let display_page = display
            .document()
            .pages
            .get(index)
            .ok_or(FootnoteProfileDisplayError::DefinitionPaintOrder)?;
        let body_command_count = usize::try_from(page.body_command_count)
            .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;
        let flow_ids: std::collections::BTreeSet<_> =
            page.flows.iter().map(|flow| flow.flow_id).collect();
        let footnote_ids: std::collections::BTreeSet<_> =
            page.flows.iter().map(|flow| &flow.footnote_id).collect();
        if page.page_index as usize != index
            || display_page.page_index != page.page_index
            || body_command_count > display_page.commands.len()
            || (page.reservation == NonNegativeLength::ZERO) != page.flows.is_empty()
            || page.ordered_footnote_ids.len() != page.flows.len()
            || flow_ids.len() != page.flows.len()
            || footnote_ids.len() != page.flows.len()
            || page
                .flows
                .windows(2)
                .any(|pair| pair[0].assignment_ordinal >= pair[1].assignment_ordinal)
            || page
                .ordered_footnote_ids
                .iter()
                .zip(&page.flows)
                .any(|(footnote_id, flow)| footnote_id != &flow.footnote_id)
        {
            return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
        }
        let page_commands: Vec<_> = closure
            .commands
            .iter()
            .filter(|command| command.page_index == page.page_index)
            .collect();
        let mut observed_references = std::collections::BTreeMap::new();
        for command in page_commands
            .iter()
            .filter(|command| command.kind == FootnotePaintCommandKind::ReferenceMarker)
        {
            let owner = command
                .reference_owner
                .ok_or(FootnoteProfileDisplayError::DefinitionPaintOrder)?;
            let footnote_id = command
                .footnote_id
                .as_ref()
                .ok_or(FootnoteProfileDisplayError::DefinitionPaintOrder)?;
            if command.page_command_index >= page.body_command_count
                || observed_references.insert(owner, footnote_id).is_some()
            {
                return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
            }
        }
        let expected_references: std::collections::BTreeMap<_, _> = page
            .references
            .iter()
            .map(|reference| (reference.reference_owner, &reference.footnote_id))
            .collect();
        if observed_references != expected_references
            || expected_references.len() != page.references.len()
        {
            return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
        }
        let separators: Vec<_> = page_commands
            .iter()
            .filter(|command| command.kind == FootnotePaintCommandKind::Separator)
            .collect();
        if separators.len() != usize::from(!page.flows.is_empty()) {
            return Err(FootnoteProfileDisplayError::SeparatorMismatch);
        }
        if page.flows.is_empty() {
            if body_command_count != display_page.commands.len()
                || page_commands
                    .iter()
                    .any(|command| command.kind == FootnotePaintCommandKind::Definition)
            {
                return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
            }
            continue;
        }
        if separators[0].page_command_index != page.body_command_count {
            return Err(FootnoteProfileDisplayError::SeparatorMismatch);
        }
        let definition_commands: Vec<_> = page_commands
            .iter()
            .filter(|command| command.kind == FootnotePaintCommandKind::Definition)
            .collect();
        if definition_commands
            .iter()
            .any(|command| command.page_command_index <= page.body_command_count)
        {
            return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
        }
        let mut previous_definition = None;
        for observation in definition_commands {
            let flow_index = page
                .flows
                .iter()
                .position(|flow| {
                    observation.assignment_ordinal == Some(flow.assignment_ordinal)
                        && observation.flow_id == Some(flow.flow_id)
                        && observation.footnote_id.as_ref() == Some(&flow.footnote_id)
                        && observation.fragment_ordinal.is_some_and(|fragment| {
                            fragment >= flow.before_fragment && fragment < flow.after_fragment
                        })
                })
                .ok_or(FootnoteProfileDisplayError::DefinitionPaintOrder)?;
            let fragment_ordinal = observation
                .fragment_ordinal
                .ok_or(FootnoteProfileDisplayError::DefinitionPaintOrder)?;
            let key = (flow_index, fragment_ordinal);
            if previous_definition.is_some_and(|previous| previous > key) {
                return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
            }
            previous_definition = Some(key);
        }
        for command_index in body_command_count + 1..display_page.commands.len() {
            let command_index = u32::try_from(command_index)
                .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;
            let Some(observation) = page_commands.iter().find(|observation| {
                observation.page_command_index == command_index
                    && observation.kind == FootnotePaintCommandKind::Definition
            }) else {
                return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
            };
            if page.flows.iter().all(|flow| {
                observation.assignment_ordinal != Some(flow.assignment_ordinal)
                    || observation.flow_id != Some(flow.flow_id)
                    || observation.footnote_id.as_ref() != Some(&flow.footnote_id)
                    || observation.fragment_ordinal.map_or(true, |fragment| {
                        fragment < flow.before_fragment || fragment >= flow.after_fragment
                    })
            }) {
                return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
            }
        }
    }
    Ok(())
}

fn encode_table_display_paint(output: &mut String, paint: Paint) {
    match paint {
        Paint::Gray(value) => {
            output.push_str("{\"kind\":\"gray\",\"value\":");
            output.push_str(&value.to_string());
        }
        Paint::Rgb { r, g, b } => {
            output.push_str("{\"b\":");
            output.push_str(&b.to_string());
            output.push_str(",\"g\":");
            output.push_str(&g.to_string());
            output.push_str(",\"kind\":\"rgb\",\"r\":");
            output.push_str(&r.to_string());
        }
        Paint::Cmyk { c, m, y, k } => {
            output.push_str("{\"c\":");
            output.push_str(&c.to_string());
            output.push_str(",\"k\":");
            output.push_str(&k.to_string());
            output.push_str(",\"kind\":\"cmyk\",\"m\":");
            output.push_str(&m.to_string());
            output.push_str(",\"y\":");
            output.push_str(&y.to_string());
        }
    }
    output.push('}');
}

fn push_table_display_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Paint {
    Gray(u16),
    Rgb { r: u16, g: u16, b: u16 },
    Cmyk { c: u16, m: u16, y: u16, k: u16 },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathVerb {
    MoveTo(Point),
    LineTo(Point),
    CurveTo(Point, Point, Point),
    Close,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathError {
    Empty,
    MustStartWithMove,
    SegmentBeforeMove,
    CloseWithoutOpenSubpath,
    EmptySubpath,
    NoDrawableSegment,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Path {
    verbs: Vec<PathVerb>,
}
impl Path {
    pub fn new(verbs: Vec<PathVerb>) -> Result<Self, PathError> {
        if verbs.is_empty() {
            return Err(PathError::Empty);
        }
        if !matches!(verbs.first(), Some(PathVerb::MoveTo(_))) {
            return Err(PathError::MustStartWithMove);
        }
        let mut open = false;
        let mut drawable = false;
        let mut any_drawable = false;
        for verb in &verbs {
            match verb {
                PathVerb::MoveTo(_) => {
                    open = true;
                    drawable = false;
                }
                PathVerb::LineTo(_) | PathVerb::CurveTo(_, _, _) if !open => {
                    return Err(PathError::SegmentBeforeMove)
                }
                PathVerb::LineTo(_) | PathVerb::CurveTo(_, _, _) => {
                    drawable = true;
                    any_drawable = true;
                }
                PathVerb::Close if !open => return Err(PathError::CloseWithoutOpenSubpath),
                PathVerb::Close if !drawable => return Err(PathError::EmptySubpath),
                PathVerb::Close => {
                    open = false;
                    drawable = false;
                }
            }
        }
        if !any_drawable {
            return Err(PathError::NoDrawableSegment);
        }
        Ok(Self { verbs })
    }
    pub fn verbs(&self) -> &[PathVerb] {
        &self.verbs
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DashPattern {
    array: Vec<NonNegativeLength>,
    phase: NonNegativeLength,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DashError {
    AllZero,
}
impl DashPattern {
    pub fn new(array: Vec<NonNegativeLength>, phase: NonNegativeLength) -> Result<Self, DashError> {
        if !array.is_empty() && array.iter().all(|value| value.get() == Length::ZERO) {
            return Err(DashError::AllZero);
        }
        Ok(Self { array, phase })
    }
    pub fn array(&self) -> &[NonNegativeLength] {
        &self.array
    }
    pub const fn phase(&self) -> NonNegativeLength {
        self.phase
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrokeStyle {
    pub width: PositiveLength,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub miter_limit: PositiveUnitless16_16,
    pub dash: DashPattern,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayGlyph {
    pub original_gid: OriginalGlyphId,
    pub advance_x: Length,
    pub advance_y: Length,
    pub offset_x: Length,
    pub offset_y: Length,
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ClusterExtraction {
    Unicode { text_span: DisplayTextSpan },
    Artifact,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayCluster {
    pub logical_ordinal: u32,
    pub glyph_start: u32,
    pub glyph_end: u32,
    pub extraction: ClusterExtraction,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayCommand {
    Save,
    Restore,
    ConcatTransform {
        matrix: AffineTransform,
    },
    ClipPath {
        path: Path,
        rule: FillRule,
    },
    FillPath {
        path: Path,
        paint: Paint,
        rule: FillRule,
    },
    StrokePath {
        path: Path,
        paint: Paint,
        stroke: StrokeStyle,
    },
    DrawGlyphRun {
        run_id: DisplayGlyphRunId,
        font_instance_id: FontInstanceId,
        text_span: DisplayTextSpan,
        origin: Point,
        font_size: PositiveLength,
        bidi_level: BidiLevel,
        fill: Paint,
        glyphs: Vec<DisplayGlyph>,
        clusters: Vec<DisplayCluster>,
    },
    DrawImage {
        image_id: ImageResourceId,
        rect: Rect,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkTarget {
    Internal(AnchorId),
    Uri(SafeUri),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkAnnotation {
    pub target: LinkTarget,
    pub rect: Rect,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DestinationView {
    Xyz { point: Point },
    FitPage,
    FitWidth { top: Option<Length> },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedDestination {
    pub anchor_id: AnchorId,
    pub page_index: u32,
    pub view: DestinationView,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayPage {
    pub page_index: u32,
    pub width: PositiveLength,
    pub height: PositiveLength,
    pub commands: Vec<DisplayCommand>,
    pub annotations: Vec<LinkAnnotation>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayTextBuffer {
    pub text_id: DisplayTextBufferId,
    pub origin: DisplayTextOrigin,
    pub utf8: String,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DisplayTextOrigin {
    Parsed(TextBufferId),
    Generated(GeneratedBufferKey),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayFontInstance {
    pub font_instance_id: FontInstanceId,
    pub font_face_id: FontFaceId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplaySourceLayout {
    layout_epoch: LayoutEpoch,
    state_fingerprint: LayoutStateFingerprint,
}
impl DisplaySourceLayout {
    fn from_selected_pagination(result: &PaginationResult) -> Self {
        let record = result.selected_pass().fingerprint_record();
        Self {
            layout_epoch: record.layout_epoch(),
            state_fingerprint: result.final_fingerprint(),
        }
    }
    pub const fn layout_epoch(self) -> LayoutEpoch {
        self.layout_epoch
    }
    pub const fn state_fingerprint(self) -> LayoutStateFingerprint {
        self.state_fingerprint
    }
    pub fn matches_selected(self, result: &PaginationResult) -> bool {
        self == Self::from_selected_pagination(result)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayDocument {
    source_layout: DisplaySourceLayout,
    pub text_buffers: Vec<DisplayTextBuffer>,
    pub font_instances: Vec<DisplayFontInstance>,
    pub destinations: Vec<NamedDestination>,
    pub pages: Vec<DisplayPage>,
}
impl DisplayDocument {
    /// Creates an untrusted wire-model payload for structural validation. The
    /// selected-state stamp is not a paint provenance receipt and this type
    /// cannot be consumed by resource finalization or PDF publication.
    pub fn from_untrusted_parts_for_selected_pagination(
        selected: &PaginationResult,
        text_buffers: Vec<DisplayTextBuffer>,
        font_instances: Vec<DisplayFontInstance>,
        destinations: Vec<NamedDestination>,
        pages: Vec<DisplayPage>,
    ) -> Self {
        Self {
            source_layout: DisplaySourceLayout::from_selected_pagination(selected),
            text_buffers,
            font_instances,
            destinations,
            pages,
        }
    }

    pub const fn source_layout(&self) -> DisplaySourceLayout {
        self.source_layout
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayValidationError {
    EmptyDocument,
    NonDenseTextBufferId,
    NonCanonicalTextBufferOrigin,
    NonDensePageIndex,
    NonDenseFontInstanceId,
    NonCanonicalFontFaceOrder,
    NonCanonicalDestinationOrder,
    DuplicateDestination,
    UnknownDestinationPage,
    DestinationOutOfBounds,
    UnknownInternalTarget,
    UnknownTextBuffer,
    TextSpanOutOfBounds,
    UnbalancedGraphicsState,
    AnnotationOutOfBounds,
    ClusterCoverage,
    EmptyCluster,
    EmptyUnicodeCluster,
    NonDenseLogicalCluster,
    InvalidClusterGlyphRange,
    UnknownFontInstance,
    UnusedFontInstance,
    UnusedTextBuffer,
    NonDenseGlyphRunId,
    GraphicsStateDepthOverflow,
    NumericOutOfRange,
    UriPolicy,
    SelectedLayoutMismatch,
    SelectedPageClosure,
    SelectedGeneratedTextMismatch,
    SelectedParsedTextMismatch,
    PackageLayoutMismatch,
    SelectedTextMapMismatch,
    NonBlankSelectedLayout,
    UnsupportedReferencePaintDomain,
    InvalidDashPattern,
    SelectedDestinationMismatch,
}

/// A Display artifact whose structure and generated text have been checked
/// against the exact materialized pagination state selected for publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedDisplayPageGeometry {
    page_index: u32,
    master_id: MasterId,
    width: PositiveLength,
    height: PositiveLength,
}
impl ValidatedDisplayPageGeometry {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn width(&self) -> PositiveLength {
        self.width
    }
    pub const fn height(&self) -> PositiveLength {
        self.height
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructurallyValidatedDisplayDocument {
    document: DisplayDocument,
    selected_page_geometry: Vec<ValidatedDisplayPageGeometry>,
}
impl StructurallyValidatedDisplayDocument {
    pub fn new(
        document: DisplayDocument,
        package: &ValidatedParsedPackage,
        selected: &PaginationResult,
        config: &EffectiveConfig,
    ) -> Result<Self, DisplayValidationError> {
        let epoch = selected.selected_pass().fingerprint_record().layout_epoch();
        if epoch.document() != package.epoch_identity().document()
            || epoch.style() != package.epoch_identity().style()
        {
            return Err(DisplayValidationError::PackageLayoutMismatch);
        }
        Self::validate(
            document,
            selected,
            config,
            Some(&package.package().text_store),
            None,
            None,
        )
    }

    fn from_verified_text_map(
        document: DisplayDocument,
        selected: &PaginationResult,
        config: &EffectiveConfig,
    ) -> Result<Self, DisplayValidationError> {
        Self::validate(document, selected, config, None, None, None)
    }

    fn from_verified_footnote_profile(
        document: DisplayDocument,
        selected: &PaginationResult,
        config: &EffectiveConfig,
        expected_destinations: &[NamedDestination],
        selected_page_geometry: Vec<ValidatedDisplayPageGeometry>,
    ) -> Result<Self, DisplayValidationError> {
        Self::validate(
            document,
            selected,
            config,
            None,
            Some(selected_page_geometry),
            Some(expected_destinations),
        )
    }

    fn from_verified_table_profile(
        document: DisplayDocument,
        selected: &PaginationResult,
        config: &EffectiveConfig,
        selected_page_geometry: Vec<ValidatedDisplayPageGeometry>,
    ) -> Result<Self, DisplayValidationError> {
        Self::validate(
            document,
            selected,
            config,
            None,
            Some(selected_page_geometry),
            None,
        )
    }

    fn validate(
        document: DisplayDocument,
        selected: &PaginationResult,
        config: &EffectiveConfig,
        parsed_store: Option<&TextStore>,
        page_geometry_override: Option<Vec<ValidatedDisplayPageGeometry>>,
        destination_override: Option<&[NamedDestination]>,
    ) -> Result<Self, DisplayValidationError> {
        if !document.source_layout.matches_selected(selected) {
            return Err(DisplayValidationError::SelectedLayoutMismatch);
        }
        if document.pages.iter().enumerate().any(|(index, page)| {
            u32::try_from(index)
                .map(|expected| page.page_index != expected)
                .unwrap_or(true)
        }) {
            return Err(DisplayValidationError::NonDensePageIndex);
        }
        let selected_pages = selected.selected_pages();
        let selected_geometry = selected.selected_page_geometry();
        let selected_page_geometry = if let Some(override_geometry) = page_geometry_override {
            if override_geometry.len() != document.pages.len()
                || override_geometry.len() < selected_geometry.len()
                || override_geometry
                    .iter()
                    .zip(&document.pages)
                    .any(|(geometry, display_page)| {
                        geometry.page_index() != display_page.page_index
                            || geometry.width() != display_page.width
                            || geometry.height() != display_page.height
                    })
                || override_geometry
                    .iter()
                    .zip(selected_geometry)
                    .any(|(actual, expected)| {
                        actual.page_index() != expected.page_index()
                            || actual.master_id() != expected.master_id()
                            || actual.width() != expected.width()
                            || actual.height() != expected.height()
                    })
            {
                return Err(DisplayValidationError::SelectedPageClosure);
            }
            override_geometry
        } else {
            if document.pages.len() != selected_pages.len()
                || selected_pages.len() != selected_geometry.len()
                || document
                    .pages
                    .iter()
                    .zip(selected_pages)
                    .zip(selected_geometry)
                    .any(|((display_page, page), geometry)| {
                        display_page.page_index != page.page_index
                            || display_page.page_index != geometry.page_index()
                            || display_page.width != geometry.width()
                            || display_page.height != geometry.height()
                    })
            {
                return Err(DisplayValidationError::SelectedPageClosure);
            }
            selected_geometry
                .iter()
                .map(|geometry| ValidatedDisplayPageGeometry {
                    page_index: geometry.page_index(),
                    master_id: geometry.master_id().clone(),
                    width: geometry.width(),
                    height: geometry.height(),
                })
                .collect()
        };
        let selected_generated = selected.selected_pass().generated_text();
        validate_selected_text_buffers(&document.text_buffers, selected_generated, parsed_store)?;
        if document.pages.is_empty() {
            return Err(DisplayValidationError::EmptyDocument);
        }
        let mut previous_origin = None;
        for (index, buffer) in document.text_buffers.iter().enumerate() {
            if buffer.text_id.get()
                != u32::try_from(index).map_err(|_| DisplayValidationError::NonDenseTextBufferId)?
            {
                return Err(DisplayValidationError::NonDenseTextBufferId);
            }
            if previous_origin.is_some_and(|previous| previous >= buffer.origin) {
                return Err(DisplayValidationError::NonCanonicalTextBufferOrigin);
            }
            previous_origin = Some(buffer.origin);
        }
        let mut known_font_instances = std::collections::BTreeSet::new();
        let mut previous_font_face = None;
        for (index, instance) in document.font_instances.iter().enumerate() {
            if instance.font_instance_id.get()
                != u32::try_from(index)
                    .map_err(|_| DisplayValidationError::NonDenseFontInstanceId)?
                || !known_font_instances.insert(instance.font_instance_id)
            {
                return Err(DisplayValidationError::NonDenseFontInstanceId);
            }
            if previous_font_face.is_some_and(|previous| previous >= instance.font_face_id) {
                return Err(DisplayValidationError::NonCanonicalFontFaceOrder);
            }
            previous_font_face = Some(instance.font_face_id);
        }
        let mut destinations = std::collections::BTreeSet::new();
        let mut previous_destination: Option<&AnchorId> = None;
        for destination in &document.destinations {
            if !destinations.insert(destination.anchor_id.clone()) {
                return Err(DisplayValidationError::DuplicateDestination);
            }
            if previous_destination.is_some_and(|previous| previous >= &destination.anchor_id) {
                return Err(DisplayValidationError::NonCanonicalDestinationOrder);
            }
            previous_destination = Some(&destination.anchor_id);
            if destination.page_index as usize >= document.pages.len() {
                return Err(DisplayValidationError::UnknownDestinationPage);
            }
            let page = &document.pages[destination.page_index as usize];
            validate_destination_numbers(&destination.view)?;
            if !destination_within_page(&destination.view, page) {
                return Err(DisplayValidationError::DestinationOutOfBounds);
            }
        }
        let expected_destinations = match destination_override {
            Some(destinations) => destinations.to_vec(),
            None => destinations_from_selected_pagination(selected)?,
        };
        if document.destinations != expected_destinations {
            return Err(DisplayValidationError::SelectedDestinationMismatch);
        }
        let mut used_font_instances = std::collections::BTreeSet::new();
        let mut used_text_buffers = std::collections::BTreeSet::new();
        let mut next_run_id = 0u32;
        for (index, page) in document.pages.iter().enumerate() {
            if page.page_index
                != u32::try_from(index).map_err(|_| DisplayValidationError::NonDensePageIndex)?
            {
                return Err(DisplayValidationError::NonDensePageIndex);
            }
            validate_length(page.width.get())?;
            validate_length(page.height.get())?;
            let mut depth = 0u32;
            for command in &page.commands {
                validate_command_numbers(command)?;
                match command {
                    DisplayCommand::Save => {
                        depth = depth
                            .checked_add(1)
                            .ok_or(DisplayValidationError::GraphicsStateDepthOverflow)?;
                    }
                    DisplayCommand::Restore if depth == 0 => {
                        return Err(DisplayValidationError::UnbalancedGraphicsState)
                    }
                    DisplayCommand::Restore => depth -= 1,
                    DisplayCommand::DrawGlyphRun {
                        run_id,
                        font_instance_id,
                        text_span,
                        glyphs,
                        clusters,
                        ..
                    } => {
                        if run_id.get() != next_run_id {
                            return Err(DisplayValidationError::NonDenseGlyphRunId);
                        }
                        next_run_id = next_run_id
                            .checked_add(1)
                            .ok_or(DisplayValidationError::NonDenseGlyphRunId)?;
                        if !known_font_instances.contains(font_instance_id) {
                            return Err(DisplayValidationError::UnknownFontInstance);
                        }
                        used_font_instances.insert(*font_instance_id);
                        validate_glyph_run_clusters(
                            &document.text_buffers,
                            *text_span,
                            glyphs.len(),
                            clusters,
                            &mut used_text_buffers,
                        )?;
                    }
                    _ => {}
                }
            }
            if depth != 0 {
                return Err(DisplayValidationError::UnbalancedGraphicsState);
            }
            for annotation in &page.annotations {
                if let LinkTarget::Internal(anchor) = &annotation.target {
                    if !destinations.contains(anchor) {
                        return Err(DisplayValidationError::UnknownInternalTarget);
                    }
                }
                if let LinkTarget::Uri(uri) = &annotation.target {
                    let schemes: Vec<&str> = config
                        .allowed_uri_schemes()
                        .iter()
                        .map(String::as_str)
                        .collect();
                    uri.validate_policy(&schemes, config.limits().get().max_uri_bytes as usize)
                        .map_err(|_| DisplayValidationError::UriPolicy)?;
                }
                validate_rect_numbers(annotation.rect)?;
                if !rect_within_page(annotation.rect, page) {
                    return Err(DisplayValidationError::AnnotationOutOfBounds);
                }
            }
        }
        if used_font_instances != known_font_instances {
            return Err(DisplayValidationError::UnusedFontInstance);
        }
        let all_text_buffers: std::collections::BTreeSet<_> = document
            .text_buffers
            .iter()
            .map(|buffer| buffer.text_id)
            .collect();
        if used_text_buffers != all_text_buffers {
            return Err(DisplayValidationError::UnusedTextBuffer);
        }
        Ok(Self {
            document,
            selected_page_geometry,
        })
    }
    pub const fn document(&self) -> &DisplayDocument {
        &self.document
    }
    pub fn selected_page_geometry(&self) -> &[ValidatedDisplayPageGeometry] {
        &self.selected_page_geometry
    }
    pub fn into_document(self) -> DisplayDocument {
        self.document
    }
    pub fn into_parts(self) -> (DisplayDocument, Vec<ValidatedDisplayPageGeometry>) {
        (self.document, self.selected_page_geometry)
    }
}

/// Publication-trusted Display artifact. Only the crate-owned paint builder
/// owner can issue this type; structural validation alone cannot be upgraded
/// into it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedDisplayDocument {
    structural: StructurallyValidatedDisplayDocument,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FootnoteTextAlign {
    Start,
    End,
    Center,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FootnoteDefinitionPaintLine {
    owner: NodeId,
    line_ordinal: u32,
    start_item: u32,
    end_item: u32,
    extent: PositiveLength,
    line_height: PositiveLength,
    space_before: NonNegativeLength,
    physical_left_inset: NonNegativeLength,
    inline_size: PositiveLength,
    text_align: FootnoteTextAlign,
    marker_owner: Option<NodeId>,
    marker_gap: Option<PositiveLength>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FootnoteDefinitionPaintFragment {
    extent: PositiveLength,
    lines: Vec<FootnoteDefinitionPaintLine>,
}

fn footnote_definition_block_anchors(
    package: &ValidatedParsedPackage,
    block_owner: NodeId,
) -> Result<Vec<AnchorId>, FootnoteProfileDisplayError> {
    let nodes = package.document_nodes();
    if !matches!(
        nodes.node_kind(block_owner),
        Some(typaxis_document::DocumentNodeKind::Paragraph)
            | Some(typaxis_document::DocumentNodeKind::Heading)
    ) {
        return Err(FootnoteProfileDisplayError::SelectedLayoutMismatch);
    }
    let block_path = nodes
        .node_path(block_owner)
        .ok_or(FootnoteProfileDisplayError::SelectedLayoutMismatch)?;
    Ok(nodes
        .anchors()
        .filter(|(_, anchor_owner)| {
            nodes
                .node_path(*anchor_owner)
                .is_some_and(|path| path.starts_with(block_path))
        })
        .map(|(anchor_id, _)| anchor_id.clone())
        .collect())
}

fn footnote_definition_paint_lines(
    package: &ValidatedParsedPackage,
    registry: &StagingFootnoteFlowRegistry,
    paragraph_items: &ValidatedParagraphItemRegistry,
) -> Result<Vec<Vec<FootnoteDefinitionPaintFragment>>, FootnoteProfileDisplayError> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(registry.flows().len())
        .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;
    for flow in registry.flows() {
        let mut lines = Vec::new();
        let mut marker_count = 0u32;
        for owner in flow.block_owners() {
            let computed = package
                .cascade_style(*owner)
                .map_err(|_| FootnoteProfileDisplayError::DefinitionFragmentMismatch)?;
            let line_height = match computed.computed().properties().get("line_height") {
                Some(StyleValue::Length(value)) => PositiveLength::new(*value),
                _ => None,
            }
            .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch)?;
            let font_size = match computed.computed().properties().get("font_size") {
                Some(StyleValue::Length(value)) => PositiveLength::new(*value),
                _ => None,
            }
            .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch)?;
            let space_before = display_nonnegative_style_length(
                computed.computed().properties().get("space_before"),
            )?;
            let space_after = display_nonnegative_style_length(
                computed.computed().properties().get("space_after"),
            )?;
            let start_indent = display_nonnegative_style_length(
                computed.computed().properties().get("start_indent"),
            )?;
            let end_indent = display_nonnegative_style_length(
                computed.computed().properties().get("end_indent"),
            )?;
            let inline_size = registry
                .maximum_footnote_frame()
                .width()
                .get()
                .checked_sub(start_indent.get())
                .and_then(|value| value.checked_sub(end_indent.get()))
                .and_then(PositiveLength::new)
                .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch)?;
            let paragraph_level = paragraph_items
                .paragraph_level(*owner)
                .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch)?;
            let physical_left_inset = if paragraph_level.get() % 2 == 1 {
                end_indent
            } else {
                start_indent
            };
            let text_align = match computed.computed().properties().get("text_align") {
                Some(StyleValue::Keyword(value)) if value == "start" => FootnoteTextAlign::Start,
                Some(StyleValue::Keyword(value)) if value == "end" => FootnoteTextAlign::End,
                Some(StyleValue::Keyword(value)) if value == "center" => FootnoteTextAlign::Center,
                _ => return Err(FootnoteProfileDisplayError::DefinitionFragmentMismatch),
            };
            let ranges: Vec<(u32, u32)> =
                if let Some(result) = paragraph_items.paragraph_break(*owner) {
                    let mut previous = 0u32;
                    let mut ranges = Vec::new();
                    ranges
                        .try_reserve_exact(result.lines.len())
                        .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;
                    for line in &result.lines {
                        if line.item_index <= previous {
                            return Err(FootnoteProfileDisplayError::DefinitionFragmentMismatch);
                        }
                        ranges.push((previous, line.item_index));
                        previous = line.item_index;
                    }
                    if previous
                        != paragraph_items
                            .item_count(*owner)
                            .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch)?
                    {
                        return Err(FootnoteProfileDisplayError::DefinitionFragmentMismatch);
                    }
                    ranges
                } else if paragraph_items.item_count(*owner) == Some(1) {
                    vec![(0, 1)]
                } else {
                    return Err(FootnoteProfileDisplayError::DefinitionFragmentMismatch);
                };
            for (line_index, (start_item, end_item)) in ranges.iter().copied().enumerate() {
                let mut expected = line_height.get();
                if line_index == 0 {
                    expected = expected
                        .checked_add(space_before.get())
                        .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
                }
                if line_index + 1 == ranges.len() {
                    expected = expected
                        .checked_add(space_after.get())
                        .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
                }
                let extent = PositiveLength::new(expected)
                    .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch)?;
                let shaped = paragraph_shaped_slices(paragraph_items, *owner, start_item, end_item)
                    .map_err(FootnoteProfileDisplayError::Display)?;
                let contains_marker = shaped.iter().any(|slice| {
                    matches!(
                        slice.shaped.source(),
                        ShapeSourceSpan::Generated(provenance)
                            if provenance.buffer_key().owner() == flow.binding().definition_owner()
                                && provenance.buffer_key().generation_kind()
                                    == GenerationKind::FootnoteMarker
                    )
                });
                if contains_marker {
                    marker_count = marker_count
                        .checked_add(1)
                        .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
                }
                let line_ordinal = u32::try_from(lines.len())
                    .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;
                lines.push(FootnoteDefinitionPaintLine {
                    owner: *owner,
                    line_ordinal,
                    start_item,
                    end_item,
                    extent,
                    line_height,
                    space_before: if line_index == 0 {
                        space_before
                    } else {
                        NonNegativeLength::ZERO
                    },
                    physical_left_inset,
                    inline_size,
                    text_align,
                    marker_owner: contains_marker.then_some(flow.binding().definition_owner()),
                    marker_gap: contains_marker.then_some(font_size),
                });
            }
        }
        if marker_count != 1 || flow.fragment_extents().len() != flow.fragment_line_counts().len() {
            return Err(FootnoteProfileDisplayError::DefinitionFragmentMismatch);
        }
        let mut fragments = Vec::new();
        fragments
            .try_reserve_exact(flow.fragment_extents().len())
            .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;
        let mut line_cursor = 0usize;
        for (extent, line_count) in flow
            .fragment_extents()
            .iter()
            .copied()
            .zip(flow.fragment_line_counts().iter())
        {
            let line_count = usize::try_from(line_count.get())
                .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;
            let line_end = line_cursor
                .checked_add(line_count)
                .filter(|end| *end <= lines.len())
                .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch)?;
            let measured = lines[line_cursor..line_end]
                .iter()
                .try_fold(Length::ZERO, |total, line| {
                    total.checked_add(line.extent.get())
                })
                .and_then(PositiveLength::new)
                .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
            if measured != extent {
                return Err(FootnoteProfileDisplayError::DefinitionFragmentMismatch);
            }
            fragments.push(FootnoteDefinitionPaintFragment {
                extent,
                lines: lines[line_cursor..line_end].to_vec(),
            });
            line_cursor = line_end;
        }
        if line_cursor != lines.len() {
            return Err(FootnoteProfileDisplayError::DefinitionFragmentMismatch);
        }
        if fragments.first().map_or(true, |fragment| {
            fragment
                .lines
                .iter()
                .all(|line| line.marker_owner.is_none())
        }) || fragments.iter().skip(1).any(|fragment| {
            fragment
                .lines
                .iter()
                .any(|line| line.marker_owner.is_some())
        }) {
            return Err(FootnoteProfileDisplayError::DefinitionFragmentMismatch);
        }
        output.push(fragments);
    }
    Ok(output)
}

fn display_nonnegative_style_length(
    value: Option<&StyleValue>,
) -> Result<NonNegativeLength, FootnoteProfileDisplayError> {
    match value {
        Some(StyleValue::Length(value)) => NonNegativeLength::new(*value)
            .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch),
        None => Ok(NonNegativeLength::ZERO),
        Some(_) => Err(FootnoteProfileDisplayError::DefinitionFragmentMismatch),
    }
}

impl ValidatedDisplayDocument {
    /// Paints selected paragraph fragments from the exact paragraph-item
    /// registry retained by the canonical flow. Each drawing command covers
    /// one validated shaping cluster, so source extraction and bidi level
    /// remain inseparable from the glyph slice which produced them.
    pub fn paint_reference_paragraphs(
        package: &ValidatedParsedPackage,
        selected: &PaginationResult,
        flow: &FlowTree,
        config: &EffectiveConfig,
    ) -> Result<Self, DisplayValidationError> {
        let selected_epoch = selected.selected_pass().fingerprint_record().layout_epoch();
        if flow.epoch() != selected_epoch
            || selected_epoch.document() != package.epoch_identity().document()
            || selected_epoch.style() != package.epoch_identity().style()
            || !package.package().document.footnotes.is_empty()
        {
            return Err(DisplayValidationError::UnsupportedReferencePaintDomain);
        }
        let registry = flow
            .paragraph_items()
            .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
        if registry.epoch() != selected_epoch {
            return Err(DisplayValidationError::UnsupportedReferencePaintDomain);
        }
        let mut parsed_spans = Vec::new();
        let mut generated_spans = Vec::new();
        for page in selected.selected_pages() {
            for fragment in &page.fragments {
                if registry.item_count(fragment.owner).is_none() {
                    continue;
                }
                for slice in fragment_shaped_slices(registry, fragment)? {
                    match slice.shaped.source() {
                        ShapeSourceSpan::Parsed(span) => parsed_spans.push(span),
                        ShapeSourceSpan::Generated(provenance) => {
                            generated_spans.push(provenance.text_span());
                        }
                    }
                }
            }
        }
        let text_map =
            DisplayTextMap::from_selected_spans(package, selected, &parsed_spans, &generated_spans)
                .map_err(|_| DisplayValidationError::SelectedTextMapMismatch)?;

        let mut used_fonts = std::collections::BTreeMap::new();
        let mut pages = Vec::new();
        let mut next_run_id = 0u32;
        for (page_plan, geometry) in selected
            .selected_pages()
            .iter()
            .zip(selected.selected_page_geometry())
        {
            let mut commands = Vec::new();
            for fragment in &page_plan.fragments {
                if registry.item_count(fragment.owner).is_none() {
                    if package.document_nodes().node_kind(fragment.owner)
                        == Some(typaxis_document::DocumentNodeKind::Figure)
                    {
                        let image_id = basic_figure_image_id(
                            &package.package().document.blocks,
                            fragment.owner,
                        )
                        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
                        commands.push(DisplayCommand::DrawImage {
                            image_id,
                            rect: fragment.bounds,
                        });
                    }
                    continue;
                }
                let logical = fragment_shaped_slices(registry, fragment)?;
                if logical.is_empty() {
                    continue;
                }
                let levels: Vec<_> = logical
                    .iter()
                    .map(|slice| slice.shaped.bidi_level())
                    .collect();
                let classes: Vec<_> = logical.iter().map(|slice| slice.class).collect();
                let paragraph_level = registry
                    .paragraph_level(fragment.owner)
                    .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
                // UAX #9 L1 must precede the final line reshape. The reshape
                // rebinds every selected cluster to the exact validated run;
                // justification then adjusts logical advances, and only then
                // may L2 derive visual order.
                let after_l1 = reset_line_bidi_levels(paragraph_level, &levels, &classes)
                    .map_err(|_| DisplayValidationError::UnsupportedReferencePaintDomain)?;
                let mut logical =
                    reference_final_line_reshape(registry, fragment.owner, logical, &after_l1)?;
                justify_reference_line(registry, fragment, &mut logical)?;
                let order = reorder_line_l2(&after_l1)
                    .map_err(|_| DisplayValidationError::UnsupportedReferencePaintDomain)?;
                let mut x = fragment.bounds.x();
                for logical_index in order.visual_to_logical() {
                    let line_slice = logical
                        .get(*logical_index as usize)
                        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
                    let shaped = line_slice.shaped;
                    let runs = registry
                        .runs(fragment.owner)
                        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
                    let run = runs
                        .get(shaped.paragraph_run_index().get() as usize)
                        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
                    let command = paint_shaped_slice(
                        package,
                        &text_map,
                        run,
                        shaped,
                        after_l1.logical_levels()[*logical_index as usize],
                        DisplayGlyphRunId::new(next_run_id),
                        Point {
                            x,
                            y: fragment.bounds.y(),
                        },
                    )?;
                    next_run_id = next_run_id
                        .checked_add(1)
                        .ok_or(DisplayValidationError::NonDenseGlyphRunId)?;
                    let instance = command_font(&command)?;
                    let face = run.font_face_id();
                    if package
                        .package()
                        .resources
                        .font_faces
                        .iter()
                        .all(|declaration| declaration.font_face_id != face)
                        || used_fonts
                            .insert(instance, face)
                            .is_some_and(|previous| previous != face)
                    {
                        return Err(DisplayValidationError::UnknownFontInstance);
                    }
                    x = x
                        .checked_add(line_slice.advance)
                        .ok_or(DisplayValidationError::NumericOutOfRange)?;
                    commands.push(command);
                }
            }
            pages.push(DisplayPage {
                page_index: geometry.page_index(),
                width: geometry.width(),
                height: geometry.height(),
                commands,
                annotations: vec![],
            });
        }
        let mut font_instances = Vec::new();
        for (expected, (instance, face)) in used_fonts.into_iter().enumerate() {
            if instance.get()
                != u32::try_from(expected)
                    .map_err(|_| DisplayValidationError::NonDenseFontInstanceId)?
            {
                return Err(DisplayValidationError::NonDenseFontInstanceId);
            }
            font_instances.push(DisplayFontInstance {
                font_instance_id: instance,
                font_face_id: face,
            });
        }
        DisplayListBuilderOwner::new().issue(selected, text_map, font_instances, pages, config)
    }

    /// Paints the public `footnote-1` profile from the immutable body and
    /// dedicated definition-flow receipts. Coordinates and command order are
    /// derived here; callers cannot supply a separator or definition page.
    pub fn paint_footnote_profile(
        package: &ValidatedStagingStylePackage,
        selected: &PaginationResult,
        flow: &FlowTree,
        footnote_registry: &StagingFootnoteFlowRegistry,
        footnote_selected: &ValidatedFootnoteSelectedLayout,
        links: Option<&ValidatedStagingMachineLinkClusters>,
        config: &EffectiveConfig,
    ) -> Result<FootnoteProfileDisplay, FootnoteProfileDisplayError> {
        let parsed = package.package();
        let epoch = selected.selected_pass().fingerprint_record().layout_epoch();
        if flow != selected.selected_flow()
            || flow.epoch() != epoch
            || epoch != footnote_selected.epoch()
            || epoch != footnote_registry.receipt().epoch()
            || epoch.document() != parsed.epoch_identity().document()
            || epoch.style() != parsed.epoch_identity().style()
            || selected.final_fingerprint() != footnote_selected.body_layout_fingerprint()
            || footnote_selected.profile_fingerprint()
                != footnote_registry.receipt().profile_fingerprint()
            || footnote_selected.registry_fingerprint() != footnote_registry.receipt().fingerprint()
            || footnote_selected.pages().len() != selected.selected_pages().len()
            || footnote_selected.master_id() != footnote_registry.master_id()
            || footnote_selected.body_frame() != footnote_registry.body_frame()
            || footnote_selected.maximum_footnote_frame()
                != footnote_registry.maximum_footnote_frame()
        {
            return Err(FootnoteProfileDisplayError::SelectedLayoutMismatch);
        }
        let paragraph_items = flow
            .paragraph_items()
            .ok_or(FootnoteProfileDisplayError::RegistryMismatch)?;
        if paragraph_items.epoch() != epoch {
            return Err(FootnoteProfileDisplayError::RegistryMismatch);
        }
        let definition_lines =
            footnote_definition_paint_lines(parsed, footnote_registry, paragraph_items)?;
        let link_usage = package.preflight_footnote_link_usage().map_err(|_| {
            FootnoteProfileDisplayError::Link(StagingMachineLinkDisplayError::ReceiptMismatch)
        })?;
        let mut link_collector = match (link_usage.links().is_empty(), links) {
            (true, None) => None,
            (false, Some(links))
                if links.verifies(package, paragraph_items)
                    && links.usage_sha256() == link_usage.usage_sha256() =>
            {
                Some(FootnoteMachineLinkCollector::new(
                    links,
                    config.limits().get().max_fragments,
                ))
            }
            _ => {
                return Err(FootnoteProfileDisplayError::Link(
                    StagingMachineLinkDisplayError::ReceiptMismatch,
                ))
            }
        };

        let mut parsed_spans = Vec::new();
        let mut generated_spans = Vec::new();
        for page in selected.selected_pages() {
            for fragment in &page.fragments {
                if paragraph_items.item_count(fragment.owner).is_some() {
                    append_display_source_spans(
                        fragment_shaped_slices(paragraph_items, fragment)
                            .map_err(FootnoteProfileDisplayError::Display)?,
                        &mut parsed_spans,
                        &mut generated_spans,
                    );
                }
            }
        }
        for page in footnote_selected.pages() {
            for flow in page.flows() {
                let line_set = definition_lines
                    .get(flow.assignment().flow_id().get() as usize)
                    .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch)?;
                for fragment in flow.fragments() {
                    let definition_fragment = line_set
                        .get(fragment.fragment_ordinal() as usize)
                        .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch)?;
                    for line in &definition_fragment.lines {
                        append_display_source_spans(
                            paragraph_shaped_slices(
                                paragraph_items,
                                line.owner,
                                line.start_item,
                                line.end_item,
                            )
                            .map_err(FootnoteProfileDisplayError::Display)?,
                            &mut parsed_spans,
                            &mut generated_spans,
                        );
                    }
                }
            }
        }
        let text_map =
            DisplayTextMap::from_selected_spans(parsed, selected, &parsed_spans, &generated_spans)
                .map_err(|_| {
                    FootnoteProfileDisplayError::Display(
                        DisplayValidationError::SelectedTextMapMismatch,
                    )
                })?;
        let mut destinations = destinations_from_selected_pagination(selected)
            .map_err(FootnoteProfileDisplayError::Display)?;
        let destination_ids: std::collections::BTreeSet<_> = destinations
            .iter()
            .map(|destination| destination.anchor_id.clone())
            .collect();
        if destination_ids.len() != destinations.len() {
            return Err(FootnoteProfileDisplayError::SelectedLayoutMismatch);
        }

        let body_end = footnote_selected
            .body_frame()
            .y()
            .checked_add(footnote_selected.body_frame().height().get())
            .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
        let separator_center_offset =
            Length::from_raw(16_384).ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
        let separator_width = Length::from_raw(32_768)
            .and_then(PositiveLength::new)
            .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
        let separator_band = Length::from_raw(FOOTNOTE_SEPARATOR_BAND_RAW)
            .and_then(PositiveLength::new)
            .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
        let miter_limit = PositiveUnitless16_16::new(Unitless16_16::from_raw(4 * 65_536))
            .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
        let empty_dash = DashPattern::new(Vec::new(), NonNegativeLength::ZERO)
            .map_err(|_| FootnoteProfileDisplayError::SeparatorMismatch)?;

        let mut used_fonts = std::collections::BTreeMap::new();
        let mut next_run_id = 0u32;
        let mut pages = Vec::new();
        let mut page_geometries = Vec::new();
        let mut page_observations = Vec::new();
        let mut command_observations = Vec::new();
        let base_geometry = selected
            .selected_page_geometry()
            .last()
            .ok_or(FootnoteProfileDisplayError::SelectedLayoutMismatch)?;
        pages
            .try_reserve_exact(footnote_selected.pages().len())
            .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;
        page_geometries
            .try_reserve_exact(footnote_selected.pages().len())
            .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;
        for (page_ordinal, footnote_page) in footnote_selected.pages().iter().enumerate() {
            let page_index = u32::try_from(page_ordinal)
                .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;
            if page_index != footnote_page.page_index() {
                return Err(FootnoteProfileDisplayError::SelectedLayoutMismatch);
            }
            let (body_fragments, page_width, page_height) =
                match selected.selected_pages().get(page_ordinal) {
                    Some(page_plan) => {
                        let geometry = selected
                            .selected_page_geometry()
                            .get(page_ordinal)
                            .ok_or(FootnoteProfileDisplayError::SelectedLayoutMismatch)?;
                        let expected_frame_count =
                            if footnote_page.reservation() == NonNegativeLength::ZERO {
                                1
                            } else {
                                2
                            };
                        let expected_footnote_y = footnote_selected
                            .body_frame()
                            .y()
                            .checked_add(footnote_selected.body_frame().height().get())
                            .and_then(|end| end.checked_sub(footnote_page.reservation().get()));
                        if page_plan.page_index != page_index
                            || page_plan.master_id != *footnote_selected.master_id()
                            || geometry.page_index() != page_index
                            || geometry.master_id() != footnote_selected.master_id()
                            || page_plan.frames.len() != expected_frame_count
                            || page_plan.frames.first().map_or(true, |frame| {
                                frame.kind != PageFrameKind::Body
                                    || frame.column_index != 0
                                    || frame.bounds != footnote_selected.body_frame()
                            })
                            || (expected_frame_count == 2
                                && page_plan.frames.get(1).map_or(true, |frame| {
                                    frame.kind != PageFrameKind::Footnote
                                        || frame.column_index != 0
                                        || Some(frame.bounds.y()) != expected_footnote_y
                                        || frame.bounds.x()
                                            != footnote_selected.maximum_footnote_frame().x()
                                        || frame.bounds.width()
                                            != footnote_selected.maximum_footnote_frame().width()
                                        || frame.bounds.height().get()
                                            != footnote_page.reservation().get()
                                }))
                        {
                            return Err(FootnoteProfileDisplayError::SelectedLayoutMismatch);
                        }
                        (
                            page_plan.fragments.as_slice(),
                            geometry.width(),
                            geometry.height(),
                        )
                    }
                    None => {
                        if !footnote_page.discovery().is_empty()
                            || !footnote_page.body_continuation().is_terminal()
                            || base_geometry.master_id() != footnote_selected.master_id()
                        {
                            return Err(FootnoteProfileDisplayError::SelectedLayoutMismatch);
                        }
                        (&[][..], base_geometry.width(), base_geometry.height())
                    }
                };
            page_geometries.push(ValidatedDisplayPageGeometry {
                page_index,
                master_id: footnote_selected.master_id().clone(),
                width: page_width,
                height: page_height,
            });
            let mut commands = Vec::new();
            let mut body_markers = Vec::new();
            for fragment in body_fragments {
                if paragraph_items.item_count(fragment.owner).is_none() {
                    if parsed.document_nodes().node_kind(fragment.owner)
                        == Some(typaxis_document::DocumentNodeKind::Figure)
                    {
                        let image_id = basic_figure_image_id(
                            &parsed.package().document.blocks,
                            fragment.owner,
                        )
                        .ok_or(FootnoteProfileDisplayError::Display(
                            DisplayValidationError::UnsupportedReferencePaintDomain,
                        ))?;
                        commands.push(DisplayCommand::DrawImage {
                            image_id,
                            rect: fragment.bounds,
                        });
                    }
                    continue;
                }
                let (start, end) = fragment_item_range(paragraph_items, fragment)
                    .map_err(FootnoteProfileDisplayError::Display)?;
                paint_reference_item_range(
                    parsed,
                    &text_map,
                    paragraph_items,
                    fragment.owner,
                    start,
                    end,
                    fragment.bounds,
                    fragment.bounds.width(),
                    FootnoteTextAlign::Start,
                    None,
                    page_index,
                    fragment.owner_local_ordinal,
                    &mut body_markers,
                    link_collector.as_mut(),
                    &mut used_fonts,
                    &mut next_run_id,
                    &mut commands,
                )
                .map_err(FootnoteProfileDisplayError::Display)?;
            }

            let references: Vec<_> = footnote_page
                .discovery()
                .iter()
                .map(|occurrence| FootnotePaintReferenceObservation {
                    footnote_id: occurrence.footnote_id().clone(),
                    reference_owner: occurrence.reference_owner(),
                })
                .collect();
            let mut reference_by_owner: std::collections::BTreeMap<_, _> = references
                .iter()
                .map(|reference| (reference.reference_owner, reference.footnote_id.clone()))
                .collect();
            if body_markers.len() != references.len()
                || reference_by_owner.len() != references.len()
            {
                return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
            }
            for (command_index, reference_owner) in body_markers.iter().copied() {
                let footnote_id = reference_by_owner
                    .remove(&reference_owner)
                    .ok_or(FootnoteProfileDisplayError::DefinitionPaintOrder)?;
                let command = commands
                    .get(command_index as usize)
                    .filter(|command| matches!(command, DisplayCommand::DrawGlyphRun { .. }))
                    .ok_or(FootnoteProfileDisplayError::DefinitionPaintOrder)?
                    .clone();
                command_observations.push(FootnotePaintCommandObservation {
                    page_index,
                    page_command_index: command_index,
                    kind: FootnotePaintCommandKind::ReferenceMarker,
                    assignment_ordinal: None,
                    flow_id: None,
                    footnote_id: Some(footnote_id),
                    fragment_ordinal: None,
                    reference_owner: Some(reference_owner),
                    command,
                });
            }
            if !reference_by_owner.is_empty() {
                return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
            }
            let body_command_count = u32::try_from(commands.len())
                .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;

            let ordered_ids: Vec<_> = footnote_page
                .ordered_footnotes()
                .iter()
                .map(|assignment| assignment.footnote_id().clone())
                .collect();
            if footnote_page.flows().len() != footnote_page.ordered_footnotes().len()
                || footnote_page
                    .flows()
                    .iter()
                    .zip(footnote_page.ordered_footnotes())
                    .any(|(flow, assignment)| flow.assignment() != assignment)
                || (footnote_page.flows().is_empty()
                    != (footnote_page.reservation() == NonNegativeLength::ZERO))
            {
                return Err(FootnoteProfileDisplayError::DefinitionPaintOrder);
            }

            let mut flow_observations = Vec::new();
            if !footnote_page.flows().is_empty() {
                let actual_start = body_end
                    .checked_sub(footnote_page.reservation().get())
                    .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
                let separator_y = actual_start
                    .checked_add(separator_center_offset)
                    .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
                let separator_end_x = footnote_selected
                    .maximum_footnote_frame()
                    .x()
                    .checked_add(footnote_selected.maximum_footnote_frame().width().get())
                    .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
                let separator = DisplayCommand::StrokePath {
                    path: Path::new(vec![
                        PathVerb::MoveTo(Point {
                            x: footnote_selected.maximum_footnote_frame().x(),
                            y: separator_y,
                        }),
                        PathVerb::LineTo(Point {
                            x: separator_end_x,
                            y: separator_y,
                        }),
                    ])
                    .map_err(|_| FootnoteProfileDisplayError::SeparatorMismatch)?,
                    paint: Paint::Gray(0),
                    stroke: StrokeStyle {
                        width: separator_width,
                        line_cap: LineCap::Butt,
                        line_join: LineJoin::Miter,
                        miter_limit,
                        dash: empty_dash.clone(),
                    },
                };
                let separator_index = u32::try_from(commands.len())
                    .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?;
                commands.push(separator.clone());
                command_observations.push(FootnotePaintCommandObservation {
                    page_index,
                    page_command_index: separator_index,
                    kind: FootnotePaintCommandKind::Separator,
                    assignment_ordinal: None,
                    flow_id: None,
                    footnote_id: None,
                    fragment_ordinal: None,
                    reference_owner: None,
                    command: separator,
                });

                let mut y = actual_start
                    .checked_add(separator_band.get())
                    .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
                for selected_flow in footnote_page.flows() {
                    let assignment = selected_flow.assignment();
                    let registered = footnote_registry
                        .flow(assignment.flow_id())
                        .filter(|registered| {
                            registered.binding().footnote_id() == assignment.footnote_id()
                        })
                        .ok_or(FootnoteProfileDisplayError::RegistryMismatch)?;
                    let line_set = definition_lines
                        .get(assignment.flow_id().get() as usize)
                        .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch)?;
                    for selected_fragment in selected_flow.fragments() {
                        let ordinal = selected_fragment.fragment_ordinal();
                        let definition_fragment = line_set
                            .get(ordinal as usize)
                            .filter(|fragment| fragment.extent == selected_fragment.block_extent())
                            .ok_or(FootnoteProfileDisplayError::DefinitionFragmentMismatch)?;
                        if registered.fragment_extents().get(ordinal as usize)
                            != Some(&selected_fragment.block_extent())
                        {
                            return Err(FootnoteProfileDisplayError::DefinitionFragmentMismatch);
                        }
                        let fragment_start_y = y;
                        for line in &definition_fragment.lines {
                            let origin_y = y
                                .checked_add(line.space_before.get())
                                .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
                            let bounds = Rect::new(
                                footnote_selected
                                    .maximum_footnote_frame()
                                    .x()
                                    .checked_add(line.physical_left_inset.get())
                                    .ok_or(FootnoteProfileDisplayError::NumericOverflow)?,
                                origin_y,
                                line.inline_size,
                                line.line_height,
                            );
                            if line.start_item == 0 {
                                for anchor_id in
                                    footnote_definition_block_anchors(parsed, line.owner)?
                                {
                                    let expected = NamedDestination {
                                        anchor_id: anchor_id.clone(),
                                        page_index,
                                        view: DestinationView::Xyz {
                                            point: Point {
                                                x: bounds.x(),
                                                y: bounds.y(),
                                            },
                                        },
                                    };
                                    if !destination_ids.contains(&anchor_id)
                                        || destinations
                                            .iter()
                                            .find(|destination| destination.anchor_id == anchor_id)
                                            != Some(&expected)
                                    {
                                        return Err(
                                            FootnoteProfileDisplayError::SelectedLayoutMismatch,
                                        );
                                    }
                                }
                            }
                            let justification_inline_size = if let Some(gap) = line.marker_gap {
                                bounds
                                    .width()
                                    .get()
                                    .checked_sub(gap.get())
                                    .and_then(PositiveLength::new)
                                    .ok_or(
                                        FootnoteProfileDisplayError::DefinitionFragmentMismatch,
                                    )?
                            } else {
                                bounds.width()
                            };
                            let command_start = commands.len();
                            let marker_count = paint_reference_item_range(
                                parsed,
                                &text_map,
                                paragraph_items,
                                line.owner,
                                line.start_item,
                                line.end_item,
                                bounds,
                                justification_inline_size,
                                line.text_align,
                                line.marker_owner.zip(line.marker_gap),
                                page_index,
                                line.line_ordinal,
                                &mut body_markers,
                                link_collector.as_mut(),
                                &mut used_fonts,
                                &mut next_run_id,
                                &mut commands,
                            )
                            .map_err(FootnoteProfileDisplayError::Display)?;
                            if marker_count != u32::from(line.marker_owner.is_some()) {
                                return Err(
                                    FootnoteProfileDisplayError::DefinitionFragmentMismatch,
                                );
                            }
                            for (command_index, command) in
                                commands.iter().enumerate().skip(command_start)
                            {
                                command_observations.push(FootnotePaintCommandObservation {
                                    page_index,
                                    page_command_index: u32::try_from(command_index).map_err(
                                        |_| FootnoteProfileDisplayError::NumericOverflow,
                                    )?,
                                    kind: FootnotePaintCommandKind::Definition,
                                    assignment_ordinal: Some(assignment.assignment_ordinal()),
                                    flow_id: Some(assignment.flow_id()),
                                    footnote_id: Some(assignment.footnote_id().clone()),
                                    fragment_ordinal: Some(ordinal),
                                    reference_owner: None,
                                    command: command.clone(),
                                });
                            }
                            y = y
                                .checked_add(line.extent.get())
                                .ok_or(FootnoteProfileDisplayError::NumericOverflow)?;
                        }
                        if y.checked_sub(fragment_start_y)
                            != Some(selected_fragment.block_extent().get())
                        {
                            return Err(FootnoteProfileDisplayError::DefinitionFragmentMismatch);
                        }
                    }
                    flow_observations.push(FootnotePaintFlowObservation {
                        footnote_id: assignment.footnote_id().clone(),
                        assignment_ordinal: assignment.assignment_ordinal(),
                        flow_id: assignment.flow_id(),
                        before_fragment: selected_flow.before_cursor().next_fragment_ordinal(),
                        after_fragment: selected_flow.after_cursor().next_fragment_ordinal(),
                        incoming_source_page: selected_flow.incoming_source_page(),
                        carries_out: selected_flow.carries_out(),
                    });
                }
                if y != body_end {
                    return Err(FootnoteProfileDisplayError::SeparatorMismatch);
                }
            }
            page_observations.push(FootnotePaintPageObservation {
                page_index,
                body_continuation_position: footnote_page.body_continuation().next_flow_position(),
                body_continuation_terminal: footnote_page.body_continuation().is_terminal(),
                body_fingerprint: footnote_page.body_fingerprint(),
                body_command_count,
                evaluation_count: footnote_page.evaluation_count(),
                reservation: footnote_page.reservation(),
                ordered_footnote_ids: ordered_ids,
                references,
                flows: flow_observations,
            });
            pages.push(DisplayPage {
                page_index,
                width: page_width,
                height: page_height,
                commands,
                annotations: Vec::new(),
            });
        }
        if let Some(collector) = link_collector {
            for annotation in collector
                .finish()
                .map_err(FootnoteProfileDisplayError::Link)?
            {
                pages
                    .get_mut(annotation.page_index as usize)
                    .ok_or(FootnoteProfileDisplayError::Link(
                        StagingMachineLinkDisplayError::WrongPage(NodeId::new(
                            annotation.link_node_id,
                        )),
                    ))?
                    .annotations
                    .push(LinkAnnotation {
                        target: annotation.target.to_display_target(),
                        rect: annotation.rect,
                    });
            }
        }
        destinations.sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
        if destinations.len() != parsed.document_nodes().anchors().len()
            || destinations
                .windows(2)
                .any(|pair| pair[0].anchor_id == pair[1].anchor_id)
        {
            return Err(FootnoteProfileDisplayError::SelectedLayoutMismatch);
        }
        let mut font_instances = Vec::new();
        for (expected, (instance, face)) in used_fonts.into_iter().enumerate() {
            if instance.get()
                != u32::try_from(expected)
                    .map_err(|_| FootnoteProfileDisplayError::NumericOverflow)?
            {
                return Err(FootnoteProfileDisplayError::Display(
                    DisplayValidationError::NonDenseFontInstanceId,
                ));
            }
            font_instances.push(DisplayFontInstance {
                font_instance_id: instance,
                font_face_id: face,
            });
        }
        let trusted = DisplayListBuilderOwner::new()
            .issue_footnote(
                selected,
                text_map,
                FootnoteDisplayIssue {
                    font_instances,
                    destinations,
                    pages,
                    selected_page_geometry: page_geometries,
                },
                config,
            )
            .map_err(FootnoteProfileDisplayError::Display)?;
        let mut closure = FootnoteDisplayClosureReceipt {
            profile_sha256: footnote_selected.profile_fingerprint().bytes(),
            registry_sha256: footnote_selected.registry_fingerprint().bytes(),
            selected_layout_sha256: footnote_selected.fingerprint().bytes(),
            body_layout_sha256: footnote_selected.body_layout_fingerprint().bytes(),
            pages: page_observations,
            commands: command_observations,
            canonical_jcs: String::new(),
        };
        closure.canonical_jcs = encode_footnote_display_closure(&closure);
        validate_footnote_display_closure(&trusted, &closure)?;
        Ok(FootnoteProfileDisplay { trusted, closure })
    }

    /// Paint the immutable `table-1` domain. Ordinary body-flow commands are
    /// retained, while cell text is issued only from the independently
    /// selected table receipts and may be repeated only by a sealed header
    /// occurrence.
    pub fn paint_table_profile(
        package: &ValidatedStagingStylePackage,
        selected: &PaginationResult,
        flow: &FlowTree,
        tables: &[TableProfilePaintInput<'_>],
        links: Option<&ValidatedStagingMachineLinkClusters>,
        config: &EffectiveConfig,
    ) -> Result<TableProfileDisplay, TableProfileDisplayError> {
        let parsed = package.package();
        let selected_epoch = selected.selected_pass().fingerprint_record().layout_epoch();
        if flow != selected.selected_flow()
            || flow.epoch() != selected_epoch
            || selected_epoch.document() != parsed.epoch_identity().document()
            || selected_epoch.style() != parsed.epoch_identity().style()
            || !parsed.package().document.footnotes.is_empty()
        {
            return Err(TableProfileDisplayError::ParagraphRegistryMismatch);
        }
        let body_registry = flow
            .paragraph_items()
            .ok_or(TableProfileDisplayError::ParagraphRegistryMismatch)?;
        let expected_table_owners: Vec<_> = parsed
            .package()
            .document
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Table { node_id, .. } => Some(*node_id),
                _ => None,
            })
            .collect();
        if expected_table_owners.len() != tables.len()
            || tables
                .iter()
                .zip(&expected_table_owners)
                .any(|(input, owner)| {
                    input.grid.table_owner() != *owner
                        || input.grid.package_sha256() != package.package_fingerprint().into_bytes()
                })
        {
            return Err(TableProfileDisplayError::TableSetMismatch);
        }
        let default_master = parsed
            .package()
            .page_masters
            .masters
            .iter()
            .find(|master| master.master_id == parsed.package().page_masters.default_master_id)
            .ok_or(TableProfileDisplayError::TableSetMismatch)?;
        validate_table_profile_page_mapping(parsed, selected, tables, default_master.body)
            .map_err(TableProfileDisplayError::Closure)?;

        let mut closures = Vec::new();
        let mut table_lines = Vec::new();
        closures
            .try_reserve_exact(tables.len())
            .map_err(|_| TableProfileDisplayError::NumericOverflow)?;
        table_lines
            .try_reserve_exact(tables.len())
            .map_err(|_| TableProfileDisplayError::NumericOverflow)?;
        for input in tables {
            if input.paragraph_items.epoch() != selected_epoch {
                return Err(TableProfileDisplayError::ParagraphRegistryMismatch);
            }
            let closure = TableDisplayClosureReceipt::from_selected(
                selected.final_fingerprint().bytes(),
                input.grid,
                input.layout,
                input.selected,
                input.page_bodies.to_vec(),
            )
            .map_err(TableProfileDisplayError::Closure)?;
            let mut lines = std::collections::BTreeMap::new();
            for cell in input.grid.cells() {
                let cell_lines = derive_table_cell_paint_lines(
                    parsed,
                    input.paragraph_items,
                    input.layout,
                    input.grid.table_owner(),
                    cell.cell_owner(),
                )?;
                if lines.insert(cell.cell_owner(), cell_lines).is_some() {
                    return Err(TableProfileDisplayError::CellContentMismatch);
                }
            }
            closures.push(closure);
            table_lines.push(lines);
        }

        let mut parsed_spans = Vec::new();
        let mut generated_spans = Vec::new();
        for page in selected.selected_pages() {
            for fragment in &page.fragments {
                if body_registry.item_count(fragment.owner).is_none() {
                    continue;
                }
                for slice in fragment_shaped_slices(body_registry, fragment)
                    .map_err(TableProfileDisplayError::Display)?
                {
                    push_table_profile_shape_span(
                        slice.shaped,
                        &mut parsed_spans,
                        &mut generated_spans,
                    );
                }
            }
        }
        for ((closure, lines), input) in closures.iter().zip(&table_lines).zip(tables) {
            for record in closure.records() {
                let cell_lines = lines
                    .get(&record.cell_node_id)
                    .ok_or(TableProfileDisplayError::CellContentMismatch)?;
                let selected_lines = table_selected_cell_lines(cell_lines, record)?;
                for line in selected_lines {
                    for slice in paragraph_shaped_slices(
                        input.paragraph_items,
                        line.paragraph_owner,
                        line.item_start,
                        line.item_end,
                    )
                    .map_err(TableProfileDisplayError::Display)?
                    {
                        push_table_profile_shape_span(
                            slice.shaped,
                            &mut parsed_spans,
                            &mut generated_spans,
                        );
                    }
                }
            }
        }
        let text_map =
            DisplayTextMap::from_selected_spans(parsed, selected, &parsed_spans, &generated_spans)
                .map_err(|_| {
                    TableProfileDisplayError::Display(
                        DisplayValidationError::SelectedTextMapMismatch,
                    )
                })?;

        let required_page_count = closures
            .iter()
            .filter_map(|closure| closure.page_bodies().last())
            .try_fold(
                u32::try_from(selected.selected_page_geometry().len())
                    .map_err(|_| TableProfileDisplayError::NumericOverflow)?,
                |count, page| {
                    page.target_page_index()
                        .checked_add(1)
                        .map(|table_count| count.max(table_count))
                        .ok_or(TableProfileDisplayError::NumericOverflow)
                },
            )?;
        if required_page_count == 0 || required_page_count > config.limits().get().max_pages {
            return Err(TableProfileDisplayError::TableSetMismatch);
        }
        let mut selected_page_geometry = Vec::new();
        selected_page_geometry
            .try_reserve_exact(required_page_count as usize)
            .map_err(|_| TableProfileDisplayError::NumericOverflow)?;
        for page_index in 0..required_page_count {
            if let Some(geometry) = selected.selected_page_geometry().get(page_index as usize) {
                selected_page_geometry.push(ValidatedDisplayPageGeometry {
                    page_index,
                    master_id: geometry.master_id().clone(),
                    width: geometry.width(),
                    height: geometry.height(),
                });
            } else {
                selected_page_geometry.push(ValidatedDisplayPageGeometry {
                    page_index,
                    master_id: default_master.master_id.clone(),
                    width: default_master.width,
                    height: default_master.height,
                });
            }
        }
        let mut pages: Vec<_> = selected_page_geometry
            .iter()
            .map(|geometry| DisplayPage {
                page_index: geometry.page_index,
                width: geometry.width,
                height: geometry.height,
                commands: Vec::new(),
                annotations: Vec::new(),
            })
            .collect();
        let mut used_fonts = std::collections::BTreeMap::new();
        let mut next_run_id = 0u32;

        for page_plan in selected.selected_pages() {
            let commands = &mut pages[page_plan.page_index as usize].commands;
            for fragment in &page_plan.fragments {
                if body_registry.item_count(fragment.owner).is_none() {
                    if parsed.document_nodes().node_kind(fragment.owner)
                        == Some(typaxis_document::DocumentNodeKind::Figure)
                    {
                        let image_id = basic_figure_image_id(
                            &parsed.package().document.blocks,
                            fragment.owner,
                        )
                        .ok_or(TableProfileDisplayError::Display(
                            DisplayValidationError::UnsupportedReferencePaintDomain,
                        ))?;
                        commands.push(DisplayCommand::DrawImage {
                            image_id,
                            rect: fragment.bounds,
                        });
                    }
                    continue;
                }
                paint_reference_fragment_commands(
                    parsed,
                    body_registry,
                    fragment,
                    &text_map,
                    &mut used_fonts,
                    &mut next_run_id,
                    commands,
                )
                .map_err(TableProfileDisplayError::Display)?;
            }
        }
        let mut painted_command_sets: Vec<Vec<TablePaintCommandObservation>> =
            (0..closures.len()).map(|_| Vec::new()).collect();
        for (table_index, ((closure, lines), input)) in
            closures.iter().zip(&table_lines).zip(tables).enumerate()
        {
            for record in closure.records() {
                let page = pages
                    .get_mut(record.page_index as usize)
                    .ok_or(TableProfileDisplayError::TableSetMismatch)?;
                let cell_lines = lines
                    .get(&record.cell_node_id)
                    .ok_or(TableProfileDisplayError::CellContentMismatch)?;
                let command_start = page.commands.len();
                for line in table_selected_cell_lines(cell_lines, record)? {
                    paint_table_cell_line_commands(
                        parsed,
                        input.paragraph_items,
                        &text_map,
                        record,
                        line,
                        &mut used_fonts,
                        &mut next_run_id,
                        &mut page.commands,
                    )?;
                }
                for (command_index, command) in page.commands[command_start..].iter().enumerate() {
                    let page_command_index = command_start
                        .checked_add(command_index)
                        .and_then(|value| u32::try_from(value).ok())
                        .ok_or(TableProfileDisplayError::NumericOverflow)?;
                    painted_command_sets[table_index].push(TablePaintCommandObservation {
                        page_index: record.page_index,
                        page_command_index,
                        fragment_id: record.fragment_id,
                        repetition_index: record.repetition_index,
                        cell_node_id: record.cell_node_id,
                        command: command.clone(),
                    });
                }
            }
        }
        for (closure, commands) in closures.iter_mut().zip(painted_command_sets) {
            closure
                .bind_painted_commands(commands, &[])
                .map_err(TableProfileDisplayError::Closure)?;
        }
        if pages.iter().flat_map(|page| &page.commands).any(|command| {
            matches!(
                command,
                DisplayCommand::ClipPath { .. }
                    | DisplayCommand::FillPath { .. }
                    | DisplayCommand::StrokePath { .. }
            )
        }) {
            return Err(TableProfileDisplayError::Closure(
                TableDisplayClosureError::DecorationForbidden,
            ));
        }

        if let Some(links) = links {
            if !links.verifies(package, body_registry) || links.ranges().is_empty() {
                return Err(TableProfileDisplayError::Link(
                    StagingMachineLinkDisplayError::ReceiptMismatch,
                ));
            }
            let annotations = derive_staging_machine_link_rectangles(
                selected,
                body_registry,
                links,
                config.limits().get().max_fragments,
            )
            .map_err(TableProfileDisplayError::Link)?;
            for annotation in annotations {
                pages
                    .get_mut(annotation.page_index as usize)
                    .ok_or(TableProfileDisplayError::Link(
                        StagingMachineLinkDisplayError::WrongPage(NodeId::new(
                            annotation.link_node_id,
                        )),
                    ))?
                    .annotations
                    .push(LinkAnnotation {
                        target: annotation.target.to_display_target(),
                        rect: annotation.rect,
                    });
            }
        }

        let mut font_instances = Vec::new();
        for (expected, (instance, face)) in used_fonts.into_iter().enumerate() {
            if instance.get()
                != u32::try_from(expected).map_err(|_| {
                    TableProfileDisplayError::Display(
                        DisplayValidationError::NonDenseFontInstanceId,
                    )
                })?
            {
                return Err(TableProfileDisplayError::Display(
                    DisplayValidationError::NonDenseFontInstanceId,
                ));
            }
            font_instances.push(DisplayFontInstance {
                font_instance_id: instance,
                font_face_id: face,
            });
        }
        let document = DisplayDocument {
            source_layout: DisplaySourceLayout::from_selected_pagination(selected),
            text_buffers: text_map.buffers().to_vec(),
            font_instances,
            destinations: destinations_from_selected_pagination(selected)
                .map_err(TableProfileDisplayError::Display)?,
            pages,
        };
        let structural = StructurallyValidatedDisplayDocument::from_verified_table_profile(
            document,
            selected,
            config,
            selected_page_geometry,
        )
        .map_err(TableProfileDisplayError::Display)?;
        Ok(TableProfileDisplay {
            trusted: ValidatedDisplayDocument { structural },
            tables: closures,
        })
    }

    /// Safe reference painter for the complete domain owned by
    /// `ReferencePaginator`: blank documents and top-level empty paragraphs
    /// containing only direct anchors.
    ///
    /// Pages are derived exclusively from the selected pagination geometry.
    /// Empty paragraph fragments produce no commands, and named destinations
    /// are derived by the private paint owner from the selected placed-anchor
    /// closure.
    pub fn paint_reference_selected(
        package: &ValidatedParsedPackage,
        selected: &PaginationResult,
        config: &EffectiveConfig,
    ) -> Result<Self, DisplayValidationError> {
        if !reference_paint_domain_is_supported(package, selected) {
            return Err(DisplayValidationError::UnsupportedReferencePaintDomain);
        }
        Self::paint_empty_selected_pages(package, selected, config)
    }

    /// Safe minimal reference painter for a genuinely empty selected layout.
    /// Every emitted page and dimension is derived from the selected geometry;
    /// callers cannot supply commands, destinations, or text payloads.
    pub fn paint_blank_selected(
        package: &ValidatedParsedPackage,
        selected: &PaginationResult,
        config: &EffectiveConfig,
    ) -> Result<Self, DisplayValidationError> {
        if !package.package().document.blocks.is_empty()
            || !package.package().document.footnotes.is_empty()
            || !package.package().text_store.buffers().is_empty()
            || !selected
                .selected_pass()
                .generated_text()
                .buffers()
                .is_empty()
            || selected.selected_pages().iter().any(|page| {
                !page.fragments.is_empty()
                    || !page.footnote_ids.is_empty()
                    || !page.float_decisions.is_empty()
                    || !page.column_decisions.is_empty()
                    || !page.resolved_references.is_empty()
            })
        {
            return Err(DisplayValidationError::NonBlankSelectedLayout);
        }
        Self::paint_empty_selected_pages(package, selected, config)
    }

    fn paint_empty_selected_pages(
        package: &ValidatedParsedPackage,
        selected: &PaginationResult,
        config: &EffectiveConfig,
    ) -> Result<Self, DisplayValidationError> {
        let text_map = DisplayTextMap::from_selected_spans(package, selected, &[], &[])
            .map_err(|_| DisplayValidationError::SelectedTextMapMismatch)?;
        let pages = selected
            .selected_page_geometry()
            .iter()
            .map(|geometry| DisplayPage {
                page_index: geometry.page_index(),
                width: geometry.width(),
                height: geometry.height(),
                commands: vec![],
                annotations: vec![],
            })
            .collect();
        DisplayListBuilderOwner::new().issue(selected, text_map, vec![], pages, config)
    }

    pub const fn document(&self) -> &DisplayDocument {
        self.structural.document()
    }
    pub fn selected_page_geometry(&self) -> &[ValidatedDisplayPageGeometry] {
        self.structural.selected_page_geometry()
    }
    pub fn into_document(self) -> DisplayDocument {
        self.structural.into_document()
    }
    pub fn into_parts(self) -> (DisplayDocument, Vec<ValidatedDisplayPageGeometry>) {
        self.structural.into_parts()
    }
}

fn basic_figure_image_id(blocks: &[Block], owner: NodeId) -> Option<ImageResourceId> {
    let mut pending: Vec<&Block> = blocks.iter().rev().collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::Figure {
                node_id,
                image_id,
                caption,
                ..
            } => {
                if *node_id == owner {
                    return Some(*image_id);
                }
                pending.extend(caption.iter().rev());
            }
            Block::List { items, .. } => {
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Table { head, body, .. } => pending.extend(
                body.iter()
                    .rev()
                    .chain(head.iter().rev())
                    .flat_map(|row| row.cells.iter().rev())
                    .flat_map(|cell| cell.blocks.iter().rev()),
            ),
            Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
        }
    }
    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TableCellTextAlign {
    Start,
    End,
    Center,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TableCellPaintLine {
    paragraph_owner: NodeId,
    item_start: u32,
    item_end: u32,
    fragment_block_offset: i64,
    fragment_block_size: PositiveLength,
    paint_block_offset: i64,
    line_height: PositiveLength,
    inline_start: NonNegativeLength,
    inline_size: PositiveLength,
    text_align: TableCellTextAlign,
}

fn derive_table_cell_paint_lines(
    package: &ValidatedParsedPackage,
    registry: &ValidatedParagraphItemRegistry,
    layout: &TableRowBandLayoutReceipt,
    table_owner: NodeId,
    cell_owner: NodeId,
) -> Result<Vec<TableCellPaintLine>, TableProfileDisplayError> {
    let table = package
        .package()
        .document
        .blocks
        .iter()
        .find(|block| matches!(block, Block::Table { node_id, .. } if *node_id == table_owner))
        .ok_or(TableProfileDisplayError::TableSetMismatch)?;
    let Block::Table { head, body, .. } = table else {
        return Err(TableProfileDisplayError::TableSetMismatch);
    };
    let cell = head
        .iter()
        .chain(body)
        .flat_map(|row| &row.cells)
        .find(|cell| cell.node_id == cell_owner)
        .ok_or(TableProfileDisplayError::CellContentMismatch)?;
    let measured = layout
        .cell(cell_owner)
        .ok_or(TableProfileDisplayError::CellContentMismatch)?;
    let mut lines = Vec::new();
    let mut block_offset = 0i64;
    for block in &cell.blocks {
        let Block::Paragraph { node_id, .. } = block else {
            return Err(TableProfileDisplayError::CellContentMismatch);
        };
        let Some(paragraph_break) = registry.paragraph_break(*node_id) else {
            continue;
        };
        let computed = package
            .cascade_style(*node_id)
            .map_err(|_| TableProfileDisplayError::CellContentMismatch)?;
        let properties = computed.computed().properties();
        let line_height = table_positive_display_length(properties.get("line_height"))?;
        let space_before = table_nonnegative_display_length(properties.get("space_before"))?;
        let space_after = table_nonnegative_display_length(properties.get("space_after"))?;
        let inline_start = table_nonnegative_display_length(properties.get("start_indent"))?;
        let inline_end = table_nonnegative_display_length(properties.get("end_indent"))?;
        let inline_size = measured
            .frame_inline_size()
            .get()
            .checked_sub(inline_start.get())
            .and_then(|value| value.checked_sub(inline_end.get()))
            .and_then(PositiveLength::new)
            .ok_or(TableProfileDisplayError::CellContentMismatch)?;
        let text_align = match properties.get("text_align") {
            None => TableCellTextAlign::Start,
            Some(StyleValue::Keyword(value)) if value == "start" => TableCellTextAlign::Start,
            Some(StyleValue::Keyword(value)) if value == "end" => TableCellTextAlign::End,
            Some(StyleValue::Keyword(value)) if value == "center" => TableCellTextAlign::Center,
            _ => return Err(TableProfileDisplayError::CellContentMismatch),
        };
        let mut item_start = 0u32;
        for (line_index, line) in paragraph_break.lines.iter().enumerate() {
            if line.item_index <= item_start {
                return Err(TableProfileDisplayError::CellContentMismatch);
            }
            let before = if line_index == 0 {
                space_before.get().raw()
            } else {
                0
            };
            let after = if line_index + 1 == paragraph_break.lines.len() {
                space_after.get().raw()
            } else {
                0
            };
            let fragment_size = line_height
                .get()
                .raw()
                .checked_add(before)
                .and_then(|value| value.checked_add(after))
                .and_then(Length::from_raw)
                .and_then(PositiveLength::new)
                .ok_or(TableProfileDisplayError::NumericOverflow)?;
            let paint_block_offset = block_offset
                .checked_add(before)
                .ok_or(TableProfileDisplayError::NumericOverflow)?;
            lines.push(TableCellPaintLine {
                paragraph_owner: *node_id,
                item_start,
                item_end: line.item_index,
                fragment_block_offset: block_offset,
                fragment_block_size: fragment_size,
                paint_block_offset,
                line_height,
                inline_start,
                inline_size,
                text_align,
            });
            block_offset = block_offset
                .checked_add(fragment_size.get().raw())
                .ok_or(TableProfileDisplayError::NumericOverflow)?;
            item_start = line.item_index;
        }
        if item_start
            != registry
                .item_count(*node_id)
                .ok_or(TableProfileDisplayError::ParagraphRegistryMismatch)?
        {
            return Err(TableProfileDisplayError::CellContentMismatch);
        }
    }
    if lines.len() != measured.fragment_block_sizes().len()
        || lines
            .iter()
            .zip(measured.fragment_block_sizes())
            .any(|(line, measured)| line.fragment_block_size != *measured)
    {
        return Err(TableProfileDisplayError::CellContentMismatch);
    }
    Ok(lines)
}

fn table_positive_display_length(
    value: Option<&StyleValue>,
) -> Result<PositiveLength, TableProfileDisplayError> {
    match value {
        Some(StyleValue::Length(value)) => {
            PositiveLength::new(*value).ok_or(TableProfileDisplayError::CellContentMismatch)
        }
        _ => Err(TableProfileDisplayError::CellContentMismatch),
    }
}

fn table_nonnegative_display_length(
    value: Option<&StyleValue>,
) -> Result<NonNegativeLength, TableProfileDisplayError> {
    match value {
        Some(StyleValue::Length(value)) => {
            NonNegativeLength::new(*value).ok_or(TableProfileDisplayError::CellContentMismatch)
        }
        None => Ok(NonNegativeLength::ZERO),
        Some(_) => Err(TableProfileDisplayError::CellContentMismatch),
    }
}

fn table_selected_cell_lines<'a>(
    lines: &'a [TableCellPaintLine],
    record: &TablePaintCellObservation,
) -> Result<&'a [TableCellPaintLine], TableProfileDisplayError> {
    let start = usize::try_from(record.content_fragment_start)
        .map_err(|_| TableProfileDisplayError::NumericOverflow)?;
    let end = usize::try_from(record.content_fragment_end)
        .map_err(|_| TableProfileDisplayError::NumericOverflow)?;
    let selected = lines
        .get(start..end)
        .ok_or(TableProfileDisplayError::CellContentMismatch)?;
    let vertical_extent = record
        .vertical_offset_after
        .checked_sub(record.vertical_offset_before)
        .ok_or(TableProfileDisplayError::NumericOverflow)?;
    if vertical_extent != record.rect.height {
        return Err(TableProfileDisplayError::CellContentMismatch);
    }
    if let (Some(first), Some(last)) = (selected.first(), selected.last()) {
        let last_end = last
            .fragment_block_offset
            .checked_add(last.fragment_block_size.get().raw())
            .ok_or(TableProfileDisplayError::NumericOverflow)?;
        if first.fragment_block_offset < record.vertical_offset_before
            || last_end > record.vertical_offset_after
        {
            return Err(TableProfileDisplayError::CellContentMismatch);
        }
    }
    Ok(selected)
}

fn push_table_profile_shape_span(
    shaped: ShapedSlice,
    parsed: &mut Vec<TextSpan>,
    generated: &mut Vec<GeneratedTextSpan>,
) {
    match shaped.source() {
        ShapeSourceSpan::Parsed(span) => parsed.push(span),
        ShapeSourceSpan::Generated(provenance) => generated.push(provenance.text_span()),
    }
}

fn paint_reference_fragment_commands(
    package: &ValidatedParsedPackage,
    registry: &ValidatedParagraphItemRegistry,
    fragment: &typaxis_pagination::PlacedFragment,
    text_map: &DisplayTextMap,
    used_fonts: &mut std::collections::BTreeMap<FontInstanceId, FontFaceId>,
    next_run_id: &mut u32,
    commands: &mut Vec<DisplayCommand>,
) -> Result<(), DisplayValidationError> {
    let logical = fragment_shaped_slices(registry, fragment)?;
    if logical.is_empty() {
        return Ok(());
    }
    let levels: Vec<_> = logical
        .iter()
        .map(|slice| slice.shaped.bidi_level())
        .collect();
    let classes: Vec<_> = logical.iter().map(|slice| slice.class).collect();
    let paragraph_level = registry
        .paragraph_level(fragment.owner)
        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
    let after_l1 = reset_line_bidi_levels(paragraph_level, &levels, &classes)
        .map_err(|_| DisplayValidationError::UnsupportedReferencePaintDomain)?;
    let mut logical = reference_final_line_reshape(registry, fragment.owner, logical, &after_l1)?;
    justify_reference_line(registry, fragment, &mut logical)?;
    let order = reorder_line_l2(&after_l1)
        .map_err(|_| DisplayValidationError::UnsupportedReferencePaintDomain)?;
    let mut x = fragment.bounds.x();
    for logical_index in order.visual_to_logical() {
        let line_slice = logical
            .get(*logical_index as usize)
            .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
        let run = registry
            .runs(fragment.owner)
            .and_then(|runs| runs.get(line_slice.shaped.paragraph_run_index().get() as usize))
            .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
        let command = paint_shaped_slice(
            package,
            text_map,
            run,
            line_slice.shaped,
            after_l1.logical_levels()[*logical_index as usize],
            DisplayGlyphRunId::new(*next_run_id),
            Point {
                x,
                y: fragment.bounds.y(),
            },
        )?;
        *next_run_id = next_run_id
            .checked_add(1)
            .ok_or(DisplayValidationError::NonDenseGlyphRunId)?;
        register_table_profile_command_font(package, run, &command, used_fonts)?;
        x = x
            .checked_add(line_slice.advance)
            .ok_or(DisplayValidationError::NumericOutOfRange)?;
        commands.push(command);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn paint_table_cell_line_commands(
    package: &ValidatedParsedPackage,
    registry: &ValidatedParagraphItemRegistry,
    text_map: &DisplayTextMap,
    record: &TablePaintCellObservation,
    line: &TableCellPaintLine,
    used_fonts: &mut std::collections::BTreeMap<FontInstanceId, FontFaceId>,
    next_run_id: &mut u32,
    commands: &mut Vec<DisplayCommand>,
) -> Result<(), TableProfileDisplayError> {
    let logical = paragraph_shaped_slices(
        registry,
        line.paragraph_owner,
        line.item_start,
        line.item_end,
    )
    .map_err(TableProfileDisplayError::Display)?;
    if logical.is_empty() {
        return Ok(());
    }
    let levels: Vec<_> = logical
        .iter()
        .map(|slice| slice.shaped.bidi_level())
        .collect();
    let classes: Vec<_> = logical.iter().map(|slice| slice.class).collect();
    let paragraph_level = registry
        .paragraph_level(line.paragraph_owner)
        .ok_or(TableProfileDisplayError::ParagraphRegistryMismatch)?;
    let after_l1 = reset_line_bidi_levels(paragraph_level, &levels, &classes)
        .map_err(|_| TableProfileDisplayError::CellBidiMismatch)?;
    let mut logical =
        reference_final_line_reshape(registry, line.paragraph_owner, logical, &after_l1)
            .map_err(TableProfileDisplayError::Display)?;
    let item_count = registry
        .item_count(line.paragraph_owner)
        .ok_or(TableProfileDisplayError::ParagraphRegistryMismatch)?;
    let unadjusted = logical
        .iter()
        .try_fold(Length::ZERO, |total, slice| {
            total.checked_add(slice.advance)
        })
        .ok_or(TableProfileDisplayError::NumericOverflow)?;
    if line.item_end != item_count || unadjusted.raw() > line.inline_size.get().raw() {
        let adjustment = line
            .inline_size
            .get()
            .checked_sub(unadjusted)
            .ok_or(TableProfileDisplayError::NumericOverflow)?;
        distribute_justification(&mut logical, adjustment)
            .map_err(TableProfileDisplayError::Display)?;
    }
    let natural = logical
        .iter()
        .try_fold(Length::ZERO, |total, slice| {
            total.checked_add(slice.advance)
        })
        .ok_or(TableProfileDisplayError::NumericOverflow)?;
    let remaining = line
        .inline_size
        .get()
        .checked_sub(natural)
        .filter(|value| value.raw() >= 0)
        .ok_or(TableProfileDisplayError::CellInlineOverflow)?;
    let start_on_right = paragraph_level.get() % 2 == 1;
    let alignment_offset = match line.text_align {
        TableCellTextAlign::Center => Length::from_raw(remaining.raw() / 2)
            .ok_or(TableProfileDisplayError::NumericOverflow)?,
        TableCellTextAlign::Start if start_on_right => remaining,
        TableCellTextAlign::End if !start_on_right => remaining,
        _ => Length::ZERO,
    };
    let order =
        reorder_line_l2(&after_l1).map_err(|_| TableProfileDisplayError::CellBidiMismatch)?;
    let x = record
        .rect
        .x
        .checked_add(line.inline_start.get().raw())
        .and_then(|value| value.checked_add(alignment_offset.raw()))
        .and_then(Length::from_raw)
        .ok_or(TableProfileDisplayError::NumericOverflow)?;
    let y = record
        .rect
        .y
        .checked_add(line.paint_block_offset)
        .and_then(|value| value.checked_sub(record.vertical_offset_before))
        .and_then(Length::from_raw)
        .ok_or(TableProfileDisplayError::NumericOverflow)?;
    let line_bottom = y
        .raw()
        .checked_add(line.line_height.get().raw())
        .ok_or(TableProfileDisplayError::NumericOverflow)?;
    let record_bottom = record
        .rect
        .y
        .checked_add(record.rect.height)
        .ok_or(TableProfileDisplayError::NumericOverflow)?;
    if y.raw() < record.rect.y || line_bottom > record_bottom {
        return Err(TableProfileDisplayError::CellPaintBoundsMismatch);
    }
    let mut x = x;
    for logical_index in order.visual_to_logical() {
        let slice = logical
            .get(*logical_index as usize)
            .ok_or(TableProfileDisplayError::CellBidiMismatch)?;
        let run = registry
            .runs(line.paragraph_owner)
            .and_then(|runs| runs.get(slice.shaped.paragraph_run_index().get() as usize))
            .ok_or(TableProfileDisplayError::ParagraphRegistryMismatch)?;
        let command = paint_shaped_slice(
            package,
            text_map,
            run,
            slice.shaped,
            after_l1.logical_levels()[*logical_index as usize],
            DisplayGlyphRunId::new(*next_run_id),
            Point { x, y },
        )
        .map_err(TableProfileDisplayError::Display)?;
        *next_run_id = next_run_id
            .checked_add(1)
            .ok_or(TableProfileDisplayError::NumericOverflow)?;
        register_table_profile_command_font(package, run, &command, used_fonts)
            .map_err(TableProfileDisplayError::Display)?;
        x = x
            .checked_add(slice.advance)
            .ok_or(TableProfileDisplayError::NumericOverflow)?;
        commands.push(command);
    }
    Ok(())
}

fn register_table_profile_command_font(
    package: &ValidatedParsedPackage,
    run: &ValidatedGlyphRun,
    command: &DisplayCommand,
    used_fonts: &mut std::collections::BTreeMap<FontInstanceId, FontFaceId>,
) -> Result<(), DisplayValidationError> {
    let instance = command_font(command)?;
    let face = run.font_face_id();
    if package
        .package()
        .resources
        .font_faces
        .iter()
        .all(|declaration| declaration.font_face_id != face)
        || used_fonts
            .insert(instance, face)
            .is_some_and(|previous| previous != face)
    {
        return Err(DisplayValidationError::UnknownFontInstance);
    }
    Ok(())
}

fn fragment_shaped_slices(
    registry: &ValidatedParagraphItemRegistry,
    fragment: &typaxis_pagination::PlacedFragment,
) -> Result<Vec<LinePaintSlice>, DisplayValidationError> {
    let owner = fragment.owner;
    if fragment.start.owner() != owner {
        return Err(DisplayValidationError::UnsupportedReferencePaintDomain);
    }
    let item_count = registry
        .item_count(owner)
        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
    let start = fragment.start.owner_local_boundary();
    let end = if fragment.end.owner() == owner {
        fragment.end.owner_local_boundary()
    } else {
        item_count
    };
    if start >= end || end > item_count {
        return Err(DisplayValidationError::UnsupportedReferencePaintDomain);
    }
    paragraph_shaped_slices(registry, owner, start, end)
}

fn fragment_item_range(
    registry: &ValidatedParagraphItemRegistry,
    fragment: &typaxis_pagination::PlacedFragment,
) -> Result<(u32, u32), DisplayValidationError> {
    let owner = fragment.owner;
    if fragment.start.owner() != owner {
        return Err(DisplayValidationError::UnsupportedReferencePaintDomain);
    }
    let item_count = registry
        .item_count(owner)
        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
    let start = fragment.start.owner_local_boundary();
    let end = if fragment.end.owner() == owner {
        fragment.end.owner_local_boundary()
    } else {
        item_count
    };
    if start >= end || end > item_count {
        return Err(DisplayValidationError::UnsupportedReferencePaintDomain);
    }
    Ok((start, end))
}

#[allow(clippy::too_many_arguments)]
fn paint_reference_item_range(
    package: &ValidatedParsedPackage,
    text_map: &DisplayTextMap,
    registry: &ValidatedParagraphItemRegistry,
    owner: NodeId,
    start: u32,
    end: u32,
    bounds: Rect,
    justification_inline_size: PositiveLength,
    text_align: FootnoteTextAlign,
    definition_marker: Option<(NodeId, PositiveLength)>,
    page_index: u32,
    line_ordinal: u32,
    body_markers: &mut Vec<(u32, NodeId)>,
    mut link_collector: Option<&mut FootnoteMachineLinkCollector<'_>>,
    used_fonts: &mut std::collections::BTreeMap<FontInstanceId, FontFaceId>,
    next_run_id: &mut u32,
    commands: &mut Vec<DisplayCommand>,
) -> Result<u32, DisplayValidationError> {
    let mut logical = paragraph_shaped_slices(registry, owner, start, end)?;
    if logical.is_empty() {
        return Ok(0);
    }
    let levels: Vec<_> = logical
        .iter()
        .map(|slice| slice.shaped.bidi_level())
        .collect();
    let classes: Vec<_> = logical.iter().map(|slice| slice.class).collect();
    let paragraph_level = registry
        .paragraph_level(owner)
        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
    let after_l1 = reset_line_bidi_levels(paragraph_level, &levels, &classes)
        .map_err(|_| DisplayValidationError::UnsupportedReferencePaintDomain)?;
    logical = reference_final_line_reshape(registry, owner, logical, &after_l1)?;
    justify_reference_item_range(
        registry,
        owner,
        end,
        justification_inline_size,
        &mut logical,
    )?;
    let natural = logical
        .iter()
        .try_fold(Length::ZERO, |total, slice| {
            total.checked_add(slice.advance)
        })
        .ok_or(DisplayValidationError::NumericOutOfRange)?;
    let remaining = justification_inline_size
        .get()
        .checked_sub(natural)
        .filter(|remaining| remaining.raw() >= 0)
        .ok_or(DisplayValidationError::NumericOutOfRange)?;
    let start_on_right = paragraph_level.get() % 2 == 1;
    let alignment_offset = match text_align {
        FootnoteTextAlign::Center if start_on_right => Length::from_raw(
            remaining
                .raw()
                .checked_add(1)
                .ok_or(DisplayValidationError::NumericOutOfRange)?
                / 2,
        )
        .ok_or(DisplayValidationError::NumericOutOfRange)?,
        FootnoteTextAlign::Center => Length::from_raw(remaining.raw() / 2)
            .ok_or(DisplayValidationError::NumericOutOfRange)?,
        FootnoteTextAlign::Start if start_on_right => remaining,
        FootnoteTextAlign::End if !start_on_right => remaining,
        _ => Length::ZERO,
    };
    let order = reorder_line_l2(&after_l1)
        .map_err(|_| DisplayValidationError::UnsupportedReferencePaintDomain)?;
    let definition_marker_visual_bounds = definition_marker.and_then(|(marker_owner, gap)| {
        footnote_marker_visual_bounds(
            order.visual_to_logical().iter().map(|logical_index| {
                logical.get(*logical_index as usize).is_some_and(|slice| {
                    shaped_slice_is_footnote_marker(slice.shaped, marker_owner)
                })
            }),
            gap,
        )
    });
    let mut x = bounds
        .x()
        .checked_add(alignment_offset)
        .ok_or(DisplayValidationError::NumericOutOfRange)?;
    let mut body_marker_owners = std::collections::BTreeSet::new();
    for (visual_index, logical_index) in order.visual_to_logical().iter().enumerate() {
        let line_slice = logical
            .get(*logical_index as usize)
            .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
        let shaped = line_slice.shaped;
        if start_on_right
            && definition_marker_visual_bounds.is_some_and(|(first, _, _)| visual_index == first)
        {
            let gap = definition_marker_visual_bounds
                .map(|(_, _, gap)| gap.get())
                .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
            x = x
                .checked_add(gap)
                .ok_or(DisplayValidationError::NumericOutOfRange)?;
        }
        let runs = registry
            .runs(owner)
            .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
        let run = runs
            .get(shaped.paragraph_run_index().get() as usize)
            .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
        let command = paint_shaped_slice(
            package,
            text_map,
            run,
            shaped,
            after_l1.logical_levels()[*logical_index as usize],
            DisplayGlyphRunId::new(*next_run_id),
            Point { x, y: bounds.y() },
        )?;
        if let Some(collector) = link_collector.as_deref_mut() {
            collector
                .observe(FootnoteMachineLinkClusterObservation {
                    paragraph_owner: owner,
                    shaped,
                    page_index,
                    line_ordinal,
                    x,
                    line_bounds: bounds,
                    advance: line_slice.advance,
                })
                .map_err(|_| DisplayValidationError::UnsupportedReferencePaintDomain)?;
        }
        *next_run_id = next_run_id
            .checked_add(1)
            .ok_or(DisplayValidationError::NonDenseGlyphRunId)?;
        register_table_profile_command_font(package, run, &command, used_fonts)?;
        x = x
            .checked_add(line_slice.advance)
            .ok_or(DisplayValidationError::NumericOutOfRange)?;
        if let ShapeSourceSpan::Generated(provenance) = shaped.source() {
            if provenance.buffer_key().generation_kind() == GenerationKind::FootnoteMarker {
                if definition_marker
                    .is_some_and(|(marker_owner, _)| marker_owner == shaped.site_owner())
                {
                    if !start_on_right
                        && definition_marker_visual_bounds
                            .is_some_and(|(_, last, _)| visual_index == last)
                    {
                        let gap = definition_marker
                            .map(|(_, gap)| gap.get())
                            .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
                        x = x
                            .checked_add(gap)
                            .ok_or(DisplayValidationError::NumericOutOfRange)?;
                    }
                } else if body_marker_owners.insert(shaped.site_owner()) {
                    body_markers.push((
                        u32::try_from(commands.len())
                            .map_err(|_| DisplayValidationError::NumericOutOfRange)?,
                        shaped.site_owner(),
                    ));
                }
            }
        }
        commands.push(command);
    }
    Ok(u32::from(definition_marker_visual_bounds.is_some()))
}

fn footnote_marker_visual_bounds(
    marker_clusters: impl IntoIterator<Item = bool>,
    gap: PositiveLength,
) -> Option<(usize, usize, PositiveLength)> {
    let mut first = None;
    let mut last = None;
    for (visual_index, is_marker) in marker_clusters.into_iter().enumerate() {
        if is_marker {
            first.get_or_insert(visual_index);
            last = Some(visual_index);
        }
    }
    Some((first?, last?, gap))
}

fn shaped_slice_is_footnote_marker(shaped: ShapedSlice, marker_owner: NodeId) -> bool {
    shaped.site_owner() == marker_owner
        && matches!(
            shaped.source(),
            ShapeSourceSpan::Generated(provenance)
                if provenance.buffer_key().generation_kind() == GenerationKind::FootnoteMarker
        )
}

fn paragraph_shaped_slices(
    registry: &ValidatedParagraphItemRegistry,
    owner: NodeId,
    start: u32,
    end: u32,
) -> Result<Vec<LinePaintSlice>, DisplayValidationError> {
    let item_count = registry
        .item_count(owner)
        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
    if start >= end || end > item_count {
        return Err(DisplayValidationError::UnsupportedReferencePaintDomain);
    }
    let items = registry
        .items(owner)
        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
    if items.is_empty() && item_count == 1 && start == 0 && end == 1 {
        return Ok(Vec::new());
    }
    let items = items
        .get(start as usize..end as usize)
        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
    let mut output = Vec::new();
    for item in items {
        match item {
            ParagraphItem::Box { width, shaped, .. } => {
                output.push(LinePaintSlice {
                    class: LineBidiClass::Other,
                    shaped: *shaped,
                    advance: width.get(),
                    stretch: Length::ZERO,
                    shrink: Length::ZERO,
                    priority: 0,
                });
            }
            ParagraphItem::Glue { .. } if item_is_line_terminal(item, items) => {}
            ParagraphItem::Glue {
                natural,
                stretch,
                shrink,
                priority,
                shaped,
                ..
            } => {
                output.push(LinePaintSlice {
                    class: LineBidiClass::Whitespace,
                    shaped: *shaped,
                    advance: natural.get(),
                    stretch: stretch.get(),
                    shrink: shrink.get(),
                    priority: *priority,
                });
            }
            ParagraphItem::Discretionary { pre_break, .. }
                if pre_break.shaped.is_some() && item_is_line_terminal(item, items) =>
            {
                let shaped = pre_break
                    .shaped
                    .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
                output.push(LinePaintSlice {
                    class: LineBidiClass::Other,
                    shaped,
                    advance: shaped.derived_width().get(),
                    stretch: Length::ZERO,
                    shrink: Length::ZERO,
                    priority: 0,
                });
            }
            ParagraphItem::Penalty { .. }
            | ParagraphItem::Discretionary { .. }
            | ParagraphItem::InlineObject { .. } => {}
        }
    }
    Ok(output)
}

fn append_display_source_spans(
    slices: Vec<LinePaintSlice>,
    parsed: &mut Vec<TextSpan>,
    generated: &mut Vec<GeneratedTextSpan>,
) {
    for slice in slices {
        match slice.shaped.source() {
            ShapeSourceSpan::Parsed(span) => parsed.push(span),
            ShapeSourceSpan::Generated(provenance) => generated.push(provenance.text_span()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LinePaintSlice {
    class: LineBidiClass,
    shaped: ShapedSlice,
    advance: Length,
    stretch: Length,
    shrink: Length,
    priority: u8,
}

fn reference_final_line_reshape(
    registry: &ValidatedParagraphItemRegistry,
    owner: typaxis_core::NodeId,
    logical: Vec<LinePaintSlice>,
    after_l1: &LineLevelsAfterL1,
) -> Result<Vec<LinePaintSlice>, DisplayValidationError> {
    if logical.len() != after_l1.logical_levels().len() {
        return Err(DisplayValidationError::UnsupportedReferencePaintDomain);
    }
    let runs = registry
        .runs(owner)
        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
    for (slice, level) in logical.iter().zip(after_l1.logical_levels()) {
        let run = runs
            .get(slice.shaped.paragraph_run_index().get() as usize)
            .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
        if run.epoch() != slice.shaped.epoch()
            || run.site_owner() != slice.shaped.site_owner()
            || run.style_owner() != slice.shaped.style_owner()
            || slice.shaped.bidi_level().get() < level.get()
        {
            return Err(DisplayValidationError::UnsupportedReferencePaintDomain);
        }
        let glyph_start = usize::try_from(slice.shaped.glyph_start())
            .map_err(|_| DisplayValidationError::InvalidClusterGlyphRange)?;
        let glyph_end = usize::try_from(slice.shaped.glyph_end())
            .map_err(|_| DisplayValidationError::InvalidClusterGlyphRange)?;
        if glyph_start >= glyph_end || run.glyphs().get(glyph_start..glyph_end).is_none() {
            return Err(DisplayValidationError::InvalidClusterGlyphRange);
        }
    }
    Ok(logical)
}

fn justify_reference_line(
    registry: &ValidatedParagraphItemRegistry,
    fragment: &typaxis_pagination::PlacedFragment,
    logical: &mut [LinePaintSlice],
) -> Result<(), DisplayValidationError> {
    let item_count = registry
        .item_count(fragment.owner)
        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
    let end = if fragment.end.owner() == fragment.owner {
        fragment.end.owner_local_boundary()
    } else {
        item_count
    };
    justify_reference_item_range(
        registry,
        fragment.owner,
        end,
        fragment.bounds.width(),
        logical,
    )
}

fn justify_reference_item_range(
    registry: &ValidatedParagraphItemRegistry,
    owner: NodeId,
    end: u32,
    inline_size: PositiveLength,
    logical: &mut [LinePaintSlice],
) -> Result<(), DisplayValidationError> {
    let item_count = registry
        .item_count(owner)
        .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?;
    // The terminal paragraph line runs through the justification stage with
    // an explicit no-adjust policy. Other lines consume available Glue by
    // ascending priority and retain deterministic logical-order rounding.
    if end == item_count {
        return Ok(());
    }
    let natural = logical
        .iter()
        .try_fold(Length::ZERO, |total, slice| {
            total.checked_add(slice.advance)
        })
        .ok_or(DisplayValidationError::NumericOutOfRange)?;
    let delta = inline_size
        .get()
        .checked_sub(natural)
        .ok_or(DisplayValidationError::NumericOutOfRange)?;
    distribute_justification(logical, delta)
}

fn distribute_justification(
    logical: &mut [LinePaintSlice],
    delta: Length,
) -> Result<(), DisplayValidationError> {
    if delta == Length::ZERO {
        return Ok(());
    }
    let expanding = delta.raw() > 0;
    let mut remaining = i128::from(delta.raw()).abs();
    let priorities: std::collections::BTreeSet<_> = logical
        .iter()
        .filter(|slice| {
            if expanding {
                slice.stretch != Length::ZERO
            } else {
                slice.shrink != Length::ZERO
            }
        })
        .map(|slice| slice.priority)
        .collect();
    for priority in priorities {
        if remaining == 0 {
            break;
        }
        let mut capacity = logical
            .iter()
            .filter(|slice| slice.priority == priority)
            .try_fold(0i128, |total, slice| {
                total.checked_add(i128::from(if expanding {
                    slice.stretch.raw()
                } else {
                    slice.shrink.raw()
                }))
            })
            .ok_or(DisplayValidationError::NumericOutOfRange)?;
        let mut allocation = remaining.min(capacity);
        remaining -= allocation;
        for slice in logical
            .iter_mut()
            .filter(|slice| slice.priority == priority)
        {
            let slice_capacity = i128::from(if expanding {
                slice.stretch.raw()
            } else {
                slice.shrink.raw()
            });
            if slice_capacity == 0 {
                continue;
            }
            let share = allocation
                .checked_mul(slice_capacity)
                .and_then(|value| value.checked_div(capacity))
                .ok_or(DisplayValidationError::NumericOutOfRange)?;
            let share =
                i64::try_from(share).map_err(|_| DisplayValidationError::NumericOutOfRange)?;
            let signed = Length::from_raw(if expanding { share } else { -share })
                .ok_or(DisplayValidationError::NumericOutOfRange)?;
            slice.advance = slice
                .advance
                .checked_add(signed)
                .filter(|advance| advance.raw() >= 0)
                .ok_or(DisplayValidationError::NumericOutOfRange)?;
            allocation -= i128::from(share);
            capacity -= slice_capacity;
        }
        if allocation != 0 {
            return Err(DisplayValidationError::NumericOutOfRange);
        }
    }
    Ok(())
}

fn item_is_line_terminal(item: &ParagraphItem, line_items: &[ParagraphItem]) -> bool {
    core::ptr::eq(item, line_items.last().unwrap_or(item))
}

fn paint_shaped_slice(
    package: &ValidatedParsedPackage,
    text_map: &DisplayTextMap,
    run: &ValidatedGlyphRun,
    shaped: ShapedSlice,
    bidi_level: BidiLevel,
    run_id: DisplayGlyphRunId,
    origin: Point,
) -> Result<DisplayCommand, DisplayValidationError> {
    if shaped.run_id() != run.run_id()
        || shaped.epoch() != run.epoch()
        || shaped.site_owner() != run.site_owner()
        || shaped.style_owner() != run.style_owner()
    {
        return Err(DisplayValidationError::UnsupportedReferencePaintDomain);
    }
    let start = shaped.glyph_start() as usize;
    let end = shaped.glyph_end() as usize;
    let glyphs = run
        .glyphs()
        .get(start..end)
        .ok_or(DisplayValidationError::InvalidClusterGlyphRange)?;
    if glyphs.is_empty() {
        return Err(DisplayValidationError::EmptyCluster);
    }
    let (text_span, extraction) = match shaped.source() {
        ShapeSourceSpan::Parsed(span) => {
            let text_span = text_map
                .map_parsed(span)
                .map_err(|_| DisplayValidationError::SelectedTextMapMismatch)?;
            (text_span, ClusterExtraction::Unicode { text_span })
        }
        ShapeSourceSpan::Generated(provenance) => {
            let generated = text_map
                .map_generated(provenance.text_span())
                .map_err(|_| DisplayValidationError::SelectedTextMapMismatch)?;
            let start = generated.range().start_byte();
            let artifact_span = DisplayTextSpan::new(generated.text_id(), start, start)
                .ok_or(DisplayValidationError::SelectedTextMapMismatch)?;
            (artifact_span, ClusterExtraction::Artifact)
        }
    };
    let font_size = match package
        .cascade_style(shaped.style_owner())
        .map_err(|_| DisplayValidationError::UnsupportedReferencePaintDomain)?
        .computed()
        .properties()
        .get("font_size")
    {
        Some(StyleValue::Length(length)) => PositiveLength::new(*length)
            .ok_or(DisplayValidationError::UnsupportedReferencePaintDomain)?,
        _ => return Err(DisplayValidationError::UnsupportedReferencePaintDomain),
    };
    Ok(DisplayCommand::DrawGlyphRun {
        run_id,
        font_instance_id: run.font(),
        text_span,
        origin,
        font_size,
        bidi_level,
        fill: Paint::Gray(0),
        glyphs: glyphs
            .iter()
            .map(|glyph| DisplayGlyph {
                original_gid: glyph.original_gid,
                advance_x: glyph.advance_x,
                advance_y: glyph.advance_y,
                offset_x: glyph.offset_x,
                offset_y: glyph.offset_y,
            })
            .collect(),
        clusters: vec![DisplayCluster {
            logical_ordinal: 0,
            glyph_start: 0,
            glyph_end: u32::try_from(glyphs.len())
                .map_err(|_| DisplayValidationError::InvalidClusterGlyphRange)?,
            extraction,
        }],
    })
}

fn command_font(command: &DisplayCommand) -> Result<FontInstanceId, DisplayValidationError> {
    match command {
        DisplayCommand::DrawGlyphRun {
            font_instance_id, ..
        } => Ok(*font_instance_id),
        _ => Err(DisplayValidationError::UnsupportedReferencePaintDomain),
    }
}

fn reference_paint_domain_is_supported(
    package: &ValidatedParsedPackage,
    selected: &PaginationResult,
) -> bool {
    let parsed = package.package();
    parsed.document.footnotes.is_empty()
        && parsed.text_store.buffers().is_empty()
        && package.document_nodes().generated_sites().len() == 0
        && parsed.document.blocks.iter().all(|block| {
            matches!(
                block,
                Block::Paragraph { children, .. }
                    if children.iter().all(|inline| matches!(inline, Inline::Anchor { .. }))
            )
        })
        && selected
            .selected_pass()
            .generated_text()
            .buffers()
            .is_empty()
        && selected.selected_pages().iter().all(|page| {
            page.footnote_ids.is_empty()
                && page.float_decisions.is_empty()
                && page.column_decisions.is_empty()
                && page.resolved_references.is_empty()
        })
}

/// Capability reserved for the in-crate layout-to-paint implementation.
/// External callers may pass a trusted Display onward but cannot stamp
/// arbitrary commands, destinations, or pages as paint output.
pub struct DisplayListBuilderOwner {
    _private: (),
}
struct FootnoteDisplayIssue {
    font_instances: Vec<DisplayFontInstance>,
    destinations: Vec<NamedDestination>,
    pages: Vec<DisplayPage>,
    selected_page_geometry: Vec<ValidatedDisplayPageGeometry>,
}

impl DisplayListBuilderOwner {
    #[allow(dead_code)] // reserved for the in-crate reference paint builder
    fn new() -> Self {
        Self { _private: () }
    }
}

fn validate_length(value: Length) -> Result<(), DisplayValidationError> {
    if value.raw() < -JSON_SAFE_INTEGER_MAX || value.raw() > JSON_SAFE_INTEGER_MAX {
        Err(DisplayValidationError::NumericOutOfRange)
    } else {
        Ok(())
    }
}

fn validate_point_numbers(point: Point) -> Result<(), DisplayValidationError> {
    validate_length(point.x)?;
    validate_length(point.y)
}

fn validate_rect_numbers(rect: Rect) -> Result<(), DisplayValidationError> {
    validate_length(rect.x())?;
    validate_length(rect.y())?;
    validate_length(rect.width().get())?;
    validate_length(rect.height().get())
}

fn validate_path_numbers(path: &Path) -> Result<(), DisplayValidationError> {
    for verb in path.verbs() {
        match verb {
            PathVerb::MoveTo(point) | PathVerb::LineTo(point) => validate_point_numbers(*point)?,
            PathVerb::CurveTo(first, second, third) => {
                validate_point_numbers(*first)?;
                validate_point_numbers(*second)?;
                validate_point_numbers(*third)?;
            }
            PathVerb::Close => {}
        }
    }
    Ok(())
}

fn validate_command_numbers(command: &DisplayCommand) -> Result<(), DisplayValidationError> {
    match command {
        DisplayCommand::Save | DisplayCommand::Restore => Ok(()),
        DisplayCommand::ConcatTransform { matrix } => {
            validate_length(matrix.e)?;
            validate_length(matrix.f)
        }
        DisplayCommand::ClipPath { path, .. } | DisplayCommand::FillPath { path, .. } => {
            validate_path_numbers(path)
        }
        DisplayCommand::StrokePath { path, stroke, .. } => {
            validate_path_numbers(path)?;
            validate_length(stroke.width.get())?;
            validate_length(stroke.dash.phase().get())?;
            if !stroke.dash.array().is_empty()
                && stroke
                    .dash
                    .array()
                    .iter()
                    .all(|value| value.get() == Length::ZERO)
            {
                return Err(DisplayValidationError::InvalidDashPattern);
            }
            for value in stroke.dash.array() {
                validate_length(value.get())?;
            }
            Ok(())
        }
        DisplayCommand::DrawGlyphRun {
            origin,
            font_size,
            glyphs,
            ..
        } => {
            validate_point_numbers(*origin)?;
            validate_length(font_size.get())?;
            for glyph in glyphs {
                validate_length(glyph.advance_x)?;
                validate_length(glyph.advance_y)?;
                validate_length(glyph.offset_x)?;
                validate_length(glyph.offset_y)?;
            }
            Ok(())
        }
        DisplayCommand::DrawImage { rect, .. } => validate_rect_numbers(*rect),
    }
}

fn validate_destination_numbers(view: &DestinationView) -> Result<(), DisplayValidationError> {
    match view {
        DestinationView::Xyz { point } => validate_point_numbers(*point),
        DestinationView::FitWidth { top: Some(top) } => validate_length(*top),
        DestinationView::FitPage | DestinationView::FitWidth { top: None } => Ok(()),
    }
}

fn destinations_from_selected_pagination(
    selected: &PaginationResult,
) -> Result<Vec<NamedDestination>, DisplayValidationError> {
    let mut destinations = Vec::new();
    for anchor in selected.selected_anchors() {
        let page = selected
            .selected_pages()
            .get(
                usize::try_from(anchor.page_index())
                    .map_err(|_| DisplayValidationError::SelectedDestinationMismatch)?,
            )
            .ok_or(DisplayValidationError::SelectedDestinationMismatch)?;
        let frame = page
            .frames
            .iter()
            .find(|frame| {
                frame.kind == anchor.frame_kind() && frame.column_index == anchor.column_index()
            })
            .ok_or(DisplayValidationError::SelectedDestinationMismatch)?;
        let point = anchor
            .position_on_page(frame.bounds)
            .ok_or(DisplayValidationError::SelectedDestinationMismatch)?;
        destinations.push(NamedDestination {
            anchor_id: anchor.anchor_id().clone(),
            page_index: anchor.page_index(),
            view: DestinationView::Xyz { point },
        });
    }
    destinations.sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
    if destinations
        .windows(2)
        .any(|pair| pair[0].anchor_id == pair[1].anchor_id)
    {
        return Err(DisplayValidationError::SelectedDestinationMismatch);
    }
    Ok(destinations)
}

fn destination_within_page(view: &DestinationView, page: &DisplayPage) -> bool {
    match view {
        DestinationView::Xyz { point } => {
            point.x.raw() >= 0
                && point.y.raw() >= 0
                && point.x.raw() <= page.width.get().raw()
                && point.y.raw() <= page.height.get().raw()
        }
        DestinationView::FitWidth { top: Some(top) } => {
            top.raw() >= 0 && top.raw() <= page.height.get().raw()
        }
        DestinationView::FitPage | DestinationView::FitWidth { top: None } => true,
    }
}

fn validate_display_span(
    buffers: &[DisplayTextBuffer],
    span: DisplayTextSpan,
) -> Result<(), DisplayValidationError> {
    let buffer = buffers
        .get(span.text_id().get() as usize)
        .ok_or(DisplayValidationError::UnknownTextBuffer)?;
    let start = span.range().start_byte().get() as usize;
    let end = span.range().end_byte().get() as usize;
    if end > buffer.utf8.len()
        || !buffer.utf8.is_char_boundary(start)
        || !buffer.utf8.is_char_boundary(end)
    {
        return Err(DisplayValidationError::TextSpanOutOfBounds);
    }
    Ok(())
}

fn validate_selected_text_buffers(
    buffers: &[DisplayTextBuffer],
    selected_generated: &GeneratedTextStore,
    parsed_store: Option<&TextStore>,
) -> Result<(), DisplayValidationError> {
    for buffer in buffers {
        match buffer.origin {
            DisplayTextOrigin::Parsed(id) => {
                if let Some(parsed_store) = parsed_store {
                    let Some(selected_buffer) = parsed_store.get(id) else {
                        return Err(DisplayValidationError::SelectedParsedTextMismatch);
                    };
                    if selected_buffer.text() != buffer.utf8 {
                        return Err(DisplayValidationError::SelectedParsedTextMismatch);
                    }
                }
            }
            DisplayTextOrigin::Generated(key) => {
                let Some(selected_buffer) = selected_generated
                    .buffers()
                    .iter()
                    .find(|candidate| candidate.key() == key)
                else {
                    return Err(DisplayValidationError::SelectedGeneratedTextMismatch);
                };
                if selected_buffer.utf8() != buffer.utf8 {
                    return Err(DisplayValidationError::SelectedGeneratedTextMismatch);
                }
            }
        }
    }
    Ok(())
}

fn validate_glyph_run_clusters(
    buffers: &[DisplayTextBuffer],
    text_span: DisplayTextSpan,
    glyph_count: usize,
    clusters: &[DisplayCluster],
    used_text_buffers: &mut std::collections::BTreeSet<DisplayTextBufferId>,
) -> Result<(), DisplayValidationError> {
    validate_display_span(buffers, text_span)?;
    used_text_buffers.insert(text_span.text_id());
    let run_range = text_span.range();
    let mut expected_start = run_range.start_byte();
    if glyph_count == 0 || clusters.is_empty() {
        return Err(DisplayValidationError::EmptyCluster);
    }
    let mut covered_glyphs = vec![false; glyph_count];
    for (logical_ordinal, cluster) in clusters.iter().enumerate() {
        if cluster.logical_ordinal
            != u32::try_from(logical_ordinal)
                .map_err(|_| DisplayValidationError::NonDenseLogicalCluster)?
        {
            return Err(DisplayValidationError::NonDenseLogicalCluster);
        }
        let start = cluster.glyph_start as usize;
        let end = cluster.glyph_end as usize;
        if start >= end
            || end > glyph_count
            || covered_glyphs[start..end].iter().any(|covered| *covered)
        {
            return Err(DisplayValidationError::InvalidClusterGlyphRange);
        }
        covered_glyphs[start..end].fill(true);
        if let ClusterExtraction::Unicode {
            text_span: cluster_span,
        } = cluster.extraction
        {
            if cluster_span.range().is_empty() {
                return Err(DisplayValidationError::EmptyUnicodeCluster);
            }
            validate_display_span(buffers, cluster_span)?;
            used_text_buffers.insert(cluster_span.text_id());
            if cluster_span.text_id() != text_span.text_id()
                || cluster_span.range().start_byte() != expected_start
                || cluster_span.range().end_byte() > run_range.end_byte()
            {
                return Err(DisplayValidationError::ClusterCoverage);
            }
            expected_start = cluster_span.range().end_byte();
        }
    }
    if expected_start != run_range.end_byte() {
        return Err(DisplayValidationError::ClusterCoverage);
    }
    if covered_glyphs.iter().any(|covered| !covered) {
        return Err(DisplayValidationError::InvalidClusterGlyphRange);
    }
    Ok(())
}

fn rect_within_page(rect: Rect, page: &DisplayPage) -> bool {
    let x = rect.x().raw();
    let y = rect.y().raw();
    x >= 0
        && y >= 0
        && x.checked_add(rect.width().get().raw())
            .is_some_and(|end| end <= page.width.get().raw())
        && y.checked_add(rect.height().get().raw())
            .is_some_and(|end| end <= page.height.get().raw())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GeneratedDisplayBuffer {
    source_id: GeneratedTextBufferId,
    key: GeneratedBufferKey,
    utf8: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextRemapError {
    DuplicateParsedBuffer,
    DuplicateGeneratedBuffer,
    DuplicateGeneratedKey,
    TooManyBuffers,
    UnknownBuffer,
    SpanOutOfBounds,
    WrongGeneratedEpoch,
    WrongSelectedPackage,
}

/// Dense artifact-local text IDs: parsed buffers first by ID, then generated
/// buffers by canonical GeneratedBufferKey.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayTextMap {
    source_layout: DisplaySourceLayout,
    contents: DisplayTextMapContents,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DisplayTextMapContents {
    buffers: Vec<DisplayTextBuffer>,
    parsed: std::collections::BTreeMap<TextBufferId, DisplayTextBufferId>,
    generated: std::collections::BTreeMap<GeneratedTextBufferId, DisplayTextBufferId>,
}
impl DisplayTextMap {
    pub fn from_selected_spans(
        package: &ValidatedParsedPackage,
        selected: &PaginationResult,
        parsed_spans: &[TextSpan],
        generated_spans: &[GeneratedTextSpan],
    ) -> Result<Self, TextRemapError> {
        let epoch = selected.selected_pass().fingerprint_record().layout_epoch();
        if epoch.document() != package.epoch_identity().document()
            || epoch.style() != package.epoch_identity().style()
        {
            return Err(TextRemapError::WrongSelectedPackage);
        }
        let generated_store = selected.selected_pass().generated_text();
        let contents = DisplayTextMapContents::from_stores(
            &package.package().text_store,
            generated_store,
            generated_store.reference_fingerprint(),
            parsed_spans,
            generated_spans,
        )?;
        Ok(Self {
            source_layout: DisplaySourceLayout::from_selected_pagination(selected),
            contents,
        })
    }

    pub fn buffers(&self) -> &[DisplayTextBuffer] {
        self.contents.buffers()
    }
    pub fn map_parsed(&self, span: TextSpan) -> Result<DisplayTextSpan, TextRemapError> {
        self.contents.map_parsed(span)
    }
    pub fn map_generated(
        &self,
        span: GeneratedTextSpan,
    ) -> Result<DisplayTextSpan, TextRemapError> {
        self.contents.map_generated(span)
    }
}

impl DisplayTextMapContents {
    fn from_stores(
        parsed_store: &TextStore,
        generated_store: &GeneratedTextStore,
        reference_fingerprint: ReferenceFingerprint,
        parsed_spans: &[TextSpan],
        generated_spans: &[GeneratedTextSpan],
    ) -> Result<Self, TextRemapError> {
        if generated_store.reference_fingerprint() != reference_fingerprint {
            return Err(TextRemapError::WrongGeneratedEpoch);
        }
        let parsed_ids: std::collections::BTreeSet<_> =
            parsed_spans.iter().map(|span| span.text_id()).collect();
        let generated_ids: std::collections::BTreeSet<_> =
            generated_spans.iter().map(|span| span.text_id()).collect();
        let mut parsed = Vec::with_capacity(parsed_ids.len());
        for id in parsed_ids {
            let buffer = parsed_store.get(id).ok_or(TextRemapError::UnknownBuffer)?;
            parsed.push((id, buffer.text().to_owned()));
        }
        let mut generated = Vec::with_capacity(generated_ids.len());
        for id in generated_ids {
            let buffer = generated_store
                .get(id)
                .ok_or(TextRemapError::UnknownBuffer)?;
            generated.push(GeneratedDisplayBuffer {
                source_id: id,
                key: buffer.key(),
                utf8: buffer.utf8().to_owned(),
            });
        }
        let mapping = Self::from_buffers(parsed, generated)?;
        for span in parsed_spans {
            mapping.map_parsed(*span)?;
        }
        for span in generated_spans {
            mapping.map_generated(*span)?;
        }
        Ok(mapping)
    }

    fn from_buffers(
        mut parsed: Vec<(TextBufferId, String)>,
        mut generated: Vec<GeneratedDisplayBuffer>,
    ) -> Result<Self, TextRemapError> {
        parsed.sort_by_key(|(id, _)| *id);
        generated.sort_by_key(|buffer| buffer.key);
        let mut parsed_map = std::collections::BTreeMap::new();
        let mut generated_map = std::collections::BTreeMap::new();
        let mut keys = std::collections::BTreeSet::new();
        let mut buffers = Vec::with_capacity(parsed.len() + generated.len());
        for (source_id, utf8) in parsed {
            let dense = DisplayTextBufferId::new(
                u32::try_from(buffers.len()).map_err(|_| TextRemapError::TooManyBuffers)?,
            );
            if parsed_map.insert(source_id, dense).is_some() {
                return Err(TextRemapError::DuplicateParsedBuffer);
            }
            buffers.push(DisplayTextBuffer {
                text_id: dense,
                origin: DisplayTextOrigin::Parsed(source_id),
                utf8,
            });
        }
        for buffer in generated {
            if !keys.insert(buffer.key) {
                return Err(TextRemapError::DuplicateGeneratedKey);
            }
            let dense = DisplayTextBufferId::new(
                u32::try_from(buffers.len()).map_err(|_| TextRemapError::TooManyBuffers)?,
            );
            if generated_map.insert(buffer.source_id, dense).is_some() {
                return Err(TextRemapError::DuplicateGeneratedBuffer);
            }
            buffers.push(DisplayTextBuffer {
                text_id: dense,
                origin: DisplayTextOrigin::Generated(buffer.key),
                utf8: buffer.utf8,
            });
        }
        Ok(Self {
            buffers,
            parsed: parsed_map,
            generated: generated_map,
        })
    }
    fn buffers(&self) -> &[DisplayTextBuffer] {
        &self.buffers
    }
    fn map_parsed(&self, span: TextSpan) -> Result<DisplayTextSpan, TextRemapError> {
        let id = *self
            .parsed
            .get(&span.text_id())
            .ok_or(TextRemapError::UnknownBuffer)?;
        let mapped = DisplayTextSpan::new(id, span.start_byte(), span.end_byte())
            .ok_or(TextRemapError::SpanOutOfBounds)?;
        validate_remapped_span(&self.buffers, mapped)?;
        Ok(mapped)
    }
    fn map_generated(&self, span: GeneratedTextSpan) -> Result<DisplayTextSpan, TextRemapError> {
        let id = *self
            .generated
            .get(&span.text_id())
            .ok_or(TextRemapError::UnknownBuffer)?;
        let mapped = DisplayTextSpan::new(id, span.range().start_byte(), span.range().end_byte())
            .ok_or(TextRemapError::SpanOutOfBounds)?;
        validate_remapped_span(&self.buffers, mapped)?;
        Ok(mapped)
    }
}

impl DisplayListBuilderOwner {
    /// Issues the only publication-trusted Display type after consuming a
    /// package/pagination-bound text map and validating the in-crate painter's
    /// complete payload.
    ///
    /// The owner constructor is not available to external callers:
    ///
    /// ```compile_fail
    /// # use typaxis_display_list::DisplayListBuilderOwner;
    /// let _ = DisplayListBuilderOwner::new();
    /// ```
    pub fn issue(
        &self,
        selected: &PaginationResult,
        text_map: DisplayTextMap,
        font_instances: Vec<DisplayFontInstance>,
        pages: Vec<DisplayPage>,
        config: &EffectiveConfig,
    ) -> Result<ValidatedDisplayDocument, DisplayValidationError> {
        let source_layout = text_map.source_layout;
        if !source_layout.matches_selected(selected) {
            return Err(DisplayValidationError::SelectedTextMapMismatch);
        }
        let document = DisplayDocument {
            source_layout,
            text_buffers: text_map.contents.buffers,
            font_instances,
            destinations: destinations_from_selected_pagination(selected)?,
            pages,
        };
        let structural = StructurallyValidatedDisplayDocument::from_verified_text_map(
            document, selected, config,
        )?;
        Ok(ValidatedDisplayDocument { structural })
    }

    fn issue_footnote(
        &self,
        selected: &PaginationResult,
        text_map: DisplayTextMap,
        issue: FootnoteDisplayIssue,
        config: &EffectiveConfig,
    ) -> Result<ValidatedDisplayDocument, DisplayValidationError> {
        let FootnoteDisplayIssue {
            font_instances,
            destinations,
            pages,
            selected_page_geometry,
        } = issue;
        let source_layout = text_map.source_layout;
        if !source_layout.matches_selected(selected) {
            return Err(DisplayValidationError::SelectedTextMapMismatch);
        }
        let document = DisplayDocument {
            source_layout,
            text_buffers: text_map.contents.buffers,
            font_instances,
            destinations: destinations.clone(),
            pages,
        };
        let structural = StructurallyValidatedDisplayDocument::from_verified_footnote_profile(
            document,
            selected,
            config,
            &destinations,
            selected_page_geometry,
        )?;
        Ok(ValidatedDisplayDocument { structural })
    }
}

fn validate_remapped_span(
    buffers: &[DisplayTextBuffer],
    span: DisplayTextSpan,
) -> Result<(), TextRemapError> {
    let buffer = buffers
        .get(span.text_id().get() as usize)
        .ok_or(TextRemapError::UnknownBuffer)?;
    let start = span.range().start_byte().get() as usize;
    let end = span.range().end_byte().get() as usize;
    if end > buffer.utf8.len()
        || !buffer.utf8.is_char_boundary(start)
        || !buffer.utf8.is_char_boundary(end)
    {
        return Err(TextRemapError::SpanOutOfBounds);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{
        ConfigResourceRoot, EffectiveDataVersions, GeneratedTextBufferId, GeneratedTextSpan,
        GenerationKind, MasterId, NodeId, PdfStreamCompression, PortablePath, ResourceLimits,
        SourceId, SourceSpan, TextBufferId, Utf8ByteOffset, ValidatedResourceLimits,
    };
    use typaxis_document::{
        Block, Document, DocumentNodeKind, Inline, ReferenceFormat, ValidatedDocumentNodeIndex,
    };
    use typaxis_layout::{
        CanonicalFlowIrBuilder, Continuation, CursorPosition, DiscoveredAnchor, FlowCursor,
        FlowTree, FragmentDraft, FragmentError, FragmentRequest, FragmentResult,
        FragmentWorkBudget, Fragmenter, LayoutEpoch, PageContext, ResolvedPageSelection,
    };
    use typaxis_linebreak::ValidatedParagraphItemRegistry;
    use typaxis_pagination::{
        ColumnDecision, ConvergenceStatus, InitialPaginationState, LayoutPass, LayoutPassInput,
        PageFrameKind, PageFramePlan, PagePlan, PaginationInput, PaginationOptions,
        PaginationOutcome, PaginationWorkBudget, ReferencePaginator,
    };
    use typaxis_resource_admission::AdmittedResourceResolver;
    use typaxis_syntax::{
        PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
        ValidatedParsedPackage,
    };
    use typaxis_text::{GeneratedBufferDraft, TextBuffer};

    fn config() -> EffectiveConfig {
        EffectiveConfig::new(
            false,
            PdfStreamCompression::Flate,
            vec![ConfigResourceRoot::ProjectRoot],
            ["http", "https", "mailto"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
            ResourceLimits::default(),
        )
        .unwrap()
    }

    fn table_paint_record_fixture() -> TablePaintCellObservation {
        TablePaintCellObservation {
            kind: TablePaintOccurrenceKind::Header,
            page_index: 1,
            fragment_id: 9,
            source_fragment_id: Some(3),
            repetition_index: Some(1),
            row_node_id: NodeId::new(2),
            logical_row_ordinal: 0,
            row_fragment_ordinal: 0,
            cell_node_id: NodeId::new(3),
            flow_id: FlowId::new(1),
            column_ordinal: 0,
            colspan: 1,
            rowspan: 1,
            rect: TablePaintRect {
                x: 10,
                y: 20,
                width: 30,
                height: 40,
            },
            content_fragment_start: 0,
            content_fragment_end: 2,
            vertical_offset_before: 0,
            vertical_offset_after: 40,
        }
    }

    #[test]
    fn table_display_closure_rejects_missing_extra_wrong_cell_page_repetition_and_rect() {
        let expected = table_paint_record_fixture();
        assert_eq!(
            validate_table_display_records(std::slice::from_ref(&expected), &[]),
            Err(TableDisplayClosureError::MissingCell)
        );
        assert_eq!(
            validate_table_display_records(
                std::slice::from_ref(&expected),
                &[expected.clone(), expected.clone()],
            ),
            Err(TableDisplayClosureError::ExtraCell)
        );

        let mut wrong = expected.clone();
        wrong.page_index = 2;
        assert_eq!(
            validate_table_display_records(std::slice::from_ref(&expected), &[wrong]),
            Err(TableDisplayClosureError::WrongPage)
        );
        let mut wrong = expected.clone();
        wrong.repetition_index = Some(2);
        assert_eq!(
            validate_table_display_records(std::slice::from_ref(&expected), &[wrong]),
            Err(TableDisplayClosureError::WrongRepetition)
        );
        let mut wrong = expected.clone();
        wrong.cell_node_id = NodeId::new(4);
        assert_eq!(
            validate_table_display_records(std::slice::from_ref(&expected), &[wrong]),
            Err(TableDisplayClosureError::WrongCell)
        );
        let mut wrong = expected.clone();
        wrong.rect.x += 1;
        assert_eq!(
            validate_table_display_records(std::slice::from_ref(&expected), &[wrong]),
            Err(TableDisplayClosureError::WrongRectangle)
        );
        let mut wrong = expected.clone();
        wrong.content_fragment_end += 1;
        assert_eq!(
            validate_table_display_records(std::slice::from_ref(&expected), &[wrong]),
            Err(TableDisplayClosureError::WrongContentRange)
        );
        let mut wrong = expected.clone();
        wrong.fragment_id += 1;
        assert_eq!(
            validate_table_display_records(std::slice::from_ref(&expected), &[wrong]),
            Err(TableDisplayClosureError::NonCanonicalOrder)
        );
    }

    #[test]
    fn table_display_closure_enforces_fixed_zero_decoration_policy() {
        assert_eq!(reject_table_decorations(&[]), Ok(()));
        for decoration in [
            TableDecorationObservation::Background,
            TableDecorationObservation::Border,
            TableDecorationObservation::BorderSpacing,
        ] {
            assert_eq!(
                reject_table_decorations(&[decoration]),
                Err(TableDisplayClosureError::DecorationForbidden)
            );
        }
    }

    #[test]
    fn footnote_definition_marker_clusters_form_one_site_and_one_gap() {
        let gap = PositiveLength::new(Length::from_raw(65_536).unwrap()).unwrap();
        assert_eq!(
            footnote_marker_visual_bounds([false, true, true, false], gap),
            Some((1, 2, gap))
        );
        assert_eq!(
            footnote_marker_visual_bounds([false, false, false], gap),
            None
        );
    }

    #[test]
    fn machine_list_display_preserves_generated_marker_usage_and_fragment_binding() {
        let display = StagingMachineListDisplay::list_pdf_test_fixture();
        assert_eq!(display.lists().len(), 1);
        assert_eq!(display.items().len(), 1);
        let item = &display.items()[0];
        assert_eq!(item.marker_utf8(), "1.");
        assert_eq!(item.marker_key().owner(), NodeId::new(2));
        assert_eq!(item.marker_fragment_id(), item.first_line_fragment_id());
        assert_eq!(item.item_flow_id(), 1);
        assert!(display
            .canonical_jcs()
            .contains("\"generation_kind\":\"list_marker\""));
    }

    #[test]
    fn forced_page_break_display_has_empty_exact_paint_closure() {
        assert_eq!(validate_staging_forced_page_break_paint_owners(&[]), Ok(()));
        assert_eq!(
            validate_staging_forced_page_break_paint_owners(&[NodeId::new(7)]),
            Err(StagingForcedPageBreakDisplayError::ExtraBreakPaint(
                NodeId::new(7)
            ))
        );
    }

    #[test]
    fn machine_list_display_rejects_missing_extra_and_wrong_item_closure() {
        let expected = [2, 5, 7];
        assert_eq!(
            validate_staging_machine_list_item_order(&expected, &[2, 5]),
            Err(StagingMachineListDisplayError::MissingItem(7))
        );
        assert_eq!(
            validate_staging_machine_list_item_order(&expected, &[2, 5, 7, 9]),
            Err(StagingMachineListDisplayError::ExtraItem(9))
        );
        assert_eq!(
            validate_staging_machine_list_item_order(&expected, &[2, 6, 7]),
            Err(StagingMachineListDisplayError::WrongItem {
                expected: 5,
                actual: 6,
            })
        );
    }

    fn reference_body(package: &ValidatedParsedPackage) -> Rect {
        package.package().page_masters.masters[0].body
    }

    fn pagination_fixture(seed: u8) -> (ValidatedParsedPackage, PaginationResult) {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new(format!("input-{seed}.tsf")).unwrap(),
            text: String::new(),
        };
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let package = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        );
        let ParseOutcome::Parsed { package, .. } = package else {
            panic!("reference package must parse");
        };
        let package = *package;
        let bounds = reference_body(&package);
        let store = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let generated = package.bind_generated_text(&store, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap();
        let flow = FlowTree::empty(&package, epoch).unwrap();
        let initial = InitialPaginationState::new(&flow, &package, &limits).unwrap();
        let package_context = package.pagination_context();
        let mut input = PaginationInput::new(
            initial,
            &package_context,
            PaginationOptions::from_limits(&limits, false),
        )
        .unwrap();
        let pages = vec![PagePlan {
            page_index: 0,
            master_id: MasterId::new("default").unwrap(),
            frames: vec![PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 0,
                bounds,
            }],
            fragments: vec![],
            footnote_ids: vec![],
            float_decisions: vec![],
            column_decisions: vec![],
            resolved_references: vec![],
        }];
        let mut budget = input.take_work_budget().unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let page_selection = ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let page_context = PageContext::select(0, &page_selection, &package_context).unwrap();
        let mut first_permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();
        first_permit
            .begin_page(&page_context, &cursor, &pages[0].frames)
            .unwrap();
        first_permit.finish_page(&pages[0]).unwrap();
        let first_receipt = first_permit.finish(&flow, &pages).unwrap();
        let first = LayoutPass::new(
            first_receipt,
            input.initial_fingerprint(),
            &flow,
            pages.clone(),
            store.clone(),
        )
        .unwrap();
        let transition = first.transition_references(&package, &limits).unwrap();
        let mut second_permit = budget
            .begin_pass(1, LayoutPassInput::transitioned(transition))
            .unwrap();
        second_permit
            .begin_page(&page_context, &cursor, &pages[0].frames)
            .unwrap();
        second_permit.finish_page(&pages[0]).unwrap();
        let second_receipt = second_permit.finish(&flow, &pages).unwrap();
        let second = LayoutPass::new(
            second_receipt,
            first.output_fingerprint(),
            &flow,
            pages,
            store,
        )
        .unwrap();
        let result = PaginationOutcome::new(
            vec![first, second],
            ConvergenceStatus::Converged,
            &input,
            budget.finish(),
        )
        .unwrap()
        .into_result();
        (package, result)
    }

    fn reference_paginator_fixture(
        source_text: &str,
    ) -> (ValidatedParsedPackage, PaginationResult) {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("reference-paint.tsf").unwrap(),
            text: source_text.to_owned(),
        };
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let ParseOutcome::Parsed { package, .. } = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        ) else {
            panic!("reference package must parse");
        };
        let package = *package;
        let generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let bound = package.bind_generated_text(&generated, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(bound, admitted.token()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(&package, epoch).unwrap();
        let mut flow_builder = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        for (node, kind) in package.document_nodes().nodes() {
            if kind == DocumentNodeKind::Paragraph {
                flow_builder.push_paragraph_item(node, 0).unwrap();
            }
        }
        let flow = flow_builder.finish(epoch).unwrap();
        let result = ReferencePaginator::new()
            .paginate(&package, &flow, &limits, false)
            .unwrap()
            .into_result();
        (package, result)
    }

    struct FinalFragmenter {
        fragment: FragmentDraft,
        terminal: FlowCursor,
        anchors: Vec<DiscoveredAnchor>,
    }
    impl Fragmenter for FinalFragmenter {
        fn fragment(
            &self,
            _request: &FragmentRequest<'_>,
            budget: &mut dyn FragmentWorkBudget,
        ) -> Result<FragmentResult, FragmentError> {
            budget.consume_fragments(1)?;
            Ok(FragmentResult {
                fragments: vec![self.fragment.clone()],
                continuation: Continuation::Exhausted(Box::new(self.terminal.clone())),
                discovered_footnotes: vec![],
                discovered_anchors: self.anchors.clone(),
            })
        }
    }

    struct NextCursorFragmenter {
        next: FlowCursor,
    }
    impl Fragmenter for NextCursorFragmenter {
        fn fragment(
            &self,
            _request: &FragmentRequest<'_>,
            _budget: &mut dyn FragmentWorkBudget,
        ) -> Result<FragmentResult, FragmentError> {
            Ok(FragmentResult {
                fragments: vec![],
                continuation: Continuation::More(Box::new(self.next.clone())),
                discovered_footnotes: vec![],
                discovered_anchors: vec![],
            })
        }
    }

    struct ReferencePassRecords {
        anchors: Vec<DiscoveredAnchor>,
        include_column_decision: bool,
    }

    fn materialized_flow_pass(
        budget: &mut PaginationWorkBudget,
        input: LayoutPassInput<'_>,
        package: &ValidatedParsedPackage,
        flow: &FlowTree,
        generated: &GeneratedTextStore,
        frame: Rect,
        records: ReferencePassRecords,
    ) -> LayoutPass {
        let input_fingerprint = input.fingerprint();
        let mut permit = budget.begin_pass(input.state_index().get(), input).unwrap();
        let cursor = FlowCursor::document_start(flow);
        let selection = ResolvedPageSelection::new(flow, &cursor, package).unwrap();
        let page_context =
            PageContext::select(0, &selection, &package.pagination_context()).unwrap();
        let mut page = PagePlan {
            page_index: 0,
            master_id: MasterId::new("default").unwrap(),
            frames: vec![PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 0,
                bounds: frame,
            }],
            fragments: vec![],
            footnote_ids: vec![],
            float_decisions: vec![],
            column_decisions: vec![],
            resolved_references: vec![],
        };
        permit
            .begin_page(&page_context, &cursor, &page.frames)
            .unwrap();
        let next = FlowCursor::at(flow, 1, CursorPosition::ParagraphItem(0)).unwrap();
        let bootstrap_request = FragmentRequest::new(
            flow,
            &cursor,
            frame,
            NonNegativeLength::ZERO,
            page_context.clone(),
        )
        .unwrap();
        permit
            .run_fragmenter(
                &NextCursorFragmenter { next: next.clone() },
                &bootstrap_request,
                PageFrameKind::Body,
                0,
            )
            .unwrap();
        let fragmenter = FinalFragmenter {
            fragment: FragmentDraft::new(
                flow.positions()[1].clone(),
                flow.positions().last().unwrap().clone(),
                frame,
                0,
            )
            .unwrap(),
            terminal: flow.terminal_cursor(),
            anchors: records.anchors,
        };
        let request =
            FragmentRequest::new(flow, &next, frame, NonNegativeLength::ZERO, page_context)
                .unwrap();
        let materialized = permit
            .run_fragmenter(&fragmenter, &request, PageFrameKind::Body, 0)
            .unwrap();
        page.fragments
            .extend_from_slice(materialized.placed_fragments());
        if records.include_column_decision {
            let decision = ColumnDecision {
                container: NodeId::new(0),
                column_index: 0,
                bounds: frame,
            };
            permit.consume_column_candidate(NodeId::new(0)).unwrap();
            permit
                .record_column_decisions(NodeId::new(0), vec![decision.clone()])
                .unwrap();
            page.column_decisions.push(decision);
        }
        permit.finish_page(&page).unwrap();
        let pages = vec![page];
        let receipt = permit.finish(flow, &pages).unwrap();
        LayoutPass::new(receipt, input_fingerprint, flow, pages, generated.clone()).unwrap()
    }

    fn pagination_fixture_with_anchor() -> (ValidatedParsedPackage, PaginationResult) {
        pagination_fixture_with_anchor_records(false)
    }

    fn pagination_fixture_with_anchor_records(
        include_column_decision: bool,
    ) -> (ValidatedParsedPackage, PaginationResult) {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("anchor-input.tsf").unwrap(),
            text: "anchor:chapter\nparagraph".to_owned(),
        };
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let package = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        );
        let ParseOutcome::Parsed { package, .. } = package else {
            panic!("anchor reference package must parse");
        };
        let package = *package;
        let frame = reference_body(&package);
        let generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let bound = package.bind_generated_text(&generated, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(bound, admitted.token()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(&package, epoch).unwrap();
        let mut flow_builder = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(1), 0).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(3), 0).unwrap();
        let flow = flow_builder.finish(epoch).unwrap();
        let initial = InitialPaginationState::new(&flow, &package, &limits).unwrap();
        let package_context = package.pagination_context();
        let mut input = PaginationInput::new(
            initial,
            &package_context,
            PaginationOptions::from_limits(&limits, false),
        )
        .unwrap();
        let mut budget = input.take_work_budget().unwrap();
        let first = materialized_flow_pass(
            &mut budget,
            LayoutPassInput::initial(&input),
            &package,
            &flow,
            &generated,
            frame,
            ReferencePassRecords {
                anchors: vec![DiscoveredAnchor {
                    anchor_id: AnchorId::new("chapter").unwrap(),
                    owner_node: NodeId::new(2),
                    position_in_frame: Point {
                        x: Length::from_raw(1).unwrap(),
                        y: Length::from_raw(2).unwrap(),
                    },
                }],
                include_column_decision,
            },
        );
        let transition = first.transition_references(&package, &limits).unwrap();
        let second = materialized_flow_pass(
            &mut budget,
            LayoutPassInput::transitioned(transition),
            &package,
            &flow,
            &generated,
            frame,
            ReferencePassRecords {
                anchors: vec![DiscoveredAnchor {
                    anchor_id: AnchorId::new("chapter").unwrap(),
                    owner_node: NodeId::new(2),
                    position_in_frame: Point {
                        x: Length::from_raw(1).unwrap(),
                        y: Length::from_raw(2).unwrap(),
                    },
                }],
                include_column_decision,
            },
        );
        let result = PaginationOutcome::new(
            vec![first, second],
            ConvergenceStatus::Converged,
            &input,
            budget.finish(),
        )
        .unwrap()
        .into_result();
        (package, result)
    }

    fn counter_index(count: u32) -> ValidatedDocumentNodeIndex {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let children = (0..count)
            .map(|index| Inline::Reference {
                node_id: NodeId::new(index + 2),
                span,
                target: AnchorId::new(format!("target{index}")).unwrap(),
                format: ReferenceFormat::Number,
            })
            .collect();
        ValidatedDocumentNodeIndex::new(&Document {
            node_id: NodeId::new(0),
            blocks: vec![Block::Paragraph {
                node_id: NodeId::new(1),
                span,
                classes: vec![],
                children,
            }],
            footnotes: vec![],
        })
        .unwrap()
    }
    #[test]
    fn path_rejects_segment_before_move() {
        assert_eq!(
            Path::new(vec![PathVerb::LineTo(Point {
                x: Length::ZERO,
                y: Length::ZERO
            })]),
            Err(PathError::MustStartWithMove)
        );
    }
    #[test]
    fn path_rejects_move_only() {
        assert_eq!(
            Path::new(vec![PathVerb::MoveTo(Point {
                x: Length::ZERO,
                y: Length::ZERO
            })]),
            Err(PathError::NoDrawableSegment),
        );
    }
    #[test]
    fn dash_rejects_all_zero_array() {
        assert_eq!(
            DashPattern::new(vec![NonNegativeLength::ZERO], NonNegativeLength::ZERO),
            Err(DashError::AllZero)
        );
    }

    #[test]
    fn structural_validation_rejects_forged_all_zero_dash() {
        let (package, selected) = pagination_fixture(1);
        let geometry = &selected.selected_page_geometry()[0];
        let one = Length::from_raw(1).unwrap();
        let path = Path::new(vec![
            PathVerb::MoveTo(Point {
                x: Length::ZERO,
                y: Length::ZERO,
            }),
            PathVerb::LineTo(Point {
                x: one,
                y: Length::ZERO,
            }),
        ])
        .unwrap();
        let forged = DashPattern {
            array: vec![NonNegativeLength::ZERO],
            phase: NonNegativeLength::ZERO,
        };
        let document = DisplayDocument::from_untrusted_parts_for_selected_pagination(
            &selected,
            vec![],
            vec![],
            vec![],
            vec![DisplayPage {
                page_index: 0,
                width: geometry.width(),
                height: geometry.height(),
                commands: vec![DisplayCommand::StrokePath {
                    path,
                    paint: Paint::Gray(0),
                    stroke: StrokeStyle {
                        width: PositiveLength::new(one).unwrap(),
                        line_cap: LineCap::Butt,
                        line_join: LineJoin::Miter,
                        miter_limit: PositiveUnitless16_16::ONE,
                        dash: forged,
                    },
                }],
                annotations: vec![],
            }],
        );
        assert_eq!(
            StructurallyValidatedDisplayDocument::new(document, &package, &selected, &config()),
            Err(DisplayValidationError::InvalidDashPattern)
        );
    }

    #[test]
    fn text_buffers_remap_parsed_then_generated_into_dense_artifact_ids() {
        let generated_span = GeneratedTextSpan::new(
            GeneratedTextBufferId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1),
        )
        .unwrap();
        let parsed = TextStore::new(vec![
            TextBuffer::new(TextBufferId::new(0), String::new(), vec![], 0).unwrap(),
            TextBuffer::new(TextBufferId::new(1), String::new(), vec![], 0).unwrap(),
        ])
        .unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let index = counter_index(1);
        let draft = GeneratedBufferDraft::new(
            &index,
            GeneratedBufferKey::new(NodeId::new(2), GenerationKind::Counter, 0),
            "1".to_owned(),
        )
        .unwrap();
        let generated = GeneratedTextStore::new(vec![draft], &index, &limits, &parsed).unwrap();
        let epoch = generated.reference_fingerprint();
        let parsed_spans = [
            TextSpan::new(
                TextBufferId::new(1),
                Utf8ByteOffset::new(0),
                Utf8ByteOffset::new(0),
            )
            .unwrap(),
            TextSpan::new(
                TextBufferId::new(0),
                Utf8ByteOffset::new(0),
                Utf8ByteOffset::new(0),
            )
            .unwrap(),
        ];
        let mapping = DisplayTextMapContents::from_stores(
            &parsed,
            &generated,
            epoch,
            &parsed_spans,
            &[generated_span],
        )
        .unwrap();
        assert_eq!(mapping.buffers()[0].text_id.get(), 0);
        assert_eq!(mapping.buffers()[1].text_id.get(), 1);
        assert_eq!(mapping.buffers()[2].text_id.get(), 2);
        assert_eq!(
            mapping
                .map_generated(generated_span)
                .unwrap()
                .text_id()
                .get(),
            2
        );
    }

    #[test]
    fn generated_display_order_is_insertion_independent() {
        let index = counter_index(2);
        let first = GeneratedBufferDraft::new(
            &index,
            GeneratedBufferKey::new(NodeId::new(2), GenerationKind::Counter, 0),
            "a".to_owned(),
        )
        .unwrap();
        let second = GeneratedBufferDraft::new(
            &index,
            GeneratedBufferKey::new(NodeId::new(3), GenerationKind::Counter, 0),
            "b".to_owned(),
        )
        .unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let empty = TextStore::new(vec![]).unwrap();
        let forward_store =
            GeneratedTextStore::new(vec![first.clone(), second.clone()], &index, &limits, &empty)
                .unwrap();
        let reverse_store =
            GeneratedTextStore::new(vec![second, first], &index, &limits, &empty).unwrap();
        let epoch = forward_store.reference_fingerprint();
        let spans = [
            GeneratedTextSpan::new(
                GeneratedTextBufferId::new(1),
                Utf8ByteOffset::new(0),
                Utf8ByteOffset::new(1),
            )
            .unwrap(),
            GeneratedTextSpan::new(
                GeneratedTextBufferId::new(0),
                Utf8ByteOffset::new(0),
                Utf8ByteOffset::new(1),
            )
            .unwrap(),
        ];
        let forward =
            DisplayTextMapContents::from_stores(&empty, &forward_store, epoch, &[], &spans)
                .unwrap();
        let reverse =
            DisplayTextMapContents::from_stores(&empty, &reverse_store, epoch, &[], &spans)
                .unwrap();
        assert_eq!(forward, reverse);

        let only_second =
            DisplayTextMapContents::from_stores(&empty, &forward_store, epoch, &[], &[spans[0]])
                .unwrap();
        assert_eq!(only_second.buffers().len(), 1);
        assert_eq!(only_second.buffers()[0].text_id.get(), 0);
        assert!(matches!(
            only_second.buffers()[0].origin,
            DisplayTextOrigin::Generated(key) if key.owner() == NodeId::new(3)
        ));
    }

    #[test]
    fn display_document_requires_a_dense_nonempty_page_sequence() {
        let (package, selected) = pagination_fixture(1);
        let size = PositiveLength::new(Length::from_raw(10).unwrap()).unwrap();
        let document = DisplayDocument::from_untrusted_parts_for_selected_pagination(
            &selected,
            vec![],
            vec![],
            vec![],
            vec![DisplayPage {
                page_index: 1,
                width: size,
                height: size,
                commands: vec![],
                annotations: vec![],
            }],
        );
        assert_eq!(
            StructurallyValidatedDisplayDocument::new(document, &package, &selected, &config()),
            Err(DisplayValidationError::NonDensePageIndex)
        );
    }

    #[test]
    fn font_instances_require_unique_canonical_face_order() {
        let (package, selected) = pagination_fixture(1);
        let geometry = &selected.selected_page_geometry()[0];
        let document = DisplayDocument::from_untrusted_parts_for_selected_pagination(
            &selected,
            vec![],
            vec![
                DisplayFontInstance {
                    font_instance_id: FontInstanceId::new(0),
                    font_face_id: FontFaceId::new(0),
                },
                DisplayFontInstance {
                    font_instance_id: FontInstanceId::new(1),
                    font_face_id: FontFaceId::new(0),
                },
            ],
            vec![],
            vec![DisplayPage {
                page_index: 0,
                width: geometry.width(),
                height: geometry.height(),
                commands: vec![],
                annotations: vec![],
            }],
        );
        assert_eq!(
            StructurallyValidatedDisplayDocument::new(document, &package, &selected, &config()),
            Err(DisplayValidationError::NonCanonicalFontFaceOrder)
        );
    }

    #[test]
    fn annotations_recheck_the_effective_uri_policy() {
        let (package, selected) = pagination_fixture(1);
        let geometry = &selected.selected_page_geometry()[0];
        let body = reference_body(&package);
        let document = DisplayDocument::from_untrusted_parts_for_selected_pagination(
            &selected,
            vec![],
            vec![],
            vec![],
            vec![DisplayPage {
                page_index: 0,
                width: geometry.width(),
                height: geometry.height(),
                commands: vec![],
                annotations: vec![LinkAnnotation {
                    target: LinkTarget::Uri(SafeUri::new("tel:+123").unwrap()),
                    rect: body,
                }],
            }],
        );
        assert_eq!(
            StructurallyValidatedDisplayDocument::new(document, &package, &selected, &config()),
            Err(DisplayValidationError::UriPolicy)
        );
    }

    #[test]
    fn display_document_validates_destination_bounds_and_cluster_coverage() {
        let (package, selected) = pagination_fixture(1);
        let geometry = &selected.selected_page_geometry()[0];
        let outside_x = geometry
            .width()
            .get()
            .checked_add(Length::from_raw(1).unwrap())
            .unwrap();
        let mut document = DisplayDocument::from_untrusted_parts_for_selected_pagination(
            &selected,
            vec![],
            vec![],
            vec![NamedDestination {
                anchor_id: AnchorId::new("outside").unwrap(),
                page_index: 0,
                view: DestinationView::Xyz {
                    point: Point {
                        x: outside_x,
                        y: Length::ZERO,
                    },
                },
            }],
            vec![DisplayPage {
                page_index: 0,
                width: geometry.width(),
                height: geometry.height(),
                commands: vec![],
                annotations: vec![],
            }],
        );
        assert_eq!(
            StructurallyValidatedDisplayDocument::new(
                document.clone(),
                &package,
                &selected,
                &config(),
            ),
            Err(DisplayValidationError::DestinationOutOfBounds)
        );

        document.destinations[0].view = DestinationView::Xyz {
            point: Point {
                x: Length::ZERO,
                y: Length::ZERO,
            },
        };
        assert_eq!(
            StructurallyValidatedDisplayDocument::new(
                document.clone(),
                &package,
                &selected,
                &config(),
            ),
            Err(DisplayValidationError::SelectedDestinationMismatch)
        );

        let text_buffers = vec![DisplayTextBuffer {
            text_id: DisplayTextBufferId::new(0),
            origin: DisplayTextOrigin::Parsed(TextBufferId::new(0)),
            utf8: "ab".to_owned(),
        }];
        let run_span = DisplayTextSpan::new(
            DisplayTextBufferId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(2),
        )
        .unwrap();
        let partial = DisplayTextSpan::new(
            DisplayTextBufferId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1),
        )
        .unwrap();
        let clusters = vec![DisplayCluster {
            logical_ordinal: 0,
            glyph_start: 0,
            glyph_end: 1,
            extraction: ClusterExtraction::Unicode { text_span: partial },
        }];
        let mut used_text_buffers = std::collections::BTreeSet::new();
        assert_eq!(
            validate_glyph_run_clusters(
                &text_buffers,
                run_span,
                1,
                &clusters,
                &mut used_text_buffers,
            ),
            Err(DisplayValidationError::ClusterCoverage)
        );
    }

    #[test]
    fn destinations_are_the_exact_selected_anchor_closure() {
        let (package, selected) = pagination_fixture_with_anchor();
        let body = reference_body(&package);
        let expected = NamedDestination {
            anchor_id: AnchorId::new("chapter").unwrap(),
            page_index: 0,
            view: DestinationView::Xyz {
                point: Point {
                    x: body.x().checked_add(Length::from_raw(1).unwrap()).unwrap(),
                    y: body.y().checked_add(Length::from_raw(2).unwrap()).unwrap(),
                },
            },
        };
        let pages = selected
            .selected_page_geometry()
            .iter()
            .map(|geometry| DisplayPage {
                page_index: geometry.page_index(),
                width: geometry.width(),
                height: geometry.height(),
                commands: vec![],
                annotations: vec![],
            })
            .collect::<Vec<_>>();
        let raw = |destinations| {
            DisplayDocument::from_untrusted_parts_for_selected_pagination(
                &selected,
                vec![],
                vec![],
                destinations,
                pages.clone(),
            )
        };

        assert_eq!(
            StructurallyValidatedDisplayDocument::new(raw(vec![]), &package, &selected, &config()),
            Err(DisplayValidationError::SelectedDestinationMismatch)
        );
        let mut wrong_point = expected.clone();
        wrong_point.view = DestinationView::Xyz {
            point: Point {
                x: body.x().checked_add(Length::from_raw(2).unwrap()).unwrap(),
                y: body.y().checked_add(Length::from_raw(2).unwrap()).unwrap(),
            },
        };
        assert_eq!(
            StructurallyValidatedDisplayDocument::new(
                raw(vec![wrong_point]),
                &package,
                &selected,
                &config(),
            ),
            Err(DisplayValidationError::SelectedDestinationMismatch)
        );
        let extra = NamedDestination {
            anchor_id: AnchorId::new("extra").unwrap(),
            page_index: 0,
            view: DestinationView::Xyz {
                point: Point {
                    x: Length::ZERO,
                    y: Length::ZERO,
                },
            },
        };
        assert_eq!(
            StructurallyValidatedDisplayDocument::new(
                raw(vec![expected.clone(), extra]),
                &package,
                &selected,
                &config(),
            ),
            Err(DisplayValidationError::SelectedDestinationMismatch)
        );

        let text_map = DisplayTextMap::from_selected_spans(&package, &selected, &[], &[]).unwrap();
        let trusted = DisplayListBuilderOwner::new()
            .issue(&selected, text_map, vec![], pages, &config())
            .unwrap();
        assert_eq!(trusted.document().destinations, vec![expected]);
    }

    #[test]
    fn structural_validation_rejects_caller_authored_parsed_text() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("parsed-text.tsf").unwrap(),
            text: "text:actual".to_owned(),
        };
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let ParseOutcome::Parsed { package, .. } = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        ) else {
            panic!("reference package must parse");
        };
        let selected_generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        assert_eq!(
            validate_selected_text_buffers(
                &[DisplayTextBuffer {
                    text_id: DisplayTextBufferId::new(0),
                    origin: DisplayTextOrigin::Parsed(TextBufferId::new(0)),
                    utf8: "forged".to_owned(),
                }],
                &selected_generated,
                Some(&package.package().text_store),
            ),
            Err(DisplayValidationError::SelectedParsedTextMismatch)
        );
    }

    #[test]
    fn trusted_display_requires_the_package_bound_text_map_and_private_paint_owner() {
        let (package, selected) = pagination_fixture(1);
        let (other_package, _) = pagination_fixture(2);
        assert_eq!(
            DisplayTextMap::from_selected_spans(&other_package, &selected, &[], &[]),
            Err(TextRemapError::WrongSelectedPackage)
        );
        let trusted =
            ValidatedDisplayDocument::paint_blank_selected(&package, &selected, &config()).unwrap();
        assert_eq!(trusted.document().pages.len(), 1);
        assert_eq!(
            trusted.document().source_layout().state_fingerprint(),
            selected.final_fingerprint()
        );
        assert_eq!(
            ValidatedDisplayDocument::paint_reference_selected(&package, &selected, &config())
                .unwrap(),
            trusted
        );
    }

    #[test]
    fn reference_painter_is_deterministic_and_emits_only_selected_geometry_and_anchors() {
        let (package, selected) = reference_paginator_fixture("anchor:z\nparagraph\nanchor:a");
        let body = reference_body(&package);
        assert_eq!(selected.selected_pages()[0].fragments.len(), 3);

        let first =
            ValidatedDisplayDocument::paint_reference_selected(&package, &selected, &config())
                .unwrap();
        let repeated =
            ValidatedDisplayDocument::paint_reference_selected(&package, &selected, &config())
                .unwrap();
        assert_eq!(first, repeated);

        let document = first.document();
        assert!(document.text_buffers.is_empty());
        assert!(document.font_instances.is_empty());
        assert_eq!(
            document.pages.len(),
            selected.selected_page_geometry().len()
        );
        for (page, geometry) in document.pages.iter().zip(selected.selected_page_geometry()) {
            assert_eq!(page.page_index, geometry.page_index());
            assert_eq!(page.width, geometry.width());
            assert_eq!(page.height, geometry.height());
            assert!(page.commands.is_empty());
            assert!(page.annotations.is_empty());
        }
        assert_eq!(
            document.destinations,
            vec![
                NamedDestination {
                    anchor_id: AnchorId::new("a").unwrap(),
                    page_index: 0,
                    view: DestinationView::Xyz {
                        point: Point {
                            x: body.x(),
                            y: body.y(),
                        },
                    },
                },
                NamedDestination {
                    anchor_id: AnchorId::new("z").unwrap(),
                    page_index: 0,
                    view: DestinationView::Xyz {
                        point: Point {
                            x: body.x(),
                            y: body.y(),
                        },
                    },
                },
            ]
        );
    }

    #[test]
    fn reference_painter_uses_exact_selected_anchor_frame_coordinates() {
        let (package, selected) = pagination_fixture_with_anchor();
        let body = reference_body(&package);
        let trusted =
            ValidatedDisplayDocument::paint_reference_selected(&package, &selected, &config())
                .unwrap();
        assert_eq!(
            trusted.document().destinations,
            vec![NamedDestination {
                anchor_id: AnchorId::new("chapter").unwrap(),
                page_index: 0,
                view: DestinationView::Xyz {
                    point: Point {
                        x: body.x().checked_add(Length::from_raw(1).unwrap()).unwrap(),
                        y: body.y().checked_add(Length::from_raw(2).unwrap()).unwrap(),
                    },
                },
            }]
        );
        assert!(trusted
            .document()
            .pages
            .iter()
            .all(|page| page.commands.is_empty()));
    }

    #[test]
    fn reference_painter_rejects_text_and_non_reference_page_records() {
        let (_, selected) = pagination_fixture(1);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("unsupported-reference-paint.tsf").unwrap(),
            text: "text:not-empty".to_owned(),
        };
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let ParseOutcome::Parsed {
            package: text_package,
            ..
        } = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        )
        else {
            panic!("text package must parse");
        };
        assert_eq!(
            ValidatedDisplayDocument::paint_reference_selected(&text_package, &selected, &config(),),
            Err(DisplayValidationError::UnsupportedReferencePaintDomain)
        );

        let (package, selected_with_column) = pagination_fixture_with_anchor_records(true);
        assert_eq!(
            ValidatedDisplayDocument::paint_reference_selected(
                &package,
                &selected_with_column,
                &config(),
            ),
            Err(DisplayValidationError::UnsupportedReferencePaintDomain)
        );
    }

    #[test]
    fn display_receipt_is_bound_to_the_selected_pagination_result() {
        let (_, selected) = pagination_fixture(1);
        let (other_package, other) = pagination_fixture(9);
        let geometry = &selected.selected_page_geometry()[0];
        let document = DisplayDocument::from_untrusted_parts_for_selected_pagination(
            &selected,
            vec![],
            vec![],
            vec![],
            vec![DisplayPage {
                page_index: 0,
                width: geometry.width(),
                height: geometry.height(),
                commands: vec![],
                annotations: vec![],
            }],
        );
        assert_eq!(
            StructurallyValidatedDisplayDocument::new(document, &other_package, &other, &config(),),
            Err(DisplayValidationError::SelectedLayoutMismatch)
        );
    }

    #[test]
    fn display_receipt_requires_selected_page_geometry() {
        let (package, selected) = pagination_fixture(1);
        let geometry = &selected.selected_page_geometry()[0];
        let wrong_width = PositiveLength::new(
            geometry
                .width()
                .get()
                .checked_add(Length::from_raw(1).unwrap())
                .unwrap(),
        )
        .unwrap();
        let document = DisplayDocument::from_untrusted_parts_for_selected_pagination(
            &selected,
            vec![],
            vec![],
            vec![],
            vec![DisplayPage {
                page_index: 0,
                width: wrong_width,
                height: geometry.height(),
                commands: vec![],
                annotations: vec![],
            }],
        );
        assert_eq!(
            StructurallyValidatedDisplayDocument::new(document, &package, &selected, &config()),
            Err(DisplayValidationError::SelectedPageClosure)
        );
    }
}
