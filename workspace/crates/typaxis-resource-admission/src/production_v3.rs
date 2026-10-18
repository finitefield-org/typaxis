//! Private-staging resource-set /3 admission. No public contract dispatch and
//! no conversion to the legacy AdmittedResourceLedger are provided here.
use super::*;
#[path = "production_font_binding.rs"]
mod binding;
pub use binding::{ProductionFontBindingBudgetFailureV3, ProductionFontBindingBudgetResultV3, ProductionFontBindingCauseV3};
#[path = "resource_resolver_setup_budget.rs"]
mod resolver_setup;
use typaxis_font::{
    Cff1AdmissionV2, Cff1FailureV2, CffBaseCmapBudgetV2,
};
#[path = "production_font_instances_observed.rs"]
mod observed_instances;
pub use observed_instances::{ProductionFontInstanceBudgetV3, ProductionFontInstanceErrorV3};
#[path = "production_ledger_observed.rs"]
mod observed_ledger;
#[path = "production_ledger_aliases_observed.rs"]
mod observed_aliases;
#[path = "production_ledger_families_observed.rs"]
mod observed_families;
pub use observed_ledger::{ProductionResourceLedgerBudgetV3,ProductionResourceLedgerBudgetFailureV3,ProductionResourceLedgerErrorV3,ProductionResourceLedgerPointV3,ProductionResourceLedgerStageV3,ProductionResourceLedgerStorageV3};

pub const STAGING_PRODUCTION_RESOURCE_SET_V3: &str = "typaxis.production-book-resource-set/3";
#[derive(Debug)]
pub enum ProductionResourceErrorV3 {
    Resource(ResourceAdmissionError),
    Cff {
        font_face_id: FontFaceId,
        error: Cff1FailureV2,
    },
}
impl From<ResourceAdmissionError> for ProductionResourceErrorV3 {
    fn from(e: ResourceAdmissionError) -> Self {
        Self::Resource(e)
    }
}
impl std::fmt::Display for ProductionResourceErrorV3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "production resource /3 {self:?}")
    }
}
impl std::error::Error for ProductionResourceErrorV3 {}

#[derive(Debug)]
pub struct AdmittedCff1FontV2 {
    declaration: FontFaceDeclaration,
    admission: Cff1AdmissionV2,
}
impl AdmittedCff1FontV2 {
    pub fn declaration(&self) -> &FontFaceDeclaration {
        &self.declaration
    }
    pub fn admission(&self) -> &Cff1AdmissionV2 {
        &self.admission
    }
}
#[derive(Debug)]
pub enum AdmittedProductionFontV3 {
    TrueType(AdmittedFont),
    Cff1V2(AdmittedCff1FontV2),
}
impl AdmittedProductionFontV3 {
    pub fn font_face_id(&self) -> FontFaceId {
        match self {
            Self::TrueType(f) => f.font_face_id(),
            Self::Cff1V2(f) => f.declaration.font_face_id,
        }
    }
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::TrueType(f) => f.bytes(),
            Self::Cff1V2(f) => f.admission.source(),
        }
    }
    pub fn family(&self) -> &str {
        match self {
            Self::TrueType(f) => f.family(),
            Self::Cff1V2(f) => &f.declaration.family,
        }
    }
    pub fn uri(&self) -> &PortablePath {
        match self {
            Self::TrueType(f) => f.uri(),
            Self::Cff1V2(f) => &f.declaration.uri,
        }
    }
    pub fn face_index(&self) -> u32 {
        match self {
            Self::TrueType(f) => f.face_index(),
            Self::Cff1V2(f) => f.declaration.face_index,
        }
    }
    pub fn content_hash(&self) -> [u8; 32] {
        match self {
            Self::TrueType(f) => f.content_hash(),
            Self::Cff1V2(f) => f.admission.source_sha256(),
        }
    }
    pub fn media_kind(&self) -> AdmittedFontMediaKind {
        match self {
            Self::TrueType(f) => f.media_kind(),
            Self::Cff1V2(_) => AdmittedFontMediaKind::SfntCff1,
        }
    }
    pub fn metadata(&self) -> AdmittedFontMetadata {
        match self {
            Self::TrueType(f) => f.metadata().clone(),
            Self::Cff1V2(f) => AdmittedFontMetadata {
                units_per_em: f.admission.units_per_em(),
                glyph_count: u32::from(f.admission.glyph_count()),
            },
        }
    }
}

/// Owns the same host read session and aggregate byte/vector budgets as legacy
/// admission. CFF /2 results have a separate map and can only finish into /3.
pub struct StagingProductionResourceResolverV3<'roots> {
    inner: AdmittedResourceResolver<'roots>,
    cff_fonts: binding::Fonts,
    profile_fingerprint: [u8; 32],
    limits: M4EffectiveResourceLimits,
}
impl<'roots> StagingProductionResourceResolverV3<'roots> {
    /// Receives the private-staging preflight fingerprint. Public contract
    /// selection must be checked by the upstream production-book-2 owner;
    /// this constructor does not authorize public profile registration.
    pub fn new(
        declarations: &StagingDeclaredBaseCatalog,
        limits: &M4EffectiveResourceLimits,
        profile_fingerprint: [u8; 32],
        roots: HostRootSetToken<'roots>,
    ) -> Result<Self, ProductionResourceErrorV3> {
        Ok(Self {
            inner: AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
                declarations,
                limits,
                profile_fingerprint,
                roots,
            )?,
            cff_fonts: binding::Fonts::new(),
            profile_fingerprint,
            limits: limits.clone(),
        })
    }
    pub fn read_font(
        &mut self,
        source: VerifiedResourceSource<'roots>,
    ) -> Result<PendingResourceBytes, ProductionResourceErrorV3> {
        Ok(self.inner.read_font(source)?)
    }
    pub fn read_font_with_budget<B: HostReadBudget>(
        &mut self,
        source: VerifiedResourceSource<'roots>,
        budget: &mut B,
    ) -> Result<PendingResourceBytes, ResourceReadError<B::Error>> {
        self.inner.read_font_with_budget(source, budget)
    }
    pub fn read_image(
        &mut self,
        source: VerifiedResourceSource<'roots>,
    ) -> Result<PendingResourceBytes, ProductionResourceErrorV3> {
        Ok(self.inner.read_image(source)?)
    }
    pub fn read_image_with_budget<B: HostReadBudget>(
        &mut self,
        source: VerifiedResourceSource<'roots>,
        budget: &mut B,
    ) -> Result<PendingResourceBytes, ResourceReadError<B::Error>> {
        self.inner.read_image_with_budget(source, budget)
    }
    pub fn parse_and_bind_image(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ProductionResourceErrorV3> {
        Ok(self.inner.parse_and_bind_declared_image(source)?)
    }
    pub fn parse_and_bind_image_with_source_budget<B:SafeSvgSourceBudget>(
        &mut self,source:PendingResourceBytes,budget:&mut B,
    )->Result<(),SafeSvgSourceError<B::Error>>{
        self.inner.parse_and_bind_declared_image_with_source_budget(source,budget)
    }
    pub fn parse_and_bind_font(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ProductionResourceErrorV3> {
        match self.parse_and_bind_font_with_base_cmap_budget(
            source,
            &mut base_cmap_budget::UnobservedBaseCmap,
        ) {
            Ok(value) => value,
            Err(failure) => match failure.failure.error {},
        }
    }
    /// Observe base-cmap requests only, after the ordinary host/session/hash checks.
    /// No callback is kept and no partially admitted font is published on refusal.
    pub fn parse_and_bind_font_with_base_cmap_budget<B: CffBaseCmapBudgetV2>(
        &mut self,
        source: PendingResourceBytes,
        budget: &mut B,
    ) -> ProductionBaseCmapBudgetResultV3<(), B::Error> {
        self.parse_and_bind_font_with_cmap_policy(source, budget, None, |bytes, face, _| Ok(font_media_budget::declared_native(bytes, face)), |source, face, limits, budget| {
            typaxis_font::admit_sfnt_cff1_v2_owned_with_base_cmap_budget(source, face, limits, budget)
        })
        .map_err(
            |(font_face_id, failure)| ProductionBaseCmapBudgetFailureV3 {
                font_face_id,
                failure,
            },
        )
    }
    /// Observe base and variation cmap requests after authentic host checks.
    pub fn parse_and_bind_font_with_cmap_budget<B: typaxis_font::CffCmapBudgetV2>(
        &mut self,
        source: PendingResourceBytes,
        budget: &mut B,
    ) -> ProductionCmapBudgetResultV3<(), B::Error> {
        self.parse_and_bind_font_with_cmap_policy(source, budget, None, |bytes, face, _| Ok(font_media_budget::declared_native(bytes, face)), |source, face, limits, budget| {
            typaxis_font::admit_sfnt_cff1_v2_owned_with_cmap_budget(source, face, limits, budget)
        })
        .map_err(|(font_face_id, failure)| ProductionCmapBudgetFailureV3 {
            font_face_id,
            failure,
        })
    }
    /// Observe SFNT preflight and cmap under the same authentic declaration.
    pub fn parse_and_bind_font_with_admission_budget<B: typaxis_font::CffAdmissionBudgetV2>(
        &mut self,
        source: PendingResourceBytes,
        budget: &mut B,
    ) -> ProductionAdmissionBudgetResultV3<(), B::Error> {
        self.parse_and_bind_font_with_cmap_policy(source, budget, None, font_media_budget::declared_with_budget, |source, face, limits, budget| {
            typaxis_font::admit_sfnt_cff1_v2_owned_with_admission_budget(source, face, limits, budget)
        })
        .map_err(
            |(font_face_id, failure)| ProductionAdmissionBudgetFailureV3 {
                font_face_id,
                failure,
            },
        )
    }
    /// Observe declaration checks/copies and fallible binding storage as well
    /// as CFF admission. Authentic session checks precede every callback.
    /// Refusal publishes no font; accepted storage can be reused on retry.
    pub fn parse_and_bind_font_with_binding_budget<B>(
        &mut self, source: PendingResourceBytes, budget: &mut B,
    ) -> ProductionFontBindingBudgetResultV3<(), <B as CffBaseCmapBudgetV2>::Error>
    where B: typaxis_font::CffAdmissionBudgetV2 + ResourceCatalogBudget<Error = <B as CffBaseCmapBudgetV2>::Error>,
    {
        let permit = if <B as ResourceCatalogBudget>::OBSERVED { Some(binding::permit::<B> as binding::Permit<_, _>) } else { None };
        self.parse_and_bind_font_with_cmap_policy(source, budget, permit,
            |bytes, face, budget| font_media_budget::declared_with_budget(bytes, face, budget).map_err(ProductionFontBindingCauseV3::Admission),
            |source, face, limits, budget| typaxis_font::admit_sfnt_cff1_v2_owned_with_admission_budget(source, face, limits, budget).map_err(ProductionFontBindingCauseV3::Admission),
        ).map_err(|(font_face_id, failure)| ProductionFontBindingBudgetFailureV3 { font_face_id, failure })
    }
    fn parse_and_bind_font_with_cmap_policy<C, E>(
        &mut self,
        source: PendingResourceBytes,
        budget: &mut C,
        permit: Option<binding::Permit<C, E>>,
        attest: impl FnOnce(&[u8], u32, &mut C) -> Result<Result<AdmittedFontMediaKind, ResourceAdmissionError>, E>,
        admit: impl FnOnce(
            Vec<u8>,
            u32,
            &M4EffectiveResourceLimits,
            &mut C,
        ) -> FontCmapAdmissionResult<E>,
    ) -> Result<Result<(), ProductionResourceErrorV3>, (FontFaceId, E)> {
        macro_rules! native {
            ($value:expr) => {
                match $value {
                    Ok(value) => value,
                    Err(failure) => return Ok(Err(failure.into())),
                }
            };
        }
        native!(self.inner.ensure_session(&source));
        let id = native!(source
            .font_face_id()
            .ok_or(ResourceAdmissionError::ReceiptKindMismatch));
        let declaration = native!(self
            .inner
            .declarations
            .font_faces
            .get(id.get() as usize)
            .filter(|d| d.font_face_id == id)
            .ok_or(ResourceAdmissionError::MissingLogicalResource));
        macro_rules! bind {
            ($value:expr) => {
                match $value {
                    Ok(value) => value,
                    Err(ResourceCatalogError::Native(error)) => return Ok(Err(error.into())),
                    Err(ResourceCatalogError::Budget(failure)) => return Err((id, failure.error)),
                }
            };
        }
        // Peek without changing the native failure order. Only the declared
        // CFF path opts into this binding policy; other media retain theirs.
        let cff = self.inner.declared_media_policy.as_ref()
            .and_then(|p| p.fonts.get(id.get() as usize)) == Some(&FontMediaType::SfntCff1);
        if let Some(permit) = permit.filter(|_| cff) {
            let existing = native!(self.cff_fonts.len().checked_add(self.inner.fonts.len()).ok_or(ResourceAdmissionError::ResourceLimit));
            bind!(binding::checks(declaration, &source, existing, &mut binding::Budget { budget, permit, id }));
        }
        if source.uri() != &declaration.uri || source.face_index() != Some(declaration.face_index) {
            return Ok(Err(ResourceAdmissionError::ReceiptIdentityMismatch.into()));
        }
        if declaration
            .expected_sha256
            .is_some_and(|h| h != source.content_hash())
        {
            return Ok(Err(ResourceAdmissionError::ExpectedHashMismatch.into()));
        }
        if self.cff_fonts.contains_key(&id) || self.inner.fonts.contains_key(&id) {
            return Ok(Err(
                ResourceAdmissionError::ConflictingLogicalResource.into()
            ));
        }
        let media = native!(self
            .inner
            .declared_media_policy
            .as_ref()
            .and_then(|p| p.fonts.get(id.get() as usize))
            .ok_or(ResourceAdmissionError::DeclaredMediaMismatch));
        if *media != FontMediaType::SfntCff1 {
            return Ok(self
                .inner
                .parse_and_bind_declared_sfnt(source)
                .map_err(Into::into));
        }
        let observed = native!(match attest(source.bytes(), declaration.face_index, budget) {
            Ok(value) => value,
            Err(failure) => return Err((id, failure)),
        });
        if observed != AdmittedFontMediaKind::SfntCff1 {
            return Ok(Err(ResourceAdmissionError::DeclaredMediaMismatch.into()));
        }
        let (declaration, position) = if let Some(permit) = permit {
            let mut observed = binding::Budget { budget, permit, id };
            let position = bind!(self.cff_fonts.prepare(self.inner.declarations.font_faces.len(), id, &mut observed));
            (bind!(binding::declaration(declaration, &mut observed)), Some(position))
        } else { (declaration.clone(), None) };
        let source_hash = source.content_hash();
        let admission = match admit(source.bytes, declaration.face_index, &self.limits, budget) {
            Ok(value) => native!(value.map_err(|error| ProductionResourceErrorV3::Cff {
                font_face_id: id,
                error
            })),
            Err(failure) => return Err((id, failure)),
        };
        if admission.source_sha256() != source_hash {
            return Ok(Err(ResourceAdmissionError::ReceiptIdentityMismatch.into()));
        }
        native!(self.cff_fonts.insert(
            id,
            AdmittedCff1FontV2 {
                declaration,
                admission,
            },
            position,
        ));
        Ok(Ok(()))
    }

    pub fn finish(self) -> Result<AdmittedProductionResourceLedgerV3, ProductionResourceErrorV3> {
        match self.finish_with_budget(&mut observed_ledger::Unobserved) {
            Ok(value)=>Ok(value),
            Err(ProductionResourceLedgerErrorV3::Native(error))=>Err(error),
            Err(ProductionResourceLedgerErrorV3::Budget(failure))=>match failure.error {},
        }
    }
    /// Observe alias and family indices, dense output buffers, and canonical
    /// encoding/digest work. Earlier admission retains its separate policy.
    pub fn finish_with_budget<B:ProductionResourceLedgerBudgetV3>(self,budget:&mut B)->Result<AdmittedProductionResourceLedgerV3,ProductionResourceLedgerErrorV3<B::Error>>{
        let inner = self.inner;
        if inner.fonts.len().checked_add(self.cff_fonts.len())
            != Some(inner.declarations.font_faces.len())
            || inner.images.len() != inner.declarations.images.len()
        {
            return Err(ResourceAdmissionError::MissingLogicalResource.into());
        }
        for (i,v) in inner.images.values().enumerate() {
            observed_ledger::work(budget,observed_ledger::point(ProductionResourceLedgerStageV3::ImageIds,None,Some(v.image_id())),64)?;
            if v.image_id().get()as usize!=i{return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());}
        }
        observed_aliases::validate_images(&inner.images,budget)?;
        let families=observed_families::families(&inner.declarations.font_faces,budget)?;
        let fonts=observed_ledger::fonts(inner.fonts,self.cff_fonts,budget)?;
        let images=observed_ledger::images(inner.images,budget)?;
        let mut ledger = AdmittedProductionResourceLedgerV3 {
            session: inner.session,
            fonts,
            images,
            families,
            profile_fingerprint: self.profile_fingerprint,
            limits: self.limits,
            fingerprint: [0; 32],
            canonical_jcs: String::new(),
        };
        ledger.canonical_jcs = encode(&ledger,budget)?;
        ledger.fingerprint = observed_ledger::digest(ledger.canonical_jcs.as_bytes(),budget)?;
        Ok(ledger)
    }
}
/// Comparison-only identity of one authoritative resource-admission session.
/// Retaining this token keeps the identity allocation alive; copying it neither
/// admits resources nor constructs a resource ledger.
///
/// ```compile_fail
/// use typaxis_resource_admission::ProductionResourceAdmissionSessionIdentityV3;
/// let forged = ProductionResourceAdmissionSessionIdentityV3::default();
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionResourceAdmissionSessionIdentityV3(ResourceAdmissionSessionIdentity);

#[cfg(test)]
mod command_identity_tests {
    use super::*;

    #[test]
    fn production_v3_command_identity_compares_authoritative_allocation() {
        let original = ResourceAdmissionSessionIdentity::fresh();
        let identity = ProductionResourceAdmissionSessionIdentityV3(original.clone());
        assert_eq!(identity, identity.clone());
        assert_eq!(
            identity,
            ProductionResourceAdmissionSessionIdentityV3(original)
        );
        assert_ne!(
            identity,
            ProductionResourceAdmissionSessionIdentityV3(ResourceAdmissionSessionIdentity::fresh())
        );
    }

    #[test]
    fn production_v3_command_identity_retains_allocation_after_original_drops() {
        let original = ResourceAdmissionSessionIdentity::fresh();
        let weak = Arc::downgrade(&original.0);
        let identity = ProductionResourceAdmissionSessionIdentityV3(original);
        assert!(weak.upgrade().is_some());
        let copied = identity.clone();
        drop(identity);
        assert!(weak.upgrade().is_some());
        drop(copied);
        assert!(weak.upgrade().is_none());
    }
}

#[derive(Debug)]
pub struct AdmittedProductionResourceLedgerV3 {
    session: ResourceAdmissionSessionIdentity,
    fonts: Vec<AdmittedProductionFontV3>,
    images: Vec<AdmittedImage>,
    families: FontFamilyTable,
    profile_fingerprint: [u8; 32],
    limits: M4EffectiveResourceLimits,
    fingerprint: [u8; 32],
    canonical_jcs: String,
}
impl AdmittedProductionResourceLedgerV3 {
    pub fn matches_declared_resources(&self, declarations: &StagingM4ResourceCatalog) -> bool {
        self.fonts.len() == declarations.font_faces.len()
            && self.images.len() == declarations.images.len()
            && self
                .fonts
                .iter()
                .zip(&declarations.font_faces)
                .all(|(font, declaration)| {
                    matches!(declaration.media, FontMediaDeclaration::Declared(media) if media.as_str() == font.media_kind().as_str())
                        && font.font_face_id() == declaration.font_face_id
                        && font.uri() == &declaration.uri
                        && font.family() == declaration.family
                        && font.face_index() == declaration.face_index
                        && declaration
                            .expected_sha256
                            .map_or(true, |expected| expected == font.content_hash())
                })
            && self
                .images
                .iter()
                .zip(&declarations.images)
                .all(|(image, declaration)| {
                    matches!(declaration.media, ImageMediaDeclaration::Declared(media) if media.as_str() == image.media_kind().as_str())
                        && image.image_id() == declaration.image_id
                        && image.uri() == &declaration.uri
                        && declaration
                            .expected_sha256
                            .map_or(true, |expected| expected == image.content_hash())
                })
    }
    pub fn same_session_as(&self, other: &Self) -> bool {
        self.session == other.session
    }
    pub fn session_identity(&self) -> ProductionResourceAdmissionSessionIdentityV3 {
        ProductionResourceAdmissionSessionIdentityV3(self.session.clone())
    }
    pub fn fonts(&self) -> &[AdmittedProductionFontV3] {
        &self.fonts
    }
    pub fn images(&self) -> &[AdmittedImage] {
        &self.images
    }
    pub fn font(&self, id: FontFaceId) -> Option<&AdmittedProductionFontV3> {
        self.fonts.get(id.get() as usize)
    }
    pub fn image(&self, id: ImageResourceId) -> Option<&AdmittedImage> {
        self.images.get(id.get() as usize)
    }
    pub fn font_families(&self) -> &FontFamilyTable {
        &self.families
    }
    pub fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
    pub fn effective_limits(&self) -> &M4EffectiveResourceLimits {
        &self.limits
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub fn resource_set_id(&self) -> &'static str {
        STAGING_PRODUCTION_RESOURCE_SET_V3
    }
}
fn encode<B:ProductionResourceLedgerBudgetV3>(
    ledger: &AdmittedProductionResourceLedgerV3,
    budget:&mut B,
) -> Result<String, ProductionResourceLedgerErrorV3<B::Error>> {
    use std::fmt::Write;
    let mut capacity = 512usize;
    for f in &ledger.fonts {
        observed_ledger::work(budget,observed_ledger::point(ProductionResourceLedgerStageV3::CanonicalCapacity,Some(f.font_face_id()),None),64)?;
        capacity = capacity
            .checked_add(512)
            .and_then(|n| {
                f.family()
                    .len()
                    .checked_mul(6)
                    .and_then(|v| n.checked_add(v))
            })
            .and_then(|n| {
                f.uri()
                    .as_str()
                    .len()
                    .checked_mul(6)
                    .and_then(|v| n.checked_add(v))
            })
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
    }
    for image in &ledger.images {
        observed_ledger::work(budget,observed_ledger::point(ProductionResourceLedgerStageV3::CanonicalCapacity,None,Some(image.image_id())),64)?;
        capacity = capacity
            .checked_add(512)
            .and_then(|n| {
                image
                    .uri()
                    .as_str()
                    .len()
                    .checked_mul(6)
                    .and_then(|v| n.checked_add(v))
            })
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
    }
    if capacity as u64 > ledger.limits.base().get().max_spool_bytes {
        return Err(ResourceAdmissionError::ResourceLimit.into());
    }
    let mut s = String::new();
    observed_ledger::reserve(budget,ProductionResourceLedgerStorageV3::CanonicalJcs,1,capacity,observed_ledger::point(ProductionResourceLedgerStageV3::CanonicalCapacity,None,None))?;
    s.try_reserve_exact(capacity)
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    s.push_str("{\"algorithm\":\"typaxis.admitted-production-resources/3\",\"fonts\":[");
    for (i, font) in ledger.fonts.iter().enumerate() {
        let work=font.family().len().checked_add(font.uri().as_str().len()).and_then(|n|n.checked_mul(6)).and_then(|n|n.checked_add(512)).ok_or(ResourceAdmissionError::ResourceLimit)?;
        observed_ledger::work(budget,observed_ledger::point(ProductionResourceLedgerStageV3::CanonicalFonts,Some(font.font_face_id()),None),work as u64)?;
        if i != 0 {
            s.push(',');
        }
        s.push('{');
        match font {
            AdmittedProductionFontV3::Cff1V2(f) => {
                s.push_str("\"admission_fingerprint\":");
                push_hash_hex(&mut s, f.admission.fingerprint());
                s.push(',');
            }
            AdmittedProductionFontV3::TrueType(_) => {}
        }
        s.push_str("\"face_index\":");
        write!(&mut s,"{}",font.face_index()).expect("String formatting");
        s.push_str(",\"family\":");
        push_jcs_string(&mut s, font.family());
        s.push_str(",\"font_face_id\":");
        write!(&mut s,"{}",font.font_face_id().get()).expect("String formatting");
        s.push_str(",\"media\":");
        push_jcs_string(&mut s, font.media_kind().as_str());
        s.push_str(",\"sha256\":");
        push_hash_hex(&mut s, font.content_hash());
        s.push_str(",\"uri\":");
        push_jcs_string(&mut s, font.uri().as_str());
        s.push('}');
    }
    s.push_str("],\"images\":[");
    for (i, image) in ledger.images.iter().enumerate() {
        let work=image.uri().as_str().len().checked_mul(6).and_then(|n|n.checked_add(512)).ok_or(ResourceAdmissionError::ResourceLimit)?;
        observed_ledger::work(budget,observed_ledger::point(ProductionResourceLedgerStageV3::CanonicalImages,None,Some(image.image_id())),work as u64)?;
        if i != 0 {
            s.push(',');
        }
        s.push_str("{\"image_id\":");
        write!(&mut s,"{}",image.image_id().get()).expect("String formatting");
        s.push_str(",\"media\":");
        push_jcs_string(&mut s, image.media_kind().as_str());
        s.push_str(",\"sha256\":");
        push_hash_hex(&mut s, image.content_hash());
        s.push_str(",\"uri\":");
        push_jcs_string(&mut s, image.uri().as_str());
        s.push('}');
    }
    s.push_str("],\"limits_fingerprint\":");
    push_hash_hex(&mut s, ledger.limits.fingerprint());
    s.push_str(",\"profile_fingerprint\":");
    push_hash_hex(&mut s, ledger.profile_fingerprint);
    s.push_str(",\"resource_set\":");
    push_jcs_string(&mut s, STAGING_PRODUCTION_RESOURCE_SET_V3);
    s.push('}');
    debug_assert!(s.len() <= capacity);
    Ok(s)
}

/// Dense instances cannot be rebound to a different ledger: this table borrows
/// its actual /3 ledger for its entire lifetime and resolves only into it.
#[derive(Debug)]
pub struct AdmittedProductionFontInstancesV3<'a> {
    ledger: &'a AdmittedProductionResourceLedgerV3,
    faces: Vec<FontFaceId>,
    fingerprint: [u8; 32],
}
impl<'a> AdmittedProductionFontInstancesV3<'a> {
    pub fn from_used_faces(
        ledger: &'a AdmittedProductionResourceLedgerV3,
        used: impl IntoIterator<Item = FontFaceId>,
    ) -> Result<Self, ProductionResourceErrorV3> {
        let mut faces = BTreeSet::new();
        let mut count = 0u64;
        for id in used {
            count = count
                .checked_add(1)
                .filter(|n| *n <= ledger.effective_limits().base().get().max_fragments)
                .ok_or(ResourceAdmissionError::ResourceLimit)?;
            if ledger.font(id).is_none() {
                return Err(ResourceAdmissionError::MissingLogicalResource.into());
            }
            faces.insert(id);
        }
        let capacity = faces
            .len()
            .checked_mul(11)
            .and_then(|n| n.checked_add(256))
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        if capacity as u64 > ledger.effective_limits().base().get().max_spool_bytes {
            return Err(ResourceAdmissionError::ResourceLimit.into());
        }
        let mut ordered = Vec::new();
        ordered
            .try_reserve_exact(faces.len())
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        ordered.extend(faces);
        let faces = ordered;
        let mut canonical = String::new();
        canonical
            .try_reserve_exact(capacity)
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        canonical.push_str("{\"algorithm\":\"typaxis.production-font-instances/3\",\"faces\":[");
        for (i, face) in faces.iter().enumerate() {
            if i != 0 {
                canonical.push(',');
            }
            canonical.push_str(&face.get().to_string());
        }
        canonical.push_str("],\"ledger_fingerprint\":");
        push_hash_hex(&mut canonical, ledger.fingerprint());
        canonical.push('}');
        Ok(Self {
            ledger,
            faces,
            fingerprint: sha256(canonical.as_bytes()),
        })
    }
    pub fn len(&self) -> usize {
        self.faces.len()
    }
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }
    pub fn ledger_fingerprint(&self) -> [u8; 32] {
        self.ledger.fingerprint()
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn resolve(
        &self,
        id: typaxis_core::FontInstanceId,
    ) -> Option<AdmittedProductionFontInstanceV3<'a>> {
        let face = *self.faces.get(id.get() as usize)?;
        Some(AdmittedProductionFontInstanceV3 {
            instance: id,
            table_fingerprint: self.fingerprint,
            font: self.ledger.font(face)?,
            ledger: self.ledger,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct AdmittedProductionFontInstanceV3<'a> {
    instance: typaxis_core::FontInstanceId,
    table_fingerprint: [u8; 32],
    font: &'a AdmittedProductionFontV3,
    ledger: &'a AdmittedProductionResourceLedgerV3,
}
impl<'a> AdmittedProductionFontInstanceV3<'a> {
    pub fn table_fingerprint(self) -> [u8; 32] {
        self.table_fingerprint
    }
    pub fn font_instance_id(self) -> typaxis_core::FontInstanceId {
        self.instance
    }
    pub fn font(self) -> &'a AdmittedProductionFontV3 {
        self.font
    }
    pub fn ledger(self) -> &'a AdmittedProductionResourceLedgerV3 {
        self.ledger
    }
}

#[path = "production_base_cmap_budget.rs"]
mod base_cmap_budget;
pub use base_cmap_budget::{ProductionBaseCmapBudgetFailureV3, ProductionBaseCmapBudgetResultV3};

#[path = "production_cmap_budget.rs"]
mod cmap_budget;
pub use cmap_budget::{ProductionCmapBudgetFailureV3, ProductionCmapBudgetResultV3};
type FontCmapAdmissionResult<E> = Result<Result<Cff1AdmissionV2, Cff1FailureV2>, E>;

#[path="production_admission_budget.rs"]
mod admission_budget;
pub use admission_budget::{ProductionAdmissionBudgetFailureV3, ProductionAdmissionBudgetResultV3};
