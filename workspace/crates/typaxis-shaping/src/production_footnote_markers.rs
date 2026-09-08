//! Admitted-font glyphs for definition-order footnote labels; no page position.
use super::list_markers::{shape_generated_marker, GeneratedMarkerGlyphs, GeneratedMarkerInput};
use super::*;

pub struct ProductionFootnoteMarkerShape<'a> {
    source: &'a typaxis_syntax::ProductionFootnoteDefinition<'a>,
    definition_index: u32,
    glyphs: GeneratedMarkerGlyphs<'a>,
}
impl<'a> ProductionFootnoteMarkerShape<'a> {
    pub const fn source(&self) -> &'a typaxis_syntax::ProductionFootnoteDefinition<'a> {
        self.source
    }
    pub const fn definition_index(&self) -> u32 {
        self.definition_index
    }
    pub const fn utf8(&self) -> &'a str {
        self.glyphs.utf8
    }
    pub const fn provenance(&self) -> GeneratedProvenance {
        self.glyphs.provenance
    }
    pub const fn font(&self) -> &ProductionBodyFont {
        &self.glyphs.font
    }
    pub const fn glyph_run(&self) -> &GlyphRun {
        &self.glyphs.run
    }
    pub const fn advance(&self) -> PositiveLength {
        self.glyphs.advance
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.glyphs.fingerprint
    }
}

pub(super) fn shape_markers<'a>(
    flow: BodyFlow<'a>,
    admitted: BodyFonts<'_>,
    limits: &M4EffectiveResourceLimits,
    output_records: &mut u64,
) -> Result<Vec<ProductionFootnoteMarkerShape<'a>>, ProductionTextShapeError> {
    use ProductionTextShapeErrorKind as E;
    let mut result = Vec::new();
    for (index, source) in flow_call!(flow, footnote_definitions()).iter().enumerate() {
        let owner = source.owner();
        let style = flow_call!(flow, footnote_marker_style(index))
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let glyphs = shape_generated_marker(
            GeneratedMarkerInput {
                owner,
                index,
                text: flow_call!(flow, footnote_marker_text(owner))
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                provenance: flow_call!(flow, footnote_marker_provenance(owner))
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                families: style
                    .font_families()
                    .ok_or_else(|| error(owner, E::MissingTextStyle))?,
                size: style
                    .font_size()
                    .ok_or_else(|| error(owner, E::MissingTextStyle))?,
                language: source.language(),
            },
            admitted,
            limits,
            output_records,
        )?;
        result
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        result.push(ProductionFootnoteMarkerShape {
            source,
            definition_index: u32::try_from(index).map_err(|_| error(owner, E::OutputLimit))?,
            glyphs,
        });
    }
    Ok(result)
}
