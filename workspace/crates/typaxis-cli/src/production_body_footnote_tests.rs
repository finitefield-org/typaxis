fn production_footnote_two_long_definitions() -> serde_json::Value {
    let mut value = production_footnote_break_fixture();
    let mut second = value["document"]["footnotes"][0].clone();
    second["footnote_id"] = "note-b".into();
    value["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let children = value["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap();
    let mut reference = children.last().unwrap().clone();
    reference["footnote_id"] = "note-b".into();
    children.push(reference);
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_footnote_required_region_starts_every_pending_definition_before_extras() {
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let value = production_footnote_two_long_definitions();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        let state = search
            .require_body(&base, 0..flow.body_items().len())
            .unwrap();
        let first = &flow.definition_items(0).unwrap()[0];
        let second = &flow.definition_items(1).unwrap()[0];
        let gap = first
            .space_after()
            .checked_add(second.space_before())
            .unwrap();
        let height = first.consumed_height().unwrap();
        let minimum = height
            .checked_add(gap)
            .unwrap()
            .checked_add(second.consumed_height().unwrap())
            .unwrap();
        let greedy = search.select_region(&state, minimum).unwrap().unwrap();
        assert_eq!(greedy.fragments().len(), 1);
        let required = search
            .select_required_region(&state, minimum)
            .unwrap()
            .unwrap();
        required.verify(&state).unwrap();
        assert_eq!(required.fragments().len(), 2);
        assert_eq!(required.used_height(), minimum);
        for (index, fragment) in required.fragments().iter().enumerate() {
            assert_eq!(fragment.fragment().definition_index(), index);
            assert_eq!(fragment.fragment().items().len(), 1);
            assert!(fragment.fragment().marker().is_some());
            assert!(
                fragment
                    .fragment()
                    .candidates()
                    .iter()
                    .all(|candidate| candidate.used_height()
                        <= fragment.fragment().available_height())
            );
        }
        assert_eq!(required.next_state().pending_definitions(), [0, 1]);
        assert!(search
            .select_required_region(
                &state,
                minimum.checked_sub(Length::from_raw(1).unwrap()).unwrap()
            )
            .unwrap()
            .is_none());
        let extra = minimum
            .checked_add(height)
            .unwrap()
            .checked_add(height)
            .unwrap();
        let expanded = search
            .select_required_region(&state, extra)
            .unwrap()
            .unwrap();
        assert_eq!(expanded.fragments()[0].fragment().items().len(), 3);
        assert_eq!(expanded.fragments()[1].fragment().items().len(), 1);
        assert_eq!(expanded.used_height(), extra);
        let continuation = search
            .select_required_region(required.next_state(), minimum)
            .unwrap()
            .unwrap();
        assert!(continuation
            .fragments()
            .iter()
            .all(|f| f.fragment().marker().is_none()));
    });
}

fn production_footnote_joint_geometry_fixture() -> serde_json::Value {
    let mut value = production_footnote_flow_fixture();
    let paragraph = value["document"]["footnotes"][1]["blocks"][0].clone();
    value["document"]["footnotes"][0]["blocks"] = serde_json::json!([paragraph]);
    value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(paragraph);
    production_body_renumber(&mut value["document"], &mut 0);
    let mut body_first = 0;
    let mut reservation = 0;
    with_production_footnote_prepared(&value, &config(), |flow, _| {
        assert_eq!(flow.body_items().len(), 2);
        body_first = flow.body_items()[0].consumed_height().unwrap().raw();
        let first = &flow.definition_items(0).unwrap()[0];
        let second = &flow.definition_items(1).unwrap()[0];
        reservation = first.consumed_height().unwrap().raw()
            + first.space_after().raw()
            + second.space_before().raw()
            + second.consumed_height().unwrap().raw()
            + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW;
    });
    let master = &mut value["page_masters"]["masters"][0];
    master["body"]["height"] = (body_first + reservation).into();
    master["footnote"]["height"] = reservation.into();
    master["footnote"]["y"] = (master["body"]["y"].as_i64().unwrap() + body_first).into();
    value
}

#[test]
fn production_footnote_joint_candidate_accounts_for_separator_and_body_collision() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionFootnoteDemandStatus as Status,
    };
    let mut value = production_footnote_joint_geometry_fixture();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        assert!(search
            .evaluate_body_candidate(&base, 0..2)
            .unwrap()
            .is_none());
        let fits = search
            .evaluate_body_candidate(&base, 0..1)
            .unwrap()
            .unwrap();
        fits.verify(&base).unwrap();
        assert_eq!(fits.body_range(), 0..1);
        assert_eq!(
            fits.body_height(),
            flow.body_items()[0].consumed_height().unwrap()
        );
        let bounds = fits.footnote_bounds().unwrap();
        assert_eq!(bounds, flow.footnote_region().unwrap());
        assert_eq!(
            bounds.height().get().raw(),
            fits.footnotes().unwrap().used_height().raw()
                + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW
        );
        assert_eq!(fits.next_state().status(0), Some(Status::Complete));
        assert_eq!(base.status(0), Some(Status::Unreferenced));
        let alternate = search.begin().unwrap();
        assert!(fits.verify(&alternate).is_err());
    });
    // Move the declared region beside the body. Shared y no longer collides.
    let body_right = value["page_masters"]["masters"][0]["body"]["x"]
        .as_i64()
        .unwrap()
        + value["page_masters"]["masters"][0]["body"]["width"]
            .as_i64()
            .unwrap();
    value["page_masters"]["masters"][0]["footnote"]["x"] = body_right.into();
    value["page_masters"]["masters"][0]["footnote"]["width"] = 3_000_000.into();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        let fits = search
            .evaluate_body_candidate(&base, 0..2)
            .unwrap()
            .unwrap();
        assert_eq!(fits.footnote_bounds().unwrap().x().raw(), body_right);
    });
    // The declared maximum can also be entirely above the body.
    let master = &mut value["page_masters"]["masters"][0];
    master["footnote"]["x"] = master["body"]["x"].clone();
    master["footnote"]["y"] = 0.into();
    let body_y = master["footnote"]["height"].as_i64().unwrap() + 65_536;
    master["body"]["y"] = body_y.into();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        let fits = search
            .evaluate_body_candidate(&base, 0..2)
            .unwrap()
            .unwrap();
        let bounds = fits.footnote_bounds().unwrap();
        assert!(bounds.y().raw() + bounds.height().get().raw() <= body_y);
    });
}

#[test]
fn production_footnote_joint_candidate_rejects_keep_cuts_and_unstarted_references() {
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let mut value = production_footnote_joint_geometry_fixture();
    value["page_masters"]["masters"][0]["body"]["height"] = 10_000_000.into();
    value["page_masters"]["masters"][0]["footnote"]["y"] = 13_000_000.into();
    production_body_set_style(&mut value, "paragraph", "keep_with_next", true.into());
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        assert!(search
            .evaluate_body_candidate(&base, 0..1)
            .unwrap()
            .is_none());
        assert!(search
            .evaluate_body_candidate(&base, 1..2)
            .unwrap()
            .is_none());
        assert!(search
            .evaluate_body_candidate(&base, 0..2)
            .unwrap()
            .is_some());
    });
    let mut forced = production_footnote_two_long_definitions();
    let span = forced["document"]["footnotes"][0]["span"].clone();
    forced["document"]["footnotes"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(
            0,
            serde_json::json!({"kind":"page_break","node_id":0,"span":span,"classes":[]}),
        );
    production_body_renumber(&mut forced["document"], &mut 0);
    with_production_footnote_prepared(&forced, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        assert!(search
            .evaluate_body_candidate(&base, 0..flow.body_items().len())
            .unwrap()
            .is_none());
    });
}

#[test]
fn production_footnote_joint_candidate_preserves_cumulative_record_and_work_budgets() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_footnote_two_long_definitions();
    let mut records = 0;
    let mut work = 0;
    for (record_delta, work_delta) in [
        (None, None),
        (Some(0), None),
        (Some(1), None),
        (None, Some(0)),
        (None, Some(1)),
    ] {
        let cfg = record_delta.map_or_else(config, |d| {
            config_with_limits(ResourceLimits {
                max_fragments: records - d,
                ..ResourceLimits::default()
            })
        });
        with_production_footnote_prepared(&value, &cfg, |flow, limits| {
            let mut search = prepare_production_footnote_demand_search(
                flow,
                limits,
                work_delta.map_or(100_000, |d| work - d),
            )
            .unwrap();
            let base = search.begin().unwrap();
            let result = search.evaluate_body_candidate(&base, 0..flow.body_items().len());
            if record_delta == Some(1) {
                assert_eq!(result.err().unwrap().kind, E::FragmentLimit);
            } else if work_delta == Some(1) {
                assert_eq!(result.err().unwrap().kind, E::FootnoteSearchLimit);
            } else {
                let selected = result.unwrap().unwrap();
                assert_eq!(selected.footnotes().unwrap().fragments().len(), 2);
                records = search.record_charge();
                work = search.work_steps();
                if record_delta == Some(0) {
                    assert_eq!(
                        search
                            .evaluate_body_candidate(&base, 0..flow.body_items().len())
                            .err()
                            .unwrap()
                            .kind,
                        E::FragmentLimit
                    );
                }
                if work_delta == Some(0) {
                    assert_eq!(
                        search
                            .evaluate_body_candidate(&base, 0..flow.body_items().len())
                            .err()
                            .unwrap()
                            .kind,
                        E::FootnoteSearchLimit
                    );
                }
            }
        });
    }
}

#[test]
fn production_footnote_pages_choose_fit_and_preserve_owned_continuity() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_footnote_joint_geometry_fixture();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let start = search.begin_pages().unwrap();
        let first = search.select_page(&start).unwrap().unwrap();
        assert_eq!(first.page_index(), 0);
        assert_eq!(first.candidate().body_range(), 0..1);
        assert_eq!(first.next_state().body_start(), 1);
        let records = search.record_charge();
        let retry = search.select_page(&start).unwrap().unwrap();
        assert_eq!(retry.candidate().body_range(), 0..1);
        assert!(search.record_charge() > records);
        let second = search.select_page(first.next_state()).unwrap().unwrap();
        assert_eq!(second.page_index(), 1);
        assert_eq!(second.candidate().body_range(), 1..2);
        assert!(second.candidate().footnotes().is_none());
        assert!(second.next_state().is_complete());
        assert!(search.select_page(second.next_state()).unwrap().is_none());
        let mut other = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        assert_eq!(
            other.select_page(&start).err().unwrap().kind,
            E::ReceiptMismatch
        );
    });
    for (pages, reflows, expected) in [(1, 8, E::PageLimit), (100, 1, E::FootnoteSearchLimit)] {
        let cfg = config_with_limits(ResourceLimits {
            max_pages: pages,
            max_footnote_reflows_per_page: reflows,
            ..ResourceLimits::default()
        });
        with_production_footnote_prepared(&value, &cfg, |flow, limits| {
            let mut search =
                prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
            let start = search.begin_pages().unwrap();
            if pages == 1 {
                let first = search.select_page(&start).unwrap().unwrap();
                assert_eq!(
                    search.select_page(first.next_state()).err().unwrap().kind,
                    expected
                );
            } else {
                assert_eq!(search.select_page(&start).err().unwrap().kind, expected);
            }
        });
    }
}

#[test]
fn production_footnote_pages_continue_notes_after_body_end() {
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let mut value = production_footnote_two_long_definitions();
    value["page_masters"]["masters"][0]["footnote"]["height"] = 2_000_000.into();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let start = search.begin_pages().unwrap();
        let mut page = search.select_page(&start).unwrap().unwrap();
        assert_eq!(page.candidate().body_range().end, flow.body_items().len());
        assert_eq!(
            search
                .place_page_content(&page)
                .unwrap()
                .footnote_markers()
                .len(),
            2
        );
        let mut continuations = 0;
        while !page.next_state().is_complete() {
            let next = search.select_page(page.next_state()).unwrap().unwrap();
            let geometry = search.place_page_content(&next).unwrap();
            assert!(geometry.separator_ink().is_some());
            assert!(geometry.footnote_markers().is_empty());
            assert!(geometry
                .fragments()
                .iter()
                .all(|f| f.definition_index().is_some()));
            assert!(next.candidate().body_range().is_empty());
            assert!(
                next.candidate().footnotes().unwrap().used_height() > typaxis_core::Length::ZERO
            );
            page = next;
            continuations += 1;
            assert!(continuations < 30);
        }
        assert!(continuations > 0);
    });
}

#[test]
fn production_footnote_pages_preserve_consecutive_and_trailing_forced_breaks() {
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let mut value = production_footnote_joint_geometry_fixture();
    let blocks = value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    let point =
        |n: &serde_json::Value| serde_json::json!({"source_id":0,"start_byte":n,"end_byte":n});
    let leading = serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":point(&blocks[0]["span"]["start_byte"])});
    let trailing = serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":point(&blocks.last().unwrap()["span"]["end_byte"])});
    blocks.insert(0, leading.clone());
    blocks.insert(0, leading);
    blocks.push(trailing);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let start = search.begin_pages().unwrap();
        let first = search.select_page(&start).unwrap().unwrap();
        assert_eq!(first.candidate().body_range(), 0..0);
        assert!(first.forced_break().is_some());
        let second = search.select_page(first.next_state()).unwrap().unwrap();
        assert_eq!(second.candidate().body_range(), 1..1);
        assert!(second.forced_break().is_some());
        let third = search.select_page(second.next_state()).unwrap().unwrap();
        assert_eq!(third.candidate().body_range(), 2..3);
        let fourth = search.select_page(third.next_state()).unwrap().unwrap();
        assert_eq!(fourth.candidate().body_range(), 3..4);
        assert!(fourth.forced_break().is_some());
        assert!(!fourth.next_state().is_complete());
        let fifth = search.select_page(fourth.next_state()).unwrap().unwrap();
        assert_eq!(fifth.page_index(), 4);
        assert_eq!(fifth.candidate().body_range(), 5..5);
        assert!(fifth.next_state().is_complete());
        let sequence = search.select_pages().unwrap();
        assert_eq!(sequence.pages().len(), 5);
        let stable = search.select_stable_pages().unwrap();
        assert_eq!(stable.passes(), 2);
        assert_eq!(stable.sequence().pages().len(), 5);
        let geometry = search.place_pages_content(&sequence).unwrap();
        for index in [0, 1, 4] {
            assert!(geometry.pages()[index].separator_ink().is_none());
            assert!(geometry.pages()[index].fragments().is_empty());
            assert!(geometry.pages()[index].list_markers().is_empty());
            assert!(geometry.pages()[index].footnote_markers().is_empty());
        }
    });
}

#[test]
fn production_footnote_pages_charge_selected_snapshot_and_discarded_alternatives() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_footnote_joint_geometry_fixture();
    let mut records = 0;
    let mut work = 0;
    for mode in 0..5 {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: if mode == 1 {
                records
            } else if mode == 2 {
                records - 1
            } else {
                ResourceLimits::default().max_fragments
            },
            ..ResourceLimits::default()
        });
        with_production_footnote_prepared(&value, &cfg, |flow, limits| {
            let mut search = prepare_production_footnote_demand_search(
                flow,
                limits,
                if mode == 3 {
                    work
                } else if mode == 4 {
                    work - 1
                } else {
                    100_000
                },
            )
            .unwrap();
            let start = search.begin_pages().unwrap();
            let result = search.select_page(&start);
            if mode == 2 || mode == 4 {
                assert_eq!(
                    result.err().unwrap().kind,
                    if mode == 2 {
                        E::FragmentLimit
                    } else {
                        E::FootnoteSearchLimit
                    }
                );
            } else {
                assert_eq!(result.unwrap().unwrap().candidate().body_range(), 0..1);
                if mode == 0 {
                    records = search.record_charge();
                    work = search.work_steps();
                }
                if mode == 1 || mode == 3 {
                    assert_eq!(
                        search.select_page(&start).err().unwrap().kind,
                        if mode == 1 {
                            E::FragmentLimit
                        } else {
                            E::FootnoteSearchLimit
                        }
                    );
                }
            }
        });
    }
}

#[test]
fn production_footnote_page_content_uses_selected_origins_and_source_scopes() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    for value in [
        production_footnote_joint_geometry_fixture(),
        production_footnote_flow_fixture(),
    ] {
        with_production_footnote_prepared(&value, &config(), |flow, limits| {
            let mut search =
                prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
            let start = search.begin_pages().unwrap();
            let selected = search.select_page(&start).unwrap().unwrap();
            let placed = search.place_page_content(&selected).unwrap();
            assert!(std::ptr::eq(placed.selection(), &selected));
            let body = selected.candidate().body_range();
            assert_eq!(
                placed
                    .fragments()
                    .iter()
                    .filter(|f| f.definition_index().is_none())
                    .count(),
                body.len()
            );
            let separator = placed.separator_ink().unwrap();
            let reservation = selected.candidate().footnote_bounds().unwrap();
            assert_eq!(separator.x(), reservation.x());
            assert_eq!(separator.y(), reservation.y());
            assert_eq!(separator.width(), reservation.width());
            assert_eq!(
                separator.height().get().raw(),
                typaxis_layout::FOOTNOTE_SEPARATOR_STROKE_RAW
            );
            assert_eq!(
                separator.height().get().raw() / 2,
                typaxis_layout::FOOTNOTE_SEPARATOR_CENTER_RAW
            );
            let region = selected.candidate().footnotes().unwrap();
            assert_eq!(
                placed.footnote_markers().len(),
                region
                    .fragments()
                    .iter()
                    .filter(|f| f.fragment().marker().is_some())
                    .count()
            );
            for marker in placed.footnote_markers() {
                let fragment = placed.fragments()[marker.fragment_index() as usize];
                assert_eq!(fragment.definition_index(), Some(marker.definition_index()));
                let binding = flow.definition_marker(marker.definition_index()).unwrap();
                assert_eq!(fragment.item_index(), binding.item_index());
                assert_eq!(
                    marker.baseline().raw(),
                    fragment.fragment().bounds().y().raw() + binding.baseline().raw()
                );
                assert!(
                    marker.bounds().x().raw() + marker.bounds().width().get().raw()
                        <= fragment.fragment().bounds().x().raw()
                );
            }
            if flow.list_marker_count() > 0 {
                assert!(!placed.list_markers().is_empty());
            }
            for marker in placed.list_markers() {
                let fragment = placed.fragments()[marker.fragment_index() as usize].fragment();
                assert_eq!(marker.page_index(), fragment.page_index());
                assert!(
                    marker.bounds().x().raw() + marker.bounds().width().get().raw()
                        <= fragment.bounds().x().raw()
                );
            }
            let origin = selected.candidate().footnote_bounds().unwrap().y().raw()
                + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW;
            for definition in region.fragments() {
                let part = definition.fragment();
                let mut top = origin + definition.offset().raw();
                let mut after = 0;
                for (offset, item) in part.items().iter().enumerate() {
                    let index = part.consumed_range().start + offset;
                    let actual = placed
                        .fragments()
                        .iter()
                        .find(|p| {
                            p.definition_index() == Some(part.definition_index())
                                && p.item_index() == index
                        })
                        .unwrap()
                        .fragment();
                    let gap = if offset == 0 {
                        0
                    } else {
                        after + item.space_before().raw()
                    };
                    assert_eq!(actual.page_index(), selected.page_index());
                    assert_eq!(actual.source(), item.source().unwrap());
                    assert_eq!(actual.owner(), item.owner());
                    assert_eq!(actual.bounds().x(), item.x());
                    assert_eq!(actual.bounds().y().raw(), top + gap + item.leading().raw());
                    assert_eq!(actual.bounds().height().get(), item.height());
                    assert_eq!(actual.effective_space_before().raw(), gap);
                    top += gap + item.consumed_height().unwrap().raw();
                    after = item.space_after().raw();
                }
                assert_eq!(
                    top,
                    origin + definition.offset().raw() + part.used_height().raw()
                );
            }
            let count = search.record_charge();
            assert_eq!(
                search.place_page_content(&selected).unwrap().fragments(),
                placed.fragments()
            );
            assert!(search.record_charge() > count);
            let mut other =
                prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
            assert_eq!(
                other.place_page_content(&selected).err().unwrap().kind,
                E::ReceiptMismatch
            );
        });
    }
}

#[test]
fn production_footnote_page_content_charges_geometry_without_refunds() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_footnote_joint_geometry_fixture();
    let mut records = 0;
    let mut work = 0;
    for mode in 0..5 {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: match mode {
                1 => records,
                2 => records - 1,
                _ => ResourceLimits::default().max_fragments,
            },
            ..ResourceLimits::default()
        });
        with_production_footnote_prepared(&value, &cfg, |flow, limits| {
            let mut search = prepare_production_footnote_demand_search(
                flow,
                limits,
                match mode {
                    3 => work,
                    4 => work - 1,
                    _ => 100_000,
                },
            )
            .unwrap();
            let start = search.begin_pages().unwrap();
            let page = search.select_page(&start).unwrap().unwrap();
            let placed = search.place_page_content(&page);
            if mode == 2 || mode == 4 {
                assert_eq!(
                    placed.err().unwrap().kind,
                    if mode == 2 {
                        E::FragmentLimit
                    } else {
                        E::FootnoteSearchLimit
                    }
                );
            } else {
                assert_eq!(placed.unwrap().fragments().len(), 3);
                if mode == 0 {
                    records = search.record_charge();
                    work = search.work_steps();
                }
                if mode == 1 || mode == 3 {
                    assert_eq!(
                        search.place_page_content(&page).err().unwrap().kind,
                        if mode == 1 {
                            E::FragmentLimit
                        } else {
                            E::FootnoteSearchLimit
                        }
                    );
                }
            }
        });
    }
}

#[test]
fn production_footnote_sequence_closes_body_and_every_demanded_continuation() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    let mut long = production_footnote_two_long_definitions();
    long["page_masters"]["masters"][0]["footnote"]["height"] = 2_000_000.into();
    for value in [production_footnote_joint_geometry_fixture(), long] {
        with_production_footnote_prepared(&value, &config(), |flow, limits| {
            let mut search =
                prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
            let selected = search.select_pages().unwrap();
            assert!(selected.pages().len() >= 2);
            assert!(selected.pages().last().unwrap().next_state().is_complete());
            let placed = search.place_pages_content(&selected).unwrap();
            assert!(std::ptr::eq(placed.sequence(), &selected));
            assert_eq!(placed.pages().len(), selected.pages().len());
            let mut body_end = 0;
            let mut definitions = [0, 0];
            let mut markers = [0, 0];
            for (index, page) in placed.pages().iter().enumerate() {
                let chosen = page.selection();
                assert_eq!(chosen.page_index() as usize, index);
                assert_eq!(chosen.candidate().body_range().start, body_end);
                body_end = chosen.candidate().body_range().end;
                for marker in page.footnote_markers() {
                    markers[marker.definition_index()] += 1;
                }
                if let Some(region) = chosen.candidate().footnotes() {
                    for fragment in region.fragments() {
                        let definition = fragment.fragment().definition_index();
                        let range = fragment.fragment().consumed_range();
                        assert_eq!(range.start, definitions[definition]);
                        definitions[definition] = range.end;
                    }
                }
            }
            assert_eq!(body_end, flow.body_items().len());
            assert_eq!(markers, [1, 1]);
            for (index, end) in definitions.into_iter().enumerate() {
                assert_eq!(end, flow.definition_items(index).unwrap().len());
            }
            let mut other =
                prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
            assert_eq!(
                other.place_pages_content(&selected).err().unwrap().kind,
                E::ReceiptMismatch
            );
        });
    }
}

#[test]
fn production_footnote_sequence_rejects_nonfit_and_partial_page_limit() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    let mut value = production_footnote_joint_geometry_fixture();
    let cfg = config_with_limits(ResourceLimits {
        max_pages: 1,
        ..ResourceLimits::default()
    });
    with_production_footnote_prepared(&value, &cfg, |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        assert_eq!(search.select_pages().err().unwrap().kind, E::PageLimit);
        assert_eq!(
            search.select_stable_pages().err().unwrap().kind,
            E::PageLimit
        );
    });
    value["page_masters"]["masters"][0]["footnote"]["height"] = 1.into();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        assert_eq!(search.select_pages().err().unwrap().kind, E::JointPageNoFit);
        assert_eq!(
            search.select_stable_pages().err().unwrap().kind,
            E::JointPageNoFit
        );
    });
}

#[test]
fn production_footnote_sequence_budgets_include_all_pages_and_geometry() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_footnote_joint_geometry_fixture();
    let mut records = 0;
    let mut work = 0;
    for mode in 0..5 {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: match mode {
                1 => records,
                2 => records - 1,
                _ => ResourceLimits::default().max_fragments,
            },
            ..ResourceLimits::default()
        });
        with_production_footnote_prepared(&value, &cfg, |flow, limits| {
            let mut search = prepare_production_footnote_demand_search(
                flow,
                limits,
                match mode {
                    3 => work,
                    4 => work - 1,
                    _ => 100_000,
                },
            )
            .unwrap();
            let sequence = search.select_pages().unwrap();
            let result = search.place_pages_content(&sequence);
            if mode == 2 || mode == 4 {
                assert_eq!(
                    result.err().unwrap().kind,
                    if mode == 2 {
                        E::FragmentLimit
                    } else {
                        E::FootnoteSearchLimit
                    }
                );
            } else {
                assert_eq!(result.unwrap().pages().len(), 2);
                if mode == 0 {
                    records = search.record_charge();
                    work = search.work_steps();
                }
                if mode == 1 || mode == 3 {
                    assert_eq!(
                        search.select_pages().err().unwrap().kind,
                        if mode == 1 {
                            E::FragmentLimit
                        } else {
                            E::FootnoteSearchLimit
                        }
                    );
                }
            }
        });
    }
}

#[test]
fn production_footnote_sequence_does_not_place_unreferenced_definitions() {
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let mut value = production_footnote_joint_geometry_fixture();
    let mut unused = value["document"]["footnotes"][1].clone();
    unused["footnote_id"] = "unused-note".into();
    value["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(unused);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_footnote_untagged_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let sequence = search.select_pages().unwrap();
        let geometry = search.place_pages_content(&sequence).unwrap();
        assert_eq!(geometry.pages().len(), 2);
        assert_eq!(
            geometry
                .pages()
                .iter()
                .map(|p| p.footnote_markers().len())
                .sum::<usize>(),
            2
        );
        assert!(geometry
            .pages()
            .iter()
            .flat_map(|p| p.fragments())
            .all(|f| f.definition_index() != Some(2)));
    });
}

#[test]
fn production_footnote_stability_repeats_complete_search_and_physical_placement() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    for value in [
        production_footnote_joint_geometry_fixture(),
        production_footnote_two_long_definitions(),
        production_footnote_flow_fixture(),
    ] {
        with_production_footnote_prepared(&value, &config(), |flow, limits| {
            let mut search =
                prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
            let stable = search.select_stable_pages().unwrap();
            assert_eq!(stable.passes(), 2);
            assert_eq!(stable.record_charge(), search.record_charge());
            assert_eq!(stable.work_steps(), search.work_steps());
            assert!(stable
                .sequence()
                .pages()
                .last()
                .unwrap()
                .next_state()
                .is_complete());
            let geometry = search.place_pages_content(stable.sequence()).unwrap();
            assert_eq!(geometry.pages().len(), stable.sequence().pages().len());
            let mut other =
                prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
            assert_eq!(
                other
                    .place_pages_content(stable.sequence())
                    .err()
                    .unwrap()
                    .kind,
                E::ReceiptMismatch
            );
            let before = search.record_charge();
            let again = search.select_stable_pages().unwrap();
            assert_eq!(again.passes(), 2);
            assert!(again.record_charge() > before);
        });
    }
    let cfg = config_with_limits(ResourceLimits {
        max_layout_passes: 1,
        ..ResourceLimits::default()
    });
    with_production_footnote_prepared(
        &production_footnote_joint_geometry_fixture(),
        &cfg,
        |flow, limits| {
            let mut search =
                prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
            assert_eq!(
                search.select_stable_pages().err().unwrap().kind,
                E::PagePassLimit
            );
        },
    );
}

#[test]
fn production_footnote_stability_charges_both_passes_and_exact_comparison() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_footnote_joint_geometry_fixture();
    let mut records = 0;
    let mut work = 0;
    for mode in 0..5 {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: match mode {
                1 => records,
                2 => records - 1,
                _ => ResourceLimits::default().max_fragments,
            },
            ..ResourceLimits::default()
        });
        with_production_footnote_prepared(&value, &cfg, |flow, limits| {
            let mut search = prepare_production_footnote_demand_search(
                flow,
                limits,
                match mode {
                    3 => work,
                    4 => work - 1,
                    _ => 100_000,
                },
            )
            .unwrap();
            let result = search.select_stable_pages();
            if mode == 2 || mode == 4 {
                assert_eq!(
                    result.err().unwrap().kind,
                    if mode == 2 {
                        E::FragmentLimit
                    } else {
                        E::FootnoteSearchLimit
                    }
                );
            } else {
                let stable = result.unwrap();
                assert_eq!(stable.passes(), 2);
                if mode == 0 {
                    records = search.record_charge();
                    work = search.work_steps();
                }
                if mode == 1 || mode == 3 {
                    assert_eq!(
                        search.select_stable_pages().err().unwrap().kind,
                        if mode == 1 {
                            E::FragmentLimit
                        } else {
                            E::FootnoteSearchLimit
                        }
                    );
                }
            }
        });
    }
}

#[test]
fn production_footnote_separator_requires_actual_content_not_a_forced_fragment() {
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let mut value = production_footnote_break_fixture();
    let paragraph = value["document"]["footnotes"][0]["blocks"][0].clone();
    let forced =
        serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":paragraph["span"]});
    value["document"]["footnotes"][0]["blocks"] = serde_json::json!([forced, paragraph, forced]);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let stable = search.select_stable_pages().unwrap();
        let placed = search.place_pages_content(stable.sequence()).unwrap();
        assert!(placed.pages().len() >= 3);
        let first = &placed.pages()[0];
        assert!(first
            .selection()
            .candidate()
            .footnotes()
            .unwrap()
            .forced_break_owner()
            .is_some());
        assert!(first.separator_ink().is_none());
        assert!(placed.pages().last().unwrap().separator_ink().is_none());
        assert!(placed.pages().iter().any(|p| p.separator_ink().is_some()));
        for page in placed.pages() {
            assert_eq!(
                page.separator_ink().is_some(),
                page.fragments()
                    .iter()
                    .any(|f| f.definition_index().is_some())
            );
        }
    });
}

fn production_footnote_numbered_definition_fixture() -> serde_json::Value {
    let mut value = production_numbered_body_fixture(3_000_000);
    let formula = value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    let span = formula["span"].clone();
    let mut region = value["document"]["blocks"][0].clone();
    region["blocks"] = serde_json::json!([formula]);
    region["span"] = span.clone();
    value["document"]["footnotes"] = serde_json::json!([{"node_id":0,"span":span,"footnote_id":"equation-note","blocks":[region]}]);
    let paragraph = &mut value["document"]["blocks"][0]["blocks"][0];
    let end = paragraph["span"]["end_byte"].clone();
    paragraph["children"].as_array_mut().unwrap().push(serde_json::json!({"kind":"footnote_reference","node_id":0,"footnote_id":"equation-note","span":{"source_id":0,"start_byte":end,"end_byte":end}}));
    let body = value["page_masters"]["masters"][0]["body"].clone();
    value["page_masters"]["masters"][0]["footnote"] =
        serde_json::json!({"x":body["x"],"y":13000000,"width":16000000,"height":6000000});
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_footnote_math_terminals_close_real_stable_body_and_definition_blocks() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    for value in [
        production_footnote_flow_fixture(),
        production_footnote_joint_geometry_fixture(),
        production_numbered_body_fixture(3_000_000),
        production_footnote_numbered_definition_fixture(),
    ] {
        with_production_footnote_math_prepared(&value, &config(), |flow, limits, registry| {
            let mut search =
                prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
            let stable = search.select_stable_pages().unwrap();
            let geometry = search.place_pages_content(stable.sequence()).unwrap();
            let terminals = search
                .finalize_page_math(&stable, &geometry, registry, limits)
                .unwrap();
            assert!(std::ptr::eq(terminals.geometry(), &geometry));
            terminals.terminals().verify(registry).unwrap();
            assert_eq!(
                terminals.terminals().receipts().len(),
                registry.flows().len()
            );
            assert_eq!(
                terminals.equation_numbers().len(),
                registry.equation_number_shapes().len()
            );
            assert!(terminals.spool_bytes() <= search.terminal_spool_charge());
            for number in terminals.equation_numbers() {
                let fragment = geometry
                    .pages()
                    .iter()
                    .flat_map(|p| p.fragments())
                    .nth(number.fragment_index() as usize)
                    .unwrap()
                    .fragment();
                assert_eq!(number.parent_owner(), fragment.owner());
                assert_eq!(number.page_index(), fragment.page_index());
            }
            let other_stable = search.select_stable_pages().unwrap();
            assert_eq!(
                search
                    .finalize_page_math(&other_stable, &geometry, registry, limits)
                    .err()
                    .unwrap()
                    .kind,
                E::ReceiptMismatch
            );
            let mut other =
                prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
            assert_eq!(
                other
                    .finalize_page_math(&stable, &geometry, registry, limits)
                    .err()
                    .unwrap()
                    .kind,
                E::ReceiptMismatch
            );
        });
    }
}

#[test]
fn production_footnote_math_terminals_keep_record_work_and_spool_budgets() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_footnote_flow_fixture();
    let mut records = 0;
    let mut work = 0;
    let mut spool = 0;
    for mode in 0..7 {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: match mode {
                1 => records,
                2 => records - 1,
                _ => ResourceLimits::default().max_fragments,
            },
            max_spool_bytes: match mode {
                5 => spool,
                6 => spool - 1,
                _ => ResourceLimits::default().max_spool_bytes,
            },
            ..ResourceLimits::default()
        });
        with_production_footnote_math_prepared(&value, &cfg, |flow, limits, registry| {
            let mut search = prepare_production_footnote_demand_search(
                flow,
                limits,
                match mode {
                    3 => work,
                    4 => work - 1,
                    _ => 100_000,
                },
            )
            .unwrap();
            let stable = search.select_stable_pages().unwrap();
            let geometry = search.place_pages_content(stable.sequence()).unwrap();
            let result = search.finalize_page_math(&stable, &geometry, registry, limits);
            if [2, 4, 6].contains(&mode) {
                assert_eq!(
                    result.err().unwrap().kind,
                    match mode {
                        2 => E::FragmentLimit,
                        4 => E::FootnoteSearchLimit,
                        _ => E::SpoolLimit,
                    }
                );
            } else {
                result.unwrap();
                if mode == 0 {
                    records = search.record_charge();
                    work = search.work_steps();
                    spool = search.terminal_spool_charge();
                }
                if [1, 3, 5].contains(&mode) {
                    assert_eq!(
                        search
                            .finalize_page_math(&stable, &geometry, registry, limits)
                            .err()
                            .unwrap()
                            .kind,
                        match mode {
                            1 => E::FragmentLimit,
                            3 => E::FootnoteSearchLimit,
                            _ => E::SpoolLimit,
                        }
                    );
                }
            }
        });
    }
}

#[test]
fn production_footnote_display_projects_real_text_vectors_numbers_and_separators() {
    use typaxis_display_list::{build_production_footnote_display, ProductionBodyDraw};
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let mut reversed = production_footnote_flow_fixture();
    let children = reversed["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap();
    let count = children.len();
    children.swap(count - 1, count - 2);
    production_body_renumber(&mut reversed["document"], &mut 0);
    let mut continued = production_footnote_two_long_definitions();
    continued["page_masters"]["masters"][0]["footnote"]["height"] = 2_000_000.into();
    for value in [
        production_footnote_flow_fixture(),
        production_footnote_joint_geometry_fixture(),
        production_footnote_numbered_definition_fixture(),
        production_numbered_body_fixture(3_000_000),
        reversed,
        continued,
        production_raster_fixture("book-venn.png", 2_000_001, 6_000_000),
    ] {
        with_production_footnote_display_prepared(
            &value,
            &config(),
            |flow, limits, registry, admitted| {
                let mut search =
                    prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
                let stable = search.select_stable_pages().unwrap();
                let geometry = search.place_pages_content(stable.sequence()).unwrap();
                let terminals = search
                    .finalize_page_math(&stable, &geometry, registry, limits)
                    .unwrap();
                let display =
                    build_production_footnote_display(&terminals, admitted, limits).unwrap();
                display.verify_resources(admitted, limits).unwrap();
                assert!(std::ptr::eq(display.source(), &terminals));
                let all: Vec<_> = geometry
                    .pages()
                    .iter()
                    .flat_map(|p| p.fragments())
                    .collect();
                let mut previous = 0;
                for draw in display.draws() {
                    let (index, page) = match draw {
                        ProductionBodyDraw::Text(d) => (d.fragment_index(), d.page_index()),
                        ProductionBodyDraw::Vector(d) => (d.fragment_index(), d.page_index()),
                        ProductionBodyDraw::Raster(d) => (d.fragment_index(), d.page_index()),
                    };
                    assert!(index >= previous);
                    previous = index;
                    assert_eq!(all[index as usize].fragment().page_index(), page);
                }
                assert_eq!(
                    display
                        .draws()
                        .iter()
                        .filter(|d| matches!(d, ProductionBodyDraw::Raster(_)))
                        .count(),
                    terminals.line_layout().figures().len()
                );
                let expected_numbers = registry.equation_number_shapes().len();
                assert_eq!(display.draws().iter().filter(|d| matches!(d, ProductionBodyDraw::Text(t) if t.equation_number().is_some())).count(), expected_numbers);
                for marker in terminals.line_layout().footnote_markers() {
                    let key = marker.provenance().buffer_key();
                    let labels: Vec<_> = display
                        .draws()
                        .iter()
                        .filter_map(|d| match d {
                            ProductionBodyDraw::Text(t)
                                if t.generated_provenance()
                                    .is_some_and(|p| p.buffer_key() == key) =>
                            {
                                Some(t)
                            }
                            _ => None,
                        })
                        .collect();
                    assert!(!labels.is_empty());
                    assert_eq!(
                        labels.iter().map(|t| t.exact_text()).collect::<String>(),
                        marker.utf8()
                    );
                    assert_eq!(
                        labels.iter().map(|t| t.glyphs().len()).sum::<usize>(),
                        marker.glyph_run().glyphs.len()
                    );
                }
                assert_eq!(
                    display.separators().len(),
                    geometry
                        .pages()
                        .iter()
                        .filter(|p| p.separator_ink().is_some())
                        .count()
                );
                for sep in display.separators() {
                    assert_eq!(
                        Some(sep.ink()),
                        geometry.pages()[sep.page_index() as usize].separator_ink()
                    );
                    assert!(sep.before_draw() < display.draws().len());
                    let draw = &display.draws()[sep.before_draw()];
                    let index = match draw {
                        ProductionBodyDraw::Text(d) => d.fragment_index(),
                        ProductionBodyDraw::Vector(d) => d.fragment_index(),
                        ProductionBodyDraw::Raster(d) => d.fragment_index(),
                    };
                    assert!(all[index as usize].definition_index().is_some());
                    assert_eq!(
                        all[index as usize].fragment().page_index(),
                        sep.page_index()
                    );
                }
                let second =
                    build_production_footnote_display(&terminals, admitted, limits).unwrap();
                assert_eq!(display.fingerprint(), second.fingerprint());
                assert_eq!(display.record_charge(), second.record_charge());
            },
        );
    }
}

#[test]
fn production_footnote_display_retains_preceding_and_projection_record_charges() {
    use typaxis_display_list::{
        build_production_footnote_display, ProductionBodyDisplayErrorKind as E,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let value = production_footnote_flow_fixture();
    let mut records = 0;
    for mode in 0..3 {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: match mode {
                1 => records,
                2 => records - 1,
                _ => ResourceLimits::default().max_fragments,
            },
            ..ResourceLimits::default()
        });
        with_production_footnote_display_prepared(
            &value,
            &cfg,
            |flow, limits, registry, admitted| {
                let mut search =
                    prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
                let stable = search.select_stable_pages().unwrap();
                let geometry = search.place_pages_content(stable.sequence()).unwrap();
                let terminals = search
                    .finalize_page_math(&stable, &geometry, registry, limits)
                    .unwrap();
                let result = build_production_footnote_display(&terminals, admitted, limits);
                if mode == 2 {
                    assert_eq!(result.err().unwrap().kind, E::RecordLimit);
                } else {
                    let display = result.unwrap();
                    assert!(display.record_charge() > terminals.record_charge());
                    if mode == 0 {
                        records = display.record_charge();
                    }
                }
            },
        );
    }
}

#[test]
fn production_footnote_structure_binds_actual_reference_and_definition_labels() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure, ProductionBodyDraw,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let mut continued = production_footnote_two_long_definitions();
    continued["page_masters"]["masters"][0]["footnote"]["height"] = 2_000_000.into();
    let mut reversed = production_footnote_flow_fixture();
    let children = reversed["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap();
    let count = children.len();
    children.swap(count - 1, count - 2);
    production_body_renumber(&mut reversed["document"], &mut 0);
    let mut multi = production_footnote_multi_digit_fixture();
    multi["page_masters"]["masters"][0]["footnote"]["height"] = 16_000_000.into();
    multi["page_masters"]["masters"][0]["footnote"]["y"] = 3_000_000.into();
    for value in [
        production_footnote_flow_fixture(),
        production_footnote_joint_geometry_fixture(),
        production_footnote_numbered_definition_fixture(),
        continued,
        reversed,
        multi,
    ] {
        with_production_footnote_structure_prepared(
            &value,
            &config(),
            |flow, limits, registry, admitted, semantics, profile| {
                let mut search =
                    prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
                let stable = search.select_stable_pages().unwrap();
                let geometry = search.place_pages_content(stable.sequence()).unwrap();
                let terminals = search
                    .finalize_page_math(&stable, &geometry, registry, limits)
                    .unwrap();
                let display =
                    build_production_footnote_display(&terminals, admitted, limits).unwrap();
                let structure = build_production_footnote_structure(
                    &display,
                    semantics,
                    profile.authorization(),
                    profile.base().authorization(),
                    admitted,
                    limits,
                )
                .unwrap();
                structure.verify(&display, admitted, limits).unwrap();
                assert_eq!(structure.separator_artifacts(), display.separators());
                assert_eq!(
                    structure
                        .registry()
                        .nodes()
                        .iter()
                        .filter(|n| n.role() == typaxis_layout::StructureRole::Note)
                        .count(),
                    flow.footnotes().definitions().len()
                );
                let mut consumed = 0;
                for page in 0..geometry.pages().len() {
                    for (mcid, group) in structure
                        .page_groups(page as u32)
                        .unwrap()
                        .iter()
                        .enumerate()
                    {
                        assert_eq!(group.mcid() as usize, mcid);
                        assert_eq!(group.draws().start, consumed);
                        consumed = group.draws().end;
                    }
                }
                assert_eq!(consumed, display.draws().len());
                for node in structure.registry().nodes() {
                    let indices = structure.node_groups(node.structure_node_id()).unwrap();
                    if node.paint_required() {
                        assert!(!indices.is_empty());
                    }
                    if node.role() == typaxis_layout::StructureRole::Label {
                        let text = indices
                            .iter()
                            .flat_map(|i| &display.draws()[structure.groups()[*i].draws()])
                            .map(|d| match d {
                                ProductionBodyDraw::Text(t) => t.exact_text(),
                                _ => panic!("label must be text"),
                            })
                            .collect::<String>();
                        assert_eq!(Some(text.as_str()), node.actual_text());
                        if let typaxis_layout::StructureOwner::Generated(key) = node.owner() {
                            if key.slot() == typaxis_layout::GeneratedStructureSlot::FootnoteLabel {
                                let parent =
                                    structure.registry().node(node.parent().unwrap()).unwrap();
                                assert!(matches!(
                                    parent.role(),
                                    typaxis_layout::StructureRole::Note
                                        | typaxis_layout::StructureRole::Reference
                                ));
                            }
                        }
                    }
                }
            },
        );
    }
}

#[test]
fn production_footnote_structure_keeps_shared_record_budget_and_display_identity() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure,
        ProductionBodyStructureError as E,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let value = production_footnote_flow_fixture();
    let mut records = 0;
    for mode in 0..3 {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: match mode {
                1 => records,
                2 => records - 1,
                _ => ResourceLimits::default().max_fragments,
            },
            ..ResourceLimits::default()
        });
        with_production_footnote_structure_prepared(
            &value,
            &cfg,
            |flow, limits, registry, admitted, semantics, profile| {
                let mut search =
                    prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
                let stable = search.select_stable_pages().unwrap();
                let geometry = search.place_pages_content(stable.sequence()).unwrap();
                let terminals = search
                    .finalize_page_math(&stable, &geometry, registry, limits)
                    .unwrap();
                let display =
                    build_production_footnote_display(&terminals, admitted, limits).unwrap();
                let result = build_production_footnote_structure(
                    &display,
                    semantics,
                    profile.authorization(),
                    profile.base().authorization(),
                    admitted,
                    limits,
                );
                if mode == 2 {
                    assert_eq!(result.err().unwrap(), E::RecordLimit);
                } else {
                    let structure = result.unwrap();
                    if mode == 0 {
                        records = structure.record_charge();
                    }
                    let other =
                        build_production_footnote_display(&terminals, admitted, limits).unwrap();
                    assert_eq!(
                        structure.verify(&other, admitted, limits).err().unwrap(),
                        E::ReceiptMismatch
                    );
                }
            },
        );
    }
}

#[test]
fn production_footnote_fonts_bind_selected_glyphs_and_generated_unicode() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure, ProductionBodyDraw,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let mut multi = production_footnote_multi_digit_fixture();
    multi["page_masters"]["masters"][0]["footnote"]["height"] = 16_000_000.into();
    multi["page_masters"]["masters"][0]["footnote"]["y"] = 3_000_000.into();
    for value in [
        production_footnote_flow_fixture(),
        production_footnote_numbered_definition_fixture(),
        multi,
    ] {
        with_production_footnote_structure_prepared(
            &value,
            &config(),
            |flow, limits, registry, admitted, semantics, profile| {
                let mut search =
                    prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
                let stable = search.select_stable_pages().unwrap();
                let geometry = search.place_pages_content(stable.sequence()).unwrap();
                let terminals = search
                    .finalize_page_math(&stable, &geometry, registry, limits)
                    .unwrap();
                let display =
                    build_production_footnote_display(&terminals, admitted, limits).unwrap();
                let structure = build_production_footnote_structure(
                    &display,
                    semantics,
                    profile.authorization(),
                    profile.base().authorization(),
                    admitted,
                    limits,
                )
                .unwrap();
                let fonts = typaxis_resources::finalize_production_footnote_fonts(
                    &structure, admitted, limits,
                )
                .unwrap();
                fonts.verify(&structure, admitted, limits).unwrap();
                assert!(std::ptr::eq(fonts.structure(), &structure));
                let expected: std::collections::BTreeSet<_> = display
                    .draws()
                    .iter()
                    .filter_map(|draw| match draw {
                        ProductionBodyDraw::Text(t) => Some(t.font_face_id()),
                        _ => None,
                    })
                    .collect();
                let actual: std::collections::BTreeSet<_> =
                    fonts.fonts().iter().map(|f| f.font_face_id()).collect();
                assert_eq!(fonts.fonts().len(), expected.len());
                assert_eq!(actual, expected);
                assert!(fonts.spool_charge() >= structure.spool_charge() + terminals.spool_bytes());
                assert!(fonts.record_charge() > structure.record_charge());
                for (index, draw) in display.draws().iter().enumerate() {
                    match draw {
                        ProductionBodyDraw::Text(text) => {
                            let (font, cluster) = fonts.text_plan(index).unwrap();
                            assert_eq!(font.font_face_id(), text.font_face_id());
                            assert_eq!(cluster.text_span(), text.text_span());
                            assert_eq!(cluster.exact_text(), text.exact_text());
                            assert_eq!(
                                cluster.glyphs(),
                                text.glyphs()
                                    .iter()
                                    .map(|g| g.original_gid())
                                    .collect::<Vec<_>>()
                            );
                            assert_eq!(cluster.cids().len(), text.glyphs().len());
                            assert!(!font.pdf_font().subset_bytes().is_empty());
                        }
                        _ => assert!(fonts.text_plan(index).is_none()),
                    }
                }
                assert!(fonts.text_plan(display.draws().len()).is_none());
                let other = build_production_footnote_structure(
                    &display,
                    semantics,
                    profile.authorization(),
                    profile.base().authorization(),
                    admitted,
                    limits,
                )
                .unwrap();
                assert_eq!(
                    fonts.verify(&other, admitted, limits).err().unwrap(),
                    typaxis_resources::ResourceError::AdmittedLedgerEpochMismatch
                );
            },
        );
    }
}

#[test]
fn production_footnote_fonts_account_for_prior_structure_records_and_spool() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let value = production_footnote_flow_fixture();
    let mut records = 0;
    let mut spool = 0;
    for mode in 0..5 {
        let cfg = config_with_limits(ResourceLimits {
            max_fragments: match mode {
                1 => records,
                2 => records - 1,
                _ => ResourceLimits::default().max_fragments,
            },
            max_spool_bytes: match mode {
                3 => spool,
                4 => spool - 1,
                _ => ResourceLimits::default().max_spool_bytes,
            },
            ..ResourceLimits::default()
        });
        with_production_footnote_structure_prepared(
            &value,
            &cfg,
            |flow, limits, registry, admitted, semantics, profile| {
                let mut search =
                    prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
                let stable = search.select_stable_pages().unwrap();
                let geometry = search.place_pages_content(stable.sequence()).unwrap();
                let terminals = search
                    .finalize_page_math(&stable, &geometry, registry, limits)
                    .unwrap();
                let display =
                    build_production_footnote_display(&terminals, admitted, limits).unwrap();
                let structure = build_production_footnote_structure(
                    &display,
                    semantics,
                    profile.authorization(),
                    profile.base().authorization(),
                    admitted,
                    limits,
                )
                .unwrap();
                let result = typaxis_resources::finalize_production_footnote_fonts(
                    &structure, admitted, limits,
                );
                if mode == 2 || mode == 4 {
                    assert_eq!(
                        result.err().unwrap(),
                        typaxis_resources::ResourceError::ResourceLimit
                    );
                } else {
                    let fonts = result.unwrap();
                    if mode == 0 {
                        records = fonts.record_charge();
                        spool = fonts.spool_charge();
                    }
                }
            },
        );
    }
}
