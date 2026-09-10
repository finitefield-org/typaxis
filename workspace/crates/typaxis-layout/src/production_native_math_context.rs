//! One immutable native computation owner shared by all convergence passes.
use super::*;

/// Source-bound computations with an immutable borrow of the admitted resources.
/// Reuse is restricted to the same profile session and exact resource owner.
pub struct ProductionNativeMathContext<'r> {
    computations: crate::ProductionNativeMathComputations,
    package_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    profile: typaxis_syntax::StagingPrecomposedVectorProfileProgressToken,
    limits_fingerprint: [u8; 32],
    admitted: &'r AdmittedResourceLedger,
}
impl ProductionNativeMathContext<'_> {
    pub fn computations(&self) -> &crate::ProductionNativeMathComputations {
        &self.computations
    }
    pub fn verify_source(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingPrecomposedVectorProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
    ) -> Result<(), ProductionInlinePreparationError> {
        if self.package_sha256 != package.canonical_jcs_sha256()
            || self.semantic_fingerprint != package.semantic_fingerprint()
            || !profile.matches_progress(&self.profile)
            || self.limits_fingerprint != limits.fingerprint()
            || !std::ptr::eq(self.admitted, admitted)
        {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}

/// Compute once before shape/page feedback. `None` issues no native receipt;
/// ordinary preparation remains responsible for native-free source verification.
pub fn prepare_production_native_math_context<'r>(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &'r AdmittedResourceLedger,
) -> Result<Option<ProductionNativeMathContext<'r>>, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let root = NodeId::new(0);
    // No computation receipt is issued for native-free documents. Their ordinary
    // preparation still verifies the profile and bindings; avoid an extra full
    // vector-profile scan here for every native-free caller.
    if package.math_nodes().is_empty() {
        return Ok(None);
    }
    profile
        .authorizes(package, limits)
        .map_err(|_| error(root, E::ReceiptMismatch))?;
    let view = typaxis_syntax::StagingMathProfileView::new_for_production(package, limits)
        .map_err(|_| error(root, E::ReceiptMismatch))?;
    let session = typaxis_syntax::StagingMathProfileSessionIdentity::fresh();
    let native_profile =
        typaxis_syntax::StagingMathProfileAuthorization::bind_production_profile_receipt(
            view,
            profile.profile_receipt_fingerprint(),
            package,
            limits,
            &session,
        )
        .map_err(|_| error(root, E::ReceiptMismatch))?;
    let computations =
        crate::compute_production_native_math(package, &native_profile, limits, admitted, 0, 0)
            .map_err(|e| error(root, E::NativeMath(e)))?;
    Ok(Some(ProductionNativeMathContext {
        computations,
        package_sha256: package.canonical_jcs_sha256(),
        semantic_fingerprint: package.semantic_fingerprint(),
        profile: profile.progress_token(),
        limits_fingerprint: limits.fingerprint(),
        admitted,
    }))
}

pub(super) enum PreparedNativeMath<'a> {
    Owned(ProductionNativeMathContext<'a>),
    Borrowed(&'a ProductionNativeMathContext<'a>),
}
impl std::ops::Deref for PreparedNativeMath<'_> {
    type Target = crate::ProductionNativeMathComputations;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(context) => context.computations(),
            Self::Borrowed(context) => context.computations(),
        }
    }
}
