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
    ExercisePart,
    Choice,
    Hint,
    Assumption,
}
impl BookV2SemanticContainerStyleKind {
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

/// Successor-only extension. The table-1 validator and consumers remain frozen.
pub fn validate_book_v2_document_styles(sheet: &StyleSheet) -> Result<(), StyleValidationError> {
    sheet.validate_table_document_styles_version(true)
}
pub fn cascade_book_v2_document_style(
    sheet: &StyleSheet,
    block: &str,
    classes: &[String],
) -> Result<ComputedStyle, StyleValidationError> {
    validate_book_v2_document_styles(sheet)?;
    if block != "table" && BasicStyleBlockKind::from_str(block).is_none() {
        return Err(StyleValidationError::InvalidSelector(
            SelectorError::InvalidBlockType,
        ));
    }
    sheet.cascade_validated(block, classes)
}
pub fn cascade_book_v2_descendant_style(
    block: &str,
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
) -> Result<SemanticContainerInheritanceStyle, StyleValidationError> {
    let computed = cascade_book_v2_document_style(sheet, block, classes)?;
    close_semantic_inheritance_style(&computed, parent)
}

#[cfg(test)]
mod named_page_tests {
    use super::*;
    #[test]
    fn named_table_page_is_a_successor_only_style_extension() {
        let sheet = StyleSheet {
            rules: vec![StyleRule {
                style_id: StyleId::new("named-table").unwrap(),
                extends: None,
                selector: "table.appendix".to_owned(),
                source_order: 0,
                declarations: vec![Declaration {
                    name: "page".to_owned(),
                    important: false,
                    value: StyleValue::Text("appendix".to_owned()),
                }],
            }],
        };
        assert_eq!(
            sheet.validate_table_document_styles(),
            Err(StyleValidationError::InvalidPageProperty)
        );
        validate_book_v2_document_styles(&sheet).unwrap();
        let value =
            cascade_book_v2_document_style(&sheet, "table", &["appendix".to_owned()]).unwrap();
        assert_eq!(value.page_name().unwrap().unwrap().as_str(), "appendix");
        let mut invalid = sheet.clone();
        invalid.rules[0].declarations.push(Declaration {
            name: "width".to_owned(),
            important: false,
            value: StyleValue::Length(Length::from_raw(65536).unwrap()),
        });
        assert_eq!(
            validate_book_v2_document_styles(&invalid),
            Err(StyleValidationError::InapplicableProperty)
        );
    }
}
