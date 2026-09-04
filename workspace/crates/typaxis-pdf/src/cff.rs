use typaxis_core::{push_jcs_string, sha256, FontInstanceId, LayoutStateFingerprint};

use super::{
    cid_set, pdf_name, to_unicode_cmap, FrozenPdfGraph, IndirectObjectBody, ObjectId,
    PdfDictionary, PdfError, PdfFontProgramKind, PdfValue,
};

pub const STAGING_CFF1_PDF_OBSERVATION_ALGORITHM: &str = "typaxis.cff1-pdf-observation/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingCff1PdfFontObject {
    font_face_id: typaxis_core::FontFaceId,
    font_instance_id: FontInstanceId,
    resource_name: String,
    object_numbers: [u32; 6],
    subset_byte_length: u64,
    subset_sha256: [u8; 32],
    pdf_plan_fingerprint: [u8; 32],
    to_unicode_sha256: [u8; 32],
    cid_set_sha256: [u8; 32],
    cid_count: u32,
    fingerprint: [u8; 32],
}

impl StagingCff1PdfFontObject {
    pub const fn font_face_id(&self) -> typaxis_core::FontFaceId {
        self.font_face_id
    }
    pub const fn font_instance_id(&self) -> FontInstanceId {
        self.font_instance_id
    }
    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }
    pub const fn object_numbers(&self) -> [u32; 6] {
        self.object_numbers
    }
    pub const fn subset_byte_length(&self) -> u64 {
        self.subset_byte_length
    }
    pub const fn subset_sha256(&self) -> [u8; 32] {
        self.subset_sha256
    }
    pub const fn pdf_plan_fingerprint(&self) -> [u8; 32] {
        self.pdf_plan_fingerprint
    }
    pub const fn to_unicode_sha256(&self) -> [u8; 32] {
        self.to_unicode_sha256
    }
    pub const fn cid_set_sha256(&self) -> [u8; 32] {
        self.cid_set_sha256
    }
    pub const fn cid_count(&self) -> u32 {
        self.cid_count
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingCff1PdfObservation {
    selected_layout_fingerprint: LayoutStateFingerprint,
    object_count: u32,
    fonts: Vec<StagingCff1PdfFontObject>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingCff1PdfObservation {
    pub const fn selected_layout_fingerprint(&self) -> LayoutStateFingerprint {
        self.selected_layout_fingerprint
    }
    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub fn fonts(&self) -> &[StagingCff1PdfFontObject] {
        &self.fonts
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(&self, graph: &FrozenPdfGraph) -> Result<(), PdfError> {
        if self == &observe_staging_cff1_pdf(graph)? {
            Ok(())
        } else {
            Err(PdfError::ResourcePlanMismatch)
        }
    }
}

pub fn observe_staging_cff1_pdf(
    graph: &FrozenPdfGraph,
) -> Result<StagingCff1PdfObservation, PdfError> {
    let mut fonts = Vec::new();
    for binding in &graph.font_bindings {
        let type0 = dictionary(graph, binding.object_id)?;
        if !name_is(type0, b"Subtype", b"Type0") || !name_is(type0, b"Encoding", b"Identity-H") {
            return Err(PdfError::ResourcePlanMismatch);
        }
        let cid_font_id = single_reference_array(type0, b"DescendantFonts")?;
        let cid_font = dictionary(graph, cid_font_id)?;
        let is_cff1 = name_is(cid_font, b"Subtype", b"CIDFontType0");
        if !is_cff1 {
            if !name_is(cid_font, b"Subtype", b"CIDFontType2") {
                return Err(PdfError::ResourcePlanMismatch);
            }
            // A CFF descendant changed to CIDFontType2 must not disappear
            // from this observation as an apparently unrelated TrueType
            // font. Prove the complete alternate program-kind boundary before
            // excluding a legitimate TrueType binding.
            let descriptor_id = reference(cid_font, b"FontDescriptor")?;
            let descriptor = dictionary(graph, descriptor_id)?;
            let font_program_id = reference(descriptor, b"FontFile2")?;
            if descriptor.contains_key(&pdf_name(b"FontFile3")?)
                || descriptor.contains_key(&pdf_name(b"CIDSet")?)
                || reference(cid_font, b"CIDToGIDMap").is_err()
                || !matches!(
                    graph.graph.objects.get(&font_program_id),
                    Some(IndirectObjectBody::FrozenFontProgram(program))
                        if program.program_kind() == PdfFontProgramKind::TrueTypeGlyf
                            && program.font_instance_id() == binding.logical_id
                )
            {
                return Err(PdfError::ResourcePlanMismatch);
            }
            continue;
        }
        let descriptor_id = reference(cid_font, b"FontDescriptor")?;
        let descriptor = dictionary(graph, descriptor_id)?;
        let font_program_id = reference(descriptor, b"FontFile3")?;
        let to_unicode_id = reference(type0, b"ToUnicode")?;
        let cid_set_id = reference(descriptor, b"CIDSet")?;
        if cid_font.contains_key(&pdf_name(b"CIDToGIDMap")?)
            || descriptor.contains_key(&pdf_name(b"FontFile2")?)
            || !integer_is(cid_font, b"DW", 1_000)
            || !same_name(type0, b"BaseFont", cid_font, b"BaseFont")
            || !same_name(type0, b"BaseFont", descriptor, b"FontName")
        {
            return Err(PdfError::ResourcePlanMismatch);
        }
        let program = match graph.graph.objects.get(&font_program_id) {
            Some(IndirectObjectBody::FrozenFontProgram(program))
                if program.program_kind() == PdfFontProgramKind::OpenTypeCff1
                    && program.font_instance_id() == binding.logical_id =>
            {
                program
            }
            _ => return Err(PdfError::ResourcePlanMismatch),
        };
        let cff = program.cff1_plan().ok_or(PdfError::ResourcePlanMismatch)?;
        if !dense_widths_match(cid_font, cff.dense_widths_1000())?
            || !matches!(
                graph.graph.objects.get(&to_unicode_id),
                Some(IndirectObjectBody::FrozenToUnicodeCMap { font_program_object })
                    if *font_program_object == font_program_id
            )
            || !matches!(
                graph.graph.objects.get(&cid_set_id),
                Some(IndirectObjectBody::FrozenCidSet { font_program_object })
                    if *font_program_object == font_program_id
            )
        {
            return Err(PdfError::ResourcePlanMismatch);
        }
        let object_numbers = [
            binding.object_id.get(),
            cid_font_id.get(),
            descriptor_id.get(),
            font_program_id.get(),
            to_unicode_id.get(),
            cid_set_id.get(),
        ];
        if object_numbers
            .windows(2)
            .any(|pair| pair[0].checked_add(1) != Some(pair[1]))
        {
            return Err(PdfError::ResourcePlanMismatch);
        }
        let subset_byte_length =
            u64::try_from(program.subset_bytes().len()).map_err(|_| PdfError::OutputTooLarge)?;
        let cid_count = u32::try_from(cff.dense_widths_1000().len())
            .map_err(|_| PdfError::ObjectCountOverflow)?;
        let to_unicode_sha256 = sha256(&to_unicode_cmap(program, u64::MAX)?);
        let cid_set_sha256 = sha256(&cid_set(program, u64::MAX)?);
        let resource_name = String::from_utf8(binding.name.0.clone())
            .map_err(|_| PdfError::ResourcePlanMismatch)?;
        let mut font = StagingCff1PdfFontObject {
            font_face_id: cff.font_face_id(),
            font_instance_id: binding.logical_id,
            resource_name,
            object_numbers,
            subset_byte_length,
            subset_sha256: cff.subset_sha256(),
            pdf_plan_fingerprint: cff.fingerprint(),
            to_unicode_sha256,
            cid_set_sha256,
            cid_count,
            fingerprint: [0; 32],
        };
        font.fingerprint = sha256(encode_font(&font).as_bytes());
        fonts.push(font);
    }
    fonts.sort_by_key(|font| (font.font_face_id, font.font_instance_id));
    let canonical_jcs = encode_observation(
        graph.selected_layout_fingerprint,
        graph.object_count,
        &fonts,
    );
    Ok(StagingCff1PdfObservation {
        selected_layout_fingerprint: graph.selected_layout_fingerprint,
        object_count: graph.object_count,
        fonts,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn dictionary(graph: &FrozenPdfGraph, id: ObjectId) -> Result<&PdfDictionary, PdfError> {
    match graph.graph.objects.get(&id) {
        Some(IndirectObjectBody::Value(PdfValue::Dictionary(dictionary))) => Ok(dictionary),
        _ => Err(PdfError::ResourcePlanMismatch),
    }
}

fn reference(dictionary: &PdfDictionary, key: &[u8]) -> Result<ObjectId, PdfError> {
    match dictionary.get(&pdf_name(key)?) {
        Some(PdfValue::Reference(id)) => Ok(*id),
        _ => Err(PdfError::ResourcePlanMismatch),
    }
}

fn single_reference_array(dictionary: &PdfDictionary, key: &[u8]) -> Result<ObjectId, PdfError> {
    match dictionary.get(&pdf_name(key)?) {
        Some(PdfValue::Array(values)) if values.len() == 1 => match values.first() {
            Some(PdfValue::Reference(id)) => Ok(*id),
            _ => Err(PdfError::ResourcePlanMismatch),
        },
        _ => Err(PdfError::ResourcePlanMismatch),
    }
}

fn name_is(dictionary: &PdfDictionary, key: &[u8], expected: &[u8]) -> bool {
    dictionary
        .get(&pdf_name_key(key))
        .is_some_and(|value| matches!(value, PdfValue::Name(name) if name.is(expected)))
}

fn integer_is(dictionary: &PdfDictionary, key: &[u8], expected: i64) -> bool {
    dictionary
        .get(&pdf_name_key(key))
        .is_some_and(|value| matches!(value, PdfValue::Integer(actual) if *actual == expected))
}

// `PdfName` has a fallible public constructor because arbitrary callers may
// pass NUL. These internal keys are fixed ASCII; this helper keeps predicates
// infallible without weakening that constructor.
fn pdf_name_key(bytes: &[u8]) -> super::PdfName {
    super::PdfName(bytes.to_vec())
}

fn same_name(
    first: &PdfDictionary,
    first_key: &[u8],
    second: &PdfDictionary,
    second_key: &[u8],
) -> bool {
    matches!(
        (
            first.get(&pdf_name_key(first_key)),
            second.get(&pdf_name_key(second_key))
        ),
        (Some(PdfValue::Name(left)), Some(PdfValue::Name(right))) if left == right
    )
}

fn dense_widths_match(dictionary: &PdfDictionary, expected: &[u32]) -> Result<bool, PdfError> {
    let Some(PdfValue::Array(values)) = dictionary.get(&pdf_name(b"W")?) else {
        return Ok(false);
    };
    let [PdfValue::Integer(0), PdfValue::Array(widths)] = values.as_slice() else {
        return Ok(false);
    };
    Ok(widths.len() == expected.len()
        && widths
            .iter()
            .zip(expected)
            .all(|(actual, expected)| *actual == PdfValue::Integer(i64::from(*expected))))
}

fn encode_observation(
    selected_layout: LayoutStateFingerprint,
    object_count: u32,
    fonts: &[StagingCff1PdfFontObject],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_CFF1_PDF_OBSERVATION_ALGORITHM);
    output.push_str(",\"fonts\":[");
    for (index, font) in fonts.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&encode_font(font));
    }
    output.push_str("],\"object_count\":");
    output.push_str(&object_count.to_string());
    output.push_str(",\"selected_layout_fingerprint\":");
    push_hash(&mut output, selected_layout.bytes());
    output.push('}');
    output
}

fn encode_font(font: &StagingCff1PdfFontObject) -> String {
    let mut output = String::from("{\"cid_count\":");
    output.push_str(&font.cid_count.to_string());
    output.push_str(",\"cid_set_sha256\":");
    push_hash(&mut output, font.cid_set_sha256);
    output.push_str(",\"font_face_id\":");
    output.push_str(&font.font_face_id.get().to_string());
    output.push_str(",\"font_instance_id\":");
    output.push_str(&font.font_instance_id.get().to_string());
    output.push_str(",\"object_numbers\":[");
    for (index, object) in font.object_numbers.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&object.to_string());
    }
    output.push_str("],\"pdf_plan_fingerprint\":");
    push_hash(&mut output, font.pdf_plan_fingerprint);
    output.push_str(",\"resource_name\":");
    push_jcs_string(&mut output, &font.resource_name);
    output.push_str(",\"subset_byte_length\":");
    output.push_str(&font.subset_byte_length.to_string());
    output.push_str(",\"subset_sha256\":");
    push_hash(&mut output, font.subset_sha256);
    output.push_str(",\"to_unicode_sha256\":");
    push_hash(&mut output, font.to_unicode_sha256);
    output.push('}');
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
