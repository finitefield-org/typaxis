use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path};

#[cfg(test)]
use typaxis_core::ImageResourceId;
use typaxis_core::{
    DocumentFingerprint, EffectiveConfig, FontInstanceId, GeneratedBufferKey, GenerationKind,
    HostAdmissionContext, JsonPointer, Length, MachineInputFingerprint, MachinePdfProfileId,
    NodeId, NonNegativeLength, PortablePath, PositiveLength, ResolvedDataTables, SourceId,
    StyleFingerprint, TextSpan, Utf8ByteOffset, ValidatedResourceLimits,
};
use typaxis_diagnostics::{
    DiagnosticBuilder, DiagnosticLocation, MachineDiagnosticLender, PublicMachineError, Severity,
};
#[cfg(test)]
use typaxis_display_list::{
    StagingForcedPageBreakDisplay, StagingMachineBlockStyleDisplay, StagingMachineFigureDisplay,
    StagingMachineLinkAnnotationTamper, StagingMachineListDisplay,
};
use typaxis_display_list::{
    StagingForcedPageBreakDisplayError, StagingMachineFigureDisplayError,
    StagingMachineLinkDisplay, StagingMachineLinkDisplayError, StagingMachineListDisplayError,
    TablePaintPageBody, TableProfilePaintInput, ValidatedDisplayDocument,
};
use typaxis_document::{Block, DocumentNodeKind, Inline, ReferenceFormat};
#[cfg(test)]
use typaxis_layout::{
    consume_typed_block_style, layout_staging_forced_page_breaks, layout_staging_machine_figures,
    layout_staging_machine_lists, StagingMachineListLayoutInput, TypedBlockLayoutInput,
};
use typaxis_layout::{
    layout_table_grid, layout_table_row_bands, CanonicalFlowIrBuilder, FlowTree, LayoutEpoch,
    LayoutEpochError, MachineGlyphCoverage, MachineParagraphFlowBuilder, MachineParagraphFlowError,
    MachineStyleFontPreparationError, MachineTextSiteSource, PreparedMachineStyleFonts,
    ProductionFlowIr, ProductionFlowIrBuilder, ShapeFontSelectionReceipt, StagingFigureLayoutError,
    StagingForcedPageBreakLayoutError, StagingMachineListLayoutError, TableCellLayoutInput,
    TableRowBandLayoutReceipt, TypedStyleConsumerError, ValidatedTableGridReceipt,
};
use typaxis_linebreak::{
    break_paragraph_validated, BoundedReferenceParagraphFactory, LineLayoutContext, LineShape,
    LineShapeExhaustion, OptimalParagraphBreaker, ParagraphShapedText, ReferenceSpaceGlue,
    StagingMachineLinkClusterError, ValidatedParagraphBreak, ValidatedParagraphItemRegistry,
    ValidatedStagingMachineLinkClusters,
};
use typaxis_machine_input::MachineInputSessionIdentity;
use typaxis_machine_profile::{
    BasicDocumentFigurePreflight, BasicDocumentFigurePreflightFailure,
    BasicDocumentForcedPageBreakPreflight, BasicDocumentLinkPreflight, BasicDocumentListPreflight,
    BasicDocumentListPreflightFailure, BasicDocumentStylePreflight,
    BasicDocumentStylePreflightFailure, MachinePdfPreflight, MachinePdfPreflightFailure,
    MachinePdfPreflightReceipt, MachinePdfReceiptMismatch,
};
use typaxis_manifest::StagingTableLayoutFacts;
#[cfg(test)]
use typaxis_manifest::{
    StagingForcedPageBreakManifestFact, StagingMachineBlockStyleManifestFact,
    StagingMachineFigureManifestFact, StagingMachineLinkManifestFact,
    StagingMachineListManifestFact,
};
#[cfg(test)]
use typaxis_pagination::{
    paginate_staging_forced_page_breaks, paginate_staging_machine_figures,
    paginate_staging_machine_lists, StagingFigureCaptionBlockInput,
    StagingForcedPageBreakPaginationInput, StagingMachineFigurePaginationInput,
    StagingMachineListPageInput,
};
use typaxis_pagination::{
    paginate_staging_table, ConvergenceStatus, InitialPaginationState, PaginationError,
    PaginationResult, ReferencePaginator, SelectedTableLayoutReceipt,
    StagingForcedPageBreakPaginationError, StagingMachineFigurePaginationError,
    StagingMachineListPaginationError, StagingTablePageInput,
};
use typaxis_resources::{
    AdmittedFontInstanceTable, AdmittedResourceLedger, AdmittedResourceResolver,
    HostResourceAdmissionSession, ReferenceResourceFinalizer, ResourceAdmissionError,
    ResourceError, ResourceFinalizationInput, ResourceFinalizer,
};
use typaxis_shaping::{CanonicalItemizer, LinkedShaper, ParagraphItemizationInput, ShapingCache};
#[cfg(test)]
use typaxis_syntax::StagingStylePackageParser;
use typaxis_syntax::{
    machine_profile_boundary::StyleValue, PackageGeneratedTextError, PackageShapeTextReceipt,
    PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
    StagingStyleReceiptMismatch, ValidatedBasicDocumentPackage, ValidatedMachinePackage,
    ValidatedParsedPackage,
};
use typaxis_text::GeneratedTextStore;

#[cfg(test)]
use typaxis_pdf::{
    PdfBackend, StagingForcedPageBreakPdf, StagingMachineBlockStylePdf, StagingMachineFigurePdf,
    StagingMachineLinkPdf, StagingMachineListPdf,
};
use typaxis_pdf::{
    StagingMachineFigurePdfError, StagingMachineLinkPdfError, TablePdfClosureReceipt,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureKind {
    Input,
    Usage,
    Io,
    Internal,
    Limit,
}

impl FailureKind {
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Input => 1,
            Self::Usage => 2,
            Self::Io => 3,
            Self::Internal => 4,
            Self::Limit => 5,
        }
    }
}

#[derive(Debug)]
pub struct Failure {
    pub kind: FailureKind,
    pub message: String,
    failed_manifest_policy: FailedManifestPolicy,
}

#[allow(dead_code)] // focused MI2 slice-test fact type; no public execution entrance
#[derive(Debug)]
pub(crate) enum StagingMachineBlockStyleRunnerError {
    Decode(typaxis_document_package::DocumentPackageDecodeError),
    Syntax(typaxis_syntax::MachineParseFailure),
    Preflight(BasicDocumentStylePreflightFailure),
    ComputedStyle(StagingStyleReceiptMismatch),
    Layout(TypedStyleConsumerError),
}

#[allow(dead_code)] // focused MI2 slice-test fact type; no public execution entrance
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StagingMachineBlockStyleArtifacts {
    display_jcs: String,
    pdf_jcs: String,
    pdf_content_observation: Vec<u8>,
    manifest_jcs: String,
}

#[allow(dead_code)]
impl StagingMachineBlockStyleArtifacts {
    pub(crate) fn display_jcs(&self) -> &str {
        &self.display_jcs
    }
    pub(crate) fn pdf_jcs(&self) -> &str {
        &self.pdf_jcs
    }
    pub(crate) fn pdf_content_observation(&self) -> &[u8] {
        &self.pdf_content_observation
    }
    pub(crate) fn manifest_jcs(&self) -> &str {
        &self.manifest_jcs
    }
}

/// Test-only MI2-03 closure harness. Public builds use the normal profile-aware
/// machine pipeline below.
#[cfg(test)]
fn exercise_basic_block_style_slice(
    package_bytes: &[u8],
    source_utf8: String,
    policy: &PackageValidationPolicy<'_>,
    limits: &ValidatedResourceLimits,
    owner: NodeId,
    input: TypedBlockLayoutInput,
    diagnostics: &mut MachineDiagnosticLender<'_>,
) -> Result<StagingMachineBlockStyleArtifacts, StagingMachineBlockStyleRunnerError> {
    let decoded = typaxis_document_package::StagingStyleDocumentPackageDecoder::new()
        .decode(
            package_bytes,
            &typaxis_document_package::DocumentPackageDecodePolicy::new(limits),
        )
        .map_err(StagingMachineBlockStyleRunnerError::Decode)?;
    let package = StagingStylePackageParser::new()
        .parse(decoded, source_utf8, policy)
        .map_err(StagingMachineBlockStyleRunnerError::Syntax)?;
    let preflight = BasicDocumentStylePreflight::STAGING
        .run(&package, diagnostics)
        .map_err(StagingMachineBlockStyleRunnerError::Preflight)?;
    debug_assert!(preflight.verifies(&package));
    let computed = package
        .compute_block_style(owner, None)
        .map_err(StagingMachineBlockStyleRunnerError::ComputedStyle)?;
    let selected = consume_typed_block_style(&computed, input)
        .map_err(StagingMachineBlockStyleRunnerError::Layout)?;
    let display = StagingMachineBlockStyleDisplay::from_selected(&selected);
    let pdf = StagingMachineBlockStylePdf::from_display(&display);
    let manifest = StagingMachineBlockStyleManifestFact::from_pdf(&pdf);
    Ok(StagingMachineBlockStyleArtifacts {
        display_jcs: display.canonical_jcs().to_owned(),
        pdf_jcs: pdf.canonical_jcs().to_owned(),
        pdf_content_observation: pdf.content_stream_observation().to_vec(),
        manifest_jcs: manifest.canonical_jcs().to_owned(),
    })
}

#[allow(dead_code)] // focused MI2 slice-test fact type; no public execution entrance
#[derive(Debug)]
pub(crate) enum StagingMachineListRunnerError {
    Decode(typaxis_document_package::DocumentPackageDecodeError),
    Syntax(typaxis_syntax::MachineParseFailure),
    StylePreflight(BasicDocumentStylePreflightFailure),
    ListPreflight(BasicDocumentListPreflightFailure),
    GeneratedText(PackageGeneratedTextError),
    ResourceAdmission(ResourceAdmissionError),
    LayoutEpoch(LayoutEpochError),
    Flow(typaxis_layout::FlowRegistryError),
    Layout(StagingMachineListLayoutError),
    Pagination(StagingMachineListPaginationError),
    Display(StagingMachineListDisplayError),
}

#[allow(dead_code)]
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StagingMachineListArtifacts {
    trace_jcs: String,
    display_jcs: String,
    pdf_jcs: String,
    pdf_content_observation: Vec<u8>,
    manifest_jcs: String,
}

#[allow(dead_code)]
impl StagingMachineListArtifacts {
    pub(crate) fn trace_jcs(&self) -> &str {
        &self.trace_jcs
    }
    pub(crate) fn display_jcs(&self) -> &str {
        &self.display_jcs
    }
    pub(crate) fn pdf_jcs(&self) -> &str {
        &self.pdf_jcs
    }
    pub(crate) fn pdf_content_observation(&self) -> &[u8] {
        &self.pdf_content_observation
    }
    pub(crate) fn manifest_jcs(&self) -> &str {
        &self.manifest_jcs
    }
}

/// Test-only MI2-04 closure harness for exact list receipts and tamper cases.
/// Public builds use the normal profile-aware machine pipeline below.
#[cfg(test)]
fn exercise_basic_list_slice(
    package_bytes: &[u8],
    source_utf8: String,
    policy: &PackageValidationPolicy<'_>,
    limits: &ValidatedResourceLimits,
    layout_input: StagingMachineListLayoutInput,
    page_input: StagingMachineListPageInput,
    diagnostics: &mut MachineDiagnosticLender<'_>,
) -> Result<StagingMachineListArtifacts, StagingMachineListRunnerError> {
    let decoded = typaxis_document_package::StagingStyleDocumentPackageDecoder::new()
        .decode(
            package_bytes,
            &typaxis_document_package::DocumentPackageDecodePolicy::new(limits),
        )
        .map_err(StagingMachineListRunnerError::Decode)?;
    let package = StagingStylePackageParser::new()
        .parse(decoded, source_utf8, policy)
        .map_err(StagingMachineListRunnerError::Syntax)?;
    let style_preflight = BasicDocumentStylePreflight::STAGING
        .run(&package, diagnostics)
        .map_err(StagingMachineListRunnerError::StylePreflight)?;
    debug_assert!(style_preflight.verifies(&package));
    let list_preflight = BasicDocumentListPreflight::STAGING
        .run(&package, limits, diagnostics)
        .map_err(StagingMachineListRunnerError::ListPreflight)?;
    let generated_store = package
        .package()
        .materialize_initial_generated_text(limits)
        .map_err(StagingMachineListRunnerError::GeneratedText)?;
    let generated = package
        .package()
        .bind_generated_text(&generated_store, limits)
        .map_err(StagingMachineListRunnerError::GeneratedText)?;
    let admitted = AdmittedResourceResolver::new(&package.package().package().resources, limits)
        .and_then(AdmittedResourceResolver::finish)
        .map_err(StagingMachineListRunnerError::ResourceAdmission)?;
    let epoch = LayoutEpoch::from_validated_inputs(generated, admitted.token())
        .map_err(StagingMachineListRunnerError::LayoutEpoch)?;
    let ir = ProductionFlowIr::for_empty_paragraph_content(package.package(), epoch, limits)
        .map_err(StagingMachineListRunnerError::Flow)?;
    let layout = layout_staging_machine_lists(
        &package,
        list_preflight.layout_receipt(),
        generated,
        &ir,
        layout_input,
    )
    .map_err(StagingMachineListRunnerError::Layout)?;
    let selected = paginate_staging_machine_lists(&layout, &ir, page_input, limits)
        .map_err(StagingMachineListRunnerError::Pagination)?;
    let trace = selected.trace_facts();
    let display = StagingMachineListDisplay::from_selected(&selected)
        .map_err(StagingMachineListRunnerError::Display)?;
    let pdf = StagingMachineListPdf::from_display(&display);
    let manifest = StagingMachineListManifestFact::from_pdf(&pdf);
    Ok(StagingMachineListArtifacts {
        trace_jcs: trace.canonical_jcs().to_owned(),
        display_jcs: display.canonical_jcs().to_owned(),
        pdf_jcs: pdf.canonical_jcs().to_owned(),
        pdf_content_observation: pdf.content_stream_observation().to_vec(),
        manifest_jcs: manifest.canonical_jcs().to_owned(),
    })
}

#[allow(dead_code)] // focused MI2 slice-test fact type; no public execution entrance
#[derive(Debug)]
pub(crate) enum StagingMachinePageBreakRunnerError {
    Decode(typaxis_document_package::DocumentPackageDecodeError),
    Syntax(typaxis_syntax::MachineParseFailure),
    StylePreflight(BasicDocumentStylePreflightFailure),
    BreakPreflight(typaxis_syntax::StagingForcedPageBreakPreflightError),
    GeneratedText(PackageGeneratedTextError),
    ResourceAdmission(ResourceAdmissionError),
    LayoutEpoch(LayoutEpochError),
    Flow(typaxis_layout::FlowRegistryError),
    Layout(StagingForcedPageBreakLayoutError),
    Pagination(StagingForcedPageBreakPaginationError),
    Display(StagingForcedPageBreakDisplayError),
}

#[allow(dead_code)]
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StagingMachinePageBreakArtifacts {
    trace_jcs: String,
    display_jcs: String,
    pdf_jcs: String,
    pdf_page_tree_observation: Vec<u8>,
    manifest_jcs: String,
}

#[allow(dead_code)]
impl StagingMachinePageBreakArtifacts {
    pub(crate) fn trace_jcs(&self) -> &str {
        &self.trace_jcs
    }

    pub(crate) fn display_jcs(&self) -> &str {
        &self.display_jcs
    }

    pub(crate) fn pdf_jcs(&self) -> &str {
        &self.pdf_jcs
    }

    pub(crate) fn pdf_page_tree_observation(&self) -> &[u8] {
        &self.pdf_page_tree_observation
    }

    pub(crate) fn manifest_jcs(&self) -> &str {
        &self.manifest_jcs
    }
}

/// Test-only MI2-05 closure harness for forced-break receipt tampering.
/// Public builds use the normal profile-aware machine pipeline below.
#[cfg(test)]
fn exercise_basic_page_break_slice(
    package_bytes: &[u8],
    source_utf8: String,
    policy: &PackageValidationPolicy<'_>,
    limits: &ValidatedResourceLimits,
    painted_content_owners: Vec<NodeId>,
    diagnostics: &mut MachineDiagnosticLender<'_>,
) -> Result<StagingMachinePageBreakArtifacts, StagingMachinePageBreakRunnerError> {
    let decoded = typaxis_document_package::StagingStyleDocumentPackageDecoder::new()
        .decode(
            package_bytes,
            &typaxis_document_package::DocumentPackageDecodePolicy::new(limits),
        )
        .map_err(StagingMachinePageBreakRunnerError::Decode)?;
    let package = StagingStylePackageParser::new()
        .parse(decoded, source_utf8, policy)
        .map_err(StagingMachinePageBreakRunnerError::Syntax)?;
    let style_preflight = BasicDocumentStylePreflight::STAGING
        .run(&package, diagnostics)
        .map_err(StagingMachinePageBreakRunnerError::StylePreflight)?;
    debug_assert!(style_preflight.verifies(&package));
    let break_preflight = BasicDocumentForcedPageBreakPreflight::STAGING
        .run(&package)
        .map_err(StagingMachinePageBreakRunnerError::BreakPreflight)?;
    debug_assert!(break_preflight.verifies(&package));
    let generated_store = package
        .package()
        .materialize_initial_generated_text(limits)
        .map_err(StagingMachinePageBreakRunnerError::GeneratedText)?;
    let generated = package
        .package()
        .bind_generated_text(&generated_store, limits)
        .map_err(StagingMachinePageBreakRunnerError::GeneratedText)?;
    let admitted = AdmittedResourceResolver::new(&package.package().package().resources, limits)
        .and_then(AdmittedResourceResolver::finish)
        .map_err(StagingMachinePageBreakRunnerError::ResourceAdmission)?;
    let epoch = LayoutEpoch::from_validated_inputs(generated, admitted.token())
        .map_err(StagingMachinePageBreakRunnerError::LayoutEpoch)?;
    let ir = ProductionFlowIr::for_empty_paragraph_content(package.package(), epoch, limits)
        .map_err(StagingMachinePageBreakRunnerError::Flow)?;
    let layout = layout_staging_forced_page_breaks(&package, break_preflight.layout_receipt(), &ir)
        .map_err(StagingMachinePageBreakRunnerError::Layout)?;
    let input = StagingForcedPageBreakPaginationInput::new(&ir, painted_content_owners)
        .map_err(StagingMachinePageBreakRunnerError::Pagination)?;
    let selected = paginate_staging_forced_page_breaks(&layout, &ir, &input, limits)
        .map_err(StagingMachinePageBreakRunnerError::Pagination)?;
    let trace = selected.trace_facts();
    let display = StagingForcedPageBreakDisplay::from_selected(&selected)
        .map_err(StagingMachinePageBreakRunnerError::Display)?;
    let pdf = StagingForcedPageBreakPdf::from_display(&display);
    let manifest = StagingForcedPageBreakManifestFact::from_pdf(&pdf);
    Ok(StagingMachinePageBreakArtifacts {
        trace_jcs: trace.canonical_jcs().to_owned(),
        display_jcs: display.canonical_jcs().to_owned(),
        pdf_jcs: pdf.canonical_jcs().to_owned(),
        pdf_page_tree_observation: pdf.page_tree_observation().to_vec(),
        manifest_jcs: manifest.canonical_jcs().to_owned(),
    })
}

#[allow(dead_code)] // focused MI2 slice-test fact type; no public execution entrance
#[derive(Debug)]
pub(crate) enum StagingMachineFigureRunnerError {
    Decode(typaxis_document_package::DocumentPackageDecodeError),
    Syntax(typaxis_syntax::MachineParseFailure),
    StylePreflight(BasicDocumentStylePreflightFailure),
    FigurePreflight(BasicDocumentFigurePreflightFailure),
    GeneratedText(PackageGeneratedTextError),
    ResourceAdmission(ResourceAdmissionError),
    LayoutEpoch(LayoutEpochError),
    Flow(typaxis_layout::FlowRegistryError),
    Layout(StagingFigureLayoutError),
    Pagination(StagingMachineFigurePaginationError),
    Display(StagingMachineFigureDisplayError),
    ResourceFinalization(ResourceError),
    Pdf(typaxis_pdf::PdfError),
    PdfObservation(StagingMachineFigurePdfError),
}

#[allow(dead_code)]
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StagingMachineFigureArtifacts {
    selected_jcs: String,
    display_jcs: String,
    pdf_jcs: String,
    manifest_jcs: String,
    pdf_receipt: typaxis_pdf::VerifiedPdfBytesReceipt,
    pdf_sha256: [u8; 32],
    page_count: u32,
    object_count: u32,
    image_xobject_count: u32,
}

#[allow(dead_code)]
impl StagingMachineFigureArtifacts {
    pub(crate) fn selected_jcs(&self) -> &str {
        &self.selected_jcs
    }
    pub(crate) fn display_jcs(&self) -> &str {
        &self.display_jcs
    }
    pub(crate) fn pdf_jcs(&self) -> &str {
        &self.pdf_jcs
    }
    pub(crate) fn manifest_jcs(&self) -> &str {
        &self.manifest_jcs
    }
    pub(crate) fn pdf_bytes(&self) -> &[u8] {
        self.pdf_receipt.bytes()
    }
    pub(crate) fn write_pdf<W: std::io::Write>(
        &self,
        sink: &mut W,
    ) -> std::io::Result<typaxis_pdf::PdfStreamWriteFacts> {
        self.pdf_receipt.write_streaming(sink)
    }
    pub(crate) const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub(crate) const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub(crate) const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub(crate) const fn image_xobject_count(&self) -> u32 {
        self.image_xobject_count
    }
}

/// Test-only MI2-06 closure harness. It retains the focused injected-caption
/// measurements used by exact-boundary and tamper tests.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn exercise_basic_figure_slice(
    package_bytes: &[u8],
    source_utf8: String,
    policy: &PackageValidationPolicy<'_>,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
    initial_consumed_block_size: NonNegativeLength,
    caption_measurements: Vec<StagingFigureCaptionBlockInput>,
    draw_image_ids: Vec<ImageResourceId>,
    diagnostics: &mut MachineDiagnosticLender<'_>,
) -> Result<StagingMachineFigureArtifacts, StagingMachineFigureRunnerError> {
    let limits = config.limits();
    let decoded = typaxis_document_package::StagingStyleDocumentPackageDecoder::new()
        .decode(
            package_bytes,
            &typaxis_document_package::DocumentPackageDecodePolicy::new(limits),
        )
        .map_err(StagingMachineFigureRunnerError::Decode)?;
    let package = StagingStylePackageParser::new()
        .parse(decoded, source_utf8, policy)
        .map_err(StagingMachineFigureRunnerError::Syntax)?;
    let style_preflight = BasicDocumentStylePreflight::STAGING
        .run(&package, diagnostics)
        .map_err(StagingMachineFigureRunnerError::StylePreflight)?;
    debug_assert!(style_preflight.verifies(&package));
    let figure_preflight = BasicDocumentFigurePreflight::STAGING
        .run(&package)
        .map_err(StagingMachineFigureRunnerError::FigurePreflight)?;
    debug_assert!(figure_preflight.verifies(&package));
    let generated_store = package
        .package()
        .materialize_initial_generated_text(limits)
        .map_err(StagingMachineFigureRunnerError::GeneratedText)?;
    let generated = package
        .package()
        .bind_generated_text(&generated_store, limits)
        .map_err(StagingMachineFigureRunnerError::GeneratedText)?;

    let resource_session = HostResourceAdmissionSession::new(
        admission,
        config,
        &package.package().package().resources,
    )
    .map_err(StagingMachineFigureRunnerError::ResourceAdmission)?;
    let mut resolver = AdmittedResourceResolver::new_with_roots(
        &package.package().package().resources,
        limits,
        resource_session.roots(),
    )
    .map_err(StagingMachineFigureRunnerError::ResourceAdmission)?;
    for declaration in &package.package().package().resources.font_faces {
        let source = resource_session
            .open_font(declaration.font_face_id)
            .map_err(StagingMachineFigureRunnerError::ResourceAdmission)?;
        let pending = resolver
            .read_font(source)
            .map_err(StagingMachineFigureRunnerError::ResourceAdmission)?;
        resolver
            .parse_and_bind_sfnt(pending)
            .map_err(StagingMachineFigureRunnerError::ResourceAdmission)?;
    }
    for declaration in &package.package().package().resources.images {
        let source = resource_session
            .open_image(declaration.image_id)
            .map_err(StagingMachineFigureRunnerError::ResourceAdmission)?;
        let pending = resolver
            .read_image(source)
            .map_err(StagingMachineFigureRunnerError::ResourceAdmission)?;
        resolver
            .parse_and_bind_png(pending)
            .map_err(StagingMachineFigureRunnerError::ResourceAdmission)?;
    }
    let admitted = resolver
        .finish()
        .map_err(StagingMachineFigureRunnerError::ResourceAdmission)?;
    let epoch = LayoutEpoch::from_validated_inputs(generated, admitted.token())
        .map_err(StagingMachineFigureRunnerError::LayoutEpoch)?;
    let ir = ProductionFlowIr::for_empty_paragraph_content(package.package(), epoch, limits)
        .map_err(StagingMachineFigureRunnerError::Flow)?;
    let layout =
        layout_staging_machine_figures(&package, figure_preflight.layout_receipt(), &admitted, &ir)
            .map_err(StagingMachineFigureRunnerError::Layout)?;
    let pagination_input = StagingMachineFigurePaginationInput::new(
        &layout,
        initial_consumed_block_size,
        caption_measurements,
    )
    .map_err(StagingMachineFigureRunnerError::Pagination)?;
    let selected = paginate_staging_machine_figures(&layout, &ir, &pagination_input, limits)
        .map_err(StagingMachineFigureRunnerError::Pagination)?;
    let selected_jcs = selected.canonical_jcs().to_owned();
    let display =
        StagingMachineFigureDisplay::from_selected_with_draw_image_ids(&selected, draw_image_ids)
            .map_err(StagingMachineFigureRunnerError::Display)?;
    let display_jcs = display.canonical_jcs().to_owned();
    let resource_plans = ReferenceResourceFinalizer::new()
        .finalize(ResourceFinalizationInput {
            display: display.validated_document(),
            admitted: &admitted,
            limits,
        })
        .map_err(StagingMachineFigureRunnerError::ResourceFinalization)?;
    let (trusted_display, display_facts) = display.into_parts();
    let graph = PdfBackend::build(trusted_display, resource_plans, limits)
        .map_err(StagingMachineFigureRunnerError::Pdf)?;
    let receipt = PdfBackend::serialize(graph.clone(), config)
        .map_err(StagingMachineFigureRunnerError::Pdf)?;
    let pdf = StagingMachineFigurePdf::from_serialized(&display_facts, &graph, &receipt)
        .map_err(StagingMachineFigureRunnerError::PdfObservation)?;
    let manifest = StagingMachineFigureManifestFact::from_pdf(&pdf);
    Ok(StagingMachineFigureArtifacts {
        selected_jcs,
        display_jcs,
        pdf_jcs: pdf.canonical_jcs().to_owned(),
        manifest_jcs: manifest.canonical_jcs().to_owned(),
        pdf_receipt: receipt,
        pdf_sha256: pdf.pdf_sha256(),
        page_count: pdf.page_count(),
        object_count: pdf.object_count(),
        image_xobject_count: pdf.image_xobject_count(),
    })
}

#[allow(dead_code)] // focused MI2 slice-test fact type; no public execution entrance
#[derive(Debug)]
pub(crate) enum StagingMachineLinkRunnerError {
    Decode(typaxis_document_package::DocumentPackageDecodeError),
    Syntax(typaxis_syntax::MachineParseFailure),
    StylePreflight(BasicDocumentStylePreflightFailure),
    LinkPreflight(typaxis_syntax::StagingLinkPreflightError),
    GeneratedText(PackageGeneratedTextError),
    UnsupportedImageResource,
    ResourceAdmission(ResourceAdmissionError),
    LayoutEpoch(LayoutEpochError),
    Flow(Failure),
    Pagination(PaginationError),
    LinkClusters(StagingMachineLinkClusterError),
    Display(StagingMachineLinkDisplayError),
    ResourceFinalization(ResourceError),
    Pdf(typaxis_pdf::PdfError),
    PdfObservation(StagingMachineLinkPdfError),
}

#[allow(dead_code)]
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StagingMachineLinkArtifacts {
    cluster_jcs: String,
    display_jcs: String,
    pdf_jcs: String,
    manifest_jcs: String,
    pdf_receipt: typaxis_pdf::VerifiedPdfBytesReceipt,
    pdf_sha256: [u8; 32],
    page_count: u32,
    object_count: u32,
    destination_count: u32,
    annotation_count: u32,
}

#[allow(dead_code)]
impl StagingMachineLinkArtifacts {
    pub(crate) fn cluster_jcs(&self) -> &str {
        &self.cluster_jcs
    }
    pub(crate) fn display_jcs(&self) -> &str {
        &self.display_jcs
    }
    pub(crate) fn pdf_jcs(&self) -> &str {
        &self.pdf_jcs
    }
    pub(crate) fn manifest_jcs(&self) -> &str {
        &self.manifest_jcs
    }
    pub(crate) fn pdf_bytes(&self) -> &[u8] {
        self.pdf_receipt.bytes()
    }
    pub(crate) fn write_pdf<W: std::io::Write>(
        &self,
        sink: &mut W,
    ) -> std::io::Result<typaxis_pdf::PdfStreamWriteFacts> {
        self.pdf_receipt.write_streaming(sink)
    }
    pub(crate) const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub(crate) const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub(crate) const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub(crate) const fn destination_count(&self) -> u32 {
        self.destination_count
    }
    pub(crate) const fn annotation_count(&self) -> u32 {
        self.annotation_count
    }
}

/// Test-only MI2-07 closure harness for injected annotation tamper cases.
/// Public builds use the normal profile-aware machine pipeline below.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn exercise_basic_link_slice(
    package_bytes: &[u8],
    source_utf8: String,
    policy: &PackageValidationPolicy<'_>,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
    annotation_tamper: StagingMachineLinkAnnotationTamper,
    diagnostics: &mut MachineDiagnosticLender<'_>,
) -> Result<StagingMachineLinkArtifacts, StagingMachineLinkRunnerError> {
    let limits = config.limits();
    let decoded = typaxis_document_package::StagingStyleDocumentPackageDecoder::new()
        .decode(
            package_bytes,
            &typaxis_document_package::DocumentPackageDecodePolicy::new(limits),
        )
        .map_err(StagingMachineLinkRunnerError::Decode)?;
    let package = StagingStylePackageParser::new()
        .parse(decoded, source_utf8, policy)
        .map_err(StagingMachineLinkRunnerError::Syntax)?;
    let style_preflight = BasicDocumentStylePreflight::STAGING
        .run(&package, diagnostics)
        .map_err(StagingMachineLinkRunnerError::StylePreflight)?;
    debug_assert!(style_preflight.verifies(&package));
    let link_preflight = BasicDocumentLinkPreflight::STAGING
        .run(&package)
        .map_err(StagingMachineLinkRunnerError::LinkPreflight)?;
    debug_assert!(link_preflight.verifies(&package));

    let generated_store = package
        .package()
        .materialize_initial_generated_text(limits)
        .map_err(StagingMachineLinkRunnerError::GeneratedText)?;
    let generated = package
        .package()
        .bind_generated_text(&generated_store, limits)
        .map_err(StagingMachineLinkRunnerError::GeneratedText)?;
    let resource_session = HostResourceAdmissionSession::new(
        admission,
        config,
        &package.package().package().resources,
    )
    .map_err(StagingMachineLinkRunnerError::ResourceAdmission)?;
    let mut resolver = AdmittedResourceResolver::new_with_roots(
        &package.package().package().resources,
        limits,
        resource_session.roots(),
    )
    .map_err(StagingMachineLinkRunnerError::ResourceAdmission)?;
    for declaration in &package.package().package().resources.font_faces {
        let source = resource_session
            .open_font(declaration.font_face_id)
            .map_err(StagingMachineLinkRunnerError::ResourceAdmission)?;
        let pending = resolver
            .read_font(source)
            .map_err(StagingMachineLinkRunnerError::ResourceAdmission)?;
        resolver
            .parse_and_bind_sfnt(pending)
            .map_err(StagingMachineLinkRunnerError::ResourceAdmission)?;
    }
    if !package.package().package().resources.images.is_empty() {
        return Err(StagingMachineLinkRunnerError::UnsupportedImageResource);
    }
    let admitted = resolver
        .finish()
        .map_err(StagingMachineLinkRunnerError::ResourceAdmission)?;
    let epoch = LayoutEpoch::from_validated_inputs(generated, admitted.token())
        .map_err(StagingMachineLinkRunnerError::LayoutEpoch)?;
    let flow = build_reference_flow(package.package(), generated, &admitted, epoch, config)
        .map_err(StagingMachineLinkRunnerError::Flow)?;
    let pagination = ReferencePaginator::new()
        .paginate_with_reflow(
            package.package(),
            &flow,
            limits,
            false,
            |store, working_epoch| {
                let binding = package
                    .package()
                    .bind_generated_text(store, limits)
                    .map_err(|_| PaginationError::PackageEpochMismatch)?;
                build_reference_flow(package.package(), binding, &admitted, working_epoch, config)
                    .map_err(|_| PaginationError::FatalLayout)
            },
        )
        .map_err(StagingMachineLinkRunnerError::Pagination)?
        .into_result();
    let registry = pagination.selected_flow().paragraph_items().ok_or(
        StagingMachineLinkRunnerError::LinkClusters(
            StagingMachineLinkClusterError::MissingParagraph,
        ),
    )?;
    let clusters = ValidatedStagingMachineLinkClusters::from_registry(
        &package,
        link_preflight.cluster_receipt(),
        registry,
    )
    .map_err(StagingMachineLinkRunnerError::LinkClusters)?;
    let cluster_jcs = clusters.canonical_jcs().to_owned();
    let display = StagingMachineLinkDisplay::from_selected_with_tamper(
        &package,
        &pagination,
        pagination.selected_flow(),
        &clusters,
        config,
        annotation_tamper,
    )
    .map_err(StagingMachineLinkRunnerError::Display)?;
    let display_jcs = display.canonical_jcs().to_owned();
    let plans = ReferenceResourceFinalizer::new()
        .finalize(ResourceFinalizationInput {
            display: display.validated_document(),
            admitted: &admitted,
            limits,
        })
        .map_err(StagingMachineLinkRunnerError::ResourceFinalization)?;
    let (trusted_display, display_facts) = display.into_parts();
    let graph = PdfBackend::build(trusted_display, plans, limits)
        .map_err(StagingMachineLinkRunnerError::Pdf)?;
    let receipt =
        PdfBackend::serialize(graph.clone(), config).map_err(StagingMachineLinkRunnerError::Pdf)?;
    let pdf = StagingMachineLinkPdf::from_serialized(&display_facts, &graph, &receipt)
        .map_err(StagingMachineLinkRunnerError::PdfObservation)?;
    let manifest = StagingMachineLinkManifestFact::from_pdf(&pdf);
    Ok(StagingMachineLinkArtifacts {
        cluster_jcs,
        display_jcs,
        pdf_jcs: pdf.canonical_jcs().to_owned(),
        manifest_jcs: manifest.canonical_jcs().to_owned(),
        pdf_receipt: receipt,
        pdf_sha256: pdf.pdf_sha256(),
        page_count: pdf.page_count(),
        object_count: pdf.object_count(),
        destination_count: pdf.destination_count(),
        annotation_count: pdf.annotation_count(),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailedManifestPolicy {
    Publish,
    LeaveTargetsUntouched,
}

impl Failure {
    pub fn input(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Input,
            message: with_default_diagnostic_code(message.into(), "P1000"),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Usage,
            message: message.into(),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }
    pub fn io(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Io,
            message: message.into(),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Internal,
            message: with_default_diagnostic_code(message.into(), "I9001"),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }
    pub fn limit(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Limit,
            message: with_default_diagnostic_code(message.into(), "I9000"),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }

    fn capability_mismatch(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Internal,
            message: with_default_diagnostic_code(message.into(), "I9190"),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }

    fn unsupported_contained_open() -> Self {
        Self {
            kind: FailureKind::Io,
            message: "resource admission I/O failed: UnsupportedContainedOpen".to_owned(),
            failed_manifest_policy: FailedManifestPolicy::LeaveTargetsUntouched,
        }
    }

    pub const fn should_publish_failed_manifest(&self) -> bool {
        matches!(self.failed_manifest_policy, FailedManifestPolicy::Publish)
    }
}

fn with_default_diagnostic_code(message: String, code: &str) -> String {
    if has_diagnostic_code(&message) {
        message
    } else {
        format!("{code}: {message}")
    }
}

fn has_diagnostic_code(message: &str) -> bool {
    let bytes = message.as_bytes();
    bytes.get(5) == Some(&b':')
        && matches!(
            bytes.get(0..2),
            Some(b"P1")
                | Some(b"T2")
                | Some(b"S3")
                | Some(b"F4")
                | Some(b"L5")
                | Some(b"G6")
                | Some(b"R7")
                | Some(b"D8")
                | Some(b"I9")
        )
        && bytes
            .get(2..5)
            .is_some_and(|digits| digits.iter().all(u8::is_ascii_digit))
}

pub fn load_package(
    input: &Path,
    config: &EffectiveConfig,
) -> Result<Box<ValidatedParsedPackage>, Failure> {
    let mut file = open_entry_source(input)?;
    #[cfg(all(
        unix,
        not(any(
            target_os = "espidf",
            target_os = "horizon",
            target_os = "solaris",
            target_os = "vita",
            target_os = "wasi"
        ))
    ))]
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockShared).map_err(
        |error| {
            Failure::io(format!(
                "cannot lock input `{}` for a stable read: {error}",
                input.display()
            ))
        },
    )?;
    let snapshot = InputFileSnapshot::from_file(&file, input)?;
    if !snapshot.regular {
        return Err(Failure::io(format!(
            "input `{}` is not a regular file",
            input.display()
        )));
    }
    let byte_length = snapshot.length;
    let limits = config.limits().get();
    if byte_length > limits.max_input_bytes || byte_length > u64::from(limits.max_source_bytes) {
        return Err(Failure::limit(format!(
            "input byte length {byte_length} exceeds the configured source/input limit"
        )));
    }
    let allocation = usize::try_from(byte_length)
        .map_err(|_| Failure::limit("input byte length exceeds this platform's address space"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(allocation)
        .map_err(|_| Failure::limit("cannot reserve the bounded input buffer"))?;
    bytes.resize(allocation, 0);
    file.read_exact(&mut bytes).map_err(|error| {
        Failure::io(format!("cannot read input `{}`: {error}", input.display()))
    })?;
    let final_snapshot = InputFileSnapshot::from_file(&file, input)?;
    let final_length = final_snapshot.length;
    if final_length > limits.max_input_bytes || final_length > u64::from(limits.max_source_bytes) {
        return Err(Failure::limit(format!(
            "input byte length {final_length} exceeds the configured source/input limit"
        )));
    }
    if final_snapshot != snapshot {
        return Err(Failure::io(format!(
            "input `{}` changed while it was being read",
            input.display()
        )));
    }
    let text = String::from_utf8(bytes).map_err(|error| {
        Failure::input(format!(
            "P1000: input is not UTF-8 (invalid byte at offset {})",
            error.utf8_error().valid_up_to()
        ))
    })?;
    preflight_reference_limits(&text, config.limits())?;
    let uri = logical_entry_uri(input);
    let source = SourceFile {
        source_id: SourceId::new(0),
        uri,
        text,
    };
    let policy = PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes())
        .map_err(|error| Failure::internal(format!("invalid URI policy: {error:?}")))?;
    match ReferenceParser::new().parse(&source, &policy) {
        ParseOutcome::Parsed {
            package,
            diagnostics,
        } => {
            for diagnostic in diagnostics {
                let diagnostic = diagnostic.as_diagnostic();
                crate::write_stderr_line(&format!(
                    "{}: {}",
                    diagnostic.code().as_str(),
                    diagnostic.message()
                ))?;
            }
            Ok(package)
        }
        ParseOutcome::Failed { failure } => {
            let message = failure
                .diagnostics()
                .iter()
                .map(|diagnostic| {
                    format!("{}: {}", diagnostic.code().as_str(), diagnostic.message())
                })
                .collect::<Vec<_>>()
                .join("\n");
            Err(Failure::input(message))
        }
    }
}

fn open_entry_source(input: &Path) -> Result<File, Failure> {
    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags};

        let descriptor = rustix::fs::open(
            input,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|error| {
            Failure::io(format!("cannot open input `{}`: {error}", input.display()))
        })?;
        Ok(descriptor.into())
    }
    #[cfg(not(unix))]
    {
        let metadata = std::fs::metadata(input).map_err(|error| {
            Failure::io(format!(
                "cannot inspect input `{}` before opening it: {error}",
                input.display()
            ))
        })?;
        if !metadata.is_file() {
            return Err(Failure::io(format!(
                "input `{}` is not a regular file",
                input.display()
            )));
        }
        File::open(input).map_err(|error| {
            Failure::io(format!("cannot open input `{}`: {error}", input.display()))
        })
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InputFileSnapshot {
    device: u128,
    inode: u128,
    length: u64,
    modified_seconds: i128,
    modified_nanoseconds: u128,
    changed_seconds: i128,
    changed_nanoseconds: u128,
    regular: bool,
}

#[cfg(any(target_os = "android", target_os = "linux"))]
impl InputFileSnapshot {
    fn from_file(file: &File, input: &Path) -> Result<Self, Failure> {
        let stat = rustix::fs::fstat(file).map_err(|error| {
            Failure::io(format!(
                "cannot inspect input `{}`: {error}",
                input.display()
            ))
        })?;
        Ok(Self {
            device: u128::from(stat.st_dev),
            inode: u128::from(stat.st_ino),
            length: u64::try_from(stat.st_size).map_err(|_| {
                Failure::limit(format!(
                    "input `{}` has an unsupported byte length",
                    input.display()
                ))
            })?,
            modified_seconds: i128::from(stat.st_mtime),
            modified_nanoseconds: u128::from(stat.st_mtime_nsec),
            changed_seconds: i128::from(stat.st_ctime),
            changed_nanoseconds: u128::from(stat.st_ctime_nsec),
            regular: rustix::fs::FileType::from_raw_mode(stat.st_mode)
                == rustix::fs::FileType::RegularFile,
        })
    }
}

#[cfg(all(unix, not(any(target_os = "android", target_os = "linux"))))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InputFileSnapshot {
    device: u64,
    inode: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
    regular: bool,
}

#[cfg(all(unix, not(any(target_os = "android", target_os = "linux"))))]
impl InputFileSnapshot {
    fn from_file(file: &File, input: &Path) -> Result<Self, Failure> {
        use std::os::unix::fs::MetadataExt;

        let metadata = file.metadata().map_err(|error| {
            Failure::io(format!(
                "cannot inspect input `{}`: {error}",
                input.display()
            ))
        })?;
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
            regular: metadata.is_file(),
        })
    }
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InputFileSnapshot {
    length: u64,
    modified: std::time::SystemTime,
    regular: bool,
}

#[cfg(not(unix))]
impl InputFileSnapshot {
    fn from_file(file: &File, input: &Path) -> Result<Self, Failure> {
        let metadata = file.metadata().map_err(|error| {
            Failure::io(format!(
                "cannot inspect input `{}`: {error}",
                input.display()
            ))
        })?;
        let modified = metadata.modified().map_err(|error| {
            Failure::io(format!(
                "cannot read input `{}` modification time for a stable read: {error}",
                input.display()
            ))
        })?;
        Ok(Self {
            length: metadata.len(),
            modified,
            regular: metadata.is_file(),
        })
    }
}

fn logical_entry_uri(input: &Path) -> PortablePath {
    if !input.is_absolute() {
        let components: Vec<_> = input
            .components()
            .filter_map(|component| match component {
                Component::Normal(value) => value.to_str(),
                _ => None,
            })
            .collect();
        if components.len()
            == input
                .components()
                .filter(|c| !matches!(c, Component::CurDir))
                .count()
        {
            let candidate = components.join("/");
            if let Ok(path) = PortablePath::new(candidate) {
                return path;
            }
        }
    }
    if let Some(name) = input.file_name().and_then(|value| value.to_str()) {
        if let Ok(path) = PortablePath::new(name) {
            return path;
        }
    }
    PortablePath::new("input.tsf").expect("static portable path is valid")
}

fn preflight_reference_limits(text: &str, limits: &ValidatedResourceLimits) -> Result<(), Failure> {
    let limits = limits.get();
    let mut ast_nodes = 1u64;
    let mut text_bytes = 0u64;
    let mut deepest = 1u32;
    for raw_line in text.split_inclusive('\n') {
        let without_lf = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let line = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        if line.is_empty() {
            continue;
        }
        ast_nodes = ast_nodes
            .checked_add(1)
            .ok_or_else(|| Failure::limit("AST node count overflow"))?;
        deepest = deepest.max(2);
        if let Some(value) = line.strip_prefix("text:") {
            let length = u64::try_from(value.len())
                .map_err(|_| Failure::limit("text buffer byte length overflow"))?;
            if length > u64::from(limits.max_text_buffer_bytes) {
                return Err(Failure::limit(format!(
                    "text buffer byte length {length} exceeds max_text_buffer_bytes"
                )));
            }
            text_bytes = text_bytes
                .checked_add(length)
                .ok_or_else(|| Failure::limit("text byte count overflow"))?;
            ast_nodes = ast_nodes
                .checked_add(1)
                .ok_or_else(|| Failure::limit("AST node count overflow"))?;
            deepest = 3;
        } else if line.starts_with("anchor:") {
            ast_nodes = ast_nodes
                .checked_add(1)
                .ok_or_else(|| Failure::limit("AST node count overflow"))?;
            deepest = 3;
        }
    }
    if ast_nodes > limits.max_ast_nodes {
        return Err(Failure::limit(format!(
            "AST node count {ast_nodes} exceeds max_ast_nodes"
        )));
    }
    if deepest > limits.max_ast_nesting_depth {
        return Err(Failure::limit(format!(
            "AST nesting depth {deepest} exceeds max_ast_nesting_depth"
        )));
    }
    if text_bytes > limits.max_text_bytes {
        return Err(Failure::limit(format!(
            "text byte count {text_bytes} exceeds max_text_bytes"
        )));
    }
    Ok(())
}

/// Shared `check-package` preparation result. It is issued only after a
/// profile receipt has been revalidated, every declared resource has produced
/// a complete ledger, and every text-producing site has a computed family and
/// dense font-instance binding. Glyph coverage is intentionally deferred to
/// build-time shaping.
pub struct MachinePackagePreparation {
    profile: MachinePdfProfileId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    package_input: MachineInputFingerprint,
    session: MachineInputSessionIdentity,
    admitted: AdmittedResourceLedger,
    generated: GeneratedTextStore,
    style_fonts: PreparedMachineStyleFonts,
}

#[allow(dead_code)] // Some receipt facts remain private-runner/test observability until MI1-17.
impl MachinePackagePreparation {
    pub const fn profile(&self) -> MachinePdfProfileId {
        self.profile
    }

    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }

    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }

    pub const fn machine_input_fingerprint(&self) -> MachineInputFingerprint {
        self.package_input
    }

    pub const fn admitted(&self) -> &AdmittedResourceLedger {
        &self.admitted
    }

    pub const fn generated(&self) -> &GeneratedTextStore {
        &self.generated
    }

    pub const fn style_fonts(&self) -> &PreparedMachineStyleFonts {
        &self.style_fonts
    }

    pub const fn glyph_coverage(&self) -> MachineGlyphCoverage {
        self.style_fonts.glyph_coverage()
    }

    fn verify(
        &self,
        package: &ValidatedMachinePackage,
        receipt: &MachinePdfPreflightReceipt,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), Failure> {
        receipt
            .verify(self.profile, package)
            .map_err(map_machine_receipt_mismatch)?;
        let identity = package.package().epoch_identity();
        if self.document != identity.document()
            || self.style != identity.style()
            || self.package_input != package.provenance().fingerprint()
            || self.session != *package.provenance().session_identity()
            || !self
                .admitted
                .matches_declarations(&package.package().package().resources)
        {
            return Err(Failure::capability_mismatch(
                "machine preparation does not match the validated package",
            ));
        }
        let generated = package
            .package()
            .bind_generated_text(&self.generated, limits)
            .map_err(|error| {
                Failure::capability_mismatch(format!(
                    "machine generated-text preparation mismatch: {error:?}"
                ))
            })?;
        let epoch = LayoutEpoch::from_validated_inputs(generated, self.admitted.token()).map_err(
            |error| {
                Failure::capability_mismatch(format!(
                    "machine layout-epoch preparation mismatch: {error:?}"
                ))
            },
        )?;
        if self.style_fonts.epoch() != epoch
            || !self.style_fonts.matches_package_epoch(package, epoch)
        {
            return Err(Failure::capability_mismatch(
                "machine style/font preparation does not match the package epoch",
            ));
        }
        Ok(())
    }
}

/// Successful `check-package` boundary. Destructuring is required before the
/// build-only layout entry can consume both the non-cloneable profile receipt
/// and the complete preparation.
pub struct CheckedMachinePackage {
    receipt: MachinePdfPreflightReceipt,
    preparation: MachinePackagePreparation,
}

impl CheckedMachinePackage {
    #[allow(dead_code)] // Kept for the shared check boundary's typed observability.
    pub const fn receipt(&self) -> &MachinePdfPreflightReceipt {
        &self.receipt
    }

    pub const fn preparation(&self) -> &MachinePackagePreparation {
        &self.preparation
    }

    pub fn into_parts(self) -> (MachinePdfPreflightReceipt, MachinePackagePreparation) {
        (self.receipt, self.preparation)
    }
}

/// Capability-to-style/font preparation boundary shared by the future build
/// and check commands. The resource closure is unreachable until the closed
/// descriptor has issued a receipt.
#[allow(dead_code)] // Compatibility entry retained beside the phase-split CLI owner.
pub fn check_machine_package_preparation(
    package: &ValidatedMachinePackage,
    diagnostics: &mut MachineDiagnosticLender<'_>,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<CheckedMachinePackage, Failure> {
    let candidates = register_machine_resource_candidates(package, config, admission)?;
    let capability = preflight_machine_package(
        package,
        MachinePdfProfileId::PARAGRAPH_1,
        config.limits(),
        diagnostics,
        candidates,
    )?;
    complete_machine_package_preparation(package, capability, config, admission)
        .map_err(MachinePreparationFailure::into_failure)
}

/// Candidate registration is a distinct observable phase: all safe declared
/// resource paths are in the command read ledger, but no resource bytes have
/// been opened and no capability receipt has been issued yet.
pub(crate) struct RegisteredMachineResourceCandidates(Option<HostResourceAdmissionSession>);

/// Successful capability gate retaining the unopened candidate session for
/// the following resource phase.
pub(crate) struct MachineCapabilityPreparation {
    receipt: MachinePdfPreflightReceipt,
    candidates: RegisteredMachineResourceCandidates,
}

impl MachineCapabilityPreparation {
    pub(crate) const fn receipt(&self) -> &MachinePdfPreflightReceipt {
        &self.receipt
    }
}

/// Resource facts established before a preparation failure. A complete
/// ledger is kept distinct from a partial owner-issued snapshot so the
/// manifest projection can advance to the exact highest trusted stage.
pub(crate) enum MachineResourcePreparationProgress {
    Partial(typaxis_resources::ResourceAdmissionProgressToken),
    Complete(AdmittedResourceLedger),
}

pub(crate) struct MachinePreparationFailure {
    failure: Failure,
    resource_progress: Option<MachineResourcePreparationProgress>,
}

impl MachinePreparationFailure {
    pub(crate) const fn resource_progress(&self) -> Option<&MachineResourcePreparationProgress> {
        self.resource_progress.as_ref()
    }

    pub(crate) fn into_failure(self) -> Failure {
        self.failure
    }

    fn before_resources(failure: Failure) -> Self {
        Self {
            failure,
            resource_progress: None,
        }
    }

    fn partial(
        failure: Failure,
        progress: typaxis_resources::ResourceAdmissionProgressToken,
    ) -> Self {
        Self {
            failure,
            resource_progress: Some(MachineResourcePreparationProgress::Partial(progress)),
        }
    }

    fn complete(failure: Failure, admitted: AdmittedResourceLedger) -> Self {
        Self {
            failure,
            resource_progress: Some(MachineResourcePreparationProgress::Complete(admitted)),
        }
    }
}

/// Apply the closed profile gate only after candidate registration. The
/// returned value still owns the unopened resource candidate session.
pub(crate) fn preflight_machine_package(
    package: &ValidatedMachinePackage,
    profile: MachinePdfProfileId,
    limits: &ValidatedResourceLimits,
    diagnostics: &mut MachineDiagnosticLender<'_>,
    candidates: RegisteredMachineResourceCandidates,
) -> Result<MachineCapabilityPreparation, Failure> {
    if matches!(
        profile,
        MachinePdfProfileId::BasicDocument1 | MachinePdfProfileId::Table1
    ) && package.contract() != typaxis_core::DocumentPackageContractId::V1_2
    {
        emit_current_contract_diagnostic(package, profile, diagnostics)?;
        return Err(Failure::input(format!(
            "P1103: {} requires raw DocumentPackage contract 1.2",
            profile.as_str()
        )));
    }
    let preflight = match profile {
        MachinePdfProfileId::BasicDocument1 => MachinePdfPreflight::BASIC_DOCUMENT_1,
        MachinePdfProfileId::Paragraph1 => MachinePdfPreflight::PARAGRAPH_1,
        MachinePdfProfileId::Table1 => MachinePdfPreflight::TABLE_1,
    };
    let receipt = preflight
        .run(package, diagnostics)
        .map_err(map_machine_preflight_failure)?;
    if matches!(
        profile,
        MachinePdfProfileId::BasicDocument1 | MachinePdfProfileId::Table1
    ) {
        let basic = package.basic_document_view().ok_or_else(|| {
            Failure::capability_mismatch("basic-document syntax view was not issued")
        })?;
        preflight_basic_document_slices(&basic, profile, limits, diagnostics)?;
    }
    Ok(MachineCapabilityPreparation {
        receipt,
        candidates,
    })
}

fn emit_current_contract_diagnostic(
    package: &ValidatedMachinePackage,
    profile: MachinePdfProfileId,
    diagnostics: &mut MachineDiagnosticLender<'_>,
) -> Result<(), Failure> {
    let uri = package
        .provenance()
        .progress()
        .package()
        .expect("validated machine package retains PACKAGE facts")
        .uri()
        .clone();
    let error = PublicMachineError::PackageContract;
    let diagnostic = DiagnosticBuilder::located(
        error.code(),
        Severity::Error,
        format!(
            "{} requires raw DocumentPackage contract 1.2",
            profile.as_str()
        ),
        DiagnosticLocation::package_json(uri, JsonPointer::root().child("contract"), None),
    )
    .map_err(|_| Failure::internal("basic-document contract diagnostic was not canonical"))?
    .build();
    let _ = diagnostics
        .emit_error_with(|| diagnostic)
        .map_err(|error| Failure::internal(format!("diagnostic budget failed: {error:?}")))?;
    Ok(())
}

fn preflight_basic_document_slices(
    package: &ValidatedBasicDocumentPackage,
    profile: MachinePdfProfileId,
    limits: &ValidatedResourceLimits,
    diagnostics: &mut MachineDiagnosticLender<'_>,
) -> Result<(), Failure> {
    let style_preflight = if profile == MachinePdfProfileId::TABLE_1 {
        BasicDocumentStylePreflight::TABLE_1
    } else {
        BasicDocumentStylePreflight::STAGING
    };
    style_preflight.run(package, diagnostics).map_err(|error| {
        Failure::input(format!("L5101: basic style preflight failed: {error:?}"))
    })?;
    BasicDocumentListPreflight::STAGING
        .run(package, limits, diagnostics)
        .map_err(|error| {
            Failure::input(format!("L5100: basic list preflight failed: {error:?}"))
        })?;
    BasicDocumentForcedPageBreakPreflight::STAGING
        .run(package)
        .map_err(|error| {
            Failure::input(format!("L5100: forced-break preflight failed: {error:?}"))
        })?;
    BasicDocumentFigurePreflight::STAGING
        .run(package)
        .map_err(|error| {
            Failure::input(format!("L5100: PNG figure preflight failed: {error:?}"))
        })?;
    BasicDocumentLinkPreflight::STAGING
        .run(package)
        .map_err(|error| Failure::input(format!("L5100: link preflight failed: {error:?}")))?;
    Ok(())
}

/// Complete resource admission plus style/font-family coverage. Glyph
/// coverage, layout, pagination, and PDF construction remain outside this
/// boundary, which is the successful `check-package` endpoint.
pub(crate) fn complete_machine_package_preparation(
    package: &ValidatedMachinePackage,
    capability: MachineCapabilityPreparation,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<CheckedMachinePackage, MachinePreparationFailure> {
    let MachineCapabilityPreparation {
        receipt,
        candidates,
    } = capability;
    let preparation = prepare_machine_package_with_registered_progress(
        package, &receipt, config, admission, candidates,
    )?;
    Ok(CheckedMachinePackage {
        receipt,
        preparation,
    })
}

#[allow(dead_code)] // production seam for side-effect ordering tests
fn check_machine_package_preparation_with(
    package: &ValidatedMachinePackage,
    diagnostics: &mut MachineDiagnosticLender<'_>,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
    admit: impl FnOnce(
        &ValidatedParsedPackage,
        &EffectiveConfig,
        &HostAdmissionContext,
    ) -> Result<AdmittedResourceLedger, Failure>,
) -> Result<CheckedMachinePackage, Failure> {
    // Candidate parent+leaf identities are deliberately registered before the
    // capability gate. Unsupported resources/content remain unopened, but a
    // failure sidecar can no longer overwrite an existing or missing input
    // candidate through `--force`.
    let _candidates = register_machine_resource_candidates(package, config, admission)?;
    let receipt = MachinePdfPreflight::PARAGRAPH_1
        .run(package, diagnostics)
        .map_err(map_machine_preflight_failure)?;
    let preparation = prepare_machine_package_with(package, &receipt, config, admission, admit)?;
    Ok(CheckedMachinePackage {
        receipt,
        preparation,
    })
}

/// The shared internal boundary used by future `build-package` and
/// `check-package` orchestration. Receipt verification happens before the
/// resource-admission closure can be invoked.
#[allow(dead_code)] // Public lower boundary retained for receipt-gating tests.
pub fn prepare_machine_package(
    package: &ValidatedMachinePackage,
    receipt: &MachinePdfPreflightReceipt,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<MachinePackagePreparation, Failure> {
    let candidates = register_machine_resource_candidates(package, config, admission)?;
    prepare_machine_package_with_registered_progress(
        package, receipt, config, admission, candidates,
    )
    .map_err(MachinePreparationFailure::into_failure)
}

pub(crate) fn register_machine_resource_candidates(
    package: &ValidatedMachinePackage,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<RegisteredMachineResourceCandidates, Failure> {
    let declarations = &package.package().package().resources;
    if declarations.font_faces.is_empty() && declarations.images.is_empty() {
        return Ok(RegisteredMachineResourceCandidates(None));
    }
    HostResourceAdmissionSession::new_with_read_ledger(
        admission,
        config,
        declarations,
        package.provenance().admission().read_ledger(),
    )
    .map(Some)
    .map(RegisteredMachineResourceCandidates)
    .map_err(map_admission_error)
}

fn prepare_machine_package_with_registered_progress(
    package: &ValidatedMachinePackage,
    receipt: &MachinePdfPreflightReceipt,
    config: &EffectiveConfig,
    _admission: &HostAdmissionContext,
    candidates: RegisteredMachineResourceCandidates,
) -> Result<MachinePackagePreparation, MachinePreparationFailure> {
    receipt
        .verify(receipt.profile(), package)
        .map_err(map_machine_receipt_mismatch)
        .map_err(MachinePreparationFailure::before_resources)?;
    let parsed = package.package();
    let generated = parsed
        .materialize_initial_generated_text(config.limits())
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "machine generated-text setup contradicted preflight: {error:?}"
            ))
        })
        .map_err(MachinePreparationFailure::before_resources)?;
    let admitted = admit_registered_resources_with_progress(parsed, config, candidates.0).map_err(
        |failure| match failure.progress {
            Some(progress) => MachinePreparationFailure::partial(failure.failure, progress),
            None => MachinePreparationFailure::before_resources(failure.failure),
        },
    )?;
    let generated_binding = match parsed.bind_generated_text(&generated, config.limits()) {
        Ok(binding) => binding,
        Err(error) => {
            return Err(MachinePreparationFailure::complete(
                Failure::capability_mismatch(format!(
                    "machine generated-text binding contradicted preflight: {error:?}"
                )),
                admitted,
            ));
        }
    };
    let style_fonts =
        match PreparedMachineStyleFonts::prepare(package, generated_binding, admitted.token()) {
            Ok(prepared) => prepared,
            Err(error) => {
                return Err(MachinePreparationFailure::complete(
                    map_machine_style_font_error(error),
                    admitted,
                ));
            }
        };
    let identity = parsed.epoch_identity();
    let preparation = MachinePackagePreparation {
        profile: receipt.profile(),
        document: identity.document(),
        style: identity.style(),
        package_input: package.provenance().fingerprint(),
        session: package.provenance().session_identity().clone(),
        admitted,
        generated,
        style_fonts,
    };
    if let Err(failure) = preparation.verify(package, receipt, config.limits()) {
        let MachinePackagePreparation { admitted, .. } = preparation;
        return Err(MachinePreparationFailure::complete(failure, admitted));
    }
    Ok(preparation)
}

#[allow(dead_code)] // production seam for side-effect ordering tests
fn prepare_machine_package_with(
    package: &ValidatedMachinePackage,
    receipt: &MachinePdfPreflightReceipt,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
    admit: impl FnOnce(
        &ValidatedParsedPackage,
        &EffectiveConfig,
        &HostAdmissionContext,
    ) -> Result<AdmittedResourceLedger, Failure>,
) -> Result<MachinePackagePreparation, Failure> {
    receipt
        .verify(receipt.profile(), package)
        .map_err(map_machine_receipt_mismatch)?;
    let parsed = package.package();
    let generated = parsed
        .materialize_initial_generated_text(config.limits())
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "machine generated-text setup contradicted preflight: {error:?}"
            ))
        })?;
    // This is the first operation permitted to open declared resource bytes.
    let admitted = admit(parsed, config, admission)?;
    let generated_binding = parsed
        .bind_generated_text(&generated, config.limits())
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "machine generated-text binding contradicted preflight: {error:?}"
            ))
        })?;
    let style_fonts =
        PreparedMachineStyleFonts::prepare(package, generated_binding, admitted.token())
            .map_err(map_machine_style_font_error)?;
    let identity = parsed.epoch_identity();
    let preparation = MachinePackagePreparation {
        profile: receipt.profile(),
        document: identity.document(),
        style: identity.style(),
        package_input: package.provenance().fingerprint(),
        session: package.provenance().session_identity().clone(),
        admitted,
        generated,
        style_fonts,
    };
    preparation.verify(package, receipt, config.limits())?;
    Ok(preparation)
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_receipt_mismatch(error: MachinePdfReceiptMismatch) -> Failure {
    Failure::capability_mismatch(format!("machine capability receipt mismatch: {error}"))
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_preflight_failure(error: MachinePdfPreflightFailure) -> Failure {
    match error {
        MachinePdfPreflightFailure::Unsupported {
            violation_count,
            primary_code,
        } => Failure::input(format!(
            "{primary_code}: machine PDF profile rejected {violation_count} capability violation(s)"
        )),
        MachinePdfPreflightFailure::WrongDiagnosticPhase
        | MachinePdfPreflightFailure::DiagnosticBudget(_) => Failure::internal(format!(
            "machine capability preflight orchestration failed: {error:?}"
        )),
    }
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_style_font_error(error: MachineStyleFontPreparationError) -> Failure {
    match error {
        MachineStyleFontPreparationError::Style(_)
        | MachineStyleFontPreparationError::LayoutStyle(_) => Failure::input(format!(
            "L5101: machine style/font coverage rejected the input: {error:?}"
        )),
        MachineStyleFontPreparationError::Resource(
            typaxis_resources::ResourceAdmissionError::ResourceLimit,
        )
        | MachineStyleFontPreparationError::ResourceLimit => {
            Failure::limit("machine style/font coverage exceeded a resource limit")
        }
        MachineStyleFontPreparationError::Resource(error) => map_admission_error(error),
        _ => Failure::capability_mismatch(format!(
            "machine style/font coverage contradicted preflight: {error:?}"
        )),
    }
}

#[derive(Debug)]
pub struct ReferenceLayout {
    pub admitted: AdmittedResourceLedger,
    pub flow: FlowTree,
    pub initial: InitialPaginationState,
    pub pagination: PaginationResult,
}

pub fn layout_reference(
    package: &ValidatedParsedPackage,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<ReferenceLayout, Failure> {
    if !package.package().document.footnotes.is_empty() {
        return Err(Failure::input(
            "L5000: the reference layout backend does not yet accept footnotes",
        ));
    }
    let limits = config.limits();
    let generated = package
        .materialize_initial_generated_text(limits)
        .map_err(|error| Failure::internal(format!("generated text setup failed: {error:?}")))?;
    let admitted = admit_resources(package, config, admission)?;
    let generated_binding = package
        .bind_generated_text(&generated, limits)
        .map_err(|error| Failure::internal(format!("generated text binding failed: {error:?}")))?;
    let epoch = LayoutEpoch::from_validated_inputs(generated_binding, admitted.token())
        .map_err(|error| Failure::internal(format!("layout epoch setup failed: {error:?}")))?;
    let flow = build_reference_flow(package, generated_binding, &admitted, epoch, config)?;
    let initial =
        InitialPaginationState::new(&flow, package, limits).map_err(map_pagination_error)?;
    // Always materialize a fallback in the reference owner. The caller emits
    // trace/failed-manifest facts before applying strict rejection.
    let outcome = ReferencePaginator::new()
        .paginate_with_reflow(package, &flow, limits, false, |store, working_epoch| {
            let binding = package
                .bind_generated_text(store, limits)
                .map_err(|_| PaginationError::PackageEpochMismatch)?;
            build_reference_flow(package, binding, &admitted, working_epoch, config)
                .map_err(|_| PaginationError::FatalLayout)
        })
        .map_err(map_pagination_error)?;
    if !config.strict() {
        for advisory in outcome.diagnostics() {
            let diagnostic = advisory.as_diagnostic();
            crate::write_stderr_line(&format!(
                "{}: {}",
                diagnostic.code().as_str(),
                diagnostic.message()
            ))?;
        }
    }
    let pagination = outcome.into_result();
    Ok(ReferenceLayout {
        admitted,
        flow,
        initial,
        pagination,
    })
}

/// Machine paragraph layout owns the already-complete preparation; no layout
/// or finalization consumer can observe a partial resource resolver.
pub struct MachineTableLayoutState {
    paragraph_items: ValidatedParagraphItemRegistry,
    grid: ValidatedTableGridReceipt,
    row_bands: TableRowBandLayoutReceipt,
    selected: SelectedTableLayoutReceipt,
    page_bodies: Vec<TablePaintPageBody>,
}

impl MachineTableLayoutState {
    pub const fn paragraph_items(&self) -> &ValidatedParagraphItemRegistry {
        &self.paragraph_items
    }
    pub const fn grid(&self) -> &ValidatedTableGridReceipt {
        &self.grid
    }
    pub const fn row_bands(&self) -> &TableRowBandLayoutReceipt {
        &self.row_bands
    }
    pub const fn selected(&self) -> &SelectedTableLayoutReceipt {
        &self.selected
    }
    pub fn page_bodies(&self) -> &[TablePaintPageBody] {
        &self.page_bodies
    }
}

#[allow(dead_code)] // wired to public command dispatch by MI1-15
pub struct MachineParagraphLayout {
    preparation: MachinePackagePreparation,
    flow: FlowTree,
    initial: InitialPaginationState,
    pagination: PaginationResult,
    flow_registry_sha256: Option<[u8; 32]>,
    tables: Vec<MachineTableLayoutState>,
}

#[allow(dead_code)] // wired to public command dispatch by MI1-15
impl MachineParagraphLayout {
    pub const fn preparation(&self) -> &MachinePackagePreparation {
        &self.preparation
    }

    pub const fn flow(&self) -> &FlowTree {
        &self.flow
    }

    pub const fn initial(&self) -> &InitialPaginationState {
        &self.initial
    }

    pub const fn pagination(&self) -> &PaginationResult {
        &self.pagination
    }

    pub const fn flow_registry_sha256(&self) -> Option<[u8; 32]> {
        self.flow_registry_sha256
    }

    pub fn tables(&self) -> &[MachineTableLayoutState] {
        &self.tables
    }

    pub fn table_manifest_facts(&self) -> Result<Vec<StagingTableLayoutFacts>, Failure> {
        let mut facts = Vec::new();
        facts
            .try_reserve_exact(self.tables.len())
            .map_err(|_| Failure::limit("table manifest allocation failed"))?;
        for table in &self.tables {
            let trace = table.selected().trace_facts().map_err(|error| {
                Failure::internal(format!("table trace closure failed: {error:?}"))
            })?;
            let target_page_start = table
                .page_bodies()
                .first()
                .map(|page| page.target_page_index())
                .ok_or_else(|| Failure::internal("table Display page closure is empty"))?;
            facts.push(
                StagingTableLayoutFacts::from_selected_at(
                    &trace,
                    table.grid(),
                    table.selected(),
                    target_page_start,
                )
                .map_err(|error| {
                    Failure::internal(format!("table manifest closure failed: {error:?}"))
                })?,
            );
        }
        Ok(facts)
    }
}

/// Receipt-gated machine layout entry. There is no profile-ID argument and no
/// overload accepting a bare `ValidatedParsedPackage`.
#[allow(dead_code)] // wired to public command dispatch by MI1-15
pub fn layout_machine_paragraphs(
    package: &ValidatedMachinePackage,
    receipt: &MachinePdfPreflightReceipt,
    preparation: MachinePackagePreparation,
    config: &EffectiveConfig,
) -> Result<MachineParagraphLayout, Failure> {
    preparation.verify(package, receipt, config.limits())?;
    let parsed = package.package();
    let generated = parsed
        .bind_generated_text(preparation.generated(), config.limits())
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "machine generated-text binding mismatch at layout: {error:?}"
            ))
        })?;
    let epoch = LayoutEpoch::from_validated_inputs(generated, preparation.admitted().token())
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "machine layout epoch mismatch at layout: {error:?}"
            ))
        })?;
    let flow = build_machine_flow(package, generated, &preparation, epoch, config)?;
    let initial = InitialPaginationState::new(&flow, parsed, config.limits())
        .map_err(map_machine_pagination_error)?;
    let mut reflow_failure = None;
    let outcome = ReferencePaginator::new().paginate_with_reflow(
        parsed,
        &flow,
        config.limits(),
        false,
        |store, working_epoch| {
            let binding = match parsed.bind_generated_text(store, config.limits()) {
                Ok(binding) => binding,
                Err(error) => {
                    reflow_failure = Some(Failure::capability_mismatch(format!(
                        "machine generated-text reflow mismatch: {error:?}"
                    )));
                    return Err(PaginationError::PackageEpochMismatch);
                }
            };
            match build_machine_flow(package, binding, &preparation, working_epoch, config) {
                Ok(flow) => Ok(flow),
                Err(failure) => {
                    reflow_failure = Some(failure);
                    Err(PaginationError::FatalLayout)
                }
            }
        },
    );
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(_error) if reflow_failure.is_some() => {
            return Err(reflow_failure.expect("guarded machine reflow failure"))
        }
        Err(error) => return Err(map_machine_pagination_error(error)),
    };
    if !config.strict() {
        for advisory in outcome.diagnostics() {
            let diagnostic = advisory.as_diagnostic();
            crate::write_stderr_line(&format!(
                "{}: {}",
                diagnostic.code().as_str(),
                diagnostic.message()
            ))?;
        }
    }
    let pagination = outcome.into_result();
    let flow_registry_sha256 = if matches!(
        preparation.profile(),
        MachinePdfProfileId::BasicDocument1 | MachinePdfProfileId::Table1
    ) {
        Some(basic_flow_registry_sha256(
            parsed,
            pagination.selected_flow(),
            config.limits(),
        )?)
    } else {
        None
    };
    let tables = if preparation.profile() == MachinePdfProfileId::TABLE_1 {
        build_machine_table_layouts(package, &preparation, config, &pagination)?
    } else {
        Vec::new()
    };
    Ok(MachineParagraphLayout {
        preparation,
        flow,
        initial,
        pagination,
        flow_registry_sha256,
        tables,
    })
}

fn basic_flow_registry_sha256(
    package: &ValidatedParsedPackage,
    flow: &FlowTree,
    limits: &ValidatedResourceLimits,
) -> Result<[u8; 32], Failure> {
    let paragraph_items = flow.paragraph_items().ok_or_else(|| {
        Failure::capability_mismatch("basic-document selected flow lacks paragraph content")
    })?;
    let mut builder = ProductionFlowIrBuilder::new(package, paragraph_items, flow.epoch(), limits)
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "basic-document flow registry construction failed: {error:?}"
            ))
        })?;
    let owners: Vec<_> = builder.expected_content_owners().collect();
    for owner in owners {
        let content = builder.issue_content(owner).map_err(|error| {
            Failure::capability_mismatch(format!(
                "basic-document flow content issuance failed: {error:?}"
            ))
        })?;
        builder.register_content(content).map_err(|error| {
            Failure::capability_mismatch(format!(
                "basic-document flow content registration failed: {error:?}"
            ))
        })?;
    }
    let registry = builder.finish().map_err(|error| {
        Failure::capability_mismatch(format!(
            "basic-document flow registry closure failed: {error:?}"
        ))
    })?;
    Ok(registry.registry().receipt().fingerprint().bytes())
}

fn build_production_flow_ir(
    package: &ValidatedParsedPackage,
    paragraph_items: &ValidatedParagraphItemRegistry,
    epoch: LayoutEpoch,
    limits: &ValidatedResourceLimits,
) -> Result<ProductionFlowIr, Failure> {
    let mut builder = ProductionFlowIrBuilder::new(package, paragraph_items, epoch, limits)
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "table flow registry construction failed: {error:?}"
            ))
        })?;
    let owners: Vec<_> = builder.expected_content_owners().collect();
    for owner in owners {
        let content = builder.issue_content(owner).map_err(|error| {
            Failure::capability_mismatch(format!("table flow content issuance failed: {error:?}"))
        })?;
        builder.register_content(content).map_err(|error| {
            Failure::capability_mismatch(format!(
                "table flow content registration failed: {error:?}"
            ))
        })?;
    }
    builder.finish().map_err(|error| {
        Failure::capability_mismatch(format!("table flow registry closure failed: {error:?}"))
    })
}

fn build_machine_table_layouts(
    package: &ValidatedMachinePackage,
    preparation: &MachinePackagePreparation,
    config: &EffectiveConfig,
    pagination: &PaginationResult,
) -> Result<Vec<MachineTableLayoutState>, Failure> {
    let parsed = package.package();
    let epoch = pagination.selected_flow().epoch();
    let table_view = package.basic_document_view().ok_or_else(|| {
        Failure::capability_mismatch("table profile syntax view was not retained")
    })?;
    let generated = parsed
        .bind_generated_text(pagination.selected_pass().generated_text(), config.limits())
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "table generated-text binding mismatch: {error:?}"
            ))
        })?;
    let default_master = parsed
        .package()
        .page_masters
        .masters
        .iter()
        .find(|master| master.master_id == parsed.package().page_masters.default_master_id)
        .ok_or_else(|| Failure::internal("default page master is missing"))?;
    let body = default_master.body;
    let body_inline_size = body.width();
    let body_block_size = body.height();

    let table_owners: Vec<_> = parsed
        .package()
        .document
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Table { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .collect();
    if table_owners.is_empty() {
        return Ok(Vec::new());
    }

    // Resolve cell frame widths before the final line-break pass. The
    // preliminary IR is already package/epoch complete; only the paragraph
    // line shapes are replaced below.
    let preliminary_items = pagination
        .selected_flow()
        .paragraph_items()
        .ok_or_else(|| Failure::capability_mismatch("table layout lacks paragraph content"))?;
    let preliminary_ir =
        build_production_flow_ir(parsed, preliminary_items, epoch, config.limits())?;
    let mut paragraph_widths = BTreeMap::new();
    for table_owner in &table_owners {
        let style = table_view
            .compute_table_style(*table_owner)
            .map_err(|error| {
                Failure::capability_mismatch(format!(
                    "table typed style receipt changed after preflight: {error:?}"
                ))
            })?;
        let grid = layout_table_grid(
            &table_view,
            *table_owner,
            &style,
            &preliminary_ir,
            body_inline_size,
            config.limits(),
        )
        .map_err(map_machine_table_grid_error)?;
        for cell in grid.cells() {
            for paragraph in table_cell_paragraph_owners(
                &parsed.package().document.blocks,
                *table_owner,
                cell.cell_owner(),
            )
            .ok_or_else(|| Failure::capability_mismatch("validated table cell owner disappeared"))?
            {
                let inline_size =
                    table_cell_paragraph_inline_size(parsed, paragraph, cell.frame_inline_size())?;
                if paragraph_widths.insert(paragraph, inline_size).is_some() {
                    return Err(Failure::capability_mismatch(
                        "table paragraph belongs to more than one cell",
                    ));
                }
            }
        }
    }

    let final_breaks = layout_paragraphs_with_fonts(
        parsed,
        generated,
        preparation.admitted(),
        epoch,
        config,
        ParagraphStyleFonts::Machine(preparation.style_fonts()),
        Some(&paragraph_widths),
    )?;
    let paragraph_items =
        ValidatedParagraphItemRegistry::from_breaks_allowing_empty(parsed, epoch, &final_breaks)
            .map_err(map_machine_break_error)?;

    let mut states = Vec::new();
    states
        .try_reserve_exact(table_owners.len())
        .map_err(|_| Failure::limit("table layout allocation failed"))?;
    let mut next_target_page = 0u32;
    for table_owner in table_owners {
        let ir = build_production_flow_ir(parsed, &paragraph_items, epoch, config.limits())?;
        let style = table_view
            .compute_table_style(table_owner)
            .map_err(|error| {
                Failure::capability_mismatch(format!(
                    "table typed style receipt changed during layout: {error:?}"
                ))
            })?;
        let grid = layout_table_grid(
            &table_view,
            table_owner,
            &style,
            &ir,
            body_inline_size,
            config.limits(),
        )
        .map_err(map_machine_table_grid_error)?;
        let mut cell_inputs = Vec::new();
        cell_inputs
            .try_reserve_exact(grid.cells().len())
            .map_err(|_| Failure::limit("table cell layout allocation failed"))?;
        for cell in grid.cells() {
            let paragraphs = table_cell_paragraph_owners(
                &parsed.package().document.blocks,
                table_owner,
                cell.cell_owner(),
            )
            .ok_or_else(|| {
                Failure::capability_mismatch("validated table cell owner disappeared")
            })?;
            let mut fragment_sizes = Vec::new();
            for paragraph in paragraphs {
                let line_count = paragraph_items
                    .paragraph_break(paragraph)
                    .map_or(0, |receipt| receipt.lines.len());
                if line_count == 0 {
                    continue;
                }
                let computed = parsed.cascade_style(paragraph).map_err(|error| {
                    Failure::capability_mismatch(format!(
                        "table cell paragraph style changed during layout: {error:?}"
                    ))
                })?;
                let line_height = match computed.computed().properties().get("line_height") {
                    Some(StyleValue::Length(value)) => PositiveLength::new(*value),
                    _ => None,
                }
                .ok_or_else(|| {
                    Failure::capability_mismatch(
                        "table cell paragraph lacks a positive line-height",
                    )
                })?;
                let space_before = table_nonnegative_style_length(
                    computed.computed().properties().get("space_before"),
                )?;
                let space_after = table_nonnegative_style_length(
                    computed.computed().properties().get("space_after"),
                )?;
                fragment_sizes
                    .try_reserve_exact(line_count)
                    .map_err(|_| Failure::limit("table line layout allocation failed"))?;
                for line_index in 0..line_count {
                    let mut block_size = line_height.get();
                    if line_index == 0 {
                        block_size =
                            block_size.checked_add(space_before.get()).ok_or_else(|| {
                                Failure::capability_mismatch(
                                    "table paragraph block-start spacing overflowed",
                                )
                            })?;
                    }
                    if line_index + 1 == line_count {
                        block_size =
                            block_size.checked_add(space_after.get()).ok_or_else(|| {
                                Failure::capability_mismatch(
                                    "table paragraph block-end spacing overflowed",
                                )
                            })?;
                    }
                    fragment_sizes.push(PositiveLength::new(block_size).ok_or_else(|| {
                        Failure::capability_mismatch(
                            "table paragraph produced a non-positive line fragment",
                        )
                    })?);
                }
            }
            cell_inputs.push(TableCellLayoutInput::new(
                cell.cell_owner(),
                cell.flow_id(),
                fragment_sizes,
            ));
        }
        let row_bands = layout_table_row_bands(&grid, cell_inputs, config.limits())
            .map_err(map_machine_table_row_band_error)?;

        let first_row_owner = grid
            .rows()
            .first()
            .map(|row| row.row_owner())
            .ok_or_else(|| Failure::capability_mismatch("validated table has no row"))?;
        let (source_page, source_y) = pagination
            .selected_pages()
            .iter()
            .find_map(|page| {
                page.fragments
                    .iter()
                    .find(|fragment| fragment.owner == first_row_owner)
                    .map(|fragment| (page.page_index, fragment.bounds.y().raw()))
            })
            .ok_or_else(|| {
                Failure::capability_mismatch("selected body flow omitted a table row placeholder")
            })?;
        let mut target_page = source_page.max(next_target_page);
        let mut first_offset = if target_page == source_page {
            source_y
                .checked_sub(body.y().raw())
                .and_then(|value| value.checked_add(grid.space_before().get().raw()))
                .ok_or_else(|| Failure::internal("table placement arithmetic overflow"))?
        } else {
            0
        };
        if first_offset < 0 {
            return Err(Failure::internal(
                "selected table placeholder starts outside the body frame",
            ));
        }
        if first_offset >= body_block_size.get().raw() {
            target_page = target_page
                .checked_add(1)
                .ok_or_else(|| Failure::limit("table page index overflow"))?;
            first_offset = 0;
        }
        let first_remaining = body_block_size
            .get()
            .raw()
            .checked_sub(first_offset)
            .and_then(Length::from_raw)
            .and_then(PositiveLength::new)
            .ok_or_else(|| Failure::internal("table first-page extent is invalid"))?;
        let page_input = StagingTablePageInput::new(body_block_size, first_remaining)
            .map_err(map_machine_table_pagination_error)?;
        let selected = paginate_staging_table(&row_bands, &ir, page_input, config.limits())
            .map_err(map_machine_table_pagination_error)?;
        let last_target_page = target_page
            .checked_add(selected.page_count().saturating_sub(1))
            .ok_or_else(|| Failure::limit("table page index overflow"))?;
        if last_target_page >= config.limits().get().max_pages {
            return Err(Failure::limit(
                "L5110: table placement exceeded the configured page limit",
            ));
        }
        let page_bodies = (0..selected.page_count())
            .map(|page_index| {
                TablePaintPageBody::new_at(page_index, target_page + page_index, body)
            })
            .collect::<Vec<_>>();
        next_target_page = last_target_page
            .checked_add(1)
            .ok_or_else(|| Failure::limit("table page index overflow"))?;
        states.push(MachineTableLayoutState {
            paragraph_items: paragraph_items.clone(),
            grid,
            row_bands,
            selected,
            page_bodies,
        });
    }
    Ok(states)
}

fn table_cell_paragraph_owners(
    blocks: &[Block],
    table_owner: NodeId,
    cell_owner: NodeId,
) -> Option<Vec<NodeId>> {
    let table = blocks
        .iter()
        .find(|block| matches!(block, Block::Table { node_id, .. } if *node_id == table_owner))?;
    let Block::Table { head, body, .. } = table else {
        return None;
    };
    let cell = head
        .iter()
        .chain(body)
        .flat_map(|row| &row.cells)
        .find(|cell| cell.node_id == cell_owner)?;
    cell.blocks
        .iter()
        .map(|block| match block {
            Block::Paragraph { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .collect()
}

fn table_nonnegative_style_length(
    value: Option<&StyleValue>,
) -> Result<NonNegativeLength, Failure> {
    match value {
        Some(StyleValue::Length(value)) => NonNegativeLength::new(*value).ok_or_else(|| {
            Failure::capability_mismatch("table cell paragraph has a negative block length")
        }),
        None => Ok(NonNegativeLength::ZERO),
        Some(_) => Err(Failure::capability_mismatch(
            "table cell paragraph has a non-length block value",
        )),
    }
}

fn table_cell_paragraph_inline_size(
    package: &ValidatedParsedPackage,
    paragraph: NodeId,
    cell_inline_size: PositiveLength,
) -> Result<PositiveLength, Failure> {
    let computed = package.cascade_style(paragraph).map_err(|error| {
        Failure::capability_mismatch(format!(
            "table cell paragraph style resolution failed: {error:?}"
        ))
    })?;
    let start =
        table_nonnegative_style_length(computed.computed().properties().get("start_indent"))?;
    let end = table_nonnegative_style_length(computed.computed().properties().get("end_indent"))?;
    cell_inline_size
        .get()
        .checked_sub(start.get())
        .and_then(|value| value.checked_sub(end.get()))
        .and_then(PositiveLength::new)
        .ok_or_else(|| {
            Failure::capability_mismatch(
                "L5100: table cell paragraph indents leave no positive inline size",
            )
        })
}

fn map_machine_table_grid_error(error: typaxis_layout::TableGridLayoutError) -> Failure {
    use typaxis_layout::TableGridLayoutError as Error;
    match error {
        Error::AstNodeLimit => {
            Failure::limit(format!("P1120: table AST limit exceeded: {error:?}"))
        }
        Error::UnsupportedTablePlacement(_)
        | Error::UnsupportedCellContent(_)
        | Error::EmptyColumns(_)
        | Error::EmptyRows(_)
        | Error::ColumnArithmetic
        | Error::GridOutOfRange(_)
        | Error::GridOverlap(_)
        | Error::GridHole(_)
        | Error::RowspanOutOfRange(_) => {
            Failure::capability_mismatch(format!("L5100: table layout rejected input: {error:?}"))
        }
        _ => Failure::internal(format!("table grid invariant failed: {error:?}")),
    }
}

fn map_machine_table_row_band_error(error: typaxis_layout::TableRowBandLayoutError) -> Failure {
    if error == typaxis_layout::TableRowBandLayoutError::FragmentLimit {
        Failure::limit(format!("L5110: table fragment limit exceeded: {error:?}"))
    } else {
        Failure::internal(format!("table row-band invariant failed: {error:?}"))
    }
}

fn map_machine_table_pagination_error(
    error: typaxis_pagination::StagingTablePaginationError,
) -> Failure {
    use typaxis_pagination::StagingTablePaginationError as Error;
    match error {
        Error::HeaderOversize(_) | Error::RowOversize(_) => {
            Failure::capability_mismatch(format!("L5100: table row is oversize: {error:?}"))
        }
        Error::FragmentLimit => {
            Failure::limit(format!("L5110: table fragment limit exceeded: {error:?}"))
        }
        Error::PageLimit => Failure::limit(format!("table page limit exceeded: {error:?}")),
        _ => Failure::internal(format!("table pagination invariant failed: {error:?}")),
    }
}

fn build_reference_flow(
    package: &ValidatedParsedPackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    admitted: &AdmittedResourceLedger,
    epoch: LayoutEpoch,
    config: &EffectiveConfig,
) -> Result<FlowTree, Failure> {
    let paragraph_breaks = layout_paragraphs(package, generated, admitted, epoch, config)?;
    let paragraph_items = ValidatedParagraphItemRegistry::from_breaks_allowing_empty(
        package,
        epoch,
        &paragraph_breaks,
    )
    .map_err(|error| Failure::input(format!("L5000: unsupported flow content: {error:?}")))?;
    let mut flow_builder = CanonicalFlowIrBuilder::new(package, &paragraph_items)
        .map_err(|error| Failure::input(format!("L5000: unsupported flow content: {error:?}")))?;
    for (node, item_count) in paragraph_items.paragraphs() {
        for item_index in 0..item_count {
            flow_builder
                .push_paragraph_item(node, item_index)
                .map_err(|error| {
                    Failure::internal(format!("flow construction failed: {error:?}"))
                })?;
        }
    }
    let flow = flow_builder
        .finish(epoch)
        .map_err(|error| Failure::internal(format!("flow construction failed: {error:?}")))?;
    Ok(flow)
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn build_machine_flow(
    package: &ValidatedMachinePackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    preparation: &MachinePackagePreparation,
    epoch: LayoutEpoch,
    config: &EffectiveConfig,
) -> Result<FlowTree, Failure> {
    let parsed = package.package();
    if !preparation
        .style_fonts()
        .matches_package_epoch(package, epoch)
    {
        return Err(Failure::capability_mismatch(
            "machine style/font coverage does not match the working layout epoch",
        ));
    }
    let paragraph_breaks =
        layout_machine_paragraphs_for_epoch(package, generated, preparation, epoch, config)?;
    let paragraph_items = ValidatedParagraphItemRegistry::from_breaks_allowing_empty(
        parsed,
        epoch,
        &paragraph_breaks,
    )
    .map_err(map_machine_break_error)?;
    if matches!(
        preparation.profile(),
        MachinePdfProfileId::BasicDocument1 | MachinePdfProfileId::Table1
    ) {
        let mut flow_builder = CanonicalFlowIrBuilder::new(parsed, &paragraph_items)
            .map_err(|error| map_machine_flow_error(MachineParagraphFlowError::Flow(error)))?;
        if preparation.profile() == MachinePdfProfileId::TABLE_1 {
            flow_builder.use_separate_table_cell_flows();
        }
        let table_cell_paragraphs = if preparation.profile() == MachinePdfProfileId::TABLE_1 {
            table_cell_paragraph_owner_set(&parsed.package().document.blocks)
        } else {
            BTreeMap::new()
        };
        for (node, kind) in parsed.document_nodes().nodes() {
            match kind {
                DocumentNodeKind::Paragraph | DocumentNodeKind::Heading => {
                    if table_cell_paragraphs.contains_key(&node) {
                        continue;
                    }
                    let item_count = paragraph_items.item_count(node).ok_or_else(|| {
                        Failure::capability_mismatch(
                            "basic-document paragraph registry is incomplete",
                        )
                    })?;
                    for item_index in 0..item_count {
                        flow_builder
                            .push_paragraph_item(node, item_index)
                            .map_err(|error| {
                                map_machine_flow_error(MachineParagraphFlowError::Flow(error))
                            })?;
                    }
                }
                DocumentNodeKind::ListItem => {
                    flow_builder.push_list_item(node).map_err(|error| {
                        map_machine_flow_error(MachineParagraphFlowError::Flow(error))
                    })?
                }
                DocumentNodeKind::Figure | DocumentNodeKind::PageBreak => {
                    flow_builder.push_block_item(node).map_err(|error| {
                        map_machine_flow_error(MachineParagraphFlowError::Flow(error))
                    })?
                }
                DocumentNodeKind::TableRow => {
                    if preparation.profile() == MachinePdfProfileId::TABLE_1 {
                        flow_builder.push_table_row(node).map_err(|error| {
                            map_machine_flow_error(MachineParagraphFlowError::Flow(error))
                        })?;
                    } else {
                        return Err(Failure::capability_mismatch(
                            "table row reached basic-document flow after preflight",
                        ));
                    }
                }
                _ => {}
            }
        }
        return flow_builder
            .finish(epoch)
            .map_err(|error| map_machine_flow_error(MachineParagraphFlowError::Flow(error)));
    }
    let mut flow_builder =
        MachineParagraphFlowBuilder::new(package, &paragraph_items, preparation.style_fonts())
            .map_err(map_machine_flow_error)?;
    for (node, item_count) in paragraph_items.paragraphs() {
        for item_index in 0..item_count {
            flow_builder
                .push_paragraph_item(node, item_index)
                .map_err(map_machine_flow_error)?;
        }
    }
    flow_builder.finish(epoch).map_err(map_machine_flow_error)
}

fn table_cell_paragraph_owner_set(blocks: &[Block]) -> BTreeMap<NodeId, NodeId> {
    let mut owners = BTreeMap::new();
    for block in blocks {
        let Block::Table {
            node_id: table_owner,
            head,
            body,
            ..
        } = block
        else {
            continue;
        };
        for paragraph in head
            .iter()
            .chain(body)
            .flat_map(|row| &row.cells)
            .flat_map(|cell| &cell.blocks)
        {
            if let Block::Paragraph { node_id, .. } = paragraph {
                owners.insert(*node_id, *table_owner);
            }
        }
    }
    owners
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_break_error(error: typaxis_linebreak::BreakError) -> Failure {
    if error == typaxis_linebreak::BreakError::UnsupportedFlowDomain {
        Failure::capability_mismatch(
            "descriptor-approved content reached an unsupported paragraph-flow domain",
        )
    } else {
        Failure::internal(format!(
            "machine paragraph registry invariant failed: {error:?}"
        ))
    }
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_flow_error(error: MachineParagraphFlowError) -> Failure {
    match error {
        MachineParagraphFlowError::PreparationMismatch
        | MachineParagraphFlowError::Flow(typaxis_layout::FlowTreeError::UnsupportedFlowDomain) => {
            Failure::capability_mismatch(
                "descriptor-approved content reached an unsupported layout domain",
            )
        }
        MachineParagraphFlowError::Flow(error) => {
            Failure::internal(format!("machine flow construction failed: {error:?}"))
        }
    }
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_pagination_error(error: PaginationError) -> Failure {
    if error == PaginationError::FatalLayout {
        Failure::capability_mismatch(
            "descriptor-approved content reached an unsupported pagination/layout domain",
        )
    } else {
        map_pagination_error(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParagraphTextSite {
    Parsed(TextSpan),
    Generated(GeneratedBufferKey),
}

#[derive(Clone, Copy)]
#[allow(dead_code)] // machine variant is wired to dispatch by MI1-15
enum ParagraphStyleFonts<'a> {
    Reference(&'a AdmittedFontInstanceTable),
    Machine(&'a PreparedMachineStyleFonts),
}

#[allow(dead_code)] // machine variant is wired to dispatch by MI1-15
impl<'a> ParagraphStyleFonts<'a> {
    const fn font_instances(self) -> &'a AdmittedFontInstanceTable {
        match self {
            Self::Reference(instances) => instances,
            Self::Machine(prepared) => prepared.font_instances(),
        }
    }

    const fn is_machine(self) -> bool {
        matches!(self, Self::Machine(_))
    }

    fn computed_style(
        self,
        package: &ValidatedParsedPackage,
        site: ParagraphTextSite,
        text: &PackageShapeTextReceipt<'_>,
    ) -> Result<(typaxis_syntax::PackageComputedStyle, Option<FontInstanceId>), Failure> {
        match self {
            Self::Reference(_) => package
                .cascade_style(text.site_owner())
                .map(|computed| (computed, None))
                .map_err(|error| {
                    Failure::input(format!(
                        "L5000: paragraph style resolution failed: {error:?}"
                    ))
                }),
            Self::Machine(prepared) => {
                let source = match site {
                    ParagraphTextSite::Parsed(span) => MachineTextSiteSource::Parsed(span),
                    ParagraphTextSite::Generated(key) => MachineTextSiteSource::Generated(key),
                };
                let prepared_site = prepared.site(source, text.site_owner()).ok_or_else(|| {
                    Failure::capability_mismatch(
                        "descriptor-approved text site is missing from style/font preparation",
                    )
                })?;
                if prepared_site.site_owner() != text.site_owner()
                    || prepared_site.style_owner() != text.style_owner()
                {
                    return Err(Failure::capability_mismatch(
                        "prepared text-site ownership does not match the package",
                    ));
                }
                Ok((
                    prepared_site.computed().clone(),
                    Some(prepared_site.font_instance_id()),
                ))
            }
        }
    }

    fn unsupported(self, message: &'static str) -> Failure {
        if self.is_machine() {
            Failure::capability_mismatch(message)
        } else {
            Failure::input(format!("L5000: {message}"))
        }
    }
}

fn map_paragraph_break_failure(
    error: typaxis_linebreak::BreakError,
    style_fonts: ParagraphStyleFonts<'_>,
    operation: &'static str,
) -> Failure {
    if style_fonts.is_machine() && error == typaxis_linebreak::BreakError::UnsupportedFlowDomain {
        Failure::capability_mismatch(
            "descriptor-approved content reached an unsupported paragraph operation",
        )
    } else {
        Failure::input(format!("L5000: {operation} failed: {error:?}"))
    }
}

fn layout_paragraphs(
    package: &ValidatedParsedPackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    admitted: &AdmittedResourceLedger,
    epoch: LayoutEpoch,
    config: &EffectiveConfig,
) -> Result<Vec<ValidatedParagraphBreak>, Failure> {
    if package
        .package()
        .document
        .blocks
        .iter()
        .any(|block| !matches!(block, Block::Paragraph { .. } | Block::Heading { .. }))
    {
        return Err(Failure::input(
            "L5000: the reference layout backend accepts only paragraph and heading blocks",
        ));
    }
    let used_faces = package
        .package()
        .resources
        .font_faces
        .iter()
        .map(|font| font.font_face_id);
    let instances = AdmittedFontInstanceTable::from_used_faces(admitted, used_faces)
        .map_err(map_admission_error)?;
    layout_paragraphs_with_fonts(
        package,
        generated,
        admitted,
        epoch,
        config,
        ParagraphStyleFonts::Reference(&instances),
        None,
    )
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn layout_machine_paragraphs_for_epoch(
    package: &ValidatedMachinePackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    preparation: &MachinePackagePreparation,
    epoch: LayoutEpoch,
    config: &EffectiveConfig,
) -> Result<Vec<ValidatedParagraphBreak>, Failure> {
    let parsed = package.package();
    if !parsed.package().document.footnotes.is_empty()
        || (preparation.profile() == MachinePdfProfileId::PARAGRAPH_1
            && parsed
                .package()
                .document
                .blocks
                .iter()
                .any(|block| !matches!(block, Block::Paragraph { .. } | Block::Heading { .. })))
    {
        return Err(Failure::capability_mismatch(
            "descriptor-approved content reached an unsupported paragraph backend domain",
        ));
    }
    layout_paragraphs_with_fonts(
        parsed,
        generated,
        preparation.admitted(),
        epoch,
        config,
        ParagraphStyleFonts::Machine(preparation.style_fonts()),
        None,
    )
}

fn layout_paragraphs_with_fonts(
    package: &ValidatedParsedPackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    admitted: &AdmittedResourceLedger,
    epoch: LayoutEpoch,
    config: &EffectiveConfig,
    style_fonts: ParagraphStyleFonts<'_>,
    paragraph_widths: Option<&BTreeMap<NodeId, PositiveLength>>,
) -> Result<Vec<ValidatedParagraphBreak>, Failure> {
    let versions = config.data_versions();
    let data_tables =
        ResolvedDataTables::resolve(versions.unicode(), versions.japanese_line_break())
            .ok_or_else(|| Failure::internal("configured Unicode data tables are not linked"))?;
    let inline_size = package
        .package()
        .page_masters
        .masters
        .iter()
        .find(|master| master.master_id == package.package().page_masters.default_master_id)
        .map(|master| master.body.width())
        .ok_or_else(|| Failure::internal("default page master is missing"))?;
    let space_glue = ReferenceSpaceGlue::new(NonNegativeLength::ZERO, NonNegativeLength::ZERO);
    let mut cache = ShapingCache::new(config.limits());
    let paragraph_blocks = collect_layout_paragraph_blocks(&package.package().document.blocks);
    let mut breaks = Vec::new();
    breaks
        .try_reserve_exact(paragraph_blocks.len())
        .map_err(|_| Failure::limit("paragraph layout allocation failed"))?;
    for block in paragraph_blocks {
        let paragraph_node = match block {
            Block::Paragraph { node_id, .. } | Block::Heading { node_id, .. } => *node_id,
            _ => unreachable!("paragraph collection contains only text blocks"),
        };
        let selected_inline_size = paragraph_widths
            .and_then(|widths| widths.get(&paragraph_node).copied())
            .unwrap_or(inline_size);
        let line_shapes = [LineShape {
            inline_size: selected_inline_size,
        }];
        if let Some(receipt) = layout_paragraph(
            package,
            generated,
            admitted,
            style_fonts,
            epoch,
            &data_tables,
            block,
            space_glue,
            &line_shapes,
            &mut cache,
            config.limits(),
        )? {
            breaks.push(receipt);
        }
    }
    Ok(breaks)
}

fn collect_layout_paragraph_blocks(blocks: &[Block]) -> Vec<&Block> {
    let mut output = Vec::new();
    let mut pending: Vec<&Block> = blocks.iter().rev().collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::Paragraph { .. } | Block::Heading { .. } => output.push(block),
            Block::List { items, .. } => {
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Figure { caption, .. } => pending.extend(caption.iter().rev()),
            Block::Table { head, body, .. } => pending.extend(
                body.iter()
                    .rev()
                    .chain(head.iter().rev())
                    .flat_map(|row| row.cells.iter().rev())
                    .flat_map(|cell| cell.blocks.iter().rev()),
            ),
            Block::PageBreak { .. } => {}
        }
    }
    output
}

#[allow(clippy::too_many_arguments)]
fn layout_paragraph(
    package: &ValidatedParsedPackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    admitted: &AdmittedResourceLedger,
    style_fonts: ParagraphStyleFonts<'_>,
    epoch: LayoutEpoch,
    data_tables: &ResolvedDataTables,
    block: &Block,
    space_glue: ReferenceSpaceGlue,
    line_shapes: &[LineShape],
    cache: &mut ShapingCache,
    limits: &ValidatedResourceLimits,
) -> Result<Option<ValidatedParagraphBreak>, Failure> {
    let (paragraph_node, children) = match block {
        Block::Paragraph {
            node_id, children, ..
        }
        | Block::Heading {
            node_id, children, ..
        } => (*node_id, children.as_slice()),
        _ => {
            return Err(style_fonts
                .unsupported("unsupported block reached paragraph layout after profile preflight"))
        }
    };
    let mut sites = Vec::new();
    let mut has_explicit_break = false;
    collect_paragraph_text_sites(children, &mut sites, &mut has_explicit_break);
    if sites.is_empty() && !has_explicit_break {
        return Ok(None);
    }
    let breaker = OptimalParagraphBreaker;
    let factory = BoundedReferenceParagraphFactory::new();
    let break_canonical = |paragraph: &typaxis_linebreak::CanonicalParagraph| {
        let mut context = LineLayoutContext::from_limits(limits);
        let mut budget = context.take_budget().map_err(|error| {
            Failure::internal(format!("line-layout budget setup failed: {error:?}"))
        })?;
        break_paragraph_validated(&breaker, &paragraph.input(), &mut budget)
            .map_err(|error| map_paragraph_break_failure(error, style_fonts, "line layout"))
    };
    if sites.is_empty() {
        let paragraph = factory
            .build(
                generated,
                paragraph_node,
                epoch,
                &[],
                space_glue,
                line_shapes,
                LineShapeExhaustion::RepeatLast,
                limits,
            )
            .map_err(|error| {
                map_paragraph_break_failure(error, style_fonts, "paragraph construction")
            })?;
        return break_canonical(&paragraph).map(Some);
    }
    let mut inputs = Vec::new();
    inputs
        .try_reserve_exact(sites.len())
        .map_err(|_| Failure::limit("paragraph itemization allocation failed"))?;
    for site in sites {
        let text = bind_paragraph_text_site(package, generated, site)?;
        let (computed, expected_instance) = style_fonts.computed_style(package, site, &text)?;
        let selection = ShapeFontSelectionReceipt::new(
            package,
            &computed,
            admitted.token(),
            style_fonts.font_instances(),
            epoch,
        )
        .map_err(|error| {
            if style_fonts.is_machine() {
                Failure::capability_mismatch(format!(
                    "prepared font selection contradicted machine preflight: {error:?}"
                ))
            } else {
                Failure::input(format!("L5000: font selection failed: {error:?}"))
            }
        })?;
        if expected_instance
            .is_some_and(|expected| expected != selection.admitted_font().font_instance_id())
        {
            return Err(Failure::capability_mismatch(
                "prepared font instance changed before shaping",
            ));
        }
        inputs.push(ParagraphItemizationInput::new(computed, text, selection));
    }
    let itemized = CanonicalItemizer::new()
        .itemize_paragraph(package, paragraph_node, &inputs, epoch, data_tables, limits)
        .map_err(|error| {
            Failure::input(format!(
                "L5000: itemization failed for node {}: {error:?}",
                paragraph_node.get()
            ))
        })?;
    let mut run_sets = Vec::new();
    run_sets
        .try_reserve_exact(itemized.len())
        .map_err(|_| Failure::limit("shaped-run allocation failed"))?;
    for site in &itemized {
        let mut runs = Vec::new();
        if let Some(site) = site {
            runs.try_reserve_exact(site.requests().len())
                .map_err(|_| Failure::limit("shaped-run allocation failed"))?;
            for request in site.requests() {
                runs.push(
                    cache
                        .shape(&LinkedShaper::new(), request.clone())
                        .map_err(|error| {
                            Failure::input(format!("L5000: shaping failed: {error:?}"))
                        })?,
                );
            }
        }
        run_sets.push(runs);
    }
    let shaped: Vec<_> = inputs
        .iter()
        .zip(&itemized)
        .zip(&run_sets)
        .map(|((input, itemized), runs)| match itemized {
            Some(itemized) => ParagraphShapedText::from_itemized(itemized, runs),
            None => ParagraphShapedText::empty(input.text_receipt()),
        })
        .collect();
    let paragraph = factory
        .build(
            generated,
            paragraph_node,
            epoch,
            &shaped,
            space_glue,
            line_shapes,
            LineShapeExhaustion::RepeatLast,
            limits,
        )
        .map_err(|error| {
            map_paragraph_break_failure(error, style_fonts, "paragraph construction")
        })?;
    break_canonical(&paragraph).map(Some)
}

fn collect_paragraph_text_sites(
    inlines: &[Inline],
    output: &mut Vec<ParagraphTextSite>,
    has_explicit_break: &mut bool,
) {
    for inline in inlines {
        match inline {
            Inline::Text { text_span, .. } => {
                output.push(ParagraphTextSite::Parsed(*text_span));
            }
            Inline::Reference {
                node_id, format, ..
            } => {
                let kind = match format {
                    ReferenceFormat::Page => GenerationKind::PageReference,
                    ReferenceFormat::Text | ReferenceFormat::Number => GenerationKind::Counter,
                };
                output.push(ParagraphTextSite::Generated(GeneratedBufferKey::new(
                    *node_id, kind, 0,
                )));
            }
            Inline::FootnoteReference { node_id, .. } => {
                output.push(ParagraphTextSite::Generated(GeneratedBufferKey::new(
                    *node_id,
                    GenerationKind::FootnoteMarker,
                    0,
                )));
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => {
                collect_paragraph_text_sites(children, output, has_explicit_break);
            }
            Inline::SoftBreak { .. } | Inline::HardBreak { .. } => {
                *has_explicit_break = true;
            }
            Inline::Anchor { .. } => {}
        }
    }
}

fn bind_paragraph_text_site<'a>(
    package: &'a ValidatedParsedPackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'a>,
    site: ParagraphTextSite,
) -> Result<PackageShapeTextReceipt<'a>, Failure> {
    match site {
        ParagraphTextSite::Parsed(span) => package
            .bind_parsed_shape_text(span)
            .map_err(|error| Failure::internal(format!("parsed text binding failed: {error:?}"))),
        ParagraphTextSite::Generated(key) => {
            let buffer = generated
                .generated_text()
                .buffers()
                .iter()
                .find(|buffer| buffer.key() == key)
                .ok_or_else(|| Failure::internal("generated text site is missing"))?;
            let end = u32::try_from(buffer.utf8().len())
                .map_err(|_| Failure::limit("generated text site is too large"))?;
            let provenance = generated
                .generated_text()
                .provenance(key, Utf8ByteOffset::new(0), Utf8ByteOffset::new(end))
                .map_err(|error| {
                    Failure::internal(format!("generated text provenance failed: {error:?}"))
                })?;
            generated
                .bind_generated_shape_text(provenance)
                .map_err(|error| {
                    Failure::internal(format!("generated text binding failed: {error:?}"))
                })
        }
    }
}

pub fn admit_resources(
    package: &ValidatedParsedPackage,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<AdmittedResourceLedger, Failure> {
    let declarations = &package.package().resources;
    if declarations.font_faces.is_empty() && declarations.images.is_empty() {
        return AdmittedResourceResolver::new(declarations, config.limits())
            .and_then(AdmittedResourceResolver::finish)
            .map_err(map_admission_error);
    }
    let session = HostResourceAdmissionSession::new(admission, config, declarations)
        .map_err(map_admission_error)?;
    admit_registered_resources(package, config, Some(session))
}

fn admit_registered_resources(
    package: &ValidatedParsedPackage,
    config: &EffectiveConfig,
    session: Option<HostResourceAdmissionSession>,
) -> Result<AdmittedResourceLedger, Failure> {
    let declarations = &package.package().resources;
    let Some(session) = session.as_ref() else {
        return AdmittedResourceResolver::new(declarations, config.limits())
            .and_then(AdmittedResourceResolver::finish)
            .map_err(map_admission_error);
    };
    let mut resolver =
        AdmittedResourceResolver::new_with_roots(declarations, config.limits(), session.roots())
            .map_err(map_admission_error)?;
    for declaration in &declarations.font_faces {
        let source = session
            .open_font(declaration.font_face_id)
            .map_err(map_admission_error)?;
        let pending = resolver.read_font(source).map_err(map_admission_error)?;
        resolver
            .parse_and_bind_sfnt(pending)
            .map_err(map_admission_error)?;
    }
    for declaration in &declarations.images {
        let source = session
            .open_image(declaration.image_id)
            .map_err(map_admission_error)?;
        let pending = resolver.read_image(source).map_err(map_admission_error)?;
        resolver
            .parse_and_bind_png(pending)
            .map_err(map_admission_error)?;
    }
    resolver.finish().map_err(map_admission_error)
}

struct ResourceAdmissionOrchestrationFailure {
    failure: Failure,
    progress: Option<typaxis_resources::ResourceAdmissionProgressToken>,
}

/// Machine orchestration retains the resolver-issued progress snapshot on
/// every failure after resolver construction. The compatibility wrapper above
/// intentionally keeps its historical `Failure` surface for reference code.
fn admit_registered_resources_with_progress(
    package: &ValidatedParsedPackage,
    config: &EffectiveConfig,
    session: Option<HostResourceAdmissionSession>,
) -> Result<AdmittedResourceLedger, ResourceAdmissionOrchestrationFailure> {
    let declarations = &package.package().resources;
    let mut resolver = match session.as_ref() {
        Some(session) => {
            AdmittedResourceResolver::new_with_roots(declarations, config.limits(), session.roots())
        }
        None => AdmittedResourceResolver::new(declarations, config.limits()),
    }
    .map_err(|error| ResourceAdmissionOrchestrationFailure {
        failure: map_admission_error(error),
        progress: None,
    })?;

    let Some(session) = session.as_ref() else {
        let progress = resolver.progress_token();
        return resolver
            .finish()
            .map_err(|error| ResourceAdmissionOrchestrationFailure {
                failure: map_admission_error(error),
                progress: Some(progress),
            });
    };

    for declaration in &declarations.font_faces {
        let source = session
            .open_font_with_subject(declaration.font_face_id)
            .map_err(|failure| ResourceAdmissionOrchestrationFailure {
                failure: map_admission_error(failure.error()),
                progress: Some(resolver.progress_token()),
            })?;
        let pending = resolver
            .read_font_with_subject(source)
            .map_err(map_resource_admission_outcome)?;
        resolver
            .parse_and_bind_sfnt_with_subject(pending)
            .map_err(map_resource_admission_outcome)?;
    }
    for declaration in &declarations.images {
        let source = session
            .open_image_with_subject(declaration.image_id)
            .map_err(|failure| ResourceAdmissionOrchestrationFailure {
                failure: map_admission_error(failure.error()),
                progress: Some(resolver.progress_token()),
            })?;
        let pending = resolver
            .read_image_with_subject(source)
            .map_err(map_resource_admission_outcome)?;
        resolver
            .parse_and_bind_png_with_subject(pending)
            .map_err(map_resource_admission_outcome)?;
    }
    let progress = resolver.progress_token();
    resolver
        .finish()
        .map_err(|error| ResourceAdmissionOrchestrationFailure {
            failure: map_admission_error(error),
            progress: Some(progress),
        })
}

fn map_resource_admission_outcome(
    outcome: typaxis_resources::ResourceAdmissionFailureOutcome,
) -> ResourceAdmissionOrchestrationFailure {
    let (failure, progress) = outcome.into_parts();
    ResourceAdmissionOrchestrationFailure {
        failure: map_admission_error(failure.error()),
        progress: Some(progress),
    }
}

pub fn reject_strict_fallback(
    layout: &ReferenceLayout,
    config: &EffectiveConfig,
) -> Result<(), Failure> {
    if config.strict() && !matches!(layout.pagination.status(), ConvergenceStatus::Converged) {
        Err(Failure::input(
            "G6001: strict mode rejected pagination fallback; no PDF was generated",
        ))
    } else {
        Ok(())
    }
}

pub fn reject_machine_strict_fallback(
    layout: &MachineParagraphLayout,
    config: &EffectiveConfig,
) -> Result<(), Failure> {
    if config.strict() && !matches!(layout.pagination.status(), ConvergenceStatus::Converged) {
        Err(Failure::input(
            "G6001: strict mode rejected pagination fallback; no PDF was generated",
        ))
    } else {
        Ok(())
    }
}

pub fn build_pdf_graph(
    package: &ValidatedParsedPackage,
    config: &EffectiveConfig,
    layout: &ReferenceLayout,
) -> Result<typaxis_pdf::FrozenPdfGraph, Failure> {
    let display = ValidatedDisplayDocument::paint_reference_paragraphs(
        package,
        &layout.pagination,
        layout.pagination.selected_flow(),
        config,
    )
    .map_err(|error| Failure::internal(format!("display construction failed: {error:?}")))?;
    let plans = ReferenceResourceFinalizer::new()
        .finalize(ResourceFinalizationInput {
            display: &display,
            admitted: &layout.admitted,
            limits: config.limits(),
        })
        .map_err(map_resource_error)?;
    typaxis_pdf::PdfBackend::build(display, plans, config.limits()).map_err(|error| match error {
        typaxis_pdf::PdfError::ObjectLimit | typaxis_pdf::PdfError::OutputTooLarge => {
            Failure::limit(format!("PDF resource limit exceeded: {error:?}"))
        }
        _ => Failure::internal(format!("PDF graph construction failed: {error:?}")),
    })
}

/// Machine PDF graph entry rechecks the same capability/session preparation
/// before consuming the complete admitted ledger. `check-package` stops before
/// this function and therefore performs no pagination, shaping, or PDF work.
#[allow(dead_code)] // wired to public command dispatch by MI1-15
pub fn build_machine_pdf_graph(
    package: &ValidatedMachinePackage,
    receipt: &MachinePdfPreflightReceipt,
    config: &EffectiveConfig,
    layout: &MachineParagraphLayout,
) -> Result<typaxis_pdf::FrozenPdfGraph, Failure> {
    layout
        .preparation
        .verify(package, receipt, config.limits())?;
    if layout.preparation.profile() == MachinePdfProfileId::TABLE_1 {
        let table_view = package.basic_document_view().ok_or_else(|| {
            Failure::capability_mismatch("table profile syntax view was not retained")
        })?;
        let links = BasicDocumentLinkPreflight::STAGING
            .run(&table_view)
            .map_err(|error| {
                Failure::capability_mismatch(format!(
                    "table profile link receipt changed after preflight: {error:?}"
                ))
            })?;
        let registry = layout
            .pagination
            .selected_flow()
            .paragraph_items()
            .ok_or_else(|| {
                Failure::capability_mismatch("table profile layout lacks paragraph clusters")
            })?;
        let clusters = if links.cluster_receipt().links().is_empty() {
            None
        } else {
            Some(
                ValidatedStagingMachineLinkClusters::from_registry(
                    &table_view,
                    links.cluster_receipt(),
                    registry,
                )
                .map_err(|error| {
                    Failure::capability_mismatch(format!(
                        "table profile link clusters failed: {error:?}"
                    ))
                })?,
            )
        };
        let table_inputs: Vec<_> = layout
            .tables()
            .iter()
            .map(|table| {
                TableProfilePaintInput::new(
                    table.grid(),
                    table.row_bands(),
                    table.selected(),
                    table.page_bodies(),
                    table.paragraph_items(),
                )
            })
            .collect();
        let display = ValidatedDisplayDocument::paint_table_profile(
            &table_view,
            &layout.pagination,
            layout.pagination.selected_flow(),
            &table_inputs,
            clusters.as_ref(),
            config,
        )
        .map_err(|error| {
            Failure::internal(format!("table display construction failed: {error:?}"))
        })?;
        let plans = ReferenceResourceFinalizer::new()
            .finalize(ResourceFinalizationInput {
                display: display.validated_document(),
                admitted: layout.preparation.admitted(),
                limits: config.limits(),
            })
            .map_err(map_resource_error)?;
        return typaxis_pdf::PdfBackend::build_table_profile(display, plans, config.limits())
            .map_err(|error| match error {
                typaxis_pdf::PdfError::ObjectLimit | typaxis_pdf::PdfError::OutputTooLarge => {
                    Failure::limit(format!("PDF resource limit exceeded: {error:?}"))
                }
                _ => Failure::internal(format!("PDF graph construction failed: {error:?}")),
            });
    }
    let display = if layout.preparation.profile() == MachinePdfProfileId::BASIC_DOCUMENT_1 {
        let basic = package.basic_document_view().ok_or_else(|| {
            Failure::capability_mismatch("basic-document syntax view was not retained")
        })?;
        let links = BasicDocumentLinkPreflight::STAGING
            .run(&basic)
            .map_err(|error| {
                Failure::capability_mismatch(format!(
                    "basic-document link receipt changed after preflight: {error:?}"
                ))
            })?;
        if links.cluster_receipt().links().is_empty() {
            ValidatedDisplayDocument::paint_reference_paragraphs(
                package.package(),
                &layout.pagination,
                layout.pagination.selected_flow(),
                config,
            )
            .map_err(|error| Failure::internal(format!("display construction failed: {error:?}")))?
        } else {
            let registry = layout
                .pagination
                .selected_flow()
                .paragraph_items()
                .ok_or_else(|| {
                    Failure::capability_mismatch(
                        "basic-document link layout lacks paragraph clusters",
                    )
                })?;
            let clusters = ValidatedStagingMachineLinkClusters::from_registry(
                &basic,
                links.cluster_receipt(),
                registry,
            )
            .map_err(|error| {
                Failure::capability_mismatch(format!(
                    "basic-document link clusters failed: {error:?}"
                ))
            })?;
            let display = StagingMachineLinkDisplay::from_selected(
                &basic,
                &layout.pagination,
                layout.pagination.selected_flow(),
                &clusters,
                config,
            )
            .map_err(|error| {
                Failure::internal(format!("link display construction failed: {error:?}"))
            })?;
            display.into_parts().0
        }
    } else {
        ValidatedDisplayDocument::paint_reference_paragraphs(
            package.package(),
            &layout.pagination,
            layout.pagination.selected_flow(),
            config,
        )
        .map_err(|error| Failure::internal(format!("display construction failed: {error:?}")))?
    };
    let plans = ReferenceResourceFinalizer::new()
        .finalize(ResourceFinalizationInput {
            display: &display,
            admitted: layout.preparation.admitted(),
            limits: config.limits(),
        })
        .map_err(map_resource_error)?;
    typaxis_pdf::PdfBackend::build(display, plans, config.limits()).map_err(|error| match error {
        typaxis_pdf::PdfError::ObjectLimit | typaxis_pdf::PdfError::OutputTooLarge => {
            Failure::limit(format!("PDF resource limit exceeded: {error:?}"))
        }
        _ => Failure::internal(format!("PDF graph construction failed: {error:?}")),
    })
}

pub fn validate_machine_table_pdf_closure(
    layout: &MachineParagraphLayout,
    graph: &typaxis_pdf::FrozenPdfGraph,
    receipt: &typaxis_pdf::VerifiedPdfBytesReceipt,
) -> Result<(), Failure> {
    if layout.preparation.profile() != MachinePdfProfileId::TABLE_1 {
        return Ok(());
    }
    if graph.table_closures().len() != layout.tables().len()
        || graph
            .table_closures()
            .iter()
            .zip(layout.tables())
            .any(|(closure, table)| closure.table_node_id() != table.grid().table_owner())
    {
        return Err(Failure::internal(
            "table PDF closure set differs from selected layout",
        ));
    }
    for closure in graph.table_closures() {
        TablePdfClosureReceipt::from_serialized(closure, graph, receipt)
            .map_err(|error| Failure::internal(format!("table PDF closure failed: {error:?}")))?;
    }
    Ok(())
}

fn map_pagination_error(error: PaginationError) -> Failure {
    match error {
        PaginationError::ResourceLimit | PaginationError::PageLimit => {
            Failure::limit(format!("pagination resource limit exceeded: {error:?}"))
        }
        PaginationError::FallbackRejectedByStrict => {
            Failure::input("G6001: strict mode rejected pagination fallback; no PDF was generated")
        }
        _ => Failure::internal(format!("pagination invariant failed: {error:?}")),
    }
}

fn map_admission_error(error: typaxis_resources::ResourceAdmissionError) -> Failure {
    use typaxis_resources::ResourceAdmissionError as Error;
    match error {
        Error::ResourceLimit => {
            Failure::limit(format!("resource admission limit exceeded: {error:?}"))
        }
        Error::MissingLogicalResource
        | Error::ConflictingLogicalResource
        | Error::ExpectedHashMismatch
        | Error::InvalidMetadata
        | Error::InvalidFontFamily
        | Error::MissingResourceCandidate
        | Error::AmbiguousResourceCandidate
        | Error::UnsafeResourceCandidate
        | Error::ResourceNotRegularFile => {
            Failure::input(format!("resource admission rejected the input: {error:?}"))
        }
        Error::UnsupportedContainedOpen => Failure::unsupported_contained_open(),
        Error::RootUnavailable
        | Error::RootNotDirectory
        | Error::ResourceRead
        | Error::ResourceLengthMismatch
        | Error::ResourceLockUnavailable => {
            Failure::io(format!("resource admission I/O failed: {error:?}"))
        }
        Error::AliasedRoot => {
            Failure::usage("configured resource roots resolve to the same directory")
        }
        Error::NonCanonicalResourceId
        | Error::ReceiptKindMismatch
        | Error::ReceiptIdentityMismatch
        | Error::ReceiptSessionMismatch
        | Error::MissingAdmittedRootSet
        | Error::RootSetMismatch => {
            Failure::internal(format!("resource admission invariant failed: {error:?}"))
        }
    }
}

fn map_resource_error(error: ResourceError) -> Failure {
    match error {
        ResourceError::ResourceLimit => {
            Failure::limit(format!("resource finalization limit exceeded: {error:?}"))
        }
        _ => Failure::internal(format!("resource finalization failed: {error:?}")),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::Cell;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use typaxis_core::{
        sha256, BidiLevel, ConfigResourceRoot, DocumentPackageContractId, EffectiveDataVersions,
        HostPath, Length, PdfStreamCompression, PositiveLength, ResourceLimits,
    };
    use typaxis_diagnostics::{
        MachineDiagnosticBudget, MachineDiagnosticPhase, L5100, L5101, T2100,
    };
    use typaxis_document_package as wire;
    use typaxis_machine_input::{HostMachineInputSession, MachineInputHostOptions};
    use typaxis_syntax::{DocumentPackageParser, MachineParseOutcome};

    fn config() -> EffectiveConfig {
        config_with_limits(ResourceLimits::default())
    }

    fn config_with_limits(limits: ResourceLimits) -> EffectiveConfig {
        EffectiveConfig::new(
            false,
            PdfStreamCompression::Flate,
            vec![ConfigResourceRoot::ProjectRoot],
            ["http", "https", "mailto", "tel"]
                .map(str::to_owned)
                .to_vec(),
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
            limits,
        )
        .unwrap()
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    struct MachineFixtureRoot(PathBuf);

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    impl MachineFixtureRoot {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "typaxis-mi1-11-{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    impl Drop for MachineFixtureRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    struct MachineFixture {
        _root: MachineFixtureRoot,
        package: Box<ValidatedMachinePackage>,
        admission: HostAdmissionContext,
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[derive(Clone, Copy)]
    enum MachineFixtureKind {
        Blank,
        Paragraph,
        Heading,
        AnchorPageReference,
        SoftHardBreaks,
        MissingGlyph,
        UnsupportedInline,
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_source_span() -> wire::WireSourceSpan {
        wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_default_master() -> wire::WirePageMaster {
        wire::WirePageMaster {
            master_id: "default".to_owned(),
            width: 10_000_000,
            height: 10_000_000,
            body: wire::WireRect {
                x: 0,
                y: 0,
                width: 10_000_000,
                height: 10_000_000,
            },
            header: None,
            footer: None,
            footnote: None,
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_style(selector: &str) -> wire::WireStyleRule {
        wire::WireStyleRule {
            style_id: format!("{selector}_style"),
            extends: None,
            selector: selector.to_owned(),
            source_order: 0,
            declarations: vec![
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::FontFamily,
                    value: wire::WireStyleValue::FontFamilyList {
                        families: vec!["Fixture".to_owned()],
                    },
                    important: false,
                },
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::FontSize,
                    value: wire::WireStyleValue::Length { value: 786_432 },
                    important: false,
                },
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::LineHeight,
                    value: wire::WireStyleValue::Length { value: 917_504 },
                    important: false,
                },
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::Page,
                    value: wire::WireStyleValue::Keyword {
                        value: "auto".to_owned(),
                    },
                    important: false,
                },
            ],
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_text_buffer(text: &str) -> wire::WireTextBuffer {
        wire::WireTextBuffer {
            text_id: 0,
            utf8: text.to_owned(),
            mappings: vec![wire::WireTextMapSegment {
                text_range: wire::WireByteRange {
                    start_byte: 0,
                    end_byte: u32::try_from(text.len()).unwrap(),
                },
                kind: wire::WireTextMapKind::Inserted,
                source_span: None,
            }],
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_text_inline(text: &str, node_id: u32) -> wire::WireInline {
        wire::WireInline::Text {
            node_id,
            span: machine_source_span(),
            text_span: wire::WireTextSpan {
                text_id: 0,
                start_byte: 0,
                end_byte: u32::try_from(text.len()).unwrap(),
            },
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_paragraph(node_id: u32, children: Vec<wire::WireInline>) -> wire::WireBlock {
        wire::WireBlock::Paragraph {
            node_id,
            span: machine_source_span(),
            classes: Vec::new(),
            children,
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_wire(kind: MachineFixtureKind) -> wire::WireDocumentPackage {
        let mut package = wire::WireDocumentPackage {
            contract: DocumentPackageContractId::V1_0,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: "input.tsf".to_owned(),
                utf8_byte_length: 0,
                sha256: sha256(&[]),
            }],
            text_buffers: Vec::new(),
            document: wire::WireDocument {
                node_id: 0,
                blocks: Vec::new(),
                footnotes: Vec::new(),
            },
            style_sheet: wire::WireStyleSheet { rules: Vec::new() },
            page_masters: wire::WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![machine_default_master()],
                selection_rules: Vec::new(),
            },
            resources: wire::WireResourceCatalog {
                font_faces: Vec::new(),
                images: Vec::new(),
            },
        };
        let mut add_font = || {
            package.resources.font_faces = vec![wire::WireFontFace {
                font_face_id: 0,
                family: "Fixture".to_owned(),
                uri: "body.ttf".to_owned(),
                face_index: 0,
                expected_sha256: Some(sha256(&synthetic_ascii_ttf())),
            }];
        };
        match kind {
            MachineFixtureKind::Blank => {}
            MachineFixtureKind::Paragraph => {
                let text = "paragraph";
                add_font();
                package.text_buffers = vec![machine_text_buffer(text)];
                package.document.blocks =
                    vec![machine_paragraph(1, vec![machine_text_inline(text, 2)])];
                package.style_sheet.rules = vec![machine_style("paragraph")];
            }
            MachineFixtureKind::Heading => {
                let text = "heading";
                add_font();
                package.text_buffers = vec![machine_text_buffer(text)];
                package.document.blocks = vec![wire::WireBlock::Heading {
                    node_id: 1,
                    span: machine_source_span(),
                    classes: Vec::new(),
                    level: 2,
                    anchor_id: Some("heading".to_owned()),
                    children: vec![machine_text_inline(text, 2)],
                }];
                package.style_sheet.rules = vec![machine_style("heading")];
            }
            MachineFixtureKind::AnchorPageReference => {
                add_font();
                package.document.blocks = vec![
                    machine_paragraph(
                        1,
                        vec![wire::WireInline::Anchor {
                            node_id: 2,
                            span: machine_source_span(),
                            anchor_id: "target".to_owned(),
                        }],
                    ),
                    machine_paragraph(
                        3,
                        vec![wire::WireInline::Reference {
                            node_id: 4,
                            span: machine_source_span(),
                            target: "target".to_owned(),
                            format: wire::WireReferenceFormat::Page,
                        }],
                    ),
                ];
                package.style_sheet.rules = vec![machine_style("paragraph")];
            }
            MachineFixtureKind::SoftHardBreaks => {
                package.document.blocks = vec![machine_paragraph(
                    1,
                    vec![
                        wire::WireInline::SoftBreak {
                            node_id: 2,
                            span: machine_source_span(),
                        },
                        wire::WireInline::HardBreak {
                            node_id: 3,
                            span: machine_source_span(),
                        },
                    ],
                )];
            }
            MachineFixtureKind::MissingGlyph => {
                let text = "é";
                add_font();
                package.text_buffers = vec![machine_text_buffer(text)];
                package.document.blocks =
                    vec![machine_paragraph(1, vec![machine_text_inline(text, 2)])];
                package.style_sheet.rules = vec![machine_style("paragraph")];
            }
            MachineFixtureKind::UnsupportedInline => {
                let text = "unsupported";
                add_font();
                package.text_buffers = vec![machine_text_buffer(text)];
                package.document.blocks = vec![machine_paragraph(
                    1,
                    vec![wire::WireInline::Emphasis {
                        node_id: 2,
                        span: machine_source_span(),
                        children: vec![machine_text_inline(text, 3)],
                    }],
                )];
                package.style_sheet.rules = vec![machine_style("paragraph")];
            }
        }
        package
    }

    /// Materialize the same sealed machine fixtures used by the lower-pipeline
    /// tests for private CLI-runner tests. Keeping one wire fixture owner avoids
    /// a second hand-maintained paragraph/font model in `main.rs`.
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    pub(crate) fn write_machine_runner_fixture(root: &Path, kind: &str) {
        let kind = match kind {
            "blank" => MachineFixtureKind::Blank,
            "paragraph" => MachineFixtureKind::Paragraph,
            "unsupported-inline" => MachineFixtureKind::UnsupportedInline,
            other => panic!("unknown machine runner fixture `{other}`"),
        };
        fs::create_dir_all(root).unwrap();
        let bytes = wire::DocumentPackageEncoder::default()
            .to_jcs_vec(&machine_wire(kind))
            .unwrap();
        fs::write(root.join("document-package.json"), bytes).unwrap();
        fs::write(root.join("input.tsf"), []).unwrap();
        fs::write(root.join("body.ttf"), synthetic_ascii_ttf()).unwrap();
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn parse_machine_fixture(label: &str, kind: MachineFixtureKind) -> MachineFixture {
        let root = MachineFixtureRoot::new(label);
        let package_path = root.path().join("document-package.json");
        let bytes = wire::DocumentPackageEncoder::default()
            .to_jcs_vec(&machine_wire(kind))
            .unwrap();
        fs::write(&package_path, bytes).unwrap();
        fs::write(root.path().join("input.tsf"), []).unwrap();
        fs::write(root.path().join("body.ttf"), synthetic_ascii_ttf()).unwrap();
        let config = config();
        let options =
            MachineInputHostOptions::new(HostPath::new(package_path.clone()).unwrap(), None);
        let (session, raw) = HostMachineInputSession::open(options, config.limits()).unwrap();
        let decoded = session
            .decode_and_bind(
                &raw,
                &wire::StrictDocumentPackageDecoder::new(),
                &wire::DocumentPackageDecodePolicy::new(config.limits()),
            )
            .unwrap();
        let sources = session.admit_sources(&decoded, config.limits()).unwrap();
        let admitted = session.finish(raw, decoded, sources).unwrap();
        let policy =
            PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes()).unwrap();
        let package = match DocumentPackageParser::new().parse(admitted, &policy) {
            MachineParseOutcome::Parsed { package } => package,
            MachineParseOutcome::Failed { failure, .. } => panic!("fixture failed: {failure}"),
        };
        let admission = HostAdmissionContext::new(
            HostPath::new(package_path).unwrap(),
            HostPath::new(root.path().to_path_buf()).unwrap(),
            None,
            Vec::new(),
        );
        MachineFixture {
            _root: root,
            package,
            admission,
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn synthetic_ascii_ttf() -> Vec<u8> {
        const GLYPHS: u16 = 96;
        let mut head = vec![0; 54];
        head[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes());
        head[18..20].copy_from_slice(&1000u16.to_be_bytes());
        head[38..40].copy_from_slice(&(-200i16).to_be_bytes());
        head[40..42].copy_from_slice(&1000i16.to_be_bytes());
        head[42..44].copy_from_slice(&800i16.to_be_bytes());
        head[46..48].copy_from_slice(&8u16.to_be_bytes());
        head[48..50].copy_from_slice(&2i16.to_be_bytes());
        head[50..52].copy_from_slice(&1i16.to_be_bytes());
        let mut hhea = vec![0; 36];
        hhea[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        hhea[4..6].copy_from_slice(&800i16.to_be_bytes());
        hhea[6..8].copy_from_slice(&(-200i16).to_be_bytes());
        hhea[10..12].copy_from_slice(&600u16.to_be_bytes());
        hhea[34..36].copy_from_slice(&GLYPHS.to_be_bytes());
        let mut maxp = vec![0; 32];
        maxp[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        maxp[4..6].copy_from_slice(&GLYPHS.to_be_bytes());
        let mut hmtx = Vec::with_capacity(usize::from(GLYPHS) * 4);
        for glyph in 0..GLYPHS {
            hmtx.extend_from_slice(&(if glyph == 1 { 300u16 } else { 600u16 }).to_be_bytes());
            hmtx.extend_from_slice(&0i16.to_be_bytes());
        }
        let loca = vec![0; (usize::from(GLYPHS) + 1) * 4];
        let mut cmap = vec![0; 44];
        cmap[2..4].copy_from_slice(&1u16.to_be_bytes());
        cmap[4..6].copy_from_slice(&3u16.to_be_bytes());
        cmap[6..8].copy_from_slice(&1u16.to_be_bytes());
        cmap[8..12].copy_from_slice(&12u32.to_be_bytes());
        cmap[12..14].copy_from_slice(&4u16.to_be_bytes());
        cmap[14..16].copy_from_slice(&32u16.to_be_bytes());
        cmap[18..20].copy_from_slice(&4u16.to_be_bytes());
        cmap[20..22].copy_from_slice(&4u16.to_be_bytes());
        cmap[22..24].copy_from_slice(&1u16.to_be_bytes());
        cmap[26..28].copy_from_slice(&0x007eu16.to_be_bytes());
        cmap[28..30].copy_from_slice(&0xffffu16.to_be_bytes());
        cmap[32..34].copy_from_slice(&0x0020u16.to_be_bytes());
        cmap[34..36].copy_from_slice(&0xffffu16.to_be_bytes());
        cmap[36..38].copy_from_slice(&(-31i16).to_be_bytes());
        cmap[38..40].copy_from_slice(&1i16.to_be_bytes());
        let postscript_name = b"TypaxisSynthetic";
        let mut name = vec![0; 18 + postscript_name.len() * 2];
        name[2..4].copy_from_slice(&1u16.to_be_bytes());
        name[4..6].copy_from_slice(&18u16.to_be_bytes());
        name[6..8].copy_from_slice(&3u16.to_be_bytes());
        name[8..10].copy_from_slice(&1u16.to_be_bytes());
        name[10..12].copy_from_slice(&0x0409u16.to_be_bytes());
        name[12..14].copy_from_slice(&6u16.to_be_bytes());
        name[14..16].copy_from_slice(&(postscript_name.len() as u16 * 2).to_be_bytes());
        for (index, byte) in postscript_name.iter().copied().enumerate() {
            name[19 + index * 2] = byte;
        }
        let mut post = vec![0; 32];
        post[..4].copy_from_slice(&0x0003_0000u32.to_be_bytes());
        build_machine_test_sfnt(vec![
            (*b"cmap", cmap),
            (*b"glyf", vec![]),
            (*b"head", head),
            (*b"hhea", hhea),
            (*b"hmtx", hmtx),
            (*b"loca", loca),
            (*b"maxp", maxp),
            (*b"name", name),
            (*b"post", post),
        ])
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn build_machine_test_sfnt(mut tables: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
        tables.sort_by_key(|(tag, _)| *tag);
        let count = tables.len() as u16;
        let directory_len = 12 + tables.len() * 16;
        let payload_len: usize = tables.iter().map(|(_, bytes)| (bytes.len() + 3) & !3).sum();
        let mut output = vec![0; directory_len + payload_len];
        output[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        output[4..6].copy_from_slice(&count.to_be_bytes());
        let selector = u16::try_from(u16::BITS - 1 - count.leading_zeros()).unwrap();
        let search = 16u16 * (1u16 << selector);
        output[6..8].copy_from_slice(&search.to_be_bytes());
        output[8..10].copy_from_slice(&selector.to_be_bytes());
        output[10..12].copy_from_slice(&(count * 16 - search).to_be_bytes());
        let mut offset = directory_len;
        let mut head_adjustment = None;
        for (index, (tag, bytes)) in tables.iter().enumerate() {
            let record = 12 + index * 16;
            output[record..record + 4].copy_from_slice(tag);
            output[record + 4..record + 8]
                .copy_from_slice(&machine_test_sfnt_checksum(bytes).to_be_bytes());
            output[record + 8..record + 12].copy_from_slice(&(offset as u32).to_be_bytes());
            output[record + 12..record + 16].copy_from_slice(&(bytes.len() as u32).to_be_bytes());
            output[offset..offset + bytes.len()].copy_from_slice(bytes);
            if tag == b"head" {
                head_adjustment = Some(offset + 8);
            }
            offset = (offset + bytes.len() + 3) & !3;
        }
        if let Some(offset) = head_adjustment {
            let adjustment = 0xB1B0_AFBAu32.wrapping_sub(machine_test_sfnt_checksum(&output));
            output[offset..offset + 4].copy_from_slice(&adjustment.to_be_bytes());
        }
        output
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_test_sfnt_checksum(bytes: &[u8]) -> u32 {
        bytes.chunks(4).fold(0u32, |checksum, chunk| {
            let mut word = [0; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            checksum.wrapping_add(u32::from_be_bytes(word))
        })
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_preflight_receipt(package: &ValidatedMachinePackage) -> MachinePdfPreflightReceipt {
        let mut budget = MachineDiagnosticBudget::new();
        let receipt = {
            let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
            MachinePdfPreflight::PARAGRAPH_1
                .run(package, &mut diagnostics)
                .unwrap()
        };
        assert!(budget.finish().diagnostics().is_empty());
        receipt
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_supported_domain_reaches_a_frozen_pdf_graph() {
        for (label, kind) in [
            ("blank", MachineFixtureKind::Blank),
            ("paragraph", MachineFixtureKind::Paragraph),
            ("heading", MachineFixtureKind::Heading),
            ("anchor-reference", MachineFixtureKind::AnchorPageReference),
            ("soft-hard-breaks", MachineFixtureKind::SoftHardBreaks),
        ] {
            let fixture = parse_machine_fixture(label, kind);
            let config = config();
            let mut budget = MachineDiagnosticBudget::new();
            let checked = {
                let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
                check_machine_package_preparation(
                    &fixture.package,
                    &mut diagnostics,
                    &config,
                    &fixture.admission,
                )
                .unwrap()
            };
            assert!(budget.finish().diagnostics().is_empty());
            assert_eq!(
                checked.preparation().glyph_coverage(),
                MachineGlyphCoverage::DeferredToBuildShaping
            );
            if label == "soft-hard-breaks" {
                assert_eq!(
                    checked
                        .preparation()
                        .style_fonts()
                        .non_text_generated_sites()
                        .len(),
                    2
                );
                assert!(checked.preparation().style_fonts().sites().is_empty());
            }
            if label == "anchor-reference" {
                assert_eq!(checked.preparation().style_fonts().sites().len(), 1);
                assert_eq!(
                    checked.preparation().style_fonts().sites()[0].site_owner(),
                    typaxis_core::NodeId::new(4)
                );
            }
            let (receipt, preparation) = checked.into_parts();
            let layout =
                layout_machine_paragraphs(&fixture.package, &receipt, preparation, &config)
                    .unwrap_or_else(|error| panic!("{label} layout failed: {error:?}"));
            assert!(!layout.pagination().selected_pages().is_empty());
            let _graph = build_machine_pdf_graph(&fixture.package, &receipt, &config, &layout)
                .unwrap_or_else(|error| panic!("{label} PDF graph failed: {error:?}"));
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn check_preparation_resolves_family_but_defers_missing_glyph_coverage() {
        let fixture = parse_machine_fixture("missing-glyph", MachineFixtureKind::MissingGlyph);
        let config = config();
        let receipt = machine_preflight_receipt(&fixture.package);
        let preparation =
            prepare_machine_package(&fixture.package, &receipt, &config, &fixture.admission)
                .unwrap();
        assert_eq!(preparation.style_fonts().sites().len(), 1);
        assert_eq!(
            preparation.style_fonts().sites()[0].site_owner(),
            typaxis_core::NodeId::new(2)
        );
        assert_eq!(
            preparation.glyph_coverage(),
            MachineGlyphCoverage::DeferredToBuildShaping
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn receipt_swap_and_wrong_document_are_i9190_internal_failures() {
        let first = parse_machine_fixture("receipt-first", MachineFixtureKind::Paragraph);
        let same_bytes_other_session =
            parse_machine_fixture("receipt-session", MachineFixtureKind::Paragraph);
        let different_document =
            parse_machine_fixture("receipt-document", MachineFixtureKind::Heading);
        let config = config();
        let first_receipt = machine_preflight_receipt(&first.package);
        let session_receipt = machine_preflight_receipt(&same_bytes_other_session.package);
        let document_receipt = machine_preflight_receipt(&different_document.package);

        let preparation =
            prepare_machine_package(&first.package, &first_receipt, &config, &first.admission)
                .unwrap();
        let error =
            match layout_machine_paragraphs(&first.package, &session_receipt, preparation, &config)
            {
                Err(error) => error,
                Ok(_) => panic!("a receipt from another session must fail"),
            };
        assert_eq!(error.kind, FailureKind::Internal);
        assert!(error.message.starts_with("I9190:"));

        let foreign_preparation = prepare_machine_package(
            &same_bytes_other_session.package,
            &session_receipt,
            &config,
            &same_bytes_other_session.admission,
        )
        .unwrap();
        let error = match layout_machine_paragraphs(
            &first.package,
            &first_receipt,
            foreign_preparation,
            &config,
        ) {
            Err(error) => error,
            Ok(_) => panic!("a preparation from another session must fail"),
        };
        assert_eq!(error.kind, FailureKind::Internal);
        assert!(error.message.starts_with("I9190:"));

        let preparation =
            prepare_machine_package(&first.package, &first_receipt, &config, &first.admission)
                .unwrap();
        let error = match layout_machine_paragraphs(
            &first.package,
            &document_receipt,
            preparation,
            &config,
        ) {
            Err(error) => error,
            Ok(_) => panic!("a receipt for another document must fail"),
        };
        assert_eq!(error.kind, FailureKind::Internal);
        assert!(error.message.starts_with("I9190:"));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn unsupported_capability_stops_resource_layout_and_pdf_stages() {
        let fixture = parse_machine_fixture("unsupported", MachineFixtureKind::UnsupportedInline);
        let config = config();
        let resource_opens = Cell::new(0u32);
        let layout_starts = Cell::new(0u32);
        let pdf_starts = Cell::new(0u32);
        let mut budget = MachineDiagnosticBudget::new();
        let checked = {
            let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
            check_machine_package_preparation_with(
                &fixture.package,
                &mut diagnostics,
                &config,
                &fixture.admission,
                |package, config, admission| {
                    resource_opens.set(resource_opens.get() + 1);
                    admit_resources(package, config, admission)
                },
            )
        };
        let result = checked.and_then(|checked| {
            layout_starts.set(layout_starts.get() + 1);
            let (receipt, preparation) = checked.into_parts();
            let layout =
                layout_machine_paragraphs(&fixture.package, &receipt, preparation, &config)?;
            pdf_starts.set(pdf_starts.get() + 1);
            build_machine_pdf_graph(&fixture.package, &receipt, &config, &layout).map(|_| ())
        });
        let error = result.unwrap_err();
        assert_eq!(error.kind, FailureKind::Input);
        assert_eq!(resource_opens.get(), 0);
        assert_eq!(layout_starts.get(), 0);
        assert_eq!(pdf_starts.get(), 0);
        let read_ledger = fixture
            .package
            .provenance()
            .admission()
            .read_ledger_token()
            .unwrap();
        assert!(read_ledger
            .conflicts_with_write_target(
                &HostPath::new(fixture._root.path().join("body.ttf")).unwrap()
            )
            .unwrap());
        assert_eq!(read_ledger.stored_opened_identity_count(), 2);
        let diagnostics = budget.finish();
        assert_eq!(diagnostics.diagnostics().len(), 1);
        assert_eq!(*diagnostics.diagnostics()[0].code(), L5100);
    }

    #[test]
    fn unexpected_machine_flow_domain_is_i9190_not_user_input() {
        let failure = map_machine_flow_error(MachineParagraphFlowError::Flow(
            typaxis_layout::FlowTreeError::UnsupportedFlowDomain,
        ));
        assert_eq!(failure.kind, FailureKind::Internal);
        assert!(failure.message.starts_with("I9190:"));
        let pagination = map_machine_pagination_error(PaginationError::FatalLayout);
        assert_eq!(pagination.kind, FailureKind::Internal);
        assert!(pagination.message.starts_with("I9190:"));
    }

    #[test]
    fn diagnostic_failures_receive_stable_default_codes_without_double_prefixing() {
        assert_eq!(Failure::input("bad source").message, "P1000: bad source");
        assert_eq!(
            Failure::limit("too much work").message,
            "I9000: too much work"
        );
        assert_eq!(
            Failure::internal("broken invariant").message,
            "I9001: broken invariant"
        );
        assert_eq!(
            Failure::input("L5000: unsupported layout").message,
            "L5000: unsupported layout"
        );
        assert_eq!(
            Failure::limit("P1001: invalid limit configuration").message,
            "P1001: invalid limit configuration"
        );
    }

    #[test]
    fn unsupported_contained_open_leaves_requested_artifact_targets_untouched() {
        let failure = map_admission_error(
            typaxis_resources::ResourceAdmissionError::UnsupportedContainedOpen,
        );
        assert_eq!(failure.kind, FailureKind::Io);
        assert_eq!(
            failure.message,
            "resource admission I/O failed: UnsupportedContainedOpen"
        );
        assert!(!failure.should_publish_failed_manifest());
        assert!(Failure::io("ordinary I/O failure").should_publish_failed_manifest());
    }

    static NEXT_TEMP_SOURCE: AtomicU64 = AtomicU64::new(0);

    fn temp_source(contents: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_TEMP_SOURCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "typaxis-cli-{}-{unique}-{sequence}.tsf",
            std::process::id()
        ));
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        path
    }

    #[test]
    fn empty_source_reaches_a_converged_blank_page() {
        let path = temp_source("\n");
        let config = config();
        let package = load_package(&path, &config).unwrap();
        let admission = HostAdmissionContext::new(
            typaxis_core::HostPath::new(path.clone()).unwrap(),
            typaxis_core::HostPath::new(path.parent().unwrap().to_path_buf()).unwrap(),
            None,
            vec![],
        );
        let layout = layout_reference(&package, &config, &admission).unwrap();
        assert_eq!(layout.pagination.selected_pages().len(), 1);
        assert_eq!(layout.pagination.passes().len(), 2);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn empty_paragraph_uses_reference_fragmentation_and_pagination() {
        let path = temp_source("paragraph\n");
        let config = config();
        let package = load_package(&path, &config).unwrap();
        let admission = HostAdmissionContext::new(
            typaxis_core::HostPath::new(path.clone()).unwrap(),
            typaxis_core::HostPath::new(path.parent().unwrap().to_path_buf()).unwrap(),
            None,
            vec![],
        );
        let layout = layout_reference(&package, &config, &admission).unwrap();
        assert_eq!(layout.pagination.selected_pages().len(), 1);
        assert_eq!(layout.pagination.selected_pages()[0].fragments.len(), 1);
        assert_eq!(layout.pagination.passes().len(), 2);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn entry_source_rejects_a_directory() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("typaxis-cli-directory-{unique}"));
        fs::create_dir(&path).unwrap();

        let error = load_package(&path, &config()).unwrap_err();
        assert_eq!(error.kind, FailureKind::Io);
        assert!(error.message.contains("not a regular file"));

        fs::remove_dir(path).unwrap();
    }

    #[test]
    fn entry_snapshot_detects_same_length_timestamp_change() {
        let path = temp_source("paragraph\n");
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.set_times(
            fs::FileTimes::new()
                .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000)),
        )
        .unwrap();
        let first = InputFileSnapshot::from_file(&file, &path).unwrap();

        file.set_times(
            fs::FileTimes::new()
                .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_001)),
        )
        .unwrap();
        let second = InputFileSnapshot::from_file(&file, &path).unwrap();

        assert_eq!(first.length, second.length);
        assert_ne!(first, second);
        drop(file);
        fs::remove_file(path).unwrap();
    }

    #[cfg(all(
        unix,
        not(any(
            target_os = "espidf",
            target_os = "horizon",
            target_os = "solaris",
            target_os = "vita",
            target_os = "wasi"
        ))
    ))]
    #[test]
    fn entry_source_rejects_a_conflicting_writer_lock() {
        let path = temp_source("\n");
        let writer = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        rustix::fs::flock(
            &writer,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .unwrap();

        let error = load_package(&path, &config()).unwrap_err();
        assert_eq!(error.kind, FailureKind::Io);
        assert!(error.message.contains("stable read"));

        drop(writer);
        fs::remove_file(path).unwrap();
    }

    #[cfg(any(target_os = "android", target_os = "linux"))]
    #[test]
    fn entry_source_rejects_a_fifo_without_blocking() {
        use rustix::fs::{Mode, CWD};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("typaxis-cli-fifo-{unique}"));
        rustix::fs::mkfifoat(CWD, &path, Mode::RUSR | Mode::WUSR).unwrap();

        let error = load_package(&path, &config()).unwrap_err();
        assert_eq!(error.kind, FailureKind::Io);
        assert!(error.message.contains("not a regular file"));

        fs::remove_file(path).unwrap();
    }

    fn staging_style_declaration(
        name: wire::WireDeclarationName,
        value: wire::WireStyleValue,
    ) -> wire::WireDeclaration {
        wire::WireDeclaration {
            name,
            value,
            important: false,
        }
    }

    fn staging_positive(raw: i64) -> PositiveLength {
        PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap()
    }

    fn staging_style_run(
        package: &wire::WireDocumentPackage,
        input: TypedBlockLayoutInput,
    ) -> (
        Result<StagingMachineBlockStyleArtifacts, StagingMachineBlockStyleRunnerError>,
        typaxis_diagnostics::MachineDiagnostics,
    ) {
        let config = config();
        let bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(package)
            .unwrap();
        let policy =
            PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes()).unwrap();
        let mut budget = MachineDiagnosticBudget::new();
        let result = {
            let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
            exercise_basic_block_style_slice(
                &bytes,
                String::new(),
                &policy,
                config.limits(),
                NodeId::new(1),
                input,
                &mut diagnostics,
            )
        };
        (result, budget.finish())
    }

    fn machine_list_wire(start: u32) -> wire::WireDocumentPackage {
        let mut package = machine_wire(MachineFixtureKind::Blank);
        let span = machine_source_span();
        package.document.blocks = vec![wire::WireBlock::List {
            node_id: 1,
            span,
            classes: vec![],
            ordered: true,
            start: Some(start),
            items: vec![
                wire::WireListItem {
                    node_id: 2,
                    span,
                    blocks: vec![
                        machine_paragraph(3, vec![]),
                        wire::WireBlock::List {
                            node_id: 4,
                            span,
                            classes: vec![],
                            ordered: false,
                            start: None,
                            items: vec![wire::WireListItem {
                                node_id: 5,
                                span,
                                blocks: vec![machine_paragraph(6, vec![])],
                            }],
                        },
                    ],
                },
                wire::WireListItem {
                    node_id: 7,
                    span,
                    blocks: vec![machine_paragraph(8, vec![])],
                },
            ],
        }];
        package.style_sheet.rules = vec![wire::WireStyleRule {
            style_id: "machine-list".to_owned(),
            extends: None,
            selector: "list".to_owned(),
            source_order: 0,
            declarations: vec![
                staging_style_declaration(
                    wire::WireDeclarationName::FontFamily,
                    wire::WireStyleValue::FontFamilyList {
                        families: vec!["Fixture".to_owned()],
                    },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::FontSize,
                    wire::WireStyleValue::Length { value: 10 },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::LineHeight,
                    wire::WireStyleValue::Length { value: 12 },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::StartIndent,
                    wire::WireStyleValue::Length { value: 5 },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::EndIndent,
                    wire::WireStyleValue::Length { value: 3 },
                ),
            ],
        }];
        package
    }

    fn machine_page_break_wire() -> wire::WireDocumentPackage {
        let mut package = machine_wire(MachineFixtureKind::Blank);
        let span = machine_source_span();
        let page_break = |node_id| wire::WireBlock::PageBreak {
            node_id,
            span,
            classes: Vec::new(),
        };
        package.document.blocks = vec![
            page_break(1),
            machine_paragraph(2, Vec::new()),
            page_break(3),
            page_break(4),
            machine_paragraph(5, Vec::new()),
            page_break(6),
        ];
        package
    }

    const MACHINE_FIGURE_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1, 1, 3,
        0, 0, 0, 206, 236, 237, 201, 0, 0, 0, 6, 80, 76, 84, 69, 255, 0, 0, 0, 255, 0, 210, 135,
        239, 113, 0, 0, 0, 2, 116, 82, 78, 83, 255, 0, 229, 183, 48, 74, 0, 0, 0, 10, 73, 68, 65,
        84, 120, 156, 99, 112, 0, 0, 0, 66, 0, 65, 41, 55, 244, 239, 0, 0, 0, 0, 73, 69, 78, 68,
        174, 66, 96, 130,
    ];

    fn machine_figure_wire(
        keep_caption: bool,
        expected_sha256: [u8; 32],
    ) -> wire::WireDocumentPackage {
        let mut package = machine_wire(MachineFixtureKind::Blank);
        let span = machine_source_span();
        package.document.blocks = vec![wire::WireBlock::Figure {
            node_id: 1,
            span,
            classes: Vec::new(),
            image_id: 0,
            alt: "opaque-extension PNG".to_owned(),
            caption: vec![
                machine_paragraph(2, Vec::new()),
                machine_paragraph(3, Vec::new()),
            ],
        }];
        package.page_masters = wire::WirePageMasterSet {
            default_master_id: "default".to_owned(),
            masters: vec![wire::WirePageMaster {
                master_id: "default".to_owned(),
                width: 100,
                height: 100,
                body: wire::WireRect {
                    x: 10,
                    y: 10,
                    width: 80,
                    height: 70,
                },
                header: None,
                footer: None,
                footnote: None,
            }],
            selection_rules: Vec::new(),
        };
        package.resources.images = vec![wire::WireImage {
            image_id: 0,
            uri: "figure.data".to_owned(),
            expected_sha256: Some(expected_sha256),
        }];
        package.style_sheet.rules = vec![wire::WireStyleRule {
            style_id: "machine-figure".to_owned(),
            extends: None,
            selector: "figure".to_owned(),
            source_order: 0,
            declarations: vec![
                staging_style_declaration(
                    wire::WireDeclarationName::StartIndent,
                    wire::WireStyleValue::Length { value: 5 },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::EndIndent,
                    wire::WireStyleValue::Length { value: 5 },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::Width,
                    wire::WireStyleValue::Length { value: 40 },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::KeepCaption,
                    wire::WireStyleValue::Boolean {
                        value: keep_caption,
                    },
                ),
            ],
        }];
        package
    }

    fn machine_figure_caption_measurements(
        first: i64,
        second: i64,
    ) -> Vec<StagingFigureCaptionBlockInput> {
        vec![
            StagingFigureCaptionBlockInput::new(NodeId::new(2), staging_positive(first)),
            StagingFigureCaptionBlockInput::new(NodeId::new(3), staging_positive(second)),
        ]
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn staging_machine_figure_run(
        label: &str,
        package: &wire::WireDocumentPackage,
        image_bytes: &[u8],
        limits: ResourceLimits,
        initial_consumed: i64,
        captions: Vec<StagingFigureCaptionBlockInput>,
        draw_image_ids: Vec<ImageResourceId>,
    ) -> (
        Result<StagingMachineFigureArtifacts, StagingMachineFigureRunnerError>,
        typaxis_diagnostics::MachineDiagnostics,
    ) {
        let package_bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(package)
            .unwrap();
        staging_machine_figure_bytes_run(
            label,
            &package_bytes,
            image_bytes,
            limits,
            initial_consumed,
            captions,
            draw_image_ids,
        )
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn staging_machine_figure_bytes_run(
        label: &str,
        package_bytes: &[u8],
        image_bytes: &[u8],
        limits: ResourceLimits,
        initial_consumed: i64,
        captions: Vec<StagingFigureCaptionBlockInput>,
        draw_image_ids: Vec<ImageResourceId>,
    ) -> (
        Result<StagingMachineFigureArtifacts, StagingMachineFigureRunnerError>,
        typaxis_diagnostics::MachineDiagnostics,
    ) {
        let root = MachineFixtureRoot::new(label);
        let package_path = root.path().join("document-package.json");
        fs::write(&package_path, package_bytes).unwrap();
        fs::write(root.path().join("input.tsf"), []).unwrap();
        fs::write(root.path().join("figure.data"), image_bytes).unwrap();
        let config = config_with_limits(limits);
        let policy =
            PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes()).unwrap();
        let admission = HostAdmissionContext::new(
            HostPath::new(package_path).unwrap(),
            HostPath::new(root.path().to_path_buf()).unwrap(),
            None,
            Vec::new(),
        );
        let initial_consumed =
            NonNegativeLength::new(Length::from_raw(initial_consumed).unwrap()).unwrap();
        let mut budget = MachineDiagnosticBudget::new();
        let result = {
            let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
            exercise_basic_figure_slice(
                package_bytes,
                String::new(),
                &policy,
                &config,
                &admission,
                initial_consumed,
                captions,
                draw_image_ids,
                &mut diagnostics,
            )
        };
        (result, budget.finish())
    }

    const MACHINE_LINK_TEXT: &str = "In External wrapped link";
    const MACHINE_LINK_SOURCE: &str = "In External wrapped link\n";

    fn machine_link_text_inline(node_id: u32, start_byte: u32, end_byte: u32) -> wire::WireInline {
        wire::WireInline::Text {
            node_id,
            span: machine_source_span(),
            text_span: wire::WireTextSpan {
                text_id: 0,
                start_byte,
                end_byte,
            },
        }
    }

    fn machine_link_wire() -> wire::WireDocumentPackage {
        let mut package = machine_wire(MachineFixtureKind::Blank);
        package.sources[0].utf8_byte_length = u32::try_from(MACHINE_LINK_SOURCE.len()).unwrap();
        package.sources[0].sha256 = sha256(MACHINE_LINK_SOURCE.as_bytes());
        package.text_buffers = vec![machine_text_buffer(MACHINE_LINK_TEXT)];
        package.document.blocks = vec![machine_paragraph(
            1,
            vec![
                wire::WireInline::Anchor {
                    node_id: 2,
                    span: machine_source_span(),
                    anchor_id: "target".to_owned(),
                },
                wire::WireInline::Link {
                    node_id: 3,
                    span: machine_source_span(),
                    target: wire::WireLinkTarget::Internal {
                        anchor_id: "target".to_owned(),
                    },
                    children: vec![machine_link_text_inline(4, 0, 3)],
                },
                wire::WireInline::Link {
                    node_id: 5,
                    span: machine_source_span(),
                    target: wire::WireLinkTarget::Uri {
                        uri: "HTTPS://example.test/Path?Q=1".to_owned(),
                    },
                    children: vec![machine_link_text_inline(
                        6,
                        3,
                        u32::try_from(MACHINE_LINK_TEXT.len()).unwrap(),
                    )],
                },
            ],
        )];
        package.style_sheet.rules = vec![machine_style("paragraph")];
        package.page_masters = wire::WirePageMasterSet {
            default_master_id: "default".to_owned(),
            masters: vec![wire::WirePageMaster {
                master_id: "default".to_owned(),
                width: 7_208_960,
                height: 13_107_200,
                body: wire::WireRect {
                    x: 655_360,
                    y: 655_360,
                    width: 5_898_240,
                    height: 11_796_480,
                },
                header: None,
                footer: None,
                footnote: None,
            }],
            selection_rules: Vec::new(),
        };
        package.resources.font_faces = vec![wire::WireFontFace {
            font_face_id: 0,
            family: "Fixture".to_owned(),
            uri: "body.ttf".to_owned(),
            face_index: 0,
            expected_sha256: Some(sha256(&synthetic_ascii_ttf())),
        }];
        package
    }

    fn staging_machine_link_run(
        label: &str,
        package: &wire::WireDocumentPackage,
        limits: ResourceLimits,
        tamper: StagingMachineLinkAnnotationTamper,
    ) -> (
        Result<StagingMachineLinkArtifacts, StagingMachineLinkRunnerError>,
        typaxis_diagnostics::MachineDiagnostics,
    ) {
        let package_bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(package)
            .unwrap();
        staging_machine_link_bytes_run(label, &package_bytes, limits, tamper)
    }

    fn staging_machine_link_bytes_run(
        label: &str,
        package_bytes: &[u8],
        limits: ResourceLimits,
        tamper: StagingMachineLinkAnnotationTamper,
    ) -> (
        Result<StagingMachineLinkArtifacts, StagingMachineLinkRunnerError>,
        typaxis_diagnostics::MachineDiagnostics,
    ) {
        let root = MachineFixtureRoot::new(label);
        let package_path = root.path().join("document-package.json");
        fs::write(&package_path, package_bytes).unwrap();
        fs::write(root.path().join("input.tsf"), MACHINE_LINK_SOURCE).unwrap();
        fs::write(root.path().join("body.ttf"), synthetic_ascii_ttf()).unwrap();
        let config = config_with_limits(limits);
        let policy =
            PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes()).unwrap();
        let admission = HostAdmissionContext::new(
            HostPath::new(package_path).unwrap(),
            HostPath::new(root.path().to_path_buf()).unwrap(),
            None,
            Vec::new(),
        );
        let mut budget = MachineDiagnosticBudget::new();
        let result = {
            let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
            exercise_basic_link_slice(
                package_bytes,
                MACHINE_LINK_SOURCE.to_owned(),
                &policy,
                &config,
                &admission,
                tamper,
                &mut diagnostics,
            )
        };
        (result, budget.finish())
    }

    fn staging_machine_page_break_bytes_run_with_limits(
        package_bytes: &[u8],
        painted_content_owners: Vec<NodeId>,
        limits: &ValidatedResourceLimits,
    ) -> (
        Result<StagingMachinePageBreakArtifacts, StagingMachinePageBreakRunnerError>,
        typaxis_diagnostics::MachineDiagnostics,
    ) {
        let config = config();
        let policy = PackageValidationPolicy::new(limits, config.allowed_uri_schemes()).unwrap();
        let mut budget = MachineDiagnosticBudget::new();
        let result = {
            let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
            exercise_basic_page_break_slice(
                package_bytes,
                String::new(),
                &policy,
                limits,
                painted_content_owners,
                &mut diagnostics,
            )
        };
        (result, budget.finish())
    }

    #[test]
    fn machine_page_break_internal_runner_is_deterministic_and_closes_all_artifacts() {
        let package = include_bytes!(
            "../../../../samples/machine-package/staging/basic-document-1/machine-page-break/job/document-package.json"
        );
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 5,
            ..ResourceLimits::default()
        })
        .unwrap();
        let painted = vec![NodeId::new(2), NodeId::new(5)];
        let (first, diagnostics) =
            staging_machine_page_break_bytes_run_with_limits(package, painted.clone(), &limits);
        let first = first.unwrap();
        assert!(diagnostics.diagnostics().is_empty());
        let (second, _) =
            staging_machine_page_break_bytes_run_with_limits(package, painted, &limits);
        assert_eq!(first, second.unwrap());
        let golden = include_str!(
            "../../../../samples/machine-package/staging/basic-document-1/machine-page-break/staging-selected-state.json"
        );
        assert_eq!(first.manifest_jcs(), golden.trim_end());
        let trace_golden = include_str!(
            "../../../../samples/machine-package/staging/basic-document-1/machine-page-break/staging-trace.json"
        );
        assert_eq!(first.trace_jcs(), trace_golden.trim_end());
        assert_eq!(first.pdf_page_tree_observation(), b"/Count 5\n");
        assert!(first.trace_jcs().contains("\"page_count\":5"));
        assert!(first.display_jcs().contains("\"paint_operations\":[]"));
        assert!(first.pdf_jcs().contains("\"page_count\":5"));
        for artifact in [
            first.trace_jcs(),
            first.display_jcs(),
            first.pdf_jcs(),
            first.manifest_jcs(),
        ] {
            assert!(artifact
                .contains("\"policy_version\":\"typaxis.basic-forced-page-break-policy/1\""));
            assert!(artifact.contains("\"produced_page_index\":4"));
        }
        assert!(first.manifest_jcs().contains(
            "\"pages\":[{\"is_blank\":true,\"page_index\":0,\"painted_content_count\":0},{\"is_blank\":false"
        ));
    }

    #[test]
    fn machine_page_break_internal_runner_enforces_exact_page_limit_and_break_paint_closure() {
        let package = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&machine_page_break_wire())
            .unwrap();
        let exact = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 5,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert!(
            staging_machine_page_break_bytes_run_with_limits(&package, Vec::new(), &exact)
                .0
                .is_ok()
        );

        let below = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 4,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert!(matches!(
            staging_machine_page_break_bytes_run_with_limits(&package, Vec::new(), &below).0,
            Err(StagingMachinePageBreakRunnerError::Pagination(
                StagingForcedPageBreakPaginationError::PageLimit
            ))
        ));
        assert!(matches!(
            staging_machine_page_break_bytes_run_with_limits(
                &package,
                vec![NodeId::new(1)],
                &exact,
            )
            .0,
            Err(StagingMachinePageBreakRunnerError::Pagination(
                StagingForcedPageBreakPaginationError::ForcedBoundaryPaint(owner)
            )) if owner == NodeId::new(1)
        ));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_figure_internal_runner_closes_png_placement_xobject_and_publication() {
        let package = include_bytes!(
            "../../../../samples/machine-package/staging/basic-document-1/machine-figure/job/document-package.json"
        );
        let fixture_hex = include_str!(
            "../../../../samples/machine-package/staging/basic-document-1/machine-figure/job/figure.data.hex"
        )
        .trim();
        let fixture_png: Vec<u8> = fixture_hex
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(fixture_png, MACHINE_FIGURE_PNG);
        let encoded = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&machine_figure_wire(false, sha256(MACHINE_FIGURE_PNG)))
            .unwrap();
        assert_eq!(package.strip_suffix(b"\n").unwrap_or(package), encoded);
        let run = |label| {
            staging_machine_figure_bytes_run(
                label,
                package,
                MACHINE_FIGURE_PNG,
                ResourceLimits::default(),
                50,
                machine_figure_caption_measurements(15, 15),
                vec![ImageResourceId::new(0)],
            )
            .0
            .unwrap()
        };
        let first = run("figure-closed-first");
        let second = run("figure-closed-second");
        assert_eq!(first, second);
        assert_eq!(
            first.manifest_jcs(),
            include_str!(
                "../../../../samples/machine-package/staging/basic-document-1/machine-figure/staging-selected-state.json"
            )
            .trim_end()
        );
        assert_eq!(first.page_count(), 2);
        assert_eq!(first.image_xobject_count(), 2);
        assert!(first.selected_jcs().contains(
            "\"caption_node_id\":2,\"page_index\":1,\"rect\":{\"height\":15,\"width\":80,\"x\":10,\"y\":10}"
        ));
        assert!(first.selected_jcs().contains(
            "\"caption_node_id\":3,\"page_index\":1,\"rect\":{\"height\":15,\"width\":80,\"x\":10,\"y\":25}"
        ));
        assert!(first.selected_jcs().contains(
            "\"figure_node_id\":1,\"image_id\":0,\"keep_policy\":\"allow_caption_split\",\"moved_to_fresh_page\":false"
        ));
        assert!(first.selected_jcs().contains(
            "\"page_index\":0,\"pixel_height\":1,\"pixel_width\":2,\"rect\":{\"height\":20,\"width\":40,\"x\":15,\"y\":60}"
        ));
        assert!(first.display_jcs().contains("\"draw_image_count\":1"));
        assert!(first.pdf_jcs().contains("\"image_xobject_count\":2"));
        assert!(first
            .manifest_jcs()
            .contains("\"alt\":\"opaque-extension PNG\",\"attested_media_kind\":\"png\""));
        assert!(first
            .manifest_jcs()
            .contains("\"image_xobjects\":[{\"image_id\":0,\"resource_name\":\"/Im0\"}]"));
        assert_eq!(sha256(first.pdf_bytes()), first.pdf_sha256());
        assert_eq!(
            first
                .pdf_bytes()
                .windows(b"/Subtype /Image".len())
                .filter(|window| *window == b"/Subtype /Image")
                .count(),
            2
        );

        #[derive(Default)]
        struct RejectPublication {
            accepted: usize,
            limit: usize,
        }
        impl Write for RejectPublication {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if self.accepted == self.limit {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "publication rejected",
                    ));
                }
                let written = bytes.len().min(self.limit - self.accepted);
                self.accepted += written;
                Ok(written)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut rejecting = RejectPublication {
            accepted: 0,
            limit: first.pdf_bytes().len() / 2,
        };
        let error = first.write_pdf(&mut rejecting).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        assert_eq!(rejecting.accepted, first.pdf_bytes().len() / 2);
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_figure_caption_keep_split_and_terminal_oversize_are_typed() {
        let hash = sha256(MACHINE_FIGURE_PNG);
        let split = staging_machine_figure_run(
            "figure-caption-split",
            &machine_figure_wire(false, hash),
            MACHINE_FIGURE_PNG,
            ResourceLimits::default(),
            50,
            machine_figure_caption_measurements(15, 15),
            vec![ImageResourceId::new(0)],
        )
        .0
        .unwrap();
        assert!(split
            .manifest_jcs()
            .contains("\"keep_policy\":\"allow_caption_split\""));
        assert!(split
            .manifest_jcs()
            .contains("\"caption_node_id\":2,\"page_index\":1"));

        let kept = staging_machine_figure_run(
            "figure-caption-kept",
            &machine_figure_wire(true, hash),
            MACHINE_FIGURE_PNG,
            ResourceLimits::default(),
            50,
            machine_figure_caption_measurements(15, 15),
            vec![ImageResourceId::new(0)],
        )
        .0
        .unwrap();
        assert!(kept
            .manifest_jcs()
            .contains("\"keep_policy\":\"keep_image_and_caption\""));
        assert!(kept.manifest_jcs().contains("\"moved_to_fresh_page\":true"));
        assert!(kept
            .manifest_jcs()
            .contains("\"oversize_policy\":\"terminal_once\",\"page_index\":1"));
        assert!(kept
            .manifest_jcs()
            .contains("\"caption_node_id\":2,\"page_index\":1"));

        let keep_oversize = staging_machine_figure_run(
            "figure-keep-oversize",
            &machine_figure_wire(true, hash),
            MACHINE_FIGURE_PNG,
            ResourceLimits::default(),
            0,
            machine_figure_caption_measurements(51, 15),
            vec![ImageResourceId::new(0)],
        )
        .0;
        assert!(matches!(
            keep_oversize,
            Err(StagingMachineFigureRunnerError::Pagination(
                StagingMachineFigurePaginationError::KeepOversize(owner)
            )) if owner == NodeId::new(1)
        ));

        let caption_oversize = staging_machine_figure_run(
            "figure-caption-oversize",
            &machine_figure_wire(false, hash),
            MACHINE_FIGURE_PNG,
            ResourceLimits::default(),
            0,
            machine_figure_caption_measurements(71, 15),
            vec![ImageResourceId::new(0)],
        )
        .0;
        assert!(matches!(
            caption_oversize,
            Err(StagingMachineFigureRunnerError::Pagination(
                StagingMachineFigurePaginationError::CaptionOversize(owner)
            )) if owner == NodeId::new(2)
        ));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_figure_admission_and_draw_closure_reject_every_tamper_class() {
        let valid_hash = sha256(MACHINE_FIGURE_PNG);
        let run = |label: &str,
                   package: wire::WireDocumentPackage,
                   bytes: &[u8],
                   limits: ResourceLimits,
                   ids: Vec<ImageResourceId>| {
            staging_machine_figure_run(
                label,
                &package,
                bytes,
                limits,
                0,
                machine_figure_caption_measurements(15, 15),
                ids,
            )
            .0
        };

        assert!(matches!(
            run(
                "figure-bad-hash",
                machine_figure_wire(false, [0; 32]),
                MACHINE_FIGURE_PNG,
                ResourceLimits::default(),
                vec![ImageResourceId::new(0)],
            ),
            Err(StagingMachineFigureRunnerError::ResourceAdmission(
                ResourceAdmissionError::ExpectedHashMismatch
            ))
        ));

        let non_png = b"this is not a PNG";
        assert!(matches!(
            run(
                "figure-non-png",
                machine_figure_wire(false, sha256(non_png)),
                non_png,
                ResourceLimits::default(),
                vec![ImageResourceId::new(0)],
            ),
            Err(StagingMachineFigureRunnerError::ResourceAdmission(
                ResourceAdmissionError::InvalidMetadata
            ))
        ));

        let mut invalid_dimensions = MACHINE_FIGURE_PNG.to_vec();
        invalid_dimensions[16..20].copy_from_slice(&0u32.to_be_bytes());
        assert!(matches!(
            run(
                "figure-invalid-dimensions",
                machine_figure_wire(false, sha256(&invalid_dimensions)),
                &invalid_dimensions,
                ResourceLimits::default(),
                vec![ImageResourceId::new(0)],
            ),
            Err(StagingMachineFigureRunnerError::ResourceAdmission(
                ResourceAdmissionError::InvalidMetadata
            ))
        ));

        assert!(matches!(
            run(
                "figure-pixel-limit",
                machine_figure_wire(false, valid_hash),
                MACHINE_FIGURE_PNG,
                ResourceLimits {
                    max_image_pixels: 1,
                    ..ResourceLimits::default()
                },
                vec![ImageResourceId::new(0)],
            ),
            Err(StagingMachineFigureRunnerError::ResourceAdmission(
                ResourceAdmissionError::ResourceLimit
            ))
        ));

        assert!(matches!(
            run(
                "figure-missing-draw",
                machine_figure_wire(false, valid_hash),
                MACHINE_FIGURE_PNG,
                ResourceLimits::default(),
                vec![],
            ),
            Err(StagingMachineFigureRunnerError::Display(
                StagingMachineFigureDisplayError::MissingDrawImage(image_id)
            )) if image_id == ImageResourceId::new(0)
        ));

        assert!(matches!(
            run(
                "figure-extra-draw",
                machine_figure_wire(false, valid_hash),
                MACHINE_FIGURE_PNG,
                ResourceLimits::default(),
                vec![ImageResourceId::new(0), ImageResourceId::new(1)],
            ),
            Err(StagingMachineFigureRunnerError::Display(
                StagingMachineFigureDisplayError::ExtraDrawImage(image_id)
            )) if image_id == ImageResourceId::new(1)
        ));

        assert!(matches!(
            run(
                "figure-wrong-id",
                machine_figure_wire(false, valid_hash),
                MACHINE_FIGURE_PNG,
                ResourceLimits::default(),
                vec![ImageResourceId::new(1)],
            ),
            Err(StagingMachineFigureRunnerError::Display(
                StagingMachineFigureDisplayError::WrongDrawImage { expected, actual }
            )) if expected == ImageResourceId::new(0) && actual == ImageResourceId::new(1)
        ));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_link_internal_runner_closes_wrapped_internal_and_external_annotations() {
        let package = machine_link_wire();
        let encoded = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&package)
            .unwrap();
        let checked_package = include_bytes!(
            "../../../../samples/machine-package/staging/basic-document-1/machine-link/job/document-package.json"
        );
        assert_eq!(
            checked_package
                .strip_suffix(b"\n")
                .unwrap_or(checked_package),
            encoded
        );
        let fixture_font_hex = include_str!(
            "../../../../samples/machine-package/staging/basic-document-1/machine-link/job/body.ttf.hex"
        )
        .trim();
        let fixture_font: Vec<u8> = fixture_font_hex
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(fixture_font, synthetic_ascii_ttf());
        let run = |label| {
            staging_machine_link_bytes_run(
                label,
                checked_package,
                ResourceLimits::default(),
                StagingMachineLinkAnnotationTamper::None,
            )
            .0
            .unwrap()
        };
        let first = run("link-closed-first");
        let second = run("link-closed-second");
        assert_eq!(first, second);
        assert_eq!(
            first.manifest_jcs(),
            include_str!(
                "../../../../samples/machine-package/staging/basic-document-1/machine-link/staging-selected-state.json"
            )
            .trim_end()
        );
        assert_eq!(sha256(first.pdf_bytes()), first.pdf_sha256());
        assert_eq!(first.destination_count(), 1);
        assert!(first.annotation_count() >= 3);
        assert_eq!(
            first
                .pdf_bytes()
                .windows(b"/Subtype /Link".len())
                .filter(|window| *window == b"/Subtype /Link")
                .count(),
            first.annotation_count() as usize
        );
        assert!(first
            .pdf_bytes()
            .windows(b"/Dest <746172676574>".len())
            .any(|window| window == b"/Dest <746172676574>"));
        assert!(first
            .pdf_bytes()
            .windows(b"/URI <68747470733A2F2F6578616D706C652E746573742F506174683F513D31>".len(),)
            .any(|window| {
                window == b"/URI <68747470733A2F2F6578616D706C652E746573742F506174683F513D31>"
            }));
        assert!(first.manifest_jcs().contains("\"anchor_id\":\"target\""));
        assert!(first.manifest_jcs().contains("\"kind\":\"internal\""));
        assert!(first
            .manifest_jcs()
            .contains("\"kind\":\"external\",\"uri\":\"https://example.test/Path?Q=1\""));
        assert!(first.cluster_jcs().contains("\"link_node_id\":3"));
        assert!(first.display_jcs().contains("\"line_ordinal\":"));
        assert!(first.pdf_jcs().contains("\"object_id\":"));
        assert!(first.page_count() >= 1);
        assert!(first.object_count() > first.annotation_count());
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_link_rejects_empty_unpainted_bad_uri_and_bad_targets_before_layout() {
        let mut empty = machine_link_wire();
        let wire::WireBlock::Paragraph {
            children: paragraph_children,
            ..
        } = &mut empty.document.blocks[0]
        else {
            unreachable!()
        };
        let wire::WireInline::Link {
            children: link_children,
            ..
        } = &mut paragraph_children[1]
        else {
            unreachable!()
        };
        link_children.clear();
        let wire::WireInline::Link {
            node_id,
            children: external_children,
            ..
        } = &mut paragraph_children[2]
        else {
            unreachable!()
        };
        *node_id = 4;
        let wire::WireInline::Text { node_id, .. } = &mut external_children[0] else {
            unreachable!()
        };
        *node_id = 5;
        let empty_result = staging_machine_link_run(
            "link-empty",
            &empty,
            ResourceLimits::default(),
            StagingMachineLinkAnnotationTamper::None,
        )
        .0;
        assert!(matches!(
            empty_result,
            Err(StagingMachineLinkRunnerError::LinkPreflight(
                typaxis_syntax::StagingLinkPreflightError::EmptyChildren(owner)
            )) if owner == NodeId::new(3)
        ));

        let mut unpainted = machine_link_wire();
        let wire::WireBlock::Paragraph { children, .. } = &mut unpainted.document.blocks[0] else {
            unreachable!()
        };
        let wire::WireInline::Link { children, .. } = &mut children[1] else {
            unreachable!()
        };
        children[0] = wire::WireInline::Anchor {
            node_id: 4,
            span: machine_source_span(),
            anchor_id: "unpainted".to_owned(),
        };
        assert!(matches!(
            staging_machine_link_run(
                "link-unpainted",
                &unpainted,
                ResourceLimits::default(),
                StagingMachineLinkAnnotationTamper::None,
            )
            .0,
            Err(StagingMachineLinkRunnerError::LinkPreflight(
                typaxis_syntax::StagingLinkPreflightError::UnpaintedChildren(owner)
            )) if owner == NodeId::new(3)
        ));

        let mut bad_uri = machine_link_wire();
        let wire::WireBlock::Paragraph { children, .. } = &mut bad_uri.document.blocks[0] else {
            unreachable!()
        };
        let wire::WireInline::Link { target, .. } = &mut children[2] else {
            unreachable!()
        };
        *target = wire::WireLinkTarget::Uri {
            uri: "javascript:alert(1)".to_owned(),
        };
        assert!(matches!(
            staging_machine_link_run(
                "link-bad-uri",
                &bad_uri,
                ResourceLimits::default(),
                StagingMachineLinkAnnotationTamper::None,
            )
            .0,
            Err(StagingMachineLinkRunnerError::Syntax(_))
        ));

        let mut bad_target = machine_link_wire();
        let wire::WireBlock::Paragraph { children, .. } = &mut bad_target.document.blocks[0] else {
            unreachable!()
        };
        let wire::WireInline::Link { target, .. } = &mut children[1] else {
            unreachable!()
        };
        *target = wire::WireLinkTarget::Internal {
            anchor_id: "missing".to_owned(),
        };
        assert!(matches!(
            staging_machine_link_run(
                "link-bad-target",
                &bad_target,
                ResourceLimits::default(),
                StagingMachineLinkAnnotationTamper::None,
            )
            .0,
            Err(StagingMachineLinkRunnerError::Syntax(_))
                | Err(StagingMachineLinkRunnerError::LinkPreflight(
                    typaxis_syntax::StagingLinkPreflightError::UnknownInternalTarget(_)
                ))
        ));

        let mut duplicate_anchor = machine_link_wire();
        let wire::WireBlock::Paragraph { children, .. } = &mut duplicate_anchor.document.blocks[0]
        else {
            unreachable!()
        };
        children[2] = wire::WireInline::Anchor {
            node_id: 5,
            span: machine_source_span(),
            anchor_id: "target".to_owned(),
        };
        assert!(matches!(
            staging_machine_link_run(
                "link-duplicate-anchor",
                &duplicate_anchor,
                ResourceLimits::default(),
                StagingMachineLinkAnnotationTamper::None,
            )
            .0,
            Err(StagingMachineLinkRunnerError::Syntax(_))
        ));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_link_rejects_annotation_tamper_and_enforces_exact_limits() {
        let package = machine_link_wire();
        let valid = staging_machine_link_run(
            "link-limit-baseline",
            &package,
            ResourceLimits::default(),
            StagingMachineLinkAnnotationTamper::None,
        )
        .0
        .unwrap();
        let rectangle_count = u64::from(valid.annotation_count());
        let object_count = valid.object_count();

        assert!(staging_machine_link_run(
            "link-rectangle-exact",
            &package,
            ResourceLimits {
                max_fragments: rectangle_count,
                ..ResourceLimits::default()
            },
            StagingMachineLinkAnnotationTamper::None,
        )
        .0
        .is_ok());
        let below_result = staging_machine_link_run(
            "link-rectangle-below",
            &package,
            ResourceLimits {
                max_fragments: rectangle_count - 1,
                ..ResourceLimits::default()
            },
            StagingMachineLinkAnnotationTamper::None,
        )
        .0;
        assert!(matches!(
            below_result,
            Err(StagingMachineLinkRunnerError::Display(
                StagingMachineLinkDisplayError::RectangleLimit
            ))
        ));
        assert!(staging_machine_link_run(
            "link-object-exact",
            &package,
            ResourceLimits {
                max_pdf_objects: object_count,
                ..ResourceLimits::default()
            },
            StagingMachineLinkAnnotationTamper::None,
        )
        .0
        .is_ok());
        assert!(matches!(
            staging_machine_link_run(
                "link-object-below",
                &package,
                ResourceLimits {
                    max_pdf_objects: object_count - 1,
                    ..ResourceLimits::default()
                },
                StagingMachineLinkAnnotationTamper::None,
            )
            .0,
            Err(StagingMachineLinkRunnerError::Pdf(
                typaxis_pdf::PdfError::ObjectLimit
            ))
        ));

        for (tamper, expected) in [
            (StagingMachineLinkAnnotationTamper::MissingFirst, "missing"),
            (StagingMachineLinkAnnotationTamper::ExtraFirst, "extra"),
            (StagingMachineLinkAnnotationTamper::WrongPageFirst, "page"),
            (
                StagingMachineLinkAnnotationTamper::WrongTargetFirst,
                "target",
            ),
            (
                StagingMachineLinkAnnotationTamper::RectangleFirst,
                "rectangle",
            ),
        ] {
            let error =
                staging_machine_link_run(expected, &package, ResourceLimits::default(), tamper)
                    .0
                    .unwrap_err();
            assert!(
                matches!(
                    (expected, error),
                    (
                        "missing",
                        StagingMachineLinkRunnerError::Display(
                            StagingMachineLinkDisplayError::MissingAnnotation(_)
                        )
                    ) | (
                        "extra",
                        StagingMachineLinkRunnerError::Display(
                            StagingMachineLinkDisplayError::ExtraAnnotation(_)
                        )
                    ) | (
                        "page",
                        StagingMachineLinkRunnerError::Display(
                            StagingMachineLinkDisplayError::WrongPage(_)
                        )
                    ) | (
                        "target",
                        StagingMachineLinkRunnerError::Display(
                            StagingMachineLinkDisplayError::WrongTarget(_)
                        )
                    ) | (
                        "rectangle",
                        StagingMachineLinkRunnerError::Display(
                            StagingMachineLinkDisplayError::RectangleMismatch(_)
                        )
                    )
                ),
                "unexpected {expected} closure error"
            );
        }
    }

    fn machine_list_paint_inputs(
        empty_first: bool,
    ) -> Vec<typaxis_layout::StagingListItemPaintInput> {
        let painted = |owner, marker_width, line_width, line_height, total_height| {
            typaxis_layout::StagingListItemPaintInput::painted(
                NodeId::new(owner),
                staging_positive(marker_width),
                staging_positive(line_width),
                staging_positive(line_height),
                staging_positive(total_height),
            )
        };
        vec![
            if empty_first {
                typaxis_layout::StagingListItemPaintInput::empty(
                    NodeId::new(2),
                    staging_positive(4),
                )
            } else {
                painted(2, 4, 20, 8, 18)
            },
            painted(5, 6, 18, 8, 12),
            painted(7, 8, 24, 8, 16),
        ]
    }

    fn staging_machine_list_run(
        package: &wire::WireDocumentPackage,
        items: Vec<typaxis_layout::StagingListItemPaintInput>,
    ) -> (
        Result<StagingMachineListArtifacts, StagingMachineListRunnerError>,
        typaxis_diagnostics::MachineDiagnostics,
    ) {
        let bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(package)
            .unwrap();
        staging_machine_list_bytes_run(&bytes, items)
    }

    fn staging_machine_list_bytes_run(
        package_bytes: &[u8],
        items: Vec<typaxis_layout::StagingListItemPaintInput>,
    ) -> (
        Result<StagingMachineListArtifacts, StagingMachineListRunnerError>,
        typaxis_diagnostics::MachineDiagnostics,
    ) {
        let config = config();
        staging_machine_list_bytes_run_with_limits(package_bytes, items, config.limits())
    }

    fn staging_machine_list_bytes_run_with_limits(
        package_bytes: &[u8],
        items: Vec<typaxis_layout::StagingListItemPaintInput>,
        limits: &ValidatedResourceLimits,
    ) -> (
        Result<StagingMachineListArtifacts, StagingMachineListRunnerError>,
        typaxis_diagnostics::MachineDiagnostics,
    ) {
        let config = config();
        let policy = PackageValidationPolicy::new(limits, config.allowed_uri_schemes()).unwrap();
        let mut budget = MachineDiagnosticBudget::new();
        let result = {
            let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
            exercise_basic_list_slice(
                package_bytes,
                String::new(),
                &policy,
                limits,
                StagingMachineListLayoutInput::new(staging_positive(100), BidiLevel::LTR, items),
                StagingMachineListPageInput::new(staging_positive(20), staging_positive(5))
                    .unwrap(),
                &mut diagnostics,
            )
        };
        (result, budget.finish())
    }

    #[test]
    fn machine_list_internal_runner_is_deterministic_across_nested_page_split_artifacts() {
        let package = include_bytes!(
            "../../../../samples/machine-package/staging/basic-document-1/machine-list/job/document-package.json"
        );
        let (first, diagnostics) =
            staging_machine_list_bytes_run(package, machine_list_paint_inputs(false));
        let first = first.unwrap();
        assert!(diagnostics.diagnostics().is_empty());
        let (second, _) = staging_machine_list_bytes_run(package, machine_list_paint_inputs(false));
        assert_eq!(first, second.unwrap());
        let manifest_golden = include_str!(
            "../../../../samples/machine-package/staging/basic-document-1/machine-list/staging-selected-state.json"
        );
        assert_eq!(first.manifest_jcs(), manifest_golden.trim_end());
        assert!(first.trace_jcs().contains("\"item_flow_id\":2"));
        assert!(first.trace_jcs().contains("\"list_flow_id\":1"));
        assert!(first.display_jcs().contains("\"marker_utf8\":\"•\""));
        assert!(first.pdf_jcs().contains("\"marker_utf8\":\"10.\""));
        assert!(std::str::from_utf8(first.pdf_content_observation())
            .unwrap()
            .contains("<e280a2> Tj"));
        assert!(first
            .manifest_jcs()
            .contains("\"first_line_fragment_id\":0"));
        for artifact in [first.display_jcs(), first.pdf_jcs(), first.manifest_jcs()] {
            assert!(artifact.contains("\"marker_fragment_id\":0"));
            assert!(artifact.contains("\"item_flow_id\":1"));
            assert!(artifact.contains("\"page_index\":1"));
        }
    }

    #[test]
    fn machine_list_internal_runner_rejects_empty_overflow_and_wrong_item_tamper() {
        let package = machine_list_wire(9);
        let (empty, _) = staging_machine_list_run(&package, machine_list_paint_inputs(true));
        assert!(matches!(
            empty,
            Err(StagingMachineListRunnerError::Layout(
                StagingMachineListLayoutError::EmptyPaintedItem(owner)
            )) if owner == NodeId::new(2)
        ));

        let overflow = machine_list_wire(u32::MAX);
        let (overflow, diagnostics) =
            staging_machine_list_run(&overflow, machine_list_paint_inputs(false));
        assert!(matches!(
            overflow,
            Err(StagingMachineListRunnerError::ListPreflight(
                BasicDocumentListPreflightFailure::MarkerOverflow { list_owner }
            )) if list_owner == NodeId::new(1)
        ));
        assert_eq!(*diagnostics.diagnostics()[0].code(), L5100);

        let mut tampered = machine_list_paint_inputs(false);
        tampered[1] = typaxis_layout::StagingListItemPaintInput::painted(
            NodeId::new(99),
            staging_positive(6),
            staging_positive(18),
            staging_positive(8),
            staging_positive(12),
        );
        let (tampered, _) = staging_machine_list_run(&package, tampered);
        assert!(matches!(
            tampered,
            Err(StagingMachineListRunnerError::Layout(
                StagingMachineListLayoutError::ExtraMeasurement(owner)
            )) if owner == NodeId::new(99)
        ));
    }

    #[test]
    fn machine_list_internal_runner_closes_single_exact_and_max_plus_one_marker_limits() {
        let mut package = machine_list_wire(1);
        let wire::WireBlock::List {
            ordered,
            start,
            items,
            ..
        } = &mut package.document.blocks[0]
        else {
            panic!("machine-list fixture root must remain a list");
        };
        *ordered = false;
        *start = None;
        items.truncate(1);
        items[0].blocks.truncate(1);
        let bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&package)
            .unwrap();
        let measurements = vec![typaxis_layout::StagingListItemPaintInput::painted(
            NodeId::new(2),
            staging_positive(6),
            staging_positive(20),
            staging_positive(8),
            staging_positive(12),
        )];
        let exact = ValidatedResourceLimits::new(ResourceLimits {
            max_text_buffer_bytes: 3,
            max_text_bytes: 3,
            max_shaping_context_bytes: 3,
            ..ResourceLimits::default()
        })
        .unwrap();
        let (single, diagnostics) =
            staging_machine_list_bytes_run_with_limits(&bytes, measurements.clone(), &exact);
        let single = single.unwrap();
        assert!(diagnostics.diagnostics().is_empty());
        assert!(single.display_jcs().contains("\"marker_utf8\":\"•\""));

        let max_plus_one = ValidatedResourceLimits::new(ResourceLimits {
            max_text_buffer_bytes: 2,
            max_text_bytes: 3,
            max_shaping_context_bytes: 2,
            ..ResourceLimits::default()
        })
        .unwrap();
        let (rejected, diagnostics) =
            staging_machine_list_bytes_run_with_limits(&bytes, measurements, &max_plus_one);
        assert!(matches!(
            rejected,
            Err(StagingMachineListRunnerError::ListPreflight(
                BasicDocumentListPreflightFailure::TextBufferLimit { item_owner }
            )) if item_owner == NodeId::new(2)
        ));
        assert_eq!(*diagnostics.diagnostics()[0].code(), T2100);
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_block_styles_internal_runner_closes_wire_display_pdf_and_manifest() {
        let mut paragraph = machine_wire(MachineFixtureKind::Blank);
        paragraph.document.blocks = vec![machine_paragraph(1, vec![])];
        paragraph.style_sheet.rules = vec![wire::WireStyleRule {
            style_id: "typed-paragraph".to_owned(),
            extends: None,
            selector: "paragraph".to_owned(),
            source_order: 0,
            declarations: vec![
                staging_style_declaration(
                    wire::WireDeclarationName::SpaceBefore,
                    wire::WireStyleValue::Length { value: 5 },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::SpaceAfter,
                    wire::WireStyleValue::Length { value: 6 },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::StartIndent,
                    wire::WireStyleValue::Length { value: 10 },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::EndIndent,
                    wire::WireStyleValue::Length { value: 10 },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::TextAlign,
                    wire::WireStyleValue::Keyword {
                        value: "center".to_owned(),
                    },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::KeepWithNext,
                    wire::WireStyleValue::Boolean { value: true },
                ),
            ],
        }];
        let input = TypedBlockLayoutInput::new(
            staging_positive(101),
            staging_positive(20),
            staging_positive(20),
            staging_positive(25),
            staging_positive(100),
            NonNegativeLength::new(Length::from_raw(7).unwrap()).unwrap(),
            false,
            false,
            BidiLevel::LTR,
        );
        let (first, diagnostics) = staging_style_run(&paragraph, input);
        let first = first.unwrap();
        assert!(diagnostics.diagnostics().is_empty());
        let (second, _) = staging_style_run(&paragraph, input);
        assert_eq!(first, second.unwrap());
        assert_eq!(first.pdf_content_observation(), b"q\n40 0 20 1 re W n\nQ\n");
        for observation in [first.display_jcs(), first.pdf_jcs(), first.manifest_jcs()] {
            assert!(observation.contains("\"effective_space_before\":0"));
            assert!(observation.contains("\"effective_space_after\":6"));
            assert!(observation.contains("\"start_indent\":10"));
            assert!(observation.contains("\"end_indent\":10"));
            assert!(observation.contains("\"logical_start_alignment_space\":30"));
            assert!(observation.contains("\"logical_end_alignment_space\":31"));
            assert!(observation.contains("\"keep_with_next\":true"));
            assert!(observation.contains("\"page_break_before\":true"));
        }

        let mut figure = machine_wire(MachineFixtureKind::Blank);
        figure.document.blocks = vec![wire::WireBlock::Figure {
            node_id: 1,
            span: machine_source_span(),
            classes: vec![],
            image_id: 0,
            alt: "fixture".to_owned(),
            caption: vec![],
        }];
        figure.resources.images = vec![wire::WireImage {
            image_id: 0,
            uri: "fixture.png".to_owned(),
            expected_sha256: None,
        }];
        figure.style_sheet.rules = vec![wire::WireStyleRule {
            style_id: "typed-figure".to_owned(),
            extends: None,
            selector: "figure".to_owned(),
            source_order: 0,
            declarations: vec![
                staging_style_declaration(
                    wire::WireDeclarationName::Width,
                    wire::WireStyleValue::Length { value: 30 },
                ),
                staging_style_declaration(
                    wire::WireDeclarationName::KeepCaption,
                    wire::WireStyleValue::Boolean { value: false },
                ),
            ],
        }];
        let (figure, diagnostics) = staging_style_run(
            &figure,
            TypedBlockLayoutInput::new(
                staging_positive(100),
                staging_positive(99),
                staging_positive(20),
                staging_positive(100),
                staging_positive(100),
                NonNegativeLength::ZERO,
                true,
                true,
                BidiLevel::LTR,
            ),
        );
        let figure = figure.unwrap();
        assert!(diagnostics.diagnostics().is_empty());
        assert!(figure.pdf_jcs().contains("\"paint_inline_size\":30"));
        assert!(figure.manifest_jcs().contains("\"keep_caption\":false"));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_block_styles_unsupported_selector_fails_preflight_before_layout() {
        let mut package = machine_wire(MachineFixtureKind::Blank);
        package.document.blocks = vec![wire::WireBlock::PageBreak {
            node_id: 1,
            span: machine_source_span(),
            classes: vec![],
        }];
        package.style_sheet.rules = vec![wire::WireStyleRule {
            style_id: "invalid-break".to_owned(),
            extends: None,
            selector: "page_break".to_owned(),
            source_order: 0,
            declarations: vec![staging_style_declaration(
                wire::WireDeclarationName::SpaceBefore,
                wire::WireStyleValue::Length { value: 1 },
            )],
        }];
        let (result, diagnostics) = staging_style_run(
            &package,
            TypedBlockLayoutInput::new(
                staging_positive(100),
                staging_positive(10),
                staging_positive(10),
                staging_positive(100),
                staging_positive(100),
                NonNegativeLength::ZERO,
                true,
                true,
                BidiLevel::LTR,
            ),
        );
        assert!(matches!(
            result,
            Err(StagingMachineBlockStyleRunnerError::Preflight(
                BasicDocumentStylePreflightFailure::Unsupported {
                    violation_count: 1,
                    primary_code: L5101,
                }
            ))
        ));
        assert_eq!(diagnostics.diagnostics().len(), 1);
        assert_eq!(*diagnostics.diagnostics()[0].code(), L5101);
    }
}
