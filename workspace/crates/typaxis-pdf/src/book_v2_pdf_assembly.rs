//! Inspectable PDF from the actual selected body, source pages and structure.
//! This owner grants no public-profile or PDF/UA conformance authority.
use super::*;
use crate::{font_encoding, text_encoding};
use std::sync::atomic::{AtomicU64, Ordering};
use typaxis_core::EngineIdentity;
use typaxis_display_list::book_v2::{BookV2AssemblyDisplay, BookV2AssemblyFrameError};
use typaxis_syntax::book_v2::{BookV2PageMasterError, BookV2SelectedPageMaster};

pub const BOOK_V2_PDF_ASSEMBLY_ALGORITHM: &str = "typaxis.book-2-pdf-assembly/1";
static NEXT_ASSEMBLY_ATTEMPT: AtomicU64 = AtomicU64::new(1);

/// Opaque identity of one assembly builder attempt. Copying this comparison
/// token cannot construct a builder or reset its replacement credit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2PdfAssemblyAttemptIdentity(u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2PdfAssemblyError {
    Resource(E),
    UnsupportedPageMaster,
    PageFrameMismatch,
    PageMaster(BookV2PageMasterError),
}
use BookV2PdfAssemblyError as AE;
impl From<E> for AE {
    fn from(e: E) -> Self {
        Self::Resource(e)
    }
}
impl From<BookV2AssemblyFrameError> for AE {
    fn from(error: BookV2AssemblyFrameError) -> Self {
        match error {
            BookV2AssemblyFrameError::Mismatch => Self::PageFrameMismatch,
            BookV2AssemblyFrameError::Master(error) => Self::PageMaster(error),
            BookV2AssemblyFrameError::Work => Self::Resource(E::Work),
        }
    }
}
impl std::fmt::Display for AE {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 PDF assembly: {self:?}")
    }
}
impl std::error::Error for AE {}
/// Actual target observations for one candidate PDF. Matching labels alone do
/// not certify a stable reflow; the caller must compare complete layout passes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2PageReferenceObservation {
    owner: NodeId,
    candidate: u32,
    target: Option<u32>,
    placed: bool,
}
impl BookV2PageReferenceObservation {
    pub fn owner(self) -> NodeId {
        self.owner
    }
    pub fn candidate_page(self) -> u32 {
        self.candidate
    }
    pub fn target_page(self) -> Option<u32> {
        self.target
    }
    pub fn is_placed(self) -> bool {
        self.placed
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ObjectRange {
    offset: usize,
    length: usize,
}
#[derive(Clone, Copy)]
struct Plan {
    first: u32,
    page_base: u32,
    node_base: u32,
    annotation_base: u32,
    outline_base: u32,
    dests: Option<u32>,
    outlines: Option<u32>,
    next: u32,
}
impl Plan {
    fn page(self, i: usize) -> u32 {
        self.page_base + i as u32 * 2
    }
    fn content(self, i: usize) -> u32 {
        self.page(i) + 1
    }
    fn node(self, i: usize) -> u32 {
        self.node_base + i as u32
    }
    fn annotation(self, i: usize) -> u32 {
        self.annotation_base + i as u32
    }
    fn outline(self, i: usize) -> u32 {
        self.outline_base + i as u32
    }
    fn catalog(self) -> u32 {
        self.first
    }
    fn pages(self) -> u32 {
        self.first + 1
    }
    fn info(self) -> u32 {
        self.first + 2
    }
    fn metadata(self) -> u32 {
        self.first + 3
    }
    fn resources(self) -> u32 {
        self.first + 4
    }
    fn structure(self) -> u32 {
        self.first + 5
    }
    fn parents(self) -> u32 {
        self.first + 6
    }
    fn ids(self) -> u32 {
        self.first + 7
    }
}

pub type BookV2PdfAssembly<
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> = BookV2PdfAssemblyData<
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'a,
    BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    BookV2BodyPaintIndex,
>;
/// The Column assembly retains its source and physical paint family.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2PdfAssembly,BookV2ColumnPdfAssembly};
/// fn body<'e,'w,'u,'h,'n,'m,'k,'j,'r,'i,'t,'o,'z,'y,'x,'c,'v,'d,'g,'q,'b,'f,'s,'p,'a>(value: BookV2ColumnPdfAssembly<'e,'w,'u,'h,'n,'m,'k,'j,'r,'i,'t,'o,'z,'y,'x,'c,'v,'d,'g,'q,'b,'f,'s,'p,'a>)
/// -> BookV2PdfAssembly<'e,'w,'u,'h,'n,'m,'k,'j,'r,'i,'t,'o,'z,'y,'x,'c,'v,'d,'g,'q,'b,'f,'s,'p,'a> { value }
/// ```
pub type BookV2ColumnPdfAssembly<
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> = BookV2PdfAssemblyData<
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'a,
    BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    BookV2ColumnPaintIndex,
>;

pub struct BookV2PdfAssemblyData<
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'a,
    D: BookV2AssemblyDisplay<'a>,
    P: Copy = <D as BookV2ResourceDisplay<'a>>::Paint,
> {
    source: &'e BookV2NavigationGeometryData<
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D,
        P,
    >,
    bytes: Vec<u8>,
    objects: Vec<ObjectRange>,
    references: Vec<BookV2PageReferenceObservation>,
    plan: Plan,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<
        'e,
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D: BookV2AssemblyDisplay<'a, Paint = P>,
        P: Copy + Eq,
    >
    BookV2PdfAssemblyData<
        'e,
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D,
        P,
    >
{
    pub fn navigation(
        &self,
    ) -> &'e BookV2NavigationGeometryData<
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D,
        P,
    > {
        self.source
    }
    pub fn page_references(&self) -> &[BookV2PageReferenceObservation] {
        &self.references
    }
    /// Unplaced references to unplaced targets have no rendered page label to
    /// resolve. Their candidate stays explicit, never an invented destination.
    pub fn page_reference_labels_match(&self) -> bool {
        self.references.iter().all(|r| match r.target {
            Some(page) => page == r.candidate,
            None => !r.placed,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn object_bytes(&self, id: u32) -> Option<&[u8]> {
        let r = self.objects.get(id as usize)?;
        if r.length == 0 {
            None
        } else {
            self.bytes.get(r.offset..r.offset + r.length)
        }
    }
    pub fn first_object(&self) -> u32 {
        self.plan.first
    }
    pub fn next_object(&self) -> u32 {
        self.plan.next
    }
    /// The actual byte range of an occupied object. Free IDs have no range.
    pub fn object_range(&self, id: u32) -> Option<std::ops::Range<usize>> {
        let range = self.objects.get(id as usize)?;
        (range.length != 0).then_some(range.offset..range.offset + range.length)
    }
    pub fn catalog_object(&self) -> u32 {
        self.plan.catalog()
    }
    pub fn page_object(&self, page: usize) -> Option<u32> {
        (page < self.source.source().source().source().pages().len()).then(|| self.plan.page(page))
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn record_charge(&self) -> u64 {
        self.budget.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.budget.spool
    }
    pub fn output_charge(&self) -> u64 {
        self.budget.output
    }
    pub fn work_steps(&self) -> u64 {
        self.budget.work
    }
}
pub type BookV2PdfAssemblyBuilder<
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> = BookV2PdfAssemblyBuilderData<
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'a,
    BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    BookV2BodyPaintIndex,
>;
/// The Column assembly retains its source and physical paint family.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2PdfAssemblyBuilder,BookV2ColumnPdfAssemblyBuilder};
/// fn body<'e,'w,'u,'h,'n,'m,'k,'j,'r,'i,'t,'o,'z,'y,'x,'c,'v,'d,'g,'q,'b,'f,'s,'p,'a>(value: BookV2ColumnPdfAssemblyBuilder<'e,'w,'u,'h,'n,'m,'k,'j,'r,'i,'t,'o,'z,'y,'x,'c,'v,'d,'g,'q,'b,'f,'s,'p,'a>)
/// -> BookV2PdfAssemblyBuilder<'e,'w,'u,'h,'n,'m,'k,'j,'r,'i,'t,'o,'z,'y,'x,'c,'v,'d,'g,'q,'b,'f,'s,'p,'a> { value }
/// ```
pub type BookV2ColumnPdfAssemblyBuilder<
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> = BookV2PdfAssemblyBuilderData<
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'a,
    BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    BookV2ColumnPaintIndex,
>;

/// A scoped continuation of one driver-owned assembly builder.
///
/// The constructor and mutable builder are private. Each loan has an invariant
/// lifetime, so independent loans cannot be swapped or escape their callback.
/// The owning builder remains outside the loan and keeps every accepted charge.
///
/// ```
/// use typaxis_pdf::book_v2::{BookV2ColumnPdfAssemblyBuilder as Builder, BookV2ColumnPdfAssembly as Pdf, BookV2PdfAssemblyError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type P<'a> = Pdf<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn rebuild<'a>(builder: &mut B<'a>) -> Result<P<'a>, BookV2PdfAssemblyError> {
///     builder.with_continuation(|loan| {
///         let _ = (loan.record_charge(), loan.spool_charge(), loan.output_charge(), loan.work_steps());
///         let _ = (loan.navigation(), loan.reservation_sizes());
///         loan.build()
///     })
/// }
/// ```
/// A raw builder cannot replace the borrowed continuation.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2ColumnPdfAssemblyBuilder as Builder, BookV2ColumnPdfAssembly as Pdf, BookV2PdfAssemblyError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn replace<'a>(builder: &mut B<'a>, replacement: B<'a>) {
///     builder.with_continuation(|loan| *loan = replacement);
/// }
/// ```
/// The private raw builder cannot be extracted.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2ColumnPdfAssemblyBuilder as Builder, BookV2ColumnPdfAssembly as Pdf, BookV2PdfAssemblyError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn raw<'a>(builder: &mut B<'a>) {
///     builder.with_continuation(|loan| { let _ = &mut loan.builder; });
/// }
/// ```
/// A callback cannot return its continuation loan.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2ColumnPdfAssemblyBuilder as Builder, BookV2ColumnPdfAssemblyContinuation as Continuation};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type C<'a> = Continuation<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn escape<'a>(builder: &'a mut B<'a>) -> &'a mut C<'a> {
///     builder.with_continuation(|loan| loan)
/// }
/// ```
/// Two callback loans have different invariant lifetimes.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2ColumnPdfAssemblyBuilder as Builder, BookV2ColumnPdfAssembly as Pdf, BookV2PdfAssemblyError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn swap<'a>(left: &mut B<'a>, right: &mut B<'a>) {
///     left.with_continuation(|outer| {
///         right.with_continuation(|inner| std::mem::swap(outer, inner));
///     });
/// }
/// ```
/// Nested callbacks may inspect each independent builder.
///
/// ```
/// use typaxis_pdf::book_v2::{BookV2ColumnPdfAssemblyBuilder as Builder, BookV2ColumnPdfAssembly as Pdf, BookV2PdfAssemblyError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn inspect<'a>(left: &mut B<'a>, right: &mut B<'a>) -> u64 {
///     left.with_continuation(|outer| {
///         right.with_continuation(|inner| outer.record_charge() + inner.record_charge())
///     })
/// }
/// ```
pub struct BookV2PdfAssemblyContinuationData<
    'loan,
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'a,
    D: BookV2AssemblyDisplay<'a>,
    P: Copy = <D as BookV2ResourceDisplay<'a>>::Paint,
> {
    builder: &'loan mut BookV2PdfAssemblyBuilderData<
        'e,
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D,
        P,
    >,
    invariant: std::marker::PhantomData<fn(&'loan ()) -> &'loan ()>,
}
pub type BookV2PdfAssemblyContinuation<
    'loan,
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> = BookV2PdfAssemblyContinuationData<
    'loan,
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'a,
    BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    BookV2BodyPaintIndex,
>;
pub type BookV2ColumnPdfAssemblyContinuation<
    'loan,
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> = BookV2PdfAssemblyContinuationData<
    'loan,
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'a,
    BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    BookV2ColumnPaintIndex,
>;
impl<
        'loan,
        'e,
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D: BookV2AssemblyDisplay<'a, Paint = P>,
        P: Copy + Eq,
    >
    BookV2PdfAssemblyContinuationData<
        'loan,
        'e,
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D,
        P,
    >
{
    pub fn navigation(
        &self,
    ) -> &'e BookV2NavigationGeometryData<
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D,
        P,
    > {
        self.builder.navigation()
    }
    pub fn record_charge(&self) -> u64 {
        self.builder.record_charge()
    }
    pub fn spool_charge(&self) -> u64 {
        self.builder.spool_charge()
    }
    pub fn output_charge(&self) -> u64 {
        self.builder.output_charge()
    }
    pub fn work_steps(&self) -> u64 {
        self.builder.work_steps()
    }
    pub fn reservation_sizes(&self) -> [usize; 4] {
        self.builder.reservation_sizes()
    }
    pub fn build(
        &mut self,
    ) -> Result<
        BookV2PdfAssemblyData<
            'e,
            'w,
            'u,
            'h,
            'n,
            'm,
            'k,
            'j,
            'r,
            'i,
            't,
            'o,
            'z,
            'y,
            'x,
            'c,
            'v,
            'a,
            D,
            P,
        >,
        AE,
    > {
        self.builder.build()
    }
}

pub struct BookV2PdfAssemblyBuilderData<
    'e,
    'w,
    'u,
    'h,
    'n,
    'm,
    'k,
    'j,
    'r,
    'i,
    't,
    'o,
    'z,
    'y,
    'x,
    'c,
    'v,
    'a,
    D: BookV2AssemblyDisplay<'a>,
    P: Copy = <D as BookV2ResourceDisplay<'a>>::Paint,
> {
    source: &'e BookV2NavigationGeometryData<
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D,
        P,
    >,
    budget: Budget,
    max_objects: u32,
    credit_available: bool,
    attempt: BookV2PdfAssemblyAttemptIdentity,
}
impl<
        'e,
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D: BookV2AssemblyDisplay<'a, Paint = P>,
        P: Copy + Eq,
    >
    BookV2PdfAssemblyBuilderData<
        'e,
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D,
        P,
    >
{
    /// Loan this builder through a nonconstructible, invariant continuation.
    /// The callback cannot replace the raw builder or retain the loan.
    pub fn with_continuation<R>(
        &mut self,
        inspect: impl for<'loan> FnOnce(
            &mut BookV2PdfAssemblyContinuationData<
                'loan,
                'e,
                'w,
                'u,
                'h,
                'n,
                'm,
                'k,
                'j,
                'r,
                'i,
                't,
                'o,
                'z,
                'y,
                'x,
                'c,
                'v,
                'a,
                D,
                P,
            >,
        ) -> R,
    ) -> R {
        let mut continuation = BookV2PdfAssemblyContinuationData {
            builder: self,
            invariant: std::marker::PhantomData,
        };
        inspect(&mut continuation)
    }

    /// The exact typed Navigation source of this assembly attempt.
    pub fn navigation(
        &self,
    ) -> &'e BookV2NavigationGeometryData<
        'w,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'a,
        D,
        P,
    > {
        self.source
    }
    pub fn new(
        source: &'e BookV2NavigationGeometryData<
            'w,
            'u,
            'h,
            'n,
            'm,
            'k,
            'j,
            'r,
            'i,
            't,
            'o,
            'z,
            'y,
            'x,
            'c,
            'v,
            'a,
            D,
            P,
        >,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, AE> {
        Self::new_counted(
            source,
            limits,
            max_work,
            prior_records,
            prior_spool,
            prior_output,
            prior_work,
            &mut 0,
            &mut 0,
            &mut 0,
            &mut 0,
        )
    }
    /// Preserve every accepted constructor reservation, including a later work
    /// refusal, in the supplied cumulative observations.
    #[allow(clippy::too_many_arguments)]
    pub fn new_counted(
        source: &'e BookV2NavigationGeometryData<
            'w,
            'u,
            'h,
            'n,
            'm,
            'k,
            'j,
            'r,
            'i,
            't,
            'o,
            'z,
            'y,
            'x,
            'c,
            'v,
            'a,
            D,
            P,
        >,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
        observed_records: &mut u64,
        observed_spool: &mut u64,
        observed_output: &mut u64,
        observed_work: &mut u64,
    ) -> Result<Self, AE> {
        *observed_records = prior_records.max(source.record_charge());
        *observed_spool = prior_spool.max(source.spool_charge());
        *observed_output = prior_output.max(source.output_charge());
        *observed_work = prior_work.max(source.work_steps());
        let display = source.source().source().source().source().display();
        display
            .verify_resource_selection(display.admitted(), limits)
            .map_err(|_| E::Identity)?;
        let mut budget = source.budget;
        budget.max_work = max_work;
        budget.records = budget.records.max(prior_records);
        budget.spool = budget.spool.max(prior_spool);
        budget.output = budget.output.max(prior_output);
        budget.work = budget.work.max(prior_work);
        let reserved = budget.reserve(1, 0, 0);
        *observed_records = budget.records;
        *observed_spool = budget.spool;
        *observed_output = budget.output;
        *observed_work = budget.work;
        reserved?;
        if budget.work > max_work {
            return Err(E::Work.into());
        }
        let attempt = BookV2PdfAssemblyAttemptIdentity(
            NEXT_ASSEMBLY_ATTEMPT
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
                .map_err(|_| E::Identity)?,
        );
        Ok(Self {
            source,
            budget,
            max_objects: limits.base().get().max_pdf_objects,
            credit_available: true,
            attempt,
        })
    }
    /// Identifies this exact builder attempt across moves and mutable loans.
    /// It is not serialized, hashed, or transferable to a fresh builder.
    pub fn attempt_identity(&self) -> BookV2PdfAssemblyAttemptIdentity {
        self.attempt
    }
    pub fn record_charge(&self) -> u64 {
        self.budget.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.budget.spool
    }
    pub fn output_charge(&self) -> u64 {
        self.budget.output
    }
    pub fn work_steps(&self) -> u64 {
        self.budget.work
    }
    /// Actual private reservation units: selected page master, object range,
    /// page-reference observation and optional structure-link index, in bytes.
    pub fn reservation_sizes(&self) -> [usize; 4] {
        [
            std::mem::size_of::<BookV2SelectedPageMaster<'_>>(),
            std::mem::size_of::<ObjectRange>(),
            std::mem::size_of::<BookV2PageReferenceObservation>(),
            std::mem::size_of::<Option<usize>>(),
        ]
    }
    pub fn build(
        &mut self,
    ) -> Result<
        BookV2PdfAssemblyData<
            'e,
            'w,
            'u,
            'h,
            'n,
            'm,
            'k,
            'j,
            'r,
            'i,
            't,
            'o,
            'z,
            'y,
            'x,
            'c,
            'v,
            'a,
            D,
            P,
        >,
        AE,
    > {
        let source = self.source;
        let relations = source.source();
        let registry = relations.source();
        let marked = registry.source();
        let scopes = marked.source();
        let display = scopes.display();
        let assembly = display.assembly_source();
        self.budget.step(1)?;
        if !assembly.has_page_frames() {
            return Err(AE::PageFrameMismatch);
        }
        if assembly.page_count() != marked.pages().len() {
            return Err(AE::PageFrameMismatch);
        }
        let pc = marked.pages().len();
        let mut page_masters = reserved::<BookV2SelectedPageMaster<'_>>(pc, &mut self.budget)?;
        let mut region_cursor = 0usize;
        for page in 0..pc {
            u32::try_from(page).map_err(|_| E::Identity)?;
            let selected = assembly
                .select_page_master(page, &mut self.budget.work, self.budget.max_work)
                .map_err(AE::PageMaster)?;
            let advanced = selected.advanced();
            if !assembly.supports_page_master(selected) {
                return Err(AE::UnsupportedPageMaster);
            }
            for (kind, content) in [
                (
                    typaxis_syntax::book_v2::BookV2PageRegionKind::Header,
                    advanced.header_content.as_ref(),
                ),
                (
                    typaxis_syntax::book_v2::BookV2PageRegionKind::Footer,
                    advanced.footer_content.as_ref(),
                ),
            ] {
                if let Some(content) = content {
                    self.budget.step(1)?;
                    let region = assembly
                        .page_region(region_cursor)
                        .ok_or(AE::UnsupportedPageMaster)?;
                    if region.page_index() != page as u32
                        || region.kind() != kind
                        || region.owner().get() != content.node_id
                    {
                        return Err(AE::PageFrameMismatch);
                    }
                    region_cursor += 1;
                }
            }
            assembly.validate_page_frame(
                page,
                selected,
                &mut self.budget.work,
                self.budget.max_work,
            )?;
            page_masters.push(selected);
        }
        if region_cursor != assembly.page_region_count() {
            return Err(AE::PageFrameMismatch);
        }
        let n = registry.nodes().len();
        let ac = source.rectangles().len();
        let oc = source.outline().len();
        let mut names = reserved::<usize>(source.destinations().len(), &mut self.budget)?;
        for (i, d) in source.destinations().iter().enumerate() {
            self.budget.step(1)?;
            if matches!(d.kind(), BookV2DestinationKind::Anchor(_)) && d.position().is_some() {
                names.push(i);
            }
        }
        // The source navigation owns a strictly ordered anchor map. Filtering
        // unplaced anchors preserves the UTF-8 byte ordering of PDF string keys.
        for pair in names.windows(2) {
            let name = |i: usize| match source.destinations()[i].kind() {
                BookV2DestinationKind::Anchor(s) => s,
                _ => unreachable!(),
            };
            self.budget
                .step(1 + name(pair[0]).len().min(name(pair[1]).len()))?;
            if name(pair[0]).as_bytes() >= name(pair[1]).as_bytes() {
                return Err(E::Identity.into());
            }
        }
        let mut node_links = reserved::<Option<usize>>(n, &mut self.budget)?;
        self.budget.step(n)?;
        node_links.resize(n, None);
        for (i, link) in source.links().iter().enumerate() {
            self.budget.step(1)?;
            let slot = node_links
                .get_mut(link.structure_node())
                .ok_or(E::Identity)?;
            if slot.replace(i).is_some() {
                return Err(E::Identity.into());
            }
        }
        let values = assembly.page_reference_values().unwrap_or(&[]);
        let mut reference_count = 0usize;
        let navigation = display.navigation_source();
        for paragraph in navigation.paragraphs() {
            self.budget.step(1)?;
            for site in paragraph.items() {
                self.budget.step(1)?;
                if matches!(
                    site.reference(),
                    Some(typaxis_syntax::ProductionInlineReference::Anchor {
                        format: typaxis_syntax::ProductionReferenceFormat::Page,
                        ..
                    })
                ) {
                    reference_count = reference_count.checked_add(1).ok_or(E::Records)?;
                }
            }
        }
        if reference_count != values.len() {
            return Err(E::Identity.into());
        }
        let mut references =
            reserved::<BookV2PageReferenceObservation>(reference_count, &mut self.budget)?;
        for &(owner, candidate) in values {
            self.budget.step(1)?;
            let node = find(
                &registry.lookup,
                Key::new(owner, Slot::ReferenceLink),
                &mut self.budget,
            )?
            .ok_or(E::Identity)?;
            let link = &source.links()[node_links[node].ok_or(E::Identity)?];
            let BookV2NavigationTarget::Destination(destination) = link.target() else {
                return Err(E::Identity.into());
            };
            let target = source.destinations()[destination]
                .position()
                .map(|p| p.page_index().checked_add(1).ok_or(E::Identity))
                .transpose()?;
            let placed = link.first_rectangle().is_some();
            if placed && target.is_none() {
                return Err(E::Identity.into());
            }
            references.push(BookV2PageReferenceObservation {
                owner,
                candidate,
                target,
                placed,
            });
        }
        let first = marked.next_object();
        let mut next = u64::from(first);
        let mut allocate = |count: usize| -> Result<u32, E> {
            let id = u32::try_from(next).map_err(|_| E::Objects)?;
            next = next.checked_add(count as u64).ok_or(E::Objects)?;
            if next > u64::from(u32::MAX) || next - 1 > u64::from(self.max_objects) {
                return Err(E::Objects);
            }
            Ok(id)
        };
        allocate(8)?;
        let page_base = allocate(pc.checked_mul(2).ok_or(E::Objects)?)?;
        let node_base = allocate(n)?;
        let annotation_base = allocate(ac)?;
        let outline_base = allocate(oc)?;
        let dests = if names.is_empty() {
            None
        } else {
            Some(allocate(1)?)
        };
        let outlines = if oc == 0 { None } else { Some(allocate(1)?) };
        let plan = Plan {
            first,
            page_base,
            node_base,
            annotation_base,
            outline_base,
            dests,
            outlines,
            next: next as u32,
        };
        let mut objects = reserved::<ObjectRange>(plan.next as usize, &mut self.budget)?;
        self.budget.step(plan.next as usize)?;
        objects.resize(plan.next as usize, ObjectRange::default());
        let fonts = marked.text().source();
        let images = scopes.source().source();
        let credit = if self.credit_available {
            fonts
                .bytes()
                .len()
                .checked_add(images.bytes().len())
                .and_then(|n| n.checked_add(marked.byte_length()))
                .ok_or(E::Output)? as u64
        } else {
            0
        };
        let retained = self.budget.output.checked_sub(credit).ok_or(E::Identity)?;
        let output_limit = self.budget.max_output.saturating_sub(retained);
        let spool_limit = self.budget.max_spool.saturating_sub(self.budget.spool);
        let mut counter = Encoder {
            budget: &mut self.budget,
            bytes: None,
            length: 0,
            output_limit,
            spool_limit,
        };
        encode(
            source,
            plan,
            &names,
            &node_links,
            &page_masters,
            &mut objects,
            &mut counter,
        )?;
        let length = counter.length;
        // Replacement credit can be spent once, after complete measurement succeeds.
        self.budget
            .reserve(0, length, length.saturating_sub(credit as usize))?;
        if (length as u64) < credit {
            self.budget.output -= credit - length as u64;
        }
        self.credit_available = false;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| E::Allocation)?;
        self.budget.step(objects.len())?;
        objects.fill(ObjectRange::default());
        let mut out = Encoder {
            budget: &mut self.budget,
            bytes: Some(&mut bytes),
            length: 0,
            output_limit: length as u64,
            spool_limit: length as u64,
        };
        encode(
            source,
            plan,
            &names,
            &node_links,
            &page_masters,
            &mut objects,
            &mut out,
        )?;
        if out.length != length {
            return Err(E::Identity.into());
        }
        let mut fp = self.budget.fold(
            source.fingerprint(),
            BOOK_V2_PDF_ASSEMBLY_ALGORITHM.as_bytes(),
        )?;
        for chunk in bytes.chunks(96) {
            fp = self.budget.fold(fp, chunk)?;
        }
        for reference in &references {
            let mut bytes = [0u8; 13];
            bytes[..4].copy_from_slice(&reference.owner.get().to_be_bytes());
            bytes[4..8].copy_from_slice(&reference.candidate.to_be_bytes());
            bytes[8..12].copy_from_slice(&reference.target.unwrap_or(0).to_be_bytes());
            bytes[12] = u8::from(reference.placed);
            fp = self.budget.fold(fp, &bytes)?;
        }
        Ok(BookV2PdfAssemblyData {
            source,
            bytes,
            objects,
            references,
            plan,
            fingerprint: fp,
            budget: self.budget,
        })
    }
}
fn encode<'a, D: BookV2AssemblyDisplay<'a, Paint = P>, P: Copy + Eq>(
    source: &BookV2NavigationGeometryData<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        'a,
        D,
        P,
    >,
    plan: Plan,
    names: &[usize],
    node_links: &[Option<usize>],
    page_masters: &[BookV2SelectedPageMaster<'_>],
    objects: &mut [ObjectRange],
    out: &mut Encoder<'_>,
) -> Result<(), E> {
    let relations = source.source();
    let registry = relations.source();
    let marked = registry.source();
    let scopes = marked.source();
    let display = scopes.display();
    let assembly = display.assembly_source();
    let metadata = assembly.metadata();
    let navigation = display.navigation_source();
    let fonts = marked.text().source();
    let images = scopes.source().source();
    let pc = marked.pages().len();
    out.extend(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n")?;
    for object in fonts.objects() {
        raw_object(
            out,
            objects,
            object.id().get(),
            &fonts.bytes()[object.byte_range()],
        )?;
    }
    for object in images.objects() {
        raw_object(
            out,
            objects,
            object.id().get(),
            &images.bytes()[object.byte_range()],
        )?;
    }
    for object in marked.anchor_objects() {
        raw_object(
            out,
            objects,
            object.id().get(),
            &marked.anchor_object_bytes()[object.byte_range()],
        )?;
    }
    object(out, objects, plan.catalog(), |out| {
        out.extend(b"<< /Type /Catalog /Pages ")?;
        reference(out, plan.pages())?;
        out.extend(b" /Lang ")?;
        text(out, registry.nodes()[0].source().language())?;
        out.extend(b" /MarkInfo << /Marked true >> /ViewerPreferences << /DisplayDocTitle true >> /StructTreeRoot ")?;
        reference(out, plan.structure())?;
        out.extend(b" /Metadata ")?;
        reference(out, plan.metadata())?;
        if let Some(id) = plan.dests {
            out.extend(b" /Names << /Dests ")?;
            reference(out, id)?;
            out.extend(b" >>")?;
        }
        if let Some(id) = plan.outlines {
            out.extend(b" /Outlines ")?;
            reference(out, id)?;
        }
        out.extend(b" >>")
    })?;
    object(out, objects, plan.pages(), |out| {
        out.extend(b"<< /Type /Pages /Count ")?;
        out.unsigned(pc as u64)?;
        out.extend(b" /Kids [")?;
        for i in 0..pc {
            reference(out, plan.page(i))?;
        }
        out.extend(b"] >>")
    })?;
    object(out, objects, plan.info(), |out| info(out, metadata))?;
    object(out, objects, plan.metadata(), |out| {
        stream(out, b" /Type /Metadata /Subtype /XML", |out| {
            let mut adapter = XmlSink {
                output: out,
                error: None,
            };
            let result = crate::tagged_pdf::write_book_xmp_fields(
                &mut adapter,
                metadata,
                registry.nodes()[0].source().language(),
                &EngineIdentity::compiled(),
                false,
            );
            result.map_err(|_| adapter.error.unwrap_or(E::Identity))
        })
    })?;
    object(out, objects, plan.resources(), |out| {
        out.extend(b"<< /Font <<")?;
        for (i, font) in fonts.source().source().fonts().iter().enumerate() {
            out.extend(b" /PB")?;
            out.unsigned(font.font_instance_id().get() as u64)?;
            out.push(b' ')?;
            reference(out, fonts.font_object(i).ok_or(E::Identity)?.get())?;
        }
        if let Some(id) = marked.anchor_font_object() {
            out.extend(b" /BMA ")?;
            reference(out, id.get())?;
        }
        out.extend(b" >> /XObject <<")?;
        for i in 0..images.source().source().source().images().len() {
            out.extend(b" /BI")?;
            out.unsigned(i as u64)?;
            out.push(b' ')?;
            reference(out, images.resource_object(i).ok_or(E::Identity)?.get())?;
        }
        out.extend(b" >> >>")
    })?;
    for (pi, page) in marked.pages().iter().enumerate() {
        let selected = page_masters.get(pi).ok_or(E::Identity)?;
        let master = selected.master();
        let advanced = selected.advanced();
        object(out, objects, plan.page(pi), |out| {
            out.extend(b"<< /Type /Page /Parent ")?;
            reference(out, plan.pages())?;
            out.extend(b" /MediaBox [0 0 ")?;
            number(out, master.width)?;
            number(out, master.height)?;
            out.extend(b"] /TrimBox [")?;
            number(out, advanced.trim.x)?;
            number(out, master.height - advanced.trim.y - advanced.trim.height)?;
            number(out, advanced.trim.x + advanced.trim.width)?;
            number(out, master.height - advanced.trim.y)?;
            out.extend(b"] /Resources ")?;
            reference(out, plan.resources())?;
            out.extend(b" /Contents ")?;
            reference(out, plan.content(pi))?;
            out.extend(b" /StructParents ")?;
            out.unsigned(pi as u64)?;
            out.extend(b" /Tabs /S")?;
            let range = source.pages.get(pi).ok_or(E::Identity)?.clone();
            if !range.is_empty() {
                out.extend(b" /Annots [")?;
                for i in range {
                    reference(out, plan.annotation(i))?;
                }
                out.push(b']')?;
            }
            out.extend(b" >>")
        })?;
        object(out, objects, plan.content(pi), |out| {
            stream(out, b"", |out| {
                out.extend(b"q\n1 0 0 -1 0 ")?;
                number(out, master.height)?;
                out.extend(b"cm\n")?;
                out.extend(marked.page_bytes(pi).ok_or(E::Identity)?)?;
                out.extend(b"Q\n")
            })
        })?;
        if page.page_index() as usize != pi {
            return Err(E::Identity);
        }
    }
    object(out, objects, plan.structure(), |out| {
        out.extend(b"<< /Type /StructTreeRoot /K [")?;
        reference(out, plan.node(0))?;
        out.extend(b"] /ParentTree ")?;
        reference(out, plan.parents())?;
        out.extend(b" /ParentTreeNextKey ")?;
        out.unsigned((pc + source.rectangles().len()) as u64)?;
        out.extend(b" /IDTree ")?;
        reference(out, plan.ids())?;
        out.extend(b" >>")
    })?;
    object(out, objects, plan.parents(), |out| {
        out.extend(b"<< /Nums [")?;
        for (pi, page) in marked.pages().iter().enumerate() {
            out.unsigned(pi as u64)?;
            out.extend(b" [")?;
            let mut mcid = 0;
            for gi in page.groups() {
                out.budget.step(1)?;
                if let Some(b) = registry.for_group(gi) {
                    if b.mcid() != mcid {
                        return Err(E::Identity);
                    }
                    reference(out, plan.node(b.node_index()))?;
                    mcid += 1;
                }
            }
            out.extend(b"] ")?;
        }
        for (i, r) in source.rectangles().iter().enumerate() {
            out.unsigned((pc + i) as u64)?;
            out.push(b' ')?;
            reference(
                out,
                plan.node(source.links()[r.link_index()].structure_node()),
            )?;
        }
        out.extend(b"] >>")
    })?;
    object(out, objects, plan.ids(), |out| {
        out.extend(b"<< /Names [")?;
        for i in 0..registry.nodes().len() {
            structure_id(out, i)?;
            out.push(b' ')?;
            reference(out, plan.node(i))?;
        }
        out.extend(b"] >>")
    })?;
    for (i, node) in registry.nodes().iter().enumerate() {
        object(out, objects, plan.node(i), |out| {
            let s = node.source();
            let rel = &relations.nodes()[i];
            out.extend(b"<< /Type /StructElem /S /")?;
            out.extend(s.pdf_role().as_bytes())?;
            out.extend(b" /P ")?;
            reference(out, rel.parent().map_or(plan.structure(), |p| plan.node(p)))?;
            out.extend(b" /Lang ")?;
            text(out, s.language())?;
            out.extend(b" /ID ")?;
            structure_id(out, i)?;
            if let Some(alt) = s.alternative() {
                out.extend(b" /Alt ")?;
                text(out, alt)?;
            }
            if let Some(kind) = s.semantic_kind() {
                out.extend(b" /TypaxisSourceKind ")?;
                text(out, kind)?;
            }
            if let Some(list) = rel.list_numbering() {
                out.extend(b" /A << /O /List /ListNumbering /")?;
                out.extend(match list {
                    BookV2ListNumbering::Decimal => b"Decimal",
                    BookV2ListNumbering::Disc => b"Disc",
                })?;
                out.extend(b" >>")?;
            }
            if let Some(cell) = s.table_cell() {
                out.extend(b" /A << /O /Table")?;
                if cell.header {
                    out.extend(b" /Scope /Column")?;
                }
                if cell.rowspan > 1 {
                    out.extend(b" /RowSpan ")?;
                    out.unsigned(cell.rowspan as u64)?;
                }
                if cell.colspan > 1 {
                    out.extend(b" /ColSpan ")?;
                    out.unsigned(cell.colspan as u64)?;
                }
                let headers = relations.headers(i).ok_or(E::Identity)?;
                if !headers.is_empty() {
                    out.extend(b" /Headers [")?;
                    for h in headers {
                        structure_id(out, *h)?;
                        out.push(b' ')?;
                    }
                    out.push(b']')?;
                }
                out.extend(b" >>")?;
            }
            let related = relations.related_nodes(i).ok_or(E::Identity)?;
            if !related.is_empty() {
                out.extend(b" /Ref [")?;
                for r in related {
                    reference(out, plan.node(*r))?;
                }
                out.push(b']')?;
            }
            out.extend(b" /K [")?;
            let mut cursor = node.first_binding();
            while let Some(bi) = cursor {
                out.budget.step(1)?;
                let b = &registry.bindings()[bi];
                out.extend(b"<< /Type /MCR /Pg ")?;
                reference(out, plan.page(b.page_index() as usize))?;
                out.extend(b" /MCID ")?;
                out.unsigned(b.mcid() as u64)?;
                out.extend(b" >> ")?;
                cursor = b.next_for_node();
            }
            let mut cursor = rel.first_child();
            while let Some(c) = cursor {
                out.budget.step(1)?;
                reference(out, plan.node(c))?;
                cursor = relations.nodes()[c].next_sibling();
            }
            if let Some(li) = node_links[i] {
                let mut cursor = source.links()[li].first_rectangle();
                while let Some(ai) = cursor {
                    out.budget.step(1)?;
                    let r = &source.rectangles()[ai];
                    out.extend(b"<< /Type /OBJR /Pg ")?;
                    reference(out, plan.page(r.page_index() as usize))?;
                    out.extend(b" /Obj ")?;
                    reference(out, plan.annotation(ai))?;
                    out.extend(b" >> ")?;
                    cursor = r.next_for_link();
                }
            }
            out.extend(b"] >>")
        })?;
    }
    for (i, r) in source.rectangles().iter().enumerate() {
        let master = page_masters
            .get(r.page_index() as usize)
            .ok_or(E::Identity)?
            .master();
        object(out, objects, plan.annotation(i), |out| {
            let link = &source.links()[r.link_index()];
            let b = r.bounds();
            out.extend(b"<< /Type /Annot /Subtype /Link /F 4 /Border [0 0 0] /Rect [")?;
            number(out, b.x().raw())?;
            number(out, master.height - b.y().raw() - b.height().get().raw())?;
            number(out, b.x().raw() + b.width().get().raw())?;
            number(out, master.height - b.y().raw())?;
            out.extend(b"] /P ")?;
            reference(out, plan.page(r.page_index() as usize))?;
            out.extend(b" /StructParent ")?;
            out.unsigned((pc + i) as u64)?;
            match link.target() {
                BookV2NavigationTarget::Destination(di) => {
                    out.extend(b" /Dest ")?;
                    destination(
                        out,
                        plan,
                        page_masters,
                        source.destinations()[di].position().ok_or(E::Identity)?,
                    )?;
                }
                BookV2NavigationTarget::Uri(uri) => {
                    out.extend(b" /A << /S /URI /URI ")?;
                    font_encoding::hex(out, uri.as_bytes())?;
                    out.extend(b" >>")?;
                }
            }
            out.extend(b" >>")
        })?;
    }
    if let Some(id) = plan.dests {
        object(out, objects, id, |out| {
            out.extend(b"<< /Names [")?;
            for i in names {
                out.budget.step(1)?;
                let d = &source.destinations()[*i];
                let BookV2DestinationKind::Anchor(name) = d.kind() else {
                    return Err(E::Identity);
                };
                font_encoding::hex(out, name.as_bytes())?;
                out.push(b' ')?;
                destination(out, plan, page_masters, d.position().ok_or(E::Identity)?)?;
                out.push(b' ')?;
            }
            out.extend(b"] >>")
        })?;
    }
    if let Some(id) = plan.outlines {
        object(out, objects, id, |out| {
            out.extend(b"<< /Type /Outlines /Count ")?;
            out.unsigned(source.outline().len() as u64)?;
            out.extend(b" /First ")?;
            reference(out, plan.outline(0))?;
            let mut last = 0;
            for (i, o) in source.outline().iter().enumerate() {
                out.budget.step(1)?;
                if o.parent().is_none() {
                    last = i;
                }
            }
            out.extend(b" /Last ")?;
            reference(out, plan.outline(last))?;
            out.extend(b" >>")
        })?;
    }
    for (i, o) in source.outline().iter().enumerate() {
        object(out, objects, plan.outline(i), |out| {
            out.extend(b"<< /Title ")?;
            text(out, &navigation.outline()[i].label)?;
            out.extend(b" /Parent ")?;
            reference(
                out,
                o.parent()
                    .map_or(plan.outlines.ok_or(E::Identity)?, |p| plan.outline(p)),
            )?;
            for (key, value) in [
                (b" /Prev".as_slice(), o.previous()),
                (b" /Next", o.next()),
                (b" /First", o.first_child()),
                (b" /Last", o.last_child()),
            ] {
                if let Some(v) = value {
                    out.extend(key)?;
                    out.push(b' ')?;
                    reference(out, plan.outline(v))?;
                }
            }
            if o.first_child().is_some() {
                out.extend(b" /Count ")?;
                out.unsigned(o.descendant_count() as u64)?;
            }
            out.extend(b" /Dest ")?;
            destination(
                out,
                plan,
                page_masters,
                source.destinations()[o.destination()]
                    .position()
                    .ok_or(E::Identity)?,
            )?;
            out.extend(b" >>")
        })?;
    }
    let xref = out.length;
    out.extend(b"xref\n0 ")?;
    out.unsigned(objects.len() as u64)?;
    out.push(b'\n')?;
    out.budget.step(objects.len())?;
    let first_free = objects
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, o)| o.length == 0)
        .map_or(0, |(i, _)| i);
    fixed_decimal(out, first_free as u64, 10)?;
    out.extend(b" 65535 f \n")?;
    let mut free_cursor = 1;
    for (i, o) in objects.iter().enumerate().skip(1) {
        out.budget.step(1)?;
        if o.length == 0 {
            free_cursor = free_cursor.max(i + 1);
            while free_cursor < objects.len() && objects[free_cursor].length != 0 {
                out.budget.step(1)?;
                free_cursor += 1;
            }
            fixed_decimal(
                out,
                if free_cursor == objects.len() {
                    0
                } else {
                    free_cursor as u64
                },
                10,
            )?;
            out.extend(b" 00000 f \n")?;
        } else {
            fixed_decimal(out, o.offset as u64, 10)?;
            out.extend(b" 00000 n \n")?;
        }
    }
    out.extend(b"trailer\n<< /Size ")?;
    out.unsigned(objects.len() as u64)?;
    out.extend(b" /Root ")?;
    reference(out, plan.catalog())?;
    out.extend(b" /Info ")?;
    reference(out, plan.info())?;
    out.extend(b" >>\nstartxref\n")?;
    out.unsigned(xref as u64)?;
    out.extend(b"\n%%EOF\n")
}
fn object(
    out: &mut Encoder<'_>,
    objects: &mut [ObjectRange],
    id: u32,
    body: impl FnOnce(&mut Encoder<'_>) -> Result<(), E>,
) -> Result<(), E> {
    let entry = objects.get_mut(id as usize).ok_or(E::Objects)?;
    if id == 0 || entry.length != 0 {
        return Err(E::Identity);
    }
    let start = out.length;
    out.unsigned(id as u64)?;
    out.extend(b" 0 obj\n")?;
    body(out)?;
    out.extend(b"\nendobj\n")?;
    *entry = ObjectRange {
        offset: start,
        length: out.length - start,
    };
    Ok(())
}
fn raw_object(
    out: &mut Encoder<'_>,
    objects: &mut [ObjectRange],
    id: u32,
    bytes: &[u8],
) -> Result<(), E> {
    let entry = objects.get_mut(id as usize).ok_or(E::Objects)?;
    if id == 0 || entry.length != 0 {
        return Err(E::Identity);
    }
    let start = out.length;
    out.extend(bytes)?;
    *entry = ObjectRange {
        offset: start,
        length: bytes.len(),
    };
    Ok(())
}
fn reference(out: &mut Encoder<'_>, id: u32) -> Result<(), E> {
    out.unsigned(id as u64)?;
    out.extend(b" 0 R ")
}
fn text(out: &mut Encoder<'_>, s: &str) -> Result<(), E> {
    font_encoding::utf16(out, s.chars(), true)
}
fn number(out: &mut Encoder<'_>, n: i64) -> Result<(), E> {
    text_encoding::number(out, n)?;
    out.push(b' ')
}
fn structure_id(out: &mut Encoder<'_>, i: usize) -> Result<(), E> {
    let i = u32::try_from(i).map_err(|_| E::Objects)?;
    font_encoding::hex(out, &i.to_be_bytes())
}
fn destination(
    out: &mut Encoder<'_>,
    p: Plan,
    page_masters: &[BookV2SelectedPageMaster<'_>],
    d: BookV2NavigationPosition,
) -> Result<(), E> {
    let height = page_masters
        .get(d.page_index() as usize)
        .ok_or(E::Identity)?
        .master()
        .height;
    out.push(b'[')?;
    reference(out, p.page(d.page_index() as usize))?;
    out.extend(b" /XYZ ")?;
    number(out, d.x().raw())?;
    number(out, height.checked_sub(d.y().raw()).ok_or(E::Identity)?)?;
    out.extend(b"null]")
}
fn fixed_decimal(out: &mut Encoder<'_>, n: u64, width: usize) -> Result<(), E> {
    let mut b = [b'0'; 20];
    let mut value = n;
    for i in (0..width).rev() {
        b[i] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    if value != 0 {
        return Err(E::Output);
    }
    out.extend(&b[..width])
}
fn stream(
    out: &mut Encoder<'_>,
    attributes: &[u8],
    mut body: impl FnMut(&mut Encoder<'_>) -> Result<(), E>,
) -> Result<(), E> {
    let mut count = Encoder {
        budget: out.budget,
        bytes: None,
        length: 0,
        output_limit: out.output_limit,
        spool_limit: out.spool_limit,
    };
    body(&mut count)?;
    let length = count.length;
    out.extend(b"<< /Length ")?;
    out.unsigned(length as u64)?;
    out.extend(attributes)?;
    out.extend(b" >>\nstream\n")?;
    let start = out.length;
    body(out)?;
    if out.length - start != length {
        return Err(E::Identity);
    }
    out.extend(b"\nendstream")
}
struct XmlSink<'a, 'b> {
    output: &'a mut Encoder<'b>,
    error: Option<E>,
}
impl std::fmt::Write for XmlSink<'_, '_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.output.extend(s.as_bytes()).map_err(|e| {
            self.error = Some(e);
            std::fmt::Error
        })
    }
}
fn info(out: &mut Encoder<'_>, m: &typaxis_syntax::StagingDocumentMetadata) -> Result<(), E> {
    out.extend(b"<<")?;
    for (key, s) in [
        (b" /Title ".as_slice(), m.title.as_deref()),
        (b" /Author ", m.author.as_deref()),
        (b" /Subject ", m.subject.as_deref()),
    ] {
        if let Some(s) = s {
            out.extend(key)?;
            text(out, s)?;
        }
    }
    if !m.keywords.is_empty() {
        out.extend(b" /Keywords ")?;
        font_encoding::utf16(
            out,
            m.keywords
                .iter()
                .enumerate()
                .flat_map(|(i, s)| (if i == 0 { "" } else { "; " }).chars().chain(s.chars())),
            true,
        )?;
    }
    for (key, date) in [
        (b" /CreationDate ".as_slice(), m.created.as_deref()),
        (b" /ModDate ", m.modified.as_deref()),
    ] {
        if let Some(s) = date {
            if s.len() != 20 || !s.is_ascii() {
                return Err(E::Identity);
            }
            out.extend(key)?;
            out.extend(b"(D:")?;
            for range in [0..4, 5..7, 8..10, 11..13, 14..16, 17..19] {
                out.extend(&s.as_bytes()[range])?;
            }
            out.extend(b"Z)")?;
        }
    }
    out.extend(b" /Producer ")?;
    let engine = EngineIdentity::compiled();
    font_encoding::utf16(
        out,
        engine
            .name()
            .chars()
            .chain(std::iter::once(' '))
            .chain(engine.version().chars()),
        true,
    )?;
    out.extend(b" >>")
}

#[path = "book_v2_pipeline.rs"]
mod pipeline;
pub use pipeline::*;
