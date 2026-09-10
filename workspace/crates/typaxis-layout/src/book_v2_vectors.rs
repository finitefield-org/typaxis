//! Successor vector bindings retain the admitted original source and actual /3
//! resources; no legacy semantic/profile/layout receipt is constructed.
use super::*;
use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
use typaxis_syntax::book_v2::{BookV2ResourcePolicy, PreparedBookVector, SourceAdmittedBookV2Body};

pub const BOOK_V2_VECTOR_EPOCH_ALGORITHM: &str = "typaxis.book-2-vector-epoch/1";
pub const BOOK_V2_VECTOR_BINDING_ALGORITHM: &str = "typaxis.book-2-vector-binding/1";
pub const BOOK_V2_VECTOR_SET_ALGORITHM: &str = "typaxis.book-2-vector-bindings/1";

#[derive(Debug)]
pub struct BookV2BoundVector<'a> {
    source: &'a PreparedBookVector,
    resource: BoundPrecomposedVectorResource,
    placement: PrecomposedVectorPlacementInput,
    provenance: Option<&'a typaxis_document::VectorProvenance>,
    fingerprint: [u8; 32],
}
impl<'a> BookV2BoundVector<'a> {
    pub fn source(&self) -> &'a PreparedBookVector {
        self.source
    }
    pub fn node_id(&self) -> NodeId {
        self.source.node_id()
    }
    pub fn kind(&self) -> PrecomposedVectorKind {
        self.source.kind()
    }
    pub fn owner_source_span(&self) -> SourceSpan {
        self.source.owner_source_span()
    }
    pub fn resource(&self) -> &BoundPrecomposedVectorResource {
        &self.resource
    }
    pub fn placement(&self) -> &PrecomposedVectorPlacementInput {
        &self.placement
    }
    pub fn provenance(&self) -> Option<&'a typaxis_document::VectorProvenance> {
        self.provenance
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn binding_fingerprint(&self) -> PrecomposedVectorBindingFingerprint {
        PrecomposedVectorBindingFingerprint::from_receipt(self.fingerprint)
    }
}
pub struct BookV2VectorBindings<'a> {
    body: &'a SourceAdmittedBookV2Body,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits_fingerprint: [u8; 32],
    epoch: [u8; 32],
    receipts: Vec<BookV2BoundVector<'a>>,
    fingerprint: [u8; 32],
}
impl<'a> BookV2VectorBindings<'a> {
    pub fn body(&self) -> &'a SourceAdmittedBookV2Body {
        self.body
    }
    pub fn epoch(&self) -> [u8; 32] {
        self.epoch
    }
    pub fn receipts(&self) -> &[BookV2BoundVector<'a>] {
        &self.receipts
    }
    pub fn receipt(&self, owner: NodeId) -> Option<&BookV2BoundVector<'a>> {
        self.receipts
            .binary_search_by_key(&owner, BookV2BoundVector::node_id)
            .ok()
            .map(|i| &self.receipts[i])
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        body: &SourceAdmittedBookV2Body,
        admitted: &AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), PrecomposedVectorBindingError> {
        if !std::ptr::eq(self.body, body)
            || !std::ptr::eq(self.admitted, admitted)
            || self.limits_fingerprint != limits.fingerprint()
        {
            return Err(PrecomposedVectorBindingError::ReceiptMismatch);
        }
        Ok(())
    }
}
impl std::fmt::Debug for BookV2VectorBindings<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BookV2VectorBindings")
            .field("receipts", &self.receipts.len())
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}
fn fold_hash(previous: [u8; 32], next: [u8; 32]) -> [u8; 32] {
    let mut bytes = [0; 64];
    bytes[..32].copy_from_slice(&previous);
    bytes[32..].copy_from_slice(&next);
    sha256(&bytes)
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2VectorBindingError {
    Binding(PrecomposedVectorBindingError),
    OutputLimit,
}
impl std::fmt::Display for BookV2VectorBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 vector binding: {self:?}")
    }
}
impl std::error::Error for BookV2VectorBindingError {}
pub fn bind_book_v2_vectors<'a>(
    policy: &BookV2ResourcePolicy<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
) -> Result<BookV2VectorBindings<'a>, BookV2VectorBindingError> {
    if policy.body().styled().body().vectors().len() as u64 > limits.base().get().max_fragments {
        return Err(BookV2VectorBindingError::OutputLimit);
    }
    build_bindings(policy, admitted, limits).map_err(BookV2VectorBindingError::Binding)
}
fn build_bindings<'a>(
    policy: &BookV2ResourcePolicy<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
) -> Result<BookV2VectorBindings<'a>, PrecomposedVectorBindingError> {
    use PrecomposedVectorBindingError as E;
    let body = policy.body();
    policy
        .verify_for(body, limits)
        .map_err(|_| E::ProfileMismatch)?;
    let source = body.styled().body();
    if admitted.profile_fingerprint() != policy.fingerprint()
        || admitted.effective_limits() != limits
        || !admitted.matches_declared_resources(source.resources())
    {
        return Err(E::AdmissionMismatch);
    }
    let epoch = fold_hash(
        fold_hash(
            sha256(BOOK_V2_VECTOR_EPOCH_ALGORITHM.as_bytes()),
            policy.fingerprint(),
        ),
        admitted.fingerprint(),
    );
    let mut receipts = Vec::new();
    receipts
        .try_reserve_exact(source.vectors().len())
        .map_err(|_| E::AllocationFailure)?;
    let mut fingerprint = fold_hash(sha256(BOOK_V2_VECTOR_SET_ALGORITHM.as_bytes()), epoch);
    let mut previous = None;
    for vector in source.vectors() {
        let owner = vector.node_id();
        if previous.is_some_and(|p| p >= owner) {
            return Err(E::ReceiptMismatch);
        }
        previous = Some(owner);
        let declaration = source
            .resources()
            .images
            .get(vector.image_id().get() as usize)
            .filter(|d| d.image_id == vector.image_id())
            .ok_or(E::ResourceMismatch(owner))?;
        let image = admitted
            .image(vector.image_id())
            .ok_or(E::ResourceMismatch(owner))?;
        let attestation = image
            .safe_vector_attestation()
            .ok_or(E::ResourceMismatch(owner))?;
        let resource = bind_vector_resource_core(
            owner,
            vector.kind(),
            declaration,
            &attestation,
            policy.fingerprint(),
            limits,
        )?;
        let placement = bind_vector_placement_core(
            owner,
            vector.kind(),
            vector.payload(),
            body.styled().vector_style(owner),
            &resource,
        )?;
        if matches!(
            vector.kind(),
            PrecomposedVectorKind::MathVector | PrecomposedVectorKind::MathVectorBlock
        ) && (vector.source_tex().is_none()
            || vector.alternative().resolved_actual_text().is_none()
            || declaration.vector_provenance.is_none())
        {
            return Err(E::ReceiptMismatch);
        }
        // Epoch commits the entire exact source/package, including TeX, Alt,
        // language, equation numbering, style and producer assertions. Node id
        // selects this immutable source record; geometry and admitted IR are
        // serialized explicitly. No source text/provenance buffers are cloned.
        let mut canonical = String::new();
        canonical
            .try_reserve_exact(4096)
            .map_err(|_| E::AllocationFailure)?;
        canonical.push_str("{\"algorithm\":");
        push_jcs_string(&mut canonical, BOOK_V2_VECTOR_BINDING_ALGORITHM);
        canonical.push_str(",\"epoch\":");
        push_hash(&mut canonical, epoch);
        canonical.push_str(",\"node_id\":");
        canonical.push_str(&owner.get().to_string());
        canonical.push_str(",\"placement\":");
        push_precomposed_vector_placement(&mut canonical, &placement);
        canonical.push_str(",\"resource\":");
        push_bound_precomposed_vector_resource(&mut canonical, &resource);
        canonical.push_str(",\"source_span\":");
        push_source_span(&mut canonical, vector.owner_source_span());
        canonical.push('}');
        let receipt = BookV2BoundVector {
            source: vector,
            resource,
            placement,
            provenance: declaration.vector_provenance.as_ref(),
            fingerprint: sha256(canonical.as_bytes()),
        };
        fingerprint = fold_hash(fingerprint, receipt.fingerprint());
        receipts.push(receipt);
    }
    Ok(BookV2VectorBindings {
        body,
        admitted,
        limits_fingerprint: limits.fingerprint(),
        epoch,
        receipts,
        fingerprint,
    })
}
