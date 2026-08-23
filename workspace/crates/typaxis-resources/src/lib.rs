#![forbid(unsafe_code)]

use core::num::NonZeroU32;
use std::collections::{BTreeMap, BTreeSet};
use typaxis_core::{DisplayTextSpan, FontInstanceId, ImageResourceId, ValidatedResourceLimits};
use typaxis_display_list::{
    ClusterExtraction, DisplayCommand, DisplayDocument, ValidatedDisplayDocument,
};
use typaxis_font::{Cid, FontSubsetPlan, OriginalGlyphId, UnicodeScalar};

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
}
pub const PDF_IMAGE_OBJECT_BLUEPRINT: [PdfImageIndirectObjectRole; 1] =
    [PdfImageIndirectObjectRole::ImageXObject];

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
    alpha_mask: Option<ImageResourceId>,
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
    pub const fn alpha_mask(&self) -> Option<ImageResourceId> {
        self.alpha_mask
    }
    pub const fn indirect_object_blueprint(&self) -> &[PdfImageIndirectObjectRole; 1] {
        &PDF_IMAGE_OBJECT_BLUEPRINT
    }
    pub const fn indirect_object_count(&self) -> u32 {
        PDF_IMAGE_OBJECT_BLUEPRINT.len() as u32
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
    pub alpha_mask: Option<ImageResourceId>,
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
        let embedded_postscript_name =
            extract_subset_postscript_name(&output.subset_bytes, output.font_instance_id)?;
        Ok(VerifiedEncoderReceipt(VerifiedEncoderOutput::Font(
            FrozenPdfFontPlan {
                font_instance_id: output.font_instance_id,
                admitted_sha256: output.admitted_sha256,
                subset_bytes: output.subset_bytes,
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
            alpha_mask: output.alpha_mask,
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
            if planned_glyphs != required_glyphs {
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
                || plan.alpha_mask == Some(plan.image_id)
            {
                return Err(ResourceError::InvalidImagePlan);
            }
            aggregate_plan_bytes = aggregate_plan_bytes
                .checked_add(encoded_bytes)
                .ok_or(ResourceError::ResourceLimit)?;
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
        let mut required_images = usage_binding.images.clone();
        let mut pending: Vec<_> = required_images.iter().copied().collect();
        while let Some(image_id) = pending.pop() {
            let plan = image_map
                .get(&image_id)
                .ok_or(ResourceError::MissingLogicalResource)?;
            if let Some(mask_id) = plan.alpha_mask {
                let mask = image_map
                    .get(&mask_id)
                    .ok_or(ResourceError::MissingLogicalResource)?;
                if mask.alpha_mask.is_some()
                    || mask.color_space != ImageColorSpace::Gray
                    || mask.width != plan.width
                    || mask.height != plan.height
                {
                    return Err(ResourceError::InvalidImagePlan);
                }
                if required_images.insert(mask_id) {
                    pending.push(mask_id);
                }
            }
        }
        if image_map.keys().copied().collect::<BTreeSet<_>>() != required_images {
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

#[cfg(test)]
mod tests {
    use super::*;
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
    fn encoder_receipt_reextracts_the_postscript_name_from_subset_bytes() {
        let owner = VerifiedEncoderReceiptOwner::new();
        assert!(owner
            .issue_font(font_encoder_output(1, "AAAAAB+Typaxis"))
            .is_ok());
        assert_eq!(
            owner.issue_font(font_encoder_output(1, "AAAAAA+Typaxis")),
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
