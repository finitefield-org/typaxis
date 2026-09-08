//! Closed successor container style kinds. The cascade shares the established
//! precedence/inheritance engine and retains the selected kind in its result.

use super::*;

/// Closed book-2 vocabulary. Each kind is retained through nested block slots;
/// no successor kind is coerced into a result, proof, or exercise.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2SemanticContainerStyleKind {
    Result,
    Proof,
    Exercise,
    Solution,
    Example,
    Counterexample,
    Remark,
    Note,
    Warning,
    CommonError,
    FormalizationNote,
    Quote,
}
impl BookV2SemanticContainerStyleKind {
    pub const ALL: [Self; 12] = [
        Self::Result,
        Self::Proof,
        Self::Exercise,
        Self::Solution,
        Self::Example,
        Self::Counterexample,
        Self::Remark,
        Self::Note,
        Self::Warning,
        Self::CommonError,
        Self::FormalizationNote,
        Self::Quote,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Result => "result",
            Self::Proof => "proof",
            Self::Exercise => "exercise",
            Self::Solution => "solution",
            Self::Example => "example",
            Self::Counterexample => "counterexample",
            Self::Remark => "remark",
            Self::Note => "note",
            Self::Warning => "warning",
            Self::CommonError => "common_error",
            Self::FormalizationNote => "formalization_note",
            Self::Quote => "quote",
        }
    }
}

/// Successor style ownership is distinct from the closed legacy style kind.
///
/// ```compile_fail
/// use typaxis_style::{book_v2::BookV2SemanticContainerComputedStyle, SemanticContainerComputedStyle};
/// fn legacy(style: BookV2SemanticContainerComputedStyle) -> SemanticContainerComputedStyle {
///     style
/// }
/// ```
pub type BookV2SemanticContainerComputedStyle =
    ComputedSemanticContainerStyle<BookV2SemanticContainerStyleKind>;

pub fn cascade_book_v2_semantic_container_style(
    kind: BookV2SemanticContainerStyleKind,
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
) -> Result<BookV2SemanticContainerComputedStyle, StyleValidationError> {
    cascade_semantic_container_style_kind(kind, classes, sheet, parent)
}
