//! Private-staging resource-set /3 admission. No public contract dispatch and
//! no conversion to the legacy AdmittedResourceLedger are provided here.
use super::*;
use typaxis_font::{admit_sfnt_cff1_v2, Cff1AdmissionV2, Cff1FailureV2};

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
    cff_fonts: BTreeMap<FontFaceId, AdmittedCff1FontV2>,
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
            cff_fonts: BTreeMap::new(),
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
    pub fn read_image(
        &mut self,
        source: VerifiedResourceSource<'roots>,
    ) -> Result<PendingResourceBytes, ProductionResourceErrorV3> {
        Ok(self.inner.read_image(source)?)
    }
    pub fn parse_and_bind_image(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ProductionResourceErrorV3> {
        Ok(self.inner.parse_and_bind_declared_image(source)?)
    }
    pub fn parse_and_bind_font(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ProductionResourceErrorV3> {
        self.inner.ensure_session(&source)?;
        let id = source
            .font_face_id()
            .ok_or(ResourceAdmissionError::ReceiptKindMismatch)?;
        let declaration = self
            .inner
            .declarations
            .font_faces
            .get(id.get() as usize)
            .filter(|d| d.font_face_id == id)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        if source.uri() != &declaration.uri || source.face_index() != Some(declaration.face_index) {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
        }
        if declaration
            .expected_sha256
            .is_some_and(|h| h != source.content_hash())
        {
            return Err(ResourceAdmissionError::ExpectedHashMismatch.into());
        }
        if self.cff_fonts.contains_key(&id) || self.inner.fonts.contains_key(&id) {
            return Err(ResourceAdmissionError::ConflictingLogicalResource.into());
        }
        let media = self
            .inner
            .declared_media_policy
            .as_ref()
            .and_then(|p| p.fonts.get(id.get() as usize))
            .ok_or(ResourceAdmissionError::DeclaredMediaMismatch)?;
        if *media != FontMediaType::SfntCff1 {
            return Ok(self.inner.parse_and_bind_declared_sfnt(source)?);
        }
        let observed = attest_declared_font_media_kind(source.bytes(), declaration.face_index)
            .map_err(|_| {
                ResourceAdmissionError::FontContainerDetailed(
                    font_diagnostic::diagnose_font_container(
                        source.bytes(),
                        declaration.face_index,
                    ),
                )
            })?;
        if observed != AdmittedFontMediaKind::SfntCff1 {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch.into());
        }
        let declaration = declaration.clone();
        let source_hash = source.content_hash();
        let admission =
            admit_sfnt_cff1_v2(source.bytes.into(), declaration.face_index, &self.limits).map_err(
                |error| ProductionResourceErrorV3::Cff {
                    font_face_id: id,
                    error,
                },
            )?;
        if admission.source_sha256() != source_hash {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
        }
        self.cff_fonts.insert(
            id,
            AdmittedCff1FontV2 {
                declaration,
                admission,
            },
        );
        Ok(())
    }
    pub fn finish(self) -> Result<AdmittedProductionResourceLedgerV3, ProductionResourceErrorV3> {
        let inner = self.inner;
        if inner.fonts.len().checked_add(self.cff_fonts.len())
            != Some(inner.declarations.font_faces.len())
            || inner.images.len() != inner.declarations.images.len()
        {
            return Err(ResourceAdmissionError::MissingLogicalResource.into());
        }
        if inner
            .images
            .values()
            .enumerate()
            .any(|(i, v)| v.image_id().get() as usize != i)
        {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
        }
        let mut aliases = Vec::new();
        aliases
            .try_reserve_exact(inner.images.len())
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        for image in inner.images.values() {
            if image.admitted_safe_vector().is_some() {
                aliases.push((image.content_hash(), image.bytes()));
            }
        }
        validate_safe_vector_digest_aliases(&aliases)?;
        drop(aliases);
        let families = FontFamilyTable::new(
            inner
                .declarations
                .font_faces
                .iter()
                .map(|d| (d.family.clone(), d.font_face_id))
                .collect(),
        )
        .map_err(map_font_family_error)?;
        let mut fonts = BTreeMap::new();
        for (id, font) in inner.fonts {
            if font.media_kind() == AdmittedFontMediaKind::SfntCff1
                || font.cff1_admission().is_some()
            {
                return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
            }
            fonts.insert(id, AdmittedProductionFontV3::TrueType(font));
        }
        for (id, font) in self.cff_fonts {
            if fonts
                .insert(id, AdmittedProductionFontV3::Cff1V2(font))
                .is_some()
            {
                return Err(ResourceAdmissionError::ConflictingLogicalResource.into());
            }
        }
        if fonts
            .values()
            .enumerate()
            .any(|(i, f)| f.font_face_id().get() as usize != i)
        {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
        }
        let mut ledger = AdmittedProductionResourceLedgerV3 {
            session: inner.session,
            fonts: fonts.into_values().collect(),
            images: inner.images.into_values().collect(),
            families,
            profile_fingerprint: self.profile_fingerprint,
            limits: self.limits,
            fingerprint: [0; 32],
            canonical_jcs: String::new(),
        };
        ledger.canonical_jcs = encode(&ledger)?;
        ledger.fingerprint = sha256(ledger.canonical_jcs.as_bytes());
        Ok(ledger)
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
    pub fn same_session_as(&self, other: &Self) -> bool {
        self.session == other.session
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
fn encode(
    ledger: &AdmittedProductionResourceLedgerV3,
) -> Result<String, ProductionResourceErrorV3> {
    let mut capacity = 512usize;
    for f in &ledger.fonts {
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
    s.try_reserve_exact(capacity)
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    s.push_str("{\"algorithm\":\"typaxis.admitted-production-resources/3\",\"fonts\":[");
    for (i, font) in ledger.fonts.iter().enumerate() {
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
        s.push_str(&font.face_index().to_string());
        s.push_str(",\"family\":");
        push_jcs_string(&mut s, font.family());
        s.push_str(",\"font_face_id\":");
        s.push_str(&font.font_face_id().get().to_string());
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
        if i != 0 {
            s.push(',');
        }
        s.push_str("{\"image_id\":");
        s.push_str(&image.image_id().get().to_string());
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
}
impl<'a> AdmittedProductionFontInstancesV3<'a> {
    pub fn from_used_faces(
        ledger: &'a AdmittedProductionResourceLedgerV3,
        used: impl IntoIterator<Item = FontFaceId>,
    ) -> Result<Self, ProductionResourceErrorV3> {
        let mut faces = BTreeSet::new();
        for id in used {
            if ledger.font(id).is_none() {
                return Err(ResourceAdmissionError::MissingLogicalResource.into());
            }
            faces.insert(id);
        }
        Ok(Self {
            ledger,
            faces: faces.into_iter().collect(),
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
    pub fn resolve(
        &self,
        id: typaxis_core::FontInstanceId,
    ) -> Option<AdmittedProductionFontInstanceV3<'a>> {
        let face = *self.faces.get(id.get() as usize)?;
        Some(AdmittedProductionFontInstanceV3 {
            instance: id,
            font: self.ledger.font(face)?,
            ledger: self.ledger,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct AdmittedProductionFontInstanceV3<'a> {
    instance: typaxis_core::FontInstanceId,
    font: &'a AdmittedProductionFontV3,
    ledger: &'a AdmittedProductionResourceLedgerV3,
}
impl<'a> AdmittedProductionFontInstanceV3<'a> {
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
