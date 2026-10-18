//! Full typed equality of every field compared by legacy source verification.
//! Fixed source values charge one comparison; UTF8 pays its full possible byte
//! comparison before use. No source fingerprint or pointer shortcut is used.
use super::*;

type Result<T> = std::result::Result<T, ProductionFlowError>;
fn fixed<T: PartialEq>(s: &mut ObservedSourceState, owner: NodeId, a: T, b: T) -> Result<bool> {
    s.charge(owner, 0, 0, 1)?;
    Ok(a == b)
}
fn text(s: &mut ObservedSourceState, owner: NodeId, a: &str, b: &str) -> Result<bool> {
    let work = (a.len() as u64)
        .checked_add(b.len() as u64)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| s.work_error(owner))?;
    s.charge(owner, 0, 0, work)?;
    Ok(a == b)
}
fn page(
    s: &mut ObservedSourceState,
    o: NodeId,
    a: Option<&typaxis_core::PageName>,
    b: Option<&typaxis_core::PageName>,
) -> Result<bool> {
    s.charge(o, 0, 0, 1)?;
    match (a, b) {
        (None, None) => Ok(true),
        (Some(a), Some(b)) => text(s, o, a.as_str(), b.as_str()),
        _ => Ok(false),
    }
}
fn style(
    s: &mut ObservedSourceState,
    o: NodeId,
    a: &SemanticContainerInheritanceStyle,
    b: &SemanticContainerInheritanceStyle,
) -> Result<bool> {
    typaxis_style::book_v2::book_v2_inheritance_styles_equal_counted(a, b, &mut |r, v, w| {
        s.charge(o, r, v, w)
    })
    .map_err(|e| match e {
        typaxis_style::book_v2::BookV2StylePreparationError::Budget(e) => e,
        typaxis_style::book_v2::BookV2StylePreparationError::Allocation => s.allocation_error(o),
        typaxis_style::book_v2::BookV2StylePreparationError::Style(_) => {
            failure(ProductionFlowErrorKind::ReceiptMismatch, o)
        }
    })
}
fn slice<T>(
    s: &mut ObservedSourceState,
    o: NodeId,
    a: &[T],
    b: &[T],
    mut each: impl FnMut(&mut ObservedSourceState, &T, &T) -> Result<bool>,
) -> Result<bool> {
    if !fixed(s, o, a.len(), b.len())? {
        return Ok(false);
    }
    for (a, b) in a.iter().zip(b) {
        if !each(s, a, b)? {
            return Ok(false);
        }
    }
    Ok(true)
}
fn fixed_slice<T: PartialEq>(
    s: &mut ObservedSourceState,
    o: NodeId,
    a: &[T],
    b: &[T],
) -> Result<bool> {
    slice(s, o, a, b, |s, a, b| fixed(s, o, a, b))
}
fn content(
    s: &mut ObservedSourceState,
    o: NodeId,
    a: ProductionInlineContent<'_>,
    b: ProductionInlineContent<'_>,
) -> Result<bool> {
    if !fixed(s, o, std::mem::discriminant(&a), std::mem::discriminant(&b))? {
        return Ok(false);
    }
    if let (
        ProductionInlineContent::Text { span: a, utf8: x },
        ProductionInlineContent::Text { span: b, utf8: y },
    ) = (a, b)
    {
        return Ok(fixed(s, o, a, b)? && text(s, o, x, y)?);
    }
    Ok(true)
}
fn reference(
    s: &mut ObservedSourceState,
    o: NodeId,
    a: Option<ProductionInlineReference<'_>>,
    b: Option<ProductionInlineReference<'_>>,
) -> Result<bool> {
    s.charge(o, 0, 0, 1)?;
    match (a, b) {
        (None, None) => Ok(true),
        (
            Some(ProductionInlineReference::Anchor {
                target: a,
                target_owner: x,
                format: f,
            }),
            Some(ProductionInlineReference::Anchor {
                target: b,
                target_owner: y,
                format: g,
            }),
        ) => Ok(text(s, o, a, b)? && fixed(s, o, x, y)? && fixed(s, o, f, g)?),
        (
            Some(ProductionInlineReference::Footnote { footnote_id: a }),
            Some(ProductionInlineReference::Footnote { footnote_id: b }),
        ) => text(s, o, a, b),
        _ => Ok(false),
    }
}
fn link(
    s: &mut ObservedSourceState,
    o: NodeId,
    a: Option<ProductionInlineLinkTarget<'_>>,
    b: Option<ProductionInlineLinkTarget<'_>>,
) -> Result<bool> {
    s.charge(o, 0, 0, 1)?;
    match (a, b) {
        (None, None) => Ok(true),
        (
            Some(ProductionInlineLinkTarget::Internal { anchor_id: a }),
            Some(ProductionInlineLinkTarget::Internal { anchor_id: b }),
        ) => text(s, o, a, b),
        (
            Some(ProductionInlineLinkTarget::Uri { uri: a }),
            Some(ProductionInlineLinkTarget::Uri { uri: b }),
        ) => text(s, o, a, b),
        _ => Ok(false),
    }
}
fn site(
    s: &mut ObservedSourceState,
    a: &ProductionInlineSite<'_>,
    b: &ProductionInlineSite<'_>,
) -> Result<bool> {
    let o = a.owner;
    Ok(fixed(s, o, a.owner, b.owner)?
        && fixed(s, o, a.source_span, b.source_span)?
        && text(s, o, a.language, b.language)?
        && content(s, o, a.content, b.content)?
        && reference(s, o, a.reference, b.reference)?
        && link(s, o, a.link_target, b.link_target)?)
}
fn table(s: &mut ObservedSourceState, a: &ProductionTable, b: &ProductionTable) -> Result<bool> {
    let o = a.owner();
    if !fixed(s, o, a.owner(), b.owner())?
        || !fixed(s, o, a.source_span(), b.source_span())?
        || !fixed(s, o, a.caption_event_range(), b.caption_event_range())?
    {
        return Ok(false);
    }
    if !slice(s, o, a.columns(), b.columns(), |s, a, b| {
        use typaxis_document::ColumnSizing;
        s.charge(o, 0, 0, 1)?;
        match (&a.sizing, &b.sizing) {
            (ColumnSizing::Fixed(a), ColumnSizing::Fixed(b)) => fixed(s, o, a, b),
            (ColumnSizing::Fraction(a), ColumnSizing::Fraction(b)) => fixed(s, o, a, b),
            _ => Ok(false),
        }
    })? || !fixed_slice(s, o, a.rows(), b.rows())?
        || !fixed_slice(s, o, a.cells(), b.cells())?
        || !style(s, o, a.style(), b.style())?
        || !page(s, o, a.page_name(), b.page_name())?
    {
        return Ok(false);
    }
    Ok(true)
}

pub(super) fn equal_flows(
    s: &mut ObservedSourceState,
    a: &PreparedBookV2TextFlow<'_>,
    b: &PreparedBookV2TextFlow<'_>,
) -> Result<bool> {
    let root = NodeId::new(0);
    if !fixed_slice(s, root, &a.events, &b.events)? {
        return Ok(false);
    }
    if !slice(s, root, &a.paragraphs, &b.paragraphs, |s, a, b| {
        let o = a.owner;
        Ok(page(s, o, a.page_name.as_ref(), b.page_name.as_ref())?
            && fixed(s, o, a.owner, b.owner)?
            && style(s, o, &a.style, &b.style)?
            && slice(s, o, &a.items, &b.items, site)?)
    })? {
        return Ok(false);
    }
    if !slice(
        s,
        root,
        &a.named_page_breaks,
        &b.named_page_breaks,
        |s, a, b| Ok(fixed(s, a.0, a.0, b.0)? && text(s, a.0, a.1.as_str(), b.1.as_str())?),
    )? {
        return Ok(false);
    }
    if !slice(s, root, &a.figures, &b.figures, |s, a, b| {
        let o = a.owner;
        Ok(fixed(s, o, a.owner, b.owner)?
            && fixed(s, o, a.source_span, b.source_span)?
            && fixed(s, o, a.image_id, b.image_id)?
            && text(s, o, a.alternative, b.alternative)?
            && text(s, o, a.placement, b.placement)?
            && style(s, o, &a.style, &b.style)?
            && page(s, o, a.page_name.as_ref(), b.page_name.as_ref())?)
    })? || !slice(s, root, &a.tables, &b.tables, table)?
        || !fixed(s, root, a.table_record_charge, b.table_record_charge)?
        || !fixed(s, root, a.source_record_charge, b.source_record_charge)?
    {
        return Ok(false);
    }
    if !slice(s, root, &a.lists, &b.lists, |s, a, b| {
        let o = a.owner;
        Ok(fixed(s, o, a.owner, b.owner)?
            && fixed(s, o, a.ordered, b.ordered)?
            && fixed(s, o, a.start, b.start)?
            && style(s, o, &a.style, &b.style)?
            && page(s, o, a.page_name.as_ref(), b.page_name.as_ref())?)
    })? || !slice(s, root, &a.list_items, &b.list_items, |s, a, b| {
        let o = a.owner;
        Ok(fixed(s, o, a.owner, b.owner)?
            && fixed(s, o, a.list_index, b.list_index)?
            && fixed(s, o, a.item_index, b.item_index)?
            && fixed(s, o, a.source_span, b.source_span)?
            && text(s, o, a.language, b.language)?
            && fixed(s, o, a.ordered_value, b.ordered_value)?)
    })? {
        return Ok(false);
    }
    if !slice(
        s,
        root,
        &a.description_lists,
        &b.description_lists,
        |s, a, b| {
            let o = a.owner();
            Ok(fixed(s, o, a.owner(), b.owner())?
                && style(s, o, a.style(), b.style())?
                && page(s, o, a.page_name(), b.page_name())?)
        },
    )? || !slice(
        s,
        root,
        &a.description_items,
        &b.description_items,
        |s, a, b| {
            let o = a.owner();
            Ok(fixed(s, o, a.owner(), b.owner())?
                && fixed(s, o, a.list_index(), b.list_index())?
                && fixed(s, o, a.item_index(), b.item_index())?
                && fixed(s, o, a.term_paragraph_index(), b.term_paragraph_index())?
                && fixed(s, o, a.source_span(), b.source_span())?
                && text(s, o, a.language(), b.language())?)
        },
    )? {
        return Ok(false);
    }
    if !slice(
        s,
        root,
        &a.footnote_definitions,
        &b.footnote_definitions,
        |s, a, b| {
            let o = a.owner;
            Ok(fixed(s, o, a.owner, b.owner)?
                && text(s, o, a.id, b.id)?
                && fixed(s, o, a.style_paragraph, b.style_paragraph)?
                && text(s, o, a.language, b.language)?)
        },
    )? {
        return Ok(false);
    }
    s.charge(root, 0, 0, 1)?;
    match (&a.footnote_base_style, &b.footnote_base_style) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            if !style(s, root, a, b)? {
                return Ok(false);
            }
        }
        _ => return Ok(false),
    }
    if !slice(
        s,
        root,
        a.generated.buffers(),
        b.generated.buffers(),
        |s, a, b| {
            Ok(fixed(s, root, a.key(), b.key())?
                && fixed(s, root, a.text_id(), b.text_id())?
                && text(s, root, a.utf8(), b.utf8())?)
        },
    )? || !fixed(
        s,
        root,
        a.generated.reference_fingerprint(),
        b.generated.reference_fingerprint(),
    )? || !fixed(
        s,
        root,
        a.generated.generated_bytes(),
        b.generated.generated_bytes(),
    )? {
        return Ok(false);
    }
    s.charge(root, 0, 0, 1)?;
    match (a.page_reference_values(), b.page_reference_values()) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            if !fixed_slice(s, root, a, b)? {
                return Ok(false);
            }
        }
        _ => return Ok(false),
    }
    Ok(
        fixed(s, root, a.text_bytes, b.text_bytes)?
            && fixed(s, root, a.fingerprint, b.fingerprint)?,
    )
}
