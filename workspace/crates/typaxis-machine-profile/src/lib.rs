#![forbid(unsafe_code)]

//! Immutable machine-PDF profiles and their pre-layout capability gate.
//!
//! [`MachineProfileDescriptor::PARAGRAPH_1`] is the sole definition of the
//! first machine-PDF profile. Capability serialization and preflight both read
//! that descriptor, so advertising a feature and accepting it cannot drift
//! into independent lists.

mod advanced_header_footer;
mod basic_figures;
mod basic_links;
mod basic_lists;
mod basic_page_breaks;
mod basic_styles;
mod capabilities;
mod descriptor;
mod preflight;

#[cfg(test)]
mod tests;

pub use advanced_header_footer::{
    preflight_staging_header_footer_profile, StagingHeaderFooterPreflightError,
    StagingHeaderFooterPreflightReceipt, StagingHeaderFooterProfileDescriptor,
    StagingHeaderFooterSessionIdentity, StagingMasterSelectionCapability,
    HEADER_FOOTER_PROFILE_RECEIPT_ALGORITHM, STAGING_HEADER_FOOTER_PROFILE_ID,
};
pub use basic_figures::{
    BasicDocumentFigureCaptionPolicy, BasicDocumentFigureDescriptor,
    BasicDocumentFigureMediaPolicy, BasicDocumentFigureOversizePolicy,
    BasicDocumentFigurePlacementPolicy, BasicDocumentFigurePreflight,
    BasicDocumentFigurePreflightFailure, BasicDocumentFigurePreflightReceipt,
    BasicDocumentFigureSizePolicy,
};
pub use basic_links::{
    BasicDocumentEmptyLinkPolicy, BasicDocumentLinkDescriptor, BasicDocumentLinkPreflight,
    BasicDocumentLinkPreflightReceipt, BasicDocumentLinkRectanglePolicy,
    BasicDocumentLinkTargetPolicy, BASIC_LINK_POLICY_VERSION, BASIC_LINK_USAGE_ALGORITHM,
};
pub use basic_lists::{
    BasicDocumentListDescriptor, BasicDocumentListKind, BasicDocumentListMarkerAlignment,
    BasicDocumentListMarkerPlan, BasicDocumentListPreflight, BasicDocumentListPreflightFailure,
    BasicDocumentListPreflightReceipt, BASIC_LIST_MARKER_USAGE_ALGORITHM,
    BASIC_LIST_POLICY_VERSION,
};
pub use basic_page_breaks::{
    BasicDocumentBlankPagePolicy, BasicDocumentForcedPageBreakDescriptor,
    BasicDocumentForcedPageBreakPreflight, BasicDocumentForcedPageBreakPreflightReceipt,
    BASIC_FORCED_PAGE_BREAK_POLICY_VERSION, BASIC_FORCED_PAGE_BREAK_USAGE_ALGORITHM,
};
pub use basic_styles::{
    BasicDocumentStyleDescriptor, BasicDocumentStylePreflight, BasicDocumentStylePreflightFailure,
    BasicDocumentStylePreflightReceipt, BASIC_DOCUMENT_PROFILE_ID,
};
pub use capabilities::{encode_capabilities_canonical, HostCapabilityDescriptor};
pub use descriptor::{
    FootnoteCapability, MachineBlockKind, MachineFontFormat, MachineImageFormat, MachineInlineKind,
    MachinePageFrame, MachinePageMasterCapability, MachinePageValue, MachinePdfFeature,
    MachineProfileDescriptor, MachineReferenceFormat, MachineSourceClosure, MachineStyleProperty,
    SourceCountBounds,
};
pub use preflight::{
    HostCapabilityPreflightError, MachinePdfPreflight, MachinePdfPreflightFailure,
    MachinePdfPreflightReceipt, MachinePdfReceiptMismatch, BASIC_PROFILE_RECEIPT_ALGORITHM,
    TABLE_PROFILE_RECEIPT_ALGORITHM,
};
pub use typaxis_core::{MachineInputLimitBounds, MachinePdfProfileId};
