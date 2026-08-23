#![forbid(unsafe_code)]

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use typaxis_core::{
    sha256, AnchorId, EffectiveConfigFingerprint, FontInstanceId, ImageResourceId,
    LayoutStateFingerprint, Length, MasterId, PdfStreamCompression, Point, PositiveLength, Rect,
    ValidatedResourceLimits,
};
use typaxis_display_list::{
    DestinationView, DisplayPage, LinkAnnotation, LinkTarget, NamedDestination,
    ValidatedDisplayDocument,
};
use typaxis_resources::{
    FrozenPdfFontPlan, FrozenPdfImagePlan, FrozenPdfResourcePlans, PdfFontIndirectObjectRole,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectId(u32);
impl ObjectId {
    pub const fn new(value: u32) -> Option<Self> {
        if value == 0 {
            None
        } else {
            Some(Self(value))
        }
    }
    pub const fn get(self) -> u32 {
        self.0
    }
}
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PdfName(Vec<u8>);
impl PdfName {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, PdfError> {
        let bytes = bytes.into();
        if bytes.is_empty() || bytes.contains(&0) {
            Err(PdfError::InvalidName)
        } else {
            Ok(Self(bytes))
        }
    }
    pub fn encoded(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(self.0.len() + 1);
        output.push(b'/');
        for &byte in &self.0 {
            let regular = (33..=126).contains(&byte) && !b"()<>[]{}/%#".contains(&byte);
            if regular {
                output.push(byte);
            } else {
                output.extend_from_slice(format!("#{byte:02X}").as_bytes());
            }
        }
        output
    }
    fn is(&self, value: &[u8]) -> bool {
        self.0 == value
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdfDecimal {
    pub coefficient: i64,
    pub scale: u8,
}
impl PdfDecimal {
    pub fn new(coefficient: i64, scale: u8) -> Result<Self, PdfError> {
        if scale > 12 {
            Err(PdfError::DecimalScaleTooLarge)
        } else {
            Ok(Self { coefficient, scale })
        }
    }
    pub fn canonical(self) -> String {
        if self.coefficient == 0 {
            return "0".to_owned();
        }
        if self.scale == 0 {
            return self.coefficient.to_string();
        }
        let negative = self.coefficient < 0;
        let digits = self.coefficient.unsigned_abs().to_string();
        let scale = usize::from(self.scale);
        let mut output = if digits.len() <= scale {
            format!("0.{}{}", "0".repeat(scale - digits.len()), digits)
        } else {
            let split = digits.len() - scale;
            format!("{}.{}", &digits[..split], &digits[split..])
        };
        while output.ends_with('0') {
            output.pop();
        }
        if output.ends_with('.') {
            output.pop();
        }
        if negative {
            output.insert(0, '-');
        }
        output
    }
}
pub type PdfDictionary = BTreeMap<PdfName, PdfValue>;
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PdfValue {
    Null,
    Bool(bool),
    Integer(i64),
    Decimal(PdfDecimal),
    Name(PdfName),
    ByteString(Vec<u8>),
    Array(Vec<PdfValue>),
    Dictionary(PdfDictionary),
    Reference(ObjectId),
}
impl Drop for PdfValue {
    fn drop(&mut self) {
        fn take_children(value: &mut PdfValue, pending: &mut Vec<PdfValue>) {
            match value {
                PdfValue::Array(values) => pending.append(values),
                PdfValue::Dictionary(dictionary) => {
                    pending.extend(std::mem::take(dictionary).into_values());
                }
                _ => {}
            }
        }

        let mut pending = Vec::new();
        take_children(self, &mut pending);
        while let Some(mut value) = pending.pop() {
            take_children(&mut value, &mut pending);
            // `value` now owns no recursive children and is safe to drop.
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamEncoding {
    None,
    Flate,
    EncodedFlate,
    Dct,
}
/// `raw_data` is unencoded for `None`/`Flate`; the two encoded variants carry
/// bytes from a sealed image-encoder receipt. The serializer always owns
/// `/Length`, `/Filter`, and `/DecodeParms`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfStreamObject {
    pub dictionary: PdfDictionary,
    pub encoding: StreamEncoding,
    pub raw_data: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IndirectObjectBody {
    Value(PdfValue),
    Stream(PdfStreamObject),
    /// The sealed subset payload and all late-finalizer facts. The surrounding
    /// Type0/CIDFont/descriptor dictionaries refer to this object; the two
    /// mapping objects find their canonical data through this object ID.
    FrozenFontProgram(FrozenPdfFontPlan),
    FrozenToUnicodeCMap {
        font_program_object: ObjectId,
    },
    FrozenCidToGidMap {
        font_program_object: ObjectId,
    },
    FrozenImageResource {
        plan: FrozenPdfImagePlan,
        alpha_mask_object: Option<ObjectId>,
    },
    DisplayPageContent(DisplayPage),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PdfError {
    InvalidName,
    DecimalScaleTooLarge,
    DuplicateObject,
    MissingRoot(ObjectId),
    MissingReference(ObjectId),
    ReservedStreamKey,
    RootIsNotCatalog,
    CatalogMissingPages,
    InvalidPageTree,
    PageTreeCycle,
    OutputTooLarge,
    SparseObjectId,
    UnreachableObject(ObjectId),
    ObjectLimit,
    ObjectCountOverflow,
    SelectedLayoutMismatch,
    SelectedPageClosure,
    PageMasterMismatch,
    ResourcePlanMismatch,
    InvalidDestinationClosure,
    InvalidAnnotationClosure,
    DirectValueDepth,
    PageTreeDepth,
}
/// Low-level object graph assembly API. The resulting value is explicitly
/// untrusted and cannot be converted into the publication `FrozenPdfGraph`.
#[derive(Clone, Debug)]
pub struct UntrustedPdfObjectGraphBuilder {
    objects: BTreeMap<ObjectId, IndirectObjectBody>,
    max_objects: u32,
}
impl UntrustedPdfObjectGraphBuilder {
    pub fn new(limits: &ValidatedResourceLimits) -> Self {
        Self {
            objects: BTreeMap::new(),
            max_objects: limits.get().max_pdf_objects,
        }
    }
    pub fn insert(&mut self, id: ObjectId, body: IndirectObjectBody) -> Result<(), PdfError> {
        if self.objects.contains_key(&id) {
            return Err(PdfError::DuplicateObject);
        }
        if self.objects.len() >= self.max_objects as usize {
            return Err(PdfError::ObjectLimit);
        }
        match self.objects.entry(id) {
            Entry::Vacant(slot) => {
                slot.insert(body);
                Ok(())
            }
            Entry::Occupied(_) => Err(PdfError::DuplicateObject),
        }
    }
    pub fn validate_untrusted(
        self,
        root: ObjectId,
    ) -> Result<ValidatedUntrustedPdfObjectGraph, PdfError> {
        if !self.objects.contains_key(&root) {
            return Err(PdfError::MissingRoot(root));
        }
        for (index, id) in self.objects.keys().enumerate() {
            let expected = u32::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1));
            if expected != Some(id.get()) {
                return Err(PdfError::SparseObjectId);
            }
        }
        for body in self.objects.values() {
            if let IndirectObjectBody::Stream(stream) = body {
                for key in stream.dictionary.keys() {
                    if key.is(b"Length") || key.is(b"Filter") || key.is(b"DecodeParms") {
                        return Err(PdfError::ReservedStreamKey);
                    }
                }
            }
        }
        let mut references = BTreeSet::new();
        for body in self.objects.values() {
            collect_references(body, &mut references)?;
        }
        for id in references {
            if !self.objects.contains_key(&id) {
                return Err(PdfError::MissingReference(id));
            }
        }
        let reachable = collect_reachable(&self.objects, root)?;
        if let Some(unreachable) = self
            .objects
            .keys()
            .find(|id| !reachable.contains(id))
            .copied()
        {
            return Err(PdfError::UnreachableObject(unreachable));
        }
        validate_page_tree(&self.objects, root)?;
        Ok(ValidatedUntrustedPdfObjectGraph {
            root,
            objects: self.objects,
        })
    }
}

fn collect_reachable(
    objects: &BTreeMap<ObjectId, IndirectObjectBody>,
    root: ObjectId,
) -> Result<BTreeSet<ObjectId>, PdfError> {
    let mut reachable = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if !reachable.insert(id) {
            continue;
        }
        if let Some(body) = objects.get(&id) {
            let mut references = BTreeSet::new();
            collect_references(body, &mut references)?;
            pending.extend(references);
        }
    }
    Ok(reachable)
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedUntrustedPdfObjectGraph {
    root: ObjectId,
    objects: BTreeMap<ObjectId, IndirectObjectBody>,
}
impl ValidatedUntrustedPdfObjectGraph {
    pub const fn root(&self) -> ObjectId {
        self.root
    }
    pub fn iter(&self) -> impl Iterator<Item = (&ObjectId, &IndirectObjectBody)> {
        self.objects.iter()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenPageGeometry {
    page_index: u32,
    master_id: MasterId,
    width: PositiveLength,
    height: PositiveLength,
}
impl FrozenPageGeometry {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn width(&self) -> PositiveLength {
        self.width
    }
    pub const fn height(&self) -> PositiveLength {
        self.height
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PdfResourceBinding<Id> {
    logical_id: Id,
    name: PdfName,
    object_id: ObjectId,
}

/// Publication graph issued only by `PdfBackend::build`. The raw graph is
/// retained privately; the low-level untrusted builder has no conversion path
/// to this type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenPdfGraph {
    graph: ValidatedUntrustedPdfObjectGraph,
    selected_layout_fingerprint: LayoutStateFingerprint,
    pages: Vec<FrozenPageGeometry>,
    page_count: u32,
    object_count: u32,
    font_bindings: Vec<PdfResourceBinding<FontInstanceId>>,
    image_bindings: Vec<PdfResourceBinding<ImageResourceId>>,
}
impl FrozenPdfGraph {
    pub const fn selected_layout_fingerprint(&self) -> LayoutStateFingerprint {
        self.selected_layout_fingerprint
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub fn pages(&self) -> &[FrozenPageGeometry] {
        &self.pages
    }
    pub fn font_resource_names(&self) -> impl Iterator<Item = (FontInstanceId, &PdfName)> {
        self.font_bindings
            .iter()
            .map(|binding| (binding.logical_id, &binding.name))
    }
    pub fn image_resource_names(&self) -> impl Iterator<Item = (ImageResourceId, &PdfName)> {
        self.image_bindings
            .iter()
            .map(|binding| (binding.logical_id, &binding.name))
    }
}

/// Bytes emitted by the crate-owned PDF serializer and bound to the exact
/// trusted graph facts consumed by manifest publication.
///
/// The receipt is deliberately non-`Clone`: one serializer emission must be
/// consumed by exactly one output/publication session.
///
/// ```compile_fail
/// use typaxis_pdf::VerifiedPdfBytesReceipt;
/// fn requires_clone<T: Clone>() {}
/// requires_clone::<VerifiedPdfBytesReceipt>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPdfBytesReceipt {
    bytes: Vec<u8>,
    sha256: [u8; 32],
    selected_layout_fingerprint: LayoutStateFingerprint,
    page_count: u32,
    object_count: u32,
    stream_compression: PdfStreamCompression,
    config_fingerprint: EffectiveConfigFingerprint,
}
impl VerifiedPdfBytesReceipt {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn selected_layout_fingerprint(&self) -> LayoutStateFingerprint {
        self.selected_layout_fingerprint
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub const fn stream_compression(&self) -> PdfStreamCompression {
        self.stream_compression
    }
    pub const fn config_fingerprint(&self) -> EffectiveConfigFingerprint {
        self.config_fingerprint
    }
}

/// Capability reserved for the in-crate serializer. External callers can pass
/// a receipt onward but cannot bless arbitrary byte slices.
#[derive(Debug)]
pub struct VerifiedPdfSerializerReceiptOwner {
    _private: (),
}
impl VerifiedPdfSerializerReceiptOwner {
    #[allow(dead_code)] // reserved for the in-crate classic-xref serializer
    fn new() -> Self {
        Self { _private: () }
    }
    pub fn issue(
        &self,
        graph: &FrozenPdfGraph,
        bytes: Vec<u8>,
        stream_compression: PdfStreamCompression,
        config_fingerprint: EffectiveConfigFingerprint,
        limits: &ValidatedResourceLimits,
    ) -> Result<VerifiedPdfBytesReceipt, PdfError> {
        let byte_length = u64::try_from(bytes.len()).map_err(|_| PdfError::OutputTooLarge)?;
        if bytes.is_empty() || byte_length > limits.get().max_output_bytes {
            return Err(PdfError::OutputTooLarge);
        }
        let digest = sha256(&bytes);
        Ok(VerifiedPdfBytesReceipt {
            bytes,
            sha256: digest,
            selected_layout_fingerprint: graph.selected_layout_fingerprint(),
            page_count: graph.page_count(),
            object_count: graph.object_count(),
            stream_compression,
            config_fingerprint,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FontObjectIds {
    type0: ObjectId,
    cid_font: ObjectId,
    descriptor: ObjectId,
    font_program: ObjectId,
    to_unicode: ObjectId,
    cid_to_gid: ObjectId,
}
impl FontObjectIds {
    fn allocate(
        plan: &FrozenPdfFontPlan,
        allocator: &mut DenseObjectAllocator,
    ) -> Result<Self, PdfError> {
        Self::allocate_blueprint(plan.indirect_object_blueprint(), allocator)
    }

    fn allocate_blueprint(
        blueprint: &[PdfFontIndirectObjectRole],
        allocator: &mut DenseObjectAllocator,
    ) -> Result<Self, PdfError> {
        let mut type0 = None;
        let mut cid_font = None;
        let mut descriptor = None;
        let mut font_program = None;
        let mut to_unicode = None;
        let mut cid_to_gid = None;
        for role in blueprint {
            let slot = match role {
                PdfFontIndirectObjectRole::Type0Font => &mut type0,
                PdfFontIndirectObjectRole::CidFont => &mut cid_font,
                PdfFontIndirectObjectRole::FontDescriptor => &mut descriptor,
                PdfFontIndirectObjectRole::EmbeddedFontProgram => &mut font_program,
                PdfFontIndirectObjectRole::ToUnicodeCMap => &mut to_unicode,
                PdfFontIndirectObjectRole::CidToGidMap => &mut cid_to_gid,
            };
            if slot.replace(allocator.allocate()?).is_some() {
                return Err(PdfError::ResourcePlanMismatch);
            }
        }
        Ok(Self {
            type0: type0.ok_or(PdfError::ResourcePlanMismatch)?,
            cid_font: cid_font.ok_or(PdfError::ResourcePlanMismatch)?,
            descriptor: descriptor.ok_or(PdfError::ResourcePlanMismatch)?,
            font_program: font_program.ok_or(PdfError::ResourcePlanMismatch)?,
            to_unicode: to_unicode.ok_or(PdfError::ResourcePlanMismatch)?,
            cid_to_gid: cid_to_gid.ok_or(PdfError::ResourcePlanMismatch)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PageObjectIds {
    page: ObjectId,
    content: ObjectId,
    annotations: Vec<ObjectId>,
}

fn required_object_count(
    resource_plans: &FrozenPdfResourcePlans,
    pages: &[DisplayPage],
) -> Result<u32, PdfError> {
    let mut required = 2usize; // Catalog and Pages root.
    for font in resource_plans.fonts() {
        required = required
            .checked_add(
                usize::try_from(font.indirect_object_count())
                    .map_err(|_| PdfError::ObjectCountOverflow)?,
            )
            .ok_or(PdfError::ObjectCountOverflow)?;
    }
    for image in resource_plans.images() {
        required = required
            .checked_add(
                usize::try_from(image.indirect_object_count())
                    .map_err(|_| PdfError::ObjectCountOverflow)?,
            )
            .ok_or(PdfError::ObjectCountOverflow)?;
    }
    for page in pages {
        required = required
            .checked_add(2)
            .and_then(|count| count.checked_add(page.annotations.len()))
            .ok_or(PdfError::ObjectCountOverflow)?;
    }
    u32::try_from(required).map_err(|_| PdfError::ObjectCountOverflow)
}

pub struct PdfBackend;
impl PdfBackend {
    pub fn build(
        display: ValidatedDisplayDocument,
        resource_plans: FrozenPdfResourcePlans,
        limits: &ValidatedResourceLimits,
    ) -> Result<FrozenPdfGraph, PdfError> {
        let selected_layout_fingerprint = display.document().source_layout().state_fingerprint();
        let selected_geometry = display.selected_page_geometry();
        if display.document().pages.len() != selected_geometry.len() || selected_geometry.is_empty()
        {
            return Err(PdfError::SelectedPageClosure);
        }

        // Every indirect-object role, including every annotation and every
        // member of a composite font plan, is counted before the allocator or
        // any object-body/resource-name collection is created.
        let required_objects = required_object_count(&resource_plans, &display.document().pages)?;
        if required_objects > limits.get().max_pdf_objects {
            return Err(PdfError::ObjectLimit);
        }
        if !resource_plans.matches_display(&display) {
            return Err(PdfError::ResourcePlanMismatch);
        }

        let mut page_geometry = Vec::new();
        page_geometry
            .try_reserve_exact(selected_geometry.len())
            .map_err(|_| PdfError::ObjectCountOverflow)?;
        for (display_page, geometry) in display.document().pages.iter().zip(selected_geometry) {
            if display_page.page_index != geometry.page_index()
                || display_page.width != geometry.width()
                || display_page.height != geometry.height()
            {
                return Err(PdfError::PageMasterMismatch);
            }
            page_geometry.push(FrozenPageGeometry {
                page_index: display_page.page_index,
                master_id: geometry.master_id().clone(),
                width: geometry.width(),
                height: geometry.height(),
            });
        }
        let page_count =
            u32::try_from(page_geometry.len()).map_err(|_| PdfError::ObjectCountOverflow)?;

        let mut allocator = DenseObjectAllocator::new(required_objects);
        let catalog_id = allocator.allocate()?;
        let pages_id = allocator.allocate()?;
        let font_object_ids: Vec<_> = resource_plans
            .fonts()
            .iter()
            .map(|plan| FontObjectIds::allocate(plan, &mut allocator))
            .collect::<Result<_, _>>()?;
        let image_object_ids: Vec<_> = resource_plans
            .images()
            .iter()
            .map(|_| allocator.allocate())
            .collect::<Result<_, _>>()?;
        let page_object_ids: Vec<_> = display
            .document()
            .pages
            .iter()
            .map(|page| {
                let page_id = allocator.allocate()?;
                let content = allocator.allocate()?;
                let annotations = page
                    .annotations
                    .iter()
                    .map(|_| allocator.allocate())
                    .collect::<Result<_, _>>()?;
                Ok(PageObjectIds {
                    page: page_id,
                    content,
                    annotations,
                })
            })
            .collect::<Result<_, PdfError>>()?;
        allocator.finish()?;

        let mut font_bindings = Vec::new();
        let mut image_bindings = Vec::new();
        let mut font_resources = PdfDictionary::new();
        let mut image_resources = PdfDictionary::new();
        for (index, (plan, object_ids)) in resource_plans
            .fonts()
            .iter()
            .zip(&font_object_ids)
            .enumerate()
        {
            let name = PdfName::from_bytes(format!("F{index}").into_bytes())?;
            font_resources.insert(name.clone(), PdfValue::Reference(object_ids.type0));
            font_bindings.push(PdfResourceBinding {
                logical_id: plan.font_instance_id(),
                name,
                object_id: object_ids.type0,
            });
        }
        for (index, (plan, object_id)) in resource_plans
            .images()
            .iter()
            .zip(&image_object_ids)
            .enumerate()
        {
            let name = PdfName::from_bytes(format!("Im{index}").into_bytes())?;
            image_resources.insert(name.clone(), PdfValue::Reference(*object_id));
            image_bindings.push(PdfResourceBinding {
                logical_id: plan.image_id(),
                name,
                object_id: *object_id,
            });
        }

        let mut resources = PdfDictionary::new();
        if !font_resources.is_empty() {
            resources.insert(pdf_name(b"Font")?, PdfValue::Dictionary(font_resources));
        }
        if !image_resources.is_empty() {
            resources.insert(pdf_name(b"XObject")?, PdfValue::Dictionary(image_resources));
        }
        let mut catalog = PdfDictionary::new();
        catalog.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Catalog")?));
        catalog.insert(pdf_name(b"Pages")?, PdfValue::Reference(pages_id));
        if !display.document().destinations.is_empty() {
            catalog.insert(
                pdf_name(b"Names")?,
                destination_name_tree(
                    &display.document().destinations,
                    &page_object_ids,
                    &page_geometry,
                )?,
            );
        }
        let mut pages = PdfDictionary::new();
        pages.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Pages")?));
        pages.insert(
            pdf_name(b"Kids")?,
            PdfValue::Array(
                page_object_ids
                    .iter()
                    .map(|ids| PdfValue::Reference(ids.page))
                    .collect(),
            ),
        );
        pages.insert(
            pdf_name(b"Count")?,
            PdfValue::Integer(i64::from(page_count)),
        );
        pages.insert(pdf_name(b"Resources")?, PdfValue::Dictionary(resources));

        let image_ids_by_logical: BTreeMap<_, _> = resource_plans
            .images()
            .iter()
            .zip(&image_object_ids)
            .map(|(plan, id)| (plan.image_id(), *id))
            .collect();
        let (font_plans, image_plans) = resource_plans.into_plans();
        let (display_document, selected_geometry_receipt) = display.into_parts();
        debug_assert_eq!(selected_geometry_receipt.len(), page_geometry.len());

        let mut builder = UntrustedPdfObjectGraphBuilder::new(limits);
        builder.insert(
            catalog_id,
            IndirectObjectBody::Value(PdfValue::Dictionary(catalog)),
        )?;
        builder.insert(
            pages_id,
            IndirectObjectBody::Value(PdfValue::Dictionary(pages)),
        )?;
        for ((plan, object_ids), binding) in font_plans
            .into_iter()
            .zip(font_object_ids)
            .zip(&font_bindings)
        {
            debug_assert_eq!(object_ids.type0, binding.object_id);
            insert_font_objects(&mut builder, plan, object_ids)?;
        }
        for (plan, object_id) in image_plans.into_iter().zip(image_object_ids) {
            let alpha_mask_object = plan
                .alpha_mask()
                .map(|mask| {
                    image_ids_by_logical
                        .get(&mask)
                        .copied()
                        .ok_or(PdfError::ResourcePlanMismatch)
                })
                .transpose()?;
            builder.insert(
                object_id,
                IndirectObjectBody::FrozenImageResource {
                    plan,
                    alpha_mask_object,
                },
            )?;
        }
        for ((geometry, object_ids), display_page) in page_geometry
            .iter()
            .zip(page_object_ids)
            .zip(display_document.pages)
        {
            if display_page.annotations.len() != object_ids.annotations.len() {
                return Err(PdfError::InvalidAnnotationClosure);
            }
            let mut page = PdfDictionary::new();
            page.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Page")?));
            page.insert(pdf_name(b"Parent")?, PdfValue::Reference(pages_id));
            page.insert(
                pdf_name(b"MediaBox")?,
                media_box(geometry.width, geometry.height)?,
            );
            page.insert(
                pdf_name(b"Contents")?,
                PdfValue::Reference(object_ids.content),
            );
            if !object_ids.annotations.is_empty() {
                page.insert(
                    pdf_name(b"Annots")?,
                    PdfValue::Array(
                        object_ids
                            .annotations
                            .iter()
                            .map(|id| PdfValue::Reference(*id))
                            .collect(),
                    ),
                );
            }
            builder.insert(
                object_ids.page,
                IndirectObjectBody::Value(PdfValue::Dictionary(page)),
            )?;
            for (annotation, annotation_id) in
                display_page.annotations.iter().zip(&object_ids.annotations)
            {
                builder.insert(
                    *annotation_id,
                    IndirectObjectBody::Value(PdfValue::Dictionary(annotation_dictionary(
                        annotation,
                        geometry.height,
                    )?)),
                )?;
            }
            builder.insert(
                object_ids.content,
                IndirectObjectBody::DisplayPageContent(display_page),
            )?;
        }
        let graph = builder.validate_untrusted(catalog_id)?;
        let object_count =
            u32::try_from(graph.objects.len()).map_err(|_| PdfError::ObjectCountOverflow)?;
        if object_count != required_objects {
            return Err(PdfError::ObjectCountOverflow);
        }
        Ok(FrozenPdfGraph {
            graph,
            selected_layout_fingerprint,
            pages: page_geometry,
            page_count,
            object_count,
            font_bindings,
            image_bindings,
        })
    }
}

fn insert_font_objects(
    builder: &mut UntrustedPdfObjectGraphBuilder,
    plan: FrozenPdfFontPlan,
    ids: FontObjectIds,
) -> Result<(), PdfError> {
    let base_font = subset_base_font_name(plan.embedded_postscript_name())?;
    let mut type0 = PdfDictionary::new();
    type0.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Font")?));
    type0.insert(pdf_name(b"Subtype")?, PdfValue::Name(pdf_name(b"Type0")?));
    type0.insert(pdf_name(b"BaseFont")?, PdfValue::Name(base_font.clone()));
    type0.insert(
        pdf_name(b"Encoding")?,
        PdfValue::Name(pdf_name(b"Identity-H")?),
    );
    type0.insert(
        pdf_name(b"DescendantFonts")?,
        PdfValue::Array(vec![PdfValue::Reference(ids.cid_font)]),
    );
    type0.insert(pdf_name(b"ToUnicode")?, PdfValue::Reference(ids.to_unicode));

    let mut cid_system_info = PdfDictionary::new();
    cid_system_info.insert(
        pdf_name(b"Registry")?,
        PdfValue::ByteString(b"Adobe".to_vec()),
    );
    cid_system_info.insert(
        pdf_name(b"Ordering")?,
        PdfValue::ByteString(b"Identity".to_vec()),
    );
    cid_system_info.insert(pdf_name(b"Supplement")?, PdfValue::Integer(0));
    let mut widths = Vec::new();
    for binding in &plan.subset_plan().cids {
        widths.push(PdfValue::Integer(i64::from(binding.cid.get())));
        widths.push(PdfValue::Array(vec![PdfValue::Integer(i64::from(
            binding.width_1000,
        ))]));
    }
    let mut cid_font = PdfDictionary::new();
    cid_font.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Font")?));
    cid_font.insert(
        pdf_name(b"Subtype")?,
        PdfValue::Name(pdf_name(b"CIDFontType2")?),
    );
    cid_font.insert(pdf_name(b"BaseFont")?, PdfValue::Name(base_font.clone()));
    cid_font.insert(
        pdf_name(b"CIDSystemInfo")?,
        PdfValue::Dictionary(cid_system_info),
    );
    cid_font.insert(
        pdf_name(b"FontDescriptor")?,
        PdfValue::Reference(ids.descriptor),
    );
    cid_font.insert(pdf_name(b"DW")?, PdfValue::Integer(1_000));
    if !widths.is_empty() {
        cid_font.insert(pdf_name(b"W")?, PdfValue::Array(widths));
    }
    cid_font.insert(
        pdf_name(b"CIDToGIDMap")?,
        PdfValue::Reference(ids.cid_to_gid),
    );

    let metrics = plan.metrics();
    let mut descriptor = PdfDictionary::new();
    descriptor.insert(
        pdf_name(b"Type")?,
        PdfValue::Name(pdf_name(b"FontDescriptor")?),
    );
    descriptor.insert(pdf_name(b"FontName")?, PdfValue::Name(base_font));
    descriptor.insert(
        pdf_name(b"Flags")?,
        PdfValue::Integer(i64::from(metrics.flags)),
    );
    descriptor.insert(
        pdf_name(b"FontBBox")?,
        PdfValue::Array(
            metrics
                .bbox_1000
                .iter()
                .map(|value| PdfValue::Integer(i64::from(*value)))
                .collect(),
        ),
    );
    descriptor.insert(
        pdf_name(b"ItalicAngle")?,
        PdfValue::Decimal(PdfDecimal::new(
            i64::from(metrics.italic_angle_milli_degrees),
            3,
        )?),
    );
    descriptor.insert(
        pdf_name(b"Ascent")?,
        PdfValue::Integer(i64::from(metrics.ascent_1000)),
    );
    descriptor.insert(
        pdf_name(b"Descent")?,
        PdfValue::Integer(i64::from(metrics.descent_1000)),
    );
    descriptor.insert(
        pdf_name(b"CapHeight")?,
        PdfValue::Integer(i64::from(metrics.cap_height_1000)),
    );
    descriptor.insert(
        pdf_name(b"StemV")?,
        PdfValue::Integer(i64::from(metrics.stem_v_1000)),
    );
    descriptor.insert(
        pdf_name(b"FontFile2")?,
        PdfValue::Reference(ids.font_program),
    );

    builder.insert(
        ids.type0,
        IndirectObjectBody::Value(PdfValue::Dictionary(type0)),
    )?;
    builder.insert(
        ids.cid_font,
        IndirectObjectBody::Value(PdfValue::Dictionary(cid_font)),
    )?;
    builder.insert(
        ids.descriptor,
        IndirectObjectBody::Value(PdfValue::Dictionary(descriptor)),
    )?;
    builder.insert(
        ids.font_program,
        IndirectObjectBody::FrozenFontProgram(plan),
    )?;
    builder.insert(
        ids.to_unicode,
        IndirectObjectBody::FrozenToUnicodeCMap {
            font_program_object: ids.font_program,
        },
    )?;
    builder.insert(
        ids.cid_to_gid,
        IndirectObjectBody::FrozenCidToGidMap {
            font_program_object: ids.font_program,
        },
    )?;
    Ok(())
}

fn subset_base_font_name(embedded_postscript_name: &str) -> Result<PdfName, PdfError> {
    PdfName::from_bytes(embedded_postscript_name.as_bytes().to_vec())
}

fn destination_name_tree(
    destinations: &[NamedDestination],
    page_ids: &[PageObjectIds],
    geometry: &[FrozenPageGeometry],
) -> Result<PdfValue, PdfError> {
    let mut names = Vec::new();
    let name_value_count = destination_name_value_count(destinations.len())?;
    names
        .try_reserve_exact(name_value_count)
        .map_err(|_| PdfError::ObjectCountOverflow)?;
    for destination in destinations {
        let page_index = usize::try_from(destination.page_index)
            .map_err(|_| PdfError::InvalidDestinationClosure)?;
        let ids = page_ids
            .get(page_index)
            .ok_or(PdfError::InvalidDestinationClosure)?;
        let page = geometry
            .get(page_index)
            .ok_or(PdfError::InvalidDestinationClosure)?;
        if page.page_index != destination.page_index {
            return Err(PdfError::InvalidDestinationClosure);
        }
        names.push(PdfValue::ByteString(
            destination.anchor_id.as_str().as_bytes().to_vec(),
        ));
        names.push(destination_array(destination, ids.page, page.height)?);
    }
    let mut destination_tree = PdfDictionary::new();
    destination_tree.insert(pdf_name(b"Names")?, PdfValue::Array(names));
    let mut names_dictionary = PdfDictionary::new();
    names_dictionary.insert(pdf_name(b"Dests")?, PdfValue::Dictionary(destination_tree));
    Ok(PdfValue::Dictionary(names_dictionary))
}

fn destination_name_value_count(destination_count: usize) -> Result<usize, PdfError> {
    destination_count
        .checked_mul(2)
        .ok_or(PdfError::ObjectCountOverflow)
}

fn destination_array(
    destination: &NamedDestination,
    page_id: ObjectId,
    page_height: PositiveLength,
) -> Result<PdfValue, PdfError> {
    let mut values = vec![PdfValue::Reference(page_id)];
    match destination.view {
        DestinationView::Xyz { point } => {
            values.push(PdfValue::Name(pdf_name(b"XYZ")?));
            values.extend(pdf_point(point, page_height)?);
            values.push(PdfValue::Null);
        }
        DestinationView::FitPage => values.push(PdfValue::Name(pdf_name(b"Fit")?)),
        DestinationView::FitWidth { top } => {
            values.push(PdfValue::Name(pdf_name(b"FitH")?));
            values.push(match top {
                Some(top) => pdf_length(pdf_y(page_height, top)?)?,
                None => PdfValue::Null,
            });
        }
    }
    Ok(PdfValue::Array(values))
}

fn annotation_dictionary(
    annotation: &LinkAnnotation,
    page_height: PositiveLength,
) -> Result<PdfDictionary, PdfError> {
    let mut dictionary = PdfDictionary::new();
    dictionary.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Annot")?));
    dictionary.insert(pdf_name(b"Subtype")?, PdfValue::Name(pdf_name(b"Link")?));
    dictionary.insert(
        pdf_name(b"Rect")?,
        annotation_rectangle(annotation.rect, page_height)?,
    );
    dictionary.insert(
        pdf_name(b"Border")?,
        PdfValue::Array(vec![
            PdfValue::Integer(0),
            PdfValue::Integer(0),
            PdfValue::Integer(0),
        ]),
    );
    match &annotation.target {
        LinkTarget::Internal(anchor) => {
            dictionary.insert(
                pdf_name(b"Dest")?,
                PdfValue::ByteString(anchor.as_str().as_bytes().to_vec()),
            );
        }
        LinkTarget::Uri(uri) => {
            let mut action = PdfDictionary::new();
            action.insert(pdf_name(b"S")?, PdfValue::Name(pdf_name(b"URI")?));
            action.insert(
                pdf_name(b"URI")?,
                PdfValue::ByteString(uri.as_str().as_bytes().to_vec()),
            );
            dictionary.insert(pdf_name(b"A")?, PdfValue::Dictionary(action));
        }
    }
    Ok(dictionary)
}

fn pdf_y(page_height: PositiveLength, y: Length) -> Result<Length, PdfError> {
    page_height
        .get()
        .checked_sub(y)
        .ok_or(PdfError::PageMasterMismatch)
}

fn pdf_point(point: Point, page_height: PositiveLength) -> Result<[PdfValue; 2], PdfError> {
    Ok([
        pdf_length(point.x)?,
        pdf_length(pdf_y(page_height, point.y)?)?,
    ])
}

fn annotation_rectangle(rect: Rect, page_height: PositiveLength) -> Result<PdfValue, PdfError> {
    let right = rect
        .x()
        .checked_add(rect.width().get())
        .ok_or(PdfError::PageMasterMismatch)?;
    let bottom = rect
        .y()
        .checked_add(rect.height().get())
        .ok_or(PdfError::PageMasterMismatch)?;
    Ok(PdfValue::Array(vec![
        pdf_length(rect.x())?,
        pdf_length(pdf_y(page_height, bottom)?)?,
        pdf_length(right)?,
        pdf_length(pdf_y(page_height, rect.y())?)?,
    ]))
}

struct DenseObjectAllocator {
    // One wider than ObjectId so the state after issuing u32::MAX is
    // representable and the inclusive configured maximum can succeed.
    next: u64,
    required: u32,
}
impl DenseObjectAllocator {
    const fn new(required: u32) -> Self {
        Self { next: 1, required }
    }
    fn allocate(&mut self) -> Result<ObjectId, PdfError> {
        if self.next > u64::from(self.required) {
            return Err(PdfError::ObjectCountOverflow);
        }
        let id = u32::try_from(self.next)
            .ok()
            .and_then(ObjectId::new)
            .ok_or(PdfError::ObjectCountOverflow)?;
        self.next = self
            .next
            .checked_add(1)
            .ok_or(PdfError::ObjectCountOverflow)?;
        Ok(id)
    }
    fn finish(self) -> Result<(), PdfError> {
        if self.next == u64::from(self.required) + 1 {
            Ok(())
        } else {
            Err(PdfError::ObjectCountOverflow)
        }
    }
}

fn pdf_name(bytes: &[u8]) -> Result<PdfName, PdfError> {
    PdfName::from_bytes(bytes.to_vec())
}

fn media_box(width: PositiveLength, height: PositiveLength) -> Result<PdfValue, PdfError> {
    Ok(PdfValue::Array(vec![
        PdfValue::Integer(0),
        PdfValue::Integer(0),
        pdf_length(width.get())?,
        pdf_length(height.get())?,
    ]))
}

fn pdf_length(length: Length) -> Result<PdfValue, PdfError> {
    if length == Length::ZERO {
        return Ok(PdfValue::Integer(0));
    }
    for scale in (0..=6u8).rev() {
        let factor = 10i128.pow(u32::from(scale));
        let numerator = i128::from(length.raw())
            .checked_mul(factor)
            .ok_or(PdfError::PageMasterMismatch)?;
        let coefficient =
            round_ratio_ties_even(numerator, 65_536).ok_or(PdfError::PageMasterMismatch)?;
        if let Ok(coefficient) = i64::try_from(coefficient) {
            if coefficient != 0 {
                return Ok(PdfValue::Decimal(PdfDecimal::new(coefficient, scale)?));
            }
        }
    }
    Err(PdfError::PageMasterMismatch)
}

fn round_ratio_ties_even(numerator: i128, denominator: i128) -> Option<i128> {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let doubled = remainder.unsigned_abs().checked_mul(2)?;
    let denominator_abs = denominator.unsigned_abs();
    let adjustment = if remainder.is_negative() { -1 } else { 1 };
    if doubled < denominator_abs || (doubled == denominator_abs && quotient % 2 == 0) {
        Some(quotient)
    } else {
        quotient.checked_add(adjustment)
    }
}

fn dictionary_for(
    objects: &BTreeMap<ObjectId, IndirectObjectBody>,
    id: ObjectId,
) -> Result<&PdfDictionary, PdfError> {
    match objects.get(&id) {
        Some(IndirectObjectBody::Value(PdfValue::Dictionary(value))) => Ok(value),
        _ => Err(PdfError::InvalidPageTree),
    }
}
fn dict_value<'a>(dict: &'a PdfDictionary, key: &[u8]) -> Option<&'a PdfValue> {
    dict.iter()
        .find_map(|(name, value)| if name.is(key) { Some(value) } else { None })
}
fn type_is(dict: &PdfDictionary, expected: &[u8]) -> bool {
    matches!(dict_value(dict, b"Type"), Some(PdfValue::Name(name)) if name.is(expected))
}
fn validate_page_tree(
    objects: &BTreeMap<ObjectId, IndirectObjectBody>,
    root: ObjectId,
) -> Result<(), PdfError> {
    let catalog = dictionary_for(objects, root).map_err(|_| PdfError::RootIsNotCatalog)?;
    if !type_is(catalog, b"Catalog") {
        return Err(PdfError::RootIsNotCatalog);
    }
    let pages = match dict_value(catalog, b"Pages") {
        Some(PdfValue::Reference(id)) => *id,
        _ => return Err(PdfError::CatalogMissingPages),
    };
    let pages_dictionary = dictionary_for(objects, pages)?;
    if !type_is(pages_dictionary, b"Pages") || dict_value(pages_dictionary, b"Parent").is_some() {
        return Err(PdfError::InvalidPageTree);
    }
    let visited = validate_page_tree_nodes(objects, pages)?;
    for (id, body) in objects {
        if let IndirectObjectBody::Value(PdfValue::Dictionary(dictionary)) = body {
            if (type_is(dictionary, b"Page") || type_is(dictionary, b"Pages"))
                && !visited.contains(id)
            {
                return Err(PdfError::InvalidPageTree);
            }
        }
    }
    validate_destinations_and_annotations(objects, catalog)?;
    Ok(())
}

fn validate_destinations_and_annotations(
    objects: &BTreeMap<ObjectId, IndirectObjectBody>,
    catalog: &PdfDictionary,
) -> Result<(), PdfError> {
    let page_ids: BTreeSet<_> = objects
        .iter()
        .filter_map(|(id, body)| match body {
            IndirectObjectBody::Value(PdfValue::Dictionary(dictionary))
                if type_is(dictionary, b"Page") =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect();
    let destinations = validate_destination_name_tree(catalog, &page_ids)?;
    let mut referenced_annotations = BTreeSet::new();
    for page_id in &page_ids {
        let page =
            dictionary_for(objects, *page_id).map_err(|_| PdfError::InvalidAnnotationClosure)?;
        let Some(annotations) = dict_value(page, b"Annots") else {
            continue;
        };
        let PdfValue::Array(annotations) = annotations else {
            return Err(PdfError::InvalidAnnotationClosure);
        };
        for annotation in annotations {
            let PdfValue::Reference(annotation_id) = annotation else {
                return Err(PdfError::InvalidAnnotationClosure);
            };
            if !referenced_annotations.insert(*annotation_id) {
                return Err(PdfError::InvalidAnnotationClosure);
            }
            let dictionary = dictionary_for(objects, *annotation_id)
                .map_err(|_| PdfError::InvalidAnnotationClosure)?;
            validate_link_annotation(dictionary, &destinations)?;
        }
    }
    for (id, body) in objects {
        if let IndirectObjectBody::Value(PdfValue::Dictionary(dictionary)) = body {
            if type_is(dictionary, b"Annot") && !referenced_annotations.contains(id) {
                return Err(PdfError::InvalidAnnotationClosure);
            }
        }
    }
    Ok(())
}

fn validate_destination_name_tree(
    catalog: &PdfDictionary,
    page_ids: &BTreeSet<ObjectId>,
) -> Result<BTreeSet<Vec<u8>>, PdfError> {
    let Some(names) = dict_value(catalog, b"Names") else {
        return Ok(BTreeSet::new());
    };
    let PdfValue::Dictionary(names) = names else {
        return Err(PdfError::InvalidDestinationClosure);
    };
    if names.len() != 1 {
        return Err(PdfError::InvalidDestinationClosure);
    }
    let Some(PdfValue::Dictionary(destinations)) = dict_value(names, b"Dests") else {
        return Err(PdfError::InvalidDestinationClosure);
    };
    if destinations.len() != 1 {
        return Err(PdfError::InvalidDestinationClosure);
    }
    let Some(PdfValue::Array(values)) = dict_value(destinations, b"Names") else {
        return Err(PdfError::InvalidDestinationClosure);
    };
    if values.is_empty() || values.len() % 2 != 0 {
        return Err(PdfError::InvalidDestinationClosure);
    }
    let mut result = BTreeSet::new();
    let mut previous: Option<&[u8]> = None;
    for entry in values.chunks_exact(2) {
        let PdfValue::ByteString(name) = &entry[0] else {
            return Err(PdfError::InvalidDestinationClosure);
        };
        if !AnchorId::is_valid(std::str::from_utf8(name).unwrap_or_default())
            || previous.is_some_and(|previous| previous >= name.as_slice())
            || !result.insert(name.clone())
            || !valid_destination_value(&entry[1], page_ids)
        {
            return Err(PdfError::InvalidDestinationClosure);
        }
        previous = Some(name);
    }
    Ok(result)
}

fn valid_destination_value(value: &PdfValue, page_ids: &BTreeSet<ObjectId>) -> bool {
    let PdfValue::Array(values) = value else {
        return false;
    };
    let Some(PdfValue::Reference(page)) = values.first() else {
        return false;
    };
    if !page_ids.contains(page) {
        return false;
    }
    match values.get(1) {
        Some(PdfValue::Name(view)) if view.is(b"XYZ") => {
            values.len() == 5
                && pdf_number(&values[2]).is_some()
                && pdf_number(&values[3]).is_some()
                && values[4] == PdfValue::Null
        }
        Some(PdfValue::Name(view)) if view.is(b"Fit") => values.len() == 2,
        Some(PdfValue::Name(view)) if view.is(b"FitH") => {
            values.len() == 3 && (values[2] == PdfValue::Null || pdf_number(&values[2]).is_some())
        }
        _ => false,
    }
}

fn validate_link_annotation(
    dictionary: &PdfDictionary,
    destinations: &BTreeSet<Vec<u8>>,
) -> Result<(), PdfError> {
    if dictionary.len() != 5
        || !type_is(dictionary, b"Annot")
        || !matches!(dict_value(dictionary, b"Subtype"), Some(PdfValue::Name(name)) if name.is(b"Link"))
        || !dict_value(dictionary, b"Rect").is_some_and(valid_page_box)
        || !matches!(dict_value(dictionary, b"Border"), Some(PdfValue::Array(values)) if values == &[PdfValue::Integer(0), PdfValue::Integer(0), PdfValue::Integer(0)])
    {
        return Err(PdfError::InvalidAnnotationClosure);
    }
    match (
        dict_value(dictionary, b"Dest"),
        dict_value(dictionary, b"A"),
    ) {
        (Some(PdfValue::ByteString(destination)), None) if destinations.contains(destination) => {
            Ok(())
        }
        (None, Some(PdfValue::Dictionary(action)))
            if action.len() == 2
                && matches!(dict_value(action, b"S"), Some(PdfValue::Name(name)) if name.is(b"URI"))
                && matches!(dict_value(action, b"URI"), Some(PdfValue::ByteString(uri)) if !uri.is_empty()) =>
        {
            Ok(())
        }
        _ => Err(PdfError::InvalidAnnotationClosure),
    }
}
const MAX_PDF_PAGE_TREE_DEPTH: usize = 64;

enum PageTreeWork {
    Enter {
        id: ObjectId,
        expected_parent: Option<ObjectId>,
        inherited_media_box: Option<PdfValue>,
        depth: usize,
    },
    Exit {
        id: ObjectId,
        kids: Vec<ObjectId>,
    },
}

fn validate_page_tree_nodes(
    objects: &BTreeMap<ObjectId, IndirectObjectBody>,
    root: ObjectId,
) -> Result<BTreeSet<ObjectId>, PdfError> {
    let mut pending = vec![PageTreeWork::Enter {
        id: root,
        expected_parent: None,
        inherited_media_box: None,
        depth: 1,
    }];
    let mut active = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut descendant_counts = BTreeMap::new();
    while let Some(work) = pending.pop() {
        match work {
            PageTreeWork::Enter {
                id,
                expected_parent,
                inherited_media_box,
                depth,
            } => {
                if depth > MAX_PDF_PAGE_TREE_DEPTH {
                    return Err(PdfError::PageTreeDepth);
                }
                if active.contains(&id) {
                    return Err(PdfError::PageTreeCycle);
                }
                if !visited.insert(id) {
                    return Err(PdfError::InvalidPageTree);
                }
                active.insert(id);
                let dict = dictionary_for(objects, id)?;
                if let Some(parent) = expected_parent {
                    if !matches!(dict_value(dict, b"Parent"), Some(PdfValue::Reference(found)) if *found == parent)
                    {
                        return Err(PdfError::InvalidPageTree);
                    }
                }
                let own_media_box = dict_value(dict, b"MediaBox");
                if own_media_box.is_some_and(|value| !valid_page_box(value)) {
                    return Err(PdfError::InvalidPageTree);
                }
                let effective_media_box = own_media_box.cloned().or(inherited_media_box);
                if dict_value(dict, b"CropBox").is_some_and(|value| !valid_page_box(value))
                    || dict_value(dict, b"Rotate").is_some_and(|value| {
                        !matches!(value, PdfValue::Integer(angle) if matches!(*angle, 0 | 90 | 180 | 270))
                    })
                {
                    return Err(PdfError::InvalidPageTree);
                }
                if type_is(dict, b"Page") {
                    if effective_media_box.is_none()
                        || dict_value(dict, b"Kids").is_some()
                        || dict_value(dict, b"Count").is_some()
                    {
                        return Err(PdfError::InvalidPageTree);
                    }
                    descendant_counts.insert(id, 1u32);
                    active.remove(&id);
                    continue;
                }
                if !type_is(dict, b"Pages")
                    || dict_value(dict, b"Contents").is_some()
                    || dict_value(dict, b"Annots").is_some()
                {
                    return Err(PdfError::InvalidPageTree);
                }
                let Some(PdfValue::Array(values)) = dict_value(dict, b"Kids") else {
                    return Err(PdfError::InvalidPageTree);
                };
                if values.is_empty() {
                    return Err(PdfError::InvalidPageTree);
                }
                let kids: Vec<_> = values
                    .iter()
                    .map(|value| match value {
                        PdfValue::Reference(kid) => Ok(*kid),
                        _ => Err(PdfError::InvalidPageTree),
                    })
                    .collect::<Result<_, _>>()?;
                pending.push(PageTreeWork::Exit {
                    id,
                    kids: kids.clone(),
                });
                let child_depth = depth.checked_add(1).ok_or(PdfError::PageTreeDepth)?;
                for kid in kids.into_iter().rev() {
                    pending.push(PageTreeWork::Enter {
                        id: kid,
                        expected_parent: Some(id),
                        inherited_media_box: effective_media_box.clone(),
                        depth: child_depth,
                    });
                }
            }
            PageTreeWork::Exit { id, kids } => {
                let actual = kids.iter().try_fold(0u32, |count, kid| {
                    count
                        .checked_add(
                            *descendant_counts
                                .get(kid)
                                .ok_or(PdfError::InvalidPageTree)?,
                        )
                        .ok_or(PdfError::InvalidPageTree)
                })?;
                let dict = dictionary_for(objects, id)?;
                if !matches!(dict_value(dict, b"Count"), Some(PdfValue::Integer(value)) if *value >= 0 && u32::try_from(*value).ok() == Some(actual))
                {
                    return Err(PdfError::InvalidPageTree);
                }
                descendant_counts.insert(id, actual);
                active.remove(&id);
            }
        }
    }
    Ok(visited)
}

fn valid_page_box(value: &PdfValue) -> bool {
    let PdfValue::Array(values) = value else {
        return false;
    };
    if values.len() != 4 {
        return false;
    }
    let Some(left) = pdf_number(&values[0]) else {
        return false;
    };
    let Some(bottom) = pdf_number(&values[1]) else {
        return false;
    };
    let Some(right) = pdf_number(&values[2]) else {
        return false;
    };
    let Some(top) = pdf_number(&values[3]) else {
        return false;
    };
    left < right && bottom < top
}

fn pdf_number(value: &PdfValue) -> Option<i128> {
    match value {
        PdfValue::Integer(value) => i128::from(*value).checked_mul(1_000_000_000_000),
        PdfValue::Decimal(value) => {
            let exponent = 12u8.checked_sub(value.scale)?;
            let factor = 10i128.checked_pow(u32::from(exponent))?;
            i128::from(value.coefficient).checked_mul(factor)
        }
        _ => None,
    }
}
const MAX_PDF_DIRECT_VALUE_DEPTH: usize = 64;

fn collect_references(
    body: &IndirectObjectBody,
    output: &mut BTreeSet<ObjectId>,
) -> Result<(), PdfError> {
    match body {
        IndirectObjectBody::Value(value) => collect_value_references(value, 1, output)?,
        IndirectObjectBody::Stream(stream) => {
            for value in stream.dictionary.values() {
                collect_value_references(value, 2, output)?;
            }
        }
        IndirectObjectBody::FrozenImageResource {
            alpha_mask_object: Some(alpha_mask_object),
            ..
        } => {
            output.insert(*alpha_mask_object);
        }
        IndirectObjectBody::FrozenToUnicodeCMap {
            font_program_object,
        }
        | IndirectObjectBody::FrozenCidToGidMap {
            font_program_object,
        } => {
            output.insert(*font_program_object);
        }
        IndirectObjectBody::FrozenFontProgram(_)
        | IndirectObjectBody::FrozenImageResource {
            alpha_mask_object: None,
            ..
        }
        | IndirectObjectBody::DisplayPageContent(_) => {}
    }
    Ok(())
}
fn collect_value_references(
    value: &PdfValue,
    root_depth: usize,
    output: &mut BTreeSet<ObjectId>,
) -> Result<(), PdfError> {
    if root_depth == 0 || root_depth > MAX_PDF_DIRECT_VALUE_DEPTH {
        return Err(PdfError::DirectValueDepth);
    }
    let mut pending = vec![(value, root_depth)];
    while let Some((value, depth)) = pending.pop() {
        match value {
            PdfValue::Reference(id) => {
                output.insert(*id);
            }
            PdfValue::Array(values) => {
                if values.is_empty() {
                    continue;
                }
                let child_depth = depth.checked_add(1).ok_or(PdfError::DirectValueDepth)?;
                if child_depth > MAX_PDF_DIRECT_VALUE_DEPTH {
                    return Err(PdfError::DirectValueDepth);
                }
                pending.extend(values.iter().rev().map(|value| (value, child_depth)));
            }
            PdfValue::Dictionary(dictionary) => {
                if dictionary.is_empty() {
                    continue;
                }
                let child_depth = depth.checked_add(1).ok_or(PdfError::DirectValueDepth)?;
                if child_depth > MAX_PDF_DIRECT_VALUE_DEPTH {
                    return Err(PdfError::DirectValueDepth);
                }
                pending.extend(dictionary.values().rev().map(|value| (value, child_depth)));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    fn name(value: &[u8]) -> PdfName {
        PdfName::from_bytes(value.to_vec()).unwrap()
    }
    fn valid_graph() -> (UntrustedPdfObjectGraphBuilder, ObjectId) {
        let catalog_id = ObjectId::new(1).unwrap();
        let pages_id = ObjectId::new(2).unwrap();
        let page_id = ObjectId::new(3).unwrap();
        let mut catalog = PdfDictionary::new();
        catalog.insert(name(b"Type"), PdfValue::Name(name(b"Catalog")));
        catalog.insert(name(b"Pages"), PdfValue::Reference(pages_id));
        let mut pages = PdfDictionary::new();
        pages.insert(name(b"Type"), PdfValue::Name(name(b"Pages")));
        pages.insert(
            name(b"Kids"),
            PdfValue::Array(vec![PdfValue::Reference(page_id)]),
        );
        pages.insert(name(b"Count"), PdfValue::Integer(1));
        pages.insert(
            name(b"MediaBox"),
            PdfValue::Array(vec![
                PdfValue::Integer(0),
                PdfValue::Integer(0),
                PdfValue::Integer(100),
                PdfValue::Integer(100),
            ]),
        );
        let mut page = PdfDictionary::new();
        page.insert(name(b"Type"), PdfValue::Name(name(b"Page")));
        page.insert(name(b"Parent"), PdfValue::Reference(pages_id));
        let mut builder = builder_with_max(ResourceLimits::default().max_pdf_objects);
        builder
            .insert(
                catalog_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(catalog)),
            )
            .unwrap();
        builder
            .insert(
                pages_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(pages)),
            )
            .unwrap();
        builder
            .insert(
                page_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(page)),
            )
            .unwrap();
        (builder, catalog_id)
    }
    fn builder_with_max(max_pdf_objects: u32) -> UntrustedPdfObjectGraphBuilder {
        let limits = ResourceLimits {
            max_pdf_objects,
            ..ResourceLimits::default()
        };
        UntrustedPdfObjectGraphBuilder::new(&ValidatedResourceLimits::new(limits).unwrap())
    }
    fn nested_value(depth: usize) -> PdfValue {
        assert!(depth > 0);
        let mut value = PdfValue::Null;
        for _ in 1..depth {
            value = PdfValue::Array(vec![value]);
        }
        value
    }
    fn page_tree_chain(pages_nodes: usize) -> (UntrustedPdfObjectGraphBuilder, ObjectId) {
        assert!(pages_nodes > 0);
        let catalog_id = ObjectId::new(1).unwrap();
        let first_pages_id = ObjectId::new(2).unwrap();
        let page_id = ObjectId::new(u32::try_from(pages_nodes + 2).unwrap()).unwrap();
        let mut catalog = PdfDictionary::new();
        catalog.insert(name(b"Type"), PdfValue::Name(name(b"Catalog")));
        catalog.insert(name(b"Pages"), PdfValue::Reference(first_pages_id));
        let mut builder = builder_with_max(u32::try_from(pages_nodes + 2).unwrap());
        builder
            .insert(
                catalog_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(catalog)),
            )
            .unwrap();
        for index in 0..pages_nodes {
            let id = ObjectId::new(u32::try_from(index + 2).unwrap()).unwrap();
            let kid = if index + 1 == pages_nodes {
                page_id
            } else {
                ObjectId::new(u32::try_from(index + 3).unwrap()).unwrap()
            };
            let mut pages = PdfDictionary::new();
            pages.insert(name(b"Type"), PdfValue::Name(name(b"Pages")));
            pages.insert(
                name(b"Kids"),
                PdfValue::Array(vec![PdfValue::Reference(kid)]),
            );
            pages.insert(name(b"Count"), PdfValue::Integer(1));
            if index == 0 {
                pages.insert(
                    name(b"MediaBox"),
                    PdfValue::Array(vec![
                        PdfValue::Integer(0),
                        PdfValue::Integer(0),
                        PdfValue::Integer(100),
                        PdfValue::Integer(100),
                    ]),
                );
            } else {
                pages.insert(
                    name(b"Parent"),
                    PdfValue::Reference(ObjectId::new(u32::try_from(index + 1).unwrap()).unwrap()),
                );
            }
            builder
                .insert(id, IndirectObjectBody::Value(PdfValue::Dictionary(pages)))
                .unwrap();
        }
        let mut page = PdfDictionary::new();
        page.insert(name(b"Type"), PdfValue::Name(name(b"Page")));
        page.insert(
            name(b"Parent"),
            PdfValue::Reference(ObjectId::new(u32::try_from(pages_nodes + 1).unwrap()).unwrap()),
        );
        builder
            .insert(
                page_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(page)),
            )
            .unwrap();
        (builder, catalog_id)
    }
    #[test]
    fn duplicate_insert_preserves_first_object() {
        let (mut builder, root) = valid_graph();
        assert_eq!(
            builder.insert(root, IndirectObjectBody::Value(PdfValue::Integer(2))),
            Err(PdfError::DuplicateObject)
        );
        assert!(builder.validate_untrusted(root).is_ok());
    }
    #[test]
    fn pdf_name_escapes_delimiters_and_space() {
        assert_eq!(
            PdfName::from_bytes(b"A B/C#".to_vec()).unwrap().encoded(),
            b"/A#20B#2FC#23".to_vec()
        );
    }
    #[test]
    fn decimal_is_canonical() {
        assert_eq!(PdfDecimal::new(12_300, 3).unwrap().canonical(), "12.3");
        assert_eq!(PdfDecimal::new(-5, 2).unwrap().canonical(), "-0.05");
    }
    #[test]
    fn valid_page_tree_freezes() {
        let (builder, root) = valid_graph();
        assert!(builder.validate_untrusted(root).is_ok());
    }
    #[test]
    fn page_requires_effective_media_box_and_rejects_tree_keys() {
        let (mut missing_box, root) = valid_graph();
        let pages_id = ObjectId::new(2).unwrap();
        if let Some(IndirectObjectBody::Value(PdfValue::Dictionary(pages))) =
            missing_box.objects.get_mut(&pages_id)
        {
            pages.remove(&name(b"MediaBox"));
        }
        assert_eq!(
            missing_box.validate_untrusted(root),
            Err(PdfError::InvalidPageTree)
        );

        let (mut leaf_with_kids, root) = valid_graph();
        let page_id = ObjectId::new(3).unwrap();
        if let Some(IndirectObjectBody::Value(PdfValue::Dictionary(page))) =
            leaf_with_kids.objects.get_mut(&page_id)
        {
            page.insert(name(b"Kids"), PdfValue::Array(vec![]));
        }
        assert_eq!(
            leaf_with_kids.validate_untrusted(root),
            Err(PdfError::InvalidPageTree)
        );
    }
    #[test]
    fn catalog_pages_must_reference_pages_node() {
        let catalog_id = ObjectId::new(1).unwrap();
        let page_id = ObjectId::new(2).unwrap();
        let mut catalog = PdfDictionary::new();
        catalog.insert(name(b"Type"), PdfValue::Name(name(b"Catalog")));
        catalog.insert(name(b"Pages"), PdfValue::Reference(page_id));
        let mut page = PdfDictionary::new();
        page.insert(name(b"Type"), PdfValue::Name(name(b"Page")));
        let mut builder = builder_with_max(ResourceLimits::default().max_pdf_objects);
        builder
            .insert(
                catalog_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(catalog)),
            )
            .unwrap();
        builder
            .insert(
                page_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(page)),
            )
            .unwrap();
        assert_eq!(
            builder.validate_untrusted(catalog_id),
            Err(PdfError::InvalidPageTree)
        );
    }
    #[test]
    fn root_pages_node_must_not_have_parent() {
        let (mut builder, root) = valid_graph();
        let pages_id = ObjectId::new(2).unwrap();
        let parent_id = ObjectId::new(4).unwrap();
        if let Some(IndirectObjectBody::Value(PdfValue::Dictionary(pages))) =
            builder.objects.get_mut(&pages_id)
        {
            pages.insert(name(b"Parent"), PdfValue::Reference(parent_id));
        } else {
            panic!("valid fixture must contain a Pages dictionary");
        }
        builder
            .insert(
                parent_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(PdfDictionary::new())),
            )
            .unwrap();
        assert_eq!(
            builder.validate_untrusted(root),
            Err(PdfError::InvalidPageTree)
        );
    }
    #[test]
    fn serializer_owned_stream_keys_are_rejected() {
        let (mut builder, root) = valid_graph();
        let stream_id = ObjectId::new(4).unwrap();
        let mut dictionary = PdfDictionary::new();
        dictionary.insert(name(b"Filter"), PdfValue::Name(name(b"FlateDecode")));
        builder
            .insert(
                stream_id,
                IndirectObjectBody::Stream(PdfStreamObject {
                    dictionary,
                    encoding: StreamEncoding::Flate,
                    raw_data: vec![],
                }),
            )
            .unwrap();
        assert_eq!(
            builder.validate_untrusted(root),
            Err(PdfError::ReservedStreamKey)
        );
    }

    #[test]
    fn sparse_and_unreachable_objects_are_rejected() {
        let (mut sparse, root) = valid_graph();
        sparse
            .insert(
                ObjectId::new(5).unwrap(),
                IndirectObjectBody::Value(PdfValue::Null),
            )
            .unwrap();
        assert_eq!(
            sparse.validate_untrusted(root),
            Err(PdfError::SparseObjectId)
        );

        let (mut orphan, root) = valid_graph();
        let orphan_id = ObjectId::new(4).unwrap();
        orphan
            .insert(orphan_id, IndirectObjectBody::Value(PdfValue::Null))
            .unwrap();
        assert_eq!(
            orphan.validate_untrusted(root),
            Err(PdfError::UnreachableObject(orphan_id))
        );
    }

    #[test]
    fn object_limit_is_checked_before_insertion() {
        let (exact, root) = valid_graph();
        assert!(exact.validate_untrusted(root).is_ok());

        let (mut limited, _) = valid_graph();
        limited.max_objects = 3;
        assert_eq!(
            limited.insert(
                ObjectId::new(4).unwrap(),
                IndirectObjectBody::Value(PdfValue::Null),
            ),
            Err(PdfError::ObjectLimit)
        );
        assert_eq!(limited.objects.len(), 3);
    }

    #[test]
    fn dense_allocator_accepts_the_last_u32_object_id_and_rejects_max_plus_one() {
        let mut exact = DenseObjectAllocator {
            next: u64::from(u32::MAX),
            required: u32::MAX,
        };
        assert_eq!(exact.allocate().unwrap().get(), u32::MAX);
        assert_eq!(exact.finish(), Ok(()));

        let mut exhausted = DenseObjectAllocator {
            next: u64::from(u32::MAX) + 1,
            required: u32::MAX,
        };
        assert_eq!(exhausted.allocate(), Err(PdfError::ObjectCountOverflow));
    }

    #[test]
    fn serializer_receipt_binds_the_exact_effective_config_fingerprint() {
        let (builder, root) = valid_graph();
        let graph = FrozenPdfGraph {
            graph: builder.validate_untrusted(root).unwrap(),
            selected_layout_fingerprint: LayoutStateFingerprint::from_untrusted_bytes([3; 32]),
            pages: vec![FrozenPageGeometry {
                page_index: 0,
                master_id: MasterId::new("default").unwrap(),
                width: PositiveLength::new(Length::from_raw(1).unwrap()).unwrap(),
                height: PositiveLength::new(Length::from_raw(1).unwrap()).unwrap(),
            }],
            page_count: 1,
            object_count: 3,
            font_bindings: vec![],
            image_bindings: vec![],
        };
        let expected = EffectiveConfigFingerprint::from_untrusted_bytes([5; 32]);
        let different = EffectiveConfigFingerprint::from_untrusted_bytes([6; 32]);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let receipt = VerifiedPdfSerializerReceiptOwner::new()
            .issue(
                &graph,
                b"%PDF-1.7\n".to_vec(),
                PdfStreamCompression::None,
                expected,
                &limits,
            )
            .unwrap();
        assert_eq!(receipt.config_fingerprint(), expected);
        assert_ne!(receipt.config_fingerprint(), different);
    }

    #[test]
    fn composite_font_blueprint_allocates_six_dense_objects() {
        let blueprint = [
            PdfFontIndirectObjectRole::Type0Font,
            PdfFontIndirectObjectRole::CidFont,
            PdfFontIndirectObjectRole::FontDescriptor,
            PdfFontIndirectObjectRole::EmbeddedFontProgram,
            PdfFontIndirectObjectRole::ToUnicodeCMap,
            PdfFontIndirectObjectRole::CidToGidMap,
        ];
        let mut allocator = DenseObjectAllocator::new(6);
        let ids = FontObjectIds::allocate_blueprint(&blueprint, &mut allocator).unwrap();
        allocator.finish().unwrap();
        assert_eq!(
            [
                ids.type0.get(),
                ids.cid_font.get(),
                ids.descriptor.get(),
                ids.font_program.get(),
                ids.to_unicode.get(),
                ids.cid_to_gid.get(),
            ],
            [1, 2, 3, 4, 5, 6]
        );
    }

    #[test]
    fn pdf_uses_the_postscript_name_bound_to_the_verified_subset_program() {
        let first = subset_base_font_name("AAAAAA+Typaxis").unwrap();
        let second = subset_base_font_name("AAAAAB+Typaxis").unwrap();
        assert_eq!(first.0, b"AAAAAA+Typaxis");
        assert_eq!(second.0, b"AAAAAB+Typaxis");
        assert_ne!(first, second);
    }

    #[test]
    fn destination_and_internal_annotation_form_a_closed_graph() {
        let (mut builder, root) = valid_graph();
        let page_id = ObjectId::new(3).unwrap();
        let annotation_id = ObjectId::new(4).unwrap();

        let destination = AnchorId::new("target").unwrap();
        let mut destination_tree = PdfDictionary::new();
        destination_tree.insert(
            name(b"Names"),
            PdfValue::Array(vec![
                PdfValue::ByteString(destination.as_str().as_bytes().to_vec()),
                PdfValue::Array(vec![
                    PdfValue::Reference(page_id),
                    PdfValue::Name(name(b"Fit")),
                ]),
            ]),
        );
        let mut names = PdfDictionary::new();
        names.insert(name(b"Dests"), PdfValue::Dictionary(destination_tree));
        if let Some(IndirectObjectBody::Value(PdfValue::Dictionary(catalog))) =
            builder.objects.get_mut(&root)
        {
            catalog.insert(name(b"Names"), PdfValue::Dictionary(names));
        } else {
            panic!("valid fixture must contain a catalog");
        }
        if let Some(IndirectObjectBody::Value(PdfValue::Dictionary(page))) =
            builder.objects.get_mut(&page_id)
        {
            page.insert(
                name(b"Annots"),
                PdfValue::Array(vec![PdfValue::Reference(annotation_id)]),
            );
        } else {
            panic!("valid fixture must contain a page");
        }
        let mut annotation = PdfDictionary::new();
        annotation.insert(name(b"Type"), PdfValue::Name(name(b"Annot")));
        annotation.insert(name(b"Subtype"), PdfValue::Name(name(b"Link")));
        annotation.insert(
            name(b"Rect"),
            PdfValue::Array(vec![
                PdfValue::Integer(1),
                PdfValue::Integer(2),
                PdfValue::Integer(3),
                PdfValue::Integer(4),
            ]),
        );
        annotation.insert(
            name(b"Border"),
            PdfValue::Array(vec![
                PdfValue::Integer(0),
                PdfValue::Integer(0),
                PdfValue::Integer(0),
            ]),
        );
        annotation.insert(
            name(b"Dest"),
            PdfValue::ByteString(destination.as_str().as_bytes().to_vec()),
        );
        builder
            .insert(
                annotation_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(annotation)),
            )
            .unwrap();
        assert!(builder.validate_untrusted(root).is_ok());
    }

    #[test]
    fn annotation_coordinates_are_converted_outside_the_content_ctm() {
        let unit = |points: i64| Length::from_raw(points * 65_536).unwrap();
        let positive = |points: i64| PositiveLength::new(unit(points)).unwrap();
        let converted = annotation_rectangle(
            Rect::new(unit(10), unit(20), positive(30), positive(40)),
            positive(100),
        )
        .unwrap();
        let PdfValue::Array(values) = &converted else {
            panic!("annotation rectangle must be an array");
        };
        let scaled: Vec<_> = values
            .iter()
            .map(|value| pdf_number(value).unwrap())
            .collect();
        assert_eq!(
            scaled,
            [10, 40, 40, 80].map(|value| i128::from(value) * 1_000_000_000_000)
        );
    }

    #[test]
    fn direct_value_depth_64_is_inclusive_and_stream_dictionary_counts_as_root() {
        let mut references = BTreeSet::new();
        assert_eq!(
            collect_references(
                &IndirectObjectBody::Value(nested_value(MAX_PDF_DIRECT_VALUE_DEPTH)),
                &mut references,
            ),
            Ok(())
        );
        assert_eq!(
            collect_references(
                &IndirectObjectBody::Value(nested_value(MAX_PDF_DIRECT_VALUE_DEPTH + 1)),
                &mut references,
            ),
            Err(PdfError::DirectValueDepth)
        );

        let mut exact_dictionary = PdfDictionary::new();
        exact_dictionary.insert(
            name(b"Nested"),
            nested_value(MAX_PDF_DIRECT_VALUE_DEPTH - 1),
        );
        assert_eq!(
            collect_references(
                &IndirectObjectBody::Stream(PdfStreamObject {
                    dictionary: exact_dictionary,
                    encoding: StreamEncoding::None,
                    raw_data: vec![],
                }),
                &mut references,
            ),
            Ok(())
        );
        let mut too_deep_dictionary = PdfDictionary::new();
        too_deep_dictionary.insert(name(b"Nested"), nested_value(MAX_PDF_DIRECT_VALUE_DEPTH));
        assert_eq!(
            collect_references(
                &IndirectObjectBody::Stream(PdfStreamObject {
                    dictionary: too_deep_dictionary,
                    encoding: StreamEncoding::None,
                    raw_data: vec![],
                }),
                &mut references,
            ),
            Err(PdfError::DirectValueDepth)
        );
    }

    #[test]
    fn page_tree_depth_64_is_inclusive_and_max_plus_one_is_rejected_iteratively() {
        // 63 Pages nodes followed by one Page leaf: root Pages depth is 1 and
        // the leaf is exactly depth 64.
        let (exact, root) = page_tree_chain(MAX_PDF_PAGE_TREE_DEPTH - 1);
        assert!(exact.validate_untrusted(root).is_ok());
        let (too_deep, root) = page_tree_chain(MAX_PDF_PAGE_TREE_DEPTH);
        assert_eq!(
            too_deep.validate_untrusted(root),
            Err(PdfError::PageTreeDepth)
        );
    }

    #[test]
    fn destination_name_preallocation_rejects_arithmetic_overflow() {
        assert_eq!(destination_name_value_count(0), Ok(0));
        assert_eq!(
            destination_name_value_count(usize::MAX / 2),
            Ok(usize::MAX - 1)
        );
        assert_eq!(
            destination_name_value_count(usize::MAX / 2 + 1),
            Err(PdfError::ObjectCountOverflow)
        );
    }
}
