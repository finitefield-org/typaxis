//! Source-preserving reading links, Note relations and table header ownership.
//! Unplaced definitions remain explicit; this is not their PDF authorization.
use super::*;
use typaxis_syntax::{ProductionTable, ProductionTableSection};
pub const BOOK_V2_STRUCTURE_RELATIONS_ALGORITHM: &str = "typaxis.book-2-structure-relations/1";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ListNumbering {
    Decimal,
    Disc,
}
#[derive(Clone, Debug)]
pub struct BookV2RelatedStructureNode {
    parent: Option<usize>,
    first_child: Option<usize>,
    last_child: Option<usize>,
    previous: Option<usize>,
    next: Option<usize>,
    depth: u32,
    headers: Range<usize>,
    related: Range<usize>,
    cursor: usize,
    list_numbering: Option<BookV2ListNumbering>,
    note_index: Option<usize>,
}
impl BookV2RelatedStructureNode {
    pub fn parent(&self) -> Option<usize> {
        self.parent
    }
    pub fn first_child(&self) -> Option<usize> {
        self.first_child
    }
    pub fn next_sibling(&self) -> Option<usize> {
        self.next
    }
    pub fn depth(&self) -> u32 {
        self.depth
    }
    pub fn list_numbering(&self) -> Option<BookV2ListNumbering> {
        self.list_numbering
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2NoteStructureEdge {
    reference: usize,
    note: usize,
    definition_index: usize,
    source_definition: Option<usize>,
    reference_link: usize,
    reference_label: usize,
    definition_link: usize,
    definition_label: usize,
    painted: bool,
    next: Option<usize>,
}
impl BookV2NoteStructureEdge {
    pub fn reference_node(&self) -> usize {
        self.reference
    }
    pub fn note_node(&self) -> usize {
        self.note
    }
    pub fn definition_index(&self) -> usize {
        self.definition_index
    }
    pub fn source_definition(&self) -> Option<usize> {
        self.source_definition
    }
    pub fn reference_link(&self) -> usize {
        self.reference_link
    }
    pub fn reference_label(&self) -> usize {
        self.reference_label
    }
    pub fn definition_link(&self) -> usize {
        self.definition_link
    }
    pub fn definition_label(&self) -> usize {
        self.definition_label
    }
    pub fn painted(&self) -> bool {
        self.painted
    }
    pub fn next_for_note(&self) -> Option<usize> {
        self.next
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2NoteStructure {
    node: usize,
    link: usize,
    label: usize,
    painted: bool,
    first: Option<usize>,
    last: Option<usize>,
    return_reference: Option<usize>,
    reading_reference: Option<usize>,
    insertion_after: Option<usize>,
}
impl BookV2NoteStructure {
    pub fn node_index(&self) -> usize {
        self.node
    }
    pub fn link_node(&self) -> usize {
        self.link
    }
    pub fn label_node(&self) -> usize {
        self.label
    }
    pub fn painted(&self) -> bool {
        self.painted
    }
    pub fn first_reference(&self) -> Option<usize> {
        self.first
    }
    /// First authored, actually painted reference edge; independent of page paint order.
    pub fn return_reference(&self) -> Option<usize> {
        self.return_reference
    }
    pub fn reading_reference(&self) -> Option<usize> {
        self.reading_reference
    }
    pub fn insertion_after(&self) -> Option<usize> {
        self.insertion_after
    }
}
pub struct BookV2StructureRelations<
    'u,
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
    source: &'u BookV2SourceStructure<
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
    nodes: Vec<BookV2RelatedStructureNode>,
    order: Vec<usize>,
    notes: Vec<BookV2NoteStructure>,
    edges: Vec<BookV2NoteStructureEdge>,
    headers: Vec<usize>,
    related: Vec<usize>,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<
        'u,
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
    BookV2StructureRelations<
        'u,
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
    ) -> &'u BookV2SourceStructure<
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
        self.source
    }
    pub fn nodes(&self) -> &[BookV2RelatedStructureNode] {
        &self.nodes
    }
    pub fn reading_order(&self) -> &[usize] {
        &self.order
    }
    pub fn notes(&self) -> &[BookV2NoteStructure] {
        &self.notes
    }
    pub fn edges(&self) -> &[BookV2NoteStructureEdge] {
        &self.edges
    }
    pub fn headers(&self, node: usize) -> Option<&[usize]> {
        self.headers.get(self.nodes.get(node)?.headers.clone())
    }
    pub fn related_nodes(&self, node: usize) -> Option<&[usize]> {
        self.related.get(self.nodes.get(node)?.related.clone())
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
pub struct BookV2StructureRelationBuilder<
    'u,
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
    source: &'u BookV2SourceStructure<
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
    budget: Budget,
}
impl<
        'u,
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
    BookV2StructureRelationBuilder<
        'u,
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
        source: &'u BookV2SourceStructure<
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
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        let display = source.source().source().display();
        display
            .verify_resources(display.admitted(), limits)
            .map_err(|_| E::Identity)?;
        let mut budget = source.budget;
        budget.max_work = max_work;
        budget.records = budget.records.max(prior_records);
        budget.spool = budget.spool.max(prior_spool);
        budget.output = budget.output.max(prior_output);
        budget.work = budget.work.max(prior_work);
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
        BookV2StructureRelations<
            'u,
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
        let source = self.source;
        let closure = source.source().source().display().source().source();
        let prepared = closure.flow();
        let flow = prepared.lines().prepared().source_flow();
        let mut header_count = 0usize;
        header_pairs(flow.tables(), &mut self.budget, |_, _, _| {
            header_count = header_count.checked_add(1).ok_or(E::Records)?;
            Ok(())
        })?;
        let n = source.nodes.len();
        let nd = prepared.footnotes().definitions().len();
        let ne = prepared.references().len();
        let nr = ne.checked_mul(2).ok_or(E::Records)?;
        let mut records = 0usize;
        let mut spool = 0usize;
        for (count, size) in [
            (n, std::mem::size_of::<BookV2RelatedStructureNode>()),
            (n, std::mem::size_of::<usize>()),
            (nd, std::mem::size_of::<BookV2NoteStructure>()),
            (ne, std::mem::size_of::<BookV2NoteStructureEdge>()),
            (header_count, std::mem::size_of::<usize>()),
            (nr, std::mem::size_of::<usize>()),
        ] {
            records = records.checked_add(count).ok_or(E::Records)?;
            spool = spool
                .checked_add(count.checked_mul(size).ok_or(E::Spool)?)
                .ok_or(E::Spool)?;
        }
        self.budget.reserve(records, spool, 0)?;
        self.budget.step(6)?;
        let mut nodes = Vec::<BookV2RelatedStructureNode>::new();
        nodes.try_reserve_exact(n).map_err(|_| E::Allocation)?;
        let mut order = Vec::new();
        order.try_reserve_exact(n).map_err(|_| E::Allocation)?;
        let mut notes = Vec::<BookV2NoteStructure>::new();
        notes.try_reserve_exact(nd).map_err(|_| E::Allocation)?;
        let mut edges = Vec::<BookV2NoteStructureEdge>::new();
        edges.try_reserve_exact(ne).map_err(|_| E::Allocation)?;
        let mut headers = Vec::new();
        headers
            .try_reserve_exact(header_count)
            .map_err(|_| E::Allocation)?;
        let mut related = Vec::new();
        related.try_reserve_exact(nr).map_err(|_| E::Allocation)?;
        for node in &source.nodes {
            self.budget.step(1)?;
            nodes.push(BookV2RelatedStructureNode {
                parent: node.parent,
                first_child: node.first_child,
                last_child: node.last_child,
                next: node.next_sibling,
                previous: None,
                depth: 0,
                headers: 0..0,
                related: 0..0,
                cursor: 0,
                list_numbering: None,
                note_index: None,
            });
        }
        for i in 0..n {
            self.budget.step(1)?;
            if let Some(next) = nodes[i].next {
                nodes[next].previous = Some(i);
            }
        }
        let locate = |budget: &mut Budget, owner: NodeId, slot| {
            find(&source.lookup, Key::new(owner, slot), budget)?.ok_or(E::Identity)
        };
        for (di, definition) in prepared.footnotes().definitions().iter().enumerate() {
            self.budget.step(1)?;
            let node = locate(&mut self.budget, definition.owner(), Slot::Source)?;
            let link = locate(&mut self.budget, definition.owner(), Slot::FootnoteLink)?;
            let label = locate(&mut self.budget, definition.owner(), Slot::FootnoteLabel)?;
            if source.nodes[node].source.pdf_role() != "Note"
                || nodes[node].parent != Some(0)
                || nodes[link].parent != Some(node)
                || nodes[label].parent != Some(link)
            {
                return Err(E::Identity);
            }
            nodes[node].note_index = Some(di);
            notes.push(BookV2NoteStructure {
                node,
                link,
                label,
                painted: source.nodes[label].first_binding.is_some(),
                first: None,
                last: None,
                return_reference: None,
                reading_reference: None,
                insertion_after: None,
            });
        }
        for reference in prepared.references() {
            self.budget.step(1)?;
            let r = reference.source();
            let note = notes.get_mut(r.definition_index()).ok_or(E::Identity)?;
            let node = locate(&mut self.budget, r.owner(), Slot::Source)?;
            let link = locate(&mut self.budget, r.owner(), Slot::FootnoteLink)?;
            let label = locate(&mut self.budget, r.owner(), Slot::FootnoteLabel)?;
            if source.nodes[node].source.pdf_role() != "Reference"
                || nodes[link].parent != Some(node)
                || nodes[label].parent != Some(link)
            {
                return Err(E::Identity);
            }
            let mut ancestor = source.nodes[node].parent;
            let mut origin = None;
            while let Some(a) = ancestor {
                self.budget.step(1)?;
                if let Some(di) = nodes[a].note_index {
                    origin = Some(di);
                    break;
                }
                ancestor = source.nodes[a].parent;
            }
            if origin != r.source_definition() {
                return Err(E::Identity);
            }
            let painted = source.nodes[label].first_binding.is_some();
            if painted && !note.painted {
                return Err(E::Identity);
            }
            let index = edges.len();
            if let Some(last) = note.last {
                edges[last].next = Some(index);
            } else {
                note.first = Some(index);
            }
            note.last = Some(index);
            if painted && note.return_reference.is_none() {
                note.return_reference = Some(index);
            }
            nodes[node].related.end += 1;
            nodes[note.node].related.end += 1;
            edges.push(BookV2NoteStructureEdge {
                reference: node,
                note: note.node,
                definition_index: r.definition_index(),
                source_definition: r.source_definition(),
                reference_link: link,
                reference_label: label,
                definition_link: note.link,
                definition_label: note.label,
                painted,
                next: None,
            });
        }
        let mut unplaced = 0;
        for note in &notes {
            self.budget.step(1)?;
            if !note.painted {
                unplaced += 1;
            }
        }
        if unplaced != closure.unreferenced_definitions() {
            return Err(E::Identity);
        }
        // The stable demand's first-reference forest roots note dependencies
        // in actually selected body content, even when definition order runs
        // backwards or later references form a cycle. It is not reconstructed
        // from the order in which page paints happen to be emitted.
        let demand = closure
            .stable()
            .sequence()
            .pages()
            .last()
            .ok_or(E::Identity)?
            .next_state()
            .source_state()
            .demand();
        for di in 0..nd {
            self.budget.step(1)?;
            let first = demand.first_reference(di);
            if first.is_some() != notes[di].painted {
                return Err(E::Identity);
            }
            let Some(first) = first else {
                continue;
            };
            let mut edge = notes[di].first;
            let mut chosen = None;
            while let Some(ei) = edge {
                self.budget.step(1)?;
                let e = edges[ei];
                edge = e.next;
                if source.nodes[e.reference].source.key().owner() == first {
                    if !e.painted || chosen.is_some() {
                        return Err(E::Identity);
                    }
                    chosen = Some(ei);
                }
            }
            let ei = chosen.ok_or(E::Identity)?;
            let (parent, branch) =
                paragraph_anchor(&source.nodes, edges[ei].reference, &mut self.budget)?;
            notes[di].reading_reference = Some(ei);
            notes[di].insertion_after = Some(branch);
            detach(&mut nodes, notes[di].node)?;
            insert_after(&mut nodes, notes[di].node, parent, branch)?;
        }
        // Reject a corrupted seed forest before any refinement could obscure
        // a cycle in the actual selected first-reference provenance.
        for note in &notes {
            self.budget.step(1)?;
            if !note.painted {
                continue;
            }
            let mut ancestor = Some(note.node);
            let mut visited = 0usize;
            while let Some(a) = ancestor {
                self.budget.step(1)?;
                visited += 1;
                if visited > n {
                    return Err(E::Identity);
                }
                ancestor = nodes[a].parent;
            }
        }
        // Select the last painted reference that does not put the note inside
        // its own descendants. Earlier authored notes are resolved first; all
        // reference edges, including dependency back-edges, remain explicit.
        for di in 0..nd {
            self.budget.step(1)?;
            if !notes[di].painted {
                continue;
            }
            let target = notes[di].node;
            let mut edge = notes[di].first;
            let mut selected = None;
            while let Some(ei) = edge {
                self.budget.step(1)?;
                let e = edges[ei];
                edge = e.next;
                if !e.painted {
                    continue;
                }
                let (parent, branch) =
                    paragraph_anchor(&source.nodes, e.reference, &mut self.budget)?;
                let mut ancestor = Some(parent);
                let mut cycle = false;
                let mut visited = 0usize;
                while let Some(a) = ancestor {
                    visited += 1;
                    if visited > n {
                        return Err(E::Identity);
                    }
                    self.budget.step(1)?;
                    if a == target {
                        cycle = true;
                        break;
                    }
                    ancestor = nodes[a].parent;
                }
                if !cycle {
                    selected = Some((ei, parent, branch));
                }
            }
            let (ei, parent, branch) = selected.ok_or(E::Identity)?;
            notes[di].reading_reference = Some(ei);
            notes[di].insertion_after = Some(branch);
            // Stable order for several notes attached after the same inline branch.
            let key = (
                source.nodes[edges[ei].reference].source.key().owner(),
                source.nodes[target].source.key().owner(),
            );
            detach(&mut nodes, target)?;
            let mut after = branch;
            while let Some(next) = nodes[after].next {
                self.budget.step(1)?;
                let Some(other) = nodes[next].note_index else {
                    break;
                };
                if notes[other].insertion_after != Some(branch) {
                    break;
                }
                let oe = notes[other].reading_reference.ok_or(E::Identity)?;
                let other_key = (
                    source.nodes[edges[oe].reference].source.key().owner(),
                    source.nodes[next].source.key().owner(),
                );
                if other_key > key {
                    break;
                }
                after = next;
            }
            insert_after(&mut nodes, target, parent, after)?;
        }
        // Iterate the actual final tree without a recursion stack; detect lost
        // nodes/cycles and charge every ascent, visit and generated depth.
        let mut current = Some(0usize);
        while let Some(i) = current {
            self.budget.step(1)?;
            if order.len() >= n || nodes[i].depth != 0 {
                return Err(E::Identity);
            }
            nodes[i].depth = match nodes[i].parent {
                Some(p) => nodes[p].depth.checked_add(1).ok_or(E::Depth)?,
                None if i == 0 => 1,
                None => return Err(E::Identity),
            };
            if nodes[i].depth > flow.body().body().limits().get().max_ast_nesting_depth {
                return Err(E::Depth);
            }
            order.push(i);
            if let Some(child) = nodes[i].first_child {
                current = Some(child);
                continue;
            }
            let mut cursor = i;
            loop {
                self.budget.step(1)?;
                if let Some(next) = nodes[cursor].next {
                    current = Some(next);
                    break;
                }
                match nodes[cursor].parent {
                    Some(parent) => cursor = parent,
                    None => {
                        current = None;
                        break;
                    }
                }
            }
        }
        if order.len() != n {
            return Err(E::Identity);
        }
        for list in flow.lists() {
            self.budget.step(1)?;
            let i = locate(&mut self.budget, list.owner(), Slot::Source)?;
            if source.nodes[i].source.pdf_role() != "L" {
                return Err(E::Identity);
            }
            nodes[i].list_numbering = Some(if list.ordered() {
                BookV2ListNumbering::Decimal
            } else {
                BookV2ListNumbering::Disc
            });
        }
        header_pairs(flow.tables(), &mut self.budget, |b, cell, header| {
            let i = locate(b, cell, Slot::Source)?;
            let h = locate(b, header, Slot::Source)?;
            if source.nodes[i].source.pdf_role() != "TD"
                || source.nodes[h].source.pdf_role() != "TH"
            {
                return Err(E::Identity);
            }
            if nodes[i].headers.is_empty() {
                nodes[i].headers = headers.len()..headers.len();
            }
            if nodes[i].headers.end != headers.len() {
                return Err(E::Identity);
            }
            headers.push(h);
            nodes[i].headers.end += 1;
            Ok(())
        })?;
        if headers.len() != header_count {
            return Err(E::Identity);
        }
        let mut total = 0usize;
        for node in &mut nodes {
            self.budget.step(1)?;
            let count = node.related.end;
            node.related = total..total.checked_add(count).ok_or(E::Records)?;
            node.cursor = total;
            total = node.related.end;
        }
        if total != nr {
            return Err(E::Identity);
        }
        for _ in 0..nr {
            self.budget.step(1)?;
            related.push(usize::MAX);
        }
        for e in &edges {
            for (a, z) in [(e.reference, e.note), (e.note, e.reference)] {
                self.budget.step(1)?;
                let cursor = nodes[a].cursor;
                if cursor >= nodes[a].related.end {
                    return Err(E::Identity);
                }
                related[cursor] = z;
                nodes[a].cursor += 1;
            }
        }
        let mut fp = self.budget.fold(
            source.fingerprint(),
            BOOK_V2_STRUCTURE_RELATIONS_ALGORITHM.as_bytes(),
        )?;
        for node in &nodes {
            for value in [
                node.parent,
                node.first_child,
                node.next,
                Some(node.depth as usize),
                Some(node.headers.start),
                Some(node.headers.end),
                Some(node.related.start),
                Some(node.related.end),
                node.note_index,
            ] {
                fp = number(&mut self.budget, fp, value)?;
            }
            fp = self.budget.fold(
                fp,
                &[match node.list_numbering {
                    None => 0,
                    Some(BookV2ListNumbering::Decimal) => 1,
                    Some(BookV2ListNumbering::Disc) => 2,
                }],
            )?;
        }
        for &value in order.iter().chain(&headers).chain(&related) {
            fp = number(&mut self.budget, fp, Some(value))?;
        }
        for note in &notes {
            for value in [
                Some(note.node),
                Some(note.link),
                Some(note.label),
                Some(usize::from(note.painted)),
                note.first,
                note.last,
                note.return_reference,
                note.reading_reference,
                note.insertion_after,
            ] {
                fp = number(&mut self.budget, fp, value)?;
            }
        }
        for e in &edges {
            for value in [
                Some(e.reference),
                Some(e.note),
                Some(e.definition_index),
                e.source_definition,
                Some(e.reference_link),
                Some(e.reference_label),
                Some(e.definition_link),
                Some(e.definition_label),
                Some(usize::from(e.painted)),
                e.next,
            ] {
                fp = number(&mut self.budget, fp, value)?;
            }
        }
        Ok(BookV2StructureRelations {
            source,
            nodes,
            order,
            notes,
            edges,
            headers,
            related,
            fingerprint: fp,
            budget: self.budget,
        })
    }
}
fn number(b: &mut Budget, fp: [u8; 32], value: Option<usize>) -> Result<[u8; 32], E> {
    b.fold(fp, &value.map_or(u64::MAX, |v| v as u64).to_be_bytes())
}
fn detach(nodes: &mut [BookV2RelatedStructureNode], i: usize) -> Result<(), E> {
    let parent = nodes[i].parent.ok_or(E::Identity)?;
    let previous = nodes[i].previous;
    let next = nodes[i].next;
    if let Some(p) = previous {
        nodes[p].next = next;
    } else {
        nodes[parent].first_child = next;
    }
    if let Some(n) = next {
        nodes[n].previous = previous;
    } else {
        nodes[parent].last_child = previous;
    }
    nodes[i].parent = None;
    nodes[i].previous = None;
    nodes[i].next = None;
    Ok(())
}
fn insert_after(
    nodes: &mut [BookV2RelatedStructureNode],
    i: usize,
    parent: usize,
    after: usize,
) -> Result<(), E> {
    if nodes[after].parent != Some(parent) || nodes[i].parent.is_some() {
        return Err(E::Identity);
    }
    let next = nodes[after].next;
    nodes[i].parent = Some(parent);
    nodes[i].previous = Some(after);
    nodes[i].next = next;
    nodes[after].next = Some(i);
    if let Some(next) = next {
        nodes[next].previous = Some(i);
    } else {
        nodes[parent].last_child = Some(i);
    }
    Ok(())
}
fn header_pairs(
    tables: &[ProductionTable],
    b: &mut Budget,
    mut emit: impl FnMut(&mut Budget, NodeId, NodeId) -> Result<(), E>,
) -> Result<(), E> {
    for table in tables {
        b.step(1)?;
        let mut head_end = 0;
        for row in table.rows() {
            b.step(1)?;
            if row.section() != ProductionTableSection::Head {
                break;
            }
            head_end = row.cells().end;
        }
        for cell in &table.cells()[head_end..] {
            b.step(1)?;
            let end = cell
                .column()
                .checked_add(u32::from(cell.colspan().get()))
                .ok_or(E::Identity)?;
            for header in &table.cells()[..head_end] {
                b.step(1)?;
                let h_end = header
                    .column()
                    .checked_add(u32::from(header.colspan().get()))
                    .ok_or(E::Identity)?;
                if cell.column() < h_end && header.column() < end {
                    emit(b, cell.owner(), header.owner())?;
                }
            }
        }
    }
    Ok(())
}

fn paragraph_anchor(
    source: &[BookV2SourceStructureNode<'_>],
    reference: usize,
    b: &mut Budget,
) -> Result<(usize, usize), E> {
    let mut branch = reference;
    let mut parent = source[branch].parent;
    while let Some(p) = parent {
        b.step(1)?;
        if matches!(
            source[p].source.pdf_role(),
            "P" | "H1" | "H2" | "H3" | "H4" | "H5" | "H6"
        ) || (source[p].source.pdf_role() == "Lbl"
            && source[p].source.key().slot() == Slot::Source)
        {
            // Authored description terms own an inline-bearing source Lbl.
            // Generated list/footnote labels have separate slots and must not
            // become containers for relocated Note reading order.
            return Ok((p, branch));
        }
        branch = p;
        parent = source[p].parent;
    }
    Err(E::Identity)
}

#[path = "book_v2_navigation_geometry.rs"]
mod navigation_geometry;
pub use navigation_geometry::*;
