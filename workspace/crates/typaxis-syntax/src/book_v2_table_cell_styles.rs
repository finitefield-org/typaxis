//! Inheritance-only styles for original Book-2 table-cell owners.
use super::*;

pub(super) fn inherit_cell_style(
    owner: NodeId,
    classes: &[String],
    rules: &StagingSemanticStyleSheets,
    parent: &SemanticContainerInheritanceStyle,
) -> Result<SemanticContainerInheritanceStyle, StagingSemanticSyntaxError> {
    initial_style::Context::legacy()
        .cell(owner, classes, rules, parent)
        .map_err(initial_style::legacy_error)
}
