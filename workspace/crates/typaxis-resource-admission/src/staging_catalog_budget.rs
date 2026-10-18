//! Fallible preparation of the declared catalog on caller-owned source cells.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceCatalogStage {
    Start,
    FontRows,
    FontMedia,
    Font,
    FamilyCopy,
    UriCopy,
    ImageRows,
    ImageMedia,
    Image,
    FontId,
    ImageId,
    FamilyIndex,
    FamilyValidate,
    FamilyLookup,
    FamilyInsert,
    Finish,
    ResolverStart,
    ResolverFontRows,
    ResolverFont,
    ResolverFamilyCopy,
    ResolverUriCopy,
    ResolverImageRows,
    ResolverImage,
    ResolverFontMedia,
    ResolverImageMedia,
    ResolverMediaCopy,
    ResolverMediaOwner,
    ResolverSessionOwner,
    ResolverLimits,
    ResolverImageCursor,
    ResolverFinish,
    CffBindingChecks,
    CffBindingStorage,
    CffBindingMigrate,
    CffBindingLookup,
    CffBindingInsert,
    CffBindingFamily,
    CffBindingUri,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceCatalogRequest {
    pub stage: ResourceCatalogStage,
    pub font_face_id: Option<FontFaceId>,
    pub image_id: Option<ImageResourceId>,
    pub records: u64,
    pub storage_bytes: u64,
    pub work: u64,
}

pub trait ResourceCatalogBudget {
    type Error;
    const OBSERVED: bool = true;
    /// Accept the entire request before its allocation or bounded operation.
    /// Earlier accepted requests remain spent after refusal or native error.
    fn charge(&mut self, request: ResourceCatalogRequest) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub struct ResourceCatalogBudgetFailure<E> {
    pub error: E,
    pub request: ResourceCatalogRequest,
}
impl<E: std::fmt::Debug> std::fmt::Display for ResourceCatalogBudgetFailure<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "resource catalog budget: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for ResourceCatalogBudgetFailure<E> {}

#[derive(Debug)]
pub enum ResourceCatalogError<E> {
    Native(ResourceAdmissionError),
    Budget(ResourceCatalogBudgetFailure<E>),
}
impl<E> From<ResourceAdmissionError> for ResourceCatalogError<E> {
    fn from(error: ResourceAdmissionError) -> Self {
        Self::Native(error)
    }
}

pub(super) fn number(n: usize) -> Result<u64, ResourceAdmissionError> {
    u64::try_from(n).map_err(|_| ResourceAdmissionError::ResourceLimit)
}
pub(super) fn amount(n: usize, multiple: u64, fixed: u64) -> Result<u64, ResourceAdmissionError> {
    number(n)?
        .checked_mul(multiple)
        .and_then(|n| n.checked_add(fixed))
        .ok_or(ResourceAdmissionError::ResourceLimit)
}
pub(super) fn point(
    stage: ResourceCatalogStage,
    font_face_id: Option<FontFaceId>,
    image_id: Option<ImageResourceId>,
) -> ResourceCatalogRequest {
    ResourceCatalogRequest {
        stage,
        font_face_id,
        image_id,
        records: 0,
        storage_bytes: 0,
        work: 64,
    }
}
pub(super) fn charge<B: ResourceCatalogBudget>(
    budget: &mut B,
    request: ResourceCatalogRequest,
) -> Result<(), ResourceCatalogError<B::Error>> {
    budget.charge(request).map_err(|error| {
        ResourceCatalogError::Budget(ResourceCatalogBudgetFailure { error, request })
    })
}
pub(super) fn rows<T, B: ResourceCatalogBudget>(
    count: usize,
    stage: ResourceCatalogStage,
    budget: &mut B,
) -> Result<Vec<T>, ResourceCatalogError<B::Error>> {
    let mut result = Vec::new();
    if count != 0 {
        let bytes = count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        charge(
            budget,
            ResourceCatalogRequest {
                records: number(count)?,
                storage_bytes: number(bytes)?,
                ..point(stage, None, None)
            },
        )?;
        result
            .try_reserve_exact(count)
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    }
    Ok(result)
}
pub(super) fn string<B: ResourceCatalogBudget>(
    text: &str,
    p: ResourceCatalogRequest,
    budget: &mut B,
) -> Result<String, ResourceCatalogError<B::Error>> {
    charge(
        budget,
        ResourceCatalogRequest {
            records: u64::from(!text.is_empty()),
            storage_bytes: number(text.len())?,
            work: amount(text.len(), 16, 64)?,
            ..p
        },
    )?;
    let mut owned = String::new();
    owned
        .try_reserve_exact(text.len())
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    owned.push_str(text);
    Ok(owned)
}
pub(super) fn uri<B: ResourceCatalogBudget>(
    path: &PortablePath,
    p: ResourceCatalogRequest,
    budget: &mut B,
) -> Result<PortablePath, ResourceCatalogError<B::Error>> {
    PortablePath::new(string(path.as_str(), p, budget)?)
        .map_err(|_| ResourceAdmissionError::ReceiptIdentityMismatch.into())
}

fn validate<B: ResourceCatalogBudget>(
    catalog: &ResourceCatalog,
    budget: &mut B,
) -> Result<(), ResourceCatalogError<B::Error>> {
    use ResourceCatalogStage as S;
    // Keep native error order: all font IDs, all image IDs, then the first
    // invalid/duplicate family in declaration order.
    for (index, d) in catalog.font_faces.iter().enumerate() {
        charge(budget, point(S::FontId, Some(d.font_face_id), None))?;
        if u32::try_from(index).ok() != Some(d.font_face_id.get()) {
            return Err(ResourceAdmissionError::NonCanonicalResourceId.into());
        }
    }
    for (index, d) in catalog.images.iter().enumerate() {
        charge(budget, point(S::ImageId, None, Some(d.image_id)))?;
        if u32::try_from(index).ok() != Some(d.image_id.get()) {
            return Err(ResourceAdmissionError::NonCanonicalResourceId.into());
        }
    }
    let mut names: Vec<&str> = rows(catalog.font_faces.len(), S::FamilyIndex, budget)?;
    for d in &catalog.font_faces {
        let id = Some(d.font_face_id);
        let name = d.family.as_str();
        charge(
            budget,
            ResourceCatalogRequest {
                work: amount(name.len(), 2, 64)?,
                ..point(S::FamilyValidate, id, None)
            },
        )?;
        if name.trim().is_empty() || name.chars().any(char::is_control) {
            return Err(ResourceAdmissionError::InvalidFontFamily.into());
        }
        let work = number(names.len())?
            .checked_add(1)
            .and_then(|n| number(name.len()).ok()?.checked_add(2)?.checked_mul(n))
            .and_then(|n| n.checked_add(1))
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        charge(
            budget,
            ResourceCatalogRequest {
                work,
                ..point(S::FamilyLookup, id, None)
            },
        )?;
        let at = match names.binary_search(&name) {
            Ok(_) => return Err(ResourceAdmissionError::InvalidFontFamily.into()),
            Err(at) => at,
        };
        charge(
            budget,
            ResourceCatalogRequest {
                work: amount(names.len() - at, std::mem::size_of::<&str>() as u64, 64)?,
                ..point(S::FamilyInsert, id, None)
            },
        )?;
        names.insert(at, name);
    }
    Ok(())
}

/// Prepare the same sealed catalog as [`staging_declared_base_catalog`], with
/// fallible arrays/strings and a borrowed family-validation index. Refusal
/// returns no catalog; it grants no host-open or resource-read capability.
/// Nonobserved callers keep the ordinary constructor and request sequence.
pub fn staging_declared_base_catalog_with_budget<B: ResourceCatalogBudget>(
    declarations: &StagingM4ResourceCatalog,
    budget: &mut B,
) -> Result<StagingDeclaredBaseCatalog, ResourceCatalogError<B::Error>> {
    if !B::OBSERVED {
        return staging_declared_base_catalog(declarations).map_err(Into::into);
    }
    use ResourceCatalogStage as S;
    charge(budget, point(S::Start, None, None))?;
    let mut fonts = rows(declarations.font_faces.len(), S::FontRows, budget)?;
    let mut font_media = rows(declarations.font_faces.len(), S::FontMedia, budget)?;
    for d in &declarations.font_faces {
        let id = Some(d.font_face_id);
        charge(budget, point(S::Font, id, None))?;
        let FontMediaDeclaration::Declared(media) = d.media else {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch.into());
        };
        let family = string(&d.family, point(S::FamilyCopy, id, None), budget)?;
        let uri = uri(&d.uri, point(S::UriCopy, id, None), budget)?;
        font_media.push(media);
        fonts.push(FontFaceDeclaration {
            font_face_id: d.font_face_id,
            family,
            uri,
            face_index: d.face_index,
            expected_sha256: d.expected_sha256,
        });
    }
    let mut images = rows(declarations.images.len(), S::ImageRows, budget)?;
    let mut image_media = rows(declarations.images.len(), S::ImageMedia, budget)?;
    for d in &declarations.images {
        let id = Some(d.image_id);
        charge(budget, point(S::Image, None, id))?;
        let ImageMediaDeclaration::Declared(media) = d.media else {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch.into());
        };
        if media == ImageMediaType::SvgSafe2 && d.expected_sha256.is_none() {
            return Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::HashMismatch,
            )
            .into());
        }
        let uri = uri(&d.uri, point(S::UriCopy, None, id), budget)?;
        image_media.push(media);
        images.push(ImageDeclaration {
            image_id: d.image_id,
            uri,
            expected_sha256: d.expected_sha256,
        });
    }
    let resource_catalog = ResourceCatalog {
        font_faces: fonts,
        images,
    };
    validate(&resource_catalog, budget)?;
    charge(budget, point(S::Finish, None, None))?;
    Ok(StagingDeclaredBaseCatalog {
        resource_catalog,
        font_media,
        image_media,
    })
}
