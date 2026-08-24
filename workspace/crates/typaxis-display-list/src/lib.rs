#![forbid(unsafe_code)]

use typaxis_core::{
    AffineTransform, AnchorId, BidiLevel, DisplayGlyphRunId, DisplayTextBufferId, DisplayTextSpan,
    EffectiveConfig, FontFaceId, FontInstanceId, GeneratedBufferKey, GeneratedTextBufferId,
    GeneratedTextSpan, ImageResourceId, LayoutStateFingerprint, Length, MasterId,
    NonNegativeLength, Point, PositiveLength, PositiveUnitless16_16, Rect, ReferenceFingerprint,
    SafeUri, TextBufferId, TextSpan, JSON_SAFE_INTEGER_MAX,
};
use typaxis_document::{Block, Inline};
use typaxis_font::OriginalGlyphId;
use typaxis_layout::{FlowTree, LayoutEpoch};
use typaxis_linebreak::{
    reorder_line_l2, reset_line_bidi_levels, LineBidiClass, LineLevelsAfterL1, ParagraphItem,
    ShapedSlice, ValidatedParagraphItemRegistry,
};
use typaxis_pagination::PaginationResult;
use typaxis_shaping::{ShapeSourceSpan, ValidatedGlyphRun};
use typaxis_style::StyleValue;
use typaxis_syntax::ValidatedParsedPackage;
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
        )
    }

    fn from_verified_text_map(
        document: DisplayDocument,
        selected: &PaginationResult,
        config: &EffectiveConfig,
    ) -> Result<Self, DisplayValidationError> {
        Self::validate(document, selected, config, None)
    }

    fn validate(
        document: DisplayDocument,
        selected: &PaginationResult,
        config: &EffectiveConfig,
        parsed_store: Option<&TextStore>,
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
        if document.destinations != destinations_from_selected_pagination(selected)? {
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
            selected_page_geometry: selected_geometry
                .iter()
                .map(|geometry| ValidatedDisplayPageGeometry {
                    page_index: geometry.page_index(),
                    master_id: geometry.master_id().clone(),
                    width: geometry.width(),
                    height: geometry.height(),
                })
                .collect(),
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
    let delta = fragment
        .bounds
        .width()
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
    let text_span = match shaped.source() {
        ShapeSourceSpan::Parsed(span) => text_map.map_parsed(span),
        ShapeSourceSpan::Generated(provenance) => text_map.map_generated(provenance.text_span()),
    }
    .map_err(|_| DisplayValidationError::SelectedTextMapMismatch)?;
    let font_size = match package
        .cascade_style(shaped.site_owner())
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
            extraction: ClusterExtraction::Unicode { text_span },
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
