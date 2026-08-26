use crate::{WireBlock, WireDocumentPackage, WireInline};
use std::cmp::Ordering;
use std::fmt;
use typaxis_core::JsonPointer;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentPackageRootMember {
    Contract,
    CoordinateUnit,
    Sources,
    TextBuffers,
    Document,
    StyleSheet,
    PageMasters,
    Resources,
}

impl DocumentPackageRootMember {
    const fn segment(self) -> &'static str {
        match self {
            Self::Contract => "contract",
            Self::CoordinateUnit => "coordinate_unit",
            Self::Sources => "sources",
            Self::TextBuffers => "text_buffers",
            Self::Document => "document",
            Self::StyleSheet => "style_sheet",
            Self::PageMasters => "page_masters",
            Self::Resources => "resources",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum JsonLocationIndexAxis {
    PackageBytes,
    Sources,
    TextBuffers,
    AstNodes,
    StyleRules,
    PageMasters,
    Fonts,
    Images,
    PlatformAddressSpace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct JsonLocationIndexBuildError {
    pub(crate) axis: JsonLocationIndexAxis,
    pub(crate) limit: u64,
    pub(crate) attempted: u64,
}

impl fmt::Display for JsonLocationIndexBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "JSON location index {:?} budget {} was exceeded by {}",
            self.axis, self.limit, self.attempted
        )
    }
}

impl std::error::Error for JsonLocationIndexBuildError {}

#[derive(Clone, Copy, Debug)]
pub(crate) struct JsonLocationIndexBudget {
    pub(crate) package_bytes: u64,
    pub(crate) sources: u64,
    pub(crate) text_buffers: u64,
    pub(crate) ast_nodes: u64,
    pub(crate) style_rules: u64,
    pub(crate) page_masters: u64,
    pub(crate) fonts: u64,
    pub(crate) images: u64,
    pub(crate) observed_ast_nodes: u64,
}

#[derive(Clone, Debug)]
struct IndexedOrdinal<K> {
    key: K,
    occurrence: usize,
    ordinal: usize,
    child_count: usize,
    path: Option<usize>,
}

#[derive(Clone, Debug)]
struct SortedOrdinalIndex<K> {
    entries: Vec<IndexedOrdinal<K>>,
    order: Vec<usize>,
}

impl<K: Ord> SortedOrdinalIndex<K> {
    fn new(
        mut entries: Vec<IndexedOrdinal<K>>,
        limit: u64,
        axis: JsonLocationIndexAxis,
    ) -> Result<Self, JsonLocationIndexBuildError> {
        let count = to_u64(entries.len());
        if count > limit {
            return Err(JsonLocationIndexBuildError {
                axis,
                limit,
                attempted: count,
            });
        }

        // Both the stable order and its merge scratch are charged and reserved
        // before either is populated. The entry vector remains in ordinal
        // order; the sorted permutation is the bounded ID index.
        let mut order = Vec::new();
        try_reserve(&mut order, entries.len(), axis, limit, count)?;
        order.extend(0..entries.len());
        let mut scratch = Vec::new();
        try_reserve(&mut scratch, entries.len(), axis, limit, count)?;
        scratch.resize(entries.len(), 0usize);
        stable_merge_sort(&entries, &mut order, &mut scratch);

        let mut previous: Option<usize> = None;
        let mut occurrence = 0usize;
        for &entry_index in &order {
            if previous.is_some_and(|old| entries[old].key == entries[entry_index].key) {
                occurrence = occurrence
                    .checked_add(1)
                    .ok_or(JsonLocationIndexBuildError {
                        axis,
                        limit,
                        attempted: u64::MAX,
                    })?;
            } else {
                occurrence = 0;
            }
            entries[entry_index].occurrence = occurrence;
            previous = Some(entry_index);
        }

        Ok(Self { entries, order })
    }

    fn get(&self, key: &K, occurrence: usize) -> Option<&IndexedOrdinal<K>> {
        let mut low = 0usize;
        let mut high = self.order.len();
        while low < high {
            let middle = low + (high - low) / 2;
            let entry = &self.entries[self.order[middle]];
            match entry
                .key
                .cmp(key)
                .then_with(|| entry.occurrence.cmp(&occurrence))
            {
                Ordering::Less => low = middle + 1,
                Ordering::Equal => return Some(entry),
                Ordering::Greater => high = middle,
            }
        }
        None
    }
}

impl SortedOrdinalIndex<String> {
    fn get_str(&self, key: &str, occurrence: usize) -> Option<&IndexedOrdinal<String>> {
        let mut low = 0usize;
        let mut high = self.order.len();
        while low < high {
            let middle = low + (high - low) / 2;
            let entry = &self.entries[self.order[middle]];
            match entry
                .key
                .as_str()
                .cmp(key)
                .then_with(|| entry.occurrence.cmp(&occurrence))
            {
                Ordering::Less => low = middle + 1,
                Ordering::Equal => return Some(entry),
                Ordering::Greater => high = middle,
            }
        }
        None
    }
}

fn stable_merge_sort<K: Ord>(
    entries: &[IndexedOrdinal<K>],
    order: &mut Vec<usize>,
    scratch: &mut Vec<usize>,
) {
    let length = order.len();
    let mut width = 1usize;
    while width < length {
        let mut start = 0usize;
        while start < length {
            let middle = start.saturating_add(width).min(length);
            let end = middle.saturating_add(width).min(length);
            let (mut left, mut right, mut output) = (start, middle, start);
            while left < middle && right < end {
                // Taking the left entry on equality is the stability rule.
                if entries[order[left]].key <= entries[order[right]].key {
                    scratch[output] = order[left];
                    left += 1;
                } else {
                    scratch[output] = order[right];
                    right += 1;
                }
                output += 1;
            }
            while left < middle {
                scratch[output] = order[left];
                left += 1;
                output += 1;
            }
            while right < end {
                scratch[output] = order[right];
                right += 1;
                output += 1;
            }
            start = end;
        }
        std::mem::swap(order, scratch);
        width = width.saturating_mul(2);
    }
}

#[derive(Clone, Copy, Debug)]
enum FixedPathSegment {
    Document,
    Blocks,
    Footnotes,
    Children,
    Items,
    Head,
    Body,
    Caption,
    Cells,
}

impl FixedPathSegment {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Blocks => "blocks",
            Self::Footnotes => "footnotes",
            Self::Children => "children",
            Self::Items => "items",
            Self::Head => "head",
            Self::Body => "body",
            Self::Caption => "caption",
            Self::Cells => "cells",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum PathSegment {
    Fixed(FixedPathSegment),
    Index(usize),
}

impl PathSegment {
    fn encoded_bytes(self) -> u64 {
        1 + match self {
            Self::Fixed(segment) => to_u64(segment.as_str().len()),
            Self::Index(index) => decimal_digits(index),
        }
    }

    fn push(self, pointer: &mut JsonPointer) {
        match self {
            Self::Fixed(segment) => pointer.push_segment(segment.as_str()),
            Self::Index(index) => pointer.push_segment(&index.to_string()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PathNode {
    parent: Option<usize>,
    segment: PathSegment,
    encoded_bytes: u64,
    depth: u16,
}

#[derive(Clone, Debug)]
struct PathArena {
    nodes: Vec<PathNode>,
    maximum_nodes: usize,
    maximum_pointer_bytes: u64,
}

impl PathArena {
    fn new(
        observed_nodes: u64,
        maximum_pointer_bytes: u64,
    ) -> Result<Self, JsonLocationIndexBuildError> {
        let maximum_nodes_u64 = observed_nodes
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(JsonLocationIndexBuildError {
                axis: JsonLocationIndexAxis::AstNodes,
                limit: observed_nodes,
                attempted: u64::MAX,
            })?;
        let maximum_nodes =
            usize::try_from(maximum_nodes_u64).map_err(|_| JsonLocationIndexBuildError {
                axis: JsonLocationIndexAxis::PlatformAddressSpace,
                limit: usize::MAX as u64,
                attempted: maximum_nodes_u64,
            })?;
        let mut nodes = Vec::new();
        try_reserve(
            &mut nodes,
            maximum_nodes,
            JsonLocationIndexAxis::AstNodes,
            observed_nodes,
            observed_nodes,
        )?;
        Ok(Self {
            nodes,
            maximum_nodes,
            maximum_pointer_bytes,
        })
    }

    fn child(
        &mut self,
        parent: Option<usize>,
        segment: PathSegment,
    ) -> Result<usize, JsonLocationIndexBuildError> {
        let attempted_nodes =
            self.nodes
                .len()
                .checked_add(1)
                .ok_or(JsonLocationIndexBuildError {
                    axis: JsonLocationIndexAxis::AstNodes,
                    limit: to_u64(self.maximum_nodes),
                    attempted: u64::MAX,
                })?;
        if attempted_nodes > self.maximum_nodes {
            return Err(JsonLocationIndexBuildError {
                axis: JsonLocationIndexAxis::AstNodes,
                limit: to_u64(self.maximum_nodes),
                attempted: to_u64(attempted_nodes),
            });
        }
        let (parent_bytes, parent_depth) = parent
            .map(|index| {
                let node = self.nodes[index];
                (node.encoded_bytes, node.depth)
            })
            .unwrap_or((0, 0));
        let encoded_bytes = parent_bytes.checked_add(segment.encoded_bytes()).ok_or(
            JsonLocationIndexBuildError {
                axis: JsonLocationIndexAxis::PackageBytes,
                limit: self.maximum_pointer_bytes,
                attempted: u64::MAX,
            },
        )?;
        if encoded_bytes > self.maximum_pointer_bytes {
            return Err(JsonLocationIndexBuildError {
                axis: JsonLocationIndexAxis::PackageBytes,
                limit: self.maximum_pointer_bytes,
                attempted: encoded_bytes,
            });
        }
        let depth = parent_depth
            .checked_add(1)
            .ok_or(JsonLocationIndexBuildError {
                axis: JsonLocationIndexAxis::AstNodes,
                limit: u64::from(u16::MAX),
                attempted: u64::from(u16::MAX) + 1,
            })?;
        let index = self.nodes.len();
        self.nodes.push(PathNode {
            parent,
            segment,
            encoded_bytes,
            depth,
        });
        Ok(index)
    }

    fn materialize(&self, terminal: usize) -> JsonPointer {
        const MAX_PATH_SEGMENTS: usize = 256;
        let mut segments = [PathSegment::Fixed(FixedPathSegment::Document); MAX_PATH_SEGMENTS];
        let depth = usize::from(self.nodes[terminal].depth);
        debug_assert!(depth <= MAX_PATH_SEGMENTS);
        let mut position = depth;
        let mut current = Some(terminal);
        while let Some(index) = current {
            position -= 1;
            segments[position] = self.nodes[index].segment;
            current = self.nodes[index].parent;
        }
        let mut pointer = JsonPointer::root();
        for segment in &segments[..depth] {
            segment.push(&mut pointer);
        }
        debug_assert!(to_u64(pointer.as_str().len()) <= self.maximum_pointer_bytes);
        pointer
    }
}

/// Bounded, ordinal-backed mapping from wire entities to canonical JSON Pointers.
///
/// ID values are lookup keys only. They are never converted into allocation
/// lengths. Duplicate IDs are addressed by zero-based occurrence, so occurrence
/// `1` is always the second declaration and therefore its primary location.
#[derive(Clone, Debug)]
pub struct JsonLocationIndex {
    package_bytes: u64,
    sources: SortedOrdinalIndex<u32>,
    text_buffers: SortedOrdinalIndex<u32>,
    nodes: SortedOrdinalIndex<u32>,
    styles: SortedOrdinalIndex<String>,
    style_source_orders: SortedOrdinalIndex<u32>,
    masters: SortedOrdinalIndex<String>,
    master_rule_source_orders: SortedOrdinalIndex<u32>,
    fonts: SortedOrdinalIndex<u32>,
    images: SortedOrdinalIndex<u32>,
    selection_rule_count: usize,
    paths: PathArena,
}

impl JsonLocationIndex {
    pub(crate) fn build(
        package: &WireDocumentPackage,
        budget: JsonLocationIndexBudget,
    ) -> Result<Self, JsonLocationIndexBuildError> {
        let sources = flat_index(
            &package.sources,
            budget.sources,
            JsonLocationIndexAxis::Sources,
            |source| source.source_id,
            |_| 0,
        )?;
        let text_buffers = flat_index(
            &package.text_buffers,
            budget.text_buffers,
            JsonLocationIndexAxis::TextBuffers,
            |buffer| buffer.text_id,
            |buffer| buffer.mappings.len(),
        )?;
        let styles = flat_index(
            &package.style_sheet.rules,
            budget.style_rules,
            JsonLocationIndexAxis::StyleRules,
            |rule| rule.style_id.clone(),
            |rule| rule.declarations.len(),
        )?;
        let style_source_orders = flat_index(
            &package.style_sheet.rules,
            budget.style_rules,
            JsonLocationIndexAxis::StyleRules,
            |rule| rule.source_order,
            |rule| rule.declarations.len(),
        )?;
        let masters = flat_index(
            &package.page_masters.masters,
            budget.page_masters,
            JsonLocationIndexAxis::PageMasters,
            |master| master.master_id.clone(),
            |_| 0,
        )?;
        let master_rule_source_orders = flat_index(
            &package.page_masters.selection_rules,
            budget.page_masters,
            JsonLocationIndexAxis::PageMasters,
            |rule| rule.source_order,
            |_| 0,
        )?;
        let fonts = flat_index(
            &package.resources.font_faces,
            budget.fonts,
            JsonLocationIndexAxis::Fonts,
            |font| font.font_face_id,
            |_| 0,
        )?;
        let images = flat_index(
            &package.resources.images,
            budget.images,
            JsonLocationIndexAxis::Images,
            |image| image.image_id,
            |_| 0,
        )?;

        let mut paths = PathArena::new(budget.observed_ast_nodes, budget.package_bytes)?;
        let node_capacity = usize::try_from(budget.observed_ast_nodes).map_err(|_| {
            JsonLocationIndexBuildError {
                axis: JsonLocationIndexAxis::PlatformAddressSpace,
                limit: usize::MAX as u64,
                attempted: budget.observed_ast_nodes,
            }
        })?;
        let mut node_entries = Vec::new();
        try_reserve(
            &mut node_entries,
            node_capacity,
            JsonLocationIndexAxis::AstNodes,
            budget.ast_nodes,
            budget.observed_ast_nodes,
        )?;
        let document_path = paths.child(None, PathSegment::Fixed(FixedPathSegment::Document))?;
        index_document(package, document_path, &mut paths, &mut node_entries)?;
        if to_u64(node_entries.len()) != budget.observed_ast_nodes {
            return Err(JsonLocationIndexBuildError {
                axis: JsonLocationIndexAxis::AstNodes,
                limit: budget.observed_ast_nodes,
                attempted: to_u64(node_entries.len()),
            });
        }
        let nodes = SortedOrdinalIndex::new(
            node_entries,
            budget.ast_nodes,
            JsonLocationIndexAxis::AstNodes,
        )?;

        let index = Self {
            package_bytes: budget.package_bytes,
            sources,
            text_buffers,
            nodes,
            styles,
            style_source_orders,
            masters,
            master_rule_source_orders,
            fonts,
            images,
            selection_rule_count: package.page_masters.selection_rules.len(),
            paths,
        };
        index.check_fixed_pointers()?;
        Ok(index)
    }

    pub fn root_member(&self, member: DocumentPackageRootMember) -> JsonPointer {
        JsonPointer::root().child(member.segment())
    }

    pub fn source(&self, source_id: u32, occurrence: usize) -> Option<JsonPointer> {
        self.sources
            .get(&source_id, occurrence)
            .map(|entry| fixed_array_pointer("sources", entry.ordinal))
    }

    pub fn text_buffer(&self, text_id: u32, occurrence: usize) -> Option<JsonPointer> {
        self.text_buffers
            .get(&text_id, occurrence)
            .map(|entry| fixed_array_pointer("text_buffers", entry.ordinal))
    }

    pub fn text_mapping(
        &self,
        text_id: u32,
        occurrence: usize,
        mapping_ordinal: usize,
    ) -> Option<JsonPointer> {
        let entry = self.text_buffers.get(&text_id, occurrence)?;
        (mapping_ordinal < entry.child_count).then(|| {
            fixed_array_pointer("text_buffers", entry.ordinal)
                .child("mappings")
                .child(&mapping_ordinal.to_string())
        })
    }

    pub fn node(&self, node_id: u32, occurrence: usize) -> Option<JsonPointer> {
        let entry = self.nodes.get(&node_id, occurrence)?;
        entry.path.map(|path| self.paths.materialize(path))
    }

    pub fn style_rule(&self, style_id: &str, occurrence: usize) -> Option<JsonPointer> {
        self.styles
            .get_str(style_id, occurrence)
            .map(|entry| style_rule_pointer(entry.ordinal))
    }

    pub fn style_rule_by_source_order(
        &self,
        source_order: u32,
        occurrence: usize,
    ) -> Option<JsonPointer> {
        self.style_source_orders
            .get(&source_order, occurrence)
            .map(|entry| style_rule_pointer(entry.ordinal))
    }

    pub fn style_declaration(
        &self,
        style_id: &str,
        occurrence: usize,
        declaration_ordinal: usize,
    ) -> Option<JsonPointer> {
        let entry = self.styles.get_str(style_id, occurrence)?;
        (declaration_ordinal < entry.child_count).then(|| {
            style_rule_pointer(entry.ordinal)
                .child("declarations")
                .child(&declaration_ordinal.to_string())
        })
    }

    pub fn page_master(&self, master_id: &str, occurrence: usize) -> Option<JsonPointer> {
        self.masters
            .get_str(master_id, occurrence)
            .map(|entry| page_master_pointer(entry.ordinal))
    }

    pub fn page_master_selection_rule(&self, ordinal: usize) -> Option<JsonPointer> {
        (ordinal < self.selection_rule_count).then(|| {
            JsonPointer::from_segments([
                "page_masters".to_owned(),
                "selection_rules".to_owned(),
                ordinal.to_string(),
            ])
        })
    }

    pub fn page_master_rule_by_source_order(
        &self,
        source_order: u32,
        occurrence: usize,
    ) -> Option<JsonPointer> {
        let entry = self
            .master_rule_source_orders
            .get(&source_order, occurrence)?;
        self.page_master_selection_rule(entry.ordinal)
    }

    pub fn font_face(&self, font_face_id: u32, occurrence: usize) -> Option<JsonPointer> {
        self.fonts
            .get(&font_face_id, occurrence)
            .map(|entry| resource_pointer("font_faces", entry.ordinal))
    }

    pub fn image(&self, image_id: u32, occurrence: usize) -> Option<JsonPointer> {
        self.images
            .get(&image_id, occurrence)
            .map(|entry| resource_pointer("images", entry.ordinal))
    }

    fn check_fixed_pointers(&self) -> Result<(), JsonLocationIndexBuildError> {
        let mut attempted = [
            self.sources
                .entries
                .last()
                .map(|entry| fixed_pointer_bytes("sources", entry.ordinal)),
            self.text_buffers
                .entries
                .last()
                .map(|entry| fixed_pointer_bytes("text_buffers", entry.ordinal)),
            self.styles
                .entries
                .last()
                .map(|entry| fixed_pointer_bytes("style_sheet/rules", entry.ordinal)),
            self.masters
                .entries
                .last()
                .map(|entry| fixed_pointer_bytes("page_masters/masters", entry.ordinal)),
            self.fonts
                .entries
                .last()
                .map(|entry| fixed_pointer_bytes("resources/font_faces", entry.ordinal)),
            self.images
                .entries
                .last()
                .map(|entry| fixed_pointer_bytes("resources/images", entry.ordinal)),
            self.selection_rule_count
                .checked_sub(1)
                .map(|ordinal| fixed_pointer_bytes("page_masters/selection_rules", ordinal)),
        ]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(0);
        for entry in &self.text_buffers.entries {
            if let Some(mapping) = entry.child_count.checked_sub(1) {
                attempted = attempted.max(
                    fixed_pointer_bytes("text_buffers", entry.ordinal)
                        + to_u64("/mappings/".len())
                        + decimal_digits(mapping),
                );
            }
        }
        for entry in &self.styles.entries {
            if let Some(declaration) = entry.child_count.checked_sub(1) {
                attempted = attempted.max(
                    fixed_pointer_bytes("style_sheet/rules", entry.ordinal)
                        + to_u64("/declarations/".len())
                        + decimal_digits(declaration),
                );
            }
        }
        if attempted > self.package_bytes {
            Err(JsonLocationIndexBuildError {
                axis: JsonLocationIndexAxis::PackageBytes,
                limit: self.package_bytes,
                attempted,
            })
        } else {
            Ok(())
        }
    }
}

fn flat_index<T, K: Ord, F, C>(
    values: &[T],
    limit: u64,
    axis: JsonLocationIndexAxis,
    mut key: F,
    mut child_count: C,
) -> Result<SortedOrdinalIndex<K>, JsonLocationIndexBuildError>
where
    F: FnMut(&T) -> K,
    C: FnMut(&T) -> usize,
{
    let count = to_u64(values.len());
    if count > limit {
        return Err(JsonLocationIndexBuildError {
            axis,
            limit,
            attempted: count,
        });
    }
    let mut entries = Vec::new();
    try_reserve(&mut entries, values.len(), axis, limit, count)?;
    for (ordinal, value) in values.iter().enumerate() {
        entries.push(IndexedOrdinal {
            key: key(value),
            occurrence: 0,
            ordinal,
            child_count: child_count(value),
            path: None,
        });
    }
    SortedOrdinalIndex::new(entries, limit, axis)
}

fn index_document(
    package: &WireDocumentPackage,
    path: usize,
    paths: &mut PathArena,
    entries: &mut Vec<IndexedOrdinal<u32>>,
) -> Result<(), JsonLocationIndexBuildError> {
    add_node(entries, package.document.node_id, path);
    index_blocks(
        &package.document.blocks,
        path,
        FixedPathSegment::Blocks,
        paths,
        entries,
    )?;
    if !package.document.footnotes.is_empty() {
        let collection =
            paths.child(path.into(), PathSegment::Fixed(FixedPathSegment::Footnotes))?;
        for (ordinal, footnote) in package.document.footnotes.iter().enumerate() {
            let item = paths.child(collection.into(), PathSegment::Index(ordinal))?;
            add_node(entries, footnote.node_id, item);
            index_blocks(
                &footnote.blocks,
                item,
                FixedPathSegment::Blocks,
                paths,
                entries,
            )?;
        }
    }
    Ok(())
}

fn index_blocks(
    blocks: &[WireBlock],
    parent: usize,
    segment: FixedPathSegment,
    paths: &mut PathArena,
    entries: &mut Vec<IndexedOrdinal<u32>>,
) -> Result<(), JsonLocationIndexBuildError> {
    if blocks.is_empty() {
        return Ok(());
    }
    let collection = paths.child(parent.into(), PathSegment::Fixed(segment))?;
    for (ordinal, block) in blocks.iter().enumerate() {
        let item = paths.child(collection.into(), PathSegment::Index(ordinal))?;
        index_block(block, item, paths, entries)?;
    }
    Ok(())
}

fn index_block(
    block: &WireBlock,
    path: usize,
    paths: &mut PathArena,
    entries: &mut Vec<IndexedOrdinal<u32>>,
) -> Result<(), JsonLocationIndexBuildError> {
    let node_id = match block {
        WireBlock::Paragraph { node_id, .. }
        | WireBlock::Heading { node_id, .. }
        | WireBlock::List { node_id, .. }
        | WireBlock::Table { node_id, .. }
        | WireBlock::Figure { node_id, .. }
        | WireBlock::PageBreak { node_id, .. } => *node_id,
    };
    add_node(entries, node_id, path);
    match block {
        WireBlock::Paragraph { children, .. } | WireBlock::Heading { children, .. } => {
            index_inlines(children, path, paths, entries)?;
        }
        WireBlock::List { items, .. } => {
            if !items.is_empty() {
                let collection =
                    paths.child(path.into(), PathSegment::Fixed(FixedPathSegment::Items))?;
                for (ordinal, item) in items.iter().enumerate() {
                    let item_path = paths.child(collection.into(), PathSegment::Index(ordinal))?;
                    add_node(entries, item.node_id, item_path);
                    index_blocks(
                        &item.blocks,
                        item_path,
                        FixedPathSegment::Blocks,
                        paths,
                        entries,
                    )?;
                }
            }
        }
        WireBlock::Table { head, body, .. } => {
            index_rows(head, path, FixedPathSegment::Head, paths, entries)?;
            index_rows(body, path, FixedPathSegment::Body, paths, entries)?;
        }
        WireBlock::Figure { caption, .. } => {
            index_blocks(caption, path, FixedPathSegment::Caption, paths, entries)?;
        }
        WireBlock::PageBreak { .. } => {}
    }
    Ok(())
}

fn index_inlines(
    inlines: &[WireInline],
    parent: usize,
    paths: &mut PathArena,
    entries: &mut Vec<IndexedOrdinal<u32>>,
) -> Result<(), JsonLocationIndexBuildError> {
    if inlines.is_empty() {
        return Ok(());
    }
    let collection = paths.child(
        parent.into(),
        PathSegment::Fixed(FixedPathSegment::Children),
    )?;
    for (ordinal, inline) in inlines.iter().enumerate() {
        let item = paths.child(collection.into(), PathSegment::Index(ordinal))?;
        let node_id = match inline {
            WireInline::Text { node_id, .. }
            | WireInline::Emphasis { node_id, .. }
            | WireInline::Strong { node_id, .. }
            | WireInline::Link { node_id, .. }
            | WireInline::Anchor { node_id, .. }
            | WireInline::Reference { node_id, .. }
            | WireInline::FootnoteReference { node_id, .. }
            | WireInline::SoftBreak { node_id, .. }
            | WireInline::HardBreak { node_id, .. } => *node_id,
        };
        add_node(entries, node_id, item);
        match inline {
            WireInline::Emphasis { children, .. }
            | WireInline::Strong { children, .. }
            | WireInline::Link { children, .. } => {
                index_inlines(children, item, paths, entries)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn index_rows(
    rows: &[crate::WireTableRow],
    parent: usize,
    segment: FixedPathSegment,
    paths: &mut PathArena,
    entries: &mut Vec<IndexedOrdinal<u32>>,
) -> Result<(), JsonLocationIndexBuildError> {
    if rows.is_empty() {
        return Ok(());
    }
    let collection = paths.child(parent.into(), PathSegment::Fixed(segment))?;
    for (ordinal, row) in rows.iter().enumerate() {
        let row_path = paths.child(collection.into(), PathSegment::Index(ordinal))?;
        add_node(entries, row.node_id, row_path);
        if !row.cells.is_empty() {
            let cells =
                paths.child(row_path.into(), PathSegment::Fixed(FixedPathSegment::Cells))?;
            for (cell_ordinal, cell) in row.cells.iter().enumerate() {
                let cell_path = paths.child(cells.into(), PathSegment::Index(cell_ordinal))?;
                add_node(entries, cell.node_id, cell_path);
                index_blocks(
                    &cell.blocks,
                    cell_path,
                    FixedPathSegment::Blocks,
                    paths,
                    entries,
                )?;
            }
        }
    }
    Ok(())
}

fn add_node(entries: &mut Vec<IndexedOrdinal<u32>>, node_id: u32, path: usize) {
    let ordinal = entries.len();
    entries.push(IndexedOrdinal {
        key: node_id,
        occurrence: 0,
        ordinal,
        child_count: 0,
        path: Some(path),
    });
}

fn fixed_array_pointer(member: &str, ordinal: usize) -> JsonPointer {
    JsonPointer::root()
        .child(member)
        .child(&ordinal.to_string())
}

fn style_rule_pointer(ordinal: usize) -> JsonPointer {
    JsonPointer::from_segments([
        "style_sheet".to_owned(),
        "rules".to_owned(),
        ordinal.to_string(),
    ])
}

fn page_master_pointer(ordinal: usize) -> JsonPointer {
    JsonPointer::from_segments([
        "page_masters".to_owned(),
        "masters".to_owned(),
        ordinal.to_string(),
    ])
}

fn resource_pointer(member: &str, ordinal: usize) -> JsonPointer {
    JsonPointer::from_segments([
        "resources".to_owned(),
        member.to_owned(),
        ordinal.to_string(),
    ])
}

fn fixed_pointer_bytes(prefix: &str, ordinal: usize) -> u64 {
    2 + to_u64(prefix.len()) + decimal_digits(ordinal)
}

fn decimal_digits(mut value: usize) -> u64 {
    let mut digits = 1u64;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}

fn try_reserve<T>(
    values: &mut Vec<T>,
    additional: usize,
    axis: JsonLocationIndexAxis,
    limit: u64,
    attempted: u64,
) -> Result<(), JsonLocationIndexBuildError> {
    values
        .try_reserve_exact(additional)
        .map_err(|_| JsonLocationIndexBuildError {
            axis,
            limit,
            attempted,
        })
}

fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
