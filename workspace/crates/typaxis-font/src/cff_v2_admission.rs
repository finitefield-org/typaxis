//! CID CFF /2 sfnt admission. The resource/profile registry and PDF publication
//! remain separate gates; a caller cannot construct this validated owner.
use super::*;

#[derive(Debug)]
pub struct Cff1AdmissionV2 {
    table_records: Vec<TableRecord>,
    family: String,
    subfamily: String,
    effective_limits: M4EffectiveResourceLimits,
    source: Arc<[u8]>,
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
    pub(super) fn table_bytes(&self, tag: &[u8; 4]) -> Result<&[u8], Cff1Error> {
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
    let mut context = FontFailureContext::new(face_index);
    if face_index != 0 {
        return Err(Cff1FailureV2 {
            kind: Cff1FailureKindV2::Sfnt(Cff1Error::InvalidFaceIndex),
            context,
        });
    }
    if source.len() as u64 > limits.base().get().max_font_bytes {
        context.phase = FontFailurePhase::SfntDirectory;
        context.reason = FontFailureReason::BudgetExceeded;
        context.limit = Some(limits.base().get().max_font_bytes);
        context.observed = Some(source.len() as u64);
        return Err(Cff1FailureV2 {
            kind: Cff1FailureKindV2::SourceByteLimit,
            context,
        });
    }
    let records =
        preflight_sfnt_with_vertical(&source, *limits.extension().get(), &mut context, true)
            .map_err(|kind| Cff1FailureV2 {
                kind: Cff1FailureKindV2::Sfnt(kind),
                context,
            })?;
    let tables = table_map(&source, &records).map_err(|kind| Cff1FailureV2 {
        kind: Cff1FailureKindV2::Sfnt(kind),
        context,
    })?;
    macro_rules! table {
        ($tag:expr,$operation:expr) => {{
            context.table(*$tag, &records, FontFailurePhase::TableDecode);
            $operation.map_err(|kind| {
                if kind == Cff1Error::GlyphLimit {
                    context.field(4);
                    context.reason = FontFailureReason::BudgetExceeded;
                    context.limit = Some(u64::from(limits.extension().get().max_font_glyphs));
                    context.observed = read_u16(tables[b"maxp"].bytes, 4, Cff1Error::InvalidMaxp)
                        .ok()
                        .map(u64::from);
                }
                Cff1FailureV2 {
                    kind: Cff1FailureKindV2::Sfnt(kind),
                    context,
                }
            })?
        }};
    }
    // Required-table presence and ranges are already established by preflight.
    let head = table!(b"head", parse_head(tables[b"head"].bytes));
    let glyph_count = table!(b"maxp", parse_maxp(tables[b"maxp"].bytes, limits));
    let hhea = table!(b"hhea", parse_hhea(tables[b"hhea"].bytes));
    let (advances, left_side_bearings) = table!(
        b"hmtx",
        parse_hmtx(tables[b"hmtx"].bytes, glyph_count, hhea.number_of_h_metrics)
    );
    let names = table!(b"name", parse_name(tables[b"name"].bytes));
    let os2 = table!(b"OS/2", parse_os2(tables[b"OS/2"].bytes));
    table!(b"post", parse_post(tables[b"post"].bytes));
    context.table(*b"OS/2", &records, FontFailurePhase::EmbeddingPermission);
    context.permission(os2.bytes);
    context.field(8);
    context.reason = FontFailureReason::RestrictedEmbedding;
    let permission = embedding_permission(os2.fs_type).map_err(|kind| Cff1FailureV2 {
        kind: Cff1FailureKindV2::Sfnt(kind),
        context,
    })?;
    validate_cff_vertical_metrics_v2(
        glyph_count,
        tables.get(b"VORG").map(|t| t.bytes),
        tables.get(b"vhea").map(|t| t.bytes),
        tables.get(b"vmtx").map(|t| t.bytes),
    )
    .map_err(|e| table_failure(e, &records, context))?;
    let cmap = validate_cff_cmap_v2(tables[b"cmap"].bytes, glyph_count).map_err(|e| match e {
        CffCmapFailureV2::Variation(e) => table_failure(e, &records, context),
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
    })?;
    validate_optional_tables(&source, &tables, glyph_count, &records, &mut context).map_err(
        |kind| Cff1FailureV2 {
            kind: Cff1FailureKindV2::Sfnt(kind),
            context,
        },
    )?;
    context.table(*b"CFF ", &records, FontFailurePhase::CffIndex);
    let cff = tables[b"CFF "].bytes;
    let program = inspect_cff1_program_v2_with_metadata(
        Arc::from(cff),
        glyph_count,
        head.units_per_em,
        limits.extension().get().max_cff_subroutines,
        Some((&names.postscript_name, head.bbox)),
    )
    .map_err(|e| {
        context.field(e.table_offset);
        context.fd = e.fd;
        context.operator = e.operator;
        context.limit = e.limit;
        context.observed = e.observed;
        context.reason = if e.limit.is_some() {
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
    })?;
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
    let units_per_em = head.units_per_em;
    Ok(Cff1AdmissionV2 {
        table_records: records,
        family: names.family,
        subfamily: names.subfamily,
        effective_limits: limits.clone(),
        source,
        source_sha256,
        fingerprint: sha256(identity.as_bytes()),
        limits_fingerprint: limits.fingerprint(),
        glyph_count,
        units_per_em,
        postscript_name: names.postscript_name,
        embedding_permission: permission,
        advances,
        left_side_bearings,
        cmap,
        program,
    })
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
        let repeated = admit_sfnt_cff1_v2(admitted.source.clone(), 0, &limits()).unwrap();
        assert_eq!(admitted.fingerprint(), repeated.fingerprint());
        let old = admit_sfnt_cff1_detailed(admitted.source(), 0, &limits()).unwrap_err();
        assert_eq!(old.context.table_tag, Some(*b"VORG"));
        assert_eq!(old.context.reason, FontFailureReason::UnsupportedTable);
        let mut lower = *limits().extension().get();
        lower.max_font_glyphs = 23059;
        let lower = M4EffectiveResourceLimits::new(limits().base().clone(), lower).unwrap();
        let e = admit_sfnt_cff1_v2(admitted.source.clone(), 0, &lower).unwrap_err();
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
