//! Safe-vector manifest from the selected common display and its actual PDF.
use super::*;
use std::collections::BTreeMap;
use typaxis_pdf::ProductionBodyAssemblyError as E;

#[derive(Debug)]
pub struct ProductionSafeVectorManifest {
    manifest: StagingSafeVectorManifestV2,
    record_charge: u64,
    spool_charge: u64,
}
impl ProductionSafeVectorManifest {
    pub fn manifest(&self) -> &StagingSafeVectorManifestV2 {
        &self.manifest
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
}

pub fn build_production_safe_vector_manifest(
    content: &typaxis_pdf::ProductionFootnotePageContent<
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
    profile: &StagingPrecomposedVectorProfileAuthorization,
    book: &crate::ProductionBookNavigationManifest,
    pdf: &typaxis_pdf::ProductionCommonTaggedPdf,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionSafeVectorManifest, E> {
    content
        .verify(content.plans().fonts(), admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    let structure = content.plans().fonts().structure();
    let display = structure.display();
    let source = display.source().line_layout().source_flow();
    let package = source.package();
    let navigation = source.navigation();
    let epoch = display.source().line_layout().binding_epoch();
    profile
        .authorizes(package, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    if pdf.package_sha256() != package.canonical_jcs_sha256()
        || pdf.limits_sha256() != limits.fingerprint()
        || pdf.display_sha256() != display.fingerprint()
        || pdf.structure_sha256() != structure.fingerprint()
        || epoch.profile_fingerprint() != profile.profile_receipt_fingerprint()
        || epoch.profile_authorization_fingerprint() != profile.profile_fingerprint()
        || book.manifest().final_pdf_sha256() != pdf.final_pdf().content_hash()
        || book.manifest().pdf_observation_sha256() != pdf.book_navigation().fingerprint()
        || book.record_charge() < pdf.record_charge()
        || book.spool_charge() < pdf.spool_charge()
    {
        return Err(E::ReceiptMismatch);
    }
    let candidates = content.plans().registry();
    let plans = content.plans().forms();
    let contribution = content.vectors();
    let final_writer = pdf.vector_final_writer();
    if pdf.safe_vector().contribution_fingerprint() != contribution.fingerprint()
        || final_writer.contribution_fingerprint() != contribution.fingerprint()
        || pdf.safe_vector().final_writer_observation_fingerprint() != final_writer.fingerprint()
        || pdf.safe_vector().final_pdf_sha256() != pdf.final_pdf().content_hash()
    {
        return Err(E::ReceiptMismatch);
    }
    let count = contribution.usages().len() as u64;
    let declarations = package.resources();
    let resource_count = declarations.images.len() as u64 + declarations.font_faces.len() as u64;
    // Retained resource/alias/placement rows and temporary content/image/role/
    // flow joins. All joins are linear-sized; none reserves placements per alias.
    let record_charge = book
        .record_charge()
        .checked_add(count.checked_mul(8).ok_or(E::RecordLimit)?)
        .and_then(|n| n.checked_add(resource_count.checked_mul(20)?))
        .and_then(|n| {
            n.checked_add((display.source().registry().flows().len() as u64).checked_mul(2)?)
        })
        .and_then(|n| n.checked_add((final_writer.object_table().len() as u64).checked_mul(2)?))
        .and_then(|n| n.checked_add(8))
        .ok_or(E::RecordLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
    // Each placement is encoded both independently and in the root record.
    // Resource/alias records, declaration closure and transient attestation
    // encodings have fixed fields plus the source strings charged below.
    let mut extra = count
        .checked_mul(2 * 4096 + 140)
        .and_then(|n| n.checked_add(resource_count.checked_mul(16384)?))
        .and_then(|n| n.checked_add(4096))
        .ok_or(E::SpoolLimit)?;
    let mut charge_text = |text: &str| -> Result<(), E> {
        extra = extra
            .checked_add((text.len() as u64).checked_mul(16).ok_or(E::SpoolLimit)?)
            .ok_or(E::SpoolLimit)?;
        Ok(())
    };
    for font in &declarations.font_faces {
        charge_text(font.uri.as_str())?;
    }
    for image in &declarations.images {
        charge_text(image.uri.as_str())?;
        if let Some(p) = &image.vector_provenance {
            for text in [&p.engine_id, &p.engine_version, &p.rules_version] {
                charge_text(text)?;
            }
        }
    }
    for draw in display.draws() {
        if let typaxis_display_list::ProductionBodyDraw::Vector(vector) = draw {
            charge_text(
                navigation
                    .languages()
                    .record(vector.binding().node_id())
                    .ok_or(E::ReceiptMismatch)?
                    .effective_language
                    .as_ref(),
            )?;
        }
    }
    let spool_charge = book
        .spool_charge()
        .checked_add(extra)
        .ok_or(E::SpoolLimit)?;
    if spool_charge > limits.base().get().max_spool_bytes {
        return Err(E::SpoolLimit);
    }
    let media =
        close_staging_declared_media(admitted, declarations).map_err(|_| E::ReceiptMismatch)?;
    let media_by_id: BTreeMap<_, _> = media.images().iter().map(|m| (m.image_id(), m)).collect();
    let absolute_roles: BTreeMap<_, _> = final_writer
        .object_table()
        .iter()
        .map(|row| (row.relative_object_role(), row.absolute_object_number()))
        .collect();
    let form_by_key: BTreeMap<_, _> = plans
        .plans()
        .iter()
        .map(|plan| (*plan.content_key(), plan))
        .collect();
    let flows: BTreeMap<_, _> = display
        .source()
        .registry()
        .flows()
        .iter()
        .map(|flow| (flow.owner(), flow))
        .collect();
    let mut by_content =
        BTreeMap::<VectorContentKey, Vec<StagingSafeVectorManifestPlacementV2>>::new();
    let mut placement_count = 0u32;
    for (paint, draw) in display.draws().iter().enumerate() {
        let typaxis_display_list::ProductionBodyDraw::Vector(vector) = draw else {
            continue;
        };
        let binding = vector.binding();
        let owner = binding.node_id();
        let usage = contribution
            .usages()
            .get(placement_count as usize)
            .ok_or(E::ReceiptMismatch)?;
        let observed = final_writer
            .usages()
            .get(placement_count as usize)
            .ok_or(E::ReceiptMismatch)?;
        if usage.usage_id() != placement_count
            || observed.usage_id() != placement_count
            || usage.image_id() != binding.resource().image_id()
            || *usage.content_key() != vector.content_key()
            || usage.page_index() != vector.page_index()
            || usage.paint_ordinal() as usize != paint
            || usage.semantic_hook().kind() != binding.kind().into()
            || usage.semantic_hook().owner() != owner
            || usage.semantic_hook().display_command_fingerprint() != vector.fingerprint()
            || observed.page_index() != vector.page_index()
            || observed.paint_ordinal() as usize != paint
            || observed.content_fingerprint() != usage.content_fingerprint()
        {
            return Err(E::ReceiptMismatch);
        }
        let group_index = structure
            .groups()
            .partition_point(|g| g.draws().end <= paint);
        let group = structure
            .groups()
            .get(group_index)
            .ok_or(E::ReceiptMismatch)?;
        if group.vector_usage_id() != Some(placement_count) || !group.draws().contains(&paint) {
            return Err(E::ReceiptMismatch);
        }
        let kind: StagingCombinedVectorKindV2 = binding.kind().into();
        let authored_actual_text_sha256 = if kind == StagingCombinedVectorKindV2::InlineVector {
            package
                .precomposed_vector_metrics_for(owner)
                .ok_or(E::ReceiptMismatch)?
                .alternative()
                .authored_actual_text_sha256()
        } else {
            None
        };
        let details = match binding.placement() {
            PrecomposedVectorPlacementInput::Inline(value) => {
                StagingSafeVectorPlacementDetailsV2::Inline {
                    metrics: metric_fact(value.metrics()),
                    spacing_before: value.spacing_before().get().raw(),
                    spacing_after: value.spacing_after().get().raw(),
                }
            }
            PrecomposedVectorPlacementInput::VectorFigure(value) => {
                StagingSafeVectorPlacementDetailsV2::VectorFigure {
                    style_fingerprint: value.style().fingerprint(),
                    alignment: value.style().text_align().as_str(),
                    space_before: value.style().space_before().get().raw(),
                    space_after: value.style().space_after().get().raw(),
                    start_indent: value.style().start_indent().get().raw(),
                    end_indent: value.style().end_indent().get().raw(),
                    keep_caption: value.style().keep_caption(),
                    keep_with_next: value.style().keep_with_next(),
                }
            }
            PrecomposedVectorPlacementInput::MathVectorBlock(value) => {
                let flow = flows.get(&owner).ok_or(E::ReceiptMismatch)?;
                let terminal = display
                    .source()
                    .terminals()
                    .receipts()
                    .get(flow.flow_id().get() as usize)
                    .ok_or(E::ReceiptMismatch)?;
                if terminal.owner() != owner
                    || terminal.flow_id() != flow.flow_id()
                    || terminal.flow_fingerprint() != flow.fingerprint()
                    || terminal.terminal() != flow.terminal()
                {
                    return Err(E::ReceiptMismatch);
                }
                StagingSafeVectorPlacementDetailsV2::MathVectorBlock {
                    metrics: metric_fact(value.metrics()),
                    style_fingerprint: value.style().fingerprint(),
                    alignment: value.style().text_align().as_str(),
                    space_before: value.style().space_before().get().raw(),
                    space_after: value.style().space_after().get().raw(),
                    start_indent: value.style().start_indent().get().raw(),
                    end_indent: value.style().end_indent().get().raw(),
                    keep_with_next: value.style().keep_with_next(),
                    flow_id: flow.flow_id().get(),
                    flow_fingerprint: flow.fingerprint(),
                    parent_flow_id: flow.parent_flow_id().get(),
                    parent_position: flow.parent_position(),
                    terminal: terminal.terminal().get(),
                    terminal_receipt_fingerprint: terminal.fingerprint(),
                }
            }
        };
        let span = binding.owner_source_span();
        let mut placement = StagingSafeVectorManifestPlacementV2 {
            usage_id: placement_count,
            owner,
            kind,
            image_id: binding.resource().image_id(),
            source_id: span.source_id().get(),
            source_start: span.start_byte().get(),
            source_end: span.end_byte().get(),
            alternative_sha256: binding.alternative_sha256(),
            authored_actual_text_sha256,
            language: navigation
                .languages()
                .record(owner)
                .ok_or(E::ReceiptMismatch)?
                .effective_language
                .to_string(),
            page_index: vector.page_index(),
            frame_index: vector.fragment_index(),
            fragment_ordinal: group.semantic_fragment_ordinal(),
            paint_ordinal: usage.paint_ordinal(),
            viewport: vector.viewport(),
            scale: vector.scale_raw(),
            matrix: vector.matrix(),
            metric_receipt_fingerprint: Some(binding.metrics_fingerprint()),
            binding_fingerprint: Some(binding.fingerprint()),
            // The common selected draw binds its source and physical placement
            // in one receipt, rather than separate staging placement/paint rows.
            selected_placement_fingerprint: vector.fingerprint(),
            display_command_fingerprint: vector.fingerprint(),
            pdf_use_fingerprint: usage.content_fingerprint(),
            pdf_page_object_number: observed.page_object_number(),
            pdf_content_object_number: observed.page_content_object_number(),
            pdf_form_object_number: observed.form_absolute_object_number(),
            details,
            canonical_jcs: String::new(),
            fingerprint: [0; 32],
        };
        placement.canonical_jcs = encode_placement(&placement);
        if placement.canonical_jcs.len() as u64 > 4096 + 6 * placement.language.len() as u64 {
            return Err(E::SpoolLimit);
        }
        placement.fingerprint = sha256(placement.canonical_jcs.as_bytes());
        by_content
            .entry(vector.content_key())
            .or_default()
            .push(placement);
        placement_count = placement_count.checked_add(1).ok_or(E::RecordLimit)?;
    }
    if placement_count as usize != contribution.usages().len() {
        return Err(E::ReceiptMismatch);
    }
    let mut resources = Vec::new();
    resources
        .try_reserve_exact(candidates.candidates().len())
        .map_err(|_| E::AllocationFailure)?;
    for candidate in candidates.candidates() {
        let ir = candidate.canonical_ir();
        let mut placements = by_content.remove(candidate.key()).unwrap_or_default();
        placements.sort_unstable_by_key(|p| (p.page_index, p.paint_ordinal));
        let mut alias_usages = BTreeMap::<ImageResourceId, Vec<[u8; 32]>>::new();
        for p in &placements {
            alias_usages
                .entry(p.image_id)
                .or_default()
                .push(p.fingerprint);
        }
        let plan = form_by_key.get(candidate.key()).copied();
        if plan.is_some() != !placements.is_empty() {
            return Err(E::ReceiptMismatch);
        }
        let (form_plan_fingerprint, pdf_form_object_number, pdf_resource_name) = match plan {
            Some(plan) => {
                if plan.total_usage_count() as usize != placements.len() {
                    return Err(E::ReceiptMismatch);
                }
                (
                    Some(plan.fingerprint()),
                    Some(
                        *absolute_roles
                            .get(&plan.form_relative_object_role())
                            .ok_or(E::ReceiptMismatch)?,
                    ),
                    Some(plan.form_resource_name().to_owned()),
                )
            }
            None => (None, None, None),
        };
        let mut aliases = Vec::new();
        aliases
            .try_reserve_exact(candidate.aliases().len())
            .map_err(|_| E::AllocationFailure)?;
        for alias in candidate.aliases() {
            let image = admitted.image(alias.image_id()).ok_or(E::ReceiptMismatch)?;
            let attestation = media_by_id
                .get(&alias.image_id())
                .copied()
                .ok_or(E::ReceiptMismatch)?;
            if image.content_hash() != candidate.key().source_sha256()
                || attestation.content_hash() != image.content_hash()
                || attestation.uri() != alias.uri()
                || attestation.declared().as_str() != candidate.key().media_type().as_str()
                || attestation.attested().as_str() != candidate.key().media_type().as_str()
                || attestation.safe_vector_ir_fingerprint()
                    != Some(candidate.key().ir_fingerprint())
                || attestation.safe_vector_allocation_charge()
                    != Some(alias.admission_allocation_charge())
                || attestation.m4_limits_fingerprint() != Some(alias.limits_fingerprint())
                || attestation.m4_profile_fingerprint() != Some(alias.profile_fingerprint())
            {
                return Err(E::ReceiptMismatch);
            }
            let usage_fingerprints = alias_usages.remove(&alias.image_id()).unwrap_or_default();
            let provenance = match alias.provenance() {
                VectorContentAliasProvenance::SafeSvg1Absent => None,
                VectorContentAliasProvenance::SafeSvg2(value) => Some(value.clone()),
            };
            let attestation_fingerprint = declared_image_attestation_fingerprint(attestation);
            aliases.push(StagingSafeVectorManifestAliasV2 {
                image_id: alias.image_id(),
                uri: alias.uri().as_str().to_owned(),
                expected_sha256: alias.expected_sha256(),
                admitted_sha256: alias.admitted_sha256(),
                admission_attestation_fingerprint: attestation_fingerprint,
                allocation_charge: alias.admission_allocation_charge(),
                provenance,
                placement_count: u32::try_from(usage_fingerprints.len())
                    .map_err(|_| E::RecordLimit)?,
                usage_fingerprints,
            });
        }
        if !alias_usages.is_empty() {
            return Err(E::ReceiptMismatch);
        }
        aliases.sort_unstable_by_key(|value| value.image_id);
        let byte_length = candidate
            .aliases()
            .first()
            .and_then(|alias| admitted.image(alias.image_id()))
            .map(|image| image.byte_length())
            .ok_or(E::ReceiptMismatch)?;
        resources.push(StagingSafeVectorManifestResourceV2 {
            content_key: *candidate.key(),
            svg_byte_length: byte_length,
            parser_id: ir.parser_id(),
            ir_id: ir.ir_id(),
            ir_fingerprint_id: ir.ir_fingerprint_id(),
            allocation_charge_id: ir.allocation_charge_id(),
            allocation_charge: ir.allocation_charge(),
            intrinsic_width: candidate.intrinsic_width().get().raw(),
            intrinsic_height: candidate.intrinsic_height().get().raw(),
            view_box: candidate.view_box(),
            aliases,
            total_placement_count: u32::try_from(placements.len()).map_err(|_| E::RecordLimit)?,
            placements,
            form_plan_fingerprint,
            pdf_form_object_number,
            pdf_resource_name,
        });
    }

    if !by_content.is_empty() {
        return Err(E::ReceiptMismatch);
    }
    let canonical_jcs = encode_manifest_facts(
        [
            ("admitted_sha256", admitted.fingerprint().bytes()),
            (
                "candidate_registry_sha256",
                candidates.receipt().fingerprint(),
            ),
            ("display_sha256", display.fingerprint()),
            ("final_writer_sha256", final_writer.fingerprint()),
            ("form_plans_sha256", plans.fingerprint()),
            ("limits_sha256", limits.fingerprint()),
            ("package_sha256", package.semantic_fingerprint()),
            ("pdf_closure_sha256", pdf.safe_vector().fingerprint()),
            ("pdf_contribution_sha256", contribution.fingerprint()),
            ("pdf_sha256", pdf.final_pdf().content_hash()),
            ("profile_sha256", profile.profile_fingerprint()),
        ],
        placement_count,
        &resources,
    );
    if canonical_jcs.len() as u64 > extra {
        return Err(E::SpoolLimit);
    }
    let manifest = StagingSafeVectorManifestV2 {
        resources,
        package_fingerprint: package.semantic_fingerprint(),
        profile_fingerprint: profile.profile_fingerprint(),
        limits_fingerprint: limits.fingerprint(),
        admitted_fingerprint: admitted.fingerprint().bytes(),
        candidate_registry_fingerprint: candidates.receipt().fingerprint(),
        display_fingerprint: display.fingerprint(),
        form_plans_fingerprint: plans.fingerprint(),
        pdf_contribution_fingerprint: contribution.fingerprint(),
        final_writer_fingerprint: final_writer.fingerprint(),
        pdf_closure_fingerprint: pdf.safe_vector().fingerprint(),
        final_pdf_sha256: pdf.final_pdf().content_hash(),
        placement_count,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    Ok(ProductionSafeVectorManifest {
        manifest,
        record_charge,
        spool_charge,
    })
}
