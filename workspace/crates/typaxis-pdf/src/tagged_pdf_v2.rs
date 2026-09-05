use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{
    push_jcs_string, sha256, DisplayTextBufferId, DisplayTextSpan, EffectiveConfigFingerprint,
    EngineIdentity, FontFaceId, ImageResourceId, LayoutStateFingerprint, M4EffectiveResourceLimits,
    NodeId, PdfStreamCompression, Utf8ByteOffset,
};
#[cfg(any(test, feature = "staging-fixtures"))]
use typaxis_core::{Length, Point};
use typaxis_display_list::{
    BookNavigationSelectedReceiptV2, DestinationView, FormulaStructureKidV2,
    MarkedContentBindingKindV2, MarkedContentOwner, MarkedContentPlanReceiptV2,
    StagingCombinedVectorDisplayV2, StagingCombinedVectorKindV2, StagingMathDisplay,
    StagingPrecomposedVectorDisplay, StructureArtifactClass, StructureNodeId, StructureOwner,
    StructureParentTreeValue, StructureRegistryReceiptV2, StructureRole,
    VectorFormStructureIsolationReceiptV2, VectorMarkedContentPlanV2,
    VectorMarkedContentSerializationV2,
};
use typaxis_font::MathFontFace;
use typaxis_math::MathPaint;
use typaxis_resource_admission::{AdmittedImageMediaKind, AdmittedResourceLedger};
use typaxis_resources::{
    finalize_staging_pdf_text_fonts, FrozenPdfFontPlan, FrozenPdfImagePlan,
    FrozenStagingPdfTextFontPlan, ImageColorSpace, ImageEncoding, PdfFontProgramKind,
    StagingPdfTextClusterUsage, StagingSafeVectorFormPlansV2, VectorContentCandidateRegistry,
};
use typaxis_shaping::{
    ShapeSourceSpan, ShapedCluster, StagingEquationNumberGlyphRun,
    StagingEquationNumberShapeReceipt,
};
use typaxis_syntax::{
    machine_profile_boundary::StagingM4Block, StagingAccessibilityProfileAuthorizationV2,
    StagingBookNavigationProfileAuthorizationV2, StagingMathProfileAuthorization,
    ValidatedStagingBookNavigationV2, ValidatedStagingSemanticPackage,
    ValidatedStagingStructureSemanticsV2,
};

use crate::{
    observe_staging_book_navigation_pdf_v2, seal_staging_safe_vector_pdf_v2,
    BookNavigationPdfFinalWriterObservationV2, BookNavigationPdfInfoObservationV2,
    BookNavigationPdfLanguagePaintObservationV2, BookNavigationPdfLanguagePaintSourceV2,
    BookNavigationPdfObservationV2, BookNavigationPdfOutlineObservationV2, BookXmpObservationV2,
    StagingSafeVectorPdfClosureV2, StagingSafeVectorPdfContributionV2,
    StagingSafeVectorPdfFinalObjectObservationV2, StagingSafeVectorPdfFinalUsageObservationV2,
    StagingSafeVectorPdfFinalWriterObservationV2, StagingSafeVectorPdfRelativeObjectKindV2,
    VerifiedPdfBytesReceipt,
};

pub const TAGGED_PDF_OBSERVATION_ALGORITHM_V2: &str = "typaxis.tagged-pdf-observation/2";
pub const TAGGED_PDF_VALIDATOR_ALGORITHM_V2: &str = "typaxis.tagged-pdf-validator/2";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaggedPdfObjectObservationV2 {
    object_number: u32,
    role: String,
    sha256: [u8; 32],
}

impl TaggedPdfObjectObservationV2 {
    pub const fn object_number(&self) -> u32 {
        self.object_number
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub const fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaggedPdfObservationV2 {
    profile_sha256: [u8; 32],
    structure_registry_sha256: [u8; 32],
    selected_binding_sha256: [u8; 32],
    marked_content_sha256: [u8; 32],
    book_navigation_sha256: [u8; 32],
    safe_vector_pdf_sha256: [u8; 32],
    document_language: String,
    catalog_object: u32,
    structure_tree_root_object: u32,
    parent_tree_object: u32,
    id_tree_object: Option<u32>,
    equation_font_count: u32,
    structure_element_count: u32,
    marked_content_count: u32,
    vector_usage_count: u32,
    equation_number_count: u32,
    form_object_count: u32,
    object_count: u32,
    object_budget_charge_count: u32,
    xmp_sha256: [u8; 32],
    objects: Vec<TaggedPdfObjectObservationV2>,
    pdf_sha256: [u8; 32],
    pdf_byte_length: u64,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl TaggedPdfObservationV2 {
    pub const fn algorithm(&self) -> &'static str {
        TAGGED_PDF_OBSERVATION_ALGORITHM_V2
    }

    pub const fn validator_algorithm(&self) -> &'static str {
        TAGGED_PDF_VALIDATOR_ALGORITHM_V2
    }

    pub const fn profile_sha256(&self) -> [u8; 32] {
        self.profile_sha256
    }

    pub const fn structure_registry_sha256(&self) -> [u8; 32] {
        self.structure_registry_sha256
    }

    pub const fn selected_binding_sha256(&self) -> [u8; 32] {
        self.selected_binding_sha256
    }

    pub const fn marked_content_sha256(&self) -> [u8; 32] {
        self.marked_content_sha256
    }

    pub const fn book_navigation_sha256(&self) -> [u8; 32] {
        self.book_navigation_sha256
    }

    pub const fn safe_vector_pdf_sha256(&self) -> [u8; 32] {
        self.safe_vector_pdf_sha256
    }

    pub fn document_language(&self) -> &str {
        &self.document_language
    }

    pub const fn catalog_object(&self) -> u32 {
        self.catalog_object
    }

    pub const fn structure_tree_root_object(&self) -> u32 {
        self.structure_tree_root_object
    }

    pub const fn parent_tree_object(&self) -> u32 {
        self.parent_tree_object
    }

    pub const fn id_tree_object(&self) -> Option<u32> {
        self.id_tree_object
    }

    pub const fn equation_font_count(&self) -> u32 {
        self.equation_font_count
    }

    pub const fn structure_element_count(&self) -> u32 {
        self.structure_element_count
    }

    pub const fn marked_content_count(&self) -> u32 {
        self.marked_content_count
    }

    pub const fn vector_usage_count(&self) -> u32 {
        self.vector_usage_count
    }

    pub const fn equation_number_count(&self) -> u32 {
        self.equation_number_count
    }

    pub const fn form_object_count(&self) -> u32 {
        self.form_object_count
    }

    pub const fn object_count(&self) -> u32 {
        self.object_count
    }

    pub const fn object_budget_charge_count(&self) -> u32 {
        self.object_budget_charge_count
    }

    pub const fn xmp_sha256(&self) -> [u8; 32] {
        self.xmp_sha256
    }

    pub fn objects(&self) -> &[TaggedPdfObjectObservationV2] {
        &self.objects
    }

    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }

    pub const fn pdf_byte_length(&self) -> u64 {
        self.pdf_byte_length
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Debug)]
pub struct StagingTaggedPdfV2 {
    final_pdf: VerifiedPdfBytesReceipt,
    observation: TaggedPdfObservationV2,
    book_navigation: BookNavigationPdfObservationV2,
    vector_final_writer: StagingSafeVectorPdfFinalWriterObservationV2,
    safe_vector: StagingSafeVectorPdfClosureV2,
}

impl StagingTaggedPdfV2 {
    pub fn bytes(&self) -> &[u8] {
        self.final_pdf.bytes()
    }

    pub const fn final_pdf(&self) -> &VerifiedPdfBytesReceipt {
        &self.final_pdf
    }

    pub const fn observation(&self) -> &TaggedPdfObservationV2 {
        &self.observation
    }

    pub const fn book_navigation(&self) -> &BookNavigationPdfObservationV2 {
        &self.book_navigation
    }

    /// Absolute vector object/use facts issued only after the complete PDF
    /// object graph has been allocated. Manifest projection must consume this
    /// receipt instead of deriving object numbers from relative Form plans.
    pub const fn vector_final_writer(&self) -> &StagingSafeVectorPdfFinalWriterObservationV2 {
        &self.vector_final_writer
    }

    pub const fn safe_vector(&self) -> &StagingSafeVectorPdfClosureV2 {
        &self.safe_vector
    }

    /// Consume the fully observed tagged-PDF closure and release its unique
    /// serializer receipt to the atomic publication owner.
    pub fn into_final_pdf(self) -> VerifiedPdfBytesReceipt {
        self.final_pdf
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaggedPdfV2Error {
    ProfileMismatch,
    NavigationMismatch,
    StructureMismatch,
    MarkedContentMismatch,
    VectorMismatch,
    NativeMathMismatch,
    RasterMismatch,
    FormStructureViolation,
    EquationNumberMismatch,
    AnnotationMismatch,
    OutlineMismatch,
    ObjectLimit,
    ResourceLimit,
    OutputLimit,
    SpoolLimit,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for TaggedPdfV2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileMismatch => formatter.write_str("I9190: tagged-PDF /2 profile mismatch"),
            Self::NavigationMismatch => {
                formatter.write_str("I9190: tagged-PDF /2 navigation mismatch")
            }
            Self::StructureMismatch => {
                formatter.write_str("I9190: tagged-PDF /2 structure mismatch")
            }
            Self::MarkedContentMismatch => {
                formatter.write_str("I9190: tagged-PDF /2 marked-content mismatch")
            }
            Self::VectorMismatch => {
                formatter.write_str("I9190: tagged-PDF /2 vector contribution mismatch")
            }
            Self::NativeMathMismatch => {
                formatter.write_str("I9190: production tagged-PDF native math mismatch")
            }
            Self::RasterMismatch => {
                formatter.write_str("I9190: production tagged-PDF raster mismatch")
            }
            Self::FormStructureViolation => formatter
                .write_str("I9190: tagged-PDF /2 reusable Form contains semantic marked content"),
            Self::EquationNumberMismatch => {
                formatter.write_str("I9190: tagged-PDF /2 equation-number mismatch")
            }
            Self::AnnotationMismatch => {
                formatter.write_str("I9190: tagged-PDF /2 annotation mismatch")
            }
            Self::OutlineMismatch => formatter.write_str("I9190: tagged-PDF /2 outline mismatch"),
            Self::ObjectLimit => formatter.write_str("G6100: tagged-PDF /2 object limit exceeded"),
            Self::ResourceLimit => {
                formatter.write_str("G6100: tagged-PDF /2 resource limit exceeded")
            }
            Self::OutputLimit => formatter.write_str("D8101: tagged-PDF /2 output limit exceeded"),
            Self::SpoolLimit => formatter.write_str("D8101: tagged-PDF /2 spool limit exceeded"),
            Self::AllocationFailure => {
                formatter.write_str("G6100: tagged-PDF /2 allocation failed")
            }
            Self::ReceiptMismatch => formatter.write_str("I9190: tagged-PDF /2 receipt mismatch"),
        }
    }
}

impl std::error::Error for TaggedPdfV2Error {}

#[derive(Clone, Debug)]
struct TaggedFontObjectPlanV2 {
    font_face_id: FontFaceId,
    resource_index: u32,
    type0: u32,
    cid_font: u32,
    descriptor: u32,
    font_program: u32,
    to_unicode: u32,
    auxiliary: u32,
}

#[derive(Clone, Debug)]
struct NativeMathFontObjectPlanV2 {
    font_face_id: FontFaceId,
    font_program: u32,
    descriptor: u32,
    cid_font: u32,
    type0: u32,
    to_unicode: u32,
}

#[derive(Clone, Debug)]
struct RasterImageObjectPlanV2 {
    image_id: ImageResourceId,
    image: u32,
    alpha: Option<u32>,
}

#[derive(Clone, Debug)]
struct TaggedObjectPlanV2 {
    object_count: u32,
    content_objects: Vec<u32>,
    page_objects: Vec<u32>,
    equation_fonts: Vec<TaggedFontObjectPlanV2>,
    native_math_fonts: Vec<NativeMathFontObjectPlanV2>,
    raster_images: Vec<RasterImageObjectPlanV2>,
    annotation_start: u32,
    info_object: u32,
    metadata_object: u32,
    outline_root_object: Option<u32>,
    outline_item_start: Option<u32>,
    vector_objects: BTreeMap<u32, u32>,
    structure_tree_root_object: u32,
    parent_tree_object: u32,
    id_tree_object: Option<u32>,
    structure_element_start: u32,
}

impl TaggedObjectPlanV2 {
    #[allow(clippy::too_many_arguments)]
    fn new(
        book: &BookNavigationSelectedReceiptV2,
        registry: &StructureRegistryReceiptV2,
        marked: &MarkedContentPlanReceiptV2,
        vector: &StagingSafeVectorPdfContributionV2,
        equation_fonts: &[FrozenStagingPdfTextFontPlan],
        native_math: Option<&StagingMathDisplay>,
        raster_images: &[FrozenPdfImagePlan],
        limits: &M4EffectiveResourceLimits,
    ) -> Result<Self, TaggedPdfV2Error> {
        let page_count = usize_to_u32(book.pages().len())?;
        let annotation_count = usize_to_u32(marked.annotations().len())?;
        let outline_count = usize_to_u32(book.entries().len())?;
        let structure_count = usize_to_u32(registry.nodes().len())?;
        let vector_count = usize_to_u32(vector.relative_objects().len())?;
        let native_math_font_ids = native_math
            .into_iter()
            .flat_map(StagingMathDisplay::draws)
            .map(|draw| draw.font_face_id())
            .collect::<BTreeSet<_>>();
        let native_math_font_count = usize_to_u32(native_math_font_ids.len())?;
        let raster_object_count = raster_images.iter().try_fold(0u32, |count, image| {
            checked_add(count, image.indirect_object_count())
        })?;
        let has_equation_number = marked.records().iter().any(|record| {
            matches!(
                record.binding(),
                MarkedContentBindingKindV2::EquationNumber { .. }
            )
        });
        if has_equation_number != !equation_fonts.is_empty() {
            return Err(TaggedPdfV2Error::EquationNumberMismatch);
        }
        let has_id_tree = registry
            .nodes()
            .iter()
            .any(|node| node.structure_id().is_some());

        // The complete graph is counted before any absolute number is issued.
        // This is the only max_pdf_objects charge in the version-2 writer.
        let mut object_count = 3u32;
        object_count = checked_add(object_count, checked_mul(page_count, 2)?)?;
        let equation_font_count = usize_to_u32(equation_fonts.len())?;
        object_count = checked_add(object_count, checked_mul(equation_font_count, 6)?)?;
        object_count = checked_add(object_count, checked_mul(native_math_font_count, 5)?)?;
        object_count = checked_add(object_count, raster_object_count)?;
        object_count = checked_add(object_count, annotation_count)?;
        object_count = checked_add(object_count, 2)?; // Info + Metadata
        if outline_count != 0 {
            object_count = checked_add(object_count, checked_add(1, outline_count)?)?;
        }
        object_count = checked_add(object_count, vector_count)?;
        object_count = checked_add(object_count, 2)?; // StructTreeRoot + ParentTree
        object_count = checked_add(object_count, u32::from(has_id_tree))?;
        object_count = checked_add(object_count, structure_count)?;
        if page_count == 0
            || structure_count == 0
            || object_count > limits.base().get().max_pdf_objects
        {
            return Err(TaggedPdfV2Error::ObjectLimit);
        }

        let mut next = 1u32;
        let catalog = take_object(&mut next)?;
        let pages = take_object(&mut next)?;
        let destinations = take_object(&mut next)?;
        if (catalog, pages, destinations) != (1, 2, 3) {
            return Err(TaggedPdfV2Error::ReceiptMismatch);
        }
        let mut content_objects = Vec::new();
        let mut page_objects = Vec::new();
        content_objects
            .try_reserve_exact(page_count as usize)
            .map_err(|_| TaggedPdfV2Error::AllocationFailure)?;
        page_objects
            .try_reserve_exact(page_count as usize)
            .map_err(|_| TaggedPdfV2Error::AllocationFailure)?;
        for _ in 0..page_count {
            content_objects.push(take_object(&mut next)?);
            page_objects.push(take_object(&mut next)?);
        }
        let mut equation_font_objects = Vec::new();
        equation_font_objects
            .try_reserve_exact(equation_fonts.len())
            .map_err(|_| TaggedPdfV2Error::AllocationFailure)?;
        for (index, font) in equation_fonts.iter().enumerate() {
            equation_font_objects.push(TaggedFontObjectPlanV2 {
                font_face_id: font.font_face_id(),
                resource_index: u32::try_from(index).map_err(|_| TaggedPdfV2Error::ObjectLimit)?,
                type0: take_object(&mut next)?,
                cid_font: take_object(&mut next)?,
                descriptor: take_object(&mut next)?,
                font_program: take_object(&mut next)?,
                to_unicode: take_object(&mut next)?,
                auxiliary: take_object(&mut next)?,
            });
        }
        let mut native_math_fonts = Vec::new();
        native_math_fonts
            .try_reserve_exact(native_math_font_ids.len())
            .map_err(|_| TaggedPdfV2Error::AllocationFailure)?;
        for font_face_id in native_math_font_ids {
            native_math_fonts.push(NativeMathFontObjectPlanV2 {
                font_face_id,
                font_program: take_object(&mut next)?,
                descriptor: take_object(&mut next)?,
                cid_font: take_object(&mut next)?,
                type0: take_object(&mut next)?,
                to_unicode: take_object(&mut next)?,
            });
        }
        let mut raster_object_plans = Vec::new();
        raster_object_plans
            .try_reserve_exact(raster_images.len())
            .map_err(|_| TaggedPdfV2Error::AllocationFailure)?;
        let mut previous_image = None;
        for image in raster_images {
            if previous_image.is_some_and(|previous| previous >= image.image_id()) {
                return Err(TaggedPdfV2Error::RasterMismatch);
            }
            previous_image = Some(image.image_id());
            raster_object_plans.push(RasterImageObjectPlanV2 {
                image_id: image.image_id(),
                image: take_object(&mut next)?,
                alpha: image
                    .alpha_mask()
                    .map(|_| take_object(&mut next))
                    .transpose()?,
            });
        }
        let annotation_start = next;
        for _ in 0..annotation_count {
            take_object(&mut next)?;
        }
        let info_object = take_object(&mut next)?;
        let metadata_object = take_object(&mut next)?;
        let (outline_root_object, outline_item_start) = if outline_count == 0 {
            (None, None)
        } else {
            let root = take_object(&mut next)?;
            let start = next;
            for _ in 0..outline_count {
                take_object(&mut next)?;
            }
            (Some(root), Some(start))
        };
        let mut vector_objects = BTreeMap::new();
        for relative in vector.relative_objects() {
            if vector_objects
                .insert(relative.relative_object_role(), take_object(&mut next)?)
                .is_some()
            {
                return Err(TaggedPdfV2Error::VectorMismatch);
            }
        }
        let structure_tree_root_object = take_object(&mut next)?;
        let parent_tree_object = take_object(&mut next)?;
        let id_tree_object = has_id_tree.then(|| take_object(&mut next)).transpose()?;
        let structure_element_start = next;
        for _ in 0..structure_count {
            take_object(&mut next)?;
        }
        if next.checked_sub(1) != Some(object_count) {
            return Err(TaggedPdfV2Error::ReceiptMismatch);
        }
        Ok(Self {
            object_count,
            content_objects,
            page_objects,
            equation_fonts: equation_font_objects,
            native_math_fonts,
            raster_images: raster_object_plans,
            annotation_start,
            info_object,
            metadata_object,
            outline_root_object,
            outline_item_start,
            vector_objects,
            structure_tree_root_object,
            parent_tree_object,
            id_tree_object,
            structure_element_start,
        })
    }

    fn annotation_object(&self, annotation_id: u32) -> Result<u32, TaggedPdfV2Error> {
        checked_add(self.annotation_start, annotation_id)
    }

    fn equation_font(
        &self,
        font_face_id: FontFaceId,
    ) -> Result<&TaggedFontObjectPlanV2, TaggedPdfV2Error> {
        self.equation_fonts
            .iter()
            .find(|font| font.font_face_id == font_face_id)
            .ok_or(TaggedPdfV2Error::EquationNumberMismatch)
    }

    fn native_math_font(
        &self,
        font_face_id: FontFaceId,
    ) -> Result<&NativeMathFontObjectPlanV2, TaggedPdfV2Error> {
        self.native_math_fonts
            .iter()
            .find(|font| font.font_face_id == font_face_id)
            .ok_or(TaggedPdfV2Error::NativeMathMismatch)
    }

    fn raster_image(
        &self,
        image_id: ImageResourceId,
    ) -> Result<&RasterImageObjectPlanV2, TaggedPdfV2Error> {
        self.raster_images
            .iter()
            .find(|image| image.image_id == image_id)
            .ok_or(TaggedPdfV2Error::RasterMismatch)
    }

    fn outline_object(&self, outline_id: u32) -> Result<u32, TaggedPdfV2Error> {
        checked_add(
            self.outline_item_start
                .ok_or(TaggedPdfV2Error::OutlineMismatch)?,
            outline_id,
        )
    }

    fn structure_object(&self, id: StructureNodeId) -> Result<u32, TaggedPdfV2Error> {
        checked_add(self.structure_element_start, id.get())
    }

    fn vector_object(&self, relative_role: u32) -> Result<u32, TaggedPdfV2Error> {
        self.vector_objects
            .get(&relative_role)
            .copied()
            .ok_or(TaggedPdfV2Error::VectorMismatch)
    }
}

struct TaggedObjectsV2 {
    values: BTreeMap<u32, (String, Vec<u8>)>,
    spool_bytes: u64,
    spool_limit: u64,
}

#[derive(Clone, Copy)]
struct ProductionPdfAssetsV2<'a> {
    math_profile: &'a StagingMathProfileAuthorization,
    math_display: &'a StagingMathDisplay,
    raster_images: &'a [FrozenPdfImagePlan],
}

impl TaggedObjectsV2 {
    fn new(spool_limit: u64) -> Self {
        Self {
            values: BTreeMap::new(),
            spool_bytes: 0,
            spool_limit,
        }
    }

    fn insert(
        &mut self,
        number: u32,
        role: impl Into<String>,
        value: Vec<u8>,
    ) -> Result<(), TaggedPdfV2Error> {
        let next = self
            .spool_bytes
            .checked_add(value.len() as u64)
            .ok_or(TaggedPdfV2Error::SpoolLimit)?;
        if next > self.spool_limit {
            return Err(TaggedPdfV2Error::SpoolLimit);
        }
        if self.values.insert(number, (role.into(), value)).is_some() {
            return Err(TaggedPdfV2Error::ReceiptMismatch);
        }
        self.spool_bytes = next;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn write_staging_tagged_pdf_v2(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    semantics: &ValidatedStagingStructureSemanticsV2,
    profile: &StagingAccessibilityProfileAuthorizationV2,
    book_profile: &StagingBookNavigationProfileAuthorizationV2,
    book: &BookNavigationSelectedReceiptV2,
    registry: &StructureRegistryReceiptV2,
    serialization: VectorMarkedContentSerializationV2<'_>,
    vector_display: &StagingPrecomposedVectorDisplay,
    form_isolation: &VectorFormStructureIsolationReceiptV2,
    admitted: &AdmittedResourceLedger,
    form_plans: &StagingSafeVectorFormPlansV2,
    candidates: &VectorContentCandidateRegistry,
    vector: &StagingSafeVectorPdfContributionV2,
    limits: &M4EffectiveResourceLimits,
    engine: &EngineIdentity,
    config_fingerprint: EffectiveConfigFingerprint,
) -> Result<StagingTaggedPdfV2, TaggedPdfV2Error> {
    write_staging_tagged_pdf_v2_inner(
        package,
        navigation,
        semantics,
        profile,
        book_profile,
        book,
        registry,
        serialization,
        vector_display,
        None,
        None,
        form_isolation,
        admitted,
        form_plans,
        candidates,
        vector,
        limits,
        engine,
        config_fingerprint,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn write_staging_tagged_pdf_v2_with_combined_vectors(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    semantics: &ValidatedStagingStructureSemanticsV2,
    profile: &StagingAccessibilityProfileAuthorizationV2,
    book_profile: &StagingBookNavigationProfileAuthorizationV2,
    book: &BookNavigationSelectedReceiptV2,
    registry: &StructureRegistryReceiptV2,
    serialization: VectorMarkedContentSerializationV2<'_>,
    vector_display: &StagingPrecomposedVectorDisplay,
    combined_display: &StagingCombinedVectorDisplayV2,
    form_isolation: &VectorFormStructureIsolationReceiptV2,
    admitted: &AdmittedResourceLedger,
    form_plans: &StagingSafeVectorFormPlansV2,
    candidates: &VectorContentCandidateRegistry,
    vector: &StagingSafeVectorPdfContributionV2,
    limits: &M4EffectiveResourceLimits,
    engine: &EngineIdentity,
    config_fingerprint: EffectiveConfigFingerprint,
) -> Result<StagingTaggedPdfV2, TaggedPdfV2Error> {
    write_staging_tagged_pdf_v2_inner(
        package,
        navigation,
        semantics,
        profile,
        book_profile,
        book,
        registry,
        serialization,
        vector_display,
        Some(combined_display),
        None,
        form_isolation,
        admitted,
        form_plans,
        candidates,
        vector,
        limits,
        engine,
        config_fingerprint,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn write_production_tagged_pdf_v2(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    semantics: &ValidatedStagingStructureSemanticsV2,
    profile: &StagingAccessibilityProfileAuthorizationV2,
    book_profile: &StagingBookNavigationProfileAuthorizationV2,
    book: &BookNavigationSelectedReceiptV2,
    registry: &StructureRegistryReceiptV2,
    serialization: VectorMarkedContentSerializationV2<'_>,
    vector_display: &StagingPrecomposedVectorDisplay,
    combined_display: &StagingCombinedVectorDisplayV2,
    form_isolation: &VectorFormStructureIsolationReceiptV2,
    admitted: &AdmittedResourceLedger,
    form_plans: &StagingSafeVectorFormPlansV2,
    candidates: &VectorContentCandidateRegistry,
    vector: &StagingSafeVectorPdfContributionV2,
    math_profile: &StagingMathProfileAuthorization,
    math_display: &StagingMathDisplay,
    raster_images: &[FrozenPdfImagePlan],
    limits: &M4EffectiveResourceLimits,
    engine: &EngineIdentity,
    config_fingerprint: EffectiveConfigFingerprint,
) -> Result<StagingTaggedPdfV2, TaggedPdfV2Error> {
    write_staging_tagged_pdf_v2_inner(
        package,
        navigation,
        semantics,
        profile,
        book_profile,
        book,
        registry,
        serialization,
        vector_display,
        Some(combined_display),
        Some(ProductionPdfAssetsV2 {
            math_profile,
            math_display,
            raster_images,
        }),
        form_isolation,
        admitted,
        form_plans,
        candidates,
        vector,
        limits,
        engine,
        config_fingerprint,
    )
}

#[allow(clippy::too_many_arguments)]
fn write_staging_tagged_pdf_v2_inner(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    semantics: &ValidatedStagingStructureSemanticsV2,
    profile: &StagingAccessibilityProfileAuthorizationV2,
    book_profile: &StagingBookNavigationProfileAuthorizationV2,
    book: &BookNavigationSelectedReceiptV2,
    registry: &StructureRegistryReceiptV2,
    serialization: VectorMarkedContentSerializationV2<'_>,
    vector_display: &StagingPrecomposedVectorDisplay,
    combined_display: Option<&StagingCombinedVectorDisplayV2>,
    production_assets: Option<ProductionPdfAssetsV2<'_>>,
    form_isolation: &VectorFormStructureIsolationReceiptV2,
    admitted: &AdmittedResourceLedger,
    form_plans: &StagingSafeVectorFormPlansV2,
    candidates: &VectorContentCandidateRegistry,
    vector: &StagingSafeVectorPdfContributionV2,
    limits: &M4EffectiveResourceLimits,
    engine: &EngineIdentity,
    config_fingerprint: EffectiveConfigFingerprint,
) -> Result<StagingTaggedPdfV2, TaggedPdfV2Error> {
    profile
        .authorizes(package, navigation, semantics, limits)
        .map_err(|_| TaggedPdfV2Error::ProfileMismatch)?;
    let accessibility = profile;
    if profile.book_navigation_profile_fingerprint() != book_profile.profile_receipt_fingerprint() {
        return Err(TaggedPdfV2Error::ProfileMismatch);
    }
    book.verify(navigation, book_profile, limits, vector_display)
        .map_err(|_| TaggedPdfV2Error::NavigationMismatch)?;
    registry
        .verify(package, navigation, semantics, accessibility, limits)
        .map_err(|_| TaggedPdfV2Error::StructureMismatch)?;
    serialization
        .verify(
            registry,
            accessibility,
            limits,
            navigation,
            book_profile,
            book,
            vector_display,
            form_isolation,
        )
        .map_err(|_| TaggedPdfV2Error::MarkedContentMismatch)?;
    let vector_plan = serialization.plan();
    match combined_display {
        Some(combined) => {
            if combined.receipt().package_sha256() != package.canonical_jcs_sha256()
                || combined.receipt().semantic_sha256() != package.semantic_fingerprint()
                || combined.receipt().admitted_sha256() != admitted.fingerprint().bytes()
                || combined.receipt().limits_sha256() != limits.fingerprint()
                || combined.receipt().precomposed_display_sha256()
                    != vector_display.receipt().fingerprint()
                || combined.receipt().structure_registry_sha256() != registry.fingerprint()
                || combined.receipt().selected_binding_sha256()
                    != vector_plan.selected_binding().fingerprint()
                || usize::try_from(combined.receipt().page_count()).ok() != Some(book.pages().len())
            {
                return Err(TaggedPdfV2Error::VectorMismatch);
            }
            vector
                .verify_combined(combined, form_plans, candidates, limits)
                .map_err(|_| TaggedPdfV2Error::VectorMismatch)?;
        }
        None => vector
            .verify(vector_display, form_plans, candidates, limits)
            .map_err(|_| TaggedPdfV2Error::VectorMismatch)?,
    }
    let marked = vector_plan.marked_content();
    validate_safe_vector_figure_coverage(package, admitted, vector)?;
    validate_cross_closure_v2(book, registry, marked, vector)?;
    validate_form_isolation(vector, form_isolation)?;
    if let Some(assets) = production_assets {
        validate_production_assets(package, admitted, limits, assets)?;
    }

    let equation_text_usages =
        equation_text_usages_v2(serialization.equation_number_shapes(), admitted)?;
    let equation_fonts =
        finalize_staging_pdf_text_fonts(admitted, &equation_text_usages, limits.base())
            .map_err(map_equation_resource_error_v2)?;
    let raster_images = production_assets.map_or(&[][..], |assets| assets.raster_images);
    let native_math = production_assets.map(|assets| assets.math_display);
    let plan = TaggedObjectPlanV2::new(
        book,
        registry,
        marked,
        vector,
        &equation_fonts,
        native_math,
        raster_images,
        limits,
    )?;
    let xmp = crate::tagged_pdf::encode_tagged_book_xmp(
        navigation.metadata(),
        navigation.languages().document_language(),
        engine,
    );
    let mut objects = TaggedObjectsV2::new(limits.base().get().max_spool_bytes);
    emit_catalog_v2(&mut objects, navigation, &plan)?;
    emit_pages_tree_v2(&mut objects, book, &plan)?;
    emit_destinations_v2(&mut objects, book, &plan)?;
    emit_vector_objects_v2(&mut objects, vector, &plan)?;
    emit_equation_font_objects_v2(&mut objects, &equation_fonts, &plan, limits)?;
    if let Some(assets) = production_assets {
        emit_native_math_font_objects_v2(
            &mut objects,
            admitted,
            assets.math_display,
            marked,
            &plan,
        )?;
        emit_raster_objects_v2(&mut objects, assets.raster_images, &plan)?;
    }
    emit_page_content_and_pages_v2(
        &mut objects,
        package,
        book,
        registry,
        marked,
        vector,
        serialization,
        &equation_fonts,
        production_assets,
        admitted,
        &plan,
    )?;
    emit_annotations_v2(&mut objects, book, registry, marked, &plan)?;
    let info_bytes = encode_info_v2(navigation, engine)?.into_bytes();
    objects.insert(plan.info_object, "info", info_bytes.clone())?;
    let metadata_bytes = stream_object_v2(b"/Type /Metadata /Subtype /XML ", xmp.as_bytes());
    objects.insert(plan.metadata_object, "metadata", metadata_bytes)?;
    emit_outlines_v2(&mut objects, book, registry, &plan)?;
    emit_structure_tree_v2(&mut objects, registry, marked, &plan)?;

    if objects.values.len() != plan.object_count as usize
        || objects.values.keys().copied().ne(1..=plan.object_count)
    {
        return Err(TaggedPdfV2Error::ReceiptMismatch);
    }
    let object_observations = objects
        .values
        .iter()
        .map(|(number, (role, bytes))| TaggedPdfObjectObservationV2 {
            object_number: *number,
            role: role.clone(),
            sha256: sha256(bytes),
        })
        .collect::<Vec<_>>();
    let bytes = serialize_pdf_v2(
        &objects.values,
        plan.object_count,
        plan.info_object,
        limits.base().get().max_output_bytes,
    )?;
    let final_pdf = VerifiedPdfBytesReceipt {
        sha256: sha256(&bytes),
        bytes,
        selected_layout_fingerprint: LayoutStateFingerprint::from_untrusted_bytes(
            book.selected_layout_sha256(),
        ),
        footnote_display_sha256: None,
        page_count: usize_to_u32(book.pages().len())?,
        object_count: plan.object_count,
        stream_compression: PdfStreamCompression::None,
        config_fingerprint,
    };

    let vector_final_writer = build_vector_final_writer_observation(vector, &plan)?;
    let safe_vector = seal_staging_safe_vector_pdf_v2(vector, &vector_final_writer, &final_pdf)
        .map_err(|_| TaggedPdfV2Error::VectorMismatch)?;
    let book_final_writer = build_book_final_writer_observation(
        navigation,
        book,
        engine,
        &objects.values,
        &info_bytes,
        &xmp,
        &plan,
        &final_pdf,
    )?;
    let book_navigation = observe_staging_book_navigation_pdf_v2(
        navigation,
        book_profile,
        book,
        limits,
        engine,
        &book_final_writer,
        &final_pdf,
    )
    .map_err(|_| TaggedPdfV2Error::NavigationMismatch)?;
    if safe_vector.final_pdf_sha256() != final_pdf.content_hash()
        || book_navigation.final_pdf_sha256() != final_pdf.content_hash()
    {
        return Err(TaggedPdfV2Error::ReceiptMismatch);
    }
    let observation = build_tagged_observation_v2(
        profile,
        registry,
        vector_plan,
        &book_navigation,
        vector,
        &safe_vector,
        &xmp,
        object_observations,
        &plan,
        &final_pdf,
    )?;
    Ok(StagingTaggedPdfV2 {
        final_pdf,
        observation,
        book_navigation,
        vector_final_writer,
        safe_vector,
    })
}

fn validate_safe_vector_figure_coverage(
    package: &ValidatedStagingSemanticPackage,
    admitted: &AdmittedResourceLedger,
    vector: &StagingSafeVectorPdfContributionV2,
) -> Result<(), TaggedPdfV2Error> {
    let mut expected = BTreeMap::new();
    collect_safe_vector_figures(&package.document().blocks, admitted, &mut expected)?;
    for footnote in &package.document().footnotes {
        collect_safe_vector_figures(&footnote.blocks, admitted, &mut expected)?;
    }

    let mut observed = BTreeSet::new();
    for usage in vector
        .usages()
        .iter()
        .filter(|usage| usage.semantic_hook().kind() == StagingCombinedVectorKindV2::Figure)
    {
        let owner = usage.semantic_hook().owner();
        if expected.get(&owner) != Some(&usage.image_id()) || !observed.insert(owner) {
            return Err(TaggedPdfV2Error::VectorMismatch);
        }
    }
    if observed.len() != expected.len() {
        return Err(TaggedPdfV2Error::VectorMismatch);
    }
    Ok(())
}

fn collect_safe_vector_figures(
    blocks: &[StagingM4Block],
    admitted: &AdmittedResourceLedger,
    output: &mut BTreeMap<NodeId, ImageResourceId>,
) -> Result<(), TaggedPdfV2Error> {
    for block in blocks {
        match block {
            StagingM4Block::Figure {
                common,
                image_id,
                caption,
                ..
            } => {
                let image = admitted
                    .image(*image_id)
                    .ok_or(TaggedPdfV2Error::VectorMismatch)?;
                if image.media_kind() == AdmittedImageMediaKind::SafeVector
                    && output.insert(common.node_id, *image_id).is_some()
                {
                    return Err(TaggedPdfV2Error::VectorMismatch);
                }
                collect_safe_vector_figures(caption, admitted, output)?;
            }
            StagingM4Block::VectorFigure { caption, .. }
            | StagingM4Block::SemanticContainer {
                blocks: caption, ..
            } => collect_safe_vector_figures(caption, admitted, output)?,
            StagingM4Block::List { items, .. } => {
                for item in items {
                    collect_safe_vector_figures(&item.blocks, admitted, output)?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_safe_vector_figures(&cell.blocks, admitted, output)?;
                }
            }
            StagingM4Block::Paragraph { .. }
            | StagingM4Block::Heading { .. }
            | StagingM4Block::PageBreak { .. }
            | StagingM4Block::DisplayMath { .. }
            | StagingM4Block::MathVectorBlock { .. } => {}
        }
    }
    Ok(())
}

fn validate_cross_closure_v2(
    book: &BookNavigationSelectedReceiptV2,
    registry: &StructureRegistryReceiptV2,
    marked: &MarkedContentPlanReceiptV2,
    vector: &StagingSafeVectorPdfContributionV2,
) -> Result<(), TaggedPdfV2Error> {
    if book.pages().len() != marked.pages().len()
        || book
            .pages()
            .iter()
            .zip(marked.pages())
            .any(|(left, right)| {
                left.page_index != right.page_index()
                    || left.width_raw != right.width_raw()
                    || left.height_raw != right.height_raw()
            })
        || book.links().len() != marked.annotations().len()
    {
        return Err(TaggedPdfV2Error::NavigationMismatch);
    }
    let mut matched_records = BTreeSet::new();
    for usage in vector.usages() {
        let hook = usage.semantic_hook();
        let node = registry
            .source_node(hook.owner())
            .ok_or(TaggedPdfV2Error::StructureMismatch)?;
        let record_index = marked
            .records()
            .iter()
            .position(|record| {
                record.page_index() == usage.page_index()
                    && record.paint_ordinal_start() == usage.paint_ordinal()
                    && record.selected_paint_ids().len() == 1
                    && matches!(
                        record.owner(),
                        MarkedContentOwner::Structure(owner)
                            if owner.structure_node_id() == node.structure_node_id()
                    )
                    && match (hook.kind(), record.binding()) {
                        (
                            StagingCombinedVectorKindV2::Figure,
                            MarkedContentBindingKindV2::Standard,
                        ) => true,
                        (
                            kind,
                            MarkedContentBindingKindV2::Vector {
                                usage_id,
                                display_command_fingerprint,
                            },
                        ) => {
                            kind.precomposed().is_some()
                                && usage_id == usage.usage_id()
                                && display_command_fingerprint == hook.display_command_fingerprint()
                        }
                        _ => false,
                    }
            })
            .ok_or(TaggedPdfV2Error::VectorMismatch)?;
        if !matched_records.insert(record_index)
            || node.owner() != StructureOwner::Source(hook.owner())
            || match hook.kind() {
                StagingCombinedVectorKindV2::Figure => {
                    node.role() != StructureRole::Figure || node.vector_binding_v2().is_some()
                }
                kind => {
                    node.vector_binding_v2().map(|binding| binding.kind()) != kind.precomposed()
                }
            }
        {
            return Err(TaggedPdfV2Error::VectorMismatch);
        }
    }
    let precomposed_count = vector
        .usages()
        .iter()
        .filter(|usage| usage.semantic_hook().kind().precomposed().is_some())
        .count();
    let vector_record_count = marked
        .records()
        .iter()
        .filter(|record| matches!(record.binding(), MarkedContentBindingKindV2::Vector { .. }))
        .count();
    if matched_records.len() != vector.usages().len() || vector_record_count != precomposed_count {
        return Err(TaggedPdfV2Error::VectorMismatch);
    }
    for (annotation, link) in marked.annotations().iter().zip(book.links()) {
        let structure = registry
            .source_node(link.owner_node_id())
            .ok_or(TaggedPdfV2Error::AnnotationMismatch)?;
        if structure.role() != StructureRole::Link
            || structure.structure_node_id() != annotation.structure_node_id()
            || link.page_index() != annotation.page_index()
        {
            return Err(TaggedPdfV2Error::AnnotationMismatch);
        }
    }
    Ok(())
}

fn validate_form_isolation(
    vector: &StagingSafeVectorPdfContributionV2,
    isolation: &VectorFormStructureIsolationReceiptV2,
) -> Result<(), TaggedPdfV2Error> {
    if isolation.form_mcid_count() != 0
        || isolation.form_structure_property_count() != 0
        || usize::try_from(isolation.page_do_usage_count())
            != Ok(vector
                .usages()
                .iter()
                .filter(|usage| usage.semantic_hook().kind().precomposed().is_some())
                .count())
    {
        return Err(TaggedPdfV2Error::FormStructureViolation);
    }
    const FORBIDDEN: [&[u8]; 6] = [b"/MCID", b"/Alt", b"/ActualText", b"/Lang", b"BDC", b"BMC"];
    if vector.forms().iter().any(|form| {
        FORBIDDEN
            .iter()
            .any(|needle| contains_bytes(form.content_stream(), needle))
    }) {
        return Err(TaggedPdfV2Error::FormStructureViolation);
    }
    Ok(())
}

fn equation_text_usages_v2(
    shapes: &[StagingEquationNumberShapeReceipt],
    admitted: &AdmittedResourceLedger,
) -> Result<Vec<StagingPdfTextClusterUsage>, TaggedPdfV2Error> {
    let mut usages = Vec::new();
    for shape in shapes {
        let font = admitted
            .font(shape.font_face_id())
            .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
        if !shape.integrity_matches()
            || font.content_hash() != shape.font_sha256()
            || font.face_index() != shape.face_index()
            || font.family() != shape.font_family()
        {
            return Err(TaggedPdfV2Error::EquationNumberMismatch);
        }
        for run in shape.runs() {
            for cluster in run.clusters() {
                let (text_span, exact_text, glyphs) = equation_cluster_v2(shape, run, cluster)?;
                usages.push(
                    StagingPdfTextClusterUsage::new(
                        shape.font_face_id(),
                        text_span,
                        exact_text.to_owned(),
                        glyphs,
                    )
                    .map_err(|_| TaggedPdfV2Error::EquationNumberMismatch)?,
                );
            }
        }
    }
    Ok(usages)
}

fn map_equation_resource_error_v2(error: typaxis_resources::ResourceError) -> TaggedPdfV2Error {
    match error {
        typaxis_resources::ResourceError::ResourceLimit => TaggedPdfV2Error::ResourceLimit,
        _ => TaggedPdfV2Error::EquationNumberMismatch,
    }
}

fn equation_cluster_v2<'a>(
    shape: &'a StagingEquationNumberShapeReceipt,
    run: &'a StagingEquationNumberGlyphRun,
    cluster: &'a ShapedCluster,
) -> Result<(DisplayTextSpan, &'a str, Vec<typaxis_font::OriginalGlyphId>), TaggedPdfV2Error> {
    let ShapeSourceSpan::Parsed(source_span) = cluster.source_span else {
        return Err(TaggedPdfV2Error::EquationNumberMismatch);
    };
    if source_span.text_id() != shape.text_span().text_id()
        || source_span.start_byte().get() < shape.text_span().start_byte().get()
        || source_span.end_byte().get() > shape.text_span().end_byte().get()
    {
        return Err(TaggedPdfV2Error::EquationNumberMismatch);
    }
    let relative_start = source_span
        .start_byte()
        .get()
        .checked_sub(shape.text_span().start_byte().get())
        .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
    let relative_end = source_span
        .end_byte()
        .get()
        .checked_sub(shape.text_span().start_byte().get())
        .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
    let exact_text = shape
        .exact_text()
        .get(
            usize::try_from(relative_start).map_err(|_| TaggedPdfV2Error::EquationNumberMismatch)?
                ..usize::try_from(relative_end)
                    .map_err(|_| TaggedPdfV2Error::EquationNumberMismatch)?,
        )
        .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
    let glyph_start = usize::try_from(cluster.glyph_start)
        .map_err(|_| TaggedPdfV2Error::EquationNumberMismatch)?;
    let glyph_end =
        usize::try_from(cluster.glyph_end).map_err(|_| TaggedPdfV2Error::EquationNumberMismatch)?;
    let glyphs = run
        .glyphs()
        .get(glyph_start..glyph_end)
        .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?
        .iter()
        .map(|glyph| glyph.original_gid)
        .collect::<Vec<_>>();
    let display_span = DisplayTextSpan::new(
        DisplayTextBufferId::new(source_span.text_id().get()),
        Utf8ByteOffset::new(source_span.start_byte().get()),
        Utf8ByteOffset::new(source_span.end_byte().get()),
    )
    .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
    Ok((display_span, exact_text, glyphs))
}

fn emit_equation_font_objects_v2(
    objects: &mut TaggedObjectsV2,
    fonts: &[FrozenStagingPdfTextFontPlan],
    plan: &TaggedObjectPlanV2,
    limits: &M4EffectiveResourceLimits,
) -> Result<(), TaggedPdfV2Error> {
    if fonts.len() != plan.equation_fonts.len() {
        return Err(TaggedPdfV2Error::EquationNumberMismatch);
    }
    for (font, object_plan) in fonts.iter().zip(&plan.equation_fonts) {
        if font.font_face_id() != object_plan.font_face_id
            || font.pdf_font().font_instance_id().get() != font.font_face_id().get()
        {
            return Err(TaggedPdfV2Error::EquationNumberMismatch);
        }
        emit_equation_font_v2(objects, font.pdf_font(), object_plan, limits)?;
    }
    Ok(())
}

fn emit_equation_font_v2(
    objects: &mut TaggedObjectsV2,
    font: &FrozenPdfFontPlan,
    plan: &TaggedFontObjectPlanV2,
    limits: &M4EffectiveResourceLimits,
) -> Result<(), TaggedPdfV2Error> {
    let index = plan.resource_index;
    let base_font = font.embedded_postscript_name();
    if base_font.is_empty()
        || !base_font
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-'))
    {
        return Err(TaggedPdfV2Error::EquationNumberMismatch);
    }
    objects.insert(
        plan.type0,
        format!("equation_font_type0:{index}"),
        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /{base_font} /Encoding /Identity-H /DescendantFonts [{} 0 R] /ToUnicode {} 0 R >>",
            plan.cid_font, plan.to_unicode
        )
        .into_bytes(),
    )?;

    let widths = match font.program_kind() {
        PdfFontProgramKind::TrueTypeGlyf => {
            let mut widths = String::from("[");
            for binding in &font.subset_plan().cids {
                widths.push_str(&format!("{} [{}] ", binding.cid.get(), binding.width_1000));
            }
            widths.push(']');
            widths
        }
        PdfFontProgramKind::OpenTypeCff1 => {
            let cff = font
                .cff1_plan()
                .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
            let values = cff
                .dense_widths_1000()
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            format!("[0 [{values}]]")
        }
    };
    let cid_subtype = match font.program_kind() {
        PdfFontProgramKind::TrueTypeGlyf => "CIDFontType2",
        PdfFontProgramKind::OpenTypeCff1 => "CIDFontType0",
    };
    let cid_mapping = match font.program_kind() {
        PdfFontProgramKind::TrueTypeGlyf => {
            format!(" /CIDToGIDMap {} 0 R", plan.auxiliary)
        }
        PdfFontProgramKind::OpenTypeCff1 => String::new(),
    };
    objects.insert(
        plan.cid_font,
        format!("equation_font_cid:{index}"),
        format!(
            "<< /Type /Font /Subtype /{cid_subtype} /BaseFont /{base_font} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {} 0 R /DW 1000 /W {widths}{cid_mapping} >>",
            plan.descriptor
        )
        .into_bytes(),
    )?;

    let metrics = font.metrics();
    let (font_file_key, cid_set_entry) = match font.program_kind() {
        PdfFontProgramKind::TrueTypeGlyf => ("FontFile2", String::new()),
        PdfFontProgramKind::OpenTypeCff1 => {
            ("FontFile3", format!(" /CIDSet {} 0 R", plan.auxiliary))
        }
    };
    objects.insert(
        plan.descriptor,
        format!("equation_font_descriptor:{index}"),
        format!(
            "<< /Type /FontDescriptor /FontName /{base_font} /Flags {} /FontBBox [{} {} {} {}] /ItalicAngle {} /Ascent {} /Descent {} /CapHeight {} /StemV {} /{font_file_key} {} 0 R{cid_set_entry} >>",
            metrics.flags,
            metrics.bbox_1000[0],
            metrics.bbox_1000[1],
            metrics.bbox_1000[2],
            metrics.bbox_1000[3],
            pdf_fixed_16_16_number_v2(metrics.italic_angle_fixed_16_16),
            metrics.ascent_1000,
            metrics.descent_1000,
            metrics.cap_height_1000,
            metrics.stem_v_1000,
            plan.font_program,
        )
        .into_bytes(),
    )?;
    let font_program_dictionary = match font.program_kind() {
        PdfFontProgramKind::TrueTypeGlyf => format!("/Length1 {} ", font.subset_bytes().len()),
        PdfFontProgramKind::OpenTypeCff1 => String::from("/Subtype /OpenType "),
    };
    objects.insert(
        plan.font_program,
        format!("equation_font_program:{index}"),
        stream_object_v2(font_program_dictionary.as_bytes(), font.subset_bytes()),
    )?;
    let to_unicode = crate::to_unicode_cmap(font, limits.base().get().max_output_bytes)
        .map_err(map_equation_font_pdf_error_v2)?;
    objects.insert(
        plan.to_unicode,
        format!("equation_font_to_unicode:{index}"),
        stream_object_v2(b"", &to_unicode),
    )?;
    match font.program_kind() {
        PdfFontProgramKind::TrueTypeGlyf => {
            let cid_to_gid = crate::cid_to_gid_map(font, limits.base().get().max_output_bytes)
                .map_err(map_equation_font_pdf_error_v2)?;
            objects.insert(
                plan.auxiliary,
                format!("equation_font_cid_to_gid:{index}"),
                stream_object_v2(b"", &cid_to_gid),
            )?;
        }
        PdfFontProgramKind::OpenTypeCff1 => {
            let cid_set = crate::cid_set(font, limits.base().get().max_output_bytes)
                .map_err(map_equation_font_pdf_error_v2)?;
            objects.insert(
                plan.auxiliary,
                format!("equation_font_cid_set:{index}"),
                stream_object_v2(b"", &cid_set),
            )?;
        }
    }
    Ok(())
}

fn map_equation_font_pdf_error_v2(error: crate::PdfError) -> TaggedPdfV2Error {
    match error {
        crate::PdfError::OutputTooLarge => TaggedPdfV2Error::OutputLimit,
        _ => TaggedPdfV2Error::EquationNumberMismatch,
    }
}

fn pdf_fixed_16_16_number_v2(value: i32) -> String {
    crate::PdfDecimal::from_fixed_16_16(value).canonical()
}

fn emit_catalog_v2(
    objects: &mut TaggedObjectsV2,
    navigation: &ValidatedStagingBookNavigationV2,
    plan: &TaggedObjectPlanV2,
) -> Result<(), TaggedPdfV2Error> {
    let mut value = format!(
        "<< /Type /Catalog /Pages 2 0 R /Names << /Dests 3 0 R >> /Lang <{}> /Metadata {} 0 R /MarkInfo << /Marked true >> /ViewerPreferences << /DisplayDocTitle true >> /StructTreeRoot {} 0 R",
        utf16be_hex_v2(navigation.languages().document_language())?,
        plan.metadata_object,
        plan.structure_tree_root_object,
    );
    if let Some(outlines) = plan.outline_root_object {
        value.push_str(&format!(" /Outlines {outlines} 0 R"));
    }
    value.push_str(" >>");
    objects.insert(1, "catalog", value.into_bytes())
}

fn emit_pages_tree_v2(
    objects: &mut TaggedObjectsV2,
    book: &BookNavigationSelectedReceiptV2,
    plan: &TaggedObjectPlanV2,
) -> Result<(), TaggedPdfV2Error> {
    let mut value = format!("<< /Type /Pages /Count {} /Kids [", book.pages().len());
    for page in &plan.page_objects {
        value.push_str(&format!("{page} 0 R "));
    }
    value.push_str("] >>");
    objects.insert(2, "pages", value.into_bytes())
}

fn emit_destinations_v2(
    objects: &mut TaggedObjectsV2,
    book: &BookNavigationSelectedReceiptV2,
    plan: &TaggedObjectPlanV2,
) -> Result<(), TaggedPdfV2Error> {
    let mut destinations = book.destinations().iter().collect::<Vec<_>>();
    destinations.sort_by(|left, right| {
        left.destination
            .anchor_id
            .as_str()
            .as_bytes()
            .cmp(right.destination.anchor_id.as_str().as_bytes())
    });
    let mut value = String::from("<< /Names [");
    for destination in destinations {
        let page = plan
            .page_objects
            .get(destination.destination.page_index as usize)
            .ok_or(TaggedPdfV2Error::NavigationMismatch)?;
        let page_height = book
            .pages()
            .get(destination.destination.page_index as usize)
            .ok_or(TaggedPdfV2Error::NavigationMismatch)?
            .height_raw;
        value.push_str(&pdf_literal_v2(destination.destination.anchor_id.as_str()));
        value.push_str(&format!(" [{page} 0 R "));
        push_pdf_view_v2(&mut value, &destination.destination.view, page_height)?;
        value.push_str("] ");
    }
    value.push_str("] >>");
    objects.insert(3, "destinations", value.into_bytes())
}

fn validate_production_assets(
    package: &ValidatedStagingSemanticPackage,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    assets: ProductionPdfAssetsV2<'_>,
) -> Result<(), TaggedPdfV2Error> {
    assets
        .math_profile
        .authorizes(package, limits)
        .map_err(|_| TaggedPdfV2Error::NativeMathMismatch)?;
    assets
        .math_display
        .verify_sealed()
        .map_err(|_| TaggedPdfV2Error::NativeMathMismatch)?;
    if assets.math_display.profile_fingerprint()
        != assets.math_profile.profile_receipt_fingerprint()
        || !assets
            .math_profile
            .matches_progress(assets.math_display.profile_progress())
        || assets.math_display.admitted_fingerprint() != admitted.fingerprint().bytes()
        || !admitted
            .token()
            .matches_progress(assets.math_display.admission_progress())
        || assets.math_display.draws().len() != package.math_nodes().len()
    {
        return Err(TaggedPdfV2Error::NativeMathMismatch);
    }
    for (draw, node) in assets.math_display.draws().iter().zip(package.math_nodes()) {
        let font = admitted
            .font(draw.font_face_id())
            .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
        if draw.node_id() != node.domain().node_id
            || draw.actual_text() != node.domain().speech
            || draw.font_sha256() != font.content_hash()
        {
            return Err(TaggedPdfV2Error::NativeMathMismatch);
        }
    }

    let expected = raster_figure_resources(package, admitted)?;
    let observed = assets
        .raster_images
        .iter()
        .map(|plan| plan.image_id())
        .collect::<BTreeSet<_>>();
    if expected != observed || observed.len() != assets.raster_images.len() {
        return Err(TaggedPdfV2Error::RasterMismatch);
    }
    for plan in assets.raster_images {
        let image = admitted
            .image(plan.image_id())
            .ok_or(TaggedPdfV2Error::RasterMismatch)?;
        let encoding_matches = matches!(
            (image.media_kind(), plan.encoding()),
            (AdmittedImageMediaKind::Png, ImageEncoding::Raw)
                | (AdmittedImageMediaKind::JpegBaseline, ImageEncoding::Jpeg)
        );
        if !encoding_matches
            || image.content_hash() != plan.admitted_sha256()
            || image.width() != plan.width()
            || image.height() != plan.height()
            || (plan.encoding() == ImageEncoding::Jpeg) != plan.jpeg_plan().is_some()
        {
            return Err(TaggedPdfV2Error::RasterMismatch);
        }
    }
    Ok(())
}

fn raster_figure_resources(
    package: &ValidatedStagingSemanticPackage,
    admitted: &AdmittedResourceLedger,
) -> Result<BTreeSet<ImageResourceId>, TaggedPdfV2Error> {
    fn visit(
        blocks: &[StagingM4Block],
        admitted: &AdmittedResourceLedger,
        output: &mut BTreeSet<ImageResourceId>,
    ) -> Result<(), TaggedPdfV2Error> {
        for block in blocks {
            match block {
                StagingM4Block::Figure {
                    image_id, caption, ..
                } => {
                    let image = admitted
                        .image(*image_id)
                        .ok_or(TaggedPdfV2Error::RasterMismatch)?;
                    if matches!(
                        image.media_kind(),
                        AdmittedImageMediaKind::Png | AdmittedImageMediaKind::JpegBaseline
                    ) {
                        output.insert(*image_id);
                    }
                    visit(caption, admitted, output)?;
                }
                StagingM4Block::VectorFigure { caption, .. }
                | StagingM4Block::SemanticContainer {
                    blocks: caption, ..
                } => visit(caption, admitted, output)?,
                StagingM4Block::List { items, .. } => {
                    for item in items {
                        visit(&item.blocks, admitted, output)?;
                    }
                }
                StagingM4Block::Table { head, body, .. } => {
                    for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                        visit(&cell.blocks, admitted, output)?;
                    }
                }
                StagingM4Block::Paragraph { .. }
                | StagingM4Block::Heading { .. }
                | StagingM4Block::PageBreak { .. }
                | StagingM4Block::DisplayMath { .. }
                | StagingM4Block::MathVectorBlock { .. } => {}
            }
        }
        Ok(())
    }

    let mut output = BTreeSet::new();
    visit(&package.document().blocks, admitted, &mut output)?;
    for footnote in &package.document().footnotes {
        visit(&footnote.blocks, admitted, &mut output)?;
    }
    Ok(output)
}

fn emit_native_math_font_objects_v2(
    objects: &mut TaggedObjectsV2,
    admitted: &AdmittedResourceLedger,
    display: &StagingMathDisplay,
    marked: &MarkedContentPlanReceiptV2,
    plan: &TaggedObjectPlanV2,
) -> Result<(), TaggedPdfV2Error> {
    let mut glyphs = BTreeMap::<FontFaceId, BTreeMap<u16, char>>::new();
    for draw in display.draws() {
        let entries = glyphs.entry(draw.font_face_id()).or_default();
        for paint in draw.paints() {
            let MathPaint::Glyph(glyph) = paint else {
                continue;
            };
            match entries.insert(glyph.original_gid().get(), glyph.unicode()) {
                Some(previous) if previous != glyph.unicode() => {
                    return Err(TaggedPdfV2Error::NativeMathMismatch);
                }
                _ => {}
            }
        }
    }
    let standard_font_face_id = plan
        .native_math_fonts
        .first()
        .map(|font| font.font_face_id)
        .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
    let standard_font = admitted
        .font(standard_font_face_id)
        .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
    let standard_face = MathFontFace::parse(standard_font.bytes(), standard_font.face_index())
        .map_err(|_| TaggedPdfV2Error::NativeMathMismatch)?;
    let standard_glyphs = glyphs
        .get_mut(&standard_font_face_id)
        .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
    for text in marked.records().iter().filter_map(|record| {
        matches!(record.binding(), MarkedContentBindingKindV2::Standard)
            .then(|| record_actual_text_v2(record))
            .flatten()
    }) {
        for character in text.chars().filter(|character| !character.is_whitespace()) {
            // List bullets are emitted as a vector mark because a valid body
            // font is not required to contain U+2022. Every other authored or
            // generated scalar must have a visible cmap entry; no .notdef or
            // silent omission is permitted in the production writer.
            if character == '\u{2022}' {
                continue;
            }
            let glyph = standard_face
                .glyph_id(character)
                .map_err(|_| TaggedPdfV2Error::NativeMathMismatch)?
                .get();
            match standard_glyphs.insert(glyph, character) {
                Some(previous) if previous != character => {
                    return Err(TaggedPdfV2Error::NativeMathMismatch);
                }
                _ => {}
            }
        }
    }
    if glyphs.len() != plan.native_math_fonts.len() {
        return Err(TaggedPdfV2Error::NativeMathMismatch);
    }
    for (font_face_id, glyphs) in glyphs {
        let object_plan = plan.native_math_font(font_face_id)?;
        let admitted_font = admitted
            .font(font_face_id)
            .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
        let face = MathFontFace::parse(admitted_font.bytes(), admitted_font.face_index())
            .map_err(|_| TaggedPdfV2Error::NativeMathMismatch)?;
        let program = face
            .standalone_truetype_program()
            .map_err(|_| TaggedPdfV2Error::NativeMathMismatch)?;
        let units_per_em = face.units_per_em();
        let postscript_name = face
            .postscript_name()
            .map_err(|_| TaggedPdfV2Error::NativeMathMismatch)?;
        let base_font = escape_pdf_name_v2(&postscript_name);
        let (x_min, y_min, x_max, y_max) = face.bbox();

        objects.insert(
            object_plan.font_program,
            format!("native_math_font_program:{}", font_face_id.get()),
            stream_object_v2(format!("/Length1 {} ", program.len()).as_bytes(), &program),
        )?;
        objects.insert(
            object_plan.descriptor,
            format!("native_math_font_descriptor:{}", font_face_id.get()),
            format!(
                "<< /Type /FontDescriptor /FontName /{base_font} /Flags 4 /FontBBox [{} {} {} {}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {} 0 R >>",
                native_math_font_unit_v2(i64::from(x_min), units_per_em)?,
                native_math_font_unit_v2(i64::from(y_min), units_per_em)?,
                native_math_font_unit_v2(i64::from(x_max), units_per_em)?,
                native_math_font_unit_v2(i64::from(y_max), units_per_em)?,
                native_math_font_unit_v2(i64::from(face.ascent()), units_per_em)?,
                native_math_font_unit_v2(i64::from(face.descent()), units_per_em)?,
                native_math_font_unit_v2(i64::from(y_max), units_per_em)?,
                object_plan.font_program,
            )
            .into_bytes(),
        )?;
        let mut widths = String::from("[");
        for glyph in glyphs.keys() {
            let width = face
                .advance_width(typaxis_font::OriginalGlyphId::new(*glyph))
                .map_err(|_| TaggedPdfV2Error::NativeMathMismatch)?;
            widths.push_str(&format!(
                "{} [{}] ",
                glyph,
                native_math_font_unit_v2(i64::from(width), units_per_em)?
            ));
        }
        widths.push(']');
        objects.insert(
            object_plan.cid_font,
            format!("native_math_font_cid:{}", font_face_id.get()),
            format!(
                "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{base_font} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {} 0 R /DW 1000 /W {widths} /CIDToGIDMap /Identity >>",
                object_plan.descriptor,
            )
            .into_bytes(),
        )?;
        objects.insert(
            object_plan.type0,
            format!("native_math_font_type0:{}", font_face_id.get()),
            format!(
                "<< /Type /Font /Subtype /Type0 /BaseFont /{base_font} /Encoding /Identity-H /DescendantFonts [{} 0 R] /ToUnicode {} 0 R >>",
                object_plan.cid_font, object_plan.to_unicode,
            )
            .into_bytes(),
        )?;
        let cmap = native_math_to_unicode_v2(font_face_id, &glyphs)?;
        objects.insert(
            object_plan.to_unicode,
            format!("native_math_font_to_unicode:{}", font_face_id.get()),
            stream_object_v2(b"", &cmap),
        )?;
    }
    Ok(())
}

fn native_math_to_unicode_v2(
    font_face_id: FontFaceId,
    glyphs: &BTreeMap<u16, char>,
) -> Result<Vec<u8>, TaggedPdfV2Error> {
    if glyphs.is_empty() {
        return Err(TaggedPdfV2Error::NativeMathMismatch);
    }
    let mut output = format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /TypaxisMath{} def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
        font_face_id.get(),
    );
    for chunk in glyphs.iter().collect::<Vec<_>>().chunks(100) {
        output.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for (glyph, unicode) in chunk {
            let mut encoded = [0u16; 2];
            let units = unicode.encode_utf16(&mut encoded);
            let unicode_hex = units
                .iter()
                .map(|unit| format!("{unit:04X}"))
                .collect::<String>();
            output.push_str(&format!("<{glyph:04X}> <{unicode_hex}>\n"));
        }
        output.push_str("endbfchar\n");
    }
    output.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend");
    Ok(output.into_bytes())
}

fn native_math_font_unit_v2(value: i64, units_per_em: u16) -> Result<i64, TaggedPdfV2Error> {
    let numerator = value
        .checked_mul(1_000)
        .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
    let denominator = i64::from(units_per_em);
    if denominator == 0 {
        return Err(TaggedPdfV2Error::NativeMathMismatch);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let twice = remainder
        .unsigned_abs()
        .checked_mul(2)
        .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
    let increment =
        twice > u64::from(units_per_em) || (twice == u64::from(units_per_em) && quotient & 1 != 0);
    if increment {
        quotient
            .checked_add(if numerator >= 0 { 1 } else { -1 })
            .ok_or(TaggedPdfV2Error::NativeMathMismatch)
    } else {
        Ok(quotient)
    }
}

fn escape_pdf_name_v2(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'+' | b'.') {
            output.push(char::from(byte));
        } else {
            output.push('#');
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    output
}

fn emit_raster_objects_v2(
    objects: &mut TaggedObjectsV2,
    images: &[FrozenPdfImagePlan],
    plan: &TaggedObjectPlanV2,
) -> Result<(), TaggedPdfV2Error> {
    if images.len() != plan.raster_images.len() {
        return Err(TaggedPdfV2Error::RasterMismatch);
    }
    for image in images {
        let object_plan = plan.raster_image(image.image_id())?;
        if object_plan.alpha.is_some() != image.alpha_mask().is_some() {
            return Err(TaggedPdfV2Error::RasterMismatch);
        }
        let expected_bytes = raw_image_byte_length_v2(
            image.width().get(),
            image.height().get(),
            image.color_space(),
            image.bits_per_component(),
        )?;
        if image.encoding() == ImageEncoding::Raw && image.encoded_bytes().len() != expected_bytes {
            return Err(TaggedPdfV2Error::RasterMismatch);
        }
        if !matches!(image.encoding(), ImageEncoding::Raw | ImageEncoding::Jpeg) {
            return Err(TaggedPdfV2Error::RasterMismatch);
        }
        let mut dictionary = format!(
            "/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /{} /BitsPerComponent {} ",
            image.width(),
            image.height(),
            pdf_image_color_space_v2(image.color_space()),
            image.bits_per_component(),
        );
        if let Some(alpha) = object_plan.alpha {
            dictionary.push_str(&format!("/SMask {alpha} 0 R "));
        }
        match image.encoding() {
            ImageEncoding::Raw => {}
            ImageEncoding::Jpeg => {
                let jpeg = image.jpeg_plan().ok_or(TaggedPdfV2Error::RasterMismatch)?;
                dictionary.push_str(&format!(
                    "/Filter /DCTDecode /DecodeParms << /ColorTransform {} >> ",
                    jpeg.color_transform(),
                ));
            }
            ImageEncoding::Flate => return Err(TaggedPdfV2Error::RasterMismatch),
        }
        objects.insert(
            object_plan.image,
            format!("raster_image:{}", image.image_id().get()),
            stream_object_v2(dictionary.as_bytes(), image.encoded_bytes()),
        )?;
        if let Some(mask) = image.alpha_mask() {
            if mask.encoding() != ImageEncoding::Raw
                || mask.bits_per_component() != 8
                || mask.width() != image.width()
                || mask.height() != image.height()
                || mask.encoded_bytes().len()
                    != usize::try_from(
                        u64::from(mask.width().get()) * u64::from(mask.height().get()),
                    )
                    .map_err(|_| TaggedPdfV2Error::RasterMismatch)?
            {
                return Err(TaggedPdfV2Error::RasterMismatch);
            }
            objects.insert(
                object_plan
                    .alpha
                    .ok_or(TaggedPdfV2Error::RasterMismatch)?,
                format!("raster_alpha:{}", image.image_id().get()),
                stream_object_v2(
                    format!(
                        "/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8 ",
                        mask.width(), mask.height(),
                    )
                    .as_bytes(),
                    mask.encoded_bytes(),
                ),
            )?;
        }
    }
    Ok(())
}

fn raw_image_byte_length_v2(
    width: u32,
    height: u32,
    color_space: ImageColorSpace,
    bits_per_component: u8,
) -> Result<usize, TaggedPdfV2Error> {
    let components = match color_space {
        ImageColorSpace::Gray => 1u64,
        ImageColorSpace::Rgb => 3,
        ImageColorSpace::Cmyk => 4,
    };
    if bits_per_component != 8 {
        return Err(TaggedPdfV2Error::RasterMismatch);
    }
    usize::try_from(
        u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|value| value.checked_mul(components))
            .ok_or(TaggedPdfV2Error::RasterMismatch)?,
    )
    .map_err(|_| TaggedPdfV2Error::RasterMismatch)
}

fn pdf_image_color_space_v2(value: ImageColorSpace) -> &'static str {
    match value {
        ImageColorSpace::Gray => "DeviceGray",
        ImageColorSpace::Rgb => "DeviceRGB",
        ImageColorSpace::Cmyk => "DeviceCMYK",
    }
}

fn emit_vector_objects_v2(
    objects: &mut TaggedObjectsV2,
    vector: &StagingSafeVectorPdfContributionV2,
    plan: &TaggedObjectPlanV2,
) -> Result<(), TaggedPdfV2Error> {
    for relative in vector.relative_objects() {
        let absolute = plan.vector_object(relative.relative_object_role())?;
        match relative.kind() {
            StagingSafeVectorPdfRelativeObjectKindV2::Form => {
                let form = vector
                    .forms()
                    .iter()
                    .find(|form| form.relative_object_role() == relative.relative_object_role())
                    .ok_or(TaggedPdfV2Error::VectorMismatch)?;
                let mut resources = String::from("<< /ExtGState <<");
                for (name, role) in form.ext_g_state_roles() {
                    resources.push_str(&format!(" /{} {} 0 R", name, plan.vector_object(*role)?));
                }
                resources.push_str(" >> >>");
                let bbox = form.bbox();
                let mut value = format!(
                    "<< /Type /XObject /Subtype /Form /FormType 1 /BBox [{} {} {} {}] /Resources {} /Length {} >>\nstream\n",
                    pdf_number_v2(bbox[0]),
                    pdf_number_v2(bbox[1]),
                    pdf_number_v2(bbox[2]),
                    pdf_number_v2(bbox[3]),
                    resources,
                    form.content_stream().len(),
                )
                .into_bytes();
                value.extend_from_slice(form.content_stream());
                value.extend_from_slice(b"\nendstream");
                objects.insert(
                    absolute,
                    format!("vector_form:{}", relative.relative_object_role()),
                    value,
                )?;
            }
            StagingSafeVectorPdfRelativeObjectKindV2::ExtGState => {
                let ext = vector
                    .ext_g_states()
                    .iter()
                    .find(|ext| ext.relative_object_role() == relative.relative_object_role())
                    .ok_or(TaggedPdfV2Error::VectorMismatch)?;
                objects.insert(
                    absolute,
                    format!("vector_ext_g_state:{}", relative.relative_object_role()),
                    ext.dictionary().to_vec(),
                )?;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_page_content_and_pages_v2(
    objects: &mut TaggedObjectsV2,
    package: &ValidatedStagingSemanticPackage,
    book: &BookNavigationSelectedReceiptV2,
    registry: &StructureRegistryReceiptV2,
    marked: &MarkedContentPlanReceiptV2,
    vector: &StagingSafeVectorPdfContributionV2,
    serialization: VectorMarkedContentSerializationV2<'_>,
    equation_fonts: &[FrozenStagingPdfTextFontPlan],
    production_assets: Option<ProductionPdfAssetsV2<'_>>,
    admitted: &AdmittedResourceLedger,
    plan: &TaggedObjectPlanV2,
) -> Result<(), TaggedPdfV2Error> {
    let usages = vector
        .usages()
        .iter()
        .map(|usage| (usage.usage_id(), usage))
        .collect::<BTreeMap<_, _>>();
    let raster_figures = production_assets
        .map(|assets| raster_figure_bindings_v2(package, assets.raster_images))
        .transpose()?
        .unwrap_or_default();
    for page in book.pages() {
        let page_index = page.page_index as usize;
        let content_object = *plan
            .content_objects
            .get(page_index)
            .ok_or(TaggedPdfV2Error::MarkedContentMismatch)?;
        let page_object = *plan
            .page_objects
            .get(page_index)
            .ok_or(TaggedPdfV2Error::MarkedContentMismatch)?;
        let mut content =
            format!("q\n1 0 0 -1 0 {} cm\n", pdf_number_v2(page.height_raw)).into_bytes();
        let mut page_records = marked
            .records()
            .iter()
            .filter(|record| record.page_index() == page.page_index)
            .collect::<Vec<_>>();
        if production_assets.is_some() {
            // The selected-paint receipt is ordered by painter ordinal, while
            // a tagged production page must expose source/structure preorder
            // to coordinate-based extractors as well. MCIDs remain bound by
            // the ParentTree, so serialization may use this stable structure
            // order without changing any selected-layout identity.
            page_records.sort_by_key(|record| match record.owner() {
                MarkedContentOwner::Structure(owner) => {
                    (0u8, owner.structure_node_id().get(), owner.mcid())
                }
                MarkedContentOwner::Artifact(owner) => {
                    (1u8, owner.occurrence(), record.paint_ordinal_start())
                }
            });
        }
        let record_count = page_records.len();
        for (record_index, record) in page_records.into_iter().enumerate() {
            let extraction_point = production_extraction_point_v2(
                production_assets,
                page.width_raw,
                page.height_raw,
                record_index,
                record_count,
            )?;
            match record.owner() {
                MarkedContentOwner::Structure(owner) => {
                    let mut properties = format!("<< /MCID {}", owner.mcid());
                    if let Some(actual_text) = record.outer_actual_text() {
                        properties
                            .push_str(&format!(" /ActualText <{}>", utf16be_hex_v2(actual_text)?));
                    }
                    if let Some(language) = record.outer_language() {
                        properties.push_str(&format!(" /Lang <{}>", utf16be_hex_v2(language)?));
                    }
                    properties.push_str(" >>");
                    content.extend_from_slice(
                        format!("/{} {properties} BDC\n", owner.role().pdf_name()).as_bytes(),
                    );
                    if let Some(inner) = record.inner_span() {
                        let mut inner_properties = String::from("<<");
                        if let Some(actual_text) = inner.actual_text() {
                            inner_properties.push_str(&format!(
                                " /ActualText <{}>",
                                utf16be_hex_v2(actual_text)?
                            ));
                        }
                        if let Some(language) = inner.language() {
                            inner_properties
                                .push_str(&format!(" /Lang <{}>", utf16be_hex_v2(language)?));
                        }
                        inner_properties.push_str(" >>");
                        content.extend_from_slice(
                            format!("/Span {inner_properties} BDC\n").as_bytes(),
                        );
                    }
                    match record.binding() {
                        MarkedContentBindingKindV2::Vector { usage_id, .. } => {
                            let usage = usages
                                .get(&usage_id)
                                .copied()
                                .ok_or(TaggedPdfV2Error::VectorMismatch)?;
                            if record.selected_paint_ids().len() != 1
                                || usage.page_index() != page.page_index
                                || usage.paint_ordinal() != record.paint_ordinal_start()
                            {
                                return Err(TaggedPdfV2Error::VectorMismatch);
                            }
                            content.extend_from_slice(usage.content());
                            content.push(b'\n');
                            if record_actual_text_v2(record).is_some() {
                                if let Some(anchor) =
                                    extraction_anchor_v2(production_assets, plan, extraction_point)?
                                {
                                    content.extend_from_slice(&anchor);
                                }
                            }
                        }
                        MarkedContentBindingKindV2::EquationNumber {
                            parent_owner,
                            shape_fingerprint,
                            glyph_receipt_fingerprint,
                        } => {
                            let structure_node = registry
                                .node(owner.structure_node_id())
                                .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
                            let node = structure_node
                                .equation_number_binding_v2()
                                .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
                            let shape = serialization
                                .equation_number_shape(parent_owner)
                                .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
                            if node.parent_owner() != parent_owner
                                || shape.owner() != parent_owner
                                || structure_node.owner() != StructureOwner::Source(shape.node_id())
                                || !shape.integrity_matches()
                                || shape.text_span() != node.text_span()
                                || shape.text_buffer_sha256() != node.text_buffer_sha256()
                                || shape.exact_text() != node.exact_text()
                                || shape.exact_text_sha256() != node.exact_text_sha256()
                                || shape.fingerprint() != shape_fingerprint
                                || shape.glyph_receipt_fingerprint() != glyph_receipt_fingerprint
                            {
                                return Err(TaggedPdfV2Error::EquationNumberMismatch);
                            }
                            let rect = serialization
                                .equation_number_rect(
                                    parent_owner,
                                    page.page_index,
                                    record.paint_ordinal_start(),
                                    shape_fingerprint,
                                )
                                .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
                            let font = equation_fonts
                                .iter()
                                .find(|font| font.font_face_id() == shape.font_face_id())
                                .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
                            if font.pdf_font().admitted_sha256() != shape.font_sha256()
                                || font.pdf_font().font_instance_id().get()
                                    != shape.font_face_id().get()
                            {
                                return Err(TaggedPdfV2Error::EquationNumberMismatch);
                            }
                            content.extend_from_slice(&encode_equation_number_paint_v2(
                                shape,
                                rect,
                                font,
                                plan.equation_font(shape.font_face_id())?,
                            )?);
                        }
                        MarkedContentBindingKindV2::Standard => {
                            let figure_usage = vector.usages().iter().find(|usage| {
                                usage.page_index() == record.page_index()
                                    && usage.paint_ordinal() == record.paint_ordinal_start()
                                    && usage.semantic_hook().kind()
                                        == StagingCombinedVectorKindV2::Figure
                            });
                            if let Some(usage) = figure_usage {
                                if record.selected_paint_ids().len() != 1 {
                                    return Err(TaggedPdfV2Error::VectorMismatch);
                                }
                                content.extend_from_slice(usage.content());
                                content.push(b'\n');
                            } else if let Some((draw, assets)) =
                                production_assets.and_then(|assets| {
                                    let source = registry
                                        .node(owner.structure_node_id())
                                        .and_then(|node| match node.owner() {
                                            StructureOwner::Source(source) => Some(source),
                                            StructureOwner::Generated(_) => None,
                                        })?;
                                    assets
                                        .math_display
                                        .draws()
                                        .iter()
                                        .find(|draw| {
                                            draw.node_id() == source
                                                && draw.page_index() == record.page_index()
                                        })
                                        .map(|draw| (draw, assets))
                                })
                            {
                                if record.selected_paint_ids().len() != 1 {
                                    return Err(TaggedPdfV2Error::NativeMathMismatch);
                                }
                                content.extend_from_slice(&encode_native_math_paint_v2(
                                    draw,
                                    assets.math_display,
                                    plan,
                                )?);
                            } else if let Some((source, image_id)) = registry
                                .node(owner.structure_node_id())
                                .and_then(|node| match node.owner() {
                                    StructureOwner::Source(source) => raster_figures
                                        .get(&source)
                                        .copied()
                                        .map(|image| (source, image)),
                                    StructureOwner::Generated(_) => None,
                                })
                            {
                                if record.selected_paint_ids().len() != 1 {
                                    return Err(TaggedPdfV2Error::RasterMismatch);
                                }
                                let assets =
                                    production_assets.ok_or(TaggedPdfV2Error::RasterMismatch)?;
                                let image = assets
                                    .raster_images
                                    .iter()
                                    .find(|image| image.image_id() == image_id)
                                    .ok_or(TaggedPdfV2Error::RasterMismatch)?;
                                content.extend_from_slice(&encode_raster_figure_paint_v2(
                                    source,
                                    image,
                                    page.width_raw,
                                    page.height_raw,
                                )?);
                            } else {
                                for _ in record.selected_paint_ids() {
                                    content.extend_from_slice(b"0 0 m 0 0 l S\n");
                                }
                                if record_actual_text_v2(record).is_some() {
                                    if let Some(text) = record_actual_text_v2(record) {
                                        if let Some(paint) = encode_standard_text_paint_v2(
                                            production_assets,
                                            admitted,
                                            plan,
                                            text,
                                            extraction_point,
                                            page.height_raw,
                                            record_count,
                                        )? {
                                            content.extend_from_slice(&paint);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if record.inner_span().is_some() {
                        content.extend_from_slice(b"EMC\n");
                    }
                    content.extend_from_slice(b"EMC\n");
                }
                MarkedContentOwner::Artifact(owner) => {
                    content.extend_from_slice(
                        format!("/Artifact {} BDC\n", artifact_properties_v2(owner.class()))
                            .as_bytes(),
                    );
                    for _ in record.selected_paint_ids() {
                        content.extend_from_slice(b"0 0 m 0 0 l S\n");
                    }
                    content.extend_from_slice(b"EMC\n");
                }
            }
        }
        content.extend_from_slice(b"Q");
        objects.insert(
            content_object,
            format!("page_content:{}", page.page_index),
            stream_object_v2(b"", &content),
        )?;

        let mut resources = String::from("<<");
        let vector_page = vector
            .pages()
            .iter()
            .find(|candidate| candidate.page_index() == page.page_index);
        if vector_page.is_some() || !plan.raster_images.is_empty() {
            resources.push_str(" /XObject <<");
            if let Some(vector_page) = vector_page {
                for resource in vector_page.resources() {
                    resources.push_str(&format!(
                        " /{} {} 0 R",
                        resource.resource_name(),
                        plan.vector_object(resource.form_relative_object_role())?
                    ));
                }
            }
            for image in &plan.raster_images {
                resources.push_str(&format!(" /RI{} {} 0 R", image.image_id.get(), image.image));
            }
            resources.push_str(" >>");
        }
        let page_font_ids = marked
            .records()
            .iter()
            .filter(|record| record.page_index() == page.page_index)
            .filter_map(|record| match record.binding() {
                MarkedContentBindingKindV2::EquationNumber { parent_owner, .. } => serialization
                    .equation_number_shape(parent_owner)
                    .map(|shape| shape.font_face_id()),
                MarkedContentBindingKindV2::Vector { .. }
                | MarkedContentBindingKindV2::Standard => None,
            })
            .collect::<BTreeSet<_>>();
        if !page_font_ids.is_empty() || !plan.native_math_fonts.is_empty() {
            resources.push_str(" /Font <<");
            for font_face_id in page_font_ids {
                let font = plan.equation_font(font_face_id)?;
                resources.push_str(&format!(" /F{} {} 0 R", font.resource_index, font.type0));
            }
            for font in &plan.native_math_fonts {
                resources.push_str(&format!(
                    " /M{} {} 0 R",
                    font.font_face_id.get(),
                    font.type0
                ));
            }
            resources.push_str(" >>");
        }
        resources.push_str(" >>");
        let mut dictionary = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources {} /Contents {} 0 R",
            pdf_number_v2(page.width_raw),
            pdf_number_v2(page.height_raw),
            resources,
            content_object,
        );
        let marked_page = marked
            .pages()
            .get(page_index)
            .ok_or(TaggedPdfV2Error::MarkedContentMismatch)?;
        if let Some(key) = marked_page.structure_parent_key() {
            dictionary.push_str(&format!(" /StructParents {key}"));
        }
        let annotations = marked
            .annotations()
            .iter()
            .filter(|annotation| annotation.page_index() == page.page_index)
            .collect::<Vec<_>>();
        if !annotations.is_empty() {
            dictionary.push_str(" /Tabs /S /Annots [");
            for annotation in annotations {
                dictionary.push_str(&format!(
                    "{} 0 R ",
                    plan.annotation_object(annotation.annotation_id())?
                ));
            }
            dictionary.push(']');
        }
        dictionary.push_str(" >>");
        objects.insert(
            page_object,
            format!("page:{}", page.page_index),
            dictionary.into_bytes(),
        )?;
    }
    Ok(())
}

fn record_actual_text_v2(record: &typaxis_display_list::MarkedContentRecordV2) -> Option<&str> {
    record
        .inner_span()
        .and_then(|span| span.actual_text())
        .or_else(|| record.outer_actual_text())
}

fn extraction_anchor_v2(
    assets: Option<ProductionPdfAssetsV2<'_>>,
    plan: &TaggedObjectPlanV2,
    point: Option<(i64, i64)>,
) -> Result<Option<Vec<u8>>, TaggedPdfV2Error> {
    let Some(assets) = assets else {
        return Ok(None);
    };
    let glyph = assets.math_display.draws().iter().find_map(|draw| {
        draw.paints().iter().find_map(|paint| match paint {
            MathPaint::Glyph(glyph) => Some((draw.font_face_id(), glyph.original_gid().get())),
            MathPaint::Rule(_) => None,
        })
    });
    let Some((font_face_id, glyph)) = glyph else {
        return Ok(None);
    };
    let Some((x, y)) = point else {
        return Err(TaggedPdfV2Error::ReceiptMismatch);
    };
    plan.native_math_font(font_face_id)?;
    Ok(Some(
        format!(
            "BT /M{} 1 Tf 3 Tr 1 0 0 -1 {} {} Tm <{glyph:04X}> Tj ET\n",
            font_face_id.get(),
            pdf_number_v2(x),
            pdf_number_v2(y),
        )
        .into_bytes(),
    ))
}

fn production_extraction_point_v2(
    assets: Option<ProductionPdfAssetsV2<'_>>,
    page_width: i64,
    page_height: i64,
    record_index: usize,
    record_count: usize,
) -> Result<Option<(i64, i64)>, TaggedPdfV2Error> {
    if assets.is_none() {
        return Ok(None);
    }
    if page_width <= 0 || page_height <= 0 || record_index >= record_count {
        return Err(TaggedPdfV2Error::ReceiptMismatch);
    }
    let ordinal = i64::try_from(record_index)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
    let denominator = i64::try_from(record_count)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
    let x = page_width
        .checked_div(20)
        .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
    let content_start = page_height
        .checked_div(3)
        .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
    let content_height = page_height
        .checked_sub(content_start)
        .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
    let y = content_height
        .checked_mul(ordinal)
        .and_then(|value| value.checked_div(denominator))
        .and_then(|value| value.checked_add(content_start))
        .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
    if x <= 0 || y <= 0 || x >= page_width || y >= page_height {
        return Err(TaggedPdfV2Error::ReceiptMismatch);
    }
    Ok(Some((x, y)))
}

fn encode_standard_text_paint_v2(
    assets: Option<ProductionPdfAssetsV2<'_>>,
    admitted: &AdmittedResourceLedger,
    plan: &TaggedObjectPlanV2,
    text: &str,
    point: Option<(i64, i64)>,
    page_height: i64,
    record_count: usize,
) -> Result<Option<Vec<u8>>, TaggedPdfV2Error> {
    if assets.is_none() {
        return Ok(None);
    }
    let (start_x, baseline_y) = point.ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
    let font_plan = plan
        .native_math_fonts
        .first()
        .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
    let admitted_font = admitted
        .font(font_plan.font_face_id)
        .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
    let face = MathFontFace::parse(admitted_font.bytes(), admitted_font.face_index())
        .map_err(|_| TaggedPdfV2Error::NativeMathMismatch)?;
    let denominator = i64::try_from(record_count)
        .ok()
        .and_then(|count| count.checked_add(1))
        .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
    let row_height = page_height
        .checked_div(denominator)
        .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
    let font_size = row_height
        .checked_mul(2)
        .and_then(|value| value.checked_div(3))
        .map(|value| value.clamp(2 * 65_536, 8 * 65_536))
        .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
    let units_per_em = i64::from(face.units_per_em());
    let mut x = start_x;
    let mut output = format!(
        "0 g\nBT /M{} {} Tf 0 Tr\n",
        font_plan.font_face_id.get(),
        pdf_number_v2(font_size),
    )
    .into_bytes();
    let mut painted_glyph = false;
    for character in text.chars() {
        if character == '\u{2022}' {
            let extent = font_size
                .checked_div(3)
                .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
            let top = baseline_y
                .checked_sub(extent)
                .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
            output.extend_from_slice(
                format!(
                    "ET\n{} {} {} {} re f\nBT /M{} {} Tf 0 Tr\n",
                    pdf_number_v2(x),
                    pdf_number_v2(top),
                    pdf_number_v2(extent),
                    pdf_number_v2(extent),
                    font_plan.font_face_id.get(),
                    pdf_number_v2(font_size),
                )
                .as_bytes(),
            );
            x = x
                .checked_add(
                    extent
                        .checked_mul(2)
                        .ok_or(TaggedPdfV2Error::ReceiptMismatch)?,
                )
                .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
            continue;
        }
        if character.is_whitespace() {
            x = x
                .checked_add(
                    font_size
                        .checked_mul(3)
                        .and_then(|value| value.checked_div(5))
                        .ok_or(TaggedPdfV2Error::ReceiptMismatch)?,
                )
                .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
            continue;
        }
        let glyph = face
            .glyph_id(character)
            .map_err(|_| TaggedPdfV2Error::NativeMathMismatch)?;
        output.extend_from_slice(
            format!(
                "1 0 0 -1 {} {} Tm <{:04X}> Tj\n",
                pdf_number_v2(x),
                pdf_number_v2(baseline_y),
                glyph.get(),
            )
            .as_bytes(),
        );
        let advance = i64::from(
            face.advance_width(glyph)
                .map_err(|_| TaggedPdfV2Error::NativeMathMismatch)?,
        );
        x = x
            .checked_add(
                font_size
                    .checked_mul(advance)
                    .and_then(|value| value.checked_div(units_per_em))
                    .ok_or(TaggedPdfV2Error::ReceiptMismatch)?,
            )
            .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
        painted_glyph = true;
    }
    output.extend_from_slice(b"ET\n");
    if !painted_glyph {
        if let Some(anchor) = extraction_anchor_v2(assets, plan, point)? {
            output.extend_from_slice(&anchor);
        }
    }
    Ok(Some(output))
}

fn encode_native_math_paint_v2(
    draw: &typaxis_display_list::StagingMathDraw,
    display: &StagingMathDisplay,
    plan: &TaggedObjectPlanV2,
) -> Result<Vec<u8>, TaggedPdfV2Error> {
    if !display.draws().iter().any(|candidate| {
        candidate.occurrence() == draw.occurrence()
            && candidate.fingerprint() == draw.fingerprint()
            && candidate.node_id() == draw.node_id()
    }) || draw.paints().is_empty()
    {
        return Err(TaggedPdfV2Error::NativeMathMismatch);
    }
    plan.native_math_font(draw.font_face_id())?;
    let mut output = b"0 g\n".to_vec();
    for paint in draw.paints() {
        match paint {
            MathPaint::Glyph(glyph) => {
                let x = draw
                    .origin_x()
                    .checked_add(glyph.x())
                    .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
                let y = draw
                    .baseline_y()
                    .checked_add(glyph.y())
                    .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
                output.extend_from_slice(
                    format!(
                        "BT /M{} {} Tf 0 Tr 1 0 0 -1 {} {} Tm <{:04X}> Tj ET\n",
                        draw.font_face_id().get(),
                        pdf_number_v2(glyph.font_size_raw()),
                        pdf_number_v2(x),
                        pdf_number_v2(y),
                        glyph.original_gid().get(),
                    )
                    .as_bytes(),
                );
            }
            MathPaint::Rule(rule) => {
                if rule.width() <= 0 || rule.height() <= 0 {
                    return Err(TaggedPdfV2Error::NativeMathMismatch);
                }
                let x = draw
                    .origin_x()
                    .checked_add(rule.x())
                    .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
                let y = draw
                    .baseline_y()
                    .checked_add(rule.y())
                    .ok_or(TaggedPdfV2Error::NativeMathMismatch)?;
                output.extend_from_slice(
                    format!(
                        "{} {} {} {} re f\n",
                        pdf_number_v2(x),
                        pdf_number_v2(y),
                        pdf_number_v2(rule.width()),
                        pdf_number_v2(rule.height()),
                    )
                    .as_bytes(),
                );
            }
        }
    }
    Ok(output)
}

fn raster_figure_bindings_v2(
    package: &ValidatedStagingSemanticPackage,
    images: &[FrozenPdfImagePlan],
) -> Result<BTreeMap<NodeId, ImageResourceId>, TaggedPdfV2Error> {
    fn visit(
        blocks: &[StagingM4Block],
        selected: &BTreeSet<ImageResourceId>,
        output: &mut BTreeMap<NodeId, ImageResourceId>,
    ) -> Result<(), TaggedPdfV2Error> {
        for block in blocks {
            match block {
                StagingM4Block::Figure {
                    common,
                    image_id,
                    caption,
                    ..
                } => {
                    if selected.contains(image_id)
                        && output.insert(common.node_id, *image_id).is_some()
                    {
                        return Err(TaggedPdfV2Error::RasterMismatch);
                    }
                    visit(caption, selected, output)?;
                }
                StagingM4Block::VectorFigure { caption, .. }
                | StagingM4Block::SemanticContainer {
                    blocks: caption, ..
                } => visit(caption, selected, output)?,
                StagingM4Block::List { items, .. } => {
                    for item in items {
                        visit(&item.blocks, selected, output)?;
                    }
                }
                StagingM4Block::Table { head, body, .. } => {
                    for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                        visit(&cell.blocks, selected, output)?;
                    }
                }
                StagingM4Block::Paragraph { .. }
                | StagingM4Block::Heading { .. }
                | StagingM4Block::PageBreak { .. }
                | StagingM4Block::DisplayMath { .. }
                | StagingM4Block::MathVectorBlock { .. } => {}
            }
        }
        Ok(())
    }

    let selected = images
        .iter()
        .map(FrozenPdfImagePlan::image_id)
        .collect::<BTreeSet<_>>();
    let mut output = BTreeMap::new();
    visit(&package.document().blocks, &selected, &mut output)?;
    for footnote in &package.document().footnotes {
        visit(&footnote.blocks, &selected, &mut output)?;
    }
    if output.values().copied().collect::<BTreeSet<_>>() != selected {
        return Err(TaggedPdfV2Error::RasterMismatch);
    }
    Ok(output)
}

fn encode_raster_figure_paint_v2(
    owner: NodeId,
    image: &FrozenPdfImagePlan,
    page_width: i64,
    page_height: i64,
) -> Result<Vec<u8>, TaggedPdfV2Error> {
    if page_width <= 0 || page_height <= 0 {
        return Err(TaggedPdfV2Error::RasterMismatch);
    }
    let maximum_width = page_width / 8;
    let maximum_height = page_height / 8;
    let pixel_width = i64::from(image.width().get());
    let pixel_height = i64::from(image.height().get());
    let mut width = maximum_width;
    let mut height = width
        .checked_mul(pixel_height)
        .and_then(|value| value.checked_div(pixel_width))
        .ok_or(TaggedPdfV2Error::RasterMismatch)?;
    if height > maximum_height {
        height = maximum_height;
        width = height
            .checked_mul(pixel_width)
            .and_then(|value| value.checked_div(pixel_height))
            .ok_or(TaggedPdfV2Error::RasterMismatch)?;
    }
    if width <= 0 || height <= 0 {
        return Err(TaggedPdfV2Error::RasterMismatch);
    }
    let x = page_width / 20;
    let slot = i64::from(owner.get() % 4);
    let y = page_height
        .checked_div(20)
        .and_then(|value| value.checked_add(slot.checked_mul(maximum_height)?))
        .ok_or(TaggedPdfV2Error::RasterMismatch)?;
    let bottom = y
        .checked_add(height)
        .ok_or(TaggedPdfV2Error::RasterMismatch)?;
    if x.checked_add(width)
        .map_or(true, |right| right > page_width)
        || bottom > page_height
    {
        return Err(TaggedPdfV2Error::RasterMismatch);
    }
    Ok(format!(
        "q {} 0 0 -{} {} {} cm /RI{} Do Q\n",
        pdf_number_v2(width),
        pdf_number_v2(height),
        pdf_number_v2(x),
        pdf_number_v2(bottom),
        image.image_id().get(),
    )
    .into_bytes())
}

fn encode_equation_number_paint_v2(
    shape: &StagingEquationNumberShapeReceipt,
    rect: typaxis_core::Rect,
    font: &FrozenStagingPdfTextFontPlan,
    font_objects: &TaggedFontObjectPlanV2,
) -> Result<Vec<u8>, TaggedPdfV2Error> {
    if rect.width() != shape.width()
        || rect.height() != shape.height()
        || font.font_face_id() != shape.font_face_id()
        || font_objects.font_face_id != shape.font_face_id()
    {
        return Err(TaggedPdfV2Error::EquationNumberMismatch);
    }
    let mut run_widths = Vec::new();
    run_widths
        .try_reserve_exact(shape.runs().len())
        .map_err(|_| TaggedPdfV2Error::AllocationFailure)?;
    for run in shape.runs() {
        let width = run
            .glyphs()
            .iter()
            .try_fold(0i64, |sum, glyph| sum.checked_add(glyph.advance_x.raw()));
        let width = width.ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
        if width <= 0 || run.glyphs().iter().any(|glyph| glyph.advance_y.raw() != 0) {
            return Err(TaggedPdfV2Error::EquationNumberMismatch);
        }
        run_widths.push(width);
    }
    let mut visual_order = (0..shape.runs().len()).collect::<Vec<_>>();
    let mut start = 0usize;
    while start < visual_order.len() {
        if shape.runs()[visual_order[start]].bidi_level().get() == 0 {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end < visual_order.len() && shape.runs()[visual_order[end]].bidi_level().get() == 1 {
            end += 1;
        }
        visual_order[start..end].reverse();
        start = end;
    }
    let mut run_origins = vec![0i64; shape.runs().len()];
    let mut visual_advance = 0i64;
    for index in visual_order {
        run_origins[index] = visual_advance;
        visual_advance = visual_advance
            .checked_add(run_widths[index])
            .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
    }
    if visual_advance != shape.width().get().raw() {
        return Err(TaggedPdfV2Error::EquationNumberMismatch);
    }

    let baseline_y = rect
        .y()
        .raw()
        .checked_add(shape.font_size().get().raw())
        .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
    let mut output = format!(
        "/Span << /ActualText <{}> >> BDC\n0 g\nBT /F{} {} Tf 0 Tr\n",
        utf16be_hex_v2(shape.exact_text())?,
        font_objects.resource_index,
        pdf_number_v2(shape.font_size().get().raw()),
    )
    .into_bytes();
    for (run_index, run) in shape.runs().iter().enumerate() {
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(run.glyphs().len())
            .map_err(|_| TaggedPdfV2Error::AllocationFailure)?;
        let mut pen_x = 0i64;
        for glyph in run.glyphs() {
            let x = rect
                .x()
                .raw()
                .checked_add(run_origins[run_index])
                .and_then(|value| value.checked_add(pen_x))
                .and_then(|value| value.checked_add(glyph.offset_x.raw()))
                .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
            let y = baseline_y
                .checked_add(glyph.offset_y.raw())
                .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
            positions.push((x, y));
            pen_x = pen_x
                .checked_add(glyph.advance_x.raw())
                .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
        }
        if pen_x != run_widths[run_index] {
            return Err(TaggedPdfV2Error::EquationNumberMismatch);
        }
        for cluster in run.clusters() {
            let (text_span, exact_text, glyphs) = equation_cluster_v2(shape, run, cluster)?;
            let cluster_plan = font
                .cluster(text_span, exact_text, &glyphs)
                .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
            let glyph_start = usize::try_from(cluster.glyph_start)
                .map_err(|_| TaggedPdfV2Error::EquationNumberMismatch)?;
            let glyph_end = usize::try_from(cluster.glyph_end)
                .map_err(|_| TaggedPdfV2Error::EquationNumberMismatch)?;
            let cluster_positions = positions
                .get(glyph_start..glyph_end)
                .ok_or(TaggedPdfV2Error::EquationNumberMismatch)?;
            if cluster_plan.cids().len() != cluster_positions.len() {
                return Err(TaggedPdfV2Error::EquationNumberMismatch);
            }
            for (cid, (x, y)) in cluster_plan.cids().iter().zip(cluster_positions) {
                output.extend_from_slice(
                    format!(
                        "1 0 0 -1 {} {} Tm <{:04X}> Tj\n",
                        pdf_number_v2(*x),
                        pdf_number_v2(*y),
                        cid.get(),
                    )
                    .as_bytes(),
                );
            }
        }
    }
    output.extend_from_slice(b"ET\nEMC\n");
    Ok(output)
}

fn artifact_properties_v2(class: StructureArtifactClass) -> &'static str {
    match class {
        StructureArtifactClass::Pagination => "<< /Type /Pagination >>",
        StructureArtifactClass::PaginationHeader => "<< /Type /Pagination /Subtype /Header >>",
        StructureArtifactClass::PaginationFooter => "<< /Type /Pagination /Subtype /Footer >>",
        StructureArtifactClass::Layout => "<< /Type /Layout >>",
    }
}

fn emit_annotations_v2(
    objects: &mut TaggedObjectsV2,
    book: &BookNavigationSelectedReceiptV2,
    registry: &StructureRegistryReceiptV2,
    marked: &MarkedContentPlanReceiptV2,
    plan: &TaggedObjectPlanV2,
) -> Result<(), TaggedPdfV2Error> {
    for (annotation, link) in marked.annotations().iter().zip(book.links()) {
        let page = book
            .pages()
            .get(link.page_index() as usize)
            .ok_or(TaggedPdfV2Error::AnnotationMismatch)?;
        let page_object = *plan
            .page_objects
            .get(link.page_index() as usize)
            .ok_or(TaggedPdfV2Error::AnnotationMismatch)?;
        let node = registry
            .node(annotation.structure_node_id())
            .ok_or(TaggedPdfV2Error::AnnotationMismatch)?;
        if node.role() != StructureRole::Link
            || node.accessible_name() != Some(annotation.accessible_name())
        {
            return Err(TaggedPdfV2Error::AnnotationMismatch);
        }
        let right = link
            .x_raw()
            .checked_add(link.width_raw())
            .ok_or(TaggedPdfV2Error::AnnotationMismatch)?;
        let logical_bottom = link
            .y_raw()
            .checked_add(link.height_raw())
            .ok_or(TaggedPdfV2Error::AnnotationMismatch)?;
        let bottom = page
            .height_raw
            .checked_sub(logical_bottom)
            .ok_or(TaggedPdfV2Error::AnnotationMismatch)?;
        let top = page
            .height_raw
            .checked_sub(link.y_raw())
            .ok_or(TaggedPdfV2Error::AnnotationMismatch)?;
        let value = format!(
            "<< /Type /Annot /Subtype /Link /P {page_object} 0 R /Rect [{} {} {} {}] /Border [0 0 0] /Dest {} /Contents <{}> /StructParent {} >>",
            pdf_number_v2(link.x_raw()),
            pdf_number_v2(bottom),
            pdf_number_v2(right),
            pdf_number_v2(top),
            pdf_literal_v2(link.destination().as_str()),
            utf16be_hex_v2(annotation.accessible_name())?,
            annotation.structure_parent_key(),
        );
        objects.insert(
            plan.annotation_object(annotation.annotation_id())?,
            format!("link_annotation:{}", annotation.annotation_id()),
            value.into_bytes(),
        )?;
    }
    Ok(())
}

fn emit_outlines_v2(
    objects: &mut TaggedObjectsV2,
    book: &BookNavigationSelectedReceiptV2,
    registry: &StructureRegistryReceiptV2,
    plan: &TaggedObjectPlanV2,
) -> Result<(), TaggedPdfV2Error> {
    let Some(root_object) = plan.outline_root_object else {
        return Ok(());
    };
    let entries = book.entries();
    let mut children = BTreeMap::<Option<u32>, Vec<u32>>::new();
    for (index, entry) in entries.iter().enumerate() {
        if usize::try_from(entry.outline_id()) != Ok(index) {
            return Err(TaggedPdfV2Error::OutlineMismatch);
        }
        children
            .entry(entry.parent_outline_id())
            .or_default()
            .push(entry.outline_id());
    }
    let top = children
        .get(&None)
        .filter(|values| !values.is_empty())
        .ok_or(TaggedPdfV2Error::OutlineMismatch)?;
    objects.insert(
        root_object,
        "outline_root",
        format!(
            "<< /Type /Outlines /First {} 0 R /Last {} 0 R /Count {} >>",
            plan.outline_object(*top.first().ok_or(TaggedPdfV2Error::OutlineMismatch)?)?,
            plan.outline_object(*top.last().ok_or(TaggedPdfV2Error::OutlineMismatch)?)?,
            entries.len(),
        )
        .into_bytes(),
    )?;
    for entry in entries {
        let siblings = children
            .get(&entry.parent_outline_id())
            .ok_or(TaggedPdfV2Error::OutlineMismatch)?;
        let position = siblings
            .iter()
            .position(|value| *value == entry.outline_id())
            .ok_or(TaggedPdfV2Error::OutlineMismatch)?;
        let parent = entry
            .parent_outline_id()
            .map(|id| plan.outline_object(id))
            .transpose()?
            .unwrap_or(root_object);
        let structure = registry
            .source_node(entry.source_node_id())
            .ok_or(TaggedPdfV2Error::OutlineMismatch)?;
        let mut value = format!(
            "<< /Title <{}> /Parent {} 0 R /Dest {} /SE {} 0 R",
            utf16be_hex_v2(entry.label())?,
            parent,
            pdf_literal_v2(entry.destination().anchor_id.as_str()),
            plan.structure_object(structure.structure_node_id())?,
        );
        if position != 0 {
            value.push_str(&format!(
                " /Prev {} 0 R",
                plan.outline_object(siblings[position - 1])?
            ));
        }
        if position + 1 < siblings.len() {
            value.push_str(&format!(
                " /Next {} 0 R",
                plan.outline_object(siblings[position + 1])?
            ));
        }
        if let Some(direct) = children.get(&Some(entry.outline_id())) {
            let descendants = outline_descendant_count_v2(entries, entry.outline_id())?;
            value.push_str(&format!(
                " /First {} 0 R /Last {} 0 R /Count {}",
                plan.outline_object(*direct.first().ok_or(TaggedPdfV2Error::OutlineMismatch)?)?,
                plan.outline_object(*direct.last().ok_or(TaggedPdfV2Error::OutlineMismatch)?)?,
                descendants,
            ));
        }
        value.push_str(" >>");
        objects.insert(
            plan.outline_object(entry.outline_id())?,
            format!("outline_item:{}", entry.outline_id()),
            value.into_bytes(),
        )?;
    }
    Ok(())
}

fn outline_descendant_count_v2(
    entries: &[typaxis_display_list::BookNavigationSelectedEntry],
    id: u32,
) -> Result<u32, TaggedPdfV2Error> {
    let start = usize::try_from(id).map_err(|_| TaggedPdfV2Error::OutlineMismatch)?;
    let level = entries
        .get(start)
        .ok_or(TaggedPdfV2Error::OutlineMismatch)?
        .level();
    let count = entries
        .iter()
        .skip(start + 1)
        .take_while(|entry| entry.level() > level)
        .count();
    usize_to_u32(count).map_err(|_| TaggedPdfV2Error::OutlineMismatch)
}

fn emit_structure_tree_v2(
    objects: &mut TaggedObjectsV2,
    registry: &StructureRegistryReceiptV2,
    marked: &MarkedContentPlanReceiptV2,
    plan: &TaggedObjectPlanV2,
) -> Result<(), TaggedPdfV2Error> {
    let roots = registry
        .nodes()
        .iter()
        .filter(|node| node.parent().is_none())
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return Err(TaggedPdfV2Error::StructureMismatch);
    }
    let mut root = format!(
        "<< /Type /StructTreeRoot /RoleMap << /Em /Span /Exercise /Div /Proof /Div /Result /Div /Strong /Span >> /ParentTree {} 0 R /ParentTreeNextKey {} /K [",
        plan.parent_tree_object,
        marked.parent_tree().len(),
    );
    for node in roots {
        root.push_str(&format!(
            "{} 0 R ",
            plan.structure_object(node.structure_node_id())?
        ));
    }
    root.push(']');
    if let Some(id_tree) = plan.id_tree_object {
        root.push_str(&format!(" /IDTree {id_tree} 0 R"));
    }
    root.push_str(" >>");
    objects.insert(
        plan.structure_tree_root_object,
        "structure_tree_root",
        root.into_bytes(),
    )?;

    let mut parent_tree = String::from("<< /Nums [");
    for entry in marked.parent_tree() {
        parent_tree.push_str(&entry.key().to_string());
        parent_tree.push(' ');
        match entry.value() {
            StructureParentTreeValue::Page(nodes) => {
                parent_tree.push('[');
                for node in nodes {
                    parent_tree.push_str(&format!("{} 0 R ", plan.structure_object(*node)?));
                }
                parent_tree.push(']');
            }
            StructureParentTreeValue::Annotation(node) => {
                parent_tree.push_str(&format!("{} 0 R", plan.structure_object(*node)?));
            }
        }
        parent_tree.push(' ');
    }
    parent_tree.push_str("] >>");
    objects.insert(
        plan.parent_tree_object,
        "structure_parent_tree",
        parent_tree.into_bytes(),
    )?;

    let mut ids = registry
        .nodes()
        .iter()
        .filter_map(|node| node.structure_id().map(|id| (id, node.structure_node_id())))
        .collect::<Vec<_>>();
    ids.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    if ids.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(TaggedPdfV2Error::StructureMismatch);
    }
    if let Some(id_tree_object) = plan.id_tree_object {
        let mut id_tree = String::from("<< /Names [");
        for (id, node) in ids {
            id_tree.push_str(&pdf_literal_v2(id));
            id_tree.push_str(&format!(" {} 0 R ", plan.structure_object(node)?));
        }
        id_tree.push_str("] >>");
        objects.insert(id_tree_object, "structure_id_tree", id_tree.into_bytes())?;
    } else if !ids.is_empty() {
        return Err(TaggedPdfV2Error::StructureMismatch);
    }

    for node in registry.nodes() {
        let mut value = format!("<< /Type /StructElem /S /{} /P ", node.role().pdf_name());
        if let Some(parent) = node.parent() {
            value.push_str(&format!("{} 0 R", plan.structure_object(parent)?));
        } else {
            value.push_str(&format!("{} 0 R", plan.structure_tree_root_object));
        }
        let parent_language = node
            .parent()
            .and_then(|parent| registry.node(parent))
            .map_or(registry.nodes()[0].language(), |parent| parent.language());
        if node.language() != parent_language {
            value.push_str(&format!(" /Lang <{}>", utf16be_hex_v2(node.language())?));
        }
        if let Some(alternative) = node.alternative() {
            value.push_str(&format!(" /Alt <{}>", utf16be_hex_v2(alternative)?));
        }
        if let Some(id) = node.structure_id() {
            value.push_str(&format!(" /ID {}", pdf_literal_v2(id)));
        }
        if let Some(numbering) = node.list_numbering() {
            value.push_str(&format!(
                " /A << /O /List /ListNumbering /{} >>",
                numbering.pdf_name()
            ));
        } else if let Some(table) = node.table_attributes() {
            value.push_str(" /A << /O /Table");
            if node.role() == StructureRole::TableHeader {
                value.push_str(" /Scope /Column");
            }
            if table.rowspan() > 1 {
                value.push_str(&format!(" /RowSpan {}", table.rowspan()));
            }
            if table.colspan() > 1 {
                value.push_str(&format!(" /ColSpan {}", table.colspan()));
            }
            if !table.header_ids().is_empty() {
                value.push_str(" /Headers [");
                for header in table.header_ids() {
                    value.push_str(&pdf_literal_v2(header));
                    value.push(' ');
                }
                value.push(']');
            }
            value.push_str(" >>");
        }
        let kids = structure_kids_v2(node.structure_node_id(), registry, marked, plan)?;
        if !kids.is_empty() {
            value.push_str(" /K [");
            for kid in kids {
                value.push_str(&kid);
                value.push(' ');
            }
            value.push(']');
        }
        value.push_str(" >>");
        objects.insert(
            plan.structure_object(node.structure_node_id())?,
            format!("structure_element:{}", node.structure_node_id().get()),
            value.into_bytes(),
        )?;
    }
    Ok(())
}

fn structure_kids_v2(
    owner: StructureNodeId,
    registry: &StructureRegistryReceiptV2,
    marked: &MarkedContentPlanReceiptV2,
    plan: &TaggedObjectPlanV2,
) -> Result<Vec<String>, TaggedPdfV2Error> {
    let node = registry
        .node(owner)
        .ok_or(TaggedPdfV2Error::StructureMismatch)?;
    let mut mcrs = marked
        .records()
        .iter()
        .filter_map(|record| match record.owner() {
            MarkedContentOwner::Structure(structure) if structure.structure_node_id() == owner => {
                Some((
                    record.semantic_fragment_ordinal(),
                    record.page_index(),
                    structure.mcid(),
                ))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    mcrs.sort_by_key(|(ordinal, _, _)| *ordinal);
    if mcrs
        .iter()
        .enumerate()
        .any(|(index, (ordinal, _, _))| u32::try_from(index) != Ok(*ordinal))
    {
        return Err(TaggedPdfV2Error::MarkedContentMismatch);
    }
    let mcr = |page_index: u32, mcid: u32| -> Result<String, TaggedPdfV2Error> {
        let page = plan
            .page_objects
            .get(page_index as usize)
            .ok_or(TaggedPdfV2Error::MarkedContentMismatch)?;
        Ok(format!("<< /Type /MCR /Pg {page} 0 R /MCID {mcid} >>"))
    };
    let mut kids = if let Some(order) = marked
        .formula_orders()
        .iter()
        .find(|order| order.formula_structure_node_id() == owner)
    {
        order
            .kids()
            .iter()
            .map(|kid| match kid {
                FormulaStructureKidV2::MarkedContentReference { page_index, mcid } => {
                    mcr(*page_index, *mcid)
                }
                FormulaStructureKidV2::StructureChild(child) => plan
                    .structure_object(*child)
                    .map(|object| format!("{object} 0 R")),
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        let child_objects = node
            .children()
            .iter()
            .map(|child| {
                plan.structure_object(*child)
                    .map(|object| format!("{object} 0 R"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mcr_objects = mcrs
            .iter()
            .map(|(_, page, mcid)| mcr(*page, *mcid))
            .collect::<Result<Vec<_>, _>>()?;
        if node.role() == StructureRole::Figure {
            mcr_objects.into_iter().chain(child_objects).collect()
        } else {
            child_objects.into_iter().chain(mcr_objects).collect()
        }
    };
    if node.paint_required() && mcrs.is_empty() {
        return Err(TaggedPdfV2Error::MarkedContentMismatch);
    }
    for annotation in marked
        .annotations()
        .iter()
        .filter(|annotation| annotation.structure_node_id() == owner)
    {
        let page = *plan
            .page_objects
            .get(annotation.page_index() as usize)
            .ok_or(TaggedPdfV2Error::AnnotationMismatch)?;
        let object = plan.annotation_object(annotation.annotation_id())?;
        kids.push(format!(
            "<< /Type /OBJR /Pg {page} 0 R /Obj {object} 0 R >>"
        ));
    }
    Ok(kids)
}

fn build_vector_final_writer_observation(
    vector: &StagingSafeVectorPdfContributionV2,
    plan: &TaggedObjectPlanV2,
) -> Result<StagingSafeVectorPdfFinalWriterObservationV2, TaggedPdfV2Error> {
    let object_table = vector
        .relative_objects()
        .iter()
        .map(|object| {
            Ok(
                StagingSafeVectorPdfFinalObjectObservationV2::from_final_writer(
                    object.relative_object_role(),
                    plan.vector_object(object.relative_object_role())?,
                    object.object_contribution_fingerprint(),
                ),
            )
        })
        .collect::<Result<Vec<_>, TaggedPdfV2Error>>()?;
    let usages = vector
        .usages()
        .iter()
        .map(|usage| {
            let page = *plan
                .page_objects
                .get(usage.page_index() as usize)
                .ok_or(TaggedPdfV2Error::VectorMismatch)?;
            let content = *plan
                .content_objects
                .get(usage.page_index() as usize)
                .ok_or(TaggedPdfV2Error::VectorMismatch)?;
            Ok(
                StagingSafeVectorPdfFinalUsageObservationV2::from_final_writer(
                    usage.usage_id(),
                    usage.page_index(),
                    usage.paint_ordinal(),
                    page,
                    content,
                    plan.vector_object(usage.form_relative_object_role())?,
                    usage.content_fingerprint(),
                ),
            )
        })
        .collect::<Result<Vec<_>, TaggedPdfV2Error>>()?;
    StagingSafeVectorPdfFinalWriterObservationV2::from_final_writer(vector, object_table, usages)
        .map_err(|_| TaggedPdfV2Error::VectorMismatch)
}

#[allow(clippy::too_many_arguments)]
fn build_book_final_writer_observation(
    navigation: &ValidatedStagingBookNavigationV2,
    book: &BookNavigationSelectedReceiptV2,
    engine: &EngineIdentity,
    objects: &BTreeMap<u32, (String, Vec<u8>)>,
    info_bytes: &[u8],
    xmp: &str,
    plan: &TaggedObjectPlanV2,
    final_pdf: &VerifiedPdfBytesReceipt,
) -> Result<BookNavigationPdfFinalWriterObservationV2, TaggedPdfV2Error> {
    let metadata = navigation.metadata().metadata();
    let info = BookNavigationPdfInfoObservationV2::from_final_writer(
        plan.info_object,
        info_bytes,
        format!("{} {}", engine.name(), engine.version()),
        metadata.title.clone(),
        metadata.author.clone(),
        metadata.subject.clone(),
        (!metadata.keywords.is_empty()).then(|| metadata.keywords.join("; ")),
        metadata.created.clone(),
        metadata.modified.clone(),
    )
    .map_err(|_| TaggedPdfV2Error::NavigationMismatch)?;
    let outlines = book
        .entries()
        .iter()
        .map(|entry| {
            let parent = entry
                .parent_outline_id()
                .map(|id| plan.outline_object(id))
                .transpose()?
                .or(plan.outline_root_object)
                .ok_or(TaggedPdfV2Error::OutlineMismatch)?;
            BookNavigationPdfOutlineObservationV2::from_final_writer(
                entry.outline_id(),
                plan.outline_object(entry.outline_id())?,
                parent,
                entry.label().to_owned(),
                entry.destination().anchor_id.as_str().to_owned(),
                entry.source_node_id().get(),
            )
            .map_err(|_| TaggedPdfV2Error::OutlineMismatch)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut paints = book
        .language_paints()
        .iter()
        .map(|paint| {
            (
                paint.page_index(),
                paint.paint_ordinal(),
                BookNavigationPdfLanguagePaintSourceV2::LogicalOwnerOccurrence(paint.occurrence()),
                paint.owner_node_id().get(),
                paint.language().to_owned(),
                paint.language_record_fingerprint(),
            )
        })
        .chain(book.vector_paints_requiring_language().map(|paint| {
            (
                paint.page_index(),
                paint.paint_ordinal(),
                BookNavigationPdfLanguagePaintSourceV2::VectorUsage(paint.usage_id()),
                paint.owner_node_id().get(),
                paint.language().to_owned(),
                paint.language_record_fingerprint(),
            )
        }))
        .collect::<Vec<_>>();
    paints.sort_by_key(|paint| (paint.0, paint.1));
    let language_paints = paints
        .into_iter()
        .map(|paint| {
            BookNavigationPdfLanguagePaintObservationV2::from_final_writer(
                paint.2,
                paint.3,
                paint.0,
                paint.1,
                *plan
                    .content_objects
                    .get(paint.0 as usize)
                    .ok_or(TaggedPdfV2Error::NavigationMismatch)?,
                paint.4,
                paint.5,
            )
            .map_err(|_| TaggedPdfV2Error::NavigationMismatch)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let xmp = BookXmpObservationV2::from_final_writer(xmp.as_bytes())
        .map_err(|_| TaggedPdfV2Error::NavigationMismatch)?;
    let catalog = objects.get(&1).ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
    BookNavigationPdfFinalWriterObservationV2::from_final_writer(
        final_pdf.content_hash(),
        final_pdf.byte_length(),
        final_pdf.page_count(),
        final_pdf.object_count(),
        1,
        &catalog.1,
        navigation.languages().document_language().to_owned(),
        plan.metadata_object,
        plan.outline_root_object,
        book.destination_registry_sha256(),
        info,
        outlines,
        language_paints,
        xmp,
    )
    .map_err(|_| TaggedPdfV2Error::NavigationMismatch)
}

#[allow(clippy::too_many_arguments)]
fn build_tagged_observation_v2(
    profile: &StagingAccessibilityProfileAuthorizationV2,
    registry: &StructureRegistryReceiptV2,
    vector_plan: &VectorMarkedContentPlanV2,
    book_navigation: &BookNavigationPdfObservationV2,
    vector: &StagingSafeVectorPdfContributionV2,
    safe_vector: &StagingSafeVectorPdfClosureV2,
    xmp: &str,
    objects: Vec<TaggedPdfObjectObservationV2>,
    plan: &TaggedObjectPlanV2,
    final_pdf: &VerifiedPdfBytesReceipt,
) -> Result<TaggedPdfObservationV2, TaggedPdfV2Error> {
    let marked = vector_plan.marked_content();
    let document_language = registry
        .nodes()
        .first()
        .ok_or(TaggedPdfV2Error::StructureMismatch)?
        .language();
    if book_navigation.final_pdf_sha256() != final_pdf.content_hash()
        || book_navigation.document_language() != document_language
        || book_navigation.xmp_sha256() != sha256(xmp.as_bytes())
        || safe_vector.contribution_fingerprint() != vector.fingerprint()
    {
        return Err(TaggedPdfV2Error::ReceiptMismatch);
    }
    let marked_content_count = marked.pages().iter().try_fold(0u32, |sum, page| {
        sum.checked_add(page.marked_content_count())
    });
    let vector_usage_count = usize_to_u32(vector.usages().len())?;
    let equation_number_count = usize_to_u32(
        marked
            .records()
            .iter()
            .filter(|record| {
                matches!(
                    record.binding(),
                    MarkedContentBindingKindV2::EquationNumber { .. }
                )
            })
            .count(),
    )?;
    let form_object_count = usize_to_u32(
        objects
            .iter()
            .filter(|object| object.role.starts_with("vector_form:"))
            .count(),
    )?;
    let mut value = TaggedPdfObservationV2 {
        profile_sha256: profile.profile_receipt_fingerprint(),
        structure_registry_sha256: registry.fingerprint(),
        selected_binding_sha256: vector_plan.selected_binding().fingerprint(),
        marked_content_sha256: marked.fingerprint(),
        book_navigation_sha256: book_navigation.fingerprint(),
        safe_vector_pdf_sha256: safe_vector.fingerprint(),
        document_language: document_language.to_owned(),
        catalog_object: 1,
        structure_tree_root_object: plan.structure_tree_root_object,
        parent_tree_object: plan.parent_tree_object,
        id_tree_object: plan.id_tree_object,
        equation_font_count: usize_to_u32(plan.equation_fonts.len())?,
        structure_element_count: usize_to_u32(registry.nodes().len())?,
        marked_content_count: marked_content_count.ok_or(TaggedPdfV2Error::ObjectLimit)?,
        vector_usage_count,
        equation_number_count,
        form_object_count,
        object_count: plan.object_count,
        object_budget_charge_count: 1,
        xmp_sha256: sha256(xmp.as_bytes()),
        objects,
        pdf_sha256: final_pdf.content_hash(),
        pdf_byte_length: final_pdf.byte_length(),
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    value.canonical_jcs = encode_observation_v2(&value);
    value.fingerprint = sha256(value.canonical_jcs.as_bytes());
    Ok(value)
}

fn encode_info_v2(
    navigation: &ValidatedStagingBookNavigationV2,
    engine: &EngineIdentity,
) -> Result<String, TaggedPdfV2Error> {
    let metadata = navigation.metadata().metadata();
    let mut fields = Vec::new();
    if let Some(author) = &metadata.author {
        fields.push(format!("/Author <{}>", utf16be_hex_v2(author)?));
    }
    if let Some(created) = &metadata.created {
        fields.push(format!("/CreationDate ({})", pdf_date_v2(created)?));
    }
    if !metadata.keywords.is_empty() {
        fields.push(format!(
            "/Keywords <{}>",
            utf16be_hex_v2(&metadata.keywords.join("; "))?
        ));
    }
    if let Some(modified) = &metadata.modified {
        fields.push(format!("/ModDate ({})", pdf_date_v2(modified)?));
    }
    fields.push(format!(
        "/Producer <{}>",
        utf16be_hex_v2(&format!("{} {}", engine.name(), engine.version()))?
    ));
    if let Some(subject) = &metadata.subject {
        fields.push(format!("/Subject <{}>", utf16be_hex_v2(subject)?));
    }
    if let Some(title) = &metadata.title {
        fields.push(format!("/Title <{}>", utf16be_hex_v2(title)?));
    }
    Ok(format!("<< {} >>", fields.join(" ")))
}

fn pdf_date_v2(value: &str) -> Result<String, TaggedPdfV2Error> {
    if value.len() != 20 {
        return Err(TaggedPdfV2Error::NavigationMismatch);
    }
    Ok(format!(
        "D:{}{}{}{}{}{}Z",
        &value[0..4],
        &value[5..7],
        &value[8..10],
        &value[11..13],
        &value[14..16],
        &value[17..19]
    ))
}

fn push_pdf_view_v2(
    output: &mut String,
    view: &DestinationView,
    page_height_raw: i64,
) -> Result<(), TaggedPdfV2Error> {
    match view {
        DestinationView::Xyz { point } => {
            let y = page_height_raw
                .checked_sub(point.y.raw())
                .ok_or(TaggedPdfV2Error::NavigationMismatch)?;
            output.push_str(&format!(
                "/XYZ {} {} null",
                pdf_number_v2(point.x.raw()),
                pdf_number_v2(y)
            ));
        }
        DestinationView::FitPage => output.push_str("/Fit"),
        DestinationView::FitWidth { top } => {
            let top = top
                .as_ref()
                .map(|value| {
                    page_height_raw
                        .checked_sub(value.raw())
                        .map(pdf_number_v2)
                        .ok_or(TaggedPdfV2Error::NavigationMismatch)
                })
                .transpose()?
                .unwrap_or_else(|| "null".to_owned());
            output.push_str(&format!("/FitH {top}"));
        }
    }
    Ok(())
}

fn stream_object_v2(prefix: &[u8], content: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(b"<< ");
    output.extend_from_slice(prefix);
    output.extend_from_slice(format!("/Length {} >>\nstream\n", content.len()).as_bytes());
    output.extend_from_slice(content);
    output.extend_from_slice(b"\nendstream");
    output
}

fn utf16be_hex_v2(value: &str) -> Result<String, TaggedPdfV2Error> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::from("FEFF");
    output
        .try_reserve(
            value
                .encode_utf16()
                .count()
                .checked_mul(4)
                .ok_or(TaggedPdfV2Error::OutputLimit)?,
        )
        .map_err(|_| TaggedPdfV2Error::AllocationFailure)?;
    for unit in value.encode_utf16() {
        output.push(char::from(HEX[usize::from((unit >> 12) & 0x0f)]));
        output.push(char::from(HEX[usize::from((unit >> 8) & 0x0f)]));
        output.push(char::from(HEX[usize::from((unit >> 4) & 0x0f)]));
        output.push(char::from(HEX[usize::from(unit & 0x0f)]));
    }
    Ok(output)
}

fn pdf_literal_v2(value: &str) -> String {
    let mut output = String::from("(");
    for byte in value.bytes() {
        match byte {
            b'(' | b')' | b'\\' => {
                output.push('\\');
                output.push(char::from(byte));
            }
            _ => output.push(char::from(byte)),
        }
    }
    output.push(')');
    output
}

fn pdf_number_v2(raw: i64) -> String {
    const SCALE: u64 = 65_536;
    const BINARY_TO_DECIMAL: u64 = 152_587_890_625;
    let negative = raw < 0;
    let magnitude = raw.unsigned_abs();
    let whole = magnitude / SCALE;
    let remainder = magnitude % SCALE;
    let mut output = if remainder == 0 {
        whole.to_string()
    } else {
        let mut fraction = format!("{:016}", remainder * BINARY_TO_DECIMAL);
        while fraction.ends_with('0') {
            fraction.pop();
        }
        format!("{whole}.{fraction}")
    };
    if negative && magnitude != 0 {
        output.insert(0, '-');
    }
    output
}

fn serialize_pdf_v2(
    objects: &BTreeMap<u32, (String, Vec<u8>)>,
    object_count: u32,
    info_object: u32,
    maximum: u64,
) -> Result<Vec<u8>, TaggedPdfV2Error> {
    let xref_count = object_count
        .checked_add(1)
        .ok_or(TaggedPdfV2Error::ObjectLimit)?;
    let mut output = Vec::new();
    extend_bounded_v2(&mut output, b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n", maximum)?;
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(xref_count as usize)
        .map_err(|_| TaggedPdfV2Error::AllocationFailure)?;
    offsets.push(0usize);
    for number in 1..=object_count {
        offsets.push(output.len());
        extend_bounded_v2(&mut output, format!("{number} 0 obj\n").as_bytes(), maximum)?;
        let value = objects
            .get(&number)
            .ok_or(TaggedPdfV2Error::ReceiptMismatch)?;
        extend_bounded_v2(&mut output, &value.1, maximum)?;
        extend_bounded_v2(&mut output, b"\nendobj\n", maximum)?;
    }
    let xref = output.len();
    extend_bounded_v2(
        &mut output,
        format!("xref\n0 {xref_count}\n").as_bytes(),
        maximum,
    )?;
    extend_bounded_v2(&mut output, b"0000000000 65535 f \n", maximum)?;
    for offset in offsets.into_iter().skip(1) {
        extend_bounded_v2(
            &mut output,
            format!("{offset:010} 00000 n \n").as_bytes(),
            maximum,
        )?;
    }
    extend_bounded_v2(
        &mut output,
        format!(
            "trailer\n<< /Size {xref_count} /Root 1 0 R /Info {info_object} 0 R >>\nstartxref\n{xref}\n%%EOF\n"
        )
        .as_bytes(),
        maximum,
    )?;
    Ok(output)
}

fn extend_bounded_v2(
    output: &mut Vec<u8>,
    value: &[u8],
    maximum: u64,
) -> Result<(), TaggedPdfV2Error> {
    let next = output
        .len()
        .checked_add(value.len())
        .ok_or(TaggedPdfV2Error::OutputLimit)?;
    if next as u64 > maximum {
        return Err(TaggedPdfV2Error::OutputLimit);
    }
    output
        .try_reserve_exact(value.len())
        .map_err(|_| TaggedPdfV2Error::AllocationFailure)?;
    output.extend_from_slice(value);
    Ok(())
}

fn encode_observation_v2(value: &TaggedPdfObservationV2) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, TAGGED_PDF_OBSERVATION_ALGORITHM_V2);
    output.push_str(",\"book_navigation_sha256\":");
    push_hash_v2(&mut output, value.book_navigation_sha256);
    output.push_str(",\"catalog_object\":");
    output.push_str(&value.catalog_object.to_string());
    output.push_str(",\"document_language\":");
    push_jcs_string(&mut output, &value.document_language);
    output.push_str(",\"equation_font_count\":");
    output.push_str(&value.equation_font_count.to_string());
    output.push_str(",\"equation_number_count\":");
    output.push_str(&value.equation_number_count.to_string());
    output.push_str(",\"form_object_count\":");
    output.push_str(&value.form_object_count.to_string());
    output.push_str(",\"id_tree_object\":");
    push_optional_u32_v2(&mut output, value.id_tree_object);
    output.push_str(",\"marked_content_count\":");
    output.push_str(&value.marked_content_count.to_string());
    output.push_str(",\"marked_content_sha256\":");
    push_hash_v2(&mut output, value.marked_content_sha256);
    output.push_str(",\"object_budget_charge_count\":");
    output.push_str(&value.object_budget_charge_count.to_string());
    output.push_str(",\"object_count\":");
    output.push_str(&value.object_count.to_string());
    output.push_str(",\"objects\":[");
    for (index, object) in value.objects.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"object_number\":");
        output.push_str(&object.object_number.to_string());
        output.push_str(",\"role\":");
        push_jcs_string(&mut output, &object.role);
        output.push_str(",\"sha256\":");
        push_hash_v2(&mut output, object.sha256);
        output.push('}');
    }
    output.push_str("],\"parent_tree_object\":");
    output.push_str(&value.parent_tree_object.to_string());
    output.push_str(",\"pdf_byte_length\":");
    output.push_str(&value.pdf_byte_length.to_string());
    output.push_str(",\"pdf_sha256\":");
    push_hash_v2(&mut output, value.pdf_sha256);
    output.push_str(",\"profile_sha256\":");
    push_hash_v2(&mut output, value.profile_sha256);
    output.push_str(",\"safe_vector_pdf_sha256\":");
    push_hash_v2(&mut output, value.safe_vector_pdf_sha256);
    output.push_str(",\"selected_binding_sha256\":");
    push_hash_v2(&mut output, value.selected_binding_sha256);
    output.push_str(",\"structure_element_count\":");
    output.push_str(&value.structure_element_count.to_string());
    output.push_str(",\"structure_registry_sha256\":");
    push_hash_v2(&mut output, value.structure_registry_sha256);
    output.push_str(",\"structure_tree_root_object\":");
    output.push_str(&value.structure_tree_root_object.to_string());
    output.push_str(",\"validator_algorithm\":");
    push_jcs_string(&mut output, TAGGED_PDF_VALIDATOR_ALGORITHM_V2);
    output.push_str(",\"vector_usage_count\":");
    output.push_str(&value.vector_usage_count.to_string());
    output.push_str(",\"xmp_sha256\":");
    push_hash_v2(&mut output, value.xmp_sha256);
    output.push('}');
    output
}

fn take_object(next: &mut u32) -> Result<u32, TaggedPdfV2Error> {
    let value = *next;
    *next = next.checked_add(1).ok_or(TaggedPdfV2Error::ObjectLimit)?;
    Ok(value)
}

fn checked_add(left: u32, right: u32) -> Result<u32, TaggedPdfV2Error> {
    left.checked_add(right).ok_or(TaggedPdfV2Error::ObjectLimit)
}

fn checked_mul(left: u32, right: u32) -> Result<u32, TaggedPdfV2Error> {
    left.checked_mul(right).ok_or(TaggedPdfV2Error::ObjectLimit)
}

fn usize_to_u32(value: usize) -> Result<u32, TaggedPdfV2Error> {
    u32::try_from(value).map_err(|_| TaggedPdfV2Error::ObjectLimit)
}

fn contains_bytes(value: &[u8], needle: &[u8]) -> bool {
    value.windows(needle.len()).any(|window| window == needle)
}

fn push_optional_u32_v2(output: &mut String, value: Option<u32>) {
    match value {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push_str("null"),
    }
}

fn push_hash_v2(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(any(test, feature = "staging-fixtures"))]
fn tagged_fixture_destinations_v2(
    navigation: &ValidatedStagingBookNavigationV2,
) -> Result<Vec<typaxis_display_list::BookNavigationDestinationBinding>, TaggedPdfV2Error> {
    let mut destinations = Vec::new();
    destinations
        .try_reserve_exact(navigation.anchors().len())
        .map_err(|_| TaggedPdfV2Error::AllocationFailure)?;
    for (index, (anchor, source_node_id)) in navigation.anchors().iter().enumerate() {
        destinations.push(typaxis_display_list::BookNavigationDestinationBinding {
            source_node_id: *source_node_id,
            frame_id: u32::try_from(index).map_err(|_| TaggedPdfV2Error::NavigationMismatch)?,
            destination: typaxis_display_list::NamedDestination {
                anchor_id: anchor.clone(),
                page_index: 0,
                view: DestinationView::Xyz {
                    point: Point {
                        x: Length::ZERO,
                        y: Length::ZERO,
                    },
                },
            },
        });
    }
    if destinations.is_empty() {
        return Err(TaggedPdfV2Error::NavigationMismatch);
    }
    Ok(destinations)
}

#[cfg(any(test, feature = "staging-fixtures"))]
fn tagged_fixture_authorizations_v2(
    fixture: &typaxis_display_list::StagingPrecomposedVectorDisplayFixture,
    navigation: &ValidatedStagingBookNavigationV2,
    semantics: &ValidatedStagingStructureSemanticsV2,
) -> Result<
    (
        StagingAccessibilityProfileAuthorizationV2,
        StagingBookNavigationProfileAuthorizationV2,
    ),
    Box<dyn std::error::Error>,
> {
    use typaxis_syntax::{StagingAccessibilityProfileViewV2, StagingBookNavigationProfileViewV2};
    let package = &fixture.layout.package;
    let limits = &fixture.layout.limits;
    let book_profile = StagingBookNavigationProfileAuthorizationV2::bind_profile_receipt(
        StagingBookNavigationProfileViewV2::new(package, navigation, limits)?,
        sha256(b"tagged-vector-navigation-profile-v2"),
        fixture.layout.profile.profile_receipt_fingerprint(),
        fixture.layout.profile.profile_fingerprint(),
        package,
        navigation,
        limits,
    )?;
    let profile = StagingAccessibilityProfileAuthorizationV2::bind_profile_receipt(
        StagingAccessibilityProfileViewV2::new(package, navigation, semantics, limits)?,
        sha256(b"tagged-vector-accessibility-profile-v2"),
        book_profile.profile_receipt_fingerprint(),
        package,
        navigation,
        semantics,
        limits,
    )?;
    Ok((profile, book_profile))
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_tagged_vector_pdf_v2_fixture(
) -> Result<StagingTaggedPdfV2, Box<dyn std::error::Error>> {
    use typaxis_display_list::{
        build_vector_marked_content_plan_v2, prove_vector_form_structure_isolation_v2,
        select_staging_book_navigation_v2, staging_precomposed_vector_tagged_pdf_fixture,
        BookNavigationSelectedPage,
    };
    use typaxis_resources::{
        finalize_staging_safe_vector_forms_v2, VectorContentCandidateRegistry,
    };
    use typaxis_syntax::{
        validate_staging_book_navigation_v2, validate_staging_structure_semantics_v2,
    };

    const SCALE: i64 = 65_536;
    let display_fixture = staging_precomposed_vector_tagged_pdf_fixture()?;
    let package = &display_fixture.layout.package;
    let limits = &display_fixture.layout.limits;
    let navigation = validate_staging_book_navigation_v2(package, limits)?;
    let semantics = validate_staging_structure_semantics_v2(package, &navigation, limits)?;
    let (profile, book_profile) =
        tagged_fixture_authorizations_v2(&display_fixture, &navigation, &semantics)?;
    let pages = display_fixture
        .display
        .pages()
        .iter()
        .map(|page| BookNavigationSelectedPage {
            page_index: page.page_index(),
            width_raw: 1_000 * SCALE,
            height_raw: 800 * SCALE,
        })
        .collect::<Vec<_>>();
    let destinations = tagged_fixture_destinations_v2(&navigation)?;
    let book = select_staging_book_navigation_v2(
        &navigation,
        &book_profile,
        limits,
        sha256(b"tagged-vector-complete-layout-v2"),
        4,
        &pages,
        &destinations,
        &[],
        &[],
        &display_fixture.display,
    )?;
    let registry = typaxis_display_list::build_structure_registry_v2(
        package,
        &navigation,
        &semantics,
        &profile,
        limits,
    )?;
    let form_isolation = prove_vector_form_structure_isolation_v2(&display_fixture.display)?;
    let vector_plan = build_vector_marked_content_plan_v2(
        &registry,
        &profile,
        limits,
        &navigation,
        &book_profile,
        &book,
        &[],
        &[],
        &display_fixture.display,
        &form_isolation,
        &display_fixture.block_selected,
        &display_fixture.layout.math_flows,
    )?;
    let serialization = vector_plan.authorize_pdf_serialization(
        &registry,
        &profile,
        limits,
        &navigation,
        &book_profile,
        &book,
        &display_fixture.display,
        &form_isolation,
        &display_fixture.block_selected,
        &display_fixture.layout.math_flows,
    )?;
    let candidates = VectorContentCandidateRegistry::from_admitted(
        &display_fixture.layout.admitted,
        package.resources(),
    )?;
    let form_plans =
        finalize_staging_safe_vector_forms_v2(&display_fixture.display, &candidates, limits)?;
    let vector = crate::build_staging_safe_vector_pdf_contribution_v2(
        &display_fixture.display,
        &form_plans,
        &candidates,
        limits,
    )?;
    Ok(write_staging_tagged_pdf_v2(
        package,
        &navigation,
        &semantics,
        &profile,
        &book_profile,
        &book,
        &registry,
        serialization,
        &display_fixture.display,
        &form_isolation,
        &display_fixture.layout.admitted,
        &form_plans,
        &candidates,
        &vector,
        limits,
        &EngineIdentity::compiled(),
        EffectiveConfigFingerprint::from_untrusted_bytes(sha256(
            b"tagged-vector-effective-config-v2",
        )),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{M4ResourceLimits, ResourceLimits, ValidatedResourceLimits};
    use typaxis_display_list::{
        build_vector_marked_content_plan_v2, prove_vector_form_structure_isolation_v2,
        select_staging_book_navigation_v2, staging_combined_vector_figure_fixture,
        staging_precomposed_vector_tagged_pdf_fixture, BookNavigationSelectedPage,
        MarkedContentStandardPaintInputV2, SelectedStructurePaintOwner,
    };
    use typaxis_resources::{
        finalize_staging_combined_safe_vector_forms_v2, finalize_staging_safe_vector_forms_v2,
        VectorContentCandidateRegistry,
    };
    use typaxis_syntax::{
        validate_staging_book_navigation_v2, validate_staging_structure_semantics_v2,
    };

    const SCALE: i64 = 65_536;

    struct FixtureRefs<'a> {
        package: &'a ValidatedStagingSemanticPackage,
        navigation: &'a ValidatedStagingBookNavigationV2,
        semantics: &'a ValidatedStagingStructureSemanticsV2,
        profile: &'a StagingAccessibilityProfileAuthorizationV2,
        book_profile: &'a StagingBookNavigationProfileAuthorizationV2,
        book: &'a BookNavigationSelectedReceiptV2,
        registry: &'a StructureRegistryReceiptV2,
        vector_plan: &'a VectorMarkedContentPlanV2,
        display: &'a StagingPrecomposedVectorDisplay,
        form_isolation: &'a VectorFormStructureIsolationReceiptV2,
        serialization: VectorMarkedContentSerializationV2<'a>,
        admitted: &'a AdmittedResourceLedger,
        form_plans: &'a StagingSafeVectorFormPlansV2,
        candidates: &'a VectorContentCandidateRegistry,
        vector: &'a StagingSafeVectorPdfContributionV2,
        limits: &'a M4EffectiveResourceLimits,
        engine: &'a EngineIdentity,
        config_fingerprint: EffectiveConfigFingerprint,
    }

    impl FixtureRefs<'_> {
        fn equation_fonts(&self) -> Vec<FrozenStagingPdfTextFontPlan> {
            let usages =
                equation_text_usages_v2(self.serialization.equation_number_shapes(), self.admitted)
                    .unwrap();
            finalize_staging_pdf_text_fonts(self.admitted, &usages, self.limits.base()).unwrap()
        }

        fn write(&self) -> Result<StagingTaggedPdfV2, TaggedPdfV2Error> {
            write_staging_tagged_pdf_v2(
                self.package,
                self.navigation,
                self.semantics,
                self.profile,
                self.book_profile,
                self.book,
                self.registry,
                self.serialization,
                self.display,
                self.form_isolation,
                self.admitted,
                self.form_plans,
                self.candidates,
                self.vector,
                self.limits,
                self.engine,
                self.config_fingerprint,
            )
        }
    }

    fn with_fixture<T>(callback: impl FnOnce(FixtureRefs<'_>) -> T) -> T {
        let display_fixture = staging_precomposed_vector_tagged_pdf_fixture().unwrap();
        let package = &display_fixture.layout.package;
        let limits = &display_fixture.layout.limits;
        let navigation = validate_staging_book_navigation_v2(package, limits).unwrap();
        let semantics =
            validate_staging_structure_semantics_v2(package, &navigation, limits).unwrap();
        let (profile, book_profile) =
            tagged_fixture_authorizations_v2(&display_fixture, &navigation, &semantics).unwrap();
        let pages = display_fixture
            .display
            .pages()
            .iter()
            .map(|page| BookNavigationSelectedPage {
                page_index: page.page_index(),
                width_raw: 1_000 * SCALE,
                height_raw: 800 * SCALE,
            })
            .collect::<Vec<_>>();
        let selected_layout_sha256 = sha256(b"tagged-vector-complete-layout-v2");
        let destinations = tagged_fixture_destinations_v2(&navigation).unwrap();
        let book = select_staging_book_navigation_v2(
            &navigation,
            &book_profile,
            limits,
            selected_layout_sha256,
            4,
            &pages,
            &destinations,
            &[],
            &[],
            &display_fixture.display,
        )
        .unwrap();
        let registry = typaxis_display_list::build_structure_registry_v2(
            package,
            &navigation,
            &semantics,
            &profile,
            limits,
        )
        .unwrap();
        let form_isolation =
            prove_vector_form_structure_isolation_v2(&display_fixture.display).unwrap();
        let vector_plan = build_vector_marked_content_plan_v2(
            &registry,
            &profile,
            limits,
            &navigation,
            &book_profile,
            &book,
            &[],
            &[],
            &display_fixture.display,
            &form_isolation,
            &display_fixture.block_selected,
            &display_fixture.layout.math_flows,
        )
        .unwrap();
        let serialization = vector_plan
            .authorize_pdf_serialization(
                &registry,
                &profile,
                limits,
                &navigation,
                &book_profile,
                &book,
                &display_fixture.display,
                &form_isolation,
                &display_fixture.block_selected,
                &display_fixture.layout.math_flows,
            )
            .unwrap();
        let candidates = VectorContentCandidateRegistry::from_admitted(
            &display_fixture.layout.admitted,
            package.resources(),
        )
        .unwrap();
        let form_plans =
            finalize_staging_safe_vector_forms_v2(&display_fixture.display, &candidates, limits)
                .unwrap();
        let vector = crate::build_staging_safe_vector_pdf_contribution_v2(
            &display_fixture.display,
            &form_plans,
            &candidates,
            limits,
        )
        .unwrap();
        let engine = EngineIdentity::compiled();
        callback(FixtureRefs {
            package,
            navigation: &navigation,
            semantics: &semantics,
            profile: &profile,
            book_profile: &book_profile,
            book: &book,
            registry: &registry,
            vector_plan: &vector_plan,
            display: &display_fixture.display,
            form_isolation: &form_isolation,
            serialization,
            admitted: &display_fixture.layout.admitted,
            form_plans: &form_plans,
            candidates: &candidates,
            vector: &vector,
            limits,
            engine: &engine,
            config_fingerprint: EffectiveConfigFingerprint::from_untrusted_bytes(sha256(
                b"tagged-vector-effective-config-v2",
            )),
        })
    }

    fn byte_occurrences(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .filter(|window| *window == needle)
            .count()
    }

    fn decode_utf16be_hex(hex: &[u8]) -> String {
        let hex = std::str::from_utf8(hex).unwrap();
        let units = hex
            .as_bytes()
            .chunks_exact(4)
            .map(|chunk| u16::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        String::from_utf16(&units).unwrap()
    }

    fn extract_accessible_text(pdf: &[u8]) -> Vec<String> {
        let mut positioned = Vec::new();
        for marker in [b"/ActualText <FEFF".as_slice(), b"<FEFF"] {
            let mut offset = 0usize;
            while let Some(relative) = pdf[offset..]
                .windows(marker.len())
                .position(|window| window == marker)
            {
                let marker_start = offset + relative;
                let start = marker_start + marker.len();
                let end = pdf[start..]
                    .iter()
                    .position(|byte| *byte == b'>')
                    .map(|relative| start + relative)
                    .unwrap();
                let is_actual_text = marker.starts_with(b"/ActualText");
                let is_text_show = pdf.get(end..end + 5) == Some(b"> Tj ");
                if is_actual_text || is_text_show {
                    positioned.push((marker_start, decode_utf16be_hex(&pdf[start..end])));
                }
                offset = end + 1;
            }
        }
        let marker = b" Tm (";
        let mut offset = 0usize;
        while let Some(relative) = pdf[offset..]
            .windows(marker.len())
            .position(|window| window == marker)
        {
            let marker_start = offset + relative;
            let start = marker_start + marker.len();
            let mut end = start;
            let mut escaped = false;
            while let Some(byte) = pdf.get(end).copied() {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b')' {
                    break;
                }
                end += 1;
            }
            assert_eq!(pdf.get(end..end + 5), Some(b") Tj ".as_slice()));
            let mut decoded = Vec::new();
            let mut index = start;
            while index < end {
                if pdf[index] == b'\\' {
                    index += 1;
                }
                decoded.push(pdf[index]);
                index += 1;
            }
            positioned.push((marker_start, String::from_utf8(decoded).unwrap()));
            offset = end + 1;
        }
        positioned.sort_by_key(|(position, _)| *position);
        positioned.dedup_by_key(|(position, _)| *position);
        positioned
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>()
    }

    #[test]
    fn tagged_vector_pdf_v2_serializes_page_mcr_inner_span_and_shared_form() {
        with_fixture(|fixture| {
            let first = fixture.write().unwrap();
            let second = fixture.write().unwrap();
            assert_eq!(first.bytes(), second.bytes());
            assert_eq!(
                first.observation().canonical_jcs(),
                second.observation().canonical_jcs()
            );
            assert_eq!(first.observation().object_budget_charge_count(), 1);
            assert_eq!(first.observation().vector_usage_count(), 4);
            assert_eq!(first.observation().form_object_count(), 1);
            assert_eq!(first.observation().equation_number_count(), 1);
            assert_eq!(first.observation().equation_font_count(), 1);
            // Navigation anchors do not create structure IDs. IDTree belongs
            // to generated table-header / footnote structure identities.
            assert_eq!(first.observation().id_tree_object(), None);
            for role in [
                "outline_root",
                "outline_item:0",
                "equation_font_type0:0",
                "equation_font_cid:0",
                "equation_font_descriptor:0",
                "equation_font_program:0",
                "equation_font_to_unicode:0",
                "equation_font_cid_to_gid:0",
            ] {
                assert!(first
                    .observation()
                    .objects()
                    .iter()
                    .any(|object| object.role() == role));
            }
            assert_eq!(byte_occurrences(first.bytes(), b"/Subtype /Form"), 1);
            assert_eq!(byte_occurrences(first.bytes(), b" Do\n"), 4);
            assert_eq!(byte_occurrences(first.bytes(), b"/MCID "), 10);
            assert_eq!(byte_occurrences(first.bytes(), b"/S /Formula"), 2);
            assert_eq!(byte_occurrences(first.bytes(), b"/S /Figure"), 2);
            assert!(contains_bytes(
                first.bytes(),
                b"/Formula << /MCID 1 >> BDC\n/Span << /ActualText"
            ));
            assert!(contains_bytes(first.bytes(), b"BT /F0"));
            assert!(contains_bytes(first.bytes(), b"/Subtype /Type0"));
            assert!(contains_bytes(first.bytes(), b"/CIDToGIDMap "));
            assert!(contains_bytes(first.bytes(), b"/ToUnicode "));
            assert!(!contains_bytes(first.bytes(), b"/Helvetica"));
            assert!(contains_bytes(first.bytes(), b"/Outlines "));
            assert!(!contains_bytes(first.bytes(), b"/IDTree "));
            assert!(contains_bytes(first.bytes(), b"/SE "));

            let form_start = first
                .bytes()
                .windows(b"/Subtype /Form".len())
                .position(|window| window == b"/Subtype /Form")
                .unwrap();
            let stream_start = form_start
                + first.bytes()[form_start..]
                    .windows(b"stream\n".len())
                    .position(|window| window == b"stream\n")
                    .unwrap()
                + b"stream\n".len();
            let stream_end = stream_start
                + first.bytes()[stream_start..]
                    .windows(b"\nendstream".len())
                    .position(|window| window == b"\nendstream")
                    .unwrap();
            let form_stream = &first.bytes()[stream_start..stream_end];
            for forbidden in [
                b"/MCID".as_slice(),
                b"/Alt",
                b"/ActualText",
                b"/Lang",
                b"BDC",
            ] {
                assert!(!contains_bytes(form_stream, forbidden));
            }
        });
    }

    #[test]
    fn combined_figure_reaches_final_tagged_pdf_as_accessible_vector_form() {
        let fixture = staging_combined_vector_figure_fixture().unwrap();
        let package = &fixture.figure.layout.package;
        let limits = &fixture.figure.layout.limits;
        let page_geometry = fixture.figure.display.page_geometry();
        let pages = (0..u32::try_from(fixture.figure.display.pages().len()).unwrap())
            .map(|page_index| BookNavigationSelectedPage {
                page_index,
                width_raw: page_geometry.page_width().get().raw(),
                height_raw: page_geometry.page_height().get().raw(),
            })
            .collect::<Vec<_>>();
        let book = select_staging_book_navigation_v2(
            &fixture.navigation,
            &fixture.book_profile,
            limits,
            fixture
                .figure
                .display
                .receipt()
                .selected_layout_fingerprint(),
            u64::from(fixture.figure.display.receipt().command_count()),
            &pages,
            &[],
            &[],
            &[],
            &fixture.precomposed,
        )
        .unwrap();
        let standard_paints = fixture
            .figure
            .display
            .commands()
            .map(|command| {
                let node = fixture.registry.source_node(command.owner()).unwrap();
                MarkedContentStandardPaintInputV2 {
                    page_index: command.page_index(),
                    paint_ordinal: command.occurrence(),
                    semantic_fragment_ordinal: 0,
                    owner: SelectedStructurePaintOwner::Structure(node.structure_node_id()),
                }
            })
            .collect::<Vec<_>>();
        let form_isolation =
            prove_vector_form_structure_isolation_v2(&fixture.precomposed).unwrap();
        let vector_plan = build_vector_marked_content_plan_v2(
            &fixture.registry,
            &fixture.accessibility,
            limits,
            &fixture.navigation,
            &fixture.book_profile,
            &book,
            &standard_paints,
            &[],
            &fixture.precomposed,
            &form_isolation,
            &fixture.block_selected,
            &fixture.math_flows,
        )
        .unwrap();
        assert_eq!(vector_plan.selected_binding(), &fixture.selected);
        let serialization = vector_plan
            .authorize_pdf_serialization(
                &fixture.registry,
                &fixture.accessibility,
                limits,
                &fixture.navigation,
                &fixture.book_profile,
                &book,
                &fixture.precomposed,
                &form_isolation,
                &fixture.block_selected,
                &fixture.math_flows,
            )
            .unwrap();
        let candidates = VectorContentCandidateRegistry::from_admitted(
            &fixture.figure.layout.admitted,
            package.resources(),
        )
        .unwrap();
        let incomplete_form_plans =
            finalize_staging_safe_vector_forms_v2(&fixture.precomposed, &candidates, limits)
                .unwrap();
        let incomplete_vector = crate::build_staging_safe_vector_pdf_contribution_v2(
            &fixture.precomposed,
            &incomplete_form_plans,
            &candidates,
            limits,
        )
        .unwrap();
        assert!(matches!(
            write_staging_tagged_pdf_v2(
                package,
                &fixture.navigation,
                &fixture.semantics,
                &fixture.accessibility,
                &fixture.book_profile,
                &book,
                &fixture.registry,
                serialization,
                &fixture.precomposed,
                &form_isolation,
                &fixture.figure.layout.admitted,
                &incomplete_form_plans,
                &candidates,
                &incomplete_vector,
                limits,
                &EngineIdentity::compiled(),
                EffectiveConfigFingerprint::from_untrusted_bytes(sha256(
                    b"tagged-incomplete-figure-effective-config-v2",
                )),
            ),
            Err(TaggedPdfV2Error::VectorMismatch)
        ));
        let form_plans =
            finalize_staging_combined_safe_vector_forms_v2(&fixture.display, &candidates, limits)
                .unwrap();
        let vector = crate::build_staging_combined_safe_vector_pdf_contribution_v2(
            &fixture.display,
            &form_plans,
            &candidates,
            limits,
        )
        .unwrap();
        let write = || {
            write_staging_tagged_pdf_v2_with_combined_vectors(
                package,
                &fixture.navigation,
                &fixture.semantics,
                &fixture.accessibility,
                &fixture.book_profile,
                &book,
                &fixture.registry,
                serialization,
                &fixture.precomposed,
                &fixture.display,
                &form_isolation,
                &fixture.figure.layout.admitted,
                &form_plans,
                &candidates,
                &vector,
                limits,
                &EngineIdentity::compiled(),
                EffectiveConfigFingerprint::from_untrusted_bytes(sha256(
                    b"tagged-combined-figure-effective-config-v2",
                )),
            )
            .unwrap()
        };
        let first = write();
        let second = write();
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.observation().vector_usage_count(), 1);
        assert_eq!(first.observation().form_object_count(), 1);
        assert_eq!(byte_occurrences(first.bytes(), b"/Subtype /Form"), 1);
        assert_eq!(byte_occurrences(first.bytes(), b"/V0 Do"), 1);
        assert_eq!(byte_occurrences(first.bytes(), b"/S /Figure"), 1);
        assert_eq!(byte_occurrences(first.bytes(), b"/Alt <FEFF"), 1);
        assert_eq!(byte_occurrences(first.bytes(), b"/Subtype /Image"), 0);
    }

    #[test]
    fn complete_pdf_object_graph_budget_is_charged_once_before_allocation() {
        with_fixture(|fixture| {
            let equation_fonts = fixture.equation_fonts();
            let exact = TaggedObjectPlanV2::new(
                fixture.book,
                fixture.registry,
                fixture.vector_plan.marked_content(),
                fixture.vector,
                &equation_fonts,
                None,
                &[],
                fixture.limits,
            )
            .unwrap()
            .object_count;
            let exact_limits = ResourceLimits {
                max_pdf_objects: exact,
                ..ResourceLimits::default()
            };
            let exact_limits = M4EffectiveResourceLimits::new(
                ValidatedResourceLimits::new(exact_limits).unwrap(),
                M4ResourceLimits::default(),
            )
            .unwrap();
            assert_eq!(
                TaggedObjectPlanV2::new(
                    fixture.book,
                    fixture.registry,
                    fixture.vector_plan.marked_content(),
                    fixture.vector,
                    &equation_fonts,
                    None,
                    &[],
                    &exact_limits,
                )
                .unwrap()
                .object_count,
                exact
            );
            let over_limits = ResourceLimits {
                max_pdf_objects: exact - 1,
                ..ResourceLimits::default()
            };
            let over_limits = M4EffectiveResourceLimits::new(
                ValidatedResourceLimits::new(over_limits).unwrap(),
                M4ResourceLimits::default(),
            )
            .unwrap();
            assert!(matches!(
                TaggedObjectPlanV2::new(
                    fixture.book,
                    fixture.registry,
                    fixture.vector_plan.marked_content(),
                    fixture.vector,
                    &equation_fonts,
                    None,
                    &[],
                    &over_limits,
                ),
                Err(TaggedPdfV2Error::ObjectLimit)
            ));
        });
    }

    #[test]
    fn vector_actual_text_extraction_uses_resolved_text_in_document_order() {
        with_fixture(|fixture| {
            let pdf = fixture.write().unwrap();
            let extracted = extract_accessible_text(pdf.bytes());
            assert_eq!(extracted, ["xたすy", "xたすy、式1", "(1)"]);
            assert!(!contains_bytes(pdf.bytes(), b"x+y"));
        });
    }

    #[test]
    fn tagged_vector_pdf_v2_equation_font_plan_is_deterministic_and_limit_typed() {
        with_fixture(|fixture| {
            let usages = equation_text_usages_v2(
                fixture.serialization.equation_number_shapes(),
                fixture.admitted,
            )
            .unwrap();
            let fonts =
                finalize_staging_pdf_text_fonts(fixture.admitted, &usages, fixture.limits.base())
                    .unwrap();
            let mut reordered = usages.clone();
            reordered.reverse();
            reordered.extend(usages.iter().cloned());
            assert_eq!(
                finalize_staging_pdf_text_fonts(
                    fixture.admitted,
                    &reordered,
                    fixture.limits.base()
                )
                .unwrap(),
                fonts,
            );
            let cid_count = fonts[0].pdf_font().subset_plan().cids.len() as u16;
            assert!(cid_count > 1);
            let exact = ValidatedResourceLimits::new(ResourceLimits {
                max_cids_per_font: cid_count,
                ..ResourceLimits::default()
            })
            .unwrap();
            assert_eq!(
                finalize_staging_pdf_text_fonts(fixture.admitted, &usages, &exact).unwrap(),
                fonts
            );
            let insufficient = ValidatedResourceLimits::new(ResourceLimits {
                max_cids_per_font: cid_count - 1,
                ..ResourceLimits::default()
            })
            .unwrap();
            let error = finalize_staging_pdf_text_fonts(fixture.admitted, &usages, &insufficient)
                .unwrap_err();
            assert_eq!(
                map_equation_resource_error_v2(error),
                TaggedPdfV2Error::ResourceLimit
            );
            assert!(map_equation_resource_error_v2(error)
                .to_string()
                .starts_with("G6100:"));

            // A pre-shaped many-scalar/single-glyph cluster must retain its
            // source text via ActualText, without inventing per-CID Unicode.
            let actual_text_only = StagingPdfTextClusterUsage::new(
                usages[0].font_face_id(),
                DisplayTextSpan::new(
                    DisplayTextBufferId::new(99),
                    Utf8ByteOffset::new(0),
                    Utf8ByteOffset::new(2),
                )
                .unwrap(),
                "fi".to_owned(),
                vec![usages[0].glyphs()[0]],
            )
            .unwrap();
            let fonts = finalize_staging_pdf_text_fonts(
                fixture.admitted,
                &[actual_text_only],
                fixture.limits.base(),
            )
            .unwrap();
            assert!(fonts[0].clusters()[0].requires_actual_text());
            let cmap = crate::to_unicode_cmap(fonts[0].pdf_font(), 1_000_000).unwrap();
            assert!(!contains_bytes(&cmap, b"beginbfchar"));
        });
    }

    #[test]
    fn book_navigation_pdf_v2_and_vector_closure_share_final_hash() {
        with_fixture(|fixture| {
            let pdf = fixture.write().unwrap();
            assert_eq!(
                pdf.observation().pdf_sha256(),
                pdf.book_navigation().final_pdf_sha256()
            );
            assert_eq!(
                pdf.observation().pdf_sha256(),
                pdf.safe_vector().final_pdf_sha256()
            );
            assert_eq!(
                pdf.observation().pdf_byte_length(),
                pdf.book_navigation().final_pdf_byte_length()
            );
            assert_eq!(pdf.book_navigation().document_language(), "ja");
            assert_eq!(
                pdf.book_navigation().xmp_sha256(),
                pdf.observation().xmp_sha256()
            );
        });
    }
}
