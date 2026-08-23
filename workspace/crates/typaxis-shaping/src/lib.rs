#![forbid(unsafe_code)]

use typaxis_core::{
    BidiLevel, FontInstanceId, GlyphRunId, Length, OpenTypeTag, ResolvedDataTables, ShaperIdentity,
    TextSpan, ValidatedResourceLimits,
};
use typaxis_font::{FeatureSetting, OriginalGlyphId};
use typaxis_layout_contract::{LayoutEpoch, ShapeFontSelectionReceipt};
use typaxis_syntax::{PackageShapeTextReceipt, PackageShapeTextSource};
use typaxis_text::GeneratedProvenance;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapeSourceSpan {
    Parsed(TextSpan),
    Generated(GeneratedProvenance),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapeInputError {
    EmptyText,
    ContextLimit,
    FontSelectionEpochMismatch,
    TextEpochMismatch,
    TextStyleOwnerMismatch,
    UnsupportedLanguage,
    UnsupportedFeatureSetting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapeTextView<'a> {
    source: ShapeSourceSpan,
    utf8: &'a str,
    site_owner: typaxis_core::NodeId,
    style_owner: typaxis_core::NodeId,
    document: typaxis_core::DocumentFingerprint,
    reference: Option<typaxis_core::ReferenceFingerprint>,
}
impl<'a> ShapeTextView<'a> {
    /// Converts only a package-issued text receipt. Raw TextStore and
    /// GeneratedTextStore references cannot create a trusted shaping view.
    pub fn from_package_receipt(receipt: PackageShapeTextReceipt<'a>) -> Self {
        let source = match receipt.source() {
            PackageShapeTextSource::Parsed(span) => ShapeSourceSpan::Parsed(span),
            PackageShapeTextSource::Generated(provenance) => ShapeSourceSpan::Generated(provenance),
        };
        Self {
            source,
            utf8: receipt.utf8(),
            site_owner: receipt.site_owner(),
            style_owner: receipt.style_owner(),
            document: receipt.document_fingerprint(),
            reference: receipt.reference_fingerprint(),
        }
    }
    pub const fn source(&self) -> ShapeSourceSpan {
        self.source
    }
    pub const fn utf8(&self) -> &'a str {
        self.utf8
    }
    pub const fn site_owner(&self) -> typaxis_core::NodeId {
        self.site_owner
    }
    pub const fn style_owner(&self) -> typaxis_core::NodeId {
        self.style_owner
    }
}

fn validate_text_views<'view, 'text>(
    views: impl IntoIterator<Item = &'view ShapeTextView<'text>>,
    expected_document: typaxis_core::DocumentFingerprint,
    expected_references: typaxis_core::ReferenceFingerprint,
    expected_style_owner: typaxis_core::NodeId,
) -> Result<(), ShapeInputError>
where
    'text: 'view,
{
    for view in views {
        if view.document != expected_document
            || view
                .reference
                .is_some_and(|reference| reference != expected_references)
        {
            return Err(ShapeInputError::TextEpochMismatch);
        }
        if view.style_owner != expected_style_owner {
            return Err(ShapeInputError::TextStyleOwnerMismatch);
        }
    }
    Ok(())
}

fn validate_main_text(view: &ShapeTextView<'_>) -> Result<(), ShapeInputError> {
    if view.utf8().is_empty() {
        Err(ShapeInputError::EmptyText)
    } else {
        Ok(())
    }
}

fn validate_profile_options(
    language: Option<&str>,
    features: &[FeatureSetting],
) -> Result<(), ShapeInputError> {
    if language.is_some() {
        return Err(ShapeInputError::UnsupportedLanguage);
    }
    if !features.is_empty() {
        return Err(ShapeInputError::UnsupportedFeatureSetting);
    }
    Ok(())
}

/// A request issued only by the crate-owned canonical itemizer. Package text
/// receipts alone cannot select arbitrary contextual neighbors, language, or
/// OpenType feature work for a trusted shaping run.
///
/// ```compile_fail
/// use typaxis_shaping::ShapeRequest;
/// let _ = ShapeRequest::new;
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapeRequest<'a> {
    run_id: GlyphRunId,
    text: ShapeTextView<'a>,
    font_selection: &'a ShapeFontSelectionReceipt<'a>,
    bidi_level: BidiLevel,
    script: OpenTypeTag,
    language: Option<&'a str>,
    features: &'a [FeatureSetting],
    pre_context: Option<ShapeTextView<'a>>,
    post_context: Option<ShapeTextView<'a>>,
    data_tables: &'a ResolvedDataTables,
    max_output_records: u32,
}
impl<'a> ShapeRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)] // reserved for the in-crate canonical itemizer
    fn new(
        run_id: GlyphRunId,
        text: ShapeTextView<'a>,
        font_selection: &'a ShapeFontSelectionReceipt<'a>,
        expected_epoch: LayoutEpoch,
        bidi_level: BidiLevel,
        script: OpenTypeTag,
        language: Option<&'a str>,
        features: &'a [FeatureSetting],
        pre_context: Option<ShapeTextView<'a>>,
        post_context: Option<ShapeTextView<'a>>,
        data_tables: &'a ResolvedDataTables,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, ShapeInputError> {
        if !font_selection.matches_epoch(expected_epoch) {
            return Err(ShapeInputError::FontSelectionEpochMismatch);
        }
        validate_profile_options(language, features)?;
        validate_text_views(
            [Some(&text), pre_context.as_ref(), post_context.as_ref()]
                .into_iter()
                .flatten(),
            expected_epoch.document(),
            expected_epoch.references(),
            font_selection.owner(),
        )?;
        validate_main_text(&text)?;
        let combined_bytes = text
            .utf8()
            .len()
            .checked_add(pre_context.as_ref().map_or(0, |view| view.utf8().len()))
            .and_then(|bytes| {
                bytes.checked_add(post_context.as_ref().map_or(0, |view| view.utf8().len()))
            })
            .ok_or(ShapeInputError::ContextLimit)?;
        if combined_bytes > limits.get().max_shaping_context_bytes as usize {
            return Err(ShapeInputError::ContextLimit);
        }
        Ok(Self {
            run_id,
            text,
            font_selection,
            bidi_level,
            script,
            language,
            features,
            pre_context,
            post_context,
            data_tables,
            max_output_records: limits.get().max_shaping_context_bytes,
        })
    }
    pub const fn run_id(&self) -> GlyphRunId {
        self.run_id
    }
    pub const fn text(&self) -> &ShapeTextView<'a> {
        &self.text
    }
    pub const fn font(&self) -> FontInstanceId {
        self.font_selection.admitted_font().font_instance_id()
    }
    pub const fn font_selection(&self) -> &'a ShapeFontSelectionReceipt<'a> {
        self.font_selection
    }
    pub const fn layout_epoch(&self) -> LayoutEpoch {
        self.font_selection.epoch()
    }
    pub const fn admitted_font_sha256(&self) -> [u8; 32] {
        self.font_selection.admitted_font().admitted_sha256()
    }
    pub fn admitted_font_bytes(&self) -> &'a [u8] {
        self.font_selection.admitted_font().font_bytes()
    }
    pub const fn bidi_level(&self) -> BidiLevel {
        self.bidi_level
    }
    pub const fn script(&self) -> OpenTypeTag {
        self.script
    }
    pub const fn language(&self) -> Option<&'a str> {
        self.language
    }
    pub const fn features(&self) -> &'a [FeatureSetting] {
        self.features
    }
    pub const fn pre_context(&self) -> Option<&ShapeTextView<'a>> {
        self.pre_context.as_ref()
    }
    pub const fn post_context(&self) -> Option<&ShapeTextView<'a>> {
        self.post_context.as_ref()
    }
    pub const fn data_tables(&self) -> &'a ResolvedDataTables {
        self.data_tables
    }
    pub const fn max_output_records(&self) -> u32 {
        self.max_output_records
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedGlyph {
    pub original_gid: OriginalGlyphId,
    pub advance_x: Length,
    pub advance_y: Length,
    pub offset_x: Length,
    pub offset_y: Length,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedCluster {
    pub source_span: ShapeSourceSpan,
    pub glyph_start: u32,
    pub glyph_end: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlyphRun {
    pub run_id: GlyphRunId,
    pub font: FontInstanceId,
    pub bidi_level: BidiLevel,
    pub source_span: ShapeSourceSpan,
    pub glyphs: Vec<ShapedGlyph>,
    pub clusters: Vec<ShapedCluster>,
}
mod shaper_seal {
    pub trait Sealed {}
}

/// Trusted shaping backends are linked into this crate and must reserve every
/// output record through `ShapeOutputBudget` before allocation. External
/// implementations cannot claim the linked backend identity.
pub trait Shaper: shaper_seal::Sealed {
    type Error;
    fn identity(&self) -> ShaperIdentity;
    fn shape(
        &self,
        request: ShapeRequest<'_>,
        budget: &mut ShapeOutputBudget,
    ) -> Result<GlyphRun, Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapeWorkError {
    GlyphLimit,
    ClusterLimit,
}

/// Allocation-before-work budget owned by the shaping entrypoint. Profile 1.0
/// caps both vectors by `max_shaping_context_bytes`, a conservative closed
/// expansion bound independent of backend allocation strategy.
#[derive(Debug, Eq, PartialEq)]
pub struct ShapeOutputBudget {
    limit: u32,
    remaining_glyphs: u32,
    remaining_clusters: u32,
}
impl ShapeOutputBudget {
    fn new(max_output_records: u32) -> Self {
        Self {
            limit: max_output_records,
            remaining_glyphs: max_output_records,
            remaining_clusters: max_output_records,
        }
    }
    pub fn reserve_glyphs(&mut self, count: u32) -> Result<(), ShapeWorkError> {
        self.remaining_glyphs = self
            .remaining_glyphs
            .checked_sub(count)
            .ok_or(ShapeWorkError::GlyphLimit)?;
        Ok(())
    }
    pub fn reserve_clusters(&mut self, count: u32) -> Result<(), ShapeWorkError> {
        self.remaining_clusters = self
            .remaining_clusters
            .checked_sub(count)
            .ok_or(ShapeWorkError::ClusterLimit)?;
        Ok(())
    }
    fn matches_output(&self, run: &GlyphRun) -> bool {
        self.limit - self.remaining_glyphs == u32::try_from(run.glyphs.len()).unwrap_or(u32::MAX)
            && self.limit - self.remaining_clusters
                == u32::try_from(run.clusters.len()).unwrap_or(u32::MAX)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphRunValidationError {
    RunIdentityMismatch,
    FontMismatch,
    BidiLevelMismatch,
    SourceMismatch,
    GlyphOutOfRange,
    OutputLimit,
    WorkReceiptMismatch,
    EmptyOutputMismatch,
    InvalidClusterSource,
    InvalidClusterGlyphRange,
    InvalidClusterUtf8Boundary,
    NonCanonicalClusterCoverage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShapeExecutionError<E> {
    Backend(E),
    InvalidOutput(GlyphRunValidationError),
}

/// Shaper output checked against the exact request and admitted font metadata.
/// Downstream line layout accepts this receipt rather than a raw `GlyphRun`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedGlyphRun {
    run: GlyphRun,
    epoch: LayoutEpoch,
    site_owner: typaxis_core::NodeId,
    style_owner: typaxis_core::NodeId,
    shaper: ShaperIdentity,
    data_tables: ResolvedDataTables,
}
impl ValidatedGlyphRun {
    pub const fn run(&self) -> &GlyphRun {
        &self.run
    }
    pub const fn run_id(&self) -> GlyphRunId {
        self.run.run_id
    }
    pub const fn font(&self) -> FontInstanceId {
        self.run.font
    }
    pub const fn bidi_level(&self) -> BidiLevel {
        self.run.bidi_level
    }
    pub const fn source_span(&self) -> ShapeSourceSpan {
        self.run.source_span
    }
    pub fn glyphs(&self) -> &[ShapedGlyph] {
        &self.run.glyphs
    }
    pub fn clusters(&self) -> &[ShapedCluster] {
        &self.run.clusters
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn site_owner(&self) -> typaxis_core::NodeId {
        self.site_owner
    }
    pub const fn style_owner(&self) -> typaxis_core::NodeId {
        self.style_owner
    }
    pub const fn shaper_identity(&self) -> ShaperIdentity {
        self.shaper
    }
    pub const fn data_tables(&self) -> &ResolvedDataTables {
        &self.data_tables
    }
}

pub fn shape_validated<S: Shaper>(
    shaper: &S,
    request: ShapeRequest<'_>,
) -> Result<ValidatedGlyphRun, ShapeExecutionError<S::Error>> {
    let expected = ExpectedShapeOutput {
        glyph_run: ExpectedGlyphRun {
            run_id: request.run_id(),
            font: request.font(),
            bidi_level: request.bidi_level(),
            source: request.text().source(),
            utf8_boundaries: utf8_boundaries(request.text().source(), request.text().utf8())
                .ok_or(ShapeExecutionError::InvalidOutput(
                    GlyphRunValidationError::InvalidClusterUtf8Boundary,
                ))?,
            glyph_count: request
                .font_selection()
                .admitted_font()
                .metadata()
                .glyph_count,
            max_output_records: request.max_output_records(),
        },
        epoch: request.layout_epoch(),
        site_owner: request.text().site_owner(),
        style_owner: request.text().style_owner(),
        shaper: shaper.identity(),
        data_tables: request.data_tables().clone(),
    };
    let mut budget = ShapeOutputBudget::new(expected.glyph_run.max_output_records);
    let run = shaper
        .shape(request, &mut budget)
        .map_err(ShapeExecutionError::Backend)?;
    if !budget.matches_output(&run) {
        return Err(ShapeExecutionError::InvalidOutput(
            GlyphRunValidationError::WorkReceiptMismatch,
        ));
    }
    validate_glyph_run(&expected.glyph_run, &run).map_err(ShapeExecutionError::InvalidOutput)?;
    Ok(ValidatedGlyphRun {
        run,
        epoch: expected.epoch,
        site_owner: expected.site_owner,
        style_owner: expected.style_owner,
        shaper: expected.shaper,
        data_tables: expected.data_tables,
    })
}

#[derive(Clone)]
struct ExpectedShapeOutput {
    glyph_run: ExpectedGlyphRun,
    epoch: LayoutEpoch,
    site_owner: typaxis_core::NodeId,
    style_owner: typaxis_core::NodeId,
    shaper: ShaperIdentity,
    data_tables: ResolvedDataTables,
}

#[derive(Clone)]
struct ExpectedGlyphRun {
    run_id: GlyphRunId,
    font: FontInstanceId,
    bidi_level: BidiLevel,
    source: ShapeSourceSpan,
    utf8_boundaries: Vec<u32>,
    glyph_count: u32,
    max_output_records: u32,
}

fn validate_glyph_run(
    expected: &ExpectedGlyphRun,
    run: &GlyphRun,
) -> Result<(), GlyphRunValidationError> {
    if run.run_id != expected.run_id {
        return Err(GlyphRunValidationError::RunIdentityMismatch);
    }
    if run.font != expected.font {
        return Err(GlyphRunValidationError::FontMismatch);
    }
    if run.bidi_level != expected.bidi_level {
        return Err(GlyphRunValidationError::BidiLevelMismatch);
    }
    if run.source_span != expected.source {
        return Err(GlyphRunValidationError::SourceMismatch);
    }
    if run
        .glyphs
        .iter()
        .any(|glyph| u32::from(glyph.original_gid.get()) >= expected.glyph_count)
    {
        return Err(GlyphRunValidationError::GlyphOutOfRange);
    }
    if run.glyphs.len() > expected.max_output_records as usize
        || run.clusters.len() > expected.max_output_records as usize
    {
        return Err(GlyphRunValidationError::OutputLimit);
    }
    if run.glyphs.is_empty() || run.clusters.is_empty() {
        return Err(GlyphRunValidationError::EmptyOutputMismatch);
    }

    let mut expected_source_start = source_range(expected.source).0;
    let expected_source_end = source_range(expected.source).1;
    let mut visual_ranges = Vec::with_capacity(run.clusters.len());
    for cluster in &run.clusters {
        if !same_source_namespace(expected.source, cluster.source_span) {
            return Err(GlyphRunValidationError::InvalidClusterSource);
        }
        let (start, end) = source_range(cluster.source_span);
        if expected.utf8_boundaries.binary_search(&start).is_err()
            || expected.utf8_boundaries.binary_search(&end).is_err()
        {
            return Err(GlyphRunValidationError::InvalidClusterUtf8Boundary);
        }
        if start != expected_source_start || start >= end || end > expected_source_end {
            return Err(GlyphRunValidationError::NonCanonicalClusterCoverage);
        }
        expected_source_start = end;
        if cluster.glyph_start >= cluster.glyph_end
            || usize::try_from(cluster.glyph_end)
                .ok()
                .map_or(true, |end| end > run.glyphs.len())
        {
            return Err(GlyphRunValidationError::InvalidClusterGlyphRange);
        }
        visual_ranges.push((cluster.glyph_start, cluster.glyph_end));
    }
    if expected_source_start != expected_source_end {
        return Err(GlyphRunValidationError::NonCanonicalClusterCoverage);
    }
    visual_ranges.sort_unstable();
    let mut expected_glyph_start = 0u32;
    for (start, end) in visual_ranges {
        if start != expected_glyph_start {
            return Err(GlyphRunValidationError::InvalidClusterGlyphRange);
        }
        expected_glyph_start = end;
    }
    if usize::try_from(expected_glyph_start).ok() != Some(run.glyphs.len()) {
        return Err(GlyphRunValidationError::InvalidClusterGlyphRange);
    }
    Ok(())
}

fn source_range(source: ShapeSourceSpan) -> (u32, u32) {
    match source {
        ShapeSourceSpan::Parsed(span) => (span.start_byte().get(), span.end_byte().get()),
        ShapeSourceSpan::Generated(provenance) => {
            let range = provenance.text_span().range();
            (range.start_byte().get(), range.end_byte().get())
        }
    }
}

fn utf8_boundaries(source: ShapeSourceSpan, utf8: &str) -> Option<Vec<u32>> {
    let (start, end) = source_range(source);
    let capacity = utf8.chars().count().checked_add(1)?;
    let mut boundaries = Vec::with_capacity(capacity);
    for (index, _) in utf8.char_indices() {
        boundaries.push(start.checked_add(u32::try_from(index).ok()?)?);
    }
    if boundaries.last().copied() != Some(end) {
        boundaries.push(end);
    }
    Some(boundaries)
}

fn same_source_namespace(left: ShapeSourceSpan, right: ShapeSourceSpan) -> bool {
    match (left, right) {
        (ShapeSourceSpan::Parsed(left), ShapeSourceSpan::Parsed(right)) => {
            left.text_id() == right.text_id()
        }
        (ShapeSourceSpan::Generated(left), ShapeSourceSpan::Generated(right)) => {
            left.buffer_key() == right.buffer_key()
                && left.text_span().text_id() == right.text_span().text_id()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{
        DocumentFingerprint, NodeId, ReferenceFingerprint, TextBufferId, Utf8ByteOffset,
    };

    fn parsed_view(document: DocumentFingerprint, style_owner: NodeId) -> ShapeTextView<'static> {
        ShapeTextView {
            source: ShapeSourceSpan::Parsed(
                TextSpan::new(
                    TextBufferId::new(0),
                    Utf8ByteOffset::new(0),
                    Utf8ByteOffset::new(1),
                )
                .expect("valid test span"),
            ),
            utf8: "x",
            site_owner: NodeId::new(2),
            style_owner,
            document,
            reference: None,
        }
    }

    fn glyph() -> ShapedGlyph {
        ShapedGlyph {
            original_gid: OriginalGlyphId::new(0),
            advance_x: Length::ZERO,
            advance_y: Length::ZERO,
            offset_x: Length::ZERO,
            offset_y: Length::ZERO,
        }
    }

    fn expected_multibyte_run() -> (ExpectedGlyphRun, GlyphRun) {
        let span = TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(3),
        )
        .unwrap();
        let expected = ExpectedGlyphRun {
            run_id: GlyphRunId::new(4),
            font: FontInstanceId::new(2),
            bidi_level: BidiLevel::new(1).unwrap(),
            source: ShapeSourceSpan::Parsed(span),
            utf8_boundaries: vec![0, 2, 3],
            glyph_count: 1,
            max_output_records: 3,
        };
        let run = GlyphRun {
            run_id: expected.run_id,
            font: expected.font,
            bidi_level: expected.bidi_level,
            source_span: expected.source,
            glyphs: vec![glyph(), glyph()],
            // Logical order is independent of visual glyph order for RTL.
            clusters: vec![
                ShapedCluster {
                    source_span: ShapeSourceSpan::Parsed(
                        TextSpan::new(
                            TextBufferId::new(0),
                            Utf8ByteOffset::new(0),
                            Utf8ByteOffset::new(2),
                        )
                        .unwrap(),
                    ),
                    glyph_start: 1,
                    glyph_end: 2,
                },
                ShapedCluster {
                    source_span: ShapeSourceSpan::Parsed(
                        TextSpan::new(
                            TextBufferId::new(0),
                            Utf8ByteOffset::new(2),
                            Utf8ByteOffset::new(3),
                        )
                        .unwrap(),
                    ),
                    glyph_start: 0,
                    glyph_end: 1,
                },
            ],
        };
        (expected, run)
    }

    #[test]
    fn text_identity_rejects_foreign_package_or_generated_state() {
        let document = DocumentFingerprint::from_untrusted_bytes([1; 32]);
        let references = ReferenceFingerprint::from_untrusted_bytes([2; 32]);
        let owner = NodeId::new(1);

        let foreign_document =
            parsed_view(DocumentFingerprint::from_untrusted_bytes([3; 32]), owner);
        assert_eq!(
            validate_text_views([&foreign_document], document, references, owner),
            Err(ShapeInputError::TextEpochMismatch)
        );

        let mut foreign_state = parsed_view(document, owner);
        foreign_state.reference = Some(ReferenceFingerprint::from_untrusted_bytes([4; 32]));
        assert_eq!(
            validate_text_views([&foreign_state], document, references, owner),
            Err(ShapeInputError::TextEpochMismatch)
        );
    }

    #[test]
    fn text_identity_rejects_context_from_another_style_owner() {
        let document = DocumentFingerprint::from_untrusted_bytes([1; 32]);
        let references = ReferenceFingerprint::from_untrusted_bytes([2; 32]);
        let main_owner = NodeId::new(1);
        let main = parsed_view(document, main_owner);
        let foreign_context = parsed_view(document, NodeId::new(3));

        assert_eq!(
            validate_text_views([&main, &foreign_context], document, references, main_owner,),
            Err(ShapeInputError::TextStyleOwnerMismatch)
        );
    }

    #[test]
    fn text_identity_accepts_same_owner_and_selected_generated_state() {
        let document = DocumentFingerprint::from_untrusted_bytes([1; 32]);
        let references = ReferenceFingerprint::from_untrusted_bytes([2; 32]);
        let owner = NodeId::new(1);
        let main = parsed_view(document, owner);
        let mut generated_context = parsed_view(document, owner);
        generated_context.reference = Some(references);

        assert_eq!(
            validate_text_views([&main, &generated_context], document, references, owner,),
            Ok(())
        );
    }

    #[test]
    fn main_text_must_be_nonempty_but_context_identity_still_allows_empty_views() {
        let document = DocumentFingerprint::from_untrusted_bytes([1; 32]);
        let owner = NodeId::new(1);
        let mut empty = parsed_view(document, owner);
        empty.utf8 = "";
        assert_eq!(validate_main_text(&empty), Err(ShapeInputError::EmptyText));
        assert_eq!(
            validate_text_views(
                [&empty],
                document,
                ReferenceFingerprint::from_untrusted_bytes([2; 32]),
                owner,
            ),
            Ok(())
        );
    }

    #[test]
    fn profile_rejects_caller_local_language_and_feature_work() {
        let feature = FeatureSetting {
            tag: OpenTypeTag::new(*b"liga").unwrap(),
            value: 1,
        };
        assert_eq!(
            validate_profile_options(Some("ja"), &[]),
            Err(ShapeInputError::UnsupportedLanguage)
        );
        assert_eq!(
            validate_profile_options(None, &[feature]),
            Err(ShapeInputError::UnsupportedFeatureSetting)
        );
        assert_eq!(validate_profile_options(None, &[]), Ok(()));
    }

    #[test]
    fn shaping_output_budget_rejects_max_plus_one_before_allocation() {
        let mut budget = ShapeOutputBudget::new(1);
        assert_eq!(budget.reserve_glyphs(1), Ok(()));
        assert_eq!(budget.reserve_glyphs(1), Err(ShapeWorkError::GlyphLimit));
        assert_eq!(budget.reserve_clusters(1), Ok(()));
        assert_eq!(
            budget.reserve_clusters(1),
            Err(ShapeWorkError::ClusterLimit)
        );
    }

    #[test]
    fn validated_run_accepts_rtl_visual_order_but_requires_exact_logical_coverage() {
        let (expected, run) = expected_multibyte_run();
        assert_eq!(validate_glyph_run(&expected, &run), Ok(()));

        let mut missing_cluster = run.clone();
        missing_cluster.clusters.clear();
        assert_eq!(
            validate_glyph_run(&expected, &missing_cluster),
            Err(GlyphRunValidationError::EmptyOutputMismatch)
        );

        let mut missing_glyph = run.clone();
        missing_glyph.glyphs.clear();
        assert_eq!(
            validate_glyph_run(&expected, &missing_glyph),
            Err(GlyphRunValidationError::EmptyOutputMismatch)
        );
    }

    #[test]
    fn validated_run_rejects_utf8_mid_scalar_cluster_boundary() {
        let (expected, mut run) = expected_multibyte_run();
        run.clusters[0].source_span = ShapeSourceSpan::Parsed(
            TextSpan::new(
                TextBufferId::new(0),
                Utf8ByteOffset::new(0),
                Utf8ByteOffset::new(1),
            )
            .unwrap(),
        );
        run.clusters[1].source_span = ShapeSourceSpan::Parsed(
            TextSpan::new(
                TextBufferId::new(0),
                Utf8ByteOffset::new(1),
                Utf8ByteOffset::new(3),
            )
            .unwrap(),
        );
        assert_eq!(
            validate_glyph_run(&expected, &run),
            Err(GlyphRunValidationError::InvalidClusterUtf8Boundary)
        );
    }

    #[test]
    fn validated_run_rejects_wrong_identity_and_out_of_range_gid() {
        let (expected, mut run) = expected_multibyte_run();
        run.run_id = GlyphRunId::new(99);
        assert_eq!(
            validate_glyph_run(&expected, &run),
            Err(GlyphRunValidationError::RunIdentityMismatch)
        );
        run.run_id = expected.run_id;
        run.glyphs[0].original_gid = OriginalGlyphId::new(1);
        assert_eq!(
            validate_glyph_run(&expected, &run),
            Err(GlyphRunValidationError::GlyphOutOfRange)
        );
    }
}
