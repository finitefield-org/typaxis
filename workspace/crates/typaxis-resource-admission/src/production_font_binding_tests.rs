use super::*;
use typaxis_font::{CffAdmissionBudgetV2, CffBaseCmapBudgetV2, CffCmapBudgetV2};
const CFF: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/vmb-book/fixture-fonts/typaxis-cff-fixture.otf"));
#[derive(Default)]
struct Trace {
    accepted: Vec<ResourceCatalogRequest>,
    stop: Option<usize>,
    observed: bool,
}
impl ResourceCatalogBudget for Trace {
    type Error = usize;
    fn charge(&mut self, request: ResourceCatalogRequest) -> Result<(), usize> {
        assert_eq!(request.image_id, None);
        assert!(request.font_face_id.is_some());
        if self.stop == Some(self.accepted.len()) {
            return Err(self.accepted.len());
        }
        self.accepted.push(request);
        Ok(())
    }
}
impl CffBaseCmapBudgetV2 for Trace {
    type Error = usize;
    const OBSERVED: bool = false;
    fn reserve(
        &mut self,
        _: typaxis_font::CffBaseCmapStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        unreachable!()
    }
    fn work(&mut self, _: u64) -> Result<(), usize> {
        unreachable!()
    }
}
impl CffCmapBudgetV2 for Trace {
    fn reserve_variation(
        &mut self,
        _: typaxis_font::CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        unreachable!()
    }
}
impl CffAdmissionBudgetV2 for Trace {
    fn reserve_sfnt(
        &mut self,
        _: typaxis_font::CffSfntStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        unreachable!()
    }
    fn work_sfnt(&mut self, _: u64) -> Result<(), usize> {
        unreachable!()
    }
}
fn bind(
    resolver: &mut StagingProductionResourceResolverV3<'_>,
    pending: PendingResourceBytes,
    trace: &mut Trace,
) {
    if trace.observed {
        resolver
            .parse_and_bind_font_with_binding_budget(pending, trace)
            .unwrap()
            .unwrap();
    } else {
        resolver.parse_and_bind_font(pending).unwrap();
    }
}
#[test]
fn production_cff_binding_every_cut_preserves_native_ledger_and_retry() {
    let tree = TempTree::new("cff-binding-cuts");
    write_base(&tree);
    for id in 1..3 {
        fs::write(tree.path().join(format!("harano-{id}.bin")), CFF).unwrap();
    }
    let declarations = declared(Some(CFF));
    let caps = effective();
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let context = host_context(tree.path(), &[]);
    let host = HostResourceAdmissionSession::new_borrowed(
        &context,
        &config,
        declarations.resource_catalog(),
    )
    .unwrap();
    let make = || {
        StagingProductionResourceResolverV3::new(&declarations, &caps, [185; 32], host.roots())
            .unwrap()
    };
    fn finish<'r, C: std::borrow::Borrow<ResourceCatalog>>(
        mut resolver: StagingProductionResourceResolverV3<'r>,
        host: &'r HostResourceAdmissionSession<C>,
    ) -> AdmittedProductionResourceLedgerV3 {
        let p = resolver
            .read_font(host.open_font(FontFaceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_font(p).unwrap();
        let p = resolver
            .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_image(p).unwrap();
        resolver.finish().unwrap()
    }
    let mut native = make();
    for id in [2, 1] {
        let p = native
            .read_font(host.open_font(FontFaceId::new(id)).unwrap())
            .unwrap();
        native.parse_and_bind_font(p).unwrap();
    }
    let native = finish(native, &host);
    let mut total_cuts = 0;
    // Empty table, migration of one native entry, and reuse of reserved storage.
    for first_observed in [false, true] {
        let mut full = make();
        let mut full_trace = Trace {
            observed: first_observed,
            ..Default::default()
        };
        let p = full
            .read_font(host.open_font(FontFaceId::new(2)).unwrap())
            .unwrap();
        bind(&mut full, p, &mut full_trace);
        full_trace.accepted.clear();
        full_trace.observed = true;
        let p = full
            .read_font(host.open_font(FontFaceId::new(1)).unwrap())
            .unwrap();
        bind(&mut full, p, &mut full_trace);
        assert_eq!(finish(full, &host).canonical_jcs(), native.canonical_jcs());
        let requests = full_trace.accepted;
        assert!(requests
            .iter()
            .all(|r| r.font_face_id == Some(FontFaceId::new(1))));
        for cut in 0..requests.len() {
            let mut resolver = make();
            let mut initial = Trace {
                observed: first_observed,
                ..Default::default()
            };
            let p = resolver
                .read_font(host.open_font(FontFaceId::new(2)).unwrap())
                .unwrap();
            bind(&mut resolver, p, &mut initial);
            let p = resolver
                .read_font(host.open_font(FontFaceId::new(1)).unwrap())
                .unwrap();
            let retry = p.clone();
            let mut trace = Trace {
                stop: Some(cut),
                ..Default::default()
            };
            let failure = resolver
                .parse_and_bind_font_with_binding_budget(p, &mut trace)
                .unwrap_err();
            assert_eq!(failure.font_face_id, FontFaceId::new(1));
            let ProductionFontBindingCauseV3::Binding(f) = failure.failure else {
                panic!("wrong cause")
            };
            assert_eq!(f.request, requests[cut]);
            assert_eq!(f.error, cut);
            assert_eq!(trace.accepted, requests[..cut]);
            let mut other = make();
            assert!(matches!(
                other.parse_and_bind_font_with_binding_budget(retry.clone(), &mut trace),
                Ok(Err(ProductionResourceErrorV3::Resource(
                    ResourceAdmissionError::ReceiptSessionMismatch
                )))
            ));
            assert_eq!(trace.accepted.len(), cut);
            trace.stop = None;
            resolver
                .parse_and_bind_font_with_binding_budget(retry.clone(), &mut trace)
                .unwrap()
                .unwrap();
            let before = trace.accepted.len();
            assert!(matches!(
                resolver.parse_and_bind_font_with_binding_budget(retry, &mut trace),
                Ok(Err(ProductionResourceErrorV3::Resource(
                    ResourceAdmissionError::ConflictingLogicalResource
                )))
            ));
            assert_eq!(trace.accepted.len(), before + 1);
            let actual = finish(resolver, &host);
            assert_eq!(actual.canonical_jcs(), native.canonical_jcs());
            assert_eq!(actual.fingerprint(), native.fingerprint());
            total_cuts += 1;
        }
    }
    // An observed first admission also permits a later ordinary admission.
    let mut mixed = make();
    let mut trace = Trace {
        observed: true,
        ..Default::default()
    };
    let p = mixed
        .read_font(host.open_font(FontFaceId::new(2)).unwrap())
        .unwrap();
    bind(&mut mixed, p, &mut trace);
    trace.observed = false;
    let p = mixed
        .read_font(host.open_font(FontFaceId::new(1)).unwrap())
        .unwrap();
    bind(&mut mixed, p, &mut trace);
    assert_eq!(finish(mixed, &host).canonical_jcs(), native.canonical_jcs());
    eprintln!("CFF binding cuts={total_cuts}; migration/reuse/mixed/native equivalence passed");
}

#[test]
fn production_cff_binding_native_error_order_is_independent_of_observation() {
    let tree = TempTree::new("cff-binding-native-errors");
    write_base(&tree);
    for id in 1..3 {
        fs::write(tree.path().join(format!("harano-{id}.bin")), CFF).unwrap();
    }
    let declarations = declared(Some(CFF));
    let caps = effective();
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let context = host_context(tree.path(), &[]);
    let host = HostResourceAdmissionSession::new_borrowed(
        &context,
        &config,
        declarations.resource_catalog(),
    )
    .unwrap();
    for case in 0..6 {
        let mut resolver =
            StagingProductionResourceResolverV3::new(&declarations, &caps, [187; 32], host.roots())
                .unwrap();
        let pending = resolver
            .read_font(host.open_font(FontFaceId::new(1)).unwrap())
            .unwrap();
        let retry = pending.clone();
        let mut damaged = pending;
        match case {
            0 => damaged.id = PendingResourceId::Image(ImageResourceId::new(0)),
            1 => damaged.id = PendingResourceId::Font(FontFaceId::new(99)),
            2 => {
                damaged.uri = PortablePath::new("wrong.otf").unwrap();
                damaged.sha256 = [0; 32];
            }
            3 => {
                damaged.face_index = Some(1);
                damaged.sha256 = [0; 32];
            }
            4 => damaged.sha256 = [0; 32],
            _ => damaged.bytes[0] = 0,
        }
        let native = resolver.parse_and_bind_font(damaged.clone()).unwrap_err();
        let mut trace = Trace::default();
        let observed = resolver
            .parse_and_bind_font_with_binding_budget(damaged, &mut trace)
            .unwrap()
            .unwrap_err();
        assert_eq!(format!("{native:?}"), format!("{observed:?}"));
        assert_eq!(trace.accepted.len(), usize::from(case > 1));
        resolver
            .parse_and_bind_font_with_binding_budget(retry, &mut trace)
            .unwrap()
            .unwrap();
    }
}
