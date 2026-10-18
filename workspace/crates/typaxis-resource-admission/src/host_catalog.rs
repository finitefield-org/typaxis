//! Share an immutable caller-owned catalog while retaining the same host authority.
use super::*;

pub(super) struct Registered {
    pub host: HostAdmissionSession,
    pub fonts: Vec<RegisteredHostReadCandidate>,
    pub images: Vec<RegisteredHostReadCandidate>,
}

pub(super) fn register(
    context: &HostAdmissionContext,
    config: &EffectiveConfig,
    declarations: &ResourceCatalog,
    ledger: Option<&HostReadIdentityLedger>,
) -> Result<Registered, ResourceAdmissionError> {
    if !ResourceAdmissionCapabilityToken::compiled().contained_resource_open() {
        return Err(ResourceAdmissionError::UnsupportedContainedOpen);
    }
    validate_declaration_order(declarations)?;
    register_validated(context, config, declarations, ledger)
}

fn register_validated(
    context: &HostAdmissionContext,
    config: &EffectiveConfig,
    declarations: &ResourceCatalog,
    ledger: Option<&HostReadIdentityLedger>,
) -> Result<Registered, ResourceAdmissionError> {
    let host = match ledger {
        Some(ledger) => HostAdmissionSession::new_with_read_ledger(context, config, ledger),
        None => HostAdmissionSession::new(context, config),
    }
    .map_err(map_host_error)?;
    let font_count = declarations.font_faces.len();
    let declaration_count = font_count
        .checked_add(declarations.images.len())
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    let paths = (0..declaration_count).map(|index| {
        if index < font_count {
            &declarations.font_faces[index].uri
        } else {
            &declarations.images[index - font_count].uri
        }
    });
    let mut fonts = host
        .roots()
        .register_candidates(paths)
        .map_err(map_host_error)?;
    let images = fonts.split_off(font_count);
    Ok(Registered {
        host,
        fonts,
        images,
    })
}

impl HostResourceAdmissionSession {
    /// Borrow a sealed catalog whose dense IDs, unique valid families and
    /// explicit media were validated when that catalog was issued.
    /// Root admission and every candidate registration remain mandatory.
    ///
    /// ```compile_fail
    /// use typaxis_core::{EffectiveConfig, HostAdmissionContext};
    /// use typaxis_document::ResourceCatalog;
    /// use typaxis_host_admission::HostReadIdentityLedger;
    /// use typaxis_resource_admission::HostResourceAdmissionSession;
    /// fn raw(c: &HostAdmissionContext, e: &EffectiveConfig, catalog: &ResourceCatalog,
    ///        ledger: &HostReadIdentityLedger) {
    ///     HostResourceAdmissionSession::new_with_declared_catalog_and_read_ledger(c,e,catalog,ledger).unwrap();
    /// }
    /// ```
    pub fn new_with_declared_catalog_and_read_ledger<'catalog>(
        context: &HostAdmissionContext,
        config: &EffectiveConfig,
        declared: &'catalog StagingDeclaredBaseCatalog,
        ledger: &HostReadIdentityLedger,
    ) -> Result<HostResourceAdmissionSession<&'catalog ResourceCatalog>, ResourceAdmissionError>
    {
        if !ResourceAdmissionCapabilityToken::compiled().contained_resource_open() {
            return Err(ResourceAdmissionError::UnsupportedContainedOpen);
        }
        let declarations = declared.resource_catalog();
        let registered = register_validated(context, config, declarations, Some(ledger))?;
        Ok(HostResourceAdmissionSession {
            host: registered.host,
            declarations,
            font_candidates: registered.fonts,
            image_candidates: registered.images,
        })
    }

    /// Admit roots and register all candidates while borrowing the immutable
    /// declarations. The session and its verified sources cannot outlive that
    /// catalog. Registration, containment and read identity checks are shared
    /// with the owning constructors.
    ///
    /// ```compile_fail
    /// use typaxis_core::{EffectiveConfig, HostAdmissionContext};
    /// use typaxis_document::ResourceCatalog;
    /// use typaxis_resource_admission::HostResourceAdmissionSession;
    /// fn escape(c: &HostAdmissionContext, e: &EffectiveConfig)
    ///     -> HostResourceAdmissionSession<&'static ResourceCatalog> {
    ///     let catalog = ResourceCatalog { font_faces: vec![], images: vec![] };
    ///     HostResourceAdmissionSession::new_borrowed(c, e, &catalog).unwrap()
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// use typaxis_core::{EffectiveConfig, HostAdmissionContext};
    /// use typaxis_document::ResourceCatalog;
    /// use typaxis_resource_admission::HostResourceAdmissionSession;
    /// fn change(c: &HostAdmissionContext, e: &EffectiveConfig, catalog: &mut ResourceCatalog) {
    ///     let session = HostResourceAdmissionSession::new_borrowed(c, e, catalog).unwrap();
    ///     catalog.font_faces.clear();
    ///     let _ = session.roots();
    /// }
    /// ```
    pub fn new_borrowed<'catalog>(
        context: &HostAdmissionContext,
        config: &EffectiveConfig,
        declarations: &'catalog ResourceCatalog,
    ) -> Result<HostResourceAdmissionSession<&'catalog ResourceCatalog>, ResourceAdmissionError>
    {
        Self::new_borrowed_inner(context, config, declarations, None)
    }

    /// Borrow the declarations and register candidates into the existing
    /// command-wide PACKAGE/config/source/resource read ledger.
    pub fn new_borrowed_with_read_ledger<'catalog>(
        context: &HostAdmissionContext,
        config: &EffectiveConfig,
        declarations: &'catalog ResourceCatalog,
        ledger: &HostReadIdentityLedger,
    ) -> Result<HostResourceAdmissionSession<&'catalog ResourceCatalog>, ResourceAdmissionError>
    {
        Self::new_borrowed_inner(context, config, declarations, Some(ledger))
    }

    fn new_borrowed_inner<'catalog>(
        context: &HostAdmissionContext,
        config: &EffectiveConfig,
        declarations: &'catalog ResourceCatalog,
        ledger: Option<&HostReadIdentityLedger>,
    ) -> Result<HostResourceAdmissionSession<&'catalog ResourceCatalog>, ResourceAdmissionError>
    {
        let registered = register(context, config, declarations, ledger)?;
        Ok(HostResourceAdmissionSession {
            host: registered.host,
            declarations,
            font_candidates: registered.fonts,
            image_candidates: registered.images,
        })
    }
}
