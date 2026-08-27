#![forbid(unsafe_code)]

use core::num::NonZeroU16;
use std::collections::BTreeSet;
use typaxis_core::{
    sha256, AdmittedResourceFingerprint, DocumentFingerprint, FontFaceId, FontInstanceId,
    FootnoteId, GeneratedBufferKey, GenerationKind, Length, NodeId, NonNegativeLength,
    PositiveLength, ReferenceFingerprint, StyleFingerprint, TextSpan, Utf8ByteOffset,
};
use typaxis_resource_admission::{
    AdmittedFontInstanceRef, AdmittedFontInstanceTable, AdmittedResourceLedgerToken,
    ResourceAdmissionError,
};
use typaxis_style::{ResolvedTextStyle, StyleValidationError};
use typaxis_syntax::{
    PackageComputedStyle, PackageGeneratedTextBinding, PackageParagraphTextSite,
    PackageShapeTextError, PackageStyleError, ValidatedMachinePackage, ValidatedParsedPackage,
};

/// Exact identity of all validated inputs that can affect one layout state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LayoutEpoch {
    document: DocumentFingerprint,
    style: StyleFingerprint,
    admitted_resources: AdmittedResourceFingerprint,
    references: ReferenceFingerprint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutEpochError {
    AdmittedResourceDocumentMismatch,
    PackageEpochMismatch,
}

impl LayoutEpoch {
    pub fn from_validated_inputs(
        generated_text: PackageGeneratedTextBinding<'_>,
        admitted_resources: AdmittedResourceLedgerToken<'_>,
    ) -> Result<Self, LayoutEpochError> {
        let package = generated_text.package();
        if !admitted_resources
            .ledger()
            .matches_declarations(&package.package().resources)
        {
            return Err(LayoutEpochError::AdmittedResourceDocumentMismatch);
        }
        Ok(Self {
            document: package.epoch_identity().document(),
            style: package.epoch_identity().style(),
            admitted_resources: admitted_resources.fingerprint(),
            references: generated_text.generated_text().reference_fingerprint(),
        })
    }

    pub const fn document(self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style(self) -> StyleFingerprint {
        self.style
    }
    pub const fn admitted_resources(self) -> AdmittedResourceFingerprint {
        self.admitted_resources
    }
    pub const fn references(self) -> ReferenceFingerprint {
        self.references
    }

    /// Reissues the epoch for the next pagination pass while preserving the
    /// stable document, style, and admitted-resource identities. The new
    /// reference identity is accepted only from the package-owned generated
    /// text validation boundary.
    pub fn with_generated_text(
        self,
        generated_text: PackageGeneratedTextBinding<'_>,
    ) -> Result<Self, LayoutEpochError> {
        let package = generated_text.package();
        if self.document != package.epoch_identity().document()
            || self.style != package.epoch_identity().style()
        {
            return Err(LayoutEpochError::PackageEpochMismatch);
        }
        Ok(Self {
            document: self.document,
            style: self.style,
            admitted_resources: self.admitted_resources,
            references: generated_text.generated_text().reference_fingerprint(),
        })
    }

    /// Returns whether two states share every pagination input other than the
    /// generated reference overlay.
    pub fn same_stable_inputs(self, other: Self) -> bool {
        self.document == other.document
            && self.style == other.style
            && self.admitted_resources == other.admitted_resources
    }
}

/// Dense identity of one independently progressing layout flow.
///
/// `0` is always the document body. Remaining IDs are allocated by the
/// production registry in typed Document preorder; accepting a `FlowId` from
/// a caller never constitutes registry admission.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FlowId(u32);

impl FlowId {
    pub const DOCUMENT_BODY: Self = Self(0);

    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Dense identity of one footnote-definition flow.
///
/// This is deliberately a different nominal type and namespace from
/// [`FlowId`]. The document body remains `FlowId::DOCUMENT_BODY`; footnote
/// definitions start at `FootnoteFlowId(0)` in canonical `FootnoteId` order
/// and can never be confused with a body, list, caption, or table-cell cursor.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FootnoteFlowId(u32);

impl FootnoteFlowId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Exclusive terminal fragment ordinal for one measured footnote flow.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FootnoteFlowTerminal(u32);

impl FootnoteFlowTerminal {
    pub const fn new(fragment_count: u32) -> Self {
        Self(fragment_count)
    }

    pub const fn fragment_count(self) -> u32 {
        self.0
    }
}

/// Closed set of implemented flow owners. `TableCell` is private until the
/// table publication gate; no unknown string is interpreted as a flow kind.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FlowOwnerKind {
    DocumentBody,
    ListItem,
    FigureCaption,
    TableCell,
}

impl FlowOwnerKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DocumentBody => "document_body",
            Self::ListItem => "list_item",
            Self::FigureCaption => "figure_caption",
            Self::TableCell => "table_cell",
        }
    }
}

/// Exhaustive content kinds accepted by the canonical flow registry. A table
/// row owns its body-flow boundary and binds zero or more cell subflows.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FlowContentKind {
    Paragraph,
    ListItem,
    FigureCaption,
    PageBreak,
    TableRow,
}

impl FlowContentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Paragraph => "paragraph",
            Self::ListItem => "list_item",
            Self::FigureCaption => "figure_caption",
            Self::PageBreak => "page_break",
            Self::TableRow => "table_row",
        }
    }
}

/// Owner-local ordinal of the terminal position for one flow. A value of
/// zero is the terminal of an empty flow.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FlowTerminal(u32);

impl FlowTerminal {
    pub const fn new(owner_local_ordinal: u32) -> Self {
        Self(owner_local_ordinal)
    }

    pub const fn owner_local_ordinal(self) -> u32 {
        self.0
    }
}

macro_rules! flow_fingerprint_type {
    ($name:ident, $algorithm:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 32]);

        impl $name {
            pub const ALGORITHM_ID: &'static str = $algorithm;

            pub const fn bytes(self) -> [u8; 32] {
                self.0
            }

            fn from_canonical_jcs(canonical_jcs: &str) -> Self {
                debug_assert!(canonical_jcs.contains(Self::ALGORITHM_ID));
                Self(sha256(canonical_jcs.as_bytes()))
            }
        }
    };
}

flow_fingerprint_type!(FlowRegistryFingerprint, "typaxis.basic-flow-registry/1");
flow_fingerprint_type!(
    MultiFlowSelectedStateFingerprint,
    "typaxis.multi-flow-selected-state/1"
);
flow_fingerprint_type!(TableGridFingerprint, "typaxis.table-grid-receipt/1");
flow_fingerprint_type!(
    TableSelectedLayoutFingerprint,
    "typaxis.table-selected-layout/1"
);
flow_fingerprint_type!(
    FootnoteProfileFingerprint,
    "typaxis.footnote-profile-receipt/1"
);
flow_fingerprint_type!(
    FootnoteFlowRegistryFingerprint,
    "typaxis.footnote-flow-registry/1"
);
flow_fingerprint_type!(
    FootnotePageEvaluationFingerprint,
    "typaxis.footnote-page-evaluation/1"
);
flow_fingerprint_type!(
    FootnoteSelectedLayoutFingerprint,
    "typaxis.footnote-selected-layout/1"
);

pub fn flow_registry_fingerprint_from_jcs(canonical_jcs: &str) -> FlowRegistryFingerprint {
    FlowRegistryFingerprint::from_canonical_jcs(canonical_jcs)
}

pub fn multi_flow_selected_state_fingerprint_from_jcs(
    canonical_jcs: &str,
) -> MultiFlowSelectedStateFingerprint {
    MultiFlowSelectedStateFingerprint::from_canonical_jcs(canonical_jcs)
}

pub fn table_selected_layout_fingerprint_from_jcs(
    canonical_jcs: &str,
) -> TableSelectedLayoutFingerprint {
    TableSelectedLayoutFingerprint::from_canonical_jcs(canonical_jcs)
}

pub fn footnote_profile_fingerprint_from_jcs(canonical_jcs: &str) -> FootnoteProfileFingerprint {
    FootnoteProfileFingerprint::from_canonical_jcs(canonical_jcs)
}

pub fn footnote_flow_registry_fingerprint_from_jcs(
    canonical_jcs: &str,
) -> FootnoteFlowRegistryFingerprint {
    FootnoteFlowRegistryFingerprint::from_canonical_jcs(canonical_jcs)
}

pub fn footnote_page_evaluation_fingerprint_from_jcs(
    canonical_jcs: &str,
) -> FootnotePageEvaluationFingerprint {
    FootnotePageEvaluationFingerprint::from_canonical_jcs(canonical_jcs)
}

pub fn footnote_selected_layout_fingerprint_from_jcs(
    canonical_jcs: &str,
) -> FootnoteSelectedLayoutFingerprint {
    FootnoteSelectedLayoutFingerprint::from_canonical_jcs(canonical_jcs)
}

/// Canonical definition binding shared by layout and pagination receipts.
/// The constructor is intentionally small: registry admission and dense-ID
/// validation remain the responsibility of the package-derived layout owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FootnoteFlowBinding {
    footnote_id: FootnoteId,
    flow_id: FootnoteFlowId,
    definition_owner: NodeId,
    terminal: FootnoteFlowTerminal,
}

impl FootnoteFlowBinding {
    pub const fn new(
        footnote_id: FootnoteId,
        flow_id: FootnoteFlowId,
        definition_owner: NodeId,
        terminal: FootnoteFlowTerminal,
    ) -> Self {
        Self {
            footnote_id,
            flow_id,
            definition_owner,
            terminal,
        }
    }

    pub const fn footnote_id(&self) -> &FootnoteId {
        &self.footnote_id
    }

    pub const fn flow_id(&self) -> FootnoteFlowId {
        self.flow_id
    }

    pub const fn definition_owner(&self) -> NodeId {
        self.definition_owner
    }

    pub const fn terminal(&self) -> FootnoteFlowTerminal {
        self.terminal
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TableSection {
    Head,
    Body,
}

impl TableSection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Body => "body",
        }
    }

    const fn order(self) -> u8 {
        match self {
            Self::Head => 0,
            Self::Body => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolvedTableColumnInput {
    Fixed(PositiveLength),
    Fraction(NonZeroU16),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedTableColumn {
    index: u32,
    input: ResolvedTableColumnInput,
    rounded_fraction_width: Option<NonNegativeLength>,
    final_width: PositiveLength,
}

impl ResolvedTableColumn {
    pub const fn fixed(index: u32, width: PositiveLength) -> Self {
        Self {
            index,
            input: ResolvedTableColumnInput::Fixed(width),
            rounded_fraction_width: None,
            final_width: width,
        }
    }

    pub const fn fraction(
        index: u32,
        weight: NonZeroU16,
        rounded_fraction_width: NonNegativeLength,
        final_width: PositiveLength,
    ) -> Self {
        Self {
            index,
            input: ResolvedTableColumnInput::Fraction(weight),
            rounded_fraction_width: Some(rounded_fraction_width),
            final_width,
        }
    }

    pub const fn index(&self) -> u32 {
        self.index
    }

    pub const fn input(&self) -> ResolvedTableColumnInput {
        self.input
    }

    pub const fn rounded_fraction_width(&self) -> Option<NonNegativeLength> {
        self.rounded_fraction_width
    }

    pub const fn final_width(&self) -> PositiveLength {
        self.final_width
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedTableRowBinding {
    row_owner: NodeId,
    section: TableSection,
    row_ordinal: u32,
}

impl ValidatedTableRowBinding {
    pub const fn new(row_owner: NodeId, section: TableSection, row_ordinal: u32) -> Self {
        Self {
            row_owner,
            section,
            row_ordinal,
        }
    }

    pub const fn row_owner(&self) -> NodeId {
        self.row_owner
    }

    pub const fn section(&self) -> TableSection {
        self.section
    }

    pub const fn row_ordinal(&self) -> u32 {
        self.row_ordinal
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedTableCellBinding {
    cell_owner: NodeId,
    row_owner: NodeId,
    section: TableSection,
    row_ordinal: u32,
    column_ordinal: u32,
    colspan: NonZeroU16,
    rowspan: NonZeroU16,
    flow_id: FlowId,
    terminal: FlowTerminal,
    frame_inline_start: NonNegativeLength,
    frame_inline_size: PositiveLength,
}

impl ValidatedTableCellBinding {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        cell_owner: NodeId,
        row_owner: NodeId,
        section: TableSection,
        row_ordinal: u32,
        column_ordinal: u32,
        colspan: NonZeroU16,
        rowspan: NonZeroU16,
        flow_id: FlowId,
        terminal: FlowTerminal,
        frame_inline_start: NonNegativeLength,
        frame_inline_size: PositiveLength,
    ) -> Self {
        Self {
            cell_owner,
            row_owner,
            section,
            row_ordinal,
            column_ordinal,
            colspan,
            rowspan,
            flow_id,
            terminal,
            frame_inline_start,
            frame_inline_size,
        }
    }

    pub const fn cell_owner(&self) -> NodeId {
        self.cell_owner
    }
    pub const fn row_owner(&self) -> NodeId {
        self.row_owner
    }
    pub const fn section(&self) -> TableSection {
        self.section
    }
    pub const fn row_ordinal(&self) -> u32 {
        self.row_ordinal
    }
    pub const fn column_ordinal(&self) -> u32 {
        self.column_ordinal
    }
    pub const fn colspan(&self) -> NonZeroU16 {
        self.colspan
    }
    pub const fn rowspan(&self) -> NonZeroU16 {
        self.rowspan
    }
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }
    pub const fn terminal(&self) -> FlowTerminal {
        self.terminal
    }
    pub const fn frame_inline_start(&self) -> NonNegativeLength {
        self.frame_inline_start
    }
    pub const fn frame_inline_size(&self) -> PositiveLength {
        self.frame_inline_size
    }
    pub const fn padding_before(&self) -> NonNegativeLength {
        NonNegativeLength::ZERO
    }
    pub const fn padding_after(&self) -> NonNegativeLength {
        NonNegativeLength::ZERO
    }
    pub const fn padding_start(&self) -> NonNegativeLength {
        NonNegativeLength::ZERO
    }
    pub const fn padding_end(&self) -> NonNegativeLength {
        NonNegativeLength::ZERO
    }
    pub const fn vertical_alignment(&self) -> TableVerticalAlignment {
        TableVerticalAlignment::BlockStart
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableVerticalAlignment {
    BlockStart,
}

#[derive(Debug)]
pub struct TableGridReceiptInput {
    pub package_sha256: [u8; 32],
    pub epoch: LayoutEpoch,
    pub flow_registry: FlowRegistryFingerprint,
    pub table_owner: NodeId,
    pub containing_flow_id: FlowId,
    pub frame_inline_size: PositiveLength,
    pub available_inline_size: PositiveLength,
    pub start_indent: NonNegativeLength,
    pub end_indent: NonNegativeLength,
    pub space_before: NonNegativeLength,
    pub space_after: NonNegativeLength,
    pub keep_with_next: bool,
    pub columns: Vec<ResolvedTableColumn>,
    pub rounding_residual: Length,
    pub residual_recipient: Option<u32>,
    pub rows: Vec<ValidatedTableRowBinding>,
    pub cells: Vec<ValidatedTableCellBinding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableGridReceiptError {
    EmptyColumns,
    EmptyRows,
    NonDenseColumnIndex(u32),
    InvalidColumnResolution(u32),
    InvalidResidualRecipient,
    WidthSumMismatch,
    FrameWidthMismatch,
    NonCanonicalRow(NodeId),
    DuplicateRowOwner(NodeId),
    NonCanonicalCell(NodeId),
    DuplicateCellOwner(NodeId),
    DuplicateCellFlow(FlowId),
    WrongRowOwner(NodeId),
    CellOutOfRange(NodeId),
    CellOverlap(NodeId),
    RowHole(NodeId),
    RowspanOutOfRange(NodeId),
    NonCanonicalCellFlow(NodeId),
    ArithmeticOverflow,
}

/// Package/epoch-bound proof of canonical column resolution, dense grid
/// ownership, exact zero-padding frames, and one independent flow per cell.
/// Construction rederives all structural facts; callers cannot bless a set of
/// widths or origins merely by wrapping it in this type.
#[derive(Debug)]
pub struct ValidatedTableGridReceipt {
    package_sha256: [u8; 32],
    epoch: LayoutEpoch,
    flow_registry: FlowRegistryFingerprint,
    table_owner: NodeId,
    containing_flow_id: FlowId,
    frame_inline_size: PositiveLength,
    available_inline_size: PositiveLength,
    start_indent: NonNegativeLength,
    end_indent: NonNegativeLength,
    space_before: NonNegativeLength,
    space_after: NonNegativeLength,
    keep_with_next: bool,
    columns: Vec<ResolvedTableColumn>,
    rounding_residual: Length,
    residual_recipient: Option<u32>,
    rows: Vec<ValidatedTableRowBinding>,
    cells: Vec<ValidatedTableCellBinding>,
    fingerprint: TableGridFingerprint,
}

impl ValidatedTableGridReceipt {
    pub fn new(input: TableGridReceiptInput) -> Result<Self, TableGridReceiptError> {
        validate_table_columns(&input)?;
        validate_table_rows_and_cells(&input)?;
        let fingerprint = TableGridFingerprint::from_canonical_jcs(&encode_table_grid_jcs(&input));
        Ok(Self {
            package_sha256: input.package_sha256,
            epoch: input.epoch,
            flow_registry: input.flow_registry,
            table_owner: input.table_owner,
            containing_flow_id: input.containing_flow_id,
            frame_inline_size: input.frame_inline_size,
            available_inline_size: input.available_inline_size,
            start_indent: input.start_indent,
            end_indent: input.end_indent,
            space_before: input.space_before,
            space_after: input.space_after,
            keep_with_next: input.keep_with_next,
            columns: input.columns,
            rounding_residual: input.rounding_residual,
            residual_recipient: input.residual_recipient,
            rows: input.rows,
            cells: input.cells,
            fingerprint,
        })
    }

    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn flow_registry(&self) -> FlowRegistryFingerprint {
        self.flow_registry
    }
    pub const fn table_owner(&self) -> NodeId {
        self.table_owner
    }
    pub const fn containing_flow_id(&self) -> FlowId {
        self.containing_flow_id
    }
    pub const fn frame_inline_size(&self) -> PositiveLength {
        self.frame_inline_size
    }
    pub const fn available_inline_size(&self) -> PositiveLength {
        self.available_inline_size
    }
    pub const fn start_indent(&self) -> NonNegativeLength {
        self.start_indent
    }
    pub const fn end_indent(&self) -> NonNegativeLength {
        self.end_indent
    }
    pub const fn space_before(&self) -> NonNegativeLength {
        self.space_before
    }
    pub const fn space_after(&self) -> NonNegativeLength {
        self.space_after
    }
    pub const fn keep_with_next(&self) -> bool {
        self.keep_with_next
    }
    pub fn columns(&self) -> &[ResolvedTableColumn] {
        &self.columns
    }
    pub const fn rounding_residual(&self) -> Length {
        self.rounding_residual
    }
    pub const fn residual_recipient(&self) -> Option<u32> {
        self.residual_recipient
    }
    pub fn rows(&self) -> &[ValidatedTableRowBinding] {
        &self.rows
    }
    pub fn cells(&self) -> &[ValidatedTableCellBinding] {
        &self.cells
    }
    pub const fn fingerprint(&self) -> TableGridFingerprint {
        self.fingerprint
    }
}

fn validate_table_columns(input: &TableGridReceiptInput) -> Result<(), TableGridReceiptError> {
    if input.columns.is_empty() {
        return Err(TableGridReceiptError::EmptyColumns);
    }
    let frame_total = i128::from(input.start_indent.get().raw())
        .checked_add(i128::from(input.available_inline_size.get().raw()))
        .and_then(|value| value.checked_add(i128::from(input.end_indent.get().raw())))
        .ok_or(TableGridReceiptError::ArithmeticOverflow)?;
    if frame_total != i128::from(input.frame_inline_size.get().raw()) {
        return Err(TableGridReceiptError::FrameWidthMismatch);
    }

    let mut fixed_sum = 0i128;
    let mut weight_sum = 0i128;
    let mut last_fraction = None;
    for (expected, column) in input.columns.iter().enumerate() {
        let expected =
            u32::try_from(expected).map_err(|_| TableGridReceiptError::ArithmeticOverflow)?;
        if column.index != expected {
            return Err(TableGridReceiptError::NonDenseColumnIndex(column.index));
        }
        match column.input {
            ResolvedTableColumnInput::Fixed(width) => {
                if column.rounded_fraction_width.is_some() || column.final_width != width {
                    return Err(TableGridReceiptError::InvalidColumnResolution(column.index));
                }
                fixed_sum = fixed_sum
                    .checked_add(i128::from(width.get().raw()))
                    .ok_or(TableGridReceiptError::ArithmeticOverflow)?;
            }
            ResolvedTableColumnInput::Fraction(weight) => {
                if column.rounded_fraction_width.is_none() {
                    return Err(TableGridReceiptError::InvalidColumnResolution(column.index));
                }
                weight_sum = weight_sum
                    .checked_add(i128::from(weight.get()))
                    .ok_or(TableGridReceiptError::ArithmeticOverflow)?;
                last_fraction = Some(column.index);
            }
        }
    }
    let available = i128::from(input.available_inline_size.get().raw());
    let remaining = available
        .checked_sub(fixed_sum)
        .ok_or(TableGridReceiptError::ArithmeticOverflow)?;
    if remaining < 0 {
        return Err(TableGridReceiptError::WidthSumMismatch);
    }

    let expected_residual = if weight_sum == 0 {
        if remaining != 0
            || input.residual_recipient.is_some()
            || input.rounding_residual != Length::ZERO
        {
            return Err(TableGridReceiptError::InvalidResidualRecipient);
        }
        0
    } else {
        if remaining <= 0 || input.residual_recipient != last_fraction {
            return Err(TableGridReceiptError::InvalidResidualRecipient);
        }
        let mut rounded_sum = 0i128;
        for column in &input.columns {
            if let ResolvedTableColumnInput::Fraction(weight) = column.input {
                let numerator = remaining
                    .checked_mul(i128::from(weight.get()))
                    .ok_or(TableGridReceiptError::ArithmeticOverflow)?;
                let rounded = round_ratio_ties_even(numerator, weight_sum)?;
                if i128::from(
                    column
                        .rounded_fraction_width
                        .expect("fraction shape checked")
                        .get()
                        .raw(),
                ) != rounded
                {
                    return Err(TableGridReceiptError::InvalidColumnResolution(column.index));
                }
                rounded_sum = rounded_sum
                    .checked_add(rounded)
                    .ok_or(TableGridReceiptError::ArithmeticOverflow)?;
            }
        }
        remaining
            .checked_sub(rounded_sum)
            .ok_or(TableGridReceiptError::ArithmeticOverflow)?
    };
    if i128::from(input.rounding_residual.raw()) != expected_residual {
        return Err(TableGridReceiptError::InvalidResidualRecipient);
    }

    let mut final_sum = 0i128;
    for column in &input.columns {
        let expected = match column.input {
            ResolvedTableColumnInput::Fixed(width) => i128::from(width.get().raw()),
            ResolvedTableColumnInput::Fraction(_) => {
                let rounded = i128::from(
                    column
                        .rounded_fraction_width
                        .expect("fraction shape checked")
                        .get()
                        .raw(),
                );
                if input.residual_recipient == Some(column.index) {
                    rounded
                        .checked_add(expected_residual)
                        .ok_or(TableGridReceiptError::ArithmeticOverflow)?
                } else {
                    rounded
                }
            }
        };
        if expected <= 0 || i128::from(column.final_width.get().raw()) != expected {
            return Err(TableGridReceiptError::InvalidColumnResolution(column.index));
        }
        final_sum = final_sum
            .checked_add(expected)
            .ok_or(TableGridReceiptError::ArithmeticOverflow)?;
    }
    if final_sum != available {
        return Err(TableGridReceiptError::WidthSumMismatch);
    }
    Ok(())
}

fn validate_table_rows_and_cells(
    input: &TableGridReceiptInput,
) -> Result<(), TableGridReceiptError> {
    if input.rows.is_empty() {
        return Err(TableGridReceiptError::EmptyRows);
    }
    let mut row_owners = BTreeSet::new();
    let mut expected_head = 0u32;
    let mut expected_body = 0u32;
    let mut body_started = false;
    for row in &input.rows {
        let expected = match row.section {
            TableSection::Head if !body_started => &mut expected_head,
            TableSection::Body => {
                body_started = true;
                &mut expected_body
            }
            TableSection::Head => {
                return Err(TableGridReceiptError::NonCanonicalRow(row.row_owner))
            }
        };
        if row.row_ordinal != *expected {
            return Err(TableGridReceiptError::NonCanonicalRow(row.row_owner));
        }
        *expected = expected
            .checked_add(1)
            .ok_or(TableGridReceiptError::ArithmeticOverflow)?;
        if !row_owners.insert(row.row_owner) {
            return Err(TableGridReceiptError::DuplicateRowOwner(row.row_owner));
        }
    }

    let mut cell_owners = BTreeSet::new();
    let mut cell_flows = BTreeSet::new();
    let mut previous_flow = None;
    for (index, cell) in input.cells.iter().enumerate() {
        if index > 0 {
            let previous = input.cells[index - 1];
            if (cell.section.order(), cell.row_ordinal, cell.column_ordinal)
                <= (
                    previous.section.order(),
                    previous.row_ordinal,
                    previous.column_ordinal,
                )
            {
                return Err(TableGridReceiptError::NonCanonicalCell(cell.cell_owner));
            }
        }
        if !cell_owners.insert(cell.cell_owner) {
            return Err(TableGridReceiptError::DuplicateCellOwner(cell.cell_owner));
        }
        if !cell_flows.insert(cell.flow_id) {
            return Err(TableGridReceiptError::DuplicateCellFlow(cell.flow_id));
        }
        if cell.flow_id == FlowId::DOCUMENT_BODY
            || previous_flow.is_some_and(|previous: FlowId| {
                previous.get().checked_add(1) != Some(cell.flow_id.get())
            })
        {
            return Err(TableGridReceiptError::NonCanonicalCellFlow(cell.cell_owner));
        }
        previous_flow = Some(cell.flow_id);
        let Some(row) = input
            .rows
            .iter()
            .find(|row| row.section == cell.section && row.row_ordinal == cell.row_ordinal)
        else {
            return Err(TableGridReceiptError::WrongRowOwner(cell.cell_owner));
        };
        if row.row_owner != cell.row_owner {
            return Err(TableGridReceiptError::WrongRowOwner(cell.cell_owner));
        }
        validate_cell_frame(&input.columns, cell)?;
    }

    validate_table_section_grid(
        &input.rows,
        &input.cells,
        TableSection::Head,
        input.columns.len(),
    )?;
    validate_table_section_grid(
        &input.rows,
        &input.cells,
        TableSection::Body,
        input.columns.len(),
    )
}

fn validate_cell_frame(
    columns: &[ResolvedTableColumn],
    cell: &ValidatedTableCellBinding,
) -> Result<(), TableGridReceiptError> {
    let start = usize::try_from(cell.column_ordinal)
        .map_err(|_| TableGridReceiptError::CellOutOfRange(cell.cell_owner))?;
    let end = start
        .checked_add(usize::from(cell.colspan.get()))
        .ok_or(TableGridReceiptError::ArithmeticOverflow)?;
    if end > columns.len() {
        return Err(TableGridReceiptError::CellOutOfRange(cell.cell_owner));
    }
    let inline_start = columns[..start].iter().try_fold(0i128, |sum, column| {
        sum.checked_add(i128::from(column.final_width.get().raw()))
            .ok_or(TableGridReceiptError::ArithmeticOverflow)
    })?;
    let inline_size = columns[start..end].iter().try_fold(0i128, |sum, column| {
        sum.checked_add(i128::from(column.final_width.get().raw()))
            .ok_or(TableGridReceiptError::ArithmeticOverflow)
    })?;
    if i128::from(cell.frame_inline_start.get().raw()) != inline_start
        || i128::from(cell.frame_inline_size.get().raw()) != inline_size
    {
        return Err(TableGridReceiptError::InvalidColumnResolution(
            cell.column_ordinal,
        ));
    }
    Ok(())
}

fn validate_table_section_grid(
    rows: &[ValidatedTableRowBinding],
    cells: &[ValidatedTableCellBinding],
    section: TableSection,
    column_count: usize,
) -> Result<(), TableGridReceiptError> {
    let section_rows: Vec<_> = rows.iter().filter(|row| row.section == section).collect();
    if section_rows.is_empty() {
        return Ok(());
    }
    let row_count =
        u32::try_from(section_rows.len()).map_err(|_| TableGridReceiptError::ArithmeticOverflow)?;
    let mut remaining = vec![0u16; column_count];
    for row in section_rows {
        for cell in cells
            .iter()
            .filter(|cell| cell.section == section && cell.row_ordinal == row.row_ordinal)
        {
            let Some(origin) = remaining.iter().position(|value| *value == 0) else {
                return Err(TableGridReceiptError::CellOutOfRange(cell.cell_owner));
            };
            if usize::try_from(cell.column_ordinal) != Ok(origin) {
                return Err(TableGridReceiptError::NonCanonicalCell(cell.cell_owner));
            }
            let end = origin
                .checked_add(usize::from(cell.colspan.get()))
                .ok_or(TableGridReceiptError::ArithmeticOverflow)?;
            if end > column_count {
                return Err(TableGridReceiptError::CellOutOfRange(cell.cell_owner));
            }
            if remaining[origin..end].iter().any(|value| *value != 0) {
                return Err(TableGridReceiptError::CellOverlap(cell.cell_owner));
            }
            if !matches!(
                row.row_ordinal
                    .checked_add(u32::from(cell.rowspan.get())),
                Some(end_row) if end_row <= row_count
            ) {
                return Err(TableGridReceiptError::RowspanOutOfRange(cell.cell_owner));
            }
            for value in &mut remaining[origin..end] {
                *value = cell.rowspan.get();
            }
        }
        if remaining.contains(&0) {
            return Err(TableGridReceiptError::RowHole(row.row_owner));
        }
        for value in &mut remaining {
            *value -= 1;
        }
    }
    if remaining.iter().any(|value| *value != 0) {
        return Err(TableGridReceiptError::RowspanOutOfRange(
            section_rows_last_owner(rows, section),
        ));
    }
    Ok(())
}

fn section_rows_last_owner(rows: &[ValidatedTableRowBinding], section: TableSection) -> NodeId {
    rows.iter()
        .rev()
        .find(|row| row.section == section)
        .map(|row| row.row_owner)
        .unwrap_or(NodeId::new(0))
}

fn round_ratio_ties_even(
    numerator: i128,
    denominator: i128,
) -> Result<i128, TableGridReceiptError> {
    if numerator < 0 || denominator <= 0 {
        return Err(TableGridReceiptError::ArithmeticOverflow);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let doubled = remainder
        .checked_mul(2)
        .ok_or(TableGridReceiptError::ArithmeticOverflow)?;
    if doubled < denominator || (doubled == denominator && quotient % 2 == 0) {
        Ok(quotient)
    } else {
        quotient
            .checked_add(1)
            .ok_or(TableGridReceiptError::ArithmeticOverflow)
    }
}

fn encode_table_grid_jcs(input: &TableGridReceiptInput) -> String {
    let mut output = String::from("{\"algorithm\":\"");
    output.push_str(TableGridFingerprint::ALGORITHM_ID);
    output.push_str("\",\"available_inline_size\":");
    output.push_str(&input.available_inline_size.get().raw().to_string());
    output.push_str(",\"cells\":[");
    for (index, cell) in input.cells.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"cell_owner\":");
        output.push_str(&cell.cell_owner.get().to_string());
        output.push_str(",\"colspan\":");
        output.push_str(&cell.colspan.get().to_string());
        output.push_str(",\"column\":");
        output.push_str(&cell.column_ordinal.to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&cell.flow_id.get().to_string());
        output.push_str(",\"frame_inline_size\":");
        output.push_str(&cell.frame_inline_size.get().raw().to_string());
        output.push_str(",\"frame_inline_start\":");
        output.push_str(&cell.frame_inline_start.get().raw().to_string());
        output.push_str(",\"row\":");
        output.push_str(&cell.row_ordinal.to_string());
        output.push_str(",\"row_owner\":");
        output.push_str(&cell.row_owner.get().to_string());
        output.push_str(",\"rowspan\":");
        output.push_str(&cell.rowspan.get().to_string());
        output.push_str(",\"section\":\"");
        output.push_str(cell.section.as_str());
        output.push_str("\",\"terminal\":");
        output.push_str(&cell.terminal.owner_local_ordinal().to_string());
        output.push('}');
    }
    output.push_str("],\"columns\":[");
    for (index, column) in input.columns.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"final_width\":");
        output.push_str(&column.final_width.get().raw().to_string());
        output.push_str(",\"index\":");
        output.push_str(&column.index.to_string());
        match column.input {
            ResolvedTableColumnInput::Fixed(width) => {
                output.push_str(",\"kind\":\"fixed\",\"value\":");
                output.push_str(&width.get().raw().to_string());
            }
            ResolvedTableColumnInput::Fraction(weight) => {
                output.push_str(",\"kind\":\"fraction\",\"rounded_width\":");
                output.push_str(
                    &column
                        .rounded_fraction_width
                        .expect("validated fraction")
                        .get()
                        .raw()
                        .to_string(),
                );
                output.push_str(",\"value\":");
                output.push_str(&weight.get().to_string());
            }
        }
        output.push('}');
    }
    output.push_str("],\"containing_flow_id\":");
    output.push_str(&input.containing_flow_id.get().to_string());
    output.push_str(",\"end_indent\":");
    output.push_str(&input.end_indent.get().raw().to_string());
    output.push_str(",\"flow_registry_sha256\":");
    push_table_hash_hex(&mut output, input.flow_registry.bytes());
    output.push_str(",\"frame_inline_size\":");
    output.push_str(&input.frame_inline_size.get().raw().to_string());
    output.push_str(",\"keep_with_next\":");
    output.push_str(if input.keep_with_next {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"layout_epoch\":");
    push_table_epoch_jcs(&mut output, input.epoch);
    output.push_str(",\"package_sha256\":");
    push_table_hash_hex(&mut output, input.package_sha256);
    output.push_str(",\"residual_recipient\":");
    match input.residual_recipient {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push_str("null"),
    }
    output.push_str(",\"rounding_residual\":");
    output.push_str(&input.rounding_residual.raw().to_string());
    output.push_str(",\"rows\":[");
    for (index, row) in input.rows.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"row\":");
        output.push_str(&row.row_ordinal.to_string());
        output.push_str(",\"row_owner\":");
        output.push_str(&row.row_owner.get().to_string());
        output.push_str(",\"section\":\"");
        output.push_str(row.section.as_str());
        output.push_str("\"}");
    }
    output.push_str("],\"space_after\":");
    output.push_str(&input.space_after.get().raw().to_string());
    output.push_str(",\"space_before\":");
    output.push_str(&input.space_before.get().raw().to_string());
    output.push_str(",\"start_indent\":");
    output.push_str(&input.start_indent.get().raw().to_string());
    output.push_str(",\"table_owner\":");
    output.push_str(&input.table_owner.get().to_string());
    output.push_str(",\"visual_policy\":{\"background\":\"transparent\",\"border\":\"none\",\"border_spacing\":0,\"cell_padding\":0,\"vertical_alignment\":\"block-start\"}}");
    output
}

fn push_table_epoch_jcs(output: &mut String, epoch: LayoutEpoch) {
    output.push_str("{\"admitted_resources_sha256\":");
    push_table_hash_hex(output, epoch.admitted_resources().bytes());
    output.push_str(",\"document_sha256\":");
    push_table_hash_hex(output, epoch.document().bytes());
    output.push_str(",\"resolved_input_sha256\":");
    push_table_hash_hex(output, epoch.references().bytes());
    output.push_str(",\"style_page_master_sha256\":");
    push_table_hash_hex(output, epoch.style().bytes());
    output.push('}');
}

fn push_table_hash_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

/// Stable identity of one parsed or generated text-producing package site.
///
/// The generated variant deliberately stores the allocation-independent key,
/// not a generated text-buffer ID. Pagination may replace generated bytes and
/// IDs while the logical site and its computed style remain unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineTextSiteSource {
    Parsed(TextSpan),
    Generated(GeneratedBufferKey),
}

/// `check-package` resolves families and binds font instances, but glyph
/// coverage remains a shaping-time property of the final generated text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineGlyphCoverage {
    DeferredToBuildShaping,
}

/// One canonically ordered text-site result from machine style/font coverage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedMachineTextSite {
    source: MachineTextSiteSource,
    site_owner: NodeId,
    style_owner: NodeId,
    computed: PackageComputedStyle,
    resolved: ResolvedLayoutTextStyle,
    font_instance_id: FontInstanceId,
}

impl PreparedMachineTextSite {
    pub const fn source(&self) -> MachineTextSiteSource {
        self.source
    }

    pub const fn site_owner(&self) -> NodeId {
        self.site_owner
    }

    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }

    pub const fn computed(&self) -> &PackageComputedStyle {
        &self.computed
    }

    pub const fn resolved(&self) -> &ResolvedLayoutTextStyle {
        &self.resolved
    }

    pub const fn font_instance_id(&self) -> FontInstanceId {
        self.font_instance_id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineStyleFontPreparationError {
    LayoutEpoch(LayoutEpochError),
    ParsedText(PackageShapeTextError),
    GeneratedText(PackageShapeTextError),
    MissingGeneratedBuffer(GeneratedBufferKey),
    GeneratedTextTooLarge(GeneratedBufferKey),
    InvalidGeneratedProvenance(GeneratedBufferKey),
    Style(PackageStyleError),
    LayoutStyle(LayoutTextStyleError),
    Resource(ResourceAdmissionError),
    FontSelection(ShapeFontSelectionError),
    ResourceLimit,
}

/// Complete style-family/font-instance coverage proof for a machine package.
///
/// Construction walks paragraph/heading sites in typed NodeId preorder, then
/// checks every remaining generated site. Discretionary break-control sites
/// are recorded separately because they cannot produce glyphs and therefore
/// need no family. The proof owns the dense font-instance table used by
/// layout, so downstream shaping cannot replace a resolved family with a
/// caller-selected face.
#[derive(Debug)]
pub struct PreparedMachineStyleFonts {
    epoch: LayoutEpoch,
    sites: Vec<PreparedMachineTextSite>,
    non_text_generated_sites: Vec<GeneratedBufferKey>,
    font_instances: AdmittedFontInstanceTable,
    glyph_coverage: MachineGlyphCoverage,
}

impl PreparedMachineStyleFonts {
    pub fn prepare(
        package: &ValidatedMachinePackage,
        generated: PackageGeneratedTextBinding<'_>,
        admitted: AdmittedResourceLedgerToken<'_>,
    ) -> Result<Self, MachineStyleFontPreparationError> {
        let parsed = package.package();
        if !core::ptr::eq(generated.package(), parsed) {
            return Err(MachineStyleFontPreparationError::LayoutEpoch(
                LayoutEpochError::PackageEpochMismatch,
            ));
        }
        let epoch = LayoutEpoch::from_validated_inputs(generated, admitted)
            .map_err(MachineStyleFontPreparationError::LayoutEpoch)?;

        let node_count = parsed.document_nodes().node_count();
        let generated_count = parsed.document_nodes().generated_sites().len();
        let capacity = node_count
            .checked_add(generated_count)
            .ok_or(MachineStyleFontPreparationError::ResourceLimit)?;
        let mut sources = Vec::new();
        sources
            .try_reserve_exact(capacity)
            .map_err(|_| MachineStyleFontPreparationError::ResourceLimit)?;
        let mut seen_generated = std::collections::BTreeSet::new();
        let mut non_text_generated_sites = Vec::new();
        non_text_generated_sites
            .try_reserve_exact(generated_count)
            .map_err(|_| MachineStyleFontPreparationError::ResourceLimit)?;

        for (owner, _kind) in parsed.document_nodes().nodes() {
            let Some(paragraph_sites) = parsed.paragraph_shape_text_sites(owner) else {
                continue;
            };
            for site in paragraph_sites {
                match site {
                    PackageParagraphTextSite::Parsed(span) => {
                        sources.push(MachineTextSiteSource::Parsed(span));
                    }
                    PackageParagraphTextSite::Generated(key) => {
                        seen_generated.insert(key);
                        sources.push(MachineTextSiteSource::Generated(key));
                    }
                }
            }
        }

        // Discretionary sites represent explicit line-break control and never
        // require a font. They are still bound against the selected generated
        // store so the coverage proof closes over the complete site registry.
        // Every other generated site can materialize text in a later
        // pagination state and therefore participates in family binding.
        for site in parsed.document_nodes().generated_sites() {
            let key = site.key();
            if !seen_generated.insert(key) {
                continue;
            }
            if key.generation_kind() == GenerationKind::Discretionary {
                let buffer = generated
                    .generated_text()
                    .buffers()
                    .iter()
                    .find(|buffer| buffer.key() == key)
                    .ok_or(MachineStyleFontPreparationError::MissingGeneratedBuffer(
                        key,
                    ))?;
                let end = u32::try_from(buffer.utf8().len())
                    .map_err(|_| MachineStyleFontPreparationError::GeneratedTextTooLarge(key))?;
                let provenance = generated
                    .generated_text()
                    .provenance(key, Utf8ByteOffset::new(0), Utf8ByteOffset::new(end))
                    .map_err(|_| {
                        MachineStyleFontPreparationError::InvalidGeneratedProvenance(key)
                    })?;
                generated
                    .bind_generated_shape_text(provenance)
                    .map_err(MachineStyleFontPreparationError::GeneratedText)?;
                non_text_generated_sites.push(key);
            } else {
                sources.push(MachineTextSiteSource::Generated(key));
            }
        }

        let mut drafts = Vec::new();
        drafts
            .try_reserve_exact(sources.len())
            .map_err(|_| MachineStyleFontPreparationError::ResourceLimit)?;
        for source in sources {
            let text = match source {
                MachineTextSiteSource::Parsed(span) => parsed
                    .bind_parsed_shape_text(span)
                    .map_err(MachineStyleFontPreparationError::ParsedText)?,
                MachineTextSiteSource::Generated(key) => {
                    let buffer = generated
                        .generated_text()
                        .buffers()
                        .iter()
                        .find(|buffer| buffer.key() == key)
                        .ok_or(MachineStyleFontPreparationError::MissingGeneratedBuffer(
                            key,
                        ))?;
                    let end = u32::try_from(buffer.utf8().len()).map_err(|_| {
                        MachineStyleFontPreparationError::GeneratedTextTooLarge(key)
                    })?;
                    let provenance = generated
                        .generated_text()
                        .provenance(key, Utf8ByteOffset::new(0), Utf8ByteOffset::new(end))
                        .map_err(|_| {
                            MachineStyleFontPreparationError::InvalidGeneratedProvenance(key)
                        })?;
                    generated
                        .bind_generated_shape_text(provenance)
                        .map_err(MachineStyleFontPreparationError::GeneratedText)?
                }
            };
            let computed = match parsed.cascade_style(text.site_owner()) {
                Ok(computed) => computed,
                Err(PackageStyleError::UnknownStyleOwner)
                    if text.style_owner() != text.site_owner()
                        && matches!(
                            source,
                            MachineTextSiteSource::Generated(key)
                                if key.generation_kind() == GenerationKind::FootnoteMarker
                        ) =>
                {
                    parsed
                        .cascade_footnote_marker_style(text.site_owner())
                        .map_err(MachineStyleFontPreparationError::Style)?
                }
                Err(error) => return Err(MachineStyleFontPreparationError::Style(error)),
            };
            let resolved = ResolvedLayoutTextStyle::new(parsed, &computed, admitted)
                .map_err(MachineStyleFontPreparationError::LayoutStyle)?;
            drafts.push((
                source,
                text.site_owner(),
                text.style_owner(),
                computed,
                resolved,
            ));
        }

        let font_instances = AdmittedFontInstanceTable::from_used_faces(
            admitted.ledger(),
            drafts
                .iter()
                .map(|(_, _, _, _, resolved)| resolved.resolved().font_face_id()),
        )
        .map_err(MachineStyleFontPreparationError::Resource)?;
        let mut sites = Vec::new();
        sites
            .try_reserve_exact(drafts.len())
            .map_err(|_| MachineStyleFontPreparationError::ResourceLimit)?;
        for (source, site_owner, style_owner, computed, resolved) in drafts {
            let selection =
                ShapeFontSelectionReceipt::new(parsed, &computed, admitted, &font_instances, epoch)
                    .map_err(MachineStyleFontPreparationError::FontSelection)?;
            sites.push(PreparedMachineTextSite {
                source,
                site_owner,
                style_owner,
                computed,
                resolved,
                font_instance_id: selection.admitted_font().font_instance_id(),
            });
        }
        sites.sort_by_key(machine_site_order_key);

        Ok(Self {
            epoch,
            sites,
            non_text_generated_sites,
            font_instances,
            glyph_coverage: MachineGlyphCoverage::DeferredToBuildShaping,
        })
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub fn sites(&self) -> &[PreparedMachineTextSite] {
        &self.sites
    }

    pub fn non_text_generated_sites(&self) -> &[GeneratedBufferKey] {
        &self.non_text_generated_sites
    }

    pub const fn font_instances(&self) -> &AdmittedFontInstanceTable {
        &self.font_instances
    }

    pub const fn glyph_coverage(&self) -> MachineGlyphCoverage {
        self.glyph_coverage
    }

    pub fn site(
        &self,
        source: MachineTextSiteSource,
        site_owner: NodeId,
    ) -> Option<&PreparedMachineTextSite> {
        self.sites
            .iter()
            .find(|site| site.source == source && site.site_owner == site_owner)
    }

    pub fn matches_package_epoch(
        &self,
        package: &ValidatedMachinePackage,
        epoch: LayoutEpoch,
    ) -> bool {
        let identity = package.package().epoch_identity();
        self.epoch.same_stable_inputs(epoch)
            && self.epoch.document() == identity.document()
            && self.epoch.style() == identity.style()
    }
}

fn machine_site_order_key(site: &PreparedMachineTextSite) -> (u32, u8, u32, u32, u32) {
    match site.source {
        MachineTextSiteSource::Parsed(span) => (
            site.site_owner.get(),
            0,
            span.text_id().get(),
            span.start_byte().get(),
            span.end_byte().get(),
        ),
        MachineTextSiteSource::Generated(key) => (
            site.site_owner.get(),
            1,
            key.owner().get(),
            machine_generation_kind_order(key.generation_kind()),
            key.owner_local_ordinal(),
        ),
    }
}

const fn machine_generation_kind_order(kind: GenerationKind) -> u32 {
    match kind {
        GenerationKind::PageReference => 0,
        GenerationKind::Counter => 1,
        GenerationKind::ListMarker => 2,
        GenerationKind::FootnoteMarker => 3,
        GenerationKind::Discretionary => 4,
    }
}

/// Text style after package/style identity and the admitted resource set have
/// all been checked at one trust boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedLayoutTextStyle {
    owner: NodeId,
    style_owner: NodeId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    admitted_resources: AdmittedResourceFingerprint,
    resolved: ResolvedTextStyle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutTextStyleError {
    PackageStyleMismatch,
    AdmittedResourceDocumentMismatch,
    InvalidStyle(StyleValidationError),
}

impl ResolvedLayoutTextStyle {
    pub fn new(
        package: &ValidatedParsedPackage,
        computed: &PackageComputedStyle,
        admitted: AdmittedResourceLedgerToken<'_>,
    ) -> Result<Self, LayoutTextStyleError> {
        if computed.document_fingerprint() != package.epoch_identity().document()
            || computed.style_fingerprint() != package.epoch_identity().style()
        {
            return Err(LayoutTextStyleError::PackageStyleMismatch);
        }
        if !admitted
            .ledger()
            .matches_declarations(&package.package().resources)
        {
            return Err(LayoutTextStyleError::AdmittedResourceDocumentMismatch);
        }
        let resolved = ResolvedTextStyle::try_from_computed(computed.computed(), admitted)
            .map_err(LayoutTextStyleError::InvalidStyle)?;
        Ok(Self {
            owner: computed.owner(),
            style_owner: computed.style_owner(),
            document: computed.document_fingerprint(),
            style: computed.style_fingerprint(),
            admitted_resources: admitted.fingerprint(),
            resolved,
        })
    }

    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }
    pub const fn admitted_resource_fingerprint(&self) -> AdmittedResourceFingerprint {
        self.admitted_resources
    }
    pub fn matches_epoch(&self, epoch: LayoutEpoch) -> bool {
        self.document == epoch.document()
            && self.style == epoch.style()
            && self.admitted_resources == epoch.admitted_resources()
    }
    pub const fn resolved(&self) -> &ResolvedTextStyle {
        &self.resolved
    }
}

/// Sealed selection proof consumed by shaping. It is impossible to construct
/// this receipt from a caller-selected instance ID, hash, or raw font bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapeFontSelectionReceipt<'a> {
    epoch: LayoutEpoch,
    style: ResolvedLayoutTextStyle,
    font: AdmittedFontInstanceRef<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapeFontSelectionError {
    LayoutStyle(LayoutTextStyleError),
    EpochMismatch,
    FontInstanceLedgerMismatch,
    MissingFontInstance(FontFaceId),
    DuplicateFontFaceInstance(FontFaceId),
}

impl<'a> ShapeFontSelectionReceipt<'a> {
    pub fn new(
        package: &ValidatedParsedPackage,
        computed: &PackageComputedStyle,
        admitted: AdmittedResourceLedgerToken<'a>,
        instances: &'a AdmittedFontInstanceTable,
        epoch: LayoutEpoch,
    ) -> Result<Self, ShapeFontSelectionError> {
        let style = ResolvedLayoutTextStyle::new(package, computed, admitted)
            .map_err(ShapeFontSelectionError::LayoutStyle)?;
        if !style.matches_epoch(epoch) {
            return Err(ShapeFontSelectionError::EpochMismatch);
        }
        let selected_face = style.resolved().font_face_id();
        let font_instance_id = select_font_instance(
            admitted.fingerprint(),
            instances.ledger_fingerprint(),
            selected_face,
            instances
                .instances()
                .iter()
                .map(|instance| (instance.font_instance_id(), instance.font_face_id())),
        )?;
        let font = instances
            .resolve(font_instance_id, admitted.ledger())
            .ok_or(ShapeFontSelectionError::FontInstanceLedgerMismatch)?;
        if font.ledger_fingerprint() != admitted.fingerprint()
            || font.font_face_id() != selected_face
        {
            return Err(ShapeFontSelectionError::FontInstanceLedgerMismatch);
        }
        Ok(Self { epoch, style, font })
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn style(&self) -> &ResolvedLayoutTextStyle {
        &self.style
    }
    pub const fn owner(&self) -> NodeId {
        self.style.style_owner()
    }
    pub const fn admitted_font(&self) -> AdmittedFontInstanceRef<'a> {
        self.font
    }
    pub fn matches_epoch(&self, epoch: LayoutEpoch) -> bool {
        self.epoch == epoch
    }
}

fn select_font_instance(
    expected_ledger: AdmittedResourceFingerprint,
    table_ledger: AdmittedResourceFingerprint,
    selected_face: FontFaceId,
    instances: impl IntoIterator<Item = (FontInstanceId, FontFaceId)>,
) -> Result<FontInstanceId, ShapeFontSelectionError> {
    if table_ledger != expected_ledger {
        return Err(ShapeFontSelectionError::FontInstanceLedgerMismatch);
    }
    let mut matching = instances
        .into_iter()
        .filter(|(_, face)| *face == selected_face);
    let selected = matching
        .next()
        .ok_or(ShapeFontSelectionError::MissingFontInstance(selected_face))?;
    if matching.next().is_some() {
        return Err(ShapeFontSelectionError::DuplicateFontFaceInstance(
            selected_face,
        ));
    }
    Ok(selected.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{
        admitted_resource_fingerprint_from_jcs, document_fingerprint_from_jcs,
        style_fingerprint_from_jcs,
    };

    fn table_length(raw: i64) -> Length {
        Length::from_raw(raw).unwrap()
    }

    fn table_positive(raw: i64) -> PositiveLength {
        PositiveLength::new(table_length(raw)).unwrap()
    }

    fn table_nonnegative(raw: i64) -> NonNegativeLength {
        NonNegativeLength::new(table_length(raw)).unwrap()
    }

    fn table_epoch() -> LayoutEpoch {
        LayoutEpoch {
            document: document_fingerprint_from_jcs("{\"document\":\"table\"}"),
            style: style_fingerprint_from_jcs("{\"style\":\"table\"}"),
            admitted_resources: admitted_resource_fingerprint_from_jcs("{\"resources\":\"table\"}"),
            references: ReferenceFingerprint::from_untrusted_bytes([4; 32]),
        }
    }

    fn table_receipt_input() -> TableGridReceiptInput {
        TableGridReceiptInput {
            package_sha256: [9; 32],
            epoch: table_epoch(),
            flow_registry: flow_registry_fingerprint_from_jcs(
                "{\"algorithm\":\"typaxis.basic-flow-registry/1\",\"table\":1}",
            ),
            table_owner: NodeId::new(1),
            containing_flow_id: FlowId::DOCUMENT_BODY,
            frame_inline_size: table_positive(5),
            available_inline_size: table_positive(3),
            start_indent: table_nonnegative(1),
            end_indent: table_nonnegative(1),
            space_before: table_nonnegative(2),
            space_after: table_nonnegative(3),
            keep_with_next: true,
            columns: vec![
                ResolvedTableColumn::fraction(
                    0,
                    NonZeroU16::new(1).unwrap(),
                    table_nonnegative(2),
                    table_positive(2),
                ),
                ResolvedTableColumn::fraction(
                    1,
                    NonZeroU16::new(1).unwrap(),
                    table_nonnegative(2),
                    table_positive(1),
                ),
            ],
            rounding_residual: table_length(-1),
            residual_recipient: Some(1),
            rows: vec![ValidatedTableRowBinding::new(
                NodeId::new(2),
                TableSection::Body,
                0,
            )],
            cells: vec![
                ValidatedTableCellBinding::new(
                    NodeId::new(3),
                    NodeId::new(2),
                    TableSection::Body,
                    0,
                    0,
                    NonZeroU16::new(1).unwrap(),
                    NonZeroU16::new(1).unwrap(),
                    FlowId::new(4),
                    FlowTerminal::new(0),
                    table_nonnegative(0),
                    table_positive(2),
                ),
                ValidatedTableCellBinding::new(
                    NodeId::new(4),
                    NodeId::new(2),
                    TableSection::Body,
                    0,
                    1,
                    NonZeroU16::new(1).unwrap(),
                    NonZeroU16::new(1).unwrap(),
                    FlowId::new(5),
                    FlowTerminal::new(2),
                    table_nonnegative(2),
                    table_positive(1),
                ),
            ],
        }
    }

    #[test]
    fn flow_registry_contract_is_closed_and_domain_separated() {
        assert_eq!(FlowId::DOCUMENT_BODY.get(), 0);
        assert_eq!(FlowId::new(7).get(), 7);
        assert_eq!(FlowOwnerKind::DocumentBody.as_str(), "document_body");
        assert_eq!(FlowOwnerKind::ListItem.as_str(), "list_item");
        assert_eq!(FlowOwnerKind::FigureCaption.as_str(), "figure_caption");
        assert_eq!(FlowOwnerKind::TableCell.as_str(), "table_cell");
        assert_eq!(FlowContentKind::Paragraph.as_str(), "paragraph");
        assert_eq!(FlowContentKind::ListItem.as_str(), "list_item");
        assert_eq!(FlowContentKind::FigureCaption.as_str(), "figure_caption");
        assert_eq!(FlowContentKind::PageBreak.as_str(), "page_break");
        assert_eq!(FlowContentKind::TableRow.as_str(), "table_row");
        assert_eq!(FlowTerminal::new(3).owner_local_ordinal(), 3);
        assert_eq!(FootnoteFlowId::new(0).get(), 0);
        assert_eq!(FootnoteFlowTerminal::new(2).fragment_count(), 2);
        let binding = FootnoteFlowBinding::new(
            FootnoteId::new("note").unwrap(),
            FootnoteFlowId::new(0),
            NodeId::new(9),
            FootnoteFlowTerminal::new(2),
        );
        assert_eq!(binding.footnote_id().as_str(), "note");
        assert_eq!(binding.flow_id(), FootnoteFlowId::new(0));
        assert_eq!(binding.definition_owner(), NodeId::new(9));

        let registry = flow_registry_fingerprint_from_jcs(
            "{\"algorithm\":\"typaxis.basic-flow-registry/1\",\"flows\":[]}",
        );
        let selected = multi_flow_selected_state_fingerprint_from_jcs(
            "{\"algorithm\":\"typaxis.multi-flow-selected-state/1\",\"flows\":[]}",
        );
        assert_ne!(registry.bytes(), selected.bytes());
        assert_eq!(
            FlowRegistryFingerprint::ALGORITHM_ID,
            "typaxis.basic-flow-registry/1"
        );
        let footnote_profile = footnote_profile_fingerprint_from_jcs(
            "{\"algorithm\":\"typaxis.footnote-profile-receipt/1\"}",
        );
        let footnote_registry = footnote_flow_registry_fingerprint_from_jcs(
            "{\"algorithm\":\"typaxis.footnote-flow-registry/1\"}",
        );
        let footnote_page = footnote_page_evaluation_fingerprint_from_jcs(
            "{\"algorithm\":\"typaxis.footnote-page-evaluation/1\"}",
        );
        assert_ne!(footnote_profile.bytes(), footnote_registry.bytes());
        assert_ne!(footnote_registry.bytes(), footnote_page.bytes());
        assert_eq!(
            FootnoteFlowRegistryFingerprint::ALGORITHM_ID,
            "typaxis.footnote-flow-registry/1"
        );
    }

    #[test]
    fn table_receipts_bind_residual_grid_flows_frames_and_fixed_policy() {
        let first = ValidatedTableGridReceipt::new(table_receipt_input()).unwrap();
        let second = ValidatedTableGridReceipt::new(table_receipt_input()).unwrap();
        assert_eq!(first.fingerprint(), second.fingerprint());
        assert_eq!(first.rounding_residual().raw(), -1);
        assert_eq!(first.residual_recipient(), Some(1));
        assert_eq!(
            first
                .columns()
                .iter()
                .map(|column| column.final_width().get().raw())
                .collect::<Vec<_>>(),
            [2, 1]
        );
        assert_eq!(first.cells()[0].flow_id(), FlowId::new(4));
        assert_eq!(first.cells()[1].frame_inline_start().get().raw(), 2);
        assert_eq!(
            first.cells()[0].vertical_alignment(),
            TableVerticalAlignment::BlockStart
        );
        assert_eq!(first.cells()[0].padding_start(), NonNegativeLength::ZERO);
        assert_eq!(
            TableGridFingerprint::ALGORITHM_ID,
            "typaxis.table-grid-receipt/1"
        );
        let selected = table_selected_layout_fingerprint_from_jcs(
            "{\"algorithm\":\"typaxis.table-selected-layout/1\",\"table_node_id\":1}",
        );
        assert_ne!(first.fingerprint().bytes(), selected.bytes());
        assert_eq!(
            TableSelectedLayoutFingerprint::ALGORITHM_ID,
            "typaxis.table-selected-layout/1"
        );
    }

    #[test]
    fn table_receipts_reject_wrong_rounding_grid_owner_frame_and_flow() {
        let mut wrong_residual = table_receipt_input();
        wrong_residual.residual_recipient = Some(0);
        assert_eq!(
            ValidatedTableGridReceipt::new(wrong_residual).unwrap_err(),
            TableGridReceiptError::InvalidResidualRecipient
        );

        let mut wrong_owner = table_receipt_input();
        wrong_owner.cells[0] = ValidatedTableCellBinding::new(
            NodeId::new(3),
            NodeId::new(99),
            TableSection::Body,
            0,
            0,
            NonZeroU16::new(1).unwrap(),
            NonZeroU16::new(1).unwrap(),
            FlowId::new(4),
            FlowTerminal::new(0),
            table_nonnegative(0),
            table_positive(2),
        );
        assert_eq!(
            ValidatedTableGridReceipt::new(wrong_owner).unwrap_err(),
            TableGridReceiptError::WrongRowOwner(NodeId::new(3))
        );

        let mut wrong_frame = table_receipt_input();
        wrong_frame.cells[1] = ValidatedTableCellBinding::new(
            NodeId::new(4),
            NodeId::new(2),
            TableSection::Body,
            0,
            1,
            NonZeroU16::new(1).unwrap(),
            NonZeroU16::new(1).unwrap(),
            FlowId::new(5),
            FlowTerminal::new(2),
            table_nonnegative(1),
            table_positive(1),
        );
        assert_eq!(
            ValidatedTableGridReceipt::new(wrong_frame).unwrap_err(),
            TableGridReceiptError::InvalidColumnResolution(1)
        );

        let mut wrong_flow = table_receipt_input();
        wrong_flow.cells[1] = ValidatedTableCellBinding::new(
            NodeId::new(4),
            NodeId::new(2),
            TableSection::Body,
            0,
            1,
            NonZeroU16::new(1).unwrap(),
            NonZeroU16::new(1).unwrap(),
            FlowId::new(7),
            FlowTerminal::new(2),
            table_nonnegative(2),
            table_positive(1),
        );
        assert_eq!(
            ValidatedTableGridReceipt::new(wrong_flow).unwrap_err(),
            TableGridReceiptError::NonCanonicalCellFlow(NodeId::new(4))
        );

        let mut hole = table_receipt_input();
        hole.cells.pop();
        assert_eq!(
            ValidatedTableGridReceipt::new(hole).unwrap_err(),
            TableGridReceiptError::RowHole(NodeId::new(2))
        );
    }

    #[test]
    fn font_selection_rejects_a_table_from_another_ledger() {
        let expected = admitted_resource_fingerprint_from_jcs("{\"ledger\":0}");
        let other = admitted_resource_fingerprint_from_jcs("{\"ledger\":1}");
        assert_eq!(
            select_font_instance(
                expected,
                other,
                FontFaceId::new(0),
                [(FontInstanceId::new(0), FontFaceId::new(0))],
            ),
            Err(ShapeFontSelectionError::FontInstanceLedgerMismatch)
        );
    }

    #[test]
    fn font_selection_rejects_wrong_or_duplicate_faces() {
        let ledger = admitted_resource_fingerprint_from_jcs("{\"ledger\":0}");
        assert_eq!(
            select_font_instance(
                ledger,
                ledger,
                FontFaceId::new(0),
                [(FontInstanceId::new(0), FontFaceId::new(1))],
            ),
            Err(ShapeFontSelectionError::MissingFontInstance(
                FontFaceId::new(0)
            ))
        );
        assert_eq!(
            select_font_instance(
                ledger,
                ledger,
                FontFaceId::new(0),
                [
                    (FontInstanceId::new(0), FontFaceId::new(0)),
                    (FontInstanceId::new(1), FontFaceId::new(0)),
                ],
            ),
            Err(ShapeFontSelectionError::DuplicateFontFaceInstance(
                FontFaceId::new(0)
            ))
        );
    }
}
