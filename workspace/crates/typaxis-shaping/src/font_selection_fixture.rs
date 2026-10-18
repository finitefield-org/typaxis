//! Genuine host admission for selection tests; no synthesized admission tokens.
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use typaxis_core::*;
use typaxis_document::{
    FontFaceDeclaration, FontMediaDeclaration, FontMediaType, ResourceCatalog,
    StagingM4FontFaceDeclaration, StagingM4ResourceCatalog,
};
use typaxis_resource_admission::*;

pub(crate) const FAMILIES: [&str; 4] = ["Zeta", "Alpha", "和文", "e\u{0301}"];
pub(crate) const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"
));
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
pub(crate) fn admit() -> (
    AdmittedResourceLedger,
    AdmittedProductionResourceLedgerV3,
    M4EffectiveResourceLimits,
) {
    let root = Root(std::env::temp_dir().join(format!(
        "typaxis-selection-ledgers-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    fs::create_dir(&root.0).unwrap();
    let catalog = ResourceCatalog {
        font_faces: FAMILIES
            .iter()
            .enumerate()
            .map(|(i, name)| FontFaceDeclaration {
                font_face_id: FontFaceId::new(i as u32),
                family: (*name).to_owned(),
                uri: PortablePath::new(format!("font-{i}.ttf")).unwrap(),
                face_index: 0,
                expected_sha256: Some(sha256(FONT)),
            })
            .collect(),
        images: vec![],
    };
    for i in 0..FAMILIES.len() {
        fs::write(root.0.join(format!("font-{i}.ttf")), FONT).unwrap();
    }
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
    let limits =
        M4EffectiveResourceLimits::new(config.limits().clone(), M4ResourceLimits::default())
            .unwrap();
    let context = HostAdmissionContext::new(
        HostPath::new(root.0.join("input.typ")).unwrap(),
        HostPath::new(root.0.clone()).unwrap(),
        None,
        vec![],
    );
    let host = HostResourceAdmissionSession::new(&context, &config, &catalog).unwrap();
    let mut legacy =
        AdmittedResourceResolver::new_with_roots(&catalog, config.limits(), host.roots()).unwrap();
    for declaration in &catalog.font_faces {
        let pending = legacy
            .read_font(host.open_font(declaration.font_face_id).unwrap())
            .unwrap();
        legacy.parse_and_bind_sfnt(pending).unwrap();
    }
    let legacy = legacy.finish().unwrap();
    let declared = StagingM4ResourceCatalog {
        font_faces: catalog
            .font_faces
            .iter()
            .map(|font| StagingM4FontFaceDeclaration {
                font_face_id: font.font_face_id,
                family: font.family.clone(),
                uri: font.uri.clone(),
                face_index: font.face_index,
                expected_sha256: font.expected_sha256,
                media: FontMediaDeclaration::Declared(FontMediaType::SfntTrueTypeGlyf),
            })
            .collect(),
        images: vec![],
    };
    let declared = staging_declared_base_catalog(&declared).unwrap();
    let mut modern =
        StagingProductionResourceResolverV3::new(&declared, &limits, [91; 32], host.roots())
            .unwrap();
    for declaration in &catalog.font_faces {
        let pending = modern
            .read_font(host.open_font(declaration.font_face_id).unwrap())
            .unwrap();
        modern.parse_and_bind_font(pending).unwrap();
    }
    (legacy, modern.finish().unwrap(), limits)
}
