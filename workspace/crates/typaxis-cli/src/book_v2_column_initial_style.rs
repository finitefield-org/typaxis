//! Initial style preparation joined to the same real Column command history.
//! Stable resource reads and observed CFF admission share the source cells.
//! Unobserved parser internals and host-resolution allocations remain separate.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ColumnInitialStyleLimit {
    Records,
    Spool,
    Work,
}

#[derive(Debug)]
pub enum BookV2ColumnInitialStylePreparationError {
    Catalog {
        body: typaxis_syntax::book_v2::SourceAdmittedBookV2Body,
        failure: typaxis_resources::ResourceCatalogBudgetFailure<
            typaxis_shaping::book_v2::BookV2ObservedAccountingError,
        >,
    },
    SvgSource {
        body: typaxis_syntax::book_v2::SourceAdmittedBookV2Body,
        failure: typaxis_resources::SafeSvgSourceBudgetFailure<
            typaxis_shaping::book_v2::BookV2ObservedAccountingError,
        >,
    },
    Ledger {
        body: typaxis_syntax::book_v2::SourceAdmittedBookV2Body,
        failure: typaxis_resources::ProductionResourceLedgerBudgetFailureV3<
            typaxis_shaping::book_v2::BookV2ObservedAccountingError,
        >,
    },
    Identity,
    Source(typaxis_syntax::book_v2::BookV2CountedSourceStyleError<BookV2ColumnInitialStyleLimit>),
    Resources(crate::book_v2_resources::BookV2ResourcePreparationError),
    Admission {
        body: typaxis_syntax::book_v2::SourceAdmittedBookV2Body,
        failure: typaxis_resources::ProductionAdmissionBudgetFailureV3<
            typaxis_shaping::book_v2::BookV2ObservedAccountingError,
        >,
    },
    Read {
        body: typaxis_syntax::book_v2::SourceAdmittedBookV2Body,
        failure: typaxis_resources::ResourceReadBudgetFailure<
            typaxis_shaping::book_v2::BookV2ObservedAccountingError,
        >,
    },
    ReadLedger {
        body: typaxis_syntax::book_v2::SourceAdmittedBookV2Body,
        failure: typaxis_resources::HostReadBudgetFailure<
            typaxis_shaping::book_v2::BookV2ObservedAccountingError,
        >,
    },
}
impl std::fmt::Display for BookV2ColumnInitialStylePreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Catalog { failure, .. } => write!(f, "initial Column catalog: {failure}"),
            Self::Admission { failure, .. } => {
                write!(f, "initial Column font admission: {failure}")
            }
            Self::Read { failure, .. } => write!(f, "initial Column resource read: {failure}"),
            Self::ReadLedger { failure, .. } => {
                write!(f, "initial Column read ledger: {failure:?}")
            }
            Self::Ledger { failure, .. } => write!(f, "initial Column resource ledger: {failure}"),
            Self::SvgSource { failure, .. } => write!(f, "initial Column SVG source: {failure}"),
            _ => write!(f, "initial Column style preparation: {self:?}"),
        }
    }
}
impl std::error::Error for BookV2ColumnInitialStylePreparationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Catalog { failure, .. } => Some(failure),
            Self::Admission { failure, .. } => Some(failure),
            Self::Read { failure, .. } => Some(failure),
            Self::ReadLedger { failure, .. } => Some(&failure.error),
            Self::Ledger { failure, .. } => Some(failure),
            Self::SvgSource { failure, .. } => Some(failure),
            _ => None,
        }
    }
}

/// Prepare a genuine initial styled/source-admitted input under the command's
/// actual accepted style R/S/W history and cmap admission requests.
/// Later convergence takes this returned input and the same `budget`.
///
/// Each style request is atomic; accepted requests remain on source/resource
/// failure. Output, CFF history and layout passes are not changed here.
pub fn prepare_counted_book_v2_column_resources(
    input: typaxis_machine_input::book_v2::AdmittedBookV2Input,
    context: &typaxis_core::HostAdmissionContext,
    config: &typaxis_core::EffectiveConfig,
    limits: &M4EffectiveResourceLimits,
    budget: &mut BookV2ColumnPdfConvergenceBudget,
) -> Result<PreparedBookV2Resources, BookV2ColumnInitialStylePreparationError> {
    use BookV2ColumnInitialStylePreparationError as Error;
    if budget.pages.inner.limits_fingerprint != limits.fingerprint() {
        return Err(Error::Identity);
    }
    let maximum = budget.pages.inner.maximum_work;
    let observed = &mut budget.pages.inner.observation;
    let styled = typaxis_syntax::book_v2::prepare_admitted_book_v2_body_with_counted_style(
        input,
        limits.base(),
        &mut |r, s, w| {
            let records = observed
                .records
                .checked_add(r)
                .filter(|v| *v <= limits.base().get().max_fragments)
                .ok_or(BookV2ColumnInitialStyleLimit::Records)?;
            let spool = observed
                .spool
                .checked_add(s)
                .filter(|v| *v <= limits.base().get().max_spool_bytes)
                .ok_or(BookV2ColumnInitialStyleLimit::Spool)?;
            let work = observed
                .work
                .checked_add(w)
                .filter(|v| *v <= maximum)
                .ok_or(BookV2ColumnInitialStyleLimit::Work)?;
            observed.records = records;
            observed.spool = spool;
            observed.work = work;
            Ok(())
        },
    )
    .map_err(Error::Source)?;
    let mut subject = None;
    let result = typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(
        &mut observed.records,
        &mut observed.spool,
        &mut observed.work,
        limits.base(),
        limits.base().get().max_fragments,
        maximum,
        |accounting| {
            // Admission belongs to the command/root; the outer typed failure
            // retains the resolver's authentic FontFaceId as its resource subject.
            let mut admission = accounting.resource_admission_budget(typaxis_core::NodeId::new(0));
            crate::book_v2_resources::admit_book_v2_resources_with_admission_budget(
                &styled,
                context,
                config,
                limits,
                &mut subject,
                &mut admission,
            )
        },
    );
    match result {
        Err(crate::book_v2_resources::BookV2ResourceBudgetFailure::Catalog(failure)) => {
            Err(Error::Catalog {
                body: styled,
                failure,
            })
        }
        Err(crate::book_v2_resources::BookV2ResourceBudgetFailure::SvgSource(failure)) => {
            Err(Error::SvgSource {
                body: styled,
                failure,
            })
        }
        Err(crate::book_v2_resources::BookV2ResourceBudgetFailure::Ledger(failure)) => {
            Err(Error::Ledger {
                body: styled,
                failure,
            })
        }
        Ok(Ok(resources)) => Ok(PreparedBookV2Resources {
            body: styled,
            resources,
        }),
        Ok(Err(failure)) => Err(Error::Resources(
            crate::book_v2_resources::BookV2ResourcePreparationError {
                body: styled,
                failure,
                subject,
            },
        )),
        Err(crate::book_v2_resources::BookV2ResourceBudgetFailure::Font(failure)) => {
            Err(Error::Admission {
                body: styled,
                failure,
            })
        }
        Err(crate::book_v2_resources::BookV2ResourceBudgetFailure::Read(failure)) => {
            Err(Error::Read {
                body: styled,
                failure,
            })
        }
        Err(crate::book_v2_resources::BookV2ResourceBudgetFailure::ReadLedger(failure)) => {
            Err(Error::ReadLedger {
                body: styled,
                failure,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    use typaxis_core::{
        ConfigResourceRoot, DocumentPackageContractId, EffectiveConfig, EffectiveDataVersions,
        HostAdmissionContext, HostPath, PdfStreamCompression, DEFAULT_ALLOWED_URI_SCHEMES,
        REGISTERED_JAPANESE_LINE_BREAK_VERSION, REGISTERED_UNICODE_VERSION,
    };
    use typaxis_document_package::DocumentPackageDecodePolicy;
    use typaxis_machine_input::{
        book_v2::{AdmittedBookV2Input, HostBookV2InputSession},
        MachineInputHostOptions,
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn hosted<R>(
        original: &PreparedBookV2Resources,
        caps: &M4EffectiveResourceLimits,
        run: impl FnOnce(AdmittedBookV2Input, &HostAdmissionContext, &EffectiveConfig, &Temp) -> R,
    ) -> R {
        hosted_with_cff(original, caps, None, run)
    }
    // Independent contract reference for a regular file whose read chunks
    // fill each requested extent. Native short-read behavior is tested in Host.
    fn resource_resolver_reference_requests(
        ledger: &typaxis_resources::AdmittedProductionResourceLedgerV3,
        image_uris: Option<&[String]>,
    ) -> Vec<typaxis_resources::ResourceCatalogRequest> {
        use typaxis_resources::{ResourceCatalogRequest as R, ResourceCatalogStage as S};
        let mut out = Vec::new();
        let point = |stage, font_face_id, image_id, records, storage_bytes, work| R {
            stage,
            font_face_id,
            image_id,
            records,
            storage_bytes,
            work,
        };
        let images = match image_uris {
            Some(v) => v.iter().map(String::as_str).collect::<Vec<_>>(),
            None => ledger.images().iter().map(|v| v.uri().as_str()).collect(),
        };
        out.push(point(
            S::ResolverStart,
            None,
            None,
            0,
            0,
            std::mem::size_of::<typaxis_core::ValidatedResourceLimits>() as u64 + 64,
        ));
        let fonts = ledger.fonts().len() as u64;
        if fonts > 0 {
            out.push(point(
                S::ResolverFontRows,
                None,
                None,
                fonts,
                fonts * std::mem::size_of::<typaxis_document::FontFaceDeclaration>() as u64,
                64,
            ));
        }
        for f in ledger.fonts() {
            let id = Some(f.font_face_id());
            out.push(point(S::ResolverFont, id, None, 0, 0, 64));
            let n = f.family().len() as u64;
            out.push(point(
                S::ResolverFamilyCopy,
                id,
                None,
                u64::from(n > 0),
                n,
                16 * n + 64,
            ));
            let n = f.uri().as_str().len() as u64;
            out.push(point(S::ResolverUriCopy, id, None, 1, n, 16 * n + 64));
        }
        if !images.is_empty() {
            let n = images.len() as u64;
            out.push(point(
                S::ResolverImageRows,
                None,
                None,
                n,
                n * std::mem::size_of::<typaxis_document::ImageDeclaration>() as u64,
                64,
            ));
        }
        for (at, uri) in images.iter().enumerate() {
            let id = Some(typaxis_core::ImageResourceId::new(at as u32));
            let n = uri.len() as u64;
            out.push(point(S::ResolverImage, None, id, 0, 0, 64));
            out.push(point(S::ResolverUriCopy, None, id, 1, n, 16 * n + 64));
        }
        if fonts > 0 {
            out.push(point(
                S::ResolverFontMedia,
                None,
                None,
                fonts,
                fonts * std::mem::size_of::<typaxis_document::FontMediaType>() as u64,
                64,
            ));
        }
        if !images.is_empty() {
            let n = images.len() as u64;
            out.push(point(
                S::ResolverImageMedia,
                None,
                None,
                n,
                n * std::mem::size_of::<typaxis_document::ImageMediaType>() as u64,
                64,
            ));
        }
        let media_bytes = fonts * std::mem::size_of::<typaxis_document::FontMediaType>() as u64
            + images.len() as u64 * std::mem::size_of::<typaxis_document::ImageMediaType>() as u64;
        out.push(point(
            S::ResolverMediaCopy,
            None,
            None,
            0,
            0,
            media_bytes + 64,
        ));
        let header = std::mem::size_of::<[std::sync::atomic::AtomicUsize; 2]>() as u64;
        let payload = std::mem::size_of::<Vec<typaxis_document::FontMediaType>>() as u64
            + std::mem::size_of::<Vec<typaxis_document::ImageMediaType>>() as u64;
        out.push(point(
            S::ResolverMediaOwner,
            None,
            None,
            1,
            header + payload,
            64,
        ));
        out.push(point(S::ResolverSessionOwner, None, None, 1, header, 64));
        out.push(point(
            S::ResolverLimits,
            None,
            None,
            0,
            0,
            2 * std::mem::size_of::<typaxis_core::M4EffectiveResourceLimits>() as u64 + 64,
        ));
        for at in 0..images.len() {
            out.push(point(
                S::ResolverImageCursor,
                None,
                Some(typaxis_core::ImageResourceId::new(at as u32)),
                0,
                0,
                64,
            ));
            if image_uris.is_some() || ledger.images()[at].admitted_safe_vector().is_some() {
                break;
            }
        }
        out.push(point(S::ResolverFinish, None, None, 0, 0, 64));
        out
    }
    fn resource_resolver_reference_fee(
        ledger: &typaxis_resources::AdmittedProductionResourceLedgerV3,
        images: Option<&[String]>,
    ) -> [u64; 3] {
        let mut fee = [0; 3];
        for r in resource_resolver_reference_requests(ledger, images) {
            for (v, n) in fee.iter_mut().zip([r.records, r.storage_bytes, r.work]) {
                *v += n;
            }
        }
        fee
    }

    fn resource_catalog_reference_requests(
        ledger: &typaxis_resources::AdmittedProductionResourceLedgerV3,
        image_uris: Option<&[String]>,
    ) -> Vec<typaxis_resources::ResourceCatalogRequest> {
        use typaxis_resources::{ResourceCatalogRequest as R, ResourceCatalogStage as S};
        let mut out = Vec::new();
        let point = |stage, font_face_id, image_id, records, storage_bytes, work| R {
            stage,
            font_face_id,
            image_id,
            records,
            storage_bytes,
            work,
        };
        let images = match image_uris {
            Some(v) => v.iter().map(String::as_str).collect::<Vec<_>>(),
            None => ledger.images().iter().map(|v| v.uri().as_str()).collect(),
        };
        out.push(point(S::Start, None, None, 0, 0, 64));
        let fonts = ledger.fonts().len() as u64;
        if fonts != 0 {
            out.push(point(
                S::FontRows,
                None,
                None,
                fonts,
                fonts * std::mem::size_of::<typaxis_document::FontFaceDeclaration>() as u64,
                64,
            ));
            out.push(point(
                S::FontMedia,
                None,
                None,
                fonts,
                fonts * std::mem::size_of::<typaxis_document::FontMediaType>() as u64,
                64,
            ));
        }
        for f in ledger.fonts() {
            let id = Some(f.font_face_id());
            out.push(point(S::Font, id, None, 0, 0, 64));
            let len = f.family().len() as u64;
            out.push(point(
                S::FamilyCopy,
                id,
                None,
                u64::from(len > 0),
                len,
                16 * len + 64,
            ));
            let len = f.uri().as_str().len() as u64;
            out.push(point(S::UriCopy, id, None, 1, len, 16 * len + 64));
        }
        if !images.is_empty() {
            let n = images.len() as u64;
            out.push(point(
                S::ImageRows,
                None,
                None,
                n,
                n * std::mem::size_of::<typaxis_document::ImageDeclaration>() as u64,
                64,
            ));
            out.push(point(
                S::ImageMedia,
                None,
                None,
                n,
                n * std::mem::size_of::<typaxis_document::ImageMediaType>() as u64,
                64,
            ));
        }
        for (at, uri) in images.iter().enumerate() {
            let id = Some(typaxis_core::ImageResourceId::new(at as u32));
            let len = uri.len() as u64;
            out.push(point(S::Image, None, id, 0, 0, 64));
            out.push(point(S::UriCopy, None, id, 1, len, 16 * len + 64));
        }
        for f in ledger.fonts() {
            out.push(point(S::FontId, Some(f.font_face_id()), None, 0, 0, 64));
        }
        for at in 0..images.len() {
            out.push(point(
                S::ImageId,
                None,
                Some(typaxis_core::ImageResourceId::new(at as u32)),
                0,
                0,
                64,
            ));
        }
        let slot = std::mem::size_of::<&str>() as u64;
        if fonts != 0 {
            out.push(point(S::FamilyIndex, None, None, fonts, fonts * slot, 64));
        }
        let mut names = std::collections::BTreeSet::new();
        for f in ledger.fonts() {
            let len = f.family().len() as u64;
            let id = Some(f.font_face_id());
            out.push(point(S::FamilyValidate, id, None, 0, 0, 2 * len + 64));
            out.push(point(
                S::FamilyLookup,
                id,
                None,
                0,
                0,
                (names.len() as u64 + 1) * (len + 2) + 1,
            ));
            let moved = names.iter().filter(|s| **s > f.family()).count() as u64;
            out.push(point(S::FamilyInsert, id, None, 0, 0, moved * slot + 64));
            assert!(names.insert(f.family()));
        }
        out.push(point(S::Finish, None, None, 0, 0, 64));
        out
    }
    fn resource_catalog_reference_fee(
        ledger: &typaxis_resources::AdmittedProductionResourceLedgerV3,
        images: Option<&[String]>,
    ) -> [u64; 3] {
        let mut fee = [0; 3];
        for r in resource_catalog_reference_requests(ledger, images) {
            for (v, n) in fee.iter_mut().zip([r.records, r.storage_bytes, r.work]) {
                *v += n;
            }
        }
        fee
    }
    fn fund_resource_catalog_reference(
        accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>,
        ledger: &typaxis_resources::AdmittedProductionResourceLedgerV3,
        images: Option<&[String]>,
    ) {
        for r in resource_catalog_reference_requests(ledger, images)
            .into_iter()
            .chain(resource_resolver_reference_requests(ledger, images))
        {
            accounting
                .debit(
                    typaxis_core::NodeId::new(0),
                    [r.records, r.storage_bytes, r.work],
                )
                .unwrap();
        }
    }

    fn fund_resource_attempt_storage_reference(
        accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>,
        declarations: usize,
    ) {
        if declarations != 0 {
            let bytes = declarations.div_ceil(64) as u64 * 8;
            accounting
                .debit(typaxis_core::NodeId::new(0), [1, bytes, bytes + 64])
                .unwrap();
        }
    }
    fn resource_cff_binding_reference_requests(
        ledger: &typaxis_resources::AdmittedProductionResourceLedgerV3,
    ) -> Vec<typaxis_resources::ResourceCatalogRequest> {
        use typaxis_resources::{ResourceCatalogRequest as R, ResourceCatalogStage as S};
        let f = &ledger.fonts()[0];
        let id = Some(f.font_face_id());
        let family = f.family().len() as u64;
        let uri = f.uri().as_str().len() as u64;
        let size = std::mem::size_of::<typaxis_resources::AdmittedCff1FontV2>() as u64;
        let point = |stage, records, storage_bytes, work| R {
            stage,
            font_face_id: id,
            image_id: None,
            records,
            storage_bytes,
            work,
        };
        vec![
            point(S::CffBindingChecks, 0, 0, 4 * uri + 128),
            point(
                S::CffBindingStorage,
                ledger.fonts().len() as u64,
                ledger.fonts().len() as u64 * size,
                64,
            ),
            point(S::CffBindingInsert, 0, 0, size + 64),
            point(
                S::CffBindingFamily,
                u64::from(family > 0),
                family,
                16 * family + 64,
            ),
            point(S::CffBindingUri, 1, uri, 16 * uri + 64),
        ]
    }
    fn fund_resource_binding_request(
        accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>,
        request: typaxis_resources::ResourceCatalogRequest,
    ) {
        accounting
            .debit(
                typaxis_core::NodeId::new(0),
                [request.records, request.storage_bytes, request.work],
            )
            .unwrap();
    }
    fn fund_resource_media_reference(
        accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>,
        bytes: &[u8],
        ledger: &typaxis_resources::AdmittedProductionResourceLedgerV3,
    ) {
        let requests = resource_cff_binding_reference_requests(ledger);
        fund_resource_binding_request(accounting, requests[0]);
        assert_eq!(&bytes[..4], b"OTTO");
        let rows = u64::from(u16::from_be_bytes(bytes[4..6].try_into().unwrap()));
        accounting
            .debit_work(typaxis_core::NodeId::new(0), 64 * (rows + 1))
            .unwrap();
        for request in requests.into_iter().skip(1) {
            fund_resource_binding_request(accounting, request);
        }
    }
    fn fund_resource_read_reference(
        accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>,
        length: usize,
        uri: &str,
        family: &str,
        candidate_attempts: usize,
        initialize_identity_index: bool,
    ) {
        let owner = typaxis_core::NodeId::new(0);
        let open_path_bytes = if uri.len() < 256 {
            0
        } else {
            uri.len() as u64 + 1
        };
        accounting
            .debit(
                owner,
                [
                    u64::from(open_path_bytes > 0),
                    open_path_bytes,
                    16 * uri.len() as u64 + 64 + if open_path_bytes == 0 { 256 } else { 0 },
                ],
            )
            .unwrap();
        accounting
            .debit_work(
                owner,
                16 * uri.len() as u64 + 128 * uri.split('/').count() as u64 + 64,
            )
            .unwrap();
        accounting.debit_work(owner, 128).unwrap();
        accounting.debit_work(owner, 64).unwrap();
        accounting
            .debit(owner, [1, 2 * std::mem::size_of::<usize>() as u64, 64])
            .unwrap();
        let identity_work =
            typaxis_resources::host_open_identity_work_bound(candidate_attempts).unwrap();
        if initialize_identity_index {
            accounting
                .debit(
                    owner,
                    [
                        candidate_attempts as u64,
                        candidate_attempts as u64
                            * typaxis_resources::HOST_OPEN_IDENTITY_NODE_BYTES as u64,
                        64,
                    ],
                )
                .unwrap();
            accounting
                .debit_work(
                    owner,
                    candidate_attempts as u64
                        * typaxis_resources::HOST_OPEN_IDENTITY_SLOT_BYTES as u64
                        + 2 * identity_work
                        + 64,
                )
                .unwrap();
        }
        accounting.debit_work(owner, identity_work).unwrap();
        accounting
            .debit_work(owner, 64 + 4 * (uri.len() + family.len()) as u64)
            .unwrap();
        accounting.debit_work(owner, 128).unwrap();
        accounting.debit(owner, [1, length as u64, 8256]).unwrap();
        for offset in (0..length).step_by(8192) {
            accounting
                .debit_work(owner, (length - offset).min(8192) as u64 + 64)
                .unwrap();
        }
        let path_bytes = if uri.len() < 256 {
            0
        } else {
            uri.len() as u64 + 1
        };
        accounting
            .debit(
                owner,
                [
                    u64::from(path_bytes > 0),
                    path_bytes,
                    16 * uri.len() as u64 + 64 + if path_bytes == 0 { 256 } else { 0 },
                ],
            )
            .unwrap();
        accounting
            .debit_work(
                owner,
                16 * uri.len() as u64 + 128 * uri.split('/').count() as u64 + 64,
            )
            .unwrap();
        accounting.debit_work(owner, 192).unwrap();
        accounting
            .debit(owner, [1, uri.len() as u64, 64 + 16 * uri.len() as u64])
            .unwrap();
    }
    fn resource_ledger_reference_requests(
        ledger: &typaxis_resources::AdmittedProductionResourceLedgerV3,
    ) -> Vec<(typaxis_resources::ProductionResourceLedgerPointV3, [u64; 3])> {
        use typaxis_resources::{
            AdmittedProductionFontV3 as Font, ProductionResourceLedgerPointV3 as Point,
            ProductionResourceLedgerStageV3 as Stage,
        };
        let mut out = Vec::new();
        let point = |stage, font_face_id, image_id| Point {
            stage,
            font_face_id,
            image_id,
        };
        for i in ledger.images() {
            out.push((point(Stage::ImageIds, None, Some(i.image_id())), [0, 0, 64]));
        }
        // Existing controls use PNG only; SVG ordering and byte comparison have
        // their own real-host control, with a request trace at the ledger seam.
        assert!(ledger
            .images()
            .iter()
            .all(|i| i.admitted_safe_vector().is_none()));
        out.push((
            point(Stage::Aliases, None, None),
            [
                ledger.images().len() as u64,
                (ledger.images().len()
                    * std::mem::size_of::<([u8; 32], &[u8], typaxis_core::ImageResourceId)>())
                    as u64,
                0,
            ],
        ));
        for i in ledger.images() {
            out.push((point(Stage::Aliases, None, Some(i.image_id())), [0, 0, 64]));
        }
        let mut names = std::collections::BTreeSet::new();
        let slot = std::mem::size_of::<(String, typaxis_core::FontFaceId)>() as u64;
        if !ledger.fonts().is_empty() {
            out.push((
                point(Stage::FamilyEntries, None, None),
                [
                    ledger.fonts().len() as u64,
                    ledger.fonts().len() as u64 * slot,
                    0,
                ],
            ));
        }
        for f in ledger.fonts() {
            let len = f.family().len() as u64;
            let id = Some(f.font_face_id());
            let count = names.len() as u64;
            out.push((point(Stage::FamilyValidate, id, None), [0, 0, 2 * len + 64]));
            out.push((
                point(Stage::FamilyLookup, id, None),
                [0, 0, 1 + (count + 1) * (len + 2)],
            ));
            out.push((point(Stage::FamilyNameCopy, id, None), [1, len, 0]));
            out.push((point(Stage::FamilyNameCopy, id, None), [0, 0, len + 64]));
            let moved = names.iter().filter(|n| **n > f.family()).count() as u64;
            out.push((
                point(Stage::FamilyInsert, id, None),
                [0, 0, moved * slot + 64],
            ));
            assert!(names.insert(f.family()));
        }
        for f in ledger
            .fonts()
            .iter()
            .filter(|f| matches!(f, Font::TrueType(_)))
        {
            out.push((
                point(Stage::FontKinds, Some(f.font_face_id()), None),
                [0, 0, 64],
            ));
        }
        let mut native = ledger
            .fonts()
            .iter()
            .filter(|f| matches!(f, Font::TrueType(_)))
            .peekable();
        let mut cff = ledger
            .fonts()
            .iter()
            .filter(|f| matches!(f, Font::Cff1V2(_)))
            .peekable();
        while let (Some(a), Some(b)) = (native.peek(), cff.peek()) {
            out.push((point(Stage::FontConflicts, None, None), [0, 0, 64]));
            if a.font_face_id() < b.font_face_id() {
                native.next();
            } else {
                cff.next();
            }
        }
        out.push((
            point(Stage::Fonts, None, None),
            [
                ledger.fonts().len() as u64,
                (ledger.fonts().len() * std::mem::size_of::<Font>()) as u64,
                0,
            ],
        ));
        for f in ledger.fonts() {
            out.push((
                point(Stage::Fonts, Some(f.font_face_id()), None),
                [0, 0, 64],
            ));
        }
        out.push((
            point(Stage::Images, None, None),
            [
                ledger.images().len() as u64,
                (ledger.images().len() * std::mem::size_of::<typaxis_resources::AdmittedImage>())
                    as u64,
                0,
            ],
        ));
        for i in ledger.images() {
            out.push((point(Stage::Images, None, Some(i.image_id())), [0, 0, 64]));
        }
        for f in ledger.fonts() {
            out.push((
                point(Stage::CanonicalCapacity, Some(f.font_face_id()), None),
                [0, 0, 64],
            ));
        }
        for i in ledger.images() {
            out.push((
                point(Stage::CanonicalCapacity, None, Some(i.image_id())),
                [0, 0, 64],
            ));
        }
        let records = ledger
            .fonts()
            .iter()
            .map(|f| 512 + 6 * (f.family().len() + f.uri().as_str().len()))
            .chain(
                ledger
                    .images()
                    .iter()
                    .map(|i| 512 + 6 * i.uri().as_str().len()),
            )
            .sum::<usize>();
        out.push((
            point(Stage::CanonicalCapacity, None, None),
            [1, (512 + records) as u64, 0],
        ));
        for f in ledger.fonts() {
            out.push((
                point(Stage::CanonicalFonts, Some(f.font_face_id()), None),
                [
                    0,
                    0,
                    (512 + 6 * (f.family().len() + f.uri().as_str().len())) as u64,
                ],
            ));
        }
        for i in ledger.images() {
            out.push((
                point(Stage::CanonicalImages, None, Some(i.image_id())),
                [0, 0, (512 + 6 * i.uri().as_str().len()) as u64],
            ));
        }
        for chunk in ledger.canonical_jcs().as_bytes().chunks(256) {
            out.push((
                point(Stage::CanonicalHash, None, None),
                [0, 0, chunk.len() as u64 + 64],
            ));
        }
        out.push((point(Stage::CanonicalFinish, None, None), [0, 0, 128]));
        out
    }
    fn fund_resource_ledger_reference(
        accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>,
        ledger: &typaxis_resources::AdmittedProductionResourceLedgerV3,
    ) {
        for (_, cells) in resource_ledger_reference_requests(ledger) {
            accounting
                .debit(typaxis_core::NodeId::new(0), cells)
                .unwrap();
        }
    }
    // Observe the generic host protocol independently of the Book adapter.
    // The real scoped Source loan below debits these public request fields.
    fn read_ledger_reference_requests(
        input: &PreparedBookV2Resources,
    ) -> Vec<typaxis_resources::HostReadRequest> {
        struct Trace(Vec<typaxis_resources::HostReadRequest>);
        impl typaxis_resources::HostReadBudget for Trace {
            type Error = std::convert::Infallible;
            const OBSERVE_LEDGER: bool = true;
            fn charge(
                &mut self,
                request: typaxis_resources::HostReadRequest,
            ) -> Result<(), Self::Error> {
                self.0.push(request);
                Ok(())
            }
        }
        let mut trace = Trace(Vec::new());
        input
            .body()
            .provenance()
            .read_ledger_token()
            .unwrap()
            .revalidate_with_budget(&mut trace)
            .unwrap();
        trace.0
    }
    fn read_ledger_request_cells(request: typaxis_resources::HostReadRequest) -> [u64; 3] {
        use typaxis_resources::HostReadStage as S;
        [
            u64::from(
                matches!(
                    request.stage,
                    S::LedgerCandidatePath | S::LedgerOpenedPath | S::LedgerAliasPath
                ) && request.buffer_bytes > 0,
            ),
            request.buffer_bytes,
            request.work,
        ]
    }
    fn fund_read_ledger_reference(
        accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>,
        input: &PreparedBookV2Resources,
    ) {
        for request in read_ledger_reference_requests(input) {
            accounting
                .debit(
                    typaxis_core::NodeId::new(0),
                    read_ledger_request_cells(request),
                )
                .unwrap();
        }
    }
    fn hosted_with_cff<R>(
        original: &PreparedBookV2Resources,
        caps: &M4EffectiveResourceLimits,
        font: Option<&[u8]>,
        run: impl FnOnce(AdmittedBookV2Input, &HostAdmissionContext, &EffectiveConfig, &Temp) -> R,
    ) -> R {
        hosted_with_resources(original, caps, font, None, run)
    }
    fn hosted_with_resources<R>(
        original: &PreparedBookV2Resources,
        caps: &M4EffectiveResourceLimits,
        font: Option<&[u8]>,
        images: Option<&[(&str, &[u8])]>,
        run: impl FnOnce(AdmittedBookV2Input, &HostAdmissionContext, &EffectiveConfig, &Temp) -> R,
    ) -> R {
        let root = Temp(std::env::temp_dir().join(format!(
            "typaxis-initial-style-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )));
        fs::create_dir(&root.0).unwrap();
        let mut raw = typaxis_document_package::book_v2::BookV2DocumentPackageEncoder::new()
            .encode(original.body().styled().body().wire())
            .unwrap();
        if let Some(font) = font {
            let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
            value["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
            value["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                .into();
            raw = serde_json::to_string(&value).unwrap();
        }
        if let Some(images) = images {
            let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
            let template = value["resources"]["images"][0].clone();
            value["resources"]["images"] = images
                .iter()
                .enumerate()
                .map(|(id, (uri, bytes))| {
                    let mut image = template.clone();
                    image["image_id"] = id.into();
                    image["uri"] = (*uri).into();
                    image["media_type"] = "svg-safe-2".into();
                    image["vector_provenance"] = serde_json::json!({
                        "engine_id":"typaxis.test-fixture-copy",
                        "engine_version":"1",
                        "rules_version":"typaxis.test-vector-aliases/1"
                    });
                    image["expected_sha256"] = typaxis_core::sha256(bytes)
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                        .into();
                    image
                })
                .collect::<Vec<_>>()
                .into();
            raw = serde_json::to_string(&value).unwrap();
        }
        fs::write(root.0.join("document-package.json"), raw).unwrap();
        for source in original.body().sources() {
            let p = root.0.join(source.facts().uri().as_str());
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap()
            }
            fs::write(p, source.text().as_bytes()).unwrap();
        }
        for font in original.resources().fonts() {
            fs::write(root.0.join(font.uri().as_str()), font.bytes()).unwrap();
        }
        if let Some(font) = font {
            fs::write(
                root.0.join(original.resources().fonts()[0].uri().as_str()),
                font,
            )
            .unwrap();
        }
        for image in original.resources().images() {
            fs::write(root.0.join(image.uri().as_str()), image.bytes()).unwrap();
        }
        if let Some(images) = images {
            for (uri, bytes) in images {
                fs::write(root.0.join(uri), bytes).unwrap();
            }
        }
        let context = HostAdmissionContext::new(
            HostPath::new(root.0.join("document-package.json")).unwrap(),
            HostPath::new(root.0.clone()).unwrap(),
            None,
            vec![],
        );
        let config = EffectiveConfig::new_for_contract_with_m4_limits(
            DocumentPackageContractId::V1_4,
            false,
            PdfStreamCompression::Flate,
            vec![ConfigResourceRoot::ProjectRoot],
            DEFAULT_ALLOWED_URI_SCHEMES
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            EffectiveDataVersions::new(
                REGISTERED_UNICODE_VERSION,
                REGISTERED_JAPANESE_LINE_BREAK_VERSION,
            )
            .unwrap(),
            caps.base().get().clone(),
            caps.extension().get().clone(),
        )
        .unwrap();
        let (session, raw) = HostBookV2InputSession::open(
            MachineInputHostOptions::new(
                HostPath::new(root.0.join("document-package.json")).unwrap(),
                None,
            ),
            caps.base(),
        )
        .unwrap();
        let decoded = session
            .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(caps.base()))
            .unwrap();
        let sources = session.admit_sources(&decoded, caps.base()).unwrap();
        let input = session.finish(raw, decoded, sources).unwrap();
        run(input, &context, &config, &root)
    }
    fn rsw(o: BookV2PdfConvergenceObservation) -> [u64; 3] {
        [o.record_charge(), o.spool_charge(), o.work_steps()]
    }

    #[test]
    #[ignore = "requires unchanged original CFF through TYPAXIS_HARFRUST_CACHE_FONTS"]
    fn observed_base_cmap_initial_admission_shares_real_command_prefix_and_typed_refusal() {
        let paths = std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap())
            .unwrap();
        let font = std::fs::read(paths.lines().next().unwrap()).unwrap();
        assert_eq!(
            typaxis_core::sha256(&font),
            [
                0x66, 0xef, 0x32, 0x70, 0xe6, 0x86, 0x90, 0x61, 0x2e, 0x8b, 0xf9, 0x82, 0xac, 0xfa,
                0xd0, 0xe8, 0xb4, 0x02, 0x12, 0xce, 0x64, 0x66, 0x1c, 0xce, 0x2b, 0xb6, 0xd3, 0xa9,
                0x8a, 0xc8, 0x47, 0x17,
            ]
        );
        with_convergence_probe_fixture("no-page", |original, caps, maximum, _| {
            let mut style = [0u64; 3];
            let mut ledger_reference = None;
            hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
                let body =
                    typaxis_syntax::book_v2::prepare_admitted_book_v2_body_with_counted_style(
                        input,
                        caps.base(),
                        &mut |r, s, w| {
                            for (cell, n) in style.iter_mut().zip([r, s, w]) {
                                *cell += n;
                            }
                            Ok::<_, std::convert::Infallible>(())
                        },
                    )
                    .unwrap();
                ledger_reference = Some(
                    crate::book_v2_resources::prepare_book_v2_resources(
                        body, context, config, caps,
                    )
                    .unwrap(),
                );
            });
            let [mut r, mut s, mut w] = style;
            let mut cff_prefix = [0; 3];
            typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                caps.base(),
                caps.base().get().max_fragments,
                maximum,
                |accounting| {
                    fund_resource_catalog_reference(accounting, original.resources(), None);
                    fund_resource_attempt_storage_reference(
                        accounting,
                        original.resources().fonts().len(),
                    );
                    fund_resource_read_reference(
                        accounting,
                        font.len(),
                        original.resources().fonts()[0].uri().as_str(),
                        original.resources().fonts()[0].family(),
                        2 + original.resources().fonts().len()
                            + original.resources().images().len(),
                        true,
                    );
                    fund_resource_media_reference(accounting, &font, original.resources());
                    typaxis_font::admit_sfnt_cff1_v2_owned_with_admission_budget(
                        font.clone(),
                        0,
                        caps,
                        &mut accounting.resource_admission_budget(typaxis_core::NodeId::new(0)),
                    )
                    .unwrap()
                    .unwrap();
                    cff_prefix = accounting.counters();
                    for font in original.resources().fonts().iter().skip(1) {
                        fund_resource_read_reference(
                            accounting,
                            font.bytes().len(),
                            font.uri().as_str(),
                            font.family(),
                            2 + original.resources().fonts().len()
                                + original.resources().images().len(),
                            false,
                        );
                    }
                    fund_resource_attempt_storage_reference(
                        accounting,
                        original.resources().images().len(),
                    );
                    for image in original.resources().images() {
                        fund_resource_read_reference(
                            accounting,
                            image.bytes().len(),
                            image.uri().as_str(),
                            "",
                            2 + original.resources().fonts().len()
                                + original.resources().images().len(),
                            false,
                        );
                    }
                    fund_resource_ledger_reference(
                        accounting,
                        ledger_reference.as_ref().unwrap().resources(),
                    );
                    fund_read_ledger_reference(accounting, ledger_reference.as_ref().unwrap());
                },
            );
            let expected = [r, s, w];
            let mut warm = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
            hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
                prepare_counted_book_v2_column_resources(input, context, config, caps, &mut warm)
                    .unwrap();
            });
            assert_eq!(rsw(warm.observation()), expected);
            assert_eq!(warm.observation().output_charge(), 0);
            let mut short = BookV2ColumnPdfConvergenceBudget::new(caps, cff_prefix[2] - 1);
            hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
                let error = prepare_counted_book_v2_column_resources(
                    input, context, config, caps, &mut short,
                )
                .unwrap_err();
                match error {
                    BookV2ColumnInitialStylePreparationError::Admission { failure, body } => {
                        assert_eq!(failure.font_face_id.get(), 0);
                        assert_eq!(
                            match failure.failure {
                                typaxis_font::CffAdmissionBudgetFailureV2::Metadata(f) => {
                                    assert_eq!(
                                        f.point.stage,
                                        typaxis_font::CffMetadataStageV2::IdentityDigest
                                    );
                                    assert_eq!(f.point.file_offset, None);
                                    f.error
                                }
                                other => panic!("expected identity digest refusal, got {other:?}"),
                            },
                            typaxis_shaping::book_v2::BookV2ObservedAccountingError::Work {
                                owner: typaxis_core::NodeId::new(0)
                            }
                        );
                        assert_eq!(body.styled().body().limits(), caps.base());
                    }
                    other => panic!("expected typed base cmap refusal, got {other:?}"),
                }
            });
            let accepted = rsw(short.observation());
            assert!(accepted[2] < expected[2]);
            hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
                assert!(matches!(
                    prepare_counted_book_v2_column_resources(
                        input, context, config, caps, &mut short
                    ),
                    Err(BookV2ColumnInitialStylePreparationError::Source(_))
                        | Err(BookV2ColumnInitialStylePreparationError::Admission { .. })
                        | Err(BookV2ColumnInitialStylePreparationError::Catalog { .. })
                ));
            });
            let retried = rsw(short.observation());
            assert!(retried[2] >= accepted[2] && retried[2] < expected[2]);
            assert!(retried[0] >= accepted[0] && retried[1] >= accepted[1]);
            eprintln!("initial CFF base cmap style={style:?}, full={expected:?}, short={accepted:?}, retry={retried:?}, font subject=0");
        });
    }

    #[test]
    fn book_v2_column_counted_initial_style_genuine_admission_same_budget_full_pdf_and_retry() {
        with_convergence_probe_fixture("no-page", |original, caps, maximum, mode| {
            let mut legacy = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
            let baseline = with_budgeted_converged_book_v2_column_pdf(
                original,
                caps,
                mode,
                &mut legacy,
                4,
                |roots, _| roots.pdf().bytes().to_vec(),
            )
            .unwrap();
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
            let before_preparation = budget.observation();
            hosted(original, caps, |input, context, config, _| {
                let prepared = prepare_counted_book_v2_column_resources(
                    input,
                    context,
                    config,
                    caps,
                    &mut budget,
                )
                .unwrap();
                let prepared_prefix = budget.observation();
                for (i, v) in rsw(prepared_prefix).into_iter().enumerate() {
                    assert!(v > rsw(before_preparation)[i]);
                }
                assert_eq!(
                    prepared_prefix.output_charge(),
                    before_preparation.output_charge()
                );
                assert_eq!(
                    prepared.body().styled().body().canonical_jcs_sha256(),
                    original.body().styled().body().canonical_jcs_sha256()
                );
                // Retry the same authentic resource owner: the command session
                // deliberately rejects a newly admitted Arc identity.
                for attempt in 0..2 {
                    let before = budget.observation();
                    let result = with_budgeted_converged_book_v2_column_pdf(
                        &prepared,
                        caps,
                        mode,
                        &mut budget,
                        4,
                        |roots, _| {
                            assert_eq!(
                                roots.caller_before(),
                                [
                                    before.record_charge(),
                                    before.spool_charge(),
                                    before.output_charge(),
                                    before.work_steps()
                                ]
                            );
                            assert_eq!(roots.pdf().bytes(), baseline);
                            Err::<(), _>("initial style application sentinel")
                        },
                    )
                    .unwrap();
                    assert_eq!(result, Err("initial style application sentinel"));
                    let after = budget.observation();
                    for (i, v) in rsw(after).into_iter().enumerate() {
                        assert!(v >= rsw(before)[i]);
                    }
                    eprintln!("initial style actual attempt={attempt} before={:?} prepared={:?} after={:?}",
                        rsw(before), rsw(prepared_prefix), rsw(after));
                }
            });
        });
    }
    #[test]
    fn book_v2_column_counted_initial_style_work_exact_short_and_later_resource_error_keep_prefix()
    {
        with_convergence_probe_fixture("no-page", |original, caps, maximum, _| {
            let mut warm = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
            hosted(original, caps, |input, context, config, _| {
                prepare_counted_book_v2_column_resources(input, context, config, caps, &mut warm)
                    .unwrap();
            });
            let exact = warm.observation();
            let mut style = [0; 3];
            hosted(original, caps, |input, _, _, _| {
                typaxis_syntax::book_v2::prepare_admitted_book_v2_body_with_counted_style(
                    input,
                    caps.base(),
                    &mut |r, s, w| {
                        for (cell, amount) in style.iter_mut().zip([r, s, w]) {
                            *cell += amount;
                        }
                        Ok::<_, std::convert::Infallible>(())
                    },
                )
                .unwrap();
            });
            for short in [false, true] {
                let mut budget = BookV2ColumnPdfConvergenceBudget::new(
                    caps,
                    exact.work_steps() - u64::from(short),
                );
                hosted(original, caps, |input, context, config, _| {
                    let result = prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        caps,
                        &mut budget,
                    );
                    if short {
                        assert!(
                            matches!(result,Err(BookV2ColumnInitialStylePreparationError::Ledger { ref failure, .. })
                                if failure.point.stage == typaxis_resources::ProductionResourceLedgerStageV3::CanonicalFinish
                                && matches!(failure.error, typaxis_shaping::book_v2::BookV2ObservedAccountingError::Work { owner } if owner.get()==0))
                        );
                        assert!(budget.observation().work_steps() < exact.work_steps());
                    } else {
                        result.unwrap();
                        assert_eq!(rsw(budget.observation()), rsw(exact));
                    }
                    assert_eq!(budget.observation().output_charge(), 0);
                });
            }
            let mut failed = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
            hosted(original, caps, |input, context, config, root| {
                fs::remove_file(root.0.join(original.resources().fonts()[0].uri().as_str()))
                    .unwrap();
                assert!(matches!(
                    prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        caps,
                        &mut failed
                    ),
                    Err(BookV2ColumnInitialStylePreparationError::Resources(_))
                ));
                // Opening is now funded before discovering the absent file.
                // This fixture has one admitted root and never reaches lock/owner.
                let uri = original.resources().fonts()[0].uri().as_str();
                let mut missing_prefix = style;
                for (v, n) in missing_prefix
                    .iter_mut()
                    .zip(resource_resolver_reference_fee(original.resources(), None))
                {
                    *v += n;
                }
                for (v, n) in missing_prefix
                    .iter_mut()
                    .zip(resource_catalog_reference_fee(original.resources(), None))
                {
                    *v += n;
                }
                if uri.len() >= 256 {
                    missing_prefix[0] += 1;
                    missing_prefix[1] += uri.len() as u64 + 1;
                }
                missing_prefix[2] += 32 * uri.len() as u64
                    + 128 * uri.split('/').count() as u64
                    + 128
                    + if uri.len() < 256 { 256 } else { 0 };
                assert_eq!(rsw(failed.observation()), missing_prefix);
            });
        });
    }
}
