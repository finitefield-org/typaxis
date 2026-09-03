#![forbid(unsafe_code)]

mod safe_vector;
mod vector_content;

pub use safe_vector::{
    finalize_staging_safe_vector_forms, FrozenSafeVectorFormPlan, StagingSafeVectorFormPlans,
    StagingSafeVectorResourceError, StagingSafeVectorUsage,
    STAGING_SAFE_VECTOR_FORM_PLANS_ALGORITHM, STAGING_SAFE_VECTOR_FORM_PLAN_ALGORITHM,
};
#[cfg(any(test, feature = "staging-fixtures"))]
pub use safe_vector::{staging_safe_vector_resource_fixture, StagingSafeVectorResourceFixture};
pub use typaxis_resource_admission::{VectorContentKey, VectorContentMediaType};
pub use vector_content::{
    VectorContentAlias, VectorContentAliasProvenance, VectorContentCandidate,
    VectorContentCandidateRegistry, VectorContentPlanningError, VectorExtGStateAlphaPair,
    VectorExtGStatePlan, VectorExtGStatePlanEntry, VectorFormDedupeReceipt,
    VECTOR_FORM_DEDUPE_ALGORITHM,
};

use core::num::NonZeroU32;
use std::collections::{BTreeMap, BTreeSet};
use typaxis_core::{DisplayTextSpan, FontInstanceId, ImageResourceId, ValidatedResourceLimits};
use typaxis_display_list::{
    ClusterExtraction, DisplayCommand, DisplayDocument, ValidatedDisplayDocument,
};
use typaxis_font::{
    Cid, CidBinding, FontSubsetPlan, GlyphSubsetBinding, OriginalGlyphId, SubsetGlyphId,
    UnicodeScalar,
};

pub use typaxis_resource_admission::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceError {
    MissingLogicalResource,
    ConflictingLogicalResource,
    DuplicateFontInstance,
    FontInstanceHashMismatch,
    InvalidFontPlan,
    ResourceLimit,
    DuplicatePlanKey,
    InvalidImagePlan,
    IncompleteUsagePlan,
    UnexpectedLogicalResource,
    NonCanonicalFontInstanceKey,
    AdmittedLedgerEpochMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontInstanceTable {
    admitted: AdmittedFontInstanceTable,
}
impl FontInstanceTable {
    pub fn from_display(
        display: &ValidatedDisplayDocument,
        admitted: &AdmittedResources,
    ) -> Result<Self, ResourceError> {
        let table = AdmittedFontInstanceTable::from_used_faces(
            admitted,
            display
                .document()
                .font_instances
                .iter()
                .map(|instance| instance.font_face_id),
        )
        .map_err(|_| ResourceError::MissingLogicalResource)?;
        if table.instances().len() != display.document().font_instances.len()
            || table
                .instances()
                .iter()
                .zip(&display.document().font_instances)
                .any(|(admitted, displayed)| {
                    admitted.font_instance_id() != displayed.font_instance_id
                        || admitted.font_face_id() != displayed.font_face_id
                })
        {
            return Err(ResourceError::NonCanonicalFontInstanceKey);
        }
        Ok(Self { admitted: table })
    }
    pub fn get(&self, id: FontInstanceId) -> Option<&AdmittedFontInstance> {
        self.admitted.get(id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClusterExtractionPlan {
    PerCid {
        text_span: DisplayTextSpan,
        cids: Vec<Cid>,
    },
    ActualText {
        text_span: DisplayTextSpan,
        cids: Vec<Cid>,
        unicode: Vec<UnicodeScalar>,
    },
    Artifact {
        cids: Vec<Cid>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfFontMetrics {
    pub ascent_1000: i32,
    pub descent_1000: i32,
    pub cap_height_1000: i32,
    pub stem_v_1000: u32,
    pub italic_angle_milli_degrees: i32,
    pub flags: u32,
    pub bbox_1000: [i32; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfFontIndirectObjectRole {
    Type0Font,
    CidFont,
    FontDescriptor,
    EmbeddedFontProgram,
    ToUnicodeCMap,
    CidToGidMap,
}
pub const PDF_FONT_OBJECT_BLUEPRINT: [PdfFontIndirectObjectRole; 6] = [
    PdfFontIndirectObjectRole::Type0Font,
    PdfFontIndirectObjectRole::CidFont,
    PdfFontIndirectObjectRole::FontDescriptor,
    PdfFontIndirectObjectRole::EmbeddedFontProgram,
    PdfFontIndirectObjectRole::ToUnicodeCMap,
    PdfFontIndirectObjectRole::CidToGidMap,
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenPdfFontPlan {
    font_instance_id: FontInstanceId,
    admitted_sha256: [u8; 32],
    subset_bytes: Vec<u8>,
    embedded_postscript_name: String,
    subset_plan: FontSubsetPlan,
    metrics: PdfFontMetrics,
    cluster_plans: Vec<ClusterExtractionPlan>,
}
impl FrozenPdfFontPlan {
    pub const fn font_instance_id(&self) -> FontInstanceId {
        self.font_instance_id
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub fn subset_bytes(&self) -> &[u8] {
        &self.subset_bytes
    }
    /// The PostScript name extracted from the rewritten `name` table of the
    /// verified subset font program. Resource finalization proves that it is
    /// the same deterministic name used by every PDF font dictionary.
    pub fn embedded_postscript_name(&self) -> &str {
        &self.embedded_postscript_name
    }
    pub const fn subset_plan(&self) -> &FontSubsetPlan {
        &self.subset_plan
    }
    pub const fn metrics(&self) -> &PdfFontMetrics {
        &self.metrics
    }
    pub fn cluster_plans(&self) -> &[ClusterExtractionPlan] {
        &self.cluster_plans
    }
    pub const fn indirect_object_blueprint(&self) -> &[PdfFontIndirectObjectRole; 6] {
        &PDF_FONT_OBJECT_BLUEPRINT
    }
    pub const fn indirect_object_count(&self) -> u32 {
        PDF_FONT_OBJECT_BLUEPRINT.len() as u32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageColorSpace {
    Gray,
    Rgb,
    Cmyk,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageEncoding {
    Raw,
    Flate,
    Jpeg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfImageIndirectObjectRole {
    ImageXObject,
    SoftMaskImageXObject,
}
pub const PDF_IMAGE_OBJECT_BLUEPRINT: [PdfImageIndirectObjectRole; 1] =
    [PdfImageIndirectObjectRole::ImageXObject];
pub const PDF_IMAGE_WITH_ALPHA_OBJECT_BLUEPRINT: [PdfImageIndirectObjectRole; 2] = [
    PdfImageIndirectObjectRole::ImageXObject,
    PdfImageIndirectObjectRole::SoftMaskImageXObject,
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenPdfAlphaMask {
    encoded_bytes: Vec<u8>,
    width: NonZeroU32,
    height: NonZeroU32,
    bits_per_component: u8,
    encoding: ImageEncoding,
}
impl FrozenPdfAlphaMask {
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.encoded_bytes
    }
    pub const fn width(&self) -> NonZeroU32 {
        self.width
    }
    pub const fn height(&self) -> NonZeroU32 {
        self.height
    }
    pub const fn bits_per_component(&self) -> u8 {
        self.bits_per_component
    }
    pub const fn encoding(&self) -> ImageEncoding {
        self.encoding
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenPdfImagePlan {
    image_id: ImageResourceId,
    admitted_sha256: [u8; 32],
    encoded_bytes: Vec<u8>,
    width: NonZeroU32,
    height: NonZeroU32,
    color_space: ImageColorSpace,
    bits_per_component: u8,
    encoding: ImageEncoding,
    alpha_mask: Option<FrozenPdfAlphaMask>,
}
impl FrozenPdfImagePlan {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.encoded_bytes
    }
    pub const fn width(&self) -> NonZeroU32 {
        self.width
    }
    pub const fn height(&self) -> NonZeroU32 {
        self.height
    }
    pub const fn color_space(&self) -> ImageColorSpace {
        self.color_space
    }
    pub const fn bits_per_component(&self) -> u8 {
        self.bits_per_component
    }
    pub const fn encoding(&self) -> ImageEncoding {
        self.encoding
    }
    pub const fn alpha_mask(&self) -> Option<&FrozenPdfAlphaMask> {
        self.alpha_mask.as_ref()
    }
    pub fn indirect_object_blueprint(&self) -> &[PdfImageIndirectObjectRole] {
        if self.alpha_mask.is_some() {
            &PDF_IMAGE_WITH_ALPHA_OBJECT_BLUEPRINT
        } else {
            &PDF_IMAGE_OBJECT_BLUEPRINT
        }
    }
    pub const fn indirect_object_count(&self) -> u32 {
        if self.alpha_mask.is_some() {
            2
        } else {
            1
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum VerifiedEncoderOutput {
    Font(FrozenPdfFontPlan),
    Image(FrozenPdfImagePlan),
}

/// Sealed proof issued by the deterministic subsetter or image encoder. The
/// payload cannot be assembled from public fields and is rechecked against the
/// Display usage union before it can become a frozen plan set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedEncoderReceipt(VerifiedEncoderOutput);
/// Capability owned by deterministic in-crate subset/image encoders. Callers
/// can transport receipts but cannot create this capability.
#[derive(Debug)]
pub struct VerifiedEncoderReceiptOwner {
    _private: (),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontEncoderOutput {
    pub font_instance_id: FontInstanceId,
    pub admitted_sha256: [u8; 32],
    pub subset_bytes: Vec<u8>,
    pub subset_plan: FontSubsetPlan,
    pub metrics: PdfFontMetrics,
    pub cluster_plans: Vec<ClusterExtractionPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageEncoderOutput {
    pub image_id: ImageResourceId,
    pub admitted_sha256: [u8; 32],
    pub encoded_bytes: Vec<u8>,
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    pub color_space: ImageColorSpace,
    pub bits_per_component: u8,
    pub encoding: ImageEncoding,
    pub alpha_mask: Option<AlphaMaskEncoderOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlphaMaskEncoderOutput {
    pub encoded_bytes: Vec<u8>,
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    pub bits_per_component: u8,
    pub encoding: ImageEncoding,
}
impl VerifiedEncoderReceiptOwner {
    #[allow(dead_code)] // reserved for the in-crate PDF resource encoders
    fn new() -> Self {
        Self { _private: () }
    }
    pub fn issue_font(
        &self,
        output: FontEncoderOutput,
    ) -> Result<VerifiedEncoderReceipt, ResourceError> {
        let subset_bytes =
            rewrite_subset_postscript_name(&output.subset_bytes, output.font_instance_id)?;
        let embedded_postscript_name =
            extract_subset_postscript_name(&subset_bytes, output.font_instance_id)?;
        Ok(VerifiedEncoderReceipt(VerifiedEncoderOutput::Font(
            FrozenPdfFontPlan {
                font_instance_id: output.font_instance_id,
                admitted_sha256: output.admitted_sha256,
                subset_bytes,
                embedded_postscript_name,
                subset_plan: output.subset_plan,
                metrics: output.metrics,
                cluster_plans: output.cluster_plans,
            },
        )))
    }
    pub fn issue_image(&self, output: ImageEncoderOutput) -> VerifiedEncoderReceipt {
        VerifiedEncoderReceipt(VerifiedEncoderOutput::Image(FrozenPdfImagePlan {
            image_id: output.image_id,
            admitted_sha256: output.admitted_sha256,
            encoded_bytes: output.encoded_bytes,
            width: output.width,
            height: output.height,
            color_space: output.color_space,
            bits_per_component: output.bits_per_component,
            encoding: output.encoding,
            alpha_mask: output.alpha_mask.map(|mask| FrozenPdfAlphaMask {
                encoded_bytes: mask.encoded_bytes,
                width: mask.width,
                height: mask.height,
                bits_per_component: mask.bits_per_component,
                encoding: mask.encoding,
            }),
        }))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenPdfResourcePlans {
    fonts: Vec<FrozenPdfFontPlan>,
    images: Vec<FrozenPdfImagePlan>,
    usage: DisplayResourceUsage,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct DisplayFontUsage {
    glyphs: BTreeSet<OriginalGlyphId>,
    clusters: BTreeSet<(ClusterExtraction, Vec<OriginalGlyphId>)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DisplayResourceUsage {
    source_layout: typaxis_display_list::DisplaySourceLayout,
    fonts: BTreeMap<FontInstanceId, DisplayFontUsage>,
    images: BTreeSet<ImageResourceId>,
}
impl DisplayResourceUsage {
    fn from_display(display: &ValidatedDisplayDocument) -> Self {
        Self::from_document(display.document())
    }
    fn from_document(document: &DisplayDocument) -> Self {
        let mut fonts: BTreeMap<FontInstanceId, DisplayFontUsage> = BTreeMap::new();
        let mut images = BTreeSet::new();
        for page in &document.pages {
            for command in &page.commands {
                match command {
                    DisplayCommand::DrawGlyphRun {
                        font_instance_id,
                        glyphs,
                        clusters,
                        ..
                    } => {
                        let usage = fonts.entry(*font_instance_id).or_default();
                        for cluster in clusters {
                            let cluster_glyphs: Vec<_> = glyphs
                                [cluster.glyph_start as usize..cluster.glyph_end as usize]
                                .iter()
                                .map(|glyph| glyph.original_gid)
                                .collect();
                            usage.glyphs.extend(cluster_glyphs.iter().copied());
                            usage
                                .clusters
                                .insert((cluster.extraction.clone(), cluster_glyphs));
                        }
                    }
                    DisplayCommand::DrawImage { image_id, .. } => {
                        images.insert(*image_id);
                    }
                    _ => {}
                }
            }
        }
        Self {
            source_layout: document.source_layout(),
            fonts,
            images,
        }
    }
}

impl FrozenPdfResourcePlans {
    pub fn from_verified_receipts(
        display: &ValidatedDisplayDocument,
        admitted: &AdmittedResourceLedger,
        limits: &ValidatedResourceLimits,
        receipts: Vec<VerifiedEncoderReceipt>,
    ) -> Result<Self, ResourceError> {
        require_admitted_epoch_binding(
            display
                .document()
                .source_layout()
                .layout_epoch()
                .admitted_resources(),
            admitted.fingerprint(),
        )?;
        let mut fonts = Vec::new();
        let mut images = Vec::new();
        for receipt in receipts {
            match receipt.0 {
                VerifiedEncoderOutput::Font(plan) => fonts.push(plan),
                VerifiedEncoderOutput::Image(plan) => images.push(plan),
            }
        }
        let instances = FontInstanceTable::from_display(display, admitted)?;
        let usage_binding = DisplayResourceUsage::from_display(display);
        let used_fonts = &usage_binding.fonts;
        let limits = limits.get();
        let mut aggregate_plan_bytes = 0u64;
        let mut font_map: BTreeMap<FontInstanceId, FrozenPdfFontPlan> = BTreeMap::new();
        for plan in fonts {
            validate_pdf_font_metrics(&plan.metrics)?;
            validate_subset_postscript_name(&plan)?;
            plan.subset_plan
                .validate()
                .map_err(|_| ResourceError::InvalidFontPlan)?;
            let instance = instances
                .get(plan.font_instance_id)
                .ok_or(ResourceError::MissingLogicalResource)?;
            if instance.admitted_sha256() != plan.admitted_sha256 {
                return Err(ResourceError::FontInstanceHashMismatch);
            }
            let admitted_font = admitted
                .font(instance.font_face_id())
                .ok_or(ResourceError::MissingLogicalResource)?;
            validate_original_glyph_bounds(
                &plan.subset_plan,
                admitted_font.metadata().glyph_count,
            )?;
            let subset_bytes =
                u64::try_from(plan.subset_bytes.len()).map_err(|_| ResourceError::ResourceLimit)?;
            if subset_bytes == 0
                || plan.subset_plan.cids.len() > usize::from(limits.max_cids_per_font)
            {
                return Err(ResourceError::ResourceLimit);
            }
            aggregate_plan_bytes = aggregate_plan_bytes
                .checked_add(subset_bytes)
                .ok_or(ResourceError::ResourceLimit)?;
            let usage = used_fonts
                .get(&plan.font_instance_id)
                .ok_or(ResourceError::UnexpectedLogicalResource)?;
            let planned_glyphs: BTreeSet<_> = plan
                .subset_plan
                .glyphs
                .iter()
                .map(|binding| binding.original_gid)
                .collect();
            let mut required_glyphs = usage.glyphs.clone();
            required_glyphs.insert(OriginalGlyphId::new(0));
            if !required_glyphs.is_subset(&planned_glyphs) {
                return Err(ResourceError::IncompleteUsagePlan);
            }
            let cid_bindings: BTreeMap<_, _> = plan
                .subset_plan
                .cids
                .iter()
                .map(|binding| (binding.cid, binding))
                .collect();
            let original_to_subset: BTreeMap<_, _> = plan
                .subset_plan
                .glyphs
                .iter()
                .map(|binding| (binding.original_gid, binding.subset_gid))
                .collect();
            if plan.cluster_plans.len() != usage.clusters.len() {
                return Err(ResourceError::IncompleteUsagePlan);
            }
            let mut used_cids = BTreeSet::new();
            for ((extraction, glyphs), cluster_plan) in
                usage.clusters.iter().zip(&plan.cluster_plans)
            {
                let cids = match cluster_plan {
                    ClusterExtractionPlan::PerCid { cids, .. }
                    | ClusterExtractionPlan::ActualText { cids, .. }
                    | ClusterExtractionPlan::Artifact { cids } => cids,
                };
                if cids.len() != glyphs.len() || cids.is_empty() {
                    return Err(ResourceError::IncompleteUsagePlan);
                }
                for (glyph, cid) in glyphs.iter().zip(cids) {
                    let expected_subset = original_to_subset
                        .get(glyph)
                        .ok_or(ResourceError::IncompleteUsagePlan)?;
                    let binding = cid_bindings
                        .get(cid)
                        .ok_or(ResourceError::IncompleteUsagePlan)?;
                    if &binding.subset_gid != expected_subset {
                        return Err(ResourceError::IncompleteUsagePlan);
                    }
                    used_cids.insert(*cid);
                }
                match (extraction, cluster_plan) {
                    (
                        ClusterExtraction::Unicode { text_span },
                        ClusterExtractionPlan::PerCid {
                            text_span: planned_span,
                            cids,
                        },
                    ) if text_span == planned_span => {
                        let expected = display_scalars(display, *text_span)?;
                        let actual: Vec<_> = cids
                            .iter()
                            .flat_map(|cid| cid_bindings[cid].unicode.iter().copied())
                            .collect();
                        if actual != expected {
                            return Err(ResourceError::IncompleteUsagePlan);
                        }
                    }
                    (
                        ClusterExtraction::Unicode { text_span },
                        ClusterExtractionPlan::ActualText {
                            text_span: planned_span,
                            unicode,
                            ..
                        },
                    ) if text_span == planned_span => {
                        let expected = display_scalars(display, *text_span)?;
                        let per_cid: Vec<_> = cids
                            .iter()
                            .flat_map(|cid| cid_bindings[cid].unicode.iter().copied())
                            .collect();
                        if *unicode != expected || per_cid == expected {
                            return Err(ResourceError::IncompleteUsagePlan);
                        }
                    }
                    (ClusterExtraction::Artifact, ClusterExtractionPlan::Artifact { .. }) => {}
                    _ => return Err(ResourceError::IncompleteUsagePlan),
                }
            }
            if used_cids != cid_bindings.keys().copied().collect() {
                return Err(ResourceError::IncompleteUsagePlan);
            }
            if font_map.insert(plan.font_instance_id, plan).is_some() {
                return Err(ResourceError::DuplicatePlanKey);
            }
        }
        if used_fonts.keys().any(|id| !font_map.contains_key(id)) {
            return Err(ResourceError::MissingLogicalResource);
        }
        let mut image_map: BTreeMap<ImageResourceId, FrozenPdfImagePlan> = BTreeMap::new();
        for plan in images {
            let image = admitted
                .image(plan.image_id)
                .ok_or(ResourceError::MissingLogicalResource)?;
            if image.content_hash() != plan.admitted_sha256 {
                return Err(ResourceError::ConflictingLogicalResource);
            }
            let encoded_bytes = u64::try_from(plan.encoded_bytes.len())
                .map_err(|_| ResourceError::ResourceLimit)?;
            if encoded_bytes == 0
                || image.width() != plan.width
                || image.height() != plan.height
                || !matches!(plan.bits_per_component, 1 | 2 | 4 | 8 | 16)
            {
                return Err(ResourceError::InvalidImagePlan);
            }
            aggregate_plan_bytes = aggregate_plan_bytes
                .checked_add(encoded_bytes)
                .ok_or(ResourceError::ResourceLimit)?;
            if let Some(mask) = &plan.alpha_mask {
                let mask_bytes = u64::try_from(mask.encoded_bytes.len())
                    .map_err(|_| ResourceError::ResourceLimit)?;
                if mask_bytes == 0
                    || mask.width != plan.width
                    || mask.height != plan.height
                    || !matches!(mask.bits_per_component, 1 | 2 | 4 | 8 | 16)
                {
                    return Err(ResourceError::InvalidImagePlan);
                }
                aggregate_plan_bytes = aggregate_plan_bytes
                    .checked_add(mask_bytes)
                    .ok_or(ResourceError::ResourceLimit)?;
            }
            if image_map.insert(plan.image_id, plan).is_some() {
                return Err(ResourceError::DuplicatePlanKey);
            }
        }
        // Admission byte limits constrain source inputs, not deterministic
        // subset/image outputs. This is only the simultaneous frozen payload
        // budget; final PDF bytes are governed by the PDF write budget.
        if aggregate_plan_bytes > limits.max_spool_bytes {
            return Err(ResourceError::ResourceLimit);
        }
        if image_map.keys().copied().collect::<BTreeSet<_>>() != usage_binding.images {
            return Err(ResourceError::UnexpectedLogicalResource);
        }
        let mut fonts: Vec<_> = font_map.into_values().collect();
        fonts.sort_by_key(|plan| (plan.admitted_sha256, plan.font_instance_id));
        let mut images: Vec<_> = image_map.into_values().collect();
        images.sort_by_key(|plan| (plan.admitted_sha256, plan.image_id));
        Ok(Self {
            fonts,
            images,
            usage: usage_binding,
        })
    }
    pub fn fonts(&self) -> &[FrozenPdfFontPlan] {
        &self.fonts
    }
    pub fn images(&self) -> &[FrozenPdfImagePlan] {
        &self.images
    }
    pub fn matches_display(&self, display: &ValidatedDisplayDocument) -> bool {
        self.usage == DisplayResourceUsage::from_display(display)
    }
    pub fn into_plans(self) -> (Vec<FrozenPdfFontPlan>, Vec<FrozenPdfImagePlan>) {
        (self.fonts, self.images)
    }
}

fn validate_pdf_font_metrics(metrics: &PdfFontMetrics) -> Result<(), ResourceError> {
    let [left, bottom, right, top] = metrics.bbox_1000;
    let symbolic = metrics.flags & 0x04 != 0;
    let nonsymbolic = metrics.flags & 0x20 != 0;
    if left >= right || bottom >= top || metrics.stem_v_1000 == 0 || symbolic == nonsymbolic {
        return Err(ResourceError::InvalidFontPlan);
    }
    Ok(())
}

fn validate_original_glyph_bounds(
    subset_plan: &FontSubsetPlan,
    admitted_glyph_count: u32,
) -> Result<(), ResourceError> {
    if admitted_glyph_count == 0
        || subset_plan
            .glyphs
            .iter()
            .any(|binding| u32::from(binding.original_gid.get()) >= admitted_glyph_count)
    {
        return Err(ResourceError::InvalidFontPlan);
    }
    Ok(())
}

fn validate_subset_postscript_name(plan: &FrozenPdfFontPlan) -> Result<(), ResourceError> {
    if plan.embedded_postscript_name != expected_subset_postscript_name(plan.font_instance_id)? {
        return Err(ResourceError::InvalidFontPlan);
    }
    Ok(())
}

fn expected_subset_postscript_name(
    font_instance_id: FontInstanceId,
) -> Result<String, ResourceError> {
    const TAG_RADIX: u32 = 26;
    const TAG_LEN: usize = 6;
    const TAG_SPACE: u32 = TAG_RADIX.pow(TAG_LEN as u32);
    let mut value = font_instance_id.get();
    if value >= TAG_SPACE {
        return Err(ResourceError::InvalidFontPlan);
    }
    let mut tag = [b'A'; TAG_LEN];
    for byte in tag.iter_mut().rev() {
        *byte =
            b'A' + u8::try_from(value % TAG_RADIX).map_err(|_| ResourceError::InvalidFontPlan)?;
        value /= TAG_RADIX;
    }
    let mut name = String::with_capacity(TAG_LEN + "+Typaxis".len());
    name.extend(tag.into_iter().map(char::from));
    name.push_str("+Typaxis");
    Ok(name)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SfntRewriteTable {
    tag: [u8; 4],
    bytes: Vec<u8>,
}

/// Rebuilds the bounded standalone TrueType table directory and replaces the
/// complete `name` table with one canonical Windows Unicode BMP/English-US
/// PostScript-name record. The rebuilt bytes are subsequently reparsed by the
/// receipt owner; no caller-supplied name string crosses the trust boundary.
fn rewrite_subset_postscript_name(
    source: &[u8],
    font_instance_id: FontInstanceId,
) -> Result<Vec<u8>, ResourceError> {
    if source.get(..4) != Some(&0x0001_0000u32.to_be_bytes()) {
        return Err(ResourceError::InvalidFontPlan);
    }
    let table_count = usize::from(read_subset_u16(source, 4)?);
    let directory_len = 12usize
        .checked_add(
            table_count
                .checked_mul(16)
                .ok_or(ResourceError::InvalidFontPlan)?,
        )
        .ok_or(ResourceError::InvalidFontPlan)?;
    if directory_len > source.len() {
        return Err(ResourceError::InvalidFontPlan);
    }
    let mut tables = Vec::new();
    tables
        .try_reserve_exact(
            table_count
                .checked_add(1)
                .ok_or(ResourceError::InvalidFontPlan)?,
        )
        .map_err(|_| ResourceError::ResourceLimit)?;
    let mut tags = BTreeSet::new();
    for index in 0..table_count {
        let record = 12usize
            .checked_add(
                index
                    .checked_mul(16)
                    .ok_or(ResourceError::InvalidFontPlan)?,
            )
            .ok_or(ResourceError::InvalidFontPlan)?;
        let tag_end = record
            .checked_add(4)
            .ok_or(ResourceError::InvalidFontPlan)?;
        let tag: [u8; 4] = source
            .get(record..tag_end)
            .ok_or(ResourceError::InvalidFontPlan)?
            .try_into()
            .map_err(|_| ResourceError::InvalidFontPlan)?;
        if !tags.insert(tag) {
            return Err(ResourceError::InvalidFontPlan);
        }
        let offset = usize::try_from(read_subset_u32(source, record + 8)?)
            .map_err(|_| ResourceError::InvalidFontPlan)?;
        let length = usize::try_from(read_subset_u32(source, record + 12)?)
            .map_err(|_| ResourceError::InvalidFontPlan)?;
        let end = offset
            .checked_add(length)
            .ok_or(ResourceError::InvalidFontPlan)?;
        if offset < directory_len || end > source.len() {
            return Err(ResourceError::InvalidFontPlan);
        }
        if &tag == b"name" {
            continue;
        }
        let mut bytes = source[offset..end].to_vec();
        if &tag == b"head" {
            let adjustment = bytes.get_mut(8..12).ok_or(ResourceError::InvalidFontPlan)?;
            adjustment.fill(0);
        }
        tables.push(SfntRewriteTable { tag, bytes });
    }
    tables.push(SfntRewriteTable {
        tag: *b"name",
        bytes: canonical_subset_name_table(font_instance_id)?,
    });
    tables.sort_by_key(|table| table.tag);

    let table_count = u16::try_from(tables.len()).map_err(|_| ResourceError::InvalidFontPlan)?;
    let directory_len = 12usize
        .checked_add(
            tables
                .len()
                .checked_mul(16)
                .ok_or(ResourceError::InvalidFontPlan)?,
        )
        .ok_or(ResourceError::InvalidFontPlan)?;
    let payload_len = tables.iter().try_fold(0usize, |total, table| {
        let padded = table
            .bytes
            .len()
            .checked_add(3)
            .map(|length| length & !3)
            .ok_or(ResourceError::InvalidFontPlan)?;
        total
            .checked_add(padded)
            .ok_or(ResourceError::InvalidFontPlan)
    })?;
    let output_len = directory_len
        .checked_add(payload_len)
        .ok_or(ResourceError::InvalidFontPlan)?;
    let mut output = vec![0; output_len];
    output[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    output[4..6].copy_from_slice(&table_count.to_be_bytes());
    let entry_selector = if table_count == 0 {
        0
    } else {
        u16::try_from(u16::BITS - 1 - table_count.leading_zeros())
            .map_err(|_| ResourceError::InvalidFontPlan)?
    };
    let search_range = 16u16
        .checked_mul(
            1u16.checked_shl(u32::from(entry_selector))
                .ok_or(ResourceError::InvalidFontPlan)?,
        )
        .ok_or(ResourceError::InvalidFontPlan)?;
    let range_shift = table_count
        .checked_mul(16)
        .and_then(|total| total.checked_sub(search_range))
        .ok_or(ResourceError::InvalidFontPlan)?;
    output[6..8].copy_from_slice(&search_range.to_be_bytes());
    output[8..10].copy_from_slice(&entry_selector.to_be_bytes());
    output[10..12].copy_from_slice(&range_shift.to_be_bytes());

    let mut payload_offset = directory_len;
    let mut head_adjustment_offset = None;
    for (index, table) in tables.iter().enumerate() {
        let record = 12usize
            .checked_add(
                index
                    .checked_mul(16)
                    .ok_or(ResourceError::InvalidFontPlan)?,
            )
            .ok_or(ResourceError::InvalidFontPlan)?;
        output[record..record + 4].copy_from_slice(&table.tag);
        output[record + 4..record + 8].copy_from_slice(&sfnt_checksum(&table.bytes).to_be_bytes());
        output[record + 8..record + 12].copy_from_slice(
            &u32::try_from(payload_offset)
                .map_err(|_| ResourceError::InvalidFontPlan)?
                .to_be_bytes(),
        );
        output[record + 12..record + 16].copy_from_slice(
            &u32::try_from(table.bytes.len())
                .map_err(|_| ResourceError::InvalidFontPlan)?
                .to_be_bytes(),
        );
        let end = payload_offset
            .checked_add(table.bytes.len())
            .ok_or(ResourceError::InvalidFontPlan)?;
        output[payload_offset..end].copy_from_slice(&table.bytes);
        if &table.tag == b"head" {
            head_adjustment_offset = Some(
                payload_offset
                    .checked_add(8)
                    .ok_or(ResourceError::InvalidFontPlan)?,
            );
        }
        payload_offset = end
            .checked_add(3)
            .map(|offset| offset & !3)
            .ok_or(ResourceError::InvalidFontPlan)?;
    }
    if let Some(offset) = head_adjustment_offset {
        let adjustment = 0xB1B0_AFBAu32.wrapping_sub(sfnt_checksum(&output));
        output[offset..offset + 4].copy_from_slice(&adjustment.to_be_bytes());
    }
    Ok(output)
}

fn canonical_subset_name_table(font_instance_id: FontInstanceId) -> Result<Vec<u8>, ResourceError> {
    let name = expected_subset_postscript_name(font_instance_id)?;
    let encoded_len = name
        .len()
        .checked_mul(2)
        .ok_or(ResourceError::InvalidFontPlan)?;
    let table_len = 18usize
        .checked_add(encoded_len)
        .ok_or(ResourceError::InvalidFontPlan)?;
    let mut table = vec![0; table_len];
    table[2..4].copy_from_slice(&1u16.to_be_bytes());
    table[4..6].copy_from_slice(&18u16.to_be_bytes());
    table[6..8].copy_from_slice(&3u16.to_be_bytes());
    table[8..10].copy_from_slice(&1u16.to_be_bytes());
    table[10..12].copy_from_slice(&0x0409u16.to_be_bytes());
    table[12..14].copy_from_slice(&6u16.to_be_bytes());
    table[14..16].copy_from_slice(
        &u16::try_from(encoded_len)
            .map_err(|_| ResourceError::InvalidFontPlan)?
            .to_be_bytes(),
    );
    for (index, byte) in name.bytes().enumerate() {
        let offset = 18usize
            .checked_add(index.checked_mul(2).ok_or(ResourceError::InvalidFontPlan)?)
            .ok_or(ResourceError::InvalidFontPlan)?;
        table[offset + 1] = byte;
    }
    Ok(table)
}

fn sfnt_checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |checksum, chunk| {
        let mut word = [0; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        checksum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn extract_subset_postscript_name(
    bytes: &[u8],
    font_instance_id: FontInstanceId,
) -> Result<String, ResourceError> {
    if bytes.get(..4) != Some(&0x0001_0000u32.to_be_bytes()) {
        return Err(ResourceError::InvalidFontPlan);
    }
    let table_count = usize::from(read_subset_u16(bytes, 4)?);
    let directory_end = 12usize
        .checked_add(
            table_count
                .checked_mul(16)
                .ok_or(ResourceError::InvalidFontPlan)?,
        )
        .ok_or(ResourceError::InvalidFontPlan)?;
    if directory_end > bytes.len() {
        return Err(ResourceError::InvalidFontPlan);
    }
    let mut name_table = None;
    for index in 0..table_count {
        let record = 12usize
            .checked_add(
                index
                    .checked_mul(16)
                    .ok_or(ResourceError::InvalidFontPlan)?,
            )
            .ok_or(ResourceError::InvalidFontPlan)?;
        let tag_end = record
            .checked_add(4)
            .ok_or(ResourceError::InvalidFontPlan)?;
        if bytes.get(record..tag_end) != Some(b"name") {
            continue;
        }
        if name_table.is_some() {
            return Err(ResourceError::InvalidFontPlan);
        }
        let offset = usize::try_from(read_subset_u32(bytes, record + 8)?)
            .map_err(|_| ResourceError::InvalidFontPlan)?;
        let length = usize::try_from(read_subset_u32(bytes, record + 12)?)
            .map_err(|_| ResourceError::InvalidFontPlan)?;
        let end = offset
            .checked_add(length)
            .ok_or(ResourceError::InvalidFontPlan)?;
        if offset < directory_end || end > bytes.len() {
            return Err(ResourceError::InvalidFontPlan);
        }
        name_table = Some(&bytes[offset..end]);
    }
    let table = name_table.ok_or(ResourceError::InvalidFontPlan)?;
    if read_subset_u16(table, 0)? != 0 {
        return Err(ResourceError::InvalidFontPlan);
    }
    let record_count = usize::from(read_subset_u16(table, 2)?);
    let string_offset = usize::from(read_subset_u16(table, 4)?);
    let records_end = 6usize
        .checked_add(
            record_count
                .checked_mul(12)
                .ok_or(ResourceError::InvalidFontPlan)?,
        )
        .ok_or(ResourceError::InvalidFontPlan)?;
    if records_end > table.len() || string_offset < records_end || string_offset > table.len() {
        return Err(ResourceError::InvalidFontPlan);
    }

    let expected = expected_subset_postscript_name(font_instance_id)?;
    let expected_utf16_len = expected
        .len()
        .checked_mul(2)
        .ok_or(ResourceError::InvalidFontPlan)?;
    let mut found = false;
    for index in 0..record_count {
        let record = 6usize
            .checked_add(
                index
                    .checked_mul(12)
                    .ok_or(ResourceError::InvalidFontPlan)?,
            )
            .ok_or(ResourceError::InvalidFontPlan)?;
        let name_id = read_subset_u16(table, record + 6)?;
        if name_id != 6 {
            continue;
        }
        if found
            || read_subset_u16(table, record)? != 3
            || read_subset_u16(table, record + 2)? != 1
            || read_subset_u16(table, record + 4)? != 0x0409
            || usize::from(read_subset_u16(table, record + 8)?) != expected_utf16_len
        {
            return Err(ResourceError::InvalidFontPlan);
        }
        let local_offset = usize::from(read_subset_u16(table, record + 10)?);
        let start = string_offset
            .checked_add(local_offset)
            .ok_or(ResourceError::InvalidFontPlan)?;
        let end = start
            .checked_add(expected_utf16_len)
            .ok_or(ResourceError::InvalidFontPlan)?;
        let encoded = table
            .get(start..end)
            .ok_or(ResourceError::InvalidFontPlan)?;
        if encoded
            .chunks_exact(2)
            .zip(expected.bytes())
            .any(|(pair, byte)| pair != [0, byte])
        {
            return Err(ResourceError::InvalidFontPlan);
        }
        found = true;
    }
    if !found {
        return Err(ResourceError::InvalidFontPlan);
    }
    Ok(expected)
}

fn read_subset_u16(bytes: &[u8], offset: usize) -> Result<u16, ResourceError> {
    let end = offset
        .checked_add(2)
        .ok_or(ResourceError::InvalidFontPlan)?;
    let value: [u8; 2] = bytes
        .get(offset..end)
        .ok_or(ResourceError::InvalidFontPlan)?
        .try_into()
        .map_err(|_| ResourceError::InvalidFontPlan)?;
    Ok(u16::from_be_bytes(value))
}

fn read_subset_u32(bytes: &[u8], offset: usize) -> Result<u32, ResourceError> {
    let end = offset
        .checked_add(4)
        .ok_or(ResourceError::InvalidFontPlan)?;
    let value: [u8; 4] = bytes
        .get(offset..end)
        .ok_or(ResourceError::InvalidFontPlan)?
        .try_into()
        .map_err(|_| ResourceError::InvalidFontPlan)?;
    Ok(u32::from_be_bytes(value))
}

fn require_admitted_epoch_binding(
    selected: typaxis_core::AdmittedResourceFingerprint,
    supplied: typaxis_core::AdmittedResourceFingerprint,
) -> Result<(), ResourceError> {
    if selected == supplied {
        Ok(())
    } else {
        Err(ResourceError::AdmittedLedgerEpochMismatch)
    }
}

fn display_scalars(
    display: &ValidatedDisplayDocument,
    span: DisplayTextSpan,
) -> Result<Vec<UnicodeScalar>, ResourceError> {
    let buffer = display
        .document()
        .text_buffers
        .get(span.text_id().get() as usize)
        .ok_or(ResourceError::IncompleteUsagePlan)?;
    let start = span.range().start_byte().get() as usize;
    let end = span.range().end_byte().get() as usize;
    let utf8 = buffer
        .utf8
        .get(start..end)
        .ok_or(ResourceError::IncompleteUsagePlan)?;
    Ok(utf8.chars().map(UnicodeScalar::new).collect())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceFinalizationInput<'a> {
    pub display: &'a ValidatedDisplayDocument,
    pub admitted: &'a AdmittedResourceLedger,
    pub limits: &'a ValidatedResourceLimits,
}

pub trait ResourceFinalizer {
    fn finalize(
        &self,
        input: ResourceFinalizationInput<'_>,
    ) -> Result<FrozenPdfResourcePlans, ResourceError>;
}

/// Deterministic in-process finalizer for the linked PDF backend. TrueType
/// programs are reduced to the exact Display glyph union plus recursive
/// composite components; all glyph-indexed tables consumed by PDF rendering
/// are rebuilt with a dense canonical subset namespace.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReferenceResourceFinalizer;

impl ReferenceResourceFinalizer {
    pub const fn new() -> Self {
        Self
    }
}

impl ResourceFinalizer for ReferenceResourceFinalizer {
    fn finalize(
        &self,
        input: ResourceFinalizationInput<'_>,
    ) -> Result<FrozenPdfResourcePlans, ResourceError> {
        let usage = DisplayResourceUsage::from_display(input.display);
        let instances = FontInstanceTable::from_display(input.display, input.admitted)?;
        let owner = VerifiedEncoderReceiptOwner::new();
        let mut receipts = Vec::new();
        receipts
            .try_reserve_exact(usage.fonts.len() + usage.images.len())
            .map_err(|_| ResourceError::ResourceLimit)?;
        for (font_instance_id, font_usage) in &usage.fonts {
            let instance = instances
                .get(*font_instance_id)
                .ok_or(ResourceError::MissingLogicalResource)?;
            let admitted = input
                .admitted
                .font(instance.font_face_id())
                .ok_or(ResourceError::MissingLogicalResource)?;
            let subset =
                subset_truetype(admitted.bytes(), admitted.face_index(), &font_usage.glyphs)?;
            let (cids, cluster_plans) = build_cid_plans(
                input.display,
                font_usage,
                &subset.original_to_subset,
                &subset.original_widths,
                admitted.metadata().units_per_em,
                input.limits,
            )?;
            let glyphs = subset
                .original_to_subset
                .iter()
                .map(|(original_gid, subset_gid)| GlyphSubsetBinding {
                    original_gid: *original_gid,
                    subset_gid: *subset_gid,
                })
                .collect();
            receipts.push(owner.issue_font(FontEncoderOutput {
                font_instance_id: *font_instance_id,
                admitted_sha256: admitted.content_hash(),
                subset_bytes: subset.bytes,
                subset_plan: FontSubsetPlan { glyphs, cids },
                metrics: subset.metrics,
                cluster_plans,
            })?);
        }
        for image_id in &usage.images {
            let admitted = input
                .admitted
                .image(*image_id)
                .ok_or(ResourceError::MissingLogicalResource)?;
            receipts.push(owner.issue_image(decode_png_for_pdf(admitted)?));
        }
        FrozenPdfResourcePlans::from_verified_receipts(
            input.display,
            input.admitted,
            input.limits,
            receipts,
        )
    }
}

/// Freeze the exact admitted PNG union selected by the advanced-pagination
/// Display receipt. This is the resource bridge for the dedicated advanced
/// page serializer: callers provide logical IDs only, while byte decoding,
/// media attestation, dimensions, alpha separation, and the simultaneous
/// encoded-payload budget remain owned here.
pub fn freeze_admitted_png_images_for_pdf(
    admitted: &AdmittedResourceLedger,
    selected_image_ids: &[ImageResourceId],
    limits: &ValidatedResourceLimits,
) -> Result<Vec<FrozenPdfImagePlan>, ResourceError> {
    let selected: BTreeSet<_> = selected_image_ids.iter().copied().collect();
    let mut plans = Vec::new();
    plans
        .try_reserve_exact(selected.len())
        .map_err(|_| ResourceError::ResourceLimit)?;
    let mut aggregate_plan_bytes = 0u64;
    for image_id in selected {
        let image = admitted
            .image(image_id)
            .ok_or(ResourceError::MissingLogicalResource)?;
        let output = decode_png_for_pdf(image)?;
        let encoded_bytes =
            u64::try_from(output.encoded_bytes.len()).map_err(|_| ResourceError::ResourceLimit)?;
        aggregate_plan_bytes = aggregate_plan_bytes
            .checked_add(encoded_bytes)
            .ok_or(ResourceError::ResourceLimit)?;
        let alpha_mask = match output.alpha_mask {
            Some(mask) => {
                let mask_bytes = u64::try_from(mask.encoded_bytes.len())
                    .map_err(|_| ResourceError::ResourceLimit)?;
                aggregate_plan_bytes = aggregate_plan_bytes
                    .checked_add(mask_bytes)
                    .ok_or(ResourceError::ResourceLimit)?;
                Some(FrozenPdfAlphaMask {
                    encoded_bytes: mask.encoded_bytes,
                    width: mask.width,
                    height: mask.height,
                    bits_per_component: mask.bits_per_component,
                    encoding: mask.encoding,
                })
            }
            None => None,
        };
        if aggregate_plan_bytes > limits.get().max_spool_bytes {
            return Err(ResourceError::ResourceLimit);
        }
        plans.push(FrozenPdfImagePlan {
            image_id: output.image_id,
            admitted_sha256: output.admitted_sha256,
            encoded_bytes: output.encoded_bytes,
            width: output.width,
            height: output.height,
            color_space: output.color_space,
            bits_per_component: output.bits_per_component,
            encoding: output.encoding,
            alpha_mask,
        });
    }
    Ok(plans)
}

fn decode_png_for_pdf(admitted: &AdmittedImage) -> Result<ImageEncoderOutput, ResourceError> {
    if admitted.media_kind() != AdmittedImageMediaKind::Png {
        return Err(ResourceError::InvalidImagePlan);
    }
    decode_png_bytes_for_pdf(
        admitted.image_id(),
        admitted.content_hash(),
        admitted.bytes(),
        admitted.width(),
        admitted.height(),
        admitted.decoded_bytes(),
    )
}

fn decode_png_bytes_for_pdf(
    image_id: ImageResourceId,
    admitted_sha256: [u8; 32],
    source: &[u8],
    width: NonZeroU32,
    height: NonZeroU32,
    decoded_byte_budget: u64,
) -> Result<ImageEncoderOutput, ResourceError> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(source));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|_| ResourceError::InvalidImagePlan)?;
    let output_len = reader
        .output_buffer_size()
        .ok_or(ResourceError::ResourceLimit)?;
    let admitted_budget =
        usize::try_from(decoded_byte_budget).map_err(|_| ResourceError::ResourceLimit)?;
    if output_len == 0 || output_len > admitted_budget {
        return Err(ResourceError::ResourceLimit);
    }
    let mut decoded = vec![0; output_len];
    let frame = reader
        .next_frame(&mut decoded)
        .map_err(|_| ResourceError::InvalidImagePlan)?;
    if frame.width != width.get()
        || frame.height != height.get()
        || frame.bit_depth != png::BitDepth::Eight
    {
        return Err(ResourceError::InvalidImagePlan);
    }
    let used = frame.buffer_size();
    if used == 0 || used > decoded.len() {
        return Err(ResourceError::InvalidImagePlan);
    }
    decoded.truncate(used);

    let pixels = match frame.color_type {
        png::ColorType::Grayscale => DecodedPdfPixels {
            color_space: ImageColorSpace::Gray,
            color: decoded,
            alpha: None,
        },
        png::ColorType::Rgb => DecodedPdfPixels {
            color_space: ImageColorSpace::Rgb,
            color: decoded,
            alpha: None,
        },
        png::ColorType::GrayscaleAlpha => split_color_and_alpha(decoded, 1, width, height)?,
        png::ColorType::Rgba => split_color_and_alpha(decoded, 3, width, height)?,
        png::ColorType::Indexed => return Err(ResourceError::InvalidImagePlan),
    };
    let alpha_mask = pixels
        .alpha
        .filter(|alpha| alpha.iter().any(|sample| *sample != u8::MAX))
        .map(|encoded_bytes| AlphaMaskEncoderOutput {
            encoded_bytes,
            width,
            height,
            bits_per_component: 8,
            encoding: ImageEncoding::Raw,
        });
    Ok(ImageEncoderOutput {
        image_id,
        admitted_sha256,
        encoded_bytes: pixels.color,
        width,
        height,
        color_space: pixels.color_space,
        bits_per_component: 8,
        encoding: ImageEncoding::Raw,
        alpha_mask,
    })
}

struct DecodedPdfPixels {
    color_space: ImageColorSpace,
    color: Vec<u8>,
    alpha: Option<Vec<u8>>,
}

fn split_color_and_alpha(
    decoded: Vec<u8>,
    color_components: usize,
    width: NonZeroU32,
    height: NonZeroU32,
) -> Result<DecodedPdfPixels, ResourceError> {
    let pixel_count = usize::try_from(
        u64::from(width.get())
            .checked_mul(u64::from(height.get()))
            .ok_or(ResourceError::ResourceLimit)?,
    )
    .map_err(|_| ResourceError::ResourceLimit)?;
    let components = color_components
        .checked_add(1)
        .ok_or(ResourceError::ResourceLimit)?;
    if decoded.len()
        != pixel_count
            .checked_mul(components)
            .ok_or(ResourceError::ResourceLimit)?
    {
        return Err(ResourceError::InvalidImagePlan);
    }
    let color_len = pixel_count
        .checked_mul(color_components)
        .ok_or(ResourceError::ResourceLimit)?;
    let mut color = Vec::new();
    color
        .try_reserve_exact(color_len)
        .map_err(|_| ResourceError::ResourceLimit)?;
    let mut alpha = Vec::new();
    alpha
        .try_reserve_exact(pixel_count)
        .map_err(|_| ResourceError::ResourceLimit)?;
    for pixel in decoded.chunks_exact(components) {
        color.extend_from_slice(&pixel[..color_components]);
        alpha.push(pixel[color_components]);
    }
    Ok(DecodedPdfPixels {
        color_space: if color_components == 1 {
            ImageColorSpace::Gray
        } else {
            ImageColorSpace::Rgb
        },
        color,
        alpha: Some(alpha),
    })
}

#[derive(Clone, Debug)]
struct TrueTypeSubset {
    bytes: Vec<u8>,
    original_to_subset: BTreeMap<OriginalGlyphId, SubsetGlyphId>,
    original_widths: BTreeMap<OriginalGlyphId, u16>,
    metrics: PdfFontMetrics,
}

#[derive(Clone, Copy)]
struct SfntTableRef<'a> {
    bytes: &'a [u8],
}

fn subset_truetype(
    source: &[u8],
    face_index: u32,
    requested: &BTreeSet<OriginalGlyphId>,
) -> Result<TrueTypeSubset, ResourceError> {
    let tables = parse_sfnt_table_map(source, face_index)?;
    let head = table_bytes(&tables, *b"head")?;
    let hhea = table_bytes(&tables, *b"hhea")?;
    let maxp = table_bytes(&tables, *b"maxp")?;
    let hmtx = table_bytes(&tables, *b"hmtx")?;
    let loca = table_bytes(&tables, *b"loca")?;
    let glyf = table_bytes(&tables, *b"glyf")?;
    if head.len() < 54 || hhea.len() < 36 || maxp.len() < 6 {
        return Err(ResourceError::InvalidFontPlan);
    }
    let glyph_count = usize::from(read_subset_u16(maxp, 4)?);
    if glyph_count == 0 {
        return Err(ResourceError::InvalidFontPlan);
    }
    let loca_format = read_subset_i16(head, 50)?;
    let locations = parse_loca(loca, glyph_count, loca_format, glyf.len())?;
    let mut closure: BTreeSet<u16> = requested.iter().map(|glyph| glyph.get()).collect();
    closure.insert(0);
    if closure
        .iter()
        .any(|glyph| usize::from(*glyph) >= glyph_count)
    {
        return Err(ResourceError::InvalidFontPlan);
    }
    let mut pending: Vec<u16> = closure.iter().copied().collect();
    while let Some(glyph) = pending.pop() {
        for component in composite_components(glyph_bytes(glyf, &locations, glyph)?)? {
            if usize::from(component) >= glyph_count {
                return Err(ResourceError::InvalidFontPlan);
            }
            if closure.insert(component) {
                pending.push(component);
            }
        }
    }
    if closure.len() > usize::from(u16::MAX) {
        return Err(ResourceError::ResourceLimit);
    }
    let original_to_subset: BTreeMap<_, _> = closure
        .iter()
        .copied()
        .enumerate()
        .map(|(index, original)| {
            Ok((
                OriginalGlyphId::new(original),
                SubsetGlyphId::new(u16::try_from(index).map_err(|_| ResourceError::ResourceLimit)?),
            ))
        })
        .collect::<Result<_, ResourceError>>()?;
    let number_of_h_metrics = usize::from(read_subset_u16(hhea, 34)?);
    if number_of_h_metrics == 0 || number_of_h_metrics > glyph_count {
        return Err(ResourceError::InvalidFontPlan);
    }
    let mut new_glyf = Vec::new();
    let mut new_loca = Vec::new();
    let mut new_hmtx = Vec::new();
    let mut original_widths = BTreeMap::new();
    for original in &closure {
        new_loca.extend_from_slice(
            &u32::try_from(new_glyf.len())
                .map_err(|_| ResourceError::ResourceLimit)?
                .to_be_bytes(),
        );
        let mut glyph = glyph_bytes(glyf, &locations, *original)?.to_vec();
        remap_composite_components(&mut glyph, &original_to_subset)?;
        new_glyf.extend_from_slice(&glyph);
        while new_glyf.len() % 4 != 0 {
            new_glyf.push(0);
        }
        let (advance, side_bearing) = horizontal_metric(
            hmtx,
            glyph_count,
            number_of_h_metrics,
            usize::from(*original),
        )?;
        original_widths.insert(OriginalGlyphId::new(*original), advance);
        new_hmtx.extend_from_slice(&advance.to_be_bytes());
        new_hmtx.extend_from_slice(&side_bearing.to_be_bytes());
    }
    new_loca.extend_from_slice(
        &u32::try_from(new_glyf.len())
            .map_err(|_| ResourceError::ResourceLimit)?
            .to_be_bytes(),
    );
    let subset_count = u16::try_from(closure.len()).map_err(|_| ResourceError::ResourceLimit)?;
    let mut new_head = head.to_vec();
    new_head[8..12].fill(0);
    new_head[50..52].copy_from_slice(&1i16.to_be_bytes());
    let mut new_hhea = hhea.to_vec();
    new_hhea[34..36].copy_from_slice(&subset_count.to_be_bytes());
    let mut new_maxp = maxp.to_vec();
    new_maxp[4..6].copy_from_slice(&subset_count.to_be_bytes());
    let mut post = vec![0; 32];
    post[..4].copy_from_slice(&0x0003_0000u32.to_be_bytes());
    if let Some(source_post) = tables.get(b"post") {
        if source_post.bytes.len() >= 16 {
            post[4..16].copy_from_slice(&source_post.bytes[4..16]);
        }
    }
    let mut output_tables = vec![
        SfntRewriteTable {
            tag: *b"glyf",
            bytes: new_glyf,
        },
        SfntRewriteTable {
            tag: *b"head",
            bytes: new_head,
        },
        SfntRewriteTable {
            tag: *b"hhea",
            bytes: new_hhea,
        },
        SfntRewriteTable {
            tag: *b"hmtx",
            bytes: new_hmtx,
        },
        SfntRewriteTable {
            tag: *b"loca",
            bytes: new_loca,
        },
        SfntRewriteTable {
            tag: *b"maxp",
            bytes: new_maxp,
        },
        SfntRewriteTable {
            tag: *b"post",
            bytes: post,
        },
    ];
    if let Some(os2) = tables.get(b"OS/2") {
        output_tables.push(SfntRewriteTable {
            tag: *b"OS/2",
            bytes: os2.bytes.to_vec(),
        });
    }
    // `issue_font` replaces this placeholder with its canonical name table.
    output_tables.push(SfntRewriteTable {
        tag: *b"name",
        bytes: canonical_subset_name_table(FontInstanceId::new(0))?,
    });
    let metrics = pdf_metrics(
        head,
        hhea,
        tables.get(b"OS/2").map(|table| table.bytes),
        tables.get(b"post").map(|table| table.bytes),
    )?;
    Ok(TrueTypeSubset {
        bytes: rebuild_sfnt(output_tables)?,
        original_to_subset,
        original_widths,
        metrics,
    })
}

fn parse_sfnt_table_map(
    source: &[u8],
    face_index: u32,
) -> Result<BTreeMap<[u8; 4], SfntTableRef<'_>>, ResourceError> {
    let face_offset = if source.get(..4) == Some(b"ttcf") {
        let count = read_subset_u32(source, 8)?;
        if face_index >= count {
            return Err(ResourceError::InvalidFontPlan);
        }
        usize::try_from(read_subset_u32(
            source,
            12usize
                .checked_add(
                    usize::try_from(face_index)
                        .map_err(|_| ResourceError::InvalidFontPlan)?
                        .checked_mul(4)
                        .ok_or(ResourceError::InvalidFontPlan)?,
                )
                .ok_or(ResourceError::InvalidFontPlan)?,
        )?)
        .map_err(|_| ResourceError::InvalidFontPlan)?
    } else if face_index == 0 {
        0
    } else {
        return Err(ResourceError::InvalidFontPlan);
    };
    let signature_end = face_offset
        .checked_add(4)
        .ok_or(ResourceError::InvalidFontPlan)?;
    if source.get(face_offset..signature_end) != Some(&0x0001_0000u32.to_be_bytes()) {
        return Err(ResourceError::InvalidFontPlan);
    }
    let count_offset = face_offset
        .checked_add(4)
        .ok_or(ResourceError::InvalidFontPlan)?;
    let count = usize::from(read_subset_u16(source, count_offset)?);
    let directory_end = face_offset
        .checked_add(12)
        .and_then(|offset| offset.checked_add(count.checked_mul(16)?))
        .ok_or(ResourceError::InvalidFontPlan)?;
    if directory_end > source.len() {
        return Err(ResourceError::InvalidFontPlan);
    }
    let mut tables = BTreeMap::new();
    for index in 0..count {
        let record = face_offset
            .checked_add(12)
            .and_then(|value| value.checked_add(index.checked_mul(16)?))
            .ok_or(ResourceError::InvalidFontPlan)?;
        let tag_end = record
            .checked_add(4)
            .ok_or(ResourceError::InvalidFontPlan)?;
        let tag: [u8; 4] = source
            .get(record..tag_end)
            .ok_or(ResourceError::InvalidFontPlan)?
            .try_into()
            .map_err(|_| ResourceError::InvalidFontPlan)?;
        let offset_field = record
            .checked_add(8)
            .ok_or(ResourceError::InvalidFontPlan)?;
        let length_field = record
            .checked_add(12)
            .ok_or(ResourceError::InvalidFontPlan)?;
        let offset = usize::try_from(read_subset_u32(source, offset_field)?)
            .map_err(|_| ResourceError::InvalidFontPlan)?;
        let length = usize::try_from(read_subset_u32(source, length_field)?)
            .map_err(|_| ResourceError::InvalidFontPlan)?;
        let end = offset
            .checked_add(length)
            .ok_or(ResourceError::InvalidFontPlan)?;
        let bytes = source
            .get(offset..end)
            .ok_or(ResourceError::InvalidFontPlan)?;
        if tables.insert(tag, SfntTableRef { bytes }).is_some() {
            return Err(ResourceError::InvalidFontPlan);
        }
    }
    Ok(tables)
}

fn table_bytes<'a>(
    tables: &'a BTreeMap<[u8; 4], SfntTableRef<'a>>,
    tag: [u8; 4],
) -> Result<&'a [u8], ResourceError> {
    tables
        .get(&tag)
        .map(|table| table.bytes)
        .ok_or(ResourceError::InvalidFontPlan)
}

fn parse_loca(
    loca: &[u8],
    glyph_count: usize,
    format: i16,
    glyf_len: usize,
) -> Result<Vec<usize>, ResourceError> {
    let count = glyph_count
        .checked_add(1)
        .ok_or(ResourceError::InvalidFontPlan)?;
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(count)
        .map_err(|_| ResourceError::ResourceLimit)?;
    for index in 0..count {
        let offset = match format {
            0 => usize::from(read_subset_u16(
                loca,
                index.checked_mul(2).ok_or(ResourceError::InvalidFontPlan)?,
            )?)
            .checked_mul(2)
            .ok_or(ResourceError::InvalidFontPlan)?,
            1 => usize::try_from(read_subset_u32(
                loca,
                index.checked_mul(4).ok_or(ResourceError::InvalidFontPlan)?,
            )?)
            .map_err(|_| ResourceError::InvalidFontPlan)?,
            _ => return Err(ResourceError::InvalidFontPlan),
        };
        if offset > glyf_len || offsets.last().is_some_and(|previous| *previous > offset) {
            return Err(ResourceError::InvalidFontPlan);
        }
        offsets.push(offset);
    }
    Ok(offsets)
}

fn glyph_bytes<'a>(
    glyf: &'a [u8],
    locations: &[usize],
    glyph: u16,
) -> Result<&'a [u8], ResourceError> {
    let index = usize::from(glyph);
    let start = *locations.get(index).ok_or(ResourceError::InvalidFontPlan)?;
    let end = *locations
        .get(index + 1)
        .ok_or(ResourceError::InvalidFontPlan)?;
    glyf.get(start..end).ok_or(ResourceError::InvalidFontPlan)
}

fn composite_components(glyph: &[u8]) -> Result<Vec<u16>, ResourceError> {
    if glyph.is_empty() {
        return Ok(Vec::new());
    }
    if glyph.len() < 10 {
        return Err(ResourceError::InvalidFontPlan);
    }
    if read_subset_i16(glyph, 0)? >= 0 {
        return Ok(Vec::new());
    }
    let mut components = Vec::new();
    walk_composite_components(glyph, |_, component| {
        components.push(component);
        Ok(())
    })?;
    Ok(components)
}

fn remap_composite_components(
    glyph: &mut [u8],
    mapping: &BTreeMap<OriginalGlyphId, SubsetGlyphId>,
) -> Result<(), ResourceError> {
    if glyph.is_empty() {
        return Ok(());
    }
    if glyph.len() < 10 {
        return Err(ResourceError::InvalidFontPlan);
    }
    if read_subset_i16(glyph, 0)? >= 0 {
        return Ok(());
    }
    let mut replacements = Vec::new();
    walk_composite_components(glyph, |offset, component| {
        let subset = mapping
            .get(&OriginalGlyphId::new(component))
            .ok_or(ResourceError::InvalidFontPlan)?;
        replacements.push((offset, subset.get()));
        Ok(())
    })?;
    for (offset, subset) in replacements {
        let end = offset
            .checked_add(2)
            .ok_or(ResourceError::InvalidFontPlan)?;
        glyph
            .get_mut(offset..end)
            .ok_or(ResourceError::InvalidFontPlan)?
            .copy_from_slice(&subset.to_be_bytes());
    }
    Ok(())
}

fn walk_composite_components(
    glyph: &[u8],
    mut visit: impl FnMut(usize, u16) -> Result<(), ResourceError>,
) -> Result<(), ResourceError> {
    const ARG_WORDS: u16 = 0x0001;
    const MORE_COMPONENTS: u16 = 0x0020;
    const WE_HAVE_A_SCALE: u16 = 0x0008;
    const WE_HAVE_XY_SCALE: u16 = 0x0040;
    const WE_HAVE_2X2: u16 = 0x0080;
    const WE_HAVE_INSTRUCTIONS: u16 = 0x0100;
    let mut cursor = 10usize;
    let final_flags = loop {
        let flags = read_subset_u16(glyph, cursor)?;
        let glyph_offset = cursor
            .checked_add(2)
            .ok_or(ResourceError::InvalidFontPlan)?;
        let component = read_subset_u16(glyph, glyph_offset)?;
        visit(glyph_offset, component)?;
        cursor = cursor
            .checked_add(4)
            .and_then(|value| value.checked_add(if flags & ARG_WORDS != 0 { 4 } else { 2 }))
            .and_then(|value| {
                value.checked_add(if flags & WE_HAVE_A_SCALE != 0 {
                    2
                } else if flags & WE_HAVE_XY_SCALE != 0 {
                    4
                } else if flags & WE_HAVE_2X2 != 0 {
                    8
                } else {
                    0
                })
            })
            .ok_or(ResourceError::InvalidFontPlan)?;
        if cursor > glyph.len() {
            return Err(ResourceError::InvalidFontPlan);
        }
        if flags & MORE_COMPONENTS == 0 {
            break flags;
        }
    };
    if final_flags & WE_HAVE_INSTRUCTIONS != 0 {
        let instruction_len = usize::from(read_subset_u16(glyph, cursor)?);
        cursor = cursor
            .checked_add(2)
            .and_then(|value| value.checked_add(instruction_len))
            .ok_or(ResourceError::InvalidFontPlan)?;
        if cursor > glyph.len() {
            return Err(ResourceError::InvalidFontPlan);
        }
    }
    Ok(())
}

fn horizontal_metric(
    hmtx: &[u8],
    glyph_count: usize,
    number_of_h_metrics: usize,
    glyph: usize,
) -> Result<(u16, i16), ResourceError> {
    if glyph >= glyph_count || number_of_h_metrics == 0 || number_of_h_metrics > glyph_count {
        return Err(ResourceError::InvalidFontPlan);
    }
    let metric_index = glyph.min(
        number_of_h_metrics
            .checked_sub(1)
            .ok_or(ResourceError::InvalidFontPlan)?,
    );
    let advance_offset = metric_index
        .checked_mul(4)
        .ok_or(ResourceError::InvalidFontPlan)?;
    let advance = read_subset_u16(hmtx, advance_offset)?;
    let bearing_offset = if glyph < number_of_h_metrics {
        glyph
            .checked_mul(4)
            .and_then(|value| value.checked_add(2))
            .ok_or(ResourceError::InvalidFontPlan)?
    } else {
        number_of_h_metrics
            .checked_mul(4)
            .and_then(|value| {
                glyph
                    .checked_sub(number_of_h_metrics)?
                    .checked_mul(2)
                    .and_then(|tail| value.checked_add(tail))
            })
            .ok_or(ResourceError::InvalidFontPlan)?
    };
    Ok((advance, read_subset_i16(hmtx, bearing_offset)?))
}

fn pdf_metrics(
    head: &[u8],
    hhea: &[u8],
    os2: Option<&[u8]>,
    post: Option<&[u8]>,
) -> Result<PdfFontMetrics, ResourceError> {
    let units = i64::from(read_subset_u16(head, 18)?);
    if units <= 0 {
        return Err(ResourceError::InvalidFontPlan);
    }
    let scale = |value: i16| -> Result<i32, ResourceError> {
        i32::try_from(i64::from(value) * 1000 / units).map_err(|_| ResourceError::InvalidFontPlan)
    };
    let mut bbox = [
        scale(read_subset_i16(head, 36)?)?,
        scale(read_subset_i16(head, 38)?)?,
        scale(read_subset_i16(head, 40)?)?,
        scale(read_subset_i16(head, 42)?)?,
    ];
    if bbox[0] >= bbox[2] || bbox[1] >= bbox[3] {
        bbox = [0, -200, 1000, 800];
    }
    let ascent = scale(read_subset_i16(hhea, 4)?)?;
    let descent = scale(read_subset_i16(hhea, 6)?)?;
    let cap_height = os2
        .filter(|table| table.len() >= 90 && read_subset_u16(table, 0).is_ok_and(|v| v >= 2))
        .and_then(|table| read_subset_i16(table, 88).ok())
        .map(scale)
        .transpose()?
        .unwrap_or(ascent);
    let italic_angle_milli_degrees = post
        .filter(|table| table.len() >= 8)
        .and_then(|table| read_subset_i32(table, 4).ok())
        .and_then(|fixed| i32::try_from(i64::from(fixed) * 1000 / 65_536).ok())
        .unwrap_or(0);
    Ok(PdfFontMetrics {
        ascent_1000: ascent,
        descent_1000: descent,
        cap_height_1000: cap_height,
        stem_v_1000: 80,
        italic_angle_milli_degrees,
        flags: 0x20,
        bbox_1000: bbox,
    })
}

fn build_cid_plans(
    display: &ValidatedDisplayDocument,
    usage: &DisplayFontUsage,
    mapping: &BTreeMap<OriginalGlyphId, SubsetGlyphId>,
    widths: &BTreeMap<OriginalGlyphId, u16>,
    units_per_em: u16,
    limits: &ValidatedResourceLimits,
) -> Result<(Vec<CidBinding>, Vec<ClusterExtractionPlan>), ResourceError> {
    let mut bindings = Vec::new();
    let mut plans = Vec::new();
    for (extraction, glyphs) in &usage.clusters {
        let scalars = match extraction {
            ClusterExtraction::Unicode { text_span } => display_scalars(display, *text_span)?,
            ClusterExtraction::Artifact => Vec::new(),
        };
        let per_cid = matches!(extraction, ClusterExtraction::Unicode { .. })
            && scalars.len() == glyphs.len();
        let mut cids = Vec::new();
        for (index, glyph) in glyphs.iter().enumerate() {
            let next = bindings
                .len()
                .checked_add(1)
                .ok_or(ResourceError::ResourceLimit)?;
            if next > usize::from(limits.get().max_cids_per_font) {
                return Err(ResourceError::ResourceLimit);
            }
            let cid = Cid::new(u16::try_from(next).map_err(|_| ResourceError::ResourceLimit)?)
                .ok_or(ResourceError::ResourceLimit)?;
            let subset_gid = *mapping.get(glyph).ok_or(ResourceError::InvalidFontPlan)?;
            let advance = *widths.get(glyph).ok_or(ResourceError::InvalidFontPlan)?;
            let width_1000 = u32::try_from(
                (u64::from(advance) * 1000 + u64::from(units_per_em) / 2) / u64::from(units_per_em),
            )
            .map_err(|_| ResourceError::InvalidFontPlan)?;
            bindings.push(CidBinding {
                cid,
                subset_gid,
                unicode: if per_cid {
                    vec![scalars[index]]
                } else {
                    vec![]
                },
                width_1000,
            });
            cids.push(cid);
        }
        plans.push(match extraction {
            ClusterExtraction::Unicode { text_span } if per_cid => ClusterExtractionPlan::PerCid {
                text_span: *text_span,
                cids,
            },
            ClusterExtraction::Unicode { text_span } => ClusterExtractionPlan::ActualText {
                text_span: *text_span,
                cids,
                unicode: scalars,
            },
            ClusterExtraction::Artifact => ClusterExtractionPlan::Artifact { cids },
        });
    }
    Ok((bindings, plans))
}

fn rebuild_sfnt(mut tables: Vec<SfntRewriteTable>) -> Result<Vec<u8>, ResourceError> {
    tables.sort_by_key(|table| table.tag);
    if tables.windows(2).any(|pair| pair[0].tag == pair[1].tag) {
        return Err(ResourceError::InvalidFontPlan);
    }
    let count = u16::try_from(tables.len()).map_err(|_| ResourceError::ResourceLimit)?;
    let directory_len = 12usize
        .checked_add(
            tables
                .len()
                .checked_mul(16)
                .ok_or(ResourceError::ResourceLimit)?,
        )
        .ok_or(ResourceError::ResourceLimit)?;
    let payload_len = tables.iter().try_fold(0usize, |total, table| {
        let padded = table
            .bytes
            .len()
            .checked_add(3)
            .map(|length| length & !3)
            .ok_or(ResourceError::ResourceLimit)?;
        total
            .checked_add(padded)
            .ok_or(ResourceError::ResourceLimit)
    })?;
    let mut output = vec![
        0;
        directory_len
            .checked_add(payload_len)
            .ok_or(ResourceError::ResourceLimit)?
    ];
    output[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    output[4..6].copy_from_slice(&count.to_be_bytes());
    let selector = if count == 0 {
        0
    } else {
        u16::try_from(u16::BITS - 1 - count.leading_zeros())
            .map_err(|_| ResourceError::InvalidFontPlan)?
    };
    let search = 16u16
        .checked_mul(
            1u16.checked_shl(u32::from(selector))
                .ok_or(ResourceError::InvalidFontPlan)?,
        )
        .ok_or(ResourceError::InvalidFontPlan)?;
    output[6..8].copy_from_slice(&search.to_be_bytes());
    output[8..10].copy_from_slice(&selector.to_be_bytes());
    output[10..12].copy_from_slice(
        &count
            .checked_mul(16)
            .and_then(|value| value.checked_sub(search))
            .ok_or(ResourceError::InvalidFontPlan)?
            .to_be_bytes(),
    );
    let mut offset = directory_len;
    let mut head_adjustment = None;
    for (index, table) in tables.iter().enumerate() {
        let record = 12usize
            .checked_add(index.checked_mul(16).ok_or(ResourceError::ResourceLimit)?)
            .ok_or(ResourceError::ResourceLimit)?;
        let checksum = record.checked_add(4).ok_or(ResourceError::ResourceLimit)?;
        let table_offset = record.checked_add(8).ok_or(ResourceError::ResourceLimit)?;
        let table_length = record.checked_add(12).ok_or(ResourceError::ResourceLimit)?;
        let record_end = record.checked_add(16).ok_or(ResourceError::ResourceLimit)?;
        output
            .get_mut(record..checksum)
            .ok_or(ResourceError::InvalidFontPlan)?
            .copy_from_slice(&table.tag);
        output
            .get_mut(checksum..table_offset)
            .ok_or(ResourceError::InvalidFontPlan)?
            .copy_from_slice(&sfnt_checksum(&table.bytes).to_be_bytes());
        output
            .get_mut(table_offset..table_length)
            .ok_or(ResourceError::InvalidFontPlan)?
            .copy_from_slice(
                &u32::try_from(offset)
                    .map_err(|_| ResourceError::ResourceLimit)?
                    .to_be_bytes(),
            );
        output
            .get_mut(table_length..record_end)
            .ok_or(ResourceError::InvalidFontPlan)?
            .copy_from_slice(
                &u32::try_from(table.bytes.len())
                    .map_err(|_| ResourceError::ResourceLimit)?
                    .to_be_bytes(),
            );
        let end = offset
            .checked_add(table.bytes.len())
            .ok_or(ResourceError::ResourceLimit)?;
        output[offset..end].copy_from_slice(&table.bytes);
        if table.tag == *b"head" {
            head_adjustment = Some(offset.checked_add(8).ok_or(ResourceError::ResourceLimit)?);
        }
        offset = end
            .checked_add(3)
            .map(|value| value & !3)
            .ok_or(ResourceError::ResourceLimit)?;
    }
    if let Some(offset) = head_adjustment {
        let adjustment = 0xB1B0_AFBAu32.wrapping_sub(sfnt_checksum(&output));
        let end = offset.checked_add(4).ok_or(ResourceError::ResourceLimit)?;
        output
            .get_mut(offset..end)
            .ok_or(ResourceError::InvalidFontPlan)?
            .copy_from_slice(&adjustment.to_be_bytes());
    }
    Ok(output)
}

fn read_subset_i16(bytes: &[u8], offset: usize) -> Result<i16, ResourceError> {
    let end = offset
        .checked_add(2)
        .ok_or(ResourceError::InvalidFontPlan)?;
    Ok(i16::from_be_bytes(
        bytes
            .get(offset..end)
            .ok_or(ResourceError::InvalidFontPlan)?
            .try_into()
            .map_err(|_| ResourceError::InvalidFontPlan)?,
    ))
}

fn read_subset_i32(bytes: &[u8], offset: usize) -> Result<i32, ResourceError> {
    let end = offset
        .checked_add(4)
        .ok_or(ResourceError::InvalidFontPlan)?;
    Ok(i32::from_be_bytes(
        bytes
            .get(offset..end)
            .ok_or(ResourceError::InvalidFontPlan)?
            .try_into()
            .map_err(|_| ResourceError::InvalidFontPlan)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use read_fonts::TableProvider;
    use typaxis_core::{
        BidiLevel, DisplayGlyphRunId, DisplayTextBufferId, DisplayTextSpan, FontFaceId,
        FontInstanceId, Length, Point, PortablePath, PositiveLength, ResourceLimits, SourceId,
        TextBufferId, Utf8ByteOffset, ValidatedResourceLimits,
    };
    use typaxis_display_list::{
        DisplayCluster, DisplayCommand, DisplayFontInstance, DisplayGlyph, DisplayPage,
        DisplayTextBuffer, DisplayTextOrigin, Paint,
    };
    use typaxis_font::{GlyphSubsetBinding, SubsetGlyphId};
    use typaxis_layout::{
        FlowCursor, FlowTree, FragmentWorkBudget, LayoutEpoch, PageContext, ResolvedPageSelection,
    };
    use typaxis_pagination::{
        ConvergenceStatus, InitialPaginationState, LayoutPass, LayoutPassInput, PageFrameKind,
        PageFramePlan, PagePlan, PaginationInput, PaginationOptions, PaginationOutcome,
        PaginationResult,
    };
    use typaxis_syntax::{
        PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
    };
    use typaxis_text::GeneratedTextStore;

    #[test]
    fn png_finalizer_expands_palette_transparency_into_a_soft_mask() {
        const PNG: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1,
            1, 3, 0, 0, 0, 206, 236, 237, 201, 0, 0, 0, 6, 80, 76, 84, 69, 255, 0, 0, 0, 255, 0,
            210, 135, 239, 113, 0, 0, 0, 2, 116, 82, 78, 83, 255, 0, 229, 183, 48, 74, 0, 0, 0, 10,
            73, 68, 65, 84, 120, 156, 99, 112, 0, 0, 0, 66, 0, 65, 41, 55, 244, 239, 0, 0, 0, 0,
            73, 69, 78, 68, 174, 66, 96, 130,
        ];
        let output = decode_png_bytes_for_pdf(
            ImageResourceId::new(0),
            [9; 32],
            PNG,
            NonZeroU32::new(2).unwrap(),
            NonZeroU32::new(1).unwrap(),
            8,
        )
        .unwrap();
        assert_eq!(output.color_space, ImageColorSpace::Rgb);
        assert_eq!(output.encoded_bytes, [255, 0, 0, 0, 255, 0]);
        let mask = output.alpha_mask.as_ref().unwrap();
        assert_eq!(mask.encoded_bytes, [255, 0]);
        assert_eq!(mask.bits_per_component, 8);
        let receipt = VerifiedEncoderReceiptOwner::new().issue_image(output);
        let VerifiedEncoderOutput::Image(plan) = receipt.0 else {
            panic!("PNG encoder must issue an image plan")
        };
        assert_eq!(plan.indirect_object_count(), 2);
        assert_eq!(
            plan.indirect_object_blueprint(),
            &PDF_IMAGE_WITH_ALPHA_OBJECT_BLUEPRINT
        );
    }

    #[test]
    fn png_finalizer_decodes_adam7_rgba_and_enforces_admission_budget() {
        const PNG: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 1, 104, 18, 244, 31, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 224, 18,
            145, 211, 0, 0, 0, 205, 0, 101, 106, 153, 132, 66, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66,
            96, 130,
        ];
        let decode = |budget| {
            decode_png_bytes_for_pdf(
                ImageResourceId::new(0),
                [0; 32],
                PNG,
                NonZeroU32::new(1).unwrap(),
                NonZeroU32::new(1).unwrap(),
                budget,
            )
        };
        let output = decode(4).unwrap();
        assert_eq!(output.encoded_bytes, [10, 20, 30]);
        assert_eq!(output.alpha_mask.unwrap().encoded_bytes, [40]);
        assert_eq!(decode(3), Err(ResourceError::ResourceLimit));
    }

    #[test]
    fn truetype_subset_closes_and_remaps_composites_and_round_trips() {
        let mut head = vec![0; 54];
        head[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes());
        head[18..20].copy_from_slice(&1000u16.to_be_bytes());
        head[38..40].copy_from_slice(&(-200i16).to_be_bytes());
        head[40..42].copy_from_slice(&1000i16.to_be_bytes());
        head[42..44].copy_from_slice(&800i16.to_be_bytes());
        head[50..52].copy_from_slice(&1i16.to_be_bytes());
        let mut hhea = vec![0; 36];
        hhea[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        hhea[4..6].copy_from_slice(&800i16.to_be_bytes());
        hhea[6..8].copy_from_slice(&(-200i16).to_be_bytes());
        hhea[34..36].copy_from_slice(&4u16.to_be_bytes());
        let mut maxp = vec![0; 32];
        maxp[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        maxp[4..6].copy_from_slice(&4u16.to_be_bytes());
        let mut hmtx = Vec::new();
        for advance in [500u16, 510, 520, 530] {
            hmtx.extend_from_slice(&advance.to_be_bytes());
            hmtx.extend_from_slice(&0i16.to_be_bytes());
        }
        let mut glyf = vec![0; 10]; // Original glyph 2 is a minimal simple glyph.
        let mut composite = vec![0; 16];
        composite[..2].copy_from_slice(&(-1i16).to_be_bytes());
        composite[12..14].copy_from_slice(&2u16.to_be_bytes());
        glyf.extend_from_slice(&composite); // Original glyph 3 references glyph 2.
        let loca: Vec<_> = [0u32, 0, 0, 10, 26]
            .into_iter()
            .flat_map(u32::to_be_bytes)
            .collect();
        let source = rebuild_sfnt(vec![
            SfntRewriteTable {
                tag: *b"glyf",
                bytes: glyf,
            },
            SfntRewriteTable {
                tag: *b"head",
                bytes: head,
            },
            SfntRewriteTable {
                tag: *b"hhea",
                bytes: hhea,
            },
            SfntRewriteTable {
                tag: *b"hmtx",
                bytes: hmtx,
            },
            SfntRewriteTable {
                tag: *b"loca",
                bytes: loca,
            },
            SfntRewriteTable {
                tag: *b"maxp",
                bytes: maxp,
            },
        ])
        .unwrap();
        let subset =
            subset_truetype(&source, 0, &[OriginalGlyphId::new(3)].into_iter().collect()).unwrap();
        assert_eq!(
            subset
                .original_to_subset
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            [
                OriginalGlyphId::new(0),
                OriginalGlyphId::new(2),
                OriginalGlyphId::new(3)
            ]
        );
        let tables = parse_sfnt_table_map(&subset.bytes, 0).unwrap();
        let locations = parse_loca(
            table_bytes(&tables, *b"loca").unwrap(),
            3,
            1,
            table_bytes(&tables, *b"glyf").unwrap().len(),
        )
        .unwrap();
        let remapped = glyph_bytes(table_bytes(&tables, *b"glyf").unwrap(), &locations, 2).unwrap();
        assert_eq!(composite_components(remapped).unwrap(), [1]);

        let independent = read_fonts::FontRef::new(&subset.bytes).unwrap();
        assert_eq!(independent.maxp().unwrap().num_glyphs(), 3);
        assert_eq!(independent.hhea().unwrap().number_of_h_metrics(), 3);
        independent.head().unwrap();
        independent.hmtx().unwrap();
        independent.loca(None).unwrap();
        independent.glyf().unwrap();
    }

    fn subset_sfnt_with_postscript_name(name: &str) -> Vec<u8> {
        let encoded_name: Vec<_> = name.bytes().flat_map(|byte| [0, byte]).collect();
        let name_table_length = 18 + encoded_name.len();
        let mut bytes = vec![0; 28 + name_table_length];
        bytes[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        bytes[4..6].copy_from_slice(&1u16.to_be_bytes());
        bytes[12..16].copy_from_slice(b"name");
        bytes[20..24].copy_from_slice(&28u32.to_be_bytes());
        bytes[24..28].copy_from_slice(&(name_table_length as u32).to_be_bytes());
        let table = &mut bytes[28..];
        table[2..4].copy_from_slice(&1u16.to_be_bytes());
        table[4..6].copy_from_slice(&18u16.to_be_bytes());
        table[6..8].copy_from_slice(&3u16.to_be_bytes());
        table[8..10].copy_from_slice(&1u16.to_be_bytes());
        table[10..12].copy_from_slice(&0x0409u16.to_be_bytes());
        table[12..14].copy_from_slice(&6u16.to_be_bytes());
        table[14..16].copy_from_slice(&(encoded_name.len() as u16).to_be_bytes());
        table[18..].copy_from_slice(&encoded_name);
        bytes
    }

    fn font_encoder_output(font_instance_id: u32, program_name: &str) -> FontEncoderOutput {
        FontEncoderOutput {
            font_instance_id: FontInstanceId::new(font_instance_id),
            admitted_sha256: [7; 32],
            subset_bytes: subset_sfnt_with_postscript_name(program_name),
            subset_plan: FontSubsetPlan {
                glyphs: vec![],
                cids: vec![],
            },
            metrics: PdfFontMetrics {
                ascent_1000: 800,
                descent_1000: -200,
                cap_height_1000: 700,
                stem_v_1000: 80,
                italic_angle_milli_degrees: 0,
                flags: 4,
                bbox_1000: [0, -200, 1000, 800],
            },
            cluster_plans: vec![],
        }
    }

    fn pagination_result() -> PaginationResult {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("input.tsf").unwrap(),
            text: String::new(),
        };
        let package = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        );
        let ParseOutcome::Parsed { package, .. } = package else {
            panic!("reference package must parse");
        };
        let package = *package;
        let store = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let generated = package.bind_generated_text(&store, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap();
        let flow = FlowTree::empty(&package, epoch).unwrap();
        let initial = InitialPaginationState::new(&flow, &package, &limits).unwrap();
        let pagination_context = package.pagination_context();
        let mut input = PaginationInput::new(
            initial,
            &pagination_context,
            PaginationOptions::from_limits(&limits, false),
        )
        .unwrap();
        let master = &package.package().page_masters.masters[0];
        let pages = vec![PagePlan {
            page_index: 0,
            master_id: master.master_id.clone(),
            frames: vec![PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 0,
                bounds: master.body,
            }],
            fragments: vec![],
            footnote_ids: vec![],
            float_decisions: vec![],
            column_decisions: vec![],
            resolved_references: vec![],
        }];
        let mut budget = input.take_work_budget().unwrap();
        let mut first_permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let page_selection = ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let page_context = PageContext::select(0, &page_selection, &pagination_context).unwrap();
        first_permit
            .begin_page(&page_context, &cursor, &pages[0].frames)
            .unwrap();
        FragmentWorkBudget::consume_fragments(&mut first_permit, 0).unwrap();
        first_permit.finish_page(&pages[0]).unwrap();
        let first_receipt = first_permit.finish(&flow, &pages).unwrap();
        let first = LayoutPass::new(
            first_receipt,
            input.initial_fingerprint(),
            &flow,
            pages.clone(),
            store.clone(),
        )
        .unwrap();
        let second_input =
            LayoutPassInput::transitioned(first.transition_references(&package, &limits).unwrap());
        let mut second_permit = budget.begin_pass(1, second_input).unwrap();
        second_permit
            .begin_page(&page_context, &cursor, &pages[0].frames)
            .unwrap();
        FragmentWorkBudget::consume_fragments(&mut second_permit, 0).unwrap();
        second_permit.finish_page(&pages[0]).unwrap();
        let second_receipt = second_permit.finish(&flow, &pages).unwrap();
        let second = LayoutPass::new(
            second_receipt,
            first.output_fingerprint(),
            &flow,
            pages,
            store,
        )
        .unwrap();
        PaginationOutcome::new(
            vec![first, second],
            ConvergenceStatus::Converged,
            &input,
            budget.finish(),
        )
        .unwrap()
        .into_result()
    }

    #[test]
    fn repeated_display_cluster_use_is_a_canonical_union() {
        let selected = pagination_result();
        let size = PositiveLength::new(Length::from_raw(1).unwrap()).unwrap();
        let span = DisplayTextSpan::new(
            DisplayTextBufferId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let glyph = DisplayGlyph {
            original_gid: OriginalGlyphId::new(1),
            advance_x: Length::ZERO,
            advance_y: Length::ZERO,
            offset_x: Length::ZERO,
            offset_y: Length::ZERO,
        };
        let command = |run_id| DisplayCommand::DrawGlyphRun {
            run_id: DisplayGlyphRunId::new(run_id),
            font_instance_id: FontInstanceId::new(0),
            text_span: span,
            origin: Point {
                x: Length::ZERO,
                y: Length::ZERO,
            },
            font_size: size,
            bidi_level: BidiLevel::new(0).unwrap(),
            fill: Paint::Gray(0),
            glyphs: vec![glyph.clone()],
            clusters: vec![DisplayCluster {
                logical_ordinal: 0,
                glyph_start: 0,
                glyph_end: 1,
                extraction: ClusterExtraction::Artifact,
            }],
        };
        let display = DisplayDocument::from_untrusted_parts_for_selected_pagination(
            &selected,
            vec![DisplayTextBuffer {
                text_id: DisplayTextBufferId::new(0),
                origin: DisplayTextOrigin::Parsed(TextBufferId::new(0)),
                utf8: String::new(),
            }],
            vec![DisplayFontInstance {
                font_instance_id: FontInstanceId::new(0),
                font_face_id: FontFaceId::new(0),
            }],
            vec![],
            vec![DisplayPage {
                page_index: 0,
                width: size,
                height: size,
                commands: vec![command(0), command(1)],
                annotations: vec![],
            }],
        );
        let usage = DisplayResourceUsage::from_document(&display);
        let font_usage = usage.fonts.get(&FontInstanceId::new(0)).unwrap();
        assert_eq!(font_usage.glyphs.len(), 1);
        assert_eq!(font_usage.clusters.len(), 1);
    }

    #[test]
    fn frozen_font_plan_declares_every_indirect_object_role() {
        let plan = FrozenPdfFontPlan {
            font_instance_id: FontInstanceId::new(0),
            admitted_sha256: [7; 32],
            subset_bytes: vec![1],
            embedded_postscript_name: "AAAAAA+Typaxis".to_owned(),
            subset_plan: FontSubsetPlan {
                glyphs: vec![GlyphSubsetBinding {
                    original_gid: OriginalGlyphId::new(0),
                    subset_gid: SubsetGlyphId::new(0),
                }],
                cids: vec![],
            },
            metrics: PdfFontMetrics {
                ascent_1000: 800,
                descent_1000: -200,
                cap_height_1000: 700,
                stem_v_1000: 80,
                italic_angle_milli_degrees: 0,
                flags: 4,
                bbox_1000: [0, -200, 1000, 800],
            },
            cluster_plans: vec![],
        };
        assert_eq!(plan.indirect_object_count(), 6);
        assert_eq!(
            plan.indirect_object_blueprint(),
            &[
                PdfFontIndirectObjectRole::Type0Font,
                PdfFontIndirectObjectRole::CidFont,
                PdfFontIndirectObjectRole::FontDescriptor,
                PdfFontIndirectObjectRole::EmbeddedFontProgram,
                PdfFontIndirectObjectRole::ToUnicodeCMap,
                PdfFontIndirectObjectRole::CidToGidMap,
            ]
        );
        assert_eq!(validate_subset_postscript_name(&plan), Ok(()));
    }

    #[test]
    fn subset_postscript_name_must_match_the_rewritten_program_name() {
        let valid = FrozenPdfFontPlan {
            font_instance_id: FontInstanceId::new(1),
            admitted_sha256: [7; 32],
            subset_bytes: vec![1],
            embedded_postscript_name: "AAAAAB+Typaxis".to_owned(),
            subset_plan: FontSubsetPlan {
                glyphs: vec![],
                cids: vec![],
            },
            metrics: PdfFontMetrics {
                ascent_1000: 800,
                descent_1000: -200,
                cap_height_1000: 700,
                stem_v_1000: 80,
                italic_angle_milli_degrees: 0,
                flags: 4,
                bbox_1000: [0, -200, 1000, 800],
            },
            cluster_plans: vec![],
        };
        assert_eq!(validate_subset_postscript_name(&valid), Ok(()));
        let mut mismatched = valid;
        mismatched.embedded_postscript_name = "AAAAAA+Typaxis".to_owned();
        assert_eq!(
            validate_subset_postscript_name(&mismatched),
            Err(ResourceError::InvalidFontPlan)
        );
    }

    #[test]
    fn encoder_owner_rewrites_and_reextracts_the_postscript_name() {
        let owner = VerifiedEncoderReceiptOwner::new();
        let receipt = owner
            .issue_font(font_encoder_output(1, "legacy-name"))
            .unwrap();
        let VerifiedEncoderOutput::Font(plan) = receipt.0 else {
            panic!("font encoder emitted an image receipt")
        };
        assert_eq!(plan.embedded_postscript_name(), "AAAAAB+Typaxis");
        assert_eq!(
            extract_subset_postscript_name(plan.subset_bytes(), FontInstanceId::new(1)).unwrap(),
            "AAAAAB+Typaxis"
        );
        assert_ne!(
            plan.subset_bytes(),
            subset_sfnt_with_postscript_name("legacy-name")
        );
        assert_eq!(
            owner.issue_font(FontEncoderOutput {
                subset_bytes: vec![0; 12],
                ..font_encoder_output(1, "legacy-name")
            }),
            Err(ResourceError::InvalidFontPlan)
        );
    }

    #[test]
    fn pdf_font_metrics_reject_invalid_descriptor_closure() {
        let valid = PdfFontMetrics {
            ascent_1000: 800,
            descent_1000: -200,
            cap_height_1000: 700,
            stem_v_1000: 80,
            italic_angle_milli_degrees: 0,
            flags: 4,
            bbox_1000: [0, -200, 1000, 800],
        };
        assert_eq!(validate_pdf_font_metrics(&valid), Ok(()));

        let mut invalid_bbox = valid.clone();
        invalid_bbox.bbox_1000[2] = invalid_bbox.bbox_1000[0];
        assert_eq!(
            validate_pdf_font_metrics(&invalid_bbox),
            Err(ResourceError::InvalidFontPlan)
        );
        let mut invalid_stem = valid.clone();
        invalid_stem.stem_v_1000 = 0;
        assert_eq!(
            validate_pdf_font_metrics(&invalid_stem),
            Err(ResourceError::InvalidFontPlan)
        );
        let mut invalid_flags = valid;
        invalid_flags.flags = 0x04 | 0x20;
        assert_eq!(
            validate_pdf_font_metrics(&invalid_flags),
            Err(ResourceError::InvalidFontPlan)
        );
    }

    #[test]
    fn subset_original_glyphs_are_bounded_by_admitted_font_metadata() {
        let plan = FontSubsetPlan {
            glyphs: vec![
                GlyphSubsetBinding {
                    original_gid: OriginalGlyphId::new(0),
                    subset_gid: SubsetGlyphId::new(0),
                },
                GlyphSubsetBinding {
                    original_gid: OriginalGlyphId::new(1),
                    subset_gid: SubsetGlyphId::new(1),
                },
            ],
            cids: vec![],
        };
        assert_eq!(validate_original_glyph_bounds(&plan, 2), Ok(()));
        assert_eq!(
            validate_original_glyph_bounds(&plan, 1),
            Err(ResourceError::InvalidFontPlan)
        );
    }

    #[test]
    fn resource_finalization_rejects_a_ledger_from_another_layout_epoch() {
        use typaxis_core::AdmittedResourceFingerprint;

        let selected = AdmittedResourceFingerprint::from_untrusted_bytes([1; 32]);
        let supplied = AdmittedResourceFingerprint::from_untrusted_bytes([2; 32]);
        assert_eq!(
            require_admitted_epoch_binding(selected, supplied),
            Err(ResourceError::AdmittedLedgerEpochMismatch)
        );
        assert_eq!(require_admitted_epoch_binding(selected, selected), Ok(()));
    }
}
