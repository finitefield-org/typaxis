//! Source relations extend authentic marked Column roots on the same ledger.
//! Marked content already owns output settlement; relations add no output credit.
use super::*;
use typaxis_display_list::book_v2::BookV2ColumnDisplay;
use typaxis_pdf::book_v2::{
    BookV2ColumnStructureRelationBuilder, BookV2ColumnStructureRelationContinuation,
    BookV2ColumnStructureRelations,
};

pub struct BookV2ColumnRelationBundle<
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
    relations: &'w BookV2ColumnStructureRelations<
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
    marked: &'w BookV2ColumnMarkedBundle<
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
    marked_prefix: BookV2PdfConvergenceObservation,
    constructors: [[u64; 4]; 12],
}
impl<
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
    BookV2ColumnRelationBundle<
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
    pub fn display(&self) -> &'v BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.marked.display()
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
        self.marked
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
        self.relations
    }
    pub fn caller_before(&self) -> [u64; 4] {
        self.marked.caller_before()
    }
    pub fn constructor_charges(&self) -> [[u64; 4]; 12] {
        self.constructors
    }
    pub fn encoded_prefix(&self) -> BookV2PdfConvergenceObservation {
        self.marked.encoded_prefix()
    }
    pub fn marked_prefix(&self) -> BookV2PdfConvergenceObservation {
        self.marked_prefix
    }
}

/// Deliver source relations after actual marked content and source hierarchy.
/// Accepted constructor, build and callback records/spool/work survive errors.
/// The callback can continue only the final relation builder.
pub fn with_budgeted_book_v2_column_relations<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    first_font_object: u32,
    inspect: impl for<'loan, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnRelationBundle<'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnStructureRelationContinuation<'loan, 'l, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    with_column_relations_phase(
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

pub(super) fn with_column_relations_phase<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    first_font_object: u32,
    inspect: impl for<'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnRelationBundle<'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnStructureRelationBuilder<'l, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
        &mut super::paints_driver::BookV2ColumnOutputPhase,
    ) -> R,
) -> Result<R, E> {
    let maximum = budget.inner.maximum_work;
    let prior = budget.inner.observation;
    let mut progress = prior;
    let outcome = super::marked_driver::with_column_marked_phase(
        input,
        limits,
        japanese_mode,
        budget,
        first_font_object,
        |marked, _parent_builder, observed, phase| {
            with_column_relations_from_marked(
                marked,
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
pub(super) fn with_column_relations_from_marked<
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
    marked: &BookV2ColumnMarkedBundle<
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
    inspect: impl for<'w> FnOnce(
        &BookV2ColumnRelationBundle<
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
        &mut BookV2ColumnStructureRelationBuilder<
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
        let mut constructors = [[0; 4]; 12];
        constructors[..11].copy_from_slice(&marked.constructor_charges());
        let mut relation_builder = BookV2ColumnStructureRelationBuilder::new_counted(
            marked.structure(),
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
        .map_err(|e| stage("column structure relation constructor", e))?;
        constructors[11] = [records, spool, output, work];
        macro_rules! retain {
            () => {{
                records = records.max(relation_builder.record_charge());
                spool = spool.max(relation_builder.spool_charge());
                work = work.max(relation_builder.work_steps());
            }};
        }
        let relations_result = relation_builder.build();
        retain!();
        let relations = relations_result.map_err(|e| stage("column structure relations", e))?;
        let roots = BookV2ColumnRelationBundle {
            relations: &relations,
            marked,
            marked_prefix: observed,
            constructors,
        };
        let relation_observed = BookV2PdfConvergenceObservation {
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
            &mut relation_builder,
            relation_observed,
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
