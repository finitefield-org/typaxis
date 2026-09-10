//! Ordered source selection shared by body and footnote definition streams.
use super::*;

pub(crate) trait SourceSearch<'b> {
    type State;
    type TableFragment;
    fn verify(&self, state: &Self::State) -> Result<(), ProductionBodyPaginationError>;
    fn items(&self) -> &'b [ProductionBodyFlowItem];
    fn maximum(&self) -> Length;
    fn charge(&mut self, count: usize, owner: NodeId) -> Result<(), ProductionBodyPaginationError>;
    fn step(&mut self, owner: NodeId) -> Result<(), ProductionBodyPaginationError>;
    fn fork_table(
        &mut self,
        state: &Self::State,
    ) -> Result<Self::State, ProductionBodyPaginationError>;
    fn pending(&self, state: &Self::State) -> bool;
    fn require_range(
        &mut self,
        state: &mut Self::State,
        range: std::ops::Range<usize>,
    ) -> Result<bool, ProductionBodyPaginationError>;
    fn require_table(
        &mut self,
        state: &mut Self::State,
        fragment: &Self::TableFragment,
    ) -> Result<bool, ProductionBodyPaginationError>;
    fn table_height(fragment: &Self::TableFragment) -> Length;
    fn table_forced(_fragment: &Self::TableFragment) -> Option<NodeId> {
        None
    }
    type Cursor: Copy;
    type Request;
    type Part;
    fn request(request: &Self::Request) -> Request<Self::Cursor>;
    fn has_tables(&self) -> bool;
    fn source_keep_before(
        &mut self,
        item: usize,
        _next_table: Option<usize>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        let _ = item;
        Ok(false)
    }
    fn table_query_work(&mut self) -> Result<(), ProductionBodyPaginationError>;
    fn ordinary_range(&self, range: std::ops::Range<usize>, next_table: Option<usize>) -> bool;
    fn table_at(&self, item: usize, next_table: Option<usize>) -> Option<usize>;
    fn table_range(&self, index: usize) -> std::ops::Range<usize>;
    fn next_table(&self, index: usize) -> Result<usize, ProductionBodyPaginationError> {
        index
            .checked_add(1)
            .ok_or_else(|| error(NodeId::new(0), E::ArithmeticOverflow))
    }
    fn table_info(&self, index: usize) -> (NodeId, Length, Length, bool);
    fn remaining_source(&self, item: usize, next_table: Option<usize>) -> bool;
    fn initial(cursor: Self::Cursor) -> bool;
    fn table_index(cursor: Self::Cursor) -> usize;
    fn terminal(cursor: Self::Cursor) -> bool;
    fn after(fragment: &Self::TableFragment) -> Self::Cursor;
    fn evaluate_table(
        &mut self,
        cursor: &Self::Cursor,
        capacity: Length,
    ) -> Result<Option<Self::TableFragment>, ProductionBodyPaginationError>;
    fn part(
        items: Option<std::ops::Range<usize>>,
        table: Option<Self::TableFragment>,
        top: Length,
        height: Length,
    ) -> Self::Part;
}
pub(crate) struct Projection<'b, S: SourceSearch<'b>> {
    pub end: usize,
    pub next_table: Option<usize>,
    pub continuation: Option<S::Cursor>,
    pub parts: Vec<S::Part>,
    pub height: Length,
    pub demanded: S::State,
    pub forced: Option<NodeId>,
}
pub(crate) fn evaluate<'b, S: SourceSearch<'b>>(
    search: &mut S,
    state: &S::State,
    start: usize,
    mut next_table: Option<usize>,
    incoming: Option<S::Cursor>,
    requests: &[S::Request],
) -> Result<Option<Projection<'b, S>>, ProductionBodyPaginationError> {
    search.verify(state)?;
    let root = NodeId::new(0);
    let items = search.items();
    if !search.has_tables() || start > items.len() {
        return Err(error(root, E::ReceiptMismatch));
    }
    search.charge(
        requests
            .len()
            .checked_add(1)
            .ok_or_else(|| error(root, E::FragmentLimit))?,
        root,
    )?;
    let mut parts = Vec::new();
    parts
        .try_reserve_exact(requests.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    let mut demanded = search.fork_table(state)?;
    let mut cursor = start;
    let mut height = Length::ZERO;
    let mut after = Length::ZERO;
    let mut keep = false;
    let mut continuation = incoming;
    let mut prior_items = false;
    let mut forced = None;
    let maximum = search.maximum();
    let continuing = matches!(requests.first().map(S::request),Some(Request::Table {cursor,..}) if !S::initial(cursor));
    if !requests.is_empty() && !continuing && search.source_keep_before(start, next_table)? {
        return Ok(None);
    }
    for (ordinal, request) in requests.iter().enumerate() {
        search.step(root)?;
        match S::request(request) {
            #[cfg(feature = "book-v2-staging")]
            Request::Forced => {
                let item = items
                    .get(cursor)
                    .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                search.table_query_work()?;
                if item.source.is_some()
                    || ordinal + 1 != requests.len()
                    || !search.ordinary_range(cursor..cursor + 1, next_table)
                {
                    return Err(error(item.owner, E::ReceiptMismatch));
                }
                if keep {
                    return Ok(None);
                }
                if !search.require_range(&mut demanded, cursor..cursor + 1)? {
                    return Ok(None);
                }
                parts.push(S::part(
                    Some(cursor..cursor + 1),
                    None,
                    height,
                    Length::ZERO,
                ));
                cursor += 1;
                forced = Some(item.owner);
            }
            Request::Items { end } => {
                let range = cursor..end;
                search.table_query_work()?;
                if prior_items
                    || range.is_empty()
                    || cursor > end
                    || end > items.len()
                    || !search.ordinary_range(range.clone(), next_table)
                {
                    return Err(error(root, E::ReceiptMismatch));
                }
                let first = &items[cursor];
                let gap = if parts.is_empty() {
                    Length::ZERO
                } else {
                    add(after, first.before, first.owner)?
                };
                let top = add(height, gap, first.owner)?;
                let mut used = Length::ZERO;
                for index in range.clone() {
                    let item = &items[index];
                    search.step(item.owner)?;
                    if item.source.is_none() {
                        return Ok(None);
                    }
                    let spacing = if index == cursor {
                        Length::ZERO
                    } else {
                        add(items[index - 1].after, item.before, item.owner)?
                    };
                    used = add(
                        add(used, spacing, item.owner)?,
                        item.consumed()?,
                        item.owner,
                    )?;
                }
                height = add(top, used, first.owner)?;
                if height > maximum || !search.require_range(&mut demanded, range.clone())? {
                    return Ok(None);
                }
                let last = &items[end - 1];
                after = last.after;
                keep = last.keep && search.remaining_source(end, next_table);
                parts.push(S::part(Some(range), None, top, used));
                cursor = end;
                prior_items = true;
            }
            Request::Table {
                cursor: table_cursor,
                capacity,
            } => {
                search.table_query_work()?;
                let index = search
                    .table_at(cursor, next_table)
                    .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                if index != S::table_index(table_cursor)
                    || (!parts.is_empty() && !S::initial(table_cursor))
                {
                    return Err(error(root, E::ReceiptMismatch));
                }
                let (owner, before, table_after, table_keep) = search.table_info(index);
                let gap = if parts.is_empty() {
                    Length::ZERO
                } else {
                    add(after, before, owner)?
                };
                let top = add(height, gap, owner)?;
                if capacity < Length::ZERO
                    || top > maximum
                    || capacity
                        > maximum
                            .checked_sub(top)
                            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?
                {
                    return Err(error(owner, E::InvalidTableCapacity));
                }
                let Some(selected) = search.evaluate_table(&table_cursor, capacity)? else {
                    if requests.len() == 1 && search.pending(state) {
                        continuation = Some(table_cursor);
                        break;
                    }
                    return Ok(None);
                };
                forced = S::table_forced(&selected);
                if forced.is_some() && ordinal + 1 < requests.len() {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                let terminal = S::terminal(S::after(&selected));
                if !terminal && ordinal + 1 < requests.len() {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                let used = S::table_height(&selected);
                height = add(top, used, owner)?;
                if !search.require_table(&mut demanded, &selected)? {
                    return Ok(None);
                }
                after = table_after;
                if terminal {
                    cursor = search.table_range(index).end;
                    if next_table.is_some() {
                        next_table = Some(search.next_table(index)?);
                    }
                    continuation = None;
                    keep = table_keep && search.remaining_source(cursor, next_table);
                } else {
                    continuation = Some(S::after(&selected));
                    keep = false;
                }
                parts.push(S::part(None, Some(selected), top, used));
                prior_items = false;
            }
        }
    }
    if keep {
        return Ok(None);
    }
    Ok(Some(Projection {
        end: cursor,
        next_table,
        continuation,
        parts,
        height,
        demanded,
        forced,
    }))
}
