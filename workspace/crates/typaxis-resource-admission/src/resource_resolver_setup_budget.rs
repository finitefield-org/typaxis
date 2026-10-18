//! Fund resolver-owned copies and identities before issuing a resolver.
use super::*;
use crate::staging_catalog_budget::{charge, point, rows, string, uri};
use ResourceCatalogStage as S;

fn owner<B: ResourceCatalogBudget>(
    stage: ResourceCatalogStage,
    value_bytes: usize,
    budget: &mut B,
) -> Result<(), ResourceCatalogError<B::Error>> {
    let bytes = std::mem::size_of::<[std::sync::atomic::AtomicUsize; 2]>()
        .checked_add(value_bytes)
        .and_then(|n| u64::try_from(n).ok())
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    charge(
        budget,
        ResourceCatalogRequest {
            records: 1,
            storage_bytes: bytes,
            ..point(stage, None, None)
        },
    )
}

impl<'roots> StagingProductionResourceResolverV3<'roots> {
    /// Set up the same resolver under the catalog budget. The sealed declared
    /// catalog already proves dense IDs, valid unique family names and closed
    /// media declarations, so this path does not rebuild a validation table.
    /// Root authority, per-resource/aggregate limits and later declaration/read
    /// identity checks remain the same. Refusal publishes no resolver.
    ///
    /// Arc owners are funded first; std Arc allocation remains infallible.
    pub fn new_with_catalog_budget<B: ResourceCatalogBudget>(
        declared: &StagingDeclaredBaseCatalog,
        limits: &M4EffectiveResourceLimits,
        profile_fingerprint: [u8; 32],
        roots: HostRootSetToken<'roots>,
        budget: &mut B,
    ) -> Result<Self, ResourceCatalogError<B::Error>> {
        if !B::OBSERVED {
            return Ok(Self {
                inner: AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
                    declared,
                    limits,
                    profile_fingerprint,
                    roots,
                )?,
                cff_fonts: binding::Fonts::new(),
                profile_fingerprint,
                limits: limits.clone(),
            });
        }
        let start = u64::try_from(std::mem::size_of::<ValidatedResourceLimits>())
            .ok()
            .and_then(|n| n.checked_add(64))
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        charge(
            budget,
            ResourceCatalogRequest {
                work: start,
                ..point(S::ResolverStart, None, None)
            },
        )?;
        let admission_budget =
            ResourceAdmissionBudget::new(declared.resource_catalog(), limits.base())?;
        let mut fonts = rows(declared.font_faces.len(), S::ResolverFontRows, budget)?;
        for d in &declared.font_faces {
            let id = Some(d.font_face_id);
            charge(budget, point(S::ResolverFont, id, None))?;
            let family = string(&d.family, point(S::ResolverFamilyCopy, id, None), budget)?;
            let uri = uri(&d.uri, point(S::ResolverUriCopy, id, None), budget)?;
            fonts.push(FontFaceDeclaration {
                font_face_id: d.font_face_id,
                family,
                uri,
                face_index: d.face_index,
                expected_sha256: d.expected_sha256,
            });
        }
        let mut images = rows(declared.images.len(), S::ResolverImageRows, budget)?;
        for d in &declared.images {
            let id = Some(d.image_id);
            charge(budget, point(S::ResolverImage, None, id))?;
            let uri = uri(&d.uri, point(S::ResolverUriCopy, None, id), budget)?;
            images.push(ImageDeclaration {
                image_id: d.image_id,
                uri,
                expected_sha256: d.expected_sha256,
            });
        }
        let mut font_media = rows(declared.font_media.len(), S::ResolverFontMedia, budget)?;
        let mut image_media = rows(declared.image_media.len(), S::ResolverImageMedia, budget)?;
        let media_bytes = declared
            .font_media
            .len()
            .checked_mul(std::mem::size_of::<FontMediaType>())
            .and_then(|n| {
                declared
                    .image_media
                    .len()
                    .checked_mul(std::mem::size_of::<ImageMediaType>())
                    .and_then(|m| n.checked_add(m))
            })
            .and_then(|n| u64::try_from(n).ok())
            .and_then(|n| n.checked_add(64))
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        charge(
            budget,
            ResourceCatalogRequest {
                work: media_bytes,
                ..point(S::ResolverMediaCopy, None, None)
            },
        )?;
        font_media.extend_from_slice(&declared.font_media);
        image_media.extend_from_slice(&declared.image_media);
        owner(
            S::ResolverMediaOwner,
            std::mem::size_of::<DeclaredMediaPolicy>(),
            budget,
        )?;
        let media = Arc::new(DeclaredMediaPolicy {
            fonts: font_media,
            images: image_media,
        });
        owner(S::ResolverSessionOwner, 0, budget)?;
        let session = ResourceAdmissionSessionIdentity::fresh();
        let limit_work = std::mem::size_of::<M4EffectiveResourceLimits>()
            .checked_mul(2)
            .and_then(|n| u64::try_from(n).ok())
            .and_then(|n| n.checked_add(64))
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        charge(
            budget,
            ResourceCatalogRequest {
                work: limit_work,
                ..point(S::ResolverLimits, None, None)
            },
        )?;
        let mut next = 0;
        for (index, kind) in media.images.iter().enumerate() {
            let id = ImageResourceId::new(
                u32::try_from(index).map_err(|_| ResourceAdmissionError::NonCanonicalResourceId)?,
            );
            charge(budget, point(S::ResolverImageCursor, None, Some(id)))?;
            if matches!(kind, ImageMediaType::SvgSafe1 | ImageMediaType::SvgSafe2) {
                break;
            }
            next = index + 1;
        }
        let inner = AdmittedResourceResolver {
            session,
            roots: Some(roots),
            declarations: ResourceCatalog {
                font_faces: fonts,
                images,
            },
            declared_media_policy: Some(media),
            budget: admission_budget,
            attempted_fonts: resource_attempts::Attempts::new(),
            attempted_images: resource_attempts::Attempts::new(),
            fonts: BTreeMap::new(),
            images: image_bindings::Images::new(),
            m4_limits: Some(limits.clone()),
            m4_profile_fingerprint: Some(profile_fingerprint),
            vector_nodes_used: 0,
            vector_path_work_used: 0,
            next_vector_declaration_index: next,
        };
        charge(budget, point(S::ResolverFinish, None, None))?;
        Ok(Self {
            inner,
            cff_fonts: binding::Fonts::new(),
            profile_fingerprint,
            limits: limits.clone(),
        })
    }
}
