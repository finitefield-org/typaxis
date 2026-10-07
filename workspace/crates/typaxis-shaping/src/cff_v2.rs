//! Admitted CFF /2 input to the existing linked backend. This font-level owner
//! does not authorize package style selection, layout epochs or PDF publication.
use super::*;
use typaxis_font::Cff1AdmissionV2;

#[derive(Clone, Copy, Debug)]
pub struct Cff1ShapeInputV2<'a> {
    pub run_id: GlyphRunId,
    pub font: FontInstanceId,
    pub source: ShapeSourceSpan,
    pub utf8: &'a str,
    pub font_size: PositiveLength,
    pub bidi_level: BidiLevel,
    pub script: OpenTypeTag,
    pub language: Option<&'a str>,
    pub pre_context: Option<&'a str>,
    pub post_context: Option<&'a str>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cff1ShapeErrorV2 {
    EmptyText,
    ContextLimit {
        limit: u32,
        observed: u64,
    },
    /// UTF-8 offsets relative to the exact run input, including its selector.
    MissingCoverage {
        byte_start: u32,
        byte_end: u32,
    },
    SplitVariationSequence,
    Backend(LinkedShaperError),
    OutputInvariant,
}
impl std::fmt::Display for Cff1ShapeErrorV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CFF /2 shaping {self:?}")
    }
}
impl std::error::Error for Cff1ShapeErrorV2 {}
#[derive(Debug)]
pub struct Cff1ShapedRunV2<'a> {
    font_size: PositiveLength,
    admission: &'a Cff1AdmissionV2,
    utf8: &'a str,
    run: GlyphRun,
}
impl Cff1ShapedRunV2<'_> {
    pub fn font_size(&self) -> PositiveLength {
        self.font_size
    }
    pub fn admission(&self) -> &Cff1AdmissionV2 {
        self.admission
    }
    pub fn utf8(&self) -> &str {
        self.utf8
    }
    pub fn glyph_run(&self) -> &GlyphRun {
        &self.run
    }
    /// Exact original text for one logical cluster, including variation
    /// selectors. Consumers must not infer Unicode from a selected GID.
    pub fn cluster_text(&self, index: usize) -> Option<&str> {
        let cluster = self.run.clusters.get(index)?;
        let origin = source_range(self.run.source_span).0;
        let (start, end) = source_range(cluster.source_span);
        self.utf8
            .get(start.checked_sub(origin)? as usize..end.checked_sub(origin)? as usize)
    }
}
fn selector(c: char) -> bool {
    matches!(c as u32,0xfe00..=0xfe0f | 0xe0100..=0xe01ef)
}
pub(super) fn coverage(admission: &Cff1AdmissionV2, text: &str) -> Result<(), Cff1ShapeErrorV2> {
    let mut chars = text.char_indices().peekable();
    while let Some((start, base)) = chars.next() {
        let vs = chars.peek().copied().filter(|(_, c)| selector(*c));
        let end = if let Some((at, c)) = vs {
            chars.next();
            at + c.len_utf8()
        } else {
            start + base.len_utf8()
        };
        let missing = selector(base)
            || if let Some((_, vs)) = vs {
                admission
                    .cmap()
                    .glyph_for_sequence(base, Some(vs))
                    .is_none()
            } else {
                !is_shaping_default_ignorable(base)
                    && admission.cmap().glyph_for_sequence(base, None).is_none()
            };
        if missing {
            return Err(Cff1ShapeErrorV2::MissingCoverage {
                byte_start: start as u32,
                byte_end: end as u32,
            });
        }
    }
    Ok(())
}

pub fn shape_cff1_run_v2<'a>(
    admission: &'a Cff1AdmissionV2,
    input: Cff1ShapeInputV2<'a>,
) -> Result<Cff1ShapedRunV2<'a>, Cff1ShapeErrorV2> {
    if input.utf8.is_empty() {
        return Err(Cff1ShapeErrorV2::EmptyText);
    }
    let limit = admission
        .effective_limits()
        .base()
        .get()
        .max_shaping_context_bytes;
    let observed = [Some(input.utf8), input.pre_context, input.post_context]
        .into_iter()
        .flatten()
        .try_fold(0u64, |sum, t| sum.checked_add(t.len() as u64))
        .unwrap_or(u64::MAX);
    if observed > u64::from(limit) {
        return Err(Cff1ShapeErrorV2::ContextLimit { limit, observed });
    }
    // A selector in following context belongs in this run's input cluster;
    // HarfRust must receive both scalars as input, not just as context.
    if input
        .post_context
        .and_then(|c| c.chars().next())
        .is_some_and(selector)
    {
        return Err(Cff1ShapeErrorV2::SplitVariationSequence);
    }
    coverage(admission, input.utf8)?;
    let records = linked_backend_record_bound(input.utf8).map_err(Cff1ShapeErrorV2::Backend)?;
    let mut budget = ShapeOutputBudget::new(limit);
    let run = shape_linked(
        LinkedBackendInput {
            run_id: input.run_id,
            font: input.font,
            source: input.source,
            utf8: input.utf8,
            font_bytes: admission.source(),
            face_index: 0,
            admitted_units_per_em: admission.units_per_em(),
            admitted_glyph_count: u32::from(admission.glyph_count()),
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
    .map_err(Cff1ShapeErrorV2::Backend)?;
    if !budget.matches_output(&run) {
        return Err(Cff1ShapeErrorV2::OutputInvariant);
    }
    Ok(Cff1ShapedRunV2 {
        font_size: input.font_size,
        admission,
        utf8: input.utf8,
        run,
    })
}

#[cfg(test)]
#[path = "cff_v2_tests.rs"]
mod tests;
