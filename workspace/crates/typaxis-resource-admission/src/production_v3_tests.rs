use super::*;
use typaxis_document::{StagingM4FontFaceDeclaration, StagingM4ImageDeclaration};
const BODY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"
));
const PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/cover.bin"
));
fn declared(harano: Option<&[u8]>) -> StagingDeclaredBaseCatalog {
    let mut fonts = vec![StagingM4FontFaceDeclaration {
        font_face_id: FontFaceId::new(0),
        family: "Body".into(),
        uri: PortablePath::new("body.bin").unwrap(),
        face_index: 0,
        expected_sha256: Some(sha256(BODY)),
        media: FontMediaDeclaration::Declared(FontMediaType::SfntTrueTypeGlyf),
    }];
    if let Some(bytes) = harano {
        for id in 1..3 {
            fonts.push(StagingM4FontFaceDeclaration {
                font_face_id: FontFaceId::new(id),
                family: format!("Harano-{id}"),
                uri: PortablePath::new(format!("harano-{id}.bin")).unwrap(),
                face_index: 0,
                expected_sha256: Some(sha256(bytes)),
                media: FontMediaDeclaration::Declared(FontMediaType::SfntCff1),
            });
        }
    }
    staging_declared_base_catalog(&StagingM4ResourceCatalog {
        font_faces: fonts,
        images: vec![StagingM4ImageDeclaration {
            image_id: ImageResourceId::new(0),
            uri: PortablePath::new("cover.bin").unwrap(),
            expected_sha256: Some(sha256(PNG)),
            media: ImageMediaDeclaration::Declared(ImageMediaType::Png),
            vector_provenance: None,
        }],
    })
    .unwrap()
}
fn effective() -> M4EffectiveResourceLimits {
    M4EffectiveResourceLimits::new(
        limits(ResourceLimits::default()),
        M4ResourceLimits::default(),
    )
    .unwrap()
}
fn write_base(tree: &TempTree) {
    fs::write(tree.path().join("body.bin"), BODY).unwrap();
    fs::write(tree.path().join("cover.bin"), PNG).unwrap();
}
#[test]
fn production_v3_uses_real_host_session_and_keeps_legacy_media_separate() {
    let tree = TempTree::new("production-v3-host");
    write_base(&tree);
    let declarations = declared(None);
    let limits = effective();
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let host = HostResourceAdmissionSession::new(
        &host_context(tree.path(), &[]),
        &config,
        declarations.resource_catalog(),
    )
    .unwrap();
    let profile = sha256(b"private-staging-production-book-2-test");
    let mut resolver =
        StagingProductionResourceResolverV3::new(&declarations, &limits, profile, host.roots())
            .unwrap();
    let mut other =
        StagingProductionResourceResolverV3::new(&declarations, &limits, profile, host.roots())
            .unwrap();
    let pending = resolver
        .read_font(host.open_font(FontFaceId::new(0)).unwrap())
        .unwrap();
    assert!(matches!(
        other.parse_and_bind_font(pending.clone()),
        Err(ProductionResourceErrorV3::Resource(
            ResourceAdmissionError::ReceiptSessionMismatch
        ))
    ));
    resolver.parse_and_bind_font(pending.clone()).unwrap();
    assert!(matches!(
        resolver.parse_and_bind_font(pending),
        Err(ProductionResourceErrorV3::Resource(
            ResourceAdmissionError::ConflictingLogicalResource
        ))
    ));
    let pending = resolver
        .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
        .unwrap();
    resolver.parse_and_bind_image(pending).unwrap();
    let ledger = resolver.finish().unwrap();
    assert_eq!(
        ledger.resource_set_id(),
        "typaxis.production-book-resource-set/3"
    );
    assert_eq!(ledger.fonts().len(), 1);
    assert_eq!(ledger.images().len(), 1);
    assert!(matches!(
        ledger.font(FontFaceId::new(0)),
        Some(AdmittedProductionFontV3::TrueType(_))
    ));
    assert_eq!(ledger.font(FontFaceId::new(0)).unwrap().bytes(), BODY);
    assert_eq!(ledger.image(ImageResourceId::new(0)).unwrap().bytes(), PNG);
    assert_eq!(ledger.profile_fingerprint(), profile);
    assert_eq!(
        ledger.fingerprint(),
        sha256(ledger.canonical_jcs().as_bytes())
    );
    assert!(matches!(
        other.finish(),
        Err(ProductionResourceErrorV3::Resource(
            ResourceAdmissionError::MissingLogicalResource
        ))
    ));
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn production_v3_original_cff_aliases_join_true_type_and_png_without_v1_receipts() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        sha256(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let tree = TempTree::new("production-v3-original");
    write_base(&tree);
    for id in 1..3 {
        fs::write(tree.path().join(format!("harano-{id}.bin")), &bytes).unwrap();
    }
    let declarations = declared(Some(&bytes));
    let limits = effective();
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let host = HostResourceAdmissionSession::new(
        &host_context(tree.path(), &[]),
        &config,
        declarations.resource_catalog(),
    )
    .unwrap();
    let profile = sha256(b"private-staging-production-book-2-test");
    let build = |order: [u32; 3]| {
        let mut resolver =
            StagingProductionResourceResolverV3::new(&declarations, &limits, profile, host.roots())
                .unwrap();
        for id in order {
            let pending = resolver
                .read_font(host.open_font(FontFaceId::new(id)).unwrap())
                .unwrap();
            resolver.parse_and_bind_font(pending).unwrap();
        }
        let pending = resolver
            .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_image(pending).unwrap();
        resolver.finish().unwrap()
    };
    let ledger = build([2, 0, 1]);
    let instances = AdmittedProductionFontInstancesV3::from_used_faces(
        &ledger,
        [
            FontFaceId::new(2),
            FontFaceId::new(0),
            FontFaceId::new(1),
            FontFaceId::new(2),
        ],
    )
    .unwrap();
    assert_eq!(instances.len(), 3);
    for id in 0..3 {
        let instance = instances
            .resolve(typaxis_core::FontInstanceId::new(id))
            .unwrap();
        assert_eq!(instance.font_instance_id().get(), id);
        assert_eq!(instance.font().font_face_id().get(), id);
        assert!(std::ptr::eq(instance.ledger(), &ledger));
    }
    assert!(instances
        .resolve(typaxis_core::FontInstanceId::new(3))
        .is_none());
    assert!(
        AdmittedProductionFontInstancesV3::from_used_faces(&ledger, [FontFaceId::new(3)]).is_err()
    );
    let reverse = build([1, 2, 0]);
    assert_eq!(ledger.fingerprint(), reverse.fingerprint());
    assert!(!ledger.same_session_as(&reverse));
    assert_eq!(
        ledger
            .fonts()
            .iter()
            .map(|f| f.font_face_id().get())
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    for id in 1..3 {
        let AdmittedProductionFontV3::Cff1V2(font) = ledger.font(FontFaceId::new(id)).unwrap()
        else {
            panic!("CFF /2 lost its variant");
        };
        assert_eq!(font.admission().source(), bytes);
        assert_eq!(font.admission().glyph_count(), 23060);
        assert_eq!(font.declaration().font_face_id, FontFaceId::new(id));
    }
    let AdmittedProductionFontV3::Cff1V2(a) = &ledger.fonts()[1] else {
        panic!()
    };
    let AdmittedProductionFontV3::Cff1V2(b) = &ledger.fonts()[2] else {
        panic!()
    };
    assert_eq!(a.admission().fingerprint(), b.admission().fingerprint());
    // The old resolver still enforces CFF /1 and cannot admit this font.
    let mut old = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
        &declarations,
        &limits,
        profile,
        host.roots(),
    )
    .unwrap();
    let pending = old
        .read_font(host.open_font(FontFaceId::new(1)).unwrap())
        .unwrap();
    assert!(old.parse_and_bind_declared_sfnt(pending).is_err());
}

#[test]
fn production_v3_font_and_image_reads_share_one_aggregate_byte_limit() {
    let tree = TempTree::new("production-v3-budget");
    write_base(&tree);
    let declarations = declared(None);
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let host = HostResourceAdmissionSession::new(
        &host_context(tree.path(), &[]),
        &config,
        declarations.resource_catalog(),
    )
    .unwrap();
    let exact = (BODY.len() + PNG.len()) as u64;
    for ceiling in [exact, exact - 1] {
        let base = ResourceLimits {
            max_resource_bytes: ceiling,
            max_font_bytes: BODY.len() as u64,
            max_image_bytes: PNG.len() as u64,
            ..ResourceLimits::default()
        };
        let limits =
            M4EffectiveResourceLimits::new(limits(base), M4ResourceLimits::default()).unwrap();
        let mut resolver =
            StagingProductionResourceResolverV3::new(&declarations, &limits, [3; 32], host.roots())
                .unwrap();
        let pending = resolver
            .read_font(host.open_font(FontFaceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_font(pending).unwrap();
        let image = resolver.read_image(host.open_image(ImageResourceId::new(0)).unwrap());
        if ceiling == exact {
            resolver.parse_and_bind_image(image.unwrap()).unwrap();
            assert_eq!(resolver.finish().unwrap().images().len(), 1);
        } else {
            assert!(matches!(
                image,
                Err(ProductionResourceErrorV3::Resource(
                    ResourceAdmissionError::ResourceLimit
                ))
            ));
            assert!(matches!(
                resolver.read_image(host.open_image(ImageResourceId::new(0)).unwrap()),
                Err(ProductionResourceErrorV3::Resource(
                    ResourceAdmissionError::ResourceLimit
                ))
            ));
            assert!(resolver.finish().is_err());
        }
    }
}
