//! Closed source adapters for shared inline itemization and line projection.
use super::*;
#[derive(Clone, Copy)]
pub(super) enum InlineFlow<'a> {
    Legacy(&'a ProductionTextFlow<'a>),
    #[cfg(feature = "book-v2-staging")]
    BookV2(&'a typaxis_syntax::book_v2::PreparedBookV2TextFlow<'a>),
}
macro_rules! flow_call {
    ($flow:expr, $method:ident ( $($arg:expr),* )) => {
        match $flow {
            InlineFlow::Legacy(flow) => flow.$method($($arg),*),
            #[cfg(feature = "book-v2-staging")]
            InlineFlow::BookV2(flow) => flow.$method($($arg),*),
        }
    };
}
pub(super) use flow_call;

#[derive(Clone, Copy)]
pub(super) enum InlineVectors<'a> {
    Legacy(&'a ValidatedPrecomposedVectorBindings),
    #[cfg(feature = "book-v2-staging")]
    BookV2(&'a crate::book_v2::BookV2VectorBindings<'a>),
}
pub(super) struct InlineVectorBinding<'a> {
    kind: PrecomposedVectorKind,
    owner_source_span: SourceSpan,
    placement: &'a PrecomposedVectorPlacementInput,
    fingerprint: typaxis_layout_contract::PrecomposedVectorBindingFingerprint,
}
impl InlineVectorBinding<'_> {
    pub fn kind(&self) -> PrecomposedVectorKind {
        self.kind
    }
    pub fn owner_source_span(&self) -> SourceSpan {
        self.owner_source_span
    }
    pub fn placement(&self) -> &PrecomposedVectorPlacementInput {
        self.placement
    }
    pub fn binding_fingerprint(
        &self,
    ) -> typaxis_layout_contract::PrecomposedVectorBindingFingerprint {
        self.fingerprint
    }
}
impl<'a> InlineVectors<'a> {
    pub fn receipt(self, owner: NodeId) -> Option<InlineVectorBinding<'a>> {
        match self {
            Self::Legacy(bindings) => {
                let r = bindings.receipt(owner)?;
                Some(InlineVectorBinding {
                    kind: r.kind(),
                    owner_source_span: r.owner_source_span(),
                    placement: r.placement(),
                    fingerprint: r.binding_fingerprint(),
                })
            }
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(bindings) => {
                let r = bindings.receipt(owner)?;
                Some(InlineVectorBinding {
                    kind: r.kind(),
                    owner_source_span: r.owner_source_span(),
                    placement: r.placement(),
                    fingerprint: r.binding_fingerprint(),
                })
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum InlineNativeMath<'a> {
    Legacy(&'a crate::ProductionNativeMathComputations),
    #[cfg(feature = "book-v2-staging")]
    BookV2(&'a crate::book_v2::BookV2NativeMath<'a>),
}
#[derive(Clone, Copy)]
pub(super) enum NativeReceipt<'a> {
    Legacy(&'a crate::ValidatedMathReceipt),
    #[cfg(feature = "book-v2-staging")]
    BookV2(&'a crate::book_v2::BookV2MathReceipt<'a>),
}
impl<'a> InlineNativeMath<'a> {
    pub fn record_charge(self) -> u64 {
        match self {
            Self::Legacy(v) => v.record_charge(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => v.record_charge(),
        }
    }
    pub fn receipt(self, owner: NodeId) -> Option<NativeReceipt<'a>> {
        match self {
            Self::Legacy(v) => v.receipt(owner).map(NativeReceipt::Legacy),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => v.receipt(owner).map(NativeReceipt::BookV2),
        }
    }
    pub fn source_span(self, owner: NodeId) -> Option<SourceSpan> {
        match self {
            Self::Legacy(v) => v.source_span(owner),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => v.source_span(owner),
        }
    }
}
impl NativeReceipt<'_> {
    pub fn fingerprint(self) -> [u8; 32] {
        match self {
            Self::Legacy(v) => v.key().bytes(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => v.fingerprint(),
        }
    }
    pub fn computation(&self) -> &typaxis_math::MathComputationReceipt {
        match self {
            Self::Legacy(v) => v.computation(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => v.computation(),
        }
    }
}

impl InlineFlow<'_> {
    pub(super) fn container_block_style(
        self,
        owner: NodeId,
    ) -> Option<typaxis_style::ComputedMachineBlockStyle> {
        match self {
            Self::Legacy(flow) => flow
                .semantic_container_style(owner)
                .map(|s| s.block_style()),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(flow) => flow
                .semantic_container_style(owner)
                .map(|s| s.block_style()),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum InlineImages<'a> {
    Legacy(&'a AdmittedResourceLedger),
    #[cfg(feature = "book-v2-staging")]
    BookV2(&'a typaxis_resource_admission::AdmittedProductionResourceLedgerV3),
}
impl InlineImages<'_> {
    pub(super) fn image(
        &self,
        id: typaxis_core::ImageResourceId,
    ) -> Option<&typaxis_resource_admission::AdmittedImage> {
        match self {
            Self::Legacy(ledger) => ledger.image(id),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(ledger) => ledger.image(id),
        }
    }
}
