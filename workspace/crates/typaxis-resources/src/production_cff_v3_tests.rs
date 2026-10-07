use super::*;
use typaxis_core::{
    sha256, BidiLevel, ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions, FontFaceId,
    GlyphRunId, HostAdmissionContext, HostPath, Length, M4EffectiveResourceLimits,
    M4ResourceLimits, OpenTypeTag, PdfStreamCompression, PortablePath, PositiveLength,
    ResourceLimits, TextBufferId, TextSpan, Utf8ByteOffset, ValidatedResourceLimits,
    DEFAULT_ALLOWED_URI_SCHEMES, REGISTERED_JAPANESE_LINE_BREAK_VERSION,
    REGISTERED_UNICODE_VERSION,
};
use typaxis_document::{
    FontMediaDeclaration, FontMediaType, StagingM4FontFaceDeclaration, StagingM4ResourceCatalog,
};
use typaxis_resource_admission::{
    staging_declared_base_catalog, AdmittedProductionFontInstancesV3, HostResourceAdmissionSession,
    StagingProductionResourceResolverV3,
};
use typaxis_shaping::{
    shape_production_run_v3, ProductionShapeErrorV3, ProductionShapeInputV3, ShapeSourceSpan,
};
const BODY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"
));
struct Temp(std::path::PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn ledger(cff: Option<&[u8]>) -> AdmittedProductionResourceLedgerV3 {
    let dir = Temp(std::env::temp_dir().join(format!(
            "typaxis-v3-shape-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    std::fs::create_dir(&dir.0).unwrap();
    std::fs::write(dir.0.join("body.bin"), BODY).unwrap();
    let mut fonts = vec![StagingM4FontFaceDeclaration {
        font_face_id: FontFaceId::new(0),
        family: "Body".into(),
        uri: PortablePath::new("body.bin").unwrap(),
        face_index: 0,
        expected_sha256: Some(sha256(BODY)),
        media: FontMediaDeclaration::Declared(FontMediaType::SfntTrueTypeGlyf),
    }];
    if let Some(bytes) = cff {
        std::fs::write(dir.0.join("harano.bin"), bytes).unwrap();
        fonts.push(StagingM4FontFaceDeclaration {
            font_face_id: FontFaceId::new(1),
            family: "Harano".into(),
            uri: PortablePath::new("harano.bin").unwrap(),
            face_index: 0,
            expected_sha256: Some(sha256(bytes)),
            media: FontMediaDeclaration::Declared(FontMediaType::SfntCff1),
        });
    }
    let declared = staging_declared_base_catalog(&StagingM4ResourceCatalog {
        font_faces: fonts,
        images: vec![],
    })
    .unwrap();
    let config = EffectiveConfig::new(
        false,
        PdfStreamCompression::Flate,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        EffectiveDataVersions::new(
            REGISTERED_UNICODE_VERSION,
            REGISTERED_JAPANESE_LINE_BREAK_VERSION,
        )
        .unwrap(),
        ResourceLimits::default(),
    )
    .unwrap();
    let context = HostAdmissionContext::new(
        HostPath::new(dir.0.join("input.typ")).unwrap(),
        HostPath::new(dir.0.clone()).unwrap(),
        None,
        vec![],
    );
    let host =
        HostResourceAdmissionSession::new(&context, &config, declared.resource_catalog()).unwrap();
    let limits = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap();
    let mut resolver = StagingProductionResourceResolverV3::new(
        &declared,
        &limits,
        sha256(b"production-v3-test-profile"),
        host.roots(),
    )
    .unwrap();
    for id in 0..declared.font_faces.len() as u32 {
        let pending = resolver
            .read_font(host.open_font(FontFaceId::new(id)).unwrap())
            .unwrap();
        resolver.parse_and_bind_font(pending).unwrap();
    }
    resolver.finish().unwrap()
}
fn input(text: &str, id: u32, size: i64) -> ProductionShapeInputV3<'_> {
    ProductionShapeInputV3 {
        run_id: GlyphRunId::new(id),
        source: ShapeSourceSpan::Parsed(
            TextSpan::new(
                TextBufferId::new(9),
                Utf8ByteOffset::new(31),
                Utf8ByteOffset::new(31 + text.len() as u32),
            )
            .unwrap(),
        ),
        utf8: text,
        font_size: PositiveLength::new(Length::from_raw(size * 65536).unwrap()).unwrap(),
        bidi_level: BidiLevel::LTR,
        script: OpenTypeTag::new(*b"Hani").unwrap(),
        language: Some("ja"),
        pre_context: None,
        post_context: None,
    }
}
#[test]
fn production_v3_true_type_shaping_uses_instance_and_exact_source() {
    let ledger = ledger(None);
    let table =
        AdmittedProductionFontInstancesV3::from_used_faces(&ledger, [FontFaceId::new(0)]).unwrap();
    let instance = table.resolve(FontInstanceId::new(0)).unwrap();
    let shaped = shape_production_run_v3(instance, input("A", 1, 11)).unwrap();
    assert_eq!(shaped.glyph_run().font, instance.font_instance_id());
    assert_eq!(shaped.cluster_text(0), Some("A"));
    assert_eq!(shaped.input().language, Some("ja"));
    assert_eq!(shaped.input().script.bytes(), *b"Hani");
    assert_eq!(shaped.input().source, shaped.glyph_run().source_span);
    assert!(shaped
        .glyph_run()
        .glyphs
        .iter()
        .all(|g| g.original_gid.get() != 0));
    assert!(shaped.cff1_v2().is_none());
    assert!(matches!(
        freeze_production_cff1_fonts_v3(&[&shaped]),
        Err(Cff1PdfPlanErrorV2::InvalidInput)
    ));
    let mut malformed = input("A", 1, 11);
    malformed.source = ShapeSourceSpan::Parsed(
        TextSpan::new(
            TextBufferId::new(9),
            Utf8ByteOffset::new(31),
            Utf8ByteOffset::new(33),
        )
        .unwrap(),
    );
    assert!(matches!(
        shape_production_run_v3(instance, malformed),
        Err(ProductionShapeErrorV3::Backend(_))
    ));
    let context = "x".repeat(
        ledger
            .effective_limits()
            .base()
            .get()
            .max_shaping_context_bytes as usize,
    );
    let mut large = input("A", 1, 11);
    large.pre_context = Some(&context);
    assert!(matches!(
        shape_production_run_v3(instance, large),
        Err(ProductionShapeErrorV3::ContextLimit)
    ));
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn production_v3_original_shape_to_subset_preserves_table_session_and_sizes() {
    let bytes = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        sha256(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let ledger = ledger(Some(&bytes));
    let all = AdmittedProductionFontInstancesV3::from_used_faces(
        &ledger,
        [FontFaceId::new(1), FontFaceId::new(0)],
    )
    .unwrap();
    let selected =
        AdmittedProductionFontInstancesV3::from_used_faces(&ledger, [FontFaceId::new(1)]).unwrap();
    assert_ne!(all.fingerprint(), selected.fingerprint());
    let instance = all.resolve(FontInstanceId::new(1)).unwrap();
    let body = shape_production_run_v3(instance, input("一\u{e0100}", 1, 11)).unwrap();
    let heading = shape_production_run_v3(instance, input("一", 2, 22)).unwrap();
    assert_eq!(body.cluster_text(0), Some("一\u{e0100}"));
    assert_eq!(
        body.glyph_run().glyphs[0].original_gid,
        heading.glyph_run().glyphs[0].original_gid
    );
    assert_eq!(
        heading.glyph_run().glyphs[0].advance_x.raw(),
        2 * body.glyph_run().glyphs[0].advance_x.raw()
    );
    let plans = freeze_production_cff1_fonts_v3(&[&body, &heading]).unwrap();
    assert!(std::ptr::eq(plans.ledger(), &ledger));
    assert_eq!(plans.instance_table_fingerprint(), all.fingerprint());
    assert_eq!(plans.fonts().len(), 1);
    let plan = &plans.fonts()[0];
    assert_eq!(plan.font_face_id(), FontFaceId::new(1));
    assert_eq!(plan.font_instance_id(), FontInstanceId::new(1));
    assert_eq!(plan.bindings().len(), 1);
    assert!(plan.clusters().iter().all(|c| c.requires_actual_text()));
    assert_eq!(plan.clusters()[0].font_size(), body.font_size());
    assert_eq!(plan.clusters()[1].font_size(), heading.font_size());
    let other_table = shape_production_run_v3(
        selected.resolve(FontInstanceId::new(0)).unwrap(),
        input("一", 3, 11),
    )
    .unwrap();
    assert!(matches!(
        freeze_production_cff1_fonts_v3(&[&body, &other_table]),
        Err(Cff1PdfPlanErrorV2::IdentityMismatch)
    ));
    let other_ledger = self::ledger(Some(&bytes));
    assert_eq!(ledger.fingerprint(), other_ledger.fingerprint());
    let other = AdmittedProductionFontInstancesV3::from_used_faces(
        &other_ledger,
        [FontFaceId::new(0), FontFaceId::new(1)],
    )
    .unwrap();
    assert_eq!(all.fingerprint(), other.fingerprint());
    let other_run = shape_production_run_v3(
        other.resolve(FontInstanceId::new(1)).unwrap(),
        input("一", 3, 11),
    )
    .unwrap();
    assert!(matches!(
        freeze_production_cff1_fonts_v3(&[&body, &other_run]),
        Err(Cff1PdfPlanErrorV2::IdentityMismatch)
    ));
}
