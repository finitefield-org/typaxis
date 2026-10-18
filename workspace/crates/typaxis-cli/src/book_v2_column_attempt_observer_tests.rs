use super::*;

#[derive(Default)]
struct ActualRootsObserver {
    attempts: Vec<Attempt>,
    sources: Vec<ColumnAttemptSourceObservation>,
    assemblies: Vec<([u64; 4], [usize; 4])>,
    returned: Vec<(
        BookV2PdfConvergenceObservation,
        typaxis_resources::book_v2::BookV2CffProgramStatistics,
        bool,
    )>,
}
impl ColumnAttemptObserver for ActualRootsObserver {
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
        roots: &'event BookV2ColumnPdfBundle<
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
        let pdf = roots.pdf();
        assert_eq!(roots.display().fingerprint(), attempt.display_fingerprint);
        assert_eq!(pdf.fingerprint(), attempt.pdf_fingerprint);
        assert_eq!(roots.constructor_charges(), attempt.constructors);
        assert!(std::ptr::eq(pdf.bytes(), attempt.pdf_bytes));
        assert!(std::ptr::eq(pdf.page_references(), attempt.references));
        assert_eq!(
            attempt.assembly_before_labels,
            [
                pdf.record_charge(),
                pdf.spool_charge(),
                pdf.output_charge(),
                pdf.work_steps(),
            ]
        );
        assert!(attempt
            .assembly_reservation_sizes
            .iter()
            .all(|size| *size > 0));
        assert_eq!(attempt.observation.record_charge(), pdf.record_charge());
        assert_eq!(attempt.observation.spool_charge(), pdf.spool_charge());
        assert_eq!(attempt.observation.output_charge(), pdf.output_charge());
        assert!(attempt.observation.work_steps() > pdf.work_steps());
        assert_outer_history_monotone(attempt.source.census_before, attempt.source.census_after);
        assert_outer_history_monotone(attempt.source.census_after, attempt.source.source_before);
        assert_outer_history_monotone(attempt.source.source_before, attempt.source.source_after);
        assert_eq!(
            attempt.source.source_after.output_charge(),
            attempt.source.source_before.output_charge()
        );
        if attempt.source.copied {
            assert!(!attempt.values.is_empty());
            assert!(
                attempt.source.source_after.record_charge()
                    > attempt.source.source_before.record_charge()
            );
            assert!(
                attempt.source.source_after.spool_charge()
                    > attempt.source.source_before.spool_charge()
            );
            assert!(
                attempt.source.source_after.work_steps()
                    > attempt.source.source_before.work_steps()
            );
        } else {
            assert!(attempt.values.is_empty());
            assert_eq!(attempt.source.source_before, attempt.source.source_after);
        }
        self.sources.push(attempt.source);
        self.assemblies.push((
            attempt.assembly_before_labels,
            attempt.assembly_reservation_sizes,
        ));
        self.attempts.push(Attempt::from_actual(attempt));
    }

    fn after_attempt(
        &mut self,
        actual: BookV2PdfConvergenceObservation,
        statistics: typaxis_resources::book_v2::BookV2CffProgramStatistics,
        outcome_is_ok: bool,
    ) {
        if let Some(complete) = self.attempts.get(self.returned.len()) {
            assert_outer_history_monotone(complete.observation, actual);
            assert_eq!(complete.observation.output_charge(), actual.output_charge());
        }
        self.returned.push((actual, statistics, outcome_is_ok));
    }
}

#[test]
fn book_v2_column_converged_pdf_static_observer_borrows_actual_roots_without_changing_attempts() {
    for page_sites in [false, true] {
        let root = Root::new();
        let caps = caps();
        let authored = if page_sites {
            forward_data(9).0
        } else {
            data(false)
        };
        let prepared = input(&root, &authored, &caps);
        let mut captured = ActualRootsObserver::default();
        let mut actual_budget = BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
        let actual = with_converged_column_pdf_observer(
            &prepared,
            &caps,
            MODE,
            &mut actual_budget,
            4,
            |bundle, observed| {
                (
                    (
                        bundle.pdf().bytes().to_vec(),
                        bundle.display().fingerprint(),
                        bundle.pdf().fingerprint(),
                    ),
                    observed,
                )
            },
            &mut captured,
        )
        .unwrap();
        let mut adapter_budget = BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
        let mut adapted = Vec::new();
        let expected = with_converged_column_pdf_observed(
            &prepared,
            &caps,
            MODE,
            &mut adapter_budget,
            4,
            |bundle, observed| {
                (
                    (
                        bundle.pdf().bytes().to_vec(),
                        bundle.display().fingerprint(),
                        bundle.pdf().fingerprint(),
                    ),
                    observed,
                )
            },
            |attempt| adapted.push(Attempt::from_actual(attempt)),
        )
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(captured.attempts, adapted);
        assert_eq!(actual_budget.observation(), adapter_budget.observation());
        assert_eq!(actual.1, actual_budget.observation());
        assert_eq!(captured.sources.len(), captured.attempts.len());
        assert_eq!(captured.assemblies.len(), captured.attempts.len());
        assert_eq!(captured.returned.len(), captured.attempts.len());
        assert!(captured.returned.iter().all(|returned| returned.2));
        assert_eq!(
            captured.returned.last().unwrap().0,
            actual_budget.observation()
        );
        assert_eq!(
            captured.returned.last().unwrap().1,
            actual_budget.cff_program_statistics()
        );
        for source in &captured.sources {
            assert_eq!(source.copied, page_sites);
            assert_eq!(source.census_before, captured.sources[0].census_before);
            assert_eq!(source.census_after, captured.sources[0].census_after);
        }
        // A genuine shortened work budget fails during an actual attempt. Its
        // post-return observation survives even if no complete event was issued.
        let mut short_budget = BookV2ColumnPdfConvergenceBudget::new(
            &caps,
            actual_budget.observation().work_steps() - 1,
        );
        let mut short = ActualRootsObserver::default();
        let mut delivered = false;
        let rejected = with_converged_column_pdf_observer(
            &prepared,
            &caps,
            MODE,
            &mut short_budget,
            4,
            |_, _| {
                delivered = true;
            },
            &mut short,
        );
        assert!(rejected.is_err() && !delivered);
        let stopped = short
            .returned
            .last()
            .expect("actual attempted work refusal");
        assert!(!stopped.2);
        assert_eq!(stopped.0, short_budget.observation());
        assert_eq!(stopped.1, short_budget.cff_program_statistics());
        // An opaque application Result is not the outer attempt Result status.
        let mut application_budget = BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
        let mut application = ActualRootsObserver::default();
        let application_result = with_converged_column_pdf_observer(
            &prepared,
            &caps,
            MODE,
            &mut application_budget,
            4,
            |_, _| Err::<(), &str>("intentional application error"),
            &mut application,
        )
        .unwrap();
        assert_eq!(application_result, Err("intentional application error"));
        assert!(application.returned.iter().all(|returned| returned.2));
        assert_eq!(
            application.returned.last().unwrap().0,
            application_budget.observation()
        );
        eprintln!(
            "Column actual immutable-root observer: Page={page_sites}; attempts={}; after={:?}",
            captured.attempts.len(),
            actual_budget.observation()
        );
    }
}
