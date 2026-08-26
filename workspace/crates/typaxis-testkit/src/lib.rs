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

    fn workspace_dependencies(manifest: &Path) -> Vec<String> {
        workspace_dependency_declarations(manifest)
            .into_iter()
            .map(|(name, declaration)| declared_package_name(name, &declaration))
            .filter(|name| name.starts_with("typaxis-"))
            .collect()
    }

    fn is_denied(from: &str, to: &str) -> bool {
        if from == "typaxis-testkit" {
            return false;
        }

        match from {
            "typaxis-host-admission" => return to != "typaxis-core",
            "typaxis-document-package" => return to != "typaxis-core",
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
            .map(|(name, declaration)| declared_package_name(name, &declaration))
            .filter_map(|dependency| {
                (dependency.starts_with("typaxis-") && is_denied(crate_name, &dependency))
                    .then(|| format!("{crate_name} -> {dependency}"))
            })
            .collect()
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
