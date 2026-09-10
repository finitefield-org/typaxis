use super::*;
use typaxis_core::{Length, PositiveLength};
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, prepare_book_v2_body_line_variant_seed,
    with_rebuilt_book_v2_body_line_variant, BookV2SourceWidthAssignments,
};

#[path = "book_v2_table_header_catalog_tests.rs"]
mod header_catalog;
#[path = "book_v2_table_header_selection_tests.rs"]
mod header_selection;
#[path = "book_v2_table_header_variant_tests.rs"]
mod header_variants;

fn check_variant_seeds(font: Option<&[u8]>, nested: bool) {
    for notes in [false, true] {
        let root = Root::new();
        let limits = driver_limits();
        let text = if font.is_some() {
            "本文を続けて組み直す本文を続けて組み直す"
        } else {
            "Pro Pro Pro Pro Pro Pro Pro"
        };
        let mut data = super::table_width_occurrences::occurrence_data(notes, "header");
        if nested {
            header_variants::nest_header(&mut data, notes);
        }
        fn expand(v: &mut Value, length: usize) {
            if let Some(a) = v.as_array_mut() {
                for v in a {
                    expand(v, length);
                }
            } else if let Some(o) = v.as_object_mut() {
                for (k, v) in o {
                    if k == "end_byte" && v.as_u64() == Some(6) {
                        *v = length.into();
                    } else {
                        expand(v, length);
                    }
                }
            }
        }
        expand(&mut data, text.len());
        data["text_buffers"][0]["utf8"] = text.into();
        let mut expected_header = std::collections::BTreeSet::new();
        header_variants::paragraph_owners(
            if notes {
                &data["document"]["footnotes"][0]["blocks"][0]["head"]
            } else {
                &data["document"]["blocks"][0]["head"]
            },
            &mut expected_header,
        );
        let input = if let Some(font) = font {
            data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
            data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                .into();
            let body = body_with_source(&root, data, text.as_bytes(), &limits);
            fs::write(root.0.join("body.bin"), font).unwrap();
            prepare_book_v2_resources(
                body,
                &root.context(),
                &config(limits.base().get().clone()),
                &limits,
            )
            .unwrap()
        } else {
            prepared(&root, data, text.as_bytes(), &limits)
        };
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let foreign = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let plan = typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_for_reflow(
            &flow, &mut 0, 1_000_000, 0, 0,
        )
        .unwrap();
        let profiles = vec![None; flow.paragraphs().len()];
        let width = |pt: i64| PositiveLength::new(Length::from_raw(pt * 65536).unwrap()).unwrap();
        let narrow = [(flow.tables()[0].owner(), width(140))];
        let wide = [(
            flow.tables()[0].owner(),
            width(if notes { 200 } else { 220 }),
        )];
        let a = BookV2SourceWidthAssignments::new(&flow, &profiles)
            .unwrap()
            .with_root_table_widths(&narrow);
        let b = BookV2SourceWidthAssignments::new(&flow, &profiles)
            .unwrap()
            .with_root_table_widths(&wide);
        let foreign_a = BookV2SourceWidthAssignments::new(&foreign, &profiles)
            .unwrap()
            .with_root_table_widths(&narrow);
        let make = |assignment, work, prior| {
            prepare_book_v2_body_line_variant_seed(
                &policy,
                &flow,
                input.resources(),
                &bindings,
                &limits,
                JapaneseLineBreakMode::Normal,
                plan.measurement_body(),
                work,
                prior,
                None,
                limits.base().get().max_line_reshape_passes,
                Some(&plan),
                Some(assignment),
            )
        };
        assert!(make(&foreign_a, 10_000_000, 0).is_err());
        let first = make(&a, 10_000_000, 0).unwrap();
        let second = make(&b, 10_000_000, first.record_charge()).unwrap();
        let sibling = first
            .prepare_with_source_widths(
                &b,
                10_000_000,
                first.record_charge(),
                limits.base().get().max_line_reshape_passes,
            )
            .unwrap();
        assert_eq!(sibling.fingerprint(), second.fingerprint());
        assert_eq!(sibling.record_charge(), second.record_charge());
        assert_eq!(sibling.work_steps(), second.work_steps());
        assert_eq!(sibling.reshape_passes(), second.reshape_passes());
        assert!(first
            .prepare_with_source_widths(
                &foreign_a,
                10_000_000,
                0,
                limits.base().get().max_line_reshape_passes
            )
            .is_err());
        assert!(first
            .prepare_with_source_widths(
                &b,
                sibling.work_steps() - 1,
                first.record_charge(),
                sibling.reshape_passes()
            )
            .is_err());
        assert!(first
            .prepare_with_source_widths(
                &b,
                sibling.work_steps(),
                first.record_charge(),
                sibling.reshape_passes() - 1
            )
            .is_err());
        assert_eq!(
            first
                .prepare_with_source_widths(
                    &b,
                    sibling.work_steps(),
                    first.record_charge(),
                    sibling.reshape_passes()
                )
                .unwrap()
                .fingerprint(),
            sibling.fingerprint()
        );
        assert_ne!(first.fingerprint(), second.fingerprint());
        assert!(std::ptr::eq(first.source_flow(), &flow));
        let captured = first
            .contexts()
            .paragraphs()
            .iter()
            .map(|p| p.ends().len() as u64 + 2)
            .sum::<u64>()
            + 1;
        let exact_prior = limits.base().get().max_fragments - captured;
        assert_eq!(
            make(&a, first.work_steps(), exact_prior)
                .unwrap()
                .record_charge(),
            limits.base().get().max_fragments
        );
        assert!(make(&a, first.work_steps(), exact_prior + 1).is_err());
        assert_eq!(
            make(&a, first.work_steps(), 0).unwrap().fingerprint(),
            first.fingerprint()
        );
        assert!(make(&a, first.work_steps() - 1, 0).is_err());
        let rebuild = |work, prior| {
            with_rebuilt_book_v2_body_line_variant(&first, work, prior, |v| {
                assert_eq!(v.lines().fingerprint(), first.fingerprint());
                (v.work_steps(), v.record_charge())
            })
        };
        let full = rebuild(10_000_000, 0).unwrap();
        assert_eq!(rebuild(full.0, 0).unwrap(), full);
        assert!(rebuild(full.0 - 1, 0).is_err());
        let exact_prior = limits.base().get().max_fragments - full.1 + first.record_charge();
        assert_eq!(
            rebuild(full.0, exact_prior).unwrap().1,
            limits.base().get().max_fragments
        );
        assert!(rebuild(full.0, exact_prior + 1).is_err());
        use typaxis_layout::book_v2::with_rebuilt_book_v2_body_line_variants;
        let seeds = [&first, &second, &first];
        let grouped = |work, prior| {
            with_rebuilt_book_v2_body_line_variants(&seeds, work, prior, |set| {
                assert_eq!(set.variants().len(), 3);
                let mut flows = Vec::new();
                for (index, v) in set.variants().iter().enumerate() {
                    assert_eq!(v.lines().fingerprint(), seeds[index].fingerprint());
                    assert!(std::ptr::eq(v.lines().prepared().source_flow(), &flow));
                    assert_eq!(v.record_charge(), set.record_charge());
                    for other in &set.variants()[..index] {
                        assert!(!std::ptr::eq(
                            v.lines().prepared(),
                            other.lines().prepared()
                        ));
                        assert!(v.footnotes().verify(other.lines(), &limits).is_err());
                    }
                    flows.push(
                        prepare_book_v2_body_flow(v.lines(), None, v.footnotes(), &limits, 0)
                            .unwrap(),
                    );
                }
                let measured = flows
                    .into_iter()
                    .map(|f| prepare_book_v2_table_measurements(f, &limits).unwrap())
                    .collect::<Vec<_>>();
                let heights = measured
                    .iter()
                    .map(|m| m.tables()[0].rows()[0].height())
                    .collect::<Vec<_>>();
                assert!(heights[0] > heights[1]);
                assert_eq!(heights[0], heights[2]);
                if work == 10_000_000 && prior == 0 {
                    header_variants::check(
                        &set,
                        &measured,
                        &limits,
                        &expected_header,
                        text.chars().count() as u32,
                    );
                    with_rebuilt_book_v2_body_line_variant(&first, 10_000_000, 0, |outside| {
                        let body = prepare_book_v2_body_flow(
                            outside.lines(),
                            None,
                            outside.footnotes(),
                            &limits,
                            0,
                        )
                        .unwrap();
                        let foreign = prepare_book_v2_table_measurements(body, &limits).unwrap();
                        assert_eq!(foreign.fingerprint(), measured[0].fingerprint());
                        assert!(
                            typaxis_pagination::book_v2::prepare_book_v2_table_header_variant(
                                &set,
                                &measured[0],
                                &foreign,
                                0,
                                &limits,
                                1_000_000,
                                0
                            )
                            .is_err()
                        );
                    })
                    .unwrap();
                }
                (set.work_steps(), set.record_charge(), set.fingerprint())
            })
        };
        let group = grouped(10_000_000, 0).unwrap();
        assert_eq!(grouped(group.0, 0).unwrap(), group);
        assert!(grouped(group.0 - 1, 0).is_err());
        let raised = limits.base().get().max_fragments / 2;
        assert!(group.1 < raised);
        let extra = grouped(group.0, raised).unwrap().1 - raised;
        let prior = limits.base().get().max_fragments - extra;
        assert_eq!(
            grouped(group.0, prior).unwrap().1,
            limits.base().get().max_fragments
        );
        assert!(grouped(group.0, prior + 1).is_err());
        let reverse = with_rebuilt_book_v2_body_line_variants(
            &[&second, &first, &first],
            10_000_000,
            0,
            |set| set.fingerprint(),
        )
        .unwrap();
        assert_ne!(reverse, group.2);
        assert!(with_rebuilt_book_v2_body_line_variants(&[], 10_000_000, 0, |_| ()).is_err());
        let foreign_seed = prepare_book_v2_body_line_variant_seed(
            &policy,
            &foreign,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            plan.measurement_body(),
            10_000_000,
            0,
            None,
            limits.base().get().max_line_reshape_passes,
            Some(&plan),
            Some(&foreign_a),
        )
        .unwrap();
        assert_eq!(foreign_seed.fingerprint(), first.fingerprint());
        let called = std::cell::Cell::new(false);
        assert!(with_rebuilt_book_v2_body_line_variants(
            &[&first, &foreign_seed],
            10_000_000,
            0,
            |_| called.set(true)
        )
        .is_err());
        assert!(!called.get());
        if !notes && font.is_none() {
            let many = vec![&first; 32];
            with_rebuilt_book_v2_body_line_variants(&many, 10_000_000, 0, |set| {
                assert_eq!(set.variants().len(), 32);
                for v in set.variants() {
                    assert_eq!(v.lines().fingerprint(), first.fingerprint());
                }
                eprintln!(
                    "line variant set 32: work={},records={}",
                    set.work_steps(),
                    set.record_charge()
                );
            })
            .unwrap();
        }
        eprintln!(
            "line variant set: notes={notes},harano={},work={},records={}",
            font.is_some(),
            group.0,
            group.1
        );
        with_rebuilt_book_v2_body_line_variant(&first,10_000_000,second.record_charge(),|left| {
            with_rebuilt_book_v2_body_line_variant(&second,10_000_000,left.record_charge(),|right| {
                let l=left.lines();let r=right.lines();
                assert!(!std::ptr::eq(l.prepared(),r.prepared()));
                assert!(!std::ptr::eq(l.prepared().shaped(),r.prepared().shaped()));
                assert!(std::ptr::eq(l.prepared().source_flow(),r.prepared().source_flow()));
                assert!(l.verify(r.prepared()).is_err());
                assert!(left.footnotes().verify(r,&limits).is_err());
                assert!(right.footnotes().verify(l,&limits).is_err());
                let mut changes=0;
                for (p,q) in l.paragraphs().iter().zip(r.paragraphs()) {
                    assert_eq!(p.owner(),q.owner());
                    if p.lines().len()!=q.lines().len() { changes+=1; }
                }
                assert!(changes>0);
                let body_l=prepare_book_v2_body_flow(l,None,left.footnotes(),&limits,0).unwrap();
                let body_r=prepare_book_v2_body_flow(r,None,right.footnotes(),&limits,0).unwrap();
                let tables_l=prepare_book_v2_table_measurements(body_l,&limits).unwrap();
                let tables_r=prepare_book_v2_table_measurements(body_r,&limits).unwrap();
                let header_l=tables_l.tables()[0].rows()[0].height();
                let header_r=tables_r.tables()[0].rows()[0].height();
                assert!(header_l>header_r);
                eprintln!("line variant seeds: notes={notes},harano={},capture={},rebuild={},records={},changed={changes},headers={}/{}",font.is_some(),first.work_steps(),full.0,right.record_charge(),header_l.raw(),header_r.raw());
            }).unwrap()
        }).unwrap();
    }
}

#[test]
fn book_v2_line_variant_seeds_rebuild_simultaneous_table_headers() {
    check_variant_seeds(None, false);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_line_variant_seeds_rebuild_original_harano_headers() {
    check_variant_seeds(
        Some(&fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap()),
        false,
    );
}

#[test]
fn book_v2_table_header_variants_retain_nested_source_geometry() {
    check_variant_seeds(None, true);
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_header_variants_retain_original_harano_nested_geometry() {
    check_variant_seeds(
        Some(&fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap()),
        true,
    );
}

#[path = "book_v2_table_header_placement_tests.rs"]
mod header_placement;
