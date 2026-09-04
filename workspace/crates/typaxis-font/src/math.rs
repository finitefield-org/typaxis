use std::borrow::Cow;

use typaxis_core::{push_jcs_string, sha256};

use crate::OriginalGlyphId;

pub const MATH_TABLE_FINGERPRINT_ALGORITHM: &str = "typaxis.opentype-math-table/1";

const MATH_CONSTANTS_LENGTH: usize = 214;
const MATH_VALUE_RECORD_COUNT: usize = 51;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MathFontError {
    InvalidContainer,
    InvalidFaceIndex,
    DuplicateTable,
    MissingTable(&'static str),
    MalformedTable(&'static str),
    UnsupportedMathVersion,
    UnsupportedMathDeviceTable,
    UnsupportedMathGlyphKerning,
    MissingGlyph(char),
    MissingVerticalVariant(u16),
    AllocationFailure,
}

impl std::fmt::Display for MathFontError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidContainer => formatter.write_str("R7100: invalid admitted font container"),
            Self::InvalidFaceIndex => {
                formatter.write_str("R7100: invalid admitted font face index")
            }
            Self::DuplicateTable => formatter.write_str("R7100: duplicate admitted font table"),
            Self::MissingTable(tag) => write!(formatter, "R7100: admitted math font lacks {tag}"),
            Self::MalformedTable(tag) => write!(formatter, "R7100: malformed admitted font {tag}"),
            Self::UnsupportedMathVersion => {
                formatter.write_str("R7100: unsupported admitted OpenType MATH version")
            }
            Self::UnsupportedMathDeviceTable => {
                formatter.write_str("R7100: unsupported MATH device/variation table")
            }
            Self::UnsupportedMathGlyphKerning => {
                formatter.write_str("R7100: unsupported MATH glyph kerning table")
            }
            Self::MissingGlyph(value) => {
                write!(
                    formatter,
                    "R7100: admitted math font lacks glyph U+{:04X}",
                    u32::from(*value)
                )
            }
            Self::MissingVerticalVariant(glyph) => {
                write!(
                    formatter,
                    "R7100: admitted math font lacks required vertical variant for glyph {glyph}"
                )
            }
            Self::AllocationFailure => {
                formatter.write_str("R7100: admitted math font allocation failed")
            }
        }
    }
}

impl std::error::Error for MathFontError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MathFontConstants {
    script_percent_scale_down: u16,
    script_script_percent_scale_down: u16,
    display_operator_min_height: u16,
    axis_height: i16,
    subscript_shift_down: i16,
    subscript_top_max: i16,
    subscript_baseline_drop_min: i16,
    superscript_shift_up: i16,
    superscript_shift_up_cramped: i16,
    superscript_bottom_min: i16,
    superscript_baseline_drop_max: i16,
    sub_superscript_gap_min: i16,
    superscript_bottom_max_with_subscript: i16,
    space_after_script: i16,
    fraction_numerator_shift_up: i16,
    fraction_numerator_display_shift_up: i16,
    fraction_denominator_shift_down: i16,
    fraction_denominator_display_shift_down: i16,
    fraction_numerator_gap_min: i16,
    fraction_numerator_display_gap_min: i16,
    fraction_rule_thickness: i16,
    fraction_denominator_gap_min: i16,
    fraction_denominator_display_gap_min: i16,
    radical_vertical_gap: i16,
    radical_display_vertical_gap: i16,
    radical_rule_thickness: i16,
}

impl MathFontConstants {
    pub const fn script_percent_scale_down(self) -> u16 {
        self.script_percent_scale_down
    }
    pub const fn script_script_percent_scale_down(self) -> u16 {
        self.script_script_percent_scale_down
    }
    pub const fn display_operator_min_height(self) -> u16 {
        self.display_operator_min_height
    }
    pub const fn axis_height(self) -> i16 {
        self.axis_height
    }
    pub const fn subscript_shift_down(self) -> i16 {
        self.subscript_shift_down
    }
    pub const fn subscript_top_max(self) -> i16 {
        self.subscript_top_max
    }
    pub const fn subscript_baseline_drop_min(self) -> i16 {
        self.subscript_baseline_drop_min
    }
    pub const fn superscript_shift_up(self) -> i16 {
        self.superscript_shift_up
    }
    pub const fn superscript_shift_up_cramped(self) -> i16 {
        self.superscript_shift_up_cramped
    }
    pub const fn superscript_bottom_min(self) -> i16 {
        self.superscript_bottom_min
    }
    pub const fn superscript_baseline_drop_max(self) -> i16 {
        self.superscript_baseline_drop_max
    }
    pub const fn sub_superscript_gap_min(self) -> i16 {
        self.sub_superscript_gap_min
    }
    pub const fn superscript_bottom_max_with_subscript(self) -> i16 {
        self.superscript_bottom_max_with_subscript
    }
    pub const fn space_after_script(self) -> i16 {
        self.space_after_script
    }
    pub const fn fraction_numerator_shift_up(self) -> i16 {
        self.fraction_numerator_shift_up
    }
    pub const fn fraction_numerator_display_shift_up(self) -> i16 {
        self.fraction_numerator_display_shift_up
    }
    pub const fn fraction_denominator_shift_down(self) -> i16 {
        self.fraction_denominator_shift_down
    }
    pub const fn fraction_denominator_display_shift_down(self) -> i16 {
        self.fraction_denominator_display_shift_down
    }
    pub const fn fraction_numerator_gap_min(self) -> i16 {
        self.fraction_numerator_gap_min
    }
    pub const fn fraction_rule_thickness(self) -> i16 {
        self.fraction_rule_thickness
    }
    pub const fn fraction_numerator_display_gap_min(self) -> i16 {
        self.fraction_numerator_display_gap_min
    }
    pub const fn fraction_denominator_gap_min(self) -> i16 {
        self.fraction_denominator_gap_min
    }
    pub const fn fraction_denominator_display_gap_min(self) -> i16 {
        self.fraction_denominator_display_gap_min
    }
    pub const fn radical_vertical_gap(self) -> i16 {
        self.radical_vertical_gap
    }
    pub const fn radical_display_vertical_gap(self) -> i16 {
        self.radical_display_vertical_gap
    }
    pub const fn radical_rule_thickness(self) -> i16 {
        self.radical_rule_thickness
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MathFontFace<'a> {
    bytes: &'a [u8],
    sfnt_offset: usize,
    face_index: u32,
    units_per_em: u16,
    glyph_count: u16,
    ascent: i16,
    descent: i16,
    bbox_x_min: i16,
    bbox_y_min: i16,
    bbox_x_max: i16,
    bbox_y_max: i16,
    number_of_h_metrics: u16,
    hmtx: &'a [u8],
    glyf: &'a [u8],
    loca: &'a [u8],
    loca_format: i16,
    cmap: &'a [u8],
    name: &'a [u8],
    cmap_format4_offset: Option<usize>,
    cmap_format12_offset: Option<usize>,
    math: &'a [u8],
    glyph_info_offset: usize,
    variants_offset: usize,
    constants: MathFontConstants,
    math_table_fingerprint: [u8; 32],
}

impl<'a> MathFontFace<'a> {
    pub fn parse(bytes: &'a [u8], face_index: u32) -> Result<Self, MathFontError> {
        let sfnt_offset = sfnt_offset(bytes, face_index)?;
        let tables = table_directory(bytes, sfnt_offset)?;
        let table = |tag: [u8; 4], name: &'static str| {
            tables
                .iter()
                .find(|entry| entry.tag == tag)
                .map(|entry| &bytes[entry.offset..entry.offset + entry.length])
                .ok_or(MathFontError::MissingTable(name))
        };
        let head = table(*b"head", "head")?;
        let hhea = table(*b"hhea", "hhea")?;
        let hmtx = table(*b"hmtx", "hmtx")?;
        let glyf = table(*b"glyf", "glyf")?;
        let loca = table(*b"loca", "loca")?;
        let maxp = table(*b"maxp", "maxp")?;
        let cmap = table(*b"cmap", "cmap")?;
        let name = table(*b"name", "name")?;
        let math = table(*b"MATH", "MATH")?;
        if head.len() < 54 || hhea.len() < 36 || maxp.len() < 32 {
            return Err(MathFontError::MalformedTable("metrics"));
        }
        let units_per_em = read_u16(head, 18).ok_or(MathFontError::MalformedTable("head"))?;
        let glyph_count = read_u16(maxp, 4).ok_or(MathFontError::MalformedTable("maxp"))?;
        let ascent = read_i16(hhea, 4).ok_or(MathFontError::MalformedTable("hhea"))?;
        let descent = read_i16(hhea, 6).ok_or(MathFontError::MalformedTable("hhea"))?;
        let bbox_x_min = read_i16(head, 36).ok_or(MathFontError::MalformedTable("head"))?;
        let bbox_y_min = read_i16(head, 38).ok_or(MathFontError::MalformedTable("head"))?;
        let bbox_x_max = read_i16(head, 40).ok_or(MathFontError::MalformedTable("head"))?;
        let bbox_y_max = read_i16(head, 42).ok_or(MathFontError::MalformedTable("head"))?;
        let number_of_h_metrics =
            read_u16(hhea, 34).ok_or(MathFontError::MalformedTable("hhea"))?;
        let loca_format = read_i16(head, 50).ok_or(MathFontError::MalformedTable("head"))?;
        if number_of_h_metrics == 0 || number_of_h_metrics > glyph_count {
            return Err(MathFontError::MalformedTable("metrics"));
        }
        let required_hmtx = usize::from(number_of_h_metrics)
            .checked_mul(4)
            .and_then(|value| {
                value.checked_add(usize::from(glyph_count - number_of_h_metrics).checked_mul(2)?)
            })
            .ok_or(MathFontError::MalformedTable("hmtx"))?;
        if units_per_em == 0
            || glyph_count == 0
            || hmtx.len() < required_hmtx
            || ascent <= 0
            || descent > 0
            || ascent <= descent
            || bbox_x_min >= bbox_x_max
            || bbox_y_min >= bbox_y_max
            || bbox_y_max <= 0
            || read_u32(head, 12) != Some(0x5f0f_3cf5)
            || read_u32(maxp, 0) != Some(0x0001_0000)
            || !matches!(loca_format, 0 | 1)
        {
            return Err(MathFontError::MalformedTable("metrics"));
        }
        validate_glyph_locations(loca, glyf, glyph_count, loca_format)?;
        let (cmap_format4_offset, cmap_format12_offset) = validate_cmap(cmap, glyph_count)?;
        parse_postscript_name(name)?;
        let math_info = validate_math_table(math, glyph_count)?;
        let mut canonical = String::from("{\"algorithm\":");
        push_jcs_string(&mut canonical, MATH_TABLE_FINGERPRINT_ALGORITHM);
        canonical.push_str(",\"face_index\":");
        canonical.push_str(&face_index.to_string());
        canonical.push_str(",\"math_sha256\":\"");
        push_hex_body(&mut canonical, sha256(math));
        canonical.push_str("\",\"units_per_em\":");
        canonical.push_str(&units_per_em.to_string());
        canonical.push('}');
        Ok(Self {
            bytes,
            sfnt_offset,
            face_index,
            units_per_em,
            glyph_count,
            ascent,
            descent,
            bbox_x_min,
            bbox_y_min,
            bbox_x_max,
            bbox_y_max,
            number_of_h_metrics,
            hmtx,
            glyf,
            loca,
            loca_format,
            cmap,
            name,
            cmap_format4_offset,
            cmap_format12_offset,
            math,
            glyph_info_offset: math_info.glyph_info_offset,
            variants_offset: math_info.variants_offset,
            constants: math_info.constants,
            math_table_fingerprint: sha256(canonical.as_bytes()),
        })
    }

    pub const fn face_index(self) -> u32 {
        self.face_index
    }
    pub const fn units_per_em(self) -> u16 {
        self.units_per_em
    }
    pub const fn glyph_count(self) -> u16 {
        self.glyph_count
    }
    pub const fn ascent(self) -> i16 {
        self.ascent
    }
    pub const fn descent(self) -> i16 {
        self.descent
    }
    pub const fn bbox(self) -> (i16, i16, i16, i16) {
        (
            self.bbox_x_min,
            self.bbox_y_min,
            self.bbox_x_max,
            self.bbox_y_max,
        )
    }
    pub const fn constants(self) -> MathFontConstants {
        self.constants
    }
    pub const fn math_table_fingerprint(self) -> [u8; 32] {
        self.math_table_fingerprint
    }
    pub const fn font_bytes(self) -> &'a [u8] {
        self.bytes
    }

    /// Return the face's validated OpenType name-ID 6 for PDF BaseFont and
    /// FontDescriptor naming. The value remains the embedded program's name.
    pub fn postscript_name(self) -> Result<String, MathFontError> {
        parse_postscript_name(self.name)
    }

    /// Return one standalone TrueType program for PDF `FontFile2`. A TTC is
    /// rebuilt without renumbering glyphs; a standalone admitted face is
    /// borrowed byte-for-byte.
    pub fn standalone_truetype_program(self) -> Result<Cow<'a, [u8]>, MathFontError> {
        if self.bytes.get(..4) == Some(b"ttcf") {
            rebuild_standalone_truetype(self.bytes, self.sfnt_offset).map(Cow::Owned)
        } else {
            Ok(Cow::Borrowed(self.bytes))
        }
    }

    pub fn glyph_id(self, value: char) -> Result<OriginalGlyphId, MathFontError> {
        let scalar = u32::from(value);
        let glyph = self
            .cmap_format12_offset
            .and_then(|offset| cmap_format12(self.cmap, offset, scalar))
            .or_else(|| {
                self.cmap_format4_offset
                    .and_then(|offset| cmap_format4(self.cmap, offset, scalar))
            })
            .filter(|glyph| *glyph != 0 && *glyph < self.glyph_count)
            .ok_or(MathFontError::MissingGlyph(value))?;
        self.validate_visible_glyph(glyph)?;
        Ok(OriginalGlyphId::new(glyph))
    }

    pub fn advance_width(self, glyph: OriginalGlyphId) -> Result<u16, MathFontError> {
        if glyph.get() >= self.glyph_count {
            return Err(MathFontError::MalformedTable("hmtx"));
        }
        let metric = glyph.get().min(self.number_of_h_metrics - 1);
        read_u16(self.hmtx, usize::from(metric) * 4)
            .filter(|advance| *advance != 0)
            .ok_or(MathFontError::MalformedTable("hmtx"))
    }

    pub fn glyph_height(self, glyph: OriginalGlyphId) -> Result<u16, MathFontError> {
        self.validate_visible_glyph(glyph.get())?;
        let (start, _) = glyph_range(self.loca, glyph.get(), self.loca_format)?;
        let y_min = read_i16(self.glyf, start + 4).ok_or(MathFontError::MalformedTable("glyf"))?;
        let y_max = read_i16(self.glyf, start + 8).ok_or(MathFontError::MalformedTable("glyf"))?;
        let height = i32::from(y_max)
            .checked_sub(i32::from(y_min))
            .filter(|height| *height > 0)
            .ok_or(MathFontError::MalformedTable("glyf"))?;
        u16::try_from(height).map_err(|_| MathFontError::MalformedTable("glyf"))
    }

    pub fn italic_correction(self, glyph: OriginalGlyphId) -> Result<i16, MathFontError> {
        if glyph.get() >= self.glyph_count {
            return Err(MathFontError::MalformedTable("MATH"));
        }
        let offset = usize::from(
            read_u16(self.math, self.glyph_info_offset)
                .ok_or(MathFontError::MalformedTable("MATH"))?,
        );
        if offset == 0 {
            return Ok(0);
        }
        let info = checked_relative(self.glyph_info_offset, offset, self.math.len())?;
        let coverage_offset =
            usize::from(read_u16(self.math, info).ok_or(MathFontError::MalformedTable("MATH"))?);
        let count = usize::from(
            read_u16(self.math, info + 2).ok_or(MathFontError::MalformedTable("MATH"))?,
        );
        let coverage = checked_relative(info, coverage_offset, self.math.len())?;
        let Some(index) = coverage_index(self.math, coverage, glyph.get())? else {
            return Ok(0);
        };
        if index >= count {
            return Err(MathFontError::MalformedTable("MATH"));
        }
        read_i16(self.math, info + 4 + index * 4).ok_or(MathFontError::MalformedTable("MATH"))
    }

    pub fn vertical_variant(
        self,
        glyph: OriginalGlyphId,
        minimum_advance: u16,
    ) -> Result<(OriginalGlyphId, u16), MathFontError> {
        if glyph.get() >= self.glyph_count || minimum_advance == 0 {
            return Err(MathFontError::MalformedTable("MATH"));
        }
        let coverage_offset = usize::from(
            read_u16(self.math, self.variants_offset + 2)
                .ok_or(MathFontError::MalformedTable("MATH"))?,
        );
        if coverage_offset == 0 {
            return Err(MathFontError::MissingVerticalVariant(glyph.get()));
        }
        let coverage = checked_relative(self.variants_offset, coverage_offset, self.math.len())?;
        let Some(index) = coverage_index(self.math, coverage, glyph.get())? else {
            return Err(MathFontError::MissingVerticalVariant(glyph.get()));
        };
        let vertical_count = usize::from(
            read_u16(self.math, self.variants_offset + 6)
                .ok_or(MathFontError::MalformedTable("MATH"))?,
        );
        if index >= vertical_count {
            return Err(MathFontError::MalformedTable("MATH"));
        }
        let construction_offset = usize::from(
            read_u16(self.math, self.variants_offset + 10 + index * 2)
                .ok_or(MathFontError::MalformedTable("MATH"))?,
        );
        let construction =
            checked_relative(self.variants_offset, construction_offset, self.math.len())?;
        let variant_count = usize::from(
            read_u16(self.math, construction + 2).ok_or(MathFontError::MalformedTable("MATH"))?,
        );
        for variant in 0..variant_count {
            let record = construction + 4 + variant * 4;
            let variant_glyph =
                read_u16(self.math, record).ok_or(MathFontError::MalformedTable("MATH"))?;
            let advance =
                read_u16(self.math, record + 2).ok_or(MathFontError::MalformedTable("MATH"))?;
            if advance >= minimum_advance {
                self.validate_visible_glyph(variant_glyph)?;
                return Ok((OriginalGlyphId::new(variant_glyph), advance));
            }
        }
        Err(MathFontError::MissingVerticalVariant(glyph.get()))
    }

    fn validate_visible_glyph(self, glyph: u16) -> Result<(), MathFontError> {
        #[derive(Clone, Copy)]
        enum Visit {
            Enter(u16),
            Leave(u16),
        }

        let mut states = Vec::new();
        states
            .try_reserve_exact(usize::from(self.glyph_count))
            .map_err(|_| MathFontError::AllocationFailure)?;
        states.resize(usize::from(self.glyph_count), 0u8);
        let mut stack = Vec::new();
        stack
            .try_reserve(1)
            .map_err(|_| MathFontError::AllocationFailure)?;
        stack.push(Visit::Enter(glyph));
        while let Some(visit) = stack.pop() {
            match visit {
                Visit::Leave(glyph) => states[usize::from(glyph)] = 2,
                Visit::Enter(glyph) => match states
                    .get(usize::from(glyph))
                    .copied()
                    .ok_or(MathFontError::MalformedTable("glyf"))?
                {
                    2 => continue,
                    1 => return Err(MathFontError::MalformedTable("glyf")),
                    _ => {
                        states[usize::from(glyph)] = 1;
                        let components = self.validate_glyph_outline(glyph)?;
                        stack
                            .try_reserve(
                                components
                                    .len()
                                    .checked_add(1)
                                    .ok_or(MathFontError::MalformedTable("glyf"))?,
                            )
                            .map_err(|_| MathFontError::AllocationFailure)?;
                        stack.push(Visit::Leave(glyph));
                        stack.extend(components.into_iter().rev().map(Visit::Enter));
                    }
                },
            }
        }
        Ok(())
    }

    fn validate_glyph_outline(self, glyph: u16) -> Result<Vec<u16>, MathFontError> {
        let (start, end) = glyph_range(self.loca, glyph, self.loca_format)?;
        if start >= end || end > self.glyf.len() || end - start < 10 {
            return Err(MathFontError::MalformedTable("glyf"));
        }
        let contours = read_i16(self.glyf, start).ok_or(MathFontError::MalformedTable("glyf"))?;
        let x_min = read_i16(self.glyf, start + 2).ok_or(MathFontError::MalformedTable("glyf"))?;
        let y_min = read_i16(self.glyf, start + 4).ok_or(MathFontError::MalformedTable("glyf"))?;
        let x_max = read_i16(self.glyf, start + 6).ok_or(MathFontError::MalformedTable("glyf"))?;
        let y_max = read_i16(self.glyf, start + 8).ok_or(MathFontError::MalformedTable("glyf"))?;
        if contours == 0 || contours < -1 || x_min >= x_max || y_min >= y_max {
            return Err(MathFontError::MalformedTable("glyf"));
        }
        if contours > 0 {
            validate_simple_glyph(self.glyf, start, end, usize::from(contours as u16))?;
            Ok(Vec::new())
        } else {
            validate_composite_glyph(self.glyf, start, end, self.glyph_count)
        }
    }
}

fn validate_simple_glyph(
    glyf: &[u8],
    start: usize,
    end: usize,
    contour_count: usize,
) -> Result<(), MathFontError> {
    let endpoints = start
        .checked_add(10)
        .ok_or(MathFontError::MalformedTable("glyf"))?;
    let instructions_length_offset = endpoints
        .checked_add(
            contour_count
                .checked_mul(2)
                .ok_or(MathFontError::MalformedTable("glyf"))?,
        )
        .ok_or(MathFontError::MalformedTable("glyf"))?;
    let mut previous_endpoint = None;
    for index in 0..contour_count {
        let endpoint =
            read_u16(glyf, endpoints + index * 2).ok_or(MathFontError::MalformedTable("glyf"))?;
        if previous_endpoint.is_some_and(|previous| endpoint <= previous) {
            return Err(MathFontError::MalformedTable("glyf"));
        }
        previous_endpoint = Some(endpoint);
    }
    let point_count = usize::from(
        previous_endpoint
            .ok_or(MathFontError::MalformedTable("glyf"))?
            .checked_add(1)
            .ok_or(MathFontError::MalformedTable("glyf"))?,
    );
    let instruction_length = usize::from(
        read_u16(glyf, instructions_length_offset).ok_or(MathFontError::MalformedTable("glyf"))?,
    );
    let mut cursor = instructions_length_offset
        .checked_add(2)
        .and_then(|value| value.checked_add(instruction_length))
        .filter(|value| *value <= end)
        .ok_or(MathFontError::MalformedTable("glyf"))?;
    let mut points = 0usize;
    let mut x_bytes = 0usize;
    let mut y_bytes = 0usize;
    while points < point_count {
        let flag = *glyf
            .get(cursor)
            .ok_or(MathFontError::MalformedTable("glyf"))?;
        cursor += 1;
        if flag & 0xc0 != 0 {
            return Err(MathFontError::MalformedTable("glyf"));
        }
        let repeated = if flag & 0x08 != 0 {
            let value = usize::from(
                *glyf
                    .get(cursor)
                    .ok_or(MathFontError::MalformedTable("glyf"))?,
            );
            cursor += 1;
            value
                .checked_add(1)
                .ok_or(MathFontError::MalformedTable("glyf"))?
        } else {
            1
        };
        points = points
            .checked_add(repeated)
            .filter(|value| *value <= point_count)
            .ok_or(MathFontError::MalformedTable("glyf"))?;
        let x_width = if flag & 0x02 != 0 {
            1
        } else if flag & 0x10 != 0 {
            0
        } else {
            2
        };
        let y_width = if flag & 0x04 != 0 {
            1
        } else if flag & 0x20 != 0 {
            0
        } else {
            2
        };
        x_bytes = x_bytes
            .checked_add(
                repeated
                    .checked_mul(x_width)
                    .ok_or(MathFontError::MalformedTable("glyf"))?,
            )
            .ok_or(MathFontError::MalformedTable("glyf"))?;
        y_bytes = y_bytes
            .checked_add(
                repeated
                    .checked_mul(y_width)
                    .ok_or(MathFontError::MalformedTable("glyf"))?,
            )
            .ok_or(MathFontError::MalformedTable("glyf"))?;
    }
    cursor = cursor
        .checked_add(x_bytes)
        .and_then(|value| value.checked_add(y_bytes))
        .filter(|value| *value <= end)
        .ok_or(MathFontError::MalformedTable("glyf"))?;
    if glyf[cursor..end].iter().any(|value| *value != 0) {
        return Err(MathFontError::MalformedTable("glyf"));
    }
    Ok(())
}

fn validate_composite_glyph(
    glyf: &[u8],
    start: usize,
    end: usize,
    glyph_count: u16,
) -> Result<Vec<u16>, MathFontError> {
    const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
    const WE_HAVE_A_SCALE: u16 = 0x0008;
    const MORE_COMPONENTS: u16 = 0x0020;
    const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
    const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;
    const WE_HAVE_INSTRUCTIONS: u16 = 0x0100;
    const ALLOWED_FLAGS: u16 = 0x1fef;

    let mut cursor = start + 10;
    let mut components = Vec::new();
    let final_flags = loop {
        let flags = read_u16(glyf, cursor).ok_or(MathFontError::MalformedTable("glyf"))?;
        let component = read_u16(glyf, cursor + 2)
            .filter(|value| *value < glyph_count)
            .ok_or(MathFontError::MalformedTable("glyf"))?;
        if flags & !ALLOWED_FLAGS != 0
            || [
                flags & WE_HAVE_A_SCALE != 0,
                flags & WE_HAVE_AN_X_AND_Y_SCALE != 0,
                flags & WE_HAVE_A_TWO_BY_TWO != 0,
            ]
            .into_iter()
            .filter(|value| *value)
            .count()
                > 1
            || flags & 0x1800 == 0x1800
        {
            return Err(MathFontError::MalformedTable("glyf"));
        }
        components
            .try_reserve(1)
            .map_err(|_| MathFontError::AllocationFailure)?;
        components.push(component);
        let argument_bytes = if flags & ARG_1_AND_2_ARE_WORDS != 0 {
            4
        } else {
            2
        };
        let transform_bytes = if flags & WE_HAVE_A_SCALE != 0 {
            2
        } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
            4
        } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
            8
        } else {
            0
        };
        cursor = cursor
            .checked_add(4 + argument_bytes + transform_bytes)
            .filter(|value| *value <= end)
            .ok_or(MathFontError::MalformedTable("glyf"))?;
        if flags & MORE_COMPONENTS == 0 {
            break flags;
        }
    };
    if final_flags & WE_HAVE_INSTRUCTIONS != 0 {
        let instruction_length =
            usize::from(read_u16(glyf, cursor).ok_or(MathFontError::MalformedTable("glyf"))?);
        cursor = cursor
            .checked_add(2)
            .and_then(|value| value.checked_add(instruction_length))
            .filter(|value| *value <= end)
            .ok_or(MathFontError::MalformedTable("glyf"))?;
    }
    if glyf[cursor..end].iter().any(|value| *value != 0) {
        return Err(MathFontError::MalformedTable("glyf"));
    }
    Ok(components)
}

#[derive(Clone, Copy)]
struct TableRecord {
    tag: [u8; 4],
    offset: usize,
    length: usize,
}

fn sfnt_offset(bytes: &[u8], face_index: u32) -> Result<usize, MathFontError> {
    if bytes.get(..4) == Some(b"ttcf") {
        if !matches!(read_u32(bytes, 4), Some(0x0001_0000 | 0x0002_0000)) {
            return Err(MathFontError::InvalidContainer);
        }
        let count = read_u32(bytes, 8).ok_or(MathFontError::InvalidContainer)?;
        if face_index >= count {
            return Err(MathFontError::InvalidFaceIndex);
        }
        let offset = read_u32(
            bytes,
            12 + usize::try_from(face_index).map_err(|_| MathFontError::InvalidFaceIndex)? * 4,
        )
        .and_then(|value| usize::try_from(value).ok())
        .filter(|offset| *offset < bytes.len())
        .ok_or(MathFontError::InvalidContainer)?;
        if bytes.get(offset..offset + 4) != Some(b"\0\x01\0\0") {
            return Err(MathFontError::InvalidContainer);
        }
        Ok(offset)
    } else if face_index == 0 && bytes.get(..4) == Some(b"\0\x01\0\0") {
        Ok(0)
    } else if face_index != 0 {
        Err(MathFontError::InvalidFaceIndex)
    } else {
        Err(MathFontError::InvalidContainer)
    }
}

fn table_directory(bytes: &[u8], sfnt_offset: usize) -> Result<Vec<TableRecord>, MathFontError> {
    let count = read_u16(bytes, sfnt_offset + 4).ok_or(MathFontError::InvalidContainer)?;
    let directory_end = sfnt_offset
        .checked_add(12)
        .and_then(|value| value.checked_add(usize::from(count) * 16))
        .filter(|value| *value <= bytes.len())
        .ok_or(MathFontError::InvalidContainer)?;
    let data_floor = if bytes.get(..4) == Some(b"ttcf") {
        ttc_directory_end(bytes)?
    } else {
        directory_end
    };
    let mut result = Vec::new();
    result
        .try_reserve_exact(usize::from(count))
        .map_err(|_| MathFontError::InvalidContainer)?;
    for index in 0..usize::from(count) {
        let record = sfnt_offset + 12 + index * 16;
        let tag: [u8; 4] = bytes[record..record + 4]
            .try_into()
            .map_err(|_| MathFontError::InvalidContainer)?;
        if result.iter().any(|entry: &TableRecord| entry.tag == tag) {
            return Err(MathFontError::DuplicateTable);
        }
        let offset = read_u32(bytes, record + 8)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(MathFontError::InvalidContainer)?;
        let length = read_u32(bytes, record + 12)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(MathFontError::InvalidContainer)?;
        if offset < data_floor
            || offset
                .checked_add(length)
                .map_or(true, |end| end > bytes.len())
        {
            return Err(MathFontError::InvalidContainer);
        }
        result.push(TableRecord {
            tag,
            offset,
            length,
        });
    }
    Ok(result)
}

fn ttc_directory_end(bytes: &[u8]) -> Result<usize, MathFontError> {
    let version = read_u32(bytes, 4).ok_or(MathFontError::InvalidContainer)?;
    let count = read_u32(bytes, 8)
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value != 0)
        .ok_or(MathFontError::InvalidContainer)?;
    let base_header_end = 12usize
        .checked_add(
            count
                .checked_mul(4)
                .ok_or(MathFontError::InvalidContainer)?,
        )
        .ok_or(MathFontError::InvalidContainer)?;
    let header_end = base_header_end
        .checked_add(if version == 0x0002_0000 { 12 } else { 0 })
        .filter(|value| *value <= bytes.len())
        .ok_or(MathFontError::InvalidContainer)?;
    let mut end = header_end;
    for index in 0..count {
        let offset = read_u32(bytes, 12 + index * 4)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(MathFontError::InvalidContainer)?;
        if offset < header_end || bytes.get(offset..offset + 4) != Some(b"\0\x01\0\0") {
            return Err(MathFontError::InvalidContainer);
        }
        let tables =
            usize::from(read_u16(bytes, offset + 4).ok_or(MathFontError::InvalidContainer)?);
        let directory_end = offset
            .checked_add(12)
            .and_then(|value| value.checked_add(tables.checked_mul(16)?))
            .filter(|value| *value <= bytes.len())
            .ok_or(MathFontError::InvalidContainer)?;
        end = end.max(directory_end);
    }
    Ok(end)
}

fn rebuild_standalone_truetype(bytes: &[u8], sfnt_offset: usize) -> Result<Vec<u8>, MathFontError> {
    let mut tables = table_directory(bytes, sfnt_offset)?;
    tables.retain(|table| table.tag != *b"DSIG");
    tables.sort_by_key(|table| table.tag);
    let table_count = u16::try_from(tables.len()).map_err(|_| MathFontError::InvalidContainer)?;
    if table_count == 0 {
        return Err(MathFontError::InvalidContainer);
    }
    let directory_length = 12usize
        .checked_add(
            tables
                .len()
                .checked_mul(16)
                .ok_or(MathFontError::InvalidContainer)?,
        )
        .ok_or(MathFontError::InvalidContainer)?;
    let total_length = tables.iter().try_fold(directory_length, |total, table| {
        total
            .checked_add(
                table
                    .length
                    .checked_add(3)
                    .ok_or(MathFontError::InvalidContainer)?
                    & !3,
            )
            .ok_or(MathFontError::InvalidContainer)
    })?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(total_length)
        .map_err(|_| MathFontError::AllocationFailure)?;
    output.resize(total_length, 0);
    output[..4].copy_from_slice(b"\0\x01\0\0");
    output[4..6].copy_from_slice(&table_count.to_be_bytes());
    let selector = u16::try_from(u16::BITS - 1 - table_count.leading_zeros())
        .map_err(|_| MathFontError::InvalidContainer)?;
    let search_range = 16u16
        .checked_mul(
            1u16.checked_shl(u32::from(selector))
                .ok_or(MathFontError::InvalidContainer)?,
        )
        .ok_or(MathFontError::InvalidContainer)?;
    output[6..8].copy_from_slice(&search_range.to_be_bytes());
    output[8..10].copy_from_slice(&selector.to_be_bytes());
    output[10..12].copy_from_slice(
        &table_count
            .checked_mul(16)
            .and_then(|value| value.checked_sub(search_range))
            .ok_or(MathFontError::InvalidContainer)?
            .to_be_bytes(),
    );
    let mut payload_offset = directory_length;
    let mut head_adjustment = None;
    for (index, table) in tables.iter().enumerate() {
        let record = 12 + index * 16;
        let payload_end = payload_offset
            .checked_add(table.length)
            .ok_or(MathFontError::InvalidContainer)?;
        output[record..record + 4].copy_from_slice(&table.tag);
        output[record + 8..record + 12].copy_from_slice(
            &u32::try_from(payload_offset)
                .map_err(|_| MathFontError::InvalidContainer)?
                .to_be_bytes(),
        );
        output[record + 12..record + 16].copy_from_slice(
            &u32::try_from(table.length)
                .map_err(|_| MathFontError::InvalidContainer)?
                .to_be_bytes(),
        );
        output[payload_offset..payload_end]
            .copy_from_slice(&bytes[table.offset..table.offset + table.length]);
        if table.tag == *b"head" {
            let adjustment = payload_offset
                .checked_add(8)
                .filter(|value| value.checked_add(4).is_some_and(|end| end <= payload_end))
                .ok_or(MathFontError::MalformedTable("head"))?;
            output[adjustment..adjustment + 4].fill(0);
            head_adjustment = Some(adjustment);
        }
        let checksum = sfnt_checksum(&output[payload_offset..payload_end]);
        output[record + 4..record + 8].copy_from_slice(&checksum.to_be_bytes());
        payload_offset = payload_end
            .checked_add(3)
            .ok_or(MathFontError::InvalidContainer)?
            & !3;
    }
    let head_adjustment = head_adjustment.ok_or(MathFontError::MissingTable("head"))?;
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(sfnt_checksum(&output));
    output[head_adjustment..head_adjustment + 4].copy_from_slice(&adjustment.to_be_bytes());
    Ok(output)
}

fn sfnt_checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn parse_postscript_name(table: &[u8]) -> Result<String, MathFontError> {
    let format = read_u16(table, 0).ok_or(MathFontError::MalformedTable("name"))?;
    if !matches!(format, 0 | 1) {
        return Err(MathFontError::MalformedTable("name"));
    }
    let count = usize::from(read_u16(table, 2).ok_or(MathFontError::MalformedTable("name"))?);
    let storage = usize::from(read_u16(table, 4).ok_or(MathFontError::MalformedTable("name"))?);
    let records_end = 6usize
        .checked_add(
            count
                .checked_mul(12)
                .ok_or(MathFontError::MalformedTable("name"))?,
        )
        .filter(|end| *end <= table.len())
        .ok_or(MathFontError::MalformedTable("name"))?;
    let metadata_end = if format == 1 {
        let language_count =
            usize::from(read_u16(table, records_end).ok_or(MathFontError::MalformedTable("name"))?);
        records_end
            .checked_add(2)
            .and_then(|value| value.checked_add(language_count.checked_mul(4)?))
            .filter(|end| *end <= table.len())
            .ok_or(MathFontError::MalformedTable("name"))?
    } else {
        records_end
    };
    if storage < metadata_end || storage > table.len() {
        return Err(MathFontError::MalformedTable("name"));
    }
    if format == 1 {
        let language_count =
            usize::from(read_u16(table, records_end).ok_or(MathFontError::MalformedTable("name"))?);
        for index in 0..language_count {
            let record = records_end + 2 + index * 4;
            let length =
                usize::from(read_u16(table, record).ok_or(MathFontError::MalformedTable("name"))?);
            let offset = usize::from(
                read_u16(table, record + 2).ok_or(MathFontError::MalformedTable("name"))?,
            );
            let start = storage
                .checked_add(offset)
                .ok_or(MathFontError::MalformedTable("name"))?;
            table
                .get(
                    start
                        ..start
                            .checked_add(length)
                            .ok_or(MathFontError::MalformedTable("name"))?,
                )
                .ok_or(MathFontError::MalformedTable("name"))?;
        }
    }

    let mut best: Option<((u8, u8, usize), String)> = None;
    for order in 0..count {
        let record = 6 + order * 12;
        let platform = read_u16(table, record).ok_or(MathFontError::MalformedTable("name"))?;
        let encoding = read_u16(table, record + 2).ok_or(MathFontError::MalformedTable("name"))?;
        let language = read_u16(table, record + 4).ok_or(MathFontError::MalformedTable("name"))?;
        let name_id = read_u16(table, record + 6).ok_or(MathFontError::MalformedTable("name"))?;
        let length =
            usize::from(read_u16(table, record + 8).ok_or(MathFontError::MalformedTable("name"))?);
        let offset =
            usize::from(read_u16(table, record + 10).ok_or(MathFontError::MalformedTable("name"))?);
        let start = storage
            .checked_add(offset)
            .ok_or(MathFontError::MalformedTable("name"))?;
        let value = table
            .get(
                start
                    ..start
                        .checked_add(length)
                        .ok_or(MathFontError::MalformedTable("name"))?,
            )
            .ok_or(MathFontError::MalformedTable("name"))?;
        if name_id != 6 {
            continue;
        }
        let Some(decoded) = decode_postscript_name(platform, value)? else {
            continue;
        };
        if !valid_postscript_name(&decoded) {
            return Err(MathFontError::MalformedTable("name"));
        }
        let rank = (
            name_language_rank(platform, language),
            name_encoding_rank(platform, encoding),
            order,
        );
        if best
            .as_ref()
            .map_or(true, |(established, _)| rank < *established)
        {
            best = Some((rank, decoded));
        }
    }
    best.map(|(_, value)| value)
        .ok_or(MathFontError::MalformedTable("name"))
}

fn decode_postscript_name(platform: u16, bytes: &[u8]) -> Result<Option<String>, MathFontError> {
    let mut output = String::new();
    output
        .try_reserve_exact(bytes.len())
        .map_err(|_| MathFontError::AllocationFailure)?;
    match platform {
        0 | 3 => {
            if bytes.len() % 2 != 0 {
                return Err(MathFontError::MalformedTable("name"));
            }
            for scalar in char::decode_utf16(
                bytes
                    .chunks_exact(2)
                    .map(|pair| u16::from_be_bytes([pair[0], pair[1]])),
            ) {
                output.push(scalar.map_err(|_| MathFontError::MalformedTable("name"))?);
            }
            Ok(Some(output))
        }
        1 | 2 => {
            output.extend(bytes.iter().map(|byte| char::from(*byte)));
            Ok(Some(output))
        }
        _ => Ok(None),
    }
}

fn valid_postscript_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .all(|byte| (33..=126).contains(&byte) && !b"[](){}<>/%".contains(&byte))
}

fn name_language_rank(platform: u16, language: u16) -> u8 {
    match (platform, language) {
        (3, 0x0409) => 0,
        (3, value) if value & 0x03ff == 0x0009 => 1,
        (0, _) => 2,
        (3, _) => 3,
        (1, 0) => 4,
        (1, _) => 5,
        _ => 6,
    }
}

fn name_encoding_rank(platform: u16, encoding: u16) -> u8 {
    match (platform, encoding) {
        (3, 10) => 0,
        (3, 1) => 1,
        (3, 0) | (0, _) | (1, 0) => 2,
        _ => 3,
    }
}

struct MathTableInfo {
    constants: MathFontConstants,
    glyph_info_offset: usize,
    variants_offset: usize,
}

fn validate_math_table(table: &[u8], glyph_count: u16) -> Result<MathTableInfo, MathFontError> {
    if table.len() < 10 {
        return Err(MathFontError::MalformedTable("MATH"));
    }
    if read_u32(table, 0) != Some(0x0001_0000) {
        return Err(MathFontError::UnsupportedMathVersion);
    }
    let constants_offset =
        usize::from(read_u16(table, 4).ok_or(MathFontError::MalformedTable("MATH"))?);
    let glyph_info_offset =
        usize::from(read_u16(table, 6).ok_or(MathFontError::MalformedTable("MATH"))?);
    let variants_offset =
        usize::from(read_u16(table, 8).ok_or(MathFontError::MalformedTable("MATH"))?);
    if constants_offset < 10
        || constants_offset
            .checked_add(MATH_CONSTANTS_LENGTH)
            .map_or(true, |end| end > table.len())
        || glyph_info_offset < 10
        || glyph_info_offset
            .checked_add(8)
            .map_or(true, |end| end > table.len())
        || variants_offset < 10
        || variants_offset
            .checked_add(10)
            .map_or(true, |end| end > table.len())
    {
        return Err(MathFontError::MalformedTable("MATH"));
    }
    let script = read_i16(table, constants_offset).ok_or(MathFontError::MalformedTable("MATH"))?;
    let script_script =
        read_i16(table, constants_offset + 2).ok_or(MathFontError::MalformedTable("MATH"))?;
    let display_operator_min_height =
        read_u16(table, constants_offset + 6).ok_or(MathFontError::MalformedTable("MATH"))?;
    if !(1..=100).contains(&script)
        || !(1..=100).contains(&script_script)
        || display_operator_min_height == 0
    {
        return Err(MathFontError::MalformedTable("MATH"));
    }
    for index in 0..MATH_VALUE_RECORD_COUNT {
        let record = constants_offset + 8 + index * 4;
        if read_u16(table, record + 2) != Some(0) {
            return Err(MathFontError::UnsupportedMathDeviceTable);
        }
    }
    validate_math_glyph_info(table, glyph_info_offset, glyph_count)?;
    validate_math_variants(table, variants_offset, glyph_count)?;
    let value = |index: usize| {
        read_i16(table, constants_offset + 8 + index * 4)
            .ok_or(MathFontError::MalformedTable("MATH"))
    };
    let constants = MathFontConstants {
        script_percent_scale_down: u16::try_from(script)
            .map_err(|_| MathFontError::MalformedTable("MATH"))?,
        script_script_percent_scale_down: u16::try_from(script_script)
            .map_err(|_| MathFontError::MalformedTable("MATH"))?,
        display_operator_min_height,
        axis_height: value(1)?,
        subscript_shift_down: value(4)?,
        subscript_top_max: value(5)?,
        subscript_baseline_drop_min: value(6)?,
        superscript_shift_up: value(7)?,
        superscript_shift_up_cramped: value(8)?,
        superscript_bottom_min: value(9)?,
        superscript_baseline_drop_max: value(10)?,
        sub_superscript_gap_min: value(11)?,
        superscript_bottom_max_with_subscript: value(12)?,
        space_after_script: value(13)?,
        fraction_numerator_shift_up: value(28)?,
        fraction_numerator_display_shift_up: value(29)?,
        fraction_denominator_shift_down: value(30)?,
        fraction_denominator_display_shift_down: value(31)?,
        fraction_numerator_gap_min: value(32)?,
        fraction_numerator_display_gap_min: value(33)?,
        fraction_rule_thickness: value(34)?,
        fraction_denominator_gap_min: value(35)?,
        fraction_denominator_display_gap_min: value(36)?,
        radical_vertical_gap: value(45)?,
        radical_display_vertical_gap: value(46)?,
        radical_rule_thickness: value(47)?,
    };
    if constants.subscript_shift_down < 0
        || constants.subscript_top_max < 0
        || constants.subscript_baseline_drop_min < 0
        || constants.superscript_shift_up < 0
        || constants.superscript_shift_up_cramped < 0
        || constants.superscript_bottom_min < 0
        || constants.superscript_baseline_drop_max < 0
        || constants.sub_superscript_gap_min < 0
        || constants.superscript_bottom_max_with_subscript < 0
        || constants.space_after_script < 0
        || constants.fraction_numerator_shift_up < 0
        || constants.fraction_numerator_display_shift_up < 0
        || constants.fraction_denominator_shift_down < 0
        || constants.fraction_denominator_display_shift_down < 0
        || constants.fraction_numerator_gap_min < 0
        || constants.fraction_numerator_display_gap_min < 0
        || constants.fraction_rule_thickness <= 0
        || constants.fraction_denominator_gap_min < 0
        || constants.fraction_denominator_display_gap_min < 0
        || constants.radical_vertical_gap < 0
        || constants.radical_display_vertical_gap < 0
        || constants.radical_rule_thickness <= 0
    {
        return Err(MathFontError::MalformedTable("MATH"));
    }
    Ok(MathTableInfo {
        constants,
        glyph_info_offset,
        variants_offset,
    })
}

/// CFF admission shares the complete MATH-table parser without requiring a
/// TrueType `glyf`/`loca` face. All glyph references remain bounded by the
/// caller's already cross-checked CFF `maxp.numGlyphs`.
pub(crate) fn validate_cff_math_table(table: &[u8], glyph_count: u16) -> Result<(), MathFontError> {
    validate_math_table(table, glyph_count).map(|_| ())
}

fn validate_math_glyph_info(
    table: &[u8],
    glyph_info: usize,
    glyph_count: u16,
) -> Result<(), MathFontError> {
    let italics =
        usize::from(read_u16(table, glyph_info).ok_or(MathFontError::MalformedTable("MATH"))?);
    if italics != 0 {
        validate_math_value_map(
            table,
            checked_relative(glyph_info, italics, table.len())?,
            glyph_count,
        )?;
    }
    let top_accents =
        usize::from(read_u16(table, glyph_info + 2).ok_or(MathFontError::MalformedTable("MATH"))?);
    if top_accents != 0 {
        validate_math_value_map(
            table,
            checked_relative(glyph_info, top_accents, table.len())?,
            glyph_count,
        )?;
    }
    let extended_shapes =
        usize::from(read_u16(table, glyph_info + 4).ok_or(MathFontError::MalformedTable("MATH"))?);
    if extended_shapes != 0 {
        validate_coverage(
            table,
            checked_relative(glyph_info, extended_shapes, table.len())?,
            glyph_count,
        )?;
    }
    if read_u16(table, glyph_info + 6) != Some(0) {
        return Err(MathFontError::UnsupportedMathGlyphKerning);
    }
    Ok(())
}

fn validate_math_value_map(
    table: &[u8],
    offset: usize,
    glyph_count: u16,
) -> Result<(), MathFontError> {
    let coverage_offset =
        usize::from(read_u16(table, offset).ok_or(MathFontError::MalformedTable("MATH"))?);
    let count =
        usize::from(read_u16(table, offset + 2).ok_or(MathFontError::MalformedTable("MATH"))?);
    if count > usize::from(glyph_count)
        || offset
            .checked_add(4)
            .and_then(|value| value.checked_add(count.checked_mul(4)?))
            .map_or(true, |end| end > table.len())
    {
        return Err(MathFontError::MalformedTable("MATH"));
    }
    let coverage = checked_relative(offset, coverage_offset, table.len())?;
    if validate_coverage(table, coverage, glyph_count)? != count {
        return Err(MathFontError::MalformedTable("MATH"));
    }
    for index in 0..count {
        if read_u16(table, offset + 4 + index * 4 + 2) != Some(0) {
            return Err(MathFontError::UnsupportedMathDeviceTable);
        }
    }
    Ok(())
}

fn validate_math_variants(
    table: &[u8],
    variants: usize,
    glyph_count: u16,
) -> Result<(), MathFontError> {
    let vertical_coverage_offset =
        usize::from(read_u16(table, variants + 2).ok_or(MathFontError::MalformedTable("MATH"))?);
    let horizontal_coverage_offset =
        usize::from(read_u16(table, variants + 4).ok_or(MathFontError::MalformedTable("MATH"))?);
    let vertical_count =
        usize::from(read_u16(table, variants + 6).ok_or(MathFontError::MalformedTable("MATH"))?);
    let horizontal_count =
        usize::from(read_u16(table, variants + 8).ok_or(MathFontError::MalformedTable("MATH"))?);
    let total = vertical_count
        .checked_add(horizontal_count)
        .ok_or(MathFontError::MalformedTable("MATH"))?;
    if total > usize::from(glyph_count)
        || variants
            .checked_add(10)
            .and_then(|value| value.checked_add(total.checked_mul(2)?))
            .map_or(true, |end| end > table.len())
        || (vertical_count == 0) != (vertical_coverage_offset == 0)
        || (horizontal_count == 0) != (horizontal_coverage_offset == 0)
    {
        return Err(MathFontError::MalformedTable("MATH"));
    }
    if vertical_count != 0 {
        let coverage = checked_relative(variants, vertical_coverage_offset, table.len())?;
        if validate_coverage(table, coverage, glyph_count)? != vertical_count {
            return Err(MathFontError::MalformedTable("MATH"));
        }
    }
    if horizontal_count != 0 {
        let coverage = checked_relative(variants, horizontal_coverage_offset, table.len())?;
        if validate_coverage(table, coverage, glyph_count)? != horizontal_count {
            return Err(MathFontError::MalformedTable("MATH"));
        }
    }
    for index in 0..total {
        let construction_offset = usize::from(
            read_u16(table, variants + 10 + index * 2)
                .ok_or(MathFontError::MalformedTable("MATH"))?,
        );
        if construction_offset == 0 {
            return Err(MathFontError::MalformedTable("MATH"));
        }
        validate_glyph_construction(
            table,
            checked_relative(variants, construction_offset, table.len())?,
            glyph_count,
        )?;
    }
    Ok(())
}

fn validate_glyph_construction(
    table: &[u8],
    construction: usize,
    glyph_count: u16,
) -> Result<(), MathFontError> {
    let assembly_offset =
        usize::from(read_u16(table, construction).ok_or(MathFontError::MalformedTable("MATH"))?);
    let count = usize::from(
        read_u16(table, construction + 2).ok_or(MathFontError::MalformedTable("MATH"))?,
    );
    if count > usize::from(glyph_count)
        || (count == 0 && assembly_offset == 0)
        || construction
            .checked_add(4)
            .and_then(|value| value.checked_add(count.checked_mul(4)?))
            .map_or(true, |end| end > table.len())
    {
        return Err(MathFontError::MalformedTable("MATH"));
    }
    let mut previous_advance = 0u16;
    for index in 0..count {
        let record = construction + 4 + index * 4;
        let glyph = read_u16(table, record).ok_or(MathFontError::MalformedTable("MATH"))?;
        let advance = read_u16(table, record + 2).ok_or(MathFontError::MalformedTable("MATH"))?;
        if glyph >= glyph_count || advance == 0 || (index != 0 && advance <= previous_advance) {
            return Err(MathFontError::MalformedTable("MATH"));
        }
        previous_advance = advance;
    }
    if assembly_offset != 0 {
        validate_glyph_assembly(
            table,
            checked_relative(construction, assembly_offset, table.len())?,
            glyph_count,
        )?;
    }
    Ok(())
}

fn validate_glyph_assembly(
    table: &[u8],
    assembly: usize,
    glyph_count: u16,
) -> Result<(), MathFontError> {
    if read_u16(table, assembly + 2) != Some(0) {
        return Err(MathFontError::UnsupportedMathDeviceTable);
    }
    let count =
        usize::from(read_u16(table, assembly + 4).ok_or(MathFontError::MalformedTable("MATH"))?);
    if count == 0
        || count > usize::from(glyph_count)
        || assembly
            .checked_add(6)
            .and_then(|value| value.checked_add(count.checked_mul(10)?))
            .map_or(true, |end| end > table.len())
    {
        return Err(MathFontError::MalformedTable("MATH"));
    }
    for index in 0..count {
        let record = assembly + 6 + index * 10;
        let glyph = read_u16(table, record).ok_or(MathFontError::MalformedTable("MATH"))?;
        let start = read_u16(table, record + 2).ok_or(MathFontError::MalformedTable("MATH"))?;
        let end = read_u16(table, record + 4).ok_or(MathFontError::MalformedTable("MATH"))?;
        let full = read_u16(table, record + 6).ok_or(MathFontError::MalformedTable("MATH"))?;
        let flags = read_u16(table, record + 8).ok_or(MathFontError::MalformedTable("MATH"))?;
        if glyph >= glyph_count || full == 0 || start > full || end > full || flags & !1 != 0 {
            return Err(MathFontError::MalformedTable("MATH"));
        }
    }
    Ok(())
}

fn checked_relative(base: usize, offset: usize, length: usize) -> Result<usize, MathFontError> {
    if offset == 0 {
        return Err(MathFontError::MalformedTable("MATH"));
    }
    base.checked_add(offset)
        .filter(|value| *value < length)
        .ok_or(MathFontError::MalformedTable("MATH"))
}

fn validate_glyph_locations(
    loca: &[u8],
    glyf: &[u8],
    glyph_count: u16,
    format: i16,
) -> Result<(), MathFontError> {
    let entries = usize::from(glyph_count)
        .checked_add(1)
        .ok_or(MathFontError::MalformedTable("loca"))?;
    let width = if format == 0 { 2 } else { 4 };
    if loca.len()
        < entries
            .checked_mul(width)
            .ok_or(MathFontError::MalformedTable("loca"))?
    {
        return Err(MathFontError::MalformedTable("loca"));
    }
    let mut previous = 0usize;
    for glyph in 0..=glyph_count {
        let current = glyph_location(loca, glyph, format)?;
        if current < previous || current > glyf.len() {
            return Err(MathFontError::MalformedTable("loca"));
        }
        previous = current;
    }
    Ok(())
}

fn glyph_range(loca: &[u8], glyph: u16, format: i16) -> Result<(usize, usize), MathFontError> {
    let next = glyph
        .checked_add(1)
        .ok_or(MathFontError::MalformedTable("loca"))?;
    Ok((
        glyph_location(loca, glyph, format)?,
        glyph_location(loca, next, format)?,
    ))
}

fn glyph_location(loca: &[u8], glyph: u16, format: i16) -> Result<usize, MathFontError> {
    match format {
        0 => read_u16(loca, usize::from(glyph) * 2)
            .and_then(|value| usize::from(value).checked_mul(2))
            .ok_or(MathFontError::MalformedTable("loca")),
        1 => read_u32(loca, usize::from(glyph) * 4)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(MathFontError::MalformedTable("loca")),
        _ => Err(MathFontError::MalformedTable("loca")),
    }
}

fn validate_coverage(
    table: &[u8],
    coverage: usize,
    glyph_count: u16,
) -> Result<usize, MathFontError> {
    match read_u16(table, coverage) {
        Some(1) => {
            let count = usize::from(
                read_u16(table, coverage + 2).ok_or(MathFontError::MalformedTable("MATH"))?,
            );
            if count > usize::from(glyph_count)
                || coverage
                    .checked_add(4)
                    .and_then(|value| value.checked_add(count.checked_mul(2)?))
                    .map_or(true, |end| end > table.len())
            {
                return Err(MathFontError::MalformedTable("MATH"));
            }
            let mut previous = None;
            for index in 0..count {
                let glyph = read_u16(table, coverage + 4 + index * 2)
                    .ok_or(MathFontError::MalformedTable("MATH"))?;
                if glyph >= glyph_count || previous.is_some_and(|value| glyph <= value) {
                    return Err(MathFontError::MalformedTable("MATH"));
                }
                previous = Some(glyph);
            }
            Ok(count)
        }
        Some(2) => {
            let ranges = usize::from(
                read_u16(table, coverage + 2).ok_or(MathFontError::MalformedTable("MATH"))?,
            );
            if ranges > usize::from(glyph_count)
                || coverage
                    .checked_add(4)
                    .and_then(|value| value.checked_add(ranges.checked_mul(6)?))
                    .map_or(true, |end| end > table.len())
            {
                return Err(MathFontError::MalformedTable("MATH"));
            }
            let mut total = 0usize;
            let mut previous_end = None;
            for index in 0..ranges {
                let record = coverage + 4 + index * 6;
                let start = read_u16(table, record).ok_or(MathFontError::MalformedTable("MATH"))?;
                let end =
                    read_u16(table, record + 2).ok_or(MathFontError::MalformedTable("MATH"))?;
                let start_index = usize::from(
                    read_u16(table, record + 4).ok_or(MathFontError::MalformedTable("MATH"))?,
                );
                if start > end
                    || end >= glyph_count
                    || previous_end.is_some_and(|value| start <= value)
                    || start_index != total
                {
                    return Err(MathFontError::MalformedTable("MATH"));
                }
                total = total
                    .checked_add(usize::from(end - start) + 1)
                    .filter(|value| *value <= usize::from(glyph_count))
                    .ok_or(MathFontError::MalformedTable("MATH"))?;
                previous_end = Some(end);
            }
            Ok(total)
        }
        _ => Err(MathFontError::MalformedTable("MATH")),
    }
}

fn coverage_index(
    table: &[u8],
    coverage: usize,
    glyph: u16,
) -> Result<Option<usize>, MathFontError> {
    match read_u16(table, coverage) {
        Some(1) => {
            let count = usize::from(
                read_u16(table, coverage + 2).ok_or(MathFontError::MalformedTable("MATH"))?,
            );
            for index in 0..count {
                let candidate = read_u16(table, coverage + 4 + index * 2)
                    .ok_or(MathFontError::MalformedTable("MATH"))?;
                if candidate == glyph {
                    return Ok(Some(index));
                }
                if candidate > glyph {
                    break;
                }
            }
            Ok(None)
        }
        Some(2) => {
            let ranges = usize::from(
                read_u16(table, coverage + 2).ok_or(MathFontError::MalformedTable("MATH"))?,
            );
            for index in 0..ranges {
                let record = coverage + 4 + index * 6;
                let start = read_u16(table, record).ok_or(MathFontError::MalformedTable("MATH"))?;
                let end =
                    read_u16(table, record + 2).ok_or(MathFontError::MalformedTable("MATH"))?;
                if (start..=end).contains(&glyph) {
                    let start_index = usize::from(
                        read_u16(table, record + 4).ok_or(MathFontError::MalformedTable("MATH"))?,
                    );
                    return start_index
                        .checked_add(usize::from(glyph - start))
                        .map(Some)
                        .ok_or(MathFontError::MalformedTable("MATH"));
                }
                if start > glyph {
                    break;
                }
            }
            Ok(None)
        }
        _ => Err(MathFontError::MalformedTable("MATH")),
    }
}

fn validate_cmap(
    cmap: &[u8],
    glyph_count: u16,
) -> Result<(Option<usize>, Option<usize>), MathFontError> {
    let count = read_u16(cmap, 2).ok_or(MathFontError::MalformedTable("cmap"))?;
    let directory_end = 4usize
        .checked_add(usize::from(count) * 8)
        .filter(|value| *value <= cmap.len())
        .ok_or(MathFontError::MalformedTable("cmap"))?;
    if count == 0 {
        return Err(MathFontError::MalformedTable("cmap"));
    }
    let mut format4 = None;
    let mut format12 = None;
    for index in 0..usize::from(count) {
        let record = 4 + index * 8;
        let platform = read_u16(cmap, record).ok_or(MathFontError::MalformedTable("cmap"))?;
        let encoding = read_u16(cmap, record + 2).ok_or(MathFontError::MalformedTable("cmap"))?;
        let offset = read_u32(cmap, record + 4)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value >= directory_end && *value < cmap.len())
            .ok_or(MathFontError::MalformedTable("cmap"))?;
        match read_u16(cmap, offset) {
            Some(4) if platform == 0 || (platform == 3 && encoding == 1) => {
                if format4.is_some_and(|established| established != offset) {
                    return Err(MathFontError::MalformedTable("cmap"));
                }
                format4 = Some(offset);
            }
            Some(12) if platform == 0 || (platform == 3 && encoding == 10) => {
                if format12.is_some_and(|established| established != offset) {
                    return Err(MathFontError::MalformedTable("cmap"));
                }
                format12 = Some(offset);
            }
            Some(_) => {}
            None => return Err(MathFontError::MalformedTable("cmap")),
        }
    }
    if let Some(offset) = format4 {
        validate_cmap_format4(cmap, offset, glyph_count)?;
    }
    if let Some(offset) = format12 {
        validate_cmap_format12(cmap, offset, glyph_count)?;
    }
    if format4.is_none() && format12.is_none() {
        return Err(MathFontError::MalformedTable("cmap"));
    }
    Ok((format4, format12))
}

fn validate_cmap_format12(
    cmap: &[u8],
    offset: usize,
    glyph_count: u16,
) -> Result<(), MathFontError> {
    let length = read_u32(cmap, offset + 4)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(MathFontError::MalformedTable("cmap"))?;
    let groups = read_u32(cmap, offset + 12)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(MathFontError::MalformedTable("cmap"))?;
    if read_u16(cmap, offset + 2) != Some(0)
        || length < 16
        || offset
            .checked_add(length)
            .map_or(true, |end| end > cmap.len())
        || 16usize
            .checked_add(
                groups
                    .checked_mul(12)
                    .ok_or(MathFontError::MalformedTable("cmap"))?,
            )
            .map_or(true, |required| required > length)
    {
        return Err(MathFontError::MalformedTable("cmap"));
    }
    let mut previous_end = None;
    for index in 0..groups {
        let record = offset + 16 + index * 12;
        let start = read_u32(cmap, record).ok_or(MathFontError::MalformedTable("cmap"))?;
        let end = read_u32(cmap, record + 4).ok_or(MathFontError::MalformedTable("cmap"))?;
        let start_glyph =
            read_u32(cmap, record + 8).ok_or(MathFontError::MalformedTable("cmap"))?;
        let final_glyph = start_glyph
            .checked_add(
                end.checked_sub(start)
                    .ok_or(MathFontError::MalformedTable("cmap"))?,
            )
            .ok_or(MathFontError::MalformedTable("cmap"))?;
        if end > 0x10ffff
            || previous_end.is_some_and(|value| start <= value)
            || final_glyph >= u32::from(glyph_count)
        {
            return Err(MathFontError::MalformedTable("cmap"));
        }
        previous_end = Some(end);
    }
    Ok(())
}

fn validate_cmap_format4(
    cmap: &[u8],
    offset: usize,
    glyph_count: u16,
) -> Result<(), MathFontError> {
    let length =
        usize::from(read_u16(cmap, offset + 2).ok_or(MathFontError::MalformedTable("cmap"))?);
    let segment_count_x2 =
        read_u16(cmap, offset + 6).ok_or(MathFontError::MalformedTable("cmap"))?;
    if length < 16
        || segment_count_x2 == 0
        || segment_count_x2 % 2 != 0
        || offset
            .checked_add(length)
            .map_or(true, |end| end > cmap.len())
    {
        return Err(MathFontError::MalformedTable("cmap"));
    }
    let segment_count = usize::from(segment_count_x2 / 2);
    let end_codes = offset + 14;
    let start_codes = end_codes
        .checked_add(segment_count * 2)
        .and_then(|value| value.checked_add(2))
        .ok_or(MathFontError::MalformedTable("cmap"))?;
    let deltas = start_codes
        .checked_add(segment_count * 2)
        .ok_or(MathFontError::MalformedTable("cmap"))?;
    let range_offsets = deltas
        .checked_add(segment_count * 2)
        .ok_or(MathFontError::MalformedTable("cmap"))?;
    let subtable_end = offset + length;
    if range_offsets
        .checked_add(segment_count * 2)
        .map_or(true, |end| end > subtable_end)
    {
        return Err(MathFontError::MalformedTable("cmap"));
    }
    let mut previous_end = None;
    for index in 0..segment_count {
        let end =
            read_u16(cmap, end_codes + index * 2).ok_or(MathFontError::MalformedTable("cmap"))?;
        let start =
            read_u16(cmap, start_codes + index * 2).ok_or(MathFontError::MalformedTable("cmap"))?;
        let delta =
            read_i16(cmap, deltas + index * 2).ok_or(MathFontError::MalformedTable("cmap"))?;
        let range = read_u16(cmap, range_offsets + index * 2)
            .ok_or(MathFontError::MalformedTable("cmap"))?;
        if start > end || previous_end.is_some_and(|value| start <= value) {
            return Err(MathFontError::MalformedTable("cmap"));
        }
        for scalar in start..=end {
            let glyph = if range == 0 {
                scalar.wrapping_add_signed(delta)
            } else {
                let glyph_offset = range_offsets
                    .checked_add(index * 2)
                    .and_then(|value| value.checked_add(usize::from(range)))
                    .and_then(|value| {
                        value.checked_add(usize::from(scalar - start).checked_mul(2)?)
                    })
                    .filter(|value| value.checked_add(2).is_some_and(|end| end <= subtable_end))
                    .ok_or(MathFontError::MalformedTable("cmap"))?;
                let mapped =
                    read_u16(cmap, glyph_offset).ok_or(MathFontError::MalformedTable("cmap"))?;
                if mapped == 0 {
                    0
                } else {
                    mapped.wrapping_add_signed(delta)
                }
            };
            if glyph >= glyph_count && glyph != 0 {
                return Err(MathFontError::MalformedTable("cmap"));
            }
        }
        previous_end = Some(end);
    }
    if read_u16(cmap, end_codes + (segment_count - 1) * 2) != Some(0xffff)
        || read_u16(cmap, start_codes + (segment_count - 1) * 2) != Some(0xffff)
    {
        return Err(MathFontError::MalformedTable("cmap"));
    }
    Ok(())
}

fn cmap_format12(cmap: &[u8], offset: usize, scalar: u32) -> Option<u16> {
    let length = usize::try_from(read_u32(cmap, offset + 4)?).ok()?;
    let groups = usize::try_from(read_u32(cmap, offset + 12)?).ok()?;
    if offset.checked_add(length)? > cmap.len()
        || 16usize.checked_add(groups.checked_mul(12)?)? > length
    {
        return None;
    }
    for index in 0..groups {
        let group = offset + 16 + index * 12;
        let start = read_u32(cmap, group)?;
        let end = read_u32(cmap, group + 4)?;
        if (start..=end).contains(&scalar) {
            let glyph = read_u32(cmap, group + 8)?.checked_add(scalar - start)?;
            return u16::try_from(glyph).ok();
        }
    }
    None
}

fn cmap_format4(cmap: &[u8], offset: usize, scalar: u32) -> Option<u16> {
    let scalar = u16::try_from(scalar).ok()?;
    let length = usize::from(read_u16(cmap, offset + 2)?);
    if offset.checked_add(length)? > cmap.len() {
        return None;
    }
    let segment_count = usize::from(read_u16(cmap, offset + 6)?) / 2;
    if segment_count == 0 {
        return None;
    }
    let end_codes = offset + 14;
    let start_codes = end_codes + segment_count * 2 + 2;
    let deltas = start_codes + segment_count * 2;
    let range_offsets = deltas + segment_count * 2;
    if range_offsets + segment_count * 2 > offset + length {
        return None;
    }
    for index in 0..segment_count {
        let end = read_u16(cmap, end_codes + index * 2)?;
        let start = read_u16(cmap, start_codes + index * 2)?;
        if (start..=end).contains(&scalar) {
            let delta = read_i16(cmap, deltas + index * 2)?;
            let range = read_u16(cmap, range_offsets + index * 2)?;
            if range == 0 {
                return Some(scalar.wrapping_add_signed(delta));
            }
            let glyph_offset = range_offsets
                .checked_add(index * 2)?
                .checked_add(usize::from(range))?
                .checked_add(usize::from(scalar - start) * 2)?;
            let glyph = read_u16(cmap, glyph_offset)?;
            return (glyph != 0).then_some(glyph.wrapping_add_signed(delta));
        }
    }
    None
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_i16(bytes: &[u8], offset: usize) -> Option<i16> {
    Some(i16::from_be_bytes(
        bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn push_hex_body(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_offset(bytes: &[u8], tag: [u8; 4]) -> usize {
        table_directory(bytes, 0)
            .unwrap()
            .into_iter()
            .find(|table| table.tag == tag)
            .unwrap()
            .offset
    }

    fn single_face_ttc(ttf: &[u8]) -> Vec<u8> {
        let mut face = ttf.to_vec();
        let table_count = usize::from(read_u16(&face, 4).unwrap());
        for index in 0..table_count {
            let field = 12 + index * 16 + 8;
            let offset = read_u32(&face, field).unwrap() + 16;
            face[field..field + 4].copy_from_slice(&offset.to_be_bytes());
        }
        let mut output = b"ttcf\0\x01\0\0\0\0\0\x01\0\0\0\x10".to_vec();
        output.extend_from_slice(&face);
        output
    }

    #[test]
    fn math_missing_table_is_rejected_without_fallback() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/profiles/basic-document-1/combined/job/body.ttf"
        ));
        assert_eq!(
            MathFontFace::parse(bytes, 0).unwrap_err(),
            MathFontError::MissingTable("MATH")
        );
    }

    #[test]
    fn math_table_metrics_and_required_glyphs_are_validated() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/math/job/math.ttf"
        ));
        let face = MathFontFace::parse(bytes, 0).unwrap();
        assert_eq!(face.units_per_em(), 1_000);
        assert_eq!(face.face_index(), 0);
        assert_eq!(face.postscript_name().unwrap(), "TypaxisSynthetic");
        assert!(face.constants().axis_height() > 0);
        let x = face.glyph_id('x').unwrap();
        assert!(face.advance_width(x).unwrap() > 0);
        assert_eq!(face.glyph_height(x).unwrap(), 700);
        assert_eq!(face.italic_correction(x).unwrap(), 20);
        let sum = face.glyph_id('∑').unwrap();
        assert_eq!(sum.get(), 1);
        let (variant, advance) = face.vertical_variant(sum, 1_300).unwrap();
        assert_eq!((variant.get(), advance), (2, 1_400));
        assert_eq!(
            face.vertical_variant(sum, 1_401),
            Err(MathFontError::MissingVerticalVariant(1))
        );

        let collection = single_face_ttc(bytes);
        let collection_face = MathFontFace::parse(&collection, 0).unwrap();
        let standalone = collection_face.standalone_truetype_program().unwrap();
        assert_eq!(standalone.get(..4), Some(b"\0\x01\0\0".as_slice()));
        let rebuilt = MathFontFace::parse(&standalone, 0).unwrap();
        assert_eq!(
            rebuilt.math_table_fingerprint(),
            collection_face.math_table_fingerprint()
        );
        assert_eq!(rebuilt.glyph_id('∑').unwrap().get(), 1);
        assert_eq!(rebuilt.postscript_name().unwrap(), "TypaxisSynthetic");
    }

    #[test]
    fn math_cmap_requires_unicode_encoding_records() {
        let mut bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/math/job/math.ttf"
        ))
        .to_vec();
        let cmap = table_offset(&bytes, *b"cmap");
        bytes[cmap + 6..cmap + 8].copy_from_slice(&0u16.to_be_bytes());
        bytes[cmap + 14..cmap + 16].copy_from_slice(&0u16.to_be_bytes());
        assert_eq!(
            MathFontFace::parse(&bytes, 0).unwrap_err(),
            MathFontError::MalformedTable("cmap")
        );
    }

    #[test]
    fn required_math_glyph_rejects_truncated_outline_program() {
        let mut bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/math/job/math.ttf"
        ))
        .to_vec();
        let loca = table_offset(&bytes, *b"loca");
        let glyph_start = read_u32(&bytes, loca + 89 * 4).unwrap();
        bytes[loca + 90 * 4..loca + 91 * 4].copy_from_slice(&(glyph_start + 12).to_be_bytes());
        let face = MathFontFace::parse(&bytes, 0).unwrap();
        assert_eq!(
            face.glyph_id('x').unwrap_err(),
            MathFontError::MalformedTable("glyf")
        );
    }
}
