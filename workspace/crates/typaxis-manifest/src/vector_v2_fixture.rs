use typaxis_core::{sha256, EffectiveConfigFingerprint, EngineIdentity, Length, Point};
use typaxis_display_list::{
    build_staging_combined_vector_display_v2, build_structure_registry_v2,
    build_vector_marked_content_plan_v2, prove_vector_form_structure_isolation_v2,
    select_staging_book_navigation_v2, staging_combined_vector_figure_fixture,
    staging_precomposed_vector_tagged_pdf_fixture, BookNavigationDestinationBinding,
    BookNavigationSelectedPage, BookNavigationSelectedReceiptV2, DestinationView,
    MarkedContentStandardPaintInputV2, NamedDestination, SelectedStructurePaintOwner,
    StagingCombinedVectorDisplayV2, StagingCombinedVectorFigureFixture,
    StagingPrecomposedVectorDisplayFixture, StructureRegistryReceiptV2,
    VectorFormStructureIsolationReceiptV2, VectorMarkedContentPlanV2,
};
use typaxis_pdf::{
    build_staging_combined_safe_vector_pdf_contribution_v2,
    write_staging_tagged_pdf_v2_with_combined_vectors, StagingSafeVectorPdfContributionV2,
    StagingTaggedPdfV2,
};
use typaxis_resources::{
    finalize_staging_combined_safe_vector_forms_v2, StagingSafeVectorFormPlansV2,
    VectorContentCandidateRegistry,
};
use typaxis_syntax::{
    validate_staging_book_navigation_v2, validate_staging_structure_semantics_v2,
    StagingAccessibilityProfileAuthorizationV2, StagingAccessibilityProfileViewV2,
    StagingBookNavigationProfileAuthorizationV2, StagingBookNavigationProfileViewV2,
    ValidatedStagingBookNavigationV2, ValidatedStagingStructureSemanticsV2,
};

pub struct ManifestVectorV2Fixture {
    pub display: StagingPrecomposedVectorDisplayFixture,
    pub navigation: ValidatedStagingBookNavigationV2,
    pub semantics: ValidatedStagingStructureSemanticsV2,
    pub accessibility: StagingAccessibilityProfileAuthorizationV2,
    pub book_profile: StagingBookNavigationProfileAuthorizationV2,
    pub book: BookNavigationSelectedReceiptV2,
    pub registry: StructureRegistryReceiptV2,
    pub form_isolation: VectorFormStructureIsolationReceiptV2,
    pub vector_plan: VectorMarkedContentPlanV2,
    pub combined_display: StagingCombinedVectorDisplayV2,
    pub candidates: VectorContentCandidateRegistry,
    pub form_plans: StagingSafeVectorFormPlansV2,
    pub contribution: StagingSafeVectorPdfContributionV2,
    pub pdf: StagingTaggedPdfV2,
}

pub struct ManifestVectorV2Products {
    pub safe: crate::StagingSafeVectorManifestV2,
    pub math: crate::StagingMathVectorManifest,
    pub book: crate::StagingBookNavigationManifestV2,
    pub tagged: crate::StagingTaggedPdfManifestV2,
}

pub struct ManifestFigureVectorV2Fixture {
    pub display: StagingCombinedVectorFigureFixture,
    pub book: BookNavigationSelectedReceiptV2,
    pub form_isolation: VectorFormStructureIsolationReceiptV2,
    pub vector_plan: VectorMarkedContentPlanV2,
    pub candidates: VectorContentCandidateRegistry,
    pub form_plans: StagingSafeVectorFormPlansV2,
    pub contribution: StagingSafeVectorPdfContributionV2,
    pub pdf: StagingTaggedPdfV2,
}

pub fn build_vector_v2_manifests(
    fixture: &ManifestVectorV2Fixture,
) -> Result<ManifestVectorV2Products, Box<dyn std::error::Error>> {
    let package = &fixture.display.layout.package;
    let limits = &fixture.display.layout.limits;
    let safe = crate::build_staging_safe_vector_manifest_v2(
        package,
        &fixture.display.layout.profile,
        limits,
        &fixture.display.layout.admitted,
        &fixture.display.layout.bindings,
        &fixture.navigation,
        &fixture.combined_display,
        &fixture.candidates,
        &fixture.form_plans,
        &fixture.contribution,
        &fixture.pdf,
    )?;
    let math = crate::build_staging_math_vector_manifest(
        package,
        &fixture.display.layout.bindings,
        &safe,
    )?;
    let book = crate::build_staging_book_navigation_manifest_v2(
        package,
        &fixture.navigation,
        &fixture.book_profile,
        &fixture.book,
        &fixture.display.display,
        &fixture.pdf,
        limits,
        &EngineIdentity::compiled(),
    )?;
    let tagged = crate::build_staging_tagged_pdf_manifest_v2(
        package,
        &fixture.navigation,
        &fixture.semantics,
        &fixture.accessibility,
        &fixture.book_profile,
        &fixture.book,
        &fixture.registry,
        &fixture.vector_plan,
        &fixture.display.display,
        &fixture.form_isolation,
        &fixture.display.block_selected,
        &fixture.display.layout.math_flows,
        &fixture.pdf,
        &safe,
        &math,
        limits,
        &EngineIdentity::compiled(),
    )?;
    Ok(ManifestVectorV2Products {
        safe,
        math,
        book,
        tagged,
    })
}

pub fn build_figure_vector_v2_manifests(
    fixture: &ManifestFigureVectorV2Fixture,
) -> Result<ManifestVectorV2Products, Box<dyn std::error::Error>> {
    let package = &fixture.display.figure.layout.package;
    let limits = &fixture.display.figure.layout.limits;
    let safe = crate::build_staging_safe_vector_manifest_v2(
        package,
        &fixture.display.profile,
        limits,
        &fixture.display.figure.layout.admitted,
        &fixture.display.bindings,
        &fixture.display.navigation,
        &fixture.display.display,
        &fixture.candidates,
        &fixture.form_plans,
        &fixture.contribution,
        &fixture.pdf,
    )?;
    let math =
        crate::build_staging_math_vector_manifest(package, &fixture.display.bindings, &safe)?;
    let book = crate::build_staging_book_navigation_manifest_v2(
        package,
        &fixture.display.navigation,
        &fixture.display.book_profile,
        &fixture.book,
        &fixture.display.precomposed,
        &fixture.pdf,
        limits,
        &EngineIdentity::compiled(),
    )?;
    let tagged = crate::build_staging_tagged_pdf_manifest_v2(
        package,
        &fixture.display.navigation,
        &fixture.display.semantics,
        &fixture.display.accessibility,
        &fixture.display.book_profile,
        &fixture.book,
        &fixture.display.registry,
        &fixture.vector_plan,
        &fixture.display.precomposed,
        &fixture.form_isolation,
        &fixture.display.block_selected,
        &fixture.display.math_flows,
        &fixture.pdf,
        &safe,
        &math,
        limits,
        &EngineIdentity::compiled(),
    )?;
    Ok(ManifestVectorV2Products {
        safe,
        math,
        book,
        tagged,
    })
}

pub fn manifest_vector_v2_fixture() -> Result<ManifestVectorV2Fixture, Box<dyn std::error::Error>> {
    const SCALE: i64 = 65_536;
    let display = staging_precomposed_vector_tagged_pdf_fixture()?;
    let package = &display.layout.package;
    let limits = &display.layout.limits;
    let navigation = validate_staging_book_navigation_v2(package, limits)?;
    let semantics = validate_staging_structure_semantics_v2(package, &navigation, limits)?;
    let book_profile = StagingBookNavigationProfileAuthorizationV2::bind_profile_receipt(
        StagingBookNavigationProfileViewV2::new(package, &navigation, limits)?,
        sha256(b"manifest-vector-navigation-profile-v2"),
        display.layout.profile.profile_receipt_fingerprint(),
        display.layout.profile.profile_fingerprint(),
        package,
        &navigation,
        limits,
    )?;
    let accessibility = StagingAccessibilityProfileAuthorizationV2::bind_profile_receipt(
        StagingAccessibilityProfileViewV2::new(package, &navigation, &semantics, limits)?,
        sha256(b"manifest-vector-accessibility-profile-v2"),
        book_profile.profile_receipt_fingerprint(),
        package,
        &navigation,
        &semantics,
        limits,
    )?;
    let pages = display
        .display
        .pages()
        .iter()
        .map(|page| BookNavigationSelectedPage {
            page_index: page.page_index(),
            width_raw: 1_000 * SCALE,
            height_raw: 800 * SCALE,
        })
        .collect::<Vec<_>>();
    let destinations = navigation
        .anchors()
        .iter()
        .enumerate()
        .map(|(index, (anchor, owner))| {
            Ok(BookNavigationDestinationBinding {
                source_node_id: *owner,
                frame_id: u32::try_from(index)?,
                destination: NamedDestination {
                    anchor_id: anchor.clone(),
                    page_index: 0,
                    view: DestinationView::Xyz {
                        point: Point {
                            x: Length::ZERO,
                            y: Length::ZERO,
                        },
                    },
                },
            })
        })
        .collect::<Result<Vec<_>, std::num::TryFromIntError>>()?;
    let book = select_staging_book_navigation_v2(
        &navigation,
        &book_profile,
        limits,
        sha256(b"manifest-vector-complete-layout-v2"),
        4,
        &pages,
        &destinations,
        &[],
        &[],
        &display.display,
    )?;
    let registry =
        build_structure_registry_v2(package, &navigation, &semantics, &accessibility, limits)?;
    let form_isolation = prove_vector_form_structure_isolation_v2(&display.display)?;
    let vector_plan = build_vector_marked_content_plan_v2(
        &registry,
        &accessibility,
        limits,
        &navigation,
        &book_profile,
        &book,
        &[],
        &[],
        &display.display,
        &form_isolation,
        &display.block_selected,
        &display.layout.math_flows,
    )?;
    let combined_display = build_staging_combined_vector_display_v2(
        package,
        &display.display,
        None,
        &display.layout.admitted,
        &registry,
        vector_plan.selected_binding(),
    )?;
    let candidates = VectorContentCandidateRegistry::from_admitted(
        &display.layout.admitted,
        package.resources(),
    )?;
    let form_plans =
        finalize_staging_combined_safe_vector_forms_v2(&combined_display, &candidates, limits)?;
    let contribution = build_staging_combined_safe_vector_pdf_contribution_v2(
        &combined_display,
        &form_plans,
        &candidates,
        limits,
    )?;
    let serialization = vector_plan.authorize_pdf_serialization(
        &registry,
        &accessibility,
        limits,
        &navigation,
        &book_profile,
        &book,
        &display.display,
        &form_isolation,
        &display.block_selected,
        &display.layout.math_flows,
    )?;
    let pdf = write_staging_tagged_pdf_v2_with_combined_vectors(
        package,
        &navigation,
        &semantics,
        &accessibility,
        &book_profile,
        &book,
        &registry,
        serialization,
        &display.display,
        &combined_display,
        &form_isolation,
        &display.layout.admitted,
        &form_plans,
        &candidates,
        &contribution,
        limits,
        &EngineIdentity::compiled(),
        EffectiveConfigFingerprint::from_untrusted_bytes(sha256(
            b"manifest-vector-effective-config-v2",
        )),
    )?;
    Ok(ManifestVectorV2Fixture {
        display,
        navigation,
        semantics,
        accessibility,
        book_profile,
        book,
        registry,
        form_isolation,
        vector_plan,
        combined_display,
        candidates,
        form_plans,
        contribution,
        pdf,
    })
}

pub fn manifest_figure_vector_v2_fixture(
) -> Result<ManifestFigureVectorV2Fixture, Box<dyn std::error::Error>> {
    let display = staging_combined_vector_figure_fixture()?;
    let package = &display.figure.layout.package;
    let limits = &display.figure.layout.limits;
    let geometry = display.figure.display.page_geometry();
    let pages = display
        .figure
        .display
        .pages()
        .iter()
        .map(|page| BookNavigationSelectedPage {
            page_index: page.page_index(),
            width_raw: geometry.page_width().get().raw(),
            height_raw: geometry.page_height().get().raw(),
        })
        .collect::<Vec<_>>();
    let book = select_staging_book_navigation_v2(
        &display.navigation,
        &display.book_profile,
        limits,
        display
            .figure
            .display
            .receipt()
            .selected_layout_fingerprint(),
        u64::from(display.figure.display.receipt().command_count()),
        &pages,
        &[],
        &[],
        &[],
        &display.precomposed,
    )?;
    let standard_paints = display
        .figure
        .display
        .commands()
        .map(|command| {
            let node = display
                .registry
                .source_node(command.owner())
                .ok_or("Figure has no structure node")?;
            Ok(MarkedContentStandardPaintInputV2 {
                page_index: command.page_index(),
                paint_ordinal: command.occurrence(),
                semantic_fragment_ordinal: 0,
                owner: SelectedStructurePaintOwner::Structure(node.structure_node_id()),
            })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let form_isolation = prove_vector_form_structure_isolation_v2(&display.precomposed)?;
    let vector_plan = build_vector_marked_content_plan_v2(
        &display.registry,
        &display.accessibility,
        limits,
        &display.navigation,
        &display.book_profile,
        &book,
        &standard_paints,
        &[],
        &display.precomposed,
        &form_isolation,
        &display.block_selected,
        &display.math_flows,
    )?;
    if vector_plan.selected_binding() != &display.selected {
        return Err("Figure selected structure binding diverged".into());
    }
    let candidates = VectorContentCandidateRegistry::from_admitted(
        &display.figure.layout.admitted,
        package.resources(),
    )?;
    let form_plans =
        finalize_staging_combined_safe_vector_forms_v2(&display.display, &candidates, limits)?;
    let contribution = build_staging_combined_safe_vector_pdf_contribution_v2(
        &display.display,
        &form_plans,
        &candidates,
        limits,
    )?;
    let serialization = vector_plan.authorize_pdf_serialization(
        &display.registry,
        &display.accessibility,
        limits,
        &display.navigation,
        &display.book_profile,
        &book,
        &display.precomposed,
        &form_isolation,
        &display.block_selected,
        &display.math_flows,
    )?;
    let pdf = write_staging_tagged_pdf_v2_with_combined_vectors(
        package,
        &display.navigation,
        &display.semantics,
        &display.accessibility,
        &display.book_profile,
        &book,
        &display.registry,
        serialization,
        &display.precomposed,
        &display.display,
        &form_isolation,
        &display.figure.layout.admitted,
        &form_plans,
        &candidates,
        &contribution,
        limits,
        &EngineIdentity::compiled(),
        EffectiveConfigFingerprint::from_untrusted_bytes(sha256(
            b"manifest-figure-vector-effective-config-v2",
        )),
    )?;
    Ok(ManifestFigureVectorV2Fixture {
        display,
        book,
        form_isolation,
        vector_plan,
        candidates,
        form_plans,
        contribution,
        pdf,
    })
}
