//! Contract-1.3 carrier originally introduced by the MI3 advanced-pagination
//! slices and now used internally by the public strict decoder. This bridge
//! removes only the additions adopted by ADR-0031, decodes the remaining
//! frozen 1.2 shape through the shared strict decoder, and retains every
//! removed fact in typed DTOs.

use crate::{
    DecodedDocumentPackage, DocumentPackageDecodeError, DocumentPackageDecodePolicy,
    JsonPreflightError, StrictDocumentPackageDecoder, StrictJsonPreflight, WireAdvancedPageMaster,
    WireAdvancedPageMasterSet, WireColumnBalance, WireColumnFill, WireColumnLayout,
    WireFigurePlacement, WireFigurePlacementRecord, WirePageProgression, WirePageRegion,
    WirePageRegionBlock, WirePageRegionInline, WirePageWritingMode, WireRect, WireSourceSpan,
    WireTextSpan,
};
use serde::de::{self, Deserialize, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;
use std::fmt;
use typaxis_core::{push_jcs_string, sha256, JSON_SAFE_INTEGER_MAX};

pub const STAGING_ADVANCED_DOCUMENT_PACKAGE_CONTRACT: &str = "typaxis.contract/1.3";

#[derive(Debug)]
pub enum StagingAdvancedDecodeError {
    Preflight(JsonPreflightError),
    Json(serde_json::Error),
    Shape(&'static str),
    Contract,
    Limit,
    Base(DocumentPackageDecodeError),
}

impl fmt::Display for StagingAdvancedDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preflight(error) => error.fmt(formatter),
            Self::Json(error) => {
                write!(formatter, "invalid advanced DocumentPackage JSON: {error}")
            }
            Self::Shape(message) => write!(formatter, "invalid contract-1.3 shape: {message}"),
            Self::Contract => formatter.write_str("expected typaxis.contract/1.3"),
            Self::Limit => formatter.write_str("contract-1.3 package exceeds a resource limit"),
            Self::Base(error) => write!(
                formatter,
                "invalid inherited DocumentPackage shape: {error}"
            ),
        }
    }
}

impl std::error::Error for StagingAdvancedDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preflight(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Base(error) => Some(error),
            Self::Shape(_) | Self::Contract | Self::Limit => None,
        }
    }
}

/// Decoder-issued internal receipt. The exact admitted 1.3 bytes and their
/// canonical typed JSON are bound independently from the inherited 1.2
/// carrier so a caller cannot substitute a neutral package after preflight.
pub struct DecodedStagingAdvancedDocumentPackage {
    base: DecodedDocumentPackage,
    page_masters: WireAdvancedPageMasterSet,
    figure_placements: Vec<WireFigurePlacementRecord>,
    raw_sha256: [u8; 32],
    canonical_jcs_sha256: [u8; 32],
}

impl fmt::Debug for DecodedStagingAdvancedDocumentPackage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DecodedStagingAdvancedDocumentPackage")
            .field("contract", &STAGING_ADVANCED_DOCUMENT_PACKAGE_CONTRACT)
            .field("masters", &self.page_masters.masters.len())
            .field("figures", &self.figure_placements.len())
            .finish_non_exhaustive()
    }
}

impl DecodedStagingAdvancedDocumentPackage {
    pub const fn page_masters(&self) -> &WireAdvancedPageMasterSet {
        &self.page_masters
    }

    pub fn figure_placements(&self) -> &[WireFigurePlacementRecord] {
        &self.figure_placements
    }

    pub const fn raw_sha256(&self) -> [u8; 32] {
        self.raw_sha256
    }

    pub const fn canonical_jcs_sha256(&self) -> [u8; 32] {
        self.canonical_jcs_sha256
    }

    pub fn into_parts(
        self,
    ) -> (
        crate::WireDocumentPackage,
        WireAdvancedPageMasterSet,
        Vec<WireFigurePlacementRecord>,
        [u8; 32],
        [u8; 32],
        crate::JsonLocationIndex,
    ) {
        // Consume the inherited decoder receipt here. Returning it would let a
        // caller redirect the stripped 1.2 carrier into the public parser and
        // silently downgrade a non-neutral 1.3 package.
        let (wire, _base_raw, _base_canonical, locations) = self.base.into_parts();
        (
            wire,
            self.page_masters,
            self.figure_placements,
            self.raw_sha256,
            self.canonical_jcs_sha256,
            locations,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StagingAdvancedDocumentPackageDecoder;

impl StagingAdvancedDocumentPackageDecoder {
    pub const fn new() -> Self {
        Self
    }

    pub fn decode(
        &self,
        input: &[u8],
        policy: &DocumentPackageDecodePolicy<'_>,
    ) -> Result<DecodedStagingAdvancedDocumentPackage, StagingAdvancedDecodeError> {
        StrictJsonPreflight::new(policy.preflight_limits())
            .check(input)
            .map_err(StagingAdvancedDecodeError::Preflight)?;

        let mut deserializer = serde_json::Deserializer::from_slice(input);
        deserializer.disable_recursion_limit();
        let stacker = serde_stacker::Deserializer::new(&mut deserializer);
        let mut root = NoDuplicateValue::deserialize(stacker)
            .map_err(StagingAdvancedDecodeError::Json)?
            .0;
        deserializer
            .end()
            .map_err(StagingAdvancedDecodeError::Json)?;

        let canonical = canonicalize_value(&root, input.len())?;
        let canonical_jcs_sha256 = sha256(canonical.as_bytes());
        let raw_sha256 = sha256(input);

        let root_object = object_mut(&mut root, "root must be an object")?;
        let contract = required_remove(root_object, "contract")?;
        if contract.as_str() != Some(STAGING_ADVANCED_DOCUMENT_PACKAGE_CONTRACT) {
            return Err(StagingAdvancedDecodeError::Contract);
        }
        root_object.insert(
            "contract".to_owned(),
            Value::String("typaxis.contract/1.2".to_owned()),
        );

        let document = root_object
            .get("document")
            .ok_or(StagingAdvancedDecodeError::Shape("document is required"))?;
        let maximum_nodes = policy.resource_limits().get().max_ast_nodes;
        let (base_nodes, advanced_node_limit) = match count_raw_base_nodes(document, maximum_nodes)
        {
            Ok(count) => (count, maximum_nodes),
            Err(StagingAdvancedDecodeError::Limit) => {
                // The inherited strict decoder below owns base-node limit
                // diagnostics and their exact JSON pointers. Finish the
                // structural projection without applying the limit a second
                // time so that decoder can issue its typed error.
                (count_raw_base_nodes(document, u64::MAX)?, u64::MAX)
            }
            Err(error) => return Err(error),
        };
        let mut total_nodes = base_nodes;

        let page_masters_value =
            root_object
                .get_mut("page_masters")
                .ok_or(StagingAdvancedDecodeError::Shape(
                    "page_masters is required",
                ))?;
        let page_masters_object = object_mut(page_masters_value, "page_masters must be an object")?;
        let progression = match required_remove(page_masters_object, "page_progression")?.as_str() {
            Some("ltr") => WirePageProgression::LeftToRight,
            _ => {
                return Err(StagingAdvancedDecodeError::Shape(
                    "unsupported page_progression",
                ))
            }
        };
        let writing_mode = match required_remove(page_masters_object, "writing_mode")?.as_str() {
            Some("horizontal-tb") => WirePageWritingMode::HorizontalTopToBottom,
            _ => {
                return Err(StagingAdvancedDecodeError::Shape(
                    "unsupported writing_mode",
                ))
            }
        };
        let selection_rule_count = page_masters_object
            .get("selection_rules")
            .and_then(Value::as_array)
            .ok_or(StagingAdvancedDecodeError::Shape(
                "page_masters.selection_rules must be an array",
            ))?
            .len();
        let masters = page_masters_object
            .get_mut("masters")
            .and_then(Value::as_array_mut)
            .ok_or(StagingAdvancedDecodeError::Shape(
                "page_masters.masters must be an array",
            ))?;
        let master_rule_count = masters
            .len()
            .checked_add(selection_rule_count)
            .and_then(|value| u64::try_from(value).ok())
            .ok_or(StagingAdvancedDecodeError::Limit)?;
        let maximum_style_rules = policy.resource_limits().get().max_style_rules;
        if u64::try_from(masters.len()).unwrap_or(u64::MAX) <= maximum_style_rules
            && master_rule_count > maximum_style_rules
        {
            return Err(StagingAdvancedDecodeError::Limit);
        }
        let mut advanced_masters = Vec::new();
        advanced_masters
            .try_reserve_exact(masters.len())
            .map_err(|_| StagingAdvancedDecodeError::Limit)?;
        for master in masters {
            let object = object_mut(master, "page master must be an object")?;
            let master_id = object
                .get("master_id")
                .and_then(Value::as_str)
                .ok_or(StagingAdvancedDecodeError::Shape(
                    "master_id must be a string",
                ))?
                .to_owned();
            let trim = parse_rect(required_remove(object, "trim")?)?;
            let header_content = parse_region(
                required_remove(object, "header_content")?,
                &mut total_nodes,
                advanced_node_limit,
            )?;
            let footer_content = parse_region(
                required_remove(object, "footer_content")?,
                &mut total_nodes,
                advanced_node_limit,
            )?;
            let column_layout = parse_column_layout(
                required_remove(object, "column_layout")?,
                &mut total_nodes,
                advanced_node_limit,
            )?;
            advanced_masters.push(WireAdvancedPageMaster {
                master_id,
                trim,
                header_content,
                footer_content,
                column_layout,
            });
        }

        let document = root_object
            .get_mut("document")
            .and_then(Value::as_object_mut)
            .ok_or(StagingAdvancedDecodeError::Shape(
                "document must be an object",
            ))?;
        let mut figure_placements = Vec::new();
        let blocks = document
            .get_mut("blocks")
            .and_then(Value::as_array_mut)
            .ok_or(StagingAdvancedDecodeError::Shape(
                "document.blocks must be an array",
            ))?;
        collect_figure_placements(blocks, &mut figure_placements)?;
        let footnotes = document
            .get_mut("footnotes")
            .and_then(Value::as_array_mut)
            .ok_or(StagingAdvancedDecodeError::Shape(
                "document.footnotes must be an array",
            ))?;
        for footnote in footnotes {
            let blocks = footnote
                .as_object_mut()
                .and_then(|value| value.get_mut("blocks"))
                .and_then(Value::as_array_mut)
                .ok_or(StagingAdvancedDecodeError::Shape(
                    "footnote blocks must be an array",
                ))?;
            collect_figure_placements(blocks, &mut figure_placements)?;
        }
        figure_placements.sort_by_key(|record| record.node_id);
        if figure_placements
            .windows(2)
            .any(|pair| pair[0].node_id == pair[1].node_id)
        {
            return Err(StagingAdvancedDecodeError::Shape(
                "duplicate Figure node_id",
            ));
        }

        let inherited = serde_json::to_vec(&root).map_err(StagingAdvancedDecodeError::Json)?;
        let base = StrictDocumentPackageDecoder::new()
            .decode(&inherited, policy)
            .map_err(StagingAdvancedDecodeError::Base)?;

        if count_base_nodes(&base.wire().document)? != base_nodes {
            return Err(StagingAdvancedDecodeError::Limit);
        }

        Ok(DecodedStagingAdvancedDocumentPackage {
            base,
            page_masters: WireAdvancedPageMasterSet {
                page_progression: progression,
                writing_mode,
                masters: advanced_masters,
            },
            figure_placements,
            raw_sha256,
            canonical_jcs_sha256,
        })
    }
}

struct NoDuplicateValue(Value);

impl<'de> Deserialize<'de> for NoDuplicateValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = NoDuplicateValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value without duplicate object members")
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::Bool(value)))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::Number(Number::from(value))))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::Number(Number::from(value))))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Number::from_f64(value)
                    .map(Value::Number)
                    .map(NoDuplicateValue)
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::String(value.to_owned())))
            }

            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::String(value)))
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::Null))
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(Value::Null))
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<NoDuplicateValue>()? {
                    values.push(value.0);
                }
                Ok(NoDuplicateValue(Value::Array(values)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    let value = map.next_value::<NoDuplicateValue>()?.0;
                    if values.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate JSON object member"));
                    }
                }
                Ok(NoDuplicateValue(Value::Object(
                    values.into_iter().collect::<Map<String, Value>>(),
                )))
            }
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}

fn canonicalize_value(
    value: &Value,
    capacity: usize,
) -> Result<String, StagingAdvancedDecodeError> {
    fn append(value: &Value, output: &mut String) -> Result<(), StagingAdvancedDecodeError> {
        match value {
            Value::Null => output.push_str("null"),
            Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    output.push_str(&value.to_string());
                } else if let Some(value) = value.as_u64() {
                    output.push_str(&value.to_string());
                } else {
                    // Typed base/advanced field decoders below own integer
                    // grammar and exact location diagnostics. This temporary
                    // representation is never issued as a successful receipt.
                    output.push_str(&value.to_string());
                }
            }
            Value::String(value) => push_jcs_string(output, value),
            Value::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    append(value, output)?;
                }
                output.push(']');
            }
            Value::Object(values) => {
                let mut members = values.iter().collect::<Vec<_>>();
                members
                    .sort_by(|(left, _), (right, _)| left.encode_utf16().cmp(right.encode_utf16()));
                output.push('{');
                for (index, (key, value)) in members.into_iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    push_jcs_string(output, key);
                    output.push(':');
                    append(value, output)?;
                }
                output.push('}');
            }
        }
        Ok(())
    }

    let mut output = String::new();
    output
        .try_reserve(capacity)
        .map_err(|_| StagingAdvancedDecodeError::Limit)?;
    append(value, &mut output)?;
    Ok(output)
}

fn object_mut<'a>(
    value: &'a mut Value,
    message: &'static str,
) -> Result<&'a mut Map<String, Value>, StagingAdvancedDecodeError> {
    value
        .as_object_mut()
        .ok_or(StagingAdvancedDecodeError::Shape(message))
}

fn required_remove(
    object: &mut Map<String, Value>,
    field: &'static str,
) -> Result<Value, StagingAdvancedDecodeError> {
    object
        .remove(field)
        .ok_or(StagingAdvancedDecodeError::Shape(field))
}

fn checked_i64(value: &Value) -> Result<i64, StagingAdvancedDecodeError> {
    value
        .as_i64()
        .filter(|value| value.unsigned_abs() <= JSON_SAFE_INTEGER_MAX as u64)
        .ok_or(StagingAdvancedDecodeError::Shape(
            "expected a JSON-safe integer",
        ))
}

fn parse_rect(value: Value) -> Result<WireRect, StagingAdvancedDecodeError> {
    let Value::Object(mut object) = value else {
        return Err(StagingAdvancedDecodeError::Shape("rect must be an object"));
    };
    let x = checked_i64(&required_remove(&mut object, "x")?)?;
    let y = checked_i64(&required_remove(&mut object, "y")?)?;
    let width = checked_i64(&required_remove(&mut object, "width")?)?;
    let height = checked_i64(&required_remove(&mut object, "height")?)?;
    if !object.is_empty() || width <= 0 || height <= 0 {
        return Err(StagingAdvancedDecodeError::Shape(
            "invalid closed positive rect",
        ));
    }
    Ok(WireRect {
        x,
        y,
        width,
        height,
    })
}

fn parse_span(value: Value) -> Result<WireSourceSpan, StagingAdvancedDecodeError> {
    let Value::Object(mut object) = value else {
        return Err(StagingAdvancedDecodeError::Shape("span must be an object"));
    };
    let source_id = required_remove(&mut object, "source_id")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(StagingAdvancedDecodeError::Shape("invalid source_id"))?;
    let start_byte = required_remove(&mut object, "start_byte")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(StagingAdvancedDecodeError::Shape("invalid start_byte"))?;
    let end_byte = required_remove(&mut object, "end_byte")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(StagingAdvancedDecodeError::Shape("invalid end_byte"))?;
    if !object.is_empty() {
        return Err(StagingAdvancedDecodeError::Shape("unknown span member"));
    }
    Ok(WireSourceSpan {
        source_id,
        start_byte,
        end_byte,
    })
}

fn parse_text_span(value: Value) -> Result<WireTextSpan, StagingAdvancedDecodeError> {
    let Value::Object(mut object) = value else {
        return Err(StagingAdvancedDecodeError::Shape(
            "text_span must be an object",
        ));
    };
    let text_id = required_remove(&mut object, "text_id")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(StagingAdvancedDecodeError::Shape("invalid text_id"))?;
    let start_byte = required_remove(&mut object, "start_byte")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(StagingAdvancedDecodeError::Shape("invalid text start"))?;
    let end_byte = required_remove(&mut object, "end_byte")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(StagingAdvancedDecodeError::Shape("invalid text end"))?;
    if !object.is_empty() {
        return Err(StagingAdvancedDecodeError::Shape(
            "unknown text_span member",
        ));
    }
    Ok(WireTextSpan {
        text_id,
        start_byte,
        end_byte,
    })
}

fn parse_region(
    value: Value,
    total_nodes: &mut u64,
    max_nodes: u64,
) -> Result<Option<WirePageRegion>, StagingAdvancedDecodeError> {
    if value.is_null() {
        return Ok(None);
    }
    charge_node(total_nodes, max_nodes)?;
    let Value::Object(mut object) = value else {
        return Err(StagingAdvancedDecodeError::Shape(
            "page region must be an object or null",
        ));
    };
    let node_id = required_remove(&mut object, "node_id")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(StagingAdvancedDecodeError::Shape("invalid region node_id"))?;
    let span = parse_span(required_remove(&mut object, "span")?)?;
    let blocks = required_remove(&mut object, "blocks")?;
    let Value::Array(blocks) = blocks else {
        return Err(StagingAdvancedDecodeError::Shape(
            "region blocks must be an array",
        ));
    };
    if !object.is_empty() {
        return Err(StagingAdvancedDecodeError::Shape(
            "unknown page-region member",
        ));
    }
    let mut parsed_blocks = Vec::new();
    for block in blocks {
        let block = parse_region_block(block, total_nodes, max_nodes)?;
        parsed_blocks
            .try_reserve(1)
            .map_err(|_| StagingAdvancedDecodeError::Limit)?;
        parsed_blocks.push(block);
    }
    Ok(Some(WirePageRegion {
        node_id,
        span,
        blocks: parsed_blocks,
    }))
}

fn parse_region_block(
    value: Value,
    total_nodes: &mut u64,
    max_nodes: u64,
) -> Result<WirePageRegionBlock, StagingAdvancedDecodeError> {
    charge_node(total_nodes, max_nodes)?;
    let Value::Object(mut object) = value else {
        return Err(StagingAdvancedDecodeError::Shape(
            "region block must be an object",
        ));
    };
    let kind = required_remove(&mut object, "kind")?
        .as_str()
        .ok_or(StagingAdvancedDecodeError::Shape(
            "region block kind must be a string",
        ))?
        .to_owned();
    let node_id = required_remove(&mut object, "node_id")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(StagingAdvancedDecodeError::Shape(
            "invalid region block node_id",
        ))?;
    let span = parse_span(required_remove(&mut object, "span")?)?;
    let classes = parse_classes(required_remove(&mut object, "classes")?)?;
    let children = required_remove(&mut object, "children")?;
    let Value::Array(children) = children else {
        return Err(StagingAdvancedDecodeError::Shape(
            "region children must be an array",
        ));
    };
    let mut parsed_children = Vec::new();
    for inline in children {
        let inline = parse_region_inline(inline, total_nodes, max_nodes)?;
        parsed_children
            .try_reserve(1)
            .map_err(|_| StagingAdvancedDecodeError::Limit)?;
        parsed_children.push(inline);
    }
    match kind.as_str() {
        "paragraph" if object.is_empty() => Ok(WirePageRegionBlock::Paragraph {
            node_id,
            span,
            classes,
            children: parsed_children,
        }),
        "heading" => {
            let level = required_remove(&mut object, "level")?
                .as_u64()
                .and_then(|value| u8::try_from(value).ok())
                .filter(|level| (1..=6).contains(level))
                .ok_or(StagingAdvancedDecodeError::Shape(
                    "invalid region heading level",
                ))?;
            if !required_remove(&mut object, "anchor_id")?.is_null() || !object.is_empty() {
                return Err(StagingAdvancedDecodeError::Shape(
                    "region heading anchor_id must be null and shape must be closed",
                ));
            }
            Ok(WirePageRegionBlock::Heading {
                node_id,
                span,
                classes,
                level,
                children: parsed_children,
            })
        }
        "paragraph" => Err(StagingAdvancedDecodeError::Shape(
            "unknown paragraph member",
        )),
        _ => Err(StagingAdvancedDecodeError::Shape(
            "unsupported page-region block",
        )),
    }
}

fn parse_classes(value: Value) -> Result<Vec<String>, StagingAdvancedDecodeError> {
    let Value::Array(values) = value else {
        return Err(StagingAdvancedDecodeError::Shape(
            "classes must be an array",
        ));
    };
    let mut classes = Vec::new();
    classes
        .try_reserve_exact(values.len())
        .map_err(|_| StagingAdvancedDecodeError::Limit)?;
    for value in values {
        classes.push(
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(StagingAdvancedDecodeError::Shape("class must be a string"))?,
        );
    }
    Ok(classes)
}

fn parse_region_inline(
    value: Value,
    total_nodes: &mut u64,
    max_nodes: u64,
) -> Result<WirePageRegionInline, StagingAdvancedDecodeError> {
    charge_node(total_nodes, max_nodes)?;
    let Value::Object(mut object) = value else {
        return Err(StagingAdvancedDecodeError::Shape(
            "region inline must be an object",
        ));
    };
    let kind = required_remove(&mut object, "kind")?
        .as_str()
        .ok_or(StagingAdvancedDecodeError::Shape(
            "region inline kind must be a string",
        ))?
        .to_owned();
    let node_id = required_remove(&mut object, "node_id")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(StagingAdvancedDecodeError::Shape(
            "invalid region inline node_id",
        ))?;
    let span = parse_span(required_remove(&mut object, "span")?)?;
    match kind.as_str() {
        "text" => {
            let text_span = parse_text_span(required_remove(&mut object, "text_span")?)?;
            if !object.is_empty() {
                return Err(StagingAdvancedDecodeError::Shape(
                    "unknown region text member",
                ));
            }
            Ok(WirePageRegionInline::Text {
                node_id,
                span,
                text_span,
            })
        }
        "soft_break" if object.is_empty() => Ok(WirePageRegionInline::SoftBreak { node_id, span }),
        "hard_break" if object.is_empty() => Ok(WirePageRegionInline::HardBreak { node_id, span }),
        _ => Err(StagingAdvancedDecodeError::Shape(
            "unsupported page-region inline",
        )),
    }
}

fn parse_column_layout(
    value: Value,
    total_nodes: &mut u64,
    max_nodes: u64,
) -> Result<Option<WireColumnLayout>, StagingAdvancedDecodeError> {
    if value.is_null() {
        return Ok(None);
    }
    charge_node(total_nodes, max_nodes)?;
    let Value::Object(mut object) = value else {
        return Err(StagingAdvancedDecodeError::Shape(
            "column_layout must be object or null",
        ));
    };
    let count = required_remove(&mut object, "count")?
        .as_u64()
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| *value >= 2)
        .ok_or(StagingAdvancedDecodeError::Shape(
            "column count must be 2..=65535",
        ))?;
    let gap = checked_i64(&required_remove(&mut object, "gap")?)?;
    if gap < 0 {
        return Err(StagingAdvancedDecodeError::Shape(
            "column gap must be nonnegative",
        ));
    }
    let fill = match required_remove(&mut object, "fill")?.as_str() {
        Some("sequential") => WireColumnFill::Sequential,
        _ => return Err(StagingAdvancedDecodeError::Shape("unsupported column fill")),
    };
    let balance = match required_remove(&mut object, "balance")?.as_str() {
        Some("none") => WireColumnBalance::None,
        Some("last_page") => WireColumnBalance::LastPage,
        _ => {
            return Err(StagingAdvancedDecodeError::Shape(
                "unsupported column balance",
            ))
        }
    };
    if !object.is_empty() {
        return Err(StagingAdvancedDecodeError::Shape(
            "unknown column_layout member",
        ));
    }
    Ok(Some(WireColumnLayout {
        count,
        gap,
        fill,
        balance,
    }))
}

fn collect_figure_placements(
    blocks: &mut [Value],
    output: &mut Vec<WireFigurePlacementRecord>,
) -> Result<(), StagingAdvancedDecodeError> {
    for block in blocks {
        let object = block
            .as_object_mut()
            .ok_or(StagingAdvancedDecodeError::Shape("block must be an object"))?;
        let kind =
            object
                .get("kind")
                .and_then(Value::as_str)
                .ok_or(StagingAdvancedDecodeError::Shape(
                    "block kind must be a string",
                ))?;
        match kind {
            "figure" => {
                let node_id = object
                    .get("node_id")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or(StagingAdvancedDecodeError::Shape("invalid Figure node_id"))?;
                let placement = match required_remove(object, "placement")?.as_str() {
                    Some("block") => WireFigurePlacement::Block,
                    Some("float") => WireFigurePlacement::Float,
                    _ => {
                        return Err(StagingAdvancedDecodeError::Shape(
                            "unsupported Figure placement",
                        ))
                    }
                };
                output
                    .try_reserve(1)
                    .map_err(|_| StagingAdvancedDecodeError::Limit)?;
                output.push(WireFigurePlacementRecord { node_id, placement });
                let caption = object
                    .get_mut("caption")
                    .and_then(Value::as_array_mut)
                    .ok_or(StagingAdvancedDecodeError::Shape(
                        "Figure caption must be an array",
                    ))?;
                collect_figure_placements(caption, output)?;
            }
            "list" => {
                let items = object
                    .get_mut("items")
                    .and_then(Value::as_array_mut)
                    .ok_or(StagingAdvancedDecodeError::Shape(
                        "list items must be an array",
                    ))?;
                for item in items {
                    let nested = item
                        .as_object_mut()
                        .and_then(|item| item.get_mut("blocks"))
                        .and_then(Value::as_array_mut)
                        .ok_or(StagingAdvancedDecodeError::Shape(
                            "list item blocks must be an array",
                        ))?;
                    collect_figure_placements(nested, output)?;
                }
            }
            "table" => {
                for section in ["head", "body"] {
                    let rows = object
                        .get_mut(section)
                        .and_then(Value::as_array_mut)
                        .ok_or(StagingAdvancedDecodeError::Shape(
                            "table section must be an array",
                        ))?;
                    for row in rows {
                        let cells = row
                            .as_object_mut()
                            .and_then(|row| row.get_mut("cells"))
                            .and_then(Value::as_array_mut)
                            .ok_or(StagingAdvancedDecodeError::Shape(
                                "table cells must be an array",
                            ))?;
                        for cell in cells {
                            let nested = cell
                                .as_object_mut()
                                .and_then(|cell| cell.get_mut("blocks"))
                                .and_then(Value::as_array_mut)
                                .ok_or(StagingAdvancedDecodeError::Shape(
                                    "table cell blocks must be an array",
                                ))?;
                            collect_figure_placements(nested, output)?;
                        }
                    }
                }
            }
            "paragraph" | "heading" | "page_break" => {}
            _ => {}
        }
    }
    Ok(())
}

fn charge_node(total: &mut u64, maximum: u64) -> Result<(), StagingAdvancedDecodeError> {
    let next = total
        .checked_add(1)
        .ok_or(StagingAdvancedDecodeError::Limit)?;
    if next > maximum {
        return Err(StagingAdvancedDecodeError::Limit);
    }
    *total = next;
    Ok(())
}

fn count_raw_base_nodes(document: &Value, maximum: u64) -> Result<u64, StagingAdvancedDecodeError> {
    #[derive(Clone, Copy)]
    enum RawNode<'a> {
        Document(&'a Value),
        Footnote(&'a Value),
        Block(&'a Value),
        ListItem(&'a Value),
        TableColumn,
        TableRow(&'a Value),
        TableCell(&'a Value),
        Inline(&'a Value),
    }

    fn member_array<'a>(
        value: &'a Value,
        member: &'static str,
    ) -> Result<&'a [Value], StagingAdvancedDecodeError> {
        value
            .as_object()
            .and_then(|object| object.get(member))
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .ok_or(StagingAdvancedDecodeError::Shape(member))
    }

    fn schedule<'a>(
        values: &'a [Value],
        wrap: impl Fn(&'a Value) -> RawNode<'a>,
        stack: &mut Vec<RawNode<'a>>,
        total: &mut u64,
        maximum: u64,
    ) -> Result<(), StagingAdvancedDecodeError> {
        let count = u64::try_from(values.len()).map_err(|_| StagingAdvancedDecodeError::Limit)?;
        let next = total
            .checked_add(count)
            .ok_or(StagingAdvancedDecodeError::Limit)?;
        if next > maximum {
            return Err(StagingAdvancedDecodeError::Limit);
        }
        stack
            .try_reserve(values.len())
            .map_err(|_| StagingAdvancedDecodeError::Limit)?;
        *total = next;
        stack.extend(values.iter().rev().map(wrap));
        Ok(())
    }

    if maximum == 0 {
        return Err(StagingAdvancedDecodeError::Limit);
    }
    let mut total = 1u64;
    let mut stack = vec![RawNode::Document(document)];
    while let Some(node) = stack.pop() {
        match node {
            RawNode::Document(value) => {
                schedule(
                    member_array(value, "footnotes")?,
                    RawNode::Footnote,
                    &mut stack,
                    &mut total,
                    maximum,
                )?;
                schedule(
                    member_array(value, "blocks")?,
                    RawNode::Block,
                    &mut stack,
                    &mut total,
                    maximum,
                )?;
            }
            RawNode::Footnote(value) | RawNode::ListItem(value) | RawNode::TableCell(value) => {
                schedule(
                    member_array(value, "blocks")?,
                    RawNode::Block,
                    &mut stack,
                    &mut total,
                    maximum,
                )?;
            }
            RawNode::TableColumn => {}
            RawNode::TableRow(value) => {
                schedule(
                    member_array(value, "cells")?,
                    RawNode::TableCell,
                    &mut stack,
                    &mut total,
                    maximum,
                )?;
            }
            RawNode::Block(value) => {
                let kind = value
                    .as_object()
                    .and_then(|object| object.get("kind"))
                    .and_then(Value::as_str)
                    .ok_or(StagingAdvancedDecodeError::Shape("block kind"))?;
                match kind {
                    "paragraph" | "heading" => schedule(
                        member_array(value, "children")?,
                        RawNode::Inline,
                        &mut stack,
                        &mut total,
                        maximum,
                    )?,
                    "list" => schedule(
                        member_array(value, "items")?,
                        RawNode::ListItem,
                        &mut stack,
                        &mut total,
                        maximum,
                    )?,
                    "table" => {
                        schedule(
                            member_array(value, "columns")?,
                            |_| RawNode::TableColumn,
                            &mut stack,
                            &mut total,
                            maximum,
                        )?;
                        schedule(
                            member_array(value, "body")?,
                            RawNode::TableRow,
                            &mut stack,
                            &mut total,
                            maximum,
                        )?;
                        schedule(
                            member_array(value, "head")?,
                            RawNode::TableRow,
                            &mut stack,
                            &mut total,
                            maximum,
                        )?;
                    }
                    "figure" => schedule(
                        member_array(value, "caption")?,
                        RawNode::Block,
                        &mut stack,
                        &mut total,
                        maximum,
                    )?,
                    "page_break" => {}
                    _ => return Err(StagingAdvancedDecodeError::Shape("unsupported block kind")),
                }
            }
            RawNode::Inline(value) => {
                let kind = value
                    .as_object()
                    .and_then(|object| object.get("kind"))
                    .and_then(Value::as_str)
                    .ok_or(StagingAdvancedDecodeError::Shape("inline kind"))?;
                match kind {
                    "emphasis" | "strong" | "link" => schedule(
                        member_array(value, "children")?,
                        RawNode::Inline,
                        &mut stack,
                        &mut total,
                        maximum,
                    )?,
                    "text" | "anchor" | "reference" | "footnote_reference" | "soft_break"
                    | "hard_break" => {}
                    _ => return Err(StagingAdvancedDecodeError::Shape("unsupported inline kind")),
                }
            }
        }
    }
    Ok(total)
}

fn count_base_nodes(document: &crate::WireDocument) -> Result<u64, StagingAdvancedDecodeError> {
    fn blocks(
        values: &[crate::WireBlock],
        count: &mut u64,
    ) -> Result<(), StagingAdvancedDecodeError> {
        for block in values {
            *count = count
                .checked_add(1)
                .ok_or(StagingAdvancedDecodeError::Limit)?;
            match block {
                crate::WireBlock::Paragraph { children, .. }
                | crate::WireBlock::Heading { children, .. } => inlines(children, count)?,
                crate::WireBlock::List { items, .. } => {
                    for item in items {
                        *count = count
                            .checked_add(1)
                            .ok_or(StagingAdvancedDecodeError::Limit)?;
                        blocks(&item.blocks, count)?;
                    }
                }
                crate::WireBlock::Table {
                    columns,
                    head,
                    body,
                    ..
                } => {
                    *count = count
                        .checked_add(
                            u64::try_from(columns.len())
                                .map_err(|_| StagingAdvancedDecodeError::Limit)?,
                        )
                        .ok_or(StagingAdvancedDecodeError::Limit)?;
                    for row in head.iter().chain(body) {
                        *count = count
                            .checked_add(1)
                            .ok_or(StagingAdvancedDecodeError::Limit)?;
                        for cell in &row.cells {
                            *count = count
                                .checked_add(1)
                                .ok_or(StagingAdvancedDecodeError::Limit)?;
                            blocks(&cell.blocks, count)?;
                        }
                    }
                }
                crate::WireBlock::Figure { caption, .. } => blocks(caption, count)?,
                crate::WireBlock::PageBreak { .. } => {}
            }
        }
        Ok(())
    }
    fn inlines(
        values: &[crate::WireInline],
        count: &mut u64,
    ) -> Result<(), StagingAdvancedDecodeError> {
        for inline in values {
            *count = count
                .checked_add(1)
                .ok_or(StagingAdvancedDecodeError::Limit)?;
            match inline {
                crate::WireInline::Emphasis { children, .. }
                | crate::WireInline::Strong { children, .. }
                | crate::WireInline::Link { children, .. } => inlines(children, count)?,
                _ => {}
            }
        }
        Ok(())
    }
    let mut count = 1u64;
    blocks(&document.blocks, &mut count)?;
    for footnote in &document.footnotes {
        count = count
            .checked_add(1)
            .ok_or(StagingAdvancedDecodeError::Limit)?;
        blocks(&footnote.blocks, &mut count)?;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};

    const COMBINED: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/header-footer-1/combined/job/document-package.json"
    ));
    const EMPTY: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/header-footer-1/empty/job/document-package.json"
    ));

    fn decode(
        bytes: &[u8],
        limits: ResourceLimits,
    ) -> Result<DecodedStagingAdvancedDocumentPackage, StagingAdvancedDecodeError> {
        let limits = ValidatedResourceLimits::new(limits).expect("test limits are valid");
        StagingAdvancedDocumentPackageDecoder::new()
            .decode(bytes, &DocumentPackageDecodePolicy::new(&limits))
    }

    #[test]
    fn staging_canonical_hash_ignores_json_formatting() {
        let compact = serde_json::to_vec(
            &serde_json::from_slice::<Value>(COMBINED).expect("fixture JSON is valid"),
        )
        .expect("fixture JSON serializes");
        assert_ne!(COMBINED, compact.as_slice());

        let pretty = decode(COMBINED, ResourceLimits::default()).expect("pretty fixture decodes");
        let compact = decode(&compact, ResourceLimits::default()).expect("compact fixture decodes");
        assert_ne!(pretty.raw_sha256(), compact.raw_sha256());
        assert_eq!(
            pretty.canonical_jcs_sha256(),
            compact.canonical_jcs_sha256()
        );
    }

    #[test]
    fn staging_ast_and_page_master_limits_are_inclusive() {
        let exact_ast = ResourceLimits {
            max_ast_nodes: 2, // document root + present empty header region
            ..ResourceLimits::default()
        };
        decode(EMPTY, exact_ast).expect("the exact effective AST maximum is accepted");

        let over_ast = ResourceLimits {
            max_ast_nodes: 1,
            ..ResourceLimits::default()
        };
        assert!(matches!(
            decode(EMPTY, over_ast),
            Err(StagingAdvancedDecodeError::Limit)
        ));

        let with_column = String::from_utf8(EMPTY.to_vec())
            .expect("fixture is UTF-8")
            .replacen(
                "\"column_layout\": null",
                "\"column_layout\": {\"balance\": \"none\", \"count\": 2, \"fill\": \"sequential\", \"gap\": 0}",
                1,
            );
        let column_over_ast = ResourceLimits {
            max_ast_nodes: 2,
            ..ResourceLimits::default()
        };
        assert!(matches!(
            decode(with_column.as_bytes(), column_over_ast),
            Err(StagingAdvancedDecodeError::Limit)
        ));

        let mut masters_only =
            serde_json::from_slice::<Value>(COMBINED).expect("combined fixture JSON is valid");
        masters_only["style_sheet"]["rules"] = Value::Array(Vec::new());
        let masters_only = serde_json::to_vec(&masters_only).expect("fixture JSON serializes");
        let exact_masters = ResourceLimits {
            max_style_rules: 5, // three masters + two selection rules
            ..ResourceLimits::default()
        };
        decode(&masters_only, exact_masters).expect("the exact master/rule maximum is accepted");

        let over_masters = ResourceLimits {
            max_style_rules: 4,
            ..ResourceLimits::default()
        };
        assert!(matches!(
            decode(&masters_only, over_masters),
            Err(StagingAdvancedDecodeError::Limit)
        ));
    }
}
