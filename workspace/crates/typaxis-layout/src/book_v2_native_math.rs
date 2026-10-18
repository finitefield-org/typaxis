//! Actual successor native computations, shared across line/page feedback.
pub use super::ProductionNativeMathBudgetObservation as BookV2NativeMathBudgetObservation;
use super::*;
use crate::book_v2::BookV2VectorBindings;
use typaxis_core::FontInstanceId;
use typaxis_resource_admission::{
    AdmittedProductionFontInstancesV3, AdmittedProductionResourceLedgerV3,
};
use typaxis_syntax::book_v2::PreparedBookMath;
#[path="book_v2_native_cff_math.rs"]
mod cff_math;
use cff_math::{ComputeFailure,NativeFace};
pub use cff_math::compute_book_v2_native_math_cff_observed_counted;


pub const BOOK_V2_NATIVE_MATH_ALGORITHM: &str = "typaxis.book-2-native-math/1";
pub const BOOK_V2_NATIVE_MATH_SET_ALGORITHM: &str = "typaxis.book-2-native-math-set/1";

#[derive(Debug)]
pub struct BookV2MathReceipt<'a> {
    source: &'a PreparedBookMath,
    style: &'a StagingMathComputedStyle,
    font_face_id: FontFaceId,
    font_instance_id: FontInstanceId,
    font_sha256: [u8; 32],
    face_index: u32,
    computation: MathComputationReceipt,
    fingerprint: [u8; 32],
}
impl<'a> BookV2MathReceipt<'a> {
    pub fn source(&self) -> &'a PreparedBookMath {
        self.source
    }
    pub fn style(&self) -> &'a StagingMathComputedStyle {
        self.style
    }
    pub fn node_id(&self) -> NodeId {
        self.source.domain().node_id
    }
    pub fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub fn font_instance_id(&self) -> FontInstanceId {
        self.font_instance_id
    }
    pub fn font_sha256(&self) -> [u8; 32] {
        self.font_sha256
    }
    pub fn face_index(&self) -> u32 {
        self.face_index
    }
    pub fn computation(&self) -> &MathComputationReceipt {
        &self.computation
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// Sealed native preflight bound to the exact immutable inputs.
/// This is a reservation plan, not a computation receipt. Font instances,
/// MATH faces and geometry are constructed only when the plan is consumed.
#[must_use]
pub struct BookV2NativeMathPreflight<'a> {
    bindings: &'a BookV2VectorBindings<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits_fingerprint: [u8; 32],
    work: u64,
    record_charge: u64,
    spool_charge: u64,
}
impl BookV2NativeMathPreflight<'_> {
    pub fn reserved_layout_units(&self) -> u64 {
        self.work
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
}
impl std::fmt::Debug for BookV2NativeMathPreflight<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BookV2NativeMathPreflight")
            .field("reserved_layout_units", &self.work)
            .field("record_charge", &self.record_charge)
            .field("spool_charge", &self.spool_charge)
            .finish_non_exhaustive()
    }
}

pub struct BookV2NativeMath<'a> {
    bindings: &'a BookV2VectorBindings<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits_fingerprint: [u8; 32],
    instances: AdmittedProductionFontInstancesV3<'a>,
    receipts: Vec<BookV2MathReceipt<'a>>,
    display_blocks: Vec<ProductionNativeMathDisplayBlock>,
    layout_work: u64,
    record_charge: u64,
    spool_charge: u64,
    fingerprint: [u8; 32],
}
impl<'a> BookV2NativeMath<'a> {
    pub fn receipts(&self) -> &[BookV2MathReceipt<'a>] {
        &self.receipts
    }
    pub fn receipt(&self, owner: NodeId) -> Option<&BookV2MathReceipt<'a>> {
        self.receipts
            .binary_search_by_key(&owner, BookV2MathReceipt::node_id)
            .ok()
            .map(|i| &self.receipts[i])
    }
    pub fn source_span(&self, owner: NodeId) -> Option<SourceSpan> {
        Some(self.receipt(owner)?.source().domain().span)
    }
    pub fn display_blocks(&self) -> &[ProductionNativeMathDisplayBlock] {
        &self.display_blocks
    }
    pub fn font_instances(&self) -> &AdmittedProductionFontInstancesV3<'a> {
        &self.instances
    }
    pub fn layout_work(&self) -> u64 {
        self.layout_work
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        bindings: &BookV2VectorBindings<'_>,
        admitted: &AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionNativeMathComputationError> {
        if !std::ptr::eq(self.bindings, bindings)
            || !std::ptr::eq(self.admitted, admitted)
            || self.limits_fingerprint != limits.fingerprint()
        {
            return Err(StagingMathLayoutError::ReceiptMismatch.into());
        }
        Ok(())
    }
}
impl std::fmt::Debug for BookV2NativeMath<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BookV2NativeMath")
            .field("receipts", &self.receipts.len())
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}
fn fold_hash(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    let mut bytes = [0; 64];
    bytes[..32].copy_from_slice(&a);
    bytes[32..].copy_from_slice(&b);
    sha256(&bytes)
}
/// Preflight every computation's work and retained records/spool before parsing
/// any MATH face. A native-free source receives no fabricated computation set.
pub fn compute_book_v2_native_math<'a>(
    bindings: &'a BookV2VectorBindings<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    prior_records: u64,
    prior_spool: u64,
) -> Result<Option<BookV2NativeMath<'a>>, ProductionNativeMathComputationError> {
    compute_book_v2_native_math_counted(
        bindings,
        admitted,
        limits,
        prior_records,
        prior_spool,
        &mut BookV2NativeMathBudgetObservation::default(),
    )
}

/// Retain the caller's history and accepted preflight reservations on all exits.
/// Font-table prefixes are included only when the shared storage bound accepts
/// them; arithmetic or identity rejection does not consume a reservation.
pub fn compute_book_v2_native_math_counted<'a>(
    bindings: &'a BookV2VectorBindings<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    prior_records: u64,
    prior_spool: u64,
    observed: &mut BookV2NativeMathBudgetObservation,
) -> Result<Option<BookV2NativeMath<'a>>, ProductionNativeMathComputationError> {
    preflight_book_v2_native_math_counted(
        bindings,
        admitted,
        limits,
        prior_records,
        prior_spool,
        observed,
    )?
    .map(|plan| compute_preflighted_book_v2_native_math(plan, limits))
    .transpose()
}

/// Reserve native storage locally without creating font instances or parsing
/// MATH faces. Callers can accept the observed bounds into a command budget
/// before consuming the sealed plan. Failed reservations retain their prefix.
pub fn preflight_book_v2_native_math_counted<'a>(
    bindings: &'a BookV2VectorBindings<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    prior_records: u64,
    prior_spool: u64,
    observed: &mut BookV2NativeMathBudgetObservation,
) -> Result<Option<BookV2NativeMathPreflight<'a>>, ProductionNativeMathComputationError> {
    *observed = BookV2NativeMathBudgetObservation::new(prior_records, prior_spool);
    bindings
        .verify(bindings.body(), admitted, limits)
        .map_err(|_| StagingMathLayoutError::ReceiptMismatch)?;
    let styled = bindings.body().styled();
    let nodes = styled.body().math();
    if nodes.is_empty() {
        return Ok(None);
    }
    // The canonical all-face instance table retains its own face records and
    // builds a temporary ordered set/canonical. Charge these in addition to
    // native per-node storage, before allocating the table.
    let face_count = u64::try_from(admitted.fonts().len())
        .map_err(|_| ProductionNativeMathComputationError::RecordLimit)?;
    let prior_records = face_count
        .checked_mul(2)
        .and_then(|n| n.checked_add(1))
        .and_then(|n| n.checked_add(prior_records))
        .ok_or(ProductionNativeMathComputationError::RecordLimit)?;
    let prior_spool = face_count
        .checked_mul(15)
        .and_then(|n| n.checked_add(256))
        .and_then(|n| n.checked_add(prior_spool))
        .ok_or(ProductionNativeMathComputationError::SpoolLimit)?;
    let (work, record_charge, spool_charge) = production_computations::native_storage_budget(
        nodes
            .iter()
            .map(|n| (n.parsed(), n.domain().language.as_str())),
        limits,
        prior_records,
        prior_spool,
        observed,
    )?;
    Ok(Some(BookV2NativeMathPreflight {
        bindings,
        admitted,
        limits_fingerprint: limits.fingerprint(),
        work,
        record_charge,
        spool_charge,
    }))
}

/// Consume the preflight using its bound source and resources. A different
/// limits identity is rejected before any native allocation or font parsing.
pub fn compute_preflighted_book_v2_native_math<'a>(
    preflight: BookV2NativeMathPreflight<'a>,
    limits: &M4EffectiveResourceLimits,
) -> Result<BookV2NativeMath<'a>, ProductionNativeMathComputationError> {
    match compute_preflighted_kernel(preflight,limits,&mut cff_math::NoCff,false) {
        Ok(value)=>Ok(value),Err(ComputeFailure::Native(error))=>Err(error),Err(ComputeFailure::Access(never))=>match never{},
    }
}
fn compute_preflighted_kernel<'a,P:cff_math::NativeProvider>(
    preflight:BookV2NativeMathPreflight<'a>,limits:&M4EffectiveResourceLimits,provider:&mut P,allow_cff:bool,
)->Result<BookV2NativeMath<'a>,ComputeFailure<P::Error>>{
    let BookV2NativeMathPreflight {
        bindings,
        admitted,
        limits_fingerprint,
        work,
        record_charge,
        spool_charge,
    } = preflight;
    if limits_fingerprint != limits.fingerprint() {
        return Err(StagingMathLayoutError::ReceiptMismatch.into());
    }
    let styled = bindings.body().styled();
    let nodes = styled.body().math();
    let instances = AdmittedProductionFontInstancesV3::from_used_faces(
        admitted,
        admitted.fonts().iter().map(|f| f.font_face_id()),
    )
    .map_err(|_| StagingMathLayoutError::ReceiptMismatch)?;
    let mut receipts = Vec::new();
    receipts
        .try_reserve_exact(nodes.len())
        .map_err(|_| StagingMathLayoutError::AllocationFailure)?;
    let mut display_blocks = Vec::new();
    display_blocks
        .try_reserve_exact(
            nodes
                .iter()
                .filter(|n| n.domain().kind == typaxis_document::StagingM4MathKind::Display)
                .count(),
        )
        .map_err(|_| StagingMathLayoutError::AllocationFailure)?;
    let mut parsed_faces = BTreeMap::new();
    let mut previous = None;
    let mut actual_work = 0u64;
    let mut fingerprint = fold_hash(
        sha256(BOOK_V2_NATIVE_MATH_SET_ALGORITHM.as_bytes()),
        bindings.epoch(),
    );
    fingerprint = fold_hash(fingerprint, instances.fingerprint());
    for source in nodes {
        let owner = source.domain().node_id;
        provider.owner(owner);
        if previous.is_some_and(|p| p >= owner) {
            return Err(StagingMathLayoutError::ReceiptMismatch.into());
        }
        previous = Some(owner);
        let style = styled
            .math_style(owner)
            .ok_or(StagingMathLayoutError::ProfileMismatch)?;
        let selected_face = admitted
            .font_families()
            .resolve(style.font_families())
            .map_err(|_| StagingMathLayoutError::UnknownMathFont(owner))?;
        provider.face(selected_face);
        let instance = instances
            .resolve(FontInstanceId::new(selected_face.get()))
            .filter(|i| i.font().font_face_id() == selected_face)
            .ok_or(StagingMathLayoutError::UnknownMathFont(owner))?;
        let font = instance.font();
        let face = match parsed_faces.get(&selected_face) {
            Some(face) => *face,
            None => {
                let face=match font {
                    typaxis_resource_admission::AdmittedProductionFontV3::TrueType(_)=>NativeFace::TrueType(
                        MathFontFace::parse(font.bytes(),font.face_index()).map_err(|_|StagingMathLayoutError::InvalidMathFont(owner))?),
                    typaxis_resource_admission::AdmittedProductionFontV3::Cff1V2(source)=>{
                        if !allow_cff{return Err(StagingMathLayoutError::InvalidMathFont(owner).into());}
                        let face=typaxis_font::CffMathFontFaceV2::from_admission_with_work(source.admission(),&mut |n|provider.work(n))
                            .map_err(ComputeFailure::Access)?.map_err(|_|StagingMathLayoutError::InvalidMathFont(owner))?;
                        NativeFace::Cff(face)
                    }
                };
                parsed_faces.insert(selected_face, face);face
            }
        };
        let required = required_math_layout_units(source.parsed()).map_err(map_math_error)?;
        let kind = match source.domain().kind {
            typaxis_document::StagingM4MathKind::Inline => MathNodeKind::Inline,
            typaxis_document::StagingM4MathKind::Display => MathNodeKind::Display,
        };
        let input = MathComputationInput::new(kind, style.font_size().get().raw(), required)
            .ok_or(StagingMathLayoutError::LayoutUnitLimit)?;
        let computation=match face {
            NativeFace::TrueType(face)=>compute_math(source.parsed(),face,input),
            NativeFace::Cff(face)=>match typaxis_math::compute_math_cff_v2(source.parsed(),face,input,provider){
                Ok(value)=>Ok(value),Err(typaxis_math::CffMathComputationErrorV2::Computation(error))=>Err(error),
                Err(typaxis_math::CffMathComputationErrorV2::Access(error))=>return Err(ComputeFailure::Access(error)),
            },
        }.map_err(|e|match e {MathComputationError::Font(_)=>StagingMathLayoutError::InvalidMathFont(owner),other=>map_math_error(other)})?;
        if computation.layout_work() != required {
            return Err(StagingMathLayoutError::ReceiptMismatch.into());
        }
        actual_work = actual_work
            .checked_add(required)
            .ok_or(StagingMathLayoutError::LayoutUnitLimit)?;
        if kind == MathNodeKind::Display {
            display_blocks.push(production_computations::native_display_block(
                owner,
                source.domain().span,
                style,
                &computation,
            )?);
        }
        let mut key = fold_hash(
            sha256(BOOK_V2_NATIVE_MATH_ALGORITHM.as_bytes()),
            bindings.epoch(),
        );
        key = fold_hash(key, sha256(&owner.get().to_be_bytes()));
        for value in [
            instances.fingerprint(),
            font.content_hash(),
            math_style_fingerprint(style),
            computation.fingerprint(),
        ] {
            key = fold_hash(key, value);
        }
        fingerprint = fold_hash(fingerprint, key);
        receipts.push(BookV2MathReceipt {
            source,
            style,
            font_face_id: selected_face,
            font_instance_id: instance.font_instance_id(),
            font_sha256: font.content_hash(),
            face_index: font.face_index(),
            computation,
            fingerprint: key,
        });
    }
    if actual_work != work {
        return Err(StagingMathLayoutError::ReceiptMismatch.into());
    }
    Ok(BookV2NativeMath {
        bindings,
        admitted,
        limits_fingerprint,
        instances,
        receipts,
        display_blocks,
        layout_work: work,
        record_charge,
        spool_charge,
        fingerprint,
    })
}

#[derive(Debug)]
pub struct BookV2PlacedInlineMath<'p, 'a> {
    receipt: &'p BookV2MathReceipt<'a>,
    pen_x: typaxis_core::Length,
    baseline: typaxis_core::Length,
}
impl<'p, 'a> BookV2PlacedInlineMath<'p, 'a> {
    pub(crate) fn new(
        receipt: &'p BookV2MathReceipt<'a>,
        pen_x: typaxis_core::Length,
        baseline: typaxis_core::Length,
    ) -> Self {
        Self {
            receipt,
            pen_x,
            baseline,
        }
    }
    pub fn receipt(&self) -> &'p BookV2MathReceipt<'a> {
        self.receipt
    }
    pub fn owner(&self) -> NodeId {
        self.receipt.node_id()
    }
    pub fn source_span(&self) -> SourceSpan {
        self.receipt.source().domain().span
    }
    pub fn pen_x(&self) -> typaxis_core::Length {
        self.pen_x
    }
    pub fn baseline(&self) -> typaxis_core::Length {
        self.baseline
    }
}

/// A native computation whose exact local preflight was accepted by this loan.
/// The immutable computation remains bound to its original source/resources.
/// No constructor can turn a raw computation or numerical prefix into payment.
pub struct BookV2ObservedNativeMath<'id, 'a> {
    inner: BookV2NativeMath<'a>,
    local_records: u64,
    local_spool: u64,
    local_work: u64,
    identity: std::marker::PhantomData<fn(&'id mut ()) -> &'id mut ()>,
}
impl<'id, 'a> BookV2ObservedNativeMath<'id, 'a> {
    pub fn math(&self) -> &BookV2NativeMath<'a> {
        &self.inner
    }
    pub fn reserved_records(&self) -> u64 {
        self.local_records
    }
    pub fn reserved_spool(&self) -> u64 {
        self.local_spool
    }
    pub fn reserved_layout_units(&self) -> u64 {
        self.local_work
    }
}

#[derive(Debug)]
pub enum BookV2ObservedNativeMathError {
    Native(ProductionNativeMathComputationError),
    Accounting(typaxis_shaping::book_v2::BookV2ObservedAccountingError),
    Cff { owner:NodeId, font_face_id:FontFaceId,error:typaxis_font::CffSelectionFailureV2 },
}
impl std::fmt::Display for BookV2ObservedNativeMathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 observed native math: {self:?}")
    }
}
impl std::error::Error for BookV2ObservedNativeMathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Native(error) => Some(error),
            Self::Accounting(error) => Some(error),
            Self::Cff { error,.. }=>Some(error),
        }
    }
}

/// Accept the real native preflight on the existing caller cells before any
/// font-instance allocation, MATH parsing or computation. Each axis is evaluated
/// independently in the original work/records/spool order, even after refusal.
///
/// ```
/// use typaxis_core::M4EffectiveResourceLimits;
/// use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
/// use typaxis_layout::book_v2::{BookV2VectorBindings, compute_book_v2_native_math_observed_counted};
/// use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;
/// fn inspect<'a>(bindings: &'a BookV2VectorBindings<'a>, admitted: &'a AdmittedProductionResourceLedgerV3,
///                limits: &M4EffectiveResourceLimits) {
///     let (mut r, mut s, mut w) = (0, 0, 0);
///     with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, limits.base(),
///         limits.base().get().max_fragments, 1_000_000, |loan| {
///             if let Ok(Some(native)) = compute_book_v2_native_math_observed_counted(bindings, admitted, limits, loan) {
///                 assert_eq!(native.math().record_charge(), native.reserved_records());
///             }
///         });
/// }
/// ```
///
/// A paid owner cannot escape its fresh caller loan.
/// ```compile_fail
/// use typaxis_core::M4EffectiveResourceLimits;
/// use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
/// use typaxis_layout::book_v2::{BookV2VectorBindings, compute_book_v2_native_math_observed_counted};
/// use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;
/// fn escape<'a>(bindings: &'a BookV2VectorBindings<'a>, admitted: &'a AdmittedProductionResourceLedgerV3,
///               limits: &M4EffectiveResourceLimits) {
///     let (mut r, mut s, mut w) = (0, 0, 0);
///     let escaped = with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, limits.base(),
///         limits.base().get().max_fragments, 1_000_000, |loan|
///             compute_book_v2_native_math_observed_counted(bindings, admitted, limits, loan).unwrap());
///     let _ = escaped;
/// }
/// ```
///
/// Native payment from a different loan cannot be used for an inline graph.
/// ```compile_fail
/// use typaxis_core::M4EffectiveResourceLimits;
/// use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
/// use typaxis_layout::book_v2::{BookV2VectorBindings, compute_book_v2_native_math_observed_counted,
///     prepare_book_v2_inline_items_with_observed_native_source_counted};
/// use typaxis_linebreak::JapaneseLineBreakMode;
/// use typaxis_shaping::book_v2::{BookV2AuthoredTextShape, with_book_v2_observed_source_accounting};
/// use typaxis_syntax::book_v2::PreparedBookV2TextFlow;
/// fn cross<'a>(flow: &'a PreparedBookV2TextFlow<'a>, shape: &'a BookV2AuthoredTextShape<'a>,
///              bindings: &'a BookV2VectorBindings<'a>, admitted: &'a AdmittedProductionResourceLedgerV3,
///              limits: &M4EffectiveResourceLimits, mode: JapaneseLineBreakMode) {
///     let (mut r, mut s, mut w, mut r2, mut s2, mut w2) = (0, 0, 0, 0, 0, 0);
///     with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, limits.base(),
///         limits.base().get().max_fragments, 1_000_000, |first| {
///             let native = compute_book_v2_native_math_observed_counted(bindings, admitted, limits, first).unwrap();
///             with_book_v2_observed_source_accounting(&mut r2, &mut s2, &mut w2, limits.base(),
///                 limits.base().get().max_fragments, 1_000_000, |second| {
///                     second.with_zero_intrinsic_records(|second, graph|
///                         prepare_book_v2_inline_items_with_observed_native_source_counted(flow, shape,
///                             admitted, bindings, limits, mode, native.as_ref(), second, graph, &mut 0).map(|_| ()));
///                 });
///         });
/// }
/// ```
///
/// A raw native computation has no public payment constructor.
/// ```compile_fail
/// use typaxis_layout::book_v2::{BookV2NativeMath, BookV2ObservedNativeMath};
/// fn forge<'id, 'a>(raw: BookV2NativeMath<'a>) {
///     let _ = BookV2ObservedNativeMath {
///         inner: raw, local_records: 0, local_spool: 0, local_work: 0,
///         identity: std::marker::PhantomData,
///     };
/// }
/// ```
///
/// Payment owners cannot be cloned.
/// ```compile_fail
/// use typaxis_layout::book_v2::BookV2ObservedNativeMath;
/// fn duplicate<'id, 'a>(native: &BookV2ObservedNativeMath<'id, 'a>) -> BookV2ObservedNativeMath<'id, 'a> {
///     native.clone()
/// }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn compute_book_v2_native_math_observed_counted<'id, 'a, 'cells>(
    bindings: &'a BookV2VectorBindings<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
) -> Result<Option<BookV2ObservedNativeMath<'id, 'a>>, BookV2ObservedNativeMathError> {
    use BookV2ObservedNativeMathError as E;
    // Bindings/policy errors keep their original precedence over loan domains.
    bindings
        .verify(bindings.body(), admitted, limits)
        .map_err(|_| E::Native(StagingMathLayoutError::ReceiptMismatch.into()))?;
    accounting
        .require_limit_domain(limits.base())
        .map_err(E::Accounting)?;
    accounting
        .require_limit_domain(bindings.body().styled().body().limits())
        .map_err(E::Accounting)?;
    let mut observed = BookV2NativeMathBudgetObservation::default();
    let plan =
        preflight_book_v2_native_math_counted(bindings, admitted, limits, 0, 0, &mut observed);
    let root = NodeId::new(0);
    let work = accounting.debit_work(root, observed.reserved_layout_units());
    let records = accounting.debit_records(root, observed.record_charge());
    let spool = accounting.debit_spool(root, observed.spool_charge());
    let plan = plan.map_err(E::Native)?;
    work.map_err(E::Accounting)?;
    records.map_err(E::Accounting)?;
    spool.map_err(E::Accounting)?;
    plan.map(|plan| {
        let inner = compute_preflighted_book_v2_native_math(plan, limits).map_err(E::Native)?;
        Ok(BookV2ObservedNativeMath {
            inner,
            local_records: observed.record_charge(),
            local_spool: observed.spool_charge(),
            local_work: observed.reserved_layout_units(),
            identity: std::marker::PhantomData,
        })
    })
    .transpose()
}
