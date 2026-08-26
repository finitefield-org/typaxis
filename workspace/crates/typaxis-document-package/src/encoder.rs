use super::*;
use std::cmp::Ordering;
use std::fmt;
use std::io::{self, Write};
use typaxis_core::{MachineInputLimitBounds, JSON_SAFE_INTEGER_MAX};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanonicalJcsStats {
    bytes: u64,
    sha256: [u8; 32],
}

impl CanonicalJcsStats {
    pub const fn bytes(self) -> u64 {
        self.bytes
    }

    pub const fn sha256(self) -> [u8; 32] {
        self.sha256
    }
}

/// A canonical-output sink that retains only the byte count and SHA-256 state.
#[derive(Clone, Debug)]
pub struct JcsCountHashSink {
    bytes: u64,
    sha256: StreamingSha256,
}

impl JcsCountHashSink {
    pub const fn new() -> Self {
        Self {
            bytes: 0,
            sha256: StreamingSha256::new(),
        }
    }

    pub fn finish(self) -> CanonicalJcsStats {
        CanonicalJcsStats {
            bytes: self.bytes,
            sha256: self.sha256.finish(),
        }
    }
}

impl Default for JcsCountHashSink {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for JcsCountHashSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length =
            u64::try_from(bytes.len()).map_err(|_| io::Error::other("JCS byte count overflow"))?;
        self.bytes = self
            .bytes
            .checked_add(length)
            .ok_or_else(|| io::Error::other("JCS byte count overflow"))?;
        self.sha256.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentPackageEncoder {
    max_bytes: u64,
}

impl DocumentPackageEncoder {
    pub fn new(max_bytes: u64) -> Result<Self, JcsEncodeError> {
        if max_bytes > MachineInputLimitBounds::HARD_MAX_DOCUMENT_PACKAGE_BYTES {
            return Err(JcsEncodeError::ByteLimitAboveHardMaximum {
                requested: max_bytes,
                maximum: MachineInputLimitBounds::HARD_MAX_DOCUMENT_PACKAGE_BYTES,
            });
        }
        Ok(Self { max_bytes })
    }

    pub const fn max_bytes(self) -> u64 {
        self.max_bytes
    }

    /// Count and hash the canonical form without retaining its bytes.
    pub fn analyze(
        &self,
        package: &WireDocumentPackage,
    ) -> Result<CanonicalJcsStats, JcsEncodeError> {
        let mut sink = JcsCountHashSink::new();
        let written = self.write_once(package, &mut sink)?;
        let stats = sink.finish();
        if written != stats.bytes {
            return Err(JcsEncodeError::NonDeterministicEncoding);
        }
        Ok(stats)
    }

    /// Run a complete count/hash preflight before the first write to `output`.
    pub fn write_preflighted<W: Write>(
        &self,
        package: &WireDocumentPackage,
        output: &mut W,
    ) -> Result<CanonicalJcsStats, JcsEncodeError> {
        let expected = self.analyze(package)?;
        let written = self.write_once(package, output)?;
        if written != expected.bytes {
            return Err(JcsEncodeError::NonDeterministicEncoding);
        }
        Ok(expected)
    }

    pub fn to_jcs_vec(&self, package: &WireDocumentPackage) -> Result<Vec<u8>, JcsEncodeError> {
        let expected = self.analyze(package)?;
        let capacity = usize::try_from(expected.bytes)
            .map_err(|_| JcsEncodeError::OutputTooLargeForPlatform)?;
        let mut output = Vec::with_capacity(capacity);
        let written = self.write_once(package, &mut output)?;
        if written != expected.bytes || output.len() != capacity {
            return Err(JcsEncodeError::NonDeterministicEncoding);
        }
        Ok(output)
    }

    pub fn to_jcs_string(&self, package: &WireDocumentPackage) -> Result<String, JcsEncodeError> {
        String::from_utf8(self.to_jcs_vec(package)?)
            .map_err(|_| JcsEncodeError::NonUtf8EncoderOutput)
    }

    fn write_once<W: Write>(
        &self,
        package: &WireDocumentPackage,
        output: W,
    ) -> Result<u64, JcsEncodeError> {
        let mut writer = JcsWriter::new(output, self.max_bytes);
        encode_package(&mut writer, package)?;
        Ok(writer.bytes_written())
    }
}

impl Default for DocumentPackageEncoder {
    fn default() -> Self {
        Self {
            max_bytes: MachineInputLimitBounds::DEFAULT_MAX_DOCUMENT_PACKAGE_BYTES,
        }
    }
}

#[derive(Debug)]
pub enum JcsEncodeError {
    ByteLimitAboveHardMaximum { requested: u64, maximum: u64 },
    ByteLimitExceeded { limit: u64, attempted: u64 },
    JsonNestingDepthExceeded { maximum: u16 },
    IntegerOutOfRange { field: &'static str },
    NonCanonicalMemberOrder { previous: String, current: String },
    OutputTooLargeForPlatform,
    NonDeterministicEncoding,
    NonUtf8EncoderOutput,
    Write(io::Error),
}

impl fmt::Display for JcsEncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ByteLimitAboveHardMaximum { requested, maximum } => write!(
                formatter,
                "JCS byte limit {requested} exceeds hard maximum {maximum}"
            ),
            Self::ByteLimitExceeded { limit, attempted } => write!(
                formatter,
                "canonical DocumentPackage would write byte {attempted} past limit {limit}"
            ),
            Self::JsonNestingDepthExceeded { maximum } => {
                write!(formatter, "canonical JSON nesting exceeds {maximum}")
            }
            Self::IntegerOutOfRange { field } => {
                write!(
                    formatter,
                    "wire integer `{field}` is outside its exact range"
                )
            }
            Self::NonCanonicalMemberOrder { previous, current } => write!(
                formatter,
                "JCS member `{current}` is not after `{previous}` in UTF-16 order"
            ),
            Self::OutputTooLargeForPlatform => {
                formatter.write_str("canonical output does not fit this platform's address space")
            }
            Self::NonDeterministicEncoding => {
                formatter.write_str("canonical encoding changed between passes")
            }
            Self::NonUtf8EncoderOutput => {
                formatter.write_str("canonical encoder produced non-UTF-8 output")
            }
            Self::Write(error) => write!(formatter, "canonical output write failed: {error}"),
        }
    }
}

impl std::error::Error for JcsEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Write(error) => Some(error),
            _ => None,
        }
    }
}

struct JcsWriter<W> {
    output: W,
    max_bytes: u64,
    bytes_written: u64,
    depth: u16,
}

impl<W: Write> JcsWriter<W> {
    const fn new(output: W, max_bytes: u64) -> Self {
        Self {
            output,
            max_bytes,
            bytes_written: 0,
            depth: 0,
        }
    }

    const fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    fn raw(&mut self, bytes: &[u8]) -> Result<(), JcsEncodeError> {
        let length =
            u64::try_from(bytes.len()).map_err(|_| JcsEncodeError::OutputTooLargeForPlatform)?;
        let attempted = self
            .bytes_written
            .checked_add(length)
            .ok_or(JcsEncodeError::OutputTooLargeForPlatform)?;
        if attempted > self.max_bytes {
            return Err(JcsEncodeError::ByteLimitExceeded {
                limit: self.max_bytes,
                attempted,
            });
        }
        self.output
            .write_all(bytes)
            .map_err(JcsEncodeError::Write)?;
        self.bytes_written = attempted;
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), JcsEncodeError> {
        self.raw(b"\"")?;
        let bytes = value.as_bytes();
        let mut unescaped_start = 0;
        for (offset, character) in value.char_indices() {
            let escape = match character {
                '\u{0008}' => Some("\\b"),
                '\u{0009}' => Some("\\t"),
                '\u{000a}' => Some("\\n"),
                '\u{000c}' => Some("\\f"),
                '\u{000d}' => Some("\\r"),
                '"' => Some("\\\""),
                '\\' => Some("\\\\"),
                _ => None,
            };
            if let Some(escape) = escape {
                self.raw(&bytes[unescaped_start..offset])?;
                self.raw(escape.as_bytes())?;
                unescaped_start = offset + character.len_utf8();
            } else if character <= '\u{001f}' {
                self.raw(&bytes[unescaped_start..offset])?;
                let code = character as u8;
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let escaped = [
                    b'\\',
                    b'u',
                    b'0',
                    b'0',
                    HEX[usize::from(code >> 4)],
                    HEX[usize::from(code & 0x0f)],
                ];
                self.raw(&escaped)?;
                unescaped_start = offset + character.len_utf8();
            }
        }
        self.raw(&bytes[unescaped_start..])?;
        self.raw(b"\"")
    }

    fn boolean(&mut self, value: bool) -> Result<(), JcsEncodeError> {
        self.raw(if value { b"true" } else { b"false" })
    }

    fn null(&mut self) -> Result<(), JcsEncodeError> {
        self.raw(b"null")
    }

    fn u32(&mut self, value: u32) -> Result<(), JcsEncodeError> {
        self.raw(value.to_string().as_bytes())
    }

    fn u16(&mut self, field: &'static str, value: u16) -> Result<(), JcsEncodeError> {
        if value == 0 {
            return Err(JcsEncodeError::IntegerOutOfRange { field });
        }
        self.raw(value.to_string().as_bytes())
    }

    fn u8_range(
        &mut self,
        field: &'static str,
        value: u8,
        minimum: u8,
        maximum: u8,
    ) -> Result<(), JcsEncodeError> {
        if !(minimum..=maximum).contains(&value) {
            return Err(JcsEncodeError::IntegerOutOfRange { field });
        }
        self.raw(value.to_string().as_bytes())
    }

    fn safe_i64(&mut self, field: &'static str, value: i64) -> Result<(), JcsEncodeError> {
        if !(-JSON_SAFE_INTEGER_MAX..=JSON_SAFE_INTEGER_MAX).contains(&value) {
            return Err(JcsEncodeError::IntegerOutOfRange { field });
        }
        self.raw(value.to_string().as_bytes())
    }

    fn positive_i64(&mut self, field: &'static str, value: i64) -> Result<(), JcsEncodeError> {
        if !(1..=JSON_SAFE_INTEGER_MAX).contains(&value) {
            return Err(JcsEncodeError::IntegerOutOfRange { field });
        }
        self.raw(value.to_string().as_bytes())
    }

    fn positive_safe_u64(&mut self, field: &'static str, value: u64) -> Result<(), JcsEncodeError> {
        if value == 0 || value > JSON_SAFE_INTEGER_MAX as u64 {
            return Err(JcsEncodeError::IntegerOutOfRange { field });
        }
        self.raw(value.to_string().as_bytes())
    }

    fn object<F>(&mut self, body: F) -> Result<(), JcsEncodeError>
    where
        F: FnOnce(&mut ObjectWriter<'_, W>) -> Result<(), JcsEncodeError>,
    {
        self.enter_container()?;
        self.raw(b"{")?;
        let result = {
            let mut object = ObjectWriter {
                writer: self,
                first: true,
                previous: None,
            };
            body(&mut object)
        };
        let close = if result.is_ok() {
            self.raw(b"}")
        } else {
            Ok(())
        };
        self.depth -= 1;
        result.and(close)
    }

    fn array<T, F>(&mut self, values: &[T], mut encode: F) -> Result<(), JcsEncodeError>
    where
        F: FnMut(&mut Self, &T) -> Result<(), JcsEncodeError>,
    {
        self.enter_container()?;
        self.raw(b"[")?;
        let mut result = Ok(());
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                result = self.raw(b",");
                if result.is_err() {
                    break;
                }
            }
            result = encode(self, value);
            if result.is_err() {
                break;
            }
        }
        let close = if result.is_ok() {
            self.raw(b"]")
        } else {
            Ok(())
        };
        self.depth -= 1;
        result.and(close)
    }

    fn enter_container(&mut self) -> Result<(), JcsEncodeError> {
        if self.depth == MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH {
            return Err(JcsEncodeError::JsonNestingDepthExceeded {
                maximum: MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH,
            });
        }
        self.depth += 1;
        Ok(())
    }
}

struct ObjectWriter<'a, W> {
    writer: &'a mut JcsWriter<W>,
    first: bool,
    previous: Option<String>,
}

impl<W: Write> ObjectWriter<'_, W> {
    fn member<F>(&mut self, name: &str, value: F) -> Result<(), JcsEncodeError>
    where
        F: FnOnce(&mut JcsWriter<W>) -> Result<(), JcsEncodeError>,
    {
        if let Some(previous) = self.previous.as_deref() {
            if utf16_cmp(previous, name) != Ordering::Less {
                return Err(JcsEncodeError::NonCanonicalMemberOrder {
                    previous: previous.to_owned(),
                    current: name.to_owned(),
                });
            }
        }
        if !self.first {
            self.writer.raw(b",")?;
        }
        self.writer.string(name)?;
        self.writer.raw(b":")?;
        value(self.writer)?;
        self.first = false;
        self.previous = Some(name.to_owned());
        Ok(())
    }
}

fn utf16_cmp(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn encode_package<W: Write>(
    writer: &mut JcsWriter<W>,
    package: &WireDocumentPackage,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("contract", |writer| {
            writer.string(package.contract.as_str())
        })?;
        object.member("coordinate_unit", |writer| {
            writer.string(package.coordinate_unit.as_str())
        })?;
        object.member("document", |writer| {
            encode_document(writer, &package.document)
        })?;
        object.member("page_masters", |writer| {
            encode_page_masters(writer, &package.page_masters)
        })?;
        object.member("resources", |writer| {
            encode_resources(writer, &package.resources)
        })?;
        object.member("sources", |writer| {
            writer.array(&package.sources, encode_source)
        })?;
        object.member("style_sheet", |writer| {
            encode_style_sheet(writer, &package.style_sheet)
        })?;
        object.member("text_buffers", |writer| {
            writer.array(&package.text_buffers, encode_text_buffer)
        })
    })
}

fn encode_source<W: Write>(
    writer: &mut JcsWriter<W>,
    source: &WireSource,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("sha256", |writer| encode_hash(writer, source.sha256))?;
        object.member("source_id", |writer| writer.u32(source.source_id))?;
        object.member("uri", |writer| writer.string(&source.uri))?;
        object.member("utf8_byte_length", |writer| {
            writer.u32(source.utf8_byte_length)
        })
    })
}

fn encode_text_buffer<W: Write>(
    writer: &mut JcsWriter<W>,
    buffer: &WireTextBuffer,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("mappings", |writer| {
            writer.array(&buffer.mappings, encode_text_mapping)
        })?;
        object.member("text_id", |writer| writer.u32(buffer.text_id))?;
        object.member("utf8", |writer| writer.string(&buffer.utf8))
    })
}

fn encode_text_mapping<W: Write>(
    writer: &mut JcsWriter<W>,
    mapping: &WireTextMapSegment,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("kind", |writer| writer.string(mapping.kind.as_str()))?;
        object.member("source_span", |writer| {
            encode_optional_source_span(writer, mapping.source_span)
        })?;
        object.member("text_range", |writer| {
            encode_byte_range(writer, mapping.text_range)
        })
    })
}

fn encode_document<W: Write>(
    writer: &mut JcsWriter<W>,
    document: &WireDocument,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("blocks", |writer| {
            writer.array(&document.blocks, encode_block)
        })?;
        object.member("footnotes", |writer| {
            writer.array(&document.footnotes, encode_footnote)
        })?;
        object.member("node_id", |writer| writer.u32(document.node_id))
    })
}

fn encode_block<W: Write>(
    writer: &mut JcsWriter<W>,
    block: &WireBlock,
) -> Result<(), JcsEncodeError> {
    match block {
        WireBlock::Paragraph {
            node_id,
            span,
            classes,
            children,
        } => writer.object(|object| {
            object.member("children", |writer| writer.array(children, encode_inline))?;
            object.member("classes", |writer| encode_strings(writer, classes))?;
            object.member("kind", |writer| writer.string("paragraph"))?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("span", |writer| encode_source_span(writer, *span))
        }),
        WireBlock::Heading {
            node_id,
            span,
            classes,
            level,
            anchor_id,
            children,
        } => writer.object(|object| {
            object.member("anchor_id", |writer| {
                encode_optional_string(writer, anchor_id.as_deref())
            })?;
            object.member("children", |writer| writer.array(children, encode_inline))?;
            object.member("classes", |writer| encode_strings(writer, classes))?;
            object.member("kind", |writer| writer.string("heading"))?;
            object.member("level", |writer| {
                writer.u8_range("document.blocks[].level", *level, 1, 6)
            })?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("span", |writer| encode_source_span(writer, *span))
        }),
        WireBlock::List {
            node_id,
            span,
            classes,
            ordered,
            start,
            items,
        } => writer.object(|object| {
            object.member("classes", |writer| encode_strings(writer, classes))?;
            object.member("items", |writer| writer.array(items, encode_list_item))?;
            object.member("kind", |writer| writer.string("list"))?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("ordered", |writer| writer.boolean(*ordered))?;
            object.member("span", |writer| encode_source_span(writer, *span))?;
            object.member("start", |writer| encode_optional_u32(writer, *start))
        }),
        WireBlock::Table {
            node_id,
            span,
            classes,
            columns,
            head,
            body,
        } => writer.object(|object| {
            object.member("body", |writer| writer.array(body, encode_table_row))?;
            object.member("classes", |writer| encode_strings(writer, classes))?;
            object.member("columns", |writer| {
                writer.array(columns, encode_table_column)
            })?;
            object.member("head", |writer| writer.array(head, encode_table_row))?;
            object.member("kind", |writer| writer.string("table"))?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("span", |writer| encode_source_span(writer, *span))
        }),
        WireBlock::Figure {
            node_id,
            span,
            classes,
            image_id,
            alt,
            caption,
        } => writer.object(|object| {
            object.member("alt", |writer| writer.string(alt))?;
            object.member("caption", |writer| writer.array(caption, encode_block))?;
            object.member("classes", |writer| encode_strings(writer, classes))?;
            object.member("image_id", |writer| writer.u32(*image_id))?;
            object.member("kind", |writer| writer.string("figure"))?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("span", |writer| encode_source_span(writer, *span))
        }),
        WireBlock::PageBreak {
            node_id,
            span,
            classes,
        } => writer.object(|object| {
            object.member("classes", |writer| encode_strings(writer, classes))?;
            object.member("kind", |writer| writer.string("page_break"))?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("span", |writer| encode_source_span(writer, *span))
        }),
    }
}

fn encode_inline<W: Write>(
    writer: &mut JcsWriter<W>,
    inline: &WireInline,
) -> Result<(), JcsEncodeError> {
    match inline {
        WireInline::Text {
            node_id,
            span,
            text_span,
        } => writer.object(|object| {
            object.member("kind", |writer| writer.string("text"))?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("span", |writer| encode_source_span(writer, *span))?;
            object.member("text_span", |writer| encode_text_span(writer, *text_span))
        }),
        WireInline::Emphasis {
            node_id,
            span,
            children,
        }
        | WireInline::Strong {
            node_id,
            span,
            children,
        } => writer.object(|object| {
            object.member("children", |writer| writer.array(children, encode_inline))?;
            object.member("kind", |writer| {
                writer.string(if matches!(inline, WireInline::Emphasis { .. }) {
                    "emphasis"
                } else {
                    "strong"
                })
            })?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("span", |writer| encode_source_span(writer, *span))
        }),
        WireInline::Link {
            node_id,
            span,
            target,
            children,
        } => writer.object(|object| {
            object.member("children", |writer| writer.array(children, encode_inline))?;
            object.member("kind", |writer| writer.string("link"))?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("span", |writer| encode_source_span(writer, *span))?;
            object.member("target", |writer| encode_link_target(writer, target))
        }),
        WireInline::Anchor {
            node_id,
            span,
            anchor_id,
        } => writer.object(|object| {
            object.member("anchor_id", |writer| writer.string(anchor_id))?;
            object.member("kind", |writer| writer.string("anchor"))?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("span", |writer| encode_source_span(writer, *span))
        }),
        WireInline::Reference {
            node_id,
            span,
            target,
            format,
        } => writer.object(|object| {
            object.member("format", |writer| writer.string(format.as_str()))?;
            object.member("kind", |writer| writer.string("reference"))?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("span", |writer| encode_source_span(writer, *span))?;
            object.member("target", |writer| writer.string(target))
        }),
        WireInline::FootnoteReference {
            node_id,
            span,
            footnote_id,
        } => writer.object(|object| {
            object.member("footnote_id", |writer| writer.string(footnote_id))?;
            object.member("kind", |writer| writer.string("footnote_reference"))?;
            object.member("node_id", |writer| writer.u32(*node_id))?;
            object.member("span", |writer| encode_source_span(writer, *span))
        }),
        WireInline::SoftBreak { node_id, span } | WireInline::HardBreak { node_id, span } => writer
            .object(|object| {
                object.member("kind", |writer| {
                    writer.string(if matches!(inline, WireInline::SoftBreak { .. }) {
                        "soft_break"
                    } else {
                        "hard_break"
                    })
                })?;
                object.member("node_id", |writer| writer.u32(*node_id))?;
                object.member("span", |writer| encode_source_span(writer, *span))
            }),
    }
}

fn encode_link_target<W: Write>(
    writer: &mut JcsWriter<W>,
    target: &WireLinkTarget,
) -> Result<(), JcsEncodeError> {
    match target {
        WireLinkTarget::Internal { anchor_id } => writer.object(|object| {
            object.member("anchor_id", |writer| writer.string(anchor_id))?;
            object.member("kind", |writer| writer.string("internal"))
        }),
        WireLinkTarget::Uri { uri } => writer.object(|object| {
            object.member("kind", |writer| writer.string("uri"))?;
            object.member("uri", |writer| writer.string(uri))
        }),
    }
}

fn encode_list_item<W: Write>(
    writer: &mut JcsWriter<W>,
    item: &WireListItem,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("blocks", |writer| writer.array(&item.blocks, encode_block))?;
        object.member("node_id", |writer| writer.u32(item.node_id))?;
        object.member("span", |writer| encode_source_span(writer, item.span))
    })
}

fn encode_table_column<W: Write>(
    writer: &mut JcsWriter<W>,
    column: &WireTableColumn,
) -> Result<(), JcsEncodeError> {
    match column {
        WireTableColumn::Fixed { width } => writer.object(|object| {
            object.member("kind", |writer| writer.string("fixed"))?;
            object.member("width", |writer| {
                writer.positive_i64("document.blocks[].columns[].width", *width)
            })
        }),
        WireTableColumn::Fraction { weight } => writer.object(|object| {
            object.member("kind", |writer| writer.string("fraction"))?;
            object.member("weight", |writer| {
                writer.u16("document.blocks[].columns[].weight", *weight)
            })
        }),
    }
}

fn encode_table_row<W: Write>(
    writer: &mut JcsWriter<W>,
    row: &WireTableRow,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("cells", |writer| {
            writer.array(&row.cells, encode_table_cell)
        })?;
        object.member("node_id", |writer| writer.u32(row.node_id))?;
        object.member("span", |writer| encode_source_span(writer, row.span))
    })
}

fn encode_table_cell<W: Write>(
    writer: &mut JcsWriter<W>,
    cell: &WireTableCell,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("blocks", |writer| writer.array(&cell.blocks, encode_block))?;
        object.member("colspan", |writer| {
            writer.u16("document.blocks[].rows[].cells[].colspan", cell.colspan)
        })?;
        object.member("node_id", |writer| writer.u32(cell.node_id))?;
        object.member("rowspan", |writer| {
            writer.u16("document.blocks[].rows[].cells[].rowspan", cell.rowspan)
        })?;
        object.member("span", |writer| encode_source_span(writer, cell.span))
    })
}

fn encode_footnote<W: Write>(
    writer: &mut JcsWriter<W>,
    footnote: &WireFootnote,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("blocks", |writer| {
            writer.array(&footnote.blocks, encode_block)
        })?;
        object.member("footnote_id", |writer| writer.string(&footnote.footnote_id))?;
        object.member("node_id", |writer| writer.u32(footnote.node_id))?;
        object.member("span", |writer| encode_source_span(writer, footnote.span))
    })
}

fn encode_style_sheet<W: Write>(
    writer: &mut JcsWriter<W>,
    style_sheet: &WireStyleSheet,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("rules", |writer| {
            writer.array(&style_sheet.rules, encode_style_rule)
        })
    })
}

fn encode_style_rule<W: Write>(
    writer: &mut JcsWriter<W>,
    rule: &WireStyleRule,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("declarations", |writer| {
            writer.array(&rule.declarations, encode_declaration)
        })?;
        object.member("extends", |writer| {
            encode_optional_string(writer, rule.extends.as_deref())
        })?;
        object.member("selector", |writer| writer.string(&rule.selector))?;
        object.member("source_order", |writer| writer.u32(rule.source_order))?;
        object.member("style_id", |writer| writer.string(&rule.style_id))
    })
}

fn encode_declaration<W: Write>(
    writer: &mut JcsWriter<W>,
    declaration: &WireDeclaration,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("important", |writer| writer.boolean(declaration.important))?;
        object.member("name", |writer| writer.string(declaration.name.as_str()))?;
        object.member("value", |writer| {
            encode_style_value(writer, &declaration.value)
        })
    })
}

fn encode_style_value<W: Write>(
    writer: &mut JcsWriter<W>,
    value: &WireStyleValue,
) -> Result<(), JcsEncodeError> {
    match value {
        WireStyleValue::Keyword { value } => encode_kind_string_value(writer, "keyword", value),
        WireStyleValue::String { value } => encode_kind_string_value(writer, "string", value),
        WireStyleValue::Integer { value } => writer.object(|object| {
            object.member("kind", |writer| writer.string("integer"))?;
            object.member("value", |writer| {
                writer.safe_i64("style_sheet.rules[].declarations[].value", *value)
            })
        }),
        WireStyleValue::Length { value } => writer.object(|object| {
            object.member("kind", |writer| writer.string("length"))?;
            object.member("value", |writer| {
                writer.safe_i64("style_sheet.rules[].declarations[].value", *value)
            })
        }),
        WireStyleValue::Boolean { value } => writer.object(|object| {
            object.member("kind", |writer| writer.string("boolean"))?;
            object.member("value", |writer| writer.boolean(*value))
        }),
        WireStyleValue::FontFamilyList { families } => writer.object(|object| {
            object.member("families", |writer| encode_strings(writer, families))?;
            object.member("kind", |writer| writer.string("font_family_list"))
        }),
        WireStyleValue::Ratio {
            numerator,
            denominator,
        } => writer.object(|object| {
            object.member("denominator", |writer| {
                writer.positive_safe_u64(
                    "style_sheet.rules[].declarations[].value.denominator",
                    *denominator,
                )
            })?;
            object.member("kind", |writer| writer.string("ratio"))?;
            object.member("numerator", |writer| {
                writer.safe_i64(
                    "style_sheet.rules[].declarations[].value.numerator",
                    *numerator,
                )
            })
        }),
    }
}

fn encode_kind_string_value<W: Write>(
    writer: &mut JcsWriter<W>,
    kind: &str,
    value: &str,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("kind", |writer| writer.string(kind))?;
        object.member("value", |writer| writer.string(value))
    })
}

fn encode_page_masters<W: Write>(
    writer: &mut JcsWriter<W>,
    page_masters: &WirePageMasterSet,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("default_master_id", |writer| {
            writer.string(&page_masters.default_master_id)
        })?;
        object.member("masters", |writer| {
            writer.array(&page_masters.masters, encode_page_master)
        })?;
        object.member("selection_rules", |writer| {
            writer.array(&page_masters.selection_rules, encode_page_master_rule)
        })
    })
}

fn encode_page_master<W: Write>(
    writer: &mut JcsWriter<W>,
    master: &WirePageMaster,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("body", |writer| encode_rect(writer, master.body))?;
        object.member("footer", |writer| {
            encode_optional_rect(writer, master.footer)
        })?;
        object.member("footnote", |writer| {
            encode_optional_rect(writer, master.footnote)
        })?;
        object.member("header", |writer| {
            encode_optional_rect(writer, master.header)
        })?;
        object.member("height", |writer| {
            writer.positive_i64("page_masters.masters[].height", master.height)
        })?;
        object.member("master_id", |writer| writer.string(&master.master_id))?;
        object.member("width", |writer| {
            writer.positive_i64("page_masters.masters[].width", master.width)
        })
    })
}

fn encode_page_master_rule<W: Write>(
    writer: &mut JcsWriter<W>,
    rule: &WirePageMasterRule,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("first", |writer| encode_optional_bool(writer, rule.first))?;
        object.member("master_id", |writer| writer.string(&rule.master_id))?;
        object.member("named_page", |writer| {
            encode_optional_string(writer, rule.named_page.as_deref())
        })?;
        object.member("parity", |writer| writer.string(rule.parity.as_str()))?;
        object.member("source_order", |writer| writer.u32(rule.source_order))
    })
}

fn encode_resources<W: Write>(
    writer: &mut JcsWriter<W>,
    resources: &WireResourceCatalog,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("font_faces", |writer| {
            writer.array(&resources.font_faces, encode_font_face)
        })?;
        object.member("images", |writer| {
            writer.array(&resources.images, encode_image)
        })
    })
}

fn encode_font_face<W: Write>(
    writer: &mut JcsWriter<W>,
    font: &WireFontFace,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("expected_sha256", |writer| {
            encode_optional_hash(writer, font.expected_sha256)
        })?;
        object.member("face_index", |writer| writer.u32(font.face_index))?;
        object.member("family", |writer| writer.string(&font.family))?;
        object.member("font_face_id", |writer| writer.u32(font.font_face_id))?;
        object.member("uri", |writer| writer.string(&font.uri))
    })
}

fn encode_image<W: Write>(
    writer: &mut JcsWriter<W>,
    image: &WireImage,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("expected_sha256", |writer| {
            encode_optional_hash(writer, image.expected_sha256)
        })?;
        object.member("image_id", |writer| writer.u32(image.image_id))?;
        object.member("uri", |writer| writer.string(&image.uri))
    })
}

fn encode_source_span<W: Write>(
    writer: &mut JcsWriter<W>,
    span: WireSourceSpan,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("end_byte", |writer| writer.u32(span.end_byte))?;
        object.member("source_id", |writer| writer.u32(span.source_id))?;
        object.member("start_byte", |writer| writer.u32(span.start_byte))
    })
}

fn encode_optional_source_span<W: Write>(
    writer: &mut JcsWriter<W>,
    span: Option<WireSourceSpan>,
) -> Result<(), JcsEncodeError> {
    match span {
        Some(span) => encode_source_span(writer, span),
        None => writer.null(),
    }
}

fn encode_text_span<W: Write>(
    writer: &mut JcsWriter<W>,
    span: WireTextSpan,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("end_byte", |writer| writer.u32(span.end_byte))?;
        object.member("start_byte", |writer| writer.u32(span.start_byte))?;
        object.member("text_id", |writer| writer.u32(span.text_id))
    })
}

fn encode_byte_range<W: Write>(
    writer: &mut JcsWriter<W>,
    range: WireByteRange,
) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("end_byte", |writer| writer.u32(range.end_byte))?;
        object.member("start_byte", |writer| writer.u32(range.start_byte))
    })
}

fn encode_rect<W: Write>(writer: &mut JcsWriter<W>, rect: WireRect) -> Result<(), JcsEncodeError> {
    writer.object(|object| {
        object.member("height", |writer| {
            writer.positive_i64("page_masters.masters[].frame.height", rect.height)
        })?;
        object.member("width", |writer| {
            writer.positive_i64("page_masters.masters[].frame.width", rect.width)
        })?;
        object.member("x", |writer| {
            writer.safe_i64("page_masters.masters[].frame.x", rect.x)
        })?;
        object.member("y", |writer| {
            writer.safe_i64("page_masters.masters[].frame.y", rect.y)
        })
    })
}

fn encode_optional_rect<W: Write>(
    writer: &mut JcsWriter<W>,
    rect: Option<WireRect>,
) -> Result<(), JcsEncodeError> {
    match rect {
        Some(rect) => encode_rect(writer, rect),
        None => writer.null(),
    }
}

fn encode_strings<W: Write>(
    writer: &mut JcsWriter<W>,
    values: &[String],
) -> Result<(), JcsEncodeError> {
    writer.array(values, |writer, value| writer.string(value))
}

fn encode_optional_string<W: Write>(
    writer: &mut JcsWriter<W>,
    value: Option<&str>,
) -> Result<(), JcsEncodeError> {
    match value {
        Some(value) => writer.string(value),
        None => writer.null(),
    }
}

fn encode_optional_bool<W: Write>(
    writer: &mut JcsWriter<W>,
    value: Option<bool>,
) -> Result<(), JcsEncodeError> {
    match value {
        Some(value) => writer.boolean(value),
        None => writer.null(),
    }
}

fn encode_optional_u32<W: Write>(
    writer: &mut JcsWriter<W>,
    value: Option<u32>,
) -> Result<(), JcsEncodeError> {
    match value {
        Some(value) => writer.u32(value),
        None => writer.null(),
    }
}

fn encode_hash<W: Write>(writer: &mut JcsWriter<W>, hash: [u8; 32]) -> Result<(), JcsEncodeError> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = [0u8; 64];
    for (index, byte) in hash.into_iter().enumerate() {
        encoded[index * 2] = HEX[usize::from(byte >> 4)];
        encoded[index * 2 + 1] = HEX[usize::from(byte & 0x0f)];
    }
    let encoded =
        std::str::from_utf8(&encoded).map_err(|_| JcsEncodeError::NonUtf8EncoderOutput)?;
    writer.string(encoded)
}

fn encode_optional_hash<W: Write>(
    writer: &mut JcsWriter<W>,
    hash: Option<[u8; 32]>,
) -> Result<(), JcsEncodeError> {
    match hash {
        Some(hash) => encode_hash(writer, hash),
        None => writer.null(),
    }
}

#[derive(Clone, Debug)]
struct StreamingSha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    bytes: u64,
}

impl StreamingSha256 {
    const fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            buffer_len: 0,
            bytes: 0,
        }
    }

    fn update(&mut self, mut input: &[u8]) {
        self.bytes = self
            .bytes
            .checked_add(u64::try_from(input.len()).expect("slice length fits u64"))
            .expect("JCS hash input length is bounded below u64::MAX");
        if self.buffer_len > 0 {
            let needed = 64 - self.buffer_len;
            let consumed = needed.min(input.len());
            self.buffer[self.buffer_len..self.buffer_len + consumed]
                .copy_from_slice(&input[..consumed]);
            self.buffer_len += consumed;
            input = &input[consumed..];
            if self.buffer_len == 64 {
                compress_sha256(&mut self.state, &self.buffer);
                self.buffer_len = 0;
            } else {
                return;
            }
        }
        while input.len() >= 64 {
            let block: &[u8; 64] = input[..64].try_into().expect("block has exact length");
            compress_sha256(&mut self.state, block);
            input = &input[64..];
        }
        self.buffer[..input.len()].copy_from_slice(input);
        self.buffer_len = input.len();
    }

    fn finish(mut self) -> [u8; 32] {
        let bit_length = self.bytes.checked_mul(8).expect("JCS input is JSON-safe");
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;
        if self.buffer_len > 56 {
            self.buffer[self.buffer_len..].fill(0);
            compress_sha256(&mut self.state, &self.buffer);
            self.buffer = [0; 64];
            self.buffer_len = 0;
        }
        self.buffer[self.buffer_len..56].fill(0);
        self.buffer[56..].copy_from_slice(&bit_length.to_be_bytes());
        compress_sha256(&mut self.state, &self.buffer);
        let mut output = [0u8; 32];
        for (chunk, word) in output.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        output
    }
}

fn compress_sha256(state: &mut [u32; 8], block: &[u8; 64]) {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut words = [0u32; 64];
    for (index, word) in words[..16].iter_mut().enumerate() {
        let start = index * 4;
        *word = u32::from_be_bytes(
            block[start..start + 4]
                .try_into()
                .expect("word is four bytes"),
        );
    }
    for index in 16..64 {
        let s0 = words[index - 15].rotate_right(7)
            ^ words[index - 15].rotate_right(18)
            ^ (words[index - 15] >> 3);
        let s1 = words[index - 2].rotate_right(17)
            ^ words[index - 2].rotate_right(19)
            ^ (words[index - 2] >> 10);
        words[index] = words[index - 16]
            .wrapping_add(s0)
            .wrapping_add(words[index - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for index in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choice = (e & f) ^ ((!e) & g);
        let first = h
            .wrapping_add(s1)
            .wrapping_add(choice)
            .wrapping_add(K[index])
            .wrapping_add(words[index]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let second = s0.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(first);
        d = c;
        c = b;
        b = a;
        a = first.wrapping_add(second);
    }
    for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *slot = slot.wrapping_add(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{sha256, DocumentPackageContractId};

    fn empty_package() -> WireDocumentPackage {
        WireDocumentPackage {
            contract: DocumentPackageContractId::CURRENT,
            coordinate_unit: WireCoordinateUnit::PdfPoint1_65536,
            sources: vec![],
            text_buffers: vec![],
            document: WireDocument {
                node_id: 0,
                blocks: vec![],
                footnotes: vec![],
            },
            style_sheet: WireStyleSheet { rules: vec![] },
            page_masters: WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![WirePageMaster {
                    master_id: "default".to_owned(),
                    width: 100,
                    height: 100,
                    body: WireRect {
                        x: 0,
                        y: 0,
                        width: 100,
                        height: 100,
                    },
                    header: None,
                    footer: None,
                    footnote: None,
                }],
                selection_rules: vec![],
            },
            resources: WireResourceCatalog {
                font_faces: vec![],
                images: vec![],
            },
        }
    }

    #[test]
    fn count_hash_path_matches_materialized_canonical_bytes() {
        let package = empty_package();
        let encoder = DocumentPackageEncoder::default();
        let stats = encoder.analyze(&package).unwrap();
        let bytes = encoder.to_jcs_vec(&package).unwrap();
        assert_eq!(stats.bytes(), bytes.len() as u64);
        assert_eq!(stats.sha256(), sha256(&bytes));
        assert!(bytes.starts_with(b"{\"contract\":\"typaxis.contract/1.1\""));
    }

    #[test]
    fn exact_limit_succeeds_and_preflight_failure_writes_nothing() {
        let package = empty_package();
        let bytes = DocumentPackageEncoder::default()
            .analyze(&package)
            .unwrap()
            .bytes();
        let exact = DocumentPackageEncoder::new(bytes).unwrap();
        let mut exact_output = Vec::new();
        exact
            .write_preflighted(&package, &mut exact_output)
            .unwrap();
        assert_eq!(exact_output.len() as u64, bytes);

        let too_small = DocumentPackageEncoder::new(bytes - 1).unwrap();
        let mut untouched = Vec::new();
        assert!(matches!(
            too_small.write_preflighted(&package, &mut untouched),
            Err(JcsEncodeError::ByteLimitExceeded { .. })
        ));
        assert!(untouched.is_empty());
    }

    #[test]
    fn strings_use_minimal_jcs_escaping() {
        let mut output = Vec::new();
        let mut writer = JcsWriter::new(&mut output, 1024);
        writer.string("a/\"\\\u{0000}\u{0008}\n 雪").unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "\"a/\\\"\\\\\\u0000\\b\\n 雪\""
        );
    }

    #[test]
    fn non_ascii_member_order_uses_utf16_code_units() {
        let mut entries = [
            ("\u{e000}".to_owned(), "bmp".to_owned()),
            ("\u{1f600}".to_owned(), "surrogate".to_owned()),
        ];
        entries.sort_by(|left, right| utf16_cmp(&left.0, &right.0));
        let mut output = Vec::new();
        let mut writer = JcsWriter::new(&mut output, 1024);
        writer
            .object(|object| {
                for (name, value) in &entries {
                    object.member(name, |writer| writer.string(value))?;
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"😀\":\"surrogate\",\"\":\"bmp\"}"
        );
        assert_eq!(
            "\u{e000}".as_bytes().cmp("\u{1f600}".as_bytes()),
            Ordering::Less
        );
    }

    #[test]
    fn exact_integer_domains_are_checked_before_output() {
        let mut package = empty_package();
        package.page_masters.masters[0].width = JSON_SAFE_INTEGER_MAX + 1;
        let mut output = Vec::new();
        assert!(matches!(
            DocumentPackageEncoder::default().write_preflighted(&package, &mut output),
            Err(JcsEncodeError::IntegerOutOfRange { .. })
        ));
        assert!(output.is_empty());
    }

    #[test]
    fn streaming_sha256_matches_boundary_vectors() {
        for length in [0usize, 1, 55, 56, 63, 64, 65, 127, 128, 129] {
            let input: Vec<u8> = (0..length).map(|index| index as u8).collect();
            let mut hash = StreamingSha256::new();
            for chunk in input.chunks(7) {
                hash.update(chunk);
            }
            assert_eq!(hash.finish(), sha256(&input));
        }
    }
}
