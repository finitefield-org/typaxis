//! Shared itemization/output kernels with private pre-allocation hooks.
//! Bidi constructor scratch and owned shaping vectors share the caller ledger.
//! Harfrust input, language, shape plans and paired glyph-buffer growth also use it.
//! Later kernel scratch/work and allocator overcapacity remain separate.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Site {
    Bidi(unicode_bidi::scratch::Site),
    Harfrust(harfrust::scratch::Site),
    Graphemes,
    ItemizedRuns,
    NumberRuns,
    VisualClusters,
    LogicalClusters,
    Glyphs,
    Clusters,
    Utf8Boundaries,
    ValidationRanges,
    BodyParagraphs,
    BodyContext,
    BodyRanges,
    BodyReferences,
    BodyRuns,
    ListMarkers,
    FootnoteMarkers,
}

struct HarfrustBudget<'call, S>(&'call mut S);
impl<S: Storage> harfrust::scratch::Budget for HarfrustBudget<'_, S> {
    const OBSERVED: bool = S::OBSERVED;
    fn reserve(
        &mut self,
        site: harfrust::scratch::Site,
        count: usize,
        element_bytes: usize,
    ) -> Result<(), harfrust::scratch::Error> {
        let bytes = count
            .checked_mul(element_bytes)
            .ok_or(harfrust::scratch::Error::ArithmeticOverflow)?;
        self.0
            .reserve::<u8>(Site::Harfrust(site), bytes)
            .map_err(|_| harfrust::scratch::Error::Refused)
    }
    fn work(&mut self, amount: usize) -> Result<(), harfrust::scratch::Error> {
        let amount =
            u64::try_from(amount).map_err(|_| harfrust::scratch::Error::ArithmeticOverflow)?;
        self.0
            .work(amount)
            .map_err(|_| harfrust::scratch::Error::Refused)
    }
}

struct BidiBudget<'call, S>(&'call mut S);
impl<S: Storage> unicode_bidi::scratch::Budget for BidiBudget<'_, S> {
    const OBSERVED: bool = S::OBSERVED;
    fn reserve(
        &mut self,
        site: unicode_bidi::scratch::Site,
        count: usize,
        element_bytes: usize,
    ) -> Result<(), unicode_bidi::scratch::Error> {
        let bytes = count
            .checked_mul(element_bytes)
            .ok_or(unicode_bidi::scratch::Error::ArithmeticOverflow)?;
        self.0
            .reserve::<u8>(Site::Bidi(site), bytes)
            .map_err(|_| unicode_bidi::scratch::Error::Refused)
    }
    fn work(&mut self, amount: usize) -> Result<(), unicode_bidi::scratch::Error> {
        let amount =
            u64::try_from(amount).map_err(|_| unicode_bidi::scratch::Error::ArithmeticOverflow)?;
        self.0
            .work(amount)
            .map_err(|_| unicode_bidi::scratch::Error::Refused)
    }
}
pub(super) trait Storage {
    const OBSERVED: bool = true;
    fn owner(&mut self, _: NodeId) {}
    fn reserve<T>(&mut self, site: Site, count: usize) -> Result<(), LinkedShaperError>;
    fn records(&mut self, count: u64) -> Result<(), LinkedShaperError>;
    fn work(&mut self, count: u64) -> Result<(), LinkedShaperError>;
}
pub(super) struct NoStorage;
impl Storage for NoStorage {
    const OBSERVED: bool = false;
    fn reserve<T>(&mut self, _: Site, _: usize) -> Result<(), LinkedShaperError> {
        Ok(())
    }
    fn records(&mut self, _: u64) -> Result<(), LinkedShaperError> {
        Ok(())
    }
    fn work(&mut self, _: u64) -> Result<(), LinkedShaperError> {
        Ok(())
    }
}

pub(super) fn itemize_run_specs_with_storage<S: Storage>(
    utf8: &str,
    storage: &mut S,
) -> Result<(BidiLevel, Vec<ItemizedRunSpec>), ItemizationError> {
    storage
        .work(utf8.len() as u64)
        .map_err(map_linked_itemization_error)?;
    if utf8
        .chars()
        .any(|character| unicode_bidi::bidi_class(character) == unicode_bidi::BidiClass::B)
    {
        return Err(ItemizationError::ParagraphBoundaryUnsupported);
    }
    let bidi = if S::OBSERVED {
        unicode_bidi::BidiInfo::try_new_with_scratch(utf8, None, &mut BidiBudget(storage)).map_err(
            |e| match e {
                unicode_bidi::scratch::Error::ArithmeticOverflow => {
                    ItemizationError::ArithmeticOverflow
                }
                _ => ItemizationError::AllocationFailure,
            },
        )?
    } else {
        unicode_bidi::BidiInfo::new(utf8, None)
    };
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
    let mut count_reader = crate::shared_grapheme_storage::Graphemes::new(utf8, storage)
        .map_err(map_linked_itemization_error)?;
    let mut grapheme_count = 0usize;
    while count_reader
        .next(storage)
        .map_err(map_linked_itemization_error)?
        .is_some()
    {
        grapheme_count = grapheme_count
            .checked_add(1)
            .ok_or(ItemizationError::ArithmeticOverflow)?;
    }
    storage
        .reserve::<GraphemeItem>(Site::Graphemes, grapheme_count)
        .map_err(map_linked_itemization_error)?;
    let mut graphemes = Vec::new();
    graphemes
        .try_reserve_exact(grapheme_count)
        .map_err(|_| ItemizationError::AllocationFailure)?;
    let mut reader = crate::shared_grapheme_storage::Graphemes::new(utf8, storage)
        .map_err(map_linked_itemization_error)?;
    while let Some((start, cluster)) = reader.next(storage).map_err(map_linked_itemization_error)? {
        let end = start
            .checked_add(cluster.len())
            .ok_or(ItemizationError::ArithmeticOverflow)?;
        let level = *bidi
            .levels
            .get(start)
            .ok_or(ItemizationError::ArithmeticOverflow)?;
        if crate::shared_text_work::mixed_levels(&bidi.levels[start..end], level, storage)
            .map_err(map_linked_itemization_error)?
        {
            return Err(ItemizationError::BidiLevelSplitsGrapheme);
        }
        let candidates = crate::shared_text_work::script_candidates(cluster, storage)
            .map_err(map_linked_itemization_error)?;
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

    storage
        .reserve::<ItemizedRunSpec>(Site::ItemizedRuns, graphemes.len())
        .map_err(map_linked_itemization_error)?;
    let mut runs: Vec<ItemizedRunSpec> = Vec::new();
    runs.try_reserve_exact(graphemes.len())
        .map_err(|_| ItemizationError::AllocationFailure)?;
    let mut group_start = 0;
    while group_start < graphemes.len() {
        storage.work(1).map_err(map_linked_itemization_error)?;
        let bidi_level = graphemes[group_start].bidi_level;
        let mut group_end = group_start + 1;
        let mut intersection = graphemes[group_start].candidates;
        while group_end < graphemes.len() {
            storage.work(1).map_err(map_linked_itemization_error)?;
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
        storage.work(1).map_err(map_linked_itemization_error)?;
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

pub(super) fn shape_equation_number_runs_with_storage<S: Storage>(
    input: EquationNumberRunInput<'_>,
    storage: &mut S,
) -> Result<
    (
        BidiLevel,
        Vec<StagingEquationNumberGlyphRun>,
        PositiveLength,
    ),
    StagingEquationNumberShapeError,
> {
    let exact_text = input.exact_text;
    let text_span = input.text_span;
    let maximum_records = input.maximum_records;
    let exact_len = u32::try_from(exact_text.len())
        .map_err(|_| StagingEquationNumberShapeError::ArithmeticOverflow)?;
    let (paragraph_level, specs) = itemize_run_specs_with_storage(exact_text, storage)
        .map_err(map_equation_number_itemization_error)?;
    if specs.is_empty() {
        return Err(StagingEquationNumberShapeError::NonPositiveShape);
    }

    let mut remaining = input.total_records.unwrap_or(u64::MAX);
    remaining = remaining
        .checked_sub(specs.len() as u64)
        .ok_or(StagingEquationNumberShapeError::ContextLimit)?;
    storage
        .records(specs.len() as u64)
        .map_err(StagingEquationNumberShapeError::Backend)?;
    storage
        .reserve::<StagingEquationNumberGlyphRun>(Site::NumberRuns, specs.len())
        .map_err(StagingEquationNumberShapeError::Backend)?;
    let mut runs = Vec::new();
    runs.try_reserve_exact(specs.len())
        .map_err(|_| StagingEquationNumberShapeError::AllocationFailure)?;
    for (index, spec) in specs.into_iter().enumerate() {
        storage
            .work(1)
            .map_err(StagingEquationNumberShapeError::Backend)?;
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
        // Backend scratch has its own context ceiling; only validated retained
        // glyph/cluster records consume the enclosing document record quota.
        let run_maximum = maximum_records;
        if remaining < 2 {
            return Err(StagingEquationNumberShapeError::ContextLimit);
        }
        let mut budget = ShapeOutputBudget::new(run_maximum);
        let raw = shape_linked_with_storage(
            LinkedBackendInput {
                run_id,
                font: input.font_instance,
                source,
                utf8: run_text,
                font_bytes: input.font_bytes,
                face_index: input.face_index,
                admitted_units_per_em: input.units_per_em,
                admitted_glyph_count: input.glyph_count,
                font_size: input.font_size.get(),
                bidi_level: spec.bidi_level,
                script: spec.script,
                language: input.language,
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
                max_output_records: run_maximum,
            },
            &mut budget,
            |_, _| Ok(()),
            storage,
        )
        .map_err(StagingEquationNumberShapeError::Backend)?;
        if !budget.matches_output(&raw) {
            return Err(StagingEquationNumberShapeError::ReceiptMismatch);
        }
        let expected = ExpectedGlyphRun {
            run_id,
            font: input.font_instance,
            bidi_level: spec.bidi_level,
            source,
            utf8_boundaries: utf8_boundaries_with_storage(source, run_text, storage)
                .map_err(StagingEquationNumberShapeError::Backend)?,
            glyph_count: input.glyph_count,
            max_output_records: run_maximum,
        };
        validate_glyph_run_with_storage(&expected, &raw, storage).map_err(|error| match error {
            GlyphRunValidationError::AllocationFailure => {
                StagingEquationNumberShapeError::AllocationFailure
            }
            _ => StagingEquationNumberShapeError::ReceiptMismatch,
        })?;
        let ShapeSourceSpan::Parsed(source_span) = raw.source_span else {
            return Err(StagingEquationNumberShapeError::ReceiptMismatch);
        };
        remaining = remaining
            .checked_sub(raw.glyphs.len() as u64)
            .and_then(|n| n.checked_sub(raw.clusters.len() as u64))
            .ok_or(StagingEquationNumberShapeError::ContextLimit)?;
        runs.push(StagingEquationNumberGlyphRun {
            run_id: raw.run_id,
            bidi_level: raw.bidi_level,
            script: spec.script,
            source_span,
            glyphs: raw.glyphs,
            clusters: raw.clusters,
        });
    }

    let width = crate::shared_text_work::number_width(&runs, storage)
        .map_err(StagingEquationNumberShapeError::Backend)?
        .ok_or(StagingEquationNumberShapeError::NonPositiveShape)?;
    Ok((paragraph_level, runs, width))
}

pub(super) fn validate_backend_clusters_with_storage<S: Storage>(
    infos: &[harfrust::GlyphInfo],
    utf8: &str,
    level: BidiLevel,
    storage: &mut S,
) -> Result<u32, LinkedShaperError> {
    let text_len = u32::try_from(utf8.len()).map_err(|_| LinkedShaperError::ArithmeticOverflow)?;
    let mut cluster_count = 0u32;
    let mut previous = None;
    for info in infos {
        storage.work(1)?;
        if info.cluster >= text_len || !utf8.is_char_boundary(info.cluster as usize) {
            return Err(LinkedShaperError::InvalidBackendCluster);
        }
        if previous != Some(info.cluster) {
            if previous.is_some_and(|start| {
                if level.is_rtl() {
                    start <= info.cluster
                } else {
                    start >= info.cluster
                }
            }) {
                return Err(LinkedShaperError::NonMonotoneBackendClusters);
            }
            cluster_count = cluster_count
                .checked_add(1)
                .ok_or(LinkedShaperError::ArithmeticOverflow)?;
            previous = Some(info.cluster);
        }
    }
    let first_source = if level.is_rtl() {
        infos.last().map(|info| info.cluster)
    } else {
        infos.first().map(|info| info.cluster)
    };
    if first_source != Some(0) {
        return Err(LinkedShaperError::InvalidBackendCluster);
    }
    Ok(cluster_count)
}

pub(super) fn shape_linked_with_storage<S: Storage>(
    input: LinkedBackendInput<'_>,
    budget: &mut ShapeOutputBudget,
    reserve_output: impl FnOnce(u32, u32) -> Result<(), LinkedShaperError>,
    storage: &mut S,
) -> Result<GlyphRun, LinkedShaperError> {
    // head, maxp, cmap, and hmtx (which also reads hhea): five searches.
    let face =
        crate::font_reader::read_with_tables(input.font_bytes, input.face_index, 5, storage)?;
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

    let shaper_data = if S::OBSERVED {
        harfrust::ShaperData::try_new_with_scratch(&face, &mut HarfrustBudget(storage)).map_err(
            |error| match error {
                harfrust::scratch::Error::ArithmeticOverflow => {
                    LinkedShaperError::ArithmeticOverflow
                }
                _ => LinkedShaperError::AllocationFailure,
            },
        )?
    } else {
        harfrust::ShaperData::new(&face)
    };
    let shaper = shaper_data.shaper(&face).build();
    let mut unicode = harfrust::UnicodeBuffer::new();
    if S::OBSERVED {
        unicode
            .try_push_str_with_scratch(input.utf8, &mut HarfrustBudget(storage))
            .map_err(|error| match error {
                harfrust::scratch::Error::ArithmeticOverflow => {
                    LinkedShaperError::ArithmeticOverflow
                }
                harfrust::scratch::Error::InputLimit => {
                    LinkedShaperError::OutputBudget(ShapeWorkError::GlyphLimit)
                }
                _ => LinkedShaperError::AllocationFailure,
            })?;
    } else {
        unicode.push_str(input.utf8);
    }
    unicode.set_direction(if input.bidi_level.is_rtl() {
        harfrust::Direction::RightToLeft
    } else {
        harfrust::Direction::LeftToRight
    });
    unicode.set_script(script);
    if let Some(language) = input.language {
        let language = if S::OBSERVED {
            harfrust::Language::try_from_str_with_scratch(language, &mut HarfrustBudget(storage))
                .map_err(|error| match error {
                    harfrust::scratch::Error::InvalidLanguage => LinkedShaperError::InvalidLanguage,
                    harfrust::scratch::Error::ArithmeticOverflow => {
                        LinkedShaperError::ArithmeticOverflow
                    }
                    _ => LinkedShaperError::AllocationFailure,
                })?
        } else {
            language
                .parse()
                .map_err(|_| LinkedShaperError::InvalidLanguage)?
        };
        unicode.set_language(language);
    }
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

    let shaped = if S::OBSERVED {
        shaper
            .try_shape_with_scratch(unicode, &[], &mut HarfrustBudget(storage))
            .map_err(|error| match error {
                harfrust::scratch::Error::ArithmeticOverflow => {
                    LinkedShaperError::ArithmeticOverflow
                }
                harfrust::scratch::Error::InputLimit => {
                    LinkedShaperError::OutputBudget(ShapeWorkError::GlyphLimit)
                }
                _ => LinkedShaperError::AllocationFailure,
            })?
    } else {
        shaper.shape(unicode, &[])
    };
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

    // Count and validate the backend's borrowed output before allocating any
    // retained glyph/cluster vectors. The backend's temporary allowance above
    // remains independent of the caller's exact retained-output reservation.
    let cluster_count =
        validate_backend_clusters_with_storage(infos, input.utf8, input.bidi_level, storage)?;
    if cluster_count > input.max_output_records {
        return Err(LinkedShaperError::OutputBudget(
            ShapeWorkError::ClusterLimit,
        ));
    }
    reserve_output(glyph_count, cluster_count)?;
    storage.records(u64::from(glyph_count) + u64::from(cluster_count))?;

    storage.reserve::<BackendCluster>(Site::VisualClusters, infos.len())?;
    let mut visual_clusters: Vec<BackendCluster> = Vec::new();
    visual_clusters
        .try_reserve_exact(infos.len())
        .map_err(|_| LinkedShaperError::AllocationFailure)?;
    for (index, info) in infos.iter().enumerate() {
        storage.work(1)?;
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
    for pair in visual_clusters.windows(2) {
        storage.work(1)?;
        let invalid = if input.bidi_level.is_rtl() {
            pair[0].source_start <= pair[1].source_start
        } else {
            pair[0].source_start >= pair[1].source_start
        };
        if invalid {
            return Err(LinkedShaperError::NonMonotoneBackendClusters);
        }
    }

    storage.reserve::<BackendCluster>(Site::LogicalClusters, visual_clusters.len())?;
    let mut logical_clusters = Vec::new();
    logical_clusters
        .try_reserve_exact(visual_clusters.len())
        .map_err(|_| LinkedShaperError::AllocationFailure)?;
    storage.work(visual_clusters.len() as u64)?;
    logical_clusters.extend_from_slice(&visual_clusters);
    sort_by_key(
        &mut logical_clusters,
        |cluster| cluster.source_start,
        storage,
    )?;
    if logical_clusters.first().map(|cluster| cluster.source_start) != Some(0) {
        return Err(LinkedShaperError::InvalidBackendCluster);
    }
    for pair in logical_clusters.windows(2) {
        storage.work(1)?;
        if pair[0].source_start == pair[1].source_start {
            return Err(LinkedShaperError::NonContiguousBackendCluster);
        }
    }
    let cluster_count = u32::try_from(logical_clusters.len())
        .map_err(|_| LinkedShaperError::OutputBudget(ShapeWorkError::ClusterLimit))?;
    if cluster_count > input.max_output_records {
        return Err(LinkedShaperError::OutputBudget(
            ShapeWorkError::ClusterLimit,
        ));
    }

    storage.reserve::<ShapedGlyph>(Site::Glyphs, infos.len())?;
    let mut glyphs = Vec::new();
    glyphs
        .try_reserve_exact(infos.len())
        .map_err(|_| LinkedShaperError::AllocationFailure)?;
    for (info, position) in infos.iter().zip(positions) {
        storage.work(1)?;
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

    storage.reserve::<ShapedCluster>(Site::Clusters, logical_clusters.len())?;
    let mut clusters = Vec::new();
    clusters
        .try_reserve_exact(logical_clusters.len())
        .map_err(|_| LinkedShaperError::AllocationFailure)?;
    for (index, cluster) in logical_clusters.iter().enumerate() {
        storage.work(1)?;
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

pub(super) fn validate_glyph_run_with_storage<S: Storage>(
    expected: &ExpectedGlyphRun,
    run: &GlyphRun,
    storage: &mut S,
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
    storage
        .reserve::<(u32, u32)>(Site::ValidationRanges, run.clusters.len())
        .map_err(|_| GlyphRunValidationError::AllocationFailure)?;
    let mut visual_ranges = Vec::new();
    visual_ranges
        .try_reserve_exact(run.clusters.len())
        .map_err(|_| GlyphRunValidationError::AllocationFailure)?;
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
    sort_by_key(&mut visual_ranges, |range| *range, storage)
        .map_err(|_| GlyphRunValidationError::AllocationFailure)?;
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

pub(super) fn utf8_boundaries_with_storage<S: Storage>(
    source: ShapeSourceSpan,
    utf8: &str,
    storage: &mut S,
) -> Result<Vec<u32>, LinkedShaperError> {
    let (start, end) = source_range(source);
    storage.work(utf8.len() as u64)?;
    let capacity = utf8
        .chars()
        .count()
        .checked_add(1)
        .ok_or(LinkedShaperError::ArithmeticOverflow)?;
    storage.reserve::<u32>(Site::Utf8Boundaries, capacity)?;
    let mut boundaries = Vec::new();
    boundaries
        .try_reserve_exact(capacity)
        .map_err(|_| LinkedShaperError::AllocationFailure)?;
    for (index, _) in utf8.char_indices() {
        let offset = u32::try_from(index).map_err(|_| LinkedShaperError::ArithmeticOverflow)?;
        boundaries.push(
            start
                .checked_add(offset)
                .ok_or(LinkedShaperError::ArithmeticOverflow)?,
        );
    }
    if boundaries.last().copied() != Some(end) {
        boundaries.push(end);
    }
    Ok(boundaries)
}

// Count each comparison and swap, without relying on a standard-library sort's
// private comparison bound or scratch behavior. Keys, not storage, are copied.
fn sort_by_key<T, K: Ord, S: Storage>(
    v: &mut [T],
    key: impl Fn(&T) -> K,
    storage: &mut S,
) -> Result<(), LinkedShaperError> {
    if !S::OBSERVED {
        v.sort_unstable_by_key(key);
        return Ok(());
    }
    fn sift<T, K: Ord, S: Storage>(
        v: &mut [T],
        mut root: usize,
        end: usize,
        key: &impl Fn(&T) -> K,
        storage: &mut S,
    ) -> Result<(), LinkedShaperError> {
        while let Some(left) = root
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .filter(|n| *n < end)
        {
            let mut child = left;
            if left + 1 < end {
                storage.work(1)?;
                if key(&v[left]) < key(&v[left + 1]) {
                    child += 1;
                }
            }
            storage.work(1)?;
            if key(&v[root]) >= key(&v[child]) {
                break;
            }
            storage.work(1)?;
            v.swap(root, child);
            root = child;
        }
        Ok(())
    }
    for root in (0..v.len() / 2).rev() {
        sift(v, root, v.len(), &key, storage)?;
    }
    for end in (1..v.len()).rev() {
        storage.work(1)?;
        v.swap(0, end);
        sift(v, 0, end, &key, storage)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "shared_shape_storage_tests.rs"]
pub(crate) mod tests;
