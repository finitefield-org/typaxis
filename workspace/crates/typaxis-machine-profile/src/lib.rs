#![forbid(unsafe_code)]

//! Immutable machine-PDF profiles and their pre-layout capability gate.
//!
//! [`MachineProfileDescriptor::PARAGRAPH_1`] is the sole definition of the
//! first machine-PDF profile. Capability serialization and preflight both read
//! that descriptor, so advertising a feature and accepting it cannot drift
//! into independent lists.

mod capabilities;
mod descriptor;
mod preflight;

#[cfg(test)]
mod tests;

pub use capabilities::{encode_capabilities_canonical, HostCapabilityDescriptor};
pub use descriptor::{
    FootnoteCapability, MachineBlockKind, MachineFontFormat, MachineImageFormat, MachineInlineKind,
    MachinePageFrame, MachinePageMasterCapability, MachinePageValue, MachinePdfFeature,
    MachineProfileDescriptor, MachineReferenceFormat, MachineSourceClosure, MachineStyleProperty,
    SourceCountBounds,
};
pub use preflight::{
    HostCapabilityPreflightError, MachinePdfPreflight, MachinePdfPreflightFailure,
    MachinePdfPreflightReceipt, MachinePdfReceiptMismatch,
};
pub use typaxis_core::{MachineInputLimitBounds, MachinePdfProfileId};
