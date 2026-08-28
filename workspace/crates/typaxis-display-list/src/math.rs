use typaxis_core::{push_jcs_string, sha256, FontFaceId, M4EffectiveResourceLimits, NodeId};
use typaxis_layout::{MathReceiptKey, StagingMathLayout};
use typaxis_math::{math_vector_fingerprint, MathPaint};
use typaxis_resource_admission::{AdmittedResourceLedger, ResourceAdmissionProgressToken};
use typaxis_syntax::{
    StagingMathProfileAuthorization, StagingMathProfileProgressToken,
    ValidatedStagingSemanticPackage,
};

pub const MATH_DISPLAY_ALGORITHM: &str = "typaxis.math-display/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathDraw {
    occurrence: u32,
    node_id: NodeId,
    receipt_key: MathReceiptKey,
    font_face_id: FontFaceId,
    font_sha256: [u8; 32],
    page_index: u32,
    frame_index: u32,
    paint_ordinal: u32,
    origin_x: i64,
    baseline_y: i64,
    actual_text: String,
    paints: Vec<MathPaint>,
    vector_fingerprint: [u8; 32],
    fingerprint: [u8; 32],
}

impl StagingMathDraw {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn receipt_key(&self) -> MathReceiptKey {
        self.receipt_key
    }
    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub const fn font_sha256(&self) -> [u8; 32] {
        self.font_sha256
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn frame_index(&self) -> u32 {
        self.frame_index
    }
    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }
    pub const fn origin_x(&self) -> i64 {
        self.origin_x
    }
    pub const fn baseline_y(&self) -> i64 {
        self.baseline_y
    }
    pub fn actual_text(&self) -> &str {
        &self.actual_text
    }
    pub fn paints(&self) -> &[MathPaint] {
        &self.paints
    }
    pub const fn vector_fingerprint(&self) -> [u8; 32] {
        self.vector_fingerprint
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathDisplay {
    layout_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    profile_progress: StagingMathProfileProgressToken,
    admitted_fingerprint: [u8; 32],
    admission_progress: ResourceAdmissionProgressToken,
    draws: Vec<StagingMathDraw>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathDisplay {
    pub fn draws(&self) -> &[StagingMathDraw] {
        &self.draws
    }
    pub const fn layout_fingerprint(&self) -> [u8; 32] {
        self.layout_fingerprint
    }
    pub const fn admitted_fingerprint(&self) -> [u8; 32] {
        self.admitted_fingerprint
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
    pub const fn profile_progress(&self) -> &StagingMathProfileProgressToken {
        &self.profile_progress
    }
    pub const fn admission_progress(&self) -> &ResourceAdmissionProgressToken {
        &self.admission_progress
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    /// Checks the sealed Display artifact without reopening layout. This is
    /// the dependency-inversion boundary consumed by the PDF backend.
    pub fn verify_sealed(&self) -> Result<(), StagingMathDisplayError> {
        for (index, draw) in self.draws.iter().enumerate() {
            if usize::try_from(draw.occurrence) != Ok(index)
                || draw.paints.is_empty()
                || math_vector_fingerprint(&draw.paints) != draw.vector_fingerprint
                || sha256(encode_draw(draw).as_bytes()) != draw.fingerprint
            {
                return Err(StagingMathDisplayError::ReceiptMismatch);
            }
        }
        let canonical_jcs = encode_display(
            self.layout_fingerprint,
            self.profile_fingerprint,
            self.admitted_fingerprint,
            &self.draws,
        );
        if self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
        {
            return Err(StagingMathDisplayError::ReceiptMismatch);
        }
        Ok(())
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingMathProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        layout: &StagingMathLayout,
    ) -> Result<(), StagingMathDisplayError> {
        layout
            .verify(package, profile, limits, admitted)
            .map_err(|_| StagingMathDisplayError::LayoutMismatch)?;
        if self.layout_fingerprint != layout.fingerprint()
            || self.profile_fingerprint != profile.profile_receipt_fingerprint()
            || !profile.matches_progress(&self.profile_progress)
            || &self.profile_progress != layout.profile_progress()
            || self.admitted_fingerprint != layout.epoch().admitted_fingerprint()
            || self.admitted_fingerprint != admitted.fingerprint().bytes()
            || &self.admission_progress != layout.admission_progress()
            || !admitted.token().matches_progress(&self.admission_progress)
            || self.draws.len() != layout.placements().len()
            || self.draws.len() != package.math_nodes().len()
        {
            return Err(StagingMathDisplayError::ReceiptMismatch);
        }
        for (index, ((draw, placement), node)) in self
            .draws
            .iter()
            .zip(layout.placements())
            .zip(package.math_nodes())
            .enumerate()
        {
            let receipt = layout
                .receipt(draw.receipt_key)
                .ok_or(StagingMathDisplayError::ReceiptMismatch)?;
            if usize::try_from(draw.occurrence) != Ok(index)
                || draw.node_id != node.domain().node_id
                || draw.node_id != placement.node_id()
                || draw.receipt_key != placement.receipt_key()
                || draw.font_face_id != receipt.font_face_id()
                || draw.font_sha256 != receipt.font_sha256()
                || draw.page_index != placement.page_index()
                || draw.frame_index != placement.frame_index()
                || draw.paint_ordinal != placement.paint_ordinal()
                || draw.origin_x != placement.origin_x()
                || draw.baseline_y != placement.baseline_y()
                || draw.actual_text != node.domain().speech
                || sha256(draw.actual_text.as_bytes()) != receipt.speech_sha256()
                || draw.paints != receipt.computation().paints()
                || draw.vector_fingerprint != receipt.computation().vector_fingerprint()
                || sha256(encode_draw(draw).as_bytes()) != draw.fingerprint
            {
                return Err(StagingMathDisplayError::ReceiptMismatch);
            }
        }
        let canonical_jcs = encode_display(
            layout.fingerprint(),
            profile.profile_receipt_fingerprint(),
            layout.epoch().admitted_fingerprint(),
            &self.draws,
        );
        if self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
        {
            return Err(StagingMathDisplayError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMathDisplayError {
    LayoutMismatch,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingMathDisplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LayoutMismatch => formatter.write_str("I9190: math layout mismatch at Display"),
            Self::ReceiptMismatch => formatter.write_str("I9190: math Display receipt mismatch"),
            Self::AllocationFailure => formatter.write_str("L5111: math Display allocation failed"),
        }
    }
}

impl std::error::Error for StagingMathDisplayError {}

pub fn build_staging_math_display(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingMathProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    layout: &StagingMathLayout,
) -> Result<StagingMathDisplay, StagingMathDisplayError> {
    layout
        .verify(package, profile, limits, admitted)
        .map_err(|_| StagingMathDisplayError::LayoutMismatch)?;
    let mut draws = Vec::new();
    draws
        .try_reserve_exact(layout.placements().len())
        .map_err(|_| StagingMathDisplayError::AllocationFailure)?;
    for (placement, node) in layout.placements().iter().zip(package.math_nodes()) {
        let receipt = layout
            .receipt(placement.receipt_key())
            .ok_or(StagingMathDisplayError::ReceiptMismatch)?;
        let mut draw = StagingMathDraw {
            occurrence: placement.occurrence(),
            node_id: placement.node_id(),
            receipt_key: placement.receipt_key(),
            font_face_id: receipt.font_face_id(),
            font_sha256: receipt.font_sha256(),
            page_index: placement.page_index(),
            frame_index: placement.frame_index(),
            paint_ordinal: placement.paint_ordinal(),
            origin_x: placement.origin_x(),
            baseline_y: placement.baseline_y(),
            actual_text: node.domain().speech.clone(),
            paints: receipt.computation().paints().to_vec(),
            vector_fingerprint: receipt.computation().vector_fingerprint(),
            fingerprint: [0; 32],
        };
        draw.fingerprint = sha256(encode_draw(&draw).as_bytes());
        draws.push(draw);
    }
    let admitted_fingerprint = layout.epoch().admitted_fingerprint();
    let canonical_jcs = encode_display(
        layout.fingerprint(),
        profile.profile_receipt_fingerprint(),
        admitted_fingerprint,
        &draws,
    );
    let display = StagingMathDisplay {
        layout_fingerprint: layout.fingerprint(),
        profile_fingerprint: profile.profile_receipt_fingerprint(),
        profile_progress: profile.progress_token(),
        admitted_fingerprint,
        admission_progress: layout.admission_progress().clone(),
        draws,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    display.verify(package, profile, limits, admitted, layout)?;
    Ok(display)
}

fn encode_draw(draw: &StagingMathDraw) -> String {
    let mut output = String::from("{\"actual_text\":");
    push_jcs_string(&mut output, &draw.actual_text);
    output.push_str(",\"baseline_y\":");
    output.push_str(&draw.baseline_y.to_string());
    output.push_str(",\"frame_index\":");
    output.push_str(&draw.frame_index.to_string());
    output.push_str(",\"font_face_id\":");
    output.push_str(&draw.font_face_id.get().to_string());
    output.push_str(",\"font_sha256\":");
    push_hash(&mut output, draw.font_sha256);
    output.push_str(",\"node_id\":");
    output.push_str(&draw.node_id.get().to_string());
    output.push_str(",\"occurrence\":");
    output.push_str(&draw.occurrence.to_string());
    output.push_str(",\"origin_x\":");
    output.push_str(&draw.origin_x.to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&draw.page_index.to_string());
    output.push_str(",\"paint_count\":");
    output.push_str(&draw.paints.len().to_string());
    output.push_str(",\"paint_ordinal\":");
    output.push_str(&draw.paint_ordinal.to_string());
    output.push_str(",\"receipt_key\":");
    push_hash(&mut output, draw.receipt_key.bytes());
    output.push_str(",\"vector_fingerprint\":");
    push_hash(&mut output, draw.vector_fingerprint);
    output.push('}');
    output
}

fn encode_display(
    layout: [u8; 32],
    profile: [u8; 32],
    admitted: [u8; 32],
    draws: &[StagingMathDraw],
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, MATH_DISPLAY_ALGORITHM);
    output.push_str(",\"draws\":[");
    for (index, draw) in draws.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_hash(&mut output, draw.fingerprint);
    }
    output.push_str("],\"layout_fingerprint\":");
    push_hash(&mut output, layout);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(&mut output, profile);
    output.push('}');
    output
}

fn push_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub struct StagingMathDisplayFixture {
    pub layout: typaxis_layout::StagingMathLayoutFixture,
    pub display: StagingMathDisplay,
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_math_display_fixture(
) -> Result<StagingMathDisplayFixture, Box<dyn std::error::Error>> {
    let layout = typaxis_layout::staging_math_layout_fixture()?;
    let display = build_staging_math_display(
        &layout.package,
        &layout.profile,
        &layout.limits,
        &layout.admitted,
        &layout.layout,
    )?;
    Ok(StagingMathDisplayFixture { layout, display })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_display_keeps_vector_and_actual_text_on_one_receipt_key() {
        let fixture = staging_math_display_fixture().unwrap();
        assert_eq!(fixture.display.draws().len(), 2);
        assert_eq!(fixture.display.draws()[0].actual_text(), "x squared");
        assert!(fixture.display.draws()[0]
            .paints()
            .iter()
            .all(|paint| matches!(paint, MathPaint::Glyph(_) | MathPaint::Rule(_))));
    }

    #[test]
    fn math_display_rejects_wrong_alternative_and_vector() {
        let fixture = staging_math_display_fixture().unwrap();
        let mut display = fixture.display;
        display.draws[0].actual_text.push('!');
        assert_eq!(
            display.verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.layout.layout
            ),
            Err(StagingMathDisplayError::ReceiptMismatch)
        );

        let fixture = staging_math_display_fixture().unwrap();
        let mut reordered = fixture.display;
        reordered.draws[1].paints.swap(0, 1);
        assert_eq!(
            reordered.verify_sealed(),
            Err(StagingMathDisplayError::ReceiptMismatch)
        );
    }

    #[test]
    fn math_display_rejects_missing_and_extra_draws() {
        let fixture = staging_math_display_fixture().unwrap();
        let mut missing = fixture.display;
        missing.draws.pop();
        assert_eq!(
            missing.verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.layout.layout
            ),
            Err(StagingMathDisplayError::ReceiptMismatch)
        );

        let fixture = staging_math_display_fixture().unwrap();
        let mut extra = fixture.display;
        extra.draws.push(extra.draws[0].clone());
        assert_eq!(
            extra.verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.layout.layout
            ),
            Err(StagingMathDisplayError::ReceiptMismatch)
        );
    }
}
