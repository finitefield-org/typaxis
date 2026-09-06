//! Real admitted-font shaping of syntax-derived generated list labels.
//! This result has no page position and cannot authorize standalone PDF paint.
use super::*;

pub struct ProductionListMarkerShape<'a> {
    source: &'a typaxis_syntax::ProductionListItem<'a>,
    marker_index: u32,
    utf8: &'a str,
    provenance: GeneratedProvenance,
    font: ProductionBodyFont,
    run: GlyphRun,
    advance: PositiveLength,
    fingerprint: [u8; 32],
}
impl<'a> ProductionListMarkerShape<'a> {
    pub const fn source(&self) -> &'a typaxis_syntax::ProductionListItem<'a> {
        self.source
    }
    pub const fn marker_index(&self) -> u32 {
        self.marker_index
    }
    pub const fn utf8(&self) -> &'a str {
        self.utf8
    }
    pub const fn provenance(&self) -> GeneratedProvenance {
        self.provenance
    }
    pub const fn font(&self) -> &ProductionBodyFont {
        &self.font
    }
    pub const fn glyph_run(&self) -> &GlyphRun {
        &self.run
    }
    pub const fn advance(&self) -> PositiveLength {
        self.advance
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

pub(super) fn shape_markers<'a>(
    flow: &'a ProductionTextFlow<'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    output_records: &mut u64,
) -> Result<Vec<ProductionListMarkerShape<'a>>, ProductionTextShapeError> {
    use ProductionTextShapeErrorKind as E;
    let maximum = limits.base().get().max_shaping_context_bytes;
    let mut output = Vec::new();
    for (index, item) in flow.list_items().iter().enumerate() {
        let owner = item.owner();
        let text = flow
            .list_marker_text(index)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let provenance = flow
            .list_marker_provenance(index)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let list = flow
            .lists()
            .get(item.list_index() as usize)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let style = list.style();
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
        validate_admitted_font_coverage(font, text)
            .map_err(|_| error(owner, E::MissingDeclaredFontCoverage))?;
        if linked_backend_record_bound(text).map_err(|e| error(owner, E::Backend(e)))? > maximum {
            return Err(error(owner, E::ContextLimit));
        }
        let face = harfrust::FontRef::from_index(font.bytes(), font.face_index())
            .map_err(|_| error(owner, E::InvalidFontMetrics))?;
        let hhea = face
            .hhea()
            .map_err(|_| error(owner, E::InvalidFontMetrics))?;
        let scale = |v| {
            scale_design_units(i32::from(v), size.get(), font.metadata().units_per_em)
                .map_err(|e| error(owner, E::Backend(e)))
        };
        let metrics = ProductionBodyFont {
            face_id,
            content_hash: font.content_hash(),
            face_index: font.face_index(),
            size,
            ascender: scale(hhea.ascender().to_i16())?,
            descender: scale(hhea.descender().to_i16())?,
            line_gap: scale(hhea.line_gap().to_i16())?,
        };
        let source = ShapeSourceSpan::Generated(provenance);
        let run_id =
            GlyphRunId::new(u32::try_from(index).map_err(|_| error(owner, E::OutputLimit))?);
        limits
            .base()
            .get()
            .max_fragments
            .checked_sub(*output_records)
            .and_then(|n| n.checked_sub(1))
            .filter(|n| *n > 0)
            .ok_or_else(|| error(owner, E::OutputLimit))?;
        // The backend's temporary ceiling includes its fixed 16,384-record
        // minimum even for one bullet. As for body runs, this is the shaping
        // context budget; exact retained output is charged to max_fragments
        // below, before the run is retained in the document result.
        let record_maximum = maximum;
        let mut budget = ShapeOutputBudget::new(record_maximum);
        // Canonical decimal-plus-period and bullet are a separate LTR label,
        // with no fabricated body context or inserted trailing space.
        let run = shape_linked(
            LinkedBackendInput {
                run_id,
                font: FontInstanceId::new(face_id.get()),
                source,
                utf8: text,
                font_bytes: font.bytes(),
                face_index: font.face_index(),
                admitted_units_per_em: font.metadata().units_per_em,
                admitted_glyph_count: font.metadata().glyph_count,
                font_size: size.get(),
                bidi_level: BidiLevel::LTR,
                script: OpenTypeTag::new(*b"DFLT")
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                language: Some(item.language()),
                pre_context: None,
                post_context: None,
                max_output_records: record_maximum,
            },
            &mut budget,
        )
        .map_err(|e| error(owner, E::Backend(e)))?;
        let expected = ExpectedGlyphRun {
            run_id,
            font: FontInstanceId::new(face_id.get()),
            bidi_level: BidiLevel::LTR,
            source,
            utf8_boundaries: utf8_boundaries(source, text)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
            glyph_count: font.metadata().glyph_count,
            max_output_records: record_maximum,
        };
        if !budget.matches_output(&run) || validate_glyph_run(&expected, &run).is_err() {
            return Err(error(owner, E::ReceiptMismatch));
        }
        if run.glyphs.iter().any(|g| g.original_gid.get() == 0) {
            return Err(error(owner, E::MissingGeneratedGlyph));
        }
        let advance = run
            .glyphs
            .iter()
            .try_fold(Length::ZERO, |n, g| n.checked_add(g.advance_x))
            .and_then(PositiveLength::new)
            .ok_or_else(|| error(owner, E::InvalidFontMetrics))?;
        *output_records = output_records
            .checked_add(1 + run.glyphs.len() as u64 + run.clusters.len() as u64)
            .filter(|n| *n <= limits.base().get().max_fragments)
            .ok_or_else(|| error(owner, E::OutputLimit))?;
        output
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        let capacity = run
            .glyphs
            .len()
            .checked_mul(34)
            .and_then(|n| n.checked_add(run.clusters.len().checked_mul(16)?))
            .and_then(|n| n.checked_add(160))
            .ok_or_else(|| error(owner, E::OutputLimit))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        bytes.extend_from_slice(&owner.get().to_be_bytes());
        bytes.extend_from_slice(&sha256(text.as_bytes()));
        bytes.extend_from_slice(&sha256(item.language().as_bytes()));
        bytes.extend_from_slice(&metrics.content_hash);
        for n in [
            face_id.get(),
            font.face_index(),
            provenance.text_span().text_id().get(),
        ] {
            bytes.extend_from_slice(&n.to_be_bytes());
        }
        for n in [
            metrics.size.get(),
            metrics.ascender,
            metrics.descender,
            metrics.line_gap,
        ] {
            bytes.extend_from_slice(&n.raw().to_be_bytes());
        }
        bytes.extend_from_slice(&(run.glyphs.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&(run.clusters.len() as u64).to_be_bytes());
        for g in &run.glyphs {
            bytes.extend_from_slice(&g.original_gid.get().to_be_bytes());
            for n in [g.advance_x, g.advance_y, g.offset_x, g.offset_y] {
                bytes.extend_from_slice(&n.raw().to_be_bytes());
            }
        }
        for c in &run.clusters {
            let ShapeSourceSpan::Generated(span) = c.source_span else {
                return Err(error(owner, E::ReceiptMismatch));
            };
            for n in [
                span.text_span().range().start_byte().get(),
                span.text_span().range().end_byte().get(),
                c.glyph_start,
                c.glyph_end,
            ] {
                bytes.extend_from_slice(&n.to_be_bytes());
            }
        }
        output.push(ProductionListMarkerShape {
            source: item,
            marker_index: index as u32,
            utf8: text,
            provenance,
            font: metrics,
            run,
            advance,
            fingerprint: sha256(&bytes),
        });
    }
    Ok(output)
}
