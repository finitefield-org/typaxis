use std::collections::{BTreeMap, BTreeSet};

use read_fonts::{
    collections::IntSet,
    tables::{gpos::PositionLookup, layout::Intersect},
    types::GlyphId,
    FontData, FontRead, FontRef, TableProvider,
};
use typaxis_core::{
    push_jcs_string, sha256, FontFaceId, FontInstanceId, M4EffectiveResourceLimits,
    M4ResourceLimits,
};

use crate::{OriginalGlyphId, SubsetGlyphId};

pub const CFF1_RESOURCE_PROFILE_ID: &str = "typaxis.resource-profile/sfnt-cff1/1";
pub const CFF1_ADMISSION_ID: &str = "typaxis.sfnt-cff1-admission/1";
pub const CFF1_CHARSTRING_EVALUATOR_ID: &str = "typaxis.cff1-charstring-evaluator/1";
pub const CFF1_GLYPH_CLOSURE_ID: &str = "typaxis.cff1-glyph-closure/1";
pub const CFF1_SUBSET_ID: &str = "typaxis.cff1-subset/1";
pub const CFF1_EMBEDDING_PERMISSION_ID: &str = "typaxis.cff1-embedding-permission/1";
pub const CFF1_PDF_PLAN_ID: &str = "typaxis.cff1-pdf-plan/1";

const REQUIRED_TABLES: [[u8; 4]; 9] = [
    *b"CFF ", *b"OS/2", *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"name", *b"post",
];
const OPTIONAL_TABLES: [[u8; 4]; 7] = [
    *b"BASE", *b"GDEF", *b"GPOS", *b"GSUB", *b"JSTF", *b"MATH", *b"kern",
];
const OUTPUT_TABLES: [[u8; 4]; 9] = REQUIRED_TABLES;
const SFNT_CHECKSUM_MAGIC: u32 = 0xB1B0_AFBA;
const TYPE2_OPERAND_STACK_LIMIT: usize = 48;
const TYPE2_CALL_DEPTH_LIMIT: usize = 10;
const TYPE2_STEM_LIMIT: u32 = 96;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cff1EmbeddingPermission {
    Installable,
    PreviewAndPrint,
    Editable,
}

impl Cff1EmbeddingPermission {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Installable => "installable",
            Self::PreviewAndPrint => "preview-and-print",
            Self::Editable => "editable",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cff1Error {
    InvalidFaceIndex,
    InvalidSfnt,
    UnsupportedTable,
    TableLimit,
    GlyphLimit,
    InvalidHead,
    InvalidMaxp,
    InvalidHhea,
    InvalidHmtx,
    InvalidCmap,
    InvalidName,
    InvalidOs2,
    InvalidPost,
    InvalidOptionalTable,
    InvalidCff,
    SubroutineLimit,
    RestrictedEmbedding,
    InvalidCharstring,
    CharstringOperationLimit,
    OutlineSegmentLimit,
    InvalidSelectedGlyph,
    SelectedGlyphLimit,
    InvalidGlyphClosure,
    InvalidSubset,
    SubsetByteLimit,
    ReceiptMismatch,
}

impl std::fmt::Display for Cff1Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = match self {
            Self::TableLimit => "R7130",
            Self::GlyphLimit => "R7131",
            Self::SubroutineLimit => "R7132",
            Self::CharstringOperationLimit => "R7133",
            Self::OutlineSegmentLimit => "R7134",
            Self::SubsetByteLimit => "R7135",
            Self::InvalidGlyphClosure | Self::ReceiptMismatch => "I9190",
            _ => "R7100",
        };
        write!(formatter, "{code}: {self:?}")
    }
}

impl std::error::Error for Cff1Error {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cff1PdfMetrics {
    pub ascent_1000: i32,
    pub descent_1000: i32,
    pub cap_height_1000: i32,
    pub stem_v_1000: u32,
    pub italic_angle_fixed_16_16: i32,
    pub flags: u32,
    pub bbox_1000: [i32; 4],
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CffProgram {
    charstrings: Vec<Vec<u8>>,
    global_subrs: Vec<Vec<u8>>,
    local_subrs: Vec<Vec<u8>>,
    default_width_x: i32,
    nominal_width_x: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cff1Admission {
    source_sha256: [u8; 32],
    source_byte_length: u64,
    table_count: u32,
    glyph_count: u32,
    subroutine_count: u32,
    family: String,
    subfamily: String,
    postscript_name: String,
    fs_type: u16,
    embedding_permission: Cff1EmbeddingPermission,
    units_per_em: u16,
    head: Vec<u8>,
    hhea: Vec<u8>,
    os2: Vec<u8>,
    post: Vec<u8>,
    advances: Vec<u16>,
    left_side_bearings: Vec<i16>,
    cmap: BTreeMap<u32, u16>,
    program: CffProgram,
    limits: M4ResourceLimits,
    limits_fingerprint: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl Cff1Admission {
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub const fn source_byte_length(&self) -> u64 {
        self.source_byte_length
    }
    pub const fn table_count(&self) -> u32 {
        self.table_count
    }
    pub const fn glyph_count(&self) -> u32 {
        self.glyph_count
    }
    pub const fn subroutine_count(&self) -> u32 {
        self.subroutine_count
    }
    pub fn family(&self) -> &str {
        &self.family
    }
    pub fn subfamily(&self) -> &str {
        &self.subfamily
    }
    pub fn postscript_name(&self) -> &str {
        &self.postscript_name
    }
    pub const fn fs_type(&self) -> u16 {
        self.fs_type
    }
    pub const fn embedding_permission(&self) -> Cff1EmbeddingPermission {
        self.embedding_permission
    }
    pub const fn units_per_em(&self) -> u16 {
        self.units_per_em
    }
    pub const fn limits(&self) -> M4ResourceLimits {
        self.limits
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cff1GlyphClosure {
    font_face_id: FontFaceId,
    font_instance_id: FontInstanceId,
    source_sha256: [u8; 32],
    source_gids: Vec<OriginalGlyphId>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl Cff1GlyphClosure {
    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub const fn font_instance_id(&self) -> FontInstanceId {
        self.font_instance_id
    }
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub fn source_gids(&self) -> &[OriginalGlyphId] {
        &self.source_gids
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cff1Subset {
    bytes: Vec<u8>,
    sha256: [u8; 32],
    postscript_name: String,
    original_to_subset: BTreeMap<OriginalGlyphId, SubsetGlyphId>,
    original_widths: BTreeMap<OriginalGlyphId, u16>,
    metrics: Cff1PdfMetrics,
    closure: Cff1GlyphClosure,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl Cff1Subset {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub const fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
    pub fn postscript_name(&self) -> &str {
        &self.postscript_name
    }
    pub const fn original_to_subset(&self) -> &BTreeMap<OriginalGlyphId, SubsetGlyphId> {
        &self.original_to_subset
    }
    pub const fn original_widths(&self) -> &BTreeMap<OriginalGlyphId, u16> {
        &self.original_widths
    }
    pub const fn metrics(&self) -> &Cff1PdfMetrics {
        &self.metrics
    }
    pub const fn closure(&self) -> &Cff1GlyphClosure {
        &self.closure
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum OutlineSegment {
    Move(i32, i32),
    Line(i32, i32),
    Cubic(i32, i32, i32, i32, i32, i32),
    Close,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EvaluatedGlyph {
    segments: Vec<OutlineSegment>,
    bbox: Option<[i32; 4]>,
}

/// Session owner for aggregate Type2 work. Evaluated face/GID pairs are
/// cached, so multiple font instances cannot reset limits or charge the same
/// source outline more than once.
#[derive(Debug)]
pub struct Cff1SubsetSession {
    limits: M4ResourceLimits,
    limits_fingerprint: [u8; 32],
    operations_used: u64,
    outline_segments_used: u64,
    evaluated: BTreeMap<(FontFaceId, [u8; 32], u16), EvaluatedGlyph>,
}

impl Cff1SubsetSession {
    pub fn new(limits: &M4EffectiveResourceLimits) -> Self {
        Self {
            limits: *limits.extension().get(),
            limits_fingerprint: limits.fingerprint(),
            operations_used: 0,
            outline_segments_used: 0,
            evaluated: BTreeMap::new(),
        }
    }

    /// Reconstruct the aggregate evaluator budget from a sealed admission
    /// receipt. This lets downstream finalizers share one budget across all
    /// CFF1 instances without accepting caller-supplied M4 limits.
    pub fn from_admission(admission: &Cff1Admission) -> Self {
        Self {
            limits: admission.limits,
            limits_fingerprint: admission.limits_fingerprint,
            operations_used: 0,
            outline_segments_used: 0,
            evaluated: BTreeMap::new(),
        }
    }

    pub const fn operations_used(&self) -> u64 {
        self.operations_used
    }
    pub const fn outline_segments_used(&self) -> u64 {
        self.outline_segments_used
    }

    /// Validate and seal one instance's exact selected source-GID set without
    /// spending Type 2 work. Aggregate owners use this before forming the
    /// cross-instance face union, so an invalid/oversized instance fails
    /// before any outline evaluation.
    pub fn close_instance_selection(
        admission: &Cff1Admission,
        font_face_id: FontFaceId,
        font_instance_id: FontInstanceId,
        selected: &BTreeSet<OriginalGlyphId>,
        max_cids_per_font: u16,
    ) -> Result<Cff1GlyphClosure, Cff1Error> {
        build_glyph_closure(
            admission,
            font_face_id,
            font_instance_id,
            selected,
            max_cids_per_font,
        )
    }

    /// Evaluate the union selected by every instance of one face before any
    /// per-instance subset is written. Multi-face owners call this in
    /// ascending `FontFaceId` order; `BTreeSet` fixes source-GID order and the
    /// cache makes each face/GID pair a one-time aggregate-budget charge.
    pub fn prepare_face(
        &mut self,
        admission: &Cff1Admission,
        font_face_id: FontFaceId,
        selected: &BTreeSet<OriginalGlyphId>,
    ) -> Result<(), Cff1Error> {
        self.require_admission(admission)?;
        self.evaluate_face_gid(admission, font_face_id, OriginalGlyphId::new(0))?;
        for gid in selected {
            self.evaluate_face_gid(admission, font_face_id, *gid)?;
        }
        Ok(())
    }

    pub fn subset(
        &mut self,
        admission: &Cff1Admission,
        font_face_id: FontFaceId,
        font_instance_id: FontInstanceId,
        selected: &BTreeSet<OriginalGlyphId>,
        max_cids_per_font: u16,
    ) -> Result<Cff1Subset, Cff1Error> {
        self.require_admission(admission)?;
        let closure = Self::close_instance_selection(
            admission,
            font_face_id,
            font_instance_id,
            selected,
            max_cids_per_font,
        )?;
        for gid in closure.source_gids() {
            self.evaluate_face_gid(admission, font_face_id, *gid)?;
        }
        write_subset(
            admission,
            closure,
            &self.evaluated,
            self.limits.max_font_subset_bytes,
        )
    }

    fn require_admission(&self, admission: &Cff1Admission) -> Result<(), Cff1Error> {
        if self.limits_fingerprint != admission.limits_fingerprint
            || self.limits != admission.limits
        {
            Err(Cff1Error::ReceiptMismatch)
        } else {
            Ok(())
        }
    }

    fn evaluate_face_gid(
        &mut self,
        admission: &Cff1Admission,
        font_face_id: FontFaceId,
        gid: OriginalGlyphId,
    ) -> Result<(), Cff1Error> {
        if u32::from(gid.get()) >= admission.glyph_count {
            return Err(Cff1Error::InvalidSelectedGlyph);
        }
        let key = (font_face_id, admission.source_sha256, gid.get());
        if !self.evaluated.contains_key(&key) {
            let outline = evaluate_glyph(admission, gid.get(), self)?;
            self.evaluated.insert(key, outline);
        }
        Ok(())
    }

    fn charge_operation(&mut self) -> Result<(), Cff1Error> {
        let next = self
            .operations_used
            .checked_add(1)
            .ok_or(Cff1Error::CharstringOperationLimit)?;
        if next > self.limits.max_cff_charstring_operations {
            return Err(Cff1Error::CharstringOperationLimit);
        }
        self.operations_used = next;
        Ok(())
    }

    fn charge_segment(&mut self) -> Result<(), Cff1Error> {
        let next = self
            .outline_segments_used
            .checked_add(1)
            .ok_or(Cff1Error::OutlineSegmentLimit)?;
        if next > self.limits.max_cff_outline_segments {
            return Err(Cff1Error::OutlineSegmentLimit);
        }
        self.outline_segments_used = next;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct TableRef<'a> {
    bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TableRecord {
    tag: [u8; 4],
    offset: usize,
    length: usize,
    padded_end: usize,
}

pub fn admit_sfnt_cff1(
    source: &[u8],
    face_index: u32,
    limits: &M4EffectiveResourceLimits,
) -> Result<Cff1Admission, Cff1Error> {
    if face_index != 0 {
        return Err(Cff1Error::InvalidFaceIndex);
    }
    let table_records = preflight_sfnt(source, *limits.extension().get())?;
    let tables = table_map(source, &table_records)?;
    typed_read_fonts_check(source)?;

    let head = parse_head(required_table(&tables, b"head")?)?;
    let maxp = parse_maxp(required_table(&tables, b"maxp")?, limits)?;
    let hhea = parse_hhea(required_table(&tables, b"hhea")?)?;
    let (advances, left_side_bearings) = parse_hmtx(
        required_table(&tables, b"hmtx")?,
        maxp,
        hhea.number_of_h_metrics,
    )?;
    let cmap = parse_cmap(required_table(&tables, b"cmap")?, maxp)?;
    let names = parse_name(required_table(&tables, b"name")?)?;
    let os2 = parse_os2(required_table(&tables, b"OS/2")?)?;
    let post = parse_post(required_table(&tables, b"post")?)?;
    validate_optional_tables(source, &tables, maxp)?;
    let program = parse_cff(
        required_table(&tables, b"CFF ")?,
        maxp,
        &names.postscript_name,
        head.bbox,
        limits,
    )?;
    let subroutine_count = u32::try_from(
        program
            .global_subrs
            .len()
            .checked_add(program.local_subrs.len())
            .ok_or(Cff1Error::SubroutineLimit)?,
    )
    .map_err(|_| Cff1Error::SubroutineLimit)?;
    let embedding_permission = embedding_permission(os2.fs_type)?;
    let source_sha256 = sha256(source);
    let source_byte_length = u64::try_from(source.len()).map_err(|_| Cff1Error::InvalidSfnt)?;
    let table_count = u32::try_from(table_records.len()).map_err(|_| Cff1Error::TableLimit)?;
    let canonical_jcs = encode_admission(&AdmissionJcsFacts {
        source_sha256,
        source_byte_length,
        table_count,
        glyph_count: u32::from(maxp),
        subroutine_count,
        names: &names,
        fs_type: os2.fs_type,
        permission: embedding_permission,
        limits_fingerprint: limits.fingerprint(),
    });
    Ok(Cff1Admission {
        source_sha256,
        source_byte_length,
        table_count,
        glyph_count: u32::from(maxp),
        subroutine_count,
        family: names.family,
        subfamily: names.subfamily,
        postscript_name: names.postscript_name,
        fs_type: os2.fs_type,
        embedding_permission,
        units_per_em: head.units_per_em,
        head: head.bytes.to_vec(),
        hhea: hhea.bytes.to_vec(),
        os2: os2.bytes.to_vec(),
        post: post.bytes.to_vec(),
        advances,
        left_side_bearings,
        cmap,
        program,
        limits: *limits.extension().get(),
        limits_fingerprint: limits.fingerprint(),
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

struct AdmissionJcsFacts<'a> {
    source_sha256: [u8; 32],
    source_byte_length: u64,
    table_count: u32,
    glyph_count: u32,
    subroutine_count: u32,
    names: &'a NameFacts,
    fs_type: u16,
    permission: Cff1EmbeddingPermission,
    limits_fingerprint: [u8; 32],
}

fn encode_admission(facts: &AdmissionJcsFacts<'_>) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, CFF1_ADMISSION_ID);
    output.push_str(",\"embedding_permission\":");
    push_jcs_string(&mut output, facts.permission.as_str());
    output.push_str(",\"embedding_permission_algorithm\":");
    push_jcs_string(&mut output, CFF1_EMBEDDING_PERMISSION_ID);
    output.push_str(",\"family\":");
    push_jcs_string(&mut output, &facts.names.family);
    output.push_str(",\"fs_type\":");
    output.push_str(&facts.fs_type.to_string());
    output.push_str(",\"glyph_count\":");
    output.push_str(&facts.glyph_count.to_string());
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, facts.limits_fingerprint);
    output.push_str(",\"postscript_name\":");
    push_jcs_string(&mut output, &facts.names.postscript_name);
    output.push_str(",\"resource_profile\":");
    push_jcs_string(&mut output, CFF1_RESOURCE_PROFILE_ID);
    output.push_str(",\"source_byte_length\":");
    output.push_str(&facts.source_byte_length.to_string());
    output.push_str(",\"source_sha256\":");
    push_hash(&mut output, facts.source_sha256);
    output.push_str(",\"subfamily\":");
    push_jcs_string(&mut output, &facts.names.subfamily);
    output.push_str(",\"subroutine_count\":");
    output.push_str(&facts.subroutine_count.to_string());
    output.push_str(",\"table_count\":");
    output.push_str(&facts.table_count.to_string());
    output.push_str(",\"units_per_em\":1000}");
    output
}

fn push_hash(output: &mut String, hash: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in hash {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

fn preflight_sfnt(source: &[u8], limits: M4ResourceLimits) -> Result<Vec<TableRecord>, Cff1Error> {
    if source.get(..4) != Some(b"OTTO") {
        return Err(Cff1Error::InvalidSfnt);
    }
    let count = usize::from(read_u16(source, 4, Cff1Error::InvalidSfnt)?);
    if count == 0
        || u32::try_from(count).map_err(|_| Cff1Error::TableLimit)? > limits.max_font_tables
    {
        return Err(Cff1Error::TableLimit);
    }
    let largest_power = 1usize << (usize::BITS - 1 - count.leading_zeros());
    let expected_search_range = largest_power
        .checked_mul(16)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or(Cff1Error::InvalidSfnt)?;
    let expected_entry_selector =
        u16::try_from(largest_power.trailing_zeros()).map_err(|_| Cff1Error::InvalidSfnt)?;
    let expected_range_shift = count
        .checked_mul(16)
        .and_then(|value| value.checked_sub(usize::from(expected_search_range)))
        .and_then(|value| u16::try_from(value).ok())
        .ok_or(Cff1Error::InvalidSfnt)?;
    if read_u16(source, 6, Cff1Error::InvalidSfnt)? != expected_search_range
        || read_u16(source, 8, Cff1Error::InvalidSfnt)? != expected_entry_selector
        || read_u16(source, 10, Cff1Error::InvalidSfnt)? != expected_range_shift
    {
        return Err(Cff1Error::InvalidSfnt);
    }
    let directory_end = 12usize
        .checked_add(count.checked_mul(16).ok_or(Cff1Error::InvalidSfnt)?)
        .ok_or(Cff1Error::InvalidSfnt)?;
    if directory_end > source.len() {
        return Err(Cff1Error::InvalidSfnt);
    }
    let mut records = Vec::new();
    records
        .try_reserve_exact(count)
        .map_err(|_| Cff1Error::TableLimit)?;
    let mut previous_tag = None;
    for index in 0..count {
        let record = 12 + index * 16;
        let tag: [u8; 4] = source[record..record + 4]
            .try_into()
            .map_err(|_| Cff1Error::InvalidSfnt)?;
        if previous_tag.is_some_and(|previous| previous >= tag) {
            return Err(Cff1Error::InvalidSfnt);
        }
        previous_tag = Some(tag);
        if !REQUIRED_TABLES.contains(&tag) && !OPTIONAL_TABLES.contains(&tag) {
            return Err(Cff1Error::UnsupportedTable);
        }
        let expected_checksum = read_u32(source, record + 4, Cff1Error::InvalidSfnt)?;
        let offset = usize::try_from(read_u32(source, record + 8, Cff1Error::InvalidSfnt)?)
            .map_err(|_| Cff1Error::InvalidSfnt)?;
        let length = usize::try_from(read_u32(source, record + 12, Cff1Error::InvalidSfnt)?)
            .map_err(|_| Cff1Error::InvalidSfnt)?;
        let end = offset.checked_add(length).ok_or(Cff1Error::InvalidSfnt)?;
        let padded_end = end
            .checked_add(3)
            .map(|value| value & !3)
            .ok_or(Cff1Error::InvalidSfnt)?;
        if length == 0
            || offset < directory_end
            || offset % 4 != 0
            || end > source.len()
            || padded_end > source.len()
        {
            return Err(Cff1Error::InvalidSfnt);
        }
        let mut checksum_bytes = Vec::new();
        checksum_bytes
            .try_reserve_exact(end - offset)
            .map_err(|_| Cff1Error::InvalidSfnt)?;
        checksum_bytes.extend_from_slice(&source[offset..end]);
        if tag == *b"head" {
            if checksum_bytes.len() < 12 {
                return Err(Cff1Error::InvalidHead);
            }
            checksum_bytes[8..12].fill(0);
        }
        if sfnt_checksum(&checksum_bytes) != expected_checksum {
            return Err(Cff1Error::InvalidSfnt);
        }
        records.push(TableRecord {
            tag,
            offset,
            length,
            padded_end,
        });
    }
    for required in REQUIRED_TABLES {
        if !records.iter().any(|record| record.tag == required) {
            return Err(Cff1Error::InvalidSfnt);
        }
    }
    let mut by_offset = records.clone();
    by_offset.sort_by_key(|record| record.offset);
    let mut cursor = directory_end;
    for record in by_offset {
        if record.offset < cursor || source[cursor..record.offset].iter().any(|byte| *byte != 0) {
            return Err(Cff1Error::InvalidSfnt);
        }
        let end = record
            .offset
            .checked_add(record.length)
            .ok_or(Cff1Error::InvalidSfnt)?;
        if source[end..record.padded_end].iter().any(|byte| *byte != 0) {
            return Err(Cff1Error::InvalidSfnt);
        }
        cursor = record.padded_end;
    }
    if cursor != source.len() || sfnt_checksum(source) != SFNT_CHECKSUM_MAGIC {
        return Err(Cff1Error::InvalidSfnt);
    }
    Ok(records)
}

fn table_map<'a>(
    source: &'a [u8],
    records: &[TableRecord],
) -> Result<BTreeMap<[u8; 4], TableRef<'a>>, Cff1Error> {
    let mut output = BTreeMap::new();
    for record in records {
        let end = record
            .offset
            .checked_add(record.length)
            .ok_or(Cff1Error::InvalidSfnt)?;
        if output
            .insert(
                record.tag,
                TableRef {
                    bytes: source
                        .get(record.offset..end)
                        .ok_or(Cff1Error::InvalidSfnt)?,
                },
            )
            .is_some()
        {
            return Err(Cff1Error::InvalidSfnt);
        }
    }
    Ok(output)
}

fn required_table<'a>(
    tables: &'a BTreeMap<[u8; 4], TableRef<'a>>,
    tag: &[u8; 4],
) -> Result<&'a [u8], Cff1Error> {
    tables
        .get(tag)
        .map(|table| table.bytes)
        .ok_or(Cff1Error::InvalidSfnt)
}

fn typed_read_fonts_check(source: &[u8]) -> Result<(), Cff1Error> {
    let font = FontRef::new(source).map_err(|_| Cff1Error::InvalidSfnt)?;
    font.head().map_err(|_| Cff1Error::InvalidHead)?;
    font.maxp().map_err(|_| Cff1Error::InvalidMaxp)?;
    font.hhea().map_err(|_| Cff1Error::InvalidHhea)?;
    font.hmtx().map_err(|_| Cff1Error::InvalidHmtx)?;
    font.cmap().map_err(|_| Cff1Error::InvalidCmap)?;
    font.name().map_err(|_| Cff1Error::InvalidName)?;
    font.os2().map_err(|_| Cff1Error::InvalidOs2)?;
    font.post().map_err(|_| Cff1Error::InvalidPost)?;
    let cff = font.cff().map_err(|_| Cff1Error::InvalidCff)?;
    if cff.names().count() != 1 || cff.top_dicts().count() != 1 {
        return Err(Cff1Error::InvalidCff);
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct HeadFacts<'a> {
    bytes: &'a [u8],
    units_per_em: u16,
    bbox: [i16; 4],
}

fn parse_head(bytes: &[u8]) -> Result<HeadFacts<'_>, Cff1Error> {
    if bytes.len() != 54
        || read_u32(bytes, 0, Cff1Error::InvalidHead)? != 0x0001_0000
        || read_u32(bytes, 12, Cff1Error::InvalidHead)? != 0x5F0F_3CF5
        || read_i16(bytes, 50, Cff1Error::InvalidHead)? != 0
        || read_i16(bytes, 52, Cff1Error::InvalidHead)? != 0
    {
        return Err(Cff1Error::InvalidHead);
    }
    let units_per_em = read_u16(bytes, 18, Cff1Error::InvalidHead)?;
    let bbox = [
        read_i16(bytes, 36, Cff1Error::InvalidHead)?,
        read_i16(bytes, 38, Cff1Error::InvalidHead)?,
        read_i16(bytes, 40, Cff1Error::InvalidHead)?,
        read_i16(bytes, 42, Cff1Error::InvalidHead)?,
    ];
    if units_per_em != 1_000 || bbox[0] >= bbox[2] || bbox[1] >= bbox[3] {
        return Err(Cff1Error::InvalidHead);
    }
    Ok(HeadFacts {
        bytes,
        units_per_em,
        bbox,
    })
}

fn parse_maxp(bytes: &[u8], limits: &M4EffectiveResourceLimits) -> Result<u16, Cff1Error> {
    if bytes.len() != 6 || read_u32(bytes, 0, Cff1Error::InvalidMaxp)? != 0x0000_5000 {
        return Err(Cff1Error::InvalidMaxp);
    }
    let glyph_count = read_u16(bytes, 4, Cff1Error::InvalidMaxp)?;
    if glyph_count == 0 {
        return Err(Cff1Error::InvalidMaxp);
    }
    if u32::from(glyph_count) > limits.extension().get().max_font_glyphs {
        return Err(Cff1Error::GlyphLimit);
    }
    Ok(glyph_count)
}

#[derive(Clone, Copy)]
struct HheaFacts<'a> {
    bytes: &'a [u8],
    number_of_h_metrics: u16,
}

fn parse_hhea(bytes: &[u8]) -> Result<HheaFacts<'_>, Cff1Error> {
    if bytes.len() != 36
        || read_u32(bytes, 0, Cff1Error::InvalidHhea)? != 0x0001_0000
        || bytes[24..34].iter().any(|byte| *byte != 0)
    {
        return Err(Cff1Error::InvalidHhea);
    }
    let number_of_h_metrics = read_u16(bytes, 34, Cff1Error::InvalidHhea)?;
    if number_of_h_metrics == 0 {
        return Err(Cff1Error::InvalidHhea);
    }
    Ok(HheaFacts {
        bytes,
        number_of_h_metrics,
    })
}

fn parse_hmtx(
    bytes: &[u8],
    glyph_count: u16,
    number_of_h_metrics: u16,
) -> Result<(Vec<u16>, Vec<i16>), Cff1Error> {
    if number_of_h_metrics > glyph_count {
        return Err(Cff1Error::InvalidHmtx);
    }
    let expected = usize::from(number_of_h_metrics)
        .checked_mul(4)
        .and_then(|value| {
            value.checked_add(usize::from(glyph_count - number_of_h_metrics).checked_mul(2)?)
        })
        .ok_or(Cff1Error::InvalidHmtx)?;
    if bytes.len() != expected {
        return Err(Cff1Error::InvalidHmtx);
    }
    let mut advances = Vec::new();
    let mut bearings = Vec::new();
    advances
        .try_reserve_exact(usize::from(glyph_count))
        .map_err(|_| Cff1Error::GlyphLimit)?;
    bearings
        .try_reserve_exact(usize::from(glyph_count))
        .map_err(|_| Cff1Error::GlyphLimit)?;
    let mut last_advance = None;
    for gid in 0..glyph_count {
        if gid < number_of_h_metrics {
            let offset = usize::from(gid) * 4;
            let advance = read_u16(bytes, offset, Cff1Error::InvalidHmtx)?;
            let bearing = read_i16(bytes, offset + 2, Cff1Error::InvalidHmtx)?;
            advances.push(advance);
            bearings.push(bearing);
            last_advance = Some(advance);
        } else {
            let offset =
                usize::from(number_of_h_metrics) * 4 + usize::from(gid - number_of_h_metrics) * 2;
            advances.push(last_advance.ok_or(Cff1Error::InvalidHmtx)?);
            bearings.push(read_i16(bytes, offset, Cff1Error::InvalidHmtx)?);
        }
    }
    Ok((advances, bearings))
}

#[derive(Clone, Copy)]
struct Os2Facts<'a> {
    bytes: &'a [u8],
    fs_type: u16,
}

fn parse_os2(bytes: &[u8]) -> Result<Os2Facts<'_>, Cff1Error> {
    let version = read_u16(bytes, 0, Cff1Error::InvalidOs2)?;
    let expected = match version {
        0 => 78,
        1 => 86,
        2..=4 => 96,
        5 => 100,
        _ => return Err(Cff1Error::InvalidOs2),
    };
    if bytes.len() != expected {
        return Err(Cff1Error::InvalidOs2);
    }
    Ok(Os2Facts {
        bytes,
        fs_type: read_u16(bytes, 8, Cff1Error::InvalidOs2)?,
    })
}

#[derive(Clone, Copy)]
struct PostFacts<'a> {
    bytes: &'a [u8],
}

fn parse_post(bytes: &[u8]) -> Result<PostFacts<'_>, Cff1Error> {
    if bytes.len() != 32 || read_u32(bytes, 0, Cff1Error::InvalidPost)? != 0x0003_0000 {
        return Err(Cff1Error::InvalidPost);
    }
    Ok(PostFacts { bytes })
}

fn embedding_permission(fs_type: u16) -> Result<Cff1EmbeddingPermission, Cff1Error> {
    match fs_type {
        0x0000 => Ok(Cff1EmbeddingPermission::Installable),
        0x0004 => Ok(Cff1EmbeddingPermission::PreviewAndPrint),
        0x0008 => Ok(Cff1EmbeddingPermission::Editable),
        _ => Err(Cff1Error::RestrictedEmbedding),
    }
}

fn validate_optional_tables(
    source: &[u8],
    tables: &BTreeMap<[u8; 4], TableRef<'_>>,
    glyph_count: u16,
) -> Result<(), Cff1Error> {
    let font = FontRef::new(source).map_err(|_| Cff1Error::InvalidOptionalTable)?;
    let all_glyphs = || {
        let mut glyphs = IntSet::empty();
        for gid in 0..glyph_count {
            glyphs.insert(GlyphId::new(u32::from(gid)));
        }
        glyphs
    };

    if let Some(table) = tables.get(b"BASE") {
        font.base().map_err(|_| Cff1Error::InvalidOptionalTable)?;
        validate_base_table(table.bytes, glyph_count)?;
    }
    if let Some(table) = tables.get(b"GDEF") {
        font.gdef().map_err(|_| Cff1Error::InvalidOptionalTable)?;
        validate_gdef_table(table.bytes, glyph_count)?;
    }
    if tables.contains_key(b"GSUB") {
        let gsub = font.gsub().map_err(|_| Cff1Error::InvalidOptionalTable)?;
        if read_u32(tables[b"GSUB"].bytes, 0, Cff1Error::InvalidOptionalTable)? != 0x0001_0000 {
            return Err(Cff1Error::InvalidOptionalTable);
        }
        let feature_list = gsub
            .feature_list()
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        let lookup_list = gsub
            .lookup_list()
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        gsub.script_list()
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        gsub.collect_features(&IntSet::all(), &IntSet::all(), &IntSet::all())
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        let mut feature_indices = IntSet::empty();
        for index in 0..feature_list.feature_count() {
            feature_indices.insert(index);
        }
        gsub.collect_lookups(&feature_indices)
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        let mut lookup_indices = IntSet::empty();
        for index in 0..lookup_list.lookup_count() {
            lookup_indices.insert(index);
        }
        let mut glyphs = all_glyphs();
        gsub.closure_glyphs(&lookup_indices, &mut glyphs)
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        if glyphs
            .iter()
            .any(|gid| gid.to_u32() >= u32::from(glyph_count))
        {
            return Err(Cff1Error::InvalidOptionalTable);
        }
    }
    let gpos_lookup_count = if tables.contains_key(b"GPOS") {
        let gpos = font.gpos().map_err(|_| Cff1Error::InvalidOptionalTable)?;
        if read_u32(tables[b"GPOS"].bytes, 0, Cff1Error::InvalidOptionalTable)? != 0x0001_0000 {
            return Err(Cff1Error::InvalidOptionalTable);
        }
        let feature_list = gpos
            .feature_list()
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        let lookup_list = gpos
            .lookup_list()
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        gpos.script_list()
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        gpos.collect_features(&IntSet::all(), &IntSet::all(), &IntSet::all())
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        let mut feature_indices = IntSet::empty();
        for index in 0..feature_list.feature_count() {
            feature_indices.insert(index);
        }
        gpos.collect_lookups(&feature_indices)
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        let mut lookup_indices = IntSet::empty();
        for index in 0..lookup_list.lookup_count() {
            lookup_indices.insert(index);
        }
        let glyphs = all_glyphs();
        gpos.closure_lookups(&glyphs, &mut lookup_indices)
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        for index in 0..lookup_list.lookup_count() {
            let lookup = lookup_list
                .lookups()
                .get(usize::from(index))
                .map_err(|_| Cff1Error::InvalidOptionalTable)?;
            lookup
                .subtables()
                .and_then(|subtables| subtables.intersects(&glyphs))
                .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        }
        lookup_list.lookup_count()
    } else {
        0
    };
    let gsub_lookup_count = if tables.contains_key(b"GSUB") {
        font.gsub()
            .and_then(|table| table.lookup_list())
            .map(|list| list.lookup_count())
            .map_err(|_| Cff1Error::InvalidOptionalTable)?
    } else {
        0
    };
    if let Some(table) = tables.get(b"JSTF") {
        validate_jstf_table(
            table.bytes,
            glyph_count,
            gsub_lookup_count,
            gpos_lookup_count,
        )?;
    }
    if let Some(table) = tables.get(b"MATH") {
        crate::math::validate_cff_math_table(table.bytes, glyph_count)
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
    }
    if let Some(table) = tables.get(b"kern") {
        font.kern().map_err(|_| Cff1Error::InvalidOptionalTable)?;
        validate_kern_table(table.bytes, glyph_count)?;
    }
    Ok(())
}

fn optional_range(bytes: &[u8], start: usize, length: usize) -> Result<&[u8], Cff1Error> {
    let end = start
        .checked_add(length)
        .ok_or(Cff1Error::InvalidOptionalTable)?;
    bytes.get(start..end).ok_or(Cff1Error::InvalidOptionalTable)
}

fn optional_relative(
    bytes: &[u8],
    base: usize,
    raw: u16,
    nullable: bool,
) -> Result<Option<usize>, Cff1Error> {
    if raw == 0 {
        return if nullable {
            Ok(None)
        } else {
            Err(Cff1Error::InvalidOptionalTable)
        };
    }
    base.checked_add(usize::from(raw))
        .filter(|offset| *offset < bytes.len())
        .map(Some)
        .ok_or(Cff1Error::InvalidOptionalTable)
}

fn optional_relative32(
    bytes: &[u8],
    base: usize,
    raw: u32,
    nullable: bool,
) -> Result<Option<usize>, Cff1Error> {
    if raw == 0 {
        return if nullable {
            Ok(None)
        } else {
            Err(Cff1Error::InvalidOptionalTable)
        };
    }
    base.checked_add(usize::try_from(raw).map_err(|_| Cff1Error::InvalidOptionalTable)?)
        .filter(|offset| *offset < bytes.len())
        .map(Some)
        .ok_or(Cff1Error::InvalidOptionalTable)
}

fn validate_tag_records(
    bytes: &[u8],
    start: usize,
    count: usize,
    record_size: usize,
) -> Result<(), Cff1Error> {
    optional_range(
        bytes,
        start,
        count
            .checked_mul(record_size)
            .ok_or(Cff1Error::InvalidOptionalTable)?,
    )?;
    let mut previous = None;
    for index in 0..count {
        let record = start + index * record_size;
        let tag: [u8; 4] = optional_range(bytes, record, 4)?
            .try_into()
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        if previous.is_some_and(|value| value >= tag) {
            return Err(Cff1Error::InvalidOptionalTable);
        }
        previous = Some(tag);
    }
    Ok(())
}

fn validate_coverage_table(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
) -> Result<usize, Cff1Error> {
    match read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)? {
        1 => {
            let count = usize::from(read_u16(
                bytes,
                offset + 2,
                Cff1Error::InvalidOptionalTable,
            )?);
            optional_range(bytes, offset + 4, count * 2)?;
            let mut previous = None;
            for index in 0..count {
                let glyph = read_u16(
                    bytes,
                    offset + 4 + index * 2,
                    Cff1Error::InvalidOptionalTable,
                )?;
                if glyph >= glyph_count || previous.is_some_and(|value| value >= glyph) {
                    return Err(Cff1Error::InvalidOptionalTable);
                }
                previous = Some(glyph);
            }
            Ok(count)
        }
        2 => {
            let range_count = usize::from(read_u16(
                bytes,
                offset + 2,
                Cff1Error::InvalidOptionalTable,
            )?);
            optional_range(bytes, offset + 4, range_count * 6)?;
            let mut previous_end = None;
            let mut covered = 0usize;
            for index in 0..range_count {
                let record = offset + 4 + index * 6;
                let first = read_u16(bytes, record, Cff1Error::InvalidOptionalTable)?;
                let last = read_u16(bytes, record + 2, Cff1Error::InvalidOptionalTable)?;
                let start_index = usize::from(read_u16(
                    bytes,
                    record + 4,
                    Cff1Error::InvalidOptionalTable,
                )?);
                if first > last
                    || last >= glyph_count
                    || previous_end.is_some_and(|value| value >= first)
                    || start_index != covered
                {
                    return Err(Cff1Error::InvalidOptionalTable);
                }
                covered = covered
                    .checked_add(usize::from(last - first) + 1)
                    .ok_or(Cff1Error::InvalidOptionalTable)?;
                previous_end = Some(last);
            }
            Ok(covered)
        }
        _ => Err(Cff1Error::InvalidOptionalTable),
    }
}

fn validate_class_def_table(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    max_class: Option<u16>,
) -> Result<(), Cff1Error> {
    match read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)? {
        1 => {
            let first = read_u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?;
            let count = usize::from(read_u16(
                bytes,
                offset + 4,
                Cff1Error::InvalidOptionalTable,
            )?);
            if usize::from(first)
                .checked_add(count)
                .map_or(true, |end| end > usize::from(glyph_count))
            {
                return Err(Cff1Error::InvalidOptionalTable);
            }
            optional_range(bytes, offset + 6, count * 2)?;
            for index in 0..count {
                let class = read_u16(
                    bytes,
                    offset + 6 + index * 2,
                    Cff1Error::InvalidOptionalTable,
                )?;
                if max_class.is_some_and(|limit| class > limit) {
                    return Err(Cff1Error::InvalidOptionalTable);
                }
            }
        }
        2 => {
            let count = usize::from(read_u16(
                bytes,
                offset + 2,
                Cff1Error::InvalidOptionalTable,
            )?);
            optional_range(bytes, offset + 4, count * 6)?;
            let mut previous_end = None;
            for index in 0..count {
                let record = offset + 4 + index * 6;
                let first = read_u16(bytes, record, Cff1Error::InvalidOptionalTable)?;
                let last = read_u16(bytes, record + 2, Cff1Error::InvalidOptionalTable)?;
                let class = read_u16(bytes, record + 4, Cff1Error::InvalidOptionalTable)?;
                if first > last
                    || last >= glyph_count
                    || previous_end.is_some_and(|value| value >= first)
                    || max_class.is_some_and(|limit| class > limit)
                {
                    return Err(Cff1Error::InvalidOptionalTable);
                }
                previous_end = Some(last);
            }
        }
        _ => return Err(Cff1Error::InvalidOptionalTable),
    }
    Ok(())
}

fn validate_device_table(bytes: &[u8], offset: usize) -> Result<(), Cff1Error> {
    let first = read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    let last = read_u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?;
    let format = read_u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)?;
    let bits = match format {
        1 => 2usize,
        2 => 4,
        3 => 8,
        _ => return Err(Cff1Error::InvalidOptionalTable),
    };
    if first > last {
        return Err(Cff1Error::InvalidOptionalTable);
    }
    let values = usize::from(last - first) + 1;
    let words = values
        .checked_mul(bits)
        .and_then(|value| value.checked_add(15))
        .map(|value| value / 16)
        .ok_or(Cff1Error::InvalidOptionalTable)?;
    optional_range(bytes, offset + 6, words * 2)?;
    Ok(())
}

fn validate_base_coord(bytes: &[u8], offset: usize, glyph_count: u16) -> Result<(), Cff1Error> {
    match read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)? {
        1 => {
            optional_range(bytes, offset, 4)?;
        }
        2 => {
            optional_range(bytes, offset, 8)?;
            if read_u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)? >= glyph_count {
                return Err(Cff1Error::InvalidOptionalTable);
            }
        }
        3 => {
            optional_range(bytes, offset, 6)?;
            let device = read_u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)?;
            if let Some(device) = optional_relative(bytes, offset, device, true)? {
                validate_device_table(bytes, device)?;
            }
        }
        _ => return Err(Cff1Error::InvalidOptionalTable),
    }
    Ok(())
}

fn validate_base_min_max(bytes: &[u8], offset: usize, glyph_count: u16) -> Result<(), Cff1Error> {
    optional_range(bytes, offset, 6)?;
    for field in [0usize, 2] {
        let raw = read_u16(bytes, offset + field, Cff1Error::InvalidOptionalTable)?;
        if let Some(coord) = optional_relative(bytes, offset, raw, true)? {
            validate_base_coord(bytes, coord, glyph_count)?;
        }
    }
    let count = usize::from(read_u16(
        bytes,
        offset + 4,
        Cff1Error::InvalidOptionalTable,
    )?);
    validate_tag_records(bytes, offset + 6, count, 8)?;
    for index in 0..count {
        let record = offset + 6 + index * 8;
        for field in [4usize, 6] {
            let raw = read_u16(bytes, record + field, Cff1Error::InvalidOptionalTable)?;
            if let Some(coord) = optional_relative(bytes, offset, raw, true)? {
                validate_base_coord(bytes, coord, glyph_count)?;
            }
        }
    }
    Ok(())
}

fn validate_base_values(bytes: &[u8], offset: usize, glyph_count: u16) -> Result<(), Cff1Error> {
    let default_index = usize::from(read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)?);
    let count = usize::from(read_u16(
        bytes,
        offset + 2,
        Cff1Error::InvalidOptionalTable,
    )?);
    if count == 0 || default_index >= count {
        return Err(Cff1Error::InvalidOptionalTable);
    }
    optional_range(bytes, offset + 4, count * 2)?;
    for index in 0..count {
        let raw = read_u16(
            bytes,
            offset + 4 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        let coord =
            optional_relative(bytes, offset, raw, false)?.ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_base_coord(bytes, coord, glyph_count)?;
    }
    Ok(())
}

fn validate_base_script(bytes: &[u8], offset: usize, glyph_count: u16) -> Result<(), Cff1Error> {
    optional_range(bytes, offset, 6)?;
    let values = read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    if let Some(values) = optional_relative(bytes, offset, values, true)? {
        validate_base_values(bytes, values, glyph_count)?;
    }
    let min_max = read_u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?;
    if let Some(min_max) = optional_relative(bytes, offset, min_max, true)? {
        validate_base_min_max(bytes, min_max, glyph_count)?;
    }
    let count = usize::from(read_u16(
        bytes,
        offset + 4,
        Cff1Error::InvalidOptionalTable,
    )?);
    validate_tag_records(bytes, offset + 6, count, 6)?;
    for index in 0..count {
        let record = offset + 6 + index * 6;
        let raw = read_u16(bytes, record + 4, Cff1Error::InvalidOptionalTable)?;
        let lang =
            optional_relative(bytes, offset, raw, false)?.ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_base_min_max(bytes, lang, glyph_count)?;
    }
    Ok(())
}

fn validate_base_axis(bytes: &[u8], offset: usize, glyph_count: u16) -> Result<(), Cff1Error> {
    optional_range(bytes, offset, 4)?;
    let tags = read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    if let Some(tags) = optional_relative(bytes, offset, tags, true)? {
        let count = usize::from(read_u16(bytes, tags, Cff1Error::InvalidOptionalTable)?);
        validate_tag_records(bytes, tags + 2, count, 4)?;
    }
    let scripts = read_u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?;
    let scripts =
        optional_relative(bytes, offset, scripts, false)?.ok_or(Cff1Error::InvalidOptionalTable)?;
    let count = usize::from(read_u16(bytes, scripts, Cff1Error::InvalidOptionalTable)?);
    validate_tag_records(bytes, scripts + 2, count, 6)?;
    for index in 0..count {
        let record = scripts + 2 + index * 6;
        let raw = read_u16(bytes, record + 4, Cff1Error::InvalidOptionalTable)?;
        let script = optional_relative(bytes, scripts, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_base_script(bytes, script, glyph_count)?;
    }
    Ok(())
}

fn validate_base_table(bytes: &[u8], glyph_count: u16) -> Result<(), Cff1Error> {
    if bytes.len() < 8 || read_u32(bytes, 0, Cff1Error::InvalidOptionalTable)? != 0x0001_0000 {
        return Err(Cff1Error::InvalidOptionalTable);
    }
    for field in [4usize, 6] {
        let raw = read_u16(bytes, field, Cff1Error::InvalidOptionalTable)?;
        if let Some(axis) = optional_relative(bytes, 0, raw, true)? {
            validate_base_axis(bytes, axis, glyph_count)?;
        }
    }
    Ok(())
}

fn validate_attach_list(bytes: &[u8], offset: usize, glyph_count: u16) -> Result<(), Cff1Error> {
    let coverage_raw = read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    let count = usize::from(read_u16(
        bytes,
        offset + 2,
        Cff1Error::InvalidOptionalTable,
    )?);
    let coverage = optional_relative(bytes, offset, coverage_raw, false)?
        .ok_or(Cff1Error::InvalidOptionalTable)?;
    if validate_coverage_table(bytes, coverage, glyph_count)? != count {
        return Err(Cff1Error::InvalidOptionalTable);
    }
    optional_range(bytes, offset + 4, count * 2)?;
    for index in 0..count {
        let raw = read_u16(
            bytes,
            offset + 4 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        let point =
            optional_relative(bytes, offset, raw, false)?.ok_or(Cff1Error::InvalidOptionalTable)?;
        let point_count = usize::from(read_u16(bytes, point, Cff1Error::InvalidOptionalTable)?);
        optional_range(bytes, point + 2, point_count * 2)?;
    }
    Ok(())
}

fn validate_caret_value(bytes: &[u8], offset: usize) -> Result<(), Cff1Error> {
    match read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)? {
        1 | 2 => {
            optional_range(bytes, offset, 4)?;
        }
        3 => {
            optional_range(bytes, offset, 6)?;
            let raw = read_u16(bytes, offset + 4, Cff1Error::InvalidOptionalTable)?;
            let device = optional_relative(bytes, offset, raw, false)?
                .ok_or(Cff1Error::InvalidOptionalTable)?;
            validate_device_table(bytes, device)?;
        }
        _ => return Err(Cff1Error::InvalidOptionalTable),
    }
    Ok(())
}

fn validate_lig_caret_list(bytes: &[u8], offset: usize, glyph_count: u16) -> Result<(), Cff1Error> {
    let coverage_raw = read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    let count = usize::from(read_u16(
        bytes,
        offset + 2,
        Cff1Error::InvalidOptionalTable,
    )?);
    let coverage = optional_relative(bytes, offset, coverage_raw, false)?
        .ok_or(Cff1Error::InvalidOptionalTable)?;
    if validate_coverage_table(bytes, coverage, glyph_count)? != count {
        return Err(Cff1Error::InvalidOptionalTable);
    }
    optional_range(bytes, offset + 4, count * 2)?;
    for index in 0..count {
        let raw = read_u16(
            bytes,
            offset + 4 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        let ligature =
            optional_relative(bytes, offset, raw, false)?.ok_or(Cff1Error::InvalidOptionalTable)?;
        let caret_count = usize::from(read_u16(bytes, ligature, Cff1Error::InvalidOptionalTable)?);
        if caret_count == 0 {
            return Err(Cff1Error::InvalidOptionalTable);
        }
        optional_range(bytes, ligature + 2, caret_count * 2)?;
        for caret in 0..caret_count {
            let raw = read_u16(
                bytes,
                ligature + 2 + caret * 2,
                Cff1Error::InvalidOptionalTable,
            )?;
            let value = optional_relative(bytes, ligature, raw, false)?
                .ok_or(Cff1Error::InvalidOptionalTable)?;
            validate_caret_value(bytes, value)?;
        }
    }
    Ok(())
}

fn validate_mark_glyph_sets(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
) -> Result<(), Cff1Error> {
    if read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)? != 1 {
        return Err(Cff1Error::InvalidOptionalTable);
    }
    let count = usize::from(read_u16(
        bytes,
        offset + 2,
        Cff1Error::InvalidOptionalTable,
    )?);
    optional_range(bytes, offset + 4, count * 4)?;
    for index in 0..count {
        let raw = read_u32(
            bytes,
            offset + 4 + index * 4,
            Cff1Error::InvalidOptionalTable,
        )?;
        let coverage = optional_relative32(bytes, offset, raw, false)?
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_coverage_table(bytes, coverage, glyph_count)?;
    }
    Ok(())
}

fn validate_gdef_table(bytes: &[u8], glyph_count: u16) -> Result<(), Cff1Error> {
    let version = read_u32(bytes, 0, Cff1Error::InvalidOptionalTable)?;
    let header_length = match version {
        0x0001_0000 => 12,
        0x0001_0002 => 14,
        _ => return Err(Cff1Error::InvalidOptionalTable),
    };
    optional_range(bytes, 0, header_length)?;
    let glyph_class = read_u16(bytes, 4, Cff1Error::InvalidOptionalTable)?;
    if let Some(class) = optional_relative(bytes, 0, glyph_class, true)? {
        validate_class_def_table(bytes, class, glyph_count, Some(4))?;
    }
    let attach = read_u16(bytes, 6, Cff1Error::InvalidOptionalTable)?;
    if let Some(attach) = optional_relative(bytes, 0, attach, true)? {
        validate_attach_list(bytes, attach, glyph_count)?;
    }
    let carets = read_u16(bytes, 8, Cff1Error::InvalidOptionalTable)?;
    if let Some(carets) = optional_relative(bytes, 0, carets, true)? {
        validate_lig_caret_list(bytes, carets, glyph_count)?;
    }
    let mark_class = read_u16(bytes, 10, Cff1Error::InvalidOptionalTable)?;
    if let Some(class) = optional_relative(bytes, 0, mark_class, true)? {
        validate_class_def_table(bytes, class, glyph_count, None)?;
    }
    if version == 0x0001_0002 {
        let sets = read_u16(bytes, 12, Cff1Error::InvalidOptionalTable)?;
        if let Some(sets) = optional_relative(bytes, 0, sets, true)? {
            validate_mark_glyph_sets(bytes, sets, glyph_count)?;
        }
    }
    Ok(())
}

fn validate_jstf_mod_list(bytes: &[u8], offset: usize, lookup_count: u16) -> Result<(), Cff1Error> {
    let count = usize::from(read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)?);
    optional_range(bytes, offset + 2, count * 2)?;
    let mut previous = None;
    for index in 0..count {
        let lookup = read_u16(
            bytes,
            offset + 2 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        if lookup >= lookup_count || previous.is_some_and(|value| value >= lookup) {
            return Err(Cff1Error::InvalidOptionalTable);
        }
        previous = Some(lookup);
    }
    Ok(())
}

fn validate_jstf_max(bytes: &[u8], offset: usize, glyph_count: u16) -> Result<(), Cff1Error> {
    let count = usize::from(read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)?);
    optional_range(bytes, offset + 2, count * 2)?;
    let mut glyphs = IntSet::empty();
    for gid in 0..glyph_count {
        glyphs.insert(GlyphId::new(u32::from(gid)));
    }
    for index in 0..count {
        let raw = read_u16(
            bytes,
            offset + 2 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        let lookup =
            optional_relative(bytes, offset, raw, false)?.ok_or(Cff1Error::InvalidOptionalTable)?;
        let lookup = PositionLookup::read(FontData::new(&bytes[lookup..]))
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
        lookup
            .subtables()
            .and_then(|subtables| subtables.intersects(&glyphs))
            .map_err(|_| Cff1Error::InvalidOptionalTable)?;
    }
    Ok(())
}

fn validate_jstf_priority(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    gsub_lookup_count: u16,
    gpos_lookup_count: u16,
) -> Result<(), Cff1Error> {
    optional_range(bytes, offset, 20)?;
    for field in [0usize, 2, 10, 12] {
        let raw = read_u16(bytes, offset + field, Cff1Error::InvalidOptionalTable)?;
        if let Some(list) = optional_relative(bytes, offset, raw, true)? {
            validate_jstf_mod_list(bytes, list, gsub_lookup_count)?;
        }
    }
    for field in [4usize, 6, 14, 16] {
        let raw = read_u16(bytes, offset + field, Cff1Error::InvalidOptionalTable)?;
        if let Some(list) = optional_relative(bytes, offset, raw, true)? {
            validate_jstf_mod_list(bytes, list, gpos_lookup_count)?;
        }
    }
    for field in [8usize, 18] {
        let raw = read_u16(bytes, offset + field, Cff1Error::InvalidOptionalTable)?;
        if let Some(max) = optional_relative(bytes, offset, raw, true)? {
            validate_jstf_max(bytes, max, glyph_count)?;
        }
    }
    Ok(())
}

fn validate_jstf_lang_sys(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    gsub_lookup_count: u16,
    gpos_lookup_count: u16,
) -> Result<(), Cff1Error> {
    let count = usize::from(read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)?);
    optional_range(bytes, offset + 2, count * 2)?;
    for index in 0..count {
        let raw = read_u16(
            bytes,
            offset + 2 + index * 2,
            Cff1Error::InvalidOptionalTable,
        )?;
        let priority =
            optional_relative(bytes, offset, raw, false)?.ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_jstf_priority(
            bytes,
            priority,
            glyph_count,
            gsub_lookup_count,
            gpos_lookup_count,
        )?;
    }
    Ok(())
}

fn validate_jstf_script(
    bytes: &[u8],
    offset: usize,
    glyph_count: u16,
    gsub_lookup_count: u16,
    gpos_lookup_count: u16,
) -> Result<(), Cff1Error> {
    optional_range(bytes, offset, 6)?;
    let extenders = read_u16(bytes, offset, Cff1Error::InvalidOptionalTable)?;
    if let Some(extenders) = optional_relative(bytes, offset, extenders, true)? {
        let count = usize::from(read_u16(bytes, extenders, Cff1Error::InvalidOptionalTable)?);
        optional_range(bytes, extenders + 2, count * 2)?;
        let mut previous = None;
        for index in 0..count {
            let glyph = read_u16(
                bytes,
                extenders + 2 + index * 2,
                Cff1Error::InvalidOptionalTable,
            )?;
            if glyph >= glyph_count || previous.is_some_and(|value| value >= glyph) {
                return Err(Cff1Error::InvalidOptionalTable);
            }
            previous = Some(glyph);
        }
    }
    let default = read_u16(bytes, offset + 2, Cff1Error::InvalidOptionalTable)?;
    if let Some(default) = optional_relative(bytes, offset, default, true)? {
        validate_jstf_lang_sys(
            bytes,
            default,
            glyph_count,
            gsub_lookup_count,
            gpos_lookup_count,
        )?;
    }
    let count = usize::from(read_u16(
        bytes,
        offset + 4,
        Cff1Error::InvalidOptionalTable,
    )?);
    validate_tag_records(bytes, offset + 6, count, 6)?;
    for index in 0..count {
        let record = offset + 6 + index * 6;
        let raw = read_u16(bytes, record + 4, Cff1Error::InvalidOptionalTable)?;
        let lang =
            optional_relative(bytes, offset, raw, false)?.ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_jstf_lang_sys(
            bytes,
            lang,
            glyph_count,
            gsub_lookup_count,
            gpos_lookup_count,
        )?;
    }
    Ok(())
}

fn validate_jstf_table(
    bytes: &[u8],
    glyph_count: u16,
    gsub_lookup_count: u16,
    gpos_lookup_count: u16,
) -> Result<(), Cff1Error> {
    if bytes.len() < 6 || read_u32(bytes, 0, Cff1Error::InvalidOptionalTable)? != 0x0001_0000 {
        return Err(Cff1Error::InvalidOptionalTable);
    }
    let count = usize::from(read_u16(bytes, 4, Cff1Error::InvalidOptionalTable)?);
    validate_tag_records(bytes, 6, count, 6)?;
    for index in 0..count {
        let record = 6 + index * 6;
        let raw = read_u16(bytes, record + 4, Cff1Error::InvalidOptionalTable)?;
        let script =
            optional_relative(bytes, 0, raw, false)?.ok_or(Cff1Error::InvalidOptionalTable)?;
        validate_jstf_script(
            bytes,
            script,
            glyph_count,
            gsub_lookup_count,
            gpos_lookup_count,
        )?;
    }
    Ok(())
}

fn validate_kern_table(bytes: &[u8], glyph_count: u16) -> Result<(), Cff1Error> {
    if read_u16(bytes, 0, Cff1Error::InvalidOptionalTable)? != 0 {
        return Err(Cff1Error::InvalidOptionalTable);
    }
    let count = usize::from(read_u16(bytes, 2, Cff1Error::InvalidOptionalTable)?);
    let mut cursor = 4usize;
    for _ in 0..count {
        optional_range(bytes, cursor, 14)?;
        let version = read_u16(bytes, cursor, Cff1Error::InvalidOptionalTable)?;
        let length = usize::from(read_u16(
            bytes,
            cursor + 2,
            Cff1Error::InvalidOptionalTable,
        )?);
        let coverage = read_u16(bytes, cursor + 4, Cff1Error::InvalidOptionalTable)?;
        let pair_count = usize::from(read_u16(
            bytes,
            cursor + 6,
            Cff1Error::InvalidOptionalTable,
        )?);
        let expected_length = 14usize
            .checked_add(
                pair_count
                    .checked_mul(6)
                    .ok_or(Cff1Error::InvalidOptionalTable)?,
            )
            .ok_or(Cff1Error::InvalidOptionalTable)?;
        let power = if pair_count == 0 {
            0
        } else {
            1usize << (usize::BITS - 1 - pair_count.leading_zeros())
        };
        let expected_search = power * 6;
        let expected_selector = if power == 0 {
            0
        } else {
            usize::try_from(power.trailing_zeros()).map_err(|_| Cff1Error::InvalidOptionalTable)?
        };
        let expected_shift = pair_count * 6 - expected_search;
        if version != 0
            || coverage >> 8 != 0
            || coverage & !0x000F != 0
            || coverage & 0x0001 == 0
            || length != expected_length
            || usize::from(read_u16(
                bytes,
                cursor + 8,
                Cff1Error::InvalidOptionalTable,
            )?) != expected_search
            || usize::from(read_u16(
                bytes,
                cursor + 10,
                Cff1Error::InvalidOptionalTable,
            )?) != expected_selector
            || usize::from(read_u16(
                bytes,
                cursor + 12,
                Cff1Error::InvalidOptionalTable,
            )?) != expected_shift
        {
            return Err(Cff1Error::InvalidOptionalTable);
        }
        optional_range(bytes, cursor, length)?;
        let mut previous = None;
        for index in 0..pair_count {
            let pair = cursor + 14 + index * 6;
            let left = read_u16(bytes, pair, Cff1Error::InvalidOptionalTable)?;
            let right = read_u16(bytes, pair + 2, Cff1Error::InvalidOptionalTable)?;
            if left >= glyph_count
                || right >= glyph_count
                || previous.is_some_and(|value| value >= (left, right))
            {
                return Err(Cff1Error::InvalidOptionalTable);
            }
            previous = Some((left, right));
        }
        cursor = cursor
            .checked_add(length)
            .ok_or(Cff1Error::InvalidOptionalTable)?;
    }
    if cursor != bytes.len() {
        return Err(Cff1Error::InvalidOptionalTable);
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize, error: Cff1Error) -> Result<u16, Cff1Error> {
    let end = offset.checked_add(2).ok_or(error)?;
    Ok(u16::from_be_bytes(
        bytes
            .get(offset..end)
            .ok_or(error)?
            .try_into()
            .map_err(|_| error)?,
    ))
}

fn read_i16(bytes: &[u8], offset: usize, error: Cff1Error) -> Result<i16, Cff1Error> {
    let end = offset.checked_add(2).ok_or(error)?;
    Ok(i16::from_be_bytes(
        bytes
            .get(offset..end)
            .ok_or(error)?
            .try_into()
            .map_err(|_| error)?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize, error: Cff1Error) -> Result<u32, Cff1Error> {
    let end = offset.checked_add(4).ok_or(error)?;
    Ok(u32::from_be_bytes(
        bytes
            .get(offset..end)
            .ok_or(error)?
            .try_into()
            .map_err(|_| error)?,
    ))
}

fn read_i32(bytes: &[u8], offset: usize, error: Cff1Error) -> Result<i32, Cff1Error> {
    let end = offset.checked_add(4).ok_or(error)?;
    Ok(i32::from_be_bytes(
        bytes
            .get(offset..end)
            .ok_or(error)?
            .try_into()
            .map_err(|_| error)?,
    ))
}

fn sfnt_checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

#[derive(Clone, Debug)]
struct NameFacts {
    family: String,
    subfamily: String,
    postscript_name: String,
}

#[derive(Clone, Debug)]
struct NameCandidate {
    rank: (u8, u16, u16),
    name_id: u16,
    value: String,
}

fn parse_name(bytes: &[u8]) -> Result<NameFacts, Cff1Error> {
    if read_u16(bytes, 0, Cff1Error::InvalidName)? != 0 {
        return Err(Cff1Error::InvalidName);
    }
    let count = usize::from(read_u16(bytes, 2, Cff1Error::InvalidName)?);
    let string_offset = usize::from(read_u16(bytes, 4, Cff1Error::InvalidName)?);
    let records_end = 6usize
        .checked_add(count.checked_mul(12).ok_or(Cff1Error::InvalidName)?)
        .ok_or(Cff1Error::InvalidName)?;
    if count == 0 || string_offset < records_end || string_offset > bytes.len() {
        return Err(Cff1Error::InvalidName);
    }
    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(count)
        .map_err(|_| Cff1Error::InvalidName)?;
    for index in 0..count {
        let record = 6 + index * 12;
        let platform = read_u16(bytes, record, Cff1Error::InvalidName)?;
        let encoding = read_u16(bytes, record + 2, Cff1Error::InvalidName)?;
        let language = read_u16(bytes, record + 4, Cff1Error::InvalidName)?;
        let name_id = read_u16(bytes, record + 6, Cff1Error::InvalidName)?;
        let length = usize::from(read_u16(bytes, record + 8, Cff1Error::InvalidName)?);
        let local_offset = usize::from(read_u16(bytes, record + 10, Cff1Error::InvalidName)?);
        let start = string_offset
            .checked_add(local_offset)
            .ok_or(Cff1Error::InvalidName)?;
        let end = start.checked_add(length).ok_or(Cff1Error::InvalidName)?;
        let payload = bytes.get(start..end).ok_or(Cff1Error::InvalidName)?;
        if !matches!(name_id, 1 | 2 | 6) {
            continue;
        }
        let rank = match (platform, encoding, language) {
            (3, 10, 0x0409) => (0, encoding, language),
            (3, 1, 0x0409) => (1, encoding, language),
            (0, _, _) => (2, encoding, language),
            (3, 10 | 1, _) => (3, encoding, language),
            _ => continue,
        };
        let value = decode_utf16_be(payload)?;
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(Cff1Error::InvalidName);
        }
        candidates.push(NameCandidate {
            rank,
            name_id,
            value,
        });
    }
    let choose = |name_id| -> Result<String, Cff1Error> {
        candidates
            .iter()
            .filter(|candidate| candidate.name_id == name_id)
            .min_by_key(|candidate| candidate.rank)
            .map(|candidate| candidate.value.clone())
            .ok_or(Cff1Error::InvalidName)
    };
    let family = choose(1)?;
    let subfamily = choose(2)?;
    let postscript_name = choose(6)?;
    if !valid_postscript_name(&postscript_name) {
        return Err(Cff1Error::InvalidName);
    }
    Ok(NameFacts {
        family,
        subfamily,
        postscript_name,
    })
}

fn decode_utf16_be(bytes: &[u8]) -> Result<String, Cff1Error> {
    if bytes.len() % 2 != 0 {
        return Err(Cff1Error::InvalidName);
    }
    let units = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]));
    let mut output = String::new();
    for scalar in char::decode_utf16(units) {
        let scalar = scalar.map_err(|_| Cff1Error::InvalidName)?;
        if u32::from(scalar) > 0xFFFF {
            return Err(Cff1Error::InvalidName);
        }
        output.push(scalar);
    }
    Ok(output)
}

fn valid_postscript_name(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 127
        || !value
            .bytes()
            .all(|byte| (33..=126).contains(&byte) && !b"()<>[]{}/%".contains(&byte))
    {
        return false;
    }
    let bytes = value.as_bytes();
    !(bytes.len() > 7
        && bytes[..6].iter().all(u8::is_ascii_uppercase)
        && bytes.get(6) == Some(&b'+'))
}

fn parse_cmap(bytes: &[u8], glyph_count: u16) -> Result<BTreeMap<u32, u16>, Cff1Error> {
    if read_u16(bytes, 0, Cff1Error::InvalidCmap)? != 0 {
        return Err(Cff1Error::InvalidCmap);
    }
    let count = usize::from(read_u16(bytes, 2, Cff1Error::InvalidCmap)?);
    if count == 0 {
        return Err(Cff1Error::InvalidCmap);
    }
    let records_end = 4usize
        .checked_add(count.checked_mul(8).ok_or(Cff1Error::InvalidCmap)?)
        .ok_or(Cff1Error::InvalidCmap)?;
    if records_end > bytes.len() {
        return Err(Cff1Error::InvalidCmap);
    }
    let mut merged = BTreeMap::new();
    for index in 0..count {
        let record = 4 + index * 8;
        let platform = read_u16(bytes, record, Cff1Error::InvalidCmap)?;
        let encoding = read_u16(bytes, record + 2, Cff1Error::InvalidCmap)?;
        if !matches!((platform, encoding), (0, _) | (3, 1) | (3, 10)) {
            return Err(Cff1Error::InvalidCmap);
        }
        let offset = usize::try_from(read_u32(bytes, record + 4, Cff1Error::InvalidCmap)?)
            .map_err(|_| Cff1Error::InvalidCmap)?;
        if offset < records_end {
            return Err(Cff1Error::InvalidCmap);
        }
        let format = read_u16(bytes, offset, Cff1Error::InvalidCmap)?;
        let mappings = match format {
            4 if platform == 0 || (platform == 3 && encoding == 1) => {
                parse_cmap_format4(bytes, offset, glyph_count)?
            }
            12 if platform == 0 || (platform == 3 && encoding == 10) => {
                parse_cmap_format12(bytes, offset, glyph_count)?
            }
            _ => return Err(Cff1Error::InvalidCmap),
        };
        for (scalar, gid) in mappings {
            if merged
                .insert(scalar, gid)
                .is_some_and(|previous| previous != gid)
            {
                return Err(Cff1Error::InvalidCmap);
            }
        }
    }
    if merged.is_empty() {
        return Err(Cff1Error::InvalidCmap);
    }
    Ok(merged)
}

fn parse_cmap_format4(
    table: &[u8],
    offset: usize,
    glyph_count: u16,
) -> Result<Vec<(u32, u16)>, Cff1Error> {
    let length = usize::from(read_u16(table, offset + 2, Cff1Error::InvalidCmap)?);
    let end = offset.checked_add(length).ok_or(Cff1Error::InvalidCmap)?;
    if length < 24 || end > table.len() || read_u16(table, offset + 4, Cff1Error::InvalidCmap)? != 0
    {
        return Err(Cff1Error::InvalidCmap);
    }
    let seg_count_x2 = read_u16(table, offset + 6, Cff1Error::InvalidCmap)?;
    if seg_count_x2 == 0 || seg_count_x2 % 2 != 0 {
        return Err(Cff1Error::InvalidCmap);
    }
    let seg_count = usize::from(seg_count_x2 / 2);
    let power = 1usize << (usize::BITS - 1 - seg_count.leading_zeros());
    if read_u16(table, offset + 8, Cff1Error::InvalidCmap)?
        != u16::try_from(power * 2).map_err(|_| Cff1Error::InvalidCmap)?
        || read_u16(table, offset + 10, Cff1Error::InvalidCmap)?
            != u16::try_from(power.trailing_zeros()).map_err(|_| Cff1Error::InvalidCmap)?
        || read_u16(table, offset + 12, Cff1Error::InvalidCmap)?
            != seg_count_x2 - u16::try_from(power * 2).map_err(|_| Cff1Error::InvalidCmap)?
    {
        return Err(Cff1Error::InvalidCmap);
    }
    let end_codes = offset + 14;
    let reserved_pad = end_codes + seg_count * 2;
    let start_codes = reserved_pad + 2;
    let deltas = start_codes + seg_count * 2;
    let range_offsets = deltas + seg_count * 2;
    let arrays_end = range_offsets + seg_count * 2;
    if arrays_end > end || read_u16(table, reserved_pad, Cff1Error::InvalidCmap)? != 0 {
        return Err(Cff1Error::InvalidCmap);
    }
    let mut mappings = Vec::new();
    let mut previous_end = None;
    for segment in 0..seg_count {
        let start = read_u16(table, start_codes + segment * 2, Cff1Error::InvalidCmap)?;
        let finish = read_u16(table, end_codes + segment * 2, Cff1Error::InvalidCmap)?;
        if start > finish || previous_end.is_some_and(|previous| previous >= start) {
            return Err(Cff1Error::InvalidCmap);
        }
        previous_end = Some(finish);
        let delta = read_i16(table, deltas + segment * 2, Cff1Error::InvalidCmap)?;
        let range = read_u16(table, range_offsets + segment * 2, Cff1Error::InvalidCmap)?;
        for scalar in start..=finish {
            let gid = if range == 0 {
                scalar.wrapping_add_signed(delta)
            } else {
                let word_position = range_offsets
                    .checked_add(segment * 2)
                    .and_then(|value| value.checked_add(usize::from(range)))
                    .and_then(|value| {
                        value.checked_add(usize::from(scalar - start).checked_mul(2)?)
                    })
                    .ok_or(Cff1Error::InvalidCmap)?;
                if word_position + 2 > end {
                    return Err(Cff1Error::InvalidCmap);
                }
                let indexed = read_u16(table, word_position, Cff1Error::InvalidCmap)?;
                if indexed == 0 {
                    0
                } else {
                    indexed.wrapping_add_signed(delta)
                }
            };
            if gid != 0 {
                let scalar32 = u32::from(scalar);
                if (0xD800..=0xDFFF).contains(&scalar32) || gid >= glyph_count {
                    return Err(Cff1Error::InvalidCmap);
                }
                mappings.push((scalar32, gid));
            }
        }
    }
    if previous_end != Some(0xFFFF) {
        return Err(Cff1Error::InvalidCmap);
    }
    Ok(mappings)
}

fn parse_cmap_format12(
    table: &[u8],
    offset: usize,
    glyph_count: u16,
) -> Result<Vec<(u32, u16)>, Cff1Error> {
    if read_u16(table, offset + 2, Cff1Error::InvalidCmap)? != 0 {
        return Err(Cff1Error::InvalidCmap);
    }
    let length = usize::try_from(read_u32(table, offset + 4, Cff1Error::InvalidCmap)?)
        .map_err(|_| Cff1Error::InvalidCmap)?;
    let end = offset.checked_add(length).ok_or(Cff1Error::InvalidCmap)?;
    let groups = usize::try_from(read_u32(table, offset + 12, Cff1Error::InvalidCmap)?)
        .map_err(|_| Cff1Error::InvalidCmap)?;
    if length < 16
        || end > table.len()
        || 16usize.checked_add(groups.checked_mul(12).ok_or(Cff1Error::InvalidCmap)?)
            != Some(length)
    {
        return Err(Cff1Error::InvalidCmap);
    }
    let mut mappings = Vec::new();
    let mut previous_end = None;
    for index in 0..groups {
        let group = offset + 16 + index * 12;
        let start = read_u32(table, group, Cff1Error::InvalidCmap)?;
        let finish = read_u32(table, group + 4, Cff1Error::InvalidCmap)?;
        let start_gid = read_u32(table, group + 8, Cff1Error::InvalidCmap)?;
        if start > finish
            || finish > 0x10FFFF
            || previous_end.is_some_and(|previous| previous >= start)
            || (start <= 0xDFFF && finish >= 0xD800)
        {
            return Err(Cff1Error::InvalidCmap);
        }
        previous_end = Some(finish);
        for scalar in start..=finish {
            let gid = start_gid
                .checked_add(scalar - start)
                .ok_or(Cff1Error::InvalidCmap)?;
            if gid != 0 {
                if gid >= u32::from(glyph_count) {
                    return Err(Cff1Error::InvalidCmap);
                }
                mappings.push((
                    scalar,
                    u16::try_from(gid).map_err(|_| Cff1Error::InvalidCmap)?,
                ));
            }
        }
    }
    Ok(mappings)
}

#[derive(Clone, Debug)]
struct CffIndex {
    objects: Vec<Vec<u8>>,
    start: usize,
    end: usize,
}

fn parse_cff_index(
    bytes: &[u8],
    offset: usize,
    maximum_count: Option<u32>,
) -> Result<CffIndex, Cff1Error> {
    let count = usize::from(read_u16(bytes, offset, Cff1Error::InvalidCff)?);
    if maximum_count
        .is_some_and(|maximum| u32::try_from(count).map_or(true, |count| count > maximum))
    {
        return Err(Cff1Error::SubroutineLimit);
    }
    if count == 0 {
        return Ok(CffIndex {
            objects: Vec::new(),
            start: offset,
            end: offset.checked_add(2).ok_or(Cff1Error::InvalidCff)?,
        });
    }
    let off_size_position = offset.checked_add(2).ok_or(Cff1Error::InvalidCff)?;
    let off_size = usize::from(*bytes.get(off_size_position).ok_or(Cff1Error::InvalidCff)?);
    if !(1..=4).contains(&off_size) {
        return Err(Cff1Error::InvalidCff);
    }
    let offsets_start = off_size_position + 1;
    let offsets_bytes = count
        .checked_add(1)
        .and_then(|value| value.checked_mul(off_size))
        .ok_or(Cff1Error::InvalidCff)?;
    let data_start = offsets_start
        .checked_add(offsets_bytes)
        .ok_or(Cff1Error::InvalidCff)?;
    if data_start > bytes.len() {
        return Err(Cff1Error::InvalidCff);
    }
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(count + 1)
        .map_err(|_| Cff1Error::InvalidCff)?;
    for index in 0..=count {
        let position = offsets_start + index * off_size;
        let mut value = 0usize;
        for byte in bytes
            .get(position..position + off_size)
            .ok_or(Cff1Error::InvalidCff)?
        {
            value = value
                .checked_mul(256)
                .and_then(|value| value.checked_add(usize::from(*byte)))
                .ok_or(Cff1Error::InvalidCff)?;
        }
        if value == 0 || offsets.last().is_some_and(|previous| *previous > value) {
            return Err(Cff1Error::InvalidCff);
        }
        offsets.push(value);
    }
    if offsets.first() != Some(&1) {
        return Err(Cff1Error::InvalidCff);
    }
    let data_len = offsets
        .last()
        .and_then(|value| value.checked_sub(1))
        .ok_or(Cff1Error::InvalidCff)?;
    let end = data_start
        .checked_add(data_len)
        .ok_or(Cff1Error::InvalidCff)?;
    if end > bytes.len() {
        return Err(Cff1Error::InvalidCff);
    }
    let mut objects = Vec::new();
    objects
        .try_reserve_exact(count)
        .map_err(|_| Cff1Error::InvalidCff)?;
    for pair in offsets.windows(2) {
        let start = data_start
            .checked_add(pair[0] - 1)
            .ok_or(Cff1Error::InvalidCff)?;
        let finish = data_start
            .checked_add(pair[1] - 1)
            .ok_or(Cff1Error::InvalidCff)?;
        let source = bytes.get(start..finish).ok_or(Cff1Error::InvalidCff)?;
        let mut object = Vec::new();
        object
            .try_reserve_exact(source.len())
            .map_err(|_| Cff1Error::InvalidCff)?;
        object.extend_from_slice(source);
        objects.push(object);
    }
    Ok(CffIndex {
        objects,
        start: offset,
        end,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DictOperand {
    Integer(i32),
    Real(String),
}

#[derive(Clone, Debug)]
struct DictEntry {
    operator: u16,
    operands: Vec<DictOperand>,
}

fn parse_dict(bytes: &[u8]) -> Result<Vec<DictEntry>, Cff1Error> {
    let mut entries = Vec::new();
    let mut operands = Vec::new();
    let mut seen = BTreeSet::new();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte >= 28 || byte == 30 {
            let (operand, consumed) = parse_dict_operand(bytes, cursor)?;
            operands.push(operand);
            cursor = cursor.checked_add(consumed).ok_or(Cff1Error::InvalidCff)?;
            continue;
        }
        let (operator, consumed) = if byte == 12 {
            let escaped = *bytes.get(cursor + 1).ok_or(Cff1Error::InvalidCff)?;
            (0x0C00 | u16::from(escaped), 2)
        } else if byte <= 21 {
            (u16::from(byte), 1)
        } else {
            return Err(Cff1Error::InvalidCff);
        };
        if !seen.insert(operator) {
            return Err(Cff1Error::InvalidCff);
        }
        entries.push(DictEntry {
            operator,
            operands: std::mem::take(&mut operands),
        });
        cursor = cursor.checked_add(consumed).ok_or(Cff1Error::InvalidCff)?;
    }
    if !operands.is_empty() {
        return Err(Cff1Error::InvalidCff);
    }
    Ok(entries)
}

fn parse_dict_operand(bytes: &[u8], offset: usize) -> Result<(DictOperand, usize), Cff1Error> {
    let first = *bytes.get(offset).ok_or(Cff1Error::InvalidCff)?;
    match first {
        28 => Ok((
            DictOperand::Integer(i32::from(read_i16(
                bytes,
                offset + 1,
                Cff1Error::InvalidCff,
            )?)),
            3,
        )),
        29 => Ok((
            DictOperand::Integer(read_i32(bytes, offset + 1, Cff1Error::InvalidCff)?),
            5,
        )),
        30 => parse_dict_real(bytes, offset),
        32..=246 => Ok((DictOperand::Integer(i32::from(first) - 139), 1)),
        247..=250 => {
            let second = i32::from(*bytes.get(offset + 1).ok_or(Cff1Error::InvalidCff)?);
            Ok((
                DictOperand::Integer((i32::from(first) - 247) * 256 + second + 108),
                2,
            ))
        }
        251..=254 => {
            let second = i32::from(*bytes.get(offset + 1).ok_or(Cff1Error::InvalidCff)?);
            Ok((
                DictOperand::Integer(-((i32::from(first) - 251) * 256) - second - 108),
                2,
            ))
        }
        _ => Err(Cff1Error::InvalidCff),
    }
}

fn parse_dict_real(bytes: &[u8], offset: usize) -> Result<(DictOperand, usize), Cff1Error> {
    let mut output = String::new();
    let mut cursor = offset + 1;
    let mut finished = false;
    while !finished {
        let byte = *bytes.get(cursor).ok_or(Cff1Error::InvalidCff)?;
        cursor = cursor.checked_add(1).ok_or(Cff1Error::InvalidCff)?;
        for nibble in [byte >> 4, byte & 0x0F] {
            match nibble {
                0..=9 => output.push(char::from(b'0' + nibble)),
                0xA => output.push('.'),
                0xB => output.push('E'),
                0xC => output.push_str("E-"),
                0xE => output.push('-'),
                0xF => {
                    finished = true;
                    break;
                }
                _ => return Err(Cff1Error::InvalidCff),
            }
        }
    }
    if output.is_empty() || output.len() > 64 || normalize_dict_decimal(&output).is_none() {
        return Err(Cff1Error::InvalidCff);
    }
    Ok((DictOperand::Real(output), cursor - offset))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NormalizedDictDecimal {
    negative: bool,
    digits: String,
    exponent: i32,
}

/// Normalize a CFF real as `sign * digits * 10^exponent` using decimal-only
/// checked arithmetic. This intentionally never routes policy through binary
/// floating point or libm.
fn normalize_dict_decimal(value: &str) -> Option<NormalizedDictDecimal> {
    let bytes = value.as_bytes();
    let (negative, unsigned) = match bytes.first() {
        Some(b'-') => (true, &bytes[1..]),
        _ => (false, bytes),
    };
    if unsigned.is_empty() {
        return None;
    }
    let exponent_marker = unsigned.iter().position(|byte| *byte == b'E');
    if exponent_marker.is_some_and(|first| unsigned[first + 1..].contains(&b'E')) {
        return None;
    }
    let (mantissa, exponent_bytes) = match exponent_marker {
        Some(index) => (&unsigned[..index], Some(&unsigned[index + 1..])),
        None => (unsigned, None),
    };
    let explicit_exponent = match exponent_bytes {
        None => 0i32,
        Some(bytes) => {
            let (negative, digits) = match bytes.first() {
                Some(b'-') => (true, &bytes[1..]),
                _ => (false, bytes),
            };
            if digits.is_empty() || digits.iter().any(|byte| !byte.is_ascii_digit()) {
                return None;
            }
            let value = digits.iter().try_fold(0i32, |value, byte| {
                value.checked_mul(10)?.checked_add(i32::from(*byte - b'0'))
            })?;
            if negative {
                value.checked_neg()?
            } else {
                value
            }
        }
    };
    let decimal = mantissa.iter().position(|byte| *byte == b'.');
    if decimal.is_some_and(|first| mantissa[first + 1..].contains(&b'.')) {
        return None;
    }
    if mantissa.is_empty()
        || mantissa
            .iter()
            .any(|byte| *byte != b'.' && !byte.is_ascii_digit())
        || !mantissa.iter().any(u8::is_ascii_digit)
    {
        return None;
    }
    let fractional_digits = decimal.map(|index| mantissa.len() - index - 1).unwrap_or(0);
    let mut digits = mantissa
        .iter()
        .filter(|byte| **byte != b'.')
        .copied()
        .collect::<Vec<_>>();
    let first_nonzero = digits.iter().position(|byte| *byte != b'0');
    let Some(first_nonzero) = first_nonzero else {
        return Some(NormalizedDictDecimal {
            negative: false,
            digits: "0".to_owned(),
            exponent: 0,
        });
    };
    digits.drain(..first_nonzero);
    let mut exponent = explicit_exponent.checked_sub(i32::try_from(fractional_digits).ok()?)?;
    while digits.last() == Some(&b'0') {
        digits.pop();
        exponent = exponent.checked_add(1)?;
    }
    Some(NormalizedDictDecimal {
        negative,
        digits: String::from_utf8(digits).ok()?,
        exponent,
    })
}

fn normalized_dict_operand(operand: &DictOperand) -> Option<NormalizedDictDecimal> {
    match operand {
        DictOperand::Integer(value) => normalize_dict_decimal(&value.to_string()),
        DictOperand::Real(value) => normalize_dict_decimal(value),
    }
}

fn dict_operand_equals_integer(operand: &DictOperand, expected: i32) -> bool {
    normalized_dict_operand(operand) == normalize_dict_decimal(&expected.to_string())
}

fn dict_operand_equals_milli(operand: &DictOperand) -> bool {
    normalized_dict_operand(operand)
        == Some(NormalizedDictDecimal {
            negative: false,
            digits: "1".to_owned(),
            exponent: -3,
        })
}

fn dict_operand_is_number(operand: &DictOperand) -> bool {
    normalized_dict_operand(operand).is_some()
}

fn dict_operand_to_fixed(operand: &DictOperand) -> Option<i32> {
    let value = normalized_dict_operand(operand)?;
    if value.digits == "0" {
        return Some(0);
    }
    let magnitude = value.digits.bytes().try_fold(0i128, |number, byte| {
        number.checked_mul(10)?.checked_add(i128::from(byte - b'0'))
    })?;
    let scaled = if value.exponent >= 0 {
        magnitude
            .checked_mul(10i128.checked_pow(u32::try_from(value.exponent).ok()?)?)?
            .checked_mul(65_536)?
    } else {
        let denominator = 10i128.checked_pow(value.exponent.unsigned_abs())?;
        let numerator = magnitude.checked_mul(65_536)?;
        if numerator % denominator != 0 {
            return None;
        }
        numerator / denominator
    };
    let signed = if value.negative {
        scaled.checked_neg()?
    } else {
        scaled
    };
    i32::try_from(signed).ok()
}

fn dict_entry(entries: &[DictEntry], operator: u16) -> Option<&[DictOperand]> {
    entries
        .iter()
        .find(|entry| entry.operator == operator)
        .map(|entry| entry.operands.as_slice())
}

fn one_dict_integer(entries: &[DictEntry], operator: u16) -> Result<Option<i32>, Cff1Error> {
    match dict_entry(entries, operator) {
        None => Ok(None),
        Some([DictOperand::Integer(value)]) => Ok(Some(*value)),
        _ => Err(Cff1Error::InvalidCff),
    }
}

fn required_dict_offset(entries: &[DictEntry], operator: u16) -> Result<usize, Cff1Error> {
    let value = one_dict_integer(entries, operator)?.ok_or(Cff1Error::InvalidCff)?;
    usize::try_from(value).map_err(|_| Cff1Error::InvalidCff)
}

fn parse_cff(
    bytes: &[u8],
    expected_glyph_count: u16,
    expected_name: &str,
    expected_bbox: [i16; 4],
    limits: &M4EffectiveResourceLimits,
) -> Result<CffProgram, Cff1Error> {
    if bytes.get(..4).is_none()
        || bytes[0] != 1
        || bytes[1] != 0
        || bytes[2] != 4
        || !(1..=4).contains(&bytes[3])
    {
        return Err(Cff1Error::InvalidCff);
    }
    let names = parse_cff_index(bytes, 4, None)?;
    if names.objects.len() != 1 || names.objects[0].as_slice() != expected_name.as_bytes() {
        return Err(Cff1Error::InvalidCff);
    }
    let top_dicts = parse_cff_index(bytes, names.end, None)?;
    if top_dicts.objects.len() != 1 {
        return Err(Cff1Error::InvalidCff);
    }
    let strings = parse_cff_index(bytes, top_dicts.end, None)?;
    let global_subrs = parse_cff_index(
        bytes,
        strings.end,
        Some(limits.extension().get().max_cff_subroutines),
    )?;
    let top = parse_dict(&top_dicts.objects[0])?;
    validate_top_dict(&top, expected_bbox, strings.objects.len())?;
    let charset_offset = one_dict_integer(&top, 15)?.unwrap_or(0);
    let encoding_offset = one_dict_integer(&top, 16)?.unwrap_or(0);
    let charstrings_offset = required_dict_offset(&top, 17)?;
    let private = dict_entry(&top, 18).ok_or(Cff1Error::InvalidCff)?;
    let [DictOperand::Integer(private_size), DictOperand::Integer(private_offset)] = private else {
        return Err(Cff1Error::InvalidCff);
    };
    let private_size = usize::try_from(*private_size).map_err(|_| Cff1Error::InvalidCff)?;
    let private_offset = usize::try_from(*private_offset).map_err(|_| Cff1Error::InvalidCff)?;
    if private_size == 0 {
        return Err(Cff1Error::InvalidCff);
    }
    let charstrings = parse_cff_index(bytes, charstrings_offset, None)?;
    if charstrings.objects.len() != usize::from(expected_glyph_count) {
        return Err(Cff1Error::InvalidCff);
    }
    let charset = parse_charset(
        bytes,
        charset_offset,
        expected_glyph_count,
        strings.objects.len(),
    )?;
    let encoding_range = parse_encoding(
        bytes,
        encoding_offset,
        expected_glyph_count,
        strings.objects.len(),
        &charset.sids,
    )?;
    let private_end = private_offset
        .checked_add(private_size)
        .ok_or(Cff1Error::InvalidCff)?;
    let private_bytes = bytes
        .get(private_offset..private_end)
        .ok_or(Cff1Error::InvalidCff)?;
    let private_entries = parse_dict(private_bytes)?;
    validate_private_dict(&private_entries)?;
    let default_width_x = dict_fixed(&private_entries, 20)?.unwrap_or(0);
    let nominal_width_x = dict_fixed(&private_entries, 21)?.unwrap_or(0);
    let (local_subrs, local_range) = match one_dict_integer(&private_entries, 19)? {
        Some(relative) => {
            let offset = private_offset
                .checked_add(usize::try_from(relative).map_err(|_| Cff1Error::InvalidCff)?)
                .ok_or(Cff1Error::InvalidCff)?;
            if offset < private_end {
                return Err(Cff1Error::InvalidCff);
            }
            let remaining = limits
                .extension()
                .get()
                .max_cff_subroutines
                .checked_sub(
                    u32::try_from(global_subrs.objects.len())
                        .map_err(|_| Cff1Error::SubroutineLimit)?,
                )
                .ok_or(Cff1Error::SubroutineLimit)?;
            let index = parse_cff_index(bytes, offset, Some(remaining))?;
            let range = Some((index.start, index.end));
            (index.objects, range)
        }
        None => (Vec::new(), None),
    };
    let mut ranges = vec![
        (0usize, global_subrs.end),
        (charstrings.start, charstrings.end),
        (private_offset, private_end),
    ];
    if let Some(range) = charset.range {
        ranges.push(range);
    }
    if let Some(range) = encoding_range {
        ranges.push(range);
    }
    if let Some(range) = local_range {
        ranges.push(range);
    }
    ranges.sort_unstable();
    let mut cursor = 0usize;
    for (start, end) in ranges {
        if start != cursor || end <= start {
            return Err(Cff1Error::InvalidCff);
        }
        cursor = end;
    }
    if cursor != bytes.len() {
        return Err(Cff1Error::InvalidCff);
    }
    Ok(CffProgram {
        charstrings: charstrings.objects,
        global_subrs: global_subrs.objects,
        local_subrs,
        default_width_x,
        nominal_width_x,
    })
}

fn validate_top_dict(
    entries: &[DictEntry],
    expected_bbox: [i16; 4],
    custom_string_count: usize,
) -> Result<(), Cff1Error> {
    const ALLOWED: &[u16] = &[
        0, 1, 2, 3, 4, 5, 13, 14, 15, 16, 17, 18, 0x0C00, 0x0C01, 0x0C02, 0x0C03, 0x0C04, 0x0C05,
        0x0C06, 0x0C07, 0x0C08,
    ];
    if entries
        .iter()
        .any(|entry| !ALLOWED.contains(&entry.operator))
    {
        return Err(Cff1Error::InvalidCff);
    }
    let bbox = dict_entry(entries, 5).ok_or(Cff1Error::InvalidCff)?;
    if bbox.len() != 4
        || bbox
            .iter()
            .zip(expected_bbox)
            .any(|(operand, expected)| !dict_operand_equals_integer(operand, i32::from(expected)))
    {
        return Err(Cff1Error::InvalidCff);
    }
    if one_dict_integer(entries, 0x0C06)? != Some(2)
        || dict_entry(entries, 0x0C05)
            .is_some_and(|values| values.len() != 1 || !dict_operand_equals_integer(&values[0], 0))
        || dict_entry(entries, 0x0C08)
            .is_some_and(|values| values.len() != 1 || !dict_operand_equals_integer(&values[0], 0))
    {
        return Err(Cff1Error::InvalidCff);
    }
    if let Some(matrix) = dict_entry(entries, 0x0C07) {
        if matrix.len() != 6
            || !dict_operand_equals_milli(&matrix[0])
            || !dict_operand_equals_integer(&matrix[1], 0)
            || !dict_operand_equals_integer(&matrix[2], 0)
            || !dict_operand_equals_milli(&matrix[3])
            || !dict_operand_equals_integer(&matrix[4], 0)
            || !dict_operand_equals_integer(&matrix[5], 0)
        {
            return Err(Cff1Error::InvalidCff);
        }
    }
    for operator in [0u16, 1, 2, 3, 4, 0x0C00] {
        if let Some(values) = dict_entry(entries, operator) {
            let [DictOperand::Integer(sid)] = values else {
                return Err(Cff1Error::InvalidCff);
            };
            let sid = u16::try_from(*sid).map_err(|_| Cff1Error::InvalidCff)?;
            if sid >= 391 && usize::from(sid - 391) >= custom_string_count {
                return Err(Cff1Error::InvalidCff);
            }
        }
    }
    if let Some(values) = dict_entry(entries, 13) {
        let [DictOperand::Integer(value)] = values else {
            return Err(Cff1Error::InvalidCff);
        };
        if !(0..=16_777_215).contains(value) {
            return Err(Cff1Error::InvalidCff);
        }
    }
    if let Some(values) = dict_entry(entries, 14) {
        if values.is_empty()
            || values
                .iter()
                .any(|value| !matches!(value, DictOperand::Integer(number) if *number >= 0))
        {
            return Err(Cff1Error::InvalidCff);
        }
    }
    for operator in [15u16, 16] {
        if dict_entry(entries, operator)
            .is_some_and(|values| !matches!(values, [DictOperand::Integer(value)] if *value >= 0))
        {
            return Err(Cff1Error::InvalidCff);
        }
    }
    if !matches!(
        dict_entry(entries, 17),
        Some([DictOperand::Integer(value)]) if *value >= 0
    ) {
        return Err(Cff1Error::InvalidCff);
    }
    if !matches!(
        dict_entry(entries, 18),
        Some([DictOperand::Integer(size), DictOperand::Integer(offset)])
            if *size > 0 && *offset >= 0
    ) {
        return Err(Cff1Error::InvalidCff);
    }
    if let Some(values) = dict_entry(entries, 0x0C01) {
        if !matches!(values, [DictOperand::Integer(value)] if matches!(*value, 0 | 1)) {
            return Err(Cff1Error::InvalidCff);
        }
    }
    for operator in [0x0C02u16, 0x0C03, 0x0C04] {
        if dict_entry(entries, operator)
            .is_some_and(|values| values.len() != 1 || !dict_operand_is_number(&values[0]))
        {
            return Err(Cff1Error::InvalidCff);
        }
    }
    if dict_entry(entries, 0x0C08)
        .is_some_and(|values| values.len() != 1 || !dict_operand_equals_integer(&values[0], 0))
    {
        return Err(Cff1Error::InvalidCff);
    }
    Ok(())
}

fn validate_private_dict(entries: &[DictEntry]) -> Result<(), Cff1Error> {
    // Blue/hint values are validated but never copied.  Operators outside the
    // CFF1 Private DICT vocabulary are rejected instead of ignored.
    const ALLOWED: &[u16] = &[
        6, 7, 8, 9, 10, 11, 19, 20, 21, 0x0C09, 0x0C0A, 0x0C0B, 0x0C0C, 0x0C0D, 0x0C0E, 0x0C11,
        0x0C12, 0x0C13,
    ];
    if entries
        .iter()
        .any(|entry| !ALLOWED.contains(&entry.operator))
    {
        return Err(Cff1Error::InvalidCff);
    }
    for (operator, maximum) in [(6, 14usize), (7, 10), (8, 14), (9, 10)] {
        if let Some(values) = dict_entry(entries, operator) {
            if values.is_empty()
                || values.len() > maximum
                || values.len() % 2 != 0
                || values.iter().any(|value| !dict_operand_is_number(value))
            {
                return Err(Cff1Error::InvalidCff);
            }
        }
    }
    for operator in [10u16, 11, 0x0C09, 0x0C0A, 0x0C0B, 0x0C12, 0x0C13] {
        if dict_entry(entries, operator)
            .is_some_and(|values| values.len() != 1 || !dict_operand_is_number(&values[0]))
        {
            return Err(Cff1Error::InvalidCff);
        }
    }
    for operator in [0x0C0C, 0x0C0D] {
        if let Some(values) = dict_entry(entries, operator) {
            if values.is_empty()
                || values.len() > 12
                || values.iter().any(|value| !dict_operand_is_number(value))
            {
                return Err(Cff1Error::InvalidCff);
            }
        }
    }
    if dict_entry(entries, 0x0C0E).is_some_and(
        |values| !matches!(values, [DictOperand::Integer(value)] if matches!(*value, 0 | 1)),
    ) || dict_entry(entries, 0x0C11).is_some_and(
        |values| !matches!(values, [DictOperand::Integer(value)] if matches!(*value, 0 | 1)),
    ) || dict_entry(entries, 19)
        .is_some_and(|values| !matches!(values, [DictOperand::Integer(value)] if *value >= 0))
    {
        return Err(Cff1Error::InvalidCff);
    }
    let _ = dict_fixed(entries, 20)?;
    let _ = dict_fixed(entries, 21)?;
    Ok(())
}

fn dict_fixed(entries: &[DictEntry], operator: u16) -> Result<Option<i32>, Cff1Error> {
    match dict_entry(entries, operator) {
        None => Ok(None),
        Some([value]) => dict_operand_to_fixed(value)
            .map(Some)
            .ok_or(Cff1Error::InvalidCff),
        _ => Err(Cff1Error::InvalidCff),
    }
}

struct ParsedCharset {
    range: Option<(usize, usize)>,
    sids: BTreeSet<u16>,
}

fn parse_charset(
    bytes: &[u8],
    offset: i32,
    glyph_count: u16,
    custom_string_count: usize,
) -> Result<ParsedCharset, Cff1Error> {
    let numeric_offset = usize::try_from(offset).map_err(|_| Cff1Error::InvalidCff)?;
    let typed = read_fonts::tables::postscript::Charset::new(
        FontData::new(bytes),
        numeric_offset,
        u32::from(glyph_count),
    )
    .map_err(|_| Cff1Error::InvalidCff)?;
    let mut typed_sids = BTreeSet::new();
    let mut typed_count = 0usize;
    for (expected_gid, (gid, sid)) in typed.iter().enumerate() {
        if gid.to_u32() != u32::try_from(expected_gid).map_err(|_| Cff1Error::InvalidCff)?
            || !typed_sids.insert(sid.to_u16())
        {
            return Err(Cff1Error::InvalidCff);
        }
        typed_count += 1;
    }
    if typed_count != usize::from(glyph_count) {
        return Err(Cff1Error::InvalidCff);
    }
    if matches!(offset, 0..=2) {
        let capacity = match offset {
            0 => 229,
            1 => 166,
            2 => 87,
            _ => unreachable!(),
        };
        if usize::from(glyph_count) > capacity {
            return Err(Cff1Error::InvalidCff);
        }
        return Ok(ParsedCharset {
            range: None,
            sids: typed_sids,
        });
    }
    let start = usize::try_from(offset).map_err(|_| Cff1Error::InvalidCff)?;
    let format = *bytes.get(start).ok_or(Cff1Error::InvalidCff)?;
    let mut cursor = start + 1;
    let mut remaining = usize::from(glyph_count) - 1;
    let mut seen = BTreeSet::new();
    match format {
        0 => {
            for _ in 0..remaining {
                let sid = read_u16(bytes, cursor, Cff1Error::InvalidCff)?;
                validate_sid(sid, custom_string_count, &mut seen)?;
                cursor += 2;
            }
        }
        1 | 2 => {
            while remaining > 0 {
                let first = read_u16(bytes, cursor, Cff1Error::InvalidCff)?;
                cursor += 2;
                let additional = if format == 1 {
                    usize::from(*bytes.get(cursor).ok_or(Cff1Error::InvalidCff)?)
                } else {
                    usize::from(read_u16(bytes, cursor, Cff1Error::InvalidCff)?)
                };
                cursor += if format == 1 { 1 } else { 2 };
                let range_count = additional.checked_add(1).ok_or(Cff1Error::InvalidCff)?;
                if range_count > remaining {
                    return Err(Cff1Error::InvalidCff);
                }
                for delta in 0..range_count {
                    let sid = u16::try_from(
                        usize::from(first)
                            .checked_add(delta)
                            .ok_or(Cff1Error::InvalidCff)?,
                    )
                    .map_err(|_| Cff1Error::InvalidCff)?;
                    validate_sid(sid, custom_string_count, &mut seen)?;
                }
                remaining -= range_count;
            }
        }
        _ => return Err(Cff1Error::InvalidCff),
    }
    if seen.iter().any(|sid| !typed_sids.contains(sid)) || seen.len() + 1 != typed_sids.len() {
        return Err(Cff1Error::InvalidCff);
    }
    Ok(ParsedCharset {
        range: Some((start, cursor)),
        sids: typed_sids,
    })
}

fn validate_sid(
    sid: u16,
    custom_string_count: usize,
    seen: &mut BTreeSet<u16>,
) -> Result<(), Cff1Error> {
    if !seen.insert(sid) || (sid >= 391 && usize::from(sid - 391) >= custom_string_count) {
        return Err(Cff1Error::InvalidCff);
    }
    Ok(())
}

fn parse_encoding(
    bytes: &[u8],
    offset: i32,
    glyph_count: u16,
    custom_string_count: usize,
    charset_sids: &BTreeSet<u16>,
) -> Result<Option<(usize, usize)>, Cff1Error> {
    if matches!(offset, 0 | 1) {
        return Ok(None);
    }
    let start = usize::try_from(offset).map_err(|_| Cff1Error::InvalidCff)?;
    let format_byte = *bytes.get(start).ok_or(Cff1Error::InvalidCff)?;
    let format = format_byte & 0x7F;
    let supplements = format_byte & 0x80 != 0;
    let mut cursor = start + 1;
    let mut encoded_glyphs = 1usize;
    let mut codes = BTreeSet::new();
    match format {
        0 => {
            let count = usize::from(*bytes.get(cursor).ok_or(Cff1Error::InvalidCff)?);
            cursor += 1;
            for code in bytes
                .get(cursor..cursor + count)
                .ok_or(Cff1Error::InvalidCff)?
            {
                if !codes.insert(*code) {
                    return Err(Cff1Error::InvalidCff);
                }
            }
            cursor += count;
            encoded_glyphs = encoded_glyphs
                .checked_add(count)
                .ok_or(Cff1Error::InvalidCff)?;
        }
        1 => {
            let range_count = usize::from(*bytes.get(cursor).ok_or(Cff1Error::InvalidCff)?);
            cursor += 1;
            for _ in 0..range_count {
                let first = *bytes.get(cursor).ok_or(Cff1Error::InvalidCff)?;
                let additional = *bytes.get(cursor + 1).ok_or(Cff1Error::InvalidCff)?;
                cursor += 2;
                for delta in 0..=additional {
                    let code = first.checked_add(delta).ok_or(Cff1Error::InvalidCff)?;
                    if !codes.insert(code) {
                        return Err(Cff1Error::InvalidCff);
                    }
                }
                encoded_glyphs = encoded_glyphs
                    .checked_add(usize::from(additional) + 1)
                    .ok_or(Cff1Error::InvalidCff)?;
            }
        }
        _ => return Err(Cff1Error::InvalidCff),
    }
    if encoded_glyphs > usize::from(glyph_count) {
        return Err(Cff1Error::InvalidCff);
    }
    if supplements {
        let count = usize::from(*bytes.get(cursor).ok_or(Cff1Error::InvalidCff)?);
        cursor += 1;
        for _ in 0..count {
            let code = *bytes.get(cursor).ok_or(Cff1Error::InvalidCff)?;
            let sid = read_u16(bytes, cursor + 1, Cff1Error::InvalidCff)?;
            if !codes.insert(code)
                || !charset_sids.contains(&sid)
                || (sid >= 391 && usize::from(sid - 391) >= custom_string_count)
            {
                return Err(Cff1Error::InvalidCff);
            }
            cursor = cursor.checked_add(3).ok_or(Cff1Error::InvalidCff)?;
        }
    }
    Ok(Some((start, cursor)))
}

#[derive(Clone, Copy, Debug)]
enum ProgramKind {
    Glyph(u16),
    Local(usize),
    Global(usize),
}

#[derive(Clone, Copy, Debug)]
struct CallFrame {
    kind: ProgramKind,
    position: usize,
}

struct Type2State {
    stack: Vec<i32>,
    frames: Vec<CallFrame>,
    x: i32,
    y: i32,
    contour_open: bool,
    stem_count: u32,
    width_seen: bool,
    segments: Vec<OutlineSegment>,
    bbox: Option<[i32; 4]>,
}

fn evaluate_glyph(
    admission: &Cff1Admission,
    gid: u16,
    budget: &mut Cff1SubsetSession,
) -> Result<EvaluatedGlyph, Cff1Error> {
    if usize::from(gid) >= admission.program.charstrings.len() {
        return Err(Cff1Error::InvalidSelectedGlyph);
    }
    let mut stack = Vec::new();
    stack
        .try_reserve_exact(TYPE2_OPERAND_STACK_LIMIT)
        .map_err(|_| Cff1Error::InvalidCharstring)?;
    let mut frames = Vec::new();
    frames
        .try_reserve_exact(TYPE2_CALL_DEPTH_LIMIT + 1)
        .map_err(|_| Cff1Error::InvalidCharstring)?;
    frames.push(CallFrame {
        kind: ProgramKind::Glyph(gid),
        position: 0,
    });
    let mut state = Type2State {
        stack,
        frames,
        x: 0,
        y: 0,
        contour_open: false,
        stem_count: 0,
        width_seen: false,
        segments: Vec::new(),
        bbox: None,
    };
    let mut ended = false;
    while !state.frames.is_empty() {
        let frame_index = state.frames.len() - 1;
        let frame = state.frames[frame_index];
        let program = program_bytes(&admission.program, frame.kind)?;
        let byte = *program
            .get(frame.position)
            .ok_or(Cff1Error::InvalidCharstring)?;
        if byte == 28 || byte >= 32 {
            budget.charge_operation()?;
            let (value, consumed) = parse_type2_number(program, frame.position)?;
            if state.stack.len() >= TYPE2_OPERAND_STACK_LIMIT {
                return Err(Cff1Error::InvalidCharstring);
            }
            state
                .stack
                .try_reserve(1)
                .map_err(|_| Cff1Error::InvalidCharstring)?;
            state.stack.push(value);
            state.frames[frame_index].position = frame
                .position
                .checked_add(consumed)
                .ok_or(Cff1Error::InvalidCharstring)?;
            continue;
        }
        state.frames[frame_index].position = frame
            .position
            .checked_add(1)
            .ok_or(Cff1Error::InvalidCharstring)?;
        budget.charge_operation()?;
        match byte {
            1 | 3 | 18 | 23 => consume_stems(admission, &mut state, budget)?,
            4 => {
                consume_width_for_path(admission, &mut state, 1)?;
                let dy = exactly(&state.stack, 1)?[0];
                state.stack.clear();
                close_contour(&mut state, budget)?;
                state.y = add_fixed(state.y, dy)?;
                emit_current_move(&mut state, budget)?;
            }
            5 => {
                require_width_seen(&state)?;
                if state.stack.len() < 2 || state.stack.len() % 2 != 0 {
                    return Err(Cff1Error::InvalidCharstring);
                }
                let values = std::mem::take(&mut state.stack);
                for pair in values.chunks_exact(2) {
                    state.x = add_fixed(state.x, pair[0])?;
                    state.y = add_fixed(state.y, pair[1])?;
                    emit_current_line(&mut state, budget)?;
                }
            }
            6 | 7 => {
                require_width_seen(&state)?;
                if state.stack.is_empty() {
                    return Err(Cff1Error::InvalidCharstring);
                }
                let values = std::mem::take(&mut state.stack);
                let mut horizontal = byte == 6;
                for value in values {
                    if horizontal {
                        state.x = add_fixed(state.x, value)?;
                    } else {
                        state.y = add_fixed(state.y, value)?;
                    }
                    emit_current_line(&mut state, budget)?;
                    horizontal = !horizontal;
                }
            }
            8 => {
                require_width_seen(&state)?;
                if state.stack.len() < 6 || state.stack.len() % 6 != 0 {
                    return Err(Cff1Error::InvalidCharstring);
                }
                let values = std::mem::take(&mut state.stack);
                for values in values.chunks_exact(6) {
                    relative_cubic(&mut state, budget, values)?;
                }
            }
            10 | 29 => {
                let operand = state.stack.pop().ok_or(Cff1Error::InvalidCharstring)?;
                let index = subroutine_index(
                    operand,
                    if byte == 10 {
                        admission.program.local_subrs.len()
                    } else {
                        admission.program.global_subrs.len()
                    },
                )?;
                if state.frames.len() > TYPE2_CALL_DEPTH_LIMIT {
                    return Err(Cff1Error::InvalidCharstring);
                }
                budget.charge_operation()?;
                state
                    .frames
                    .try_reserve(1)
                    .map_err(|_| Cff1Error::InvalidCharstring)?;
                state.frames.push(CallFrame {
                    kind: if byte == 10 {
                        ProgramKind::Local(index)
                    } else {
                        ProgramKind::Global(index)
                    },
                    position: 0,
                });
            }
            11 => {
                if state.frames.len() <= 1 {
                    return Err(Cff1Error::InvalidCharstring);
                }
                budget.charge_operation()?;
                state.frames.pop();
            }
            12 => {
                let frame_index = state.frames.len() - 1;
                let frame = state.frames[frame_index];
                let program = program_bytes(&admission.program, frame.kind)?;
                let escaped = *program
                    .get(frame.position)
                    .ok_or(Cff1Error::InvalidCharstring)?;
                state.frames[frame_index].position = frame.position + 1;
                evaluate_flex(escaped, &mut state, budget)?;
            }
            14 => {
                consume_width_for_endchar(admission, &mut state)?;
                if !state.stack.is_empty() || state.frames.len() != 1 {
                    return Err(Cff1Error::InvalidCharstring);
                }
                budget.charge_operation()?;
                close_contour(&mut state, budget)?;
                ended = true;
                state.frames.clear();
            }
            19 | 20 => {
                consume_stems(admission, &mut state, budget)?;
                let mask_bytes = usize::try_from(state.stem_count.div_ceil(8))
                    .map_err(|_| Cff1Error::InvalidCharstring)?;
                let frame_index = state.frames.len() - 1;
                let frame = state.frames[frame_index];
                let program = program_bytes(&admission.program, frame.kind)?;
                let end = frame
                    .position
                    .checked_add(mask_bytes)
                    .ok_or(Cff1Error::InvalidCharstring)?;
                if program.get(frame.position..end).is_none() {
                    return Err(Cff1Error::InvalidCharstring);
                }
                for _ in 0..mask_bytes {
                    budget.charge_operation()?;
                }
                state.frames[frame_index].position = end;
            }
            21 => {
                consume_width_for_path(admission, &mut state, 2)?;
                let values = exactly(&state.stack, 2)?;
                let dx = values[0];
                let dy = values[1];
                state.stack.clear();
                close_contour(&mut state, budget)?;
                state.x = add_fixed(state.x, dx)?;
                state.y = add_fixed(state.y, dy)?;
                emit_current_move(&mut state, budget)?;
            }
            22 => {
                consume_width_for_path(admission, &mut state, 1)?;
                let dx = exactly(&state.stack, 1)?[0];
                state.stack.clear();
                close_contour(&mut state, budget)?;
                state.x = add_fixed(state.x, dx)?;
                emit_current_move(&mut state, budget)?;
            }
            24 => {
                require_width_seen(&state)?;
                if state.stack.len() < 8 || (state.stack.len() - 2) % 6 != 0 {
                    return Err(Cff1Error::InvalidCharstring);
                }
                let values = std::mem::take(&mut state.stack);
                let curve_end = values.len() - 2;
                for curve in values[..curve_end].chunks_exact(6) {
                    relative_cubic(&mut state, budget, curve)?;
                }
                state.x = add_fixed(state.x, values[curve_end])?;
                state.y = add_fixed(state.y, values[curve_end + 1])?;
                emit_current_line(&mut state, budget)?;
            }
            25 => {
                require_width_seen(&state)?;
                if state.stack.len() < 8 || (state.stack.len() - 6) % 2 != 0 {
                    return Err(Cff1Error::InvalidCharstring);
                }
                let values = std::mem::take(&mut state.stack);
                let curve_start = values.len() - 6;
                for line in values[..curve_start].chunks_exact(2) {
                    state.x = add_fixed(state.x, line[0])?;
                    state.y = add_fixed(state.y, line[1])?;
                    emit_current_line(&mut state, budget)?;
                }
                relative_cubic(&mut state, budget, &values[curve_start..])?;
            }
            26 => evaluate_vvcurveto(&mut state, budget)?,
            27 => evaluate_hhcurveto(&mut state, budget)?,
            30 | 31 => evaluate_alternating_curves(byte == 31, &mut state, budget)?,
            _ => return Err(Cff1Error::InvalidCharstring),
        }
    }
    if !ended {
        return Err(Cff1Error::InvalidCharstring);
    }
    Ok(EvaluatedGlyph {
        segments: state.segments,
        bbox: state.bbox,
    })
}

fn program_bytes(program: &CffProgram, kind: ProgramKind) -> Result<&[u8], Cff1Error> {
    match kind {
        ProgramKind::Glyph(gid) => program.charstrings.get(usize::from(gid)),
        ProgramKind::Local(index) => program.local_subrs.get(index),
        ProgramKind::Global(index) => program.global_subrs.get(index),
    }
    .map(Vec::as_slice)
    .ok_or(Cff1Error::InvalidCharstring)
}

fn parse_type2_number(bytes: &[u8], offset: usize) -> Result<(i32, usize), Cff1Error> {
    let first = *bytes.get(offset).ok_or(Cff1Error::InvalidCharstring)?;
    match first {
        28 => Ok((
            i32::from(read_i16(bytes, offset + 1, Cff1Error::InvalidCharstring)?)
                .checked_mul(65_536)
                .ok_or(Cff1Error::InvalidCharstring)?,
            3,
        )),
        32..=246 => Ok(((i32::from(first) - 139) * 65_536, 1)),
        247..=250 => {
            let second = i32::from(*bytes.get(offset + 1).ok_or(Cff1Error::InvalidCharstring)?);
            let value = (i32::from(first) - 247) * 256 + second + 108;
            Ok((
                value
                    .checked_mul(65_536)
                    .ok_or(Cff1Error::InvalidCharstring)?,
                2,
            ))
        }
        251..=254 => {
            let second = i32::from(*bytes.get(offset + 1).ok_or(Cff1Error::InvalidCharstring)?);
            let value = -((i32::from(first) - 251) * 256) - second - 108;
            Ok((
                value
                    .checked_mul(65_536)
                    .ok_or(Cff1Error::InvalidCharstring)?,
                2,
            ))
        }
        255 => Ok((
            read_i32(bytes, offset + 1, Cff1Error::InvalidCharstring)?,
            5,
        )),
        _ => Err(Cff1Error::InvalidCharstring),
    }
}

fn exactly(values: &[i32], count: usize) -> Result<&[i32], Cff1Error> {
    if values.len() == count {
        Ok(values)
    } else {
        Err(Cff1Error::InvalidCharstring)
    }
}

fn require_width_seen(state: &Type2State) -> Result<(), Cff1Error> {
    if state.width_seen {
        Ok(())
    } else {
        Err(Cff1Error::InvalidCharstring)
    }
}

fn validate_source_width(admission: &Cff1Admission, operand: Option<i32>) -> Result<(), Cff1Error> {
    let _width = match operand {
        Some(value) => admission
            .program
            .nominal_width_x
            .checked_add(value)
            .ok_or(Cff1Error::InvalidCharstring)?,
        None => admission.program.default_width_x,
    };
    Ok(())
}

fn consume_width_for_path(
    admission: &Cff1Admission,
    state: &mut Type2State,
    expected_arguments: usize,
) -> Result<(), Cff1Error> {
    if state.width_seen {
        if state.stack.len() != expected_arguments {
            return Err(Cff1Error::InvalidCharstring);
        }
    } else {
        let width = if state.stack.len() == expected_arguments + 1 {
            Some(state.stack.remove(0))
        } else if state.stack.len() == expected_arguments {
            None
        } else {
            return Err(Cff1Error::InvalidCharstring);
        };
        validate_source_width(admission, width)?;
        state.width_seen = true;
    }
    Ok(())
}

fn consume_width_for_endchar(
    admission: &Cff1Admission,
    state: &mut Type2State,
) -> Result<(), Cff1Error> {
    if !state.width_seen {
        let width = match state.stack.len() {
            0 => None,
            1 => Some(state.stack.remove(0)),
            4 | 5 => return Err(Cff1Error::InvalidCharstring),
            _ => return Err(Cff1Error::InvalidCharstring),
        };
        validate_source_width(admission, width)?;
        state.width_seen = true;
    }
    Ok(())
}

fn consume_stems(
    admission: &Cff1Admission,
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
) -> Result<(), Cff1Error> {
    if !state.width_seen {
        let width = if state.stack.len() % 2 == 1 {
            Some(state.stack.remove(0))
        } else {
            None
        };
        validate_source_width(admission, width)?;
        state.width_seen = true;
    }
    if state.stack.len() % 2 != 0 {
        return Err(Cff1Error::InvalidCharstring);
    }
    let stems = u32::try_from(state.stack.len() / 2).map_err(|_| Cff1Error::InvalidCharstring)?;
    let next = state
        .stem_count
        .checked_add(stems)
        .ok_or(Cff1Error::InvalidCharstring)?;
    if next > TYPE2_STEM_LIMIT {
        return Err(Cff1Error::InvalidCharstring);
    }
    for _ in 0..stems {
        budget.charge_operation()?;
    }
    state.stem_count = next;
    state.stack.clear();
    Ok(())
}

fn subroutine_index(operand: i32, count: usize) -> Result<usize, Cff1Error> {
    if operand % 65_536 != 0 {
        return Err(Cff1Error::InvalidCharstring);
    }
    let count_i32 = i32::try_from(count).map_err(|_| Cff1Error::InvalidCharstring)?;
    let bias = if count < 1_240 {
        107
    } else if count < 33_900 {
        1_131
    } else {
        32_768
    };
    let index = operand / 65_536;
    let biased = index
        .checked_add(bias)
        .ok_or(Cff1Error::InvalidCharstring)?;
    if biased < 0 || biased >= count_i32 {
        return Err(Cff1Error::InvalidCharstring);
    }
    usize::try_from(biased).map_err(|_| Cff1Error::InvalidCharstring)
}

fn add_fixed(first: i32, second: i32) -> Result<i32, Cff1Error> {
    let value = i64::from(first)
        .checked_add(i64::from(second))
        .ok_or(Cff1Error::InvalidCharstring)?;
    i32::try_from(value).map_err(|_| Cff1Error::InvalidCharstring)
}

fn update_bbox(bbox: &mut Option<[i32; 4]>, x: i32, y: i32) {
    match bbox {
        Some(bounds) => {
            bounds[0] = bounds[0].min(x);
            bounds[1] = bounds[1].min(y);
            bounds[2] = bounds[2].max(x);
            bounds[3] = bounds[3].max(y);
        }
        None => *bbox = Some([x, y, x, y]),
    }
}

fn emit_move(
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
    x: i32,
    y: i32,
) -> Result<(), Cff1Error> {
    reserve_segment(state, budget)?;
    update_bbox(&mut state.bbox, x, y);
    state.segments.push(OutlineSegment::Move(x, y));
    state.contour_open = true;
    Ok(())
}

fn emit_current_move(
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
) -> Result<(), Cff1Error> {
    let (x, y) = (state.x, state.y);
    emit_move(state, budget, x, y)
}

fn emit_line(
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
    x: i32,
    y: i32,
) -> Result<(), Cff1Error> {
    if !state.contour_open {
        return Err(Cff1Error::InvalidCharstring);
    }
    reserve_segment(state, budget)?;
    update_bbox(&mut state.bbox, x, y);
    state.segments.push(OutlineSegment::Line(x, y));
    Ok(())
}

fn emit_current_line(
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
) -> Result<(), Cff1Error> {
    let (x, y) = (state.x, state.y);
    emit_line(state, budget, x, y)
}

fn emit_cubic(
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
    points: [i32; 6],
) -> Result<(), Cff1Error> {
    if !state.contour_open {
        return Err(Cff1Error::InvalidCharstring);
    }
    reserve_segment(state, budget)?;
    for point in points.chunks_exact(2) {
        update_bbox(&mut state.bbox, point[0], point[1]);
    }
    state.segments.push(OutlineSegment::Cubic(
        points[0], points[1], points[2], points[3], points[4], points[5],
    ));
    state.x = points[4];
    state.y = points[5];
    Ok(())
}

fn close_contour(state: &mut Type2State, budget: &mut Cff1SubsetSession) -> Result<(), Cff1Error> {
    if state.contour_open {
        reserve_segment(state, budget)?;
        state.segments.push(OutlineSegment::Close);
        state.contour_open = false;
    }
    Ok(())
}

fn reserve_segment(
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
) -> Result<(), Cff1Error> {
    // The inclusive work limit wins before allocation or append. A host
    // allocation refusal stays in the same bounded outline-output domain and
    // cannot trigger an infallible Vec growth or a partial subset.
    budget.charge_segment()?;
    state
        .segments
        .try_reserve(1)
        .map_err(|_| Cff1Error::OutlineSegmentLimit)
}

fn relative_cubic(
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
    deltas: &[i32],
) -> Result<(), Cff1Error> {
    let values = exactly(deltas, 6)?;
    let x1 = add_fixed(state.x, values[0])?;
    let y1 = add_fixed(state.y, values[1])?;
    let x2 = add_fixed(x1, values[2])?;
    let y2 = add_fixed(y1, values[3])?;
    let x3 = add_fixed(x2, values[4])?;
    let y3 = add_fixed(y2, values[5])?;
    emit_cubic(state, budget, [x1, y1, x2, y2, x3, y3])
}

fn evaluate_vvcurveto(
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
) -> Result<(), Cff1Error> {
    require_width_seen(state)?;
    let values = std::mem::take(&mut state.stack);
    if values.len() < 4 || values.len() % 4 > 1 {
        return Err(Cff1Error::InvalidCharstring);
    }
    let mut index = 0usize;
    let mut first_dx = 0;
    if values.len() % 4 == 1 {
        first_dx = values[0];
        index = 1;
    }
    let mut first = true;
    while index < values.len() {
        let dx1 = if first { first_dx } else { 0 };
        let x1 = add_fixed(state.x, dx1)?;
        let y1 = add_fixed(state.y, values[index])?;
        let x2 = add_fixed(x1, values[index + 1])?;
        let y2 = add_fixed(y1, values[index + 2])?;
        let x3 = x2;
        let y3 = add_fixed(y2, values[index + 3])?;
        emit_cubic(state, budget, [x1, y1, x2, y2, x3, y3])?;
        first = false;
        index += 4;
    }
    Ok(())
}

fn evaluate_hhcurveto(
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
) -> Result<(), Cff1Error> {
    require_width_seen(state)?;
    let values = std::mem::take(&mut state.stack);
    if values.len() < 4 || values.len() % 4 > 1 {
        return Err(Cff1Error::InvalidCharstring);
    }
    let mut index = 0usize;
    let mut first_dy = 0;
    if values.len() % 4 == 1 {
        first_dy = values[0];
        index = 1;
    }
    let mut first = true;
    while index < values.len() {
        let x1 = add_fixed(state.x, values[index])?;
        let y1 = add_fixed(state.y, if first { first_dy } else { 0 })?;
        let x2 = add_fixed(x1, values[index + 1])?;
        let y2 = add_fixed(y1, values[index + 2])?;
        let x3 = add_fixed(x2, values[index + 3])?;
        let y3 = y2;
        emit_cubic(state, budget, [x1, y1, x2, y2, x3, y3])?;
        first = false;
        index += 4;
    }
    Ok(())
}

fn evaluate_alternating_curves(
    horizontal_first: bool,
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
) -> Result<(), Cff1Error> {
    require_width_seen(state)?;
    let values = std::mem::take(&mut state.stack);
    if values.len() < 4 || !matches!(values.len() % 4, 0 | 1) {
        return Err(Cff1Error::InvalidCharstring);
    }
    let mut index = 0usize;
    let mut horizontal = horizontal_first;
    while index < values.len() {
        let remaining = values.len() - index;
        if remaining < 4 {
            return Err(Cff1Error::InvalidCharstring);
        }
        let last_extra = remaining == 5;
        let (x1, y1, x2, y2, x3, y3) = if horizontal {
            let x1 = add_fixed(state.x, values[index])?;
            let y1 = state.y;
            let x2 = add_fixed(x1, values[index + 1])?;
            let y2 = add_fixed(y1, values[index + 2])?;
            let x3 = if last_extra {
                add_fixed(x2, values[index + 4])?
            } else {
                x2
            };
            let y3 = add_fixed(y2, values[index + 3])?;
            (x1, y1, x2, y2, x3, y3)
        } else {
            let x1 = state.x;
            let y1 = add_fixed(state.y, values[index])?;
            let x2 = add_fixed(x1, values[index + 1])?;
            let y2 = add_fixed(y1, values[index + 2])?;
            let x3 = add_fixed(x2, values[index + 3])?;
            let y3 = if last_extra {
                add_fixed(y2, values[index + 4])?
            } else {
                y2
            };
            (x1, y1, x2, y2, x3, y3)
        };
        emit_cubic(state, budget, [x1, y1, x2, y2, x3, y3])?;
        index += if last_extra { 5 } else { 4 };
        horizontal = !horizontal;
    }
    Ok(())
}

fn evaluate_flex(
    operator: u8,
    state: &mut Type2State,
    budget: &mut Cff1SubsetSession,
) -> Result<(), Cff1Error> {
    require_width_seen(state)?;
    let values = std::mem::take(&mut state.stack);
    let deltas = match operator {
        34 => {
            let v = exactly(&values, 7)?;
            [v[0], 0, v[1], v[2], v[3], 0, v[4], 0, v[5], -v[2], v[6], 0]
        }
        35 => {
            let v = exactly(&values, 13)?;
            [
                v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8], v[9], v[10], v[11],
            ]
        }
        36 => {
            let v = exactly(&values, 9)?;
            let final_dy = i32::try_from(-i64::from(v[1]) - i64::from(v[3]) - i64::from(v[7]))
                .map_err(|_| Cff1Error::InvalidCharstring)?;
            [
                v[0], v[1], v[2], v[3], v[4], 0, v[5], 0, v[6], v[7], v[8], final_dy,
            ]
        }
        37 => {
            let v = exactly(&values, 11)?;
            let sum_x = i64::from(v[0])
                + i64::from(v[2])
                + i64::from(v[4])
                + i64::from(v[6])
                + i64::from(v[8]);
            let sum_y = i64::from(v[1])
                + i64::from(v[3])
                + i64::from(v[5])
                + i64::from(v[7])
                + i64::from(v[9]);
            let (dx6, dy6) = if sum_x.unsigned_abs() > sum_y.unsigned_abs() {
                (
                    v[10],
                    i32::try_from(-sum_y).map_err(|_| Cff1Error::InvalidCharstring)?,
                )
            } else {
                (
                    i32::try_from(-sum_x).map_err(|_| Cff1Error::InvalidCharstring)?,
                    v[10],
                )
            };
            [
                v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8], v[9], dx6, dy6,
            ]
        }
        _ => return Err(Cff1Error::InvalidCharstring),
    };
    budget.charge_operation()?;
    relative_cubic(state, budget, &deltas[..6])?;
    budget.charge_operation()?;
    relative_cubic(state, budget, &deltas[6..])
}

fn build_glyph_closure(
    admission: &Cff1Admission,
    font_face_id: FontFaceId,
    font_instance_id: FontInstanceId,
    selected: &BTreeSet<OriginalGlyphId>,
    max_cids_per_font: u16,
) -> Result<Cff1GlyphClosure, Cff1Error> {
    let maximum = usize::from(max_cids_per_font.min(65_534));
    let nonzero_count = selected.iter().filter(|gid| gid.get() != 0).count();
    if nonzero_count > maximum {
        return Err(Cff1Error::SelectedGlyphLimit);
    }
    if selected
        .iter()
        .any(|gid| u32::from(gid.get()) >= admission.glyph_count)
    {
        return Err(Cff1Error::InvalidSelectedGlyph);
    }
    let mut source_gids = Vec::new();
    source_gids
        .try_reserve_exact(nonzero_count + 1)
        .map_err(|_| Cff1Error::SelectedGlyphLimit)?;
    source_gids.push(OriginalGlyphId::new(0));
    source_gids.extend(selected.iter().copied().filter(|gid| gid.get() != 0));
    if source_gids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(Cff1Error::InvalidGlyphClosure);
    }
    let mut canonical_jcs = String::from("{\"algorithm\":");
    push_jcs_string(&mut canonical_jcs, CFF1_GLYPH_CLOSURE_ID);
    canonical_jcs.push_str(",\"font_face_id\":");
    canonical_jcs.push_str(&font_face_id.get().to_string());
    canonical_jcs.push_str(",\"font_instance_id\":");
    canonical_jcs.push_str(&font_instance_id.get().to_string());
    canonical_jcs.push_str(",\"source_gids\":[");
    for (index, gid) in source_gids.iter().enumerate() {
        if index > 0 {
            canonical_jcs.push(',');
        }
        canonical_jcs.push_str(&gid.get().to_string());
    }
    canonical_jcs.push_str("],\"source_sha256\":");
    push_hash(&mut canonical_jcs, admission.source_sha256);
    canonical_jcs.push('}');
    Ok(Cff1GlyphClosure {
        font_face_id,
        font_instance_id,
        source_sha256: admission.source_sha256,
        source_gids,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn write_subset(
    admission: &Cff1Admission,
    closure: Cff1GlyphClosure,
    evaluated: &BTreeMap<(FontFaceId, [u8; 32], u16), EvaluatedGlyph>,
    max_subset_bytes: u64,
) -> Result<Cff1Subset, Cff1Error> {
    let subset_name = subset_postscript_name(closure.font_instance_id)?;
    let mut original_to_subset = BTreeMap::new();
    let mut original_widths = BTreeMap::new();
    let mut charstrings = Vec::new();
    let mut glyph_bboxes = Vec::new();
    charstrings
        .try_reserve_exact(closure.source_gids.len())
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    glyph_bboxes
        .try_reserve_exact(closure.source_gids.len())
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    let mut global_bbox: Option<[i16; 4]> = None;
    for (subset_gid, original_gid) in closure.source_gids.iter().enumerate() {
        let subset_gid = u16::try_from(subset_gid).map_err(|_| Cff1Error::SelectedGlyphLimit)?;
        let outline = evaluated
            .get(&(
                closure.font_face_id,
                admission.source_sha256,
                original_gid.get(),
            ))
            .ok_or(Cff1Error::InvalidGlyphClosure)?;
        let advance = *admission
            .advances
            .get(usize::from(original_gid.get()))
            .ok_or(Cff1Error::InvalidSubset)?;
        let bbox = outline
            .bbox
            .map(outward_i16_bbox)
            .transpose()?
            .unwrap_or([0, 0, 0, 0]);
        if outline.bbox.is_some() {
            global_bbox = Some(match global_bbox {
                Some(bounds) => [
                    bounds[0].min(bbox[0]),
                    bounds[1].min(bbox[1]),
                    bounds[2].max(bbox[2]),
                    bounds[3].max(bbox[3]),
                ],
                None => bbox,
            });
        }
        glyph_bboxes.push(bbox);
        charstrings.push(canonical_charstring(advance, &outline.segments)?);
        original_to_subset.insert(*original_gid, SubsetGlyphId::new(subset_gid));
        original_widths.insert(*original_gid, advance);
    }
    let global_bbox = global_bbox.ok_or(Cff1Error::InvalidSubset)?;
    if global_bbox[0] >= global_bbox[2] || global_bbox[1] >= global_bbox[3] {
        return Err(Cff1Error::InvalidSubset);
    }

    let cff = build_cid_cff(&subset_name, global_bbox, &charstrings)?;
    let cmap = build_subset_cmap(&admission.cmap, &original_to_subset)?;
    let head = build_subset_head(&admission.head, global_bbox)?;
    let (hhea, hmtx) =
        build_subset_horizontal_metrics(admission, closure.source_gids(), &glyph_bboxes)?;
    let maxp = build_subset_maxp(closure.source_gids.len())?;
    let name = build_subset_name_table(&admission.family, &admission.subfamily, &subset_name)?;
    let tables = vec![
        RewriteTable {
            tag: *b"CFF ",
            bytes: cff,
        },
        RewriteTable {
            tag: *b"OS/2",
            bytes: admission.os2.clone(),
        },
        RewriteTable {
            tag: *b"cmap",
            bytes: cmap,
        },
        RewriteTable {
            tag: *b"head",
            bytes: head,
        },
        RewriteTable {
            tag: *b"hhea",
            bytes: hhea,
        },
        RewriteTable {
            tag: *b"hmtx",
            bytes: hmtx,
        },
        RewriteTable {
            tag: *b"maxp",
            bytes: maxp,
        },
        RewriteTable {
            tag: *b"name",
            bytes: name,
        },
        RewriteTable {
            tag: *b"post",
            bytes: admission.post.clone(),
        },
    ];
    let estimated = sfnt_output_size(&tables)?;
    if estimated > max_subset_bytes {
        return Err(Cff1Error::SubsetByteLimit);
    }
    let bytes = rebuild_sfnt(tables)?;
    if u64::try_from(bytes.len()).map_err(|_| Cff1Error::SubsetByteLimit)? != estimated {
        return Err(Cff1Error::InvalidSubset);
    }
    let subset_sha256 = sha256(&bytes);
    let metrics = subset_pdf_metrics(admission, global_bbox)?;
    let mut canonical_jcs = String::from("{\"algorithm\":");
    push_jcs_string(&mut canonical_jcs, CFF1_SUBSET_ID);
    canonical_jcs.push_str(",\"byte_length\":");
    canonical_jcs.push_str(&bytes.len().to_string());
    canonical_jcs.push_str(",\"closure_fingerprint\":");
    push_hash(&mut canonical_jcs, closure.fingerprint());
    canonical_jcs.push_str(",\"font_face_id\":");
    canonical_jcs.push_str(&closure.font_face_id.get().to_string());
    canonical_jcs.push_str(",\"font_instance_id\":");
    canonical_jcs.push_str(&closure.font_instance_id.get().to_string());
    canonical_jcs.push_str(",\"postscript_name\":");
    push_jcs_string(&mut canonical_jcs, &subset_name);
    canonical_jcs.push_str(",\"sha256\":");
    push_hash(&mut canonical_jcs, subset_sha256);
    canonical_jcs.push_str(",\"source_sha256\":");
    push_hash(&mut canonical_jcs, admission.source_sha256);
    canonical_jcs.push('}');
    Ok(Cff1Subset {
        bytes,
        sha256: subset_sha256,
        postscript_name: subset_name,
        original_to_subset,
        original_widths,
        metrics,
        closure,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn subset_postscript_name(font_instance_id: FontInstanceId) -> Result<String, Cff1Error> {
    const RADIX: u32 = 26;
    const LENGTH: usize = 6;
    const SPACE: u32 = RADIX.pow(LENGTH as u32);
    let mut value = font_instance_id.get();
    if value >= SPACE {
        return Err(Cff1Error::InvalidSubset);
    }
    let mut tag = [b'A'; LENGTH];
    for byte in tag.iter_mut().rev() {
        *byte = b'A' + u8::try_from(value % RADIX).map_err(|_| Cff1Error::InvalidSubset)?;
        value /= RADIX;
    }
    let mut output = String::with_capacity(LENGTH + 8);
    output.extend(tag.into_iter().map(char::from));
    output.push_str("+Typaxis");
    Ok(output)
}

fn outward_i16_bbox(bounds: [i32; 4]) -> Result<[i16; 4], Cff1Error> {
    let floor = |value: i32| i32::div_euclid(value, 65_536);
    let ceil = |value: i32| {
        let quotient = i32::div_euclid(value, 65_536);
        if i32::rem_euclid(value, 65_536) == 0 {
            quotient
        } else {
            quotient + 1
        }
    };
    Ok([
        i16::try_from(floor(bounds[0])).map_err(|_| Cff1Error::InvalidSubset)?,
        i16::try_from(floor(bounds[1])).map_err(|_| Cff1Error::InvalidSubset)?,
        i16::try_from(ceil(bounds[2])).map_err(|_| Cff1Error::InvalidSubset)?,
        i16::try_from(ceil(bounds[3])).map_err(|_| Cff1Error::InvalidSubset)?,
    ])
}

fn canonical_charstring(advance: u16, segments: &[OutlineSegment]) -> Result<Vec<u8>, Cff1Error> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(
            segments
                .len()
                .checked_mul(31)
                .and_then(|value| value.checked_add(6))
                .ok_or(Cff1Error::SubsetByteLimit)?,
        )
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    let width = i32::from(advance) - 32_768;
    encode_type2_number(
        width.checked_mul(65_536).ok_or(Cff1Error::InvalidSubset)?,
        &mut output,
    );
    let mut x = 0i32;
    let mut y = 0i32;
    for segment in segments {
        match *segment {
            OutlineSegment::Move(next_x, next_y) => {
                encode_type2_number(
                    next_x.checked_sub(x).ok_or(Cff1Error::InvalidSubset)?,
                    &mut output,
                );
                encode_type2_number(
                    next_y.checked_sub(y).ok_or(Cff1Error::InvalidSubset)?,
                    &mut output,
                );
                output.push(21);
                x = next_x;
                y = next_y;
            }
            OutlineSegment::Line(next_x, next_y) => {
                encode_type2_number(
                    next_x.checked_sub(x).ok_or(Cff1Error::InvalidSubset)?,
                    &mut output,
                );
                encode_type2_number(
                    next_y.checked_sub(y).ok_or(Cff1Error::InvalidSubset)?,
                    &mut output,
                );
                output.push(5);
                x = next_x;
                y = next_y;
            }
            OutlineSegment::Cubic(x1, y1, x2, y2, x3, y3) => {
                for value in [
                    x1.checked_sub(x),
                    y1.checked_sub(y),
                    x2.checked_sub(x1),
                    y2.checked_sub(y1),
                    x3.checked_sub(x2),
                    y3.checked_sub(y2),
                ] {
                    encode_type2_number(value.ok_or(Cff1Error::InvalidSubset)?, &mut output);
                }
                output.push(8);
                x = x3;
                y = y3;
            }
            OutlineSegment::Close => {}
        }
    }
    output.push(14);
    Ok(output)
}

fn encode_type2_number(raw: i32, output: &mut Vec<u8>) {
    if raw % 65_536 != 0 {
        output.push(255);
        output.extend_from_slice(&raw.to_be_bytes());
        return;
    }
    let value = raw / 65_536;
    if (-107..=107).contains(&value) {
        output.push(u8::try_from(value + 139).expect("bounded Type2 single-byte integer"));
    } else if (108..=1_131).contains(&value) {
        let adjusted = value - 108;
        output.push(u8::try_from(adjusted / 256 + 247).expect("bounded Type2 positive integer"));
        output.push(u8::try_from(adjusted % 256).expect("bounded Type2 positive integer"));
    } else if (-1_131..=-108).contains(&value) {
        let adjusted = -value - 108;
        output.push(u8::try_from(adjusted / 256 + 251).expect("bounded Type2 negative integer"));
        output.push(u8::try_from(adjusted % 256).expect("bounded Type2 negative integer"));
    } else if let Ok(value) = i16::try_from(value) {
        output.push(28);
        output.extend_from_slice(&value.to_be_bytes());
    } else {
        output.push(255);
        output.extend_from_slice(&raw.to_be_bytes());
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
struct CffBuildOffsets {
    charset: usize,
    charstrings: usize,
    fd_select: usize,
    fd_array: usize,
    private: usize,
}

fn build_cid_cff(
    name: &str,
    bbox: [i16; 4],
    charstrings: &[Vec<u8>],
) -> Result<Vec<u8>, Cff1Error> {
    let name_index = encode_cff_index(&[name.as_bytes().to_vec()])?;
    let string_index = encode_cff_index(&[
        b"Adobe".to_vec(),
        b"Identity".to_vec(),
        name.as_bytes().to_vec(),
    ])?;
    let global_subrs = vec![0, 0];
    let charset_capacity = charstrings
        .len()
        .saturating_sub(1)
        .checked_mul(2)
        .and_then(|value| value.checked_add(1))
        .ok_or(Cff1Error::SubsetByteLimit)?;
    let mut charset = Vec::new();
    charset
        .try_reserve_exact(charset_capacity)
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    charset.push(0);
    for cid in 1..charstrings.len() {
        charset.extend_from_slice(
            &u16::try_from(cid)
                .map_err(|_| Cff1Error::InvalidSubset)?
                .to_be_bytes(),
        );
    }
    let charstrings_index = encode_cff_index(charstrings)?;
    let mut fd_select = Vec::new();
    fd_select
        .try_reserve_exact(
            charstrings
                .len()
                .checked_add(1)
                .ok_or(Cff1Error::SubsetByteLimit)?,
        )
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    fd_select.push(0);
    fd_select.resize(charstrings.len() + 1, 0);
    let mut private = Vec::new();
    encode_dict_integer(0, &mut private);
    private.push(20);
    encode_dict_integer(32_768, &mut private);
    private.push(21);

    let mut offsets = CffBuildOffsets::default();
    for _ in 0..32 {
        let top_dict = build_cid_top_dict(offsets, bbox, charstrings.len())?;
        let top_index = encode_cff_index(&[top_dict])?;
        let font_dict = build_font_dict(private.len(), offsets.private)?;
        let fd_array = encode_cff_index(&[font_dict])?;
        let prefix_len = 4usize
            .checked_add(name_index.len())
            .and_then(|value| value.checked_add(top_index.len()))
            .and_then(|value| value.checked_add(string_index.len()))
            .and_then(|value| value.checked_add(global_subrs.len()))
            .ok_or(Cff1Error::InvalidSubset)?;
        let next = CffBuildOffsets {
            charset: prefix_len,
            charstrings: prefix_len
                .checked_add(charset.len())
                .ok_or(Cff1Error::InvalidSubset)?,
            fd_select: prefix_len
                .checked_add(charset.len())
                .and_then(|value| value.checked_add(charstrings_index.len()))
                .ok_or(Cff1Error::InvalidSubset)?,
            fd_array: prefix_len
                .checked_add(charset.len())
                .and_then(|value| value.checked_add(charstrings_index.len()))
                .and_then(|value| value.checked_add(fd_select.len()))
                .ok_or(Cff1Error::InvalidSubset)?,
            private: prefix_len
                .checked_add(charset.len())
                .and_then(|value| value.checked_add(charstrings_index.len()))
                .and_then(|value| value.checked_add(fd_select.len()))
                .and_then(|value| value.checked_add(fd_array.len()))
                .ok_or(Cff1Error::InvalidSubset)?,
        };
        if next == offsets {
            let mut output = Vec::new();
            output.extend_from_slice(&[1, 0, 4, 4]);
            output.extend_from_slice(&name_index);
            output.extend_from_slice(&top_index);
            output.extend_from_slice(&string_index);
            output.extend_from_slice(&global_subrs);
            output.extend_from_slice(&charset);
            output.extend_from_slice(&charstrings_index);
            output.extend_from_slice(&fd_select);
            output.extend_from_slice(&fd_array);
            output.extend_from_slice(&private);
            if output.len() != offsets.private + private.len() {
                return Err(Cff1Error::InvalidSubset);
            }
            return Ok(output);
        }
        offsets = next;
    }
    Err(Cff1Error::InvalidSubset)
}

fn build_cid_top_dict(
    offsets: CffBuildOffsets,
    bbox: [i16; 4],
    glyph_count: usize,
) -> Result<Vec<u8>, Cff1Error> {
    let mut output = Vec::new();
    for value in bbox {
        encode_dict_integer(i32::from(value), &mut output);
    }
    output.push(5);
    encode_dict_integer(
        i32::try_from(offsets.charset).map_err(|_| Cff1Error::InvalidSubset)?,
        &mut output,
    );
    output.push(15);
    encode_dict_integer(
        i32::try_from(offsets.charstrings).map_err(|_| Cff1Error::InvalidSubset)?,
        &mut output,
    );
    output.push(17);
    encode_dict_integer(2, &mut output);
    output.extend_from_slice(&[12, 6]);
    for value in [391, 392, 0] {
        encode_dict_integer(value, &mut output);
    }
    output.extend_from_slice(&[12, 30]);
    encode_dict_integer(0, &mut output);
    output.extend_from_slice(&[12, 33]);
    encode_dict_integer(
        i32::try_from(glyph_count).map_err(|_| Cff1Error::InvalidSubset)?,
        &mut output,
    );
    output.extend_from_slice(&[12, 34]);
    encode_dict_integer(
        i32::try_from(offsets.fd_array).map_err(|_| Cff1Error::InvalidSubset)?,
        &mut output,
    );
    output.extend_from_slice(&[12, 36]);
    encode_dict_integer(
        i32::try_from(offsets.fd_select).map_err(|_| Cff1Error::InvalidSubset)?,
        &mut output,
    );
    output.extend_from_slice(&[12, 37]);
    Ok(output)
}

fn build_font_dict(private_size: usize, private_offset: usize) -> Result<Vec<u8>, Cff1Error> {
    let mut output = Vec::new();
    encode_dict_integer(
        i32::try_from(private_size).map_err(|_| Cff1Error::InvalidSubset)?,
        &mut output,
    );
    encode_dict_integer(
        i32::try_from(private_offset).map_err(|_| Cff1Error::InvalidSubset)?,
        &mut output,
    );
    output.push(18);
    encode_dict_integer(393, &mut output);
    output.extend_from_slice(&[12, 38]);
    Ok(output)
}

fn encode_dict_integer(value: i32, output: &mut Vec<u8>) {
    if (-107..=107).contains(&value) {
        output.push(u8::try_from(value + 139).expect("bounded DICT single-byte integer"));
    } else if (108..=1_131).contains(&value) {
        let adjusted = value - 108;
        output.push(u8::try_from(adjusted / 256 + 247).expect("bounded DICT positive integer"));
        output.push(u8::try_from(adjusted % 256).expect("bounded DICT positive integer"));
    } else if (-1_131..=-108).contains(&value) {
        let adjusted = -value - 108;
        output.push(u8::try_from(adjusted / 256 + 251).expect("bounded DICT negative integer"));
        output.push(u8::try_from(adjusted % 256).expect("bounded DICT negative integer"));
    } else if let Ok(value) = i16::try_from(value) {
        output.push(28);
        output.extend_from_slice(&value.to_be_bytes());
    } else {
        output.push(29);
        output.extend_from_slice(&value.to_be_bytes());
    }
}

fn encode_cff_index(objects: &[Vec<u8>]) -> Result<Vec<u8>, Cff1Error> {
    let count = u16::try_from(objects.len()).map_err(|_| Cff1Error::InvalidSubset)?;
    if count == 0 {
        return Ok(vec![0, 0]);
    }
    let data_len = objects.iter().try_fold(0usize, |sum, object| {
        sum.checked_add(object.len())
            .ok_or(Cff1Error::InvalidSubset)
    })?;
    let maximum_offset = data_len.checked_add(1).ok_or(Cff1Error::InvalidSubset)?;
    let off_size = if maximum_offset <= 0xFF {
        1
    } else if maximum_offset <= 0xFFFF {
        2
    } else if maximum_offset <= 0xFF_FFFF {
        3
    } else {
        4
    };
    let header_len = 3usize
        .checked_add(
            (objects.len() + 1)
                .checked_mul(off_size)
                .ok_or(Cff1Error::InvalidSubset)?,
        )
        .ok_or(Cff1Error::InvalidSubset)?;
    let total_len = header_len
        .checked_add(data_len)
        .ok_or(Cff1Error::SubsetByteLimit)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(total_len)
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    output.extend_from_slice(&count.to_be_bytes());
    output.push(u8::try_from(off_size).map_err(|_| Cff1Error::InvalidSubset)?);
    let mut offset = 1usize;
    encode_cff_offset(offset, off_size, &mut output)?;
    for object in objects {
        offset = offset
            .checked_add(object.len())
            .ok_or(Cff1Error::InvalidSubset)?;
        encode_cff_offset(offset, off_size, &mut output)?;
    }
    for object in objects {
        output.extend_from_slice(object);
    }
    Ok(output)
}

fn encode_cff_offset(value: usize, size: usize, output: &mut Vec<u8>) -> Result<(), Cff1Error> {
    let value = u32::try_from(value).map_err(|_| Cff1Error::InvalidSubset)?;
    let bytes = value.to_be_bytes();
    output.extend_from_slice(&bytes[4 - size..]);
    Ok(())
}

fn build_subset_cmap(
    source: &BTreeMap<u32, u16>,
    mapping: &BTreeMap<OriginalGlyphId, SubsetGlyphId>,
) -> Result<Vec<u8>, Cff1Error> {
    let mut selected = Vec::new();
    selected
        .try_reserve_exact(source.len())
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    for (scalar, source_gid) in source {
        if *source_gid == 0 {
            continue;
        }
        if let Some(subset_gid) = mapping.get(&OriginalGlyphId::new(*source_gid)) {
            selected.push((*scalar, u32::from(subset_gid.get())));
        }
    }
    if selected.is_empty() {
        return Err(Cff1Error::InvalidSubset);
    }
    let mut groups = Vec::<[u32; 3]>::new();
    groups
        .try_reserve_exact(selected.len())
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    for (scalar, gid) in selected {
        match groups.last_mut() {
            Some(group)
                if group[1].checked_add(1) == Some(scalar)
                    && group[2].checked_add(scalar - group[0]) == Some(gid) =>
            {
                group[1] = scalar;
            }
            _ => groups.push([scalar, scalar, gid]),
        }
    }
    let subtable_len = 16usize
        .checked_add(
            groups
                .len()
                .checked_mul(12)
                .ok_or(Cff1Error::InvalidSubset)?,
        )
        .ok_or(Cff1Error::InvalidSubset)?;
    let total_len = 12usize
        .checked_add(subtable_len)
        .ok_or(Cff1Error::InvalidSubset)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(total_len)
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    output.extend_from_slice(&0u16.to_be_bytes());
    output.extend_from_slice(&1u16.to_be_bytes());
    output.extend_from_slice(&3u16.to_be_bytes());
    output.extend_from_slice(&10u16.to_be_bytes());
    output.extend_from_slice(&12u32.to_be_bytes());
    output.extend_from_slice(&12u16.to_be_bytes());
    output.extend_from_slice(&0u16.to_be_bytes());
    output.extend_from_slice(
        &u32::try_from(subtable_len)
            .map_err(|_| Cff1Error::InvalidSubset)?
            .to_be_bytes(),
    );
    output.extend_from_slice(&0u32.to_be_bytes());
    output.extend_from_slice(
        &u32::try_from(groups.len())
            .map_err(|_| Cff1Error::InvalidSubset)?
            .to_be_bytes(),
    );
    for [start, end, start_gid] in groups {
        output.extend_from_slice(&start.to_be_bytes());
        output.extend_from_slice(&end.to_be_bytes());
        output.extend_from_slice(&start_gid.to_be_bytes());
    }
    Ok(output)
}

fn build_subset_head(source: &[u8], bbox: [i16; 4]) -> Result<Vec<u8>, Cff1Error> {
    if source.len() != 54 {
        return Err(Cff1Error::InvalidSubset);
    }
    let mut output = source.to_vec();
    output[8..12].fill(0);
    output[20..36].fill(0);
    for (index, value) in bbox.into_iter().enumerate() {
        let offset = 36 + index * 2;
        output[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }
    output[50..54].fill(0);
    Ok(output)
}

fn build_subset_horizontal_metrics(
    admission: &Cff1Admission,
    source_gids: &[OriginalGlyphId],
    bboxes: &[[i16; 4]],
) -> Result<(Vec<u8>, Vec<u8>), Cff1Error> {
    if source_gids.len() != bboxes.len() || admission.hhea.len() != 36 {
        return Err(Cff1Error::InvalidSubset);
    }
    let mut hmtx = Vec::new();
    hmtx.try_reserve_exact(
        source_gids
            .len()
            .checked_mul(4)
            .ok_or(Cff1Error::InvalidSubset)?,
    )
    .map_err(|_| Cff1Error::SubsetByteLimit)?;
    let mut advance_width_max = 0u16;
    let mut min_lsb = i32::MAX;
    let mut min_rsb = i32::MAX;
    let mut max_extent = i32::MIN;
    for (gid, bbox) in source_gids.iter().zip(bboxes) {
        let index = usize::from(gid.get());
        let advance = *admission
            .advances
            .get(index)
            .ok_or(Cff1Error::InvalidSubset)?;
        let lsb = *admission
            .left_side_bearings
            .get(index)
            .ok_or(Cff1Error::InvalidSubset)?;
        hmtx.extend_from_slice(&advance.to_be_bytes());
        hmtx.extend_from_slice(&lsb.to_be_bytes());
        let width = i32::from(bbox[2])
            .checked_sub(i32::from(bbox[0]))
            .ok_or(Cff1Error::InvalidSubset)?;
        let rsb = i32::from(advance)
            .checked_sub(i32::from(lsb))
            .and_then(|value| value.checked_sub(width))
            .ok_or(Cff1Error::InvalidSubset)?;
        let extent = i32::from(lsb)
            .checked_add(width)
            .ok_or(Cff1Error::InvalidSubset)?;
        advance_width_max = advance_width_max.max(advance);
        min_lsb = min_lsb.min(i32::from(lsb));
        min_rsb = min_rsb.min(rsb);
        max_extent = max_extent.max(extent);
    }
    let mut hhea = admission.hhea.clone();
    hhea[10..12].copy_from_slice(&advance_width_max.to_be_bytes());
    hhea[12..14].copy_from_slice(
        &i16::try_from(min_lsb)
            .map_err(|_| Cff1Error::InvalidSubset)?
            .to_be_bytes(),
    );
    hhea[14..16].copy_from_slice(
        &i16::try_from(min_rsb)
            .map_err(|_| Cff1Error::InvalidSubset)?
            .to_be_bytes(),
    );
    hhea[16..18].copy_from_slice(
        &i16::try_from(max_extent)
            .map_err(|_| Cff1Error::InvalidSubset)?
            .to_be_bytes(),
    );
    hhea[24..34].fill(0);
    hhea[34..36].copy_from_slice(
        &u16::try_from(source_gids.len())
            .map_err(|_| Cff1Error::InvalidSubset)?
            .to_be_bytes(),
    );
    Ok((hhea, hmtx))
}

fn build_subset_maxp(glyph_count: usize) -> Result<Vec<u8>, Cff1Error> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(6)
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    output.extend_from_slice(&0x0000_5000u32.to_be_bytes());
    output.extend_from_slice(
        &u16::try_from(glyph_count)
            .map_err(|_| Cff1Error::InvalidSubset)?
            .to_be_bytes(),
    );
    Ok(output)
}

fn build_subset_name_table(
    family: &str,
    subfamily: &str,
    postscript_name: &str,
) -> Result<Vec<u8>, Cff1Error> {
    let values = [
        (1u16, encode_utf16_be(family)?),
        (2u16, encode_utf16_be(subfamily)?),
        (6u16, encode_utf16_be(postscript_name)?),
    ];
    let record_bytes = values.len() * 12;
    let string_offset = 6usize
        .checked_add(record_bytes)
        .ok_or(Cff1Error::InvalidSubset)?;
    let payload_len = values.iter().try_fold(0usize, |sum, (_, bytes)| {
        sum.checked_add(bytes.len()).ok_or(Cff1Error::InvalidSubset)
    })?;
    let output_len = string_offset
        .checked_add(payload_len)
        .ok_or(Cff1Error::SubsetByteLimit)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(output_len)
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    output.extend_from_slice(&0u16.to_be_bytes());
    output.extend_from_slice(&3u16.to_be_bytes());
    output.extend_from_slice(
        &u16::try_from(string_offset)
            .map_err(|_| Cff1Error::InvalidSubset)?
            .to_be_bytes(),
    );
    let mut payload_offset = 0usize;
    for (name_id, bytes) in &values {
        output.extend_from_slice(&3u16.to_be_bytes());
        output.extend_from_slice(&10u16.to_be_bytes());
        output.extend_from_slice(&0x0409u16.to_be_bytes());
        output.extend_from_slice(&name_id.to_be_bytes());
        output.extend_from_slice(
            &u16::try_from(bytes.len())
                .map_err(|_| Cff1Error::InvalidSubset)?
                .to_be_bytes(),
        );
        output.extend_from_slice(
            &u16::try_from(payload_offset)
                .map_err(|_| Cff1Error::InvalidSubset)?
                .to_be_bytes(),
        );
        payload_offset = payload_offset
            .checked_add(bytes.len())
            .ok_or(Cff1Error::InvalidSubset)?;
    }
    for (_, bytes) in values {
        output.extend_from_slice(&bytes);
    }
    Ok(output)
}

fn encode_utf16_be(value: &str) -> Result<Vec<u8>, Cff1Error> {
    if value.chars().any(|scalar| u32::from(scalar) > 0xFFFF) {
        return Err(Cff1Error::InvalidSubset);
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(value.len().saturating_mul(2))
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    for unit in value.encode_utf16() {
        output.extend_from_slice(&unit.to_be_bytes());
    }
    Ok(output)
}

#[derive(Clone, Debug)]
struct RewriteTable {
    tag: [u8; 4],
    bytes: Vec<u8>,
}

fn sfnt_output_size(tables: &[RewriteTable]) -> Result<u64, Cff1Error> {
    let directory = 12u64
        .checked_add(
            u64::try_from(tables.len())
                .map_err(|_| Cff1Error::SubsetByteLimit)?
                .checked_mul(16)
                .ok_or(Cff1Error::SubsetByteLimit)?,
        )
        .ok_or(Cff1Error::SubsetByteLimit)?;
    tables.iter().try_fold(directory, |sum, table| {
        let len = u64::try_from(table.bytes.len()).map_err(|_| Cff1Error::SubsetByteLimit)?;
        let padded = len.checked_add(3).ok_or(Cff1Error::SubsetByteLimit)? & !3;
        sum.checked_add(padded).ok_or(Cff1Error::SubsetByteLimit)
    })
}

fn rebuild_sfnt(mut tables: Vec<RewriteTable>) -> Result<Vec<u8>, Cff1Error> {
    tables.sort_by_key(|table| table.tag);
    if tables.len() != OUTPUT_TABLES.len() || tables.iter().map(|table| table.tag).ne(OUTPUT_TABLES)
    {
        return Err(Cff1Error::InvalidSubset);
    }
    let total_len =
        usize::try_from(sfnt_output_size(&tables)?).map_err(|_| Cff1Error::SubsetByteLimit)?;
    let count = u16::try_from(tables.len()).map_err(|_| Cff1Error::InvalidSubset)?;
    let power = 1usize << (usize::BITS - 1 - tables.len().leading_zeros());
    let search_range = u16::try_from(power * 16).map_err(|_| Cff1Error::InvalidSubset)?;
    let entry_selector =
        u16::try_from(power.trailing_zeros()).map_err(|_| Cff1Error::InvalidSubset)?;
    let range_shift = count
        .checked_mul(16)
        .and_then(|value| value.checked_sub(search_range))
        .ok_or(Cff1Error::InvalidSubset)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(total_len)
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    output.resize(total_len, 0);
    output[..4].copy_from_slice(b"OTTO");
    output[4..6].copy_from_slice(&count.to_be_bytes());
    output[6..8].copy_from_slice(&search_range.to_be_bytes());
    output[8..10].copy_from_slice(&entry_selector.to_be_bytes());
    output[10..12].copy_from_slice(&range_shift.to_be_bytes());
    let mut payload_offset = 12 + tables.len() * 16;
    let mut head_adjustment = None;
    for (index, table) in tables.iter().enumerate() {
        let record = 12 + index * 16;
        output[record..record + 4].copy_from_slice(&table.tag);
        output[record + 4..record + 8].copy_from_slice(&sfnt_checksum(&table.bytes).to_be_bytes());
        output[record + 8..record + 12].copy_from_slice(
            &u32::try_from(payload_offset)
                .map_err(|_| Cff1Error::SubsetByteLimit)?
                .to_be_bytes(),
        );
        output[record + 12..record + 16].copy_from_slice(
            &u32::try_from(table.bytes.len())
                .map_err(|_| Cff1Error::SubsetByteLimit)?
                .to_be_bytes(),
        );
        let end = payload_offset
            .checked_add(table.bytes.len())
            .ok_or(Cff1Error::SubsetByteLimit)?;
        output[payload_offset..end].copy_from_slice(&table.bytes);
        if table.tag == *b"head" {
            head_adjustment = Some(payload_offset + 8);
        }
        payload_offset = end.checked_add(3).ok_or(Cff1Error::SubsetByteLimit)? & !3;
    }
    if payload_offset != output.len() {
        return Err(Cff1Error::InvalidSubset);
    }
    let adjustment_offset = head_adjustment.ok_or(Cff1Error::InvalidSubset)?;
    let adjustment = SFNT_CHECKSUM_MAGIC.wrapping_sub(sfnt_checksum(&output));
    output[adjustment_offset..adjustment_offset + 4].copy_from_slice(&adjustment.to_be_bytes());
    if sfnt_checksum(&output) != SFNT_CHECKSUM_MAGIC {
        return Err(Cff1Error::InvalidSubset);
    }
    Ok(output)
}

fn subset_pdf_metrics(
    admission: &Cff1Admission,
    bbox: [i16; 4],
) -> Result<Cff1PdfMetrics, Cff1Error> {
    let ascent = i32::from(read_i16(&admission.hhea, 4, Cff1Error::InvalidSubset)?);
    let descent = i32::from(read_i16(&admission.hhea, 6, Cff1Error::InvalidSubset)?);
    let os2_version = read_u16(&admission.os2, 0, Cff1Error::InvalidSubset)?;
    let cap_height = if os2_version >= 2 {
        i32::from(read_i16(&admission.os2, 88, Cff1Error::InvalidSubset)?)
    } else {
        ascent
    };
    if ascent <= 0 || descent >= 0 || cap_height <= 0 {
        return Err(Cff1Error::InvalidSubset);
    }
    let italic_fixed = read_i32(&admission.post, 4, Cff1Error::InvalidSubset)?;
    let fixed_pitch = read_u32(&admission.post, 12, Cff1Error::InvalidSubset)? != 0;
    let mut flags = 0x20;
    if fixed_pitch {
        flags |= 0x01;
    }
    if italic_fixed != 0 {
        flags |= 0x40;
    }
    Ok(Cff1PdfMetrics {
        ascent_1000: ascent,
        descent_1000: descent,
        cap_height_1000: cap_height,
        stem_v_1000: 80,
        italic_angle_fixed_16_16: italic_fixed,
        flags,
        bbox_1000: bbox.map(i32::from),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};

    const FIXTURE_HEX: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/cff-media/typaxis-cff-fixture.otf.hex"
    ));

    fn fixture() -> Vec<u8> {
        let digits = FIXTURE_HEX
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect::<Vec<_>>();
        digits
            .chunks_exact(2)
            .map(|pair| {
                let nibble = |byte: u8| match byte {
                    b'0'..=b'9' => byte - b'0',
                    b'a'..=b'f' => byte - b'a' + 10,
                    _ => panic!("fixture contains a non-hex byte"),
                };
                (nibble(pair[0]) << 4) | nibble(pair[1])
            })
            .collect()
    }

    fn limits() -> M4EffectiveResourceLimits {
        M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap()
    }

    fn limits_with(extension: M4ResourceLimits) -> M4EffectiveResourceLimits {
        M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
            extension,
        )
        .unwrap()
    }

    fn table_location(bytes: &[u8], tag: &[u8; 4]) -> (usize, usize, usize) {
        let count = usize::from(u16::from_be_bytes(bytes[4..6].try_into().unwrap()));
        (0..count)
            .find_map(|index| {
                let record = 12 + index * 16;
                if bytes.get(record..record + 4) == Some(tag) {
                    let offset = usize::try_from(u32::from_be_bytes(
                        bytes[record + 8..record + 12].try_into().unwrap(),
                    ))
                    .unwrap();
                    let length = usize::try_from(u32::from_be_bytes(
                        bytes[record + 12..record + 16].try_into().unwrap(),
                    ))
                    .unwrap();
                    Some((record, offset, length))
                } else {
                    None
                }
            })
            .unwrap()
    }

    fn recompute_sfnt_checksums(bytes: &mut [u8]) {
        let (_, head, head_len) = table_location(bytes, b"head");
        assert_eq!(head_len, 54);
        bytes[head + 8..head + 12].fill(0);
        let count = usize::from(u16::from_be_bytes(bytes[4..6].try_into().unwrap()));
        for index in 0..count {
            let record = 12 + index * 16;
            let offset = usize::try_from(u32::from_be_bytes(
                bytes[record + 8..record + 12].try_into().unwrap(),
            ))
            .unwrap();
            let length = usize::try_from(u32::from_be_bytes(
                bytes[record + 12..record + 16].try_into().unwrap(),
            ))
            .unwrap();
            let checksum = sfnt_checksum(&bytes[offset..offset + length]);
            bytes[record + 4..record + 8].copy_from_slice(&checksum.to_be_bytes());
        }
        let adjustment = SFNT_CHECKSUM_MAGIC.wrapping_sub(sfnt_checksum(bytes));
        bytes[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
        assert_eq!(sfnt_checksum(bytes), SFNT_CHECKSUM_MAGIC);
    }

    fn with_fs_type(value: u16) -> Vec<u8> {
        let mut bytes = fixture();
        let (_, os2, _) = table_location(&bytes, b"OS/2");
        bytes[os2 + 8..os2 + 10].copy_from_slice(&value.to_be_bytes());
        recompute_sfnt_checksums(&mut bytes);
        bytes
    }

    fn with_optional_tables(additional: Vec<RewriteTable>) -> Vec<u8> {
        let source = fixture();
        let count = usize::from(u16::from_be_bytes(source[4..6].try_into().unwrap()));
        let mut tables = Vec::with_capacity(count + additional.len());
        for index in 0..count {
            let record = 12 + index * 16;
            let tag = source[record..record + 4].try_into().unwrap();
            let offset = usize::try_from(u32::from_be_bytes(
                source[record + 8..record + 12].try_into().unwrap(),
            ))
            .unwrap();
            let length = usize::try_from(u32::from_be_bytes(
                source[record + 12..record + 16].try_into().unwrap(),
            ))
            .unwrap();
            let mut bytes = source[offset..offset + length].to_vec();
            if tag == *b"head" {
                bytes[8..12].fill(0);
            }
            tables.push(RewriteTable { tag, bytes });
        }
        tables.extend(additional);
        tables.sort_by_key(|table| table.tag);
        assert!(tables.windows(2).all(|pair| pair[0].tag < pair[1].tag));

        let total_len = usize::try_from(sfnt_output_size(&tables).unwrap()).unwrap();
        let count = u16::try_from(tables.len()).unwrap();
        let power = 1usize << (usize::BITS - 1 - tables.len().leading_zeros());
        let search_range = u16::try_from(power * 16).unwrap();
        let entry_selector = u16::try_from(power.trailing_zeros()).unwrap();
        let range_shift = count * 16 - search_range;
        let mut output = vec![0; total_len];
        output[..4].copy_from_slice(b"OTTO");
        output[4..6].copy_from_slice(&count.to_be_bytes());
        output[6..8].copy_from_slice(&search_range.to_be_bytes());
        output[8..10].copy_from_slice(&entry_selector.to_be_bytes());
        output[10..12].copy_from_slice(&range_shift.to_be_bytes());
        let mut payload = 12 + tables.len() * 16;
        let mut head_adjustment = None;
        for (index, table) in tables.iter().enumerate() {
            let record = 12 + index * 16;
            output[record..record + 4].copy_from_slice(&table.tag);
            output[record + 4..record + 8]
                .copy_from_slice(&sfnt_checksum(&table.bytes).to_be_bytes());
            output[record + 8..record + 12]
                .copy_from_slice(&u32::try_from(payload).unwrap().to_be_bytes());
            output[record + 12..record + 16]
                .copy_from_slice(&u32::try_from(table.bytes.len()).unwrap().to_be_bytes());
            let end = payload + table.bytes.len();
            output[payload..end].copy_from_slice(&table.bytes);
            if table.tag == *b"head" {
                head_adjustment = Some(payload + 8);
            }
            payload = (end + 3) & !3;
        }
        let adjustment = SFNT_CHECKSUM_MAGIC.wrapping_sub(sfnt_checksum(&output));
        let head_adjustment = head_adjustment.unwrap();
        output[head_adjustment..head_adjustment + 4].copy_from_slice(&adjustment.to_be_bytes());
        assert_eq!(sfnt_checksum(&output), SFNT_CHECKSUM_MAGIC);
        output
    }

    fn minimal_layout_table() -> Vec<u8> {
        vec![
            0, 1, 0, 0, // version 1.0
            0, 10, // ScriptList
            0, 12, // FeatureList
            0, 14, // LookupList
            0, 0, // no scripts
            0, 0, // no features
            0, 0, // no lookups
        ]
    }

    fn minimal_math_table() -> Vec<u8> {
        const CONSTANTS_LENGTH: usize = 214;
        let constants = 10usize;
        let glyph_info = constants + CONSTANTS_LENGTH;
        let variants = glyph_info + 8;
        let mut table = vec![0; variants + 10];
        table[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        table[4..6].copy_from_slice(&u16::try_from(constants).unwrap().to_be_bytes());
        table[6..8].copy_from_slice(&u16::try_from(glyph_info).unwrap().to_be_bytes());
        table[8..10].copy_from_slice(&u16::try_from(variants).unwrap().to_be_bytes());
        table[constants..constants + 2].copy_from_slice(&80i16.to_be_bytes());
        table[constants + 2..constants + 4].copy_from_slice(&60i16.to_be_bytes());
        table[constants + 6..constants + 8].copy_from_slice(&1u16.to_be_bytes());
        for index in [34usize, 47] {
            let value = constants + 8 + index * 4;
            table[value..value + 2].copy_from_slice(&1i16.to_be_bytes());
        }
        table[variants..variants + 2].copy_from_slice(&1u16.to_be_bytes());
        table
    }

    fn selected_ab() -> BTreeSet<OriginalGlyphId> {
        [OriginalGlyphId::new(1), OriginalGlyphId::new(2)]
            .into_iter()
            .collect()
    }

    #[test]
    fn cff_admission_and_subset_are_deterministic() {
        let bytes = fixture();
        let admission = admit_sfnt_cff1(&bytes, 0, &limits()).unwrap();
        assert_eq!(admission.glyph_count(), 4);
        assert_eq!(admission.units_per_em(), 1_000);
        assert_eq!(admission.fs_type(), 0);
        assert_eq!(
            admission.embedding_permission(),
            Cff1EmbeddingPermission::Installable
        );
        let selected = selected_ab();
        let mut first_session = Cff1SubsetSession::new(&limits());
        let first = first_session
            .subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &selected,
                65_535,
            )
            .unwrap();
        let mut second_session = Cff1SubsetSession::new(&limits());
        let second = second_session
            .subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &selected,
                65_535,
            )
            .unwrap();
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.sha256(), second.sha256());
        assert_eq!(first.postscript_name(), "AAAAAA+Typaxis");
        assert_eq!(&first.bytes()[..4], b"OTTO");
    }

    #[test]
    fn cff_face_media_permission_and_limit_fail_closed() {
        let bytes = fixture();
        assert_eq!(
            admit_sfnt_cff1(&bytes, 1, &limits()),
            Err(Cff1Error::InvalidFaceIndex)
        );
        let mut wrong_magic = bytes.clone();
        wrong_magic[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        assert_eq!(
            admit_sfnt_cff1(&wrong_magic, 0, &limits()),
            Err(Cff1Error::InvalidSfnt)
        );
        let extension = M4ResourceLimits {
            max_font_tables: 8,
            ..M4ResourceLimits::default()
        };
        let constrained = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
            extension,
        )
        .unwrap();
        assert_eq!(
            admit_sfnt_cff1(&bytes, 0, &constrained),
            Err(Cff1Error::TableLimit)
        );

        let exact_tables = M4ResourceLimits {
            max_font_tables: 9,
            ..M4ResourceLimits::default()
        };
        assert!(admit_sfnt_cff1(&bytes, 0, &limits_with(exact_tables)).is_ok());

        let below_glyphs = M4ResourceLimits {
            max_font_glyphs: 3,
            ..M4ResourceLimits::default()
        };
        assert_eq!(
            admit_sfnt_cff1(&bytes, 0, &limits_with(below_glyphs)),
            Err(Cff1Error::GlyphLimit)
        );
        let exact_glyphs = M4ResourceLimits {
            max_font_glyphs: 4,
            ..M4ResourceLimits::default()
        };
        assert!(admit_sfnt_cff1(&bytes, 0, &limits_with(exact_glyphs)).is_ok());

        assert_eq!(
            admit_sfnt_cff1(&with_fs_type(0x0002), 0, &limits()),
            Err(Cff1Error::RestrictedEmbedding)
        );
        assert_eq!(
            admit_sfnt_cff1(&with_fs_type(0x0004), 0, &limits())
                .unwrap()
                .embedding_permission(),
            Cff1EmbeddingPermission::PreviewAndPrint
        );
        assert_eq!(
            admit_sfnt_cff1(&with_fs_type(0x0008), 0, &limits())
                .unwrap()
                .embedding_permission(),
            Cff1EmbeddingPermission::Editable
        );
    }

    #[test]
    fn cff_selected_closure_is_exact_and_cached_per_face_gid() {
        let admission = admit_sfnt_cff1(&fixture(), 0, &limits()).unwrap();
        let selected = selected_ab();
        let effective = limits();
        let mut session = Cff1SubsetSession::new(&effective);
        session
            .prepare_face(&admission, FontFaceId::new(0), &selected)
            .unwrap();
        let operations = session.operations_used();
        session
            .subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &[OriginalGlyphId::new(2)].into_iter().collect(),
                4,
            )
            .unwrap();
        session
            .subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(1),
                &[OriginalGlyphId::new(1)].into_iter().collect(),
                4,
            )
            .unwrap();
        assert_eq!(session.operations_used(), operations);
    }

    #[test]
    fn cff_optional_tables_are_fully_walked_before_admission() {
        let valid = with_optional_tables(vec![
            RewriteTable {
                tag: *b"BASE",
                bytes: vec![0, 1, 0, 0, 0, 0, 0, 0],
            },
            RewriteTable {
                tag: *b"GDEF",
                bytes: vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            },
            RewriteTable {
                tag: *b"GPOS",
                bytes: minimal_layout_table(),
            },
            RewriteTable {
                tag: *b"GSUB",
                bytes: minimal_layout_table(),
            },
            RewriteTable {
                tag: *b"JSTF",
                bytes: vec![0, 1, 0, 0, 0, 0],
            },
            RewriteTable {
                tag: *b"MATH",
                bytes: minimal_math_table(),
            },
            RewriteTable {
                tag: *b"kern",
                bytes: vec![0, 0, 0, 0],
            },
        ]);
        let admitted = admit_sfnt_cff1(&valid, 0, &limits()).unwrap();
        assert_eq!(admitted.table_count(), 16);

        for (tag, bytes) in [
            (*b"BASE", vec![0, 1, 0, 0]),
            (*b"GDEF", vec![0, 1, 0, 0]),
            (*b"GPOS", vec![0, 1, 0, 0]),
            (*b"GSUB", vec![0, 1, 0, 0]),
            (*b"JSTF", vec![0, 1, 0, 0]),
            (*b"MATH", vec![0, 1, 0, 0]),
            (*b"kern", vec![0, 0, 0, 1]),
        ] {
            let malformed = with_optional_tables(vec![RewriteTable { tag, bytes }]);
            assert_eq!(
                admit_sfnt_cff1(&malformed, 0, &limits()),
                Err(Cff1Error::InvalidOptionalTable),
                "optional table {:?} was not rejected",
                std::str::from_utf8(&tag).unwrap()
            );
        }
    }

    #[test]
    fn cff_every_truncated_prefix_and_malformed_container_fail_closed() {
        let bytes = fixture();
        for end in 0..bytes.len() {
            assert!(
                admit_sfnt_cff1(&bytes[..end], 0, &limits()).is_err(),
                "truncated prefix {end} was admitted"
            );
        }

        let mut corrupt_table = bytes.clone();
        let (_, cmap, _) = table_location(&corrupt_table, b"cmap");
        corrupt_table[cmap] ^= 1;
        assert_eq!(
            admit_sfnt_cff1(&corrupt_table, 0, &limits()),
            Err(Cff1Error::InvalidSfnt)
        );

        let mut unsupported = bytes.clone();
        let (record, _, _) = table_location(&unsupported, b"CFF ");
        unsupported[record..record + 4].copy_from_slice(b"CFF2");
        recompute_sfnt_checksums(&mut unsupported);
        assert_eq!(
            admit_sfnt_cff1(&unsupported, 0, &limits()),
            Err(Cff1Error::UnsupportedTable)
        );

        let mut overlap = bytes.clone();
        let (cmap_record, _, _) = table_location(&overlap, b"cmap");
        let (_, cff_offset, _) = table_location(&overlap, b"CFF ");
        overlap[cmap_record + 8..cmap_record + 12]
            .copy_from_slice(&u32::try_from(cff_offset).unwrap().to_be_bytes());
        assert!(admit_sfnt_cff1(&overlap, 0, &limits()).is_err());
    }

    #[test]
    fn cff_operation_outline_subset_and_selected_limits_are_inclusive() {
        let bytes = fixture();
        let selected = selected_ab();
        let admission = admit_sfnt_cff1(&bytes, 0, &limits()).unwrap();
        let mut baseline = Cff1SubsetSession::from_admission(&admission);
        let subset = baseline
            .subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &selected,
                2,
            )
            .unwrap();
        let operations = baseline.operations_used();
        let segments = baseline.outline_segments_used();
        let subset_bytes = u64::try_from(subset.bytes().len()).unwrap();
        assert!(operations > 1);
        assert!(segments > 1);
        assert!(subset_bytes > 1);

        let exact_operations = M4ResourceLimits {
            max_cff_charstring_operations: operations,
            ..M4ResourceLimits::default()
        };
        let exact = limits_with(exact_operations);
        let admission = admit_sfnt_cff1(&bytes, 0, &exact).unwrap();
        let mut session = Cff1SubsetSession::from_admission(&admission);
        session
            .subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &selected,
                2,
            )
            .unwrap();
        assert_eq!(session.operations_used(), operations);

        let below_operations = M4ResourceLimits {
            max_cff_charstring_operations: operations - 1,
            ..M4ResourceLimits::default()
        };
        let below = limits_with(below_operations);
        let admission = admit_sfnt_cff1(&bytes, 0, &below).unwrap();
        let mut session = Cff1SubsetSession::from_admission(&admission);
        assert_eq!(
            session.subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &selected,
                2,
            ),
            Err(Cff1Error::CharstringOperationLimit)
        );
        assert_eq!(session.operations_used(), operations - 1);

        let exact_segments = M4ResourceLimits {
            max_cff_outline_segments: segments,
            ..M4ResourceLimits::default()
        };
        let exact = limits_with(exact_segments);
        let admission = admit_sfnt_cff1(&bytes, 0, &exact).unwrap();
        let mut session = Cff1SubsetSession::from_admission(&admission);
        session
            .subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &selected,
                2,
            )
            .unwrap();
        assert_eq!(session.outline_segments_used(), segments);

        let below_segments = M4ResourceLimits {
            max_cff_outline_segments: segments - 1,
            ..M4ResourceLimits::default()
        };
        let below = limits_with(below_segments);
        let admission = admit_sfnt_cff1(&bytes, 0, &below).unwrap();
        let mut session = Cff1SubsetSession::from_admission(&admission);
        assert_eq!(
            session.subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &selected,
                2,
            ),
            Err(Cff1Error::OutlineSegmentLimit)
        );
        assert_eq!(session.outline_segments_used(), segments - 1);

        let exact_subset = M4ResourceLimits {
            max_font_subset_bytes: subset_bytes,
            ..M4ResourceLimits::default()
        };
        let exact = limits_with(exact_subset);
        let admission = admit_sfnt_cff1(&bytes, 0, &exact).unwrap();
        let mut session = Cff1SubsetSession::from_admission(&admission);
        assert_eq!(
            u64::try_from(
                session
                    .subset(
                        &admission,
                        FontFaceId::new(0),
                        FontInstanceId::new(0),
                        &selected,
                        2,
                    )
                    .unwrap()
                    .bytes()
                    .len()
            )
            .unwrap(),
            subset_bytes
        );

        let below_subset = M4ResourceLimits {
            max_font_subset_bytes: subset_bytes - 1,
            ..M4ResourceLimits::default()
        };
        let below = limits_with(below_subset);
        let admission = admit_sfnt_cff1(&bytes, 0, &below).unwrap();
        let mut session = Cff1SubsetSession::from_admission(&admission);
        assert_eq!(
            session.subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &selected,
                2,
            ),
            Err(Cff1Error::SubsetByteLimit)
        );

        let mut session = Cff1SubsetSession::from_admission(&admission);
        assert_eq!(
            session.subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &selected,
                1,
            ),
            Err(Cff1Error::SelectedGlyphLimit)
        );
        let invalid = [OriginalGlyphId::new(4)].into_iter().collect();
        assert_eq!(
            session.subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &invalid,
                2,
            ),
            Err(Cff1Error::InvalidSelectedGlyph)
        );
    }

    #[test]
    fn cff_subroutine_recursion_is_iterative_and_depth_bounded() {
        let mut admission = admit_sfnt_cff1(&fixture(), 0, &limits()).unwrap();
        // With one subroutine the Type 2 bias is 107; encoded -107 is byte 32.
        admission.program.global_subrs = vec![vec![32, 29, 11]];
        admission.program.charstrings[1] = vec![32, 29, 14];
        let selected = [OriginalGlyphId::new(1)].into_iter().collect();
        let mut session = Cff1SubsetSession::from_admission(&admission);
        assert_eq!(
            session.subset(
                &admission,
                FontFaceId::new(0),
                FontInstanceId::new(0),
                &selected,
                1,
            ),
            Err(Cff1Error::InvalidCharstring)
        );
        assert!(session.operations_used() < 100);
    }

    #[test]
    fn cff_top_dict_requires_explicit_type2_and_exact_font_matrix() {
        let bbox = [0, -200, 600, 800];
        let mut valid = vec![
            DictEntry {
                operator: 5,
                operands: bbox
                    .into_iter()
                    .map(|value| DictOperand::Integer(i32::from(value)))
                    .collect(),
            },
            DictEntry {
                operator: 17,
                operands: vec![DictOperand::Integer(100)],
            },
            DictEntry {
                operator: 18,
                operands: vec![DictOperand::Integer(10), DictOperand::Integer(200)],
            },
            DictEntry {
                operator: 0x0C06,
                operands: vec![DictOperand::Integer(2)],
            },
            DictEntry {
                operator: 0x0C07,
                operands: vec![
                    DictOperand::Real("1E-3".to_owned()),
                    DictOperand::Real("0.0".to_owned()),
                    DictOperand::Integer(0),
                    DictOperand::Real(".0010".to_owned()),
                    DictOperand::Integer(0),
                    DictOperand::Integer(0),
                ],
            },
        ];
        assert_eq!(validate_top_dict(&valid, bbox, 0), Ok(()));

        let mut without_type = valid.clone();
        without_type.retain(|entry| entry.operator != 0x0C06);
        assert_eq!(
            validate_top_dict(&without_type, bbox, 0),
            Err(Cff1Error::InvalidCff)
        );
        let matrix = valid
            .iter_mut()
            .find(|entry| entry.operator == 0x0C07)
            .unwrap();
        matrix.operands[0] = DictOperand::Real("0.01".to_owned());
        assert_eq!(
            validate_top_dict(&valid, bbox, 0),
            Err(Cff1Error::InvalidCff)
        );
        for malformed in ["", "-", ".", "1E", "1E-", "1.2.3", "1E2E3"] {
            assert!(normalize_dict_decimal(malformed).is_none());
        }
    }
}
