//! Navigation geometry extends authentic Column relations on the same ledger.
//! Marked content owns output settlement; navigation adds no output credit.
use super::*;
use typaxis_display_list::book_v2::BookV2ColumnDisplay;
use typaxis_pdf::book_v2::{
    BookV2ColumnNavigationGeometry, BookV2ColumnNavigationGeometryBuilder,
    BookV2ColumnNavigationGeometryContinuation, BookV2ColumnStructureRelations,
};

pub struct BookV2ColumnNavigationBundle<
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
    navigation: &'nav BookV2ColumnNavigationGeometry<
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
    relation_bundle: &'nav BookV2ColumnRelationBundle<
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
    relations_prefix: BookV2PdfConvergenceObservation,
    constructors: [[u64; 4]; 13],
}
impl<
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
    BookV2ColumnNavigationBundle<
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
        self.navigation
    }
    pub fn relations(
        &self,
    ) -> &'w BookV2ColumnStructureRelations<
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
        self.relation_bundle.relations()
    }
    pub fn relation_bundle(
        &self,
    ) -> &'nav BookV2ColumnRelationBundle<
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
        self.relation_bundle
    }
    pub fn marked(
        &self,
    ) -> &'w BookV2ColumnMarkedBundle<
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
        self.relation_bundle.marked()
    }
    pub fn display(&self) -> &'v BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.relation_bundle.display()
    }
    pub fn caller_before(&self) -> [u64; 4] {
        self.relation_bundle.caller_before()
    }
    pub fn encoded_prefix(&self) -> BookV2PdfConvergenceObservation {
        self.relation_bundle.encoded_prefix()
    }
    pub fn marked_prefix(&self) -> BookV2PdfConvergenceObservation {
        self.relation_bundle.marked_prefix()
    }
    pub fn relations_prefix(&self) -> BookV2PdfConvergenceObservation {
        self.relations_prefix
    }
    pub fn constructor_charges(&self) -> [[u64; 4]; 13] {
        self.constructors
    }
}

/// Deliver navigation after actual marked content, hierarchy and relations.
/// Accepted constructor, build and callback records/spool/work survive errors.
/// The callback can continue only the final navigation builder.
pub fn with_budgeted_book_v2_column_navigation<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    first_font_object: u32,
    inspect: impl for<'loan, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnNavigationBundle<'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnNavigationGeometryContinuation<'loan, 'w, 'l, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    with_column_navigation_phase(
        input,
        limits,
        japanese_mode,
        budget,
        first_font_object,
        |roots, builder, observed, _phase| {
            builder.with_continuation(|loan| inspect(roots, loan, observed))
        },
    )
}

pub(super) fn with_column_navigation_phase<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    first_font_object: u32,
    inspect: impl for<'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnNavigationBundle<'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnNavigationGeometryBuilder<'w, 'l, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
        &mut super::paints_driver::BookV2ColumnOutputPhase,
    ) -> R,
) -> Result<R, E> {
    let maximum = budget.inner.maximum_work;
    let prior = budget.inner.observation;
    let mut progress = prior;
    let outcome = super::relations_driver::with_column_relations_phase(
        input,
        limits,
        japanese_mode,
        budget,
        first_font_object,
        |relations, _parent_builder, observed, phase| {
            with_column_navigation_from_relations(
                relations,
                limits,
                maximum,
                observed,
                &mut progress,
                phase,
                |roots, builder, observed, phase, _progress| {
                    inspect(roots, builder, observed, phase)
                },
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
pub(super) fn with_column_navigation_from_relations<
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
    relations: &BookV2ColumnRelationBundle<
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
    observed: BookV2PdfConvergenceObservation,
    progress: &mut BookV2PdfConvergenceObservation,
    phase: &mut super::paints_driver::BookV2ColumnOutputPhase,
    inspect: impl for<'nav> FnOnce(
        &BookV2ColumnNavigationBundle<
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
        &mut BookV2ColumnNavigationGeometryBuilder<
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
        progress.records.max(observed.records),
        progress.spool.max(observed.spool),
        progress.work.max(observed.work),
    );
    let mut output = observed.output;
    *progress = BookV2PdfConvergenceObservation {
        records,
        spool,
        output,
        work,
        ..observed
    };
    let outcome = (|| {
        let mut constructors = [[0; 4]; 13];
        constructors[..12].copy_from_slice(&relations.constructor_charges());
        let mut navigation_builder = BookV2ColumnNavigationGeometryBuilder::new_counted(
            relations.relations(),
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
        .map_err(|e| stage("column navigation constructor", e))?;
        constructors[12] = [records, spool, output, work];
        macro_rules! retain {
            () => {{
                records = records.max(navigation_builder.record_charge());
                spool = spool.max(navigation_builder.spool_charge());
                work = work.max(navigation_builder.work_steps());
            }};
        }
        let navigation_result = navigation_builder.build();
        retain!();
        let navigation = navigation_result.map_err(|e| stage("column navigation geometry", e))?;
        phase.bind_navigation(&navigation)?;
        let roots = BookV2ColumnNavigationBundle {
            navigation: &navigation,
            relation_bundle: relations,
            relations_prefix: observed,
            constructors,
        };
        let navigation_observed = BookV2PdfConvergenceObservation {
            records,
            spool,
            work,
            ..observed
        };
        progress.records = progress.records.max(records);
        progress.spool = progress.spool.max(spool);
        progress.work = progress.work.max(work);
        // A descendant may have settled output below this ancestor's floor.
        // Read only the authentic typed phase; never revive gross output.
        progress.output = phase.accepted_output().unwrap_or(output);
        let result = inspect(
            &roots,
            &mut navigation_builder,
            navigation_observed,
            phase,
            progress,
        );
        retain!();
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
