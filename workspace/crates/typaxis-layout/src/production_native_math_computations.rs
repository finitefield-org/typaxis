//! Source-bound native computations for common placement; no page coordinates.
use super::*;
use typaxis_core::{Length, PositiveLength};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductionNativeMathComputationError {
    Math(StagingMathLayoutError),
    RecordLimit,
    SpoolLimit,
}
impl From<StagingMathLayoutError> for ProductionNativeMathComputationError {
    fn from(value: StagingMathLayoutError) -> Self {
        Self::Math(value)
    }
}

impl std::fmt::Display for ProductionNativeMathComputationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Math(error) => error.fmt(f),
            Self::RecordLimit => {
                f.write_str("L5110: native math computation record limit exceeded")
            }
            Self::SpoolLimit => f.write_str("D8101: native math computation spool limit exceeded"),
        }
    }
}
impl std::error::Error for ProductionNativeMathComputationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Math(error) => Some(error),
            _ => None,
        }
    }
}

/// Page-independent geometry derived once from an authorized display computation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionNativeMathDisplayBlock {
    owner: NodeId,
    source_span: SourceSpan,
    style: typaxis_style::ComputedMachineBlockStyle,
    named_page: bool,
    left: Length,
    width: PositiveLength,
    height: PositiveLength,
    baseline: Length,
}
impl ProductionNativeMathDisplayBlock {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn style(&self) -> typaxis_style::ComputedMachineBlockStyle {
        self.style
    }
    pub const fn has_named_page(&self) -> bool {
        self.named_page
    }
    pub const fn left(&self) -> Length {
        self.left
    }
    pub const fn width(&self) -> PositiveLength {
        self.width
    }
    pub const fn height(&self) -> PositiveLength {
        self.height
    }
    pub const fn baseline(&self) -> Length {
        self.baseline
    }
}

pub struct ProductionNativeMathComputations {
    epoch: StagingMathLayoutEpoch,
    limits_fingerprint: [u8; 32],
    receipts: Vec<ValidatedMathReceipt>,
    by_owner: Vec<(NodeId, usize, SourceSpan)>,
    display_blocks: Vec<ProductionNativeMathDisplayBlock>,
    layout_work: u64,
    record_charge: u64,
    spool_charge: u64,
    fingerprint: [u8; 32],
}
impl ProductionNativeMathComputations {
    pub fn display_blocks(&self) -> &[ProductionNativeMathDisplayBlock] {
        &self.display_blocks
    }

    pub fn receipts(&self) -> &[ValidatedMathReceipt] {
        &self.receipts
    }
    pub fn receipt(&self, owner: NodeId) -> Option<&ValidatedMathReceipt> {
        let index = self.by_owner.binary_search_by_key(&owner, |x| x.0).ok()?;
        self.receipts.get(self.by_owner[index].1)
    }
    pub fn source_span(&self, owner: NodeId) -> Option<SourceSpan> {
        let index = self.by_owner.binary_search_by_key(&owner, |x| x.0).ok()?;
        Some(self.by_owner[index].2)
    }
    pub const fn layout_work(&self) -> u64 {
        self.layout_work
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify_source(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingMathProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
    ) -> Result<(), ProductionNativeMathComputationError> {
        profile
            .authorizes(package, limits)
            .map_err(|_| StagingMathLayoutError::ProfileMismatch)?;
        if self.epoch != StagingMathLayoutEpoch::new(package, profile, admitted)
            || self.limits_fingerprint != limits.fingerprint()
        {
            return Err(StagingMathLayoutError::ReceiptMismatch.into());
        }
        Ok(())
    }
}

pub(super) fn native_display_block(
    owner: NodeId, source_span: SourceSpan, style: &StagingMathComputedStyle,
    computation: &MathComputationReceipt,
) -> Result<ProductionNativeMathDisplayBlock, StagingMathLayoutError> {
        let dimensions = computation.dimensions();
        let left = dimensions.bbox().0.min(0);
        let right = dimensions.bbox().2.max(dimensions.advance());
        let natural_height = checked_add(dimensions.ascent(), dimensions.descent())?;
        let height = natural_height.max(style.line_height().get().raw());
        let baseline = checked_add(
            round_half_even(checked_sub(height, natural_height)?)?,
            dimensions.ascent(),
        )?;
        let positive = |value| {
            Length::from_raw(value)
                .and_then(PositiveLength::new)
                .ok_or(StagingMathLayoutError::ArithmeticOverflow)
        };
        Ok(ProductionNativeMathDisplayBlock {
            owner: owner,
            source_span: source_span,
            style: style.block_style(),
            named_page: style.page_name().is_some(),
            left: Length::from_raw(left).ok_or(StagingMathLayoutError::ArithmeticOverflow)?,
            width: positive(checked_sub(right, left)?)?,
            height: positive(height)?,
            baseline: Length::from_raw(baseline)
                .ok_or(StagingMathLayoutError::ArithmeticOverflow)?,
        })
}

pub(super) fn native_storage_budget<'a>(
    nodes: impl IntoIterator<Item = (&'a typaxis_math::ParsedMathReceipt, &'a str)>,
    limits: &M4EffectiveResourceLimits,
    prior_records: u64,
    prior_spool: u64,
) -> Result<(u64,u64,u64), ProductionNativeMathComputationError> {
    use ProductionNativeMathComputationError as E;
    let mut count = 0u64;
    let mut work = 0u64;
    let mut language_bytes = 0u64;
    for (parsed, language) in nodes {
        count = count.checked_add(1).ok_or(E::RecordLimit)?;
        work = work
            .checked_add(required_math_layout_units(parsed).map_err(map_math_error)?)
            .ok_or(E::RecordLimit)?;
        language_bytes = language_bytes
            .checked_add(language.len() as u64)
            .ok_or(E::SpoolLimit)?;
    }
    if work > limits.extension().get().max_math_layout_units {
        return Err(StagingMathLayoutError::LayoutUnitLimit.into());
    }
    // Work bounds the layout boxes, temporary paints and retained paints.
    // Per-node allowance covers receipts, owner lookup, cached faces and display metrics.
    let record_charge = work
        .checked_mul(4)
        .and_then(|n| count.checked_mul(4).and_then(|c| n.checked_add(c)))
        .and_then(|n| n.checked_add(4))
        .and_then(|n| n.checked_add(prior_records))
        .filter(|n| *n <= limits.base().get().max_fragments)
        .ok_or(E::RecordLimit)?;
    // Canonicals contain fixed-width hashes/numbers plus escaped language.
    // The work allowance also covers transient geometry and fingerprint bytes.
    let spool_charge = work
        .checked_mul(2048)
        .and_then(|n| count.checked_mul(8192).and_then(|c| n.checked_add(c)))
        .and_then(|n| language_bytes.checked_mul(6).and_then(|c| n.checked_add(c)))
        .and_then(|n| n.checked_add(1024))
        .and_then(|n| n.checked_add(prior_spool))
        .filter(|n| *n <= limits.base().get().max_spool_bytes)
        .ok_or(E::SpoolLimit)?;
    Ok((work, record_charge, spool_charge))
}

/// Reserve computation storage before parsing fonts or computing any equation.
/// Charges are cumulative: callers must retain them through common placement.
pub fn compute_production_native_math(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingMathProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    prior_records: u64,
    prior_spool: u64,
) -> Result<ProductionNativeMathComputations, ProductionNativeMathComputationError> {
    use ProductionNativeMathComputationError as E;
    profile
        .authorizes(package, limits)
        .map_err(|_| StagingMathLayoutError::ProfileMismatch)?;
    let (work, record_charge, spool_charge) = native_storage_budget(
        package.math_nodes().iter().map(|n| (n.parsed(), n.domain().language.as_str())),
        limits, prior_records, prior_spool,
    )?;
    let epoch = StagingMathLayoutEpoch::new(package, profile, admitted);
    let (receipts, layout_work) =
        compute_math_receipts(package, profile, limits, &epoch, admitted)?;
    if work != layout_work {
        return Err(StagingMathLayoutError::ReceiptMismatch.into());
    }
    let mut display_blocks = Vec::new();
    display_blocks
        .try_reserve_exact(
            receipts
                .iter()
                .filter(|r| r.kind() == MathNodeKind::Display)
                .count(),
        )
        .map_err(|_| StagingMathLayoutError::AllocationFailure)?;
    for (receipt, source) in receipts.iter().zip(package.math_nodes()) {
        if receipt.kind() != MathNodeKind::Display {
            continue;
        }
        display_blocks.push(native_display_block(receipt.node_id(), source.domain().span, source.computed_style(), receipt.computation())?);
    }
    let mut by_owner = Vec::new();
    by_owner
        .try_reserve_exact(receipts.len())
        .map_err(|_| StagingMathLayoutError::AllocationFailure)?;
    let mut canonical = Vec::new();
    let canonical_size = receipts
        .len()
        .checked_mul(32)
        .and_then(|n| n.checked_add(128))
        .ok_or(E::SpoolLimit)?;
    canonical
        .try_reserve_exact(canonical_size)
        .map_err(|_| StagingMathLayoutError::AllocationFailure)?;
    canonical.extend_from_slice(&sha256(b"typaxis.production-native-math-computations/1"));
    canonical.extend_from_slice(&epoch.fingerprint());
    canonical.extend_from_slice(&limits.fingerprint());
    for (index, (receipt, source)) in receipts.iter().zip(package.math_nodes()).enumerate() {
        if receipt.node_id() != source.domain().node_id {
            return Err(StagingMathLayoutError::ReceiptMismatch.into());
        }
        by_owner.push((receipt.node_id(), index, source.domain().span));
        canonical.extend_from_slice(&receipt.key().bytes());
    }
    by_owner.sort_unstable_by_key(|x| x.0);
    if by_owner.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(StagingMathLayoutError::ReceiptMismatch.into());
    }
    Ok(ProductionNativeMathComputations {
        epoch,
        limits_fingerprint: limits.fingerprint(),
        receipts,
        by_owner,
        display_blocks,
        layout_work,
        record_charge,
        spool_charge,
        fingerprint: sha256(&canonical),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_native_computations_preserve_receipts_and_cumulative_budgets() {
        let f = staging_math_layout_fixture().unwrap();
        let compute = |records, spool| {
            compute_production_native_math(
                &f.package,
                &f.profile,
                &f.limits,
                &f.admitted,
                records,
                spool,
            )
        };
        let initial = compute(0, 0).unwrap();
        assert_eq!(initial.receipts(), f.layout.receipts());
        assert_eq!(initial.layout_work(), f.layout.total_layout_work());
        initial
            .verify_source(&f.package, &f.profile, &f.limits, &f.admitted)
            .unwrap();
        for receipt in f.layout.receipts() {
            assert_eq!(initial.receipt(receipt.node_id()), Some(receipt));
        }
        assert!(initial.receipt(NodeId::new(u32::MAX)).is_none());
        let records = f.limits.base().get().max_fragments - initial.record_charge();
        let spool = f.limits.base().get().max_spool_bytes - initial.spool_charge();
        let exact = compute(records, spool).unwrap();
        assert_eq!(exact.record_charge(), f.limits.base().get().max_fragments);
        assert_eq!(exact.spool_charge(), f.limits.base().get().max_spool_bytes);
        assert_eq!(exact.fingerprint(), initial.fingerprint());
        assert!(matches!(
            compute(records + 1, spool),
            Err(ProductionNativeMathComputationError::RecordLimit)
        ));
        assert!(matches!(
            compute(records, spool + 1),
            Err(ProductionNativeMathComputationError::SpoolLimit)
        ));
        assert!(matches!(
            compute(u64::MAX, 0),
            Err(ProductionNativeMathComputationError::RecordLimit)
        ));
        assert!(matches!(
            compute(0, u64::MAX),
            Err(ProductionNativeMathComputationError::SpoolLimit)
        ));
        let foreign = staging_math_layout_fixture_for("page-document-package.json").unwrap();
        assert!(initial
            .verify_source(
                &foreign.package,
                &foreign.profile,
                &foreign.limits,
                &foreign.admitted
            )
            .is_err());
    }
}
