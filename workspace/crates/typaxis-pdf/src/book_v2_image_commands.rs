//! Image occurrences in actual selected page coordinates. Per-occurrence
//! semantic scopes and the page's global Y flip belong to the document owner.
use super::*;
use crate::safe_vector_v2::encoding;
use crate::{ObjectId, image_encoding};
use typaxis_core::NodeId;
use typaxis_display_list::book_v2::BookV2ImagePaint;
use typaxis_display_list::book_v2::{
    BookV2BodyDisplay, BookV2BodyPaintIndex, BookV2ColumnDisplay, BookV2ColumnPaintIndex,
    BookV2ResourceDisplay,
};
pub const BOOK_V2_IMAGE_COMMANDS_ALGORITHM: &str = "typaxis.book-2-image-commands/1";
#[derive(Clone, Debug)]
pub struct BookV2ImageCommand {
    paint: usize,
    resource: usize,
    object: ObjectId,
    page: u32,
    owner: NodeId,
    range: Range<usize>,
}
impl BookV2ImageCommand {
    pub fn paint_index(&self) -> usize {
        self.paint
    }
    pub fn resource_index(&self) -> usize {
        self.resource
    }
    pub fn resource_object(&self) -> ObjectId {
        self.object
    }
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn byte_range(&self) -> Range<usize> {
        self.range.clone()
    }
}
pub type BookV2ImageCommands<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2ImageCommandsData<
        'k,
        'j,
        'r,
        'i,
        'v,
        'a,
        BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2BodyPaintIndex,
    >;
/// Column stages retain their authentic source display and typed paint index.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2ImageCommands, BookV2ColumnImageCommands};
/// fn body<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(value: BookV2ColumnImageCommands<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>)
///     -> BookV2ImageCommands<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> { value }
/// ```
pub type BookV2ColumnImageCommands<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2ImageCommandsData<
        'k,
        'j,
        'r,
        'i,
        'v,
        'a,
        BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2ColumnPaintIndex,
    >;
pub type BookV2ImageCommandBuilder<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2ImageCommandBuilderData<
        'k,
        'j,
        'r,
        'i,
        'v,
        'a,
        BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2BodyPaintIndex,
    >;
/// Column stages retain their authentic source display and typed paint index.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2ImageCommandBuilder, BookV2ColumnImageCommandBuilder};
/// fn body<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(value: BookV2ColumnImageCommandBuilder<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>)
///     -> BookV2ImageCommandBuilder<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> { value }
/// ```
pub type BookV2ColumnImageCommandBuilder<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2ImageCommandBuilderData<
        'k,
        'j,
        'r,
        'i,
        'v,
        'a,
        BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2ColumnPaintIndex,
    >;
pub struct BookV2ImageCommandsData<
    'k,
    'j,
    'r,
    'i,
    'v,
    'a,
    D: BookV2ResourceDisplay<'a>,
    P: Copy = <D as BookV2ResourceDisplay<'a>>::Paint,
> {
    source: &'k BookV2ImageObjectsData<'j, 'r, 'i, 'v, 'a, D, P>,
    commands: Vec<BookV2ImageCommand>,
    bytes: Vec<u8>,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<'k, 'j, 'r, 'i, 'v, 'a, D: BookV2ResourceDisplay<'a>, P: Copy>
    BookV2ImageCommandsData<'k, 'j, 'r, 'i, 'v, 'a, D, P>
{
    pub fn source(&self) -> &'k BookV2ImageObjectsData<'j, 'r, 'i, 'v, 'a, D, P> {
        self.source
    }
    pub fn commands(&self) -> &[BookV2ImageCommand] {
        &self.commands
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn for_paint(&self, paint: usize) -> Option<&BookV2ImageCommand> {
        self.commands
            .binary_search_by_key(&paint, |c| c.paint)
            .ok()
            .map(|i| &self.commands[i])
    }
    pub fn command_bytes(&self, index: usize) -> Option<&[u8]> {
        self.bytes.get(self.commands.get(index)?.range.clone())
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
pub struct BookV2ImageCommandBuilderData<
    'k,
    'j,
    'r,
    'i,
    'v,
    'a,
    D: BookV2ResourceDisplay<'a>,
    P: Copy = <D as BookV2ResourceDisplay<'a>>::Paint,
> {
    source: &'k BookV2ImageObjectsData<'j, 'r, 'i, 'v, 'a, D, P>,
    pub(super) budget: Budget,
}
impl<'k, 'j, 'r, 'i, 'v, 'a, D: BookV2ResourceDisplay<'a>, P: Copy>
    BookV2ImageCommandBuilderData<'k, 'j, 'r, 'i, 'v, 'a, D, P>
{
    pub fn new(
        source: &'k BookV2ImageObjectsData<'j, 'r, 'i, 'v, 'a, D, P>,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
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
    /// Return accepted constructor charges even when a later cap rejects the builder.
    #[allow(clippy::too_many_arguments)]
    pub fn new_counted(
        source: &'k BookV2ImageObjectsData<'j, 'r, 'i, 'v, 'a, D, P>,
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
    ) -> Result<Self, E> {
        let records = prior_records.max(source.record_charge());
        let spool = prior_spool.max(source.spool_charge());
        let output = prior_output.max(source.output_charge());
        let work = prior_work.max(source.work_steps());
        *observed_records = records;
        *observed_spool = spool;
        *observed_output = output;
        *observed_work = work;
        let display = source.source().source().source().display();
        display
            .verify_resource_selection(display.admitted(), limits)
            .map_err(|_| E::Identity)?;
        let base = limits.base().get();
        let records = records
            .checked_add(1)
            .filter(|n| *n <= base.max_fragments)
            .ok_or(E::Records)?;
        *observed_records = records;
        if spool > base.max_spool_bytes {
            return Err(E::Spool);
        }
        if output > base.max_output_bytes {
            return Err(E::Output);
        }
        if work > max_work {
            return Err(E::Work);
        }
        Ok(Self {
            source,
            budget: Budget {
                max_records: base.max_fragments,
                max_spool: base.max_spool_bytes,
                max_output: base.max_output_bytes,
                max_work,
                records,
                spool,
                output,
                work,
            },
        })
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
    pub fn build(&mut self) -> Result<BookV2ImageCommandsData<'k, 'j, 'r, 'i, 'v, 'a, D, P>, E> {
        let count = self.source.source().source().source().uses().len();
        let metadata = count
            .checked_mul(std::mem::size_of::<BookV2ImageCommand>())
            .ok_or(E::Spool)?;
        let output_limit = self.budget.max_output - self.budget.output;
        let spool_limit = (self.budget.max_spool - self.budget.spool)
            .checked_sub(metadata as u64)
            .ok_or(E::Spool)?;
        let mut counter = Encoder {
            budget: &mut self.budget,
            bytes: None,
            length: 0,
            output_limit,
            spool_limit,
        };
        encode(self.source, &mut counter, None)?;
        let length = counter.length;
        self.budget.reserve(
            count.checked_add(1).ok_or(E::Records)?,
            metadata.checked_add(length).ok_or(E::Spool)?,
            length,
        )?;
        self.budget.step(1)?;
        let mut bytes = Vec::new();
        let mut commands = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| E::Allocation)?;
        commands
            .try_reserve_exact(count)
            .map_err(|_| E::Allocation)?;
        let mut out = Encoder {
            budget: &mut self.budget,
            bytes: Some(&mut bytes),
            length: 0,
            output_limit: length as u64,
            spool_limit: length as u64,
        };
        encode(self.source, &mut out, Some(&mut commands))?;
        if out.length != length || commands.len() != count {
            return Err(E::Identity);
        }
        self.budget.step(length.div_ceil(64) + 1)?;
        let mut fp = self.budget.fold(
            sha256(BOOK_V2_IMAGE_COMMANDS_ALGORITHM.as_bytes()),
            &self.source.fingerprint(),
        )?;
        fp = self.budget.fold(fp, &sha256(&bytes))?;
        for c in &commands {
            let mut b = [0; 44];
            b[..8].copy_from_slice(&(c.paint as u64).to_be_bytes());
            b[8..16].copy_from_slice(&(c.resource as u64).to_be_bytes());
            b[16..20].copy_from_slice(&c.object.get().to_be_bytes());
            b[20..24].copy_from_slice(&c.page.to_be_bytes());
            b[24..28].copy_from_slice(&c.owner.get().to_be_bytes());
            b[28..36].copy_from_slice(&(c.range.start as u64).to_be_bytes());
            b[36..].copy_from_slice(&(c.range.end as u64).to_be_bytes());
            fp = self.budget.fold(fp, &b)?;
        }
        Ok(BookV2ImageCommandsData {
            source: self.source,
            commands,
            bytes,
            fingerprint: fp,
            budget: self.budget,
        })
    }
}
struct Placement<'o, 'b>(&'o mut Encoder<'b>);
impl Sink for Placement<'_, '_> {
    type Error = E;
    fn extend(&mut self, b: &[u8]) -> Result<(), E> {
        self.0.extend(b)
    }
}
impl encoding::VectorSink for Placement<'_, '_> {
    fn state(&mut self, _: u32, _: u32) -> Result<(), E> {
        Err(E::Vector)
    }
}
fn encode<'a, D: BookV2ResourceDisplay<'a>, P: Copy>(
    source: &BookV2ImageObjectsData<'_, '_, '_, '_, 'a, D, P>,
    out: &mut Encoder<'_>,
    mut commands: Option<&mut Vec<BookV2ImageCommand>>,
) -> Result<(), E> {
    for selected in source.source().source().source().uses() {
        out.budget.step(1)?;
        let usage = selected.usage();
        let resource = selected.resource_index();
        let objects = source.for_image(resource).ok_or(E::Identity)?;
        let object = objects.first().ok_or(E::Identity)?;
        let start = out.length;
        match usage.geometry() {
            BookV2ImagePaint::Raster { viewport } => {
                if object.role() != BookV2ImageObjectRole::Image {
                    return Err(E::Identity);
                }
                let bottom = viewport
                    .y()
                    .checked_add(viewport.height().get())
                    .ok_or(E::Identity)?;
                image_encoding::raster_placement(
                    out,
                    [
                        viewport.width().get().raw(),
                        viewport
                            .height()
                            .get()
                            .raw()
                            .checked_neg()
                            .ok_or(E::Identity)?,
                        viewport.x().raw(),
                        bottom.raw(),
                    ],
                    |out| {
                        out.extend(b"BI")?;
                        out.unsigned(resource as u64)
                    },
                )?;
            }
            BookV2ImagePaint::Vector { matrix, color, .. } => {
                if object.role() != BookV2ImageObjectRole::Form {
                    return Err(E::Identity);
                }
                encoding::placement(&mut Placement(out), matrix, color, |out| {
                    out.extend(b"BI")?;
                    out.unsigned(resource as u64)
                })?;
                out.push(b'\n')?;
            }
        }
        if let Some(commands) = &mut commands {
            commands.push(BookV2ImageCommand {
                paint: selected.paint_index(),
                resource,
                object: object.id(),
                page: usage.page_index(),
                owner: usage.owner(),
                range: start..out.length,
            });
        }
    }
    Ok(())
}

// Borrowed command continuation: existing builder fields and kernels above stay unchanged.
/// A scoped continuation of one driver-owned image-command builder.
///
/// Private fields and an invariant loan keep the original mutable budget with
/// its driver. Normal callback return retains charges even if a borrowed
/// reference is forgotten; this does not claim panic or unwind recovery.
///
/// Builds and reads the original command builder.
///
/// ```
/// use typaxis_pdf::book_v2::{BookV2ColumnImageCommandBuilder as Builder, BookV2ColumnImageCommands as Commands, BookV2ColumnImageCommandContinuation as Continuation, BookV2FontStreamError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type C<'a> = Continuation<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type V<'a> = Commands<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn rebuild<'a>(builder: &mut B<'a>) -> Result<V<'a>, BookV2FontStreamError> {
///     builder.with_continuation(|loan| {
///         let _ = (loan.record_charge(), loan.spool_charge(), loan.output_charge(), loan.work_steps());
///         loan.build()
///     })
/// }
/// ```
///
/// A raw builder cannot replace the borrowed continuation.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2ColumnImageCommandBuilder as Builder, BookV2ColumnImageCommands as Commands, BookV2ColumnImageCommandContinuation as Continuation, BookV2FontStreamError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type C<'a> = Continuation<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type V<'a> = Commands<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn replace<'a>(builder: &mut B<'a>, replacement: B<'a>) {
///     builder.with_continuation(|loan| *loan = replacement);
/// }
/// ```
///
/// The original mutable builder is private.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2ColumnImageCommandBuilder as Builder, BookV2ColumnImageCommands as Commands, BookV2ColumnImageCommandContinuation as Continuation, BookV2FontStreamError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type C<'a> = Continuation<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type V<'a> = Commands<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn raw<'a>(builder: &mut B<'a>) {
///     builder.with_continuation(|loan| { let _ = &mut loan.builder; });
/// }
/// ```
///
/// A continuation cannot escape its loan.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2ColumnImageCommandBuilder as Builder, BookV2ColumnImageCommands as Commands, BookV2ColumnImageCommandContinuation as Continuation, BookV2FontStreamError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type C<'a> = Continuation<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type V<'a> = Commands<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn escape<'a>(builder: &'a mut B<'a>) -> &'a mut C<'a> {
///     builder.with_continuation(|loan| loan)
/// }
/// ```
///
/// Independent invariant loans cannot be swapped.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2ColumnImageCommandBuilder as Builder, BookV2ColumnImageCommands as Commands, BookV2ColumnImageCommandContinuation as Continuation, BookV2FontStreamError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type C<'a> = Continuation<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type V<'a> = Commands<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn swap<'a>(left: &mut B<'a>, right: &mut B<'a>) {
///     left.with_continuation(|outer| {
///         right.with_continuation(|inner| std::mem::swap(outer, inner));
///     });
/// }
/// ```
///
/// Independent nested loans may inspect their own builders.
///
/// ```
/// use typaxis_pdf::book_v2::{BookV2ColumnImageCommandBuilder as Builder, BookV2ColumnImageCommands as Commands, BookV2ColumnImageCommandContinuation as Continuation, BookV2FontStreamError};
/// type B<'a> = Builder<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type C<'a> = Continuation<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// type V<'a> = Commands<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a>;
/// fn inspect<'a>(left: &mut B<'a>, right: &mut B<'a>) -> u64 {
///     left.with_continuation(|outer| {
///         right.with_continuation(|inner| outer.record_charge() + inner.record_charge())
///     })
/// }
/// ```
pub struct BookV2ImageCommandContinuationData<
    'loan,
    'k,
    'j,
    'r,
    'i,
    'v,
    'a,
    D: BookV2ResourceDisplay<'a>,
    P: Copy = <D as BookV2ResourceDisplay<'a>>::Paint,
> {
    builder: &'loan mut BookV2ImageCommandBuilderData<'k, 'j, 'r, 'i, 'v, 'a, D, P>,
    invariant: std::marker::PhantomData<fn(&'loan ()) -> &'loan ()>,
}
pub type BookV2ImageCommandContinuation<'loan, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2ImageCommandContinuationData<
        'loan,
        'k,
        'j,
        'r,
        'i,
        'v,
        'a,
        BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2BodyPaintIndex,
    >;
pub type BookV2ColumnImageCommandContinuation<
    'loan,
    'k,
    'j,
    'r,
    'i,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> = BookV2ImageCommandContinuationData<
    'loan,
    'k,
    'j,
    'r,
    'i,
    'v,
    'a,
    BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    BookV2ColumnPaintIndex,
>;
impl<'k, 'j, 'r, 'i, 'v, 'a, D: BookV2ResourceDisplay<'a>, P: Copy>
    BookV2ImageCommandContinuationData<'_, 'k, 'j, 'r, 'i, 'v, 'a, D, P>
{
    pub fn build(&mut self) -> Result<BookV2ImageCommandsData<'k, 'j, 'r, 'i, 'v, 'a, D, P>, E> {
        self.builder.build()
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
}
impl<'k, 'j, 'r, 'i, 'v, 'a, D: BookV2ResourceDisplay<'a>, P: Copy>
    BookV2ImageCommandBuilderData<'k, 'j, 'r, 'i, 'v, 'a, D, P>
{
    /// Loan this builder without exposing a replaceable mutable owner.
    pub fn with_continuation<R>(
        &mut self,
        inspect: impl for<'loan> FnOnce(
            &mut BookV2ImageCommandContinuationData<'loan, 'k, 'j, 'r, 'i, 'v, 'a, D, P>,
        ) -> R,
    ) -> R {
        let mut continuation = BookV2ImageCommandContinuationData {
            builder: self,
            invariant: std::marker::PhantomData,
        };
        inspect(&mut continuation)
    }
}
