use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use typaxis_core::{push_jcs_string, sha256, ImageResourceId, M4EffectiveResourceLimits};
use typaxis_display_list::{StagingDrawVector, StagingSafeVectorDisplay};
use typaxis_resource_admission::{AdmittedImageMediaKind, AdmittedResourceLedger, SafeVectorIr};

pub const STAGING_SAFE_VECTOR_FORM_PLAN_ALGORITHM: &str = "typaxis.safe-vector-form-plan/1";
pub const STAGING_SAFE_VECTOR_FORM_PLANS_ALGORITHM: &str = "typaxis.safe-vector-form-plans/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorUsage {
    occurrence: u32,
    page_index: u32,
    display_command_fingerprint: [u8; 32],
}

impl StagingSafeVectorUsage {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn display_command_fingerprint(&self) -> [u8; 32] {
        self.display_command_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenSafeVectorFormPlan {
    image_id: ImageResourceId,
    admitted_sha256: [u8; 32],
    ir_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    ir: Arc<SafeVectorIr>,
    usages: Vec<StagingSafeVectorUsage>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl FrozenSafeVectorFormPlan {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub const fn ir_fingerprint(&self) -> [u8; 32] {
        self.ir_fingerprint
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub fn ir(&self) -> &SafeVectorIr {
        &self.ir
    }
    pub fn usages(&self) -> &[StagingSafeVectorUsage] {
        &self.usages
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorFormPlans {
    display_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    plans: Vec<FrozenSafeVectorFormPlan>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorFormPlans {
    pub fn plans(&self) -> &[FrozenSafeVectorFormPlan] {
        &self.plans
    }
    pub fn plan(&self, id: ImageResourceId) -> Option<&FrozenSafeVectorFormPlan> {
        self.plans.iter().find(|plan| plan.image_id == id)
    }
    pub const fn display_fingerprint(&self) -> [u8; 32] {
        self.display_fingerprint
    }
    pub const fn admitted_fingerprint(&self) -> [u8; 32] {
        self.admitted_fingerprint
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify_pdf_closure(
        &self,
        display: &StagingSafeVectorDisplay,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSafeVectorResourceError> {
        display
            .verify_resource_closure()
            .map_err(|_| StagingSafeVectorResourceError::DisplayMismatch)?;
        if self.display_fingerprint != display.receipt().fingerprint()
            || self.limits_fingerprint != limits.fingerprint()
            || self
                .plans
                .windows(2)
                .any(|pair| pair[0].image_id >= pair[1].image_id)
        {
            return Err(StagingSafeVectorResourceError::ReceiptMismatch);
        }
        let mut observed = BTreeSet::new();
        for plan in &self.plans {
            if plan.usages.is_empty()
                || plan.ir.fingerprint() != plan.ir_fingerprint
                || plan.limits_fingerprint != limits.fingerprint()
                || plan.canonical_jcs
                    != encode_plan(
                        plan.image_id,
                        plan.admitted_sha256,
                        plan.ir_fingerprint,
                        plan.limits_fingerprint,
                        &plan.usages,
                    )
                || plan.fingerprint != sha256(plan.canonical_jcs.as_bytes())
            {
                return Err(StagingSafeVectorResourceError::ReceiptMismatch);
            }
            for usage in &plan.usages {
                if !observed.insert(usage.occurrence) {
                    return Err(StagingSafeVectorResourceError::DuplicateOccurrence);
                }
                let command = display
                    .commands()
                    .find(|command| command.occurrence() == usage.occurrence)
                    .ok_or(StagingSafeVectorResourceError::ReceiptMismatch)?;
                if command.image_id() != plan.image_id
                    || command.admitted_sha256() != plan.admitted_sha256
                    || command.ir_fingerprint() != plan.ir_fingerprint
                    || command.page_index() != usage.page_index
                    || command.fingerprint() != usage.display_command_fingerprint
                {
                    return Err(StagingSafeVectorResourceError::ReceiptMismatch);
                }
            }
        }
        if observed.len() != display.receipt().command_count() as usize
            || self.canonical_jcs
                != encode_plans(
                    self.display_fingerprint,
                    self.admitted_fingerprint,
                    self.limits_fingerprint,
                    &self.plans,
                )
            || self.fingerprint != sha256(self.canonical_jcs.as_bytes())
        {
            return Err(StagingSafeVectorResourceError::ReceiptMismatch);
        }
        Ok(())
    }

    pub fn verify(
        &self,
        display: &StagingSafeVectorDisplay,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSafeVectorResourceError> {
        display
            .verify_resource_closure()
            .map_err(|_| StagingSafeVectorResourceError::DisplayMismatch)?;
        let expected = assemble_plans(display, admitted, limits)?;
        if self != &expected {
            return Err(StagingSafeVectorResourceError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorResourceError {
    DisplayMismatch,
    MissingAdmittedVector(ImageResourceId),
    WrongMedia(ImageResourceId),
    HashMismatch(ImageResourceId),
    IrMismatch(ImageResourceId),
    LimitsMismatch(ImageResourceId),
    ProfileMismatch(ImageResourceId),
    DuplicateOccurrence,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingSafeVectorResourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DisplayMismatch => formatter.write_str("I9190: DrawVector Display mismatch"),
            Self::MissingAdmittedVector(id) => {
                write!(formatter, "I9190: vector {} is not admitted", id.get())
            }
            Self::WrongMedia(id) => {
                write!(formatter, "I9190: image {} is not SafeVector", id.get())
            }
            Self::HashMismatch(id) => {
                write!(formatter, "I9190: vector {} byte hash mismatch", id.get())
            }
            Self::IrMismatch(id) => write!(formatter, "I9190: vector {} IR mismatch", id.get()),
            Self::LimitsMismatch(id) => {
                write!(formatter, "I9190: vector {} limits mismatch", id.get())
            }
            Self::ProfileMismatch(id) => {
                write!(formatter, "I9190: vector {} profile mismatch", id.get())
            }
            Self::DuplicateOccurrence => {
                formatter.write_str("I9190: duplicate DrawVector occurrence")
            }
            Self::ReceiptMismatch => formatter.write_str("I9190: frozen vector Form plan mismatch"),
            Self::AllocationFailure => {
                formatter.write_str("D8101: vector Form plan allocation failed")
            }
        }
    }
}

impl std::error::Error for StagingSafeVectorResourceError {}

pub fn finalize_staging_safe_vector_forms(
    display: &StagingSafeVectorDisplay,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingSafeVectorFormPlans, StagingSafeVectorResourceError> {
    display
        .verify_resource_closure()
        .map_err(|_| StagingSafeVectorResourceError::DisplayMismatch)?;
    assemble_plans(display, admitted, limits)
}

fn assemble_plans(
    display: &StagingSafeVectorDisplay,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingSafeVectorFormPlans, StagingSafeVectorResourceError> {
    let mut by_image: BTreeMap<ImageResourceId, Vec<&StagingDrawVector>> = BTreeMap::new();
    let mut occurrences = BTreeSet::new();
    for command in display.commands() {
        if !occurrences.insert(command.occurrence()) {
            return Err(StagingSafeVectorResourceError::DuplicateOccurrence);
        }
        let image = admitted.image(command.image_id()).ok_or(
            StagingSafeVectorResourceError::MissingAdmittedVector(command.image_id()),
        )?;
        if image.media_kind() != AdmittedImageMediaKind::SafeVector {
            return Err(StagingSafeVectorResourceError::WrongMedia(
                command.image_id(),
            ));
        }
        let ir = image
            .safe_vector()
            .ok_or(StagingSafeVectorResourceError::WrongMedia(
                command.image_id(),
            ))?;
        if image.content_hash() != command.admitted_sha256() {
            return Err(StagingSafeVectorResourceError::HashMismatch(
                command.image_id(),
            ));
        }
        if ir.fingerprint() != command.ir_fingerprint() {
            return Err(StagingSafeVectorResourceError::IrMismatch(
                command.image_id(),
            ));
        }
        if image.m4_limits_fingerprint() != Some(limits.fingerprint()) {
            return Err(StagingSafeVectorResourceError::LimitsMismatch(
                command.image_id(),
            ));
        }
        if image.m4_profile_fingerprint() != Some(display.receipt().profile_fingerprint()) {
            return Err(StagingSafeVectorResourceError::ProfileMismatch(
                command.image_id(),
            ));
        }
        by_image
            .entry(command.image_id())
            .or_default()
            .push(command);
    }
    let mut plans = Vec::new();
    plans
        .try_reserve_exact(by_image.len())
        .map_err(|_| StagingSafeVectorResourceError::AllocationFailure)?;
    for image in admitted.images() {
        if image.media_kind() != AdmittedImageMediaKind::SafeVector {
            continue;
        }
        let Some(commands) = by_image.remove(&image.image_id()) else {
            // An admitted but unused vector deliberately has no Form plan.
            continue;
        };
        let ir = image
            .safe_vector_arc()
            .ok_or(StagingSafeVectorResourceError::WrongMedia(image.image_id()))?;
        let usages: Vec<_> = commands
            .into_iter()
            .map(|command| StagingSafeVectorUsage {
                occurrence: command.occurrence(),
                page_index: command.page_index(),
                display_command_fingerprint: command.fingerprint(),
            })
            .collect();
        let canonical_jcs = encode_plan(
            image.image_id(),
            image.content_hash(),
            ir.fingerprint(),
            limits.fingerprint(),
            &usages,
        );
        plans.push(FrozenSafeVectorFormPlan {
            image_id: image.image_id(),
            admitted_sha256: image.content_hash(),
            ir_fingerprint: ir.fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            ir,
            usages,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        });
    }
    if !by_image.is_empty() {
        return Err(StagingSafeVectorResourceError::MissingAdmittedVector(
            *by_image.keys().next().expect("nonempty map has a key"),
        ));
    }
    let canonical_jcs = encode_plans(
        display.receipt().fingerprint(),
        admitted.fingerprint().bytes(),
        limits.fingerprint(),
        &plans,
    );
    Ok(StagingSafeVectorFormPlans {
        display_fingerprint: display.receipt().fingerprint(),
        admitted_fingerprint: admitted.fingerprint().bytes(),
        limits_fingerprint: limits.fingerprint(),
        plans,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn encode_plan(
    image_id: ImageResourceId,
    admitted_sha256: [u8; 32],
    ir_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    usages: &[StagingSafeVectorUsage],
) -> String {
    let mut output = String::from("{\"admitted_sha256\":");
    push_hash(&mut output, admitted_sha256);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_SAFE_VECTOR_FORM_PLAN_ALGORITHM);
    output.push_str(",\"image_id\":");
    output.push_str(&image_id.get().to_string());
    output.push_str(",\"ir_fingerprint\":");
    push_hash(&mut output, ir_fingerprint);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits_fingerprint);
    output.push_str(",\"usages\":[");
    for (index, usage) in usages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"display_command_fingerprint\":");
        push_hash(&mut output, usage.display_command_fingerprint);
        output.push_str(",\"occurrence\":");
        output.push_str(&usage.occurrence.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&usage.page_index.to_string());
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn encode_plans(
    display: [u8; 32],
    admitted: [u8; 32],
    limits: [u8; 32],
    plans: &[FrozenSafeVectorFormPlan],
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_SAFE_VECTOR_FORM_PLANS_ALGORITHM);
    output.push_str(",\"display_fingerprint\":");
    push_hash(&mut output, display);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits);
    output.push_str(",\"plans\":[");
    for (index, plan) in plans.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(plan.canonical_jcs());
    }
    output.push_str("]}");
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
pub struct StagingSafeVectorResourceFixture {
    pub display: typaxis_display_list::StagingSafeVectorDisplayFixture,
    pub plans: StagingSafeVectorFormPlans,
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_safe_vector_resource_fixture(
) -> Result<StagingSafeVectorResourceFixture, Box<dyn std::error::Error>> {
    let display = typaxis_display_list::staging_safe_vector_display_fixture()?;
    let plans = finalize_staging_safe_vector_forms(
        &display.display,
        &display.layout.admitted,
        &display.layout.limits,
    )?;
    Ok(StagingSafeVectorResourceFixture { display, plans })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_form_plan_is_unique_per_used_image_and_excludes_alternative_text() {
        let fixture = staging_safe_vector_resource_fixture().unwrap();
        assert_eq!(fixture.display.layout.admitted.images().len(), 2);
        assert_eq!(fixture.plans.plans().len(), 1);
        assert!(fixture.plans.plan(ImageResourceId::new(1)).is_none());
        let plan = &fixture.plans.plans()[0];
        assert_eq!(plan.image_id(), ImageResourceId::new(0));
        assert_eq!(plan.usages().len(), 1);
        assert!(!plan.canonical_jcs().contains("Blue vector geometry"));
        fixture
            .plans
            .verify(
                &fixture.display.display,
                &fixture.display.layout.admitted,
                &fixture.display.layout.limits,
            )
            .unwrap();
    }
}
