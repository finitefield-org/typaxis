//! Real reusable Form content and alpha dictionaries from selected admitted IR.
//! Page placement, object numbers and per-occurrence semantics are later stages.
use super::*;
use crate::safe_vector_v2::{encoding, StagingSafeVectorPdfV2Error};
use typaxis_resources::{
    book_v2::{BookV2RasterPrograms, BookV2SelectedImage},
    AdmittedSafeVector,
};
pub const BOOK_V2_VECTOR_PROGRAMS_ALGORITHM: &str = "typaxis.book-2-vector-programs/1";
impl From<StagingSafeVectorPdfV2Error> for E {
    fn from(_: StagingSafeVectorPdfV2Error) -> Self {
        E::Vector
    }
}
#[derive(Clone, Debug)]
pub struct BookV2VectorState {
    fill: u32,
    stroke: u32,
    dictionary: Range<usize>,
}
impl BookV2VectorState {
    pub fn fill_alpha_raw(&self) -> u32 {
        self.fill
    }
    pub fn stroke_alpha_raw(&self) -> u32 {
        self.stroke
    }
    fn key(&self) -> (u32, u32) {
        (self.fill, self.stroke)
    }
}
pub struct BookV2VectorProgram<'i, 'v> {
    source: &'i BookV2SelectedImage<'v>,
    ir: &'v AdmittedSafeVector,
    states: Vec<BookV2VectorState>,
    bytes: Vec<u8>,
    content: Range<usize>,
    fingerprint: [u8; 32],
}
impl<'i, 'v> BookV2VectorProgram<'i, 'v> {
    pub fn source(&self) -> &'i BookV2SelectedImage<'v> {
        self.source
    }
    pub fn ir(&self) -> &'v AdmittedSafeVector {
        self.ir
    }
    pub fn bbox(&self) -> [i64; 4] {
        [
            0,
            0,
            self.ir.intrinsic_width().get().raw(),
            self.ir.intrinsic_height().get().raw(),
        ]
    }
    pub fn states(&self) -> &[BookV2VectorState] {
        &self.states
    }
    pub fn content(&self) -> &[u8] {
        &self.bytes[self.content.clone()]
    }
    pub fn state_dictionary(&self, index: usize) -> Option<&[u8]> {
        self.bytes.get(self.states.get(index)?.dictionary.clone())
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}
pub struct BookV2VectorPrograms<'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'r BookV2RasterPrograms<'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    programs: Vec<Option<BookV2VectorProgram<'i, 'v>>>,
    prior_output: u64,
    fingerprint: [u8; 32],
    budget: Budget,
}
impl<'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2VectorPrograms<'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn source(&self) -> &'r BookV2RasterPrograms<'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.source
    }
    pub fn prior_output_charge(&self) -> u64 {
        self.prior_output
    }
    pub fn program(&self, index: usize) -> Option<&BookV2VectorProgram<'i, 'v>> {
        self.programs.get(index)?.as_ref()
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
pub struct BookV2VectorProgramBuilder<'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'r BookV2RasterPrograms<'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    budget: Budget,
}
impl<'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    BookV2VectorProgramBuilder<'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn new(
        source: &'r BookV2RasterPrograms<'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        let display = source.source().display();
        display
            .verify_resources(display.admitted(), limits)
            .map_err(|_| E::Identity)?;
        let base = limits.base().get();
        let records = prior_records
            .max(source.record_charge())
            .checked_add(1)
            .ok_or(E::Records)?;
        let spool = prior_spool.max(source.spool_charge());
        let work = prior_work.max(source.work_steps());
        if records > base.max_fragments {
            return Err(E::Records);
        }
        if spool > base.max_spool_bytes {
            return Err(E::Spool);
        }
        if prior_output > base.max_output_bytes {
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
                output: prior_output,
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
    ) -> Result<BookV2VectorPrograms<'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
        let prior_output = self.budget.output;
        let source = self.source;
        let images = source.source().images();
        self.budget.reserve(
            images.len().checked_add(1).ok_or(E::Records)?,
            images
                .len()
                .checked_mul(std::mem::size_of::<Option<BookV2VectorProgram<'i, 'v>>>())
                .ok_or(E::Spool)?,
            0,
        )?;
        self.budget.step(1)?;
        let mut programs = Vec::new();
        programs
            .try_reserve_exact(images.len())
            .map_err(|_| E::Allocation)?;
        let mut fingerprint = sha256(BOOK_V2_VECTOR_PROGRAMS_ALGORITHM.as_bytes());
        fingerprint = self.budget.fold(fingerprint, &source.fingerprint())?;
        for (index, image) in images.iter().enumerate() {
            self.budget.step(1)?;
            let Some(ir) = image.image().admitted_safe_vector() else {
                programs.push(None);
                continue;
            };
            if source.program(index).is_some() {
                return Err(E::Identity);
            }
            let mut states = collect_states(ir, &mut self.budget)?;
            let output_limit = self.budget.max_output - self.budget.output;
            let spool_limit = self.budget.max_spool - self.budget.spool;
            let mut counter = Encoder {
                budget: &mut self.budget,
                bytes: None,
                length: 0,
                output_limit,
                spool_limit,
            };
            let content_length = encode_program(ir, &mut states, &mut counter, false)?;
            let length = counter.length;
            self.budget.reserve(1, length, length)?;
            self.budget.step(1)?;
            let mut bytes = Vec::new();
            bytes.try_reserve_exact(length).map_err(|_| E::Allocation)?;
            let mut output = Encoder {
                budget: &mut self.budget,
                bytes: Some(&mut bytes),
                length: 0,
                output_limit: length as u64,
                spool_limit: length as u64,
            };
            if encode_program(ir, &mut states, &mut output, true)? != content_length
                || output.length != length
            {
                return Err(E::Identity);
            }
            self.budget.step(length.div_ceil(64) + 1)?;
            let mut fp = self.budget.fold(
                sha256(BOOK_V2_VECTOR_PROGRAMS_ALGORITHM.as_bytes()),
                &image.image().content_hash(),
            )?;
            fp = self.budget.fold(fp, &ir.fingerprint())?;
            fp = self.budget.fold(fp, &sha256(&bytes))?;
            for state in &states {
                let mut b = [0; 24];
                b[..4].copy_from_slice(&state.fill.to_be_bytes());
                b[4..8].copy_from_slice(&state.stroke.to_be_bytes());
                b[8..16].copy_from_slice(&(state.dictionary.start as u64).to_be_bytes());
                b[16..].copy_from_slice(&(state.dictionary.end as u64).to_be_bytes());
                fp = self.budget.fold(fp, &b)?;
            }
            fingerprint = self
                .budget
                .fold(fingerprint, &(index as u64).to_be_bytes())?;
            fingerprint = self.budget.fold(fingerprint, &fp)?;
            programs.push(Some(BookV2VectorProgram {
                source: image,
                ir,
                states,
                bytes,
                content: 0..content_length,
                fingerprint: fp,
            }));
        }
        Ok(BookV2VectorPrograms {
            prior_output,
            source,
            programs,
            fingerprint,
            budget: self.budget,
        })
    }
}
fn collect_states(
    ir: &AdmittedSafeVector,
    budget: &mut Budget,
) -> Result<Vec<BookV2VectorState>, E> {
    let n = match ir {
        AdmittedSafeVector::V1(v) => v.draws().len(),
        AdmittedSafeVector::V2(v) => v.draws().len(),
    };
    budget.reserve(
        n,
        n.checked_mul(std::mem::size_of::<BookV2VectorState>())
            .ok_or(E::Spool)?,
        0,
    )?;
    budget.step(1)?;
    let mut states = Vec::new();
    states.try_reserve_exact(n).map_err(|_| E::Allocation)?;
    for i in 0..n {
        budget.step(1)?;
        let (fill, stroke) = match ir {
            AdmittedSafeVector::V1(_) => (65536, 65536),
            AdmittedSafeVector::V2(v) => {
                let d = &v.draws()[i];
                (d.fill().alpha().raw(), d.stroke().paint().alpha().raw())
            }
        };
        states.push(BookV2VectorState {
            fill,
            stroke,
            dictionary: 0..0,
        });
    }
    // Fallible heapsort: each comparison and swap consumes work, no scratch allocation.
    fn sift(
        v: &mut [BookV2VectorState],
        mut root: usize,
        end: usize,
        b: &mut Budget,
    ) -> Result<(), E> {
        loop {
            b.step(1)?;
            let Some(mut child) = root
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .filter(|n| *n < end)
            else {
                return Ok(());
            };
            if child + 1 < end {
                b.step(1)?;
                if v[child].key() < v[child + 1].key() {
                    child += 1;
                }
            }
            b.step(1)?;
            if v[root].key() >= v[child].key() {
                return Ok(());
            }
            b.step(1)?;
            v.swap(root, child);
            root = child;
        }
    }
    for root in (0..n / 2).rev() {
        sift(&mut states, root, n, budget)?;
    }
    for end in (1..n).rev() {
        budget.step(1)?;
        states.swap(0, end);
        sift(&mut states, 0, end, budget)?;
    }
    let mut retained = 0;
    for i in 0..n {
        budget.step(1)?;
        if retained == 0 || states[retained - 1].key() != states[i].key() {
            states.swap(retained, i);
            retained += 1;
        }
    }
    states.truncate(retained);
    Ok(states)
}
struct VectorOutput<'o, 'b> {
    out: &'o mut Encoder<'b>,
    states: &'o [BookV2VectorState],
}
impl Sink for VectorOutput<'_, '_> {
    type Error = E;
    fn extend(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.out.extend(bytes)
    }
}
impl encoding::VectorSink for VectorOutput<'_, '_> {
    fn separate_fill_stroke(&self) -> bool {
        true
    }
    fn state(&mut self, fill: u32, stroke: u32) -> Result<(), E> {
        self.out
            .budget
            .step((usize::BITS - self.states.len().leading_zeros()) as usize + 1)?;
        let index = self
            .states
            .binary_search_by_key(&(fill, stroke), BookV2VectorState::key)
            .map_err(|_| E::Vector)?;
        self.extend(b"/GS")?;
        self.unsigned(index as u64)?;
        self.extend(b" gs\n")
    }
}
fn encode_program(
    ir: &AdmittedSafeVector,
    states: &mut [BookV2VectorState],
    output: &mut Encoder<'_>,
    save: bool,
) -> Result<usize, E> {
    encoding::encode(
        ir,
        &mut VectorOutput {
            out: output,
            states,
        },
    )?;
    let content = output.length;
    for state in states {
        let start = output.length;
        encoding::state_dictionary(output, state.fill, state.stroke)?;
        if save {
            state.dictionary = start..output.length;
        }
    }
    Ok(content)
}
