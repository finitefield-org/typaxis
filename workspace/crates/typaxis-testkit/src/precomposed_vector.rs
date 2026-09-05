use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use typaxis_core::{
    sha256, ConfigResourceRoot, DocumentPackageContractId, EffectiveConfig, EffectiveDataVersions,
    HostAdmissionContext, HostPath, ImageResourceId, Length, M4EffectiveResourceLimits,
    M4ResourceLimits, NonNegativeLength, PdfStreamCompression, PortablePath, PositiveLength, Rect,
    ResourceLimits, ValidatedResourceLimits, DEFAULT_ALLOWED_URI_SCHEMES,
};
use typaxis_manifest::{
    build_figure_vector_v2_manifests, build_vector_v2_manifests, manifest_figure_vector_v2_fixture,
    manifest_vector_v2_fixture, StagingProductionBuildManifestVectorFields,
};
use typaxis_pdf::{
    build_staging_safe_vector_pdf_contribution_v2,
    staging_safe_vector_accessible_isolated_pdf_fixture_v2,
    staging_safe_vector_isolated_pdf_fixture_v2, StagingSafeVectorIsolatedRoleV2,
    StagingSafeVectorIsolatedSemanticUseV2,
};
use typaxis_resources::{
    finalize_staging_safe_vector_forms_v2, staging_declared_base_catalog, AdmittedResourceLedger,
    AdmittedResourceResolver, HostResourceAdmissionSession, ResourceAdmissionError,
    SafeVectorFailureReason, VectorContentCandidateRegistry,
};
use typaxis_syntax::PrecomposedVectorKind;

/// Owner-private completion order used only to prove that artifact collection
/// does not leak worker scheduling into canonical output bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrecomposedVectorBuildSchedule {
    Forward,
    ReverseCompletion,
}

/// Complete generated outputs of the contract-1.4 production-vector closure.
///
/// The map is keyed by portable artifact name and therefore has one canonical
/// order regardless of the owner-private completion schedule used to collect
/// its members. No member contains a host path, wall-clock value, locale, or
/// filesystem enumeration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrecomposedVectorArtifactSet {
    files: BTreeMap<String, Vec<u8>>,
}

impl PrecomposedVectorArtifactSet {
    pub fn files(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.files
    }

    pub fn file(&self, name: &str) -> Option<&[u8]> {
        self.files.get(name).map(Vec::as_slice)
    }

    pub fn canonical_digest(&self) -> [u8; 32] {
        let mut bytes = Vec::new();
        for (name, payload) in &self.files {
            bytes.extend_from_slice(&(name.len() as u64).to_be_bytes());
            bytes.extend_from_slice(name.as_bytes());
            bytes.extend_from_slice(&(payload.len() as u64).to_be_bytes());
            bytes.extend_from_slice(payload);
        }
        sha256(&bytes)
    }
}

/// Run the production fixture through the complete, receipt-gated
/// chain and return only generated artifacts. The fixture constructors invoke
/// the real Wire, syntax/profile, admission, layout, Display, Form planning,
/// structure, final tagged-PDF, observation, and manifest implementations.
pub fn build_precomposed_vector_artifacts(
    schedule: PrecomposedVectorBuildSchedule,
) -> Result<PrecomposedVectorArtifactSet, Box<dyn std::error::Error>> {
    let fixture = manifest_vector_v2_fixture()?;
    let effective_package =
        typaxis_syntax::machine_profile_boundary::wire::StagingSemanticDocumentPackageEncoder::new(
        )
        .encode(fixture.display.layout.package.checked_wire()?)?;
    let effective_package_sha256 = sha256(effective_package.as_bytes());
    let products = build_vector_v2_manifests(&fixture)?;
    let root = StagingProductionBuildManifestVectorFields::built(
        &products.book,
        &products.safe,
        &products.math,
        &products.tagged,
    )?;

    // Re-run the legacy Figure closure through the same /2 final path. It is
    // retained as a separate generated artifact so the private integration
    // gate catches regressions in the existing Figure implementation too.
    let figure_fixture = manifest_figure_vector_v2_fixture()?;
    let figure_products = build_figure_vector_v2_manifests(&figure_fixture)?;
    let figure_root = StagingProductionBuildManifestVectorFields::built(
        &figure_products.book,
        &figure_products.safe,
        &figure_products.math,
        &figure_products.tagged,
    )?;

    // The ten-use closure is deliberately assertion-only: it proves one Form
    // XObject is invoked ten times without being mistaken for a second
    // production manifest owner.
    let ten = typaxis_display_list::staging_precomposed_vector_display_ten_use_fixture()?;
    let ten_candidates = VectorContentCandidateRegistry::from_admitted(
        &ten.layout.admitted,
        ten.layout.package.resources(),
    )?;
    let ten_plans =
        finalize_staging_safe_vector_forms_v2(&ten.display, &ten_candidates, &ten.layout.limits)?;
    let ten_contribution = build_staging_safe_vector_pdf_contribution_v2(
        &ten.display,
        &ten_plans,
        &ten_candidates,
        &ten.layout.limits,
    )?;
    let ten_pdf =
        staging_safe_vector_isolated_pdf_fixture_v2(&ten_contribution, 240 * 65_536, 140 * 65_536)?;

    // Re-admit two logical image IDs with identical stable bytes and distinct
    // producer provenance, then paint both aliases through one content-key
    // Form. This is generated evidence, not only a ledger assertion.
    let alias_base = typaxis_display_list::staging_precomposed_vector_display_fixture()?;
    let mut alias_declarations = alias_base.layout.package.resources().clone();
    alias_declarations.font_faces.clear();
    let first_alias = alias_declarations.images[0].clone();
    let second_alias = alias_declarations
        .images
        .get_mut(1)
        .ok_or("precomposed alias fixture requires logical image ID 1")?;
    second_alias.uri = first_alias.uri;
    second_alias.expected_sha256 = first_alias.expected_sha256;
    second_alias.media = first_alias.media;
    second_alias.vector_provenance = first_alias.vector_provenance;
    second_alias
        .vector_provenance
        .as_mut()
        .ok_or("Safe-SVG 2 alias provenance is missing")?
        .engine_id = "vmb.texToSvg.cache-replay".to_owned();
    let corpus_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("samples/machine-package/staging/production-book-1/precomposed-vector");
    let (corpus_admitted, corpus_candidates) = admit_full_vector_corpus(
        &corpus_root,
        alias_base.layout.package.resources(),
        &alias_base.layout.limits,
        alias_base.layout.profile.profile_fingerprint(),
        schedule == PrecomposedVectorBuildSchedule::ReverseCompletion,
    )?;
    let corpus = build_all_category_corpus_pdf(
        &corpus_root,
        &alias_base,
        &corpus_admitted,
        &corpus_candidates,
    )?;
    let alias_admitted = admit_vector_catalog(
        &corpus_root,
        &corpus_root.join("document-package.json"),
        &alias_declarations,
        &alias_base.layout.limits,
        alias_base.layout.profile.profile_fingerprint(),
    )?;
    let alias_candidates =
        VectorContentCandidateRegistry::staging_from_admitted_with_reverse_completion(
            &alias_admitted,
            &alias_declarations,
            schedule == PrecomposedVectorBuildSchedule::ReverseCompletion,
        )?;
    let alias_display =
        typaxis_display_list::staging_precomposed_vector_display_two_alias_use_fixture()?;
    let alias_plans = finalize_staging_safe_vector_forms_v2(
        &alias_display,
        &alias_candidates,
        &alias_base.layout.limits,
    )?;
    let alias_contribution = build_staging_safe_vector_pdf_contribution_v2(
        &alias_display,
        &alias_plans,
        &alias_candidates,
        &alias_base.layout.limits,
    )?;
    let alias_pdf = staging_safe_vector_isolated_pdf_fixture_v2(
        &alias_contribution,
        240 * 65_536,
        140 * 65_536,
    )?;
    let alias_candidate = alias_candidates
        .candidates()
        .first()
        .ok_or("precomposed alias fixture has no content candidate")?;
    let alias_count = alias_candidate.aliases().len();
    let provenance_count = alias_candidate
        .aliases()
        .iter()
        .filter(|alias| alias.provenance().producer().is_some())
        .count();

    let pdf = fixture.pdf.bytes();
    let observation = fixture.pdf.observation();
    let figure_pdf = figure_fixture.pdf.bytes();
    let figure_observation = figure_fixture.pdf.observation();
    let phase_receipts = phase_receipts_json();
    let tagged_pdf_expectation = tagged_pdf_expectation_json(observation);
    let verification = verification_json(
        fixture.display.display.receipt().page_count(),
        products.safe.resources().len(),
        products.safe.placement_count(),
        products.math.facts().len(),
        products.tagged.vector_structures().len(),
        observation.form_object_count(),
        observation.vector_usage_count(),
        observation.object_count(),
        sha256(pdf),
        ten_contribution.forms().len(),
        ten_contribution.usages().len(),
        sha256(ten_pdf.bytes()),
        alias_contribution.forms().len(),
        alias_contribution.usages().len(),
        alias_count,
        provenance_count,
        sha256(alias_pdf.bytes()),
        corpus.form_count,
        corpus.do_count,
        corpus.page_count,
        sha256(corpus.pdf.bytes()),
        effective_package_sha256,
        figure_observation.form_object_count(),
        figure_observation.vector_usage_count(),
        figure_products.tagged.vector_structures().len(),
        sha256(figure_pdf),
    );

    let mut pending = vec![
        (
            "block-layout-trace.json",
            line(
                fixture
                    .display
                    .block_selected
                    .trace_json(&fixture.display.layout.layout),
            ),
        ),
        (
            "book-navigation-manifest.json",
            line(products.book.canonical_jcs().to_owned()),
        ),
        (
            "build-manifest-vector.json",
            line(root.canonical_root_projection()),
        ),
        (
            "corpus-admission.json",
            line(corpus_candidates.receipt().canonical_jcs().to_owned()),
        ),
        ("corpus-display.json", line(corpus.display_trace)),
        ("corpus-output.pdf", corpus.pdf.bytes().to_vec()),
        ("dedupe-ten-use.pdf", ten_pdf.bytes().to_vec()),
        ("dedupe-two-alias.pdf", alias_pdf.bytes().to_vec()),
        (
            "display-v2.json",
            line(fixture.display.display.trace_json()),
        ),
        ("effective-document-package.json", line(effective_package)),
        (
            "figure-build-manifest-vector.json",
            line(figure_root.canonical_root_projection()),
        ),
        ("figure-output.pdf", figure_pdf.to_vec()),
        (
            "inline-layout-trace.json",
            line(fixture.display.inline_selected.trace_json()),
        ),
        (
            "math-vector-manifest.json",
            line(products.math.canonical_jcs().to_owned()),
        ),
        ("output.pdf", pdf.to_vec()),
        (
            "pdf-observation.json",
            line(observation.canonical_jcs().to_owned()),
        ),
        ("phase-receipts.json", line(phase_receipts)),
        (
            "safe-vector-manifest.json",
            line(products.safe.canonical_jcs().to_owned()),
        ),
        (
            "tagged-pdf-manifest.json",
            line(products.tagged.canonical_jcs().to_owned()),
        ),
        ("tagged-pdf-expectation.json", line(tagged_pdf_expectation)),
        ("verification.json", line(verification)),
    ];
    if schedule == PrecomposedVectorBuildSchedule::ReverseCompletion {
        pending.reverse();
    }
    let mut files = BTreeMap::new();
    for (name, payload) in pending {
        if files.insert(name.to_owned(), payload).is_some() {
            return Err(format!("duplicate private artifact name {name}").into());
        }
    }
    let index = artifact_index_json(&files);
    files.insert("artifact-index.json".to_owned(), line(index));
    Ok(PrecomposedVectorArtifactSet { files })
}

/// Writer-independent `/2` validator input for the closed publication
/// fixture. Object hashes come from the sealed final-PDF observation while
/// semantic expectations remain authored facts: deriving alternatives,
/// languages, MCIDs, or Formula/number parentage by parsing the PDF would let
/// the writer define its own oracle.
fn tagged_pdf_expectation_json(observation: &typaxis_pdf::TaggedPdfObservationV2) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_string(&mut output, observation.validator_algorithm());
    output.push_str(",\"document_language\":");
    push_string(&mut output, observation.document_language());
    output.push_str(concat!(
        ",\"equation_numbers\":[{\"exact_text\":\"(1)\",\"font_index\":0,",
        "\"mcid\":2,\"page_index\":1,\"paint_language\":\"en-US\",",
        "\"parent_structure_node_id\":6,\"structure_language\":null,",
        "\"structure_node_id\":7}],\"form_count\":"
    ));
    output.push_str(&observation.form_object_count().to_string());
    output.push_str(",\"object_budget_charge_count\":");
    output.push_str(&observation.object_budget_charge_count().to_string());
    output.push_str(",\"observation_algorithm\":");
    push_string(&mut output, observation.algorithm());
    output.push_str(",\"page_count\":2,\"pdf\":{\"byte_length\":");
    output.push_str(&observation.pdf_byte_length().to_string());
    output.push_str(",\"object_count\":");
    output.push_str(&observation.object_count().to_string());
    output.push_str(",\"objects\":[");
    for (index, object) in observation.objects().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"object_number\":");
        output.push_str(&object.object_number().to_string());
        output.push_str(",\"role\":");
        push_string(&mut output, object.role());
        output.push_str(",\"sha256\":\"");
        output.push_str(&hex(object.sha256()));
        output.push_str("\"}");
    }
    output.push_str("],\"sha256\":\"");
    output.push_str(&hex(observation.pdf_sha256()));
    output.push_str(concat!(
        "\"},\"vectors\":[",
        "{\"actual_text\":null,\"alternative\":\"丸括弧で囲んだ二項目\",",
        "\"form_index\":0,\"kind\":\"inline_vector\",\"mcid\":0,",
        "\"page_index\":0,\"paint_language\":\"en-US\",",
        "\"structure_language\":\"en-US\",\"structure_node_id\":3},",
        "{\"actual_text\":\"xたすy\",\"alternative\":\"xたすy\",",
        "\"form_index\":0,\"kind\":\"math_vector\",\"mcid\":1,",
        "\"page_index\":0,\"paint_language\":\"en-US\",",
        "\"structure_language\":\"en-US\",\"structure_node_id\":4},",
        "{\"actual_text\":null,\"alternative\":\"配置図\",",
        "\"form_index\":0,\"kind\":\"vector_figure\",\"mcid\":0,",
        "\"page_index\":1,\"paint_language\":\"en-US\",",
        "\"structure_language\":\"en-US\",\"structure_node_id\":5},",
        "{\"actual_text\":\"xたすy、式1\",\"alternative\":\"xたすy、式1\",",
        "\"form_index\":0,\"kind\":\"math_vector_block\",\"mcid\":1,",
        "\"page_index\":1,\"paint_language\":\"en-US\",",
        "\"structure_language\":\"en-US\",\"structure_node_id\":6}],",
        "\"xmp_sha256\":\""
    ));
    output.push_str(&hex(observation.xmp_sha256()));
    output.push_str("\"}");
    output
}

#[derive(Clone, Debug)]
struct CorpusVectorCase {
    kind: PrecomposedVectorKind,
    image_id: ImageResourceId,
    alt: String,
    actual_text: String,
    language: String,
    advance: Option<i64>,
    origin_x: Option<i64>,
    baseline: Option<i64>,
    viewport_width: i64,
    viewport_height: i64,
    spacing_before: Option<i64>,
    spacing_after: Option<i64>,
}

struct AllCategoryCorpusPdf {
    display_trace: String,
    pdf: typaxis_pdf::StagingSafeVectorIsolatedPdfFixtureV2,
    form_count: usize,
    do_count: usize,
    page_count: usize,
}

fn build_all_category_corpus_pdf(
    corpus_root: &Path,
    base: &typaxis_display_list::StagingPrecomposedVectorDisplayFixture,
    admitted: &AdmittedResourceLedger,
    candidates: &VectorContentCandidateRegistry,
) -> Result<AllCategoryCorpusPdf, Box<dyn std::error::Error>> {
    const SCALE: i64 = 65_536;
    const PAGE_WIDTH: i64 = 500 * SCALE;
    const PAGE_HEIGHT: i64 = 600 * SCALE;
    const LEFT: i64 = 20 * SCALE;
    const RIGHT: i64 = 480 * SCALE;
    const ROW_STEP: i64 = 60 * SCALE;

    let cases = read_corpus_cases(corpus_root)?;
    let fragment_rows = read_exact_tsv(
        &corpus_root.join("fragments.tsv"),
        &[
            "fragment_id",
            "text_path",
            "cases",
            "inline_remaining_width",
            "block_frame_width",
            "block_remaining_height",
            "next_empty_frame_height",
            "categories",
        ],
    )?;
    let mut inputs = Vec::new();
    let mut semantics = Vec::new();
    let mut page_trailing_text = Vec::new();
    for (page_index, row) in fragment_rows.iter().enumerate() {
        let fragment = fs::read_to_string(corpus_root.join(&row[1]))?;
        let (preceding, marker_ids, trailing) = split_fragment_markers(&fragment)?;
        let declared_ids = row[2].split(',').collect::<Vec<_>>();
        if marker_ids
            .iter()
            .map(String::as_str)
            .ne(declared_ids.iter().copied())
        {
            return Err(format!("{} marker order differs from fragments.tsv", row[0]).into());
        }
        let page_index = u32::try_from(page_index)?;
        let mut inline_x = LEFT;
        let mut inline_baseline_y = 60 * SCALE;
        let mut block_y = 150 * SCALE;
        for (paint_ordinal, ((case_id, text_before), declared_id)) in marker_ids
            .iter()
            .zip(&preceding)
            .zip(declared_ids)
            .enumerate()
        {
            if case_id != declared_id {
                return Err(format!("{} occurrence binding differs", row[0]).into());
            }
            let case = cases
                .get(case_id)
                .ok_or_else(|| format!("unknown corpus case {case_id}"))?;
            let is_inline = matches!(
                case.kind,
                PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::MathVector
            );
            let (x, y, pen_origin_x, baseline) = if is_inline {
                let advance = case.advance.ok_or("inline corpus case has no advance")?;
                let before = case
                    .spacing_before
                    .ok_or("inline corpus case has no spacing.before")?;
                let after = case
                    .spacing_after
                    .ok_or("inline corpus case has no spacing.after")?;
                let occupied_end = |start: i64| {
                    start
                        .checked_add(before)
                        .and_then(|value| value.checked_add(advance))
                        .and_then(|value| value.checked_add(after))
                };
                if occupied_end(inline_x).ok_or("inline corpus advance overflow")? > RIGHT {
                    inline_x = LEFT;
                    inline_baseline_y = inline_baseline_y
                        .checked_add(ROW_STEP)
                        .ok_or("inline corpus baseline overflow")?;
                }
                let next_inline_x = occupied_end(inline_x)
                    .filter(|value| *value <= RIGHT)
                    .ok_or("inline corpus item exceeds the line width")?;
                let baseline = case.baseline.ok_or("inline corpus case has no baseline")?;
                let origin_x = case.origin_x.ok_or("inline corpus case has no origin_x")?;
                let pen_origin_x = inline_x
                    .checked_add(before)
                    .ok_or("inline corpus spacing overflow")?;
                let x = pen_origin_x
                    .checked_add(origin_x)
                    .ok_or("inline corpus x overflow")?;
                let y = inline_baseline_y
                    .checked_sub(baseline)
                    .ok_or("inline corpus y overflow")?;
                inline_x = next_inline_x;
                (x, y, Some(pen_origin_x), Some(baseline))
            } else {
                let x = PAGE_WIDTH
                    .checked_sub(case.viewport_width)
                    .and_then(|value| value.checked_div(2))
                    .ok_or("block corpus width exceeds the page")?;
                let y = block_y;
                block_y = block_y
                    .checked_add(case.viewport_height)
                    .and_then(|value| value.checked_add(20 * SCALE))
                    .ok_or("block corpus position overflow")?;
                if block_y > PAGE_HEIGHT {
                    return Err(format!("{} block corpus page overflow", row[0]).into());
                }
                match case.kind {
                    PrecomposedVectorKind::MathVectorBlock => (
                        x,
                        y,
                        Some(
                            x.checked_sub(case.origin_x.ok_or("block corpus has no origin_x")?)
                                .ok_or("block corpus pen origin overflow")?,
                        ),
                        case.baseline,
                    ),
                    PrecomposedVectorKind::VectorFigure => (x, y, None, None),
                    _ => unreachable!(),
                }
            };
            let viewport = Rect::new(
                Length::from_raw(x).ok_or("invalid corpus viewport x")?,
                Length::from_raw(y).ok_or("invalid corpus viewport y")?,
                PositiveLength::new(
                    Length::from_raw(case.viewport_width).ok_or("invalid corpus viewport width")?,
                )
                .ok_or("nonpositive corpus viewport width")?,
                PositiveLength::new(
                    Length::from_raw(case.viewport_height)
                        .ok_or("invalid corpus viewport height")?,
                )
                .ok_or("nonpositive corpus viewport height")?,
            );
            inputs.push(
                typaxis_display_list::StagingPrecomposedVectorCorpusDisplayInput::new(
                    case.kind,
                    case.image_id,
                    page_index,
                    u32::try_from(paint_ordinal)?,
                    viewport,
                    pen_origin_x
                        .map(|value| Length::from_raw(value).ok_or("invalid corpus pen origin"))
                        .transpose()?,
                    baseline
                        .map(|value| {
                            Length::from_raw(value)
                                .and_then(NonNegativeLength::new)
                                .ok_or("invalid corpus baseline")
                        })
                        .transpose()?,
                ),
            );
            semantics.push(StagingSafeVectorIsolatedSemanticUseV2::new(
                u32::try_from(semantics.len())?,
                if matches!(
                    case.kind,
                    PrecomposedVectorKind::MathVector | PrecomposedVectorKind::MathVectorBlock
                ) {
                    StagingSafeVectorIsolatedRoleV2::Formula
                } else {
                    StagingSafeVectorIsolatedRoleV2::Figure
                },
                text_before.clone(),
                case.actual_text.clone(),
                case.language.clone(),
            ));
        }
        page_trailing_text.push(trailing);
    }
    let display = typaxis_display_list::staging_precomposed_vector_corpus_display_fixture(
        base, admitted, &inputs,
    )?;
    let plans = finalize_staging_safe_vector_forms_v2(&display, candidates, &base.layout.limits)?;
    let contribution = build_staging_safe_vector_pdf_contribution_v2(
        &display,
        &plans,
        candidates,
        &base.layout.limits,
    )?;
    let pdf = staging_safe_vector_accessible_isolated_pdf_fixture_v2(
        &contribution,
        PAGE_WIDTH,
        PAGE_HEIGHT,
        &semantics,
        &page_trailing_text,
        "ja",
    )?;
    Ok(AllCategoryCorpusPdf {
        display_trace: display.trace_json(),
        form_count: contribution.forms().len(),
        do_count: contribution.usages().len(),
        page_count: display.pages().len(),
        pdf,
    })
}

fn read_corpus_cases(
    corpus_root: &Path,
) -> Result<BTreeMap<String, CorpusVectorCase>, Box<dyn std::error::Error>> {
    let rows = read_exact_tsv(
        &corpus_root.join("cases.tsv"),
        &[
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
        ],
    )?;
    let mut cases = BTreeMap::new();
    for row in rows {
        let kind = match row[1].as_str() {
            "inline_vector" => PrecomposedVectorKind::InlineVector,
            "math_vector" => PrecomposedVectorKind::MathVector,
            "vector_figure" => PrecomposedVectorKind::VectorFigure,
            "math_vector_block" => PrecomposedVectorKind::MathVectorBlock,
            _ => return Err(format!("{} has an unknown vector kind", row[0]).into()),
        };
        let optional = |value: &str| -> Result<Option<i64>, Box<dyn std::error::Error>> {
            Ok((value != "-").then(|| value.parse()).transpose()?)
        };
        let case = CorpusVectorCase {
            kind,
            image_id: ImageResourceId::new(row[2].parse()?),
            alt: row[5].clone(),
            actual_text: if row[6] == "-" {
                row[5].clone()
            } else {
                row[6].clone()
            },
            language: if row[7] == "inherit" {
                "ja".to_owned()
            } else {
                row[7].clone()
            },
            advance: optional(&row[8])?,
            origin_x: optional(&row[11])?,
            baseline: optional(&row[12])?,
            viewport_width: row[13].parse()?,
            viewport_height: row[14].parse()?,
            spacing_before: optional(&row[15])?,
            spacing_after: optional(&row[16])?,
        };
        if case.alt.trim().is_empty()
            || case.actual_text.trim().is_empty()
            || case.viewport_width <= 0
            || case.viewport_height <= 0
            || cases.insert(row[0].clone(), case).is_some()
        {
            return Err(format!("{} corpus case is invalid or duplicated", row[0]).into());
        }
    }
    Ok(cases)
}

fn read_exact_tsv(
    path: &Path,
    header: &[&str],
) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let payload = fs::read(path)?;
    if !payload.ends_with(b"\n") || payload.contains(&b'\r') {
        return Err(format!("{} is not canonical LF TSV", path.display()).into());
    }
    let text = String::from_utf8(payload)?;
    let mut lines = text[..text.len() - 1].split('\n');
    if lines
        .next()
        .map(|line| line.split('\t').collect::<Vec<_>>())
        != Some(header.to_vec())
    {
        return Err(format!("{} has an unexpected TSV header", path.display()).into());
    }
    let rows = lines
        .map(|line| line.split('\t').map(str::to_owned).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    if rows.is_empty()
        || rows
            .iter()
            .any(|row| row.len() != header.len() || row.iter().any(String::is_empty))
    {
        return Err(format!("{} has an invalid TSV row", path.display()).into());
    }
    Ok(rows)
}

type FragmentMarkerSplit = (Vec<String>, Vec<String>, String);

fn split_fragment_markers(text: &str) -> Result<FragmentMarkerSplit, Box<dyn std::error::Error>> {
    let mut preceding = Vec::new();
    let mut markers = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative_open) = text[cursor..].find('{') {
        let open = cursor + relative_open;
        let close = text[open + 1..]
            .find('}')
            .map(|offset| open + 1 + offset)
            .ok_or("unterminated corpus fragment marker")?;
        preceding.push(text[cursor..open].to_owned());
        let marker = &text[open + 1..close];
        if marker.is_empty()
            || !marker
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err("invalid corpus fragment marker".into());
        }
        markers.push(marker.to_owned());
        cursor = close + 1;
    }
    if markers.is_empty() || text[cursor..].contains('}') {
        return Err("corpus fragment contains an invalid marker".into());
    }
    Ok((preceding, markers, text[cursor..].to_owned()))
}

fn admit_full_vector_corpus(
    corpus_root: &Path,
    template: &typaxis_document::StagingM4ResourceCatalog,
    limits: &M4EffectiveResourceLimits,
    profile_fingerprint: [u8; 32],
    reverse_completion: bool,
) -> Result<(AdmittedResourceLedger, VectorContentCandidateRegistry), Box<dyn std::error::Error>> {
    const SVG_NAMES: [&str; 13] = [
        "x-plus-y",
        "x-plus-y",
        "similar",
        "not-divides",
        "ordered-pair",
        "fraction-equality",
        "sum",
        "integral",
        "scripts",
        "large-brackets",
        "matrix",
        "aligned",
        "long-block",
    ];

    let declaration_template = template
        .images
        .first()
        .ok_or("precomposed corpus declaration template is missing")?;
    let mut declarations = template.clone();
    declarations.font_faces.clear();
    declarations.images.clear();
    declarations.images.try_reserve_exact(SVG_NAMES.len())?;
    for (index, name) in SVG_NAMES.iter().enumerate() {
        let mut declaration = declaration_template.clone();
        declaration.image_id = ImageResourceId::new(u32::try_from(index)?);
        declaration.uri = PortablePath::new(format!("svg/{name}.svg"))
            .map_err(|error| format!("invalid corpus resource path: {error:?}"))?;
        declaration.expected_sha256 = Some(sha256(&fs::read(
            corpus_root.join(declaration.uri.as_str()),
        )?));
        let provenance = declaration
            .vector_provenance
            .as_mut()
            .ok_or("Safe-SVG 2 corpus provenance is missing")?;
        provenance.engine_id = if index == 1 {
            "vmb.texToSvg.cache-replay".to_owned()
        } else {
            "vmb.texToSvg".to_owned()
        };
        provenance.engine_version = "2026.09.0".to_owned();
        provenance.rules_version = "vmb.math-safe-svg/1".to_owned();
        declarations.images.push(declaration);
    }
    let admitted = admit_vector_catalog(
        corpus_root,
        &corpus_root.join("document-package.json"),
        &declarations,
        limits,
        profile_fingerprint,
    )?;
    let candidates = VectorContentCandidateRegistry::staging_from_admitted_with_reverse_completion(
        &admitted,
        &declarations,
        reverse_completion,
    )?;
    if admitted.images().len() != SVG_NAMES.len()
        || candidates.receipt().alias_count() != u32::try_from(SVG_NAMES.len())?
        || candidates.receipt().candidate_count() != 12
    {
        return Err("precomposed corpus admission did not close 13 aliases to 12 contents".into());
    }
    Ok((admitted, candidates))
}

fn admit_vector_catalog(
    corpus_root: &Path,
    package_path: &Path,
    declarations: &typaxis_document::StagingM4ResourceCatalog,
    limits: &M4EffectiveResourceLimits,
    profile_fingerprint: [u8; 32],
) -> Result<AdmittedResourceLedger, Box<dyn std::error::Error>> {
    let base = staging_declared_base_catalog(declarations)?;
    let config = EffectiveConfig::new_for_contract(
        DocumentPackageContractId::V1_3,
        false,
        PdfStreamCompression::None,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0")
            .ok_or("invalid private fixture data versions")?,
        limits.base().get().clone(),
    )?;
    let admission = HostAdmissionContext::new(
        HostPath::new(package_path.to_path_buf())?,
        HostPath::new(corpus_root.to_path_buf())?,
        None,
        Vec::new(),
    );
    let session = HostResourceAdmissionSession::new(&admission, &config, &base)?;
    let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
        &base,
        limits,
        profile_fingerprint,
        session.roots(),
    )?;
    for declaration in &declarations.images {
        let pending = resolver.read_image(session.open_image(declaration.image_id)?)?;
        resolver.parse_and_bind_declared_image(pending)?;
    }
    Ok(resolver.finish()?)
}

/// Atomically publish generated evidence below an explicit caller-owned path.
/// Refusing every existing leaf prevents stale-file acceptance and means a
/// failed write never exposes a partial artifact set at the final path.
pub fn publish_precomposed_vector_artifacts(
    artifacts: &PrecomposedVectorArtifactSet,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    match fs::symlink_metadata(output) {
        Ok(_) => return Err("precomposed-vector artifact directory must be absent".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    for name in artifacts.files().keys() {
        if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
            return Err(format!("non-portable private artifact name {name}").into());
        }
    }
    let parent = output
        .parent()
        .ok_or("precomposed-vector artifact path has no parent")?;
    fs::create_dir_all(parent)?;
    let leaf = output
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("precomposed-vector artifact leaf is not UTF-8")?;
    let staging = parent.join(format!(".{leaf}.tmp-{}", std::process::id()));
    match fs::symlink_metadata(&staging) {
        Ok(_) => return Err("precomposed-vector staging directory already exists".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    fs::create_dir(&staging)?;
    let publication = (|| -> Result<(), Box<dyn std::error::Error>> {
        for (name, payload) in artifacts.files() {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(staging.join(name))?;
            file.write_all(payload)?;
            file.sync_all()?;
        }
        fs::rename(&staging, output)?;
        Ok(())
    })();
    if publication.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    publication
}

/// Admit one checked-in negative Safe-SVG 2 resource through the real stable
/// host-read and parser boundary, returning its typed terminal reason. The
/// caller supplies only a contained `negative-svg/*.svg` path; the helper
/// preserves the production package/profile/limits identities and replaces
/// the declaration hash with the negative file's real SHA-256 so parser
/// rejection is not obscured by the earlier hash gate.
pub fn reject_precomposed_vector_svg(
    corpus_root: &Path,
    relative_svg: &str,
) -> Result<SafeVectorFailureReason, Box<dyn std::error::Error>> {
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
        StagingSemanticDocumentPackageEncoder,
    };
    use typaxis_syntax::{
        StagingPrecomposedVectorProfileAuthorization,
        StagingPrecomposedVectorProfileSessionIdentity, StagingSemanticPackageParser,
    };

    let relative = Path::new(relative_svg);
    if relative.is_absolute()
        || relative_svg.contains('\\')
        || !relative.starts_with("negative-svg")
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err("negative SVG path is not contained below negative-svg".into());
    }
    let svg_path = corpus_root.join(relative);
    let svg = fs::read(&svg_path)?;
    let package_path = corpus_root.join("document-package.json");
    let base_limits = ValidatedResourceLimits::new(ResourceLimits::default())?;
    let limits = M4EffectiveResourceLimits::new(base_limits.clone(), M4ResourceLimits::default())?;
    let decoded = StagingSemanticDocumentPackageDecoder::new().decode(
        &fs::read(&package_path)?,
        &DocumentPackageDecodePolicy::new(&base_limits),
    )?;
    let mut wire = decoded.into_wire();
    let document = wire.document().clone();
    let mut resources = wire.resources().clone();
    resources.images[0].uri = relative_svg.to_owned();
    resources.images[0].expected_sha256 = Some(hex(sha256(&svg)));
    let image_id = ImageResourceId::new(resources.images[0].image_id);
    wire.replace_typed_regions(document, resources);
    let encoded = StagingSemanticDocumentPackageEncoder::new().encode(&wire)?;
    let decoded = StagingSemanticDocumentPackageDecoder::new().decode(
        encoded.as_bytes(),
        &DocumentPackageDecodePolicy::new(&base_limits),
    )?;
    let package = StagingSemanticPackageParser::new().parse(decoded, &base_limits)?;
    let profile = StagingPrecomposedVectorProfileAuthorization::bind_profile_receipt(
        sha256(b"typaxis.testkit/private-precomposed-negative-profile"),
        &package,
        &limits,
        &StagingPrecomposedVectorProfileSessionIdentity::fresh(),
    )?;
    let catalog = staging_declared_base_catalog(package.resources())?;
    let config = EffectiveConfig::new_for_contract(
        DocumentPackageContractId::V1_3,
        false,
        PdfStreamCompression::Flate,
        vec![ConfigResourceRoot::ProjectRoot],
        ["http", "https", "mailto", "tel"]
            .map(str::to_owned)
            .to_vec(),
        EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0")
            .ok_or("invalid private fixture data versions")?,
        ResourceLimits::default(),
    )?;
    let admission = HostAdmissionContext::new(
        HostPath::new(package_path)?,
        HostPath::new(corpus_root.to_path_buf())?,
        None,
        Vec::new(),
    );
    let session = HostResourceAdmissionSession::new(&admission, &config, &catalog)?;
    let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
        &catalog,
        &limits,
        profile.profile_fingerprint(),
        session.roots(),
    )?;
    let pending = resolver.read_image(session.open_image(image_id)?)?;
    match resolver.parse_and_bind_declared_safe_vector(pending) {
        Err(ResourceAdmissionError::InvalidSafeVectorV2(reason)) => Ok(reason),
        Err(error) => Err(format!("unexpected Safe-SVG 2 admission error: {error}").into()),
        Ok(()) => Err("negative Safe-SVG 2 resource was silently accepted".into()),
    }
}

fn line(mut value: String) -> Vec<u8> {
    value.push('\n');
    value.into_bytes()
}

fn phase_receipts_json() -> String {
    let phases = [
        "wire",
        "syntax-metrics-source-language",
        "profile-style",
        "resource-admission",
        "metric-math-binding",
        "inline-block-layout",
        "display-navigation",
        "content-form-plan",
        "structure-marked-content",
        "final-tagged-pdf-observations",
        "manifests",
    ];
    let mut out =
        String::from("{\"contract\":\"typaxis.private-production-phase-receipts/1\",\"phases\":[");
    for (index, phase) in phases.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        push_string(&mut out, phase);
    }
    out.push_str("],\"status\":\"built\"}");
    out
}

#[allow(clippy::too_many_arguments)]
fn verification_json(
    page_count: u32,
    resource_count: usize,
    placement_count: u32,
    math_fact_count: usize,
    structure_count: usize,
    form_count: u32,
    do_count: u32,
    object_count: u32,
    pdf_sha256: [u8; 32],
    ten_form_count: usize,
    ten_do_count: usize,
    ten_pdf_sha256: [u8; 32],
    alias_form_count: usize,
    alias_do_count: usize,
    alias_count: usize,
    alias_provenance_count: usize,
    alias_pdf_sha256: [u8; 32],
    corpus_form_count: usize,
    corpus_do_count: usize,
    corpus_page_count: usize,
    corpus_pdf_sha256: [u8; 32],
    effective_package_sha256: [u8; 32],
    figure_form_count: u32,
    figure_do_count: u32,
    figure_structure_count: usize,
    figure_pdf_sha256: [u8; 32],
) -> String {
    format!(
        concat!(
            "{{\"alias_use\":{{\"aliases\":{alias_count},\"do\":{alias_do_count},",
            "\"forms\":{alias_form_count},\"pdf_sha256\":\"{alias_hash}\",",
            "\"provenance_facts\":{alias_provenance_count}}},",
            "\"contract\":\"typaxis.private-precomposed-vector-verification/1\",",
            "\"corpus\":{{\"do\":{corpus_do_count},\"forms\":{corpus_form_count},",
            "\"pages\":{corpus_page_count},\"pdf_sha256\":\"{corpus_hash}\"}},",
            "\"counts\":{{\"do\":{do_count},\"forms\":{form_count},",
            "\"math_facts\":{math_fact_count},\"objects\":{object_count},",
            "\"pages\":{page_count},\"placements\":{placement_count},",
            "\"resources\":{resource_count},\"structures\":{structure_count}}},",
            "\"effective_package_sha256\":\"{effective_package_hash}\",",
            "\"figure\":{{\"do\":{figure_do_count},\"forms\":{figure_form_count},",
            "\"pdf_sha256\":\"{figure_hash}\",\"structures\":{figure_structure_count}}},",
            "\"pdf_sha256\":\"{pdf_hash}\",",
            "\"ten_use\":{{\"do\":{ten_do_count},\"forms\":{ten_form_count},",
            "\"pdf_sha256\":\"{ten_hash}\"}}}}"
        ),
        do_count = do_count,
        form_count = form_count,
        math_fact_count = math_fact_count,
        object_count = object_count,
        page_count = page_count,
        placement_count = placement_count,
        resource_count = resource_count,
        structure_count = structure_count,
        ten_do_count = ten_do_count,
        ten_form_count = ten_form_count,
        pdf_hash = hex(pdf_sha256),
        ten_hash = hex(ten_pdf_sha256),
        alias_count = alias_count,
        alias_do_count = alias_do_count,
        alias_form_count = alias_form_count,
        alias_hash = hex(alias_pdf_sha256),
        alias_provenance_count = alias_provenance_count,
        corpus_do_count = corpus_do_count,
        corpus_form_count = corpus_form_count,
        corpus_hash = hex(corpus_pdf_sha256),
        corpus_page_count = corpus_page_count,
        effective_package_hash = hex(effective_package_sha256),
        figure_do_count = figure_do_count,
        figure_form_count = figure_form_count,
        figure_hash = hex(figure_pdf_sha256),
        figure_structure_count = figure_structure_count,
    )
}

fn artifact_index_json(files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut out = String::from("{\"artifacts\":[");
    for (index, (name, payload)) in files.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        out.push_str("{\"bytes\":");
        out.push_str(&payload.len().to_string());
        out.push_str(",\"name\":");
        push_string(&mut out, name);
        out.push_str(",\"sha256\":\"");
        out.push_str(&hex(sha256(payload)));
        out.push_str("\"}");
    }
    out.push_str("],\"contract\":\"typaxis.private-precomposed-vector-artifacts/1\"}");
    out
}

fn push_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            value if value <= '\u{001f}' => {
                use std::fmt::Write;
                write!(output, "\\u{:04x}", u32::from(value)).unwrap();
            }
            value => output.push(value),
        }
    }
    output.push('"');
}

fn hex(value: [u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in value {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}
