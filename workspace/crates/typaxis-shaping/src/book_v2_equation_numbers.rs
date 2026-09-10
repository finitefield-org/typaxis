//! Authored equation labels using the actual /3 font instances and shared shaper.
use super::*;
use typaxis_syntax::book_v2::PreparedBookVector;

pub const BOOK_V2_EQUATION_NUMBER_ALGORITHM: &str = "typaxis.book-2-equation-number-shape/1";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2EquationNumberErrorKind {
    ReceiptMismatch,
    MissingTextStyle,
    MissingSelectedFont,
    Coverage(ProductionTextShapeErrorKind),
    Shape(StagingEquationNumberShapeError),
    OutputLimit,
    SpoolLimit,
    AllocationFailure,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2EquationNumberError {
    pub owner: NodeId,
    pub kind: BookV2EquationNumberErrorKind,
}
impl std::fmt::Display for BookV2EquationNumberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 equation number: {self:?}")
    }
}
impl std::error::Error for BookV2EquationNumberError {}

pub struct BookV2EquationNumberShape<'a> {
    source: &'a PreparedBookVector,
    text: &'a str,
    language: &'a str,
    font_face: FontFaceId,
    font_instance: FontInstanceId,
    font_sha256: [u8; 32],
    face_index: u32,
    font_size: PositiveLength,
    line_height: PositiveLength,
    paragraph_level: BidiLevel,
    runs: Vec<StagingEquationNumberGlyphRun>,
    width: PositiveLength,
    fingerprint: [u8; 32],
}
impl<'a> BookV2EquationNumberShape<'a> {
    pub fn source(&self) -> &'a PreparedBookVector {
        self.source
    }
    pub fn text(&self) -> &'a str {
        self.text
    }
    pub fn language(&self) -> &'a str {
        self.language
    }
    pub fn font_face_id(&self) -> FontFaceId {
        self.font_face
    }
    pub fn font_instance_id(&self) -> FontInstanceId {
        self.font_instance
    }
    pub fn font_sha256(&self) -> [u8; 32] {
        self.font_sha256
    }
    pub fn face_index(&self) -> u32 {
        self.face_index
    }
    pub fn font_size(&self) -> PositiveLength {
        self.font_size
    }
    pub fn height(&self) -> PositiveLength {
        self.line_height
    }
    pub fn width(&self) -> PositiveLength {
        self.width
    }
    pub fn paragraph_level(&self) -> BidiLevel {
        self.paragraph_level
    }
    pub fn runs(&self) -> &[StagingEquationNumberGlyphRun] {
        &self.runs
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}
/// Borrows the same canonical instance table as the final authored body shape.
/// It grants no page position or legacy equation-number receipt.
pub struct BookV2EquationNumberShapes<'a> {
    shaped: &'a BookV2AuthoredTextShape<'a>,
    shapes: Vec<BookV2EquationNumberShape<'a>>,
    record_charge: u64,
    prior_records: u64,
    fingerprint: [u8; 32],
}
impl<'a> BookV2EquationNumberShapes<'a> {
    pub fn shapes(&self) -> &[BookV2EquationNumberShape<'a>] {
        &self.shapes
    }
    pub fn shape(&self, owner: NodeId) -> Option<&BookV2EquationNumberShape<'a>> {
        self.shapes
            .binary_search_by_key(&owner, |s| s.source.node_id())
            .ok()
            .map(|i| &self.shapes[i])
    }
    pub fn prior_records(&self) -> u64 {
        self.prior_records
    }
    pub fn retained_records(&self) -> u64 {
        self.record_charge - self.prior_records
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        shaped: &BookV2AuthoredTextShape<'_>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), BookV2EquationNumberError> {
        if !std::ptr::eq(self.shaped, shaped) || shaped.limits_fingerprint != limits.fingerprint() {
            return Err(BookV2EquationNumberError {
                owner: NodeId::new(0),
                kind: BookV2EquationNumberErrorKind::ReceiptMismatch,
            });
        }
        Ok(())
    }
}
pub fn shape_book_v2_equation_numbers<'a>(
    shaped: &'a BookV2AuthoredTextShape<'a>,
    limits: &M4EffectiveResourceLimits,
    prior_records: u64,
) -> Result<Option<BookV2EquationNumberShapes<'a>>, BookV2EquationNumberError> {
    use BookV2EquationNumberErrorKind as E;
    let failure = |owner, kind| BookV2EquationNumberError { owner, kind };
    shaped
        .verify(shaped.flow, shaped.admitted, limits, shaped.epoch)
        .map_err(|e| failure(e.owner, E::ReceiptMismatch))?;
    let body = shaped.flow.body();
    let sources = body.body().vectors();
    let count = sources
        .iter()
        .filter(|s| s.equation_number().is_some())
        .count();
    if count == 0 {
        return Ok(None);
    }
    let mut charge = prior_records
        .checked_add(1)
        .and_then(|n| n.checked_add(count as u64))
        .filter(|n| *n <= limits.base().get().max_fragments)
        .ok_or_else(|| failure(NodeId::new(0), E::OutputLimit))?;
    let mut shapes = Vec::new();
    shapes
        .try_reserve_exact(count)
        .map_err(|_| failure(NodeId::new(0), E::AllocationFailure))?;
    let fonts = BodyFonts::BookV2(&shaped.instances, shaped.admitted);
    let mut fingerprint = sha256(BOOK_V2_EQUATION_NUMBER_ALGORITHM.as_bytes());
    for source in sources.iter().filter(|s| s.equation_number().is_some()) {
        let owner = source.node_id();
        let number = source
            .equation_number()
            .ok_or_else(|| failure(owner, E::ReceiptMismatch))?;
        let style = body
            .vector_style(owner)
            .ok_or_else(|| failure(owner, E::MissingTextStyle))?;
        let text_style = style
            .equation_number_text_style()
            .ok_or_else(|| failure(owner, E::MissingTextStyle))?;
        let font_size = text_style
            .font_size()
            .ok_or_else(|| failure(owner, E::MissingTextStyle))?;
        let line_height = text_style
            .line_height()
            .ok_or_else(|| failure(owner, E::MissingTextStyle))?;
        let families = text_style
            .font_families()
            .ok_or_else(|| failure(owner, E::MissingTextStyle))?;
        let font_face = fonts
            .font_families()
            .resolve(families)
            .map_err(|_| failure(owner, E::MissingSelectedFont))?;
        let font = fonts
            .font(font_face)
            .ok_or_else(|| failure(owner, E::MissingSelectedFont))?;
        let language = shaped
            .flow
            .navigation()
            .language(owner)
            .ok_or_else(|| failure(owner, E::ReceiptMismatch))?
            .effective_language();
        let text_span = number.text().text_span();
        let buffer = body
            .body()
            .wire()
            .text_buffers()
            .get(text_span.text_id().get() as usize)
            .filter(|b| b.text_id == text_span.text_id().get())
            .ok_or_else(|| failure(owner, E::ReceiptMismatch))?;
        let text = buffer
            .utf8
            .get(text_span.start_byte().get() as usize..text_span.end_byte().get() as usize)
            .ok_or_else(|| failure(owner, E::ReceiptMismatch))?;
        if sha256(buffer.utf8.as_bytes()) != number.text().text_buffer_sha256()
            || sha256(text.as_bytes()) != number.text().exact_text_sha256()
        {
            return Err(failure(owner, E::ReceiptMismatch));
        }
        if text.chars().any(|c| {
            matches!(c, '\u{2028}' | '\u{2029}')
                || unicode_bidi::bidi_class(c) == unicode_bidi::BidiClass::B
        }) {
            return Err(failure(
                owner,
                E::Shape(StagingEquationNumberShapeError::RequiresSecondLine),
            ));
        }
        let maximum_records = limits.base().get().max_shaping_context_bytes;
        if text.is_empty() || text.len() > maximum_records as usize {
            return Err(failure(
                owner,
                E::Shape(StagingEquationNumberShapeError::ContextLimit),
            ));
        }
        font.coverage(text)
            .map_err(|e| failure(owner, E::Coverage(e)))?;
        let metadata = font.metadata();
        let (paragraph_level, runs, width) = shape_equation_number_runs(EquationNumberRunInput {
            exact_text: text,
            text_span,
            font_bytes: font.bytes(),
            face_index: font.face_index(),
            units_per_em: metadata.units_per_em,
            glyph_count: metadata.glyph_count,
            font_instance: font.instance_id(),
            font_size,
            language: Some(language),
            maximum_records,
            total_records: Some(limits.base().get().max_fragments - charge),
        })
        .map_err(|e| failure(owner, E::Shape(e)))?;
        if !equation_number_runs_cover_with_level(text_span, &runs, typaxis_core::MAX_BIDI_LEVEL) {
            return Err(failure(owner, E::ReceiptMismatch));
        }
        let records = runs
            .iter()
            .try_fold(runs.len() as u64, |n, r| {
                n.checked_add(r.glyphs().len() as u64)?
                    .checked_add(r.clusters().len() as u64)
            })
            .ok_or_else(|| failure(owner, E::OutputLimit))?;
        charge = charge
            .checked_add(records)
            .filter(|n| *n <= limits.base().get().max_fragments)
            .ok_or_else(|| failure(owner, E::OutputLimit))?;
        // The shared glyph encoder's scalar fields are bounded; preflight its
        // temporary canonical storage before constructing the string.
        if records
            .checked_mul(1024)
            .and_then(|n| n.checked_add(1024))
            .map_or(true, |n| n > limits.base().get().max_spool_bytes)
        {
            return Err(failure(owner, E::SpoolLimit));
        }
        let glyph_digest = sha256(encode_equation_number_glyph_receipt(&runs).as_bytes());
        let mut digest = Vec::new();
        digest
            .try_reserve_exact(256)
            .map_err(|_| failure(owner, E::AllocationFailure))?;
        digest.extend_from_slice(&sha256(BOOK_V2_EQUATION_NUMBER_ALGORITHM.as_bytes()));
        digest.extend_from_slice(&shaped.fingerprint());
        digest.extend_from_slice(&style.fingerprint());
        digest.extend_from_slice(&glyph_digest);
        digest.extend_from_slice(&font.content_hash());
        digest.extend_from_slice(&sha256(language.as_bytes()));
        digest.extend_from_slice(&owner.get().to_be_bytes());
        digest.extend_from_slice(&number.node_id().get().to_be_bytes());
        digest.extend_from_slice(&font.instance_id().get().to_be_bytes());
        digest.extend_from_slice(&width.get().raw().to_be_bytes());
        digest.extend_from_slice(&line_height.get().raw().to_be_bytes());
        let shape_fingerprint = sha256(&digest);
        let mut fold = [0; 64];
        fold[..32].copy_from_slice(&fingerprint);
        fold[32..].copy_from_slice(&shape_fingerprint);
        fingerprint = sha256(&fold);
        shapes.push(BookV2EquationNumberShape {
            source,
            text,
            language,
            font_face,
            font_instance: font.instance_id(),
            font_sha256: font.content_hash(),
            face_index: font.face_index(),
            font_size,
            line_height,
            paragraph_level,
            runs,
            width,
            fingerprint: shape_fingerprint,
        });
    }
    if shapes
        .windows(2)
        .any(|p| p[0].source.node_id() >= p[1].source.node_id())
    {
        return Err(failure(NodeId::new(0), E::ReceiptMismatch));
    }
    Ok(Some(BookV2EquationNumberShapes {
        shaped,
        shapes,
        record_charge: charge,
        prior_records,
        fingerprint,
    }))
}

/// Actual metrics from the same admitted /3 font as the authored number shape.
pub fn book_v2_equation_number_font(
    shape: &BookV2EquationNumberShape<'_>,
    admitted: &AdmittedProductionResourceLedgerV3,
) -> Result<ProductionBodyFont, ProductionTextShapeError> {
    let owner = shape
        .source()
        .equation_number()
        .expect("number shape has authored source")
        .node_id();
    let font = admitted
        .font(shape.font_face_id())
        .filter(|f| f.content_hash() == shape.font_sha256() && f.face_index() == shape.face_index())
        .ok_or_else(|| error(owner, ProductionTextShapeErrorKind::MissingSelectedFont))?;
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

#[cfg(test)]
mod number_coverage_tests {
    use super::*;
    #[test]
    fn successor_embedding_levels_do_not_relax_legacy_or_source_coverage() {
        let span = TextSpan::new(
            typaxis_core::TextBufferId::new(0),
            typaxis_core::Utf8ByteOffset::new(0),
            typaxis_core::Utf8ByteOffset::new(1),
        )
        .unwrap();
        let mut runs = vec![StagingEquationNumberGlyphRun {
            run_id: GlyphRunId::new(0),
            bidi_level: BidiLevel::new(2).unwrap(),
            script: OpenTypeTag::new(*b"Latn").unwrap(),
            source_span: span,
            glyphs: vec![ShapedGlyph {
                original_gid: OriginalGlyphId::new(1),
                advance_x: Length::from_raw(1).unwrap(),
                advance_y: Length::ZERO,
                offset_x: Length::ZERO,
                offset_y: Length::ZERO,
            }],
            clusters: vec![ShapedCluster {
                source_span: ShapeSourceSpan::Parsed(span),
                glyph_start: 0,
                glyph_end: 1,
            }],
        }];
        assert!(!equation_number_runs_cover(span, &runs));
        assert!(equation_number_runs_cover_with_level(
            span,
            &runs,
            typaxis_core::MAX_BIDI_LEVEL
        ));
        runs[0].clusters[0].glyph_end = 2;
        assert!(!equation_number_runs_cover_with_level(
            span,
            &runs,
            typaxis_core::MAX_BIDI_LEVEL
        ));
        runs[0].clusters[0].glyph_end = 1;
        runs[0].run_id = GlyphRunId::new(1);
        assert!(!equation_number_runs_cover_with_level(
            span,
            &runs,
            typaxis_core::MAX_BIDI_LEVEL
        ));
        runs[0].run_id = GlyphRunId::new(0);
        runs[0].bidi_level = BidiLevel::LTR;
        assert!(equation_number_runs_cover(span, &runs));
    }
}
