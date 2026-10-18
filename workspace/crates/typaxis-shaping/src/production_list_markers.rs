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

pub(super) fn shape_markers<'a, S: Storage>(
    flow: BodyFlow<'a>,
    admitted: BodyFonts<'_>,
    limits: &M4EffectiveResourceLimits,
    output_records: &mut u64,
    maximum_records: Option<u64>,
    storage: &mut S,
) -> Result<Vec<ProductionListMarkerShape<'a>>, ProductionTextShapeError> {
    use ProductionTextShapeErrorKind as E;
    let mut output = Vec::new();
    for (index, item) in flow_call!(flow, list_items()).iter().enumerate() {
        let owner = item.owner();
        storage.owner(owner);
        storage.work(1).map_err(|e| error(owner, E::Backend(e)))?;
        let (provenance, text) = if S::OBSERVED {
            flow_call!(flow, list_marker_with_work(index, &mut |n| storage.work(n)))
                .map_err(|e| error(owner, E::Backend(e)))?
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?
        } else {
            let text = flow_call!(flow, list_marker_text(index))
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let provenance = flow_call!(flow, list_marker_provenance(index))
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            (provenance, text)
        };
        let list = flow_call!(flow, lists())
            .get(item.list_index() as usize)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let style = list.style();
        let shaped = shape_generated_marker(
            GeneratedMarkerInput {
                owner,
                index,
                text,
                provenance,
                families: style
                    .font_families()
                    .ok_or_else(|| error(owner, E::MissingTextStyle))?,
                size: style
                    .font_size()
                    .ok_or_else(|| error(owner, E::MissingTextStyle))?,
                language: item.language(),
            },
            admitted,
            limits,
            output_records,
            maximum_records,
            storage,
        )?;
        body_reserve_one(&mut output, storage, Site::ListMarkers, owner)?;
        output.push(ProductionListMarkerShape {
            source: item,
            marker_index: index as u32,
            utf8: shaped.utf8,
            provenance: shaped.provenance,
            font: shaped.font,
            run: shaped.run,
            advance: shaped.advance,
            fingerprint: shaped.fingerprint,
        });
    }
    Ok(output)
}

pub(super) struct GeneratedMarkerInput<'a> {
    pub owner: NodeId,
    pub index: usize,
    pub text: &'a str,
    pub provenance: GeneratedProvenance,
    pub families: &'a [String],
    pub size: PositiveLength,
    pub language: &'a str,
}
pub(super) struct GeneratedMarkerGlyphs<'a> {
    pub utf8: &'a str,
    pub provenance: GeneratedProvenance,
    pub font: ProductionBodyFont,
    pub run: GlyphRun,
    pub advance: PositiveLength,
    pub fingerprint: [u8; 32],
}
pub(super) fn shape_generated_marker<'a, S: Storage>(
    input: GeneratedMarkerInput<'a>,
    admitted: BodyFonts<'_>,
    limits: &M4EffectiveResourceLimits,
    output_records: &mut u64,
    maximum_records: Option<u64>,
    storage: &mut S,
) -> Result<GeneratedMarkerGlyphs<'a>, ProductionTextShapeError> {
    use ProductionTextShapeErrorKind as E;
    let GeneratedMarkerInput {
        owner,
        index,
        text,
        provenance,
        families,
        size,
        language,
    } = input;
    let maximum = limits.base().get().max_shaping_context_bytes;
    if provenance.buffer_key().owner() != owner {
        return Err(error(owner, E::ReceiptMismatch));
    }
    let (face_id, font) = admitted
        .select_with_storage(families, storage)
        .map_err(|kind| error(owner, kind))?;
    font.coverage_with_storage(text, storage).map_err(|kind| {
        error(
            owner,
            match kind {
                E::CffV2(_) => kind,
                _ => E::MissingDeclaredFontCoverage,
            },
        )
    })?;
    if linked_backend_record_bound(text).map_err(|e| error(owner, E::Backend(e)))? > maximum {
        return Err(error(owner, E::ContextLimit));
    }
    let face = crate::font_reader::read_with_tables(font.bytes(), font.face_index(), 1, storage)
        .map_err(|e| {
            error(
                owner,
                match e {
                    LinkedShaperError::InvalidFontOrFace => E::InvalidFontMetrics,
                    e => E::Backend(e),
                },
            )
        })?;
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
    let run_id = GlyphRunId::new(u32::try_from(index).map_err(|_| error(owner, E::OutputLimit))?);
    maximum_records
        .unwrap_or(limits.base().get().max_fragments)
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
    // Canonical list and footnote numbers/bullets are separate LTR labels,
    // with no fabricated body context or inserted trailing space.
    let run = shape_linked_with_record_limit(
        LinkedBackendInput {
            run_id,
            font: font.instance_id(),
            source,
            utf8: text,
            font_bytes: font.bytes(),
            face_index: font.face_index(),
            admitted_units_per_em: font.metadata().units_per_em,
            admitted_glyph_count: font.metadata().glyph_count,
            font_size: size.get(),
            bidi_level: BidiLevel::LTR,
            script: OpenTypeTag::new(*b"DFLT").ok_or_else(|| error(owner, E::ReceiptMismatch))?,
            language: Some(language),
            pre_context: None,
            post_context: None,
            max_output_records: record_maximum,
        },
        &mut budget,
        owner,
        output_records,
        maximum_records,
        storage,
    )?;
    let expected = ExpectedGlyphRun {
        run_id,
        font: font.instance_id(),
        bidi_level: BidiLevel::LTR,
        source,
        utf8_boundaries: shared_storage::utf8_boundaries_with_storage(source, text, storage)
            .map_err(|_| error(owner, E::ReceiptMismatch))?,
        glyph_count: font.metadata().glyph_count,
        max_output_records: record_maximum,
    };
    if !budget.matches_output(&run)
        || shared_storage::validate_glyph_run_with_storage(&expected, &run, storage).is_err()
    {
        return Err(error(owner, E::ReceiptMismatch));
    }
    for glyph in &run.glyphs {
        storage.work(1).map_err(|e| error(owner, E::Backend(e)))?;
        if glyph.original_gid.get() == 0 {
            return Err(error(owner, E::MissingGeneratedGlyph));
        }
    }
    let mut advance = Length::ZERO;
    for glyph in &run.glyphs {
        storage.work(1).map_err(|e| error(owner, E::Backend(e)))?;
        advance = advance
            .checked_add(glyph.advance_x)
            .ok_or_else(|| error(owner, E::InvalidFontMetrics))?;
    }
    let advance =
        PositiveLength::new(advance).ok_or_else(|| error(owner, E::InvalidFontMetrics))?;
    if maximum_records.is_none() {
        *output_records = output_records
            .checked_add(1 + run.glyphs.len() as u64 + run.clusters.len() as u64)
            .filter(|n| *n <= limits.base().get().max_fragments)
            .ok_or_else(|| error(owner, E::OutputLimit))?;
    }
    let _capacity = run
        .glyphs
        .len()
        .checked_mul(34)
        .and_then(|n| n.checked_add(run.clusters.len().checked_mul(16)?))
        .and_then(|n| n.checked_add(224))
        .ok_or_else(|| error(owner, E::OutputLimit))?;
    let mut bytes = Digest::new(owner, storage)?;
    bytes.update(&owner.get().to_be_bytes())?;
    bytes.hash_into(
        typaxis_core::generation_kind_wire_name(provenance.buffer_key().generation_kind())
            .as_bytes(),
    )?;
    bytes.update(&provenance.buffer_key().owner_local_ordinal().to_be_bytes())?;
    bytes.hash_into(text.as_bytes())?;
    bytes.hash_into(language.as_bytes())?;
    bytes.update(&metrics.content_hash)?;
    for n in [
        face_id.get(),
        font.face_index(),
        provenance.text_span().text_id().get(),
    ] {
        bytes.update(&n.to_be_bytes())?;
    }
    for n in [
        metrics.size.get(),
        metrics.ascender,
        metrics.descender,
        metrics.line_gap,
    ] {
        bytes.update(&n.raw().to_be_bytes())?;
    }
    bytes.update(&(run.glyphs.len() as u64).to_be_bytes())?;
    bytes.update(&(run.clusters.len() as u64).to_be_bytes())?;
    for g in &run.glyphs {
        bytes.update(&g.original_gid.get().to_be_bytes())?;
        for n in [g.advance_x, g.advance_y, g.offset_x, g.offset_y] {
            bytes.update(&n.raw().to_be_bytes())?;
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
            bytes.update(&n.to_be_bytes())?;
        }
    }
    Ok(GeneratedMarkerGlyphs {
        utf8: text,
        provenance,
        font: metrics,
        run,
        advance,
        fingerprint: bytes.finish()?,
    })
}
