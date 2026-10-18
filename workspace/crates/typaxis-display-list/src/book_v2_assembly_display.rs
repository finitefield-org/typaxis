//! Narrow original metadata and authentic physical-frame facts for PDF assembly.
//! Neither family can expose a prepared Body flow through this view.
use super::*;
use typaxis_syntax::book_v2::{
    select_book_v2_page_master, BookV2PageMasterError, BookV2PageRegionKind,
    BookV2SelectedPageMaster,
};
use typaxis_syntax::StagingDocumentMetadata;

#[derive(Debug)]
pub enum BookV2AssemblyFrameError {
    Mismatch,
    Master(BookV2PageMasterError),
    Work,
}
impl From<BookV2PageMasterError> for BookV2AssemblyFrameError {
    fn from(value: BookV2PageMasterError) -> Self {
        Self::Master(value)
    }
}

#[derive(Clone, Copy)]
pub struct BookV2AssemblyRegionView {
    owner: NodeId,
    page: u32,
    kind: BookV2PageRegionKind,
}
impl BookV2AssemblyRegionView {
    pub fn owner(self) -> NodeId {
        self.owner
    }
    pub fn page_index(self) -> u32 {
        self.page
    }
    pub fn kind(self) -> BookV2PageRegionKind {
        self.kind
    }
}

mod assembly_sealed {
    use super::*;
    pub trait Facts {
        fn metadata(&self) -> &StagingDocumentMetadata;
        fn page_reference_values(&self) -> Option<&[(NodeId, u32)]>;
        fn has_page_frames(&self) -> bool;
        fn physical_page_count(&self) -> usize;
        fn select_page_master(
            &self,
            page: usize,
            work: &mut u64,
            maximum: u64,
        ) -> Result<BookV2SelectedPageMaster<'_>, BookV2PageMasterError>;
        fn supports_page_master(&self, selected: BookV2SelectedPageMaster<'_>) -> bool;
        fn validate_page_frame(
            &self,
            page: usize,
            selected: BookV2SelectedPageMaster<'_>,
            work: &mut u64,
            maximum: u64,
        ) -> Result<(), BookV2AssemblyFrameError>;
        fn page_region_count(&self) -> usize;
        fn page_region(&self, index: usize) -> Option<BookV2AssemblyRegionView>;
    }
}

/// Original metadata and genuine selected frame facts cannot authorize another
/// physical display family or expose its internal measurement flow.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2AssemblySourceView, BookV2BodyDisplay, BookV2ColumnDisplay};
/// fn body<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(v: BookV2AssemblySourceView<'v, BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>>)
/// -> BookV2AssemblySourceView<'v, BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>> { v }
/// ```
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2AssemblySourceView, BookV2AssemblyDisplay};
/// fn escape<'a, D: BookV2AssemblyDisplay<'a>>(v: BookV2AssemblySourceView<'_, D>) { let _ = v.display; }
/// ```
pub struct BookV2AssemblySourceView<'v, D: assembly_sealed::Facts> {
    display: &'v D,
}
impl<'v, D: assembly_sealed::Facts> BookV2AssemblySourceView<'v, D> {
    pub fn metadata(&self) -> &'v StagingDocumentMetadata {
        self.display.metadata()
    }
    pub fn page_reference_values(&self) -> Option<&'v [(NodeId, u32)]> {
        self.display.page_reference_values()
    }
    pub fn has_page_frames(&self) -> bool {
        self.display.has_page_frames()
    }
    pub fn page_count(&self) -> usize {
        self.display.physical_page_count()
    }
    pub fn select_page_master(
        &self,
        page: usize,
        work: &mut u64,
        maximum: u64,
    ) -> Result<BookV2SelectedPageMaster<'v>, BookV2PageMasterError> {
        self.display.select_page_master(page, work, maximum)
    }
    pub fn supports_page_master(&self, selected: BookV2SelectedPageMaster<'_>) -> bool {
        self.display.supports_page_master(selected)
    }
    pub fn validate_page_frame(
        &self,
        page: usize,
        selected: BookV2SelectedPageMaster<'_>,
        work: &mut u64,
        maximum: u64,
    ) -> Result<(), BookV2AssemblyFrameError> {
        self.display
            .validate_page_frame(page, selected, work, maximum)
    }
    pub fn page_region_count(&self) -> usize {
        self.display.page_region_count()
    }
    pub fn page_region(&self, index: usize) -> Option<BookV2AssemblyRegionView> {
        self.display.page_region(index)
    }
}

/// The private Facts supertrait seals original metadata/frame access to the two
/// authentic displays. It is separate from the Page-label convergence proof.
pub trait BookV2AssemblyDisplay<'a>: BookV2NavigationDisplay<'a> + assembly_sealed::Facts {
    fn assembly_source(&self) -> BookV2AssemblySourceView<'_, Self>
    where
        Self: Sized,
    {
        BookV2AssemblySourceView { display: self }
    }
}

macro_rules! original_assembly_facts {
    () => {
        fn metadata(&self) -> &StagingDocumentMetadata {
            self.source()
                .original_graph()
                .lines()
                .prepared()
                .source_flow()
                .navigation()
                .metadata()
        }
        fn page_reference_values(&self) -> Option<&[(NodeId, u32)]> {
            self.source()
                .original_graph()
                .lines()
                .prepared()
                .source_flow()
                .page_reference_values()
        }
        fn has_page_frames(&self) -> bool {
            self.source().original_graph().lines().frames().is_some()
        }
        fn physical_page_count(&self) -> usize {
            self.source().source().geometry().pages().len()
        }
        fn select_page_master(
            &self,
            page: usize,
            work: &mut u64,
            maximum: u64,
        ) -> Result<BookV2SelectedPageMaster<'_>, BookV2PageMasterError> {
            let original = self
                .source()
                .original_graph()
                .lines()
                .prepared()
                .source_flow()
                .body();
            let placed = self
                .source()
                .source()
                .geometry()
                .pages()
                .get(page)
                .ok_or(BookV2PageMasterError::Identity)?;
            select_book_v2_page_master(
                original,
                u32::try_from(page).map_err(|_| BookV2PageMasterError::Identity)?,
                placed.selection().named_page(),
                work,
                maximum,
            )
        }
        fn page_region_count(&self) -> usize {
            self.page_regions().len()
        }
        fn page_region(&self, index: usize) -> Option<BookV2AssemblyRegionView> {
            let region = self.page_regions().get(index)?;
            Some(BookV2AssemblyRegionView {
                owner: region.owner(),
                page: region.page_index(),
                kind: region.kind(),
            })
        }
    };
}

impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> assembly_sealed::Facts
    for BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    original_assembly_facts!();
    fn supports_page_master(&self, selected: BookV2SelectedPageMaster<'_>) -> bool {
        selected.advanced().column_layout.is_none()
    }
    fn validate_page_frame(
        &self,
        page: usize,
        selected: BookV2SelectedPageMaster<'_>,
        _work: &mut u64,
        _maximum: u64,
    ) -> Result<(), BookV2AssemblyFrameError> {
        use BookV2AssemblyFrameError as F;
        let lines = self.source().source().flow().lines();
        let flow = lines.prepared().source_flow();
        let frames = lines.frames().ok_or(F::Mismatch)?;
        let placement = self
            .source()
            .source()
            .geometry()
            .pages()
            .get(page)
            .ok_or(F::Mismatch)?
            .selection();
        let page = u32::try_from(page).map_err(|_| F::Mismatch)?;
        if !std::ptr::eq(selected.source(), flow.body()) || selected.page_index() != page {
            return Err(F::Mismatch);
        }
        let body = placement.body_bounds();
        if let Some(plan) = frames.page_plan() {
            if !std::ptr::eq(plan.source(), flow.body())
                || frames.body() != plan.measurement_body()
                || plan.named_page(page, placement.named_page_index())?.body() != body
                || plan
                    .named_page(page, placement.named_page_index())?
                    .footnote()
                    != placement.declared_footnote_region()
            {
                return Err(F::Mismatch);
            }
        } else if body != frames.body() {
            return Err(F::Mismatch);
        }
        let authored = &selected.master().body;
        if (
            body.x().raw(),
            body.y().raw(),
            body.width().get().raw(),
            body.height().get().raw(),
        ) != (authored.x, authored.y, authored.width, authored.height)
        {
            return Err(F::Mismatch);
        }
        Ok(())
    }
}

impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> assembly_sealed::Facts
    for BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    original_assembly_facts!();
    fn supports_page_master(&self, _selected: BookV2SelectedPageMaster<'_>) -> bool {
        true
    }
    fn validate_page_frame(
        &self,
        page: usize,
        selected: BookV2SelectedPageMaster<'_>,
        work: &mut u64,
        maximum: u64,
    ) -> Result<(), BookV2AssemblyFrameError> {
        use BookV2AssemblyFrameError as F;
        let source = self.source().source();
        let original = source.original_graph().lines();
        let frames = original.frames().ok_or(F::Mismatch)?;
        let plan = frames.column_plan().ok_or(F::Mismatch)?;
        let body = original.prepared().source_flow().body();
        let geometry = source.geometry();
        let placed = geometry.pages().get(page).ok_or(F::Mismatch)?;
        let selection = placed.selection();
        let candidate = selection.candidate();
        let actual = candidate.frames();
        let index = u32::try_from(page).map_err(|_| F::Mismatch)?;
        let expected = plan.named_page(index, actual.named_page_index())?;
        // Matching geometry alone cannot identify the selected named master:
        // another authored master may share its body and columns while using
        // different Header/Footer, note or paper metadata. Charge the original
        // selection scan and bind both retained master identities explicitly.
        let authenticated =
            select_book_v2_page_master(body, index, selection.named_page(), work, maximum)?;
        if !std::ptr::eq(authenticated.master(), selected.master())
            || !std::ptr::eq(authenticated.advanced(), selected.advanced())
        {
            return Err(F::Mismatch);
        }
        if !std::ptr::eq(geometry.sequence(), source.stable().sequence())
            || !std::ptr::eq(
                selection,
                source
                    .stable()
                    .sequence()
                    .pages()
                    .get(page)
                    .ok_or(F::Mismatch)?,
            )
            || !std::ptr::eq(plan.source(), body)
            || frames.body() != plan.measurement_body()
            || !std::ptr::eq(selected.source(), body)
            || selected.page_index() != index
            || !std::ptr::eq(actual.source(), body)
            || actual.page_index() != index
            || actual.named_page_index() != expected.named_page_index()
            || actual.body() != expected.body()
            || actual.footnote() != expected.footnote()
            || actual.column_layout() != expected.column_layout()
            || actual.column_count() != expected.column_count()
            || actual.column_layout() != selected.advanced().column_layout
            || placed.columns().len() != usize::from(actual.column_count())
            || candidate.columns().len() != placed.columns().len()
        {
            return Err(F::Mismatch);
        }
        let authored = &selected.master().body;
        let full = actual.body();
        if (
            full.x().raw(),
            full.y().raw(),
            full.width().get().raw(),
            full.height().get().raw(),
        ) != (authored.x, authored.y, authored.width, authored.height)
        {
            return Err(F::Mismatch);
        }
        if plan.name(actual.named_page_index().unwrap_or(usize::MAX)) != selection.named_page() {
            return Err(F::Mismatch);
        }
        for (column, (placed, candidate)) in
            placed.columns().iter().zip(candidate.columns()).enumerate()
        {
            *work = work
                .checked_add(1)
                .filter(|n| *n <= maximum)
                .ok_or(F::Work)?;
            let template = expected.column(u16::try_from(column).map_err(|_| F::Mismatch)?)?;
            let actual = placed.bounds();
            let height = selection
                .balance()
                .map_or(template.height(), |balance| balance.selected_height());
            if actual != candidate.bounds()
                || actual.x() != template.x()
                || actual.y() != template.y()
                || actual.width() != template.width()
                || actual.height() != height
                || actual.height().get() > template.height().get()
            {
                return Err(F::Mismatch);
            }
        }
        Ok(())
    }
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2AssemblyDisplay<'a>
    for BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2AssemblyDisplay<'a>
    for BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
}
