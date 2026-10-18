//! Column resource selection retains its own display throughout both chains.
//! This resource-only join precedes encoded text and does not grant PDF authority.
use super::*;
use typaxis_display_list::book_v2::BookV2ColumnDisplay;
use typaxis_resources::book_v2::{
    BookV2ColumnCidPlans, BookV2ColumnFontSelection, BookV2ColumnFontSelectionBuilder,
    BookV2ColumnFontSelectionContinuation, BookV2ColumnImageSelection, BookV2ColumnRasterPrograms,
};

/// Borrowed resource roots from one authentic column display. A PDF pipeline
/// must insert its encoded font/text branch before allocating image resources.
pub struct BookV2ColumnResourceBundle<'r, 'i, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    cids: &'r BookV2ColumnCidPlans<'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    rasters: &'r BookV2ColumnRasterPrograms<'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    caller_before: [u64; 3],
    constructors: [[u64; 3]; 2],
}
impl<'r, 'i, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2ColumnResourceBundle<'r, 'i, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn display(&self) -> &'v BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.fonts().display()
    }
    pub fn fonts(&self) -> &'c BookV2ColumnFontSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.cids.source().source().selection()
    }
    pub fn images(&self) -> &'i BookV2ColumnImageSelection<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.rasters.source()
    }
    pub fn cids(&self) -> &'r BookV2ColumnCidPlans<'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.cids
    }
    pub fn rasters(
        &self,
    ) -> &'r BookV2ColumnRasterPrograms<'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.rasters
    }
    /// Actual record/spool/work prefix before any source or resource operation.
    pub fn caller_before(&self) -> [u64; 3] {
        self.caller_before
    }
    /// Accepted record/spool/work prefixes after the font and image constructors.
    pub fn constructor_charges(&self) -> [[u64; 3]; 2] {
        self.constructors
    }
}

/// Select, close and write every selected font, plan its CIDs, then select and
/// write ordinary raster resources. The callback receives a scoped continuation of the final
/// builder so additional operations start after all preceding charges.
pub fn with_budgeted_book_v2_column_resources<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'loan, 'r, 'i, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnResourceBundle<'r, 'i, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnFontSelectionContinuation<'loan, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    let maximum = budget.inner.maximum_work;
    let mut records = budget.inner.observation.records;
    let mut spool = budget.inner.observation.spool;
    let mut work = budget.inner.observation.work;
    let caller_before = [records, spool, work];
    let outcome = with_budgeted_book_v2_column_display(
        input,
        limits,
        japanese_mode,
        budget,
        |display, _display_builder, observed| {
            macro_rules! retain {
                ($builder:ident) => {{
                    records = records.max($builder.record_charge());
                    spool = spool.max($builder.spool_charge());
                    work = work.max($builder.work_steps());
                }};
            }
            macro_rules! resource {
                ($builder:ident, $operation:expr, $name:literal) => {{
                    let value = $operation;
                    retain!($builder);
                    value.map_err(|e| stage($name, e))?
                }};
            }
            let constructor = BookV2ColumnFontSelectionBuilder::new_counted(
                display,
                limits,
                maximum,
                observed.records,
                observed.spool,
                observed.work,
                &mut records,
                &mut spool,
                &mut work,
            );
            let mut fonts = constructor.map_err(|e| stage("column font constructor", e))?;
            let font_constructor = [
                fonts.record_charge(),
                fonts.spool_charge(),
                fonts.work_steps(),
            ];
            let selection = resource!(fonts, fonts.build(), "column font selection");
            let closures = resource!(
                fonts,
                fonts.prepare_font_closures(&selection),
                "column font closure"
            );
            let programs = resource!(
                fonts,
                fonts.write_font_programs(&closures),
                "column font programs"
            );
            let cids = resource!(fonts, fonts.plan_cids(&programs), "column CIDs");
            let constructor = fonts.next_stage_counted(&mut records, &mut spool, &mut work);
            let mut images = constructor.map_err(|e| stage("column image constructor", e))?;
            let image_constructor = [
                images.record_charge(),
                images.spool_charge(),
                images.work_steps(),
            ];
            let image_selection =
                resource!(images, images.select_images(), "column image selection");
            let rasters = resource!(
                images,
                images.write_raster_programs(&image_selection),
                "column raster programs"
            );
            let roots = BookV2ColumnResourceBundle {
                cids: &cids,
                rasters: &rasters,
                caller_before,
                constructors: [font_constructor, image_constructor],
            };
            let observed = BookV2PdfConvergenceObservation {
                records,
                spool,
                work,
                ..observed
            };
            let result = images.with_continuation(|loan| inspect(&roots, loan, observed));
            retain!(images);
            Ok(result)
        },
    );
    budget.inner.observation.records = budget.inner.observation.records.max(records);
    budget.inner.observation.spool = budget.inner.observation.spool.max(spool);
    budget.inner.observation.work = budget.inner.observation.work.max(work);
    outcome?
}
