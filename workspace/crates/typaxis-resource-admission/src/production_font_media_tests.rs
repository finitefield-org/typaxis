use super::*;
#[derive(Default)]
struct Media {
    calls: usize,
    stop: Option<usize>,
}
impl typaxis_font::CffBaseCmapBudgetV2 for Media {
    type Error = usize;
    fn reserve(
        &mut self,
        _: typaxis_font::CffBaseCmapStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        Ok(())
    }
    fn work(&mut self, _: u64) -> Result<(), usize> {
        Ok(())
    }
}
impl typaxis_font::CffCmapBudgetV2 for Media {
    const OBSERVE_VARIATIONS: bool = false;
    fn reserve_variation(
        &mut self,
        _: typaxis_font::CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        Ok(())
    }
}
impl typaxis_font::CffAdmissionBudgetV2 for Media {
    const OBSERVE_SFNT: bool = false;
    const OBSERVE_RESOURCE_MEDIA: bool = true;
    fn reserve_sfnt(
        &mut self,
        _: typaxis_font::CffSfntStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), usize> {
        Ok(())
    }
    fn work_sfnt(&mut self, _: u64) -> Result<(), usize> {
        if self.stop == Some(self.calls) {
            return Err(self.calls);
        }
        self.calls += 1;
        Ok(())
    }
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn production_media_refusal_keeps_same_resolver_retry_and_authentic_subject() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        sha256(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let tree = TempTree::new("production-media-retry");
    write_base(&tree);
    for id in 1..3 {
        fs::write(tree.path().join(format!("harano-{id}.bin")), &bytes).unwrap();
    }
    let declarations = declared(Some(&bytes));
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
        StagingProductionResourceResolverV3::new(&declarations, &caps, [181; 32], host.roots())
            .unwrap()
    };
    fn finish<'r, C: std::borrow::Borrow<ResourceCatalog>>(
        mut resolver: StagingProductionResourceResolverV3<'r>,
        host: &'r HostResourceAdmissionSession<C>,
    ) -> AdmittedProductionResourceLedgerV3 {
        for id in [0, 2] {
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
    }
    let mut native = make();
    let pending = native
        .read_font(host.open_font(FontFaceId::new(1)).unwrap())
        .unwrap();
    native.parse_and_bind_font(pending).unwrap();
    let native = finish(native, &host);
    let count = 1 + u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    for stop in 0..count {
        let mut resolver = make();
        let mut other = make();
        let pending = resolver
            .read_font(host.open_font(FontFaceId::new(1)).unwrap())
            .unwrap();
        let retry = pending.clone();
        let mut trace = Media {
            stop: Some(stop),
            ..Default::default()
        };
        let failure = resolver
            .parse_and_bind_font_with_admission_budget(pending, &mut trace)
            .unwrap_err();
        assert_eq!(failure.font_face_id.get(), 1);
        let typaxis_font::CffAdmissionBudgetFailureV2::Sfnt(f) = failure.failure else {
            panic!("wrong failure")
        };
        assert_eq!(f.error, stop);
        assert_eq!(trace.calls, stop);
        let calls = trace.calls;
        assert!(matches!(
            other.parse_and_bind_font_with_admission_budget(retry.clone(), &mut trace),
            Ok(Err(ProductionResourceErrorV3::Resource(
                ResourceAdmissionError::ReceiptSessionMismatch
            )))
        ));
        assert_eq!(trace.calls, calls);
        trace.stop = None;
        resolver
            .parse_and_bind_font_with_admission_budget(retry, &mut trace)
            .unwrap()
            .unwrap();
        assert_eq!(trace.calls, stop + count);
        let observed = finish(resolver, &host);
        assert_eq!(observed.canonical_jcs(), native.canonical_jcs());
        assert_eq!(observed.fingerprint(), native.fingerprint());
    }
    eprintln!("original CFF media refusals={count} same_resolver_retry=true authentic_font=1");
}
