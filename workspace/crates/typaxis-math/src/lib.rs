#![forbid(unsafe_code)]

use typaxis_core::{push_jcs_string, sha256, JSON_SAFE_INTEGER_MAX};
use typaxis_font::{MathFontError, MathFontFace, OriginalGlyphId};

pub const MATH_SOURCE_LANGUAGE: &str = "typaxis-math";
pub const MATH_SOURCE_VERSION: &str = "1";
pub const MATH_SOURCE_ID: &str = "typaxis.math-source/1";
pub const MATH_PARSER_ID: &str = "typaxis.math-parser/1";
pub const MATH_FORMATTER_ID: &str = "typaxis.math-formatter/1";
pub const MATH_AST_FINGERPRINT_ID: &str = "typaxis.math-ast-fingerprint/1";
pub const MATH_COMPUTATION_ID: &str = "typaxis.math-layout/1";
pub const MATH_LAYOUT_WORK_ID: &str = "typaxis.math-layout-work/1";
pub const MATH_VECTOR_IR_ID: &str = "typaxis.math-vector-ir/1";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MathNodeKind {
    Inline,
    Display,
}

impl MathNodeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline_math",
            Self::Display => "display_math",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MathParseLimits {
    max_nodes: u64,
    max_depth: u32,
}

impl MathParseLimits {
    pub const fn new(max_nodes: u64, max_depth: u32) -> Option<Self> {
        if max_nodes == 0 || max_depth == 0 {
            None
        } else {
            Some(Self {
                max_nodes,
                max_depth,
            })
        }
    }
    pub const fn max_nodes(self) -> u64 {
        self.max_nodes
    }
    pub const fn max_depth(self) -> u32 {
        self.max_depth
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MathSourceErrorKind {
    ForbiddenByte,
    UnexpectedToken,
    UnknownCommand,
    InvalidNumber,
    EmptyRow,
    DuplicateScript,
    MissingDelimiter,
    NodeLimit,
    DepthLimit,
    AllocationFailure,
    ReceiptMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MathSourceError {
    kind: MathSourceErrorKind,
    byte_offset: u32,
}

impl MathSourceError {
    const fn new(kind: MathSourceErrorKind, byte_offset: usize) -> Self {
        Self {
            kind,
            byte_offset: if byte_offset > u32::MAX as usize {
                u32::MAX
            } else {
                byte_offset as u32
            },
        }
    }
    pub const fn kind(self) -> MathSourceErrorKind {
        self.kind
    }
    pub const fn byte_offset(self) -> u32 {
        self.byte_offset
    }
}

impl std::fmt::Display for MathSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self.kind {
            MathSourceErrorKind::ForbiddenByte => "forbidden source byte",
            MathSourceErrorKind::UnexpectedToken => "unexpected token",
            MathSourceErrorKind::UnknownCommand => "unknown command",
            MathSourceErrorKind::InvalidNumber => "invalid number",
            MathSourceErrorKind::EmptyRow => "empty row",
            MathSourceErrorKind::DuplicateScript => "duplicate script",
            MathSourceErrorKind::MissingDelimiter => "missing delimiter",
            MathSourceErrorKind::NodeLimit => "AST node limit exceeded",
            MathSourceErrorKind::DepthLimit => "AST depth limit exceeded",
            MathSourceErrorKind::AllocationFailure => "allocation failed",
            MathSourceErrorKind::ReceiptMismatch => "parser/formatter receipt mismatch",
        };
        let code = match self.kind {
            MathSourceErrorKind::NodeLimit => "P1120",
            MathSourceErrorKind::DepthLimit => "P1121",
            _ => "P1102",
        };
        write!(
            formatter,
            "{code}: math {message} at source byte {}",
            self.byte_offset
        )
    }
}

impl std::error::Error for MathSourceError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MathAst {
    root: Row,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Row {
    terms: Vec<Term>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Term {
    atom: Atom,
    subscript: Option<Atom>,
    superscript: Option<Atom>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Atom {
    Identifier(String),
    Number(String),
    Symbol(char),
    Group(Row),
    Fraction { numerator: Row, denominator: Row },
    Radical(Row),
    Operator(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedMathReceipt {
    source: String,
    source_sha256: [u8; 32],
    ast: MathAst,
    ast_node_count: u64,
    ast_depth: u32,
    ast_jcs: String,
    ast_fingerprint: [u8; 32],
    canonical_source: String,
    receipt_jcs: String,
    receipt_fingerprint: [u8; 32],
}

impl ParsedMathReceipt {
    pub fn source(&self) -> &str {
        &self.source
    }
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub const fn ast_node_count(&self) -> u64 {
        self.ast_node_count
    }
    pub const fn ast_depth(&self) -> u32 {
        self.ast_depth
    }
    pub const fn ast_fingerprint(&self) -> [u8; 32] {
        self.ast_fingerprint
    }
    pub fn canonical_source(&self) -> &str {
        &self.canonical_source
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.receipt_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.receipt_fingerprint
    }

    pub fn verify(&self) -> Result<(), MathSourceError> {
        let (ast_node_count, ast_depth) = measure_ast(&self.ast)?;
        let ast_jcs = encode_ast(&self.ast);
        let canonical_source = format_row(&self.ast.root);
        let receipt_jcs = encode_parsed_receipt(
            sha256(self.source.as_bytes()),
            sha256(ast_jcs.as_bytes()),
            ast_node_count,
            ast_depth,
            &canonical_source,
        );
        if sha256(self.source.as_bytes()) != self.source_sha256
            || ast_node_count != self.ast_node_count
            || ast_depth != self.ast_depth
            || ast_jcs != self.ast_jcs
            || sha256(ast_jcs.as_bytes()) != self.ast_fingerprint
            || canonical_source != self.canonical_source
            || receipt_jcs != self.receipt_jcs
            || sha256(receipt_jcs.as_bytes()) != self.receipt_fingerprint
        {
            return Err(MathSourceError::new(
                MathSourceErrorKind::ReceiptMismatch,
                0,
            ));
        }
        Ok(())
    }
}

fn measure_ast(ast: &MathAst) -> Result<(u64, u32), MathSourceError> {
    fn charge(count: &mut u64, maximum_depth: &mut u32, depth: u32) -> Result<(), MathSourceError> {
        *count = count
            .checked_add(1)
            .ok_or_else(|| MathSourceError::new(MathSourceErrorKind::ReceiptMismatch, 0))?;
        *maximum_depth = (*maximum_depth).max(depth);
        Ok(())
    }

    fn row(
        value: &Row,
        depth: u32,
        count: &mut u64,
        maximum_depth: &mut u32,
    ) -> Result<(), MathSourceError> {
        charge(count, maximum_depth, depth)?;
        let child_depth = depth
            .checked_add(1)
            .ok_or_else(|| MathSourceError::new(MathSourceErrorKind::ReceiptMismatch, 0))?;
        for value in &value.terms {
            term(value, child_depth, count, maximum_depth)?;
        }
        Ok(())
    }

    fn term(
        value: &Term,
        depth: u32,
        count: &mut u64,
        maximum_depth: &mut u32,
    ) -> Result<(), MathSourceError> {
        charge(count, maximum_depth, depth)?;
        let atom_depth = depth
            .checked_add(1)
            .ok_or_else(|| MathSourceError::new(MathSourceErrorKind::ReceiptMismatch, 0))?;
        atom(&value.atom, atom_depth, count, maximum_depth)?;
        if let Some(value) = &value.subscript {
            atom(value, atom_depth, count, maximum_depth)?;
        }
        if let Some(value) = &value.superscript {
            atom(value, atom_depth, count, maximum_depth)?;
        }
        Ok(())
    }

    fn atom(
        value: &Atom,
        depth: u32,
        count: &mut u64,
        maximum_depth: &mut u32,
    ) -> Result<(), MathSourceError> {
        charge(count, maximum_depth, depth)?;
        let child_depth = depth
            .checked_add(1)
            .ok_or_else(|| MathSourceError::new(MathSourceErrorKind::ReceiptMismatch, 0))?;
        match value {
            Atom::Group(value) | Atom::Radical(value) => {
                row(value, child_depth, count, maximum_depth)?;
            }
            Atom::Fraction {
                numerator,
                denominator,
            } => {
                row(numerator, child_depth, count, maximum_depth)?;
                row(denominator, child_depth, count, maximum_depth)?;
            }
            Atom::Identifier(_) | Atom::Number(_) | Atom::Symbol(_) | Atom::Operator(_) => {}
        }
        Ok(())
    }

    let mut count = 0;
    let mut maximum_depth = 0;
    row(&ast.root, 1, &mut count, &mut maximum_depth)?;
    Ok((count, maximum_depth))
}

pub fn parse_math_source(
    source: &str,
    limits: MathParseLimits,
) -> Result<ParsedMathReceipt, MathSourceError> {
    validate_source_bytes(source)?;
    let preflight = preflight_math_source(source, limits)?;
    let mut parser = Parser {
        source,
        offset: 0,
        limits,
        node_count: 0,
        maximum_depth: 0,
    };
    let root = parser.parse_row(None, 1)?;
    parser.skip_space();
    if parser.offset != source.len() {
        return Err(parser.error(MathSourceErrorKind::UnexpectedToken));
    }
    if (parser.node_count, parser.maximum_depth) != preflight {
        return Err(MathSourceError::new(
            MathSourceErrorKind::ReceiptMismatch,
            0,
        ));
    }
    let ast = MathAst { root };
    let ast_jcs = encode_ast(&ast);
    let ast_fingerprint = sha256(ast_jcs.as_bytes());
    let canonical_source = format_row(&ast.root);
    let reparsed = parse_unchecked_for_round_trip(&canonical_source, limits)?;
    if encode_ast(&reparsed) != ast_jcs {
        return Err(MathSourceError::new(
            MathSourceErrorKind::ReceiptMismatch,
            0,
        ));
    }
    let source_sha256 = sha256(source.as_bytes());
    let receipt_jcs = encode_parsed_receipt(
        source_sha256,
        ast_fingerprint,
        parser.node_count,
        parser.maximum_depth,
        &canonical_source,
    );
    Ok(ParsedMathReceipt {
        source: source.to_owned(),
        source_sha256,
        ast,
        ast_node_count: parser.node_count,
        ast_depth: parser.maximum_depth,
        ast_jcs,
        ast_fingerprint,
        canonical_source,
        receipt_fingerprint: sha256(receipt_jcs.as_bytes()),
        receipt_jcs,
    })
}

fn parse_unchecked_for_round_trip(
    source: &str,
    limits: MathParseLimits,
) -> Result<MathAst, MathSourceError> {
    validate_source_bytes(source)?;
    let mut parser = Parser {
        source,
        offset: 0,
        limits,
        node_count: 0,
        maximum_depth: 0,
    };
    let root = parser.parse_row(None, 1)?;
    parser.skip_space();
    if parser.offset != source.len() {
        return Err(parser.error(MathSourceErrorKind::UnexpectedToken));
    }
    Ok(MathAst { root })
}

fn validate_source_bytes(source: &str) -> Result<(), MathSourceError> {
    if source.as_bytes().starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(MathSourceError::new(MathSourceErrorKind::ForbiddenByte, 0));
    }
    for (offset, value) in source.char_indices() {
        if value == '\0'
            || (('\u{0001}'..='\u{001f}').contains(&value) && value != '\n')
            || ('\u{007f}'..='\u{009f}').contains(&value)
        {
            return Err(MathSourceError::new(
                MathSourceErrorKind::ForbiddenByte,
                offset,
            ));
        }
    }
    Ok(())
}

/// Allocation-free first pass over producer input. This establishes the exact
/// node/depth charge and refuses max+1 before any AST vector or token string is
/// allocated by the typed parser.
fn preflight_math_source(
    source: &str,
    limits: MathParseLimits,
) -> Result<(u64, u32), MathSourceError> {
    let mut parser = MathSourcePreflight {
        source,
        offset: 0,
        limits,
        node_count: 0,
        maximum_depth: 0,
    };
    parser.parse_row(None, 1)?;
    parser.skip_space();
    if parser.offset != source.len() {
        return Err(parser.error(MathSourceErrorKind::UnexpectedToken));
    }
    Ok((parser.node_count, parser.maximum_depth))
}

struct MathSourcePreflight<'a> {
    source: &'a str,
    offset: usize,
    limits: MathParseLimits,
    node_count: u64,
    maximum_depth: u32,
}

impl MathSourcePreflight<'_> {
    fn error(&self, kind: MathSourceErrorKind) -> MathSourceError {
        MathSourceError::new(kind, self.offset)
    }

    fn charge(&mut self, depth: u32) -> Result<(), MathSourceError> {
        if depth > self.limits.max_depth {
            return Err(self.error(MathSourceErrorKind::DepthLimit));
        }
        self.node_count = self
            .node_count
            .checked_add(1)
            .ok_or_else(|| self.error(MathSourceErrorKind::NodeLimit))?;
        if self.node_count > self.limits.max_nodes {
            return Err(self.error(MathSourceErrorKind::NodeLimit));
        }
        self.maximum_depth = self.maximum_depth.max(depth);
        Ok(())
    }

    fn parse_row(&mut self, terminator: Option<char>, depth: u32) -> Result<(), MathSourceError> {
        self.charge(depth)?;
        let mut nonempty = false;
        loop {
            self.skip_space();
            if self.peek() == terminator || self.peek().is_none() {
                break;
            }
            self.parse_term(
                depth
                    .checked_add(1)
                    .ok_or_else(|| self.error(MathSourceErrorKind::DepthLimit))?,
            )?;
            nonempty = true;
        }
        if !nonempty {
            return Err(self.error(MathSourceErrorKind::EmptyRow));
        }
        Ok(())
    }

    fn parse_term(&mut self, depth: u32) -> Result<(), MathSourceError> {
        self.charge(depth)?;
        let atom_depth = depth
            .checked_add(1)
            .ok_or_else(|| self.error(MathSourceErrorKind::DepthLimit))?;
        self.parse_atom(atom_depth)?;
        let mut subscript = false;
        let mut superscript = false;
        loop {
            self.skip_space();
            match self.peek() {
                Some('_') => {
                    if subscript {
                        return Err(self.error(MathSourceErrorKind::DuplicateScript));
                    }
                    subscript = true;
                    self.bump();
                    self.skip_space();
                    self.parse_atom(atom_depth)?;
                }
                Some('^') => {
                    if superscript {
                        return Err(self.error(MathSourceErrorKind::DuplicateScript));
                    }
                    superscript = true;
                    self.bump();
                    self.skip_space();
                    self.parse_atom(atom_depth)?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn parse_atom(&mut self, depth: u32) -> Result<(), MathSourceError> {
        self.charge(depth)?;
        let Some(value) = self.peek() else {
            return Err(self.error(MathSourceErrorKind::UnexpectedToken));
        };
        if value.is_ascii_alphabetic() {
            self.take_identifier()?;
            return Ok(());
        }
        if value.is_ascii_digit() {
            self.take_number()?;
            return Ok(());
        }
        match value {
            '{' => {
                self.bump();
                self.parse_row(
                    Some('}'),
                    depth
                        .checked_add(1)
                        .ok_or_else(|| self.error(MathSourceErrorKind::DepthLimit))?,
                )?;
                self.expect('}')
            }
            '\\' => self.parse_command(depth),
            _ if is_literal_symbol(value) || is_greek(value) => {
                self.bump();
                Ok(())
            }
            _ => Err(self.error(MathSourceErrorKind::UnexpectedToken)),
        }
    }

    fn parse_command(&mut self, depth: u32) -> Result<(), MathSourceError> {
        self.bump();
        let command = self.take_identifier()?;
        match command {
            "frac" => {
                self.parse_required_group(depth)?;
                self.parse_required_group(depth)
            }
            "sqrt" => self.parse_required_group(depth),
            "operator" => {
                self.skip_space();
                self.expect('{')?;
                self.skip_space();
                self.take_identifier()?;
                self.skip_space();
                self.expect('}')
            }
            _ => Err(self.error(MathSourceErrorKind::UnknownCommand)),
        }
    }

    fn parse_required_group(&mut self, depth: u32) -> Result<(), MathSourceError> {
        self.skip_space();
        self.expect('{')?;
        self.parse_row(
            Some('}'),
            depth
                .checked_add(1)
                .ok_or_else(|| self.error(MathSourceErrorKind::DepthLimit))?,
        )?;
        self.expect('}')
    }

    fn take_identifier(&mut self) -> Result<&str, MathSourceError> {
        let start = self.offset;
        while self.peek().is_some_and(|value| value.is_ascii_alphabetic()) {
            self.bump();
        }
        if start == self.offset {
            return Err(self.error(MathSourceErrorKind::UnexpectedToken));
        }
        Ok(&self.source[start..self.offset])
    }

    fn take_number(&mut self) -> Result<(), MathSourceError> {
        if self.peek() == Some('0') {
            self.bump();
            if self.peek().is_some_and(|value| value.is_ascii_digit()) {
                return Err(self.error(MathSourceErrorKind::InvalidNumber));
            }
        } else {
            while self.peek().is_some_and(|value| value.is_ascii_digit()) {
                self.bump();
            }
        }
        if self.peek() == Some('.') {
            let dot = self.offset;
            self.bump();
            if !self.peek().is_some_and(|value| value.is_ascii_digit()) {
                self.offset = dot;
            } else {
                while self.peek().is_some_and(|value| value.is_ascii_digit()) {
                    self.bump();
                }
            }
        }
        Ok(())
    }

    fn expect(&mut self, expected: char) -> Result<(), MathSourceError> {
        self.skip_space();
        if self.peek() != Some(expected) {
            return Err(self.error(MathSourceErrorKind::MissingDelimiter));
        }
        self.bump();
        Ok(())
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(' ' | '\n')) {
            self.bump();
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn bump(&mut self) {
        if let Some(value) = self.peek() {
            self.offset += value.len_utf8();
        }
    }
}

struct Parser<'a> {
    source: &'a str,
    offset: usize,
    limits: MathParseLimits,
    node_count: u64,
    maximum_depth: u32,
}

impl Parser<'_> {
    fn error(&self, kind: MathSourceErrorKind) -> MathSourceError {
        MathSourceError::new(kind, self.offset)
    }

    fn charge(&mut self, depth: u32) -> Result<(), MathSourceError> {
        if depth > self.limits.max_depth {
            return Err(self.error(MathSourceErrorKind::DepthLimit));
        }
        self.node_count = self
            .node_count
            .checked_add(1)
            .ok_or_else(|| self.error(MathSourceErrorKind::NodeLimit))?;
        if self.node_count > self.limits.max_nodes {
            return Err(self.error(MathSourceErrorKind::NodeLimit));
        }
        self.maximum_depth = self.maximum_depth.max(depth);
        Ok(())
    }

    fn parse_row(&mut self, terminator: Option<char>, depth: u32) -> Result<Row, MathSourceError> {
        self.charge(depth)?;
        let mut terms = Vec::new();
        loop {
            self.skip_space();
            if self.peek() == terminator || self.peek().is_none() {
                break;
            }
            terms
                .try_reserve(1)
                .map_err(|_| self.error(MathSourceErrorKind::AllocationFailure))?;
            terms.push(
                self.parse_term(
                    depth
                        .checked_add(1)
                        .ok_or_else(|| self.error(MathSourceErrorKind::DepthLimit))?,
                )?,
            );
        }
        if terms.is_empty() {
            return Err(self.error(MathSourceErrorKind::EmptyRow));
        }
        Ok(Row { terms })
    }

    fn parse_term(&mut self, depth: u32) -> Result<Term, MathSourceError> {
        self.charge(depth)?;
        let atom_depth = depth
            .checked_add(1)
            .ok_or_else(|| self.error(MathSourceErrorKind::DepthLimit))?;
        let atom = self.parse_atom(atom_depth)?;
        let mut subscript = None;
        let mut superscript = None;
        loop {
            self.skip_space();
            match self.peek() {
                Some('_') => {
                    if subscript.is_some() {
                        return Err(self.error(MathSourceErrorKind::DuplicateScript));
                    }
                    self.bump();
                    self.skip_space();
                    subscript = Some(self.parse_atom(atom_depth)?);
                }
                Some('^') => {
                    if superscript.is_some() {
                        return Err(self.error(MathSourceErrorKind::DuplicateScript));
                    }
                    self.bump();
                    self.skip_space();
                    superscript = Some(self.parse_atom(atom_depth)?);
                }
                _ => break,
            }
        }
        Ok(Term {
            atom,
            subscript,
            superscript,
        })
    }

    fn parse_atom(&mut self, depth: u32) -> Result<Atom, MathSourceError> {
        self.charge(depth)?;
        let Some(value) = self.peek() else {
            return Err(self.error(MathSourceErrorKind::UnexpectedToken));
        };
        if value.is_ascii_alphabetic() {
            return self.take_identifier().map(Atom::Identifier);
        }
        if value.is_ascii_digit() {
            return self.take_number().map(Atom::Number);
        }
        match value {
            '{' => {
                self.bump();
                let row = self.parse_row(
                    Some('}'),
                    depth
                        .checked_add(1)
                        .ok_or_else(|| self.error(MathSourceErrorKind::DepthLimit))?,
                )?;
                self.expect('}')?;
                Ok(Atom::Group(row))
            }
            '\\' => self.parse_command(depth),
            _ if is_literal_symbol(value) || is_greek(value) => {
                self.bump();
                Ok(Atom::Symbol(value))
            }
            _ => Err(self.error(MathSourceErrorKind::UnexpectedToken)),
        }
    }

    fn parse_command(&mut self, depth: u32) -> Result<Atom, MathSourceError> {
        self.bump();
        let command = self.take_identifier()?;
        match command.as_str() {
            "frac" => {
                let numerator = self.parse_required_group(depth)?;
                let denominator = self.parse_required_group(depth)?;
                Ok(Atom::Fraction {
                    numerator,
                    denominator,
                })
            }
            "sqrt" => Ok(Atom::Radical(self.parse_required_group(depth)?)),
            "operator" => {
                self.skip_space();
                self.expect('{')?;
                self.skip_space();
                let value = self.take_identifier()?;
                self.skip_space();
                self.expect('}')?;
                Ok(Atom::Operator(value))
            }
            _ => Err(self.error(MathSourceErrorKind::UnknownCommand)),
        }
    }

    fn parse_required_group(&mut self, depth: u32) -> Result<Row, MathSourceError> {
        self.skip_space();
        self.expect('{')?;
        let row = self.parse_row(
            Some('}'),
            depth
                .checked_add(1)
                .ok_or_else(|| self.error(MathSourceErrorKind::DepthLimit))?,
        )?;
        self.expect('}')?;
        Ok(row)
    }

    fn take_identifier(&mut self) -> Result<String, MathSourceError> {
        let start = self.offset;
        while self.peek().is_some_and(|value| value.is_ascii_alphabetic()) {
            self.bump();
        }
        if start == self.offset {
            return Err(self.error(MathSourceErrorKind::UnexpectedToken));
        }
        Ok(self.source[start..self.offset].to_owned())
    }

    fn take_number(&mut self) -> Result<String, MathSourceError> {
        let start = self.offset;
        if self.peek() == Some('0') {
            self.bump();
            if self.peek().is_some_and(|value| value.is_ascii_digit()) {
                return Err(self.error(MathSourceErrorKind::InvalidNumber));
            }
        } else {
            while self.peek().is_some_and(|value| value.is_ascii_digit()) {
                self.bump();
            }
        }
        if self.peek() == Some('.') {
            let dot = self.offset;
            self.bump();
            if !self.peek().is_some_and(|value| value.is_ascii_digit()) {
                self.offset = dot;
            } else {
                while self.peek().is_some_and(|value| value.is_ascii_digit()) {
                    self.bump();
                }
            }
        }
        Ok(self.source[start..self.offset].to_owned())
    }

    fn expect(&mut self, expected: char) -> Result<(), MathSourceError> {
        self.skip_space();
        if self.peek() != Some(expected) {
            return Err(self.error(MathSourceErrorKind::MissingDelimiter));
        }
        self.bump();
        Ok(())
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(' ' | '\n')) {
            self.bump();
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn bump(&mut self) {
        if let Some(value) = self.peek() {
            self.offset += value.len_utf8();
        }
    }
}

fn is_literal_symbol(value: char) -> bool {
    matches!(
        value,
        '+' | '-'
            | '±'
            | '∓'
            | '×'
            | '÷'
            | '·'
            | '='
            | '≠'
            | '<'
            | '>'
            | '≤'
            | '≥'
            | '≈'
            | '≡'
            | '∼'
            | '∝'
            | '∈'
            | '∉'
            | '∋'
            | '⊂'
            | '⊆'
            | '⊃'
            | '⊇'
            | '∪'
            | '∩'
            | '∧'
            | '∨'
            | '¬'
            | '∀'
            | '∃'
            | '∅'
            | '∞'
            | '∂'
            | '∇'
            | '∑'
            | '∏'
            | '∫'
            | '→'
            | '←'
            | '↔'
            | '↦'
            | '('
            | ')'
            | '['
            | ']'
            | '|'
            | '‖'
            | ','
            | '.'
            | ':'
            | ';'
            | '!'
    )
}

fn is_greek(value: char) -> bool {
    matches!(
        value,
        'α' | 'β'
            | 'γ'
            | 'δ'
            | 'ε'
            | 'ζ'
            | 'η'
            | 'θ'
            | 'ι'
            | 'κ'
            | 'λ'
            | 'μ'
            | 'ν'
            | 'ξ'
            | 'ο'
            | 'π'
            | 'ρ'
            | 'σ'
            | 'τ'
            | 'υ'
            | 'φ'
            | 'χ'
            | 'ψ'
            | 'ω'
            | 'Α'
            | 'Β'
            | 'Γ'
            | 'Δ'
            | 'Ε'
            | 'Ζ'
            | 'Η'
            | 'Θ'
            | 'Ι'
            | 'Κ'
            | 'Λ'
            | 'Μ'
            | 'Ν'
            | 'Ξ'
            | 'Ο'
            | 'Π'
            | 'Ρ'
            | 'Σ'
            | 'Τ'
            | 'Υ'
            | 'Φ'
            | 'Χ'
            | 'Ψ'
            | 'Ω'
            | 'ϑ'
            | 'ϕ'
            | 'ϖ'
            | 'ϱ'
            | 'ϵ'
    )
}

fn encode_ast(ast: &MathAst) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MATH_AST_FINGERPRINT_ID);
    output.push_str(",\"root\":");
    encode_row(&ast.root, &mut output);
    output.push('}');
    output
}

fn encode_row(row: &Row, output: &mut String) {
    output.push_str("{\"kind\":\"row\",\"terms\":[");
    for (index, term) in row.terms.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"atom\":");
        encode_atom(&term.atom, output);
        output.push_str(",\"kind\":\"term\",\"subscript\":");
        if let Some(atom) = &term.subscript {
            encode_atom(atom, output);
        } else {
            output.push_str("null");
        }
        output.push_str(",\"superscript\":");
        if let Some(atom) = &term.superscript {
            encode_atom(atom, output);
        } else {
            output.push_str("null");
        }
        output.push('}');
    }
    output.push_str("]}");
}

fn encode_atom(atom: &Atom, output: &mut String) {
    match atom {
        Atom::Identifier(value) => encode_token_atom("identifier", value, output),
        Atom::Number(value) => encode_token_atom("number", value, output),
        Atom::Symbol(value) => encode_token_atom("symbol", &value.to_string(), output),
        Atom::Group(row) => {
            output.push_str("{\"kind\":\"group\",\"row\":");
            encode_row(row, output);
            output.push('}');
        }
        Atom::Fraction {
            numerator,
            denominator,
        } => {
            output.push_str("{\"denominator\":");
            encode_row(denominator, output);
            output.push_str(",\"kind\":\"fraction\",\"numerator\":");
            encode_row(numerator, output);
            output.push('}');
        }
        Atom::Radical(row) => {
            output.push_str("{\"kind\":\"radical\",\"radicand\":");
            encode_row(row, output);
            output.push('}');
        }
        Atom::Operator(value) => encode_token_atom("operator", value, output),
    }
}

fn encode_token_atom(kind: &str, value: &str, output: &mut String) {
    output.push_str("{\"kind\":");
    push_jcs_string(output, kind);
    output.push_str(",\"value\":");
    push_jcs_string(output, value);
    output.push('}');
}

fn format_row(row: &Row) -> String {
    let mut output = String::new();
    let mut previous_last = None;
    for term in &row.terms {
        let formatted = format_term(term);
        let first = formatted.chars().next();
        if separator_required(previous_last, first) {
            output.push(' ');
        }
        output.push_str(&formatted);
        previous_last = formatted.chars().last();
    }
    output
}

fn separator_required(left: Option<char>, right: Option<char>) -> bool {
    matches!((left, right), (Some(a), Some(b)) if
        (a.is_ascii_alphabetic() && b.is_ascii_alphabetic())
        || (a.is_ascii_digit() && b.is_ascii_digit())
        || (a.is_ascii_digit() && b == '.')
        || (a == '.' && b.is_ascii_digit()))
}

fn format_term(term: &Term) -> String {
    let mut output = format_atom(&term.atom);
    if let Some(atom) = &term.subscript {
        output.push('_');
        output.push_str(&format_atom(atom));
    }
    if let Some(atom) = &term.superscript {
        output.push('^');
        output.push_str(&format_atom(atom));
    }
    output
}

fn format_atom(atom: &Atom) -> String {
    match atom {
        Atom::Identifier(value) | Atom::Number(value) | Atom::Operator(value) => {
            if matches!(atom, Atom::Operator(_)) {
                format!("\\operator{{{value}}}")
            } else {
                value.clone()
            }
        }
        Atom::Symbol(value) => value.to_string(),
        Atom::Group(row) => format!("{{{}}}", format_row(row)),
        Atom::Fraction {
            numerator,
            denominator,
        } => format!(
            "\\frac{{{}}}{{{}}}",
            format_row(numerator),
            format_row(denominator)
        ),
        Atom::Radical(row) => format!("\\sqrt{{{}}}", format_row(row)),
    }
}

fn encode_parsed_receipt(
    source_sha256: [u8; 32],
    ast_fingerprint: [u8; 32],
    ast_node_count: u64,
    ast_depth: u32,
    canonical_source: &str,
) -> String {
    let mut output =
        String::from("{\"algorithm\":\"typaxis.math-parsed-source-receipt/1\",\"ast_depth\":");
    output.push_str(&ast_depth.to_string());
    output.push_str(",\"ast_fingerprint\":");
    push_hash(&mut output, ast_fingerprint);
    output.push_str(",\"ast_fingerprint_algorithm\":");
    push_jcs_string(&mut output, MATH_AST_FINGERPRINT_ID);
    output.push_str(",\"ast_node_count\":");
    output.push_str(&ast_node_count.to_string());
    output.push_str(",\"canonical_source\":");
    push_jcs_string(&mut output, canonical_source);
    output.push_str(",\"formatter\":");
    push_jcs_string(&mut output, MATH_FORMATTER_ID);
    output.push_str(",\"parser\":");
    push_jcs_string(&mut output, MATH_PARSER_ID);
    output.push_str(",\"source_identity\":");
    push_jcs_string(&mut output, MATH_SOURCE_ID);
    output.push_str(",\"source_sha256\":");
    push_hash(&mut output, source_sha256);
    output.push('}');
    output
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MathComputationInput {
    kind: MathNodeKind,
    font_size_raw: i64,
    max_layout_units: u64,
}

impl MathComputationInput {
    pub const fn new(
        kind: MathNodeKind,
        font_size_raw: i64,
        max_layout_units: u64,
    ) -> Option<Self> {
        if font_size_raw <= 0 || max_layout_units == 0 {
            None
        } else {
            Some(Self {
                kind,
                font_size_raw,
                max_layout_units,
            })
        }
    }
    pub const fn kind(self) -> MathNodeKind {
        self.kind
    }
    pub const fn font_size_raw(self) -> i64 {
        self.font_size_raw
    }
    pub const fn max_layout_units(self) -> u64 {
        self.max_layout_units
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MathGlyphPaint {
    original_gid: OriginalGlyphId,
    unicode: char,
    logical_ordinal: u32,
    x: i64,
    y: i64,
    font_size_raw: i64,
    advance: i64,
}

impl MathGlyphPaint {
    pub const fn original_gid(&self) -> OriginalGlyphId {
        self.original_gid
    }
    pub const fn unicode(&self) -> char {
        self.unicode
    }
    pub const fn logical_ordinal(&self) -> u32 {
        self.logical_ordinal
    }
    pub const fn x(&self) -> i64 {
        self.x
    }
    pub const fn y(&self) -> i64 {
        self.y
    }
    pub const fn font_size_raw(&self) -> i64 {
        self.font_size_raw
    }
    pub const fn advance(&self) -> i64 {
        self.advance
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MathRulePaint {
    x: i64,
    y: i64,
    width: i64,
    height: i64,
}

impl MathRulePaint {
    pub const fn x(self) -> i64 {
        self.x
    }
    pub const fn y(self) -> i64 {
        self.y
    }
    pub const fn width(self) -> i64 {
        self.width
    }
    pub const fn height(self) -> i64 {
        self.height
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MathPaint {
    Glyph(MathGlyphPaint),
    Rule(MathRulePaint),
}

pub fn math_vector_fingerprint(paints: &[MathPaint]) -> [u8; 32] {
    sha256(encode_vector(paints).as_bytes())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MathDimensions {
    advance: i64,
    ascent: i64,
    descent: i64,
    baseline: i64,
    axis: i64,
    bbox_x_min: i64,
    bbox_y_min: i64,
    bbox_x_max: i64,
    bbox_y_max: i64,
}

impl MathDimensions {
    pub const fn advance(self) -> i64 {
        self.advance
    }
    pub const fn ascent(self) -> i64 {
        self.ascent
    }
    pub const fn descent(self) -> i64 {
        self.descent
    }
    pub const fn baseline(self) -> i64 {
        self.baseline
    }
    pub const fn axis(self) -> i64 {
        self.axis
    }
    pub const fn bbox(self) -> (i64, i64, i64, i64) {
        (
            self.bbox_x_min,
            self.bbox_y_min,
            self.bbox_x_max,
            self.bbox_y_max,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MathComputationError {
    ParsedReceipt,
    Font(MathFontError),
    LayoutUnitLimit,
    ArithmeticOverflow,
    EmptyPaint,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for MathComputationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParsedReceipt => formatter.write_str("I9190: parsed math receipt mismatch"),
            Self::Font(error) => error.fmt(formatter),
            Self::LayoutUnitLimit => formatter.write_str("L5111: math layout work limit exceeded"),
            Self::ArithmeticOverflow => {
                formatter.write_str("L5100: math layout arithmetic overflow")
            }
            Self::EmptyPaint => formatter.write_str("L5100: math produced no vector paint"),
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: math computation receipt mismatch")
            }
            Self::AllocationFailure => formatter.write_str("L5111: math layout allocation failed"),
        }
    }
}

impl std::error::Error for MathComputationError {}

impl From<MathFontError> for MathComputationError {
    fn from(value: MathFontError) -> Self {
        Self::Font(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MathComputationReceipt {
    parsed_fingerprint: [u8; 32],
    kind: MathNodeKind,
    font_size_raw: i64,
    math_table_fingerprint: [u8; 32],
    dimensions: MathDimensions,
    paints: Vec<MathPaint>,
    vector_fingerprint: [u8; 32],
    layout_work: u64,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl MathComputationReceipt {
    pub const fn parsed_fingerprint(&self) -> [u8; 32] {
        self.parsed_fingerprint
    }
    pub const fn kind(&self) -> MathNodeKind {
        self.kind
    }
    pub const fn font_size_raw(&self) -> i64 {
        self.font_size_raw
    }
    pub const fn math_table_fingerprint(&self) -> [u8; 32] {
        self.math_table_fingerprint
    }
    pub const fn dimensions(&self) -> MathDimensions {
        self.dimensions
    }
    pub fn paints(&self) -> &[MathPaint] {
        &self.paints
    }
    pub const fn vector_fingerprint(&self) -> [u8; 32] {
        self.vector_fingerprint
    }
    pub const fn layout_work(&self) -> u64 {
        self.layout_work
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify_sealed(
        &self,
        parsed: &ParsedMathReceipt,
        face: MathFontFace<'_>,
    ) -> Result<(), MathComputationError> {
        parsed
            .verify()
            .map_err(|_| MathComputationError::ParsedReceipt)?;
        let input =
            MathComputationInput::new(self.kind, self.font_size_raw, self.layout_work.max(1))
                .ok_or(MathComputationError::ReceiptMismatch)?;
        let canonical_jcs = encode_computation(
            parsed.fingerprint(),
            input,
            face.math_table_fingerprint(),
            self.dimensions,
            math_vector_fingerprint(&self.paints),
            self.layout_work,
        );
        if self.parsed_fingerprint != parsed.fingerprint()
            || self.math_table_fingerprint != face.math_table_fingerprint()
            || self.paints.is_empty()
            || self.dimensions.advance <= 0
            || self.dimensions.ascent <= 0
            || self.dimensions.descent < 0
            || self.vector_fingerprint != math_vector_fingerprint(&self.paints)
            || self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
        {
            return Err(MathComputationError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn compute_math(
    parsed: &ParsedMathReceipt,
    face: MathFontFace<'_>,
    input: MathComputationInput,
) -> Result<MathComputationReceipt, MathComputationError> {
    parsed
        .verify()
        .map_err(|_| MathComputationError::ParsedReceipt)?;
    let preflight = preflight_layout_work(&parsed.ast)?;
    let expected_layout_work = required_math_layout_units_from_preflight(parsed, preflight)?;
    if expected_layout_work > input.max_layout_units {
        return Err(MathComputationError::LayoutUnitLimit);
    }
    let work = LayoutWorkBudget::new(parsed.ast_node_count, expected_layout_work)?;
    let mut context = LayoutContext {
        face,
        kind: input.kind,
        root_size: input.font_size_raw,
        logical_ordinal: 0,
        work,
    };
    let result = context.layout_row(&parsed.ast.root, 0)?;
    if result.paints.is_empty() || result.width <= 0 || result.ascent <= 0 || result.descent < 0 {
        return Err(MathComputationError::EmptyPaint);
    }
    validate_portable_geometry(&result)?;
    let paint_count =
        u64::try_from(result.paints.len()).map_err(|_| MathComputationError::LayoutUnitLimit)?;
    let layout_work = context.work.consumed();
    if result.box_count != preflight.boxes
        || paint_count != preflight.paints
        || layout_work != expected_layout_work
    {
        return Err(MathComputationError::ReceiptMismatch);
    }
    let axis = scale_font_units(
        i64::from(face.constants().axis_height()),
        input.font_size_raw,
        face.units_per_em(),
    )?;
    if !portable_integer(axis) {
        return Err(MathComputationError::ArithmeticOverflow);
    }
    let dimensions = MathDimensions {
        advance: result.width,
        ascent: result.ascent,
        descent: result.descent,
        baseline: result.ascent,
        axis,
        bbox_x_min: 0,
        bbox_y_min: checked_sub(0, result.ascent)?,
        bbox_x_max: result.width,
        bbox_y_max: result.descent,
    };
    let vector_fingerprint = math_vector_fingerprint(&result.paints);
    let canonical_jcs = encode_computation(
        parsed.fingerprint(),
        input,
        face.math_table_fingerprint(),
        dimensions,
        vector_fingerprint,
        layout_work,
    );
    Ok(MathComputationReceipt {
        parsed_fingerprint: parsed.fingerprint(),
        kind: input.kind,
        font_size_raw: input.font_size_raw,
        math_table_fingerprint: face.math_table_fingerprint(),
        dimensions,
        paints: result.paints,
        vector_fingerprint,
        layout_work,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

/// Exact units that must be reserved from the package/session budget before
/// font parsing or math evaluation. This performs only the bounded typed-AST
/// accounting pass and issues no layout box or paint command.
pub fn required_math_layout_units(parsed: &ParsedMathReceipt) -> Result<u64, MathComputationError> {
    parsed
        .verify()
        .map_err(|_| MathComputationError::ParsedReceipt)?;
    required_math_layout_units_from_preflight(parsed, preflight_layout_work(&parsed.ast)?)
}

fn required_math_layout_units_from_preflight(
    parsed: &ParsedMathReceipt,
    preflight: LayoutWorkPreflight,
) -> Result<u64, MathComputationError> {
    parsed
        .ast_node_count
        .checked_add(preflight.boxes)
        .and_then(|value| value.checked_add(preflight.paints))
        .ok_or(MathComputationError::LayoutUnitLimit)
}

#[derive(Clone, Copy)]
struct LayoutWorkPreflight {
    boxes: u64,
    paints: u64,
}

impl LayoutWorkPreflight {
    const fn zero() -> Self {
        Self {
            boxes: 0,
            paints: 0,
        }
    }

    fn add(self, other: Self) -> Result<Self, MathComputationError> {
        Ok(Self {
            boxes: self
                .boxes
                .checked_add(other.boxes)
                .ok_or(MathComputationError::LayoutUnitLimit)?,
            paints: self
                .paints
                .checked_add(other.paints)
                .ok_or(MathComputationError::LayoutUnitLimit)?,
        })
    }
}

fn preflight_layout_work(ast: &MathAst) -> Result<LayoutWorkPreflight, MathComputationError> {
    fn row(value: &Row) -> Result<LayoutWorkPreflight, MathComputationError> {
        let mut result = LayoutWorkPreflight {
            boxes: 1,
            paints: 0,
        };
        for value in &value.terms {
            result = result.add(term(value)?)?;
        }
        Ok(result)
    }

    fn term(value: &Term) -> Result<LayoutWorkPreflight, MathComputationError> {
        let mut result = LayoutWorkPreflight {
            boxes: 1,
            paints: 0,
        }
        .add(atom(&value.atom)?)?;
        if let Some(value) = &value.subscript {
            result = result.add(atom(value)?)?;
        }
        if let Some(value) = &value.superscript {
            result = result.add(atom(value)?)?;
        }
        Ok(result)
    }

    fn atom(value: &Atom) -> Result<LayoutWorkPreflight, MathComputationError> {
        match value {
            Atom::Identifier(value) | Atom::Number(value) | Atom::Operator(value) => {
                Ok(LayoutWorkPreflight {
                    boxes: 1,
                    paints: u64::try_from(value.chars().count())
                        .map_err(|_| MathComputationError::LayoutUnitLimit)?,
                })
            }
            Atom::Symbol(_) => Ok(LayoutWorkPreflight {
                boxes: 1,
                paints: 1,
            }),
            Atom::Group(value) => LayoutWorkPreflight {
                boxes: 1,
                paints: 0,
            }
            .add(row(value)?),
            Atom::Fraction {
                numerator,
                denominator,
            } => LayoutWorkPreflight {
                boxes: 1,
                paints: 1,
            }
            .add(row(numerator)?)?
            .add(row(denominator)?),
            Atom::Radical(value) => LayoutWorkPreflight {
                boxes: 2,
                paints: 2,
            }
            .add(row(value)?),
        }
    }

    LayoutWorkPreflight::zero().add(row(&ast.root)?)
}

fn portable_integer(value: i64) -> bool {
    (-JSON_SAFE_INTEGER_MAX..=JSON_SAFE_INTEGER_MAX).contains(&value)
}

fn validate_portable_geometry(value: &LayoutBox) -> Result<(), MathComputationError> {
    if !portable_integer(value.width)
        || !portable_integer(value.ascent)
        || !portable_integer(value.descent)
    {
        return Err(MathComputationError::ArithmeticOverflow);
    }
    for paint in &value.paints {
        let valid = match paint {
            MathPaint::Glyph(glyph) => {
                glyph.font_size_raw > 0
                    && glyph.advance > 0
                    && portable_integer(glyph.x)
                    && portable_integer(glyph.y)
                    && portable_integer(glyph.font_size_raw)
                    && portable_integer(glyph.advance)
            }
            MathPaint::Rule(rule) => {
                rule.width > 0
                    && rule.height > 0
                    && portable_integer(rule.x)
                    && portable_integer(rule.y)
                    && portable_integer(rule.width)
                    && portable_integer(rule.height)
            }
        };
        if !valid {
            return Err(MathComputationError::ArithmeticOverflow);
        }
    }
    Ok(())
}

struct LayoutContext<'a> {
    face: MathFontFace<'a>,
    kind: MathNodeKind,
    root_size: i64,
    logical_ordinal: u32,
    work: LayoutWorkBudget,
}

struct LayoutWorkBudget {
    maximum: u64,
    consumed: u64,
}

impl LayoutWorkBudget {
    fn new(ast_nodes: u64, maximum: u64) -> Result<Self, MathComputationError> {
        if ast_nodes > maximum {
            return Err(MathComputationError::LayoutUnitLimit);
        }
        Ok(Self {
            maximum,
            consumed: ast_nodes,
        })
    }

    fn charge(&mut self, units: u64) -> Result<(), MathComputationError> {
        let next = self
            .consumed
            .checked_add(units)
            .ok_or(MathComputationError::LayoutUnitLimit)?;
        if next > self.maximum {
            return Err(MathComputationError::LayoutUnitLimit);
        }
        self.consumed = next;
        Ok(())
    }

    const fn consumed(&self) -> u64 {
        self.consumed
    }
}

struct LayoutBox {
    width: i64,
    ascent: i64,
    descent: i64,
    paints: Vec<MathPaint>,
    box_count: u64,
    italic_correction: i64,
}

impl LayoutContext<'_> {
    fn size_for_level(&self, level: u8) -> Result<i64, MathComputationError> {
        let percent = match level {
            0 => return Ok(self.root_size),
            1 => self.face.constants().script_percent_scale_down(),
            _ => self.face.constants().script_script_percent_scale_down(),
        };
        round_ratio(self.root_size, i64::from(percent), 100)
    }

    fn layout_row(&mut self, row: &Row, level: u8) -> Result<LayoutBox, MathComputationError> {
        self.work.charge(1)?;
        let mut output = LayoutBox {
            width: 0,
            ascent: 0,
            descent: 0,
            paints: Vec::new(),
            box_count: 1,
            italic_correction: 0,
        };
        for term in &row.terms {
            let mut item = self.layout_term(term, level)?;
            translate_paints(&mut item.paints, output.width, 0)?;
            output.width = checked_add(output.width, item.width)?;
            output.ascent = output.ascent.max(item.ascent);
            output.descent = output.descent.max(item.descent);
            output.box_count = output
                .box_count
                .checked_add(item.box_count)
                .ok_or(MathComputationError::ArithmeticOverflow)?;
            output
                .paints
                .try_reserve(item.paints.len())
                .map_err(|_| MathComputationError::AllocationFailure)?;
            output.italic_correction = item.italic_correction;
            output.paints.append(&mut item.paints);
        }
        Ok(output)
    }

    fn layout_term(&mut self, term: &Term, level: u8) -> Result<LayoutBox, MathComputationError> {
        self.work.charge(1)?;
        let mut base = self.layout_atom(&term.atom, level)?;
        base.box_count = base
            .box_count
            .checked_add(1)
            .ok_or(MathComputationError::ArithmeticOverflow)?;
        if term.subscript.is_none() && term.superscript.is_none() {
            return Ok(base);
        }
        let next_level = level.saturating_add(1);
        let base_size = self.size_for_level(level)?;
        let constants = self.face.constants();
        let mut subscript = term
            .subscript
            .as_ref()
            .map(|atom| self.layout_atom(atom, next_level))
            .transpose()?;
        let mut superscript = term
            .superscript
            .as_ref()
            .map(|atom| self.layout_atom(atom, next_level))
            .transpose()?;
        let mut sub_shift = if let Some(script) = &subscript {
            let shift = scale_font_units(
                i64::from(constants.subscript_shift_down()),
                base_size,
                self.face.units_per_em(),
            )?;
            let drop = scale_font_units(
                i64::from(constants.subscript_baseline_drop_min()),
                base_size,
                self.face.units_per_em(),
            )?;
            let top_max = scale_font_units(
                i64::from(constants.subscript_top_max()),
                base_size,
                self.face.units_per_em(),
            )?;
            shift
                .max(checked_add(base.descent, drop)?)
                .max(checked_sub(script.ascent, top_max)?)
        } else {
            0
        };
        let mut super_shift = if let Some(script) = &superscript {
            let shift_constant = if level == 0 {
                constants.superscript_shift_up()
            } else {
                constants.superscript_shift_up_cramped()
            };
            let shift = scale_font_units(
                i64::from(shift_constant),
                base_size,
                self.face.units_per_em(),
            )?;
            let drop = scale_font_units(
                i64::from(constants.superscript_baseline_drop_max()),
                base_size,
                self.face.units_per_em(),
            )?;
            let bottom = scale_font_units(
                i64::from(constants.superscript_bottom_min()),
                base_size,
                self.face.units_per_em(),
            )?;
            shift
                .max(checked_sub(base.ascent, drop)?)
                .max(checked_add(script.descent, bottom)?)
        } else {
            0
        };
        if let (Some(sub), Some(sup)) = (&subscript, &superscript) {
            let bottom_max = scale_font_units(
                i64::from(constants.superscript_bottom_max_with_subscript()),
                base_size,
                self.face.units_per_em(),
            )?;
            super_shift = super_shift.max(checked_add(sup.descent, bottom_max)?);
            let gap_min = scale_font_units(
                i64::from(constants.sub_superscript_gap_min()),
                base_size,
                self.face.units_per_em(),
            )?;
            let actual_gap = checked_sub(
                checked_add(sub_shift, super_shift)?,
                checked_add(sub.ascent, sup.descent)?,
            )?;
            if actual_gap < gap_min {
                sub_shift = checked_add(sub_shift, checked_sub(gap_min, actual_gap)?)?;
            }
        }
        let base_width = base.width;
        let mut right = base_width;
        let space_after = scale_font_units(
            i64::from(constants.space_after_script()),
            base_size,
            self.face.units_per_em(),
        )?;
        if let Some(script) = &mut subscript {
            translate_paints(&mut script.paints, base_width, sub_shift)?;
            right = right.max(checked_add(
                checked_add(base_width, script.width)?,
                space_after,
            )?);
            base.descent = base.descent.max(checked_add(sub_shift, script.descent)?);
            base.box_count = checked_add_u64(base.box_count, script.box_count)?;
            base.paints.append(&mut script.paints);
        }
        if let Some(script) = &mut superscript {
            let x = checked_add(base_width, base.italic_correction.max(0))?;
            translate_paints(&mut script.paints, x, checked_sub(0, super_shift)?)?;
            right = right.max(checked_add(checked_add(x, script.width)?, space_after)?);
            base.ascent = base.ascent.max(checked_add(super_shift, script.ascent)?);
            base.box_count = checked_add_u64(base.box_count, script.box_count)?;
            base.paints.append(&mut script.paints);
        }
        base.width = right;
        base.italic_correction = 0;
        Ok(base)
    }

    fn layout_atom(&mut self, atom: &Atom, level: u8) -> Result<LayoutBox, MathComputationError> {
        match atom {
            Atom::Identifier(value) | Atom::Number(value) | Atom::Operator(value) => {
                self.layout_token(value, level)
            }
            Atom::Symbol(value) => self.layout_symbol(*value, level),
            Atom::Group(row) => {
                self.work.charge(1)?;
                let mut result = self.layout_row(row, level)?;
                result.box_count = checked_add_u64(result.box_count, 1)?;
                Ok(result)
            }
            Atom::Fraction {
                numerator,
                denominator,
            } => self.layout_fraction(numerator, denominator, level),
            Atom::Radical(row) => self.layout_radical(row, level),
        }
    }

    fn layout_token(&mut self, value: &str, level: u8) -> Result<LayoutBox, MathComputationError> {
        self.work.charge(1)?;
        let size = self.size_for_level(level)?;
        let ascent = scale_font_units(
            i64::from(self.face.ascent()),
            size,
            self.face.units_per_em(),
        )?;
        let descent = scale_font_units(
            i64::from(self.face.descent())
                .checked_neg()
                .ok_or(MathComputationError::ArithmeticOverflow)?,
            size,
            self.face.units_per_em(),
        )?;
        let mut width = 0;
        let mut italic_correction = 0;
        let mut paints = Vec::new();
        let paint_count = value.chars().count();
        self.work.charge(
            u64::try_from(paint_count).map_err(|_| MathComputationError::LayoutUnitLimit)?,
        )?;
        paints
            .try_reserve_exact(paint_count)
            .map_err(|_| MathComputationError::AllocationFailure)?;
        for character in value.chars() {
            let glyph = self.face.glyph_id(character)?;
            let advance = scale_font_units(
                i64::from(self.face.advance_width(glyph)?),
                size,
                self.face.units_per_em(),
            )?;
            paints.push(MathPaint::Glyph(MathGlyphPaint {
                original_gid: glyph,
                unicode: character,
                logical_ordinal: self.logical_ordinal,
                x: width,
                y: 0,
                font_size_raw: size,
                advance,
            }));
            self.logical_ordinal = self
                .logical_ordinal
                .checked_add(1)
                .ok_or(MathComputationError::ArithmeticOverflow)?;
            width = checked_add(width, advance)?;
            italic_correction = scale_font_units(
                i64::from(self.face.italic_correction(glyph)?),
                size,
                self.face.units_per_em(),
            )?;
        }
        Ok(LayoutBox {
            width,
            ascent,
            descent,
            paints,
            box_count: 1,
            italic_correction,
        })
    }

    fn layout_symbol(
        &mut self,
        character: char,
        level: u8,
    ) -> Result<LayoutBox, MathComputationError> {
        let size = self.size_for_level(level)?;
        let base = self.face.glyph_id(character)?;
        let nominal_height = self.face.glyph_height(base)?;
        let (glyph, vertical_advance) = if self.kind == MathNodeKind::Display
            && level == 0
            && matches!(character, '∑' | '∏' | '∫')
            && nominal_height < self.face.constants().display_operator_min_height()
        {
            let (variant, advance) = self
                .face
                .vertical_variant(base, self.face.constants().display_operator_min_height())?;
            (variant, Some(advance))
        } else {
            (base, None)
        };
        self.layout_single_glyph(character, glyph, size, vertical_advance)
    }

    fn layout_single_glyph(
        &mut self,
        character: char,
        glyph: OriginalGlyphId,
        size: i64,
        vertical_advance: Option<u16>,
    ) -> Result<LayoutBox, MathComputationError> {
        self.work.charge(2)?;
        let mut ascent = scale_font_units(
            i64::from(self.face.ascent()),
            size,
            self.face.units_per_em(),
        )?;
        let mut descent = scale_font_units(
            i64::from(self.face.descent())
                .checked_neg()
                .ok_or(MathComputationError::ArithmeticOverflow)?,
            size,
            self.face.units_per_em(),
        )?;
        if let Some(vertical_advance) = vertical_advance {
            let total =
                scale_font_units(i64::from(vertical_advance), size, self.face.units_per_em())?;
            let nominal = checked_add(ascent, descent)?;
            if total > nominal {
                let extra = checked_sub(total, nominal)?;
                let extra_ascent = round_ratio(extra, 1, 2)?;
                ascent = checked_add(ascent, extra_ascent)?;
                descent = checked_add(descent, checked_sub(extra, extra_ascent)?)?;
            }
        }
        let advance = scale_font_units(
            i64::from(self.face.advance_width(glyph)?),
            size,
            self.face.units_per_em(),
        )?;
        let italic_correction = scale_font_units(
            i64::from(self.face.italic_correction(glyph)?),
            size,
            self.face.units_per_em(),
        )?;
        let paint = MathGlyphPaint {
            original_gid: glyph,
            unicode: character,
            logical_ordinal: self.logical_ordinal,
            x: 0,
            y: 0,
            font_size_raw: size,
            advance,
        };
        self.logical_ordinal = self
            .logical_ordinal
            .checked_add(1)
            .ok_or(MathComputationError::ArithmeticOverflow)?;
        let mut paints = Vec::new();
        paints
            .try_reserve_exact(1)
            .map_err(|_| MathComputationError::AllocationFailure)?;
        paints.push(MathPaint::Glyph(paint));
        Ok(LayoutBox {
            width: advance,
            ascent,
            descent,
            paints,
            box_count: 1,
            italic_correction,
        })
    }

    fn layout_fraction(
        &mut self,
        numerator: &Row,
        denominator: &Row,
        level: u8,
    ) -> Result<LayoutBox, MathComputationError> {
        self.work.charge(1)?;
        let size = self.size_for_level(level)?;
        let child_level = level.saturating_add(1);
        let mut top = self.layout_row(numerator, child_level)?;
        let mut bottom = self.layout_row(denominator, child_level)?;
        let constants = self.face.constants();
        let rule = scale_font_units(
            i64::from(constants.fraction_rule_thickness()),
            size,
            self.face.units_per_em(),
        )?
        .max(1);
        let display_style = self.kind == MathNodeKind::Display && level == 0;
        let numerator_gap = if display_style {
            constants.fraction_numerator_display_gap_min()
        } else {
            constants.fraction_numerator_gap_min()
        };
        let denominator_gap = if display_style {
            constants.fraction_denominator_display_gap_min()
        } else {
            constants.fraction_denominator_gap_min()
        };
        let top_gap = scale_font_units(
            i64::from(numerator_gap.max(0)),
            size,
            self.face.units_per_em(),
        )?;
        let bottom_gap = scale_font_units(
            i64::from(denominator_gap.max(0)),
            size,
            self.face.units_per_em(),
        )?;
        let side = (size / 10).max(1);
        let width = checked_add(top.width.max(bottom.width), checked_add(side, side)?)?;
        let side_pair = checked_add(side, side)?;
        let top_x = checked_add(
            side,
            checked_sub(checked_sub(width, side_pair)?, top.width)? / 2,
        )?;
        let bottom_x = checked_add(
            side,
            checked_sub(checked_sub(width, side_pair)?, bottom.width)? / 2,
        )?;
        let numerator_shift = if display_style {
            constants.fraction_numerator_display_shift_up()
        } else {
            constants.fraction_numerator_shift_up()
        };
        let denominator_shift = if display_style {
            constants.fraction_denominator_display_shift_down()
        } else {
            constants.fraction_denominator_shift_down()
        };
        let axis = scale_font_units(
            i64::from(constants.axis_height()),
            size,
            self.face.units_per_em(),
        )?;
        let rule_top = checked_sub(checked_sub(0, axis)?, rule / 2)?;
        let rule_above_axis = rule / 2;
        let rule_below_axis = checked_sub(rule, rule_above_axis)?;
        let top_minimum = checked_add(
            checked_add(axis, rule_above_axis)?,
            checked_add(top_gap, top.descent)?,
        )?;
        let top_shift =
            scale_font_units(i64::from(numerator_shift), size, self.face.units_per_em())?
                .max(top_minimum);
        let bottom_minimum = checked_add(
            checked_sub(rule_below_axis, axis)?,
            checked_add(bottom_gap, bottom.ascent)?,
        )?;
        let bottom_shift =
            scale_font_units(i64::from(denominator_shift), size, self.face.units_per_em())?
                .max(bottom_minimum);
        let top_y = checked_sub(0, top_shift)?;
        let bottom_y = bottom_shift;
        translate_paints(&mut top.paints, top_x, top_y)?;
        translate_paints(&mut bottom.paints, bottom_x, bottom_y)?;
        let ascent = checked_add(top.ascent, -top_y)?;
        let descent = checked_add(bottom.descent, bottom_y)?;
        let mut paints = top.paints;
        self.work.charge(1)?;
        let additional_paints = bottom
            .paints
            .len()
            .checked_add(1)
            .ok_or(MathComputationError::ArithmeticOverflow)?;
        paints
            .try_reserve(additional_paints)
            .map_err(|_| MathComputationError::AllocationFailure)?;
        paints.push(MathPaint::Rule(MathRulePaint {
            x: 0,
            y: rule_top,
            width,
            height: rule,
        }));
        paints.append(&mut bottom.paints);
        Ok(LayoutBox {
            width,
            ascent,
            descent,
            paints,
            box_count: checked_add_u64(checked_add_u64(top.box_count, bottom.box_count)?, 1)?,
            italic_correction: 0,
        })
    }

    fn layout_radical(&mut self, row: &Row, level: u8) -> Result<LayoutBox, MathComputationError> {
        self.work.charge(1)?;
        let size = self.size_for_level(level)?;
        let mut radicand = self.layout_row(row, level)?;
        let radical_gap = if self.kind == MathNodeKind::Display && level == 0 {
            self.face.constants().radical_display_vertical_gap()
        } else {
            self.face.constants().radical_vertical_gap()
        };
        let gap = scale_font_units(
            i64::from(radical_gap.max(0)),
            size,
            self.face.units_per_em(),
        )?;
        let rule = scale_font_units(
            i64::from(self.face.constants().radical_rule_thickness()),
            size,
            self.face.units_per_em(),
        )?
        .max(1);
        let target_raw = checked_add(
            checked_add(radicand.ascent, radicand.descent)?,
            checked_add(gap, rule)?,
        )?;
        let target_units = unscale_font_units_ceiling(target_raw, size, self.face.units_per_em())?;
        let base = self.face.glyph_id('√')?;
        let nominal_height = self.face.glyph_height(base)?;
        let (glyph, vertical_advance) = if target_units > nominal_height {
            let (variant, advance) = self.face.vertical_variant(base, target_units)?;
            (variant, Some(advance))
        } else {
            (base, None)
        };
        let mut radical = self.layout_single_glyph('√', glyph, size, vertical_advance)?;
        let x = radical.width;
        translate_paints(&mut radicand.paints, x, 0)?;
        let width = checked_add(radical.width, radicand.width)?;
        let ascent = radical
            .ascent
            .max(checked_add(checked_add(radicand.ascent, gap)?, rule)?);
        self.work.charge(1)?;
        let additional_paints = radicand
            .paints
            .len()
            .checked_add(1)
            .ok_or(MathComputationError::ArithmeticOverflow)?;
        radical
            .paints
            .try_reserve(additional_paints)
            .map_err(|_| MathComputationError::AllocationFailure)?;
        radical.paints.push(MathPaint::Rule(MathRulePaint {
            x,
            y: -checked_add(radicand.ascent, gap)?,
            width: radicand.width,
            height: rule,
        }));
        radical.paints.append(&mut radicand.paints);
        Ok(LayoutBox {
            width,
            ascent,
            descent: radical.descent.max(radicand.descent),
            paints: radical.paints,
            box_count: checked_add_u64(checked_add_u64(radical.box_count, radicand.box_count)?, 1)?,
            italic_correction: 0,
        })
    }
}

fn unscale_font_units_ceiling(
    raw: i64,
    size_raw: i64,
    units_per_em: u16,
) -> Result<u16, MathComputationError> {
    if raw <= 0 || size_raw <= 0 {
        return Err(MathComputationError::ArithmeticOverflow);
    }
    let numerator = raw
        .checked_mul(i64::from(units_per_em))
        .ok_or(MathComputationError::ArithmeticOverflow)?;
    let units = numerator
        .checked_add(size_raw - 1)
        .ok_or(MathComputationError::ArithmeticOverflow)?
        / size_raw;
    u16::try_from(units).map_err(|_| MathComputationError::ArithmeticOverflow)
}

fn translate_paints(
    paints: &mut [MathPaint],
    delta_x: i64,
    delta_y: i64,
) -> Result<(), MathComputationError> {
    for paint in paints {
        match paint {
            MathPaint::Glyph(glyph) => {
                glyph.x = checked_add(glyph.x, delta_x)?;
                glyph.y = checked_add(glyph.y, delta_y)?;
            }
            MathPaint::Rule(rule) => {
                rule.x = checked_add(rule.x, delta_x)?;
                rule.y = checked_add(rule.y, delta_y)?;
            }
        }
    }
    Ok(())
}

fn scale_font_units(
    units: i64,
    size_raw: i64,
    units_per_em: u16,
) -> Result<i64, MathComputationError> {
    let numerator = units
        .checked_mul(size_raw)
        .ok_or(MathComputationError::ArithmeticOverflow)?;
    round_ratio(numerator, 1, i64::from(units_per_em))
}

fn round_ratio(value: i64, multiplier: i64, denominator: i64) -> Result<i64, MathComputationError> {
    let numerator = value
        .checked_mul(multiplier)
        .ok_or(MathComputationError::ArithmeticOverflow)?;
    if denominator <= 0 {
        return Err(MathComputationError::ArithmeticOverflow);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let magnitude = remainder.unsigned_abs();
    let denominator_u =
        u64::try_from(denominator).map_err(|_| MathComputationError::ArithmeticOverflow)?;
    let twice = magnitude
        .checked_mul(2)
        .ok_or(MathComputationError::ArithmeticOverflow)?;
    let increment = twice > denominator_u || (twice == denominator_u && quotient & 1 != 0);
    if increment {
        quotient
            .checked_add(if numerator >= 0 { 1 } else { -1 })
            .ok_or(MathComputationError::ArithmeticOverflow)
    } else {
        Ok(quotient)
    }
}

fn checked_add(left: i64, right: i64) -> Result<i64, MathComputationError> {
    left.checked_add(right)
        .ok_or(MathComputationError::ArithmeticOverflow)
}

fn checked_sub(left: i64, right: i64) -> Result<i64, MathComputationError> {
    left.checked_sub(right)
        .ok_or(MathComputationError::ArithmeticOverflow)
}

fn checked_add_u64(left: u64, right: u64) -> Result<u64, MathComputationError> {
    left.checked_add(right)
        .ok_or(MathComputationError::ArithmeticOverflow)
}

fn encode_vector(paints: &[MathPaint]) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MATH_VECTOR_IR_ID);
    output.push_str(",\"paint\":[");
    for (index, paint) in paints.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        match paint {
            MathPaint::Glyph(glyph) => {
                output.push_str("{\"advance\":");
                output.push_str(&glyph.advance.to_string());
                output.push_str(",\"font_size\":");
                output.push_str(&glyph.font_size_raw.to_string());
                output.push_str(",\"kind\":\"glyph\",\"logical_ordinal\":");
                output.push_str(&glyph.logical_ordinal.to_string());
                output.push_str(",\"original_gid\":");
                output.push_str(&glyph.original_gid.get().to_string());
                output.push_str(",\"unicode\":");
                push_jcs_string(&mut output, &glyph.unicode.to_string());
                output.push_str(",\"x\":");
                output.push_str(&glyph.x.to_string());
                output.push_str(",\"y\":");
                output.push_str(&glyph.y.to_string());
                output.push('}');
            }
            MathPaint::Rule(rule) => {
                output.push_str("{\"height\":");
                output.push_str(&rule.height.to_string());
                output.push_str(",\"kind\":\"rule\",\"width\":");
                output.push_str(&rule.width.to_string());
                output.push_str(",\"x\":");
                output.push_str(&rule.x.to_string());
                output.push_str(",\"y\":");
                output.push_str(&rule.y.to_string());
                output.push('}');
            }
        }
    }
    output.push_str("]}");
    output
}

fn encode_computation(
    parsed_fingerprint: [u8; 32],
    input: MathComputationInput,
    math_table_fingerprint: [u8; 32],
    dimensions: MathDimensions,
    vector_fingerprint: [u8; 32],
    layout_work: u64,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MATH_COMPUTATION_ID);
    output.push_str(",\"dimensions\":{\"advance\":");
    output.push_str(&dimensions.advance.to_string());
    output.push_str(",\"ascent\":");
    output.push_str(&dimensions.ascent.to_string());
    output.push_str(",\"axis\":");
    output.push_str(&dimensions.axis.to_string());
    output.push_str(",\"baseline\":");
    output.push_str(&dimensions.baseline.to_string());
    output.push_str(",\"bbox\":[");
    output.push_str(&dimensions.bbox_x_min.to_string());
    output.push(',');
    output.push_str(&dimensions.bbox_y_min.to_string());
    output.push(',');
    output.push_str(&dimensions.bbox_x_max.to_string());
    output.push(',');
    output.push_str(&dimensions.bbox_y_max.to_string());
    output.push_str("],\"descent\":");
    output.push_str(&dimensions.descent.to_string());
    output.push_str("},\"font_size\":");
    output.push_str(&input.font_size_raw.to_string());
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, input.kind.as_str());
    output.push_str(",\"layout_work\":");
    output.push_str(&layout_work.to_string());
    output.push_str(",\"math_table_fingerprint\":");
    push_hash(&mut output, math_table_fingerprint);
    output.push_str(",\"parsed_fingerprint\":");
    push_hash(&mut output, parsed_fingerprint);
    output.push_str(",\"vector_algorithm\":");
    push_jcs_string(&mut output, MATH_VECTOR_IR_ID);
    output.push_str(",\"vector_fingerprint\":");
    push_hash(&mut output, vector_fingerprint);
    output.push_str(",\"work_algorithm\":");
    push_jcs_string(&mut output, MATH_LAYOUT_WORK_ID);
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

    fn limits() -> MathParseLimits {
        MathParseLimits::new(1_000, 64).unwrap()
    }

    #[test]
    fn parser_and_formatter_round_trip_closed_grammar() {
        for source in [
            "x^{2}",
            "x\n+\n1",
            "x^2_1",
            "\\frac{x+1}{y-2}",
            "\\sqrt{x}",
            "\\operator{sin} x",
            "α+β=γ",
            "1.25+.5",
        ] {
            let parsed = parse_math_source(source, limits()).unwrap();
            parsed.verify().unwrap();
            assert!(!parsed.canonical_source().is_empty());
            assert_ne!(parsed.ast_fingerprint(), [0; 32]);
        }
        let ordered = parse_math_source("x^2_1", limits()).unwrap();
        assert_eq!(ordered.canonical_source(), "x_1^2");
        assert_eq!(
            parse_math_source("x\n+\n1", limits())
                .unwrap()
                .canonical_source(),
            "x+1"
        );
        assert_eq!(
            parse_math_source(". 5", limits())
                .unwrap()
                .canonical_source(),
            ". 5"
        );
    }

    #[test]
    fn parser_rejects_macros_controls_empty_and_duplicate_scripts() {
        for source in [
            "",
            "{}",
            "x^^2",
            "x^2^3",
            "\\unknown{x}",
            "01",
            "x\t+1",
            "$x$",
        ] {
            assert!(parse_math_source(source, limits()).is_err(), "{source:?}");
        }
    }

    #[test]
    fn parser_enforces_inclusive_node_and_depth_limits() {
        let parsed = parse_math_source("x", limits()).unwrap();
        assert!(parse_math_source(
            "x",
            MathParseLimits::new(parsed.ast_node_count(), parsed.ast_depth()).unwrap()
        )
        .is_ok());
        assert_eq!(
            parse_math_source(
                "x",
                MathParseLimits::new(parsed.ast_node_count() - 1, parsed.ast_depth()).unwrap()
            )
            .unwrap_err()
            .kind(),
            MathSourceErrorKind::NodeLimit
        );
        assert_eq!(
            parse_math_source(
                "x",
                MathParseLimits::new(parsed.ast_node_count(), parsed.ast_depth() - 1).unwrap()
            )
            .unwrap_err()
            .kind(),
            MathSourceErrorKind::DepthLimit
        );
    }

    #[test]
    fn parsed_receipt_recomputes_ast_accounting() {
        let mut parsed = parse_math_source("x^{2}", limits()).unwrap();
        parsed.ast_node_count += 1;
        parsed.receipt_jcs = encode_parsed_receipt(
            parsed.source_sha256,
            parsed.ast_fingerprint,
            parsed.ast_node_count,
            parsed.ast_depth,
            &parsed.canonical_source,
        );
        parsed.receipt_fingerprint = sha256(parsed.receipt_jcs.as_bytes());
        assert_eq!(
            parsed.verify().unwrap_err().kind(),
            MathSourceErrorKind::ReceiptMismatch
        );
    }

    #[test]
    fn math_layout_work_limit_is_inclusive_and_does_not_reparse_on_receipt_reuse() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/math/job/math.ttf"
        ));
        let face = MathFontFace::parse(bytes, 0).unwrap();
        let parsed = parse_math_source("x^{2}", limits()).unwrap();
        let unbounded = compute_math(
            &parsed,
            face,
            MathComputationInput::new(MathNodeKind::Inline, 12 * 65_536, u64::MAX).unwrap(),
        )
        .unwrap();
        let exact = unbounded.layout_work();
        assert_eq!(
            compute_math(
                &parsed,
                face,
                MathComputationInput::new(MathNodeKind::Inline, 12 * 65_536, exact).unwrap(),
            )
            .unwrap()
            .layout_work(),
            exact
        );
        assert_eq!(
            compute_math(
                &parsed,
                face,
                MathComputationInput::new(MathNodeKind::Inline, 12 * 65_536, exact - 1).unwrap(),
            )
            .unwrap_err(),
            MathComputationError::LayoutUnitLimit
        );
        parsed.verify().unwrap();
        parsed.verify().unwrap();
    }

    #[test]
    fn math_layout_uses_math_scripts_fractions_radicals_and_display_variants() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/math/job/math.ttf"
        ));
        let face = MathFontFace::parse(bytes, 0).unwrap();
        let compute = |source, kind| {
            let parsed = parse_math_source(source, limits()).unwrap();
            compute_math(
                &parsed,
                face,
                MathComputationInput::new(kind, 12 * 65_536, 1_000).unwrap(),
            )
            .unwrap()
        };

        let scripts = compute("x^{x^2}", MathNodeKind::Inline);
        let glyphs = scripts
            .paints()
            .iter()
            .filter_map(|paint| match paint {
                MathPaint::Glyph(glyph) => Some(glyph),
                MathPaint::Rule(_) => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            glyphs
                .iter()
                .map(|glyph| glyph.font_size_raw())
                .collect::<Vec<_>>(),
            [
                12 * 65_536,
                round_ratio(12 * 65_536, 80, 100).unwrap(),
                round_ratio(12 * 65_536, 60, 100).unwrap(),
            ]
        );
        assert!(glyphs[1].x() > glyphs[0].advance());

        let fraction = compute("\\frac{x}{2}", MathNodeKind::Display);
        assert!(fraction
            .paints()
            .iter()
            .any(|paint| matches!(paint, MathPaint::Rule(_))));

        let radical = compute("\\sqrt{x}", MathNodeKind::Inline);
        assert!(radical.paints().iter().any(|paint| matches!(
            paint,
            MathPaint::Glyph(glyph) if glyph.unicode() == '√' && glyph.original_gid().get() == 2
        )));

        let inline_sum = compute("∑", MathNodeKind::Inline);
        let display_sum = compute("∑", MathNodeKind::Display);
        let first_gid = |receipt: &MathComputationReceipt| match &receipt.paints()[0] {
            MathPaint::Glyph(glyph) => glyph.original_gid().get(),
            MathPaint::Rule(_) => unreachable!(),
        };
        assert_eq!(first_gid(&inline_sum), 1);
        assert_eq!(first_gid(&display_sum), 2);
        assert!(display_sum.dimensions().ascent() > inline_sum.dimensions().ascent());
    }
}
