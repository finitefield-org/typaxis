use typaxis_core::{push_jcs_string, sha256, ImageResourceId, NodeId};
use typaxis_display_list::StagingSemanticContainerDisplay;
use typaxis_document::{FontMediaDeclaration, ImageMediaDeclaration, StagingM4Block};
use typaxis_layout::StagingSemanticContainerSelectedLayout;
use typaxis_machine_profile::StagingSemanticContainerPreflightReceipt;
use typaxis_pdf::StagingSemanticContainerPdf;
use typaxis_resource_admission::StagingDeclaredMediaLedger;
use typaxis_syntax::ValidatedStagingSemanticPackage;

const MANIFEST_ALGORITHM: &str = "typaxis.semantic-container-manifest/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingDeclaredMediaManifestRecord {
    resource_kind: &'static str,
    resource_id: u32,
    declaration_kind: &'static str,
    media_type: &'static str,
    attested_media_kind: &'static str,
    sha256: [u8; 32],
}

impl StagingDeclaredMediaManifestRecord {
    pub const fn resource_kind(&self) -> &'static str {
        self.resource_kind
    }
    pub const fn resource_id(&self) -> u32 {
        self.resource_id
    }
    pub const fn declaration_kind(&self) -> &'static str {
        self.declaration_kind
    }
    pub const fn media_type(&self) -> &'static str {
        self.media_type
    }
    pub const fn attested_media_kind(&self) -> &'static str {
        self.attested_media_kind
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerSelectedFact {
    owner: NodeId,
    fragment_index: u32,
    page_index: u32,
    semantic_kind: &'static str,
    source_id: u32,
    source_start: u32,
    source_end: u32,
    style_fingerprint: [u8; 32],
    selected_fragment_fingerprint: [u8; 32],
    display_paint_fingerprint: [u8; 32],
    raster_fingerprint: [u8; 32],
    pdf_content_stream_fingerprint: [u8; 32],
    child_owners: Vec<NodeId>,
}

impl StagingSemanticContainerSelectedFact {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn semantic_kind(&self) -> &'static str {
        self.semantic_kind
    }
    pub const fn selected_fragment_fingerprint(&self) -> [u8; 32] {
        self.selected_fragment_fingerprint
    }
    pub const fn display_paint_fingerprint(&self) -> [u8; 32] {
        self.display_paint_fingerprint
    }
    pub const fn raster_fingerprint(&self) -> [u8; 32] {
        self.raster_fingerprint
    }
    pub const fn pdf_content_stream_fingerprint(&self) -> [u8; 32] {
        self.pdf_content_stream_fingerprint
    }
    pub fn child_owners(&self) -> &[NodeId] {
        &self.child_owners
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerManifest {
    selected_layout_fingerprint: [u8; 32],
    display_fingerprint: [u8; 32],
    pdf_fingerprint: [u8; 32],
    declared_media_fingerprint: [u8; 32],
    selected_facts: Vec<StagingSemanticContainerSelectedFact>,
    resources: Vec<StagingDeclaredMediaManifestRecord>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSemanticContainerManifest {
    pub const fn selected_layout_fingerprint(&self) -> [u8; 32] {
        self.selected_layout_fingerprint
    }
    pub const fn display_fingerprint(&self) -> [u8; 32] {
        self.display_fingerprint
    }
    pub const fn pdf_fingerprint(&self) -> [u8; 32] {
        self.pdf_fingerprint
    }
    pub const fn declared_media_fingerprint(&self) -> [u8; 32] {
        self.declared_media_fingerprint
    }
    pub fn selected_facts(&self) -> &[StagingSemanticContainerSelectedFact] {
        &self.selected_facts
    }
    pub fn resources(&self) -> &[StagingDeclaredMediaManifestRecord] {
        &self.resources
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingSemanticContainerPreflightReceipt,
        selected: &StagingSemanticContainerSelectedLayout,
        display: &StagingSemanticContainerDisplay,
        pdf: &StagingSemanticContainerPdf,
        media: &StagingDeclaredMediaLedger,
    ) -> Result<(), StagingSemanticContainerManifestError> {
        let expected = derive_staging_semantic_container_manifest(
            package, profile, selected, display, pdf, media,
        )?;
        if *self != expected {
            return Err(StagingSemanticContainerManifestError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSemanticContainerManifestError {
    SelectedMismatch,
    DisplayMismatch,
    PdfMismatch,
    MissingMediaAttestation,
    MediaMismatch,
    ReceiptMismatch,
    ArithmeticOverflow,
    PrecomposedVectorStaging(NodeId),
    SvgSafe2Staging(ImageResourceId),
}

impl std::fmt::Display for StagingSemanticContainerManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelectedMismatch => {
                formatter.write_str("I9190: manifest selected-layout mismatch")
            }
            Self::DisplayMismatch => formatter.write_str("I9190: manifest Display mismatch"),
            Self::PdfMismatch => formatter.write_str("I9190: manifest PDF mismatch"),
            Self::MissingMediaAttestation => {
                formatter.write_str("R7100: manifest media attestation is missing")
            }
            Self::MediaMismatch => {
                formatter.write_str("R7100: manifest declared/attested media mismatch")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: semantic manifest receipt mismatch")
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("I9190: semantic manifest arithmetic overflow")
            }
            Self::PrecomposedVectorStaging(owner) => write!(
                formatter,
                "P1102: precomposed vector at node {} requires the versioned manifest",
                owner.get()
            ),
            Self::SvgSafe2Staging(id) => write!(
                formatter,
                "P1102: svg-safe-2 image {} requires the versioned manifest",
                id.get()
            ),
        }
    }
}

impl std::error::Error for StagingSemanticContainerManifestError {}

pub fn build_staging_semantic_container_manifest(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSemanticContainerPreflightReceipt,
    selected: &StagingSemanticContainerSelectedLayout,
    display: &StagingSemanticContainerDisplay,
    pdf: &StagingSemanticContainerPdf,
    media: &StagingDeclaredMediaLedger,
) -> Result<StagingSemanticContainerManifest, StagingSemanticContainerManifestError> {
    let manifest = derive_staging_semantic_container_manifest(
        package, profile, selected, display, pdf, media,
    )?;
    manifest.verify(package, profile, selected, display, pdf, media)?;
    Ok(manifest)
}

fn derive_staging_semantic_container_manifest(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSemanticContainerPreflightReceipt,
    selected: &StagingSemanticContainerSelectedLayout,
    display: &StagingSemanticContainerDisplay,
    pdf: &StagingSemanticContainerPdf,
    media: &StagingDeclaredMediaLedger,
) -> Result<StagingSemanticContainerManifest, StagingSemanticContainerManifestError> {
    if let Some(owner) = first_precomposed_vector_owner(&package.document().blocks).or_else(|| {
        package
            .document()
            .footnotes
            .iter()
            .find_map(|footnote| first_precomposed_vector_owner(&footnote.blocks))
    }) {
        return Err(StagingSemanticContainerManifestError::PrecomposedVectorStaging(owner));
    }
    if let Some(image) = package.resources().images.iter().find(|image| {
        image.media == ImageMediaDeclaration::Declared(typaxis_document::ImageMediaType::SvgSafe2)
    }) {
        return Err(StagingSemanticContainerManifestError::SvgSafe2Staging(
            image.image_id,
        ));
    }
    selected
        .verify(package, profile.authorization())
        .map_err(|_| StagingSemanticContainerManifestError::SelectedMismatch)?;
    display
        .verify(package, profile.authorization())
        .map_err(|_| StagingSemanticContainerManifestError::DisplayMismatch)?;
    pdf.verify(package, profile.authorization(), display)
        .map_err(|_| StagingSemanticContainerManifestError::PdfMismatch)?;
    if display.receipt().selected_layout_fingerprint() != selected.receipt().fingerprint()
        || pdf.receipt().selected_layout_fingerprint() != selected.receipt().fingerprint()
    {
        return Err(StagingSemanticContainerManifestError::ReceiptMismatch);
    }
    let resources = close_media_records(package, media)?;
    let mut selected_facts = Vec::new();
    for ((fragment, page), pdf_page) in selected
        .fragments()
        .iter()
        .zip(display.pages())
        .zip(pdf.pages())
    {
        let paint = page
            .paints()
            .first()
            .ok_or(StagingSemanticContainerManifestError::DisplayMismatch)?;
        if page.paints().len() != 1
            || paint.owner() != fragment.owner()
            || paint.selected_fragment_fingerprint() != fragment.fingerprint()
            || pdf_page.owner() != fragment.owner()
            || pdf_page.fragment_index() != fragment.fragment_index()
            || pdf_page.display_paint_fingerprint() != paint.fingerprint()
            || pdf_page.raster_fingerprint() != page.raster_observation().raster_fingerprint()
        {
            return Err(StagingSemanticContainerManifestError::ReceiptMismatch);
        }
        let span = fragment.source_span();
        selected_facts.push(StagingSemanticContainerSelectedFact {
            owner: fragment.owner(),
            fragment_index: fragment.fragment_index(),
            page_index: fragment.page_index(),
            semantic_kind: fragment.semantic_kind().as_str(),
            source_id: span.source_id().get(),
            source_start: span.start_byte().get(),
            source_end: span.end_byte().get(),
            style_fingerprint: fragment.style_fingerprint(),
            selected_fragment_fingerprint: fragment.fingerprint(),
            display_paint_fingerprint: paint.fingerprint(),
            raster_fingerprint: page.raster_observation().raster_fingerprint(),
            pdf_content_stream_fingerprint: pdf_page.content_stream_fingerprint(),
            child_owners: fragment.child_owners().to_vec(),
        });
    }
    if selected_facts.len() != selected.fragments().len()
        || selected_facts.len() != display.pages().len()
        || selected_facts.len() != pdf.pages().len()
    {
        return Err(StagingSemanticContainerManifestError::ReceiptMismatch);
    }
    let canonical_jcs = encode_manifest(
        selected.receipt().fingerprint(),
        display.receipt().fingerprint(),
        pdf.receipt().fingerprint(),
        media.fingerprint(),
        &selected_facts,
        &resources,
    );
    Ok(StagingSemanticContainerManifest {
        selected_layout_fingerprint: selected.receipt().fingerprint(),
        display_fingerprint: display.receipt().fingerprint(),
        pdf_fingerprint: pdf.receipt().fingerprint(),
        declared_media_fingerprint: media.fingerprint(),
        selected_facts,
        resources,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn first_precomposed_vector_owner(blocks: &[StagingM4Block]) -> Option<NodeId> {
    for block in blocks {
        let owner = match block {
            StagingM4Block::Paragraph { inline_vectors, .. }
            | StagingM4Block::Heading { inline_vectors, .. } => {
                inline_vectors.first().map(|vector| vector.node_id)
            }
            StagingM4Block::VectorFigure { common, .. }
            | StagingM4Block::MathVectorBlock { common, .. } => Some(common.node_id),
            StagingM4Block::List { items, .. } => items
                .iter()
                .find_map(|item| first_precomposed_vector_owner(&item.blocks)),
            StagingM4Block::Table { head, body, .. } => head
                .iter()
                .chain(body)
                .flat_map(|row| &row.cells)
                .find_map(|cell| first_precomposed_vector_owner(&cell.blocks)),
            StagingM4Block::Figure { caption, .. } => first_precomposed_vector_owner(caption),
            StagingM4Block::SemanticContainer { blocks, .. } => {
                first_precomposed_vector_owner(blocks)
            }
            StagingM4Block::PageBreak { .. } | StagingM4Block::DisplayMath { .. } => None,
        };
        if owner.is_some() {
            return owner;
        }
    }
    None
}

fn close_media_records(
    package: &ValidatedStagingSemanticPackage,
    media: &StagingDeclaredMediaLedger,
) -> Result<Vec<StagingDeclaredMediaManifestRecord>, StagingSemanticContainerManifestError> {
    if package.resources().font_faces.len() != media.fonts().len()
        || package.resources().images.len() != media.images().len()
    {
        return Err(StagingSemanticContainerManifestError::MissingMediaAttestation);
    }
    let mut records = Vec::new();
    records
        .try_reserve_exact(
            package
                .resources()
                .font_faces
                .len()
                .checked_add(package.resources().images.len())
                .ok_or(StagingSemanticContainerManifestError::ArithmeticOverflow)?,
        )
        .map_err(|_| StagingSemanticContainerManifestError::ArithmeticOverflow)?;
    for (declaration, attestation) in package.resources().font_faces.iter().zip(media.fonts()) {
        let FontMediaDeclaration::Declared(declared) = declaration.media else {
            return Err(StagingSemanticContainerManifestError::MediaMismatch);
        };
        if declaration.font_face_id != attestation.font_face_id()
            || &declaration.uri != attestation.uri()
            || declaration.family != attestation.family()
            || declaration.face_index != attestation.face_index()
            || declared != attestation.declared()
            || !matches!(
                (declared, attestation.attested()),
                (
                    typaxis_document::FontMediaType::SfntTrueTypeGlyf,
                    typaxis_resource_admission::AdmittedFontMediaKind::SfntTrueTypeGlyf
                ) | (
                    typaxis_document::FontMediaType::TtcTrueTypeGlyf,
                    typaxis_resource_admission::AdmittedFontMediaKind::TtcTrueTypeGlyf
                )
            )
            || declaration
                .expected_sha256
                .is_some_and(|hash| hash != attestation.content_hash())
        {
            return Err(StagingSemanticContainerManifestError::MediaMismatch);
        }
        records.push(StagingDeclaredMediaManifestRecord {
            resource_kind: "font",
            resource_id: declaration.font_face_id.get(),
            declaration_kind: "declared",
            media_type: declared.as_str(),
            attested_media_kind: attestation.attested().as_str(),
            sha256: attestation.content_hash(),
        });
    }
    for (declaration, attestation) in package.resources().images.iter().zip(media.images()) {
        let ImageMediaDeclaration::Declared(declared) = declaration.media else {
            return Err(StagingSemanticContainerManifestError::MediaMismatch);
        };
        if declaration.image_id != attestation.image_id()
            || &declaration.uri != attestation.uri()
            || declared != attestation.declared()
            || !matches!(
                (declared, attestation.attested()),
                (
                    typaxis_document::ImageMediaType::Png,
                    typaxis_resource_admission::AdmittedImageMediaKind::Png
                )
            )
            || declaration
                .expected_sha256
                .is_some_and(|hash| hash != attestation.content_hash())
        {
            return Err(StagingSemanticContainerManifestError::MediaMismatch);
        }
        records.push(StagingDeclaredMediaManifestRecord {
            resource_kind: "image",
            resource_id: declaration.image_id.get(),
            declaration_kind: "declared",
            media_type: declared.as_str(),
            attested_media_kind: attestation.attested().as_str(),
            sha256: attestation.content_hash(),
        });
    }
    Ok(records)
}

fn encode_manifest(
    selected: [u8; 32],
    display: [u8; 32],
    pdf: [u8; 32],
    media: [u8; 32],
    facts: &[StagingSemanticContainerSelectedFact],
    resources: &[StagingDeclaredMediaManifestRecord],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MANIFEST_ALGORITHM);
    output.push_str(",\"declared_media_fingerprint\":");
    push_hash(&mut output, media);
    output.push_str(",\"display_fingerprint\":");
    push_hash(&mut output, display);
    output.push_str(",\"pdf_fingerprint\":");
    push_hash(&mut output, pdf);
    output.push_str(",\"resources\":[");
    for (index, resource) in resources.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"attested_media_kind\":");
        push_jcs_string(&mut output, resource.attested_media_kind);
        output.push_str(",\"media_declaration\":{\"kind\":");
        push_jcs_string(&mut output, resource.declaration_kind);
        output.push_str(",\"media_type\":");
        push_jcs_string(&mut output, resource.media_type);
        output.push('}');
        output.push_str(",\"resource_id\":");
        output.push_str(&resource.resource_id.to_string());
        output.push_str(",\"resource_kind\":");
        push_jcs_string(&mut output, resource.resource_kind);
        output.push_str(",\"sha256\":");
        push_hash(&mut output, resource.sha256);
        output.push('}');
    }
    output.push_str("],\"selected_facts\":[");
    for (index, fact) in facts.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"child_owners\":[");
        for (child_index, owner) in fact.child_owners.iter().enumerate() {
            if child_index > 0 {
                output.push(',');
            }
            output.push_str(&owner.get().to_string());
        }
        output.push_str("],\"display_paint_fingerprint\":");
        push_hash(&mut output, fact.display_paint_fingerprint);
        output.push_str(",\"fragment_index\":");
        output.push_str(&fact.fragment_index.to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(&mut output, fact.semantic_kind);
        output.push_str(",\"owner\":");
        output.push_str(&fact.owner.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&fact.page_index.to_string());
        output.push_str(",\"pdf_content_stream_fingerprint\":");
        push_hash(&mut output, fact.pdf_content_stream_fingerprint);
        output.push_str(",\"raster_fingerprint\":");
        push_hash(&mut output, fact.raster_fingerprint);
        output.push_str(",\"selected_fragment_fingerprint\":");
        push_hash(&mut output, fact.selected_fragment_fingerprint);
        output.push_str(",\"source_span\":{\"end_byte\":");
        output.push_str(&fact.source_end.to_string());
        output.push_str(",\"source_id\":");
        output.push_str(&fact.source_id.to_string());
        output.push_str(",\"start_byte\":");
        output.push_str(&fact.source_start.to_string());
        output.push('}');
        output.push_str(",\"style_fingerprint\":");
        push_hash(&mut output, fact.style_fingerprint);
        output.push('}');
    }
    output.push_str("],\"selected_layout_fingerprint\":");
    push_hash(&mut output, selected);
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
