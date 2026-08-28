use std::collections::BTreeMap;

use typaxis_core::{push_jcs_string, sha256, EngineIdentity, ValidatedResourceLimits};
use typaxis_display_list::{BookInternalLink, BookNavigationSelectedReceipt, DestinationView};
use typaxis_syntax::{StagingBookNavigationProfileAuthorization, ValidatedStagingBookNavigation};

pub const BOOK_NAVIGATION_PDF_ALGORITHM: &str = "typaxis.book-navigation-pdf/1";
pub const BOOK_XMP_ALGORITHM: &str = "typaxis.book-xmp/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookNavigationPdfObjectObservation {
    object_number: u32,
    role: String,
    sha256: [u8; 32],
}

impl BookNavigationPdfObjectObservation {
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
pub struct BookNavigationPdfOutlineObservation {
    outline_id: u32,
    object_number: u32,
    parent_object: u32,
    previous_object: Option<u32>,
    next_object: Option<u32>,
    first_child_object: Option<u32>,
    last_child_object: Option<u32>,
    descendant_count: u32,
    title: String,
    destination: String,
    source_node_id: u32,
    structure_element_object: Option<u32>,
}

impl BookNavigationPdfOutlineObservation {
    pub const fn outline_id(&self) -> u32 {
        self.outline_id
    }
    pub const fn object_number(&self) -> u32 {
        self.object_number
    }
    pub const fn parent_object(&self) -> u32 {
        self.parent_object
    }
    pub const fn previous_object(&self) -> Option<u32> {
        self.previous_object
    }
    pub const fn next_object(&self) -> Option<u32> {
        self.next_object
    }
    pub const fn first_child_object(&self) -> Option<u32> {
        self.first_child_object
    }
    pub const fn last_child_object(&self) -> Option<u32> {
        self.last_child_object
    }
    pub const fn descendant_count(&self) -> u32 {
        self.descendant_count
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn destination(&self) -> &str {
        &self.destination
    }
    pub const fn source_node_id(&self) -> u32 {
        self.source_node_id
    }
    pub const fn structure_element_object(&self) -> Option<u32> {
        self.structure_element_object
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookNavigationPdfLinkObservation {
    object_number: u32,
    owner_node_id: u32,
    page_index: u32,
    destination: String,
}

impl BookNavigationPdfLinkObservation {
    pub const fn object_number(&self) -> u32 {
        self.object_number
    }
    pub const fn owner_node_id(&self) -> u32 {
        self.owner_node_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn destination(&self) -> &str {
        &self.destination
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookNavigationPdfObservation {
    catalog_object: u32,
    info_object: u32,
    metadata_object: u32,
    outline_root_object: Option<u32>,
    document_language: String,
    producer: String,
    info_title: Option<String>,
    info_author: Option<String>,
    info_subject: Option<String>,
    info_keywords: Option<String>,
    info_creation_date: Option<String>,
    info_modification_date: Option<String>,
    xmp_sha256: [u8; 32],
    destination_registry_sha256: [u8; 32],
    outline_items: Vec<BookNavigationPdfOutlineObservation>,
    links: Vec<BookNavigationPdfLinkObservation>,
    objects: Vec<BookNavigationPdfObjectObservation>,
    pdf_sha256: [u8; 32],
    pdf_byte_length: u64,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl BookNavigationPdfObservation {
    pub const fn catalog_object(&self) -> u32 {
        self.catalog_object
    }
    pub const fn info_object(&self) -> u32 {
        self.info_object
    }
    pub const fn metadata_object(&self) -> u32 {
        self.metadata_object
    }
    pub const fn outline_root_object(&self) -> Option<u32> {
        self.outline_root_object
    }
    pub fn document_language(&self) -> &str {
        &self.document_language
    }
    pub fn producer(&self) -> &str {
        &self.producer
    }
    pub fn info_title(&self) -> Option<&str> {
        self.info_title.as_deref()
    }
    pub fn info_author(&self) -> Option<&str> {
        self.info_author.as_deref()
    }
    pub fn info_subject(&self) -> Option<&str> {
        self.info_subject.as_deref()
    }
    pub fn info_keywords(&self) -> Option<&str> {
        self.info_keywords.as_deref()
    }
    pub fn info_creation_date(&self) -> Option<&str> {
        self.info_creation_date.as_deref()
    }
    pub fn info_modification_date(&self) -> Option<&str> {
        self.info_modification_date.as_deref()
    }
    pub const fn xmp_sha256(&self) -> [u8; 32] {
        self.xmp_sha256
    }
    pub const fn destination_registry_sha256(&self) -> [u8; 32] {
        self.destination_registry_sha256
    }
    pub fn outline_items(&self) -> &[BookNavigationPdfOutlineObservation] {
        &self.outline_items
    }
    pub fn links(&self) -> &[BookNavigationPdfLinkObservation] {
        &self.links
    }
    pub fn objects(&self) -> &[BookNavigationPdfObjectObservation] {
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
pub struct StagingBookNavigationPdf {
    bytes: Vec<u8>,
    observation: BookNavigationPdfObservation,
}

impl StagingBookNavigationPdf {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub const fn observation(&self) -> &BookNavigationPdfObservation {
        &self.observation
    }

    pub fn verify(
        &self,
        navigation: &ValidatedStagingBookNavigation,
        profile: &StagingBookNavigationProfileAuthorization,
        selected: &BookNavigationSelectedReceipt,
        limits: &ValidatedResourceLimits,
        engine: &EngineIdentity,
    ) -> Result<(), BookNavigationPdfError> {
        selected
            .verify(navigation, profile, limits)
            .map_err(|_| BookNavigationPdfError::ReceiptMismatch)?;
        let plan = PdfObjectPlan::new(selected, limits)?;
        let xmp = encode_xmp(navigation, engine);
        let (expected_objects, outline_observations) =
            build_pdf_objects(navigation, selected, limits, engine, &plan, &xmp)?;
        let object_observations = observe_pdf_objects(&expected_objects)?;
        verify_object_roles(selected, &plan, &object_observations)?;
        let expected_bytes = serialize_pdf(
            &expected_objects.values,
            plan.object_count,
            plan.info_object,
            limits.get().max_output_bytes,
        )?;
        let expected = expected_observation(
            navigation,
            selected,
            engine,
            &plan,
            &xmp,
            &expected_bytes,
            object_observations,
        )?;
        if expected.outline_items != outline_observations
            || self.observation != expected
            || self.bytes != expected_bytes
        {
            return Err(BookNavigationPdfError::ReceiptMismatch);
        }
        verify_object_hashes(&self.bytes, &expected.objects)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookNavigationPdfError {
    ReceiptMismatch,
    ObjectLimit,
    OutputLimit,
    SpoolLimit,
    InvalidDestination,
    InvalidOutline,
    InvalidMetadata,
    AllocationFailure,
}

impl std::fmt::Display for BookNavigationPdfError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: book-navigation PDF receipt mismatch")
            }
            Self::ObjectLimit => {
                formatter.write_str("G6100: book-navigation PDF object limit exceeded")
            }
            Self::OutputLimit => {
                formatter.write_str("D8101: book-navigation PDF output limit exceeded")
            }
            Self::SpoolLimit => {
                formatter.write_str("D8101: book-navigation PDF spool limit exceeded")
            }
            Self::InvalidDestination => {
                formatter.write_str("I9190: PDF named destination mismatch")
            }
            Self::InvalidOutline => formatter.write_str("I9190: PDF outline hierarchy mismatch"),
            Self::InvalidMetadata => formatter.write_str("I9190: PDF metadata projection mismatch"),
            Self::AllocationFailure => {
                formatter.write_str("G6100: book-navigation PDF allocation failed")
            }
        }
    }
}

impl std::error::Error for BookNavigationPdfError {}

#[derive(Clone, Debug)]
struct PdfObjectPlan {
    object_count: u32,
    content_objects: Vec<u32>,
    page_objects: Vec<u32>,
    annotation_start: u32,
    info_object: u32,
    metadata_object: u32,
    outline_root_object: Option<u32>,
    outline_item_start: Option<u32>,
}

impl PdfObjectPlan {
    fn new(
        selected: &BookNavigationSelectedReceipt,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, BookNavigationPdfError> {
        let page_count = u32::try_from(selected.pages().len())
            .map_err(|_| BookNavigationPdfError::ObjectLimit)?;
        let link_count = u32::try_from(selected.links().len())
            .map_err(|_| BookNavigationPdfError::ObjectLimit)?;
        let outline_count = u32::try_from(selected.entries().len())
            .map_err(|_| BookNavigationPdfError::ObjectLimit)?;
        let annotation_start = 4u32
            .checked_add(
                page_count
                    .checked_mul(2)
                    .ok_or(BookNavigationPdfError::ObjectLimit)?,
            )
            .ok_or(BookNavigationPdfError::ObjectLimit)?;
        let info_object = annotation_start
            .checked_add(link_count)
            .ok_or(BookNavigationPdfError::ObjectLimit)?;
        let metadata_object = info_object
            .checked_add(1)
            .ok_or(BookNavigationPdfError::ObjectLimit)?;
        let (outline_root_object, outline_item_start, object_count) = if outline_count == 0 {
            (None, None, metadata_object)
        } else {
            let root = metadata_object
                .checked_add(1)
                .ok_or(BookNavigationPdfError::ObjectLimit)?;
            let start = root
                .checked_add(1)
                .ok_or(BookNavigationPdfError::ObjectLimit)?;
            let count = root
                .checked_add(outline_count)
                .ok_or(BookNavigationPdfError::ObjectLimit)?;
            (Some(root), Some(start), count)
        };
        if object_count > limits.get().max_pdf_objects {
            return Err(BookNavigationPdfError::ObjectLimit);
        }
        let mut content_objects = Vec::new();
        let mut page_objects = Vec::new();
        let page_capacity =
            usize::try_from(page_count).map_err(|_| BookNavigationPdfError::ObjectLimit)?;
        content_objects
            .try_reserve_exact(page_capacity)
            .map_err(|_| BookNavigationPdfError::AllocationFailure)?;
        page_objects
            .try_reserve_exact(page_capacity)
            .map_err(|_| BookNavigationPdfError::AllocationFailure)?;
        for page in 0..page_count {
            let content = 4u32
                .checked_add(
                    page.checked_mul(2)
                        .ok_or(BookNavigationPdfError::ObjectLimit)?,
                )
                .ok_or(BookNavigationPdfError::ObjectLimit)?;
            content_objects.push(content);
            page_objects.push(
                content
                    .checked_add(1)
                    .ok_or(BookNavigationPdfError::ObjectLimit)?,
            );
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
        })
    }

    fn outline_object(&self, outline_id: u32) -> Result<u32, BookNavigationPdfError> {
        self.outline_item_start
            .and_then(|start| start.checked_add(outline_id))
            .ok_or(BookNavigationPdfError::InvalidOutline)
    }
}

struct PdfObjects {
    values: BTreeMap<u32, (String, Vec<u8>)>,
    spool_bytes: u64,
    spool_limit: u64,
}

impl PdfObjects {
    fn new(spool_limit: u64, initial_spool_bytes: u64) -> Result<Self, BookNavigationPdfError> {
        if initial_spool_bytes > spool_limit {
            return Err(BookNavigationPdfError::SpoolLimit);
        }
        Ok(Self {
            values: BTreeMap::new(),
            spool_bytes: initial_spool_bytes,
            spool_limit,
        })
    }

    fn insert(
        &mut self,
        number: u32,
        role: impl Into<String>,
        value: Vec<u8>,
    ) -> Result<(), BookNavigationPdfError> {
        let next = self
            .spool_bytes
            .checked_add(value.len() as u64)
            .ok_or(BookNavigationPdfError::SpoolLimit)?;
        if next > self.spool_limit {
            return Err(BookNavigationPdfError::SpoolLimit);
        }
        if self.values.insert(number, (role.into(), value)).is_some() {
            return Err(BookNavigationPdfError::ReceiptMismatch);
        }
        self.spool_bytes = next;
        Ok(())
    }
}

fn build_pdf_objects(
    navigation: &ValidatedStagingBookNavigation,
    selected: &BookNavigationSelectedReceipt,
    limits: &ValidatedResourceLimits,
    engine: &EngineIdentity,
    plan: &PdfObjectPlan,
    xmp: &str,
) -> Result<(PdfObjects, Vec<BookNavigationPdfOutlineObservation>), BookNavigationPdfError> {
    let xmp_bytes = u64::try_from(xmp.len()).map_err(|_| BookNavigationPdfError::SpoolLimit)?;
    let mut objects = PdfObjects::new(limits.get().max_spool_bytes, xmp_bytes)?;

    let mut catalog = format!(
        "<< /Type /Catalog /Pages 2 0 R /Names << /Dests 3 0 R >> /Lang <{}> /Metadata {} 0 R",
        utf16be_hex(navigation.languages().document_language())?,
        plan.metadata_object,
    );
    if let Some(root) = plan.outline_root_object {
        catalog.push_str(&format!(" /Outlines {root} 0 R"));
    }
    catalog.push_str(" >>");
    objects.insert(1, "catalog", catalog.into_bytes())?;

    let mut pages = format!("<< /Type /Pages /Count {} /Kids [", selected.pages().len());
    for page in &plan.page_objects {
        pages.push_str(&format!("{page} 0 R "));
    }
    pages.push_str("] >>");
    objects.insert(2, "pages", pages.into_bytes())?;

    let mut names = String::from("<< /Names [");
    for binding in selected.destinations() {
        let page_index = binding.destination.page_index as usize;
        let page_object = *plan
            .page_objects
            .get(page_index)
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let page = selected
            .pages()
            .get(page_index)
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        names.push_str(&pdf_literal(binding.destination.anchor_id.as_str()));
        names.push_str(&format!(" [{page_object} 0 R "));
        push_pdf_view(&mut names, &binding.destination.view, page.height_raw)?;
        names.push_str("] ");
    }
    names.push_str("] >>");
    objects.insert(3, "destination_name_tree", names.into_bytes())?;

    for page in selected.pages() {
        let page_index = page.page_index as usize;
        let content_object = plan.content_objects[page_index];
        let page_object = plan.page_objects[page_index];
        let mut content = Vec::new();
        for paint in selected
            .language_paints()
            .iter()
            .filter(|paint| paint.page_index() == page.page_index)
        {
            let dictionary = format!("<< /Lang <{}> >>", utf16be_hex(paint.language())?);
            content.extend_from_slice(
                format!("/Span {dictionary} BDC\n0 0 m 0 0 l S\nEMC\n").as_bytes(),
            );
        }
        let stream = stream_object(b"", &content);
        objects.insert(
            content_object,
            format!("page_content:{}", page.page_index),
            stream,
        )?;

        let page_links: Vec<(usize, &BookInternalLink)> = selected
            .links()
            .iter()
            .enumerate()
            .filter(|(_, link)| link.page_index() == page.page_index)
            .collect();
        let mut dictionary = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << >> /Contents {} 0 R",
            pdf_number(page.width_raw),
            pdf_number(page.height_raw),
            content_object,
        );
        if !page_links.is_empty() {
            dictionary.push_str(" /Annots [");
            for (index, _) in &page_links {
                let number = plan.annotation_start
                    + u32::try_from(*index).map_err(|_| BookNavigationPdfError::ObjectLimit)?;
                dictionary.push_str(&format!("{number} 0 R "));
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

    for (index, link) in selected.links().iter().enumerate() {
        let number = plan.annotation_start
            + u32::try_from(index).map_err(|_| BookNavigationPdfError::ObjectLimit)?;
        let page = selected
            .pages()
            .get(link.page_index() as usize)
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let right = link
            .x_raw()
            .checked_add(link.width_raw())
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let logical_bottom = link
            .y_raw()
            .checked_add(link.height_raw())
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let pdf_bottom = page
            .height_raw
            .checked_sub(logical_bottom)
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let pdf_top = page
            .height_raw
            .checked_sub(link.y_raw())
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let value = format!(
            "<< /Type /Annot /Subtype /Link /Rect [{} {} {} {}] /Border [0 0 0] /Dest {} >>",
            pdf_number(link.x_raw()),
            pdf_number(pdf_bottom),
            pdf_number(right),
            pdf_number(pdf_top),
            pdf_literal(link.destination().as_str()),
        );
        objects.insert(
            number,
            format!("link_annotation:{index}"),
            value.into_bytes(),
        )?;
    }

    objects.insert(
        plan.info_object,
        "info",
        encode_info(navigation, engine)?.into_bytes(),
    )?;
    let metadata_prefix = b"/Type /Metadata /Subtype /XML ";
    objects.insert(
        plan.metadata_object,
        "metadata",
        stream_object(metadata_prefix, xmp.as_bytes()),
    )?;

    let outline_observations = if let Some(root) = plan.outline_root_object {
        emit_outlines(&mut objects, navigation, selected, plan, root)?
    } else {
        Vec::new()
    };
    if objects.values.len() != plan.object_count as usize
        || objects.values.keys().copied().ne(1..=plan.object_count)
    {
        return Err(BookNavigationPdfError::ReceiptMismatch);
    }
    Ok((objects, outline_observations))
}

fn observe_pdf_objects(
    objects: &PdfObjects,
) -> Result<Vec<BookNavigationPdfObjectObservation>, BookNavigationPdfError> {
    let mut observations = Vec::new();
    observations
        .try_reserve_exact(objects.values.len())
        .map_err(|_| BookNavigationPdfError::AllocationFailure)?;
    for (number, (role, value)) in &objects.values {
        observations.push(BookNavigationPdfObjectObservation {
            object_number: *number,
            role: role.clone(),
            sha256: sha256(value),
        });
    }
    Ok(observations)
}

pub fn write_staging_book_navigation_pdf(
    navigation: &ValidatedStagingBookNavigation,
    profile: &StagingBookNavigationProfileAuthorization,
    selected: &BookNavigationSelectedReceipt,
    limits: &ValidatedResourceLimits,
    engine: &EngineIdentity,
) -> Result<StagingBookNavigationPdf, BookNavigationPdfError> {
    selected
        .verify(navigation, profile, limits)
        .map_err(|_| BookNavigationPdfError::ReceiptMismatch)?;
    let plan = PdfObjectPlan::new(selected, limits)?;
    let xmp = encode_xmp(navigation, engine);
    let xmp_bytes = u64::try_from(xmp.len()).map_err(|_| BookNavigationPdfError::SpoolLimit)?;
    let mut objects = PdfObjects::new(limits.get().max_spool_bytes, xmp_bytes)?;

    let mut catalog = format!(
        "<< /Type /Catalog /Pages 2 0 R /Names << /Dests 3 0 R >> /Lang <{}> /Metadata {} 0 R",
        utf16be_hex(navigation.languages().document_language())?,
        plan.metadata_object,
    );
    if let Some(root) = plan.outline_root_object {
        catalog.push_str(&format!(" /Outlines {root} 0 R"));
    }
    catalog.push_str(" >>");
    objects.insert(1, "catalog", catalog.into_bytes())?;

    let mut pages = format!("<< /Type /Pages /Count {} /Kids [", selected.pages().len());
    for page in &plan.page_objects {
        pages.push_str(&format!("{page} 0 R "));
    }
    pages.push_str("] >>");
    objects.insert(2, "pages", pages.into_bytes())?;

    let mut names = String::from("<< /Names [");
    for binding in selected.destinations() {
        let page_index = binding.destination.page_index as usize;
        let page_object = *plan
            .page_objects
            .get(page_index)
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let page = selected
            .pages()
            .get(page_index)
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        names.push_str(&pdf_literal(binding.destination.anchor_id.as_str()));
        names.push_str(&format!(" [{page_object} 0 R "));
        push_pdf_view(&mut names, &binding.destination.view, page.height_raw)?;
        names.push_str("] ");
    }
    names.push_str("] >>");
    objects.insert(3, "destination_name_tree", names.into_bytes())?;

    for page in selected.pages() {
        let page_index = page.page_index as usize;
        let content_object = plan.content_objects[page_index];
        let page_object = plan.page_objects[page_index];
        let mut content = Vec::new();
        for paint in selected
            .language_paints()
            .iter()
            .filter(|paint| paint.page_index() == page.page_index)
        {
            let dictionary = format!("<< /Lang <{}> >>", utf16be_hex(paint.language())?);
            content.extend_from_slice(
                format!("/Span {dictionary} BDC\n0 0 m 0 0 l S\nEMC\n").as_bytes(),
            );
        }
        let stream = stream_object(b"", &content);
        objects.insert(
            content_object,
            format!("page_content:{}", page.page_index),
            stream,
        )?;

        let page_links: Vec<(usize, &BookInternalLink)> = selected
            .links()
            .iter()
            .enumerate()
            .filter(|(_, link)| link.page_index() == page.page_index)
            .collect();
        let mut dictionary =
            format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << >> /Contents {} 0 R",
            pdf_number(page.width_raw), pdf_number(page.height_raw), content_object,
        );
        if !page_links.is_empty() {
            dictionary.push_str(" /Annots [");
            for (index, _) in &page_links {
                let number = plan.annotation_start
                    + u32::try_from(*index).map_err(|_| BookNavigationPdfError::ObjectLimit)?;
                dictionary.push_str(&format!("{number} 0 R "));
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

    for (index, link) in selected.links().iter().enumerate() {
        let number = plan.annotation_start
            + u32::try_from(index).map_err(|_| BookNavigationPdfError::ObjectLimit)?;
        let page = selected
            .pages()
            .get(link.page_index() as usize)
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let right = link
            .x_raw()
            .checked_add(link.width_raw())
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let logical_bottom = link
            .y_raw()
            .checked_add(link.height_raw())
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let pdf_bottom = page
            .height_raw
            .checked_sub(logical_bottom)
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let pdf_top = page
            .height_raw
            .checked_sub(link.y_raw())
            .ok_or(BookNavigationPdfError::InvalidDestination)?;
        let value = format!(
            "<< /Type /Annot /Subtype /Link /Rect [{} {} {} {}] /Border [0 0 0] /Dest {} >>",
            pdf_number(link.x_raw()),
            pdf_number(pdf_bottom),
            pdf_number(right),
            pdf_number(pdf_top),
            pdf_literal(link.destination().as_str()),
        );
        objects.insert(
            number,
            format!("link_annotation:{index}"),
            value.into_bytes(),
        )?;
    }

    objects.insert(
        plan.info_object,
        "info",
        encode_info(navigation, engine)?.into_bytes(),
    )?;
    let metadata_prefix = b"/Type /Metadata /Subtype /XML ";
    objects.insert(
        plan.metadata_object,
        "metadata",
        stream_object(metadata_prefix, xmp.as_bytes()),
    )?;

    let outline_observations = if let Some(root) = plan.outline_root_object {
        emit_outlines(&mut objects, navigation, selected, &plan, root)?
    } else {
        Vec::new()
    };

    if objects.values.len() != plan.object_count as usize
        || objects.values.keys().copied().ne(1..=plan.object_count)
    {
        return Err(BookNavigationPdfError::ReceiptMismatch);
    }
    let object_observations = observe_pdf_objects(&objects)?;
    let bytes = serialize_pdf(
        &objects.values,
        plan.object_count,
        plan.info_object,
        limits.get().max_output_bytes,
    )?;
    let mut observation = expected_observation(
        navigation,
        selected,
        engine,
        &plan,
        &xmp,
        &bytes,
        object_observations,
    )?;
    if observation.outline_items != outline_observations {
        return Err(BookNavigationPdfError::InvalidOutline);
    }
    observation.canonical_jcs = encode_observation(&observation);
    observation.fingerprint = sha256(observation.canonical_jcs.as_bytes());
    let pdf = StagingBookNavigationPdf { bytes, observation };
    pdf.verify(navigation, profile, selected, limits, engine)?;
    Ok(pdf)
}

fn emit_outlines(
    objects: &mut PdfObjects,
    _navigation: &ValidatedStagingBookNavigation,
    selected: &BookNavigationSelectedReceipt,
    plan: &PdfObjectPlan,
    root: u32,
) -> Result<Vec<BookNavigationPdfOutlineObservation>, BookNavigationPdfError> {
    let observations = build_outline_observations(selected, plan, root)?;
    let top_level: Vec<_> = observations
        .iter()
        .filter(|item| item.parent_object == root)
        .collect();
    let first = top_level
        .first()
        .ok_or(BookNavigationPdfError::InvalidOutline)?;
    let last = top_level
        .last()
        .ok_or(BookNavigationPdfError::InvalidOutline)?;
    objects.insert(
        root,
        "outline_root",
        format!(
            "<< /Type /Outlines /First {} 0 R /Last {} 0 R /Count {} >>",
            first.object_number,
            last.object_number,
            observations.len(),
        )
        .into_bytes(),
    )?;
    for item in &observations {
        let mut value = format!(
            "<< /Title <{}> /Parent {} 0 R /Dest {}",
            utf16be_hex(&item.title)?,
            item.parent_object,
            pdf_literal(&item.destination),
        );
        if let Some(previous) = item.previous_object {
            value.push_str(&format!(" /Prev {previous} 0 R"));
        }
        if let Some(next) = item.next_object {
            value.push_str(&format!(" /Next {next} 0 R"));
        }
        if let Some(first_child) = item.first_child_object {
            value.push_str(&format!(" /First {first_child} 0 R"));
            value.push_str(&format!(
                " /Last {} 0 R /Count {}",
                item.last_child_object
                    .ok_or(BookNavigationPdfError::InvalidOutline)?,
                item.descendant_count,
            ));
        } else if item.last_child_object.is_some() || item.descendant_count != 0 {
            return Err(BookNavigationPdfError::InvalidOutline);
        }
        value.push_str(" >>");
        objects.insert(
            item.object_number,
            format!("outline_item:{}", item.outline_id),
            value.into_bytes(),
        )?;
    }
    Ok(observations)
}

fn build_outline_observations(
    selected: &BookNavigationSelectedReceipt,
    plan: &PdfObjectPlan,
    root: u32,
) -> Result<Vec<BookNavigationPdfOutlineObservation>, BookNavigationPdfError> {
    let entries = selected.entries();
    let mut children: BTreeMap<Option<u32>, Vec<u32>> = BTreeMap::new();
    let mut sibling_positions = Vec::new();
    sibling_positions
        .try_reserve_exact(entries.len())
        .map_err(|_| BookNavigationPdfError::AllocationFailure)?;
    for (index, entry) in entries.iter().enumerate() {
        if usize::try_from(entry.outline_id()) != Ok(index) {
            return Err(BookNavigationPdfError::InvalidOutline);
        }
        let siblings = children.entry(entry.parent_outline_id()).or_default();
        siblings
            .try_reserve(1)
            .map_err(|_| BookNavigationPdfError::AllocationFailure)?;
        sibling_positions.push(siblings.len());
        siblings.push(entry.outline_id());
    }
    let mut observations = Vec::new();
    observations
        .try_reserve_exact(entries.len())
        .map_err(|_| BookNavigationPdfError::AllocationFailure)?;
    for (index, entry) in entries.iter().enumerate() {
        let object_number = plan.outline_object(entry.outline_id())?;
        let parent_object = entry
            .parent_outline_id()
            .map(|parent| plan.outline_object(parent))
            .transpose()?
            .unwrap_or(root);
        let siblings = children
            .get(&entry.parent_outline_id())
            .ok_or(BookNavigationPdfError::InvalidOutline)?;
        let sibling_index = sibling_positions[index];
        let previous_object = sibling_index
            .checked_sub(1)
            .map(|position| plan.outline_object(siblings[position]))
            .transpose()?;
        let next_object = siblings
            .get(sibling_index + 1)
            .map(|outline_id| plan.outline_object(*outline_id))
            .transpose()?;
        let direct_children = children.get(&Some(entry.outline_id()));
        let first_child_object = direct_children
            .and_then(|values| values.first())
            .map(|outline_id| plan.outline_object(*outline_id))
            .transpose()?;
        let last_child_object = direct_children
            .and_then(|values| values.last())
            .map(|outline_id| plan.outline_object(*outline_id))
            .transpose()?;
        let descendant_count = entries[index + 1..]
            .iter()
            .take_while(|candidate| candidate.level() > entry.level())
            .count();
        observations.push(BookNavigationPdfOutlineObservation {
            outline_id: entry.outline_id(),
            object_number,
            parent_object,
            previous_object,
            next_object,
            first_child_object,
            last_child_object,
            descendant_count: u32::try_from(descendant_count)
                .map_err(|_| BookNavigationPdfError::InvalidOutline)?,
            title: entry.label().to_owned(),
            destination: entry.destination().anchor_id.as_str().to_owned(),
            source_node_id: entry.source_node_id().get(),
            structure_element_object: None,
        });
    }
    Ok(observations)
}

fn encode_info(
    navigation: &ValidatedStagingBookNavigation,
    engine: &EngineIdentity,
) -> Result<String, BookNavigationPdfError> {
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

fn pdf_date(value: &str) -> Result<String, BookNavigationPdfError> {
    if value.len() != 20 {
        return Err(BookNavigationPdfError::InvalidMetadata);
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

fn encode_xmp(navigation: &ValidatedStagingBookNavigation, engine: &EngineIdentity) -> String {
    let metadata = navigation.metadata().metadata();
    let language = navigation.languages().document_language();
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
    properties.push_str("</pdf:Producer>");
    format!(
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">{properties}</rdf:Description></rdf:RDF></x:xmpmeta>"
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

fn expected_observation(
    navigation: &ValidatedStagingBookNavigation,
    selected: &BookNavigationSelectedReceipt,
    engine: &EngineIdentity,
    plan: &PdfObjectPlan,
    xmp: &str,
    bytes: &[u8],
    objects: Vec<BookNavigationPdfObjectObservation>,
) -> Result<BookNavigationPdfObservation, BookNavigationPdfError> {
    let metadata = navigation.metadata().metadata();
    let producer = format!("{} {}", engine.name(), engine.version());
    let outline_items = if let Some(root) = plan.outline_root_object {
        build_outline_observations(selected, plan, root)?
    } else {
        Vec::new()
    };
    let links = selected
        .links()
        .iter()
        .enumerate()
        .map(|(index, link)| {
            Ok(BookNavigationPdfLinkObservation {
                object_number: plan
                    .annotation_start
                    .checked_add(
                        u32::try_from(index).map_err(|_| BookNavigationPdfError::ObjectLimit)?,
                    )
                    .ok_or(BookNavigationPdfError::ObjectLimit)?,
                owner_node_id: link.owner_node_id().get(),
                page_index: link.page_index(),
                destination: link.destination().as_str().to_owned(),
            })
        })
        .collect::<Result<Vec<_>, BookNavigationPdfError>>()?;
    let mut value = BookNavigationPdfObservation {
        catalog_object: 1,
        info_object: plan.info_object,
        metadata_object: plan.metadata_object,
        outline_root_object: plan.outline_root_object,
        document_language: navigation.languages().document_language().to_owned(),
        producer,
        info_title: metadata.title.clone(),
        info_author: metadata.author.clone(),
        info_subject: metadata.subject.clone(),
        info_keywords: (!metadata.keywords.is_empty()).then(|| metadata.keywords.join("; ")),
        info_creation_date: metadata.created.as_deref().map(pdf_date).transpose()?,
        info_modification_date: metadata.modified.as_deref().map(pdf_date).transpose()?,
        xmp_sha256: sha256(xmp.as_bytes()),
        destination_registry_sha256: selected.destination_registry_sha256(),
        outline_items,
        links,
        objects,
        pdf_sha256: sha256(bytes),
        pdf_byte_length: bytes.len() as u64,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    value.canonical_jcs = encode_observation(&value);
    value.fingerprint = sha256(value.canonical_jcs.as_bytes());
    Ok(value)
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

fn push_pdf_view(
    output: &mut String,
    view: &DestinationView,
    page_height_raw: i64,
) -> Result<(), BookNavigationPdfError> {
    match view {
        DestinationView::Xyz { point } => {
            let y = page_height_raw
                .checked_sub(point.y.raw())
                .ok_or(BookNavigationPdfError::InvalidDestination)?;
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
                        .ok_or(BookNavigationPdfError::InvalidDestination)
                })
                .transpose()?
                .unwrap_or_else(|| "null".to_owned());
            output.push_str(&format!("/FitH {top}"));
        }
    }
    Ok(())
}

fn utf16be_hex(value: &str) -> Result<String, BookNavigationPdfError> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::from("FEFF");
    output
        .try_reserve(
            value
                .encode_utf16()
                .count()
                .checked_mul(4)
                .ok_or(BookNavigationPdfError::OutputLimit)?,
        )
        .map_err(|_| BookNavigationPdfError::AllocationFailure)?;
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
) -> Result<Vec<u8>, BookNavigationPdfError> {
    let xref_count = object_count
        .checked_add(1)
        .ok_or(BookNavigationPdfError::ObjectLimit)?;
    let mut output = Vec::new();
    extend_bounded(&mut output, b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n", maximum)?;
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(
            usize::try_from(xref_count).map_err(|_| BookNavigationPdfError::ObjectLimit)?,
        )
        .map_err(|_| BookNavigationPdfError::AllocationFailure)?;
    offsets.push(0usize);
    for number in 1..=object_count {
        offsets.push(output.len());
        extend_bounded(&mut output, format!("{number} 0 obj\n").as_bytes(), maximum)?;
        let value = objects
            .get(&number)
            .ok_or(BookNavigationPdfError::ReceiptMismatch)?;
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
            "trailer\n<< /Size {} /Root 1 0 R /Info {} 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            xref_count, info_object,
        )
        .as_bytes(),
        maximum,
    )?;
    Ok(output)
}

fn extend_bounded(
    output: &mut Vec<u8>,
    value: &[u8],
    maximum: u64,
) -> Result<(), BookNavigationPdfError> {
    let next = output
        .len()
        .checked_add(value.len())
        .ok_or(BookNavigationPdfError::OutputLimit)?;
    if next as u64 > maximum {
        return Err(BookNavigationPdfError::OutputLimit);
    }
    output
        .try_reserve_exact(value.len())
        .map_err(|_| BookNavigationPdfError::AllocationFailure)?;
    output.extend_from_slice(value);
    Ok(())
}

fn verify_object_hashes(
    bytes: &[u8],
    expected: &[BookNavigationPdfObjectObservation],
) -> Result<(), BookNavigationPdfError> {
    let offsets = parse_xref_offsets(bytes)?;
    if offsets.len() != expected.len() + 1 {
        return Err(BookNavigationPdfError::ReceiptMismatch);
    }
    for item in expected {
        let offset = *offsets
            .get(item.object_number as usize)
            .ok_or(BookNavigationPdfError::ReceiptMismatch)?;
        let header = format!("{} 0 obj\n", item.object_number);
        let payload = bytes
            .get(offset..)
            .and_then(|value| value.strip_prefix(header.as_bytes()))
            .ok_or(BookNavigationPdfError::ReceiptMismatch)?;
        let end = payload
            .windows(b"\nendobj\n".len())
            .position(|window| window == b"\nendobj\n")
            .ok_or(BookNavigationPdfError::ReceiptMismatch)?;
        if sha256(&payload[..end]) != item.sha256 {
            return Err(BookNavigationPdfError::ReceiptMismatch);
        }
    }
    Ok(())
}

fn verify_object_roles(
    selected: &BookNavigationSelectedReceipt,
    plan: &PdfObjectPlan,
    observed: &[BookNavigationPdfObjectObservation],
) -> Result<(), BookNavigationPdfError> {
    let capacity =
        usize::try_from(plan.object_count).map_err(|_| BookNavigationPdfError::ObjectLimit)?;
    let mut expected = Vec::new();
    expected
        .try_reserve_exact(capacity)
        .map_err(|_| BookNavigationPdfError::AllocationFailure)?;
    expected.push((1, "catalog".to_owned()));
    expected.push((2, "pages".to_owned()));
    expected.push((3, "destination_name_tree".to_owned()));
    for page in selected.pages() {
        let index = page.page_index as usize;
        expected.push((
            *plan
                .content_objects
                .get(index)
                .ok_or(BookNavigationPdfError::ReceiptMismatch)?,
            format!("page_content:{}", page.page_index),
        ));
        expected.push((
            *plan
                .page_objects
                .get(index)
                .ok_or(BookNavigationPdfError::ReceiptMismatch)?,
            format!("page:{}", page.page_index),
        ));
    }
    for index in 0..selected.links().len() {
        let number = plan
            .annotation_start
            .checked_add(u32::try_from(index).map_err(|_| BookNavigationPdfError::ObjectLimit)?)
            .ok_or(BookNavigationPdfError::ObjectLimit)?;
        expected.push((number, format!("link_annotation:{index}")));
    }
    expected.push((plan.info_object, "info".to_owned()));
    expected.push((plan.metadata_object, "metadata".to_owned()));
    if let Some(root) = plan.outline_root_object {
        expected.push((root, "outline_root".to_owned()));
        for entry in selected.entries() {
            expected.push((
                plan.outline_object(entry.outline_id())?,
                format!("outline_item:{}", entry.outline_id()),
            ));
        }
    }
    if expected.len() != capacity
        || observed.len() != expected.len()
        || observed
            .iter()
            .zip(expected)
            .any(|(actual, (number, role))| actual.object_number != number || actual.role != role)
    {
        return Err(BookNavigationPdfError::ReceiptMismatch);
    }
    Ok(())
}

fn parse_xref_offsets(bytes: &[u8]) -> Result<Vec<usize>, BookNavigationPdfError> {
    let marker = b"startxref\n";
    let start = bytes
        .windows(marker.len())
        .rposition(|window| window == marker)
        .and_then(|position| position.checked_add(marker.len()))
        .ok_or(BookNavigationPdfError::ReceiptMismatch)?;
    let end = bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .and_then(|length| start.checked_add(length))
        .ok_or(BookNavigationPdfError::ReceiptMismatch)?;
    let xref = parse_decimal(&bytes[start..end])?;
    let mut lines = bytes
        .get(xref..)
        .ok_or(BookNavigationPdfError::ReceiptMismatch)?
        .split(|byte| *byte == b'\n');
    if lines.next() != Some(b"xref".as_slice()) {
        return Err(BookNavigationPdfError::ReceiptMismatch);
    }
    let header = lines
        .next()
        .ok_or(BookNavigationPdfError::ReceiptMismatch)?;
    let mut parts = header.split(|byte| *byte == b' ');
    if parts.next() != Some(b"0".as_slice()) {
        return Err(BookNavigationPdfError::ReceiptMismatch);
    }
    let count = parse_decimal(
        parts
            .next()
            .ok_or(BookNavigationPdfError::ReceiptMismatch)?,
    )?;
    if parts.next().is_some() || count == 0 {
        return Err(BookNavigationPdfError::ReceiptMismatch);
    }
    let mut offsets = Vec::new();
    for index in 0..count {
        let line = lines
            .next()
            .ok_or(BookNavigationPdfError::ReceiptMismatch)?;
        if line.len() != 19
            || (index == 0 && &line[11..] != b"65535 f ")
            || (index != 0 && &line[11..] != b"00000 n ")
        {
            return Err(BookNavigationPdfError::ReceiptMismatch);
        }
        offsets.push(parse_decimal(&line[..10])?);
    }
    Ok(offsets)
}

fn parse_decimal(value: &[u8]) -> Result<usize, BookNavigationPdfError> {
    if value.is_empty() || value.iter().any(|byte| !byte.is_ascii_digit()) {
        return Err(BookNavigationPdfError::ReceiptMismatch);
    }
    value.iter().try_fold(0usize, |total, byte| {
        total
            .checked_mul(10)
            .and_then(|total| total.checked_add(usize::from(byte - b'0')))
            .ok_or(BookNavigationPdfError::ReceiptMismatch)
    })
}

fn encode_observation(value: &BookNavigationPdfObservation) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, BOOK_NAVIGATION_PDF_ALGORITHM);
    output.push_str(",\"catalog_object\":");
    output.push_str(&value.catalog_object.to_string());
    output.push_str(",\"destination_registry_sha256\":");
    push_hash(&mut output, value.destination_registry_sha256);
    output.push_str(",\"document_language\":");
    push_jcs_string(&mut output, &value.document_language);
    output.push_str(",\"info\":{\"author\":");
    push_nullable(&mut output, value.info_author.as_deref());
    output.push_str(",\"creation_date\":");
    push_nullable(&mut output, value.info_creation_date.as_deref());
    output.push_str(",\"keywords\":");
    push_nullable(&mut output, value.info_keywords.as_deref());
    output.push_str(",\"modification_date\":");
    push_nullable(&mut output, value.info_modification_date.as_deref());
    output.push_str(",\"object\":");
    output.push_str(&value.info_object.to_string());
    output.push_str(",\"producer\":");
    push_jcs_string(&mut output, &value.producer);
    output.push_str(",\"subject\":");
    push_nullable(&mut output, value.info_subject.as_deref());
    output.push_str(",\"title\":");
    push_nullable(&mut output, value.info_title.as_deref());
    output.push_str("},\"links\":[");
    for (index, link) in value.links.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"destination\":");
        push_jcs_string(&mut output, &link.destination);
        output.push_str(",\"object\":");
        output.push_str(&link.object_number.to_string());
        output.push_str(",\"owner_node_id\":");
        output.push_str(&link.owner_node_id.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&link.page_index.to_string());
        output.push('}');
    }
    output.push_str("],\"metadata_object\":");
    output.push_str(&value.metadata_object.to_string());
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
    output.push_str("],\"outline_items\":[");
    for (index, item) in value.outline_items.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"descendant_count\":");
        output.push_str(&item.descendant_count.to_string());
        output.push_str(",\"destination\":");
        push_jcs_string(&mut output, &item.destination);
        output.push_str(",\"first_child_object\":");
        push_optional_u32(&mut output, item.first_child_object);
        output.push_str(",\"last_child_object\":");
        push_optional_u32(&mut output, item.last_child_object);
        output.push_str(",\"next_object\":");
        push_optional_u32(&mut output, item.next_object);
        output.push_str(",\"object_number\":");
        output.push_str(&item.object_number.to_string());
        output.push_str(",\"outline_id\":");
        output.push_str(&item.outline_id.to_string());
        output.push_str(",\"parent_object\":");
        output.push_str(&item.parent_object.to_string());
        output.push_str(",\"previous_object\":");
        push_optional_u32(&mut output, item.previous_object);
        output.push_str(",\"source_node_id\":");
        output.push_str(&item.source_node_id.to_string());
        output.push_str(",\"structure_element_object\":");
        push_optional_u32(&mut output, item.structure_element_object);
        output.push_str(",\"title\":");
        push_jcs_string(&mut output, &item.title);
        output.push('}');
    }
    output.push_str("],\"outline_root_object\":");
    push_optional_u32(&mut output, value.outline_root_object);
    output.push_str(",\"pdf_byte_length\":");
    output.push_str(&value.pdf_byte_length.to_string());
    output.push_str(",\"pdf_sha256\":");
    push_hash(&mut output, value.pdf_sha256);
    output.push_str(",\"xmp\":{\"algorithm\":");
    push_jcs_string(&mut output, BOOK_XMP_ALGORITHM);
    output.push_str(",\"sha256\":");
    push_hash(&mut output, value.xmp_sha256);
    output.push_str("}}");
    output
}

fn push_optional_u32(output: &mut String, value: Option<u32>) {
    if let Some(value) = value {
        output.push_str(&value.to_string());
    } else {
        output.push_str("null");
    }
}

fn push_nullable(output: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        push_jcs_string(output, value);
    } else {
        output.push_str("null");
    }
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
    use typaxis_core::{
        AnchorId, EngineIdentity, Length, NodeId, Point, ResourceLimits, ValidatedResourceLimits,
    };
    use typaxis_display_list::{
        select_staging_book_navigation, BookInternalLinkInput, BookLanguagePaintInput,
        BookNavigationDestinationBinding, BookNavigationSelectedPage, NamedDestination,
    };
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::{
        validate_staging_book_navigation, StagingBookNavigationProfileAuthorization,
        StagingBookNavigationProfileView, StagingSemanticPackageParser,
    };

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/book-navigation/job/document-package.json"
    ));
    const SCALE: i64 = 65_536;

    fn selected_fixture() -> (
        typaxis_syntax::ValidatedStagingSemanticPackage,
        ValidatedStagingBookNavigation,
        StagingBookNavigationProfileAuthorization,
        ValidatedResourceLimits,
        BookNavigationSelectedReceipt,
    ) {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let navigation = validate_staging_book_navigation(&package, &limits).unwrap();
        let profile = StagingBookNavigationProfileAuthorization::bind_profile_receipt(
            StagingBookNavigationProfileView::new(&package, &navigation, &limits).unwrap(),
            sha256(b"test-book-navigation-profile"),
            &package,
            &navigation,
            &limits,
        )
        .unwrap();
        let pages = [
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
        let destination = |anchor: &str, source: u32, frame_id: u32, page: u32, x: i64, y: i64| {
            BookNavigationDestinationBinding {
                source_node_id: NodeId::new(source),
                frame_id,
                destination: NamedDestination {
                    anchor_id: AnchorId::new(anchor).unwrap(),
                    page_index: page,
                    view: DestinationView::Xyz {
                        point: Point {
                            x: Length::from_raw(x * SCALE).unwrap(),
                            y: Length::from_raw(y * SCALE).unwrap(),
                        },
                    },
                },
            }
        };
        let destinations = [
            destination("chapter-1", 2, 1, 0, 100, 700),
            destination("exercise-1", 7, 2, 1, 100, 700),
            destination("part-1", 1, 0, 0, 0, 800),
        ];
        let paints = [
            BookLanguagePaintInput {
                owner_node_id: NodeId::new(3),
                occurrence: 0,
                page_index: 0,
            },
            BookLanguagePaintInput {
                owner_node_id: NodeId::new(6),
                occurrence: 0,
                page_index: 0,
            },
            BookLanguagePaintInput {
                owner_node_id: NodeId::new(9),
                occurrence: 0,
                page_index: 1,
            },
        ];
        let links = [BookInternalLinkInput {
            owner_node_id: NodeId::new(5),
            page_index: 0,
            destination: AnchorId::new("chapter-1").unwrap(),
            x_raw: 100 * SCALE,
            y_raw: 650 * SCALE,
            width_raw: 60 * SCALE,
            height_raw: 20 * SCALE,
        }];
        let selected = select_staging_book_navigation(
            &navigation,
            &profile,
            &limits,
            sha256(b"selected-book-layout"),
            3,
            &pages,
            &destinations,
            &paints,
            &links,
        )
        .unwrap();
        (package, navigation, profile, limits, selected)
    }

    #[test]
    fn metadata_outline_pdf_is_deterministic_and_receipt_closed() {
        let (_package, navigation, profile, limits, selected) = selected_fixture();
        let engine = EngineIdentity::compiled();
        let first =
            write_staging_book_navigation_pdf(&navigation, &profile, &selected, &limits, &engine)
                .unwrap();
        let second =
            write_staging_book_navigation_pdf(&navigation, &profile, &selected, &limits, &engine)
                .unwrap();
        assert_eq!(first, second);
        first
            .verify(&navigation, &profile, &selected, &limits, &engine)
            .unwrap();
        assert_eq!(first.observation().document_language(), "en-US");
        assert_eq!(first.observation().outline_items().len(), 3);
        assert_eq!(first.observation().links().len(), 1);
        assert_eq!(first.observation().info_title(), Some("Typaxis Book"));
        assert!(first
            .bytes()
            .windows(b"<dc:identifier>urn:example:book:1</dc:identifier>".len())
            .any(|window| window == b"<dc:identifier>urn:example:book:1</dc:identifier>"));
        assert!(first
            .bytes()
            .windows(b"/Dest (chapter-1)".len())
            .any(|window| window == b"/Dest (chapter-1)"));
        assert!(first
            .bytes()
            .windows(b"/XYZ 100 100 null".len())
            .any(|window| window == b"/XYZ 100 100 null"));
        assert!(first
            .bytes()
            .windows(b"/XYZ 0 0 null".len())
            .any(|window| window == b"/XYZ 0 0 null"));
        assert!(first
            .bytes()
            .windows(b"/Rect [100 130 160 150]".len())
            .any(|window| window == b"/Rect [100 130 160 150]"));
        assert!(!first.bytes().windows(3).any(|window| window == b"/A "));
        assert!(!first.bytes().windows(4).any(|window| window == b"/SE "));
        assert!(!first.bytes().windows(4).any(|window| window == b"/ID "));

        let mut tampered = first.clone();
        tampered.observation.objects[0].role = "metadata".to_owned();
        tampered.observation.canonical_jcs = encode_observation(&tampered.observation);
        tampered.observation.fingerprint = sha256(tampered.observation.canonical_jcs.as_bytes());
        assert_eq!(
            tampered.verify(&navigation, &profile, &selected, &limits, &engine),
            Err(BookNavigationPdfError::ReceiptMismatch)
        );

        let mut payload_tampered = first.clone();
        let marker = b"/Span ";
        let marker_position = payload_tampered
            .bytes
            .windows(marker.len())
            .position(|window| window == marker)
            .unwrap();
        payload_tampered.bytes[marker_position + 4] = b'o';
        let offsets = parse_xref_offsets(&payload_tampered.bytes).unwrap();
        for object in &mut payload_tampered.observation.objects {
            let offset = offsets[object.object_number as usize];
            let header = format!("{} 0 obj\n", object.object_number);
            let payload = payload_tampered.bytes[offset..]
                .strip_prefix(header.as_bytes())
                .unwrap();
            let end = payload
                .windows(b"\nendobj\n".len())
                .position(|window| window == b"\nendobj\n")
                .unwrap();
            object.sha256 = sha256(&payload[..end]);
        }
        payload_tampered.observation.pdf_sha256 = sha256(&payload_tampered.bytes);
        payload_tampered.observation.canonical_jcs =
            encode_observation(&payload_tampered.observation);
        payload_tampered.observation.fingerprint =
            sha256(payload_tampered.observation.canonical_jcs.as_bytes());
        assert_eq!(
            payload_tampered.verify(&navigation, &profile, &selected, &limits, &engine),
            Err(BookNavigationPdfError::ReceiptMismatch)
        );
    }

    #[test]
    fn metadata_outline_pdf_enforces_object_output_and_spool_limits() {
        let (_package, navigation, profile, limits, selected) = selected_fixture();
        let engine = EngineIdentity::compiled();
        let object_count = u32::try_from(
            selected.pages().len() * 2 + selected.links().len() + selected.entries().len() + 6,
        )
        .unwrap();
        let mut raw = limits.get().clone();
        raw.max_pdf_objects = object_count - 1;
        let object_limits = ValidatedResourceLimits::new(raw).unwrap();
        assert_eq!(
            PdfObjectPlan::new(&selected, &object_limits).unwrap_err(),
            BookNavigationPdfError::ObjectLimit
        );

        let pdf =
            write_staging_book_navigation_pdf(&navigation, &profile, &selected, &limits, &engine)
                .unwrap();
        assert!(pdf.bytes().len() < limits.get().max_output_bytes as usize);

        let one = BTreeMap::from([(1, ("only".to_owned(), b"<< >>".to_vec()))]);
        let exact = serialize_pdf(&one, 1, 1, u64::MAX).unwrap();
        assert_eq!(
            serialize_pdf(&one, 1, 1, exact.len() as u64).unwrap(),
            exact
        );
        assert_eq!(
            serialize_pdf(&one, 1, 1, (exact.len() - 1) as u64),
            Err(BookNavigationPdfError::OutputLimit)
        );
        assert_eq!(
            serialize_pdf(&BTreeMap::new(), u32::MAX, 1, u64::MAX),
            Err(BookNavigationPdfError::ObjectLimit)
        );
        assert!(matches!(
            PdfObjects::new(4, 5),
            Err(BookNavigationPdfError::SpoolLimit)
        ));
        let mut spool = PdfObjects::new(10, 5).unwrap();
        spool.insert(1, "exact", vec![0; 5]).unwrap();
        assert_eq!(
            spool.insert(2, "too_large", vec![0]),
            Err(BookNavigationPdfError::SpoolLimit)
        );
    }
}
