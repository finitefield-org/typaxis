//! Inheritance-only styles for original Book-2 table-cell owners.
use super::*;

#[cfg(feature = "book-v2-staging")]
pub(super) fn isolate_cell_styles(
    parsed: &[StyleRule],
    direct: &[bool],
) -> Result<StyleSheet, StagingSemanticSyntaxError> {
    let sheet = isolate_staging_style_rules(parsed, direct)?;
    // Check all included extends ancestors, even for currently unselected cells.
    if sheet.rules.iter().flat_map(|r| &r.declarations).any(|d| {
        !matches!(
            d.name.as_str(),
            "text_align" | "font_family" | "font_size" | "line_height"
        )
    }) {
        return Err(StagingSemanticSyntaxError::InapplicableStyle);
    }
    Ok(sheet)
}

pub(super) fn inherit_cell_style(
    owner: NodeId,
    classes: &[String],
    _rules: &StagingSemanticStyleSheets,
    parent: &SemanticContainerInheritanceStyle,
) -> Result<SemanticContainerInheritanceStyle, StagingSemanticSyntaxError> {
    #[cfg(feature = "book-v2-staging")]
    if let Some(sheet) = &_rules.table_cells {
        if sheet.rules.is_empty() {
            return Ok(parent.clone());
        }
        return cascade_staging_semantic_descendant_style(
            "paragraph",
            classes,
            sheet,
            Some(parent),
        )
        .map_err(map_semantic_style_error);
    }
    if !classes.is_empty() {
        return Err(StagingSemanticSyntaxError::TableCellStyleStaging(owner));
    }
    Ok(parent.clone())
}
