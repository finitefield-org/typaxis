//! Authored description terms are inline-bearing flow regions, with their own
//! owners. They neither occupy the ordinary list registry nor generate markers.
use super::*;
use typaxis_document_package::WireSemanticDescriptionItem;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookV2DescriptionList {
    owner: NodeId,
    style: SemanticContainerInheritanceStyle,
    page_name: Option<typaxis_core::PageName>,
}
impl BookV2DescriptionList {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn style(&self) -> &SemanticContainerInheritanceStyle {
        &self.style
    }
    pub const fn page_name(&self) -> Option<&typaxis_core::PageName> {
        self.page_name.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookV2DescriptionItem<'a> {
    owner: NodeId,
    list_index: u32,
    item_index: u32,
    term_paragraph_index: u32,
    source_span: SourceSpan,
    language: &'a str,
}
impl<'a> BookV2DescriptionItem<'a> {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn list_index(&self) -> u32 {
        self.list_index
    }
    pub const fn item_index(&self) -> u32 {
        self.item_index
    }
    /// Index into the common inline-bearing flow storage. The corresponding
    /// Begin event and source node remain DescriptionTerm, never Paragraph.
    pub const fn term_paragraph_index(&self) -> u32 {
        self.term_paragraph_index
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn language(&self) -> &'a str {
        self.language
    }
}

impl BookV2DescriptionList {
    pub(super) fn try_copy_for_page_candidate(
        &self,
        copy: &mut super::book_v2::page_candidates::CandidateCopy<'_>,
    ) -> Result<Self, super::book_v2::page_candidates::BookV2PageCandidateError> {
        let style = copy.style(self.owner, &self.style)?;
        let page_name = copy.page_name(self.owner, self.page_name.as_ref())?;
        Ok(Self {
            owner: self.owner,
            style,
            page_name,
        })
    }
}
impl<'a> BookV2DescriptionItem<'a> {
    pub(super) fn copy_for_page_candidate(&self) -> Self {
        Self {
            owner: self.owner,
            list_index: self.list_index,
            item_index: self.item_index,
            term_paragraph_index: self.term_paragraph_index,
            source_span: self.source_span,
            language: self.language,
        }
    }
}

impl<'a, P, N> SourceTextFlow<'a, P, N> {
    pub fn description_lists(&self) -> &[BookV2DescriptionList] {
        &self.description_lists
    }
    pub fn description_items(&self) -> &[BookV2DescriptionItem<'a>] {
        &self.description_items
    }
}

impl<'a, S: FlowSource<'a>> Collector<'a, S> {
    pub(super) fn description_list(
        &mut self,
        owner: NodeId,
        classes: &[String],
        items: &'a [WireSemanticDescriptionItem<S::Kind>],
        parent: Option<&SemanticContainerInheritanceStyle>,
    ) -> Result<(), ProductionFlowError> {
        use ProductionFlowRegionKind as Kind;
        self.source.reserve_records(owner, 1)?;
        let (list_rules, _) = self
            .rules
            .descriptions
            .as_ref()
            .ok_or_else(|| failure(ProductionFlowErrorKind::DescriptionListStaging, owner))?;
        let style = self.source.operations().descendant(
            owner,
            &self.rules,
            "paragraph",
            classes,
            list_rules,
            parent,
            true,
        )?;
        let page_name = self.source.operations().page_name(
            owner,
            &self.rules,
            list_rules,
            "paragraph",
            classes,
            true,
        )?;
        self.begin(owner, Kind::DescriptionList)?;
        if (self.description_lists.len() as u64)
            .checked_add(self.description_items.len() as u64)
            .is_none_or(|n| n >= self.source.limits().get().max_fragments)
        {
            return Err(failure(ProductionFlowErrorKind::NodeLimit, owner));
        }
        let list_index = u32::try_from(self.description_lists.len())
            .map_err(|_| failure(ProductionFlowErrorKind::NodeLimit, owner))?;
        self.source
            .operations()
            .grow(owner, &mut self.description_lists, 1, false)?;
        self.description_lists.push(BookV2DescriptionList {
            owner,
            style: self.source.operations().copy_style(owner, &style)?,
            page_name,
        });
        for (index, item) in items.iter().enumerate() {
            let item_owner = NodeId::new(item.node_id);
            self.begin(item_owner, Kind::DescriptionItem)?;
            if (self.description_lists.len() as u64)
                .checked_add(self.description_items.len() as u64)
                .is_none_or(|n| n >= self.source.limits().get().max_fragments)
            {
                return Err(failure(ProductionFlowErrorKind::NodeLimit, item_owner));
            }
            let item_index = u32::try_from(index)
                .map_err(|_| failure(ProductionFlowErrorKind::NodeLimit, item_owner))?;
            let term_paragraph_index = u32::try_from(self.paragraphs.len())
                .map_err(|_| failure(ProductionFlowErrorKind::NodeLimit, item_owner))?;
            let source_span = lower_span(item.span)
                .map_err(|_| failure(ProductionFlowErrorKind::ReceiptMismatch, item_owner))?;
            let language = self.language(item_owner)?;
            self.source.reserve_records(item_owner, 1)?;
            self.source
                .operations()
                .grow(item_owner, &mut self.description_items, 1, false)?;
            self.description_items.push(BookV2DescriptionItem {
                owner: item_owner,
                list_index,
                item_index,
                term_paragraph_index,
                source_span,
                language,
            });
            self.description_term(&item.term, &style)?;
            self.blocks(&item.blocks, Some(&style))?;
            self.end(item_owner)?;
        }
        self.end(owner)
    }

    fn description_term(
        &mut self,
        term: &'a typaxis_document_package::WireDescriptionTerm,
        parent: &SemanticContainerInheritanceStyle,
    ) -> Result<(), ProductionFlowError> {
        let owner = NodeId::new(term.node_id);
        self.source.reserve_records(owner, 1)?;
        let (_, rules) = self
            .rules
            .descriptions
            .as_ref()
            .ok_or_else(|| failure(ProductionFlowErrorKind::DescriptionListStaging, owner))?;
        let style = self.source.operations().descendant(
            owner,
            &self.rules,
            "paragraph",
            &term.classes,
            rules,
            Some(parent),
            true,
        )?;
        let page_name = self.source.operations().page_name(
            owner,
            &self.rules,
            rules,
            "paragraph",
            &term.classes,
            true,
        )?;
        self.begin(owner, ProductionFlowRegionKind::DescriptionTerm)?;
        let mut items = Vec::new();
        let language = self.language(owner)?;
        self.inlines(&term.children, language, &mut items)?;
        self.source.operations().step(owner, items.len() as u64)?;
        let needs_font = items.iter().any(|i| {
            matches!(i.content, ProductionInlineContent::Text { utf8, .. } if !utf8.is_empty())
                || matches!(
                    i.content,
                    ProductionInlineContent::Reference | ProductionInlineContent::FootnoteReference
                )
        });
        if needs_font
            && (style.font_families().is_none()
                || style.font_size().is_none()
                || style.line_height().is_none())
        {
            return Err(failure(ProductionFlowErrorKind::MissingTextStyle, owner));
        }
        let index = u32::try_from(self.paragraphs.len())
            .map_err(|_| failure(ProductionFlowErrorKind::NodeLimit, owner))?;
        self.source
            .operations()
            .grow(owner, &mut self.paragraphs, 1, false)?;
        self.paragraphs.push(ProductionTextParagraph {
            owner,
            style,
            page_name,
            items,
        });
        self.event(ProductionFlowEvent::Paragraph { index }, owner)?;
        self.end(owner)
    }
}
