use super::*;
use typaxis_pagination::book_v2::{BookV2BodyMixedPlacedSequence, BookV2TableMeasurements};

pub(super) fn verify_geometry(
    geometry: &BookV2BodyMixedPlacedSequence<'_, '_, '_, '_, '_, '_>,
    base: &BookV2TableMeasurements<'_, '_, '_, '_>,
) {
    let mut copies = 0;
    for page in geometry.pages() {
        let mut last = None;
        for variant in page.header_variants() {
            assert!(last.is_none_or(|p| p < variant.fragment_index()));
            last = Some(variant.fragment_index());
            assert!(std::ptr::eq(
                page.header_variant(variant.fragment_index()).unwrap(),
                variant
            ));
            let placed = &page.fragments()[variant.fragment_index()];
            assert!(std::ptr::eq(
                variant.measurements(),
                variant.header().variant()
            ));
            assert_eq!(
                placed.definition_index(),
                variant.header().definition_index()
            );
            let item = variant
                .measurements()
                .item(variant.global_item_index())
                .unwrap();
            assert_eq!(item.owner(), placed.fragment().owner());
            assert_eq!(item.source(), Some(placed.fragment().source()));
            assert!(
                page.fragments_with_roles()
                    .nth(variant.fragment_index())
                    .unwrap()
                    .2
            );
            let mut found = false;
            let mut inspect =
                |table: &typaxis_pagination::book_v2::BookV2TableFragmentSelection<
                    '_,
                    '_,
                    '_,
                    '_,
                    '_,
                >,
                 origin: Length| {
                    if !table
                        .header_variant()
                        .is_some_and(|h| std::ptr::eq(h, variant.header()))
                    {
                        return;
                    }
                    for leaf in table.variant_placement_leaves() {
                        let leaf = leaf.unwrap();
                        if leaf.uses_header_variant()
                            && leaf.global_item_index() == variant.global_item_index()
                        {
                            assert!(!found);
                            found = true;
                            assert_eq!(
                                placed.fragment().bounds().y(),
                                origin.checked_add(leaf.top()).unwrap()
                            );
                            let role = page.cell_roles()[variant.fragment_index()];
                            assert_eq!(role.map(|r| r.owner()), leaf.cell_owner());
                        }
                    }
                };
            for part in page.selection().candidate().parts() {
                if let Some(table) = part.table() {
                    inspect(
                        table,
                        page.selection()
                            .body_bounds()
                            .y()
                            .checked_add(part.top())
                            .unwrap(),
                    );
                }
            }
            if let Some(notes) = page.selection().candidate().footnotes() {
                for note in notes.fragments() {
                    if let Some(mixed) = note.fragment().mixed() {
                        let start = page
                            .selection()
                            .candidate()
                            .footnote_bounds()
                            .unwrap()
                            .y()
                            .checked_add(
                                Length::from_raw(typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW)
                                    .unwrap(),
                            )
                            .unwrap()
                            .checked_add(note.offset())
                            .unwrap();
                        for part in mixed.parts() {
                            if let Some(table) = part.table() {
                                inspect(table, start.checked_add(part.top()).unwrap());
                            }
                        }
                    }
                }
            }
            assert!(found);
            copies += 1;
        }
        for (index, placed) in page.fragments().iter().enumerate() {
            let flow = page
                .header_variant(index)
                .map_or(base.flow(), |v| v.measurements().flow());
            let items = placed
                .definition_index()
                .map_or_else(|| flow.body_items(), |d| flow.definition_items(d).unwrap());
            let item = &items[placed.item_index()];
            assert_eq!(item.owner(), placed.fragment().owner());
            assert_eq!(item.source(), Some(placed.fragment().source()));
            let frames = flow.lines().frames().unwrap();
            let (actual, measured) = if placed.definition_index().is_some() {
                (
                    page.selection().declared_footnote_region().unwrap(),
                    frames.footnote_region().unwrap(),
                )
            } else {
                (page.selection().body_bounds(), frames.body())
            };
            let delta = actual.x().checked_sub(measured.x()).unwrap();
            let fragment = placed.fragment();
            assert_eq!(fragment.bounds().x(), item.x().checked_add(delta).unwrap());
            assert_eq!(fragment.bounds().width(), item.width());
            assert_eq!(fragment.bounds().height().get(), item.height());
            assert_eq!(fragment.page_index(), page.selection().page_index());
            if let typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } = fragment.source()
            {
                let line = &flow.lines().paragraphs()[paragraph_index as usize].lines()
                    [line_index as usize];
                assert_eq!(
                    fragment.baseline(),
                    Some(fragment.bounds().y().checked_add(line.baseline()).unwrap())
                );
            }
        }
        for marker in page.list_markers() {
            let index = marker.fragment_index() as usize;
            let placed = &page.fragments()[index];
            let flow = page
                .header_variant(index)
                .map_or(base.flow(), |v| v.measurements().flow());
            let shape =
                &flow.lines().prepared().shaped().list_markers()[marker.marker_index() as usize];
            assert_eq!(shape.source().owner(), marker.owner());
            let frames = flow.lines().frames().unwrap();
            let column = &frames.lists()[shape.source().list_index() as usize];
            let (actual, measured) = if placed.definition_index().is_some() {
                (
                    page.selection().declared_footnote_region().unwrap(),
                    frames.footnote_region().unwrap(),
                )
            } else {
                (page.selection().body_bounds(), frames.body())
            };
            let delta = actual.x().checked_sub(measured.x()).unwrap();
            let slack = column
                .marker_width()
                .get()
                .checked_sub(shape.advance().get())
                .unwrap();
            assert_eq!(
                marker.bounds().x(),
                frames
                    .body()
                    .x()
                    .checked_add(column.marker_start())
                    .unwrap()
                    .checked_add(slack)
                    .unwrap()
                    .checked_add(delta)
                    .unwrap()
            );
            assert_eq!(marker.bounds().width(), shape.advance());
            assert_eq!(Some(marker.baseline()), placed.fragment().baseline());
        }
        for marker in page.footnote_markers() {
            assert!(page
                .header_variant(marker.fragment_index() as usize)
                .is_none());
            let placed = &page.fragments()[marker.fragment_index() as usize];
            let binding = base
                .flow()
                .definition_marker(marker.definition_index())
                .unwrap();
            assert_eq!(binding.item_index(), placed.item_index());
            assert_eq!(
                marker.baseline(),
                placed
                    .fragment()
                    .bounds()
                    .y()
                    .checked_add(binding.baseline())
                    .unwrap()
            );
        }
    }
    assert!(copies > 0);
}
#[test]
fn book_v2_table_header_placement_preserves_body_note_and_marker_geometry() {
    super::header_catalog::check(None, true);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_header_placement_preserves_original_harano_geometry() {
    super::header_catalog::check(
        Some(&fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap()),
        true,
    );
}

pub(super) fn verify_width_feedback(
    geometry: &BookV2BodyMixedPlacedSequence<'_, '_, '_, '_, '_, '_>,
    base: &BookV2TableMeasurements<'_, '_, '_, '_>,
    report: &typaxis_pagination::book_v2::BookV2TableWidthOccurrences,
    feedback: &typaxis_pagination::book_v2::BookV2ParagraphWidthFeedback,
) {
    use typaxis_pagination::book_v2::BookV2TableWidthSource;
    let mut different_repeat = false;
    let mut checked = 0;
    for occurrence in report.occurrences() {
        let page = geometry
            .pages()
            .iter()
            .find(|p| p.selection().page_index() == occurrence.page_index())
            .unwrap();
        let actual = page
            .header_variants()
            .iter()
            .filter(|v| {
                v.header().definition_index() == occurrence.definition_index()
                    && v.measurements().tables()[v.header().table_index()].owner()
                        == occurrence.owner()
            })
            .collect::<Vec<_>>();
        let observed = occurrence
            .pieces()
            .iter()
            .filter(|p| p.uses_header_variant())
            .collect::<Vec<_>>();
        assert_eq!(actual.len(), observed.len());
        for (actual, piece) in actual.into_iter().zip(observed) {
            let flow = actual.measurements().flow();
            let item = actual
                .measurements()
                .item(actual.global_item_index())
                .unwrap();
            let typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } = item.source().unwrap()
            else {
                panic!("this dedicated fixture contains paragraph/list header leaves");
            };
            let selected = &flow.lines().paragraphs()[paragraph_index as usize]
                .selected()
                .unwrap()
                .lines()[line_index as usize];
            assert_eq!(
                piece.source(),
                &BookV2TableWidthSource::Paragraph {
                    owner: item.owner(),
                    units: selected.line().start_unit()..selected.line().end_unit(),
                }
            );
            let frames = flow.lines().frames().unwrap();
            let start = frames
                .source_unit_start(paragraph_index as usize, selected.line().start_unit())
                .unwrap_or(frames.paragraphs()[paragraph_index as usize].start());
            assert_eq!(
                piece.measured_header_frame(),
                Some((start, selected.inline_size()))
            );
            let target = piece.frame().unwrap();
            let candidate = &feedback.paragraphs()[paragraph_index as usize];
            for unit in selected.line().start_unit() as usize..selected.line().end_unit() as usize {
                different_repeat |= candidate.widths()[unit] != target.width()
                    || candidate.source_unit_starts().unwrap()[unit] != target.start();
            }
            checked += 1;
        }
        // Each original source interval receives its physical semantic frame.
        // Later copies at different widths must not overwrite this assignment.
        for piece in occurrence.pieces().iter().filter(|p| !p.repeated()) {
            if let BookV2TableWidthSource::Paragraph { owner, units } = piece.source() {
                let index = base
                    .flow()
                    .lines()
                    .paragraphs()
                    .iter()
                    .position(|p| p.owner() == *owner)
                    .unwrap();
                let candidate = &feedback.paragraphs()[index];
                let frame = piece.frame().unwrap();
                let end = if units.is_empty() {
                    1
                } else {
                    units.end as usize
                };
                for unit in units.start as usize..end {
                    assert_eq!(candidate.widths()[unit], frame.width());
                    assert_eq!(candidate.source_unit_starts().unwrap()[unit], frame.start());
                }
            }
        }
    }
    assert_eq!(
        checked,
        geometry
            .pages()
            .iter()
            .map(|p| p.header_variants().len())
            .sum::<usize>()
    );
    assert!(
        different_repeat,
        "both actual header widths must remain separate from base source assignments"
    );
}
