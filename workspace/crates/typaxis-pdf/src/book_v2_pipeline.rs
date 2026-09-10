//! One stateful resource/PDF pipeline over an actual selected Book-2 display.
//! A failed attempt retains every component's completed budget consumption.
use super::*;
use typaxis_resources::book_v2::*;

#[derive(Debug)]
pub enum BookV2PdfPipelineError {
    Selection(BookV2FontSelectionError),
    Closure(BookV2FontClosureError),
    Program(BookV2FontProgramsError),
    Cid(BookV2CidError),
    Raster(BookV2RasterError),
    Pdf(BookV2FontStreamError),
    Navigation(BookV2NavigationGeometryError),
    Assembly(BookV2PdfAssemblyError),
}
impl From<BookV2FontSelectionError> for BookV2PdfPipelineError {
    fn from(e: BookV2FontSelectionError) -> Self {
        Self::Selection(e)
    }
}
impl From<BookV2FontClosureError> for BookV2PdfPipelineError {
    fn from(e: BookV2FontClosureError) -> Self {
        Self::Closure(e)
    }
}
impl From<BookV2FontProgramsError> for BookV2PdfPipelineError {
    fn from(e: BookV2FontProgramsError) -> Self {
        Self::Program(e)
    }
}
impl From<BookV2CidError> for BookV2PdfPipelineError {
    fn from(e: BookV2CidError) -> Self {
        Self::Cid(e)
    }
}
impl From<BookV2RasterError> for BookV2PdfPipelineError {
    fn from(e: BookV2RasterError) -> Self {
        Self::Raster(e)
    }
}
impl From<BookV2FontStreamError> for BookV2PdfPipelineError {
    fn from(e: BookV2FontStreamError) -> Self {
        Self::Pdf(e)
    }
}
impl From<BookV2NavigationGeometryError> for BookV2PdfPipelineError {
    fn from(e: BookV2NavigationGeometryError) -> Self {
        Self::Navigation(e)
    }
}
impl From<BookV2PdfAssemblyError> for BookV2PdfPipelineError {
    fn from(e: BookV2PdfAssemblyError) -> Self {
        Self::Assembly(e)
    }
}
impl std::fmt::Display for BookV2PdfPipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 PDF pipeline: {self:?}")
    }
}
impl std::error::Error for BookV2PdfPipelineError {}
pub struct BookV2PdfPipeline<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    display: &'v BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    limits: &'v M4EffectiveResourceLimits,
    first_font_object: u32,
    budget: Budget,
}
impl<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2PdfPipeline<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        display: &'v BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        first_font_object: u32,
        limits: &'v M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, BookV2PdfPipelineError> {
        display
            .verify_resources(display.admitted(), limits)
            .map_err(|_| E::Identity)?;
        if first_font_object == 0 || first_font_object > limits.base().get().max_pdf_objects {
            return Err(E::Objects.into());
        }
        let mut budget = Budget {
            max_records: limits.base().get().max_fragments,
            max_spool: limits.base().get().max_spool_bytes,
            max_output: limits.base().get().max_output_bytes,
            max_work: maximum_work,
            records: prior_records.max(display.record_charge()),
            spool: prior_spool.max(display.source().spool_charge()),
            output: prior_output,
            work: prior_work.max(display.work_steps()),
        };
        budget.reserve(1, 0, 0)?;
        if budget.work > maximum_work {
            return Err(E::Work.into());
        }
        Ok(Self {
            display,
            limits,
            first_font_object,
            budget,
        })
    }
    pub fn record_charge(&self) -> u64 {
        self.budget.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.budget.spool
    }
    pub fn output_charge(&self) -> u64 {
        self.budget.output
    }
    pub fn work_steps(&self) -> u64 {
        self.budget.work
    }
    /// All resources, structure and final bytes remain alive through the callback.
    /// The callback is invoked only after the complete source-page graph succeeds.
    /// No component-error retry is performed internally or refunds prior charges.
    pub fn with_pdf<R>(
        &mut self,
        inspect: impl FnOnce(
            &BookV2PdfAssembly<
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
        ) -> R,
    ) -> Result<R, BookV2PdfPipelineError> {
        let display = self.display;
        let limits = self.limits;
        let work = self.budget.max_work;
        macro_rules! resource {
            ($builder:ident,$operation:expr) => {{
                let result = $operation;
                self.budget.records = $builder.record_charge();
                self.budget.spool = $builder.spool_charge();
                self.budget.work = $builder.work_steps();
                result?
            }};
        }
        macro_rules! pdf {
            ($constructor:expr) => {{
                let mut builder = $constructor?;
                let result = builder.build();
                self.budget.records = builder.record_charge();
                self.budget.spool = builder.spool_charge();
                self.budget.output = builder.output_charge();
                self.budget.work = builder.work_steps();
                result?
            }};
        }
        let mut fonts = BookV2FontSelectionBuilder::new(
            display,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.work,
        )?;
        let selection = resource!(fonts, fonts.build());
        let closures = resource!(fonts, fonts.prepare_font_closures(&selection));
        let programs = resource!(fonts, fonts.write_font_programs(&closures));
        let cids = resource!(fonts, fonts.plan_cids(&programs));
        let streams = pdf!(BookV2FontStreamBuilder::new(
            &cids,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        let font_objects = pdf!(BookV2FontObjectBuilder::new(
            &streams,
            self.first_font_object,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        let text = pdf!(BookV2TextCommandBuilder::new(
            &font_objects,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        // Start the image branch with the complete text branch's actual consumption.
        // ImageSelection retains this prefix; marked-content merging checks it.
        let mut images = BookV2FontSelectionBuilder::new(
            display,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.work,
        )?;
        let image_selection = resource!(images, images.select_images());
        let rasters = resource!(images, images.write_raster_programs(&image_selection));
        let vectors = pdf!(BookV2VectorProgramBuilder::new(
            &rasters,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        let image_objects = pdf!(BookV2ImageObjectBuilder::new(
            &vectors,
            font_objects.next_object(),
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        let commands = pdf!(BookV2ImageCommandBuilder::new(
            &image_objects,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        let scopes = pdf!(BookV2MarkedScopeBuilder::new(
            &commands,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        let marked = pdf!(BookV2MarkedContentBuilder::new(
            &scopes,
            &text,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        let registry = pdf!(BookV2SourceStructureBuilder::new(
            &marked,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        let relations = pdf!(BookV2StructureRelationBuilder::new(
            &registry,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        let navigation = pdf!(BookV2NavigationGeometryBuilder::new(
            &relations,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        let assembly = pdf!(BookV2PdfAssemblyBuilder::new(
            &navigation,
            limits,
            work,
            self.budget.records,
            self.budget.spool,
            self.budget.output,
            self.budget.work
        ));
        Ok(inspect(&assembly))
    }
}
