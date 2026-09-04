#![forbid(unsafe_code)]

mod advanced_columns;
mod advanced_float;
mod advanced_header_footer;
mod book_navigation;
mod book_navigation_v2;
mod math;
mod math_vector;
mod safe_vector;
mod safe_vector_v2;
mod semantic_container;
mod tagged_pdf;
mod tagged_pdf_v2;
mod vector_build_manifest;
#[cfg(test)]
mod vector_v2_fixture;

pub use advanced_columns::project_staging_columns_manifest;
pub use advanced_float::project_staging_float_manifest;
pub use advanced_header_footer::{
    project_staging_header_footer_manifest, StagingAdvancedPaginationManifest,
    StagingAdvancedPaginationManifestError, ADVANCED_PAGINATION_MANIFEST_ALGORITHM,
};
pub use book_navigation::{
    build_staging_book_navigation_manifest, StagingBookNavigationManifest,
    StagingBookNavigationManifestError, STAGING_BOOK_NAVIGATION_MANIFEST_ALGORITHM,
};
pub use book_navigation_v2::{
    build_staging_book_navigation_manifest_v2, StagingBookNavigationManifestV2,
    StagingBookNavigationManifestV2Error, STAGING_BOOK_NAVIGATION_MANIFEST_V2_ALGORITHM,
};
pub use math::{
    build_staging_math_manifest, StagingMathManifest, StagingMathManifestError,
    StagingMathManifestFact, STAGING_MATH_MANIFEST_ALGORITHM,
};
pub use math_vector::{
    build_staging_math_vector_manifest, StagingMathVectorManifest, StagingMathVectorManifestError,
    StagingMathVectorManifestFact, STAGING_MATH_VECTOR_MANIFEST_ALGORITHM,
};
pub use safe_vector::{
    build_staging_safe_vector_manifest, StagingSafeVectorManifest, StagingSafeVectorManifestError,
    StagingSafeVectorManifestResource, StagingSafeVectorManifestUsage,
    STAGING_SAFE_VECTOR_MANIFEST_ALGORITHM,
};
pub use safe_vector_v2::{
    build_staging_safe_vector_manifest_v2, StagingSafeVectorManifestAliasV2,
    StagingSafeVectorManifestPlacementV2, StagingSafeVectorManifestResourceV2,
    StagingSafeVectorManifestV2, StagingSafeVectorManifestV2Error,
    StagingSafeVectorPlacementDetailsV2, StagingVectorMetricFactV2,
    STAGING_SAFE_VECTOR_MANIFEST_V2_ALGORITHM,
};
pub use semantic_container::{
    build_staging_semantic_container_manifest, StagingDeclaredMediaManifestRecord,
    StagingSemanticContainerManifest, StagingSemanticContainerManifestError,
    StagingSemanticContainerSelectedFact,
};
pub use tagged_pdf::{
    build_staging_tagged_pdf_manifest, StagingTaggedPdfManifest, StagingTaggedPdfManifestError,
    STAGING_TAGGED_PDF_MANIFEST_ALGORITHM,
};
pub use tagged_pdf_v2::{
    build_staging_tagged_pdf_manifest_v2, StagingTaggedPdfManifestV2,
    StagingTaggedPdfManifestV2Error, StagingTaggedPdfVectorStructureFactV2,
    STAGING_TAGGED_PDF_MANIFEST_V2_ALGORITHM,
};
pub use vector_build_manifest::{
    StagingProductionBuildManifestVectorFields, StagingProductionBuildManifestVectorFieldsError,
    StagingVectorBuildStatus,
};

use core::num::{NonZeroU16, NonZeroU64};
use std::collections::BTreeMap;
#[cfg(unix)]
use std::ffi::OsString;
use std::fs;
#[cfg(unix)]
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use typaxis_core::{
    push_generated_buffer_key_jcs, push_jcs_string, AdmittedResourceFingerprint,
    BuildExecutionContext, BuildExecutionError, EffectiveConfig, EffectiveConfigFingerprint,
    EngineIdentity, FontFaceId, FootnoteId, GeneratedBufferKey, HostPath, ImageResourceId,
    LayoutStateFingerprint, MachinePdfProfileId, PortablePath, Rect, ReplacePolicy,
    ResolvedDataTables, ShaperIdentity, SourceId, ValidatedResourceLimits, JSON_SAFE_INTEGER_MAX,
};
pub use typaxis_core::{OutputSink, PdfStreamCompression};
use typaxis_display_list::FootnoteDisplayClosureReceipt;
pub use typaxis_host_admission::HostReadIdentityLedgerToken as PublicationReadLedgerToken;
use typaxis_host_admission::{HostAdmissionError, HostReadIdentityLedgerToken};
use typaxis_layout_contract::{ResolvedTableColumnInput, ValidatedTableGridReceipt};
use typaxis_machine_input::{
    MachineInputFingerprint, MachineInputProgress, MachineInputSessionIdentity, MachineInputStage,
};
use typaxis_machine_profile::{MachinePdfPreflightReceipt, MachineProfileDescriptor};
use typaxis_pagination::{
    ConvergenceStatus, MultiFlowSelectedStateReceipt, MultiFlowTraceFacts, PaginationResult,
    RowspanContinuationReceipt, SelectedTableLayoutReceipt, StagingTableCellFragmentReceipt,
    StagingTableTraceFacts, ValidatedFootnoteSelectedLayout,
};
use typaxis_pdf::{
    PdfStreamWriteFacts, StagingForcedPageBreakPdf, StagingMachineBlockStylePdf,
    StagingMachineFigurePdf, StagingMachineLinkPdf, StagingMachineListPdf, VerifiedPdfBytesReceipt,
};
use typaxis_resources::{AdmittedResourceLedgerToken, ResourceAdmissionProgressToken};
use typaxis_syntax::machine_profile_boundary::Block;
use typaxis_syntax::{PackageEpochIdentity, ValidatedMachinePackage, ValidatedParsedPackage};
use typaxis_text::SourceRecord;

pub const CONTRACT: &str = typaxis_core::CONTRACT;
pub const ENGINE_NAME: &str = typaxis_core::PRODUCT_NAME;
pub const PDF_PROFILE: &str = "pdf-1.7-classic-xref";
pub const REFERENCE_INPUT_PROFILE: &str = "typaxis.reference-source/1";

#[cfg(unix)]
static OUTPUT_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
static OUTPUT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static PUBLICATION_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildStatus {
    Built,
    Failed,
}

/// Closed input identity bound before any terminal manifest can be prepared.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildInputProfile {
    ReferenceSource1,
    MachinePdfBasicDocument1,
    MachinePdfColumns1,
    MachinePdfFloat1,
    MachinePdfFootnote1,
    MachinePdfHeaderFooter1,
    MachinePdfParagraph1,
    MachinePdfTable1,
}

impl BuildInputProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReferenceSource1 => REFERENCE_INPUT_PROFILE,
            Self::MachinePdfBasicDocument1 => MachinePdfProfileId::BASIC_DOCUMENT_1.as_str(),
            Self::MachinePdfColumns1 => MachinePdfProfileId::COLUMNS_1.as_str(),
            Self::MachinePdfFloat1 => MachinePdfProfileId::FLOAT_1.as_str(),
            Self::MachinePdfFootnote1 => MachinePdfProfileId::FOOTNOTE_1.as_str(),
            Self::MachinePdfHeaderFooter1 => MachinePdfProfileId::HEADER_FOOTER_1.as_str(),
            Self::MachinePdfParagraph1 => MachinePdfProfileId::PARAGRAPH_1.as_str(),
            Self::MachinePdfTable1 => MachinePdfProfileId::TABLE_1.as_str(),
        }
    }

    pub const fn machine_profile(self) -> Option<MachinePdfProfileId> {
        match self {
            Self::ReferenceSource1 => None,
            Self::MachinePdfBasicDocument1 => Some(MachinePdfProfileId::BASIC_DOCUMENT_1),
            Self::MachinePdfColumns1 => Some(MachinePdfProfileId::COLUMNS_1),
            Self::MachinePdfFloat1 => Some(MachinePdfProfileId::FLOAT_1),
            Self::MachinePdfFootnote1 => Some(MachinePdfProfileId::FOOTNOTE_1),
            Self::MachinePdfHeaderFooter1 => Some(MachinePdfProfileId::HEADER_FOOTER_1),
            Self::MachinePdfParagraph1 => Some(MachinePdfProfileId::PARAGRAPH_1),
            Self::MachinePdfTable1 => Some(MachinePdfProfileId::TABLE_1),
        }
    }

    fn from_descriptor(descriptor: MachineProfileDescriptor) -> Self {
        match descriptor.id() {
            MachinePdfProfileId::BasicDocument1 => Self::MachinePdfBasicDocument1,
            MachinePdfProfileId::Columns1 => Self::MachinePdfColumns1,
            MachinePdfProfileId::Float1 => Self::MachinePdfFloat1,
            MachinePdfProfileId::Footnote1 => Self::MachinePdfFootnote1,
            MachinePdfProfileId::HeaderFooter1 => Self::MachinePdfHeaderFooter1,
            MachinePdfProfileId::Paragraph1 => Self::MachinePdfParagraph1,
            MachinePdfProfileId::Table1 => Self::MachinePdfTable1,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutStatus {
    Converged,
    CycleFallback,
    MaxPassFallback,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutFallbackPolicy {
    LowestCostThenEarliest,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayoutRecord {
    status: LayoutStatus,
    pass_count: NonZeroU16,
    selected_state: NonZeroU16,
    final_fingerprint: LayoutStateFingerprint,
    fallback_policy: Option<LayoutFallbackPolicy>,
    flow_registry_sha256: Option<[u8; 32]>,
    profile_receipt_sha256: Option<[u8; 32]>,
}
impl LayoutRecord {
    fn new(
        status: LayoutStatus,
        pass_count: NonZeroU16,
        selected_state: NonZeroU16,
        final_fingerprint: LayoutStateFingerprint,
    ) -> Option<Self> {
        if selected_state.get() > pass_count.get()
            || (status == LayoutStatus::Converged && selected_state != pass_count)
        {
            return None;
        }
        let fallback_policy = match status {
            LayoutStatus::Converged => None,
            LayoutStatus::CycleFallback | LayoutStatus::MaxPassFallback => {
                Some(LayoutFallbackPolicy::LowestCostThenEarliest)
            }
        };
        Some(Self {
            status,
            pass_count,
            selected_state,
            final_fingerprint,
            fallback_policy,
            flow_registry_sha256: None,
            profile_receipt_sha256: None,
        })
    }

    fn bind_machine_capability(
        mut self,
        capability: &MachinePdfPreflightReceipt,
        flow_registry_sha256: Option<[u8; 32]>,
    ) -> Result<Self, BuildManifestError> {
        match capability.profile() {
            MachinePdfProfileId::BasicDocument1
            | MachinePdfProfileId::Columns1
            | MachinePdfProfileId::Float1
            | MachinePdfProfileId::Footnote1
            | MachinePdfProfileId::HeaderFooter1
            | MachinePdfProfileId::Table1
                if flow_registry_sha256.is_none() =>
            {
                return Err(BuildManifestError::MachineCapabilityMismatch)
            }
            MachinePdfProfileId::Paragraph1 if flow_registry_sha256.is_some() => {
                return Err(BuildManifestError::MachineCapabilityMismatch)
            }
            MachinePdfProfileId::BasicDocument1
            | MachinePdfProfileId::Columns1
            | MachinePdfProfileId::Float1
            | MachinePdfProfileId::Footnote1
            | MachinePdfProfileId::HeaderFooter1
            | MachinePdfProfileId::Paragraph1
            | MachinePdfProfileId::Table1 => {}
        }
        self.profile_receipt_sha256 = Some(capability.profile_receipt_sha256());
        self.flow_registry_sha256 = flow_registry_sha256;
        Ok(self)
    }

    pub const fn flow_registry_sha256(self) -> Option<[u8; 32]> {
        self.flow_registry_sha256
    }

    pub const fn profile_receipt_sha256(self) -> Option<[u8; 32]> {
        self.profile_receipt_sha256
    }

    fn from_pagination(result: &PaginationResult) -> Result<Self, BuildManifestError> {
        let pass_count = u16::try_from(result.passes().len())
            .ok()
            .and_then(NonZeroU16::new)
            .ok_or(BuildManifestError::LayoutPassCountMismatch)?;
        let selected_state = NonZeroU16::new(result.selected_state().get())
            .ok_or(BuildManifestError::LayoutPassCountMismatch)?;
        let status = match result.status() {
            ConvergenceStatus::Converged => LayoutStatus::Converged,
            ConvergenceStatus::CycleFallback { .. } => LayoutStatus::CycleFallback,
            ConvergenceStatus::MaxPassFallback => LayoutStatus::MaxPassFallback,
        };
        Self::new(
            status,
            pass_count,
            selected_state,
            result.final_fingerprint(),
        )
        .ok_or(BuildManifestError::LayoutPassCountMismatch)
    }
    pub const fn status(self) -> LayoutStatus {
        self.status
    }
    pub const fn pass_count(self) -> NonZeroU16 {
        self.pass_count
    }
    pub const fn selected_state(self) -> NonZeroU16 {
        self.selected_state
    }
    pub const fn final_fingerprint(self) -> LayoutStateFingerprint {
        self.final_fingerprint
    }
    pub const fn fallback_policy(self) -> Option<LayoutFallbackPolicy> {
        self.fallback_policy
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingManifestFlowFact {
    flow_id: u32,
    owner_node_id: u32,
    parent_flow_id: Option<u32>,
    terminal: u32,
}

impl StagingManifestFlowFact {
    pub const fn flow_id(&self) -> u32 {
        self.flow_id
    }

    pub const fn owner_node_id(&self) -> u32 {
        self.owner_node_id
    }

    pub const fn parent_flow_id(&self) -> Option<u32> {
        self.parent_flow_id
    }

    pub const fn terminal(&self) -> u32 {
        self.terminal
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMultiFlowManifestError {
    RegistryMismatch,
    SelectedStateMismatch,
    FlowCountMismatch,
    AllocationFailure,
}

/// Focused 1.2 multi-flow projection retained for slice-level closure tests.
/// It can only be derived from matching trace and selected-state receipts;
/// the public build manifest binds the corresponding registry hash.
#[derive(Debug)]
pub struct StagingMultiFlowLayoutFacts {
    flow_registry_sha256: [u8; 32],
    selected_state_sha256: [u8; 32],
    flows: Vec<StagingManifestFlowFact>,
    canonical_jcs: String,
}

impl StagingMultiFlowLayoutFacts {
    pub fn from_selected(
        trace: &MultiFlowTraceFacts,
        selected: &MultiFlowSelectedStateReceipt,
    ) -> Result<Self, StagingMultiFlowManifestError> {
        if trace.registry_fingerprint() != selected.registry_fingerprint() {
            return Err(StagingMultiFlowManifestError::RegistryMismatch);
        }
        if trace.selected_state_fingerprint() != selected.fingerprint() {
            return Err(StagingMultiFlowManifestError::SelectedStateMismatch);
        }
        let terminal_positions = trace
            .positions()
            .iter()
            .filter(|position| position.is_terminal())
            .count();
        if terminal_positions != selected.terminals().len() {
            return Err(StagingMultiFlowManifestError::FlowCountMismatch);
        }
        let mut flows = Vec::new();
        flows
            .try_reserve_exact(selected.terminals().len())
            .map_err(|_| StagingMultiFlowManifestError::AllocationFailure)?;
        for terminal in selected.terminals() {
            flows.push(StagingManifestFlowFact {
                flow_id: terminal.flow_id().get(),
                owner_node_id: terminal.owner_node_id().get(),
                parent_flow_id: terminal.parent_flow_id().map(|flow_id| flow_id.get()),
                terminal: terminal.terminal(),
            });
        }
        let flow_registry_sha256 = selected.registry_fingerprint().bytes();
        let selected_state_sha256 = selected.fingerprint().bytes();
        let canonical_jcs = encode_staging_multi_flow_layout_facts(
            flow_registry_sha256,
            selected_state_sha256,
            &flows,
        );
        Ok(Self {
            flow_registry_sha256,
            selected_state_sha256,
            flows,
            canonical_jcs,
        })
    }

    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }

    pub const fn selected_state_sha256(&self) -> [u8; 32] {
        self.selected_state_sha256
    }

    pub fn flows(&self) -> &[StagingManifestFlowFact] {
        &self.flows
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

pub const STAGING_TABLE_MANIFEST_ALGORITHM: &str = "typaxis.table-layout-manifest/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingTableManifestError {
    TraceMismatch,
    SelectedStateMismatch,
    GridMismatch,
    FactCountMismatch,
    AllocationFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingTableManifestColumnFact {
    column_ordinal: u32,
    input_kind: &'static str,
    input_value: i64,
    rounded_fraction_width: Option<i64>,
    final_width: i64,
}

impl StagingTableManifestColumnFact {
    pub const fn column_ordinal(self) -> u32 {
        self.column_ordinal
    }
    pub const fn input_kind(self) -> &'static str {
        self.input_kind
    }
    pub const fn input_value(self) -> i64 {
        self.input_value
    }
    pub const fn rounded_fraction_width(self) -> Option<i64> {
        self.rounded_fraction_width
    }
    pub const fn final_width(self) -> i64 {
        self.final_width
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTableManifestCellFact {
    cell_node_id: u32,
    flow_id: u32,
    column_ordinal: u32,
    colspan: u16,
    rowspan: u16,
    selected_block_extent: i64,
    before_fragment_ordinal: u32,
    after_fragment_ordinal: u32,
    before_terminal: bool,
    after_terminal: bool,
    vertical_offset_before: i64,
    vertical_offset_after: i64,
}

impl StagingTableManifestCellFact {
    pub const fn cell_node_id(&self) -> u32 {
        self.cell_node_id
    }
    pub const fn flow_id(&self) -> u32 {
        self.flow_id
    }
    pub const fn column_ordinal(&self) -> u32 {
        self.column_ordinal
    }
    pub const fn colspan(&self) -> u16 {
        self.colspan
    }
    pub const fn rowspan(&self) -> u16 {
        self.rowspan
    }
    pub const fn selected_block_extent(&self) -> i64 {
        self.selected_block_extent
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTableManifestRowFact {
    fragment_id: u64,
    row_node_id: u32,
    logical_row_ordinal: u32,
    row_fragment_ordinal: u32,
    page_index: u32,
    page_block_offset: i64,
    selected_block_extent: i64,
    cells: Vec<StagingTableManifestCellFact>,
    continuation_before: RowspanContinuationReceipt,
    continuation_after: RowspanContinuationReceipt,
}

impl StagingTableManifestRowFact {
    pub const fn fragment_id(&self) -> u64 {
        self.fragment_id
    }
    pub const fn row_node_id(&self) -> u32 {
        self.row_node_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn selected_block_extent(&self) -> i64 {
        self.selected_block_extent
    }
    pub fn cells(&self) -> &[StagingTableManifestCellFact] {
        &self.cells
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTableManifestHeaderSourceFact {
    source_fragment_id: u64,
    row_node_id: u32,
    row_ordinal: u32,
    group_block_offset: i64,
    selected_block_extent: i64,
    cells: Vec<StagingTableManifestCellFact>,
}

impl StagingTableManifestHeaderSourceFact {
    pub const fn source_fragment_id(&self) -> u64 {
        self.source_fragment_id
    }
    pub const fn row_node_id(&self) -> u32 {
        self.row_node_id
    }
    pub fn cells(&self) -> &[StagingTableManifestCellFact] {
        &self.cells
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingTableManifestHeaderOccurrenceFact {
    repetition_index: u32,
    target_page_index: u32,
    fragment_id: u64,
    source_fragment_id: u64,
    row_node_id: u32,
    target_block_offset: i64,
}

impl StagingTableManifestHeaderOccurrenceFact {
    pub const fn repetition_index(self) -> u32 {
        self.repetition_index
    }
    pub const fn target_page_index(self) -> u32 {
        self.target_page_index
    }
    pub const fn fragment_id(self) -> u64 {
        self.fragment_id
    }
    pub const fn source_fragment_id(self) -> u64 {
        self.source_fragment_id
    }
}

/// Private MI3-03 manifest projection. It accepts only a trace/selection pair
/// with identical grid, row-band, flow-registry, and selected-layout hashes;
/// row, cell, and header facts are then copied without geometry recomputation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTableLayoutFacts {
    package_sha256: [u8; 32],
    body_block_size: i64,
    first_page_remaining_block_size: i64,
    flow_registry_sha256: [u8; 32],
    grid_sha256: [u8; 32],
    columns: Vec<StagingTableManifestColumnFact>,
    rounding_residual: i64,
    residual_recipient: Option<u32>,
    row_band_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    table_node_id: u32,
    target_page_start: u32,
    page_count: u32,
    rows: Vec<StagingTableManifestRowFact>,
    header_sources: Vec<StagingTableManifestHeaderSourceFact>,
    header_occurrences: Vec<StagingTableManifestHeaderOccurrenceFact>,
    canonical_jcs: String,
}

impl StagingTableLayoutFacts {
    pub fn from_selected(
        trace: &StagingTableTraceFacts,
        grid: &ValidatedTableGridReceipt,
        selected: &SelectedTableLayoutReceipt,
    ) -> Result<Self, StagingTableManifestError> {
        Self::from_selected_at(trace, grid, selected, 0)
    }

    pub fn from_selected_at(
        trace: &StagingTableTraceFacts,
        grid: &ValidatedTableGridReceipt,
        selected: &SelectedTableLayoutReceipt,
        target_page_start: u32,
    ) -> Result<Self, StagingTableManifestError> {
        if trace.selected_layout_sha256() != selected.fingerprint().bytes() {
            return Err(StagingTableManifestError::SelectedStateMismatch);
        }
        if trace.flow_registry_sha256() != selected.flow_registry_fingerprint().bytes()
            || trace.grid_sha256() != selected.grid_sha256()
            || trace.row_band_sha256() != selected.row_band_sha256()
            || trace.page_count() != selected.page_count()
        {
            return Err(StagingTableManifestError::TraceMismatch);
        }
        if grid.package_sha256() != selected.package_sha256()
            || grid.epoch() != selected.epoch()
            || grid.flow_registry() != selected.flow_registry_fingerprint()
            || grid.fingerprint().bytes() != selected.grid_sha256()
            || grid.table_owner() != selected.table_owner()
        {
            return Err(StagingTableManifestError::GridMismatch);
        }
        if trace.row_fragment_count()
            != u64::try_from(selected.row_fragments().len())
                .map_err(|_| StagingTableManifestError::FactCountMismatch)?
            || trace.header_occurrence_count()
                != selected
                    .header_repetitions()
                    .iter()
                    .try_fold(0u64, |total, receipt| {
                        total.checked_add(u64::try_from(receipt.rows().len()).ok()?)
                    })
                    .ok_or(StagingTableManifestError::FactCountMismatch)?
        {
            return Err(StagingTableManifestError::FactCountMismatch);
        }

        let mut columns = Vec::new();
        columns
            .try_reserve_exact(grid.columns().len())
            .map_err(|_| StagingTableManifestError::AllocationFailure)?;
        for column in grid.columns() {
            let (input_kind, input_value) = match column.input() {
                ResolvedTableColumnInput::Fixed(width) => ("fixed", width.get().raw()),
                ResolvedTableColumnInput::Fraction(weight) => ("fraction", i64::from(weight.get())),
            };
            columns.push(StagingTableManifestColumnFact {
                column_ordinal: column.index(),
                input_kind,
                input_value,
                rounded_fraction_width: column
                    .rounded_fraction_width()
                    .map(|width| width.get().raw()),
                final_width: column.final_width().get().raw(),
            });
        }

        let mut rows = Vec::new();
        rows.try_reserve_exact(selected.row_fragments().len())
            .map_err(|_| StagingTableManifestError::AllocationFailure)?;
        let mut observed_cell_count = 0u64;
        for fragment in selected.row_fragments() {
            let mut cells = Vec::new();
            cells
                .try_reserve_exact(fragment.cells().len())
                .map_err(|_| StagingTableManifestError::AllocationFailure)?;
            for cell in fragment.cells() {
                cells.push(staging_table_manifest_cell(grid, cell)?);
                observed_cell_count = observed_cell_count
                    .checked_add(1)
                    .ok_or(StagingTableManifestError::FactCountMismatch)?;
            }
            rows.push(StagingTableManifestRowFact {
                fragment_id: fragment.fragment_id(),
                row_node_id: fragment.row_owner().get(),
                logical_row_ordinal: fragment.logical_row_ordinal(),
                row_fragment_ordinal: fragment.row_fragment_ordinal(),
                page_index: target_page_start
                    .checked_add(fragment.page_index())
                    .ok_or(StagingTableManifestError::FactCountMismatch)?,
                page_block_offset: fragment.page_block_offset(),
                selected_block_extent: fragment.selected_block_extent(),
                cells,
                continuation_before: fragment.continuation_before().clone(),
                continuation_after: fragment.continuation_after().clone(),
            });
        }

        let mut header_sources = Vec::new();
        header_sources
            .try_reserve_exact(selected.header_sources().len())
            .map_err(|_| StagingTableManifestError::AllocationFailure)?;
        for source in selected.header_sources() {
            let mut cells = Vec::new();
            cells
                .try_reserve_exact(source.cells().len())
                .map_err(|_| StagingTableManifestError::AllocationFailure)?;
            for cell in source.cells() {
                cells.push(staging_table_manifest_cell(grid, cell)?);
                observed_cell_count = observed_cell_count
                    .checked_add(1)
                    .ok_or(StagingTableManifestError::FactCountMismatch)?;
            }
            header_sources.push(StagingTableManifestHeaderSourceFact {
                source_fragment_id: source.source_fragment_id(),
                row_node_id: source.row_owner().get(),
                row_ordinal: source.row_ordinal(),
                group_block_offset: source.group_block_offset(),
                selected_block_extent: source.selected_block_extent(),
                cells,
            });
        }
        if observed_cell_count != trace.cell_fragment_count() {
            return Err(StagingTableManifestError::FactCountMismatch);
        }

        let occurrence_count = usize::try_from(trace.header_occurrence_count())
            .map_err(|_| StagingTableManifestError::FactCountMismatch)?;
        let mut header_occurrences = Vec::new();
        header_occurrences
            .try_reserve_exact(occurrence_count)
            .map_err(|_| StagingTableManifestError::AllocationFailure)?;
        for repetition in selected.header_repetitions() {
            for row in repetition.rows() {
                header_occurrences.push(StagingTableManifestHeaderOccurrenceFact {
                    repetition_index: repetition.repetition_index(),
                    target_page_index: target_page_start
                        .checked_add(repetition.target_page_index())
                        .ok_or(StagingTableManifestError::FactCountMismatch)?,
                    fragment_id: row.fragment_id(),
                    source_fragment_id: row.source_fragment_id(),
                    row_node_id: row.row_owner().get(),
                    target_block_offset: row.target_block_offset(),
                });
            }
        }
        let mut facts = Self {
            package_sha256: selected.package_sha256(),
            body_block_size: selected.body_block_size(),
            first_page_remaining_block_size: selected.first_page_remaining_block_size(),
            flow_registry_sha256: trace.flow_registry_sha256(),
            grid_sha256: trace.grid_sha256(),
            columns,
            rounding_residual: grid.rounding_residual().raw(),
            residual_recipient: grid.residual_recipient(),
            row_band_sha256: trace.row_band_sha256(),
            selected_layout_sha256: trace.selected_layout_sha256(),
            table_node_id: selected.table_owner().get(),
            target_page_start,
            page_count: selected.page_count(),
            rows,
            header_sources,
            header_occurrences,
            canonical_jcs: String::new(),
        };
        facts.canonical_jcs = encode_staging_table_layout_facts(&facts);
        Ok(facts)
    }

    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }
    pub fn columns(&self) -> &[StagingTableManifestColumnFact] {
        &self.columns
    }
    pub const fn rounding_residual(&self) -> i64 {
        self.rounding_residual
    }
    pub const fn residual_recipient(&self) -> Option<u32> {
        self.residual_recipient
    }
    pub const fn body_block_size(&self) -> i64 {
        self.body_block_size
    }
    pub const fn first_page_remaining_block_size(&self) -> i64 {
        self.first_page_remaining_block_size
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn table_node_id(&self) -> u32 {
        self.table_node_id
    }
    pub const fn target_page_start(&self) -> u32 {
        self.target_page_start
    }
    pub fn target_page_end(&self) -> Option<u32> {
        self.target_page_start.checked_add(self.page_count)
    }
    pub fn rows(&self) -> &[StagingTableManifestRowFact] {
        &self.rows
    }
    pub fn header_sources(&self) -> &[StagingTableManifestHeaderSourceFact] {
        &self.header_sources
    }
    pub fn header_occurrences(&self) -> &[StagingTableManifestHeaderOccurrenceFact] {
        &self.header_occurrences
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn staging_table_manifest_cell(
    grid: &ValidatedTableGridReceipt,
    cell: &StagingTableCellFragmentReceipt,
) -> Result<StagingTableManifestCellFact, StagingTableManifestError> {
    let binding = grid
        .cells()
        .iter()
        .find(|binding| binding.cell_owner() == cell.cell_owner())
        .ok_or(StagingTableManifestError::GridMismatch)?;
    if binding.flow_id() != cell.flow_id()
        || cell.before_cursor().flow_id() != cell.flow_id()
        || cell.after_cursor().flow_id() != cell.flow_id()
    {
        return Err(StagingTableManifestError::GridMismatch);
    }
    Ok(StagingTableManifestCellFact {
        cell_node_id: cell.cell_owner().get(),
        flow_id: cell.flow_id().get(),
        column_ordinal: binding.column_ordinal(),
        colspan: binding.colspan().get(),
        rowspan: binding.rowspan().get(),
        selected_block_extent: cell.selected_block_extent(),
        before_fragment_ordinal: cell.before_cursor().next_fragment_ordinal(),
        after_fragment_ordinal: cell.after_cursor().next_fragment_ordinal(),
        before_terminal: cell.before_cursor().is_terminal(),
        after_terminal: cell.after_cursor().is_terminal(),
        vertical_offset_before: cell.vertical_offset_before(),
        vertical_offset_after: cell.vertical_offset_after(),
    })
}

pub const STAGING_FOOTNOTE_MANIFEST_ALGORITHM: &str = "typaxis.footnote-manifest/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingFootnoteManifestError {
    SelectedStateMismatch,
    PaintStateMismatch,
    AllocationFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnoteManifestFlowFact {
    footnote_id: FootnoteId,
    assignment_ordinal: u32,
    flow_id: u32,
    before_fragment: u32,
    after_fragment: u32,
    incoming_source_page: Option<u32>,
    carries_out: bool,
}

impl StagingFootnoteManifestFlowFact {
    pub const fn footnote_id(&self) -> &FootnoteId {
        &self.footnote_id
    }
    pub const fn assignment_ordinal(&self) -> u32 {
        self.assignment_ordinal
    }
    pub const fn flow_id(&self) -> u32 {
        self.flow_id
    }
    pub const fn before_fragment(&self) -> u32 {
        self.before_fragment
    }
    pub const fn after_fragment(&self) -> u32 {
        self.after_fragment
    }
    pub const fn incoming_source_page(&self) -> Option<u32> {
        self.incoming_source_page
    }
    pub const fn carries_out(&self) -> bool {
        self.carries_out
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnoteManifestPageFact {
    page_index: u32,
    body_continuation_position: u32,
    body_continuation_terminal: bool,
    body_fingerprint: [u8; 32],
    evaluation_count: u32,
    reservation: i64,
    ordered_footnote_ids: Vec<FootnoteId>,
    flows: Vec<StagingFootnoteManifestFlowFact>,
}

impl StagingFootnoteManifestPageFact {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn body_continuation_position(&self) -> u32 {
        self.body_continuation_position
    }
    pub const fn body_continuation_terminal(&self) -> bool {
        self.body_continuation_terminal
    }
    pub const fn body_fingerprint(&self) -> [u8; 32] {
        self.body_fingerprint
    }
    pub const fn evaluation_count(&self) -> u32 {
        self.evaluation_count
    }
    pub const fn reservation(&self) -> i64 {
        self.reservation
    }
    pub fn ordered_footnote_ids(&self) -> &[FootnoteId] {
        &self.ordered_footnote_ids
    }
    pub fn flows(&self) -> &[StagingFootnoteManifestFlowFact] {
        &self.flows
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnoteLayoutFacts {
    profile_sha256: [u8; 32],
    registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    body_layout_sha256: [u8; 32],
    paint_sha256: [u8; 32],
    pages: Vec<StagingFootnoteManifestPageFact>,
    canonical_jcs: String,
}

impl StagingFootnoteLayoutFacts {
    pub fn from_selected(
        selected: &ValidatedFootnoteSelectedLayout,
        display: &FootnoteDisplayClosureReceipt,
    ) -> Result<Self, StagingFootnoteManifestError> {
        if display.profile_sha256() != selected.profile_fingerprint().bytes()
            || display.registry_sha256() != selected.registry_fingerprint().bytes()
            || display.selected_layout_sha256() != selected.fingerprint().bytes()
            || display.body_layout_sha256() != selected.body_layout_fingerprint().bytes()
            || display.pages().len() != selected.pages().len()
        {
            return Err(StagingFootnoteManifestError::SelectedStateMismatch);
        }
        let mut pages = Vec::new();
        pages
            .try_reserve_exact(selected.pages().len())
            .map_err(|_| StagingFootnoteManifestError::AllocationFailure)?;
        for (selected_page, painted_page) in selected.pages().iter().zip(display.pages()) {
            if selected_page.page_index() != painted_page.page_index()
                || selected_page.body_continuation().next_flow_position()
                    != painted_page.body_continuation_position()
                || selected_page.body_continuation().is_terminal()
                    != painted_page.body_continuation_terminal()
                || selected_page.body_fingerprint() != painted_page.body_fingerprint()
                || selected_page.evaluation_count() != painted_page.evaluation_count()
                || selected_page.reservation() != painted_page.reservation()
                || selected_page.flows().len() != painted_page.flows().len()
            {
                return Err(StagingFootnoteManifestError::PaintStateMismatch);
            }
            let selected_ids: Vec<_> = selected_page
                .ordered_footnotes()
                .iter()
                .map(|assignment| assignment.footnote_id().clone())
                .collect();
            if selected_ids != painted_page.ordered_footnote_ids() {
                return Err(StagingFootnoteManifestError::PaintStateMismatch);
            }
            let mut flows = Vec::new();
            flows
                .try_reserve_exact(selected_page.flows().len())
                .map_err(|_| StagingFootnoteManifestError::AllocationFailure)?;
            for (selected_flow, painted_flow) in
                selected_page.flows().iter().zip(painted_page.flows())
            {
                let assignment = selected_flow.assignment();
                if assignment.footnote_id() != painted_flow.footnote_id()
                    || assignment.assignment_ordinal() != painted_flow.assignment_ordinal()
                    || assignment.flow_id() != painted_flow.flow_id()
                    || selected_flow.before_cursor().next_fragment_ordinal()
                        != painted_flow.before_fragment()
                    || selected_flow.after_cursor().next_fragment_ordinal()
                        != painted_flow.after_fragment()
                    || selected_flow.incoming_source_page() != painted_flow.incoming_source_page()
                    || selected_flow.carries_out() != painted_flow.carries_out()
                {
                    return Err(StagingFootnoteManifestError::PaintStateMismatch);
                }
                flows.push(StagingFootnoteManifestFlowFact {
                    footnote_id: assignment.footnote_id().clone(),
                    assignment_ordinal: assignment.assignment_ordinal(),
                    flow_id: assignment.flow_id().get(),
                    before_fragment: selected_flow.before_cursor().next_fragment_ordinal(),
                    after_fragment: selected_flow.after_cursor().next_fragment_ordinal(),
                    incoming_source_page: selected_flow.incoming_source_page(),
                    carries_out: selected_flow.carries_out(),
                });
            }
            pages.push(StagingFootnoteManifestPageFact {
                page_index: selected_page.page_index(),
                body_continuation_position: selected_page.body_continuation().next_flow_position(),
                body_continuation_terminal: selected_page.body_continuation().is_terminal(),
                body_fingerprint: selected_page.body_fingerprint().bytes(),
                evaluation_count: selected_page.evaluation_count(),
                reservation: selected_page.reservation().get().raw(),
                ordered_footnote_ids: selected_ids,
                flows,
            });
        }
        let mut facts = Self {
            profile_sha256: display.profile_sha256(),
            registry_sha256: display.registry_sha256(),
            selected_layout_sha256: display.selected_layout_sha256(),
            body_layout_sha256: display.body_layout_sha256(),
            paint_sha256: display.fingerprint(),
            pages,
            canonical_jcs: String::new(),
        };
        facts.canonical_jcs = encode_staging_footnote_layout_facts(&facts);
        Ok(facts)
    }

    pub const fn profile_sha256(&self) -> [u8; 32] {
        self.profile_sha256
    }
    pub const fn registry_sha256(&self) -> [u8; 32] {
        self.registry_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub const fn body_layout_sha256(&self) -> [u8; 32] {
        self.body_layout_sha256
    }
    pub const fn paint_sha256(&self) -> [u8; 32] {
        self.paint_sha256
    }
    pub fn pages(&self) -> &[StagingFootnoteManifestPageFact] {
        &self.pages
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn encode_staging_footnote_layout_facts(value: &StagingFootnoteLayoutFacts) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_FOOTNOTE_MANIFEST_ALGORITHM);
    output.push_str(",\"body_layout_sha256\":");
    push_manifest_hex(&mut output, value.body_layout_sha256);
    output.push_str(",\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"body_continuation_position\":");
        output.push_str(&page.body_continuation_position.to_string());
        output.push_str(",\"body_continuation_terminal\":");
        output.push_str(if page.body_continuation_terminal {
            "true"
        } else {
            "false"
        });
        output.push_str(",\"body_fingerprint\":");
        push_manifest_hex(&mut output, page.body_fingerprint);
        output.push_str(",\"evaluation_count\":");
        output.push_str(&page.evaluation_count.to_string());
        output.push_str(",\"flows\":[");
        for (flow_index, flow) in page.flows.iter().enumerate() {
            if flow_index > 0 {
                output.push(',');
            }
            output.push_str("{\"after_fragment\":");
            output.push_str(&flow.after_fragment.to_string());
            output.push_str(",\"assignment_ordinal\":");
            output.push_str(&flow.assignment_ordinal.to_string());
            output.push_str(",\"before_fragment\":");
            output.push_str(&flow.before_fragment.to_string());
            output.push_str(",\"carries_out\":");
            output.push_str(if flow.carries_out { "true" } else { "false" });
            output.push_str(",\"flow_id\":");
            output.push_str(&flow.flow_id.to_string());
            output.push_str(",\"footnote_id\":");
            push_jcs_string(&mut output, flow.footnote_id.as_str());
            output.push_str(",\"incoming_source_page\":");
            match flow.incoming_source_page {
                Some(value) => output.push_str(&value.to_string()),
                None => output.push_str("null"),
            }
            output.push('}');
        }
        output.push_str("],\"ordered_footnote_ids\":[");
        for (footnote_index, footnote_id) in page.ordered_footnote_ids.iter().enumerate() {
            if footnote_index > 0 {
                output.push(',');
            }
            push_jcs_string(&mut output, footnote_id.as_str());
        }
        output.push_str("],\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"reservation\":");
        output.push_str(&page.reservation.to_string());
        output.push('}');
    }
    output.push_str("],\"paint_sha256\":");
    push_manifest_hex(&mut output, value.paint_sha256);
    output.push_str(",\"profile_sha256\":");
    push_manifest_hex(&mut output, value.profile_sha256);
    output.push_str(",\"registry_sha256\":");
    push_manifest_hex(&mut output, value.registry_sha256);
    output.push_str(",\"selected_layout_sha256\":");
    push_manifest_hex(&mut output, value.selected_layout_sha256);
    output.push('}');
    output
}

fn push_manifest_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

/// Closed set of selected-layout facts consumed by machine manifest
/// publication. Grouping these phase-owned values keeps the publication API
/// explicit without allowing callers to insert manifest records directly.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingMachineLayoutFacts {
    flow_registry_sha256: Option<[u8; 32]>,
    table_layouts: Vec<StagingTableLayoutFacts>,
    footnote_layout: Option<StagingFootnoteLayoutFacts>,
}

impl StagingMachineLayoutFacts {
    pub fn new(
        flow_registry_sha256: Option<[u8; 32]>,
        table_layouts: Vec<StagingTableLayoutFacts>,
    ) -> Self {
        Self {
            flow_registry_sha256,
            table_layouts,
            footnote_layout: None,
        }
    }

    pub fn with_footnote(mut self, footnote: StagingFootnoteLayoutFacts) -> Self {
        self.footnote_layout = Some(footnote);
        self
    }

    pub const fn flow_registry_sha256(&self) -> Option<[u8; 32]> {
        self.flow_registry_sha256
    }

    pub fn table_layouts(&self) -> &[StagingTableLayoutFacts] {
        &self.table_layouts
    }

    pub const fn footnote_layout(&self) -> Option<&StagingFootnoteLayoutFacts> {
        self.footnote_layout.as_ref()
    }

    fn into_parts(
        self,
    ) -> (
        Option<[u8; 32]>,
        Vec<StagingTableLayoutFacts>,
        Option<StagingFootnoteLayoutFacts>,
    ) {
        (
            self.flow_registry_sha256,
            self.table_layouts,
            self.footnote_layout,
        )
    }
}

pub const STAGING_MACHINE_BLOCK_STYLE_MANIFEST_ALGORITHM: &str =
    "typaxis.machine-block-style-manifest/1";

/// Selected-state manifest projection for one MI2-03 typed block. Every fact
/// is copied from the PDF-stage receipt, so the manifest cannot reinterpret a
/// declaration or recompute geometry.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingMachineBlockStyleManifestFact {
    pdf_observation_sha256: [u8; 32],
    package_sha256: [u8; 32],
    registry_version: &'static str,
    owner_node_id: u32,
    block_kind: &'static str,
    frame_inline_size: i64,
    available_inline_size: i64,
    paint_inline_size: i64,
    start_indent: i64,
    end_indent: i64,
    logical_start_alignment_space: i64,
    logical_end_alignment_space: i64,
    paint_left_inset: i64,
    effective_space_before: i64,
    effective_space_after: i64,
    page_break_before: bool,
    keep_with_next: bool,
    keep_caption: bool,
    canonical_jcs: String,
}

impl StagingMachineBlockStyleManifestFact {
    pub fn from_pdf(pdf: &StagingMachineBlockStylePdf) -> Self {
        let mut value = Self {
            pdf_observation_sha256: typaxis_core::sha256(pdf.canonical_jcs().as_bytes()),
            package_sha256: pdf.package_sha256(),
            registry_version: pdf.registry_version(),
            owner_node_id: pdf.owner_node_id(),
            block_kind: pdf.block_kind(),
            frame_inline_size: pdf.frame_inline_size(),
            available_inline_size: pdf.available_inline_size(),
            paint_inline_size: pdf.paint_inline_size(),
            start_indent: pdf.start_indent(),
            end_indent: pdf.end_indent(),
            logical_start_alignment_space: pdf.logical_start_alignment_space(),
            logical_end_alignment_space: pdf.logical_end_alignment_space(),
            paint_left_inset: pdf.paint_left_inset(),
            effective_space_before: pdf.effective_space_before(),
            effective_space_after: pdf.effective_space_after(),
            page_break_before: pdf.page_break_before(),
            keep_with_next: pdf.keep_with_next(),
            keep_caption: pdf.keep_caption(),
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_block_style_manifest(&value);
        value
    }

    pub const fn pdf_observation_sha256(&self) -> [u8; 32] {
        self.pdf_observation_sha256
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn registry_version(&self) -> &'static str {
        self.registry_version
    }
    pub const fn owner_node_id(&self) -> u32 {
        self.owner_node_id
    }
    pub const fn block_kind(&self) -> &'static str {
        self.block_kind
    }
    pub const fn frame_inline_size(&self) -> i64 {
        self.frame_inline_size
    }
    pub const fn available_inline_size(&self) -> i64 {
        self.available_inline_size
    }
    pub const fn paint_inline_size(&self) -> i64 {
        self.paint_inline_size
    }
    pub const fn start_indent(&self) -> i64 {
        self.start_indent
    }
    pub const fn end_indent(&self) -> i64 {
        self.end_indent
    }
    pub const fn logical_start_alignment_space(&self) -> i64 {
        self.logical_start_alignment_space
    }
    pub const fn logical_end_alignment_space(&self) -> i64 {
        self.logical_end_alignment_space
    }
    pub const fn paint_left_inset(&self) -> i64 {
        self.paint_left_inset
    }
    pub const fn effective_space_before(&self) -> i64 {
        self.effective_space_before
    }
    pub const fn effective_space_after(&self) -> i64 {
        self.effective_space_after
    }
    pub const fn page_break_before(&self) -> bool {
        self.page_break_before
    }
    pub const fn keep_with_next(&self) -> bool {
        self.keep_with_next
    }
    pub const fn keep_caption(&self) -> bool {
        self.keep_caption
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn encode_staging_machine_block_style_manifest(
    value: &StagingMachineBlockStyleManifestFact,
) -> String {
    let mut output = String::from("{\"algorithm\":\"");
    output.push_str(STAGING_MACHINE_BLOCK_STYLE_MANIFEST_ALGORITHM);
    output.push_str("\",\"available_inline_size\":");
    output.push_str(&value.available_inline_size.to_string());
    output.push_str(",\"block_kind\":\"");
    output.push_str(value.block_kind);
    output.push_str("\",\"contract\":\"typaxis.contract/1.2\",\"effective_space_after\":");
    output.push_str(&value.effective_space_after.to_string());
    output.push_str(",\"effective_space_before\":");
    output.push_str(&value.effective_space_before.to_string());
    output.push_str(",\"end_indent\":");
    output.push_str(&value.end_indent.to_string());
    output.push_str(",\"frame_inline_size\":");
    output.push_str(&value.frame_inline_size.to_string());
    output.push_str(",\"keep_caption\":");
    output.push_str(if value.keep_caption { "true" } else { "false" });
    output.push_str(",\"keep_with_next\":");
    output.push_str(if value.keep_with_next {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"logical_end_alignment_space\":");
    output.push_str(&value.logical_end_alignment_space.to_string());
    output.push_str(",\"logical_start_alignment_space\":");
    output.push_str(&value.logical_start_alignment_space.to_string());
    output.push_str(",\"owner_node_id\":");
    output.push_str(&value.owner_node_id.to_string());
    output.push_str(",\"package_sha256\":");
    push_json_hex(&mut output, &value.package_sha256);
    output.push_str(",\"page_break_before\":");
    output.push_str(if value.page_break_before {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"paint_inline_size\":");
    output.push_str(&value.paint_inline_size.to_string());
    output.push_str(",\"paint_left_inset\":");
    output.push_str(&value.paint_left_inset.to_string());
    output.push_str(",\"pdf_observation_sha256\":");
    push_json_hex(&mut output, &value.pdf_observation_sha256);
    output.push_str(",\"registry_version\":\"");
    output.push_str(value.registry_version);
    output.push_str("\",\"start_indent\":");
    output.push_str(&value.start_indent.to_string());
    output.push('}');
    output
}

pub const STAGING_MACHINE_FIGURE_MANIFEST_ALGORITHM: &str = "typaxis.machine-figure-manifest/1";

/// Non-current MI2-06 manifest fact derived exclusively from the closed PDF
/// observation. It does not add a caller-declared media field to the public
/// build manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigureManifestFact {
    pdf: StagingMachineFigurePdf,
    canonical_jcs: String,
}

impl StagingMachineFigureManifestFact {
    pub fn from_pdf(pdf: &StagingMachineFigurePdf) -> Self {
        let mut value = Self {
            pdf: pdf.clone(),
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_figure_manifest(&value.pdf);
        value
    }

    pub const fn pdf(&self) -> &StagingMachineFigurePdf {
        &self.pdf
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn encode_staging_machine_figure_manifest(pdf: &StagingMachineFigurePdf) -> String {
    let display = pdf.display();
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_FIGURE_MANIFEST_ALGORITHM);
    output.push_str(",\"body\":");
    encode_staging_machine_figure_manifest_rect(&mut output, display.body());
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"display_sha256\":");
    push_json_hex(&mut output, &pdf.display_sha256());
    output.push_str(",\"figure_usage_sha256\":");
    push_json_hex(&mut output, &display.figure_usage_sha256());
    output.push_str(",\"figures\":[");
    for (index, figure) in display.figures().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"admitted_byte_length\":");
        output.push_str(&figure.admitted_byte_length().to_string());
        output.push_str(",\"admitted_sha256\":");
        push_json_hex(&mut output, &figure.admitted_sha256());
        output.push_str(",\"alt\":");
        push_jcs_string(&mut output, figure.alt());
        output.push_str(",\"attested_media_kind\":");
        push_jcs_string(&mut output, figure.attested_media_kind());
        output.push_str(",\"caption_flow_id\":");
        output.push_str(&figure.caption_flow_id().to_string());
        output.push_str(",\"caption_fragments\":[");
        for (caption_index, caption) in figure.caption_fragments().iter().enumerate() {
            if caption_index > 0 {
                output.push(',');
            }
            output.push_str("{\"caption_flow_id\":");
            output.push_str(&caption.caption_flow_id().to_string());
            output.push_str(",\"caption_node_id\":");
            output.push_str(&caption.caption_node_id().to_string());
            output.push_str(",\"page_index\":");
            output.push_str(&caption.page_index().to_string());
            output.push_str(",\"rect\":");
            encode_staging_machine_figure_manifest_rect(&mut output, caption.rect());
            output.push('}');
        }
        output.push_str("],\"decoded_bytes\":");
        output.push_str(&figure.decoded_bytes().to_string());
        output.push_str(",\"document_ordinal\":");
        output.push_str(&figure.document_ordinal().to_string());
        output.push_str(",\"draw_image_count\":1,\"effective_space_before\":");
        output.push_str(&figure.effective_space_before().to_string());
        output.push_str(",\"figure_flow_id\":");
        output.push_str(&figure.figure_flow_id().to_string());
        output.push_str(",\"figure_node_id\":");
        output.push_str(&figure.figure_node_id().to_string());
        output.push_str(",\"image_id\":");
        output.push_str(&figure.image_id().get().to_string());
        output.push_str(",\"keep_policy\":");
        push_jcs_string(&mut output, figure.keep_policy());
        output.push_str(",\"moved_to_fresh_page\":");
        output.push_str(if figure.moved_to_fresh_page() {
            "true"
        } else {
            "false"
        });
        output.push_str(",\"oversize_policy\":");
        push_jcs_string(&mut output, figure.oversize_policy());
        output.push_str(",\"page_index\":");
        output.push_str(&figure.page_index().to_string());
        output.push_str(",\"pixel_height\":");
        output.push_str(&figure.pixel_height().to_string());
        output.push_str(",\"pixel_width\":");
        output.push_str(&figure.pixel_width().to_string());
        output.push_str(",\"rect\":");
        encode_staging_machine_figure_manifest_rect(&mut output, figure.rect());
        output.push('}');
    }
    output.push_str("],\"flow_registry_sha256\":");
    push_json_hex(&mut output, &display.flow_registry_sha256());
    output.push_str(",\"image_xobject_count\":");
    output.push_str(&pdf.image_xobject_count().to_string());
    output.push_str(",\"image_xobjects\":[");
    for (index, binding) in pdf.image_xobjects().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"image_id\":");
        output.push_str(&binding.image_id().get().to_string());
        output.push_str(",\"resource_name\":");
        push_jcs_string(&mut output, binding.resource_name());
        output.push('}');
    }
    output.push_str("],\"layout_epoch\":");
    let epoch = display.epoch();
    output.push_str("{\"admitted_resources_sha256\":");
    push_json_hex(&mut output, &epoch.admitted_resources_sha256());
    output.push_str(",\"document_sha256\":");
    push_json_hex(&mut output, &epoch.document_sha256());
    output.push_str(",\"resolved_input_sha256\":");
    push_json_hex(&mut output, &epoch.resolved_input_sha256());
    output.push_str(",\"style_page_master_sha256\":");
    push_json_hex(&mut output, &epoch.style_page_master_sha256());
    output.push('}');
    output.push_str(",\"layout_state_sha256\":");
    push_json_hex(&mut output, &display.layout_state_sha256());
    output.push_str(",\"master_id\":");
    push_jcs_string(&mut output, display.master_id().as_str());
    output.push_str(",\"object_count\":");
    output.push_str(&pdf.object_count().to_string());
    output.push_str(",\"package_sha256\":");
    push_json_hex(&mut output, &display.package_sha256());
    output.push_str(",\"page_count\":");
    output.push_str(&pdf.page_count().to_string());
    output.push_str(",\"page_height\":");
    output.push_str(&display.page_height().get().raw().to_string());
    output.push_str(",\"page_width\":");
    output.push_str(&display.page_width().get().raw().to_string());
    output.push_str(",\"pages\":[");
    for (index, page) in display.pages().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"caption_block_count\":");
        output.push_str(&page.caption_block_count().to_string());
        output.push_str(",\"figure_count\":");
        output.push_str(&page.figure_count().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index().to_string());
        output.push('}');
    }
    output.push_str("],\"pdf_byte_length\":");
    output.push_str(&pdf.pdf_byte_length().to_string());
    output.push_str(",\"pdf_sha256\":");
    push_json_hex(&mut output, &pdf.pdf_sha256());
    output.push_str(",\"policy_version\":");
    push_jcs_string(&mut output, display.policy_version());
    output.push_str(
        ",\"profile\":\"typaxis.machine-pdf/basic-document-1\",\"selected_state_sha256\":",
    );
    push_json_hex(&mut output, &display.selected_state_sha256());
    output.push('}');
    output
}

fn encode_staging_machine_figure_manifest_rect(output: &mut String, rect: Rect) {
    output.push_str("{\"height\":");
    output.push_str(&rect.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&rect.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&rect.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&rect.y().raw().to_string());
    output.push('}');
}

pub const STAGING_MACHINE_LINK_MANIFEST_ALGORITHM: &str = "typaxis.machine-link-manifest/1";

/// Focused MI2-07 selected-state manifest. Every fact is projected from the
/// PDF-stage annotation/destination closure; the public 1.2 build manifest
/// binds the complete pipeline receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineLinkManifestFact {
    pdf: StagingMachineLinkPdf,
    canonical_jcs: String,
}

impl StagingMachineLinkManifestFact {
    pub fn from_pdf(pdf: &StagingMachineLinkPdf) -> Self {
        let mut value = Self {
            pdf: pdf.clone(),
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_link_manifest(&value.pdf);
        value
    }

    pub const fn pdf(&self) -> &StagingMachineLinkPdf {
        &self.pdf
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn encode_staging_machine_link_manifest(pdf: &StagingMachineLinkPdf) -> String {
    let display = pdf.display();
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_LINK_MANIFEST_ALGORITHM);
    output.push_str(",\"annotation_count\":");
    output.push_str(&pdf.annotation_count().to_string());
    output.push_str(",\"cluster_receipt_sha256\":");
    push_json_hex(&mut output, &display.cluster_receipt_sha256());
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"destination_count\":");
    output.push_str(&pdf.destination_count().to_string());
    output.push_str(",\"destinations\":[");
    for (index, destination) in display.destinations().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"anchor_id\":");
        push_jcs_string(&mut output, destination.anchor_id().as_str());
        output.push_str(",\"owner_node_id\":");
        output.push_str(&destination.owner_node_id().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&destination.page_index().to_string());
        output.push_str(",\"point\":{");
        output.push_str("\"x\":");
        output.push_str(&destination.point().x.raw().to_string());
        output.push_str(",\"y\":");
        output.push_str(&destination.point().y.raw().to_string());
        output.push_str("}}");
    }
    output.push_str("],\"display_sha256\":");
    push_json_hex(&mut output, &pdf.display_sha256());
    output.push_str(",\"layout_epoch\":{");
    let epoch = display.epoch();
    output.push_str("\"admitted_resources_sha256\":");
    push_json_hex(&mut output, &epoch.admitted_resources().bytes());
    output.push_str(",\"document_sha256\":");
    push_json_hex(&mut output, &epoch.document().bytes());
    output.push_str(",\"resolved_input_sha256\":");
    push_json_hex(&mut output, &epoch.references().bytes());
    output.push_str(",\"style_page_master_sha256\":");
    push_json_hex(&mut output, &epoch.style().bytes());
    output.push_str("},\"layout_state_sha256\":");
    push_json_hex(&mut output, &display.layout_state_sha256());
    output.push_str(",\"links\":[");
    for (index, link) in display.links().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"link_node_id\":");
        output.push_str(&link.link_node_id().to_string());
        output.push_str(",\"logical_cluster_count\":");
        output.push_str(&link.logical_cluster_count().to_string());
        output.push_str(",\"logical_cluster_end\":");
        output.push_str(&link.logical_cluster_end().to_string());
        output.push_str(",\"logical_cluster_start\":");
        output.push_str(&link.logical_cluster_start().to_string());
        output.push_str(",\"paragraph_node_id\":");
        output.push_str(&link.paragraph_node_id().to_string());
        output.push_str(",\"rectangles\":[");
        for (rectangle_index, rectangle) in link.rectangles().iter().enumerate() {
            if rectangle_index > 0 {
                output.push(',');
            }
            let annotation = pdf
                .annotations()
                .iter()
                .find(|annotation| {
                    annotation.link_node_id() == rectangle.link_node_id()
                        && annotation.page_index() == rectangle.page_index()
                        && annotation.line_ordinal() == rectangle.line_ordinal()
                })
                .expect("PDF link closure covers every Display rectangle");
            output.push_str("{\"annotation_object_id\":");
            output.push_str(&annotation.object_id().to_string());
            output.push_str(",\"line_ordinal\":");
            output.push_str(&rectangle.line_ordinal().to_string());
            output.push_str(",\"page_index\":");
            output.push_str(&rectangle.page_index().to_string());
            output.push_str(",\"rect\":");
            encode_staging_machine_figure_manifest_rect(&mut output, rectangle.rect());
            output.push('}');
        }
        output.push_str("],\"target\":");
        let target = link.target();
        if let Some(anchor_id) = target.anchor_id() {
            output.push_str("{\"anchor_id\":");
            push_jcs_string(&mut output, anchor_id.as_str());
            output.push_str(",\"anchor_owner_node_id\":");
            output.push_str(
                &target
                    .anchor_owner_node_id()
                    .expect("an internal target has an anchor owner")
                    .to_string(),
            );
            output.push_str(",\"kind\":\"internal\"}");
        } else {
            output.push_str("{\"kind\":\"external\",\"uri\":");
            push_jcs_string(
                &mut output,
                target
                    .uri()
                    .expect("an external target has a SafeUri")
                    .as_str(),
            );
            output.push('}');
        }
        output.push('}');
    }
    output.push_str("],\"object_count\":");
    output.push_str(&pdf.object_count().to_string());
    output.push_str(",\"package_sha256\":");
    push_json_hex(&mut output, &display.package_sha256());
    output.push_str(",\"page_count\":");
    output.push_str(&pdf.page_count().to_string());
    output.push_str(",\"pages\":[");
    for (index, page) in display.pages().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"annotation_count\":");
        output.push_str(&page.annotation_count().to_string());
        output.push_str(",\"height\":");
        output.push_str(&page.height().get().raw().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index().to_string());
        output.push_str(",\"width\":");
        output.push_str(&page.width().get().raw().to_string());
        output.push('}');
    }
    output.push_str("],\"pdf_byte_length\":");
    output.push_str(&pdf.pdf_byte_length().to_string());
    output.push_str(",\"pdf_sha256\":");
    push_json_hex(&mut output, &pdf.pdf_sha256());
    output.push_str(",\"policy_version\":");
    push_jcs_string(&mut output, display.policy_version());
    output.push_str(
        ",\"profile\":\"typaxis.machine-pdf/basic-document-1\",\"selected_state_sha256\":",
    );
    push_json_hex(&mut output, &display.selected_state_sha256());
    output.push_str(",\"usage_sha256\":");
    push_json_hex(&mut output, &display.usage_sha256());
    output.push('}');
    output
}

pub const STAGING_FORCED_PAGE_BREAK_MANIFEST_ALGORITHM: &str =
    "typaxis.forced-page-break-manifest/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakManifestPage {
    page_index: u32,
    painted_content_count: u32,
}

impl StagingForcedPageBreakManifestPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn painted_content_count(&self) -> u32 {
        self.painted_content_count
    }

    pub const fn is_blank(&self) -> bool {
        self.painted_content_count == 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakManifestBoundary {
    break_node_id: u32,
    document_ordinal: u32,
    flow_id: u32,
    before_flow_local_ordinal: u32,
    after_flow_local_ordinal: u32,
    produced_page_index: u32,
}

impl StagingForcedPageBreakManifestBoundary {
    pub const fn break_node_id(&self) -> u32 {
        self.break_node_id
    }

    pub const fn document_ordinal(&self) -> u32 {
        self.document_ordinal
    }

    pub const fn flow_id(&self) -> u32 {
        self.flow_id
    }

    pub const fn before_flow_local_ordinal(&self) -> u32 {
        self.before_flow_local_ordinal
    }

    pub const fn after_flow_local_ordinal(&self) -> u32 {
        self.after_flow_local_ordinal
    }

    pub const fn produced_page_index(&self) -> u32 {
        self.produced_page_index
    }
}

/// Versioned MI2-05 manifest fact derived only from the PDF page-tree
/// observation and exact forced-boundary receipt chain.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakManifestFact {
    pdf_observation_sha256: [u8; 32],
    pdf_page_tree_sha256: [u8; 32],
    package_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    usage_sha256: [u8; 32],
    policy_version: &'static str,
    page_count: u32,
    pages: Vec<StagingForcedPageBreakManifestPage>,
    breaks: Vec<StagingForcedPageBreakManifestBoundary>,
    canonical_jcs: String,
}

impl StagingForcedPageBreakManifestFact {
    pub fn from_pdf(pdf: &StagingForcedPageBreakPdf) -> Self {
        let pages = pdf
            .pages()
            .iter()
            .map(|page| StagingForcedPageBreakManifestPage {
                page_index: page.page_index(),
                painted_content_count: page.painted_content_count(),
            })
            .collect();
        let breaks = pdf
            .breaks()
            .iter()
            .map(|boundary| StagingForcedPageBreakManifestBoundary {
                break_node_id: boundary.break_node_id(),
                document_ordinal: boundary.document_ordinal(),
                flow_id: boundary.flow_id(),
                before_flow_local_ordinal: boundary.before_flow_local_ordinal(),
                after_flow_local_ordinal: boundary.after_flow_local_ordinal(),
                produced_page_index: boundary.produced_page_index(),
            })
            .collect();
        let mut value = Self {
            pdf_observation_sha256: typaxis_core::sha256(pdf.canonical_jcs().as_bytes()),
            pdf_page_tree_sha256: typaxis_core::sha256(pdf.page_tree_observation()),
            package_sha256: pdf.package_sha256(),
            flow_registry_sha256: pdf.flow_registry_sha256(),
            usage_sha256: pdf.usage_sha256(),
            policy_version: pdf.policy_version(),
            page_count: pdf.page_count(),
            pages,
            breaks,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_forced_page_break_manifest(&value);
        value
    }

    pub const fn pdf_observation_sha256(&self) -> [u8; 32] {
        self.pdf_observation_sha256
    }

    pub const fn pdf_page_tree_sha256(&self) -> [u8; 32] {
        self.pdf_page_tree_sha256
    }

    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }

    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }

    pub const fn usage_sha256(&self) -> [u8; 32] {
        self.usage_sha256
    }

    pub const fn page_count(&self) -> u32 {
        self.page_count
    }

    pub fn pages(&self) -> &[StagingForcedPageBreakManifestPage] {
        &self.pages
    }

    pub fn breaks(&self) -> &[StagingForcedPageBreakManifestBoundary] {
        &self.breaks
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn encode_staging_forced_page_break_manifest(value: &StagingForcedPageBreakManifestFact) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_FORCED_PAGE_BREAK_MANIFEST_ALGORITHM);
    output.push_str(",\"break_usage_sha256\":");
    push_json_hex(&mut output, &value.usage_sha256);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"flow_registry_sha256\":");
    push_json_hex(&mut output, &value.flow_registry_sha256);
    output.push_str(",\"forced_page_breaks\":[");
    for (index, boundary) in value.breaks.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_staging_forced_page_break_manifest_boundary(&mut output, boundary);
    }
    output.push_str("],\"package_sha256\":");
    push_json_hex(&mut output, &value.package_sha256);
    output.push_str(",\"page_count\":");
    output.push_str(&value.page_count.to_string());
    output.push_str(",\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"is_blank\":");
        output.push_str(if page.is_blank() { "true" } else { "false" });
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"painted_content_count\":");
        output.push_str(&page.painted_content_count.to_string());
        output.push('}');
    }
    output.push_str("],\"pdf_observation_sha256\":");
    push_json_hex(&mut output, &value.pdf_observation_sha256);
    output.push_str(",\"pdf_page_tree_sha256\":");
    push_json_hex(&mut output, &value.pdf_page_tree_sha256);
    output.push_str(",\"policy_version\":");
    push_jcs_string(&mut output, value.policy_version);
    output.push('}');
    output
}

fn encode_staging_forced_page_break_manifest_boundary(
    output: &mut String,
    boundary: &StagingForcedPageBreakManifestBoundary,
) {
    output.push_str("{\"after_cursor\":{\"flow_id\":");
    output.push_str(&boundary.flow_id.to_string());
    output.push_str(",\"flow_local_ordinal\":");
    output.push_str(&boundary.after_flow_local_ordinal.to_string());
    output.push_str("},\"before_cursor\":{\"flow_id\":");
    output.push_str(&boundary.flow_id.to_string());
    output.push_str(",\"flow_local_ordinal\":");
    output.push_str(&boundary.before_flow_local_ordinal.to_string());
    output.push_str("},\"break_node_id\":");
    output.push_str(&boundary.break_node_id.to_string());
    output.push_str(",\"document_ordinal\":");
    output.push_str(&boundary.document_ordinal.to_string());
    output.push_str(",\"produced_page_index\":");
    output.push_str(&boundary.produced_page_index.to_string());
    output.push('}');
}

pub const STAGING_MACHINE_LIST_MANIFEST_ALGORITHM: &str = "typaxis.machine-list-manifest/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListManifestList {
    list_node_id: u32,
    list_flow_id: u32,
    marker_column_width: i64,
    marker_gap: i64,
    start_indent: i64,
    end_indent: i64,
    item_frame_inline_size: i64,
}

impl StagingMachineListManifestList {
    pub const fn list_node_id(&self) -> u32 {
        self.list_node_id
    }
    pub const fn list_flow_id(&self) -> u32 {
        self.list_flow_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListManifestItem {
    list_node_id: u32,
    item_node_id: u32,
    item_index: u32,
    list_flow_id: u32,
    item_flow_id: u32,
    marker_key: GeneratedBufferKey,
    marker_utf8: String,
    marker_fragment_id: u64,
    first_line_fragment_id: u64,
    page_index: u32,
    fragment_ids: Vec<u64>,
    marker_inline_size: i64,
    marker_column_width: i64,
    marker_physical_left: i64,
    content_physical_left: i64,
    content_inline_size: i64,
    first_line_inline_size: i64,
    first_line_block_size: i64,
    block_offset: i64,
}

impl StagingMachineListManifestItem {
    pub const fn item_node_id(&self) -> u32 {
        self.item_node_id
    }
    pub const fn list_flow_id(&self) -> u32 {
        self.list_flow_id
    }
    pub const fn item_flow_id(&self) -> u32 {
        self.item_flow_id
    }
    pub const fn marker_fragment_id(&self) -> u64 {
        self.marker_fragment_id
    }
    pub const fn first_line_fragment_id(&self) -> u64 {
        self.first_line_fragment_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn marker_key(&self) -> GeneratedBufferKey {
        self.marker_key
    }
    pub fn marker_utf8(&self) -> &str {
        &self.marker_utf8
    }
}

/// Versioned MI2-04 manifest fact. It is derived only after the PDF
/// observation has retained every selected marker/item relation.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingMachineListManifestFact {
    pdf_observation_sha256: [u8; 32],
    pdf_content_sha256: [u8; 32],
    package_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    marker_usage_sha256: [u8; 32],
    policy_version: &'static str,
    page_count: u32,
    lists: Vec<StagingMachineListManifestList>,
    items: Vec<StagingMachineListManifestItem>,
    canonical_jcs: String,
}

impl StagingMachineListManifestFact {
    pub fn from_pdf(pdf: &StagingMachineListPdf) -> Self {
        let lists = pdf
            .lists()
            .iter()
            .map(|list| StagingMachineListManifestList {
                list_node_id: list.list_node_id(),
                list_flow_id: list.list_flow_id(),
                marker_column_width: list.marker_column_width(),
                marker_gap: list.marker_gap(),
                start_indent: list.start_indent(),
                end_indent: list.end_indent(),
                item_frame_inline_size: list.item_frame_inline_size(),
            })
            .collect();
        let items = pdf
            .items()
            .iter()
            .map(|item| StagingMachineListManifestItem {
                list_node_id: item.list_node_id(),
                item_node_id: item.item_node_id(),
                item_index: item.item_index(),
                list_flow_id: item.list_flow_id(),
                item_flow_id: item.item_flow_id(),
                marker_key: item.marker_key(),
                marker_utf8: item.marker_utf8().to_owned(),
                marker_fragment_id: item.marker_fragment_id(),
                first_line_fragment_id: item.first_line_fragment_id(),
                page_index: item.page_index(),
                fragment_ids: item.fragment_ids().to_vec(),
                marker_inline_size: item.marker_inline_size(),
                marker_column_width: item.marker_column_width(),
                marker_physical_left: item.marker_physical_left(),
                content_physical_left: item.content_physical_left(),
                content_inline_size: item.content_inline_size(),
                first_line_inline_size: item.first_line_inline_size(),
                first_line_block_size: item.first_line_block_size(),
                block_offset: item.block_offset(),
            })
            .collect();
        let mut value = Self {
            pdf_observation_sha256: typaxis_core::sha256(pdf.canonical_jcs().as_bytes()),
            pdf_content_sha256: typaxis_core::sha256(pdf.content_stream_observation()),
            package_sha256: pdf.package_sha256(),
            flow_registry_sha256: pdf.flow_registry_sha256(),
            marker_usage_sha256: pdf.marker_usage_sha256(),
            policy_version: pdf.policy_version(),
            page_count: pdf.page_count(),
            lists,
            items,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_list_manifest(&value);
        value
    }

    pub const fn pdf_observation_sha256(&self) -> [u8; 32] {
        self.pdf_observation_sha256
    }
    pub const fn pdf_content_sha256(&self) -> [u8; 32] {
        self.pdf_content_sha256
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }
    pub const fn marker_usage_sha256(&self) -> [u8; 32] {
        self.marker_usage_sha256
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub fn lists(&self) -> &[StagingMachineListManifestList] {
        &self.lists
    }
    pub fn items(&self) -> &[StagingMachineListManifestItem] {
        &self.items
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn encode_staging_machine_list_manifest(value: &StagingMachineListManifestFact) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_LIST_MANIFEST_ALGORITHM);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"flow_registry_sha256\":");
    push_json_hex(&mut output, &value.flow_registry_sha256);
    output.push_str(",\"items\":[");
    for (index, item) in value.items.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_staging_machine_list_manifest_item(&mut output, item);
    }
    output.push_str("],\"list_flows\":[");
    for (index, list) in value.lists.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"end_indent\":");
        output.push_str(&list.end_indent.to_string());
        output.push_str(",\"item_frame_inline_size\":");
        output.push_str(&list.item_frame_inline_size.to_string());
        output.push_str(",\"list_flow_id\":");
        output.push_str(&list.list_flow_id.to_string());
        output.push_str(",\"list_node_id\":");
        output.push_str(&list.list_node_id.to_string());
        output.push_str(",\"marker_column_width\":");
        output.push_str(&list.marker_column_width.to_string());
        output.push_str(",\"marker_gap\":");
        output.push_str(&list.marker_gap.to_string());
        output.push_str(",\"start_indent\":");
        output.push_str(&list.start_indent.to_string());
        output.push('}');
    }
    output.push_str("],\"marker_usage_sha256\":");
    push_json_hex(&mut output, &value.marker_usage_sha256);
    output.push_str(",\"package_sha256\":");
    push_json_hex(&mut output, &value.package_sha256);
    output.push_str(",\"page_count\":");
    output.push_str(&value.page_count.to_string());
    output.push_str(",\"pdf_content_sha256\":");
    push_json_hex(&mut output, &value.pdf_content_sha256);
    output.push_str(",\"pdf_observation_sha256\":");
    push_json_hex(&mut output, &value.pdf_observation_sha256);
    output.push_str(",\"policy_version\":");
    push_jcs_string(&mut output, value.policy_version);
    output.push('}');
    output
}

fn encode_staging_machine_list_manifest_item(
    output: &mut String,
    item: &StagingMachineListManifestItem,
) {
    output.push_str("{\"block_offset\":");
    output.push_str(&item.block_offset.to_string());
    output.push_str(",\"content_inline_size\":");
    output.push_str(&item.content_inline_size.to_string());
    output.push_str(",\"content_physical_left\":");
    output.push_str(&item.content_physical_left.to_string());
    output.push_str(",\"first_line_block_size\":");
    output.push_str(&item.first_line_block_size.to_string());
    output.push_str(",\"first_line_fragment_id\":");
    output.push_str(&item.first_line_fragment_id.to_string());
    output.push_str(",\"first_line_inline_size\":");
    output.push_str(&item.first_line_inline_size.to_string());
    output.push_str(",\"fragment_ids\":[");
    for (index, fragment) in item.fragment_ids.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&fragment.to_string());
    }
    output.push_str("],\"item_flow_id\":");
    output.push_str(&item.item_flow_id.to_string());
    output.push_str(",\"item_index\":");
    output.push_str(&item.item_index.to_string());
    output.push_str(",\"item_node_id\":");
    output.push_str(&item.item_node_id.to_string());
    output.push_str(",\"list_flow_id\":");
    output.push_str(&item.list_flow_id.to_string());
    output.push_str(",\"list_node_id\":");
    output.push_str(&item.list_node_id.to_string());
    output.push_str(",\"marker_column_width\":");
    output.push_str(&item.marker_column_width.to_string());
    output.push_str(",\"marker_fragment_id\":");
    output.push_str(&item.marker_fragment_id.to_string());
    output.push_str(",\"marker_inline_size\":");
    output.push_str(&item.marker_inline_size.to_string());
    output.push_str(",\"marker_key\":");
    push_generated_buffer_key_jcs(output, item.marker_key);
    output.push_str(",\"marker_physical_left\":");
    output.push_str(&item.marker_physical_left.to_string());
    output.push_str(",\"marker_utf8\":");
    push_jcs_string(output, &item.marker_utf8);
    output.push_str(",\"page_index\":");
    output.push_str(&item.page_index.to_string());
    output.push('}');
}

fn encode_staging_multi_flow_layout_facts(
    flow_registry_sha256: [u8; 32],
    selected_state_sha256: [u8; 32],
    flows: &[StagingManifestFlowFact],
) -> String {
    let mut json = String::from("{\"contract\":\"typaxis.contract/1.2\",\"flow_registry_sha256\":");
    push_json_hex(&mut json, &flow_registry_sha256);
    json.push_str(",\"flows\":[");
    for (index, flow) in flows.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str("{\"flow_id\":");
        json.push_str(&flow.flow_id.to_string());
        json.push_str(",\"owner_node_id\":");
        json.push_str(&flow.owner_node_id.to_string());
        json.push_str(",\"parent_flow_id\":");
        match flow.parent_flow_id {
            Some(parent) => json.push_str(&parent.to_string()),
            None => json.push_str("null"),
        }
        json.push_str(",\"terminal\":");
        json.push_str(&flow.terminal.to_string());
        json.push('}');
    }
    json.push_str("],\"selected_state_sha256\":");
    push_json_hex(&mut json, &selected_state_sha256);
    json.push('}');
    json
}

fn encode_staging_table_layout_facts(value: &StagingTableLayoutFacts) -> String {
    let mut json = String::from("{\"algorithm\":");
    push_jcs_string(&mut json, STAGING_TABLE_MANIFEST_ALGORITHM);
    json.push_str(",\"body_block_size\":");
    json.push_str(&value.body_block_size.to_string());
    json.push_str(",\"columns\":[");
    for (index, column) in value.columns.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push_str("{\"column_ordinal\":");
        json.push_str(&column.column_ordinal.to_string());
        json.push_str(",\"final_width\":");
        json.push_str(&column.final_width.to_string());
        json.push_str(",\"input_kind\":");
        push_jcs_string(&mut json, column.input_kind);
        json.push_str(",\"input_value\":");
        json.push_str(&column.input_value.to_string());
        json.push_str(",\"rounded_fraction_width\":");
        match column.rounded_fraction_width {
            Some(width) => json.push_str(&width.to_string()),
            None => json.push_str("null"),
        }
        json.push('}');
    }
    json.push(']');
    json.push_str(",\"first_page_remaining_block_size\":");
    json.push_str(&value.first_page_remaining_block_size.to_string());
    json.push_str(",\"flow_registry_sha256\":");
    push_json_hex(&mut json, &value.flow_registry_sha256);
    json.push_str(",\"grid_sha256\":");
    push_json_hex(&mut json, &value.grid_sha256);
    json.push_str(",\"header_occurrences\":[");
    for (index, occurrence) in value.header_occurrences.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push_str("{\"fragment_id\":");
        json.push_str(&occurrence.fragment_id.to_string());
        json.push_str(",\"repetition_index\":");
        json.push_str(&occurrence.repetition_index.to_string());
        json.push_str(",\"row_node_id\":");
        json.push_str(&occurrence.row_node_id.to_string());
        json.push_str(",\"source_fragment_id\":");
        json.push_str(&occurrence.source_fragment_id.to_string());
        json.push_str(",\"target_block_offset\":");
        json.push_str(&occurrence.target_block_offset.to_string());
        json.push_str(",\"target_page_index\":");
        json.push_str(&occurrence.target_page_index.to_string());
        json.push('}');
    }
    json.push_str("],\"header_sources\":[");
    for (index, source) in value.header_sources.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push_str("{\"cells\":[");
        encode_staging_table_manifest_cells(&mut json, &source.cells);
        json.push_str("],\"group_block_offset\":");
        json.push_str(&source.group_block_offset.to_string());
        json.push_str(",\"row_node_id\":");
        json.push_str(&source.row_node_id.to_string());
        json.push_str(",\"row_ordinal\":");
        json.push_str(&source.row_ordinal.to_string());
        json.push_str(",\"selected_block_extent\":");
        json.push_str(&source.selected_block_extent.to_string());
        json.push_str(",\"source_fragment_id\":");
        json.push_str(&source.source_fragment_id.to_string());
        json.push('}');
    }
    json.push_str("],\"page_count\":");
    json.push_str(&value.page_count.to_string());
    json.push_str(",\"residual_recipient\":");
    match value.residual_recipient {
        Some(recipient) => json.push_str(&recipient.to_string()),
        None => json.push_str("null"),
    }
    json.push_str(",\"rounding_residual\":");
    json.push_str(&value.rounding_residual.to_string());
    json.push_str(",\"row_band_sha256\":");
    push_json_hex(&mut json, &value.row_band_sha256);
    json.push_str(",\"rows\":[");
    for (index, row) in value.rows.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push_str("{\"cells\":[");
        encode_staging_table_manifest_cells(&mut json, &row.cells);
        json.push_str("],\"continuation_after\":");
        encode_staging_table_manifest_continuation(&mut json, &row.continuation_after);
        json.push_str(",\"continuation_before\":");
        encode_staging_table_manifest_continuation(&mut json, &row.continuation_before);
        json.push_str(",\"fragment_id\":");
        json.push_str(&row.fragment_id.to_string());
        json.push_str(",\"logical_row_ordinal\":");
        json.push_str(&row.logical_row_ordinal.to_string());
        json.push_str(",\"page_block_offset\":");
        json.push_str(&row.page_block_offset.to_string());
        json.push_str(",\"page_index\":");
        json.push_str(&row.page_index.to_string());
        json.push_str(",\"row_fragment_ordinal\":");
        json.push_str(&row.row_fragment_ordinal.to_string());
        json.push_str(",\"row_node_id\":");
        json.push_str(&row.row_node_id.to_string());
        json.push_str(",\"selected_block_extent\":");
        json.push_str(&row.selected_block_extent.to_string());
        json.push('}');
    }
    json.push_str("],\"selected_layout_sha256\":");
    push_json_hex(&mut json, &value.selected_layout_sha256);
    json.push_str(",\"table_node_id\":");
    json.push_str(&value.table_node_id.to_string());
    json.push_str(",\"target_page_start\":");
    json.push_str(&value.target_page_start.to_string());
    json.push('}');
    json
}

fn encode_staging_table_manifest_cells(
    output: &mut String,
    cells: &[StagingTableManifestCellFact],
) {
    for (index, cell) in cells.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"after_fragment_ordinal\":");
        output.push_str(&cell.after_fragment_ordinal.to_string());
        output.push_str(",\"after_terminal\":");
        output.push_str(if cell.after_terminal { "true" } else { "false" });
        output.push_str(",\"before_fragment_ordinal\":");
        output.push_str(&cell.before_fragment_ordinal.to_string());
        output.push_str(",\"before_terminal\":");
        output.push_str(if cell.before_terminal {
            "true"
        } else {
            "false"
        });
        output.push_str(",\"cell_node_id\":");
        output.push_str(&cell.cell_node_id.to_string());
        output.push_str(",\"column_ordinal\":");
        output.push_str(&cell.column_ordinal.to_string());
        output.push_str(",\"colspan\":");
        output.push_str(&cell.colspan.to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&cell.flow_id.to_string());
        output.push_str(",\"rowspan\":");
        output.push_str(&cell.rowspan.to_string());
        output.push_str(",\"selected_block_extent\":");
        output.push_str(&cell.selected_block_extent.to_string());
        output.push_str(",\"vertical_offset_after\":");
        output.push_str(&cell.vertical_offset_after.to_string());
        output.push_str(",\"vertical_offset_before\":");
        output.push_str(&cell.vertical_offset_before.to_string());
        output.push('}');
    }
}

fn encode_staging_table_manifest_continuation(
    output: &mut String,
    continuation: &RowspanContinuationReceipt,
) {
    output.push_str("{\"entries\":[");
    for (index, entry) in continuation.entries().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        let cursor = entry.cell_flow_cursor();
        output.push_str("{\"cell_flow_cursor\":{\"flow_id\":");
        output.push_str(&cursor.flow_id().get().to_string());
        output.push_str(",\"next_fragment_ordinal\":");
        output.push_str(&cursor.next_fragment_ordinal().to_string());
        output.push_str(",\"terminal\":");
        output.push_str(if cursor.is_terminal() {
            "true"
        } else {
            "false"
        });
        output.push_str("},\"cell_node_id\":");
        output.push_str(&entry.cell_owner().get().to_string());
        output.push_str(",\"column_ordinal\":");
        output.push_str(&entry.column_ordinal().to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&entry.flow_id().get().to_string());
        output.push_str(",\"remaining_logical_rows\":");
        output.push_str(&entry.remaining_logical_rows().get().to_string());
        output.push_str(",\"vertical_offset\":");
        output.push_str(&entry.vertical_offset().to_string());
        output.push('}');
    }
    output.push_str("],\"logical_row_ordinal\":");
    output.push_str(&continuation.logical_row_ordinal().to_string());
    output.push('}');
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineRecord {
    name: String,
    version: String,
    rust_version: String,
    git_commit: Option<String>,
}
impl EngineRecord {
    pub fn from_identity(identity: &EngineIdentity) -> Self {
        Self {
            name: identity.name().to_owned(),
            version: identity.version().to_owned(),
            rust_version: identity.rust_version().to_owned(),
            git_commit: identity.git_commit().map(str::to_owned),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn rust_version(&self) -> &str {
        &self.rust_version
    }
    pub fn git_commit(&self) -> Option<&str> {
        self.git_commit.as_deref()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataVersions {
    unicode: String,
    japanese_line_break: String,
    shaper_backend: String,
    shaper_version: String,
}
impl DataVersions {
    fn from_runtime(tables: &ResolvedDataTables, shaper: ShaperIdentity) -> Self {
        let config = tables.versions();
        Self {
            unicode: config.unicode().to_owned(),
            japanese_line_break: config.japanese_line_break().to_owned(),
            shaper_backend: shaper.backend().to_owned(),
            shaper_version: shaper.version().to_owned(),
        }
    }
    pub fn unicode(&self) -> &str {
        &self.unicode
    }
    pub fn japanese_line_break(&self) -> &str {
        &self.japanese_line_break
    }
    pub fn shaper_backend(&self) -> &str {
        &self.shaper_backend
    }
    pub fn shaper_version(&self) -> &str {
        &self.shaper_version
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileRecord {
    uri: PortablePath,
    bytes: u64,
    sha256: [u8; 32],
}

/// Portable PACKAGE identity projected only from machine-input owner receipts.
///
/// ```compile_fail
/// use typaxis_manifest::PackageInputRecord;
/// use typaxis_core::PortablePath;
/// let _forged = PackageInputRecord {
///     uri: PortablePath::new("package.json").unwrap(),
///     bytes: 1,
///     sha256: [0; 32],
///     contract: None,
///     canonical_sha256: None,
///     profile_receipt_sha256: None,
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageInputRecord {
    uri: PortablePath,
    bytes: u64,
    sha256: [u8; 32],
    contract: Option<typaxis_core::DocumentPackageContractId>,
    canonical_sha256: Option<[u8; 32]>,
    profile_receipt_sha256: Option<[u8; 32]>,
}

impl PackageInputRecord {
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }

    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    pub const fn sha256(&self) -> [u8; 32] {
        self.sha256
    }

    pub const fn contract(&self) -> Option<typaxis_core::DocumentPackageContractId> {
        self.contract
    }

    pub const fn canonical_sha256(&self) -> Option<[u8; 32]> {
        self.canonical_sha256
    }

    pub const fn profile_receipt_sha256(&self) -> Option<[u8; 32]> {
        self.profile_receipt_sha256
    }

    fn is_decoded(&self) -> bool {
        self.contract.is_some() && self.canonical_sha256.is_some()
    }
}
impl FileRecord {
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontRecord {
    font_face_id: FontFaceId,
    uri: PortablePath,
    face_index: u32,
    bytes: u64,
    sha256: [u8; 32],
    units_per_em: u16,
    glyph_count: u32,
}
impl FontRecord {
    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn units_per_em(&self) -> u16 {
        self.units_per_em
    }
    pub const fn glyph_count(&self) -> u32 {
        self.glyph_count
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageRecord {
    image_id: ImageResourceId,
    uri: PortablePath,
    bytes: u64,
    sha256: [u8; 32],
    attested_media_kind: &'static str,
    pixel_width: u32,
    pixel_height: u32,
    decoded_bytes: u64,
}
impl ImageRecord {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn attested_media_kind(&self) -> &'static str {
        self.attested_media_kind
    }
    pub const fn pixel_width(&self) -> u32 {
        self.pixel_width
    }
    pub const fn pixel_height(&self) -> u32 {
        self.pixel_height
    }
    pub const fn decoded_bytes(&self) -> u64 {
        self.decoded_bytes
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputRecord {
    sink: OutputSink,
    bytes: u64,
    sha256: [u8; 32],
    page_count: u32,
    pdf_object_count: u32,
}
impl OutputRecord {
    pub const fn sink(&self) -> OutputSink {
        self.sink
    }
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn pdf_object_count(&self) -> u32 {
        self.pdf_object_count
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildManifest {
    advanced_pagination: Option<String>,
    contract: String,
    status: BuildStatus,
    deterministic: bool,
    engine: EngineRecord,
    data_versions: DataVersions,
    config_sha256: [u8; 32],
    input_profile: BuildInputProfile,
    package_input: Option<PackageInputRecord>,
    inputs: Vec<FileRecord>,
    fonts: Vec<FontRecord>,
    images: Vec<ImageRecord>,
    pdf_profile: String,
    stream_compression: PdfStreamCompression,
    layout: Option<LayoutRecord>,
    output: Option<OutputRecord>,
    table_layouts: Vec<StagingTableLayoutFacts>,
    footnote_layout: Option<StagingFootnoteLayoutFacts>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildManifestError {
    WrongContract,
    NonDeterministic,
    WrongEngineName,
    EmptyVersion,
    WrongPdfProfile,
    IntegerNotJsonSafe,
    BuiltRequiresLayoutAndOutput,
    NonBuiltMustNotHaveOutput,
    NonCanonicalInputs,
    NonCanonicalFonts,
    NonCanonicalImages,
    StreamCompressionMismatch,
    DataVersionMismatch,
    ConfigFingerprintMismatch,
    EmptyBuiltOutput,
    EmptyAdmittedResource,
    PackageInputBytesLimit,
    InputSourceLimit,
    InputAggregateLimit,
    FontCountLimit,
    FontBytesLimit,
    ImageCountLimit,
    ImageBytesLimit,
    ResourceAggregateLimit,
    PageLimit,
    OutputBytesLimit,
    PdfObjectLimit,
    MissingEntryInput,
    IncludeFileLimit,
    ImagePixelLimit,
    ImageDecodedBytesLimit,
    InvalidFontMetadata,
    OutputSinkMismatch,
    OutputReceiptBindingMismatch,
    InputProfileMismatch,
    MachineProgressRegression,
    MachineSessionMismatch,
    MachinePackageMismatch,
    MachineCapabilityMismatch,
    ResourceProgressMismatch,
    AdmissionLedgerBindingMismatch,
    NonDenseAdmissionSource,
    DuplicateAdmissionRecord,
    PackageResourceMismatch,
    PackagePaginationMismatch,
    LayoutPassCountMismatch,
    LayoutLimit,
    PaginationReceiptMismatch,
    PdfGraphReceiptMismatch,
    IncompleteLayoutAdmission,
    ReadLedgerAlreadyBound,
    ReadLedgerUnavailable,
    AdvancedPaginationMismatch,
}

/// Per-build owner of the configured PDF sink. This context exists whether
/// or not a build manifest was requested. Its private session identity keeps
/// trusted output capabilities from different build executions separate even
/// when their configuration and host paths happen to be identical.
#[derive(Debug, Eq, PartialEq)]
pub struct BuildOutputCommitContext {
    session: OutputCommitSessionId,
    config_fingerprint: EffectiveConfigFingerprint,
    input_profile: BuildInputProfile,
    stream_compression: PdfStreamCompression,
    limits: ValidatedResourceLimits,
    execution: BuildExecutionContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildOutputCommitContextError {
    Execution(BuildExecutionError),
    SessionIdentityExhausted,
}

impl BuildOutputCommitContext {
    pub fn new(
        config: &EffectiveConfig,
        execution: &BuildExecutionContext,
    ) -> Result<Self, BuildOutputCommitContextError> {
        Self::new_bound(config, execution, BuildInputProfile::ReferenceSource1)
    }

    /// Creates an output session whose machine profile is derived from the
    /// immutable adopted descriptor, not a string or caller-authored record.
    pub fn new_machine(
        config: &EffectiveConfig,
        execution: &BuildExecutionContext,
        descriptor: MachineProfileDescriptor,
    ) -> Result<Self, BuildOutputCommitContextError> {
        Self::new_bound(
            config,
            execution,
            BuildInputProfile::from_descriptor(descriptor),
        )
    }

    fn new_bound(
        config: &EffectiveConfig,
        execution: &BuildExecutionContext,
        input_profile: BuildInputProfile,
    ) -> Result<Self, BuildOutputCommitContextError> {
        execution
            .revalidate_write_targets()
            .map_err(BuildOutputCommitContextError::Execution)?;
        Ok(Self {
            session: OutputCommitSessionId::allocate()?,
            config_fingerprint: config.fingerprint(),
            input_profile,
            stream_compression: config.stream_compression(),
            limits: config.limits().clone(),
            execution: execution.clone(),
        })
    }

    pub const fn config_fingerprint(&self) -> EffectiveConfigFingerprint {
        self.config_fingerprint
    }
    pub const fn input_profile(&self) -> BuildInputProfile {
        self.input_profile
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }
    pub const fn output_sink(&self) -> OutputSink {
        self.execution.output_sink()
    }
    pub const fn manifest_requested(&self) -> bool {
        self.execution.manifest_target().is_some()
    }

    /// Commits serializer-owned PDF bytes when no build manifest was
    /// requested. If a manifest target is configured, callers must instead
    /// obtain a sealed `PreparedBuiltPublication` so a terminal built record
    /// is completely preflighted before the PDF becomes visible.
    ///
    /// Arbitrary bytes and success callbacks cannot enter this API:
    ///
    /// ```compile_fail
    /// # use typaxis_manifest::BuildOutputCommitContext;
    /// # fn forge(output: &BuildOutputCommitContext) {
    /// let _ = output.commit_pdf_without_manifest(b"caller bytes".to_vec());
    /// let _ = output.clone();
    /// # }
    /// ```
    pub fn commit_pdf_without_manifest(
        self,
        pdf: VerifiedPdfBytesReceipt,
    ) -> Result<PdfSinkCommitReceipt, PdfSinkCommitError> {
        self.prepare_pdf_without_manifest_guarded(pdf, None)?
            .commit()
    }

    /// Standalone PDF publication guarded by the last sealed command-wide
    /// read ledger. This is used by machine builds that did not request a
    /// manifest; `--force` still cannot replace an input candidate.
    pub fn commit_pdf_without_manifest_with_read_ledger(
        self,
        pdf: VerifiedPdfBytesReceipt,
        read_ledger: HostReadIdentityLedgerToken,
    ) -> Result<PdfSinkCommitReceipt, PdfSinkCommitError> {
        self.prepare_pdf_without_manifest_guarded(pdf, Some(read_ledger))?
            .commit()
    }

    pub fn prepare_pdf_without_manifest(
        self,
        pdf: VerifiedPdfBytesReceipt,
    ) -> Result<PreparedStandalonePdfPublication, PdfSinkCommitError> {
        self.prepare_pdf_without_manifest_guarded(pdf, None)
    }

    pub fn prepare_pdf_without_manifest_with_read_ledger(
        self,
        pdf: VerifiedPdfBytesReceipt,
        read_ledger: HostReadIdentityLedgerToken,
    ) -> Result<PreparedStandalonePdfPublication, PdfSinkCommitError> {
        self.prepare_pdf_without_manifest_guarded(pdf, Some(read_ledger))
    }

    fn prepare_pdf_without_manifest_guarded(
        self,
        pdf: VerifiedPdfBytesReceipt,
        read_ledger: Option<HostReadIdentityLedgerToken>,
    ) -> Result<PreparedStandalonePdfPublication, PdfSinkCommitError> {
        if self.manifest_requested() {
            return Err(PdfSinkCommitError::ManifestPreflightRequired);
        }
        let facts = validate_standalone_pdf_output_facts(&self, &pdf)
            .map_err(PdfSinkCommitError::InvalidFacts)?;
        let staged_pdf = match self.execution.output_path() {
            Some(target) => Some(prepare_file_atomically(
                &self.execution,
                target.as_path(),
                pdf.bytes(),
                self.execution.replace_policy(),
                read_ledger.as_ref(),
            )?),
            None => None,
        };
        Ok(PreparedStandalonePdfPublication {
            output: self,
            pdf,
            facts,
            read_ledger,
            staged_pdf,
        })
    }

    /// Commits a manifest-bound publication that has already passed every
    /// fallible built-manifest check. The manifest and sink receipt are
    /// released together only after the configured sink accepts all bytes.
    /// If the PDF sink rejects the bytes, this consumes the failed-manifest
    /// counterpart sealed by the same preflight and attempts its atomic
    /// publication before returning the original sink error.
    ///
    /// ```compile_fail
    /// # use typaxis_manifest::{BuildOutputCommitContext, PreparedBuiltPublication};
    /// # fn forge(output: &BuildOutputCommitContext, prepared: PreparedBuiltPublication) {
    /// let _ = output.commit_prepared_built(prepared, || Ok::<(), ()>(()));
    /// # }
    /// ```
    pub fn commit_prepared_built(
        self,
        prepared: PreparedBuiltPublication,
    ) -> Result<CommittedBuiltPublication, BuiltPublicationCommitError> {
        match self.commit_prepared_pdf(prepared) {
            Ok(pending) => pending.commit_built_manifest(),
            Err(PreparedPdfCommitError::Invalid(source)) => {
                Err(BuiltPublicationCommitError::Pdf(source))
            }
            Err(PreparedPdfCommitError::SinkFailed { source, failed }) => {
                let failed_manifest = match failed.commit_failed_manifest() {
                    Ok(publication) => FailedManifestPublication::Committed(Box::new(publication)),
                    Err(error) => FailedManifestPublication::CommitError(Box::new(error)),
                };
                Err(BuiltPublicationCommitError::PdfSinkFailed {
                    source,
                    failed_manifest,
                })
            }
            Err(PreparedPdfCommitError::DurabilityUncertain {
                pdf_receipt,
                source,
            }) => Err(BuiltPublicationCommitError::PdfDurability {
                pdf_receipt,
                source,
            }),
        }
    }

    /// Publish only the PDF stage of a fully preflighted built result. The
    /// returned capability owns the terminal built manifest, allowing the CLI
    /// to publish diagnostics between PDF visibility and the terminal record.
    /// Dropping that capability after diagnostics failure leaves the built
    /// manifest unpublished.
    pub fn commit_prepared_pdf(
        self,
        prepared: PreparedBuiltPublication,
    ) -> Result<PendingBuiltManifestPublication, PreparedPdfCommitError> {
        if prepared.binding.output != self.binding() {
            return Err(PreparedPdfCommitError::Invalid(
                PdfSinkCommitError::InvalidFacts(BuildManifestError::OutputReceiptBindingMismatch),
            ));
        }
        if self.execution.manifest_target().is_none() {
            return Err(PreparedPdfCommitError::Invalid(
                PdfSinkCommitError::ManifestPreflightRequired,
            ));
        }
        let pdf_durability = match commit_verified_pdf(
            &self.execution,
            &prepared.pdf,
            prepared.read_ledger.as_ref(),
        ) {
            Ok(durability) => durability,
            Err(source) => {
                let failed = PreparedFailedPublication {
                    binding: prepared.binding,
                    manifest: prepared.failed_manifest,
                    manifest_bytes: prepared.failed_manifest_bytes,
                    read_ledger: prepared.read_ledger,
                };
                return Err(PreparedPdfCommitError::SinkFailed {
                    source,
                    failed: Box::new(PendingFailedManifestPublication {
                        output: self,
                        prepared: failed,
                        staged_manifest: None,
                    }),
                });
            }
        };
        let receipt = self.issue_receipt(prepared.output);
        if let SinkCommitDurability::PublishedButDurabilityUncertain(source) = pdf_durability {
            return Err(PreparedPdfCommitError::DurabilityUncertain {
                pdf_receipt: Box::new(receipt),
                source,
            });
        }
        Ok(PendingBuiltManifestPublication {
            output: self,
            binding: prepared.binding,
            manifest: prepared.manifest,
            manifest_bytes: prepared.manifest_bytes,
            read_ledger: prepared.read_ledger,
            pdf_receipt: receipt,
            staged_manifest: None,
        })
    }

    /// Convert a preflighted built plan to its already-sealed output-null
    /// failed counterpart without attempting PDF. This is used when trace
    /// publication fails before the PDF stage.
    pub fn fail_prepared_built(
        self,
        prepared: PreparedBuiltPublication,
    ) -> Result<PendingFailedManifestPublication, PdfSinkCommitError> {
        if prepared.binding.output != self.binding() {
            return Err(PdfSinkCommitError::InvalidFacts(
                BuildManifestError::OutputReceiptBindingMismatch,
            ));
        }
        Ok(PendingFailedManifestPublication {
            output: self,
            prepared: PreparedFailedPublication {
                binding: prepared.binding,
                manifest: prepared.failed_manifest,
                manifest_bytes: prepared.failed_manifest_bytes,
                read_ledger: prepared.read_ledger,
            },
            staged_manifest: None,
        })
    }

    /// Wrap a directly preflighted processing failure so diagnostics can be
    /// attempted first while preserving one-shot manifest publication.
    pub fn defer_prepared_failed(
        self,
        prepared: PreparedFailedPublication,
    ) -> Result<PendingFailedManifestPublication, ManifestSinkCommitError> {
        if prepared.binding.output != self.binding() {
            return Err(ManifestSinkCommitError::InvalidFacts(
                BuildManifestError::OutputReceiptBindingMismatch,
            ));
        }
        Ok(PendingFailedManifestPublication {
            output: self,
            prepared,
            staged_manifest: None,
        })
    }

    /// Prewrite and fsync a processing-failure manifest before diagnostics are
    /// published. The returned capability remains one-shot and can be consumed
    /// after the diagnostics attempt regardless of that attempt's result.
    pub fn stage_prepared_failed(
        self,
        prepared: PreparedFailedPublication,
    ) -> Result<PendingFailedManifestPublication, ManifestSinkCommitError> {
        if prepared.binding.output != self.binding() {
            return Err(ManifestSinkCommitError::InvalidFacts(
                BuildManifestError::OutputReceiptBindingMismatch,
            ));
        }
        let manifest_target = self
            .execution
            .manifest_target()
            .ok_or(ManifestSinkCommitError::MissingManifestTarget)?;
        let staged_manifest = prepare_file_atomically(
            &self.execution,
            manifest_target.as_path(),
            &prepared.manifest_bytes,
            self.execution.replace_policy(),
            prepared.read_ledger.as_ref(),
        )
        .map_err(map_pdf_error_to_manifest_error)?;
        Ok(PendingFailedManifestPublication {
            output: self,
            prepared,
            staged_manifest: Some(staged_manifest),
        })
    }

    /// Prewrite and fsync the PDF file (when applicable) plus both terminal
    /// manifest alternatives before the first trace/PDF/diagnostics publish.
    /// No target becomes visible during this method.
    pub fn stage_prepared_built(
        self,
        prepared: PreparedBuiltPublication,
    ) -> Result<StagedBuiltPublication, BuiltPublicationStagingError> {
        if prepared.binding.output != self.binding() {
            return Err(BuiltPublicationStagingError::Invalid(
                PdfSinkCommitError::InvalidFacts(BuildManifestError::OutputReceiptBindingMismatch),
            ));
        }
        let manifest_target =
            self.execution
                .manifest_target()
                .ok_or(BuiltPublicationStagingError::Invalid(
                    PdfSinkCommitError::ManifestPreflightRequired,
                ))?;
        let staged_pdf = match self.execution.output_path() {
            Some(target) => Some(
                prepare_file_atomically(
                    &self.execution,
                    target.as_path(),
                    prepared.pdf.bytes(),
                    self.execution.replace_policy(),
                    prepared.read_ledger.as_ref(),
                )
                .map_err(BuiltPublicationStagingError::Pdf)?,
            ),
            None => None,
        };
        let staged_built_manifest = prepare_file_atomically(
            &self.execution,
            manifest_target.as_path(),
            &prepared.manifest_bytes,
            self.execution.replace_policy(),
            prepared.read_ledger.as_ref(),
        )
        .map_err(BuiltPublicationStagingError::BuiltManifest)?;
        let staged_failed_manifest = prepare_file_atomically(
            &self.execution,
            manifest_target.as_path(),
            &prepared.failed_manifest_bytes,
            self.execution.replace_policy(),
            prepared.read_ledger.as_ref(),
        )
        .map_err(BuiltPublicationStagingError::FailedManifest)?;
        Ok(StagedBuiltPublication {
            output: self,
            prepared,
            staged_pdf,
            staged_built_manifest,
            staged_failed_manifest,
        })
    }

    fn commit_pending_built_manifest(
        self,
        binding: PublicationBinding,
        manifest: ValidatedBuildManifest,
        manifest_bytes: Vec<u8>,
        read_ledger: Option<HostReadIdentityLedgerToken>,
        receipt: PdfSinkCommitReceipt,
        staged_manifest: Option<PreparedAtomicFile>,
    ) -> Result<CommittedBuiltPublication, BuiltPublicationCommitError> {
        let manifest_target = self
            .execution
            .manifest_target()
            .expect("pending built publication retains its manifest target")
            .clone();
        let manifest_durability = match match staged_manifest {
            Some(prepared) => {
                publish_prepared_file(&self.execution, prepared, read_ledger.as_ref())
            }
            None => commit_file_bytes(
                &self.execution,
                manifest_target.as_path(),
                &manifest_bytes,
                read_ledger.as_ref(),
            ),
        } {
            Ok(durability) => durability,
            Err(error) => {
                return Err(match error {
                    PdfSinkCommitError::Execution(source) => {
                        BuiltPublicationCommitError::ManifestExecution {
                            pdf_receipt: Box::new(receipt),
                            source,
                        }
                    }
                    PdfSinkCommitError::Io(source) => BuiltPublicationCommitError::ManifestIo {
                        pdf_receipt: Box::new(receipt),
                        source,
                    },
                    PdfSinkCommitError::InvalidFacts(_)
                    | PdfSinkCommitError::ManifestPreflightRequired
                    | PdfSinkCommitError::StdoutPartial { .. } => {
                        BuiltPublicationCommitError::ManifestInvariant {
                            pdf_receipt: Box::new(receipt),
                        }
                    }
                    PdfSinkCommitError::PublishedButDurabilityUncertain { .. } => {
                        BuiltPublicationCommitError::ManifestInvariant {
                            pdf_receipt: Box::new(receipt),
                        }
                    }
                })
            }
        };
        let manifest_receipt = ManifestSinkCommitReceipt {
            binding,
            bytes: manifest_bytes.len() as u64,
        };
        let committed = CommittedBuiltPublication {
            manifest,
            receipt,
            manifest_receipt,
        };
        match manifest_durability {
            SinkCommitDurability::Durable => Ok(committed),
            SinkCommitDurability::PublishedButDurabilityUncertain(source) => {
                Err(BuiltPublicationCommitError::ManifestDurability {
                    publication: Box::new(committed),
                    source,
                })
            }
        }
    }

    /// Atomically publishes a preflighted terminal failed manifest without
    /// emitting PDF bytes. The output context is consumed even on failure, so
    /// another terminal result cannot be published through the same session.
    pub fn commit_prepared_failed(
        self,
        prepared: PreparedFailedPublication,
    ) -> Result<CommittedFailedPublication, ManifestSinkCommitError> {
        self.commit_prepared_failed_inner(prepared, None)
    }

    fn commit_prepared_failed_inner(
        self,
        prepared: PreparedFailedPublication,
        staged_manifest: Option<PreparedAtomicFile>,
    ) -> Result<CommittedFailedPublication, ManifestSinkCommitError> {
        if prepared.binding.output != self.binding() {
            return Err(ManifestSinkCommitError::InvalidFacts(
                BuildManifestError::OutputReceiptBindingMismatch,
            ));
        }
        let manifest_target = self
            .execution
            .manifest_target()
            .ok_or(ManifestSinkCommitError::MissingManifestTarget)?
            .clone();
        let durability = match staged_manifest {
            Some(staged) => {
                publish_prepared_file(&self.execution, staged, prepared.read_ledger.as_ref())
            }
            None => commit_file_bytes(
                &self.execution,
                manifest_target.as_path(),
                &prepared.manifest_bytes,
                prepared.read_ledger.as_ref(),
            ),
        }
        .map_err(map_pdf_error_to_manifest_error)?;
        let receipt = ManifestSinkCommitReceipt {
            binding: prepared.binding,
            bytes: prepared.manifest_bytes.len() as u64,
        };
        let committed = CommittedFailedPublication {
            manifest: prepared.manifest,
            receipt,
        };
        match durability {
            SinkCommitDurability::Durable => Ok(committed),
            SinkCommitDurability::PublishedButDurabilityUncertain(source) => {
                Err(ManifestSinkCommitError::PublishedButDurabilityUncertain {
                    publication: Box::new(committed),
                    source,
                })
            }
        }
    }

    fn issue_receipt(&self, facts: PreparedPdfOutputFacts) -> PdfSinkCommitReceipt {
        PdfSinkCommitReceipt {
            binding: self.binding(),
            selected_fingerprint: facts.selected_fingerprint,
            sink: facts.sink,
            bytes: facts.bytes,
            sha256: facts.sha256,
            page_count: facts.page_count,
            pdf_object_count: facts.pdf_object_count,
        }
    }

    fn binding(&self) -> OutputCommitBinding {
        OutputCommitBinding {
            session: self.session,
            config_fingerprint: self.config_fingerprint,
            input_profile: self.input_profile,
            stream_compression: self.stream_compression,
            limits: self.limits.clone(),
            execution: self.execution.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OutputCommitSessionId(NonZeroU64);
impl OutputCommitSessionId {
    fn allocate() -> Result<Self, BuildOutputCommitContextError> {
        let value = OUTPUT_SESSION_COUNTER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| BuildOutputCommitContextError::SessionIdentityExhausted)?;
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(BuildOutputCommitContextError::SessionIdentityExhausted)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OutputCommitBinding {
    session: OutputCommitSessionId,
    config_fingerprint: EffectiveConfigFingerprint,
    input_profile: BuildInputProfile,
    stream_compression: PdfStreamCompression,
    limits: ValidatedResourceLimits,
    execution: BuildExecutionContext,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ManifestPublicationContext {
    session: PublicationSessionId,
    output: OutputCommitBinding,
    manifest_target: HostPath,
    config_fingerprint: EffectiveConfigFingerprint,
    input_profile: BuildInputProfile,
    stream_compression: PdfStreamCompression,
    data_versions: DataVersions,
    engine: EngineRecord,
    limits: ValidatedResourceLimits,
    execution: BuildExecutionContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestPublicationError {
    MissingManifestTarget,
    DataTableMismatch,
    OutputContextMismatch,
    SessionIdentityExhausted,
}

impl ManifestPublicationContext {
    pub fn new(
        config: &EffectiveConfig,
        output: &BuildOutputCommitContext,
        shaper: ShaperIdentity,
        tables: &ResolvedDataTables,
    ) -> Result<Self, ManifestPublicationError> {
        if tables.versions() != config.data_versions() {
            return Err(ManifestPublicationError::DataTableMismatch);
        }
        if config.fingerprint() != output.config_fingerprint
            || config.stream_compression() != output.stream_compression
            || config.limits() != &output.limits
        {
            return Err(ManifestPublicationError::OutputContextMismatch);
        }
        let manifest_target = output
            .execution
            .manifest_target()
            .ok_or(ManifestPublicationError::MissingManifestTarget)?
            .clone();
        let session = PublicationSessionId::allocate()?;
        Ok(Self {
            session,
            output: output.binding(),
            manifest_target,
            config_fingerprint: config.fingerprint(),
            input_profile: output.input_profile,
            stream_compression: config.stream_compression(),
            data_versions: DataVersions::from_runtime(tables, shaper),
            engine: EngineRecord::from_identity(&EngineIdentity::compiled()),
            limits: config.limits().clone(),
            execution: output.execution.clone(),
        })
    }
    pub const fn manifest_target(&self) -> &HostPath {
        &self.manifest_target
    }
    pub const fn config_fingerprint(&self) -> EffectiveConfigFingerprint {
        self.config_fingerprint
    }
    pub const fn input_profile(&self) -> BuildInputProfile {
        self.input_profile
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }
    pub const fn output_sink(&self) -> OutputSink {
        self.execution.output_sink()
    }

    /// Starts a publication-bound ledger. The ledger owns only facts copied
    /// from phase artifacts; callers cannot insert manifest records directly.
    pub fn begin_admission_ledger(&self) -> ManifestAdmissionLedger {
        ManifestAdmissionLedger {
            binding: self.binding(),
            machine: self
                .input_profile
                .machine_profile()
                .map(|_| MachineLedgerState::no_input()),
            package_input: None,
            sources: BTreeMap::new(),
            fonts: BTreeMap::new(),
            images: BTreeMap::new(),
            expected_fonts: Vec::new(),
            expected_images: Vec::new(),
            package_epoch: None,
            resource_progress: None,
            resource_fingerprint: None,
        }
    }

    /// Preflights the complete built publication without performing output
    /// I/O. The returned one-shot token owns the serializer-issued bytes and a
    /// fully validated manifest, but does not expose that manifest before the
    /// configured sink has accepted the bytes.
    pub fn prepare_built(
        self,
        package: &ValidatedParsedPackage,
        admitted: AdmittedResourceLedgerToken<'_>,
        pagination: &PaginationResult,
        pdf: VerifiedPdfBytesReceipt,
    ) -> Result<PreparedBuiltPublication, BuildManifestError> {
        if self.input_profile != BuildInputProfile::ReferenceSource1 {
            return Err(BuildManifestError::InputProfileMismatch);
        }
        let output = validate_pdf_output_facts(&self, pagination, &pdf)?;
        let manifest = prepare_built_manifest(&self, package, admitted, pagination, output)?;
        let manifest_bytes = manifest.manifest().to_canonical_json_bytes();
        let mut failure_ledger = self.begin_admission_ledger();
        failure_ledger.admit_validated_package_sources(package)?;
        failure_ledger.admit_resources(admitted)?;
        let failed_manifest =
            ValidatedBuildManifest::failed(&self, &failure_ledger, Some(pagination))?;
        let failed_manifest_bytes = failed_manifest.manifest().to_canonical_json_bytes();
        Ok(PreparedBuiltPublication {
            binding: self.binding(),
            manifest,
            manifest_bytes,
            failed_manifest,
            failed_manifest_bytes,
            pdf,
            output,
            read_ledger: None,
        })
    }

    /// Machine-only built preflight. The adopted capability receipt,
    /// provenance, complete resource session, selected pagination result, and
    /// serializer receipt are closed together before any publication I/O.
    pub fn prepare_machine_built(
        self,
        package: &ValidatedMachinePackage,
        capability: &MachinePdfPreflightReceipt,
        admitted: AdmittedResourceLedgerToken<'_>,
        pagination: &PaginationResult,
        layout_facts: StagingMachineLayoutFacts,
        pdf: VerifiedPdfBytesReceipt,
    ) -> Result<PreparedBuiltPublication, BuildManifestError> {
        if self.input_profile.machine_profile() != Some(capability.profile()) {
            return Err(BuildManifestError::InputProfileMismatch);
        }
        Self::validate_machine_table_layout_facts(
            package,
            capability,
            pagination,
            &layout_facts,
            &pdf,
        )?;
        let base_page_count = u32::try_from(pagination.selected_pages().len())
            .map_err(|_| BuildManifestError::PageLimit)?;
        let expected_page_count = if capability.profile() == MachinePdfProfileId::TABLE_1 {
            layout_facts
                .table_layouts()
                .iter()
                .filter_map(StagingTableLayoutFacts::target_page_end)
                .max()
                .unwrap_or(base_page_count)
                .max(base_page_count)
        } else if capability.profile() == MachinePdfProfileId::FOOTNOTE_1 {
            u32::try_from(
                layout_facts
                    .footnote_layout()
                    .ok_or(BuildManifestError::MachineCapabilityMismatch)?
                    .pages()
                    .len(),
            )
            .map_err(|_| BuildManifestError::PageLimit)?
        } else {
            if !layout_facts.table_layouts().is_empty() {
                return Err(BuildManifestError::MachineCapabilityMismatch);
            }
            base_page_count
        };
        let output = validate_pdf_output_facts_with_page_count(
            &self,
            pagination,
            &pdf,
            expected_page_count,
        )?;
        let (manifest, ledger) = prepare_machine_built_manifest(
            &self,
            package,
            capability,
            admitted,
            pagination,
            layout_facts,
            output,
        )?;
        let manifest_bytes = manifest.manifest().to_canonical_json_bytes();
        let failed_manifest = ValidatedBuildManifest::failed(&self, &ledger, Some(pagination))?;
        let failed_manifest_bytes = failed_manifest.manifest().to_canonical_json_bytes();
        let read_ledger = package
            .provenance()
            .admission()
            .read_ledger_token()
            .map_err(|_| BuildManifestError::ReadLedgerUnavailable)?;
        Ok(PreparedBuiltPublication {
            binding: self.binding(),
            manifest,
            manifest_bytes,
            failed_manifest,
            failed_manifest_bytes,
            pdf,
            output,
            read_ledger: Some(read_ledger),
        })
    }

    /// Contract-1.3 terminal preflight for an ADR-0031 selected-layout
    /// receipt. The advanced projector has already reopened Display and PDF;
    /// this owner binds that projection to admission, resources, output, and
    /// the ordinary machine capability receipt before any publication I/O.
    pub fn prepare_advanced_machine_built(
        self,
        package: &ValidatedMachinePackage,
        capability: &MachinePdfPreflightReceipt,
        admitted: AdmittedResourceLedgerToken<'_>,
        advanced: StagingAdvancedPaginationManifest,
        pdf: VerifiedPdfBytesReceipt,
    ) -> Result<PreparedBuiltPublication, BuildManifestError> {
        let profile = capability.profile();
        if self.input_profile.machine_profile() != Some(profile)
            || !matches!(
                profile,
                MachinePdfProfileId::Columns1
                    | MachinePdfProfileId::Float1
                    | MachinePdfProfileId::HeaderFooter1
            )
            || package.contract() != typaxis_core::DocumentPackageContractId::V1_3
            || package.advanced_view().is_none()
            || advanced.profile() != profile
            || advanced.profile_receipt_sha256() != capability.profile_receipt_sha256()
            || capability.verify(profile, package).is_err()
        {
            return Err(BuildManifestError::MachineCapabilityMismatch);
        }
        let selected_fingerprint =
            LayoutStateFingerprint::from_untrusted_bytes(advanced.selected_layout_sha256());
        let output = validate_pdf_receipt_facts(
            self.config_fingerprint,
            self.stream_compression,
            &self.limits,
            self.output_sink(),
            &pdf,
        )?;
        if output.selected_fingerprint != selected_fingerprint
            || output.page_count != advanced.page_count()
        {
            return Err(BuildManifestError::PdfGraphReceiptMismatch);
        }
        let layout = LayoutRecord::new(
            LayoutStatus::Converged,
            NonZeroU16::new(1).expect("one is nonzero"),
            NonZeroU16::new(1).expect("one is nonzero"),
            selected_fingerprint,
        )
        .ok_or(BuildManifestError::IncompleteLayoutAdmission)?
        .bind_machine_capability(capability, Some(advanced.flow_registry_sha256()))?;

        let mut ledger = self.begin_admission_ledger();
        ledger.admit_validated_machine_package(package)?;
        ledger.admit_machine_capability(package, capability)?;
        validate_complete_resource_closure(package.package(), admitted)?;
        ledger.admit_resources(admitted)?;
        let records = ledger.manifest_records();
        let output_record = OutputRecord {
            sink: output.sink,
            bytes: output.bytes,
            sha256: output.sha256,
            page_count: output.page_count,
            pdf_object_count: output.pdf_object_count,
        };
        let manifest = BuildManifest::terminal(
            &self,
            BuildStatus::Built,
            records,
            Some(layout),
            Some(output_record),
            ManifestProfileFacts {
                advanced_pagination: Some(advanced.canonical_jcs().to_owned()),
                ..ManifestProfileFacts::default()
            },
        );
        let manifest =
            ValidatedBuildManifest::new(manifest, ManifestExpectations::from_publication(&self))?;
        let manifest_bytes = manifest.manifest().to_canonical_json_bytes();
        let failed_manifest = ValidatedBuildManifest::failed(&self, &ledger, None)?;
        let failed_manifest_bytes = failed_manifest.manifest().to_canonical_json_bytes();
        let read_ledger = package
            .provenance()
            .admission()
            .read_ledger_token()
            .map_err(|_| BuildManifestError::ReadLedgerUnavailable)?;
        Ok(PreparedBuiltPublication {
            binding: self.binding(),
            manifest,
            manifest_bytes,
            failed_manifest,
            failed_manifest_bytes,
            pdf,
            output,
            read_ledger: Some(read_ledger),
        })
    }

    fn validate_machine_table_layout_facts(
        package: &ValidatedMachinePackage,
        capability: &MachinePdfPreflightReceipt,
        pagination: &PaginationResult,
        layout_facts: &StagingMachineLayoutFacts,
        pdf: &VerifiedPdfBytesReceipt,
    ) -> Result<(), BuildManifestError> {
        if capability.profile() == MachinePdfProfileId::FOOTNOTE_1 {
            let footnote = layout_facts
                .footnote_layout()
                .ok_or(BuildManifestError::MachineCapabilityMismatch)?;
            return if layout_facts.table_layouts().is_empty()
                && footnote.body_layout_sha256() == pagination.final_fingerprint().bytes()
                && pdf.footnote_display_sha256() == Some(footnote.paint_sha256())
                && footnote.pages().len() == pagination.selected_pages().len()
                && footnote
                    .pages()
                    .iter()
                    .enumerate()
                    .all(|(index, page)| page.page_index() as usize == index)
            {
                Ok(())
            } else {
                Err(BuildManifestError::MachineCapabilityMismatch)
            };
        }
        if layout_facts.footnote_layout().is_some() || pdf.footnote_display_sha256().is_some() {
            return Err(BuildManifestError::MachineCapabilityMismatch);
        }
        if capability.profile() != MachinePdfProfileId::TABLE_1 {
            return if layout_facts.table_layouts().is_empty() {
                Ok(())
            } else {
                Err(BuildManifestError::MachineCapabilityMismatch)
            };
        }
        let expected_owners: Vec<_> = package
            .package()
            .package()
            .document
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Table { node_id, .. } => Some(node_id.get()),
                _ => None,
            })
            .collect();
        let tables = layout_facts.table_layouts();
        let package_sha256 = package.provenance().canonical_jcs_sha256().into_bytes();
        let flow_registry_sha256 = layout_facts
            .flow_registry_sha256()
            .ok_or(BuildManifestError::MachineCapabilityMismatch)?;
        if expected_owners.len() != tables.len()
            || tables.iter().zip(expected_owners).any(|(table, owner)| {
                table.table_node_id() != owner
                    || table.package_sha256() != package_sha256
                    || table.flow_registry_sha256() != flow_registry_sha256
            })
            || tables.windows(2).any(|pair| {
                pair[0]
                    .target_page_end()
                    .map_or(true, |end| end > pair[1].target_page_start())
            })
        {
            return Err(BuildManifestError::MachineCapabilityMismatch);
        }
        Ok(())
    }

    /// Preflights a terminal failed manifest from facts admitted by this exact
    /// publication session. The validated record remains inaccessible until
    /// the paired output owner atomically publishes its canonical bytes.
    pub fn prepare_failed(
        self,
        ledger: ManifestAdmissionLedger,
        pagination: Option<&PaginationResult>,
    ) -> Result<PreparedFailedPublication, BuildManifestError> {
        let manifest = ValidatedBuildManifest::failed(&self, &ledger, pagination)?;
        let manifest_bytes = manifest.manifest().to_canonical_json_bytes();
        Ok(PreparedFailedPublication {
            binding: self.binding(),
            manifest,
            manifest_bytes,
            read_ledger: None,
        })
    }

    fn binding(&self) -> PublicationBinding {
        PublicationBinding {
            session: self.session,
            output: self.output.clone(),
            config_fingerprint: self.config_fingerprint,
            execution: self.execution.clone(),
            limits: self.limits.clone(),
            stream_compression: self.stream_compression,
            data_versions: self.data_versions.clone(),
            engine: self.engine.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PublicationSessionId(NonZeroU64);
impl PublicationSessionId {
    fn allocate() -> Result<Self, ManifestPublicationError> {
        let value = PUBLICATION_SESSION_COUNTER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| ManifestPublicationError::SessionIdentityExhausted)?;
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(ManifestPublicationError::SessionIdentityExhausted)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PublicationBinding {
    session: PublicationSessionId,
    output: OutputCommitBinding,
    config_fingerprint: EffectiveConfigFingerprint,
    execution: BuildExecutionContext,
    limits: ValidatedResourceLimits,
    stream_compression: PdfStreamCompression,
    data_versions: DataVersions,
    engine: EngineRecord,
}

/// Highest trusted machine build phase admitted into a manifest ledger.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ManifestAdmissionStage {
    NoInput,
    RawPackageAdmitted,
    PackageDecoded,
    SourcesAdmitted,
    PackageValidated,
    CapabilityValidated,
    ResourcesAdmitted,
    LayoutSelected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MachineLedgerState {
    stage: ManifestAdmissionStage,
    session: Option<MachineInputSessionIdentity>,
    fingerprint: Option<MachineInputFingerprint>,
}

impl MachineLedgerState {
    const fn no_input() -> Self {
        Self {
            stage: ManifestAdmissionStage::NoInput,
            session: None,
            fingerprint: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedFontResource {
    id: FontFaceId,
    uri: PortablePath,
    family: String,
    face_index: u32,
    expected_sha256: Option<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedImageResource {
    id: ImageResourceId,
    uri: PortablePath,
    expected_sha256: Option<[u8; 32]>,
}

type ManifestFontRecords = BTreeMap<FontFaceId, FontRecord>;
type ManifestImageRecords = BTreeMap<ImageResourceId, ImageRecord>;

struct ManifestTerminalRecords {
    package_input: Option<PackageInputRecord>,
    inputs: Vec<FileRecord>,
    fonts: Vec<FontRecord>,
    images: Vec<ImageRecord>,
}

/// Canonical facts admitted so far for a terminal failed manifest. This token
/// is tied to one publication context and can only copy facts from owner types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestAdmissionLedger {
    binding: PublicationBinding,
    machine: Option<MachineLedgerState>,
    package_input: Option<PackageInputRecord>,
    sources: BTreeMap<SourceId, FileRecord>,
    fonts: BTreeMap<FontFaceId, FontRecord>,
    images: BTreeMap<ImageResourceId, ImageRecord>,
    expected_fonts: Vec<ExpectedFontResource>,
    expected_images: Vec<ExpectedImageResource>,
    package_epoch: Option<PackageEpochIdentity>,
    resource_progress: Option<ResourceAdmissionProgressToken>,
    resource_fingerprint: Option<AdmittedResourceFingerprint>,
}
impl ManifestAdmissionLedger {
    pub fn admit_source(&mut self, source: &SourceRecord) -> Result<(), BuildManifestError> {
        if self.machine.is_some() {
            return Err(BuildManifestError::InputProfileMismatch);
        }
        if self.package_epoch.is_some() {
            return Err(BuildManifestError::DuplicateAdmissionRecord);
        }
        let expected = u32::try_from(self.sources.len())
            .map_err(|_| BuildManifestError::NonDenseAdmissionSource)?;
        if source.source_id().get() != expected {
            return Err(BuildManifestError::NonDenseAdmissionSource);
        }
        if self
            .sources
            .values()
            .any(|record| record.uri == *source.uri())
        {
            return Err(BuildManifestError::DuplicateAdmissionRecord);
        }
        let mut sources = self.sources.clone();
        if sources
            .insert(
                source.source_id(),
                FileRecord {
                    uri: source.uri().clone(),
                    bytes: u64::from(source.utf8_byte_length()),
                    sha256: source.content_hash(),
                },
            )
            .is_some()
        {
            return Err(BuildManifestError::DuplicateAdmissionRecord);
        }
        validate_admission_limits(&sources, &self.fonts, &self.images, &self.binding)?;
        self.sources = sources;
        Ok(())
    }

    pub fn admit_validated_package_sources(
        &mut self,
        package: &ValidatedParsedPackage,
    ) -> Result<(), BuildManifestError> {
        if self.machine.is_some() {
            return Err(BuildManifestError::InputProfileMismatch);
        }
        let mut candidate = self.clone();
        for source in package.package().sources.records() {
            candidate.admit_source(source)?;
        }
        candidate.package_epoch = Some(package.epoch_identity().clone());
        candidate.expected_fonts = package
            .package()
            .resources
            .font_faces
            .iter()
            .map(|resource| ExpectedFontResource {
                id: resource.font_face_id,
                uri: resource.uri.clone(),
                family: resource.family.clone(),
                face_index: resource.face_index,
                expected_sha256: resource.expected_sha256,
            })
            .collect();
        candidate.expected_images = package
            .package()
            .resources
            .images
            .iter()
            .map(|resource| ExpectedImageResource {
                id: resource.image_id,
                uri: resource.uri.clone(),
                expected_sha256: resource.expected_sha256,
            })
            .collect();
        *self = candidate;
        Ok(())
    }

    /// Projects a machine-input owner's sealed progress snapshot. No record
    /// field is accepted independently, and later snapshots must retain every
    /// already-admitted package/source/session fact exactly.
    pub fn admit_machine_input_progress(
        &mut self,
        progress: &MachineInputProgress,
    ) -> Result<(), BuildManifestError> {
        let mut candidate = self.clone();
        candidate.admit_machine_input_progress_inner(progress)?;
        *self = candidate;
        Ok(())
    }

    fn admit_machine_input_progress_inner(
        &mut self,
        progress: &MachineInputProgress,
    ) -> Result<(), BuildManifestError> {
        let state = self
            .machine
            .as_mut()
            .ok_or(BuildManifestError::InputProfileMismatch)?;
        let incoming_stage = match progress.stage() {
            MachineInputStage::NoInput => ManifestAdmissionStage::NoInput,
            MachineInputStage::RawPackageAdmitted => ManifestAdmissionStage::RawPackageAdmitted,
            MachineInputStage::PackageDecoded => ManifestAdmissionStage::PackageDecoded,
            MachineInputStage::SourcesAdmitted => ManifestAdmissionStage::SourcesAdmitted,
        };
        if incoming_stage < state.stage || state.stage > ManifestAdmissionStage::SourcesAdmitted {
            return Err(BuildManifestError::MachineProgressRegression);
        }

        let incoming_session = progress.session_identity();
        if state
            .session
            .as_ref()
            .zip(incoming_session)
            .is_some_and(|(established, incoming)| established != incoming)
        {
            return Err(BuildManifestError::MachineSessionMismatch);
        }

        let package = progress.package().map(|raw| PackageInputRecord {
            uri: raw.uri().clone(),
            bytes: raw.bytes(),
            sha256: raw.sha256(),
            contract: progress.decoded().map(|decoded| decoded.contract()),
            canonical_sha256: progress.decoded().map(|decoded| decoded.canonical_sha256()),
            profile_receipt_sha256: None,
        });
        let shape_is_valid = match incoming_stage {
            ManifestAdmissionStage::NoInput => {
                incoming_session.is_none()
                    && package.is_none()
                    && progress.decoded().is_none()
                    && progress.sources().is_empty()
                    && progress.fingerprint().is_none()
            }
            ManifestAdmissionStage::RawPackageAdmitted => {
                incoming_session.is_some()
                    && package.is_some()
                    && progress.decoded().is_none()
                    && progress.sources().is_empty()
                    && progress.fingerprint().is_none()
            }
            ManifestAdmissionStage::PackageDecoded => {
                incoming_session.is_some()
                    && package.as_ref().is_some_and(PackageInputRecord::is_decoded)
                    && progress.sources().is_empty()
                    && progress.fingerprint().is_none()
            }
            ManifestAdmissionStage::SourcesAdmitted => {
                incoming_session.is_some()
                    && package.as_ref().is_some_and(PackageInputRecord::is_decoded)
                    && progress.sources().len() == 1
                    && progress.fingerprint().is_some()
            }
            ManifestAdmissionStage::PackageValidated
            | ManifestAdmissionStage::CapabilityValidated
            | ManifestAdmissionStage::ResourcesAdmitted
            | ManifestAdmissionStage::LayoutSelected => false,
        };
        if !shape_is_valid {
            return Err(BuildManifestError::MachinePackageMismatch);
        }
        if self
            .package_input
            .as_ref()
            .zip(package.as_ref())
            .is_some_and(|(established, incoming)| {
                established.uri != incoming.uri
                    || established.bytes != incoming.bytes
                    || established.sha256 != incoming.sha256
                    || established
                        .contract
                        .is_some_and(|value| Some(value) != incoming.contract)
                    || established
                        .canonical_sha256
                        .is_some_and(|value| Some(value) != incoming.canonical_sha256)
            })
            || (self.package_input.is_some() && package.is_none())
        {
            return Err(BuildManifestError::MachinePackageMismatch);
        }

        let mut sources = BTreeMap::new();
        for (expected, source) in progress.sources().iter().enumerate() {
            if source.source_id().get()
                != u32::try_from(expected)
                    .map_err(|_| BuildManifestError::NonDenseAdmissionSource)?
                || sources
                    .values()
                    .any(|record: &FileRecord| record.uri == *source.uri())
            {
                return Err(BuildManifestError::NonDenseAdmissionSource);
            }
            sources.insert(
                source.source_id(),
                FileRecord {
                    uri: source.uri().clone(),
                    bytes: source.bytes(),
                    sha256: source.sha256(),
                },
            );
        }
        if !self.sources.is_empty() && self.sources != sources {
            return Err(BuildManifestError::MachinePackageMismatch);
        }
        if !sources.is_empty() {
            validate_admission_limits(&sources, &self.fonts, &self.images, &self.binding)?;
        }
        if package
            .as_ref()
            .is_some_and(|record| record.bytes > JSON_SAFE_INTEGER_MAX as u64)
        {
            return Err(BuildManifestError::IntegerNotJsonSafe);
        }
        if state
            .fingerprint
            .zip(progress.fingerprint())
            .is_some_and(|(established, incoming)| established != incoming)
            || (state.fingerprint.is_some() && progress.fingerprint().is_none())
        {
            return Err(BuildManifestError::MachinePackageMismatch);
        }

        state.stage = incoming_stage;
        if state.session.is_none() {
            state.session = incoming_session.cloned();
        }
        if state.fingerprint.is_none() {
            state.fingerprint = progress.fingerprint();
        }
        self.package_input = package.or_else(|| self.package_input.take());
        if self.sources.is_empty() {
            self.sources = sources;
        }
        Ok(())
    }

    /// Admits syntax validation only from the wrapper that still owns exact
    /// machine provenance and the trusted parsed package.
    pub fn admit_validated_machine_package(
        &mut self,
        package: &ValidatedMachinePackage,
    ) -> Result<(), BuildManifestError> {
        let mut candidate = self.clone();
        candidate.admit_machine_input_progress_inner(package.provenance().progress())?;
        let state = candidate
            .machine
            .as_mut()
            .ok_or(BuildManifestError::InputProfileMismatch)?;
        if state.stage != ManifestAdmissionStage::SourcesAdmitted
            || state.session.as_ref() != Some(package.provenance().session_identity())
            || state.fingerprint != Some(package.provenance().fingerprint())
        {
            return Err(BuildManifestError::MachinePackageMismatch);
        }
        let package_input = candidate
            .package_input
            .as_ref()
            .ok_or(BuildManifestError::MachinePackageMismatch)?;
        if package_input.sha256 != package.provenance().raw_sha256().into_bytes()
            || package_input.canonical_sha256
                != Some(package.provenance().canonical_jcs_sha256().into_bytes())
            || package.package().package().sources.records().len() != candidate.sources.len()
            || package
                .package()
                .package()
                .sources
                .records()
                .iter()
                .any(|source| {
                    candidate
                        .sources
                        .get(&source.source_id())
                        .map_or(true, |record| {
                            record.uri != *source.uri()
                                || record.bytes != u64::from(source.utf8_byte_length())
                                || record.sha256 != source.content_hash()
                        })
                })
        {
            return Err(BuildManifestError::MachinePackageMismatch);
        }
        candidate.package_epoch = Some(package.package().epoch_identity().clone());
        candidate.expected_fonts = package
            .package()
            .package()
            .resources
            .font_faces
            .iter()
            .map(|resource| ExpectedFontResource {
                id: resource.font_face_id,
                uri: resource.uri.clone(),
                family: resource.family.clone(),
                face_index: resource.face_index,
                expected_sha256: resource.expected_sha256,
            })
            .collect();
        candidate.expected_images = package
            .package()
            .package()
            .resources
            .images
            .iter()
            .map(|resource| ExpectedImageResource {
                id: resource.image_id,
                uri: resource.uri.clone(),
                expected_sha256: resource.expected_sha256,
            })
            .collect();
        state.stage = ManifestAdmissionStage::PackageValidated;
        *self = candidate;
        Ok(())
    }

    /// Admits the capability gate by asking its non-forgeable receipt to
    /// verify the exact package and profile already bound to this ledger.
    pub fn admit_machine_capability(
        &mut self,
        package: &ValidatedMachinePackage,
        receipt: &MachinePdfPreflightReceipt,
    ) -> Result<(), BuildManifestError> {
        let profile = self
            .binding
            .output
            .input_profile
            .machine_profile()
            .ok_or(BuildManifestError::InputProfileMismatch)?;
        let state = self
            .machine
            .as_mut()
            .ok_or(BuildManifestError::InputProfileMismatch)?;
        if state.stage != ManifestAdmissionStage::PackageValidated
            || state.session.as_ref() != Some(package.provenance().session_identity())
            || state.fingerprint != Some(package.provenance().fingerprint())
            || self.package_epoch.as_ref() != Some(package.package().epoch_identity())
            || receipt.verify(profile, package).is_err()
        {
            return Err(BuildManifestError::MachineCapabilityMismatch);
        }
        self.package_input
            .as_mut()
            .ok_or(BuildManifestError::MachineCapabilityMismatch)?
            .profile_receipt_sha256 = Some(receipt.profile_receipt_sha256());
        state.stage = ManifestAdmissionStage::CapabilityValidated;
        Ok(())
    }

    /// Replaces the resource projection with a later verified snapshot from
    /// the same resolver session. Partial resource progress never becomes a
    /// layout capability.
    pub fn admit_resource_progress(
        &mut self,
        progress: ResourceAdmissionProgressToken,
    ) -> Result<(), BuildManifestError> {
        if let Some(state) = self.machine.as_ref() {
            if state.stage != ManifestAdmissionStage::CapabilityValidated {
                return Err(BuildManifestError::MachineProgressRegression);
            }
        } else if self.package_epoch.is_none() {
            return Err(BuildManifestError::MachineProgressRegression);
        }
        if self
            .resource_progress
            .as_ref()
            .is_some_and(|previous| progress != *previous && !progress.continues(previous))
        {
            return Err(BuildManifestError::ResourceProgressMismatch);
        }
        validate_expected_resources(
            &progress,
            &self.expected_fonts,
            &self.expected_images,
            false,
        )?;
        let (fonts, images) = resource_progress_records(&progress)?;
        validate_admission_limits(&self.sources, &fonts, &images, &self.binding)?;
        self.fonts = fonts;
        self.images = images;
        self.resource_progress = Some(progress);
        Ok(())
    }

    /// Copies the complete canonical resource set after resource admission.
    /// The later built factory additionally matches these facts against the
    /// exact validated package declarations.
    pub fn admit_resources(
        &mut self,
        admitted: AdmittedResourceLedgerToken<'_>,
    ) -> Result<(), BuildManifestError> {
        if self.resource_fingerprint.is_some() {
            return Err(BuildManifestError::DuplicateAdmissionRecord);
        }
        let mut fonts = BTreeMap::new();
        for font in admitted.fonts() {
            if fonts
                .insert(
                    font.font_face_id(),
                    FontRecord {
                        font_face_id: font.font_face_id(),
                        uri: font.uri().clone(),
                        face_index: font.face_index(),
                        bytes: font.byte_length(),
                        sha256: font.content_hash(),
                        units_per_em: font.metadata().units_per_em,
                        glyph_count: font.metadata().glyph_count,
                    },
                )
                .is_some()
            {
                return Err(BuildManifestError::DuplicateAdmissionRecord);
            }
        }
        let mut images = BTreeMap::new();
        for image in admitted.images() {
            if images
                .insert(
                    image.image_id(),
                    ImageRecord {
                        image_id: image.image_id(),
                        uri: image.uri().clone(),
                        bytes: image.byte_length(),
                        sha256: image.content_hash(),
                        attested_media_kind: image.media_kind().as_str(),
                        pixel_width: image.width().get(),
                        pixel_height: image.height().get(),
                        decoded_bytes: image.decoded_bytes(),
                    },
                )
                .is_some()
            {
                return Err(BuildManifestError::DuplicateAdmissionRecord);
            }
        }
        if let Some(state) = self.machine.as_mut() {
            if state.stage != ManifestAdmissionStage::CapabilityValidated {
                return Err(BuildManifestError::MachineProgressRegression);
            }
            if self
                .resource_progress
                .as_ref()
                .is_some_and(|progress| !admitted.continues_progress(progress))
            {
                return Err(BuildManifestError::ResourceProgressMismatch);
            }
            let complete_progress = admitted.ledger().progress_token();
            validate_expected_resources(
                &complete_progress,
                &self.expected_fonts,
                &self.expected_images,
                true,
            )?;
            state.stage = ManifestAdmissionStage::ResourcesAdmitted;
        } else {
            if self.package_epoch.is_none() {
                return Err(BuildManifestError::PackageResourceMismatch);
            }
            if self
                .resource_progress
                .as_ref()
                .is_some_and(|progress| !admitted.continues_progress(progress))
            {
                return Err(BuildManifestError::ResourceProgressMismatch);
            }
            validate_expected_resources(
                &admitted.ledger().progress_token(),
                &self.expected_fonts,
                &self.expected_images,
                true,
            )?;
        }
        validate_admission_limits(&self.sources, &fonts, &images, &self.binding)?;
        self.fonts = fonts;
        self.images = images;
        self.resource_progress = Some(admitted.ledger().progress_token());
        self.resource_fingerprint = Some(admitted.fingerprint());
        Ok(())
    }

    pub fn admit_layout_selected(
        &mut self,
        pagination: &PaginationResult,
    ) -> Result<(), BuildManifestError> {
        let stage = self
            .machine
            .as_ref()
            .map(|state| state.stage)
            .ok_or(BuildManifestError::InputProfileMismatch)?;
        if stage != ManifestAdmissionStage::ResourcesAdmitted {
            return Err(BuildManifestError::MachineProgressRegression);
        }
        validate_ledger_pagination_closure(self, pagination)?;
        self.machine
            .as_mut()
            .expect("machine profile was checked above")
            .stage = ManifestAdmissionStage::LayoutSelected;
        Ok(())
    }

    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    pub fn machine_stage(&self) -> Option<ManifestAdmissionStage> {
        self.machine.as_ref().map(|state| state.stage)
    }

    pub const fn package_input(&self) -> Option<&PackageInputRecord> {
        self.package_input.as_ref()
    }

    fn manifest_records(&self) -> ManifestTerminalRecords {
        let mut inputs: Vec<_> = self.sources.values().cloned().collect();
        inputs.sort_by(|left, right| left.uri.cmp(&right.uri));
        ManifestTerminalRecords {
            package_input: self.package_input.clone(),
            inputs,
            fonts: self.fonts.values().cloned().collect(),
            images: self.images.values().cloned().collect(),
        }
    }
}

fn resource_progress_records(
    progress: &ResourceAdmissionProgressToken,
) -> Result<(ManifestFontRecords, ManifestImageRecords), BuildManifestError> {
    let mut fonts = BTreeMap::new();
    for font in progress.fonts() {
        if fonts
            .insert(
                font.font_face_id(),
                FontRecord {
                    font_face_id: font.font_face_id(),
                    uri: font.uri().clone(),
                    face_index: font.face_index(),
                    bytes: font.byte_length(),
                    sha256: font.content_hash(),
                    units_per_em: font.metadata().units_per_em,
                    glyph_count: font.metadata().glyph_count,
                },
            )
            .is_some()
        {
            return Err(BuildManifestError::DuplicateAdmissionRecord);
        }
    }
    let mut images = BTreeMap::new();
    for image in progress.images() {
        if images
            .insert(
                image.image_id(),
                ImageRecord {
                    image_id: image.image_id(),
                    uri: image.uri().clone(),
                    bytes: image.byte_length(),
                    sha256: image.content_hash(),
                    attested_media_kind: image.media_kind().as_str(),
                    pixel_width: image.width().get(),
                    pixel_height: image.height().get(),
                    decoded_bytes: image.decoded_bytes(),
                },
            )
            .is_some()
        {
            return Err(BuildManifestError::DuplicateAdmissionRecord);
        }
    }
    Ok((fonts, images))
}

fn validate_expected_resources(
    progress: &ResourceAdmissionProgressToken,
    expected_fonts: &[ExpectedFontResource],
    expected_images: &[ExpectedImageResource],
    complete: bool,
) -> Result<(), BuildManifestError> {
    if (complete
        && (progress.fonts().len() != expected_fonts.len()
            || progress.images().len() != expected_images.len()))
        || progress.fonts().len() > expected_fonts.len()
        || progress.images().len() > expected_images.len()
    {
        return Err(BuildManifestError::PackageResourceMismatch);
    }
    for font in progress.fonts() {
        let Some(expected) = expected_fonts.get(font.font_face_id().get() as usize) else {
            return Err(BuildManifestError::PackageResourceMismatch);
        };
        if expected.id != font.font_face_id()
            || expected.uri != *font.uri()
            || expected.family != font.family()
            || expected.face_index != font.face_index()
            || expected
                .expected_sha256
                .is_some_and(|hash| hash != font.content_hash())
        {
            return Err(BuildManifestError::PackageResourceMismatch);
        }
    }
    for image in progress.images() {
        let Some(expected) = expected_images.get(image.image_id().get() as usize) else {
            return Err(BuildManifestError::PackageResourceMismatch);
        };
        if expected.id != image.image_id()
            || expected.uri != *image.uri()
            || expected
                .expected_sha256
                .is_some_and(|hash| hash != image.content_hash())
        {
            return Err(BuildManifestError::PackageResourceMismatch);
        }
    }
    Ok(())
}

/// One-shot capability proving that all built-publication invariants were
/// checked before output I/O. Both terminal manifest shapes are sealed here:
/// the built record remains inaccessible until PDF commit, while its
/// output-null failed counterpart can only be released if that sink commit
/// fails. `BuildOutputCommitContext::commit_prepared_built` consumes the token
/// through the exact output session captured here.
#[derive(Debug)]
pub struct PreparedStandalonePdfPublication {
    output: BuildOutputCommitContext,
    pdf: VerifiedPdfBytesReceipt,
    facts: PreparedPdfOutputFacts,
    read_ledger: Option<HostReadIdentityLedgerToken>,
    staged_pdf: Option<PreparedAtomicFile>,
}

impl PreparedStandalonePdfPublication {
    pub fn commit(self) -> Result<PdfSinkCommitReceipt, PdfSinkCommitError> {
        let durability = match self.staged_pdf {
            Some(prepared) => {
                publish_prepared_file(&self.output.execution, prepared, self.read_ledger.as_ref())?
            }
            None => {
                commit_verified_pdf(&self.output.execution, &self.pdf, self.read_ledger.as_ref())?
            }
        };
        let receipt = self.output.issue_receipt(self.facts);
        match durability {
            SinkCommitDurability::Durable => Ok(receipt),
            SinkCommitDurability::PublishedButDurabilityUncertain(source) => {
                Err(PdfSinkCommitError::PublishedButDurabilityUncertain {
                    receipt: Box::new(receipt),
                    source,
                })
            }
        }
    }
}

#[derive(Debug)]
pub struct PreparedBuiltPublication {
    binding: PublicationBinding,
    manifest: ValidatedBuildManifest,
    manifest_bytes: Vec<u8>,
    failed_manifest: ValidatedBuildManifest,
    failed_manifest_bytes: Vec<u8>,
    pdf: VerifiedPdfBytesReceipt,
    output: PreparedPdfOutputFacts,
    read_ledger: Option<HostReadIdentityLedgerToken>,
}

impl PreparedBuiltPublication {
    /// Attach the final sealed read ledger before any terminal artifact is
    /// published. Machine built preflight does this automatically; this method
    /// lets reference/failure integration share the same alias guard.
    pub fn bind_read_ledger(
        mut self,
        read_ledger: HostReadIdentityLedgerToken,
    ) -> Result<Self, BuildManifestError> {
        if self.read_ledger.is_some() {
            return Err(BuildManifestError::ReadLedgerAlreadyBound);
        }
        self.read_ledger = Some(read_ledger);
        Ok(self)
    }
}

/// Built terminal plan whose complete file temporaries were written and
/// fsynced without making any requested artifact visible. Each target is
/// published atomically on its own; this is deliberately not a multi-file
/// transaction and never promises rollback of an earlier visible artifact.
#[derive(Debug)]
pub struct StagedBuiltPublication {
    output: BuildOutputCommitContext,
    prepared: PreparedBuiltPublication,
    staged_pdf: Option<PreparedAtomicFile>,
    staged_built_manifest: PreparedAtomicFile,
    staged_failed_manifest: PreparedAtomicFile,
}

impl StagedBuiltPublication {
    pub fn fail_before_pdf(self) -> PendingFailedManifestPublication {
        PendingFailedManifestPublication {
            output: self.output,
            prepared: PreparedFailedPublication {
                binding: self.prepared.binding,
                manifest: self.prepared.failed_manifest,
                manifest_bytes: self.prepared.failed_manifest_bytes,
                read_ledger: self.prepared.read_ledger,
            },
            staged_manifest: Some(self.staged_failed_manifest),
        }
    }

    pub fn commit_pdf(self) -> Result<PendingBuiltManifestPublication, PreparedPdfCommitError> {
        let pdf_durability = match self.staged_pdf {
            Some(staged) => publish_prepared_file(
                &self.output.execution,
                staged,
                self.prepared.read_ledger.as_ref(),
            ),
            None => commit_verified_pdf(
                &self.output.execution,
                &self.prepared.pdf,
                self.prepared.read_ledger.as_ref(),
            ),
        };
        let pdf_durability = match pdf_durability {
            Ok(durability) => durability,
            Err(source) => {
                return Err(PreparedPdfCommitError::SinkFailed {
                    source,
                    failed: Box::new(PendingFailedManifestPublication {
                        output: self.output,
                        prepared: PreparedFailedPublication {
                            binding: self.prepared.binding,
                            manifest: self.prepared.failed_manifest,
                            manifest_bytes: self.prepared.failed_manifest_bytes,
                            read_ledger: self.prepared.read_ledger,
                        },
                        staged_manifest: Some(self.staged_failed_manifest),
                    }),
                })
            }
        };
        let receipt = self.output.issue_receipt(self.prepared.output);
        if let SinkCommitDurability::PublishedButDurabilityUncertain(source) = pdf_durability {
            return Err(PreparedPdfCommitError::DurabilityUncertain {
                pdf_receipt: Box::new(receipt),
                source,
            });
        }
        Ok(PendingBuiltManifestPublication {
            output: self.output,
            binding: self.prepared.binding,
            manifest: self.prepared.manifest,
            manifest_bytes: self.prepared.manifest_bytes,
            read_ledger: self.prepared.read_ledger,
            pdf_receipt: receipt,
            staged_manifest: Some(self.staged_built_manifest),
        })
    }
}

#[derive(Debug)]
pub enum BuiltPublicationStagingError {
    Invalid(PdfSinkCommitError),
    Pdf(PdfSinkCommitError),
    BuiltManifest(PdfSinkCommitError),
    FailedManifest(PdfSinkCommitError),
}

/// One-shot capability for atomic publication of a terminal failed manifest.
/// Its record and canonical bytes are private until the bound output session
/// consumes it.
#[derive(Debug)]
pub struct PreparedFailedPublication {
    binding: PublicationBinding,
    manifest: ValidatedBuildManifest,
    manifest_bytes: Vec<u8>,
    read_ledger: Option<HostReadIdentityLedgerToken>,
}

/// PDF-visible success whose terminal built manifest is still private and
/// unpublished. Diagnostics publication is the only intended intervening
/// operation; dropping this capability is the fail-closed diagnostics-error
/// path.
#[derive(Debug)]
pub struct PendingBuiltManifestPublication {
    output: BuildOutputCommitContext,
    binding: PublicationBinding,
    manifest: ValidatedBuildManifest,
    manifest_bytes: Vec<u8>,
    read_ledger: Option<HostReadIdentityLedgerToken>,
    pdf_receipt: PdfSinkCommitReceipt,
    staged_manifest: Option<PreparedAtomicFile>,
}

impl PendingBuiltManifestPublication {
    pub const fn pdf_receipt(&self) -> &PdfSinkCommitReceipt {
        &self.pdf_receipt
    }

    pub fn commit_built_manifest(
        self,
    ) -> Result<CommittedBuiltPublication, BuiltPublicationCommitError> {
        self.output.commit_pending_built_manifest(
            self.binding,
            self.manifest,
            self.manifest_bytes,
            self.read_ledger,
            self.pdf_receipt,
            self.staged_manifest,
        )
    }
}

/// Output-null failed terminal record held while diagnostics publication is
/// attempted. The manifest can be attempted exactly once even when diagnostics
/// itself failed.
#[derive(Debug)]
pub struct PendingFailedManifestPublication {
    output: BuildOutputCommitContext,
    prepared: PreparedFailedPublication,
    staged_manifest: Option<PreparedAtomicFile>,
}

impl PendingFailedManifestPublication {
    pub fn commit_failed_manifest(
        self,
    ) -> Result<CommittedFailedPublication, ManifestSinkCommitError> {
        self.output
            .commit_prepared_failed_inner(self.prepared, self.staged_manifest)
    }
}

#[derive(Debug)]
pub enum PreparedPdfCommitError {
    /// No output bytes were attempted because the staged capabilities did not
    /// match the output owner.
    Invalid(PdfSinkCommitError),
    /// The PDF sink failed (including a rollback-impossible stdout prefix).
    /// Diagnostics must be attempted before consuming `failed`.
    SinkFailed {
        source: PdfSinkCommitError,
        failed: Box<PendingFailedManifestPublication>,
    },
    /// A file PDF is visible but its parent sync failed. No failed manifest is
    /// offered because the visible PDF must not be described as ungenerated.
    DurabilityUncertain {
        pdf_receipt: Box<PdfSinkCommitReceipt>,
        source: io::Error,
    },
}

impl PreparedFailedPublication {
    pub fn bind_read_ledger(
        mut self,
        read_ledger: HostReadIdentityLedgerToken,
    ) -> Result<Self, BuildManifestError> {
        if self.read_ledger.is_some() {
            return Err(BuildManifestError::ReadLedgerAlreadyBound);
        }
        self.read_ledger = Some(read_ledger);
        Ok(self)
    }
}

/// Trusted artifacts released together after the configured PDF sink commit
/// succeeds. No fallible manifest validation remains after that commit.
#[derive(Debug, Eq, PartialEq)]
pub struct CommittedBuiltPublication {
    manifest: ValidatedBuildManifest,
    receipt: PdfSinkCommitReceipt,
    manifest_receipt: ManifestSinkCommitReceipt,
}
impl CommittedBuiltPublication {
    pub const fn manifest(&self) -> &ValidatedBuildManifest {
        &self.manifest
    }
    pub const fn receipt(&self) -> &PdfSinkCommitReceipt {
        &self.receipt
    }
    pub const fn manifest_receipt(&self) -> &ManifestSinkCommitReceipt {
        &self.manifest_receipt
    }
    pub fn into_parts(
        self,
    ) -> (
        ValidatedBuildManifest,
        PdfSinkCommitReceipt,
        ManifestSinkCommitReceipt,
    ) {
        (self.manifest, self.receipt, self.manifest_receipt)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CommittedFailedPublication {
    manifest: ValidatedBuildManifest,
    receipt: ManifestSinkCommitReceipt,
}
impl CommittedFailedPublication {
    pub const fn manifest(&self) -> &ValidatedBuildManifest {
        &self.manifest
    }
    pub const fn receipt(&self) -> &ManifestSinkCommitReceipt {
        &self.receipt
    }
    pub fn into_parts(self) -> (ValidatedBuildManifest, ManifestSinkCommitReceipt) {
        (self.manifest, self.receipt)
    }
}

/// Proof that the canonical manifest bytes reached the configured manifest
/// file through the publication owner's atomic sidecar committer. The target
/// HostPath remains runtime-only and is not exposed as a manifest fact.
#[derive(Debug, Eq, PartialEq)]
pub struct ManifestSinkCommitReceipt {
    binding: PublicationBinding,
    bytes: u64,
}
impl ManifestSinkCommitReceipt {
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreparedPdfOutputFacts {
    selected_fingerprint: LayoutStateFingerprint,
    sink: OutputSink,
    bytes: u64,
    sha256: [u8; 32],
    page_count: u32,
    pdf_object_count: u32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PdfSinkCommitReceipt {
    binding: OutputCommitBinding,
    selected_fingerprint: LayoutStateFingerprint,
    sink: OutputSink,
    bytes: u64,
    sha256: [u8; 32],
    page_count: u32,
    pdf_object_count: u32,
}
impl PdfSinkCommitReceipt {
    pub const fn sink(&self) -> OutputSink {
        self.sink
    }
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn pdf_object_count(&self) -> u32 {
        self.pdf_object_count
    }
    pub const fn selected_fingerprint(&self) -> LayoutStateFingerprint {
        self.selected_fingerprint
    }
}

#[derive(Debug)]
pub enum PdfSinkCommitError {
    InvalidFacts(BuildManifestError),
    ManifestPreflightRequired,
    Execution(BuildExecutionError),
    Io(io::Error),
    /// Stdout accepted at least one byte but the complete verified receipt was
    /// not durably delivered. Unlike a file pre-publication failure, this
    /// prefix is externally visible and cannot be rolled back.
    StdoutPartial {
        bytes_written: u64,
        source: io::Error,
    },
    /// The atomic target update completed, but synchronizing the containing
    /// directory failed. The receipt proves visibility and prevents callers
    /// from treating this as a rollback or blindly retrying the publication.
    PublishedButDurabilityUncertain {
        receipt: Box<PdfSinkCommitReceipt>,
        source: io::Error,
    },
}

#[derive(Debug)]
pub enum BuiltPublicationCommitError {
    /// The PDF was not committed.
    Pdf(PdfSinkCommitError),
    /// The PDF sink rejected the bytes before issuing a receipt. A terminal
    /// output-null manifest publication was attempted from the failed record
    /// sealed during built preflight. `CommitError` can itself own a visible
    /// publication when only its directory synchronization failed.
    PdfSinkFailed {
        source: PdfSinkCommitError,
        failed_manifest: FailedManifestPublication,
    },
    /// The PDF target became visible, but its containing directory could not
    /// be synchronized. No manifest write was attempted.
    PdfDurability {
        pdf_receipt: Box<PdfSinkCommitReceipt>,
        source: io::Error,
    },
    /// The PDF commit succeeded, but a current target-alias check rejected the
    /// manifest write. The existing manifest, if any, was preserved.
    ManifestExecution {
        pdf_receipt: Box<PdfSinkCommitReceipt>,
        source: BuildExecutionError,
    },
    /// The PDF commit succeeded, but atomic manifest publication failed. The
    /// existing manifest, if any, was preserved.
    ManifestIo {
        pdf_receipt: Box<PdfSinkCommitReceipt>,
        source: io::Error,
    },
    /// A private publication invariant failed after PDF commit. This variant
    /// is fail-closed and still reports that the PDF reached its sink.
    ManifestInvariant {
        pdf_receipt: Box<PdfSinkCommitReceipt>,
    },
    /// Both target updates became visible, but the manifest directory sync
    /// failed. The complete publication is returned so callers cannot report
    /// either artifact as rolled back.
    ManifestDurability {
        publication: Box<CommittedBuiltPublication>,
        source: io::Error,
    },
}

/// Outcome of publishing the terminal failed manifest after the PDF sink
/// rejected a fully preflighted built publication.
#[derive(Debug)]
pub enum FailedManifestPublication {
    Committed(Box<CommittedFailedPublication>),
    CommitError(Box<ManifestSinkCommitError>),
}

#[derive(Debug)]
pub enum ManifestSinkCommitError {
    InvalidFacts(BuildManifestError),
    MissingManifestTarget,
    Execution(BuildExecutionError),
    Io(io::Error),
    /// The failed manifest became visible, but its directory sync failed.
    PublishedButDurabilityUncertain {
        publication: Box<CommittedFailedPublication>,
        source: io::Error,
    },
}

fn map_pdf_error_to_manifest_error(error: PdfSinkCommitError) -> ManifestSinkCommitError {
    match error {
        PdfSinkCommitError::Execution(source) => ManifestSinkCommitError::Execution(source),
        PdfSinkCommitError::Io(source) => ManifestSinkCommitError::Io(source),
        PdfSinkCommitError::InvalidFacts(error) => ManifestSinkCommitError::InvalidFacts(error),
        PdfSinkCommitError::ManifestPreflightRequired => {
            ManifestSinkCommitError::MissingManifestTarget
        }
        PdfSinkCommitError::StdoutPartial { .. }
        | PdfSinkCommitError::PublishedButDurabilityUncertain { .. } => {
            ManifestSinkCommitError::InvalidFacts(BuildManifestError::OutputReceiptBindingMismatch)
        }
    }
}

#[derive(Debug)]
enum SinkCommitDurability {
    Durable,
    PublishedButDurabilityUncertain(io::Error),
}

fn commit_verified_pdf(
    execution: &BuildExecutionContext,
    pdf: &VerifiedPdfBytesReceipt,
    read_ledger: Option<&HostReadIdentityLedgerToken>,
) -> Result<SinkCommitDurability, PdfSinkCommitError> {
    match execution.output_sink() {
        OutputSink::Stdout => {
            revalidate_publication_targets(execution, read_ledger)
                .map_err(PdfSinkCommitError::Execution)?;
            let stdout = io::stdout();
            let mut sink = stdout.lock();
            stream_verified_pdf(pdf, &mut sink)?;
            Ok(SinkCommitDurability::Durable)
        }
        OutputSink::File => commit_file_pdf_bytes_guarded(execution, pdf.bytes(), read_ledger),
    }
}

struct CountingWriter<W> {
    inner: W,
    bytes_written: u64,
}

impl<W> CountingWriter<W> {
    const fn new(inner: W) -> Self {
        Self {
            inner,
            bytes_written: 0,
        }
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(bytes)?;
        self.bytes_written = self
            .bytes_written
            .checked_add(u64::try_from(written).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "stdout byte count overflowed")
            })?)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "stdout byte count overflowed")
            })?;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn stream_verified_pdf<W: Write>(
    pdf: &VerifiedPdfBytesReceipt,
    sink: &mut W,
) -> Result<(), PdfSinkCommitError> {
    let mut counted = CountingWriter::new(sink);
    let streamed = match pdf.write_streaming(&mut counted) {
        Ok(streamed) => streamed,
        Err(source) if counted.bytes_written == 0 => return Err(PdfSinkCommitError::Io(source)),
        Err(source) => {
            return Err(PdfSinkCommitError::StdoutPartial {
                bytes_written: counted.bytes_written,
                source,
            })
        }
    };
    if let Err(error) = validate_streamed_pdf_facts(pdf, streamed) {
        return Err(PdfSinkCommitError::StdoutPartial {
            bytes_written: counted.bytes_written,
            source: io::Error::new(
                io::ErrorKind::InvalidData,
                format!("streamed PDF facts failed validation: {error:?}"),
            ),
        });
    }
    counted
        .flush()
        .map_err(|source| PdfSinkCommitError::StdoutPartial {
            bytes_written: counted.bytes_written,
            source,
        })
}

fn revalidate_publication_targets(
    execution: &BuildExecutionContext,
    read_ledger: Option<&HostReadIdentityLedgerToken>,
) -> Result<(), BuildExecutionError> {
    execution.revalidate_write_targets()?;
    let Some(read_ledger) = read_ledger else {
        return Ok(());
    };
    for target in [
        execution.output_path(),
        execution.trace_target(),
        execution.manifest_target(),
        execution.diagnostics_target(),
    ]
    .into_iter()
    .flatten()
    {
        match read_ledger.revalidate_write_target(target) {
            Ok(false) => {}
            Ok(true) => return Err(BuildExecutionError::AliasedReadWriteTarget),
            Err(error) => return Err(map_read_ledger_error(error)),
        }
    }
    Ok(())
}

const fn map_read_ledger_error(_error: HostAdmissionError) -> BuildExecutionError {
    BuildExecutionError::ReadTargetChanged
}

fn validate_streamed_pdf_facts(
    pdf: &VerifiedPdfBytesReceipt,
    streamed: PdfStreamWriteFacts,
) -> Result<(), BuildManifestError> {
    if streamed.byte_length() != pdf.byte_length() || streamed.content_hash() != pdf.content_hash()
    {
        return Err(BuildManifestError::OutputReceiptBindingMismatch);
    }
    if streamed.selected_layout_fingerprint() != pdf.selected_layout_fingerprint()
        || streamed.page_count() != pdf.page_count()
        || streamed.object_count() != pdf.object_count()
    {
        return Err(BuildManifestError::PdfGraphReceiptMismatch);
    }
    if streamed.stream_compression() != pdf.stream_compression() {
        return Err(BuildManifestError::StreamCompressionMismatch);
    }
    if streamed.config_fingerprint() != pdf.config_fingerprint() {
        return Err(BuildManifestError::ConfigFingerprintMismatch);
    }
    Ok(())
}

#[cfg(test)]
fn commit_file_pdf_bytes(
    execution: &BuildExecutionContext,
    bytes: &[u8],
) -> Result<SinkCommitDurability, PdfSinkCommitError> {
    commit_file_pdf_bytes_guarded(execution, bytes, None)
}

fn commit_file_pdf_bytes_guarded(
    execution: &BuildExecutionContext,
    bytes: &[u8],
    read_ledger: Option<&HostReadIdentityLedgerToken>,
) -> Result<SinkCommitDurability, PdfSinkCommitError> {
    let target = execution.output_path().ok_or_else(|| {
        PdfSinkCommitError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "file output has no target",
        ))
    })?;
    commit_file_bytes(execution, target.as_path(), bytes, read_ledger)
}

fn commit_file_bytes(
    execution: &BuildExecutionContext,
    target: &Path,
    bytes: &[u8],
    read_ledger: Option<&HostReadIdentityLedgerToken>,
) -> Result<SinkCommitDurability, PdfSinkCommitError> {
    let prepared = prepare_file_atomically(
        execution,
        target,
        bytes,
        execution.replace_policy(),
        read_ledger,
    )?;
    publish_prepared_file(execution, prepared, read_ledger)
}

#[derive(Debug)]
struct PreparedAtomicFile {
    target: PathBuf,
    parent: PathBuf,
    temporary: Option<PathBuf>,
    replace_policy: ReplacePolicy,
}

impl Drop for PreparedAtomicFile {
    fn drop(&mut self) {
        if let Some(temporary) = self.temporary.take() {
            let _ = fs::remove_file(temporary);
        }
    }
}

#[cfg(unix)]
fn prepare_file_atomically(
    execution: &BuildExecutionContext,
    target: &Path,
    bytes: &[u8],
    replace_policy: ReplacePolicy,
    read_ledger: Option<&HostReadIdentityLedgerToken>,
) -> Result<PreparedAtomicFile, PdfSinkCommitError> {
    revalidate_publication_targets(execution, read_ledger)
        .map_err(PdfSinkCommitError::Execution)?;
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let leaf = target.file_name().ok_or_else(|| {
        PdfSinkCommitError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output target has no file name",
        ))
    })?;
    let (temporary, mut file) =
        create_output_temporary(parent, leaf).map_err(PdfSinkCommitError::Io)?;
    if let Err(error) = (|| {
        file.write_all(bytes).map_err(PdfSinkCommitError::Io)?;
        file.sync_all().map_err(PdfSinkCommitError::Io)?;
        Ok(())
    })() {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(PreparedAtomicFile {
        target: target.to_path_buf(),
        parent: parent.to_path_buf(),
        temporary: Some(temporary),
        replace_policy,
    })
}

#[cfg(unix)]
fn publish_prepared_file(
    execution: &BuildExecutionContext,
    mut prepared: PreparedAtomicFile,
    read_ledger: Option<&HostReadIdentityLedgerToken>,
) -> Result<SinkCommitDurability, PdfSinkCommitError> {
    revalidate_publication_targets(execution, read_ledger)
        .map_err(PdfSinkCommitError::Execution)?;
    let temporary = prepared.temporary.as_ref().ok_or_else(|| {
        PdfSinkCommitError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "prepared output temporary is missing",
        ))
    })?;
    match prepared.replace_policy {
        ReplacePolicy::NoReplace => {
            fs::hard_link(temporary, &prepared.target).map_err(PdfSinkCommitError::Io)?;
        }
        ReplacePolicy::Replace => {
            fs::rename(temporary, &prepared.target).map_err(PdfSinkCommitError::Io)?;
        }
    }
    if prepared.replace_policy == ReplacePolicy::NoReplace {
        // The target link is already the published artifact. Temporary-name
        // cleanup is best effort and must not turn a successful publication
        // into a false rollback report.
        if let Some(temporary) = prepared.temporary.take() {
            let _ = fs::remove_file(temporary);
        }
    } else {
        prepared.temporary = None;
    }
    Ok(classify_parent_sync(sync_parent_directory(
        &prepared.parent,
    )))
}

fn classify_parent_sync(result: io::Result<()>) -> SinkCommitDurability {
    match result {
        Ok(()) => SinkCommitDurability::Durable,
        Err(source) => SinkCommitDurability::PublishedButDurabilityUncertain(source),
    }
}

#[cfg(unix)]
fn create_output_temporary(parent: &Path, leaf: &std::ffi::OsStr) -> io::Result<(PathBuf, File)> {
    for _ in 0..128 {
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = OsString::from(".");
        temporary_name.push(leaf);
        temporary_name.push(format!(".typaxis-{}-{ordinal}.tmp", std::process::id()));
        let temporary = parent.join(temporary_name);
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
        {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique output temporary",
    ))
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> io::Result<()> {
    File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn prepare_file_atomically(
    _execution: &BuildExecutionContext,
    _target: &Path,
    _bytes: &[u8],
    _replace_policy: ReplacePolicy,
    _read_ledger: Option<&HostReadIdentityLedgerToken>,
) -> Result<PreparedAtomicFile, PdfSinkCommitError> {
    Err(PdfSinkCommitError::Io(io::Error::new(
        io::ErrorKind::Unsupported,
        "no atomic file-output committer is registered for this platform",
    )))
}

#[cfg(not(unix))]
fn publish_prepared_file(
    _execution: &BuildExecutionContext,
    _prepared: PreparedAtomicFile,
    _read_ledger: Option<&HostReadIdentityLedgerToken>,
) -> Result<SinkCommitDurability, PdfSinkCommitError> {
    Err(PdfSinkCommitError::Io(io::Error::new(
        io::ErrorKind::Unsupported,
        "no atomic file-output publisher is registered for this platform",
    )))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ManifestExpectations<'a> {
    publication: &'a ManifestPublicationContext,
}
impl<'a> ManifestExpectations<'a> {
    const fn from_publication(publication: &'a ManifestPublicationContext) -> Self {
        Self { publication }
    }
}

#[derive(Debug, Default)]
struct ManifestProfileFacts {
    table_layouts: Vec<StagingTableLayoutFacts>,
    footnote_layout: Option<StagingFootnoteLayoutFacts>,
    advanced_pagination: Option<String>,
}

impl BuildManifest {
    fn terminal(
        publication: &ManifestPublicationContext,
        status: BuildStatus,
        records: ManifestTerminalRecords,
        layout: Option<LayoutRecord>,
        output: Option<OutputRecord>,
        profile_facts: ManifestProfileFacts,
    ) -> Self {
        let ManifestProfileFacts {
            table_layouts,
            footnote_layout,
            advanced_pagination,
        } = profile_facts;
        Self {
            advanced_pagination,
            contract: CONTRACT.to_owned(),
            status,
            deterministic: true,
            engine: publication.engine.clone(),
            data_versions: publication.data_versions.clone(),
            config_sha256: publication.config_fingerprint.bytes(),
            input_profile: publication.input_profile,
            package_input: records.package_input,
            inputs: records.inputs,
            fonts: records.fonts,
            images: records.images,
            pdf_profile: PDF_PROFILE.to_owned(),
            stream_compression: publication.stream_compression,
            layout,
            output,
            table_layouts,
            footnote_layout,
        }
    }

    pub fn contract(&self) -> &str {
        &self.contract
    }
    pub fn advanced_pagination(&self) -> Option<&str> {
        self.advanced_pagination.as_deref()
    }
    pub const fn status(&self) -> BuildStatus {
        self.status
    }
    pub const fn deterministic(&self) -> bool {
        self.deterministic
    }
    pub const fn engine(&self) -> &EngineRecord {
        &self.engine
    }
    pub const fn data_versions(&self) -> &DataVersions {
        &self.data_versions
    }
    pub const fn config_sha256(&self) -> [u8; 32] {
        self.config_sha256
    }
    pub const fn input_profile(&self) -> BuildInputProfile {
        self.input_profile
    }
    pub const fn package_input(&self) -> Option<&PackageInputRecord> {
        self.package_input.as_ref()
    }
    pub fn inputs(&self) -> &[FileRecord] {
        &self.inputs
    }
    pub fn fonts(&self) -> &[FontRecord] {
        &self.fonts
    }
    pub fn images(&self) -> &[ImageRecord] {
        &self.images
    }
    pub fn pdf_profile(&self) -> &str {
        &self.pdf_profile
    }
    pub const fn stream_compression(&self) -> PdfStreamCompression {
        self.stream_compression
    }
    pub const fn layout(&self) -> Option<LayoutRecord> {
        self.layout
    }
    pub const fn output(&self) -> Option<&OutputRecord> {
        self.output.as_ref()
    }
    pub fn table_layouts(&self) -> &[StagingTableLayoutFacts] {
        &self.table_layouts
    }
    pub const fn footnote_layout(&self) -> Option<&StagingFootnoteLayoutFacts> {
        self.footnote_layout.as_ref()
    }

    fn to_canonical_json_bytes(&self) -> Vec<u8> {
        canonical_manifest_json(self).into_bytes()
    }

    fn validate(&self, expectations: ManifestExpectations<'_>) -> Result<(), BuildManifestError> {
        if self.contract != CONTRACT {
            return Err(BuildManifestError::WrongContract);
        }
        if !self.deterministic {
            return Err(BuildManifestError::NonDeterministic);
        }
        if self.engine.name != ENGINE_NAME {
            return Err(BuildManifestError::WrongEngineName);
        }
        if self.engine.version.is_empty() || self.engine.rust_version.is_empty() {
            return Err(BuildManifestError::EmptyVersion);
        }
        if self.engine != expectations.publication.engine {
            return Err(BuildManifestError::WrongEngineName);
        }
        if self.pdf_profile != PDF_PROFILE {
            return Err(BuildManifestError::WrongPdfProfile);
        }
        if self.stream_compression != expectations.publication.stream_compression {
            return Err(BuildManifestError::StreamCompressionMismatch);
        }
        if self.data_versions != expectations.publication.data_versions {
            return Err(BuildManifestError::DataVersionMismatch);
        }
        if self.config_sha256 != expectations.publication.config_fingerprint.bytes() {
            return Err(BuildManifestError::ConfigFingerprintMismatch);
        }
        if self.input_profile != expectations.publication.input_profile {
            return Err(BuildManifestError::InputProfileMismatch);
        }
        let advanced_profile = matches!(
            self.input_profile,
            BuildInputProfile::MachinePdfColumns1
                | BuildInputProfile::MachinePdfFloat1
                | BuildInputProfile::MachinePdfHeaderFooter1
        );
        if (self.status == BuildStatus::Built && advanced_profile)
            != self.advanced_pagination.is_some()
        {
            return Err(BuildManifestError::AdvancedPaginationMismatch);
        }
        match (self.input_profile, self.status, self.package_input.as_ref()) {
            (BuildInputProfile::ReferenceSource1, _, None) => {}
            (BuildInputProfile::ReferenceSource1, _, Some(_)) => {
                return Err(BuildManifestError::MachinePackageMismatch)
            }
            (
                BuildInputProfile::MachinePdfBasicDocument1
                | BuildInputProfile::MachinePdfColumns1
                | BuildInputProfile::MachinePdfFloat1
                | BuildInputProfile::MachinePdfFootnote1
                | BuildInputProfile::MachinePdfHeaderFooter1
                | BuildInputProfile::MachinePdfParagraph1
                | BuildInputProfile::MachinePdfTable1,
                BuildStatus::Built,
                Some(package),
            ) if package.is_decoded()
                && package.profile_receipt_sha256.is_some()
                && self.inputs.len() == 1 => {}
            (
                BuildInputProfile::MachinePdfBasicDocument1
                | BuildInputProfile::MachinePdfColumns1
                | BuildInputProfile::MachinePdfFloat1
                | BuildInputProfile::MachinePdfFootnote1
                | BuildInputProfile::MachinePdfHeaderFooter1
                | BuildInputProfile::MachinePdfParagraph1
                | BuildInputProfile::MachinePdfTable1,
                BuildStatus::Built,
                _,
            ) => return Err(BuildManifestError::MachinePackageMismatch),
            (
                BuildInputProfile::MachinePdfBasicDocument1
                | BuildInputProfile::MachinePdfColumns1
                | BuildInputProfile::MachinePdfFloat1
                | BuildInputProfile::MachinePdfFootnote1
                | BuildInputProfile::MachinePdfHeaderFooter1
                | BuildInputProfile::MachinePdfParagraph1
                | BuildInputProfile::MachinePdfTable1,
                BuildStatus::Failed,
                Some(package),
            ) if package.contract.is_some() == package.canonical_sha256.is_some() => {}
            (
                BuildInputProfile::MachinePdfBasicDocument1
                | BuildInputProfile::MachinePdfColumns1
                | BuildInputProfile::MachinePdfFloat1
                | BuildInputProfile::MachinePdfFootnote1
                | BuildInputProfile::MachinePdfHeaderFooter1
                | BuildInputProfile::MachinePdfParagraph1
                | BuildInputProfile::MachinePdfTable1,
                BuildStatus::Failed,
                None,
            ) => {}
            (
                BuildInputProfile::MachinePdfBasicDocument1
                | BuildInputProfile::MachinePdfColumns1
                | BuildInputProfile::MachinePdfFloat1
                | BuildInputProfile::MachinePdfFootnote1
                | BuildInputProfile::MachinePdfHeaderFooter1
                | BuildInputProfile::MachinePdfParagraph1
                | BuildInputProfile::MachinePdfTable1,
                BuildStatus::Failed,
                Some(_),
            ) => return Err(BuildManifestError::MachinePackageMismatch),
        }
        if self.status == BuildStatus::Built {
            match self.input_profile {
                BuildInputProfile::ReferenceSource1 => {
                    if self.layout.is_some_and(|layout| {
                        layout.profile_receipt_sha256.is_some()
                            || layout.flow_registry_sha256.is_some()
                    }) {
                        return Err(BuildManifestError::MachineCapabilityMismatch);
                    }
                }
                BuildInputProfile::MachinePdfParagraph1 => {
                    let layout = self
                        .layout
                        .ok_or(BuildManifestError::MachineCapabilityMismatch)?;
                    if layout.profile_receipt_sha256
                        != self
                            .package_input
                            .as_ref()
                            .and_then(|package| package.profile_receipt_sha256)
                        || layout.flow_registry_sha256.is_some()
                    {
                        return Err(BuildManifestError::MachineCapabilityMismatch);
                    }
                }
                BuildInputProfile::MachinePdfBasicDocument1
                | BuildInputProfile::MachinePdfColumns1
                | BuildInputProfile::MachinePdfFloat1
                | BuildInputProfile::MachinePdfFootnote1
                | BuildInputProfile::MachinePdfHeaderFooter1
                | BuildInputProfile::MachinePdfTable1 => {
                    let layout = self
                        .layout
                        .ok_or(BuildManifestError::MachineCapabilityMismatch)?;
                    if layout.profile_receipt_sha256
                        != self
                            .package_input
                            .as_ref()
                            .and_then(|package| package.profile_receipt_sha256)
                        || layout.flow_registry_sha256.is_none()
                    {
                        return Err(BuildManifestError::MachineCapabilityMismatch);
                    }
                }
            }
        }
        if self
            .inputs
            .windows(2)
            .any(|pair| pair[0].uri >= pair[1].uri)
        {
            return Err(BuildManifestError::NonCanonicalInputs);
        }
        let include_count = match self.inputs.len().checked_sub(1) {
            Some(count) => count,
            None if self.status == BuildStatus::Failed => 0,
            None => return Err(BuildManifestError::MissingEntryInput),
        };
        if self
            .fonts
            .windows(2)
            .any(|pair| pair[0].font_face_id >= pair[1].font_face_id)
        {
            return Err(BuildManifestError::NonCanonicalFonts);
        }
        if self
            .images
            .windows(2)
            .any(|pair| pair[0].image_id >= pair[1].image_id)
        {
            return Err(BuildManifestError::NonCanonicalImages);
        }
        let largest_bytes = self
            .inputs
            .iter()
            .map(|record| record.bytes)
            .chain(self.fonts.iter().map(|record| record.bytes))
            .chain(self.images.iter().map(|record| record.bytes))
            .chain(self.package_input.iter().map(|record| record.bytes))
            .chain(self.output.iter().map(|record| record.bytes))
            .max()
            .unwrap_or(0);
        if largest_bytes > JSON_SAFE_INTEGER_MAX as u64 {
            return Err(BuildManifestError::IntegerNotJsonSafe);
        }
        if self.fonts.iter().any(|record| record.bytes == 0)
            || self.images.iter().any(|record| record.bytes == 0)
        {
            return Err(BuildManifestError::EmptyAdmittedResource);
        }
        if self
            .fonts
            .iter()
            .any(|record| record.units_per_em == 0 || record.glyph_count == 0)
        {
            return Err(BuildManifestError::InvalidFontMetadata);
        }
        let limits = expectations.publication.limits.get();
        if self
            .package_input
            .as_ref()
            .is_some_and(|record| record.bytes > limits.max_document_package_bytes)
        {
            return Err(BuildManifestError::PackageInputBytesLimit);
        }
        if self
            .layout
            .is_some_and(|layout| layout.pass_count.get() > limits.max_layout_passes)
        {
            return Err(BuildManifestError::LayoutLimit);
        }
        if include_count > limits.max_include_files as usize {
            return Err(BuildManifestError::IncludeFileLimit);
        }
        if self
            .inputs
            .iter()
            .any(|record| record.bytes > u64::from(limits.max_source_bytes))
        {
            return Err(BuildManifestError::InputSourceLimit);
        }
        if checked_byte_sum(self.inputs.iter().map(|record| record.bytes))
            .map_or(true, |bytes| bytes > limits.max_input_bytes)
        {
            return Err(BuildManifestError::InputAggregateLimit);
        }
        if self.fonts.len() > limits.max_fonts as usize {
            return Err(BuildManifestError::FontCountLimit);
        }
        if self
            .fonts
            .iter()
            .any(|record| record.bytes > limits.max_font_bytes)
        {
            return Err(BuildManifestError::FontBytesLimit);
        }
        if self.images.len() > limits.max_images as usize {
            return Err(BuildManifestError::ImageCountLimit);
        }
        if self
            .images
            .iter()
            .any(|record| record.bytes > limits.max_image_bytes)
        {
            return Err(BuildManifestError::ImageBytesLimit);
        }
        for image in &self.images {
            let pixels = u64::from(image.pixel_width)
                .checked_mul(u64::from(image.pixel_height))
                .ok_or(BuildManifestError::ImagePixelLimit)?;
            if image.pixel_width == 0 || image.pixel_height == 0 || pixels > limits.max_image_pixels
            {
                return Err(BuildManifestError::ImagePixelLimit);
            }
            if image.decoded_bytes == 0 || image.decoded_bytes > limits.max_decoded_image_bytes {
                return Err(BuildManifestError::ImageDecodedBytesLimit);
            }
        }
        if checked_byte_sum(
            self.fonts
                .iter()
                .map(|record| record.bytes)
                .chain(self.images.iter().map(|record| record.bytes)),
        )
        .map_or(true, |bytes| bytes > limits.max_resource_bytes)
        {
            return Err(BuildManifestError::ResourceAggregateLimit);
        }
        match self.status {
            BuildStatus::Built if self.layout.is_none() || self.output.is_none() => {
                return Err(BuildManifestError::BuiltRequiresLayoutAndOutput)
            }
            BuildStatus::Built
                if self.output.as_ref().is_some_and(|output| {
                    output.bytes == 0 || output.page_count == 0 || output.pdf_object_count == 0
                }) =>
            {
                return Err(BuildManifestError::EmptyBuiltOutput)
            }
            BuildStatus::Failed if self.output.is_some() => {
                return Err(BuildManifestError::NonBuiltMustNotHaveOutput)
            }
            _ => {}
        }
        if let Some(output) = &self.output {
            if output.sink != expectations.publication.output_sink() {
                return Err(BuildManifestError::OutputSinkMismatch);
            }
            if output.page_count > limits.max_pages {
                return Err(BuildManifestError::PageLimit);
            }
            if output.bytes > limits.max_output_bytes {
                return Err(BuildManifestError::OutputBytesLimit);
            }
            if output.pdf_object_count > limits.max_pdf_objects {
                return Err(BuildManifestError::PdfObjectLimit);
            }
        }
        match (self.input_profile, self.status) {
            (BuildInputProfile::MachinePdfFootnote1, BuildStatus::Built) => {
                let footnote = self
                    .footnote_layout
                    .as_ref()
                    .ok_or(BuildManifestError::MachineCapabilityMismatch)?;
                let output_pages = self
                    .output
                    .as_ref()
                    .map(|output| output.page_count as usize)
                    .ok_or(BuildManifestError::BuiltRequiresLayoutAndOutput)?;
                if !self.table_layouts.is_empty()
                    || footnote.pages().len() != output_pages
                    || footnote
                        .pages()
                        .iter()
                        .enumerate()
                        .any(|(index, page)| page.page_index() as usize != index)
                {
                    return Err(BuildManifestError::MachineCapabilityMismatch);
                }
            }
            (BuildInputProfile::MachinePdfTable1, BuildStatus::Built) => {
                let output_pages = self
                    .output
                    .as_ref()
                    .map(|output| output.page_count)
                    .ok_or(BuildManifestError::BuiltRequiresLayoutAndOutput)?;
                if self
                    .table_layouts
                    .windows(2)
                    .any(|pair| pair[0].table_node_id() >= pair[1].table_node_id())
                    || self.table_layouts.iter().any(|table| {
                        table
                            .target_page_end()
                            .map_or(true, |end| end > output_pages)
                    })
                {
                    return Err(BuildManifestError::MachineCapabilityMismatch);
                }
            }
            _ if !self.table_layouts.is_empty() || self.footnote_layout.is_some() => {
                return Err(BuildManifestError::MachineCapabilityMismatch)
            }
            _ => {}
        }
        Ok(())
    }
}

fn canonical_manifest_json(manifest: &BuildManifest) -> String {
    let mut json = String::new();
    json.push('{');
    let mut first = true;
    if let Some(advanced) = manifest.advanced_pagination.as_deref() {
        push_json_member_name(&mut json, "advanced_pagination", first);
        json.push_str(advanced);
        first = false;
    }
    push_json_member_name(&mut json, "config_sha256", first);
    push_json_hex(&mut json, &manifest.config_sha256);
    push_json_member_name(&mut json, "contract", false);
    push_json_string(&mut json, &manifest.contract);
    push_json_member_name(&mut json, "data_versions", false);
    push_data_versions_json(&mut json, &manifest.data_versions);
    push_json_member_name(&mut json, "deterministic", false);
    json.push_str("true");
    push_json_member_name(&mut json, "engine", false);
    push_engine_json(&mut json, &manifest.engine);
    push_json_member_name(&mut json, "fonts", false);
    push_fonts_json(&mut json, &manifest.fonts);
    if manifest.input_profile == BuildInputProfile::MachinePdfFootnote1
        && manifest.status == BuildStatus::Built
    {
        push_json_member_name(&mut json, "footnote_layout", false);
        json.push_str(
            manifest
                .footnote_layout
                .as_ref()
                .expect("validated footnote manifest requires layout facts")
                .canonical_jcs(),
        );
    }
    push_json_member_name(&mut json, "images", false);
    push_images_json(&mut json, &manifest.images);
    push_json_member_name(&mut json, "input_profile", false);
    push_json_string(&mut json, manifest.input_profile.as_str());
    push_json_member_name(&mut json, "inputs", false);
    push_inputs_json(&mut json, &manifest.inputs);
    push_json_member_name(&mut json, "layout", false);
    match manifest.layout {
        Some(layout) => push_layout_json(&mut json, layout),
        None => json.push_str("null"),
    }
    push_json_member_name(&mut json, "output", false);
    match &manifest.output {
        Some(output) => push_output_json(&mut json, output),
        None => json.push_str("null"),
    }
    push_json_member_name(&mut json, "package_input", false);
    match &manifest.package_input {
        Some(package) => push_package_input_json(&mut json, package),
        None => json.push_str("null"),
    }
    push_json_member_name(&mut json, "pdf_profile", false);
    push_json_string(&mut json, &manifest.pdf_profile);
    push_json_member_name(&mut json, "status", false);
    push_json_string(
        &mut json,
        match manifest.status {
            BuildStatus::Built => "built",
            BuildStatus::Failed => "failed",
        },
    );
    push_json_member_name(&mut json, "stream_compression", false);
    push_json_string(
        &mut json,
        match manifest.stream_compression {
            PdfStreamCompression::Flate => "flate",
            PdfStreamCompression::None => "none",
        },
    );
    if manifest.input_profile == BuildInputProfile::MachinePdfTable1
        && manifest.status == BuildStatus::Built
    {
        push_json_member_name(&mut json, "table_layouts", false);
        json.push('[');
        for (index, table) in manifest.table_layouts.iter().enumerate() {
            if index != 0 {
                json.push(',');
            }
            json.push_str(table.canonical_jcs());
        }
        json.push(']');
    }
    json.push('}');
    json
}

fn push_package_input_json(json: &mut String, record: &PackageInputRecord) {
    json.push('{');
    push_json_member_name(json, "bytes", true);
    json.push_str(&record.bytes.to_string());
    push_json_member_name(json, "canonical_sha256", false);
    match record.canonical_sha256 {
        Some(hash) => push_json_hex(json, &hash),
        None => json.push_str("null"),
    }
    push_json_member_name(json, "contract", false);
    match record.contract {
        Some(contract) => push_json_string(json, contract.as_str()),
        None => json.push_str("null"),
    }
    push_json_member_name(json, "profile_receipt_sha256", false);
    match record.profile_receipt_sha256 {
        Some(hash) => push_json_hex(json, &hash),
        None => json.push_str("null"),
    }
    push_json_member_name(json, "sha256", false);
    push_json_hex(json, &record.sha256);
    push_json_member_name(json, "uri", false);
    push_json_string(json, record.uri.as_str());
    json.push('}');
}

fn push_engine_json(json: &mut String, engine: &EngineRecord) {
    json.push('{');
    push_json_member_name(json, "git_commit", true);
    match &engine.git_commit {
        Some(commit) => push_json_string(json, commit),
        None => json.push_str("null"),
    }
    push_json_member_name(json, "name", false);
    push_json_string(json, &engine.name);
    push_json_member_name(json, "rust_version", false);
    push_json_string(json, &engine.rust_version);
    push_json_member_name(json, "version", false);
    push_json_string(json, &engine.version);
    json.push('}');
}

fn push_data_versions_json(json: &mut String, versions: &DataVersions) {
    json.push('{');
    push_json_member_name(json, "japanese_line_break", true);
    push_json_string(json, &versions.japanese_line_break);
    push_json_member_name(json, "shaper_backend", false);
    push_json_string(json, &versions.shaper_backend);
    push_json_member_name(json, "shaper_version", false);
    push_json_string(json, &versions.shaper_version);
    push_json_member_name(json, "unicode", false);
    push_json_string(json, &versions.unicode);
    json.push('}');
}

fn push_inputs_json(json: &mut String, records: &[FileRecord]) {
    json.push('[');
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push('{');
        push_json_member_name(json, "bytes", true);
        json.push_str(&record.bytes.to_string());
        push_json_member_name(json, "sha256", false);
        push_json_hex(json, &record.sha256);
        push_json_member_name(json, "uri", false);
        push_json_string(json, record.uri.as_str());
        json.push('}');
    }
    json.push(']');
}

fn push_fonts_json(json: &mut String, records: &[FontRecord]) {
    json.push('[');
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push('{');
        push_json_member_name(json, "bytes", true);
        json.push_str(&record.bytes.to_string());
        push_json_member_name(json, "face_index", false);
        json.push_str(&record.face_index.to_string());
        push_json_member_name(json, "font_face_id", false);
        json.push_str(&record.font_face_id.get().to_string());
        push_json_member_name(json, "glyph_count", false);
        json.push_str(&record.glyph_count.to_string());
        push_json_member_name(json, "sha256", false);
        push_json_hex(json, &record.sha256);
        push_json_member_name(json, "units_per_em", false);
        json.push_str(&record.units_per_em.to_string());
        push_json_member_name(json, "uri", false);
        push_json_string(json, record.uri.as_str());
        json.push('}');
    }
    json.push(']');
}

fn push_images_json(json: &mut String, records: &[ImageRecord]) {
    json.push('[');
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push('{');
        push_json_member_name(json, "attested_media_kind", true);
        push_json_string(json, record.attested_media_kind);
        push_json_member_name(json, "bytes", false);
        json.push_str(&record.bytes.to_string());
        push_json_member_name(json, "decoded_bytes", false);
        json.push_str(&record.decoded_bytes.to_string());
        push_json_member_name(json, "image_id", false);
        json.push_str(&record.image_id.get().to_string());
        push_json_member_name(json, "pixel_height", false);
        json.push_str(&record.pixel_height.to_string());
        push_json_member_name(json, "pixel_width", false);
        json.push_str(&record.pixel_width.to_string());
        push_json_member_name(json, "sha256", false);
        push_json_hex(json, &record.sha256);
        push_json_member_name(json, "uri", false);
        push_json_string(json, record.uri.as_str());
        json.push('}');
    }
    json.push(']');
}

fn push_layout_json(json: &mut String, layout: LayoutRecord) {
    json.push('{');
    push_json_member_name(json, "fallback_policy", true);
    match layout.fallback_policy {
        Some(LayoutFallbackPolicy::LowestCostThenEarliest) => {
            push_json_string(json, "lowest_cost_then_earliest")
        }
        None => json.push_str("null"),
    }
    push_json_member_name(json, "final_fingerprint", false);
    push_json_hex(json, &layout.final_fingerprint.bytes());
    push_json_member_name(json, "flow_registry_sha256", false);
    match layout.flow_registry_sha256 {
        Some(hash) => push_json_hex(json, &hash),
        None => json.push_str("null"),
    }
    push_json_member_name(json, "pass_count", false);
    json.push_str(&layout.pass_count.get().to_string());
    push_json_member_name(json, "profile_receipt_sha256", false);
    match layout.profile_receipt_sha256 {
        Some(hash) => push_json_hex(json, &hash),
        None => json.push_str("null"),
    }
    push_json_member_name(json, "selected_state", false);
    json.push_str(&layout.selected_state.get().to_string());
    push_json_member_name(json, "status", false);
    push_json_string(
        json,
        match layout.status {
            LayoutStatus::Converged => "converged",
            LayoutStatus::CycleFallback => "cycle_fallback",
            LayoutStatus::MaxPassFallback => "max_pass_fallback",
        },
    );
    json.push('}');
}

fn push_output_json(json: &mut String, output: &OutputRecord) {
    json.push('{');
    push_json_member_name(json, "bytes", true);
    json.push_str(&output.bytes.to_string());
    push_json_member_name(json, "page_count", false);
    json.push_str(&output.page_count.to_string());
    push_json_member_name(json, "pdf_object_count", false);
    json.push_str(&output.pdf_object_count.to_string());
    push_json_member_name(json, "sha256", false);
    push_json_hex(json, &output.sha256);
    push_json_member_name(json, "sink", false);
    push_json_string(
        json,
        match output.sink {
            OutputSink::File => "file",
            OutputSink::Stdout => "stdout",
        },
    );
    json.push('}');
}

fn push_json_member_name(json: &mut String, name: &str, first: bool) {
    if !first {
        json.push(',');
    }
    push_json_string(json, name);
    json.push(':');
}

fn push_json_hex(json: &mut String, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    json.push('"');
    for byte in bytes {
        json.push(char::from(HEX[usize::from(byte >> 4)]));
        json.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    json.push('"');
}

fn push_json_string(json: &mut String, value: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    json.push('"');
    for character in value.chars() {
        match character {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\u{08}' => json.push_str("\\b"),
            '\u{09}' => json.push_str("\\t"),
            '\u{0a}' => json.push_str("\\n"),
            '\u{0c}' => json.push_str("\\f"),
            '\u{0d}' => json.push_str("\\r"),
            character if character <= '\u{1f}' => {
                let byte = character as u8;
                json.push_str("\\u00");
                json.push(char::from(HEX[usize::from(byte >> 4)]));
                json.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
            character => json.push(character),
        }
    }
    json.push('"');
}

fn checked_byte_sum(mut values: impl Iterator<Item = u64>) -> Option<u64> {
    values.try_fold(0u64, u64::checked_add)
}

fn validate_admission_limits(
    sources: &BTreeMap<SourceId, FileRecord>,
    fonts: &BTreeMap<FontFaceId, FontRecord>,
    images: &BTreeMap<ImageResourceId, ImageRecord>,
    binding: &PublicationBinding,
) -> Result<(), BuildManifestError> {
    let limits = binding.limits.get();
    let include_count = sources
        .len()
        .checked_sub(1)
        .ok_or(BuildManifestError::MissingEntryInput)?;
    if include_count > limits.max_include_files as usize {
        return Err(BuildManifestError::IncludeFileLimit);
    }
    if sources
        .values()
        .any(|record| record.bytes > u64::from(limits.max_source_bytes))
    {
        return Err(BuildManifestError::InputSourceLimit);
    }
    if checked_byte_sum(sources.values().map(|record| record.bytes))
        .map_or(true, |bytes| bytes > limits.max_input_bytes)
    {
        return Err(BuildManifestError::InputAggregateLimit);
    }
    if fonts.len() > limits.max_fonts as usize {
        return Err(BuildManifestError::FontCountLimit);
    }
    if fonts.values().any(|record| {
        record.bytes == 0
            || record.bytes > limits.max_font_bytes
            || record.units_per_em == 0
            || record.glyph_count == 0
    }) {
        return Err(BuildManifestError::FontBytesLimit);
    }
    if images.len() > limits.max_images as usize {
        return Err(BuildManifestError::ImageCountLimit);
    }
    for image in images.values() {
        if image.bytes == 0 || image.bytes > limits.max_image_bytes {
            return Err(BuildManifestError::ImageBytesLimit);
        }
        let pixels = u64::from(image.pixel_width)
            .checked_mul(u64::from(image.pixel_height))
            .ok_or(BuildManifestError::ImagePixelLimit)?;
        if image.pixel_width == 0 || image.pixel_height == 0 || pixels > limits.max_image_pixels {
            return Err(BuildManifestError::ImagePixelLimit);
        }
        if image.decoded_bytes == 0 || image.decoded_bytes > limits.max_decoded_image_bytes {
            return Err(BuildManifestError::ImageDecodedBytesLimit);
        }
    }
    if checked_byte_sum(
        fonts
            .values()
            .map(|record| record.bytes)
            .chain(images.values().map(|record| record.bytes)),
    )
    .map_or(true, |bytes| bytes > limits.max_resource_bytes)
    {
        return Err(BuildManifestError::ResourceAggregateLimit);
    }
    if sources
        .values()
        .map(|record| record.bytes)
        .chain(fonts.values().map(|record| record.bytes))
        .chain(images.values().map(|record| record.bytes))
        .any(|bytes| bytes > JSON_SAFE_INTEGER_MAX as u64)
    {
        return Err(BuildManifestError::IntegerNotJsonSafe);
    }
    Ok(())
}

fn layout_record_for(
    publication: &ManifestPublicationContext,
    pagination: &PaginationResult,
) -> Result<LayoutRecord, BuildManifestError> {
    if pagination.passes().len() > usize::from(publication.limits.get().max_layout_passes) {
        return Err(BuildManifestError::LayoutLimit);
    }
    let page_count = u32::try_from(pagination.selected_pages().len())
        .map_err(|_| BuildManifestError::PageLimit)?;
    if page_count == 0 || page_count > publication.limits.get().max_pages {
        return Err(BuildManifestError::PageLimit);
    }
    LayoutRecord::from_pagination(pagination)
}

fn validate_complete_resource_closure(
    package: &ValidatedParsedPackage,
    admitted: AdmittedResourceLedgerToken<'_>,
) -> Result<(), BuildManifestError> {
    let declarations = &package.package().resources;
    if declarations.font_faces.len() != admitted.fonts().len()
        || declarations.images.len() != admitted.images().len()
    {
        return Err(BuildManifestError::PackageResourceMismatch);
    }
    for (declaration, resource) in declarations.font_faces.iter().zip(admitted.fonts()) {
        if declaration.font_face_id != resource.font_face_id()
            || declaration.uri != *resource.uri()
            || declaration.family != resource.family()
            || declaration.face_index != resource.face_index()
            || declaration
                .expected_sha256
                .is_some_and(|expected| expected != resource.content_hash())
        {
            return Err(BuildManifestError::PackageResourceMismatch);
        }
    }
    for (declaration, resource) in declarations.images.iter().zip(admitted.images()) {
        if declaration.image_id != resource.image_id()
            || declaration.uri != *resource.uri()
            || declaration
                .expected_sha256
                .is_some_and(|expected| expected != resource.content_hash())
        {
            return Err(BuildManifestError::PackageResourceMismatch);
        }
    }
    Ok(())
}

fn validate_ledger_pagination_closure(
    ledger: &ManifestAdmissionLedger,
    pagination: &PaginationResult,
) -> Result<(), BuildManifestError> {
    let package = ledger
        .package_epoch
        .as_ref()
        .ok_or(BuildManifestError::IncompleteLayoutAdmission)?;
    let resources = ledger
        .resource_fingerprint
        .ok_or(BuildManifestError::IncompleteLayoutAdmission)?;
    let epoch = pagination
        .selected_pass()
        .fingerprint_record()
        .layout_epoch();
    if epoch.document() != package.document()
        || epoch.style() != package.style()
        || epoch.admitted_resources() != resources
    {
        return Err(BuildManifestError::PackagePaginationMismatch);
    }
    Ok(())
}

fn validate_pdf_output_facts(
    publication: &ManifestPublicationContext,
    pagination: &PaginationResult,
    pdf: &VerifiedPdfBytesReceipt,
) -> Result<PreparedPdfOutputFacts, BuildManifestError> {
    let page_count = u32::try_from(pagination.selected_pages().len())
        .map_err(|_| BuildManifestError::PageLimit)?;
    validate_pdf_output_facts_with_page_count(publication, pagination, pdf, page_count)
}

fn validate_pdf_output_facts_with_page_count(
    publication: &ManifestPublicationContext,
    pagination: &PaginationResult,
    pdf: &VerifiedPdfBytesReceipt,
    page_count: u32,
) -> Result<PreparedPdfOutputFacts, BuildManifestError> {
    let facts = validate_pdf_receipt_facts(
        publication.config_fingerprint,
        publication.stream_compression,
        &publication.limits,
        publication.output_sink(),
        pdf,
    )?;
    if pdf.selected_layout_fingerprint() != pagination.final_fingerprint()
        || pdf.page_count() != page_count
    {
        return Err(BuildManifestError::PdfGraphReceiptMismatch);
    }
    Ok(facts)
}

fn validate_standalone_pdf_output_facts(
    output: &BuildOutputCommitContext,
    pdf: &VerifiedPdfBytesReceipt,
) -> Result<PreparedPdfOutputFacts, BuildManifestError> {
    validate_pdf_receipt_facts(
        output.config_fingerprint,
        output.stream_compression,
        &output.limits,
        output.output_sink(),
        pdf,
    )
}

fn validate_pdf_receipt_facts(
    config_fingerprint: EffectiveConfigFingerprint,
    stream_compression: PdfStreamCompression,
    limits: &ValidatedResourceLimits,
    sink: OutputSink,
    pdf: &VerifiedPdfBytesReceipt,
) -> Result<PreparedPdfOutputFacts, BuildManifestError> {
    let page_count = pdf.page_count();
    let pdf_object_count = pdf.object_count();
    let bytes = pdf.byte_length();
    let limits = limits.get();
    if pdf.config_fingerprint() != config_fingerprint {
        return Err(BuildManifestError::ConfigFingerprintMismatch);
    }
    if pdf.stream_compression() != stream_compression {
        return Err(BuildManifestError::StreamCompressionMismatch);
    }
    if page_count == 0 || pdf_object_count == 0 || bytes == 0 {
        return Err(BuildManifestError::EmptyBuiltOutput);
    }
    if page_count > limits.max_pages {
        return Err(BuildManifestError::PageLimit);
    }
    if pdf_object_count > limits.max_pdf_objects {
        return Err(BuildManifestError::PdfObjectLimit);
    }
    if bytes > limits.max_output_bytes || bytes > JSON_SAFE_INTEGER_MAX as u64 {
        return Err(BuildManifestError::OutputBytesLimit);
    }
    Ok(PreparedPdfOutputFacts {
        selected_fingerprint: pdf.selected_layout_fingerprint(),
        sink,
        bytes,
        sha256: pdf.content_hash(),
        page_count,
        pdf_object_count,
    })
}

fn prepare_built_manifest(
    publication: &ManifestPublicationContext,
    package: &ValidatedParsedPackage,
    admitted: AdmittedResourceLedgerToken<'_>,
    pagination: &PaginationResult,
    output: PreparedPdfOutputFacts,
) -> Result<ValidatedBuildManifest, BuildManifestError> {
    if output.sink != publication.output_sink() {
        return Err(BuildManifestError::OutputSinkMismatch);
    }
    let layout = layout_record_for(publication, pagination)?;
    let selected_epoch = pagination
        .selected_pass()
        .fingerprint_record()
        .layout_epoch();
    if selected_epoch.document() != package.epoch_identity().document()
        || selected_epoch.style() != package.epoch_identity().style()
        || selected_epoch.admitted_resources() != admitted.fingerprint()
    {
        return Err(BuildManifestError::PackagePaginationMismatch);
    }
    let page_count = u32::try_from(pagination.selected_pages().len())
        .map_err(|_| BuildManifestError::PageLimit)?;
    if output.selected_fingerprint != pagination.final_fingerprint()
        || output.page_count != page_count
    {
        return Err(BuildManifestError::PaginationReceiptMismatch);
    }
    let mut ledger = publication.begin_admission_ledger();
    ledger.admit_validated_package_sources(package)?;
    validate_complete_resource_closure(package, admitted)?;
    ledger.admit_resources(admitted)?;
    let records = ledger.manifest_records();
    let output = OutputRecord {
        sink: output.sink,
        bytes: output.bytes,
        sha256: output.sha256,
        page_count: output.page_count,
        pdf_object_count: output.pdf_object_count,
    };
    let manifest = BuildManifest::terminal(
        publication,
        BuildStatus::Built,
        records,
        Some(layout),
        Some(output),
        ManifestProfileFacts::default(),
    );
    ValidatedBuildManifest::new(
        manifest,
        ManifestExpectations::from_publication(publication),
    )
}

fn prepare_machine_built_manifest(
    publication: &ManifestPublicationContext,
    package: &ValidatedMachinePackage,
    capability: &MachinePdfPreflightReceipt,
    admitted: AdmittedResourceLedgerToken<'_>,
    pagination: &PaginationResult,
    layout_facts: StagingMachineLayoutFacts,
    output: PreparedPdfOutputFacts,
) -> Result<(ValidatedBuildManifest, ManifestAdmissionLedger), BuildManifestError> {
    if output.sink != publication.output_sink() {
        return Err(BuildManifestError::OutputSinkMismatch);
    }
    let layout = layout_record_for(publication, pagination)?
        .bind_machine_capability(capability, layout_facts.flow_registry_sha256())?;
    let (_, table_layouts, footnote_layout) = layout_facts.into_parts();
    let base_page_count = u32::try_from(pagination.selected_pages().len())
        .map_err(|_| BuildManifestError::PageLimit)?;
    let page_count = if capability.profile() == MachinePdfProfileId::TABLE_1 {
        table_layouts
            .iter()
            .filter_map(StagingTableLayoutFacts::target_page_end)
            .max()
            .unwrap_or(base_page_count)
            .max(base_page_count)
    } else if capability.profile() == MachinePdfProfileId::FOOTNOTE_1 {
        u32::try_from(
            footnote_layout
                .as_ref()
                .ok_or(BuildManifestError::MachineCapabilityMismatch)?
                .pages()
                .len(),
        )
        .map_err(|_| BuildManifestError::PageLimit)?
    } else {
        base_page_count
    };
    if output.selected_fingerprint != pagination.final_fingerprint()
        || output.page_count != page_count
    {
        return Err(BuildManifestError::PaginationReceiptMismatch);
    }

    let mut ledger = publication.begin_admission_ledger();
    ledger.admit_validated_machine_package(package)?;
    ledger.admit_machine_capability(package, capability)?;
    validate_complete_resource_closure(package.package(), admitted)?;
    ledger.admit_resources(admitted)?;
    ledger.admit_layout_selected(pagination)?;
    let records = ledger.manifest_records();
    let output = OutputRecord {
        sink: output.sink,
        bytes: output.bytes,
        sha256: output.sha256,
        page_count: output.page_count,
        pdf_object_count: output.pdf_object_count,
    };
    let manifest = BuildManifest::terminal(
        publication,
        BuildStatus::Built,
        records,
        Some(layout),
        Some(output),
        ManifestProfileFacts {
            table_layouts,
            footnote_layout,
            advanced_pagination: None,
        },
    );
    let validated = ValidatedBuildManifest::new(
        manifest,
        ManifestExpectations::from_publication(publication),
    )?;
    Ok((validated, ledger))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedBuildManifest(BuildManifest);
impl ValidatedBuildManifest {
    fn new(
        manifest: BuildManifest,
        expectations: ManifestExpectations<'_>,
    ) -> Result<Self, BuildManifestError> {
        manifest.validate(expectations)?;
        Ok(Self(manifest))
    }

    fn failed(
        publication: &ManifestPublicationContext,
        ledger: &ManifestAdmissionLedger,
        pagination: Option<&PaginationResult>,
    ) -> Result<Self, BuildManifestError> {
        if ledger.binding != publication.binding() {
            return Err(BuildManifestError::AdmissionLedgerBindingMismatch);
        }
        let mut candidate = ledger.clone();
        let layout = pagination
            .map(|pagination| {
                if candidate.machine.is_some() {
                    if candidate.machine_stage() == Some(ManifestAdmissionStage::ResourcesAdmitted)
                    {
                        candidate.admit_layout_selected(pagination)?;
                    } else if candidate.machine_stage()
                        != Some(ManifestAdmissionStage::LayoutSelected)
                    {
                        return Err(BuildManifestError::IncompleteLayoutAdmission);
                    } else {
                        validate_ledger_pagination_closure(&candidate, pagination)?;
                    }
                } else {
                    validate_ledger_pagination_closure(&candidate, pagination)?;
                }
                layout_record_for(publication, pagination)
            })
            .transpose()?;
        let records = candidate.manifest_records();
        let manifest = BuildManifest::terminal(
            publication,
            BuildStatus::Failed,
            records,
            layout,
            None,
            ManifestProfileFacts::default(),
        );
        Self::new(
            manifest,
            ManifestExpectations::from_publication(publication),
        )
    }

    pub const fn manifest(&self) -> &BuildManifest {
        &self.0
    }
    pub fn into_manifest(self) -> BuildManifest {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use typaxis_core::{
        sha256, BuildExecutionContext, ConfigResourceRoot, DocumentPackageContractId,
        EffectiveConfig, EffectiveDataVersions, HostPath, Length, PositiveLength, ReplacePolicy,
        ResourceLimits,
    };
    use typaxis_diagnostics::{MachineDiagnosticBudget, MachineDiagnosticPhase};
    use typaxis_display_list::{
        StagingForcedPageBreakDisplay, StagingMachineListDisplay, ValidatedDisplayDocument,
    };
    use typaxis_host_admission::{HostAdmissionSession, HostReadIdentityLedger};
    use typaxis_layout::{
        layout_table_grid, layout_table_row_bands, FlowCursor, FlowId, FlowTree, LayoutEpoch,
        PageContext, ProductionFlowIr, ResolvedPageSelection, TableCellLayoutInput,
    };
    use typaxis_machine_input::{
        AdmittedPackageBytes, HostMachineInputSession, MachineInputHostOptions,
    };
    use typaxis_machine_profile::MachinePdfPreflight;
    use typaxis_pagination::{
        paginate_staging_table, FlowWorkerCursor, InitialPaginationState, LayoutPass,
        LayoutPassInput, MultiFlowSelectionBuilder, MultiFlowTraceFacts, PageFrameKind,
        PageFramePlan, PagePlan, PaginationInput, PaginationOptions, PaginationOutcome,
        StagingTablePageInput,
    };
    use typaxis_pdf::PdfBackend;
    use typaxis_resources::{
        AdmittedResourceLedger, AdmittedResourceResolver, FrozenPdfResourcePlans,
    };
    use typaxis_syntax::{
        machine_profile_boundary::wire, DocumentPackageParser, MachineParseOutcome,
        PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
        StagingStylePackageParser, ValidatedMachinePackage, ValidatedParsedPackage,
    };
    use typaxis_text::GeneratedTextStore;

    fn shaper_identity() -> ShaperIdentity {
        ShaperIdentity::linked_reference()
    }

    #[test]
    fn machine_list_manifest_preserves_pdf_flow_fragment_and_marker_usage_closure() {
        let display = StagingMachineListDisplay::list_pdf_test_fixture();
        let pdf = StagingMachineListPdf::from_display(&display);
        let manifest = StagingMachineListManifestFact::from_pdf(&pdf);
        assert_eq!(manifest.lists().len(), 1);
        assert_eq!(manifest.items().len(), 1);
        let item = &manifest.items()[0];
        assert_eq!(item.item_node_id(), 2);
        assert_eq!(item.list_flow_id(), 0);
        assert_eq!(item.item_flow_id(), 1);
        assert_eq!(item.marker_fragment_id(), item.first_line_fragment_id());
        assert_eq!(item.marker_utf8(), "1.");
        assert_eq!(manifest.flow_registry_sha256(), pdf.flow_registry_sha256());
        assert_eq!(manifest.marker_usage_sha256(), pdf.marker_usage_sha256());
        assert!(manifest
            .canonical_jcs()
            .contains("\"generation_kind\":\"list_marker\""));
    }

    #[test]
    fn forced_page_break_manifest_preserves_pdf_page_tree_and_cursor_closure() {
        let display = StagingForcedPageBreakDisplay::forced_page_break_pdf_test_fixture();
        let pdf = StagingForcedPageBreakPdf::from_display(&display);
        let manifest = StagingForcedPageBreakManifestFact::from_pdf(&pdf);
        assert_eq!(manifest.page_count(), 2);
        assert_eq!(manifest.pages().len(), 2);
        assert_eq!(manifest.breaks().len(), 1);
        assert_eq!(manifest.breaks()[0].before_flow_local_ordinal(), 0);
        assert_eq!(manifest.breaks()[0].after_flow_local_ordinal(), 1);
        assert_eq!(
            manifest.pdf_page_tree_sha256(),
            typaxis_core::sha256(b"/Count 2\n")
        );
        assert!(manifest
            .canonical_jcs()
            .contains("\"produced_page_index\":1"));
    }

    fn tables() -> ResolvedDataTables {
        ResolvedDataTables::resolve("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap()
    }

    fn effective_config() -> EffectiveConfig {
        effective_config_with_limits(ResourceLimits::default())
    }

    fn effective_config_with_limits(limits: ResourceLimits) -> EffectiveConfig {
        EffectiveConfig::new(
            false,
            PdfStreamCompression::Flate,
            vec![ConfigResourceRoot::ProjectRoot],
            ["http", "https", "mailto"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
            limits,
        )
        .unwrap()
    }

    fn publication() -> ManifestPublicationContext {
        publication_for(&effective_config())
    }

    fn publication_for(config: &EffectiveConfig) -> ManifestPublicationContext {
        let execution = BuildExecutionContext::from_cli_token(
            OsStr::new("-"),
            None,
            Some(HostPath::new("target/build-manifest.json").unwrap()),
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let output = BuildOutputCommitContext::new(config, &execution).unwrap();
        ManifestPublicationContext::new(config, &output, shaper_identity(), &tables()).unwrap()
    }

    fn machine_publication_for(config: &EffectiveConfig) -> ManifestPublicationContext {
        machine_contexts_for(config).1
    }

    fn machine_contexts_for(
        config: &EffectiveConfig,
    ) -> (BuildOutputCommitContext, ManifestPublicationContext) {
        let execution = BuildExecutionContext::from_cli_token(
            OsStr::new("-"),
            None,
            Some(HostPath::new("target/machine-build-manifest.json").unwrap()),
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let output = BuildOutputCommitContext::new_machine(
            config,
            &execution,
            MachineProfileDescriptor::PARAGRAPH_1,
        )
        .unwrap();
        let publication =
            ManifestPublicationContext::new(config, &output, shaper_identity(), &tables()).unwrap();
        (output, publication)
    }

    static NEXT_MACHINE_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct MachineFixtureRoot(PathBuf);

    impl MachineFixtureRoot {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "typaxis-manifest-machine-{label}-{}-{}",
                std::process::id(),
                NEXT_MACHINE_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for MachineFixtureRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn machine_wire(with_paragraph: bool) -> wire::WireDocumentPackage {
        let blocks = if with_paragraph {
            vec![wire::WireBlock::Paragraph {
                node_id: 1,
                span: wire::WireSourceSpan {
                    source_id: 0,
                    start_byte: 0,
                    end_byte: 0,
                },
                classes: Vec::new(),
                children: Vec::new(),
            }]
        } else {
            Vec::new()
        };
        wire::WireDocumentPackage {
            contract: DocumentPackageContractId::V1_0,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            advanced: None,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: "input.tsf".to_owned(),
                utf8_byte_length: 0,
                sha256: sha256(&[]),
            }],
            text_buffers: Vec::new(),
            document: wire::WireDocument {
                node_id: 0,
                blocks,
                footnotes: Vec::new(),
            },
            style_sheet: wire::WireStyleSheet { rules: Vec::new() },
            page_masters: wire::WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![wire::WirePageMaster {
                    master_id: "default".to_owned(),
                    width: 100,
                    height: 100,
                    body: wire::WireRect {
                        x: 0,
                        y: 0,
                        width: 100,
                        height: 100,
                    },
                    header: None,
                    footer: None,
                    footnote: None,
                }],
                selection_rules: Vec::new(),
            },
            resources: wire::WireResourceCatalog {
                font_faces: Vec::new(),
                images: Vec::new(),
            },
        }
    }

    fn multi_flow_machine_wire() -> wire::WireDocumentPackage {
        let span = wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let paragraph = |node_id| wire::WireBlock::Paragraph {
            node_id,
            span,
            classes: Vec::new(),
            children: Vec::new(),
        };
        let mut package = machine_wire(false);
        package.contract = DocumentPackageContractId::V1_1;
        package.document.blocks = vec![
            paragraph(1),
            wire::WireBlock::List {
                node_id: 2,
                span,
                classes: Vec::new(),
                ordered: true,
                start: Some(1),
                items: vec![wire::WireListItem {
                    node_id: 3,
                    span,
                    blocks: vec![
                        paragraph(4),
                        wire::WireBlock::List {
                            node_id: 5,
                            span,
                            classes: Vec::new(),
                            ordered: false,
                            start: None,
                            items: vec![wire::WireListItem {
                                node_id: 6,
                                span,
                                blocks: vec![paragraph(7)],
                            }],
                        },
                    ],
                }],
            },
            wire::WireBlock::PageBreak {
                node_id: 8,
                span,
                classes: Vec::new(),
            },
        ];
        package
    }

    fn open_machine_fixture(
        label: &str,
        package: &wire::WireDocumentPackage,
        limits: &ValidatedResourceLimits,
    ) -> (
        MachineFixtureRoot,
        HostMachineInputSession,
        AdmittedPackageBytes,
    ) {
        let root = MachineFixtureRoot::new(label);
        let package_path = root.path().join("document-package.json");
        let bytes = wire::DocumentPackageEncoder::default()
            .to_jcs_vec(package)
            .unwrap();
        fs::write(&package_path, bytes).unwrap();
        fs::write(root.path().join("input.tsf"), []).unwrap();
        let options = MachineInputHostOptions::new(HostPath::new(package_path).unwrap(), None);
        let (session, raw) = HostMachineInputSession::open(options, limits).unwrap();
        (root, session, raw)
    }

    fn validated_machine_fixture(
        label: &str,
        wire_package: &wire::WireDocumentPackage,
    ) -> Box<ValidatedMachinePackage> {
        validated_machine_fixture_with_root(label, wire_package).1
    }

    fn validated_machine_fixture_with_root(
        label: &str,
        wire_package: &wire::WireDocumentPackage,
    ) -> (MachineFixtureRoot, Box<ValidatedMachinePackage>) {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let (root, session, raw) = open_machine_fixture(label, wire_package, &limits);
        let decoded = session
            .decode_and_bind(
                &raw,
                &wire::StrictDocumentPackageDecoder::new(),
                &wire::DocumentPackageDecodePolicy::new(&limits),
            )
            .unwrap();
        let sources = session.admit_sources(&decoded, &limits).unwrap();
        let admitted = session.finish(raw, decoded, sources).unwrap();
        let allowed_schemes = typaxis_core::DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>();
        let policy = PackageValidationPolicy::new(&limits, &allowed_schemes).unwrap();
        let package = match DocumentPackageParser::new().parse(admitted, &policy) {
            MachineParseOutcome::Parsed { package } => package,
            MachineParseOutcome::Failed { failure, .. } => panic!("machine fixture: {failure}"),
        };
        (root, package)
    }

    fn machine_capability(package: &ValidatedMachinePackage) -> MachinePdfPreflightReceipt {
        let mut budget = MachineDiagnosticBudget::new();
        let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
        MachinePdfPreflight::PARAGRAPH_1
            .run(package, &mut diagnostics)
            .unwrap()
    }

    fn failed_machine_manifest(progress: &MachineInputProgress) -> BuildManifest {
        let publication = machine_publication_for(&effective_config());
        let mut ledger = publication.begin_admission_ledger();
        ledger.admit_machine_input_progress(progress).unwrap();
        publication
            .prepare_failed(ledger, None)
            .unwrap()
            .manifest
            .into_manifest()
    }

    fn failed_manifest() -> BuildManifest {
        let config = effective_config();
        failed_manifest_for(&config)
    }

    fn failed_manifest_for(config: &EffectiveConfig) -> BuildManifest {
        BuildManifest {
            advanced_pagination: None,
            contract: CONTRACT.to_owned(),
            status: BuildStatus::Failed,
            deterministic: true,
            engine: EngineRecord::from_identity(&EngineIdentity::compiled()),
            data_versions: DataVersions::from_runtime(&tables(), shaper_identity()),
            config_sha256: config.fingerprint().bytes(),
            input_profile: BuildInputProfile::ReferenceSource1,
            package_input: None,
            inputs: vec![FileRecord {
                uri: PortablePath::new("entry.tsf").unwrap(),
                bytes: 0,
                sha256: [0; 32],
            }],
            fonts: vec![],
            images: vec![],
            pdf_profile: PDF_PROFILE.to_owned(),
            stream_compression: PdfStreamCompression::Flate,
            layout: None,
            output: None,
            table_layouts: Vec::new(),
            footnote_layout: None,
        }
    }

    fn manifest_table_length(raw: i64) -> PositiveLength {
        PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap()
    }

    #[test]
    fn staging_table_manifest_binds_selected_row_cell_and_header_facts() {
        let span = wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let cell = |node_id| wire::WireTableCell {
            node_id,
            span,
            colspan: 1,
            rowspan: 1,
            blocks: Vec::new(),
        };
        let wire_package = wire::WireDocumentPackage {
            contract: DocumentPackageContractId::V1_2,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            advanced: None,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: "manifest-table.tsf".to_owned(),
                utf8_byte_length: 0,
                sha256: sha256(&[]),
            }],
            text_buffers: Vec::new(),
            document: wire::WireDocument {
                node_id: 0,
                blocks: vec![wire::WireBlock::Table {
                    node_id: 1,
                    span,
                    classes: Vec::new(),
                    columns: vec![wire::WireTableColumn::Fixed { width: 10 }],
                    head: vec![wire::WireTableRow {
                        node_id: 2,
                        span,
                        cells: vec![cell(3)],
                    }],
                    body: vec![wire::WireTableRow {
                        node_id: 4,
                        span,
                        cells: vec![cell(5)],
                    }],
                }],
                footnotes: Vec::new(),
            },
            style_sheet: wire::WireStyleSheet { rules: Vec::new() },
            page_masters: wire::WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![wire::WirePageMaster {
                    master_id: "default".to_owned(),
                    width: 10,
                    height: 5,
                    body: wire::WireRect {
                        x: 0,
                        y: 0,
                        width: 10,
                        height: 5,
                    },
                    header: None,
                    footer: None,
                    footnote: None,
                }],
                selection_rules: Vec::new(),
            },
            resources: wire::WireResourceCatalog {
                font_faces: Vec::new(),
                images: Vec::new(),
            },
        };
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_fragments: 7,
            ..ResourceLimits::default()
        })
        .unwrap();
        let bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&wire_package)
            .unwrap();
        let decoded = wire::StagingStyleDocumentPackageDecoder::new()
            .decode(&bytes, &wire::DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let package = StagingStylePackageParser::new()
            .parse(
                decoded,
                String::new(),
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            )
            .unwrap();
        let generated = package
            .package()
            .materialize_initial_generated_text(&limits)
            .unwrap();
        let admitted =
            AdmittedResourceResolver::new(&package.package().package().resources, &limits)
                .unwrap()
                .finish()
                .unwrap();
        let bound = package
            .package()
            .bind_generated_text(&generated, &limits)
            .unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(bound, admitted.token()).unwrap();
        let ir = ProductionFlowIr::for_empty_paragraph_content(package.package(), epoch, &limits)
            .unwrap();
        let style = package
            .compute_table_style(typaxis_core::NodeId::new(1))
            .unwrap();
        let grid = layout_table_grid(
            &package,
            typaxis_core::NodeId::new(1),
            &style,
            &ir,
            manifest_table_length(10),
            &limits,
        )
        .unwrap();
        let measurements = grid
            .cells()
            .iter()
            .map(|cell| {
                let sizes = if cell.cell_owner() == typaxis_core::NodeId::new(3) {
                    vec![manifest_table_length(2)]
                } else {
                    vec![manifest_table_length(2), manifest_table_length(2)]
                };
                TableCellLayoutInput::new(cell.cell_owner(), cell.flow_id(), sizes)
            })
            .collect();
        let rows = layout_table_row_bands(&grid, measurements, &limits).unwrap();
        let selected = paginate_staging_table(
            &rows,
            &ir,
            StagingTablePageInput::new(manifest_table_length(5), manifest_table_length(5)).unwrap(),
            &limits,
        )
        .unwrap();
        let trace = selected.trace_facts().unwrap();
        let facts = StagingTableLayoutFacts::from_selected(&trace, &grid, &selected).unwrap();

        assert_eq!(facts.page_count(), 2);
        assert_eq!(facts.body_block_size(), 5);
        assert_eq!(facts.first_page_remaining_block_size(), 5);
        assert_eq!(facts.columns().len(), 1);
        assert_eq!(facts.columns()[0].column_ordinal(), 0);
        assert_eq!(facts.columns()[0].input_kind(), "fixed");
        assert_eq!(facts.columns()[0].input_value(), 10);
        assert_eq!(facts.columns()[0].rounded_fraction_width(), None);
        assert_eq!(facts.columns()[0].final_width(), 10);
        assert_eq!(facts.rounding_residual(), 0);
        assert_eq!(facts.residual_recipient(), None);
        assert_eq!(facts.rows().len(), 2);
        assert_eq!(facts.header_sources().len(), 1);
        assert_eq!(facts.header_occurrences().len(), 2);
        assert_eq!(facts.rows()[0].cells()[0].selected_block_extent(), 2);
        assert_eq!(
            facts.header_occurrences()[1].source_fragment_id(),
            facts.header_sources()[0].source_fragment_id()
        );
        assert_eq!(
            facts.selected_layout_sha256(),
            selected.fingerprint().bytes()
        );
        assert!(facts
            .canonical_jcs()
            .contains("\"selected_block_extent\":2"));
        assert!(facts.canonical_jcs().contains("\"colspan\":1"));
        assert!(facts.canonical_jcs().contains("\"continuation_before\""));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn staging_multi_flow_manifest_facts_bind_all_canonical_terminals() {
        let machine = validated_machine_fixture("multi-flow-facts", &multi_flow_machine_wire());
        let package = machine.package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = package.materialize_initial_generated_text(&limits).unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let bound = package.bind_generated_text(&generated, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(bound, admitted.token()).unwrap();
        let ir = ProductionFlowIr::for_empty_paragraph_content(package, epoch, &limits).unwrap();

        let mut selection = MultiFlowSelectionBuilder::new(&ir);
        for flow_id in [FlowId::new(2), FlowId::new(0), FlowId::new(1)] {
            let mut worker = FlowWorkerCursor::new(&ir, flow_id).unwrap();
            let terminal = ir
                .registry()
                .flow(flow_id)
                .unwrap()
                .terminal()
                .owner_local_ordinal();
            while worker.next_boundary() < terminal {
                worker.advance(&ir).unwrap();
            }
            selection.register(worker.finish(&ir).unwrap()).unwrap();
        }
        let selected = selection.finish().unwrap();
        let trace = MultiFlowTraceFacts::new(&ir, &selected).unwrap();
        let facts = StagingMultiFlowLayoutFacts::from_selected(&trace, &selected).unwrap();

        assert_eq!(facts.flows().len(), 3);
        assert_eq!(
            facts
                .flows()
                .iter()
                .map(|flow| (
                    flow.flow_id(),
                    flow.owner_node_id(),
                    flow.parent_flow_id(),
                    flow.terminal(),
                ))
                .collect::<Vec<_>>(),
            vec![(0, 0, None, 3), (1, 3, Some(0), 2), (2, 6, Some(1), 1)]
        );
        assert_eq!(
            facts.flow_registry_sha256(),
            ir.registry().receipt().fingerprint().bytes()
        );
        assert_eq!(
            facts.selected_state_sha256(),
            selected.fingerprint().bytes()
        );
        assert!(facts
            .canonical_jcs()
            .starts_with("{\"contract\":\"typaxis.contract/1.2\",\"flow_registry_sha256\":"));
        assert!(facts
            .canonical_jcs()
            .contains("\"parent_flow_id\":1,\"terminal\":1"));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_failed_projection_preserves_decode_stage_nullability() {
        let config = effective_config();
        let limits = config.limits().clone();

        let missing_root = MachineFixtureRoot::new("no-input");
        let missing = MachineInputHostOptions::new(
            HostPath::new(missing_root.path().join("missing-package.json")).unwrap(),
            None,
        );
        let error = HostMachineInputSession::open(missing, &limits).unwrap_err();
        let (_, no_input) = error.into_parts();
        let manifest = failed_machine_manifest(&no_input);
        assert_eq!(
            manifest.input_profile(),
            BuildInputProfile::MachinePdfParagraph1
        );
        assert!(manifest.package_input().is_none());
        assert!(manifest.inputs().is_empty());

        let wire_package = machine_wire(false);
        let (_root, session, raw) = open_machine_fixture("stages", &wire_package, &limits);
        let raw_manifest = failed_machine_manifest(&session.progress());
        let raw_record = raw_manifest.package_input().unwrap();
        assert_eq!(raw_record.uri().as_str(), "document-package.json");
        assert!(raw_record.contract().is_none());
        assert!(raw_record.canonical_sha256().is_none());
        assert!(raw_manifest.inputs().is_empty());

        let decoded = session
            .decode_and_bind(
                &raw,
                &wire::StrictDocumentPackageDecoder::new(),
                &wire::DocumentPackageDecodePolicy::new(&limits),
            )
            .unwrap();
        let decoded_manifest = failed_machine_manifest(&session.progress());
        let decoded_record = decoded_manifest.package_input().unwrap();
        assert_eq!(
            decoded_record.contract(),
            Some(DocumentPackageContractId::V1_0)
        );
        assert!(decoded_record.canonical_sha256().is_some());
        assert!(decoded_manifest.inputs().is_empty());

        let sources = session.admit_sources(&decoded, &limits).unwrap();
        let sources_manifest = failed_machine_manifest(&session.progress());
        assert_eq!(sources_manifest.inputs().len(), 1);
        assert_eq!(sources_manifest.inputs()[0].uri().as_str(), "input.tsf");
        assert_ne!(
            sources_manifest.package_input().unwrap().uri(),
            sources_manifest.inputs()[0].uri()
        );
        let _ = session.finish(raw, decoded, sources).unwrap();
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_built_preflight_closes_profile_package_resources_layout_and_pdf() {
        let config = effective_config();
        let (root, package) = validated_machine_fixture_with_root("built", &machine_wire(false));
        let capability = machine_capability(&package);
        let admitted =
            AdmittedResourceResolver::new(&package.package().package().resources, config.limits())
                .unwrap()
                .finish()
                .unwrap();
        let pagination = pagination_for(package.package(), &admitted);
        let pdf = serialized_pdf_for(package.package(), &admitted, &pagination, &config);
        let output_path = root.path().join("output.pdf");
        let manifest_path = root.path().join("manifest.json");
        let execution = BuildExecutionContext::from_cli_token(
            output_path.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let output_context = BuildOutputCommitContext::new_machine(
            &config,
            &execution,
            MachineProfileDescriptor::PARAGRAPH_1,
        )
        .unwrap();
        let publication =
            ManifestPublicationContext::new(&config, &output_context, shaper_identity(), &tables())
                .unwrap();
        assert_eq!(
            publication.input_profile(),
            BuildInputProfile::MachinePdfParagraph1
        );

        let prepared = publication
            .prepare_machine_built(
                &package,
                &capability,
                admitted.token(),
                &pagination,
                StagingMachineLayoutFacts::new(None, Vec::new()),
                pdf,
            )
            .unwrap();
        let manifest = prepared.manifest.manifest();
        assert_eq!(manifest.status(), BuildStatus::Built);
        assert_eq!(
            manifest.input_profile(),
            BuildInputProfile::MachinePdfParagraph1
        );
        let package_record = manifest.package_input().unwrap();
        assert_eq!(
            package_record.contract(),
            Some(DocumentPackageContractId::V1_0)
        );
        assert!(package_record.canonical_sha256().is_some());
        assert_eq!(manifest.inputs().len(), 1);
        assert_eq!(manifest.inputs()[0].uri().as_str(), "input.tsf");
        assert!(manifest.layout().is_some());
        assert!(manifest.output().is_some());
        let committed = output_context.commit_prepared_built(prepared).unwrap();
        assert_eq!(committed.manifest().manifest().status(), BuildStatus::Built);
        assert!(fs::read(output_path).unwrap().starts_with(b"%PDF-"));
        assert!(fs::read_to_string(manifest_path)
            .unwrap()
            .contains("\"input_profile\":\"typaxis.machine-pdf/paragraph-1\""));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_ledger_rejects_profile_package_session_and_resource_swaps() {
        let config = effective_config();
        let first = validated_machine_fixture("swap-first", &machine_wire(false));
        let identical_other_session =
            validated_machine_fixture("swap-session", &machine_wire(false));
        let different_package = validated_machine_fixture("swap-package", &machine_wire(true));
        let first_capability = machine_capability(&first);
        let session_capability = machine_capability(&identical_other_session);
        let different_capability = machine_capability(&different_package);

        let reference = publication();
        assert_eq!(
            reference
                .begin_admission_ledger()
                .admit_validated_machine_package(&first),
            Err(BuildManifestError::InputProfileMismatch)
        );

        let machine = machine_publication_for(&config);
        let mut session_swap = machine.begin_admission_ledger();
        session_swap
            .admit_validated_machine_package(&first)
            .unwrap();
        assert_eq!(
            session_swap.admit_machine_capability(&first, &session_capability),
            Err(BuildManifestError::MachineCapabilityMismatch)
        );

        let machine = machine_publication_for(&config);
        let mut package_swap = machine.begin_admission_ledger();
        package_swap
            .admit_validated_machine_package(&first)
            .unwrap();
        assert_eq!(
            package_swap.admit_machine_capability(&different_package, &different_capability),
            Err(BuildManifestError::MachineCapabilityMismatch)
        );

        let machine = machine_publication_for(&config);
        let mut resource_swap = machine.begin_admission_ledger();
        resource_swap
            .admit_validated_machine_package(&first)
            .unwrap();
        resource_swap
            .admit_machine_capability(&first, &first_capability)
            .unwrap();
        let first_resolver = AdmittedResourceResolver::new_empty(config.limits()).unwrap();
        let first_progress = first_resolver.progress_token();
        let first_resources = first_resolver.finish().unwrap();
        let foreign_resources = AdmittedResourceResolver::new_empty(config.limits())
            .unwrap()
            .finish()
            .unwrap();
        resource_swap
            .admit_resource_progress(first_progress)
            .unwrap();
        assert_eq!(
            resource_swap.admit_resources(foreign_resources.token()),
            Err(BuildManifestError::ResourceProgressMismatch)
        );
        resource_swap
            .admit_resources(first_resources.token())
            .unwrap();
        assert_eq!(
            resource_swap.machine_stage(),
            Some(ManifestAdmissionStage::ResourcesAdmitted)
        );
    }

    fn validate(manifest: BuildManifest) -> Result<ValidatedBuildManifest, BuildManifestError> {
        let publication = publication();
        validate_against(manifest, &publication)
    }

    fn validate_against(
        manifest: BuildManifest,
        publication: &ManifestPublicationContext,
    ) -> Result<ValidatedBuildManifest, BuildManifestError> {
        ValidatedBuildManifest::new(
            manifest,
            ManifestExpectations::from_publication(publication),
        )
    }

    fn validated_package(source_text: &str) -> ValidatedParsedPackage {
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new(format!("entry-{}.tsf", source_text.replace(' ', "-"))).unwrap(),
            text: String::new(),
        };
        let config = effective_config();
        let outcome = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes()).unwrap(),
        );
        let ParseOutcome::Parsed { package, .. } = outcome else {
            panic!("reference package must parse");
        };
        *package
    }

    fn pagination_for(
        package: &ValidatedParsedPackage,
        admitted: &typaxis_resources::AdmittedResourceLedger,
    ) -> PaginationResult {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let store = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let generated = package.bind_generated_text(&store, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap();
        let flow = FlowTree::empty(package, epoch).unwrap();
        let initial = InitialPaginationState::new(&flow, package, &limits).unwrap();
        let package_context = package.pagination_context();
        let mut input = PaginationInput::new(
            initial,
            &package_context,
            PaginationOptions::from_limits(&limits, false),
        )
        .unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let master = &package.package().page_masters.masters[0];
        let pages = vec![PagePlan {
            page_index: 0,
            master_id: master.master_id.clone(),
            frames: vec![PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 0,
                bounds: master.body,
            }],
            fragments: vec![],
            footnote_ids: vec![],
            float_decisions: vec![],
            column_decisions: vec![],
            resolved_references: vec![],
        }];
        let page_selection = ResolvedPageSelection::new(&flow, &cursor, package).unwrap();
        let page_context = PageContext::select(0, &page_selection, &package_context).unwrap();
        let mut budget = input.take_work_budget().unwrap();
        let first_input = LayoutPassInput::initial(&input);
        let first_fingerprint = first_input.fingerprint();
        let mut first_permit = budget.begin_pass(0, first_input).unwrap();
        for page in &pages {
            first_permit
                .begin_page(&page_context, &cursor, &page.frames)
                .unwrap();
            first_permit.finish_page(page).unwrap();
        }
        let first_receipt = first_permit.finish(&flow, &pages).unwrap();
        let first = LayoutPass::new(
            first_receipt,
            first_fingerprint,
            &flow,
            pages.clone(),
            store.clone(),
        )
        .unwrap();
        let transition = first.transition_references(package, &limits).unwrap();
        let second_input = LayoutPassInput::transitioned(transition);
        let second_fingerprint = second_input.fingerprint();
        let mut second_permit = budget.begin_pass(1, second_input).unwrap();
        for page in &pages {
            second_permit
                .begin_page(&page_context, &cursor, &page.frames)
                .unwrap();
            second_permit.finish_page(page).unwrap();
        }
        let second_receipt = second_permit.finish(&flow, &pages).unwrap();
        let second =
            LayoutPass::new(second_receipt, second_fingerprint, &flow, pages, store).unwrap();
        PaginationOutcome::new(
            vec![first, second],
            ConvergenceStatus::Converged,
            &input,
            budget.finish(),
        )
        .unwrap()
        .into_result()
    }

    fn serialized_pdf_for(
        package: &ValidatedParsedPackage,
        admitted: &AdmittedResourceLedger,
        pagination: &PaginationResult,
        config: &EffectiveConfig,
    ) -> VerifiedPdfBytesReceipt {
        let display =
            ValidatedDisplayDocument::paint_blank_selected(package, pagination, config).unwrap();
        let resources = FrozenPdfResourcePlans::from_verified_receipts(
            &display,
            admitted,
            config.limits(),
            vec![],
        )
        .unwrap();
        let graph = PdfBackend::build(display, resources, config.limits()).unwrap();
        PdfBackend::serialize(graph, config).unwrap()
    }

    #[test]
    fn streamed_pdf_facts_close_over_the_serializer_receipt() {
        let config = effective_config();
        let package = validated_package("");
        let admitted = AdmittedResourceResolver::new(&package.package().resources, config.limits())
            .unwrap()
            .finish()
            .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let pdf = serialized_pdf_for(&package, &admitted, &pagination, &config);
        let mut sink = Vec::new();
        let streamed = pdf.write_streaming(&mut sink).unwrap();

        assert_eq!(sink, pdf.bytes());
        assert_eq!(validate_streamed_pdf_facts(&pdf, streamed), Ok(()));
    }

    #[test]
    fn stdout_prefix_failure_is_distinct_from_prepublication_io() {
        struct PrefixThenFail {
            accepted: usize,
            maximum: usize,
        }

        impl Write for PrefixThenFail {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.accepted == self.maximum {
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, "injected"));
                }
                let written = bytes.len().min(self.maximum - self.accepted);
                self.accepted += written;
                Ok(written)
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let config = effective_config();
        let package = validated_package("");
        let admitted = AdmittedResourceResolver::new(&package.package().resources, config.limits())
            .unwrap()
            .finish()
            .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let pdf = serialized_pdf_for(&package, &admitted, &pagination, &config);
        let mut sink = PrefixThenFail {
            accepted: 0,
            maximum: 11,
        };
        assert!(matches!(
            stream_verified_pdf(&pdf, &mut sink),
            Err(PdfSinkCommitError::StdoutPartial {
                bytes_written: 11,
                source,
            }) if source.kind() == io::ErrorKind::BrokenPipe
        ));
    }

    #[test]
    fn failed_factory_owns_only_publication_bound_admission_facts() {
        let publication = publication();
        let mut ledger = publication.begin_admission_ledger();
        ledger
            .admit_source(
                &SourceRecord::new(
                    SourceId::new(0),
                    PortablePath::new("z-entry.tsf").unwrap(),
                    "entry".to_owned(),
                )
                .unwrap(),
            )
            .unwrap();
        ledger
            .admit_source(
                &SourceRecord::new(
                    SourceId::new(1),
                    PortablePath::new("a-include.tsf").unwrap(),
                    "include".to_owned(),
                )
                .unwrap(),
            )
            .unwrap();

        let failed = ValidatedBuildManifest::failed(&publication, &ledger, None).unwrap();
        assert_eq!(failed.manifest().status(), BuildStatus::Failed);
        assert_eq!(failed.manifest().inputs().len(), 2);
        assert_eq!(
            failed.manifest().inputs()[0].uri().as_str(),
            "a-include.tsf"
        );
        assert_eq!(failed.manifest().inputs()[1].uri().as_str(), "z-entry.tsf");
        assert!(failed.manifest().output().is_none());

        let other_config = effective_config_with_limits(ResourceLimits {
            max_pages: ResourceLimits::default().max_pages - 1,
            ..ResourceLimits::default()
        });
        assert_eq!(
            ValidatedBuildManifest::failed(&publication_for(&other_config), &ledger, None),
            Err(BuildManifestError::AdmissionLedgerBindingMismatch)
        );
        assert_eq!(
            ValidatedBuildManifest::failed(&publication_for(&effective_config()), &ledger, None,),
            Err(BuildManifestError::AdmissionLedgerBindingMismatch)
        );
    }

    #[cfg(unix)]
    #[test]
    fn file_output_owner_commits_atomically_and_honors_replace_policy() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("manifest-commit-{}-{ordinal}", std::process::id());
        let output = output_root.join(format!("{stem}.pdf"));

        let no_replace = BuildExecutionContext::from_cli_token(
            output.as_os_str(),
            None,
            None,
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        commit_file_pdf_bytes(&no_replace, b"first complete pdf").unwrap();
        assert_eq!(fs::read(&output).unwrap(), b"first complete pdf");
        let second = commit_file_pdf_bytes(&no_replace, b"must not replace").unwrap_err();
        assert!(matches!(
            second,
            PdfSinkCommitError::Io(error) if error.kind() == io::ErrorKind::AlreadyExists
        ));
        assert_eq!(fs::read(&output).unwrap(), b"first complete pdf");

        let replace = BuildExecutionContext::from_cli_token(
            output.as_os_str(),
            None,
            None,
            None,
            ReplacePolicy::Replace,
        )
        .unwrap();
        commit_file_pdf_bytes(&replace, b"replacement pdf").unwrap();
        assert_eq!(fs::read(&output).unwrap(), b"replacement pdf");
        fs::remove_file(output).unwrap();
    }

    #[test]
    fn post_publish_sync_failure_is_not_classified_as_rollback() {
        let outcome =
            classify_parent_sync(Err(io::Error::other("injected directory sync failure")));
        assert!(matches!(
            outcome,
            SinkCommitDurability::PublishedButDurabilityUncertain(error)
                if error.kind() == io::ErrorKind::Other
        ));
    }

    #[cfg(unix)]
    #[test]
    fn sink_owner_revalidates_target_aliases_immediately_before_write() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("manifest-alias-recheck-{}-{ordinal}", std::process::id());
        let output = output_root.join(format!("{stem}.pdf"));
        let manifest_path = output_root.join(format!("{stem}.json"));
        fs::write(&output, b"previous pdf").unwrap();
        fs::write(&manifest_path, b"previous manifest").unwrap();
        let execution = BuildExecutionContext::from_cli_token(
            output.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            None,
            ReplacePolicy::Replace,
        )
        .unwrap();

        fs::remove_file(&manifest_path).unwrap();
        fs::hard_link(&output, &manifest_path).unwrap();
        assert!(matches!(
            commit_file_pdf_bytes(&execution, b"new pdf"),
            Err(PdfSinkCommitError::Execution(
                BuildExecutionError::AliasedWriteTarget
            ))
        ));
        assert_eq!(fs::read(&output).unwrap(), b"previous pdf");

        fs::remove_file(manifest_path).unwrap();
        fs::remove_file(output).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn failed_manifest_is_session_bound_and_atomically_published() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("manifest-failed-publish-{}-{ordinal}", std::process::id());
        let output_path = output_root.join(format!("{stem}.pdf"));
        let manifest_path = output_root.join(format!("{stem}.json"));
        let config = effective_config();
        let execution = BuildExecutionContext::from_cli_token(
            output_path.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            None,
            ReplacePolicy::Replace,
        )
        .unwrap();

        let expected_output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let wrong_output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        assert_ne!(expected_output.binding(), wrong_output.binding());
        let publication = ManifestPublicationContext::new(
            &config,
            &expected_output,
            shaper_identity(),
            &tables(),
        )
        .unwrap();
        let ledger = publication.begin_admission_ledger();
        let prepared = publication.prepare_failed(ledger, None).unwrap();
        assert!(matches!(
            wrong_output.commit_prepared_failed(prepared),
            Err(ManifestSinkCommitError::InvalidFacts(
                BuildManifestError::OutputReceiptBindingMismatch
            ))
        ));
        assert!(!manifest_path.exists());

        let output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let publication =
            ManifestPublicationContext::new(&config, &output, shaper_identity(), &tables())
                .unwrap();
        let ledger = publication.begin_admission_ledger();
        let prepared = publication.prepare_failed(ledger, None).unwrap();
        let committed = output.commit_prepared_failed(prepared).unwrap();
        let bytes = fs::read(&manifest_path).unwrap();
        assert!(bytes.starts_with(b"{\"config_sha256\":"));
        assert!(!bytes.contains(&b'\n'));
        assert_eq!(
            committed.manifest().manifest().status(),
            BuildStatus::Failed
        );
        assert_eq!(committed.receipt().bytes(), bytes.len() as u64);
        assert!(!output_path.exists());

        fs::remove_file(manifest_path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn failed_manifest_no_replace_preserves_existing_sidecar() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("manifest-no-replace-{}-{ordinal}", std::process::id());
        let output_path = output_root.join(format!("{stem}.pdf"));
        let manifest_path = output_root.join(format!("{stem}.json"));
        fs::write(&manifest_path, b"existing manifest").unwrap();
        let config = effective_config();
        let execution = BuildExecutionContext::from_cli_token(
            output_path.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let publication =
            ManifestPublicationContext::new(&config, &output, shaper_identity(), &tables())
                .unwrap();
        let ledger = publication.begin_admission_ledger();
        let prepared = publication.prepare_failed(ledger, None).unwrap();
        assert!(matches!(
            output.commit_prepared_failed(prepared),
            Err(ManifestSinkCommitError::Io(error))
                if error.kind() == io::ErrorKind::AlreadyExists
        ));
        assert_eq!(fs::read(&manifest_path).unwrap(), b"existing manifest");
        assert!(!output_path.exists());

        fs::remove_file(manifest_path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn prepared_build_pdf_sink_failure_publishes_failed_manifest() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!(
            "manifest-built-sink-failure-{}-{ordinal}",
            std::process::id()
        );
        let output_path = output_root.join(format!("{stem}.pdf"));
        let manifest_path = output_root.join(format!("{stem}.json"));
        fs::write(&output_path, b"existing pdf must be preserved").unwrap();

        let config = effective_config();
        let package = validated_package("");
        let admitted = AdmittedResourceResolver::new(&package.package().resources, config.limits())
            .unwrap()
            .finish()
            .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let pdf = serialized_pdf_for(&package, &admitted, &pagination, &config);
        let execution = BuildExecutionContext::from_cli_token(
            output_path.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let publication =
            ManifestPublicationContext::new(&config, &output, shaper_identity(), &tables())
                .unwrap();
        let prepared = publication
            .prepare_built(&package, admitted.token(), &pagination, pdf)
            .unwrap();

        let error = output.commit_prepared_built(prepared).unwrap_err();
        let BuiltPublicationCommitError::PdfSinkFailed {
            source,
            failed_manifest,
        } = error
        else {
            panic!("PDF no-replace failure must publish the sealed failed manifest");
        };
        assert!(matches!(
            source,
            PdfSinkCommitError::Io(error) if error.kind() == io::ErrorKind::AlreadyExists
        ));
        let FailedManifestPublication::Committed(committed) = failed_manifest else {
            panic!("failed manifest must commit when its target is available");
        };
        assert_eq!(
            committed.manifest().manifest().status(),
            BuildStatus::Failed
        );
        assert!(committed.manifest().manifest().layout().is_some());
        assert!(committed.manifest().manifest().output().is_none());

        let manifest_bytes = fs::read(&manifest_path).unwrap();
        assert!(manifest_bytes
            .windows(17)
            .any(|bytes| bytes == b"\"status\":\"failed\""));
        assert!(manifest_bytes
            .windows(13)
            .any(|bytes| bytes == b"\"output\":null"));
        assert_eq!(committed.receipt().bytes(), manifest_bytes.len() as u64);
        assert_eq!(
            fs::read(&output_path).unwrap(),
            b"existing pdf must be preserved"
        );

        fs::remove_file(manifest_path).unwrap();
        fs::remove_file(output_path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn sealed_read_alias_blocks_pdf_and_failed_manifest_publication() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = output_root.join(format!(
            "manifest-read-alias-{}-{ordinal}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let input_path = root.join("input.pdf");
        let manifest_path = root.join("manifest.json");
        fs::write(&input_path, b"admitted input").unwrap();

        let read_ledger = HostReadIdentityLedger::new();
        let host = HostAdmissionSession::new_contained_root_with_read_ledger(
            &HostPath::new(root.clone()).unwrap(),
            &read_ledger,
        )
        .unwrap();
        drop(
            host.roots()
                .open(&PortablePath::new("input.pdf").unwrap())
                .unwrap(),
        );
        let token = read_ledger.token().unwrap();

        let config = effective_config();
        let package = validated_package("");
        let admitted = AdmittedResourceResolver::new(&package.package().resources, config.limits())
            .unwrap()
            .finish()
            .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let pdf = serialized_pdf_for(&package, &admitted, &pagination, &config);
        let execution = BuildExecutionContext::from_cli_token(
            input_path.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            None,
            ReplacePolicy::Replace,
        )
        .unwrap();
        let output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let publication =
            ManifestPublicationContext::new(&config, &output, shaper_identity(), &tables())
                .unwrap();
        let prepared = publication
            .prepare_built(&package, admitted.token(), &pagination, pdf)
            .unwrap()
            .bind_read_ledger(token)
            .unwrap();
        let error = output.commit_prepared_built(prepared).unwrap_err();
        let BuiltPublicationCommitError::PdfSinkFailed {
            source: PdfSinkCommitError::Execution(BuildExecutionError::AliasedReadWriteTarget),
            failed_manifest: FailedManifestPublication::CommitError(failed_manifest),
        } = error
        else {
            panic!("read/write alias must block both terminal writes");
        };
        assert!(matches!(
            *failed_manifest,
            ManifestSinkCommitError::Execution(BuildExecutionError::AliasedReadWriteTarget)
        ));
        assert_eq!(fs::read(&input_path).unwrap(), b"admitted input");
        assert!(!manifest_path.exists());

        fs::remove_file(input_path).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn successful_pdf_commit_still_preserves_manifest_failure_receipt() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("manifest-post-pdf-failure-{}-{ordinal}", std::process::id());
        let output_path = output_root.join(format!("{stem}.pdf"));
        let manifest_path = output_root.join(format!("{stem}.json"));
        fs::write(&manifest_path, b"existing manifest must be preserved").unwrap();

        let config = effective_config();
        let package = validated_package("");
        let admitted = AdmittedResourceResolver::new(&package.package().resources, config.limits())
            .unwrap()
            .finish()
            .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let pdf = serialized_pdf_for(&package, &admitted, &pagination, &config);
        let expected_pdf_bytes = pdf.byte_length();
        let execution = BuildExecutionContext::from_cli_token(
            output_path.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let publication =
            ManifestPublicationContext::new(&config, &output, shaper_identity(), &tables())
                .unwrap();
        let prepared = publication
            .prepare_built(&package, admitted.token(), &pagination, pdf)
            .unwrap();

        let error = output.commit_prepared_built(prepared).unwrap_err();
        let BuiltPublicationCommitError::ManifestIo {
            pdf_receipt,
            source,
        } = error
        else {
            panic!("a post-PDF no-replace failure must retain the PDF receipt");
        };
        assert_eq!(source.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(pdf_receipt.bytes(), expected_pdf_bytes);
        assert_eq!(
            fs::metadata(&output_path).unwrap().len(),
            expected_pdf_bytes
        );
        assert_eq!(
            fs::read(&manifest_path).unwrap(),
            b"existing manifest must be preserved"
        );

        fs::remove_file(manifest_path).unwrap();
        fs::remove_file(output_path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn diagnostics_failure_after_pdf_leaves_built_manifest_unpublished() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("manifest-diagnostics-stop-{}-{ordinal}", std::process::id());
        let output_path = output_root.join(format!("{stem}.pdf"));
        let manifest_path = output_root.join(format!("{stem}.json"));
        let diagnostics_path = output_root.join(format!("{stem}.diagnostics.json"));

        let config = effective_config();
        let package = validated_package("");
        let admitted = AdmittedResourceResolver::new(&package.package().resources, config.limits())
            .unwrap()
            .finish()
            .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let pdf = serialized_pdf_for(&package, &admitted, &pagination, &config);
        let execution = BuildExecutionContext::from_cli_token(
            output_path.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            Some(HostPath::new(diagnostics_path).unwrap()),
            ReplacePolicy::Replace,
        )
        .unwrap();
        let output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let publication =
            ManifestPublicationContext::new(&config, &output, shaper_identity(), &tables())
                .unwrap();
        let prepared = publication
            .prepare_built(&package, admitted.token(), &pagination, pdf)
            .unwrap();

        let staged = output.stage_prepared_built(prepared).unwrap();
        assert!(!output_path.exists());
        assert!(!manifest_path.exists());
        let pending = staged.commit_pdf().unwrap();
        assert!(pending.pdf_receipt().bytes() > 0);
        assert!(output_path.exists());
        assert!(!manifest_path.exists());
        // A diagnostics publisher would drop this capability on error.
        drop(pending);
        assert!(!manifest_path.exists());

        fs::remove_file(output_path).unwrap();
    }

    #[test]
    fn built_preflight_closes_package_resources_pagination_and_output_facts() {
        let publication = publication();
        let package = validated_package("entry");
        let admitted =
            AdmittedResourceResolver::new(&package.package().resources, publication.limits())
                .unwrap()
                .finish()
                .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let output = PreparedPdfOutputFacts {
            selected_fingerprint: pagination.final_fingerprint(),
            sink: publication.output_sink(),
            bytes: 4,
            sha256: [7; 32],
            page_count: 1,
            pdf_object_count: 3,
        };

        let built = prepare_built_manifest(
            &publication,
            &package,
            admitted.token(),
            &pagination,
            output,
        )
        .unwrap();
        assert_eq!(built.manifest().status(), BuildStatus::Built);
        assert_eq!(built.manifest().inputs().len(), 1);
        assert_eq!(
            built.manifest().layout().unwrap().final_fingerprint(),
            pagination.final_fingerprint()
        );
        assert_eq!(built.manifest().output().unwrap().page_count(), 1);
        assert_eq!(built.manifest().output().unwrap().pdf_object_count(), 3);

        let other_package = validated_package("different document");
        assert_eq!(
            prepare_built_manifest(
                &publication,
                &other_package,
                admitted.token(),
                &pagination,
                output,
            ),
            Err(BuildManifestError::PackagePaginationMismatch)
        );

        let mut wrong_output = output;
        wrong_output.page_count = 2;
        assert_eq!(
            prepare_built_manifest(
                &publication,
                &package,
                admitted.token(),
                &pagination,
                wrong_output,
            ),
            Err(BuildManifestError::PaginationReceiptMismatch)
        );
    }

    #[test]
    fn failed_layout_requires_complete_package_and_resource_identity() {
        let publication = publication();
        let package = validated_package("entry");
        let admitted =
            AdmittedResourceResolver::new(&package.package().resources, publication.limits())
                .unwrap()
                .finish()
                .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let ledger = publication.begin_admission_ledger();
        assert_eq!(
            ValidatedBuildManifest::failed(&publication, &ledger, Some(&pagination)),
            Err(BuildManifestError::IncompleteLayoutAdmission)
        );

        let mut complete = publication.begin_admission_ledger();
        complete.admit_validated_package_sources(&package).unwrap();
        complete.admit_resources(admitted.token()).unwrap();
        let failed =
            ValidatedBuildManifest::failed(&publication, &complete, Some(&pagination)).unwrap();
        assert!(failed.manifest().layout().is_some());
    }

    #[test]
    fn layout_record_selects_only_a_materialized_state() {
        let pass_count = NonZeroU16::new(2).unwrap();
        let selected = NonZeroU16::new(2).unwrap();
        let record = LayoutRecord::new(
            LayoutStatus::Converged,
            pass_count,
            selected,
            LayoutStateFingerprint::from_untrusted_bytes([1; 32]),
        )
        .unwrap();
        assert_eq!(record.selected_state(), selected);
        assert_eq!(record.fallback_policy(), None);

        let fallback = LayoutRecord::new(
            LayoutStatus::MaxPassFallback,
            pass_count,
            selected,
            LayoutStateFingerprint::from_untrusted_bytes([1; 32]),
        )
        .unwrap();
        assert_eq!(
            fallback.fallback_policy(),
            Some(LayoutFallbackPolicy::LowestCostThenEarliest)
        );

        let out_of_range = NonZeroU16::new(3).unwrap();
        assert!(LayoutRecord::new(
            LayoutStatus::MaxPassFallback,
            pass_count,
            out_of_range,
            LayoutStateFingerprint::from_untrusted_bytes([2; 32]),
        )
        .is_none());
        assert!(LayoutRecord::new(
            LayoutStatus::Converged,
            pass_count,
            NonZeroU16::new(1).unwrap(),
            LayoutStateFingerprint::from_untrusted_bytes([2; 32]),
        )
        .is_none());
    }

    #[test]
    fn validated_manifest_enforces_determinism_and_status_shape() {
        assert!(validate(failed_manifest()).is_ok());

        let mut failed_before_entry_admission = failed_manifest();
        failed_before_entry_admission.inputs.clear();
        assert!(validate(failed_before_entry_admission).is_ok());

        let mut nondeterministic = failed_manifest();
        nondeterministic.deterministic = false;
        assert_eq!(
            validate(nondeterministic),
            Err(BuildManifestError::NonDeterministic)
        );

        let mut built_without_artifacts = failed_manifest();
        built_without_artifacts.status = BuildStatus::Built;
        assert_eq!(
            validate(built_without_artifacts),
            Err(BuildManifestError::BuiltRequiresLayoutAndOutput)
        );

        let mut failed_with_output = failed_manifest();
        failed_with_output.output = Some(OutputRecord {
            sink: OutputSink::File,
            bytes: 1,
            sha256: [1; 32],
            page_count: 1,
            pdf_object_count: 1,
        });
        assert_eq!(
            validate(failed_with_output),
            Err(BuildManifestError::NonBuiltMustNotHaveOutput)
        );
    }

    #[test]
    fn manifest_requires_canonical_records_and_nonempty_built_output() {
        let mut unsorted = failed_manifest();
        unsorted.inputs = vec![
            FileRecord {
                uri: PortablePath::new("b.tsf").unwrap(),
                bytes: 1,
                sha256: [1; 32],
            },
            FileRecord {
                uri: PortablePath::new("a.tsf").unwrap(),
                bytes: 1,
                sha256: [2; 32],
            },
        ];
        assert_eq!(
            validate(unsorted),
            Err(BuildManifestError::NonCanonicalInputs)
        );

        let mut empty_built = failed_manifest();
        empty_built.status = BuildStatus::Built;
        empty_built.layout = LayoutRecord::new(
            LayoutStatus::Converged,
            NonZeroU16::new(1).unwrap(),
            NonZeroU16::new(1).unwrap(),
            LayoutStateFingerprint::from_untrusted_bytes([1; 32]),
        );
        empty_built.output = Some(OutputRecord {
            sink: OutputSink::File,
            bytes: 0,
            sha256: [0; 32],
            page_count: 0,
            pdf_object_count: 0,
        });
        assert_eq!(
            validate(empty_built),
            Err(BuildManifestError::EmptyBuiltOutput)
        );
    }

    #[test]
    fn package_input_bytes_use_the_effective_inclusive_limit() {
        let config = effective_config();
        let publication = machine_publication_for(&config);
        let mut manifest = failed_manifest_for(&config);
        manifest.input_profile = BuildInputProfile::MachinePdfParagraph1;
        manifest.package_input = Some(PackageInputRecord {
            uri: PortablePath::new("document-package.json").unwrap(),
            bytes: config.limits().get().max_document_package_bytes,
            sha256: [0; 32],
            contract: None,
            canonical_sha256: None,
            profile_receipt_sha256: None,
        });
        let canonical = canonical_manifest_json(&manifest);
        assert!(canonical.contains("\"input_profile\":\"typaxis.machine-pdf/paragraph-1\""));
        assert!(canonical.contains(&format!(
            "\"package_input\":{{\"bytes\":{},\"canonical_sha256\":null,\"contract\":null,\"profile_receipt_sha256\":null,\"sha256\":\"{}\",\"uri\":\"document-package.json\"}}",
            config.limits().get().max_document_package_bytes,
            "0".repeat(64),
        )));
        assert!(validate_against(manifest.clone(), &publication).is_ok());

        manifest.package_input.as_mut().unwrap().bytes += 1;
        assert_eq!(
            validate_against(manifest, &publication),
            Err(BuildManifestError::PackageInputBytesLimit)
        );
    }

    #[test]
    fn publication_context_exists_only_after_config_and_manifest_target() {
        let config = effective_config();
        let without_target = BuildExecutionContext::from_cli_token(
            OsStr::new("-"),
            None,
            None,
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let without_target = BuildOutputCommitContext::new(&config, &without_target).unwrap();
        assert!(!without_target.manifest_requested());
        let _manifest_free_commit_api: fn(
            BuildOutputCommitContext,
            VerifiedPdfBytesReceipt,
        )
            -> Result<PdfSinkCommitReceipt, PdfSinkCommitError> =
            BuildOutputCommitContext::commit_pdf_without_manifest;
        assert_eq!(
            ManifestPublicationContext::new(&config, &without_target, shaper_identity(), &tables(),),
            Err(ManifestPublicationError::MissingManifestTarget)
        );
        let context = publication();
        assert_eq!(context.config_fingerprint(), config.fingerprint());

        let mut forged = failed_manifest();
        forged.config_sha256 = [9; 32];
        assert_eq!(
            validate(forged),
            Err(BuildManifestError::ConfigFingerprintMismatch)
        );
    }

    fn manifest_limit_config() -> EffectiveConfig {
        let limits = ResourceLimits {
            max_input_bytes: 6,
            max_source_bytes: 3,
            max_font_bytes: 4,
            max_fonts: 2,
            max_image_bytes: 4,
            max_images: 2,
            max_resource_bytes: 15,
            max_image_pixels: 1,
            max_decoded_image_bytes: 4,
            max_pages: 2,
            max_pdf_objects: 2,
            max_output_bytes: 6,
            ..ResourceLimits::default()
        };
        effective_config_with_limits(limits)
    }

    fn built_manifest_at_limits(config: &EffectiveConfig) -> BuildManifest {
        let mut manifest = failed_manifest_for(config);
        manifest.status = BuildStatus::Built;
        manifest.inputs = vec![
            FileRecord {
                uri: PortablePath::new("a.tsf").unwrap(),
                bytes: 3,
                sha256: [1; 32],
            },
            FileRecord {
                uri: PortablePath::new("b.tsf").unwrap(),
                bytes: 3,
                sha256: [2; 32],
            },
        ];
        manifest.fonts = vec![
            FontRecord {
                font_face_id: FontFaceId::new(0),
                uri: PortablePath::new("fonts/a.ttf").unwrap(),
                face_index: 0,
                bytes: 4,
                sha256: [3; 32],
                units_per_em: 1000,
                glyph_count: 1,
            },
            FontRecord {
                font_face_id: FontFaceId::new(1),
                uri: PortablePath::new("fonts/b.ttf").unwrap(),
                face_index: 0,
                bytes: 4,
                sha256: [4; 32],
                units_per_em: 1000,
                glyph_count: 1,
            },
        ];
        manifest.images = vec![
            ImageRecord {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("images/a.png").unwrap(),
                bytes: 4,
                sha256: [5; 32],
                attested_media_kind: "png",
                pixel_width: 1,
                pixel_height: 1,
                decoded_bytes: 4,
            },
            ImageRecord {
                image_id: ImageResourceId::new(1),
                uri: PortablePath::new("images/b.png").unwrap(),
                bytes: 3,
                sha256: [6; 32],
                attested_media_kind: "png",
                pixel_width: 1,
                pixel_height: 1,
                decoded_bytes: 4,
            },
        ];
        manifest.layout = LayoutRecord::new(
            LayoutStatus::Converged,
            NonZeroU16::new(1).unwrap(),
            NonZeroU16::new(1).unwrap(),
            LayoutStateFingerprint::from_untrusted_bytes([7; 32]),
        );
        manifest.output = Some(OutputRecord {
            sink: OutputSink::Stdout,
            bytes: 6,
            sha256: [8; 32],
            page_count: 2,
            pdf_object_count: 2,
        });
        manifest
    }

    #[test]
    fn manifest_actual_facts_accept_exact_limits_and_reject_max_plus_one() {
        let config = manifest_limit_config();
        let publication = publication_for(&config);
        let exact = built_manifest_at_limits(&config);
        assert!(validate_against(exact.clone(), &publication).is_ok());
        assert_eq!(publication.limits(), config.limits());

        let mut source = exact.clone();
        source.inputs[0].bytes = 4;
        assert_eq!(
            validate_against(source, &publication),
            Err(BuildManifestError::InputSourceLimit)
        );

        let mut input_aggregate = exact.clone();
        input_aggregate.inputs.push(FileRecord {
            uri: PortablePath::new("c.tsf").unwrap(),
            bytes: 1,
            sha256: [9; 32],
        });
        assert_eq!(
            validate_against(input_aggregate, &publication),
            Err(BuildManifestError::InputAggregateLimit)
        );

        let mut font_count = exact.clone();
        font_count.fonts.push(FontRecord {
            font_face_id: FontFaceId::new(2),
            uri: PortablePath::new("fonts/c.ttf").unwrap(),
            face_index: 0,
            bytes: 1,
            sha256: [10; 32],
            units_per_em: 1000,
            glyph_count: 1,
        });
        assert_eq!(
            validate_against(font_count, &publication),
            Err(BuildManifestError::FontCountLimit)
        );

        let mut font_bytes = exact.clone();
        font_bytes.fonts[0].bytes = 5;
        assert_eq!(
            validate_against(font_bytes, &publication),
            Err(BuildManifestError::FontBytesLimit)
        );

        let mut image_count = exact.clone();
        image_count.images.push(ImageRecord {
            image_id: ImageResourceId::new(2),
            uri: PortablePath::new("images/c.png").unwrap(),
            bytes: 1,
            sha256: [11; 32],
            attested_media_kind: "png",
            pixel_width: 1,
            pixel_height: 1,
            decoded_bytes: 1,
        });
        assert_eq!(
            validate_against(image_count, &publication),
            Err(BuildManifestError::ImageCountLimit)
        );

        let mut image_bytes = exact.clone();
        image_bytes.images[0].bytes = 5;
        assert_eq!(
            validate_against(image_bytes, &publication),
            Err(BuildManifestError::ImageBytesLimit)
        );

        let mut aggregate = exact.clone();
        aggregate.images[1].bytes = 4;
        assert_eq!(
            validate_against(aggregate, &publication),
            Err(BuildManifestError::ResourceAggregateLimit)
        );

        let mut pages = exact.clone();
        pages.output.as_mut().unwrap().page_count = 3;
        assert_eq!(
            validate_against(pages, &publication),
            Err(BuildManifestError::PageLimit)
        );

        let mut output_bytes = exact.clone();
        output_bytes.output.as_mut().unwrap().bytes = 7;
        assert_eq!(
            validate_against(output_bytes, &publication),
            Err(BuildManifestError::OutputBytesLimit)
        );

        let mut objects = exact;
        objects.output.as_mut().unwrap().pdf_object_count = 3;
        assert_eq!(
            validate_against(objects, &publication),
            Err(BuildManifestError::PdfObjectLimit)
        );
    }

    #[test]
    fn failed_manifest_partial_facts_are_checked_against_limits() {
        let config = manifest_limit_config();
        let publication = publication_for(&config);
        let mut partial = built_manifest_at_limits(&config);
        partial.status = BuildStatus::Failed;
        partial.layout = None;
        partial.output = None;
        assert!(validate_against(partial.clone(), &publication).is_ok());

        partial.inputs.push(FileRecord {
            uri: PortablePath::new("c.tsf").unwrap(),
            bytes: 1,
            sha256: [9; 32],
        });
        assert_eq!(
            validate_against(partial, &publication),
            Err(BuildManifestError::InputAggregateLimit)
        );
    }
}
