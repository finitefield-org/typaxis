//! Authored body text shaping, before reference resolution and line selection.
//! The sealed result borrows its syntax/admission owners. It cannot authorize PDF
//! paint: atomic math and all line/page positions are separate downstream owners.
//! Canonical list and footnote labels use the generated text namespace.
//! Footnote placement remains pending even after its inline marker is shaped.
use super::*;
use typaxis_core::M4EffectiveResourceLimits;
use typaxis_syntax::{
    ProductionInlineContent, ProductionTextFlow, ProductionTextParagraph,
    ValidatedStagingBookNavigationV2,
};

#[path = "production_text_inputs.rs"]
mod inputs;
use inputs::{flow_call, BodyFlow, BodyFonts};

#[cfg(feature = "book-v2-staging")]
#[path = "book_v2_text.rs"]
pub mod book_v2;

#[path = "production_list_markers.rs"]
mod list_markers;
pub use list_markers::ProductionListMarkerShape;
#[path = "production_footnote_markers.rs"]
mod footnote_markers;
pub use footnote_markers::ProductionFootnoteMarkerShape;
pub const PRODUCTION_AUTHORED_TEXT_SHAPE_ALGORITHM: &str =
    "typaxis.production-authored-text-shape/5";

// Shared error variants stay stable when a dependency enables staging without
// enabling the consuming crate's staging entry points.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionTextShapeErrorKind {
    ReceiptMismatch,
    MissingTextStyle,
    MissingSelectedFont,
    MissingDeclaredFontCoverage,
    MissingShapedGlyph {
        span: TextSpan,
    },
    MissingGeneratedGlyph,
    InvalidFontMetrics,
    InvalidLineContext,
    ContextLimit,
    OutputLimit,
    AllocationFailure,
    ArithmeticOverflow,
    Itemization(ItemizationError),
    Backend(LinkedShaperError),
    CffV2(Cff1ShapeErrorV2),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionTextShapeError {
    pub owner: NodeId,
    pub kind: ProductionTextShapeErrorKind,
}
impl std::fmt::Display for ProductionTextShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "production_authored_text {:?}: node {}",
            self.kind,
            self.owner.get()
        )
    }
}
impl std::error::Error for ProductionTextShapeError {}
fn error(owner: NodeId, kind: ProductionTextShapeErrorKind) -> ProductionTextShapeError {
    ProductionTextShapeError { owner, kind }
}

/// Horizontal hhea metrics scaled from the selected admitted face. These signed
/// metrics are not glyph ink bounds and must not be used as a clipping rectangle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyFont {
    face_id: FontFaceId,
    content_hash: [u8; 32],
    face_index: u32,
    size: PositiveLength,
    ascender: Length,
    descender: Length,
    line_gap: Length,
}
impl ProductionBodyFont {
    pub const fn face_id(&self) -> FontFaceId {
        self.face_id
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.content_hash
    }
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }
    pub const fn size(&self) -> PositiveLength {
        self.size
    }
    pub const fn ascender(&self) -> Length {
        self.ascender
    }
    pub const fn descender(&self) -> Length {
        self.descender
    }
    pub const fn line_gap(&self) -> Length {
        self.line_gap
    }
}

#[derive(Debug)]
pub struct ProductionBodyTextRun<'a> {
    site_index: u32,
    owner: NodeId,
    language: &'a str,
    script: OpenTypeTag,
    run: GlyphRun,
}
impl<'a> ProductionBodyTextRun<'a> {
    pub const fn site_index(&self) -> u32 {
        self.site_index
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn language(&self) -> &'a str {
        self.language
    }
    pub const fn script(&self) -> OpenTypeTag {
        self.script
    }
    pub const fn glyph_run(&self) -> &GlyphRun {
        &self.run
    }
}

#[derive(Debug)]
pub struct ProductionBodyParagraphShape<'a> {
    owner: NodeId,
    font: Option<ProductionBodyFont>,
    paragraph_level: BidiLevel,
    runs: Vec<ProductionBodyTextRun<'a>>,
    pending_references: Vec<NodeId>,
    fingerprint: [u8; 32],
}
impl<'a> ProductionBodyParagraphShape<'a> {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    /// None for a paragraph without authored text or a canonical inline label.
    /// Math-only paragraphs need no invented body font; their atomic metrics
    /// have a separate owner.
    pub const fn font(&self) -> Option<&ProductionBodyFont> {
        self.font.as_ref()
    }
    pub const fn paragraph_level(&self) -> BidiLevel {
        self.paragraph_level
    }
    pub fn runs(&self) -> &[ProductionBodyTextRun<'a>] {
        &self.runs
    }
    /// Ordinary references still require label resolution. Footnote markers are
    /// shaped, but their definitions still require placement and a page owner.
    pub fn pending_references(&self) -> &[NodeId] {
        &self.pending_references
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// UTF-8 end offsets in the paragraph shaping context (including U+FFFC for
/// atomic objects and U+2028 for hard breaks). Boundaries must partition it at
/// grapheme boundaries. This input is not a convergence/publication permit.
#[derive(Clone, Copy, Debug)]
pub struct ProductionParagraphLineContext<'a> {
    pub owner: NodeId,
    pub ends: &'a [u32],
}

pub struct ProductionAuthoredTextShape<'a> {
    flow: &'a ProductionTextFlow<'a>,
    admitted: &'a AdmittedResourceLedger,
    limits_fingerprint: [u8; 32],
    line_context_fingerprint: Option<[u8; 32]>,
    epoch: [u8; 32],
    paragraphs: Vec<ProductionBodyParagraphShape<'a>>,
    list_markers: Vec<ProductionListMarkerShape<'a>>,
    footnote_markers: Vec<ProductionFootnoteMarkerShape<'a>>,
    output_records: u64,
    fingerprint: [u8; 32],
}
impl<'a> ProductionAuthoredTextShape<'a> {
    pub const fn line_context_fingerprint(&self) -> Option<[u8; 32]> {
        self.line_context_fingerprint
    }
    pub fn footnote_markers(&self) -> &[ProductionFootnoteMarkerShape<'a>] {
        &self.footnote_markers
    }
    pub fn list_markers(&self) -> &[ProductionListMarkerShape<'a>] {
        &self.list_markers
    }
    pub fn paragraphs(&self) -> &[ProductionBodyParagraphShape<'a>] {
        &self.paragraphs
    }
    pub const fn output_records(&self) -> u64 {
        self.output_records
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    /// This result is immutable, non-deserializable and constructed only here.
    /// Verify owners once at a downstream stage boundary, not for every glyph.
    pub fn verify(
        &self,
        flow: &ProductionTextFlow<'_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
        epoch: [u8; 32],
    ) -> Result<(), ProductionTextShapeError> {
        if !std::ptr::eq(self.flow, flow)
            || !std::ptr::eq(self.admitted, admitted)
            || self.limits_fingerprint != limits.fingerprint()
            || self.epoch != epoch
        {
            return Err(error(
                NodeId::new(0),
                ProductionTextShapeErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}

/// Shapes authored text with the paragraph's actual declared body font, without
/// requiring native math or a MATH table. Paragraphs/runs remain in logical source
/// order; bidi visual ordering belongs to selected lines, not this receipt.
pub fn shape_production_authored_text<'a>(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    flow: &'a ProductionTextFlow<'a>,
    admitted: &'a AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
) -> Result<ProductionAuthoredTextShape<'a>, ProductionTextShapeError> {
    shape_authored_text(package, navigation, flow, admitted, limits, epoch, None)
}

/// Re-shape against selected line contexts. The caller still owns the bounded
/// compare/rebreak loop and must never treat this result as stable by itself.
#[allow(clippy::too_many_arguments)]
pub fn reshape_production_authored_text<'a>(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    flow: &'a ProductionTextFlow<'a>,
    admitted: &'a AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: &[ProductionParagraphLineContext<'_>],
) -> Result<ProductionAuthoredTextShape<'a>, ProductionTextShapeError> {
    shape_authored_text(
        package,
        navigation,
        flow,
        admitted,
        limits,
        epoch,
        Some(lines),
    )
}

#[allow(clippy::too_many_arguments)]
fn shape_authored_text<'a>(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    flow: &'a ProductionTextFlow<'a>,
    admitted: &'a AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
) -> Result<ProductionAuthoredTextShape<'a>, ProductionTextShapeError> {
    use ProductionTextShapeErrorKind as E;
    let root = NodeId::new(0);
    flow.verify(package, navigation, limits)
        .map_err(|e| error(e.owner, E::ReceiptMismatch))?;
    let catalog = staging_declared_base_catalog(package.resources())
        .map_err(|_| error(root, E::ReceiptMismatch))?;
    if epoch == [0; 32]
        || !admitted.matches_declarations(catalog.resource_catalog())
        || unicode_bidi::UNICODE_VERSION != (16, 0, 0)
        || unicode_script::UNICODE_VERSION != (16, 0, 0)
        || unicode_segmentation::UNICODE_VERSION != (16, 0, 0)
    {
        return Err(error(root, E::ReceiptMismatch));
    }
    let BodyShapeOutput {
        line_context_fingerprint,
        paragraphs,
        list_markers,
        footnote_markers,
        output_records,
        fingerprint,
    } = shape_document(
        BodyFlow::Legacy(flow),
        BodyFonts::Legacy(admitted),
        limits,
        epoch,
        lines,
        PRODUCTION_AUTHORED_TEXT_SHAPE_ALGORITHM,
    )?;

    Ok(ProductionAuthoredTextShape {
        flow,
        admitted,
        limits_fingerprint: limits.fingerprint(),
        line_context_fingerprint,
        epoch,
        paragraphs,
        list_markers,
        footnote_markers,
        output_records,
        fingerprint,
    })
}

struct BodyShapeOutput<'a> {
    line_context_fingerprint: Option<[u8; 32]>,
    paragraphs: Vec<ProductionBodyParagraphShape<'a>>,
    list_markers: Vec<ProductionListMarkerShape<'a>>,
    footnote_markers: Vec<ProductionFootnoteMarkerShape<'a>>,
    output_records: u64,
    fingerprint: [u8; 32],
}
fn shape_document<'a>(
    flow: BodyFlow<'a>,
    admitted: BodyFonts<'_>,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
    algorithm: &str,
) -> Result<BodyShapeOutput<'a>, ProductionTextShapeError> {
    use ProductionTextShapeErrorKind as E;
    let root = NodeId::new(0);
    let mut output_records = 0u64;
    let mut line_context_fingerprint = None;
    if let Some(contexts) = lines {
        if contexts.len() != flow_call!(flow, paragraphs()).len() {
            return Err(error(root, E::InvalidLineContext));
        }
        let mut hash = sha256(b"typaxis.production-line-context/1");
        for (context, paragraph) in contexts.iter().zip(flow_call!(flow, paragraphs())) {
            if context.owner != paragraph.owner() {
                return Err(error(paragraph.owner(), E::InvalidLineContext));
            }
            output_records = output_records
                .checked_add(context.ends.len() as u64)
                .and_then(|n| n.checked_add(1))
                .filter(|n| *n <= limits.base().get().max_fragments)
                .ok_or_else(|| error(context.owner, E::OutputLimit))?;
            let mut record = [0u8; 44];
            record[..32].copy_from_slice(&hash);
            record[32..36].copy_from_slice(&context.owner.get().to_be_bytes());
            record[36..].copy_from_slice(&(context.ends.len() as u64).to_be_bytes());
            hash = sha256(&record);
            for end in context.ends {
                let mut record = [0u8; 36];
                record[..32].copy_from_slice(&hash);
                record[32..].copy_from_slice(&end.to_be_bytes());
                hash = sha256(&record);
            }
        }
        line_context_fingerprint = Some(hash);
    }
    let mut paragraphs = Vec::new();
    paragraphs
        .try_reserve_exact(flow_call!(flow, paragraphs()).len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    for (index, paragraph) in flow_call!(flow, paragraphs()).iter().enumerate() {
        paragraphs.push(shape_paragraph(
            flow,
            paragraph,
            admitted,
            limits,
            &mut output_records,
            lines.map(|contexts| contexts[index].ends),
        )?);
    }
    let list_markers = list_markers::shape_markers(flow, admitted, limits, &mut output_records)?;
    let footnote_markers =
        footnote_markers::shape_markers(flow, admitted, limits, &mut output_records)?;
    // Fixed-size paragraph digests bound the document receipt allocation even for
    // books with millions of glyphs. Each paragraph owns a separate glyph digest.
    let capacity = paragraphs
        .len()
        .checked_add(list_markers.len())
        .and_then(|n| n.checked_add(footnote_markers.len()))
        .ok_or_else(|| error(root, E::ArithmeticOverflow))?
        .checked_mul(32)
        .and_then(|n| n.checked_add(256))
        .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| error(root, E::AllocationFailure))?;
    bytes.extend_from_slice(&sha256(algorithm.as_bytes()));
    bytes.extend_from_slice(&flow_call!(flow, fingerprint()));
    bytes.extend_from_slice(&admitted.fingerprint());
    bytes.extend_from_slice(&limits.fingerprint());
    bytes.extend_from_slice(&epoch);
    let shaper = ShaperIdentity::linked_reference();
    bytes.extend_from_slice(&sha256(shaper.backend().as_bytes()));
    bytes.extend_from_slice(&sha256(shaper.version().as_bytes()));
    bytes.extend_from_slice(&output_records.to_be_bytes());
    for paragraph in &paragraphs {
        bytes.extend_from_slice(&paragraph.fingerprint);
    }
    for marker in &list_markers {
        bytes.extend_from_slice(&marker.fingerprint());
    }
    for marker in &footnote_markers {
        bytes.extend_from_slice(&marker.fingerprint());
    }
    let mut fingerprint = sha256(&bytes);
    if let Some(context) = line_context_fingerprint {
        let mut record = [0u8; 64];
        record[..32].copy_from_slice(&fingerprint);
        record[32..].copy_from_slice(&context);
        fingerprint = sha256(&record);
    }
    Ok(BodyShapeOutput {
        line_context_fingerprint,
        paragraphs,
        list_markers,
        footnote_markers,
        output_records,
        fingerprint,
    })
}

fn shape_paragraph<'a>(
    flow: BodyFlow<'a>,
    paragraph: &ProductionTextParagraph<'a>,
    admitted: BodyFonts<'_>,
    limits: &M4EffectiveResourceLimits,
    output_records: &mut u64,
    line_ends: Option<&[u32]>,
) -> Result<ProductionBodyParagraphShape<'a>, ProductionTextShapeError> {
    use ProductionTextShapeErrorKind as E;
    let owner = paragraph.owner();
    let maximum = limits.base().get().max_shaping_context_bytes;
    let (context, ranges) = paragraph_context(flow, paragraph, maximum)?;
    if let Some(ends) = line_ends {
        if ends.last().copied().unwrap_or(0) as usize != context.len()
            || (context.len() > 0 && ends.is_empty())
            || ends.windows(2).any(|w| w[0] > w[1])
        {
            return Err(error(owner, E::InvalidLineContext));
        }
        let mut boundaries = UnicodeSegmentation::grapheme_indices(context.as_str(), true)
            .map(|(i, _)| i)
            .chain(std::iter::once(context.len()))
            .peekable();
        for end in ends {
            let end = *end as usize;
            while boundaries.peek().is_some_and(|v| *v < end) {
                boundaries.next();
            }
            if boundaries.peek() != Some(&end) {
                return Err(error(owner, E::InvalidLineContext));
            }
        }
    }
    let mut pending_references = Vec::new();
    pending_references
        .try_reserve_exact(paragraph.items().len())
        .map_err(|_| error(owner, E::AllocationFailure))?;
    let mut has_text = false;
    for site in paragraph.items() {
        match site.content() {
            ProductionInlineContent::Reference | ProductionInlineContent::FootnoteReference => {
                pending_references.push(site.owner());
                has_text |= site_text(flow, site).is_some_and(|(_, text)| !text.is_empty());
            }
            ProductionInlineContent::Text { utf8, .. } => has_text |= !utf8.is_empty(),
            _ => (),
        }
    }
    let (paragraph_level, specs) = if context.is_empty() {
        (BidiLevel::LTR, Vec::new())
    } else {
        itemize_run_specs(&context).map_err(|e| error(owner, E::Itemization(e)))?
    };
    let mut result = ProductionBodyParagraphShape {
        owner,
        font: None,
        paragraph_level,
        runs: Vec::new(),
        pending_references,
        fingerprint: [0; 32],
    };
    if has_text {
        let style = paragraph.style();
        let families = style
            .font_families()
            .ok_or_else(|| error(owner, E::MissingTextStyle))?;
        let size = style
            .font_size()
            .ok_or_else(|| error(owner, E::MissingTextStyle))?;
        let face_id = admitted
            .font_families()
            .resolve(families)
            .map_err(|_| error(owner, E::MissingSelectedFont))?;
        let font = admitted
            .font(face_id)
            .ok_or_else(|| error(owner, E::MissingSelectedFont))?;
        let face = harfrust::FontRef::from_index(font.bytes(), font.face_index())
            .map_err(|_| error(owner, E::InvalidFontMetrics))?;
        let hhea = face
            .hhea()
            .map_err(|_| error(owner, E::InvalidFontMetrics))?;
        let scale = |v| {
            scale_design_units(i32::from(v), size.get(), font.metadata().units_per_em)
                .map_err(|e| error(owner, E::Backend(e)))
        };
        result.font = Some(ProductionBodyFont {
            face_id,
            content_hash: font.content_hash(),
            face_index: font.face_index(),
            size,
            ascender: scale(hhea.ascender().to_i16())?,
            descender: scale(hhea.descender().to_i16())?,
            line_gap: scale(hhea.line_gap().to_i16())?,
        });
        // Intersect two ordered partitions with a moving cursor; do not scan all
        // itemized runs again for every source site in a long paragraph.
        let mut cursor = 0;
        for (index, (site, &(start, end))) in paragraph.items().iter().zip(&ranges).enumerate() {
            let Some((site_source, utf8)) = site_text(flow, site) else {
                continue;
            };
            if utf8.is_empty() {
                continue;
            }
            font.coverage(utf8)
                .map_err(|kind| error(site.owner(), kind))?;
            while cursor < specs.len() && specs[cursor].end as usize <= start {
                cursor += 1;
            }
            for spec in specs[cursor..]
                .iter()
                .take_while(|spec| (spec.start as usize) < end)
            {
                let mut run_start = start.max(spec.start as usize);
                let spec_end = end.min(spec.end as usize);
                while run_start < spec_end {
                    let (context_start, context_end) = match line_ends {
                        None => (0, context.len()),
                        Some(ends) => {
                            let line = ends.partition_point(|end| *end as usize <= run_start);
                            let end = *ends
                                .get(line)
                                .ok_or_else(|| error(owner, E::InvalidLineContext))?
                                as usize;
                            (
                                if line == 0 {
                                    0
                                } else {
                                    ends[line - 1] as usize
                                },
                                end,
                            )
                        }
                    };
                    let run_end = spec_end.min(context_end);
                    let run_text = &context[run_start..run_end];
                    if linked_backend_record_bound(run_text)
                        .map_err(|e| error(site.owner(), E::Backend(e)))?
                        > maximum
                    {
                        return Err(error(site.owner(), E::ContextLimit));
                    }
                    let source = source_subspan(
                        site_source,
                        (run_start - start) as u32,
                        (run_end - start) as u32,
                    )
                    .map_err(|e| error(site.owner(), E::Backend(e)))?;
                    let run_id = GlyphRunId::new(
                        u32::try_from(result.runs.len())
                            .map_err(|_| error(site.owner(), E::ArithmeticOverflow))?,
                    );
                    font.run_coverage(run_text, &context[run_end..context_end])
                        .map_err(|kind| error(site.owner(), kind))?;
                    let mut budget = ShapeOutputBudget::new(maximum);
                    let run = shape_linked(
                        LinkedBackendInput {
                            run_id,
                            font: font.instance_id(),
                            source,
                            utf8: run_text,
                            font_bytes: font.bytes(),
                            face_index: font.face_index(),
                            admitted_units_per_em: font.metadata().units_per_em,
                            admitted_glyph_count: font.metadata().glyph_count,
                            font_size: size.get(),
                            bidi_level: spec.bidi_level,
                            script: spec.script,
                            language: Some(site.language()),
                            pre_context: (run_start > context_start)
                                .then_some(&context[context_start..run_start]),
                            post_context: (run_end < context_end)
                                .then_some(&context[run_end..context_end]),
                            max_output_records: maximum,
                        },
                        &mut budget,
                    )
                    .map_err(|e| error(site.owner(), E::Backend(e)))?;
                    let expected = ExpectedGlyphRun {
                        run_id,
                        font: font.instance_id(),
                        bidi_level: spec.bidi_level,
                        source,
                        utf8_boundaries: utf8_boundaries(source, run_text)
                            .ok_or_else(|| error(site.owner(), E::ReceiptMismatch))?,
                        glyph_count: font.metadata().glyph_count,
                        max_output_records: maximum,
                    };
                    if !budget.matches_output(&run) || validate_glyph_run(&expected, &run).is_err()
                    {
                        return Err(error(site.owner(), E::ReceiptMismatch));
                    }
                    validate_body_glyph_coverage(&run).map_err(|kind| error(site.owner(), kind))?;
                    // Bound retained output across the complete document, in addition
                    // to the backend's per-request allocation ceiling.
                    let charge = 1 + run.glyphs.len() as u64 + run.clusters.len() as u64;
                    *output_records = output_records
                        .checked_add(charge)
                        .filter(|n| *n <= limits.base().get().max_fragments)
                        .ok_or_else(|| error(site.owner(), E::OutputLimit))?;
                    result
                        .runs
                        .try_reserve(1)
                        .map_err(|_| error(site.owner(), E::AllocationFailure))?;
                    result.runs.push(ProductionBodyTextRun {
                        site_index: index as u32,
                        owner: site.owner(),
                        language: site.language(),
                        script: spec.script,
                        run,
                    });
                    run_start = run_end;
                }
            }
        }
    }
    result.fingerprint = paragraph_fingerprint(&result)?;
    Ok(result)
}

fn validate_body_glyph_coverage(run: &GlyphRun) -> Result<(), ProductionTextShapeErrorKind> {
    use ProductionTextShapeErrorKind as E;
    for cluster in &run.clusters {
        let glyphs = &run.glyphs[cluster.glyph_start as usize..cluster.glyph_end as usize];
        if !glyphs.iter().any(|glyph| glyph.original_gid.get() == 0) {
            continue;
        }
        let ShapeSourceSpan::Parsed(span) = cluster.source_span else {
            return Err(E::MissingGeneratedGlyph);
        };
        // Zero advance does not prove that glyph 0 has no ink. Until selected
        // layout has an explicit suppressed-cluster owner, do not authorize a
        // missing glyph even for an entirely default-ignorable source cluster.
        return Err(E::MissingShapedGlyph { span });
    }
    Ok(())
}

fn site_text<'a>(
    flow: BodyFlow<'a>,
    site: &typaxis_syntax::ProductionInlineSite<'a>,
) -> Option<(ShapeSourceSpan, &'a str)> {
    match site.content() {
        ProductionInlineContent::Text { span, utf8 } => Some((ShapeSourceSpan::Parsed(span), utf8)),
        ProductionInlineContent::Reference => Some((
            ShapeSourceSpan::Generated(flow_call!(flow, reference_provenance(site.owner()))?),
            flow_call!(flow, reference_text(site.owner()))?,
        )),
        ProductionInlineContent::FootnoteReference => Some((
            ShapeSourceSpan::Generated(flow_call!(flow, footnote_marker_provenance(site.owner()))?),
            flow_call!(flow, footnote_marker_text(site.owner()))?,
        )),
        _ => None,
    }
}

fn paragraph_context<'a>(
    flow: BodyFlow<'a>,
    paragraph: &ProductionTextParagraph<'a>,
    maximum: u32,
) -> Result<(String, Vec<(usize, usize)>), ProductionTextShapeError> {
    use ProductionInlineContent as C;
    use ProductionTextShapeErrorKind as E;
    let value = |site: &typaxis_syntax::ProductionInlineSite<'a>| {
        if let Some((_, text)) = site_text(flow, site) {
            return text;
        }
        match site.content() {
            C::Text { utf8, .. } => utf8,
            C::NativeMath
            | C::InlineVector
            | C::MathVector
            | C::Reference
            | C::FootnoteReference => "\u{fffc}",
            // An explicit soft break is a zero-width opportunity (docs/07), not
            // an authored/generated space. It contributes no shaping scalar.
            C::SoftBreak => "",
            C::HardBreak => "\u{2028}",
            _ => "",
        }
    };
    let owner = paragraph.owner();
    let len = paragraph.items().iter().try_fold(0usize, |n, site| {
        n.checked_add(value(site).len())
            .filter(|n| *n <= maximum as usize)
            .ok_or_else(|| error(site.owner(), E::ContextLimit))
    })?;
    let mut context = String::new();
    context
        .try_reserve_exact(len)
        .map_err(|_| error(owner, E::AllocationFailure))?;
    let mut ranges = Vec::new();
    ranges
        .try_reserve_exact(paragraph.items().len())
        .map_err(|_| error(owner, E::AllocationFailure))?;
    for site in paragraph.items() {
        let start = context.len();
        context.push_str(value(site));
        ranges.push((start, context.len()));
    }
    // Linear merge instead of scanning all graphemes per site. Zero-width syntax
    // boundaries must also not split an extended grapheme across two text owners.
    let mut boundaries = UnicodeSegmentation::grapheme_indices(context.as_str(), true)
        .map(|(start, _)| start)
        .chain(std::iter::once(context.len()))
        .peekable();
    for (site, &(start, end)) in paragraph.items().iter().zip(&ranges) {
        for boundary in [start, end] {
            while boundaries.peek().is_some_and(|n| *n < boundary) {
                boundaries.next();
            }
            if boundaries.peek() != Some(&boundary) {
                return Err(error(
                    site.owner(),
                    E::Itemization(ItemizationError::SiteBoundarySplitsGrapheme),
                ));
            }
        }
    }
    Ok((context, ranges))
}

fn paragraph_fingerprint(
    p: &ProductionBodyParagraphShape<'_>,
) -> Result<[u8; 32], ProductionTextShapeError> {
    use ProductionTextShapeErrorKind as E;
    let mut records = p
        .runs
        .len()
        .checked_mul(128)
        .and_then(|n| n.checked_add(256))
        .and_then(|n| n.checked_add(p.pending_references.len().checked_mul(4)?))
        .ok_or_else(|| error(p.owner, E::ArithmeticOverflow))?;
    for run in &p.runs {
        records = run
            .run
            .glyphs
            .len()
            .checked_mul(36)
            .and_then(|n| n.checked_add(run.run.clusters.len().checked_mul(40)?))
            .and_then(|n| n.checked_add(records))
            .ok_or_else(|| error(p.owner, E::ArithmeticOverflow))?;
    }
    let mut b = Vec::new();
    b.try_reserve_exact(records)
        .map_err(|_| error(p.owner, E::AllocationFailure))?;
    b.extend_from_slice(&p.owner.get().to_be_bytes());
    b.push(p.paragraph_level.get());
    b.push(u8::from(p.font.is_some()));
    if let Some(font) = &p.font {
        b.extend_from_slice(&font.face_id.get().to_be_bytes());
        b.extend_from_slice(&font.content_hash);
        b.extend_from_slice(&font.face_index.to_be_bytes());
        for metric in [
            font.size.get(),
            font.ascender,
            font.descender,
            font.line_gap,
        ] {
            b.extend_from_slice(&metric.raw().to_be_bytes());
        }
    }
    b.extend_from_slice(&(p.pending_references.len() as u64).to_be_bytes());
    for owner in &p.pending_references {
        b.extend_from_slice(&owner.get().to_be_bytes());
    }
    b.extend_from_slice(&(p.runs.len() as u64).to_be_bytes());
    for r in &p.runs {
        b.extend_from_slice(&r.site_index.to_be_bytes());
        b.extend_from_slice(&r.owner.get().to_be_bytes());
        b.extend_from_slice(&sha256(r.language.as_bytes()));
        b.extend_from_slice(&r.script.bytes());
        b.extend_from_slice(&r.run.run_id.get().to_be_bytes());
        b.extend_from_slice(&r.run.font.get().to_be_bytes());
        b.push(r.run.bidi_level.get());
        let span_bytes = |b: &mut Vec<u8>, source| -> Result<(), ProductionTextShapeError> {
            match source {
                ShapeSourceSpan::Parsed(span) => {
                    b.push(0);
                    b.extend_from_slice(&span.text_id().get().to_be_bytes());
                    b.extend_from_slice(&span.start_byte().get().to_be_bytes());
                    b.extend_from_slice(&span.end_byte().get().to_be_bytes());
                }
                ShapeSourceSpan::Generated(provenance) => {
                    let key = provenance.buffer_key();
                    b.push(match key.generation_kind() {
                        typaxis_core::GenerationKind::FootnoteMarker => 1,
                        typaxis_core::GenerationKind::PageReference => 2,
                        typaxis_core::GenerationKind::Counter => 3,
                        _ => return Err(error(r.owner, E::ReceiptMismatch)),
                    });
                    b.extend_from_slice(&key.owner().get().to_be_bytes());
                    b.extend_from_slice(&key.owner_local_ordinal().to_be_bytes());
                    let span = provenance.text_span();
                    b.extend_from_slice(&span.text_id().get().to_be_bytes());
                    b.extend_from_slice(&span.range().start_byte().get().to_be_bytes());
                    b.extend_from_slice(&span.range().end_byte().get().to_be_bytes());
                }
            }
            Ok(())
        };
        span_bytes(&mut b, r.run.source_span)?;
        b.extend_from_slice(&(r.run.glyphs.len() as u64).to_be_bytes());
        for g in &r.run.glyphs {
            b.extend_from_slice(&g.original_gid.get().to_be_bytes());
            for metric in [g.advance_x, g.advance_y, g.offset_x, g.offset_y] {
                b.extend_from_slice(&metric.raw().to_be_bytes());
            }
        }
        b.extend_from_slice(&(r.run.clusters.len() as u64).to_be_bytes());
        for c in &r.run.clusters {
            span_bytes(&mut b, c.source_span)?;
            b.extend_from_slice(&c.glyph_start.to_be_bytes());
            b.extend_from_slice(&c.glyph_end.to_be_bytes());
        }
    }
    Ok(sha256(&b))
}

/// Resolve actual horizontal metrics for a sealed, producer-authored number.
/// The production path uses these metrics; the frozen staging PDF recipe is unchanged.
pub fn production_equation_number_font(
    shape: &StagingEquationNumberShapeReceipt,
    admitted: &AdmittedResourceLedger,
) -> Result<ProductionBodyFont, ProductionTextShapeError> {
    use ProductionTextShapeErrorKind as E;
    let owner = shape.node_id();
    let font = admitted
        .font(shape.font_face_id())
        .filter(|f| f.content_hash() == shape.font_sha256() && f.face_index() == shape.face_index())
        .ok_or_else(|| error(owner, E::MissingSelectedFont))?;
    equation_number_font_metrics(
        owner,
        shape.font_face_id(),
        shape.font_size(),
        font.bytes(),
        font.face_index(),
        font.content_hash(),
        font.metadata().units_per_em,
    )
}

fn equation_number_font_metrics(
    owner: NodeId,
    face_id: FontFaceId,
    size: PositiveLength,
    bytes: &[u8],
    face_index: u32,
    content_hash: [u8; 32],
    units_per_em: u16,
) -> Result<ProductionBodyFont, ProductionTextShapeError> {
    use ProductionTextShapeErrorKind as E;
    let face = harfrust::FontRef::from_index(bytes, face_index)
        .map_err(|_| error(owner, E::InvalidFontMetrics))?;
    let hhea = face
        .hhea()
        .map_err(|_| error(owner, E::InvalidFontMetrics))?;
    let scale = |v| {
        scale_design_units(i32::from(v), size.get(), units_per_em)
            .map_err(|e| error(owner, E::Backend(e)))
    };
    let metrics = ProductionBodyFont {
        face_id: face_id,
        content_hash: content_hash,
        face_index: face_index,
        size: size,
        ascender: scale(hhea.ascender().to_i16())?,
        descender: scale(hhea.descender().to_i16())?,
        line_gap: scale(hhea.line_gap().to_i16())?,
    };
    if metrics.ascender() < Length::ZERO || metrics.descender() > Length::ZERO {
        return Err(error(owner, E::InvalidFontMetrics));
    }
    Ok(metrics)
}
