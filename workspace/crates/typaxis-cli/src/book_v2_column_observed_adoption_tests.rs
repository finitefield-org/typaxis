//! Compare the live observed topology with the existing pagination kernel on
//! the same admitted source, native math, tables, notes, and original font.
use super::*;
use driver::observed_adoption::{reserve_controls, with_test_catalog};
#[path = "book_v2_column_observed_pdf_tests.rs"]
mod observed_pdf;

fn page_facts(pages: &BookV2ColumnRepeatedPages<'_, '_, '_, '_, '_>) -> Value {
    json!(pages
        .sequence()
        .pages()
        .iter()
        .map(|p| json!({
            "index": p.page_index(), "name": p.named_page(),
            "forced": p.forced_break().map(|o| o.get()),
            "columns": p.candidate().columns().iter().map(|c| json!({
                "x": c.bounds().x().raw(), "y": c.bounds().y().raw(),
                "width": c.bounds().width().get().raw(),
                "height": c.bounds().height().get().raw(),
                "used": c.used_height().raw(),
                "parts": c.parts().iter().map(|part| part.items()).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        }))
        .collect::<Vec<_>>())
}

fn facts(
    pages: &BookV2ColumnRepeatedPages<'_, '_, '_, '_, '_>,
    search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
    feedback: &BookV2ColumnWidthFeedback<'_, '_>,
) -> Value {
    assert!(feedback.matches_selected_line_widths());
    assert!(feedback.matches_selected_block_widths());
    json!({
        "pages": page_facts(pages), "lines": line_facts(search.source_lines()),
        "widths": feedback.assignment_fingerprint(),
        "native": search.source_lines().prepared().native_math().map(|n| n.fingerprint())
    })
}

#[test]
fn book_v2_column_observed_adoption_preserves_native_and_plain_geometry() {
    for native in [false, true] {
        for rich in [false, true] {
            let root = Root::new();
            let limits = caps();
            let input = native_input(&root, variant_data(native, rich), &limits, MATH_FONT);
            let mut legacy = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
            let expected = driver::with_budgeted_book_v2_column_pages(
                &input,
                &limits,
                MODE,
                &mut legacy,
                |pages, search, feedback, _| facts(pages, search, feedback),
            )
            .unwrap();
            let mut observed = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
            let actual = driver::observed_adoption::with_original_pages(
                &input,
                &limits,
                MODE,
                &mut observed,
                |pages, search, feedback, snapshot| {
                    assert_eq!(snapshot.candidates, 1);
                    assert!(snapshot.records > 0 && snapshot.work > 0 && snapshot.spool > 0);
                    Ok(facts(pages, search, feedback))
                },
            )
            .unwrap();
            assert_eq!(actual, expected, "native={native} rich={rich}");
            let snapshot = observed.observation();
            assert_eq!(snapshot.candidates, 1);
            eprintln!("observed adoption native={native} rich={rich}: {snapshot:?}");
        }
    }
}

#[test]
fn book_v2_column_observed_adoption_callback_error_retains_completed_history() {
    let root = Root::new();
    let limits = caps();
    let input = native_input(&root, variant_data(true, true), &limits, MATH_FONT);
    let mut budget = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    let mut delivered = None;
    let error = driver::observed_adoption::with_original_pages(
        &input,
        &limits,
        MODE,
        &mut budget,
        |_, _, _, snapshot| {
            delivered = Some(snapshot);
            Err::<(), _>(driver::E::Identity)
        },
    )
    .unwrap_err();
    assert!(matches!(error, driver::E::Identity));
    let prefix = delivered.unwrap();
    assert_eq!(budget.observation(), prefix);
    let mut repeated = None;
    driver::observed_adoption::with_original_pages(
        &input,
        &limits,
        MODE,
        &mut budget,
        |_, _, _, snapshot| {
            repeated = Some(snapshot);
            Ok(())
        },
    )
    .unwrap();
    let final_history = budget.observation();
    assert_eq!(Some(final_history), repeated);
    assert_eq!(final_history.candidates, 2);
    assert!(final_history.records > prefix.records);
    assert!(final_history.spool > prefix.spool);
    assert!(final_history.work > prefix.work);
    assert!(final_history.line_passes > prefix.line_passes);
    assert!(final_history.page_passes > prefix.page_passes);
}

fn nested_data() -> Value {
    let mut d = variant_data(true, true);
    let blocks = d["document"]["blocks"][0]["blocks"].as_array_mut().unwrap();
    let table = blocks.last_mut().unwrap();
    let mut nested = table.clone();
    nested["body"].as_array_mut().unwrap().truncate(1);
    table["body"][0]["cells"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .extend([nested.clone(), nested]);
    renumber(&mut d["document"], &mut 0);
    d
}

fn assert_paid(request: &reserve_controls::Reservation) {
    let n = request.count as u64;
    assert_eq!(
        request.paid,
        [
            request.before[0] + n,
            request.before[1] + n * request.element_bytes as u64,
            request.before[2] + n,
        ],
        "{request:?}"
    );
}

fn allocation_refusal(result: Result<(), driver::E>) {
    assert!(
        matches!(
            result,
            Err(driver::E::ObservedPagination(
                typaxis_pagination::ProductionBodyPaginationError {
                    kind: typaxis_pagination::ProductionBodyPaginationErrorKind::AllocationFailure,
                    ..
                }
            ))
        ),
        "{result:?}"
    );
}

#[test]
fn book_v2_column_observed_adoption_nested_headers_share_one_real_sibling_but_pay_all_views() {
    let root = Root::new();
    let limits = caps();
    let input = native_input(&root, nested_data(), &limits, MATH_FONT);
    let requests = [(0, width(120)), (1, width(120)), (2, width(120))];
    let mut budget = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    let (result, trace) = reserve_controls::run(None, || {
        with_test_catalog(
            &input,
            &limits,
            &mut budget,
            &requests,
            |catalog, snapshot| {
                assert_eq!(catalog.len(), 3);
                assert_eq!(catalog.base().tables().len(), 3);
                // One base and one shared root-width sibling each begin two real
                // line passes. Three header requests do not create three siblings.
                assert_eq!(snapshot.line_passes, 4);
            },
        )
    });
    result.unwrap();
    for request in &trace {
        assert_paid(request);
    }
    for site in ["Seeds", "Variants", "Numbers", "Blocks", "Measurements"] {
        assert_eq!(
            trace.iter().find(|r| r.site == site).unwrap().count,
            4,
            "{site}: Q+1 prepayment"
        );
    }
    assert_eq!(trace.iter().filter(|r| r.site == "RootWidths").count(), 1);
    assert!(trace.iter().any(|r| r.site == "ScopeOwners" && r.count > 0));
    assert_eq!(trace.iter().filter(|r| r.site == "OwnerUnion").count(), 2);
    for (index, request) in trace.iter().enumerate().filter(|(_, r)| r.count > 0) {
        let mut budget = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
        let (result, refused) = reserve_controls::run(Some(index), || {
            with_test_catalog(&input, &limits, &mut budget, &requests, |_, _| {
                panic!("fault at {} delivered a catalog", request.site)
            })
        });
        allocation_refusal(result);
        assert_eq!(refused.len(), index + 1);
        let paid = refused.last().unwrap();
        assert_paid(paid);
        assert_eq!(paid.site, request.site);
        assert_eq!(
            [
                budget.observation().records,
                budget.observation().spool,
                budget.observation().work
            ],
            paid.paid
        );
    }
    eprintln!(
        "observed nested catalog: {} actual metadata requests, {} nonempty failure controls",
        trace.len(),
        trace.iter().filter(|r| r.count > 0).count()
    );
}

#[test]
fn book_v2_column_observed_adoption_every_real_metadata_reserve_keeps_its_paid_prefix() {
    let root = Root::new();
    let limits = caps();
    let input = native_input(&root, variant_data(true, true), &limits, MATH_FONT);
    let mut budget = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    let (result, trace) = reserve_controls::run(None, || {
        driver::observed_adoption::with_original_pages(
            &input,
            &limits,
            MODE,
            &mut budget,
            |_, _, _, _| Ok(()),
        )
    });
    result.unwrap();
    assert!(trace
        .iter()
        .any(|r| r.site == "DiscoveryRequests" && r.count > 0));
    for request in &trace {
        assert_paid(request);
    }
    for (index, request) in trace.iter().enumerate().filter(|(_, r)| r.count > 0) {
        let mut budget = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
        let (result, refused) = reserve_controls::run(Some(index), || {
            driver::observed_adoption::with_original_pages(
                &input,
                &limits,
                MODE,
                &mut budget,
                |_, _, _, _| panic!("fault at {} delivered pages", request.site),
            )
        });
        allocation_refusal(result);
        assert_eq!(refused.len(), index + 1);
        let paid = refused.last().unwrap();
        assert_paid(paid);
        assert_eq!(
            [
                budget.observation().records,
                budget.observation().spool,
                budget.observation().work
            ],
            paid.paid
        );
        assert_eq!(budget.observation().candidates, 0);
    }
    eprintln!(
        "observed native pagination: {} actual metadata requests, {} nonempty failure controls",
        trace.len(),
        trace.iter().filter(|r| r.count > 0).count()
    );
}

#[test]
fn book_v2_column_observed_adoption_named_master_width_retry_matches_existing_kernel() {
    let root = Root::new();
    let limits = caps();
    let mut d = variant_data(true, false);
    d["document"]["blocks"][0]["blocks"][2]["classes"] = json!(["narrow"]);
    let rules = d["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"observed-narrow","selector":"paragraph.narrow","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"page","important":false,"value":{"kind":"string","value":"narrow"}}]}));
    let mut named = d["page_masters"]["masters"][0].clone();
    named["master_id"] = "narrow-master".into();
    named["body"]["width"] = (200 * 65536).into();
    d["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .push(named);
    d["page_masters"]["selection_rules"] = json!([
        {"master_id":"narrow-master","parity":"any","first":null,"named_page":"narrow","source_order":0}
    ]);
    let input = native_input(&root, d, &limits, MATH_FONT);
    let mut legacy = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    let expected = driver::with_budgeted_book_v2_column_pages(
        &input,
        &limits,
        MODE,
        &mut legacy,
        |pages, search, feedback, _| facts(pages, search, feedback),
    )
    .unwrap();
    let mut budget = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    let (result, trace) = reserve_controls::run(None, || {
        driver::observed_adoption::with_original_pages(
            &input,
            &limits,
            MODE,
            &mut budget,
            |pages, search, feedback, _| Ok(facts(pages, search, feedback)),
        )
    });
    assert_eq!(result.unwrap(), expected);
    assert!(budget.observation().width_passes > 1);
    for site in ["WidthViews", "StartViews", "EndViews"] {
        assert!(
            trace.iter().any(|r| r.site == site && r.count > 0),
            "{site}"
        );
    }
    for (index, request) in trace
        .iter()
        .enumerate()
        .filter(|(_, r)| matches!(r.site.as_str(), "WidthViews" | "StartViews" | "EndViews"))
    {
        let mut budget = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
        let (result, refused) = reserve_controls::run(Some(index), || {
            driver::observed_adoption::with_original_pages(
                &input,
                &limits,
                MODE,
                &mut budget,
                |_, _, _, _| Ok(()),
            )
        });
        allocation_refusal(result);
        assert_eq!(refused.last().unwrap().site, request.site);
        assert_eq!(
            refused.last().unwrap().paid,
            [
                budget.observation().records,
                budget.observation().spool,
                budget.observation().work
            ]
        );
        assert_eq!(budget.observation().width_passes, 1);
    }
}

#[test]
fn book_v2_column_observed_adoption_complete_exact_and_short_work_budgets_keep_history() {
    let root = Root::new();
    let limits = caps();
    let input = native_input(&root, variant_data(true, true), &limits, MATH_FONT);
    let mut baseline = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    driver::observed_adoption::with_original_pages(
        &input,
        &limits,
        MODE,
        &mut baseline,
        |_, _, _, _| Ok(()),
    )
    .unwrap();
    let exact = baseline.observation().work;
    for maximum in [exact - 1, exact] {
        let mut budget = driver::BookV2ColumnPageBudget::new(&limits, maximum);
        let mut delivered = false;
        let result = driver::observed_adoption::with_original_pages(
            &input,
            &limits,
            MODE,
            &mut budget,
            |_, _, _, _| {
                delivered = true;
                Ok(())
            },
        );
        assert_eq!(result.is_ok(), maximum == exact, "{result:?}");
        assert_eq!(delivered, maximum == exact);
        let prefix = budget.observation();
        assert!(prefix.records > 0 && prefix.spool > 0 && prefix.work > 0);
        assert!(prefix.work <= maximum);
        if maximum == exact {
            assert_eq!(prefix, baseline.observation());
        } else {
            let result = driver::observed_adoption::with_original_pages(
                &input,
                &limits,
                MODE,
                &mut budget,
                |_, _, _, _| Ok(()),
            );
            assert!(result.is_err());
            let after = budget.observation();
            assert!(
                after.records >= prefix.records
                    && after.spool >= prefix.spool
                    && after.work >= prefix.work
            );
        }
    }
}

#[test]
fn book_v2_column_observed_adoption_foreign_limits_and_invalid_header_requests_never_allocate_views(
) {
    let root = Root::new();
    let limits = caps();
    let input = native_input(&root, variant_data(true, true), &limits, MATH_FONT);
    let foreign = changed(&limits, |base| base.max_layout_passes += 1);
    let mut budget = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    let before = budget.observation();
    let (result, trace) = reserve_controls::run(None, || {
        driver::observed_adoption::with_original_pages::<()>(
            &input,
            &foreign,
            MODE,
            &mut budget,
            |_, _, _, _| panic!("foreign command limits delivered pages"),
        )
    });
    assert!(matches!(result, Err(driver::E::Identity)));
    assert_eq!(budget.observation(), before);
    assert!(trace.is_empty());
    let (result, trace) = reserve_controls::run(None, || {
        with_test_catalog::<()>(
            &input,
            &limits,
            &mut budget,
            &[(usize::MAX, width(120))],
            |_, _| panic!("invalid table request delivered a catalog"),
        )
    });
    assert!(matches!(result, Err(driver::E::Identity)));
    assert!(trace.is_empty());
    assert!(budget.observation().records > 0 && budget.observation().work > 0);
    let mut budget = driver::BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    with_test_catalog(&input, &limits, &mut budget, &[], |catalog, snapshot| {
        let (mut r, mut s, mut w) = (snapshot.records, snapshot.spool, snapshot.work);
        let result = with_book_v2_observed_source_accounting(
            &mut r, &mut s, &mut w, limits.base(), limits.base().get().max_fragments - 1, MAXIMUM,
            |a| typaxis_pagination::book_v2::prepare_book_v2_column_table_header_catalog_with_observed_source_counted(
                catalog.base(), &[], &limits, a).map(|_| ()),
        );
        assert_eq!(result, Err(typaxis_pagination::book_v2::BookV2ObservedColumnHeaderCatalogError::Accounting(A::LimitsMismatch)));
        assert_eq!([r, s, w], [snapshot.records, snapshot.spool, snapshot.work]);
    }).unwrap();
}

fn prepared_saved_export(
    record: &Value,
    limits: &M4EffectiveResourceLimits,
) -> (PreparedBookV2Resources, PathBuf, Vec<u8>) {
    let directory = PathBuf::from(record["directory"].as_str().unwrap());
    let path = directory.join("document-package.json");
    let original = fs::read(&path).unwrap();
    let hex = |bytes: &[u8]| {
        typaxis_core::sha256(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    assert_eq!(hex(&original), record["package_sha256"].as_str().unwrap());
    let context = HostAdmissionContext::new(
        HostPath::new(path.clone()).unwrap(),
        HostPath::new(directory).unwrap(),
        None,
        vec![],
    );
    let (session, raw) = HostBookV2InputSession::open(
        MachineInputHostOptions::new(HostPath::new(path.clone()).unwrap(), None),
        limits.base(),
    )
    .unwrap();
    let decoded = session
        .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(limits.base()))
        .unwrap();
    let sources = session.admit_sources(&decoded, limits.base()).unwrap();
    let body = prepare_admitted_book_v2_body(
        session.finish(raw, decoded, sources).unwrap(),
        limits.base(),
    )
    .unwrap();
    let input = crate::book_v2_resources::prepare_book_v2_resources(
        body,
        &context,
        &config(&limits),
        &limits,
    )
    .unwrap();
    assert_eq!(
        hex(input.resources().fonts()[0].bytes()),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    (input, path, original)
}

#[test]
#[ignore = "requires TYPAXIS_COLUMN_OBSERVED_VMB_JOBS with the saved original-font VMB export index"]
fn book_v2_column_observed_adoption_real_vmb_exports_keep_original_harano_geometry() {
    let index_path = std::env::var("TYPAXIS_COLUMN_OBSERVED_VMB_JOBS").unwrap();
    let index: Value = serde_json::from_slice(&fs::read(index_path).unwrap()).unwrap();
    let entries = index.as_object().unwrap();
    assert!(!entries.is_empty());
    for (name, record) in entries {
        let limits = caps();
        let (input, path, original) = prepared_saved_export(record, &limits);
        let maximum = 1_000_000_000;
        let mut legacy = driver::BookV2ColumnPageBudget::new(&limits, maximum);
        let expected = driver::with_budgeted_book_v2_column_pages(
            &input,
            &limits,
            MODE,
            &mut legacy,
            |pages, search, feedback, _| facts(pages, search, feedback),
        )
        .unwrap();
        let mut observed = driver::BookV2ColumnPageBudget::new(&limits, maximum);
        let actual = driver::observed_adoption::with_original_pages(
            &input,
            &limits,
            MODE,
            &mut observed,
            |pages, search, feedback, _| Ok(facts(pages, search, feedback)),
        )
        .unwrap();
        assert_eq!(actual, expected, "original VMB export {name}");
        assert_eq!(fs::read(path).unwrap(), original);
        eprintln!(
            "observed original-font VMB export {name}: {:?}",
            observed.observation()
        );
    }
}
