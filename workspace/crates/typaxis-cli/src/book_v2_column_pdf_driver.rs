//! Assemble an inspectable Column PDF on the authentic navigation ledger.
//! The private typed output phase retains signed settlement on every result.
use super::*;
use typaxis_display_list::book_v2::BookV2ColumnDisplay;
use typaxis_pdf::book_v2::{
    BookV2ColumnNavigationGeometry, BookV2ColumnPdfAssembly, BookV2ColumnPdfAssemblyBuilder,
    BookV2ColumnPdfAssemblyContinuation,
};

/// Borrowed roots of one actual assembled Column candidate. This does not grant
/// public-profile, PDF/UA or whole-book acceptance authority.
pub struct BookV2ColumnPdfBundle<
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
> {
    pdf: &'pdf BookV2ColumnPdfAssembly<
        'nav,
        'w,
        'l,
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
    navigation_bundle: &'pdf BookV2ColumnNavigationBundle<
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
    navigation_prefix: BookV2PdfConvergenceObservation,
    previous: BookV2PdfConvergenceObservation,
    constructors: [[u64; 4]; 14],
}
impl<
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
    >
    BookV2ColumnPdfBundle<
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
    >
{
    pub fn pdf(
        &self,
    ) -> &'pdf BookV2ColumnPdfAssembly<
        'nav,
        'w,
        'l,
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
    > {
        self.pdf
    }
    pub fn navigation_bundle(
        &self,
    ) -> &'pdf BookV2ColumnNavigationBundle<
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
    > {
        self.navigation_bundle
    }
    pub fn navigation(
        &self,
    ) -> &'nav BookV2ColumnNavigationGeometry<
        'w,
        'l,
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
    > {
        self.navigation_bundle.navigation()
    }
    pub fn display(&self) -> &'v BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.navigation_bundle.display()
    }
    pub fn navigation_prefix(&self) -> BookV2PdfConvergenceObservation {
        self.navigation_prefix
    }
    pub fn previous_observation(&self) -> BookV2PdfConvergenceObservation {
        self.previous
    }
    pub fn caller_before(&self) -> [u64; 4] {
        self.navigation_bundle.caller_before()
    }
    pub fn constructor_charges(&self) -> [[u64; 4]; 14] {
        self.constructors
    }
}

/// Deliver one actual Column PDF, retaining constructor/build/callback and retry
/// prefixes. The callback can continue only the final assembly builder.
pub fn with_budgeted_book_v2_column_pdf<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    first_font_object: u32,
    inspect: impl for<'loan, 'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnPdfBundle<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPdfAssemblyContinuation<'loan, 'nav, 'w, 'l, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    let maximum = budget.inner.maximum_work;
    let previous = budget.inner.observation;
    let mut progress = previous;
    let outcome = super::navigation_driver::with_column_navigation_phase(
        input,
        limits,
        japanese_mode,
        budget,
        first_font_object,
        |navigation, _parent_builder, navigation_observed, phase| {
            with_column_pdf_from_navigation(
                navigation,
                limits,
                maximum,
                navigation_observed,
                previous,
                &mut progress,
                phase,
                |roots, builder, observed, _phase, _progress| inspect(roots, builder, observed),
            )
        },
    );
    budget.inner.observation.records = budget.inner.observation.records.max(progress.records);
    budget.inner.observation.spool = budget.inner.observation.spool.max(progress.spool);
    budget.inner.observation.work = budget.inner.observation.work.max(progress.work);
    // The shared typed paint phase retains exact output on every result.
    outcome?
}

/// Continue one authentic parent without rebuilding any earlier stage.
/// Parent lifetimes remain fixed; only roots created here are scoped to inspect.
/// The caller owns progress so accepted descendant prefixes survive nested Err.
#[allow(clippy::too_many_arguments)]
pub(super) fn with_column_pdf_from_navigation<
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
    R,
>(
    navigation: &BookV2ColumnNavigationBundle<
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
    limits: &M4EffectiveResourceLimits,
    maximum: u64,
    navigation_observed: BookV2PdfConvergenceObservation,
    previous: BookV2PdfConvergenceObservation,
    progress: &mut BookV2PdfConvergenceObservation,
    phase: &mut super::paints_driver::BookV2ColumnOutputPhase,
    inspect: impl for<'loan, 'pdf> FnOnce(
        &BookV2ColumnPdfBundle<
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
        &mut BookV2ColumnPdfAssemblyContinuation<
            'loan,
            'nav,
            'w,
            'l,
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
        BookV2PdfConvergenceObservation,
        &mut super::paints_driver::BookV2ColumnOutputPhase,
        &mut BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    let (mut records, mut spool, mut work) = (
        progress.records.max(navigation_observed.records),
        progress.spool.max(navigation_observed.spool),
        progress.work.max(navigation_observed.work),
    );
    let mut output = navigation_observed.output;
    *progress = BookV2PdfConvergenceObservation {
        records,
        spool,
        output,
        work,
        ..navigation_observed
    };
    let outcome = (|| {
        let mut constructors = [[0; 4]; 14];
        constructors[..13].copy_from_slice(&navigation.constructor_charges());
        let mut assembly_builder = BookV2ColumnPdfAssemblyBuilder::new_counted(
            navigation.navigation(),
            limits,
            maximum,
            records,
            spool,
            output,
            work,
            &mut records,
            &mut spool,
            &mut output,
            &mut work,
        )
        .map_err(|e| stage("column PDF assembly constructor", e))?;
        constructors[13] = [records, spool, output, work];
        macro_rules! retain {
            () => {{
                records = records.max(assembly_builder.record_charge());
                spool = spool.max(assembly_builder.spool_charge());
                work = work.max(assembly_builder.work_steps());
            }};
        }
        phase.bind_assembly_attempt(assembly_builder.attempt_identity())?;
        let assembled = assembly_builder.build();
        retain!();
        phase.retain_assembly(&assembly_builder)?;
        let pdf = assembled.map_err(|e| stage("column PDF assembly", e))?;
        let roots = BookV2ColumnPdfBundle {
            pdf: &pdf,
            navigation_bundle: navigation,
            navigation_prefix: navigation_observed,
            previous,
            constructors,
        };
        let observed = BookV2PdfConvergenceObservation {
            records,
            spool,
            output: assembly_builder.output_charge(),
            work,
            ..navigation_observed
        };
        progress.records = progress.records.max(records);
        progress.spool = progress.spool.max(spool);
        progress.work = progress.work.max(work);
        // A descendant may have settled output below this ancestor's floor.
        // Read only the authentic typed phase; never revive gross output.
        progress.output = phase.accepted_output().unwrap_or(output);
        let result = assembly_builder.with_continuation(|continuation| {
            inspect(&roots, continuation, observed, phase, progress)
        });
        retain!();
        phase.retain_assembly(&assembly_builder)?;
        Ok(result)
    })();
    progress.records = progress.records.max(records);
    progress.spool = progress.spool.max(spool);
    progress.work = progress.work.max(work);
    // A descendant may have settled output below this ancestor's floor.
    // Read only the authentic typed phase; never revive gross output.
    progress.output = phase.accepted_output().unwrap_or(output);
    outcome
}
