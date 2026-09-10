use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{DisplayTextSpan, FontFaceId, FontInstanceId, ValidatedResourceLimits};
use typaxis_font::{
    Cff1Subset, Cff1SubsetSession, Cid, CidBinding, FontSubsetPlan, OriginalGlyphId, UnicodeScalar,
};
use typaxis_resource_admission::{AdmittedFont, AdmittedFontMediaKind, AdmittedResourceLedger};

use super::{
    subset_truetype, validate_original_glyph_bounds, validate_pdf_font_metrics,
    validate_subset_postscript_name, Cff1FontEncoderOutput, ClusterExtractionPlan,
    FontEncoderOutput, FrozenPdfFontPlan, ResourceError, VerifiedEncoderOutput,
    VerifiedEncoderReceiptOwner,
};

/// Native glyph identity is tied to an issued draw, never to a fabricated text range.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct NativeMathGlyphSource {
    owner: typaxis_core::NodeId,
    receipt_sha256: [u8; 32],
    computation_sha256: [u8; 32],
    paint_index: u32,
    logical_ordinal: u32,
}
impl NativeMathGlyphSource {
    pub const fn owner(&self) -> typaxis_core::NodeId {
        self.owner
    }
    pub const fn receipt_sha256(&self) -> [u8; 32] {
        self.receipt_sha256
    }
    pub const fn computation_sha256(&self) -> [u8; 32] {
        self.computation_sha256
    }
    pub const fn paint_index(&self) -> u32 {
        self.paint_index
    }
    pub const fn logical_ordinal(&self) -> u32 {
        self.logical_ordinal
    }
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PdfFontClusterSource {
    Text(DisplayTextSpan),
    NativeMathGlyph(NativeMathGlyphSource),
}

/// One selected text cluster or native math glyph with its nominal source.
/// The resource finalizer never reshapes or normalizes this input.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StagingPdfTextClusterUsage {
    font_face_id: FontFaceId,
    source: PdfFontClusterSource,
    exact_text: String,
    glyphs: Vec<OriginalGlyphId>,
}

impl StagingPdfTextClusterUsage {
    pub fn new(
        font_face_id: FontFaceId,
        text_span: DisplayTextSpan,
        exact_text: String,
        glyphs: Vec<OriginalGlyphId>,
    ) -> Result<Self, ResourceError> {
        let range = text_span.range();
        let span_length = range
            .end_byte()
            .get()
            .checked_sub(range.start_byte().get())
            .ok_or(ResourceError::InvalidFontPlan)?;
        if exact_text.is_empty()
            || u32::try_from(exact_text.len()) != Ok(span_length)
            || glyphs.is_empty()
        {
            return Err(ResourceError::InvalidFontPlan);
        }
        Ok(Self {
            font_face_id,
            source: PdfFontClusterSource::Text(text_span),
            exact_text,
            glyphs,
        })
    }

    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }

    pub const fn source(&self) -> &PdfFontClusterSource {
        &self.source
    }
    pub const fn text_span(&self) -> Option<DisplayTextSpan> {
        match self.source {
            PdfFontClusterSource::Text(span) => Some(span),
            _ => None,
        }
    }
    pub fn from_native_math_glyph(
        draw: &typaxis_display_list::ProductionBodyNativeMathDraw<'_>,
        paint_index: usize,
    ) -> Result<Self, ResourceError> {
        let Some(typaxis_display_list::ProductionNativeMathPaint::Glyph {
            original_gid,
            unicode,
            logical_ordinal,
            ..
        }) = draw.paints().get(paint_index)
        else {
            return Err(ResourceError::InvalidFontPlan);
        };
        Ok(Self {
            font_face_id: draw.receipt().font_face_id(),
            source: PdfFontClusterSource::NativeMathGlyph(NativeMathGlyphSource {
                owner: draw.owner(),
                receipt_sha256: draw.receipt().key().bytes(),
                computation_sha256: draw.receipt().computation().fingerprint(),
                paint_index: u32::try_from(paint_index)
                    .map_err(|_| ResourceError::ResourceLimit)?,
                logical_ordinal: *logical_ordinal,
            }),
            exact_text: unicode.to_string(),
            glyphs: vec![*original_gid],
        })
    }
    fn extraction_plan(
        &self,
        cids: Vec<Cid>,
        unicode: Vec<UnicodeScalar>,
        actual: bool,
    ) -> ClusterExtractionPlan {
        match &self.source {
            PdfFontClusterSource::Text(text_span) if actual => ClusterExtractionPlan::ActualText {
                text_span: *text_span,
                cids,
                unicode,
            },
            PdfFontClusterSource::Text(text_span) => ClusterExtractionPlan::PerCid {
                text_span: *text_span,
                cids,
            },
            PdfFontClusterSource::NativeMathGlyph(source) => {
                ClusterExtractionPlan::NativeMathGlyph {
                    source: source.clone(),
                    cids,
                    unicode,
                }
            }
        }
    }

    pub fn exact_text(&self) -> &str {
        &self.exact_text
    }

    pub fn glyphs(&self) -> &[OriginalGlyphId] {
        &self.glyphs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenStagingPdfTextClusterPlan {
    usage: StagingPdfTextClusterUsage,
    cids: Vec<Cid>,
    requires_actual_text: bool,
}

impl FrozenStagingPdfTextClusterPlan {
    pub const fn source(&self) -> &PdfFontClusterSource {
        self.usage.source()
    }
    pub const fn text_span(&self) -> Option<DisplayTextSpan> {
        self.usage.text_span()
    }

    pub fn exact_text(&self) -> &str {
        &self.usage.exact_text
    }

    pub fn glyphs(&self) -> &[OriginalGlyphId] {
        &self.usage.glyphs
    }

    pub fn cids(&self) -> &[Cid] {
        &self.cids
    }

    pub const fn requires_actual_text(&self) -> bool {
        self.requires_actual_text
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenStagingPdfTextFontPlan {
    font_face_id: FontFaceId,
    pdf_font: FrozenPdfFontPlan,
    clusters: Vec<FrozenStagingPdfTextClusterPlan>,
}

impl FrozenStagingPdfTextFontPlan {
    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }

    pub const fn pdf_font(&self) -> &FrozenPdfFontPlan {
        &self.pdf_font
    }

    pub fn clusters(&self) -> &[FrozenStagingPdfTextClusterPlan] {
        &self.clusters
    }

    pub fn cluster(
        &self,
        text_span: DisplayTextSpan,
        exact_text: &str,
        glyphs: &[OriginalGlyphId],
    ) -> Option<&FrozenStagingPdfTextClusterPlan> {
        // Every plan is built in the BTreeSet usage order, within one face.
        self.clusters
            .binary_search_by(|cluster| {
                cluster
                    .source()
                    .cmp(&PdfFontClusterSource::Text(text_span))
                    .then_with(|| cluster.exact_text().cmp(exact_text))
                    .then_with(|| cluster.glyphs().cmp(glyphs))
            })
            .ok()
            .map(|index| &self.clusters[index])
    }
}

/// Deterministically subset every selected equation-number font and assign a
/// canonical CID sequence to each distinct source cluster.
pub fn finalize_staging_pdf_text_fonts(
    admitted: &AdmittedResourceLedger,
    usages: &[StagingPdfTextClusterUsage],
    limits: &ValidatedResourceLimits,
) -> Result<Vec<FrozenStagingPdfTextFontPlan>, ResourceError> {
    finalize_text_fonts(admitted, usages, limits, false)
}

// Only the sealed production display bridge may authorize this input.
pub(crate) fn finalize_production_text_fonts(
    admitted: &AdmittedResourceLedger,
    usages: &[StagingPdfTextClusterUsage],
    limits: &ValidatedResourceLimits,
) -> Result<Vec<FrozenStagingPdfTextFontPlan>, ResourceError> {
    finalize_text_fonts(admitted, usages, limits, true)
}

fn finalize_text_fonts(
    admitted: &AdmittedResourceLedger,
    usages: &[StagingPdfTextClusterUsage],
    limits: &ValidatedResourceLimits,
    shared_cids: bool,
) -> Result<Vec<FrozenStagingPdfTextFontPlan>, ResourceError> {
    if usages.is_empty() {
        return Ok(Vec::new());
    }
    let mut by_font = BTreeMap::<FontFaceId, BTreeSet<StagingPdfTextClusterUsage>>::new();
    for usage in usages {
        by_font
            .entry(usage.font_face_id)
            .or_default()
            .insert(usage.clone());
    }

    let owner = VerifiedEncoderReceiptOwner::new();
    let mut output = Vec::new();
    output
        .try_reserve_exact(by_font.len())
        .map_err(|_| ResourceError::ResourceLimit)?;
    let mut aggregate_subset_bytes = 0u64;
    let mut cff1_session = None::<Cff1SubsetSession>;
    for (font_face_id, clusters) in by_font {
        let admitted_font = admitted
            .font(font_face_id)
            .ok_or(ResourceError::MissingLogicalResource)?;
        let requested = clusters
            .iter()
            .flat_map(|cluster| cluster.glyphs.iter().copied())
            .collect::<BTreeSet<_>>();
        if admitted_font.media_kind() == AdmittedFontMediaKind::SfntCff1 {
            let admission = admitted_font
                .cff1_admission()
                .ok_or(ResourceError::InvalidFontPlan)?;
            let session =
                cff1_session.get_or_insert_with(|| Cff1SubsetSession::from_admission(admission));
            let subset = session
                .subset(
                    admission,
                    font_face_id,
                    FontInstanceId::new(font_face_id.get()),
                    &requested,
                    limits.get().max_cids_per_font,
                )
                .map_err(ResourceError::Cff1)?;
            let (cid_bindings, extraction_plans, frozen_clusters) =
                build_staging_cff1_plans(&clusters, &subset, limits)?;
            let receipt = owner.issue_cff1_font(Cff1FontEncoderOutput {
                font_face_id,
                font_instance_id: FontInstanceId::new(font_face_id.get()),
                admission,
                subset,
                cids: cid_bindings,
                cluster_plans: extraction_plans,
                profile_fingerprint: admitted_font
                    .m4_profile_fingerprint()
                    .ok_or(ResourceError::InvalidFontPlan)?,
            })?;
            let VerifiedEncoderOutput::Font(pdf_font) = receipt.0 else {
                return Err(ResourceError::InvalidFontPlan);
            };
            validate_staging_font_plan(&pdf_font, admitted_font)?;
            aggregate_subset_bytes = aggregate_subset_bytes
                .checked_add(
                    u64::try_from(pdf_font.subset_bytes().len())
                        .map_err(|_| ResourceError::ResourceLimit)?,
                )
                .ok_or(ResourceError::ResourceLimit)?;
            if aggregate_subset_bytes > limits.get().max_spool_bytes {
                return Err(ResourceError::ResourceLimit);
            }
            output.push(FrozenStagingPdfTextFontPlan {
                font_face_id,
                pdf_font: *pdf_font,
                clusters: frozen_clusters,
            });
            continue;
        }
        if !matches!(
            admitted_font.media_kind(),
            AdmittedFontMediaKind::SfntTrueTypeGlyf | AdmittedFontMediaKind::TtcTrueTypeGlyf
        ) {
            return Err(ResourceError::InvalidFontPlan);
        }
        let subset = subset_truetype(
            admitted_font.bytes(),
            admitted_font.face_index(),
            &requested,
        )?;
        let (cid_bindings, extraction_plans, frozen_clusters) = if shared_cids {
            build_production_truetype_plans(
                &clusters,
                &subset,
                admitted_font.metadata().units_per_em,
                limits,
            )?
        } else {
            let mut cid_bindings = Vec::new();
            let mut extraction_plans = Vec::new();
            let mut frozen_clusters = Vec::new();
            extraction_plans
                .try_reserve_exact(clusters.len())
                .map_err(|_| ResourceError::ResourceLimit)?;
            frozen_clusters
                .try_reserve_exact(clusters.len())
                .map_err(|_| ResourceError::ResourceLimit)?;
            for usage in clusters {
                let scalars = usage
                    .exact_text
                    .chars()
                    .map(UnicodeScalar::new)
                    .collect::<Vec<_>>();
                let per_cid = scalars.len() == usage.glyphs.len();
                let mut cids = Vec::new();
                cids.try_reserve_exact(usage.glyphs.len())
                    .map_err(|_| ResourceError::ResourceLimit)?;
                for (index, glyph) in usage.glyphs.iter().enumerate() {
                    let next = cid_bindings
                        .len()
                        .checked_add(1)
                        .ok_or(ResourceError::ResourceLimit)?;
                    if next > usize::from(limits.get().max_cids_per_font) {
                        return Err(ResourceError::ResourceLimit);
                    }
                    let cid =
                        Cid::new(u16::try_from(next).map_err(|_| ResourceError::ResourceLimit)?)
                            .ok_or(ResourceError::ResourceLimit)?;
                    let subset_gid = *subset
                        .original_to_subset
                        .get(glyph)
                        .ok_or(ResourceError::InvalidFontPlan)?;
                    let advance = *subset
                        .original_widths
                        .get(glyph)
                        .ok_or(ResourceError::InvalidFontPlan)?;
                    let width_1000 = u32::try_from(
                        (u64::from(advance) * 1_000
                            + u64::from(admitted_font.metadata().units_per_em) / 2)
                            / u64::from(admitted_font.metadata().units_per_em),
                    )
                    .map_err(|_| ResourceError::InvalidFontPlan)?;
                    cid_bindings.push(CidBinding {
                        cid,
                        subset_gid,
                        unicode: if per_cid {
                            vec![scalars[index]]
                        } else {
                            Vec::new()
                        },
                        width_1000,
                    });
                    cids.push(cid);
                }
                extraction_plans.push(usage.extraction_plan(cids.clone(), scalars, !per_cid));
                frozen_clusters.push(FrozenStagingPdfTextClusterPlan {
                    usage,
                    cids,
                    requires_actual_text: !per_cid,
                });
            }

            (cid_bindings, extraction_plans, frozen_clusters)
        };

        let glyphs = subset
            .original_to_subset
            .iter()
            .map(
                |(original_gid, subset_gid)| typaxis_font::GlyphSubsetBinding {
                    original_gid: *original_gid,
                    subset_gid: *subset_gid,
                },
            )
            .collect();
        let receipt = owner.issue_font(FontEncoderOutput {
            font_instance_id: FontInstanceId::new(font_face_id.get()),
            admitted_sha256: admitted_font.content_hash(),
            subset_bytes: subset.bytes,
            subset_plan: FontSubsetPlan {
                glyphs,
                cids: cid_bindings,
            },
            metrics: subset.metrics,
            cluster_plans: extraction_plans,
        })?;
        let VerifiedEncoderOutput::Font(pdf_font) = receipt.0 else {
            return Err(ResourceError::InvalidFontPlan);
        };
        validate_staging_font_plan(&pdf_font, admitted_font)?;
        aggregate_subset_bytes = aggregate_subset_bytes
            .checked_add(
                u64::try_from(pdf_font.subset_bytes().len())
                    .map_err(|_| ResourceError::ResourceLimit)?,
            )
            .ok_or(ResourceError::ResourceLimit)?;
        if aggregate_subset_bytes > limits.get().max_spool_bytes {
            return Err(ResourceError::ResourceLimit);
        }
        output.push(FrozenStagingPdfTextFontPlan {
            font_face_id,
            pdf_font: *pdf_font,
            clusters: frozen_clusters,
        });
    }
    output.sort_by_key(|font| {
        (
            font.pdf_font.admitted_sha256(),
            font.pdf_font.font_instance_id(),
        )
    });
    Ok(output)
}

fn validate_staging_font_plan(
    pdf_font: &FrozenPdfFontPlan,
    admitted_font: &AdmittedFont,
) -> Result<(), ResourceError> {
    validate_pdf_font_metrics(pdf_font.metrics(), pdf_font.program_kind())?;
    validate_subset_postscript_name(pdf_font)?;
    validate_original_glyph_bounds(pdf_font.subset_plan(), admitted_font.metadata().glyph_count)?;
    pdf_font
        .subset_plan()
        .validate()
        .map_err(|_| ResourceError::InvalidFontPlan)?;
    match (pdf_font.program_kind(), admitted_font.media_kind()) {
        (
            super::PdfFontProgramKind::TrueTypeGlyf,
            AdmittedFontMediaKind::SfntTrueTypeGlyf | AdmittedFontMediaKind::TtcTrueTypeGlyf,
        ) => Ok(()),
        (super::PdfFontProgramKind::OpenTypeCff1, AdmittedFontMediaKind::SfntCff1) => {
            let required = pdf_font
                .subset_plan()
                .glyphs
                .iter()
                .map(|binding| binding.original_gid)
                .collect::<BTreeSet<_>>();
            super::validate_cff1_font_plan(pdf_font, admitted_font, &required)
        }
        _ => Err(ResourceError::InvalidFontPlan),
    }
}

type StagingCff1Plans = (
    Vec<CidBinding>,
    Vec<ClusterExtractionPlan>,
    Vec<FrozenStagingPdfTextClusterPlan>,
);

fn build_staging_cff1_plans(
    clusters: &BTreeSet<StagingPdfTextClusterUsage>,
    subset: &Cff1Subset,
    limits: &ValidatedResourceLimits,
) -> Result<StagingCff1Plans, ResourceError> {
    if clusters
        .iter()
        .any(|cluster| cluster.glyphs.contains(&OriginalGlyphId::new(0)))
    {
        return Err(ResourceError::InvalidFontPlan);
    }
    let mut unicode_by_glyph = BTreeMap::<OriginalGlyphId, Option<UnicodeScalar>>::new();
    for cluster in clusters {
        let scalars = cluster
            .exact_text
            .chars()
            .map(UnicodeScalar::new)
            .collect::<Vec<_>>();
        if scalars.len() != cluster.glyphs.len() {
            for glyph in &cluster.glyphs {
                unicode_by_glyph.insert(*glyph, None);
            }
            continue;
        }
        for (glyph, scalar) in cluster.glyphs.iter().zip(scalars) {
            match unicode_by_glyph.entry(*glyph) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(Some(scalar));
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    if entry.get().is_some_and(|existing| existing != scalar) {
                        entry.insert(None);
                    }
                }
            }
        }
    }

    let mut bindings = Vec::new();
    bindings
        .try_reserve_exact(subset.original_to_subset().len().saturating_sub(1))
        .map_err(|_| ResourceError::ResourceLimit)?;
    for (original_gid, subset_gid) in subset.original_to_subset() {
        if original_gid.get() == 0 {
            continue;
        }
        let expected = bindings
            .len()
            .checked_add(1)
            .ok_or(ResourceError::ResourceLimit)?;
        if expected > usize::from(limits.get().max_cids_per_font)
            || usize::from(subset_gid.get()) != expected
        {
            return Err(ResourceError::InvalidFontPlan);
        }
        bindings.push(CidBinding {
            cid: Cid::new(subset_gid.get()).ok_or(ResourceError::InvalidFontPlan)?,
            subset_gid: *subset_gid,
            unicode: unicode_by_glyph
                .get(original_gid)
                .copied()
                .flatten()
                .into_iter()
                .collect(),
            width_1000: u32::from(
                *subset
                    .original_widths()
                    .get(original_gid)
                    .ok_or(ResourceError::InvalidFontPlan)?,
            ),
        });
    }

    let mut extraction_plans = Vec::new();
    let mut frozen_clusters = Vec::new();
    extraction_plans
        .try_reserve_exact(clusters.len())
        .map_err(|_| ResourceError::ResourceLimit)?;
    frozen_clusters
        .try_reserve_exact(clusters.len())
        .map_err(|_| ResourceError::ResourceLimit)?;
    for usage in clusters.iter().cloned() {
        let scalars = usage
            .exact_text
            .chars()
            .map(UnicodeScalar::new)
            .collect::<Vec<_>>();
        let mut cids = Vec::new();
        cids.try_reserve_exact(usage.glyphs.len())
            .map_err(|_| ResourceError::ResourceLimit)?;
        for glyph in &usage.glyphs {
            let subset_gid = subset
                .original_to_subset()
                .get(glyph)
                .ok_or(ResourceError::InvalidFontPlan)?;
            cids.push(Cid::new(subset_gid.get()).ok_or(ResourceError::InvalidFontPlan)?);
        }
        let extracted = cids
            .iter()
            .flat_map(|cid| bindings[usize::from(cid.get()) - 1].unicode.iter().copied())
            .collect::<Vec<_>>();
        let requires_actual_text = extracted != scalars;
        extraction_plans.push(usage.extraction_plan(cids.clone(), scalars, requires_actual_text));
        frozen_clusters.push(FrozenStagingPdfTextClusterPlan {
            usage,
            cids,
            requires_actual_text,
        });
    }
    Ok((bindings, extraction_plans, frozen_clusters))
}

// Pure Unicode-claim merge shared with book-2. Each caller still verifies
// and owns its original source/glyph occurrence; no receipt is translated.
pub(crate) fn merge_single_glyph_unicode(
    current: Option<UnicodeScalar>, observed: UnicodeScalar,
) -> Option<UnicodeScalar> {
    current.filter(|value| *value == observed)
}

/// CID count follows glyph diversity, while extraction remains occurrence-local.
fn build_production_truetype_plans(
    clusters: &BTreeSet<StagingPdfTextClusterUsage>,
    subset: &super::TrueTypeSubset,
    units_per_em: u16,
    limits: &ValidatedResourceLimits,
) -> Result<StagingCff1Plans, ResourceError> {
    let mut unicode_by_glyph = BTreeMap::<OriginalGlyphId, Option<UnicodeScalar>>::new();
    let mut requested = BTreeSet::new();
    for cluster in clusters {
        for glyph in &cluster.glyphs {
            if glyph.get() == 0 {
                return Err(ResourceError::InvalidFontPlan);
            }
            requested.insert(*glyph);
        }
        let mut chars = cluster.exact_text.chars();
        let first = chars.next();
        if cluster.glyphs.len() == 1 && first.is_some() && chars.next().is_none() {
            let scalar = UnicodeScalar::new(first.unwrap());
            unicode_by_glyph
                .entry(cluster.glyphs[0])
                .and_modify(|current| {
                    *current = merge_single_glyph_unicode(*current, scalar);
                })
                .or_insert(Some(scalar));
        }
    }
    if requested.len() > usize::from(limits.get().max_cids_per_font) {
        return Err(ResourceError::ResourceLimit);
    }
    let mut bindings = Vec::new();
    bindings
        .try_reserve_exact(requested.len())
        .map_err(|_| ResourceError::ResourceLimit)?;
    let mut cid_by_glyph = BTreeMap::new();
    for glyph in requested {
        let cid =
            Cid::new(u16::try_from(bindings.len() + 1).map_err(|_| ResourceError::ResourceLimit)?)
                .ok_or(ResourceError::ResourceLimit)?;
        let subset_gid = *subset
            .original_to_subset
            .get(&glyph)
            .ok_or(ResourceError::InvalidFontPlan)?;
        let advance = *subset
            .original_widths
            .get(&glyph)
            .ok_or(ResourceError::InvalidFontPlan)?;
        let units = u64::from(units_per_em);
        if units == 0 {
            return Err(ResourceError::InvalidFontPlan);
        }
        let width_1000 = u32::try_from((u64::from(advance) * 1000 + units / 2) / units)
            .map_err(|_| ResourceError::InvalidFontPlan)?;
        bindings.push(CidBinding {
            cid,
            subset_gid,
            width_1000,
            unicode: unicode_by_glyph
                .get(&glyph)
                .copied()
                .flatten()
                .into_iter()
                .collect(),
        });
        cid_by_glyph.insert(glyph, cid);
    }
    let mut extraction = Vec::new();
    let mut frozen = Vec::new();
    extraction
        .try_reserve_exact(clusters.len())
        .map_err(|_| ResourceError::ResourceLimit)?;
    frozen
        .try_reserve_exact(clusters.len())
        .map_err(|_| ResourceError::ResourceLimit)?;
    for usage in clusters.iter().cloned() {
        let cids = usage
            .glyphs
            .iter()
            .map(|g| {
                cid_by_glyph
                    .get(g)
                    .copied()
                    .ok_or(ResourceError::InvalidFontPlan)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let scalars = usage
            .exact_text
            .chars()
            .map(UnicodeScalar::new)
            .collect::<Vec<_>>();
        let extracted = cids
            .iter()
            .flat_map(|cid| bindings[usize::from(cid.get()) - 1].unicode.iter().copied());
        let requires_actual_text = !extracted.eq(scalars.iter().copied());
        extraction.push(usage.extraction_plan(cids.clone(), scalars, requires_actual_text));
        frozen.push(FrozenStagingPdfTextClusterPlan {
            usage,
            cids,
            requires_actual_text,
        });
    }
    Ok((bindings, extraction, frozen))
}

#[cfg(test)]
mod production_tests {
    use super::*;
    use typaxis_core::{DisplayTextBufferId, ResourceLimits, Utf8ByteOffset};
    fn usage(index: u32, text: &str, gids: &[u16]) -> StagingPdfTextClusterUsage {
        StagingPdfTextClusterUsage::new(
            FontFaceId::new(0),
            DisplayTextSpan::new(
                DisplayTextBufferId::new(index),
                Utf8ByteOffset::new(0),
                Utf8ByteOffset::new(text.len() as u32),
            )
            .unwrap(),
            text.to_owned(),
            gids.iter().map(|g| OriginalGlyphId::new(*g)).collect(),
        )
        .unwrap()
    }
    #[test]
    fn production_shared_cids_preserve_ambiguous_ligature_and_multiglyph_cluster_text() {
        // These are intentional mapping-level usages, not an authored-shape receipt.
        let clusters = [
            usage(0, "A", &[1]),
            usage(1, "B", &[1]),
            usage(2, "fi", &[2]),
            usage(3, "e\u{301}", &[1, 2]),
            usage(4, "AB", &[1, 2]),
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        let source = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/profiles/basic-document-1/combined/job/body.ttf"
        ));
        let requested = [OriginalGlyphId::new(1), OriginalGlyphId::new(2)]
            .into_iter()
            .collect();
        let subset = subset_truetype(source, 0, &requested).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let (bindings, extraction, frozen) =
            build_production_truetype_plans(&clusters, &subset, 1000, &limits).unwrap();
        assert_eq!(bindings.len(), 2);
        assert!(bindings.iter().all(|b| b.unicode.is_empty()));
        assert!(frozen.iter().all(|c| c.requires_actual_text()));
        for (cluster, plan) in frozen.iter().zip(extraction) {
            let ClusterExtractionPlan::ActualText {
                unicode,
                text_span,
                cids,
            } = plan
            else {
                panic!("exact cluster text")
            };
            assert_eq!(Some(text_span), cluster.text_span());
            assert_eq!(cids, cluster.cids());
            assert_eq!(
                unicode,
                cluster
                    .exact_text()
                    .chars()
                    .map(UnicodeScalar::new)
                    .collect::<Vec<_>>()
            );
        }
        let lower = ValidatedResourceLimits::new(ResourceLimits {
            max_cids_per_font: 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert_eq!(
            build_production_truetype_plans(&clusters, &subset, 1000, &lower),
            Err(ResourceError::ResourceLimit)
        );
    }
}
