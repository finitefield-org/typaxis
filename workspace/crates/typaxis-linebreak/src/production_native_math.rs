//! Native math is an atomic source occurrence, not a producer SVG binding.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionNativeMathInlineItem {
    owner: NodeId,
    paragraph: NodeId,
    source_span: SourceSpan,
    computation_sha256: [u8; 32],
    receipt_sha256: [u8; 32],
    advance: NonNegativeLength,
    ascent: NonNegativeLength,
    descent: NonNegativeLength,
    left: Length,
    right: Length,
    fingerprint: [u8; 32],
}
impl ProductionNativeMathInlineItem {
    /// Geometry is read only from an issued math computation. The layout owner
    /// binds the source/receipt identity before authorizing selected paint.
    pub fn from_computation(
        owner: NodeId,
        paragraph: NodeId,
        source_span: SourceSpan,
        receipt_sha256: [u8; 32],
        computation: &typaxis_math::MathComputationReceipt,
    ) -> Result<Self, AtomicVectorInlineError> {
        if owner == paragraph || computation.kind() != typaxis_math::MathNodeKind::Inline {
            return Err(AtomicVectorInlineError::InvalidBinding);
        }
        let atom = crate::AtomicMathInlineItem::from_computation(computation)
            .map_err(|_| AtomicVectorInlineError::InvalidBinding)?;
        let length = |v| Length::from_raw(v).ok_or(AtomicVectorInlineError::ArithmeticOverflow);
        let nonnegative = |v| {
            length(v).and_then(|v| {
                NonNegativeLength::new(v).ok_or(AtomicVectorInlineError::InvalidBinding)
            })
        };
        let (left, _, right, _) = computation.dimensions().bbox();
        if left > right {
            return Err(AtomicVectorInlineError::InvalidBinding);
        }
        let mut canonical = format!(
            "typaxis.production-native-math-inline/1/{}/{}/",
            owner.get(),
            paragraph.get()
        );
        push_source_span(&mut canonical, source_span);
        push_hash(&mut canonical, receipt_sha256);
        push_hash(&mut canonical, computation.fingerprint());
        Ok(Self {
            owner,
            paragraph,
            source_span,
            receipt_sha256,
            computation_sha256: computation.fingerprint(),
            advance: nonnegative(atom.advance())?,
            ascent: nonnegative(atom.ascent())?,
            descent: nonnegative(atom.descent())?,
            left: length(left)?,
            right: length(right)?,
            fingerprint: sha256(canonical.as_bytes()),
        })
    }
    pub const fn owner(self) -> NodeId {
        self.owner
    }
    pub const fn paragraph(self) -> NodeId {
        self.paragraph
    }
    pub const fn source_span(self) -> SourceSpan {
        self.source_span
    }
    pub const fn computation_sha256(self) -> [u8; 32] {
        self.computation_sha256
    }
    pub const fn receipt_sha256(self) -> [u8; 32] {
        self.receipt_sha256
    }
    pub const fn advance(self) -> NonNegativeLength {
        self.advance
    }
    pub const fn ascent(self) -> NonNegativeLength {
        self.ascent
    }
    pub const fn descent(self) -> NonNegativeLength {
        self.descent
    }
    pub const fn left(self) -> Length {
        self.left
    }
    pub const fn right(self) -> Length {
        self.right
    }
    pub const fn fingerprint(self) -> [u8; 32] {
        self.fingerprint
    }
}
