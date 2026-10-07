//! Reproject original table leaves in each selected body column or note region.
use super::*;

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    fn column_table_parent_width(
        &mut self,
        index: usize,
        actual: Rect,
    ) -> Result<PositiveLength, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let table = self
            .content
            .flow
            .collected
            .tables
            .tables
            .get(index)
            .filter(|t| t.parent.is_none())
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let frames = self
            .content
            .flow
            .lines()
            .frames()
            .ok_or_else(|| error(table.owner, E::ReceiptMismatch))?;
        for _ in 0..frames.measurement_region_lookup_work() {
            self.content.step(table.owner)?;
        }
        let original = frames
            .measurement_region(table.owner)
            .ok_or_else(|| error(table.owner, E::ReceiptMismatch))?;
        let measured = if table.definition.is_some() {
            frames
                .footnote_region()
                .ok_or_else(|| error(table.owner, E::ReceiptMismatch))?
        } else {
            frames.body()
        };
        actual
            .width()
            .get()
            .checked_sub(measured.width().get())
            .and_then(|delta| original.width().get().checked_add(delta))
            .and_then(PositiveLength::new)
            .ok_or_else(|| error(table.owner, E::WidthMismatch))
    }

    pub(in crate::production_body::body_flow) fn collect_column_table_width_frames(
        &mut self,
        sequence: &BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2TableWidthOccurrences, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let mut count = 0usize;
        for page in sequence.pages() {
            self.content.step(root)?;
            for column in page.candidate().columns() {
                self.content.step(root)?;
                for part in column.parts() {
                    self.content.step(root)?;
                    if part.table().is_some() {
                        count = count
                            .checked_add(1)
                            .ok_or_else(|| error(root, E::FragmentLimit))?;
                    }
                }
            }
            if let Some(notes) = page.candidate().footnotes() {
                for selected in notes.fragments() {
                    self.content.step(root)?;
                    if let Some(mixed) = selected.fragment().mixed() {
                        for part in mixed.parts() {
                            self.content.step(root)?;
                            if part.table().is_some() {
                                count = count
                                    .checked_add(1)
                                    .ok_or_else(|| error(root, E::FragmentLimit))?;
                            }
                        }
                    }
                }
            }
        }
        self.content.charge.take(
            count
                .checked_add(1)
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut occurrences = Vec::new();
        occurrences
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let source = self
            .content
            .flow
            .lines()
            .prepared()
            .source_flow()
            .fingerprint();
        let mut report = BookV2TableWidthOccurrences {
            source,
            remeasured: true,
            fingerprint: source,
            occurrences,
            records: 0,
            work: 0,
        };
        self.fold_table_occurrence(&mut report.fingerprint, 0x434f4c54414231, root)?;
        for page in sequence.pages() {
            self.content.step(root)?;
            for (column_index, column) in page.candidate().columns().iter().enumerate() {
                self.content.step(root)?;
                self.fold_table_occurrence(&mut report.fingerprint, column_index as u64, root)?;
                for part in column.parts() {
                    self.content.step(root)?;
                    if let Some(table) = part.table() {
                        self.append_table_occurrence_in_frame(
                            &mut report,
                            page.page_index(),
                            table,
                            None,
                            |search, index| {
                                search.column_table_parent_width(index, column.bounds())
                            },
                        )?;
                    }
                }
            }
            if let Some(notes) = page.candidate().footnotes() {
                let bounds = page
                    .candidate()
                    .frames()
                    .footnote()
                    .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                for selected in notes.fragments() {
                    self.content.step(root)?;
                    let fragment = selected.fragment();
                    if let Some(mixed) = fragment.mixed() {
                        for part in mixed.parts() {
                            self.content.step(root)?;
                            if let Some(table) = part.table() {
                                self.append_table_occurrence_in_frame(
                                    &mut report,
                                    page.page_index(),
                                    table,
                                    Some(fragment.definition_index()),
                                    |search, index| search.column_table_parent_width(index, bounds),
                                )?;
                            }
                        }
                    }
                }
            }
        }
        if report.occurrences.len() != count {
            return Err(error(root, E::ReceiptMismatch));
        }
        // Empty root tables also need their own occurrence; unreferenced note
        // definitions remain unobserved. Nested tables belong to their root.
        let tables = &self.content.flow.collected.tables.tables;
        self.content.charge.take(tables.len(), root)?;
        let mut seen = Vec::new();
        seen.try_reserve_exact(tables.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        for _ in tables {
            self.content.step(root)?;
            seen.push(false);
        }
        for occurrence in report.occurrences() {
            let mut found = false;
            for (index, table) in tables.iter().enumerate() {
                self.content.step(occurrence.owner())?;
                if table.owner == occurrence.owner() {
                    if table.parent.is_some() || table.definition != occurrence.definition_index() {
                        return Err(error(table.owner, E::ReceiptMismatch));
                    }
                    seen[index] = true;
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(error(occurrence.owner(), E::ReceiptMismatch));
            }
        }
        let state = sequence
            .pages()
            .last()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?
            .next_state()
            .source_state();
        for (table, seen) in tables.iter().zip(seen) {
            self.content.step(table.owner)?;
            if table.parent.is_none() {
                let expected = table.definition.is_none_or(|d| {
                    state.definition_status(d) == Some(ProductionFootnoteDemandStatus::Complete)
                });
                if expected != seen {
                    return Err(error(table.owner, E::ReceiptMismatch));
                }
            }
        }
        self.fold_table_occurrence(&mut report.fingerprint, count as u64, root)?;
        report.records = self.record_charge();
        report.work = self.work_steps();
        Ok(report)
    }
}
