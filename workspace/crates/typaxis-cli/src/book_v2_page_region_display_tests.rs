use super::*;
use typaxis_display_list::book_v2::BookV2PageRegionDisplayBuilder as Builder;
use typaxis_layout::book_v2::with_converged_book_v2_page_region_lines as converge;
use typaxis_linebreak::JapaneseLineBreakMode;

#[test]
fn book_v2_page_region_display_retains_owned_glyphs_after_layout_drops() {
    check_display(None, "Result");
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_page_region_display_retains_original_harano_artifact_glyphs() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check_display(Some(&font), "本文の柱");
}
fn check_display(font: Option<&[u8]>, text: &str) {
    let root = Root::new();
    let limits = limits();
    let mut data = region_data(text);
    data["page_masters"]["masters"][0]["header"]["height"] = (50 * 65536).into();
    let input = if let Some(font) = font {
        vector_tests::vector_input_with_font(&root, data, &limits, font, text.as_bytes())
    } else {
        prepared(&root, data, text.as_bytes(), &limits)
    };
    let body = input.body().styled();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let mut builder =
        Builder::new(body, input.resources(), &limits, EPOCH, 0, 0, 1_000_000).unwrap();
    let initial_records = builder.record_charge();
    let initial_work = builder.work_steps();
    let mut observed_records = u64::MAX;
    let mut observed_work = u64::MAX;
    let counted = Builder::new_counted(
        body, input.resources(), &limits, EPOCH, 0, 0, initial_work,
        &mut observed_records, &mut observed_work,
    ).unwrap();
    assert_eq!(observed_records, initial_records);
    assert_eq!(observed_work, initial_work);
    assert_eq!(counted.record_charge(), initial_records);
    assert_eq!(counted.work_steps(), initial_work);
    assert!(Builder::new_counted(
        body, input.resources(), &limits, EPOCH, 0, 0, initial_work - 1,
        &mut observed_records, &mut observed_work,
    ).is_err());
    assert_eq!((observed_records, observed_work), (initial_records, 0));
    assert!(Builder::new_counted(
        body, input.resources(), &limits, EPOCH, observed_records, observed_work, initial_work - 1,
        &mut observed_records, &mut observed_work,
    ).is_err());
    assert_eq!((observed_records, observed_work), (2 * initial_records, 0));
    for (epoch, prior) in [([0; 32], 7), (EPOCH, limits.base().get().max_fragments)] {
        assert!(Builder::new_counted(
            body, input.resources(), &limits, epoch, prior, 11, 1_000_000,
            &mut observed_records, &mut observed_work,
        ).is_err());
        assert_eq!((observed_records, observed_work), (prior, 11));
    }
    let mut displays = Vec::new();
    let mut expected = Vec::new();
    // These source/shape/line owners must be gone before retained displays are consumed.
    {
        let nav = prepare_book_v2_navigation(body).unwrap();
        for page in [0, 1] {
            let selected = select_book_v2_page_master(body, page, None, &mut 0, 1_000_000).unwrap();
            for kind in [Kind::Header, Kind::Footer] {
                let flow = region_flow(selected, kind, &nav, 0).unwrap();
                let display = converge(
                    &policy,
                    &flow,
                    input.resources(),
                    &limits,
                    EPOCH,
                    JapaneseLineBreakMode::Normal,
                    selected,
                    1_000_000,
                    limits.base().get().max_line_reshape_passes,
                    0,
                    |stable| {
                        let lines = stable.lines();
                        let mut actual = Vec::new();
                        for origin in lines.origins() {
                            let paragraph = &lines.paragraphs()[origin.paragraph_index() as usize];
                            for item in paragraph.lines()[origin.line_index() as usize].items() {
                                if let typaxis_layout::ProductionPlacedInline::Text(c) = item {
                                    for glyph in c.glyphs() {
                                        actual.push((
                                            c.run().owner(),
                                            glyph.glyph().original_gid,
                                            origin.x().checked_add(glyph.x()).unwrap(),
                                            origin.y().checked_add(glyph.y()).unwrap(),
                                        ));
                                    }
                                }
                            }
                        }
                        expected.push(actual);
                        let display = builder.build(&stable).unwrap();
                        assert_eq!(display.layout_fingerprint(), lines.fingerprint());
                        assert_eq!(display.frame(), lines.frame());
                        if page == 0 && kind == Kind::Header {
                            let records = builder.record_charge();
                            let work = builder.work_steps();
                            let mut exact = Builder::new(
                                body,
                                input.resources(),
                                &limits,
                                EPOCH,
                                limits.base().get().max_fragments - records,
                                0,
                                work,
                            )
                            .unwrap();
                            let same = exact.build(&stable).unwrap();
                            assert_eq!(same.fingerprint(), display.fingerprint());
                            assert_eq!(exact.record_charge(), limits.base().get().max_fragments);
                            let mut short = Builder::new(
                                body,
                                input.resources(),
                                &limits,
                                EPOCH,
                                limits.base().get().max_fragments - records + 1,
                                0,
                                work,
                            )
                            .unwrap();
                            assert!(short.build(&stable).is_err());
                            let mut work_short = Builder::new(
                                body,
                                input.resources(),
                                &limits,
                                EPOCH,
                                0,
                                0,
                                work - 1,
                            )
                            .unwrap();
                            assert!(work_short.build(&stable).is_err());
                            let charged = work_short.record_charge();
                            let spent = work_short.work_steps();
                            assert!(charged > initial_records);
                            assert!(work_short.build(&stable).is_err());
                            assert!(work_short.work_steps() > spent);
                            assert_eq!(work_short.record_charge(), charged);
                            let mut epoch = Builder::new(
                                body,
                                input.resources(),
                                &limits,
                                [77; 32],
                                0,
                                0,
                                work,
                            )
                            .unwrap();
                            assert!(epoch.build(&stable).is_err());
                        }
                        display
                    },
                )
                .unwrap();
                displays.push(display);
            }
        }
    }
    assert_eq!(displays.len(), 4);
    for (i, display) in displays.iter().enumerate() {
        display
            .verify_resources(body, input.resources(), &limits, EPOCH)
            .unwrap();
        assert!(display
            .verify_resources(body, input.resources(), &limits, [88; 32])
            .is_err());
        assert!(std::ptr::eq(display.source(), body));
        assert!(std::ptr::eq(display.admitted(), input.resources()));
        assert_eq!(display.page_index(), (i / 2) as u32);
        assert_eq!(
            display.kind(),
            if i % 2 == 0 {
                Kind::Header
            } else {
                Kind::Footer
            }
        );
        assert_eq!(display.owner().get(), if i % 2 == 0 { 4 } else { 10 });
        let text_actual: String = display.draws().iter().map(|d| d.exact_text()).collect();
        assert_eq!(
            text_actual,
            if i % 2 == 0 {
                text.repeat(2)
            } else {
                text.into()
            }
        );
        let actual: Vec<_> = display
            .draws()
            .iter()
            .flat_map(|d| {
                d.glyphs()
                    .iter()
                    .map(move |g| (d.owner(), g.original_gid(), g.x(), g.y()))
            })
            .collect();
        assert_eq!(actual, expected[i]);
        for draw in display.draws() {
            let span = draw.source_span();
            let buffer = &body.body().wire().text_buffers()[span.text_id().get() as usize].utf8;
            assert_eq!(
                draw.exact_text().as_ptr(),
                buffer[span.start_byte().get() as usize..].as_ptr()
            );
            assert_eq!(
                draw.font().content_hash(),
                typaxis_core::sha256(font.unwrap_or(FONT))
            );
            assert!(std::ptr::eq(
                draw.font_instance().ledger(),
                input.resources()
            ));
            assert_eq!(
                draw.font_instance().font().font_face_id(),
                draw.font().face_id()
            );
            assert!(draw.glyphs().iter().all(|g| g.original_gid().get() != 0));
            if let Some(bounds) = draw.logical_bounds() {
                assert!(bounds.y() >= display.frame().y());
                assert!(
                    bounds.y().checked_add(bounds.height().get()).unwrap()
                        <= display
                            .frame()
                            .y()
                            .checked_add(display.frame().height().get())
                            .unwrap()
                );
            }
        }
    }
    assert_ne!(displays[0].fingerprint(), displays[2].fingerprint());
    assert_ne!(displays[0].fingerprint(), displays[1].fingerprint());
    assert_eq!(expected[0], expected[2]);
    assert_eq!(expected[1], expected[3]);
    assert!(builder.record_charge() > initial_records);
    assert!(builder.work_steps() > initial_work);
    assert!(Builder::new(body, input.resources(), &limits, [0; 32], 0, 0, 1_000_000).is_err());
    assert!(Builder::new(
        body,
        input.resources(),
        &limits,
        EPOCH,
        u64::MAX,
        0,
        1_000_000
    )
    .is_err());
    assert!(Builder::new(
        body,
        input.resources(),
        &limits,
        EPOCH,
        0,
        u64::MAX,
        u64::MAX
    )
    .is_err());
    eprintln!(
        "region display font={} draws={} records={} work={}",
        if font.is_some() { "Harano" } else { "TT" },
        displays.iter().map(|d| d.draws().len()).sum::<usize>(),
        builder.record_charge(),
        builder.work_steps()
    );
}

#[test]
fn book_v2_page_region_display_keeps_empty_region_and_rejects_foreign_source() {
    let root = Root::new();
    let limits = limits();
    let mut data = region_data("Result");
    data["page_masters"]["masters"][0]["header_content"]["blocks"][0]["children"] = json!([]);
    let input = prepared(&root, data.clone(), b"Result", &limits);
    let foreign_root = Root::new();
    let foreign = prepared(&foreign_root, data, b"Result", &limits);
    let body = input.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let selected = select_book_v2_page_master(body, 0, None, &mut 0, 1_000_000).unwrap();
    let flow = region_flow(selected, Kind::Header, &nav, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let mut builder =
        Builder::new(body, input.resources(), &limits, EPOCH, 0, 0, 1_000_000).unwrap();
    let before = builder.record_charge();
    let mut foreign_builder = Builder::new(
        foreign.body().styled(),
        foreign.resources(),
        &limits,
        EPOCH,
        0,
        0,
        1_000_000,
    )
    .unwrap();
    let display = converge(
        &policy,
        &flow,
        input.resources(),
        &limits,
        EPOCH,
        JapaneseLineBreakMode::Normal,
        selected,
        1_000_000,
        limits.base().get().max_line_reshape_passes,
        0,
        |stable| {
            assert!(foreign_builder.build(&stable).is_err());
            builder.build(&stable).unwrap()
        },
    )
    .unwrap();
    assert!(display.draws().is_empty());
    assert_eq!(display.owner(), NodeId::new(4));
    assert_eq!(display.record_charge(), before + 1);
    assert!(display
        .verify_resources(foreign.body().styled(), input.resources(), &limits, EPOCH)
        .is_err());
    assert!(display
        .verify_resources(body, foreign.resources(), &limits, EPOCH)
        .is_err());
    assert_ne!(display.fingerprint(), [0; 32]);
}
