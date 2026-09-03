use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::sha256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeVectorPdfIndependentExpectations {
    page_count: u32,
    form_bboxes: Vec<[i64; 4]>,
    form_content_sha256s: Vec<[u8; 32]>,
    ext_g_state_count: u32,
    do_count: u32,
    ext_g_state_alpha_pairs: Vec<[u32; 2]>,
    placement_form_indices: Vec<u32>,
    placement_matrices: Vec<[i64; 6]>,
    placement_colors_rgb8: Vec<[u8; 3]>,
}

impl SafeVectorPdfIndependentExpectations {
    pub fn new(
        page_count: u32,
        form_bboxes: Vec<[i64; 4]>,
        form_content_sha256s: Vec<[u8; 32]>,
        ext_g_state_alpha_pairs: Vec<[u32; 2]>,
        placement_form_indices: Vec<u32>,
        placement_matrices: Vec<[i64; 6]>,
        placement_colors_rgb8: Vec<[u8; 3]>,
    ) -> Result<Self, SafeVectorPdfIndependentError> {
        let ext_g_state_count = u32::try_from(ext_g_state_alpha_pairs.len())
            .map_err(|_| SafeVectorPdfIndependentError::InvalidExpectation)?;
        let do_count = u32::try_from(placement_matrices.len())
            .map_err(|_| SafeVectorPdfIndependentError::InvalidExpectation)?;
        if page_count == 0
            || form_bboxes.is_empty()
            || form_bboxes.len() != form_content_sha256s.len()
            || ext_g_state_alpha_pairs.is_empty()
            || placement_form_indices.is_empty()
            || placement_form_indices.len() != placement_matrices.len()
            || placement_matrices.is_empty()
            || placement_matrices.len() != placement_colors_rgb8.len()
        {
            return Err(SafeVectorPdfIndependentError::InvalidExpectation);
        }
        if form_bboxes
            .iter()
            .any(|bbox| bbox[0] != 0 || bbox[1] != 0 || bbox[2] <= 0 || bbox[3] <= 0)
        {
            return Err(SafeVectorPdfIndependentError::InvalidExpectation);
        }
        if ext_g_state_alpha_pairs
            .iter()
            .flatten()
            .any(|alpha| *alpha > 65_536)
            || placement_matrices.iter().any(|matrix| {
                matrix[0] <= 0 || matrix[0] != matrix[3] || matrix[1] != 0 || matrix[2] != 0
            })
        {
            return Err(SafeVectorPdfIndependentError::InvalidExpectation);
        }
        let form_count = u32::try_from(form_bboxes.len())
            .map_err(|_| SafeVectorPdfIndependentError::InvalidExpectation)?;
        if placement_form_indices
            .iter()
            .any(|index| *index >= form_count)
            || placement_form_indices
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != (0..form_count).collect()
        {
            return Err(SafeVectorPdfIndependentError::InvalidExpectation);
        }
        Ok(Self {
            page_count,
            form_bboxes,
            form_content_sha256s,
            ext_g_state_count,
            do_count,
            ext_g_state_alpha_pairs,
            placement_form_indices,
            placement_matrices,
            placement_colors_rgb8,
        })
    }

    pub const fn page_count(&self) -> u32 {
        self.page_count
    }

    pub fn form_bboxes(&self) -> &[[i64; 4]] {
        &self.form_bboxes
    }

    pub fn form_content_sha256s(&self) -> &[[u8; 32]] {
        &self.form_content_sha256s
    }

    pub const fn ext_g_state_count(&self) -> u32 {
        self.ext_g_state_count
    }

    pub const fn do_count(&self) -> u32 {
        self.do_count
    }

    pub fn ext_g_state_alpha_pairs(&self) -> &[[u32; 2]] {
        &self.ext_g_state_alpha_pairs
    }

    pub fn placement_form_indices(&self) -> &[u32] {
        &self.placement_form_indices
    }

    pub fn placement_matrices(&self) -> &[[i64; 6]] {
        &self.placement_matrices
    }

    pub fn placement_colors_rgb8(&self) -> &[[u8; 3]] {
        &self.placement_colors_rgb8
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeVectorPdfIndependentReport {
    pdf_sha256: [u8; 32],
    object_count: u32,
    page_count: u32,
    form_count: u32,
    ext_g_state_count: u32,
    do_count: u32,
    page_root_y_flip_count: u32,
}

impl SafeVectorPdfIndependentReport {
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

    pub const fn ext_g_state_count(&self) -> u32 {
        self.ext_g_state_count
    }

    pub const fn do_count(&self) -> u32 {
        self.do_count
    }

    pub const fn page_root_y_flip_count(&self) -> u32 {
        self.page_root_y_flip_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafeVectorPdfIndependentError {
    InvalidExpectation,
    MalformedPdf,
    DuplicateObject,
    UnexpectedPageCount,
    UnexpectedFormCount,
    UnexpectedExtGStateCount,
    UnexpectedDoCount,
    UnexpectedBBox,
    InvalidFormResources,
    InvalidFormOperators,
    InvalidExtGState,
    InvalidPageResources,
    InvalidPageTransform,
    InvalidPageOperators,
    RasterContent,
    FormSemanticContent,
}

impl std::fmt::Display for SafeVectorPdfIndependentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidExpectation => formatter.write_str("invalid vector PDF expectation"),
            Self::MalformedPdf => formatter.write_str("malformed independently parsed PDF"),
            Self::DuplicateObject => formatter.write_str("duplicate PDF indirect object"),
            Self::UnexpectedPageCount => formatter.write_str("unexpected PDF page count"),
            Self::UnexpectedFormCount => formatter.write_str("unexpected vector Form count"),
            Self::UnexpectedExtGStateCount => {
                formatter.write_str("unexpected vector ExtGState count")
            }
            Self::UnexpectedDoCount => formatter.write_str("unexpected vector Do count"),
            Self::UnexpectedBBox => formatter.write_str("unexpected vector Form BBox"),
            Self::InvalidFormResources => {
                formatter.write_str("invalid vector Form resource dictionary")
            }
            Self::InvalidFormOperators => formatter.write_str("invalid vector Form operators"),
            Self::InvalidExtGState => formatter.write_str("invalid vector ExtGState dictionary"),
            Self::InvalidPageResources => {
                formatter.write_str("invalid vector page resource dictionary")
            }
            Self::InvalidPageTransform => {
                formatter.write_str("invalid or repeated page-root Y flip")
            }
            Self::InvalidPageOperators => {
                formatter.write_str("invalid vector page placement operators")
            }
            Self::RasterContent => formatter.write_str("raster content found in vector PDF"),
            Self::FormSemanticContent => {
                formatter.write_str("tagging or alternative text found in reusable Form")
            }
        }
    }
}

impl std::error::Error for SafeVectorPdfIndependentError {}

pub(super) struct ParsedObject<'a> {
    pub(super) number: u32,
    pub(super) body: &'a [u8],
}

pub fn inspect_safe_vector_pdf(
    pdf: &[u8],
    expected: &SafeVectorPdfIndependentExpectations,
) -> Result<SafeVectorPdfIndependentReport, SafeVectorPdfIndependentError> {
    if !pdf.starts_with(b"%PDF-")
        || !contains(pdf, b"xref\n")
        || !contains(pdf, b"trailer\n")
        || !pdf.ends_with(b"%%EOF\n")
    {
        return Err(SafeVectorPdfIndependentError::MalformedPdf);
    }
    if contains(pdf, b"/Subtype /Image") || contains(pdf, b"/ImageMask") {
        return Err(SafeVectorPdfIndependentError::RasterContent);
    }
    let objects = parse_objects(pdf)?;
    let page_count = count_objects(&objects, b"/Type /Page", b"/Type /Pages")?;
    if page_count != expected.page_count {
        return Err(SafeVectorPdfIndependentError::UnexpectedPageCount);
    }

    let ext_g_states: Vec<_> = objects
        .iter()
        .filter(|object| contains(object.body, b"/Type /ExtGState"))
        .collect();
    if u32::try_from(ext_g_states.len()).ok() != Some(expected.ext_g_state_count) {
        return Err(SafeVectorPdfIndependentError::UnexpectedExtGStateCount);
    }
    for (ext, alpha_pair) in ext_g_states.iter().zip(&expected.ext_g_state_alpha_pairs) {
        inspect_ext_g_state(ext.body, *alpha_pair)?;
    }
    let ext_object_numbers = ext_g_states
        .iter()
        .map(|object| object.number)
        .collect::<BTreeSet<_>>();

    let forms: Vec<_> = objects
        .iter()
        .filter(|object| contains(object.body, b"/Subtype /Form"))
        .collect();
    if expected.form_bboxes.len() != forms.len() {
        return Err(SafeVectorPdfIndependentError::UnexpectedFormCount);
    }
    let mut bound_ext_objects = BTreeSet::new();
    for ((form, bbox), content_sha256) in forms
        .iter()
        .zip(&expected.form_bboxes)
        .zip(&expected.form_content_sha256s)
    {
        for target in inspect_form(form.body, *bbox, *content_sha256, &ext_object_numbers)? {
            if !bound_ext_objects.insert(target) {
                return Err(SafeVectorPdfIndependentError::InvalidFormResources);
            }
        }
    }
    if bound_ext_objects != ext_object_numbers {
        return Err(SafeVectorPdfIndependentError::InvalidFormResources);
    }

    let mut expected_form_targets = BTreeMap::new();
    for (index, form) in forms.iter().enumerate() {
        let name = format!("/V{index}").into_bytes();
        if expected_form_targets.insert(name, form.number).is_some() {
            return Err(SafeVectorPdfIndependentError::InvalidPageResources);
        }
    }
    let page_objects: Vec<_> = objects
        .iter()
        .filter(|object| {
            contains(object.body, b"/Type /Page") && !contains(object.body, b"/Type /Pages")
        })
        .collect();
    let mut do_count = 0u32;
    let mut page_root_y_flip_count = 0u32;
    let mut matrix_offset = 0usize;
    for page in page_objects {
        let page_tokens = ascii_tokens(page.body)?;
        let content_object_number = indirect_reference(&page_tokens, b"/Contents")?;
        let page_height = page_media_box_height(page.body)?;
        let bindings = page_vector_bindings(&page_tokens, &expected_form_targets)?;
        let content = objects
            .binary_search_by_key(&content_object_number, |object| object.number)
            .ok()
            .and_then(|index| objects.get(index))
            .ok_or(SafeVectorPdfIndependentError::InvalidPageResources)?;
        let stream = object_stream(content.body)?
            .ok_or(SafeVectorPdfIndependentError::InvalidPageOperators)?;
        let expected_matrices = expected
            .placement_matrices
            .get(matrix_offset..)
            .ok_or(SafeVectorPdfIndependentError::InvalidPageOperators)?;
        let expected_form_indices = expected
            .placement_form_indices
            .get(matrix_offset..)
            .ok_or(SafeVectorPdfIndependentError::InvalidPageOperators)?;
        let expected_colors = expected
            .placement_colors_rgb8
            .get(matrix_offset..)
            .ok_or(SafeVectorPdfIndependentError::InvalidPageOperators)?;
        let (stream_do_count, stream_root_y_flip_count) = inspect_page_stream(
            stream,
            &bindings,
            page_height,
            expected_form_indices,
            expected_matrices,
            expected_colors,
        )?;
        matrix_offset = matrix_offset
            .checked_add(
                usize::try_from(stream_do_count)
                    .map_err(|_| SafeVectorPdfIndependentError::MalformedPdf)?,
            )
            .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
        do_count = do_count
            .checked_add(stream_do_count)
            .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
        page_root_y_flip_count = page_root_y_flip_count
            .checked_add(stream_root_y_flip_count)
            .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
    }
    if do_count != expected.do_count || matrix_offset != expected.placement_matrices.len() {
        return Err(SafeVectorPdfIndependentError::UnexpectedDoCount);
    }
    if page_root_y_flip_count != expected.page_count {
        return Err(SafeVectorPdfIndependentError::InvalidPageTransform);
    }
    Ok(SafeVectorPdfIndependentReport {
        pdf_sha256: sha256(pdf),
        object_count: u32::try_from(objects.len())
            .map_err(|_| SafeVectorPdfIndependentError::MalformedPdf)?,
        page_count,
        form_count: u32::try_from(forms.len())
            .map_err(|_| SafeVectorPdfIndependentError::MalformedPdf)?,
        ext_g_state_count: u32::try_from(ext_g_states.len())
            .map_err(|_| SafeVectorPdfIndependentError::MalformedPdf)?,
        do_count,
        page_root_y_flip_count,
    })
}

fn indirect_reference(tokens: &[&[u8]], key: &[u8]) -> Result<u32, SafeVectorPdfIndependentError> {
    if tokens.iter().filter(|token| **token == key).count() != 1 {
        return Err(SafeVectorPdfIndependentError::InvalidPageResources);
    }
    let index = tokens
        .iter()
        .position(|token| *token == key)
        .ok_or(SafeVectorPdfIndependentError::InvalidPageResources)?;
    let number = index
        .checked_add(1)
        .and_then(|index| tokens.get(index))
        .and_then(|value| std::str::from_utf8(value).ok())
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|number| *number > 0)
        .ok_or(SafeVectorPdfIndependentError::InvalidPageResources)?;
    if tokens.get(index + 2) != Some(&b"0".as_slice())
        || tokens.get(index + 3) != Some(&b"R".as_slice())
    {
        return Err(SafeVectorPdfIndependentError::InvalidPageResources);
    }
    Ok(number)
}

fn page_vector_bindings(
    tokens: &[&[u8]],
    expected_targets: &BTreeMap<Vec<u8>, u32>,
) -> Result<BTreeSet<Vec<u8>>, SafeVectorPdfIndependentError> {
    let resources = tokens
        .windows(3)
        .any(|window| window[0] == b"/Resources" && window[1] == b"<<" && window[2] == b"/XObject");
    if !resources {
        return Err(SafeVectorPdfIndependentError::InvalidPageResources);
    }
    let mut bindings = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        if !is_vector_resource_name(token) {
            continue;
        }
        let expected = expected_targets
            .get(*token)
            .ok_or(SafeVectorPdfIndependentError::InvalidPageResources)?;
        let observed = index
            .checked_add(1)
            .and_then(|index| tokens.get(index))
            .and_then(|value| std::str::from_utf8(value).ok())
            .and_then(|value| value.parse::<u32>().ok());
        if observed != Some(*expected)
            || tokens.get(index + 2) != Some(&b"0".as_slice())
            || tokens.get(index + 3) != Some(&b"R".as_slice())
            || !bindings.insert((*token).to_vec())
        {
            return Err(SafeVectorPdfIndependentError::InvalidPageResources);
        }
    }
    Ok(bindings)
}

fn inspect_page_stream(
    stream: &[u8],
    bindings: &BTreeSet<Vec<u8>>,
    page_height: i64,
    expected_form_indices: &[u32],
    expected_matrices: &[[i64; 6]],
    expected_colors: &[[u8; 3]],
) -> Result<(u32, u32), SafeVectorPdfIndependentError> {
    let tokens = ascii_tokens(stream)?;
    let stream_do_count = token_count_from_tokens(&tokens, b"Do")?;
    let stream_root_y_flip_count = byte_count(stream, b"1 0 0 -1 0 ")?;
    let q_count = token_count_from_tokens(&tokens, b"q")?;
    let restore_count = token_count_from_tokens(&tokens, b"Q")?;
    let matrix_count = token_count_from_tokens(&tokens, b"cm")?;
    let nonstroking_color_count = token_count_from_tokens(&tokens, b"rg")?;
    let stroking_color_count = token_count_from_tokens(&tokens, b"RG")?;
    let expected_isolation_count = stream_do_count
        .checked_add(1)
        .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
    let expected_matrix_count = stream_do_count
        .checked_add(stream_root_y_flip_count)
        .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
    let placement_count = usize::try_from(stream_do_count)
        .map_err(|_| SafeVectorPdfIndependentError::MalformedPdf)?;
    if expected_form_indices.len() < placement_count
        || expected_matrices.len() < placement_count
        || expected_colors.len() < placement_count
    {
        return Err(SafeVectorPdfIndependentError::InvalidPageOperators);
    }
    let used_names = inspect_page_token_grammar(
        &tokens,
        page_height,
        &expected_form_indices[..placement_count],
        &expected_matrices[..placement_count],
        &expected_colors[..placement_count],
        bindings,
    )?;
    if stream_root_y_flip_count != 1
        || q_count != expected_isolation_count
        || restore_count != expected_isolation_count
        || matrix_count != expected_matrix_count
        || nonstroking_color_count != stream_do_count
        || stroking_color_count != stream_do_count
        || used_names != *bindings
    {
        return Err(SafeVectorPdfIndependentError::InvalidPageOperators);
    }
    Ok((stream_do_count, stream_root_y_flip_count))
}

fn inspect_page_token_grammar(
    tokens: &[&[u8]],
    page_height: i64,
    expected_form_indices: &[u32],
    expected_matrices: &[[i64; 6]],
    expected_colors: &[[u8; 3]],
    bindings: &BTreeSet<Vec<u8>>,
) -> Result<BTreeSet<Vec<u8>>, SafeVectorPdfIndependentError> {
    const ROOT_TOKEN_COUNT: usize = 8;
    const USAGE_TOKEN_COUNT: usize = 19;
    let root = tokens
        .get(..ROOT_TOKEN_COUNT)
        .ok_or(SafeVectorPdfIndependentError::InvalidPageOperators)?;
    if root[0] != b"q"
        || root[1] != b"1"
        || root[2] != b"0"
        || root[3] != b"0"
        || root[4] != b"-1"
        || root[5] != b"0"
        || root[6] != pdf_fixed(page_height).as_bytes()
        || root[7] != b"cm"
    {
        return Err(SafeVectorPdfIndependentError::InvalidPageOperators);
    }
    let mut cursor = ROOT_TOKEN_COUNT;
    let mut used_names = BTreeSet::new();
    for ((expected_form_index, expected_matrix), expected_color) in expected_form_indices
        .iter()
        .zip(expected_matrices)
        .zip(expected_colors)
    {
        let end = cursor
            .checked_add(USAGE_TOKEN_COUNT)
            .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
        let usage = tokens
            .get(cursor..end)
            .ok_or(SafeVectorPdfIndependentError::InvalidPageOperators)?;
        let color_is_closed = usage[1..4] == usage[5..8]
            && usage[1..4]
                .iter()
                .zip(expected_color)
                .all(|(observed, expected)| {
                    *observed == pdf_fixed(color_fixed(*expected)).as_bytes()
                });
        let matrix_is_exact = usage[9..15]
            .iter()
            .zip(expected_matrix)
            .all(|(observed, raw)| *observed == pdf_fixed(*raw).as_bytes());
        let name = usage[16];
        let expected_name = format!("/V{expected_form_index}");
        if usage[0] != b"q"
            || !color_is_closed
            || usage[4] != b"rg"
            || usage[8] != b"RG"
            || !matrix_is_exact
            || usage[15] != b"cm"
            || !is_vector_resource_name(name)
            || name != expected_name.as_bytes()
            || !bindings.contains(name)
            || usage[17] != b"Do"
            || usage[18] != b"Q"
        {
            return Err(SafeVectorPdfIndependentError::InvalidPageOperators);
        }
        used_names.insert(name.to_vec());
        cursor = end;
    }
    if tokens.get(cursor) != Some(&b"Q".as_slice()) || cursor.checked_add(1) != Some(tokens.len()) {
        return Err(SafeVectorPdfIndependentError::InvalidPageOperators);
    }
    Ok(used_names)
}

fn page_media_box_height(body: &[u8]) -> Result<i64, SafeVectorPdfIndependentError> {
    const PREFIX: &[u8] = b"/MediaBox [0 0 ";
    if byte_count(body, PREFIX)? != 1 {
        return Err(SafeVectorPdfIndependentError::InvalidPageTransform);
    }
    let start = find_from(body, PREFIX, 0)
        .and_then(|offset| offset.checked_add(PREFIX.len()))
        .ok_or(SafeVectorPdfIndependentError::InvalidPageTransform)?;
    let end =
        find_from(body, b"]", start).ok_or(SafeVectorPdfIndependentError::InvalidPageTransform)?;
    let components = ascii_tokens(
        body.get(start..end)
            .ok_or(SafeVectorPdfIndependentError::InvalidPageTransform)?,
    )?;
    if components.len() != 2
        || !canonical_positive_fixed(components[0])
        || !canonical_positive_fixed(components[1])
    {
        return Err(SafeVectorPdfIndependentError::InvalidPageTransform);
    }
    parse_canonical_nonnegative_fixed(
        std::str::from_utf8(components[1])
            .map_err(|_| SafeVectorPdfIndependentError::InvalidPageTransform)?,
    )
    .ok_or(SafeVectorPdfIndependentError::InvalidPageTransform)
}

fn color_fixed(byte: u8) -> i64 {
    (i64::from(byte) * 65_536 + 127) / 255
}

fn canonical_positive_fixed(value: &[u8]) -> bool {
    let Ok(parsed) = std::str::from_utf8(value) else {
        return false;
    };
    let Some(raw) = parse_canonical_nonnegative_fixed(parsed) else {
        return false;
    };
    raw > 0 && pdf_fixed(raw).as_bytes() == value
}

fn parse_canonical_nonnegative_fixed(value: &str) -> Option<i64> {
    let (integer, fraction) = value
        .split_once('.')
        .map_or((value, None), |(integer, fraction)| {
            (integer, Some(fraction))
        });
    if integer.is_empty()
        || (integer.len() > 1 && integer.starts_with('0'))
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let integer = integer.parse::<u128>().ok()?;
    let mut raw = integer.checked_mul(65_536)?;
    if let Some(fraction) = fraction {
        if fraction.is_empty()
            || fraction.len() > 16
            || fraction.ends_with('0')
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        let digits = fraction.parse::<u128>().ok()?;
        let denominator = 10u128.checked_pow(fraction.len() as u32)?;
        let scaled = digits.checked_mul(65_536)?;
        if scaled % denominator != 0 {
            return None;
        }
        raw = raw.checked_add(scaled / denominator)?;
    }
    i64::try_from(raw).ok()
}

fn is_vector_resource_name(value: &[u8]) -> bool {
    value
        .strip_prefix(b"/V")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.iter().all(u8::is_ascii_digit))
}

pub(super) fn parse_objects(
    pdf: &[u8],
) -> Result<Vec<ParsedObject<'_>>, SafeVectorPdfIndependentError> {
    let mut objects = Vec::new();
    let mut cursor = 0usize;
    while let Some(marker) = find_from(pdf, b" 0 obj\n", cursor) {
        let line_start = pdf[..marker]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        let number = std::str::from_utf8(&pdf[line_start..marker])
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| *value > 0)
            .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
        let body_start = marker
            .checked_add(b" 0 obj\n".len())
            .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
        let body_end = find_from(pdf, b"\nendobj\n", body_start)
            .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
        if objects
            .iter()
            .any(|object: &ParsedObject<'_>| object.number == number)
        {
            return Err(SafeVectorPdfIndependentError::DuplicateObject);
        }
        objects.push(ParsedObject {
            number,
            body: &pdf[body_start..body_end],
        });
        cursor = body_end
            .checked_add(b"\nendobj\n".len())
            .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
    }
    if objects.is_empty() {
        return Err(SafeVectorPdfIndependentError::MalformedPdf);
    }
    objects.sort_unstable_by_key(|object| object.number);
    if objects
        .iter()
        .enumerate()
        .any(|(index, object)| u32::try_from(index + 1).ok() != Some(object.number))
    {
        return Err(SafeVectorPdfIndependentError::MalformedPdf);
    }
    Ok(objects)
}

fn inspect_form(
    body: &[u8],
    bbox: [i64; 4],
    expected_content_sha256: [u8; 32],
    ext_object_numbers: &BTreeSet<u32>,
) -> Result<BTreeSet<u32>, SafeVectorPdfIndependentError> {
    if !contains(body, b"/Type /XObject")
        || !contains(body, b"/FormType 1")
        || !contains(body, b"/Resources << /ExtGState <<")
        || byte_count(body, b"/BBox ")? != 1
    {
        return Err(SafeVectorPdfIndependentError::InvalidFormResources);
    }
    let expected_bbox = format!(
        "/BBox [{} {} {} {}]",
        pdf_fixed(bbox[0]),
        pdf_fixed(bbox[1]),
        pdf_fixed(bbox[2]),
        pdf_fixed(bbox[3])
    );
    if !contains(body, expected_bbox.as_bytes()) {
        return Err(SafeVectorPdfIndependentError::UnexpectedBBox);
    }
    let stream = object_stream(body)?.ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
    if sha256(stream) != expected_content_sha256 {
        return Err(SafeVectorPdfIndependentError::InvalidFormOperators);
    }
    let bindings = form_ext_g_state_bindings(body, ext_object_numbers)?;
    if [
        b"/MCID".as_slice(),
        b"/Alt",
        b"/ActualText",
        b"/Lang",
        b"BDC",
        b"BMC",
    ]
    .iter()
    .any(|value| contains(stream, value))
    {
        return Err(SafeVectorPdfIndependentError::FormSemanticContent);
    }
    let tokens = ascii_tokens(stream)?;
    let q = token_count_from_tokens(&tokens, b"q")?;
    let restore = token_count_from_tokens(&tokens, b"Q")?;
    let gs = token_count_from_tokens(&tokens, b"gs")?;
    let paints = [b"S".as_slice(), b"f", b"f*", b"B", b"B*"]
        .into_iter()
        .try_fold(0u32, |total, operator| {
            total
                .checked_add(token_count_from_tokens(&tokens, operator)?)
                .ok_or(SafeVectorPdfIndependentError::MalformedPdf)
        })?;
    let stroked_paints =
        [b"S".as_slice(), b"B", b"B*"]
            .into_iter()
            .try_fold(0u32, |total, operator| {
                total
                    .checked_add(token_count_from_tokens(&tokens, operator)?)
                    .ok_or(SafeVectorPdfIndependentError::MalformedPdf)
            })?;
    let stroke_style_is_closed =
        [b"w".as_slice(), b"J", b"j", b"M"]
            .into_iter()
            .try_fold(true, |closed, operator| {
                Ok::<_, SafeVectorPdfIndependentError>(
                    closed && token_count_from_tokens(&tokens, operator)? == stroked_paints,
                )
            })?;
    let mut used_ext_g_states = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        if *token != b"gs" {
            continue;
        }
        let name = index
            .checked_sub(1)
            .and_then(|previous| tokens.get(previous))
            .filter(|name| is_ext_g_state_resource_name(name))
            .ok_or(SafeVectorPdfIndependentError::InvalidFormResources)?;
        if !bindings.contains_key(*name) {
            return Err(SafeVectorPdfIndependentError::InvalidFormResources);
        }
        used_ext_g_states.insert((*name).to_vec());
    }
    if q == 0
        || q != restore
        || gs == 0
        || gs != paints
        || token_count_from_tokens(&tokens, b"re")? == 0
        || token_count_from_tokens(&tokens, b"W")?
            .checked_add(token_count_from_tokens(&tokens, b"W*")?)
            == Some(0)
        || token_count_from_tokens(&tokens, b"m")? == 0
        || token_count_from_tokens(&tokens, b"cm")? == 0
        || (stroked_paints > 0 && !stroke_style_is_closed)
    {
        return Err(SafeVectorPdfIndependentError::InvalidFormOperators);
    }
    if used_ext_g_states != bindings.keys().cloned().collect() {
        return Err(SafeVectorPdfIndependentError::InvalidFormResources);
    }
    Ok(bindings.into_values().collect())
}

fn form_ext_g_state_bindings(
    body: &[u8],
    ext_object_numbers: &BTreeSet<u32>,
) -> Result<BTreeMap<Vec<u8>, u32>, SafeVectorPdfIndependentError> {
    let dictionary_end =
        find_from(body, b"\nstream\n", 0).ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
    let tokens = ascii_tokens(&body[..dictionary_end])?;
    let mut bindings = BTreeMap::new();
    for (index, token) in tokens.iter().enumerate() {
        if !is_ext_g_state_resource_name(token) {
            continue;
        }
        let expected_name = format!("/GS{}", bindings.len());
        let target = index
            .checked_add(1)
            .and_then(|index| tokens.get(index))
            .and_then(|value| std::str::from_utf8(value).ok())
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|target| ext_object_numbers.contains(target))
            .ok_or(SafeVectorPdfIndependentError::InvalidFormResources)?;
        if *token != expected_name.as_bytes()
            || tokens.get(index + 2) != Some(&b"0".as_slice())
            || tokens.get(index + 3) != Some(&b"R".as_slice())
            || bindings.insert((*token).to_vec(), target).is_some()
        {
            return Err(SafeVectorPdfIndependentError::InvalidFormResources);
        }
    }
    if bindings.is_empty() {
        return Err(SafeVectorPdfIndependentError::InvalidFormResources);
    }
    Ok(bindings)
}

fn is_ext_g_state_resource_name(value: &[u8]) -> bool {
    value
        .strip_prefix(b"/GS")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.iter().all(u8::is_ascii_digit))
}

fn inspect_ext_g_state(
    body: &[u8],
    expected_alpha_pair: [u32; 2],
) -> Result<(), SafeVectorPdfIndependentError> {
    let tokens = ascii_tokens(body)?;
    if tokens.len() != 8
        || tokens[0] != b"<<"
        || tokens[1] != b"/Type"
        || tokens[2] != b"/ExtGState"
        || tokens[3] != b"/ca"
        || tokens[5] != b"/CA"
        || !valid_unit_interval(tokens[4])
        || !valid_unit_interval(tokens[6])
        || tokens[7] != b">>"
    {
        return Err(SafeVectorPdfIndependentError::InvalidExtGState);
    }
    let expected = format!(
        "<< /Type /ExtGState /ca {} /CA {} >>",
        pdf_fixed(i64::from(expected_alpha_pair[0])),
        pdf_fixed(i64::from(expected_alpha_pair[1]))
    );
    if body != expected.as_bytes() {
        return Err(SafeVectorPdfIndependentError::InvalidExtGState);
    }
    Ok(())
}

fn valid_unit_interval(value: &[u8]) -> bool {
    if value == b"0" || value == b"1" {
        return true;
    }
    let Some(fraction) = value.strip_prefix(b"0.") else {
        return false;
    };
    if fraction.is_empty()
        || fraction.len() > 16
        || !fraction.iter().all(u8::is_ascii_digit)
        || fraction.last() == Some(&b'0')
    {
        return false;
    }
    let Some(digits) = std::str::from_utf8(fraction)
        .ok()
        .and_then(|value| value.parse::<u128>().ok())
    else {
        return false;
    };
    let Some(denominator) = 10u128.checked_pow(fraction.len() as u32) else {
        return false;
    };
    let Some(scaled) = digits.checked_mul(65_536) else {
        return false;
    };
    if scaled % denominator != 0 {
        return false;
    }
    let raw = scaled / denominator;
    raw <= 65_536 && pdf_fixed(raw as i64).as_bytes() == value
}

pub(super) fn object_stream(body: &[u8]) -> Result<Option<&[u8]>, SafeVectorPdfIndependentError> {
    let Some(start) = find_from(body, b"stream\n", 0) else {
        return Ok(None);
    };
    let start = start
        .checked_add(b"stream\n".len())
        .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
    let end = find_from(body, b"\nendstream", start)
        .ok_or(SafeVectorPdfIndependentError::MalformedPdf)?;
    Ok(Some(&body[start..end]))
}

fn count_objects(
    objects: &[ParsedObject<'_>],
    needle: &[u8],
    exclusion: &[u8],
) -> Result<u32, SafeVectorPdfIndependentError> {
    u32::try_from(
        objects
            .iter()
            .filter(|object| contains(object.body, needle) && !contains(object.body, exclusion))
            .count(),
    )
    .map_err(|_| SafeVectorPdfIndependentError::MalformedPdf)
}

fn token_count_from_tokens(
    tokens: &[&[u8]],
    token: &[u8],
) -> Result<u32, SafeVectorPdfIndependentError> {
    u32::try_from(
        tokens
            .iter()
            .filter(|candidate| **candidate == token)
            .count(),
    )
    .map_err(|_| SafeVectorPdfIndependentError::MalformedPdf)
}

pub(super) fn ascii_tokens(value: &[u8]) -> Result<Vec<&[u8]>, SafeVectorPdfIndependentError> {
    if !value.is_ascii() {
        return Err(SafeVectorPdfIndependentError::MalformedPdf);
    }
    Ok(value
        .split(|byte| byte.is_ascii_whitespace())
        .filter(|token| !token.is_empty())
        .collect())
}

fn byte_count(value: &[u8], needle: &[u8]) -> Result<u32, SafeVectorPdfIndependentError> {
    u32::try_from(
        value
            .windows(needle.len())
            .filter(|window| *window == needle)
            .count(),
    )
    .map_err(|_| SafeVectorPdfIndependentError::MalformedPdf)
}

pub(super) fn contains(value: &[u8], needle: &[u8]) -> bool {
    value.windows(needle.len()).any(|window| window == needle)
}

pub(super) fn find_from(value: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    value
        .get(start..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .and_then(|position| start.checked_add(position))
}

fn pdf_fixed(raw: i64) -> String {
    const FIXED_ONE: u64 = 65_536;
    const BINARY_TO_DECIMAL: u64 = 152_587_890_625;
    let negative = raw < 0;
    let magnitude = raw.unsigned_abs();
    let integer = magnitude / FIXED_ONE;
    let fraction = magnitude % FIXED_ONE;
    if fraction == 0 {
        return if negative {
            format!("-{integer}")
        } else {
            integer.to_string()
        };
    }
    let mut fraction_text = format!("{:016}", fraction * BINARY_TO_DECIMAL);
    while fraction_text.ends_with('0') {
        fraction_text.pop();
    }
    if negative {
        format!("-{integer}.{fraction_text}")
    } else {
        format!("{integer}.{fraction_text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORM_STREAM: &[u8] = b"q\n0 0 30 12 re W n\n1 0 0 1 0 0 cm\nq\n/GS0 gs\n0 0 m\n1 1 l\n1 w\n0 J\n0 j\n10 M\nS\nQ\nQ";
    const PAGE_STREAM: &[u8] =
        b"q\n1 0 0 -1 0 140 cm\nq\n0 0 0 rg\n0 0 0 RG\n1 0 0 1 0 0 cm\n/V0 Do\nQ\nQ";

    fn synthetic_expectations() -> SafeVectorPdfIndependentExpectations {
        SafeVectorPdfIndependentExpectations::new(
            1,
            vec![[0, 0, 30 * 65_536, 12 * 65_536]],
            vec![sha256(FORM_STREAM)],
            vec![[65_536, 65_536]],
            vec![0],
            vec![[65_536, 0, 0, 65_536, 0, 0]],
            vec![[0, 0, 0]],
        )
        .unwrap()
    }

    #[test]
    fn independent_parser_accepts_actual_mi4_v13_isolated_writer_output() {
        let fixture =
            typaxis_display_list::staging_precomposed_vector_display_ten_use_fixture().unwrap();
        let registry = typaxis_resources::VectorContentCandidateRegistry::from_admitted(
            &fixture.layout.admitted,
            fixture.layout.package.resources(),
        )
        .unwrap();
        let plans = typaxis_resources::finalize_staging_safe_vector_forms_v2(
            &fixture.display,
            &registry,
            &fixture.layout.limits,
        )
        .unwrap();
        let contribution = typaxis_pdf::build_staging_safe_vector_pdf_contribution_v2(
            &fixture.display,
            &plans,
            &registry,
            &fixture.layout.limits,
        )
        .unwrap();
        let isolated = typaxis_pdf::staging_safe_vector_isolated_pdf_fixture_v2(
            &contribution,
            240 * 65_536,
            140 * 65_536,
        )
        .unwrap();
        let expected = SafeVectorPdfIndependentExpectations::new(
            u32::try_from(contribution.pages().len()).unwrap(),
            contribution
                .forms()
                .iter()
                .map(typaxis_pdf::StagingSafeVectorPdfFormV2::bbox)
                .collect(),
            contribution
                .forms()
                .iter()
                .map(typaxis_pdf::StagingSafeVectorPdfFormV2::content_stream_fingerprint)
                .collect(),
            contribution
                .ext_g_states()
                .iter()
                .map(|ext| [ext.fill_alpha_raw(), ext.stroke_alpha_raw()])
                .collect(),
            vec![0; contribution.usages().len()],
            contribution
                .usages()
                .iter()
                .map(|usage| {
                    let matrix = usage.matrix();
                    [
                        i64::from(matrix.a.raw()),
                        i64::from(matrix.b.raw()),
                        i64::from(matrix.c.raw()),
                        i64::from(matrix.d.raw()),
                        matrix.e.raw(),
                        matrix.f.raw(),
                    ]
                })
                .collect(),
            vec![[0, 0, 0]; contribution.usages().len()],
        )
        .unwrap();
        let report = inspect_safe_vector_pdf(isolated.bytes(), &expected).unwrap();
        assert_eq!(report.pdf_sha256(), sha256(isolated.bytes()));
        assert_eq!(report.form_count(), 1);
        assert_eq!(report.do_count(), 10);
        assert_eq!(report.page_root_y_flip_count(), 1);
    }

    fn pdf(ext: &[u8], form_stream: &[u8], page_stream: &[u8]) -> Vec<u8> {
        let form = format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 30 12] /Resources << /ExtGState << /GS0 6 0 R >> >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            form_stream.len(),
            std::str::from_utf8(form_stream).unwrap()
        );
        let page = format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            page_stream.len(),
            std::str::from_utf8(page_stream).unwrap()
        );
        format!(
            "%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 240 140] /Resources << /XObject << /V0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n{page}{form}6 0 obj\n{}\nendobj\nxref\ntrailer\n%%EOF\n",
            std::str::from_utf8(ext).unwrap()
        )
        .into_bytes()
    }

    #[test]
    fn independent_safe_vector_pdf_parser_accepts_vector_form_and_page_do() {
        let bytes = pdf(
            b"<< /Type /ExtGState /ca 1 /CA 1 >>",
            FORM_STREAM,
            PAGE_STREAM,
        );
        let expected = synthetic_expectations();
        let report = inspect_safe_vector_pdf(&bytes, &expected).unwrap();
        assert_eq!(report.page_count(), 1);
        assert_eq!(report.form_count(), 1);
        assert_eq!(report.ext_g_state_count(), 1);
        assert_eq!(report.do_count(), 1);
        assert_eq!(report.page_root_y_flip_count(), 1);

        let mut wrong_alpha = expected.clone();
        wrong_alpha.ext_g_state_alpha_pairs[0][0] = 32_768;
        assert_eq!(
            inspect_safe_vector_pdf(&bytes, &wrong_alpha),
            Err(SafeVectorPdfIndependentError::InvalidExtGState)
        );
        let mut wrong_matrix = expected;
        wrong_matrix.placement_matrices[0][4] = 1;
        assert_eq!(
            inspect_safe_vector_pdf(&bytes, &wrong_matrix),
            Err(SafeVectorPdfIndependentError::InvalidPageOperators)
        );

        let mut wrong_color = synthetic_expectations();
        wrong_color.placement_colors_rgb8[0] = [1, 2, 3];
        assert_eq!(
            inspect_safe_vector_pdf(&bytes, &wrong_color),
            Err(SafeVectorPdfIndependentError::InvalidPageOperators)
        );

        let mut wrong_form = synthetic_expectations();
        wrong_form.placement_form_indices[0] = 1;
        assert_eq!(
            inspect_safe_vector_pdf(&bytes, &wrong_form),
            Err(SafeVectorPdfIndependentError::InvalidPageOperators)
        );
    }

    #[test]
    fn independent_safe_vector_pdf_parser_rejects_raster_semantics_and_alpha_keys() {
        let expected = synthetic_expectations();
        let raster = pdf(
            b"<< /Type /ExtGState /ca 1 /CA 1 >>",
            b"q /Subtype /Image Q",
            b"q 1 0 0 -1 0 140 cm /V0 Do Q",
        );
        assert_eq!(
            inspect_safe_vector_pdf(&raster, &expected),
            Err(SafeVectorPdfIndependentError::RasterContent)
        );
        let extra_alpha = pdf(
            b"<< /Type /ExtGState /ca 1 /CA 1 /BM /Normal >>",
            FORM_STREAM,
            PAGE_STREAM,
        );
        assert_eq!(
            inspect_safe_vector_pdf(&extra_alpha, &expected),
            Err(SafeVectorPdfIndependentError::InvalidExtGState)
        );
        let mut wrong_ext_target = pdf(
            b"<< /Type /ExtGState /ca 1 /CA 1 >>",
            FORM_STREAM,
            PAGE_STREAM,
        );
        let target = find_from(&wrong_ext_target, b"/GS0 6 0 R", 0).unwrap() + b"/GS0 ".len();
        wrong_ext_target[target] = b'5';
        assert_eq!(
            inspect_safe_vector_pdf(&wrong_ext_target, &expected),
            Err(SafeVectorPdfIndependentError::InvalidFormResources)
        );
        let missing_matrix = pdf(
            b"<< /Type /ExtGState /ca 1 /CA 1 >>",
            FORM_STREAM,
            b"q 1 0 0 -1 0 140 cm q 0 0 0 rg 0 0 0 RG /V0 Do Q Q",
        );
        assert_eq!(
            inspect_safe_vector_pdf(&missing_matrix, &expected),
            Err(SafeVectorPdfIndependentError::InvalidPageOperators)
        );
        let nonfinite_alpha = pdf(
            b"<< /Type /ExtGState /ca NaN /CA 1 >>",
            FORM_STREAM,
            PAGE_STREAM,
        );
        assert_eq!(
            inspect_safe_vector_pdf(&nonfinite_alpha, &expected),
            Err(SafeVectorPdfIndependentError::InvalidExtGState)
        );

        let wrong_root_height = pdf(
            b"<< /Type /ExtGState /ca 1 /CA 1 >>",
            FORM_STREAM,
            b"q 1 0 0 -1 0 139 cm q 0 0 0 rg 0 0 0 RG 1 0 0 1 0 0 cm /V0 Do Q Q",
        );
        assert_eq!(
            inspect_safe_vector_pdf(&wrong_root_height, &expected),
            Err(SafeVectorPdfIndependentError::InvalidPageOperators)
        );
    }
}
