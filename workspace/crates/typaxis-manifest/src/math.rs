use typaxis_core::{
    push_jcs_string, sha256, FontFaceId, M4EffectiveResourceLimits, NodeId, SourceSpan, TextSpan,
};
use typaxis_display_list::StagingMathDisplay;
use typaxis_layout::{MathFlowId, MathReceiptKey, StagingMathLayout, MATH_BINDING_ALGORITHM};
use typaxis_machine_profile::StagingMathProfileReceipt;
use typaxis_math::{
    MATH_AST_FINGERPRINT_ID, MATH_COMPUTATION_ID, MATH_FORMATTER_ID, MATH_LAYOUT_WORK_ID,
    MATH_PARSER_ID, MATH_SOURCE_ID, MATH_SOURCE_LANGUAGE, MATH_SOURCE_VERSION, MATH_VECTOR_IR_ID,
};
use typaxis_pdf::StagingMathPdf;
use typaxis_resource_admission::AdmittedResourceLedger;
use typaxis_syntax::ValidatedStagingSemanticPackage;

pub const STAGING_MATH_MANIFEST_ALGORITHM: &str = "typaxis.math-manifest/1";

/// One complete source-to-PDF math occurrence. All fields are copied from
/// opaque receipts that are reverified before this projection is issued.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathManifestFact {
    occurrence: u32,
    node_id: NodeId,
    kind: &'static str,
    receipt_key: MathReceiptKey,
    source_span: SourceSpan,
    text_span: TextSpan,
    source_sha256: [u8; 32],
    speech_sha256: [u8; 32],
    actual_text_sha256: [u8; 32],
    parsed_fingerprint: [u8; 32],
    ast_fingerprint: [u8; 32],
    font_face_id: FontFaceId,
    font_sha256: [u8; 32],
    face_index: u32,
    math_table_fingerprint: [u8; 32],
    computation_fingerprint: [u8; 32],
    vector_fingerprint: [u8; 32],
    advance: i64,
    ascent: i64,
    descent: i64,
    baseline: i64,
    axis: i64,
    bbox: (i64, i64, i64, i64),
    layout_work: u64,
    parent_flow_id: u32,
    display_flow_id: Option<MathFlowId>,
    page_index: u32,
    frame_index: u32,
    fragment_ordinal: u32,
    paint_ordinal: u32,
    origin_x: i64,
    baseline_y: i64,
    selected_placement_fingerprint: [u8; 32],
    display_draw_fingerprint: [u8; 32],
    pdf_page_object: u32,
    pdf_content_object: u32,
    pdf_font_object: u32,
    pdf_marked_content_sha256: [u8; 32],
    pdf_observation_fingerprint: [u8; 32],
}

impl StagingMathManifestFact {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn kind(&self) -> &'static str {
        self.kind
    }
    pub const fn receipt_key(&self) -> MathReceiptKey {
        self.receipt_key
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn text_span(&self) -> TextSpan {
        self.text_span
    }
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub const fn speech_sha256(&self) -> [u8; 32] {
        self.speech_sha256
    }
    pub const fn actual_text_sha256(&self) -> [u8; 32] {
        self.actual_text_sha256
    }
    pub const fn ast_fingerprint(&self) -> [u8; 32] {
        self.ast_fingerprint
    }
    pub const fn vector_fingerprint(&self) -> [u8; 32] {
        self.vector_fingerprint
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn display_draw_fingerprint(&self) -> [u8; 32] {
        self.display_draw_fingerprint
    }
    pub const fn pdf_observation_fingerprint(&self) -> [u8; 32] {
        self.pdf_observation_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathManifest {
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    profile_authorization_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    layout_fingerprint: [u8; 32],
    display_fingerprint: [u8; 32],
    pdf_fingerprint: [u8; 32],
    pdf_sha256: [u8; 32],
    facts: Vec<StagingMathManifestFact>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathManifest {
    pub fn facts(&self) -> &[StagingMathManifestFact] {
        &self.facts
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
        profile: &StagingMathProfileReceipt,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        layout: &StagingMathLayout,
        display: &StagingMathDisplay,
        pdf: &StagingMathPdf,
    ) -> Result<(), StagingMathManifestError> {
        let expected = assemble(package, profile, limits, admitted, layout, display, pdf)?;
        if self != &expected {
            return Err(StagingMathManifestError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMathManifestError {
    ProfileMismatch,
    LayoutMismatch,
    DisplayMismatch,
    PdfMismatch,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingMathManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileMismatch => {
                formatter.write_str("I9190: math profile mismatch at manifest")
            }
            Self::LayoutMismatch => formatter.write_str("I9190: math layout mismatch at manifest"),
            Self::DisplayMismatch => {
                formatter.write_str("I9190: math Display mismatch at manifest")
            }
            Self::PdfMismatch => formatter.write_str("I9190: math PDF mismatch at manifest"),
            Self::ReceiptMismatch => formatter.write_str("I9190: math manifest receipt mismatch"),
            Self::AllocationFailure => {
                formatter.write_str("G6100: math manifest allocation failed")
            }
        }
    }
}

impl std::error::Error for StagingMathManifestError {}

#[allow(clippy::too_many_arguments)]
pub fn build_staging_math_manifest(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingMathProfileReceipt,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    layout: &StagingMathLayout,
    display: &StagingMathDisplay,
    pdf: &StagingMathPdf,
) -> Result<StagingMathManifest, StagingMathManifestError> {
    assemble(package, profile, limits, admitted, layout, display, pdf)
}

#[allow(clippy::too_many_arguments)]
fn assemble(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingMathProfileReceipt,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    layout: &StagingMathLayout,
    display: &StagingMathDisplay,
    pdf: &StagingMathPdf,
) -> Result<StagingMathManifest, StagingMathManifestError> {
    profile
        .authorizes(package, limits)
        .map_err(|_| StagingMathManifestError::ProfileMismatch)?;
    layout
        .verify(package, profile.authorization(), limits, admitted)
        .map_err(|_| StagingMathManifestError::LayoutMismatch)?;
    display
        .verify(package, profile.authorization(), limits, admitted, layout)
        .map_err(|_| StagingMathManifestError::DisplayMismatch)?;
    pdf.verify(package, profile.authorization(), limits, admitted, display)
        .map_err(|_| StagingMathManifestError::PdfMismatch)?;
    if package.math_nodes().len() != layout.receipts().len()
        || package.math_nodes().len() != layout.placements().len()
        || package.math_nodes().len() != display.draws().len()
        || package.math_nodes().len() != pdf.observations().len()
    {
        return Err(StagingMathManifestError::ReceiptMismatch);
    }

    let mut facts = Vec::new();
    facts
        .try_reserve_exact(package.math_nodes().len())
        .map_err(|_| StagingMathManifestError::AllocationFailure)?;
    for (occurrence, (((node, receipt), placement), (draw, observation))) in package
        .math_nodes()
        .iter()
        .zip(layout.receipts())
        .zip(layout.placements())
        .zip(display.draws().iter().zip(pdf.observations()))
        .enumerate()
    {
        if usize::try_from(draw.occurrence()) != Ok(occurrence)
            || usize::try_from(observation.occurrence()) != Ok(occurrence)
            || receipt.key() != placement.receipt_key()
            || receipt.key() != draw.receipt_key()
            || receipt.key().bytes() != observation.receipt_key()
        {
            return Err(StagingMathManifestError::ReceiptMismatch);
        }
        let dimensions = receipt.computation().dimensions();
        facts.push(StagingMathManifestFact {
            occurrence: draw.occurrence(),
            node_id: node.domain().node_id,
            kind: receipt.kind().as_str(),
            receipt_key: receipt.key(),
            source_span: node.domain().span,
            text_span: node.domain().text_span,
            source_sha256: receipt.source_sha256(),
            speech_sha256: receipt.speech_sha256(),
            actual_text_sha256: observation.actual_text_sha256(),
            parsed_fingerprint: node.parsed().fingerprint(),
            ast_fingerprint: node.parsed().ast_fingerprint(),
            font_face_id: receipt.font_face_id(),
            font_sha256: receipt.font_sha256(),
            face_index: receipt.face_index(),
            math_table_fingerprint: receipt.computation().math_table_fingerprint(),
            computation_fingerprint: receipt.computation().fingerprint(),
            vector_fingerprint: receipt.computation().vector_fingerprint(),
            advance: dimensions.advance(),
            ascent: dimensions.ascent(),
            descent: dimensions.descent(),
            baseline: dimensions.baseline(),
            axis: dimensions.axis(),
            bbox: dimensions.bbox(),
            layout_work: receipt.computation().layout_work(),
            parent_flow_id: placement.parent_flow_id().get(),
            display_flow_id: placement.display_flow_id(),
            page_index: placement.page_index(),
            frame_index: placement.frame_index(),
            fragment_ordinal: placement.fragment_ordinal(),
            paint_ordinal: placement.paint_ordinal(),
            origin_x: placement.origin_x(),
            baseline_y: placement.baseline_y(),
            selected_placement_fingerprint: placement.fingerprint(),
            display_draw_fingerprint: draw.fingerprint(),
            pdf_page_object: observation.page_object(),
            pdf_content_object: observation.content_object(),
            pdf_font_object: observation.font_object(),
            pdf_marked_content_sha256: observation.marked_content_sha256(),
            pdf_observation_fingerprint: observation.fingerprint(),
        });
    }
    let canonical_jcs = encode_manifest(
        package.semantic_fingerprint(),
        profile.fingerprint(),
        profile.authorization().profile_fingerprint(),
        limits.fingerprint(),
        admitted.fingerprint().bytes(),
        layout.fingerprint(),
        display.fingerprint(),
        pdf.fingerprint(),
        sha256(pdf.bytes()),
        &facts,
    );
    Ok(StagingMathManifest {
        package_fingerprint: package.semantic_fingerprint(),
        profile_fingerprint: profile.fingerprint(),
        profile_authorization_fingerprint: profile.authorization().profile_fingerprint(),
        limits_fingerprint: limits.fingerprint(),
        admitted_fingerprint: admitted.fingerprint().bytes(),
        layout_fingerprint: layout.fingerprint(),
        display_fingerprint: display.fingerprint(),
        pdf_fingerprint: pdf.fingerprint(),
        pdf_sha256: sha256(pdf.bytes()),
        facts,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

#[allow(clippy::too_many_arguments)]
fn encode_manifest(
    package: [u8; 32],
    profile: [u8; 32],
    profile_authorization: [u8; 32],
    limits: [u8; 32],
    admitted: [u8; 32],
    layout: [u8; 32],
    display: [u8; 32],
    pdf: [u8; 32],
    pdf_sha256: [u8; 32],
    facts: &[StagingMathManifestFact],
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MATH_MANIFEST_ALGORITHM);
    output.push_str(",\"display_fingerprint\":");
    push_hash(&mut output, display);
    output.push_str(",\"facts\":[");
    for (index, fact) in facts.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_fact(&mut output, fact);
    }
    output.push_str("],\"layout_fingerprint\":");
    push_hash(&mut output, layout);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits);
    output.push_str(",\"package_fingerprint\":");
    push_hash(&mut output, package);
    output.push_str(",\"pdf_fingerprint\":");
    push_hash(&mut output, pdf);
    output.push_str(",\"pdf_sha256\":");
    push_hash(&mut output, pdf_sha256);
    output.push_str(",\"profile_authorization_fingerprint\":");
    push_hash(&mut output, profile_authorization);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(&mut output, profile);
    output.push('}');
    output
}

fn encode_fact(output: &mut String, fact: &StagingMathManifestFact) {
    output.push_str("{\"actual_text_sha256\":");
    push_hash(output, fact.actual_text_sha256);
    output.push_str(",\"ast_fingerprint\":");
    push_hash(output, fact.ast_fingerprint);
    output.push_str(",\"ast_fingerprint_algorithm\":");
    push_jcs_string(output, MATH_AST_FINGERPRINT_ID);
    output.push_str(",\"binding_algorithm\":");
    push_jcs_string(output, MATH_BINDING_ALGORITHM);
    output.push_str(",\"computation_fingerprint\":");
    push_hash(output, fact.computation_fingerprint);
    output.push_str(",\"dimensions\":{\"advance\":");
    output.push_str(&fact.advance.to_string());
    output.push_str(",\"ascent\":");
    output.push_str(&fact.ascent.to_string());
    output.push_str(",\"axis\":");
    output.push_str(&fact.axis.to_string());
    output.push_str(",\"baseline\":");
    output.push_str(&fact.baseline.to_string());
    output.push_str(",\"bbox\":[");
    output.push_str(&fact.bbox.0.to_string());
    output.push(',');
    output.push_str(&fact.bbox.1.to_string());
    output.push(',');
    output.push_str(&fact.bbox.2.to_string());
    output.push(',');
    output.push_str(&fact.bbox.3.to_string());
    output.push(']');
    output.push_str(",\"descent\":");
    output.push_str(&fact.descent.to_string());
    output.push('}');
    output.push_str(",\"display_draw_fingerprint\":");
    push_hash(output, fact.display_draw_fingerprint);
    output.push_str(",\"face_index\":");
    output.push_str(&fact.face_index.to_string());
    output.push_str(",\"font_face_id\":");
    output.push_str(&fact.font_face_id.get().to_string());
    output.push_str(",\"font_sha256\":");
    push_hash(output, fact.font_sha256);
    output.push_str(",\"formatter\":");
    push_jcs_string(output, MATH_FORMATTER_ID);
    output.push_str(",\"kind\":");
    push_jcs_string(output, fact.kind);
    output.push_str(",\"layout_algorithm\":");
    push_jcs_string(output, MATH_COMPUTATION_ID);
    output.push_str(",\"layout_work\":");
    output.push_str(&fact.layout_work.to_string());
    output.push_str(",\"layout_work_algorithm\":");
    push_jcs_string(output, MATH_LAYOUT_WORK_ID);
    output.push_str(",\"math_table_fingerprint\":");
    push_hash(output, fact.math_table_fingerprint);
    output.push_str(",\"node_id\":");
    output.push_str(&fact.node_id.get().to_string());
    output.push_str(",\"occurrence\":");
    output.push_str(&fact.occurrence.to_string());
    output.push_str(",\"parsed_fingerprint\":");
    push_hash(output, fact.parsed_fingerprint);
    output.push_str(",\"parser\":");
    push_jcs_string(output, MATH_PARSER_ID);
    output.push_str(",\"pdf\":{\"content_object\":");
    output.push_str(&fact.pdf_content_object.to_string());
    output.push_str(",\"font_object\":");
    output.push_str(&fact.pdf_font_object.to_string());
    output.push_str(",\"marked_content_sha256\":");
    push_hash(output, fact.pdf_marked_content_sha256);
    output.push_str(",\"observation_fingerprint\":");
    push_hash(output, fact.pdf_observation_fingerprint);
    output.push_str(",\"page_object\":");
    output.push_str(&fact.pdf_page_object.to_string());
    output.push('}');
    output.push_str(",\"receipt_key\":");
    push_hash(output, fact.receipt_key.bytes());
    output.push_str(",\"selected\":{\"baseline_y\":");
    output.push_str(&fact.baseline_y.to_string());
    output.push_str(",\"display_flow_id\":");
    if let Some(value) = fact.display_flow_id {
        output.push_str(&value.get().to_string());
    } else {
        output.push_str("null");
    }
    output.push_str(",\"fragment_ordinal\":");
    output.push_str(&fact.fragment_ordinal.to_string());
    output.push_str(",\"frame_index\":");
    output.push_str(&fact.frame_index.to_string());
    output.push_str(",\"origin_x\":");
    output.push_str(&fact.origin_x.to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&fact.page_index.to_string());
    output.push_str(",\"paint_ordinal\":");
    output.push_str(&fact.paint_ordinal.to_string());
    output.push_str(",\"parent_flow_id\":");
    output.push_str(&fact.parent_flow_id.to_string());
    output.push_str(",\"placement_fingerprint\":");
    push_hash(output, fact.selected_placement_fingerprint);
    output.push('}');
    output.push_str(",\"source\":{\"language\":");
    push_jcs_string(output, MATH_SOURCE_LANGUAGE);
    output.push_str(",\"sha256\":");
    push_hash(output, fact.source_sha256);
    output.push_str(",\"source_span\":{\"end_byte\":");
    output.push_str(&fact.source_span.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&fact.source_span.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&fact.source_span.start_byte().get().to_string());
    output.push_str("},\"text_span\":{\"end_byte\":");
    output.push_str(&fact.text_span.end_byte().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&fact.text_span.start_byte().get().to_string());
    output.push_str(",\"text_id\":");
    output.push_str(&fact.text_span.text_id().get().to_string());
    output.push_str("},\"version\":");
    push_jcs_string(output, MATH_SOURCE_VERSION);
    output.push('}');
    output.push_str(",\"source_identity\":");
    push_jcs_string(output, MATH_SOURCE_ID);
    output.push_str(",\"speech_sha256\":");
    push_hash(output, fact.speech_sha256);
    output.push_str(",\"vector_algorithm\":");
    push_jcs_string(output, MATH_VECTOR_IR_ID);
    output.push_str(",\"vector_fingerprint\":");
    push_hash(output, fact.vector_fingerprint);
    output.push('}');
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
    use std::path::PathBuf;
    use typaxis_core::{
        ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions, HostAdmissionContext, HostPath,
        M4ResourceLimits, PdfStreamCompression, ResourceLimits, ValidatedResourceLimits,
        DEFAULT_ALLOWED_URI_SCHEMES,
    };
    use typaxis_display_list::{build_staging_math_display, StagingMathDisplay};
    use typaxis_layout::{layout_staging_math, StagingMathLayout};
    use typaxis_machine_profile::{
        preflight_staging_math_profile, StagingSemanticContainerSessionIdentity,
    };
    use typaxis_pdf::write_staging_math_pdf;
    use typaxis_resource_admission::{
        staging_declared_base_catalog, AdmittedResourceResolver, HostResourceAdmissionSession,
    };
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::StagingSemanticPackageParser;

    struct MathManifestFixture {
        package: ValidatedStagingSemanticPackage,
        profile: StagingMathProfileReceipt,
        limits: M4EffectiveResourceLimits,
        admitted: AdmittedResourceLedger,
        layout: StagingMathLayout,
        display: StagingMathDisplay,
        pdf: StagingMathPdf,
    }

    fn math_manifest_fixture() -> Result<MathManifestFixture, Box<dyn std::error::Error>> {
        let job = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/machine-package/staging/production-book-1/math/job");
        let package_path = job.join("document-package.json");
        let package_bytes = std::fs::read(&package_path)?;
        let base_limits = ValidatedResourceLimits::new(ResourceLimits::default())?;
        let limits =
            M4EffectiveResourceLimits::new(base_limits.clone(), M4ResourceLimits::default())?;
        let decoded = StagingSemanticDocumentPackageDecoder::new().decode(
            &package_bytes,
            &DocumentPackageDecodePolicy::new(&base_limits),
        )?;
        let package = StagingSemanticPackageParser::new().parse(decoded, &base_limits)?;
        let profile = preflight_staging_math_profile(
            &package,
            &limits,
            &StagingSemanticContainerSessionIdentity::fresh(),
        )?;
        let base = staging_declared_base_catalog(package.resources())?;
        let config = EffectiveConfig::new(
            true,
            PdfStreamCompression::None,
            vec![ConfigResourceRoot::ProjectRoot],
            DEFAULT_ALLOWED_URI_SCHEMES
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0")
                .expect("registered fixture data versions"),
            ResourceLimits::default(),
        )?;
        let context = HostAdmissionContext::new(
            HostPath::new(package_path)?,
            HostPath::new(job)?,
            None,
            Vec::new(),
        );
        let session = HostResourceAdmissionSession::new(&context, &config, &base)?;
        let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &base,
            &limits,
            profile.authorization().profile_fingerprint(),
            session.roots(),
        )?;
        for declaration in &package.resources().font_faces {
            let pending = resolver.read_font(session.open_font(declaration.font_face_id)?)?;
            resolver.parse_and_bind_declared_sfnt(pending)?;
        }
        let admitted = resolver.finish()?;
        let layout = layout_staging_math(&package, profile.authorization(), &limits, &admitted)?;
        let display = build_staging_math_display(
            &package,
            profile.authorization(),
            &limits,
            &admitted,
            &layout,
        )?;
        let pdf = write_staging_math_pdf(
            &package,
            profile.authorization(),
            &limits,
            &admitted,
            &display,
        )?;
        Ok(MathManifestFixture {
            package,
            profile,
            limits,
            admitted,
            layout,
            display,
            pdf,
        })
    }

    #[test]
    fn math_manifest_closes_source_alternative_vector_page_and_pdf_observation() {
        let fixture = math_manifest_fixture().unwrap();
        let manifest = build_staging_math_manifest(
            &fixture.package,
            &fixture.profile,
            &fixture.limits,
            &fixture.admitted,
            &fixture.layout,
            &fixture.display,
            &fixture.pdf,
        )
        .unwrap();
        assert_eq!(manifest.facts().len(), 2);
        assert_eq!(manifest.facts()[0].kind(), "inline_math");
        assert_eq!(manifest.facts()[1].kind(), "display_math");
        assert_eq!(
            manifest.facts()[0].speech_sha256(),
            manifest.facts()[0].actual_text_sha256()
        );
        assert_eq!(
            manifest.canonical_jcs(),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../samples/machine-package/staging/production-book-1/math/manifest.json"
            ))
            .trim_end()
        );

        let mut wrong_page = manifest.clone();
        wrong_page.facts[0].page_index += 1;
        assert_eq!(
            wrong_page.verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
                &fixture.layout,
                &fixture.display,
                &fixture.pdf,
            ),
            Err(StagingMathManifestError::ReceiptMismatch)
        );

        let mut wrong_source_span = manifest.clone();
        let replacement = wrong_source_span.facts[1].source_span;
        wrong_source_span.facts[0].source_span = replacement;
        assert_eq!(
            wrong_source_span.verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
                &fixture.layout,
                &fixture.display,
                &fixture.pdf,
            ),
            Err(StagingMathManifestError::ReceiptMismatch)
        );
    }
}
