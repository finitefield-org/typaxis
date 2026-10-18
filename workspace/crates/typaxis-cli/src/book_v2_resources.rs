//! Private host/source -> resource-set /3 connection. Public CLI dispatch remains
//! closed until the complete successor layout, PDF and publication gates exist.
use typaxis_core::{EffectiveConfig, HostAdmissionContext, M4EffectiveResourceLimits};
use typaxis_diagnostics::ResourceErrorSubject;
use typaxis_machine_profile::book_v2::{
    prepare_book_v2_resource_policy, BookV2ResourcePolicyError,
};
use typaxis_resources::{
    staging_declared_base_catalog_with_budget, AdmittedProductionResourceLedgerV3,
    HostResourceAdmissionSession, ProductionResourceErrorV3, ResourceAdmissionError,
    StagingProductionResourceResolverV3,
};
use typaxis_syntax::book_v2::SourceAdmittedBookV2Body;

#[path = "book_v2_converged_pdf.rs"]
mod converged_pdf;
pub use converged_pdf::*;

#[derive(Debug)]
pub enum BookV2ResourceFailure {
    Policy(BookV2ResourcePolicyError),
    ConfigLimits,
    Resource(ResourceAdmissionError),
    Admission(ProductionResourceErrorV3),
    HostLedger(typaxis_host_admission::HostAdmissionError),
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
    let mut subject = None;
    let outcome = match admit_book_v2_resources_with_admission_budget(
        &body,
        context,
        config,
        limits,
        &mut subject,
        &mut NoBookAdmissionBudget,
    ) {
        Ok(value) => value,
        Err(BookV2ResourceBudgetFailure::Catalog(failure)) => match failure.error {},
        Err(BookV2ResourceBudgetFailure::Read(failure)) => match failure.failure.error {},
        Err(BookV2ResourceBudgetFailure::ReadLedger(failure)) => match failure.error {},
        Err(BookV2ResourceBudgetFailure::Ledger(failure)) => match failure.error {},
        Err(BookV2ResourceBudgetFailure::SvgSource(failure)) => match failure.error {},
        Err(BookV2ResourceBudgetFailure::Font(failure)) => match failure.failure {
            typaxis_font::CffAdmissionBudgetFailureV2::Sfnt(f) => match f.error {},
            typaxis_font::CffAdmissionBudgetFailureV2::Cmap(e) => match e {
                typaxis_font::CffCmapBudgetFailureV2::Base(f) => match f.error {},
                typaxis_font::CffCmapBudgetFailureV2::Variation(f) => match f.error {},
            },
        },
    };
    match outcome {
        Ok(resources) => Ok(PreparedBookV2Resources { body, resources }),
        Err(failure) => Err(BookV2ResourcePreparationError {
            body,
            failure,
            subject,
        }),
    }
}

struct NoBookAdmissionBudget;
impl typaxis_resources::ResourceCatalogBudget for NoBookAdmissionBudget {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn charge(&mut self, _: typaxis_resources::ResourceCatalogRequest) -> Result<(), Self::Error> {
        unreachable!("ordinary catalog preparation")
    }
}
impl typaxis_resources::SafeSvgSourceBudget for NoBookAdmissionBudget {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn work(&mut self, _: typaxis_resources::SafeSvgSourceRequest) -> Result<(), Self::Error> {
        unreachable!("ordinary SVG source")
    }
}
impl typaxis_resources::ProductionResourceLedgerBudgetV3 for NoBookAdmissionBudget {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn reserve(
        &mut self,
        _: typaxis_resources::ProductionResourceLedgerStorageV3,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        unreachable!("ordinary ledger storage")
    }
    fn work(
        &mut self,
        _: typaxis_resources::ProductionResourceLedgerPointV3,
        _: u64,
    ) -> Result<(), Self::Error> {
        unreachable!("ordinary ledger work")
    }
}
impl typaxis_resources::HostReadBudget for NoBookAdmissionBudget {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn charge(&mut self, _: typaxis_resources::HostReadRequest) -> Result<(), Self::Error> {
        unreachable!("ordinary resource preparation never calls host read hooks")
    }
}
impl typaxis_font::CffBaseCmapBudgetV2 for NoBookAdmissionBudget {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn reserve(
        &mut self,
        _: typaxis_font::CffBaseCmapStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        unreachable!("ordinary resource preparation never calls a budget hook")
    }
    fn work(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("ordinary resource preparation never calls a budget hook")
    }
}
impl typaxis_font::CffCmapBudgetV2 for NoBookAdmissionBudget {
    const OBSERVE_VARIATIONS: bool = false;
    fn reserve_variation(
        &mut self,
        _: typaxis_font::CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        unreachable!("ordinary resource preparation never calls variation hooks")
    }
}
impl typaxis_font::CffAdmissionBudgetV2 for NoBookAdmissionBudget {
    const OBSERVE_SFNT: bool = false;
    fn reserve_sfnt(
        &mut self,
        _: typaxis_font::CffSfntStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        unreachable!("ordinary SFNT reserve")
    }
    fn work_sfnt(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("ordinary SFNT work")
    }
}
pub(in crate::book_v2_resources) enum BookV2ResourceBudgetFailure<E> {
    Catalog(typaxis_resources::ResourceCatalogBudgetFailure<E>),
    SvgSource(typaxis_resources::SafeSvgSourceBudgetFailure<E>),
    Ledger(typaxis_resources::ProductionResourceLedgerBudgetFailureV3<E>),
    Read(typaxis_resources::ResourceReadBudgetFailure<E>),
    ReadLedger(typaxis_resources::HostReadBudgetFailure<E>),
    Font(typaxis_resources::ProductionAdmissionBudgetFailureV3<E>),
}
type AdmissionResourceResult<E> = Result<
    Result<AdmittedProductionResourceLedgerV3, BookV2ResourceFailure>,
    BookV2ResourceBudgetFailure<E>,
>;
/// Borrow the source-admitted owner so either native or budget failure can
/// return that same owner without an error-side allocation.
pub(in crate::book_v2_resources) fn admit_book_v2_resources_with_admission_budget<
    B: typaxis_resources::ResourceCatalogBudget<
            Error = <B as typaxis_font::CffBaseCmapBudgetV2>::Error,
        > + typaxis_font::CffAdmissionBudgetV2
        + typaxis_resources::SafeSvgSourceBudget<
            Error = <B as typaxis_font::CffBaseCmapBudgetV2>::Error,
        > + typaxis_resources::HostReadBudget<Error = <B as typaxis_font::CffBaseCmapBudgetV2>::Error>
        + typaxis_resources::ProductionResourceLedgerBudgetV3<
            Error = <B as typaxis_font::CffBaseCmapBudgetV2>::Error,
        >,
>(
    body: &SourceAdmittedBookV2Body,
    context: &HostAdmissionContext,
    config: &EffectiveConfig,
    limits: &M4EffectiveResourceLimits,
    subject: &mut Option<ResourceErrorSubject>,
    budget: &mut B,
) -> AdmissionResourceResult<<B as typaxis_font::CffBaseCmapBudgetV2>::Error> {
    use BookV2ResourceFailure as E;
    macro_rules! native {
        ($value:expr) => {
            match $value {
                Ok(value) => value,
                Err(failure) => return Ok(Err(failure)),
            }
        };
    }
    macro_rules! read_resource {
        ($value:expr) => {
            match $value {
                Ok(value) => value,
                Err(typaxis_resources::ResourceReadError::Native(error)) => {
                    return Ok(Err(E::Admission(error.into())));
                }
                Err(typaxis_resources::ResourceReadError::Budget(failure)) => {
                    return Err(BookV2ResourceBudgetFailure::Read(failure));
                }
            }
        };
    }
    macro_rules! open_resource {
        ($value:expr) => {
            match $value {
                Ok(value) => value,
                Err(typaxis_resources::ResourceReadError::Native(error)) => {
                    return Ok(Err(E::Resource(error)))
                }
                Err(typaxis_resources::ResourceReadError::Budget(failure)) => {
                    return Err(BookV2ResourceBudgetFailure::Read(failure))
                }
            }
        };
    }
    let policy = native!(prepare_book_v2_resource_policy(body, limits).map_err(E::Policy));
    if config.limits() != limits.base()
        || config
            .m4_limits()
            .is_some_and(|configured| configured != limits)
    {
        return Ok(Err(E::ConfigLimits));
    }
    let declared =
        match staging_declared_base_catalog_with_budget(body.styled().body().resources(), budget) {
            Ok(catalog) => catalog,
            Err(typaxis_resources::ResourceCatalogError::Native(error)) => {
                return Ok(Err(E::Resource(error)))
            }
            Err(typaxis_resources::ResourceCatalogError::Budget(failure)) => {
                return Err(BookV2ResourceBudgetFailure::Catalog(failure))
            }
        };
    // Register every resource candidate before opening any resource and keep
    // failures in the same command-wide ledger as PACKAGE and original sources.
    let host = native!(
        HostResourceAdmissionSession::new_with_declared_catalog_and_read_ledger(
            context,
            config,
            &declared,
            body.provenance().read_ledger(),
        )
        .map_err(E::Resource)
    );
    let mut resolver = match StagingProductionResourceResolverV3::new_with_catalog_budget(
        &declared,
        limits,
        policy.fingerprint(),
        host.roots(),
        budget,
    ) {
        Ok(resolver) => resolver,
        Err(typaxis_resources::ResourceCatalogError::Native(error)) => {
            return Ok(Err(E::Admission(error.into())))
        }
        Err(typaxis_resources::ResourceCatalogError::Budget(failure)) => {
            return Err(BookV2ResourceBudgetFailure::Catalog(failure))
        }
    };
    for font in &declared.resource_catalog().font_faces {
        *subject = Some(ResourceErrorSubject::FontFace(font.font_face_id));
        let file = open_resource!(host.open_font_with_budget(font.font_face_id, budget));
        let pending = read_resource!(resolver.read_font_with_budget(file, budget));
        native!(resolver
            .parse_and_bind_font_with_binding_budget(pending, budget)
            .map_err(|failure| match failure.failure {
                typaxis_resources::ProductionFontBindingCauseV3::Admission(admission) =>
                    BookV2ResourceBudgetFailure::Font(
                        typaxis_resources::ProductionAdmissionBudgetFailureV3 {
                            font_face_id: failure.font_face_id,
                            failure: admission
                        }
                    ),
                typaxis_resources::ProductionFontBindingCauseV3::Binding(binding) =>
                    BookV2ResourceBudgetFailure::Catalog(binding),
            })?
            .map_err(E::Admission));
    }
    for image in &declared.resource_catalog().images {
        *subject = Some(ResourceErrorSubject::Image(image.image_id));
        let file = open_resource!(host.open_image_with_budget(image.image_id, budget));
        let pending = read_resource!(resolver.read_image_with_budget(file, budget));
        match resolver.parse_and_bind_image_with_source_budget(pending, budget) {
            Ok(()) => {}
            Err(typaxis_resources::SafeSvgSourceError::Native(error)) => {
                return Ok(Err(E::Admission(error.into())))
            }
            Err(typaxis_resources::SafeSvgSourceError::Budget(failure)) => {
                return Err(BookV2ResourceBudgetFailure::SvgSource(failure))
            }
        }
    }
    *subject = None;
    match resolver.finish_with_budget(budget) {
        Ok(ledger) => {
            // The observed command pays on the same live Source loan after all
            // candidates and actual opens have joined the command-wide ledger.
            // This check is not a publication permit; publishers must recheck.
            if <B as typaxis_resources::HostReadBudget>::OBSERVED && B::OBSERVE_LEDGER {
                let token = native!(body.provenance().read_ledger_token().map_err(E::HostLedger));
                match token.revalidate_with_budget(budget) {
                    Ok(()) => {}
                    Err(typaxis_host_admission::HostReadError::Native(error)) => {
                        return Ok(Err(E::HostLedger(error)));
                    }
                    Err(typaxis_host_admission::HostReadError::Budget(failure)) => {
                        return Err(BookV2ResourceBudgetFailure::ReadLedger(failure));
                    }
                }
            }
            Ok(Ok(ledger))
        }
        Err(typaxis_resources::ProductionResourceLedgerErrorV3::Native(error)) => {
            Ok(Err(E::Admission(error)))
        }
        Err(typaxis_resources::ProductionResourceLedgerErrorV3::Budget(failure)) => {
            Err(BookV2ResourceBudgetFailure::Ledger(failure))
        }
    }
}

#[cfg(test)]
#[path = "book_v2_resources_tests.rs"]
mod tests;
