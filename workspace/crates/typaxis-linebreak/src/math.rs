use typaxis_math::MathComputationReceipt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MathAtomicItemError {
    InvalidComputation,
    InvalidLineSize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtomicMathPlacement {
    FitsCurrentLine,
    MoveIntactToNextLine,
    Oversize,
}

/// An inline math computation enters itemization as exactly one object. No
/// method exposes internal break candidates or glyph clusters to the breaker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtomicMathInlineItem {
    computation_fingerprint: [u8; 32],
    advance: i64,
    ascent: i64,
    descent: i64,
}

impl AtomicMathInlineItem {
    pub fn from_computation(
        computation: &MathComputationReceipt,
    ) -> Result<Self, MathAtomicItemError> {
        let dimensions = computation.dimensions();
        if dimensions.advance() <= 0 || dimensions.ascent() <= 0 || dimensions.descent() < 0 {
            return Err(MathAtomicItemError::InvalidComputation);
        }
        Ok(Self {
            computation_fingerprint: computation.fingerprint(),
            advance: dimensions.advance(),
            ascent: dimensions.ascent(),
            descent: dimensions.descent(),
        })
    }

    pub const fn computation_fingerprint(&self) -> [u8; 32] {
        self.computation_fingerprint
    }
    pub const fn advance(&self) -> i64 {
        self.advance
    }
    pub const fn ascent(&self) -> i64 {
        self.ascent
    }
    pub const fn descent(&self) -> i64 {
        self.descent
    }

    pub fn place(
        &self,
        remaining_inline_size: i64,
        empty_line_inline_size: i64,
    ) -> Result<AtomicMathPlacement, MathAtomicItemError> {
        if remaining_inline_size < 0 || empty_line_inline_size <= 0 {
            return Err(MathAtomicItemError::InvalidLineSize);
        }
        if self.advance <= remaining_inline_size {
            Ok(AtomicMathPlacement::FitsCurrentLine)
        } else if self.advance <= empty_line_inline_size {
            Ok(AtomicMathPlacement::MoveIntactToNextLine)
        } else {
            Ok(AtomicMathPlacement::Oversize)
        }
    }
}
