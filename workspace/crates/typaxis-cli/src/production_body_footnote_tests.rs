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
                let encoded =
                    typaxis_pdf::encode_production_footnote_text(&fonts, admitted, limits).unwrap();
                encoded.verify(&fonts, admitted, limits).unwrap();
                assert!(std::ptr::eq(encoded.fonts(), &fonts));
                let text_draws: Vec<_> = display
                    .draws()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, d)| {
                        if let ProductionBodyDraw::Text(t) = d {
                            Some((i, t))
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(encoded.paints().len(), text_draws.len());
                assert_eq!(
                    encoded.record_charge(),
                    fonts.record_charge() + text_draws.len() as u64
                );
                for (paint_index, (paint, (draw_index, text))) in
                    encoded.paints().iter().zip(text_draws).enumerate()
                {
                    let (font, cluster) = fonts.text_plan(draw_index).unwrap();
                    assert_eq!(paint.draw_index(), draw_index);
                    assert_eq!(paint.page_index(), text.page_index());
                    assert_eq!(paint.font_instance_id(), font.pdf_font().font_instance_id());
                    let commands =
                        std::str::from_utf8(encoded.paint_commands(paint_index).unwrap()).unwrap();
                    assert_eq!(commands.matches(" Tj\n").count(), text.glyphs().len());
                    for (line, (glyph, cid)) in commands
                        .lines()
                        .filter(|line| line.ends_with(" Tj"))
                        .zip(text.glyphs().iter().zip(cluster.cids()))
                    {
                        let tokens: Vec<_> = line.split_whitespace().collect();
                        assert_eq!(&tokens[..4], &["1", "0", "0", "-1"]);
                        assert_eq!(
                            tokens[4].parse::<f64>().unwrap() * 65536.0,
                            glyph.x().raw() as f64
                        );
                        assert_eq!(
                            tokens[5].parse::<f64>().unwrap() * 65536.0,
                            glyph.y().raw() as f64
                        );
                        assert_eq!(tokens[6], "Tm");
                        assert_eq!(tokens[7], format!("<{:04X}>", cid.get()));
                    }
                    let bytes =
                        std::str::from_utf8(encoded.paint_bytes(paint_index).unwrap()).unwrap();
                    assert_eq!(
                        bytes.contains("/ActualText"),
                        cluster.requires_actual_text()
                    );
                    assert!(!commands.contains("/ActualText"));
                }
                assert!(encoded.paint_bytes(encoded.paints().len()).is_none());
                assert!(encoded.paint_commands(encoded.paints().len()).is_none());
                let other_fonts = typaxis_resources::finalize_production_footnote_fonts(
                    &structure, admitted, limits,
                )
                .unwrap();
                assert_eq!(
                    encoded
                        .verify(&other_fonts, admitted, limits)
                        .err()
                        .unwrap(),
                    typaxis_pdf::ProductionBodyTextError::ReceiptMismatch
                );
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

#[test]
fn production_footnote_text_accounts_for_prior_fonts_records_and_spool() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let value = production_footnote_flow_fixture();
    let mut records = 0;
    let mut spool = 0;
    let mut output = 0;
    for mode in 0..7 {
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
            max_output_bytes: match mode {
                5 => output,
                6 => output - 1,
                _ => ResourceLimits::default().max_output_bytes,
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
                let fonts = typaxis_resources::finalize_production_footnote_fonts(
                    &structure, admitted, limits,
                )
                .unwrap();
                let result = typaxis_pdf::encode_production_footnote_text(&fonts, admitted, limits);
                if mode == 2 || mode == 4 || mode == 6 {
                    assert_eq!(
                        result.err().unwrap(),
                        if mode == 2 {
                            typaxis_pdf::ProductionBodyTextError::RecordLimit
                        } else {
                            typaxis_pdf::ProductionBodyTextError::OutputLimit
                        }
                    );
                } else {
                    let encoded = result.unwrap();
                    if mode == 0 {
                        records = encoded.record_charge();
                        spool = fonts.spool_charge() + encoded.byte_length();
                        output = encoded.byte_length();
                    }
                }
            },
        );
    }
}

#[test]
fn production_footnote_vectors_preserve_forms_occurrences_and_actual_pdf_placement() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure, ProductionBodyDraw,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    for value in [
        production_footnote_flow_fixture(),
        production_footnote_numbered_definition_fixture(),
        production_footnote_two_long_definitions(),
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
                let text =
                    typaxis_pdf::encode_production_footnote_text(&fonts, admitted, limits).unwrap();
                let plans = typaxis_resources::finalize_production_footnote_vectors(
                    &fonts, admitted, limits,
                )
                .unwrap();
                plans.verify(&fonts, admitted, limits).unwrap();
                assert!(plans.record_charge() >= text.record_charge());
                let pdf = typaxis_pdf::build_production_footnote_vector_contribution(
                    &plans, &text, admitted, limits,
                )
                .unwrap();
                assert_eq!(pdf.display_fingerprint(), display.fingerprint());
                assert_eq!(pdf.form_plans_fingerprint(), plans.forms().fingerprint());
                assert_eq!(
                    pdf.candidate_registry_fingerprint(),
                    plans.registry().receipt().fingerprint()
                );
                assert_eq!(pdf.limits_fingerprint(), limits.fingerprint());
                assert_eq!(pdf.pages().len(), geometry.pages().len());
                let vectors: Vec<_> = display
                    .draws()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, d)| {
                        if let ProductionBodyDraw::Vector(v) = d {
                            Some((i, v))
                        } else {
                            None
                        }
                    })
                    .collect();
                let mut keys = std::collections::BTreeMap::new();
                for (_, vector) in &vectors {
                    *keys.entry(vector.content_key()).or_insert(0u32) += 1;
                }
                assert_eq!(pdf.forms().len(), keys.len());
                assert_eq!(plans.forms().plans().len(), keys.len());
                assert_eq!(pdf.usages().len(), vectors.len());
                for plan in plans.forms().plans() {
                    assert_eq!(plan.total_usage_count(), keys[plan.content_key()]);
                    assert_eq!(plan.usages().len(), keys[plan.content_key()] as usize);
                }
                for (ordinal, (usage, (draw_index, vector))) in
                    pdf.usages().iter().zip(vectors).enumerate()
                {
                    assert_eq!(usage.usage_id(), ordinal as u32);
                    assert_eq!(usage.paint_ordinal(), draw_index as u32);
                    assert_eq!(usage.page_index(), vector.page_index());
                    assert_eq!(*usage.content_key(), vector.content_key());
                    assert_eq!(usage.image_id(), vector.binding().resource().image_id());
                    assert_eq!(usage.matrix(), vector.matrix());
                    assert_eq!(
                        usage.resolved_current_color(),
                        vector.resolved_current_color()
                    );
                    assert_eq!(usage.semantic_hook().owner(), vector.binding().node_id());
                    assert_eq!(
                        usage.semantic_hook().display_command_fingerprint(),
                        vector.fingerprint()
                    );
                    let commands = std::str::from_utf8(usage.content()).unwrap();
                    assert_eq!(commands.matches(" Do").count(), 1);
                    assert!(commands.contains(&format!("/{} Do", usage.form_resource_name())));
                    assert!(!commands.contains("BDC"));
                }
                let again = typaxis_pdf::build_production_footnote_vector_contribution(
                    &plans, &text, admitted, limits,
                )
                .unwrap();
                assert_eq!(again.fingerprint(), pdf.fingerprint());
                let other = typaxis_resources::finalize_production_footnote_fonts(
                    &structure, admitted, limits,
                )
                .unwrap();
                assert_eq!(
                    plans.verify(&other, admitted, limits).err().unwrap(),
                    typaxis_resources::StagingSafeVectorResourceV2Error::ReceiptMismatch
                );
                let other_text =
                    typaxis_pdf::encode_production_footnote_text(&other, admitted, limits).unwrap();
                assert_eq!(
                    typaxis_pdf::build_production_footnote_vector_contribution(
                        &plans,
                        &other_text,
                        admitted,
                        limits
                    )
                    .err()
                    .unwrap(),
                    typaxis_pdf::StagingSafeVectorPdfV2Error::DisplayMismatch
                );
            },
        );
    }
}

#[test]
fn production_footnote_vectors_account_for_prior_records_and_text_spool() {
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
                let fonts = typaxis_resources::finalize_production_footnote_fonts(
                    &structure, admitted, limits,
                )
                .unwrap();

                let text =
                    typaxis_pdf::encode_production_footnote_text(&fonts, admitted, limits).unwrap();
                let result = typaxis_resources::finalize_production_footnote_vectors(
                    &fonts, admitted, limits,
                );
                if mode == 2 {
                    assert_eq!(
                        result.err().unwrap(),
                        typaxis_resources::StagingSafeVectorResourceV2Error::RecordLimit
                    );
                    return;
                }
                let plans = result.unwrap();
                let result = typaxis_pdf::build_production_footnote_vector_contribution(
                    &plans, &text, admitted, limits,
                );
                if mode == 4 {
                    assert_eq!(
                        result.err().unwrap(),
                        typaxis_pdf::StagingSafeVectorPdfV2Error::SpoolLimit
                    );
                } else {
                    let pdf = result.unwrap();
                    if mode == 0 {
                        records = plans.record_charge();
                        spool = fonts.spool_charge() + text.byte_length() + pdf.spool_bytes();
                    }
                }
            },
        );
    }
}

#[test]
fn production_footnote_page_content_combines_draws_and_separator_artifacts() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure, ProductionBodyDraw,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let mut blank = production_footnote_joint_geometry_fixture();
    let blocks = blank["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    let point =
        |n: &serde_json::Value| serde_json::json!({"source_id":0,"start_byte":n,"end_byte":n});
    let leading = serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":point(&blocks[0]["span"]["start_byte"])});
    let trailing = serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":point(&blocks.last().unwrap()["span"]["end_byte"])});
    blocks.insert(0, leading.clone());
    blocks.insert(0, leading);
    blocks.push(trailing);
    production_body_renumber(&mut blank["document"], &mut 0);
    let mut raster_note = production_raster_fixture("book-venn.png", 2_000_001, 6_000_000);
    let mut wrapper = raster_note["document"]["blocks"][0].clone();
    let figure = raster_note["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .remove(2);
    wrapper["blocks"] = serde_json::json!([figure]);
    let end = raster_note["document"]["blocks"][0]["blocks"][0]["span"]["end_byte"].clone();
    raster_note["document"]["blocks"][0]["blocks"][0]["children"].as_array_mut().unwrap()
        .push(serde_json::json!({"kind":"footnote_reference","node_id":0,"span":point(&end),"footnote_id":"raster-note"}));
    raster_note["document"]["footnotes"] = serde_json::json!([{
        "node_id":0,"span":wrapper["span"],"footnote_id":"raster-note","blocks":[wrapper]
    }]);
    raster_note["page_masters"]["masters"][0]["footnote"] =
        serde_json::json!({"x":655360,"y":13000000,"width":16000000,"height":6000000});
    production_body_renumber(&mut raster_note["document"], &mut 0);
    let mut multi = production_footnote_multi_digit_fixture();
    multi["page_masters"]["masters"][0]["footnote"]["height"] = 16_000_000.into();
    multi["page_masters"]["masters"][0]["footnote"]["y"] = 3_000_000.into();
    let mut reversed = production_footnote_flow_fixture();
    let children = reversed["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap();
    let count = children.len();
    children.swap(count - 1, count - 2);
    production_body_renumber(&mut reversed["document"], &mut 0);
    let mut list_note = production_footnote_flow_fixture();
    list_note["document"]["footnotes"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    production_body_renumber(&mut list_note["document"], &mut 0);
    let mut navigation_note = production_footnote_flow_fixture();
    let paragraph = &mut navigation_note["document"]["footnotes"][1]["blocks"][0];
    let children = paragraph["children"].clone();
    paragraph["children"] = serde_json::json!([
        {"kind":"anchor","node_id":0,"span":paragraph["span"],"anchor_id":"foot-target"},
        {"kind":"link","node_id":0,"span":paragraph["span"],"target":{"kind":"uri","uri":"https://example.com/footnote"},"children":children}
    ]);
    let body = &mut navigation_note["document"]["blocks"][0]["blocks"][0];
    let span = body["span"].clone();
    let children = body["children"].as_array_mut().unwrap();
    let references = children.split_off(children.len() - 2);
    let linked = serde_json::Value::Array(std::mem::take(children));
    *children = vec![serde_json::json!({"kind":"link","node_id":0,"span":span,
        "target":{"kind":"internal","anchor_id":"foot-target"},"children":linked})];
    children.extend(references);
    production_body_renumber(&mut navigation_note["document"], &mut 0);
    for value in [
        production_body_navigation_vmb_fixture(),
        production_body_navigation_multiline_fixture(),
        navigation_note,
        list_note,
        reversed,
        multi,
        raster_note,
        production_footnote_flow_fixture(),
        production_footnote_numbered_definition_fixture(),
        production_footnote_two_long_definitions(),
        blank,
        production_raster_fixture("book-venn.png", 2_000_001, 6_000_000),
        production_raster_fixture("color-2x1.jpg", 2_000_001, 6_000_000),
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
                let content =
                    typaxis_pdf::build_production_footnote_page_content(&fonts, admitted, limits)
                        .unwrap();
                content.verify(&fonts, admitted, limits).unwrap();
                assert_eq!(content.pages().len(), geometry.pages().len());
                assert!(content.spool_charge() > fonts.spool_charge());
                let mut draw_index = 0;
                let mut separator_index = 0;
                for (page_index, page) in content.pages().iter().enumerate() {
                    assert_eq!(page.page_index(), page_index as u32);
                    let bytes = std::str::from_utf8(page.content()).unwrap();
                    let root_end = bytes.find(" cm\n").unwrap() + 4;
                    assert!(bytes.starts_with("q\n1 0 0 -1 0 "));
                    let root_tokens: Vec<_> =
                        bytes.lines().nth(1).unwrap().split_whitespace().collect();
                    assert_eq!(
                        root_tokens[5].parse::<f64>().unwrap() * 65536.0,
                        terminals
                            .block_layout()
                            .page_geometry()
                            .page_height()
                            .get()
                            .raw() as f64
                    );
                    let mut expected = bytes[..root_end].to_owned();
                    let mut artifact_index = 0;
                    for (ordinal, draw) in page.draws().iter().enumerate() {
                        assert_eq!(draw.draw_index(), draw_index);
                        if let Some(artifact) = page.artifacts().get(artifact_index) {
                            if artifact.before_draw() == draw_index {
                                let separator = &display.separators()[separator_index];
                                assert_eq!(separator.before_draw(), draw_index);
                                assert_eq!(separator.page_index(), page_index as u32);
                                let commands = std::str::from_utf8(
                                    page.artifact_content(artifact_index).unwrap(),
                                )
                                .unwrap();
                                assert!(commands
                                    .starts_with("/Artifact BMC\nq\n0 G\n0.5 w 0 J [] 0 d\n"));
                                assert!(commands.ends_with("Q\nEMC\n"));
                                assert!(!commands.contains("MCID"));
                                let line = commands.lines().find(|l| l.ends_with(" l S")).unwrap();
                                let values: Vec<_> = line.split_whitespace().collect();
                                let ink = separator.ink();
                                assert_eq!(
                                    values[0].parse::<f64>().unwrap() * 65536.0,
                                    ink.x().raw() as f64
                                );
                                assert_eq!(
                                    values[1].parse::<f64>().unwrap() * 65536.0,
                                    (ink.y().raw() + 16384) as f64
                                );
                                assert_eq!(
                                    values[3].parse::<f64>().unwrap() * 65536.0,
                                    (ink.x().raw() + ink.width().get().raw()) as f64
                                );
                                assert_eq!(values[1], values[4]);
                                expected.push_str(commands);
                                artifact_index += 1;
                                separator_index += 1;
                            }
                        }
                        let commands = page.draw_content(ordinal).unwrap();
                        match (&display.draws()[draw_index], draw.source()) {
                            (
                                ProductionBodyDraw::Text(t),
                                typaxis_pdf::ProductionBodyPageDrawSource::Text { paint_index },
                            ) => {
                                assert_eq!(t.page_index(), page_index as u32);
                                assert_eq!(
                                    commands,
                                    content.text().paint_bytes(paint_index).unwrap()
                                );
                            }
                            (
                                ProductionBodyDraw::Vector(v),
                                typaxis_pdf::ProductionBodyPageDrawSource::Vector { usage_index },
                            ) => {
                                assert_eq!(v.page_index(), page_index as u32);
                                assert_eq!(
                                    commands,
                                    content.vectors().usages()[usage_index].content()
                                );
                            }
                            (
                                ProductionBodyDraw::Raster(r),
                                typaxis_pdf::ProductionBodyPageDrawSource::Raster { plan_index },
                            ) => {
                                assert_eq!(r.page_index(), page_index as u32);
                                assert_eq!(
                                    Some(plan_index),
                                    content.rasters().draw_plan(draw_index)
                                );
                                let plan = &content.rasters().plans()[plan_index];
                                assert_eq!(plan.width(), r.image().width());
                                assert_eq!(plan.height(), r.image().height());
                                assert!(!plan.encoded_bytes().is_empty());
                                let matrix: Vec<_> = std::str::from_utf8(commands)
                                    .unwrap()
                                    .lines()
                                    .nth(1)
                                    .unwrap()
                                    .split_whitespace()
                                    .collect();
                                let viewport = r.viewport();
                                assert_eq!(
                                    matrix[0].parse::<f64>().unwrap() * 65536.0,
                                    viewport.width().get().raw() as f64
                                );
                                assert_eq!(matrix[1], "0");
                                assert_eq!(matrix[2], "0");
                                assert_eq!(
                                    matrix[3].parse::<f64>().unwrap() * 65536.0,
                                    -viewport.height().get().raw() as f64
                                );
                                assert_eq!(
                                    matrix[4].parse::<f64>().unwrap() * 65536.0,
                                    viewport.x().raw() as f64
                                );
                                assert_eq!(
                                    matrix[5].parse::<f64>().unwrap() * 65536.0,
                                    (viewport.y().raw() + viewport.height().get().raw()) as f64
                                );

                                assert!(std::str::from_utf8(commands)
                                    .unwrap()
                                    .contains(&format!("/PBR{plan_index} Do")));
                            }
                            _ => panic!("draw source mismatch"),
                        }
                        expected.push_str(std::str::from_utf8(commands).unwrap());
                        expected.push('\n');
                        draw_index += 1;
                    }
                    assert_eq!(artifact_index, page.artifacts().len());
                    expected.push_str("Q\n");
                    assert_eq!(expected, bytes);
                    if page.draws().is_empty() {
                        assert!(page.artifacts().is_empty());
                    }
                }
                assert_eq!(draw_index, display.draws().len());
                assert_eq!(separator_index, display.separators().len());

                let marked = typaxis_pdf::build_production_footnote_marked_content(
                    &content, admitted, limits,
                )
                .unwrap();
                marked.verify(&content, admitted, limits).unwrap();
                assert!(std::ptr::eq(marked.structure(), &structure));
                assert!(std::ptr::eq(marked.content(), &content));
                assert_eq!(marked.pages().len(), content.pages().len());
                assert_eq!(
                    marked.record_charge(),
                    content.record_charge()
                        + marked.pages().len() as u64
                        + marked.anchors().len() as u64
                );
                let mut group_index = 0;
                for (page_index, page) in marked.pages().iter().enumerate() {
                    let commands = std::str::from_utf8(page.content()).unwrap();
                    let groups = structure.page_groups(page_index as u32).unwrap();
                    assert_eq!(commands.matches("/MCID ").count(), groups.len());
                    let mut scope_depth = 0usize;
                    let mut artifacts = 0;
                    for line in commands.lines() {
                        if line == "/Artifact BMC" {
                            assert_eq!(scope_depth, 0, "artifact inside structural content");
                            scope_depth += 1;
                            artifacts += 1;
                        } else if line.ends_with(" BDC") {
                            scope_depth += 1;
                        } else if line == "EMC" {
                            scope_depth = scope_depth.checked_sub(1).expect("unbalanced EMC");
                        }
                    }
                    assert_eq!(scope_depth, 0);
                    assert_eq!(artifacts, content.pages()[page_index].artifacts().len());
                    for ordinal in 0..artifacts {
                        let artifact = std::str::from_utf8(
                            content.pages()[page_index]
                                .artifact_content(ordinal)
                                .unwrap(),
                        )
                        .unwrap();
                        assert!(commands.contains(artifact));
                    }
                    let actual: Vec<_> = commands
                        .lines()
                        .filter(|l| l.contains("/ActualText"))
                        .map(str::to_owned)
                        .collect();
                    let mut expected = Vec::new();
                    for (mcid, group) in groups.iter().enumerate() {
                        let node = structure.registry().node(group.node()).unwrap();
                        assert!(commands.contains(&format!(
                            "/{} << /MCID {} /Lang",
                            node.role().pdf_name(),
                            mcid
                        )));
                        let actual_text = if group.is_text() {
                            Some(
                                group
                                    .draws()
                                    .map(|i| match &display.draws()[i] {
                                        ProductionBodyDraw::Text(t) => t.exact_text(),
                                        _ => panic!("non-text in text group"),
                                    })
                                    .collect::<String>(),
                            )
                        } else {
                            structure.group_actual_text(group_index).map(str::to_owned)
                        };
                        if let Some(text) = actual_text {
                            let hex: String =
                                text.encode_utf16().map(|u| format!("{u:04X}")).collect();
                            expected.push(format!("/Span << /ActualText <FEFF{hex}> >> BDC"));
                        }
                        group_index += 1;
                    }
                    assert_eq!(actual, expected);
                }
                assert_eq!(group_index, structure.groups().len());
                assert_eq!(
                    marked.anchors().len(),
                    (0..structure.groups().len())
                        .filter(|&i| structure.group_actual_text(i).is_some())
                        .count()
                );
                for anchor in marked.anchors() {
                    let group = &structure.groups()[anchor.group_index()];
                    let ProductionBodyDraw::Vector(vector) = &display.draws()[group.draws().start]
                    else {
                        panic!("anchor without formula");
                    };
                    assert_eq!(anchor.viewport(), vector.viewport());
                    assert_eq!(anchor.baseline(), vector.baseline().unwrap());
                    assert_eq!(anchor.page_index(), vector.page_index());
                }

                let nav = typaxis_display_list::build_production_footnote_reference_navigation(
                    &structure,
                    admitted,
                    limits,
                    marked.record_charge(),
                )
                .unwrap();
                nav.verify(&structure, admitted, limits).unwrap();
                assert_eq!(nav.record_base(), marked.record_charge());
                assert!(nav.additional_records() > 0);
                let source_notes = terminals.footnote_lines();
                assert_eq!(nav.destinations().len(), source_notes.definitions().len());
                let fragments: Vec<_> = geometry
                    .pages()
                    .iter()
                    .flat_map(|p| p.fragments())
                    .collect();
                for (index, destination) in nav.destinations().iter().enumerate() {
                    assert_eq!(destination.definition_index(), index);
                    assert_eq!(
                        destination.owner(),
                        source_notes.definitions()[index].owner()
                    );
                    assert_eq!(
                        structure
                            .registry()
                            .node(destination.node())
                            .unwrap()
                            .role(),
                        typaxis_display_list::StructureRole::Note
                    );
                    let (first_index, first) = fragments
                        .iter()
                        .enumerate()
                        .find(|(_, f)| f.definition_index() == Some(index))
                        .unwrap();
                    assert_eq!(destination.fragment_index(), first_index as u32);
                    assert_eq!(destination.page_index(), first.fragment().page_index());
                    let bounds = first.fragment().bounds();
                    assert!(destination.bounds().x() <= bounds.x());
                    assert!(destination.bounds().y() <= bounds.y());
                    assert!(
                        destination.bounds().x().raw() + destination.bounds().width().get().raw()
                            >= bounds.x().raw() + bounds.width().get().raw()
                    );
                    assert!(
                        destination.bounds().y().raw() + destination.bounds().height().get().raw()
                            >= bounds.y().raw() + bounds.height().get().raw()
                    );
                    for draw in display.draws() {
                        let (fragment, rect) = match draw {
                            ProductionBodyDraw::Text(t) => (t.fragment_index(), t.logical_bounds()),
                            ProductionBodyDraw::Vector(v) => {
                                (v.fragment_index(), Some(v.viewport()))
                            }
                            ProductionBodyDraw::Raster(r) => {
                                (r.fragment_index(), Some(r.viewport()))
                            }
                        };
                        if fragment == first_index as u32 {
                            if let Some(rect) = rect {
                                let target = destination.bounds();
                                assert!(target.x() <= rect.x() && target.y() <= rect.y());
                                assert!(
                                    target.x().raw() + target.width().get().raw()
                                        >= rect.x().raw() + rect.width().get().raw()
                                );
                                assert!(
                                    target.y().raw() + target.height().get().raw()
                                        >= rect.y().raw() + rect.height().get().raw()
                                );
                            }
                        }
                    }
                }
                let expected_owners: std::collections::BTreeSet<_> = source_notes
                    .references()
                    .iter()
                    .map(|r| r.owner())
                    .collect();
                let actual_owners: std::collections::BTreeSet<_> =
                    nav.links().iter().map(|r| r.owner()).collect();
                assert_eq!(expected_owners, actual_owners);
                for link in nav.links() {
                    let reference = source_notes
                        .references()
                        .iter()
                        .find(|r| r.owner() == link.owner())
                        .unwrap();
                    assert_eq!(link.definition_index(), reference.definition_index());
                    assert_eq!(
                        structure.registry().node(link.node()).unwrap().role(),
                        typaxis_display_list::StructureRole::Reference
                    );
                    let rects: Vec<_> = display
                        .draws()
                        .iter()
                        .filter_map(|draw| match draw {
                            ProductionBodyDraw::Text(t)
                                if t.owner() == link.owner()
                                    && t.page_index() == link.page_index()
                                    && t.fragment_index() == link.fragment_index() =>
                            {
                                t.logical_bounds()
                            }
                            _ => None,
                        })
                        .collect();
                    let left = rects.iter().map(|r| r.x().raw()).min().unwrap();
                    let top = rects.iter().map(|r| r.y().raw()).min().unwrap();
                    let right = rects
                        .iter()
                        .map(|r| r.x().raw() + r.width().get().raw())
                        .max()
                        .unwrap();
                    let bottom = rects
                        .iter()
                        .map(|r| r.y().raw() + r.height().get().raw())
                        .max()
                        .unwrap();
                    assert_eq!(
                        (
                            link.bounds().x().raw(),
                            link.bounds().y().raw(),
                            link.bounds().width().get().raw(),
                            link.bounds().height().get().raw()
                        ),
                        (left, top, right - left, bottom - top)
                    );
                }
                let mut cursor = 0;
                for page in 0..geometry.pages().len() {
                    let range = nav.page_links(page as u32).unwrap();
                    assert_eq!(range.start, cursor);
                    assert!(nav.links()[range.clone()]
                        .iter()
                        .all(|l| l.page_index() == page as u32));
                    cursor = range.end;
                }
                assert_eq!(cursor, nav.links().len());
                assert!(nav.page_links(geometry.pages().len() as u32).is_none());
                let again = typaxis_display_list::build_production_footnote_reference_navigation(
                    &structure,
                    admitted,
                    limits,
                    marked.record_charge(),
                )
                .unwrap();
                assert_eq!(again.fingerprint(), nav.fingerprint());
                assert_eq!(again.destinations(), nav.destinations());
                assert_eq!(again.links(), nav.links());
                let other_structure = build_production_footnote_structure(
                    &display,
                    semantics,
                    profile.authorization(),
                    profile.base().authorization(),
                    admitted,
                    limits,
                )
                .unwrap();
                assert_eq!(
                    nav.verify(&other_structure, admitted, limits)
                        .err()
                        .unwrap()
                        .kind,
                    typaxis_display_list::ProductionBodyNavigationErrorKind::ReceiptMismatch
                );
                assert_eq!(
                    typaxis_display_list::build_production_footnote_reference_navigation(
                        &structure, admitted, limits, 0
                    )
                    .err()
                    .unwrap()
                    .kind,
                    typaxis_display_list::ProductionBodyNavigationErrorKind::ReceiptMismatch
                );

                let combined = typaxis_display_list::build_production_footnote_navigation(
                    &structure,
                    admitted,
                    limits,
                    marked.record_charge(),
                )
                .unwrap();
                combined.verify(&structure).unwrap();
                assert_eq!(
                    combined.footnote_references().destinations(),
                    nav.destinations()
                );
                assert_eq!(combined.footnote_references().links(), nav.links());
                let ordinary_charge =
                    combined.footnote_references().record_base() - marked.record_charge();
                assert_eq!(
                    combined.additional_records(),
                    ordinary_charge + combined.footnote_references().additional_records()
                );
                let source = terminals.line_layout().source_flow().navigation();
                assert_eq!(combined.destinations().len(), source.anchors().len());
                for (index, destination) in combined.destinations().iter().enumerate() {
                    let (name, owner) = &source.anchors()[index];
                    assert_eq!(combined.destination_name(index as u32), Some(name));
                    assert_eq!(destination.owner(), *owner);
                    if let Some(anchor) = display
                        .inline_anchors()
                        .iter()
                        .find(|a| a.source().source().owner() == *owner)
                    {
                        assert_eq!(destination.page_index(), anchor.page_index());
                        assert_eq!(destination.fragment_index(), anchor.fragment_index());
                        assert_eq!(destination.x(), anchor.x());
                        assert_eq!(destination.y(), anchor.baseline());
                    }
                }
                assert_eq!(combined.outline_entries(), source.outline().entries());
                assert_eq!(combined.outline().len(), source.outline().entries().len());
                assert_eq!(
                    combined.outline_root().descendants() as usize,
                    source.outline().entries().len()
                );
                let mut next_link = 0;
                for page in 0..geometry.pages().len() {
                    let range = combined.page_links(page as u32).unwrap();
                    assert_eq!(range.start, next_link);
                    assert!(combined.links()[range.clone()]
                        .iter()
                        .all(|l| l.page_index() == page as u32));
                    next_link = range.end;
                }
                assert_eq!(next_link, combined.links().len());
                for (index, link) in combined.links().iter().enumerate() {
                    assert!(combined
                        .node_links(link.node())
                        .unwrap()
                        .contains(&(index as u32)));
                    assert_eq!(
                        structure.registry().node(link.node()).unwrap().role(),
                        typaxis_display_list::StructureRole::Link
                    );
                    match link.target() {
                        typaxis_display_list::ProductionBodyLinkTarget::Internal(target) => {
                            assert!(combined.destinations().get(target as usize).is_some())
                        }
                        typaxis_display_list::ProductionBodyLinkTarget::Uri(uri) => {
                            assert_eq!(uri, "https://example.com/footnote")
                        }
                    }
                }
                assert_eq!(
                    combined.verify(&other_structure).err().unwrap().kind,
                    typaxis_display_list::ProductionBodyNavigationErrorKind::ReceiptMismatch
                );

                let annotations =
                    typaxis_pdf::build_production_footnote_annotations(&marked, admitted, limits)
                        .unwrap();
                annotations.verify(&marked, admitted, limits).unwrap();
                assert_eq!(
                    annotations.objects().len(),
                    combined.links().len() + combined.footnote_references().links().len()
                );
                assert_eq!(annotations.bindings().len(), annotations.objects().len());
                let height = terminals
                    .block_layout()
                    .page_geometry()
                    .page_height()
                    .get()
                    .raw();
                let mut previous = None;
                for (index, (object, binding)) in annotations
                    .objects()
                    .iter()
                    .zip(annotations.bindings())
                    .enumerate()
                {
                    use typaxis_pdf::{
                        ProductionBodyObjectChunk as Chunk, ProductionBodyObjectRole as Role,
                        ProductionFootnoteAnnotationSource as Source,
                    };
                    assert_eq!(object.role(), Role::LinkAnnotation(index as u32));
                    assert_eq!(
                        binding.parent_key(),
                        geometry.pages().len() as u32 + index as u32
                    );
                    assert!(annotations
                        .node_annotations(binding.node())
                        .unwrap()
                        .contains(&(index as u32)));
                    let bytes: Vec<u8> = object
                        .chunks()
                        .iter()
                        .flat_map(|c| {
                            if let Chunk::Bytes(b) = c {
                                b.as_slice()
                            } else {
                                &[]
                            }
                        })
                        .copied()
                        .collect();
                    let dictionary = std::str::from_utf8(&bytes).unwrap();
                    let references: Vec<_> = object
                        .chunks()
                        .iter()
                        .filter_map(|c| {
                            if let Chunk::Reference(r) = c {
                                Some(*r)
                            } else {
                                None
                            }
                        })
                        .collect();
                    assert_eq!(references[0], Role::Page(binding.page_index()));
                    let (owner, fragment, bounds) = match binding.source() {
                        Source::Ordinary(i) => {
                            let link = &combined.links()[i];
                            assert_eq!(binding.node(), link.node());
                            assert_eq!(binding.page_index(), link.page_index());
                            assert_eq!(references.len(), 1);
                            match link.target() {
                                typaxis_display_list::ProductionBodyLinkTarget::Internal(
                                    target,
                                ) => {
                                    let hex: String = combined
                                        .destination_name(target)
                                        .unwrap()
                                        .as_str()
                                        .encode_utf16()
                                        .map(|u| format!("{u:04X}"))
                                        .collect();
                                    assert!(dictionary.contains(&format!("/Dest <FEFF{hex}>")));
                                }
                                typaxis_display_list::ProductionBodyLinkTarget::Uri(uri) => {
                                    let hex: String =
                                        uri.bytes().map(|b| format!("{b:02X}")).collect();
                                    assert!(dictionary.contains(&format!("/URI <{hex}>")));
                                }
                            }
                            (link.owner(), link.fragment_index(), link.bounds())
                        }
                        Source::Footnote(i) => {
                            let link = &combined.footnote_references().links()[i];
                            let target = &combined.footnote_references().destinations()
                                [link.definition_index()];
                            assert_eq!(binding.node(), link.node());
                            assert_eq!(
                                references,
                                vec![
                                    Role::Page(link.page_index()),
                                    Role::Page(target.page_index())
                                ]
                            );
                            assert!(dictionary.contains("/Dest ["));
                            let xyz: Vec<_> = dictionary
                                .split("/XYZ ")
                                .nth(1)
                                .unwrap()
                                .split_whitespace()
                                .take(2)
                                .collect();
                            assert_eq!(
                                xyz[0].parse::<f64>().unwrap() * 65536.0,
                                target.bounds().x().raw() as f64
                            );
                            assert_eq!(
                                xyz[1].parse::<f64>().unwrap() * 65536.0,
                                (height - target.bounds().y().raw()) as f64
                            );
                            let label = terminals
                                .line_layout()
                                .source_flow()
                                .footnote_marker_text(link.owner())
                                .unwrap();
                            let hex: String =
                                label.encode_utf16().map(|u| format!("{u:04X}")).collect();
                            assert!(dictionary.contains(&format!("/Contents <FEFF{hex}>")));
                            (link.owner(), link.fragment_index(), link.bounds())
                        }
                    };
                    let key = (binding.page_index(), fragment, owner, binding.source());
                    assert!(previous.is_none_or(|p| p < key));
                    previous = Some(key);
                    let rect: Vec<f64> = dictionary
                        .split("/Rect [")
                        .nth(1)
                        .unwrap()
                        .split(']')
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .map(|v| v.parse().unwrap())
                        .collect();
                    assert_eq!(
                        rect.iter().map(|v| v * 65536.0).collect::<Vec<_>>(),
                        vec![
                            bounds.x().raw() as f64,
                            (height - bounds.y().raw() - bounds.height().get().raw()) as f64,
                            (bounds.x().raw() + bounds.width().get().raw()) as f64,
                            (height - bounds.y().raw()) as f64
                        ]
                    );
                    assert!(
                        dictionary.contains(&format!("/StructParent {} /P", binding.parent_key()))
                    );
                }
                let mut end = 0;
                for page in 0..geometry.pages().len() {
                    let range = annotations.page_annotations(page as u32).unwrap();
                    assert_eq!(range.start, end);
                    assert!(annotations.bindings()[range.clone()]
                        .iter()
                        .all(|a| a.page_index() == page as u32));
                    end = range.end;
                }
                assert_eq!(end, annotations.bindings().len());
                let other_marked = typaxis_pdf::build_production_footnote_marked_content(
                    &content, admitted, limits,
                )
                .unwrap();
                assert_eq!(
                    annotations
                        .verify(&other_marked, admitted, limits)
                        .err()
                        .unwrap(),
                    typaxis_pdf::ProductionBodyObjectError::ReceiptMismatch
                );
                let other_content =
                    typaxis_pdf::build_production_footnote_page_content(&fonts, admitted, limits)
                        .unwrap();
                assert_eq!(
                    marked
                        .verify(&other_content, admitted, limits)
                        .err()
                        .unwrap(),
                    typaxis_pdf::ProductionBodyMarkedError::ReceiptMismatch
                );
                let other = typaxis_resources::finalize_production_footnote_fonts(
                    &structure, admitted, limits,
                )
                .unwrap();
                assert_eq!(
                    content.verify(&other, admitted, limits).err().unwrap(),
                    typaxis_pdf::ProductionBodyPageError::ReceiptMismatch
                );
            },
        );
    }
}

#[test]
fn production_footnote_page_content_accounts_for_prior_records_spool_and_output() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let value = production_footnote_flow_fixture();
    let mut records = 0;
    let mut spool = 0;
    let mut output = 0;
    for mode in 0..7 {
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
            max_output_bytes: match mode {
                5 => output,
                6 => output - 1,
                _ => ResourceLimits::default().max_output_bytes,
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
                let fonts = typaxis_resources::finalize_production_footnote_fonts(
                    &structure, admitted, limits,
                )
                .unwrap();
                let result =
                    typaxis_pdf::build_production_footnote_page_content(&fonts, admitted, limits);
                if mode == 2 || mode == 4 || mode == 6 {
                    assert_eq!(
                        result.err().unwrap(),
                        if mode == 2 {
                            typaxis_pdf::ProductionBodyPageError::Rasters(
                                typaxis_resources::ResourceError::ResourceLimit,
                            )
                        } else {
                            typaxis_pdf::ProductionBodyPageError::OutputLimit
                        }
                    );
                } else {
                    let encoded = result.unwrap();
                    if mode == 0 {
                        records = encoded.record_charge();
                        spool = encoded
                            .spool_charge()
                            .max(encoded.rasters().peak_spool_charge());
                        output = encoded
                            .pages()
                            .iter()
                            .map(|p| p.content().len() as u64)
                            .sum();
                    }
                }
            },
        );
    }
}

#[test]
fn production_footnote_marked_content_preserves_prior_records_spool_and_output() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let value = production_footnote_flow_fixture();
    let mut records = 0;
    let mut spool = 0;
    let mut output = 0;
    for mode in 0..7 {
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
            max_output_bytes: match mode {
                5 => output,
                6 => output - 1,
                _ => ResourceLimits::default().max_output_bytes,
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
                let fonts = typaxis_resources::finalize_production_footnote_fonts(
                    &structure, admitted, limits,
                )
                .unwrap();
                let content =
                    typaxis_pdf::build_production_footnote_page_content(&fonts, admitted, limits)
                        .unwrap();
                let result = typaxis_pdf::build_production_footnote_marked_content(
                    &content, admitted, limits,
                );
                if mode == 2 || mode == 4 || mode == 6 {
                    assert_eq!(
                        result.err().unwrap(),
                        if mode == 2 {
                            typaxis_pdf::ProductionBodyMarkedError::RecordLimit
                        } else {
                            typaxis_pdf::ProductionBodyMarkedError::OutputLimit
                        }
                    );
                } else {
                    let encoded = result.unwrap();
                    if mode == 0 {
                        records = encoded.record_charge();
                        spool = encoded.spool_charge();
                        output = encoded
                            .pages()
                            .iter()
                            .map(|p| p.content().len() as u64)
                            .sum();
                    }
                }
            },
        );
    }
}

#[test]
fn production_footnote_reference_navigation_keeps_retained_record_budget() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure,
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
                let content =
                    typaxis_pdf::build_production_footnote_page_content(&fonts, admitted, limits)
                        .unwrap();

                let marked = typaxis_pdf::build_production_footnote_marked_content(
                    &content, admitted, limits,
                )
                .unwrap();
                let result = typaxis_display_list::build_production_footnote_reference_navigation(
                    &structure,
                    admitted,
                    limits,
                    marked.record_charge(),
                );
                if mode == 2 {
                    assert_eq!(
                        result.err().unwrap().kind,
                        typaxis_display_list::ProductionBodyNavigationErrorKind::RecordLimit
                    );
                } else {
                    let nav = result.unwrap();
                    if mode == 0 {
                        records = nav.record_base() + nav.additional_records();
                    }
                }
            },
        );
    }
}

#[test]
fn production_footnote_navigation_keeps_combined_retained_record_budget() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let value = production_body_navigation_vmb_fixture();
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
                let content =
                    typaxis_pdf::build_production_footnote_page_content(&fonts, admitted, limits)
                        .unwrap();

                let marked = typaxis_pdf::build_production_footnote_marked_content(
                    &content, admitted, limits,
                )
                .unwrap();
                let result = typaxis_display_list::build_production_footnote_navigation(
                    &structure,
                    admitted,
                    limits,
                    marked.record_charge(),
                );
                if mode == 2 {
                    assert_eq!(
                        result.err().unwrap().kind,
                        typaxis_display_list::ProductionBodyNavigationErrorKind::RecordLimit
                    );
                } else {
                    let nav = result.unwrap();
                    if mode == 0 {
                        records = nav.record_base() + nav.additional_records();
                    }
                }
            },
        );
    }
}

#[test]
fn production_footnote_annotations_preserve_retained_record_and_spool_budgets() {
    use typaxis_display_list::{
        build_production_footnote_display, build_production_footnote_structure,
    };
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let value = production_body_navigation_vmb_fixture();
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
                let fonts = typaxis_resources::finalize_production_footnote_fonts(
                    &structure, admitted, limits,
                )
                .unwrap();
                let content =
                    typaxis_pdf::build_production_footnote_page_content(&fonts, admitted, limits)
                        .unwrap();

                let marked = typaxis_pdf::build_production_footnote_marked_content(
                    &content, admitted, limits,
                )
                .unwrap();

                let result =
                    typaxis_pdf::build_production_footnote_annotations(&marked, admitted, limits);
                if mode == 2 || mode == 4 {
                    assert_eq!(
                        result.err().unwrap(),
                        if mode == 2 {
                            typaxis_pdf::ProductionBodyObjectError::RecordLimit
                        } else {
                            typaxis_pdf::ProductionBodyObjectError::SpoolLimit
                        }
                    );
                } else {
                    let annotations = result.unwrap();
                    if mode == 0 {
                        records = annotations.record_charge();
                        spool = annotations.spool_charge();
                    }
                }
            },
        );
    }
}

#[test]
fn production_footnote_annotations_conflicting_enclosing_link_is_rejected_by_profile() {
    let mut value = production_footnote_flow_fixture();
    let paragraph = &mut value["document"]["blocks"][0]["blocks"][0];
    let children = paragraph["children"].clone();
    paragraph["children"] = serde_json::json!([{"kind":"link","node_id":0,"span":paragraph["span"],
        "target":{"kind":"uri","uri":"https://example.com/"},"children":children}]);
    production_body_renumber(&mut value["document"], &mut 0);
    // The authenticated tagged path refuses this semantic overlap before
    // annotation construction. This helper asserts UnsupportedSemantic.
    with_production_footnote_untagged_prepared(&value, &config(), |_, _| {});
}
