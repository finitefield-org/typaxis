use super::*;
use typaxis_layout::book_v2::{
    layout_book_v2_source_width_lines_from_flow,
    with_converged_book_v2_body_lines_with_source_widths, BookV2SourceWidthAssignments,
};
use typaxis_pagination::book_v2::*;

#[test]
fn book_v2_block_widths_rebind_original_parent_frames_through_shaping() {
    for align in ["start", "center", "end"] {
        let root = Root::new();
        let limits = limits();
        let input = vector_input(&root, block_data(align, align != "end"), &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let foreign = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let profiles = vec![None; flow.paragraphs().len()];
        let width = |raw| PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap();
        let widths = [
            (NodeId::new(7), width(5_000_000)),
            (NodeId::new(8), width(6_000_000)),
        ];
        let run = |assignments: &BookV2SourceWidthAssignments<'_, '_>, maximum| {
            with_converged_book_v2_body_lines_with_source_widths(
                &policy,
                &flow,
                input.resources(),
                &bindings,
                &limits,
                JapaneseLineBreakMode::Normal,
                body(),
                maximum,
                None,
                limits.base().get().max_line_reshape_passes,
                None,
                Some(assignments),
                |stable| {
                    let lines = stable.lines();
                    let numbers =
                        shape_book_v2_equation_numbers(lines.prepared().shaped(), &limits, 0)
                            .unwrap();
                    let blocks = prepare_book_v2_vector_blocks(lines, numbers.as_ref(), &limits, 0)
                        .unwrap()
                        .unwrap();
                    let frames = lines.frames().unwrap();
                    for (index, block) in blocks.blocks().iter().enumerate() {
                        let owner = widths[index].0;
                        let original = frames.measurement_region(owner).unwrap();
                        let actual = frames.region(owner).unwrap();
                        assert_eq!(original.width().get().raw(), 9_700_000);
                        assert_eq!(actual.start(), original.start());
                        assert_eq!(actual.width(), widths[index].1);
                        assert_eq!(block.inner_frame_left().raw(), 731_072);
                        assert_eq!(
                            block.inner_frame_width().get().raw(),
                            widths[index].1.get().raw() - 327_680
                        );
                        assert_eq!(block.viewport_width().get().raw(), 1_966_080);
                        let slack = block.inner_frame_width().get().raw()
                            - block.viewport_width().get().raw();
                        let offset = match align {
                            "start" => 0,
                            "center" => slack / 2,
                            _ => slack,
                        };
                        assert_eq!(block.viewport_left().raw(), 731_072 + offset);
                        assert!(std::ptr::eq(
                            block.binding(),
                            bindings.receipt(owner).unwrap()
                        ));
                    }
                    let envelopes = lines
                        .paragraphs()
                        .iter()
                        .map(|p| p.inline_size())
                        .collect::<Vec<_>>();
                    assert!(layout_book_v2_source_width_lines_from_flow(
                        lines.prepared(),
                        &envelopes,
                        assignments,
                        maximum,
                        0
                    )
                    .is_err());
                    // Even a component caller without a varying page plan may
                    // not paint these provisional narrower parent frames.
                    let flow = prepare_book_v2_body_flow(
                        lines,
                        Some(&blocks),
                        stable.footnotes(),
                        &limits,
                        0,
                    )
                    .unwrap();
                    let measured = prepare_book_v2_table_measurements(flow, &limits).unwrap();
                    let mut search = prepare_book_v2_table_body_search(
                        &measured,
                        &limits,
                        1_000_000,
                        measured.record_charge(),
                    )
                    .unwrap();
                    let pages = search
                        .select_stable_mixed_pages(limits.base().get().max_layout_passes)
                        .unwrap();
                    let placed = search.place_mixed_pages(pages.sequence()).unwrap();
                    let closed = search.close_mixed_page_sources(&pages, &placed).unwrap();
                    assert!(
                        matches!(search.finalize_mixed_page_math(closed, &limits, 0), Err(e)
                        if e.kind == typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch)
                    );
                    (
                        stable.candidate_steps(),
                        lines.fingerprint(),
                        blocks.fingerprint(),
                    )
                },
            )
        };
        let assignments = BookV2SourceWidthAssignments::new(&flow, &profiles)
            .unwrap()
            .with_block_widths(&widths);
        let full = run(&assignments, 1_000_000).unwrap();
        assert_eq!(run(&assignments, full.0).unwrap(), full);
        assert!(run(&assignments, full.0 - 1).is_err());
        if align == "center" {
            let narrow = [(widths[0].0, widths[0].1), (widths[1].0, width(4_000_000))];
            let assignment = BookV2SourceWidthAssignments::new(&flow, &profiles)
                .unwrap()
                .with_block_widths(&narrow);
            with_converged_book_v2_body_lines_with_source_widths(
                &policy,
                &flow,
                input.resources(),
                &bindings,
                &limits,
                JapaneseLineBreakMode::Normal,
                body(),
                1_000_000,
                None,
                limits.base().get().max_line_reshape_passes,
                None,
                Some(&assignment),
                |stable| {
                    let lines = stable.lines();
                    let numbers =
                        shape_book_v2_equation_numbers(lines.prepared().shaped(), &limits, 0)
                            .unwrap();
                    assert!(matches!(
                        prepare_book_v2_vector_blocks(lines, numbers.as_ref(), &limits, 0),
                        Err(BookV2VectorBlockError::Geometry(_))
                    ));
                },
            )
            .unwrap();
        }
        let foreign = BookV2SourceWidthAssignments::new(&foreign, &profiles)
            .unwrap()
            .with_block_widths(&widths);
        assert!(run(&foreign, 1_000_000).is_err());
        for invalid in [
            vec![widths[1], widths[0]],
            vec![widths[0]],
            vec![widths[0], widths[0]],
            vec![(NodeId::new(1), widths[0].1), widths[1]],
            vec![(widths[0].0, width(10_000_000)), widths[1]],
        ] {
            let invalid = BookV2SourceWidthAssignments::new(&flow, &profiles)
                .unwrap()
                .with_block_widths(&invalid);
            assert!(run(&invalid, 1_000_000).is_err());
        }
    }
}
