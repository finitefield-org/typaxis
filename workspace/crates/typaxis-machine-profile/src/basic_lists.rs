use typaxis_core::{GeneratedBufferKey, JsonPointer, NodeId, ValidatedResourceLimits};
use typaxis_diagnostics::{
    DiagnosticBuilder, DiagnosticCode, DiagnosticLocation, MachineDiagnosticBudgetError,
    MachineDiagnosticLender, MachineDiagnosticPhase, Severity, L5100, L5101, T2100, T2101,
};
use typaxis_syntax::{
    PackageGeneratedTextBinding, StagingListMarkerPreflightError,
    ValidatedStagingListMarkerUsageReceipt, ValidatedStagingStylePackage,
};

use crate::BASIC_DOCUMENT_PROFILE_ID;

pub const BASIC_LIST_POLICY_VERSION: &str = typaxis_syntax::STAGING_BASIC_LIST_POLICY_VERSION;
pub const BASIC_LIST_MARKER_USAGE_ALGORITHM: &str =
    typaxis_syntax::STAGING_LIST_MARKER_USAGE_ALGORITHM;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BasicDocumentListKind {
    Ordered,
    Unordered,
}

impl BasicDocumentListKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ordered => "ordered",
            Self::Unordered => "unordered",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentListMarkerAlignment {
    End,
}

/// Closed, non-public list policy for the immutable M2 staging profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BasicDocumentListDescriptor;

impl BasicDocumentListDescriptor {
    pub const STAGING: Self = Self;
    const ACCEPTED_KINDS: &'static [BasicDocumentListKind] = &[
        BasicDocumentListKind::Ordered,
        BasicDocumentListKind::Unordered,
    ];

    pub const fn profile_id(self) -> &'static str {
        BASIC_DOCUMENT_PROFILE_ID
    }

    pub const fn policy_version(self) -> &'static str {
        BASIC_LIST_POLICY_VERSION
    }

    pub const fn accepted_kinds(self) -> &'static [BasicDocumentListKind] {
        Self::ACCEPTED_KINDS
    }

    pub const fn marker_alignment(self) -> BasicDocumentListMarkerAlignment {
        BasicDocumentListMarkerAlignment::End
    }

    pub const fn marker_gap_font_sizes(self) -> u8 {
        1
    }

    pub const fn nested_lists(self) -> bool {
        true
    }

    pub const fn accepts_caller_marker_text(self) -> bool {
        false
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BasicDocumentListMarkerPlan {
    list_owner: NodeId,
    item_owner: NodeId,
    item_index: u32,
    kind: BasicDocumentListKind,
    ordered_value: Option<u32>,
    key: GeneratedBufferKey,
    utf8: String,
}

impl BasicDocumentListMarkerPlan {
    pub const fn list_owner(&self) -> NodeId {
        self.list_owner
    }

    pub const fn item_owner(&self) -> NodeId {
        self.item_owner
    }

    pub const fn item_index(&self) -> u32 {
        self.item_index
    }

    pub const fn kind(&self) -> BasicDocumentListKind {
        self.kind
    }

    pub const fn ordered_value(&self) -> Option<u32> {
        self.ordered_value
    }

    pub const fn key(&self) -> GeneratedBufferKey {
        self.key
    }

    pub fn utf8(&self) -> &str {
        &self.utf8
    }
}

#[derive(Debug)]
struct BasicDocumentListBinding;

/// Package-bound proof that every marker was derived from list semantics and
/// that both generated-text limits were consumed before marker allocation.
#[derive(Debug)]
pub struct BasicDocumentListPreflightReceipt {
    profile_id: &'static str,
    policy_version: &'static str,
    markers: Vec<BasicDocumentListMarkerPlan>,
    layout_receipt: ValidatedStagingListMarkerUsageReceipt,
    _binding: BasicDocumentListBinding,
}

impl BasicDocumentListPreflightReceipt {
    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.layout_receipt.package_fingerprint().into_bytes()
    }

    pub const fn profile_id(&self) -> &'static str {
        self.profile_id
    }

    pub const fn policy_version(&self) -> &'static str {
        self.policy_version
    }

    pub fn markers(&self) -> &[BasicDocumentListMarkerPlan] {
        &self.markers
    }

    pub const fn marker_usage_sha256(&self) -> [u8; 32] {
        self.layout_receipt.marker_usage_sha256()
    }

    pub const fn parsed_text_bytes(&self) -> u64 {
        self.layout_receipt.parsed_text_bytes()
    }

    pub const fn generated_marker_bytes(&self) -> u64 {
        self.layout_receipt.generated_marker_bytes()
    }

    pub const fn max_text_buffer_bytes(&self) -> u32 {
        self.layout_receipt.max_text_buffer_bytes()
    }

    pub const fn max_text_bytes(&self) -> u64 {
        self.layout_receipt.max_text_bytes()
    }

    pub const fn layout_receipt(&self) -> &ValidatedStagingListMarkerUsageReceipt {
        &self.layout_receipt
    }

    pub fn verifies(&self, package: &ValidatedStagingStylePackage) -> bool {
        self.layout_receipt.verifies(package)
            && self.profile_id == BASIC_DOCUMENT_PROFILE_ID
            && self.policy_version == BASIC_LIST_POLICY_VERSION
    }

    /// Rechecks the complete marker usage ledger issued by `GeneratedTextStore`.
    pub fn verifies_generated_text(&self, generated: PackageGeneratedTextBinding<'_>) -> bool {
        self.layout_receipt.verifies_generated_text(generated)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentListPreflightFailure {
    WrongDiagnosticPhase,
    DiagnosticBudget(MachineDiagnosticBudgetError),
    MarkerOverflow { list_owner: NodeId },
    MissingMarkerTextStyle { list_owner: NodeId },
    TextBufferLimit { item_owner: NodeId },
    TextTotalLimit,
    ArithmeticOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BasicDocumentListPreflight {
    descriptor: BasicDocumentListDescriptor,
}

impl BasicDocumentListPreflight {
    pub const STAGING: Self = Self {
        descriptor: BasicDocumentListDescriptor::STAGING,
    };

    pub fn run(
        self,
        package: &ValidatedStagingStylePackage,
        limits: &ValidatedResourceLimits,
        diagnostics: &mut MachineDiagnosticLender<'_>,
    ) -> Result<BasicDocumentListPreflightReceipt, BasicDocumentListPreflightFailure> {
        if diagnostics.phase() != MachineDiagnosticPhase::Capability {
            return Err(BasicDocumentListPreflightFailure::WrongDiagnosticPhase);
        }
        let layout_receipt = match package.preflight_list_marker_usage(limits) {
            Ok(receipt) => receipt,
            Err(
                StagingListMarkerPreflightError::ArithmeticOverflow
                | StagingListMarkerPreflightError::AllocationFailure,
            ) => return Err(BasicDocumentListPreflightFailure::ArithmeticOverflow),
            Err(error) => {
                let (owner, code, message, failure) = match error {
                    StagingListMarkerPreflightError::MarkerOverflow { list_owner } => (
                        list_owner,
                        L5100,
                        "ordered list marker exceeds the selected machine PDF numeric domain",
                        BasicDocumentListPreflightFailure::MarkerOverflow { list_owner },
                    ),
                    StagingListMarkerPreflightError::MissingMarkerTextStyle { list_owner } => (
                        list_owner,
                        L5101,
                        "list marker requires a complete computed text style",
                        BasicDocumentListPreflightFailure::MissingMarkerTextStyle { list_owner },
                    ),
                    StagingListMarkerPreflightError::TextBufferLimit { item_owner } => (
                        item_owner,
                        T2100,
                        "generated list marker exceeds the configured text-buffer limit",
                        BasicDocumentListPreflightFailure::TextBufferLimit { item_owner },
                    ),
                    StagingListMarkerPreflightError::TextTotalLimit => (
                        package.package().package().document.node_id,
                        T2101,
                        "generated list markers exceed the configured aggregate text limit",
                        BasicDocumentListPreflightFailure::TextTotalLimit,
                    ),
                    StagingListMarkerPreflightError::ArithmeticOverflow
                    | StagingListMarkerPreflightError::AllocationFailure => unreachable!(),
                };
                emit_list_diagnostic(package, owner, code, message, diagnostics)?;
                return Err(failure);
            }
        };
        let mut markers = Vec::new();
        markers
            .try_reserve_exact(layout_receipt.markers().len())
            .map_err(|_| BasicDocumentListPreflightFailure::ArithmeticOverflow)?;
        for marker in layout_receipt.markers() {
            markers.push(BasicDocumentListMarkerPlan {
                list_owner: marker.list_owner(),
                item_owner: marker.item_owner(),
                item_index: marker.item_index(),
                kind: if marker.is_ordered() {
                    BasicDocumentListKind::Ordered
                } else {
                    BasicDocumentListKind::Unordered
                },
                ordered_value: marker.ordered_value(),
                key: marker.key(),
                utf8: marker.utf8().to_owned(),
            });
        }
        Ok(BasicDocumentListPreflightReceipt {
            profile_id: self.descriptor.profile_id(),
            policy_version: self.descriptor.policy_version(),
            markers,
            layout_receipt,
            _binding: BasicDocumentListBinding,
        })
    }
}

fn emit_list_diagnostic(
    package: &ValidatedStagingStylePackage,
    owner: NodeId,
    code: DiagnosticCode,
    message: &'static str,
    diagnostics: &mut MachineDiagnosticLender<'_>,
) -> Result<(), BasicDocumentListPreflightFailure> {
    let parsed = package.package().package();
    let pointer = package
        .locations()
        .node(owner.get(), 0)
        .unwrap_or_else(|| JsonPointer::root().child("document"));
    let uri = parsed
        .sources
        .records()
        .first()
        .expect("staging syntax enforces one source")
        .uri()
        .clone();
    let diagnostic = DiagnosticBuilder::located(
        code,
        Severity::Error,
        message,
        DiagnosticLocation::package_json(uri, pointer, None),
    )
    .expect("static list diagnostics are canonical")
    .build();
    let _ = diagnostics
        .emit_error_with(|| diagnostic)
        .map_err(BasicDocumentListPreflightFailure::DiagnosticBudget)?;
    Ok(())
}
