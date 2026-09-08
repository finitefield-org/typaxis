//! Private host/source -> resource-set /3 connection. Public CLI dispatch remains
//! closed until the complete successor layout, PDF and publication gates exist.
use typaxis_core::{EffectiveConfig, HostAdmissionContext, M4EffectiveResourceLimits};
use typaxis_diagnostics::ResourceErrorSubject;
use typaxis_machine_profile::book_v2::{
    prepare_book_v2_resource_policy, BookV2ResourcePolicyError,
};
use typaxis_resources::{
    staging_declared_base_catalog, AdmittedProductionResourceLedgerV3,
    HostResourceAdmissionSession, ProductionResourceErrorV3, ResourceAdmissionError,
    StagingProductionResourceResolverV3,
};
use typaxis_syntax::book_v2::SourceAdmittedBookV2Body;

#[derive(Debug)]
pub enum BookV2ResourceFailure {
    Policy(BookV2ResourcePolicyError),
    ConfigLimits,
    Resource(ResourceAdmissionError),
    Admission(ProductionResourceErrorV3),
}
impl std::fmt::Display for BookV2ResourceFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 resource preparation: {self:?}")
    }
}
impl std::error::Error for BookV2ResourceFailure {}

#[derive(Debug)]
pub struct BookV2ResourcePreparationError {
    body: SourceAdmittedBookV2Body,
    failure: BookV2ResourceFailure,
    subject: Option<ResourceErrorSubject>,
}
impl BookV2ResourcePreparationError {
    pub fn body(&self) -> &SourceAdmittedBookV2Body {
        &self.body
    }
    pub fn subject(&self) -> Option<&ResourceErrorSubject> {
        self.subject.as_ref()
    }
    pub fn failure(&self) -> &BookV2ResourceFailure {
        &self.failure
    }
}
impl std::fmt::Display for BookV2ResourcePreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.failure.fmt(f)
    }
}
impl std::error::Error for BookV2ResourcePreparationError {}

/// Owns the exact source-admitted body and resources read under its policy.
/// It cannot become a legacy syntax package or a resource-set /2 ledger.
#[derive(Debug)]
pub struct PreparedBookV2Resources {
    body: SourceAdmittedBookV2Body,
    resources: AdmittedProductionResourceLedgerV3,
}
impl PreparedBookV2Resources {
    pub fn body(&self) -> &SourceAdmittedBookV2Body {
        &self.body
    }
    pub fn resources(&self) -> &AdmittedProductionResourceLedgerV3 {
        &self.resources
    }
}

pub fn prepare_book_v2_resources(
    body: SourceAdmittedBookV2Body,
    context: &HostAdmissionContext,
    config: &EffectiveConfig,
    limits: &M4EffectiveResourceLimits,
) -> Result<PreparedBookV2Resources, BookV2ResourcePreparationError> {
    use BookV2ResourceFailure as E;
    let mut subject = None;
    let mut admit = || -> Result<AdmittedProductionResourceLedgerV3, BookV2ResourceFailure> {
        let policy = prepare_book_v2_resource_policy(&body, limits).map_err(E::Policy)?;
        if config.limits() != limits.base()
            || config
                .m4_limits()
                .is_some_and(|configured| configured != limits)
        {
            return Err(E::ConfigLimits);
        }
        let declared =
            staging_declared_base_catalog(body.styled().body().resources()).map_err(E::Resource)?;
        // Register every resource candidate before opening any resource and keep
        // failures in the same command-wide ledger as PACKAGE and original sources.
        let host = HostResourceAdmissionSession::new_with_read_ledger(
            context,
            config,
            declared.resource_catalog(),
            body.provenance().read_ledger(),
        )
        .map_err(E::Resource)?;
        let mut resolver = StagingProductionResourceResolverV3::new(
            &declared,
            limits,
            policy.fingerprint(),
            host.roots(),
        )
        .map_err(E::Admission)?;
        for font in &declared.resource_catalog().font_faces {
            subject = Some(ResourceErrorSubject::FontFace(font.font_face_id));
            let file = host.open_font(font.font_face_id).map_err(E::Resource)?;
            let pending = resolver.read_font(file).map_err(E::Admission)?;
            resolver
                .parse_and_bind_font(pending)
                .map_err(E::Admission)?;
        }
        for image in &declared.resource_catalog().images {
            subject = Some(ResourceErrorSubject::Image(image.image_id));
            let file = host.open_image(image.image_id).map_err(E::Resource)?;
            let pending = resolver.read_image(file).map_err(E::Admission)?;
            resolver
                .parse_and_bind_image(pending)
                .map_err(E::Admission)?;
        }
        subject = None;
        resolver.finish().map_err(E::Admission)
    };
    match admit() {
        Ok(resources) => Ok(PreparedBookV2Resources { body, resources }),
        Err(failure) => Err(BookV2ResourcePreparationError {
            body,
            failure,
            subject,
        }),
    }
}

#[cfg(test)]
#[path = "book_v2_resources_tests.rs"]
mod tests;
