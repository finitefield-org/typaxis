use super::*;
#[derive(Default)]
struct ReadTrace {
    requests: Vec<HostReadRequest>,
    stop: Option<usize>,
}
impl HostReadBudget for ReadTrace {
    type Error = usize;
    fn charge(&mut self, request: HostReadRequest) -> Result<(), usize> {
        let index = self.requests.len();
        if self.stop == Some(index) {
            return Err(index);
        }
        self.requests.push(request);
        Ok(())
    }
}

#[test]
fn resource_reads_keep_authentic_subjects_and_paid_prefixes() {
    let tree = TempTree::new("observed-resource-reads");
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
    for image in [false, true] {
        let resolver = || {
            StagingProductionResourceResolverV3::new(&declarations, &limits, [67; 32], host.roots())
                .unwrap()
        };
        let mut full = ReadTrace::default();
        let mut original = resolver();
        let pending = if image {
            original.read_image_with_budget(
                host.open_image(ImageResourceId::new(0)).unwrap(),
                &mut full,
            )
        } else {
            original.read_font_with_budget(host.open_font(FontFaceId::new(0)).unwrap(), &mut full)
        }
        .unwrap();
        assert_eq!(pending.bytes(), if image { PNG } else { BODY });
        assert_eq!(pending.content_hash(), sha256(pending.bytes()));
        let mut ordinary = resolver();
        let native = if image {
            ordinary.read_image(host.open_image(ImageResourceId::new(0)).unwrap())
        } else {
            ordinary.read_font(host.open_font(FontFaceId::new(0)).unwrap())
        }
        .unwrap();
        assert_eq!(pending.bytes(), native.bytes());
        assert_eq!(pending.content_hash(), native.content_hash());
        for stop in 0..full.requests.len() {
            let mut current = resolver();
            let mut short = ReadTrace {
                stop: Some(stop),
                ..ReadTrace::default()
            };
            let result = if image {
                current.read_image_with_budget(
                    host.open_image(ImageResourceId::new(0)).unwrap(),
                    &mut short,
                )
            } else {
                current
                    .read_font_with_budget(host.open_font(FontFaceId::new(0)).unwrap(), &mut short)
            };
            let ResourceReadError::Budget(failure) = result.unwrap_err() else {
                panic!("wrong error")
            };
            assert!(
                matches!(failure.subject, ResourceErrorSubject::Image(id) if image && id.get()==0)
                    || matches!(failure.subject, ResourceErrorSubject::FontFace(id) if !image && id.get()==0)
            );
            assert_eq!(failure.failure.error, stop);
            assert_eq!(failure.failure.request, full.requests[stop]);
            assert_eq!(short.requests, full.requests[..stop]);
            // The same attempted logical resource cannot be read twice in one resolver.
            let result = if image {
                current.read_image_with_budget(
                    host.open_image(ImageResourceId::new(0)).unwrap(),
                    &mut short,
                )
            } else {
                current
                    .read_font_with_budget(host.open_font(FontFaceId::new(0)).unwrap(), &mut short)
            };
            assert!(matches!(
                result,
                Err(ResourceReadError::Native(
                    ResourceAdmissionError::ConflictingLogicalResource
                ))
            ));
            assert_eq!(short.requests, full.requests[..stop]);
            // A fresh resolver retains the caller's spent history on retry.
            short.stop = None;
            let mut retry = resolver();
            if image {
                retry.read_image_with_budget(
                    host.open_image(ImageResourceId::new(0)).unwrap(),
                    &mut short,
                )
            } else {
                retry.read_font_with_budget(host.open_font(FontFaceId::new(0)).unwrap(), &mut short)
            }
            .unwrap();
            assert_eq!(&short.requests[stop..], full.requests);
        }
    }
    let mut counted =
        StagingProductionResourceResolverV3::new(&declarations, &limits, [67; 32], host.roots())
            .unwrap();
    let mut plain =
        StagingProductionResourceResolverV3::new(&declarations, &limits, [67; 32], host.roots())
            .unwrap();
    let mut trace = ReadTrace::default();
    let p = counted
        .read_font_with_budget(host.open_font(FontFaceId::new(0)).unwrap(), &mut trace)
        .unwrap();
    counted.parse_and_bind_font(p).unwrap();
    let p = counted
        .read_image_with_budget(
            host.open_image(ImageResourceId::new(0)).unwrap(),
            &mut trace,
        )
        .unwrap();
    counted.parse_and_bind_image(p).unwrap();
    let p = plain
        .read_font(host.open_font(FontFaceId::new(0)).unwrap())
        .unwrap();
    plain.parse_and_bind_font(p).unwrap();
    let p = plain
        .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
        .unwrap();
    plain.parse_and_bind_image(p).unwrap();
    assert_eq!(
        counted.finish().unwrap().fingerprint(),
        plain.finish().unwrap().fingerprint()
    );
}

#[test]
fn resource_read_native_identity_and_hash_failures_keep_their_order() {
    let tree = TempTree::new("observed-resource-native");
    write_base(&tree);
    let declarations = declared(None);
    let limits = effective();
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let context = host_context(tree.path(), &[]);
    let a = HostResourceAdmissionSession::new(&context, &config, declarations.resource_catalog())
        .unwrap();
    let b = HostResourceAdmissionSession::new(&context, &config, declarations.resource_catalog())
        .unwrap();
    let mut resolver =
        StagingProductionResourceResolverV3::new(&declarations, &limits, [69; 32], a.roots())
            .unwrap();
    let mut trace = ReadTrace::default();
    assert!(matches!(
        resolver.read_font_with_budget(b.open_font(FontFaceId::new(0)).unwrap(), &mut trace),
        Err(ResourceReadError::Native(
            ResourceAdmissionError::RootSetMismatch
        ))
    ));
    assert!(trace.requests.is_empty());
    assert!(matches!(
        resolver.read_font_with_budget(a.open_image(ImageResourceId::new(0)).unwrap(), &mut trace),
        Err(ResourceReadError::Native(
            ResourceAdmissionError::ReceiptKindMismatch
        ))
    ));
    assert!(trace.requests.is_empty());
    fs::write(tree.path().join("body.bin"), b"different stable font bytes").unwrap();
    let pending = resolver
        .read_font_with_budget(a.open_font(FontFaceId::new(0)).unwrap(), &mut trace)
        .unwrap();
    assert!(matches!(
        resolver.parse_and_bind_font(pending),
        Err(ProductionResourceErrorV3::Resource(
            ResourceAdmissionError::ExpectedHashMismatch
        ))
    ));
    assert!(trace
        .requests
        .iter()
        .any(|r| r.stage == HostReadStage::DigestFinish));
}

#[test]
fn resource_read_aliases_pay_each_read_and_native_byte_limit_precedes_hooks() {
    let tree = TempTree::new("observed-resource-aliases");
    write_base(&tree);
    let declarations = staging_declared_base_catalog(&StagingM4ResourceCatalog {
        font_faces: (0..2)
            .map(|id| StagingM4FontFaceDeclaration {
                font_face_id: FontFaceId::new(id),
                family: format!("Body-{id}"),
                uri: PortablePath::new("body.bin").unwrap(),
                face_index: 0,
                expected_sha256: Some(sha256(BODY)),
                media: FontMediaDeclaration::Declared(FontMediaType::SfntTrueTypeGlyf),
            })
            .collect(),
        images: vec![],
    })
    .unwrap();
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let host = HostResourceAdmissionSession::new(
        &host_context(tree.path(), &[]),
        &config,
        declarations.resource_catalog(),
    )
    .unwrap();
    let limits = effective();
    let mut resolver =
        StagingProductionResourceResolverV3::new(&declarations, &limits, [71; 32], host.roots())
            .unwrap();
    let mut trace = ReadTrace::default();
    for id in 0..2 {
        let pending = resolver
            .read_font_with_budget(host.open_font(FontFaceId::new(id)).unwrap(), &mut trace)
            .unwrap();
        resolver.parse_and_bind_font(pending).unwrap();
    }
    assert_eq!(resolver.finish().unwrap().fonts().len(), 2);
    assert_eq!(
        trace
            .requests
            .iter()
            .filter(|r| r.stage == HostReadStage::Buffer)
            .count(),
        2
    );
    assert_eq!(
        trace.requests.iter().map(|r| r.buffer_bytes).sum::<u64>(),
        2 * BODY.len() as u64
    );
    let base = ValidatedResourceLimits::new(ResourceLimits {
        max_font_bytes: BODY.len() as u64 - 1,
        ..ResourceLimits::default()
    })
    .unwrap();
    let limited = M4EffectiveResourceLimits::defaults_for(&base);
    let mut resolver =
        StagingProductionResourceResolverV3::new(&declarations, &limited, [71; 32], host.roots())
            .unwrap();
    let mut no_calls = ReadTrace::default();
    assert!(matches!(
        resolver.read_font_with_budget(host.open_font(FontFaceId::new(0)).unwrap(), &mut no_calls),
        Err(ResourceReadError::Native(
            ResourceAdmissionError::ResourceLimit
        ))
    ));
    assert!(no_calls.requests.is_empty());
}

#[derive(Default)]
struct AttemptReadTrace(ReadTrace);
impl HostReadBudget for AttemptReadTrace {
    type Error = usize;
    const OBSERVE_ATTEMPTS: bool = true;
    fn charge(&mut self, request: HostReadRequest) -> Result<(), usize> {
        self.0.charge(request)
    }
}

fn read_attempt<'roots>(
    r: &mut AdmittedResourceResolver<'roots>,
    host: &'roots HostResourceAdmissionSession,
    image: bool,
    trace: &mut AttemptReadTrace,
) -> Result<PendingResourceBytes, ResourceReadError<usize>> {
    if image {
        r.read_image_with_budget(host.open_image(ImageResourceId::new(0)).unwrap(), trace)
    } else {
        r.read_font_with_budget(host.open_font(FontFaceId::new(0)).unwrap(), trace)
    }
}

#[test]
fn authentic_attempt_permits_preserve_bytes_subjects_and_retry_state() {
    let tree = TempTree::new("observed-resource-attempts");
    write_base(&tree);
    let declarations = declared(None);
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let host = HostResourceAdmissionSession::new(
        &host_context(tree.path(), &[]),
        &config,
        declarations.resource_catalog(),
    )
    .unwrap();
    let limits = effective();
    for image in [false, true] {
        let resolver = || {
            AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
                &declarations,
                &limits,
                [73; 32],
                host.roots(),
            )
            .unwrap()
        };
        let mut full = AttemptReadTrace::default();
        let mut positive = resolver();
        let pending = read_attempt(&mut positive, &host, image, &mut full).unwrap();
        assert_eq!(pending.bytes(), if image { PNG } else { BODY });
        assert_eq!(pending.content_hash(), sha256(pending.bytes()));
        assert_eq!(
            full.0.requests.iter().map(|r| r.stage).collect::<Vec<_>>(),
            [
                HostReadStage::AttemptStorage,
                HostReadStage::AttemptLookup,
                HostReadStage::AttemptMark,
                HostReadStage::Buffer,
                HostReadStage::Chunk,
                HostReadStage::DigestFinish,
            ]
        );
        for stop in 0..full.0.requests.len() {
            let mut current = resolver();
            let mut short = AttemptReadTrace(ReadTrace {
                stop: Some(stop),
                ..ReadTrace::default()
            });
            let ResourceReadError::Budget(failure) =
                read_attempt(&mut current, &host, image, &mut short).unwrap_err()
            else {
                panic!("wrong error")
            };
            assert_eq!(failure.failure.request, full.0.requests[stop]);
            assert_eq!(short.0.requests, full.0.requests[..stop]);
            assert!(
                matches!(failure.subject, ResourceErrorSubject::Image(id) if image && id.get()==0)
                    || matches!(failure.subject, ResourceErrorSubject::FontFace(id) if !image && id.get()==0)
            );
            assert_eq!(
                current.budget.reserved_bytes,
                if stop < 3 {
                    0
                } else {
                    pending.bytes().len() as u64
                }
            );
            short.0.stop = None;
            if stop < 3 {
                let retried = read_attempt(&mut current, &host, image, &mut short).unwrap();
                assert_eq!(retried.bytes(), pending.bytes());
                assert_eq!(current.budget.reserved_bytes, pending.bytes().len() as u64);
            } else {
                let before = short.0.requests.len();
                assert!(matches!(
                    read_attempt(&mut current, &host, image, &mut short),
                    Err(ResourceReadError::Native(
                        ResourceAdmissionError::ConflictingLogicalResource
                    ))
                ));
                assert_eq!(short.0.requests.len(), before + 1);
                assert_eq!(short.0.requests[before].stage, HostReadStage::AttemptLookup);
            }
        }
    }
    let mut observed =
        StagingProductionResourceResolverV3::new(&declarations, &limits, [73; 32], host.roots())
            .unwrap();
    let mut ordinary =
        StagingProductionResourceResolverV3::new(&declarations, &limits, [73; 32], host.roots())
            .unwrap();
    let mut trace = AttemptReadTrace::default();
    let pending = observed
        .read_font_with_budget(host.open_font(FontFaceId::new(0)).unwrap(), &mut trace)
        .unwrap();
    observed.parse_and_bind_font(pending).unwrap();
    let pending = observed
        .read_image_with_budget(
            host.open_image(ImageResourceId::new(0)).unwrap(),
            &mut trace,
        )
        .unwrap();
    observed.parse_and_bind_image(pending).unwrap();
    let pending = ordinary
        .read_font(host.open_font(FontFaceId::new(0)).unwrap())
        .unwrap();
    ordinary.parse_and_bind_font(pending).unwrap();
    let pending = ordinary
        .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
        .unwrap();
    ordinary.parse_and_bind_image(pending).unwrap();
    assert_eq!(
        observed.finish().unwrap().canonical_jcs(),
        ordinary.finish().unwrap().canonical_jcs()
    );
}

#[derive(Default)]
struct DeclarationReadTrace(ReadTrace);
impl HostReadBudget for DeclarationReadTrace {
    type Error = usize;
    const OBSERVE_ATTEMPTS: bool = true;
    const OBSERVE_DECLARATIONS: bool = true;
    fn charge(&mut self, request: HostReadRequest) -> Result<(), usize> {
        self.0.charge(request)
    }
}
fn read_declaration<'roots>(
    r: &mut AdmittedResourceResolver<'roots>,
    host: &'roots HostResourceAdmissionSession,
    image: bool,
    trace: &mut DeclarationReadTrace,
) -> Result<PendingResourceBytes, ResourceReadError<usize>> {
    let source = if image {
        host.open_image(ImageResourceId::new(0)).unwrap()
    } else {
        host.open_font(FontFaceId::new(0)).unwrap()
    };
    match source.declaration {
        PendingResourceDeclaration::Font(value) => {
            assert!(std::ptr::eq(value, &host.declarations.font_faces[0]))
        }
        PendingResourceDeclaration::Image(value) => {
            assert!(std::ptr::eq(value, &host.declarations.images[0]))
        }
    }
    if image {
        r.read_image_with_budget(source, trace)
    } else {
        r.read_font_with_budget(source, trace)
    }
}
#[test]
fn authentic_borrowed_declaration_reads_keep_permit_prefixes_and_ownership() {
    let tree = TempTree::new("observed-declaration-reads");
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
    let make = || {
        AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &declarations,
            &limits,
            [79; 32],
            host.roots(),
        )
        .unwrap()
    };
    for image in [false, true] {
        let mut full = DeclarationReadTrace::default();
        let mut resolver = make();
        let pending = read_declaration(&mut resolver, &host, image, &mut full).unwrap();
        assert_eq!(pending.bytes(), if image { PNG } else { BODY });
        let original_uri = if image {
            &host.declarations.images[0].uri
        } else {
            &host.declarations.font_faces[0].uri
        };
        assert_eq!(pending.uri(), original_uri);
        assert_ne!(
            pending.uri().as_str().as_ptr(),
            original_uri.as_str().as_ptr()
        );
        assert_eq!(
            full.0.requests.iter().map(|r| r.stage).collect::<Vec<_>>(),
            [
                HostReadStage::DeclarationCheck,
                HostReadStage::AttemptStorage,
                HostReadStage::AttemptLookup,
                HostReadStage::AttemptMark,
                HostReadStage::Buffer,
                HostReadStage::Chunk,
                HostReadStage::DigestFinish,
                HostReadStage::DeclarationUri
            ]
        );
        for stop in 0..full.0.requests.len() {
            let mut resolver = make();
            let mut short = DeclarationReadTrace(ReadTrace {
                stop: Some(stop),
                ..ReadTrace::default()
            });
            let ResourceReadError::Budget(failure) =
                read_declaration(&mut resolver, &host, image, &mut short).unwrap_err()
            else {
                panic!("wrong refusal")
            };
            assert_eq!(failure.failure.request, full.0.requests[stop]);
            assert_eq!(short.0.requests, full.0.requests[..stop]);
            assert!(
                matches!(failure.subject, ResourceErrorSubject::Image(id) if image && id.get()==0)
                    || matches!(failure.subject, ResourceErrorSubject::FontFace(id) if !image && id.get()==0)
            );
            assert_eq!(
                resolver.budget.reserved_bytes,
                if stop < 4 {
                    0
                } else {
                    pending.bytes().len() as u64
                }
            );
            short.0.stop = None;
            if stop < 4 {
                assert_eq!(
                    read_declaration(&mut resolver, &host, image, &mut short)
                        .unwrap()
                        .bytes(),
                    pending.bytes()
                );
            } else {
                assert!(matches!(
                    read_declaration(&mut resolver, &host, image, &mut short),
                    Err(ResourceReadError::Native(
                        ResourceAdmissionError::ConflictingLogicalResource
                    ))
                ));
            }
        }
    }
    // Pending bytes own their URI and may outlive the host that supplied it.
    let pending = {
        let host = HostResourceAdmissionSession::new(
            &host_context(tree.path(), &[]),
            &config,
            declarations.resource_catalog(),
        )
        .unwrap();
        let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &declarations,
            &limits,
            [79; 32],
            host.roots(),
        )
        .unwrap();
        read_declaration(
            &mut resolver,
            &host,
            false,
            &mut DeclarationReadTrace::default(),
        )
        .unwrap()
    };
    assert_eq!(
        pending.uri(),
        &declarations.resource_catalog().font_faces[0].uri
    );
    assert_eq!(pending.bytes(), BODY);
}
#[test]
fn borrowed_declaration_identity_mismatch_precedes_attempt_or_read() {
    let tree = TempTree::new("borrowed-declaration-identity");
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
    let mut altered = declarations.resource_catalog().clone();
    altered.font_faces[0].family.push_str(" mismatch");
    let mut resolver =
        AdmittedResourceResolver::new_with_roots(&altered, limits.base(), host.roots()).unwrap();
    let mut trace = DeclarationReadTrace::default();
    assert!(matches!(
        resolver.read_font_with_budget(host.open_font(FontFaceId::new(0)).unwrap(), &mut trace),
        Err(ResourceReadError::Native(
            ResourceAdmissionError::ReceiptIdentityMismatch
        ))
    ));
    assert_eq!(trace.0.requests.len(), 1);
    assert_eq!(trace.0.requests[0].stage, HostReadStage::DeclarationCheck);
    assert_eq!(resolver.budget.reserved_bytes, 0);
}

#[derive(Default)]
struct OpenTrace(ReadTrace);
impl HostReadBudget for OpenTrace {
    type Error = usize;
    const OBSERVE_OPEN: bool = true;
    fn charge(&mut self, r: HostReadRequest) -> Result<(), usize> {
        self.0.charge(r)
    }
}
#[test]
fn resource_open_budget_retains_authentic_font_image_subjects() {
    let tree = TempTree::new("resource-open-budget");
    write_base(&tree);
    let declarations = declared(None);
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let make = || {
        HostResourceAdmissionSession::new(
            &host_context(tree.path(), &[]),
            &config,
            declarations.resource_catalog(),
        )
        .unwrap()
    };
    let mut cuts = 0;
    for image in [false, true] {
        let host = make();
        let mut full = OpenTrace::default();
        let source = if image {
            host.open_image_with_budget(ImageResourceId::new(0), &mut full)
        } else {
            host.open_font_with_budget(FontFaceId::new(0), &mut full)
        }
        .unwrap();
        let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &declarations,
            &effective(),
            [181; 32],
            host.roots(),
        )
        .unwrap();
        let pending = if image {
            resolver.read_image(source)
        } else {
            resolver.read_font(source)
        }
        .unwrap();
        assert_eq!(pending.bytes(), if image { PNG } else { BODY });
        for stop in 0..full.0.requests.len() {
            let host = make();
            let token = host.read_ledger_token().unwrap();
            let mut short = OpenTrace(ReadTrace {
                stop: Some(stop),
                ..ReadTrace::default()
            });
            let result = if image {
                host.open_image_with_budget(ImageResourceId::new(0), &mut short)
            } else {
                host.open_font_with_budget(FontFaceId::new(0), &mut short)
            };
            let failure = match result {
                Err(ResourceReadError::Budget(f)) => f,
                _ => panic!("wrong open result"),
            };
            assert_eq!(failure.failure.request, full.0.requests[stop]);
            assert_eq!(short.0.requests, full.0.requests[..stop]);
            assert!(
                matches!(failure.subject,ResourceErrorSubject::Image(id) if image && id.get()==0)
                    || matches!(failure.subject,ResourceErrorSubject::FontFace(id) if !image && id.get()==0)
            );
            token.revalidate().unwrap();
            short.0.stop = None;
            if image {
                host.open_image_with_budget(ImageResourceId::new(0), &mut short)
                    .unwrap();
            } else {
                host.open_font_with_budget(FontFaceId::new(0), &mut short)
                    .unwrap();
            }
            assert_eq!(&short.0.requests[stop..], full.0.requests);
            cuts += 1;
        }
    }
    assert_eq!(cuts, 10);
    println!("resource observed opens subjects=font,image callback refusals=10");
}
