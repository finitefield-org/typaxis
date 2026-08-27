use typaxis_syntax::{
    StagingLinkPreflightError, ValidatedStagingLinkUsageReceipt, ValidatedStagingStylePackage,
    STAGING_BASIC_LINK_POLICY_VERSION, STAGING_LINK_USAGE_ALGORITHM,
};

use crate::BASIC_DOCUMENT_PROFILE_ID;

pub const BASIC_LINK_POLICY_VERSION: &str = STAGING_BASIC_LINK_POLICY_VERSION;
pub const BASIC_LINK_USAGE_ALGORITHM: &str = STAGING_LINK_USAGE_ALGORITHM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentLinkTargetPolicy {
    PackageAnchorOrSafeUri,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentLinkRectanglePolicy {
    CanonicalVisualClusterUnionPerPageLine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentEmptyLinkPolicy {
    RejectBeforeLayout,
}

/// Closed private MI2-07 descriptor. URI schemes come from the already
/// validated effective configuration; this descriptor never accepts a raw
/// action dictionary, arbitrary PDF destination syntax, or nested links.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BasicDocumentLinkDescriptor;

impl BasicDocumentLinkDescriptor {
    pub const STAGING: Self = Self;

    pub const fn profile_id(self) -> &'static str {
        BASIC_DOCUMENT_PROFILE_ID
    }

    pub const fn policy_version(self) -> &'static str {
        BASIC_LINK_POLICY_VERSION
    }

    pub const fn target_policy(self) -> BasicDocumentLinkTargetPolicy {
        BasicDocumentLinkTargetPolicy::PackageAnchorOrSafeUri
    }

    pub const fn rectangle_policy(self) -> BasicDocumentLinkRectanglePolicy {
        BasicDocumentLinkRectanglePolicy::CanonicalVisualClusterUnionPerPageLine
    }

    pub const fn empty_link_policy(self) -> BasicDocumentEmptyLinkPolicy {
        BasicDocumentEmptyLinkPolicy::RejectBeforeLayout
    }

    pub const fn permits_nested_links(self) -> bool {
        false
    }

    pub const fn permits_raw_pdf_actions(self) -> bool {
        false
    }
}

#[derive(Debug)]
struct BasicDocumentLinkBinding;

/// Profile-owned proof that syntax's complete link/anchor projection passed
/// the immutable staging descriptor.
#[derive(Debug)]
pub struct BasicDocumentLinkPreflightReceipt {
    package: [u8; 32],
    descriptor: BasicDocumentLinkDescriptor,
    cluster_receipt: ValidatedStagingLinkUsageReceipt,
    _binding: BasicDocumentLinkBinding,
}

impl BasicDocumentLinkPreflightReceipt {
    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.package
    }

    pub const fn profile_id(&self) -> &'static str {
        self.descriptor.profile_id()
    }

    pub const fn policy_version(&self) -> &'static str {
        self.descriptor.policy_version()
    }

    pub const fn cluster_receipt(&self) -> &ValidatedStagingLinkUsageReceipt {
        &self.cluster_receipt
    }

    pub fn verifies(&self, package: &ValidatedStagingStylePackage) -> bool {
        self.package == package.package_fingerprint().into_bytes()
            && self.descriptor == BasicDocumentLinkDescriptor::STAGING
            && self.cluster_receipt.verifies(package)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BasicDocumentLinkPreflight {
    descriptor: BasicDocumentLinkDescriptor,
}

impl BasicDocumentLinkPreflight {
    pub const STAGING: Self = Self {
        descriptor: BasicDocumentLinkDescriptor::STAGING,
    };

    pub fn run(
        self,
        package: &ValidatedStagingStylePackage,
    ) -> Result<BasicDocumentLinkPreflightReceipt, StagingLinkPreflightError> {
        let cluster_receipt = package.preflight_link_usage()?;
        Ok(BasicDocumentLinkPreflightReceipt {
            package: package.package_fingerprint().into_bytes(),
            descriptor: self.descriptor,
            cluster_receipt,
            _binding: BasicDocumentLinkBinding,
        })
    }
}
