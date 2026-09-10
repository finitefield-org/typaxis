//! Bounded backtracking for newly demanded notes inside selected definitions.
//! Every accepted demand is caused by an actual retained fragment. Provisional
//! references never survive a backtrack, and definition cycles do not recurse.
use super::*;
struct Frame<'b, 'f, 's, 'p, 'a> {
    state: BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    arrived: Option<BookV2FootnoteRegionFragment<'b, 'f, 's, 'p, 'a>>,
    used: Length,
    last_after: Option<Length>,
    forced: Option<NodeId>,
    cursor: Option<BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a>>,
    position: usize,
    offset: Length,
    capacity: Length,
    choice: Option<page_breaks::BoundarySelection>,
    last_key: Option<(i64, u32)>,
    mixed: Option<std::vec::IntoIter<BookV2RankedDefinitionCandidate<'b, 'f, 's, 'p, 'a>>>,
    exhausted: bool,
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    fn dependency_frame(
        &mut self,
        state: BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        arrived: Option<BookV2FootnoteRegionFragment<'b, 'f, 's, 'p, 'a>>,
        used: Length,
        last_after: Option<Length>,
        forced: Option<NodeId>,
        available: Length,
        visited: &[bool],
        entry: Option<&BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>>,
    ) -> Result<Frame<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        self.verify_state(&state)?;
        self.content.charge(1, root)?;
        let mut next = None;
        for (position, &index) in state.pending.iter().enumerate() {
            self.content.step(root)?;
            if !visited[index]
                && entry.is_none_or(|state| {
                    state.status(index) == Some(ProductionFootnoteDemandStatus::Pending)
                })
            {
                next = Some((position, index));
                break;
            }
        }
        let mut frame = Frame {
            state,
            arrived,
            used,
            last_after,
            forced,
            cursor: None,
            position: 0,
            offset: used,
            capacity: Length::ZERO,
            choice: None,
            last_key: None,
            mixed: None,
            exhausted: false,
        };
        let Some((position, index)) = next else {
            return Ok(frame);
        };
        let BookV2Demand::Pending { cursor, .. } = frame.state.definitions[index] else {
            return Err(error(root, E::ReceiptMismatch));
        };
        frame.cursor = Some(cursor);
        frame.position = position;
        if forced.is_some() {
            frame.exhausted = true;
            return Ok(frame);
        }
        let items = self
            .content
            .flow
            .definition_items(index)
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let (owner, paint, before) = if self.definition_tables.is_some() {
            self.mixed_definition_start_at(&frame.state, index)?
        } else {
            let first = items
                .get(cursor.next_item())
                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
            (first.owner, first.source.is_some(), first.before)
        };
        let gap = if paint {
            last_after
                .map(|v| add(v, before, owner))
                .transpose()?
                .unwrap_or(Length::ZERO)
        } else {
            Length::ZERO
        };
        frame.offset = add(used, gap, owner)?;
        if frame.offset > available {
            frame.exhausted = true;
            return Ok(frame);
        }
        frame.capacity = available
            .checked_sub(frame.offset)
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
        if self.definition_tables.is_some() {
            let choices = self.enumerate_definition(&frame.state, index, frame.capacity)?;
            choices.verify_demand(&frame.state)?;
            frame.mixed = Some(choices.into_choices());
            return Ok(frame);
        }
        if paint {
            let content = self.content.kernel();
            frame.choice = match page_breaks::all_boundaries(
                items,
                cursor.next_item(),
                frame.capacity,
                content.paragraph_lengths,
                content.headings,
                content.maximum_candidates,
                content.charge,
                |owner| visit(content.steps, content.maximum_steps, owner),
            ) {
                Ok(choice) => Some(choice),
                Err(e) if e.kind == E::Oversize && frame.capacity < content.maximum_height => {
                    frame.exhausted = true;
                    None
                }
                Err(e) => return Err(e),
            };
        }
        Ok(frame)
    }
    /// Used only after the common reservation leaves a newly referenced note
    /// unstarted. Candidate/work exhaustion remains an error, never a partial fit.
    pub(in crate::production_body::body_flow) fn select_dependency_closed_region(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.select_definition_reservation(state, available, true)
    }
    pub(super) fn select_definition_reservation(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        available: Length,
        close_dependencies: bool,
    ) -> Result<
        Option<BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        let root = NodeId::new(0);
        self.verify_state(state)?;
        if available < Length::ZERO || available > self.content.maximum_height {
            return Err(error(root, E::InvalidFootnoteCapacity));
        }
        if state.pending.is_empty() {
            return Ok(None);
        }
        let count = state.definitions.len();
        self.content.charge(
            count
                .checked_add(1)
                .ok_or_else(|| error(root, E::ArithmeticOverflow))?,
            root,
        )?;
        let mut visited = Vec::new();
        visited
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        for _ in 0..count {
            self.content.step(root)?;
            visited.push(false);
        }
        let mut stack = Vec::new();
        self.content.charge(1, root)?;
        stack
            .try_reserve_exact(1)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let initial = self.fork(state, 0)?;
        stack.push(self.dependency_frame(
            initial,
            None,
            Length::ZERO,
            None,
            None,
            available,
            &visited,
            (!close_dependencies).then_some(state),
        )?);
        while !stack.is_empty() {
            self.content.step(root)?;
            let frame = stack
                .last_mut()
                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
            let Some(cursor) = frame.cursor else {
                let mut terminal = stack.pop().ok_or_else(|| error(root, E::ReceiptMismatch))?;
                if close_dependencies && self.definition_tables.is_some() {
                    let mut closed = true;
                    for arrived in stack
                        .iter()
                        .filter_map(|frame| frame.arrived.as_ref())
                        .chain(terminal.arrived.as_ref())
                    {
                        self.query_work()?;
                        for reference in arrived.fragment().references() {
                            self.content.step(reference.source().owner())?;
                            if !terminal
                                .state
                                .definition_started(reference.source().definition_index())
                            {
                                closed = false;
                            }
                        }
                    }
                    if !closed {
                        if let Some(arrived) = terminal.arrived.take() {
                            visited[arrived.fragment().definition_index()] = false;
                        }
                        continue;
                    }
                }
                let length = stack.len();
                self.content.charge(length, root)?;
                let mut fragments = Vec::new();
                fragments
                    .try_reserve_exact(length)
                    .map_err(|_| error(root, E::AllocationFailure))?;
                for mut frame in stack {
                    self.content.step(root)?;
                    if let Some(fragment) = frame.arrived.take() {
                        fragments.push(fragment);
                    }
                }
                if let Some(fragment) = terminal.arrived.take() {
                    fragments.push(fragment);
                }
                if fragments.len() != length {
                    return Err(error(root, E::ReceiptMismatch));
                }
                return Ok(Some(BookV2FootnoteRegionSelection {
                    owner_id: self.owner_id,
                    state_id: state.state_id,
                    fragments,
                    used_height: terminal.used,
                    available_height: available,
                    forced_break_owner: terminal.forced,
                    next_state: terminal.state,
                }));
            };
            let items = self
                .content
                .flow
                .definition_items(cursor.definition_index())
                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
            let start = cursor.next_item();
            let mut candidate = None;
            let mixed = if let Some(choices) = &mut frame.mixed {
                let next = choices.next();
                frame.exhausted |= next.is_none();
                next
            } else {
                None
            };
            if !frame.exhausted && frame.mixed.is_none() {
                if let Some(choice) = &frame.choice {
                    for (i, c) in choice.candidates.iter().enumerate() {
                        self.content.step(c.owner())?;
                        let key = (c.costs().total(), c.end_item());
                        if frame.last_key.is_none_or(|last| key > last)
                            && candidate.is_none_or(|(_, prior)| key < prior)
                        {
                            candidate = Some((i, key));
                        }
                    }
                    if candidate.is_none() {
                        frame.exhausted = true;
                    }
                }
            }
            if frame.exhausted {
                let failed = stack.pop().ok_or_else(|| error(root, E::ReceiptMismatch))?;
                if let Some(arrived) = failed.arrived {
                    visited[arrived.fragment().definition_index()] = false;
                }
                continue;
            }
            let selected = if let Some(mixed) = mixed {
                self.mixed_definition_selection(
                    &frame.state,
                    mixed.into_candidate(),
                    frame.capacity,
                )?
            } else {
                let projection = if let Some((index, key)) = candidate {
                    frame.last_key = Some(key);
                    let choice = frame
                        .choice
                        .as_ref()
                        .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                    let chosen = choice.candidates[index];
                    let end = chosen.end_item() as usize;
                    self.query_work()?;
                    if !fit_kernel::complete_references(
                        self.content
                            .flow
                            .references_in_items(Some(cursor.definition_index()), start..end),
                        start..end,
                        &mut self.content.steps,
                        self.content.maximum_steps,
                    )? {
                        continue;
                    }
                    let reason = if end == items.len() {
                        ProductionBodyBreakReason::End
                    } else if items[end].source.is_none() {
                        ProductionBodyBreakReason::Forced
                    } else {
                        ProductionBodyBreakReason::Overflow
                    };
                    let forced =
                        (reason == ProductionBodyBreakReason::Forced).then(|| items[end].owner);
                    self.content.charge(
                        choice
                            .candidates
                            .len()
                            .checked_add(1)
                            .ok_or_else(|| error(root, E::ArithmeticOverflow))?,
                        root,
                    )?;
                    let mut candidates = Vec::new();
                    candidates
                        .try_reserve_exact(choice.candidates.len())
                        .map_err(|_| error(root, E::AllocationFailure))?;
                    for c in &choice.candidates {
                        self.content.step(c.owner())?;
                        candidates.push(*c);
                    }
                    FootnoteFragmentProjection {
                        content_end: end,
                        consumed_end: end + usize::from(forced.is_some()),
                        used_height: chosen.used_height(),
                        available_height: frame.capacity,
                        reason,
                        forced_break_owner: forced,
                        choice: Some(page_breaks::BoundarySelection {
                            start_item: choice.start_item,
                            selected_candidate: index as u32,
                            reason,
                            candidates,
                        }),
                    }
                } else {
                    frame.exhausted = true;
                    self.content
                        .kernel()
                        .evaluate(items, start, frame.capacity)?
                        .ok_or_else(|| error(root, E::ReceiptMismatch))?
                };
                BookV2FootnoteDemandSelection {
                    owner_id: self.owner_id,
                    state_id: frame.state.state_id,
                    fragment: BookV2FootnoteFragmentSelection::from_projection(cursor, projection),
                }
            };
            let fragment = selected.fragment();
            // A parallel sibling may advance while the first marked leaf is
            // still absent. Such content cannot reserve this note's first
            // fragment. Authored forced-only boundaries retain their explicit
            // nonpainting progress semantics.
            if self.definition_tables.is_some()
                && !cursor.definition_started()
                && fragment.marker().is_none()
                && fragment.forced_break_owner().is_none()
            {
                continue;
            }
            let used = add(frame.offset, fragment.used_height(), root)?;
            let forced = fragment.forced_break_owner();
            let last_after = fragment.space_after().or(frame.last_after);
            let next = self.advance_at(&frame.state, &selected, frame.position)?;
            visited[cursor.definition_index()] = true;
            let arrived = BookV2FootnoteRegionFragment {
                offset: frame.offset,
                selected,
            };
            let child = self.dependency_frame(
                next,
                Some(arrived),
                used,
                last_after,
                forced,
                available,
                &visited,
                (!close_dependencies).then_some(state),
            )?;
            if stack.len() > count {
                return Err(error(root, E::ReceiptMismatch));
            }
            self.content.charge(1, root)?;
            stack
                .try_reserve_exact(1)
                .map_err(|_| error(root, E::AllocationFailure))?;
            stack.push(child);
        }
        Ok(None)
    }
}
