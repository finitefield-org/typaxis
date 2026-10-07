//! Image occurrences in actual selected page coordinates. Per-occurrence
//! semantic scopes and the page's global Y flip belong to the document owner.
use super::*;
use crate::safe_vector_v2::encoding;
use crate::{image_encoding, ObjectId};
use typaxis_core::NodeId;
use typaxis_display_list::book_v2::BookV2ImagePaint;
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
pub struct BookV2ImageCommands<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'k BookV2ImageObjects<'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    commands: Vec<BookV2ImageCommand>,
    bytes: Vec<u8>,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2ImageCommands<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn source(&self) -> &'k BookV2ImageObjects<'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
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
pub struct BookV2ImageCommandBuilder<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'k BookV2ImageObjects<'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    pub(super) budget: Budget,
}
impl<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2ImageCommandBuilder<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn new(
        source: &'k BookV2ImageObjects<'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        let display = source.source().source().source().display();
        display
            .verify_resources(display.admitted(), limits)
            .map_err(|_| E::Identity)?;
        let base = limits.base().get();
        let records = prior_records
            .max(source.record_charge())
            .checked_add(1)
            .ok_or(E::Records)?;
        let spool = prior_spool.max(source.spool_charge());
        let output = prior_output.max(source.output_charge());
        let work = prior_work.max(source.work_steps());
        if records > base.max_fragments {
            return Err(E::Records);
        }
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
    pub fn build(
        &mut self,
    ) -> Result<BookV2ImageCommands<'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
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
        Ok(BookV2ImageCommands {
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
fn encode(
    source: &BookV2ImageObjects<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
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
