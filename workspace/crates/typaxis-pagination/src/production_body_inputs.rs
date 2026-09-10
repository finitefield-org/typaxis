//! Closed, borrowed measurement views. Constructors remain at the verified
//! legacy/book-2 entry points; these views never issue source or paint receipts.
use super::*;
use typaxis_layout::{
    ProductionInlineFrame, ProductionInlineParagraphLineLayout, ProductionListFrame,
    ProductionNativeMathDisplayBlock, ProductionPreparedFigure, ProductionTableFrame,
};

#[derive(Clone, Copy)]
pub(super) enum BodyLines<'s, 'p, 'a> {
    Legacy(&'s ProductionInlineLineLayout<'p, 'a>),
    #[cfg(feature = "book-v2-staging")]
    BookV2(&'s typaxis_layout::book_v2::BookV2InlineLineLayout<'p, 'a>),
}
macro_rules! lines_call {
    ($v:expr, $($call:tt)+) => { match $v {
        BodyLines::Legacy(v) => v.$($call)+,
        #[cfg(feature = "book-v2-staging")]
        BodyLines::BookV2(v) => v.$($call)+,
    }};
}
impl<'s, 'p, 'a> BodyLines<'s, 'p, 'a> {
    pub fn has_named_page_plan(self) -> bool {
        match self {
            Self::Legacy(_) => false,
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(lines) => lines
                .frames()
                .and_then(|f| f.page_plan())
                .is_some_and(|p| p.has_source_names()),
        }
    }

    pub fn source_unit_start(self, _paragraph: usize, _unit: u32) -> Option<Length> {
        match self {
            Self::Legacy(_) => None,
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(lines) => lines.frames()?.source_unit_start(_paragraph, _unit),
        }
    }
    pub fn paragraphs(self) -> &'s [ProductionInlineParagraphLineLayout<'p, 'a>] {
        lines_call!(self, paragraphs())
    }
    pub fn source_flow(self) -> BodyFlow<'s> {
        match self {
            Self::Legacy(v) => BodyFlow::Legacy(v.source_flow()),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => BodyFlow::BookV2(v.prepared().source_flow()),
        }
    }
    pub fn frames(self) -> Option<BodyFrames<'s, 'p, 'a>> {
        match self {
            Self::Legacy(v) => v.frames().map(BodyFrames::Legacy),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => v.frames().map(BodyFrames::BookV2),
        }
    }
    pub fn figures(self) -> &'s [ProductionPreparedFigure<'a>] {
        match self {
            Self::Legacy(v) => v.figures(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => v.prepared().figures(),
        }
    }
    pub fn native_math_blocks(self) -> &'s [ProductionNativeMathDisplayBlock] {
        match self {
            Self::Legacy(v) => v.native_math_blocks(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => v
                .prepared()
                .native_math()
                .map_or(&[], |m| m.display_blocks()),
        }
    }
    pub fn marker_geometry(self, index: usize, footnote: bool) -> Option<BodyMarker> {
        macro_rules! get {
            ($markers:expr,$list:expr) => {{
                let m = $markers.get(index)?;
                BodyMarker {
                    owner: m.source().owner(),
                    list_index: $list,
                    advance: m.advance(),
                    ascent: m.font().ascender(),
                    descent: m.font().descender(),
                }
            }};
        }
        Some(match (self, footnote) {
            (Self::Legacy(v), false) => get!(
                v.list_markers(),
                Some(v.list_markers().get(index)?.source().list_index() as usize)
            ),
            (Self::Legacy(v), true) => get!(v.footnote_markers(), None),
            #[cfg(feature = "book-v2-staging")]
            (Self::BookV2(v), false) => get!(
                v.prepared().shaped().list_markers(),
                Some(
                    v.prepared()
                        .shaped()
                        .list_markers()
                        .get(index)?
                        .source()
                        .list_index() as usize
                )
            ),
            #[cfg(feature = "book-v2-staging")]
            (Self::BookV2(v), true) => get!(v.prepared().shaped().footnote_markers(), None),
        })
    }
    pub fn marker_count(self, footnote: bool) -> usize {
        match (self, footnote) {
            (Self::Legacy(v), false) => v.list_markers().len(),
            (Self::Legacy(v), true) => v.footnote_markers().len(),
            #[cfg(feature = "book-v2-staging")]
            (Self::BookV2(v), false) => v.prepared().shaped().list_markers().len(),
            #[cfg(feature = "book-v2-staging")]
            (Self::BookV2(v), true) => v.prepared().shaped().footnote_markers().len(),
        }
    }
    pub fn marker(self, index: usize, footnote: bool) -> Option<(NodeId, Length, Length)> {
        macro_rules! get {
            ($markers:expr) => {{
                let m = $markers.get(index)?;
                Some((
                    m.source().owner(),
                    m.font().ascender(),
                    m.font().descender(),
                ))
            }};
        }
        match (self, footnote) {
            (Self::Legacy(v), false) => get!(v.list_markers()),
            (Self::Legacy(v), true) => get!(v.footnote_markers()),
            #[cfg(feature = "book-v2-staging")]
            (Self::BookV2(v), false) => get!(v.prepared().shaped().list_markers()),
            #[cfg(feature = "book-v2-staging")]
            (Self::BookV2(v), true) => get!(v.prepared().shaped().footnote_markers()),
        }
    }
}
pub(super) struct BodyMarker {
    pub owner: NodeId,
    pub list_index: Option<usize>,
    pub advance: PositiveLength,
    pub ascent: Length,
    pub descent: Length,
}
#[derive(Clone, Copy)]
pub(super) enum BodyFlow<'a> {
    Legacy(&'a typaxis_syntax::ProductionTextFlow<'a>),
    #[cfg(feature = "book-v2-staging")]
    BookV2(&'a typaxis_syntax::book_v2::PreparedBookV2TextFlow<'a>),
}
macro_rules! flow_call {
    ($v:expr, $($call:tt)+) => { match $v {
        BodyFlow::Legacy(v) => v.$($call)+,
        #[cfg(feature = "book-v2-staging")]
        BodyFlow::BookV2(v) => v.$($call)+,
    }};
}
impl<'a> BodyFlow<'a> {
    pub fn has_named_page_break(self, owner: NodeId) -> bool {
        match self {
            Self::Legacy(_) => {
                let _ = owner;
                false
            }
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(flow) => flow.page_break_name(owner).is_some(),
        }
    }

    pub fn events(self) -> &'a [Event] {
        flow_call!(self, events())
    }
    pub fn paragraphs(self) -> &'a [typaxis_syntax::ProductionTextParagraph<'a>] {
        flow_call!(self, paragraphs())
    }
    pub fn lists(self) -> &'a [typaxis_syntax::ProductionList] {
        flow_call!(self, lists())
    }
    pub fn list_items(self) -> &'a [typaxis_syntax::ProductionListItem<'a>] {
        flow_call!(self, list_items())
    }
    #[cfg(feature = "book-v2-staging")]
    pub fn description_lists(self) -> &'a [typaxis_syntax::book_v2::BookV2DescriptionList] {
        flow_call!(self, description_lists())
    }
    pub fn tables(self) -> &'a [typaxis_syntax::ProductionTable] {
        flow_call!(self, tables())
    }
    pub fn container(self, owner: NodeId) -> Option<(ComputedMachineBlockStyle, bool)> {
        flow_call!(
            self,
            semantic_container_style(owner).map(|s| (s.block_style(), s.page_name().is_some()))
        )
    }
}
#[derive(Clone, Copy)]
pub(super) enum BodyFrames<'s, 'p, 'a> {
    Legacy(&'s typaxis_layout::ProductionBodyInlineFrames<'p, 'a>),
    #[cfg(feature = "book-v2-staging")]
    BookV2(&'s typaxis_layout::book_v2::BookV2BodyInlineFrames<'p, 'a>),
}
macro_rules! frames_call {
    ($v:expr, $($call:tt)+) => { match $v {
        BodyFrames::Legacy(v) => v.$($call)+,
        #[cfg(feature = "book-v2-staging")]
        BodyFrames::BookV2(v) => v.$($call)+,
    }};
}
impl<'s, 'p, 'a> BodyFrames<'s, 'p, 'a> {
    pub fn footnotes(self) -> &'s [typaxis_layout::ProductionFootnoteFrame] {
        match self {
            Self::Legacy(v) => v.footnotes(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => v.footnotes(),
        }
    }
    pub fn region(self, owner: NodeId) -> Option<ProductionInlineFrame> {
        frames_call!(self, region(owner))
    }
    pub fn lists(self) -> &'s [ProductionListFrame] {
        frames_call!(self, lists())
    }
    pub fn tables(self) -> &'s [ProductionTableFrame] {
        frames_call!(self, tables())
    }
}
#[derive(Clone, Copy)]
pub(super) enum BodyBlocks<'a> {
    Legacy(&'a [typaxis_layout::StagingPreparedVectorBlock]),
    #[cfg(feature = "book-v2-staging")]
    BookV2(&'a [typaxis_layout::book_v2::BookV2VectorBlock<'a>]),
}
impl<'a> BodyBlocks<'a> {
    pub fn len(self) -> usize {
        match self {
            Self::Legacy(v) => v.len(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => v.len(),
        }
    }
    pub fn get(self, index: usize) -> Option<BodyBlock<'a>> {
        match self {
            Self::Legacy(v) => v.get(index).map(BodyBlock::Legacy),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(v) => v.get(index).map(BodyBlock::BookV2),
        }
    }
}
#[derive(Clone, Copy)]
pub(super) enum BodyBlock<'a> {
    Legacy(&'a typaxis_layout::StagingPreparedVectorBlock),
    #[cfg(feature = "book-v2-staging")]
    BookV2(&'a typaxis_layout::book_v2::BookV2VectorBlock<'a>),
}
macro_rules! block_call {
    ($v:expr, $($call:tt)+) => { match $v {
        BodyBlock::Legacy(v) => v.$($call)+,
        #[cfg(feature = "book-v2-staging")]
        BodyBlock::BookV2(v) => v.$($call)+,
    }};
}
impl BodyBlock<'_> {
    pub fn owner(self) -> NodeId {
        block_call!(self, owner())
    }
    pub fn has_named_page(self) -> bool {
        block_call!(self, page_name().is_some())
    }
    pub fn viewport_width(self) -> PositiveLength {
        block_call!(self, viewport_width())
    }
    pub fn viewport_height(self) -> PositiveLength {
        block_call!(self, viewport_height())
    }
    pub fn content_height(self) -> PositiveLength {
        block_call!(self, content_height())
    }
    pub fn space_before(self) -> typaxis_core::NonNegativeLength {
        block_call!(self, space_before())
    }
    pub fn space_after(self) -> typaxis_core::NonNegativeLength {
        block_call!(self, space_after())
    }
    pub fn keep_caption(self) -> bool {
        block_call!(self, keep_caption())
    }
    pub fn keep_with_next(self) -> bool {
        block_call!(self, keep_with_next())
    }
    pub fn baseline(self) -> Option<typaxis_core::NonNegativeLength> {
        block_call!(self, baseline())
    }
    pub fn viewport_top_offset(self) -> typaxis_core::NonNegativeLength {
        block_call!(self, viewport_top_offset())
    }
    pub fn frame(
        self,
        lines: BodyLines<'_, '_, '_>,
        body: Rect,
    ) -> Result<(Length, PositiveLength, Length), ProductionBodyPaginationError> {
        match self {
            Self::Legacy(block) => list::block_frame_shared(lines, body, block),
            // The successor block was prepared against these exact lines and
            // already includes its actual containing frame and number collision.
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(block) => Ok((
                block.inner_frame_left(),
                block.inner_frame_width(),
                block.viewport_left(),
            )),
        }
    }
}
