use super::*;
#[derive(Clone, Debug, Eq, PartialEq)]
enum LedgerRequest {
    Reserve(ProductionResourceLedgerStorageV3, usize, usize),
    Work(ProductionResourceLedgerPointV3, u64),
}
#[derive(Default)]
struct LedgerTrace {
    requests: Vec<LedgerRequest>,
    stop: Option<usize>,
}
impl LedgerTrace {
    fn request(&mut self, r: LedgerRequest) -> Result<(), usize> {
        let at = self.requests.len();
        if self.stop == Some(at) {
            return Err(at);
        }
        self.requests.push(r);
        Ok(())
    }
}
impl ProductionResourceLedgerBudgetV3 for LedgerTrace {
    type Error = usize;
    fn reserve(
        &mut self,
        s: ProductionResourceLedgerStorageV3,
        r: usize,
        b: usize,
    ) -> Result<(), usize> {
        self.request(LedgerRequest::Reserve(s, r, b))
    }
    fn work(&mut self, p: ProductionResourceLedgerPointV3, n: u64) -> Result<(), usize> {
        self.request(LedgerRequest::Work(p, n))
    }
}

#[test]
fn observed_resource_ledger_matches_native_and_refuses_every_request_without_partial_outputs() {
    for count in [0, 1, 16] {
        let tree = TempTree::new("observed-ledger");
        write_base(&tree);
        let declarations = staging_declared_base_catalog(&StagingM4ResourceCatalog {
            font_faces: (0..count)
                .map(|id| StagingM4FontFaceDeclaration {
                    font_face_id: FontFaceId::new(id),
                    family: format!("Body-{id}-\"和\""),
                    uri: PortablePath::new("body.bin").unwrap(),
                    face_index: 0,
                    expected_sha256: Some(sha256(BODY)),
                    media: FontMediaDeclaration::Declared(FontMediaType::SfntTrueTypeGlyf),
                })
                .collect(),
            images: vec![StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("cover.bin").unwrap(),
                expected_sha256: Some(sha256(PNG)),
                media: ImageMediaDeclaration::Declared(ImageMediaType::Png),
                vector_provenance: None,
            }],
        })
        .unwrap();
        let limits = effective();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let host = HostResourceAdmissionSession::new(
            &host_context(tree.path(), &[]),
            &config,
            declarations.resource_catalog(),
        )
        .unwrap();
        let build = || {
            let mut resolver = StagingProductionResourceResolverV3::new(
                &declarations,
                &limits,
                [79; 32],
                host.roots(),
            )
            .unwrap();
            // Reverse admission still materializes in declaration order.
            for id in (0..count).rev() {
                let p = resolver
                    .read_font(host.open_font(FontFaceId::new(id)).unwrap())
                    .unwrap();
                resolver.parse_and_bind_font(p).unwrap();
            }
            let p = resolver
                .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
                .unwrap();
            resolver.parse_and_bind_image(p).unwrap();
            resolver
        };
        let native = build().finish().unwrap();
        let mut full = LedgerTrace::default();
        let observed = build().finish_with_budget(&mut full).unwrap();
        assert_eq!(observed.canonical_jcs(), native.canonical_jcs());
        assert_eq!(observed.fingerprint(), native.fingerprint());
        assert_eq!(
            observed.fingerprint(),
            sha256(observed.canonical_jcs().as_bytes())
        );
        assert_eq!(
            observed
                .fonts()
                .iter()
                .map(|f| f.font_face_id().get())
                .collect::<Vec<_>>(),
            (0..count).collect::<Vec<_>>()
        );
        assert_eq!(observed.font_families(), native.font_families());
        assert!(full.requests.iter().any(|r|matches!(r,LedgerRequest::Work(p,_) if p.stage==ProductionResourceLedgerStageV3::CanonicalHash)));
        let mut refusals = 0;
        for stop in 0..full.requests.len() {
            let mut short = LedgerTrace {
                stop: Some(stop),
                ..LedgerTrace::default()
            };
            let ProductionResourceLedgerErrorV3::Budget(f) =
                build().finish_with_budget(&mut short).unwrap_err()
            else {
                panic!("wrong failure")
            };
            assert_eq!(f.error, stop);
            assert_eq!(short.requests, full.requests[..stop]);
            let expected = match full.requests[stop] {
                LedgerRequest::Work(p, _) => p,
                LedgerRequest::Reserve(s, _, _) => ProductionResourceLedgerPointV3 {
                    stage: match s {
                        ProductionResourceLedgerStorageV3::FamilyEntries => {
                            ProductionResourceLedgerStageV3::FamilyEntries
                        }
                        ProductionResourceLedgerStorageV3::FamilyNames => {
                            ProductionResourceLedgerStageV3::FamilyNameCopy
                        }
                        ProductionResourceLedgerStorageV3::Aliases => {
                            ProductionResourceLedgerStageV3::Aliases
                        }
                        ProductionResourceLedgerStorageV3::Fonts => {
                            ProductionResourceLedgerStageV3::Fonts
                        }
                        ProductionResourceLedgerStorageV3::Images => {
                            ProductionResourceLedgerStageV3::Images
                        }
                        ProductionResourceLedgerStorageV3::CanonicalJcs => {
                            ProductionResourceLedgerStageV3::CanonicalCapacity
                        }
                    },
                    font_face_id: if matches!(s, ProductionResourceLedgerStorageV3::FamilyNames) {
                        full.requests[..stop].iter().rev().find_map(|r| match r {
                            LedgerRequest::Work(p, _) => p.font_face_id,
                            _ => None,
                        })
                    } else {
                        None
                    },
                    image_id: None,
                },
            };
            assert_eq!(f.point, expected);
            short.stop = None;
            let retried = build().finish_with_budget(&mut short).unwrap();
            assert_eq!(&short.requests[stop..], full.requests);
            assert_eq!(retried.canonical_jcs(), native.canonical_jcs());
            refusals += 1;
        }
        eprintln!(
            "resource ledger count={count} canonical_bytes={} requests={} refusals={refusals}",
            observed.canonical_jcs().len(),
            full.requests.len()
        );
        eprintln!("RESOURCE_LEDGER_CANONICAL {}", observed.canonical_jcs());
    }
}

#[test]
fn observed_resource_ledger_preserves_missing_resource_before_budget_hooks() {
    let tree = TempTree::new("observed-ledger-missing");
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
    let resolver =
        StagingProductionResourceResolverV3::new(&declarations, &limits, [81; 32], host.roots())
            .unwrap();
    let mut trace = LedgerTrace {
        stop: Some(0),
        ..LedgerTrace::default()
    };
    assert!(matches!(
        resolver.finish_with_budget(&mut trace),
        Err(ProductionResourceLedgerErrorV3::Native(
            ProductionResourceErrorV3::Resource(ResourceAdmissionError::MissingLogicalResource)
        ))
    ));
    assert!(trace.requests.is_empty());
}

#[test]
fn observed_ledger_vector_aliases_use_real_host_resources_and_typed_refusals() {
    let svg=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/fraction-equality.svg"));
    let tree = TempTree::new("observed-ledger-vector-aliases");
    fs::write(tree.path().join("formula.svg"), svg).unwrap();
    let declarations = staging_declared_base_catalog(&StagingM4ResourceCatalog {
        font_faces: vec![],
        images: (0..9)
            .map(|id| StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(id),
                uri: PortablePath::new("formula.svg").unwrap(),
                expected_sha256: Some(sha256(svg)),
                media: ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2),
                vector_provenance: None,
            })
            .collect(),
    })
    .unwrap();
    let limits = effective();
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let host = HostResourceAdmissionSession::new(
        &host_context(tree.path(), &[]),
        &config,
        declarations.resource_catalog(),
    )
    .unwrap();
    let build = || {
        let mut resolver = StagingProductionResourceResolverV3::new(
            &declarations,
            &limits,
            [83; 32],
            host.roots(),
        )
        .unwrap();
        // Vector aggregate admission intentionally follows declaration order.
        for id in 0..9 {
            let p = resolver
                .read_image(host.open_image(ImageResourceId::new(id)).unwrap())
                .unwrap();
            resolver.parse_and_bind_image(p).unwrap();
        }
        resolver
    };
    let native = build().finish().unwrap();
    let mut trace = LedgerTrace::default();
    let observed = build().finish_with_budget(&mut trace).unwrap();
    assert_eq!(observed.canonical_jcs(), native.canonical_jcs());
    assert_eq!(observed.fingerprint(), native.fingerprint());
    let mut cuts = 0;
    for (stop, request) in trace.requests.iter().enumerate() {
        let point = match request {
            LedgerRequest::Reserve(ProductionResourceLedgerStorageV3::Aliases, _, _) => {
                ProductionResourceLedgerPointV3 {
                    stage: ProductionResourceLedgerStageV3::Aliases,
                    font_face_id: None,
                    image_id: None,
                }
            }
            LedgerRequest::Work(p, _)
                if matches!(
                    p.stage,
                    ProductionResourceLedgerStageV3::Aliases
                        | ProductionResourceLedgerStageV3::AliasOrder
                        | ProductionResourceLedgerStageV3::AliasCompare
                ) =>
            {
                *p
            }
            _ => continue,
        };
        let mut short = LedgerTrace {
            stop: Some(stop),
            ..LedgerTrace::default()
        };
        let ProductionResourceLedgerErrorV3::Budget(error) =
            build().finish_with_budget(&mut short).unwrap_err()
        else {
            panic!("wrong refusal")
        };
        assert_eq!(error.point, point);
        assert_eq!(error.error, stop);
        assert_eq!(short.requests, trace.requests[..stop]);
        cuts += 1;
    }
    assert!(cuts > 9);
    eprintln!("real-host SVG aliases=9 alias callback refusals={cuts}");
}

#[test]
fn observed_svg_source_real_host_preserves_ir_and_authentic_refusal_subjects() {
    #[derive(Default)]
    struct SvgTrace {
        requests: Vec<SafeSvgSourceRequest>,
        stop: Option<usize>,
    }
    impl SafeSvgSourceBudget for SvgTrace {
        type Error = usize;
        fn work(&mut self, r: SafeSvgSourceRequest) -> Result<(), usize> {
            let at = self.requests.len();
            if self.stop == Some(at) {
                return Err(at);
            }
            self.requests.push(r);
            Ok(())
        }
    }
    let svg=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/fraction-equality.svg"));
    let tree = TempTree::new("svg-source-host");
    fs::write(tree.path().join("formula.svg"), svg).unwrap();
    write_base(&tree);
    let declarations = staging_declared_base_catalog(&StagingM4ResourceCatalog {
        font_faces: vec![],
        images: (0..3)
            .map(|id| StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(id),
                uri: PortablePath::new(if id == 0 { "cover.bin" } else { "formula.svg" }).unwrap(),
                expected_sha256: Some(sha256(if id == 0 { PNG } else { svg })),
                media: ImageMediaDeclaration::Declared(if id == 0 {
                    ImageMediaType::Png
                } else {
                    ImageMediaType::SvgSafe2
                }),
                vector_provenance: None,
            })
            .collect(),
    })
    .unwrap();
    let caps = effective();
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let host = HostResourceAdmissionSession::new(
        &host_context(tree.path(), &[]),
        &config,
        declarations.resource_catalog(),
    )
    .unwrap();
    let build = |trace: &mut SvgTrace, observed: bool| {
        let mut resolver =
            StagingProductionResourceResolverV3::new(&declarations, &caps, [89; 32], host.roots())
                .unwrap();
        for id in 0..3 {
            let p = resolver
                .read_image(host.open_image(ImageResourceId::new(id)).unwrap())
                .unwrap();
            if observed {
                resolver
                    .parse_and_bind_image_with_source_budget(p, trace)
                    .unwrap();
            } else {
                resolver.parse_and_bind_image(p).unwrap();
            }
        }
        resolver.finish().unwrap()
    };
    let native = build(&mut SvgTrace::default(), false);
    let mut full = SvgTrace::default();
    let observed = build(&mut full, true);
    assert_eq!(native.canonical_jcs(), observed.canonical_jcs());
    assert_eq!(native.fingerprint(), observed.fingerprint());
    for id in 1..3 {
        assert_eq!(
            native
                .image(ImageResourceId::new(id))
                .unwrap()
                .admitted_safe_vector(),
            observed
                .image(ImageResourceId::new(id))
                .unwrap()
                .admitted_safe_vector()
        );
    }
    assert_eq!(full.requests.len(), 6);
    assert!(full
        .requests
        .iter()
        .all(|r| r.image_id.get() == 1 || r.image_id.get() == 2));
    for stop in 0..full.requests.len() {
        let mut trace = SvgTrace {
            stop: Some(stop),
            ..SvgTrace::default()
        };
        let mut resolver =
            StagingProductionResourceResolverV3::new(&declarations, &caps, [89; 32], host.roots())
                .unwrap();
        for id in 0..=full.requests[stop].image_id.get() {
            let p = resolver
                .read_image(host.open_image(ImageResourceId::new(id)).unwrap())
                .unwrap();
            if id < full.requests[stop].image_id.get() {
                resolver
                    .parse_and_bind_image_with_source_budget(p, &mut trace)
                    .unwrap();
                continue;
            }
            let retry = p.clone();
            let SafeSvgSourceError::Budget(f) = resolver
                .parse_and_bind_image_with_source_budget(p, &mut trace)
                .unwrap_err()
            else {
                panic!("wrong refusal")
            };
            assert_eq!(f.error, stop);
            assert_eq!(f.request, full.requests[stop]);
            assert_eq!(trace.requests, full.requests[..stop]);
            trace.stop = None;
            resolver
                .parse_and_bind_image_with_source_budget(retry, &mut trace)
                .unwrap();
            for remaining in id + 1..3 {
                let p = resolver
                    .read_image(host.open_image(ImageResourceId::new(remaining)).unwrap())
                    .unwrap();
                resolver
                    .parse_and_bind_image_with_source_budget(p, &mut trace)
                    .unwrap();
            }
            break;
        }
        assert_eq!(
            resolver.finish().unwrap().canonical_jcs(),
            native.canonical_jcs()
        );
    }
    eprintln!("real-host SVG source aliases=2 requests=6 refusals=6 PNG hooks=0");
}
