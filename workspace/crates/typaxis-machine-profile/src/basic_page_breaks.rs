use typaxis_core::NodeId;
use typaxis_syntax::{
    StagingForcedPageBreakPreflightError, ValidatedStagingForcedPageBreakUsageReceipt,
    ValidatedStagingStylePackage, STAGING_FORCED_PAGE_BREAK_POLICY_VERSION,
    STAGING_FORCED_PAGE_BREAK_USAGE_ALGORITHM,
};

use crate::BASIC_DOCUMENT_PROFILE_ID;

pub const BASIC_FORCED_PAGE_BREAK_POLICY_VERSION: &str = STAGING_FORCED_PAGE_BREAK_POLICY_VERSION;
pub const BASIC_FORCED_PAGE_BREAK_USAGE_ALGORITHM: &str = STAGING_FORCED_PAGE_BREAK_USAGE_ALGORITHM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentBlankPagePolicy {
    PreserveLeadingConsecutiveAndTrailing,
}

impl BasicDocumentBlankPagePolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreserveLeadingConsecutiveAndTrailing => {
                "preserve_leading_consecutive_and_trailing"
            }
        }
    }
}

/// Closed policy descriptor for the private `basic-document-1` staging
/// profile. Public `paragraph-1` capability data continues to reject
/// `page_break` and does not serialize this descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BasicDocumentForcedPageBreakDescriptor;

impl BasicDocumentForcedPageBreakDescriptor {
    pub const STAGING: Self = Self;

    pub const fn profile_id(self) -> &'static str {
        BASIC_DOCUMENT_PROFILE_ID
    }

    pub const fn policy_version(self) -> &'static str {
        BASIC_FORCED_PAGE_BREAK_POLICY_VERSION
    }

    pub const fn blank_page_policy(self) -> BasicDocumentBlankPagePolicy {
        BasicDocumentBlankPagePolicy::PreserveLeadingConsecutiveAndTrailing
    }

    pub const fn starts_with_open_page(self) -> bool {
        true
    }

    pub const fn cursor_advances_per_break(self) -> u8 {
        1
    }

    pub const fn emits_display_paint(self) -> bool {
        false
    }
}

#[derive(Debug)]
struct BasicDocumentForcedPageBreakBinding;

/// Profile-owned wrapper around syntax's package-bound break usage proof.
/// The lower receipt is projected to layout without adding a reverse
/// dependency from layout or pagination to the profile crate.
#[derive(Debug)]
pub struct BasicDocumentForcedPageBreakPreflightReceipt {
    profile_id: &'static str,
    descriptor: BasicDocumentForcedPageBreakDescriptor,
    layout_receipt: ValidatedStagingForcedPageBreakUsageReceipt,
    _binding: BasicDocumentForcedPageBreakBinding,
}

impl BasicDocumentForcedPageBreakPreflightReceipt {
    pub const fn profile_id(&self) -> &'static str {
        self.profile_id
    }

    pub const fn policy_version(&self) -> &'static str {
        self.descriptor.policy_version()
    }

    pub const fn blank_page_policy(&self) -> BasicDocumentBlankPagePolicy {
        self.descriptor.blank_page_policy()
    }

    pub const fn usage_sha256(&self) -> [u8; 32] {
        self.layout_receipt.usage_sha256()
    }

    pub fn break_owners(&self) -> impl ExactSizeIterator<Item = NodeId> + '_ {
        self.layout_receipt
            .breaks()
            .iter()
            .map(|boundary| boundary.owner())
    }

    pub const fn layout_receipt(&self) -> &ValidatedStagingForcedPageBreakUsageReceipt {
        &self.layout_receipt
    }

    pub fn verifies(&self, package: &ValidatedStagingStylePackage) -> bool {
        self.profile_id == BASIC_DOCUMENT_PROFILE_ID
            && self.descriptor == BasicDocumentForcedPageBreakDescriptor::STAGING
            && self.layout_receipt.verifies(package)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BasicDocumentForcedPageBreakPreflight {
    descriptor: BasicDocumentForcedPageBreakDescriptor,
}

impl BasicDocumentForcedPageBreakPreflight {
    pub const STAGING: Self = Self {
        descriptor: BasicDocumentForcedPageBreakDescriptor::STAGING,
    };

    pub fn run(
        self,
        package: &ValidatedStagingStylePackage,
    ) -> Result<BasicDocumentForcedPageBreakPreflightReceipt, StagingForcedPageBreakPreflightError>
    {
        let layout_receipt = package.preflight_forced_page_break_usage()?;
        Ok(BasicDocumentForcedPageBreakPreflightReceipt {
            profile_id: self.descriptor.profile_id(),
            descriptor: self.descriptor,
            layout_receipt,
            _binding: BasicDocumentForcedPageBreakBinding,
        })
    }
}
