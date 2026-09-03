#![forbid(unsafe_code)]

use read_fonts::TableProvider;
use typaxis_core::{
    push_jcs_string, sha256, BidiLevel, FontFaceId, FontInstanceId, GeneratedBufferKey, GlyphRunId,
    Length, LengthError, NodeId, OpenTypeTag, PositiveLength, ResolvedDataTables, ShaperIdentity,
    SourceSpan, TextSpan, Utf8ByteOffset, ValidatedResourceLimits,
};
use typaxis_font::{FeatureSetting, OriginalGlyphId};
use typaxis_layout_contract::{LayoutEpoch, ShapeFontSelectionReceipt};
use typaxis_resource_admission::{
    staging_declared_base_catalog, AdmittedFont, AdmittedResourceLedger,
};
use typaxis_syntax::{
    PackageComputedStyle, PackageParagraphTextSite, PackageShapeTextReceipt,
    PackageShapeTextSource, PrecomposedVectorKind, ValidatedParsedPackage,
    ValidatedPrecomposedVectorEffectiveLanguage, ValidatedPrecomposedVectorMetrics,
    ValidatedStagingSemanticPackage,
};
use typaxis_text::GeneratedProvenance;
use unicode_script::{Script as UnicodeScriptValue, ScriptExtension};
use unicode_segmentation::UnicodeSegmentation;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemizationError {
    PackageIdentityMismatch,
    StyleIdentityMismatch,
    FontSelectionIdentityMismatch,
    TextIdentityMismatch,
    GeneratedReferenceMismatch,
    NonCanonicalTextReceipt,
    EmptyText,
    ContextLimit,
    BackendOutputLimit,
    UnicodeDataVersionMismatch,
    ParagraphBoundaryUnsupported,
    UnsupportedBidiLevel,
    BidiLevelSplitsGrapheme,
    SiteBoundarySplitsGrapheme,
    ParagraphSiteMismatch,
    UnsupportedScriptCluster,
    AmbiguousScriptCluster,
    InvalidScriptTag,
    MissingDeclaredFontCoverage,
    InvalidFontOrFace,
    AllocationFailure,
    ArithmeticOverflow,
    ShapeInput(ShapeInputError),
}

/// Requests derived from one package-issued logical text receipt. Run IDs are
/// dense in logical order and callers cannot insert or replace requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemizedShapeRequests<'a> {
    text: PackageShapeTextReceipt<'a>,
    paragraph_level: BidiLevel,
    requests: Vec<ShapeRequest<'a>>,
}

/// One canonical paragraph site with its package-computed style and sealed
/// font selection. Construction is public for orchestration, but the itemizer
/// rechecks every identity and the exact package-derived site order before it
/// issues any `ShapeRequest`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParagraphItemizationInput<'a> {
    computed: PackageComputedStyle,
    text: PackageShapeTextReceipt<'a>,
    font_selection: ShapeFontSelectionReceipt<'a>,
}
impl<'a> ParagraphItemizationInput<'a> {
    pub const fn new(
        computed: PackageComputedStyle,
        text: PackageShapeTextReceipt<'a>,
        font_selection: ShapeFontSelectionReceipt<'a>,
    ) -> Self {
        Self {
            computed,
            text,
            font_selection,
        }
    }
    pub const fn text_receipt(&self) -> PackageShapeTextReceipt<'a> {
        self.text
    }
}
impl<'a> ItemizedShapeRequests<'a> {
    pub const fn text_receipt(&self) -> PackageShapeTextReceipt<'a> {
        self.text
    }
    /// UAX #9 paragraph embedding level issued by the same canonical
    /// itemization pass that derived every run request. Keeping this fact on
    /// the itemized owner prevents line layout from guessing the paragraph
    /// level from the shaped runs (which is not valid in the presence of
    /// explicit embeddings).
    pub const fn paragraph_level(&self) -> BidiLevel {
        self.paragraph_level
    }
    pub fn requests(&self) -> &[ShapeRequest<'a>] {
        &self.requests
    }
    pub fn into_requests(self) -> Vec<ShapeRequest<'a>> {
        self.requests
    }
}

/// Profile 1.0's package-owned itemizer. It is the only production owner that
/// invokes the private `ShapeRequest` constructor.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CanonicalItemizer;
impl CanonicalItemizer {
    pub const fn new() -> Self {
        Self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn itemize<'a>(
        &self,
        package: &ValidatedParsedPackage,
        computed: &PackageComputedStyle,
        text: PackageShapeTextReceipt<'a>,
        font_selection: &'a ShapeFontSelectionReceipt<'a>,
        expected_epoch: LayoutEpoch,
        data_tables: &'a ResolvedDataTables,
        limits: &ValidatedResourceLimits,
    ) -> Result<ItemizedShapeRequests<'a>, ItemizationError> {
        validate_itemizer_identities(package, computed, text, font_selection, expected_epoch)?;
        validate_itemizer_unicode_tables(data_tables)?;
        if !text.covers_complete_site() || !text.is_standalone_logical_text() {
            return Err(ItemizationError::NonCanonicalTextReceipt);
        }
        if text.utf8().is_empty() {
            return Err(ItemizationError::EmptyText);
        }
        let text_len =
            u32::try_from(text.utf8().len()).map_err(|_| ItemizationError::ArithmeticOverflow)?;
        if text_len > limits.get().max_shaping_context_bytes {
            return Err(ItemizationError::ContextLimit);
        }
        let (paragraph_level, specs) = itemize_run_specs(text.utf8())?;
        for spec in &specs {
            let run_utf8 = itemized_run_utf8(text.utf8(), *spec)?;
            let backend_record_bound =
                linked_backend_record_bound(run_utf8).map_err(map_linked_itemization_error)?;
            if backend_record_bound > limits.get().max_shaping_context_bytes {
                return Err(ItemizationError::BackendOutputLimit);
            }
        }
        validate_selected_font_coverage(font_selection, text.utf8())?;
        let whole = ShapeTextView::from_package_receipt(text);
        let mut requests = Vec::new();
        requests
            .try_reserve_exact(specs.len())
            .map_err(|_| ItemizationError::AllocationFailure)?;
        for (index, spec) in specs.into_iter().enumerate() {
            let run_id = GlyphRunId::new(
                u32::try_from(index).map_err(|_| ItemizationError::ArithmeticOverflow)?,
            );
            let main = narrow_text_view(&whole, spec.start, spec.end)?;
            let pre_context = if spec.start == 0 {
                None
            } else {
                Some(narrow_text_view(&whole, 0, spec.start)?)
            };
            let post_context = if spec.end == text_len {
                None
            } else {
                Some(narrow_text_view(&whole, spec.end, text_len)?)
            };
            let request = ShapeRequest::new(
                run_id,
                main,
                font_selection,
                expected_epoch,
                spec.bidi_level,
                spec.script,
                None,
                &[],
                pre_context,
                post_context,
                data_tables,
                limits,
            )
            .map_err(ItemizationError::ShapeInput)?;
            requests.push(request);
        }
        Ok(ItemizedShapeRequests {
            text,
            paragraph_level,
            requests,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn itemize_paragraph<'a>(
        &self,
        package: &ValidatedParsedPackage,
        paragraph_owner: NodeId,
        sites: &'a [ParagraphItemizationInput<'a>],
        expected_epoch: LayoutEpoch,
        data_tables: &'a ResolvedDataTables,
        limits: &ValidatedResourceLimits,
    ) -> Result<Vec<Option<ItemizedShapeRequests<'a>>>, ItemizationError> {
        validate_itemizer_unicode_tables(data_tables)?;
        let expected_sites = package
            .paragraph_shape_text_sites(paragraph_owner)
            .ok_or(ItemizationError::ParagraphSiteMismatch)?;
        if expected_sites.len() != sites.len() {
            return Err(ItemizationError::ParagraphSiteMismatch);
        }
        let mut whole_utf8 = String::new();
        let total_len = sites.iter().try_fold(0usize, |total, site| {
            total
                .checked_add(site.text.utf8().len())
                .ok_or(ItemizationError::ArithmeticOverflow)
        })?;
        if total_len > limits.get().max_shaping_context_bytes as usize {
            return Err(ItemizationError::ContextLimit);
        }
        whole_utf8
            .try_reserve_exact(total_len)
            .map_err(|_| ItemizationError::AllocationFailure)?;
        let mut ranges = Vec::new();
        ranges
            .try_reserve_exact(sites.len())
            .map_err(|_| ItemizationError::AllocationFailure)?;
        let mut views = Vec::new();
        views
            .try_reserve_exact(sites.len())
            .map_err(|_| ItemizationError::AllocationFailure)?;
        for (expected, site) in expected_sites.iter().zip(sites) {
            validate_itemizer_identities(
                package,
                &site.computed,
                site.text,
                &site.font_selection,
                expected_epoch,
            )?;
            if !site.text.covers_complete_site()
                || site.text.style_owner() != paragraph_owner
                || !paragraph_site_matches(*expected, site.text.source())
            {
                return Err(ItemizationError::ParagraphSiteMismatch);
            }
            let start = whole_utf8.len();
            whole_utf8.push_str(site.text.utf8());
            ranges.push((start, whole_utf8.len()));
            views.push(ShapeTextView::from_package_receipt(site.text));
        }
        if whole_utf8.is_empty() {
            return Ok(sites.iter().map(|_| None).collect());
        }
        let internal_boundary_count = ranges
            .len()
            .checked_sub(1)
            .ok_or(ItemizationError::ParagraphSiteMismatch)?;
        for (_, boundary) in ranges.iter().take(internal_boundary_count) {
            if !UnicodeSegmentation::grapheme_indices(whole_utf8.as_str(), true)
                .any(|(start, _)| start == *boundary)
            {
                return Err(ItemizationError::SiteBoundarySplitsGrapheme);
            }
        }
        let (paragraph_level, specs) = itemize_run_specs(&whole_utf8)?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(sites.len())
            .map_err(|_| ItemizationError::AllocationFailure)?;
        for (site_index, site) in sites.iter().enumerate() {
            let (site_start, site_end) = ranges[site_index];
            if site_start == site_end {
                result.push(None);
                continue;
            }
            validate_selected_font_coverage(&site.font_selection, site.text.utf8())?;
            let site_len = u32::try_from(site_end - site_start)
                .map_err(|_| ItemizationError::ArithmeticOverflow)?;
            let mut requests = Vec::new();
            for spec in &specs {
                let spec_start = usize::try_from(spec.start)
                    .map_err(|_| ItemizationError::ArithmeticOverflow)?;
                let spec_end =
                    usize::try_from(spec.end).map_err(|_| ItemizationError::ArithmeticOverflow)?;
                let overlap_start = spec_start.max(site_start);
                let overlap_end = spec_end.min(site_end);
                if overlap_start >= overlap_end {
                    continue;
                }
                let local_start = u32::try_from(overlap_start - site_start)
                    .map_err(|_| ItemizationError::ArithmeticOverflow)?;
                let local_end = u32::try_from(overlap_end - site_start)
                    .map_err(|_| ItemizationError::ArithmeticOverflow)?;
                let run_utf8 = site
                    .text
                    .utf8()
                    .get(
                        usize::try_from(local_start)
                            .map_err(|_| ItemizationError::ArithmeticOverflow)?
                            ..usize::try_from(local_end)
                                .map_err(|_| ItemizationError::ArithmeticOverflow)?,
                    )
                    .ok_or(ItemizationError::ArithmeticOverflow)?;
                let backend_record_bound =
                    linked_backend_record_bound(run_utf8).map_err(map_linked_itemization_error)?;
                if backend_record_bound > limits.get().max_shaping_context_bytes {
                    return Err(ItemizationError::BackendOutputLimit);
                }
                let main = narrow_text_view(&views[site_index], local_start, local_end)?;
                let pre_context = if local_start > 0 {
                    Some(narrow_text_view(&views[site_index], 0, local_start)?)
                } else {
                    previous_nonempty_view(&views, site_index).cloned()
                };
                let post_context = if local_end < site_len {
                    Some(narrow_text_view(&views[site_index], local_end, site_len)?)
                } else {
                    next_nonempty_view(&views, site_index).cloned()
                };
                let run_id = GlyphRunId::new(
                    u32::try_from(requests.len())
                        .map_err(|_| ItemizationError::ArithmeticOverflow)?,
                );
                requests.push(
                    ShapeRequest::new(
                        run_id,
                        main,
                        &site.font_selection,
                        expected_epoch,
                        spec.bidi_level,
                        spec.script,
                        None,
                        &[],
                        pre_context,
                        post_context,
                        data_tables,
                        limits,
                    )
                    .map_err(ItemizationError::ShapeInput)?,
                );
            }
            if requests.is_empty() {
                return Err(ItemizationError::ParagraphSiteMismatch);
            }
            result.push(Some(ItemizedShapeRequests {
                text: site.text,
                paragraph_level,
                requests,
            }));
        }
        Ok(result)
    }
}

fn paragraph_site_matches(
    expected: PackageParagraphTextSite,
    actual: PackageShapeTextSource,
) -> bool {
    match (expected, actual) {
        (PackageParagraphTextSite::Parsed(expected), PackageShapeTextSource::Parsed(actual)) => {
            expected == actual
        }
        (
            PackageParagraphTextSite::Generated(expected),
            PackageShapeTextSource::Generated(actual),
        ) => expected == actual.buffer_key(),
        _ => false,
    }
}

fn previous_nonempty_view<'slice, 'text>(
    views: &'slice [ShapeTextView<'text>],
    index: usize,
) -> Option<&'slice ShapeTextView<'text>> {
    views[..index]
        .iter()
        .rev()
        .find(|view| !view.utf8().is_empty())
}

fn next_nonempty_view<'slice, 'text>(
    views: &'slice [ShapeTextView<'text>],
    index: usize,
) -> Option<&'slice ShapeTextView<'text>> {
    views
        .get(index.checked_add(1)?..)?
        .iter()
        .find(|view| !view.utf8().is_empty())
}

fn itemized_run_utf8(utf8: &str, spec: ItemizedRunSpec) -> Result<&str, ItemizationError> {
    utf8.get(
        usize::try_from(spec.start).map_err(|_| ItemizationError::ArithmeticOverflow)?
            ..usize::try_from(spec.end).map_err(|_| ItemizationError::ArithmeticOverflow)?,
    )
    .ok_or(ItemizationError::ArithmeticOverflow)
}

fn validate_itemizer_identities(
    package: &ValidatedParsedPackage,
    computed: &PackageComputedStyle,
    text: PackageShapeTextReceipt<'_>,
    font_selection: &ShapeFontSelectionReceipt<'_>,
    expected_epoch: LayoutEpoch,
) -> Result<(), ItemizationError> {
    if package.epoch_identity().document() != expected_epoch.document()
        || package.epoch_identity().style() != expected_epoch.style()
    {
        return Err(ItemizationError::PackageIdentityMismatch);
    }
    if computed.document_fingerprint() != expected_epoch.document()
        || computed.style_fingerprint() != expected_epoch.style()
        || computed.owner() != font_selection.style().owner()
        || computed.style_owner() != font_selection.style().style_owner()
    {
        return Err(ItemizationError::StyleIdentityMismatch);
    }
    if !font_selection.matches_epoch(expected_epoch)
        || !font_selection.style().matches_epoch(expected_epoch)
    {
        return Err(ItemizationError::FontSelectionIdentityMismatch);
    }
    if text.document_fingerprint() != expected_epoch.document()
        || text.site_owner() != computed.owner()
        || text.style_owner() != computed.style_owner()
    {
        return Err(ItemizationError::TextIdentityMismatch);
    }
    match (text.source(), text.reference_fingerprint()) {
        (PackageShapeTextSource::Parsed(_), None) => {}
        (PackageShapeTextSource::Generated(_), Some(reference))
            if reference == expected_epoch.references() => {}
        (PackageShapeTextSource::Generated(_), _) => {
            return Err(ItemizationError::GeneratedReferenceMismatch)
        }
        (PackageShapeTextSource::Parsed(_), Some(_)) => {
            return Err(ItemizationError::TextIdentityMismatch)
        }
    }
    Ok(())
}

fn validate_itemizer_unicode_tables(
    data_tables: &ResolvedDataTables,
) -> Result<(), ItemizationError> {
    const UNICODE_16: (u64, u64, u64) = (16, 0, 0);
    if unicode_bidi::UNICODE_VERSION != UNICODE_16
        || unicode_script::UNICODE_VERSION != UNICODE_16
        || unicode_segmentation::UNICODE_VERSION != UNICODE_16
        || data_tables.versions().unicode() != "16.0.0"
    {
        return Err(ItemizationError::UnicodeDataVersionMismatch);
    }
    Ok(())
}

fn validate_selected_font_coverage(
    font_selection: &ShapeFontSelectionReceipt<'_>,
    utf8: &str,
) -> Result<(), ItemizationError> {
    let admitted = font_selection.admitted_font();
    let face = harfrust::FontRef::from_index(admitted.font_bytes(), admitted.face_index())
        .map_err(|_| ItemizationError::InvalidFontOrFace)?;
    let cmap = face
        .cmap()
        .map_err(|_| ItemizationError::InvalidFontOrFace)?;
    for cluster in UnicodeSegmentation::graphemes(utf8, true) {
        for character in cluster.chars() {
            if is_shaping_default_ignorable(character) {
                continue;
            }
            let Some(glyph) = cmap.map_codepoint(character) else {
                return Err(ItemizationError::MissingDeclaredFontCoverage);
            };
            let glyph_id = glyph.to_u32();
            if glyph_id == 0 || glyph_id >= admitted.metadata().glyph_count {
                return Err(ItemizationError::MissingDeclaredFontCoverage);
            }
        }
    }
    Ok(())
}

fn is_shaping_default_ignorable(character: char) -> bool {
    let codepoint = u32::from(character);
    // Unicode 16.0.0 DerivedCoreProperties.txt, exact
    // Default_Ignorable_Code_Point ranges. Keep this table synchronized with
    // `validate_itemizer_unicode_tables`; font fallback must test observable
    // bases rather than demand nominal cmap entries for shaping controls.
    matches!(
        codepoint,
        0x00ad
            | 0x034f
            | 0x061c
            | 0x115f..=0x1160
            | 0x17b4..=0x17b5
            | 0x180b..=0x180f
            | 0x200b..=0x200f
            | 0x202a..=0x202e
            | 0x2060..=0x206f
            | 0x3164
            | 0xfe00..=0xfe0f
            | 0xfeff
            | 0xffa0
            | 0xfff0..=0xfff8
            | 0x1bca0..=0x1bca3
            | 0x1d173..=0x1d17a
            | 0xe0000..=0xe0fff
    )
}

fn map_linked_itemization_error(error: LinkedShaperError) -> ItemizationError {
    match error {
        LinkedShaperError::ArithmeticOverflow | LinkedShaperError::LengthConversion(_) => {
            ItemizationError::ArithmeticOverflow
        }
        LinkedShaperError::AllocationFailure => ItemizationError::AllocationFailure,
        _ => ItemizationError::BackendOutputLimit,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ItemizedRunSpec {
    start: u32,
    end: u32,
    bidi_level: BidiLevel,
    script: OpenTypeTag,
}

#[derive(Clone, Copy)]
struct GraphemeItem {
    start: u32,
    end: u32,
    bidi_level: BidiLevel,
    candidates: ScriptExtension,
}

fn itemize_run_specs(utf8: &str) -> Result<(BidiLevel, Vec<ItemizedRunSpec>), ItemizationError> {
    if utf8
        .chars()
        .any(|character| unicode_bidi::bidi_class(character) == unicode_bidi::BidiClass::B)
    {
        return Err(ItemizationError::ParagraphBoundaryUnsupported);
    }
    let bidi = unicode_bidi::BidiInfo::new(utf8, None);
    if bidi.levels.len() != utf8.len() {
        return Err(ItemizationError::ArithmeticOverflow);
    }
    let paragraph = match bidi.paragraphs.as_slice() {
        [paragraph] if paragraph.range == (0..utf8.len()) => paragraph,
        _ => return Err(ItemizationError::ParagraphBoundaryUnsupported),
    };
    let paragraph_level = BidiLevel::new(paragraph.level.number())
        .filter(|level| level.get() <= 1)
        .ok_or(ItemizationError::UnsupportedBidiLevel)?;
    let grapheme_count = UnicodeSegmentation::graphemes(utf8, true).count();
    let mut graphemes = Vec::new();
    graphemes
        .try_reserve_exact(grapheme_count)
        .map_err(|_| ItemizationError::AllocationFailure)?;
    for (start, cluster) in UnicodeSegmentation::grapheme_indices(utf8, true) {
        let end = start
            .checked_add(cluster.len())
            .ok_or(ItemizationError::ArithmeticOverflow)?;
        let level = *bidi
            .levels
            .get(start)
            .ok_or(ItemizationError::ArithmeticOverflow)?;
        if bidi.levels[start..end]
            .iter()
            .any(|candidate| *candidate != level)
        {
            return Err(ItemizationError::BidiLevelSplitsGrapheme);
        }
        let candidates = ScriptExtension::for_str(cluster);
        if candidates.is_empty() {
            return Err(ItemizationError::UnsupportedScriptCluster);
        }
        graphemes.push(GraphemeItem {
            start: u32::try_from(start).map_err(|_| ItemizationError::ArithmeticOverflow)?,
            end: u32::try_from(end).map_err(|_| ItemizationError::ArithmeticOverflow)?,
            bidi_level: BidiLevel::new(level.number())
                .ok_or(ItemizationError::UnsupportedBidiLevel)?,
            candidates,
        });
    }

    let mut runs: Vec<ItemizedRunSpec> = Vec::new();
    runs.try_reserve_exact(graphemes.len())
        .map_err(|_| ItemizationError::AllocationFailure)?;
    let mut group_start = 0;
    while group_start < graphemes.len() {
        let bidi_level = graphemes[group_start].bidi_level;
        let mut group_end = group_start + 1;
        let mut intersection = graphemes[group_start].candidates;
        while group_end < graphemes.len() {
            let next = graphemes[group_end];
            if next.bidi_level != bidi_level {
                break;
            }
            let narrowed = intersection.intersection(next.candidates);
            if narrowed.is_empty() {
                break;
            }
            intersection = narrowed;
            group_end += 1;
        }
        let script = resolve_script_intersection(intersection)?;
        let script_bytes: [u8; 4] = script
            .short_name()
            .as_bytes()
            .try_into()
            .map_err(|_| ItemizationError::InvalidScriptTag)?;
        let script = OpenTypeTag::new(script_bytes).ok_or(ItemizationError::InvalidScriptTag)?;
        runs.push(ItemizedRunSpec {
            start: graphemes[group_start].start,
            end: graphemes[group_end - 1].end,
            bidi_level,
            script,
        });
        group_start = group_end;
    }
    Ok((paragraph_level, runs))
}

fn resolve_script_intersection(
    candidates: ScriptExtension,
) -> Result<UnicodeScriptValue, ItemizationError> {
    if candidates.is_common() {
        Ok(UnicodeScriptValue::Common)
    } else if candidates.is_inherited() {
        Ok(UnicodeScriptValue::Inherited)
    } else if candidates.len() == 1 {
        candidates
            .iter()
            .next()
            .ok_or(ItemizationError::UnsupportedScriptCluster)
    } else {
        Err(ItemizationError::AmbiguousScriptCluster)
    }
}

fn narrow_text_view<'a>(
    whole: &ShapeTextView<'a>,
    start: u32,
    end: u32,
) -> Result<ShapeTextView<'a>, ItemizationError> {
    if start >= end {
        return Err(ItemizationError::ArithmeticOverflow);
    }
    let utf8 = whole
        .utf8
        .get(
            usize::try_from(start).map_err(|_| ItemizationError::ArithmeticOverflow)?
                ..usize::try_from(end).map_err(|_| ItemizationError::ArithmeticOverflow)?,
        )
        .ok_or(ItemizationError::ArithmeticOverflow)?;
    let source = source_subspan(whole.source, start, end)
        .map_err(|_| ItemizationError::ArithmeticOverflow)?;
    Ok(ShapeTextView {
        source,
        utf8,
        site_owner: whole.site_owner,
        style_owner: whole.style_owner,
        document: whole.document,
        reference: whole.reference,
    })
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

pub const EQUATION_NUMBER_SHAPE_ALGORITHM: &str = "typaxis.equation-number-shape/1";
const EQUATION_NUMBER_GLYPH_RECEIPT_ALGORITHM: &str = "typaxis.equation-number-glyphs/1";
const EQUATION_NUMBER_UNICODE_VERSION: &str = "16.0.0";

/// One itemized glyph run inside an atomic, nonwrapping equation-number line.
/// The selected font is owned by the enclosing receipt, so a run cannot
/// introduce an independent fallback face.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingEquationNumberGlyphRun {
    run_id: GlyphRunId,
    bidi_level: BidiLevel,
    script: OpenTypeTag,
    source_span: TextSpan,
    glyphs: Vec<ShapedGlyph>,
    clusters: Vec<ShapedCluster>,
}

impl StagingEquationNumberGlyphRun {
    pub const fn run_id(&self) -> GlyphRunId {
        self.run_id
    }

    pub const fn bidi_level(&self) -> BidiLevel {
        self.bidi_level
    }

    pub const fn script(&self) -> OpenTypeTag {
        self.script
    }

    pub const fn source_span(&self) -> TextSpan {
        self.source_span
    }

    pub fn glyphs(&self) -> &[ShapedGlyph] {
        &self.glyphs
    }

    pub fn clusters(&self) -> &[ShapedCluster] {
        &self.clusters
    }
}

/// Sealed one-line shape receipt for a producer-authored equation number.
///
/// The receipt deliberately contains no image identity, SVG hash, source-TeX
/// hash, formula alternative, or formula ActualText. Those remain in the
/// independently verified math-vector binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingEquationNumberShapeReceipt {
    owner: NodeId,
    node_id: NodeId,
    source_span: SourceSpan,
    text_span: TextSpan,
    text_buffer_sha256: [u8; 32],
    exact_text: String,
    exact_text_sha256: [u8; 32],
    computed_style_fingerprint: [u8; 32],
    layout_epoch_fingerprint: [u8; 32],
    owner_language: String,
    owner_language_fingerprint: [u8; 32],
    font_face_id: FontFaceId,
    font_family: String,
    font_sha256: [u8; 32],
    face_index: u32,
    font_size: PositiveLength,
    line_height: PositiveLength,
    paragraph_level: BidiLevel,
    shaper_backend: &'static str,
    shaper_version: &'static str,
    unicode_version: &'static str,
    runs: Vec<StagingEquationNumberGlyphRun>,
    width: PositiveLength,
    glyph_receipt_fingerprint: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingEquationNumberShapeReceipt {
    pub const fn algorithm(&self) -> &'static str {
        EQUATION_NUMBER_SHAPE_ALGORITHM
    }

    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }

    pub const fn text_span(&self) -> TextSpan {
        self.text_span
    }

    pub const fn text_buffer_sha256(&self) -> [u8; 32] {
        self.text_buffer_sha256
    }

    pub fn exact_text(&self) -> &str {
        &self.exact_text
    }

    pub const fn exact_text_sha256(&self) -> [u8; 32] {
        self.exact_text_sha256
    }

    pub const fn computed_style_fingerprint(&self) -> [u8; 32] {
        self.computed_style_fingerprint
    }

    pub const fn layout_epoch_fingerprint(&self) -> [u8; 32] {
        self.layout_epoch_fingerprint
    }

    pub fn owner_language(&self) -> &str {
        &self.owner_language
    }

    pub const fn owner_language_fingerprint(&self) -> [u8; 32] {
        self.owner_language_fingerprint
    }

    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }

    pub fn font_family(&self) -> &str {
        &self.font_family
    }

    pub const fn font_sha256(&self) -> [u8; 32] {
        self.font_sha256
    }

    pub const fn face_index(&self) -> u32 {
        self.face_index
    }

    pub const fn font_size(&self) -> PositiveLength {
        self.font_size
    }

    pub const fn line_height(&self) -> PositiveLength {
        self.line_height
    }

    pub const fn paragraph_level(&self) -> BidiLevel {
        self.paragraph_level
    }

    pub const fn shaper_backend(&self) -> &'static str {
        self.shaper_backend
    }

    pub const fn shaper_version(&self) -> &'static str {
        self.shaper_version
    }

    pub const fn unicode_version(&self) -> &'static str {
        self.unicode_version
    }

    pub fn runs(&self) -> &[StagingEquationNumberGlyphRun] {
        &self.runs
    }

    pub const fn width(&self) -> PositiveLength {
        self.width
    }

    pub const fn height(&self) -> PositiveLength {
        self.line_height
    }

    pub const fn glyph_receipt_fingerprint(&self) -> [u8; 32] {
        self.glyph_receipt_fingerprint
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn integrity_matches(&self) -> bool {
        let Some(width) = equation_number_runs_width(&self.runs) else {
            return false;
        };
        let Some(expected_node_id) = self.owner.get().checked_add(1) else {
            return false;
        };
        let Some(text_span_len) = self
            .text_span
            .end_byte()
            .get()
            .checked_sub(self.text_span.start_byte().get())
        else {
            return false;
        };
        let glyphs = encode_equation_number_glyph_receipt(&self.runs);
        let canonical = encode_equation_number_shape_receipt(self);
        self.exact_text_sha256 == sha256(self.exact_text.as_bytes())
            && self.node_id.get() == expected_node_id
            && u32::try_from(self.exact_text.len()) == Ok(text_span_len)
            && !self.exact_text.is_empty()
            && !self.owner_language.is_empty()
            && self.owner_language_fingerprint != [0; 32]
            && self.layout_epoch_fingerprint != [0; 32]
            && !self.font_family.is_empty()
            && self.paragraph_level.get() <= 1
            && self.shaper_backend == ShaperIdentity::linked_reference().backend()
            && self.shaper_version == ShaperIdentity::linked_reference().version()
            && self.unicode_version == EQUATION_NUMBER_UNICODE_VERSION
            && self.width == width
            && self.line_height.get().raw() > 0
            && equation_number_runs_cover(self.text_span, &self.runs)
            && self.glyph_receipt_fingerprint == sha256(glyphs.as_bytes())
            && self.canonical_jcs == canonical
            && self.fingerprint == sha256(canonical.as_bytes())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingEquationNumberShapeError {
    ReceiptMismatch,
    MissingComputedTextStyle,
    MissingSelectedFont,
    MissingDeclaredFontCoverage,
    RequiresSecondLine,
    NonPositiveShape,
    ContextLimit,
    InvalidFontOrFace,
    Backend(LinkedShaperError),
    AllocationFailure,
    ArithmeticOverflow,
}

impl std::fmt::Display for StagingEquationNumberShapeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReceiptMismatch => formatter.write_str("equation-number receipt mismatch"),
            Self::MissingComputedTextStyle => {
                formatter.write_str("equation-number computed text style is incomplete")
            }
            Self::MissingSelectedFont => {
                formatter.write_str("equation-number selected font is unavailable")
            }
            Self::MissingDeclaredFontCoverage => {
                formatter.write_str("equation-number selected font lacks required glyph coverage")
            }
            Self::RequiresSecondLine => {
                formatter.write_str("equation number is not one nonwrapping line")
            }
            Self::NonPositiveShape => {
                formatter.write_str("equation-number shape has non-positive dimensions")
            }
            Self::ContextLimit => formatter.write_str("equation-number shaping limit exceeded"),
            Self::InvalidFontOrFace => {
                formatter.write_str("equation-number selected font face is invalid")
            }
            Self::Backend(error) => {
                write!(
                    formatter,
                    "equation-number shaping backend failed: {error:?}"
                )
            }
            Self::AllocationFailure => {
                formatter.write_str("equation-number shaping allocation failed")
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("equation-number shaping arithmetic overflow")
            }
        }
    }
}

impl std::error::Error for StagingEquationNumberShapeError {}

/// Shapes the optional equation-number child of one validated
/// `math_vector_block` as exactly one nonwrapping logical line.
///
/// This staging bridge uses the same Unicode itemizer, coverage check, linked
/// HarfRust backend, output budget, cluster validation, and fixed-point
/// scaling as ordinary text shaping. The caller supplies only the syntax-
/// issued owner-language receipt and the vector layout epoch that the
/// enclosing layout receipt will rederive and verify.
pub fn shape_staging_equation_number(
    package: &ValidatedStagingSemanticPackage,
    metrics: &ValidatedPrecomposedVectorMetrics,
    admitted: &AdmittedResourceLedger,
    layout_epoch_fingerprint: [u8; 32],
    owner_language: &ValidatedPrecomposedVectorEffectiveLanguage,
) -> Result<Option<StagingEquationNumberShapeReceipt>, StagingEquationNumberShapeError> {
    package
        .verify_precomposed_vector_metrics(metrics)
        .map_err(|_| StagingEquationNumberShapeError::ReceiptMismatch)?;
    if metrics.kind() != PrecomposedVectorKind::MathVectorBlock {
        return Err(StagingEquationNumberShapeError::ReceiptMismatch);
    }
    package
        .verify_precomposed_vector_effective_language(owner_language)
        .map_err(|_| StagingEquationNumberShapeError::ReceiptMismatch)?;
    if owner_language.owner() != metrics.node_id()
        || owner_language.kind() != PrecomposedVectorKind::MathVectorBlock
    {
        return Err(StagingEquationNumberShapeError::ReceiptMismatch);
    }
    if unicode_bidi::UNICODE_VERSION != (16, 0, 0)
        || unicode_script::UNICODE_VERSION != (16, 0, 0)
        || unicode_segmentation::UNICODE_VERSION != (16, 0, 0)
    {
        return Err(StagingEquationNumberShapeError::ReceiptMismatch);
    }
    let Some(number) = metrics.equation_number() else {
        return Ok(None);
    };
    let owner = metrics.node_id();
    let style = package
        .precomposed_vector_style(owner)
        .ok_or(StagingEquationNumberShapeError::ReceiptMismatch)?;
    package
        .verify_precomposed_vector_style(style)
        .map_err(|_| StagingEquationNumberShapeError::ReceiptMismatch)?;
    let number_style = style
        .equation_number_text_style()
        .ok_or(StagingEquationNumberShapeError::ReceiptMismatch)?;
    let families = number_style
        .font_families()
        .ok_or(StagingEquationNumberShapeError::MissingComputedTextStyle)?;
    let font_size = number_style
        .font_size()
        .ok_or(StagingEquationNumberShapeError::MissingComputedTextStyle)?;
    let line_height = number_style
        .line_height()
        .ok_or(StagingEquationNumberShapeError::MissingComputedTextStyle)?;

    let declarations = staging_declared_base_catalog(package.resources())
        .map_err(|_| StagingEquationNumberShapeError::ReceiptMismatch)?;
    if !admitted.matches_declarations(declarations.resource_catalog()) {
        return Err(StagingEquationNumberShapeError::ReceiptMismatch);
    }
    let font_face_id = admitted
        .font_families()
        .resolve(families)
        .map_err(|_| StagingEquationNumberShapeError::MissingSelectedFont)?;
    let font = admitted
        .font(font_face_id)
        .ok_or(StagingEquationNumberShapeError::MissingSelectedFont)?;

    let text_span = number.text().text_span();
    let wire = package
        .checked_wire()
        .map_err(|_| StagingEquationNumberShapeError::ReceiptMismatch)?;
    let buffer = wire
        .text_buffers()
        .iter()
        .find(|buffer| buffer.text_id == text_span.text_id().get())
        .ok_or(StagingEquationNumberShapeError::ReceiptMismatch)?;
    let start = usize::try_from(text_span.start_byte().get())
        .map_err(|_| StagingEquationNumberShapeError::ArithmeticOverflow)?;
    let end = usize::try_from(text_span.end_byte().get())
        .map_err(|_| StagingEquationNumberShapeError::ArithmeticOverflow)?;
    let exact_text = buffer
        .utf8
        .get(start..end)
        .ok_or(StagingEquationNumberShapeError::ReceiptMismatch)?;
    if sha256(buffer.utf8.as_bytes()) != number.text().text_buffer_sha256()
        || sha256(exact_text.as_bytes()) != number.text().exact_text_sha256()
        || layout_epoch_fingerprint == [0; 32]
    {
        return Err(StagingEquationNumberShapeError::ReceiptMismatch);
    }
    if exact_text.chars().any(|character| {
        matches!(character, '\u{2028}' | '\u{2029}')
            || unicode_bidi::bidi_class(character) == unicode_bidi::BidiClass::B
    }) {
        return Err(StagingEquationNumberShapeError::RequiresSecondLine);
    }
    let exact_len = u32::try_from(exact_text.len())
        .map_err(|_| StagingEquationNumberShapeError::ArithmeticOverflow)?;
    let maximum_records = package.limits().get().max_shaping_context_bytes;
    if exact_len == 0 || exact_len > maximum_records {
        return Err(StagingEquationNumberShapeError::ContextLimit);
    }
    validate_admitted_font_coverage(font, exact_text)?;
    let (paragraph_level, specs) =
        itemize_run_specs(exact_text).map_err(map_equation_number_itemization_error)?;
    if specs.is_empty() {
        return Err(StagingEquationNumberShapeError::NonPositiveShape);
    }

    let mut runs = Vec::new();
    runs.try_reserve_exact(specs.len())
        .map_err(|_| StagingEquationNumberShapeError::AllocationFailure)?;
    for (index, spec) in specs.into_iter().enumerate() {
        let run_text =
            itemized_run_utf8(exact_text, spec).map_err(map_equation_number_itemization_error)?;
        let backend_bound = linked_backend_record_bound(run_text)
            .map_err(StagingEquationNumberShapeError::Backend)?;
        if backend_bound > maximum_records {
            return Err(StagingEquationNumberShapeError::ContextLimit);
        }
        let run_id = GlyphRunId::new(
            u32::try_from(index)
                .map_err(|_| StagingEquationNumberShapeError::ArithmeticOverflow)?,
        );
        let source = source_subspan(ShapeSourceSpan::Parsed(text_span), spec.start, spec.end)
            .map_err(StagingEquationNumberShapeError::Backend)?;
        let mut budget = ShapeOutputBudget::new(maximum_records);
        let raw = shape_linked(
            LinkedBackendInput {
                run_id,
                font: FontInstanceId::new(font_face_id.get()),
                source,
                utf8: run_text,
                font_bytes: font.bytes(),
                face_index: font.face_index(),
                admitted_units_per_em: font.metadata().units_per_em,
                admitted_glyph_count: font.metadata().glyph_count,
                font_size: font_size.get(),
                bidi_level: spec.bidi_level,
                script: spec.script,
                pre_context: if spec.start == 0 {
                    None
                } else {
                    exact_text.get(
                        ..usize::try_from(spec.start)
                            .map_err(|_| StagingEquationNumberShapeError::ArithmeticOverflow)?,
                    )
                },
                post_context: if spec.end == exact_len {
                    None
                } else {
                    exact_text.get(
                        usize::try_from(spec.end)
                            .map_err(|_| StagingEquationNumberShapeError::ArithmeticOverflow)?..,
                    )
                },
                max_output_records: maximum_records,
            },
            &mut budget,
        )
        .map_err(StagingEquationNumberShapeError::Backend)?;
        if !budget.matches_output(&raw) {
            return Err(StagingEquationNumberShapeError::ReceiptMismatch);
        }
        let expected = ExpectedGlyphRun {
            run_id,
            font: FontInstanceId::new(font_face_id.get()),
            bidi_level: spec.bidi_level,
            source,
            utf8_boundaries: utf8_boundaries(source, run_text)
                .ok_or(StagingEquationNumberShapeError::ReceiptMismatch)?,
            glyph_count: font.metadata().glyph_count,
            max_output_records: maximum_records,
        };
        validate_glyph_run(&expected, &raw)
            .map_err(|_| StagingEquationNumberShapeError::ReceiptMismatch)?;
        let ShapeSourceSpan::Parsed(source_span) = raw.source_span else {
            return Err(StagingEquationNumberShapeError::ReceiptMismatch);
        };
        runs.push(StagingEquationNumberGlyphRun {
            run_id: raw.run_id,
            bidi_level: raw.bidi_level,
            script: spec.script,
            source_span,
            glyphs: raw.glyphs,
            clusters: raw.clusters,
        });
    }

    let width = equation_number_runs_width(&runs)
        .ok_or(StagingEquationNumberShapeError::NonPositiveShape)?;
    let glyph_jcs = encode_equation_number_glyph_receipt(&runs);
    let shaper = ShaperIdentity::linked_reference();
    let mut receipt = StagingEquationNumberShapeReceipt {
        owner,
        node_id: number.node_id(),
        source_span: number.span(),
        text_span,
        text_buffer_sha256: number.text().text_buffer_sha256(),
        exact_text: exact_text.to_owned(),
        exact_text_sha256: number.text().exact_text_sha256(),
        computed_style_fingerprint: style.fingerprint(),
        layout_epoch_fingerprint,
        owner_language: owner_language.language().to_owned(),
        owner_language_fingerprint: owner_language.fingerprint(),
        font_face_id,
        font_family: font.family().to_owned(),
        font_sha256: font.content_hash(),
        face_index: font.face_index(),
        font_size,
        line_height,
        paragraph_level,
        shaper_backend: shaper.backend(),
        shaper_version: shaper.version(),
        unicode_version: EQUATION_NUMBER_UNICODE_VERSION,
        runs,
        width,
        glyph_receipt_fingerprint: sha256(glyph_jcs.as_bytes()),
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs = encode_equation_number_shape_receipt(&receipt);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    if !receipt.integrity_matches() {
        return Err(StagingEquationNumberShapeError::ReceiptMismatch);
    }
    Ok(Some(receipt))
}

fn validate_admitted_font_coverage(
    admitted: &AdmittedFont,
    utf8: &str,
) -> Result<(), StagingEquationNumberShapeError> {
    let face = harfrust::FontRef::from_index(admitted.bytes(), admitted.face_index())
        .map_err(|_| StagingEquationNumberShapeError::InvalidFontOrFace)?;
    let cmap = face
        .cmap()
        .map_err(|_| StagingEquationNumberShapeError::InvalidFontOrFace)?;
    for cluster in UnicodeSegmentation::graphemes(utf8, true) {
        for character in cluster.chars() {
            if is_shaping_default_ignorable(character) {
                continue;
            }
            let Some(glyph) = cmap.map_codepoint(character) else {
                return Err(StagingEquationNumberShapeError::MissingDeclaredFontCoverage);
            };
            let glyph_id = glyph.to_u32();
            if glyph_id == 0 || glyph_id >= admitted.metadata().glyph_count {
                return Err(StagingEquationNumberShapeError::MissingDeclaredFontCoverage);
            }
        }
    }
    Ok(())
}

fn map_equation_number_itemization_error(
    error: ItemizationError,
) -> StagingEquationNumberShapeError {
    match error {
        ItemizationError::ParagraphBoundaryUnsupported => {
            StagingEquationNumberShapeError::RequiresSecondLine
        }
        ItemizationError::ContextLimit | ItemizationError::BackendOutputLimit => {
            StagingEquationNumberShapeError::ContextLimit
        }
        ItemizationError::MissingDeclaredFontCoverage => {
            StagingEquationNumberShapeError::MissingDeclaredFontCoverage
        }
        ItemizationError::InvalidFontOrFace => StagingEquationNumberShapeError::InvalidFontOrFace,
        ItemizationError::AllocationFailure => StagingEquationNumberShapeError::AllocationFailure,
        ItemizationError::ArithmeticOverflow => StagingEquationNumberShapeError::ArithmeticOverflow,
        _ => StagingEquationNumberShapeError::ReceiptMismatch,
    }
}

fn equation_number_runs_width(runs: &[StagingEquationNumberGlyphRun]) -> Option<PositiveLength> {
    let raw = runs.iter().try_fold(0i64, |total, run| {
        run.glyphs.iter().try_fold(total, |total, glyph| {
            total.checked_add(glyph.advance_x.raw())
        })
    })?;
    Length::from_raw(raw).and_then(PositiveLength::new)
}

fn equation_number_runs_cover(text_span: TextSpan, runs: &[StagingEquationNumberGlyphRun]) -> bool {
    let mut expected_start = text_span.start_byte().get();
    for (index, run) in runs.iter().enumerate() {
        if usize::try_from(run.run_id.get()) != Ok(index)
            || run.source_span.text_id() != text_span.text_id()
            || run.source_span.start_byte().get() != expected_start
            || run.source_span.end_byte().get() <= expected_start
            || run.glyphs.is_empty()
            || run.clusters.is_empty()
            || run.bidi_level.get() > 1
        {
            return false;
        }
        let mut expected_cluster_start = run.source_span.start_byte().get();
        for cluster in &run.clusters {
            let ShapeSourceSpan::Parsed(cluster_span) = cluster.source_span else {
                return false;
            };
            if cluster_span.text_id() != text_span.text_id()
                || cluster_span.start_byte().get() != expected_cluster_start
                || cluster_span.end_byte().get() <= expected_cluster_start
                || cluster_span.end_byte().get() > run.source_span.end_byte().get()
                || cluster.glyph_start >= cluster.glyph_end
                || usize::try_from(cluster.glyph_end)
                    .map_or(true, |glyph_end| glyph_end > run.glyphs.len())
            {
                return false;
            }
            expected_cluster_start = cluster_span.end_byte().get();
        }
        if expected_cluster_start != run.source_span.end_byte().get() {
            return false;
        }
        expected_start = run.source_span.end_byte().get();
    }
    expected_start == text_span.end_byte().get()
}

fn encode_equation_number_glyph_receipt(runs: &[StagingEquationNumberGlyphRun]) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, EQUATION_NUMBER_GLYPH_RECEIPT_ALGORITHM);
    output.push_str(",\"runs\":[");
    for (run_index, run) in runs.iter().enumerate() {
        if run_index > 0 {
            output.push(',');
        }
        output.push_str("{\"bidi_level\":");
        output.push_str(&run.bidi_level.get().to_string());
        output.push_str(",\"clusters\":[");
        for (cluster_index, cluster) in run.clusters.iter().enumerate() {
            if cluster_index > 0 {
                output.push(',');
            }
            output.push_str("{\"glyph_end\":");
            output.push_str(&cluster.glyph_end.to_string());
            output.push_str(",\"glyph_start\":");
            output.push_str(&cluster.glyph_start.to_string());
            output.push_str(",\"source_span\":");
            push_shape_source_span(&mut output, cluster.source_span);
            output.push('}');
        }
        output.push_str("],\"glyphs\":[");
        for (glyph_index, glyph) in run.glyphs.iter().enumerate() {
            if glyph_index > 0 {
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
        output.push_str("],\"run_id\":");
        output.push_str(&run.run_id.get().to_string());
        output.push_str(",\"script\":");
        let script_bytes = run.script.bytes();
        let script = String::from_utf8_lossy(&script_bytes);
        push_jcs_string(&mut output, &script);
        output.push_str(",\"source_span\":");
        push_text_span(&mut output, run.source_span);
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn encode_equation_number_shape_receipt(value: &StagingEquationNumberShapeReceipt) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, EQUATION_NUMBER_SHAPE_ALGORITHM);
    output.push_str(",\"computed_style_fingerprint\":");
    push_shape_hash(&mut output, value.computed_style_fingerprint);
    output.push_str(",\"exact_text_sha256\":");
    push_shape_hash(&mut output, value.exact_text_sha256);
    output.push_str(",\"face_index\":");
    output.push_str(&value.face_index.to_string());
    output.push_str(",\"font_face_id\":");
    output.push_str(&value.font_face_id.get().to_string());
    output.push_str(",\"font_family\":");
    push_jcs_string(&mut output, &value.font_family);
    output.push_str(",\"font_sha256\":");
    push_shape_hash(&mut output, value.font_sha256);
    output.push_str(",\"font_size\":");
    output.push_str(&value.font_size.get().raw().to_string());
    output.push_str(",\"glyph_receipt_fingerprint\":");
    push_shape_hash(&mut output, value.glyph_receipt_fingerprint);
    output.push_str(",\"height\":");
    output.push_str(&value.line_height.get().raw().to_string());
    output.push_str(",\"layout_epoch_fingerprint\":");
    push_shape_hash(&mut output, value.layout_epoch_fingerprint);
    output.push_str(",\"line_count\":1,\"node_id\":");
    output.push_str(&value.node_id.get().to_string());
    output.push_str(",\"nonwrapping\":true,\"owner\":");
    output.push_str(&value.owner.get().to_string());
    output.push_str(",\"owner_language\":");
    push_jcs_string(&mut output, &value.owner_language);
    output.push_str(",\"owner_language_fingerprint\":");
    push_shape_hash(&mut output, value.owner_language_fingerprint);
    output.push_str(",\"paragraph_level\":");
    output.push_str(&value.paragraph_level.get().to_string());
    output.push_str(",\"shaper_backend\":");
    push_jcs_string(&mut output, value.shaper_backend);
    output.push_str(",\"shaper_version\":");
    push_jcs_string(&mut output, value.shaper_version);
    output.push_str(",\"source_span\":");
    push_source_span(&mut output, value.source_span);
    output.push_str(",\"text_buffer_sha256\":");
    push_shape_hash(&mut output, value.text_buffer_sha256);
    output.push_str(",\"text_span\":");
    push_text_span(&mut output, value.text_span);
    output.push_str(",\"unicode_version\":");
    push_jcs_string(&mut output, value.unicode_version);
    output.push_str(",\"width\":");
    output.push_str(&value.width.get().raw().to_string());
    output.push('}');
    output
}

fn push_shape_source_span(output: &mut String, value: ShapeSourceSpan) {
    match value {
        ShapeSourceSpan::Parsed(value) => push_text_span(output, value),
        ShapeSourceSpan::Generated(_) => output.push_str("null"),
    }
}

fn push_text_span(output: &mut String, value: TextSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&value.end_byte().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&value.start_byte().get().to_string());
    output.push_str(",\"text_id\":");
    output.push_str(&value.text_id().get().to_string());
    output.push('}');
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

fn push_shape_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
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
    fn reserve_linked_capacity(&mut self, count: u32) -> Result<(), ShapeWorkError> {
        let remaining_glyphs = self
            .remaining_glyphs
            .checked_sub(count)
            .ok_or(ShapeWorkError::GlyphLimit)?;
        let remaining_clusters = self
            .remaining_clusters
            .checked_sub(count)
            .ok_or(ShapeWorkError::ClusterLimit)?;
        self.remaining_glyphs = remaining_glyphs;
        self.remaining_clusters = remaining_clusters;
        Ok(())
    }
    fn commit_linked_output(
        &mut self,
        reserved: u32,
        glyphs: u32,
        clusters: u32,
    ) -> Result<(), ShapeWorkError> {
        let unused_glyphs = reserved
            .checked_sub(glyphs)
            .ok_or(ShapeWorkError::GlyphLimit)?;
        let unused_clusters = reserved
            .checked_sub(clusters)
            .ok_or(ShapeWorkError::ClusterLimit)?;
        let remaining_glyphs = self
            .remaining_glyphs
            .checked_add(unused_glyphs)
            .filter(|remaining| *remaining <= self.limit)
            .ok_or(ShapeWorkError::GlyphLimit)?;
        let remaining_clusters = self
            .remaining_clusters
            .checked_add(unused_clusters)
            .filter(|remaining| *remaining <= self.limit)
            .ok_or(ShapeWorkError::ClusterLimit)?;
        self.remaining_glyphs = remaining_glyphs;
        self.remaining_clusters = remaining_clusters;
        Ok(())
    }
    fn matches_output(&self, run: &GlyphRun) -> bool {
        let Ok(glyphs) = u32::try_from(run.glyphs.len()) else {
            return false;
        };
        let Ok(clusters) = u32::try_from(run.clusters.len()) else {
            return false;
        };
        self.limit - self.remaining_glyphs == glyphs
            && self.limit - self.remaining_clusters == clusters
    }
}

/// Errors emitted by the linked, build-selected shaping backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkedShaperError {
    InvalidFontOrFace,
    FontMetadataMismatch,
    InvalidScript,
    EmptyBackendOutput,
    InconsistentBackendOutput,
    GlyphIdOutOfRange,
    SourceLengthMismatch,
    InvalidBackendCluster,
    NonContiguousBackendCluster,
    NonMonotoneBackendClusters,
    AllocationFailure,
    ArithmeticOverflow,
    LengthConversion(LengthError),
    OutputBudget(ShapeWorkError),
}

/// The Profile 1.0 linked shaper. It uses the exact admitted face bytes and
/// face index carried by `ShapeRequest`; callers cannot replace either input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LinkedShaper;
impl LinkedShaper {
    pub const fn new() -> Self {
        Self
    }
}
impl shaper_seal::Sealed for LinkedShaper {}
impl Shaper for LinkedShaper {
    type Error = LinkedShaperError;

    fn identity(&self) -> ShaperIdentity {
        ShaperIdentity::linked_reference()
    }

    fn shape(
        &self,
        request: ShapeRequest<'_>,
        budget: &mut ShapeOutputBudget,
    ) -> Result<GlyphRun, Self::Error> {
        let admitted_font = request.font_selection().admitted_font();
        let input = LinkedBackendInput {
            run_id: request.run_id(),
            font: request.font(),
            source: request.text().source(),
            utf8: request.text().utf8(),
            font_bytes: admitted_font.font_bytes(),
            face_index: admitted_font.face_index(),
            admitted_units_per_em: admitted_font.metadata().units_per_em,
            admitted_glyph_count: admitted_font.metadata().glyph_count,
            font_size: request
                .font_selection()
                .style()
                .resolved()
                .font_size()
                .get(),
            bidi_level: request.bidi_level(),
            script: request.script(),
            pre_context: request.pre_context().map(ShapeTextView::utf8),
            post_context: request.post_context().map(ShapeTextView::utf8),
            max_output_records: request.max_output_records(),
        };
        shape_linked(input, budget)
    }
}

#[derive(Clone, Copy)]
struct LinkedBackendInput<'a> {
    run_id: GlyphRunId,
    font: FontInstanceId,
    source: ShapeSourceSpan,
    utf8: &'a str,
    font_bytes: &'a [u8],
    face_index: u32,
    admitted_units_per_em: u16,
    admitted_glyph_count: u32,
    font_size: Length,
    bidi_level: BidiLevel,
    script: OpenTypeTag,
    pre_context: Option<&'a str>,
    post_context: Option<&'a str>,
    max_output_records: u32,
}

#[derive(Clone, Copy)]
struct BackendCluster {
    source_start: u32,
    glyph_start: u32,
    glyph_end: u32,
}

// These are part of the reviewed harfrust 0.1.1 integration contract. That
// exact version assigns max(input scalar count * 64, 16_384) to its private
// glyph-buffer record ceiling before shaping. The Cargo pin must not change
// without reviewing these constants and this preflight together.
const HARFRUST_MAX_LEN_FACTOR: u32 = 64;
const HARFRUST_MAX_LEN_MIN: u32 = 16_384;

fn linked_backend_record_bound(utf8: &str) -> Result<u32, LinkedShaperError> {
    let scalar_count =
        u32::try_from(utf8.chars().count()).map_err(|_| LinkedShaperError::ArithmeticOverflow)?;
    scalar_count
        .checked_mul(HARFRUST_MAX_LEN_FACTOR)
        .map(|bound| bound.max(HARFRUST_MAX_LEN_MIN))
        .ok_or(LinkedShaperError::ArithmeticOverflow)
}

fn shape_linked(
    input: LinkedBackendInput<'_>,
    budget: &mut ShapeOutputBudget,
) -> Result<GlyphRun, LinkedShaperError> {
    let face = harfrust::FontRef::from_index(input.font_bytes, input.face_index)
        .map_err(|_| LinkedShaperError::InvalidFontOrFace)?;
    let head = face
        .head()
        .map_err(|_| LinkedShaperError::InvalidFontOrFace)?;
    let maxp = face
        .maxp()
        .map_err(|_| LinkedShaperError::InvalidFontOrFace)?;
    face.cmap()
        .map_err(|_| LinkedShaperError::InvalidFontOrFace)?;
    face.hmtx()
        .map_err(|_| LinkedShaperError::InvalidFontOrFace)?;
    if head.units_per_em() != input.admitted_units_per_em
        || u32::from(maxp.num_glyphs()) != input.admitted_glyph_count
    {
        return Err(LinkedShaperError::FontMetadataMismatch);
    }
    let script = harfrust::Script::from_iso15924_tag(harfrust::Tag::new(&input.script.bytes()))
        .ok_or(LinkedShaperError::InvalidScript)?;
    let text_len =
        u32::try_from(input.utf8.len()).map_err(|_| LinkedShaperError::ArithmeticOverflow)?;
    let (source_start, source_end) = source_range(input.source);
    if source_end.checked_sub(source_start) != Some(text_len) {
        return Err(LinkedShaperError::SourceLengthMismatch);
    }
    let backend_record_bound = linked_backend_record_bound(input.utf8)?;
    if backend_record_bound > input.max_output_records {
        return Err(LinkedShaperError::OutputBudget(ShapeWorkError::GlyphLimit));
    }

    // Reserve the complete request-bound output allowance before harfrust
    // builds its caches, allocates its input buffer, or performs shaping. The
    // preflight above proves this allowance also covers harfrust's private
    // glyph-record ceiling. The unused portion is returned only after the
    // exact output counts are known.
    budget
        .reserve_linked_capacity(input.max_output_records)
        .map_err(LinkedShaperError::OutputBudget)?;

    let shaper_data = harfrust::ShaperData::new(&face);
    let shaper = shaper_data.shaper(&face).build();
    let mut unicode = harfrust::UnicodeBuffer::new();
    unicode.push_str(input.utf8);
    unicode.set_direction(if input.bidi_level.is_rtl() {
        harfrust::Direction::RightToLeft
    } else {
        harfrust::Direction::LeftToRight
    });
    unicode.set_script(script);
    unicode.set_cluster_level(harfrust::BufferClusterLevel::MonotoneGraphemes);
    let mut flags = harfrust::BufferFlags::empty();
    match input.pre_context {
        Some(context) => unicode.set_pre_context(context),
        None => flags.insert(harfrust::BufferFlags::BEGINNING_OF_TEXT),
    }
    match input.post_context {
        Some(context) => unicode.set_post_context(context),
        None => flags.insert(harfrust::BufferFlags::END_OF_TEXT),
    }
    unicode.set_flags(flags);

    let shaped = shaper.shape(unicode, &[]);
    let infos = shaped.glyph_infos();
    let positions = shaped.glyph_positions();
    if infos.is_empty() || positions.is_empty() {
        return Err(LinkedShaperError::EmptyBackendOutput);
    }
    if infos.len() != positions.len() {
        return Err(LinkedShaperError::InconsistentBackendOutput);
    }
    let glyph_count = u32::try_from(infos.len())
        .map_err(|_| LinkedShaperError::OutputBudget(ShapeWorkError::GlyphLimit))?;
    if glyph_count > input.max_output_records {
        return Err(LinkedShaperError::OutputBudget(ShapeWorkError::GlyphLimit));
    }

    let mut visual_clusters: Vec<BackendCluster> = Vec::new();
    visual_clusters
        .try_reserve_exact(infos.len())
        .map_err(|_| LinkedShaperError::AllocationFailure)?;
    for (index, info) in infos.iter().enumerate() {
        let glyph_start =
            u32::try_from(index).map_err(|_| LinkedShaperError::ArithmeticOverflow)?;
        let glyph_end = glyph_start
            .checked_add(1)
            .ok_or(LinkedShaperError::ArithmeticOverflow)?;
        if info.cluster >= text_len
            || !input.utf8.is_char_boundary(
                usize::try_from(info.cluster)
                    .map_err(|_| LinkedShaperError::InvalidBackendCluster)?,
            )
        {
            return Err(LinkedShaperError::InvalidBackendCluster);
        }
        match visual_clusters.last_mut() {
            Some(previous) if previous.source_start == info.cluster => {
                previous.glyph_end = glyph_end;
            }
            _ => visual_clusters.push(BackendCluster {
                source_start: info.cluster,
                glyph_start,
                glyph_end,
            }),
        }
    }
    if visual_clusters.windows(2).any(|pair| {
        if input.bidi_level.is_rtl() {
            pair[0].source_start <= pair[1].source_start
        } else {
            pair[0].source_start >= pair[1].source_start
        }
    }) {
        return Err(LinkedShaperError::NonMonotoneBackendClusters);
    }

    let mut logical_clusters = Vec::new();
    logical_clusters
        .try_reserve_exact(visual_clusters.len())
        .map_err(|_| LinkedShaperError::AllocationFailure)?;
    logical_clusters.extend_from_slice(&visual_clusters);
    logical_clusters.sort_unstable_by_key(|cluster| cluster.source_start);
    if logical_clusters.first().map(|cluster| cluster.source_start) != Some(0) {
        return Err(LinkedShaperError::InvalidBackendCluster);
    }
    if logical_clusters
        .windows(2)
        .any(|pair| pair[0].source_start == pair[1].source_start)
    {
        return Err(LinkedShaperError::NonContiguousBackendCluster);
    }
    let cluster_count = u32::try_from(logical_clusters.len())
        .map_err(|_| LinkedShaperError::OutputBudget(ShapeWorkError::ClusterLimit))?;
    if cluster_count > input.max_output_records {
        return Err(LinkedShaperError::OutputBudget(
            ShapeWorkError::ClusterLimit,
        ));
    }

    let mut glyphs = Vec::new();
    glyphs
        .try_reserve_exact(infos.len())
        .map_err(|_| LinkedShaperError::AllocationFailure)?;
    for (info, position) in infos.iter().zip(positions) {
        let glyph_id =
            u16::try_from(info.glyph_id).map_err(|_| LinkedShaperError::GlyphIdOutOfRange)?;
        if u32::from(glyph_id) >= input.admitted_glyph_count {
            return Err(LinkedShaperError::GlyphIdOutOfRange);
        }
        glyphs.push(ShapedGlyph {
            original_gid: OriginalGlyphId::new(glyph_id),
            advance_x: scale_design_units(
                position.x_advance,
                input.font_size,
                input.admitted_units_per_em,
            )?,
            advance_y: scale_design_units(
                position.y_advance,
                input.font_size,
                input.admitted_units_per_em,
            )?,
            offset_x: scale_design_units(
                position.x_offset,
                input.font_size,
                input.admitted_units_per_em,
            )?,
            offset_y: scale_design_units(
                position.y_offset,
                input.font_size,
                input.admitted_units_per_em,
            )?,
        });
    }

    let mut clusters = Vec::new();
    clusters
        .try_reserve_exact(logical_clusters.len())
        .map_err(|_| LinkedShaperError::AllocationFailure)?;
    for (index, cluster) in logical_clusters.iter().enumerate() {
        let relative_end = logical_clusters
            .get(index + 1)
            .map_or(text_len, |next| next.source_start);
        if cluster.source_start >= relative_end {
            return Err(LinkedShaperError::InvalidBackendCluster);
        }
        clusters.push(ShapedCluster {
            source_span: source_subspan(input.source, cluster.source_start, relative_end)?,
            glyph_start: cluster.glyph_start,
            glyph_end: cluster.glyph_end,
        });
    }

    budget
        .commit_linked_output(input.max_output_records, glyph_count, cluster_count)
        .map_err(LinkedShaperError::OutputBudget)?;
    Ok(GlyphRun {
        run_id: input.run_id,
        font: input.font,
        bidi_level: input.bidi_level,
        source_span: input.source,
        glyphs,
        clusters,
    })
}

fn scale_design_units(
    design_units: i32,
    font_size: Length,
    units_per_em: u16,
) -> Result<Length, LinkedShaperError> {
    let numerator = i128::from(design_units)
        .checked_mul(i128::from(font_size.raw()))
        .ok_or(LinkedShaperError::ArithmeticOverflow)?;
    let denominator = i128::from(units_per_em)
        .checked_mul(65_536)
        .ok_or(LinkedShaperError::ArithmeticOverflow)?;
    Length::from_rational_pdf_points(numerator, denominator)
        .map_err(LinkedShaperError::LengthConversion)
}

fn source_subspan(
    source: ShapeSourceSpan,
    relative_start: u32,
    relative_end: u32,
) -> Result<ShapeSourceSpan, LinkedShaperError> {
    let (base_start, base_end) = source_range(source);
    let start = base_start
        .checked_add(relative_start)
        .ok_or(LinkedShaperError::ArithmeticOverflow)?;
    let end = base_start
        .checked_add(relative_end)
        .ok_or(LinkedShaperError::ArithmeticOverflow)?;
    if start >= end || end > base_end {
        return Err(LinkedShaperError::InvalidBackendCluster);
    }
    let start = Utf8ByteOffset::new(start);
    let end = Utf8ByteOffset::new(end);
    match source {
        ShapeSourceSpan::Parsed(span) => TextSpan::new(span.text_id(), start, end)
            .map(ShapeSourceSpan::Parsed)
            .ok_or(LinkedShaperError::InvalidBackendCluster),
        ShapeSourceSpan::Generated(provenance) => provenance
            .subspan(start, end)
            .map(ShapeSourceSpan::Generated)
            .ok_or(LinkedShaperError::InvalidBackendCluster),
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShapingCacheError<E> {
    Shape(ShapeExecutionError<E>),
    InvalidCachedValue(GlyphRunValidationError),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ShapeCacheSourceIdentity {
    Parsed {
        text_id: u32,
        start: u32,
        end: u32,
    },
    Generated {
        key: GeneratedBufferKey,
        start: u32,
        end: u32,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ShapeCacheTextFact {
    source: ShapeCacheSourceIdentity,
    site_owner: NodeId,
    style_owner: NodeId,
    document: [u8; 32],
    reference: Option<[u8; 32]>,
    utf8: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ShapeCacheKey {
    epoch_document: [u8; 32],
    epoch_style: [u8; 32],
    epoch_admitted_resources: [u8; 32],
    epoch_references: [u8; 32],
    run_id: GlyphRunId,
    font_instance: FontInstanceId,
    admitted_sha256: [u8; 32],
    face_index: u32,
    units_per_em: u16,
    admitted_glyph_count: u32,
    font_size_raw: i64,
    text: ShapeCacheTextFact,
    pre_context: Option<ShapeCacheTextFact>,
    post_context: Option<ShapeCacheTextFact>,
    bidi_level: BidiLevel,
    right_to_left: bool,
    script: OpenTypeTag,
    language: Option<String>,
    features: Vec<(OpenTypeTag, u32)>,
    shaper_backend: String,
    shaper_version: String,
    unicode_version: String,
    japanese_line_break_version: String,
    max_output_records: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ShapeCacheEntry {
    key: ShapeCacheKey,
    run: GlyphRun,
}

/// Deterministic in-process shaping cache. Entries are kept in canonical key
/// order, so lookup and final state do not depend on request insertion order.
#[derive(Debug, Eq, PartialEq)]
pub struct ShapingCache {
    entries: Vec<ShapeCacheEntry>,
    max_owned_bytes: u64,
    owned_bytes: u64,
}
impl ShapingCache {
    /// Creates a cache whose persistent logical storage is capped by the
    /// package's validated aggregate text budget. A miss that cannot reserve
    /// its full worst-case persistent allowance is shaped and validated
    /// without insertion, so cache pressure never rejects valid input.
    pub const fn new(limits: &ValidatedResourceLimits) -> Self {
        Self {
            entries: Vec::new(),
            max_owned_bytes: limits.get().max_text_bytes,
            owned_bytes: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub const fn owned_bytes(&self) -> u64 {
        self.owned_bytes
    }
    pub const fn max_owned_bytes(&self) -> u64 {
        self.max_owned_bytes
    }

    fn has_persistent_capacity(&self, additional_bytes: u64) -> bool {
        self.max_owned_bytes
            .checked_sub(self.owned_bytes)
            .is_some_and(|remaining| additional_bytes <= remaining)
    }

    fn find(&self, key: &ShapeCacheKey) -> Result<usize, usize> {
        self.entries.binary_search_by(|entry| entry.key.cmp(key))
    }

    pub fn shape<S: Shaper>(
        &mut self,
        shaper: &S,
        request: ShapeRequest<'_>,
    ) -> Result<ValidatedGlyphRun, ShapingCacheError<S::Error>> {
        let Some(persistent_key_bytes) =
            persistent_shape_cache_key_bytes(shaper.identity(), &request)
        else {
            return shape_validated(shaper, request).map_err(ShapingCacheError::Shape);
        };
        if persistent_key_bytes > self.max_owned_bytes {
            return shape_validated(shaper, request).map_err(ShapingCacheError::Shape);
        }
        let Ok(key) = ShapeCacheKey::from_request(shaper.identity(), &request) else {
            return shape_validated(shaper, request).map_err(ShapingCacheError::Shape);
        };
        match self.find(&key) {
            Ok(index) => {
                let expected =
                    authorize_cached_run(shaper.identity(), &request, &self.entries[index].run)
                        .map_err(ShapingCacheError::InvalidCachedValue)?;
                let Ok(run) = try_clone_glyph_run(&self.entries[index].run) else {
                    return shape_validated(shaper, request).map_err(ShapingCacheError::Shape);
                };
                finish_validated_run(expected, run).map_err(ShapingCacheError::InvalidCachedValue)
            }
            Err(index) => {
                let Some(worst_entry_bytes) = persistent_shape_cache_output_bound(&request)
                    .and_then(|output| persistent_key_bytes.checked_add(output))
                else {
                    return shape_validated(shaper, request).map_err(ShapingCacheError::Shape);
                };
                if !self.has_persistent_capacity(worst_entry_bytes) {
                    return shape_validated(shaper, request).map_err(ShapingCacheError::Shape);
                }
                if self.entries.try_reserve(1).is_err() {
                    return shape_validated(shaper, request).map_err(ShapingCacheError::Shape);
                }
                let validated =
                    shape_validated(shaper, request).map_err(ShapingCacheError::Shape)?;
                let Some(entry_bytes) = persistent_shape_cache_output_bytes(validated.run())
                    .and_then(|output| persistent_key_bytes.checked_add(output))
                    .filter(|entry| *entry <= worst_entry_bytes)
                else {
                    return Ok(validated);
                };
                let Some(next_owned_bytes) = self.owned_bytes.checked_add(entry_bytes) else {
                    return Ok(validated);
                };
                let Ok(run) = try_clone_glyph_run(validated.run()) else {
                    return Ok(validated);
                };
                self.entries.insert(index, ShapeCacheEntry { key, run });
                self.owned_bytes = next_owned_bytes;
                Ok(validated)
            }
        }
    }
}

fn persistent_shape_cache_key_bytes(
    shaper: ShaperIdentity,
    request: &ShapeRequest<'_>,
) -> Option<u64> {
    let string_bytes = [
        request.text().utf8().len(),
        request.pre_context().map_or(0, |view| view.utf8().len()),
        request.post_context().map_or(0, |view| view.utf8().len()),
        request.language().map_or(0, str::len),
        shaper.backend().len(),
        shaper.version().len(),
        request.data_tables().versions().unicode().len(),
        request.data_tables().versions().japanese_line_break().len(),
    ]
    .into_iter()
    .try_fold(0u64, |total, bytes| {
        total.checked_add(u64::try_from(bytes).ok()?)
    })?;
    let feature_bytes = u64::try_from(request.features().len())
        .ok()?
        .checked_mul(u64::try_from(core::mem::size_of::<(OpenTypeTag, u32)>()).ok()?)?;
    u64::try_from(core::mem::size_of::<ShapeCacheEntry>())
        .ok()?
        .checked_add(string_bytes)?
        .checked_add(feature_bytes)
}

fn persistent_shape_cache_output_bound(request: &ShapeRequest<'_>) -> Option<u64> {
    let records = u64::from(request.max_output_records());
    let glyph_bytes =
        records.checked_mul(u64::try_from(core::mem::size_of::<ShapedGlyph>()).ok()?)?;
    let cluster_bytes =
        records.checked_mul(u64::try_from(core::mem::size_of::<ShapedCluster>()).ok()?)?;
    glyph_bytes.checked_add(cluster_bytes)
}

fn persistent_shape_cache_output_bytes(run: &GlyphRun) -> Option<u64> {
    let glyph_bytes = u64::try_from(run.glyphs.len())
        .ok()?
        .checked_mul(u64::try_from(core::mem::size_of::<ShapedGlyph>()).ok()?)?;
    let cluster_bytes = u64::try_from(run.clusters.len())
        .ok()?
        .checked_mul(u64::try_from(core::mem::size_of::<ShapedCluster>()).ok()?)?;
    glyph_bytes.checked_add(cluster_bytes)
}

impl ShapeCacheKey {
    fn from_request(shaper: ShaperIdentity, request: &ShapeRequest<'_>) -> Result<Self, ()> {
        let admitted = request.font_selection().admitted_font();
        let mut features = Vec::new();
        features
            .try_reserve_exact(request.features().len())
            .map_err(|_| ())?;
        features.extend(
            request
                .features()
                .iter()
                .map(|feature| (feature.tag, feature.value)),
        );
        let epoch = request.layout_epoch();
        Ok(Self {
            epoch_document: epoch.document().bytes(),
            epoch_style: epoch.style().bytes(),
            epoch_admitted_resources: epoch.admitted_resources().bytes(),
            epoch_references: epoch.references().bytes(),
            run_id: request.run_id(),
            font_instance: request.font(),
            admitted_sha256: admitted.admitted_sha256(),
            face_index: admitted.face_index(),
            units_per_em: admitted.metadata().units_per_em,
            admitted_glyph_count: admitted.metadata().glyph_count,
            font_size_raw: request
                .font_selection()
                .style()
                .resolved()
                .font_size()
                .get()
                .raw(),
            text: ShapeCacheTextFact::from_view(request.text())?,
            pre_context: request
                .pre_context()
                .map(ShapeCacheTextFact::from_view)
                .transpose()?,
            post_context: request
                .post_context()
                .map(ShapeCacheTextFact::from_view)
                .transpose()?,
            bidi_level: request.bidi_level(),
            right_to_left: request.bidi_level().is_rtl(),
            script: request.script(),
            language: request.language().map(try_copy_string).transpose()?,
            features,
            shaper_backend: try_copy_string(shaper.backend())?,
            shaper_version: try_copy_string(shaper.version())?,
            unicode_version: try_copy_string(request.data_tables().versions().unicode())?,
            japanese_line_break_version: try_copy_string(
                request.data_tables().versions().japanese_line_break(),
            )?,
            max_output_records: request.max_output_records(),
        })
    }
}

impl ShapeCacheTextFact {
    fn from_view(view: &ShapeTextView<'_>) -> Result<Self, ()> {
        Ok(Self {
            source: ShapeCacheSourceIdentity::from_source(view.source()),
            site_owner: view.site_owner(),
            style_owner: view.style_owner(),
            document: view.document.bytes(),
            reference: view.reference.map(|reference| reference.bytes()),
            utf8: try_copy_string(view.utf8())?,
        })
    }
}

impl ShapeCacheSourceIdentity {
    fn from_source(source: ShapeSourceSpan) -> Self {
        match source {
            ShapeSourceSpan::Parsed(span) => Self::Parsed {
                text_id: span.text_id().get(),
                start: span.start_byte().get(),
                end: span.end_byte().get(),
            },
            ShapeSourceSpan::Generated(provenance) => {
                let range = provenance.text_span().range();
                Self::Generated {
                    key: provenance.buffer_key(),
                    start: range.start_byte().get(),
                    end: range.end_byte().get(),
                }
            }
        }
    }
}

fn try_copy_string(value: &str) -> Result<String, ()> {
    let mut output = String::new();
    output.try_reserve_exact(value.len()).map_err(|_| ())?;
    output.push_str(value);
    Ok(output)
}

fn try_clone_glyph_run(run: &GlyphRun) -> Result<GlyphRun, ()> {
    let mut glyphs = Vec::new();
    glyphs.try_reserve_exact(run.glyphs.len()).map_err(|_| ())?;
    glyphs.extend(run.glyphs.iter().cloned());
    let mut clusters = Vec::new();
    clusters
        .try_reserve_exact(run.clusters.len())
        .map_err(|_| ())?;
    clusters.extend(run.clusters.iter().cloned());
    Ok(GlyphRun {
        run_id: run.run_id,
        font: run.font,
        bidi_level: run.bidi_level,
        source_span: run.source_span,
        glyphs,
        clusters,
    })
}

/// Shaper output checked against the exact request and admitted font metadata.
/// Downstream line layout accepts this receipt rather than a raw `GlyphRun`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedGlyphRun {
    run: GlyphRun,
    font_face_id: typaxis_core::FontFaceId,
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
    pub const fn font_face_id(&self) -> typaxis_core::FontFaceId {
        self.font_face_id
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
    let expected = expected_shape_output(shaper.identity(), &request)
        .map_err(ShapeExecutionError::InvalidOutput)?;
    let mut budget = ShapeOutputBudget::new(expected.glyph_run.max_output_records);
    let run = shaper
        .shape(request, &mut budget)
        .map_err(ShapeExecutionError::Backend)?;
    if !budget.matches_output(&run) {
        return Err(ShapeExecutionError::InvalidOutput(
            GlyphRunValidationError::WorkReceiptMismatch,
        ));
    }
    finish_validated_run(expected, run).map_err(ShapeExecutionError::InvalidOutput)
}

fn expected_shape_output(
    shaper: ShaperIdentity,
    request: &ShapeRequest<'_>,
) -> Result<ExpectedShapeOutput, GlyphRunValidationError> {
    Ok(ExpectedShapeOutput {
        glyph_run: ExpectedGlyphRun {
            run_id: request.run_id(),
            font: request.font(),
            bidi_level: request.bidi_level(),
            source: request.text().source(),
            utf8_boundaries: utf8_boundaries(request.text().source(), request.text().utf8())
                .ok_or(GlyphRunValidationError::InvalidClusterUtf8Boundary)?,
            glyph_count: request
                .font_selection()
                .admitted_font()
                .metadata()
                .glyph_count,
            max_output_records: request.max_output_records(),
        },
        epoch: request.layout_epoch(),
        font_face_id: request.font_selection().admitted_font().font_face_id(),
        site_owner: request.text().site_owner(),
        style_owner: request.text().style_owner(),
        shaper,
        data_tables: request.data_tables().clone(),
    })
}

fn authorize_cached_run(
    shaper: ShaperIdentity,
    request: &ShapeRequest<'_>,
    run: &GlyphRun,
) -> Result<ExpectedShapeOutput, GlyphRunValidationError> {
    let expected = expected_shape_output(shaper, request)?;
    authorize_cached_glyph_run(&expected.glyph_run, run)?;
    Ok(expected)
}

fn authorize_cached_glyph_run(
    expected: &ExpectedGlyphRun,
    run: &GlyphRun,
) -> Result<(), GlyphRunValidationError> {
    let glyph_count = u32::try_from(run.glyphs.len())
        .map_err(|_| GlyphRunValidationError::WorkReceiptMismatch)?;
    let cluster_count = u32::try_from(run.clusters.len())
        .map_err(|_| GlyphRunValidationError::WorkReceiptMismatch)?;
    let mut budget = ShapeOutputBudget::new(expected.max_output_records);
    budget
        .reserve_glyphs(glyph_count)
        .and_then(|()| budget.reserve_clusters(cluster_count))
        .map_err(|_| GlyphRunValidationError::WorkReceiptMismatch)?;
    if !budget.matches_output(run) {
        return Err(GlyphRunValidationError::WorkReceiptMismatch);
    }
    validate_glyph_run(expected, run)
}

fn finish_validated_run(
    expected: ExpectedShapeOutput,
    run: GlyphRun,
) -> Result<ValidatedGlyphRun, GlyphRunValidationError> {
    validate_glyph_run(&expected.glyph_run, &run)?;
    Ok(ValidatedGlyphRun {
        run,
        font_face_id: expected.font_face_id,
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
    font_face_id: typaxis_core::FontFaceId,
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
        DocumentFingerprint, NodeId, PortablePath, ReferenceFingerprint, ResourceLimits, SourceId,
        TextBufferId, Utf8ByteOffset,
    };
    use typaxis_syntax::{
        PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
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

    fn push_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn push_i16(bytes: &mut Vec<u8>, value: i16) {
        bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn test_cmap() -> Vec<u8> {
        let mut cmap = Vec::new();
        // A Windows Unicode full-repertoire encoding record pointing at a
        // format-12 subtable.
        push_u16(&mut cmap, 0);
        push_u16(&mut cmap, 1);
        push_u16(&mut cmap, 3);
        push_u16(&mut cmap, 10);
        push_u32(&mut cmap, 12);
        push_u16(&mut cmap, 12);
        push_u16(&mut cmap, 0);
        push_u32(&mut cmap, 40);
        push_u32(&mut cmap, 0);
        push_u32(&mut cmap, 2);
        // a..b => glyphs 1..2.
        push_u32(&mut cmap, u32::from('a'));
        push_u32(&mut cmap, u32::from('b'));
        push_u32(&mut cmap, 1);
        // Hebrew alef..bet => glyphs 3..4.
        push_u32(&mut cmap, u32::from('\u{05d0}'));
        push_u32(&mut cmap, u32::from('\u{05d1}'));
        push_u32(&mut cmap, 3);
        cmap
    }

    fn test_head() -> Vec<u8> {
        let mut head = Vec::new();
        push_u16(&mut head, 1);
        push_u16(&mut head, 0);
        push_u32(&mut head, 0x0001_0000);
        push_u32(&mut head, 0);
        push_u32(&mut head, 0x5f0f_3cf5);
        push_u16(&mut head, 0);
        push_u16(&mut head, 1_000);
        head.extend_from_slice(&[0; 16]);
        for value in [0, 0, 750, 800] {
            push_i16(&mut head, value);
        }
        push_u16(&mut head, 0);
        push_u16(&mut head, 8);
        push_i16(&mut head, 2);
        push_i16(&mut head, 0);
        push_i16(&mut head, 0);
        assert_eq!(head.len(), 54);
        head
    }

    fn test_hhea() -> Vec<u8> {
        let mut hhea = Vec::new();
        push_u32(&mut hhea, 0x0001_0000);
        for value in [800, -200, 0, 750, 0, 0, 750, 1, 0, 0, 0, 0, 0, 0, 0] {
            push_i16(&mut hhea, value);
        }
        push_u16(&mut hhea, 5);
        assert_eq!(hhea.len(), 36);
        hhea
    }

    fn test_hmtx() -> Vec<u8> {
        let mut hmtx = Vec::new();
        for advance in [500, 500, 750, 500, 750] {
            push_u16(&mut hmtx, advance);
            push_i16(&mut hmtx, 0);
        }
        hmtx
    }

    fn test_maxp() -> Vec<u8> {
        let mut maxp = Vec::new();
        push_u32(&mut maxp, 0x0000_5000);
        push_u16(&mut maxp, 5);
        maxp
    }

    /// Builds the smallest deterministic SFNT needed by the linked shaper.
    /// Keeping it synthetic avoids a platform font dependency in the tests.
    fn test_font() -> Vec<u8> {
        let tables = [
            (*b"cmap", test_cmap()),
            (*b"head", test_head()),
            (*b"hhea", test_hhea()),
            (*b"hmtx", test_hmtx()),
            (*b"maxp", test_maxp()),
        ];
        let directory_len = 12usize + tables.len() * 16;
        let mut next_offset = directory_len;
        let mut records = Vec::new();
        for (tag, table) in &tables {
            next_offset = (next_offset + 3) & !3;
            records.push((
                *tag,
                u32::try_from(next_offset).expect("bounded fixture offset"),
                u32::try_from(table.len()).expect("bounded fixture table"),
            ));
            next_offset += table.len();
        }

        let mut font = Vec::new();
        push_u32(&mut font, 0x0001_0000);
        push_u16(&mut font, 5);
        push_u16(&mut font, 64);
        push_u16(&mut font, 2);
        push_u16(&mut font, 16);
        for (tag, offset, length) in &records {
            font.extend_from_slice(tag);
            push_u32(&mut font, 0); // Checksums are not admission inputs.
            push_u32(&mut font, *offset);
            push_u32(&mut font, *length);
        }
        for ((_, table), (_, offset, _)) in tables.iter().zip(&records) {
            let offset = usize::try_from(*offset).expect("bounded fixture offset");
            font.resize(offset, 0);
            font.extend_from_slice(table);
        }
        font
    }

    fn linked_input<'a>(
        font_bytes: &'a [u8],
        utf8: &'a str,
        source_start: u32,
        bidi_level: u8,
        script: [u8; 4],
        max_output_records: u32,
    ) -> LinkedBackendInput<'a> {
        let text_len = u32::try_from(utf8.len()).expect("bounded test text");
        let source_end = source_start
            .checked_add(text_len)
            .expect("bounded test span");
        LinkedBackendInput {
            run_id: GlyphRunId::new(17),
            font: FontInstanceId::new(23),
            source: ShapeSourceSpan::Parsed(
                TextSpan::new(
                    TextBufferId::new(29),
                    Utf8ByteOffset::new(source_start),
                    Utf8ByteOffset::new(source_end),
                )
                .expect("valid test span"),
            ),
            utf8,
            font_bytes,
            face_index: 0,
            admitted_units_per_em: 1_000,
            admitted_glyph_count: 5,
            font_size: Length::from_raw(65_536).expect("one PDF point"),
            bidi_level: BidiLevel::new(bidi_level).expect("valid bidi level"),
            script: OpenTypeTag::new(script).expect("valid script"),
            pre_context: None,
            post_context: None,
            max_output_records,
        }
    }

    fn expected_linked(input: LinkedBackendInput<'_>) -> ExpectedGlyphRun {
        ExpectedGlyphRun {
            run_id: input.run_id,
            font: input.font,
            bidi_level: input.bidi_level,
            source: input.source,
            utf8_boundaries: utf8_boundaries(input.source, input.utf8)
                .expect("valid fixture boundaries"),
            glyph_count: input.admitted_glyph_count,
            max_output_records: input.max_output_records,
        }
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
    fn text_identity_rejects_an_actual_foreign_package_receipt() {
        fn package(uri: &str, text: &str) -> Box<ValidatedParsedPackage> {
            let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
            let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
            let outcome = ReferenceParser::new().parse(
                &SourceFile {
                    source_id: SourceId::new(0),
                    uri: PortablePath::new(uri).unwrap(),
                    text: format!("text:{text}"),
                },
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            );
            let ParseOutcome::Parsed { package, .. } = outcome else {
                panic!("fixture must parse")
            };
            package
        }

        let local = package("local.tsf", "a");
        let foreign = package("foreign.tsf", "b");
        let span = TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(1),
        )
        .unwrap();
        let foreign_receipt = foreign.bind_parsed_shape_text(span).unwrap();
        let foreign_view = ShapeTextView::from_package_receipt(foreign_receipt);
        assert_eq!(
            validate_text_views(
                [&foreign_view],
                local.epoch_identity().document(),
                ReferenceFingerprint::from_untrusted_bytes([7; 32]),
                foreign_view.style_owner(),
            ),
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

    fn itemization_facts(utf8: &str) -> Vec<(u32, u32, u8, [u8; 4])> {
        itemize_run_specs(utf8)
            .expect("test text must itemize")
            .1
            .into_iter()
            .map(|run| (run.start, run.end, run.bidi_level.get(), run.script.bytes()))
            .collect()
    }

    #[test]
    fn itemizer_derives_ltr_rtl_and_mixed_script_runs_in_logical_order() {
        assert_eq!(itemize_run_specs("abc").unwrap().0, BidiLevel::LTR);
        assert_eq!(
            itemize_run_specs("\u{05d0}\u{05d1}").unwrap().0,
            BidiLevel::RTL
        );
        assert_eq!(itemization_facts("abc"), vec![(0, 3, 0, *b"Latn")]);
        assert_eq!(
            itemization_facts("\u{05d0}\u{05d1}"),
            vec![(0, 4, 1, *b"Hebr")]
        );
        assert_eq!(
            itemization_facts("ab \u{05d0}\u{05d1}"),
            vec![(0, 3, 0, *b"Latn"), (3, 7, 1, *b"Hebr")]
        );
        assert_eq!(
            itemization_facts("\u{05d0}\u{05d1} ab"),
            vec![(0, 5, 1, *b"Hebr"), (5, 7, 2, *b"Latn")]
        );
    }

    #[test]
    fn itemizer_keeps_graphemes_whole_and_scopes_script_resolution_to_level_runs() {
        assert_eq!(itemization_facts("a\u{0301}"), vec![(0, 3, 0, *b"Latn")]);
        assert_eq!(
            itemization_facts("\u{30ab}\u{30fc}"),
            vec![(0, 6, 0, *b"Kana")]
        );
        assert_eq!(
            itemization_facts("a-\u{05d0}"),
            vec![(0, 2, 0, *b"Latn"), (2, 4, 1, *b"Hebr")]
        );
    }

    #[test]
    fn itemizer_fails_closed_for_unknown_script_and_paragraph_boundaries() {
        assert_eq!(
            itemize_run_specs("\u{0378}"),
            Err(ItemizationError::UnsupportedScriptCluster)
        );
        assert_eq!(
            itemize_run_specs("\u{30fc}"),
            Err(ItemizationError::AmbiguousScriptCluster)
        );
        assert_eq!(
            itemize_run_specs("a\u{2029}"),
            Err(ItemizationError::ParagraphBoundaryUnsupported)
        );
        assert_eq!(
            itemize_run_specs("a\nb"),
            Err(ItemizationError::ParagraphBoundaryUnsupported)
        );

        let mut excessive_level = "\u{202b}".repeat(63);
        excessive_level.push('a');
        excessive_level.push_str(&"\u{202c}".repeat(63));
        assert_eq!(
            itemize_run_specs(&excessive_level),
            Err(ItemizationError::UnsupportedBidiLevel)
        );
    }

    #[test]
    fn canonical_context_views_are_exact_touching_logical_neighbors() {
        let document = DocumentFingerprint::from_untrusted_bytes([1; 32]);
        let owner = NodeId::new(7);
        let whole_span = TextSpan::new(
            TextBufferId::new(3),
            Utf8ByteOffset::new(10),
            Utf8ByteOffset::new(17),
        )
        .unwrap();
        let whole = ShapeTextView {
            source: ShapeSourceSpan::Parsed(whole_span),
            utf8: "ab \u{05d0}\u{05d1}",
            site_owner: NodeId::new(8),
            style_owner: owner,
            document,
            reference: None,
        };
        let (paragraph_level, specs) = itemize_run_specs(whole.utf8()).unwrap();
        assert_eq!(paragraph_level, BidiLevel::new(0).unwrap());
        assert_eq!(specs.len(), 2);
        let main = narrow_text_view(&whole, specs[1].start, specs[1].end).unwrap();
        let pre = narrow_text_view(&whole, 0, specs[1].start).unwrap();
        assert_eq!(main.utf8(), "\u{05d0}\u{05d1}");
        assert_eq!(pre.utf8(), "ab ");
        assert_eq!(source_range(pre.source()), (10, 13));
        assert_eq!(source_range(main.source()), (13, 17));
        assert_eq!(pre.style_owner(), main.style_owner());
        assert_eq!(pre.site_owner(), main.site_owner());
    }

    #[test]
    fn font_coverage_exempts_exact_unicode_16_default_ignorables() {
        for character in [
            '\u{00ad}',
            '\u{034f}',
            '\u{061c}',
            '\u{115f}',
            '\u{180f}',
            '\u{200d}',
            '\u{2066}',
            '\u{3164}',
            '\u{fe0f}',
            '\u{ffa0}',
            '\u{1bca0}',
            '\u{1d173}',
            '\u{e0100}',
        ] {
            assert!(is_shaping_default_ignorable(character));
        }
        for character in ['a', '\u{0301}', '\u{05d0}', '\u{13430}'] {
            assert!(!is_shaping_default_ignorable(character));
        }
    }

    fn cache_text_fact(utf8: &str) -> ShapeCacheTextFact {
        ShapeCacheTextFact {
            source: ShapeCacheSourceIdentity::Parsed {
                text_id: 1,
                start: 0,
                end: u32::try_from(utf8.len()).unwrap(),
            },
            site_owner: NodeId::new(2),
            style_owner: NodeId::new(3),
            document: [4; 32],
            reference: None,
            utf8: utf8.to_owned(),
        }
    }

    fn cache_key(utf8: &str) -> ShapeCacheKey {
        ShapeCacheKey {
            epoch_document: [1; 32],
            epoch_style: [2; 32],
            epoch_admitted_resources: [3; 32],
            epoch_references: [4; 32],
            run_id: GlyphRunId::new(0),
            font_instance: FontInstanceId::new(0),
            admitted_sha256: [5; 32],
            face_index: 0,
            units_per_em: 1_000,
            admitted_glyph_count: 10,
            font_size_raw: 65_536,
            text: cache_text_fact(utf8),
            pre_context: None,
            post_context: None,
            bidi_level: BidiLevel::new(0).unwrap(),
            right_to_left: false,
            script: OpenTypeTag::new(*b"Latn").unwrap(),
            language: None,
            features: vec![],
            shaper_backend: "linked".to_owned(),
            shaper_version: "1".to_owned(),
            unicode_version: "16.0.0".to_owned(),
            japanese_line_break_version: "1".to_owned(),
            max_output_records: 65_536,
        }
    }

    fn empty_cache_run(key: &ShapeCacheKey) -> GlyphRun {
        let ShapeCacheSourceIdentity::Parsed {
            text_id,
            start,
            end,
        } = key.text.source
        else {
            panic!("fixture uses parsed text")
        };
        GlyphRun {
            run_id: key.run_id,
            font: key.font_instance,
            bidi_level: key.bidi_level,
            source_span: ShapeSourceSpan::Parsed(
                TextSpan::new(
                    TextBufferId::new(text_id),
                    Utf8ByteOffset::new(start),
                    Utf8ByteOffset::new(end),
                )
                .unwrap(),
            ),
            glyphs: vec![],
            clusters: vec![],
        }
    }

    #[test]
    fn cache_key_closes_over_epoch_font_text_context_and_runtime_facts() {
        let base = cache_key("a");
        let mut variants = Vec::new();

        let mut changed = base.clone();
        changed.epoch_references[0] ^= 1;
        variants.push(changed);
        let mut changed = base.clone();
        changed.admitted_sha256[0] ^= 1;
        variants.push(changed);
        let mut changed = base.clone();
        changed.face_index = 1;
        variants.push(changed);
        let mut changed = base.clone();
        changed.text = cache_text_fact("b");
        variants.push(changed);
        let mut changed = base.clone();
        changed.pre_context = Some(cache_text_fact("p"));
        variants.push(changed);
        let mut changed = base.clone();
        changed.post_context = Some(cache_text_fact("q"));
        variants.push(changed);
        let mut changed = base.clone();
        changed.bidi_level = BidiLevel::new(1).unwrap();
        changed.right_to_left = true;
        variants.push(changed);
        let mut changed = base.clone();
        changed.script = OpenTypeTag::new(*b"Hebr").unwrap();
        variants.push(changed);
        let mut changed = base.clone();
        changed.language = Some("he".to_owned());
        variants.push(changed);
        let mut changed = base.clone();
        changed.features = vec![(OpenTypeTag::new(*b"liga").unwrap(), 1)];
        variants.push(changed);
        let mut changed = base.clone();
        changed.shaper_backend = "other".to_owned();
        variants.push(changed);
        let mut changed = base.clone();
        changed.shaper_version = "2".to_owned();
        variants.push(changed);
        let mut changed = base.clone();
        changed.unicode_version = "other".to_owned();
        variants.push(changed);
        let mut changed = base.clone();
        changed.japanese_line_break_version = "other".to_owned();
        variants.push(changed);

        assert!(variants.iter().all(|candidate| candidate != &base));
    }

    fn cache_with_insertion_order(order: [&str; 3]) -> ShapingCache {
        let mut cache = ShapingCache {
            entries: Vec::new(),
            max_owned_bytes: u64::MAX,
            owned_bytes: 0,
        };
        for utf8 in order {
            let key = cache_key(utf8);
            let index = cache.find(&key).expect_err("fixture keys are unique");
            let run = empty_cache_run(&key);
            cache.entries.insert(index, ShapeCacheEntry { key, run });
        }
        cache
    }

    #[test]
    fn cache_state_is_canonical_across_insertion_orders_and_lookup_hits() {
        let forward = cache_with_insertion_order(["a", "b", "c"]);
        let reverse = cache_with_insertion_order(["c", "b", "a"]);
        assert_eq!(forward.entries, reverse.entries);

        let hit_key = cache_key("b");
        let hit = forward.find(&hit_key).expect("inserted key must hit");
        assert_eq!(forward.entries[hit].run, empty_cache_run(&hit_key));
        assert!(forward.find(&cache_key("d")).is_err());
    }

    #[test]
    fn cache_capacity_accepts_exact_bound_and_bypasses_max_plus_one() {
        let cache = ShapingCache {
            entries: Vec::new(),
            max_owned_bytes: 100,
            owned_bytes: 40,
        };
        assert!(cache.has_persistent_capacity(60));
        assert!(!cache.has_persistent_capacity(61));
    }

    #[test]
    fn cached_output_is_budget_authorized_and_revalidated() {
        let (expected, run) = expected_multibyte_run();
        assert_eq!(authorize_cached_glyph_run(&expected, &run), Ok(()));

        let mut corrupted = run.clone();
        corrupted.run_id = GlyphRunId::new(99);
        assert_eq!(
            authorize_cached_glyph_run(&expected, &corrupted),
            Err(GlyphRunValidationError::RunIdentityMismatch)
        );

        let mut over_budget = run;
        over_budget.glyphs.push(glyph());
        over_budget.glyphs.push(glyph());
        assert_eq!(
            authorize_cached_glyph_run(&expected, &over_budget),
            Err(GlyphRunValidationError::WorkReceiptMismatch)
        );
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
    fn linked_shaper_shapes_ltr_with_scaled_positions_and_canonical_clusters() {
        let font = test_font();
        let input = linked_input(&font, "ab", 11, 0, *b"Latn", HARFRUST_MAX_LEN_MIN);
        let expected = expected_linked(input);
        let mut budget = ShapeOutputBudget::new(HARFRUST_MAX_LEN_MIN);

        let run = shape_linked(input, &mut budget).expect("valid linked shaping");

        assert_eq!(
            run.glyphs
                .iter()
                .map(|glyph| glyph.original_gid.get())
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(run.glyphs[0].advance_x.raw(), 32_768);
        assert_eq!(run.glyphs[1].advance_x.raw(), 49_152);
        assert_eq!(
            run.clusters
                .iter()
                .map(|cluster| (
                    source_range(cluster.source_span),
                    cluster.glyph_start..cluster.glyph_end
                ))
                .collect::<Vec<_>>(),
            vec![((11, 12), 0..1), ((12, 13), 1..2)]
        );
        assert!(budget.matches_output(&run));
        assert_eq!(validate_glyph_run(&expected, &run), Ok(()));
    }

    #[test]
    fn linked_shaper_keeps_rtl_glyphs_visual_and_clusters_logical() {
        let font = test_font();
        let input = linked_input(
            &font,
            "\u{05d0}\u{05d1}",
            31,
            1,
            *b"Hebr",
            HARFRUST_MAX_LEN_MIN,
        );
        let expected = expected_linked(input);
        let mut budget = ShapeOutputBudget::new(HARFRUST_MAX_LEN_MIN);

        let run = shape_linked(input, &mut budget).expect("valid linked shaping");

        assert_eq!(
            run.glyphs
                .iter()
                .map(|glyph| glyph.original_gid.get())
                .collect::<Vec<_>>(),
            vec![4, 3]
        );
        assert_eq!(
            run.clusters
                .iter()
                .map(|cluster| (
                    source_range(cluster.source_span),
                    cluster.glyph_start..cluster.glyph_end
                ))
                .collect::<Vec<_>>(),
            vec![((31, 33), 1..2), ((33, 35), 0..1)]
        );
        assert!(budget.matches_output(&run));
        assert_eq!(validate_glyph_run(&expected, &run), Ok(()));
    }

    #[test]
    fn linked_shaper_reports_font_face_metadata_and_budget_failures() {
        let font = test_font();

        let mut invalid_font = linked_input(b"not a font", "a", 0, 0, *b"Latn", 1);
        let mut budget = ShapeOutputBudget::new(1);
        assert_eq!(
            shape_linked(invalid_font, &mut budget),
            Err(LinkedShaperError::InvalidFontOrFace)
        );

        invalid_font = linked_input(&font, "a", 0, 0, *b"Latn", 1);
        invalid_font.face_index = 1;
        let mut budget = ShapeOutputBudget::new(1);
        assert_eq!(
            shape_linked(invalid_font, &mut budget),
            Err(LinkedShaperError::InvalidFontOrFace)
        );

        let mut metadata_mismatch = linked_input(&font, "a", 0, 0, *b"Latn", 1);
        metadata_mismatch.admitted_glyph_count = 6;
        let mut budget = ShapeOutputBudget::new(1);
        assert_eq!(
            shape_linked(metadata_mismatch, &mut budget),
            Err(LinkedShaperError::FontMetadataMismatch)
        );

        // The backend's reviewed minimum record ceiling cannot be reserved
        // from a smaller entrypoint budget, so shaping never starts.
        let input = linked_input(&font, "ab", 0, 0, *b"Latn", HARFRUST_MAX_LEN_MIN);
        let mut budget = ShapeOutputBudget::new(HARFRUST_MAX_LEN_MIN - 1);
        assert_eq!(
            shape_linked(input, &mut budget),
            Err(LinkedShaperError::OutputBudget(ShapeWorkError::GlyphLimit))
        );
    }

    #[test]
    fn linked_shaper_preflights_exact_backend_record_bound_and_max_plus_one() {
        let font = test_font();
        assert_eq!(linked_backend_record_bound("a"), Ok(HARFRUST_MAX_LEN_MIN));

        let exact = "a".repeat(1_024);
        assert_eq!(linked_backend_record_bound(&exact), Ok(65_536));
        let input = linked_input(&font, &exact, 0, 0, *b"Latn", 65_536);
        let mut budget = ShapeOutputBudget::new(65_536);
        let run = shape_linked(input, &mut budget).expect("exact bound is admitted");
        assert_eq!(run.glyphs.len(), 1_024);
        assert!(budget.matches_output(&run));

        let max_plus_one = "a".repeat(1_025);
        assert_eq!(linked_backend_record_bound(&max_plus_one), Ok(65_600));
        let input = linked_input(&font, &max_plus_one, 0, 0, *b"Latn", 65_536);
        let mut budget = ShapeOutputBudget::new(65_536);
        assert_eq!(
            shape_linked(input, &mut budget),
            Err(LinkedShaperError::OutputBudget(ShapeWorkError::GlyphLimit))
        );
    }

    #[test]
    fn linked_position_scaling_is_checked_and_rounds_ties_to_even() {
        let half_raw_unit = Length::from_raw(1).expect("valid test length");
        assert_eq!(scale_design_units(1, half_raw_unit, 2), Ok(Length::ZERO));
        assert_eq!(
            scale_design_units(3, half_raw_unit, 2),
            Ok(Length::from_raw(2).unwrap())
        );
        assert_eq!(scale_design_units(-1, half_raw_unit, 2), Ok(Length::ZERO));
        assert_eq!(
            scale_design_units(-3, half_raw_unit, 2),
            Ok(Length::from_raw(-2).unwrap())
        );
        assert_eq!(
            scale_design_units(1, half_raw_unit, 0),
            Err(LinkedShaperError::LengthConversion(
                LengthError::ZeroDenominator
            ))
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
