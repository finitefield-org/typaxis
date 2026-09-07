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
    let initial_values = {
        let flow = typaxis_syntax::prepare_production_text_flow(package, navigation, limits)
            .map_err(map_production_input_error)?;
        let owners = || {
            flow.paragraphs()
                .iter()
                .flat_map(|p| p.items())
                .filter_map(|site| {
                    matches!(
                        site.reference(),
                        Some(typaxis_syntax::ProductionInlineReference::Anchor {
                            format: typaxis_syntax::ProductionReferenceFormat::Page,
                            ..
                        })
                    )
                    .then_some(site.owner())
                })
        };
        let count = owners().count();
        if count == 0 {
            None
        } else {
            if (count as u64)
                .checked_mul(3)
                .is_none_or(|n| n > limits.base().get().max_fragments)
            {
                return Err(Failure::limit("L5110: page reference initial records"));
            }
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| Failure::limit("L5110: page reference initial allocation"))?;
            // Page 1 is only an initial candidate. No consumer sees it until
            // actual destinations and repeated completed selections agree.
            values.extend(owners().map(|owner| (owner, 1)));
            values.sort_unstable_by_key(|value| value.0);
            Some(values)
        }
    };
    if let Some(values) = initial_values {
        return with_converged_production_page_reference_pdf(
            package,
            navigation,
            semantics,
            profile,
            admitted,
            limits,
            japanese_mode,
            max_candidate_steps,
            &values,
            |pdf, pages, book, book_pdf, mut observation, total| {
                observation.line_reshape_passes = total.line_reshape_passes;
                observation.page_passes = total.page_passes;
                observation.line_candidate_steps = total.line_candidate_steps;
                observation.page_work_steps = total.page_work_steps;
                observation.record_charge = total.record_charge;
                observation.spool_charge = total.spool_charge;
                inspect(pdf, pages, book, book_pdf, observation)
            },
        );
    }
    with_production_common_footnote_pdf_candidates(
        package,
        navigation,
        semantics,
        profile,
        admitted,
        limits,
        japanese_mode,
        max_candidate_steps,
        None,
        limits.base().get().max_layout_passes,
        inspect,
    )
}

// Candidate labels are not final page evidence. The convergence owner must
// compare their targets with actual destinations before authorizing publication.
pub(crate) fn with_production_common_footnote_pdf_candidates<R>(
    package: &typaxis_syntax::ValidatedStagingSemanticPackage,
    navigation: &typaxis_syntax::ValidatedStagingBookNavigationV2,
    semantics: &typaxis_syntax::ValidatedStagingStructureSemanticsV2,
    profile: &typaxis_machine_profile::StagingTaggedPdfProfileReceiptV2,
    admitted: &AdmittedResourceLedger,
    limits: &typaxis_core::M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    max_candidate_steps: u64,
    page_values: Option<&[(NodeId, u32)]>,
    remaining_page_passes: u16,
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
    let flow = match page_values {
        Some(values) => typaxis_syntax::prepare_production_text_flow_with_page_references(
            package, navigation, limits, values,
        ),
        None => typaxis_syntax::prepare_production_text_flow(package, navigation, limits),
    }
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
                .select_stable_pages_with_pass_limit(remaining_page_passes)
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
                    .map_err(map_common_font_error)?;
            let content =
                typaxis_pdf::build_production_footnote_page_content(&fonts, admitted, limits)
                    .map_err(map_common_content_error)?;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProductionPageReferenceConvergenceObservation {
    pub passes: u16,
    pub line_reshape_passes: usize,
    pub page_passes: u16,
    pub line_candidate_steps: u64,
    pub page_work_steps: u64,
    pub record_charge: u64,
    pub spool_charge: u64,
}

// This diagnostic owner updates page labels from actual destination placements
// and requires two identical completed PDF selections. It does not issue public
// publication receipts. Stage-local allocation admission remains in each stage;
// aggregate completed-pass charges below do not replace pre-allocation admission
// across all intermediate allocations, which is still a separate integration.
#[allow(dead_code, clippy::too_many_arguments)]
pub(crate) fn with_converged_production_page_reference_pdf<R>(
    package: &typaxis_syntax::ValidatedStagingSemanticPackage,
    navigation: &typaxis_syntax::ValidatedStagingBookNavigationV2,
    semantics: &typaxis_syntax::ValidatedStagingStructureSemanticsV2,
    profile: &typaxis_machine_profile::StagingTaggedPdfProfileReceiptV2,
    admitted: &AdmittedResourceLedger,
    limits: &typaxis_core::M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    max_candidate_steps: u64,
    initial_values: &[(NodeId, u32)],
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
        ProductionPageReferenceConvergenceObservation,
    ) -> Result<R, Failure>,
) -> Result<R, Failure> {
    let caps = limits.base().get();
    let row_charge = (initial_values.len() as u64)
        .checked_mul(3)
        .ok_or_else(|| Failure::limit("L5110: page reference feedback records"))?;
    if row_charge > caps.max_fragments {
        return Err(Failure::limit("L5110: page reference feedback records"));
    }
    let mut values = Vec::new();
    let mut next = Vec::new();
    values
        .try_reserve_exact(initial_values.len())
        .map_err(|_| Failure::limit("L5110: page reference feedback allocation"))?;
    next.try_reserve_exact(initial_values.len())
        .map_err(|_| Failure::limit("L5110: page reference feedback allocation"))?;
    values.extend_from_slice(initial_values);
    let mut total = ProductionPageReferenceConvergenceObservation {
        passes: 0,
        line_reshape_passes: 0,
        page_passes: 0,
        line_candidate_steps: 0,
        page_work_steps: 0,
        record_charge: row_charge,
        spool_charge: 0,
    };
    let mut previous = None;
    let mut inspect = Some(inspect);
    loop {
        // Every invocation needs at least two actual page-selection passes.
        // All calls share the work and pass ceilings; no retry refunds work.
        if caps.max_layout_passes.saturating_sub(total.page_passes) < 2 {
            return Err(Failure::limit(
                "L5110: page reference convergence pass limit",
            ));
        }
        let remaining = max_candidate_steps
            .checked_sub(total.line_candidate_steps)
            .and_then(|n| n.checked_sub(total.page_work_steps))
            .ok_or_else(|| Failure::limit("L5110: page reference convergence work limit"))?;
        next.clear();
        let result = with_production_common_footnote_pdf_candidates(
            package,
            navigation,
            semantics,
            profile,
            admitted,
            limits,
            japanese_mode,
            remaining,
            Some(&values),
            caps.max_layout_passes - total.page_passes,
            |pdf, pages, book, book_pdf, observation| {
                total.passes = total
                    .passes
                    .checked_add(1)
                    .ok_or_else(|| Failure::limit("L5110: page reference pass overflow"))?;
                total.line_reshape_passes = total
                    .line_reshape_passes
                    .checked_add(observation.line_reshape_passes)
                    .ok_or_else(|| Failure::limit("L5110: page reference reshape pass overflow"))?;
                total.page_passes = total
                    .page_passes
                    .checked_add(observation.page_passes)
                    .filter(|n| *n <= caps.max_layout_passes)
                    .ok_or_else(|| {
                        Failure::limit("L5110: page reference convergence pass limit")
                    })?;
                total.line_candidate_steps = total
                    .line_candidate_steps
                    .checked_add(observation.line_candidate_steps)
                    .ok_or_else(|| {
                        Failure::limit("L5110: page reference convergence work limit")
                    })?;
                total.page_work_steps = total
                    .page_work_steps
                    .checked_add(observation.page_work_steps)
                    .ok_or_else(|| {
                        Failure::limit("L5110: page reference convergence work limit")
                    })?;
                if total
                    .line_candidate_steps
                    .checked_add(total.page_work_steps)
                    .is_none_or(|n| n > max_candidate_steps)
                {
                    return Err(Failure::limit(
                        "L5110: page reference convergence work limit",
                    ));
                }
                total.record_charge = total
                    .record_charge
                    .checked_add(observation.record_charge)
                    .filter(|n| *n <= caps.max_fragments)
                    .ok_or_else(|| {
                        Failure::limit("L5110: page reference completed-pass records")
                    })?;
                total.spool_charge = total
                    .spool_charge
                    .checked_add(observation.spool_charge)
                    .filter(|n| *n <= caps.max_spool_bytes)
                    .ok_or_else(|| Failure::limit("D8101: page reference completed-pass spool"))?;
                for resolved in book.resolved_page_references() {
                    if next.len() == initial_values.len() {
                        return Err(Failure::internal(
                            "page reference feedback source count mismatch",
                        ));
                    }
                    next.push(resolved.map_err(map_production_internal_error)?);
                }
                next.sort_unstable_by_key(|value| value.0);
                if next.len() != values.len() || next.iter().zip(&values).any(|(a, b)| a.0 != b.0) {
                    return Err(Failure::internal(
                        "page reference feedback source identity mismatch",
                    ));
                }
                let state = (
                    observation.display_sha256,
                    book.selected().fingerprint(),
                    pdf.content_hash(),
                );
                let labels_match = next == values;
                if labels_match && previous == Some(state) {
                    let references = typaxis_pdf::seal_production_page_reference_pdf(
                        pdf,
                        book,
                        profile.base().authorization(),
                        admitted,
                        limits,
                        total.record_charge,
                    )
                    .map_err(|e| map_common_assembly_error("final page references", e))?;
                    references
                        .verify(pdf, book, profile.base().authorization(), admitted, limits)
                        .map_err(|e| {
                            map_common_assembly_error("final page reference identity", e)
                        })?;
                    total.record_charge = references.record_charge();
                    let inspect = inspect
                        .take()
                        .ok_or_else(|| Failure::internal("page reference consumer reused"))?;
                    return inspect(pdf, pages, book, book_pdf, observation, total).map(Some);
                }
                previous = if labels_match { Some(state) } else { None };
                Ok(None)
            },
        )?;
        if let Some(result) = result {
            return Ok(result);
        }
        std::mem::swap(&mut values, &mut next);
    }
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

fn map_common_content_error(error: typaxis_pdf::ProductionBodyPageError) -> Failure {
    use typaxis_pdf::{
        ProductionBodyPageError as E, ProductionBodyTextError as T,
        StagingSafeVectorPdfV2Error as V,
    };
    use typaxis_resources::{ResourceError, StagingSafeVectorResourceV2Error as F};
    let code = match error {
        E::OutputLimit | E::AllocationFailure => Some("D8101"),
        E::ReceiptMismatch => None,
        E::Text(cause) => match cause {
            T::RecordLimit => Some("L5110"),
            T::OutputLimit | T::AllocationFailure => Some("D8101"),
            T::ReceiptMismatch => None,
        },
        E::Forms(cause) => match cause {
            F::CountOverflow
            | F::RecordLimit
            | F::ObjectRoleCountOverflow
            | F::AllocationFailure => Some("D8101"),
            F::DisplayMismatch
            | F::CandidateMismatch
            | F::AliasMismatch(_)
            | F::LimitsMismatch
            | F::ReceiptMismatch => None,
        },
        E::Vectors(cause) => match cause {
            V::CountOverflow | V::SpoolLimit | V::AllocationFailure => Some("D8101"),
            V::DisplayMismatch
            | V::FormPlanMismatch
            | V::CandidateMismatch
            | V::InvalidIr
            | V::InvalidPlacement
            | V::ArithmeticOverflow
            | V::ContributionMismatch
            | V::FinalWriterMismatch
            | V::FinalPdfMismatch => None,
        },
        E::Rasters(ResourceError::ResourceLimit) => Some("G6100"),
        E::Rasters(_) => None,
    };
    common_pdf_failure("content", code, error)
}

fn map_common_font_error(error: typaxis_resources::ResourceError) -> Failure {
    use typaxis_font::Cff1Error as C;
    use typaxis_resources::ResourceError as E;
    match error {
        E::ResourceLimit => Failure::limit(format!("G6100: common footnote fonts: {error:?}")),
        E::Cff1(cause) => {
            let message = format!("{cause}; common footnote fonts");
            match cause {
                C::TableLimit
                | C::GlyphLimit
                | C::SubroutineLimit
                | C::CharstringOperationLimit
                | C::OutlineSegmentLimit
                | C::SelectedGlyphLimit
                | C::SubsetByteLimit => Failure::limit(message),
                C::InvalidGlyphClosure | C::ReceiptMismatch => Failure::internal(message),
                _ => Failure::input(message),
            }
        }
        E::MissingLogicalResource
        | E::ConflictingLogicalResource
        | E::DuplicateFontInstance
        | E::FontInstanceHashMismatch
        | E::InvalidFontPlan
        | E::DuplicatePlanKey
        | E::InvalidImagePlan
        | E::IncompleteUsagePlan
        | E::UnexpectedLogicalResource
        | E::NonCanonicalFontInstanceKey
        | E::AdmittedLedgerEpochMismatch => {
            Failure::internal(format!("I9190: common footnote fonts: {error:?}"))
        }
    }
}
