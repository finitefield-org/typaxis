#![forbid(unsafe_code)]

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
    use std::fs;
    use std::path::{Path, PathBuf};
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

    fn workspace_dependencies(manifest: &Path) -> Vec<String> {
        let contents = fs::read_to_string(manifest).expect("workspace manifest must be readable");
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
            let Some((name, _)) = line.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if name.starts_with("typaxis-") {
                dependencies.push(name.to_owned());
            }
        }
        dependencies
    }

    fn is_denied(from: &str, to: &str) -> bool {
        if matches!(from, "typaxis-cli" | "typaxis-testkit") {
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
    fn workspace_dependency_deny_edges_are_absent() {
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
            let manifest = entry.path().join("Cargo.toml");
            for dependency in workspace_dependencies(&manifest) {
                if is_denied(&crate_name, &dependency) {
                    violations.push(format!("{crate_name} -> {dependency}"));
                }
            }
        }
        assert!(
            violations.is_empty(),
            "forbidden workspace dependencies: {violations:?}"
        );
    }
}
