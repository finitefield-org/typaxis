use super::*;
use typaxis_document::{StagingM4FontFaceDeclaration, StagingM4ImageDeclaration};

#[derive(Default)]
struct Trace {
    accepted: Vec<ResourceCatalogRequest>,
    stop: Option<usize>,
}
impl ResourceCatalogBudget for Trace {
    type Error = usize;
    fn charge(&mut self, r: ResourceCatalogRequest) -> Result<(), usize> {
        let at = self.accepted.len();
        if self.stop == Some(at) {
            return Err(at);
        }
        self.accepted.push(r);
        Ok(())
    }
}
fn declarations(n: u32) -> StagingM4ResourceCatalog {
    StagingM4ResourceCatalog {
        font_faces: (0..n)
            .map(|id| StagingM4FontFaceDeclaration {
                font_face_id: FontFaceId::new(id),
                family: format!("本文-{:04}", n - id),
                uri: PortablePath::new(format!("font/和-{id}.bin")).unwrap(),
                face_index: id,
                expected_sha256: Some([id as u8; 32]),
                media: FontMediaDeclaration::Declared(if id % 2 == 0 {
                    FontMediaType::SfntTrueTypeGlyf
                } else {
                    FontMediaType::SfntCff1
                }),
            })
            .collect(),
        images: (0..n)
            .map(|id| StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(id),
                uri: PortablePath::new(format!("image/図-{id}.bin")).unwrap(),
                expected_sha256: Some([id as u8; 32]),
                media: ImageMediaDeclaration::Declared(if id % 2 == 0 {
                    ImageMediaType::SvgSafe2
                } else {
                    ImageMediaType::Png
                }),
                vector_provenance: None,
            })
            .collect(),
    }
}

#[test]
fn observed_catalog_matches_native_and_refuses_each_request_before_returning_a_catalog() {
    let mut refusals = 0;
    for n in [0, 1, 7] {
        let input = declarations(n);
        let unchanged = input.clone();
        let native = staging_declared_base_catalog(&input).unwrap();
        let mut full = Trace::default();
        let observed = staging_declared_base_catalog_with_budget(&input, &mut full).unwrap();
        assert_eq!(observed, native);
        assert_eq!(input, unchanged);
        for stop in 0..full.accepted.len() {
            let mut trace = Trace {
                stop: Some(stop),
                ..Trace::default()
            };
            let ResourceCatalogError::Budget(failure) =
                staging_declared_base_catalog_with_budget(&input, &mut trace).unwrap_err()
            else {
                panic!("wrong error")
            };
            assert_eq!(failure.error, stop);
            assert_eq!(failure.request, full.accepted[stop]);
            assert_eq!(trace.accepted, full.accepted[..stop]);
            assert_eq!(input, unchanged);
            trace.stop = None;
            assert_eq!(
                staging_declared_base_catalog_with_budget(&input, &mut trace).unwrap(),
                native
            );
            assert_eq!(&trace.accepted[stop..], full.accepted);
            refusals += 1;
        }
    }
    println!("catalog complete-prefix callback refusals={refusals}");
}

#[test]
fn observed_catalog_preserves_native_error_precedence_and_all_declaration_fields() {
    let original = declarations(3);
    let mut cases = vec![];
    let mut add = |edit: fn(&mut StagingM4ResourceCatalog)| {
        let mut v = original.clone();
        edit(&mut v);
        cases.push(v);
    };
    add(|v| v.font_faces[0].media = FontMediaDeclaration::LegacyUnspecified);
    add(|v| v.images[0].media = ImageMediaDeclaration::LegacyUnspecified);
    add(|v| v.images[0].expected_sha256 = None);
    add(|v| v.font_faces[0].font_face_id = FontFaceId::new(2));
    add(|v| v.images[1].image_id = ImageResourceId::new(0));
    add(|v| v.font_faces[1].family = "\u{2003}\t".into());
    add(|v| v.font_faces[1].family.push('\u{0085}'));
    add(|v| v.font_faces[1].family = v.font_faces[0].family.clone());
    add(|v| {
        v.font_faces[1].family = v.font_faces[0].family.clone();
        v.images[1].image_id = ImageResourceId::new(0);
    });
    add(|v| {
        v.font_faces[0].font_face_id = FontFaceId::new(9);
        v.images[1].media = ImageMediaDeclaration::LegacyUnspecified;
    });
    for input in cases {
        let native = staging_declared_base_catalog(&input).unwrap_err();
        assert!(matches!(
            staging_declared_base_catalog_with_budget(&input, &mut Trace::default()),
            Err(ResourceCatalogError::Native(error)) if error == native
        ));
    }
    struct Ordinary;
    impl ResourceCatalogBudget for Ordinary {
        type Error = std::convert::Infallible;
        const OBSERVED: bool = false;
        fn charge(&mut self, _: ResourceCatalogRequest) -> Result<(), Self::Error> {
            panic!("ordinary constructor must not call hooks")
        }
    }
    assert_eq!(
        staging_declared_base_catalog_with_budget(&original, &mut Ordinary).unwrap(),
        staging_declared_base_catalog(&original).unwrap()
    );
}

#[test]
fn observed_catalog_family_index_scales_and_keeps_duplicate_error_order() {
    for n in [64, 8192] {
        let input = declarations(n);
        let result =
            staging_declared_base_catalog_with_budget(&input, &mut Trace::default()).unwrap();
        assert_eq!(result, staging_declared_base_catalog(&input).unwrap());
        let mut duplicate = input;
        duplicate.font_faces[n as usize - 1].family = duplicate.font_faces[0].family.clone();
        assert!(matches!(
            staging_declared_base_catalog_with_budget(&duplicate, &mut Trace::default()),
            Err(ResourceCatalogError::Native(
                ResourceAdmissionError::InvalidFontFamily
            ))
        ));
    }
}
