// Own the common body graph through stable line selection and exact PDF
// assembly. This is preparation for the public writer, not its publication
// receipt: page/generated-reference convergence and manifest closure remain.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProductionCommonBodyObservation {
    pub line_reshape_passes: usize,
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
            let selected =
                typaxis_pagination::paginate_production_body(stable.lines(), &blocks, limits)
                    .map_err(map_production_input_error)?;
            let selected = typaxis_pagination::finalize_production_body_math_terminals(
                selected, &math, limits,
            )
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
            inspect(&pdf, observation)
        },
    )
    .map_err(map_production_input_error)?
}
