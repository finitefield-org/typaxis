//! Actual successor native computations, shared across line/page feedback.
use super::*;
use crate::book_v2::BookV2VectorBindings;
use typaxis_core::FontInstanceId;
use typaxis_resource_admission::{
    AdmittedProductionFontInstancesV3, AdmittedProductionResourceLedgerV3,
};
use typaxis_syntax::book_v2::PreparedBookMath;

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
    )?;
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
        let instance = instances
            .resolve(FontInstanceId::new(selected_face.get()))
            .filter(|i| i.font().font_face_id() == selected_face)
            .ok_or(StagingMathLayoutError::UnknownMathFont(owner))?;
        let font = instance.font();
        let face = match parsed_faces.get(&selected_face) {
            Some(face) => *face,
            None => {
                let face = MathFontFace::parse(font.bytes(), font.face_index())
                    .map_err(|_| StagingMathLayoutError::InvalidMathFont(owner))?;
                parsed_faces.insert(selected_face, face);
                face
            }
        };
        let required = required_math_layout_units(source.parsed()).map_err(map_math_error)?;
        let kind = match source.domain().kind {
            typaxis_document::StagingM4MathKind::Inline => MathNodeKind::Inline,
            typaxis_document::StagingM4MathKind::Display => MathNodeKind::Display,
        };
        let input = MathComputationInput::new(kind, style.font_size().get().raw(), required)
            .ok_or(StagingMathLayoutError::LayoutUnitLimit)?;
        let computation = compute_math(source.parsed(), face, input).map_err(|e| match e {
            MathComputationError::Font(_) => StagingMathLayoutError::InvalidMathFont(owner),
            other => map_math_error(other),
        })?;
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
    Ok(Some(BookV2NativeMath {
        bindings,
        admitted,
        limits_fingerprint: limits.fingerprint(),
        instances,
        receipts,
        display_blocks,
        layout_work: work,
        record_charge,
        spool_charge,
        fingerprint,
    }))
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
