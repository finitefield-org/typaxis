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
        &typaxis_display_list::ProductionFootnoteBookNavigationInputs<
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
            .map_err(map_production_input_error)?;
            let remaining_steps = max_candidate_steps
                .checked_sub(stable.candidate_steps())
                .ok_or_else(|| Failure::input("common footnote candidate budget exhausted"))?;
            let mut search = typaxis_pagination::prepare_production_footnote_demand_search(
                &prepared,
                limits,
                remaining_steps,
            )
            .map_err(map_production_input_error)?;
            let pages = search
                .select_stable_pages()
                .map_err(map_production_input_error)?;
            let geometry = search
                .place_pages_content(pages.sequence())
                .map_err(map_production_input_error)?;
            let terminals = search
                .finalize_page_math(&pages, &geometry, &math, limits)
                .map_err(map_production_internal_error)?;
            terminals
                .terminals()
                .verify(&math)
                .map_err(map_production_internal_error)?;
            let display = typaxis_display_list::build_production_footnote_display(
                &terminals, admitted, limits,
            )
            .map_err(|e| Failure::internal(format!("common footnote display: {e:?}")))?;
            let structure = typaxis_display_list::build_production_footnote_structure(
                &display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .map_err(|e| Failure::internal(format!("common footnote structure: {e:?}")))?;
            let fonts =
                typaxis_resources::finalize_production_footnote_fonts(&structure, admitted, limits)
                    .map_err(|e| Failure::input(format!("common footnote fonts: {e:?}")))?;
            let content =
                typaxis_pdf::build_production_footnote_page_content(&fonts, admitted, limits)
                    .map_err(|e| Failure::internal(format!("common footnote content: {e:?}")))?;
            let marked = typaxis_pdf::build_production_footnote_marked_content(
                &content, admitted, limits,
            )
            .map_err(|e| Failure::internal(format!("common footnote marked content: {e:?}")))?;
            let annotations =
                typaxis_pdf::build_production_footnote_annotations(&marked, admitted, limits)
                    .map_err(|e| {
                        Failure::internal(format!("common footnote annotations: {e:?}"))
                    })?;
            let structure_objects = typaxis_pdf::build_production_footnote_structure_objects(
                &annotations,
                admitted,
                limits,
            )
            .map_err(|e| Failure::internal(format!("common footnote structure objects: {e:?}")))?;
            let resources = typaxis_pdf::build_production_footnote_resource_objects(
                &structure_objects,
                admitted,
                limits,
            )
            .map_err(|e| Failure::internal(format!("common footnote resources: {e:?}")))?;
            let pdf = typaxis_pdf::assemble_production_footnote_pdf(&resources, admitted, limits)
                .map_err(|e| Failure::internal(format!("common footnote assembly: {e:?}")))?;
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
            let observation = ProductionCommonFootnoteObservation {
                line_reshape_passes: stable.passes().len(),
                page_passes: pages.passes(),
                line_candidate_steps: stable.candidate_steps(),
                page_work_steps: search.work_steps(),
                record_charge: book_inputs.record_charge(),
                spool_charge: book_inputs.spool_charge(),
                display_sha256: display.fingerprint(),
                flow_registry_sha256: math.receipt().fingerprint(),
                block_math_terminals: terminals.terminals().receipts().len(),
            };
            inspect(&pdf, &pages, &book_inputs, observation)
        },
    )
    .map_err(map_production_input_error)?
}
