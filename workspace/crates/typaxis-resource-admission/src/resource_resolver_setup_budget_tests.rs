use super::*;

#[derive(Default)]
struct Trace {
    accepted: Vec<ResourceCatalogRequest>,
    stop: Option<usize>,
    ledger: Option<HostReadIdentityLedger>,
}
impl ResourceCatalogBudget for Trace {
    type Error = usize;
    fn charge(&mut self, r: ResourceCatalogRequest) -> Result<(), usize> {
        if let Some(ledger) = &self.ledger {
            ledger.token().unwrap();
        }
        let at = self.accepted.len();
        if self.stop == Some(at) {
            return Err(at);
        }
        self.accepted.push(r);
        Ok(())
    }
}
fn finish<'roots, C: std::borrow::Borrow<ResourceCatalog>>(
    mut resolver: StagingProductionResourceResolverV3<'roots>,
    host: &'roots HostResourceAdmissionSession<C>,
) -> AdmittedProductionResourceLedgerV3 {
    let font = resolver
        .read_font(host.open_font(FontFaceId::new(0)).unwrap())
        .unwrap();
    resolver.parse_and_bind_font(font).unwrap();
    let image = resolver
        .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
        .unwrap();
    resolver.parse_and_bind_image(image).unwrap();
    resolver.finish().unwrap()
}

#[test]
fn resolver_setup_matches_native_and_refuses_every_request_with_reentrant_ledger() {
    let tree = TempTree::new("observed-resolver-setup");
    write_base(&tree);
    let declarations = declared(None);
    let caps = effective();
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let context = host_context(tree.path(), &[]);
    let ledger = HostReadIdentityLedger::new();
    let host = HostResourceAdmissionSession::new_with_declared_catalog_and_read_ledger(
        &context,
        &config,
        &declarations,
        &ledger,
    )
    .unwrap();
    let token = ledger.token().unwrap();
    let native = finish(
        StagingProductionResourceResolverV3::new(&declarations, &caps, [81; 32], host.roots())
            .unwrap(),
        &host,
    );
    // Re-seal after the native control opened its files. Setup itself cannot
    // change either candidate or opened identities.
    assert!(token.revalidate().is_err());
    let token = ledger.token().unwrap();
    let mut full = Trace {
        ledger: Some(ledger.clone()),
        ..Trace::default()
    };
    let observed = StagingProductionResourceResolverV3::new_with_catalog_budget(
        &declarations,
        &caps,
        [81; 32],
        host.roots(),
        &mut full,
    )
    .unwrap();
    token.revalidate().unwrap();
    let observed = finish(observed, &host);
    assert_eq!(observed.canonical_jcs(), native.canonical_jcs());
    assert_eq!(observed.fingerprint(), native.fingerprint());
    for stop in 0..full.accepted.len() {
        let mut short = Trace {
            stop: Some(stop),
            ledger: Some(ledger.clone()),
            ..Trace::default()
        };
        let error = match StagingProductionResourceResolverV3::new_with_catalog_budget(
            &declarations,
            &caps,
            [81; 32],
            host.roots(),
            &mut short,
        ) {
            Err(e) => e,
            Ok(_) => panic!("refused setup returned a resolver"),
        };
        let ResourceCatalogError::Budget(f) = error else {
            panic!("wrong error")
        };
        assert_eq!(f.error, stop);
        assert_eq!(f.request, full.accepted[stop]);
        assert_eq!(short.accepted, full.accepted[..stop]);
        token.revalidate().unwrap();
        short.stop = None;
        let retried = StagingProductionResourceResolverV3::new_with_catalog_budget(
            &declarations,
            &caps,
            [81; 32],
            host.roots(),
            &mut short,
        )
        .unwrap();
        assert_eq!(&short.accepted[stop..], full.accepted);
        assert_eq!(
            finish(retried, &host).canonical_jcs(),
            native.canonical_jcs()
        );
    }
    println!(
        "resolver setup callback refusals={} reentrant_ledger=true",
        full.accepted.len()
    );
}

#[test]
fn resolver_setup_keeps_native_limits_and_fresh_receipt_identity() {
    let tree = TempTree::new("resolver-setup-native");
    write_base(&tree);
    let declarations = declared(None);
    let caps = effective();
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let context = host_context(tree.path(), &[]);
    let host = HostResourceAdmissionSession::new_borrowed(
        &context,
        &config,
        declarations.resource_catalog(),
    )
    .unwrap();
    let mut a = StagingProductionResourceResolverV3::new_with_catalog_budget(
        &declarations,
        &caps,
        [82; 32],
        host.roots(),
        &mut Trace::default(),
    )
    .unwrap();
    let mut b = StagingProductionResourceResolverV3::new_with_catalog_budget(
        &declarations,
        &caps,
        [82; 32],
        host.roots(),
        &mut Trace::default(),
    )
    .unwrap();
    let pending = a
        .read_font(host.open_font(FontFaceId::new(0)).unwrap())
        .unwrap();
    assert!(matches!(
        b.parse_and_bind_font(pending),
        Err(ProductionResourceErrorV3::Resource(
            ResourceAdmissionError::ReceiptSessionMismatch
        ))
    ));
    let too_many = declared(Some(BODY));
    let mut base = caps.base().get().clone();
    base.max_fonts = 1;
    let strict = M4EffectiveResourceLimits::defaults_for(&limits(base));
    assert!(matches!(
        StagingProductionResourceResolverV3::new(&too_many, &strict, [82; 32], host.roots()),
        Err(ProductionResourceErrorV3::Resource(
            ResourceAdmissionError::ResourceLimit
        ))
    ));
    let mut trace = Trace::default();
    assert!(matches!(
        StagingProductionResourceResolverV3::new_with_catalog_budget(
            &too_many,
            &strict,
            [82; 32],
            host.roots(),
            &mut trace
        ),
        Err(ResourceCatalogError::Native(
            ResourceAdmissionError::ResourceLimit
        ))
    ));
    assert_eq!(trace.accepted.len(), 1);
    assert_eq!(trace.accepted[0].stage, ResourceCatalogStage::ResolverStart);
    struct Ordinary;
    impl ResourceCatalogBudget for Ordinary {
        type Error = std::convert::Infallible;
        const OBSERVED: bool = false;
        fn charge(&mut self, _: ResourceCatalogRequest) -> Result<(), Self::Error> {
            panic!("ordinary setup must not call hooks")
        }
    }
    assert_eq!(
        finish(
            StagingProductionResourceResolverV3::new_with_catalog_budget(
                &declarations,
                &caps,
                [82; 32],
                host.roots(),
                &mut Ordinary
            )
            .unwrap(),
            &host
        )
        .canonical_jcs(),
        finish(
            StagingProductionResourceResolverV3::new(&declarations, &caps, [82; 32], host.roots())
                .unwrap(),
            &host
        )
        .canonical_jcs()
    );
}

#[test]
fn resolver_setup_initial_image_cursor_stops_at_the_first_vector() {
    let tree = TempTree::new("resolver-setup-cursor");
    write_base(&tree);
    let caps = effective();
    let mut raw = StagingM4ResourceCatalog {
        font_faces: vec![],
        images: (0..4)
            .map(|id| StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(id),
                uri: PortablePath::new("cover.bin").unwrap(),
                expected_sha256: Some(sha256(PNG)),
                media: ImageMediaDeclaration::Declared(if id == 2 {
                    ImageMediaType::SvgSafe2
                } else {
                    ImageMediaType::Png
                }),
                vector_provenance: None,
            })
            .collect(),
    };
    for first in [Some(0), Some(2), None] {
        for d in &mut raw.images {
            d.media = ImageMediaDeclaration::Declared(if Some(d.image_id.get()) == first {
                ImageMediaType::SvgSafe2
            } else {
                ImageMediaType::Png
            });
        }
        let declared = staging_declared_base_catalog(&raw).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let context = host_context(tree.path(), &[]);
        let ledger = HostReadIdentityLedger::new();
        let host = HostResourceAdmissionSession::new_with_declared_catalog_and_read_ledger(
            &context, &config, &declared, &ledger,
        )
        .unwrap();
        let mut trace = Trace::default();
        StagingProductionResourceResolverV3::new_with_catalog_budget(
            &declared,
            &caps,
            [83; 32],
            host.roots(),
            &mut trace,
        )
        .unwrap();
        assert_eq!(
            trace
                .accepted
                .iter()
                .filter(|r| r.stage == ResourceCatalogStage::ResolverImageCursor)
                .map(|r| r.image_id.unwrap().get())
                .collect::<Vec<_>>(),
            (0..first.map_or(4, |v| v + 1)).collect::<Vec<_>>()
        );
        assert_eq!(ledger.token().unwrap().stored_opened_identity_count(), 0);
    }
}
