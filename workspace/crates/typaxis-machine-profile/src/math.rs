use typaxis_core::{push_jcs_string, sha256, M4EffectiveResourceLimits, NodeId};
use typaxis_syntax::{
    StagingMathProfileAuthorization, StagingMathProfileView, ValidatedStagingSemanticPackage,
};

use crate::{
    safe_vector::preflight_staging_safe_vector_profile_for_math, StagingSafeVectorProfileReceipt,
    StagingSemanticContainerSessionIdentity,
};

pub const STAGING_MATH_PROFILE_ALGORITHM: &str = "typaxis.production-book-math-profile/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMathProfileError {
    BaseProfile,
    MissingMath,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingMathProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BaseProfile => {
                formatter.write_str("L5100: production-book base preflight failed")
            }
            Self::MissingMath => formatter.write_str("L5100: math profile requires typed math"),
            Self::ReceiptMismatch => formatter.write_str("I9190: math profile receipt mismatch"),
        }
    }
}

impl std::error::Error for StagingMathProfileError {}

#[derive(Debug)]
pub struct StagingMathProfileReceipt {
    base: StagingSafeVectorProfileReceipt,
    session: StagingSemanticContainerSessionIdentity,
    math_node_ids: Vec<NodeId>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
    authorization: StagingMathProfileAuthorization,
}

impl StagingMathProfileReceipt {
    pub const fn base(&self) -> &StagingSafeVectorProfileReceipt {
        &self.base
    }
    pub fn math_node_ids(&self) -> &[NodeId] {
        &self.math_node_ids
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub const fn authorization(&self) -> &StagingMathProfileAuthorization {
        &self.authorization
    }

    pub fn authorizes(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingMathProfileError> {
        let view = StagingMathProfileView::new(package, limits)
            .map_err(|_| StagingMathProfileError::ReceiptMismatch)?;
        let math_node_ids: Vec<_> = package
            .math_nodes()
            .iter()
            .map(|value| value.domain().node_id)
            .collect();
        let canonical_jcs = encode(
            package,
            self.base.fingerprint(),
            limits.fingerprint(),
            &math_node_ids,
            &view,
        );
        if self.math_node_ids != math_node_ids
            || self.authorization.view() != &view
            || self.authorization.profile_receipt_fingerprint() != self.fingerprint
            || self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
            || self.authorization.authorizes(package, limits).is_err()
        {
            return Err(StagingMathProfileError::ReceiptMismatch);
        }
        Ok(())
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
        session: &StagingSemanticContainerSessionIdentity,
    ) -> Result<(), StagingMathProfileError> {
        self.base
            .verify(package, limits, session)
            .map_err(|_| StagingMathProfileError::ReceiptMismatch)?;
        if self.session != *session {
            return Err(StagingMathProfileError::ReceiptMismatch);
        }
        self.authorizes(package, limits)
    }
}

pub fn preflight_staging_math_profile(
    package: &ValidatedStagingSemanticPackage,
    limits: &M4EffectiveResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingMathProfileReceipt, StagingMathProfileError> {
    if package.math_nodes().is_empty() {
        return Err(StagingMathProfileError::MissingMath);
    }
    let base = preflight_staging_safe_vector_profile_for_math(package, limits, session)
        .map_err(|_| StagingMathProfileError::BaseProfile)?;
    let view = StagingMathProfileView::new(package, limits)
        .map_err(|_| StagingMathProfileError::ReceiptMismatch)?;
    let math_node_ids: Vec<_> = package
        .math_nodes()
        .iter()
        .map(|value| value.domain().node_id)
        .collect();
    let canonical_jcs = encode(
        package,
        base.fingerprint(),
        limits.fingerprint(),
        &math_node_ids,
        &view,
    );
    let fingerprint = sha256(canonical_jcs.as_bytes());
    let authorization = StagingMathProfileAuthorization::bind_profile_receipt(
        view,
        fingerprint,
        package,
        limits,
        session.math_profile_session(),
    )
    .map_err(|_| StagingMathProfileError::ReceiptMismatch)?;
    let receipt = StagingMathProfileReceipt {
        base,
        session: session.clone(),
        math_node_ids,
        fingerprint,
        canonical_jcs,
        authorization,
    };
    receipt.verify(package, limits, session)?;
    Ok(receipt)
}

fn encode(
    package: &ValidatedStagingSemanticPackage,
    base: [u8; 32],
    limits: [u8; 32],
    math_node_ids: &[NodeId],
    authorization: &StagingMathProfileView,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MATH_PROFILE_ALGORITHM);
    output.push_str(",\"alternative\":\"producer-speech\",\"ast_fingerprint_algorithm\":");
    push_jcs_string(&mut output, authorization.ast_fingerprint_algorithm());
    output.push_str(",\"authorization\":");
    push_hash(&mut output, authorization.profile_fingerprint());
    output.push_str(",\"base_profile_fingerprint\":");
    push_hash(&mut output, base);
    output.push_str(",\"formatter\":");
    push_jcs_string(&mut output, authorization.formatter());
    output.push_str(",\"kinds\":[\"inline_math\",\"display_math\"]");
    output.push_str(",\"layout_algorithm\":");
    push_jcs_string(&mut output, authorization.layout_algorithm());
    output.push_str(",\"layout_work_algorithm\":");
    push_jcs_string(&mut output, authorization.layout_work_algorithm());
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits);
    output.push_str(",\"math_node_ids\":[");
    for (index, node_id) in math_node_ids.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&node_id.get().to_string());
    }
    output.push_str("],\"package_fingerprint\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push_str(",\"parser\":");
    push_jcs_string(&mut output, authorization.parser());
    output.push_str(",\"required_vector_media\":\"svg-safe-1\",\"source\":{\"language\":\"typaxis-math\",\"version\":\"1\"}");
    output.push_str(",\"source_identity\":");
    push_jcs_string(&mut output, authorization.source_identity());
    output.push_str(",\"vector_algorithm\":");
    push_jcs_string(&mut output, authorization.vector_algorithm());
    output.push('}');
    output
}

fn push_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{M4ResourceLimits, ResourceLimits, ValidatedResourceLimits};
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::StagingSemanticPackageParser;

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/math/job/document-package.json"
    ));

    #[test]
    fn math_profile_is_closed_and_old_public_profiles_remain_unaware() {
        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&base))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &base)
            .unwrap();
        let limits = M4EffectiveResourceLimits::new(base, M4ResourceLimits::default()).unwrap();
        let session = StagingSemanticContainerSessionIdentity::fresh();
        let receipt = preflight_staging_math_profile(&package, &limits, &session).unwrap();
        assert_eq!(receipt.math_node_ids(), [NodeId::new(3), NodeId::new(4)]);
        assert!(receipt.canonical_jcs().contains("producer-speech"));
        assert!(receipt.canonical_jcs().contains("svg-safe-1"));
        receipt.verify(&package, &limits, &session).unwrap();
        assert!(matches!(
            crate::preflight_staging_safe_vector_profile(
                &package,
                &limits,
                &StagingSemanticContainerSessionIdentity::fresh(),
            ),
            Err(crate::StagingSafeVectorProfileError::UnsupportedMath)
        ));
        assert!(matches!(
            crate::preflight_staging_semantic_container_profile(
                &package,
                limits.base(),
                &StagingSemanticContainerSessionIdentity::fresh(),
            ),
            Err(crate::StagingSemanticContainerPreflightError::UnsupportedMath)
        ));

        let public =
            crate::encode_capabilities_canonical(crate::HostCapabilityDescriptor::compiled());
        assert!(!public.contains("inline_math"));
        assert!(!public.contains("display_math"));
    }
}
