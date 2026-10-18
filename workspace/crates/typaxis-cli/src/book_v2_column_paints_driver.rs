//! Emit typed column paint contributions while retaining every accepted prefix.
//! Complete marked content, structure, page resources and PDF authority follow.
use super::*;
use typaxis_display_list::book_v2::BookV2ColumnDisplay;
use typaxis_pdf::book_v2::{
    BookV2ColumnFontObjectBuilder, BookV2ColumnFontStreamBuilder, BookV2ColumnImageCommandBuilder,
    BookV2ColumnImageCommandContinuation, BookV2ColumnImageCommands,
    BookV2ColumnImageObjectBuilder, BookV2ColumnTextCommandBuilder, BookV2ColumnTextCommands,
    BookV2ColumnVectorProgramBuilder,
};
use typaxis_resources::book_v2::{
    BookV2ColumnCidPlans, BookV2ColumnFontSelectionBuilder, BookV2ColumnRasterPrograms,
};

#[path = "book_v2_column_font_stages.rs"]
mod font_stages;

/// Borrowed paint roots from the same authentic column display. Object ranges
/// here cover contributions only; a complete document owner must bind them.
pub struct BookV2ColumnPaintBundle<
    'h,
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
    text: &'h BookV2ColumnTextCommands<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    images: &'h BookV2ColumnImageCommands<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    caller_before: [u64; 4],
    constructors: [[u64; 4]; 8],
    reservation_sizes: [usize; 2],
}
impl<'h, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2ColumnPaintBundle<
        'h,
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
        self.text.display()
    }
    pub fn text(
        &self,
    ) -> &'h BookV2ColumnTextCommands<'t, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    {
        self.text
    }
    pub fn images(
        &self,
    ) -> &'h BookV2ColumnImageCommands<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.images
    }
    pub fn cids(&self) -> &'z BookV2ColumnCidPlans<'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.text.source().source().source()
    }
    pub fn rasters(
        &self,
    ) -> &'r BookV2ColumnRasterPrograms<'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.images.source().source().source()
    }
    /// Actual prefix before source selection, in record/spool/output/work order.
    pub fn caller_before(&self) -> [u64; 4] {
        self.caller_before
    }
    /// Accepted prefixes after font selection, font streams, font objects,
    /// text commands, image handoff, vector programs, image objects and image
    /// commands constructors, in record/spool/output/work order.
    pub fn constructor_charges(&self) -> [[u64; 4]; 8] {
        self.constructors
    }
    /// Actual private font resource buffer units, in bytes, from the builder.
    pub fn reservation_sizes(&self) -> [usize; 2] {
        self.reservation_sizes
    }
}

pub fn with_budgeted_book_v2_column_paints<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    first_font_object: u32,
    inspect: impl for<'loan, 'h, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnPaintBundle<'h, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnImageCommandContinuation<'loan, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    with_column_paint_phase(
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

/// Only an authentic typed marked-content builder can settle raw contributions.
/// This private bridge has no numeric refund setter in the public paint API.
#[derive(Default)]
pub(super) struct BookV2ColumnOutputPhase {
    accepted: Option<u64>,
    paints: Option<[*const (); 3]>,
    navigation: Option<*const ()>,
    assembly_attempt: Option<typaxis_pdf::book_v2::BookV2PdfAssemblyAttemptIdentity>,
}
impl BookV2ColumnOutputPhase {
    pub(super) fn accepted_output(&self) -> Option<u64> {
        self.accepted
    }
    pub(super) fn bind_assembly_attempt(
        &mut self,
        attempt: typaxis_pdf::book_v2::BookV2PdfAssemblyAttemptIdentity,
    ) -> Result<(), E> {
        if self.assembly_attempt.is_some_and(|bound| bound != attempt) {
            return Err(E::Identity);
        }
        self.assembly_attempt = Some(attempt);
        Ok(())
    }
    // Pointer tokens are compared only while these borrowed roots are alive;
    // they are never dereferenced and grant no public source authority.
    fn bind_paints(
        &mut self,
        roots: &BookV2ColumnPaintBundle<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
    ) -> Result<(), E> {
        let actual = [
            std::ptr::from_ref(roots.display()).cast::<()>(),
            std::ptr::from_ref(roots.text()).cast::<()>(),
            std::ptr::from_ref(roots.images()).cast::<()>(),
        ];
        if self.paints.is_some_and(|bound| bound != actual) {
            return Err(E::Identity);
        }
        self.paints = Some(actual);
        Ok(())
    }
    pub(super) fn bind_navigation(
        &mut self,
        navigation: &typaxis_pdf::book_v2::BookV2ColumnNavigationGeometry<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
    ) -> Result<(), E> {
        let content = navigation.source().source().source();
        let scopes = content.source();
        let actual = [
            std::ptr::from_ref(scopes.display()).cast::<()>(),
            std::ptr::from_ref(content.text()).cast::<()>(),
            std::ptr::from_ref(scopes.source()).cast::<()>(),
        ];
        if self.paints != Some(actual) {
            return Err(E::Identity);
        }
        let actual_navigation = std::ptr::from_ref(navigation).cast::<()>();
        if self
            .navigation
            .is_some_and(|bound| bound != actual_navigation)
        {
            return Err(E::Identity);
        }
        self.navigation = Some(actual_navigation);
        Ok(())
    }
    pub(super) fn retain_assembly(
        &mut self,
        builder: &typaxis_pdf::book_v2::BookV2ColumnPdfAssemblyBuilder<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
    ) -> Result<(), E> {
        if self.assembly_attempt != Some(builder.attempt_identity()) {
            return Err(E::Identity);
        }
        let navigation = builder.navigation();
        if self.navigation != Some(std::ptr::from_ref(navigation).cast::<()>()) {
            return Err(E::Identity);
        }
        self.bind_navigation(navigation)?;
        // This typed builder owns its signed settlement even if a later
        // allocation/work/fold or callback failed after output reservation.
        self.accepted = Some(builder.output_charge());
        Ok(())
    }
    pub(super) fn retain_scopes(
        &mut self,
        builder: &typaxis_pdf::book_v2::BookV2ColumnMarkedScopeBuilder<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
    ) {
        // Scope delimiters are accepted output even when a later stage cannot
        // begin. Only the subsequent typed content owner can settle them.
        self.accepted = Some(builder.output_charge());
    }
    pub(super) fn retain_content(
        &mut self,
        builder: &typaxis_pdf::book_v2::BookV2ColumnMarkedContentBuilder<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
    ) {
        // The builder exposes its genuine reservation even after a later
        // allocation/work error. Before reservation this remains the raw floor.
        self.accepted = Some(builder.output_charge());
    }
}

pub(super) fn with_column_paint_phase<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    first_font_object: u32,
    inspect: impl for<'h, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnPaintBundle<'h, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnImageCommandBuilder<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
        &mut BookV2ColumnOutputPhase,
    ) -> R,
) -> Result<R, E> {
    let maximum = budget.inner.maximum_work;
    let before = budget.inner.observation;
    let (mut records, mut spool, mut output, mut work) =
        (before.records, before.spool, before.output, before.work);
    let caller_before = [records, spool, output, work];
    let mut phase = BookV2ColumnOutputPhase::default();
    let outcome = with_budgeted_book_v2_column_display(
        input,
        limits,
        japanese_mode,
        budget,
        |display, _display_builder, observed| {
            records = records.max(observed.records);
            spool = spool.max(observed.spool);
            output = output.max(observed.output);
            work = work.max(observed.work);
            let mut fonts = BookV2ColumnFontSelectionBuilder::new_counted(
                display,
                limits,
                maximum,
                records,
                spool,
                work,
                &mut records,
                &mut spool,
                &mut work,
            )
            .map_err(|e| stage("column paint font constructor", e))?;
            let mut progress = BookV2PdfConvergenceObservation {
                records,
                spool,
                output,
                work,
                ..observed
            };
            let result = font_stages::with_font_paints(
                &mut fonts,
                limits,
                maximum,
                first_font_object,
                caller_before,
                &mut progress,
                &mut phase,
                |roots, builder, observed, phase, _progress| {
                    inspect(roots, builder, observed, phase)
                },
            );
            records = records.max(progress.records);
            spool = spool.max(progress.spool);
            output = output.max(progress.output);
            work = work.max(progress.work);
            result
        },
    );
    budget.inner.observation.records = budget.inner.observation.records.max(records);
    budget.inner.observation.spool = budget.inner.observation.spool.max(spool);
    budget.inner.observation.output = phase
        .accepted
        .unwrap_or_else(|| budget.inner.observation.output.max(output));
    budget.inner.observation.work = budget.inner.observation.work.max(work);
    outcome?
}

/// Continue an authentic joined Display using the command's retained CFF state.
/// Constructor observations and the real carrier history are merged before
/// either layer of Result is propagated. Every borrowed root stays in its loan.
#[allow(clippy::too_many_arguments)]
pub(super) fn with_column_paints_from_session<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a, R>(
    display: &BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    session: &mut typaxis_resources::book_v2::BookV2FontCommandSession,
    limits: &M4EffectiveResourceLimits,
    maximum: u64,
    first_font_object: u32,
    caller_before: [u64; 4],
    progress: &mut BookV2PdfConvergenceObservation,
    phase: &mut BookV2ColumnOutputPhase,
    inspect: impl for<'h, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v> FnOnce(
        &BookV2ColumnPaintBundle<
            'h,
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
        &mut BookV2ColumnImageCommandBuilder<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
        &mut BookV2ColumnOutputPhase,
        &mut BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    let (mut records, mut spool, mut work) = (progress.records, progress.spool, progress.work);
    let result = session.with_builder_counted(
        display,
        limits,
        records,
        spool,
        work,
        &mut records,
        &mut spool,
        &mut work,
        |fonts| {
            font_stages::with_font_paints(
                fonts,
                limits,
                maximum,
                first_font_object,
                caller_before,
                progress,
                phase,
                inspect,
            )
        },
    );
    progress.records = progress.records.max(records).max(session.record_charge());
    progress.spool = progress.spool.max(spool).max(session.spool_charge());
    progress.work = progress.work.max(work).max(session.work_steps());
    result.map_err(|e| stage("column paint font constructor", e))?
}
