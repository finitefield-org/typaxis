//! The trusted physical consumer finishes every Search mutation and retains
//! its counters before downstream display/region/PDF work can spend Source.
use super::*;

struct PhysicalMath<F>(F);
impl<'id, 'cells, R, F> Delivery<'id, 'cells, R> for PhysicalMath<F>
where
    F: for<'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut ColumnProgressView<'_, 'id, 'cells>,
    ) -> Result<R, E>,
{
    fn finish<'b, 'f, 's, 'p, 'a>(
        self,
        _: &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        search: &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        feedback: &BookV2ColumnWidthFeedback<'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        progress: &mut ColumnProgressView<'_, 'id, 'cells>,
        mark: &mut ColumnSearchChargeMark,
    ) -> Result<R, E> {
        let remaining = limits
            .base()
            .get()
            .max_layout_passes
            .checked_sub(*progress.rest.page_passes)
            .ok_or(E::Limit("observed physical passes"))?;
        let mut begun = 0;
        let pages = search.select_stable_column_pages_counted(remaining, &mut begun);
        let retained = retain_pages(progress, begun, limits);
        let pages = pages.map_err(E::ObservedPagination)?;
        retained?;
        *progress.rest.width_passes = progress
            .rest
            .width_passes
            .checked_add(1)
            .ok_or(E::Limit("observed physical width passes"))?;
        let physical = search
            .paragraph_frame_feedback(pages.sequence())
            .map_err(E::ObservedPagination)?;
        if physical.assignment_fingerprint() != feedback.assignment_fingerprint()
            || !physical.matches_selected_line_widths()
            || !physical.matches_selected_block_widths()
        {
            return Err(E::Identity);
        }
        let geometry = search
            .place_column_pages(pages.sequence())
            .map_err(E::ObservedPagination)?;
        let source = search
            .close_column_page_sources(&pages, &geometry)
            .map_err(E::ObservedPagination)?;
        let terminals = search
            .finalize_column_page_math(source, limits, progress.accounting.counters()[1])
            .map_err(E::ObservedPagination)?;
        retain_search(progress, search, mark)?;
        *progress.rest.candidates = progress
            .rest
            .candidates
            .checked_add(1)
            .ok_or(E::Limit("observed physical candidates"))?;
        // The search stays owned here and is readonly for the remaining scope.
        // Its final caller retention consequently has zero outstanding deltas.
        (self.0)(&terminals, progress)
    }
}

pub(in super::super) fn with_candidate_math<'id, 'cells, R>(
    original: &P1ObservedColumnOriginalContext<'_, 'id>,
    flow: &PreparedBookV2TextFlow<'_>,
    progress: &mut ColumnProgressView<'_, 'id, 'cells>,
    inspect: impl for<'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut ColumnProgressView<'_, 'id, 'cells>,
    ) -> Result<R, E>,
) -> Result<R, E> {
    with_source_pages(original, flow, progress, PhysicalMath(inspect))
}
