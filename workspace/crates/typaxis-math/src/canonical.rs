//! One canonical visitor for retained strings and allocation-free comparison.
use super::{
    Atom, MathAst, Row, Term, MATH_AST_FINGERPRINT_ID, MATH_FORMATTER_ID, MATH_PARSER_ID,
    MATH_SOURCE_ID,
};
use std::fmt::{self, Write};
use typaxis_core::write_jcs_string;

pub(super) struct CanonicalComparison<'a> {
    remaining: &'a [u8],
}
impl Write for CanonicalComparison<'_> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.remaining = self
            .remaining
            .strip_prefix(value.as_bytes())
            .ok_or(fmt::Error)?;
        Ok(())
    }
}
pub(super) fn matches<'a>(
    expected: &'a str,
    emit: impl FnOnce(&mut CanonicalComparison<'a>) -> fmt::Result,
) -> bool {
    let mut output = CanonicalComparison {
        remaining: expected.as_bytes(),
    };
    emit(&mut output).is_ok() && output.remaining.is_empty()
}

pub(super) fn encode_ast(ast: &MathAst) -> String {
    let mut output = String::new();
    write_ast(ast, &mut output).expect("writing a math AST to a String cannot fail");
    output
}
pub(super) fn write_ast<W: Write + ?Sized>(ast: &MathAst, output: &mut W) -> fmt::Result {
    output.write_str("{\"algorithm\":")?;
    write_jcs_string(output, MATH_AST_FINGERPRINT_ID)?;
    output.write_str(",\"root\":")?;
    write_row(&ast.root, output)?;
    output.write_char('}')
}
fn write_row<W: Write + ?Sized>(row: &Row, output: &mut W) -> fmt::Result {
    output.write_str("{\"kind\":\"row\",\"terms\":[")?;
    for (index, term) in row.terms.iter().enumerate() {
        if index > 0 {
            output.write_char(',')?;
        }
        output.write_str("{\"atom\":")?;
        write_atom(&term.atom, output)?;
        output.write_str(",\"kind\":\"term\",\"subscript\":")?;
        if let Some(atom) = &term.subscript {
            write_atom(atom, output)?;
        } else {
            output.write_str("null")?;
        }
        output.write_str(",\"superscript\":")?;
        if let Some(atom) = &term.superscript {
            write_atom(atom, output)?;
        } else {
            output.write_str("null")?;
        }
        output.write_char('}')?;
    }
    output.write_str("]}")
}
fn write_atom<W: Write + ?Sized>(atom: &Atom, output: &mut W) -> fmt::Result {
    match atom {
        Atom::Identifier(value) => write_token("identifier", value, output),
        Atom::Number(value) => write_token("number", value, output),
        Atom::Symbol(value) => {
            let mut bytes = [0; 4];
            write_token("symbol", value.encode_utf8(&mut bytes), output)
        }
        Atom::Group(row) => {
            output.write_str("{\"kind\":\"group\",\"row\":")?;
            write_row(row, output)?;
            output.write_char('}')
        }
        Atom::Fraction {
            numerator,
            denominator,
        } => {
            output.write_str("{\"denominator\":")?;
            write_row(denominator, output)?;
            output.write_str(",\"kind\":\"fraction\",\"numerator\":")?;
            write_row(numerator, output)?;
            output.write_char('}')
        }
        Atom::Radical(row) => {
            output.write_str("{\"kind\":\"radical\",\"radicand\":")?;
            write_row(row, output)?;
            output.write_char('}')
        }
        Atom::Operator(value) => write_token("operator", value, output),
    }
}
fn write_token<W: Write + ?Sized>(kind: &str, value: &str, output: &mut W) -> fmt::Result {
    output.write_str("{\"kind\":")?;
    write_jcs_string(output, kind)?;
    output.write_str(",\"value\":")?;
    write_jcs_string(output, value)?;
    output.write_char('}')
}

pub(super) fn format_row(row: &Row) -> String {
    let mut output = String::new();
    write_formatted_row(row, &mut output).expect("writing formatted math to a String cannot fail");
    output
}
pub(super) fn write_formatted_row<W: Write + ?Sized>(row: &Row, output: &mut W) -> fmt::Result {
    let mut previous_last = None;
    for term in &row.terms {
        let (first, last) = term_boundaries(term);
        if separator_required(previous_last, first) {
            output.write_char(' ')?;
        }
        write_formatted_atom(&term.atom, output)?;
        for (marker, script) in [('_', &term.subscript), ('^', &term.superscript)] {
            if let Some(atom) = script {
                output.write_char(marker)?;
                write_formatted_atom(atom, output)?;
            }
        }
        previous_last = last;
    }
    Ok(())
}
fn separator_required(left: Option<char>, right: Option<char>) -> bool {
    matches!((left, right), (Some(a), Some(b)) if
        (a.is_ascii_alphabetic() && b.is_ascii_alphabetic())
        || (a.is_ascii_digit() && b.is_ascii_digit())
        || (a.is_ascii_digit() && b == '.')
        || (a == '.' && b.is_ascii_digit()))
}
fn atom_boundaries(atom: &Atom) -> (Option<char>, Option<char>) {
    match atom {
        Atom::Identifier(value) | Atom::Number(value) => {
            (value.chars().next(), value.chars().next_back())
        }
        Atom::Symbol(value) => (Some(*value), Some(*value)),
        Atom::Group(_) => (Some('{'), Some('}')),
        Atom::Operator(_) | Atom::Fraction { .. } | Atom::Radical(_) => (Some('\\'), Some('}')),
    }
}
fn term_boundaries(term: &Term) -> (Option<char>, Option<char>) {
    let (mut first, mut last) = atom_boundaries(&term.atom);
    for (marker, script) in [('_', &term.subscript), ('^', &term.superscript)] {
        if let Some(atom) = script {
            first = first.or(Some(marker));
            last = atom_boundaries(atom).1.or(Some(marker));
        }
    }
    (first, last)
}
fn write_formatted_atom<W: Write + ?Sized>(atom: &Atom, output: &mut W) -> fmt::Result {
    match atom {
        Atom::Identifier(value) | Atom::Number(value) => output.write_str(value),
        Atom::Symbol(value) => output.write_char(*value),
        Atom::Operator(value) => {
            output.write_str("\\operator{")?;
            output.write_str(value)?;
            output.write_char('}')
        }
        Atom::Group(row) => {
            output.write_char('{')?;
            write_formatted_row(row, output)?;
            output.write_char('}')
        }
        Atom::Fraction {
            numerator,
            denominator,
        } => {
            output.write_str("\\frac{")?;
            write_formatted_row(numerator, output)?;
            output.write_str("}{")?;
            write_formatted_row(denominator, output)?;
            output.write_char('}')
        }
        Atom::Radical(row) => {
            output.write_str("\\sqrt{")?;
            write_formatted_row(row, output)?;
            output.write_char('}')
        }
    }
}

pub(super) fn encode_parsed_receipt(
    source_sha256: [u8; 32],
    ast_fingerprint: [u8; 32],
    ast_node_count: u64,
    ast_depth: u32,
    canonical_source: &str,
) -> String {
    let mut output = String::new();
    write_parsed_receipt(
        source_sha256,
        ast_fingerprint,
        ast_node_count,
        ast_depth,
        canonical_source,
        &mut output,
    )
    .expect("writing a parsed math receipt to a String cannot fail");
    output
}
pub(super) fn write_parsed_receipt<W: Write + ?Sized>(
    source_sha256: [u8; 32],
    ast_fingerprint: [u8; 32],
    ast_node_count: u64,
    ast_depth: u32,
    canonical_source: &str,
    output: &mut W,
) -> fmt::Result {
    output.write_str("{\"algorithm\":\"typaxis.math-parsed-source-receipt/1\",\"ast_depth\":")?;
    write!(output, "{ast_depth}")?;
    output.write_str(",\"ast_fingerprint\":")?;
    write_hash(output, ast_fingerprint)?;
    output.write_str(",\"ast_fingerprint_algorithm\":")?;
    write_jcs_string(output, MATH_AST_FINGERPRINT_ID)?;
    output.write_str(",\"ast_node_count\":")?;
    write!(output, "{ast_node_count}")?;
    output.write_str(",\"canonical_source\":")?;
    write_jcs_string(output, canonical_source)?;
    output.write_str(",\"formatter\":")?;
    write_jcs_string(output, MATH_FORMATTER_ID)?;
    output.write_str(",\"parser\":")?;
    write_jcs_string(output, MATH_PARSER_ID)?;
    output.write_str(",\"source_identity\":")?;
    write_jcs_string(output, MATH_SOURCE_ID)?;
    output.write_str(",\"source_sha256\":")?;
    write_hash(output, source_sha256)?;
    output.write_char('}')
}
pub(super) fn write_hash<W: Write + ?Sized>(output: &mut W, value: [u8; 32]) -> fmt::Result {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.write_char('"')?;
    for byte in value {
        output.write_char(char::from(HEX[usize::from(byte >> 4)]))?;
        output.write_char(char::from(HEX[usize::from(byte & 0x0f)]))?;
    }
    output.write_char('"')
}
