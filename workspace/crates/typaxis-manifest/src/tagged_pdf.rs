use typaxis_core::{push_jcs_string, sha256, EngineIdentity, ValidatedResourceLimits};
use typaxis_display_list::{
    BookNavigationSelectedReceipt, DestinationView, MarkedContentOwner, MarkedContentPlanReceipt,
    SelectedStructureBindingReceipt, StructureOwner, StructureParentTreeValue,
    StructureRegistryReceipt,
};
use typaxis_machine_profile::{
    StagingSemanticContainerSessionIdentity, StagingTaggedPdfProfileReceipt,
    STAGING_PDFUA1_PROFILE_ID,
};
use typaxis_pdf::StagingTaggedPdf;
use typaxis_syntax::{
    ValidatedStagingBookNavigation, ValidatedStagingSemanticPackage,
    ValidatedStagingStructureSemantics,
};

pub const STAGING_TAGGED_PDF_MANIFEST_ALGORITHM: &str = "typaxis.tagged-pdf-manifest/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTaggedPdfManifest {
    package_sha256: [u8; 32],
    profile_sha256: [u8; 32],
    structure_registry_sha256: [u8; 32],
    selected_binding_sha256: [u8; 32],
    marked_content_sha256: [u8; 32],
    pdf_observation_sha256: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingTaggedPdfManifest {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
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
    pub const fn pdf_observation_sha256(&self) -> [u8; 32] {
        self.pdf_observation_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigation,
        semantics: &ValidatedStagingStructureSemantics,
        profile: &StagingTaggedPdfProfileReceipt,
        session: &StagingSemanticContainerSessionIdentity,
        book: &BookNavigationSelectedReceipt,
        registry: &StructureRegistryReceipt,
        binding: &SelectedStructureBindingReceipt,
        marked: &MarkedContentPlanReceipt,
        pdf: &StagingTaggedPdf,
        limits: &ValidatedResourceLimits,
        engine: &EngineIdentity,
    ) -> Result<(), StagingTaggedPdfManifestError> {
        let observed = derive_manifest(
            package, navigation, semantics, profile, session, book, registry, binding, marked, pdf,
            limits, engine,
        )?;
        if self != &observed {
            return Err(StagingTaggedPdfManifestError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingTaggedPdfManifestError {
    ProfileMismatch,
    StructureMismatch,
    SelectedMismatch,
    MarkedContentMismatch,
    PdfMismatch,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingTaggedPdfManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileMismatch => formatter.write_str("I9190: accessibility profile mismatch"),
            Self::StructureMismatch => formatter.write_str("I9190: manifest structure mismatch"),
            Self::SelectedMismatch => {
                formatter.write_str("I9190: manifest selected binding mismatch")
            }
            Self::MarkedContentMismatch => {
                formatter.write_str("I9190: manifest marked content mismatch")
            }
            Self::PdfMismatch => formatter.write_str("I9190: manifest tagged PDF mismatch"),
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: tagged-PDF manifest receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingTaggedPdfManifestError {}

#[allow(clippy::too_many_arguments)]
pub fn build_staging_tagged_pdf_manifest(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    semantics: &ValidatedStagingStructureSemantics,
    profile: &StagingTaggedPdfProfileReceipt,
    session: &StagingSemanticContainerSessionIdentity,
    book: &BookNavigationSelectedReceipt,
    registry: &StructureRegistryReceipt,
    binding: &SelectedStructureBindingReceipt,
    marked: &MarkedContentPlanReceipt,
    pdf: &StagingTaggedPdf,
    limits: &ValidatedResourceLimits,
    engine: &EngineIdentity,
) -> Result<StagingTaggedPdfManifest, StagingTaggedPdfManifestError> {
    let value = derive_manifest(
        package, navigation, semantics, profile, session, book, registry, binding, marked, pdf,
        limits, engine,
    )?;
    value.verify(
        package, navigation, semantics, profile, session, book, registry, binding, marked, pdf,
        limits, engine,
    )?;
    Ok(value)
}

#[allow(clippy::too_many_arguments)]
fn derive_manifest(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    semantics: &ValidatedStagingStructureSemantics,
    profile: &StagingTaggedPdfProfileReceipt,
    session: &StagingSemanticContainerSessionIdentity,
    book: &BookNavigationSelectedReceipt,
    registry: &StructureRegistryReceipt,
    binding: &SelectedStructureBindingReceipt,
    marked: &MarkedContentPlanReceipt,
    pdf: &StagingTaggedPdf,
    limits: &ValidatedResourceLimits,
    engine: &EngineIdentity,
) -> Result<StagingTaggedPdfManifest, StagingTaggedPdfManifestError> {
    profile
        .verify(package, navigation, semantics, limits, session)
        .map_err(|_| StagingTaggedPdfManifestError::ProfileMismatch)?;
    registry
        .verify(
            package,
            navigation,
            semantics,
            profile.authorization(),
            limits,
        )
        .map_err(|_| StagingTaggedPdfManifestError::StructureMismatch)?;
    binding
        .verify(registry, profile.authorization(), limits)
        .map_err(|_| StagingTaggedPdfManifestError::SelectedMismatch)?;
    marked
        .verify(registry, binding, profile.authorization(), limits)
        .map_err(|_| StagingTaggedPdfManifestError::MarkedContentMismatch)?;
    pdf.verify(
        navigation,
        profile.base().authorization(),
        profile.authorization(),
        book,
        registry,
        binding,
        marked,
        limits,
        engine,
    )
    .map_err(|_| StagingTaggedPdfManifestError::PdfMismatch)?;
    let canonical_jcs = encode_manifest(
        package, navigation, profile, book, registry, binding, marked, pdf, engine,
    );
    Ok(StagingTaggedPdfManifest {
        package_sha256: package.canonical_jcs_sha256(),
        profile_sha256: profile.fingerprint(),
        structure_registry_sha256: registry.fingerprint(),
        selected_binding_sha256: binding.fingerprint(),
        marked_content_sha256: marked.fingerprint(),
        pdf_observation_sha256: pdf.observation().fingerprint(),
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

#[allow(clippy::too_many_arguments)]
fn encode_manifest(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    profile: &StagingTaggedPdfProfileReceipt,
    book: &BookNavigationSelectedReceipt,
    registry: &StructureRegistryReceipt,
    binding: &SelectedStructureBindingReceipt,
    marked: &MarkedContentPlanReceipt,
    pdf: &StagingTaggedPdf,
    engine: &EngineIdentity,
) -> String {
    let mut output = String::from("{\"accessibility_profile\":");
    push_jcs_string(&mut output, STAGING_PDFUA1_PROFILE_ID);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_TAGGED_PDF_MANIFEST_ALGORITHM);
    output.push_str(",\"contract\":\"typaxis.contract/1.4\",\"destinations\":[");
    encode_destinations(&mut output, book);
    output.push_str("],\"document_language\":");
    push_jcs_string(&mut output, navigation.languages().document_language());
    output.push_str(",\"engine\":{\"name\":");
    push_jcs_string(&mut output, engine.name());
    output.push_str(",\"version\":");
    push_jcs_string(&mut output, engine.version());
    output.push_str("},\"fingerprints\":{\"book_navigation_sha256\":");
    push_hash(&mut output, book.fingerprint());
    output.push_str(",\"destination_registry_sha256\":");
    push_hash(&mut output, book.destination_registry_sha256());
    output.push_str(",\"language_sha256\":");
    push_hash(&mut output, navigation.languages().fingerprint());
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, binding.limits_sha256());
    output.push_str(",\"marked_content_sha256\":");
    push_hash(&mut output, marked.fingerprint());
    output.push_str(",\"metadata_sha256\":");
    push_hash(&mut output, navigation.metadata().fingerprint());
    output.push_str(",\"outline_sha256\":");
    push_hash(&mut output, navigation.outline().fingerprint());
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, package.canonical_jcs_sha256());
    output.push_str(",\"pdf_observation_sha256\":");
    push_hash(&mut output, pdf.observation().fingerprint());
    output.push_str(",\"pdf_sha256\":");
    push_hash(&mut output, pdf.observation().pdf_sha256());
    output.push_str(",\"profile_sha256\":");
    push_hash(&mut output, profile.fingerprint());
    output.push_str(",\"selected_binding_sha256\":");
    push_hash(&mut output, binding.fingerprint());
    output.push_str(",\"semantic_sha256\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push_str(",\"structure_registry_sha256\":");
    push_hash(&mut output, registry.fingerprint());
    output.push_str(",\"xmp_sha256\":");
    push_hash(&mut output, pdf.observation().xmp_sha256());
    output.push_str("},\"marked_content\":{\"annotations\":[");
    encode_annotations(&mut output, marked, book);
    output.push_str("],\"pages\":[");
    encode_pages(&mut output, marked);
    output.push_str("],\"parent_tree\":[");
    encode_parent_tree(&mut output, marked);
    output.push_str("],\"records\":[");
    encode_marked_records(&mut output, marked);
    output.push_str("],\"selected_layout_fragment_count\":");
    output.push_str(&binding.selected_layout_fragment_count().to_string());
    output.push_str("},\"metadata\":");
    encode_metadata(&mut output, navigation);
    output.push_str(",\"outline\":[");
    for (index, entry) in book.entries().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        let structure = registry
            .source_node(entry.source_node_id())
            .expect("verified outline owner");
        output.push_str("{\"destination\":");
        push_jcs_string(&mut output, entry.destination().anchor_id.as_str());
        output.push_str(",\"label\":");
        push_jcs_string(&mut output, entry.label());
        output.push_str(",\"level\":");
        output.push_str(&entry.level().to_string());
        output.push_str(",\"outline_id\":");
        output.push_str(&entry.outline_id().to_string());
        output.push_str(",\"parent_outline_id\":");
        push_optional_u32(&mut output, entry.parent_outline_id());
        output.push_str(",\"source_node_id\":");
        output.push_str(&entry.source_node_id().get().to_string());
        output.push_str(",\"structure_node_id\":");
        output.push_str(&structure.structure_node_id().get().to_string());
        output.push('}');
    }
    let observation = pdf.observation();
    output.push_str("],\"pdf\":{\"artifact_count\":");
    output.push_str(&observation.artifact_count().to_string());
    output.push_str(",\"byte_length\":");
    output.push_str(&observation.pdf_byte_length().to_string());
    output.push_str(",\"id_tree_object\":");
    push_optional_u32(&mut output, observation.id_tree_object());
    output.push_str(",\"link_annotation_count\":");
    output.push_str(&observation.link_annotation_count().to_string());
    output.push_str(",\"marked_content_count\":");
    output.push_str(&observation.marked_content_count().to_string());
    output.push_str(",\"objects\":[");
    for (index, object) in observation.objects().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"object_number\":");
        output.push_str(&object.object_number().to_string());
        output.push_str(",\"role\":");
        push_jcs_string(&mut output, object.role());
        output.push_str(",\"sha256\":");
        push_hash(&mut output, object.sha256());
        output.push('}');
    }
    output.push_str("],\"parent_tree_object\":");
    output.push_str(&observation.parent_tree_object().to_string());
    output.push_str(",\"structure_element_count\":");
    output.push_str(&observation.structure_element_count().to_string());
    output.push_str(",\"structure_tree_root_object\":");
    output.push_str(&observation.structure_tree_root_object().to_string());
    output.push_str("},\"profile_id\":\"typaxis.machine-pdf/production-book-1\",\"structure\":[");
    encode_structure(&mut output, registry);
    output.push_str("],\"validators\":[\"typaxis.tagged-pdf-validator/1\",\"verapdf-greenfield/1.30.2:ua1\",\"typaxis.matterhorn-assessment/1\"]}");
    output
}

fn encode_destinations(output: &mut String, book: &BookNavigationSelectedReceipt) {
    for (index, binding) in book.destinations().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"anchor_id\":");
        push_jcs_string(output, binding.destination.anchor_id.as_str());
        output.push_str(",\"frame_id\":");
        output.push_str(&binding.frame_id.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&binding.destination.page_index.to_string());
        output.push_str(",\"source_node_id\":");
        output.push_str(&binding.source_node_id.get().to_string());
        output.push_str(",\"view\":");
        encode_view(output, &binding.destination.view);
        output.push('}');
    }
}

fn encode_view(output: &mut String, view: &DestinationView) {
    match view {
        DestinationView::Xyz { point } => {
            output.push_str("{\"kind\":\"xyz\",\"x\":");
            output.push_str(&point.x.raw().to_string());
            output.push_str(",\"y\":");
            output.push_str(&point.y.raw().to_string());
            output.push('}');
        }
        DestinationView::FitPage => output.push_str("{\"kind\":\"fit_page\"}"),
        DestinationView::FitWidth { top } => {
            output.push_str("{\"kind\":\"fit_width\",\"top\":");
            if let Some(top) = top {
                output.push_str(&top.raw().to_string());
            } else {
                output.push_str("null");
            }
            output.push('}');
        }
    }
}

fn encode_metadata(output: &mut String, navigation: &ValidatedStagingBookNavigation) {
    let metadata = navigation.metadata().metadata();
    output.push_str("{\"author\":");
    push_optional_string(output, metadata.author.as_deref());
    output.push_str(",\"created\":");
    push_optional_string(output, metadata.created.as_deref());
    output.push_str(",\"identifier\":");
    push_optional_string(output, metadata.identifier.as_deref());
    output.push_str(",\"keywords\":[");
    for (index, keyword) in metadata.keywords.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(output, keyword);
    }
    output.push_str("],\"modified\":");
    push_optional_string(output, metadata.modified.as_deref());
    output.push_str(",\"subject\":");
    push_optional_string(output, metadata.subject.as_deref());
    output.push_str(",\"title\":");
    push_optional_string(output, metadata.title.as_deref());
    output.push('}');
}

fn encode_annotations(
    output: &mut String,
    marked: &MarkedContentPlanReceipt,
    book: &BookNavigationSelectedReceipt,
) {
    for (index, (value, link)) in marked.annotations().iter().zip(book.links()).enumerate() {
        if index != 0 {
            output.push(',');
        }
        let page = &book.pages()[link.page_index() as usize];
        let right = link.x_raw() + link.width_raw();
        let bottom = page.height_raw - (link.y_raw() + link.height_raw());
        let top = page.height_raw - link.y_raw();
        output.push_str("{\"accessible_name\":");
        push_jcs_string(output, value.accessible_name());
        output.push_str(",\"annotation_id\":");
        output.push_str(&value.annotation_id().to_string());
        output.push_str(",\"destination\":");
        push_jcs_string(output, link.destination().as_str());
        output.push_str(",\"page_index\":");
        output.push_str(&value.page_index().to_string());
        output.push_str(",\"rect\":[");
        for (coordinate_index, coordinate) in [link.x_raw(), bottom, right, top].iter().enumerate()
        {
            if coordinate_index != 0 {
                output.push(',');
            }
            output.push_str(&coordinate.to_string());
        }
        output.push(']');
        output.push_str(",\"structure_node_id\":");
        output.push_str(&value.structure_node_id().get().to_string());
        output.push_str(",\"structure_parent_key\":");
        output.push_str(&value.structure_parent_key().to_string());
        output.push('}');
    }
}

fn encode_pages(output: &mut String, marked: &MarkedContentPlanReceipt) {
    for (index, value) in marked.pages().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"artifact_count\":");
        output.push_str(&value.artifact_count().to_string());
        output.push_str(",\"height_raw\":");
        output.push_str(&value.height_raw().to_string());
        output.push_str(",\"marked_content_count\":");
        output.push_str(&value.marked_content_count().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&value.page_index().to_string());
        output.push_str(",\"structure_parent_key\":");
        push_optional_u32(output, value.structure_parent_key());
        output.push_str(",\"width_raw\":");
        output.push_str(&value.width_raw().to_string());
        output.push('}');
    }
}

fn encode_parent_tree(output: &mut String, marked: &MarkedContentPlanReceipt) {
    for (index, value) in marked.parent_tree().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"key\":");
        output.push_str(&value.key().to_string());
        match value.value() {
            StructureParentTreeValue::Page(nodes) => {
                output.push_str(",\"kind\":\"page\",\"structure_node_ids\":[");
                for (node_index, node) in nodes.iter().enumerate() {
                    if node_index != 0 {
                        output.push(',');
                    }
                    output.push_str(&node.get().to_string());
                }
                output.push(']');
            }
            StructureParentTreeValue::Annotation(node) => {
                output.push_str(",\"kind\":\"annotation\",\"structure_node_id\":");
                output.push_str(&node.get().to_string());
            }
        }
        output.push('}');
    }
}

fn encode_marked_records(output: &mut String, marked: &MarkedContentPlanReceipt) {
    for (index, value) in marked.records().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"actual_text\":");
        push_optional_string(output, value.actual_text());
        output.push_str(",\"language\":");
        push_optional_string(output, value.language());
        output.push_str(",\"owner\":");
        match value.owner() {
            MarkedContentOwner::Structure(owner) => {
                output.push_str("{\"kind\":\"structure\",\"mcid\":");
                output.push_str(&owner.mcid().to_string());
                output.push_str(",\"role\":");
                push_jcs_string(output, owner.role().pdf_name());
                output.push_str(",\"structure_node_id\":");
                output.push_str(&owner.structure_node_id().get().to_string());
                output.push('}');
            }
            MarkedContentOwner::Artifact(owner) => {
                output.push_str("{\"class\":");
                push_jcs_string(output, owner.class().as_str());
                output.push_str(",\"kind\":\"artifact\",\"occurrence\":");
                output.push_str(&owner.occurrence().to_string());
                output.push('}');
            }
        }
        output.push_str(",\"page_index\":");
        output.push_str(&value.page_index().to_string());
        output.push_str(",\"paint_ordinal_start\":");
        output.push_str(&value.paint_ordinal_start().to_string());
        output.push_str(",\"selected_paint_ids\":[");
        for (paint_index, paint_id) in value.selected_paint_ids().iter().enumerate() {
            if paint_index != 0 {
                output.push(',');
            }
            output.push_str(&paint_id.to_string());
        }
        output.push(']');
        output.push_str(",\"semantic_fragment_ordinal\":");
        output.push_str(&value.semantic_fragment_ordinal().to_string());
        output.push('}');
    }
}

fn encode_structure(output: &mut String, registry: &StructureRegistryReceipt) {
    for (index, value) in registry.nodes().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"accessible_name\":");
        push_optional_string(output, value.accessible_name());
        output.push_str(",\"actual_text\":");
        push_optional_string(output, value.actual_text());
        output.push_str(",\"alternative\":");
        push_optional_string(output, value.alternative());
        output.push_str(",\"children\":[");
        for (child_index, child) in value.children().iter().enumerate() {
            if child_index != 0 {
                output.push(',');
            }
            output.push_str(&child.get().to_string());
        }
        output.push_str("],\"language\":");
        push_jcs_string(output, value.language());
        output.push_str(",\"list_numbering\":");
        if let Some(numbering) = value.list_numbering() {
            push_jcs_string(output, numbering.as_str());
        } else {
            output.push_str("null");
        }
        output.push_str(",\"marker\":");
        push_optional_string(output, value.marker());
        output.push_str(",\"outline_ids\":[");
        for (outline_index, outline) in value.outline_ids().iter().enumerate() {
            if outline_index != 0 {
                output.push(',');
            }
            output.push_str(&outline.to_string());
        }
        output.push_str("],\"owner\":");
        match value.owner() {
            StructureOwner::Source(node) => {
                output.push_str("{\"kind\":\"source\",\"node_id\":");
                output.push_str(&node.get().to_string());
                output.push('}');
            }
            StructureOwner::Generated(key) => {
                output.push_str("{\"kind\":\"generated\",\"ordinal\":");
                output.push_str(&key.ordinal().to_string());
                output.push_str(",\"owner_node_id\":");
                output.push_str(&key.owner_node_id().get().to_string());
                output.push_str(",\"slot\":");
                push_jcs_string(output, key.slot().as_str());
                output.push('}');
            }
        }
        output.push_str(",\"paint_required\":");
        output.push_str(if value.paint_required() {
            "true"
        } else {
            "false"
        });
        output.push_str(",\"parent\":");
        push_optional_u32(output, value.parent().map(|parent| parent.get()));
        output.push_str(",\"related_nodes\":[");
        for (related_index, related) in value.related_nodes().iter().enumerate() {
            if related_index != 0 {
                output.push(',');
            }
            output.push_str(&related.get().to_string());
        }
        output.push_str("],\"role\":");
        push_jcs_string(output, value.role().pdf_name());
        output.push_str(",\"source_span\":");
        if let Some(span) = value.source_span() {
            output.push_str("{\"end_byte\":");
            output.push_str(&span.end_byte().get().to_string());
            output.push_str(",\"source_id\":");
            output.push_str(&span.source_id().get().to_string());
            output.push_str(",\"start_byte\":");
            output.push_str(&span.start_byte().get().to_string());
            output.push('}');
        } else {
            output.push_str("null");
        }
        output.push_str(",\"structure_id\":");
        push_optional_string(output, value.structure_id());
        output.push_str(",\"structure_node_id\":");
        output.push_str(&value.structure_node_id().get().to_string());
        output.push_str(",\"table\":");
        if let Some(table) = value.table_attributes() {
            output.push_str("{\"colspan\":");
            output.push_str(&table.colspan().to_string());
            output.push_str(",\"column_ordinal\":");
            output.push_str(&table.column_ordinal().to_string());
            output.push_str(",\"header_ids\":[");
            for (header_index, header) in table.header_ids().iter().enumerate() {
                if header_index != 0 {
                    output.push(',');
                }
                push_jcs_string(output, header);
            }
            output.push_str("],\"row_ordinal\":");
            output.push_str(&table.row_ordinal().to_string());
            output.push_str(",\"rowspan\":");
            output.push_str(&table.rowspan().to_string());
            output.push_str(",\"section\":");
            push_jcs_string(output, table.section().as_str());
            output.push('}');
        } else {
            output.push_str("null");
        }
        output.push('}');
    }
}

fn push_optional_string(output: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        push_jcs_string(output, value);
    } else {
        output.push_str("null");
    }
}

fn push_optional_u32(output: &mut String, value: Option<u32>) {
    if let Some(value) = value {
        output.push_str(&value.to_string());
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
