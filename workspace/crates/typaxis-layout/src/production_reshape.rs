//! Own the real shape/selection feedback lifetime. This stage does not issue
//! pagination convergence, paint authorization, or a public PDF receipt.
use super::*;
use typaxis_core::Rect;
use typaxis_linebreak::{
    BreakError, LineLayoutContext, LineLayoutStateFingerprint, LineReshapeFeedback,
    LineReshapeObservation, LineReshapePassRecord,
};
use typaxis_shaping::{
    reshape_production_authored_text, shape_production_authored_text,
    ProductionParagraphLineContext, ProductionTextShapeError,
};

#[derive(Debug)]
pub enum ProductionBodyReshapeError {
    Shape(ProductionTextShapeError),
    Layout(ProductionInlinePreparationError),
    Feedback(BreakError),
}
impl std::fmt::Display for ProductionBodyReshapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "production_body_reshape {self:?}")
    }
}
impl std::error::Error for ProductionBodyReshapeError {}
impl From<ProductionTextShapeError> for ProductionBodyReshapeError {
    fn from(e: ProductionTextShapeError) -> Self {
        Self::Shape(e)
    }
}
impl From<ProductionInlinePreparationError> for ProductionBodyReshapeError {
    fn from(e: ProductionInlinePreparationError) -> Self {
        Self::Layout(e)
    }
}
impl From<BreakError> for ProductionBodyReshapeError {
    fn from(e: BreakError) -> Self {
        Self::Feedback(e)
    }
}

/// Only the owner below constructs this view, and only after an actual stable
/// comparison. The callback cannot retain the borrowed shape/selection graph.
pub struct ProductionConvergedBodyLines<'s, 'p, 'a> {
    lines: &'s ProductionInlineLineLayout<'p, 'a>,
    footnotes: ProductionFootnoteLines<'s, 'p, 'a>,
    passes: &'s [LineReshapePassRecord],
    candidate_steps: u64,
}
impl<'s, 'p, 'a> ProductionConvergedBodyLines<'s, 'p, 'a> {
    pub const fn footnotes(&self) -> &ProductionFootnoteLines<'s, 'p, 'a> {
        &self.footnotes
    }
    pub const fn lines(&self) -> &'s ProductionInlineLineLayout<'p, 'a> {
        self.lines
    }
    pub const fn passes(&self) -> &'s [LineReshapePassRecord] {
        self.passes
    }
    pub const fn candidate_steps(&self) -> u64 {
        self.candidate_steps
    }
}

fn selected_state(
    lines: &ProductionInlineLineLayout<'_, '_>,
) -> Result<LineLayoutStateFingerprint, BreakError> {
    // Fixed-size canonical JSON; no caller-supplied hash or stability flag.
    const PREFIX: &[u8] = b"{\"selected_line_layout_fingerprint\":\"";
    let mut bytes = [0u8; PREFIX.len() + 64 + 2];
    bytes[..PREFIX.len()].copy_from_slice(PREFIX);
    for (index, byte) in lines.fingerprint().iter().enumerate() {
        bytes[PREFIX.len() + index * 2] = b"0123456789abcdef"[(byte >> 4) as usize];
        bytes[PREFIX.len() + index * 2 + 1] = b"0123456789abcdef"[(byte & 15) as usize];
    }
    bytes[PREFIX.len() + 64..].copy_from_slice(b"\"}");
    LineLayoutStateFingerprint::from_canonical_bytes(&bytes)
}

/// Initial paragraph shaping is followed by quota-owned, selected-line shaping
/// and rebreaking. The candidate budget spans the initial break and every pass.
/// Each stage retains its document fragment ceiling; full pipeline allocation
/// accounting and page/generated-reference convergence are separate owners.
#[allow(clippy::too_many_arguments)]
pub fn with_converged_production_body_lines<R>(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    flow: &ProductionTextFlow<'_>,
    bindings: &ValidatedPrecomposedVectorBindings,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    max_candidate_steps: u64,
    use_stable: impl FnOnce(ProductionConvergedBodyLines<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    let epoch = bindings.epoch().fingerprint();
    let mut remaining_steps = max_candidate_steps;
    let (mut contexts, initial_state) = {
        let shape =
            shape_production_authored_text(package, navigation, flow, admitted, limits, epoch)?;
        let prepared = prepare_production_inline_items(
            package,
            navigation,
            profile,
            limits,
            admitted,
            flow,
            &shape,
            bindings,
            japanese_mode,
        )?;
        let selected = layout_production_body_inline_lines(&prepared, body, remaining_steps)?;
        remaining_steps -= selected.candidate_steps();
        (
            production_selected_line_contexts(&selected)?,
            selected_state(&selected)?,
        )
    };
    let mut feedback = LineReshapeFeedback::new(initial_state);
    let mut context = LineLayoutContext::from_limits(limits.base());
    let mut budget = context.take_budget()?;
    loop {
        // The owned context constructor charged its corresponding borrowed view.
        let mut inputs = Vec::new();
        inputs
            .try_reserve_exact(contexts.paragraphs().len())
            .map_err(|_| BreakError::AllocationFailure)?;
        inputs.extend(
            contexts
                .paragraphs()
                .iter()
                .map(|p| ProductionParagraphLineContext {
                    owner: p.owner(),
                    ends: p.ends(),
                }),
        );
        let permit = feedback.begin_pass(&mut budget)?;
        let shape = reshape_production_authored_text(
            package, navigation, flow, admitted, limits, epoch, &inputs,
        )?;
        let prepared = prepare_production_inline_items(
            package,
            navigation,
            profile,
            limits,
            admitted,
            flow,
            &shape,
            bindings,
            japanese_mode,
        )?;
        let selected = layout_production_body_inline_lines(&prepared, body, remaining_steps)?;
        remaining_steps -= selected.candidate_steps();
        match permit.complete(selected_state(&selected)?)? {
            LineReshapeObservation::Stable => {
                return Ok(use_stable(ProductionConvergedBodyLines {
                    lines: &selected,
                    footnotes: prepare_production_footnote_lines(&selected, limits)?,
                    passes: feedback.records(),
                    candidate_steps: max_candidate_steps - remaining_steps,
                }))
            }
            LineReshapeObservation::RebreakRequired => {
                contexts = production_selected_line_contexts(&selected)?;
            }
        }
    }
}
