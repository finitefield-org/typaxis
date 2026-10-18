//! Readonly semantic facts from authentic physical display owners.
use super::*;
use typaxis_syntax::book_v2::{
    visit_book_v2_structure, BookV2StructureVisitor, PreparedBookV2Language,
    PreparedBookV2LanguageChild, PreparedBookV2TextFlow,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2SemanticKind {
    Text,
    Label,
    Formula,
    Figure,
    EquationNumber,
    Separator,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2SemanticArtifact {
    RepeatedHeader,
    FootnoteSeparator,
    RunningHeader,
    RunningFooter,
}

/// Semantic facts keep their physical display's paint family.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2SemanticPaintView, BookV2ColumnPaintIndex, BookV2BodyPaintIndex};
/// fn body<'v>(view: BookV2SemanticPaintView<'v, BookV2ColumnPaintIndex>) -> BookV2SemanticPaintView<'v, BookV2BodyPaintIndex> { view }
/// ```
pub struct BookV2SemanticPaintView<'v, P: Copy> {
    paint: P,
    owner: Option<NodeId>,
    page: u32,
    fragment: usize,
    kind: BookV2SemanticKind,
    artifact: Option<BookV2SemanticArtifact>,
    alternative: Option<&'v str>,
    actual: Option<&'v str>,
}
impl<'v, P: Copy> BookV2SemanticPaintView<'v, P> {
    pub fn paint(&self) -> P {
        self.paint
    }
    pub fn owner(&self) -> Option<NodeId> {
        self.owner
    }
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn fragment_index(&self) -> usize {
        self.fragment
    }
    pub fn kind(&self) -> BookV2SemanticKind {
        self.kind
    }
    pub fn artifact(&self) -> Option<BookV2SemanticArtifact> {
        self.artifact
    }
    pub fn alternative(&self) -> Option<&'v str> {
        self.alternative
    }
    pub fn actual_text(&self) -> Option<&'v str> {
        self.actual
    }
}

/// Syntax traversal keeps node strings borrowed from the retained source.
/// The temporary view cannot expose a prepared physical body flow.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::BookV2SyntaxStructureView;
/// fn escape(view: BookV2SyntaxStructureView<'_>) { let _ = view.flow; }
/// ```
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::BookV2SyntaxStructureView;
/// use typaxis_syntax::book_v2::PreparedBookV2TextFlow;
/// fn forged<'src>(flow: &'src PreparedBookV2TextFlow<'src>) -> BookV2SyntaxStructureView<'src> {
///     BookV2SyntaxStructureView { flow }
/// }
/// ```
pub struct BookV2SyntaxStructureView<'src> {
    flow: &'src PreparedBookV2TextFlow<'src>,
}
impl<'src> BookV2SyntaxStructureView<'src> {
    pub fn visit<V: BookV2StructureVisitor<'src>>(&self, visitor: &mut V) -> Result<(), V::Error> {
        visit_book_v2_structure(self.flow, visitor)
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.flow.fingerprint()
    }
    pub fn max_ast_nesting_depth(&self) -> u32 {
        self.flow.body().body().limits().get().max_ast_nesting_depth
    }
    pub fn languages(&self) -> &'src [PreparedBookV2Language<'src>] {
        self.flow.navigation().languages()
    }
    pub fn language_children(&self) -> &'src [PreparedBookV2LanguageChild] {
        self.flow.navigation().language_children()
    }
}

/// The CommandDisplay supertrait seals semantic views to Body and Column.
pub trait BookV2SemanticDisplay<'a>: BookV2CommandDisplay<'a> {
    fn semantic_paint(
        &self,
        index: usize,
    ) -> Result<Option<BookV2SemanticPaintView<'_, Self::Paint>>, BookV2MathDisplayError>;
    fn whole_text(&self, index: usize) -> Result<Option<&str>, BookV2MathDisplayError>;
    fn semantic_image_baseline(
        &self,
        index: usize,
    ) -> Result<Option<Length>, BookV2MathDisplayError>;
    fn page_count(&self) -> usize;
    fn structure_source(&self) -> BookV2SyntaxStructureView<'_>;
}

macro_rules! authentic_semantic_display {
    ($display:ident, $paint:ident $(, $regions:ident)?) => {
        impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2SemanticDisplay<'a>
            for $display<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
        {
            fn semantic_paint(&self, index: usize) -> Result<Option<BookV2SemanticPaintView<'_, Self::Paint>>, BookV2MathDisplayError> {
                use $paint as P;
                use BookV2SemanticKind as K;
                let Some(paint) = self.paints().get(index).copied() else { return Ok(None); };
                let (owner, page, fragment, kind, repeated, alternative, actual) = match paint {
                    P::Text(i) => {
                        let d = &self.text().draws()[i];
                        (Some(d.owner()), d.page_index(), d.fragment_index(), K::Text, d.repeated_header(), None, None)
                    }
                    $(P::PageRegionText { .. } => {
                        let (region, draw) = self.page_region_paint(paint)
                            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
                        let r = &self.$regions()[region];
                        let d = &r.draws()[draw];
                        return Ok(Some(BookV2SemanticPaintView {
                            paint, owner: Some(d.owner()), page: r.page_index(), fragment: region, kind: K::Text,
                            artifact: Some(match r.kind() {
                                typaxis_syntax::book_v2::BookV2PageRegionKind::Header => BookV2SemanticArtifact::RunningHeader,
                                typaxis_syntax::book_v2::BookV2PageRegionKind::Footer => BookV2SemanticArtifact::RunningFooter,
                            }), alternative: None, actual: None,
                        }));
                    },)?
                    P::Marker(i) => {
                        let d = &self.markers().draws()[i];
                        (Some(d.source().owner()), d.fragment().fragment().page_index(), d.fragment_index(), K::Label, d.repeated_header(), None, None)
                    }
                    P::Math(i) => {
                        let draw = &self.math().draws()[i];
                        let d = draw.terminal();
                        let actual = draw.actual_text().ok_or_else(|| error(d.source().owner(), E::ReceiptMismatch))?;
                        (Some(d.source().owner()), d.page_index(), d.fragment_index(), K::Formula, d.repeated_header(), Some(draw.alternative()), Some(actual))
                    }
                    P::Image(i) => {
                        let d = &self.images().draws()[i];
                        let (alt, actual) = match d.source() {
                            BookV2ImageSource::Figure(f) => (f.source().alternative(), None),
                            BookV2ImageSource::Vector(v) => {
                                let alt = v.source().alternative();
                                (alt.alternative(), alt.authored_actual_text())
                            }
                        };
                        (Some(d.source().owner()), d.fragment().fragment().page_index(), d.fragment_index(), K::Figure, d.repeated_header(), Some(alt), actual)
                    }
                    P::EquationNumber(i) => {
                        let d = self.numbers().draws()[i].placement();
                        (Some(d.geometry().owner()), d.geometry().page_index(), d.geometry().fragment_index() as usize, K::EquationNumber, d.repeated_header(), None, None)
                    }
                    P::FootnoteSeparator(i) => {
                        let d = &self.markers().separators()[i];
                        (None, d.page_index(), d.before_fragment_index(), K::Separator, false, None, None)
                    }
                };
                let artifact = if repeated { Some(BookV2SemanticArtifact::RepeatedHeader) }
                    else if kind == K::Separator { Some(BookV2SemanticArtifact::FootnoteSeparator) }
                    else { None };
                Ok(Some(BookV2SemanticPaintView { paint, owner, page, fragment, kind, artifact, alternative, actual }))
            }
            fn whole_text(&self, index: usize) -> Result<Option<&str>, BookV2MathDisplayError> {
                use $paint as P;
                Ok(match self.paints().get(index).copied() {
                    Some(P::Text(i)) => Some(self.text().draws()[i].exact_text()),
                    Some(P::Marker(i)) => Some(self.markers().draws()[i].source().utf8()),
                    Some(P::EquationNumber(i)) => Some(self.numbers().draws()[i].shape().text()),
                    _ => None,
                })
            }
            fn semantic_image_baseline(&self, index: usize) -> Result<Option<Length>, BookV2MathDisplayError> {
                use $paint as P;
                Ok(match self.paints().get(index).copied() {
                    Some(P::Math(i)) => Some(self.math().draws()[i].terminal().baseline()),
                    Some(P::Image(i)) => self.images().draws()[i].fragment().fragment().baseline(),
                    _ => None,
                })
            }
            fn page_count(&self) -> usize { self.source().source().geometry().pages().len() }
            fn structure_source(&self) -> BookV2SyntaxStructureView<'_> {
                BookV2SyntaxStructureView { flow: self.source().source().original_graph().lines().prepared().source_flow() }
            }
        }
    };
}
authentic_semantic_display!(BookV2BodyDisplay, BookV2BodyPaintIndex, page_regions);
authentic_semantic_display!(BookV2ColumnDisplay, BookV2ColumnPaintIndex, page_regions);

impl<'src> BookV2SyntaxStructureView<'src> {
    pub fn tables(&self) -> &'src [typaxis_syntax::ProductionTable] {
        self.flow.tables()
    }
    pub fn lists(&self) -> &'src [typaxis_syntax::ProductionList] {
        self.flow.lists()
    }
}
