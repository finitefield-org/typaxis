use typaxis_core::{push_jcs_string, sha256, EngineIdentity, M4EffectiveResourceLimits, NodeId};
use typaxis_display_list::{
    BookNavigationSelectedReceiptV2, MarkedContentBindingKindV2, MarkedContentOwner,
    StagingCombinedVectorKindV2, StagingPrecomposedVectorDisplay, StructureOwner,
    StructureRegistryReceiptV2, StructureRole, VectorFormStructureIsolationReceiptV2,
    VectorMarkedContentPlanV2,
};
use typaxis_layout::StagingMathVectorFlowRegistry;
use typaxis_machine_profile::STAGING_PDFUA1_PROFILE_ID_V2;
use typaxis_pagination::StagingAtomicVectorBlockSelectedLayout;
use typaxis_pdf::StagingTaggedPdfV2;
use typaxis_syntax::{
    PrecomposedVectorKind, StagingAccessibilityProfileAuthorizationV2,
    StagingBookNavigationProfileAuthorizationV2, ValidatedStagingBookNavigationV2,
    ValidatedStagingSemanticPackage, ValidatedStagingStructureSemanticsV2,
};

use crate::{StagingMathVectorManifest, StagingSafeVectorManifestV2};

pub const STAGING_TAGGED_PDF_MANIFEST_V2_ALGORITHM: &str = "typaxis.tagged-pdf-manifest/2";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTaggedPdfVectorStructureFactV2 {
    structure_node_id: u32,
    owner: NodeId,
    kind: StagingCombinedVectorKindV2,
    role: &'static str,
    language: String,
    metrics_fingerprint: Option<[u8; 32]>,
    safe_vector_usage_fingerprint: [u8; 32],
    marked_content_record_fingerprint: [u8; 32],
    math_binding_fingerprint: Option<[u8; 32]>,
    canonical_jcs: String,
}

impl StagingTaggedPdfVectorStructureFactV2 {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn kind(&self) -> StagingCombinedVectorKindV2 {
        self.kind
    }
    pub const fn math_binding_fingerprint(&self) -> Option<[u8; 32]> {
        self.math_binding_fingerprint
    }
    pub const fn safe_vector_usage_fingerprint(&self) -> [u8; 32] {
        self.safe_vector_usage_fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTaggedPdfManifestV2 {
    package_sha256: [u8; 32],
    safe_vector_manifest_fingerprint: [u8; 32],
    math_vector_manifest_fingerprint: [u8; 32],
    structure_registry_fingerprint: [u8; 32],
    selected_binding_fingerprint: [u8; 32],
    marked_content_fingerprint: [u8; 32],
    pdf_observation_fingerprint: [u8; 32],
    final_pdf_sha256: [u8; 32],
    vector_structures: Vec<StagingTaggedPdfVectorStructureFactV2>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingTaggedPdfManifestV2 {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn safe_vector_manifest_fingerprint(&self) -> [u8; 32] {
        self.safe_vector_manifest_fingerprint
    }
    pub const fn math_vector_manifest_fingerprint(&self) -> [u8; 32] {
        self.math_vector_manifest_fingerprint
    }
    pub fn vector_structures(&self) -> &[StagingTaggedPdfVectorStructureFactV2] {
        &self.vector_structures
    }
    pub const fn final_pdf_sha256(&self) -> [u8; 32] {
        self.final_pdf_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingTaggedPdfManifestV2Error {
    ProfileMismatch,
    StructureMismatch,
    MarkedContentMismatch,
    SafeVectorMismatch,
    MathVectorMismatch,
    PdfMismatch,
    AllocationFailure,
    ReceiptMismatch,
}
impl std::fmt::Display for StagingTaggedPdfManifestV2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "I9190: tagged-PDF manifest /2 {:?}", self)
    }
}
impl std::error::Error for StagingTaggedPdfManifestV2Error {}

#[allow(clippy::too_many_arguments)]
pub fn build_staging_tagged_pdf_manifest_v2(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    semantics: &ValidatedStagingStructureSemanticsV2,
    profile: &StagingAccessibilityProfileAuthorizationV2,
    book_profile: &StagingBookNavigationProfileAuthorizationV2,
    book: &BookNavigationSelectedReceiptV2,
    registry: &StructureRegistryReceiptV2,
    vector_plan: &VectorMarkedContentPlanV2,
    display: &StagingPrecomposedVectorDisplay,
    form_isolation: &VectorFormStructureIsolationReceiptV2,
    block_selected: &StagingAtomicVectorBlockSelectedLayout,
    math_flows: &StagingMathVectorFlowRegistry,
    pdf: &StagingTaggedPdfV2,
    safe_vector: &StagingSafeVectorManifestV2,
    math_vector: &StagingMathVectorManifest,
    limits: &M4EffectiveResourceLimits,
    engine: &EngineIdentity,
) -> Result<StagingTaggedPdfManifestV2, StagingTaggedPdfManifestV2Error> {
    profile
        .authorizes(package, navigation, semantics, limits)
        .map_err(|_| StagingTaggedPdfManifestV2Error::ProfileMismatch)?;
    registry
        .verify(package, navigation, semantics, profile, limits)
        .map_err(|_| StagingTaggedPdfManifestV2Error::StructureMismatch)?;
    vector_plan
        .verify(
            registry,
            profile,
            limits,
            navigation,
            book_profile,
            book,
            display,
            form_isolation,
            block_selected,
            math_flows,
        )
        .map_err(|_| StagingTaggedPdfManifestV2Error::MarkedContentMismatch)?;
    if safe_vector.package_fingerprint() != package.semantic_fingerprint()
        || math_vector.package_fingerprint() != package.semantic_fingerprint()
        || math_vector.safe_vector_manifest_fingerprint() != safe_vector.fingerprint()
        || safe_vector.final_pdf_sha256() != pdf.final_pdf().content_hash()
        || pdf.observation().structure_registry_sha256() != registry.fingerprint()
        || pdf.observation().selected_binding_sha256()
            != vector_plan.selected_binding().fingerprint()
        || pdf.observation().marked_content_sha256() != vector_plan.marked_content().fingerprint()
        || pdf.observation().pdf_sha256() != pdf.final_pdf().content_hash()
    {
        return Err(StagingTaggedPdfManifestV2Error::PdfMismatch);
    }
    let marked = vector_plan.marked_content();
    let expected_placements = usize::try_from(safe_vector.placement_count())
        .map_err(|_| StagingTaggedPdfManifestV2Error::StructureMismatch)?;
    let mut vector_structures = Vec::new();
    vector_structures
        .try_reserve_exact(expected_placements)
        .map_err(|_| StagingTaggedPdfManifestV2Error::AllocationFailure)?;
    for usage in safe_vector
        .resources()
        .iter()
        .flat_map(|resource| resource.placements())
    {
        let owner = usage.owner();
        let node = registry
            .source_node(owner)
            .filter(|node| node.owner() == StructureOwner::Source(owner))
            .ok_or(StagingTaggedPdfManifestV2Error::StructureMismatch)?;
        let (metrics_fingerprint, math_binding_fingerprint) = match usage.kind() {
            StagingCombinedVectorKindV2::Figure => {
                if node.role() != StructureRole::Figure
                    || node.vector_binding_v2().is_some()
                    || usage.binding_fingerprint().is_some()
                    || usage.metric_receipt_fingerprint().is_some()
                {
                    return Err(StagingTaggedPdfManifestV2Error::StructureMismatch);
                }
                (None, None)
            }
            kind => {
                if usage.binding_fingerprint().is_none() {
                    return Err(StagingTaggedPdfManifestV2Error::StructureMismatch);
                }
                let binding = node
                    .vector_binding_v2()
                    .filter(|binding| Some(binding.kind()) == kind.precomposed())
                    .ok_or(StagingTaggedPdfManifestV2Error::StructureMismatch)?;
                if usage.metric_receipt_fingerprint() != Some(binding.metrics_fingerprint()) {
                    return Err(StagingTaggedPdfManifestV2Error::StructureMismatch);
                }
                let math_binding_fingerprint =
                    match binding.kind() {
                        PrecomposedVectorKind::MathVector
                        | PrecomposedVectorKind::MathVectorBlock => Some(
                            math_vector
                                .fact(owner)
                                .ok_or(StagingTaggedPdfManifestV2Error::MathVectorMismatch)?
                                .math_binding_fingerprint(),
                        ),
                        PrecomposedVectorKind::InlineVector
                        | PrecomposedVectorKind::VectorFigure => None,
                    };
                (
                    Some(binding.metrics_fingerprint()),
                    math_binding_fingerprint,
                )
            }
        };
        let record = marked
            .records()
            .iter()
            .find(|record| {
                record.page_index() == usage.page_index()
                    && record.paint_ordinal_start() == usage.paint_ordinal()
                    && match (usage.kind(), record.binding()) {
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
                                && display_command_fingerprint
                                    == usage.display_command_fingerprint()
                        }
                        _ => false,
                    }
            })
            .ok_or(StagingTaggedPdfManifestV2Error::MarkedContentMismatch)?;
        let MarkedContentOwner::Structure(marked_owner) = record.owner() else {
            return Err(StagingTaggedPdfManifestV2Error::MarkedContentMismatch);
        };
        if marked_owner.structure_node_id() != node.structure_node_id()
            || marked_owner.role() != node.role()
            || usage.language() != node.language()
        {
            return Err(StagingTaggedPdfManifestV2Error::MarkedContentMismatch);
        }
        let marked_content_record_fingerprint =
            marked_record_fingerprint(record, marked.fingerprint());
        let mut fact = StagingTaggedPdfVectorStructureFactV2 {
            structure_node_id: node.structure_node_id().get(),
            owner,
            kind: usage.kind(),
            role: node.role().pdf_name(),
            language: node.language().to_owned(),
            metrics_fingerprint,
            safe_vector_usage_fingerprint: usage.fingerprint(),
            marked_content_record_fingerprint,
            math_binding_fingerprint,
            canonical_jcs: String::new(),
        };
        fact.canonical_jcs = encode_structure_fact(&fact);
        vector_structures.push(fact);
    }
    vector_structures.sort_unstable_by_key(|fact| fact.structure_node_id);
    let observed_math = vector_structures
        .iter()
        .filter(|fact| fact.math_binding_fingerprint.is_some())
        .count();
    if vector_structures.len() != expected_placements
        || vector_structures
            .windows(2)
            .any(|pair| pair[0].structure_node_id >= pair[1].structure_node_id)
        || observed_math != math_vector.facts().len()
    {
        return Err(StagingTaggedPdfManifestV2Error::StructureMismatch);
    }
    let canonical_jcs = encode_manifest(
        package,
        navigation,
        profile,
        registry,
        vector_plan,
        pdf,
        safe_vector,
        math_vector,
        engine,
        &vector_structures,
    );
    Ok(StagingTaggedPdfManifestV2 {
        package_sha256: package.canonical_jcs_sha256(),
        safe_vector_manifest_fingerprint: safe_vector.fingerprint(),
        math_vector_manifest_fingerprint: math_vector.fingerprint(),
        structure_registry_fingerprint: registry.fingerprint(),
        selected_binding_fingerprint: vector_plan.selected_binding().fingerprint(),
        marked_content_fingerprint: marked.fingerprint(),
        pdf_observation_fingerprint: pdf.observation().fingerprint(),
        final_pdf_sha256: pdf.final_pdf().content_hash(),
        vector_structures,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn marked_record_fingerprint(
    record: &typaxis_display_list::MarkedContentRecordV2,
    marked: [u8; 32],
) -> [u8; 32] {
    let mut value = format!(
        "{}:{}:{}:{}:",
        record.page_index(),
        record.paint_ordinal_start(),
        record.semantic_fragment_ordinal(),
        record.selected_paint_ids().len()
    );
    for id in record.selected_paint_ids() {
        value.push_str(&format!("{id},"));
    }
    value.push_str(&hex(marked));
    sha256(value.as_bytes())
}
#[allow(clippy::too_many_arguments)]
fn encode_manifest(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    profile: &StagingAccessibilityProfileAuthorizationV2,
    registry: &StructureRegistryReceiptV2,
    plan: &VectorMarkedContentPlanV2,
    pdf: &StagingTaggedPdfV2,
    safe: &StagingSafeVectorManifestV2,
    math: &StagingMathVectorManifest,
    engine: &EngineIdentity,
    facts: &[StagingTaggedPdfVectorStructureFactV2],
) -> String {
    let mut out = String::from("{\"accessibility_profile\":");
    push_jcs_string(&mut out, STAGING_PDFUA1_PROFILE_ID_V2);
    out.push_str(",\"algorithm\":");
    push_jcs_string(&mut out, STAGING_TAGGED_PDF_MANIFEST_V2_ALGORITHM);
    out.push_str(",\"contract\":\"typaxis.contract/1.4\",\"document_language\":");
    push_jcs_string(&mut out, navigation.languages().document_language());
    out.push_str(",\"engine\":{\"name\":");
    push_jcs_string(&mut out, engine.name());
    out.push_str(",\"version\":");
    push_jcs_string(&mut out, engine.version());
    out.push_str("},\"fingerprints\":{");
    for (index, (key, value)) in [
        ("marked_content_sha256", plan.marked_content().fingerprint()),
        ("math_vector_manifest_sha256", math.fingerprint()),
        ("package_sha256", package.canonical_jcs_sha256()),
        ("pdf_observation_sha256", pdf.observation().fingerprint()),
        ("pdf_sha256", pdf.final_pdf().content_hash()),
        ("profile_sha256", profile.fingerprint()),
        ("safe_vector_manifest_sha256", safe.fingerprint()),
        (
            "selected_binding_sha256",
            plan.selected_binding().fingerprint(),
        ),
        ("structure_registry_sha256", registry.fingerprint()),
    ]
    .into_iter()
    .enumerate()
    {
        if index > 0 {
            out.push(',')
        }
        push_jcs_string(&mut out, key);
        out.push(':');
        push_hash(&mut out, value)
    }
    out.push_str("},\"vector_structures\":[");
    for (index, fact) in facts.iter().enumerate() {
        if index > 0 {
            out.push(',')
        }
        out.push_str(fact.canonical_jcs())
    }
    out.push_str("]}");
    out
}
fn encode_structure_fact(value: &StagingTaggedPdfVectorStructureFactV2) -> String {
    let mut out = String::from("{\"kind\":");
    push_jcs_string(&mut out, value.kind.as_str());
    out.push_str(",\"language\":");
    push_jcs_string(&mut out, &value.language);
    out.push_str(",\"marked_content_record_fingerprint\":");
    push_hash(&mut out, value.marked_content_record_fingerprint);
    if let Some(math) = value.math_binding_fingerprint {
        out.push_str(",\"math_binding_fingerprint\":");
        push_hash(&mut out, math)
    }
    if let Some(metrics) = value.metrics_fingerprint {
        out.push_str(",\"metrics_fingerprint\":");
        push_hash(&mut out, metrics);
    }
    out.push_str(",\"node_id\":");
    out.push_str(&value.owner.get().to_string());
    out.push_str(",\"role\":");
    push_jcs_string(&mut out, value.role);
    out.push_str(",\"safe_vector_usage_fingerprint\":");
    push_hash(&mut out, value.safe_vector_usage_fingerprint);
    out.push_str(",\"structure_node_id\":");
    out.push_str(&value.structure_node_id.to_string());
    out.push('}');
    out
}
fn push_hash(out: &mut String, value: [u8; 32]) {
    out.push('"');
    out.push_str(&hex(value));
    out.push('"')
}
fn hex(value: [u8; 32]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in value {
        out.push(char::from(H[usize::from(byte >> 4)]));
        out.push(char::from(H[usize::from(byte & 15)]))
    }
    out
}
