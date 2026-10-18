//! Opt-in permits for Safe-SVG 2 parsing, IR storage, and canonicalization.
use crate::ResourceAdmissionError;
use typaxis_core::ImageResourceId;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafeSvgSourceStage {
    Validation,
    Count,
    CountFinish,
    ClipStorage,
    ClipCollect,
    ClipOrder,
    ClipCompare,
    ClipLookup,
    ClipReplay,
    ClipFinish,
    Analyze,
    AnalyzeDefinitions,
    AnalyzeReferences,
    AnalyzePath,
    AnalyzeClip,
    Build,
    BuildDefinitions,
    BuildReferences,
    BuildDraws,
    BuildPath,
    BuildClips,
    BuildClip,
    IrDefinitions,
    CanonicalMeasure,
    CanonicalStorage,
    CanonicalWrite,
    IrHash,
    IrFinish,
    IrOwner,
    BindingStorage,
    BindingMigrate,
    BindingLookup,
    BindingInsert,
    BindingAdvance,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SafeSvgSourceRequest {
    pub image_id: ImageResourceId,
    pub stage: SafeSvgSourceStage,
    pub byte_offset: u64,
    pub byte_length: u64,
    pub work: u64,
    pub records: u64,
    pub storage_bytes: u64,
}
pub trait SafeSvgSourceBudget {
    type Error;
    const OBSERVED: bool = true;
    /// Source-only callers keep their existing requests unless they opt in.
    const OBSERVE_COUNT: bool = false;
    /// Opt in to the borrowed, fallible clip-closure index after native Analyze.
    const OBSERVE_CLIP_CLOSURE: bool = false;
    /// Observe Count, Analyze, clip closure, Build, and canonical IR storage/work.
    const OBSERVE_IR: bool = false;
    /// Observe the shared IR owner and the resolver's reserved image bindings.
    const OBSERVE_BINDING: bool = false;
    /// Approve this allowance before its work or logical storage reservation.
    fn work(&mut self, request: SafeSvgSourceRequest) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub struct SafeSvgSourceBudgetFailure<E> {
    pub error: E,
    pub request: SafeSvgSourceRequest,
}
impl<E: std::fmt::Debug> std::fmt::Display for SafeSvgSourceBudgetFailure<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SVG admission work budget: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for SafeSvgSourceBudgetFailure<E> {}
#[derive(Debug)]
pub enum SafeSvgSourceError<E> {
    Native(ResourceAdmissionError),
    Budget(SafeSvgSourceBudgetFailure<E>),
}
impl<E> From<ResourceAdmissionError> for SafeSvgSourceError<E> {
    fn from(e: ResourceAdmissionError) -> Self {
        Self::Native(e)
    }
}
impl<E> SafeSvgSourceError<E> {
    pub(crate) fn map_native(
        self,
        map: impl FnOnce(ResourceAdmissionError) -> ResourceAdmissionError,
    ) -> Self {
        match self {
            Self::Native(e) => Self::Native(map(e)),
            e => e,
        }
    }
}
pub(crate) struct Unobserved;
impl SafeSvgSourceBudget for Unobserved {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn work(&mut self, _: SafeSvgSourceRequest) -> Result<(), Self::Error> {
        unreachable!()
    }
}
pub(crate) fn charge<B: SafeSvgSourceBudget>(
    bytes: usize,
    image_id: ImageResourceId,
    budget: &mut B,
) -> Result<(), SafeSvgSourceError<B::Error>> {
    charge_stage(bytes, image_id, SafeSvgSourceStage::Validation, budget)
}
pub(crate) fn charge_stage<B: SafeSvgSourceBudget>(
    bytes: usize,
    image_id: ImageResourceId,
    stage: SafeSvgSourceStage,
    budget: &mut B,
) -> Result<(), SafeSvgSourceError<B::Error>> {
    let per_byte = match stage {
        SafeSvgSourceStage::Validation => 16,
        SafeSvgSourceStage::Count => 128,
        SafeSvgSourceStage::Analyze | SafeSvgSourceStage::Build => 256,
        SafeSvgSourceStage::CountFinish => 0,
        _ => unreachable!("chunk charging is only used for parser passes"),
    };
    if stage == SafeSvgSourceStage::CountFinish {
        let request = SafeSvgSourceRequest {
            image_id,
            stage,
            byte_offset: u64::try_from(bytes).map_err(|_| ResourceAdmissionError::ResourceLimit)?,
            byte_length: 0,
            work: 64,
            records: 0,
            storage_bytes: 0,
        };
        return budget.work(request).map_err(|error| {
            SafeSvgSourceError::Budget(SafeSvgSourceBudgetFailure { error, request })
        });
    }
    for offset in (0..bytes.max(1)).step_by(256) {
        let length = (bytes - offset).min(256) as u64;
        let request = SafeSvgSourceRequest {
            image_id,
            stage,
            byte_offset: u64::try_from(offset)
                .map_err(|_| ResourceAdmissionError::ResourceLimit)?,
            byte_length: length,
            work: per_byte * length + 64,
            records: 0,
            storage_bytes: 0,
        };
        budget.work(request).map_err(|error| {
            SafeSvgSourceError::Budget(SafeSvgSourceBudgetFailure { error, request })
        })?;
    }
    Ok(())
}

pub(crate) fn permit<B: SafeSvgSourceBudget>(
    budget: &mut B,
    image_id: ImageResourceId,
    stage: SafeSvgSourceStage,
    records: usize,
    storage_bytes: usize,
    work: u64,
) -> Result<(), SafeSvgSourceError<B::Error>> {
    let request = SafeSvgSourceRequest {
        image_id,
        stage,
        byte_offset: 0,
        byte_length: 0,
        work,
        records: u64::try_from(records).map_err(|_| ResourceAdmissionError::ResourceLimit)?,
        storage_bytes: u64::try_from(storage_bytes)
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?,
    };
    budget
        .work(request)
        .map_err(|error| SafeSvgSourceError::Budget(SafeSvgSourceBudgetFailure { error, request }))
}

pub(crate) fn reserve_vec<T, B: SafeSvgSourceBudget>(
    budget: &mut B,
    image_id: ImageResourceId,
    stage: SafeSvgSourceStage,
    rows: usize,
) -> Result<Vec<T>, SafeSvgSourceError<B::Error>> {
    let bytes = rows
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    permit(budget, image_id, stage, rows, bytes, 64)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(rows)
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    Ok(output)
}

pub(crate) fn push_within<T>(
    values: &mut Vec<T>,
    value: T,
    bound: Option<usize>,
) -> Result<(), ResourceAdmissionError> {
    if bound.is_some_and(|n| values.len() >= n) {
        return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
    }
    values.push(value);
    Ok(())
}

#[path = "safe_svg_clip_closure.rs"]
pub(crate) mod clip_closure;

#[cfg(test)]
#[path = "safe_svg_source_tests.rs"]
mod tests;
