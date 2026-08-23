#![forbid(unsafe_code)]

use core::cmp::Ordering;
use core::num::NonZeroU32;
use typaxis_core::{
    AnchorId, DocumentFingerprint, FootnoteId, MasterId, NodeId, NonNegativeLength, PageName,
    Point, Rect, StyleFingerprint,
};
use typaxis_document::DocumentNodeKind;
pub use typaxis_layout_contract::{
    LayoutEpoch, LayoutEpochError, LayoutTextStyleError, ResolvedLayoutTextStyle,
    ShapeFontSelectionError, ShapeFontSelectionReceipt,
};
use typaxis_linebreak::ValidatedParagraphItemRegistry;
use typaxis_style::{
    PageMaster, PageMasterValidationError, PageSelectionContext, PageSelectionError,
};
use typaxis_syntax::{PackagePaginationContext, PackageStyleError, ValidatedParsedPackage};
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FlowPosition {
    epoch: LayoutEpoch,
    global_flow_ordinal: u64,
    owner: NodeId,
    block_child_path: Vec<u32>,
    owner_local_boundary: u32,
}
impl FlowPosition {
    fn new(
        epoch: LayoutEpoch,
        global_flow_ordinal: u64,
        owner: NodeId,
        block_child_path: Vec<u32>,
        owner_local_boundary: u32,
    ) -> Self {
        Self {
            epoch,
            global_flow_ordinal,
            owner,
            block_child_path,
            owner_local_boundary,
        }
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn global_flow_ordinal(&self) -> u64 {
        self.global_flow_ordinal
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn block_child_path(&self) -> &[u32] {
        &self.block_child_path
    }
    pub const fn owner_local_boundary(&self) -> u32 {
        self.owner_local_boundary
    }
    pub fn cmp_within_epoch(&self, other: &Self) -> Result<Ordering, FragmentError> {
        if self.epoch != other.epoch {
            return Err(FragmentError::InvalidCursorEpoch);
        }
        Ok((
            self.global_flow_ordinal,
            self.owner,
            &self.block_child_path,
            self.owner_local_boundary,
        )
            .cmp(&(
                other.global_flow_ordinal,
                other.owner,
                &other.block_child_path,
                other.owner_local_boundary,
            )))
    }
}

/// One owner-local boundary in canonical FlowTree preorder. The FlowTree,
/// rather than a caller-provided ordinal, assigns its global position.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum FlowBoundaryKind {
    DocumentStart,
    ParagraphItem,
    TableRow,
    ListItem,
    BlockItem,
    End,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct FlowBoundary {
    owner: NodeId,
    block_child_path: Vec<u32>,
    owner_local_boundary: u32,
    kind: FlowBoundaryKind,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CursorPosition {
    DocumentStart,
    ParagraphItem(u32),
    TableRow(u32),
    ListItem(u32),
    BlockItem(u32),
    End,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowCursor {
    owner_node: NodeId,
    epoch: LayoutEpoch,
    position: FlowPosition,
    location: CursorPosition,
}
impl FlowCursor {
    pub fn document_start(flow: &FlowTree) -> Self {
        let position = flow.positions[0].clone();
        Self {
            owner_node: flow.root_node,
            epoch: flow.epoch,
            position,
            location: CursorPosition::DocumentStart,
        }
    }
    pub fn at(
        flow: &FlowTree,
        global_flow_ordinal: u64,
        location: CursorPosition,
    ) -> Result<Self, FragmentError> {
        let position = flow
            .positions
            .get(
                usize::try_from(global_flow_ordinal)
                    .map_err(|_| FragmentError::UnknownFlowPosition)?,
            )
            .ok_or(FragmentError::UnknownFlowPosition)?
            .clone();
        let terminal_ordinal = u64::try_from(flow.positions.len() - 1)
            .map_err(|_| FragmentError::UnknownFlowPosition)?;
        let location_matches = match location {
            CursorPosition::DocumentStart => {
                global_flow_ordinal == 0
                    && flow.boundary_kind(global_flow_ordinal)
                        == Some(FlowBoundaryKind::DocumentStart)
            }
            CursorPosition::ParagraphItem(index) => {
                index == position.owner_local_boundary()
                    && flow.boundary_kind(global_flow_ordinal)
                        == Some(FlowBoundaryKind::ParagraphItem)
            }
            CursorPosition::TableRow(index) => {
                index == position.owner_local_boundary()
                    && flow.boundary_kind(global_flow_ordinal) == Some(FlowBoundaryKind::TableRow)
            }
            CursorPosition::ListItem(index) => {
                index == position.owner_local_boundary()
                    && flow.boundary_kind(global_flow_ordinal) == Some(FlowBoundaryKind::ListItem)
            }
            CursorPosition::BlockItem(index) => {
                index == position.owner_local_boundary()
                    && flow.boundary_kind(global_flow_ordinal) == Some(FlowBoundaryKind::BlockItem)
            }
            CursorPosition::End => global_flow_ordinal == terminal_ordinal,
        };
        if !location_matches {
            return Err(FragmentError::InvalidCursorLocation);
        }
        Ok(Self {
            owner_node: position.owner(),
            epoch: flow.epoch,
            position,
            location,
        })
    }
    pub const fn owner_node(&self) -> NodeId {
        self.owner_node
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn position(&self) -> &FlowPosition {
        &self.position
    }
    pub fn location(&self) -> &CursorPosition {
        &self.location
    }
    pub fn is_end(&self) -> bool {
        matches!(self.location, CursorPosition::End)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowTree {
    root_node: NodeId,
    epoch: LayoutEpoch,
    positions: Vec<FlowPosition>,
    boundary_kinds: Vec<FlowBoundaryKind>,
    anchors: std::collections::BTreeMap<AnchorId, NodeId>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowTreeError {
    MissingDocumentStart,
    DuplicateBoundary,
    NonDenseOwnerBoundary,
    TooManyBoundaries,
    UnknownOwner,
    InvalidOwnerKind,
    NonEmptyDocument,
    MissingOwnerBoundary,
    EpochPackageMismatch,
    InvalidOwnerBoundary,
    UnsupportedFlowDomain,
    ParagraphItemRegistryMismatch,
}

/// Sole issuer for canonical flow boundaries. Owners and typed child paths
/// come from a validated document index; owner-local ordinals are assigned by
/// this builder rather than supplied by layout workers.
pub struct CanonicalFlowIrBuilder<'a> {
    package: &'a ValidatedParsedPackage,
    paragraph_items: &'a ValidatedParagraphItemRegistry,
    boundaries: Vec<FlowBoundary>,
    inserted_boundaries: std::collections::BTreeMap<NodeId, u32>,
}

impl<'a> CanonicalFlowIrBuilder<'a> {
    pub fn new(
        package: &'a ValidatedParsedPackage,
        paragraph_items: &'a ValidatedParagraphItemRegistry,
    ) -> Result<Self, FlowTreeError> {
        if !package.package().document.footnotes.is_empty() {
            return Err(FlowTreeError::UnsupportedFlowDomain);
        }
        if paragraph_items.epoch().document() != package.epoch_identity().document()
            || paragraph_items.epoch().style() != package.epoch_identity().style()
        {
            return Err(FlowTreeError::ParagraphItemRegistryMismatch);
        }
        let document_nodes = package.document_nodes();
        let root = NodeId::new(0);
        if document_nodes.node_kind(root) != Some(DocumentNodeKind::Document)
            || document_nodes.node_path(root) != Some([].as_slice())
        {
            return Err(FlowTreeError::MissingDocumentStart);
        }
        Ok(Self {
            package,
            paragraph_items,
            boundaries: vec![FlowBoundary {
                owner: root,
                block_child_path: Vec::new(),
                owner_local_boundary: 0,
                kind: FlowBoundaryKind::DocumentStart,
            }],
            inserted_boundaries: std::collections::BTreeMap::new(),
        })
    }
    /// `item_index` is the semantic paragraph-item index, not a worker
    /// allocation ordinal. Finish canonicalizes insertion order and requires a
    /// dense 0-based owner-local sequence.
    pub fn push_paragraph_item(
        &mut self,
        owner: NodeId,
        item_index: u32,
    ) -> Result<(), FlowTreeError> {
        match self.package.document_nodes().node_kind(owner) {
            Some(DocumentNodeKind::Paragraph | DocumentNodeKind::Heading)
                if self
                    .paragraph_items
                    .item_count(owner)
                    .is_some_and(|count| item_index < count) =>
            {
                self.push(owner, item_index, FlowBoundaryKind::ParagraphItem)
            }
            Some(DocumentNodeKind::Paragraph | DocumentNodeKind::Heading) => {
                Err(FlowTreeError::InvalidOwnerBoundary)
            }
            Some(_) => Err(FlowTreeError::InvalidOwnerKind),
            None => Err(FlowTreeError::UnknownOwner),
        }
    }
    pub fn push_table_row(&mut self, owner: NodeId) -> Result<(), FlowTreeError> {
        match self.package.document_nodes().node_kind(owner) {
            Some(DocumentNodeKind::TableRow) => self.push(owner, 0, FlowBoundaryKind::TableRow),
            Some(_) => Err(FlowTreeError::InvalidOwnerKind),
            None => Err(FlowTreeError::UnknownOwner),
        }
    }
    pub fn push_list_item(&mut self, owner: NodeId) -> Result<(), FlowTreeError> {
        match self.package.document_nodes().node_kind(owner) {
            Some(DocumentNodeKind::ListItem) => self.push(owner, 0, FlowBoundaryKind::ListItem),
            Some(_) => Err(FlowTreeError::InvalidOwnerKind),
            None => Err(FlowTreeError::UnknownOwner),
        }
    }
    pub fn push_block_item(&mut self, owner: NodeId) -> Result<(), FlowTreeError> {
        match self.package.document_nodes().node_kind(owner) {
            Some(DocumentNodeKind::Figure | DocumentNodeKind::PageBreak) => {
                self.push(owner, 0, FlowBoundaryKind::BlockItem)
            }
            Some(_) => Err(FlowTreeError::InvalidOwnerKind),
            None => Err(FlowTreeError::UnknownOwner),
        }
    }
    fn push(
        &mut self,
        owner: NodeId,
        owner_local_boundary: u32,
        kind: FlowBoundaryKind,
    ) -> Result<(), FlowTreeError> {
        let path = self
            .package
            .document_nodes()
            .node_path(owner)
            .ok_or(FlowTreeError::UnknownOwner)?
            .to_vec();
        let inserted = self.inserted_boundaries.entry(owner).or_insert(0);
        *inserted = inserted
            .checked_add(1)
            .ok_or(FlowTreeError::TooManyBoundaries)?;
        self.boundaries.push(FlowBoundary {
            owner,
            block_child_path: path,
            owner_local_boundary,
            kind,
        });
        Ok(())
    }
    pub fn finish(self, epoch: LayoutEpoch) -> Result<FlowTree, FlowTreeError> {
        if epoch != self.paragraph_items.epoch()
            || epoch.document() != self.package.epoch_identity().document()
            || epoch.style() != self.package.epoch_identity().style()
        {
            return Err(FlowTreeError::EpochPackageMismatch);
        }
        for (node_id, kind) in self.package.document_nodes().nodes() {
            let needs_boundary = matches!(
                kind,
                DocumentNodeKind::Paragraph
                    | DocumentNodeKind::Heading
                    | DocumentNodeKind::ListItem
                    | DocumentNodeKind::TableRow
                    | DocumentNodeKind::Figure
                    | DocumentNodeKind::PageBreak
            );
            if needs_boundary {
                let expected = self.paragraph_items.item_count(node_id).unwrap_or(1);
                let actual = self.inserted_boundaries.get(&node_id).copied().unwrap_or(0);
                if actual == 0 {
                    return Err(FlowTreeError::MissingOwnerBoundary);
                }
                if actual != expected {
                    return Err(FlowTreeError::InvalidOwnerBoundary);
                }
            }
        }
        let anchors = self
            .package
            .document_nodes()
            .anchors()
            .map(|(id, owner)| (id.clone(), owner))
            .collect();
        FlowTree::from_boundaries(NodeId::new(0), epoch, self.boundaries, anchors)
    }
}

impl FlowTree {
    fn from_boundaries(
        root_node: NodeId,
        epoch: LayoutEpoch,
        mut boundaries: Vec<FlowBoundary>,
        anchors: std::collections::BTreeMap<AnchorId, NodeId>,
    ) -> Result<Self, FlowTreeError> {
        boundaries.sort_by(|left, right| {
            (
                left.owner,
                &left.block_child_path,
                left.owner_local_boundary,
            )
                .cmp(&(
                    right.owner,
                    &right.block_child_path,
                    right.owner_local_boundary,
                ))
        });
        if !matches!(
            boundaries.first(),
            Some(boundary)
                if boundary.owner == root_node
                    && boundary.block_child_path.is_empty()
                    && boundary.owner_local_boundary == 0
        ) {
            return Err(FlowTreeError::MissingDocumentStart);
        }
        let mut unique = std::collections::BTreeSet::new();
        let mut positions = Vec::with_capacity(boundaries.len());
        let mut boundary_kinds = Vec::with_capacity(boundaries.len());
        let mut previous_group: Option<(NodeId, Vec<u32>, u32)> = None;
        for (ordinal, boundary) in boundaries.into_iter().enumerate() {
            if !unique.insert((
                boundary.owner,
                boundary.block_child_path.clone(),
                boundary.owner_local_boundary,
            )) {
                return Err(FlowTreeError::DuplicateBoundary);
            }
            let expected_local = match &previous_group {
                Some((owner, path, local))
                    if *owner == boundary.owner && *path == boundary.block_child_path =>
                {
                    local
                        .checked_add(1)
                        .ok_or(FlowTreeError::NonDenseOwnerBoundary)?
                }
                _ => 0,
            };
            if boundary.owner_local_boundary != expected_local {
                return Err(FlowTreeError::NonDenseOwnerBoundary);
            }
            previous_group = Some((
                boundary.owner,
                boundary.block_child_path.clone(),
                boundary.owner_local_boundary,
            ));
            positions.push(FlowPosition::new(
                epoch,
                u64::try_from(ordinal).map_err(|_| FlowTreeError::TooManyBoundaries)?,
                boundary.owner,
                boundary.block_child_path,
                boundary.owner_local_boundary,
            ));
            boundary_kinds.push(boundary.kind);
        }
        if positions.len() > 1 {
            positions.push(FlowPosition::new(
                epoch,
                u64::try_from(positions.len()).map_err(|_| FlowTreeError::TooManyBoundaries)?,
                root_node,
                Vec::new(),
                1,
            ));
            boundary_kinds.push(FlowBoundaryKind::End);
        }
        Ok(Self {
            root_node,
            epoch,
            positions,
            boundary_kinds,
            anchors,
        })
    }
    pub fn empty(
        package: &ValidatedParsedPackage,
        epoch: LayoutEpoch,
    ) -> Result<Self, FlowTreeError> {
        if package.document_nodes().node_count() != 1 {
            return Err(FlowTreeError::NonEmptyDocument);
        }
        if epoch.document() != package.epoch_identity().document()
            || epoch.style() != package.epoch_identity().style()
        {
            return Err(FlowTreeError::EpochPackageMismatch);
        }
        FlowTree::from_boundaries(
            NodeId::new(0),
            epoch,
            vec![FlowBoundary {
                owner: NodeId::new(0),
                block_child_path: Vec::new(),
                owner_local_boundary: 0,
                kind: FlowBoundaryKind::DocumentStart,
            }],
            std::collections::BTreeMap::new(),
        )
    }
    pub const fn root_node(&self) -> NodeId {
        self.root_node
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub fn positions(&self) -> &[FlowPosition] {
        &self.positions
    }
    pub fn contains_position(&self, position: &FlowPosition) -> bool {
        position.epoch() == self.epoch
            && usize::try_from(position.global_flow_ordinal())
                .ok()
                .and_then(|index| self.positions.get(index))
                == Some(position)
    }
    pub fn contains_owner(&self, owner: NodeId) -> bool {
        self.positions
            .iter()
            .any(|position| position.owner() == owner)
    }
    pub fn anchor_owner(&self, anchor_id: &AnchorId) -> Option<NodeId> {
        self.anchors.get(anchor_id).copied()
    }
    pub fn anchors(&self) -> impl ExactSizeIterator<Item = (&AnchorId, NodeId)> {
        self.anchors.iter().map(|(id, owner)| (id, *owner))
    }
    pub fn terminal_cursor(&self) -> FlowCursor {
        let position = self.positions[self.positions.len() - 1].clone();
        FlowCursor {
            owner_node: position.owner(),
            epoch: self.epoch,
            position,
            location: CursorPosition::End,
        }
    }
    fn boundary_kind(&self, ordinal: u64) -> Option<FlowBoundaryKind> {
        usize::try_from(ordinal)
            .ok()
            .and_then(|index| self.boundary_kinds.get(index))
            .copied()
    }
    fn is_terminal_position(&self, position: &FlowPosition) -> bool {
        self.positions.last() == Some(position)
    }
    fn is_document_bootstrap(&self, start: &FlowPosition, next: &FlowPosition) -> bool {
        self.contains_position(start)
            && self.contains_position(next)
            && start.global_flow_ordinal() == 0
            && self.boundary_kind(0) == Some(FlowBoundaryKind::DocumentStart)
            && next.global_flow_ordinal() == 1
    }
    fn is_paintable_position(&self, position: &FlowPosition) -> bool {
        self.contains_position(position)
            && matches!(
                self.boundary_kind(position.global_flow_ordinal()),
                Some(
                    FlowBoundaryKind::ParagraphItem
                        | FlowBoundaryKind::TableRow
                        | FlowBoundaryKind::ListItem
                        | FlowBoundaryKind::BlockItem
                )
            )
    }
}

/// Flow-issued site at which the next page begins. The next content owner can
/// differ from the cursor owner at document start.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPageSelection {
    page_start: FlowPosition,
    flow_owner: NodeId,
    content_owner: NodeId,
    style_owner: NodeId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    page_name: Option<PageName>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageStyleResolutionError {
    InvalidCursor,
    EpochPackageMismatch,
    InvalidPackageStyle(PackageStyleError),
}
impl ResolvedPageSelection {
    pub fn new(
        flow: &FlowTree,
        cursor: &FlowCursor,
        package: &ValidatedParsedPackage,
    ) -> Result<Self, PageStyleResolutionError> {
        if cursor.epoch() != flow.epoch()
            || !flow.contains_position(cursor.position())
            || cursor.owner_node() != cursor.position().owner()
        {
            return Err(PageStyleResolutionError::InvalidCursor);
        }
        if flow.epoch().document() != package.epoch_identity().document()
            || flow.epoch().style() != package.epoch_identity().style()
        {
            return Err(PageStyleResolutionError::EpochPackageMismatch);
        }
        let current = usize::try_from(cursor.position().global_flow_ordinal())
            .map_err(|_| PageStyleResolutionError::InvalidCursor)?;
        let blank = flow.positions.len() == 1;
        if blank && !matches!(cursor.location(), CursorPosition::DocumentStart) {
            return Err(PageStyleResolutionError::InvalidCursor);
        }
        let content_owner = if blank {
            flow.root_node
        } else if flow.boundary_kinds[current] == FlowBoundaryKind::DocumentStart {
            flow.positions
                .get(current + 1)
                .ok_or(PageStyleResolutionError::InvalidCursor)?
                .owner()
        } else if cursor.is_end() {
            return Err(PageStyleResolutionError::InvalidCursor);
        } else {
            cursor.position().owner()
        };
        let package_selection = if blank {
            package.resolve_blank_page_selection()
        } else {
            package.resolve_page_selection(content_owner)
        }
        .map_err(PageStyleResolutionError::InvalidPackageStyle)?;
        if package_selection.owner() != content_owner {
            return Err(PageStyleResolutionError::InvalidPackageStyle(
                PackageStyleError::UnknownStyleOwner,
            ));
        }
        Ok(Self {
            page_start: cursor.position().clone(),
            flow_owner: cursor.owner_node(),
            content_owner,
            style_owner: package_selection.style_owner(),
            document: package_selection.document_fingerprint(),
            style: package_selection.style_fingerprint(),
            page_name: package_selection.page_name().cloned(),
        })
    }
    pub const fn page_start(&self) -> &FlowPosition {
        &self.page_start
    }
    pub const fn flow_owner(&self) -> NodeId {
        self.flow_owner
    }
    pub const fn content_owner(&self) -> NodeId {
        self.content_owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
}

/// Page parity and first-page status are derived, never independently stored.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageContext {
    page_index: u32,
    physical_page_number: NonZeroU32,
    named_page: Option<PageName>,
    page_start: FlowPosition,
    flow_owner: NodeId,
    content_owner: NodeId,
    style_owner: NodeId,
    package_document: DocumentFingerprint,
    package_style: StyleFingerprint,
    selected_master: PageMaster,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageContextError {
    PageNumberOverflow,
    InvalidPageMasters(PageMasterValidationError),
    InvalidPageSelection(PageSelectionError),
    PackageStyleMismatch,
}
impl PageContext {
    pub fn select(
        page_index: u32,
        resolved_page: &ResolvedPageSelection,
        package_context: &PackagePaginationContext,
    ) -> Result<Self, PageContextError> {
        if resolved_page.document != package_context.document_fingerprint()
            || resolved_page.style != package_context.style_fingerprint()
        {
            return Err(PageContextError::PackageStyleMismatch);
        }
        let physical_page_number = page_index
            .checked_add(1)
            .and_then(NonZeroU32::new)
            .ok_or(PageContextError::PageNumberOverflow)?;
        let named_page = resolved_page.page_name.clone();
        let selection = PageSelectionContext::new(page_index, named_page.clone())
            .map_err(PageContextError::InvalidPageSelection)?;
        let selected_master = package_context
            .page_masters()
            .select(&selection)
            .map_err(PageContextError::InvalidPageMasters)?
            .clone();
        Ok(Self {
            page_index,
            physical_page_number,
            named_page,
            page_start: resolved_page.page_start.clone(),
            flow_owner: resolved_page.flow_owner,
            content_owner: resolved_page.content_owner,
            style_owner: resolved_page.style_owner,
            package_document: package_context.document_fingerprint(),
            package_style: package_context.style_fingerprint(),
            selected_master,
        })
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn physical_page_number(&self) -> NonZeroU32 {
        self.physical_page_number
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.selected_master.master_id
    }
    /// Returns the exact validated page master selected for this page. Work
    /// permits bind frame geometry to this receipt before layout begins.
    pub const fn selected_master(&self) -> &PageMaster {
        &self.selected_master
    }
    pub const fn package_document_fingerprint(&self) -> DocumentFingerprint {
        self.package_document
    }
    pub const fn package_style_fingerprint(&self) -> StyleFingerprint {
        self.package_style
    }
    pub const fn named_page(&self) -> Option<&PageName> {
        self.named_page.as_ref()
    }
    pub const fn page_start(&self) -> &FlowPosition {
        &self.page_start
    }
    pub const fn flow_owner(&self) -> NodeId {
        self.flow_owner
    }
    pub const fn content_owner(&self) -> NodeId {
        self.content_owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
    pub const fn is_first(&self) -> bool {
        self.page_index == 0
    }
    pub const fn is_odd(&self) -> bool {
        self.physical_page_number.get() % 2 == 1
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FragmentRequest<'a> {
    flow: &'a FlowTree,
    cursor: &'a FlowCursor,
    frame: Rect,
    reserved_footnote_height: NonNegativeLength,
    page: PageContext,
}
impl<'a> FragmentRequest<'a> {
    pub fn new(
        flow: &'a FlowTree,
        cursor: &'a FlowCursor,
        frame: Rect,
        reserved_footnote_height: NonNegativeLength,
        page: PageContext,
    ) -> Result<Self, FragmentError> {
        let request = Self {
            flow,
            cursor,
            frame,
            reserved_footnote_height,
            page,
        };
        request.validate()?;
        Ok(request)
    }
    pub fn validate(&self) -> Result<(), FragmentError> {
        if self.cursor.epoch() != self.flow.epoch {
            return Err(FragmentError::InvalidCursorEpoch);
        }
        if self.page.package_document_fingerprint() != self.flow.epoch.document()
            || self.page.package_style_fingerprint() != self.flow.epoch.style()
            || self.page.page_start().epoch() != self.flow.epoch
            || !self.flow.contains_position(self.page.page_start())
            || self.page.flow_owner() != self.page.page_start().owner()
        {
            return Err(FragmentError::InvalidPageContext);
        }
        if !self.flow.contains_position(self.cursor.position()) {
            return Err(FragmentError::UnknownFlowPosition);
        }
        if self.cursor.owner_node() != self.cursor.position().owner() {
            return Err(FragmentError::InvalidCursorOwner);
        }
        Ok(())
    }
    pub const fn flow(&self) -> &FlowTree {
        self.flow
    }
    pub const fn cursor(&self) -> &FlowCursor {
        self.cursor
    }
    pub const fn frame(&self) -> Rect {
        self.frame
    }
    pub const fn reserved_footnote_height(&self) -> NonNegativeLength {
        self.reserved_footnote_height
    }
    pub const fn page(&self) -> &PageContext {
        &self.page
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FragmentDraft {
    start: FlowPosition,
    end: FlowPosition,
    bounds: Rect,
    break_after_penalty: i32,
}
impl FragmentDraft {
    pub fn new(
        start: FlowPosition,
        end: FlowPosition,
        bounds: Rect,
        break_after_penalty: i32,
    ) -> Result<Self, FragmentError> {
        if start.cmp_within_epoch(&end)? != Ordering::Less {
            return Err(FragmentError::InvalidFragmentRange);
        }
        Ok(Self {
            start,
            end,
            bounds,
            break_after_penalty,
        })
    }
    pub const fn start(&self) -> &FlowPosition {
        &self.start
    }
    pub const fn end(&self) -> &FlowPosition {
        &self.end
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
    pub const fn break_after_penalty(&self) -> i32 {
        self.break_after_penalty
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredAnchor {
    pub anchor_id: AnchorId,
    pub owner_node: NodeId,
    pub position_in_frame: Point,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Continuation {
    Exhausted(Box<FlowCursor>),
    More(Box<FlowCursor>),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FragmentResult {
    pub fragments: Vec<FragmentDraft>,
    pub continuation: Continuation,
    pub discovered_footnotes: Vec<FootnoteId>,
    pub discovered_anchors: Vec<DiscoveredAnchor>,
}
impl FragmentResult {
    pub fn validate_progress(&self, request: &FragmentRequest<'_>) -> Result<(), FragmentError> {
        request.validate()?;
        let input = request.cursor();
        let continuation = match &self.continuation {
            Continuation::Exhausted(terminal)
                if terminal.epoch() != input.epoch()
                    || !request.flow().contains_position(terminal.position()) =>
            {
                return Err(FragmentError::InvalidCursorEpoch);
            }
            Continuation::Exhausted(terminal) if !terminal.is_end() => {
                return Err(FragmentError::InvalidCursorLocation);
            }
            Continuation::Exhausted(terminal) => {
                match terminal.position().cmp_within_epoch(input.position())? {
                    Ordering::Greater | Ordering::Equal => terminal.position(),
                    Ordering::Less => return Err(FragmentError::NoProgress),
                }
            }
            Continuation::More(next) if next.epoch() != input.epoch() => {
                return Err(FragmentError::InvalidCursorEpoch);
            }
            Continuation::More(next)
                if next.is_end() || request.flow().is_terminal_position(next.position()) =>
            {
                return Err(FragmentError::InvalidCursorLocation);
            }
            Continuation::More(next) => match next.position().cmp_within_epoch(input.position())? {
                Ordering::Greater => next.position(),
                Ordering::Equal | Ordering::Less => return Err(FragmentError::NoProgress),
            },
        };
        if self.fragments.is_empty()
            && continuation.cmp_within_epoch(input.position())? == Ordering::Greater
            && !request
                .flow()
                .is_document_bootstrap(input.position(), continuation)
        {
            return Err(FragmentError::InvalidFragmentRange);
        }
        let mut previous_end: Option<&FlowPosition> = None;
        for (index, fragment) in self.fragments.iter().enumerate() {
            if !request.flow().contains_position(fragment.start())
                || !request.flow().contains_position(fragment.end())
            {
                return Err(FragmentError::UnknownFlowPosition);
            }
            if !request.flow().is_paintable_position(fragment.start())
                || fragment.start().cmp_within_epoch(fragment.end())? != Ordering::Less
                || (index == 0 && fragment.start() != input.position())
                || fragment.end().cmp_within_epoch(continuation)? == Ordering::Greater
                || previous_end.is_some_and(|end| {
                    end.cmp_within_epoch(fragment.start())
                        .is_ok_and(|ordering| ordering != Ordering::Equal)
                })
            {
                return Err(FragmentError::InvalidFragmentRange);
            }
            previous_end = Some(fragment.end());
        }
        if previous_end.is_some_and(|end| end != continuation) {
            return Err(FragmentError::InvalidFragmentRange);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FragmentError {
    InvalidCursorEpoch,
    InvalidCursorOwner,
    InvalidCursorLocation,
    UnknownFlowPosition,
    NoProgress,
    Unplaceable,
    ArithmeticOverflow,
    ResourceLimit,
    InvalidFragmentRange,
    InvalidFragmentKey,
    InvalidPageContext,
    InvalidFloatState,
    UnsupportedFlowDomain,
}
pub trait FragmentWorkBudget {
    fn consume_fragments(&mut self, count: u64) -> Result<(), FragmentError>;
    fn consume_footnote_reflow(&mut self, page_index: u32) -> Result<(), FragmentError>;
    fn consume_column_candidate(&mut self, container: NodeId) -> Result<(), FragmentError>;
    fn enqueue_float(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError>;
    fn dequeue_float(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError>;
    fn consume_float_carry(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError>;
}
pub trait Fragmenter {
    fn fragment(
        &self,
        request: &FragmentRequest<'_>,
        budget: &mut dyn FragmentWorkBudget,
    ) -> Result<FragmentResult, FragmentError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{PortablePath, ResourceLimits, SourceId, ValidatedResourceLimits};
    use typaxis_resource_admission::{AdmittedFontInstanceTable, AdmittedResourceResolver};
    use typaxis_style::StyleValidationError;
    use typaxis_syntax::{
        PackageGeneratedTextError, PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser,
        SourceFile, ValidatedParsedPackage,
    };
    use typaxis_text::{GeneratedTextStore, TextStore};
    fn parsed_reference_package(seed: u8, text: &str) -> ValidatedParsedPackage {
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new(format!("input-{seed}.tsf")).unwrap(),
            text: text.to_owned(),
        };
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        match ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        ) {
            ParseOutcome::Parsed { package, .. } => *package,
            ParseOutcome::Failed { failure } => panic!("reference parse failed: {failure:?}"),
        }
    }
    fn validated_package(seed: u8) -> ValidatedParsedPackage {
        parsed_reference_package(seed, "")
    }
    fn paragraph_package(seed: u8) -> ValidatedParsedPackage {
        parsed_reference_package(seed, "paragraph\nparagraph")
    }
    fn epoch(package: &ValidatedParsedPackage) -> LayoutEpoch {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let generated = package.bind_generated_text(&generated, &limits).unwrap();
        LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap()
    }
    fn frame() -> Rect {
        let size =
            typaxis_core::PositiveLength::new(typaxis_core::Length::from_raw(10).unwrap()).unwrap();
        Rect::new(
            typaxis_core::Length::ZERO,
            typaxis_core::Length::ZERO,
            size,
            size,
        )
    }
    #[test]
    fn page_flags_are_derived() {
        let package = validated_package(1);
        let flow = FlowTree::empty(&package, epoch(&package)).unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let selection = ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let package_context = package.pagination_context();
        let context = PageContext::select(0, &selection, &package_context).unwrap();
        assert!(context.is_first());
        assert!(context.is_odd());
        assert_eq!(context.physical_page_number().get(), 1);
        assert_eq!(
            PageContext::select(u32::MAX, &selection, &package_context),
            Err(PageContextError::PageNumberOverflow)
        );
    }
    #[test]
    fn request_rejects_cursor_from_another_epoch() {
        let package = validated_package(1);
        let other_package = validated_package(9);
        assert_eq!(
            FlowTree::empty(&package, epoch(&other_package)),
            Err(FlowTreeError::EpochPackageMismatch)
        );
        let flow = FlowTree::empty(&package, epoch(&package)).unwrap();
        let other = FlowTree::empty(&other_package, epoch(&other_package)).unwrap();
        let cursor = FlowCursor::document_start(&other);
        let package_context = package.pagination_context();
        let selection =
            ResolvedPageSelection::new(&flow, &FlowCursor::document_start(&flow), &package)
                .unwrap();
        let page = PageContext::select(0, &selection, &package_context).unwrap();
        assert_eq!(
            FragmentRequest::new(&flow, &cursor, frame(), NonNegativeLength::ZERO, page),
            Err(FragmentError::InvalidCursorEpoch)
        );
    }
    #[test]
    fn epoch_rejects_generated_overlay_from_another_document_registry() {
        let package = validated_package(1);
        let paragraph_package = paragraph_package(2);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            vec![],
            paragraph_package.document_nodes(),
            &limits,
            &TextStore::new(vec![]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            package.bind_generated_text(&generated, &limits),
            Err(PackageGeneratedTextError::DocumentMismatch)
        );
    }
    #[test]
    fn resolved_text_style_is_bound_to_package_style_and_admission() {
        let package = paragraph_package(1);
        let other = paragraph_package(2);
        let computed = other.cascade_style(NodeId::new(1)).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        assert_eq!(
            ResolvedLayoutTextStyle::new(&package, &computed, admitted.token()),
            Err(LayoutTextStyleError::PackageStyleMismatch)
        );

        let computed = package.cascade_style(NodeId::new(1)).unwrap();
        assert_eq!(
            ResolvedLayoutTextStyle::new(&package, &computed, admitted.token()),
            Err(LayoutTextStyleError::InvalidStyle(
                StyleValidationError::MissingTextProperty
            ))
        );

        let instances = AdmittedFontInstanceTable::from_used_faces(&admitted, []).unwrap();
        let other_computed = other.cascade_style(NodeId::new(1)).unwrap();
        assert_eq!(
            ShapeFontSelectionReceipt::new(
                &package,
                &other_computed,
                admitted.token(),
                &instances,
                epoch(&package),
            ),
            Err(ShapeFontSelectionError::LayoutStyle(
                LayoutTextStyleError::PackageStyleMismatch
            ))
        );
        assert_eq!(
            ShapeFontSelectionReceipt::new(
                &package,
                &computed,
                admitted.token(),
                &instances,
                epoch(&package),
            ),
            Err(ShapeFontSelectionError::LayoutStyle(
                LayoutTextStyleError::InvalidStyle(StyleValidationError::MissingTextProperty)
            ))
        );
    }
    #[test]
    fn continuation_requires_monotonic_structured_progress() {
        let package = paragraph_package(1);
        let package_epoch = epoch(&package);
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(&package, package_epoch).unwrap();
        let mut builder = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        builder.push_paragraph_item(NodeId::new(1), 0).unwrap();
        builder.push_paragraph_item(NodeId::new(2), 0).unwrap();
        let flow = builder.finish(package_epoch).unwrap();
        assert_eq!(flow.positions().len(), 4);
        assert_eq!(flow.positions().last().unwrap().owner(), NodeId::new(0));
        assert_eq!(flow.positions().last().unwrap().owner_local_boundary(), 1);
        let cursor = FlowCursor::document_start(&flow);
        let request = FragmentRequest::new(
            &flow,
            &cursor,
            frame(),
            NonNegativeLength::ZERO,
            PageContext::select(
                0,
                &ResolvedPageSelection::new(&flow, &cursor, &package).unwrap(),
                &package.pagination_context(),
            )
            .unwrap(),
        )
        .unwrap();
        let stalled = FlowCursor::document_start(&flow);
        let result = FragmentResult {
            fragments: vec![],
            continuation: Continuation::More(Box::new(stalled)),
            discovered_footnotes: vec![],
            discovered_anchors: vec![],
        };
        assert_eq!(
            result.validate_progress(&request),
            Err(FragmentError::NoProgress)
        );

        let advanced = FlowCursor::at(&flow, 1, CursorPosition::ParagraphItem(0)).unwrap();
        let result = FragmentResult {
            fragments: vec![],
            continuation: Continuation::More(Box::new(advanced.clone())),
            discovered_footnotes: vec![],
            discovered_anchors: vec![],
        };
        assert!(result.validate_progress(&request).is_ok());

        let root_fragment = FragmentDraft::new(
            flow.positions()[0].clone(),
            flow.positions()[1].clone(),
            frame(),
            0,
        )
        .unwrap();
        assert_eq!(
            FragmentResult {
                fragments: vec![root_fragment],
                continuation: Continuation::More(Box::new(advanced.clone())),
                discovered_footnotes: vec![],
                discovered_anchors: vec![],
            }
            .validate_progress(&request),
            Err(FragmentError::InvalidFragmentRange)
        );

        let advanced_request = FragmentRequest::new(
            &flow,
            &advanced,
            frame(),
            NonNegativeLength::ZERO,
            request.page().clone(),
        )
        .unwrap();

        let terminal = flow.terminal_cursor();
        assert_eq!(
            FragmentResult {
                fragments: vec![],
                continuation: Continuation::More(Box::new(terminal.clone())),
                discovered_footnotes: vec![],
                discovered_anchors: vec![],
            }
            .validate_progress(&request),
            Err(FragmentError::InvalidCursorLocation)
        );
        assert!(FragmentResult {
            fragments: vec![FragmentDraft::new(
                flow.positions()[1].clone(),
                flow.positions().last().unwrap().clone(),
                frame(),
                0,
            )
            .unwrap()],
            continuation: Continuation::Exhausted(Box::new(terminal)),
            discovered_footnotes: vec![],
            discovered_anchors: vec![],
        }
        .validate_progress(&advanced_request)
        .is_ok());

        assert_eq!(
            FlowCursor::at(&flow, 0, CursorPosition::End,),
            Err(FragmentError::InvalidCursorLocation)
        );

        let mut same = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        same.push_paragraph_item(NodeId::new(2), 0).unwrap();
        same.push_paragraph_item(NodeId::new(1), 0).unwrap();
        assert_eq!(
            flow.positions(),
            same.finish(package_epoch).unwrap().positions()
        );

        let mut invalid = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        assert_eq!(
            invalid.push_table_row(NodeId::new(1)),
            Err(FlowTreeError::InvalidOwnerKind)
        );
        assert_eq!(
            invalid.push_paragraph_item(NodeId::new(99), 0),
            Err(FlowTreeError::UnknownOwner)
        );
        assert_eq!(
            invalid.push_paragraph_item(NodeId::new(1), 1),
            Err(FlowTreeError::InvalidOwnerBoundary)
        );
        assert_eq!(
            FlowTree::empty(&package, epoch(&package)),
            Err(FlowTreeError::NonEmptyDocument)
        );
        assert_eq!(
            CanonicalFlowIrBuilder::new(&package, &paragraph_items)
                .unwrap()
                .finish(package_epoch),
            Err(FlowTreeError::MissingOwnerBoundary)
        );

        let valid_fragment = FragmentDraft::new(
            flow.positions()[1].clone(),
            flow.positions().last().unwrap().clone(),
            frame(),
            0,
        )
        .unwrap();
        assert_eq!(valid_fragment.start(), &flow.positions()[1]);
        assert_eq!(valid_fragment.end(), flow.positions().last().unwrap());
        assert_eq!(
            FragmentDraft::new(
                flow.positions().last().unwrap().clone(),
                flow.positions()[1].clone(),
                frame(),
                0,
            ),
            Err(FragmentError::InvalidFragmentRange)
        );
        let terminal = flow.terminal_cursor();
        assert!(FragmentResult {
            fragments: vec![valid_fragment.clone()],
            continuation: Continuation::Exhausted(Box::new(terminal.clone())),
            discovered_footnotes: vec![],
            discovered_anchors: vec![],
        }
        .validate_progress(&advanced_request)
        .is_ok());
        assert_eq!(
            FragmentResult {
                fragments: vec![valid_fragment.clone(), valid_fragment],
                continuation: Continuation::Exhausted(Box::new(terminal)),
                discovered_footnotes: vec![],
                discovered_anchors: vec![],
            }
            .validate_progress(&advanced_request),
            Err(FragmentError::InvalidFragmentRange)
        );
        let other_package = paragraph_package(2);
        let other_epoch = epoch(&other_package);
        let other_items =
            ValidatedParagraphItemRegistry::for_empty_content(&other_package, other_epoch).unwrap();
        let mut other_builder = CanonicalFlowIrBuilder::new(&other_package, &other_items).unwrap();
        other_builder
            .push_paragraph_item(NodeId::new(1), 0)
            .unwrap();
        other_builder
            .push_paragraph_item(NodeId::new(2), 0)
            .unwrap();
        let other = other_builder.finish(other_epoch).unwrap();
        let outside = FragmentDraft::new(
            flow.positions()[1].clone(),
            other.positions()[1].clone(),
            frame(),
            0,
        );
        assert_eq!(outside, Err(FragmentError::InvalidCursorEpoch));
    }
}
