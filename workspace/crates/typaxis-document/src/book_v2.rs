//! Internal successor body domain; this module grants no profile or layout admission.

use crate::{
    SemanticBlock, SemanticDocument, SemanticFootnoteDefinition, SemanticListItem,
    SemanticTableCell, SemanticTableRow,
};

/// Closed book-2 vocabulary. Each kind is retained through nested block slots;
/// no successor kind is coerced into a result, proof, or exercise.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2SemanticContainerKind {
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
    ExercisePart,
    Choice,
    Hint,
    Assumption,
}
impl BookV2SemanticContainerKind {
    pub const ALL: [Self; 16] = [
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
        Self::ExercisePart,
        Self::Choice,
        Self::Hint,
        Self::Assumption,
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
            Self::ExercisePart => "exercise_part",
            Self::Choice => "choice",
            Self::Hint => "hint",
            Self::Assumption => "assumption",
        }
    }
}

pub type BookV2Block = SemanticBlock<BookV2SemanticContainerKind>;
pub type BookV2ListItem = SemanticListItem<BookV2SemanticContainerKind>;
pub type BookV2TableCell = SemanticTableCell<BookV2SemanticContainerKind>;
pub type BookV2TableRow = SemanticTableRow<BookV2SemanticContainerKind>;
pub type BookV2FootnoteDefinition = SemanticFootnoteDefinition<BookV2SemanticContainerKind>;
pub type BookV2Document = SemanticDocument<BookV2SemanticContainerKind>;

pub use crate::SemanticDescriptionTerm as BookV2DescriptionTerm;
pub type BookV2DescriptionItem = crate::SemanticDescriptionItem<BookV2SemanticContainerKind>;

#[path = "book_v2_language.rs"]
mod language;
pub use language::BookV2LanguageNodeKind;
