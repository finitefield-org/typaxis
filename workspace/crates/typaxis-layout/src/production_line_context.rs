//! Derive shaping context boundaries from real selected clusters and atoms.
//! This is a reshape input owner, not evidence that the rebreak loop converged.
use super::*;

pub struct ProductionSelectedParagraphContext {
    owner: NodeId,
    ends: Vec<u32>,
}
impl ProductionSelectedParagraphContext {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn ends(&self) -> &[u32] {
        &self.ends
    }
}
pub struct ProductionSelectedLineContexts {
    paragraphs: Vec<ProductionSelectedParagraphContext>,
    source_fingerprint: [u8; 32],
    record_charge: u64,
}
impl ProductionSelectedLineContexts {
    pub fn paragraphs(&self) -> &[ProductionSelectedParagraphContext] {
        &self.paragraphs
    }
    pub const fn source_fingerprint(&self) -> [u8; 32] {
        self.source_fingerprint
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
}

pub fn production_selected_line_contexts(
    selected: &ProductionInlineLineLayout<'_, '_>,
) -> Result<ProductionSelectedLineContexts, ProductionInlinePreparationError> {
    selected_contexts(
        selected.paragraphs(),
        selected.output_records(),
        selected.prepared_limit(),
        selected.fingerprint(),
    )
}
pub(super) fn selected_contexts(
    selected_paragraphs: &[ProductionInlineParagraphLineLayout<'_, '_>],
    output_records: u64,
    max_fragments: u64,
    fingerprint: [u8; 32],
) -> Result<ProductionSelectedLineContexts, ProductionInlinePreparationError> {
    selected_contexts_counted(
        selected_paragraphs,
        output_records,
        max_fragments,
        fingerprint,
        &mut 0,
    )
}
pub(super) fn selected_contexts_counted(
    selected_paragraphs: &[ProductionInlineParagraphLineLayout<'_, '_>],
    output_records: u64,
    max_fragments: u64,
    fingerprint: [u8; 32],
    observed_records: &mut u64,
) -> Result<ProductionSelectedLineContexts, ProductionInlinePreparationError> {
    selected_contexts_kernel(
        selected_paragraphs,
        output_records,
        max_fragments,
        fingerprint,
        observed_records,
        &mut LegacyContextOps,
    )
}
// Private concrete allocation boundary shared only by the running-region
// adapters. Production has no hook or storage; test failure/trace lives in
// this library's own unit process, never in CLI dependency builds.
#[cfg(feature = "book-v2-staging")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RegionMetadataKind {
    ContextParagraphs,
    ContextEnds,
    Widths,
    Origins,
    Inputs,
    Feedback,
    State,
}
#[cfg(all(test, feature = "book-v2-staging"))]
std::thread_local! {
    static REGION_METADATA:std::cell::RefCell<Option<(Option<usize>,Vec<(RegionMetadataKind,[u64;3])>)>>=const{std::cell::RefCell::new(None)};
}
#[cfg(feature = "book-v2-staging")]
pub(super) fn region_metadata_boundary(
    kind: RegionMetadataKind,
    counters: [u64; 3],
    owner: NodeId,
) -> Result<(), ProductionInlinePreparationError> {
    #[cfg(test)]
    return REGION_METADATA.with(|state| {
        let mut state = state.borrow_mut();
        if let Some((fault, events)) = state.as_mut() {
            let at = events.len();
            events.push((kind, counters));
            if *fault == Some(at) {
                return Err(error(
                    owner,
                    ProductionInlinePreparationErrorKind::AllocationFailure,
                ));
            }
        }
        Ok(())
    });
    #[cfg(not(test))]
    {
        let _ = (kind, counters, owner);
        Ok(())
    }
}
#[cfg(all(test, feature = "book-v2-staging"))]
pub(super) fn with_region_metadata_fault<R>(
    fault: Option<usize>,
    run: impl FnOnce() -> R,
) -> (R, Vec<(RegionMetadataKind, [u64; 3])>) {
    REGION_METADATA
        .with(|state| assert!(state.borrow_mut().replace((fault, Vec::new())).is_none()));
    let result = run();
    let (_, trace) =
        REGION_METADATA.with(|state| state.borrow_mut().take().expect("installed region trace"));
    (result, trace)
}
trait ContextOps {
    type Error: From<ProductionInlinePreparationError>;
    fn step(&mut self, owner: NodeId) -> Result<(), Self::Error>;
    fn request<T>(&mut self, owner: NodeId, n: usize) -> Result<(), Self::Error>;
    fn reserve<T>(
        &mut self,
        owner: NodeId,
        out: &mut Vec<T>,
        n: usize,
        ends: bool,
    ) -> Result<(), Self::Error> {
        self.request::<T>(owner, n)?;
        self.boundary(ends, owner)?;
        out.try_reserve_exact(n).map_err(|_| {
            error(
                owner,
                ProductionInlinePreparationErrorKind::AllocationFailure,
            )
        })?;
        Ok(())
    }
    fn boundary(&self, _ends: bool, _owner: NodeId) -> Result<(), Self::Error> {
        Ok(())
    }
}
struct LegacyContextOps;
impl ContextOps for LegacyContextOps {
    type Error = ProductionInlinePreparationError;
    fn step(&mut self, _: NodeId) -> Result<(), Self::Error> {
        Ok(())
    }
    fn request<T>(&mut self, _: NodeId, _: usize) -> Result<(), Self::Error> {
        Ok(())
    }
}
#[cfg(feature = "book-v2-staging")]
struct ObservedContextOps<'a, 'id, 'cells>(
    &'a mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
);
#[cfg(feature = "book-v2-staging")]
impl ContextOps for ObservedContextOps<'_, '_, '_> {
    type Error = book_v2::BookV2ObservedPageRegionLayoutError;
    fn step(&mut self, owner: NodeId) -> Result<(), Self::Error> {
        self.0.debit_work(owner, 1).map_err(Self::Error::Accounting)
    }
    fn request<T>(&mut self, owner: NodeId, n: usize) -> Result<(), Self::Error> {
        self.step(owner)?;
        let overflow = || {
            error(
                owner,
                ProductionInlinePreparationErrorKind::ArithmeticOverflow,
            )
        };
        let r = u64::try_from(n).map_err(|_| overflow())?;
        let s = n
            .checked_mul(std::mem::size_of::<T>())
            .and_then(|n| u64::try_from(n).ok())
            .ok_or_else(overflow)?;
        let w = r.checked_add(1).ok_or_else(overflow)?;
        self.0
            .debit(owner, [r, s, w])
            .map_err(Self::Error::Accounting)
    }
    fn boundary(&self, ends: bool, owner: NodeId) -> Result<(), Self::Error> {
        region_metadata_boundary(
            if ends {
                RegionMetadataKind::ContextEnds
            } else {
                RegionMetadataKind::ContextParagraphs
            },
            self.0.counters(),
            owner,
        )
        .map_err(Into::into)
    }
}
#[cfg(feature = "book-v2-staging")]
pub(super) fn selected_contexts_observed_counted(
    selected_paragraphs: &[ProductionInlineParagraphLineLayout<'_, '_>],
    output_records: u64,
    max_fragments: u64,
    fingerprint: [u8; 32],
    observed_records: &mut u64,
    accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<ProductionSelectedLineContexts, book_v2::BookV2ObservedPageRegionLayoutError> {
    selected_contexts_kernel(
        selected_paragraphs,
        output_records,
        max_fragments,
        fingerprint,
        observed_records,
        &mut ObservedContextOps(accounting),
    )
}
fn selected_contexts_kernel<A: ContextOps>(
    selected_paragraphs: &[ProductionInlineParagraphLineLayout<'_, '_>],
    output_records: u64,
    max_fragments: u64,
    fingerprint: [u8; 32],
    observed_records: &mut u64,
    ops: &mut A,
) -> Result<ProductionSelectedLineContexts, A::Error> {
    *observed_records = output_records;
    use ProductionInlinePreparationErrorKind as E;
    let root = NodeId::new(0);
    let mut record_charge = output_records;
    // An owned paragraph record, a borrowed shaper input view, and each line end.
    for paragraph in selected_paragraphs {
        ops.step(paragraph.owner())?;
        record_charge = record_charge
            .checked_add(paragraph.lines().len() as u64)
            .and_then(|n| n.checked_add(2))
            .filter(|n| *n <= max_fragments)
            .ok_or_else(|| error(paragraph.owner(), E::UnitLimit))?;
        *observed_records = record_charge;
    }
    let mut paragraphs = Vec::new();
    ops.reserve::<ProductionSelectedParagraphContext>(
        root,
        &mut paragraphs,
        selected_paragraphs.len(),
        false,
    )?;
    for paragraph in selected_paragraphs {
        ops.step(paragraph.owner())?;
        let mut ends = Vec::new();
        ops.reserve::<u32>(paragraph.owner(), &mut ends, paragraph.lines().len(), true)?;
        let mut offset = 0u32;
        for line in paragraph.lines() {
            ops.step(paragraph.owner())?;
            for item in line.items() {
                ops.step(paragraph.owner())?;
                let count = match item {
                    ProductionPlacedInline::Text(t) => u32::try_from(t.utf8().len())
                        .map_err(|_| error(paragraph.owner(), E::ArithmeticOverflow))?,
                    ProductionPlacedInline::Vector(_) | ProductionPlacedInline::Math(_) => 3, // U+FFFC
                    #[cfg(feature = "book-v2-staging")]
                    ProductionPlacedInline::BookV2Math(_) => 3,
                    ProductionPlacedInline::Break(b) if b.kind() == BreakKind::Mandatory => 3, // U+2028
                    ProductionPlacedInline::Break(_) => 0,
                };
                offset = offset
                    .checked_add(count)
                    .ok_or_else(|| error(paragraph.owner(), E::ArithmeticOverflow))?;
            }
            ends.push(offset);
        }
        paragraphs.push(ProductionSelectedParagraphContext {
            owner: paragraph.owner(),
            ends,
        });
    }
    Ok(ProductionSelectedLineContexts {
        paragraphs,
        source_fingerprint: fingerprint,
        record_charge,
    })
}

#[cfg(all(test, feature = "book-v2-staging"))]
mod observed_region_request_tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_shaping::book_v2::{
        with_book_v2_observed_source_accounting, BookV2ObservedAccountingError as A,
    };
    type E = book_v2::BookV2ObservedPageRegionLayoutError;
    fn request<T>(
        n: usize,
        ends: bool,
        prior: [u64; 3],
        maxr: u64,
        maxw: u64,
        fault: bool,
    ) -> (Result<(), E>, [u64; 3], Vec<(RegionMetadataKind, [u64; 3])>) {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let mut current = prior;
        let (result, trace) = with_region_metadata_fault(fault.then_some(0), || {
            let [r, s, w] = &mut current;
            with_book_v2_observed_source_accounting(r, s, w, &limits, maxr, maxw, |a| {
                let mut out = Vec::<T>::new();
                ObservedContextOps(a).reserve(NodeId::new(17), &mut out, n, ends)
            })
        });
        (result, current, trace)
    }
    fn check<T>(ends: bool) {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        for n in [0usize, 1, 3] {
            let bytes = (n * std::mem::size_of::<T>()) as u64;
            let prior = [5, limits.get().max_spool_bytes - bytes, 11];
            let exact = [5 + n as u64, limits.get().max_spool_bytes, 13 + n as u64];
            let (ok, current, trace) = request::<T>(n, ends, prior, exact[0], exact[2], false);
            assert!(ok.is_ok());
            assert_eq!(current, exact);
            assert_eq!(trace.len(), 1);
            assert_eq!(
                trace[0],
                (
                    if ends {
                        RegionMetadataKind::ContextEnds
                    } else {
                        RegionMetadataKind::ContextParagraphs
                    },
                    exact
                )
            );
            let (failed, current, _) = request::<T>(n, ends, prior, exact[0], exact[2], true);
            assert!(
                matches!(failed,Err(E::Region(book_v2::BookV2PageRegionLayoutError::Inline(e))) if e.kind==ProductionInlinePreparationErrorKind::AllocationFailure)
            );
            assert_eq!(current, exact);
            if n > 0 {
                for axis in 0..3 {
                    let mut p = prior;
                    let mut mr = exact[0];
                    let mut mw = exact[2];
                    if axis == 0 {
                        mr -= 1;
                    } else if axis == 1 {
                        p[1] += 1;
                    } else {
                        mw -= 1;
                    }
                    let (failed, current, trace) = request::<T>(n, ends, p, mr, mw, false);
                    assert!(
                        matches!(failed,Err(E::Accounting(e)) if e==match axis{0=>A::Records{owner:NodeId::new(17)},1=>A::Spool{owner:NodeId::new(17)},_=>A::Work{owner:NodeId::new(17)}})
                    );
                    assert_eq!(current, [p[0], p[1], p[2] + 1]);
                    assert!(trace.is_empty());
                }
            }
        }
    }
    #[test]
    fn book_v2_observed_region_context_request_axes_faults_and_zero() {
        // Actual shared concrete reserve helpers, not a Source/shape receipt.
        check::<ProductionSelectedParagraphContext>(false);
        check::<u32>(true);
    }
}
