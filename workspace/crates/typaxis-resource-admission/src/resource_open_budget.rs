//! Authentic registered opens retain logical font/image subjects on refusal.
use super::*;

fn opened<'roots, B: HostReadBudget>(
    roots: HostRootSetToken<'roots>,
    candidate: &RegisteredHostReadCandidate,
    subject: ResourceErrorSubject,
    budget: &mut B,
) -> Result<OpenedContainedFile<'roots>, ResourceReadError<B::Error>> {
    match roots.open_registered_with_budget(candidate, budget) {
        Ok(value) => Ok(value),
        Err(typaxis_host_admission::HostReadError::Native(error)) => {
            Err(map_host_error(error).into())
        }
        Err(typaxis_host_admission::HostReadError::Budget(failure)) => {
            Err(ResourceReadError::Budget(ResourceReadBudgetFailure {
                subject,
                failure,
            }))
        }
    }
}

impl<C: std::borrow::Borrow<ResourceCatalog>> HostResourceAdmissionSession<C> {
    pub fn open_font_with_budget<B: HostReadBudget>(
        &self,
        id: FontFaceId,
        budget: &mut B,
    ) -> Result<VerifiedResourceSource<'_>, ResourceReadError<B::Error>> {
        let declaration = self
            .declarations
            .borrow()
            .font_faces
            .get(id.get() as usize)
            .filter(|v| v.font_face_id == id)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        let candidate = self
            .font_candidates
            .get(id.get() as usize)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        let opened = opened(
            self.roots(),
            candidate,
            ResourceErrorSubject::FontFace(id),
            budget,
        )?;
        Ok(VerifiedResourceSource {
            id: PendingResourceId::Font(id),
            declaration: PendingResourceDeclaration::Font(declaration),
            opened,
        })
    }
    pub fn open_image_with_budget<B: HostReadBudget>(
        &self,
        id: ImageResourceId,
        budget: &mut B,
    ) -> Result<VerifiedResourceSource<'_>, ResourceReadError<B::Error>> {
        let declaration = self
            .declarations
            .borrow()
            .images
            .get(id.get() as usize)
            .filter(|v| v.image_id == id)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        let candidate = self
            .image_candidates
            .get(id.get() as usize)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        let opened = opened(
            self.roots(),
            candidate,
            ResourceErrorSubject::Image(id),
            budget,
        )?;
        Ok(VerifiedResourceSource {
            id: PendingResourceId::Image(id),
            declaration: PendingResourceDeclaration::Image(declaration),
            opened,
        })
    }
}
