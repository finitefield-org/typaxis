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

/// One source-text cluster and its already-shaped glyph sequence for a
/// staging PDF text contribution. The resource finalizer never reshapes or
/// normalizes this input.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StagingPdfTextClusterUsage {
    font_face_id: FontFaceId,
    text_span: DisplayTextSpan,
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
            text_span,
            exact_text,
            glyphs,
        })
    }

    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }

    pub const fn text_span(&self) -> DisplayTextSpan {
        self.text_span
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
    pub const fn text_span(&self) -> DisplayTextSpan {
        self.usage.text_span
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
        self.clusters.iter().find(|cluster| {
            cluster.text_span() == text_span
                && cluster.exact_text() == exact_text
                && cluster.glyphs() == glyphs
        })
    }
}

/// Deterministically subset every selected equation-number font and assign a
/// canonical CID sequence to each distinct source cluster.
pub fn finalize_staging_pdf_text_fonts(
    admitted: &AdmittedResourceLedger,
    usages: &[StagingPdfTextClusterUsage],
    limits: &ValidatedResourceLimits,
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
                let cid = Cid::new(u16::try_from(next).map_err(|_| ResourceError::ResourceLimit)?)
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
            extraction_plans.push(if per_cid {
                ClusterExtractionPlan::PerCid {
                    text_span: usage.text_span,
                    cids: cids.clone(),
                }
            } else {
                ClusterExtractionPlan::ActualText {
                    text_span: usage.text_span,
                    cids: cids.clone(),
                    unicode: scalars,
                }
            });
            frozen_clusters.push(FrozenStagingPdfTextClusterPlan {
                usage,
                cids,
                requires_actual_text: !per_cid,
            });
        }

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
        extraction_plans.push(if requires_actual_text {
            ClusterExtractionPlan::ActualText {
                text_span: usage.text_span,
                cids: cids.clone(),
                unicode: scalars,
            }
        } else {
            ClusterExtractionPlan::PerCid {
                text_span: usage.text_span,
                cids: cids.clone(),
            }
        });
        frozen_clusters.push(FrozenStagingPdfTextClusterPlan {
            usage,
            cids,
            requires_actual_text,
        });
    }
    Ok((bindings, extraction_plans, frozen_clusters))
}
