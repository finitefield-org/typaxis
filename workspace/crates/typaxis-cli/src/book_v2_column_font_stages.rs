//! Shared private font capability and unchanged Column paint stage kernels.
use super::*;
use typaxis_display_list::book_v2::BookV2ResourceDisplay;
use typaxis_resources::book_v2::{
    BookV2CidError, BookV2CidPlansData, BookV2CommandFontBuilder, BookV2FontClosureError,
    BookV2FontClosuresData, BookV2FontProgramsData, BookV2FontProgramsError,
    BookV2FontSelectionBuilderData, BookV2FontSelectionData, BookV2FontSelectionError,
    BookV2ImageSelectionData, BookV2RasterError, BookV2RasterProgramsData,
};

// This capability is private to this stage. It exposes only existing typed
// operations; neither adapter exposes its mutable underlying builder.
pub(super) trait ColumnFontOps<'v, 'a, D: BookV2ResourceDisplay<'a> + 'v> {
    fn record_charge(&self) -> u64;
    fn spool_charge(&self) -> u64;
    fn work_steps(&self) -> u64;
    fn reservation_sizes(&self) -> [usize; 2];
    fn build(&mut self) -> Result<BookV2FontSelectionData<'v, 'a, D>, BookV2FontSelectionError>;
    fn prepare_font_closures<'c>(
        &mut self,
        selection: &'c BookV2FontSelectionData<'v, 'a, D>,
    ) -> Result<BookV2FontClosuresData<'c, 'v, 'a, D>, BookV2FontClosureError>;
    fn write_font_programs<'x, 'c>(
        &mut self,
        closures: &'x BookV2FontClosuresData<'c, 'v, 'a, D>,
    ) -> Result<BookV2FontProgramsData<'x, 'c, 'v, 'a, D>, BookV2FontProgramsError>;
    fn plan_cids<'y, 'x, 'c>(
        &mut self,
        programs: &'y BookV2FontProgramsData<'x, 'c, 'v, 'a, D>,
    ) -> Result<BookV2CidPlansData<'y, 'x, 'c, 'v, 'a, D>, BookV2CidError>;
    fn next_stage_after_counted(
        &mut self,
        records: u64,
        spool: u64,
        work: u64,
        observed_records: &mut u64,
        observed_spool: &mut u64,
        observed_work: &mut u64,
    ) -> Result<(), BookV2FontSelectionError>;
    fn select_images(
        &mut self,
    ) -> Result<BookV2ImageSelectionData<'v, 'a, D>, BookV2FontSelectionError>;
    fn write_raster_programs<'i>(
        &mut self,
        selection: &'i BookV2ImageSelectionData<'v, 'a, D>,
    ) -> Result<BookV2RasterProgramsData<'i, 'v, 'a, D>, BookV2RasterError>;
}

macro_rules! font_ops {
    ($builder:ident, $handoff:ident) => {
        impl<'v, 'a, D: BookV2ResourceDisplay<'a> + 'v> ColumnFontOps<'v, 'a, D>
            for $builder<'v, 'a, D>
        {
            fn record_charge(&self) -> u64 {
                $builder::record_charge(self)
            }
            fn spool_charge(&self) -> u64 {
                $builder::spool_charge(self)
            }
            fn work_steps(&self) -> u64 {
                $builder::work_steps(self)
            }
            fn reservation_sizes(&self) -> [usize; 2] {
                $builder::reservation_sizes(self)
            }
            fn build(
                &mut self,
            ) -> Result<BookV2FontSelectionData<'v, 'a, D>, BookV2FontSelectionError> {
                $builder::build(self)
            }
            fn prepare_font_closures<'c>(
                &mut self,
                selection: &'c BookV2FontSelectionData<'v, 'a, D>,
            ) -> Result<BookV2FontClosuresData<'c, 'v, 'a, D>, BookV2FontClosureError> {
                $builder::prepare_font_closures(self, selection)
            }
            fn write_font_programs<'x, 'c>(
                &mut self,
                closures: &'x BookV2FontClosuresData<'c, 'v, 'a, D>,
            ) -> Result<BookV2FontProgramsData<'x, 'c, 'v, 'a, D>, BookV2FontProgramsError> {
                $builder::write_font_programs(self, closures)
            }
            fn plan_cids<'y, 'x, 'c>(
                &mut self,
                programs: &'y BookV2FontProgramsData<'x, 'c, 'v, 'a, D>,
            ) -> Result<BookV2CidPlansData<'y, 'x, 'c, 'v, 'a, D>, BookV2CidError> {
                $builder::plan_cids(self, programs)
            }
            fn next_stage_after_counted(
                &mut self,
                records: u64,
                spool: u64,
                work: u64,
                observed_records: &mut u64,
                observed_spool: &mut u64,
                observed_work: &mut u64,
            ) -> Result<(), BookV2FontSelectionError> {
                $builder::$handoff(
                    self,
                    records,
                    spool,
                    work,
                    observed_records,
                    observed_spool,
                    observed_work,
                )
            }
            fn select_images(
                &mut self,
            ) -> Result<BookV2ImageSelectionData<'v, 'a, D>, BookV2FontSelectionError> {
                $builder::select_images(self)
            }
            fn write_raster_programs<'i>(
                &mut self,
                selection: &'i BookV2ImageSelectionData<'v, 'a, D>,
            ) -> Result<BookV2RasterProgramsData<'i, 'v, 'a, D>, BookV2RasterError> {
                $builder::write_raster_programs(self, selection)
            }
        }
    };
}
font_ops!(
    BookV2FontSelectionBuilderData,
    next_stage_after_in_place_counted
);
font_ops!(BookV2CommandFontBuilder, next_stage_after_counted);

#[allow(clippy::too_many_arguments)]
pub(super) fn with_font_paints<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a, F, R>(
    fonts: &mut F,
    limits: &M4EffectiveResourceLimits,
    maximum: u64,
    first_font_object: u32,
    caller_before: [u64; 4],
    progress: &mut BookV2PdfConvergenceObservation,
    phase: &mut BookV2ColumnOutputPhase,
    inspect: impl for<'h, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c> FnOnce(
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
) -> Result<R, E>
where
    'a: 'p,
    'p: 's,
    's: 'f,
    'f: 'b,
    'b: 'q,
    'q: 'g,
    'g: 'd,
    'd: 'v,
    F: ColumnFontOps<'v, 'a, BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>>,
{
    let mut records = progress.records.max(fonts.record_charge());
    let mut spool = progress.spool.max(fonts.spool_charge());
    let mut output = progress.output;
    let mut work = progress.work.max(fonts.work_steps());
    let observed = *progress;
    let result = (|| {
        macro_rules! retain_resource {
            ($builder:ident) => {{
                records = records.max($builder.record_charge());
                spool = spool.max($builder.spool_charge());
                work = work.max($builder.work_steps());
            }};
        }
        macro_rules! resource {
            ($builder:ident, $operation:expr, $name:literal) => {{
                let value = $operation;
                retain_resource!($builder);
                value.map_err(|e| stage($name, e))?
            }};
        }
        macro_rules! retain_pdf {
            ($builder:ident) => {{
                retain_resource!($builder);
                output = output.max($builder.output_charge());
            }};
        }
        macro_rules! pdf {
            ($builder:ident, $name:literal) => {{
                let value = $builder.build();
                retain_pdf!($builder);
                value.map_err(|e| stage($name, e))?
            }};
        }
        let mut constructors = [[0; 4]; 8];
        constructors[0] = [records, spool, output, work];
        let reservation_sizes = fonts.reservation_sizes();
        let selection = resource!(fonts, fonts.build(), "column paint font selection");
        let display = selection.display();
        let closures = resource!(
            fonts,
            fonts.prepare_font_closures(&selection),
            "column paint font closure"
        );
        let programs = resource!(
            fonts,
            fonts.write_font_programs(&closures),
            "column paint font programs"
        );
        let cids = resource!(fonts, fonts.plan_cids(&programs), "column paint CIDs");
        let mut stream_builder = BookV2ColumnFontStreamBuilder::new_counted(
            &cids,
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
        .map_err(|e| stage("column font stream constructor", e))?;
        constructors[1] = [records, spool, output, work];
        let streams = pdf!(stream_builder, "column font streams");
        let mut object_builder = BookV2ColumnFontObjectBuilder::new_counted(
            &streams,
            first_font_object,
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
        .map_err(|e| stage("column font object constructor", e))?;
        constructors[2] = [records, spool, output, work];
        let font_objects = pdf!(object_builder, "column font objects");
        let mut text_builder = BookV2ColumnTextCommandBuilder::new_counted(
            &font_objects,
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
        .map_err(|e| stage("column text constructor", e))?;
        constructors[3] = [records, spool, output, work];
        let text = pdf!(text_builder, "column text commands");
        if !std::ptr::eq(text.display(), display) {
            return Err(E::Identity);
        }
        fonts
            .next_stage_after_counted(records, spool, work, &mut records, &mut spool, &mut work)
            .map_err(|e| stage("column paint image constructor", e))?;
        let images = fonts;
        constructors[4] = [records, spool, output, work];
        let image_selection = resource!(
            images,
            images.select_images(),
            "column paint image selection"
        );
        let rasters = resource!(
            images,
            images.write_raster_programs(&image_selection),
            "column paint rasters"
        );
        let mut vector_builder = BookV2ColumnVectorProgramBuilder::new_counted(
            &rasters,
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
        .map_err(|e| stage("column vector constructor", e))?;
        constructors[5] = [records, spool, output, work];
        let vectors = pdf!(vector_builder, "column vector programs");
        let mut image_object_builder = BookV2ColumnImageObjectBuilder::new_counted(
            &vectors,
            font_objects.next_object(),
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
        .map_err(|e| stage("column image object constructor", e))?;
        constructors[6] = [records, spool, output, work];
        let image_objects = pdf!(image_object_builder, "column image objects");
        let mut command_builder = BookV2ColumnImageCommandBuilder::new_counted(
            &image_objects,
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
        .map_err(|e| stage("column image command constructor", e))?;
        constructors[7] = [records, spool, output, work];
        let commands = pdf!(command_builder, "column image commands");
        let image_prior = image_selection.prior_charges();
        if !std::ptr::eq(image_selection.display(), text.display())
            || image_prior[0] < text.record_charge()
            || image_prior[1] < text.spool_charge()
            || image_prior[2] < text.work_steps()
            || vectors.prior_output_charge() < text.output_charge()
        {
            return Err(E::Identity);
        }
        let roots = BookV2ColumnPaintBundle {
            text: &text,
            images: &commands,
            caller_before,
            constructors,
            reservation_sizes,
        };
        let observed = BookV2PdfConvergenceObservation {
            records,
            spool,
            output,
            work,
            ..observed
        };
        phase.bind_paints(&roots)?;
        *progress = observed;
        let result = inspect(&roots, &mut command_builder, observed, phase, progress);
        retain_pdf!(command_builder);
        Ok(result)
    })();
    progress.records = progress.records.max(records);
    progress.spool = progress.spool.max(spool);
    progress.output = phase
        .accepted
        .unwrap_or_else(|| progress.output.max(output));
    progress.work = progress.work.max(work);
    result
}
