use super::*;

#[test]
#[ignore = "requires explicit TYPAXIS_BOOK_V2_VMB_JOB from the VMB Book-2 exporter"]
fn book_v2_saved_vmb_description_export_builds_original_source_and_harano_pdf() {
    check_saved_export(
        "TYPAXIS_BOOK_V2_VMB_JOB",
        "TYPAXIS_BOOK_V2_VMB_PDF",
        &[],
        1,
        3,
        &[],
    );
}

#[test]
#[ignore = "requires explicit TYPAXIS_BOOK_V2_VMB_ADMONITION_JOB from the VMB Book-2 exporter"]
fn book_v2_saved_vmb_admonitions_keep_all_source_kinds_in_harano_pdf() {
    check_saved_export(
        "TYPAXIS_BOOK_V2_VMB_ADMONITION_JOB",
        "TYPAXIS_BOOK_V2_VMB_ADMONITION_PDF",
        &[
            "formalization_note",
            "common_error",
            "warning",
            "note",
            "remark",
            "counterexample",
            "example",
        ],
        1,
        3,
        &[],
    );
}

#[test]
#[ignore = "requires explicit TYPAXIS_BOOK_V2_VMB_SOLUTION_JOB from the VMB Book-2 exporter"]
fn book_v2_saved_vmb_solution_keeps_forward_exercise_link_and_nested_source() {
    check_saved_export(
        "TYPAXIS_BOOK_V2_VMB_SOLUTION_JOB",
        "TYPAXIS_BOOK_V2_VMB_SOLUTION_PDF",
        &[
            "solution",
            "formalization_note",
            "common_error",
            "warning",
            "note",
            "remark",
            "counterexample",
            "example",
            "exercise",
        ],
        1,
        3,
        &["fixture.exercise"],
    );
}

#[test]
#[ignore = "requires explicit TYPAXIS_BOOK_V2_VMB_FORMAL_SOLUTION_JOB from the VMB Book-2 exporter"]
fn book_v2_saved_vmb_formal_solution_keeps_original_npa_source_in_pdf() {
    check_saved_export(
        "TYPAXIS_BOOK_V2_VMB_FORMAL_SOLUTION_JOB",
        "TYPAXIS_BOOK_V2_VMB_FORMAL_SOLUTION_PDF",
        &["solution", "exercise"],
        0,
        2,
        &["exercise.test"],
    );
}

#[test]
#[ignore = "requires explicit TYPAXIS_BOOK_V2_VMB_TEACHING_JOB from the VMB Book-2 exporter"]
fn book_v2_saved_vmb_teaching_units_keep_metadata_source_and_bidirectional_links() {
    check_saved_export(
        "TYPAXIS_BOOK_V2_VMB_TEACHING_JOB",
        "TYPAXIS_BOOK_V2_VMB_TEACHING_PDF",
        &["exercise", "exercise_part", "choice", "hint", "solution"],
        1,
        4,
        &["teaching.exercise", "teaching.solution"],
    );
}

#[test]
#[ignore = "requires explicit TYPAXIS_BOOK_V2_VMB_AUTHORED_JOB from the VMB Book-2 exporter"]
fn book_v2_saved_vmb_assumptions_and_quote_keep_authored_text_and_links() {
    check_saved_export(
        "TYPAXIS_BOOK_V2_VMB_AUTHORED_JOB",
        "TYPAXIS_BOOK_V2_VMB_AUTHORED_PDF",
        &["result", "assumption", "assumption", "quote"],
        1,
        6,
        &["authored.result", "authored.quote"],
    );
}

#[test]
#[ignore = "requires explicit TYPAXIS_BOOK_V2_VMB_NUMBER_JOB from the VMB Book-2 exporter"]
fn book_v2_saved_vmb_numbers_keep_section_result_figure_and_equation_targets() {
    check_saved_export_options(
        "TYPAXIS_BOOK_V2_VMB_NUMBER_JOB",
        "TYPAXIS_BOOK_V2_VMB_NUMBER_PDF",
        &["result"],
        0,
        6,
        &[
            "section.test",
            "figure.red",
            "eq.thm.logic.de-morgan",
            "theorem.number",
        ],
        SavedNumberExpectations {
            images: 4,
            references: 5,
            bindings: &[
                ("section.test", "1"),
                ("figure.red", "1"),
                ("eq.thm.logic.de-morgan", "1.2"),
                ("theorem.number", "1.12"),
            ],
        },
    );
}

struct SavedNumberExpectations<'a> {
    images: usize,
    references: usize,
    bindings: &'a [(&'a str, &'a str)],
}

#[test]
#[ignore = "requires explicit TYPAXIS_BOOK_V2_VMB_TABLE_CAPTION_JOB from the VMB Book-2 exporter"]
fn book_v2_saved_vmb_table_captions_keep_title_math_and_number_targets() {
    check_saved_export_options(
        "TYPAXIS_BOOK_V2_VMB_TABLE_CAPTION_JOB",
        "TYPAXIS_BOOK_V2_VMB_TABLE_CAPTION_PDF",
        &[],
        0,
        3,
        &["table.test"],
        SavedNumberExpectations {
            images: 1,
            references: 1,
            bindings: &[("table.test", "1.3")],
        },
    );
}

#[test]
#[ignore = "requires explicit TYPAXIS_BOOK_V2_VMB_FIGURE_TITLE_JOB from the VMB Book-2 exporter"]
fn book_v2_saved_vmb_figure_titles_keep_rich_text_captions_math_and_numbers() {
    check_saved_export_options(
        "TYPAXIS_BOOK_V2_VMB_FIGURE_TITLE_JOB",
        "TYPAXIS_BOOK_V2_VMB_FIGURE_TITLE_PDF",
        &["result"],
        0,
        7,
        &[
            "section.test",
            "figure.red",
            "figure.shared",
            "eq.thm.logic.de-morgan",
            "theorem.number",
        ],
        SavedNumberExpectations {
            images: 4,
            references: 5,
            bindings: &[
                ("section.test", "1"),
                ("figure.red", "1"),
                ("eq.thm.logic.de-morgan", "1.2"),
                ("theorem.number", "1.12"),
            ],
        },
    );
}

fn check_saved_export(
    job_variable: &str,
    pdf_variable: &str,
    semantic_kinds: &[&str],
    descriptions: usize,
    math: usize,
    linked_targets: &[&str],
) {
    check_saved_export_options(
        job_variable,
        pdf_variable,
        semantic_kinds,
        descriptions,
        math,
        linked_targets,
        SavedNumberExpectations {
            images: 2,
            references: 0,
            bindings: &[],
        },
    );
}

fn check_saved_export_options(
    job_variable: &str,
    pdf_variable: &str,
    semantic_kinds: &[&str],
    descriptions: usize,
    math: usize,
    linked_targets: &[&str],
    numbers: SavedNumberExpectations<'_>,
) {
    let directory = PathBuf::from(std::env::var(job_variable).unwrap());
    let path = directory.join("document-package.json");
    let original = fs::read(&path).unwrap();
    let limits = limits();
    let context = HostAdmissionContext::new(
        HostPath::new(path.clone()).unwrap(),
        HostPath::new(directory).unwrap(),
        None,
        vec![],
    );
    let (session, raw) = HostBookV2InputSession::open(
        MachineInputHostOptions::new(HostPath::new(path.clone()).unwrap(), None),
        limits.base(),
    )
    .unwrap();
    let decoded = session
        .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(limits.base()))
        .unwrap();
    let sources = session.admit_sources(&decoded, limits.base()).unwrap();
    let body = prepare_admitted_book_v2_body(
        session.finish(raw, decoded, sources).unwrap(),
        limits.base(),
    )
    .unwrap();
    let input = prepare_book_v2_resources(
        body,
        &context,
        &config(limits.base().get().clone()),
        &limits,
    )
    .unwrap();
    assert_eq!(input.resources().images().len(), numbers.images);
    assert_eq!(input.resources().fonts().len(), 1);
    let hash = typaxis_core::sha256(input.resources().fonts()[0].bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hash,
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    with_converged_book_v2_pdf(
        &input,
        &limits,
        typaxis_linebreak::JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            let registry = pdf.navigation().source().source();
            let kinds = registry
                .nodes()
                .iter()
                .filter_map(|n| n.source().semantic_kind())
                .collect::<Vec<_>>();
            assert_eq!(kinds, semantic_kinds);
            for exercise_target in linked_targets {
                use typaxis_pdf::book_v2::{BookV2DestinationKind, BookV2NavigationTarget};
                let navigation = pdf.navigation();
                let target = navigation.destinations().iter().position(|d|
                        matches!(d.kind(), BookV2DestinationKind::Anchor(target) if target == *exercise_target)
                    && d.position().is_some()
                ).unwrap();
                assert!(navigation.links().iter().any(|l|
                    matches!(l.target(), BookV2NavigationTarget::Destination(index) if index == target)
                    && l.first_rectangle().is_some()
                ));
            }
            let display = pdf
                .navigation()
                .source()
                .source()
                .source()
                .source()
                .display();
            let terminals = display.source();
            let flow = terminals.source().flow().lines().prepared().source_flow();
            assert_eq!(flow.navigation().number_bindings().len(), numbers.bindings.len());
            for &(target, text) in numbers.bindings {
                assert_eq!(flow.navigation().reference_number(target), Some(text));
            }
            let mut number_references = 0;
            for site in flow.paragraphs().iter().flat_map(|p| p.items()) {
                if let Some(typaxis_syntax::ProductionInlineReference::Anchor {
                    target, format:typaxis_syntax::ProductionReferenceFormat::Number, ..
                }) = site.reference() {
                    number_references += 1;
                    let expected = numbers.bindings.iter().find(|(name, _)| *name == target).unwrap().1;
                    assert_eq!(flow.reference_text(site.owner()), Some(expected));
                    assert!(flow.page_reference_text(site.owner()).is_none());
                    assert!(flow.reference_provenance(site.owner()).is_some());
                }
            }
            assert_eq!(number_references, numbers.references);
            assert_eq!(flow.description_lists().len(), descriptions);
            assert_eq!(flow.description_items().len(), descriptions);
            assert_eq!(terminals.source().semantic_math(), math);
            assert_math_terminals(terminals);
            record_driver_pdf(pdf, observation, &limits);
            if let Ok(path) = std::env::var(pdf_variable) {
                use std::io::Write;
                let mut file = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(path)
                    .unwrap();
                file.write_all(pdf.bytes()).unwrap();
                file.sync_all().unwrap();
            }
        },
    )
    .unwrap();
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
#[ignore = "requires explicit TYPAXIS_BOOK_V2_VMB_TABLE_ALIGNMENT_JOB from the VMB Book-2 exporter"]
fn book_v2_saved_vmb_table_alignment_keeps_cells_title_math_and_number_targets() {
    check_saved_export_options(
        "TYPAXIS_BOOK_V2_VMB_TABLE_ALIGNMENT_JOB",
        "TYPAXIS_BOOK_V2_VMB_TABLE_ALIGNMENT_PDF",
        &[],
        0,
        3,
        &["table.test"],
        SavedNumberExpectations {
            images: 1,
            references: 1,
            bindings: &[("table.test", "1.3")],
        },
    );
}

