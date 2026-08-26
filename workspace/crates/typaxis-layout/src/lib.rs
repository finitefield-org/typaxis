#![forbid(unsafe_code)]

use core::cmp::Ordering;
use core::num::NonZeroU32;
use typaxis_core::{
    AnchorId, DocumentFingerprint, FootnoteId, Length, MasterId, NodeId, NonNegativeLength,
    PageName, Point, PositiveLength, Rect, StyleFingerprint,
};
use typaxis_document::{Block, DocumentNodeKind, Inline};
pub use typaxis_layout_contract::{
    LayoutEpoch, LayoutEpochError, LayoutTextStyleError, MachineGlyphCoverage,
    MachineStyleFontPreparationError, MachineTextSiteSource, PreparedMachineStyleFonts,
    PreparedMachineTextSite, ResolvedLayoutTextStyle, ShapeFontSelectionError,
    ShapeFontSelectionReceipt,
};
use typaxis_linebreak::ValidatedParagraphItemRegistry;
use typaxis_style::{
    PageMaster, PageMasterValidationError, PageSelectionContext, PageSelectionError, StyleValue,
};
use typaxis_syntax::{
    PackagePaginationContext, PackageStyleError, ValidatedMachinePackage, ValidatedParsedPackage,
};
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
    paragraph_items: Option<ValidatedParagraphItemRegistry>,
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
        FlowTree::from_boundaries(
            NodeId::new(0),
            epoch,
            self.boundaries,
            anchors,
            Some(self.paragraph_items.clone()),
        )
    }
}

/// Paragraph-only flow builder reachable only after machine style/font
/// preparation has bound the same package and stable layout epoch.
pub struct MachineParagraphFlowBuilder<'a> {
    inner: CanonicalFlowIrBuilder<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineParagraphFlowError {
    PreparationMismatch,
    Flow(FlowTreeError),
}

impl<'a> MachineParagraphFlowBuilder<'a> {
    pub fn new(
        package: &'a ValidatedMachinePackage,
        paragraph_items: &'a ValidatedParagraphItemRegistry,
        preparation: &PreparedMachineStyleFonts,
    ) -> Result<Self, MachineParagraphFlowError> {
        if !preparation.matches_package_epoch(package, paragraph_items.epoch()) {
            return Err(MachineParagraphFlowError::PreparationMismatch);
        }
        let inner = CanonicalFlowIrBuilder::new(package.package(), paragraph_items)
            .map_err(MachineParagraphFlowError::Flow)?;
        Ok(Self { inner })
    }

    pub fn push_paragraph_item(
        &mut self,
        owner: NodeId,
        item_index: u32,
    ) -> Result<(), MachineParagraphFlowError> {
        self.inner
            .push_paragraph_item(owner, item_index)
            .map_err(MachineParagraphFlowError::Flow)
    }

    pub fn finish(self, epoch: LayoutEpoch) -> Result<FlowTree, MachineParagraphFlowError> {
        self.inner
            .finish(epoch)
            .map_err(MachineParagraphFlowError::Flow)
    }
}

impl FlowTree {
    fn from_boundaries(
        root_node: NodeId,
        epoch: LayoutEpoch,
        mut boundaries: Vec<FlowBoundary>,
        anchors: std::collections::BTreeMap<AnchorId, NodeId>,
        paragraph_items: Option<ValidatedParagraphItemRegistry>,
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
            paragraph_items,
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
            None,
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
    pub fn paragraph_items(&self) -> Option<&ValidatedParagraphItemRegistry> {
        self.paragraph_items.as_ref()
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReferenceAnchorPlacement {
    flow_ordinal: u64,
    anchor_id: AnchorId,
    owner_node: NodeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReferenceLinePlacement {
    start: usize,
    end: usize,
    height: PositiveLength,
}

fn reference_line_height(
    package: &ValidatedParsedPackage,
    owner: NodeId,
) -> Result<PositiveLength, FragmentError> {
    let computed = package
        .cascade_style(owner)
        .map_err(|_| FragmentError::InvalidFragmentKey)?;
    match computed.computed().properties().get("line_height") {
        Some(StyleValue::Length(value)) => {
            PositiveLength::new(*value).ok_or(FragmentError::InvalidFragmentKey)
        }
        // Empty reference paragraphs intentionally have no text style. Their
        // legacy fragment mode paints no glyphs and replaces this placeholder
        // with the complete requested frame.
        None => PositiveLength::new(Length::from_raw(1).ok_or(FragmentError::ArithmeticOverflow)?)
            .ok_or(FragmentError::ArithmeticOverflow),
        Some(_) => Err(FragmentError::InvalidFragmentKey),
    }
}

/// Deterministic reference fragmenter for validated top-level paragraphs and
/// headings. Line ranges come from the paragraph-break receipts retained by
/// the canonical FlowTree; callers cannot substitute item counts or breaks.
#[derive(Clone, Debug)]
pub struct ReferenceFragmenter<'flow> {
    flow: &'flow FlowTree,
    anchors: Vec<ReferenceAnchorPlacement>,
    lines: Vec<ReferenceLinePlacement>,
    legacy_full_frame: bool,
}

impl<'flow> ReferenceFragmenter<'flow> {
    pub fn for_empty_paragraphs(
        package: &ValidatedParsedPackage,
        flow: &'flow FlowTree,
    ) -> Result<Self, FragmentError> {
        if !package.package().document.footnotes.is_empty()
            || !package.package().text_store.buffers().is_empty()
            || package.document_nodes().generated_sites().len() != 0
        {
            return Err(FragmentError::UnsupportedFlowDomain);
        }
        let mut fragmenter = Self::for_paragraphs(package, flow)?;
        fragmenter.legacy_full_frame = true;
        Ok(fragmenter)
    }

    pub fn for_paragraphs(
        package: &ValidatedParsedPackage,
        flow: &'flow FlowTree,
    ) -> Result<Self, FragmentError> {
        if !package.package().document.footnotes.is_empty() {
            return Err(FragmentError::UnsupportedFlowDomain);
        }
        if flow.epoch.document() != package.epoch_identity().document()
            || flow.epoch.style() != package.epoch_identity().style()
        {
            return Err(FragmentError::InvalidCursorEpoch);
        }
        let Some(root_position) = flow.positions.first() else {
            return Err(FragmentError::InvalidFragmentKey);
        };
        if flow.root_node != NodeId::new(0)
            || package.document_nodes().node_kind(flow.root_node)
                != Some(DocumentNodeKind::Document)
            || root_position.owner() != flow.root_node
            || !root_position.block_child_path().is_empty()
            || root_position.owner_local_boundary() != 0
            || flow.boundary_kinds.first() != Some(&FlowBoundaryKind::DocumentStart)
        {
            return Err(FragmentError::InvalidFragmentKey);
        }

        let blocks = &package.package().document.blocks;
        if blocks
            .iter()
            .any(|block| !matches!(block, Block::Paragraph { .. } | Block::Heading { .. }))
        {
            return Err(FragmentError::UnsupportedFlowDomain);
        }
        let registry = flow.paragraph_items();
        if !blocks.is_empty() && registry.is_none() {
            return Err(FragmentError::InvalidFragmentKey);
        }
        let paragraph_item_count = blocks.iter().try_fold(0usize, |total, block| {
            let node_id = match block {
                Block::Paragraph { node_id, .. } | Block::Heading { node_id, .. } => *node_id,
                _ => return Err(FragmentError::UnsupportedFlowDomain),
            };
            let count = registry
                .and_then(|items| items.item_count(node_id))
                .ok_or(FragmentError::InvalidFragmentKey)?;
            total
                .checked_add(usize::try_from(count).map_err(|_| FragmentError::ArithmeticOverflow)?)
                .ok_or(FragmentError::ArithmeticOverflow)
        })?;
        let expected_position_count = if blocks.is_empty() {
            1
        } else {
            paragraph_item_count
                .checked_add(2)
                .ok_or(FragmentError::ArithmeticOverflow)?
        };
        if flow.positions.len() != expected_position_count
            || flow.boundary_kinds.len() != expected_position_count
        {
            return Err(FragmentError::InvalidFragmentKey);
        }

        let mut anchors = Vec::new();
        let mut lines = Vec::new();
        let mut position_index = 1usize;
        for block in blocks {
            let (node_id, heading_anchor, children) = match block {
                Block::Paragraph {
                    node_id, children, ..
                } => (*node_id, None, children.as_slice()),
                Block::Heading {
                    node_id,
                    anchor_id,
                    children,
                    ..
                } => (*node_id, anchor_id.as_ref(), children.as_slice()),
                _ => return Err(FragmentError::UnsupportedFlowDomain),
            };
            let expected_path = package
                .document_nodes()
                .node_path(node_id)
                .ok_or(FragmentError::InvalidFragmentKey)?;
            let item_count = registry
                .and_then(|items| items.item_count(node_id))
                .ok_or(FragmentError::InvalidFragmentKey)?;
            let line_height = reference_line_height(package, node_id)?;
            for local in 0..item_count {
                let position = flow
                    .positions
                    .get(position_index)
                    .ok_or(FragmentError::InvalidFragmentKey)?;
                if position.owner() != node_id
                    || position.block_child_path() != expected_path
                    || position.owner_local_boundary() != local
                    || flow.boundary_kinds[position_index] != FlowBoundaryKind::ParagraphItem
                {
                    return Err(FragmentError::InvalidFragmentKey);
                }
                position_index = position_index
                    .checked_add(1)
                    .ok_or(FragmentError::ArithmeticOverflow)?;
            }
            let paragraph_start = position_index
                .checked_sub(
                    usize::try_from(item_count).map_err(|_| FragmentError::ArithmeticOverflow)?,
                )
                .ok_or(FragmentError::ArithmeticOverflow)?;
            let mut previous_item = 0u32;
            if let Some(result) = registry.and_then(|items| items.paragraph_break(node_id)) {
                for line in &result.lines {
                    if line.item_index <= previous_item || line.item_index > item_count {
                        return Err(FragmentError::InvalidFragmentKey);
                    }
                    lines.push(ReferenceLinePlacement {
                        start: paragraph_start
                            .checked_add(
                                usize::try_from(previous_item)
                                    .map_err(|_| FragmentError::ArithmeticOverflow)?,
                            )
                            .ok_or(FragmentError::ArithmeticOverflow)?,
                        end: paragraph_start
                            .checked_add(
                                usize::try_from(line.item_index)
                                    .map_err(|_| FragmentError::ArithmeticOverflow)?,
                            )
                            .ok_or(FragmentError::ArithmeticOverflow)?,
                        height: line_height,
                    });
                    previous_item = line.item_index;
                }
                if previous_item != item_count {
                    return Err(FragmentError::InvalidFragmentKey);
                }
            } else {
                lines.push(ReferenceLinePlacement {
                    start: paragraph_start,
                    end: position_index,
                    height: line_height,
                });
            }
            if let Some(anchor_id) = heading_anchor {
                if package.document_nodes().anchor_owner(anchor_id) != Some(node_id)
                    || flow.anchor_owner(anchor_id) != Some(node_id)
                {
                    return Err(FragmentError::InvalidFragmentKey);
                }
                anchors.push(ReferenceAnchorPlacement {
                    flow_ordinal: u64::try_from(paragraph_start)
                        .map_err(|_| FragmentError::ArithmeticOverflow)?,
                    anchor_id: anchor_id.clone(),
                    owner_node: node_id,
                });
            }
            collect_reference_anchors(
                children,
                package,
                flow,
                u64::try_from(paragraph_start).map_err(|_| FragmentError::ArithmeticOverflow)?,
                &mut anchors,
            )?;
        }

        if !blocks.is_empty() {
            let terminal_index = expected_position_count - 1;
            let terminal = &flow.positions[terminal_index];
            if terminal.owner() != flow.root_node
                || !terminal.block_child_path().is_empty()
                || terminal.owner_local_boundary() != 1
                || flow.boundary_kinds[terminal_index] != FlowBoundaryKind::End
            {
                return Err(FragmentError::InvalidFragmentKey);
            }
        }

        anchors.sort_by(|left, right| {
            (left.flow_ordinal, &left.anchor_id).cmp(&(right.flow_ordinal, &right.anchor_id))
        });
        if anchors
            .windows(2)
            .any(|pair| pair[0].anchor_id == pair[1].anchor_id)
            || anchors.len() != flow.anchors.len()
            || anchors
                .iter()
                .any(|anchor| flow.anchor_owner(&anchor.anchor_id) != Some(anchor.owner_node))
        {
            return Err(FragmentError::InvalidFragmentKey);
        }
        Ok(Self {
            flow,
            anchors,
            lines,
            legacy_full_frame: false,
        })
    }

    fn cursor_at(&self, position_index: usize) -> Result<FlowCursor, FragmentError> {
        let position = self
            .flow
            .positions
            .get(position_index)
            .ok_or(FragmentError::UnknownFlowPosition)?;
        let terminal = self
            .flow
            .positions
            .len()
            .checked_sub(1)
            .ok_or(FragmentError::UnknownFlowPosition)?;
        let location = match self.flow.boundary_kinds.get(position_index) {
            Some(_) if position_index == terminal => CursorPosition::End,
            Some(FlowBoundaryKind::DocumentStart) => CursorPosition::DocumentStart,
            Some(FlowBoundaryKind::ParagraphItem) => {
                CursorPosition::ParagraphItem(position.owner_local_boundary())
            }
            Some(FlowBoundaryKind::End) => CursorPosition::End,
            Some(
                FlowBoundaryKind::TableRow
                | FlowBoundaryKind::ListItem
                | FlowBoundaryKind::BlockItem,
            ) => return Err(FragmentError::UnsupportedFlowDomain),
            None => return Err(FragmentError::UnknownFlowPosition),
        };
        FlowCursor::at(
            self.flow,
            u64::try_from(position_index).map_err(|_| FragmentError::ArithmeticOverflow)?,
            location,
        )
    }
}

impl Fragmenter for ReferenceFragmenter<'_> {
    fn fragment(
        &self,
        request: &FragmentRequest<'_>,
        budget: &mut dyn FragmentWorkBudget,
    ) -> Result<FragmentResult, FragmentError> {
        request.validate()?;
        if request.flow().epoch() != self.flow.epoch() {
            return Err(FragmentError::InvalidCursorEpoch);
        }
        if request.flow() != self.flow {
            return Err(FragmentError::InvalidFragmentKey);
        }
        let current = usize::try_from(request.cursor().position().global_flow_ordinal())
            .map_err(|_| FragmentError::UnknownFlowPosition)?;
        let terminal = self
            .flow
            .positions
            .len()
            .checked_sub(1)
            .ok_or(FragmentError::UnknownFlowPosition)?;

        match (
            self.flow.boundary_kinds.get(current),
            request.cursor().location(),
        ) {
            (Some(FlowBoundaryKind::DocumentStart), CursorPosition::DocumentStart)
                if current == terminal =>
            {
                return Ok(FragmentResult {
                    fragments: Vec::new(),
                    continuation: Continuation::Exhausted(Box::new(self.cursor_at(terminal)?)),
                    discovered_footnotes: Vec::new(),
                    discovered_anchors: Vec::new(),
                });
            }
            (Some(FlowBoundaryKind::DocumentStart), CursorPosition::DocumentStart) => {
                return Ok(FragmentResult {
                    fragments: Vec::new(),
                    continuation: Continuation::More(Box::new(self.cursor_at(current + 1)?)),
                    discovered_footnotes: Vec::new(),
                    discovered_anchors: Vec::new(),
                });
            }
            (Some(FlowBoundaryKind::ParagraphItem), CursorPosition::ParagraphItem(local))
                if *local == request.cursor().position().owner_local_boundary() => {}
            (Some(FlowBoundaryKind::End), CursorPosition::End) => {
                return Err(FragmentError::InvalidCursorLocation);
            }
            (Some(_), _) => return Err(FragmentError::InvalidCursorLocation),
            (None, _) => return Err(FragmentError::UnknownFlowPosition),
        }

        let first_line = self
            .lines
            .iter()
            .position(|line| line.start == current)
            .ok_or(FragmentError::InvalidCursorLocation)?;
        let available = request
            .frame()
            .height()
            .get()
            .raw()
            .checked_sub(request.reserved_footnote_height().get().raw())
            .ok_or(FragmentError::ArithmeticOverflow)?;
        let capacity = if self.legacy_full_frame {
            self.lines.len()
        } else {
            let mut occupied = 0i64;
            let mut count = 0usize;
            for line in &self.lines[first_line..] {
                let next = occupied
                    .checked_add(line.height.get().raw())
                    .ok_or(FragmentError::ArithmeticOverflow)?;
                if next > available {
                    break;
                }
                occupied = next;
                count = count
                    .checked_add(1)
                    .ok_or(FragmentError::ArithmeticOverflow)?;
            }
            count
        };
        if capacity == 0 {
            return Err(FragmentError::Unplaceable);
        }
        let fragment_count = (self.lines.len() - first_line).min(capacity);
        budget.consume_fragments(
            u64::try_from(fragment_count).map_err(|_| FragmentError::ArithmeticOverflow)?,
        )?;
        let mut fragments = Vec::with_capacity(fragment_count);
        let mut y_delta = 0i64;
        for line in &self.lines[first_line..first_line + fragment_count] {
            let y = request
                .frame()
                .y()
                .raw()
                .checked_add(y_delta)
                .and_then(Length::from_raw)
                .ok_or(FragmentError::ArithmeticOverflow)?;
            fragments.push(FragmentDraft::new(
                self.flow.positions[line.start].clone(),
                self.flow.positions[line.end].clone(),
                if self.legacy_full_frame {
                    request.frame()
                } else {
                    Rect::new(request.frame().x(), y, request.frame().width(), line.height)
                },
                0,
            )?);
            y_delta = y_delta
                .checked_add(line.height.get().raw())
                .ok_or(FragmentError::ArithmeticOverflow)?;
        }
        let current_ordinal =
            u64::try_from(current).map_err(|_| FragmentError::ArithmeticOverflow)?;
        let continuation_index = self.lines[first_line + fragment_count - 1].end;
        let continuation_ordinal =
            u64::try_from(continuation_index).map_err(|_| FragmentError::ArithmeticOverflow)?;
        let discovered_anchors = self
            .anchors
            .iter()
            .filter(|anchor| {
                anchor.flow_ordinal >= current_ordinal && anchor.flow_ordinal < continuation_ordinal
            })
            .map(|anchor| DiscoveredAnchor {
                anchor_id: anchor.anchor_id.clone(),
                owner_node: anchor.owner_node,
                position_in_frame: Point {
                    x: Length::ZERO,
                    y: Length::ZERO,
                },
            })
            .collect();
        Ok(FragmentResult {
            fragments,
            continuation: if continuation_index == terminal {
                Continuation::Exhausted(Box::new(self.cursor_at(terminal)?))
            } else {
                Continuation::More(Box::new(self.cursor_at(continuation_index)?))
            },
            discovered_footnotes: Vec::new(),
            discovered_anchors,
        })
    }
}

fn collect_reference_anchors(
    inlines: &[Inline],
    package: &ValidatedParsedPackage,
    flow: &FlowTree,
    flow_ordinal: u64,
    output: &mut Vec<ReferenceAnchorPlacement>,
) -> Result<(), FragmentError> {
    for inline in inlines {
        match inline {
            Inline::Anchor {
                node_id, anchor_id, ..
            } => {
                if package.document_nodes().anchor_owner(anchor_id) != Some(*node_id)
                    || flow.anchor_owner(anchor_id) != Some(*node_id)
                {
                    return Err(FragmentError::InvalidFragmentKey);
                }
                output.push(ReferenceAnchorPlacement {
                    flow_ordinal,
                    anchor_id: anchor_id.clone(),
                    owner_node: *node_id,
                });
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => {
                collect_reference_anchors(children, package, flow, flow_ordinal, output)?;
            }
            Inline::Text { .. }
            | Inline::Reference { .. }
            | Inline::FootnoteReference { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => {}
        }
    }
    Ok(())
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
    fn empty_paragraph_flow(package: &ValidatedParsedPackage) -> FlowTree {
        let package_epoch = epoch(package);
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(package, package_epoch).unwrap();
        let mut builder = CanonicalFlowIrBuilder::new(package, &paragraph_items).unwrap();
        for block in &package.package().document.blocks {
            let Block::Paragraph { node_id, .. } = block else {
                panic!("test package must contain only paragraphs");
            };
            builder.push_paragraph_item(*node_id, 0).unwrap();
        }
        builder.finish(package_epoch).unwrap()
    }
    fn page_context(
        package: &ValidatedParsedPackage,
        flow: &FlowTree,
        cursor: &FlowCursor,
    ) -> PageContext {
        PageContext::select(
            0,
            &ResolvedPageSelection::new(flow, cursor, package).unwrap(),
            &package.pagination_context(),
        )
        .unwrap()
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct CountingBudget {
        remaining_fragments: u64,
        consumed_fragments: u64,
        fragment_calls: u64,
    }
    impl CountingBudget {
        const fn new(remaining_fragments: u64) -> Self {
            Self {
                remaining_fragments,
                consumed_fragments: 0,
                fragment_calls: 0,
            }
        }
    }
    impl FragmentWorkBudget for CountingBudget {
        fn consume_fragments(&mut self, count: u64) -> Result<(), FragmentError> {
            self.fragment_calls = self
                .fragment_calls
                .checked_add(1)
                .ok_or(FragmentError::ArithmeticOverflow)?;
            let remaining = self
                .remaining_fragments
                .checked_sub(count)
                .ok_or(FragmentError::ResourceLimit)?;
            self.remaining_fragments = remaining;
            self.consumed_fragments = self
                .consumed_fragments
                .checked_add(count)
                .ok_or(FragmentError::ArithmeticOverflow)?;
            Ok(())
        }
        fn consume_footnote_reflow(&mut self, _page_index: u32) -> Result<(), FragmentError> {
            Err(FragmentError::UnsupportedFlowDomain)
        }
        fn consume_column_candidate(&mut self, _container: NodeId) -> Result<(), FragmentError> {
            Err(FragmentError::UnsupportedFlowDomain)
        }
        fn enqueue_float(
            &mut self,
            _owner: NodeId,
            _owner_local_ordinal: u32,
        ) -> Result<(), FragmentError> {
            Err(FragmentError::UnsupportedFlowDomain)
        }
        fn dequeue_float(
            &mut self,
            _owner: NodeId,
            _owner_local_ordinal: u32,
        ) -> Result<(), FragmentError> {
            Err(FragmentError::UnsupportedFlowDomain)
        }
        fn consume_float_carry(
            &mut self,
            _owner: NodeId,
            _owner_local_ordinal: u32,
        ) -> Result<(), FragmentError> {
            Err(FragmentError::UnsupportedFlowDomain)
        }
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

    #[test]
    fn reference_fragmenter_is_reentrant_and_deterministic() {
        let package = parsed_reference_package(17, "anchor:z\nparagraph\nanchor:a");
        let flow = empty_paragraph_flow(&package);
        let fragmenter = ReferenceFragmenter::for_empty_paragraphs(&package, &flow).unwrap();
        let start = FlowCursor::document_start(&flow);
        let request = FragmentRequest::new(
            &flow,
            &start,
            frame(),
            NonNegativeLength::ZERO,
            page_context(&package, &flow, &start),
        )
        .unwrap();

        let mut first_budget = CountingBudget::new(u64::MAX);
        let first = fragmenter.fragment(&request, &mut first_budget).unwrap();
        let mut repeated_budget = CountingBudget::new(u64::MAX);
        let repeated = fragmenter.fragment(&request, &mut repeated_budget).unwrap();
        assert_eq!(first, repeated);
        assert_eq!(first_budget.consumed_fragments, 0);
        assert_eq!(repeated_budget.consumed_fragments, 0);
        assert!(first.validate_progress(&request).is_ok());
        let next = match &first.continuation {
            Continuation::More(next) => next.as_ref().clone(),
            Continuation::Exhausted(_) => panic!("nonblank bootstrap must continue"),
        };
        assert_eq!(next.position(), &flow.positions()[1]);

        let continuation_request = FragmentRequest::new(
            &flow,
            &next,
            frame(),
            NonNegativeLength::ZERO,
            request.page().clone(),
        )
        .unwrap();
        let mut continuation_budget = CountingBudget::new(u64::MAX);
        let laid_out = fragmenter
            .fragment(&continuation_request, &mut continuation_budget)
            .unwrap();
        let mut repeated_continuation_budget = CountingBudget::new(u64::MAX);
        let repeated_laid_out = fragmenter
            .fragment(&continuation_request, &mut repeated_continuation_budget)
            .unwrap();
        assert_eq!(laid_out, repeated_laid_out);
        assert_eq!(continuation_budget.consumed_fragments, 3);
        assert_eq!(repeated_continuation_budget.consumed_fragments, 3);
        assert_eq!(laid_out.fragments.len(), 3);
        assert!(laid_out.validate_progress(&continuation_request).is_ok());
        for (index, fragment) in laid_out.fragments.iter().enumerate() {
            assert_eq!(fragment.start(), &flow.positions()[index + 1]);
            assert_eq!(fragment.end(), &flow.positions()[index + 2]);
            assert_eq!(fragment.bounds(), frame());
            assert_eq!(fragment.break_after_penalty(), 0);
        }
        assert_eq!(
            laid_out
                .discovered_anchors
                .iter()
                .map(|anchor| anchor.anchor_id.clone())
                .collect::<Vec<_>>(),
            vec![AnchorId::new("z").unwrap(), AnchorId::new("a").unwrap()]
        );
        for anchor in &laid_out.discovered_anchors {
            assert_eq!(
                package.document_nodes().anchor_owner(&anchor.anchor_id),
                Some(anchor.owner_node)
            );
            assert_eq!(
                flow.anchor_owner(&anchor.anchor_id),
                Some(anchor.owner_node)
            );
            assert_eq!(
                anchor.position_in_frame,
                Point {
                    x: Length::ZERO,
                    y: Length::ZERO,
                }
            );
        }
        assert_eq!(
            laid_out.continuation,
            Continuation::Exhausted(Box::new(flow.terminal_cursor()))
        );
        assert!(laid_out.discovered_footnotes.is_empty());
    }

    #[test]
    fn reference_fragmenter_honors_blank_and_terminal_semantics() {
        let package = validated_package(18);
        let flow = FlowTree::empty(&package, epoch(&package)).unwrap();
        let fragmenter = ReferenceFragmenter::for_empty_paragraphs(&package, &flow).unwrap();
        let start = FlowCursor::document_start(&flow);
        let request = FragmentRequest::new(
            &flow,
            &start,
            frame(),
            NonNegativeLength::ZERO,
            page_context(&package, &flow, &start),
        )
        .unwrap();
        let mut budget = CountingBudget::new(0);
        let result = fragmenter.fragment(&request, &mut budget).unwrap();
        assert!(result.fragments.is_empty());
        assert!(result.discovered_anchors.is_empty());
        assert_eq!(budget.consumed_fragments, 0);
        assert_eq!(
            result.continuation,
            Continuation::Exhausted(Box::new(flow.terminal_cursor()))
        );
        assert!(result.validate_progress(&request).is_ok());

        let terminal = flow.terminal_cursor();
        let terminal_request = FragmentRequest::new(
            &flow,
            &terminal,
            frame(),
            NonNegativeLength::ZERO,
            request.page().clone(),
        )
        .unwrap();
        assert_eq!(
            fragmenter.fragment(&terminal_request, &mut budget),
            Err(FragmentError::InvalidCursorLocation)
        );
        assert_eq!(budget.fragment_calls, 0);
    }

    #[test]
    fn reference_fragmenter_rejects_unsupported_content_and_budget_before_output() {
        let supported = paragraph_package(19);
        let flow = empty_paragraph_flow(&supported);
        let unsupported = parsed_reference_package(20, "paragraph\ntext:actual");
        assert!(matches!(
            ReferenceFragmenter::for_empty_paragraphs(&unsupported, &flow),
            Err(FragmentError::UnsupportedFlowDomain)
        ));

        let fragmenter = ReferenceFragmenter::for_empty_paragraphs(&supported, &flow).unwrap();
        let start = FlowCursor::document_start(&flow);
        let page = page_context(&supported, &flow, &start);
        let bootstrap_request = FragmentRequest::new(
            &flow,
            &start,
            frame(),
            NonNegativeLength::ZERO,
            page.clone(),
        )
        .unwrap();
        let mut bootstrap_budget = CountingBudget::new(0);
        let bootstrap = fragmenter
            .fragment(&bootstrap_request, &mut bootstrap_budget)
            .unwrap();
        let next = match bootstrap.continuation {
            Continuation::More(next) => *next,
            Continuation::Exhausted(_) => panic!("nonblank bootstrap must continue"),
        };
        let request =
            FragmentRequest::new(&flow, &next, frame(), NonNegativeLength::ZERO, page).unwrap();
        let mut insufficient = CountingBudget::new(1);
        assert_eq!(
            fragmenter.fragment(&request, &mut insufficient),
            Err(FragmentError::ResourceLimit)
        );
        assert_eq!(insufficient.fragment_calls, 1);
        assert_eq!(insufficient.consumed_fragments, 0);
    }
}
