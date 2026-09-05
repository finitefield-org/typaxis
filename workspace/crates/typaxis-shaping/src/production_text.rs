//! Authored body text shaping, before reference resolution and line selection.
//! The sealed result borrows its syntax/admission owners. It cannot authorize PDF
//! paint: atomic math, generated labels, and all line/page positions are pending.
use super::*;
use typaxis_core::M4EffectiveResourceLimits;
use typaxis_syntax::{
    ProductionInlineContent, ProductionTextFlow, ProductionTextParagraph,
    ValidatedStagingBookNavigationV2,
};

pub const PRODUCTION_AUTHORED_TEXT_SHAPE_ALGORITHM: &str =
    "typaxis.production-authored-text-shape/2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionTextShapeErrorKind {
    ReceiptMismatch,
    MissingTextStyle,
    MissingSelectedFont,
    MissingDeclaredFontCoverage,
    MissingShapedGlyph { span: TextSpan },
    InvalidFontMetrics,
    ContextLimit,
    OutputLimit,
    AllocationFailure,
    ArithmeticOverflow,
    Itemization(ItemizationError),
    Backend(LinkedShaperError),
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
    /// None for a paragraph without authored text. Math-only paragraphs need no
    /// invented body font; their atomic metrics have a separate owner.
    pub const fn font(&self) -> Option<&ProductionBodyFont> {
        self.font.as_ref()
    }
    pub const fn paragraph_level(&self) -> BidiLevel {
        self.paragraph_level
    }
    pub fn runs(&self) -> &[ProductionBodyTextRun<'a>] {
        &self.runs
    }
    /// Resolving these labels changes paragraph context and requires reshaping.
    pub fn pending_references(&self) -> &[NodeId] {
        &self.pending_references
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

pub struct ProductionAuthoredTextShape<'a> {
    flow: &'a ProductionTextFlow<'a>,
    admitted: &'a AdmittedResourceLedger,
    limits_fingerprint: [u8; 32],
    epoch: [u8; 32],
    paragraphs: Vec<ProductionBodyParagraphShape<'a>>,
    output_records: u64,
    fingerprint: [u8; 32],
}
impl<'a> ProductionAuthoredTextShape<'a> {
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
    let mut paragraphs = Vec::new();
    paragraphs
        .try_reserve_exact(flow.paragraphs().len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    let mut output_records = 0;
    for paragraph in flow.paragraphs() {
        paragraphs.push(shape_paragraph(
            paragraph,
            admitted,
            limits,
            &mut output_records,
        )?);
    }
    // Fixed-size paragraph digests bound the document receipt allocation even for
    // books with millions of glyphs. Each paragraph owns a separate glyph digest.
    let capacity = paragraphs
        .len()
        .checked_mul(32)
        .and_then(|n| n.checked_add(256))
        .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| error(root, E::AllocationFailure))?;
    bytes.extend_from_slice(&sha256(PRODUCTION_AUTHORED_TEXT_SHAPE_ALGORITHM.as_bytes()));
    bytes.extend_from_slice(&flow.fingerprint());
    bytes.extend_from_slice(&admitted.fingerprint().bytes());
    bytes.extend_from_slice(&limits.fingerprint());
    bytes.extend_from_slice(&epoch);
    let shaper = ShaperIdentity::linked_reference();
    bytes.extend_from_slice(&sha256(shaper.backend().as_bytes()));
    bytes.extend_from_slice(&sha256(shaper.version().as_bytes()));
    bytes.extend_from_slice(&output_records.to_be_bytes());
    for paragraph in &paragraphs {
        bytes.extend_from_slice(&paragraph.fingerprint);
    }
    Ok(ProductionAuthoredTextShape {
        flow,
        admitted,
        limits_fingerprint: limits.fingerprint(),
        epoch,
        paragraphs,
        output_records,
        fingerprint: sha256(&bytes),
    })
}

fn shape_paragraph<'a>(
    paragraph: &ProductionTextParagraph<'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    output_records: &mut u64,
) -> Result<ProductionBodyParagraphShape<'a>, ProductionTextShapeError> {
    use ProductionTextShapeErrorKind as E;
    let owner = paragraph.owner();
    let maximum = limits.base().get().max_shaping_context_bytes;
    let (context, ranges) = paragraph_context(paragraph, maximum)?;
    let mut pending_references = Vec::new();
    pending_references
        .try_reserve_exact(paragraph.items().len())
        .map_err(|_| error(owner, E::AllocationFailure))?;
    let mut has_text = false;
    for site in paragraph.items() {
        match site.content() {
            ProductionInlineContent::Reference | ProductionInlineContent::FootnoteReference => {
                pending_references.push(site.owner())
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
            let ProductionInlineContent::Text { span, utf8 } = site.content() else {
                continue;
            };
            if utf8.is_empty() {
                continue;
            }
            validate_admitted_font_coverage(font, utf8).map_err(|e| {
                error(
                    site.owner(),
                    match e {
                        StagingEquationNumberShapeError::MissingDeclaredFontCoverage => {
                            E::MissingDeclaredFontCoverage
                        }
                        _ => E::InvalidFontMetrics,
                    },
                )
            })?;
            while cursor < specs.len() && specs[cursor].end as usize <= start {
                cursor += 1;
            }
            for spec in specs[cursor..]
                .iter()
                .take_while(|spec| (spec.start as usize) < end)
            {
                let run_start = start.max(spec.start as usize);
                let run_end = end.min(spec.end as usize);
                let run_text = &context[run_start..run_end];
                if linked_backend_record_bound(run_text)
                    .map_err(|e| error(site.owner(), E::Backend(e)))?
                    > maximum
                {
                    return Err(error(site.owner(), E::ContextLimit));
                }
                let source = source_subspan(
                    ShapeSourceSpan::Parsed(span),
                    (run_start - start) as u32,
                    (run_end - start) as u32,
                )
                .map_err(|e| error(site.owner(), E::Backend(e)))?;
                let run_id = GlyphRunId::new(
                    u32::try_from(result.runs.len())
                        .map_err(|_| error(site.owner(), E::ArithmeticOverflow))?,
                );
                let mut budget = ShapeOutputBudget::new(maximum);
                let run = shape_linked(
                    LinkedBackendInput {
                        run_id,
                        font: FontInstanceId::new(face_id.get()),
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
                        pre_context: (run_start > 0).then_some(&context[..run_start]),
                        post_context: (run_end < context.len()).then_some(&context[run_end..]),
                        max_output_records: maximum,
                    },
                    &mut budget,
                )
                .map_err(|e| error(site.owner(), E::Backend(e)))?;
                let expected = ExpectedGlyphRun {
                    run_id,
                    font: FontInstanceId::new(face_id.get()),
                    bidi_level: spec.bidi_level,
                    source,
                    utf8_boundaries: utf8_boundaries(source, run_text)
                        .ok_or_else(|| error(site.owner(), E::ReceiptMismatch))?,
                    glyph_count: font.metadata().glyph_count,
                    max_output_records: maximum,
                };
                if !budget.matches_output(&run) || validate_glyph_run(&expected, &run).is_err() {
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
            return Err(E::ReceiptMismatch);
        };
        // Zero advance does not prove that glyph 0 has no ink. Until selected
        // layout has an explicit suppressed-cluster owner, do not authorize a
        // missing glyph even for an entirely default-ignorable source cluster.
        return Err(E::MissingShapedGlyph { span });
    }
    Ok(())
}

fn paragraph_context(
    paragraph: &ProductionTextParagraph<'_>,
    maximum: u32,
) -> Result<(String, Vec<(usize, usize)>), ProductionTextShapeError> {
    use ProductionInlineContent as C;
    use ProductionTextShapeErrorKind as E;
    let value = |content| match content {
        C::Text { utf8, .. } => utf8,
        C::NativeMath | C::InlineVector | C::MathVector | C::Reference | C::FootnoteReference => {
            "\u{fffc}"
        }
        // An explicit soft break is a zero-width opportunity (docs/07), not
        // an authored/generated space. It contributes no shaping scalar.
        C::SoftBreak => "",
        C::HardBreak => "\u{2028}",
        _ => "",
    };
    let owner = paragraph.owner();
    let len = paragraph.items().iter().try_fold(0usize, |n, site| {
        n.checked_add(value(site.content()).len())
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
        context.push_str(value(site.content()));
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
            .and_then(|n| n.checked_add(run.run.clusters.len().checked_mul(20)?))
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
        let span_bytes = |b: &mut Vec<u8>, source| {
            let ShapeSourceSpan::Parsed(span) = source else {
                unreachable!("authored text only")
            };
            b.extend_from_slice(&span.text_id().get().to_be_bytes());
            b.extend_from_slice(&span.start_byte().get().to_be_bytes());
            b.extend_from_slice(&span.end_byte().get().to_be_bytes());
        };
        span_bytes(&mut b, r.run.source_span);
        b.extend_from_slice(&(r.run.glyphs.len() as u64).to_be_bytes());
        for g in &r.run.glyphs {
            b.extend_from_slice(&g.original_gid.get().to_be_bytes());
            for metric in [g.advance_x, g.advance_y, g.offset_x, g.offset_y] {
                b.extend_from_slice(&metric.raw().to_be_bytes());
            }
        }
        b.extend_from_slice(&(r.run.clusters.len() as u64).to_be_bytes());
        for c in &r.run.clusters {
            span_bytes(&mut b, c.source_span);
            b.extend_from_slice(&c.glyph_start.to_be_bytes());
            b.extend_from_slice(&c.glyph_end.to_be_bytes());
        }
    }
    Ok(sha256(&b))
}
