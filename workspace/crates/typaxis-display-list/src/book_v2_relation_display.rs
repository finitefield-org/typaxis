//! Readonly original relations and authentic selected first-reference demand.
//! The display owner keeps Body and Column source families distinct.
use super::*;
use typaxis_core::NodeId;
pub use typaxis_pagination::book_v2::BookV2SourceReferenceView as BookV2RelationReferenceView;
use typaxis_syntax::{ProductionList, ProductionTable};

mod relation_sealed {
    use super::*;
    pub trait Facts {
        fn relation_tables(&self) -> &[ProductionTable];
        fn relation_lists(&self) -> &[ProductionList];
        fn relation_definition_count(&self) -> usize;
        fn relation_definition_owner(&self, index: usize) -> Option<NodeId>;
        fn relation_reference_count(&self) -> usize;
        fn relation_reference(&self, index: usize) -> Option<BookV2RelationReferenceView>;
        fn relation_unreferenced_definitions(&self) -> usize;
        fn relation_has_selected_demand(&self) -> bool;
        fn relation_first_reference(&self, definition: usize) -> Option<NodeId>;
    }
}

/// The generic owner keeps Body and Column source families distinct.
/// Its constructor and field remain private.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2RelationSourceView, BookV2BodyDisplay, BookV2ColumnDisplay};
/// fn body<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(view: BookV2RelationSourceView<'v, BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>>) -> BookV2RelationSourceView<'v, BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>> { view }
/// ```
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2RelationSourceView, BookV2BodyDisplay};
/// fn forge<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(display: &'v BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>) -> BookV2RelationSourceView<'v, BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>> { BookV2RelationSourceView { display } }
/// ```
pub struct BookV2RelationSourceView<'v, D: relation_sealed::Facts> {
    display: &'v D,
}
impl<'v, D: relation_sealed::Facts> BookV2RelationSourceView<'v, D> {
    pub fn tables(&self) -> &'v [ProductionTable] {
        self.display.relation_tables()
    }
    pub fn lists(&self) -> &'v [ProductionList] {
        self.display.relation_lists()
    }
    pub fn definition_count(&self) -> usize {
        self.display.relation_definition_count()
    }
    pub fn definition_owner(&self, index: usize) -> Option<NodeId> {
        self.display.relation_definition_owner(index)
    }
    pub fn reference_count(&self) -> usize {
        self.display.relation_reference_count()
    }
    pub fn reference(&self, index: usize) -> Option<BookV2RelationReferenceView> {
        self.display.relation_reference(index)
    }
    pub fn unreferenced_definitions(&self) -> usize {
        self.display.relation_unreferenced_definitions()
    }
    pub fn has_selected_demand(&self) -> bool {
        self.display.relation_has_selected_demand()
    }
    pub fn first_reference(&self, definition: usize) -> Option<NodeId> {
        self.display.relation_first_reference(definition)
    }
}

pub trait BookV2RelationDisplay<'a>: BookV2SemanticDisplay<'a> + relation_sealed::Facts {
    fn relation_source(&self) -> BookV2RelationSourceView<'_, Self>
    where
        Self: Sized,
    {
        BookV2RelationSourceView { display: self }
    }
}

macro_rules! authentic_relation_display {
    ($display:ident, $first:ident) => {
        impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> relation_sealed::Facts
            for $display<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
        {
            fn relation_tables(&self) -> &[ProductionTable] {
                self.structure_source().tables()
            }
            fn relation_lists(&self) -> &[ProductionList] {
                self.structure_source().lists()
            }
            fn relation_definition_count(&self) -> usize {
                self.source().source().original_graph().definition_count()
            }
            fn relation_definition_owner(&self, index: usize) -> Option<NodeId> {
                self.source()
                    .source()
                    .original_graph()
                    .definition_owner(index)
            }
            fn relation_reference_count(&self) -> usize {
                self.source().source().original_graph().reference_count()
            }
            fn relation_reference(&self, index: usize) -> Option<BookV2RelationReferenceView> {
                self.source().source().original_graph().reference(index)
            }
            fn relation_unreferenced_definitions(&self) -> usize {
                self.source().source().unreferenced_definitions()
            }
            fn relation_has_selected_demand(&self) -> bool {
                self.source()
                    .source()
                    .stable()
                    .sequence()
                    .pages()
                    .last()
                    .is_some()
            }
            fn relation_first_reference(&self, definition: usize) -> Option<NodeId> {
                self.source()
                    .source()
                    .stable()
                    .sequence()
                    .pages()
                    .last()
                    .and_then(|page| $first(page.next_state().source_state(), definition))
            }
        }
        impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2RelationDisplay<'a>
            for $display<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
        {
        }
    };
}
authentic_relation_display!(BookV2BodyDisplay, body_first_reference);
authentic_relation_display!(BookV2ColumnDisplay, column_first_reference);

fn body_first_reference(
    state: &typaxis_pagination::book_v2::BookV2BodySourceState<'_, '_, '_, '_, '_>,
    definition: usize,
) -> Option<NodeId> {
    state.demand().first_reference(definition)
}
fn column_first_reference(
    state: &typaxis_pagination::book_v2::BookV2ColumnPageState<'_, '_, '_, '_, '_>,
    definition: usize,
) -> Option<NodeId> {
    state.first_reference(definition)
}
