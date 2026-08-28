use std::io::Write;
use typaxis_core::{
    push_jcs_string, LayoutStateFingerprint, Rect, ValidatedResourceLimits, CONTRACT,
    COORDINATE_UNIT,
};
use typaxis_document_package::{CanonicalJcsStats, DocumentPackageEncoder, JcsEncodeError};
use typaxis_layout::{FlowPosition, FlowTree, LayoutEpoch};
use typaxis_manifest::{StagingFootnoteLayoutFacts, StagingTableLayoutFacts};
use typaxis_pagination::{
    ConvergenceStatus, InitialPaginationState, PageFrameKind, PagePlan, PaginationResult,
    PlacedAnchor, ResolvedReference,
};
use typaxis_syntax::{DocumentPackageConversionError, ValidatedParsedPackage};
use typaxis_text::GeneratedTextStore;

#[cfg(test)]
pub(crate) fn staging_m4_document_package_from_attested_media(
    package: &typaxis_syntax::ValidatedStagingSemanticPackage,
    media: &typaxis_resources::StagingDeclaredMediaLedger,
) -> Result<String, String> {
    use typaxis_document::ImageMediaType;
    use typaxis_document_package::{
        StagingSemanticDocumentPackageEncoder, WireFontMediaType, WireImageMediaType,
    };
    use typaxis_resources::{AdmittedFontMediaKind, AdmittedImageMediaKind};

    let mut wire = package
        .checked_wire()
        .map_err(|error| error.to_string())?
        .clone();
    let mut resources = wire.resources().clone();
    if resources.font_faces.len() != media.fonts().len()
        || resources.images.len() != media.images().len()
    {
        return Err("stable media attestation set is incomplete".to_owned());
    }
    for (declaration, attestation) in resources.font_faces.iter_mut().zip(media.fonts()) {
        if declaration.font_face_id != attestation.font_face_id().get()
            || declaration.uri != attestation.uri().as_str()
            || declaration.family != attestation.family()
            || declaration.face_index != attestation.face_index()
            || declaration
                .expected_sha256
                .as_deref()
                .is_some_and(|value| value != hex_hash(attestation.content_hash()))
        {
            return Err("stable font attestation identity mismatch".to_owned());
        }
        declaration.media_type = match attestation.attested() {
            AdmittedFontMediaKind::SfntTrueTypeGlyf => WireFontMediaType::SfntTrueTypeGlyf,
            AdmittedFontMediaKind::TtcTrueTypeGlyf => WireFontMediaType::TtcTrueTypeGlyf,
        };
    }
    for (declaration, attestation) in resources.images.iter_mut().zip(media.images()) {
        if declaration.image_id != attestation.image_id().get()
            || declaration.uri != attestation.uri().as_str()
            || !matches!(
                (declaration.media_type, attestation.declared()),
                (WireImageMediaType::Png, ImageMediaType::Png)
                    | (WireImageMediaType::SvgSafe1, ImageMediaType::SvgSafe1)
            )
            || declaration
                .expected_sha256
                .as_deref()
                .is_some_and(|value| value != hex_hash(attestation.content_hash()))
        {
            return Err("stable image attestation identity mismatch".to_owned());
        }
        declaration.media_type = match attestation.attested() {
            AdmittedImageMediaKind::Png => WireImageMediaType::Png,
            AdmittedImageMediaKind::SafeVector => WireImageMediaType::SvgSafe1,
        };
    }
    let document = wire.document().clone();
    wire.replace_typed_regions(document, resources);
    StagingSemanticDocumentPackageEncoder::new()
        .encode(&wire)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
fn hex_hash(value: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

pub const GENERATED_TRACE_TEXT_REQUIRES_OPT_IN: &str =
    "generated text requires `--trace-text` for a complete trace";

#[derive(Debug)]
pub enum DocumentPackageArtifactError {
    Conversion(DocumentPackageConversionError),
    Encoding(JcsEncodeError),
}

impl std::fmt::Display for DocumentPackageArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conversion(error) => error.fmt(formatter),
            Self::Encoding(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for DocumentPackageArtifactError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Conversion(error) => Some(error),
            Self::Encoding(error) => Some(error),
        }
    }
}

pub fn write_document_package_json<W: Write>(
    package: &ValidatedParsedPackage,
    limits: &ValidatedResourceLimits,
    output: &mut W,
) -> Result<CanonicalJcsStats, DocumentPackageArtifactError> {
    let wire_package = package
        .to_wire_document_package()
        .map_err(DocumentPackageArtifactError::Conversion)?;
    DocumentPackageEncoder::new(limits.get().max_document_package_bytes)
        .map_err(DocumentPackageArtifactError::Encoding)?
        .write_preflighted(&wire_package, output)
        .map_err(DocumentPackageArtifactError::Encoding)
}

#[cfg(test)]
fn document_package_json(
    package: &ValidatedParsedPackage,
    limits: &ValidatedResourceLimits,
) -> Result<String, String> {
    let mut bytes = Vec::new();
    write_document_package_json(package, limits, &mut bytes).map_err(|error| error.to_string())?;
    String::from_utf8(bytes).map_err(|error| error.to_string())
}

pub fn reference_layout_page_json(
    page: &PagePlan,
    width: i64,
    height: i64,
) -> Result<String, &'static str> {
    ensure_reference_page_shape(page)?;
    let mut json = String::from("{\"contract\":");
    push_jcs_string(&mut json, CONTRACT);
    json.push_str(",\"coordinate_unit\":");
    push_jcs_string(&mut json, COORDINATE_UNIT);
    json.push_str(",\"page\":{\"fragments\":[");
    push_fragments(&mut json, &page.fragments);
    json.push_str("],\"frames\":[");
    push_frames(&mut json, &page.frames);
    json.push_str("],\"height\":");
    json.push_str(&height.to_string());
    json.push_str(",\"master_id\":");
    push_jcs_string(&mut json, page.master_id.as_str());
    json.push_str(",\"page_index\":");
    json.push_str(&page.page_index.to_string());
    json.push_str(",\"width\":");
    json.push_str(&width.to_string());
    json.push_str("}}");
    Ok(json)
}

pub fn reference_layout_trace_json(
    flow: &FlowTree,
    initial: &InitialPaginationState,
    pagination: &PaginationResult,
    max_layout_passes: u16,
    include_trace_text: bool,
) -> Result<String, &'static str> {
    layout_trace_json(
        flow,
        initial,
        pagination,
        max_layout_passes,
        include_trace_text,
        LayoutTraceProfileProjection {
            machine_binding: None,
            table_layouts: None,
            footnote_layout: None,
        },
    )
}

pub struct MachineTraceBinding<'a> {
    capability: &'a typaxis_machine_profile::MachinePdfPreflightReceipt,
    flow_registry_sha256: Option<[u8; 32]>,
    table_layouts: &'a [StagingTableLayoutFacts],
    footnote_layout: Option<&'a StagingFootnoteLayoutFacts>,
}

impl<'a> MachineTraceBinding<'a> {
    pub const fn new(
        capability: &'a typaxis_machine_profile::MachinePdfPreflightReceipt,
        flow_registry_sha256: Option<[u8; 32]>,
        table_layouts: &'a [StagingTableLayoutFacts],
        footnote_layout: Option<&'a StagingFootnoteLayoutFacts>,
    ) -> Self {
        Self {
            capability,
            flow_registry_sha256,
            table_layouts,
            footnote_layout,
        }
    }
}

pub fn machine_layout_trace_json(
    flow: &FlowTree,
    initial: &InitialPaginationState,
    pagination: &PaginationResult,
    max_layout_passes: u16,
    include_trace_text: bool,
    binding: MachineTraceBinding<'_>,
) -> Result<String, &'static str> {
    let table_layouts = (binding.capability.profile()
        == typaxis_core::MachinePdfProfileId::TABLE_1)
        .then_some(binding.table_layouts);
    let footnote_layout = match (binding.capability.profile(), binding.footnote_layout) {
        (typaxis_core::MachinePdfProfileId::Footnote1, Some(facts)) => Some(facts),
        (typaxis_core::MachinePdfProfileId::Footnote1, None) => {
            return Err("footnote trace facts are required for the footnote profile")
        }
        (_, Some(_)) => return Err("footnote trace facts require the footnote profile"),
        (_, None) => None,
    };
    layout_trace_json(
        flow,
        initial,
        pagination,
        max_layout_passes,
        include_trace_text,
        LayoutTraceProfileProjection {
            machine_binding: Some((
                binding.capability.profile_receipt_sha256(),
                binding.flow_registry_sha256,
            )),
            table_layouts,
            footnote_layout,
        },
    )
}

pub fn advanced_machine_layout_trace_json(
    manifest: &typaxis_manifest::StagingAdvancedPaginationManifest,
) -> String {
    let mut json = String::from("{\"advanced_pagination\":");
    json.push_str(manifest.canonical_jcs());
    json.push_str(",\"contract\":");
    push_jcs_string(&mut json, CONTRACT);
    json.push_str(",\"coordinate_unit\":");
    push_jcs_string(&mut json, COORDINATE_UNIT);
    json.push_str(",\"flow_registry_sha256\":");
    push_hex(&mut json, manifest.flow_registry_sha256());
    json.push_str(",\"profile_receipt_sha256\":");
    push_hex(&mut json, manifest.profile_receipt_sha256());
    json.push('}');
    json
}

struct LayoutTraceProfileProjection<'a> {
    machine_binding: Option<([u8; 32], Option<[u8; 32]>)>,
    table_layouts: Option<&'a [StagingTableLayoutFacts]>,
    footnote_layout: Option<&'a StagingFootnoteLayoutFacts>,
}

fn layout_trace_json(
    flow: &FlowTree,
    initial: &InitialPaginationState,
    pagination: &PaginationResult,
    max_layout_passes: u16,
    include_trace_text: bool,
    projection: LayoutTraceProfileProjection<'_>,
) -> Result<String, &'static str> {
    let LayoutTraceProfileProjection {
        machine_binding,
        table_layouts,
        footnote_layout,
    } = projection;
    let contains_trace_text = !initial.generated_text().buffers().is_empty()
        || pagination
            .passes()
            .iter()
            .any(|pass| !pass.generated_text().buffers().is_empty());
    ensure_requested_trace_text_is_representable(include_trace_text, contains_trace_text)?;
    if pagination.passes().iter().any(|pass| {
        pass.pages().iter().any(|page| {
            (!page.footnote_ids.is_empty() && footnote_layout.is_none())
                || !page.float_decisions.is_empty()
                || !page.column_decisions.is_empty()
        })
    }) {
        return Err("the reference trace encoder received unsupported layout content");
    }
    if footnote_layout.is_some_and(|facts| {
        facts.body_layout_sha256() != pagination.final_fingerprint().bytes()
            || facts.pages().len() != pagination.selected_pages().len()
    }) {
        return Err("footnote trace facts do not match selected pagination");
    }
    let mut json = String::from("{\"contract\":");
    push_jcs_string(&mut json, CONTRACT);
    json.push_str(",\"coordinate_unit\":");
    push_jcs_string(&mut json, COORDINATE_UNIT);
    if let Some((_, flow_registry_sha256)) = machine_binding {
        json.push_str(",\"flow_registry_sha256\":");
        match flow_registry_sha256 {
            Some(value) => push_hex(&mut json, value),
            None => json.push_str("null"),
        }
    }
    if let Some(footnote_layout) = footnote_layout {
        json.push_str(",\"footnote_layout\":");
        json.push_str(footnote_layout.canonical_jcs());
    }
    json.push_str(",\"initial_fingerprint\":");
    push_hex(&mut json, initial.fingerprint().bytes());
    json.push_str(",\"initial_state\":{\"algorithm\":");
    push_jcs_string(&mut json, LayoutStateFingerprint::INITIAL_ALGORITHM_ID);
    json.push_str(",\"flow_positions\":[");
    push_flow_positions(&mut json, flow.positions());
    json.push_str("],\"layout_epoch\":");
    push_layout_epoch(&mut json, initial.layout_epoch());
    json.push_str(",\"resolved_generated_text\":[");
    push_generated_text(&mut json, initial.generated_text());
    json.push_str("]},\"max_layout_passes\":");
    json.push_str(&max_layout_passes.to_string());
    json.push_str(",\"passes\":[");
    for (index, pass) in pagination.passes().iter().enumerate() {
        comma(&mut json, index);
        json.push_str("{\"change_summary\":[");
        if index == 0 {
            let has_reference_flow = pass.pages().iter().any(|page| !page.fragments.is_empty())
                || pass.placed_anchors().next().is_some();
            push_jcs_string(
                &mut json,
                if has_reference_flow {
                    "materialized reference flow"
                } else {
                    "materialized blank page"
                },
            );
        }
        json.push_str("],\"cost\":{");
        let score = pass.fallback_score();
        let components = score.components();
        json.push_str("\"footnote_split\":");
        json.push_str(&components.footnote_split().to_string());
        json.push_str(",\"hard_violations\":");
        json.push_str(&score.hard_violations().to_string());
        json.push_str(",\"heading_isolation\":");
        json.push_str(&components.heading_isolation().to_string());
        json.push_str(",\"keep\":");
        json.push_str(&components.keep().to_string());
        json.push_str(",\"overflow\":");
        json.push_str(&components.overflow().to_string());
        json.push_str(",\"table_split\":");
        json.push_str(&components.table_split().to_string());
        json.push_str(",\"total\":");
        json.push_str(&components.total().to_string());
        json.push_str(",\"unused_space\":");
        json.push_str(&components.unused_space().to_string());
        json.push_str(",\"widow_orphan\":");
        json.push_str(&components.widow_orphan().to_string());
        json.push_str("},\"input_fingerprint\":");
        push_hex(&mut json, pass.input_fingerprint().bytes());
        json.push_str(",\"output_fingerprint\":");
        push_hex(&mut json, pass.output_fingerprint().bytes());
        json.push_str(",\"pass_index\":");
        json.push_str(&pass.pass_index().to_string());
        json.push_str(",\"state\":{\"algorithm\":");
        push_jcs_string(&mut json, LayoutStateFingerprint::MATERIALIZED_ALGORITHM_ID);
        json.push_str(",\"flow_positions\":[");
        push_flow_positions(&mut json, flow.positions());
        json.push_str("],\"layout_epoch\":");
        push_layout_epoch(&mut json, pass.fingerprint_record().layout_epoch());
        json.push_str(",\"pages\":[");
        for (page_index, page) in pass.pages().iter().enumerate() {
            comma(&mut json, page_index);
            push_reference_trace_page_plan(&mut json, page, footnote_layout.is_some())?;
        }
        json.push_str("],\"placed_anchors\":[");
        for (anchor_index, anchor) in pass.placed_anchors().enumerate() {
            comma(&mut json, anchor_index);
            push_placed_anchor(&mut json, anchor);
        }
        json.push_str("],\"resolved_generated_text\":[");
        push_generated_text(&mut json, pass.generated_text());
        json.push_str("]}}");
    }
    json.push(']');
    if let Some((profile_receipt_sha256, _)) = machine_binding {
        json.push_str(",\"profile_receipt_sha256\":");
        push_hex(&mut json, profile_receipt_sha256);
    }
    json.push_str(",\"result\":{");
    if let ConvergenceStatus::CycleFallback { cycle_start_state } = pagination.status() {
        json.push_str("\"cycle_start_state\":");
        json.push_str(&cycle_start_state.get().to_string());
        json.push(',');
    }
    json.push_str("\"fallback_policy\":");
    if matches!(pagination.status(), ConvergenceStatus::Converged) {
        json.push_str("null");
    } else {
        push_jcs_string(&mut json, "lowest_cost_then_earliest");
    }
    json.push_str(",\"final_fingerprint\":");
    push_hex(&mut json, pagination.final_fingerprint().bytes());
    json.push_str(",\"pass_count\":");
    json.push_str(&pagination.passes().len().to_string());
    json.push_str(",\"selected_state\":");
    json.push_str(&pagination.selected_state().get().to_string());
    json.push_str(",\"status\":");
    match pagination.status() {
        ConvergenceStatus::Converged => push_jcs_string(&mut json, "converged"),
        ConvergenceStatus::CycleFallback { .. } => push_jcs_string(&mut json, "cycle_fallback"),
        ConvergenceStatus::MaxPassFallback => push_jcs_string(&mut json, "max_pass_fallback"),
    }
    json.push('}');
    if let Some(table_layouts) = table_layouts {
        json.push_str(",\"table_layouts\":[");
        for (index, table) in table_layouts.iter().enumerate() {
            comma(&mut json, index);
            json.push_str(table.canonical_jcs());
        }
        json.push(']');
    }
    json.push('}');
    Ok(json)
}

fn ensure_requested_trace_text_is_representable(
    include_trace_text: bool,
    contains_trace_text: bool,
) -> Result<(), &'static str> {
    if contains_trace_text && !include_trace_text {
        Err(GENERATED_TRACE_TEXT_REQUIRES_OPT_IN)
    } else {
        Ok(())
    }
}

fn push_generated_text(json: &mut String, generated: &GeneratedTextStore) {
    for (index, buffer) in generated.buffers().iter().enumerate() {
        comma(json, index);
        json.push_str("{\"end_byte\":");
        json.push_str(&buffer.utf8().len().to_string());
        json.push_str(",\"key\":");
        typaxis_core::push_generated_buffer_key_jcs(json, buffer.key());
        json.push_str(",\"start_byte\":0,\"utf8\":");
        push_jcs_string(json, buffer.utf8());
        json.push('}');
    }
}

fn push_flow_positions(json: &mut String, positions: &[FlowPosition]) {
    for (index, position) in positions.iter().enumerate() {
        comma(json, index);
        json.push_str("{\"block_child_path\":[");
        for (path_index, component) in position.block_child_path().iter().enumerate() {
            comma(json, path_index);
            json.push_str(&component.to_string());
        }
        json.push_str("],\"epoch\":");
        push_layout_epoch(json, position.epoch());
        json.push_str(",\"global_flow_ordinal\":");
        json.push_str(&position.global_flow_ordinal().to_string());
        json.push_str(",\"owner\":");
        json.push_str(&position.owner().get().to_string());
        json.push_str(",\"owner_local_boundary\":");
        json.push_str(&position.owner_local_boundary().to_string());
        json.push('}');
    }
}

fn push_layout_epoch(json: &mut String, epoch: LayoutEpoch) {
    json.push_str("{\"admitted_resources_sha256\":");
    push_hex(json, epoch.admitted_resources().bytes());
    json.push_str(",\"document_sha256\":");
    push_hex(json, epoch.document().bytes());
    json.push_str(",\"resolved_input_sha256\":");
    push_hex(json, epoch.references().bytes());
    json.push_str(",\"style_page_master_sha256\":");
    push_hex(json, epoch.style().bytes());
    json.push('}');
}

fn ensure_reference_page_shape(page: &PagePlan) -> Result<(), &'static str> {
    if !page.footnote_ids.is_empty()
        || !page.float_decisions.is_empty()
        || !page.column_decisions.is_empty()
        || !page.resolved_references.is_empty()
    {
        Err("the reference artifact encoder received unsupported page content")
    } else {
        Ok(())
    }
}

fn push_reference_trace_page_plan(
    json: &mut String,
    page: &PagePlan,
    allow_footnotes: bool,
) -> Result<(), &'static str> {
    if (!allow_footnotes && !page.footnote_ids.is_empty())
        || !page.float_decisions.is_empty()
        || !page.column_decisions.is_empty()
    {
        return Err("the reference trace encoder received unsupported page content");
    }
    json.push_str("{\"column_decisions\":[],\"float_decisions\":[],\"footnote_ids\":[");
    for (index, footnote_id) in page.footnote_ids.iter().enumerate() {
        comma(json, index);
        push_jcs_string(json, footnote_id.as_str());
    }
    json.push_str("],\"fragments\":[");
    push_fragments(json, &page.fragments);
    json.push_str("],\"frames\":[");
    push_frames(json, &page.frames);
    json.push_str("],\"master_id\":");
    push_jcs_string(json, page.master_id.as_str());
    json.push_str(",\"page_index\":");
    json.push_str(&page.page_index.to_string());
    json.push_str(",\"resolved_references\":[");
    for (index, reference) in page.resolved_references.iter().enumerate() {
        comma(json, index);
        push_resolved_reference(json, reference);
    }
    json.push_str("]}");
    Ok(())
}

fn push_resolved_reference(json: &mut String, reference: &ResolvedReference) {
    let provenance = reference.provenance();
    let range = provenance.text_span().range();
    json.push_str("{\"anchor_id\":");
    push_jcs_string(json, reference.anchor_id().as_str());
    json.push_str(",\"buffer_key\":");
    typaxis_core::push_generated_buffer_key_jcs(json, provenance.buffer_key());
    json.push_str(",\"end_byte\":");
    json.push_str(&range.end_byte().get().to_string());
    json.push_str(",\"start_byte\":");
    json.push_str(&range.start_byte().get().to_string());
    json.push_str(",\"utf8\":");
    push_jcs_string(json, reference.utf8());
    json.push('}');
}

fn push_fragments(json: &mut String, fragments: &[typaxis_pagination::PlacedFragment]) {
    for (index, fragment) in fragments.iter().enumerate() {
        comma(json, index);
        json.push_str("{\"bounds\":");
        push_rect(json, fragment.bounds);
        json.push_str(",\"column_index\":");
        json.push_str(&fragment.column_index.to_string());
        json.push_str(",\"end\":");
        push_flow_position(json, &fragment.end);
        json.push_str(",\"frame_kind\":");
        push_jcs_string(json, frame_kind_name(fragment.frame_kind));
        json.push_str(",\"owner\":");
        json.push_str(&fragment.owner.get().to_string());
        json.push_str(",\"owner_local_ordinal\":");
        json.push_str(&fragment.owner_local_ordinal.to_string());
        json.push_str(",\"start\":");
        push_flow_position(json, &fragment.start);
        json.push('}');
    }
}

fn push_frames(json: &mut String, frames: &[typaxis_pagination::PageFramePlan]) {
    for (index, frame) in frames.iter().enumerate() {
        comma(json, index);
        json.push_str("{\"bounds\":");
        push_rect(json, frame.bounds);
        json.push_str(",\"column_index\":");
        json.push_str(&frame.column_index.to_string());
        json.push_str(",\"kind\":");
        push_jcs_string(json, frame_kind_name(frame.kind));
        json.push('}');
    }
}

fn push_flow_position(json: &mut String, position: &FlowPosition) {
    json.push_str("{\"block_child_path\":[");
    for (path_index, component) in position.block_child_path().iter().enumerate() {
        comma(json, path_index);
        json.push_str(&component.to_string());
    }
    json.push_str("],\"epoch\":");
    push_layout_epoch(json, position.epoch());
    json.push_str(",\"global_flow_ordinal\":");
    json.push_str(&position.global_flow_ordinal().to_string());
    json.push_str(",\"owner\":");
    json.push_str(&position.owner().get().to_string());
    json.push_str(",\"owner_local_boundary\":");
    json.push_str(&position.owner_local_boundary().to_string());
    json.push('}');
}

fn push_placed_anchor(json: &mut String, anchor: &PlacedAnchor) {
    json.push_str("{\"anchor_id\":");
    push_jcs_string(json, anchor.anchor_id().as_str());
    json.push_str(",\"column_index\":");
    json.push_str(&anchor.column_index().to_string());
    json.push_str(",\"frame_kind\":");
    push_jcs_string(json, frame_kind_name(anchor.frame_kind()));
    json.push_str(",\"owner\":");
    json.push_str(&anchor.owner_node().get().to_string());
    json.push_str(",\"page_index\":");
    json.push_str(&anchor.page_index().to_string());
    json.push_str(",\"position_in_frame\":{\"x\":");
    json.push_str(&anchor.position_in_frame().x.raw().to_string());
    json.push_str(",\"y\":");
    json.push_str(&anchor.position_in_frame().y.raw().to_string());
    json.push_str("}}");
}

const fn frame_kind_name(kind: PageFrameKind) -> &'static str {
    match kind {
        PageFrameKind::Body => "body",
        PageFrameKind::Header => "header",
        PageFrameKind::Footer => "footer",
        PageFrameKind::Footnote => "footnote",
    }
}

fn push_rect(json: &mut String, rect: Rect) {
    json.push_str("{\"height\":");
    json.push_str(&rect.height().get().raw().to_string());
    json.push_str(",\"width\":");
    json.push_str(&rect.width().get().raw().to_string());
    json.push_str(",\"x\":");
    json.push_str(&rect.x().raw().to_string());
    json.push_str(",\"y\":");
    json.push_str(&rect.y().raw().to_string());
    json.push('}');
}

fn comma(output: &mut String, index: usize) {
    if index > 0 {
        output.push(',');
    }
}

pub fn push_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline;
    use typaxis_core::{
        ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions, PdfStreamCompression,
        PortablePath, ResourceLimits, SourceId,
    };
    use typaxis_syntax::{
        PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
    };

    fn config() -> EffectiveConfig {
        EffectiveConfig::new(
            false,
            PdfStreamCompression::Flate,
            vec![ConfigResourceRoot::ProjectRoot],
            ["http", "https", "mailto", "tel"]
                .map(str::to_owned)
                .to_vec(),
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
            ResourceLimits::default(),
        )
        .unwrap()
    }

    fn package_with_config(text: &str, config: &EffectiveConfig) -> Box<ValidatedParsedPackage> {
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("input.tsf").unwrap(),
            text: text.to_owned(),
        };
        let ParseOutcome::Parsed { package, .. } = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes()).unwrap(),
        ) else {
            panic!("source should parse")
        };
        package
    }

    fn assert_jcs_member_order(json: &str) {
        fn string_end(bytes: &[u8], mut position: usize) -> usize {
            assert_eq!(bytes.get(position), Some(&b'"'));
            position += 1;
            while let Some(&byte) = bytes.get(position) {
                match byte {
                    b'"' => return position + 1,
                    b'\\' => {
                        position += 2;
                        if bytes.get(position.wrapping_sub(1)) == Some(&b'u') {
                            position += 4;
                        }
                    }
                    _ => position += 1,
                }
            }
            panic!("unterminated JSON string")
        }

        fn value_end(bytes: &[u8], position: usize) -> usize {
            match bytes.get(position) {
                Some(b'{') => object_end(bytes, position),
                Some(b'[') => {
                    let mut position = position + 1;
                    if bytes.get(position) == Some(&b']') {
                        return position + 1;
                    }
                    loop {
                        position = value_end(bytes, position);
                        match bytes.get(position) {
                            Some(b',') => position += 1,
                            Some(b']') => return position + 1,
                            _ => panic!("invalid JSON array at byte {position}"),
                        }
                    }
                }
                Some(b'"') => string_end(bytes, position),
                Some(b't') => {
                    assert_eq!(bytes.get(position..position + 4), Some(b"true".as_slice()));
                    position + 4
                }
                Some(b'n') => {
                    assert_eq!(bytes.get(position..position + 4), Some(b"null".as_slice()));
                    position + 4
                }
                Some(b'f') => {
                    assert_eq!(bytes.get(position..position + 5), Some(b"false".as_slice()));
                    position + 5
                }
                Some(b'-' | b'0'..=b'9') => {
                    let mut position = position + 1;
                    while matches!(
                        bytes.get(position),
                        Some(b'+' | b'-' | b'.' | b'0'..=b'9' | b'E' | b'e')
                    ) {
                        position += 1;
                    }
                    position
                }
                _ => panic!("invalid JSON value at byte {position}"),
            }
        }

        fn object_end(bytes: &[u8], mut position: usize) -> usize {
            assert_eq!(bytes.get(position), Some(&b'{'));
            position += 1;
            if bytes.get(position) == Some(&b'}') {
                return position + 1;
            }
            let mut previous_key: Option<&[u8]> = None;
            loop {
                let key_start = position + 1;
                let key_end_with_quote = string_end(bytes, position);
                let key = &bytes[key_start..key_end_with_quote - 1];
                assert!(!key.contains(&b'\\'), "artifact member names must be ASCII");
                if let Some(previous) = previous_key {
                    assert!(
                        previous < key,
                        "non-canonical member order: `{}` before `{}`",
                        String::from_utf8_lossy(previous),
                        String::from_utf8_lossy(key)
                    );
                }
                previous_key = Some(key);
                position = key_end_with_quote;
                assert_eq!(bytes.get(position), Some(&b':'));
                position = value_end(bytes, position + 1);
                match bytes.get(position) {
                    Some(b',') => position += 1,
                    Some(b'}') => return position + 1,
                    _ => panic!("invalid JSON object at byte {position}"),
                }
            }
        }

        let bytes = json.as_bytes();
        assert_eq!(value_end(bytes, 0), bytes.len());
    }

    #[test]
    fn package_json_contains_reference_parser_facts_without_source_text() {
        let config = config();
        let json = document_package_json(
            &package_with_config("text:hello\nanchor:target\n", &config),
            config.limits(),
        )
        .unwrap();
        assert!(json.starts_with("{\"contract\":\"typaxis.contract/1.3\""));
        assert!(json.contains("\"kind\":\"text\""));
        assert!(json.contains("\"anchor_id\":\"target\""));
        assert!(!json.contains("text:hello"));
    }

    #[test]
    fn package_json_uses_the_full_converter_for_styles_and_resources() {
        let config = config();
        let json = document_package_json(
            &package_with_config("font:Body:body.ttf\ntext:hello\n", &config),
            config.limits(),
        )
        .unwrap();
        assert!(json.contains("\"font_faces\":[{"));
        assert!(json.contains("\"family\":\"Body\""));
        assert!(json.contains("\"name\":\"font_family\""));
        assert!(json.contains("\"kind\":\"font_family_list\""));
        assert_jcs_member_order(&json);
    }

    #[test]
    fn document_package_uses_jcs_member_order_recursively() {
        let config = config();
        let json = document_package_json(
            &package_with_config("text:hello\nanchor:target\n", &config),
            config.limits(),
        )
        .unwrap();
        assert_jcs_member_order(&json);
    }

    #[test]
    fn layout_page_uses_jcs_member_order_recursively() {
        let config = config();
        let package = package_with_config("", &config);
        let current = std::env::current_dir().unwrap();
        let admission = typaxis_core::HostAdmissionContext::new(
            typaxis_core::HostPath::new(current.join("input.tsf")).unwrap(),
            typaxis_core::HostPath::new(current).unwrap(),
            None,
            vec![],
        );
        let layout = pipeline::layout_reference(&package, &config, &admission).unwrap();
        let page = &layout.pagination.selected_pages()[0];
        let master = &package.package().page_masters.masters[0];
        let json =
            reference_layout_page_json(page, master.width.get().raw(), master.height.get().raw())
                .unwrap();
        assert_jcs_member_order(&json);
    }

    #[test]
    fn layout_trace_uses_jcs_member_order_recursively() {
        let config = config();
        let package = package_with_config("", &config);
        let current = std::env::current_dir().unwrap();
        let admission = typaxis_core::HostAdmissionContext::new(
            typaxis_core::HostPath::new(current.join("input.tsf")).unwrap(),
            typaxis_core::HostPath::new(current).unwrap(),
            None,
            vec![],
        );
        let layout = pipeline::layout_reference(&package, &config, &admission).unwrap();
        let json = reference_layout_trace_json(
            &layout.flow,
            &layout.initial,
            &layout.pagination,
            config.limits().get().max_layout_passes,
            false,
        )
        .unwrap();
        let opted_in = reference_layout_trace_json(
            &layout.flow,
            &layout.initial,
            &layout.pagination,
            config.limits().get().max_layout_passes,
            true,
        )
        .unwrap();
        assert_eq!(opted_in, json);
        assert_jcs_member_order(&json);
    }

    #[test]
    fn reference_artifacts_encode_fragments_and_anchors() {
        let config = config();
        let package = package_with_config("paragraph\nanchor:target\n", &config);
        let current = std::env::current_dir().unwrap();
        let admission = typaxis_core::HostAdmissionContext::new(
            typaxis_core::HostPath::new(current.join("input.tsf")).unwrap(),
            typaxis_core::HostPath::new(current).unwrap(),
            None,
            vec![],
        );
        let layout = pipeline::layout_reference(&package, &config, &admission).unwrap();
        let page = &layout.pagination.selected_pages()[0];
        let master = &package.package().page_masters.masters[0];
        let page_json =
            reference_layout_page_json(page, master.width.get().raw(), master.height.get().raw())
                .unwrap();
        let trace_json = reference_layout_trace_json(
            &layout.flow,
            &layout.initial,
            &layout.pagination,
            config.limits().get().max_layout_passes,
            false,
        )
        .unwrap();

        assert!(page_json.contains("\"fragments\":[{"));
        assert!(trace_json.contains("\"anchor_id\":\"target\""));
        assert!(trace_json.contains("\"fragments\":[{"));
        assert_jcs_member_order(&page_json);
        assert_jcs_member_order(&trace_json);
    }

    #[test]
    fn generated_trace_text_requires_explicit_opt_in() {
        assert!(ensure_requested_trace_text_is_representable(true, false).is_ok());
        assert!(ensure_requested_trace_text_is_representable(true, true).is_ok());
        assert_eq!(
            ensure_requested_trace_text_is_representable(false, true),
            Err(GENERATED_TRACE_TEXT_REQUIRES_OPT_IN)
        );
    }
}
