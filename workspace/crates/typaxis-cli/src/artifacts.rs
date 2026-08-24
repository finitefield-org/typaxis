use typaxis_core::{
    push_jcs_string, LayoutStateFingerprint, Rect, SourceSpan, TextSpan, CONTRACT, COORDINATE_UNIT,
};
use typaxis_document::{Block, Inline};
use typaxis_layout::{FlowPosition, FlowTree, LayoutEpoch};
use typaxis_pagination::{
    ConvergenceStatus, InitialPaginationState, PageFrameKind, PagePlan, PaginationResult,
    PlacedAnchor,
};
use typaxis_syntax::ValidatedParsedPackage;
use typaxis_text::TextMapKind;

pub fn document_package_json(package: &ValidatedParsedPackage) -> Result<String, &'static str> {
    let package = package.package();
    if !package.style_sheet.rules.is_empty()
        || !package.page_masters.selection_rules.is_empty()
        || !package.resources.font_faces.is_empty()
        || !package.resources.images.is_empty()
        || !package.document.footnotes.is_empty()
    {
        return Err("the reference artifact encoder received an unsupported package shape");
    }

    let mut json = String::from("{\"contract\":");
    push_jcs_string(&mut json, CONTRACT);
    json.push_str(",\"coordinate_unit\":");
    push_jcs_string(&mut json, COORDINATE_UNIT);
    json.push_str(",\"document\":{\"blocks\":[");
    for (index, block) in package.document.blocks.iter().enumerate() {
        comma(&mut json, index);
        push_reference_block(&mut json, block)?;
    }
    json.push_str("],\"footnotes\":[],\"node_id\":");
    json.push_str(&package.document.node_id.get().to_string());
    json.push_str("},\"page_masters\":{");
    json.push_str("\"default_master_id\":");
    push_jcs_string(&mut json, package.page_masters.default_master_id.as_str());
    json.push_str(",\"masters\":[");
    for (index, master) in package.page_masters.masters.iter().enumerate() {
        comma(&mut json, index);
        json.push_str("{\"body\":");
        push_rect(&mut json, master.body);
        json.push_str(",\"footer\":");
        push_optional_rect(&mut json, master.footer);
        json.push_str(",\"footnote\":");
        push_optional_rect(&mut json, master.footnote);
        json.push_str(",\"header\":");
        push_optional_rect(&mut json, master.header);
        json.push_str(",\"height\":");
        json.push_str(&master.height.get().raw().to_string());
        json.push_str(",\"master_id\":");
        push_jcs_string(&mut json, master.master_id.as_str());
        json.push_str(",\"width\":");
        json.push_str(&master.width.get().raw().to_string());
        json.push('}');
    }
    json.push_str(
        "],\"selection_rules\":[]},\"resources\":{\"font_faces\":[],\"images\":[]},\"sources\":[",
    );
    for (index, source) in package.sources.records().iter().enumerate() {
        comma(&mut json, index);
        json.push_str("{\"sha256\":");
        push_hex(&mut json, source.content_hash());
        json.push_str(",\"source_id\":");
        json.push_str(&source.source_id().get().to_string());
        json.push_str(",\"uri\":");
        push_jcs_string(&mut json, source.uri().as_str());
        json.push_str(",\"utf8_byte_length\":");
        json.push_str(&source.utf8_byte_length().to_string());
        json.push('}');
    }
    json.push_str("],\"style_sheet\":{\"rules\":[]},\"text_buffers\":[");
    for (index, buffer) in package.text_store.buffers().iter().enumerate() {
        comma(&mut json, index);
        json.push_str("{\"mappings\":[");
        for (mapping_index, mapping) in buffer.mappings().iter().enumerate() {
            comma(&mut json, mapping_index);
            json.push_str("{\"kind\":");
            push_jcs_string(
                &mut json,
                match mapping.kind {
                    TextMapKind::Identity => "identity",
                    TextMapKind::Replacement => "replacement",
                    TextMapKind::Inserted => "inserted",
                },
            );
            json.push_str(",\"source_span\":");
            match mapping.source_span {
                Some(span) => push_source_span(&mut json, span),
                None => json.push_str("null"),
            }
            json.push_str(",\"text_range\":{\"end_byte\":");
            json.push_str(&mapping.text_range.end_byte().get().to_string());
            json.push_str(",\"start_byte\":");
            json.push_str(&mapping.text_range.start_byte().get().to_string());
            json.push_str("}}");
        }
        json.push_str("],\"text_id\":");
        json.push_str(&buffer.text_id().get().to_string());
        json.push_str(",\"utf8\":");
        push_jcs_string(&mut json, buffer.text());
        json.push('}');
    }
    json.push_str("]}");
    Ok(json)
}

fn push_reference_block(json: &mut String, block: &Block) -> Result<(), &'static str> {
    let Block::Paragraph {
        node_id,
        span,
        classes,
        children,
    } = block
    else {
        return Err("the reference parser emitted an unsupported block");
    };
    json.push_str("{\"children\":[");
    for (index, inline) in children.iter().enumerate() {
        comma(json, index);
        push_reference_inline(json, inline)?;
    }
    json.push_str("],\"classes\":[");
    for (index, class) in classes.iter().enumerate() {
        comma(json, index);
        push_jcs_string(json, class);
    }
    json.push_str("],\"kind\":\"paragraph\",\"node_id\":");
    json.push_str(&node_id.get().to_string());
    json.push_str(",\"span\":");
    push_source_span(json, *span);
    json.push('}');
    Ok(())
}

fn push_reference_inline(json: &mut String, inline: &Inline) -> Result<(), &'static str> {
    match inline {
        Inline::Text {
            node_id,
            span,
            text_span,
        } => {
            json.push_str("{\"kind\":\"text\",\"node_id\":");
            json.push_str(&node_id.get().to_string());
            json.push_str(",\"span\":");
            push_source_span(json, *span);
            json.push_str(",\"text_span\":");
            push_text_span(json, *text_span);
            json.push('}');
        }
        Inline::Anchor {
            node_id,
            span,
            anchor_id,
        } => {
            json.push_str("{\"anchor_id\":");
            push_jcs_string(json, anchor_id.as_str());
            json.push_str(",\"kind\":\"anchor\",\"node_id\":");
            json.push_str(&node_id.get().to_string());
            json.push_str(",\"span\":");
            push_source_span(json, *span);
            json.push('}');
        }
        _ => return Err("the reference parser emitted an unsupported inline"),
    }
    Ok(())
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
    let contains_trace_text = !initial.generated_text().buffers().is_empty()
        || pagination
            .passes()
            .iter()
            .any(|pass| !pass.generated_text().buffers().is_empty());
    ensure_requested_trace_text_is_representable(include_trace_text, contains_trace_text)?;
    if contains_trace_text {
        return Err("the reference trace encoder received unsupported generated text");
    }
    if pagination.passes().iter().any(|pass| {
        pass.pages().iter().any(|page| {
            !page.footnote_ids.is_empty()
                || !page.float_decisions.is_empty()
                || !page.column_decisions.is_empty()
                || !page.resolved_references.is_empty()
        })
    }) {
        return Err("the reference trace encoder received unsupported layout content");
    }
    let mut json = String::from("{\"contract\":");
    push_jcs_string(&mut json, CONTRACT);
    json.push_str(",\"coordinate_unit\":");
    push_jcs_string(&mut json, COORDINATE_UNIT);
    json.push_str(",\"initial_fingerprint\":");
    push_hex(&mut json, initial.fingerprint().bytes());
    json.push_str(",\"initial_state\":{\"algorithm\":");
    push_jcs_string(&mut json, LayoutStateFingerprint::INITIAL_ALGORITHM_ID);
    json.push_str(",\"flow_positions\":[");
    push_flow_positions(&mut json, flow.positions());
    json.push_str("],\"layout_epoch\":");
    push_layout_epoch(&mut json, initial.layout_epoch());
    json.push_str(",\"resolved_generated_text\":[]},\"max_layout_passes\":");
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
            push_reference_page_plan(&mut json, page)?;
        }
        json.push_str("],\"placed_anchors\":[");
        for (anchor_index, anchor) in pass.placed_anchors().enumerate() {
            comma(&mut json, anchor_index);
            push_placed_anchor(&mut json, anchor);
        }
        json.push_str("],\"resolved_generated_text\":[]}}");
    }
    json.push_str("],\"result\":{");
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
    json.push_str("}}");
    Ok(json)
}

fn ensure_requested_trace_text_is_representable(
    include_trace_text: bool,
    contains_trace_text: bool,
) -> Result<(), &'static str> {
    if include_trace_text && contains_trace_text {
        Err("requested trace text is not representable by the reference trace encoder")
    } else {
        Ok(())
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

fn push_reference_page_plan(json: &mut String, page: &PagePlan) -> Result<(), &'static str> {
    ensure_reference_page_shape(page)?;
    json.push_str(
        "{\"column_decisions\":[],\"float_decisions\":[],\"footnote_ids\":[],\"fragments\":[",
    );
    push_fragments(json, &page.fragments);
    json.push_str("],\"frames\":[");
    push_frames(json, &page.frames);
    json.push_str("],\"master_id\":");
    push_jcs_string(json, page.master_id.as_str());
    json.push_str(",\"page_index\":");
    json.push_str(&page.page_index.to_string());
    json.push_str(",\"resolved_references\":[]}");
    Ok(())
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

fn push_source_span(json: &mut String, span: SourceSpan) {
    json.push_str("{\"end_byte\":");
    json.push_str(&span.end_byte().get().to_string());
    json.push_str(",\"source_id\":");
    json.push_str(&span.source_id().get().to_string());
    json.push_str(",\"start_byte\":");
    json.push_str(&span.start_byte().get().to_string());
    json.push('}');
}

fn push_text_span(json: &mut String, span: TextSpan) {
    json.push_str("{\"end_byte\":");
    json.push_str(&span.end_byte().get().to_string());
    json.push_str(",\"start_byte\":");
    json.push_str(&span.start_byte().get().to_string());
    json.push_str(",\"text_id\":");
    json.push_str(&span.text_id().get().to_string());
    json.push('}');
}

fn push_optional_rect(json: &mut String, rect: Option<Rect>) {
    match rect {
        Some(rect) => push_rect(json, rect),
        None => json.push_str("null"),
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

    fn package(text: &str) -> Box<ValidatedParsedPackage> {
        package_with_config(text, &config())
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
        let json = document_package_json(&package("text:hello\nanchor:target\n")).unwrap();
        assert!(json.starts_with("{\"contract\":\"typaxis.contract/1.0\""));
        assert!(json.contains("\"kind\":\"text\""));
        assert!(json.contains("\"anchor_id\":\"target\""));
        assert!(!json.contains("text:hello"));
    }

    #[test]
    fn document_package_uses_jcs_member_order_recursively() {
        let json = document_package_json(&package("text:hello\nanchor:target\n")).unwrap();
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
    fn requested_trace_text_fails_closed_only_when_text_is_present() {
        assert!(ensure_requested_trace_text_is_representable(true, false).is_ok());
        assert!(ensure_requested_trace_text_is_representable(false, true).is_ok());
        assert_eq!(
            ensure_requested_trace_text_is_representable(true, true),
            Err("requested trace text is not representable by the reference trace encoder")
        );
    }
}
