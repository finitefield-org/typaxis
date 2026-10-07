use super::*;
use typaxis_core::{Length, PositiveLength};
use typaxis_layout::book_v2::*;
use typaxis_linebreak::JapaneseLineBreakMode;
use typaxis_pagination::book_v2::prepare_book_v2_body_flow_counted;
use typaxis_pagination::ProductionBodyPaginationErrorKind;
use typaxis_syntax::book_v2::{prepare_book_v2_column_frame_plan, BookV2ColumnFramePlan};

const MODE: JapaneseLineBreakMode = JapaneseLineBreakMode::Normal;

fn input(
    root: &Root,
    mut data: Value,
    text: &str,
    font: Option<&[u8]>,
    limits: &M4EffectiveResourceLimits,
) -> PreparedBookV2Resources {
    data["page_masters"]["masters"][0]["column_layout"] =
        json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"last_page"});
    if let Some(font) = font {
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            .into();
        let body = body_with_source(root, data, text.as_bytes(), limits);
        fs::write(root.0.join("body.bin"), font).unwrap();
        prepare_book_v2_resources(
            body,
            &root.context(),
            &config(limits.base().get().clone()),
            limits,
        )
        .unwrap()
    } else {
        prepared(root, data, text.as_bytes(), limits)
    }
}

fn actual_text(paragraph: &typaxis_layout::ProductionInlineParagraphLineLayout<'_, '_>) -> String {
    let mut text = String::new();
    for line in paragraph.lines() {
        for item in line.items() {
            if let typaxis_layout::ProductionPlacedInline::Text(cluster) = item {
                text.push_str(cluster.utf8());
                for glyph in cluster.glyphs() {
                    assert!(glyph.glyph().original_gid.get() > 0);
                    assert!(std::ptr::eq(
                        glyph.glyph(),
                        &cluster.run().glyph_run().glyphs[glyph.glyph_index() as usize]
                    ));
                }
            }
        }
    }
    text
}

fn check_plain(font: Option<&[u8]>) {
    let text = if font.is_some() {
        "元の字形を保ち段の幅で本文を組み直す元の字形を保ち段の幅で本文を組み直す"
    } else {
        "Result Result Result Result Result Result Result Result"
    };
    let root = Root::new();
    let limits = limits();
    let data = super::body_line_budget::wide_data(text);
    let input = input(&root, data.clone(), text, font, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let prior = command_source_record_charge(&flow) + plan.record_charge();
    let maximum_records = limits.base().get().max_fragments;
    let passes = limits.base().get().max_line_reshape_passes;
    let callbacks = std::cell::Cell::new(0);
    let run = |budget: &mut BookV2BodyLineBudget,
               columns: &BookV2ColumnFramePlan<'_>,
               assignments: Option<&BookV2SourceWidthAssignments<'_, '_>>| {
        with_budgeted_book_v2_column_lines(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            MODE,
            None,
            budget,
            columns,
            assignments,
            |stable| {
                callbacks.set(callbacks.get() + 1);
                let lines = stable.lines();
                let frames = lines.frames().unwrap();
                assert!(std::ptr::eq(stable.column_plan(), columns));
                assert!(frames.page_plan().is_none());
                assert_eq!(frames.body(), columns.measurement_body());
                assert_eq!(actual_text(&lines.paragraphs()[0]), text);
                assert!(!stable.passes().is_empty());
                let full = layout_book_v2_body_inline_lines(
                    lines.prepared(),
                    columns.page(0).unwrap().body(),
                    1_000_000,
                )
                .unwrap();
                assert!(lines.paragraphs()[0].lines().len() > full.paragraphs()[0].lines().len());
                let mut observed = 0;
                assert!(matches!(prepare_book_v2_body_flow_counted(
                    lines, None, stable.footnotes(), &limits, 17, &mut observed),
                    Err(e) if e.kind == ProductionBodyPaginationErrorKind::PendingRegion("column_pages")));
                assert_eq!(observed, 17.max(stable.footnotes().record_charge()));
                (
                    lines.fingerprint(),
                    stable.candidate_steps(),
                    stable.passes().len() as u16,
                    stable.source_record_charge(),
                    stable.retained_record_charge(),
                    lines.prepared().paragraphs()[0]
                        .items()
                        .unwrap()
                        .units()
                        .len(),
                    lines.paragraphs()[0].inline_size(),
                    lines.paragraphs()[0].lines().len(),
                    lines.paragraphs()[0].line_inline_size(0).unwrap(),
                )
            },
        )
    };
    let mut budget =
        BookV2BodyLineBudget::new_with_source_records(1_000_000, passes, prior, maximum_records);
    let expected = run(&mut budget, &plan, None).unwrap();
    assert_eq!(expected.3, budget.source_record_charge());
    assert_eq!(expected.4, budget.record_charge());
    assert!(expected.3 > prior && expected.4 > 0);
    let mut exact = BookV2BodyLineBudget::new_with_source_records(
        expected.1,
        expected.2,
        prior,
        maximum_records,
    );
    assert_eq!(run(&mut exact, &plan, None).unwrap(), expected);
    assert_eq!((exact.remaining_steps(), exact.remaining_passes()), (0, 0));
    let before = callbacks.get();
    let mut short = BookV2BodyLineBudget::new_with_source_records(
        expected.1 - 1,
        passes,
        prior,
        maximum_records,
    );
    assert!(run(&mut short, &plan, None).is_err());
    let retained = (
        short.candidate_steps(),
        short.reshape_passes(),
        short.source_record_charge(),
        short.record_charge(),
    );
    assert!(retained.0 > 0 && retained.1 > 0 && retained.2 > prior && retained.3 > 0);
    assert!(run(&mut short, &plan, None).is_err());
    assert!(short.candidate_steps() >= retained.0 && short.reshape_passes() >= retained.1);
    assert!(short.source_record_charge() >= retained.2 && short.record_charge() >= retained.3);
    let mut no_pass = BookV2BodyLineBudget::new(1_000_000, expected.2 - 1);
    assert!(run(&mut no_pass, &plan, None).is_err());
    let mut no_records =
        BookV2BodyLineBudget::new_with_source_records(1_000_000, passes, prior, prior);
    assert!(run(&mut no_records, &plan, None).is_err());
    assert_eq!(no_records.source_record_charge(), prior);
    assert_eq!(callbacks.get(), before);

    let width =
        PositiveLength::new(Length::from_raw(expected.6.get().raw() * 3 / 4).unwrap()).unwrap();
    let widths = vec![width; expected.5];
    let profiles = [Some(widths.as_slice())];
    let assigned = BookV2SourceWidthAssignments::new(&flow, &profiles).unwrap();
    let mut budget = BookV2BodyLineBudget::new(1_000_000, passes);
    let narrowed = run(&mut budget, &plan, Some(&assigned)).unwrap();
    assert!(narrowed.7 >= expected.7);
    assert_eq!(narrowed.8, width);
    assert_ne!(narrowed.0, expected.0);
    let foreign_flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let foreign = BookV2SourceWidthAssignments::new(&foreign_flow, &profiles).unwrap();
    let before = callbacks.get();
    let mut budget = BookV2BodyLineBudget::new(1_000_000, passes);
    assert!(run(&mut budget, &plan, Some(&foreign)).is_err());
    assert_eq!(budget.candidate_steps(), 0);

    let other_root = Root::new();
    let other = self::input(&other_root, data, text, font, &limits);
    let nav = prepare_book_v2_navigation(other.body().styled()).unwrap();
    let other_flow = prepare_book_v2_text_flow(other.body().styled(), &nav).unwrap();
    let other_plan =
        prepare_book_v2_column_frame_plan(&other_flow, &mut 0, 1_000_000, 0, 0).unwrap();
    let mut untouched =
        BookV2BodyLineBudget::new_with_source_records(1_000_000, passes, prior, maximum_records);
    assert!(run(&mut untouched, &other_plan, None).is_err());
    assert_eq!(
        (
            untouched.candidate_steps(),
            untouched.reshape_passes(),
            untouched.record_charge()
        ),
        (0, 0, 0)
    );
    assert_eq!(untouched.source_record_charge(), prior);
    assert_eq!(callbacks.get(), before);
}

fn check_tables(font: Option<&[u8]>) {
    let text = if font.is_some() {
        "表の本文を元の字形で組む"
    } else {
        "Pro Pro Pro"
    };
    for notes in [false, true] {
        let root = Root::new();
        let limits = limits();
        let input = input(
            &root,
            super::table_width_frames::table_data(text, notes),
            text,
            font,
            &limits,
        );
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        assert_eq!(plan.measurement_body().width().get().raw(), 115 * 65536);
        let mut budget =
            BookV2BodyLineBudget::new(100_000_000, limits.base().get().max_line_reshape_passes);
        with_budgeted_book_v2_column_lines(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            MODE,
            None,
            &mut budget,
            &plan,
            None,
            |stable| {
                let lines = stable.lines();
                let frames = lines.frames().unwrap();
                assert_eq!(frames.tables().len(), 4);
                assert_eq!(stable.footnotes().definitions().len(), usize::from(notes));
                if notes {
                    assert_eq!(
                        frames.footnote_region().unwrap().width().get().raw(),
                        230 * 65536
                    );
                }
                for index in [0, 2] {
                    let outer = &frames.tables()[index];
                    let marker = if notes {
                        let note = &frames.footnotes()[0];
                        assert_eq!(
                            note.marker_width(),
                            lines.prepared().shaped().footnote_markers()[0].advance()
                        );
                        note.marker_width().get().raw() + note.marker_gap().get().raw()
                    } else {
                        0
                    };
                    let available = (if notes { 230 } else { 115 }) * 65536 - marker - 5 * 65536;
                    assert_eq!(outer.content().width().get().raw(), available);
                    assert_eq!(outer.columns()[0].final_width().get().raw(), 40 * 65536);
                    let left = outer.columns()[1].final_width().get().raw();
                    let right = outer.columns()[2].final_width().get().raw();
                    assert_eq!(left + right, available - 40 * 65536);
                    assert!((left - right).abs() <= 1);
                    assert_eq!(outer.last_fraction(), Some(2));
                    let nested = &frames.tables()[index + 1];
                    assert_eq!(nested.content().width().get().raw(), available - 45 * 65536);
                }
                for (paragraph, frame) in lines.paragraphs().iter().zip(frames.paragraphs()) {
                    assert!(actual_text(paragraph).starts_with(text));
                    assert_eq!(paragraph.inline_size(), frame.width());
                }
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_column_lines_reflow_original_text_and_retain_failure_budgets() {
    check_plain(None);
}

#[test]
fn book_v2_column_lines_measure_nested_tables_and_full_width_footnotes() {
    check_tables(None);
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_column_lines_preserve_original_harano_glyphs() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    check_plain(Some(&font));
    check_tables(Some(&font));
}
