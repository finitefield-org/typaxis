use std::collections::BTreeMap;
use typaxis_core::{
    push_jcs_string, sha256, Length, MasterId, NodeId, NonNegativeLength, Rect,
    ValidatedResourceLimits,
};
use typaxis_document::Block;
use typaxis_layout::{
    FlowId, StagingHeaderFooterLayout, StagingPageRegionKind, StagingPageRegionLayout,
};
use typaxis_style::{PageMaster, PageSelectionContext};
use typaxis_syntax::ValidatedStagingAdvancedPackage;

pub const ADVANCED_SELECTED_LAYOUT_ALGORITHM: &str =
    "typaxis.advanced-pagination-selected-layout/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingHeaderFooterBodyPage {
    page_index: u32,
    before_position: u32,
    after_position: u32,
    terminal: bool,
}

impl StagingHeaderFooterBodyPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn before_position(&self) -> u32 {
        self.before_position
    }
    pub const fn after_position(&self) -> u32 {
        self.after_position
    }
    pub const fn terminal(&self) -> bool {
        self.terminal
    }
}

/// Canonical body-page boundaries for the private runner.  Only authored
/// top-level page breaks split this lightweight staging body pass; the caller
/// supplies neither page indexes nor master IDs.
pub fn derive_staging_header_footer_body_pages(
    package: &ValidatedStagingAdvancedPackage,
    limits: &ValidatedResourceLimits,
) -> Result<Vec<StagingHeaderFooterBodyPage>, StagingHeaderFooterPaginationError> {
    let blocks = &package.package().package().document.blocks;
    let terminal = u32::try_from(blocks.len())
        .map_err(|_| StagingHeaderFooterPaginationError::ArithmeticOverflow)?;
    let mut pages = Vec::new();
    let mut start = 0u32;
    for (index, block) in blocks.iter().enumerate() {
        if matches!(block, Block::PageBreak { .. }) {
            let after = u32::try_from(index)
                .map_err(|_| StagingHeaderFooterPaginationError::ArithmeticOverflow)?
                .checked_add(1)
                .ok_or(StagingHeaderFooterPaginationError::ArithmeticOverflow)?;
            let page_index = u32::try_from(pages.len())
                .map_err(|_| StagingHeaderFooterPaginationError::ArithmeticOverflow)?;
            push_body_page(
                &mut pages,
                StagingHeaderFooterBodyPage {
                    page_index,
                    before_position: start,
                    after_position: after,
                    terminal: false,
                },
                limits,
            )?;
            start = after;
        }
    }
    // A trailing page break intentionally opens one final blank page. An
    // empty document follows the same existing one-page policy.
    let page_index = u32::try_from(pages.len())
        .map_err(|_| StagingHeaderFooterPaginationError::ArithmeticOverflow)?;
    push_body_page(
        &mut pages,
        StagingHeaderFooterBodyPage {
            page_index,
            before_position: start,
            after_position: terminal,
            terminal: true,
        },
        limits,
    )?;
    Ok(pages)
}

fn push_body_page(
    pages: &mut Vec<StagingHeaderFooterBodyPage>,
    page: StagingHeaderFooterBodyPage,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingHeaderFooterPaginationError> {
    if pages.len() >= limits.get().max_pages as usize {
        return Err(StagingHeaderFooterPaginationError::PageLimit);
    }
    pages
        .try_reserve(1)
        .map_err(|_| StagingHeaderFooterPaginationError::AllocationFailure)?;
    pages.push(page);
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingPdfPageBox {
    x_min: i64,
    y_min: i64,
    x_max: i64,
    y_max: i64,
}

impl StagingPdfPageBox {
    pub(crate) const fn new(x_min: i64, y_min: i64, x_max: i64, y_max: i64) -> Self {
        Self {
            x_min,
            y_min,
            x_max,
            y_max,
        }
    }

    pub const fn values(self) -> [i64; 4] {
        [self.x_min, self.y_min, self.x_max, self.y_max]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingSelectedPageBoxes {
    media_box: StagingPdfPageBox,
    crop_box: StagingPdfPageBox,
    trim_box: StagingPdfPageBox,
}

impl StagingSelectedPageBoxes {
    pub(crate) const fn new(
        media_box: StagingPdfPageBox,
        crop_box: StagingPdfPageBox,
        trim_box: StagingPdfPageBox,
    ) -> Self {
        Self {
            media_box,
            crop_box,
            trim_box,
        }
    }

    pub const fn media_box(self) -> StagingPdfPageBox {
        self.media_box
    }
    pub const fn crop_box(self) -> StagingPdfPageBox {
        self.crop_box
    }
    pub const fn trim_box(self) -> StagingPdfPageBox {
        self.trim_box
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingPageMargins {
    top: NonNegativeLength,
    right: NonNegativeLength,
    bottom: NonNegativeLength,
    left: NonNegativeLength,
}

impl StagingPageMargins {
    pub(crate) const fn new(
        top: NonNegativeLength,
        right: NonNegativeLength,
        bottom: NonNegativeLength,
        left: NonNegativeLength,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn top(self) -> NonNegativeLength {
        self.top
    }
    pub const fn right(self) -> NonNegativeLength {
        self.right
    }
    pub const fn bottom(self) -> NonNegativeLength {
        self.bottom
    }
    pub const fn left(self) -> NonNegativeLength {
        self.left
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingAdvancedPageFrameKind {
    Header,
    Body,
    Footer,
}

impl StagingAdvancedPageFrameKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Body => "body",
            Self::Footer => "footer",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingAdvancedFlowPosition {
    flow_id: FlowId,
    ordinal: u32,
}

impl StagingAdvancedFlowPosition {
    pub(crate) const fn new(flow_id: FlowId, ordinal: u32) -> Self {
        Self { flow_id, ordinal }
    }

    pub const fn flow_id(self) -> FlowId {
        self.flow_id
    }
    pub const fn ordinal(self) -> u32 {
        self.ordinal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSelectedAdvancedFrame {
    kind: StagingAdvancedPageFrameKind,
    column_index: Option<u32>,
    frame_flow_id: FlowId,
    source_flow_id: FlowId,
    rect: Rect,
    before_position: StagingAdvancedFlowPosition,
    after_position: StagingAdvancedFlowPosition,
    terminal: bool,
    repetition_index: Option<u32>,
}

impl StagingSelectedAdvancedFrame {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
        kind: StagingAdvancedPageFrameKind,
        column_index: Option<u32>,
        frame_flow_id: FlowId,
        source_flow_id: FlowId,
        rect: Rect,
        before_position: StagingAdvancedFlowPosition,
        after_position: StagingAdvancedFlowPosition,
        terminal: bool,
        repetition_index: Option<u32>,
    ) -> Self {
        Self {
            kind,
            column_index,
            frame_flow_id,
            source_flow_id,
            rect,
            before_position,
            after_position,
            terminal,
            repetition_index,
        }
    }

    pub const fn kind(&self) -> StagingAdvancedPageFrameKind {
        self.kind
    }
    pub const fn column_index(&self) -> Option<u32> {
        self.column_index
    }
    pub const fn frame_flow_id(&self) -> FlowId {
        self.frame_flow_id
    }
    pub const fn source_flow_id(&self) -> FlowId {
        self.source_flow_id
    }
    pub const fn rect(&self) -> Rect {
        self.rect
    }
    pub const fn before_position(&self) -> StagingAdvancedFlowPosition {
        self.before_position
    }
    pub const fn after_position(&self) -> StagingAdvancedFlowPosition {
        self.after_position
    }
    pub const fn terminal(&self) -> bool {
        self.terminal
    }
    pub const fn repetition_index(&self) -> Option<u32> {
        self.repetition_index
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingRepeatedRegionFragment {
    page_index: u32,
    master_id: MasterId,
    kind: StagingPageRegionKind,
    source_flow_id: FlowId,
    source_node_id: NodeId,
    block_node_id: NodeId,
    repetition_index: u32,
    before_position: u32,
    after_position: u32,
    bounds: Rect,
}

impl StagingRepeatedRegionFragment {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn kind(&self) -> StagingPageRegionKind {
        self.kind
    }
    pub const fn source_flow_id(&self) -> FlowId {
        self.source_flow_id
    }
    pub const fn source_node_id(&self) -> NodeId {
        self.source_node_id
    }
    pub const fn block_node_id(&self) -> NodeId {
        self.block_node_id
    }
    pub const fn repetition_index(&self) -> u32 {
        self.repetition_index
    }
    pub const fn before_position(&self) -> u32 {
        self.before_position
    }
    pub const fn after_position(&self) -> u32 {
        self.after_position
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSelectedAdvancedPage {
    page_index: u32,
    master_id: MasterId,
    boxes: StagingSelectedPageBoxes,
    margins: StagingPageMargins,
    frames: Vec<StagingSelectedAdvancedFrame>,
    region_fragments: Vec<StagingRepeatedRegionFragment>,
}

impl StagingSelectedAdvancedPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn boxes(&self) -> StagingSelectedPageBoxes {
        self.boxes
    }
    pub const fn margins(&self) -> StagingPageMargins {
        self.margins
    }
    pub fn frames(&self) -> &[StagingSelectedAdvancedFrame] {
        &self.frames
    }
    pub fn region_fragments(&self) -> &[StagingRepeatedRegionFragment] {
        &self.region_fragments
    }
}

#[derive(Debug)]
pub struct StagingHeaderFooterSelectedLayoutReceipt {
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    canonical_jcs: String,
}

impl StagingHeaderFooterSelectedLayoutReceipt {
    pub const fn profile_receipt_sha256(&self) -> [u8; 32] {
        self.profile_receipt_sha256
    }
    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Debug)]
pub struct StagingHeaderFooterSelectedLayout {
    pages: Vec<StagingSelectedAdvancedPage>,
    receipt: StagingHeaderFooterSelectedLayoutReceipt,
}

impl StagingHeaderFooterSelectedLayout {
    pub fn pages(&self) -> &[StagingSelectedAdvancedPage] {
        &self.pages
    }
    pub const fn receipt(&self) -> &StagingHeaderFooterSelectedLayoutReceipt {
        &self.receipt
    }

    pub fn verify_receipt(&self) -> Result<(), StagingHeaderFooterPaginationError> {
        let canonical = encode_selected_layout(
            self.receipt.profile_receipt_sha256,
            self.receipt.flow_registry_sha256,
            &self.pages,
        );
        if self.pages.is_empty()
            || canonical != self.receipt.canonical_jcs
            || sha256(canonical.as_bytes()) != self.receipt.selected_layout_sha256
        {
            return Err(StagingHeaderFooterPaginationError::SelectedReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingHeaderFooterPaginationError {
    EmptyBodyPages,
    NonCanonicalBodyPage,
    BodyCursorMismatch,
    PageLimit,
    FragmentLimit,
    PageNumberOverflow,
    MasterSelection,
    MasterReceiptMismatch,
    SelectedReceiptMismatch,
    Geometry,
    RegionOversize {
        master_id: MasterId,
        kind: StagingPageRegionKind,
    },
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for StagingHeaderFooterPaginationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyBodyPages => formatter.write_str("I9190: body page set is empty"),
            Self::NonCanonicalBodyPage => formatter.write_str("I9190: non-canonical body page"),
            Self::BodyCursorMismatch => formatter.write_str("I9190: body cursor mismatch"),
            Self::PageLimit => formatter.write_str("L5110: page limit exceeded"),
            Self::FragmentLimit => formatter.write_str("L5110: selected fragment limit exceeded"),
            Self::PageNumberOverflow => formatter.write_str("L5101: physical page number overflow"),
            Self::MasterSelection => formatter.write_str("L5101: page-master selection failed"),
            Self::MasterReceiptMismatch => formatter.write_str("I9190: selected master mismatch"),
            Self::SelectedReceiptMismatch => {
                formatter.write_str("I9190: selected layout receipt mismatch")
            }
            Self::Geometry => formatter.write_str("L5101: selected page geometry is invalid"),
            Self::RegionOversize { master_id, kind } => write!(
                formatter,
                "L5100: {} content does not fit master {master_id}",
                kind.as_str()
            ),
            Self::ArithmeticOverflow => {
                formatter.write_str("L5101: pagination arithmetic overflow")
            }
            Self::AllocationFailure => formatter.write_str("L5110: pagination allocation failure"),
        }
    }
}

impl std::error::Error for StagingHeaderFooterPaginationError {}

pub fn paginate_staging_header_footer(
    layout: &StagingHeaderFooterLayout,
    body_pages: &[StagingHeaderFooterBodyPage],
    limits: &ValidatedResourceLimits,
) -> Result<StagingHeaderFooterSelectedLayout, StagingHeaderFooterPaginationError> {
    validate_body_pages(layout, body_pages)?;
    if body_pages.is_empty() {
        return Err(StagingHeaderFooterPaginationError::EmptyBodyPages);
    }
    if body_pages.len() > limits.get().max_pages as usize {
        return Err(StagingHeaderFooterPaginationError::PageLimit);
    }
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(body_pages.len())
        .map_err(|_| StagingHeaderFooterPaginationError::AllocationFailure)?;
    let mut repetitions = BTreeMap::<(MasterId, StagingPageRegionKind), u32>::new();
    let mut selected_records = 0u64;
    for body_page in body_pages {
        let context = PageSelectionContext::new(body_page.page_index, None)
            .map_err(|_| StagingHeaderFooterPaginationError::PageNumberOverflow)?;
        let master = layout
            .page_masters()
            .select(&context)
            .map_err(|_| StagingHeaderFooterPaginationError::MasterSelection)?;
        let advanced = layout
            .advanced_page_masters()
            .master(&master.master_id)
            .ok_or(StagingHeaderFooterPaginationError::MasterReceiptMismatch)?;
        let (boxes, margins) = derive_boxes_and_margins(master, advanced.trim)?;
        let header_region = selected_region(
            layout,
            &master.master_id,
            StagingPageRegionKind::Header,
            master.header,
        )?;
        let footer_region = selected_region(
            layout,
            &master.master_id,
            StagingPageRegionKind::Footer,
            master.footer,
        )?;
        let frame_count = 1usize
            .checked_add(usize::from(header_region.is_some()))
            .and_then(|value| value.checked_add(usize::from(footer_region.is_some())))
            .ok_or(StagingHeaderFooterPaginationError::ArithmeticOverflow)?;
        let fragment_count = header_region
            .map_or(0, |region| region.blocks().len())
            .checked_add(footer_region.map_or(0, |region| region.blocks().len()))
            .ok_or(StagingHeaderFooterPaginationError::ArithmeticOverflow)?;
        let page_records = u64::try_from(frame_count)
            .ok()
            .and_then(|frames| {
                let repetitions = u64::from(header_region.is_some())
                    .checked_add(u64::from(footer_region.is_some()))?;
                frames.checked_add(repetitions)
            })
            .and_then(|value| value.checked_add(u64::try_from(fragment_count).ok()?))
            .ok_or(StagingHeaderFooterPaginationError::ArithmeticOverflow)?;

        // Commit the complete page's selected-record budget before issuing a
        // repetition index or allocating any selected frame/fragment state.
        charge(&mut selected_records, page_records, limits)?;
        let mut frames = Vec::new();
        frames
            .try_reserve_exact(frame_count)
            .map_err(|_| StagingHeaderFooterPaginationError::AllocationFailure)?;
        let mut fragments = Vec::new();
        fragments
            .try_reserve_exact(fragment_count)
            .map_err(|_| StagingHeaderFooterPaginationError::AllocationFailure)?;

        if let Some(rect) = master.header {
            let region =
                header_region.ok_or(StagingHeaderFooterPaginationError::MasterReceiptMismatch)?;
            let repetition = next_repetition(
                &mut repetitions,
                &master.master_id,
                StagingPageRegionKind::Header,
            )?;
            frames.push(region_frame(
                StagingAdvancedPageFrameKind::Header,
                rect,
                region,
                repetition,
            )?);
            append_region_fragments(
                &mut fragments,
                body_page.page_index,
                rect,
                region,
                repetition,
            )?;
        }

        frames.push(StagingSelectedAdvancedFrame {
            kind: StagingAdvancedPageFrameKind::Body,
            column_index: Some(0),
            frame_flow_id: FlowId::DOCUMENT_BODY,
            source_flow_id: FlowId::DOCUMENT_BODY,
            rect: master.body,
            before_position: StagingAdvancedFlowPosition {
                flow_id: FlowId::DOCUMENT_BODY,
                ordinal: body_page.before_position,
            },
            after_position: StagingAdvancedFlowPosition {
                flow_id: FlowId::DOCUMENT_BODY,
                ordinal: body_page.after_position,
            },
            terminal: body_page.terminal,
            repetition_index: None,
        });

        if let Some(rect) = master.footer {
            let region =
                footer_region.ok_or(StagingHeaderFooterPaginationError::MasterReceiptMismatch)?;
            let repetition = next_repetition(
                &mut repetitions,
                &master.master_id,
                StagingPageRegionKind::Footer,
            )?;
            frames.push(region_frame(
                StagingAdvancedPageFrameKind::Footer,
                rect,
                region,
                repetition,
            )?);
            append_region_fragments(
                &mut fragments,
                body_page.page_index,
                rect,
                region,
                repetition,
            )?;
        }
        pages.push(StagingSelectedAdvancedPage {
            page_index: body_page.page_index,
            master_id: master.master_id.clone(),
            boxes,
            margins,
            frames,
            region_fragments: fragments,
        });
    }
    let canonical_jcs = encode_selected_layout(
        layout.receipt().profile_receipt_sha256(),
        layout.receipt().fingerprint(),
        &pages,
    );
    let receipt = StagingHeaderFooterSelectedLayoutReceipt {
        profile_receipt_sha256: layout.receipt().profile_receipt_sha256(),
        flow_registry_sha256: layout.receipt().fingerprint(),
        selected_layout_sha256: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    Ok(StagingHeaderFooterSelectedLayout { pages, receipt })
}

fn validate_body_pages(
    layout: &StagingHeaderFooterLayout,
    pages: &[StagingHeaderFooterBodyPage],
) -> Result<(), StagingHeaderFooterPaginationError> {
    if pages.is_empty() {
        return Err(StagingHeaderFooterPaginationError::EmptyBodyPages);
    }
    let terminal = layout.receipt().body_terminal();
    let mut cursor = 0u32;
    for (index, page) in pages.iter().enumerate() {
        let expected_index = u32::try_from(index)
            .map_err(|_| StagingHeaderFooterPaginationError::ArithmeticOverflow)?;
        let is_last = index + 1 == pages.len();
        if page.page_index != expected_index
            || page.before_position != cursor
            || page.after_position < page.before_position
            || page.after_position > terminal
            || (!is_last && page.after_position == page.before_position)
            || page.terminal != is_last
            || (is_last && page.after_position != terminal)
        {
            return Err(StagingHeaderFooterPaginationError::NonCanonicalBodyPage);
        }
        cursor = page.after_position;
    }
    if cursor != terminal {
        return Err(StagingHeaderFooterPaginationError::BodyCursorMismatch);
    }
    Ok(())
}

fn selected_region<'a>(
    layout: &'a StagingHeaderFooterLayout,
    master_id: &MasterId,
    kind: StagingPageRegionKind,
    rect: Option<Rect>,
) -> Result<Option<&'a StagingPageRegionLayout>, StagingHeaderFooterPaginationError> {
    let region = layout.region(master_id, kind);
    match (rect, region) {
        (None, None) => Ok(None),
        (Some(frame), Some(region)) => {
            validate_region_fit(frame, region)?;
            Ok(Some(region))
        }
        (None, Some(_)) | (Some(_), None) => {
            Err(StagingHeaderFooterPaginationError::MasterReceiptMismatch)
        }
    }
}

fn validate_region_fit(
    frame: Rect,
    region: &StagingPageRegionLayout,
) -> Result<(), StagingHeaderFooterPaginationError> {
    if region.total_extent().get().raw() > frame.height().get().raw() {
        return Err(StagingHeaderFooterPaginationError::RegionOversize {
            master_id: region.master_id().clone(),
            kind: region.kind(),
        });
    }
    Ok(())
}

fn derive_boxes_and_margins(
    master: &PageMaster,
    trim: Rect,
) -> Result<(StagingSelectedPageBoxes, StagingPageMargins), StagingHeaderFooterPaginationError> {
    let width = master.width.get().raw();
    let height = master.height.get().raw();
    let trim_right = trim
        .x()
        .raw()
        .checked_add(trim.width().get().raw())
        .ok_or(StagingHeaderFooterPaginationError::Geometry)?;
    let trim_bottom = trim
        .y()
        .raw()
        .checked_add(trim.height().get().raw())
        .ok_or(StagingHeaderFooterPaginationError::Geometry)?;
    let body_right = master
        .body
        .x()
        .raw()
        .checked_add(master.body.width().get().raw())
        .ok_or(StagingHeaderFooterPaginationError::Geometry)?;
    let body_bottom = master
        .body
        .y()
        .raw()
        .checked_add(master.body.height().get().raw())
        .ok_or(StagingHeaderFooterPaginationError::Geometry)?;
    if trim.x().raw() < 0
        || trim.y().raw() < 0
        || trim_right > width
        || trim_bottom > height
        || master.body.x().raw() < trim.x().raw()
        || master.body.y().raw() < trim.y().raw()
        || body_right > trim_right
        || body_bottom > trim_bottom
    {
        return Err(StagingHeaderFooterPaginationError::Geometry);
    }
    let margin = |raw| {
        Length::from_raw(raw)
            .and_then(NonNegativeLength::new)
            .ok_or(StagingHeaderFooterPaginationError::Geometry)
    };
    let margins = StagingPageMargins {
        top: margin(master.body.y().raw() - trim.y().raw())?,
        right: margin(trim_right - body_right)?,
        bottom: margin(trim_bottom - body_bottom)?,
        left: margin(master.body.x().raw() - trim.x().raw())?,
    };
    let media = StagingPdfPageBox {
        x_min: 0,
        y_min: 0,
        x_max: width,
        y_max: height,
    };
    let trim_box = StagingPdfPageBox {
        x_min: trim.x().raw(),
        y_min: height
            .checked_sub(trim_bottom)
            .ok_or(StagingHeaderFooterPaginationError::Geometry)?,
        x_max: trim_right,
        y_max: height
            .checked_sub(trim.y().raw())
            .ok_or(StagingHeaderFooterPaginationError::Geometry)?,
    };
    Ok((
        StagingSelectedPageBoxes {
            media_box: media,
            crop_box: media,
            trim_box,
        },
        margins,
    ))
}

fn next_repetition(
    repetitions: &mut BTreeMap<(MasterId, StagingPageRegionKind), u32>,
    master_id: &MasterId,
    kind: StagingPageRegionKind,
) -> Result<u32, StagingHeaderFooterPaginationError> {
    let value = repetitions.entry((master_id.clone(), kind)).or_insert(0);
    let repetition = *value;
    *value = value
        .checked_add(1)
        .ok_or(StagingHeaderFooterPaginationError::ArithmeticOverflow)?;
    Ok(repetition)
}

fn region_frame(
    kind: StagingAdvancedPageFrameKind,
    rect: Rect,
    region: &StagingPageRegionLayout,
    repetition_index: u32,
) -> Result<StagingSelectedAdvancedFrame, StagingHeaderFooterPaginationError> {
    validate_region_fit(rect, region)?;
    Ok(StagingSelectedAdvancedFrame {
        kind,
        column_index: None,
        frame_flow_id: region.flow_id(),
        source_flow_id: region.flow_id(),
        rect,
        before_position: StagingAdvancedFlowPosition {
            flow_id: region.flow_id(),
            ordinal: 0,
        },
        after_position: StagingAdvancedFlowPosition {
            flow_id: region.flow_id(),
            ordinal: region.terminal(),
        },
        terminal: true,
        repetition_index: Some(repetition_index),
    })
}

#[allow(clippy::too_many_arguments)]
fn append_region_fragments(
    fragments: &mut Vec<StagingRepeatedRegionFragment>,
    page_index: u32,
    frame: Rect,
    region: &StagingPageRegionLayout,
    repetition_index: u32,
) -> Result<(), StagingHeaderFooterPaginationError> {
    for block in region.blocks() {
        let y = frame
            .y()
            .checked_add(block.y_offset().get())
            .ok_or(StagingHeaderFooterPaginationError::Geometry)?;
        let bounds = Rect::new(frame.x(), y, frame.width(), block.block_extent());
        let bottom = y
            .raw()
            .checked_add(block.block_extent().get().raw())
            .ok_or(StagingHeaderFooterPaginationError::Geometry)?;
        let frame_bottom = frame
            .y()
            .raw()
            .checked_add(frame.height().get().raw())
            .ok_or(StagingHeaderFooterPaginationError::Geometry)?;
        if bottom > frame_bottom {
            return Err(StagingHeaderFooterPaginationError::RegionOversize {
                master_id: region.master_id().clone(),
                kind: region.kind(),
            });
        }
        fragments.push(StagingRepeatedRegionFragment {
            page_index,
            master_id: region.master_id().clone(),
            kind: region.kind(),
            source_flow_id: region.flow_id(),
            source_node_id: region.source_node_id(),
            block_node_id: block.node_id(),
            repetition_index,
            before_position: block.before_position(),
            after_position: block.after_position(),
            bounds,
        });
    }
    Ok(())
}

fn charge(
    count: &mut u64,
    amount: u64,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingHeaderFooterPaginationError> {
    let next = count
        .checked_add(amount)
        .ok_or(StagingHeaderFooterPaginationError::FragmentLimit)?;
    if next > limits.get().max_fragments {
        return Err(StagingHeaderFooterPaginationError::FragmentLimit);
    }
    *count = next;
    Ok(())
}

fn encode_selected_layout(
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    pages: &[StagingSelectedAdvancedPage],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ADVANCED_SELECTED_LAYOUT_ALGORITHM);
    output.push_str(",\"flow_registry_sha256\":");
    push_hex(&mut output, flow_registry_sha256);
    output.push_str(",\"pages\":[");
    for (index, page) in pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_page(&mut output, page);
    }
    output.push_str("],\"profile_receipt_sha256\":");
    push_hex(&mut output, profile_receipt_sha256);
    output.push('}');
    output
}

fn encode_page(output: &mut String, page: &StagingSelectedAdvancedPage) {
    output.push_str("{\"boxes\":{\"crop\":");
    push_box(output, page.boxes.crop_box);
    output.push_str(",\"media\":");
    push_box(output, page.boxes.media_box);
    output.push_str(",\"trim\":");
    push_box(output, page.boxes.trim_box);
    output.push_str("},\"frames\":[");
    for (index, frame) in page.frames.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"after_position\":");
        push_position(output, frame.after_position);
        output.push_str(",\"before_position\":");
        push_position(output, frame.before_position);
        output.push_str(",\"column_index\":");
        match frame.column_index {
            Some(value) => output.push_str(&value.to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"frame_flow_id\":");
        output.push_str(&frame.frame_flow_id.get().to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(output, frame.kind.as_str());
        output.push_str(",\"rect\":");
        push_rect(output, frame.rect);
        output.push_str(",\"repetition_index\":");
        match frame.repetition_index {
            Some(value) => output.push_str(&value.to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"source_flow_id\":");
        output.push_str(&frame.source_flow_id.get().to_string());
        output.push_str(",\"terminal\":");
        output.push_str(if frame.terminal { "true" } else { "false" });
        output.push('}');
    }
    output.push_str("],\"margins\":{\"bottom\":");
    output.push_str(&page.margins.bottom.get().raw().to_string());
    output.push_str(",\"left\":");
    output.push_str(&page.margins.left.get().raw().to_string());
    output.push_str(",\"right\":");
    output.push_str(&page.margins.right.get().raw().to_string());
    output.push_str(",\"top\":");
    output.push_str(&page.margins.top.get().raw().to_string());
    output.push_str("},\"master_id\":");
    push_jcs_string(output, page.master_id.as_str());
    output.push_str(",\"page_index\":");
    output.push_str(&page.page_index.to_string());
    output.push_str(",\"region_fragments\":[");
    for (index, fragment) in page.region_fragments.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"after_position\":");
        output.push_str(&fragment.after_position.to_string());
        output.push_str(",\"before_position\":");
        output.push_str(&fragment.before_position.to_string());
        output.push_str(",\"block_node_id\":");
        output.push_str(&fragment.block_node_id.get().to_string());
        output.push_str(",\"bounds\":");
        push_rect(output, fragment.bounds);
        output.push_str(",\"kind\":");
        push_jcs_string(output, fragment.kind.as_str());
        output.push_str(",\"repetition_index\":");
        output.push_str(&fragment.repetition_index.to_string());
        output.push_str(",\"source_flow_id\":");
        output.push_str(&fragment.source_flow_id.get().to_string());
        output.push_str(",\"source_node_id\":");
        output.push_str(&fragment.source_node_id.get().to_string());
        output.push('}');
    }
    output.push_str("]}");
}

fn push_position(output: &mut String, position: StagingAdvancedFlowPosition) {
    output.push_str("{\"flow_id\":");
    output.push_str(&position.flow_id.get().to_string());
    output.push_str(",\"ordinal\":");
    output.push_str(&position.ordinal.to_string());
    output.push('}');
}

fn push_box(output: &mut String, value: StagingPdfPageBox) {
    let values = value.values();
    output.push('[');
    for (index, value) in values.into_iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&value.to_string());
    }
    output.push(']');
}

fn push_rect(output: &mut String, value: Rect) {
    output.push_str("{\"height\":");
    output.push_str(&value.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&value.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&value.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&value.y().raw().to_string());
    output.push('}');
}

fn push_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_layout::staging_header_footer_page_master_fixture;

    fn fixture() -> StagingHeaderFooterLayout {
        staging_header_footer_page_master_fixture()
    }

    #[test]
    fn page_master_selection_boxes_repetitions_and_subflow_progress_are_canonical() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let layout = fixture();
        let body_pages = vec![
            StagingHeaderFooterBodyPage {
                page_index: 0,
                before_position: 0,
                after_position: 1,
                terminal: false,
            },
            StagingHeaderFooterBodyPage {
                page_index: 1,
                before_position: 1,
                after_position: 2,
                terminal: false,
            },
            StagingHeaderFooterBodyPage {
                page_index: 2,
                before_position: 2,
                after_position: 3,
                terminal: false,
            },
            StagingHeaderFooterBodyPage {
                page_index: 3,
                before_position: 3,
                after_position: 4,
                terminal: false,
            },
            StagingHeaderFooterBodyPage {
                page_index: 4,
                before_position: 4,
                after_position: 7,
                terminal: true,
            },
        ];
        let selected = paginate_staging_header_footer(&layout, &body_pages, &limits).unwrap();
        assert_eq!(
            selected
                .pages()
                .iter()
                .map(|page| page.master_id().as_str())
                .collect::<Vec<_>>(),
            ["first", "left", "right", "left", "right"]
        );
        for (page_index, page) in selected.pages().iter().enumerate() {
            assert_eq!(page.page_index(), page_index as u32);
            assert_eq!(
                page.boxes().media_box().values(),
                [0, 0, 40_000_000, 50_000_000]
            );
            assert_eq!(
                page.boxes().trim_box().values(),
                [1_000_000, 1_000_000, 39_000_000, 49_000_000]
            );
            assert_eq!(
                page.frames()
                    .iter()
                    .map(StagingSelectedAdvancedFrame::kind)
                    .collect::<Vec<_>>(),
                [
                    StagingAdvancedPageFrameKind::Header,
                    StagingAdvancedPageFrameKind::Body,
                    StagingAdvancedPageFrameKind::Footer,
                ]
            );
            let body = &page.frames()[1];
            assert_eq!(
                body.before_position().ordinal(),
                body_pages[page_index].before_position()
            );
            assert_eq!(
                body.after_position().ordinal(),
                body_pages[page_index].after_position()
            );
            for region in [&page.frames()[0], &page.frames()[2]] {
                assert_eq!(region.before_position().ordinal(), 0);
                assert!(region.terminal());
                assert_ne!(region.source_flow_id(), FlowId::DOCUMENT_BODY);
            }
        }
        assert_eq!(selected.pages()[1].frames()[0].repetition_index(), Some(0));
        assert_eq!(selected.pages()[3].frames()[0].repetition_index(), Some(1));
        assert_eq!(selected.pages()[2].frames()[2].repetition_index(), Some(0));
        assert_eq!(selected.pages()[4].frames()[2].repetition_index(), Some(1));
    }

    #[test]
    fn page_master_selected_record_limit_is_inclusive_and_body_tamper_is_rejected() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let layout = fixture();
        let pages = vec![
            StagingHeaderFooterBodyPage {
                page_index: 0,
                before_position: 0,
                after_position: 4,
                terminal: false,
            },
            StagingHeaderFooterBodyPage {
                page_index: 1,
                before_position: 4,
                after_position: 6,
                terminal: false,
            },
            StagingHeaderFooterBodyPage {
                page_index: 2,
                before_position: 6,
                after_position: 7,
                terminal: true,
            },
        ];

        let exact_limits = ResourceLimits {
            max_fragments: 19,
            ..ResourceLimits::default()
        };
        let exact_limits = ValidatedResourceLimits::new(exact_limits).unwrap();
        assert!(paginate_staging_header_footer(&layout, &pages, &exact_limits).is_ok());

        let over_limits = ResourceLimits {
            max_fragments: 18,
            ..ResourceLimits::default()
        };
        let over_limits = ValidatedResourceLimits::new(over_limits).unwrap();
        assert!(matches!(
            paginate_staging_header_footer(&layout, &pages, &over_limits),
            Err(StagingHeaderFooterPaginationError::FragmentLimit)
        ));

        let mut zero_progress = pages.clone();
        zero_progress[1].after_position = zero_progress[1].before_position;
        zero_progress[2].before_position = zero_progress[1].after_position;
        assert!(matches!(
            paginate_staging_header_footer(&layout, &zero_progress, &limits),
            Err(StagingHeaderFooterPaginationError::NonCanonicalBodyPage)
        ));

        let mut tampered = pages;
        tampered[1].before_position = 0;
        assert!(matches!(
            paginate_staging_header_footer(&layout, &tampered, &limits),
            Err(StagingHeaderFooterPaginationError::NonCanonicalBodyPage)
        ));
    }

    #[test]
    fn page_master_receipt_and_page_limit_reject_before_extra_state() {
        let layout = fixture();
        let one_page_limits = ResourceLimits {
            max_pages: 1,
            ..ResourceLimits::default()
        };
        let one_page_limits = ValidatedResourceLimits::new(one_page_limits).unwrap();
        let mut pages = Vec::new();
        push_body_page(
            &mut pages,
            StagingHeaderFooterBodyPage {
                page_index: 0,
                before_position: 0,
                after_position: 7,
                terminal: true,
            },
            &one_page_limits,
        )
        .unwrap();
        assert!(matches!(
            push_body_page(
                &mut pages,
                StagingHeaderFooterBodyPage {
                    page_index: 1,
                    before_position: 7,
                    after_position: 7,
                    terminal: true,
                },
                &one_page_limits,
            ),
            Err(StagingHeaderFooterPaginationError::PageLimit)
        ));
        assert_eq!(pages.len(), 1);

        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let mut selected = paginate_staging_header_footer(&layout, &pages, &limits).unwrap();
        selected.verify_receipt().unwrap();
        selected.pages[0].master_id = MasterId::new("tampered").unwrap();
        assert!(matches!(
            selected.verify_receipt(),
            Err(StagingHeaderFooterPaginationError::SelectedReceiptMismatch)
        ));
    }
}
