#![forbid(unsafe_code)]

//! Immutable machine-PDF profiles and their pre-layout capability gate.
//!
//! [`MachineProfileDescriptor::PARAGRAPH_1`] is the sole definition of the
//! first machine-PDF profile. Capability serialization and preflight both read
//! that descriptor, so advertising a feature and accepting it cannot drift
//! into independent lists.

mod advanced_columns;
mod advanced_float;
mod advanced_header_footer;
mod basic_figures;
mod basic_links;
mod basic_lists;
mod basic_page_breaks;
mod basic_styles;
mod book_navigation;
mod capabilities;
mod descriptor;
mod jpeg;
mod math;
mod preflight;
mod safe_vector;
mod semantic_container;
mod tagged_pdf;

#[cfg(test)]
mod tests;

pub use advanced_columns::{
    preflight_staging_columns_profile, StagingColumnsPreflightError,
    StagingColumnsPreflightReceipt, StagingColumnsProfileDescriptor, StagingColumnsSessionIdentity,
    COLUMNS_PROFILE_RECEIPT_ALGORITHM, STAGING_COLUMNS_PROFILE_ID,
};
pub use advanced_float::{
    preflight_staging_float_profile, StagingFloatPlacementClass, StagingFloatPreflightError,
    StagingFloatPreflightReceipt, StagingFloatProfileDescriptor, StagingFloatSessionIdentity,
    FLOAT_PROFILE_RECEIPT_ALGORITHM, STAGING_FLOAT_PROFILE_ID,
};
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
pub use book_navigation::{
    preflight_staging_book_navigation_profile, preflight_staging_book_navigation_profile_v2,
    StagingBookNavigationProfileDescriptor, StagingBookNavigationProfileDescriptorV2,
    StagingBookNavigationProfileError, StagingBookNavigationProfileReceipt,
    StagingBookNavigationProfileReceiptV2, STAGING_BOOK_NAVIGATION_PROFILE_ALGORITHM,
    STAGING_BOOK_NAVIGATION_PROFILE_ALGORITHM_V2,
};
pub use capabilities::{encode_capabilities_canonical, HostCapabilityDescriptor};
pub use descriptor::{
    FootnoteCapability, MachineBlockKind, MachineCoarseImageFormat, MachineFontFormat,
    MachineImageFormat, MachineInlineKind, MachinePageFrame, MachinePageMasterCapability,
    MachinePageValue, MachinePdfFeature, MachineProfileDescriptor, MachineReferenceFormat,
    MachineSourceClosure, MachineStyleProperty, MachineVectorBlockKind, MachineVectorFeature,
    MachineVectorFeaturesByProfile, MachineVectorFormat, MachineVectorInlineKind,
    MachineVectorKind, MachineVectorMediaByKind, MachineVectorMetric, MachineVectorProfile,
    PrecomposedVectorCapabilityProjection, SourceCountBounds,
};
pub use jpeg::{
    preflight_staging_jpeg_profile, StagingJpegProfileDescriptor, StagingJpegProfileError,
    StagingJpegProfileReceipt, STAGING_JPEG_PROFILE_ALGORITHM, STAGING_JPEG_PROFILE_ID,
    STAGING_JPEG_RESOURCE_PROFILE_ID,
};
pub use math::{
    preflight_staging_math_profile, StagingMathProfileError, StagingMathProfileReceipt,
    STAGING_MATH_PROFILE_ALGORITHM,
};
pub use preflight::{
    preflight_advanced_machine_pdf, AdvancedMachinePdfPreflightError, HostCapabilityPreflightError,
    MachinePdfPreflight, MachinePdfPreflightFailure, MachinePdfPreflightReceipt,
    MachinePdfReceiptMismatch, BASIC_PROFILE_RECEIPT_ALGORITHM, TABLE_PROFILE_RECEIPT_ALGORITHM,
};
pub use safe_vector::{
    preflight_staging_precomposed_vector_profile, preflight_staging_safe_vector_profile,
    StagingPrecomposedVectorProfileDescriptor, StagingPrecomposedVectorProfileError,
    StagingPrecomposedVectorProfileReceipt, StagingSafeVectorProfileError,
    StagingSafeVectorProfileReceipt, STAGING_PRECOMPOSED_VECTOR_PROFILE_ALGORITHM,
    STAGING_PRODUCTION_BOOK_RESOURCE_SET_V2, STAGING_SAFE_VECTOR_PROFILE_ALGORITHM,
    STAGING_SAFE_VECTOR_PROFILE_V2,
};
pub use semantic_container::{
    preflight_staging_semantic_container_profile, StagingSemanticContainerParentKind,
    StagingSemanticContainerPreflightError, StagingSemanticContainerPreflightReceipt,
    StagingSemanticContainerProfileDescriptor, StagingSemanticContainerSessionIdentity,
    STAGING_PRODUCTION_BOOK_PROFILE_ID, STAGING_PRODUCTION_BOOK_PROFILE_RECEIPT_ALGORITHM,
};
pub use tagged_pdf::{
    preflight_staging_tagged_pdf_profile, preflight_staging_tagged_pdf_profile_v2,
    StagingTaggedPdfProfileDescriptor, StagingTaggedPdfProfileDescriptorV2,
    StagingTaggedPdfProfileError, StagingTaggedPdfProfileReceipt, StagingTaggedPdfProfileReceiptV2,
    STAGING_PDFUA1_PROFILE_ID, STAGING_PDFUA1_PROFILE_ID_V2, STAGING_TAGGED_PDF_PROFILE_ALGORITHM,
    STAGING_TAGGED_PDF_PROFILE_ALGORITHM_V2,
};
pub use typaxis_core::{MachineInputLimitBounds, MachinePdfProfileId};
