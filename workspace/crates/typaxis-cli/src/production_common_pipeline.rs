// Own the common body graph through stable line selection and exact PDF
// assembly. This is preparation for the public writer, not its publication
// receipt: page/generated-reference convergence and manifest closure remain.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProductionCommonBodyObservation {
    pub line_reshape_passes: usize,
    pub page_passes: usize,
    pub page_record_charge: u64,
    pub candidate_steps: u64,
    pub selected_layout_sha256: [u8; 32],
    pub flow_registry_sha256: [u8; 32],
    pub block_math_terminals: usize,
}

// Kept callable outside tests so the public terminal/writer integration can use
// this exact lifetime owner. No CLI branch may publish the callback bytes yet.
#[allow(dead_code, clippy::too_many_arguments)]
pub(crate) fn with_production_common_body_pdf<R>(
    package: &typaxis_syntax::ValidatedStagingSemanticPackage,
    navigation: &typaxis_syntax::ValidatedStagingBookNavigationV2,
    semantics: &typaxis_syntax::ValidatedStagingStructureSemanticsV2,
    profile: &typaxis_machine_profile::StagingTaggedPdfProfileReceiptV2,
    admitted: &AdmittedResourceLedger,
    limits: &typaxis_core::M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    max_candidate_steps: u64,
    inspect: impl FnOnce(
        &typaxis_pdf::ProductionBodyPdfAssembly<'_, '_, '_, '_, '_, '_, '_, '_, '_>,
        &typaxis_pagination::ProductionBodyPageStability<'_, '_, '_>,
        ProductionCommonBodyObservation,
    ) -> Result<R, Failure>,
) -> Result<R, Failure> {
    let authorization = profile.base().base().authorization();
    let flow = typaxis_syntax::prepare_production_text_flow(package, navigation, limits)
        .map_err(map_production_input_error)?;
    let bindings =
        typaxis_layout::bind_staging_precomposed_vectors(package, authorization, limits, admitted)
            .map_err(map_production_input_error)?;
    let math = typaxis_layout::prepare_staging_math_vector_flows(
        package,
        authorization,
        limits,
        admitted,
        &bindings,
    )
    .map_err(map_production_input_error)?;
    let blocks = typaxis_layout::prepare_staging_precomposed_vector_blocks(
        package,
        authorization,
        limits,
        admitted,
        &bindings,
        &math,
    )
    .map_err(map_production_input_error)?;
    typaxis_layout::with_converged_production_body_lines(
        package,
        navigation,
        authorization,
        limits,
        admitted,
        &flow,
        &bindings,
        japanese_mode,
        blocks.page_geometry().body(),
        max_candidate_steps,
        |stable| {
            stable
                .footnotes()
                .verify(stable.lines(), limits)
                .map_err(map_production_internal_error)?;
            let stable_pages = typaxis_pagination::paginate_stable_production_body(
                stable.lines(),
                &blocks,
                limits,
            )
            .map_err(map_production_input_error)?;
            let page_passes = stable_pages.passes().len();
            let page_record_charge = stable_pages.selected().record_charge();
            let (selected, page_stability) = stable_pages.into_parts();
            let selected = typaxis_pagination::finalize_production_body_math_terminals(
                selected, &math, limits,
            )
            .map_err(map_production_internal_error)?;
            page_stability
                .verify(&selected, limits)
                .map_err(map_production_internal_error)?;
            let terminals = selected.math_terminals().ok_or_else(|| {
                Failure::internal("common body is missing completed math terminals")
            })?;
            terminals
                .terminals()
                .verify(&math)
                .map_err(map_production_internal_error)?;
            let observation = ProductionCommonBodyObservation {
                line_reshape_passes: stable.passes().len(),
                page_passes,
                page_record_charge,
                candidate_steps: stable.candidate_steps(),
                selected_layout_sha256: selected.fingerprint(),
                flow_registry_sha256: math.receipt().fingerprint(),
                block_math_terminals: terminals.terminals().receipts().len(),
            };
            let display =
                typaxis_display_list::build_production_body_display(&selected, admitted, limits)
                    .map_err(|error| {
                        Failure::internal(format!("common body display: {error:?}"))
                    })?;
            let fonts =
                typaxis_resources::finalize_production_body_fonts(&display, admitted, limits)
                    .map_err(|error| Failure::input(format!("common body fonts: {error:?}")))?;
            let content = typaxis_pdf::build_production_body_page_content(&fonts, admitted, limits)
                .map_err(|error| Failure::internal(format!("common body content: {error:?}")))?;
            let structure = typaxis_display_list::build_production_body_structure(
                &display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .map_err(|error| Failure::internal(format!("common body structure: {error:?}")))?;
            let marked = typaxis_pdf::build_production_body_marked_content(
                &content, &structure, admitted, limits,
            )
            .map_err(|error| Failure::internal(format!("common body marked content: {error:?}")))?;
            let objects = typaxis_pdf::build_production_body_objects(&marked, admitted, limits)
                .map_err(|error| Failure::internal(format!("common body objects: {error:?}")))?;
            let pdf = typaxis_pdf::assemble_production_body_pdf(&objects, admitted, limits)
                .map_err(|error| Failure::internal(format!("common body assembly: {error:?}")))?;
            pdf.verify(&objects, admitted, limits).map_err(|error| {
                Failure::internal(format!("common body assembly identity: {error:?}"))
            })?;
            page_stability
                .verify(&selected, limits)
                .map_err(map_production_internal_error)?;
            inspect(&pdf, &page_stability, observation)
        },
    )
    .map_err(map_production_input_error)?
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProductionCommonFootnoteObservation {
    pub line_reshape_passes: usize,
    pub page_passes: u16,
    pub line_candidate_steps: u64,
    pub page_work_steps: u64,
    pub record_charge: u64,
    pub spool_charge: u64,
    pub display_sha256: [u8; 32],
    pub flow_registry_sha256: [u8; 32],
    pub block_math_terminals: usize,
}

// Own every intermediate receipt until the callback finishes. These bytes still
// require public terminal/manifest closure before CLI publication.
#[allow(dead_code, clippy::too_many_arguments)]
pub(crate) fn with_production_common_footnote_pdf<R>(
    package: &typaxis_syntax::ValidatedStagingSemanticPackage,
    navigation: &typaxis_syntax::ValidatedStagingBookNavigationV2,
    semantics: &typaxis_syntax::ValidatedStagingStructureSemanticsV2,
    profile: &typaxis_machine_profile::StagingTaggedPdfProfileReceiptV2,
    admitted: &AdmittedResourceLedger,
    limits: &typaxis_core::M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    max_candidate_steps: u64,
    inspect: impl FnOnce(
        &typaxis_pdf::ProductionFootnotePdfAssembly<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
        &typaxis_pagination::ProductionBodyFootnoteStablePages<'_, '_, '_, '_, '_>,
        &typaxis_display_list::ProductionFootnoteBookNavigation<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
        &typaxis_pdf::ProductionBookPdfObservation,
        ProductionCommonFootnoteObservation,
    ) -> Result<R, Failure>,
) -> Result<R, Failure> {
    let authorization = profile.base().base().authorization();
    let flow = typaxis_syntax::prepare_production_text_flow(package, navigation, limits)
        .map_err(map_production_input_error)?;
    let bindings =
        typaxis_layout::bind_staging_precomposed_vectors(package, authorization, limits, admitted)
            .map_err(map_production_input_error)?;
    let math = typaxis_layout::prepare_staging_math_vector_flows(
        package,
        authorization,
        limits,
        admitted,
        &bindings,
    )
    .map_err(map_production_input_error)?;
    let blocks = typaxis_layout::prepare_staging_precomposed_vector_blocks(
        package,
        authorization,
        limits,
        admitted,
        &bindings,
        &math,
    )
    .map_err(map_production_input_error)?;
    typaxis_layout::with_converged_production_body_lines(
        package,
        navigation,
        authorization,
        limits,
        admitted,
        &flow,
        &bindings,
        japanese_mode,
        blocks.page_geometry().body(),
        max_candidate_steps,
        |stable| {
            let prepared = typaxis_pagination::prepare_production_body_flow(
                stable.lines(),
                &blocks,
                stable.footnotes(),
                limits,
            )
            .map_err(map_common_pagination_error)?;
            let remaining_steps = max_candidate_steps
                .checked_sub(stable.candidate_steps())
                .ok_or_else(|| {
                    Failure::limit("L5110: common footnote candidate budget exhausted")
                })?;
            let mut search = typaxis_pagination::prepare_production_footnote_demand_search(
                &prepared,
                limits,
                remaining_steps,
            )
            .map_err(map_common_pagination_error)?;
            let pages = search
                .select_stable_pages()
                .map_err(map_common_pagination_error)?;
            let geometry = search
                .place_pages_content(pages.sequence())
                .map_err(map_common_pagination_error)?;
            let terminals = search
                .finalize_page_math(&pages, &geometry, &math, limits)
                .map_err(map_common_pagination_error)?;
            terminals
                .terminals()
                .verify(&math)
                .map_err(map_production_internal_error)?;
            let display = typaxis_display_list::build_production_footnote_display(
                &terminals, admitted, limits,
            )
            .map_err(map_common_display_error)?;
            let structure = typaxis_display_list::build_production_footnote_structure(
                &display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .map_err(map_common_structure_error)?;
            let fonts =
                typaxis_resources::finalize_production_footnote_fonts(&structure, admitted, limits)
                    .map_err(|e| Failure::input(format!("common footnote fonts: {e:?}")))?;
            let content =
                typaxis_pdf::build_production_footnote_page_content(&fonts, admitted, limits)
                    .map_err(|e| Failure::internal(format!("common footnote content: {e:?}")))?;
            let marked =
                typaxis_pdf::build_production_footnote_marked_content(&content, admitted, limits)
                    .map_err(map_common_marked_error)?;
            let annotations =
                typaxis_pdf::build_production_footnote_annotations(&marked, admitted, limits)
                    .map_err(|e| map_common_object_error("annotations", e))?;
            let structure_objects = typaxis_pdf::build_production_footnote_structure_objects(
                &annotations,
                admitted,
                limits,
            )
            .map_err(|e| map_common_object_error("structure objects", e))?;
            let resources = typaxis_pdf::build_production_footnote_resource_objects(
                &structure_objects,
                admitted,
                limits,
            )
            .map_err(|e| map_common_object_error("resources", e))?;
            let pdf = typaxis_pdf::assemble_production_footnote_pdf(&resources, admitted, limits)
                .map_err(|e| map_common_assembly_error("assembly", e))?;
            pdf.verify(&resources, admitted, limits).map_err(|e| {
                Failure::internal(format!("common footnote assembly identity: {e:?}"))
            })?;
            let book_inputs = typaxis_display_list::project_production_footnote_book_navigation(
                annotations.navigation(),
                admitted,
                limits,
                pdf.record_charge(),
                pdf.spool_charge(),
            )
            .map_err(map_production_input_error)?;
            book_inputs
                .verify(annotations.navigation(), admitted, limits)
                .map_err(map_production_internal_error)?;
            let book_inputs = typaxis_display_list::seal_production_footnote_book_navigation(
                book_inputs,
                profile.base().authorization(),
                admitted,
                limits,
            )
            .map_err(map_production_input_error)?;
            book_inputs
                .verify(
                    annotations.navigation(),
                    profile.base().authorization(),
                    admitted,
                    limits,
                )
                .map_err(map_production_internal_error)?;
            let book_pdf = typaxis_pdf::observe_production_footnote_book_pdf(
                &pdf,
                &book_inputs,
                profile.base().authorization(),
                admitted,
                limits,
            )
            .map_err(|e| map_common_assembly_error("book PDF observation", e))?;
            let observation = ProductionCommonFootnoteObservation {
                line_reshape_passes: stable.passes().len(),
                page_passes: pages.passes(),
                line_candidate_steps: stable.candidate_steps(),
                page_work_steps: search.work_steps(),
                record_charge: book_pdf.record_charge(),
                spool_charge: book_pdf.spool_charge(),
                display_sha256: display.fingerprint(),
                flow_registry_sha256: math.receipt().fingerprint(),
                block_math_terminals: terminals.terminals().receipts().len(),
            };
            inspect(&pdf, &pages, &book_inputs, &book_pdf, observation)
        },
    )
    .map_err(map_common_reshape_error)?
}

fn common_pdf_failure(stage: &str, code: Option<&str>, error: impl std::fmt::Debug) -> Failure {
    match code {
        Some(code) => Failure::limit(format!("{code}: common footnote {stage}: {error:?}")),
        None => Failure::internal(format!("I9190: common footnote {stage}: {error:?}")),
    }
}
fn map_common_assembly_error(
    stage: &str,
    error: typaxis_pdf::ProductionBodyAssemblyError,
) -> Failure {
    use typaxis_pdf::ProductionBodyAssemblyError as E;
    let code = match error {
        E::ObjectLimit | E::AllocationFailure => Some("G6100"),
        E::RecordLimit => Some("L5110"),
        E::SpoolLimit | E::OutputLimit => Some("D8101"),
        E::ReceiptMismatch | E::Metadata => None,
    };
    common_pdf_failure(stage, code, error)
}
fn map_common_object_error(stage: &str, error: typaxis_pdf::ProductionBodyObjectError) -> Failure {
    use typaxis_display_list::ProductionBodyNavigationErrorKind as N;
    use typaxis_pdf::ProductionBodyObjectError as E;
    let code = match &error {
        E::ObjectLimit | E::AllocationFailure => Some("G6100"),
        E::RecordLimit => Some("L5110"),
        E::SpoolLimit | E::OutputLimit => Some("D8101"),
        E::Navigation(e) => match e.kind {
            N::RecordLimit | N::AllocationFailure => Some("L5110"),
            _ => None,
        },
        E::ReceiptMismatch | E::InvalidFont | E::InvalidStructure => None,
    };
    common_pdf_failure(stage, code, error)
}
fn map_common_marked_error(error: typaxis_pdf::ProductionBodyMarkedError) -> Failure {
    use typaxis_pdf::ProductionBodyMarkedError as E;
    let code = match error {
        E::RecordLimit => Some("L5110"),
        E::OutputLimit => Some("D8101"),
        E::AllocationFailure => Some("G6100"),
        E::ReceiptMismatch => None,
    };
    common_pdf_failure("marked content", code, error)
}

fn map_common_pagination_error(
    error: typaxis_pagination::ProductionBodyPaginationError,
) -> Failure {
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    match error.kind {
        E::PageLimit
        | E::PagePassLimit
        | E::FootnoteSearchLimit
        | E::PageBreakLookbackLimit { .. }
        | E::FragmentLimit
        | E::AllocationFailure => Failure::limit(format!("L5110: {error}")),
        E::SpoolLimit => Failure::limit(format!("D8101: {error}")),
        E::JointPageNoFit | E::Oversize | E::InvalidFootnoteCapacity | E::KeepAcrossForcedBreak => {
            Failure::input(format!("L5100: {error}"))
        }
        E::ReceiptMismatch | E::WidthMismatch | E::ArithmeticOverflow => {
            Failure::internal(format!("I9190: {error}"))
        }
        E::MathTerminal(cause) => {
            let mut failure = map_production_internal_error(cause);
            failure
                .message
                .push_str(&format!("; pagination node {}", error.owner.get()));
            failure
        }
        E::PendingRegion(_)
        | E::PendingNamedPage
        | E::PendingEquationNumber
        | E::EmptyListItem
        | E::EmptyFootnote
        | E::PendingContainerIndent
        | E::EmptyParagraph => Failure::input(error.to_string()),
    }
}

fn map_common_reshape_error(error: typaxis_layout::ProductionBodyReshapeError) -> Failure {
    use typaxis_layout::{
        ProductionBodyReshapeError as E, ProductionInlinePreparationErrorKind as L,
    };
    use typaxis_linebreak::{AtomicVectorInlineError as A, BreakError as B};
    use typaxis_shaping::ProductionTextShapeErrorKind as S;
    let (kind, code) = match &error {
        E::Layout(e) => match &e.kind {
            L::UnitLimit | L::AllocationFailure => (FailureKind::Limit, "L5110"),
            L::ReceiptMismatch | L::ArithmeticOverflow => (FailureKind::Internal, "I9190"),
            L::Atomic(cause) => match cause {
                A::CandidateLimit | A::SelectionLimit | A::AllocationFailure => {
                    (FailureKind::Limit, "L5110")
                }
                A::InvalidBinding | A::ArithmeticOverflow => (FailureKind::Internal, "I9190"),
                A::EmptyParagraph
                | A::MissingVector
                | A::UnicodeLineBreak
                | A::InvalidLineSize
                | A::NoFeasibleLine
                | A::Oversize(_) => (FailureKind::Input, "L5100"),
            },
            _ => (FailureKind::Input, "L5100"),
        },
        E::Shape(e) => match e.kind {
            S::ContextLimit | S::OutputLimit | S::AllocationFailure => {
                (FailureKind::Limit, "L5110")
            }
            S::ReceiptMismatch | S::InvalidLineContext | S::ArithmeticOverflow => {
                (FailureKind::Internal, "I9190")
            }
            _ => (FailureKind::Input, "L5100"),
        },
        E::Feedback(e) => match e {
            B::IterationLimit
            | B::LineShapeLimit
            | B::ParagraphTextLimit
            | B::AllocationFailure => (FailureKind::Limit, "L5110"),
            B::NoFeasibleBreak => (FailureKind::Input, "L5100"),
            _ => (FailureKind::Internal, "I9190"),
        },
    };
    let message = format!("{code}: {error}");
    match kind {
        FailureKind::Limit => Failure::limit(message),
        FailureKind::Internal => Failure::internal(message),
        _ => Failure::input(message),
    }
}

fn map_common_display_error(error: typaxis_display_list::ProductionBodyDisplayError) -> Failure {
    use typaxis_display_list::ProductionBodyDisplayErrorKind as E;
    match error.kind {
        E::RecordLimit | E::AllocationFailure => Failure::limit(format!("L5110: {error}")),
        E::PendingEquationNumber => Failure::input(error.to_string()),
        E::ReceiptMismatch | E::ArithmeticOverflow => Failure::internal(format!("I9190: {error}")),
    }
}
fn map_common_structure_error(
    error: typaxis_display_list::ProductionBodyStructureError,
) -> Failure {
    use typaxis_display_list::ProductionBodyStructureError as E;
    let code = match error {
        E::RecordLimit | E::AllocationFailure => Some("L5110"),
        E::SpoolLimit => Some("D8101"),
        E::ReceiptMismatch | E::Registry | E::MissingPaint | E::InvalidPaint => None,
    };
    common_pdf_failure("structure", code, error)
}
