//! Exact source hierarchy and actual page/MCID ownership. Footnote relocation,
//! annotation relations, table Headers and final StructElem encoding are later
//! selected-structure work; this source registry is not a PDF/UA receipt.
use super::*;
use typaxis_core::NodeId;
use typaxis_syntax::book_v2::{
    visit_book_v2_structure, BookV2StructureKey as Key, BookV2StructureSlot as Slot,
    BookV2StructureSourceError, BookV2StructureSourceNode as SourceNode, BookV2StructureVisitor,
};

pub const BOOK_V2_SOURCE_STRUCTURE_ALGORITHM: &str = "typaxis.book-2-source-structure/1";
impl From<BookV2StructureSourceError> for E {
    fn from(error: BookV2StructureSourceError) -> Self {
        match error {
            BookV2StructureSourceError::Nodes => E::Records,
            BookV2StructureSourceError::Depth => E::Depth,
            BookV2StructureSourceError::Identity => E::Identity,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct BookV2SourceStructureNode<'a> {
    source: SourceNode<'a>,
    parent: Option<usize>,
    depth: u32,
    first_child: Option<usize>,
    last_child: Option<usize>,
    next_sibling: Option<usize>,
    first_binding: Option<usize>,
    last_binding: Option<usize>,
}
impl<'a> BookV2SourceStructureNode<'a> {
    pub fn source(&self) -> &SourceNode<'a> {
        &self.source
    }
    pub fn parent(&self) -> Option<usize> {
        self.parent
    }
    pub fn depth(&self) -> u32 {
        self.depth
    }
    pub fn first_child(&self) -> Option<usize> {
        self.first_child
    }
    pub fn next_sibling(&self) -> Option<usize> {
        self.next_sibling
    }
    pub fn first_binding(&self) -> Option<usize> {
        self.first_binding
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2StructureContentBinding {
    node: usize,
    group: usize,
    page: u32,
    mcid: u32,
    next: Option<usize>,
}
impl BookV2StructureContentBinding {
    pub fn node_index(&self) -> usize {
        self.node
    }
    pub fn group_index(&self) -> usize {
        self.group
    }
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn mcid(&self) -> u32 {
        self.mcid
    }
    pub fn next_for_node(&self) -> Option<usize> {
        self.next
    }
}
pub struct BookV2SourceStructure<
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> {
    source: &'h BookV2MarkedContent<
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >,
    nodes: Vec<BookV2SourceStructureNode<'h>>,
    lookup: Vec<(Key, usize)>,
    bindings: Vec<BookV2StructureContentBinding>,
    groups: Vec<Option<usize>>,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2SourceStructure<
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >
{
    pub fn source(
        &self,
    ) -> &'h BookV2MarkedContent<
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    > {
        self.source
    }
    pub fn nodes(&self) -> &[BookV2SourceStructureNode<'h>] {
        &self.nodes
    }
    pub fn node_index(&self, key: Key) -> Option<usize> {
        self.lookup
            .binary_search_by_key(&key, |r| r.0)
            .ok()
            .map(|i| self.lookup[i].1)
    }
    pub fn bindings(&self) -> &[BookV2StructureContentBinding] {
        &self.bindings
    }
    pub fn for_group(&self, group: usize) -> Option<&BookV2StructureContentBinding> {
        self.bindings.get(*self.groups.get(group)?.as_ref()?)
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn record_charge(&self) -> u64 {
        self.budget.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.budget.spool
    }
    pub fn output_charge(&self) -> u64 {
        self.budget.output
    }
    pub fn work_steps(&self) -> u64 {
        self.budget.work
    }
}
pub struct BookV2SourceStructureBuilder<
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> {
    source: &'h BookV2MarkedContent<
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >,
    budget: Budget,
}
impl<'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2SourceStructureBuilder<
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >
{
    pub fn new(
        source: &'h BookV2MarkedContent<
            'n,
            'm,
            'k,
            'j,
            'r,
            'i,
            't,
            'o,
            'z,
            'y,
            'x,
            'c,
            'v,
            'd,
            'g,
            'q,
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        let display = source.source().display();
        display
            .verify_resources(display.admitted(), limits)
            .map_err(|_| E::Identity)?;
        let base = limits.base().get();
        // Inherit the completed body budget: earlier raw payload credits have
        // already been consumed and must not be resurrected by a new builder.
        let mut budget = Budget {
            max_records: base.max_fragments,
            max_spool: base.max_spool_bytes,
            max_output: base.max_output_bytes,
            max_work,
            records: prior_records.max(source.record_charge()),
            spool: prior_spool.max(source.spool_charge()),
            output: prior_output.max(source.output_charge()),
            work: prior_work.max(source.work_steps()),
        };
        budget.reserve(1, 0, 0)?;
        if budget.work > max_work {
            return Err(E::Work);
        }
        Ok(Self { source, budget })
    }

    pub fn record_charge(&self) -> u64 {
        self.budget.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.budget.spool
    }
    pub fn output_charge(&self) -> u64 {
        self.budget.output
    }
    pub fn work_steps(&self) -> u64 {
        self.budget.work
    }
    pub fn build(
        &mut self,
    ) -> Result<
        BookV2SourceStructure<
            'h,
            'n,
            'm,
            'k,
            'j,
            'r,
            'i,
            't,
            'o,
            'z,
            'y,
            'x,
            'c,
            'v,
            'd,
            'g,
            'q,
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        E,
    > {
        let flow = self
            .source
            .source()
            .display()
            .source()
            .source()
            .flow()
            .lines()
            .prepared()
            .source_flow();
        let mut counter = Collect {
            budget: &mut self.budget,
            count: 0,
            nodes: None,
        };
        visit_book_v2_structure(flow, &mut counter)?;
        let count = counter.count;
        let scopes = self.source.source().groups();
        let mut semantic_count = 0usize;
        for group in scopes {
            self.budget.step(1)?;
            if group.mcid().is_some() {
                semantic_count = semantic_count.checked_add(1).ok_or(E::Records)?;
            }
        }
        let bytes = count
            .checked_mul(
                std::mem::size_of::<BookV2SourceStructureNode<'_>>()
                    + std::mem::size_of::<(Key, usize)>(),
            )
            .and_then(|n| {
                n.checked_add(
                    semantic_count
                        .checked_mul(std::mem::size_of::<BookV2StructureContentBinding>())?,
                )
            })
            .and_then(|n| {
                n.checked_add(
                    scopes
                        .len()
                        .checked_mul(std::mem::size_of::<Option<usize>>())?,
                )
            })
            .ok_or(E::Spool)?;
        let records = count
            .checked_mul(2)
            .and_then(|n| n.checked_add(semantic_count))
            .and_then(|n| n.checked_add(scopes.len()))
            .ok_or(E::Records)?;
        self.budget.reserve(records, bytes, 0)?;
        self.budget.step(4)?;
        let mut nodes = Vec::new();
        nodes.try_reserve_exact(count).map_err(|_| E::Allocation)?;
        let mut lookup = Vec::new();
        lookup.try_reserve_exact(count).map_err(|_| E::Allocation)?;
        let mut bindings = Vec::<BookV2StructureContentBinding>::new();
        bindings
            .try_reserve_exact(semantic_count)
            .map_err(|_| E::Allocation)?;
        let mut groups = Vec::new();
        groups
            .try_reserve_exact(scopes.len())
            .map_err(|_| E::Allocation)?;
        let mut collector = Collect {
            budget: &mut self.budget,
            count: 0,
            nodes: Some(&mut nodes),
        };
        visit_book_v2_structure(flow, &mut collector)?;
        if collector.count != count {
            return Err(E::Identity);
        }
        for (i, node) in nodes.iter().enumerate() {
            self.budget.step(1)?;
            lookup.push((node.source.key(), i));
        }
        sort(&mut lookup, &mut self.budget)?;
        for pair in lookup.windows(2) {
            self.budget.step(1)?;
            if pair[0].0 >= pair[1].0 {
                return Err(E::Identity);
            }
        }
        if nodes.first().map(|n| (n.source.key(), n.source.parent()))
            != Some((Key::new(NodeId::new(0), Slot::Source), None))
        {
            return Err(E::Identity);
        }
        for i in 0..count {
            self.budget.step(1)?;
            if let Some(parent) = nodes[i].source.parent() {
                let parent = find(&lookup, parent, &mut self.budget)?.ok_or(E::Identity)?;
                if parent >= i {
                    return Err(E::Identity);
                }
                nodes[i].parent = Some(parent);
                nodes[i].depth = nodes[parent].depth.checked_add(1).ok_or(E::Depth)?;
                if nodes[i].depth > flow.body().body().limits().get().max_ast_nesting_depth {
                    return Err(E::Depth);
                }
                if let Some(last) = nodes[parent].last_child {
                    nodes[last].next_sibling = Some(i);
                } else {
                    nodes[parent].first_child = Some(i);
                }
                nodes[parent].last_child = Some(i);
            } else if i != 0 {
                return Err(E::Identity);
            }
        }
        // Every prepared language owner and source number appears exactly once.
        // Break/anchor records carry no semantics and are deliberately excluded.
        for language in flow.navigation().languages() {
            self.budget.step(1)?;
            find(
                &lookup,
                Key::new(language.node_id(), Slot::Source),
                &mut self.budget,
            )?
            .ok_or(E::Identity)?;
        }
        for number in flow.navigation().language_children() {
            self.budget.step(1)?;
            let i = find(
                &lookup,
                Key::new(number.node_id(), Slot::Source),
                &mut self.budget,
            )?
            .ok_or(E::Identity)?;
            let n = nodes[i].source;
            if n.source_span() != Some(number.source_span())
                || n.language() != number.effective_language()
                || n.parent() != Some(Key::new(number.parent(), Slot::Source))
            {
                return Err(E::Identity);
            }
        }
        for (group_index, group) in scopes.iter().enumerate() {
            self.budget.step(1)?;
            let Some(mcid) = group.mcid() else {
                if group.artifact().is_none() {
                    return Err(E::Identity);
                }
                groups.push(None);
                continue;
            };
            if group.artifact().is_some() {
                return Err(E::Identity);
            }
            let owner = group.owner().ok_or(E::Identity)?;
            let mut index = find(&lookup, Key::new(owner, Slot::Source), &mut self.budget)?
                .ok_or(E::Identity)?;
            if group.role() == BookV2MarkedRole::Label {
                let slot = if nodes[index].source.pdf_role() == "LI" {
                    Slot::ListLabel
                } else {
                    Slot::FootnoteLabel
                };
                index =
                    find(&lookup, Key::new(owner, slot), &mut self.budget)?.ok_or(E::Identity)?;
            }
            // Generated references use the ordinary text painter. Keep its
            // MCID under the real Link child used for the annotation OBJR.
            if group.role() == BookV2MarkedRole::Text
                && nodes[index].source.pdf_role() == "Reference"
            {
                if let Some(label) = find(
                    &lookup,
                    Key::new(owner, Slot::FootnoteLabel),
                    &mut self.budget,
                )? {
                    index = label;
                } else {
                    index = find(
                        &lookup,
                        Key::new(owner, Slot::ReferenceLabel),
                        &mut self.budget,
                    )?
                    .ok_or(E::Identity)?;
                }
            }
            let node = nodes[index].source;
            let valid = match group.role() {
                BookV2MarkedRole::Text => {
                    matches!(node.pdf_role(), "Span" | "Reference")
                        || (node.pdf_role() == "Lbl" && node.key().slot() == Slot::FootnoteLabel)
                }
                BookV2MarkedRole::Label => node.pdf_role() == "Lbl",
                BookV2MarkedRole::Formula => node.pdf_role() == "Formula",
                BookV2MarkedRole::Figure => node.pdf_role() == "Figure",
                BookV2MarkedRole::EquationNumber => node.pdf_role() == "Span",
                BookV2MarkedRole::Separator => false,
            };
            if !valid || node.alternative() != self.source.source().alternative(group_index) {
                return Err(E::Identity);
            }
            let binding_index = bindings.len();
            if let Some(last) = nodes[index].last_binding {
                bindings[last].next = Some(binding_index);
            } else {
                nodes[index].first_binding = Some(binding_index);
            }
            nodes[index].last_binding = Some(binding_index);
            bindings.push(BookV2StructureContentBinding {
                node: index,
                group: group_index,
                page: group.page_index(),
                mcid,
                next: None,
            });
            groups.push(Some(binding_index));
        }
        if bindings.len() != semantic_count {
            return Err(E::Identity);
        }
        let mut fp = self.budget.fold(
            self.source.fingerprint(),
            BOOK_V2_SOURCE_STRUCTURE_ALGORITHM.as_bytes(),
        )?;
        fp = self.budget.fold(fp, &flow.fingerprint())?;
        for node in &nodes {
            let n = node.source;
            fp = self.budget.fold(fp, &n.key().owner().get().to_be_bytes())?;
            fp = self.budget.fold(fp, &[n.key().slot() as u8])?;
            fp = self.budget.fold(
                fp,
                &(node.parent.map_or(u64::MAX, |v| v as u64)).to_be_bytes(),
            )?;
            for value in [
                Some(n.pdf_role()),
                n.semantic_kind(),
                Some(n.language()),
                n.alternative(),
            ] {
                fp = string(&mut self.budget, fp, value)?;
            }
            fp = self
                .budget
                .fold(fp, &[u8::from(n.source_span().is_some())])?;
            if let Some(span) = n.source_span() {
                for value in [
                    span.source_id().get(),
                    span.start_byte().get(),
                    span.end_byte().get(),
                ] {
                    fp = self.budget.fold(fp, &value.to_be_bytes())?;
                }
            }
            fp = self
                .budget
                .fold(fp, &[u8::from(n.table_cell().is_some())])?;
            if let Some(cell) = n.table_cell() {
                for value in [
                    cell.table.get(),
                    cell.row,
                    cell.column,
                    u32::from(cell.colspan),
                    u32::from(cell.rowspan),
                    u32::from(cell.header),
                ] {
                    fp = self.budget.fold(fp, &value.to_be_bytes())?;
                }
            }
        }
        for binding in &bindings {
            for value in [
                binding.node as u64,
                binding.group as u64,
                binding.page as u64,
                binding.mcid as u64,
            ] {
                fp = self.budget.fold(fp, &value.to_be_bytes())?;
            }
        }
        Ok(BookV2SourceStructure {
            source: self.source,
            nodes,
            lookup,
            bindings,
            groups,
            fingerprint: fp,
            budget: self.budget,
        })
    }
}
struct Collect<'v, 'b, 'a> {
    budget: &'b mut Budget,
    count: usize,
    nodes: Option<&'v mut Vec<BookV2SourceStructureNode<'a>>>,
}
impl<'a> BookV2StructureVisitor<'a> for Collect<'_, '_, 'a> {
    type Error = E;
    fn step(&mut self, work: usize) -> Result<(), E> {
        self.budget.step(work)
    }
    fn node(&mut self, source: SourceNode<'a>) -> Result<(), E> {
        self.budget.step(1)?;
        self.count = self.count.checked_add(1).ok_or(E::Records)?;
        if self.count as u64 > self.budget.max_records {
            return Err(E::Records);
        }
        if let Some(nodes) = &mut self.nodes {
            if nodes.len() == nodes.capacity() {
                return Err(E::Records);
            }
            nodes.push(BookV2SourceStructureNode {
                source,
                parent: None,
                depth: 1,
                first_child: None,
                last_child: None,
                next_sibling: None,
                first_binding: None,
                last_binding: None,
            });
        }
        Ok(())
    }
}
fn find(lookup: &[(Key, usize)], key: Key, budget: &mut Budget) -> Result<Option<usize>, E> {
    let (mut low, mut high) = (0, lookup.len());
    while low < high {
        budget.step(1)?;
        let mid = low + (high - low) / 2;
        match lookup[mid].0.cmp(&key) {
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid,
            std::cmp::Ordering::Equal => return Ok(Some(lookup[mid].1)),
        }
    }
    Ok(None)
}
fn sort(v: &mut [(Key, usize)], b: &mut Budget) -> Result<(), E> {
    fn sift(v: &mut [(Key, usize)], mut root: usize, end: usize, b: &mut Budget) -> Result<(), E> {
        loop {
            b.step(1)?;
            let Some(mut child) = root
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .filter(|n| *n < end)
            else {
                return Ok(());
            };
            if child + 1 < end {
                b.step(1)?;
                if v[child].0 < v[child + 1].0 {
                    child += 1;
                }
            }
            b.step(1)?;
            if v[root].0 >= v[child].0 {
                return Ok(());
            }
            b.step(1)?;
            v.swap(root, child);
            root = child;
        }
    }
    let n = v.len();
    for root in (0..n / 2).rev() {
        sift(v, root, n, b)?;
    }
    for end in (1..n).rev() {
        b.step(1)?;
        v.swap(0, end);
        sift(v, 0, end, b)?;
    }
    Ok(())
}
fn string(b: &mut Budget, mut fp: [u8; 32], value: Option<&str>) -> Result<[u8; 32], E> {
    fp = b.fold(
        fp,
        &(value.map_or(u64::MAX, |s| s.len() as u64)).to_be_bytes(),
    )?;
    if let Some(value) = value {
        for chunk in value.as_bytes().chunks(96) {
            fp = b.fold(fp, chunk)?;
        }
    }
    Ok(fp)
}

#[path = "book_v2_structure_relations.rs"]
mod relations;
pub use relations::*;
