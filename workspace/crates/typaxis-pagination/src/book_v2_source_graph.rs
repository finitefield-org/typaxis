//! Read-only source data for a closed physical occurrence. This view never
//! exposes the prepared flow that authorizes a single-body consumer.
use super::*;
use typaxis_layout::book_v2::{BookV2InlineLineLayout, BookV2VectorBlockLayout};

/// The view cannot expose a single-body flow authorization.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2SourceGraph, BookV2PreparedBodyFlow};
/// fn flow<'b, 'f, 's, 'p, 'a>(graph: BookV2SourceGraph<'b, 'f, 's, 'p, 'a>)
///     -> &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a> { graph.flow }
/// ```
#[derive(Clone, Copy)]
pub struct BookV2SourceGraph<'b, 'f, 's, 'p, 'a> {
    flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2SourceGraph<'b, 'f, 's, 'p, 'a> {
    pub(in crate::production_body::body_flow) fn new(
        flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    ) -> Self {
        Self { flow }
    }
    pub fn lines(self) -> &'s BookV2InlineLineLayout<'p, 'a> {
        self.flow.lines()
    }
    pub fn blocks(self) -> Option<&'f BookV2VectorBlockLayout<'s, 'p, 'a>> {
        self.flow.blocks()
    }
    pub fn verify(
        self,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.flow.verify(
            self.flow.lines(),
            self.flow.blocks(),
            self.flow.footnotes(),
            limits,
        )
    }
}

impl<'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn original_graph(&self) -> BookV2SourceGraph<'b, 'f, 's, 'p, 'a> {
        self.source().original_graph()
    }
    pub fn fragment_graph(&self, index: usize) -> Option<BookV2SourceGraph<'b, 'f, 's, 'p, 'a>> {
        self.fragment_flow(index).map(BookV2SourceGraph::new)
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a> {
    pub fn graph(&self) -> BookV2SourceGraph<'b, 'f, 's, 'p, 'a> {
        BookV2SourceGraph::new(self.measurements().flow())
    }
}

/// Reference facts come only from an authentic retained source graph.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::BookV2SourceReferenceView;
/// use typaxis_core::NodeId;
/// fn forged() -> BookV2SourceReferenceView {
///     BookV2SourceReferenceView { owner: NodeId::new(1), definition: 0, source_definition: None }
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2SourceReferenceView {
    owner: NodeId,
    definition: usize,
    source_definition: Option<usize>,
}
impl BookV2SourceReferenceView {
    pub fn owner(self) -> NodeId {
        self.owner
    }
    pub fn definition_index(self) -> usize {
        self.definition
    }
    pub fn source_definition(self) -> Option<usize> {
        self.source_definition
    }
}
impl<'b, 'f, 's, 'p, 'a> BookV2SourceGraph<'b, 'f, 's, 'p, 'a> {
    pub fn definition_count(self) -> usize {
        self.flow.footnotes().definitions().len()
    }
    pub fn definition_owner(self, index: usize) -> Option<NodeId> {
        self.flow
            .footnotes()
            .definitions()
            .get(index)
            .map(|definition| definition.owner())
    }
    pub fn reference_count(self) -> usize {
        self.flow.references().len()
    }
    pub fn reference(self, index: usize) -> Option<BookV2SourceReferenceView> {
        let source = self.flow.references().get(index)?.source();
        Some(BookV2SourceReferenceView {
            owner: source.owner(),
            definition: source.definition_index(),
            source_definition: source.source_definition(),
        })
    }
}
