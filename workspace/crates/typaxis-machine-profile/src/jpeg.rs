use typaxis_core::{push_jcs_string, sha256, ImageResourceId, M4EffectiveResourceLimits};
use typaxis_syntax::{
    StagingJpegProfileView, StagingSemanticSyntaxError, ValidatedStagingSemanticPackage,
};

pub const STAGING_JPEG_PROFILE_ID: &str =
    crate::semantic_container::STAGING_PRODUCTION_BOOK_PROFILE_ID;
pub const STAGING_JPEG_PROFILE_ALGORITHM: &str = "typaxis.production-book-jpeg-profile/1";
pub const STAGING_JPEG_RESOURCE_PROFILE_ID: &str = "typaxis.resource-profile/jpeg-baseline/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingJpegProfileDescriptor;

impl StagingJpegProfileDescriptor {
    pub const STAGING: Self = Self;

    pub const fn profile_id(self) -> &'static str {
        STAGING_JPEG_PROFILE_ID
    }
    pub const fn resource_profile_id(self) -> &'static str {
        STAGING_JPEG_RESOURCE_PROFILE_ID
    }
    pub const fn image_media(self) -> &'static str {
        "jpeg-baseline"
    }
    pub const fn placement_policy(self) -> &'static str {
        "non-floating-image-only-figure/1"
    }
    pub const fn sizing_policy(self) -> &'static str {
        "body-width-pixel-aspect-ties-even/1"
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingJpegProfileError {
    Syntax(StagingSemanticSyntaxError),
    ReceiptMismatch,
}

impl std::fmt::Display for StagingJpegProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax(error) => write!(formatter, "{error}"),
            Self::ReceiptMismatch => formatter.write_str("I9190: JPEG profile receipt mismatch"),
        }
    }
}

impl std::error::Error for StagingJpegProfileError {}

impl From<StagingSemanticSyntaxError> for StagingJpegProfileError {
    fn from(value: StagingSemanticSyntaxError) -> Self {
        Self::Syntax(value)
    }
}

/// Machine-profile-owned proof that JPEG was advertised for this exact
/// package before resource opening.  Downstream crates receive only the
/// syntax-owned authorization projection.
#[derive(Debug)]
pub struct StagingJpegProfileReceipt {
    descriptor: StagingJpegProfileDescriptor,
    authorization: StagingJpegProfileView,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingJpegProfileReceipt {
    pub const fn descriptor(&self) -> StagingJpegProfileDescriptor {
        self.descriptor
    }
    pub const fn authorization(&self) -> &StagingJpegProfileView {
        &self.authorization
    }
    pub fn resource_ids(&self) -> &[ImageResourceId] {
        self.authorization.jpeg_resource_ids()
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
    ) -> Result<(), StagingJpegProfileError> {
        self.authorization.authorizes(package, limits)?;
        let expected = encode_profile(&self.authorization, self.descriptor);
        if self.descriptor != StagingJpegProfileDescriptor::STAGING
            || self.canonical_jcs != expected
            || self.fingerprint != sha256(expected.as_bytes())
        {
            return Err(StagingJpegProfileError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn preflight_staging_jpeg_profile(
    package: &ValidatedStagingSemanticPackage,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingJpegProfileReceipt, StagingJpegProfileError> {
    let authorization = StagingJpegProfileView::new(package, limits)?;
    let descriptor = StagingJpegProfileDescriptor::STAGING;
    let canonical_jcs = encode_profile(&authorization, descriptor);
    let receipt = StagingJpegProfileReceipt {
        descriptor,
        authorization,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    receipt.verify(package, limits)?;
    Ok(receipt)
}

fn encode_profile(
    authorization: &StagingJpegProfileView,
    descriptor: StagingJpegProfileDescriptor,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_JPEG_PROFILE_ALGORITHM);
    output.push_str(",\"authorization_fingerprint\":");
    push_hash(&mut output, authorization.profile_fingerprint());
    output.push_str(",\"image_media\":");
    push_jcs_string(&mut output, descriptor.image_media());
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, authorization.limits_fingerprint());
    output.push_str(",\"placement_policy\":");
    push_jcs_string(&mut output, descriptor.placement_policy());
    output.push_str(",\"profile_id\":");
    push_jcs_string(&mut output, descriptor.profile_id());
    output.push_str(",\"resource_profile_id\":");
    push_jcs_string(&mut output, descriptor.resource_profile_id());
    output.push_str(",\"sizing_policy\":");
    push_jcs_string(&mut output, descriptor.sizing_policy());
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

    const JPEG_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/jpeg-media/job/document-package.json"
    ));

    fn fixture() -> (ValidatedStagingSemanticPackage, M4EffectiveResourceLimits) {
        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(JPEG_FIXTURE, &DocumentPackageDecodePolicy::new(&base))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &base)
            .unwrap();
        let limits = M4EffectiveResourceLimits::new(base, M4ResourceLimits::default()).unwrap();
        (package, limits)
    }

    #[test]
    fn jpeg_media_profile_preflight_is_explicit_and_receipt_bound() {
        let (package, limits) = fixture();
        let receipt = preflight_staging_jpeg_profile(&package, &limits).unwrap();
        assert_eq!(receipt.descriptor().profile_id(), STAGING_JPEG_PROFILE_ID);
        assert_eq!(
            receipt.descriptor().profile_id(),
            "typaxis.machine-pdf/production-book-1"
        );
        assert_eq!(
            receipt.descriptor().resource_profile_id(),
            STAGING_JPEG_RESOURCE_PROFILE_ID
        );
        assert_eq!(receipt.descriptor().image_media(), "jpeg-baseline");
        assert_eq!(
            receipt.resource_ids(),
            [
                ImageResourceId::new(0),
                ImageResourceId::new(1),
                ImageResourceId::new(2)
            ]
        );
        assert_eq!(receipt.authorization().figures().len(), 3);
        receipt.verify(&package, &limits).unwrap();

        let mut altered_base = ResourceLimits::default();
        altered_base.max_pages -= 1;
        let altered = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(altered_base).unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap();
        assert!(receipt.verify(&package, &altered).is_err());
    }
}
