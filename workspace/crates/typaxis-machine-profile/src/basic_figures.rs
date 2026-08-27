use typaxis_core::NodeId;
use typaxis_syntax::{
    StagingFigurePreflightError, ValidatedStagingFigureUsageReceipt, ValidatedStagingStylePackage,
};

use crate::basic_styles::BASIC_DOCUMENT_PROFILE_ID;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentFigurePlacementPolicy {
    NonFloatingBlock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentFigureMediaPolicy {
    DecoderAttestedPng,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentFigureSizePolicy {
    ComputedWidthAndPixelAspectRatioTiesEven,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentFigureCaptionPolicy {
    TypedKeepCaption,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentFigureOversizePolicy {
    TerminalOnce,
}

/// Closed private descriptor for MI2-06. It deliberately has no media string,
/// DPI, float, or caller-selected fit-policy field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BasicDocumentFigureDescriptor;

impl BasicDocumentFigureDescriptor {
    pub const STAGING: Self = Self;

    pub const fn profile_id(self) -> &'static str {
        BASIC_DOCUMENT_PROFILE_ID
    }

    pub const fn placement_policy(self) -> BasicDocumentFigurePlacementPolicy {
        BasicDocumentFigurePlacementPolicy::NonFloatingBlock
    }

    pub const fn media_policy(self) -> BasicDocumentFigureMediaPolicy {
        BasicDocumentFigureMediaPolicy::DecoderAttestedPng
    }

    pub const fn size_policy(self) -> BasicDocumentFigureSizePolicy {
        BasicDocumentFigureSizePolicy::ComputedWidthAndPixelAspectRatioTiesEven
    }

    pub const fn caption_policy(self) -> BasicDocumentFigureCaptionPolicy {
        BasicDocumentFigureCaptionPolicy::TypedKeepCaption
    }

    pub const fn oversize_policy(self) -> BasicDocumentFigureOversizePolicy {
        BasicDocumentFigureOversizePolicy::TerminalOnce
    }

    pub const fn permits_float(self) -> bool {
        false
    }
}

#[derive(Debug)]
struct BasicDocumentFigureBinding;

/// Profile-owned proof that the exact syntax Figure set passed the closed
/// descriptor. Layout receives only its syntax-owned lower projection.
#[derive(Debug)]
pub struct BasicDocumentFigurePreflightReceipt {
    package: [u8; 32],
    descriptor: BasicDocumentFigureDescriptor,
    layout_receipt: ValidatedStagingFigureUsageReceipt,
    _binding: BasicDocumentFigureBinding,
}

impl BasicDocumentFigurePreflightReceipt {
    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.package
    }

    pub const fn profile_id(&self) -> &'static str {
        self.descriptor.profile_id()
    }

    pub const fn layout_receipt(&self) -> &ValidatedStagingFigureUsageReceipt {
        &self.layout_receipt
    }

    pub fn verifies(&self, package: &ValidatedStagingStylePackage) -> bool {
        self.package == package.package_fingerprint().into_bytes()
            && self.descriptor == BasicDocumentFigureDescriptor::STAGING
            && self.layout_receipt.verifies(package)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BasicDocumentFigurePreflight {
    descriptor: BasicDocumentFigureDescriptor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentFigurePreflightFailure {
    FigureUsage(StagingFigurePreflightError),
    ComputedStyle(NodeId),
    UnsupportedFitPolicy(NodeId),
}

impl BasicDocumentFigurePreflight {
    pub const STAGING: Self = Self {
        descriptor: BasicDocumentFigureDescriptor::STAGING,
    };

    pub fn run(
        self,
        package: &ValidatedStagingStylePackage,
    ) -> Result<BasicDocumentFigurePreflightReceipt, BasicDocumentFigurePreflightFailure> {
        let layout_receipt = package
            .preflight_figure_usage()
            .map_err(BasicDocumentFigurePreflightFailure::FigureUsage)?;
        for figure in layout_receipt.figures() {
            let computed = package
                .compute_block_style(figure.owner(), None)
                .map_err(|_| BasicDocumentFigurePreflightFailure::ComputedStyle(figure.owner()))?;
            // There is no float/fit caller field in the closed wire grammar.
            // `keep_with_next` on a Figure is the only otherwise-applicable
            // fit-like request and must be rejected before layout starts.
            if computed.computed().keep_with_next() {
                return Err(BasicDocumentFigurePreflightFailure::UnsupportedFitPolicy(
                    figure.owner(),
                ));
            }
        }
        Ok(BasicDocumentFigurePreflightReceipt {
            package: package.package_fingerprint().into_bytes(),
            descriptor: self.descriptor,
            layout_receipt,
            _binding: BasicDocumentFigureBinding,
        })
    }
}
