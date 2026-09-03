//! Unicode 16.0 default line-break opportunities (UAX #14 revision 53).

use core::fmt;

include!("unicode_linebreak_data.rs");

const CLASS_MASK: u16 = 0x003f;
const INITIAL_PUNCTUATION: u16 = 1 << 6;
const FINAL_PUNCTUATION: u16 = 1 << 7;
const EAST_ASIAN: u16 = 1 << 8;
const MARK: u16 = 1 << 9;
const UNASSIGNED_EXTENDED_PICTOGRAPHIC: u16 = 1 << 10;

/// The exact Unicode data version compiled into the default line breaker.
pub const UNICODE_VERSION: &str = UNICODE_LINE_BREAK_VERSION;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum Class {
    Ai,
    Ak,
    Al,
    Ap,
    As,
    B2,
    Ba,
    Bb,
    Bk,
    Cb,
    Cj,
    Cl,
    Cm,
    Cp,
    Cr,
    Eb,
    Em,
    Ex,
    Gl,
    H2,
    H3,
    Hl,
    Hy,
    Id,
    In,
    Is,
    Jl,
    Jt,
    Jv,
    Lf,
    Nl,
    Ns,
    Nu,
    Op,
    Po,
    Pr,
    Qu,
    Ri,
    Sa,
    Sg,
    Sp,
    Sy,
    Vf,
    Vi,
    Wj,
    Xx,
    Zw,
    Zwj,
}

impl Class {
    fn from_property(property: u16) -> Self {
        match property & CLASS_MASK {
            0 => Self::Ai,
            1 => Self::Ak,
            2 => Self::Al,
            3 => Self::Ap,
            4 => Self::As,
            5 => Self::B2,
            6 => Self::Ba,
            7 => Self::Bb,
            8 => Self::Bk,
            9 => Self::Cb,
            10 => Self::Cj,
            11 => Self::Cl,
            12 => Self::Cm,
            13 => Self::Cp,
            14 => Self::Cr,
            15 => Self::Eb,
            16 => Self::Em,
            17 => Self::Ex,
            18 => Self::Gl,
            19 => Self::H2,
            20 => Self::H3,
            21 => Self::Hl,
            22 => Self::Hy,
            23 => Self::Id,
            24 => Self::In,
            25 => Self::Is,
            26 => Self::Jl,
            27 => Self::Jt,
            28 => Self::Jv,
            29 => Self::Lf,
            30 => Self::Nl,
            31 => Self::Ns,
            32 => Self::Nu,
            33 => Self::Op,
            34 => Self::Po,
            35 => Self::Pr,
            36 => Self::Qu,
            37 => Self::Ri,
            38 => Self::Sa,
            39 => Self::Sg,
            40 => Self::Sp,
            41 => Self::Sy,
            42 => Self::Vf,
            43 => Self::Vi,
            44 => Self::Wj,
            45 => Self::Xx,
            46 => Self::Zw,
            47 => Self::Zwj,
            _ => unreachable!("generated line-break class is out of range"),
        }
    }
}

/// Classification of one Unicode line-break boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnicodeBreakKind {
    Allowed,
    Mandatory,
}

/// One legal default UAX #14 boundary, expressed as a UTF-8 byte offset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnicodeBreak {
    byte_offset: usize,
    kind: UnicodeBreakKind,
}

/// One logical unit supplied to the Unicode line-break classifier. A
/// producer-composed inline vector is represented directly as `SyntheticAl`;
/// no object-replacement scalar is inserted into source text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnicodeLineBreakUnit {
    Scalar(char),
    SyntheticAl,
}

/// A legal boundary in a typed logical-unit sequence. `unit_offset` is the
/// exclusive logical-unit index and therefore remains stable even when the
/// sequence contains synthetic units with no UTF-8 byte representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnicodeUnitBreak {
    unit_offset: usize,
    kind: UnicodeBreakKind,
}

impl UnicodeUnitBreak {
    pub const fn unit_offset(self) -> usize {
        self.unit_offset
    }

    pub const fn kind(self) -> UnicodeBreakKind {
        self.kind
    }
}

impl UnicodeBreak {
    pub const fn byte_offset(self) -> usize {
        self.byte_offset
    }

    pub const fn kind(self) -> UnicodeBreakKind {
        self.kind
    }
}

/// A bounded allocation failed while deriving Unicode line breaks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnicodeLineBreakError;

impl fmt::Display for UnicodeLineBreakError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("unable to allocate Unicode line-break state")
    }
}

impl std::error::Error for UnicodeLineBreakError {}

#[derive(Clone, Copy, Debug)]
struct Unit {
    codepoint: u32,
    byte_offset: usize,
    source_property: u16,
    property: u16,
    original: Class,
    class: Class,
    ignored: bool,
    ignored_tail_has_zwj: bool,
}

impl Unit {
    fn is_initial_punctuation(self) -> bool {
        self.property & INITIAL_PUNCTUATION != 0
    }

    fn is_final_punctuation(self) -> bool {
        self.property & FINAL_PUNCTUATION != 0
    }

    fn is_east_asian(self) -> bool {
        self.property & EAST_ASIAN != 0
    }

    fn is_unassigned_extended_pictographic(self) -> bool {
        self.property & UNASSIGNED_EXTENDED_PICTOGRAPHIC != 0
    }
}

#[derive(Debug)]
struct Context {
    previous_significant: Vec<Option<usize>>,
    next_significant: Vec<Option<usize>>,
    previous_non_space: Vec<Option<usize>>,
    numeric_chain: Vec<bool>,
    regional_indicator_odd: Vec<bool>,
}

/// Returns all allowed and mandatory default UAX #14 boundaries.
///
/// Complex-context (`SA`) letters use the standard default `AL` resolution;
/// language-specific dictionary segmentation is intentionally a separate
/// tailoring layer. The returned collection always contains the mandatory end
/// boundary, including for empty input.
pub fn unicode_line_breaks(text: &str) -> Result<Vec<UnicodeBreak>, UnicodeLineBreakError> {
    let scalar_count = text.chars().count();
    let mut units = Vec::new();
    units
        .try_reserve_exact(scalar_count)
        .map_err(|_| UnicodeLineBreakError)?;
    for (byte_offset, scalar) in text.char_indices() {
        units.push(scalar_unit(scalar, byte_offset));
    }

    apply_lb9(&mut units);
    let context = Context::new(&units)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(units.len().checked_add(1).ok_or(UnicodeLineBreakError)?)
        .map_err(|_| UnicodeLineBreakError)?;

    if units.is_empty() {
        output.push(UnicodeBreak {
            byte_offset: 0,
            kind: UnicodeBreakKind::Mandatory,
        });
        return Ok(output);
    }

    for boundary in 1..units.len() {
        if let Some(kind) = boundary_kind(&units, &context, boundary) {
            output.push(UnicodeBreak {
                byte_offset: units[boundary].byte_offset,
                kind,
            });
        }
    }
    output.push(UnicodeBreak {
        byte_offset: text.len(),
        kind: UnicodeBreakKind::Mandatory,
    });
    Ok(output)
}

/// Returns UAX #14 boundaries for a complete logical-unit sequence which may
/// contain producer-composed atomic vectors. A synthetic vector has exact
/// class `AL` and occupies one logical unit. The caller retains its source
/// provenance alongside the unit index.
pub fn unicode_line_breaks_for_units(
    input: &[UnicodeLineBreakUnit],
) -> Result<Vec<UnicodeUnitBreak>, UnicodeLineBreakError> {
    let mut units = Vec::new();
    units
        .try_reserve_exact(input.len())
        .map_err(|_| UnicodeLineBreakError)?;
    for (unit_offset, input_unit) in input.iter().copied().enumerate() {
        units.push(match input_unit {
            UnicodeLineBreakUnit::Scalar(scalar) => scalar_unit(scalar, unit_offset),
            UnicodeLineBreakUnit::SyntheticAl => Unit {
                codepoint: 0,
                byte_offset: unit_offset,
                source_property: Class::Al as u16,
                property: Class::Al as u16,
                original: Class::Al,
                class: Class::Al,
                ignored: false,
                ignored_tail_has_zwj: false,
            },
        });
    }

    apply_lb9(&mut units);
    let context = Context::new(&units)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(units.len().checked_add(1).ok_or(UnicodeLineBreakError)?)
        .map_err(|_| UnicodeLineBreakError)?;
    if units.is_empty() {
        output.push(UnicodeUnitBreak {
            unit_offset: 0,
            kind: UnicodeBreakKind::Mandatory,
        });
        return Ok(output);
    }
    for boundary in 1..units.len() {
        if let Some(kind) = boundary_kind(&units, &context, boundary) {
            output.push(UnicodeUnitBreak {
                unit_offset: boundary,
                kind,
            });
        }
    }
    output.push(UnicodeUnitBreak {
        unit_offset: input.len(),
        kind: UnicodeBreakKind::Mandatory,
    });
    Ok(output)
}

fn scalar_unit(scalar: char, byte_offset: usize) -> Unit {
    let codepoint = scalar as u32;
    let source_property = property(codepoint);
    let original = Class::from_property(source_property);
    let mut class = match original {
        Class::Ai | Class::Sg | Class::Xx => Class::Al,
        Class::Cj => Class::Ns,
        Class::Sa if source_property & MARK != 0 => Class::Cm,
        Class::Sa => Class::Al,
        other => other,
    };
    // LB10 gives remaining CM/ZWJ all properties of U+0041.
    let mut effective_property = source_property;
    if matches!(class, Class::Cm | Class::Zwj) {
        class = Class::Al;
        effective_property = Class::Al as u16;
    }
    Unit {
        codepoint,
        byte_offset,
        source_property,
        property: effective_property,
        original,
        class,
        ignored: false,
        ignored_tail_has_zwj: false,
    }
}

fn property(codepoint: u32) -> u16 {
    let index = UNICODE_LINE_BREAK_RANGES.partition_point(|(_, end, _)| *end < codepoint);
    UNICODE_LINE_BREAK_RANGES
        .get(index)
        .filter(|(start, _, _)| *start <= codepoint)
        .map_or(Class::Xx as u16, |(_, _, value)| *value)
}

fn apply_lb9(units: &mut [Unit]) {
    let mut base: Option<(Class, u16)> = None;
    let mut tail_has_zwj = false;
    for unit in units {
        let resolved_before_lb10 = match unit.original {
            Class::Sa if unit.source_property & MARK != 0 => Class::Cm,
            other => other,
        };
        let absorbable = matches!(resolved_before_lb10, Class::Cm | Class::Zwj);
        if absorbable {
            if let Some((base_class, base_property)) = base {
                unit.class = base_class;
                unit.property = base_property;
                unit.ignored = true;
                tail_has_zwj |= unit.original == Class::Zwj;
                unit.ignored_tail_has_zwj = tail_has_zwj;
                continue;
            }
        }
        unit.ignored_tail_has_zwj = false;
        tail_has_zwj = false;
        base = if matches!(
            unit.class,
            Class::Bk | Class::Cr | Class::Lf | Class::Nl | Class::Sp | Class::Zw
        ) {
            None
        } else {
            Some((unit.class, unit.property))
        };
    }
}

impl Context {
    fn new(units: &[Unit]) -> Result<Self, UnicodeLineBreakError> {
        let count = units.len().checked_add(1).ok_or(UnicodeLineBreakError)?;
        let mut previous_significant = Vec::new();
        let mut previous_non_space = Vec::new();
        let mut numeric_chain = Vec::new();
        let mut regional_indicator_odd = Vec::new();
        for vector_capacity in [&mut previous_significant, &mut previous_non_space] {
            vector_capacity
                .try_reserve_exact(count)
                .map_err(|_| UnicodeLineBreakError)?;
        }
        numeric_chain
            .try_reserve_exact(count)
            .map_err(|_| UnicodeLineBreakError)?;
        regional_indicator_odd
            .try_reserve_exact(count)
            .map_err(|_| UnicodeLineBreakError)?;

        let mut previous = None;
        let mut non_space = None;
        let mut in_numeric_chain = false;
        let mut ri_odd = false;
        for (index, unit) in units.iter().enumerate() {
            previous_significant.push(previous);
            previous_non_space.push(non_space);
            numeric_chain.push(in_numeric_chain);
            regional_indicator_odd.push(ri_odd);
            if unit.ignored {
                continue;
            }
            previous = Some(index);
            if unit.class != Class::Sp {
                non_space = Some(index);
            }
            in_numeric_chain = unit.class == Class::Nu
                || (matches!(unit.class, Class::Sy | Class::Is) && in_numeric_chain);
            ri_odd = if unit.class == Class::Ri {
                !ri_odd
            } else {
                false
            };
        }
        previous_significant.push(previous);
        previous_non_space.push(non_space);
        numeric_chain.push(in_numeric_chain);
        regional_indicator_odd.push(ri_odd);

        let mut next_significant = Vec::new();
        next_significant
            .try_reserve_exact(count)
            .map_err(|_| UnicodeLineBreakError)?;
        next_significant.resize(count, None);
        let mut next = None;
        for index in (0..units.len()).rev() {
            if !units[index].ignored {
                next = Some(index);
            }
            next_significant[index] = next;
        }
        Ok(Self {
            previous_significant,
            next_significant,
            previous_non_space,
            numeric_chain,
            regional_indicator_odd,
        })
    }

    fn previous(&self, boundary: usize) -> Option<usize> {
        self.previous_significant[boundary]
    }

    fn next_after(&self, index: usize) -> Option<usize> {
        self.next_significant[index + 1]
    }
}

#[allow(clippy::too_many_lines)]
fn boundary_kind(units: &[Unit], context: &Context, boundary: usize) -> Option<UnicodeBreakKind> {
    let physical_left = units[boundary - 1];
    let right = units[boundary];
    let left_index = context.previous(boundary)?;
    let left = units[left_index];

    // LB4-LB6: mandatory breaks and their exclusions.
    if left.class == Class::Cr && right.class == Class::Lf {
        return None;
    }
    if matches!(left.class, Class::Bk | Class::Cr | Class::Lf | Class::Nl) {
        return Some(UnicodeBreakKind::Mandatory);
    }
    if matches!(right.class, Class::Bk | Class::Cr | Class::Lf | Class::Nl) {
        return None;
    }
    // LB7-LB8a.
    if matches!(right.class, Class::Sp | Class::Zw) {
        return None;
    }
    if context.previous_non_space[boundary].is_some_and(|index| units[index].class == Class::Zw) {
        return Some(UnicodeBreakKind::Allowed);
    }
    if physical_left.original == Class::Zwj || physical_left.ignored_tail_has_zwj {
        return None;
    }
    // LB9. Ignored CM/ZWJ never begin a line.
    if right.ignored {
        return None;
    }

    // LB11-LB13.
    if right.class == Class::Wj || left.class == Class::Wj || left.class == Class::Gl {
        return None;
    }
    if right.class == Class::Gl && !matches!(left.class, Class::Sp | Class::Ba | Class::Hy) {
        return None;
    }
    if matches!(
        right.class,
        Class::Cl | Class::Cp | Class::Ex | Class::Is | Class::Sy
    ) {
        // LB15c takes precedence over LB15d/LB13 for SP ÷ IS NU.
        if left.class == Class::Sp
            && right.class == Class::Is
            && context
                .next_after(boundary)
                .is_some_and(|index| units[index].class == Class::Nu)
        {
            return Some(UnicodeBreakKind::Allowed);
        }
        return None;
    }

    // LB14-LB17: rules spanning an optional run of spaces.
    let non_space_left = context.previous_non_space[boundary].map(|index| units[index]);
    if non_space_left.is_some_and(|unit| unit.class == Class::Op) {
        return None;
    }
    if let Some(quote_index) = context.previous_non_space[boundary] {
        let quote = units[quote_index];
        if quote.class == Class::Qu && quote.is_initial_punctuation() {
            let before_quote = context.previous(quote_index);
            if before_quote.map_or(true, |index| {
                matches!(
                    units[index].class,
                    Class::Bk
                        | Class::Cr
                        | Class::Lf
                        | Class::Nl
                        | Class::Op
                        | Class::Qu
                        | Class::Gl
                        | Class::Sp
                        | Class::Zw
                )
            }) {
                return None;
            }
        }
    }
    if right.class == Class::Qu && right.is_final_punctuation() {
        let after_quote = context.next_after(boundary);
        if after_quote.map_or(true, |index| {
            matches!(
                units[index].class,
                Class::Sp
                    | Class::Gl
                    | Class::Wj
                    | Class::Cl
                    | Class::Qu
                    | Class::Cp
                    | Class::Ex
                    | Class::Is
                    | Class::Sy
                    | Class::Bk
                    | Class::Cr
                    | Class::Lf
                    | Class::Nl
                    | Class::Zw
            )
        }) {
            return None;
        }
    }
    if right.class == Class::Ns
        && non_space_left.is_some_and(|unit| matches!(unit.class, Class::Cl | Class::Cp))
    {
        return None;
    }
    if right.class == Class::B2 && non_space_left.is_some_and(|unit| unit.class == Class::B2) {
        return None;
    }

    // LB18.
    if left.class == Class::Sp {
        return Some(UnicodeBreakKind::Allowed);
    }

    // LB19 and LB19a.
    if right.class == Class::Qu && !right.is_initial_punctuation() {
        return None;
    }
    if left.class == Class::Qu && !left.is_final_punctuation() {
        return None;
    }
    if right.class == Class::Qu {
        let following = context.next_after(boundary);
        if !left.is_east_asian() || following.map_or(true, |index| !units[index].is_east_asian()) {
            return None;
        }
    }
    if left.class == Class::Qu {
        let before_quote = context.previous(left_index);
        if !right.is_east_asian()
            || before_quote.map_or(true, |index| !units[index].is_east_asian())
        {
            return None;
        }
    }

    // LB20-LB22.
    if left.class == Class::Cb || right.class == Class::Cb {
        return Some(UnicodeBreakKind::Allowed);
    }
    if right.class == Class::Al
        && (left.class == Class::Hy || left.codepoint == 0x2010)
        && context.previous(left_index).map_or(true, |index| {
            matches!(
                units[index].class,
                Class::Bk
                    | Class::Cr
                    | Class::Lf
                    | Class::Nl
                    | Class::Sp
                    | Class::Zw
                    | Class::Cb
                    | Class::Gl
            )
        })
    {
        return None;
    }
    if matches!(right.class, Class::Ba | Class::Hy | Class::Ns) || left.class == Class::Bb {
        return None;
    }
    if right.class != Class::Hl
        && (left.class == Class::Hy || (left.class == Class::Ba && !left.is_east_asian()))
        && context
            .previous(left_index)
            .is_some_and(|index| units[index].class == Class::Hl)
    {
        return None;
    }
    if left.class == Class::Sy && right.class == Class::Hl {
        return None;
    }
    if right.class == Class::In {
        return None;
    }

    // LB23-LB24.
    if (matches!(left.class, Class::Al | Class::Hl) && right.class == Class::Nu)
        || (left.class == Class::Nu && matches!(right.class, Class::Al | Class::Hl))
        || (left.class == Class::Pr && matches!(right.class, Class::Id | Class::Eb | Class::Em))
        || (matches!(left.class, Class::Id | Class::Eb | Class::Em) && right.class == Class::Po)
        || (matches!(left.class, Class::Pr | Class::Po)
            && matches!(right.class, Class::Al | Class::Hl))
        || (matches!(left.class, Class::Al | Class::Hl)
            && matches!(right.class, Class::Pr | Class::Po))
    {
        return None;
    }

    // LB25.
    if matches!(right.class, Class::Po | Class::Pr)
        && (context.numeric_chain[boundary]
            || (matches!(left.class, Class::Cl | Class::Cp) && context.numeric_chain[left_index]))
    {
        return None;
    }
    if matches!(left.class, Class::Po | Class::Pr) && right.class == Class::Op {
        let after_open = context.next_after(boundary);
        if after_open.is_some_and(|index| {
            units[index].class == Class::Nu
                || (units[index].class == Class::Is
                    && context
                        .next_after(index)
                        .is_some_and(|next| units[next].class == Class::Nu))
        }) {
            return None;
        }
    }
    if (context.numeric_chain[boundary]
        || matches!(left.class, Class::Po | Class::Pr | Class::Hy | Class::Is))
        && right.class == Class::Nu
    {
        return None;
    }

    // LB26-LB27.
    if (left.class == Class::Jl
        && matches!(right.class, Class::Jl | Class::Jv | Class::H2 | Class::H3))
        || (matches!(left.class, Class::Jv | Class::H2)
            && matches!(right.class, Class::Jv | Class::Jt))
        || (matches!(left.class, Class::Jt | Class::H3) && right.class == Class::Jt)
        || (matches!(
            left.class,
            Class::Jl | Class::Jv | Class::Jt | Class::H2 | Class::H3
        ) && right.class == Class::Po)
        || (left.class == Class::Pr
            && matches!(
                right.class,
                Class::Jl | Class::Jv | Class::Jt | Class::H2 | Class::H3
            ))
    {
        return None;
    }

    // LB28-LB28a.
    if matches!(left.class, Class::Al | Class::Hl) && matches!(right.class, Class::Al | Class::Hl) {
        return None;
    }
    let is_aksara_base =
        |unit: Unit| matches!(unit.class, Class::Ak | Class::As) || unit.codepoint == 0x25cc;
    let is_aksara_following_base = |unit: Unit| unit.class == Class::Ak || unit.codepoint == 0x25cc;
    if (left.class == Class::Ap && is_aksara_base(right))
        || (is_aksara_base(left) && matches!(right.class, Class::Vf | Class::Vi))
        || (left.class == Class::Vi
            && is_aksara_following_base(right)
            && context
                .previous(left_index)
                .is_some_and(|index| is_aksara_base(units[index])))
        || (is_aksara_base(left)
            && is_aksara_base(right)
            && context
                .next_after(boundary)
                .is_some_and(|index| units[index].class == Class::Vf))
    {
        return None;
    }

    // LB29-LB30b.
    if left.class == Class::Is && matches!(right.class, Class::Al | Class::Hl) {
        return None;
    }
    if (matches!(left.class, Class::Al | Class::Hl | Class::Nu)
        && right.class == Class::Op
        && !right.is_east_asian())
        || (left.class == Class::Cp
            && !left.is_east_asian()
            && matches!(right.class, Class::Al | Class::Hl | Class::Nu))
    {
        return None;
    }
    if left.class == Class::Ri
        && right.class == Class::Ri
        && context.regional_indicator_odd[boundary]
    {
        return None;
    }
    if (left.class == Class::Eb || left.is_unassigned_extended_pictographic())
        && right.class == Class::Em
    {
        return None;
    }

    // LB31.
    Some(UnicodeBreakKind::Allowed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_version_and_table_are_pinned() {
        assert_eq!(UNICODE_VERSION, "16.0.0");
        assert!(UNICODE_LINE_BREAK_RANGES.len() > 1_000);
    }

    #[test]
    fn empty_input_has_one_mandatory_end_boundary() {
        assert_eq!(
            unicode_line_breaks("").expect("line breaks"),
            [UnicodeBreak {
                byte_offset: 0,
                kind: UnicodeBreakKind::Mandatory,
            }]
        );
    }

    #[test]
    fn basic_and_aksara_boundaries_follow_uax14() {
        let latin = unicode_line_breaks("one two").expect("line breaks");
        assert_eq!(latin[0].byte_offset(), 4);
        assert_eq!(
            latin.last().expect("end").kind(),
            UnicodeBreakKind::Mandatory
        );

        // BALINESE LETTER AKARA + ADEG ADEG + LETTER KA stays one orthographic syllable.
        let aksara = "\u{1b05}\u{1b44}\u{1b13}";
        assert_eq!(
            unicode_line_breaks(aksara).expect("line breaks"),
            [UnicodeBreak {
                byte_offset: aksara.len(),
                kind: UnicodeBreakKind::Mandatory,
            }]
        );
    }

    #[test]
    fn atomic_vector_inline_synthetic_al_matches_al_without_utf8_substitution() {
        let typed = [
            UnicodeLineBreakUnit::Scalar('日'),
            UnicodeLineBreakUnit::SyntheticAl,
            UnicodeLineBreakUnit::Scalar('、'),
        ];
        let scalar = [
            UnicodeLineBreakUnit::Scalar('日'),
            UnicodeLineBreakUnit::Scalar('A'),
            UnicodeLineBreakUnit::Scalar('、'),
        ];
        assert_eq!(
            unicode_line_breaks_for_units(&typed).expect("typed breaks"),
            unicode_line_breaks_for_units(&scalar).expect("scalar breaks")
        );
        assert!(!typed
            .iter()
            .any(|unit| matches!(unit, UnicodeLineBreakUnit::Scalar('\u{fffc}'))));
    }
}
