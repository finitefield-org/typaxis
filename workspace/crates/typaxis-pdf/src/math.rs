use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{push_jcs_string, sha256, FontFaceId, M4EffectiveResourceLimits};
use typaxis_display_list::StagingMathDisplay;
use typaxis_font::MathFontFace;
use typaxis_math::MathPaint;
use typaxis_resource_admission::AdmittedResourceLedger;
use typaxis_syntax::{
    StagingMathProfileAuthorization, StagingMathProfileProgressToken,
    ValidatedStagingSemanticPackage,
};

pub const MATH_PDF_ALGORITHM: &str = "typaxis.math-pdf-observation/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathPdfObservation {
    occurrence: u32,
    receipt_key: [u8; 32],
    page_index: u32,
    page_object: u32,
    content_object: u32,
    font_object: u32,
    actual_text_sha256: [u8; 32],
    vector_fingerprint: [u8; 32],
    marked_content_sha256: [u8; 32],
    fingerprint: [u8; 32],
}

impl StagingMathPdfObservation {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn receipt_key(&self) -> [u8; 32] {
        self.receipt_key
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn page_object(&self) -> u32 {
        self.page_object
    }
    pub const fn content_object(&self) -> u32 {
        self.content_object
    }
    pub const fn font_object(&self) -> u32 {
        self.font_object
    }
    pub const fn actual_text_sha256(&self) -> [u8; 32] {
        self.actual_text_sha256
    }
    pub const fn vector_fingerprint(&self) -> [u8; 32] {
        self.vector_fingerprint
    }
    pub const fn marked_content_sha256(&self) -> [u8; 32] {
        self.marked_content_sha256
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathPdf {
    bytes: Vec<u8>,
    display_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    profile_progress: StagingMathProfileProgressToken,
    observations: Vec<StagingMathPdfObservation>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathPdf {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn observations(&self) -> &[StagingMathPdfObservation] {
        &self.observations
    }
    pub const fn display_fingerprint(&self) -> [u8; 32] {
        self.display_fingerprint
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
    pub const fn profile_progress(&self) -> &StagingMathProfileProgressToken {
        &self.profile_progress
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingMathProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        display: &StagingMathDisplay,
    ) -> Result<(), StagingMathPdfError> {
        validate_input(package, profile, limits, admitted, display)?;
        let object_plan = plan_pdf_objects(display, limits)?;
        if self.display_fingerprint != display.fingerprint()
            || self.profile_fingerprint != profile.profile_receipt_fingerprint()
            || !profile.matches_progress(&self.profile_progress)
            || &self.profile_progress != display.profile_progress()
            || self.observations.len() != display.draws().len()
            || !self.bytes.starts_with(b"%PDF-1.7\n")
            || !self.bytes.ends_with(b"%%EOF\n")
            || u64::try_from(self.bytes.len())
                .map_or(true, |length| length > limits.base().get().max_output_bytes)
        {
            return Err(StagingMathPdfError::ReceiptMismatch);
        }
        for (index, ((observation, draw), node)) in self
            .observations
            .iter()
            .zip(display.draws())
            .zip(package.math_nodes())
            .enumerate()
        {
            let expected_content = object_plan
                .page_start
                .checked_add(
                    observation
                        .page_index
                        .checked_mul(2)
                        .ok_or(StagingMathPdfError::ObjectLimit)?,
                )
                .ok_or(StagingMathPdfError::ObjectLimit)?;
            let expected_page = expected_content
                .checked_add(1)
                .ok_or(StagingMathPdfError::ObjectLimit)?;
            let expected_font = object_plan
                .font_objects
                .get(&draw.font_face_id())
                .map(|objects| objects.3)
                .ok_or(StagingMathPdfError::MissingFont)?;
            let expected_group = encode_marked_content(
                draw,
                draw.font_face_id(),
                profile.page_geometry().page_height().get().raw(),
                limits.base().get().max_output_bytes,
            )?;
            if usize::try_from(observation.occurrence) != Ok(index)
                || observation.receipt_key != draw.receipt_key().bytes()
                || observation.page_index != draw.page_index()
                || observation.page_index >= object_plan.page_count
                || observation.content_object != expected_content
                || observation.page_object != expected_page
                || observation.font_object != expected_font
                || observation.actual_text_sha256 != sha256(draw.actual_text().as_bytes())
                || observation.actual_text_sha256 != sha256(node.domain().speech.as_bytes())
                || observation.vector_fingerprint != draw.vector_fingerprint()
                || observation.marked_content_sha256 != sha256(&expected_group)
                || sha256(encode_observation(observation).as_bytes()) != observation.fingerprint
            {
                return Err(StagingMathPdfError::ReceiptMismatch);
            }
        }
        let actual_text = extract_actual_text(&self.bytes, &self.observations)?;
        if actual_text
            != package
                .math_nodes()
                .iter()
                .map(|value| value.domain().speech.clone())
                .collect::<Vec<_>>()
        {
            return Err(StagingMathPdfError::ActualTextMismatch);
        }
        let canonical_jcs = encode_pdf(
            display.fingerprint(),
            profile.profile_receipt_fingerprint(),
            sha256(&self.bytes),
            &self.observations,
        );
        if self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
        {
            return Err(StagingMathPdfError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMathPdfError {
    DisplayMismatch,
    MissingFont,
    ObjectLimit,
    OutputLimit,
    ArithmeticOverflow,
    ActualTextMismatch,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingMathPdfError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DisplayMismatch => formatter.write_str("I9190: math Display mismatch at PDF"),
            Self::MissingFont => formatter.write_str("R7100: math PDF font binding is missing"),
            Self::ObjectLimit => formatter.write_str("G6100: math PDF object limit exceeded"),
            Self::OutputLimit => formatter.write_str("G6101: math PDF output limit exceeded"),
            Self::ArithmeticOverflow => formatter.write_str("G6100: math PDF arithmetic overflow"),
            Self::ActualTextMismatch => formatter.write_str("I9190: math PDF ActualText mismatch"),
            Self::ReceiptMismatch => formatter.write_str("I9190: math PDF observation mismatch"),
            Self::AllocationFailure => formatter.write_str("G6100: math PDF allocation failed"),
        }
    }
}

impl std::error::Error for StagingMathPdfError {}

struct MathPdfObjectPlan {
    object_count: u32,
    page_count: u32,
    page_start: u32,
    font_objects: BTreeMap<FontFaceId, (u32, u32, u32, u32)>,
}

fn plan_pdf_objects(
    display: &StagingMathDisplay,
    limits: &M4EffectiveResourceLimits,
) -> Result<MathPdfObjectPlan, StagingMathPdfError> {
    let font_ids: BTreeSet<_> = display
        .draws()
        .iter()
        .map(|draw| draw.font_face_id())
        .collect();
    let page_count = display
        .draws()
        .iter()
        .map(|draw| draw.page_index())
        .max()
        .and_then(|page| page.checked_add(1))
        .unwrap_or(1);
    let font_count = u32::try_from(font_ids.len()).map_err(|_| StagingMathPdfError::ObjectLimit)?;
    let page_start = 3u32
        .checked_add(
            font_count
                .checked_mul(4)
                .ok_or(StagingMathPdfError::ObjectLimit)?,
        )
        .ok_or(StagingMathPdfError::ObjectLimit)?;
    let object_count = 2u32
        .checked_add(
            font_count
                .checked_mul(4)
                .ok_or(StagingMathPdfError::ObjectLimit)?,
        )
        .and_then(|value| value.checked_add(page_count.checked_mul(2)?))
        .ok_or(StagingMathPdfError::ObjectLimit)?;
    if object_count > limits.base().get().max_pdf_objects {
        return Err(StagingMathPdfError::ObjectLimit);
    }
    let mut font_objects = BTreeMap::new();
    for (index, id) in font_ids.into_iter().enumerate() {
        let start = 3u32
            .checked_add(
                u32::try_from(index)
                    .map_err(|_| StagingMathPdfError::ObjectLimit)?
                    .checked_mul(4)
                    .ok_or(StagingMathPdfError::ObjectLimit)?,
            )
            .ok_or(StagingMathPdfError::ObjectLimit)?;
        font_objects.insert(
            id,
            (
                start,
                start
                    .checked_add(1)
                    .ok_or(StagingMathPdfError::ObjectLimit)?,
                start
                    .checked_add(2)
                    .ok_or(StagingMathPdfError::ObjectLimit)?,
                start
                    .checked_add(3)
                    .ok_or(StagingMathPdfError::ObjectLimit)?,
            ),
        );
    }
    Ok(MathPdfObjectPlan {
        object_count,
        page_count,
        page_start,
        font_objects,
    })
}

pub fn write_staging_math_pdf(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingMathProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    display: &StagingMathDisplay,
) -> Result<StagingMathPdf, StagingMathPdfError> {
    validate_input(package, profile, limits, admitted, display)?;
    let max_output_bytes = limits.base().get().max_output_bytes;
    let object_plan = plan_pdf_objects(display, limits)?;
    let page_count = object_plan.page_count;
    let object_count = object_plan.object_count;
    let page_start = object_plan.page_start;
    let font_objects = &object_plan.font_objects;
    let mut used_glyphs: BTreeMap<FontFaceId, BTreeSet<u16>> = BTreeMap::new();
    for draw in display.draws() {
        let glyphs = used_glyphs.entry(draw.font_face_id()).or_default();
        for paint in draw.paints() {
            if let MathPaint::Glyph(glyph) = paint {
                glyphs.insert(glyph.original_gid().get());
            }
        }
    }
    let mut objects: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
    let mut stored_object_bytes = 0u64;
    insert_pdf_object(
        &mut objects,
        &mut stored_object_bytes,
        1,
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        max_output_bytes,
    )?;
    let mut pages = format!("<< /Type /Pages /Count {page_count} /Kids [");
    for page in 0..page_count {
        let page_object = page_start + page * 2 + 1;
        pages.push_str(&format!("{page_object} 0 R "));
    }
    pages.push_str("] >>");
    insert_pdf_object(
        &mut objects,
        &mut stored_object_bytes,
        2,
        pages.into_bytes(),
        max_output_bytes,
    )?;
    for (id, (font_file, descriptor, cid_font, type0)) in font_objects {
        let font = admitted.font(*id).ok_or(StagingMathPdfError::MissingFont)?;
        let face = MathFontFace::parse(font.bytes(), font.face_index())
            .map_err(|_| StagingMathPdfError::MissingFont)?;
        let font_program = face
            .standalone_truetype_program()
            .map_err(|_| StagingMathPdfError::MissingFont)?;
        let units_per_em = face.units_per_em();
        let postscript_name = face
            .postscript_name()
            .map_err(|_| StagingMathPdfError::MissingFont)?;
        let pdf_postscript_name = escape_pdf_name(&postscript_name);
        let (x_min, y_min, x_max, y_max) = face.bbox();
        let mut stream = Vec::new();
        extend_bounded(
            &mut stream,
            format!(
                "<< /Length {} /Length1 {} >>\nstream\n",
                font_program.len(),
                font_program.len()
            )
            .as_bytes(),
            max_output_bytes,
        )?;
        extend_bounded(&mut stream, &font_program, max_output_bytes)?;
        extend_bounded(&mut stream, b"\nendstream", max_output_bytes)?;
        insert_pdf_object(
            &mut objects,
            &mut stored_object_bytes,
            *font_file,
            stream,
            max_output_bytes,
        )?;
        insert_pdf_object(
            &mut objects,
            &mut stored_object_bytes,
            *descriptor,
            format!(
                "<< /Type /FontDescriptor /FontName /{} /Flags 4 /FontBBox [{} {} {} {}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {} 0 R >>",
                pdf_postscript_name,
                pdf_font_unit(i64::from(x_min), units_per_em)?,
                pdf_font_unit(i64::from(y_min), units_per_em)?,
                pdf_font_unit(i64::from(x_max), units_per_em)?,
                pdf_font_unit(i64::from(y_max), units_per_em)?,
                pdf_font_unit(i64::from(face.ascent()), units_per_em)?,
                pdf_font_unit(i64::from(face.descent()), units_per_em)?,
                pdf_font_unit(i64::from(y_max), units_per_em)?,
                font_file
            )
            .into_bytes(),
            max_output_bytes,
        )?;
        let mut widths = String::from("[");
        for glyph in used_glyphs.get(id).into_iter().flatten() {
            let width = face
                .advance_width(typaxis_font::OriginalGlyphId::new(*glyph))
                .map_err(|_| StagingMathPdfError::MissingFont)?;
            widths.push_str(&format!(
                "{} [{}] ",
                glyph,
                pdf_font_unit(i64::from(width), units_per_em)?
            ));
        }
        widths.push(']');
        insert_pdf_object(
            &mut objects,
            &mut stored_object_bytes,
            *cid_font,
            format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {} 0 R /DW 1000 /W {} /CIDToGIDMap /Identity >>", pdf_postscript_name, descriptor, widths).into_bytes(),
            max_output_bytes,
        )?;
        insert_pdf_object(
            &mut objects,
            &mut stored_object_bytes,
            *type0,
            format!("<< /Type /Font /Subtype /Type0 /BaseFont /{} /Encoding /Identity-H /DescendantFonts [{} 0 R] >>", pdf_postscript_name, cid_font).into_bytes(),
            max_output_bytes,
        )?;
    }

    let page_height = profile.page_geometry().page_height().get().raw();
    let mut observations = Vec::new();
    observations
        .try_reserve_exact(display.draws().len())
        .map_err(|_| StagingMathPdfError::AllocationFailure)?;
    for page in 0..page_count {
        let content_object = page_start + page * 2;
        let page_object = content_object + 1;
        let mut content = Vec::new();
        for draw in display
            .draws()
            .iter()
            .filter(|draw| draw.page_index() == page)
        {
            let (_, _, _, font_object) = font_objects
                .get(&draw.font_face_id())
                .copied()
                .ok_or(StagingMathPdfError::MissingFont)?;
            let group =
                encode_marked_content(draw, draw.font_face_id(), page_height, max_output_bytes)?;
            let mut observation = StagingMathPdfObservation {
                occurrence: draw.occurrence(),
                receipt_key: draw.receipt_key().bytes(),
                page_index: page,
                page_object,
                content_object,
                font_object,
                actual_text_sha256: sha256(draw.actual_text().as_bytes()),
                vector_fingerprint: draw.vector_fingerprint(),
                marked_content_sha256: sha256(&group),
                fingerprint: [0; 32],
            };
            observation.fingerprint = sha256(encode_observation(&observation).as_bytes());
            observations.push(observation);
            extend_bounded(&mut content, &group, max_output_bytes)?;
        }
        let mut stream = Vec::new();
        extend_bounded(
            &mut stream,
            format!("<< /Length {} >>\nstream\n", content.len()).as_bytes(),
            max_output_bytes,
        )?;
        extend_bounded(&mut stream, &content, max_output_bytes)?;
        extend_bounded(&mut stream, b"endstream", max_output_bytes)?;
        insert_pdf_object(
            &mut objects,
            &mut stored_object_bytes,
            content_object,
            stream,
            max_output_bytes,
        )?;
        let mut resources = String::from("<< /Font << ");
        for (id, (_, _, _, type0)) in font_objects {
            resources.push_str(&format!("/M{} {} 0 R ", id.get(), type0));
        }
        resources.push_str(">> >>");
        insert_pdf_object(
            &mut objects,
            &mut stored_object_bytes,
            page_object,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources {} /Contents {} 0 R >>",
                pdf_number(
                    profile.page_geometry().page_width().get().raw()
                ),
                pdf_number(page_height),
                resources,
                content_object
            )
            .into_bytes(),
            max_output_bytes,
        )?;
    }
    observations.sort_by_key(|value| value.occurrence);
    let bytes = serialize_pdf(&objects, object_count, max_output_bytes)?;
    let canonical_jcs = encode_pdf(
        display.fingerprint(),
        profile.profile_receipt_fingerprint(),
        sha256(&bytes),
        &observations,
    );
    let pdf = StagingMathPdf {
        bytes,
        display_fingerprint: display.fingerprint(),
        profile_fingerprint: profile.profile_receipt_fingerprint(),
        profile_progress: profile.progress_token(),
        observations,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    pdf.verify(package, profile, limits, admitted, display)?;
    Ok(pdf)
}

fn insert_pdf_object(
    objects: &mut BTreeMap<u32, Vec<u8>>,
    stored_bytes: &mut u64,
    number: u32,
    value: Vec<u8>,
    maximum: u64,
) -> Result<(), StagingMathPdfError> {
    let value_bytes = u64::try_from(value.len()).map_err(|_| StagingMathPdfError::OutputLimit)?;
    let next = stored_bytes
        .checked_add(value_bytes)
        .ok_or(StagingMathPdfError::OutputLimit)?;
    if next > maximum {
        return Err(StagingMathPdfError::OutputLimit);
    }
    if objects.insert(number, value).is_some() {
        return Err(StagingMathPdfError::ReceiptMismatch);
    }
    *stored_bytes = next;
    Ok(())
}

fn validate_input(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingMathProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    display: &StagingMathDisplay,
) -> Result<(), StagingMathPdfError> {
    display
        .verify_sealed()
        .map_err(|_| StagingMathPdfError::DisplayMismatch)?;
    if profile.authorizes(package, limits).is_err()
        || display.profile_fingerprint() != profile.profile_receipt_fingerprint()
        || !profile.matches_progress(display.profile_progress())
        || display.admitted_fingerprint() != admitted.fingerprint().bytes()
        || !admitted
            .token()
            .matches_progress(display.admission_progress())
        || display.draws().len() != package.math_nodes().len()
    {
        return Err(StagingMathPdfError::DisplayMismatch);
    }
    for (draw, node) in display.draws().iter().zip(package.math_nodes()) {
        let font = admitted
            .font(draw.font_face_id())
            .ok_or(StagingMathPdfError::MissingFont)?;
        if draw.node_id() != node.domain().node_id
            || draw.actual_text() != node.domain().speech
            || draw.font_sha256() != font.content_hash()
        {
            return Err(StagingMathPdfError::DisplayMismatch);
        }
    }
    Ok(())
}

fn encode_marked_content(
    draw: &typaxis_display_list::StagingMathDraw,
    font_face_id: FontFaceId,
    page_height: i64,
    max_output_bytes: u64,
) -> Result<Vec<u8>, StagingMathPdfError> {
    let actual_text = utf16be_hex(draw.actual_text(), max_output_bytes)?;
    let mut output = Vec::new();
    extend_bounded(
        &mut output,
        format!("/Span << /ActualText <{actual_text}> >> BDC\n").as_bytes(),
        max_output_bytes,
    )?;
    for paint in draw.paints() {
        match paint {
            MathPaint::Glyph(glyph) => {
                let x = draw
                    .origin_x()
                    .checked_add(glyph.x())
                    .ok_or(StagingMathPdfError::ArithmeticOverflow)?;
                let baseline = draw
                    .baseline_y()
                    .checked_add(glyph.y())
                    .ok_or(StagingMathPdfError::ArithmeticOverflow)?;
                let y = page_height
                    .checked_sub(baseline)
                    .ok_or(StagingMathPdfError::ArithmeticOverflow)?;
                extend_bounded(
                    &mut output,
                    format!(
                        "BT /M{} {} Tf 1 0 0 1 {} {} Tm <{:04X}> Tj ET\n",
                        font_face_id.get(),
                        pdf_number(glyph.font_size_raw()),
                        pdf_number(x),
                        pdf_number(y),
                        glyph.original_gid().get()
                    )
                    .as_bytes(),
                    max_output_bytes,
                )?;
            }
            MathPaint::Rule(rule) => {
                let x = draw
                    .origin_x()
                    .checked_add(rule.x())
                    .ok_or(StagingMathPdfError::ArithmeticOverflow)?;
                let bottom_from_top = draw
                    .baseline_y()
                    .checked_add(rule.y())
                    .and_then(|value| value.checked_add(rule.height()))
                    .ok_or(StagingMathPdfError::ArithmeticOverflow)?;
                let y = page_height
                    .checked_sub(bottom_from_top)
                    .ok_or(StagingMathPdfError::ArithmeticOverflow)?;
                extend_bounded(
                    &mut output,
                    format!(
                        "{} {} {} {} re f\n",
                        pdf_number(x),
                        pdf_number(y),
                        pdf_number(rule.width()),
                        pdf_number(rule.height())
                    )
                    .as_bytes(),
                    max_output_bytes,
                )?;
            }
        }
    }
    extend_bounded(&mut output, b"EMC\n", max_output_bytes)?;
    Ok(output)
}

fn serialize_pdf(
    objects: &BTreeMap<u32, Vec<u8>>,
    object_count: u32,
    max_output_bytes: u64,
) -> Result<Vec<u8>, StagingMathPdfError> {
    if objects.len() != object_count as usize || objects.keys().copied().ne(1..=object_count) {
        return Err(StagingMathPdfError::ReceiptMismatch);
    }
    let mut output = Vec::new();
    extend_bounded(
        &mut output,
        b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n",
        max_output_bytes,
    )?;
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(
            usize::try_from(object_count)
                .map_err(|_| StagingMathPdfError::ObjectLimit)?
                .checked_add(1)
                .ok_or(StagingMathPdfError::ObjectLimit)?,
        )
        .map_err(|_| StagingMathPdfError::AllocationFailure)?;
    offsets.push(0usize);
    for number in 1..=object_count {
        offsets.push(output.len());
        extend_bounded(
            &mut output,
            format!("{number} 0 obj\n").as_bytes(),
            max_output_bytes,
        )?;
        extend_bounded(&mut output, &objects[&number], max_output_bytes)?;
        extend_bounded(&mut output, b"\nendobj\n", max_output_bytes)?;
    }
    let xref = output.len();
    extend_bounded(
        &mut output,
        format!("xref\n0 {}\n", object_count + 1).as_bytes(),
        max_output_bytes,
    )?;
    extend_bounded(&mut output, b"0000000000 65535 f \n", max_output_bytes)?;
    for offset in offsets.into_iter().skip(1) {
        extend_bounded(
            &mut output,
            format!("{offset:010} 00000 n \n").as_bytes(),
            max_output_bytes,
        )?;
    }
    extend_bounded(
        &mut output,
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            object_count + 1
        )
        .as_bytes(),
        max_output_bytes,
    )?;
    Ok(output)
}

fn utf16be_hex(value: &str, max_output_bytes: u64) -> Result<String, StagingMathPdfError> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let units = value.encode_utf16().count();
    let capacity = units
        .checked_add(1)
        .and_then(|value| value.checked_mul(4))
        .ok_or(StagingMathPdfError::OutputLimit)?;
    if u64::try_from(capacity).map_or(true, |value| value > max_output_bytes) {
        return Err(StagingMathPdfError::OutputLimit);
    }
    let mut output = String::from("FEFF");
    output
        .try_reserve_exact(capacity.saturating_sub(4))
        .map_err(|_| StagingMathPdfError::AllocationFailure)?;
    for unit in value.encode_utf16() {
        output.push(char::from(HEX[usize::from((unit >> 12) & 0x0f)]));
        output.push(char::from(HEX[usize::from((unit >> 8) & 0x0f)]));
        output.push(char::from(HEX[usize::from((unit >> 4) & 0x0f)]));
        output.push(char::from(HEX[usize::from(unit & 0x0f)]));
    }
    Ok(output)
}

fn extend_bounded(
    output: &mut Vec<u8>,
    bytes: &[u8],
    maximum: u64,
) -> Result<(), StagingMathPdfError> {
    let next = output
        .len()
        .checked_add(bytes.len())
        .ok_or(StagingMathPdfError::OutputLimit)?;
    if u64::try_from(next).map_or(true, |value| value > maximum) {
        return Err(StagingMathPdfError::OutputLimit);
    }
    output
        .try_reserve_exact(bytes.len())
        .map_err(|_| StagingMathPdfError::AllocationFailure)?;
    output.extend_from_slice(bytes);
    Ok(())
}

fn extract_actual_text(
    bytes: &[u8],
    observations: &[StagingMathPdfObservation],
) -> Result<Vec<String>, StagingMathPdfError> {
    let offsets = parse_xref_offsets(bytes)?;
    let mut by_content: BTreeMap<u32, Vec<&StagingMathPdfObservation>> = BTreeMap::new();
    for observation in observations {
        by_content
            .entry(observation.content_object)
            .or_default()
            .push(observation);
    }
    let mut values = vec![None; observations.len()];
    for (object, expected) in by_content {
        let offset = offsets
            .get(usize::try_from(object).map_err(|_| StagingMathPdfError::ActualTextMismatch)?)
            .copied()
            .ok_or(StagingMathPdfError::ActualTextMismatch)?;
        let stream = object_stream(bytes, offset, object)?;
        let groups = extract_content_groups(stream)?;
        if groups.len() != expected.len() {
            return Err(StagingMathPdfError::ActualTextMismatch);
        }
        for ((actual_text, group), observation) in groups.into_iter().zip(expected) {
            let occurrence = usize::try_from(observation.occurrence)
                .map_err(|_| StagingMathPdfError::ActualTextMismatch)?;
            let slot = values
                .get_mut(occurrence)
                .ok_or(StagingMathPdfError::ActualTextMismatch)?;
            if slot.is_some()
                || sha256(actual_text.as_bytes()) != observation.actual_text_sha256
                || sha256(group) != observation.marked_content_sha256
            {
                return Err(StagingMathPdfError::ActualTextMismatch);
            }
            *slot = Some(actual_text);
        }
    }
    values
        .into_iter()
        .map(|value| value.ok_or(StagingMathPdfError::ActualTextMismatch))
        .collect()
}

fn parse_xref_offsets(bytes: &[u8]) -> Result<Vec<usize>, StagingMathPdfError> {
    let marker = b"startxref\n";
    let start = bytes
        .windows(marker.len())
        .rposition(|window| window == marker)
        .and_then(|value| value.checked_add(marker.len()))
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    let end = bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .and_then(|value| start.checked_add(value))
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    let xref = parse_usize_decimal(&bytes[start..end])?;
    let section = bytes
        .get(xref..)
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    let mut lines = section.split(|byte| *byte == b'\n');
    if lines.next() != Some(b"xref".as_slice()) {
        return Err(StagingMathPdfError::ActualTextMismatch);
    }
    let header = lines
        .next()
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    let mut header_parts = header.split(|byte| *byte == b' ');
    if header_parts.next() != Some(b"0".as_slice()) {
        return Err(StagingMathPdfError::ActualTextMismatch);
    }
    let count = parse_usize_decimal(
        header_parts
            .next()
            .ok_or(StagingMathPdfError::ActualTextMismatch)?,
    )?;
    if count == 0
        || count > bytes.len().saturating_div(19).saturating_add(1)
        || header_parts.next().is_some()
    {
        return Err(StagingMathPdfError::ActualTextMismatch);
    }
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(count)
        .map_err(|_| StagingMathPdfError::AllocationFailure)?;
    for object in 0..count {
        let line = lines
            .next()
            .ok_or(StagingMathPdfError::ActualTextMismatch)?;
        if line.len() != 19
            || (object == 0 && &line[11..] != b"65535 f ")
            || (object != 0 && &line[11..] != b"00000 n ")
        {
            return Err(StagingMathPdfError::ActualTextMismatch);
        }
        offsets.push(parse_usize_decimal(&line[..10])?);
    }
    Ok(offsets)
}

fn object_stream(bytes: &[u8], offset: usize, object: u32) -> Result<&[u8], StagingMathPdfError> {
    let header = format!("{object} 0 obj\n");
    let payload = bytes
        .get(offset..)
        .and_then(|value| value.strip_prefix(header.as_bytes()))
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    let length_prefix = b"<< /Length ";
    let length_start = payload
        .strip_prefix(length_prefix)
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    let length_end = length_start
        .iter()
        .position(|byte| *byte == b' ')
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    let length = parse_usize_decimal(&length_start[..length_end])?;
    let dictionary_end = length_start
        .windows(b">>\nstream\n".len())
        .position(|window| window == b">>\nstream\n")
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    let stream_start = length_prefix
        .len()
        .checked_add(dictionary_end)
        .and_then(|value| value.checked_add(b">>\nstream\n".len()))
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    let stream_end = stream_start
        .checked_add(length)
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    let stream = payload
        .get(stream_start..stream_end)
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    let trailer = payload
        .get(stream_end..)
        .ok_or(StagingMathPdfError::ActualTextMismatch)?;
    if !trailer.starts_with(b"endstream") && !trailer.starts_with(b"\nendstream") {
        return Err(StagingMathPdfError::ActualTextMismatch);
    }
    Ok(stream)
}

fn extract_content_groups(stream: &[u8]) -> Result<Vec<(String, &[u8])>, StagingMathPdfError> {
    let prefix = b"/Span << /ActualText <";
    let suffix = b"EMC\n";
    let mut cursor = 0usize;
    let mut groups = Vec::new();
    while cursor < stream.len() {
        let group_start = cursor;
        cursor = cursor
            .checked_add(prefix.len())
            .filter(|end| stream.get(group_start..*end) == Some(prefix))
            .ok_or(StagingMathPdfError::ActualTextMismatch)?;
        let hex_end = stream[cursor..]
            .windows(b"> >> BDC\n".len())
            .position(|window| window == b"> >> BDC\n")
            .and_then(|value| cursor.checked_add(value))
            .ok_or(StagingMathPdfError::ActualTextMismatch)?;
        let actual_text = decode_utf16be_hex(&stream[cursor..hex_end])?;
        let group_end = stream[hex_end..]
            .windows(suffix.len())
            .position(|window| window == suffix)
            .and_then(|value| hex_end.checked_add(value))
            .and_then(|value| value.checked_add(suffix.len()))
            .ok_or(StagingMathPdfError::ActualTextMismatch)?;
        groups.push((actual_text, &stream[group_start..group_end]));
        cursor = group_end;
    }
    Ok(groups)
}

fn decode_utf16be_hex(hex: &[u8]) -> Result<String, StagingMathPdfError> {
    if !hex.starts_with(b"FEFF") || hex.len() % 4 != 0 {
        return Err(StagingMathPdfError::ActualTextMismatch);
    }
    let mut units = Vec::new();
    units
        .try_reserve_exact((hex.len() - 4) / 4)
        .map_err(|_| StagingMathPdfError::AllocationFailure)?;
    for offset in (4..hex.len()).step_by(4) {
        let value = std::str::from_utf8(&hex[offset..offset + 4])
            .map_err(|_| StagingMathPdfError::ActualTextMismatch)?;
        units.push(
            u16::from_str_radix(value, 16).map_err(|_| StagingMathPdfError::ActualTextMismatch)?,
        );
    }
    String::from_utf16(&units).map_err(|_| StagingMathPdfError::ActualTextMismatch)
}

fn parse_usize_decimal(bytes: &[u8]) -> Result<usize, StagingMathPdfError> {
    if bytes.is_empty() || bytes.iter().any(|byte| !byte.is_ascii_digit()) {
        return Err(StagingMathPdfError::ActualTextMismatch);
    }
    bytes.iter().try_fold(0usize, |value, byte| {
        value
            .checked_mul(10)
            .and_then(|value| value.checked_add(usize::from(byte - b'0')))
            .ok_or(StagingMathPdfError::ActualTextMismatch)
    })
}

fn pdf_number(raw: i64) -> String {
    const SCALE: u64 = 65_536;
    const BINARY_TO_DECIMAL: u64 = 152_587_890_625; // 5^16
    let negative = raw < 0;
    let magnitude = raw.unsigned_abs();
    let whole = magnitude / SCALE;
    let remainder = magnitude % SCALE;
    let mut output = if remainder == 0 {
        whole.to_string()
    } else {
        let mut fraction = format!("{:016}", remainder * BINARY_TO_DECIMAL);
        while fraction.ends_with('0') {
            fraction.pop();
        }
        let mut value = format!("{whole}.{fraction}");
        while value.ends_with('0') {
            value.pop();
        }
        value
    };
    if negative && magnitude != 0 {
        output.insert(0, '-');
    }
    output
}

fn pdf_font_unit(value: i64, units_per_em: u16) -> Result<i64, StagingMathPdfError> {
    let numerator = value
        .checked_mul(1_000)
        .ok_or(StagingMathPdfError::ArithmeticOverflow)?;
    let denominator = i64::from(units_per_em);
    if denominator == 0 {
        return Err(StagingMathPdfError::ArithmeticOverflow);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let twice = remainder
        .unsigned_abs()
        .checked_mul(2)
        .ok_or(StagingMathPdfError::ArithmeticOverflow)?;
    let increment =
        twice > u64::from(units_per_em) || (twice == u64::from(units_per_em) && quotient & 1 != 0);
    if increment {
        quotient
            .checked_add(if numerator >= 0 { 1 } else { -1 })
            .ok_or(StagingMathPdfError::ArithmeticOverflow)
    } else {
        Ok(quotient)
    }
}

fn escape_pdf_name(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::new();
    for byte in value.bytes() {
        if byte == b'#' {
            output.push('#');
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        } else {
            output.push(char::from(byte));
        }
    }
    output
}

fn encode_observation(value: &StagingMathPdfObservation) -> String {
    let mut output = String::from("{\"actual_text_sha256\":");
    push_hash(&mut output, value.actual_text_sha256);
    output.push_str(",\"content_object\":");
    output.push_str(&value.content_object.to_string());
    output.push_str(",\"font_object\":");
    output.push_str(&value.font_object.to_string());
    output.push_str(",\"marked_content_sha256\":");
    push_hash(&mut output, value.marked_content_sha256);
    output.push_str(",\"occurrence\":");
    output.push_str(&value.occurrence.to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&value.page_index.to_string());
    output.push_str(",\"page_object\":");
    output.push_str(&value.page_object.to_string());
    output.push_str(",\"receipt_key\":");
    push_hash(&mut output, value.receipt_key);
    output.push_str(",\"vector_fingerprint\":");
    push_hash(&mut output, value.vector_fingerprint);
    output.push('}');
    output
}

fn encode_pdf(
    display: [u8; 32],
    profile: [u8; 32],
    bytes_sha256: [u8; 32],
    observations: &[StagingMathPdfObservation],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MATH_PDF_ALGORITHM);
    output.push_str(",\"bytes_sha256\":");
    push_hash(&mut output, bytes_sha256);
    output.push_str(",\"display_fingerprint\":");
    push_hash(&mut output, display);
    output.push_str(",\"observations\":[");
    for (index, observation) in observations.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_hash(&mut output, observation.fingerprint);
    }
    output.push_str("],\"profile_fingerprint\":");
    push_hash(&mut output, profile);
    output.push('}');
    output
}

fn push_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_actual_text_is_exact_utf16be_and_visual_glyph_paint_is_present() {
        let fixture = typaxis_display_list::staging_math_display_fixture().unwrap();
        let pdf = write_staging_math_pdf(
            &fixture.layout.package,
            &fixture.layout.profile,
            &fixture.layout.limits,
            &fixture.layout.admitted,
            &fixture.display,
        )
        .unwrap();
        assert_eq!(
            extract_actual_text(pdf.bytes(), pdf.observations()).unwrap(),
            ["x squared".to_owned(), "x plus one".to_owned()]
        );
        assert!(pdf.bytes().windows(3).any(|window| window == b" Tj"));
        assert!(pdf
            .bytes()
            .windows(10)
            .any(|window| window == b"/FontFile2"));
        assert!(pdf
            .bytes()
            .windows(b"/BaseFont /TypaxisSynthetic".len())
            .any(|window| window == b"/BaseFont /TypaxisSynthetic"));
        assert!(!pdf
            .bytes()
            .windows(b"/TypaxisMath0".len())
            .any(|window| window == b"/TypaxisMath0"));
    }

    #[test]
    fn math_actual_text_tamper_is_rejected() {
        let fixture = typaxis_display_list::staging_math_display_fixture().unwrap();
        let mut pdf = write_staging_math_pdf(
            &fixture.layout.package,
            &fixture.layout.profile,
            &fixture.layout.limits,
            &fixture.layout.admitted,
            &fixture.display,
        )
        .unwrap();
        let offset = pdf
            .bytes
            .windows(b"007800200073".len())
            .position(|window| window == b"007800200073")
            .unwrap();
        pdf.bytes[offset] = b'1';
        assert_eq!(
            pdf.verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.display
            ),
            Err(StagingMathPdfError::ActualTextMismatch)
        );
    }

    #[test]
    fn math_pdf_rejects_a_foreign_admission_session_with_identical_font_bytes() {
        let fixture = typaxis_display_list::staging_math_display_fixture().unwrap();
        let foreign = typaxis_display_list::staging_math_display_fixture().unwrap();
        assert_eq!(
            write_staging_math_pdf(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &foreign.layout.admitted,
                &fixture.display,
            ),
            Err(StagingMathPdfError::DisplayMismatch)
        );
    }

    #[test]
    fn math_actual_text_extraction_ignores_embedded_font_bytes() {
        let fixture = typaxis_display_list::staging_math_display_fixture().unwrap();
        let pdf = write_staging_math_pdf(
            &fixture.layout.package,
            &fixture.layout.profile,
            &fixture.layout.limits,
            &fixture.layout.admitted,
            &fixture.display,
        )
        .unwrap();
        let mut bytes = pdf.bytes().to_vec();
        let offsets = parse_xref_offsets(&bytes).unwrap();
        let font_file_object = pdf.observations()[0].font_object() - 3;
        let font_stream = object_stream(
            &bytes,
            offsets[usize::try_from(font_file_object).unwrap()],
            font_file_object,
        )
        .unwrap();
        let stream_offset = font_stream.as_ptr() as usize - bytes.as_ptr() as usize;
        let decoy = b"/ActualText <FEFF0066>";
        bytes[stream_offset..stream_offset + decoy.len()].copy_from_slice(decoy);
        assert_eq!(
            extract_actual_text(&bytes, pdf.observations()).unwrap(),
            ["x squared".to_owned(), "x plus one".to_owned()]
        );
    }

    #[test]
    fn math_actual_text_pdf_output_limit_is_inclusive() {
        let objects = BTreeMap::from([(1, b"<< /Type /Catalog >>".to_vec())]);
        let unbounded = serialize_pdf(&objects, 1, u64::MAX).unwrap();
        let exact = u64::try_from(unbounded.len()).unwrap();
        assert_eq!(serialize_pdf(&objects, 1, exact).unwrap(), unbounded);
        assert_eq!(
            serialize_pdf(&objects, 1, exact - 1),
            Err(StagingMathPdfError::OutputLimit)
        );
    }
}
