//! Test-host snapshots of genuine complete convergence attempts. Serializers
//! borrow the actual roots; all companion graphs and JSON allocations belong to
//! the host and are excluded from the command's ledger.
use super::*;
use crate::book_v2_resources::converged_pdf::{
    with_converged_column_pdf_observer, with_convergence_probe_fixture, ColumnAttemptObserver,
    ColumnPdfAttempt,
};
use crate::book_v2_resources::BookV2ColumnPdfConvergenceBudget;

fn observation(value: BookV2PdfConvergenceObservation) -> Value {
    json!({
        "charges":pdf_quartet(value),
        "candidate_passes":value.candidate_passes(),
        "balance_candidates":value.balance_candidates(),
        "width_passes":value.width_feedback_passes(),
        "width_refinements":value.width_refinement_passes(),
        "line_passes":value.line_reshape_passes(),
        "page_passes":value.page_passes()
    })
}
fn statistics(value: typaxis_resources::book_v2::BookV2CffProgramStatistics) -> Value {
    json!({"operations_used":value.operations_used,
        "outline_segments_used":value.outline_segments_used,"cached_glyphs":value.cached_glyphs})
}
fn page_owners(value: &Value, result: &mut Vec<u32>) {
    match value {
        Value::Object(object) => {
            if object.get("kind").and_then(Value::as_str) == Some("reference")
                && object.get("format").and_then(Value::as_str) == Some("page")
            {
                result.push(object["node_id"].as_u64().unwrap().try_into().unwrap());
            }
            for child in object.values() {
                page_owners(child, result);
            }
        }
        Value::Array(values) => {
            for child in values {
                page_owners(child, result);
            }
        }
        _ => {}
    }
}
fn census(source: &Value) -> Value {
    let mut main = Vec::new();
    page_owners(&source["document"], &mut main);
    main.sort_unstable();
    assert!(main.windows(2).all(|pair| pair[0] < pair[1]));
    let mut rows = Vec::new();
    for (index, master) in source["page_masters"]["masters"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        for role in ["header_content", "footer_content"] {
            let content = &master[role];
            let mut owners = Vec::new();
            page_owners(content, &mut owners);
            assert!(
                owners.is_empty(),
                "admitted running grammar contains no Page sites"
            );
            rows.push(json!({"master":index,"master_id":master["master_id"],
                "role":role,"present":!content.is_null(),"source":content,
                "page_reference_owners":owners}));
        }
    }
    json!({"main_page_owners":main,"all_master_rows":rows})
}

struct Capture<'host> {
    input: &'host PreparedBookV2Resources,
    caps: &'host M4EffectiveResourceLimits,
    maximum: u64,
    case: String,
    directory: PathBuf,
    pending: Option<Value>,
    attempts: Vec<Value>,
    early_failure_prefixes: Vec<Value>,
}
impl ColumnAttemptObserver for Capture<'_> {
    fn observe<
        'event,
        'pdf,
        'nav,
        'w,
        'l,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >(
        &mut self,
        roots: &'event crate::book_v2_resources::BookV2ColumnPdfBundle<
            'pdf,
            'nav,
            'w,
            'l,
            'u,
            'h,
            'n,
            'm,
            'k,
            'j,
            'r,
            'i,
            't,
            'o,
            'z,
            'y,
            'x,
            'c,
            'v,
            'd,
            'g,
            'q,
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        attempt: ColumnPdfAttempt<'event>,
    ) {
        assert!(self.pending.is_none());
        assert!(std::ptr::eq(roots.pdf().bytes(), attempt.pdf_bytes));
        assert_eq!(roots.constructor_charges(), attempt.constructors);
        let display = roots.display();
        let names = display
            .source()
            .source()
            .geometry()
            .pages()
            .iter()
            .enumerate()
            .map(|(i, p)| (i as u32, p.selection().named_page().map(str::to_owned)))
            .collect::<Vec<_>>();
        let epoch = display
            .source()
            .original_graph()
            .lines()
            .prepared()
            .shaped()
            .binding_epoch();
        // This existing authentic local preparation is a test-host companion;
        // its allocations and counters never enter the command ledger.
        let (_, local) = original_regions(self.input, self.caps, epoch, &names);
        for (region, proof) in display.page_regions().iter().zip(&local) {
            assert_projection(region, proof);
        }
        assert_eq!(display.page_regions().len(), local.len());
        let case = format!("{}-attempt-{}", self.case, self.attempts.len());
        let parent = region_probes::snapshot_convergence_nav(
            &self.directory,
            &case,
            roots.navigation_bundle(),
            self.caps,
            roots.navigation_prefix(),
            &local,
            self.maximum,
        );
        let context = json!({"scope":"convergence_attempt","parent_scope":"convergence_attempt",
            "maximum_work":self.maximum,"constructors":attempt.constructors,
            "caller_before":roots.caller_before(),"previous":pdf_quartet(roots.previous_observation()),
            "navigation_prefix":pdf_quartet(roots.navigation_prefix()),
            "source_display_prefix":observation(attempt.display_ready),
            "build_after":attempt.assembly_before_labels,"caller_after":null});
        let pdf = pdf_probes::snapshot_roots(
            &self.directory,
            &case,
            roots.pdf(),
            self.caps,
            attempt.assembly_reservation_sizes,
            parent,
            context,
            "typaxis.book-2-column-convergence-attempt/2",
            self.maximum,
        );
        let source = attempt.source;
        self.pending = Some(json!({
            "index":self.attempts.len(),
            "values":attempt.values.iter().map(|(owner,page)|[owner.get(),*page]).collect::<Vec<_>>(),
            "references":attempt.references.iter().map(|r|json!({"owner":r.owner().get(),
                "candidate":r.candidate_page(),"target":r.target_page(),"placed":r.is_placed()})).collect::<Vec<_>>(),
            "source":{"copied":source.copied,"census_before":observation(source.census_before),
                "census_after":observation(source.census_after),"before":observation(source.source_before),
                "after":observation(source.source_after)},
            "assembly_before_labels":attempt.assembly_before_labels,
            "assembly_reservation_sizes":attempt.assembly_reservation_sizes,
            "after_labels":observation(attempt.observation),"decision":attempt.decision_name(),
            "display_fingerprint":probes::hex(&attempt.display_fingerprint),
            "pdf_fingerprint":probes::hex(&attempt.pdf_fingerprint),"pdf":pdf,
            "after_return":null
        }));
    }

    fn after_attempt(
        &mut self,
        actual: BookV2PdfConvergenceObservation,
        cff: typaxis_resources::book_v2::BookV2CffProgramStatistics,
        outcome_is_ok: bool,
    ) {
        let returned = json!({"observation":observation(actual),"cff":statistics(cff),
            "outcome_is_ok":outcome_is_ok});
        if let Some(mut pending) = self.pending.take() {
            pending["after_return"] = returned;
            self.attempts.push(pending);
        } else {
            // An actual early error is a genuine retained prefix, not a complete
            // PDF snapshot or an independently replayed failed-attempt trace.
            self.early_failure_prefixes.push(returned);
        }
    }
}

#[test]
fn book_v2_column_converged_pdf_captures_actual_complete_roots_and_post_return_prefixes() {
    let temporary = Root::new();
    let destination = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE")
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.0.clone())
        .join("column-page-label-convergence");
    fs::create_dir_all(&destination).unwrap();
    for (fixture, expected_attempts) in [
        ("no-page", 1),
        ("forward-ten", 3),
        ("wrap-ten", 3),
        ("wrap-hundred", 4),
        ("rich-five", 2),
        ("unplaced-target", 2),
    ] {
        with_convergence_probe_fixture(fixture, |input, caps, maximum, mode| {
            let canonical = typaxis_document_package::book_v2::BookV2DocumentPackageEncoder::new()
                .encode(input.body().styled().body().wire())
                .unwrap();
            let original: Value = serde_json::from_str(&canonical).unwrap();
            let authored_census = census(&original);
            let owners = authored_census["main_page_owners"].as_array().unwrap();
            let case = format!("page-label-{fixture}");
            let unplaced_witness = (fixture == "unplaced-target").then(|| {
                let unused = &original["document"]["footnotes"][1];
                let reference = unused["blocks"][1]["children"][0]["node_id"]
                    .as_u64()
                    .unwrap();
                let target = unused["blocks"][0]["node_id"].as_u64().unwrap();
                (
                    NodeId::new(reference.try_into().unwrap()),
                    NodeId::new(target.try_into().unwrap()),
                )
            });
            let mut capture = Capture {
                input,
                caps,
                maximum,
                case: case.clone(),
                directory: destination.clone(),
                pending: None,
                attempts: Vec::new(),
                early_failure_prefixes: Vec::new(),
            };
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
            let before = budget.observation();
            let mut called = 0;
            let (final_bytes, final_display, final_pdf, delivered) =
                with_converged_column_pdf_observer(
                    input,
                    caps,
                    mode,
                    &mut budget,
                    4,
                    |roots, actual| {
                        called += 1;
                        if let Some((owner, target_owner)) = unplaced_witness {
                            let reference = roots
                                .pdf()
                                .page_references()
                                .iter()
                                .find(|r| r.owner() == owner)
                                .unwrap();
                            assert_eq!(reference.candidate_page(), 1);
                            assert_eq!(reference.target_page(), None);
                            assert!(!reference.is_placed());
                            let destination = roots
                                .navigation()
                                .destinations()
                                .iter()
                                .find(|d| d.owner() == target_owner)
                                .unwrap();
                            assert_eq!(destination.position(), None);
                            assert!(matches!(
                                destination.kind(),
                                typaxis_pdf::book_v2::BookV2DestinationKind::Anchor(
                                    "unplaced-page-target"
                                )
                            ));
                            assert!(!roots
                                .pdf()
                                .bytes()
                                .windows(b"unplaced-page-target".len())
                                .any(|w| w == b"unplaced-page-target"));
                        }
                        (
                            roots.pdf().bytes().to_vec(),
                            roots.display().fingerprint(),
                            roots.pdf().fingerprint(),
                            actual,
                        )
                    },
                    &mut capture,
                )
                .unwrap();
            assert_eq!(called, 1);
            assert!(capture.pending.is_none() && capture.early_failure_prefixes.is_empty());
            assert_eq!(capture.attempts.len(), expected_attempts, "{fixture}");
            let after = budget.observation();
            assert_eq!(delivered, after);
            let last = capture.attempts.last().unwrap();
            assert_eq!(last["after_return"]["observation"], observation(after));
            assert_eq!(
                last["after_return"]["cff"],
                statistics(budget.cff_program_statistics())
            );
            assert_eq!(last["display_fingerprint"], probes::hex(&final_display));
            assert_eq!(last["pdf_fingerprint"], probes::hex(&final_pdf));
            assert_eq!(
                last["pdf"]["assembly"]["pdf"]["sha256"],
                probes::hex(&typaxis_core::sha256(&final_bytes))
            );
            assert_eq!(last["decision"], "ready");
            assert_eq!(last["pdf"]["assembly"]["labels_match"], true);
            for attempt in &capture.attempts {
                if let Some((owner, _)) = unplaced_witness {
                    let reference = attempt["references"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|r| r["owner"] == owner.get())
                        .unwrap();
                    assert_eq!(reference["candidate"], 1);
                    assert_eq!(reference["target"], Value::Null);
                    assert_eq!(reference["placed"], false);
                }
                assert_eq!(attempt["after_return"]["outcome_is_ok"], true);
                assert_eq!(attempt["pdf"]["observed"]["caller_after"], Value::Null);
                assert_eq!(attempt["pdf"]["observed"]["scope"], "convergence_attempt");
                assert_eq!(
                    attempt["values"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v[0].clone())
                        .collect::<Vec<_>>(),
                    *owners
                );
                assert_eq!(attempt["source"]["copied"], !owners.is_empty());
            }
            if !owners.is_empty() {
                let earlier = &capture.attempts[capture.attempts.len() - 2];
                assert_eq!(earlier["display_fingerprint"], last["display_fingerprint"]);
                assert_eq!(earlier["pdf_fingerprint"], last["pdf_fingerprint"]);
                assert_eq!(
                    earlier["pdf"]["assembly"]["pdf"],
                    last["pdf"]["assembly"]["pdf"]
                );
            }
            let result = json!({"wire":"typaxis.book-2-column-page-label-convergence-probe/2",
                "case":case,"fixture":fixture,"source_canonical_jcs":canonical,
                "source_census":authored_census,"limits_canonical_jcs":caps.canonical_jcs(),
                "limits_fingerprint":probes::hex(&caps.fingerprint()),
                "observed":{"scope":"converged_column_pdf_driver","maximum_work":maximum,
                    "caller_before":observation(before),"caller_after":observation(after),
                    "final_callback_count":called,"cff_after":statistics(budget.cff_program_statistics())},
                "attempts":capture.attempts,"early_failure_prefixes":capture.early_failure_prefixes,
                "limits_of_evidence":["complete actual successful snapshots; early failure/retry prefixes are CLI controls",
                    "local source/region companions and JSON artifacts are test-host work outside command counters",
                    "TT fixture statistics do not establish a positive original CFF execution gate",
                    "observed Source/display prefixes are issuer-linked witnesses, not allocator traces"]});
            let bytes = serde_json::to_vec_pretty(&result).unwrap();
            let hash = probes::hex(&typaxis_core::sha256(&bytes));
            fs::write(destination.join(format!("{case}-{hash}.json")), bytes).unwrap();
            eprintln!("Column actual captured convergence {fixture}: attempts={expected_attempts}; after={:?}",pdf_quartet(after));
        });
    }
}
