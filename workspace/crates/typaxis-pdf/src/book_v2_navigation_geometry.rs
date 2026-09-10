//! Actual navigation positions and annotation rectangles over the selected body.
//! Coordinates remain Y-down until the real page object supplies its transform.
use super::*;
use typaxis_core::{Length, PositiveLength, Rect};
use typaxis_display_list::book_v2::{
    BookV2BodyDisplay, BookV2BodyPaintIndex as Paint, BookV2MathPaint,
};
use typaxis_syntax::{
    ProductionInlineLinkTarget as LinkTarget, ProductionInlineReference as Reference,
};
pub const BOOK_V2_NAVIGATION_GEOMETRY_ALGORITHM: &str = "typaxis.book-2-navigation-geometry/1";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2NavigationGeometryError {
    Resource(E),
    UnplacedDestination(NodeId),
    ConflictingLinks(NodeId),
}
use BookV2NavigationGeometryError as NE;
impl From<E> for NE {
    fn from(e: E) -> Self {
        Self::Resource(e)
    }
}
impl std::fmt::Display for NE {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 navigation: {self:?}")
    }
}
impl std::error::Error for NE {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2NavigationPosition {
    page: u32,
    fragment: usize,
    x: Length,
    y: Length,
}
impl BookV2NavigationPosition {
    pub fn page_index(self) -> u32 {
        self.page
    }
    pub fn fragment_index(self) -> usize {
        self.fragment
    }
    pub fn x(self) -> Length {
        self.x
    }
    pub fn y(self) -> Length {
        self.y
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2DestinationKind<'a> {
    Anchor(&'a str),
    Footnote(usize),
    Reference(usize),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2NavigationDestination<'a> {
    owner: NodeId,
    kind: BookV2DestinationKind<'a>,
    position: Option<BookV2NavigationPosition>,
}
impl<'a> BookV2NavigationDestination<'a> {
    pub fn owner(self) -> NodeId {
        self.owner
    }
    pub fn kind(self) -> BookV2DestinationKind<'a> {
        self.kind
    }
    pub fn position(self) -> Option<BookV2NavigationPosition> {
        self.position
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2NavigationTarget<'a> {
    Destination(usize),
    Uri(&'a str),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2NavigationLink<'a> {
    node: usize,
    target: BookV2NavigationTarget<'a>,
    first: Option<usize>,
    last: Option<usize>,
}
impl<'a> BookV2NavigationLink<'a> {
    pub fn structure_node(self) -> usize {
        self.node
    }
    pub fn target(self) -> BookV2NavigationTarget<'a> {
        self.target
    }
    pub fn first_rectangle(self) -> Option<usize> {
        self.first
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2NavigationRectangle {
    link: usize,
    page: u32,
    fragment: usize,
    first_group: usize,
    bounds: Rect,
    next: Option<usize>,
}
impl BookV2NavigationRectangle {
    pub fn link_index(self) -> usize {
        self.link
    }
    pub fn page_index(self) -> u32 {
        self.page
    }
    pub fn fragment_index(self) -> usize {
        self.fragment
    }
    pub fn first_group(self) -> usize {
        self.first_group
    }
    pub fn bounds(self) -> Rect {
        self.bounds
    }
    pub fn next_for_link(self) -> Option<usize> {
        self.next
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2NavigationOutline {
    destination: usize,
    parent: Option<usize>,
    first_child: Option<usize>,
    last_child: Option<usize>,
    previous: Option<usize>,
    next: Option<usize>,
    descendants: usize,
}
impl BookV2NavigationOutline {
    pub fn destination(self) -> usize {
        self.destination
    }
    pub fn parent(self) -> Option<usize> {
        self.parent
    }
    pub fn first_child(self) -> Option<usize> {
        self.first_child
    }
    pub fn last_child(self) -> Option<usize> {
        self.last_child
    }
    pub fn previous(self) -> Option<usize> {
        self.previous
    }
    pub fn next(self) -> Option<usize> {
        self.next
    }
    pub fn descendant_count(self) -> usize {
        self.descendants
    }
}
#[derive(Clone, Copy)]
struct FirstBounds {
    page: u32,
    fragment: usize,
    bounds: Rect,
}
impl FirstBounds {
    fn position(self) -> BookV2NavigationPosition {
        BookV2NavigationPosition {
            page: self.page,
            fragment: self.fragment,
            x: self.bounds.x(),
            y: self.bounds.y(),
        }
    }
}
pub struct BookV2NavigationGeometry<
    'w,
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
    source: &'w BookV2StructureRelations<
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
    destinations: Vec<BookV2NavigationDestination<'w>>,
    links: Vec<BookV2NavigationLink<'w>>,
    rectangles: Vec<BookV2NavigationRectangle>,
    pages: Vec<Range<usize>>,
    outline: Vec<BookV2NavigationOutline>,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<
        'w,
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
    BookV2NavigationGeometry<
        'w,
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
    ) -> &'w BookV2StructureRelations<
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
        self.source
    }
    pub fn destinations(&self) -> &[BookV2NavigationDestination<'w>] {
        &self.destinations
    }
    pub fn links(&self) -> &[BookV2NavigationLink<'w>] {
        &self.links
    }
    pub fn rectangles(&self) -> &[BookV2NavigationRectangle] {
        &self.rectangles
    }
    pub fn page_rectangles(&self, page: usize) -> Option<&[BookV2NavigationRectangle]> {
        self.rectangles.get(self.pages.get(page)?.clone())
    }
    pub fn outline(&self) -> &[BookV2NavigationOutline] {
        &self.outline
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
pub struct BookV2NavigationGeometryBuilder<
    'w,
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
    source: &'w BookV2StructureRelations<
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
    budget: Budget,
}
impl<
        'w,
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
    BookV2NavigationGeometryBuilder<
        'w,
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
        source: &'w BookV2StructureRelations<
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
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, NE> {
        let display = source.source().source().source().display();
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
            return Err(E::Work.into());
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
        BookV2NavigationGeometry<
            'w,
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
        NE,
    > {
        let relations = self.source;
        let registry = relations.source();
        let marked = registry.source();
        let scopes = marked.source();
        let display = scopes.display();
        let flow = display
            .source()
            .source()
            .flow()
            .lines()
            .prepared()
            .source_flow();
        let navigation = flow.navigation();
        let n = registry.nodes().len();
        let na = navigation.anchors().len();
        let nd = relations.notes().len();
        let ne = relations.edges().len();
        let dc = na
            .checked_add(nd)
            .and_then(|v| v.checked_add(ne))
            .ok_or(E::Records)?;
        let mut lc = ne;
        for p in flow.paragraphs() {
            self.budget.step(1)?;
            for site in p.items() {
                self.budget.step(1)?;
                if site.link_target().is_some()
                    || matches!(site.reference(), Some(Reference::Anchor { .. }))
                {
                    lc = lc.checked_add(1).ok_or(E::Records)?;
                }
            }
        }
        for note in relations.notes() {
            self.budget.step(1)?;
            lc = lc
                .checked_add(usize::from(note.return_reference().is_some()))
                .ok_or(E::Records)?;
        }
        let mut destinations = reserved::<BookV2NavigationDestination>(dc, &mut self.budget)?;
        let mut links = reserved::<BookV2NavigationLink>(lc, &mut self.budget)?;
        let mut rectangles =
            reserved::<BookV2NavigationRectangle>(scopes.groups().len(), &mut self.budget)?;
        let mut pages = reserved::<Range<usize>>(marked.pages().len(), &mut self.budget)?;
        let mut outline =
            reserved::<BookV2NavigationOutline>(navigation.outline().len(), &mut self.budget)?;
        let mut active = reserved::<Option<usize>>(n, &mut self.budget)?;
        let mut first = reserved::<Option<FirstBounds>>(n, &mut self.budget)?;
        for _ in 0..n {
            self.budget.step(1)?;
            active.push(None);
            first.push(None);
        }
        for (id, owner) in navigation.anchors() {
            self.budget.step(1)?;
            destinations.push(BookV2NavigationDestination {
                owner: *owner,
                kind: BookV2DestinationKind::Anchor(id.as_str()),
                position: None,
            });
        }
        for (i, note) in relations.notes().iter().enumerate() {
            self.budget.step(1)?;
            destinations.push(BookV2NavigationDestination {
                owner: registry.nodes()[note.node_index()].source().key().owner(),
                kind: BookV2DestinationKind::Footnote(i),
                position: None,
            });
        }
        for (i, edge) in relations.edges().iter().enumerate() {
            self.budget.step(1)?;
            destinations.push(BookV2NavigationDestination {
                owner: registry.nodes()[edge.reference_node()]
                    .source()
                    .key()
                    .owner(),
                kind: BookV2DestinationKind::Reference(i),
                position: None,
            });
        }
        let locate = |owner, slot, b: &mut Budget| {
            find(&registry.lookup, Key::new(owner, slot), b)?.ok_or(E::Identity)
        };
        for p in flow.paragraphs() {
            self.budget.step(1)?;
            for site in p.items() {
                self.budget.step(1)?;
                let target = match (site.link_target(), site.reference()) {
                    (Some(LinkTarget::Internal { anchor_id }), None) => {
                        Some(BookV2NavigationTarget::Destination(anchor_index(
                            &destinations[..na],
                            anchor_id,
                            &mut self.budget,
                        )?))
                    }
                    (Some(LinkTarget::Uri { uri }), None) => Some(BookV2NavigationTarget::Uri(uri)),
                    (None, Some(Reference::Anchor { target, .. })) => {
                        Some(BookV2NavigationTarget::Destination(anchor_index(
                            &destinations[..na],
                            target,
                            &mut self.budget,
                        )?))
                    }
                    (None, _) => None,
                    _ => return Err(E::Identity.into()),
                };
                if let Some(target) = target {
                    let slot = if matches!(site.reference(), Some(Reference::Anchor { .. })) {
                        Slot::ReferenceLink
                    } else {
                        Slot::Source
                    };
                    let node = locate(site.owner(), slot, &mut self.budget)?;
                    push_link(&mut links, &mut active, node, target)?;
                }
            }
        }
        for edge in relations.edges() {
            self.budget.step(1)?;
            push_link(
                &mut links,
                &mut active,
                edge.reference_link(),
                BookV2NavigationTarget::Destination(na + edge.definition_index()),
            )?;
        }
        for note in relations.notes() {
            self.budget.step(1)?;
            if let Some(edge) = note.return_reference() {
                push_link(
                    &mut links,
                    &mut active,
                    note.link_node(),
                    BookV2NavigationTarget::Destination(na + nd + edge),
                )?;
            }
        }
        if links.len() != lc || destinations.len() != dc {
            return Err(E::Identity.into());
        }
        // Use the original source ancestry. Relocated Note reading order must
        // not merge a definition's ink into its referring paragraph or link.
        for i in 0..n {
            self.budget.step(1)?;
            if let Some(parent) = registry.nodes()[i].parent() {
                if parent >= i {
                    return Err(E::Identity.into());
                }
                if active[i].is_some() && active[parent].is_some() {
                    return Err(NE::ConflictingLinks(
                        registry.nodes()[i].source().key().owner(),
                    ));
                }
                if active[i].is_none() {
                    active[i] = active[parent];
                }
            }
        }
        // A real empty line has source geometry without paint or an MCID. Use
        // its selected line box for source destinations, never link rectangles.
        // The original header owns the destination; repeated copies do not.
        for empty in display.anchors().nonpainting_lines() {
            self.budget.step(1)?;
            if empty.repeated_header() {
                continue;
            }
            let fragment = empty.fragment().fragment();
            let value = FirstBounds {
                page: fragment.page_index(),
                fragment: empty.fragment_index(),
                bounds: fragment.bounds(),
            };
            let mut cursor = Some(
                find(
                    &registry.lookup,
                    Key::new(fragment.owner(), Slot::Source),
                    &mut self.budget,
                )?
                .ok_or(E::Identity)?,
            );
            while let Some(node) = cursor {
                self.budget.step(1)?;
                if first[node].is_none() {
                    first[node] = Some(value);
                }
                cursor = registry.nodes()[node].parent();
            }
        }
        for (pi, page) in marked.pages().iter().enumerate() {
            self.budget.step(1)?;
            let start = rectangles.len();
            for gi in page.groups() {
                self.budget.step(1)?;
                let group = &scopes.groups()[gi];
                let Some(binding) = registry.for_group(gi) else {
                    continue;
                };
                if group.page_index() as usize != pi {
                    return Err(E::Identity.into());
                }
                let Some(bounds) = group_bounds(display, group, &mut self.budget)? else {
                    continue;
                };
                let value = FirstBounds {
                    page: pi as u32,
                    fragment: group.fragment_index(),
                    bounds,
                };
                let mut cursor = Some(binding.node_index());
                while let Some(node) = cursor {
                    self.budget.step(1)?;
                    match &mut first[node] {
                        None => first[node] = Some(value),
                        Some(prior)
                            if (value.page, value.fragment) < (prior.page, prior.fragment) =>
                        {
                            *prior = value;
                        }
                        Some(prior)
                            if prior.fragment == value.fragment && prior.page == value.page =>
                        {
                            prior.bounds = union(prior.bounds, bounds)?
                        }
                        Some(_) => {}
                    }
                    cursor = registry.nodes()[node].parent();
                }
                if let Some(li) = active[binding.node_index()] {
                    let link = &mut links[li];
                    if let Some(last) = link.last {
                        let r = &mut rectangles[last];
                        if r.page == value.page && r.fragment == value.fragment {
                            r.bounds = union(r.bounds, bounds)?;
                            continue;
                        }
                    }
                    let index = rectangles.len();
                    if let Some(last) = link.last {
                        rectangles[last].next = Some(index);
                    } else {
                        link.first = Some(index);
                    }
                    link.last = Some(index);
                    rectangles.push(BookV2NavigationRectangle {
                        link: li,
                        page: value.page,
                        fragment: value.fragment,
                        first_group: gi,
                        bounds,
                        next: None,
                    });
                }
            }
            pages.push(start..rectangles.len());
        }
        for d in &mut destinations[..na] {
            self.budget.step(1)?;
            if let Some(node) = find(
                &registry.lookup,
                Key::new(d.owner, Slot::Source),
                &mut self.budget,
            )? {
                d.position = first[node].map(FirstBounds::position);
            }
        }
        for a in display.anchors().positions() {
            self.budget.step(1)?;
            if a.repeated_header() {
                continue;
            }
            let owner = a.anchor().source().owner();
            let mut found = false;
            for d in &mut destinations[..na] {
                self.budget.step(1)?;
                if d.owner == owner {
                    if found || d.position.is_some() {
                        return Err(E::Identity.into());
                    }
                    d.position = Some(BookV2NavigationPosition {
                        page: a.fragment().fragment().page_index(),
                        fragment: a.fragment_index(),
                        x: a.x(),
                        y: a.baseline(),
                    });
                    found = true;
                }
            }
            if !found {
                return Err(E::Identity.into());
            }
        }
        for (i, note) in relations.notes().iter().enumerate() {
            self.budget.step(1)?;
            destinations[na + i].position = first[note.node_index()].map(FirstBounds::position);
            if destinations[na + i].position.is_some() != note.painted() {
                return Err(E::Identity.into());
            }
        }
        for (i, edge) in relations.edges().iter().enumerate() {
            self.budget.step(1)?;
            destinations[na + nd + i].position =
                first[edge.reference_label()].map(FirstBounds::position);
            if destinations[na + nd + i].position.is_some() != edge.painted() {
                return Err(E::Identity.into());
            }
        }
        for link in &links {
            self.budget.step(1)?;
            if link.first.is_some() {
                if let BookV2NavigationTarget::Destination(i) = link.target {
                    if destinations[i].position.is_none() {
                        return Err(NE::UnplacedDestination(destinations[i].owner));
                    }
                }
            }
        }
        let mut last_root = None;
        for (i, entry) in navigation.outline().iter().enumerate() {
            self.budget.step(1)?;
            let destination = anchor_index(
                &destinations[..na],
                entry.destination.as_str(),
                &mut self.budget,
            )?;
            if destinations[destination].position.is_none() {
                return Err(NE::UnplacedDestination(destinations[destination].owner));
            }
            let mut parent = None;
            if let Some(id) = entry.parent_outline_id {
                for (j, p) in navigation.outline()[..i].iter().enumerate() {
                    self.budget.step(1)?;
                    if p.outline_id == id {
                        parent = Some(j);
                        break;
                    }
                }
                if parent.is_none() {
                    return Err(E::Identity.into());
                }
            }
            let previous = parent.map_or(last_root, |p| outline[p].last_child);
            outline.push(BookV2NavigationOutline {
                destination,
                parent,
                first_child: None,
                last_child: None,
                previous,
                next: None,
                descendants: 0,
            });
            if let Some(previous) = previous {
                outline[previous].next = Some(i);
            }
            if let Some(parent) = parent {
                if outline[parent].first_child.is_none() {
                    outline[parent].first_child = Some(i);
                }
                outline[parent].last_child = Some(i);
            } else {
                last_root = Some(i);
            }
            let mut ancestor = parent;
            while let Some(p) = ancestor {
                self.budget.step(1)?;
                outline[p].descendants = outline[p].descendants.checked_add(1).ok_or(E::Records)?;
                ancestor = outline[p].parent;
            }
        }
        let mut fp = self.budget.fold(
            relations.fingerprint(),
            BOOK_V2_NAVIGATION_GEOMETRY_ALGORITHM.as_bytes(),
        )?;
        for d in &destinations {
            fp = number(&mut self.budget, fp, Some(d.owner.get() as usize))?;
            match d.kind {
                BookV2DestinationKind::Anchor(s) => {
                    fp = self.budget.fold(fp, &[0])?;
                    fp = string_hash(&mut self.budget, fp, s)?;
                }
                BookV2DestinationKind::Footnote(i) => {
                    fp = self.budget.fold(fp, &[1])?;
                    fp = number(&mut self.budget, fp, Some(i))?;
                }
                BookV2DestinationKind::Reference(i) => {
                    fp = self.budget.fold(fp, &[2])?;
                    fp = number(&mut self.budget, fp, Some(i))?;
                }
            }
            fp = self.budget.fold(fp, &[u8::from(d.position.is_some())])?;
            if let Some(p) = d.position {
                fp = number(&mut self.budget, fp, Some(p.page as usize))?;
                fp = number(&mut self.budget, fp, Some(p.fragment))?;
                fp = self.budget.fold(fp, &p.x.raw().to_be_bytes())?;
                fp = self.budget.fold(fp, &p.y.raw().to_be_bytes())?;
            }
        }
        for link in &links {
            for n in [Some(link.node), link.first, link.last] {
                fp = number(&mut self.budget, fp, n)?;
            }
            match link.target {
                BookV2NavigationTarget::Destination(i) => {
                    fp = self.budget.fold(fp, &[0])?;
                    fp = number(&mut self.budget, fp, Some(i))?;
                }
                BookV2NavigationTarget::Uri(s) => {
                    fp = self.budget.fold(fp, &[1])?;
                    fp = string_hash(&mut self.budget, fp, s)?;
                }
            }
        }
        for r in &rectangles {
            for n in [
                Some(r.link),
                Some(r.page as usize),
                Some(r.fragment),
                Some(r.first_group),
                r.next,
            ] {
                fp = number(&mut self.budget, fp, n)?;
            }
            for v in [
                r.bounds.x(),
                r.bounds.y(),
                r.bounds.width().get(),
                r.bounds.height().get(),
            ] {
                fp = self.budget.fold(fp, &v.raw().to_be_bytes())?;
            }
        }
        for p in &pages {
            fp = number(&mut self.budget, fp, Some(p.start))?;
            fp = number(&mut self.budget, fp, Some(p.end))?;
        }
        for (o, entry) in outline.iter().zip(navigation.outline()) {
            for n in [
                Some(o.destination),
                o.parent,
                o.first_child,
                o.last_child,
                o.previous,
                o.next,
                Some(o.descendants),
                Some(entry.outline_id as usize),
                Some(entry.level as usize),
            ] {
                fp = number(&mut self.budget, fp, n)?;
            }
            fp = string_hash(&mut self.budget, fp, &entry.label)?;
        }
        Ok(BookV2NavigationGeometry {
            source: relations,
            destinations,
            links,
            rectangles,
            pages,
            outline,
            fingerprint: fp,
            budget: self.budget,
        })
    }
}
fn reserved<T>(count: usize, b: &mut Budget) -> Result<Vec<T>, E> {
    b.reserve(
        count,
        count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(E::Spool)?,
        0,
    )?;
    b.step(1)?;
    let mut result = Vec::new();
    result.try_reserve_exact(count).map_err(|_| E::Allocation)?;
    Ok(result)
}
fn push_link<'a>(
    links: &mut Vec<BookV2NavigationLink<'a>>,
    active: &mut [Option<usize>],
    node: usize,
    target: BookV2NavigationTarget<'a>,
) -> Result<(), E> {
    if node >= active.len() || active[node].is_some() || links.len() == links.capacity() {
        return Err(E::Identity);
    }
    active[node] = Some(links.len());
    links.push(BookV2NavigationLink {
        node,
        target,
        first: None,
        last: None,
    });
    Ok(())
}
fn anchor_index(
    d: &[BookV2NavigationDestination<'_>],
    name: &str,
    b: &mut Budget,
) -> Result<usize, E> {
    for (i, d) in d.iter().enumerate() {
        b.step(1)?;
        if d.kind == BookV2DestinationKind::Anchor(name) {
            return Ok(i);
        }
    }
    Err(E::Identity)
}
fn union(a: Rect, b: Rect) -> Result<Rect, E> {
    let x = a.x().min(b.x());
    let y = a.y().min(b.y());
    let right = a
        .x()
        .checked_add(a.width().get())
        .ok_or(E::Identity)?
        .max(b.x().checked_add(b.width().get()).ok_or(E::Identity)?);
    let bottom = a
        .y()
        .checked_add(a.height().get())
        .ok_or(E::Identity)?
        .max(b.y().checked_add(b.height().get()).ok_or(E::Identity)?);
    Ok(Rect::new(
        x,
        y,
        PositiveLength::new(right.checked_sub(x).ok_or(E::Identity)?).ok_or(E::Identity)?,
        PositiveLength::new(bottom.checked_sub(y).ok_or(E::Identity)?).ok_or(E::Identity)?,
    ))
}
fn group_bounds(
    display: &BookV2BodyDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    group: &BookV2MarkedScope,
    b: &mut Budget,
) -> Result<Option<Rect>, E> {
    let mut bounds = None;
    for pi in group.paints() {
        b.step(1)?;
        let next = match display.paints()[pi] {
            Paint::Text(i) => display.text().draws()[i].logical_bounds(),
            Paint::Marker(i) => Some(display.markers().draws()[i].placement().bounds()),
            Paint::Math(i) => Some(match display.math().draws()[i].paint() {
                BookV2MathPaint::Native(n) => n.bounds(),
                BookV2MathPaint::Vector(v) => v.viewport(),
            }),
            Paint::Image(i) => Some(display.images().draws()[i].paint().viewport()),
            Paint::EquationNumber(i) => {
                Some(display.numbers().draws()[i].placement().geometry().bounds())
            }
            Paint::FootnoteSeparator(_) => return Err(E::Identity),
        };
        if let Some(next) = next {
            bounds = Some(match bounds {
                None => next,
                Some(prior) => union(prior, next)?,
            });
        }
    }
    Ok(bounds)
}
fn string_hash(b: &mut Budget, mut fp: [u8; 32], s: &str) -> Result<[u8; 32], E> {
    fp = number(b, fp, Some(s.len()))?;
    for part in s.as_bytes().chunks(96) {
        fp = b.fold(fp, part)?;
    }
    Ok(fp)
}

#[path = "book_v2_pdf_assembly.rs"]
mod pdf_assembly;
pub use pdf_assembly::*;
