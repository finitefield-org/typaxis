use std::collections::BTreeMap;

use typaxis_core::{push_jcs_string, sha256, EngineIdentity, ValidatedResourceLimits};
use typaxis_display_list::{
    BookNavigationSelectedReceipt, DestinationView, MarkedContentOwner, MarkedContentPlanReceipt,
    SelectedStructureBindingReceipt, StructureArtifactClass, StructureNodeId,
    StructureParentTreeValue, StructureRegistryReceipt, StructureRole,
};
use typaxis_syntax::{
    StagingAccessibilityProfileAuthorization, StagingBookNavigationProfileAuthorization,
    ValidatedStagingBookNavigation,
};

pub const TAGGED_PDF_ALGORITHM: &str = "typaxis.tagged-pdf/1";
pub const TAGGED_PDF_XMP_ALGORITHM: &str = "typaxis.book-xmp/2";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaggedPdfObjectObservation {
    object_number: u32,
    role: String,
    sha256: [u8; 32],
}

impl TaggedPdfObjectObservation {
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
pub struct TaggedPdfObservation {
    profile_sha256: [u8; 32],
    structure_registry_sha256: [u8; 32],
    selected_binding_sha256: [u8; 32],
    marked_content_sha256: [u8; 32],
    book_navigation_sha256: [u8; 32],
    document_language: String,
    catalog_object: u32,
    structure_tree_root_object: u32,
    parent_tree_object: u32,
    id_tree_object: Option<u32>,
    structure_element_count: u32,
    marked_content_count: u32,
    artifact_count: u32,
    link_annotation_count: u32,
    outline_count: u32,
    xmp_sha256: [u8; 32],
    objects: Vec<TaggedPdfObjectObservation>,
    pdf_sha256: [u8; 32],
    pdf_byte_length: u64,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl TaggedPdfObservation {
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
    pub fn document_language(&self) -> &str {
        &self.document_language
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
    pub const fn structure_element_count(&self) -> u32 {
        self.structure_element_count
    }
    pub const fn marked_content_count(&self) -> u32 {
        self.marked_content_count
    }
    pub const fn artifact_count(&self) -> u32 {
        self.artifact_count
    }
    pub const fn link_annotation_count(&self) -> u32 {
        self.link_annotation_count
    }
    pub const fn outline_count(&self) -> u32 {
        self.outline_count
    }
    pub const fn xmp_sha256(&self) -> [u8; 32] {
        self.xmp_sha256
    }
    pub fn objects(&self) -> &[TaggedPdfObjectObservation] {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTaggedPdf {
    bytes: Vec<u8>,
    observation: TaggedPdfObservation,
}

impl StagingTaggedPdf {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub const fn observation(&self) -> &TaggedPdfObservation {
        &self.observation
    }

    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        &self,
        navigation: &ValidatedStagingBookNavigation,
        book_profile: &StagingBookNavigationProfileAuthorization,
        accessibility: &StagingAccessibilityProfileAuthorization,
        book: &BookNavigationSelectedReceipt,
        registry: &StructureRegistryReceipt,
        binding: &SelectedStructureBindingReceipt,
        marked: &MarkedContentPlanReceipt,
        limits: &ValidatedResourceLimits,
        engine: &EngineIdentity,
    ) -> Result<(), TaggedPdfError> {
        let observed = build_tagged_pdf(
            navigation,
            book_profile,
            accessibility,
            book,
            registry,
            binding,
            marked,
            limits,
            engine,
        )?;
        if self != &observed {
            return Err(TaggedPdfError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaggedPdfError {
    ReceiptMismatch,
    NavigationMismatch,
    StructureMismatch,
    MarkedContentMismatch,
    AnnotationMismatch,
    OutlineMismatch,
    ObjectLimit,
    OutputLimit,
    SpoolLimit,
    AllocationFailure,
}

impl std::fmt::Display for TaggedPdfError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReceiptMismatch => formatter.write_str("I9190: tagged-PDF receipt mismatch"),
            Self::NavigationMismatch => {
                formatter.write_str("I9190: tagged-PDF navigation mismatch")
            }
            Self::StructureMismatch => formatter.write_str("I9190: PDF structure tree mismatch"),
            Self::MarkedContentMismatch => {
                formatter.write_str("I9190: PDF marked-content mismatch")
            }
            Self::AnnotationMismatch => formatter.write_str("I9190: PDF Link/OBJR mismatch"),
            Self::OutlineMismatch => formatter.write_str("I9190: PDF outline/structure mismatch"),
            Self::ObjectLimit => formatter.write_str("G6100: tagged-PDF object limit exceeded"),
            Self::OutputLimit => formatter.write_str("D8101: tagged-PDF output limit exceeded"),
            Self::SpoolLimit => formatter.write_str("D8101: tagged-PDF spool limit exceeded"),
            Self::AllocationFailure => formatter.write_str("G6100: tagged-PDF allocation failed"),
        }
    }
}

impl std::error::Error for TaggedPdfError {}

#[derive(Clone, Debug)]
struct TaggedObjectPlan {
    object_count: u32,
    content_objects: Vec<u32>,
    page_objects: Vec<u32>,
    annotation_start: u32,
    info_object: u32,
    metadata_object: u32,
    outline_root_object: Option<u32>,
    outline_item_start: Option<u32>,
    structure_tree_root_object: u32,
    parent_tree_object: u32,
    id_tree_object: Option<u32>,
    structure_element_start: u32,
}

impl TaggedObjectPlan {
    fn new(
        book: &BookNavigationSelectedReceipt,
        registry: &StructureRegistryReceipt,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, TaggedPdfError> {
        let page_count =
            u32::try_from(book.pages().len()).map_err(|_| TaggedPdfError::ObjectLimit)?;
        let annotation_count =
            u32::try_from(book.links().len()).map_err(|_| TaggedPdfError::ObjectLimit)?;
        let outline_count =
            u32::try_from(book.entries().len()).map_err(|_| TaggedPdfError::ObjectLimit)?;
        let structure_count =
            u32::try_from(registry.nodes().len()).map_err(|_| TaggedPdfError::ObjectLimit)?;
        let annotation_start = checked_add(4, checked_mul(page_count, 2)?)?;
        let info_object = checked_add(annotation_start, annotation_count)?;
        let metadata_object = checked_add(info_object, 1)?;
        let (outline_root_object, outline_item_start, after_outlines) = if outline_count == 0 {
            (None, None, checked_add(metadata_object, 1)?)
        } else {
            let root = checked_add(metadata_object, 1)?;
            let start = checked_add(root, 1)?;
            (Some(root), Some(start), checked_add(start, outline_count)?)
        };
        let structure_tree_root_object = after_outlines;
        let parent_tree_object = checked_add(structure_tree_root_object, 1)?;
        let id_tree_object = registry
            .nodes()
            .iter()
            .any(|node| node.structure_id().is_some())
            .then(|| checked_add(parent_tree_object, 1))
            .transpose()?;
        let structure_element_start = checked_add(id_tree_object.unwrap_or(parent_tree_object), 1)?;
        let object_count = structure_element_start
            .checked_add(structure_count)
            .and_then(|value| value.checked_sub(1))
            .ok_or(TaggedPdfError::ObjectLimit)?;
        if structure_count == 0 || object_count > limits.get().max_pdf_objects {
            return Err(TaggedPdfError::ObjectLimit);
        }
        let mut content_objects = Vec::new();
        let mut page_objects = Vec::new();
        content_objects
            .try_reserve_exact(page_count as usize)
            .map_err(|_| TaggedPdfError::AllocationFailure)?;
        page_objects
            .try_reserve_exact(page_count as usize)
            .map_err(|_| TaggedPdfError::AllocationFailure)?;
        for page in 0..page_count {
            let content = checked_add(4, checked_mul(page, 2)?)?;
            content_objects.push(content);
            page_objects.push(checked_add(content, 1)?);
        }
        Ok(Self {
            object_count,
            content_objects,
            page_objects,
            annotation_start,
            info_object,
            metadata_object,
            outline_root_object,
            outline_item_start,
            structure_tree_root_object,
            parent_tree_object,
            id_tree_object,
            structure_element_start,
        })
    }

    fn annotation_object(&self, annotation_id: u32) -> Result<u32, TaggedPdfError> {
        checked_add(self.annotation_start, annotation_id)
    }
    fn outline_object(&self, outline_id: u32) -> Result<u32, TaggedPdfError> {
        checked_add(
            self.outline_item_start
                .ok_or(TaggedPdfError::OutlineMismatch)?,
            outline_id,
        )
    }
    fn structure_object(&self, structure_node_id: StructureNodeId) -> Result<u32, TaggedPdfError> {
        checked_add(self.structure_element_start, structure_node_id.get())
    }
}

fn checked_add(left: u32, right: u32) -> Result<u32, TaggedPdfError> {
    left.checked_add(right).ok_or(TaggedPdfError::ObjectLimit)
}

fn checked_mul(left: u32, right: u32) -> Result<u32, TaggedPdfError> {
    left.checked_mul(right).ok_or(TaggedPdfError::ObjectLimit)
}

struct TaggedObjects {
    values: BTreeMap<u32, (String, Vec<u8>)>,
    spool_bytes: u64,
    spool_limit: u64,
}

impl TaggedObjects {
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
    ) -> Result<(), TaggedPdfError> {
        let next = self
            .spool_bytes
            .checked_add(value.len() as u64)
            .ok_or(TaggedPdfError::SpoolLimit)?;
        if next > self.spool_limit {
            return Err(TaggedPdfError::SpoolLimit);
        }
        if self.values.insert(number, (role.into(), value)).is_some() {
            return Err(TaggedPdfError::ReceiptMismatch);
        }
        self.spool_bytes = next;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn write_staging_tagged_pdf(
    navigation: &ValidatedStagingBookNavigation,
    book_profile: &StagingBookNavigationProfileAuthorization,
    accessibility: &StagingAccessibilityProfileAuthorization,
    book: &BookNavigationSelectedReceipt,
    registry: &StructureRegistryReceipt,
    binding: &SelectedStructureBindingReceipt,
    marked: &MarkedContentPlanReceipt,
    limits: &ValidatedResourceLimits,
    engine: &EngineIdentity,
) -> Result<StagingTaggedPdf, TaggedPdfError> {
    build_tagged_pdf(
        navigation,
        book_profile,
        accessibility,
        book,
        registry,
        binding,
        marked,
        limits,
        engine,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_tagged_pdf(
    navigation: &ValidatedStagingBookNavigation,
    book_profile: &StagingBookNavigationProfileAuthorization,
    accessibility: &StagingAccessibilityProfileAuthorization,
    book: &BookNavigationSelectedReceipt,
    registry: &StructureRegistryReceipt,
    binding: &SelectedStructureBindingReceipt,
    marked: &MarkedContentPlanReceipt,
    limits: &ValidatedResourceLimits,
    engine: &EngineIdentity,
) -> Result<StagingTaggedPdf, TaggedPdfError> {
    book.verify(navigation, book_profile, limits)
        .map_err(|_| TaggedPdfError::NavigationMismatch)?;
    marked
        .verify(registry, binding, accessibility, limits)
        .map_err(|_| TaggedPdfError::MarkedContentMismatch)?;
    validate_cross_closure(book, registry, binding, marked)?;
    let plan = TaggedObjectPlan::new(book, registry, limits)?;
    let xmp = encode_xmp(navigation, engine);
    let mut objects = TaggedObjects::new(limits.get().max_spool_bytes);

    emit_catalog(&mut objects, navigation, &plan)?;
    emit_pages_tree(&mut objects, book, &plan)?;
    emit_destinations(&mut objects, book, &plan)?;
    emit_page_content_and_pages(&mut objects, book, marked, &plan)?;
    emit_annotations(&mut objects, book, registry, marked, &plan)?;
    objects.insert(
        plan.info_object,
        "info",
        encode_info(navigation, engine)?.into_bytes(),
    )?;
    objects.insert(
        plan.metadata_object,
        "metadata",
        stream_object(b"/Type /Metadata /Subtype /XML ", xmp.as_bytes()),
    )?;
    emit_outlines(&mut objects, book, registry, &plan)?;
    emit_structure_tree(&mut objects, registry, marked, &plan)?;

    if objects.values.len() != plan.object_count as usize
        || objects.values.keys().copied().ne(1..=plan.object_count)
    {
        return Err(TaggedPdfError::ReceiptMismatch);
    }
    let observations = objects
        .values
        .iter()
        .map(|(number, (role, value))| TaggedPdfObjectObservation {
            object_number: *number,
            role: role.clone(),
            sha256: sha256(value),
        })
        .collect::<Vec<_>>();
    let bytes = serialize_pdf(
        &objects.values,
        plan.object_count,
        plan.info_object,
        limits
            .get()
            .max_output_bytes
            .min(limits.get().max_spool_bytes),
    )?;
    let marked_content_count = marked
        .pages()
        .iter()
        .try_fold(0u32, |sum, page| {
            sum.checked_add(page.marked_content_count())
        })
        .ok_or(TaggedPdfError::MarkedContentMismatch)?;
    let artifact_count = marked
        .pages()
        .iter()
        .try_fold(0u32, |sum, page| sum.checked_add(page.artifact_count()))
        .ok_or(TaggedPdfError::MarkedContentMismatch)?;
    let mut observation = TaggedPdfObservation {
        profile_sha256: accessibility.profile_receipt_fingerprint(),
        structure_registry_sha256: registry.fingerprint(),
        selected_binding_sha256: binding.fingerprint(),
        marked_content_sha256: marked.fingerprint(),
        book_navigation_sha256: book.fingerprint(),
        document_language: navigation.languages().document_language().to_owned(),
        catalog_object: 1,
        structure_tree_root_object: plan.structure_tree_root_object,
        parent_tree_object: plan.parent_tree_object,
        id_tree_object: plan.id_tree_object,
        structure_element_count: u32::try_from(registry.nodes().len())
            .map_err(|_| TaggedPdfError::ObjectLimit)?,
        marked_content_count,
        artifact_count,
        link_annotation_count: u32::try_from(marked.annotations().len())
            .map_err(|_| TaggedPdfError::ObjectLimit)?,
        outline_count: u32::try_from(book.entries().len())
            .map_err(|_| TaggedPdfError::ObjectLimit)?,
        xmp_sha256: sha256(xmp.as_bytes()),
        objects: observations,
        pdf_sha256: sha256(&bytes),
        pdf_byte_length: bytes.len() as u64,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    observation.canonical_jcs = encode_observation(&observation);
    observation.fingerprint = sha256(observation.canonical_jcs.as_bytes());
    Ok(StagingTaggedPdf { bytes, observation })
}

fn validate_cross_closure(
    book: &BookNavigationSelectedReceipt,
    registry: &StructureRegistryReceipt,
    binding: &SelectedStructureBindingReceipt,
    marked: &MarkedContentPlanReceipt,
) -> Result<(), TaggedPdfError> {
    if book.pages().len() != binding.pages().len()
        || book
            .pages()
            .iter()
            .zip(binding.pages())
            .any(|(left, right)| {
                left.page_index != right.page_index
                    || left.width_raw != right.width_raw
                    || left.height_raw != right.height_raw
            })
        || marked.pages().len() != book.pages().len()
        || marked.annotations().len() != book.links().len()
    {
        return Err(TaggedPdfError::NavigationMismatch);
    }
    for (annotation, link) in marked.annotations().iter().zip(book.links()) {
        let structure = registry
            .source_node(link.owner_node_id())
            .ok_or(TaggedPdfError::AnnotationMismatch)?;
        if structure.role() != StructureRole::Link
            || structure.structure_node_id() != annotation.structure_node_id()
            || link.page_index() != annotation.page_index()
        {
            return Err(TaggedPdfError::AnnotationMismatch);
        }
    }
    for entry in book.entries() {
        let structure = registry
            .source_node(entry.source_node_id())
            .ok_or(TaggedPdfError::OutlineMismatch)?;
        if !matches!(
            structure.role(),
            StructureRole::Heading1
                | StructureRole::Heading2
                | StructureRole::Heading3
                | StructureRole::Heading4
                | StructureRole::Heading5
                | StructureRole::Heading6
                | StructureRole::Result
                | StructureRole::Proof
                | StructureRole::Exercise
        ) {
            return Err(TaggedPdfError::OutlineMismatch);
        }
    }
    Ok(())
}

fn emit_catalog(
    objects: &mut TaggedObjects,
    navigation: &ValidatedStagingBookNavigation,
    plan: &TaggedObjectPlan,
) -> Result<(), TaggedPdfError> {
    let mut value = format!(
        "<< /Type /Catalog /Pages 2 0 R /Names << /Dests 3 0 R >> /Lang <{}> /Metadata {} 0 R /MarkInfo << /Marked true >> /ViewerPreferences << /DisplayDocTitle true >> /StructTreeRoot {} 0 R",
        utf16be_hex(navigation.languages().document_language())?,
        plan.metadata_object,
        plan.structure_tree_root_object,
    );
    if let Some(outlines) = plan.outline_root_object {
        value.push_str(&format!(" /Outlines {outlines} 0 R"));
    }
    value.push_str(" >>");
    objects.insert(1, "catalog", value.into_bytes())
}

fn emit_pages_tree(
    objects: &mut TaggedObjects,
    book: &BookNavigationSelectedReceipt,
    plan: &TaggedObjectPlan,
) -> Result<(), TaggedPdfError> {
    let mut value = format!("<< /Type /Pages /Count {} /Kids [", book.pages().len());
    for page in &plan.page_objects {
        value.push_str(&format!("{page} 0 R "));
    }
    value.push_str("] >>");
    objects.insert(2, "pages", value.into_bytes())
}

fn emit_destinations(
    objects: &mut TaggedObjects,
    book: &BookNavigationSelectedReceipt,
    plan: &TaggedObjectPlan,
) -> Result<(), TaggedPdfError> {
    let mut value = String::from("<< /Names [");
    for binding in book.destinations() {
        let page = book
            .pages()
            .get(binding.destination.page_index as usize)
            .ok_or(TaggedPdfError::NavigationMismatch)?;
        let page_object = *plan
            .page_objects
            .get(binding.destination.page_index as usize)
            .ok_or(TaggedPdfError::NavigationMismatch)?;
        value.push_str(&pdf_literal(binding.destination.anchor_id.as_str()));
        value.push_str(&format!(" [{page_object} 0 R "));
        push_pdf_view(&mut value, &binding.destination.view, page.height_raw)?;
        value.push_str("] ");
    }
    value.push_str("] >>");
    objects.insert(3, "destination_name_tree", value.into_bytes())
}

fn emit_page_content_and_pages(
    objects: &mut TaggedObjects,
    book: &BookNavigationSelectedReceipt,
    marked: &MarkedContentPlanReceipt,
    plan: &TaggedObjectPlan,
) -> Result<(), TaggedPdfError> {
    for page in book.pages() {
        let page_index = page.page_index as usize;
        let content_object = *plan
            .content_objects
            .get(page_index)
            .ok_or(TaggedPdfError::MarkedContentMismatch)?;
        let page_object = *plan
            .page_objects
            .get(page_index)
            .ok_or(TaggedPdfError::MarkedContentMismatch)?;
        let mut content = Vec::new();
        for record in marked
            .records()
            .iter()
            .filter(|record| record.page_index() == page.page_index)
        {
            match record.owner() {
                MarkedContentOwner::Structure(owner) => {
                    let mut properties = format!("<< /MCID {}", owner.mcid());
                    if owner.role() == StructureRole::Span {
                        if let Some(actual_text) = record.actual_text() {
                            properties
                                .push_str(&format!(" /ActualText <{}>", utf16be_hex(actual_text)?));
                        }
                        if let Some(language) = record.language() {
                            properties.push_str(&format!(" /Lang <{}>", utf16be_hex(language)?));
                        }
                    }
                    properties.push_str(" >>");
                    content.extend_from_slice(
                        format!("/{} {properties} BDC\n", owner.role().pdf_name()).as_bytes(),
                    );
                    let nested = owner.role() != StructureRole::Span
                        && (record.actual_text().is_some() || record.language().is_some());
                    if nested {
                        let mut nested_properties = String::from("<<");
                        if let Some(actual_text) = record.actual_text() {
                            nested_properties
                                .push_str(&format!(" /ActualText <{}>", utf16be_hex(actual_text)?));
                        }
                        if let Some(language) = record.language() {
                            nested_properties
                                .push_str(&format!(" /Lang <{}>", utf16be_hex(language)?));
                        }
                        nested_properties.push_str(" >>");
                        content.extend_from_slice(
                            format!("/Span {nested_properties} BDC\n").as_bytes(),
                        );
                    }
                    for _ in record.selected_paint_ids() {
                        content.extend_from_slice(b"0 0 m 0 0 l S\n");
                    }
                    content.extend_from_slice(b"EMC\n");
                    if nested {
                        content.extend_from_slice(b"EMC\n");
                    }
                }
                MarkedContentOwner::Artifact(owner) => {
                    let properties = artifact_properties(owner.class());
                    content.extend_from_slice(format!("/Artifact {properties} BDC\n").as_bytes());
                    for _ in record.selected_paint_ids() {
                        content.extend_from_slice(b"0 0 m 0 0 l S\n");
                    }
                    content.extend_from_slice(b"EMC\n");
                }
            }
        }
        objects.insert(
            content_object,
            format!("page_content:{}", page.page_index),
            stream_object(b"", &content),
        )?;
        let page_annotations = marked
            .annotations()
            .iter()
            .filter(|annotation| annotation.page_index() == page.page_index)
            .collect::<Vec<_>>();
        let marked_page = marked
            .pages()
            .get(page_index)
            .ok_or(TaggedPdfError::MarkedContentMismatch)?;
        let mut dictionary = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << >> /Contents {} 0 R",
            pdf_number(page.width_raw),
            pdf_number(page.height_raw),
            content_object,
        );
        if let Some(key) = marked_page.structure_parent_key() {
            dictionary.push_str(&format!(" /StructParents {key}"));
        }
        if !page_annotations.is_empty() {
            dictionary.push_str(" /Tabs /S");
            dictionary.push_str(" /Annots [");
            for annotation in page_annotations {
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

fn artifact_properties(class: StructureArtifactClass) -> &'static str {
    match class {
        StructureArtifactClass::Pagination => "<< /Type /Pagination >>",
        StructureArtifactClass::PaginationHeader => "<< /Type /Pagination /Subtype /Header >>",
        StructureArtifactClass::PaginationFooter => "<< /Type /Pagination /Subtype /Footer >>",
        StructureArtifactClass::Layout => "<< /Type /Layout >>",
    }
}

fn emit_annotations(
    objects: &mut TaggedObjects,
    book: &BookNavigationSelectedReceipt,
    registry: &StructureRegistryReceipt,
    marked: &MarkedContentPlanReceipt,
    plan: &TaggedObjectPlan,
) -> Result<(), TaggedPdfError> {
    for (annotation, link) in marked.annotations().iter().zip(book.links()) {
        let page = book
            .pages()
            .get(link.page_index() as usize)
            .ok_or(TaggedPdfError::AnnotationMismatch)?;
        let page_object = *plan
            .page_objects
            .get(link.page_index() as usize)
            .ok_or(TaggedPdfError::AnnotationMismatch)?;
        let node = registry
            .node(annotation.structure_node_id())
            .ok_or(TaggedPdfError::AnnotationMismatch)?;
        if node.role() != StructureRole::Link
            || node.accessible_name() != Some(annotation.accessible_name())
        {
            return Err(TaggedPdfError::AnnotationMismatch);
        }
        let right = link
            .x_raw()
            .checked_add(link.width_raw())
            .ok_or(TaggedPdfError::AnnotationMismatch)?;
        let logical_bottom = link
            .y_raw()
            .checked_add(link.height_raw())
            .ok_or(TaggedPdfError::AnnotationMismatch)?;
        let bottom = page
            .height_raw
            .checked_sub(logical_bottom)
            .ok_or(TaggedPdfError::AnnotationMismatch)?;
        let top = page
            .height_raw
            .checked_sub(link.y_raw())
            .ok_or(TaggedPdfError::AnnotationMismatch)?;
        let value = format!(
            "<< /Type /Annot /Subtype /Link /P {page_object} 0 R /Rect [{} {} {} {}] /Border [0 0 0] /Dest {} /Contents <{}> /StructParent {} >>",
            pdf_number(link.x_raw()),
            pdf_number(bottom),
            pdf_number(right),
            pdf_number(top),
            pdf_literal(link.destination().as_str()),
            utf16be_hex(annotation.accessible_name())?,
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

fn emit_outlines(
    objects: &mut TaggedObjects,
    book: &BookNavigationSelectedReceipt,
    registry: &StructureRegistryReceipt,
    plan: &TaggedObjectPlan,
) -> Result<(), TaggedPdfError> {
    let Some(root_object) = plan.outline_root_object else {
        return Ok(());
    };
    let entries = book.entries();
    let mut children = BTreeMap::<Option<u32>, Vec<u32>>::new();
    for (index, entry) in entries.iter().enumerate() {
        if usize::try_from(entry.outline_id()) != Ok(index) {
            return Err(TaggedPdfError::OutlineMismatch);
        }
        children
            .entry(entry.parent_outline_id())
            .or_default()
            .push(entry.outline_id());
    }
    let top = children
        .get(&None)
        .filter(|values| !values.is_empty())
        .ok_or(TaggedPdfError::OutlineMismatch)?;
    objects.insert(
        root_object,
        "outline_root",
        format!(
            "<< /Type /Outlines /First {} 0 R /Last {} 0 R /Count {} >>",
            plan.outline_object(*top.first().ok_or(TaggedPdfError::OutlineMismatch)?)?,
            plan.outline_object(*top.last().ok_or(TaggedPdfError::OutlineMismatch)?)?,
            entries.len(),
        )
        .into_bytes(),
    )?;
    for entry in entries {
        let siblings = children
            .get(&entry.parent_outline_id())
            .ok_or(TaggedPdfError::OutlineMismatch)?;
        let position = siblings
            .iter()
            .position(|value| *value == entry.outline_id())
            .ok_or(TaggedPdfError::OutlineMismatch)?;
        let parent = entry
            .parent_outline_id()
            .map(|value| plan.outline_object(value))
            .transpose()?
            .unwrap_or(root_object);
        let structure = registry
            .source_node(entry.source_node_id())
            .ok_or(TaggedPdfError::OutlineMismatch)?;
        let mut value = format!(
            "<< /Title <{}> /Parent {parent} 0 R /Dest {} /SE {} 0 R",
            utf16be_hex(entry.label())?,
            pdf_literal(entry.destination().anchor_id.as_str()),
            plan.structure_object(structure.structure_node_id())?,
        );
        if position > 0 {
            value.push_str(&format!(
                " /Prev {} 0 R",
                plan.outline_object(siblings[position - 1])?
            ));
        }
        if let Some(next) = siblings.get(position + 1) {
            value.push_str(&format!(" /Next {} 0 R", plan.outline_object(*next)?));
        }
        if let Some(direct) = children.get(&Some(entry.outline_id())) {
            let first = *direct.first().ok_or(TaggedPdfError::OutlineMismatch)?;
            let last = *direct.last().ok_or(TaggedPdfError::OutlineMismatch)?;
            let descendants = entries
                .iter()
                .skip(entry.outline_id() as usize + 1)
                .take_while(|candidate| candidate.level() > entry.level())
                .count();
            value.push_str(&format!(
                " /First {} 0 R /Last {} 0 R /Count {descendants}",
                plan.outline_object(first)?,
                plan.outline_object(last)?,
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

fn emit_structure_tree(
    objects: &mut TaggedObjects,
    registry: &StructureRegistryReceipt,
    marked: &MarkedContentPlanReceipt,
    plan: &TaggedObjectPlan,
) -> Result<(), TaggedPdfError> {
    let root = registry
        .nodes()
        .first()
        .ok_or(TaggedPdfError::StructureMismatch)?;
    if root.role() != StructureRole::Document || root.parent().is_some() {
        return Err(TaggedPdfError::StructureMismatch);
    }
    let mut tree = format!(
        "<< /Type /StructTreeRoot /K [{} 0 R] /ParentTree {} 0 R /ParentTreeNextKey {} /RoleMap << /Em /Span /Exercise /Div /Proof /Div /Result /Div /Strong /Span >>",
        plan.structure_object(root.structure_node_id())?,
        plan.parent_tree_object,
        marked.parent_tree().len(),
    );
    if let Some(id_tree) = plan.id_tree_object {
        tree.push_str(&format!(" /IDTree {id_tree} 0 R"));
    }
    tree.push_str(" >>");
    if registry.nodes().is_empty() {
        tree.clear();
        return Err(TaggedPdfError::StructureMismatch);
    }
    objects.insert(
        plan.structure_tree_root_object,
        "structure_tree_root",
        tree.into_bytes(),
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
        "parent_tree",
        parent_tree.into_bytes(),
    )?;

    let mut ids = registry
        .nodes()
        .iter()
        .filter_map(|node| node.structure_id().map(|id| (id, node.structure_node_id())))
        .collect::<Vec<_>>();
    ids.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    if ids.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(TaggedPdfError::StructureMismatch);
    }
    if let Some(id_tree_object) = plan.id_tree_object {
        let mut id_tree = String::from("<< /Names [");
        for (id, node) in ids {
            id_tree.push_str(&pdf_literal(id));
            id_tree.push_str(&format!(" {} 0 R ", plan.structure_object(node)?));
        }
        id_tree.push_str("] >>");
        objects.insert(id_tree_object, "structure_id_tree", id_tree.into_bytes())?;
    } else if !ids.is_empty() {
        return Err(TaggedPdfError::StructureMismatch);
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
            value.push_str(&format!(" /Lang <{}>", utf16be_hex(node.language())?));
        }
        if let Some(alternative) = node.alternative() {
            value.push_str(&format!(" /Alt <{}>", utf16be_hex(alternative)?));
        }
        if let Some(id) = node.structure_id() {
            value.push_str(&format!(" /ID {}", pdf_literal(id)));
        }
        if let Some(numbering) = node.list_numbering() {
            if node.role() != StructureRole::List || node.table_attributes().is_some() {
                return Err(TaggedPdfError::StructureMismatch);
            }
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
                    value.push_str(&pdf_literal(header));
                    value.push(' ');
                }
                value.push(']');
            }
            value.push_str(" >>");
        }
        let kids = structure_kids(node.structure_node_id(), registry, marked, plan)?;
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

fn structure_kids(
    owner: StructureNodeId,
    registry: &StructureRegistryReceipt,
    marked: &MarkedContentPlanReceipt,
    plan: &TaggedObjectPlan,
) -> Result<Vec<String>, TaggedPdfError> {
    let node = registry
        .node(owner)
        .ok_or(TaggedPdfError::StructureMismatch)?;
    let mut kids = node
        .children()
        .iter()
        .map(|child| {
            plan.structure_object(*child)
                .map(|object| format!("{object} 0 R"))
        })
        .collect::<Result<Vec<_>, _>>()?;
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
    mcrs.sort_by_key(|(semantic_ordinal, _, _)| *semantic_ordinal);
    if mcrs
        .iter()
        .enumerate()
        .any(|(index, (ordinal, _, _))| u32::try_from(index) != Ok(*ordinal))
    {
        return Err(TaggedPdfError::MarkedContentMismatch);
    }
    let mcrs = mcrs
        .into_iter()
        .map(|(_, page_index, mcid)| {
            let page = plan
                .page_objects
                .get(page_index as usize)
                .ok_or(TaggedPdfError::MarkedContentMismatch)?;
            Ok(format!("<< /Type /MCR /Pg {page} 0 R /MCID {mcid} >>"))
        })
        .collect::<Result<Vec<_>, TaggedPdfError>>()?;
    if node.paint_required() && mcrs.is_empty() {
        return Err(TaggedPdfError::MarkedContentMismatch);
    }
    if matches!(node.role(), StructureRole::Figure | StructureRole::Formula) {
        kids.splice(0..0, mcrs);
    } else {
        kids.extend(mcrs);
    }
    for annotation in marked
        .annotations()
        .iter()
        .filter(|annotation| annotation.structure_node_id() == owner)
    {
        let page = *plan
            .page_objects
            .get(annotation.page_index() as usize)
            .ok_or(TaggedPdfError::AnnotationMismatch)?;
        let object = plan.annotation_object(annotation.annotation_id())?;
        kids.push(format!(
            "<< /Type /OBJR /Pg {page} 0 R /Obj {object} 0 R >>"
        ));
    }
    Ok(kids)
}

fn encode_info(
    navigation: &ValidatedStagingBookNavigation,
    engine: &EngineIdentity,
) -> Result<String, TaggedPdfError> {
    let metadata = navigation.metadata().metadata();
    let mut fields = Vec::new();
    if let Some(author) = &metadata.author {
        fields.push(format!("/Author <{}>", utf16be_hex(author)?));
    }
    if let Some(created) = &metadata.created {
        fields.push(format!("/CreationDate ({})", pdf_date(created)?));
    }
    if !metadata.keywords.is_empty() {
        fields.push(format!(
            "/Keywords <{}>",
            utf16be_hex(&metadata.keywords.join("; "))?
        ));
    }
    if let Some(modified) = &metadata.modified {
        fields.push(format!("/ModDate ({})", pdf_date(modified)?));
    }
    fields.push(format!(
        "/Producer <{}>",
        utf16be_hex(&format!("{} {}", engine.name(), engine.version()))?
    ));
    if let Some(subject) = &metadata.subject {
        fields.push(format!("/Subject <{}>", utf16be_hex(subject)?));
    }
    if let Some(title) = &metadata.title {
        fields.push(format!("/Title <{}>", utf16be_hex(title)?));
    }
    Ok(format!("<< {} >>", fields.join(" ")))
}

fn pdf_date(value: &str) -> Result<String, TaggedPdfError> {
    if value.len() != 20 {
        return Err(TaggedPdfError::NavigationMismatch);
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

pub(crate) fn encode_xmp(
    navigation: &ValidatedStagingBookNavigation,
    engine: &EngineIdentity,
) -> String {
    encode_tagged_book_xmp(
        navigation.metadata(),
        navigation.languages().document_language(),
        engine,
    )
}

pub(crate) fn encode_tagged_book_xmp(
    metadata: &typaxis_syntax::DocumentMetadataReceipt,
    language: &str,
    engine: &EngineIdentity,
) -> String {
    let metadata = metadata.metadata();
    let producer = format!("{} {}", engine.name(), engine.version());
    let mut properties = String::new();
    if let Some(title) = &metadata.title {
        push_xmp_alt(&mut properties, "dc:title", title, language);
    }
    if let Some(author) = &metadata.author {
        properties.push_str("<dc:creator><rdf:Seq><rdf:li>");
        properties.push_str(&xml_text(author));
        properties.push_str("</rdf:li></rdf:Seq></dc:creator>");
    }
    if let Some(subject) = &metadata.subject {
        push_xmp_alt(&mut properties, "dc:description", subject, language);
    }
    if !metadata.keywords.is_empty() {
        properties.push_str("<dc:subject><rdf:Bag>");
        for keyword in &metadata.keywords {
            properties.push_str("<rdf:li>");
            properties.push_str(&xml_text(keyword));
            properties.push_str("</rdf:li>");
        }
        properties.push_str("</rdf:Bag></dc:subject><pdf:Keywords>");
        properties.push_str(&xml_text(&metadata.keywords.join("; ")));
        properties.push_str("</pdf:Keywords>");
    }
    if let Some(identifier) = &metadata.identifier {
        properties.push_str("<dc:identifier>");
        properties.push_str(&xml_text(identifier));
        properties.push_str("</dc:identifier>");
    }
    if let Some(created) = &metadata.created {
        properties.push_str("<xmp:CreateDate>");
        properties.push_str(created);
        properties.push_str("</xmp:CreateDate>");
    }
    if let Some(modified) = &metadata.modified {
        properties.push_str("<xmp:ModifyDate>");
        properties.push_str(modified);
        properties.push_str("</xmp:ModifyDate>");
    }
    properties.push_str("<dc:language><rdf:Bag><rdf:li>");
    properties.push_str(&xml_text(language));
    properties.push_str("</rdf:li></rdf:Bag></dc:language><pdf:Producer>");
    properties.push_str(&xml_text(&producer));
    properties.push_str("</pdf:Producer><pdfuaid:part>1</pdfuaid:part>");
    format!(
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" xmlns:pdfuaid=\"http://www.aiim.org/pdfua/ns/id/\">{properties}</rdf:Description></rdf:RDF></x:xmpmeta>"
    )
}

fn push_xmp_alt(output: &mut String, property: &str, value: &str, language: &str) {
    output.push('<');
    output.push_str(property);
    output.push_str("><rdf:Alt><rdf:li xml:lang=\"x-default\">");
    output.push_str(&xml_text(value));
    output.push_str("</rdf:li>");
    if language != "x-default" {
        output.push_str("<rdf:li xml:lang=\"");
        output.push_str(&xml_attribute(language));
        output.push_str("\">");
        output.push_str(&xml_text(value));
        output.push_str("</rdf:li>");
    }
    output.push_str("</rdf:Alt></");
    output.push_str(property);
    output.push('>');
}

fn xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn xml_attribute(value: &str) -> String {
    xml_text(value).replace('"', "&quot;")
}

fn push_pdf_view(
    output: &mut String,
    view: &DestinationView,
    page_height_raw: i64,
) -> Result<(), TaggedPdfError> {
    match view {
        DestinationView::Xyz { point } => {
            let y = page_height_raw
                .checked_sub(point.y.raw())
                .ok_or(TaggedPdfError::NavigationMismatch)?;
            output.push_str(&format!(
                "/XYZ {} {} null",
                pdf_number(point.x.raw()),
                pdf_number(y)
            ));
        }
        DestinationView::FitPage => output.push_str("/Fit"),
        DestinationView::FitWidth { top } => {
            let top = top
                .as_ref()
                .map(|value| {
                    page_height_raw
                        .checked_sub(value.raw())
                        .map(pdf_number)
                        .ok_or(TaggedPdfError::NavigationMismatch)
                })
                .transpose()?
                .unwrap_or_else(|| "null".to_owned());
            output.push_str(&format!("/FitH {top}"));
        }
    }
    Ok(())
}

fn stream_object(prefix: &[u8], content: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(b"<< ");
    output.extend_from_slice(prefix);
    output.extend_from_slice(format!("/Length {} >>\nstream\n", content.len()).as_bytes());
    output.extend_from_slice(content);
    output.extend_from_slice(b"\nendstream");
    output
}

fn utf16be_hex(value: &str) -> Result<String, TaggedPdfError> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::from("FEFF");
    output
        .try_reserve(
            value
                .encode_utf16()
                .count()
                .checked_mul(4)
                .ok_or(TaggedPdfError::OutputLimit)?,
        )
        .map_err(|_| TaggedPdfError::AllocationFailure)?;
    for unit in value.encode_utf16() {
        output.push(char::from(HEX[usize::from((unit >> 12) & 0x0f)]));
        output.push(char::from(HEX[usize::from((unit >> 8) & 0x0f)]));
        output.push(char::from(HEX[usize::from((unit >> 4) & 0x0f)]));
        output.push(char::from(HEX[usize::from(unit & 0x0f)]));
    }
    Ok(output)
}

fn pdf_literal(value: &str) -> String {
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

fn pdf_number(raw: i64) -> String {
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

fn serialize_pdf(
    objects: &BTreeMap<u32, (String, Vec<u8>)>,
    object_count: u32,
    info_object: u32,
    maximum: u64,
) -> Result<Vec<u8>, TaggedPdfError> {
    let xref_count = object_count
        .checked_add(1)
        .ok_or(TaggedPdfError::ObjectLimit)?;
    let mut output = Vec::new();
    extend_bounded(&mut output, b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n", maximum)?;
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(xref_count as usize)
        .map_err(|_| TaggedPdfError::AllocationFailure)?;
    offsets.push(0usize);
    for number in 1..=object_count {
        offsets.push(output.len());
        extend_bounded(&mut output, format!("{number} 0 obj\n").as_bytes(), maximum)?;
        let value = objects
            .get(&number)
            .ok_or(TaggedPdfError::ReceiptMismatch)?;
        extend_bounded(&mut output, &value.1, maximum)?;
        extend_bounded(&mut output, b"\nendobj\n", maximum)?;
    }
    let xref = output.len();
    extend_bounded(
        &mut output,
        format!("xref\n0 {xref_count}\n").as_bytes(),
        maximum,
    )?;
    extend_bounded(&mut output, b"0000000000 65535 f \n", maximum)?;
    for offset in offsets.into_iter().skip(1) {
        extend_bounded(
            &mut output,
            format!("{offset:010} 00000 n \n").as_bytes(),
            maximum,
        )?;
    }
    extend_bounded(
        &mut output,
        format!(
            "trailer\n<< /Size {xref_count} /Root 1 0 R /Info {info_object} 0 R >>\nstartxref\n{xref}\n%%EOF\n"
        )
        .as_bytes(),
        maximum,
    )?;
    Ok(output)
}

fn extend_bounded(output: &mut Vec<u8>, value: &[u8], maximum: u64) -> Result<(), TaggedPdfError> {
    let next = output
        .len()
        .checked_add(value.len())
        .ok_or(TaggedPdfError::OutputLimit)?;
    if next as u64 > maximum {
        return Err(TaggedPdfError::OutputLimit);
    }
    output
        .try_reserve_exact(value.len())
        .map_err(|_| TaggedPdfError::AllocationFailure)?;
    output.extend_from_slice(value);
    Ok(())
}

fn encode_observation(value: &TaggedPdfObservation) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, TAGGED_PDF_ALGORITHM);
    output.push_str(",\"artifact_count\":");
    output.push_str(&value.artifact_count.to_string());
    output.push_str(",\"book_navigation_sha256\":");
    push_hash(&mut output, value.book_navigation_sha256);
    output.push_str(",\"catalog_object\":");
    output.push_str(&value.catalog_object.to_string());
    output.push_str(",\"document_language\":");
    push_jcs_string(&mut output, &value.document_language);
    output.push_str(",\"id_tree_object\":");
    if let Some(object) = value.id_tree_object {
        output.push_str(&object.to_string());
    } else {
        output.push_str("null");
    }
    output.push_str(",\"link_annotation_count\":");
    output.push_str(&value.link_annotation_count.to_string());
    output.push_str(",\"marked_content_count\":");
    output.push_str(&value.marked_content_count.to_string());
    output.push_str(",\"marked_content_sha256\":");
    push_hash(&mut output, value.marked_content_sha256);
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
        push_hash(&mut output, object.sha256);
        output.push('}');
    }
    output.push_str("],\"outline_count\":");
    output.push_str(&value.outline_count.to_string());
    output.push_str(",\"parent_tree_object\":");
    output.push_str(&value.parent_tree_object.to_string());
    output.push_str(",\"pdf_byte_length\":");
    output.push_str(&value.pdf_byte_length.to_string());
    output.push_str(",\"pdf_sha256\":");
    push_hash(&mut output, value.pdf_sha256);
    output.push_str(",\"profile_sha256\":");
    push_hash(&mut output, value.profile_sha256);
    output.push_str(",\"selected_binding_sha256\":");
    push_hash(&mut output, value.selected_binding_sha256);
    output.push_str(",\"structure_element_count\":");
    output.push_str(&value.structure_element_count.to_string());
    output.push_str(",\"structure_registry_sha256\":");
    push_hash(&mut output, value.structure_registry_sha256);
    output.push_str(",\"structure_tree_root_object\":");
    output.push_str(&value.structure_tree_root_object.to_string());
    output.push_str(",\"xmp_algorithm\":");
    push_jcs_string(&mut output, TAGGED_PDF_XMP_ALGORITHM);
    output.push_str(",\"xmp_sha256\":");
    push_hash(&mut output, value.xmp_sha256);
    output.push('}');
    output
}

fn push_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{AnchorId, Length, Point, ResourceLimits};
    use typaxis_display_list::{
        build_marked_content_plan, select_staging_book_navigation, BookInternalLinkInput,
        BookNavigationDestinationBinding, BookNavigationSelectedPage, NamedDestination,
        SelectedStructureAnnotationInput, SelectedStructurePage, SelectedStructurePaintInput,
        SelectedStructurePaintOwner, StructureOwner,
    };
    use typaxis_display_list::{build_structure_registry, select_structure_bindings};
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::{
        validate_staging_book_navigation, validate_staging_structure_semantics,
        StagingAccessibilityProfileView, StagingBookNavigationProfileView,
        StagingSemanticPackageParser,
    };

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/accessibility/job/document-package.json"
    ));
    const SCALE: i64 = 65_536;

    #[test]
    fn tagged_pdf_closes_structure_marked_content_links_outline_and_xmp() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let navigation = validate_staging_book_navigation(&package, &limits).unwrap();
        let semantics = validate_staging_structure_semantics(&package, &navigation).unwrap();
        let book_profile = StagingBookNavigationProfileAuthorization::bind_profile_receipt(
            StagingBookNavigationProfileView::new(&package, &navigation, &limits).unwrap(),
            sha256(b"tagged-pdf-book-profile"),
            &package,
            &navigation,
            &limits,
        )
        .unwrap();
        let accessibility = StagingAccessibilityProfileAuthorization::bind_profile_receipt(
            StagingAccessibilityProfileView::new(&package, &navigation, &semantics).unwrap(),
            sha256(b"tagged-pdf-accessibility-profile"),
            &package,
            &navigation,
            &semantics,
        )
        .unwrap();
        let registry =
            build_structure_registry(&package, &navigation, &semantics, &accessibility, &limits)
                .unwrap();
        let book_pages = [
            BookNavigationSelectedPage {
                page_index: 0,
                width_raw: 1_000 * SCALE,
                height_raw: 800 * SCALE,
            },
            BookNavigationSelectedPage {
                page_index: 1,
                width_raw: 1_000 * SCALE,
                height_raw: 800 * SCALE,
            },
        ];
        let destinations = navigation
            .anchors()
            .iter()
            .enumerate()
            .map(
                |(index, (anchor, source))| BookNavigationDestinationBinding {
                    source_node_id: *source,
                    frame_id: index as u32,
                    destination: NamedDestination {
                        anchor_id: anchor.clone(),
                        page_index: 0,
                        view: DestinationView::Xyz {
                            point: Point {
                                x: Length::from_raw(index as i64 * 10 * SCALE).unwrap(),
                                y: Length::from_raw(700 * SCALE).unwrap(),
                            },
                        },
                    },
                },
            )
            .collect::<Vec<_>>();
        let links = navigation
            .internal_links()
            .iter()
            .enumerate()
            .map(|(index, (owner, target))| BookInternalLinkInput {
                owner_node_id: *owner,
                page_index: 0,
                destination: target.clone(),
                x_raw: (100 + index as i64 * 100) * SCALE,
                y_raw: 650 * SCALE,
                width_raw: 60 * SCALE,
                height_raw: 20 * SCALE,
            })
            .collect::<Vec<_>>();
        let book = select_staging_book_navigation(
            &navigation,
            &book_profile,
            &limits,
            sha256(b"tagged-book-layout"),
            10,
            &book_pages,
            &destinations,
            &[],
            &links,
        )
        .unwrap();

        let mut structure_paints = registry
            .nodes()
            .iter()
            .filter(|node| node.paint_required())
            .enumerate()
            .map(|(index, node)| SelectedStructurePaintInput {
                selected_paint_id: index as u32,
                page_index: 0,
                paint_ordinal: index as u32,
                semantic_fragment_ordinal: 0,
                owner: SelectedStructurePaintOwner::Structure(node.structure_node_id()),
            })
            .collect::<Vec<_>>();
        let duplicate = structure_paints[0];
        structure_paints.insert(1, duplicate);
        for (index, paint) in structure_paints.iter_mut().enumerate() {
            paint.selected_paint_id = index as u32;
            paint.paint_ordinal = index as u32;
        }
        for (class, occurrence) in [
            (StructureArtifactClass::Pagination, 0),
            (StructureArtifactClass::PaginationHeader, 0),
            (StructureArtifactClass::PaginationFooter, 0),
            (StructureArtifactClass::Layout, 0),
        ] {
            let id = structure_paints.len() as u32;
            structure_paints.push(SelectedStructurePaintInput {
                selected_paint_id: id,
                page_index: 0,
                paint_ordinal: id,
                semantic_fragment_ordinal: 0,
                owner: SelectedStructurePaintOwner::Artifact { class, occurrence },
            });
        }
        let annotations = registry
            .nodes()
            .iter()
            .filter(|node| node.role() == StructureRole::Link)
            .enumerate()
            .map(|(index, node)| SelectedStructureAnnotationInput {
                annotation_id: index as u32,
                page_index: 0,
                annotation_ordinal: index as u32,
                owner_node_id: match node.owner() {
                    StructureOwner::Source(source) => source,
                    StructureOwner::Generated(_) => panic!("Link must be source-owned"),
                },
            })
            .collect::<Vec<_>>();
        let structure_pages = book_pages.map(|page| SelectedStructurePage {
            page_index: page.page_index,
            width_raw: page.width_raw,
            height_raw: page.height_raw,
        });
        let binding = select_structure_bindings(
            &registry,
            &accessibility,
            &limits,
            sha256(b"tagged-structure-layout"),
            (structure_paints.len() - 1) as u64,
            &structure_pages,
            &structure_paints,
            &annotations,
        )
        .unwrap();
        let marked =
            build_marked_content_plan(&registry, &binding, &accessibility, &limits).unwrap();
        assert_eq!(marked.records()[0].selected_paint_ids(), &[0, 1]);
        let engine = EngineIdentity::compiled();
        let first = write_staging_tagged_pdf(
            &navigation,
            &book_profile,
            &accessibility,
            &book,
            &registry,
            &binding,
            &marked,
            &limits,
            &engine,
        )
        .unwrap();
        let second = write_staging_tagged_pdf(
            &navigation,
            &book_profile,
            &accessibility,
            &book,
            &registry,
            &binding,
            &marked,
            &limits,
            &engine,
        )
        .unwrap();
        assert_eq!(first, second);
        first
            .verify(
                &navigation,
                &book_profile,
                &accessibility,
                &book,
                &registry,
                &binding,
                &marked,
                &limits,
                &engine,
            )
            .unwrap();
        for token in [
            b"/StructTreeRoot".as_slice(),
            b"/ParentTree",
            b"/StructParents",
            b"/StructParent",
            b"/ActualText",
            b"/Alt",
            b"/OBJR",
            b"/SE",
            b"/ListNumbering /Decimal",
            b"/ListNumbering /Disc",
            b"<pdfuaid:part>1</pdfuaid:part>",
        ] {
            assert!(first
                .bytes()
                .windows(token.len())
                .any(|window| window == token));
        }
        assert_eq!(
            first
                .bytes()
                .windows(b"/StructParents".len())
                .filter(|window| *window == b"/StructParents")
                .count(),
            1
        );
        assert_eq!(
            first
                .bytes()
                .windows(b"/Tabs /S".len())
                .filter(|window| *window == b"/Tabs /S")
                .count(),
            1
        );
        assert!(!first
            .bytes()
            .windows(b"/THead /Table".len())
            .any(|window| window == b"/THead /Table"));
        let role_map =
            b"/RoleMap << /Em /Span /Exercise /Div /Proof /Div /Result /Div /Strong /Span >>";
        assert!(first
            .bytes()
            .windows(role_map.len())
            .any(|window| window == role_map));
        assert!(!first
            .bytes()
            .windows(b"xpacket".len())
            .any(|window| window == b"xpacket"));
        let mut tampered = first.clone();
        tampered.bytes[0] = b'!';
        assert_eq!(
            tampered.verify(
                &navigation,
                &book_profile,
                &accessibility,
                &book,
                &registry,
                &binding,
                &marked,
                &limits,
                &engine,
            ),
            Err(TaggedPdfError::ReceiptMismatch)
        );
        assert_eq!(
            navigation.internal_links()[0].1,
            AnchorId::new("top").unwrap()
        );
    }
}
