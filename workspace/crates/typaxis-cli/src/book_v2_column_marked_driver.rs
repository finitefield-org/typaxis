//! Marked content and original source hierarchy share authentic Column roots.
use super::*;
use typaxis_display_list::book_v2::BookV2ColumnDisplay;
use typaxis_pdf::book_v2::{
    BookV2ColumnMarkedContent, BookV2ColumnMarkedContentBuilder, BookV2ColumnMarkedScopeBuilder,
    BookV2ColumnSourceStructure, BookV2ColumnSourceStructureBuilder,
    BookV2ColumnSourceStructureContinuation,
};

pub struct BookV2ColumnMarkedBundle<
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
    structure: &'l BookV2ColumnSourceStructure<
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
    paints: &'l BookV2ColumnPaintBundle<
        'u,
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
    encoded: BookV2PdfConvergenceObservation,
    constructors: [[u64; 4]; 11],
}
impl<
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
    BookV2ColumnMarkedBundle<
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
        self.paints.display()
    }
    pub fn paints(
        &self,
    ) -> &'l BookV2ColumnPaintBundle<
        'u,
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
        self.paints
    }
    pub fn content(
        &self,
    ) -> &'h BookV2ColumnMarkedContent<
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
        self.structure.source()
    }
    pub fn structure(
        &self,
    ) -> &'l BookV2ColumnSourceStructure<
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
        self.structure
    }
    pub fn caller_before(&self) -> [u64; 4] {
        self.paints.caller_before()
    }
    pub fn constructor_charges(&self) -> [[u64; 4]; 11] {
        self.constructors
    }
    pub fn encoded_prefix(&self) -> BookV2PdfConvergenceObservation {
        self.encoded
    }
}

pub fn with_budgeted_book_v2_column_marked<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    first_font_object: u32,
    inspect: impl for<'loan, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnMarkedBundle<'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnSourceStructureContinuation<'loan, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    with_column_marked_phase(
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

pub(super) fn with_column_marked_phase<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    first_font_object: u32,
    inspect: impl for<'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnMarkedBundle<'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnSourceStructureBuilder<'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
        &mut super::paints_driver::BookV2ColumnOutputPhase,
    ) -> R,
) -> Result<R, E> {
    let maximum = budget.inner.maximum_work;
    let prior = budget.inner.observation;
    let mut progress = prior;
    let outcome = super::paints_driver::with_column_paint_phase(
        input,
        limits,
        japanese_mode,
        budget,
        first_font_object,
        |paints, _parent_builder, encoded, phase| {
            with_column_marked_from_paints(
                paints,
                limits,
                maximum,
                encoded,
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
pub(super) fn with_column_marked_from_paints<
    'u,
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
    paints: &BookV2ColumnPaintBundle<
        'u,
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
    encoded: BookV2PdfConvergenceObservation,
    progress: &mut BookV2PdfConvergenceObservation,
    phase: &mut super::paints_driver::BookV2ColumnOutputPhase,
    inspect: impl for<'l, 'h, 'n, 'm> FnOnce(
        &BookV2ColumnMarkedBundle<
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
        &mut BookV2ColumnSourceStructureBuilder<
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
        progress.records.max(encoded.records),
        progress.spool.max(encoded.spool),
        progress.work.max(encoded.work),
    );
    let mut output = encoded.output;
    *progress = BookV2PdfConvergenceObservation {
        records,
        spool,
        output,
        work,
        ..encoded
    };
    let outcome = (|| {
        macro_rules! retain {
            ($builder:ident) => {{
                records = records.max($builder.record_charge());
                spool = spool.max($builder.spool_charge());
                work = work.max($builder.work_steps());
                // This is the authentic phase observation, including net
                // output reserved before a subsequent work/allocation error.
                output = $builder.output_charge();
            }};
        }
        let mut constructors = [[0; 4]; 11];
        constructors[..8].copy_from_slice(&paints.constructor_charges());
        let mut scopes_builder = BookV2ColumnMarkedScopeBuilder::new_counted(
            paints.images(),
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
        .map_err(|e| stage("column marked scope constructor", e))?;
        constructors[8] = [records, spool, output, work];
        let scopes_result = scopes_builder.build();
        retain!(scopes_builder);
        phase.retain_scopes(&scopes_builder);
        let scopes = scopes_result.map_err(|e| stage("column marked scopes", e))?;
        let mut content_builder = BookV2ColumnMarkedContentBuilder::new_counted(
            &scopes,
            paints.text(),
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
        .map_err(|e| stage("column marked content constructor", e))?;
        constructors[9] = [records, spool, output, work];
        let content_result = content_builder.build();
        retain!(content_builder);
        phase.retain_content(&content_builder);
        let content = content_result.map_err(|e| stage("column marked content", e))?;
        let mut structure_builder = BookV2ColumnSourceStructureBuilder::new_counted(
            &content,
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
        .map_err(|e| stage("column source structure constructor", e))?;
        constructors[10] = [records, spool, output, work];
        let structure_result = structure_builder.build();
        retain!(structure_builder);
        let structure = structure_result.map_err(|e| stage("column source structure", e))?;
        let roots = BookV2ColumnMarkedBundle {
            structure: &structure,
            paints,
            encoded,
            constructors,
        };
        let observed = BookV2PdfConvergenceObservation {
            records,
            spool,
            output,
            work,
            ..encoded
        };
        progress.records = progress.records.max(records);
        progress.spool = progress.spool.max(spool);
        progress.work = progress.work.max(work);
        // A descendant may have settled output below this ancestor's floor.
        // Read only the authentic typed phase; never revive gross output.
        progress.output = phase.accepted_output().unwrap_or(output);
        let result = inspect(&roots, &mut structure_builder, observed, phase, progress);
        retain!(structure_builder);
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
