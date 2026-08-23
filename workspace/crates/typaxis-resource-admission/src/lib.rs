#![forbid(unsafe_code)]

use core::num::NonZeroU32;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::sync::Arc;
use typaxis_core::{
    admitted_resource_fingerprint_from_jcs, push_jcs_string, AdmittedResourceFingerprint,
    FontFaceId, ImageResourceId, PortablePath, ValidatedResourceLimits,
};
use typaxis_document::ResourceCatalog;
use typaxis_font::{FontFamilyError, FontFamilyTable};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedFontMetadata {
    pub units_per_em: u16,
    pub glyph_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedFont {
    font_face_id: FontFaceId,
    uri: PortablePath,
    family: String,
    face_index: u32,
    bytes: Vec<u8>,
    sha256: [u8; 32],
    metadata: AdmittedFontMetadata,
}
impl AdmittedFont {
    fn from_verified(
        font_face_id: FontFaceId,
        uri: PortablePath,
        family: String,
        face_index: u32,
        bytes: Vec<u8>,
        sha256: [u8; 32],
        metadata: AdmittedFontMetadata,
    ) -> Self {
        Self {
            font_face_id,
            uri,
            family,
            face_index,
            bytes,
            sha256,
            metadata,
        }
    }
    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub fn family(&self) -> &str {
        &self.family
    }
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn metadata(&self) -> &AdmittedFontMetadata {
        &self.metadata
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedImage {
    image_id: ImageResourceId,
    uri: PortablePath,
    bytes: Vec<u8>,
    sha256: [u8; 32],
    width: NonZeroU32,
    height: NonZeroU32,
    decoded_bytes: u64,
}
impl AdmittedImage {
    fn from_verified(
        image_id: ImageResourceId,
        uri: PortablePath,
        bytes: Vec<u8>,
        sha256: [u8; 32],
        width: NonZeroU32,
        height: NonZeroU32,
        decoded_bytes: u64,
    ) -> Self {
        Self {
            image_id,
            uri,
            bytes,
            sha256,
            width,
            height,
            decoded_bytes,
        }
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn width(&self) -> NonZeroU32 {
        self.width
    }
    pub const fn height(&self) -> NonZeroU32 {
        self.height
    }
    pub const fn decoded_bytes(&self) -> u64 {
        self.decoded_bytes
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceAdmissionError {
    MissingLogicalResource,
    ConflictingLogicalResource,
    ResourceLimit,
    ExpectedHashMismatch,
    InvalidMetadata,
    InvalidFontFamily,
    NonCanonicalResourceId,
    ResourceRead,
    ResourceLengthMismatch,
    ReceiptKindMismatch,
    ReceiptIdentityMismatch,
    ReceiptSessionMismatch,
    MissingAdmittedRootSet,
    RootSetMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingResourceId {
    Font(FontFaceId),
    Image(ImageResourceId),
}

/// An opened, contained regular-file handle bound to the length observed by
/// the crate-owned opener. Callers can transport this value but cannot wrap an
/// arbitrary `Read` or choose the trusted extent themselves.
pub struct VerifiedResourceSource<'roots, R> {
    roots: &'roots AdmittedRootSet,
    id: PendingResourceId,
    exact_length: u64,
    reader: R,
}

/// Read handle whose owner can re-check that the same immutable snapshot still
/// has the admitted extent after the final chunk. Implementing this trait does
/// not grant the capability to construct `VerifiedResourceSource`.
pub trait ResourceExtentReader: Read {
    fn current_length(&self) -> Result<u64, ResourceAdmissionError>;
}

/// Opaque root-set capability issued from one configured host-admission
/// session. There is deliberately no constructor from raw host paths.
#[derive(Debug, Eq, PartialEq)]
pub struct AdmittedRootSet {
    _identity: Box<u8>,
}
impl AdmittedRootSet {
    pub const fn token(&self) -> AdmittedRootSetToken<'_> {
        AdmittedRootSetToken { roots: self }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AdmittedRootSetToken<'roots> {
    roots: &'roots AdmittedRootSet,
}

/// Capability reserved for the platform contained-open implementation. It
/// must resolve the EffectiveConfig/CLI roots, reject aliased roots, and make
/// every subsequent open relative to the issued root handles.
#[derive(Debug)]
pub struct AdmittedRootSetOwner {
    _private: (),
}
impl AdmittedRootSetOwner {
    #[allow(dead_code)] // reserved for the platform host-admission owner
    fn new() -> Self {
        Self { _private: () }
    }
    #[allow(dead_code)] // issued only after canonical root admission succeeds
    fn issue(&self) -> AdmittedRootSet {
        AdmittedRootSet {
            _identity: Box::new(0),
        }
    }
}

/// Capability reserved for the contained resource opener/stat owner. A future
/// filesystem implementation creates this owner only from one admitted root
/// set and only after no-follow containment and regular-file checks have
/// succeeded on the same open handle.
#[derive(Debug)]
pub struct VerifiedResourceSourceOwner<'roots> {
    roots: &'roots AdmittedRootSet,
}
impl<'roots> VerifiedResourceSourceOwner<'roots> {
    #[allow(dead_code)] // reserved for the in-crate contained file opener
    fn new(roots: AdmittedRootSetToken<'roots>) -> Self {
        Self { roots: roots.roots }
    }
    #[allow(dead_code)] // called by the platform contained-open owner
    fn issue_font<R: ResourceExtentReader>(
        &self,
        font_face_id: FontFaceId,
        exact_length: u64,
        reader: R,
    ) -> VerifiedResourceSource<'roots, R> {
        VerifiedResourceSource {
            roots: self.roots,
            id: PendingResourceId::Font(font_face_id),
            exact_length,
            reader,
        }
    }
    #[allow(dead_code)] // called by the platform contained-open owner
    fn issue_image<R: ResourceExtentReader>(
        &self,
        image_id: ImageResourceId,
        exact_length: u64,
        reader: R,
    ) -> VerifiedResourceSource<'roots, R> {
        VerifiedResourceSource {
            roots: self.roots,
            id: PendingResourceId::Image(image_id),
            exact_length,
            reader,
        }
    }
}

/// Bytes read under an admission permit. Only `AdmittedResourceResolver`
/// can construct this value; metadata decoders may inspect it but cannot
/// replace its logical identity, exact length, or streaming digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingResourceBytes {
    session: Arc<()>,
    id: PendingResourceId,
    uri: PortablePath,
    face_index: Option<u32>,
    bytes: Vec<u8>,
    sha256: [u8; 32],
}
impl PendingResourceBytes {
    pub const fn font_face_id(&self) -> Option<FontFaceId> {
        match self.id {
            PendingResourceId::Font(id) => Some(id),
            PendingResourceId::Image(_) => None,
        }
    }
    pub const fn image_id(&self) -> Option<ImageResourceId> {
        match self.id {
            PendingResourceId::Font(_) => None,
            PendingResourceId::Image(id) => Some(id),
        }
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub const fn face_index(&self) -> Option<u32> {
        self.face_index
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum VerifiedMetadata {
    Font {
        source: PendingResourceBytes,
        metadata: AdmittedFontMetadata,
    },
    Image {
        source: PendingResourceBytes,
        width: NonZeroU32,
        height: NonZeroU32,
        decoded_bytes: u64,
    },
}

/// Unforgeable proof that a crate-owned parser derived metadata from the exact
/// bytes and identity in a `PendingResourceBytes` value. Constructors remain
/// crate-private so arbitrary caller metadata cannot cross the trusted boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedMetadataReceipt(VerifiedMetadata);
/// Capability owned by the in-crate metadata parser. It is deliberately not
/// constructible or cloneable by callers, while its issue methods define the
/// hand-off API for the eventual parser implementation.
#[derive(Debug)]
pub struct VerifiedMetadataReceiptOwner {
    _private: (),
}
impl VerifiedMetadataReceiptOwner {
    #[allow(dead_code)] // reserved for the in-crate font/image metadata parser
    fn new() -> Self {
        Self { _private: () }
    }
    pub fn issue_font(
        &self,
        source: PendingResourceBytes,
        metadata: AdmittedFontMetadata,
    ) -> Result<VerifiedMetadataReceipt, ResourceAdmissionError> {
        if source.font_face_id().is_none()
            || source.face_index().is_none()
            || !(16..=16_384).contains(&metadata.units_per_em)
            || metadata.glyph_count == 0
        {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        Ok(VerifiedMetadataReceipt(VerifiedMetadata::Font {
            source,
            metadata,
        }))
    }
    pub fn issue_image(
        &self,
        source: PendingResourceBytes,
        width: NonZeroU32,
        height: NonZeroU32,
        decoded_bytes: u64,
    ) -> Result<VerifiedMetadataReceipt, ResourceAdmissionError> {
        if source.image_id().is_none() || decoded_bytes == 0 {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        Ok(VerifiedMetadataReceipt(VerifiedMetadata::Image {
            source,
            width,
            height,
            decoded_bytes,
        }))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResourceReadKind {
    Font,
    Image,
}

#[derive(Clone, Debug)]
struct ResourceAdmissionBudget {
    limits: ValidatedResourceLimits,
    reserved_bytes: u64,
}
impl ResourceAdmissionBudget {
    fn new(
        declarations: &ResourceCatalog,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, ResourceAdmissionError> {
        if declarations.font_faces.len() > limits.get().max_fonts as usize
            || declarations.images.len() > limits.get().max_images as usize
        {
            return Err(ResourceAdmissionError::ResourceLimit);
        }
        Ok(Self {
            limits: limits.clone(),
            reserved_bytes: 0,
        })
    }
    fn reserve(
        &mut self,
        kind: ResourceReadKind,
        exact_length: u64,
    ) -> Result<(), ResourceAdmissionError> {
        let per_resource = match kind {
            ResourceReadKind::Font => self.limits.get().max_font_bytes,
            ResourceReadKind::Image => self.limits.get().max_image_bytes,
        };
        if exact_length == 0 || exact_length > per_resource {
            return Err(ResourceAdmissionError::ResourceLimit);
        }
        let aggregate = self
            .reserved_bytes
            .checked_add(exact_length)
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        if aggregate > self.limits.get().max_resource_bytes {
            return Err(ResourceAdmissionError::ResourceLimit);
        }
        self.reserved_bytes = aggregate;
        Ok(())
    }
}

/// Exact-length reader issued only after count, per-resource, and aggregate
/// budgets have been consumed. It hashes every chunk as it is read, rejects an
/// early EOF, and never performs a max+1 probe beyond the verified extent.
pub struct BoundedResourceReader<R> {
    inner: R,
    exact_length: u64,
}
impl<R: ResourceExtentReader> BoundedResourceReader<R> {
    fn new(inner: R, exact_length: u64) -> Self {
        Self {
            inner,
            exact_length,
        }
    }
    fn read_verified(mut self) -> Result<(Vec<u8>, [u8; 32]), ResourceAdmissionError> {
        let capacity = usize::try_from(self.exact_length)
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        let mut hasher = StreamingSha256::new();
        let mut remaining = self.exact_length;
        let mut chunk = [0u8; 8192];
        while remaining > 0 {
            let allowed = usize::try_from(remaining.min(chunk.len() as u64))
                .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
            let read = self
                .inner
                .read(&mut chunk[..allowed])
                .map_err(|_| ResourceAdmissionError::ResourceRead)?;
            if read == 0 {
                return Err(ResourceAdmissionError::ResourceLengthMismatch);
            }
            hasher.update(&chunk[..read]);
            bytes.extend_from_slice(&chunk[..read]);
            remaining -= read as u64;
        }
        if self.inner.current_length()? != self.exact_length {
            return Err(ResourceAdmissionError::ResourceLengthMismatch);
        }
        Ok((bytes, hasher.finalize()))
    }
}

#[derive(Clone, Debug)]
struct StreamingSha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffered: usize,
    byte_length: u64,
}
impl StreamingSha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            buffered: 0,
            byte_length: 0,
        }
    }
    fn update(&mut self, mut input: &[u8]) {
        self.byte_length = self.byte_length.wrapping_add(input.len() as u64);
        if self.buffered != 0 {
            let copied = (64 - self.buffered).min(input.len());
            self.buffer[self.buffered..self.buffered + copied].copy_from_slice(&input[..copied]);
            self.buffered += copied;
            input = &input[copied..];
            if self.buffered == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffered = 0;
            }
        }
        while input.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&input[..64]);
            self.compress(&block);
            input = &input[64..];
        }
        self.buffer[..input.len()].copy_from_slice(input);
        self.buffered = input.len();
    }
    fn finalize(mut self) -> [u8; 32] {
        let bit_length = self.byte_length.wrapping_mul(8);
        let mut final_blocks = [0u8; 128];
        final_blocks[..self.buffered].copy_from_slice(&self.buffer[..self.buffered]);
        final_blocks[self.buffered] = 0x80;
        let used = if self.buffered < 56 { 64 } else { 128 };
        final_blocks[used - 8..used].copy_from_slice(&bit_length.to_be_bytes());
        for block in final_blocks[..used].chunks_exact(64) {
            let mut owned = [0u8; 64];
            owned.copy_from_slice(block);
            self.compress(&owned);
        }
        let mut output = [0u8; 32];
        for (chunk, word) in output.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        output
    }
    fn compress(&mut self, block: &[u8; 64]) {
        let mut words = [0u32; 64];
        for (index, word) in words[..16].iter_mut().enumerate() {
            let start = index * 4;
            *word = u32::from_be_bytes([
                block[start],
                block[start + 1],
                block[start + 2],
                block[start + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for (index, constant) in Self::K.iter().enumerate() {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(*constant)
                .wrapping_add(words[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

/// Stateful owner of all resource admission work. It reserves every resource
/// before reading, and issues the immutable ledger only after every declaration
/// has one matching metadata receipt.
#[derive(Debug)]
pub struct AdmittedResourceResolver<'roots> {
    session: Arc<()>,
    roots: Option<&'roots AdmittedRootSet>,
    declarations: ResourceCatalog,
    budget: ResourceAdmissionBudget,
    attempted_fonts: BTreeSet<FontFaceId>,
    attempted_images: BTreeSet<ImageResourceId>,
    fonts: BTreeMap<FontFaceId, AdmittedFont>,
    images: BTreeMap<ImageResourceId, AdmittedImage>,
}
impl AdmittedResourceResolver<'static> {
    /// Safe empty-package workflow for lower crates that must not depend on
    /// `typaxis-document` merely to assemble an empty catalog in tests or
    /// blank-document execution.
    pub fn new_empty(limits: &ValidatedResourceLimits) -> Result<Self, ResourceAdmissionError> {
        Self::new_inner(
            &ResourceCatalog {
                font_faces: vec![],
                images: vec![],
            },
            limits,
            None,
        )
    }

    /// Empty packages need no filesystem capability. Any non-empty resource
    /// catalog must use `new_with_roots`; this prevents a caller from omitting
    /// the configured host-admission context.
    pub fn new(
        declarations: &ResourceCatalog,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, ResourceAdmissionError> {
        if !declarations.font_faces.is_empty() || !declarations.images.is_empty() {
            return Err(ResourceAdmissionError::MissingAdmittedRootSet);
        }
        Self::new_inner(declarations, limits, None)
    }
}
impl<'roots> AdmittedResourceResolver<'roots> {
    pub fn new_with_roots(
        declarations: &ResourceCatalog,
        limits: &ValidatedResourceLimits,
        roots: AdmittedRootSetToken<'roots>,
    ) -> Result<Self, ResourceAdmissionError> {
        Self::new_inner(declarations, limits, Some(roots.roots))
    }

    fn new_inner(
        declarations: &ResourceCatalog,
        limits: &ValidatedResourceLimits,
        roots: Option<&'roots AdmittedRootSet>,
    ) -> Result<Self, ResourceAdmissionError> {
        let budget = ResourceAdmissionBudget::new(declarations, limits)?;
        validate_declaration_order(declarations)?;
        Ok(Self {
            session: Arc::new(()),
            roots,
            declarations: declarations.clone(),
            budget,
            attempted_fonts: BTreeSet::new(),
            attempted_images: BTreeSet::new(),
            fonts: BTreeMap::new(),
            images: BTreeMap::new(),
        })
    }
    pub fn read_font<R: ResourceExtentReader>(
        &mut self,
        source: VerifiedResourceSource<'roots, R>,
    ) -> Result<PendingResourceBytes, ResourceAdmissionError> {
        if !self
            .roots
            .is_some_and(|expected| std::ptr::eq(expected, source.roots))
        {
            return Err(ResourceAdmissionError::RootSetMismatch);
        }
        let PendingResourceId::Font(font_face_id) = source.id else {
            return Err(ResourceAdmissionError::ReceiptKindMismatch);
        };
        let declaration = self
            .declarations
            .font_faces
            .get(font_face_id.get() as usize)
            .filter(|candidate| candidate.font_face_id == font_face_id)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        if self.attempted_fonts.contains(&font_face_id) {
            return Err(ResourceAdmissionError::ConflictingLogicalResource);
        }
        self.budget
            .reserve(ResourceReadKind::Font, source.exact_length)?;
        self.attempted_fonts.insert(font_face_id);
        let (bytes, sha256) =
            BoundedResourceReader::new(source.reader, source.exact_length).read_verified()?;
        Ok(PendingResourceBytes {
            session: Arc::clone(&self.session),
            id: PendingResourceId::Font(font_face_id),
            uri: declaration.uri.clone(),
            face_index: Some(declaration.face_index),
            bytes,
            sha256,
        })
    }
    pub fn read_image<R: ResourceExtentReader>(
        &mut self,
        source: VerifiedResourceSource<'roots, R>,
    ) -> Result<PendingResourceBytes, ResourceAdmissionError> {
        if !self
            .roots
            .is_some_and(|expected| std::ptr::eq(expected, source.roots))
        {
            return Err(ResourceAdmissionError::RootSetMismatch);
        }
        let PendingResourceId::Image(image_id) = source.id else {
            return Err(ResourceAdmissionError::ReceiptKindMismatch);
        };
        let declaration = self
            .declarations
            .images
            .get(image_id.get() as usize)
            .filter(|candidate| candidate.image_id == image_id)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        if self.attempted_images.contains(&image_id) {
            return Err(ResourceAdmissionError::ConflictingLogicalResource);
        }
        self.budget
            .reserve(ResourceReadKind::Image, source.exact_length)?;
        self.attempted_images.insert(image_id);
        let (bytes, sha256) =
            BoundedResourceReader::new(source.reader, source.exact_length).read_verified()?;
        Ok(PendingResourceBytes {
            session: Arc::clone(&self.session),
            id: PendingResourceId::Image(image_id),
            uri: declaration.uri.clone(),
            face_index: None,
            bytes,
            sha256,
        })
    }
    pub fn parse_and_bind_sfnt(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionError> {
        self.ensure_session(&source)?;
        let (units_per_em, glyph_count) = parse_sfnt_metadata(
            source.bytes(),
            source
                .face_index()
                .ok_or(ResourceAdmissionError::ReceiptKindMismatch)?,
        )?;
        let owner = VerifiedMetadataReceiptOwner::new();
        let receipt = owner.issue_font(
            source,
            AdmittedFontMetadata {
                units_per_em,
                glyph_count,
            },
        )?;
        self.bind_verified_metadata(receipt)
    }
    pub fn parse_and_bind_png(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionError> {
        self.ensure_session(&source)?;
        let (width, height, decoded_bytes) = parse_png_metadata(source.bytes())?;
        let owner = VerifiedMetadataReceiptOwner::new();
        let receipt = owner.issue_image(source, width, height, decoded_bytes)?;
        self.bind_verified_metadata(receipt)
    }
    pub fn bind_verified_metadata(
        &mut self,
        receipt: VerifiedMetadataReceipt,
    ) -> Result<(), ResourceAdmissionError> {
        match receipt.0 {
            VerifiedMetadata::Font { source, metadata } => {
                self.ensure_session(&source)?;
                let id = source
                    .font_face_id()
                    .ok_or(ResourceAdmissionError::ReceiptKindMismatch)?;
                let declaration = self
                    .declarations
                    .font_faces
                    .get(id.get() as usize)
                    .filter(|candidate| candidate.font_face_id == id)
                    .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
                if source.uri() != &declaration.uri
                    || source.face_index() != Some(declaration.face_index)
                {
                    return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
                }
                if declaration
                    .expected_sha256
                    .is_some_and(|expected| expected != source.content_hash())
                {
                    return Err(ResourceAdmissionError::ExpectedHashMismatch);
                }
                let font = AdmittedFont::from_verified(
                    id,
                    source.uri,
                    declaration.family.clone(),
                    declaration.face_index,
                    source.bytes,
                    source.sha256,
                    metadata,
                );
                if self.fonts.insert(id, font).is_some() {
                    return Err(ResourceAdmissionError::ConflictingLogicalResource);
                }
            }
            VerifiedMetadata::Image {
                source,
                width,
                height,
                decoded_bytes,
            } => {
                self.ensure_session(&source)?;
                let id = source
                    .image_id()
                    .ok_or(ResourceAdmissionError::ReceiptKindMismatch)?;
                let declaration = self
                    .declarations
                    .images
                    .get(id.get() as usize)
                    .filter(|candidate| candidate.image_id == id)
                    .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
                if source.uri() != &declaration.uri || source.face_index().is_some() {
                    return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
                }
                if declaration
                    .expected_sha256
                    .is_some_and(|expected| expected != source.content_hash())
                {
                    return Err(ResourceAdmissionError::ExpectedHashMismatch);
                }
                let pixels = u64::from(width.get())
                    .checked_mul(u64::from(height.get()))
                    .ok_or(ResourceAdmissionError::ResourceLimit)?;
                if pixels > self.budget.limits.get().max_image_pixels
                    || decoded_bytes > self.budget.limits.get().max_decoded_image_bytes
                {
                    return Err(ResourceAdmissionError::ResourceLimit);
                }
                let image = AdmittedImage::from_verified(
                    id,
                    source.uri,
                    source.bytes,
                    source.sha256,
                    width,
                    height,
                    decoded_bytes,
                );
                if self.images.insert(id, image).is_some() {
                    return Err(ResourceAdmissionError::ConflictingLogicalResource);
                }
            }
        }
        Ok(())
    }
    fn ensure_session(&self, source: &PendingResourceBytes) -> Result<(), ResourceAdmissionError> {
        if Arc::ptr_eq(&self.session, &source.session) {
            Ok(())
        } else {
            Err(ResourceAdmissionError::ReceiptSessionMismatch)
        }
    }
    pub fn finish(self) -> Result<AdmittedResourceLedger, ResourceAdmissionError> {
        if self.fonts.len() != self.declarations.font_faces.len()
            || self.images.len() != self.declarations.images.len()
        {
            return Err(ResourceAdmissionError::MissingLogicalResource);
        }
        let font_families = FontFamilyTable::new(
            self.declarations
                .font_faces
                .iter()
                .map(|declaration| (declaration.family.clone(), declaration.font_face_id))
                .collect(),
        )
        .map_err(map_font_family_error)?;
        Ok(AdmittedResourceLedger {
            fonts: self.fonts.into_values().collect(),
            images: self.images.into_values().collect(),
            font_families,
        })
    }
}

fn parse_sfnt_metadata(
    bytes: &[u8],
    face_index: u32,
) -> Result<(u16, u32), ResourceAdmissionError> {
    let directory_offset = if bytes.get(..4) == Some(b"ttcf") {
        let count = read_be_u32(bytes, 8)?;
        if face_index >= count {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        let offset_position = 12usize
            .checked_add(
                usize::try_from(face_index)
                    .map_err(|_| ResourceAdmissionError::InvalidMetadata)?
                    .checked_mul(4)
                    .ok_or(ResourceAdmissionError::InvalidMetadata)?,
            )
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        usize::try_from(read_be_u32(bytes, offset_position)?)
            .map_err(|_| ResourceAdmissionError::InvalidMetadata)?
    } else {
        if face_index != 0 {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        0
    };
    let signature_end = directory_offset
        .checked_add(4)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    if bytes.get(directory_offset..signature_end) != Some(&0x0001_0000u32.to_be_bytes()) {
        // Profile 1.0 emits CIDFontType2 + FontFile2 and therefore admits
        // TrueType-outline sfnt faces only. OTTO/CFF needs a different PDF
        // object blueprint and must fail closed here.
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    let table_count_offset = directory_offset
        .checked_add(4)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let table_count = usize::from(read_be_u16(bytes, table_count_offset)?);
    let directory_start = directory_offset
        .checked_add(12)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let mut table_tags = BTreeSet::new();
    let mut head = None;
    let mut maxp = None;
    for index in 0..table_count {
        let record = directory_start
            .checked_add(
                index
                    .checked_mul(16)
                    .ok_or(ResourceAdmissionError::InvalidMetadata)?,
            )
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        let tag_end = record
            .checked_add(4)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        let tag: [u8; 4] = bytes
            .get(record..tag_end)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?
            .try_into()
            .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
        if !table_tags.insert(tag) {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        let offset_field = record
            .checked_add(8)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        let length_field = record
            .checked_add(12)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        let offset = usize::try_from(read_be_u32(bytes, offset_field)?)
            .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
        let length = usize::try_from(read_be_u32(bytes, length_field)?)
            .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
        let end = offset
            .checked_add(length)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        if end > bytes.len() {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        match &tag {
            b"head" if length >= 20 => head = Some(offset),
            b"maxp" if length >= 6 => maxp = Some(offset),
            b"head" | b"maxp" => return Err(ResourceAdmissionError::InvalidMetadata),
            _ => {}
        }
    }
    let units_offset = head
        .ok_or(ResourceAdmissionError::InvalidMetadata)?
        .checked_add(18)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let glyph_count_offset = maxp
        .ok_or(ResourceAdmissionError::InvalidMetadata)?
        .checked_add(4)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let units_per_em = read_be_u16(bytes, units_offset)?;
    let glyph_count = u32::from(read_be_u16(bytes, glyph_count_offset)?);
    if !(16..=16_384).contains(&units_per_em) || glyph_count == 0 {
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    Ok((units_per_em, glyph_count))
}

fn parse_png_metadata(
    bytes: &[u8],
) -> Result<(NonZeroU32, NonZeroU32, u64), ResourceAdmissionError> {
    if bytes.get(..8) != Some(b"\x89PNG\r\n\x1a\n")
        || read_be_u32(bytes, 8)? != 13
        || bytes.get(12..16) != Some(b"IHDR")
    {
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    let width =
        NonZeroU32::new(read_be_u32(bytes, 16)?).ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let height =
        NonZeroU32::new(read_be_u32(bytes, 20)?).ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let bit_depth = *bytes
        .get(24)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let color_type = *bytes
        .get(25)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let legal_depth = match color_type {
        0 => matches!(bit_depth, 1 | 2 | 4 | 8 | 16),
        2 => matches!(bit_depth, 8 | 16),
        3 => matches!(bit_depth, 1 | 2 | 4 | 8),
        4 | 6 => matches!(bit_depth, 8 | 16),
        _ => return Err(ResourceAdmissionError::InvalidMetadata),
    };
    if !legal_depth
        || bytes.get(26) != Some(&0)
        || bytes.get(27) != Some(&0)
        || !matches!(bytes.get(28), Some(0) | Some(1))
    {
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    // The admission budget measures the canonical decoded pixel buffer, not
    // packed scanline bytes. Formats that may carry tRNS reserve an alpha
    // channel even when a particular file omits it; palette input is RGBA8;
    // 16-bit samples remain two bytes/sample.
    let decoded_bytes_per_pixel = match (color_type, bit_depth) {
        (0, 16) => 4,
        (0, _) => 2,
        (2, 16) => 8,
        (2, _) => 4,
        (3, _) => 4,
        (4, 16) => 4,
        (4, _) => 2,
        (6, 16) => 8,
        (6, _) => 4,
        _ => return Err(ResourceAdmissionError::InvalidMetadata),
    };
    let decoded_bytes = u64::from(width.get())
        .checked_mul(u64::from(height.get()))
        .and_then(|value| value.checked_mul(decoded_bytes_per_pixel))
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    Ok((width, height, decoded_bytes))
}

fn read_be_u16(bytes: &[u8], offset: usize) -> Result<u16, ResourceAdmissionError> {
    let end = offset
        .checked_add(2)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let encoded: [u8; 2] = bytes
        .get(offset..end)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?
        .try_into()
        .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
    Ok(u16::from_be_bytes(encoded))
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32, ResourceAdmissionError> {
    let end = offset
        .checked_add(4)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let encoded: [u8; 4] = bytes
        .get(offset..end)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?
        .try_into()
        .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
    Ok(u32::from_be_bytes(encoded))
}

fn validate_declaration_order(
    declarations: &ResourceCatalog,
) -> Result<(), ResourceAdmissionError> {
    for (index, declaration) in declarations.font_faces.iter().enumerate() {
        if declaration.font_face_id.get()
            != u32::try_from(index).map_err(|_| ResourceAdmissionError::NonCanonicalResourceId)?
        {
            return Err(ResourceAdmissionError::NonCanonicalResourceId);
        }
    }
    for (index, declaration) in declarations.images.iter().enumerate() {
        if declaration.image_id.get()
            != u32::try_from(index).map_err(|_| ResourceAdmissionError::NonCanonicalResourceId)?
        {
            return Err(ResourceAdmissionError::NonCanonicalResourceId);
        }
    }
    FontFamilyTable::new(
        declarations
            .font_faces
            .iter()
            .map(|declaration| (declaration.family.clone(), declaration.font_face_id))
            .collect(),
    )
    .map_err(map_font_family_error)?;
    Ok(())
}

fn map_font_family_error(_error: FontFamilyError) -> ResourceAdmissionError {
    ResourceAdmissionError::InvalidFontFamily
}

/// Immutable complete-set proof emitted by `AdmittedResourceResolver`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedResourceLedger {
    fonts: Vec<AdmittedFont>,
    images: Vec<AdmittedImage>,
    font_families: FontFamilyTable,
}
impl AdmittedResourceLedger {
    pub fn fonts(&self) -> &[AdmittedFont] {
        &self.fonts
    }
    pub fn images(&self) -> &[AdmittedImage] {
        &self.images
    }
    pub fn font(&self, id: FontFaceId) -> Option<&AdmittedFont> {
        self.fonts.iter().find(|font| font.font_face_id() == id)
    }
    pub fn image(&self, id: ImageResourceId) -> Option<&AdmittedImage> {
        self.images.iter().find(|image| image.image_id() == id)
    }
    pub const fn font_families(&self) -> &FontFamilyTable {
        &self.font_families
    }
    pub const fn token(&self) -> AdmittedResourceLedgerToken<'_> {
        AdmittedResourceLedgerToken { ledger: self }
    }
    pub fn matches_declarations(&self, declarations: &ResourceCatalog) -> bool {
        self.fonts.len() == declarations.font_faces.len()
            && self.images.len() == declarations.images.len()
            && self
                .fonts
                .iter()
                .zip(&declarations.font_faces)
                .all(|(font, declaration)| {
                    font.font_face_id() == declaration.font_face_id
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
                    image.image_id() == declaration.image_id
                        && image.uri() == &declaration.uri
                        && declaration
                            .expected_sha256
                            .map_or(true, |expected| expected == image.content_hash())
                })
    }
    pub fn fingerprint(&self) -> AdmittedResourceFingerprint {
        let mut canonical = String::from("{\"algorithm\":");
        push_jcs_string(&mut canonical, AdmittedResourceFingerprint::ALGORITHM_ID);
        canonical.push_str(",\"fonts\":[");
        for (index, font) in self.fonts.iter().enumerate() {
            if index > 0 {
                canonical.push(',');
            }
            canonical.push_str("{\"face_index\":");
            canonical.push_str(&font.face_index().to_string());
            canonical.push_str(",\"family\":");
            push_jcs_string(&mut canonical, font.family());
            canonical.push_str(",\"font_face_id\":");
            canonical.push_str(&font.font_face_id().get().to_string());
            canonical.push_str(",\"glyph_count\":");
            canonical.push_str(&font.metadata().glyph_count.to_string());
            canonical.push_str(",\"sha256\":");
            push_hash_hex(&mut canonical, font.content_hash());
            canonical.push_str(",\"units_per_em\":");
            canonical.push_str(&font.metadata().units_per_em.to_string());
            canonical.push('}');
        }
        canonical.push_str("],\"images\":[");
        for (index, image) in self.images.iter().enumerate() {
            if index > 0 {
                canonical.push(',');
            }
            canonical.push_str("{\"decoded_bytes\":");
            canonical.push_str(&image.decoded_bytes().to_string());
            canonical.push_str(",\"image_id\":");
            canonical.push_str(&image.image_id().get().to_string());
            canonical.push_str(",\"pixel_height\":");
            canonical.push_str(&image.height().get().to_string());
            canonical.push_str(",\"pixel_width\":");
            canonical.push_str(&image.width().get().to_string());
            canonical.push_str(",\"sha256\":");
            push_hash_hex(&mut canonical, image.content_hash());
            canonical.push('}');
        }
        canonical.push_str("]}");
        admitted_resource_fingerprint_from_jcs(&canonical)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AdmittedResourceLedgerToken<'a> {
    ledger: &'a AdmittedResourceLedger,
}
impl<'a> AdmittedResourceLedgerToken<'a> {
    pub const fn ledger(self) -> &'a AdmittedResourceLedger {
        self.ledger
    }
    pub fn fonts(self) -> &'a [AdmittedFont] {
        self.ledger.fonts()
    }
    pub fn images(self) -> &'a [AdmittedImage] {
        self.ledger.images()
    }
    pub fn fingerprint(self) -> AdmittedResourceFingerprint {
        self.ledger.fingerprint()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmittedFontInstance {
    font_instance_id: typaxis_core::FontInstanceId,
    font_face_id: FontFaceId,
    admitted_sha256: [u8; 32],
}
impl AdmittedFontInstance {
    pub const fn font_instance_id(self) -> typaxis_core::FontInstanceId {
        self.font_instance_id
    }
    pub const fn font_face_id(self) -> FontFaceId {
        self.font_face_id
    }
    pub const fn admitted_sha256(self) -> [u8; 32] {
        self.admitted_sha256
    }
}

/// Canonical dense instance IDs derived from a selected set of faces in one
/// immutable admitted ledger. Caller order and worker completion order cannot
/// influence the assigned IDs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedFontInstanceTable {
    ledger_fingerprint: AdmittedResourceFingerprint,
    instances: Vec<AdmittedFontInstance>,
}
impl AdmittedFontInstanceTable {
    pub fn from_used_faces(
        ledger: &AdmittedResourceLedger,
        used_faces: impl IntoIterator<Item = FontFaceId>,
    ) -> Result<Self, ResourceAdmissionError> {
        let used_faces: BTreeSet<_> = used_faces.into_iter().collect();
        let mut keyed = Vec::new();
        keyed
            .try_reserve_exact(used_faces.len())
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        for font_face_id in used_faces {
            let font = ledger
                .font(font_face_id)
                .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
            keyed.push((font_face_id, font.content_hash()));
        }
        keyed.sort_unstable();
        let mut instances = Vec::new();
        instances
            .try_reserve_exact(keyed.len())
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        for (index, (font_face_id, admitted_sha256)) in keyed.into_iter().enumerate() {
            let index = u32::try_from(index).map_err(|_| ResourceAdmissionError::ResourceLimit)?;
            instances.push(AdmittedFontInstance {
                font_instance_id: typaxis_core::FontInstanceId::new(index),
                font_face_id,
                admitted_sha256,
            });
        }
        Ok(Self {
            ledger_fingerprint: ledger.fingerprint(),
            instances,
        })
    }
    pub fn instances(&self) -> &[AdmittedFontInstance] {
        &self.instances
    }
    pub const fn ledger_fingerprint(&self) -> AdmittedResourceFingerprint {
        self.ledger_fingerprint
    }
    pub fn get(&self, id: typaxis_core::FontInstanceId) -> Option<&AdmittedFontInstance> {
        self.instances
            .get(id.get() as usize)
            .filter(|instance| instance.font_instance_id == id)
    }
    pub fn resolve<'a>(
        &'a self,
        id: typaxis_core::FontInstanceId,
        ledger: &'a AdmittedResourceLedger,
    ) -> Option<AdmittedFontInstanceRef<'a>> {
        if ledger.fingerprint() != self.ledger_fingerprint {
            return None;
        }
        let instance = self.get(id)?;
        let font = ledger.font(instance.font_face_id)?;
        if font.content_hash() != instance.admitted_sha256 {
            return None;
        }
        Some(AdmittedFontInstanceRef {
            ledger_fingerprint: self.ledger_fingerprint,
            instance,
            font,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmittedFontInstanceRef<'a> {
    ledger_fingerprint: AdmittedResourceFingerprint,
    instance: &'a AdmittedFontInstance,
    font: &'a AdmittedFont,
}
impl<'a> AdmittedFontInstanceRef<'a> {
    pub const fn ledger_fingerprint(self) -> AdmittedResourceFingerprint {
        self.ledger_fingerprint
    }
    pub const fn font_instance_id(self) -> typaxis_core::FontInstanceId {
        self.instance.font_instance_id
    }
    pub const fn font_face_id(self) -> FontFaceId {
        self.instance.font_face_id
    }
    pub const fn admitted_sha256(self) -> [u8; 32] {
        self.instance.admitted_sha256
    }
    pub fn font_bytes(self) -> &'a [u8] {
        self.font.bytes()
    }
    pub const fn face_index(self) -> u32 {
        self.font.face_index()
    }
    pub const fn metadata(self) -> &'a AdmittedFontMetadata {
        self.font.metadata()
    }
}

/// Compatibility name for read-only consumers; no public constructor exists.
pub type AdmittedResources = AdmittedResourceLedger;

fn push_hash_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use typaxis_core::{sha256, ResourceLimits};
    use typaxis_document::{FontFaceDeclaration, ImageDeclaration};

    impl ResourceExtentReader for Cursor<Vec<u8>> {
        fn current_length(&self) -> Result<u64, ResourceAdmissionError> {
            Ok(self.get_ref().len() as u64)
        }
    }

    fn limits(overrides: ResourceLimits) -> ValidatedResourceLimits {
        ValidatedResourceLimits::new(overrides).unwrap()
    }

    fn font_catalog(count: u32) -> ResourceCatalog {
        ResourceCatalog {
            font_faces: (0..count)
                .map(|id| FontFaceDeclaration {
                    font_face_id: FontFaceId::new(id),
                    family: format!("font-{id}"),
                    uri: PortablePath::new(format!("font-{id}.ttf")).unwrap(),
                    face_index: 0,
                    expected_sha256: None,
                })
                .collect(),
            images: vec![],
        }
    }

    fn png_header(
        width: u32,
        height: u32,
        bit_depth: u8,
        color_type: u8,
        compression: u8,
        filter: u8,
        interlace: u8,
    ) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[bit_depth, color_type, compression, filter, interlace]);
        bytes
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        png_header(width, height, 8, 6, 0, 0, 0)
    }

    fn sfnt_with_units_per_em(units_per_em: u16) -> Vec<u8> {
        let mut bytes = vec![0; 70];
        bytes[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        bytes[4..6].copy_from_slice(&2u16.to_be_bytes());
        bytes[12..16].copy_from_slice(b"head");
        bytes[20..24].copy_from_slice(&44u32.to_be_bytes());
        bytes[24..28].copy_from_slice(&20u32.to_be_bytes());
        bytes[28..32].copy_from_slice(b"maxp");
        bytes[36..40].copy_from_slice(&64u32.to_be_bytes());
        bytes[40..44].copy_from_slice(&6u32.to_be_bytes());
        bytes[62..64].copy_from_slice(&units_per_em.to_be_bytes());
        bytes[68..70].copy_from_slice(&3u16.to_be_bytes());
        bytes
    }

    fn sfnt() -> Vec<u8> {
        sfnt_with_units_per_em(1000)
    }

    #[test]
    fn admission_reserves_aggregate_before_second_read() {
        let limits = limits(ResourceLimits {
            max_font_bytes: 4,
            max_image_bytes: 4,
            max_resource_bytes: 4,
            ..ResourceLimits::default()
        });
        let catalog = font_catalog(2);
        let roots = AdmittedRootSetOwner::new().issue();
        let owner = VerifiedResourceSourceOwner::new(roots.token());
        let mut resolver =
            AdmittedResourceResolver::new_with_roots(&catalog, &limits, roots.token()).unwrap();
        resolver
            .read_font(owner.issue_font(FontFaceId::new(0), 3, Cursor::new(b"abc".to_vec())))
            .unwrap();
        assert_eq!(
            resolver.read_font(owner.issue_font(
                FontFaceId::new(1),
                3,
                Cursor::new(b"def".to_vec()),
            )),
            Err(ResourceAdmissionError::ResourceLimit)
        );
    }

    #[test]
    fn nonempty_admission_requires_the_same_sealed_root_set() {
        let limits = limits(ResourceLimits::default());
        let catalog = font_catalog(1);
        assert_eq!(
            AdmittedResourceResolver::new(&catalog, &limits).unwrap_err(),
            ResourceAdmissionError::MissingAdmittedRootSet
        );

        let expected_roots = AdmittedRootSetOwner::new().issue();
        let other_roots = AdmittedRootSetOwner::new().issue();
        let source_owner = VerifiedResourceSourceOwner::new(other_roots.token());
        let mut resolver =
            AdmittedResourceResolver::new_with_roots(&catalog, &limits, expected_roots.token())
                .unwrap();
        assert_eq!(
            resolver.read_font(source_owner.issue_font(
                FontFaceId::new(0),
                3,
                Cursor::new(b"abc".to_vec()),
            )),
            Err(ResourceAdmissionError::RootSetMismatch)
        );
    }

    #[test]
    fn bounded_reader_hashes_stream_and_rechecks_extent() {
        let bytes = vec![0x5a; 20_000];
        let (read, digest) =
            BoundedResourceReader::new(Cursor::new(bytes.clone()), bytes.len() as u64)
                .read_verified()
                .unwrap();
        assert_eq!(digest, sha256(&read));
        assert_eq!(read, bytes);
        assert_eq!(
            BoundedResourceReader::new(Cursor::new(b"ab".to_vec()), 3).read_verified(),
            Err(ResourceAdmissionError::ResourceLengthMismatch)
        );
        assert_eq!(
            BoundedResourceReader::new(Cursor::new(b"abc".to_vec()), 2).read_verified(),
            Err(ResourceAdmissionError::ResourceLengthMismatch)
        );
    }

    #[test]
    fn png_metadata_is_derived_from_admitted_bytes() {
        let bytes = png(2, 3);
        let catalog = ResourceCatalog {
            font_faces: vec![],
            images: vec![ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("image.png").unwrap(),
                expected_sha256: Some(sha256(&bytes)),
            }],
        };
        let limits = limits(ResourceLimits::default());
        let roots = AdmittedRootSetOwner::new().issue();
        let owner = VerifiedResourceSourceOwner::new(roots.token());
        let mut resolver =
            AdmittedResourceResolver::new_with_roots(&catalog, &limits, roots.token()).unwrap();
        let pending = resolver
            .read_image(owner.issue_image(
                ImageResourceId::new(0),
                bytes.len() as u64,
                Cursor::new(bytes),
            ))
            .unwrap();
        resolver.parse_and_bind_png(pending).unwrap();
        let ledger = resolver.finish().unwrap();
        let image = ledger.image(ImageResourceId::new(0)).unwrap();
        assert_eq!((image.width().get(), image.height().get()), (2, 3));
        assert_eq!(image.decoded_bytes(), 24);
        assert!(ledger.matches_declarations(&catalog));
    }

    #[test]
    fn png_decoded_budget_uses_canonical_expanded_pixels() {
        assert_eq!(
            parse_png_metadata(&png_header(2, 3, 1, 0, 0, 0, 0))
                .unwrap()
                .2,
            12
        );
        assert_eq!(
            parse_png_metadata(&png_header(2, 3, 8, 2, 0, 0, 0))
                .unwrap()
                .2,
            24
        );
        assert_eq!(
            parse_png_metadata(&png_header(2, 3, 1, 3, 0, 0, 0))
                .unwrap()
                .2,
            24
        );
        assert_eq!(
            parse_png_metadata(&png_header(2, 3, 16, 6, 0, 0, 1))
                .unwrap()
                .2,
            48
        );
        for invalid in [
            png_header(1, 1, 4, 2, 0, 0, 0),
            png_header(1, 1, 16, 3, 0, 0, 0),
            png_header(1, 1, 8, 6, 1, 0, 0),
            png_header(1, 1, 8, 6, 0, 1, 0),
            png_header(1, 1, 8, 6, 0, 0, 2),
        ] {
            assert_eq!(
                parse_png_metadata(&invalid),
                Err(ResourceAdmissionError::InvalidMetadata)
            );
        }

        let bytes = png(2, 3);
        let catalog = ResourceCatalog {
            font_faces: vec![],
            images: vec![ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("image.png").unwrap(),
                expected_sha256: Some(sha256(&bytes)),
            }],
        };
        for (max_decoded_image_bytes, expected) in [
            (24, Ok(())),
            (23, Err(ResourceAdmissionError::ResourceLimit)),
        ] {
            let limits = limits(ResourceLimits {
                max_decoded_image_bytes,
                ..ResourceLimits::default()
            });
            let roots = AdmittedRootSetOwner::new().issue();
            let owner = VerifiedResourceSourceOwner::new(roots.token());
            let mut resolver =
                AdmittedResourceResolver::new_with_roots(&catalog, &limits, roots.token()).unwrap();
            let pending = resolver
                .read_image(owner.issue_image(
                    ImageResourceId::new(0),
                    bytes.len() as u64,
                    Cursor::new(bytes.clone()),
                ))
                .unwrap();
            assert_eq!(resolver.parse_and_bind_png(pending), expected);
        }
    }

    #[test]
    fn cidfont_type2_admission_rejects_cff_and_invalid_units_per_em() {
        assert!(parse_sfnt_metadata(&sfnt_with_units_per_em(16), 0).is_ok());
        assert!(parse_sfnt_metadata(&sfnt_with_units_per_em(16_384), 0).is_ok());
        assert_eq!(
            parse_sfnt_metadata(&sfnt_with_units_per_em(15), 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );
        assert_eq!(
            parse_sfnt_metadata(&sfnt_with_units_per_em(16_385), 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );
        let mut cff = sfnt();
        cff[..4].copy_from_slice(b"OTTO");
        assert_eq!(
            parse_sfnt_metadata(&cff, 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );
        let mut duplicate_head = sfnt();
        duplicate_head[28..32].copy_from_slice(b"head");
        assert_eq!(
            parse_sfnt_metadata(&duplicate_head, 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );

        let mut duplicate_optional = vec![0; 104];
        duplicate_optional[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        duplicate_optional[4..6].copy_from_slice(&4u16.to_be_bytes());
        for (record, tag, offset, length) in [
            (12usize, b"head", 76u32, 20u32),
            (28, b"maxp", 96, 6),
            (44, b"name", 102, 1),
            (60, b"name", 103, 1),
        ] {
            duplicate_optional[record..record + 4].copy_from_slice(tag);
            duplicate_optional[record + 8..record + 12].copy_from_slice(&offset.to_be_bytes());
            duplicate_optional[record + 12..record + 16].copy_from_slice(&length.to_be_bytes());
        }
        duplicate_optional[94..96].copy_from_slice(&1_000u16.to_be_bytes());
        duplicate_optional[100..102].copy_from_slice(&3u16.to_be_bytes());
        assert_eq!(
            parse_sfnt_metadata(&duplicate_optional, 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );
    }

    #[test]
    fn verified_font_metadata_receipt_rechecks_the_profile_units_range() {
        let owner = VerifiedMetadataReceiptOwner::new();
        let source = |units_per_em| {
            owner.issue_font(
                PendingResourceBytes {
                    session: Arc::new(()),
                    id: PendingResourceId::Font(FontFaceId::new(0)),
                    uri: PortablePath::new("font.ttf").unwrap(),
                    face_index: Some(0),
                    bytes: vec![1],
                    sha256: [2; 32],
                },
                AdmittedFontMetadata {
                    units_per_em,
                    glyph_count: 1,
                },
            )
        };
        assert!(source(16).is_ok());
        assert!(source(16_384).is_ok());
        assert_eq!(source(15), Err(ResourceAdmissionError::InvalidMetadata));
        assert_eq!(source(16_385), Err(ResourceAdmissionError::InvalidMetadata));
    }

    #[test]
    fn pending_bytes_cannot_bypass_another_resolvers_budget_session() {
        let bytes = sfnt();
        let catalog = font_catalog(1);
        let roots = AdmittedRootSetOwner::new().issue();
        let source_owner = VerifiedResourceSourceOwner::new(roots.token());
        let permissive_limits = limits(ResourceLimits::default());
        let mut issuing =
            AdmittedResourceResolver::new_with_roots(&catalog, &permissive_limits, roots.token())
                .unwrap();
        let pending = issuing
            .read_font(source_owner.issue_font(
                FontFaceId::new(0),
                bytes.len() as u64,
                Cursor::new(bytes),
            ))
            .unwrap();

        let strict_limits = limits(ResourceLimits {
            max_font_bytes: 1,
            ..ResourceLimits::default()
        });
        let mut foreign =
            AdmittedResourceResolver::new_with_roots(&catalog, &strict_limits, roots.token())
                .unwrap();
        assert_eq!(
            foreign.parse_and_bind_sfnt(pending),
            Err(ResourceAdmissionError::ReceiptSessionMismatch)
        );
        assert_eq!(
            foreign.finish(),
            Err(ResourceAdmissionError::MissingLogicalResource)
        );
    }

    #[test]
    fn font_instance_identity_is_ledger_issued_and_dense() {
        let bytes = sfnt();
        let mut catalog = font_catalog(1);
        catalog.font_faces[0].expected_sha256 = Some(sha256(&bytes));
        let limits = limits(ResourceLimits::default());
        let roots = AdmittedRootSetOwner::new().issue();
        let owner = VerifiedResourceSourceOwner::new(roots.token());
        let mut resolver =
            AdmittedResourceResolver::new_with_roots(&catalog, &limits, roots.token()).unwrap();
        let pending = resolver
            .read_font(owner.issue_font(
                FontFaceId::new(0),
                bytes.len() as u64,
                Cursor::new(bytes.clone()),
            ))
            .unwrap();
        resolver.parse_and_bind_sfnt(pending).unwrap();
        let ledger = resolver.finish().unwrap();
        let table =
            AdmittedFontInstanceTable::from_used_faces(&ledger, [FontFaceId::new(0)]).unwrap();
        let instance = table
            .resolve(typaxis_core::FontInstanceId::new(0), &ledger)
            .unwrap();
        assert_eq!(instance.font_face_id(), FontFaceId::new(0));
        assert_eq!(instance.ledger_fingerprint(), ledger.fingerprint());
        assert_eq!(instance.font_bytes(), bytes);
        assert_eq!(instance.admitted_sha256(), sha256(&bytes));
        assert_eq!(instance.metadata().units_per_em, 1000);
        assert_eq!(instance.metadata().glyph_count, 3);
    }
}
