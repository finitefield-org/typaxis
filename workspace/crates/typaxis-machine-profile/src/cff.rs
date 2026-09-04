use typaxis_core::{push_jcs_string, sha256, FontFaceId, M4EffectiveResourceLimits};
use typaxis_syntax::{
    StagingCffProfileView, StagingSemanticSyntaxError, ValidatedStagingSemanticPackage,
};

pub const STAGING_CFF_PROFILE_ID: &str =
    crate::semantic_container::STAGING_PRODUCTION_BOOK_PROFILE_ID;
pub const STAGING_CFF_PROFILE_ALGORITHM: &str = "typaxis.production-book-cff-profile/1";
pub const STAGING_CFF_RESOURCE_PROFILE_ID: &str = "typaxis.resource-profile/sfnt-cff1/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingCffProfileDescriptor;

impl StagingCffProfileDescriptor {
    pub const STAGING: Self = Self;

    pub const fn profile_id(self) -> &'static str {
        STAGING_CFF_PROFILE_ID
    }
    pub const fn resource_profile_id(self) -> &'static str {
        STAGING_CFF_RESOURCE_PROFILE_ID
    }
    pub const fn font_media(self) -> &'static str {
        "sfnt-cff1"
    }
    pub const fn admission(self) -> &'static str {
        "typaxis.sfnt-cff1-admission/1"
    }
    pub const fn evaluator(self) -> &'static str {
        "typaxis.cff1-charstring-evaluator/1"
    }
    pub const fn glyph_closure(self) -> &'static str {
        "typaxis.cff1-glyph-closure/1"
    }
    pub const fn embedding_permission(self) -> &'static str {
        "typaxis.cff1-embedding-permission/1"
    }
    pub const fn subsetter(self) -> &'static str {
        "typaxis.cff1-subset/1"
    }
    pub const fn pdf_plan(self) -> &'static str {
        "typaxis.cff1-pdf-plan/1"
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingCffProfileError {
    Syntax(StagingSemanticSyntaxError),
    ReceiptMismatch,
}

impl std::fmt::Display for StagingCffProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax(error) => write!(formatter, "{error}"),
            Self::ReceiptMismatch => formatter.write_str("I9190: CFF profile receipt mismatch"),
        }
    }
}

impl std::error::Error for StagingCffProfileError {}

impl From<StagingSemanticSyntaxError> for StagingCffProfileError {
    fn from(value: StagingSemanticSyntaxError) -> Self {
        Self::Syntax(value)
    }
}

/// Machine-profile proof that the private production profile admitted every
/// CFF declaration before the host resource capability is exercised.
#[derive(Debug)]
pub struct StagingCffProfileReceipt {
    descriptor: StagingCffProfileDescriptor,
    authorization: StagingCffProfileView,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingCffProfileReceipt {
    pub const fn descriptor(&self) -> StagingCffProfileDescriptor {
        self.descriptor
    }
    pub const fn authorization(&self) -> &StagingCffProfileView {
        &self.authorization
    }
    pub fn font_face_ids(&self) -> &[FontFaceId] {
        self.authorization.font_face_ids()
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingCffProfileError> {
        self.authorization.authorizes(package, limits)?;
        let expected = encode_profile(&self.authorization, self.descriptor);
        if self.descriptor != StagingCffProfileDescriptor::STAGING
            || self.canonical_jcs != expected
            || self.fingerprint != sha256(expected.as_bytes())
        {
            return Err(StagingCffProfileError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn preflight_staging_cff_profile(
    package: &ValidatedStagingSemanticPackage,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingCffProfileReceipt, StagingCffProfileError> {
    let authorization = StagingCffProfileView::new(package, limits)?;
    let descriptor = StagingCffProfileDescriptor::STAGING;
    let canonical_jcs = encode_profile(&authorization, descriptor);
    let receipt = StagingCffProfileReceipt {
        descriptor,
        authorization,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    receipt.verify(package, limits)?;
    Ok(receipt)
}

fn encode_profile(
    authorization: &StagingCffProfileView,
    descriptor: StagingCffProfileDescriptor,
) -> String {
    let mut output = String::from("{\"admission\":");
    push_jcs_string(&mut output, descriptor.admission());
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_CFF_PROFILE_ALGORITHM);
    output.push_str(",\"authorization_fingerprint\":");
    push_hash(&mut output, authorization.profile_fingerprint());
    output.push_str(",\"embedding_permission\":");
    push_jcs_string(&mut output, descriptor.embedding_permission());
    output.push_str(",\"evaluator\":");
    push_jcs_string(&mut output, descriptor.evaluator());
    output.push_str(",\"font_media\":");
    push_jcs_string(&mut output, descriptor.font_media());
    output.push_str(",\"glyph_closure\":");
    push_jcs_string(&mut output, descriptor.glyph_closure());
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, authorization.limits_fingerprint());
    output.push_str(",\"pdf_plan\":");
    push_jcs_string(&mut output, descriptor.pdf_plan());
    output.push_str(",\"profile_id\":");
    push_jcs_string(&mut output, descriptor.profile_id());
    output.push_str(",\"resource_profile_id\":");
    push_jcs_string(&mut output, descriptor.resource_profile_id());
    output.push_str(",\"subsetter\":");
    push_jcs_string(&mut output, descriptor.subsetter());
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
    use typaxis_core::{M4ResourceLimits, ResourceLimits, ValidatedResourceLimits};
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::StagingSemanticPackageParser;

    use super::*;

    const CFF_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/cff-media/job/document-package.json"
    ));

    fn fixture() -> (ValidatedStagingSemanticPackage, M4EffectiveResourceLimits) {
        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(CFF_FIXTURE, &DocumentPackageDecodePolicy::new(&base))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &base)
            .unwrap();
        let limits = M4EffectiveResourceLimits::new(base, M4ResourceLimits::default()).unwrap();
        (package, limits)
    }

    #[test]
    fn font_media_cff_profile_preflight_is_explicit_and_receipt_bound() {
        let (package, limits) = fixture();
        let receipt = preflight_staging_cff_profile(&package, &limits).unwrap();
        let descriptor = receipt.descriptor();
        assert_eq!(descriptor.profile_id(), STAGING_CFF_PROFILE_ID);
        assert_eq!(
            descriptor.resource_profile_id(),
            STAGING_CFF_RESOURCE_PROFILE_ID
        );
        assert_eq!(descriptor.font_media(), "sfnt-cff1");
        assert_eq!(descriptor.admission(), "typaxis.sfnt-cff1-admission/1");
        assert_eq!(
            descriptor.evaluator(),
            "typaxis.cff1-charstring-evaluator/1"
        );
        assert_eq!(descriptor.glyph_closure(), "typaxis.cff1-glyph-closure/1");
        assert_eq!(
            descriptor.embedding_permission(),
            "typaxis.cff1-embedding-permission/1"
        );
        assert_eq!(descriptor.subsetter(), "typaxis.cff1-subset/1");
        assert_eq!(descriptor.pdf_plan(), "typaxis.cff1-pdf-plan/1");
        assert_eq!(receipt.font_face_ids(), [FontFaceId::new(0)]);
        receipt.verify(&package, &limits).unwrap();

        let mut altered = M4ResourceLimits::default();
        altered.max_cff_charstring_operations -= 1;
        let altered = M4EffectiveResourceLimits::new(limits.base().clone(), altered).unwrap();
        assert!(receipt.verify(&package, &altered).is_err());
    }
}
