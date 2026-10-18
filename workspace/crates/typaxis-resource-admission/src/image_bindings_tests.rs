use super::*;
use crate::*;
use typaxis_core::*;
#[derive(Default)]
struct Trace {
    requests: Vec<SafeSvgSourceRequest>,
    stop: Option<usize>,
}
impl SafeSvgSourceBudget for Trace {
    type Error = usize;
    const OBSERVE_BINDING: bool = true;
    fn work(&mut self, r: SafeSvgSourceRequest) -> Result<(), usize> {
        let at = self.requests.len();
        if self.stop == Some(at) {
            return Err(at);
        }
        self.requests.push(r);
        Ok(())
    }
}
fn image(id: u32) -> AdmittedImage {
    AdmittedImage::from_verified(
        ImageResourceId::new(id),
        PortablePath::new(format!("image-{id}.png")).unwrap(),
        vec![1],
        sha256(&[1]),
        AdmittedImageMediaKind::Png,
        std::num::NonZeroU32::MIN,
        std::num::NonZeroU32::MIN,
        1,
    )
}
fn initial() -> Images {
    let mut values = Images::new();
    values.insert(ImageResourceId::new(3), image(3)).unwrap();
    values.insert(ImageResourceId::new(1), image(1)).unwrap();
    values
}
fn ids(values: &Images) -> Vec<u32> {
    values.values().map(|v| v.image_id().get()).collect()
}
#[test]
fn observed_image_bindings_preserve_order_and_refuse_each_preparation_step() {
    let mut full = initial();
    let mut trace = Trace::default();
    let at = full
        .prepare(6, &mut trace, ImageResourceId::new(2))
        .unwrap();
    assert_eq!(at, 1);
    full.insert_prepared(at, image(2)).unwrap();
    assert_eq!(ids(&full), vec![1, 2, 3]);
    assert_eq!(
        (trace.requests[0].records, trace.requests[0].storage_bytes),
        (6, 6 * std::mem::size_of::<AdmittedImage>() as u64)
    );
    for stop in 0..trace.requests.len() {
        let mut values = initial();
        let mut short = Trace {
            stop: Some(stop),
            ..Trace::default()
        };
        let SafeSvgSourceError::Budget(f) = values
            .prepare(6, &mut short, ImageResourceId::new(2))
            .unwrap_err()
        else {
            panic!("unpaid binding operation")
        };
        assert_eq!(f.error, stop);
        assert_eq!(f.request, trace.requests[stop]);
        assert_eq!(short.requests, trace.requests[..stop]);
        assert_eq!(ids(&values), vec![1, 3]);
        short.stop = None;
        let at = values
            .prepare(6, &mut short, ImageResourceId::new(2))
            .unwrap();
        values.insert_prepared(at, image(2)).unwrap();
        assert_eq!(ids(&values), vec![1, 2, 3]);
    }
    eprintln!(
        "SVG binding preparation callback refusals={}",
        trace.requests.len()
    );
}
#[test]
fn observed_image_bindings_keep_capacity_and_native_insert_semantics() {
    let mut values = Images::new();
    let mut trace = Trace::default();
    let at = values
        .prepare(4, &mut trace, ImageResourceId::new(2))
        .unwrap();
    values.insert_prepared(at, image(2)).unwrap();
    for id in [0, 3, 1] {
        values.insert(ImageResourceId::new(id), image(id)).unwrap();
    }
    assert_eq!(ids(&values), vec![0, 1, 2, 3]);
    assert_eq!(values.len(), 4);
    assert!(values.contains_key(&ImageResourceId::new(3)));
    assert!(!values.contains_key(&ImageResourceId::new(4)));
    assert!(values
        .insert(ImageResourceId::new(2), image(2))
        .unwrap()
        .is_some());
    assert!(matches!(
        values.prepare(4, &mut trace, ImageResourceId::new(2)),
        Err(SafeSvgSourceError::Native(
            ResourceAdmissionError::ConflictingLogicalResource
        ))
    ));
    assert!(matches!(
        values.prepare(5, &mut trace, ImageResourceId::new(4)),
        Err(SafeSvgSourceError::Native(
            ResourceAdmissionError::ReceiptIdentityMismatch
        ))
    ));
    assert!(matches!(
        values.insert(ImageResourceId::new(4), image(4)),
        Err(ResourceAdmissionError::ReceiptIdentityMismatch)
    ));
    assert_eq!(
        values
            .into_values()
            .map(|v| v.image_id().get())
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    let mut values = Images::new();
    let mut trace = Trace::default();
    assert!(matches!(
        values.prepare(usize::MAX, &mut trace, ImageResourceId::new(0)),
        Err(SafeSvgSourceError::Native(
            ResourceAdmissionError::ResourceLimit
        ))
    ));
    assert!(trace.requests.is_empty());
}

#[test]
fn observed_svg_binding_real_host_mixed_images_matches_native_and_retries() {
    use typaxis_document::{
        ImageMediaDeclaration, ImageMediaType, StagingM4ImageDeclaration, StagingM4ResourceCatalog,
    };
    struct Full(Trace);
    impl SafeSvgSourceBudget for Full {
        type Error = usize;
        const OBSERVE_IR: bool = true;
        const OBSERVE_BINDING: bool = true;
        fn work(&mut self, r: SafeSvgSourceRequest) -> Result<(), usize> {
            self.0.work(r)
        }
    }
    let token = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "typaxis-svg-binding-{}-{token}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    let svg=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/fraction-equality.svg"));
    let png=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/vmb-book/raster/orientation-alpha.png"));
    std::fs::write(root.join("formula.svg"), svg).unwrap();
    std::fs::write(root.join("cover.png"), png).unwrap();
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
    let caps = M4EffectiveResourceLimits::defaults_for(config.limits());
    let declarations = staging_declared_base_catalog(&StagingM4ResourceCatalog {
        font_faces: vec![],
        images: (0..4)
            .map(|i| {
                let vector = i % 2 == 1;
                StagingM4ImageDeclaration {
                    image_id: ImageResourceId::new(i),
                    uri: PortablePath::new(if vector { "formula.svg" } else { "cover.png" })
                        .unwrap(),
                    expected_sha256: Some(sha256(if vector {
                        svg.as_slice()
                    } else {
                        png.as_slice()
                    })),
                    media: ImageMediaDeclaration::Declared(if vector {
                        ImageMediaType::SvgSafe2
                    } else {
                        ImageMediaType::Png
                    }),
                    vector_provenance: None,
                }
            })
            .collect(),
    })
    .unwrap();
    let context = HostAdmissionContext::new(
        HostPath::new(root.join("input.typ")).unwrap(),
        HostPath::new(root.clone()).unwrap(),
        None,
        vec![],
    );
    let host =
        HostResourceAdmissionSession::new(&context, &config, declarations.resource_catalog())
            .unwrap();
    let make = || {
        AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &declarations,
            &caps,
            [131; 32],
            host.roots(),
        )
        .unwrap()
    };
    let build = |trace: &mut Full,
                 observed: bool|
     -> Result<AdmittedResourceLedger, SafeSvgSourceError<usize>> {
        let mut resolver = make();
        for i in 0..4 {
            let pending = resolver
                .read_image(host.open_image(ImageResourceId::new(i)).unwrap())
                .unwrap();
            if observed {
                resolver.parse_and_bind_declared_image_with_source_budget(pending, trace)?;
            } else {
                resolver.parse_and_bind_declared_image(pending)?;
            }
        }
        resolver.finish().map_err(Into::into)
    };
    let native = build(&mut Full(Trace::default()), false).unwrap();
    let mut full = Full(Trace::default());
    let observed = build(&mut full, true).unwrap();
    assert_eq!(native.images(), observed.images());
    assert_eq!(native.fingerprint(), observed.fingerprint());
    let storage = full
        .0
        .requests
        .iter()
        .filter(|r| r.stage == S::BindingStorage)
        .collect::<Vec<_>>();
    assert_eq!(storage.len(), 1);
    assert_eq!(
        (storage[0].records, storage[0].storage_bytes),
        (4, 4 * std::mem::size_of::<AdmittedImage>() as u64)
    );
    assert_eq!(
        full.0
            .requests
            .iter()
            .filter(|r| r.stage == S::IrOwner)
            .count(),
        2
    );
    let mut refusals = 0;
    for (stop, request) in full.0.requests.iter().enumerate().filter(|(_, r)| {
        matches!(
            r.stage,
            S::IrOwner
                | S::BindingStorage
                | S::BindingMigrate
                | S::BindingLookup
                | S::BindingInsert
                | S::BindingAdvance
        )
    }) {
        let mut trace = Full(Trace {
            stop: Some(stop),
            ..Trace::default()
        });
        let mut resolver = make();
        for id in 0..=request.image_id.get() {
            let pending = resolver
                .read_image(host.open_image(ImageResourceId::new(id)).unwrap())
                .unwrap();
            if id < request.image_id.get() {
                resolver
                    .parse_and_bind_declared_image_with_source_budget(pending, &mut trace)
                    .unwrap();
                continue;
            }
            let retry = pending.clone();
            let before = resolver.progress_token();
            let SafeSvgSourceError::Budget(f) = resolver
                .parse_and_bind_declared_image_with_source_budget(pending, &mut trace)
                .unwrap_err()
            else {
                panic!("unpaid observed binding")
            };
            assert_eq!(f.error, stop);
            assert_eq!(f.request, *request);
            assert_eq!(trace.0.requests, full.0.requests[..stop]);
            assert_eq!(before.images(), resolver.progress_token().images());
            trace.0.stop = None;
            resolver
                .parse_and_bind_declared_image_with_source_budget(retry, &mut trace)
                .unwrap();
            for later in id + 1..4 {
                let pending = resolver
                    .read_image(host.open_image(ImageResourceId::new(later)).unwrap())
                    .unwrap();
                resolver
                    .parse_and_bind_declared_image_with_source_budget(pending, &mut trace)
                    .unwrap();
            }
            assert_eq!(resolver.finish().unwrap().images(), native.images());
            break;
        }
        refusals += 1;
    }
    std::fs::remove_dir_all(root).unwrap();
    eprintln!("SVG real-host binding images=4 vectors=2 callback refusals={refusals} migration=1 native_equal=true");
}

#[test]
fn observed_image_bindings_scale_keeps_one_reservation_and_sorted_ids() {
    for (count, order) in [(512u32, 0), (512, 1), (512, 2), (8192, 0)] {
        let mut input = (0..count).collect::<Vec<_>>();
        if order == 1 {
            input.reverse();
        }
        if order == 2 {
            input.rotate_left(count as usize / 3);
        }
        let mut table = Images::new();
        let mut trace = Trace::default();
        for id in input {
            let at = table
                .prepare(count as usize, &mut trace, ImageResourceId::new(id))
                .unwrap();
            table.insert_prepared(at, image(id)).unwrap();
        }
        assert_eq!(
            table
                .values()
                .map(|v| v.image_id().get())
                .collect::<Vec<_>>(),
            (0..count).collect::<Vec<_>>()
        );
        assert_eq!(
            trace
                .requests
                .iter()
                .filter(|v| v.stage == S::BindingStorage)
                .count(),
            1
        );
        let Images::Reserved { values, bound } = table else {
            panic!()
        };
        assert_eq!(bound, count as usize);
        assert!(values.capacity() >= bound);
    }
    eprintln!(
        "SVG image binding scale=8192 reverse_and_rotated=512 storage_reservations_per_table=1"
    );
}
