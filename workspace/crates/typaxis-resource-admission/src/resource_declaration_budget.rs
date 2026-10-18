//! Borrow sealed declarations; fund the single owned URI needed after a read.
use super::{
    HostReadBudget, HostReadBudgetFailure, HostReadRequest, HostReadStage,
    PendingResourceDeclaration, PortablePath, ResourceAdmissionError, ResourceErrorSubject,
    ResourceReadBudgetFailure, ResourceReadError,
};

fn charge<B: HostReadBudget>(
    subject: ResourceErrorSubject,
    stage: HostReadStage,
    bytes: u64,
    work: u64,
    budget: &mut B,
) -> Result<(), ResourceReadError<B::Error>> {
    let request = HostReadRequest {
        stage,
        file_offset: 0,
        buffer_bytes: bytes,
        work,
    };
    budget.charge(request).map_err(|error| {
        ResourceReadError::Budget(ResourceReadBudgetFailure {
            subject,
            failure: HostReadBudgetFailure { error, request },
        })
    })
}

fn string_bytes(value: PendingResourceDeclaration<'_>) -> Result<u64, ResourceAdmissionError> {
    let bytes = match value {
        PendingResourceDeclaration::Font(font) => font
            .family
            .len()
            .checked_add(font.uri.as_str().len())
            .ok_or(ResourceAdmissionError::ResourceLimit)?,
        PendingResourceDeclaration::Image(image) => image.uri.as_str().len(),
    };
    u64::try_from(bytes).map_err(|_| ResourceAdmissionError::ResourceLimit)
}

pub(super) fn matches<B: HostReadBudget>(
    source: PendingResourceDeclaration<'_>,
    expected: PendingResourceDeclaration<'_>,
    subject: ResourceErrorSubject,
    budget: &mut B,
) -> Result<bool, ResourceReadError<B::Error>> {
    if B::OBSERVED && B::OBSERVE_DECLARATIONS {
        let work = string_bytes(source)?
            .checked_add(string_bytes(expected)?)
            .and_then(|n| n.checked_mul(2))
            .and_then(|n| n.checked_add(64))
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        charge(subject, HostReadStage::DeclarationCheck, 0, work, budget)?;
    }
    Ok(source == expected)
}

pub(super) fn pending_uri<B: HostReadBudget>(
    uri: &PortablePath,
    subject: ResourceErrorSubject,
    budget: &mut B,
) -> Result<PortablePath, ResourceReadError<B::Error>> {
    if !B::OBSERVED || !B::OBSERVE_DECLARATIONS {
        return Ok(uri.clone());
    }
    let bytes =
        u64::try_from(uri.as_str().len()).map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    let work = bytes
        .checked_mul(16)
        .and_then(|n| n.checked_add(64))
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    charge(subject, HostReadStage::DeclarationUri, bytes, work, budget)?;
    let mut owned = String::new();
    owned
        .try_reserve_exact(uri.as_str().len())
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    owned.push_str(uri.as_str());
    // The input is already a validated portable path. Reusing its public
    // constructor consumes the reserved String and does not copy its buffer.
    PortablePath::new(owned).map_err(|_| ResourceAdmissionError::ReceiptIdentityMismatch.into())
}

#[cfg(test)]
#[path = "resource_declaration_budget_tests.rs"]
mod tests;
