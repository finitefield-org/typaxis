//! CID CFF /2 sfnt admission. The resource/profile registry and PDF publication
//! remain separate gates; a caller cannot construct this validated owner.
use super::super::font_tables::{self, Lookup};
use super::*;

#[derive(Debug)]
pub struct Cff1AdmissionV2 {
    table_records: Vec<TableRecord>,
    family: String,
    subfamily: String,
    effective_limits: M4EffectiveResourceLimits,
    source: SourceBytesV2,
    source_sha256: [u8; 32],
    fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    glyph_count: u16,
    units_per_em: u16,
    postscript_name: String,
    embedding_permission: Cff1EmbeddingPermission,
    advances: Vec<u16>,
    left_side_bearings: Vec<i16>,
    cmap: CffCmapV2,
    program: CffProgramInspectionV2,
}
impl Cff1AdmissionV2 {
    pub(crate) fn table_count(&self)->usize { self.table_records.len() }
    pub(super) fn glyph_failure_context(&self, failure: &CffGlyphFailureV2) -> FontFailureContext {
        let mut context = FontFailureContext::new(0);
        context.table(*b"CFF ", &self.table_records, FontFailurePhase::Charstring);
        if let Ok(os2) = self.table_bytes(b"OS/2") {
            context.permission_v2(os2);
        }
        context.gid = Some(u32::from(failure.gid));
        context.fd = failure.fd.map(u16::from);
        context.operator = failure.operator;
        if let Some(offset) = failure.table_offset {
            context.at(offset);
            context.position_is_exact = failure.position_is_exact;
            context.position_is_end = !failure.position_is_exact;
        }
        context.reason = match failure.kind {
            Cff1Error::CharstringOperationLimit
            | Cff1Error::OutlineSegmentLimit
            | Cff1Error::SubsetByteLimit => FontFailureReason::BudgetExceeded,
            Cff1Error::InvalidSelectedGlyph => FontFailureReason::InvalidSelectedGlyph,
            Cff1Error::ReceiptMismatch | Cff1Error::InvalidGlyphClosure => {
                FontFailureReason::Invariant
            }
            _ if failure.reason == CffGlyphFailureReasonV2::InvalidWidth => {
                FontFailureReason::InvalidCharstringWidth
            }
            _ if failure.reason == CffGlyphFailureReasonV2::UnsupportedOperator => {
                FontFailureReason::UnsupportedCffOperator
            }
            _ if failure.reason == CffGlyphFailureReasonV2::ReservedOperator => {
                FontFailureReason::ReservedCffOperator
            }
            _ => FontFailureReason::InvalidCharstring,
        };
        context
    }
    pub(crate) fn table_bytes(&self, tag: &[u8; 4]) -> Result<&[u8], Cff1Error> {
        let r = self
            .table_records
            .iter()
            .find(|r| &r.tag == tag)
            .ok_or(Cff1Error::InvalidSubset)?;
        Ok(&self.source[r.offset..r.offset + r.length])
    }
    pub(super) fn family_names(&self) -> (&str, &str) {
        (&self.family, &self.subfamily)
    }
    pub(super) fn horizontal_metrics(&self) -> (&[u16], &[i16]) {
        (&self.advances, &self.left_side_bearings)
    }
    pub fn effective_limits(&self) -> &M4EffectiveResourceLimits {
        &self.effective_limits
    }
    pub fn source(&self) -> &[u8] {
        &self.source
    }
    pub fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub fn glyph_count(&self) -> u16 {
        self.glyph_count
    }
    pub fn units_per_em(&self) -> u16 {
        self.units_per_em
    }
    pub fn postscript_name(&self) -> &str {
        &self.postscript_name
    }
    pub fn embedding_permission(&self) -> Cff1EmbeddingPermission {
        self.embedding_permission
    }
    pub fn cmap(&self) -> &CffCmapV2 {
        &self.cmap
    }
    pub fn program(&self) -> &CffProgramInspectionV2 {
        &self.program
    }
    pub fn horizontal_metric(&self, gid: u16) -> Option<(u16, i16)> {
        Some((
            *self.advances.get(usize::from(gid))?,
            *self.left_side_bearings.get(usize::from(gid))?,
        ))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cff1FailureKindV2 {
    Sfnt(Cff1Error),
    Table(CffTableFailureKindV2),
    Program(CffProgramErrorKindV2),
    SourceByteLimit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cff1FailureV2 {
    pub kind: Cff1FailureKindV2,
    pub context: FontFailureContext,
}
impl std::fmt::Display for Cff1FailureV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CFF /2 {:?}: {:?}", self.kind, self.context)
    }
}
impl std::error::Error for Cff1FailureV2 {}

pub fn admit_sfnt_cff1_v2(
    source: Arc<[u8]>,
    face_index: u32,
    limits: &M4EffectiveResourceLimits,
) -> Result<Cff1AdmissionV2, Cff1FailureV2> {
    match admit_sfnt_cff1_v2_with_base_cmap_budget(
        source,
        face_index,
        limits,
        &mut super::base_cmap_budget::NoBaseCmapBudget,
    ) {
        Ok(value) => value,
        Err(failure) => match failure.error {},
    }
}
/// Observe only base-cmap work/mapping vectors during genuine sfnt admission.
/// Other tables, source/program storage and admission hashing remain separate.
pub fn admit_sfnt_cff1_v2_with_base_cmap_budget<B: CffBaseCmapBudgetV2>(
    source: Arc<[u8]>,
    face_index: u32,
    limits: &M4EffectiveResourceLimits,
    budget: &mut B,
) -> CffBaseCmapBudgetResultV2<Cff1AdmissionV2, Cff1FailureV2, B::Error> {
    admit_sfnt_cff1_v2_with_cmap_budget(
        source,
        face_index,
        limits,
        &mut super::cmap_budget::BaseOnly(budget),
    )
    .map_err(CffCmapBudgetFailureV2::base_only)
}
/// Observe the complete cmap phase while preserving all other admission phases.
pub fn admit_sfnt_cff1_v2_with_cmap_budget<B: CffCmapBudgetV2>(
    source: Arc<[u8]>,
    face_index: u32,
    limits: &M4EffectiveResourceLimits,
    budget: &mut B,
) -> CffCmapBudgetResultV2<Cff1AdmissionV2, Cff1FailureV2, B::Error> {
    admit_sfnt_cff1_v2_with_admission_budget(
        source,
        face_index,
        limits,
        &mut super::admission_budget::CmapOnly(budget),
    )
    .map_err(CffAdmissionBudgetFailureV2::cmap_only)
}
/// Observe SFNT preflight and complete cmap, with optional metrics, names and
/// admission hashes. Other tables and program inspection remain separate.
pub fn admit_sfnt_cff1_v2_with_admission_budget<B: CffAdmissionBudgetV2>(
    source: Arc<[u8]>,
    face_index: u32,
    limits: &M4EffectiveResourceLimits,
    budget: &mut B,
) -> CffAdmissionBudgetResultV2<Cff1AdmissionV2, Cff1FailureV2, B::Error> {
    let context = match source_preflight(source.len(), face_index, limits) {
        Ok(value) => value,
        Err(failure) => return Ok(Err(failure)),
    };
    admit_prechecked_source(source.into(), context, limits, budget)
}

/// Admit an owned stable buffer without a second allocation of its byte extent.
/// The caller owns the input allocation; only the shared owner is created here.
pub fn admit_sfnt_cff1_v2_owned(
    source: Vec<u8>,
    face_index: u32,
    limits: &M4EffectiveResourceLimits,
) -> Result<Cff1AdmissionV2, Cff1FailureV2> {
    match admit_sfnt_cff1_v2_owned_with_admission_budget(
        source,
        face_index,
        limits,
        &mut admission_budget::UnobservedAdmission,
    ) {
        Ok(value) => value,
        Err(failure) => match failure {
            CffAdmissionBudgetFailureV2::Sfnt(f) => match f.error {},
            CffAdmissionBudgetFailureV2::Cmap(CffCmapBudgetFailureV2::Base(f)) => match f.error {},
            CffAdmissionBudgetFailureV2::Cmap(CffCmapBudgetFailureV2::Variation(f)) => {
                match f.error {}
            }
        },
    }
}
pub fn admit_sfnt_cff1_v2_owned_with_base_cmap_budget<B: CffBaseCmapBudgetV2>(
    source: Vec<u8>,
    face_index: u32,
    limits: &M4EffectiveResourceLimits,
    budget: &mut B,
) -> CffBaseCmapBudgetResultV2<Cff1AdmissionV2, Cff1FailureV2, B::Error> {
    admit_sfnt_cff1_v2_owned_with_cmap_budget(
        source,
        face_index,
        limits,
        &mut super::cmap_budget::BaseOnly(budget),
    )
    .map_err(CffCmapBudgetFailureV2::base_only)
}
pub fn admit_sfnt_cff1_v2_owned_with_cmap_budget<B: CffCmapBudgetV2>(
    source: Vec<u8>,
    face_index: u32,
    limits: &M4EffectiveResourceLimits,
    budget: &mut B,
) -> CffCmapBudgetResultV2<Cff1AdmissionV2, Cff1FailureV2, B::Error> {
    admit_sfnt_cff1_v2_owned_with_admission_budget(
        source,
        face_index,
        limits,
        &mut super::admission_budget::CmapOnly(budget),
    )
    .map_err(CffAdmissionBudgetFailureV2::cmap_only)
}
/// Same native grammar and identity as the slice API. Owner billing is an
/// independent opt-in, preserving every existing phase-only budget adapter.
pub fn admit_sfnt_cff1_v2_owned_with_admission_budget<B: CffAdmissionBudgetV2>(
    source: Vec<u8>,
    face_index: u32,
    limits: &M4EffectiveResourceLimits,
    budget: &mut B,
) -> CffAdmissionBudgetResultV2<Cff1AdmissionV2, Cff1FailureV2, B::Error> {
    let context = match source_preflight(source.len(), face_index, limits) {
        Ok(value) => value,
        Err(failure) => return Ok(Err(failure)),
    };
    let source = source_owner::from_owned(source, budget)?;
    admit_prechecked_source(source, context, limits, budget)
}

fn source_preflight(
    length: usize,
    face_index: u32,
    limits: &M4EffectiveResourceLimits,
) -> Result<FontFailureContext, Cff1FailureV2> {
    let mut context = FontFailureContext::new(face_index);
    if face_index != 0 {
        return Err(Cff1FailureV2 {
            kind: Cff1FailureKindV2::Sfnt(Cff1Error::InvalidFaceIndex),
            context,
        });
    }
    if length as u64 > limits.base().get().max_font_bytes {
        context.phase = FontFailurePhase::SfntDirectory;
        context.reason = FontFailureReason::BudgetExceeded;
        context.limit = Some(limits.base().get().max_font_bytes);
        context.observed = Some(length as u64);
        return Err(Cff1FailureV2 {
            kind: Cff1FailureKindV2::SourceByteLimit,
            context,
        });
    }
    Ok(context)
}
fn admit_prechecked_source<B: CffAdmissionBudgetV2>(
    source: SourceBytesV2,
    mut context: FontFailureContext,
    limits: &M4EffectiveResourceLimits,
    budget: &mut B,
) -> CffAdmissionBudgetResultV2<Cff1AdmissionV2, Cff1FailureV2, B::Error> {
    macro_rules! native {
        ($value:expr) => {
            match $value {
                Ok(value) => value,
                Err(failure) => return Ok(Err(failure)),
            }
        };
    }
    let records = match super::super::sfnt_preflight::with_v2_budget(
        &source,
        *limits.extension().get(),
        &mut context,
        budget,
    )
    .map_err(CffAdmissionBudgetFailureV2::Sfnt)?
    {
        Ok(v) => v,
        Err(super::super::sfnt_preflight::NativeFailure::Native(kind)) => {
            return Ok(Err(Cff1FailureV2 {
                kind: Cff1FailureKindV2::Sfnt(kind),
                context,
            }))
        }
        Err(super::super::sfnt_preflight::NativeFailure::Allocation) => {
            context.directory(None, None, FontFailureReason::BudgetExceeded);
            return Ok(Err(Cff1FailureV2 {
                kind: Cff1FailureKindV2::Table(CffTableFailureKindV2::AllocationFailure),
                context,
            }));
        }
    };
    let tables = native!(font_tables::Tables::new(&source, &records, budget)
        .map_err(CffAdmissionBudgetFailureV2::Tables)?
        .map_err(|kind| Cff1FailureV2 {
            kind: Cff1FailureKindV2::Sfnt(kind),
            context,
        }));
    macro_rules! tables_work {
        ($stage:ident,$tag:expr,$amount:expr) => {
            font_tables::work(
                budget,
                font_tables::point(CffTableAdmissionStageV2::$stage, $tag, None),
                $amount,
            )
            .map_err(CffAdmissionBudgetFailureV2::Tables)?
        };
    }
    macro_rules! table_bytes {
        ($tag:expr) => {{
            tables_work!(TableLookup, Some(*$tag), font_tables::LOOKUP_ALLOWANCE);
            tables[$tag].bytes
        }};
    }
    macro_rules! optional_bytes {
        ($tag:expr) => {{
            tables_work!(TableLookup, Some(*$tag), font_tables::LOOKUP_ALLOWANCE);
            tables.get($tag).map(|v| v.bytes)
        }};
    }
    macro_rules! context_table {
        ($tag:expr,$phase:expr) => {{
            tables_work!(ContextLookup, Some(*$tag), records.len() as u64);
            context.table(*$tag, &records, $phase);
        }};
    }
    macro_rules! table {
        ($tag:expr,$operation:expr) => {{
            context_table!($tag, FontFailurePhase::TableDecode);
            let bytes = table_bytes!($tag);
            let decoded = if matches!($tag, b"head" | b"maxp" | b"hhea" | b"OS/2" | b"post") {
                font_tables::fixed(budget, *$tag, || ($operation)(bytes))
                    .map_err(CffAdmissionBudgetFailureV2::Tables)?
            } else {
                ($operation)(bytes)
            };
            match decoded {
                Ok(value) => value,
                Err(kind) => {
                    if kind == Cff1Error::GlyphLimit {
                        context.field(4);
                        context.reason = FontFailureReason::BudgetExceeded;
                        context.limit = Some(u64::from(limits.extension().get().max_font_glyphs));
                        context.observed =
                            read_u16(table_bytes!(b"maxp"), 4, Cff1Error::InvalidMaxp)
                                .ok()
                                .map(u64::from);
                    }
                    return Ok(Err(Cff1FailureV2 {
                        kind: Cff1FailureKindV2::Sfnt(kind),
                        context,
                    }));
                }
            }
        }};
    }
    macro_rules! metadata {
        ($operation:expr) => {
            match $operation {
                Ok(value) => value,
                Err(super::super::font_metadata::Failure::Budget(mut failure)) => {
                    if let Some(tag) = failure.point.table_tag {
                        let record = records.iter().find(|r| r.tag == tag).unwrap();
                        failure.bind_file_offset(record.offset);
                    }
                    return Err(CffAdmissionBudgetFailureV2::Metadata(failure));
                }
                Err(super::super::font_metadata::Failure::Native(kind)) => {
                    return Ok(Err(Cff1FailureV2 {
                        kind: Cff1FailureKindV2::Sfnt(kind),
                        context,
                    }));
                }
                Err(super::super::font_metadata::Failure::Allocation) => {
                    context.reason = FontFailureReason::BudgetExceeded;
                    return Ok(Err(Cff1FailureV2 {
                        kind: Cff1FailureKindV2::Table(CffTableFailureKindV2::AllocationFailure),
                        context,
                    }));
                }
            }
        };
    }
    // Required-table presence and ranges are already established by preflight.
    let head = table!(b"head", parse_head);
    let glyph_count = table!(b"maxp", |bytes| parse_maxp(bytes, limits));
    let hhea = table!(b"hhea", parse_hhea);
    context_table!(b"hmtx", FontFailurePhase::TableDecode);
    let (advances, left_side_bearings) = if B::OBSERVE_METADATA {
        metadata!(super::super::font_metadata::hmtx(
            table_bytes!(b"hmtx"),
            glyph_count,
            hhea.number_of_h_metrics,
            budget
        ))
    } else {
        table!(b"hmtx", |bytes| parse_hmtx(
            bytes,
            glyph_count,
            hhea.number_of_h_metrics
        ))
    };
    context_table!(b"name", FontFailurePhase::TableDecode);
    let names = if B::OBSERVE_METADATA {
        metadata!(super::super::font_metadata::names(
            table_bytes!(b"name"),
            budget
        ))
    } else {
        table!(b"name", parse_name)
    };
    let os2 = table!(b"OS/2", parse_os2);
    table!(b"post", parse_post);
    context_table!(b"OS/2", FontFailurePhase::EmbeddingPermission);
    tables_work!(EmbeddingPermission, Some(*b"OS/2"), 1);
    context.permission_v2(os2.bytes);
    context.field(8);
    context.reason = FontFailureReason::RestrictedEmbedding;
    let permission = native!(
        embedding_permission_v2(os2.fs_type).map_err(|kind| Cff1FailureV2 {
            kind: Cff1FailureKindV2::Sfnt(kind),
            context,
        })
    );
    native!(super::vertical::with_budget(
        glyph_count,
        optional_bytes!(b"VORG"),
        optional_bytes!(b"vhea"),
        optional_bytes!(b"vmtx"),
        budget,
    )
    .map_err(|mut f| {
        f.bind_file_offset(&records);
        CffAdmissionBudgetFailureV2::Tables(f)
    })?
    .map_err(|e| table_failure(e, &records, context)));
    let cmap_result =
        validate_cff_cmap_v2_with_cmap_budget(table_bytes!(b"cmap"), glyph_count, budget)
            .map_err(|mut failure| {
                let record = records
                    .iter()
                    .find(|record| record.tag == *b"cmap")
                    .unwrap();
                failure.bind_file_offset(record.offset);
                failure
            })
            .map_err(CffAdmissionBudgetFailureV2::Cmap)?;
    let cmap = native!(cmap_result.map_err(|e| match e {
        CffCmapFailureV2::Variation(e) | CffCmapFailureV2::Storage(e) => {
            table_failure(e, &records, context)
        }
        CffCmapFailureV2::Base { kind, context: c } => {
            context.table(*b"cmap", &records, FontFailurePhase::Cmap);
            if let Some(offset) = c.table_offset {
                context.field(offset as usize);
            }
            context.reason = c.reason;
            context.cmap_format = c.cmap_format;
            Cff1FailureV2 {
                kind: Cff1FailureKindV2::Sfnt(kind),
                context,
            }
        }
    }));
    // At most 24 local table queries and eight context-directory walks. This
    // does not fund read-fonts' own optional-table traversals or IntSet storage.
    tables_work!(
        OptionalDispatch,
        None,
        24 * font_tables::LOOKUP_ALLOWANCE + 8 * records.len() as u64
    );
    native!(local_optional::finish(validate_optional_tables_with_budget(
        &source,
        &tables,
        glyph_count,
        &records,
        &mut context,
        crate::math::validation::VariantOrder::CffV2Nondecreasing,
        budget
    ))
    .map_err(CffAdmissionBudgetFailureV2::Tables)?
    .map_err(|kind| Cff1FailureV2 {
        kind: Cff1FailureKindV2::Sfnt(kind),
        context,
    },));
    context_table!(b"CFF ", FontFailurePhase::CffIndex);
    let cff = table_bytes!(b"CFF ");
    // The authenticated table slice and source Arc share one allocation.
    let start = cff.as_ptr() as usize - source.as_ptr() as usize;
    if B::OBSERVE_PROGRAM {
        budget.work_program(1).map_err(|error| {
            CffAdmissionBudgetFailureV2::Program(CffProgramBudgetFailureV2 {
                error,
                point: CffProgramWorkPointV2 {
                    stage: CffProgramStageV2::Owner,
                    table_offset: None,
                    file_offset: None,
                    fd: None,
                    operator: None,
                },
            })
        })?;
    }
    let program = native!(inspect_program_range_with_budget(
        source.clone(),
        Some(ProgramSpan {
            start,
            end: start + cff.len()
        }),
        glyph_count,
        head.units_per_em,
        limits.extension().get().max_cff_subroutines,
        Some((&names.postscript_name, head.bbox)),
        budget,
    )
    .map_err(|mut e| {
        e.bind_file_offset(start);
        CffAdmissionBudgetFailureV2::Program(e)
    })?
    .map_err(|e| {
        context.field(e.table_offset);
        context.fd = e.fd;
        context.operator = e.operator;
        context.limit = e.limit;
        context.observed = e.observed;
        context.reason = if e.limit.is_some() || e.kind == CffProgramErrorKindV2::AllocationFailure
        {
            FontFailureReason::BudgetExceeded
        } else if e.kind == CffProgramErrorKindV2::UnsupportedOperator {
            FontFailureReason::UnsupportedCffOperator
        } else {
            FontFailureReason::InvalidTable
        };
        Cff1FailureV2 {
            kind: Cff1FailureKindV2::Program(e.kind),
            context,
        }
    }));
    let (source_sha256, fingerprint) = if B::OBSERVE_METADATA {
        metadata!(super::super::font_metadata::hashes(
            &source,
            limits.fingerprint(),
            budget
        ))
    } else {
        let source_sha256 = sha256(&source);
        let mut identity = String::from(
            "{\"algorithm\":\"typaxis.sfnt-cff1-admission/2\",\"face_index\":0,\"limits_fingerprint\":",
        );
        push_hash(&mut identity, limits.fingerprint());
        identity.push_str(
            ",\"resource_profile\":\"typaxis.resource-profile/sfnt-cff1/2\",\"source_sha256\":",
        );
        push_hash(&mut identity, source_sha256);
        identity.push('}');
        (source_sha256, sha256(identity.as_bytes()))
    };
    let units_per_em = head.units_per_em;
    Ok(Ok(Cff1AdmissionV2 {
        table_records: records,
        family: names.family,
        subfamily: names.subfamily,
        effective_limits: limits.clone(),
        source,
        source_sha256,
        fingerprint,
        limits_fingerprint: limits.fingerprint(),
        glyph_count,
        units_per_em,
        postscript_name: names.postscript_name,
        embedding_permission: permission,
        advances,
        left_side_bearings,
        cmap,
        program,
    }))
}

fn table_failure(
    e: CffTableFailureV2,
    records: &[TableRecord],
    mut context: FontFailureContext,
) -> Cff1FailureV2 {
    context.table(
        e.table,
        records,
        if e.table == *b"cmap" {
            FontFailurePhase::Cmap
        } else {
            FontFailurePhase::TableDecode
        },
    );
    context.field(e.table_offset);
    context.limit = e.limit;
    context.observed = e.observed;
    if e.limit.is_some() {
        context.reason = FontFailureReason::BudgetExceeded;
    }
    Cff1FailureV2 {
        kind: Cff1FailureKindV2::Table(e.kind),
        context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod math_variant_tests {
        include!("cff_v2_math_variant_tests.rs");
    }
    fn limits() -> M4EffectiveResourceLimits {
        M4EffectiveResourceLimits::new(
            typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
                .unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap()
    }
    #[test]
    fn cff_v2_admission_rejects_face_and_directory_before_decoding() {
        let e = admit_sfnt_cff1_v2(Arc::from([]), 1, &limits()).unwrap_err();
        assert_eq!(e.kind, Cff1FailureKindV2::Sfnt(Cff1Error::InvalidFaceIndex));
        let e = admit_sfnt_cff1_v2(Arc::from([]), 0, &limits()).unwrap_err();
        assert_eq!(e.kind, Cff1FailureKindV2::Sfnt(Cff1Error::InvalidSfnt));
        assert_eq!(e.context.phase, FontFailurePhase::SfntDirectory);
        let mut base = typaxis_core::ResourceLimits::default();
        base.max_font_bytes = 1;
        let small = M4EffectiveResourceLimits::new(
            typaxis_core::ValidatedResourceLimits::new(base).unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap();
        let e = admit_sfnt_cff1_v2(Arc::from([0u8; 2]), 0, &small).unwrap_err();
        assert_eq!(
            (e.kind, e.context.limit, e.context.observed),
            (Cff1FailureKindV2::SourceByteLimit, Some(1), Some(2))
        );
    }

    #[test]
    #[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
    fn cff_v2_admission_operator_failures_retain_source_positions_and_typed_class() {
        let original = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
        assert_eq!(
            sha256(&original)
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect::<String>(),
            "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
        );
        let defaults = limits();
        let base = admit_sfnt_cff1_v2(original.clone().into(), 0, &defaults).unwrap();
        let records = &base.table_records;
        let cff_base = records.iter().find(|r| r.tag == *b"CFF ").unwrap().offset;
        let head = records.iter().find(|r| r.tag == *b"head").unwrap().offset;
        let span = base.program.program.charstrings[0];
        let fd = base.program.program.fd_by_gid[0];
        for (token, reason, class) in [
            (
                [12, 10],
                FontFailureReason::UnsupportedCffOperator,
                "unsupported",
            ),
            (
                [12, 1],
                FontFailureReason::ReservedCffOperator,
                "malformed-or-invalid-input",
            ),
            (
                [0, 14],
                FontFailureReason::ReservedCffOperator,
                "malformed-or-invalid-input",
            ),
        ] {
            // Negative-only copies preserve real SFNT structure and directory
            // checksums. They are never evidence of original-font acceptance.
            let mut source = original.clone();
            let offset = cff_base + span.start;
            source[offset..offset + 2].copy_from_slice(&token);
            source[head + 8..head + 12].fill(0);
            for (i, record) in records.iter().enumerate() {
                let checksum = sfnt_checksum(&source[record.offset..record.offset + record.length]);
                source[16 + i * 16..20 + i * 16].copy_from_slice(&checksum.to_be_bytes());
            }
            let adjustment = SFNT_CHECKSUM_MAGIC.wrapping_sub(sfnt_checksum(&source));
            source[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
            let admitted = admit_sfnt_cff1_v2(source.into(), 0, &defaults).unwrap();
            assert_ne!(admitted.source_sha256(), base.source_sha256());
            let closure = Cff1SubsetSessionV2::close_instance_selection(
                &admitted,
                FontFaceId::new(1),
                FontInstanceId::new(1),
                &BTreeSet::new(),
                10,
            )
            .unwrap();
            let mut session = Cff1SubsetSessionV2::from_admission(&admitted);
            let CffSelectionFailureV2::Glyph(error) =
                session.prepare_closure(&admitted, &closure).unwrap_err()
            else {
                panic!("operator error lost its glyph context");
            };
            let context = error.font_context.unwrap();
            assert_eq!(error.kind, Cff1Error::InvalidCharstring);
            assert_eq!(context.phase, FontFailurePhase::Charstring);
            assert_eq!(context.reason, reason);
            assert_eq!(context.reason.class(), class);
            assert_eq!(
                (context.file_offset, context.table_offset),
                (Some(offset as u64), Some(span.start as u64))
            );
            assert_eq!((context.gid, context.fd), (Some(0), Some(u16::from(fd))));
            assert_eq!(context.table_tag, Some(*b"CFF "));
            assert_eq!(context.embedding, FontEmbeddingStatus::Allowed(0));
            assert_eq!((context.limit, context.observed), (None, None));
            assert!(context.position_is_exact && !context.position_is_end);
            let operator = if token[0] == 12 {
                0x0c00 | u16::from(token[1])
            } else {
                u16::from(token[0])
            };
            assert_eq!(context.operator, Some(operator));
            let note = Cff1Failure {
                kind: error.kind,
                context,
            }
            .context_note();
            assert!(note.contains(&format!("reason={}; class={class}", reason.as_str())));
            assert_eq!(session.cached_glyph_count(), 0);
            assert_eq!(
                (session.operations_used(), session.outline_segments_used()),
                (1, 0)
            );
        }
    }
    #[test]
    #[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
    fn cff_v2_admission_original_harano_whole_sfnt() {
        let source = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
        let hex = |b: &[u8]| b.iter().map(|v| format!("{v:02x}")).collect::<String>();
        assert_eq!(
            hex(&sha256(&source)),
            "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
        );
        let admitted = admit_sfnt_cff1_v2(Arc::from(source), 0, &limits()).unwrap();
        assert_eq!(admitted.glyph_count(), 23060);
        assert_eq!(admitted.units_per_em(), 1000);
        assert_eq!(admitted.postscript_name(), "HaranoAjiMincho-Regular");
        assert_eq!(
            admitted.embedding_permission(),
            Cff1EmbeddingPermission::Installable
        );
        assert_eq!(admitted.cmap().base_mapping_count(), 15815);
        assert_eq!(admitted.program().font_dict_count(), 18);
        assert_eq!(admitted.horizontal_metric(151).unwrap().0, 1000);
        assert_eq!(admitted.horizontal_metric(233).unwrap().0, 500);
        assert_eq!(admitted.horizontal_metric(23060), None);
        let SourceBytesV2::Slice(source) = &admitted.source else {
            panic!("slice owner changed")
        };
        let repeated = admit_sfnt_cff1_v2(source.clone(), 0, &limits()).unwrap();
        assert_eq!(admitted.fingerprint(), repeated.fingerprint());
        let old = admit_sfnt_cff1_detailed(admitted.source(), 0, &limits()).unwrap_err();
        assert_eq!(old.context.table_tag, Some(*b"VORG"));
        assert_eq!(old.context.reason, FontFailureReason::UnsupportedTable);
        let mut lower = *limits().extension().get();
        lower.max_font_glyphs = 23059;
        let lower = M4EffectiveResourceLimits::new(limits().base().clone(), lower).unwrap();
        let e = admit_sfnt_cff1_v2(source.clone(), 0, &lower).unwrap_err();
        assert_eq!(
            (
                e.context.table_tag,
                e.context.table_offset,
                e.context.limit,
                e.context.observed
            ),
            (Some(*b"maxp"), Some(4), Some(23059), Some(23060))
        );
        // Negative-only copies have deliberately altered table bytes and new
        // checksums. The positive gate above always uses the original bytes.
        for (tag, offset, value, expected) in [
            (
                *b"VORG",
                1,
                2,
                Cff1FailureKindV2::Table(CffTableFailureKindV2::UnsupportedVersion),
            ),
            (
                *b"vhea",
                25,
                1,
                Cff1FailureKindV2::Table(CffTableFailureKindV2::InvalidReservedField),
            ),
            (
                *b"OS/2",
                9,
                2,
                Cff1FailureKindV2::Sfnt(Cff1Error::RestrictedEmbedding),
            ),
        ] {
            let records = preflight_sfnt_with_vertical(
                admitted.source(),
                *limits().extension().get(),
                &mut FontFailureContext::new(0),
                true,
            )
            .unwrap();
            let mut changed = admitted.source().to_vec();
            let head = records.iter().find(|r| r.tag == *b"head").unwrap().offset;
            changed[head + 8..head + 12].fill(0);
            let target = records.iter().find(|r| r.tag == tag).unwrap().offset;
            changed[target + offset] = value;
            for (index, record) in records.iter().enumerate() {
                let checksum =
                    sfnt_checksum(&changed[record.offset..record.offset + record.length]);
                changed[12 + index * 16 + 4..12 + index * 16 + 8]
                    .copy_from_slice(&checksum.to_be_bytes());
            }
            let adjustment = SFNT_CHECKSUM_MAGIC.wrapping_sub(sfnt_checksum(&changed));
            changed[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
            let e = admit_sfnt_cff1_v2(Arc::from(changed), 0, &limits()).unwrap_err();
            assert_eq!(e.kind, expected);
            assert_eq!(e.context.table_tag, Some(tag));
            assert!(e.context.file_offset.is_some());
            if tag == *b"OS/2" {
                assert_eq!(e.context.embedding, FontEmbeddingStatus::Denied(2));
                assert_eq!(e.context.phase, FontFailurePhase::EmbeddingPermission);
            }
        }
    }
}
