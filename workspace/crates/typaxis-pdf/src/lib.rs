#![forbid(unsafe_code)]

mod advanced_columns;
mod advanced_content;
mod advanced_float;
mod advanced_header_footer;
mod book_navigation;
mod math;
mod safe_vector;
mod safe_vector_v2;
mod semantic_container;
mod tagged_pdf;

pub use advanced_columns::{
    serialize_columns_pdf, serialize_staging_columns_pdf, StagingColumnsPdf,
    StagingColumnsPdfClosureReceipt, StagingColumnsPdfError, StagingColumnsPdfPageObservation,
};
pub use advanced_float::{
    serialize_float_pdf, serialize_staging_float_pdf, StagingFloatPdf,
    StagingFloatPdfClosureReceipt, StagingFloatPdfError, StagingFloatPdfObjectUsage,
    StagingFloatPdfPageObservation,
};
pub use advanced_header_footer::{
    serialize_header_footer_pdf, serialize_staging_header_footer_pdf, StagingHeaderFooterPdf,
    StagingHeaderFooterPdfClosureReceipt, StagingHeaderFooterPdfError,
    StagingHeaderFooterPdfPageObservation, STAGING_HEADER_FOOTER_PDF_CLOSURE_ALGORITHM,
};
pub use book_navigation::{
    write_staging_book_navigation_pdf, BookNavigationPdfError, BookNavigationPdfLinkObservation,
    BookNavigationPdfObjectObservation, BookNavigationPdfObservation,
    BookNavigationPdfOutlineObservation, StagingBookNavigationPdf, BOOK_NAVIGATION_PDF_ALGORITHM,
    BOOK_XMP_ALGORITHM,
};
pub use math::{
    write_staging_math_pdf, StagingMathPdf, StagingMathPdfError, StagingMathPdfObservation,
    MATH_PDF_ALGORITHM,
};
pub use safe_vector::{
    write_staging_safe_vector_pdf, StagingSafeVectorPdf, StagingSafeVectorPdfError,
    StagingSafeVectorPdfFormObject, StagingSafeVectorPdfReceipt, StagingSafeVectorPdfUsage,
    STAGING_SAFE_VECTOR_PDF_ALGORITHM,
};
pub use safe_vector_v2::{
    build_staging_safe_vector_pdf_contribution_v2, seal_staging_safe_vector_pdf_v2,
    write_staging_safe_vector_pdf_contribution_v2, StagingSafeVectorPdfClosureV2,
    StagingSafeVectorPdfContributionV2, StagingSafeVectorPdfExtGStateV2,
    StagingSafeVectorPdfFinalObjectObservationV2, StagingSafeVectorPdfFinalUsageObservationV2,
    StagingSafeVectorPdfFinalWriterObservationV2, StagingSafeVectorPdfFormV2,
    StagingSafeVectorPdfPageResourceV2, StagingSafeVectorPdfPageV2,
    StagingSafeVectorPdfRelativeObjectKindV2, StagingSafeVectorPdfRelativeObjectV2,
    StagingSafeVectorPdfSemanticUsageHookV2, StagingSafeVectorPdfUsageV2,
    StagingSafeVectorPdfV2Error, STAGING_SAFE_VECTOR_PDF_ALGORITHM_V2,
    STAGING_SAFE_VECTOR_PDF_CONTRIBUTION_V2_ALGORITHM,
};
#[cfg(any(test, feature = "staging-fixtures"))]
pub use safe_vector_v2::{
    staging_safe_vector_isolated_pdf_fixture_v2, StagingSafeVectorIsolatedPdfFixtureV2,
};
pub use semantic_container::{
    write_staging_semantic_container_pdf, StagingSemanticContainerPdf,
    StagingSemanticContainerPdfClosureReceipt, StagingSemanticContainerPdfError,
    StagingSemanticContainerPdfPageObservation, StagingSemanticContainerStructureInput,
};
pub use tagged_pdf::{
    write_staging_tagged_pdf, StagingTaggedPdf, TaggedPdfError, TaggedPdfObjectObservation,
    TaggedPdfObservation, TAGGED_PDF_ALGORITHM, TAGGED_PDF_XMP_ALGORITHM,
};

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::io::{self, Write};
use typaxis_core::{
    push_generated_buffer_key_jcs, push_jcs_string, sha256, AnchorId, EffectiveConfig,
    EffectiveConfigFingerprint, FontInstanceId, GeneratedBufferKey, ImageResourceId,
    LayoutStateFingerprint, Length, MasterId, PdfStreamCompression, Point, PositiveLength, Rect,
    ValidatedResourceLimits,
};
use typaxis_display_list::{
    ClusterExtraction, DestinationView, DisplayCommand, DisplayGlyph, DisplayPage, FillRule,
    FootnoteDisplayClosureReceipt, FootnotePaintCommandKind, FootnoteProfileDisplay, LineCap,
    LineJoin, LinkAnnotation, LinkTarget, NamedDestination, Paint, Path, PathVerb,
    StagingForcedPageBreakDisplay, StagingMachineBlockStyleDisplay,
    StagingMachineFigureDisplayFacts, StagingMachineLinkDisplayFacts,
    StagingMachineLinkDisplayRectangle, StagingMachineLinkDisplayTarget, StagingMachineListDisplay,
    TableDisplayClosureReceipt, TablePaintCellObservation, TablePaintCommandObservation,
    TablePaintOccurrenceKind, TablePaintRect, TableProfileDisplay, ValidatedDisplayDocument,
};
use typaxis_resources::{
    ClusterExtractionPlan, FrozenPdfAlphaMask, FrozenPdfFontPlan, FrozenPdfImagePlan,
    FrozenPdfResourcePlans, ImageColorSpace, ImageEncoding, PdfFontIndirectObjectRole,
};

pub const TABLE_PDF_CLOSURE_ALGORITHM: &str = "typaxis.table-pdf-closure/1";
pub const FOOTNOTE_PDF_CLOSURE_ALGORITHM: &str = "typaxis.footnote-pdf-closure/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FootnotePdfClosureError {
    DisplayStateMismatch,
    PageClosure,
    CommandClosure,
    PdfReceiptMismatch,
}

/// Serializer-bound MI3-07 receipt. Exact separator and definition commands
/// remain available through the retained Display closure; this receipt binds
/// that closure to the one emitted byte stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FootnotePdfClosureReceipt {
    display_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    body_layout_sha256: [u8; 32],
    pdf_sha256: [u8; 32],
    pdf_byte_length: u64,
    page_count: u32,
    object_count: u32,
    reference_marker_count: u32,
    separator_count: u32,
    definition_command_count: u32,
    canonical_jcs: String,
}

impl FootnotePdfClosureReceipt {
    pub fn from_serialized(
        display: &FootnoteDisplayClosureReceipt,
        graph: &FrozenPdfGraph,
        receipt: &VerifiedPdfBytesReceipt,
    ) -> Result<Self, FootnotePdfClosureError> {
        if graph.footnote_closure() != Some(display)
            || graph.selected_layout_fingerprint().bytes() != display.body_layout_sha256()
            || receipt.selected_layout_fingerprint() != graph.selected_layout_fingerprint()
            || receipt.footnote_display_sha256() != Some(display.fingerprint())
        {
            return Err(FootnotePdfClosureError::DisplayStateMismatch);
        }
        let display_page_count = u32::try_from(display.pages().len())
            .map_err(|_| FootnotePdfClosureError::PageClosure)?;
        if graph.page_count() != display_page_count || receipt.page_count() != graph.page_count() {
            return Err(FootnotePdfClosureError::PageClosure);
        }
        if receipt.object_count() != graph.object_count()
            || receipt.byte_length() == 0
            || sha256(receipt.bytes()) != receipt.content_hash()
        {
            return Err(FootnotePdfClosureError::PdfReceiptMismatch);
        }
        let separator_count = u32::try_from(
            display
                .commands()
                .iter()
                .filter(|command| command.kind() == FootnotePaintCommandKind::Separator)
                .count(),
        )
        .map_err(|_| FootnotePdfClosureError::CommandClosure)?;
        let reference_marker_count = u32::try_from(
            display
                .commands()
                .iter()
                .filter(|command| command.kind() == FootnotePaintCommandKind::ReferenceMarker)
                .count(),
        )
        .map_err(|_| FootnotePdfClosureError::CommandClosure)?;
        let definition_command_count = u32::try_from(
            display
                .commands()
                .iter()
                .filter(|command| command.kind() == FootnotePaintCommandKind::Definition)
                .count(),
        )
        .map_err(|_| FootnotePdfClosureError::CommandClosure)?;
        let mut value = Self {
            display_sha256: display.fingerprint(),
            selected_layout_sha256: display.selected_layout_sha256(),
            body_layout_sha256: display.body_layout_sha256(),
            pdf_sha256: receipt.content_hash(),
            pdf_byte_length: receipt.byte_length(),
            page_count: receipt.page_count(),
            object_count: receipt.object_count(),
            reference_marker_count,
            separator_count,
            definition_command_count,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_footnote_pdf_closure(&value);
        Ok(value)
    }

    pub const fn display_sha256(&self) -> [u8; 32] {
        self.display_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub const fn body_layout_sha256(&self) -> [u8; 32] {
        self.body_layout_sha256
    }
    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub const fn separator_count(&self) -> u32 {
        self.separator_count
    }
    pub const fn reference_marker_count(&self) -> u32 {
        self.reference_marker_count
    }
    pub const fn definition_command_count(&self) -> u32 {
        self.definition_command_count
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn encode_footnote_pdf_closure(value: &FootnotePdfClosureReceipt) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, FOOTNOTE_PDF_CLOSURE_ALGORITHM);
    output.push_str(",\"body_layout_sha256\":");
    push_pdf_hex(&mut output, value.body_layout_sha256);
    output.push_str(",\"definition_command_count\":");
    output.push_str(&value.definition_command_count.to_string());
    output.push_str(",\"display_sha256\":");
    push_pdf_hex(&mut output, value.display_sha256);
    output.push_str(",\"object_count\":");
    output.push_str(&value.object_count.to_string());
    output.push_str(",\"page_count\":");
    output.push_str(&value.page_count.to_string());
    output.push_str(",\"pdf_byte_length\":");
    output.push_str(&value.pdf_byte_length.to_string());
    output.push_str(",\"pdf_sha256\":");
    push_pdf_hex(&mut output, value.pdf_sha256);
    output.push_str(",\"reference_marker_count\":");
    output.push_str(&value.reference_marker_count.to_string());
    output.push_str(",\"selected_layout_sha256\":");
    push_pdf_hex(&mut output, value.selected_layout_sha256);
    output.push_str(",\"separator_count\":");
    output.push_str(&value.separator_count.to_string());
    output.push('}');
    output
}

fn push_pdf_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TablePdfDecorationObservation {
    Background,
    Border,
    BorderSpacing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TablePdfCellObservation {
    pub kind: TablePaintOccurrenceKind,
    pub page_index: u32,
    pub fragment_id: u64,
    pub source_fragment_id: Option<u64>,
    pub repetition_index: Option<u32>,
    pub row_node_id: u32,
    pub logical_row_ordinal: u32,
    pub row_fragment_ordinal: u32,
    pub cell_node_id: u32,
    pub flow_id: u32,
    pub column_ordinal: u32,
    pub colspan: u16,
    pub rowspan: u16,
    pub rect: TablePaintRect,
    pub content_fragment_start: u32,
    pub content_fragment_end: u32,
}

impl From<&TablePaintCellObservation> for TablePdfCellObservation {
    fn from(value: &TablePaintCellObservation) -> Self {
        Self {
            kind: value.kind,
            page_index: value.page_index,
            fragment_id: value.fragment_id,
            source_fragment_id: value.source_fragment_id,
            repetition_index: value.repetition_index,
            row_node_id: value.row_node_id.get(),
            logical_row_ordinal: value.logical_row_ordinal,
            row_fragment_ordinal: value.row_fragment_ordinal,
            cell_node_id: value.cell_node_id.get(),
            flow_id: value.flow_id.get(),
            column_ordinal: value.column_ordinal,
            colspan: value.colspan,
            rowspan: value.rowspan,
            rect: value.rect,
            content_fragment_start: value.content_fragment_start,
            content_fragment_end: value.content_fragment_end,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TablePdfClosureError {
    DisplayStateMismatch,
    PageClosure,
    PdfReceiptMismatch,
    MissingCell,
    ExtraCell,
    WrongCell,
    WrongPage,
    WrongRepetition,
    WrongRectangle,
    WrongContentRange,
    MissingCommand,
    ExtraCommand,
    WrongCommand,
    DecorationForbidden,
    AllocationFailure,
}

/// PDF observation bound to the exact selected table Display receipt and the
/// serializer's non-cloneable byte receipt. The table contributes no PDF path,
/// fill, background, border, or spacing operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TablePdfClosureReceipt {
    display_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    pdf_sha256: [u8; 32],
    pdf_byte_length: u64,
    page_count: u32,
    object_count: u32,
    records: Vec<TablePdfCellObservation>,
    commands: Vec<TablePaintCommandObservation>,
    decoration_op_count: u32,
    canonical_jcs: String,
}

impl TablePdfClosureReceipt {
    pub fn from_serialized(
        display: &TableDisplayClosureReceipt,
        graph: &FrozenPdfGraph,
        receipt: &VerifiedPdfBytesReceipt,
    ) -> Result<Self, TablePdfClosureError> {
        let observations = display.records().iter().map(Into::into).collect();
        Self::from_serialized_observed(
            display,
            graph,
            receipt,
            observations,
            display.commands().to_vec(),
            &[],
        )
    }

    pub fn from_serialized_observed(
        display: &TableDisplayClosureReceipt,
        graph: &FrozenPdfGraph,
        receipt: &VerifiedPdfBytesReceipt,
        observed: Vec<TablePdfCellObservation>,
        observed_commands: Vec<TablePaintCommandObservation>,
        decorations: &[TablePdfDecorationObservation],
    ) -> Result<Self, TablePdfClosureError> {
        reject_table_pdf_decorations(display.decoration_op_count(), decorations)?;
        if graph.selected_layout_fingerprint().bytes() != display.layout_state_sha256()
            || receipt.selected_layout_fingerprint() != graph.selected_layout_fingerprint()
            || !graph
                .table_closures()
                .iter()
                .any(|closure| closure == display)
        {
            return Err(TablePdfClosureError::DisplayStateMismatch);
        }
        let expected_pages = display
            .page_bodies()
            .last()
            .and_then(|page| page.target_page_index().checked_add(1))
            .ok_or(TablePdfClosureError::PageClosure)?;
        if graph.page_count() < expected_pages || receipt.page_count() != graph.page_count() {
            return Err(TablePdfClosureError::PageClosure);
        }
        if receipt.object_count() != graph.object_count()
            || receipt.byte_length() == 0
            || sha256(receipt.bytes()) != receipt.content_hash()
        {
            return Err(TablePdfClosureError::PdfReceiptMismatch);
        }
        let expected: Vec<_> = display.records().iter().map(Into::into).collect();
        validate_table_pdf_records(&expected, &observed)?;
        validate_table_pdf_commands(display.commands(), &observed_commands, graph)?;
        let mut value = Self {
            display_sha256: sha256(display.canonical_jcs().as_bytes()),
            selected_layout_sha256: display.selected_layout_sha256(),
            pdf_sha256: receipt.content_hash(),
            pdf_byte_length: receipt.byte_length(),
            page_count: receipt.page_count(),
            object_count: receipt.object_count(),
            records: observed,
            commands: observed_commands,
            decoration_op_count: 0,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_table_pdf_closure(&value);
        Ok(value)
    }

    pub const fn display_sha256(&self) -> [u8; 32] {
        self.display_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub const fn pdf_byte_length(&self) -> u64 {
        self.pdf_byte_length
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub fn records(&self) -> &[TablePdfCellObservation] {
        &self.records
    }
    pub fn commands(&self) -> &[TablePaintCommandObservation] {
        &self.commands
    }
    pub const fn decoration_op_count(&self) -> u32 {
        self.decoration_op_count
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn reject_table_pdf_decorations(
    display_decoration_count: u32,
    decorations: &[TablePdfDecorationObservation],
) -> Result<(), TablePdfClosureError> {
    if display_decoration_count == 0 && decorations.is_empty() {
        Ok(())
    } else {
        Err(TablePdfClosureError::DecorationForbidden)
    }
}

fn validate_table_pdf_commands(
    expected: &[TablePaintCommandObservation],
    observed: &[TablePaintCommandObservation],
    graph: &FrozenPdfGraph,
) -> Result<(), TablePdfClosureError> {
    if observed.len() < expected.len() {
        return Err(TablePdfClosureError::MissingCommand);
    }
    if observed.len() > expected.len() {
        return Err(TablePdfClosureError::ExtraCommand);
    }
    for (actual, expected) in observed.iter().zip(expected) {
        if actual != expected {
            if actual.page_index != expected.page_index {
                return Err(TablePdfClosureError::WrongPage);
            }
            if actual.repetition_index != expected.repetition_index {
                return Err(TablePdfClosureError::WrongRepetition);
            }
            if actual.cell_node_id != expected.cell_node_id
                || actual.fragment_id != expected.fragment_id
            {
                return Err(TablePdfClosureError::WrongCell);
            }
            return Err(TablePdfClosureError::WrongCommand);
        }
        let page = graph
            .graph
            .iter()
            .filter_map(|(_, body)| match body {
                IndirectObjectBody::DisplayPageContent(page)
                    if page.page_index == actual.page_index =>
                {
                    Some(page)
                }
                _ => None,
            })
            .next()
            .ok_or(TablePdfClosureError::WrongPage)?;
        if page.commands.get(actual.page_command_index as usize) != Some(&actual.command) {
            return Err(TablePdfClosureError::WrongCommand);
        }
    }
    validate_no_unclaimed_table_graph_commands(graph)?;
    if graph.graph.iter().any(|(_, body)| {
        matches!(body, IndirectObjectBody::DisplayPageContent(page) if page.commands.iter().any(|command| matches!(command, DisplayCommand::ClipPath { .. } | DisplayCommand::FillPath { .. } | DisplayCommand::StrokePath { .. })))
    }) {
        return Err(TablePdfClosureError::DecorationForbidden);
    }
    Ok(())
}

fn validate_no_unclaimed_table_graph_commands(
    graph: &FrozenPdfGraph,
) -> Result<(), TablePdfClosureError> {
    let claimed: BTreeSet<_> = graph
        .table_closures()
        .iter()
        .flat_map(|closure| closure.commands())
        .map(|observation| (observation.page_index, observation.page_command_index))
        .collect();
    let expected_commands: Vec<_> = graph
        .table_closures()
        .iter()
        .flat_map(|closure| closure.commands().iter().map(|command| &command.command))
        .collect();
    let records: Vec<_> = graph
        .table_closures()
        .iter()
        .flat_map(|closure| closure.records())
        .collect();
    for page in graph.graph.iter().filter_map(|(_, body)| match body {
        IndirectObjectBody::DisplayPageContent(page) => Some(page),
        _ => None,
    }) {
        for (command_index, command) in page.commands.iter().enumerate() {
            let command_index = u32::try_from(command_index)
                .map_err(|_| TablePdfClosureError::AllocationFailure)?;
            if claimed.contains(&(page.page_index, command_index)) {
                continue;
            }
            if is_unclaimed_table_command(command, page.page_index, &expected_commands, &records) {
                return Err(TablePdfClosureError::ExtraCommand);
            }
        }
    }
    Ok(())
}

fn is_unclaimed_table_command(
    command: &DisplayCommand,
    page_index: u32,
    expected_commands: &[&DisplayCommand],
    records: &[&TablePaintCellObservation],
) -> bool {
    expected_commands.contains(&command)
        || command_intersects_table_records(command, page_index, records)
}

fn command_intersects_table_records(
    command: &DisplayCommand,
    page_index: u32,
    records: &[&TablePaintCellObservation],
) -> bool {
    records.iter().any(|record| {
        if record.page_index != page_index {
            return false;
        }
        match command {
            DisplayCommand::DrawGlyphRun { origin, .. } => {
                table_rect_contains_point(record.rect, origin.x.raw(), origin.y.raw())
            }
            DisplayCommand::DrawImage { rect, .. } => table_rects_intersect(record.rect, *rect),
            _ => false,
        }
    })
}

fn table_rect_contains_point(rect: TablePaintRect, x: i64, y: i64) -> bool {
    let Some(right) = rect.x().checked_add(rect.width()) else {
        return false;
    };
    let Some(bottom) = rect.y().checked_add(rect.height()) else {
        return false;
    };
    x >= rect.x() && x < right && y >= rect.y() && y < bottom
}

fn table_rects_intersect(table: TablePaintRect, other: Rect) -> bool {
    let Some(table_right) = table.x().checked_add(table.width()) else {
        return false;
    };
    let Some(table_bottom) = table.y().checked_add(table.height()) else {
        return false;
    };
    let Some(other_right) = other.x().raw().checked_add(other.width().get().raw()) else {
        return false;
    };
    let Some(other_bottom) = other.y().raw().checked_add(other.height().get().raw()) else {
        return false;
    };
    table.x() < other_right
        && other.x().raw() < table_right
        && table.y() < other_bottom
        && other.y().raw() < table_bottom
}

fn validate_table_pdf_records(
    expected: &[TablePdfCellObservation],
    observed: &[TablePdfCellObservation],
) -> Result<(), TablePdfClosureError> {
    if observed.len() < expected.len() {
        return Err(TablePdfClosureError::MissingCell);
    }
    if observed.len() > expected.len() {
        return Err(TablePdfClosureError::ExtraCell);
    }
    for (actual, expected) in observed.iter().zip(expected) {
        if actual == expected {
            continue;
        }
        if actual.page_index != expected.page_index {
            return Err(TablePdfClosureError::WrongPage);
        }
        if actual.kind != expected.kind
            || actual.repetition_index != expected.repetition_index
            || actual.source_fragment_id != expected.source_fragment_id
        {
            return Err(TablePdfClosureError::WrongRepetition);
        }
        if actual.row_node_id != expected.row_node_id
            || actual.logical_row_ordinal != expected.logical_row_ordinal
            || actual.row_fragment_ordinal != expected.row_fragment_ordinal
            || actual.cell_node_id != expected.cell_node_id
            || actual.flow_id != expected.flow_id
            || actual.column_ordinal != expected.column_ordinal
            || actual.colspan != expected.colspan
            || actual.rowspan != expected.rowspan
        {
            return Err(TablePdfClosureError::WrongCell);
        }
        if actual.rect != expected.rect {
            return Err(TablePdfClosureError::WrongRectangle);
        }
        if actual.content_fragment_start != expected.content_fragment_start
            || actual.content_fragment_end != expected.content_fragment_end
        {
            return Err(TablePdfClosureError::WrongContentRange);
        }
        return Err(TablePdfClosureError::WrongCell);
    }
    Ok(())
}

fn encode_table_pdf_closure(value: &TablePdfClosureReceipt) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, TABLE_PDF_CLOSURE_ALGORITHM);
    output.push_str(",\"command_count\":");
    output.push_str(&value.commands.len().to_string());
    output.push_str(",\"decoration_op_count\":");
    output.push_str(&value.decoration_op_count.to_string());
    output.push_str(",\"display_sha256\":");
    push_json_hex(&mut output, &value.display_sha256);
    output.push_str(",\"object_count\":");
    output.push_str(&value.object_count.to_string());
    output.push_str(",\"page_count\":");
    output.push_str(&value.page_count.to_string());
    output.push_str(",\"pdf_byte_length\":");
    output.push_str(&value.pdf_byte_length.to_string());
    output.push_str(",\"pdf_sha256\":");
    push_json_hex(&mut output, &value.pdf_sha256);
    output.push_str(",\"records\":[");
    for (index, record) in value.records.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"cell_node_id\":");
        output.push_str(&record.cell_node_id.to_string());
        output.push_str(",\"column_ordinal\":");
        output.push_str(&record.column_ordinal.to_string());
        output.push_str(",\"colspan\":");
        output.push_str(&record.colspan.to_string());
        output.push_str(",\"content_fragment_end\":");
        output.push_str(&record.content_fragment_end.to_string());
        output.push_str(",\"content_fragment_start\":");
        output.push_str(&record.content_fragment_start.to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&record.flow_id.to_string());
        output.push_str(",\"fragment_id\":");
        output.push_str(&record.fragment_id.to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(
            &mut output,
            match record.kind {
                TablePaintOccurrenceKind::Header => "header",
                TablePaintOccurrenceKind::Body => "body",
            },
        );
        output.push_str(",\"logical_row_ordinal\":");
        output.push_str(&record.logical_row_ordinal.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&record.page_index.to_string());
        output.push_str(",\"rect\":{\"height\":");
        output.push_str(&record.rect.height().to_string());
        output.push_str(",\"width\":");
        output.push_str(&record.rect.width().to_string());
        output.push_str(",\"x\":");
        output.push_str(&record.rect.x().to_string());
        output.push_str(",\"y\":");
        output.push_str(&record.rect.y().to_string());
        output.push_str("},\"repetition_index\":");
        match record.repetition_index {
            Some(value) => output.push_str(&value.to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"row_fragment_ordinal\":");
        output.push_str(&record.row_fragment_ordinal.to_string());
        output.push_str(",\"row_node_id\":");
        output.push_str(&record.row_node_id.to_string());
        output.push_str(",\"rowspan\":");
        output.push_str(&record.rowspan.to_string());
        output.push_str(",\"source_fragment_id\":");
        match record.source_fragment_id {
            Some(value) => output.push_str(&value.to_string()),
            None => output.push_str("null"),
        }
        output.push('}');
    }
    output.push_str("],\"selected_layout_sha256\":");
    push_json_hex(&mut output, &value.selected_layout_sha256);
    output.push('}');
    output
}

pub const STAGING_MACHINE_BLOCK_STYLE_PDF_ALGORITHM: &str = "typaxis.machine-block-style-pdf/1";

/// PDF-stage observation used by the focused 1.2 block-style slice tests. It
/// is derived solely from the Display receipt; the public pipeline serializes
/// the complete basic-document Display document through the normal backend.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingMachineBlockStylePdf {
    display_sha256: [u8; 32],
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
    content_stream_observation: Vec<u8>,
    canonical_jcs: String,
}

impl StagingMachineBlockStylePdf {
    pub fn from_display(display: &StagingMachineBlockStyleDisplay) -> Self {
        let content_stream_observation = format!(
            "q\n{} 0 {} 1 re W n\nQ\n",
            display.paint_left_inset(),
            display.paint_inline_size()
        )
        .into_bytes();
        let mut value = Self {
            display_sha256: sha256(display.canonical_jcs().as_bytes()),
            package_sha256: display.package_sha256(),
            registry_version: display.registry_version(),
            owner_node_id: display.owner_node_id(),
            block_kind: display.block_kind().as_str(),
            frame_inline_size: display.frame_inline_size(),
            available_inline_size: display.available_inline_size(),
            paint_inline_size: display.paint_inline_size(),
            start_indent: display.start_indent(),
            end_indent: display.end_indent(),
            logical_start_alignment_space: display.logical_start_alignment_space(),
            logical_end_alignment_space: display.logical_end_alignment_space(),
            paint_left_inset: display.paint_left_inset(),
            effective_space_before: display.effective_space_before(),
            effective_space_after: display.effective_space_after(),
            page_break_before: display.page_break_before(),
            keep_with_next: display.keep_with_next(),
            keep_caption: display.keep_caption(),
            content_stream_observation,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_block_style_pdf(&value);
        value
    }

    pub const fn display_sha256(&self) -> [u8; 32] {
        self.display_sha256
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
    pub fn content_stream_observation(&self) -> &[u8] {
        &self.content_stream_observation
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn encode_staging_machine_block_style_pdf(value: &StagingMachineBlockStylePdf) -> String {
    let mut output = String::from("{\"algorithm\":\"");
    output.push_str(STAGING_MACHINE_BLOCK_STYLE_PDF_ALGORITHM);
    output.push_str("\",\"available_inline_size\":");
    output.push_str(&value.available_inline_size.to_string());
    output.push_str(",\"block_kind\":\"");
    output.push_str(value.block_kind);
    output.push_str("\",\"display_sha256\":\"");
    push_staging_pdf_hex(&mut output, value.display_sha256);
    output.push_str("\",\"effective_space_after\":");
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
    output.push_str(",\"package_sha256\":\"");
    push_staging_pdf_hex(&mut output, value.package_sha256);
    output.push_str("\",\"page_break_before\":");
    output.push_str(if value.page_break_before {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"paint_inline_size\":");
    output.push_str(&value.paint_inline_size.to_string());
    output.push_str(",\"paint_left_inset\":");
    output.push_str(&value.paint_left_inset.to_string());
    output.push_str(",\"registry_version\":\"");
    output.push_str(value.registry_version);
    output.push_str("\",\"start_indent\":");
    output.push_str(&value.start_indent.to_string());
    output.push('}');
    output
}

fn push_staging_pdf_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
}

fn push_json_hex(output: &mut String, bytes: &[u8; 32]) {
    output.push('"');
    push_staging_pdf_hex(output, *bytes);
    output.push('"');
}

pub const STAGING_MACHINE_FIGURE_PDF_ALGORITHM: &str = "typaxis.machine-figure-pdf/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMachineFigurePdfError {
    DisplayStateMismatch,
    PdfReceiptMismatch,
    PageClosure,
    ImageXObjectClosure,
    InvalidResourceName,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigurePdfXObject {
    image_id: ImageResourceId,
    resource_name: String,
}

impl StagingMachineFigurePdfXObject {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }
}

/// PDF-stage MI2-06 proof. Logical image closure comes from the frozen graph;
/// the serialized hash/length and Image XObject observation come from the
/// exact non-cloneable serializer receipt before any publication attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigurePdf {
    display: StagingMachineFigureDisplayFacts,
    display_sha256: [u8; 32],
    pdf_sha256: [u8; 32],
    pdf_byte_length: u64,
    page_count: u32,
    object_count: u32,
    image_xobject_count: u32,
    image_xobjects: Vec<StagingMachineFigurePdfXObject>,
    canonical_jcs: String,
}

impl StagingMachineFigurePdf {
    pub fn from_serialized(
        display: &StagingMachineFigureDisplayFacts,
        graph: &FrozenPdfGraph,
        receipt: &VerifiedPdfBytesReceipt,
    ) -> Result<Self, StagingMachineFigurePdfError> {
        if graph.selected_layout_fingerprint().bytes() != display.layout_state_sha256()
            || receipt.selected_layout_fingerprint() != graph.selected_layout_fingerprint()
        {
            return Err(StagingMachineFigurePdfError::DisplayStateMismatch);
        }
        if graph.page_count() != u32::try_from(display.pages().len()).unwrap_or(u32::MAX)
            || receipt.page_count() != graph.page_count()
        {
            return Err(StagingMachineFigurePdfError::PageClosure);
        }
        if receipt.object_count() != graph.object_count()
            || receipt.byte_length() == 0
            || sha256(receipt.bytes()) != receipt.content_hash()
        {
            return Err(StagingMachineFigurePdfError::PdfReceiptMismatch);
        }

        let expected: BTreeSet<_> = display
            .figures()
            .iter()
            .map(|figure| figure.image_id())
            .collect();
        let image_xobjects =
            close_staging_machine_figure_image_bindings(&expected, graph.image_resource_names())?;
        let image_xobject_count = close_staging_machine_figure_graph_images(graph)?;
        if image_xobject_count
            < u32::try_from(expected.len())
                .map_err(|_| StagingMachineFigurePdfError::ImageXObjectClosure)?
        {
            return Err(StagingMachineFigurePdfError::ImageXObjectClosure);
        }
        require_staging_serialized_image_xobjects(receipt.bytes(), image_xobject_count)?;
        let mut value = Self {
            display: display.clone(),
            display_sha256: sha256(display.canonical_jcs().as_bytes()),
            pdf_sha256: receipt.content_hash(),
            pdf_byte_length: receipt.byte_length(),
            page_count: receipt.page_count(),
            object_count: receipt.object_count(),
            image_xobject_count,
            image_xobjects,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_figure_pdf(&value);
        Ok(value)
    }

    pub const fn display(&self) -> &StagingMachineFigureDisplayFacts {
        &self.display
    }
    pub const fn display_sha256(&self) -> [u8; 32] {
        self.display_sha256
    }
    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub const fn pdf_byte_length(&self) -> u64 {
        self.pdf_byte_length
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub const fn image_xobject_count(&self) -> u32 {
        self.image_xobject_count
    }
    pub fn image_xobjects(&self) -> &[StagingMachineFigurePdfXObject] {
        &self.image_xobjects
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn close_staging_machine_figure_image_bindings<'a>(
    expected: &BTreeSet<ImageResourceId>,
    bindings: impl IntoIterator<Item = (ImageResourceId, &'a PdfName)>,
) -> Result<Vec<StagingMachineFigurePdfXObject>, StagingMachineFigurePdfError> {
    let mut observed = BTreeSet::new();
    let mut image_xobjects = Vec::new();
    for (image_id, name) in bindings {
        if !observed.insert(image_id) {
            return Err(StagingMachineFigurePdfError::ImageXObjectClosure);
        }
        let resource_name = String::from_utf8(name.encoded())
            .map_err(|_| StagingMachineFigurePdfError::InvalidResourceName)?;
        image_xobjects.push(StagingMachineFigurePdfXObject {
            image_id,
            resource_name,
        });
    }
    if &observed != expected {
        return Err(StagingMachineFigurePdfError::ImageXObjectClosure);
    }
    image_xobjects.sort_by_key(|binding| binding.image_id);
    Ok(image_xobjects)
}

fn close_staging_machine_figure_graph_images(
    graph: &FrozenPdfGraph,
) -> Result<u32, StagingMachineFigurePdfError> {
    let mut closed_objects = BTreeSet::new();
    for binding in &graph.image_bindings {
        if !closed_objects.insert(binding.object_id) {
            return Err(StagingMachineFigurePdfError::ImageXObjectClosure);
        }
        let Some(IndirectObjectBody::FrozenImageResource {
            plan,
            alpha_mask_object,
        }) = graph.graph.objects.get(&binding.object_id)
        else {
            return Err(StagingMachineFigurePdfError::ImageXObjectClosure);
        };
        if plan.image_id() != binding.logical_id
            || plan.alpha_mask().is_some() != alpha_mask_object.is_some()
        {
            return Err(StagingMachineFigurePdfError::ImageXObjectClosure);
        }
        if let (Some(expected_mask), Some(mask_object)) = (plan.alpha_mask(), *alpha_mask_object) {
            if !closed_objects.insert(mask_object)
                || !matches!(
                    graph.graph.objects.get(&mask_object),
                    Some(IndirectObjectBody::FrozenImageAlphaMask(actual_mask))
                        if actual_mask == expected_mask
                )
            {
                return Err(StagingMachineFigurePdfError::ImageXObjectClosure);
            }
        }
    }
    let graph_image_objects: BTreeSet<_> = graph
        .graph
        .objects
        .iter()
        .filter_map(|(object_id, body)| {
            matches!(
                body,
                IndirectObjectBody::FrozenImageResource { .. }
                    | IndirectObjectBody::FrozenImageAlphaMask(_)
            )
            .then_some(*object_id)
        })
        .collect();
    if graph_image_objects != closed_objects {
        return Err(StagingMachineFigurePdfError::ImageXObjectClosure);
    }
    u32::try_from(closed_objects.len())
        .map_err(|_| StagingMachineFigurePdfError::ImageXObjectClosure)
}

fn require_staging_serialized_image_xobjects(
    pdf_bytes: &[u8],
    expected_graph_image_count: u32,
) -> Result<(), StagingMachineFigurePdfError> {
    let serialized_marker_count = u32::try_from(
        pdf_bytes
            .windows(b"/Subtype /Image".len())
            .filter(|window| *window == b"/Subtype /Image")
            .count(),
    )
    .map_err(|_| StagingMachineFigurePdfError::ImageXObjectClosure)?;
    if serialized_marker_count < expected_graph_image_count {
        return Err(StagingMachineFigurePdfError::ImageXObjectClosure);
    }
    Ok(())
}

fn encode_staging_machine_figure_pdf(value: &StagingMachineFigurePdf) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_FIGURE_PDF_ALGORITHM);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"display_sha256\":");
    push_json_hex(&mut output, &value.display_sha256);
    output.push_str(",\"image_xobject_count\":");
    output.push_str(&value.image_xobject_count.to_string());
    output.push_str(",\"image_xobjects\":[");
    for (index, binding) in value.image_xobjects.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"image_id\":");
        output.push_str(&binding.image_id.get().to_string());
        output.push_str(",\"resource_name\":");
        push_jcs_string(&mut output, &binding.resource_name);
        output.push('}');
    }
    output.push_str("],\"object_count\":");
    output.push_str(&value.object_count.to_string());
    output.push_str(",\"page_count\":");
    output.push_str(&value.page_count.to_string());
    output.push_str(",\"pdf_byte_length\":");
    output.push_str(&value.pdf_byte_length.to_string());
    output.push_str(",\"pdf_sha256\":");
    push_json_hex(&mut output, &value.pdf_sha256);
    output.push('}');
    output
}

pub const STAGING_MACHINE_LINK_PDF_ALGORITHM: &str = "typaxis.machine-link-pdf/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMachineLinkPdfError {
    DisplayStateMismatch,
    PdfReceiptMismatch,
    PageClosure,
    DestinationClosure,
    AnnotationClosure,
    SerializedClosure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineLinkPdfAnnotation {
    link_node_id: u32,
    paragraph_node_id: u32,
    page_index: u32,
    line_ordinal: u32,
    rect: Rect,
    target: StagingMachineLinkDisplayTarget,
    object_id: u32,
}

impl StagingMachineLinkPdfAnnotation {
    pub const fn link_node_id(&self) -> u32 {
        self.link_node_id
    }
    pub const fn paragraph_node_id(&self) -> u32 {
        self.paragraph_node_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn line_ordinal(&self) -> u32 {
        self.line_ordinal
    }
    pub const fn rect(&self) -> Rect {
        self.rect
    }
    pub const fn target(&self) -> &StagingMachineLinkDisplayTarget {
        &self.target
    }
    pub const fn object_id(&self) -> u32 {
        self.object_id
    }
}

/// PDF-stage MI2-07 proof. It reopens the frozen graph to compare every page
/// `/Annots` reference and annotation dictionary with the selected Display
/// rectangle/target facts, and compares the complete destination name tree
/// before retaining the serializer hash.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineLinkPdf {
    display: StagingMachineLinkDisplayFacts,
    display_sha256: [u8; 32],
    pdf_sha256: [u8; 32],
    pdf_byte_length: u64,
    page_count: u32,
    object_count: u32,
    destination_count: u32,
    annotation_count: u32,
    annotations: Vec<StagingMachineLinkPdfAnnotation>,
    canonical_jcs: String,
}

impl StagingMachineLinkPdf {
    pub fn from_serialized(
        display: &StagingMachineLinkDisplayFacts,
        graph: &FrozenPdfGraph,
        receipt: &VerifiedPdfBytesReceipt,
    ) -> Result<Self, StagingMachineLinkPdfError> {
        if graph.selected_layout_fingerprint().bytes() != display.layout_state_sha256()
            || receipt.selected_layout_fingerprint() != graph.selected_layout_fingerprint()
        {
            return Err(StagingMachineLinkPdfError::DisplayStateMismatch);
        }
        if graph.page_count() != u32::try_from(display.pages().len()).unwrap_or(u32::MAX)
            || receipt.page_count() != graph.page_count()
            || graph
                .pages()
                .iter()
                .zip(display.pages())
                .any(|(graph, display)| {
                    graph.page_index() != display.page_index()
                        || graph.width() != display.width()
                        || graph.height() != display.height()
                })
        {
            return Err(StagingMachineLinkPdfError::PageClosure);
        }
        if receipt.object_count() != graph.object_count()
            || receipt.byte_length() == 0
            || sha256(receipt.bytes()) != receipt.content_hash()
        {
            return Err(StagingMachineLinkPdfError::PdfReceiptMismatch);
        }

        let page_ids = staging_machine_link_page_ids(graph)?;
        close_staging_machine_link_destinations(display, graph, &page_ids)?;
        let annotations = close_staging_machine_link_annotations(display, graph, &page_ids)?;
        let annotation_count = u32::try_from(annotations.len())
            .map_err(|_| StagingMachineLinkPdfError::AnnotationClosure)?;
        if annotation_count != display.annotation_count() {
            return Err(StagingMachineLinkPdfError::AnnotationClosure);
        }
        require_staging_serialized_link_annotations(receipt.bytes(), annotation_count)?;
        let destination_count = u32::try_from(display.destinations().len())
            .map_err(|_| StagingMachineLinkPdfError::DestinationClosure)?;
        let mut value = Self {
            display: display.clone(),
            display_sha256: sha256(display.canonical_jcs().as_bytes()),
            pdf_sha256: receipt.content_hash(),
            pdf_byte_length: receipt.byte_length(),
            page_count: receipt.page_count(),
            object_count: receipt.object_count(),
            destination_count,
            annotation_count,
            annotations,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_link_pdf(&value);
        Ok(value)
    }

    pub const fn display(&self) -> &StagingMachineLinkDisplayFacts {
        &self.display
    }
    pub const fn display_sha256(&self) -> [u8; 32] {
        self.display_sha256
    }
    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub const fn pdf_byte_length(&self) -> u64 {
        self.pdf_byte_length
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub const fn destination_count(&self) -> u32 {
        self.destination_count
    }
    pub const fn annotation_count(&self) -> u32 {
        self.annotation_count
    }
    pub fn annotations(&self) -> &[StagingMachineLinkPdfAnnotation] {
        &self.annotations
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn staging_machine_link_page_ids(
    graph: &FrozenPdfGraph,
) -> Result<Vec<ObjectId>, StagingMachineLinkPdfError> {
    let catalog = dictionary_for(&graph.graph.objects, graph.graph.root)
        .map_err(|_| StagingMachineLinkPdfError::PageClosure)?;
    let pages_id = match dict_value(catalog, b"Pages") {
        Some(PdfValue::Reference(id)) => *id,
        _ => return Err(StagingMachineLinkPdfError::PageClosure),
    };
    let pages = dictionary_for(&graph.graph.objects, pages_id)
        .map_err(|_| StagingMachineLinkPdfError::PageClosure)?;
    let kids = match dict_value(pages, b"Kids") {
        Some(PdfValue::Array(kids)) => kids,
        _ => return Err(StagingMachineLinkPdfError::PageClosure),
    };
    let page_ids = kids
        .iter()
        .map(|kid| match kid {
            PdfValue::Reference(id)
                if dictionary_for(&graph.graph.objects, *id)
                    .is_ok_and(|page| type_is(page, b"Page")) =>
            {
                Ok(*id)
            }
            _ => Err(StagingMachineLinkPdfError::PageClosure),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if page_ids.len() != graph.pages.len() {
        return Err(StagingMachineLinkPdfError::PageClosure);
    }
    Ok(page_ids)
}

fn close_staging_machine_link_destinations(
    display: &StagingMachineLinkDisplayFacts,
    graph: &FrozenPdfGraph,
    page_ids: &[ObjectId],
) -> Result<(), StagingMachineLinkPdfError> {
    let catalog = dictionary_for(&graph.graph.objects, graph.graph.root)
        .map_err(|_| StagingMachineLinkPdfError::DestinationClosure)?;
    if display.destinations().is_empty() {
        return if dict_value(catalog, b"Names").is_none() {
            Ok(())
        } else {
            Err(StagingMachineLinkPdfError::DestinationClosure)
        };
    }
    let names = match dict_value(catalog, b"Names") {
        Some(PdfValue::Dictionary(names)) if names.len() == 1 => names,
        _ => return Err(StagingMachineLinkPdfError::DestinationClosure),
    };
    let destinations = match dict_value(names, b"Dests") {
        Some(PdfValue::Dictionary(destinations)) if destinations.len() == 1 => destinations,
        _ => return Err(StagingMachineLinkPdfError::DestinationClosure),
    };
    let expected_value_count = display
        .destinations()
        .len()
        .checked_mul(2)
        .ok_or(StagingMachineLinkPdfError::DestinationClosure)?;
    let values = match dict_value(destinations, b"Names") {
        Some(PdfValue::Array(values)) if values.len() == expected_value_count => values,
        _ => return Err(StagingMachineLinkPdfError::DestinationClosure),
    };
    for (destination, pair) in display.destinations().iter().zip(values.chunks_exact(2)) {
        if pair[0] != PdfValue::ByteString(destination.anchor_id().as_str().as_bytes().to_vec()) {
            return Err(StagingMachineLinkPdfError::DestinationClosure);
        }
        let page_index = usize::try_from(destination.page_index())
            .map_err(|_| StagingMachineLinkPdfError::DestinationClosure)?;
        let named = NamedDestination {
            anchor_id: destination.anchor_id().clone(),
            page_index: destination.page_index(),
            view: DestinationView::Xyz {
                point: destination.point(),
            },
        };
        let expected = destination_array(
            &named,
            *page_ids
                .get(page_index)
                .ok_or(StagingMachineLinkPdfError::DestinationClosure)?,
            display
                .pages()
                .get(page_index)
                .ok_or(StagingMachineLinkPdfError::DestinationClosure)?
                .height(),
        )
        .map_err(|_| StagingMachineLinkPdfError::DestinationClosure)?;
        if pair[1] != expected {
            return Err(StagingMachineLinkPdfError::DestinationClosure);
        }
    }
    Ok(())
}

fn staging_machine_link_annotation(fact: &StagingMachineLinkDisplayRectangle) -> LinkAnnotation {
    let target = match fact.target() {
        StagingMachineLinkDisplayTarget::Internal { anchor_id, .. } => {
            LinkTarget::Internal(anchor_id.clone())
        }
        StagingMachineLinkDisplayTarget::External { uri } => LinkTarget::Uri(uri.clone()),
    };
    LinkAnnotation {
        target,
        rect: fact.rect(),
    }
}

fn close_staging_machine_link_annotations(
    display: &StagingMachineLinkDisplayFacts,
    graph: &FrozenPdfGraph,
    page_ids: &[ObjectId],
) -> Result<Vec<StagingMachineLinkPdfAnnotation>, StagingMachineLinkPdfError> {
    let mut object_by_key = BTreeMap::new();
    let mut closed_annotation_ids = BTreeSet::new();
    for (page_index, page_id) in page_ids.iter().copied().enumerate() {
        let page = dictionary_for(&graph.graph.objects, page_id)
            .map_err(|_| StagingMachineLinkPdfError::AnnotationClosure)?;
        let expected_facts: Vec<_> = display
            .annotations()
            .filter(|annotation| annotation.page_index() as usize == page_index)
            .collect();
        let expected_annotations: Vec<_> = expected_facts
            .iter()
            .map(|annotation| staging_machine_link_annotation(annotation))
            .collect();
        let annotation_ids: Vec<_> = match dict_value(page, b"Annots") {
            None if expected_annotations.is_empty() => Vec::new(),
            Some(PdfValue::Array(values)) if values.len() == expected_annotations.len() => values
                .iter()
                .map(|value| match value {
                    PdfValue::Reference(id) => Ok(*id),
                    _ => Err(StagingMachineLinkPdfError::AnnotationClosure),
                })
                .collect::<Result<_, _>>()?,
            _ => return Err(StagingMachineLinkPdfError::AnnotationClosure),
        };
        let content_id = match dict_value(page, b"Contents") {
            Some(PdfValue::Reference(id)) => *id,
            _ => return Err(StagingMachineLinkPdfError::AnnotationClosure),
        };
        let display_page = match graph.graph.objects.get(&content_id) {
            Some(IndirectObjectBody::DisplayPageContent(page)) => page,
            _ => return Err(StagingMachineLinkPdfError::AnnotationClosure),
        };
        if display_page.page_index as usize != page_index
            || display_page.annotations != expected_annotations
        {
            return Err(StagingMachineLinkPdfError::AnnotationClosure);
        }
        let page_height = display
            .pages()
            .get(page_index)
            .ok_or(StagingMachineLinkPdfError::PageClosure)?
            .height();
        for ((fact, annotation), object_id) in expected_facts
            .iter()
            .zip(&expected_annotations)
            .zip(annotation_ids)
        {
            if !closed_annotation_ids.insert(object_id) {
                return Err(StagingMachineLinkPdfError::AnnotationClosure);
            }
            let expected_dictionary = annotation_dictionary(annotation, page_height)
                .map_err(|_| StagingMachineLinkPdfError::AnnotationClosure)?;
            if !matches!(
                graph.graph.objects.get(&object_id),
                Some(IndirectObjectBody::Value(PdfValue::Dictionary(actual)))
                    if actual == &expected_dictionary
            ) {
                return Err(StagingMachineLinkPdfError::AnnotationClosure);
            }
            let key = (fact.link_node_id(), fact.page_index(), fact.line_ordinal());
            if object_by_key.insert(key, object_id).is_some() {
                return Err(StagingMachineLinkPdfError::AnnotationClosure);
            }
        }
    }
    let graph_annotation_ids: BTreeSet<_> = graph
        .graph
        .objects
        .iter()
        .filter_map(|(id, body)| match body {
            IndirectObjectBody::Value(PdfValue::Dictionary(dictionary))
                if type_is(dictionary, b"Annot") =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect();
    if graph_annotation_ids != closed_annotation_ids {
        return Err(StagingMachineLinkPdfError::AnnotationClosure);
    }
    display
        .annotations()
        .map(|fact| {
            let object_id = object_by_key
                .get(&(fact.link_node_id(), fact.page_index(), fact.line_ordinal()))
                .ok_or(StagingMachineLinkPdfError::AnnotationClosure)?;
            Ok(StagingMachineLinkPdfAnnotation {
                link_node_id: fact.link_node_id(),
                paragraph_node_id: fact.paragraph_node_id(),
                page_index: fact.page_index(),
                line_ordinal: fact.line_ordinal(),
                rect: fact.rect(),
                target: fact.target().clone(),
                object_id: object_id.get(),
            })
        })
        .collect()
}

fn require_staging_serialized_link_annotations(
    pdf_bytes: &[u8],
    expected_count: u32,
) -> Result<(), StagingMachineLinkPdfError> {
    let marker = b"/Subtype /Link";
    let count = u32::try_from(
        pdf_bytes
            .windows(marker.len())
            .filter(|window| *window == marker)
            .count(),
    )
    .map_err(|_| StagingMachineLinkPdfError::SerializedClosure)?;
    if count != expected_count {
        return Err(StagingMachineLinkPdfError::SerializedClosure);
    }
    Ok(())
}

fn encode_staging_machine_link_pdf(value: &StagingMachineLinkPdf) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_LINK_PDF_ALGORITHM);
    output.push_str(",\"annotation_count\":");
    output.push_str(&value.annotation_count.to_string());
    output.push_str(",\"annotations\":[");
    for (index, annotation) in value.annotations.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"line_ordinal\":");
        output.push_str(&annotation.line_ordinal.to_string());
        output.push_str(",\"link_node_id\":");
        output.push_str(&annotation.link_node_id.to_string());
        output.push_str(",\"object_id\":");
        output.push_str(&annotation.object_id.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&annotation.page_index.to_string());
        output.push('}');
    }
    output.push_str("],\"contract\":\"typaxis.contract/1.2\",\"destination_count\":");
    output.push_str(&value.destination_count.to_string());
    output.push_str(",\"display_sha256\":");
    push_json_hex(&mut output, &value.display_sha256);
    output.push_str(",\"object_count\":");
    output.push_str(&value.object_count.to_string());
    output.push_str(",\"page_count\":");
    output.push_str(&value.page_count.to_string());
    output.push_str(",\"pdf_byte_length\":");
    output.push_str(&value.pdf_byte_length.to_string());
    output.push_str(",\"pdf_sha256\":");
    push_json_hex(&mut output, &value.pdf_sha256);
    output.push('}');
    output
}

pub const STAGING_FORCED_PAGE_BREAK_PDF_ALGORITHM: &str = "typaxis.forced-page-break-pdf/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakPdfPage {
    page_index: u32,
    painted_content_count: u32,
}

impl StagingForcedPageBreakPdfPage {
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
pub struct StagingForcedPageBreakPdfBoundary {
    break_node_id: u32,
    document_ordinal: u32,
    flow_id: u32,
    before_flow_local_ordinal: u32,
    after_flow_local_ordinal: u32,
    produced_page_index: u32,
}

impl StagingForcedPageBreakPdfBoundary {
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

/// PDF-stage forced-boundary observation. Page-tree count is materialized,
/// while page breaks themselves contribute no content-stream operation.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakPdf {
    display_sha256: [u8; 32],
    package_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    usage_sha256: [u8; 32],
    policy_version: &'static str,
    page_count: u32,
    pages: Vec<StagingForcedPageBreakPdfPage>,
    breaks: Vec<StagingForcedPageBreakPdfBoundary>,
    page_tree_observation: Vec<u8>,
    canonical_jcs: String,
}

impl StagingForcedPageBreakPdf {
    pub fn from_display(display: &StagingForcedPageBreakDisplay) -> Self {
        debug_assert_eq!(display.paint_operation_count(), 0);
        let pages = display
            .pages()
            .iter()
            .map(|page| StagingForcedPageBreakPdfPage {
                page_index: page.page_index(),
                painted_content_count: page.painted_content_count(),
            })
            .collect();
        let breaks = display
            .breaks()
            .iter()
            .map(|boundary| StagingForcedPageBreakPdfBoundary {
                break_node_id: boundary.break_node_id(),
                document_ordinal: boundary.document_ordinal(),
                flow_id: boundary.flow_id(),
                before_flow_local_ordinal: boundary.before_flow_local_ordinal(),
                after_flow_local_ordinal: boundary.after_flow_local_ordinal(),
                produced_page_index: boundary.produced_page_index(),
            })
            .collect();
        let page_tree_observation = format!("/Count {}\n", display.page_count()).into_bytes();
        let mut value = Self {
            display_sha256: sha256(display.canonical_jcs().as_bytes()),
            package_sha256: display.package_sha256(),
            flow_registry_sha256: display.flow_registry_sha256(),
            usage_sha256: display.usage_sha256(),
            policy_version: display.policy_version(),
            page_count: display.page_count(),
            pages,
            breaks,
            page_tree_observation,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_forced_page_break_pdf(&value);
        value
    }

    pub const fn display_sha256(&self) -> [u8; 32] {
        self.display_sha256
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

    pub const fn policy_version(&self) -> &'static str {
        self.policy_version
    }

    pub const fn page_count(&self) -> u32 {
        self.page_count
    }

    pub fn pages(&self) -> &[StagingForcedPageBreakPdfPage] {
        &self.pages
    }

    pub fn breaks(&self) -> &[StagingForcedPageBreakPdfBoundary] {
        &self.breaks
    }

    pub fn page_tree_observation(&self) -> &[u8] {
        &self.page_tree_observation
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn encode_staging_forced_page_break_pdf(value: &StagingForcedPageBreakPdf) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_FORCED_PAGE_BREAK_PDF_ALGORITHM);
    output.push_str(",\"break_usage_sha256\":");
    push_json_hex(&mut output, &value.usage_sha256);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"display_sha256\":");
    push_json_hex(&mut output, &value.display_sha256);
    output.push_str(",\"flow_registry_sha256\":");
    push_json_hex(&mut output, &value.flow_registry_sha256);
    output.push_str(",\"forced_page_breaks\":[");
    for (index, boundary) in value.breaks.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_staging_forced_page_break_pdf_boundary(&mut output, boundary);
    }
    output.push_str("],\"package_sha256\":");
    push_json_hex(&mut output, &value.package_sha256);
    output.push_str(",\"page_count\":");
    output.push_str(&value.page_count.to_string());
    output.push_str(",\"page_tree_sha256\":");
    push_json_hex(&mut output, &sha256(&value.page_tree_observation));
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
    output.push_str("],\"policy_version\":");
    push_jcs_string(&mut output, value.policy_version);
    output.push('}');
    output
}

fn encode_staging_forced_page_break_pdf_boundary(
    output: &mut String,
    boundary: &StagingForcedPageBreakPdfBoundary,
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

pub const STAGING_MACHINE_LIST_PDF_ALGORITHM: &str = "typaxis.machine-list-pdf/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListPdfList {
    list_node_id: u32,
    list_flow_id: u32,
    marker_column_width: i64,
    marker_gap: i64,
    start_indent: i64,
    end_indent: i64,
    item_frame_inline_size: i64,
}

impl StagingMachineListPdfList {
    pub const fn list_node_id(&self) -> u32 {
        self.list_node_id
    }
    pub const fn list_flow_id(&self) -> u32 {
        self.list_flow_id
    }
    pub const fn marker_column_width(&self) -> i64 {
        self.marker_column_width
    }
    pub const fn marker_gap(&self) -> i64 {
        self.marker_gap
    }
    pub const fn start_indent(&self) -> i64 {
        self.start_indent
    }
    pub const fn end_indent(&self) -> i64 {
        self.end_indent
    }
    pub const fn item_frame_inline_size(&self) -> i64 {
        self.item_frame_inline_size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListPdfItem {
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

impl StagingMachineListPdfItem {
    pub const fn list_node_id(&self) -> u32 {
        self.list_node_id
    }
    pub const fn item_node_id(&self) -> u32 {
        self.item_node_id
    }
    pub const fn item_index(&self) -> u32 {
        self.item_index
    }
    pub const fn list_flow_id(&self) -> u32 {
        self.list_flow_id
    }
    pub const fn item_flow_id(&self) -> u32 {
        self.item_flow_id
    }
    pub const fn marker_key(&self) -> GeneratedBufferKey {
        self.marker_key
    }
    pub fn marker_utf8(&self) -> &str {
        &self.marker_utf8
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
    pub fn fragment_ids(&self) -> &[u64] {
        &self.fragment_ids
    }
    pub const fn marker_inline_size(&self) -> i64 {
        self.marker_inline_size
    }
    pub const fn marker_column_width(&self) -> i64 {
        self.marker_column_width
    }
    pub const fn marker_physical_left(&self) -> i64 {
        self.marker_physical_left
    }
    pub const fn content_physical_left(&self) -> i64 {
        self.content_physical_left
    }
    pub const fn content_inline_size(&self) -> i64 {
        self.content_inline_size
    }
    pub const fn first_line_inline_size(&self) -> i64 {
        self.first_line_inline_size
    }
    pub const fn first_line_block_size(&self) -> i64 {
        self.first_line_block_size
    }
    pub const fn block_offset(&self) -> i64 {
        self.block_offset
    }
}

/// PDF-stage list observation. The marker text operation is produced from the
/// Display-owned generated buffer and retains the exact selected fragment.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingMachineListPdf {
    display_sha256: [u8; 32],
    package_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    marker_usage_sha256: [u8; 32],
    policy_version: &'static str,
    page_count: u32,
    lists: Vec<StagingMachineListPdfList>,
    items: Vec<StagingMachineListPdfItem>,
    content_stream_observation: Vec<u8>,
    canonical_jcs: String,
}

impl StagingMachineListPdf {
    pub fn from_display(display: &StagingMachineListDisplay) -> Self {
        let lists = display
            .lists()
            .iter()
            .map(|list| StagingMachineListPdfList {
                list_node_id: list.list_node_id(),
                list_flow_id: list.list_flow_id(),
                marker_column_width: list.marker_column_width(),
                marker_gap: list.marker_gap(),
                start_indent: list.start_indent(),
                end_indent: list.end_indent(),
                item_frame_inline_size: list.item_frame_inline_size(),
            })
            .collect();
        let items: Vec<_> = display
            .items()
            .iter()
            .map(|item| StagingMachineListPdfItem {
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
        let content_stream_observation = encode_staging_machine_list_content(&items);
        let mut value = Self {
            display_sha256: sha256(display.canonical_jcs().as_bytes()),
            package_sha256: display.package_sha256(),
            flow_registry_sha256: display.flow_registry_sha256(),
            marker_usage_sha256: display.marker_usage_sha256(),
            policy_version: display.policy_version(),
            page_count: display.page_count(),
            lists,
            items,
            content_stream_observation,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_list_pdf(&value);
        value
    }

    pub const fn display_sha256(&self) -> [u8; 32] {
        self.display_sha256
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
    pub const fn policy_version(&self) -> &'static str {
        self.policy_version
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub fn lists(&self) -> &[StagingMachineListPdfList] {
        &self.lists
    }
    pub fn items(&self) -> &[StagingMachineListPdfItem] {
        &self.items
    }
    pub fn content_stream_observation(&self) -> &[u8] {
        &self.content_stream_observation
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn encode_staging_machine_list_content(items: &[StagingMachineListPdfItem]) -> Vec<u8> {
    let mut output = String::new();
    for item in items {
        output.push_str("q\n% item ");
        output.push_str(&item.item_node_id.to_string());
        output.push_str(" flow ");
        output.push_str(&item.item_flow_id.to_string());
        output.push_str(" fragment ");
        output.push_str(&item.marker_fragment_id.to_string());
        output.push_str(" page ");
        output.push_str(&item.page_index.to_string());
        output.push_str("\nBT ");
        output.push_str(&item.marker_physical_left.to_string());
        output.push(' ');
        output.push_str(&item.block_offset.to_string());
        output.push_str(" Td <");
        push_bytes_hex(&mut output, item.marker_utf8.as_bytes());
        output.push_str("> Tj ET\nQ\n");
    }
    output.into_bytes()
}

fn encode_staging_machine_list_pdf(value: &StagingMachineListPdf) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_LIST_PDF_ALGORITHM);
    output.push_str(",\"content_stream_sha256\":\"");
    push_staging_pdf_hex(&mut output, sha256(&value.content_stream_observation));
    output.push_str("\",\"contract\":\"typaxis.contract/1.2\",\"display_sha256\":\"");
    push_staging_pdf_hex(&mut output, value.display_sha256);
    output.push_str("\",\"flow_registry_sha256\":\"");
    push_staging_pdf_hex(&mut output, value.flow_registry_sha256);
    output.push_str("\",\"items\":[");
    for (index, item) in value.items.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_staging_machine_list_pdf_item(&mut output, item);
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
    output.push_str("],\"marker_usage_sha256\":\"");
    push_staging_pdf_hex(&mut output, value.marker_usage_sha256);
    output.push_str("\",\"package_sha256\":\"");
    push_staging_pdf_hex(&mut output, value.package_sha256);
    output.push_str("\",\"page_count\":");
    output.push_str(&value.page_count.to_string());
    output.push_str(",\"policy_version\":");
    push_jcs_string(&mut output, value.policy_version);
    output.push('}');
    output
}

fn encode_staging_machine_list_pdf_item(output: &mut String, item: &StagingMachineListPdfItem) {
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

fn push_bytes_hex(output: &mut String, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectId(u32);
impl ObjectId {
    pub const fn new(value: u32) -> Option<Self> {
        if value == 0 {
            None
        } else {
            Some(Self(value))
        }
    }
    pub const fn get(self) -> u32 {
        self.0
    }
}
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PdfName(Vec<u8>);
impl PdfName {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, PdfError> {
        let bytes = bytes.into();
        if bytes.is_empty() || bytes.contains(&0) {
            Err(PdfError::InvalidName)
        } else {
            Ok(Self(bytes))
        }
    }
    pub fn encoded(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(self.0.len() + 1);
        output.push(b'/');
        for &byte in &self.0 {
            let regular = (33..=126).contains(&byte) && !b"()<>[]{}/%#".contains(&byte);
            if regular {
                output.push(byte);
            } else {
                output.extend_from_slice(format!("#{byte:02X}").as_bytes());
            }
        }
        output
    }
    fn is(&self, value: &[u8]) -> bool {
        self.0 == value
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdfDecimal {
    pub coefficient: i64,
    pub scale: u8,
}
impl PdfDecimal {
    pub fn new(coefficient: i64, scale: u8) -> Result<Self, PdfError> {
        if scale > 12 {
            Err(PdfError::DecimalScaleTooLarge)
        } else {
            Ok(Self { coefficient, scale })
        }
    }
    pub fn canonical(self) -> String {
        if self.coefficient == 0 {
            return "0".to_owned();
        }
        if self.scale == 0 {
            return self.coefficient.to_string();
        }
        let negative = self.coefficient < 0;
        let digits = self.coefficient.unsigned_abs().to_string();
        let scale = usize::from(self.scale);
        let mut output = if digits.len() <= scale {
            format!("0.{}{}", "0".repeat(scale - digits.len()), digits)
        } else {
            let split = digits.len() - scale;
            format!("{}.{}", &digits[..split], &digits[split..])
        };
        while output.ends_with('0') {
            output.pop();
        }
        if output.ends_with('.') {
            output.pop();
        }
        if negative {
            output.insert(0, '-');
        }
        output
    }
}
pub type PdfDictionary = BTreeMap<PdfName, PdfValue>;
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PdfValue {
    Null,
    Bool(bool),
    Integer(i64),
    Decimal(PdfDecimal),
    Name(PdfName),
    ByteString(Vec<u8>),
    Array(Vec<PdfValue>),
    Dictionary(PdfDictionary),
    Reference(ObjectId),
}
impl Drop for PdfValue {
    fn drop(&mut self) {
        fn take_children(value: &mut PdfValue, pending: &mut Vec<PdfValue>) {
            match value {
                PdfValue::Array(values) => pending.append(values),
                PdfValue::Dictionary(dictionary) => {
                    pending.extend(std::mem::take(dictionary).into_values());
                }
                _ => {}
            }
        }

        let mut pending = Vec::new();
        take_children(self, &mut pending);
        while let Some(mut value) = pending.pop() {
            take_children(&mut value, &mut pending);
            // `value` now owns no recursive children and is safe to drop.
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamEncoding {
    None,
    Flate,
    EncodedFlate,
    Dct,
}
/// `raw_data` is unencoded for `None`/`Flate`; the two encoded variants carry
/// bytes from a sealed image-encoder receipt. The serializer always owns
/// `/Length`, `/Filter`, and `/DecodeParms`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfStreamObject {
    pub dictionary: PdfDictionary,
    pub encoding: StreamEncoding,
    pub raw_data: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IndirectObjectBody {
    Value(PdfValue),
    Stream(PdfStreamObject),
    /// The sealed subset payload and all late-finalizer facts. The surrounding
    /// Type0/CIDFont/descriptor dictionaries refer to this object; the two
    /// mapping objects find their canonical data through this object ID.
    FrozenFontProgram(FrozenPdfFontPlan),
    FrozenToUnicodeCMap {
        font_program_object: ObjectId,
    },
    FrozenCidToGidMap {
        font_program_object: ObjectId,
    },
    FrozenImageResource {
        plan: FrozenPdfImagePlan,
        alpha_mask_object: Option<ObjectId>,
    },
    FrozenImageAlphaMask(FrozenPdfAlphaMask),
    DisplayPageContent(DisplayPage),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PdfError {
    InvalidName,
    DecimalScaleTooLarge,
    DuplicateObject,
    MissingRoot(ObjectId),
    MissingReference(ObjectId),
    ReservedStreamKey,
    RootIsNotCatalog,
    CatalogMissingPages,
    InvalidPageTree,
    PageTreeCycle,
    OutputTooLarge,
    SparseObjectId,
    UnreachableObject(ObjectId),
    ObjectLimit,
    ObjectCountOverflow,
    SelectedLayoutMismatch,
    SelectedPageClosure,
    PageMasterMismatch,
    ResourcePlanMismatch,
    InvalidDestinationClosure,
    InvalidAnnotationClosure,
    TableClosure,
    FootnoteClosure,
    DirectValueDepth,
    PageTreeDepth,
    ContentStream,
}
/// Low-level object graph assembly API. The resulting value is explicitly
/// untrusted and cannot be converted into the publication `FrozenPdfGraph`.
#[derive(Clone, Debug)]
pub struct UntrustedPdfObjectGraphBuilder {
    objects: BTreeMap<ObjectId, IndirectObjectBody>,
    max_objects: u32,
}
impl UntrustedPdfObjectGraphBuilder {
    pub fn new(limits: &ValidatedResourceLimits) -> Self {
        Self {
            objects: BTreeMap::new(),
            max_objects: limits.get().max_pdf_objects,
        }
    }
    pub fn insert(&mut self, id: ObjectId, body: IndirectObjectBody) -> Result<(), PdfError> {
        if self.objects.contains_key(&id) {
            return Err(PdfError::DuplicateObject);
        }
        if self.objects.len() >= self.max_objects as usize {
            return Err(PdfError::ObjectLimit);
        }
        match self.objects.entry(id) {
            Entry::Vacant(slot) => {
                slot.insert(body);
                Ok(())
            }
            Entry::Occupied(_) => Err(PdfError::DuplicateObject),
        }
    }
    pub fn validate_untrusted(
        self,
        root: ObjectId,
    ) -> Result<ValidatedUntrustedPdfObjectGraph, PdfError> {
        if !self.objects.contains_key(&root) {
            return Err(PdfError::MissingRoot(root));
        }
        for (index, id) in self.objects.keys().enumerate() {
            let expected = u32::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1));
            if expected != Some(id.get()) {
                return Err(PdfError::SparseObjectId);
            }
        }
        for body in self.objects.values() {
            if let IndirectObjectBody::Stream(stream) = body {
                for key in stream.dictionary.keys() {
                    if key.is(b"Length") || key.is(b"Filter") || key.is(b"DecodeParms") {
                        return Err(PdfError::ReservedStreamKey);
                    }
                }
            }
        }
        let mut references = BTreeSet::new();
        for body in self.objects.values() {
            collect_references(body, &mut references)?;
        }
        for id in references {
            if !self.objects.contains_key(&id) {
                return Err(PdfError::MissingReference(id));
            }
        }
        let reachable = collect_reachable(&self.objects, root)?;
        if let Some(unreachable) = self
            .objects
            .keys()
            .find(|id| !reachable.contains(id))
            .copied()
        {
            return Err(PdfError::UnreachableObject(unreachable));
        }
        validate_page_tree(&self.objects, root)?;
        Ok(ValidatedUntrustedPdfObjectGraph {
            root,
            objects: self.objects,
        })
    }
}

fn collect_reachable(
    objects: &BTreeMap<ObjectId, IndirectObjectBody>,
    root: ObjectId,
) -> Result<BTreeSet<ObjectId>, PdfError> {
    let mut reachable = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if !reachable.insert(id) {
            continue;
        }
        if let Some(body) = objects.get(&id) {
            let mut references = BTreeSet::new();
            collect_references(body, &mut references)?;
            pending.extend(references);
        }
    }
    Ok(reachable)
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedUntrustedPdfObjectGraph {
    root: ObjectId,
    objects: BTreeMap<ObjectId, IndirectObjectBody>,
}
impl ValidatedUntrustedPdfObjectGraph {
    pub const fn root(&self) -> ObjectId {
        self.root
    }
    pub fn iter(&self) -> impl Iterator<Item = (&ObjectId, &IndirectObjectBody)> {
        self.objects.iter()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenPageGeometry {
    page_index: u32,
    master_id: MasterId,
    width: PositiveLength,
    height: PositiveLength,
}
impl FrozenPageGeometry {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn width(&self) -> PositiveLength {
        self.width
    }
    pub const fn height(&self) -> PositiveLength {
        self.height
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PdfResourceBinding<Id> {
    logical_id: Id,
    name: PdfName,
    object_id: ObjectId,
}

/// Publication graph issued only by `PdfBackend::build`. The raw graph is
/// retained privately; the low-level untrusted builder has no conversion path
/// to this type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenPdfGraph {
    graph: ValidatedUntrustedPdfObjectGraph,
    selected_layout_fingerprint: LayoutStateFingerprint,
    pages: Vec<FrozenPageGeometry>,
    page_count: u32,
    object_count: u32,
    font_bindings: Vec<PdfResourceBinding<FontInstanceId>>,
    image_bindings: Vec<PdfResourceBinding<ImageResourceId>>,
    table_closures: Vec<TableDisplayClosureReceipt>,
    footnote_closure: Option<FootnoteDisplayClosureReceipt>,
}
impl FrozenPdfGraph {
    pub const fn selected_layout_fingerprint(&self) -> LayoutStateFingerprint {
        self.selected_layout_fingerprint
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub fn pages(&self) -> &[FrozenPageGeometry] {
        &self.pages
    }
    pub fn font_resource_names(&self) -> impl Iterator<Item = (FontInstanceId, &PdfName)> {
        self.font_bindings
            .iter()
            .map(|binding| (binding.logical_id, &binding.name))
    }
    pub fn image_resource_names(&self) -> impl Iterator<Item = (ImageResourceId, &PdfName)> {
        self.image_bindings
            .iter()
            .map(|binding| (binding.logical_id, &binding.name))
    }
    pub fn table_closures(&self) -> &[TableDisplayClosureReceipt] {
        &self.table_closures
    }
    pub const fn footnote_closure(&self) -> Option<&FootnoteDisplayClosureReceipt> {
        self.footnote_closure.as_ref()
    }
}

/// Bytes emitted by the crate-owned PDF serializer and bound to the exact
/// trusted graph facts consumed by manifest publication.
///
/// The receipt is deliberately non-`Clone`: one serializer emission must be
/// consumed by exactly one output/publication session.
///
/// ```compile_fail
/// use typaxis_pdf::VerifiedPdfBytesReceipt;
/// fn requires_clone<T: Clone>() {}
/// requires_clone::<VerifiedPdfBytesReceipt>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPdfBytesReceipt {
    bytes: Vec<u8>,
    sha256: [u8; 32],
    selected_layout_fingerprint: LayoutStateFingerprint,
    footnote_display_sha256: Option<[u8; 32]>,
    page_count: u32,
    object_count: u32,
    stream_compression: PdfStreamCompression,
    config_fingerprint: EffectiveConfigFingerprint,
}

/// Facts observed while replaying one complete serializer receipt to a byte
/// sink. The byte length and digest are aggregated from the successful writes;
/// the remaining facts stay bound to the graph that produced the receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdfStreamWriteFacts {
    byte_length: u64,
    sha256: [u8; 32],
    selected_layout_fingerprint: LayoutStateFingerprint,
    page_count: u32,
    object_count: u32,
    stream_compression: PdfStreamCompression,
    config_fingerprint: EffectiveConfigFingerprint,
}
impl PdfStreamWriteFacts {
    pub const fn byte_length(self) -> u64 {
        self.byte_length
    }
    pub const fn content_hash(self) -> [u8; 32] {
        self.sha256
    }
    pub const fn selected_layout_fingerprint(self) -> LayoutStateFingerprint {
        self.selected_layout_fingerprint
    }
    pub const fn page_count(self) -> u32 {
        self.page_count
    }
    pub const fn object_count(self) -> u32 {
        self.object_count
    }
    pub const fn stream_compression(self) -> PdfStreamCompression {
        self.stream_compression
    }
    pub const fn config_fingerprint(self) -> EffectiveConfigFingerprint {
        self.config_fingerprint
    }
}

impl VerifiedPdfBytesReceipt {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn selected_layout_fingerprint(&self) -> LayoutStateFingerprint {
        self.selected_layout_fingerprint
    }
    pub const fn footnote_display_sha256(&self) -> Option<[u8; 32]> {
        self.footnote_display_sha256
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub const fn stream_compression(&self) -> PdfStreamCompression {
        self.stream_compression
    }
    pub const fn config_fingerprint(&self) -> EffectiveConfigFingerprint {
        self.config_fingerprint
    }

    /// Replays the sealed PDF in bounded chunks and aggregates the exact bytes
    /// accepted by the sink. Short writes and interruptions are handled
    /// explicitly. A successful return proves that the streamed byte count and
    /// SHA-256 match this receipt; flushing and publication remain the output
    /// owner's responsibility.
    pub fn write_streaming<W: Write>(&self, sink: &mut W) -> io::Result<PdfStreamWriteFacts> {
        const WRITE_CHUNK_BYTES: usize = 64 * 1024;

        let mut byte_length = 0u64;
        let mut sha256 = PdfSha256::new();
        for chunk in self.bytes.chunks(WRITE_CHUNK_BYTES) {
            let mut remaining = chunk;
            while !remaining.is_empty() {
                match sink.write(remaining) {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::WriteZero,
                            "failed to stream the complete PDF receipt",
                        ))
                    }
                    Ok(written) if written <= remaining.len() => {
                        let accepted = &remaining[..written];
                        byte_length = byte_length
                            .checked_add(u64::try_from(written).map_err(|_| {
                                io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "streamed PDF byte count overflowed",
                                )
                            })?)
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "streamed PDF byte count overflowed",
                                )
                            })?;
                        sha256.update(accepted);
                        remaining = &remaining[written..];
                    }
                    Ok(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "PDF sink reported an impossible write length",
                        ))
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(error),
                }
            }
        }
        let digest = sha256.finish();
        if byte_length != self.byte_length() || digest != self.content_hash() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "streamed PDF facts do not match the serializer receipt",
            ));
        }
        Ok(PdfStreamWriteFacts {
            byte_length,
            sha256: digest,
            selected_layout_fingerprint: self.selected_layout_fingerprint,
            page_count: self.page_count,
            object_count: self.object_count,
            stream_compression: self.stream_compression,
            config_fingerprint: self.config_fingerprint,
        })
    }
}

/// Capability reserved for the in-crate serializer. External callers can pass
/// a receipt onward but cannot bless arbitrary byte slices.
#[derive(Debug)]
pub struct VerifiedPdfSerializerReceiptOwner {
    _private: (),
}
impl VerifiedPdfSerializerReceiptOwner {
    fn new() -> Self {
        Self { _private: () }
    }
    pub fn issue(
        &self,
        graph: &FrozenPdfGraph,
        bytes: Vec<u8>,
        stream_compression: PdfStreamCompression,
        config_fingerprint: EffectiveConfigFingerprint,
        limits: &ValidatedResourceLimits,
    ) -> Result<VerifiedPdfBytesReceipt, PdfError> {
        let digest = pdf_sha256(&bytes);
        self.issue_with_digest(
            graph,
            bytes,
            digest,
            stream_compression,
            config_fingerprint,
            limits,
        )
    }

    fn issue_serialized(
        &self,
        graph: &FrozenPdfGraph,
        serialized: SerializedPdfBytes,
        stream_compression: PdfStreamCompression,
        config_fingerprint: EffectiveConfigFingerprint,
        limits: &ValidatedResourceLimits,
    ) -> Result<VerifiedPdfBytesReceipt, PdfError> {
        self.issue_with_digest(
            graph,
            serialized.bytes,
            serialized.sha256,
            stream_compression,
            config_fingerprint,
            limits,
        )
    }

    fn issue_with_digest(
        &self,
        graph: &FrozenPdfGraph,
        bytes: Vec<u8>,
        digest: [u8; 32],
        stream_compression: PdfStreamCompression,
        config_fingerprint: EffectiveConfigFingerprint,
        limits: &ValidatedResourceLimits,
    ) -> Result<VerifiedPdfBytesReceipt, PdfError> {
        let byte_length = u64::try_from(bytes.len()).map_err(|_| PdfError::OutputTooLarge)?;
        if bytes.is_empty() || byte_length > limits.get().max_output_bytes {
            return Err(PdfError::OutputTooLarge);
        }
        Ok(VerifiedPdfBytesReceipt {
            bytes,
            sha256: digest,
            selected_layout_fingerprint: graph.selected_layout_fingerprint(),
            footnote_display_sha256: graph
                .footnote_closure()
                .map(FootnoteDisplayClosureReceipt::fingerprint),
            page_count: graph.page_count(),
            object_count: graph.object_count(),
            stream_compression,
            config_fingerprint,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FontObjectIds {
    type0: ObjectId,
    cid_font: ObjectId,
    descriptor: ObjectId,
    font_program: ObjectId,
    to_unicode: ObjectId,
    cid_to_gid: ObjectId,
}
impl FontObjectIds {
    fn allocate(
        plan: &FrozenPdfFontPlan,
        allocator: &mut DenseObjectAllocator,
    ) -> Result<Self, PdfError> {
        Self::allocate_blueprint(plan.indirect_object_blueprint(), allocator)
    }

    fn allocate_blueprint(
        blueprint: &[PdfFontIndirectObjectRole],
        allocator: &mut DenseObjectAllocator,
    ) -> Result<Self, PdfError> {
        let mut type0 = None;
        let mut cid_font = None;
        let mut descriptor = None;
        let mut font_program = None;
        let mut to_unicode = None;
        let mut cid_to_gid = None;
        for role in blueprint {
            let slot = match role {
                PdfFontIndirectObjectRole::Type0Font => &mut type0,
                PdfFontIndirectObjectRole::CidFont => &mut cid_font,
                PdfFontIndirectObjectRole::FontDescriptor => &mut descriptor,
                PdfFontIndirectObjectRole::EmbeddedFontProgram => &mut font_program,
                PdfFontIndirectObjectRole::ToUnicodeCMap => &mut to_unicode,
                PdfFontIndirectObjectRole::CidToGidMap => &mut cid_to_gid,
            };
            if slot.replace(allocator.allocate()?).is_some() {
                return Err(PdfError::ResourcePlanMismatch);
            }
        }
        Ok(Self {
            type0: type0.ok_or(PdfError::ResourcePlanMismatch)?,
            cid_font: cid_font.ok_or(PdfError::ResourcePlanMismatch)?,
            descriptor: descriptor.ok_or(PdfError::ResourcePlanMismatch)?,
            font_program: font_program.ok_or(PdfError::ResourcePlanMismatch)?,
            to_unicode: to_unicode.ok_or(PdfError::ResourcePlanMismatch)?,
            cid_to_gid: cid_to_gid.ok_or(PdfError::ResourcePlanMismatch)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PageObjectIds {
    page: ObjectId,
    content: ObjectId,
    annotations: Vec<ObjectId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ImageObjectIds {
    image: ObjectId,
    alpha_mask: Option<ObjectId>,
}
impl ImageObjectIds {
    fn allocate(
        plan: &FrozenPdfImagePlan,
        allocator: &mut DenseObjectAllocator,
    ) -> Result<Self, PdfError> {
        let image = allocator.allocate()?;
        let alpha_mask = plan
            .alpha_mask()
            .map(|_| allocator.allocate())
            .transpose()?;
        Ok(Self { image, alpha_mask })
    }
}

fn required_object_count(
    resource_plans: &FrozenPdfResourcePlans,
    pages: &[DisplayPage],
) -> Result<u32, PdfError> {
    let mut required = 2usize; // Catalog and Pages root.
    for font in resource_plans.fonts() {
        required = required
            .checked_add(
                usize::try_from(font.indirect_object_count())
                    .map_err(|_| PdfError::ObjectCountOverflow)?,
            )
            .ok_or(PdfError::ObjectCountOverflow)?;
    }
    for image in resource_plans.images() {
        required = required
            .checked_add(
                usize::try_from(image.indirect_object_count())
                    .map_err(|_| PdfError::ObjectCountOverflow)?,
            )
            .ok_or(PdfError::ObjectCountOverflow)?;
    }
    for page in pages {
        required = required
            .checked_add(2)
            .and_then(|count| count.checked_add(page.annotations.len()))
            .ok_or(PdfError::ObjectCountOverflow)?;
    }
    u32::try_from(required).map_err(|_| PdfError::ObjectCountOverflow)
}

fn validate_table_display_graph_closure(
    display: &ValidatedDisplayDocument,
    closures: &[TableDisplayClosureReceipt],
) -> Result<(), PdfError> {
    let layout_state = display
        .document()
        .source_layout()
        .state_fingerprint()
        .bytes();
    let mut previous_table = None;
    let mut claimed_commands = BTreeSet::new();
    for closure in closures {
        if closure.layout_state_sha256() != layout_state
            || closure.decoration_op_count() != 0
            || previous_table.is_some_and(|owner| owner >= closure.table_node_id())
        {
            return Err(PdfError::TableClosure);
        }
        previous_table = Some(closure.table_node_id());
        for observation in closure.commands() {
            let page = display
                .document()
                .pages
                .get(observation.page_index as usize)
                .filter(|page| page.page_index == observation.page_index)
                .ok_or(PdfError::TableClosure)?;
            let command = page
                .commands
                .get(observation.page_command_index as usize)
                .ok_or(PdfError::TableClosure)?;
            if command != &observation.command
                || !matches!(command, DisplayCommand::DrawGlyphRun { .. })
                || !claimed_commands
                    .insert((observation.page_index, observation.page_command_index))
            {
                return Err(PdfError::TableClosure);
            }
        }
    }
    let expected_commands: Vec<_> = closures
        .iter()
        .flat_map(|closure| closure.commands().iter().map(|command| &command.command))
        .collect();
    let records: Vec<_> = closures
        .iter()
        .flat_map(|closure| closure.records())
        .collect();
    for page in &display.document().pages {
        for (command_index, command) in page.commands.iter().enumerate() {
            let command_index = u32::try_from(command_index).map_err(|_| PdfError::TableClosure)?;
            if claimed_commands.contains(&(page.page_index, command_index)) {
                continue;
            }
            if is_unclaimed_table_command(command, page.page_index, &expected_commands, &records) {
                return Err(PdfError::TableClosure);
            }
        }
    }
    if display
        .document()
        .pages
        .iter()
        .flat_map(|page| &page.commands)
        .any(|command| {
            matches!(
                command,
                DisplayCommand::ClipPath { .. }
                    | DisplayCommand::FillPath { .. }
                    | DisplayCommand::StrokePath { .. }
            )
        })
    {
        return Err(PdfError::TableClosure);
    }
    Ok(())
}

fn validate_footnote_display_graph_closure(
    display: &ValidatedDisplayDocument,
    closure: &FootnoteDisplayClosureReceipt,
) -> Result<(), PdfError> {
    if display
        .document()
        .source_layout()
        .state_fingerprint()
        .bytes()
        != closure.body_layout_sha256()
        || display.document().pages.len() != closure.pages().len()
    {
        return Err(PdfError::FootnoteClosure);
    }
    let mut claimed = BTreeSet::new();
    for observation in closure.commands() {
        let page = display
            .document()
            .pages
            .get(observation.page_index() as usize)
            .filter(|page| page.page_index == observation.page_index())
            .ok_or(PdfError::FootnoteClosure)?;
        let command = page
            .commands
            .get(observation.page_command_index() as usize)
            .ok_or(PdfError::FootnoteClosure)?;
        if command != observation.command()
            || !claimed.insert((observation.page_index(), observation.page_command_index()))
        {
            return Err(PdfError::FootnoteClosure);
        }
    }
    for (page, facts) in display.document().pages.iter().zip(closure.pages()) {
        let body_command_count =
            usize::try_from(facts.body_command_count()).map_err(|_| PdfError::FootnoteClosure)?;
        if page.page_index != facts.page_index() || body_command_count > page.commands.len() {
            return Err(PdfError::FootnoteClosure);
        }
        let references: Vec<_> = closure
            .commands()
            .iter()
            .filter(|command| {
                command.page_index() == page.page_index
                    && command.kind() == FootnotePaintCommandKind::ReferenceMarker
            })
            .collect();
        if references.len() != facts.references().len()
            || references
                .iter()
                .any(|reference| reference.page_command_index() >= facts.body_command_count())
        {
            return Err(PdfError::FootnoteClosure);
        }
        let separator: Vec<_> = closure
            .commands()
            .iter()
            .filter(|command| {
                command.page_index() == page.page_index
                    && command.kind() == FootnotePaintCommandKind::Separator
            })
            .collect();
        if separator.len() != usize::from(facts.reservation().get() != Length::ZERO) {
            return Err(PdfError::FootnoteClosure);
        }
        if let Some(separator) = separator.first() {
            if separator.page_command_index() != facts.body_command_count() {
                return Err(PdfError::FootnoteClosure);
            }
            for index in separator.page_command_index() as usize + 1..page.commands.len() {
                let index = u32::try_from(index).map_err(|_| PdfError::FootnoteClosure)?;
                if !claimed.contains(&(page.page_index, index)) {
                    return Err(PdfError::FootnoteClosure);
                }
            }
        } else if body_command_count != page.commands.len() {
            return Err(PdfError::FootnoteClosure);
        }
        for (index, command) in page.commands.iter().enumerate() {
            if matches!(command, DisplayCommand::StrokePath { .. })
                && !claimed.contains(&(
                    page.page_index,
                    u32::try_from(index).map_err(|_| PdfError::FootnoteClosure)?,
                ))
            {
                return Err(PdfError::FootnoteClosure);
            }
        }
    }
    Ok(())
}

pub struct PdfBackend;
impl PdfBackend {
    pub fn build_footnote_profile(
        display: FootnoteProfileDisplay,
        resource_plans: FrozenPdfResourcePlans,
        limits: &ValidatedResourceLimits,
    ) -> Result<FrozenPdfGraph, PdfError> {
        let (display, closure) = display.into_parts();
        validate_footnote_display_graph_closure(&display, &closure)?;
        let mut graph = Self::build(display, resource_plans, limits)?;
        graph.footnote_closure = Some(closure);
        Ok(graph)
    }

    pub fn build_table_profile(
        display: TableProfileDisplay,
        resource_plans: FrozenPdfResourcePlans,
        limits: &ValidatedResourceLimits,
    ) -> Result<FrozenPdfGraph, PdfError> {
        let (display, table_closures) = display.into_parts();
        validate_table_display_graph_closure(&display, &table_closures)?;
        let mut graph = Self::build(display, resource_plans, limits)?;
        graph.table_closures = table_closures;
        Ok(graph)
    }

    pub fn build(
        display: ValidatedDisplayDocument,
        resource_plans: FrozenPdfResourcePlans,
        limits: &ValidatedResourceLimits,
    ) -> Result<FrozenPdfGraph, PdfError> {
        let selected_layout_fingerprint = display.document().source_layout().state_fingerprint();
        let selected_geometry = display.selected_page_geometry();
        if display.document().pages.len() != selected_geometry.len() || selected_geometry.is_empty()
        {
            return Err(PdfError::SelectedPageClosure);
        }

        // Every indirect-object role, including every annotation and every
        // member of a composite font plan, is counted before the allocator or
        // any object-body/resource-name collection is created.
        let required_objects = required_object_count(&resource_plans, &display.document().pages)?;
        if required_objects > limits.get().max_pdf_objects {
            return Err(PdfError::ObjectLimit);
        }
        if !resource_plans.matches_display(&display) {
            return Err(PdfError::ResourcePlanMismatch);
        }

        let mut page_geometry = Vec::new();
        page_geometry
            .try_reserve_exact(selected_geometry.len())
            .map_err(|_| PdfError::ObjectCountOverflow)?;
        for (display_page, geometry) in display.document().pages.iter().zip(selected_geometry) {
            if display_page.page_index != geometry.page_index()
                || display_page.width != geometry.width()
                || display_page.height != geometry.height()
            {
                return Err(PdfError::PageMasterMismatch);
            }
            page_geometry.push(FrozenPageGeometry {
                page_index: display_page.page_index,
                master_id: geometry.master_id().clone(),
                width: geometry.width(),
                height: geometry.height(),
            });
        }
        let page_count =
            u32::try_from(page_geometry.len()).map_err(|_| PdfError::ObjectCountOverflow)?;

        let mut allocator = DenseObjectAllocator::new(required_objects);
        let catalog_id = allocator.allocate()?;
        let pages_id = allocator.allocate()?;
        let font_object_ids: Vec<_> = resource_plans
            .fonts()
            .iter()
            .map(|plan| FontObjectIds::allocate(plan, &mut allocator))
            .collect::<Result<_, _>>()?;
        let image_object_ids: Vec<_> = resource_plans
            .images()
            .iter()
            .map(|plan| ImageObjectIds::allocate(plan, &mut allocator))
            .collect::<Result<_, _>>()?;
        let page_object_ids: Vec<_> = display
            .document()
            .pages
            .iter()
            .map(|page| {
                let page_id = allocator.allocate()?;
                let content = allocator.allocate()?;
                let annotations = page
                    .annotations
                    .iter()
                    .map(|_| allocator.allocate())
                    .collect::<Result<_, _>>()?;
                Ok(PageObjectIds {
                    page: page_id,
                    content,
                    annotations,
                })
            })
            .collect::<Result<_, PdfError>>()?;
        allocator.finish()?;

        let mut font_bindings = Vec::new();
        let mut image_bindings = Vec::new();
        let mut font_resources = PdfDictionary::new();
        let mut image_resources = PdfDictionary::new();
        for (index, (plan, object_ids)) in resource_plans
            .fonts()
            .iter()
            .zip(&font_object_ids)
            .enumerate()
        {
            let name = PdfName::from_bytes(format!("F{index}").into_bytes())?;
            font_resources.insert(name.clone(), PdfValue::Reference(object_ids.type0));
            font_bindings.push(PdfResourceBinding {
                logical_id: plan.font_instance_id(),
                name,
                object_id: object_ids.type0,
            });
        }
        for (index, (plan, object_id)) in resource_plans
            .images()
            .iter()
            .zip(&image_object_ids)
            .enumerate()
        {
            let name = PdfName::from_bytes(format!("Im{index}").into_bytes())?;
            image_resources.insert(name.clone(), PdfValue::Reference(object_id.image));
            image_bindings.push(PdfResourceBinding {
                logical_id: plan.image_id(),
                name,
                object_id: object_id.image,
            });
        }

        let mut resources = PdfDictionary::new();
        if !font_resources.is_empty() {
            resources.insert(pdf_name(b"Font")?, PdfValue::Dictionary(font_resources));
        }
        if !image_resources.is_empty() {
            resources.insert(pdf_name(b"XObject")?, PdfValue::Dictionary(image_resources));
        }
        let mut catalog = PdfDictionary::new();
        catalog.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Catalog")?));
        catalog.insert(pdf_name(b"Pages")?, PdfValue::Reference(pages_id));
        if !display.document().destinations.is_empty() {
            catalog.insert(
                pdf_name(b"Names")?,
                destination_name_tree(
                    &display.document().destinations,
                    &page_object_ids,
                    &page_geometry,
                )?,
            );
        }
        let mut pages = PdfDictionary::new();
        pages.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Pages")?));
        pages.insert(
            pdf_name(b"Kids")?,
            PdfValue::Array(
                page_object_ids
                    .iter()
                    .map(|ids| PdfValue::Reference(ids.page))
                    .collect(),
            ),
        );
        pages.insert(
            pdf_name(b"Count")?,
            PdfValue::Integer(i64::from(page_count)),
        );
        pages.insert(pdf_name(b"Resources")?, PdfValue::Dictionary(resources));

        let (font_plans, image_plans) = resource_plans.into_plans();
        let (display_document, selected_geometry_receipt) = display.into_parts();
        debug_assert_eq!(selected_geometry_receipt.len(), page_geometry.len());

        let mut builder = UntrustedPdfObjectGraphBuilder::new(limits);
        builder.insert(
            catalog_id,
            IndirectObjectBody::Value(PdfValue::Dictionary(catalog)),
        )?;
        builder.insert(
            pages_id,
            IndirectObjectBody::Value(PdfValue::Dictionary(pages)),
        )?;
        for ((plan, object_ids), binding) in font_plans
            .into_iter()
            .zip(font_object_ids)
            .zip(&font_bindings)
        {
            debug_assert_eq!(object_ids.type0, binding.object_id);
            insert_font_objects(&mut builder, plan, object_ids)?;
        }
        for (plan, object_ids) in image_plans.into_iter().zip(image_object_ids) {
            let alpha_mask = plan.alpha_mask().cloned();
            if alpha_mask.is_some() != object_ids.alpha_mask.is_some() {
                return Err(PdfError::ResourcePlanMismatch);
            }
            builder.insert(
                object_ids.image,
                IndirectObjectBody::FrozenImageResource {
                    plan,
                    alpha_mask_object: object_ids.alpha_mask,
                },
            )?;
            if let (Some(mask), Some(mask_object)) = (alpha_mask, object_ids.alpha_mask) {
                builder.insert(mask_object, IndirectObjectBody::FrozenImageAlphaMask(mask))?;
            }
        }
        for ((geometry, object_ids), display_page) in page_geometry
            .iter()
            .zip(page_object_ids)
            .zip(display_document.pages)
        {
            if display_page.annotations.len() != object_ids.annotations.len() {
                return Err(PdfError::InvalidAnnotationClosure);
            }
            let mut page = PdfDictionary::new();
            page.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Page")?));
            page.insert(pdf_name(b"Parent")?, PdfValue::Reference(pages_id));
            page.insert(
                pdf_name(b"MediaBox")?,
                media_box(geometry.width, geometry.height)?,
            );
            page.insert(
                pdf_name(b"Contents")?,
                PdfValue::Reference(object_ids.content),
            );
            if !object_ids.annotations.is_empty() {
                page.insert(
                    pdf_name(b"Annots")?,
                    PdfValue::Array(
                        object_ids
                            .annotations
                            .iter()
                            .map(|id| PdfValue::Reference(*id))
                            .collect(),
                    ),
                );
            }
            builder.insert(
                object_ids.page,
                IndirectObjectBody::Value(PdfValue::Dictionary(page)),
            )?;
            for (annotation, annotation_id) in
                display_page.annotations.iter().zip(&object_ids.annotations)
            {
                builder.insert(
                    *annotation_id,
                    IndirectObjectBody::Value(PdfValue::Dictionary(annotation_dictionary(
                        annotation,
                        geometry.height,
                    )?)),
                )?;
            }
            builder.insert(
                object_ids.content,
                IndirectObjectBody::DisplayPageContent(display_page),
            )?;
        }
        let graph = builder.validate_untrusted(catalog_id)?;
        let object_count =
            u32::try_from(graph.objects.len()).map_err(|_| PdfError::ObjectCountOverflow)?;
        if object_count != required_objects {
            return Err(PdfError::ObjectCountOverflow);
        }
        Ok(FrozenPdfGraph {
            graph,
            selected_layout_fingerprint,
            pages: page_geometry,
            page_count,
            object_count,
            font_bindings,
            image_bindings,
            table_closures: Vec::new(),
            footnote_closure: None,
        })
    }

    /// Serializes a publication-trusted graph as a deterministic PDF 1.7
    /// file with a classic cross-reference table.
    ///
    /// The output budget is enforced before output-buffer growth and the
    /// principal variable-size encoded-payload allocations. Stream lengths
    /// are derived from the bytes after the selected filter has been applied,
    /// and the returned receipt is bound to this exact graph and effective
    /// configuration.
    pub fn serialize(
        graph: FrozenPdfGraph,
        config: &EffectiveConfig,
    ) -> Result<VerifiedPdfBytesReceipt, PdfError> {
        let serialized = serialize_classic_xref(&graph, config)?;
        VerifiedPdfSerializerReceiptOwner::new().issue_serialized(
            &graph,
            serialized,
            config.stream_compression(),
            config.fingerprint(),
            config.limits(),
        )
    }
}

fn insert_font_objects(
    builder: &mut UntrustedPdfObjectGraphBuilder,
    plan: FrozenPdfFontPlan,
    ids: FontObjectIds,
) -> Result<(), PdfError> {
    let base_font = subset_base_font_name(plan.embedded_postscript_name())?;
    let mut type0 = PdfDictionary::new();
    type0.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Font")?));
    type0.insert(pdf_name(b"Subtype")?, PdfValue::Name(pdf_name(b"Type0")?));
    type0.insert(pdf_name(b"BaseFont")?, PdfValue::Name(base_font.clone()));
    type0.insert(
        pdf_name(b"Encoding")?,
        PdfValue::Name(pdf_name(b"Identity-H")?),
    );
    type0.insert(
        pdf_name(b"DescendantFonts")?,
        PdfValue::Array(vec![PdfValue::Reference(ids.cid_font)]),
    );
    type0.insert(pdf_name(b"ToUnicode")?, PdfValue::Reference(ids.to_unicode));

    let mut cid_system_info = PdfDictionary::new();
    cid_system_info.insert(
        pdf_name(b"Registry")?,
        PdfValue::ByteString(b"Adobe".to_vec()),
    );
    cid_system_info.insert(
        pdf_name(b"Ordering")?,
        PdfValue::ByteString(b"Identity".to_vec()),
    );
    cid_system_info.insert(pdf_name(b"Supplement")?, PdfValue::Integer(0));
    let mut widths = Vec::new();
    for binding in &plan.subset_plan().cids {
        widths.push(PdfValue::Integer(i64::from(binding.cid.get())));
        widths.push(PdfValue::Array(vec![PdfValue::Integer(i64::from(
            binding.width_1000,
        ))]));
    }
    let mut cid_font = PdfDictionary::new();
    cid_font.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Font")?));
    cid_font.insert(
        pdf_name(b"Subtype")?,
        PdfValue::Name(pdf_name(b"CIDFontType2")?),
    );
    cid_font.insert(pdf_name(b"BaseFont")?, PdfValue::Name(base_font.clone()));
    cid_font.insert(
        pdf_name(b"CIDSystemInfo")?,
        PdfValue::Dictionary(cid_system_info),
    );
    cid_font.insert(
        pdf_name(b"FontDescriptor")?,
        PdfValue::Reference(ids.descriptor),
    );
    cid_font.insert(pdf_name(b"DW")?, PdfValue::Integer(1_000));
    if !widths.is_empty() {
        cid_font.insert(pdf_name(b"W")?, PdfValue::Array(widths));
    }
    cid_font.insert(
        pdf_name(b"CIDToGIDMap")?,
        PdfValue::Reference(ids.cid_to_gid),
    );

    let metrics = plan.metrics();
    let mut descriptor = PdfDictionary::new();
    descriptor.insert(
        pdf_name(b"Type")?,
        PdfValue::Name(pdf_name(b"FontDescriptor")?),
    );
    descriptor.insert(pdf_name(b"FontName")?, PdfValue::Name(base_font));
    descriptor.insert(
        pdf_name(b"Flags")?,
        PdfValue::Integer(i64::from(metrics.flags)),
    );
    descriptor.insert(
        pdf_name(b"FontBBox")?,
        PdfValue::Array(
            metrics
                .bbox_1000
                .iter()
                .map(|value| PdfValue::Integer(i64::from(*value)))
                .collect(),
        ),
    );
    descriptor.insert(
        pdf_name(b"ItalicAngle")?,
        PdfValue::Decimal(PdfDecimal::new(
            i64::from(metrics.italic_angle_milli_degrees),
            3,
        )?),
    );
    descriptor.insert(
        pdf_name(b"Ascent")?,
        PdfValue::Integer(i64::from(metrics.ascent_1000)),
    );
    descriptor.insert(
        pdf_name(b"Descent")?,
        PdfValue::Integer(i64::from(metrics.descent_1000)),
    );
    descriptor.insert(
        pdf_name(b"CapHeight")?,
        PdfValue::Integer(i64::from(metrics.cap_height_1000)),
    );
    descriptor.insert(
        pdf_name(b"StemV")?,
        PdfValue::Integer(i64::from(metrics.stem_v_1000)),
    );
    descriptor.insert(
        pdf_name(b"FontFile2")?,
        PdfValue::Reference(ids.font_program),
    );

    builder.insert(
        ids.type0,
        IndirectObjectBody::Value(PdfValue::Dictionary(type0)),
    )?;
    builder.insert(
        ids.cid_font,
        IndirectObjectBody::Value(PdfValue::Dictionary(cid_font)),
    )?;
    builder.insert(
        ids.descriptor,
        IndirectObjectBody::Value(PdfValue::Dictionary(descriptor)),
    )?;
    builder.insert(
        ids.font_program,
        IndirectObjectBody::FrozenFontProgram(plan),
    )?;
    builder.insert(
        ids.to_unicode,
        IndirectObjectBody::FrozenToUnicodeCMap {
            font_program_object: ids.font_program,
        },
    )?;
    builder.insert(
        ids.cid_to_gid,
        IndirectObjectBody::FrozenCidToGidMap {
            font_program_object: ids.font_program,
        },
    )?;
    Ok(())
}

fn subset_base_font_name(embedded_postscript_name: &str) -> Result<PdfName, PdfError> {
    PdfName::from_bytes(embedded_postscript_name.as_bytes().to_vec())
}

fn destination_name_tree(
    destinations: &[NamedDestination],
    page_ids: &[PageObjectIds],
    geometry: &[FrozenPageGeometry],
) -> Result<PdfValue, PdfError> {
    let mut names = Vec::new();
    let name_value_count = destination_name_value_count(destinations.len())?;
    names
        .try_reserve_exact(name_value_count)
        .map_err(|_| PdfError::ObjectCountOverflow)?;
    for destination in destinations {
        let page_index = usize::try_from(destination.page_index)
            .map_err(|_| PdfError::InvalidDestinationClosure)?;
        let ids = page_ids
            .get(page_index)
            .ok_or(PdfError::InvalidDestinationClosure)?;
        let page = geometry
            .get(page_index)
            .ok_or(PdfError::InvalidDestinationClosure)?;
        if page.page_index != destination.page_index {
            return Err(PdfError::InvalidDestinationClosure);
        }
        names.push(PdfValue::ByteString(
            destination.anchor_id.as_str().as_bytes().to_vec(),
        ));
        names.push(destination_array(destination, ids.page, page.height)?);
    }
    let mut destination_tree = PdfDictionary::new();
    destination_tree.insert(pdf_name(b"Names")?, PdfValue::Array(names));
    let mut names_dictionary = PdfDictionary::new();
    names_dictionary.insert(pdf_name(b"Dests")?, PdfValue::Dictionary(destination_tree));
    Ok(PdfValue::Dictionary(names_dictionary))
}

fn destination_name_value_count(destination_count: usize) -> Result<usize, PdfError> {
    destination_count
        .checked_mul(2)
        .ok_or(PdfError::ObjectCountOverflow)
}

fn destination_array(
    destination: &NamedDestination,
    page_id: ObjectId,
    page_height: PositiveLength,
) -> Result<PdfValue, PdfError> {
    let mut values = vec![PdfValue::Reference(page_id)];
    match destination.view {
        DestinationView::Xyz { point } => {
            values.push(PdfValue::Name(pdf_name(b"XYZ")?));
            values.extend(pdf_point(point, page_height)?);
            values.push(PdfValue::Null);
        }
        DestinationView::FitPage => values.push(PdfValue::Name(pdf_name(b"Fit")?)),
        DestinationView::FitWidth { top } => {
            values.push(PdfValue::Name(pdf_name(b"FitH")?));
            values.push(match top {
                Some(top) => pdf_length(pdf_y(page_height, top)?)?,
                None => PdfValue::Null,
            });
        }
    }
    Ok(PdfValue::Array(values))
}

fn annotation_dictionary(
    annotation: &LinkAnnotation,
    page_height: PositiveLength,
) -> Result<PdfDictionary, PdfError> {
    let mut dictionary = PdfDictionary::new();
    dictionary.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"Annot")?));
    dictionary.insert(pdf_name(b"Subtype")?, PdfValue::Name(pdf_name(b"Link")?));
    dictionary.insert(
        pdf_name(b"Rect")?,
        annotation_rectangle(annotation.rect, page_height)?,
    );
    dictionary.insert(
        pdf_name(b"Border")?,
        PdfValue::Array(vec![
            PdfValue::Integer(0),
            PdfValue::Integer(0),
            PdfValue::Integer(0),
        ]),
    );
    match &annotation.target {
        LinkTarget::Internal(anchor) => {
            dictionary.insert(
                pdf_name(b"Dest")?,
                PdfValue::ByteString(anchor.as_str().as_bytes().to_vec()),
            );
        }
        LinkTarget::Uri(uri) => {
            let mut action = PdfDictionary::new();
            action.insert(pdf_name(b"S")?, PdfValue::Name(pdf_name(b"URI")?));
            action.insert(
                pdf_name(b"URI")?,
                PdfValue::ByteString(uri.as_str().as_bytes().to_vec()),
            );
            dictionary.insert(pdf_name(b"A")?, PdfValue::Dictionary(action));
        }
    }
    Ok(dictionary)
}

fn pdf_y(page_height: PositiveLength, y: Length) -> Result<Length, PdfError> {
    page_height
        .get()
        .checked_sub(y)
        .ok_or(PdfError::PageMasterMismatch)
}

fn pdf_point(point: Point, page_height: PositiveLength) -> Result<[PdfValue; 2], PdfError> {
    Ok([
        pdf_length(point.x)?,
        pdf_length(pdf_y(page_height, point.y)?)?,
    ])
}

fn annotation_rectangle(rect: Rect, page_height: PositiveLength) -> Result<PdfValue, PdfError> {
    let right = rect
        .x()
        .checked_add(rect.width().get())
        .ok_or(PdfError::PageMasterMismatch)?;
    let bottom = rect
        .y()
        .checked_add(rect.height().get())
        .ok_or(PdfError::PageMasterMismatch)?;
    Ok(PdfValue::Array(vec![
        pdf_length(rect.x())?,
        pdf_length(pdf_y(page_height, bottom)?)?,
        pdf_length(right)?,
        pdf_length(pdf_y(page_height, rect.y())?)?,
    ]))
}

struct DenseObjectAllocator {
    // One wider than ObjectId so the state after issuing u32::MAX is
    // representable and the inclusive configured maximum can succeed.
    next: u64,
    required: u32,
}
impl DenseObjectAllocator {
    const fn new(required: u32) -> Self {
        Self { next: 1, required }
    }
    fn allocate(&mut self) -> Result<ObjectId, PdfError> {
        if self.next > u64::from(self.required) {
            return Err(PdfError::ObjectCountOverflow);
        }
        let id = u32::try_from(self.next)
            .ok()
            .and_then(ObjectId::new)
            .ok_or(PdfError::ObjectCountOverflow)?;
        self.next = self
            .next
            .checked_add(1)
            .ok_or(PdfError::ObjectCountOverflow)?;
        Ok(id)
    }
    fn finish(self) -> Result<(), PdfError> {
        if self.next == u64::from(self.required) + 1 {
            Ok(())
        } else {
            Err(PdfError::ObjectCountOverflow)
        }
    }
}

fn pdf_name(bytes: &[u8]) -> Result<PdfName, PdfError> {
    PdfName::from_bytes(bytes.to_vec())
}

fn media_box(width: PositiveLength, height: PositiveLength) -> Result<PdfValue, PdfError> {
    Ok(PdfValue::Array(vec![
        PdfValue::Integer(0),
        PdfValue::Integer(0),
        pdf_length(width.get())?,
        pdf_length(height.get())?,
    ]))
}

fn pdf_length(length: Length) -> Result<PdfValue, PdfError> {
    if length == Length::ZERO {
        return Ok(PdfValue::Integer(0));
    }
    for scale in (0..=6u8).rev() {
        let factor = 10i128.pow(u32::from(scale));
        let numerator = i128::from(length.raw())
            .checked_mul(factor)
            .ok_or(PdfError::PageMasterMismatch)?;
        let coefficient =
            round_ratio_ties_even(numerator, 65_536).ok_or(PdfError::PageMasterMismatch)?;
        if let Ok(coefficient) = i64::try_from(coefficient) {
            if coefficient != 0 {
                return Ok(PdfValue::Decimal(PdfDecimal::new(coefficient, scale)?));
            }
        }
    }
    Err(PdfError::PageMasterMismatch)
}

const CLASSIC_XREF_MAX_OFFSET: u64 = 9_999_999_999;

struct LimitedPdfBuffer {
    bytes: Vec<u8>,
    max_len: u64,
    sha256: PdfSha256,
}

impl LimitedPdfBuffer {
    fn new(max_len: u64) -> Self {
        Self {
            bytes: Vec::new(),
            max_len,
            sha256: PdfSha256::new(),
        }
    }

    fn len_u64(&self) -> Result<u64, PdfError> {
        u64::try_from(self.bytes.len()).map_err(|_| PdfError::OutputTooLarge)
    }

    fn remaining(&self) -> Result<u64, PdfError> {
        self.max_len
            .checked_sub(self.len_u64()?)
            .ok_or(PdfError::OutputTooLarge)
    }

    fn extend(&mut self, bytes: &[u8]) -> Result<(), PdfError> {
        let additional = u64::try_from(bytes.len()).map_err(|_| PdfError::OutputTooLarge)?;
        if additional > self.remaining()? {
            return Err(PdfError::OutputTooLarge);
        }
        self.bytes
            .try_reserve_exact(bytes.len())
            .map_err(|_| PdfError::OutputTooLarge)?;
        self.bytes.extend_from_slice(bytes);
        self.sha256.update(bytes);
        Ok(())
    }

    fn push(&mut self, byte: u8) -> Result<(), PdfError> {
        self.extend(&[byte])
    }

    fn integer(&mut self, value: i64) -> Result<(), PdfError> {
        let mut digits = [0u8; 20];
        let mut start = digits.len();
        let mut magnitude = value.unsigned_abs();
        loop {
            start -= 1;
            digits[start] = b'0' + u8::try_from(magnitude % 10).unwrap_or(0);
            magnitude /= 10;
            if magnitude == 0 {
                break;
            }
        }
        if value.is_negative() {
            start -= 1;
            digits[start] = b'-';
        }
        self.extend(&digits[start..])
    }

    fn unsigned(&mut self, value: u64) -> Result<(), PdfError> {
        let (digits, start) = decimal_digits(value);
        self.extend(&digits[start..])
    }

    fn zero_padded_unsigned(&mut self, value: u64, width: usize) -> Result<(), PdfError> {
        const ZEROES: &[u8; 20] = b"00000000000000000000";
        let (digits, start) = decimal_digits(value);
        let digit_count = digits.len() - start;
        let padding = width
            .checked_sub(digit_count)
            .ok_or(PdfError::OutputTooLarge)?;
        if padding > ZEROES.len() {
            return Err(PdfError::OutputTooLarge);
        }
        let required = u64::try_from(width).map_err(|_| PdfError::OutputTooLarge)?;
        if required > self.remaining()? {
            return Err(PdfError::OutputTooLarge);
        }
        self.extend(&ZEROES[..padding])?;
        self.extend(&digits[start..])
    }

    fn into_serialized(self) -> SerializedPdfBytes {
        SerializedPdfBytes {
            bytes: self.bytes,
            sha256: self.sha256.finish(),
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

struct SerializedPdfBytes {
    bytes: Vec<u8>,
    sha256: [u8; 32],
}

fn decimal_digits(mut value: u64) -> ([u8; 20], usize) {
    let mut digits = [0u8; 20];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + u8::try_from(value % 10).unwrap_or(0);
        value /= 10;
        if value == 0 {
            break;
        }
    }
    (digits, start)
}

struct PdfSerializationContext<'a> {
    graph: &'a FrozenPdfGraph,
    font_names: Vec<(FontInstanceId, &'a PdfName)>,
    image_names: Vec<(ImageResourceId, &'a PdfName)>,
    font_plans: Vec<(FontInstanceId, &'a FrozenPdfFontPlan)>,
}

impl<'a> PdfSerializationContext<'a> {
    fn new(graph: &'a FrozenPdfGraph) -> Result<Self, PdfError> {
        let mut font_names = Vec::new();
        font_names
            .try_reserve_exact(graph.font_bindings.len())
            .map_err(|_| PdfError::OutputTooLarge)?;
        font_names.extend(
            graph
                .font_bindings
                .iter()
                .map(|binding| (binding.logical_id, &binding.name)),
        );
        font_names.sort_unstable_by_key(|(id, _)| *id);
        if font_names.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(PdfError::ResourcePlanMismatch);
        }

        let mut image_names = Vec::new();
        image_names
            .try_reserve_exact(graph.image_bindings.len())
            .map_err(|_| PdfError::OutputTooLarge)?;
        image_names.extend(
            graph
                .image_bindings
                .iter()
                .map(|binding| (binding.logical_id, &binding.name)),
        );
        image_names.sort_unstable_by_key(|(id, _)| *id);
        if image_names.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(PdfError::ResourcePlanMismatch);
        }

        let mut font_plans = Vec::new();
        font_plans
            .try_reserve_exact(graph.font_bindings.len())
            .map_err(|_| PdfError::OutputTooLarge)?;
        let mut image_plans = Vec::new();
        image_plans
            .try_reserve_exact(graph.image_bindings.len())
            .map_err(|_| PdfError::OutputTooLarge)?;
        for body in graph.graph.objects.values() {
            match body {
                IndirectObjectBody::FrozenFontProgram(plan) => {
                    if font_plans.len() == graph.font_bindings.len() {
                        return Err(PdfError::ResourcePlanMismatch);
                    }
                    font_plans.push((plan.font_instance_id(), plan));
                }
                IndirectObjectBody::FrozenImageResource { plan, .. } => {
                    if image_plans.len() == graph.image_bindings.len() {
                        return Err(PdfError::ResourcePlanMismatch);
                    }
                    image_plans.push(plan.image_id());
                }
                _ => {}
            }
        }
        font_plans.sort_unstable_by_key(|(id, _)| *id);
        image_plans.sort_unstable();
        if font_names
            .iter()
            .map(|(id, _)| *id)
            .ne(font_plans.iter().map(|(id, _)| *id))
            || image_names
                .iter()
                .map(|(id, _)| *id)
                .ne(image_plans.iter().copied())
        {
            return Err(PdfError::ResourcePlanMismatch);
        }
        Ok(Self {
            graph,
            font_names,
            image_names,
            font_plans,
        })
    }

    fn font_name(&self, id: FontInstanceId) -> Option<&'a PdfName> {
        self.font_names
            .binary_search_by_key(&id, |(found, _)| *found)
            .ok()
            .map(|index| self.font_names[index].1)
    }

    fn image_name(&self, id: ImageResourceId) -> Option<&'a PdfName> {
        self.image_names
            .binary_search_by_key(&id, |(found, _)| *found)
            .ok()
            .map(|index| self.image_names[index].1)
    }

    fn font_plan(&self, id: FontInstanceId) -> Option<&'a FrozenPdfFontPlan> {
        self.font_plans
            .binary_search_by_key(&id, |(found, _)| *found)
            .ok()
            .map(|index| self.font_plans[index].1)
    }

    fn font_program(&self, id: ObjectId) -> Result<&'a FrozenPdfFontPlan, PdfError> {
        match self.graph.graph.objects.get(&id) {
            Some(IndirectObjectBody::FrozenFontProgram(plan)) => Ok(plan),
            _ => Err(PdfError::ResourcePlanMismatch),
        }
    }
}

fn serialize_classic_xref(
    graph: &FrozenPdfGraph,
    config: &EffectiveConfig,
) -> Result<SerializedPdfBytes, PdfError> {
    if graph.object_count > config.limits().get().max_pdf_objects {
        return Err(PdfError::ObjectLimit);
    }
    if graph.page_count == 0
        || graph.page_count > config.limits().get().max_pages
        || usize::try_from(graph.page_count).ok() != Some(graph.pages.len())
    {
        return Err(PdfError::SelectedPageClosure);
    }
    let object_count =
        usize::try_from(graph.object_count).map_err(|_| PdfError::ObjectCountOverflow)?;
    if object_count != graph.graph.objects.len()
        || !graph.graph.objects.contains_key(&graph.graph.root)
    {
        return Err(PdfError::ObjectCountOverflow);
    }
    let offset_count = object_count
        .checked_add(1)
        .ok_or(PdfError::ObjectCountOverflow)?;
    let max_len = config
        .limits()
        .get()
        .max_output_bytes
        .min(CLASSIC_XREF_MAX_OFFSET);
    // Every object needs one fixed-width xref record, as does free object
    // zero. Reject an impossible output before allocating the offset table or
    // the resource lookup context.
    let minimum_structural_bytes = u64::try_from(offset_count)
        .ok()
        .and_then(|count| count.checked_mul(20))
        .and_then(|bytes| {
            u64::try_from(object_count)
                .ok()
                .and_then(|count| count.checked_mul(17))
                .and_then(|object_bytes| bytes.checked_add(object_bytes))
        })
        .and_then(|bytes| bytes.checked_add(15)) // PDF header and binary marker
        .ok_or(PdfError::OutputTooLarge)?;
    let bookkeeping_bytes = u64::try_from(offset_count)
        .ok()
        .and_then(|count| count.checked_mul(std::mem::size_of::<u64>() as u64))
        .and_then(|bytes| {
            u64::try_from(graph.font_bindings.len())
                .ok()
                .and_then(|count| {
                    count.checked_mul(
                        (std::mem::size_of::<(FontInstanceId, &PdfName)>()
                            + std::mem::size_of::<(FontInstanceId, &FrozenPdfFontPlan)>())
                            as u64,
                    )
                })
                .and_then(|font_bytes| bytes.checked_add(font_bytes))
        })
        .and_then(|bytes| {
            u64::try_from(graph.image_bindings.len())
                .ok()
                .and_then(|count| {
                    count.checked_mul(
                        (std::mem::size_of::<(ImageResourceId, &PdfName)>()
                            + std::mem::size_of::<ImageResourceId>())
                            as u64,
                    )
                })
                .and_then(|image_bytes| bytes.checked_add(image_bytes))
        })
        .ok_or(PdfError::OutputTooLarge)?;
    if minimum_structural_bytes > max_len || bookkeeping_bytes > max_len {
        return Err(PdfError::OutputTooLarge);
    }
    let context = PdfSerializationContext::new(graph)?;
    let mut output = LimitedPdfBuffer::new(max_len);
    output.extend(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n")?;

    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(offset_count)
        .map_err(|_| PdfError::ObjectCountOverflow)?;
    offsets.push(0u64); // object zero is the head of the free list

    for (index, (id, body)) in graph.graph.objects.iter().enumerate() {
        let expected = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(PdfError::ObjectCountOverflow)?;
        if u64::from(id.get()) != expected {
            return Err(PdfError::SparseObjectId);
        }
        let offset = output.len_u64()?;
        if offset > CLASSIC_XREF_MAX_OFFSET {
            return Err(PdfError::OutputTooLarge);
        }
        offsets.push(offset);
        output.unsigned(u64::from(id.get()))?;
        output.extend(b" 0 obj\n")?;
        write_indirect_body(&mut output, body, &context, config)?;
        output.extend(b"\nendobj\n")?;
    }

    let xref_offset = output.len_u64()?;
    if xref_offset > CLASSIC_XREF_MAX_OFFSET {
        return Err(PdfError::OutputTooLarge);
    }
    let xref_size = u64::try_from(object_count)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or(PdfError::ObjectCountOverflow)?;
    output.extend(b"xref\n0 ")?;
    output.unsigned(xref_size)?;
    output.push(b'\n')?;
    output.extend(b"0000000000 65535 f \n")?;
    for offset in offsets.into_iter().skip(1) {
        write_xref_entry(&mut output, offset)?;
    }
    output.extend(b"trailer\n<< /Size ")?;
    output.unsigned(xref_size)?;
    output.extend(b" /Root ")?;
    output.unsigned(u64::from(graph.graph.root.get()))?;
    output.extend(b" 0 R >>\nstartxref\n")?;
    output.unsigned(xref_offset)?;
    output.extend(b"\n%%EOF\n")?;
    Ok(output.into_serialized())
}

fn write_xref_entry(output: &mut LimitedPdfBuffer, offset: u64) -> Result<(), PdfError> {
    if offset > CLASSIC_XREF_MAX_OFFSET {
        return Err(PdfError::OutputTooLarge);
    }
    output.zero_padded_unsigned(offset, 10)?;
    output.extend(b" 00000 n \n")
}

fn write_indirect_body(
    output: &mut LimitedPdfBuffer,
    body: &IndirectObjectBody,
    context: &PdfSerializationContext<'_>,
    config: &EffectiveConfig,
) -> Result<(), PdfError> {
    match body {
        IndirectObjectBody::Value(value) => write_pdf_value(output, value),
        IndirectObjectBody::Stream(stream) => match stream.encoding {
            StreamEncoding::None => {
                write_stream(output, &stream.dictionary, stream.raw_data.as_slice(), None)
            }
            StreamEncoding::Flate => {
                let data = zlib_stored(stream.raw_data.as_slice(), output.remaining()?)?;
                write_stream(output, &stream.dictionary, &data, Some(b"FlateDecode"))
            }
            StreamEncoding::EncodedFlate => write_stream(
                output,
                &stream.dictionary,
                stream.raw_data.as_slice(),
                Some(b"FlateDecode"),
            ),
            StreamEncoding::Dct => write_stream(
                output,
                &stream.dictionary,
                stream.raw_data.as_slice(),
                Some(b"DCTDecode"),
            ),
        },
        IndirectObjectBody::FrozenFontProgram(plan) => {
            let mut dictionary = PdfDictionary::new();
            dictionary.insert(
                pdf_name(b"Length1")?,
                PdfValue::Integer(
                    i64::try_from(plan.subset_bytes().len())
                        .map_err(|_| PdfError::OutputTooLarge)?,
                ),
            );
            write_generated_stream(output, &dictionary, plan.subset_bytes(), config)
        }
        IndirectObjectBody::FrozenToUnicodeCMap {
            font_program_object,
        } => {
            let plan = context.font_program(*font_program_object)?;
            let data = to_unicode_cmap(plan, output.remaining()?)?;
            write_generated_stream(output, &PdfDictionary::new(), &data, config)
        }
        IndirectObjectBody::FrozenCidToGidMap {
            font_program_object,
        } => {
            let plan = context.font_program(*font_program_object)?;
            let data = cid_to_gid_map(plan, output.remaining()?)?;
            write_generated_stream(output, &PdfDictionary::new(), &data, config)
        }
        IndirectObjectBody::FrozenImageResource {
            plan,
            alpha_mask_object,
        } => write_image_stream(output, plan, *alpha_mask_object, config),
        IndirectObjectBody::FrozenImageAlphaMask(mask) => {
            write_alpha_mask_stream(output, mask, config)
        }
        IndirectObjectBody::DisplayPageContent(page) => {
            let data = page_content_stream(page, context, output.remaining()?)?;
            write_generated_stream(output, &PdfDictionary::new(), &data, config)
        }
    }
}

fn write_pdf_value(output: &mut LimitedPdfBuffer, value: &PdfValue) -> Result<(), PdfError> {
    match value {
        PdfValue::Null => output.extend(b"null"),
        PdfValue::Bool(true) => output.extend(b"true"),
        PdfValue::Bool(false) => output.extend(b"false"),
        PdfValue::Integer(value) => output.integer(*value),
        PdfValue::Decimal(value) => write_pdf_decimal(output, *value),
        PdfValue::Name(name) => write_pdf_name(output, name),
        PdfValue::ByteString(bytes) => write_hex_string(output, bytes),
        PdfValue::Array(values) => {
            output.push(b'[')?;
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(b' ')?;
                }
                write_pdf_value(output, value)?;
            }
            output.push(b']')
        }
        PdfValue::Dictionary(dictionary) => write_dictionary(output, dictionary),
        PdfValue::Reference(id) => {
            output.unsigned(u64::from(id.get()))?;
            output.extend(b" 0 R")
        }
    }
}

fn write_pdf_name(output: &mut LimitedPdfBuffer, name: &PdfName) -> Result<(), PdfError> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let encoded_len = name.0.iter().try_fold(1u64, |length, byte| {
        let regular = (33..=126).contains(byte) && !b"()<>[]{}/%#".contains(byte);
        length.checked_add(if regular { 1 } else { 3 })
    });
    if encoded_len.ok_or(PdfError::OutputTooLarge)? > output.remaining()? {
        return Err(PdfError::OutputTooLarge);
    }
    output.push(b'/')?;
    for byte in &name.0 {
        let regular = (33..=126).contains(byte) && !b"()<>[]{}/%#".contains(byte);
        if regular {
            output.push(*byte)?;
        } else {
            output.push(b'#')?;
            output.push(HEX[usize::from(byte >> 4)])?;
            output.push(HEX[usize::from(byte & 0x0f)])?;
        }
    }
    Ok(())
}

fn write_pdf_decimal(output: &mut LimitedPdfBuffer, decimal: PdfDecimal) -> Result<(), PdfError> {
    if decimal.coefficient == 0 {
        return output.push(b'0');
    }
    let (digits, start) = decimal_digits(decimal.coefficient.unsigned_abs());
    let digits = &digits[start..];
    let scale = usize::from(decimal.scale);
    let mut token = [0u8; 24];
    let mut length = 0usize;
    if decimal.coefficient.is_negative() {
        token[length] = b'-';
        length += 1;
    }
    if scale == 0 {
        token[length..length + digits.len()].copy_from_slice(digits);
        length += digits.len();
    } else if digits.len() <= scale {
        token[length] = b'0';
        token[length + 1] = b'.';
        length += 2;
        let zeroes = scale - digits.len();
        token[length..length + zeroes].fill(b'0');
        length += zeroes;
        token[length..length + digits.len()].copy_from_slice(digits);
        length += digits.len();
    } else {
        let split = digits.len() - scale;
        token[length..length + split].copy_from_slice(&digits[..split]);
        length += split;
        token[length] = b'.';
        length += 1;
        token[length..length + scale].copy_from_slice(&digits[split..]);
        length += scale;
    }
    if scale > 0 {
        while token.get(length.wrapping_sub(1)) == Some(&b'0') {
            length -= 1;
        }
        if token.get(length.wrapping_sub(1)) == Some(&b'.') {
            length -= 1;
        }
    }
    output.extend(&token[..length])
}

fn write_dictionary(
    output: &mut LimitedPdfBuffer,
    dictionary: &PdfDictionary,
) -> Result<(), PdfError> {
    output.extend(b"<<")?;
    for (key, value) in dictionary {
        output.push(b' ')?;
        write_pdf_name(output, key)?;
        output.push(b' ')?;
        write_pdf_value(output, value)?;
    }
    if !dictionary.is_empty() {
        output.push(b' ')?;
    }
    output.extend(b">>")
}

fn write_hex_string(output: &mut LimitedPdfBuffer, bytes: &[u8]) -> Result<(), PdfError> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let encoded_len = u64::try_from(bytes.len())
        .ok()
        .and_then(|len| len.checked_mul(2))
        .and_then(|len| len.checked_add(2))
        .ok_or(PdfError::OutputTooLarge)?;
    if encoded_len > output.remaining()? {
        return Err(PdfError::OutputTooLarge);
    }
    output.push(b'<')?;
    for byte in bytes {
        output.push(HEX[usize::from(byte >> 4)])?;
        output.push(HEX[usize::from(byte & 0x0f)])?;
    }
    output.push(b'>')
}

fn write_stream(
    output: &mut LimitedPdfBuffer,
    dictionary: &PdfDictionary,
    data: &[u8],
    filter: Option<&[u8]>,
) -> Result<(), PdfError> {
    if dictionary
        .keys()
        .any(|key| key.is(b"Length") || key.is(b"Filter") || key.is(b"DecodeParms"))
    {
        return Err(PdfError::ReservedStreamKey);
    }
    let data_len = i64::try_from(data.len()).map_err(|_| PdfError::OutputTooLarge)?;
    output.extend(b"<<")?;
    let mut filter_pending = filter;
    let mut length_pending = true;
    for (key, value) in dictionary {
        if filter_pending.is_some() && b"Filter".as_slice() < key.0.as_slice() {
            let filter = filter_pending
                .take()
                .ok_or(PdfError::ResourcePlanMismatch)?;
            write_filter_entry(output, filter)?;
        }
        if length_pending && b"Length".as_slice() < key.0.as_slice() {
            write_length_entry(output, data_len)?;
            length_pending = false;
        }
        output.push(b' ')?;
        write_pdf_name(output, key)?;
        output.push(b' ')?;
        write_pdf_value(output, value)?;
    }
    if let Some(filter) = filter_pending {
        write_filter_entry(output, filter)?;
    }
    if length_pending {
        write_length_entry(output, data_len)?;
    }
    output.push(b' ')?;
    output.extend(b">>")?;
    output.extend(b"\nstream\n")?;
    output.extend(data)?;
    if data.is_empty() {
        output.extend(b"endstream")
    } else {
        output.extend(b"\nendstream")
    }
}

fn write_filter_entry(output: &mut LimitedPdfBuffer, filter: &[u8]) -> Result<(), PdfError> {
    debug_assert!(filter.iter().all(|byte| byte.is_ascii_alphanumeric()));
    output.extend(b" /Filter /")?;
    output.extend(filter)
}

fn write_length_entry(output: &mut LimitedPdfBuffer, length: i64) -> Result<(), PdfError> {
    output.extend(b" /Length ")?;
    output.integer(length)
}

fn write_generated_stream(
    output: &mut LimitedPdfBuffer,
    dictionary: &PdfDictionary,
    raw_data: &[u8],
    config: &EffectiveConfig,
) -> Result<(), PdfError> {
    match config.stream_compression() {
        PdfStreamCompression::None => write_stream(output, dictionary, raw_data, None),
        PdfStreamCompression::Flate => {
            let encoded = zlib_stored(raw_data, output.remaining()?)?;
            write_stream(output, dictionary, &encoded, Some(b"FlateDecode"))
        }
    }
}

/// Deterministic zlib stream containing only stored DEFLATE blocks. This is a
/// valid `/FlateDecode` payload and avoids a platform-dependent compressor.
fn zlib_stored(input: &[u8], max_len: u64) -> Result<Vec<u8>, PdfError> {
    const BLOCK: usize = u16::MAX as usize;
    let blocks = if input.is_empty() {
        1usize
    } else {
        input
            .len()
            .checked_add(BLOCK - 1)
            .ok_or(PdfError::OutputTooLarge)?
            / BLOCK
    };
    let encoded_len = input
        .len()
        .checked_add(blocks.checked_mul(5).ok_or(PdfError::OutputTooLarge)?)
        .and_then(|len| len.checked_add(6))
        .ok_or(PdfError::OutputTooLarge)?;
    if u64::try_from(encoded_len).map_err(|_| PdfError::OutputTooLarge)? > max_len {
        return Err(PdfError::OutputTooLarge);
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(encoded_len)
        .map_err(|_| PdfError::OutputTooLarge)?;
    // CMF/FLG for DEFLATE, 32 KiB window, fastest/no-compression level.
    output.extend_from_slice(&[0x78, 0x01]);
    if input.is_empty() {
        output.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    } else {
        let total_blocks = blocks;
        for (index, chunk) in input.chunks(BLOCK).enumerate() {
            output.push(if index + 1 == total_blocks {
                0x01
            } else {
                0x00
            });
            let len = u16::try_from(chunk.len()).map_err(|_| PdfError::OutputTooLarge)?;
            output.extend_from_slice(&len.to_le_bytes());
            output.extend_from_slice(&(!len).to_le_bytes());
            output.extend_from_slice(chunk);
        }
    }
    output.extend_from_slice(&adler32(input).to_be_bytes());
    debug_assert_eq!(output.len(), encoded_len);
    Ok(output)
}

fn adler32(bytes: &[u8]) -> u32 {
    const MODULUS: u64 = 65_521;
    let mut a = 1u64;
    let mut b = 0u64;
    // 5,552 bytes is the conventional bound that keeps both accumulators
    // comfortably within an integer word before reduction.
    for chunk in bytes.chunks(5_552) {
        for byte in chunk {
            a += u64::from(*byte);
            b += a;
        }
        a %= MODULUS;
        b %= MODULUS;
    }
    ((b as u32) << 16) | a as u32
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PdfSha256 {
    state: [u32; 8],
    pending: [u8; 64],
    pending_len: usize,
    byte_length: u64,
}

impl PdfSha256 {
    const fn new() -> Self {
        Self {
            state: [
                0x6a09e667u32,
                0xbb67ae85,
                0x3c6ef372,
                0xa54ff53a,
                0x510e527f,
                0x9b05688c,
                0x1f83d9ab,
                0x5be0cd19,
            ],
            pending: [0; 64],
            pending_len: 0,
            byte_length: 0,
        }
    }

    fn update(&mut self, mut bytes: &[u8]) {
        self.byte_length = self.byte_length.wrapping_add(bytes.len() as u64);
        if self.pending_len != 0 {
            let copied = (64 - self.pending_len).min(bytes.len());
            let end = self.pending_len + copied;
            self.pending[self.pending_len..end].copy_from_slice(&bytes[..copied]);
            self.pending_len = end;
            bytes = &bytes[copied..];
            if self.pending_len == 64 {
                sha256_compress(&mut self.state, &self.pending);
                self.pending_len = 0;
            } else {
                return;
            }
        }

        let mut blocks = bytes.chunks_exact(64);
        for block in &mut blocks {
            sha256_compress(&mut self.state, block);
        }
        let remainder = blocks.remainder();
        self.pending[..remainder.len()].copy_from_slice(remainder);
        self.pending_len = remainder.len();
    }

    fn finish(mut self) -> [u8; 32] {
        let bit_length = self.byte_length.wrapping_mul(8);
        self.pending[self.pending_len] = 0x80;
        if self.pending_len >= 56 {
            self.pending[self.pending_len + 1..].fill(0);
            sha256_compress(&mut self.state, &self.pending);
            self.pending.fill(0);
        } else {
            self.pending[self.pending_len + 1..56].fill(0);
        }
        self.pending[56..].copy_from_slice(&bit_length.to_be_bytes());
        sha256_compress(&mut self.state, &self.pending);

        let mut digest = [0u8; 32];
        for (chunk, word) in digest.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        digest
    }
}

/// SHA-256 without constructing a second, padded copy of the complete PDF.
fn pdf_sha256(bytes: &[u8]) -> [u8; 32] {
    let mut sha256 = PdfSha256::new();
    sha256.update(bytes);
    sha256.finish()
}

fn sha256_compress(state: &mut [u32; 8], chunk: &[u8]) {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    debug_assert_eq!(chunk.len(), 64);
    let mut words = [0u32; 64];
    for (index, word) in words[..16].iter_mut().enumerate() {
        let start = index * 4;
        *word = u32::from_be_bytes([
            chunk[start],
            chunk[start + 1],
            chunk[start + 2],
            chunk[start + 3],
        ]);
    }
    for index in 16..64 {
        let s0 = words[index - 15].rotate_right(7)
            ^ words[index - 15].rotate_right(18)
            ^ (words[index - 15] >> 3);
        let s1 = words[index - 2].rotate_right(17)
            ^ words[index - 2].rotate_right(19)
            ^ (words[index - 2] >> 10);
        words[index] = words[index - 16]
            .wrapping_add(s0)
            .wrapping_add(words[index - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for index in 0..64 {
        let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choice = (e & f) ^ ((!e) & g);
        let first = h
            .wrapping_add(sum1)
            .wrapping_add(choice)
            .wrapping_add(K[index])
            .wrapping_add(words[index]);
        let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let second = sum0.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(first);
        d = c;
        c = b;
        b = a;
        a = first.wrapping_add(second);
    }
    for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *slot = slot.wrapping_add(value);
    }
}

fn to_unicode_cmap(plan: &FrozenPdfFontPlan, max_len: u64) -> Result<Vec<u8>, PdfError> {
    let mut output = LimitedPdfBuffer::new(max_len);
    output.extend(
        b"/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n\
/CMapName /Typaxis-Identity-UCS def\n\
/CMapType 2 def\n\
1 begincodespacerange\n\
<0000> <FFFF>\n\
endcodespacerange\n",
    )?;
    let mapping_count = plan
        .subset_plan()
        .cids
        .iter()
        .filter(|binding| !binding.unicode.is_empty())
        .count();
    // A bfchar entry has at least a four-hex-digit source and destination.
    let minimum_mapping_bytes = u64::try_from(mapping_count)
        .ok()
        .and_then(|count| count.checked_mul(14))
        .ok_or(PdfError::OutputTooLarge)?;
    if minimum_mapping_bytes > output.remaining()? {
        return Err(PdfError::OutputTooLarge);
    }
    let mut mappings = Vec::new();
    mappings
        .try_reserve_exact(mapping_count)
        .map_err(|_| PdfError::OutputTooLarge)?;
    mappings.extend(
        plan.subset_plan()
            .cids
            .iter()
            .filter(|binding| !binding.unicode.is_empty()),
    );
    for chunk in mappings.chunks(100) {
        output.unsigned(u64::try_from(chunk.len()).map_err(|_| PdfError::OutputTooLarge)?)?;
        output.extend(b" beginbfchar\n")?;
        for binding in chunk {
            write_hex_string(&mut output, &binding.cid.get().to_be_bytes())?;
            output.push(b' ')?;
            write_utf16be_hex(
                &mut output,
                binding.unicode.iter().map(|scalar| scalar.get()),
                false,
            )?;
            output.push(b'\n')?;
        }
        output.extend(b"endbfchar\n")?;
    }
    output.extend(
        b"endcmap\n\
CMapName currentdict /CMap defineresource pop\n\
end\n\
end\n",
    )?;
    Ok(output.into_bytes())
}

fn write_utf16be_hex(
    output: &mut LimitedPdfBuffer,
    scalars: impl IntoIterator<Item = char>,
    bom: bool,
) -> Result<(), PdfError> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    output.push(b'<')?;
    if bom {
        output.extend(b"FEFF")?;
    }
    for scalar in scalars {
        let mut units = [0u16; 2];
        for unit in scalar.encode_utf16(&mut units) {
            for byte in unit.to_be_bytes() {
                output.push(HEX[usize::from(byte >> 4)])?;
                output.push(HEX[usize::from(byte & 0x0f)])?;
            }
        }
    }
    output.push(b'>')
}

fn cid_to_gid_map(plan: &FrozenPdfFontPlan, max_len: u64) -> Result<Vec<u8>, PdfError> {
    let byte_len = plan
        .subset_plan()
        .cids
        .len()
        .checked_add(1)
        .and_then(|count| count.checked_mul(2))
        .ok_or(PdfError::OutputTooLarge)?;
    if u64::try_from(byte_len).map_err(|_| PdfError::OutputTooLarge)? > max_len {
        return Err(PdfError::OutputTooLarge);
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(byte_len)
        .map_err(|_| PdfError::OutputTooLarge)?;
    output.extend_from_slice(&0u16.to_be_bytes());
    for (index, binding) in plan.subset_plan().cids.iter().enumerate() {
        if usize::from(binding.cid.get()) != index + 1 {
            return Err(PdfError::ResourcePlanMismatch);
        }
        output.extend_from_slice(&binding.subset_gid.get().to_be_bytes());
    }
    Ok(output)
}

fn write_image_stream(
    output: &mut LimitedPdfBuffer,
    plan: &FrozenPdfImagePlan,
    alpha_mask_object: Option<ObjectId>,
    config: &EffectiveConfig,
) -> Result<(), PdfError> {
    let mut dictionary = PdfDictionary::new();
    dictionary.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"XObject")?));
    dictionary.insert(pdf_name(b"Subtype")?, PdfValue::Name(pdf_name(b"Image")?));
    dictionary.insert(
        pdf_name(b"Width")?,
        PdfValue::Integer(i64::from(plan.width().get())),
    );
    dictionary.insert(
        pdf_name(b"Height")?,
        PdfValue::Integer(i64::from(plan.height().get())),
    );
    dictionary.insert(
        pdf_name(b"ColorSpace")?,
        PdfValue::Name(pdf_name(match plan.color_space() {
            ImageColorSpace::Gray => b"DeviceGray",
            ImageColorSpace::Rgb => b"DeviceRGB",
            ImageColorSpace::Cmyk => b"DeviceCMYK",
        })?),
    );
    dictionary.insert(
        pdf_name(b"BitsPerComponent")?,
        PdfValue::Integer(i64::from(plan.bits_per_component())),
    );
    if let Some(mask) = alpha_mask_object {
        dictionary.insert(pdf_name(b"SMask")?, PdfValue::Reference(mask));
    }
    match plan.encoding() {
        ImageEncoding::Raw => {
            write_generated_stream(output, &dictionary, plan.encoded_bytes(), config)
        }
        ImageEncoding::Flate => write_stream(
            output,
            &dictionary,
            plan.encoded_bytes(),
            Some(b"FlateDecode"),
        ),
        ImageEncoding::Jpeg => write_stream(
            output,
            &dictionary,
            plan.encoded_bytes(),
            Some(b"DCTDecode"),
        ),
    }
}

fn write_alpha_mask_stream(
    output: &mut LimitedPdfBuffer,
    mask: &FrozenPdfAlphaMask,
    config: &EffectiveConfig,
) -> Result<(), PdfError> {
    let mut dictionary = PdfDictionary::new();
    dictionary.insert(pdf_name(b"Type")?, PdfValue::Name(pdf_name(b"XObject")?));
    dictionary.insert(pdf_name(b"Subtype")?, PdfValue::Name(pdf_name(b"Image")?));
    dictionary.insert(
        pdf_name(b"Width")?,
        PdfValue::Integer(i64::from(mask.width().get())),
    );
    dictionary.insert(
        pdf_name(b"Height")?,
        PdfValue::Integer(i64::from(mask.height().get())),
    );
    dictionary.insert(
        pdf_name(b"ColorSpace")?,
        PdfValue::Name(pdf_name(b"DeviceGray")?),
    );
    dictionary.insert(
        pdf_name(b"BitsPerComponent")?,
        PdfValue::Integer(i64::from(mask.bits_per_component())),
    );
    match mask.encoding() {
        ImageEncoding::Raw => {
            write_generated_stream(output, &dictionary, mask.encoded_bytes(), config)
        }
        ImageEncoding::Flate => write_stream(
            output,
            &dictionary,
            mask.encoded_bytes(),
            Some(b"FlateDecode"),
        ),
        ImageEncoding::Jpeg => Err(PdfError::ResourcePlanMismatch),
    }
}

fn page_content_stream(
    page: &DisplayPage,
    context: &PdfSerializationContext<'_>,
    max_len: u64,
) -> Result<Vec<u8>, PdfError> {
    let mut output = LimitedPdfBuffer::new(max_len);
    output.extend(b"q\n1 0 0 -1 0 ")?;
    write_length_token(&mut output, page.height.get())?;
    output.extend(b" cm\n")?;
    for command in &page.commands {
        write_display_command(&mut output, command, context)?;
    }
    output.extend(b"Q\n")?;
    Ok(output.into_bytes())
}

fn write_display_command(
    output: &mut LimitedPdfBuffer,
    command: &DisplayCommand,
    context: &PdfSerializationContext<'_>,
) -> Result<(), PdfError> {
    match command {
        DisplayCommand::Save => output.extend(b"q\n"),
        DisplayCommand::Restore => output.extend(b"Q\n"),
        DisplayCommand::ConcatTransform { matrix } => {
            write_fixed_token(output, i128::from(matrix.a.raw()), 65_536)?;
            output.push(b' ')?;
            write_fixed_token(output, i128::from(matrix.b.raw()), 65_536)?;
            output.push(b' ')?;
            write_fixed_token(output, i128::from(matrix.c.raw()), 65_536)?;
            output.push(b' ')?;
            write_fixed_token(output, i128::from(matrix.d.raw()), 65_536)?;
            output.push(b' ')?;
            write_length_token(output, matrix.e)?;
            output.push(b' ')?;
            write_length_token(output, matrix.f)?;
            output.extend(b" cm\n")
        }
        DisplayCommand::ClipPath { path, rule } => {
            write_path(output, path)?;
            output.extend(match rule {
                FillRule::NonZero => b"W n\n",
                FillRule::EvenOdd => b"W* n\n",
            })
        }
        DisplayCommand::FillPath { path, paint, rule } => {
            write_paint(output, *paint, false)?;
            write_path(output, path)?;
            output.extend(match rule {
                FillRule::NonZero => b"f\n",
                FillRule::EvenOdd => b"f*\n",
            })
        }
        DisplayCommand::StrokePath {
            path,
            paint,
            stroke,
        } => {
            write_paint(output, *paint, true)?;
            write_length_token(output, stroke.width.get())?;
            output.extend(b" w\n")?;
            output.integer(match stroke.line_cap {
                LineCap::Butt => 0,
                LineCap::Round => 1,
                LineCap::Square => 2,
            })?;
            output.extend(b" J\n")?;
            output.integer(match stroke.line_join {
                LineJoin::Miter => 0,
                LineJoin::Round => 1,
                LineJoin::Bevel => 2,
            })?;
            output.extend(b" j\n")?;
            write_fixed_token(output, i128::from(stroke.miter_limit.get().raw()), 65_536)?;
            output.extend(b" M\n[")?;
            for (index, dash) in stroke.dash.array().iter().enumerate() {
                if index > 0 {
                    output.push(b' ')?;
                }
                write_length_token(output, dash.get())?;
            }
            output.extend(b"] ")?;
            write_length_token(output, stroke.dash.phase().get())?;
            output.extend(b" d\n")?;
            write_path(output, path)?;
            output.extend(b"S\n")
        }
        DisplayCommand::DrawImage { image_id, rect } => {
            let name = context
                .image_name(*image_id)
                .ok_or(PdfError::ResourcePlanMismatch)?;
            write_image_placement(output, name, *rect)
        }
        DisplayCommand::DrawGlyphRun {
            font_instance_id,
            origin,
            font_size,
            fill,
            glyphs,
            clusters,
            ..
        } => write_glyph_run(
            output,
            *font_instance_id,
            *origin,
            *font_size,
            *fill,
            glyphs,
            clusters,
            context,
        ),
    }
}

fn write_image_placement(
    output: &mut LimitedPdfBuffer,
    name: &PdfName,
    rect: Rect,
) -> Result<(), PdfError> {
    // The page CTM reflects the internal Y-down coordinate system. PDF image
    // samples already run from the top row downward, so counter-reflect the
    // unit image here (as text does in its text matrix) to keep it upright.
    let negative_height =
        Length::from_raw(-rect.height().get().raw()).ok_or(PdfError::ContentStream)?;
    let bottom = rect
        .y()
        .checked_add(rect.height().get())
        .ok_or(PdfError::ContentStream)?;
    output.extend(b"q\n")?;
    write_length_token(output, rect.width().get())?;
    output.extend(b" 0 0 ")?;
    write_length_token(output, negative_height)?;
    output.push(b' ')?;
    write_length_token(output, rect.x())?;
    output.push(b' ')?;
    write_length_token(output, bottom)?;
    output.extend(b" cm\n")?;
    write_pdf_name(output, name)?;
    output.extend(b" Do\nQ\n")
}

fn write_path(output: &mut LimitedPdfBuffer, path: &Path) -> Result<(), PdfError> {
    for verb in path.verbs() {
        match verb {
            PathVerb::MoveTo(point) => write_point_operator(output, *point, b"m\n")?,
            PathVerb::LineTo(point) => write_point_operator(output, *point, b"l\n")?,
            PathVerb::CurveTo(first, second, third) => {
                for point in [first, second, third] {
                    write_length_token(output, point.x)?;
                    output.push(b' ')?;
                    write_length_token(output, point.y)?;
                    output.push(b' ')?;
                }
                output.extend(b"c\n")?;
            }
            PathVerb::Close => output.extend(b"h\n")?,
        }
    }
    Ok(())
}

fn write_point_operator(
    output: &mut LimitedPdfBuffer,
    point: Point,
    operator: &[u8],
) -> Result<(), PdfError> {
    write_length_token(output, point.x)?;
    output.push(b' ')?;
    write_length_token(output, point.y)?;
    output.push(b' ')?;
    output.extend(operator)
}

fn write_paint(output: &mut LimitedPdfBuffer, paint: Paint, stroke: bool) -> Result<(), PdfError> {
    let operator = match (paint, stroke) {
        (Paint::Gray(gray), false) => {
            write_fixed_token(output, i128::from(gray), 65_535)?;
            b" g\n".as_slice()
        }
        (Paint::Gray(gray), true) => {
            write_fixed_token(output, i128::from(gray), 65_535)?;
            b" G\n".as_slice()
        }
        (Paint::Rgb { r, g, b }, false) => {
            write_color_components(output, &[r, g, b])?;
            b" rg\n".as_slice()
        }
        (Paint::Rgb { r, g, b }, true) => {
            write_color_components(output, &[r, g, b])?;
            b" RG\n".as_slice()
        }
        (Paint::Cmyk { c, m, y, k }, false) => {
            write_color_components(output, &[c, m, y, k])?;
            b" k\n".as_slice()
        }
        (Paint::Cmyk { c, m, y, k }, true) => {
            write_color_components(output, &[c, m, y, k])?;
            b" K\n".as_slice()
        }
    };
    output.extend(operator)
}

fn write_color_components(
    output: &mut LimitedPdfBuffer,
    components: &[u16],
) -> Result<(), PdfError> {
    for (index, component) in components.iter().enumerate() {
        if index > 0 {
            output.push(b' ')?;
        }
        write_fixed_token(output, i128::from(*component), 65_535)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_glyph_run(
    output: &mut LimitedPdfBuffer,
    font_instance_id: FontInstanceId,
    origin: Point,
    font_size: PositiveLength,
    fill: Paint,
    glyphs: &[DisplayGlyph],
    clusters: &[typaxis_display_list::DisplayCluster],
    context: &PdfSerializationContext<'_>,
) -> Result<(), PdfError> {
    let font_name = context
        .font_name(font_instance_id)
        .ok_or(PdfError::ResourcePlanMismatch)?;
    let font_plan = context
        .font_plan(font_instance_id)
        .ok_or(PdfError::ResourcePlanMismatch)?;

    // Each glyph necessarily emits a text matrix, a two-byte CID hex string,
    // and a show operator. This conservative lower bound keeps the transient
    // position table smaller than the remaining possible output.
    let minimum_glyph_bytes = u64::try_from(glyphs.len())
        .ok()
        .and_then(|count| count.checked_mul(20))
        .ok_or(PdfError::OutputTooLarge)?;
    if minimum_glyph_bytes > output.remaining()? {
        return Err(PdfError::OutputTooLarge);
    }
    let mut positions = Vec::new();
    positions
        .try_reserve_exact(glyphs.len())
        .map_err(|_| PdfError::OutputTooLarge)?;
    let mut pen_x = Length::ZERO;
    let mut pen_y = Length::ZERO;
    for glyph in glyphs {
        let x = origin
            .x
            .checked_add(pen_x)
            .and_then(|value| value.checked_add(glyph.offset_x))
            .ok_or(PdfError::ContentStream)?;
        let y = origin
            .y
            .checked_add(pen_y)
            .and_then(|value| value.checked_add(glyph.offset_y))
            .ok_or(PdfError::ContentStream)?;
        positions.push(Point { x, y });
        pen_x = pen_x
            .checked_add(glyph.advance_x)
            .ok_or(PdfError::ContentStream)?;
        pen_y = pen_y
            .checked_add(glyph.advance_y)
            .ok_or(PdfError::ContentStream)?;
    }

    write_paint(output, fill, false)?;
    output.extend(b"BT\n")?;
    write_pdf_name(output, font_name)?;
    output.push(b' ')?;
    write_length_token(output, font_size.get())?;
    output.extend(b" Tf\n")?;
    // Display clusters are deliberately stored in logical order even when
    // their glyph ranges are in reverse visual order for RTL runs. Absolute
    // text matrices preserve every glyph's visual position while logical
    // emission order preserves ToUnicode/ActualText extraction order.
    for cluster in clusters {
        let start = usize::try_from(cluster.glyph_start).map_err(|_| PdfError::ContentStream)?;
        let end = usize::try_from(cluster.glyph_end).map_err(|_| PdfError::ContentStream)?;
        let cluster_glyphs = glyphs.get(start..end).ok_or(PdfError::ContentStream)?;
        let extraction = cluster_plan_for(font_plan, &cluster.extraction, cluster_glyphs)
            .ok_or(PdfError::ResourcePlanMismatch)?;
        match extraction {
            ClusterExtractionPlan::ActualText { unicode, .. } => {
                output.extend(b"/Span << /ActualText ")?;
                write_utf16be_hex(output, unicode.iter().map(|scalar| scalar.get()), true)?;
                output.extend(b" >> BDC\n")?;
            }
            ClusterExtractionPlan::Artifact { .. } => {
                // A zero-length ActualText prevents extractors that ignore the
                // Artifact tag from falling back to the raw CID value.
                output.extend(b"/Artifact << /ActualText <> >> BDC\n")?;
            }
            ClusterExtractionPlan::PerCid { .. } => {}
        }
        let cid_count = cluster_plan_cid_count(extraction);
        if cid_count != cluster_glyphs.len() {
            return Err(PdfError::ResourcePlanMismatch);
        }
        for local_index in 0..cid_count {
            let cid =
                cluster_plan_cid(extraction, local_index).ok_or(PdfError::ResourcePlanMismatch)?;
            let position = positions
                .get(start + local_index)
                .ok_or(PdfError::ContentStream)?;
            output.extend(b"1 0 0 -1 ")?;
            write_length_token(output, position.x)?;
            output.push(b' ')?;
            write_length_token(output, position.y)?;
            output.extend(b" Tm ")?;
            write_hex_string(output, &cid.to_be_bytes())?;
            output.extend(b" Tj\n")?;
        }
        if matches!(
            extraction,
            ClusterExtractionPlan::ActualText { .. } | ClusterExtractionPlan::Artifact { .. }
        ) {
            output.extend(b"EMC\n")?;
        }
    }
    output.extend(b"ET\n")
}

fn cluster_plan_cid_count(plan: &ClusterExtractionPlan) -> usize {
    match plan {
        ClusterExtractionPlan::PerCid { cids, .. }
        | ClusterExtractionPlan::ActualText { cids, .. }
        | ClusterExtractionPlan::Artifact { cids } => cids.len(),
    }
}

fn cluster_plan_cid(plan: &ClusterExtractionPlan, index: usize) -> Option<u16> {
    match plan {
        ClusterExtractionPlan::PerCid { cids, .. }
        | ClusterExtractionPlan::ActualText { cids, .. }
        | ClusterExtractionPlan::Artifact { cids } => cids.get(index).map(|cid| cid.get()),
    }
}

fn cluster_plan_for<'a>(
    font: &'a FrozenPdfFontPlan,
    extraction: &ClusterExtraction,
    glyphs: &[DisplayGlyph],
) -> Option<&'a ClusterExtractionPlan> {
    font.cluster_plans().iter().find(|plan| {
        let extraction_matches = match (extraction, plan) {
            (
                ClusterExtraction::Unicode { text_span },
                ClusterExtractionPlan::PerCid {
                    text_span: planned, ..
                }
                | ClusterExtractionPlan::ActualText {
                    text_span: planned, ..
                },
            ) => text_span == planned,
            (ClusterExtraction::Artifact, ClusterExtractionPlan::Artifact { .. }) => true,
            _ => false,
        };
        if !extraction_matches {
            return false;
        }
        if cluster_plan_cid_count(plan) != glyphs.len() {
            return false;
        }
        glyphs.iter().enumerate().all(|(index, glyph)| {
            let Some(cid) = cluster_plan_cid(plan, index) else {
                return false;
            };
            let subset = font
                .subset_plan()
                .glyphs
                .iter()
                .find(|binding| binding.original_gid == glyph.original_gid)
                .map(|binding| binding.subset_gid);
            let cid_subset = font
                .subset_plan()
                .cids
                .get(usize::from(cid) - 1)
                .filter(|binding| binding.cid.get() == cid)
                .map(|binding| binding.subset_gid);
            subset.is_some() && subset == cid_subset
        })
    })
}

fn write_length_token(output: &mut LimitedPdfBuffer, length: Length) -> Result<(), PdfError> {
    write_fixed_token(output, i128::from(length.raw()), 65_536)
}

fn write_fixed_token(
    output: &mut LimitedPdfBuffer,
    numerator: i128,
    denominator: i128,
) -> Result<(), PdfError> {
    if numerator == 0 {
        return output.push(b'0');
    }
    let scaled = numerator
        .checked_mul(1_000_000)
        .ok_or(PdfError::ContentStream)?;
    let coefficient = round_ratio_ties_even(scaled, denominator)
        .and_then(|value| i64::try_from(value).ok())
        .ok_or(PdfError::ContentStream)?;
    write_pdf_decimal(output, PdfDecimal::new(coefficient, 6)?)
}

fn round_ratio_ties_even(numerator: i128, denominator: i128) -> Option<i128> {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let doubled = remainder.unsigned_abs().checked_mul(2)?;
    let denominator_abs = denominator.unsigned_abs();
    let adjustment = if remainder.is_negative() { -1 } else { 1 };
    if doubled < denominator_abs || (doubled == denominator_abs && quotient % 2 == 0) {
        Some(quotient)
    } else {
        quotient.checked_add(adjustment)
    }
}

fn dictionary_for(
    objects: &BTreeMap<ObjectId, IndirectObjectBody>,
    id: ObjectId,
) -> Result<&PdfDictionary, PdfError> {
    match objects.get(&id) {
        Some(IndirectObjectBody::Value(PdfValue::Dictionary(value))) => Ok(value),
        _ => Err(PdfError::InvalidPageTree),
    }
}
fn dict_value<'a>(dict: &'a PdfDictionary, key: &[u8]) -> Option<&'a PdfValue> {
    dict.iter()
        .find_map(|(name, value)| if name.is(key) { Some(value) } else { None })
}
fn type_is(dict: &PdfDictionary, expected: &[u8]) -> bool {
    matches!(dict_value(dict, b"Type"), Some(PdfValue::Name(name)) if name.is(expected))
}
fn validate_page_tree(
    objects: &BTreeMap<ObjectId, IndirectObjectBody>,
    root: ObjectId,
) -> Result<(), PdfError> {
    let catalog = dictionary_for(objects, root).map_err(|_| PdfError::RootIsNotCatalog)?;
    if !type_is(catalog, b"Catalog") {
        return Err(PdfError::RootIsNotCatalog);
    }
    let pages = match dict_value(catalog, b"Pages") {
        Some(PdfValue::Reference(id)) => *id,
        _ => return Err(PdfError::CatalogMissingPages),
    };
    let pages_dictionary = dictionary_for(objects, pages)?;
    if !type_is(pages_dictionary, b"Pages") || dict_value(pages_dictionary, b"Parent").is_some() {
        return Err(PdfError::InvalidPageTree);
    }
    let visited = validate_page_tree_nodes(objects, pages)?;
    for (id, body) in objects {
        if let IndirectObjectBody::Value(PdfValue::Dictionary(dictionary)) = body {
            if (type_is(dictionary, b"Page") || type_is(dictionary, b"Pages"))
                && !visited.contains(id)
            {
                return Err(PdfError::InvalidPageTree);
            }
        }
    }
    validate_destinations_and_annotations(objects, catalog)?;
    Ok(())
}

fn validate_destinations_and_annotations(
    objects: &BTreeMap<ObjectId, IndirectObjectBody>,
    catalog: &PdfDictionary,
) -> Result<(), PdfError> {
    let page_ids: BTreeSet<_> = objects
        .iter()
        .filter_map(|(id, body)| match body {
            IndirectObjectBody::Value(PdfValue::Dictionary(dictionary))
                if type_is(dictionary, b"Page") =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect();
    let destinations = validate_destination_name_tree(catalog, &page_ids)?;
    let mut referenced_annotations = BTreeSet::new();
    for page_id in &page_ids {
        let page =
            dictionary_for(objects, *page_id).map_err(|_| PdfError::InvalidAnnotationClosure)?;
        let Some(annotations) = dict_value(page, b"Annots") else {
            continue;
        };
        let PdfValue::Array(annotations) = annotations else {
            return Err(PdfError::InvalidAnnotationClosure);
        };
        for annotation in annotations {
            let PdfValue::Reference(annotation_id) = annotation else {
                return Err(PdfError::InvalidAnnotationClosure);
            };
            if !referenced_annotations.insert(*annotation_id) {
                return Err(PdfError::InvalidAnnotationClosure);
            }
            let dictionary = dictionary_for(objects, *annotation_id)
                .map_err(|_| PdfError::InvalidAnnotationClosure)?;
            validate_link_annotation(dictionary, &destinations)?;
        }
    }
    for (id, body) in objects {
        if let IndirectObjectBody::Value(PdfValue::Dictionary(dictionary)) = body {
            if type_is(dictionary, b"Annot") && !referenced_annotations.contains(id) {
                return Err(PdfError::InvalidAnnotationClosure);
            }
        }
    }
    Ok(())
}

fn validate_destination_name_tree(
    catalog: &PdfDictionary,
    page_ids: &BTreeSet<ObjectId>,
) -> Result<BTreeSet<Vec<u8>>, PdfError> {
    let Some(names) = dict_value(catalog, b"Names") else {
        return Ok(BTreeSet::new());
    };
    let PdfValue::Dictionary(names) = names else {
        return Err(PdfError::InvalidDestinationClosure);
    };
    if names.len() != 1 {
        return Err(PdfError::InvalidDestinationClosure);
    }
    let Some(PdfValue::Dictionary(destinations)) = dict_value(names, b"Dests") else {
        return Err(PdfError::InvalidDestinationClosure);
    };
    if destinations.len() != 1 {
        return Err(PdfError::InvalidDestinationClosure);
    }
    let Some(PdfValue::Array(values)) = dict_value(destinations, b"Names") else {
        return Err(PdfError::InvalidDestinationClosure);
    };
    if values.is_empty() || values.len() % 2 != 0 {
        return Err(PdfError::InvalidDestinationClosure);
    }
    let mut result = BTreeSet::new();
    let mut previous: Option<&[u8]> = None;
    for entry in values.chunks_exact(2) {
        let PdfValue::ByteString(name) = &entry[0] else {
            return Err(PdfError::InvalidDestinationClosure);
        };
        if !AnchorId::is_valid(std::str::from_utf8(name).unwrap_or_default())
            || previous.is_some_and(|previous| previous >= name.as_slice())
            || !result.insert(name.clone())
            || !valid_destination_value(&entry[1], page_ids)
        {
            return Err(PdfError::InvalidDestinationClosure);
        }
        previous = Some(name);
    }
    Ok(result)
}

fn valid_destination_value(value: &PdfValue, page_ids: &BTreeSet<ObjectId>) -> bool {
    let PdfValue::Array(values) = value else {
        return false;
    };
    let Some(PdfValue::Reference(page)) = values.first() else {
        return false;
    };
    if !page_ids.contains(page) {
        return false;
    }
    match values.get(1) {
        Some(PdfValue::Name(view)) if view.is(b"XYZ") => {
            values.len() == 5
                && pdf_number(&values[2]).is_some()
                && pdf_number(&values[3]).is_some()
                && values[4] == PdfValue::Null
        }
        Some(PdfValue::Name(view)) if view.is(b"Fit") => values.len() == 2,
        Some(PdfValue::Name(view)) if view.is(b"FitH") => {
            values.len() == 3 && (values[2] == PdfValue::Null || pdf_number(&values[2]).is_some())
        }
        _ => false,
    }
}

fn validate_link_annotation(
    dictionary: &PdfDictionary,
    destinations: &BTreeSet<Vec<u8>>,
) -> Result<(), PdfError> {
    if dictionary.len() != 5
        || !type_is(dictionary, b"Annot")
        || !matches!(dict_value(dictionary, b"Subtype"), Some(PdfValue::Name(name)) if name.is(b"Link"))
        || !dict_value(dictionary, b"Rect").is_some_and(valid_page_box)
        || !matches!(dict_value(dictionary, b"Border"), Some(PdfValue::Array(values)) if values == &[PdfValue::Integer(0), PdfValue::Integer(0), PdfValue::Integer(0)])
    {
        return Err(PdfError::InvalidAnnotationClosure);
    }
    match (
        dict_value(dictionary, b"Dest"),
        dict_value(dictionary, b"A"),
    ) {
        (Some(PdfValue::ByteString(destination)), None) if destinations.contains(destination) => {
            Ok(())
        }
        (None, Some(PdfValue::Dictionary(action)))
            if action.len() == 2
                && matches!(dict_value(action, b"S"), Some(PdfValue::Name(name)) if name.is(b"URI"))
                && matches!(dict_value(action, b"URI"), Some(PdfValue::ByteString(uri)) if !uri.is_empty()) =>
        {
            Ok(())
        }
        _ => Err(PdfError::InvalidAnnotationClosure),
    }
}
const MAX_PDF_PAGE_TREE_DEPTH: usize = 64;

enum PageTreeWork {
    Enter {
        id: ObjectId,
        expected_parent: Option<ObjectId>,
        inherited_media_box: Option<PdfValue>,
        depth: usize,
    },
    Exit {
        id: ObjectId,
        kids: Vec<ObjectId>,
    },
}

fn validate_page_tree_nodes(
    objects: &BTreeMap<ObjectId, IndirectObjectBody>,
    root: ObjectId,
) -> Result<BTreeSet<ObjectId>, PdfError> {
    let mut pending = vec![PageTreeWork::Enter {
        id: root,
        expected_parent: None,
        inherited_media_box: None,
        depth: 1,
    }];
    let mut active = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut descendant_counts = BTreeMap::new();
    while let Some(work) = pending.pop() {
        match work {
            PageTreeWork::Enter {
                id,
                expected_parent,
                inherited_media_box,
                depth,
            } => {
                if depth > MAX_PDF_PAGE_TREE_DEPTH {
                    return Err(PdfError::PageTreeDepth);
                }
                if active.contains(&id) {
                    return Err(PdfError::PageTreeCycle);
                }
                if !visited.insert(id) {
                    return Err(PdfError::InvalidPageTree);
                }
                active.insert(id);
                let dict = dictionary_for(objects, id)?;
                if let Some(parent) = expected_parent {
                    if !matches!(dict_value(dict, b"Parent"), Some(PdfValue::Reference(found)) if *found == parent)
                    {
                        return Err(PdfError::InvalidPageTree);
                    }
                }
                let own_media_box = dict_value(dict, b"MediaBox");
                if own_media_box.is_some_and(|value| !valid_page_box(value)) {
                    return Err(PdfError::InvalidPageTree);
                }
                let effective_media_box = own_media_box.cloned().or(inherited_media_box);
                if dict_value(dict, b"CropBox").is_some_and(|value| !valid_page_box(value))
                    || dict_value(dict, b"Rotate").is_some_and(|value| {
                        !matches!(value, PdfValue::Integer(angle) if matches!(*angle, 0 | 90 | 180 | 270))
                    })
                {
                    return Err(PdfError::InvalidPageTree);
                }
                if type_is(dict, b"Page") {
                    if effective_media_box.is_none()
                        || dict_value(dict, b"Kids").is_some()
                        || dict_value(dict, b"Count").is_some()
                    {
                        return Err(PdfError::InvalidPageTree);
                    }
                    descendant_counts.insert(id, 1u32);
                    active.remove(&id);
                    continue;
                }
                if !type_is(dict, b"Pages")
                    || dict_value(dict, b"Contents").is_some()
                    || dict_value(dict, b"Annots").is_some()
                {
                    return Err(PdfError::InvalidPageTree);
                }
                let Some(PdfValue::Array(values)) = dict_value(dict, b"Kids") else {
                    return Err(PdfError::InvalidPageTree);
                };
                if values.is_empty() {
                    return Err(PdfError::InvalidPageTree);
                }
                let kids: Vec<_> = values
                    .iter()
                    .map(|value| match value {
                        PdfValue::Reference(kid) => Ok(*kid),
                        _ => Err(PdfError::InvalidPageTree),
                    })
                    .collect::<Result<_, _>>()?;
                pending.push(PageTreeWork::Exit {
                    id,
                    kids: kids.clone(),
                });
                let child_depth = depth.checked_add(1).ok_or(PdfError::PageTreeDepth)?;
                for kid in kids.into_iter().rev() {
                    pending.push(PageTreeWork::Enter {
                        id: kid,
                        expected_parent: Some(id),
                        inherited_media_box: effective_media_box.clone(),
                        depth: child_depth,
                    });
                }
            }
            PageTreeWork::Exit { id, kids } => {
                let actual = kids.iter().try_fold(0u32, |count, kid| {
                    count
                        .checked_add(
                            *descendant_counts
                                .get(kid)
                                .ok_or(PdfError::InvalidPageTree)?,
                        )
                        .ok_or(PdfError::InvalidPageTree)
                })?;
                let dict = dictionary_for(objects, id)?;
                if !matches!(dict_value(dict, b"Count"), Some(PdfValue::Integer(value)) if *value >= 0 && u32::try_from(*value).ok() == Some(actual))
                {
                    return Err(PdfError::InvalidPageTree);
                }
                descendant_counts.insert(id, actual);
                active.remove(&id);
            }
        }
    }
    Ok(visited)
}

fn valid_page_box(value: &PdfValue) -> bool {
    let PdfValue::Array(values) = value else {
        return false;
    };
    if values.len() != 4 {
        return false;
    }
    let Some(left) = pdf_number(&values[0]) else {
        return false;
    };
    let Some(bottom) = pdf_number(&values[1]) else {
        return false;
    };
    let Some(right) = pdf_number(&values[2]) else {
        return false;
    };
    let Some(top) = pdf_number(&values[3]) else {
        return false;
    };
    left < right && bottom < top
}

fn pdf_number(value: &PdfValue) -> Option<i128> {
    match value {
        PdfValue::Integer(value) => i128::from(*value).checked_mul(1_000_000_000_000),
        PdfValue::Decimal(value) => {
            let exponent = 12u8.checked_sub(value.scale)?;
            let factor = 10i128.checked_pow(u32::from(exponent))?;
            i128::from(value.coefficient).checked_mul(factor)
        }
        _ => None,
    }
}
const MAX_PDF_DIRECT_VALUE_DEPTH: usize = 64;

fn collect_references(
    body: &IndirectObjectBody,
    output: &mut BTreeSet<ObjectId>,
) -> Result<(), PdfError> {
    match body {
        IndirectObjectBody::Value(value) => collect_value_references(value, 1, output)?,
        IndirectObjectBody::Stream(stream) => {
            for value in stream.dictionary.values() {
                collect_value_references(value, 2, output)?;
            }
        }
        IndirectObjectBody::FrozenImageResource {
            alpha_mask_object: Some(alpha_mask_object),
            ..
        } => {
            output.insert(*alpha_mask_object);
        }
        IndirectObjectBody::FrozenToUnicodeCMap {
            font_program_object,
        }
        | IndirectObjectBody::FrozenCidToGidMap {
            font_program_object,
        } => {
            output.insert(*font_program_object);
        }
        IndirectObjectBody::FrozenFontProgram(_)
        | IndirectObjectBody::FrozenImageAlphaMask(_)
        | IndirectObjectBody::FrozenImageResource {
            alpha_mask_object: None,
            ..
        }
        | IndirectObjectBody::DisplayPageContent(_) => {}
    }
    Ok(())
}
fn collect_value_references(
    value: &PdfValue,
    root_depth: usize,
    output: &mut BTreeSet<ObjectId>,
) -> Result<(), PdfError> {
    if root_depth == 0 || root_depth > MAX_PDF_DIRECT_VALUE_DEPTH {
        return Err(PdfError::DirectValueDepth);
    }
    let mut pending = vec![(value, root_depth)];
    while let Some((value, depth)) = pending.pop() {
        match value {
            PdfValue::Reference(id) => {
                output.insert(*id);
            }
            PdfValue::Array(values) => {
                if values.is_empty() {
                    continue;
                }
                let child_depth = depth.checked_add(1).ok_or(PdfError::DirectValueDepth)?;
                if child_depth > MAX_PDF_DIRECT_VALUE_DEPTH {
                    return Err(PdfError::DirectValueDepth);
                }
                pending.extend(values.iter().rev().map(|value| (value, child_depth)));
            }
            PdfValue::Dictionary(dictionary) => {
                if dictionary.is_empty() {
                    continue;
                }
                let child_depth = depth.checked_add(1).ok_or(PdfError::DirectValueDepth)?;
                if child_depth > MAX_PDF_DIRECT_VALUE_DEPTH {
                    return Err(PdfError::DirectValueDepth);
                }
                pending.extend(dictionary.values().rev().map(|value| (value, child_depth)));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{EffectiveDataVersions, NodeId, ResourceLimits, ValidatedResourceLimits};
    fn name(value: &[u8]) -> PdfName {
        PdfName::from_bytes(value.to_vec()).unwrap()
    }
    fn effective_config(
        compression: PdfStreamCompression,
        limits: ResourceLimits,
    ) -> EffectiveConfig {
        EffectiveConfig::new(
            false,
            compression,
            vec![],
            vec![],
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
            limits,
        )
        .unwrap()
    }

    fn table_pdf_record_fixture() -> TablePdfCellObservation {
        TablePdfCellObservation {
            kind: TablePaintOccurrenceKind::Header,
            page_index: 1,
            fragment_id: 9,
            source_fragment_id: Some(3),
            repetition_index: Some(1),
            row_node_id: 2,
            logical_row_ordinal: 0,
            row_fragment_ordinal: 0,
            cell_node_id: 3,
            flow_id: 1,
            column_ordinal: 0,
            colspan: 1,
            rowspan: 1,
            rect: TablePaintRect::from_untrusted_parts(10, 20, 30, 40),
            content_fragment_start: 0,
            content_fragment_end: 2,
        }
    }

    #[test]
    fn table_pdf_closure_rejects_missing_extra_wrong_cell_page_repetition_and_content() {
        let expected = table_pdf_record_fixture();
        assert_eq!(
            validate_table_pdf_records(std::slice::from_ref(&expected), &[]),
            Err(TablePdfClosureError::MissingCell)
        );
        assert_eq!(
            validate_table_pdf_records(
                std::slice::from_ref(&expected),
                &[expected.clone(), expected.clone()],
            ),
            Err(TablePdfClosureError::ExtraCell)
        );
        let mut wrong = expected.clone();
        wrong.page_index += 1;
        assert_eq!(
            validate_table_pdf_records(std::slice::from_ref(&expected), &[wrong]),
            Err(TablePdfClosureError::WrongPage)
        );
        let mut wrong = expected.clone();
        wrong.repetition_index = Some(2);
        assert_eq!(
            validate_table_pdf_records(std::slice::from_ref(&expected), &[wrong]),
            Err(TablePdfClosureError::WrongRepetition)
        );
        let mut wrong = expected.clone();
        wrong.cell_node_id += 1;
        assert_eq!(
            validate_table_pdf_records(std::slice::from_ref(&expected), &[wrong]),
            Err(TablePdfClosureError::WrongCell)
        );
        let mut wrong = expected.clone();
        wrong.rect = TablePaintRect::from_untrusted_parts(11, 20, 30, 40);
        assert_eq!(
            validate_table_pdf_records(std::slice::from_ref(&expected), &[wrong]),
            Err(TablePdfClosureError::WrongRectangle)
        );
        let mut wrong = expected.clone();
        wrong.content_fragment_end += 1;
        assert_eq!(
            validate_table_pdf_records(std::slice::from_ref(&expected), &[wrong]),
            Err(TablePdfClosureError::WrongContentRange)
        );
    }

    #[test]
    fn table_pdf_closure_rejects_command_and_decoration_tampering() {
        assert_eq!(reject_table_pdf_decorations(0, &[]), Ok(()));
        assert_eq!(
            reject_table_pdf_decorations(1, &[]),
            Err(TablePdfClosureError::DecorationForbidden)
        );
        assert_eq!(
            reject_table_pdf_decorations(0, &[TablePdfDecorationObservation::Border]),
            Err(TablePdfClosureError::DecorationForbidden)
        );

        let graph = blank_content_graph();
        let observation = TablePaintCommandObservation {
            page_index: 0,
            page_command_index: 0,
            fragment_id: 1,
            repetition_index: None,
            cell_node_id: NodeId::new(3),
            command: DisplayCommand::DrawImage {
                image_id: ImageResourceId::new(0),
                rect: Rect::new(
                    Length::ZERO,
                    Length::ZERO,
                    positive_points(1),
                    positive_points(1),
                ),
            },
        };
        assert!(is_unclaimed_table_command(
            &observation.command,
            0,
            &[&observation.command],
            &[],
        ));
        let selected_rect = TablePaintRect::from_untrusted_parts(10, 20, 30, 40);
        assert!(table_rect_contains_point(selected_rect, 10, 20));
        assert!(!table_rect_contains_point(selected_rect, 40, 20));
        assert!(table_rects_intersect(
            selected_rect,
            Rect::new(
                Length::from_raw(39).unwrap(),
                Length::from_raw(59).unwrap(),
                PositiveLength::new(Length::from_raw(2).unwrap()).unwrap(),
                PositiveLength::new(Length::from_raw(2).unwrap()).unwrap(),
            ),
        ));
        assert_eq!(
            validate_table_pdf_commands(std::slice::from_ref(&observation), &[], &graph),
            Err(TablePdfClosureError::MissingCommand)
        );
        assert_eq!(
            validate_table_pdf_commands(&[], std::slice::from_ref(&observation), &graph),
            Err(TablePdfClosureError::ExtraCommand)
        );
        let mut wrong = observation.clone();
        wrong.page_index = 1;
        assert_eq!(
            validate_table_pdf_commands(std::slice::from_ref(&observation), &[wrong], &graph,),
            Err(TablePdfClosureError::WrongPage)
        );
    }
    fn positive_points(points: i64) -> PositiveLength {
        PositiveLength::new(Length::from_raw(points * 65_536).unwrap()).unwrap()
    }
    fn freeze_for_serialization(
        builder: UntrustedPdfObjectGraphBuilder,
        root: ObjectId,
    ) -> FrozenPdfGraph {
        let graph = builder.validate_untrusted(root).unwrap();
        let object_count = u32::try_from(graph.objects.len()).unwrap();
        FrozenPdfGraph {
            graph,
            selected_layout_fingerprint: LayoutStateFingerprint::from_untrusted_bytes([3; 32]),
            pages: vec![FrozenPageGeometry {
                page_index: 0,
                master_id: MasterId::new("default").unwrap(),
                width: positive_points(100),
                height: positive_points(100),
            }],
            page_count: 1,
            object_count,
            font_bindings: vec![],
            image_bindings: vec![],
            table_closures: vec![],
            footnote_closure: None,
        }
    }
    fn blank_content_graph() -> FrozenPdfGraph {
        let (mut builder, root) = valid_graph();
        let content_id = ObjectId::new(4).unwrap();
        let page_id = ObjectId::new(3).unwrap();
        let Some(IndirectObjectBody::Value(PdfValue::Dictionary(page))) =
            builder.objects.get_mut(&page_id)
        else {
            panic!("fixture page must be a dictionary");
        };
        page.insert(name(b"Contents"), PdfValue::Reference(content_id));
        builder
            .insert(
                content_id,
                IndirectObjectBody::DisplayPageContent(DisplayPage {
                    page_index: 0,
                    width: positive_points(100),
                    height: positive_points(100),
                    commands: vec![],
                    annotations: vec![],
                }),
            )
            .unwrap();
        freeze_for_serialization(builder, root)
    }

    fn footnote_pdf_receipt_fixture() -> (
        FootnoteDisplayClosureReceipt,
        FrozenPdfGraph,
        VerifiedPdfBytesReceipt,
    ) {
        let closure = FootnoteDisplayClosureReceipt::serializer_pdf_test_fixture();
        let mut graph = blank_content_graph();
        graph.selected_layout_fingerprint =
            LayoutStateFingerprint::from_untrusted_bytes(closure.body_layout_sha256());
        graph.footnote_closure = Some(closure.clone());
        let bytes = b"%PDF-1.7\nfootnote-closure\n".to_vec();
        let receipt = VerifiedPdfBytesReceipt {
            sha256: sha256(&bytes),
            bytes,
            selected_layout_fingerprint: graph.selected_layout_fingerprint,
            footnote_display_sha256: Some(closure.fingerprint()),
            page_count: graph.page_count,
            object_count: graph.object_count,
            stream_compression: PdfStreamCompression::None,
            config_fingerprint: EffectiveConfigFingerprint::from_untrusted_bytes([7; 32]),
        };
        (closure, graph, receipt)
    }

    #[test]
    fn footnote_pdf_closure_binds_markers_separator_definitions_and_exact_bytes() {
        let (closure, graph, receipt) = footnote_pdf_receipt_fixture();
        let bound = FootnotePdfClosureReceipt::from_serialized(&closure, &graph, &receipt).unwrap();
        assert_eq!(bound.display_sha256(), closure.fingerprint());
        assert_eq!(bound.selected_layout_sha256(), [3; 32]);
        assert_eq!(bound.body_layout_sha256(), [4; 32]);
        assert_eq!(bound.pdf_sha256(), sha256(receipt.bytes()));
        assert_eq!(bound.reference_marker_count(), 1);
        assert_eq!(bound.separator_count(), 1);
        assert_eq!(bound.definition_command_count(), 1);
        assert!(bound
            .canonical_jcs()
            .contains("\"reference_marker_count\":1"));
    }

    #[test]
    fn footnote_pdf_closure_rejects_display_page_and_serializer_tampering() {
        let (closure, mut graph, receipt) = footnote_pdf_receipt_fixture();
        graph.footnote_closure = None;
        assert_eq!(
            FootnotePdfClosureReceipt::from_serialized(&closure, &graph, &receipt),
            Err(FootnotePdfClosureError::DisplayStateMismatch)
        );

        let (closure, graph, mut receipt) = footnote_pdf_receipt_fixture();
        receipt.footnote_display_sha256 = None;
        assert_eq!(
            FootnotePdfClosureReceipt::from_serialized(&closure, &graph, &receipt),
            Err(FootnotePdfClosureError::DisplayStateMismatch)
        );

        let (closure, graph, mut receipt) = footnote_pdf_receipt_fixture();
        receipt.page_count += 1;
        assert_eq!(
            FootnotePdfClosureReceipt::from_serialized(&closure, &graph, &receipt),
            Err(FootnotePdfClosureError::PageClosure)
        );

        let (closure, graph, mut receipt) = footnote_pdf_receipt_fixture();
        receipt.sha256 = [9; 32];
        assert_eq!(
            FootnotePdfClosureReceipt::from_serialized(&closure, &graph, &receipt),
            Err(FootnotePdfClosureError::PdfReceiptMismatch)
        );
    }
    fn valid_graph() -> (UntrustedPdfObjectGraphBuilder, ObjectId) {
        let catalog_id = ObjectId::new(1).unwrap();
        let pages_id = ObjectId::new(2).unwrap();
        let page_id = ObjectId::new(3).unwrap();
        let mut catalog = PdfDictionary::new();
        catalog.insert(name(b"Type"), PdfValue::Name(name(b"Catalog")));
        catalog.insert(name(b"Pages"), PdfValue::Reference(pages_id));
        let mut pages = PdfDictionary::new();
        pages.insert(name(b"Type"), PdfValue::Name(name(b"Pages")));
        pages.insert(
            name(b"Kids"),
            PdfValue::Array(vec![PdfValue::Reference(page_id)]),
        );
        pages.insert(name(b"Count"), PdfValue::Integer(1));
        pages.insert(
            name(b"MediaBox"),
            PdfValue::Array(vec![
                PdfValue::Integer(0),
                PdfValue::Integer(0),
                PdfValue::Integer(100),
                PdfValue::Integer(100),
            ]),
        );
        let mut page = PdfDictionary::new();
        page.insert(name(b"Type"), PdfValue::Name(name(b"Page")));
        page.insert(name(b"Parent"), PdfValue::Reference(pages_id));
        let mut builder = builder_with_max(ResourceLimits::default().max_pdf_objects);
        builder
            .insert(
                catalog_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(catalog)),
            )
            .unwrap();
        builder
            .insert(
                pages_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(pages)),
            )
            .unwrap();
        builder
            .insert(
                page_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(page)),
            )
            .unwrap();
        (builder, catalog_id)
    }
    fn builder_with_max(max_pdf_objects: u32) -> UntrustedPdfObjectGraphBuilder {
        let limits = ResourceLimits {
            max_pdf_objects,
            ..ResourceLimits::default()
        };
        UntrustedPdfObjectGraphBuilder::new(&ValidatedResourceLimits::new(limits).unwrap())
    }
    fn nested_value(depth: usize) -> PdfValue {
        assert!(depth > 0);
        let mut value = PdfValue::Null;
        for _ in 1..depth {
            value = PdfValue::Array(vec![value]);
        }
        value
    }
    fn page_tree_chain(pages_nodes: usize) -> (UntrustedPdfObjectGraphBuilder, ObjectId) {
        assert!(pages_nodes > 0);
        let catalog_id = ObjectId::new(1).unwrap();
        let first_pages_id = ObjectId::new(2).unwrap();
        let page_id = ObjectId::new(u32::try_from(pages_nodes + 2).unwrap()).unwrap();
        let mut catalog = PdfDictionary::new();
        catalog.insert(name(b"Type"), PdfValue::Name(name(b"Catalog")));
        catalog.insert(name(b"Pages"), PdfValue::Reference(first_pages_id));
        let mut builder = builder_with_max(u32::try_from(pages_nodes + 2).unwrap());
        builder
            .insert(
                catalog_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(catalog)),
            )
            .unwrap();
        for index in 0..pages_nodes {
            let id = ObjectId::new(u32::try_from(index + 2).unwrap()).unwrap();
            let kid = if index + 1 == pages_nodes {
                page_id
            } else {
                ObjectId::new(u32::try_from(index + 3).unwrap()).unwrap()
            };
            let mut pages = PdfDictionary::new();
            pages.insert(name(b"Type"), PdfValue::Name(name(b"Pages")));
            pages.insert(
                name(b"Kids"),
                PdfValue::Array(vec![PdfValue::Reference(kid)]),
            );
            pages.insert(name(b"Count"), PdfValue::Integer(1));
            if index == 0 {
                pages.insert(
                    name(b"MediaBox"),
                    PdfValue::Array(vec![
                        PdfValue::Integer(0),
                        PdfValue::Integer(0),
                        PdfValue::Integer(100),
                        PdfValue::Integer(100),
                    ]),
                );
            } else {
                pages.insert(
                    name(b"Parent"),
                    PdfValue::Reference(ObjectId::new(u32::try_from(index + 1).unwrap()).unwrap()),
                );
            }
            builder
                .insert(id, IndirectObjectBody::Value(PdfValue::Dictionary(pages)))
                .unwrap();
        }
        let mut page = PdfDictionary::new();
        page.insert(name(b"Type"), PdfValue::Name(name(b"Page")));
        page.insert(
            name(b"Parent"),
            PdfValue::Reference(ObjectId::new(u32::try_from(pages_nodes + 1).unwrap()).unwrap()),
        );
        builder
            .insert(
                page_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(page)),
            )
            .unwrap();
        (builder, catalog_id)
    }
    #[test]
    fn duplicate_insert_preserves_first_object() {
        let (mut builder, root) = valid_graph();
        assert_eq!(
            builder.insert(root, IndirectObjectBody::Value(PdfValue::Integer(2))),
            Err(PdfError::DuplicateObject)
        );
        assert!(builder.validate_untrusted(root).is_ok());
    }
    #[test]
    fn pdf_name_escapes_delimiters_and_space() {
        assert_eq!(
            PdfName::from_bytes(b"A B/C#".to_vec()).unwrap().encoded(),
            b"/A#20B#2FC#23".to_vec()
        );
    }
    #[test]
    fn decimal_is_canonical() {
        assert_eq!(PdfDecimal::new(12_300, 3).unwrap().canonical(), "12.3");
        assert_eq!(PdfDecimal::new(-5, 2).unwrap().canonical(), "-0.05");
    }
    #[test]
    fn allocation_free_numeric_tokens_match_the_public_canonical_form() {
        let mut output = LimitedPdfBuffer::new(1_024);
        output.integer(i64::MIN).unwrap();
        output.push(b' ').unwrap();
        output.unsigned(u64::MAX).unwrap();
        output.push(b' ').unwrap();
        output.zero_padded_unsigned(42, 10).unwrap();
        assert_eq!(
            output.bytes,
            b"-9223372036854775808 18446744073709551615 0000000042"
        );

        for decimal in [
            PdfDecimal::new(1_200, 0).unwrap(),
            PdfDecimal::new(12_300, 3).unwrap(),
            PdfDecimal::new(-5, 2).unwrap(),
            PdfDecimal::new(i64::MIN, 12).unwrap(),
        ] {
            let mut encoded = LimitedPdfBuffer::new(64);
            write_pdf_decimal(&mut encoded, decimal).unwrap();
            assert_eq!(encoded.bytes, decimal.canonical().as_bytes());
        }
    }
    #[test]
    fn serializer_emits_deterministic_classic_xref_and_all_direct_values() {
        let (mut builder, root) = valid_graph();
        let values_id = ObjectId::new(4).unwrap();
        let Some(IndirectObjectBody::Value(PdfValue::Dictionary(catalog))) =
            builder.objects.get_mut(&root)
        else {
            panic!("fixture root must be a catalog dictionary");
        };
        catalog.insert(name(b"Extras"), PdfValue::Reference(values_id));
        let mut nested = PdfDictionary::new();
        nested.insert(name(b"K"), PdfValue::ByteString(b"V".to_vec()));
        builder
            .insert(
                values_id,
                IndirectObjectBody::Value(PdfValue::Array(vec![
                    PdfValue::Null,
                    PdfValue::Bool(true),
                    PdfValue::Bool(false),
                    PdfValue::Integer(-7),
                    PdfValue::Decimal(PdfDecimal::new(12_300, 3).unwrap()),
                    PdfValue::Name(name(b"A B")),
                    PdfValue::ByteString(vec![0, b'(', b')', 0xff]),
                    PdfValue::Array(vec![PdfValue::Reference(root)]),
                    PdfValue::Dictionary(nested),
                ])),
            )
            .unwrap();
        let graph = freeze_for_serialization(builder, root);
        let config = effective_config(PdfStreamCompression::None, ResourceLimits::default());
        let first = PdfBackend::serialize(graph.clone(), &config).unwrap();
        let second = PdfBackend::serialize(graph, &config).unwrap();
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.content_hash(), typaxis_core::sha256(first.bytes()));
        assert_eq!(first.config_fingerprint(), config.fingerprint());
        assert_eq!(first.object_count(), 4);
        assert!(first.bytes().starts_with(b"%PDF-1.7\n"));
        let direct_values = b"[null true false -7 12.3 /A#20B <002829FF> [1 0 R] << /K <56> >>]";
        assert!(first
            .bytes()
            .windows(direct_values.len())
            .any(|window| window == direct_values));

        let bytes = first.bytes();
        let xref = bytes
            .windows(b"xref\n".len())
            .position(|window| window == b"xref\n")
            .unwrap();
        let xref_text = std::str::from_utf8(&bytes[xref..]).unwrap();
        let lines: Vec<_> = xref_text.lines().collect();
        assert_eq!(lines[0], "xref");
        assert_eq!(lines[1], "0 5");
        assert_eq!(lines[2], "0000000000 65535 f ");
        for object in 1..=4u32 {
            let entry = lines[usize::try_from(object).unwrap() + 2];
            assert_eq!(entry.len(), 19);
            assert_eq!(&entry[11..16], "00000");
            assert_eq!(&entry[17..], "n ");
            let offset: usize = entry[..10].parse().unwrap();
            assert!(bytes[offset..].starts_with(format!("{object} 0 obj\n").as_bytes()));
        }
        let startxref = xref_text
            .split("startxref\n")
            .nth(1)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        assert_eq!(startxref, xref);
        assert!(xref_text.ends_with("%%EOF\n"));
    }

    #[test]
    fn blank_page_content_has_one_top_left_root_transform() {
        let graph = blank_content_graph();
        let config = effective_config(PdfStreamCompression::None, ResourceLimits::default());
        let receipt = PdfBackend::serialize(graph, &config).unwrap();
        let expected = b"stream\nq\n1 0 0 -1 0 100 cm\nQ\n\nendstream";
        assert!(receipt
            .bytes()
            .windows(expected.len())
            .any(|window| window == expected));
        assert_eq!(receipt.stream_compression(), PdfStreamCompression::None);
    }

    #[test]
    fn image_xobject_placement_counter_reflects_the_page_root_transform() {
        let mut output = LimitedPdfBuffer::new(1_024);
        write_image_placement(
            &mut output,
            &name(b"Im0"),
            Rect::new(
                Length::from_raw(10 * 65_536).unwrap(),
                Length::from_raw(20 * 65_536).unwrap(),
                positive_points(30),
                positive_points(40),
            ),
        )
        .unwrap();
        assert_eq!(output.bytes, b"q\n30 0 0 -40 10 60 cm\n/Im0 Do\nQ\n");
    }

    #[test]
    fn image_xobject_closure_rejects_missing_extra_wrong_and_duplicate_bindings() {
        let image_0 = ImageResourceId::new(0);
        let image_1 = ImageResourceId::new(1);
        let expected = BTreeSet::from([image_0]);
        let im0 = name(b"Im0");
        let im1 = name(b"Im1");
        let closed =
            close_staging_machine_figure_image_bindings(&expected, [(image_0, &im0)]).unwrap();
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].image_id(), image_0);
        assert_eq!(closed[0].resource_name(), "/Im0");

        for bindings in [
            Vec::new(),
            vec![(image_0, &im0), (image_1, &im1)],
            vec![(image_1, &im1)],
            vec![(image_0, &im0), (image_0, &im1)],
        ] {
            assert_eq!(
                close_staging_machine_figure_image_bindings(&expected, bindings),
                Err(StagingMachineFigurePdfError::ImageXObjectClosure)
            );
        }
    }

    #[test]
    fn image_xobject_serialized_closure_allows_soft_mask_but_requires_logical_image() {
        assert_eq!(
            require_staging_serialized_image_xobjects(b"%PDF", 1),
            Err(StagingMachineFigurePdfError::ImageXObjectClosure)
        );
        assert_eq!(
            require_staging_serialized_image_xobjects(b"/Subtype /Image", 1),
            Ok(())
        );
        assert_eq!(
            require_staging_serialized_image_xobjects(
                b"/Subtype /Image /SMask 4 0 R /Subtype /Image",
                2,
            ),
            Ok(())
        );
        assert_eq!(
            require_staging_serialized_image_xobjects(b"/Subtype /Image", 2),
            Err(StagingMachineFigurePdfError::ImageXObjectClosure)
        );
    }

    #[test]
    fn flate_mode_uses_deterministic_valid_stored_zlib_blocks() {
        assert_eq!(
            zlib_stored(b"hello", 16).unwrap(),
            [
                &[0x78, 0x01, 0x01, 0x05, 0x00, 0xfa, 0xff][..],
                &b"hello"[..],
                &[0x06, 0x2c, 0x02, 0x15][..],
            ]
            .concat()
        );
        let graph = blank_content_graph();
        let config = effective_config(PdfStreamCompression::Flate, ResourceLimits::default());
        let receipt = PdfBackend::serialize(graph, &config).unwrap();
        assert!(receipt
            .bytes()
            .windows(b"/Filter /FlateDecode".len())
            .any(|window| window == b"/Filter /FlateDecode"));
        assert_eq!(receipt.stream_compression(), PdfStreamCompression::Flate);
    }

    #[test]
    fn stream_owned_entries_merge_without_temporary_dictionaries() {
        let mut dictionary = PdfDictionary::new();
        dictionary.insert(name(b"A"), PdfValue::Integer(1));
        dictionary.insert(name(b"Z"), PdfValue::Integer(2));
        let mut output = LimitedPdfBuffer::new(1_024);
        write_stream(&mut output, &dictionary, b"abc", Some(b"FlateDecode")).unwrap();
        assert_eq!(
            output.bytes,
            b"<< /A 1 /Filter /FlateDecode /Length 3 /Z 2 >>\nstream\nabc\nendstream"
        );
    }

    #[test]
    fn receipt_hashing_matches_sha256_without_copying_the_pdf() {
        for length in [0usize, 1, 55, 56, 63, 64, 65, 1_000] {
            let bytes: Vec<_> = (0..length)
                .map(|index| u8::try_from(index % 251).unwrap())
                .collect();
            assert_eq!(pdf_sha256(&bytes), typaxis_core::sha256(&bytes));
            for chunk_size in [1usize, 7, 64, 127] {
                let mut streaming = PdfSha256::new();
                for chunk in bytes.chunks(chunk_size) {
                    streaming.update(chunk);
                }
                assert_eq!(streaming.finish(), typaxis_core::sha256(&bytes));
            }
        }
    }

    #[test]
    fn receipt_streaming_aggregates_short_writes_and_rejects_partial_output() {
        #[derive(Default)]
        struct ShortWriter {
            bytes: Vec<u8>,
            max_write: usize,
            write_calls: usize,
        }
        impl Write for ShortWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.write_calls += 1;
                let written = bytes.len().min(self.max_write);
                self.bytes.extend_from_slice(&bytes[..written]);
                Ok(written)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        struct FailingWriter {
            bytes: Vec<u8>,
            accepted_limit: usize,
        }
        impl Write for FailingWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.bytes.len() == self.accepted_limit {
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"));
                }
                let remaining = self.accepted_limit - self.bytes.len();
                let written = bytes.len().min(remaining);
                self.bytes.extend_from_slice(&bytes[..written]);
                Ok(written)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let config = effective_config(PdfStreamCompression::None, ResourceLimits::default());
        let receipt = PdfBackend::serialize(blank_content_graph(), &config).unwrap();
        let mut short = ShortWriter {
            max_write: 7,
            ..ShortWriter::default()
        };
        let facts = receipt.write_streaming(&mut short).unwrap();
        assert!(short.write_calls > 1);
        assert_eq!(short.bytes, receipt.bytes());
        assert_eq!(facts.byte_length(), receipt.byte_length());
        assert_eq!(facts.content_hash(), receipt.content_hash());
        assert_eq!(
            facts.selected_layout_fingerprint(),
            receipt.selected_layout_fingerprint()
        );
        assert_eq!(facts.page_count(), receipt.page_count());
        assert_eq!(facts.object_count(), receipt.object_count());
        assert_eq!(facts.stream_compression(), receipt.stream_compression());
        assert_eq!(facts.config_fingerprint(), receipt.config_fingerprint());

        let accepted_limit = receipt.bytes().len() / 2;
        let mut failing = FailingWriter {
            bytes: Vec::new(),
            accepted_limit,
        };
        let error = receipt.write_streaming(&mut failing).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(failing.bytes, receipt.bytes()[..accepted_limit]);

        let mut mismatched = PdfBackend::serialize(blank_content_graph(), &config).unwrap();
        mismatched.sha256[0] ^= 0xff;
        let mut sink = Vec::new();
        let error = mismatched.write_streaming(&mut sink).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(sink, mismatched.bytes());
    }

    #[test]
    fn serializer_enforces_output_and_classic_xref_limits_before_writes() {
        let limits = ResourceLimits {
            max_output_bytes: 64,
            ..ResourceLimits::default()
        };
        let config = effective_config(PdfStreamCompression::None, limits);
        assert_eq!(
            PdfBackend::serialize(blank_content_graph(), &config),
            Err(PdfError::OutputTooLarge)
        );
        let object_limited = effective_config(
            PdfStreamCompression::None,
            ResourceLimits {
                max_pdf_objects: 3,
                ..ResourceLimits::default()
            },
        );
        assert_eq!(
            PdfBackend::serialize(blank_content_graph(), &object_limited),
            Err(PdfError::ObjectLimit)
        );

        let mut entry = LimitedPdfBuffer::new(32);
        assert_eq!(
            write_xref_entry(&mut entry, CLASSIC_XREF_MAX_OFFSET),
            Ok(())
        );
        let before = entry.bytes.clone();
        assert_eq!(
            write_xref_entry(&mut entry, CLASSIC_XREF_MAX_OFFSET + 1),
            Err(PdfError::OutputTooLarge)
        );
        assert_eq!(entry.bytes, before);
    }
    #[test]
    fn valid_page_tree_freezes() {
        let (builder, root) = valid_graph();
        assert!(builder.validate_untrusted(root).is_ok());
    }
    #[test]
    fn page_requires_effective_media_box_and_rejects_tree_keys() {
        let (mut missing_box, root) = valid_graph();
        let pages_id = ObjectId::new(2).unwrap();
        if let Some(IndirectObjectBody::Value(PdfValue::Dictionary(pages))) =
            missing_box.objects.get_mut(&pages_id)
        {
            pages.remove(&name(b"MediaBox"));
        }
        assert_eq!(
            missing_box.validate_untrusted(root),
            Err(PdfError::InvalidPageTree)
        );

        let (mut leaf_with_kids, root) = valid_graph();
        let page_id = ObjectId::new(3).unwrap();
        if let Some(IndirectObjectBody::Value(PdfValue::Dictionary(page))) =
            leaf_with_kids.objects.get_mut(&page_id)
        {
            page.insert(name(b"Kids"), PdfValue::Array(vec![]));
        }
        assert_eq!(
            leaf_with_kids.validate_untrusted(root),
            Err(PdfError::InvalidPageTree)
        );
    }
    #[test]
    fn catalog_pages_must_reference_pages_node() {
        let catalog_id = ObjectId::new(1).unwrap();
        let page_id = ObjectId::new(2).unwrap();
        let mut catalog = PdfDictionary::new();
        catalog.insert(name(b"Type"), PdfValue::Name(name(b"Catalog")));
        catalog.insert(name(b"Pages"), PdfValue::Reference(page_id));
        let mut page = PdfDictionary::new();
        page.insert(name(b"Type"), PdfValue::Name(name(b"Page")));
        let mut builder = builder_with_max(ResourceLimits::default().max_pdf_objects);
        builder
            .insert(
                catalog_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(catalog)),
            )
            .unwrap();
        builder
            .insert(
                page_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(page)),
            )
            .unwrap();
        assert_eq!(
            builder.validate_untrusted(catalog_id),
            Err(PdfError::InvalidPageTree)
        );
    }
    #[test]
    fn root_pages_node_must_not_have_parent() {
        let (mut builder, root) = valid_graph();
        let pages_id = ObjectId::new(2).unwrap();
        let parent_id = ObjectId::new(4).unwrap();
        if let Some(IndirectObjectBody::Value(PdfValue::Dictionary(pages))) =
            builder.objects.get_mut(&pages_id)
        {
            pages.insert(name(b"Parent"), PdfValue::Reference(parent_id));
        } else {
            panic!("valid fixture must contain a Pages dictionary");
        }
        builder
            .insert(
                parent_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(PdfDictionary::new())),
            )
            .unwrap();
        assert_eq!(
            builder.validate_untrusted(root),
            Err(PdfError::InvalidPageTree)
        );
    }
    #[test]
    fn serializer_owned_stream_keys_are_rejected() {
        let (mut builder, root) = valid_graph();
        let stream_id = ObjectId::new(4).unwrap();
        let mut dictionary = PdfDictionary::new();
        dictionary.insert(name(b"Filter"), PdfValue::Name(name(b"FlateDecode")));
        builder
            .insert(
                stream_id,
                IndirectObjectBody::Stream(PdfStreamObject {
                    dictionary,
                    encoding: StreamEncoding::Flate,
                    raw_data: vec![],
                }),
            )
            .unwrap();
        assert_eq!(
            builder.validate_untrusted(root),
            Err(PdfError::ReservedStreamKey)
        );
    }

    #[test]
    fn sparse_and_unreachable_objects_are_rejected() {
        let (mut sparse, root) = valid_graph();
        sparse
            .insert(
                ObjectId::new(5).unwrap(),
                IndirectObjectBody::Value(PdfValue::Null),
            )
            .unwrap();
        assert_eq!(
            sparse.validate_untrusted(root),
            Err(PdfError::SparseObjectId)
        );

        let (mut orphan, root) = valid_graph();
        let orphan_id = ObjectId::new(4).unwrap();
        orphan
            .insert(orphan_id, IndirectObjectBody::Value(PdfValue::Null))
            .unwrap();
        assert_eq!(
            orphan.validate_untrusted(root),
            Err(PdfError::UnreachableObject(orphan_id))
        );
    }

    #[test]
    fn object_limit_is_checked_before_insertion() {
        let (exact, root) = valid_graph();
        assert!(exact.validate_untrusted(root).is_ok());

        let (mut limited, _) = valid_graph();
        limited.max_objects = 3;
        assert_eq!(
            limited.insert(
                ObjectId::new(4).unwrap(),
                IndirectObjectBody::Value(PdfValue::Null),
            ),
            Err(PdfError::ObjectLimit)
        );
        assert_eq!(limited.objects.len(), 3);
    }

    #[test]
    fn dense_allocator_accepts_the_last_u32_object_id_and_rejects_max_plus_one() {
        let mut exact = DenseObjectAllocator {
            next: u64::from(u32::MAX),
            required: u32::MAX,
        };
        assert_eq!(exact.allocate().unwrap().get(), u32::MAX);
        assert_eq!(exact.finish(), Ok(()));

        let mut exhausted = DenseObjectAllocator {
            next: u64::from(u32::MAX) + 1,
            required: u32::MAX,
        };
        assert_eq!(exhausted.allocate(), Err(PdfError::ObjectCountOverflow));
    }

    #[test]
    fn serializer_receipt_binds_the_exact_effective_config_fingerprint() {
        let (builder, root) = valid_graph();
        let graph = FrozenPdfGraph {
            graph: builder.validate_untrusted(root).unwrap(),
            selected_layout_fingerprint: LayoutStateFingerprint::from_untrusted_bytes([3; 32]),
            pages: vec![FrozenPageGeometry {
                page_index: 0,
                master_id: MasterId::new("default").unwrap(),
                width: PositiveLength::new(Length::from_raw(1).unwrap()).unwrap(),
                height: PositiveLength::new(Length::from_raw(1).unwrap()).unwrap(),
            }],
            page_count: 1,
            object_count: 3,
            font_bindings: vec![],
            image_bindings: vec![],
            table_closures: vec![],
            footnote_closure: None,
        };
        let expected = EffectiveConfigFingerprint::from_untrusted_bytes([5; 32]);
        let different = EffectiveConfigFingerprint::from_untrusted_bytes([6; 32]);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let receipt = VerifiedPdfSerializerReceiptOwner::new()
            .issue(
                &graph,
                b"%PDF-1.7\n".to_vec(),
                PdfStreamCompression::None,
                expected,
                &limits,
            )
            .unwrap();
        assert_eq!(receipt.config_fingerprint(), expected);
        assert_ne!(receipt.config_fingerprint(), different);
    }

    #[test]
    fn composite_font_blueprint_allocates_six_dense_objects() {
        let blueprint = [
            PdfFontIndirectObjectRole::Type0Font,
            PdfFontIndirectObjectRole::CidFont,
            PdfFontIndirectObjectRole::FontDescriptor,
            PdfFontIndirectObjectRole::EmbeddedFontProgram,
            PdfFontIndirectObjectRole::ToUnicodeCMap,
            PdfFontIndirectObjectRole::CidToGidMap,
        ];
        let mut allocator = DenseObjectAllocator::new(6);
        let ids = FontObjectIds::allocate_blueprint(&blueprint, &mut allocator).unwrap();
        allocator.finish().unwrap();
        assert_eq!(
            [
                ids.type0.get(),
                ids.cid_font.get(),
                ids.descriptor.get(),
                ids.font_program.get(),
                ids.to_unicode.get(),
                ids.cid_to_gid.get(),
            ],
            [1, 2, 3, 4, 5, 6]
        );
    }

    #[test]
    fn pdf_uses_the_postscript_name_bound_to_the_verified_subset_program() {
        let first = subset_base_font_name("AAAAAA+Typaxis").unwrap();
        let second = subset_base_font_name("AAAAAB+Typaxis").unwrap();
        assert_eq!(first.0, b"AAAAAA+Typaxis");
        assert_eq!(second.0, b"AAAAAB+Typaxis");
        assert_ne!(first, second);
    }

    #[test]
    fn annotations_destination_and_internal_target_form_a_closed_graph() {
        let (mut builder, root) = valid_graph();
        let page_id = ObjectId::new(3).unwrap();
        let annotation_id = ObjectId::new(4).unwrap();

        let destination = AnchorId::new("target").unwrap();
        let mut destination_tree = PdfDictionary::new();
        destination_tree.insert(
            name(b"Names"),
            PdfValue::Array(vec![
                PdfValue::ByteString(destination.as_str().as_bytes().to_vec()),
                PdfValue::Array(vec![
                    PdfValue::Reference(page_id),
                    PdfValue::Name(name(b"Fit")),
                ]),
            ]),
        );
        let mut names = PdfDictionary::new();
        names.insert(name(b"Dests"), PdfValue::Dictionary(destination_tree));
        if let Some(IndirectObjectBody::Value(PdfValue::Dictionary(catalog))) =
            builder.objects.get_mut(&root)
        {
            catalog.insert(name(b"Names"), PdfValue::Dictionary(names));
        } else {
            panic!("valid fixture must contain a catalog");
        }
        if let Some(IndirectObjectBody::Value(PdfValue::Dictionary(page))) =
            builder.objects.get_mut(&page_id)
        {
            page.insert(
                name(b"Annots"),
                PdfValue::Array(vec![PdfValue::Reference(annotation_id)]),
            );
        } else {
            panic!("valid fixture must contain a page");
        }
        let mut annotation = PdfDictionary::new();
        annotation.insert(name(b"Type"), PdfValue::Name(name(b"Annot")));
        annotation.insert(name(b"Subtype"), PdfValue::Name(name(b"Link")));
        annotation.insert(
            name(b"Rect"),
            PdfValue::Array(vec![
                PdfValue::Integer(1),
                PdfValue::Integer(2),
                PdfValue::Integer(3),
                PdfValue::Integer(4),
            ]),
        );
        annotation.insert(
            name(b"Border"),
            PdfValue::Array(vec![
                PdfValue::Integer(0),
                PdfValue::Integer(0),
                PdfValue::Integer(0),
            ]),
        );
        annotation.insert(
            name(b"Dest"),
            PdfValue::ByteString(destination.as_str().as_bytes().to_vec()),
        );
        builder
            .insert(
                annotation_id,
                IndirectObjectBody::Value(PdfValue::Dictionary(annotation)),
            )
            .unwrap();
        assert!(builder.clone().validate_untrusted(root).is_ok());

        let mut missing = builder.clone();
        missing.objects.remove(&annotation_id);
        assert_eq!(
            missing.validate_untrusted(root),
            Err(PdfError::MissingReference(annotation_id))
        );

        let mut extra = builder.clone();
        let Some(IndirectObjectBody::Value(PdfValue::Dictionary(page))) =
            extra.objects.get_mut(&page_id)
        else {
            panic!("valid fixture must contain a page");
        };
        page.insert(
            name(b"Annots"),
            PdfValue::Array(vec![
                PdfValue::Reference(annotation_id),
                PdfValue::Reference(annotation_id),
            ]),
        );
        assert_eq!(
            extra.validate_untrusted(root),
            Err(PdfError::InvalidAnnotationClosure)
        );

        let mut wrong_page_reference = builder.clone();
        let Some(IndirectObjectBody::Value(PdfValue::Dictionary(page))) =
            wrong_page_reference.objects.get_mut(&page_id)
        else {
            panic!("valid fixture must contain a page");
        };
        page.insert(
            name(b"Annots"),
            PdfValue::Array(vec![PdfValue::Reference(root)]),
        );
        assert_eq!(
            wrong_page_reference.validate_untrusted(root),
            Err(PdfError::UnreachableObject(annotation_id))
        );

        let mut wrong_target = builder;
        let Some(IndirectObjectBody::Value(PdfValue::Dictionary(annotation))) =
            wrong_target.objects.get_mut(&annotation_id)
        else {
            panic!("valid fixture must contain an annotation");
        };
        annotation.insert(
            name(b"Dest"),
            PdfValue::ByteString(b"wrong-target".to_vec()),
        );
        assert_eq!(
            wrong_target.validate_untrusted(root),
            Err(PdfError::InvalidAnnotationClosure)
        );
    }

    #[test]
    fn annotations_coordinates_are_converted_outside_the_content_ctm() {
        let unit = |points: i64| Length::from_raw(points * 65_536).unwrap();
        let positive = |points: i64| PositiveLength::new(unit(points)).unwrap();
        let converted = annotation_rectangle(
            Rect::new(unit(10), unit(20), positive(30), positive(40)),
            positive(100),
        )
        .unwrap();
        let PdfValue::Array(values) = &converted else {
            panic!("annotation rectangle must be an array");
        };
        let scaled: Vec<_> = values
            .iter()
            .map(|value| pdf_number(value).unwrap())
            .collect();
        assert_eq!(
            scaled,
            [10, 40, 40, 80].map(|value| i128::from(value) * 1_000_000_000_000)
        );
    }

    #[test]
    fn direct_value_depth_64_is_inclusive_and_stream_dictionary_counts_as_root() {
        let mut references = BTreeSet::new();
        assert_eq!(
            collect_references(
                &IndirectObjectBody::Value(nested_value(MAX_PDF_DIRECT_VALUE_DEPTH)),
                &mut references,
            ),
            Ok(())
        );
        assert_eq!(
            collect_references(
                &IndirectObjectBody::Value(nested_value(MAX_PDF_DIRECT_VALUE_DEPTH + 1)),
                &mut references,
            ),
            Err(PdfError::DirectValueDepth)
        );

        let mut exact_dictionary = PdfDictionary::new();
        exact_dictionary.insert(
            name(b"Nested"),
            nested_value(MAX_PDF_DIRECT_VALUE_DEPTH - 1),
        );
        assert_eq!(
            collect_references(
                &IndirectObjectBody::Stream(PdfStreamObject {
                    dictionary: exact_dictionary,
                    encoding: StreamEncoding::None,
                    raw_data: vec![],
                }),
                &mut references,
            ),
            Ok(())
        );
        let mut too_deep_dictionary = PdfDictionary::new();
        too_deep_dictionary.insert(name(b"Nested"), nested_value(MAX_PDF_DIRECT_VALUE_DEPTH));
        assert_eq!(
            collect_references(
                &IndirectObjectBody::Stream(PdfStreamObject {
                    dictionary: too_deep_dictionary,
                    encoding: StreamEncoding::None,
                    raw_data: vec![],
                }),
                &mut references,
            ),
            Err(PdfError::DirectValueDepth)
        );
    }

    #[test]
    fn page_tree_depth_64_is_inclusive_and_max_plus_one_is_rejected_iteratively() {
        // 63 Pages nodes followed by one Page leaf: root Pages depth is 1 and
        // the leaf is exactly depth 64.
        let (exact, root) = page_tree_chain(MAX_PDF_PAGE_TREE_DEPTH - 1);
        assert!(exact.validate_untrusted(root).is_ok());
        let (too_deep, root) = page_tree_chain(MAX_PDF_PAGE_TREE_DEPTH);
        assert_eq!(
            too_deep.validate_untrusted(root),
            Err(PdfError::PageTreeDepth)
        );
    }

    #[test]
    fn destination_name_preallocation_rejects_arithmetic_overflow() {
        assert_eq!(destination_name_value_count(0), Ok(0));
        assert_eq!(
            destination_name_value_count(usize::MAX / 2),
            Ok(usize::MAX - 1)
        );
        assert_eq!(
            destination_name_value_count(usize::MAX / 2 + 1),
            Err(PdfError::ObjectCountOverflow)
        );
    }

    #[test]
    fn machine_block_styles_pdf_observation_preserves_all_typed_selected_facts() {
        let display = StagingMachineBlockStyleDisplay::paragraph_pdf_test_fixture();
        let pdf = StagingMachineBlockStylePdf::from_display(&display);
        assert_eq!(
            pdf.display_sha256(),
            typaxis_core::sha256(display.canonical_jcs().as_bytes())
        );
        assert_eq!(pdf.package_sha256(), display.package_sha256());
        assert_eq!(pdf.start_indent(), 10);
        assert_eq!(pdf.end_indent(), 10);
        assert_eq!(pdf.logical_start_alignment_space(), 30);
        assert_eq!(pdf.logical_end_alignment_space(), 31);
        assert_eq!(pdf.paint_left_inset(), 40);
        assert_eq!(pdf.paint_inline_size(), 20);
        assert_eq!(pdf.effective_space_before(), 0);
        assert_eq!(pdf.effective_space_after(), 6);
        assert!(pdf.page_break_before());
        assert!(pdf.keep_with_next());
        assert_eq!(
            pdf.content_stream_observation(),
            b"q\n40 0 20 1 re W n\nQ\n"
        );

        let display = StagingMachineBlockStyleDisplay::figure_pdf_test_fixture();
        let pdf = StagingMachineBlockStylePdf::from_display(&display);
        assert_eq!(pdf.paint_inline_size(), 30);
        assert!(!pdf.keep_caption());
        assert!(pdf.canonical_jcs().contains("\"keep_caption\":false"));
    }

    #[test]
    fn machine_list_pdf_observation_uses_selected_generated_marker_and_item_flow() {
        let display = StagingMachineListDisplay::list_pdf_test_fixture();
        let pdf = StagingMachineListPdf::from_display(&display);
        assert_eq!(pdf.lists().len(), 1);
        assert_eq!(pdf.items().len(), 1);
        assert_eq!(pdf.items()[0].marker_utf8(), "1.");
        assert_eq!(pdf.items()[0].item_flow_id(), 1);
        assert_eq!(pdf.items()[0].marker_fragment_id(), 0);
        let content = std::str::from_utf8(pdf.content_stream_observation()).unwrap();
        assert!(content.contains("item 2 flow 1 fragment 0 page 0"));
        assert!(content.contains("<312e> Tj"));
        assert!(pdf
            .canonical_jcs()
            .contains("\"generation_kind\":\"list_marker\""));
    }

    #[test]
    fn forced_page_break_pdf_retains_page_count_without_paint_operation() {
        let display = StagingForcedPageBreakDisplay::forced_page_break_pdf_test_fixture();
        assert_eq!(display.paint_operation_count(), 0);
        let pdf = StagingForcedPageBreakPdf::from_display(&display);
        assert_eq!(pdf.page_count(), 2);
        assert_eq!(pdf.pages().len(), 2);
        assert!(pdf
            .pages()
            .iter()
            .all(StagingForcedPageBreakPdfPage::is_blank));
        assert_eq!(pdf.breaks().len(), 1);
        assert_eq!(pdf.breaks()[0].produced_page_index(), 1);
        assert_eq!(pdf.page_tree_observation(), b"/Count 2\n");
        assert!(pdf.canonical_jcs().contains("\"produced_page_index\":1"));
    }
}
