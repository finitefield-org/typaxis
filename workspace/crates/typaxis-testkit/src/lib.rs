#![forbid(unsafe_code)]

mod safe_vector_pdf;

pub use safe_vector_pdf::{
    inspect_safe_vector_pdf, SafeVectorPdfIndependentError, SafeVectorPdfIndependentExpectations,
    SafeVectorPdfIndependentReport,
};

use typaxis_core::{
    Length, MasterId, PortablePath, PositiveLength, Rect, ResourceLimits, SourceId,
    ValidatedResourceLimits,
};
use typaxis_layout::{FlowCursor, FlowTree, LayoutEpoch, PageContext, ResolvedPageSelection};
use typaxis_pagination::{
    ConvergenceStatus, InitialPaginationState, LayoutPass, LayoutPassInput, PageFrameKind,
    PageFramePlan, PagePlan, PaginationInput, PaginationOptions, PaginationOutcome,
    PaginationResult, PreparedLayout,
};
use typaxis_resources::AdmittedResourceResolver;
use typaxis_style::{PageMaster, PageMasterSet};
use typaxis_syntax::{
    PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
    ValidatedParsedPackage,
};
use typaxis_text::GeneratedTextStore;

fn limits() -> ValidatedResourceLimits {
    ValidatedResourceLimits::new(ResourceLimits::default()).unwrap()
}

fn default_masters() -> PageMasterSet {
    let size = PositiveLength::new(Length::from_raw(10).unwrap()).unwrap();
    PageMasterSet {
        default_master_id: MasterId::new("default").unwrap(),
        masters: vec![PageMaster {
            master_id: MasterId::new("default").unwrap(),
            width: size,
            height: size,
            body: Rect::new(Length::ZERO, Length::ZERO, size, size),
            header: None,
            footer: None,
            footnote: None,
        }],
        selection_rules: vec![],
    }
}

fn validated_blank_package() -> ValidatedParsedPackage {
    let limits = limits();
    let source = SourceFile {
        source_id: SourceId::new(0),
        uri: PortablePath::new("input.tsf").unwrap(),
        text: String::new(),
    };
    let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
    let outcome = ReferenceParser::new().parse(
        &source,
        &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
    );
    let ParseOutcome::Parsed { package, .. } = outcome else {
        panic!("reference package must parse");
    };
    *package
}

fn generated_store(
    package: &ValidatedParsedPackage,
    limits: &ValidatedResourceLimits,
) -> GeneratedTextStore {
    GeneratedTextStore::new(
        vec![],
        package.document_nodes(),
        limits,
        &package.package().text_store,
    )
    .unwrap()
}

fn layout_epoch(
    package: &ValidatedParsedPackage,
    generated: &GeneratedTextStore,
    limits: &ValidatedResourceLimits,
) -> LayoutEpoch {
    let admitted = AdmittedResourceResolver::new(&package.package().resources, limits)
        .unwrap()
        .finish()
        .unwrap();
    let generated = package.bind_generated_text(generated, limits).unwrap();
    LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap()
}

pub fn epoch() -> LayoutEpoch {
    let limits = limits();
    let package = validated_blank_package();
    let generated = generated_store(&package, &limits);
    layout_epoch(&package, &generated, &limits)
}
pub fn empty_flow() -> FlowTree {
    let limits = limits();
    let package = validated_blank_package();
    let generated = generated_store(&package, &limits);
    FlowTree::empty(&package, layout_epoch(&package, &generated, &limits)).unwrap()
}
pub fn start_cursor(flow: &FlowTree) -> FlowCursor {
    FlowCursor::document_start(flow)
}
pub fn prepared_layout(
    input: LayoutPassInput<'_>,
    flow: FlowTree,
    cursor: FlowCursor,
) -> PreparedLayout {
    PreparedLayout::new(input, flow, cursor).unwrap()
}
pub fn selected_blank_pagination() -> PaginationResult {
    let limits = limits();
    let masters = default_masters();
    let bounds = masters.masters[0].body;
    let package = validated_blank_package();
    let generated = generated_store(&package, &limits);
    let flow = FlowTree::empty(&package, layout_epoch(&package, &generated, &limits)).unwrap();
    let initial = InitialPaginationState::new(&flow, &package, &limits).unwrap();
    let package_context = package.pagination_context();
    let mut input = PaginationInput::new(
        initial,
        &package_context,
        PaginationOptions::from_limits(&limits, false),
    )
    .unwrap();
    let cursor = FlowCursor::document_start(&flow);
    let selection = ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
    let pages = vec![PagePlan {
        page_index: 0,
        master_id: MasterId::new("default").unwrap(),
        frames: vec![PageFramePlan {
            kind: PageFrameKind::Body,
            column_index: 0,
            bounds,
        }],
        fragments: vec![],
        footnote_ids: vec![],
        float_decisions: vec![],
        column_decisions: vec![],
        resolved_references: vec![],
    }];
    let mut budget = input.take_work_budget().unwrap();
    let mut first_permit = budget
        .begin_pass(0, LayoutPassInput::initial(&input))
        .unwrap();
    let context = PageContext::select(0, &selection, &package_context).unwrap();
    first_permit
        .begin_page(&context, &cursor, &pages[0].frames)
        .unwrap();
    first_permit.finish_page(&pages[0]).unwrap();
    let first_receipt = first_permit.finish(&flow, &pages).unwrap();
    let first = LayoutPass::new(
        first_receipt,
        input.initial_fingerprint(),
        &flow,
        pages.clone(),
        generated.clone(),
    )
    .unwrap();
    let transition = first.transition_references(&package, &limits).unwrap();
    let mut second_permit = budget
        .begin_pass(1, LayoutPassInput::transitioned(transition))
        .unwrap();
    second_permit
        .begin_page(&context, &cursor, &pages[0].frames)
        .unwrap();
    second_permit.finish_page(&pages[0]).unwrap();
    let second_receipt = second_permit.finish(&flow, &pages).unwrap();
    let second = LayoutPass::new(
        second_receipt,
        first.output_fingerprint(),
        &flow,
        pages,
        generated,
    )
    .unwrap();
    PaginationOutcome::new(
        vec![first, second],
        ConvergenceStatus::Converged,
        &input,
        budget.finish(),
    )
    .unwrap()
    .into_result()
}
pub fn rect(width: i64, height: i64) -> Rect {
    Rect::new(
        Length::ZERO,
        Length::ZERO,
        PositiveLength::new(Length::from_raw(width).unwrap()).unwrap(),
        PositiveLength::new(Length::from_raw(height).unwrap()).unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Component, Path, PathBuf};
    use typaxis_display_list::ValidatedDisplayDocument;
    use typaxis_pdf::{PdfBackend, PdfError};
    use typaxis_resources::FrozenPdfResourcePlans;

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("testkit must be nested under workspace/crates")
            .to_path_buf()
    }

    const VMB_RESOURCE_HEADER: [&str; 8] = [
        "image_id",
        "media_type",
        "uri",
        "svg_path",
        "expected_sha256",
        "engine_id",
        "engine_version",
        "rules_version",
    ];
    const VMB_CASE_HEADER: [&str; 20] = [
        "case_id",
        "kind",
        "image_id",
        "expected_sha256",
        "source_tex_path",
        "alt",
        "actual_text",
        "language",
        "advance",
        "ascent",
        "descent",
        "origin_x",
        "baseline",
        "viewport_width",
        "viewport_height",
        "spacing_before",
        "spacing_after",
        "equation_number",
        "minimum_gap",
        "categories",
    ];
    const VMB_FRAGMENT_HEADER: [&str; 8] = [
        "fragment_id",
        "text_path",
        "cases",
        "inline_remaining_width",
        "block_frame_width",
        "block_remaining_height",
        "next_empty_frame_height",
        "categories",
    ];
    const VMB_NEGATIVE_HEADER: [&str; 3] = ["case_id", "expected_reason", "svg_path"];

    #[derive(Clone, Debug, Default)]
    struct SafeSvgFacts {
        current_color: bool,
        stroke: bool,
        clip: bool,
        definitions: bool,
        fill_opacity: bool,
        stroke_opacity: bool,
    }

    #[derive(Debug, Default)]
    struct SafeSvgValidationState {
        facts: SafeSvgFacts,
        ids: BTreeSet<String>,
        references: BTreeSet<String>,
    }

    #[derive(Clone, Debug)]
    struct VmbResource {
        sha256: String,
        provenance: (String, String, String),
        intrinsic_width: i64,
        intrinsic_height: i64,
        facts: SafeSvgFacts,
    }

    #[derive(Clone, Copy, Debug)]
    struct VmbMetrics {
        advance: i64,
        ascent: i64,
        descent: i64,
        origin_x: i64,
        baseline: i64,
        viewport_width: i64,
        viewport_height: i64,
    }

    #[derive(Clone, Debug)]
    struct VmbCase {
        kind: String,
        image_id: u32,
        source_tex: Option<String>,
        alt: String,
        actual_text: Option<String>,
        language: String,
        metrics: Option<VmbMetrics>,
        spacing: Option<(i64, i64)>,
        equation_number: Option<(String, i64)>,
        categories: Vec<String>,
    }

    #[derive(Clone, Debug)]
    struct VmbFragment {
        text: String,
        cases: Vec<String>,
        inline_remaining_width: Option<i64>,
        block_context: Option<(i64, i64, i64)>,
        categories: Vec<String>,
    }

    #[derive(Debug)]
    struct VmbCorpus {
        resources: BTreeMap<u32, VmbResource>,
        cases: BTreeMap<String, VmbCase>,
        fragments: BTreeMap<String, VmbFragment>,
    }

    fn vmb_corpus_root() -> PathBuf {
        workspace_root()
            .parent()
            .expect("workspace must be below the repository root")
            .join("samples/machine-package/staging/production-book-1/precomposed-vector")
    }

    fn canonical_utf8<'a>(bytes: &'a [u8], label: &str) -> Result<&'a str, String> {
        let text = std::str::from_utf8(bytes).map_err(|error| format!("{label}: {error}"))?;
        if text.starts_with('\u{feff}') {
            return Err(format!("{label}: UTF-8 BOM is forbidden"));
        }
        if text.contains('\0') {
            return Err(format!("{label}: NUL is forbidden"));
        }
        if text.contains('\r') {
            return Err(format!("{label}: CR is forbidden; use LF"));
        }
        if !text.ends_with('\n') {
            return Err(format!("{label}: final LF is required"));
        }
        Ok(text)
    }

    fn read_canonical_utf8(path: &Path) -> Result<String, String> {
        let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        canonical_utf8(&bytes, &path.display().to_string()).map(str::to_owned)
    }

    fn parse_tsv<'a>(
        text: &'a str,
        expected_header: &[&str],
        label: &str,
    ) -> Result<Vec<Vec<&'a str>>, String> {
        let body = text
            .strip_suffix('\n')
            .ok_or_else(|| format!("{label}: final LF is required"))?;
        let mut lines = body.split('\n');
        let header = lines
            .next()
            .ok_or_else(|| format!("{label}: header is required"))?;
        if header != expected_header.join("\t") {
            return Err(format!("{label}: unexpected TSV header"));
        }
        let mut rows = Vec::new();
        for (index, line) in lines.enumerate() {
            if line.is_empty() {
                return Err(format!("{label}: blank row at line {}", index + 2));
            }
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != expected_header.len() || fields.iter().any(|field| field.is_empty())
            {
                return Err(format!(
                    "{label}: line {} must have {} nonempty fields",
                    index + 2,
                    expected_header.len()
                ));
            }
            rows.push(fields);
        }
        if rows.is_empty() {
            return Err(format!("{label}: at least one data row is required"));
        }
        Ok(rows)
    }

    fn is_portable_relative_path(value: &str) -> bool {
        if value.is_empty() || value.contains('\\') || !value.is_ascii() {
            return false;
        }
        let path = Path::new(value);
        !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
    }

    fn contained_file(root: &Path, value: &str) -> Result<PathBuf, String> {
        if !is_portable_relative_path(value) {
            return Err(format!("non-contained portable path: {value}"));
        }
        let canonical_root = root
            .canonicalize()
            .map_err(|error| format!("{}: {error}", root.display()))?;
        let candidate = root.join(value);
        let canonical_candidate = candidate
            .canonicalize()
            .map_err(|error| format!("{}: {error}", candidate.display()))?;
        if !canonical_candidate.starts_with(&canonical_root) {
            return Err(format!("path escapes corpus root: {value}"));
        }
        if !canonical_candidate
            .metadata()
            .map_err(|error| format!("{}: {error}", candidate.display()))?
            .is_file()
        {
            return Err(format!("path is not a regular file: {value}"));
        }
        Ok(canonical_candidate)
    }

    fn parse_canonical_i64(value: &str, label: &str) -> Result<i64, String> {
        if value.starts_with('+') || value == "-0" {
            return Err(format!("{label}: noncanonical integer {value:?}"));
        }
        let parsed = value
            .parse::<i64>()
            .map_err(|_| format!("{label}: invalid integer {value:?}"))?;
        if parsed.to_string() != value {
            return Err(format!("{label}: noncanonical integer {value:?}"));
        }
        Ok(parsed)
    }

    fn parse_u32(value: &str, label: &str) -> Result<u32, String> {
        let value = parse_canonical_i64(value, label)?;
        u32::try_from(value).map_err(|_| format!("{label}: out of u32 range"))
    }

    fn parse_optional_nonnegative(value: &str, label: &str) -> Result<Option<i64>, String> {
        if value == "-" {
            return Ok(None);
        }
        let value = parse_canonical_i64(value, label)?;
        if value < 0 {
            return Err(format!("{label}: value must be nonnegative"));
        }
        Ok(Some(value))
    }

    fn lowercase_sha256(value: &str) -> bool {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        typaxis_core::sha256(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn validate_expected_hash(expected: &str, bytes: &[u8], label: &str) -> Result<(), String> {
        if !lowercase_sha256(expected) {
            return Err(format!("{label}: SHA-256 must be 64 lowercase hex digits"));
        }
        let actual = sha256_hex(bytes);
        if actual != expected {
            return Err(format!(
                "{label}: SHA-256 mismatch: expected {expected}, got {actual}"
            ));
        }
        Ok(())
    }

    fn validate_printable_ascii(value: &str, label: &str) -> Result<(), String> {
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
        {
            return Err(format!(
                "{label}: provenance must be 1..=128 printable ASCII bytes"
            ));
        }
        Ok(())
    }

    fn identifier(value: &str) -> bool {
        !value.is_empty()
            && !value.starts_with('-')
            && !value.ends_with('-')
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    }

    fn parse_sorted_list(value: &str, label: &str) -> Result<Vec<String>, String> {
        let values: Vec<_> = value.split(',').map(str::to_owned).collect();
        if values.iter().any(|value| !identifier(value)) {
            return Err(format!("{label}: invalid identifier list"));
        }
        let mut expected = values.clone();
        expected.sort();
        expected.dedup();
        if expected != values {
            return Err(format!("{label}: list must be unique and byte sorted"));
        }
        Ok(values)
    }

    fn meaningful_text(value: &str, label: &str) -> Result<(), String> {
        if value == "-"
            || !value.chars().any(|character| !character.is_whitespace())
            || value.chars().any(char::is_control)
        {
            return Err(format!("{label}: meaningful control-free text is required"));
        }
        Ok(())
    }

    fn language(value: &str) -> bool {
        value == "inherit"
            || (identifier(value)
                && value
                    .split('-')
                    .all(|part| !part.is_empty() && part.len() <= 8))
    }

    fn parse_nonnegative_decimal(value: &str, positive: bool, label: &str) -> Result<(), String> {
        if value.is_empty()
            || value.starts_with('+')
            || value.starts_with('-')
            || value.ends_with('.')
            || value.matches('.').count() > 1
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'.')
        {
            return Err(format!("{label}: invalid canonical decimal {value:?}"));
        }
        let integer = value.split('.').next().unwrap_or_default();
        if integer.len() > 1 && integer.starts_with('0') {
            return Err(format!("{label}: noncanonical leading zero"));
        }
        if positive
            && !value
                .bytes()
                .any(|byte| byte.is_ascii_digit() && byte != b'0')
        {
            return Err(format!("{label}: decimal is outside the permitted range"));
        }
        Ok(())
    }

    fn parse_svg_attributes<'a>(
        source: &'a str,
        label: &str,
    ) -> Result<(&'a str, BTreeMap<&'a str, &'a str>), String> {
        let name_end = source.find(' ').unwrap_or(source.len());
        let name = &source[..name_end];
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(format!("{label}: invalid SVG element name"));
        }
        let mut attributes = BTreeMap::new();
        let mut rest = &source[name_end..];
        while !rest.is_empty() {
            if !rest.starts_with(' ') {
                return Err(format!("{label}: attributes require one ASCII space"));
            }
            rest = &rest[1..];
            if rest.is_empty() || rest.starts_with(' ') {
                return Err(format!("{label}: noncanonical attribute whitespace"));
            }
            let equals = rest
                .find('=')
                .ok_or_else(|| format!("{label}: attribute is missing '='"))?;
            let attribute = &rest[..equals];
            if attribute.is_empty()
                || !attribute
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b':'))
            {
                return Err(format!("{label}: invalid SVG attribute name"));
            }
            rest = &rest[equals + 1..];
            if !rest.starts_with('"') {
                return Err(format!("{label}: attribute values require double quotes"));
            }
            rest = &rest[1..];
            let quote = rest
                .find('"')
                .ok_or_else(|| format!("{label}: unterminated attribute value"))?;
            let value = &rest[..quote];
            if value.is_empty() || value.contains(['<', '>', '&', '\'', '\n', '\t']) {
                return Err(format!("{label}: unsafe or empty attribute value"));
            }
            if attributes.insert(attribute, value).is_some() {
                return Err(format!("{label}: duplicate attribute {attribute}"));
            }
            rest = &rest[quote + 1..];
        }
        Ok((name, attributes))
    }

    fn validate_paint(value: &str, label: &str) -> Result<(), String> {
        if matches!(value, "none" | "currentColor")
            || (value.len() == 7
                && value.starts_with('#')
                && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Ok(());
        }
        Err(format!("{label}: unsupported Safe-SVG 2 paint {value:?}"))
    }

    fn validate_opacity(value: &str, label: &str) -> Result<(), String> {
        let valid = matches!(value, "0" | "1")
            || value.strip_prefix("0.").is_some_and(|fraction| {
                (1..=6).contains(&fraction.len())
                    && fraction.bytes().all(|byte| byte.is_ascii_digit())
            })
            || value.strip_prefix("1.").is_some_and(|fraction| {
                (1..=6).contains(&fraction.len()) && fraction.bytes().all(|byte| byte == b'0')
            });
        if valid {
            Ok(())
        } else {
            Err(format!("{label}: invalid Safe-SVG 2 opacity {value:?}"))
        }
    }

    fn validate_svg_element(
        name: &str,
        attributes: &BTreeMap<&str, &str>,
        is_root: bool,
        self_closing: bool,
        inside_clip: bool,
        state: &mut SafeSvgValidationState,
        label: &str,
    ) -> Result<Option<(i64, i64)>, String> {
        let allowed_tags = ["svg", "g", "defs", "clipPath", "path", "rect", "circle"];
        if !allowed_tags.contains(&name) {
            return Err(format!("{label}: unsupported SVG element <{name}>"));
        }
        if is_root != (name == "svg") {
            return Err(format!("{label}: exactly one outer <svg> is required"));
        }
        let container = matches!(name, "svg" | "g" | "defs" | "clipPath");
        if container == self_closing {
            return Err(format!(
                "{label}: noncanonical container/leaf closing for <{name}>"
            ));
        }

        let paint_attributes = [
            "fill",
            "fill-opacity",
            "stroke",
            "stroke-opacity",
            "stroke-width",
            "stroke-linecap",
            "stroke-linejoin",
            "clip-path",
        ];
        let allowed_attributes: &[&str] = match name {
            "svg" => &["height", "viewBox", "width", "xmlns"],
            "g" => &paint_attributes,
            "defs" => &[],
            "clipPath" => &["id"],
            "path" => &[
                "clip-path",
                "d",
                "fill",
                "fill-opacity",
                "stroke",
                "stroke-linecap",
                "stroke-linejoin",
                "stroke-opacity",
                "stroke-width",
            ],
            "rect" => &[
                "clip-path",
                "fill",
                "fill-opacity",
                "height",
                "stroke",
                "stroke-linecap",
                "stroke-linejoin",
                "stroke-opacity",
                "stroke-width",
                "width",
                "x",
                "y",
            ],
            "circle" => &[
                "clip-path",
                "cx",
                "cy",
                "fill",
                "fill-opacity",
                "r",
                "stroke",
                "stroke-linecap",
                "stroke-linejoin",
                "stroke-opacity",
                "stroke-width",
            ],
            _ => unreachable!(),
        };
        for attribute in attributes.keys() {
            if !allowed_attributes.contains(attribute) {
                return Err(format!(
                    "{label}: unsupported attribute {attribute} on <{name}>"
                ));
            }
        }
        let required_attributes: &[&str] = match name {
            "svg" => &["height", "viewBox", "width", "xmlns"],
            "clipPath" => &["id"],
            "path" => &["d"],
            "rect" => &["height", "width"],
            "circle" => &["cx", "cy", "r"],
            "g" | "defs" => &[],
            _ => unreachable!(),
        };
        if required_attributes
            .iter()
            .any(|attribute| !attributes.contains_key(attribute))
        {
            return Err(format!("{label}: <{name}> is missing a required attribute"));
        }
        if inside_clip
            && attributes
                .keys()
                .any(|attribute| paint_attributes.contains(attribute))
        {
            return Err(format!(
                "{label}: nested clip and paint attributes are forbidden on clip geometry"
            ));
        }

        if let Some(id) = attributes.get("id") {
            if !identifier(id) || !state.ids.insert((*id).to_owned()) {
                return Err(format!("{label}: invalid or duplicate local id {id:?}"));
            }
        }
        for attribute in ["fill", "stroke"] {
            if let Some(value) = attributes.get(attribute) {
                validate_paint(value, label)?;
                state.facts.current_color |= *value == "currentColor";
                state.facts.stroke |= attribute == "stroke" && *value != "none";
            }
        }
        for attribute in ["fill-opacity", "stroke-opacity"] {
            if let Some(value) = attributes.get(attribute) {
                validate_opacity(value, label)?;
                state.facts.fill_opacity |= attribute == "fill-opacity";
                state.facts.stroke_opacity |= attribute == "stroke-opacity";
            }
        }
        if let Some(value) = attributes.get("stroke-width") {
            parse_nonnegative_decimal(value, true, label)?;
        }
        if let Some(value) = attributes.get("stroke-linecap") {
            if !matches!(*value, "butt" | "round" | "square") {
                return Err(format!("{label}: invalid stroke-linecap"));
            }
        }
        if let Some(value) = attributes.get("stroke-linejoin") {
            if !matches!(*value, "bevel" | "miter" | "round") {
                return Err(format!("{label}: invalid stroke-linejoin"));
            }
        }
        if let Some(value) = attributes.get("clip-path") {
            let reference = value
                .strip_prefix("url(#")
                .and_then(|value| value.strip_suffix(')'))
                .filter(|value| identifier(value))
                .ok_or_else(|| format!("{label}: clip-path must be a local fragment"))?;
            if !state.ids.contains(reference) {
                return Err(format!(
                    "{label}: clip-path reference must resolve backward to a local definition"
                ));
            }
            state.references.insert(reference.to_owned());
            state.facts.clip = true;
        }
        if name == "defs" {
            state.facts.definitions = true;
        }
        if let Some(path) = attributes.get("d") {
            if path.is_empty()
                || !path.bytes().all(|byte| {
                    byte.is_ascii_digit()
                        || matches!(
                            byte,
                            b' ' | b','
                                | b'.'
                                | b'+'
                                | b'-'
                                | b'M'
                                | b'm'
                                | b'L'
                                | b'l'
                                | b'H'
                                | b'h'
                                | b'V'
                                | b'v'
                                | b'C'
                                | b'c'
                                | b'Q'
                                | b'q'
                                | b'Z'
                                | b'z'
                        )
                })
            {
                return Err(format!(
                    "{label}: path data is outside the fixed corpus subset"
                ));
            }
        }
        let numeric_attributes: &[&str] = if name == "svg" {
            &[]
        } else {
            &["x", "y", "width", "height", "cx", "cy", "r"]
        };
        for attribute in numeric_attributes {
            if let Some(value) = attributes.get(attribute) {
                parse_nonnegative_decimal(
                    value,
                    matches!(*attribute, "width" | "height" | "r"),
                    label,
                )?;
            }
        }

        if name != "svg" {
            return Ok(None);
        }
        if attributes.len() != 4 || attributes.get("xmlns") != Some(&"http://www.w3.org/2000/svg") {
            return Err(format!(
                "{label}: root namespace and exact geometry are required"
            ));
        }
        let width = attributes
            .get("width")
            .and_then(|value| value.strip_suffix("pt"))
            .ok_or_else(|| format!("{label}: root width must use pt"))?;
        let height = attributes
            .get("height")
            .and_then(|value| value.strip_suffix("pt"))
            .ok_or_else(|| format!("{label}: root height must use pt"))?;
        let width = parse_canonical_i64(width, label)?;
        let height = parse_canonical_i64(height, label)?;
        if width <= 0 || height <= 0 {
            return Err(format!("{label}: intrinsic dimensions must be positive"));
        }
        let view_box: Vec<_> = attributes
            .get("viewBox")
            .ok_or_else(|| format!("{label}: viewBox is required"))?
            .split(' ')
            .collect();
        if view_box != ["0", "0", &width.to_string(), &height.to_string()] {
            return Err(format!("{label}: viewBox must exactly match pt dimensions"));
        }
        Ok(Some((width, height)))
    }

    fn validate_safe_svg2(text: &str, label: &str) -> Result<(i64, i64, SafeSvgFacts), String> {
        if !text.is_ascii() {
            return Err(format!("{label}: canonical lowered SVG must be ASCII"));
        }
        let xml = text
            .strip_suffix('\n')
            .ok_or_else(|| format!("{label}: final LF is required"))?;
        if xml.contains('\n') || xml.contains("<?") || xml.contains("<!") {
            return Err(format!(
                "{label}: declarations, comments, and embedded LF are forbidden"
            ));
        }
        let mut cursor = 0;
        let mut stack: Vec<String> = Vec::new();
        let mut root_geometry = None;
        let mut state = SafeSvgValidationState::default();
        while cursor < xml.len() {
            let start = xml[cursor..]
                .find('<')
                .map(|offset| cursor + offset)
                .ok_or_else(|| format!("{label}: trailing non-element content"))?;
            if !xml[cursor..start].trim().is_empty() {
                return Err(format!("{label}: text nodes are forbidden"));
            }
            let end = xml[start + 1..]
                .find('>')
                .map(|offset| start + 1 + offset)
                .ok_or_else(|| format!("{label}: unterminated element"))?;
            let token = &xml[start + 1..end];
            if let Some(closing) = token.strip_prefix('/') {
                if closing.is_empty() || closing.contains(' ') {
                    return Err(format!("{label}: invalid closing tag"));
                }
                let opened = stack
                    .pop()
                    .ok_or_else(|| format!("{label}: unmatched closing tag"))?;
                if opened != closing {
                    return Err(format!("{label}: closing tag mismatch"));
                }
            } else {
                let (opening, self_closing) = token
                    .strip_suffix('/')
                    .map_or((token, false), |opening| (opening, true));
                let (name, attributes) = parse_svg_attributes(opening, label)?;
                if stack.is_empty() && root_geometry.is_some() {
                    return Err(format!(
                        "{label}: content after the root element is forbidden"
                    ));
                }
                let is_root = root_geometry.is_none() && stack.is_empty();
                let inside_clip = stack.iter().any(|element| element == "clipPath");
                let geometry = validate_svg_element(
                    name,
                    &attributes,
                    is_root,
                    self_closing,
                    inside_clip,
                    &mut state,
                    label,
                )?;
                if let Some(geometry) = geometry {
                    if root_geometry.replace(geometry).is_some() {
                        return Err(format!("{label}: multiple roots are forbidden"));
                    }
                }
                if !self_closing {
                    stack.push(name.to_owned());
                }
            }
            cursor = end + 1;
        }
        if !stack.is_empty() {
            return Err(format!("{label}: unclosed element"));
        }
        if state.references != state.ids {
            return Err(format!(
                "{label}: every local clip definition must be used exactly by name"
            ));
        }
        let (width, height) = root_geometry.ok_or_else(|| format!("{label}: missing root"))?;
        Ok((width, height, state.facts))
    }

    fn validate_dense_ids(ids: &[u32], label: &str) -> Result<(), String> {
        for (expected, actual) in ids.iter().copied().enumerate() {
            if actual != expected as u32 {
                return Err(format!(
                    "{label}: IDs must be unique, dense, and row ordered; expected {expected}, got {actual}"
                ));
            }
        }
        Ok(())
    }

    fn round_half_even_positive(numerator: i128, denominator: i128) -> Result<i128, String> {
        if numerator < 0 || denominator <= 0 {
            return Err("round-half-to-even requires nonnegative/positive operands".to_owned());
        }
        let quotient = numerator / denominator;
        let remainder = numerator % denominator;
        let doubled = remainder
            .checked_mul(2)
            .ok_or_else(|| "round-half-to-even overflow".to_owned())?;
        if doubled < denominator || (doubled == denominator && quotient % 2 == 0) {
            Ok(quotient)
        } else {
            quotient
                .checked_add(1)
                .ok_or_else(|| "round-half-to-even overflow".to_owned())
        }
    }

    fn validate_uniform_viewport(
        viewport_width: i64,
        viewport_height: i64,
        resource: &VmbResource,
        label: &str,
    ) -> Result<(), String> {
        if viewport_width <= 0 || viewport_height <= 0 {
            return Err(format!("{label}: viewport dimensions must be positive"));
        }
        let intrinsic_width = i128::from(resource.intrinsic_width)
            .checked_mul(65_536)
            .ok_or_else(|| format!("{label}: intrinsic width overflow"))?;
        let intrinsic_height = i128::from(resource.intrinsic_height)
            .checked_mul(65_536)
            .ok_or_else(|| format!("{label}: intrinsic height overflow"))?;
        let scale = round_half_even_positive(
            i128::from(viewport_width)
                .checked_mul(65_536)
                .ok_or_else(|| format!("{label}: scale numerator overflow"))?,
            intrinsic_width,
        )?;
        if scale <= 0 || scale > i128::from(i64::MAX) {
            return Err(format!("{label}: 16.16 uniform scale is out of range"));
        }
        let scaled_width = round_half_even_positive(
            intrinsic_width
                .checked_mul(scale)
                .ok_or_else(|| format!("{label}: scaled width overflow"))?,
            65_536,
        )?;
        let scaled_height = round_half_even_positive(
            intrinsic_height
                .checked_mul(scale)
                .ok_or_else(|| format!("{label}: scaled height overflow"))?,
            65_536,
        )?;
        if scaled_width != i128::from(viewport_width)
            || scaled_height != i128::from(viewport_height)
        {
            return Err(format!(
                "{label}: viewport is not the one-scale round-half-to-even result"
            ));
        }
        Ok(())
    }

    fn validate_metric_relations(metrics: &VmbMetrics, label: &str) -> Result<(), String> {
        if metrics.advance <= 0
            || metrics.ascent <= 0
            || metrics.descent < 0
            || metrics.viewport_width <= 0
            || metrics.viewport_height <= 0
            || metrics.baseline < 0
            || metrics.baseline > metrics.viewport_height
        {
            return Err(format!("{label}: metric sign/range invariant failed"));
        }
        if metrics.ascent < metrics.baseline
            || metrics.descent < metrics.viewport_height - metrics.baseline
        {
            return Err(format!(
                "{label}: ascent/descent does not contain the viewport"
            ));
        }
        metrics
            .origin_x
            .checked_add(metrics.viewport_width)
            .ok_or_else(|| format!("{label}: origin_x + viewport_width overflow"))?;
        Ok(())
    }

    fn parse_vmb_resources(root: &Path) -> Result<BTreeMap<u32, VmbResource>, String> {
        let path = root.join("resources.tsv");
        let text = read_canonical_utf8(&path)?;
        let rows = parse_tsv(&text, &VMB_RESOURCE_HEADER, "resources.tsv")?;
        let mut parsed_rows = Vec::new();
        let mut ids = Vec::new();
        let mut bytes_by_hash: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for (row_index, row) in rows.into_iter().enumerate() {
            let label = format!("resources.tsv row {}", row_index + 2);
            let image_id = parse_u32(row[0], &format!("{label} image_id"))?;
            ids.push(image_id);
            if row[1] != "svg-safe-2" {
                return Err(format!("{label}: media_type must be svg-safe-2"));
            }
            if !lowercase_sha256(row[4]) {
                return Err(format!("{label}: invalid expected SHA-256"));
            }
            let expected_uri = format!("math/sha256-{}.svg", row[4]);
            if row[2] != expected_uri {
                return Err(format!("{label}: URI is not derived from content hash"));
            }
            if !row[3].starts_with("svg/") || !row[3].ends_with(".svg") {
                return Err(format!("{label}: svg_path must address the svg directory"));
            }
            let svg_path = contained_file(root, row[3])?;
            let svg_bytes =
                fs::read(&svg_path).map_err(|error| format!("{}: {error}", svg_path.display()))?;
            let svg_text = canonical_utf8(&svg_bytes, &svg_path.display().to_string())?;
            validate_expected_hash(row[4], &svg_bytes, &label)?;
            let (intrinsic_width, intrinsic_height, facts) =
                validate_safe_svg2(svg_text, &svg_path.display().to_string())?;
            if let Some(previous) = bytes_by_hash.get(row[4]) {
                if previous != &svg_bytes {
                    return Err(format!(
                        "{label}: same hash is bound to different stable bytes"
                    ));
                }
            } else {
                bytes_by_hash.insert(row[4].to_owned(), svg_bytes);
            }
            for (field, value) in [
                ("engine_id", row[5]),
                ("engine_version", row[6]),
                ("rules_version", row[7]),
            ] {
                validate_printable_ascii(value, &format!("{label} {field}"))?;
            }
            parsed_rows.push((
                image_id,
                VmbResource {
                    sha256: row[4].to_owned(),
                    provenance: (row[5].to_owned(), row[6].to_owned(), row[7].to_owned()),
                    intrinsic_width,
                    intrinsic_height,
                    facts,
                },
            ));
        }
        validate_dense_ids(&ids, "resources.tsv")?;
        Ok(parsed_rows.into_iter().collect())
    }

    fn parse_vmb_metrics(
        row: &[&str],
        resource: &VmbResource,
        label: &str,
    ) -> Result<VmbMetrics, String> {
        let metrics = VmbMetrics {
            advance: parse_canonical_i64(row[8], &format!("{label} advance"))?,
            ascent: parse_canonical_i64(row[9], &format!("{label} ascent"))?,
            descent: parse_canonical_i64(row[10], &format!("{label} descent"))?,
            origin_x: parse_canonical_i64(row[11], &format!("{label} origin_x"))?,
            baseline: parse_canonical_i64(row[12], &format!("{label} baseline"))?,
            viewport_width: parse_canonical_i64(row[13], &format!("{label} viewport_width"))?,
            viewport_height: parse_canonical_i64(row[14], &format!("{label} viewport_height"))?,
        };
        validate_metric_relations(&metrics, label)?;
        validate_uniform_viewport(
            metrics.viewport_width,
            metrics.viewport_height,
            resource,
            label,
        )?;
        Ok(metrics)
    }

    fn parse_vmb_cases(
        root: &Path,
        resources: &BTreeMap<u32, VmbResource>,
    ) -> Result<BTreeMap<String, VmbCase>, String> {
        let path = root.join("cases.tsv");
        let text = read_canonical_utf8(&path)?;
        let rows = parse_tsv(&text, &VMB_CASE_HEADER, "cases.tsv")?;
        let mut cases = BTreeMap::new();
        let mut previous_id: Option<String> = None;
        for (row_index, row) in rows.into_iter().enumerate() {
            let label = format!("cases.tsv row {}", row_index + 2);
            let case_id = row[0];
            if !identifier(case_id) {
                return Err(format!("{label}: invalid case_id"));
            }
            if previous_id
                .as_deref()
                .is_some_and(|previous| previous >= case_id)
            {
                return Err(format!("{label}: case rows must be unique and sorted"));
            }
            previous_id = Some(case_id.to_owned());
            let kind = row[1];
            if !matches!(
                kind,
                "inline_vector" | "math_vector" | "vector_figure" | "math_vector_block"
            ) {
                return Err(format!("{label}: unknown case kind"));
            }
            let image_id = parse_u32(row[2], &format!("{label} image_id"))?;
            let resource = resources
                .get(&image_id)
                .ok_or_else(|| format!("{label}: unknown image_id {image_id}"))?;
            if row[3] != resource.sha256 {
                return Err(format!("{label}: case/resource hash binding mismatch"));
            }
            meaningful_text(row[5], &format!("{label} alt"))?;
            let is_math = matches!(kind, "math_vector" | "math_vector_block");
            let source_tex = if is_math {
                if row[4] == "-" || !row[4].starts_with("tex/") || !row[4].ends_with(".tex") {
                    return Err(format!("{label}: math source_tex_path is required"));
                }
                let tex_path = contained_file(root, row[4])?;
                let tex = read_canonical_utf8(&tex_path)?;
                if !tex.chars().any(|character| !character.is_whitespace()) {
                    return Err(format!("{label}: source TeX is empty"));
                }
                Some(tex)
            } else {
                if row[4] != "-" {
                    return Err(format!("{label}: generic vector forbids source TeX"));
                }
                None
            };
            let actual_text = if is_math && row[6] != "-" {
                meaningful_text(row[6], &format!("{label} actual_text"))?;
                Some(row[6].to_owned())
            } else if row[6] != "-" {
                return Err(format!("{label}: generic vector forbids actual_text"));
            } else {
                None
            };
            if !language(row[7]) {
                return Err(format!("{label}: invalid language intent"));
            }

            let metrics = if kind == "vector_figure" {
                if row[8..=12].iter().any(|value| *value != "-") {
                    return Err(format!("{label}: vector_figure forbids inline metrics"));
                }
                let viewport_width =
                    parse_canonical_i64(row[13], &format!("{label} viewport_width"))?;
                let viewport_height =
                    parse_canonical_i64(row[14], &format!("{label} viewport_height"))?;
                validate_uniform_viewport(viewport_width, viewport_height, resource, &label)?;
                None
            } else {
                Some(parse_vmb_metrics(&row, resource, &label)?)
            };

            let is_inline = matches!(kind, "inline_vector" | "math_vector");
            let spacing = if is_inline {
                let before = parse_canonical_i64(row[15], &format!("{label} spacing_before"))?;
                let after = parse_canonical_i64(row[16], &format!("{label} spacing_after"))?;
                if before < 0 || after < 0 {
                    return Err(format!("{label}: spacing must be nonnegative"));
                }
                Some((before, after))
            } else if row[15] != "-" || row[16] != "-" {
                return Err(format!("{label}: block spacing belongs to typed style"));
            } else {
                None
            };

            let equation_number = if kind == "math_vector_block" {
                match (row[17], row[18]) {
                    ("-", "-") => None,
                    (number, gap) if number != "-" && gap != "-" => {
                        meaningful_text(number, &format!("{label} equation_number"))?;
                        let gap = parse_canonical_i64(gap, &format!("{label} minimum_gap"))?;
                        if gap <= 0 {
                            return Err(format!("{label}: minimum_gap must be positive"));
                        }
                        Some((number.to_owned(), gap))
                    }
                    _ => return Err(format!("{label}: incomplete equation number binding")),
                }
            } else if row[17] != "-" || row[18] != "-" {
                return Err(format!(
                    "{label}: equation number is only valid on math block"
                ));
            } else {
                None
            };
            let categories = parse_sorted_list(row[19], &format!("{label} categories"))?;
            cases.insert(
                case_id.to_owned(),
                VmbCase {
                    kind: kind.to_owned(),
                    image_id,
                    source_tex,
                    alt: row[5].to_owned(),
                    actual_text,
                    language: row[7].to_owned(),
                    metrics,
                    spacing,
                    equation_number,
                    categories,
                },
            );
        }
        Ok(cases)
    }

    fn fragment_markers(text: &str, label: &str) -> Result<Vec<String>, String> {
        let mut markers = Vec::new();
        let mut rest = text;
        loop {
            let Some(start) = rest.find('{') else {
                if rest.contains('}') {
                    return Err(format!("{label}: unmatched closing brace"));
                }
                break;
            };
            if rest[..start].contains('}') {
                return Err(format!("{label}: unmatched closing brace"));
            }
            let after_start = &rest[start + 1..];
            let end = after_start
                .find('}')
                .ok_or_else(|| format!("{label}: unterminated case marker"))?;
            let marker = &after_start[..end];
            if marker.contains('{') || !identifier(marker) {
                return Err(format!("{label}: invalid case marker"));
            }
            markers.push(marker.to_owned());
            rest = &after_start[end + 1..];
        }
        Ok(markers)
    }

    fn parse_vmb_fragments(
        root: &Path,
        cases: &BTreeMap<String, VmbCase>,
    ) -> Result<BTreeMap<String, VmbFragment>, String> {
        let path = root.join("fragments.tsv");
        let text = read_canonical_utf8(&path)?;
        let rows = parse_tsv(&text, &VMB_FRAGMENT_HEADER, "fragments.tsv")?;
        let mut fragments = BTreeMap::new();
        let mut previous_id: Option<String> = None;
        for (row_index, row) in rows.into_iter().enumerate() {
            let label = format!("fragments.tsv row {}", row_index + 2);
            let fragment_id = row[0];
            if !identifier(fragment_id) {
                return Err(format!("{label}: invalid fragment_id"));
            }
            if previous_id
                .as_deref()
                .is_some_and(|previous| previous >= fragment_id)
            {
                return Err(format!("{label}: fragment rows must be unique and sorted"));
            }
            previous_id = Some(fragment_id.to_owned());
            if !row[1].starts_with("fragments/") || !row[1].ends_with(".txt") {
                return Err(format!("{label}: text_path must address fragments"));
            }
            let text_path = contained_file(root, row[1])?;
            let fragment_text = read_canonical_utf8(&text_path)?;
            let occurrences: Vec<String> = row[2].split(',').map(str::to_owned).collect();
            if occurrences.is_empty()
                || occurrences
                    .iter()
                    .any(|case_id| !identifier(case_id) || !cases.contains_key(case_id))
            {
                return Err(format!("{label}: unknown or invalid occurrence list"));
            }
            let actual_markers = fragment_markers(&fragment_text, &label)?;
            if actual_markers != occurrences {
                return Err(format!(
                    "{label}: ledger occurrence order differs from fragment markers"
                ));
            }
            let inline_remaining_width =
                parse_optional_nonnegative(row[3], &format!("{label} inline_remaining_width"))?;
            let block_values = [
                parse_optional_nonnegative(row[4], &format!("{label} block_frame_width"))?,
                parse_optional_nonnegative(row[5], &format!("{label} block_remaining_height"))?,
                parse_optional_nonnegative(row[6], &format!("{label} next_empty_frame_height"))?,
            ];
            let block_context = match block_values {
                [None, None, None] => None,
                [Some(width), Some(remaining), Some(next)] if width > 0 && next > 0 => {
                    Some((width, remaining, next))
                }
                _ => {
                    return Err(format!(
                        "{label}: block fit context is incomplete or invalid"
                    ))
                }
            };
            let categories = parse_sorted_list(row[7], &format!("{label} categories"))?;
            fragments.insert(
                fragment_id.to_owned(),
                VmbFragment {
                    text: fragment_text,
                    cases: occurrences,
                    inline_remaining_width,
                    block_context,
                    categories,
                },
            );
        }
        Ok(fragments)
    }

    fn case_has_category(case: &VmbCase, category: &str) -> bool {
        case.categories
            .binary_search_by(|candidate| candidate.as_str().cmp(category))
            .is_ok()
    }

    fn fragment_has_category(fragment: &VmbFragment, category: &str) -> bool {
        fragment
            .categories
            .binary_search_by(|candidate| candidate.as_str().cmp(category))
            .is_ok()
    }

    fn require_feature_category(
        corpus: &VmbCorpus,
        category: &str,
        feature: impl Fn(&SafeSvgFacts) -> bool,
    ) -> Result<(), String> {
        let matched = corpus.cases.values().any(|case| {
            case_has_category(case, category)
                && corpus
                    .resources
                    .get(&case.image_id)
                    .is_some_and(|resource| feature(&resource.facts))
        });
        if matched {
            Ok(())
        } else {
            Err(format!(
                "category {category} is not bound to an SVG exhibiting that feature"
            ))
        }
    }

    fn validate_vmb_coverage(corpus: &VmbCorpus) -> Result<(), String> {
        let required_case_ids: BTreeSet<_> = [
            "aligned-block",
            "fraction-equality",
            "generic-block-inherit",
            "generic-block-override",
            "generic-inline-inherit",
            "generic-inline-override",
            "integral",
            "large-brackets",
            "long-block",
            "matrix",
            "not-divides",
            "numbered-aligned",
            "ordered-pair",
            "scripts",
            "similar",
            "sum",
            "x-plus-y",
            "x-plus-y-alias",
        ]
        .into_iter()
        .collect();
        let actual_case_ids: BTreeSet<_> = corpus.cases.keys().map(String::as_str).collect();
        if actual_case_ids != required_case_ids {
            return Err("canonical corpus case set changed without updating the gate".to_owned());
        }

        let mut case_category_counts: BTreeMap<&str, usize> = BTreeMap::new();
        for category in corpus
            .cases
            .values()
            .flat_map(|case| case.categories.iter().map(String::as_str))
        {
            *case_category_counts.entry(category).or_default() += 1;
        }
        for required in [
            "actual-text-alt-fallback",
            "actual-text-authored",
            "aligned",
            "clip",
            "current-color",
            "equation-number",
            "fill-opacity",
            "fraction-equality",
            "integral",
            "language-generic-block-inherit",
            "language-generic-block-override",
            "language-generic-inline-inherit",
            "language-generic-inline-override",
            "language-math-block-inherit",
            "language-math-block-override",
            "language-math-inline-inherit",
            "language-math-inline-override",
            "large-brackets",
            "long-block",
            "matrix",
            "not-divides",
            "ordered-pair",
            "same-content-alias",
            "similar",
            "stroke",
            "stroke-opacity",
            "subscript",
            "sum",
            "superscript",
            "x-plus-y",
        ] {
            if case_category_counts.get(required) != Some(&1) {
                return Err(format!(
                    "required case category {required} must identify exactly one case"
                ));
            }
        }
        let fragment_categories: BTreeSet<_> = corpus
            .fragments
            .values()
            .flat_map(|fragment| fragment.categories.iter().map(String::as_str))
            .collect();
        for required in [
            "brackets",
            "cross-id-alias",
            "dedupe",
            "inline-math",
            "japanese",
            "language-inheritance",
            "language-override",
            "line-end",
            "mixed-heights",
            "page-end",
            "punctuation",
            "ten-use",
        ] {
            if !fragment_categories.contains(required) {
                return Err(format!("missing required fragment category {required}"));
            }
        }

        require_feature_category(corpus, "current-color", |facts| facts.current_color)?;
        require_feature_category(corpus, "stroke", |facts| facts.stroke)?;
        require_feature_category(corpus, "clip", |facts| facts.clip && facts.definitions)?;
        require_feature_category(corpus, "fill-opacity", |facts| facts.fill_opacity)?;
        require_feature_category(corpus, "stroke-opacity", |facts| facts.stroke_opacity)?;

        for (case_id, expected_kind, expected_language, category) in [
            (
                "x-plus-y",
                "math_vector",
                "inherit",
                "language-math-inline-inherit",
            ),
            (
                "similar",
                "math_vector",
                "en",
                "language-math-inline-override",
            ),
            (
                "generic-inline-inherit",
                "inline_vector",
                "inherit",
                "language-generic-inline-inherit",
            ),
            (
                "generic-inline-override",
                "inline_vector",
                "en",
                "language-generic-inline-override",
            ),
            (
                "aligned-block",
                "math_vector_block",
                "inherit",
                "language-math-block-inherit",
            ),
            (
                "long-block",
                "math_vector_block",
                "en",
                "language-math-block-override",
            ),
            (
                "generic-block-inherit",
                "vector_figure",
                "inherit",
                "language-generic-block-inherit",
            ),
            (
                "generic-block-override",
                "vector_figure",
                "en",
                "language-generic-block-override",
            ),
        ] {
            let case = &corpus.cases[case_id];
            if case.kind != expected_kind
                || case.language != expected_language
                || !case_has_category(case, category)
            {
                return Err(format!("{case_id}: kind/language coverage binding changed"));
            }
        }
        if corpus.fragments["language-kinds"].cases
            != [
                "x-plus-y",
                "similar",
                "generic-inline-inherit",
                "generic-inline-override",
                "aligned-block",
                "long-block",
                "generic-block-inherit",
                "generic-block-override",
            ]
        {
            return Err("language-kinds fragment coverage changed".to_owned());
        }
        let fallback = &corpus.cases["x-plus-y"];
        if fallback.actual_text.is_some()
            || !case_has_category(fallback, "actual-text-alt-fallback")
        {
            return Err("x-plus-y: alt fallback binding changed".to_owned());
        }
        let authored = &corpus.cases["similar"];
        if !matches!(authored.actual_text.as_deref(), Some(actual) if actual != authored.alt.as_str())
            || !case_has_category(authored, "actual-text-authored")
        {
            return Err("similar: distinct authored actual text is required".to_owned());
        }
        if !matches!(
            corpus.cases["numbered-aligned"].equation_number.as_ref(),
            Some((number, gap)) if number == "(1)" && *gap == 65_536
        ) || corpus.cases["aligned-block"].equation_number.is_some()
            || corpus.fragments["block-number"].cases != ["numbered-aligned"]
        {
            return Err("numbered/unnumbered block bindings changed".to_owned());
        }

        let exact_tex = [
            ("x-plus-y", r"x+y"),
            ("similar", r"x\sim y"),
            ("not-divides", r"2\nmid 8"),
            ("ordered-pair", r"(a,b)"),
            ("fraction-equality", r"\frac{1}{2}=\frac{2}{4}"),
            ("scripts", r"x_i^2"),
            ("large-brackets", r"\left(\frac{a}{b}\right)"),
            ("matrix", r"\begin{pmatrix}a&b\\c&d\end{pmatrix}"),
            ("sum", r"\sum_{i=1}^{n} i"),
            ("integral", r"\int_0^1 x\,dx"),
        ];
        for (case_id, expected) in exact_tex {
            let actual = corpus.cases[case_id]
                .source_tex
                .as_deref()
                .and_then(|text| text.strip_suffix('\n'))
                .ok_or_else(|| format!("{case_id}: missing canonical source TeX"))?;
            if actual != expected {
                return Err(format!("{case_id}: exact source TeX changed"));
            }
        }
        let aligned = corpus.cases["aligned-block"]
            .source_tex
            .as_deref()
            .ok_or_else(|| "aligned-block: missing source TeX".to_owned())?;
        if !aligned.contains(r"\begin{aligned}") || !aligned.contains(r"\\") {
            return Err("aligned-block: multiple aligned rows are required".to_owned());
        }
        let long_block = corpus.cases["long-block"]
            .source_tex
            .as_deref()
            .ok_or_else(|| "long-block: missing source TeX".to_owned())?;
        if long_block.len() < 70 || !long_block.contains(r"\sum") || !long_block.contains(r"\frac")
        {
            return Err("long-block: long display TeX evidence is required".to_owned());
        }

        let referenced_resources: BTreeSet<_> =
            corpus.cases.values().map(|case| case.image_id).collect();
        if referenced_resources != corpus.resources.keys().copied().collect() {
            return Err("every resource must be referenced by at least one case".to_owned());
        }
        let referenced_cases: BTreeSet<_> = corpus
            .fragments
            .values()
            .flat_map(|fragment| fragment.cases.iter().map(String::as_str))
            .collect();
        if referenced_cases != actual_case_ids {
            return Err("every case must occur in at least one fragment".to_owned());
        }

        let primary_case = &corpus.cases["x-plus-y"];
        let alias_case = &corpus.cases["x-plus-y-alias"];
        if primary_case.image_id == alias_case.image_id {
            return Err("alias evidence requires different logical image IDs".to_owned());
        }
        let primary_resource = &corpus.resources[&primary_case.image_id];
        let alias_resource = &corpus.resources[&alias_case.image_id];
        if primary_resource.sha256 != alias_resource.sha256
            || primary_resource.provenance == alias_resource.provenance
        {
            return Err(
                "alias evidence requires same bytes/hash and different provenance".to_owned(),
            );
        }
        let aliases = &corpus.fragments["dedupe-aliases"];
        if aliases.cases != ["x-plus-y", "x-plus-y-alias"] {
            return Err("dedupe-aliases fragment changed".to_owned());
        }
        let ten_use = &corpus.fragments["dedupe-ten-use"];
        if ten_use.cases.len() != 10 || ten_use.cases.iter().any(|case| case != "x-plus-y") {
            return Err("dedupe-ten-use must place one resource exactly ten times".to_owned());
        }

        let japanese_fragment = &corpus.fragments["japanese-boundaries"];
        if japanese_fragment.cases != ["x-plus-y", "not-divides", "ordered-pair", "similar"] {
            return Err("japanese-boundaries case order changed".to_owned());
        }
        let japanese = &japanese_fragment.text;
        for required in ["日本語", "、", "。", "（", "）", "「", "」"] {
            if !japanese.contains(required) {
                return Err(format!("japanese-boundaries is missing {required:?}"));
            }
        }
        for (fragment_id, fragment) in &corpus.fragments {
            if fragment_id != "line-end" && fragment.inline_remaining_width.is_some() {
                return Err(format!(
                    "{fragment_id}: unexpected inline fit context outside line-end"
                ));
            }
            if fragment_id != "block-page-end" && fragment.block_context.is_some() {
                return Err(format!(
                    "{fragment_id}: unexpected block fit context outside block-page-end"
                ));
            }
        }
        let line_end = &corpus.fragments["line-end"];
        if line_end.cases != ["x-plus-y"] {
            return Err("line-end must bind the x-plus-y case".to_owned());
        }
        let line_case = &corpus.cases["x-plus-y"];
        let line_metrics = line_case
            .metrics
            .ok_or_else(|| "x-plus-y: line-end metrics are required".to_owned())?;
        let (spacing_before, spacing_after) = line_case
            .spacing
            .ok_or_else(|| "x-plus-y: line-end spacing is required".to_owned())?;
        let expected_line_width = line_metrics
            .advance
            .checked_add(spacing_before)
            .ok_or_else(|| "line-end inline occupancy overflow".to_owned())?;
        if line_end.inline_remaining_width != Some(expected_line_width)
            || spacing_after <= 0
            || !fragment_has_category(line_end, "line-end")
        {
            return Err(
                "line-end context must fit advance plus before and suppress positive after"
                    .to_owned(),
            );
        }
        let page_end = &corpus.fragments["block-page-end"];
        if page_end.cases != ["long-block"] {
            return Err("block-page-end must bind the long-block case".to_owned());
        }
        let (frame_width, remaining_height, next_height) = page_end
            .block_context
            .ok_or_else(|| "block-page-end: block fit context is required".to_owned())?;
        let block_metrics = corpus.cases["long-block"]
            .metrics
            .ok_or_else(|| "long-block: metrics are required".to_owned())?;
        let near_width =
            i128::from(block_metrics.viewport_width) * 100 >= i128::from(frame_width) * 90;
        if block_metrics.viewport_width >= frame_width
            || !near_width
            || remaining_height >= block_metrics.viewport_height
            || block_metrics.viewport_height > next_height
            || !fragment_has_category(page_end, "page-end")
        {
            return Err("line/page-end placement evidence is missing".to_owned());
        }
        let mixed = &corpus.fragments["mixed-heights"];
        if mixed.cases
            != [
                "fraction-equality",
                "sum",
                "integral",
                "scripts",
                "large-brackets",
                "matrix",
            ]
        {
            return Err("mixed-heights case order changed".to_owned());
        }
        let line_metrics: BTreeSet<_> = mixed
            .cases
            .iter()
            .map(|case_id| {
                corpus.cases[case_id]
                    .metrics
                    .map(|metrics| (metrics.ascent, metrics.descent))
                    .ok_or_else(|| format!("{case_id}: mixed-height case is missing metrics"))
            })
            .collect::<Result<_, _>>()?;
        if line_metrics.len() < 3 {
            return Err("mixed-heights must exercise varied ascent/descent pairs".to_owned());
        }
        if corpus.cases["generic-block-inherit"].kind != "vector_figure"
            || corpus.cases["aligned-block"].kind != "math_vector_block"
        {
            return Err("generic/math block kind evidence changed".to_owned());
        }
        Ok(())
    }

    fn load_vmb_corpus(root: &Path) -> Result<VmbCorpus, String> {
        let resources = parse_vmb_resources(root)?;
        let cases = parse_vmb_cases(root, &resources)?;
        let fragments = parse_vmb_fragments(root, &cases)?;
        let corpus = VmbCorpus {
            resources,
            cases,
            fragments,
        };
        validate_vmb_coverage(&corpus)?;
        Ok(corpus)
    }

    fn dependency_declarations(contents: &str) -> Vec<(String, String)> {
        let mut in_dependencies = false;
        let mut dependencies = Vec::new();
        for line in contents.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                let section = line.trim_matches(['[', ']']);
                in_dependencies = matches!(
                    section,
                    "dependencies" | "dev-dependencies" | "build-dependencies"
                ) || section.ends_with(".dependencies")
                    || section.ends_with(".dev-dependencies")
                    || section.ends_with(".build-dependencies");
                continue;
            }
            if !in_dependencies || line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((name, declaration)) = line.split_once('=') else {
                continue;
            };
            let name = name.trim();
            dependencies.push((name.to_owned(), declaration.trim().to_owned()));
        }
        dependencies
    }

    fn workspace_dependency_declarations(manifest: &Path) -> Vec<(String, String)> {
        let contents = fs::read_to_string(manifest).expect("workspace manifest must be readable");
        dependency_declarations(&contents)
    }

    fn declared_package_name(name: String, declaration: &str) -> String {
        let compact: String = declaration
            .chars()
            .filter(|character| !character.is_ascii_whitespace())
            .collect();
        let Some(start) = compact.find("package=\"") else {
            return name;
        };
        let package = &compact[start + "package=\"".len()..];
        package
            .split_once('"')
            .map_or(name, |(package, _)| package.to_owned())
    }

    fn is_denied(from: &str, to: &str) -> bool {
        if from == "typaxis-testkit" {
            return false;
        }

        match from {
            "typaxis-host-admission" => return to != "typaxis-core",
            "typaxis-document-package" => return to != "typaxis-core",
            "typaxis-math" => return !matches!(to, "typaxis-core" | "typaxis-font"),
            "typaxis-machine-input" => {
                return !matches!(
                    to,
                    "typaxis-core" | "typaxis-host-admission" | "typaxis-document-package"
                )
            }
            "typaxis-machine-profile" => {
                return !matches!(
                    to,
                    "typaxis-core" | "typaxis-syntax" | "typaxis-diagnostics"
                )
            }
            _ => {}
        }

        if matches!(
            to,
            "typaxis-host-admission"
                | "typaxis-document-package"
                | "typaxis-machine-input"
                | "typaxis-machine-profile"
        ) {
            return !matches!(
                (from, to),
                (
                    "typaxis-syntax",
                    "typaxis-document-package" | "typaxis-machine-input"
                ) | ("typaxis-resource-admission", "typaxis-host-admission")
                    | (
                        "typaxis-manifest",
                        "typaxis-host-admission"
                            | "typaxis-machine-input"
                            | "typaxis-machine-profile"
                    )
                    | (
                        "typaxis-cli",
                        "typaxis-document-package"
                            | "typaxis-machine-input"
                            | "typaxis-machine-profile"
                    )
            );
        }

        if from == "typaxis-cli" {
            return false;
        }

        matches!(
            (from, to),
            ("typaxis-core", _)
                | ("typaxis-document", "typaxis-style")
                | ("typaxis-style", "typaxis-document")
                | (
                    "typaxis-layout-contract",
                    "typaxis-diagnostics"
                        | "typaxis-text"
                        | "typaxis-document"
                        | "typaxis-font"
                        | "typaxis-shaping"
                        | "typaxis-linebreak"
                        | "typaxis-layout"
                        | "typaxis-pagination"
                        | "typaxis-display-list"
                        | "typaxis-resources"
                        | "typaxis-manifest"
                        | "typaxis-pdf"
                )
                | ("typaxis-layout", "typaxis-display-list" | "typaxis-pdf")
                | ("typaxis-pagination", "typaxis-pdf")
                | (
                    "typaxis-pdf",
                    "typaxis-document" | "typaxis-style" | "typaxis-layout" | "typaxis-pagination"
                )
                | (
                    "typaxis-resource-admission",
                    "typaxis-style"
                        | "typaxis-syntax"
                        | "typaxis-layout"
                        | "typaxis-shaping"
                        | "typaxis-display-list"
                        | "typaxis-resources"
                        | "typaxis-pdf"
                )
        )
    }

    fn forbidden_edges(crate_name: &str, manifest: &str) -> Vec<String> {
        dependency_declarations(manifest)
            .into_iter()
            .filter_map(|(name, declaration)| {
                let dependency = declared_package_name(name, &declaration);
                let forbidden_workspace_edge =
                    dependency.starts_with("typaxis-") && is_denied(crate_name, &dependency);
                let forbidden_parser_supply_chain = crate_name == "typaxis-resource-admission"
                    && !dependency.starts_with("typaxis-")
                    && !(dependency == "png" && declaration == "\"=0.18.1\"");
                let forbidden_math_supply_chain =
                    crate_name == "typaxis-math" && !dependency.starts_with("typaxis-");
                (forbidden_workspace_edge
                    || forbidden_parser_supply_chain
                    || forbidden_math_supply_chain)
                    .then(|| format!("{crate_name} -> {dependency}"))
            })
            .collect()
    }

    #[test]
    fn vmb_precomposed_vector_corpus_is_canonical_and_complete() {
        let corpus = load_vmb_corpus(&vmb_corpus_root())
            .unwrap_or_else(|error| panic!("VMB precomposed-vector corpus is invalid: {error}"));
        assert_eq!(corpus.resources.len(), 13);
        assert_eq!(corpus.cases.len(), 18);
        assert_eq!(corpus.fragments.len(), 8);
    }

    #[test]
    fn vmb_precomposed_vector_corpus_rejects_interface_mutants() {
        assert!(!is_portable_relative_path("../outside.svg"));
        assert!(!is_portable_relative_path("svg\\outside.svg"));
        assert!(canonical_utf8(b"missing final LF", "mutant").is_err());
        assert!(canonical_utf8(b"\xef\xbb\xbfvalue\n", "mutant").is_err());
        assert!(canonical_utf8(b"value\0\n", "mutant").is_err());
        assert!(canonical_utf8(b"value\r\n", "mutant").is_err());
        assert!(parse_canonical_i64("01", "mutant").is_err());
        assert!(parse_canonical_i64("+1", "mutant").is_err());
        assert!(parse_canonical_i64("-", "missing metric").is_err());
        assert!(meaningful_text("-", "missing alt").is_err());
        assert!(validate_opacity("0.1234567", "mutant").is_err());
        assert!(validate_opacity("1.1", "mutant").is_err());
        assert!(validate_dense_ids(&[0, 0], "duplicate IDs").is_err());
        assert!(validate_expected_hash(&"0".repeat(64), b"different bytes", "mutant").is_err());

        let invalid_metrics = VmbMetrics {
            advance: 65_536,
            ascent: 32_768,
            descent: 0,
            origin_x: 0,
            baseline: 49_152,
            viewport_width: 65_536,
            viewport_height: 65_536,
        };
        assert!(validate_metric_relations(&invalid_metrics, "mutant").is_err());

        let forbidden_use = concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1pt\" ",
            "height=\"1pt\" viewBox=\"0 0 1 1\"><use href=\"#glyph\"/></svg>\n"
        );
        assert!(validate_safe_svg2(forbidden_use, "mutant").is_err());
        let external_image = concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1pt\" ",
            "height=\"1pt\" viewBox=\"0 0 1 1\"><image ",
            "href=\"https://example.invalid/a.png\"/></svg>\n"
        );
        assert!(validate_safe_svg2(external_image, "mutant").is_err());
        let clip_alpha = concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1pt\" ",
            "height=\"1pt\" viewBox=\"0 0 1 1\"><defs><clipPath id=\"c\">",
            "<rect width=\"1\" height=\"1\" fill-opacity=\"0.5\"/>",
            "</clipPath></defs><rect width=\"1\" height=\"1\" clip-path=\"url(#c)\"/></svg>\n"
        );
        assert!(validate_safe_svg2(clip_alpha, "mutant").is_err());
    }

    #[test]
    fn vmb_safe_svg_negative_corpus_is_closed_and_rejected() {
        let root = vmb_corpus_root();
        let manifest = read_canonical_utf8(&root.join("negative.tsv")).unwrap();
        let rows = parse_tsv(&manifest, &VMB_NEGATIVE_HEADER, "negative.tsv").unwrap();
        let expected_ids: BTreeSet<_> = [
            "clip-alpha",
            "external-image",
            "forbidden-script",
            "invalid-alpha",
            "malformed-unclosed",
            "unsupported-text",
        ]
        .into_iter()
        .collect();
        let actual_ids: BTreeSet<_> = rows.iter().map(|row| row[0]).collect();
        assert_eq!(actual_ids, expected_ids);

        let allowed_reasons = [
            "malformed_svg",
            "forbidden_feature",
            "external_reference",
            "unsupported_feature",
        ];
        let mut previous = None;
        let mut observed_reasons = BTreeSet::new();
        for row in rows {
            let [case_id, expected_reason, svg_path]: [&str; 3] = row.try_into().unwrap();
            assert!(previous.is_none_or(|previous| previous < case_id));
            previous = Some(case_id);
            assert!(allowed_reasons.contains(&expected_reason));
            observed_reasons.insert(expected_reason);
            assert!(is_portable_relative_path(svg_path));
            let svg = read_canonical_utf8(&root.join(svg_path)).unwrap();
            assert!(
                validate_safe_svg2(&svg, case_id).is_err(),
                "negative Safe-SVG 2 case {case_id} was silently accepted"
            );
        }
        assert_eq!(
            observed_reasons,
            allowed_reasons.into_iter().collect::<BTreeSet<_>>()
        );
    }

    #[test]
    fn trusted_pdf_backend_signature_requires_sealed_display_and_resource_plans() {
        let build: fn(
            ValidatedDisplayDocument,
            FrozenPdfResourcePlans,
            &ValidatedResourceLimits,
        ) -> Result<typaxis_pdf::FrozenPdfGraph, PdfError> = PdfBackend::build;
        let _ = build;
    }

    #[test]
    fn forbidden_dependency_edges_are_absent() {
        let crates = workspace_root().join("crates");
        let mut violations = Vec::new();
        for entry in fs::read_dir(crates).expect("workspace crates directory must be readable") {
            let entry = entry.expect("crate directory entry must be readable");
            if !entry
                .file_type()
                .expect("file type must be readable")
                .is_dir()
            {
                continue;
            }
            let crate_name = entry.file_name().to_string_lossy().into_owned();
            let manifest = fs::read_to_string(entry.path().join("Cargo.toml"))
                .expect("workspace manifest must be readable");
            violations.extend(forbidden_edges(&crate_name, &manifest));
        }
        assert!(
            violations.is_empty(),
            "forbidden workspace dependencies: {violations:?}"
        );
    }

    #[test]
    fn forbidden_dependency_edges_exclude_safe_vector_parser_supply_chain() {
        const FORBIDDEN_PACKAGES: [&str; 20] = [
            "cssparser",
            "curl",
            "fantoccini",
            "headless_chrome",
            "html5ever",
            "hyper",
            "libxml",
            "quick-xml",
            "reqwest",
            "resvg",
            "roxmltree",
            "scraper",
            "selenium",
            "surf",
            "svg",
            "ureq",
            "usvg",
            "webkit2gtk",
            "xml-rs",
            "xmltree",
        ];
        let lock = fs::read_to_string(workspace_root().join("Cargo.lock"))
            .expect("workspace lockfile must be readable");
        for package in FORBIDDEN_PACKAGES {
            let lock_entry = format!("name = \"{package}\"");
            assert!(
                !lock.lines().any(|line| line == lock_entry),
                "forbidden SafeVector parser dependency is locked: {package}"
            );
        }

        let source = fs::read_to_string(
            workspace_root().join("crates/typaxis-resource-admission/src/safe_vector.rs"),
        )
        .expect("SafeVector parser source must be readable");
        for forbidden_api in [
            "std::fs",
            "std::net",
            "std::process",
            "Command::",
            "TcpStream",
            "UdpSocket",
            "libloading",
            "extern \"C\"",
        ] {
            assert!(
                !source.contains(forbidden_api),
                "SafeVector parser uses forbidden host API: {forbidden_api}"
            );
        }
    }

    #[test]
    fn forbidden_dependency_edges_pin_the_math_parser_supply_chain() {
        let manifest = fs::read_to_string(workspace_root().join("crates/typaxis-math/Cargo.toml"))
            .expect("math manifest must be readable");
        let mut dependencies: Vec<_> = dependency_declarations(&manifest)
            .into_iter()
            .map(|(name, declaration)| declared_package_name(name, &declaration))
            .collect();
        dependencies.sort();
        assert_eq!(dependencies, ["typaxis-core", "typaxis-font"]);

        let lock = fs::read_to_string(workspace_root().join("Cargo.lock"))
            .expect("workspace lockfile must be readable");
        for package in [
            "harfbuzz_rs",
            "katex",
            "latex2mathml",
            "mathjax",
            "mathml",
            "quick-xml",
            "resvg",
            "roxmltree",
            "tectonic",
            "usvg",
            "xmlparser",
        ] {
            assert!(
                !lock
                    .lines()
                    .any(|line| line == format!("name = \"{package}\"")),
                "forbidden math parser/renderer dependency is locked: {package}"
            );
        }
        let source = fs::read_to_string(workspace_root().join("crates/typaxis-math/src/lib.rs"))
            .expect("math parser source must be readable");
        for forbidden_api in [
            "std::fs",
            "std::net",
            "std::process",
            "Command::",
            "TcpStream",
            "UdpSocket",
            "libloading",
            "extern \"C\"",
        ] {
            assert!(
                !source.contains(forbidden_api),
                "math parser uses forbidden host API: {forbidden_api}"
            );
        }
    }

    #[test]
    fn forbidden_dependency_edges_detect_mutant_manifests() {
        let mutants = [
            (
                "typaxis-machine-input",
                "[dependencies]\ntypaxis-syntax = { path = \"../typaxis-syntax\" }\n",
                "typaxis-machine-input -> typaxis-syntax",
            ),
            (
                "typaxis-host-admission",
                "[dependencies]\ntypaxis-document = { path = \"../typaxis-document\" }\n",
                "typaxis-host-admission -> typaxis-document",
            ),
            (
                "typaxis-document-package",
                "[dependencies]\ntypaxis-host-admission = { path = \"../typaxis-host-admission\" }\n",
                "typaxis-document-package -> typaxis-host-admission",
            ),
            (
                "typaxis-syntax",
                "[dependencies]\ntypaxis-host-admission = { path = \"../typaxis-host-admission\" }\n",
                "typaxis-syntax -> typaxis-host-admission",
            ),
            (
                "typaxis-machine-input",
                "[dependencies]\nsyntax_alias = { package = \"typaxis-syntax\", path = \"../typaxis-syntax\" }\n",
                "typaxis-machine-input -> typaxis-syntax",
            ),
            (
                "typaxis-resource-admission",
                "[dependencies]\nroxmltree = \"=0.20.0\"\n",
                "typaxis-resource-admission -> roxmltree",
            ),
            (
                "typaxis-resource-admission",
                "[dependencies]\npng = \"0.18.1\"\n",
                "typaxis-resource-admission -> png",
            ),
        ];

        for (crate_name, manifest, expected) in mutants {
            assert_eq!(forbidden_edges(crate_name, manifest), [expected]);
        }
    }

    #[test]
    fn host_admission_api_has_only_generic_host_trust_vocabulary() {
        let source = fs::read_to_string(
            workspace_root()
                .join("crates")
                .join("typaxis-host-admission")
                .join("src")
                .join("lib.rs"),
        )
        .expect("host admission source must be readable");

        for required in [
            "OpenedContainedFile",
            "BoundedReadPermit",
            "StableFileBytesReceipt",
            "HostSessionIdentity",
            "HostRootIdentity",
            "HostReadIdentity",
        ] {
            assert!(
                source.contains(required),
                "host admission must expose {required}"
            );
        }

        for forbidden in [
            "FontFaceId",
            "ImageResourceId",
            "ResourceCatalog",
            "ManifestRecord",
            "DiagnosticRecord",
            "CanonicalRecord",
        ] {
            assert!(
                !source.contains(forbidden),
                "generic host admission leaked domain API vocabulary: {forbidden}"
            );
        }

        let public_declarations = source
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with("pub "))
            .collect::<Vec<_>>()
            .join("\n")
            .to_ascii_lowercase();
        for forbidden in ["manifest", "diagnostic", "canonical"] {
            assert!(
                !public_declarations.contains(forbidden),
                "generic host admission exposed a {forbidden} API"
            );
        }
    }

    #[test]
    fn document_package_exclusively_owns_exact_pinned_json_dependencies() {
        const JSON_DEPENDENCIES: [&str; 4] = [
            "serde",
            "serde_json",
            "serde_path_to_error",
            "serde_stacker",
        ];

        let crates = workspace_root().join("crates");
        let mut found = Vec::new();
        for entry in fs::read_dir(crates).expect("workspace crates directory must be readable") {
            let entry = entry.expect("crate directory entry must be readable");
            if !entry
                .file_type()
                .expect("file type must be readable")
                .is_dir()
            {
                continue;
            }
            let crate_name = entry.file_name().to_string_lossy().into_owned();
            let manifest = entry.path().join("Cargo.toml");
            for (name, declaration) in workspace_dependency_declarations(&manifest) {
                let dependency = declared_package_name(name, &declaration);
                if !JSON_DEPENDENCIES.contains(&dependency.as_str()) {
                    continue;
                }
                assert_eq!(
                    crate_name, "typaxis-document-package",
                    "{dependency} must only be a direct dependency of typaxis-document-package"
                );
                assert!(
                    declaration.contains("\"="),
                    "{dependency} must use an exact version pin: {declaration}"
                );
                found.push(dependency);
            }
        }
        found.sort();
        assert_eq!(
            found,
            [
                "serde",
                "serde_json",
                "serde_path_to_error",
                "serde_stacker"
            ]
        );

        let manifest = fs::read_to_string(
            workspace_root()
                .join("crates")
                .join("typaxis-document-package")
                .join("Cargo.toml"),
        )
        .expect("DocumentPackage manifest must be readable");
        let declarations = dependency_declarations(&manifest);
        let serde = declarations
            .iter()
            .find(|(name, _)| name == "serde")
            .expect("serde dependency must exist");
        assert!(serde.1.contains("\"derive\""));
        let serde_json = declarations
            .iter()
            .find(|(name, _)| name == "serde_json")
            .expect("serde_json dependency must exist");
        assert!(serde_json.1.contains("\"unbounded_depth\""));
    }
}
