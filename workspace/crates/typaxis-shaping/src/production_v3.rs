//! Actual linked shaping from a ledger-issued /3 instance. Paragraph style,
//! layout convergence and publication remain the responsibilities of callers.
use super::*;
use typaxis_resource_admission::{AdmittedProductionFontInstanceV3, AdmittedProductionFontV3};

#[derive(Clone, Copy, Debug)]
pub struct ProductionShapeInputV3<'a> {
    pub run_id: GlyphRunId,
    pub source: ShapeSourceSpan,
    pub utf8: &'a str,
    pub font_size: PositiveLength,
    pub bidi_level: BidiLevel,
    pub script: OpenTypeTag,
    pub language: Option<&'a str>,
    pub pre_context: Option<&'a str>,
    pub post_context: Option<&'a str>,
}
#[derive(Debug)]
pub enum ProductionShapeErrorV3 {
    EmptyText,
    ContextLimit,
    TrueTypeCoverage(StagingEquationNumberShapeError),
    Backend(LinkedShaperError),
    Cff(Cff1ShapeErrorV2),
    OutputInvariant,
}
impl std::fmt::Display for ProductionShapeErrorV3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "production /3 shaping {self:?}")
    }
}
impl std::error::Error for ProductionShapeErrorV3 {}
#[derive(Debug)]
enum ProductionRunV3<'a> {
    TrueType(GlyphRun),
    Cff1V2(Cff1ShapedRunV2<'a>),
}
#[derive(Debug)]
pub struct ShapedProductionRunV3<'a> {
    instance: AdmittedProductionFontInstanceV3<'a>,
    input: ProductionShapeInputV3<'a>,
    run: ProductionRunV3<'a>,
}
impl<'a> ShapedProductionRunV3<'a> {
    pub fn input(&self) -> ProductionShapeInputV3<'a> {
        self.input
    }
    pub fn instance(&self) -> AdmittedProductionFontInstanceV3<'a> {
        self.instance
    }
    pub fn utf8(&self) -> &'a str {
        self.input.utf8
    }
    pub fn font_size(&self) -> PositiveLength {
        self.input.font_size
    }
    pub fn glyph_run(&self) -> &GlyphRun {
        match &self.run {
            ProductionRunV3::TrueType(run) => run,
            ProductionRunV3::Cff1V2(run) => run.glyph_run(),
        }
    }
    pub fn cff1_v2(&self) -> Option<&Cff1ShapedRunV2<'a>> {
        match &self.run {
            ProductionRunV3::TrueType(_) => None,
            ProductionRunV3::Cff1V2(run) => Some(run),
        }
    }
    pub fn cluster_text(&self, index: usize) -> Option<&'a str> {
        let run = self.glyph_run();
        let cluster = run.clusters.get(index)?;
        let origin = source_range(run.source_span).0;
        let (start, end) = source_range(cluster.source_span);
        self.input
            .utf8
            .get(start.checked_sub(origin)? as usize..end.checked_sub(origin)? as usize)
    }
}
pub fn shape_production_run_v3<'a>(
    instance: AdmittedProductionFontInstanceV3<'a>,
    input: ProductionShapeInputV3<'a>,
) -> Result<ShapedProductionRunV3<'a>, ProductionShapeErrorV3> {
    use ProductionShapeErrorV3 as E;
    if input.utf8.is_empty() {
        return Err(E::EmptyText);
    }
    let limit = instance
        .ledger()
        .effective_limits()
        .base()
        .get()
        .max_shaping_context_bytes;
    let bytes = [Some(input.utf8), input.pre_context, input.post_context]
        .into_iter()
        .flatten()
        .try_fold(0u64, |n, s| n.checked_add(s.len() as u64))
        .ok_or(E::ContextLimit)?;
    if bytes > u64::from(limit) {
        return Err(E::ContextLimit);
    }
    let run = match instance.font() {
        AdmittedProductionFontV3::Cff1V2(font) => ProductionRunV3::Cff1V2(
            shape_cff1_run_v2(
                font.admission(),
                Cff1ShapeInputV2 {
                    run_id: input.run_id,
                    font: instance.font_instance_id(),
                    source: input.source,
                    utf8: input.utf8,
                    font_size: input.font_size,
                    bidi_level: input.bidi_level,
                    script: input.script,
                    language: input.language,
                    pre_context: input.pre_context,
                    post_context: input.post_context,
                },
            )
            .map_err(E::Cff)?,
        ),
        AdmittedProductionFontV3::TrueType(font) => {
            validate_admitted_font_coverage(font, input.utf8).map_err(E::TrueTypeCoverage)?;
            let records = linked_backend_record_bound(input.utf8).map_err(E::Backend)?;
            let mut budget = ShapeOutputBudget::new(limit);
            let run = shape_linked(
                LinkedBackendInput {
                    run_id: input.run_id,
                    font: instance.font_instance_id(),
                    source: input.source,
                    utf8: input.utf8,
                    font_bytes: font.bytes(),
                    face_index: font.face_index(),
                    admitted_units_per_em: font.metadata().units_per_em,
                    admitted_glyph_count: font.metadata().glyph_count,
                    font_size: input.font_size.get(),
                    bidi_level: input.bidi_level,
                    script: input.script,
                    language: input.language,
                    pre_context: input.pre_context,
                    post_context: input.post_context,
                    max_output_records: records,
                },
                &mut budget,
            )
            .map_err(E::Backend)?;
            if !budget.matches_output(&run) {
                return Err(E::OutputInvariant);
            }
            ProductionRunV3::TrueType(run)
        }
    };
    Ok(ShapedProductionRunV3 {
        instance,
        input,
        run,
    })
}
