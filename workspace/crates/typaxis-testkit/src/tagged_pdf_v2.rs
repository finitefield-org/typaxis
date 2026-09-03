use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::sha256;

use crate::safe_vector_pdf::{contains, find_from, object_stream, parse_objects, ParsedObject};

pub const TAGGED_PDF_INDEPENDENT_VALIDATOR_ALGORITHM_V2: &str = "typaxis.tagged-pdf-validator/2";
pub const TAGGED_PDF_OBSERVATION_ALGORITHM_V2: &str = "typaxis.tagged-pdf-observation/2";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaggedPdfObjectClosureV2 {
    object_number: u32,
    role: String,
    sha256: [u8; 32],
}

impl TaggedPdfObjectClosureV2 {
    pub fn new(object_number: u32, role: String, sha256: [u8; 32]) -> Self {
        Self {
            object_number,
            role,
            sha256,
        }
    }

    pub const fn object_number(&self) -> u32 {
        self.object_number
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub const fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaggedPdfIndependentClosureV2 {
    observation_algorithm: String,
    pdf_sha256: [u8; 32],
    pdf_byte_length: u64,
    object_count: u32,
    object_budget_charge_count: u32,
    xmp_sha256: [u8; 32],
    objects: Vec<TaggedPdfObjectClosureV2>,
}

impl TaggedPdfIndependentClosureV2 {
    pub fn new(
        observation_algorithm: String,
        pdf_sha256: [u8; 32],
        pdf_byte_length: u64,
        object_count: u32,
        object_budget_charge_count: u32,
        xmp_sha256: [u8; 32],
        objects: Vec<TaggedPdfObjectClosureV2>,
    ) -> Result<Self, TaggedPdfIndependentErrorV2> {
        if observation_algorithm != TAGGED_PDF_OBSERVATION_ALGORITHM_V2
            || pdf_byte_length == 0
            || object_count == 0
            || object_budget_charge_count != 1
            || xmp_sha256 == [0; 32]
            || usize::try_from(object_count) != Ok(objects.len())
            || objects.iter().enumerate().any(|(index, object)| {
                u32::try_from(index + 1) != Ok(object.object_number)
                    || object.role.is_empty()
                    || !canonical_object_role(&object.role)
                    || object.sha256 == [0; 32]
            })
            || objects
                .iter()
                .map(|object| object.role.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                != objects.len()
        {
            return Err(TaggedPdfIndependentErrorV2::InvalidClosure);
        }
        Ok(Self {
            observation_algorithm,
            pdf_sha256,
            pdf_byte_length,
            object_count,
            object_budget_charge_count,
            xmp_sha256,
            objects,
        })
    }

    pub fn observation_algorithm(&self) -> &str {
        &self.observation_algorithm
    }

    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }

    pub const fn pdf_byte_length(&self) -> u64 {
        self.pdf_byte_length
    }

    pub const fn object_count(&self) -> u32 {
        self.object_count
    }

    pub const fn object_budget_charge_count(&self) -> u32 {
        self.object_budget_charge_count
    }

    pub const fn xmp_sha256(&self) -> [u8; 32] {
        self.xmp_sha256
    }

    pub fn objects(&self) -> &[TaggedPdfObjectClosureV2] {
        &self.objects
    }

    fn object_for_role(&self, role: &str) -> Option<u32> {
        self.objects
            .iter()
            .find(|object| object.role == role)
            .map(|object| object.object_number)
    }
}

fn canonical_object_role(role: &str) -> bool {
    const SINGLETONS: &[&str] = &[
        "catalog",
        "pages",
        "destinations",
        "info",
        "metadata",
        "outline_root",
        "structure_tree_root",
        "structure_parent_tree",
        "structure_id_tree",
    ];
    const INDEXED: &[&str] = &[
        "page_content",
        "page",
        "equation_font_type0",
        "equation_font_cid",
        "equation_font_descriptor",
        "equation_font_program",
        "equation_font_to_unicode",
        "equation_font_cid_to_gid",
        "link_annotation",
        "outline_item",
        "vector_form",
        "vector_ext_g_state",
        "structure_element",
    ];
    if SINGLETONS.contains(&role) {
        return true;
    }
    let Some((prefix, suffix)) = role.split_once(':') else {
        return false;
    };
    INDEXED.contains(&prefix)
        && !suffix.is_empty()
        && (suffix == "0" || !suffix.starts_with('0'))
        && suffix.bytes().all(|byte| byte.is_ascii_digit())
        && suffix.parse::<u32>().is_ok()
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TaggedVectorSemanticKindV2 {
    InlineVector,
    MathVector,
    VectorFigure,
    MathVectorBlock,
}

impl TaggedVectorSemanticKindV2 {
    const fn role(self) -> &'static str {
        match self {
            Self::InlineVector | Self::VectorFigure => "Figure",
            Self::MathVector | Self::MathVectorBlock => "Formula",
        }
    }

    const fn is_math(self) -> bool {
        matches!(self, Self::MathVector | Self::MathVectorBlock)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaggedVectorExpectationV2 {
    kind: TaggedVectorSemanticKindV2,
    page_index: u32,
    mcid: u32,
    structure_node_id: u32,
    alternative: String,
    actual_text: Option<String>,
    paint_language: Option<String>,
    structure_language: Option<String>,
    form_index: u32,
}

impl TaggedVectorExpectationV2 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: TaggedVectorSemanticKindV2,
        page_index: u32,
        mcid: u32,
        structure_node_id: u32,
        alternative: String,
        actual_text: Option<String>,
        paint_language: Option<String>,
        structure_language: Option<String>,
        form_index: u32,
    ) -> Result<Self, TaggedPdfIndependentErrorV2> {
        if structure_node_id == 0
            || alternative.trim().is_empty()
            || actual_text
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            || paint_language
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            || structure_language
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            || (kind.is_math() && actual_text.is_none())
            || (kind == TaggedVectorSemanticKindV2::VectorFigure && actual_text.is_some())
        {
            return Err(TaggedPdfIndependentErrorV2::InvalidExpectation);
        }
        Ok(Self {
            kind,
            page_index,
            mcid,
            structure_node_id,
            alternative,
            actual_text,
            paint_language,
            structure_language,
            form_index,
        })
    }

    pub const fn kind(&self) -> TaggedVectorSemanticKindV2 {
        self.kind
    }

    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn mcid(&self) -> u32 {
        self.mcid
    }

    pub const fn structure_node_id(&self) -> u32 {
        self.structure_node_id
    }

    pub fn alternative(&self) -> &str {
        &self.alternative
    }

    pub fn actual_text(&self) -> Option<&str> {
        self.actual_text.as_deref()
    }

    pub fn paint_language(&self) -> Option<&str> {
        self.paint_language.as_deref()
    }

    pub fn structure_language(&self) -> Option<&str> {
        self.structure_language.as_deref()
    }

    pub const fn form_index(&self) -> u32 {
        self.form_index
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaggedEquationNumberExpectationV2 {
    page_index: u32,
    mcid: u32,
    structure_node_id: u32,
    parent_structure_node_id: u32,
    exact_text: String,
    paint_language: Option<String>,
    structure_language: Option<String>,
    font_index: u32,
}

impl TaggedEquationNumberExpectationV2 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        page_index: u32,
        mcid: u32,
        structure_node_id: u32,
        parent_structure_node_id: u32,
        exact_text: String,
        paint_language: Option<String>,
        structure_language: Option<String>,
        font_index: u32,
    ) -> Result<Self, TaggedPdfIndependentErrorV2> {
        if structure_node_id == 0
            || parent_structure_node_id == 0
            || structure_node_id == parent_structure_node_id
            || exact_text.trim().is_empty()
            || exact_text.chars().any(char::is_control)
            || paint_language
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            || structure_language
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(TaggedPdfIndependentErrorV2::InvalidExpectation);
        }
        Ok(Self {
            page_index,
            mcid,
            structure_node_id,
            parent_structure_node_id,
            exact_text,
            paint_language,
            structure_language,
            font_index,
        })
    }

    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn mcid(&self) -> u32 {
        self.mcid
    }

    pub const fn structure_node_id(&self) -> u32 {
        self.structure_node_id
    }

    pub const fn parent_structure_node_id(&self) -> u32 {
        self.parent_structure_node_id
    }

    pub fn exact_text(&self) -> &str {
        &self.exact_text
    }

    pub fn paint_language(&self) -> Option<&str> {
        self.paint_language.as_deref()
    }

    pub fn structure_language(&self) -> Option<&str> {
        self.structure_language.as_deref()
    }

    pub const fn font_index(&self) -> u32 {
        self.font_index
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaggedPdfIndependentExpectationsV2 {
    document_language: String,
    page_count: u32,
    form_count: u32,
    vectors: Vec<TaggedVectorExpectationV2>,
    equation_numbers: Vec<TaggedEquationNumberExpectationV2>,
    closure: TaggedPdfIndependentClosureV2,
}

impl TaggedPdfIndependentExpectationsV2 {
    pub fn new(
        document_language: String,
        page_count: u32,
        form_count: u32,
        vectors: Vec<TaggedVectorExpectationV2>,
        equation_numbers: Vec<TaggedEquationNumberExpectationV2>,
        closure: TaggedPdfIndependentClosureV2,
    ) -> Result<Self, TaggedPdfIndependentErrorV2> {
        if document_language.trim().is_empty()
            || page_count == 0
            || form_count == 0
            || vectors.is_empty()
            || vectors
                .iter()
                .any(|vector| vector.page_index >= page_count || vector.form_index >= form_count)
            || equation_numbers
                .iter()
                .any(|number| number.page_index >= page_count)
            || vectors
                .iter()
                .map(|vector| vector.structure_node_id)
                .chain(
                    equation_numbers
                        .iter()
                        .map(|number| number.structure_node_id),
                )
                .collect::<BTreeSet<_>>()
                .len()
                != vectors.len() + equation_numbers.len()
            || vectors
                .iter()
                .map(|vector| vector.form_index)
                .collect::<BTreeSet<_>>()
                != (0..form_count).collect()
        {
            return Err(TaggedPdfIndependentErrorV2::InvalidExpectation);
        }
        let mut page_mcids = BTreeMap::<u32, Vec<u32>>::new();
        for vector in &vectors {
            page_mcids
                .entry(vector.page_index)
                .or_default()
                .push(vector.mcid);
        }
        for number in &equation_numbers {
            page_mcids
                .entry(number.page_index)
                .or_default()
                .push(number.mcid);
            let Some(parent) = vectors
                .iter()
                .find(|vector| vector.structure_node_id == number.parent_structure_node_id)
            else {
                return Err(TaggedPdfIndependentErrorV2::InvalidExpectation);
            };
            if parent.kind != TaggedVectorSemanticKindV2::MathVectorBlock
                || parent.page_index != number.page_index
                || parent.mcid.checked_add(1) != Some(number.mcid)
            {
                return Err(TaggedPdfIndependentErrorV2::InvalidExpectation);
            }
        }
        if (0..page_count).any(|page| {
            let mut mcids = page_mcids.remove(&page).unwrap_or_default();
            mcids.sort_unstable();
            mcids.iter().copied().ne(0..mcids.len() as u32)
        }) || !page_mcids.is_empty()
            || !valid_object_role_plan(
                &closure,
                page_count,
                form_count,
                &vectors,
                &equation_numbers,
            )
        {
            return Err(TaggedPdfIndependentErrorV2::InvalidExpectation);
        }
        Ok(Self {
            document_language,
            page_count,
            form_count,
            vectors,
            equation_numbers,
            closure,
        })
    }

    pub fn document_language(&self) -> &str {
        &self.document_language
    }

    pub const fn page_count(&self) -> u32 {
        self.page_count
    }

    pub const fn form_count(&self) -> u32 {
        self.form_count
    }

    pub fn vectors(&self) -> &[TaggedVectorExpectationV2] {
        &self.vectors
    }

    pub fn equation_numbers(&self) -> &[TaggedEquationNumberExpectationV2] {
        &self.equation_numbers
    }

    pub const fn closure(&self) -> &TaggedPdfIndependentClosureV2 {
        &self.closure
    }
}

fn valid_object_role_plan(
    closure: &TaggedPdfIndependentClosureV2,
    page_count: u32,
    form_count: u32,
    vectors: &[TaggedVectorExpectationV2],
    equation_numbers: &[TaggedEquationNumberExpectationV2],
) -> bool {
    let required = [
        "catalog",
        "pages",
        "destinations",
        "info",
        "metadata",
        "structure_tree_root",
        "structure_parent_tree",
    ];
    if closure.object_for_role("catalog") != Some(1)
        || required
            .iter()
            .any(|role| closure.object_for_role(role).is_none())
        || indexed_roles(closure, "page") != Some((0..page_count).collect())
        || indexed_roles(closure, "page_content") != Some((0..page_count).collect())
    {
        return false;
    }

    let Some(forms) = indexed_roles(closure, "vector_form") else {
        return false;
    };
    let Some(ext_g_states) = indexed_roles(closure, "vector_ext_g_state") else {
        return false;
    };
    let mut vector_objects = forms.clone();
    vector_objects.extend(ext_g_states);
    vector_objects.sort_unstable();
    let Some(structure_elements) = indexed_roles(closure, "structure_element") else {
        return false;
    };
    let Some(link_annotations) = indexed_roles(closure, "link_annotation") else {
        return false;
    };
    let Some(outline_items) = indexed_roles(closure, "outline_item") else {
        return false;
    };
    let font_indices = equation_numbers
        .iter()
        .map(|number| number.font_index)
        .collect::<BTreeSet<_>>();
    let font_count = match u32::try_from(font_indices.len()) {
        Ok(count) => count,
        Err(_) => return false,
    };
    let expected_fonts = (0..font_count).collect::<Vec<_>>();
    let font_roles_match = [
        "equation_font_type0",
        "equation_font_cid",
        "equation_font_descriptor",
        "equation_font_program",
        "equation_font_to_unicode",
        "equation_font_cid_to_gid",
    ]
    .iter()
    .all(|prefix| indexed_roles(closure, prefix).as_deref() == Some(expected_fonts.as_slice()));
    usize::try_from(form_count) == Ok(forms.len())
        && font_indices.iter().copied().eq(0..font_count)
        && font_roles_match
        && is_dense_zero_based(&vector_objects)
        && is_dense_zero_based(&structure_elements)
        && is_dense_zero_based(&link_annotations)
        && is_dense_zero_based(&outline_items)
        && (closure.object_for_role("outline_root").is_some() == !outline_items.is_empty())
        && vectors
            .iter()
            .map(|vector| vector.structure_node_id)
            .chain(
                equation_numbers
                    .iter()
                    .map(|number| number.structure_node_id),
            )
            .all(|node| structure_elements.binary_search(&node).is_ok())
}

fn indexed_roles(closure: &TaggedPdfIndependentClosureV2, prefix: &str) -> Option<Vec<u32>> {
    let marker = format!("{prefix}:");
    let mut values = closure
        .objects
        .iter()
        .filter_map(|object| object.role.strip_prefix(&marker))
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    values.sort_unstable();
    Some(values)
}

fn is_dense_zero_based(values: &[u32]) -> bool {
    u32::try_from(values.len()).is_ok_and(|length| values.iter().copied().eq(0..length))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaggedPdfIndependentReportV2 {
    validator_algorithm: &'static str,
    pdf_sha256: [u8; 32],
    object_count: u32,
    page_count: u32,
    form_count: u32,
    vector_count: u32,
    equation_number_count: u32,
    form_do_count: u32,
    extracted_text: Vec<String>,
}

impl TaggedPdfIndependentReportV2 {
    pub const fn validator_algorithm(&self) -> &'static str {
        self.validator_algorithm
    }

    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }

    pub const fn object_count(&self) -> u32 {
        self.object_count
    }

    pub const fn page_count(&self) -> u32 {
        self.page_count
    }

    pub const fn form_count(&self) -> u32 {
        self.form_count
    }

    pub const fn vector_count(&self) -> u32 {
        self.vector_count
    }

    pub const fn equation_number_count(&self) -> u32 {
        self.equation_number_count
    }

    pub const fn form_do_count(&self) -> u32 {
        self.form_do_count
    }

    pub fn extracted_text(&self) -> &[String] {
        &self.extracted_text
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaggedPdfIndependentErrorV2 {
    InvalidExpectation,
    InvalidClosure,
    MalformedPdf,
    PdfHashMismatch,
    ObjectClosureMismatch,
    CatalogMismatch,
    OutlineMismatch,
    MetadataMismatch,
    PageMismatch,
    MarkedContentMismatch,
    StructureMismatch,
    ParentTreeMismatch,
    FormSemanticContent,
    FormUsageMismatch,
    TextExtractionMismatch,
}

impl std::fmt::Display for TaggedPdfIndependentErrorV2 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidExpectation => formatter.write_str("invalid tagged-PDF /2 expectation"),
            Self::InvalidClosure => {
                formatter.write_str("invalid tagged-PDF observation /2 closure")
            }
            Self::MalformedPdf => formatter.write_str("malformed independently parsed tagged PDF"),
            Self::PdfHashMismatch => formatter.write_str("tagged-PDF final byte hash differs"),
            Self::ObjectClosureMismatch => formatter.write_str("tagged-PDF object closure differs"),
            Self::CatalogMismatch => formatter.write_str("tagged-PDF catalog differs"),
            Self::OutlineMismatch => formatter.write_str("tagged-PDF outline graph differs"),
            Self::MetadataMismatch => formatter.write_str("tagged-PDF XMP metadata differs"),
            Self::PageMismatch => formatter.write_str("tagged-PDF page graph differs"),
            Self::MarkedContentMismatch => formatter.write_str("tagged-PDF marked content differs"),
            Self::StructureMismatch => formatter.write_str("tagged-PDF structure element differs"),
            Self::ParentTreeMismatch => formatter.write_str("tagged-PDF ParentTree differs"),
            Self::FormSemanticContent => {
                formatter.write_str("reusable vector Form contains semantic state")
            }
            Self::FormUsageMismatch => formatter.write_str("shared vector Form usage differs"),
            Self::TextExtractionMismatch => {
                formatter.write_str("tagged-PDF accessible text differs")
            }
        }
    }
}

impl std::error::Error for TaggedPdfIndependentErrorV2 {}

pub fn inspect_tagged_pdf_v2(
    pdf: &[u8],
    expected: &TaggedPdfIndependentExpectationsV2,
) -> Result<TaggedPdfIndependentReportV2, TaggedPdfIndependentErrorV2> {
    if !pdf.starts_with(b"%PDF-1.7\n")
        || !contains(pdf, b"xref\n")
        || !contains(pdf, b"trailer\n")
        || !pdf.ends_with(b"%%EOF\n")
    {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    let byte_length =
        u64::try_from(pdf.len()).map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?;
    if byte_length != expected.closure.pdf_byte_length || sha256(pdf) != expected.closure.pdf_sha256
    {
        return Err(TaggedPdfIndependentErrorV2::PdfHashMismatch);
    }
    let objects = parse_objects(pdf).map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?;
    validate_serialization_envelope_v2(pdf, &objects, &expected.closure)?;
    validate_object_closure(&objects, &expected.closure)?;
    let object_map = objects
        .iter()
        .map(|object| (object.number, object.body))
        .collect::<BTreeMap<_, _>>();

    validate_catalog(&object_map, expected)?;
    validate_outlines(&object_map, expected)?;
    validate_pages(&object_map, expected)?;
    validate_metadata(&object_map, &expected.closure)?;
    let form_objects = expected
        .closure
        .objects
        .iter()
        .filter(|object| object.role.starts_with("vector_form:"))
        .map(|object| object.object_number)
        .collect::<Vec<_>>();
    if u32::try_from(form_objects.len()) != Ok(expected.form_count) {
        return Err(TaggedPdfIndependentErrorV2::FormUsageMismatch);
    }
    validate_forms(&object_map, &form_objects)?;
    let equation_font_max_cids = validate_equation_fonts(&object_map, expected)?;

    let mut extracted_text = Vec::new();
    let mut form_do_count = 0u32;
    for page_index in 0..expected.page_count {
        form_do_count = form_do_count
            .checked_add(validate_page(
                &object_map,
                expected,
                page_index,
                &form_objects,
                &equation_font_max_cids,
                &mut extracted_text,
            )?)
            .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
    }
    validate_structure(&object_map, expected)?;
    validate_parent_tree(&object_map, expected)?;
    let expected_do_count = u32::try_from(expected.vectors.len())
        .map_err(|_| TaggedPdfIndependentErrorV2::InvalidExpectation)?;
    if form_do_count != expected_do_count {
        return Err(TaggedPdfIndependentErrorV2::FormUsageMismatch);
    }
    let expected_text = expected_accessible_text(expected);
    if extracted_text != expected_text {
        return Err(TaggedPdfIndependentErrorV2::TextExtractionMismatch);
    }

    Ok(TaggedPdfIndependentReportV2 {
        validator_algorithm: TAGGED_PDF_INDEPENDENT_VALIDATOR_ALGORITHM_V2,
        pdf_sha256: sha256(pdf),
        object_count: expected.closure.object_count,
        page_count: expected.page_count,
        form_count: expected.form_count,
        vector_count: expected_do_count,
        equation_number_count: u32::try_from(expected.equation_numbers.len())
            .map_err(|_| TaggedPdfIndependentErrorV2::InvalidExpectation)?,
        form_do_count,
        extracted_text,
    })
}

fn validate_serialization_envelope_v2(
    pdf: &[u8],
    objects: &[ParsedObject<'_>],
    closure: &TaggedPdfIndependentClosureV2,
) -> Result<(), TaggedPdfIndependentErrorV2> {
    let xref_count = closure
        .object_count
        .checked_add(1)
        .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
    let info = closure
        .object_for_role("info")
        .ok_or(TaggedPdfIndependentErrorV2::InvalidClosure)?;
    let mut reconstructed = Vec::new();
    reconstructed
        .try_reserve_exact(pdf.len())
        .map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?;
    reconstructed.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(objects.len())
        .map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?;
    for object in objects {
        offsets.push(reconstructed.len());
        reconstructed.extend_from_slice(format!("{} 0 obj\n", object.number).as_bytes());
        reconstructed.extend_from_slice(object.body);
        reconstructed.extend_from_slice(b"\nendobj\n");
    }
    let xref = reconstructed.len();
    reconstructed.extend_from_slice(format!("xref\n0 {xref_count}\n").as_bytes());
    reconstructed.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        if offset > 9_999_999_999 {
            return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
        }
        reconstructed.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    reconstructed.extend_from_slice(
        format!(
            "trailer\n<< /Size {xref_count} /Root 1 0 R /Info {info} 0 R >>\nstartxref\n{xref}\n%%EOF\n"
        )
        .as_bytes(),
    );
    if reconstructed != pdf {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    Ok(())
}

fn validate_object_closure(
    objects: &[ParsedObject<'_>],
    closure: &TaggedPdfIndependentClosureV2,
) -> Result<(), TaggedPdfIndependentErrorV2> {
    if usize::try_from(closure.object_count) != Ok(objects.len())
        || objects
            .iter()
            .zip(&closure.objects)
            .any(|(object, expected)| {
                object.number != expected.object_number || sha256(object.body) != expected.sha256
            })
    {
        return Err(TaggedPdfIndependentErrorV2::ObjectClosureMismatch);
    }
    Ok(())
}

fn validate_catalog(
    objects: &BTreeMap<u32, &[u8]>,
    expected: &TaggedPdfIndependentExpectationsV2,
) -> Result<(), TaggedPdfIndependentErrorV2> {
    let catalog_number = expected
        .closure
        .object_for_role("catalog")
        .ok_or(TaggedPdfIndependentErrorV2::InvalidClosure)?;
    let structure_root = expected
        .closure
        .object_for_role("structure_tree_root")
        .ok_or(TaggedPdfIndependentErrorV2::InvalidClosure)?;
    let pages = role_object(&expected.closure, "pages")?;
    let destinations = role_object(&expected.closure, "destinations")?;
    let metadata = role_object(&expected.closure, "metadata")?;
    let catalog = object(objects, catalog_number)?;
    let language = format!("/Lang <{}>", utf16be_hex(&expected.document_language));
    let structure = format!("/StructTreeRoot {structure_root} 0 R");
    let pages_ref = format!("/Pages {pages} 0 R");
    let destinations_ref = format!("/Names << /Dests {destinations} 0 R >>");
    let metadata_ref = format!("/Metadata {metadata} 0 R");
    let outline_matches = match expected.closure.object_for_role("outline_root") {
        Some(outline) => contains(catalog, format!("/Outlines {outline} 0 R").as_bytes()),
        None => !contains(catalog, b"/Outlines "),
    };
    if catalog_number != 1
        || !contains(catalog, b"/Type /Catalog")
        || !contains(catalog, pages_ref.as_bytes())
        || !contains(catalog, destinations_ref.as_bytes())
        || !contains(catalog, metadata_ref.as_bytes())
        || !contains(catalog, b"/MarkInfo << /Marked true >>")
        || !contains(catalog, b"/ViewerPreferences << /DisplayDocTitle true >>")
        || !contains(catalog, language.as_bytes())
        || !contains(catalog, structure.as_bytes())
        || !outline_matches
    {
        return Err(TaggedPdfIndependentErrorV2::CatalogMismatch);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParsedOutlineItemV2 {
    parent: u32,
    previous: Option<u32>,
    next: Option<u32>,
    first: Option<u32>,
    last: Option<u32>,
    structure_element: u32,
}

fn validate_outlines(
    objects: &BTreeMap<u32, &[u8]>,
    expected: &TaggedPdfIndependentExpectationsV2,
) -> Result<(), TaggedPdfIndependentErrorV2> {
    let destination_names = validate_destination_names_v2(objects, expected)?;
    let item_indices = indexed_roles(&expected.closure, "outline_item")
        .ok_or(TaggedPdfIndependentErrorV2::InvalidClosure)?;
    let Some(root_number) = expected.closure.object_for_role("outline_root") else {
        return if item_indices.is_empty() {
            Ok(())
        } else {
            Err(TaggedPdfIndependentErrorV2::InvalidClosure)
        };
    };
    if item_indices.is_empty() {
        return Err(TaggedPdfIndependentErrorV2::InvalidClosure);
    }
    let item_numbers = item_indices
        .iter()
        .map(|index| role_object(&expected.closure, &format!("outline_item:{index}")))
        .collect::<Result<Vec<_>, _>>()?;
    let item_set = item_numbers.iter().copied().collect::<BTreeSet<_>>();
    let structure_set = indexed_roles(&expected.closure, "structure_element")
        .ok_or(TaggedPdfIndependentErrorV2::InvalidClosure)?
        .into_iter()
        .map(|index| role_object(&expected.closure, &format!("structure_element:{index}")))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let root = object(objects, root_number)?;
    if count_bytes(root, b"/Type /Outlines")? != 1
        || parse_u32_after(root, b"/Count ")? != item_numbers.len() as u32
    {
        return Err(TaggedPdfIndependentErrorV2::OutlineMismatch);
    }

    let mut parsed = BTreeMap::new();
    for number in &item_numbers {
        let body = object(objects, *number)?;
        let title = parse_hex_string_after(body, b"/Title ")?;
        let destination = parse_pdf_literal_after(body, b"/Dest ")?;
        if !title.starts_with(b"FEFF")
            || title.len() <= 4
            || (title.len() - 4) % 4 != 0
            || !destination_names.contains(&destination.decoded)
        {
            return Err(TaggedPdfIndependentErrorV2::OutlineMismatch);
        }
        let item = ParsedOutlineItemV2 {
            parent: parse_reference_after(body, b"/Parent ")?,
            previous: parse_optional_reference_after(body, b"/Prev ")?,
            next: parse_optional_reference_after(body, b"/Next ")?,
            first: parse_optional_reference_after(body, b"/First ")?,
            last: parse_optional_reference_after(body, b"/Last ")?,
            structure_element: parse_reference_after(body, b"/SE ")?,
        };
        if (item.parent != root_number && !item_set.contains(&item.parent))
            || !structure_set.contains(&item.structure_element)
            || item
                .previous
                .is_some_and(|value| !item_set.contains(&value))
            || item.next.is_some_and(|value| !item_set.contains(&value))
            || item.first.is_some_and(|value| !item_set.contains(&value))
            || item.last.is_some_and(|value| !item_set.contains(&value))
            || parsed.insert(*number, item).is_some()
        {
            return Err(TaggedPdfIndependentErrorV2::OutlineMismatch);
        }
    }

    // Every item must terminate at the outline root; this also rejects parent cycles.
    let mut descendant_counts = BTreeMap::<u32, u32>::new();
    for number in &item_numbers {
        let mut parent = parsed
            .get(number)
            .ok_or(TaggedPdfIndependentErrorV2::OutlineMismatch)?
            .parent;
        for _ in 0..=item_numbers.len() {
            if parent == root_number {
                break;
            }
            let count = descendant_counts.entry(parent).or_default();
            *count = count
                .checked_add(1)
                .ok_or(TaggedPdfIndependentErrorV2::OutlineMismatch)?;
            parent = parsed
                .get(&parent)
                .ok_or(TaggedPdfIndependentErrorV2::OutlineMismatch)?
                .parent;
        }
        if parent != root_number {
            return Err(TaggedPdfIndependentErrorV2::OutlineMismatch);
        }
    }

    let mut children = BTreeMap::<u32, Vec<u32>>::new();
    for (number, item) in &parsed {
        children.entry(item.parent).or_default().push(*number);
    }
    validate_outline_siblings_v2(root_number, root, &children, &parsed)?;
    for number in &item_numbers {
        let body = object(objects, *number)?;
        validate_outline_siblings_v2(*number, body, &children, &parsed)?;
        let descendants = descendant_counts.get(number).copied().unwrap_or(0);
        match children.get(number) {
            Some(values) if !values.is_empty() => {
                if parse_u32_after(body, b"/Count ")? != descendants {
                    return Err(TaggedPdfIndependentErrorV2::OutlineMismatch);
                }
            }
            _ if contains(body, b"/Count ") => {
                return Err(TaggedPdfIndependentErrorV2::OutlineMismatch);
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_destination_names_v2(
    objects: &BTreeMap<u32, &[u8]>,
    expected: &TaggedPdfIndependentExpectationsV2,
) -> Result<BTreeSet<Vec<u8>>, TaggedPdfIndependentErrorV2> {
    let invalid = TaggedPdfIndependentErrorV2::OutlineMismatch;
    let body = object(objects, role_object(&expected.closure, "destinations")?)?;
    let mut remaining = body
        .strip_prefix(b"<< /Names [")
        .and_then(|body| body.strip_suffix(b"] >>"))
        .ok_or(invalid)?;
    let pages = indexed_roles(&expected.closure, "page")
        .ok_or(TaggedPdfIndependentErrorV2::InvalidClosure)?
        .into_iter()
        .map(|index| role_object(&expected.closure, &format!("page:{index}")))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut names = BTreeSet::new();
    while !remaining.is_empty() {
        let name = parse_pdf_literal_at(remaining, 0)?;
        if name.decoded.is_empty()
            || !name.decoded.is_ascii()
            || name.decoded.iter().any(u8::is_ascii_control)
            || names
                .last()
                .is_some_and(|previous| previous >= &name.decoded)
        {
            return Err(invalid);
        }
        remaining = remaining
            .get(name.raw.len()..)
            .and_then(|body| body.strip_prefix(b" ["))
            .ok_or(invalid)?;
        let view_end = find_from(remaining, b"] ", 0).ok_or(invalid)?;
        let tokens = std::str::from_utf8(&remaining[..view_end])
            .map_err(|_| invalid)?
            .split(' ')
            .collect::<Vec<_>>();
        let Some(page) = tokens.first().and_then(|value| value.parse::<u32>().ok()) else {
            return Err(invalid);
        };
        let number = |value: &str| {
            let unsigned = value.strip_prefix('-').unwrap_or(value);
            let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
            !whole.is_empty()
                && whole.bytes().all(|byte| byte.is_ascii_digit())
                && (whole.len() == 1 || !whole.starts_with('0'))
                && if unsigned.contains('.') {
                    !fraction.is_empty()
                        && fraction.len() <= 16
                        && !fraction.ends_with('0')
                        && fraction.bytes().all(|byte| byte.is_ascii_digit())
                } else {
                    value != "-0"
                }
        };
        let view_matches = match tokens.get(3..) {
            Some(["/XYZ", x, y, "null"]) => number(x) && number(y),
            Some(["/Fit"]) => true,
            Some(["/FitH", top]) => *top == "null" || number(top),
            _ => false,
        };
        if tokens.first().copied() != Some(page.to_string().as_str())
            || tokens.get(1..3) != Some(["0", "R"].as_slice())
            || !pages.contains(&page)
            || !view_matches
        {
            return Err(invalid);
        }
        names.insert(name.decoded);
        remaining = &remaining[view_end + 2..];
    }
    Ok(names)
}

fn validate_outline_siblings_v2(
    parent_number: u32,
    parent_body: &[u8],
    children: &BTreeMap<u32, Vec<u32>>,
    parsed: &BTreeMap<u32, ParsedOutlineItemV2>,
) -> Result<(), TaggedPdfIndependentErrorV2> {
    let direct = children
        .get(&parent_number)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if direct.is_empty() {
        if contains(parent_body, b"/First ") || contains(parent_body, b"/Last ") {
            return Err(TaggedPdfIndependentErrorV2::OutlineMismatch);
        }
        return Ok(());
    }
    let first = parse_reference_after(parent_body, b"/First ")?;
    let last = parse_reference_after(parent_body, b"/Last ")?;
    let direct_set = direct.iter().copied().collect::<BTreeSet<_>>();
    let mut visited = BTreeSet::new();
    let mut current = Some(first);
    let mut previous = None;
    while let Some(number) = current {
        if !direct_set.contains(&number) || !visited.insert(number) {
            return Err(TaggedPdfIndependentErrorV2::OutlineMismatch);
        }
        let item = parsed
            .get(&number)
            .ok_or(TaggedPdfIndependentErrorV2::OutlineMismatch)?;
        if item.parent != parent_number || item.previous != previous {
            return Err(TaggedPdfIndependentErrorV2::OutlineMismatch);
        }
        previous = Some(number);
        current = item.next;
    }
    if previous != Some(last) || visited != direct_set {
        return Err(TaggedPdfIndependentErrorV2::OutlineMismatch);
    }
    Ok(())
}

fn validate_pages(
    objects: &BTreeMap<u32, &[u8]>,
    expected: &TaggedPdfIndependentExpectationsV2,
) -> Result<(), TaggedPdfIndependentErrorV2> {
    let pages_number = role_object(&expected.closure, "pages")?;
    let pages = object(objects, pages_number)?;
    let mut kids = String::from("/Kids [");
    for page_index in 0..expected.page_count {
        kids.push_str(&format!(
            "{} 0 R ",
            role_object(&expected.closure, &format!("page:{page_index}"))?
        ));
    }
    kids.push(']');
    let count = format!("/Count {}", expected.page_count);
    if !contains(pages, b"/Type /Pages")
        || !contains(pages, count.as_bytes())
        || !contains(pages, kids.as_bytes())
    {
        return Err(TaggedPdfIndependentErrorV2::PageMismatch);
    }
    Ok(())
}

fn validate_metadata(
    objects: &BTreeMap<u32, &[u8]>,
    closure: &TaggedPdfIndependentClosureV2,
) -> Result<(), TaggedPdfIndependentErrorV2> {
    let metadata = object(objects, role_object(closure, "metadata")?)?;
    let stream = object_stream(metadata)
        .map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?
        .ok_or(TaggedPdfIndependentErrorV2::MetadataMismatch)?;
    if !contains(metadata, b"/Type /Metadata")
        || !contains(metadata, b"/Subtype /XML")
        || sha256(stream) != closure.xmp_sha256
    {
        return Err(TaggedPdfIndependentErrorV2::MetadataMismatch);
    }
    Ok(())
}

fn validate_forms(
    objects: &BTreeMap<u32, &[u8]>,
    form_objects: &[u32],
) -> Result<(), TaggedPdfIndependentErrorV2> {
    for number in form_objects {
        let body = object(objects, *number)?;
        if !contains(body, b"/Type /XObject") || !contains(body, b"/Subtype /Form") {
            return Err(TaggedPdfIndependentErrorV2::FormUsageMismatch);
        }
        let stream = object_stream(body)
            .map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?
            .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
        if [
            b"/MCID".as_slice(),
            b"/Alt",
            b"/ActualText",
            b"/Lang",
            b"BDC",
            b"BMC",
        ]
        .iter()
        .any(|needle| contains(stream, needle))
        {
            return Err(TaggedPdfIndependentErrorV2::FormSemanticContent);
        }
    }
    Ok(())
}

fn validate_equation_fonts(
    objects: &BTreeMap<u32, &[u8]>,
    expected: &TaggedPdfIndependentExpectationsV2,
) -> Result<BTreeMap<u32, u16>, TaggedPdfIndependentErrorV2> {
    let mut max_cids = BTreeMap::new();
    let font_indices = expected
        .equation_numbers
        .iter()
        .map(|number| number.font_index)
        .collect::<BTreeSet<_>>();
    for index in font_indices {
        let type0 = role_object(&expected.closure, &format!("equation_font_type0:{index}"))?;
        let cid = role_object(&expected.closure, &format!("equation_font_cid:{index}"))?;
        let descriptor = role_object(
            &expected.closure,
            &format!("equation_font_descriptor:{index}"),
        )?;
        let program = role_object(&expected.closure, &format!("equation_font_program:{index}"))?;
        let to_unicode = role_object(
            &expected.closure,
            &format!("equation_font_to_unicode:{index}"),
        )?;
        let cid_to_gid = role_object(
            &expected.closure,
            &format!("equation_font_cid_to_gid:{index}"),
        )?;
        let type0_body = object(objects, type0)?;
        let cid_body = object(objects, cid)?;
        let descriptor_body = object(objects, descriptor)?;
        if !contains(type0_body, b"/Type /Font")
            || !contains(type0_body, b"/Subtype /Type0")
            || !contains(type0_body, b"/Encoding /Identity-H")
            || !contains(
                type0_body,
                format!("/DescendantFonts [{cid} 0 R]").as_bytes(),
            )
            || !contains(
                type0_body,
                format!("/ToUnicode {to_unicode} 0 R").as_bytes(),
            )
            || !contains(cid_body, b"/Subtype /CIDFontType2")
            || !contains(
                cid_body,
                format!("/FontDescriptor {descriptor} 0 R").as_bytes(),
            )
            || !contains(
                cid_body,
                format!("/CIDToGIDMap {cid_to_gid} 0 R").as_bytes(),
            )
            || !contains(
                descriptor_body,
                format!("/FontFile2 {program} 0 R").as_bytes(),
            )
        {
            return Err(TaggedPdfIndependentErrorV2::TextExtractionMismatch);
        }
        let font_program = object_stream(object(objects, program)?)
            .map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?
            .ok_or(TaggedPdfIndependentErrorV2::TextExtractionMismatch)?;
        let cmap = object_stream(object(objects, to_unicode)?)
            .map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?
            .ok_or(TaggedPdfIndependentErrorV2::TextExtractionMismatch)?;
        let gid_map = object_stream(object(objects, cid_to_gid)?)
            .map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?
            .ok_or(TaggedPdfIndependentErrorV2::TextExtractionMismatch)?;
        if font_program.get(..4) != Some(&0x0001_0000u32.to_be_bytes())
            || gid_map.len() < 4
            || gid_map.len() % 2 != 0
            || gid_map.get(..2) != Some(&[0, 0])
        {
            return Err(TaggedPdfIndependentErrorV2::TextExtractionMismatch);
        }
        let cid_count = gid_map
            .len()
            .checked_div(2)
            .and_then(|count| count.checked_sub(1))
            .and_then(|count| u16::try_from(count).ok())
            .ok_or(TaggedPdfIndependentErrorV2::TextExtractionMismatch)?;
        if cid_count == 0 || max_cids.insert(index, cid_count).is_some() {
            return Err(TaggedPdfIndependentErrorV2::TextExtractionMismatch);
        }
        validate_equation_cmap(cmap, cid_count)?;
    }
    Ok(max_cids)
}

fn validate_equation_cmap(cmap: &[u8], max_cid: u16) -> Result<(), TaggedPdfIndependentErrorV2> {
    let invalid = TaggedPdfIndependentErrorV2::TextExtractionMismatch;
    let text = std::str::from_utf8(cmap).map_err(|_| invalid)?;
    let mappings = text
        .strip_prefix(concat!(
            "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n",
            "/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n",
            "/CMapName /Typaxis-Identity-UCS def\n/CMapType 2 def\n",
            "1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
        ))
        .and_then(|body| {
            body.strip_suffix("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n")
        })
        .ok_or(invalid)?;
    // Empty mappings are valid when every shaped cluster uses ActualText.
    // Page validation independently requires an exact ActualText scope around
    // every equation-number glyph, so no extraction is silently lost here.
    let mut lines = mappings.lines();
    let mut previous_cid = 0u16;
    while let Some(header) = lines.next() {
        let count_text = header.strip_suffix(" beginbfchar").ok_or(invalid)?;
        let count = count_text.parse::<u16>().map_err(|_| invalid)?;
        if !(1..=100).contains(&count) || count.to_string() != count_text {
            return Err(invalid);
        }
        for _ in 0..count {
            let line = lines.next().ok_or(invalid)?;
            let (source, destination) = line
                .strip_prefix('<')
                .and_then(|line| line.strip_suffix('>'))
                .and_then(|line| line.split_once("> <"))
                .ok_or(invalid)?;
            let canonical_hex = |value: &str| {
                value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
            };
            if source.len() != 4
                || destination.is_empty()
                || destination.len() % 4 != 0
                || !canonical_hex(source)
                || !canonical_hex(destination)
            {
                return Err(invalid);
            }
            let cid = u16::from_str_radix(source, 16).map_err(|_| invalid)?;
            if cid <= previous_cid || cid > max_cid {
                return Err(invalid);
            }
            let units = destination
                .as_bytes()
                .chunks_exact(4)
                .map(|chunk| {
                    let text = std::str::from_utf8(chunk).map_err(|_| invalid)?;
                    u16::from_str_radix(text, 16).map_err(|_| invalid)
                })
                .collect::<Result<Vec<_>, _>>()?;
            String::from_utf16(&units).map_err(|_| invalid)?;
            previous_cid = cid;
        }
        if lines.next() != Some("endbfchar") {
            return Err(invalid);
        }
    }
    Ok(())
}

fn validate_page(
    objects: &BTreeMap<u32, &[u8]>,
    expected: &TaggedPdfIndependentExpectationsV2,
    page_index: u32,
    form_objects: &[u32],
    equation_font_max_cids: &BTreeMap<u32, u16>,
    extracted_text: &mut Vec<String>,
) -> Result<u32, TaggedPdfIndependentErrorV2> {
    let page_number = role_object(&expected.closure, &format!("page:{page_index}"))?;
    let content_number = role_object(&expected.closure, &format!("page_content:{page_index}"))?;
    let page = object(objects, page_number)?;
    let contents = format!("/Contents {content_number} 0 R");
    let struct_parents = format!("/StructParents {page_index}");
    let parent = format!("/Parent {} 0 R", role_object(&expected.closure, "pages")?);
    if !contains(page, b"/Type /Page")
        || contains(page, b"/Type /Pages")
        || !contains(page, parent.as_bytes())
        || !contains(page, contents.as_bytes())
        || !contains(page, struct_parents.as_bytes())
    {
        return Err(TaggedPdfIndependentErrorV2::PageMismatch);
    }
    let stream = object_stream(object(objects, content_number)?)
        .map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?
        .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
    let mut records = Vec::new();
    records.extend(
        expected
            .vectors
            .iter()
            .filter(|record| record.page_index == page_index)
            .map(PageRecord::Vector),
    );
    records.extend(
        expected
            .equation_numbers
            .iter()
            .filter(|record| record.page_index == page_index)
            .map(PageRecord::EquationNumber),
    );
    records.sort_by_key(PageRecord::mcid);
    if count_bytes(stream, b"/MCID ")? != records.len() {
        return Err(TaggedPdfIndependentErrorV2::MarkedContentMismatch);
    }
    let mut cursor = 0usize;
    let mut do_count = 0u32;
    for record in records {
        match record {
            PageRecord::Vector(vector) => {
                let outer = format!("/{} << /MCID {} >> BDC\n", vector.kind.role(), vector.mcid);
                let start = exact_next(stream, outer.as_bytes(), cursor)?;
                let inner = property_span(vector.actual_text(), vector.paint_language());
                let paint_start = start
                    .checked_add(outer.len())
                    .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
                let (body_start, closing) = if let Some(inner) = inner {
                    if stream.get(paint_start..paint_start + inner.len()) != Some(inner.as_bytes())
                    {
                        return Err(TaggedPdfIndependentErrorV2::MarkedContentMismatch);
                    }
                    (paint_start + inner.len(), b"EMC\nEMC\n".as_slice())
                } else {
                    (paint_start, b"EMC\n".as_slice())
                };
                let end = find_from(stream, closing, body_start)
                    .ok_or(TaggedPdfIndependentErrorV2::MarkedContentMismatch)?;
                let paint = &stream[body_start..end];
                let resource_name = single_do_resource(paint)?;
                let target = page_xobject_target(page, &resource_name)?;
                let expected_form = form_objects
                    .get(vector.form_index as usize)
                    .copied()
                    .ok_or(TaggedPdfIndependentErrorV2::InvalidExpectation)?;
                if target != expected_form
                    || [
                        b"/MCID".as_slice(),
                        b"/Alt",
                        b"/ActualText",
                        b"/Lang",
                        b"BDC",
                        b"BMC",
                        b"EMC",
                        b"BT ",
                    ]
                    .iter()
                    .any(|needle| contains(paint, needle))
                {
                    return Err(TaggedPdfIndependentErrorV2::FormUsageMismatch);
                }
                if let Some(actual_text) = vector.actual_text() {
                    extracted_text.push(actual_text.to_owned());
                }
                do_count = do_count
                    .checked_add(1)
                    .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
                cursor = end
                    .checked_add(closing.len())
                    .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
            }
            PageRecord::EquationNumber(number) => {
                let mut properties = format!("<< /MCID {}", number.mcid);
                if let Some(language) = number.paint_language() {
                    properties.push_str(&format!(" /Lang <{}>", utf16be_hex(language)));
                }
                properties.push_str(" >>");
                let outer = format!("/Span {properties} BDC\n");
                let start = exact_next(stream, outer.as_bytes(), cursor)?;
                let inner = format!(
                    "/Span << /ActualText <{}> >> BDC\n",
                    utf16be_hex(&number.exact_text)
                );
                let inner_start = start
                    .checked_add(outer.len())
                    .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
                if stream.get(inner_start..inner_start + inner.len()) != Some(inner.as_bytes()) {
                    return Err(TaggedPdfIndependentErrorV2::TextExtractionMismatch);
                }
                let body_start = inner_start
                    .checked_add(inner.len())
                    .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
                let closing = b"EMC\nEMC\n";
                let end = find_from(stream, closing, body_start)
                    .ok_or(TaggedPdfIndependentErrorV2::MarkedContentMismatch)?;
                let paint = &stream[body_start..end];
                let font_name = format!("F{}", number.font_index);
                let font_target = page_xobject_target(page, font_name.as_bytes())?;
                let expected_font = role_object(
                    &expected.closure,
                    &format!("equation_font_type0:{}", number.font_index),
                )?;
                let show_count = count_bytes(paint, b" Tj\n")?;
                let shown_cids = equation_shown_cids_v2(paint)?;
                let max_cid = equation_font_max_cids
                    .get(&number.font_index)
                    .copied()
                    .ok_or(TaggedPdfIndependentErrorV2::TextExtractionMismatch)?;
                if count_bytes(paint, b"BT ")? != 1
                    || show_count == 0
                    || count_bytes(paint, b"> Tj\n")? != show_count
                    || shown_cids.len() != show_count
                    || shown_cids.iter().any(|cid| *cid == 0 || *cid > max_cid)
                    || !contains(paint, format!("BT /{font_name} ").as_bytes())
                    || font_target != expected_font
                    || contains(paint, b" Do\n")
                    || [
                        b"/MCID".as_slice(),
                        b"/Alt",
                        b"/ActualText",
                        b"/Lang",
                        b"BDC",
                        b"BMC",
                        b"EMC",
                    ]
                    .iter()
                    .any(|needle| contains(paint, needle))
                {
                    return Err(TaggedPdfIndependentErrorV2::TextExtractionMismatch);
                }
                extracted_text.push(number.exact_text.clone());
                cursor = end
                    .checked_add(closing.len())
                    .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
            }
        }
    }
    let do_count_usize =
        usize::try_from(do_count).map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?;
    if count_bytes(stream, b" Do\n")? != do_count_usize || contains(&stream[cursor..], b"/MCID ") {
        return Err(TaggedPdfIndependentErrorV2::MarkedContentMismatch);
    }
    Ok(do_count)
}

fn equation_shown_cids_v2(paint: &[u8]) -> Result<Vec<u16>, TaggedPdfIndependentErrorV2> {
    let mut cids = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative_end) = paint[cursor..]
        .windows(5)
        .position(|value| value == b"> Tj\n")
    {
        let end = cursor
            .checked_add(relative_end)
            .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
        let start = end
            .checked_sub(5)
            .ok_or(TaggedPdfIndependentErrorV2::TextExtractionMismatch)?;
        let operand = paint
            .get(start..=end)
            .ok_or(TaggedPdfIndependentErrorV2::TextExtractionMismatch)?;
        if operand.first() != Some(&b'<')
            || operand.len() != 6
            || !operand[1..5]
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(byte))
        {
            return Err(TaggedPdfIndependentErrorV2::TextExtractionMismatch);
        }
        let text = std::str::from_utf8(&operand[1..5])
            .map_err(|_| TaggedPdfIndependentErrorV2::TextExtractionMismatch)?;
        cids.push(
            u16::from_str_radix(text, 16)
                .map_err(|_| TaggedPdfIndependentErrorV2::TextExtractionMismatch)?,
        );
        cursor = end
            .checked_add(5)
            .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
    }
    Ok(cids)
}

fn validate_structure(
    objects: &BTreeMap<u32, &[u8]>,
    expected: &TaggedPdfIndependentExpectationsV2,
) -> Result<(), TaggedPdfIndependentErrorV2> {
    let structure_root = role_object(&expected.closure, "structure_tree_root")?;
    let parent_tree = role_object(&expected.closure, "structure_parent_tree")?;
    let root = object(objects, structure_root)?;
    let parent_ref = format!("/ParentTree {parent_tree} 0 R");
    if !contains(root, b"/Type /StructTreeRoot")
        || !contains(root, b"/RoleMap <<")
        || !contains(root, parent_ref.as_bytes())
    {
        return Err(TaggedPdfIndependentErrorV2::StructureMismatch);
    }
    validate_structure_ids_v2(objects, &expected.closure, root)?;
    for vector in &expected.vectors {
        let number = structure_object(&expected.closure, vector.structure_node_id)?;
        let body = object(objects, number)?;
        let page = role_object(&expected.closure, &format!("page:{}", vector.page_index))?;
        let role = format!("/S /{}", vector.kind.role());
        let alternative = format!("/Alt <{}>", utf16be_hex(&vector.alternative));
        let mcr = format!("<< /Type /MCR /Pg {page} 0 R /MCID {} >>", vector.mcid);
        if !contains(body, b"/Type /StructElem")
            || !contains(body, role.as_bytes())
            || !contains(body, alternative.as_bytes())
            || contains(body, b"/ActualText")
        {
            return Err(TaggedPdfIndependentErrorV2::StructureMismatch);
        }
        validate_optional_structure_language(body, vector.structure_language())?;
        if let Some(equation) = expected
            .equation_numbers
            .iter()
            .find(|equation| equation.parent_structure_node_id == vector.structure_node_id)
        {
            let equation_object = structure_object(&expected.closure, equation.structure_node_id)?;
            let ordered = format!("/K [{mcr} {equation_object} 0 R ");
            if !contains(body, ordered.as_bytes()) {
                return Err(TaggedPdfIndependentErrorV2::StructureMismatch);
            }
        } else if !contains(body, mcr.as_bytes()) {
            return Err(TaggedPdfIndependentErrorV2::StructureMismatch);
        }
    }
    for number in &expected.equation_numbers {
        let object_number = structure_object(&expected.closure, number.structure_node_id)?;
        let parent = structure_object(&expected.closure, number.parent_structure_node_id)?;
        let page = role_object(&expected.closure, &format!("page:{}", number.page_index))?;
        let body = object(objects, object_number)?;
        let parent_ref = format!("/P {parent} 0 R");
        let mcr = format!("<< /Type /MCR /Pg {page} 0 R /MCID {} >>", number.mcid);
        if !contains(body, b"/Type /StructElem /S /Span")
            || !contains(body, parent_ref.as_bytes())
            || !contains(body, mcr.as_bytes())
            || contains(body, b"/Alt")
            || contains(body, b"/ActualText")
        {
            return Err(TaggedPdfIndependentErrorV2::StructureMismatch);
        }
        validate_optional_structure_language(body, number.structure_language())?;
    }
    Ok(())
}

fn validate_structure_ids_v2(
    objects: &BTreeMap<u32, &[u8]>,
    closure: &TaggedPdfIndependentClosureV2,
    root: &[u8],
) -> Result<(), TaggedPdfIndependentErrorV2> {
    let indices = indexed_roles(closure, "structure_element")
        .ok_or(TaggedPdfIndependentErrorV2::InvalidClosure)?;
    let mut ids = Vec::new();
    for index in indices {
        let number = structure_object(closure, index)?;
        let body = object(objects, number)?;
        if contains(body, b"/ID ") {
            let literal = parse_pdf_literal_after(body, b"/ID ")?;
            if literal.decoded.is_empty() {
                return Err(TaggedPdfIndependentErrorV2::StructureMismatch);
            }
            ids.push((literal.decoded, literal.raw, number));
        }
    }
    ids.sort_by(|left, right| left.0.cmp(&right.0));
    if ids.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(TaggedPdfIndependentErrorV2::StructureMismatch);
    }
    let Some(id_tree) = closure.object_for_role("structure_id_tree") else {
        return if ids.is_empty() && !contains(root, b"/IDTree ") {
            Ok(())
        } else {
            Err(TaggedPdfIndependentErrorV2::StructureMismatch)
        };
    };
    if ids.is_empty() || parse_reference_after(root, b"/IDTree ")? != id_tree {
        return Err(TaggedPdfIndependentErrorV2::StructureMismatch);
    }
    let mut expected = b"<< /Names [".to_vec();
    for (_, literal, number) in ids {
        expected.extend_from_slice(literal);
        expected.extend_from_slice(format!(" {number} 0 R ").as_bytes());
    }
    expected.extend_from_slice(b"] >>");
    if object(objects, id_tree)? != expected {
        return Err(TaggedPdfIndependentErrorV2::StructureMismatch);
    }
    Ok(())
}

fn validate_parent_tree(
    objects: &BTreeMap<u32, &[u8]>,
    expected: &TaggedPdfIndependentExpectationsV2,
) -> Result<(), TaggedPdfIndependentErrorV2> {
    let parent_tree = role_object(&expected.closure, "structure_parent_tree")?;
    let mut value = String::from("<< /Nums [");
    for page in 0..expected.page_count {
        value.push_str(&format!("{page} ["));
        let mut records = expected
            .vectors
            .iter()
            .filter(|record| record.page_index == page)
            .map(|record| (record.mcid, record.structure_node_id))
            .chain(
                expected
                    .equation_numbers
                    .iter()
                    .filter(|record| record.page_index == page)
                    .map(|record| (record.mcid, record.structure_node_id)),
            )
            .collect::<Vec<_>>();
        records.sort_unstable();
        for (_, node) in records {
            value.push_str(&format!(
                "{} 0 R ",
                structure_object(&expected.closure, node)?
            ));
        }
        value.push_str("] ");
    }
    value.push_str("] >>");
    if object(objects, parent_tree)? != value.as_bytes() {
        return Err(TaggedPdfIndependentErrorV2::ParentTreeMismatch);
    }
    Ok(())
}

fn validate_optional_structure_language(
    body: &[u8],
    expected: Option<&str>,
) -> Result<(), TaggedPdfIndependentErrorV2> {
    match expected {
        Some(language) => {
            let value = format!("/Lang <{}>", utf16be_hex(language));
            if count_bytes(body, b"/Lang ")? != 1 || !contains(body, value.as_bytes()) {
                return Err(TaggedPdfIndependentErrorV2::StructureMismatch);
            }
        }
        None if contains(body, b"/Lang ") => {
            return Err(TaggedPdfIndependentErrorV2::StructureMismatch);
        }
        None => {}
    }
    Ok(())
}

fn expected_accessible_text(expected: &TaggedPdfIndependentExpectationsV2) -> Vec<String> {
    let mut records = expected
        .vectors
        .iter()
        .filter_map(|record| {
            record
                .actual_text
                .as_ref()
                .map(|text| (record.page_index, record.mcid, text.clone()))
        })
        .chain(
            expected
                .equation_numbers
                .iter()
                .map(|record| (record.page_index, record.mcid, record.exact_text.clone())),
        )
        .collect::<Vec<_>>();
    records.sort_by_key(|(page, mcid, _)| (*page, *mcid));
    records.into_iter().map(|(_, _, text)| text).collect()
}

enum PageRecord<'a> {
    Vector(&'a TaggedVectorExpectationV2),
    EquationNumber(&'a TaggedEquationNumberExpectationV2),
}

impl PageRecord<'_> {
    const fn mcid(&self) -> u32 {
        match self {
            Self::Vector(record) => record.mcid,
            Self::EquationNumber(record) => record.mcid,
        }
    }
}

fn object<'a>(
    objects: &'a BTreeMap<u32, &'a [u8]>,
    number: u32,
) -> Result<&'a [u8], TaggedPdfIndependentErrorV2> {
    objects
        .get(&number)
        .copied()
        .ok_or(TaggedPdfIndependentErrorV2::ObjectClosureMismatch)
}

fn role_object(
    closure: &TaggedPdfIndependentClosureV2,
    role: &str,
) -> Result<u32, TaggedPdfIndependentErrorV2> {
    closure
        .object_for_role(role)
        .ok_or(TaggedPdfIndependentErrorV2::InvalidClosure)
}

fn structure_object(
    closure: &TaggedPdfIndependentClosureV2,
    structure_node_id: u32,
) -> Result<u32, TaggedPdfIndependentErrorV2> {
    role_object(closure, &format!("structure_element:{structure_node_id}"))
}

fn property_span(actual_text: Option<&str>, language: Option<&str>) -> Option<String> {
    if actual_text.is_none() && language.is_none() {
        return None;
    }
    let mut value = String::from("/Span <<");
    if let Some(actual_text) = actual_text {
        value.push_str(&format!(" /ActualText <{}>", utf16be_hex(actual_text)));
    }
    if let Some(language) = language {
        value.push_str(&format!(" /Lang <{}>", utf16be_hex(language)));
    }
    value.push_str(" >> BDC\n");
    Some(value)
}

fn exact_next(
    value: &[u8],
    needle: &[u8],
    cursor: usize,
) -> Result<usize, TaggedPdfIndependentErrorV2> {
    let position = find_from(value, needle, cursor)
        .ok_or(TaggedPdfIndependentErrorV2::MarkedContentMismatch)?;
    if contains(&value[cursor..position], b"/MCID ") {
        return Err(TaggedPdfIndependentErrorV2::MarkedContentMismatch);
    }
    Ok(position)
}

fn single_do_resource(paint: &[u8]) -> Result<Vec<u8>, TaggedPdfIndependentErrorV2> {
    let mut found = None;
    for line in paint.split(|byte| *byte == b'\n') {
        let Some(name) = line.strip_suffix(b" Do") else {
            continue;
        };
        if !name.starts_with(b"/V")
            || name.len() == 2
            || !name[2..].iter().all(u8::is_ascii_digit)
            || found.replace(name[1..].to_vec()).is_some()
        {
            return Err(TaggedPdfIndependentErrorV2::FormUsageMismatch);
        }
    }
    found.ok_or(TaggedPdfIndependentErrorV2::FormUsageMismatch)
}

fn page_xobject_target(
    page: &[u8],
    resource_name: &[u8],
) -> Result<u32, TaggedPdfIndependentErrorV2> {
    let mut needle = Vec::with_capacity(resource_name.len() + 2);
    needle.extend_from_slice(b"/");
    needle.extend_from_slice(resource_name);
    needle.push(b' ');
    let start = find_from(page, &needle, 0)
        .and_then(|start| start.checked_add(needle.len()))
        .ok_or(TaggedPdfIndependentErrorV2::FormUsageMismatch)?;
    let end = page[start..]
        .iter()
        .position(|byte| *byte == b' ')
        .and_then(|length| start.checked_add(length))
        .ok_or(TaggedPdfIndependentErrorV2::FormUsageMismatch)?;
    if page.get(end..end + 5) != Some(b" 0 R ") {
        return Err(TaggedPdfIndependentErrorV2::FormUsageMismatch);
    }
    std::str::from_utf8(&page[start..end])
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .ok_or(TaggedPdfIndependentErrorV2::FormUsageMismatch)
}

struct ParsedPdfLiteralV2<'a> {
    raw: &'a [u8],
    decoded: Vec<u8>,
}

fn parse_pdf_literal_after<'a>(
    value: &'a [u8],
    marker: &[u8],
) -> Result<ParsedPdfLiteralV2<'a>, TaggedPdfIndependentErrorV2> {
    if count_bytes(value, marker)? != 1 {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    let start = find_from(value, marker, 0)
        .and_then(|offset| offset.checked_add(marker.len()))
        .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
    parse_pdf_literal_at(value, start)
}

fn parse_pdf_literal_at(
    value: &[u8],
    start: usize,
) -> Result<ParsedPdfLiteralV2<'_>, TaggedPdfIndependentErrorV2> {
    if value.get(start) != Some(&b'(') {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    let mut decoded = Vec::new();
    let mut cursor = start + 1;
    while let Some(byte) = value.get(cursor).copied() {
        match byte {
            b')' => {
                return Ok(ParsedPdfLiteralV2 {
                    raw: &value[start..=cursor],
                    decoded,
                });
            }
            b'\\' => {
                let escaped = value
                    .get(cursor + 1)
                    .copied()
                    .filter(|escaped| matches!(escaped, b'(' | b')' | b'\\'))
                    .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
                decoded.push(escaped);
                cursor += 2;
            }
            b'(' | b'\r' | b'\n' | 0 => {
                return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
            }
            _ => {
                decoded.push(byte);
                cursor += 1;
            }
        }
    }
    Err(TaggedPdfIndependentErrorV2::MalformedPdf)
}

fn parse_hex_string_after<'a>(
    value: &'a [u8],
    marker: &[u8],
) -> Result<&'a [u8], TaggedPdfIndependentErrorV2> {
    if count_bytes(value, marker)? != 1 {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    let start = find_from(value, marker, 0)
        .and_then(|offset| offset.checked_add(marker.len()))
        .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
    if value.get(start) != Some(&b'<') || value.get(start + 1) == Some(&b'<') {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    let end = value[start + 1..]
        .iter()
        .position(|byte| *byte == b'>')
        .and_then(|length| start.checked_add(length + 1))
        .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
    let bytes = &value[start + 1..end];
    if bytes.is_empty()
        || bytes.len() % 2 != 0
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(byte))
    {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    Ok(bytes)
}

fn parse_reference_after(value: &[u8], marker: &[u8]) -> Result<u32, TaggedPdfIndependentErrorV2> {
    if count_bytes(value, marker)? != 1 {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    let start = find_from(value, marker, 0)
        .and_then(|offset| offset.checked_add(marker.len()))
        .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
    let end = value[start..]
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .and_then(|length| start.checked_add(length))
        .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
    if start == end || value.get(end..end + 4) != Some(b" 0 R") {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    std::str::from_utf8(&value[start..end])
        .ok()
        .and_then(|number| number.parse::<u32>().ok())
        .filter(|number| *number != 0)
        .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)
}

fn parse_optional_reference_after(
    value: &[u8],
    marker: &[u8],
) -> Result<Option<u32>, TaggedPdfIndependentErrorV2> {
    match count_bytes(value, marker)? {
        0 => Ok(None),
        1 => parse_reference_after(value, marker).map(Some),
        _ => Err(TaggedPdfIndependentErrorV2::MalformedPdf),
    }
}

fn parse_u32_after(value: &[u8], marker: &[u8]) -> Result<u32, TaggedPdfIndependentErrorV2> {
    if count_bytes(value, marker)? != 1 {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    let start = find_from(value, marker, 0)
        .and_then(|offset| offset.checked_add(marker.len()))
        .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
    let end = value[start..]
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .and_then(|length| start.checked_add(length))
        .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)?;
    if start == end {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    std::str::from_utf8(&value[start..end])
        .ok()
        .and_then(|number| number.parse::<u32>().ok())
        .ok_or(TaggedPdfIndependentErrorV2::MalformedPdf)
}

fn count_bytes(value: &[u8], needle: &[u8]) -> Result<usize, TaggedPdfIndependentErrorV2> {
    if needle.is_empty() {
        return Err(TaggedPdfIndependentErrorV2::MalformedPdf);
    }
    Ok(value
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count())
}

fn utf16be_hex(value: &str) -> String {
    use std::fmt::Write;

    let mut output = String::from("FEFF");
    for unit in value.encode_utf16() {
        write!(&mut output, "{unit:04X}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn production_fixture_output_v2() -> (typaxis_pdf::StagingTaggedPdfV2, [u8; 32]) {
        use typaxis_core::{EffectiveConfigFingerprint, EngineIdentity, Length, Point};
        use typaxis_display_list::{
            build_structure_registry_v2, build_vector_marked_content_plan_v2,
            prove_vector_form_structure_isolation_v2, select_staging_book_navigation_v2,
            staging_precomposed_vector_tagged_pdf_fixture, BookNavigationDestinationBinding,
            BookNavigationSelectedPage, DestinationView, NamedDestination,
        };
        use typaxis_machine_profile::{
            preflight_staging_tagged_pdf_profile_v2, StagingSemanticContainerSessionIdentity,
        };
        use typaxis_resources::{
            finalize_staging_safe_vector_forms_v2, VectorContentCandidateRegistry,
        };
        use typaxis_syntax::{
            validate_staging_book_navigation_v2, validate_staging_structure_semantics_v2,
        };

        let fixture = staging_precomposed_vector_tagged_pdf_fixture().unwrap();
        let package = &fixture.layout.package;
        let limits = &fixture.layout.limits;
        let navigation = validate_staging_book_navigation_v2(package, limits).unwrap();
        let semantics =
            validate_staging_structure_semantics_v2(package, &navigation, limits).unwrap();
        let session = StagingSemanticContainerSessionIdentity::fresh();
        let profile = preflight_staging_tagged_pdf_profile_v2(
            package,
            &navigation,
            &semantics,
            limits,
            &session,
        )
        .unwrap();
        profile
            .verify(package, &navigation, &semantics, limits, &session)
            .unwrap();
        let pages = fixture
            .display
            .pages()
            .iter()
            .map(|page| BookNavigationSelectedPage {
                page_index: page.page_index(),
                width_raw: 1_000 * 65_536,
                height_raw: 800 * 65_536,
            })
            .collect::<Vec<_>>();
        let destinations = navigation
            .anchors()
            .iter()
            .enumerate()
            .map(
                |(index, (anchor, source))| BookNavigationDestinationBinding {
                    source_node_id: *source,
                    frame_id: index as u32,
                    destination: NamedDestination {
                        anchor_id: anchor.clone(),
                        page_index: 0,
                        view: DestinationView::Xyz {
                            point: Point {
                                x: Length::ZERO,
                                y: Length::ZERO,
                            },
                        },
                    },
                },
            )
            .collect::<Vec<_>>();
        let book = select_staging_book_navigation_v2(
            &navigation,
            profile.base().authorization(),
            limits,
            sha256(b"tagged-vector-complete-layout-v2"),
            4,
            &pages,
            &destinations,
            &[],
            &[],
            &fixture.display,
        )
        .unwrap();
        let registry = build_structure_registry_v2(
            package,
            &navigation,
            &semantics,
            profile.authorization(),
            limits,
        )
        .unwrap();
        let isolation = prove_vector_form_structure_isolation_v2(&fixture.display).unwrap();
        let plan = build_vector_marked_content_plan_v2(
            &registry,
            profile.authorization(),
            limits,
            &navigation,
            profile.base().authorization(),
            &book,
            &[],
            &[],
            &fixture.display,
            &isolation,
            &fixture.block_selected,
            &fixture.layout.math_flows,
        )
        .unwrap();
        let serialization = plan
            .authorize_pdf_serialization(
                &registry,
                profile.authorization(),
                limits,
                &navigation,
                profile.base().authorization(),
                &book,
                &fixture.display,
                &isolation,
                &fixture.block_selected,
                &fixture.layout.math_flows,
            )
            .unwrap();
        let candidates = VectorContentCandidateRegistry::from_admitted(
            &fixture.layout.admitted,
            package.resources(),
        )
        .unwrap();
        let forms =
            finalize_staging_safe_vector_forms_v2(&fixture.display, &candidates, limits).unwrap();
        let vector = typaxis_pdf::build_staging_safe_vector_pdf_contribution_v2(
            &fixture.display,
            &forms,
            &candidates,
            limits,
        )
        .unwrap();
        let output = typaxis_pdf::write_staging_tagged_pdf_v2(
            package,
            &navigation,
            &semantics,
            profile.authorization(),
            profile.base().authorization(),
            &book,
            &registry,
            serialization,
            &fixture.display,
            &isolation,
            &fixture.layout.admitted,
            &forms,
            &candidates,
            &vector,
            limits,
            &EngineIdentity::compiled(),
            EffectiveConfigFingerprint::from_untrusted_bytes(sha256(
                b"tagged-vector-effective-config-v2",
            )),
        )
        .unwrap();
        (output, profile.fingerprint())
    }

    fn closure_from_pdf(
        pdf: &[u8],
        roles: impl IntoIterator<Item = (u32, String)>,
        object_budget_charge_count: u32,
        observation_algorithm: &str,
        xmp_sha256: [u8; 32],
    ) -> Result<TaggedPdfIndependentClosureV2, TaggedPdfIndependentErrorV2> {
        let objects = parse_objects(pdf).map_err(|_| TaggedPdfIndependentErrorV2::MalformedPdf)?;
        let roles = roles.into_iter().collect::<BTreeMap<_, _>>();
        let closed = objects
            .iter()
            .map(|object| {
                Ok(TaggedPdfObjectClosureV2::new(
                    object.number,
                    roles
                        .get(&object.number)
                        .cloned()
                        .ok_or(TaggedPdfIndependentErrorV2::InvalidClosure)?,
                    sha256(object.body),
                ))
            })
            .collect::<Result<Vec<_>, TaggedPdfIndependentErrorV2>>()?;
        TaggedPdfIndependentClosureV2::new(
            observation_algorithm.to_owned(),
            sha256(pdf),
            pdf.len() as u64,
            closed.len() as u32,
            object_budget_charge_count,
            xmp_sha256,
            closed,
        )
    }

    fn fixture() -> (Vec<u8>, TaggedPdfIndependentExpectationsV2) {
        let (output, profile_fingerprint) = production_fixture_output_v2();
        let observation = output.observation();
        assert_eq!(observation.profile_sha256(), profile_fingerprint);
        let roles = observation
            .objects()
            .iter()
            .map(|object| (object.object_number(), object.role().to_owned()))
            .collect::<Vec<_>>();
        let closure = closure_from_pdf(
            output.bytes(),
            roles,
            observation.object_budget_charge_count(),
            typaxis_pdf::TAGGED_PDF_OBSERVATION_ALGORITHM_V2,
            observation.xmp_sha256(),
        )
        .unwrap();
        assert_eq!(closure.pdf_sha256(), observation.pdf_sha256());
        let vectors = vec![
            TaggedVectorExpectationV2::new(
                TaggedVectorSemanticKindV2::InlineVector,
                0,
                0,
                3,
                "丸括弧で囲んだ二項目".to_owned(),
                None,
                Some("en-US".to_owned()),
                Some("en-US".to_owned()),
                0,
            )
            .unwrap(),
            TaggedVectorExpectationV2::new(
                TaggedVectorSemanticKindV2::MathVector,
                0,
                1,
                4,
                "xたすy".to_owned(),
                Some("xたすy".to_owned()),
                Some("en-US".to_owned()),
                Some("en-US".to_owned()),
                0,
            )
            .unwrap(),
            TaggedVectorExpectationV2::new(
                TaggedVectorSemanticKindV2::VectorFigure,
                1,
                0,
                5,
                "配置図".to_owned(),
                None,
                Some("en-US".to_owned()),
                Some("en-US".to_owned()),
                0,
            )
            .unwrap(),
            TaggedVectorExpectationV2::new(
                TaggedVectorSemanticKindV2::MathVectorBlock,
                1,
                1,
                6,
                "xたすy、式1".to_owned(),
                Some("xたすy、式1".to_owned()),
                Some("en-US".to_owned()),
                Some("en-US".to_owned()),
                0,
            )
            .unwrap(),
        ];
        let numbers = vec![TaggedEquationNumberExpectationV2::new(
            1,
            2,
            7,
            6,
            "(1)".to_owned(),
            Some("en-US".to_owned()),
            None,
            0,
        )
        .unwrap()];
        let expected = TaggedPdfIndependentExpectationsV2::new(
            "ja".to_owned(),
            2,
            1,
            vectors,
            numbers,
            closure,
        )
        .unwrap();
        (output.bytes().to_vec(), expected)
    }

    fn rebind(pdf: &[u8], expected: &mut TaggedPdfIndependentExpectationsV2) {
        let roles = expected
            .closure
            .objects()
            .iter()
            .map(|object| (object.object_number(), object.role().to_owned()))
            .collect::<Vec<_>>();
        expected.closure = closure_from_pdf(
            pdf,
            roles,
            1,
            TAGGED_PDF_OBSERVATION_ALGORITHM_V2,
            expected.closure.xmp_sha256(),
        )
        .unwrap();
    }

    fn replace_once(pdf: &mut [u8], original: &[u8], replacement: &[u8]) {
        assert_eq!(original.len(), replacement.len());
        let start = find_from(pdf, original, 0).expect("fixture token must exist");
        pdf[start..start + original.len()].copy_from_slice(replacement);
    }

    fn replace_once_in_object(
        pdf: &mut [u8],
        object_number: u32,
        original: &[u8],
        replacement: &[u8],
    ) {
        assert_eq!(original.len(), replacement.len());
        let marker = format!("{object_number} 0 obj\n");
        let object_start = find_from(pdf, marker.as_bytes(), 0).unwrap() + marker.len();
        let object_end = find_from(pdf, b"\nendobj\n", object_start).unwrap();
        let start = find_from(pdf, original, object_start).expect("fixture token must exist");
        assert!(start < object_end);
        pdf[start..start + original.len()].copy_from_slice(replacement);
    }

    #[test]
    fn independent_tagged_pdf_v2_validator_accepts_actual_writer_output() {
        let (pdf, expected) = fixture();
        let report = inspect_tagged_pdf_v2(&pdf, &expected).unwrap();
        assert_eq!(
            report.validator_algorithm(),
            TAGGED_PDF_INDEPENDENT_VALIDATOR_ALGORITHM_V2
        );
        assert_eq!(report.pdf_sha256(), sha256(&pdf));
        assert_eq!(report.page_count(), 2);
        assert_eq!(report.form_count(), 1);
        assert_eq!(report.vector_count(), 4);
        assert_eq!(report.form_do_count(), 4);
        assert_eq!(report.equation_number_count(), 1);
        assert_eq!(report.extracted_text(), ["xたすy", "xたすy、式1", "(1)"]);
        assert!(expected.closure.object_for_role("outline_root").is_some());
        assert!(expected
            .closure
            .object_for_role("structure_id_tree")
            .is_none());
    }

    #[test]
    fn tagged_pdf_v2_equation_number_expectation_accepts_unicode_normal_text() {
        let number = TaggedEquationNumberExpectationV2::new(
            1,
            2,
            7,
            6,
            "第1式".to_owned(),
            Some("ja".to_owned()),
            None,
            0,
        )
        .unwrap();
        assert_eq!(number.exact_text(), "第1式");
        assert_eq!(
            TaggedEquationNumberExpectationV2::new(
                1,
                2,
                7,
                6,
                "第1\n式".to_owned(),
                Some("ja".to_owned()),
                None,
                0,
            ),
            Err(TaggedPdfIndependentErrorV2::InvalidExpectation),
        );
    }

    #[test]
    fn independent_tagged_pdf_v2_validator_rejects_alt_actual_text_language_and_role_tamper() {
        let mutations = [
            (
                format!("/Alt <{}>", utf16be_hex("配置図")),
                format!("/Alt <{}>", utf16be_hex("誤配置")),
                TaggedPdfIndependentErrorV2::StructureMismatch,
            ),
            (
                format!("/ActualText <{}>", utf16be_hex("xたすy")),
                format!("/ActualText <{}>", utf16be_hex("xひくy")),
                TaggedPdfIndependentErrorV2::MarkedContentMismatch,
            ),
            (
                format!("/Lang <{}>", utf16be_hex("en-US")),
                format!("/Lang <{}>", utf16be_hex("fr-FR")),
                TaggedPdfIndependentErrorV2::MarkedContentMismatch,
            ),
            (
                "/Figure << /MCID 0 >>".to_owned(),
                "/Formul << /MCID 0 >>".to_owned(),
                TaggedPdfIndependentErrorV2::MarkedContentMismatch,
            ),
            (
                format!("/Alt <{}>", utf16be_hex("配置図")),
                format!("/Xlt <{}>", utf16be_hex("配置図")),
                TaggedPdfIndependentErrorV2::StructureMismatch,
            ),
            (
                format!("/ActualText <{}>", utf16be_hex("xたすy")),
                format!("/XctualText <{}>", utf16be_hex("xたすy")),
                TaggedPdfIndependentErrorV2::MarkedContentMismatch,
            ),
            (
                format!("/Lang <{}>", utf16be_hex("en-US")),
                format!("/Xang <{}>", utf16be_hex("en-US")),
                TaggedPdfIndependentErrorV2::MarkedContentMismatch,
            ),
        ];
        for (original, replacement, error) in mutations {
            let (mut pdf, mut expected) = fixture();
            let start = find_from(&pdf, original.as_bytes(), 0).unwrap();
            pdf[start..start + original.len()].copy_from_slice(replacement.as_bytes());
            rebind(&pdf, &mut expected);
            assert_eq!(inspect_tagged_pdf_v2(&pdf, &expected), Err(error));
        }
    }

    #[test]
    fn independent_tagged_pdf_v2_validator_rejects_mcid_page_parent_tree_and_formula_order() {
        let (mut mcid_pdf, mut mcid_expected) = fixture();
        replace_once(
            &mut mcid_pdf,
            b"/Figure << /MCID 0 >>",
            b"/Figure << /MCID 9 >>",
        );
        rebind(&mcid_pdf, &mut mcid_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&mcid_pdf, &mcid_expected),
            Err(TaggedPdfIndependentErrorV2::MarkedContentMismatch)
        );

        let (mut page_pdf, mut page_expected) = fixture();
        let page_one = role_object(&page_expected.closure, "page:1").unwrap();
        let page_zero = role_object(&page_expected.closure, "page:0").unwrap();
        let original = format!("/Pg {page_one} 0 R /MCID 1");
        let replacement = format!("/Pg {page_zero} 0 R /MCID 1");
        replace_once(&mut page_pdf, original.as_bytes(), replacement.as_bytes());
        rebind(&page_pdf, &mut page_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&page_pdf, &page_expected),
            Err(TaggedPdfIndependentErrorV2::StructureMismatch)
        );

        let (mut parent_pdf, mut parent_expected) = fixture();
        let node_three = structure_object(&parent_expected.closure, 3).unwrap();
        let node_four = structure_object(&parent_expected.closure, 4).unwrap();
        replace_once(
            &mut parent_pdf,
            format!("0 [{node_three} 0 R {node_four} 0 R ").as_bytes(),
            format!("0 [{node_four} 0 R {node_three} 0 R ").as_bytes(),
        );
        rebind(&parent_pdf, &mut parent_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&parent_pdf, &parent_expected),
            Err(TaggedPdfIndependentErrorV2::ParentTreeMismatch)
        );

        let (mut order_pdf, mut order_expected) = fixture();
        let number = structure_object(&order_expected.closure, 7).unwrap();
        let page = role_object(&order_expected.closure, "page:1").unwrap();
        let mcr = format!("<< /Type /MCR /Pg {page} 0 R /MCID 1 >>");
        let original = format!("/K [{mcr} {number} 0 R ");
        let replacement = format!("/K [{number} 0 R {mcr} ");
        replace_once(&mut order_pdf, original.as_bytes(), replacement.as_bytes());
        rebind(&order_pdf, &mut order_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&order_pdf, &order_expected),
            Err(TaggedPdfIndependentErrorV2::StructureMismatch)
        );

        let (mut leaf_pdf, mut leaf_expected) = fixture();
        let leaf = structure_object(&leaf_expected.closure, 7).unwrap();
        replace_once_in_object(&mut leaf_pdf, leaf, b"/S /Span", b"/S /Div ");
        rebind(&leaf_pdf, &mut leaf_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&leaf_pdf, &leaf_expected),
            Err(TaggedPdfIndependentErrorV2::StructureMismatch)
        );
    }

    #[test]
    fn independent_tagged_pdf_v2_validator_rejects_form_semantics_and_same_length_stream_tamper() {
        let (mut tampered, expected) = fixture();
        let form = role_object(&expected.closure, "vector_form:0").unwrap();
        replace_once_in_object(&mut tampered, form, b"2 2 m", b"9 9 m");
        assert_eq!(
            inspect_tagged_pdf_v2(&tampered, &expected),
            Err(TaggedPdfIndependentErrorV2::PdfHashMismatch)
        );

        let (mut pdf, mut expected) = fixture();
        let form = role_object(&expected.closure, "vector_form:0").unwrap();
        replace_once_in_object(&mut pdf, form, b"2 2 m", b"BMC  ");
        rebind(&pdf, &mut expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&pdf, &expected),
            Err(TaggedPdfIndependentErrorV2::FormSemanticContent)
        );
    }

    #[test]
    fn independent_tagged_pdf_v2_validator_rejects_equation_font_and_cid_tamper() {
        let (mut glyph_pdf, mut glyph_expected) = fixture();
        let page_content = role_object(&glyph_expected.closure, "page_content:1").unwrap();
        replace_once_in_object(&mut glyph_pdf, page_content, b"<0001> Tj", b"<FFFF> Tj");
        rebind(&glyph_pdf, &mut glyph_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&glyph_pdf, &glyph_expected),
            Err(TaggedPdfIndependentErrorV2::TextExtractionMismatch)
        );

        let (mut font_pdf, mut font_expected) = fixture();
        let type0 = role_object(&font_expected.closure, "equation_font_type0:0").unwrap();
        replace_once_in_object(
            &mut font_pdf,
            type0,
            b"/Encoding /Identity-H",
            b"/Encoding /Identity-X",
        );
        rebind(&font_pdf, &mut font_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&font_pdf, &font_expected),
            Err(TaggedPdfIndependentErrorV2::TextExtractionMismatch)
        );
    }

    #[test]
    fn tagged_pdf_v2_cmap_accepts_actual_text_only_and_rejects_unknown_operators() {
        let (pdf, expected) = fixture();
        let objects = parse_objects(&pdf).unwrap();
        let cmap_number = role_object(&expected.closure, "equation_font_to_unicode:0").unwrap();
        let body = objects
            .iter()
            .find(|object| object.number == cmap_number)
            .unwrap()
            .body;
        let cmap = object_stream(body).unwrap().unwrap();
        let prefix_end =
            find_from(cmap, b"endcodespacerange\n", 0).unwrap() + b"endcodespacerange\n".len();
        let suffix_start = find_from(cmap, b"endcmap\n", prefix_end).unwrap();
        let mut empty = cmap[..prefix_end].to_vec();
        empty.extend_from_slice(&cmap[suffix_start..]);
        assert_eq!(validate_equation_cmap(&empty, 3), Ok(()));
        for (from, to) in [
            (b"beginbfchar".as_slice(), b"beginxfchar".as_slice()),
            (b"3 beginbfchar", b"2 beginbfchar"),
            (b"<0001>", b"<FFFF>"),
        ] {
            let mut bad = cmap.to_vec();
            replace_once(&mut bad, from, to);
            assert_eq!(
                validate_equation_cmap(&bad, 3),
                Err(TaggedPdfIndependentErrorV2::TextExtractionMismatch)
            );
        }
    }

    #[test]
    fn independent_tagged_pdf_v2_validator_closes_catalog_pages_and_xmp() {
        let (mut trailer_pdf, mut trailer_expected) = fixture();
        let info = role_object(&trailer_expected.closure, "info").unwrap();
        let metadata = role_object(&trailer_expected.closure, "metadata").unwrap();
        let info_ref = format!("/Info {info} 0 R");
        let metadata_ref = format!("/Info {metadata} 0 R");
        replace_once(
            &mut trailer_pdf,
            info_ref.as_bytes(),
            metadata_ref.as_bytes(),
        );
        rebind(&trailer_pdf, &mut trailer_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&trailer_pdf, &trailer_expected),
            Err(TaggedPdfIndependentErrorV2::MalformedPdf)
        );

        let (mut catalog_pdf, mut catalog_expected) = fixture();
        replace_once_in_object(&mut catalog_pdf, 1, b"/Pages 2 0 R", b"/Pages 3 0 R");
        rebind(&catalog_pdf, &mut catalog_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&catalog_pdf, &catalog_expected),
            Err(TaggedPdfIndependentErrorV2::CatalogMismatch)
        );

        let (mut page_pdf, mut page_expected) = fixture();
        let page = role_object(&page_expected.closure, "page:0").unwrap();
        replace_once_in_object(&mut page_pdf, page, b"/Parent 2 0 R", b"/Parent 3 0 R");
        rebind(&page_pdf, &mut page_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&page_pdf, &page_expected),
            Err(TaggedPdfIndependentErrorV2::PageMismatch)
        );

        let (mut metadata_pdf, mut metadata_expected) = fixture();
        let metadata = role_object(&metadata_expected.closure, "metadata").unwrap();
        replace_once_in_object(&mut metadata_pdf, metadata, b"<x:xmpmeta", b"<y:xmpmeta");
        rebind(&metadata_pdf, &mut metadata_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&metadata_pdf, &metadata_expected),
            Err(TaggedPdfIndependentErrorV2::MetadataMismatch)
        );
    }

    #[test]
    fn independent_tagged_pdf_v2_validator_closes_outline_graph() {
        let (mut destination_pdf, mut destination_expected) = fixture();
        let item = role_object(&destination_expected.closure, "outline_item:0").unwrap();
        replace_once_in_object(
            &mut destination_pdf,
            item,
            b"/Dest (vector-result)",
            b"/Dest (vector-absent)",
        );
        rebind(&destination_pdf, &mut destination_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&destination_pdf, &destination_expected),
            Err(TaggedPdfIndependentErrorV2::OutlineMismatch)
        );

        let (mut page_pdf, mut page_expected) = fixture();
        let destinations = role_object(&page_expected.closure, "destinations").unwrap();
        let page = role_object(&page_expected.closure, "page:0").unwrap();
        replace_once_in_object(
            &mut page_pdf,
            destinations,
            format!("[{page} 0 R /XYZ").as_bytes(),
            b"[3 0 R /XYZ",
        );
        rebind(&page_pdf, &mut page_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&page_pdf, &page_expected),
            Err(TaggedPdfIndependentErrorV2::OutlineMismatch)
        );

        let (mut outline_pdf, mut outline_expected) = fixture();
        let item = role_object(&outline_expected.closure, "outline_item:0").unwrap();
        let structure = structure_object(&outline_expected.closure, 1).unwrap();
        let form = role_object(&outline_expected.closure, "vector_form:0").unwrap();
        replace_once_in_object(
            &mut outline_pdf,
            item,
            format!("/SE {structure} 0 R").as_bytes(),
            format!("/SE {form} 0 R").as_bytes(),
        );
        rebind(&outline_pdf, &mut outline_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&outline_pdf, &outline_expected),
            Err(TaggedPdfIndependentErrorV2::OutlineMismatch)
        );

        let (mut cycle_pdf, mut cycle_expected) = fixture();
        let item = role_object(&cycle_expected.closure, "outline_item:0").unwrap();
        let root = role_object(&cycle_expected.closure, "outline_root").unwrap();
        replace_once_in_object(
            &mut cycle_pdf,
            item,
            format!("/Parent {root} 0 R").as_bytes(),
            format!("/Parent {item} 0 R").as_bytes(),
        );
        rebind(&cycle_pdf, &mut cycle_expected);
        assert_eq!(
            inspect_tagged_pdf_v2(&cycle_pdf, &cycle_expected),
            Err(TaggedPdfIndependentErrorV2::OutlineMismatch)
        );
    }

    #[test]
    fn independent_tagged_pdf_v2_validator_closes_id_tree_bidirectionally() {
        // The vector-only writer fixture has no table headers / footnotes and
        // correctly omits IDTree. Exercise its independent graph check with
        // an explicitly identified StructElem, without changing source IDs.
        let (pdf, mut expected) = fixture();
        let structure = structure_object(&expected.closure, 1).unwrap();
        let root = role_object(&expected.closure, "structure_tree_root").unwrap();
        let id_tree = expected.closure.object_count + 1;
        let mut owned = parse_objects(&pdf)
            .unwrap()
            .into_iter()
            .map(|object| (object.number, object.body.to_vec()))
            .collect::<BTreeMap<_, _>>();
        for (number, suffix) in [
            (root, format!(" /IDTree {id_tree} 0 R >>")),
            (structure, " /ID (typaxis-se-00000001) >>".to_owned()),
        ] {
            let body = owned.get_mut(&number).unwrap();
            body.truncate(body.len() - 3);
            body.extend_from_slice(suffix.as_bytes());
        }
        owned.insert(
            id_tree,
            format!("<< /Names [(typaxis-se-00000001) {structure} 0 R ] >>").into_bytes(),
        );
        expected.closure.objects.push(TaggedPdfObjectClosureV2::new(
            id_tree,
            "structure_id_tree".to_owned(),
            sha256(owned.get(&id_tree).unwrap()),
        ));
        expected.closure.object_count += 1;
        let borrowed = owned
            .iter()
            .map(|(number, body)| (*number, body.as_slice()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            validate_structure_ids_v2(&borrowed, &expected.closure, borrowed[&root]),
            Ok(())
        );
        let mut bad_root = borrowed[&root].to_vec();
        replace_once(
            &mut bad_root,
            format!("/IDTree {id_tree} 0 R").as_bytes(),
            format!("/IDTree {structure} 0 R").as_bytes(),
        );
        assert_eq!(
            validate_structure_ids_v2(&borrowed, &expected.closure, &bad_root),
            Err(TaggedPdfIndependentErrorV2::StructureMismatch)
        );
        let mut names = borrowed[&id_tree].to_vec();
        replace_once(&mut names, b"typaxis-se-00000001", b"typaxis-se-00000002");
        let mut bad_names = borrowed;
        bad_names.insert(id_tree, &names);
        assert_eq!(
            validate_structure_ids_v2(&bad_names, &expected.closure, bad_names[&root]),
            Err(TaggedPdfIndependentErrorV2::StructureMismatch)
        );
    }

    #[test]
    fn tagged_pdf_v2_closure_rejects_legacy_algorithm_and_non_single_object_charge() {
        let (pdf, expected) = fixture();
        let roles = expected
            .closure
            .objects()
            .iter()
            .map(|object| (object.object_number(), object.role().to_owned()))
            .collect::<Vec<_>>();
        let xmp_sha256 = expected.closure.xmp_sha256();
        assert_eq!(
            closure_from_pdf(
                &pdf,
                roles.clone(),
                1,
                "typaxis.tagged-pdf-observation/1",
                xmp_sha256,
            ),
            Err(TaggedPdfIndependentErrorV2::InvalidClosure)
        );
        for count in [0, 2] {
            assert_eq!(
                closure_from_pdf(
                    &pdf,
                    roles.clone(),
                    count,
                    TAGGED_PDF_OBSERVATION_ALGORITHM_V2,
                    xmp_sha256,
                ),
                Err(TaggedPdfIndependentErrorV2::InvalidClosure)
            );
        }
        let mut duplicate_roles = roles;
        duplicate_roles[1].1 = duplicate_roles[0].1.clone();
        assert_eq!(
            closure_from_pdf(
                &pdf,
                duplicate_roles,
                1,
                TAGGED_PDF_OBSERVATION_ALGORITHM_V2,
                xmp_sha256,
            ),
            Err(TaggedPdfIndependentErrorV2::InvalidClosure)
        );
        let mut malformed_roles = expected
            .closure
            .objects()
            .iter()
            .map(|object| (object.object_number(), object.role().to_owned()))
            .collect::<Vec<_>>();
        let form = malformed_roles
            .iter_mut()
            .find(|(_, role)| role.starts_with("vector_form:"))
            .unwrap();
        form.1 = "vector_form:not-a-number".to_owned();
        assert_eq!(
            closure_from_pdf(
                &pdf,
                malformed_roles,
                1,
                TAGGED_PDF_OBSERVATION_ALGORITHM_V2,
                xmp_sha256,
            ),
            Err(TaggedPdfIndependentErrorV2::InvalidClosure)
        );
    }
}
