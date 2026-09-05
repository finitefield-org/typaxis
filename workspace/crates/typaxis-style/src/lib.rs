#![forbid(unsafe_code)]

use core::num::{NonZeroU32, NonZeroU64};
use std::collections::{BTreeMap, BTreeSet};
use typaxis_core::{
    push_jcs_string, sha256, FontFaceId, Length, MasterId, NonNegativeLength, PageName,
    PositiveLength, Rect, StyleId, JSON_SAFE_INTEGER_MAX,
};
use typaxis_resource_admission::AdmittedResourceLedgerToken;

pub const STYLEABLE_BLOCK_TYPES: &[&str] = &[
    "paragraph",
    "heading",
    "list",
    "table",
    "figure",
    "page_break",
];

/// Contract-1.4 production selector extension. These names deliberately do
/// not appear in [`STYLEABLE_BLOCK_TYPES`], so frozen profile registries remain
/// closed while `production-book-1` opts into this extension explicitly.
pub const PRECOMPOSED_VECTOR_STYLEABLE_BLOCK_TYPES: &[&str] =
    &["math_vector_block", "vector_figure"];

const STAGING_PRECOMPOSED_VECTOR_STYLEABLE_BLOCK_TYPES: &[&str] = &[
    "paragraph",
    "heading",
    "list",
    "table",
    "figure",
    "page_break",
    "math_vector_block",
    "vector_figure",
];

pub fn is_style_identifier(value: &str) -> bool {
    StyleId::is_valid(value)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectorError {
    InvalidBlockType,
    InvalidClass,
    DuplicateClass,
    NonCanonicalClassOrder,
}

pub fn validate_selector(selector: &str) -> Result<(), SelectorError> {
    validate_selector_for(selector, STYLEABLE_BLOCK_TYPES)
}

pub fn validate_precomposed_vector_selector(selector: &str) -> Result<(), SelectorError> {
    validate_selector_for(selector, PRECOMPOSED_VECTOR_STYLEABLE_BLOCK_TYPES)
}

fn validate_selector_for(selector: &str, block_types: &[&str]) -> Result<(), SelectorError> {
    let mut components = selector.split('.');
    let block_type = components.next().unwrap_or_default();
    if !block_types.contains(&block_type) {
        return Err(SelectorError::InvalidBlockType);
    }
    let mut classes = BTreeSet::new();
    let mut previous: Option<&str> = None;
    for class in components {
        if !is_style_identifier(class) {
            return Err(SelectorError::InvalidClass);
        }
        if !classes.insert(class) {
            return Err(SelectorError::DuplicateClass);
        }
        if previous.is_some_and(|value| value > class) {
            return Err(SelectorError::NonCanonicalClassOrder);
        }
        previous = Some(class);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StyleValue {
    Keyword(String),
    Text(String),
    Integer(i64),
    Length(Length),
    Boolean(bool),
    FontFamilyList(Vec<String>),
    Ratio {
        numerator: i64,
        denominator: NonZeroU64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: StyleValue,
    pub important: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleRule {
    pub style_id: StyleId,
    pub extends: Option<StyleId>,
    pub selector: String,
    pub source_order: u32,
    pub declarations: Vec<Declaration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleSheet {
    pub rules: Vec<StyleRule>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StyleValidationError {
    DuplicateStyleId,
    SourceOrderMismatch,
    InvalidSelector(SelectorError),
    UnknownParent,
    InheritanceCycle,
    InvalidDeclarationName,
    InvalidDeclarationValue,
    InvalidPageProperty,
    UnknownProperty,
    MissingTextProperty,
    UnknownFontFamily,
    CascadePriorityOverflow,
    InapplicableProperty,
}

impl StyleSheet {
    pub fn validate(&self) -> Result<(), StyleValidationError> {
        self.validate_contract_for(false, STYLEABLE_BLOCK_TYPES)
    }

    /// Validates the additive contract 1.2 style shape without publishing it
    /// through the current contract. Selector/property applicability remains a
    /// profile concern and is checked by [`Self::validate_basic_document_styles`].
    pub fn validate_basic_document_style_shape(&self) -> Result<(), StyleValidationError> {
        self.validate_contract_for(true, STYLEABLE_BLOCK_TYPES)
    }

    /// Validates both the 1.2 tagged-value domain and the immutable
    /// `basic-document-1` selector/property coverage table.
    pub fn validate_basic_document_styles(&self) -> Result<(), StyleValidationError> {
        self.validate_contract_for(true, STYLEABLE_BLOCK_TYPES)?;
        for rule in &self.rules {
            let block_type = rule.selector.split('.').next().unwrap_or_default();
            if BasicStyleBlockKind::from_str(block_type).is_none() {
                return Err(StyleValidationError::InapplicableProperty);
            }
            for declaration in &rule.declarations {
                let property = BasicStyleProperty::from_str(&declaration.name)
                    .ok_or(StyleValidationError::UnknownProperty)?;
                if !property.applies_to_str(block_type) {
                    return Err(StyleValidationError::InapplicableProperty);
                }
            }
        }
        Ok(())
    }

    /// Validates the private `table-1` style domain without widening the
    /// immutable `basic-document-1` profile. Existing M2 selectors retain
    /// their applicability rules; `table` accepts only the already-typed
    /// block placement properties fixed by ADR-0029.
    pub fn validate_table_document_styles(&self) -> Result<(), StyleValidationError> {
        self.validate_contract_for(true, STYLEABLE_BLOCK_TYPES)?;
        for rule in &self.rules {
            let block_type = rule.selector.split('.').next().unwrap_or_default();
            if block_type != "table" && BasicStyleBlockKind::from_str(block_type).is_none() {
                return Err(StyleValidationError::InapplicableProperty);
            }
            for declaration in &rule.declarations {
                let property = BasicStyleProperty::from_str(&declaration.name)
                    .ok_or(StyleValidationError::UnknownProperty)?;
                let applies = if block_type == "table" {
                    property.applies_to_table()
                } else {
                    BasicStyleBlockKind::from_str(block_type)
                        .is_some_and(|block| property.applies_to(block))
                };
                if !applies {
                    return Err(StyleValidationError::InapplicableProperty);
                }
                if block_type == "table"
                    && property == BasicStyleProperty::Page
                    && !matches!(
                        &declaration.value,
                        StyleValue::Keyword(value) if value == "auto"
                    )
                {
                    return Err(StyleValidationError::InvalidPageProperty);
                }
            }
        }
        Ok(())
    }

    /// Validates the shared declaration/tag/graph shape while admitting both
    /// existing selectors and the private precomposed-vector selectors.
    /// Registry applicability is still checked separately, so this helper
    /// cannot make a new selector valid in the basic `/1` registry.
    pub fn validate_staging_precomposed_vector_style_shape(
        &self,
    ) -> Result<(), StyleValidationError> {
        self.validate_contract_for(true, STAGING_PRECOMPOSED_VECTOR_STYLEABLE_BLOCK_TYPES)
    }

    fn validate_contract_for(
        &self,
        staging_1_2: bool,
        block_types: &[&str],
    ) -> Result<(), StyleValidationError> {
        let mut by_id = BTreeMap::new();
        for (index, rule) in self.rules.iter().enumerate() {
            if rule.source_order
                != u32::try_from(index).map_err(|_| StyleValidationError::SourceOrderMismatch)?
            {
                return Err(StyleValidationError::SourceOrderMismatch);
            }
            validate_selector_for(&rule.selector, block_types)
                .map_err(StyleValidationError::InvalidSelector)?;
            for declaration in &rule.declarations {
                if !valid_declaration_name(&declaration.name) {
                    return Err(StyleValidationError::InvalidDeclarationName);
                }
                match declaration.name.as_str() {
                    "font_family"
                        if !matches!(
                            &declaration.value,
                            StyleValue::FontFamilyList(families)
                                if valid_font_family_list(families)
                        ) =>
                    {
                        return Err(StyleValidationError::InvalidDeclarationValue)
                    }
                    "font_size" | "line_height"
                        if !matches!(
                            declaration.value,
                            StyleValue::Length(value)
                                if value.raw() > 0
                                    && value.raw() <= JSON_SAFE_INTEGER_MAX
                        ) =>
                    {
                        return Err(StyleValidationError::InvalidDeclarationValue)
                    }
                    "page" if !valid_page_value(&declaration.value) => {
                        return Err(StyleValidationError::InvalidPageProperty)
                    }
                    "space_before" | "space_after" | "start_indent" | "end_indent"
                        if staging_1_2
                            && !matches!(
                                declaration.value,
                                StyleValue::Length(value)
                                    if value.raw() >= 0
                                        && value.raw() <= JSON_SAFE_INTEGER_MAX
                            ) =>
                    {
                        return Err(StyleValidationError::InvalidDeclarationValue)
                    }
                    "text_align"
                        if staging_1_2
                            && !matches!(
                                &declaration.value,
                                StyleValue::Keyword(value)
                                    if matches!(value.as_str(), "start" | "end" | "center")
                            ) =>
                    {
                        return Err(StyleValidationError::InvalidDeclarationValue)
                    }
                    "width"
                        if staging_1_2
                            && !matches!(
                                &declaration.value,
                                StyleValue::Keyword(value) if value == "auto"
                            )
                            && !matches!(
                                declaration.value,
                                StyleValue::Length(value)
                                    if value.raw() > 0
                                        && value.raw() <= JSON_SAFE_INTEGER_MAX
                            ) =>
                    {
                        return Err(StyleValidationError::InvalidDeclarationValue)
                    }
                    "keep_with_next" | "keep_caption"
                        if staging_1_2 && !matches!(declaration.value, StyleValue::Boolean(_)) =>
                    {
                        return Err(StyleValidationError::InvalidDeclarationValue)
                    }
                    "font_family" | "font_size" | "line_height" | "page" => {}
                    "space_before" | "space_after" | "start_indent" | "end_indent"
                    | "text_align" | "width" | "keep_with_next" | "keep_caption"
                        if staging_1_2 => {}
                    _ => return Err(StyleValidationError::UnknownProperty),
                }
            }
            if by_id.insert(&rule.style_id, rule).is_some() {
                return Err(StyleValidationError::DuplicateStyleId);
            }
        }

        for rule in &self.rules {
            if let Some(parent) = rule.extends.as_ref() {
                if !by_id.contains_key(parent) {
                    return Err(StyleValidationError::UnknownParent);
                }
                let mut path = BTreeSet::new();
                let mut current = Some(&rule.style_id);
                while let Some(style_id) = current {
                    if !path.insert(style_id) {
                        return Err(StyleValidationError::InheritanceCycle);
                    }
                    current = by_id.get(style_id).and_then(|style| style.extends.as_ref());
                }
            }
        }
        Ok(())
    }
}

fn valid_declaration_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(first) if first.is_ascii_lowercase())
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

fn valid_page_value(value: &StyleValue) -> bool {
    match value {
        StyleValue::Keyword(value) => value == "auto",
        StyleValue::Text(value) => PageName::is_valid(value),
        _ => false,
    }
}

fn valid_font_family_list(families: &[String]) -> bool {
    if families.is_empty() {
        return false;
    }
    let mut unique = BTreeSet::new();
    families.iter().all(|family| {
        !family.trim().is_empty()
            && !family.chars().any(char::is_control)
            && unique.insert(family.as_str())
    })
}

fn selector_parts(selector: &str) -> Result<(&str, BTreeSet<&str>), SelectorError> {
    selector_parts_for(selector, STYLEABLE_BLOCK_TYPES)
}

fn selector_parts_for<'a>(
    selector: &'a str,
    block_types: &[&str],
) -> Result<(&'a str, BTreeSet<&'a str>), SelectorError> {
    validate_selector_for(selector, block_types)?;
    let mut components = selector.split('.');
    let block_type = components.next().unwrap_or_default();
    Ok((block_type, components.collect()))
}

pub fn selector_matches(
    selector: &str,
    block_type: &str,
    classes: &[String],
) -> Result<bool, SelectorError> {
    let (selector_type, selector_classes) = selector_parts(selector)?;
    let target_classes: BTreeSet<&str> = classes.iter().map(String::as_str).collect();
    Ok(selector_type == block_type && selector_classes.is_subset(&target_classes))
}

fn precomposed_vector_selector_matches(
    selector: &str,
    block_type: &str,
    classes: &[String],
) -> Result<bool, SelectorError> {
    let (selector_type, selector_classes) =
        selector_parts_for(selector, PRECOMPOSED_VECTOR_STYLEABLE_BLOCK_TYPES)?;
    let target_classes: BTreeSet<&str> = classes.iter().map(String::as_str).collect();
    Ok(selector_type == block_type && selector_classes.is_subset(&target_classes))
}

/// Immutable registry identity bound into every computed M2 block-style
/// receipt. Changing any property shape, default, inheritance, applicability,
/// or consumer requires a new registry identifier.
pub const BASIC_BLOCK_STYLE_REGISTRY_VERSION: &str = "typaxis.basic-block-style-registry/1";
pub const TABLE_BLOCK_STYLE_REGISTRY_VERSION: &str = "typaxis.table-block-style-registry/1";
pub const PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION: &str = "typaxis.precomposed-vector-style/1";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BasicStyleBlockKind {
    Figure,
    Heading,
    List,
    PageBreak,
    Paragraph,
}

impl BasicStyleBlockKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Figure => "figure",
            Self::Heading => "heading",
            Self::List => "list",
            Self::PageBreak => "page_break",
            Self::Paragraph => "paragraph",
        }
    }

    pub const fn from_str(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"figure" => Some(Self::Figure),
            b"heading" => Some(Self::Heading),
            b"list" => Some(Self::List),
            b"page_break" => Some(Self::PageBreak),
            b"paragraph" => Some(Self::Paragraph),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BasicStyleProperty {
    FontFamily,
    FontSize,
    LineHeight,
    Page,
    SpaceBefore,
    SpaceAfter,
    StartIndent,
    EndIndent,
    TextAlign,
    Width,
    KeepWithNext,
    KeepCaption,
}

impl BasicStyleProperty {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FontFamily => "font_family",
            Self::FontSize => "font_size",
            Self::LineHeight => "line_height",
            Self::Page => "page",
            Self::SpaceBefore => "space_before",
            Self::SpaceAfter => "space_after",
            Self::StartIndent => "start_indent",
            Self::EndIndent => "end_indent",
            Self::TextAlign => "text_align",
            Self::Width => "width",
            Self::KeepWithNext => "keep_with_next",
            Self::KeepCaption => "keep_caption",
        }
    }

    pub const fn from_str(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"font_family" => Some(Self::FontFamily),
            b"font_size" => Some(Self::FontSize),
            b"line_height" => Some(Self::LineHeight),
            b"page" => Some(Self::Page),
            b"space_before" => Some(Self::SpaceBefore),
            b"space_after" => Some(Self::SpaceAfter),
            b"start_indent" => Some(Self::StartIndent),
            b"end_indent" => Some(Self::EndIndent),
            b"text_align" => Some(Self::TextAlign),
            b"width" => Some(Self::Width),
            b"keep_with_next" => Some(Self::KeepWithNext),
            b"keep_caption" => Some(Self::KeepCaption),
            _ => None,
        }
    }

    pub const fn applies_to(self, block: BasicStyleBlockKind) -> bool {
        match self {
            Self::FontFamily | Self::FontSize | Self::LineHeight => matches!(
                block,
                BasicStyleBlockKind::Paragraph
                    | BasicStyleBlockKind::Heading
                    | BasicStyleBlockKind::List
            ),
            Self::Page => true,
            Self::SpaceBefore
            | Self::SpaceAfter
            | Self::StartIndent
            | Self::EndIndent
            | Self::KeepWithNext => !matches!(block, BasicStyleBlockKind::PageBreak),
            Self::TextAlign => matches!(
                block,
                BasicStyleBlockKind::Paragraph | BasicStyleBlockKind::Heading
            ),
            Self::Width | Self::KeepCaption => matches!(block, BasicStyleBlockKind::Figure),
        }
    }

    /// Closed table applicability set for the private `table-1` profile.
    /// This is deliberately separate from [`Self::applies_to`] so adding the
    /// table slice cannot broaden either current public M2 profile.
    pub const fn applies_to_table(self) -> bool {
        matches!(
            self,
            Self::Page
                | Self::SpaceBefore
                | Self::SpaceAfter
                | Self::StartIndent
                | Self::EndIndent
                | Self::KeepWithNext
        )
    }

    const fn applies_to_str(self, block: &str) -> bool {
        match BasicStyleBlockKind::from_str(block) {
            Some(block) => self.applies_to(block),
            None => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BasicBlockStyleConsumer {
    BlockFlowGlueBefore,
    BlockFlowGlueAfter,
    LogicalStartInset,
    LogicalEndInset,
    LinePlacement,
    FigureInlineSize,
    BlockNextKeep,
    FigureCaptionKeep,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicBlockStyleInitial {
    ZeroLength,
    TextStart,
    WidthAuto,
    Boolean(bool),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BasicBlockStylePropertyDescriptor {
    pub property: BasicStyleProperty,
    pub initial: BasicBlockStyleInitial,
    pub inherited: bool,
    pub consumer: BasicBlockStyleConsumer,
}

/// Complete additive 1.2 property table. Profile preflight, fixtures, and
/// typed consumers enumerate this table instead of maintaining name lists.
pub const BASIC_BLOCK_STYLE_PROPERTIES: &[BasicBlockStylePropertyDescriptor] = &[
    BasicBlockStylePropertyDescriptor {
        property: BasicStyleProperty::SpaceBefore,
        initial: BasicBlockStyleInitial::ZeroLength,
        inherited: false,
        consumer: BasicBlockStyleConsumer::BlockFlowGlueBefore,
    },
    BasicBlockStylePropertyDescriptor {
        property: BasicStyleProperty::SpaceAfter,
        initial: BasicBlockStyleInitial::ZeroLength,
        inherited: false,
        consumer: BasicBlockStyleConsumer::BlockFlowGlueAfter,
    },
    BasicBlockStylePropertyDescriptor {
        property: BasicStyleProperty::StartIndent,
        initial: BasicBlockStyleInitial::ZeroLength,
        inherited: false,
        consumer: BasicBlockStyleConsumer::LogicalStartInset,
    },
    BasicBlockStylePropertyDescriptor {
        property: BasicStyleProperty::EndIndent,
        initial: BasicBlockStyleInitial::ZeroLength,
        inherited: false,
        consumer: BasicBlockStyleConsumer::LogicalEndInset,
    },
    BasicBlockStylePropertyDescriptor {
        property: BasicStyleProperty::TextAlign,
        initial: BasicBlockStyleInitial::TextStart,
        inherited: true,
        consumer: BasicBlockStyleConsumer::LinePlacement,
    },
    BasicBlockStylePropertyDescriptor {
        property: BasicStyleProperty::Width,
        initial: BasicBlockStyleInitial::WidthAuto,
        inherited: false,
        consumer: BasicBlockStyleConsumer::FigureInlineSize,
    },
    BasicBlockStylePropertyDescriptor {
        property: BasicStyleProperty::KeepWithNext,
        initial: BasicBlockStyleInitial::Boolean(false),
        inherited: false,
        consumer: BasicBlockStyleConsumer::BlockNextKeep,
    },
    BasicBlockStylePropertyDescriptor {
        property: BasicStyleProperty::KeepCaption,
        initial: BasicBlockStyleInitial::Boolean(true),
        inherited: false,
        consumer: BasicBlockStyleConsumer::FigureCaptionKeep,
    },
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PrecomposedVectorStyleKind {
    MathVectorBlock,
    VectorFigure,
}

impl PrecomposedVectorStyleKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MathVectorBlock => "math_vector_block",
            Self::VectorFigure => "vector_figure",
        }
    }

    pub const fn from_str(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"math_vector_block" => Some(Self::MathVectorBlock),
            b"vector_figure" => Some(Self::VectorFigure),
            _ => None,
        }
    }
}

pub const PRECOMPOSED_VECTOR_STYLE_KINDS: &[PrecomposedVectorStyleKind] = &[
    PrecomposedVectorStyleKind::MathVectorBlock,
    PrecomposedVectorStyleKind::VectorFigure,
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PrecomposedVectorStyleProperty {
    EndIndent,
    FontFamily,
    FontSize,
    KeepCaption,
    KeepWithNext,
    LineHeight,
    Page,
    SpaceAfter,
    SpaceBefore,
    StartIndent,
    TextAlign,
    Width,
}

impl PrecomposedVectorStyleProperty {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EndIndent => "end_indent",
            Self::FontFamily => "font_family",
            Self::FontSize => "font_size",
            Self::KeepCaption => "keep_caption",
            Self::KeepWithNext => "keep_with_next",
            Self::LineHeight => "line_height",
            Self::Page => "page",
            Self::SpaceAfter => "space_after",
            Self::SpaceBefore => "space_before",
            Self::StartIndent => "start_indent",
            Self::TextAlign => "text_align",
            Self::Width => "width",
        }
    }

    pub const fn from_str(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"end_indent" => Some(Self::EndIndent),
            b"font_family" => Some(Self::FontFamily),
            b"font_size" => Some(Self::FontSize),
            b"keep_caption" => Some(Self::KeepCaption),
            b"keep_with_next" => Some(Self::KeepWithNext),
            b"line_height" => Some(Self::LineHeight),
            b"page" => Some(Self::Page),
            b"space_after" => Some(Self::SpaceAfter),
            b"space_before" => Some(Self::SpaceBefore),
            b"start_indent" => Some(Self::StartIndent),
            b"text_align" => Some(Self::TextAlign),
            b"width" => Some(Self::Width),
            _ => None,
        }
    }
}

pub const PRECOMPOSED_VECTOR_STYLE_PROPERTY_DOMAIN: &[PrecomposedVectorStyleProperty] = &[
    PrecomposedVectorStyleProperty::EndIndent,
    PrecomposedVectorStyleProperty::FontFamily,
    PrecomposedVectorStyleProperty::FontSize,
    PrecomposedVectorStyleProperty::KeepCaption,
    PrecomposedVectorStyleProperty::KeepWithNext,
    PrecomposedVectorStyleProperty::LineHeight,
    PrecomposedVectorStyleProperty::Page,
    PrecomposedVectorStyleProperty::SpaceAfter,
    PrecomposedVectorStyleProperty::SpaceBefore,
    PrecomposedVectorStyleProperty::StartIndent,
    PrecomposedVectorStyleProperty::TextAlign,
    PrecomposedVectorStyleProperty::Width,
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PrecomposedVectorStyleConsumer {
    BlockFlowGlueAfter,
    BlockFlowGlueBefore,
    BlockNextKeep,
    EquationNumberFontFamily,
    EquationNumberFontSize,
    EquationNumberLineHeight,
    FigureCaptionKeep,
    LogicalEndInset,
    LogicalStartInset,
    NamedPage,
    VectorAlignment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrecomposedVectorStyleInitial {
    Absent,
    Boolean(bool),
    PageAuto,
    TextStart,
    ZeroLength,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrecomposedVectorStylePropertyDescriptor {
    pub kind: PrecomposedVectorStyleKind,
    pub property: PrecomposedVectorStyleProperty,
    pub initial: PrecomposedVectorStyleInitial,
    pub inherited: bool,
    pub consumer: PrecomposedVectorStyleConsumer,
}

macro_rules! precomposed_vector_shared_property {
    ($kind:expr, $property:ident, $initial:expr, $inherited:expr, $consumer:ident) => {
        PrecomposedVectorStylePropertyDescriptor {
            kind: $kind,
            property: PrecomposedVectorStyleProperty::$property,
            initial: $initial,
            inherited: $inherited,
            consumer: PrecomposedVectorStyleConsumer::$consumer,
        }
    };
}

/// Exhaustive accepted kind/property table for the private registry. `width`,
/// math `keep_caption`, and vector-figure font properties remain in the known
/// property domain but intentionally have no row, making them typed L5101
/// applicability failures rather than unknown-property aliases.
pub const PRECOMPOSED_VECTOR_STYLE_PROPERTIES: &[PrecomposedVectorStylePropertyDescriptor] = &[
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::MathVectorBlock,
        SpaceBefore,
        PrecomposedVectorStyleInitial::ZeroLength,
        false,
        BlockFlowGlueBefore
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::MathVectorBlock,
        SpaceAfter,
        PrecomposedVectorStyleInitial::ZeroLength,
        false,
        BlockFlowGlueAfter
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::MathVectorBlock,
        StartIndent,
        PrecomposedVectorStyleInitial::ZeroLength,
        false,
        LogicalStartInset
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::MathVectorBlock,
        EndIndent,
        PrecomposedVectorStyleInitial::ZeroLength,
        false,
        LogicalEndInset
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::MathVectorBlock,
        TextAlign,
        PrecomposedVectorStyleInitial::TextStart,
        true,
        VectorAlignment
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::MathVectorBlock,
        Page,
        PrecomposedVectorStyleInitial::PageAuto,
        false,
        NamedPage
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::MathVectorBlock,
        KeepWithNext,
        PrecomposedVectorStyleInitial::Boolean(false),
        false,
        BlockNextKeep
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::MathVectorBlock,
        FontFamily,
        PrecomposedVectorStyleInitial::Absent,
        true,
        EquationNumberFontFamily
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::MathVectorBlock,
        FontSize,
        PrecomposedVectorStyleInitial::Absent,
        true,
        EquationNumberFontSize
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::MathVectorBlock,
        LineHeight,
        PrecomposedVectorStyleInitial::Absent,
        true,
        EquationNumberLineHeight
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::VectorFigure,
        SpaceBefore,
        PrecomposedVectorStyleInitial::ZeroLength,
        false,
        BlockFlowGlueBefore
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::VectorFigure,
        SpaceAfter,
        PrecomposedVectorStyleInitial::ZeroLength,
        false,
        BlockFlowGlueAfter
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::VectorFigure,
        StartIndent,
        PrecomposedVectorStyleInitial::ZeroLength,
        false,
        LogicalStartInset
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::VectorFigure,
        EndIndent,
        PrecomposedVectorStyleInitial::ZeroLength,
        false,
        LogicalEndInset
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::VectorFigure,
        TextAlign,
        PrecomposedVectorStyleInitial::TextStart,
        true,
        VectorAlignment
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::VectorFigure,
        Page,
        PrecomposedVectorStyleInitial::PageAuto,
        false,
        NamedPage
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::VectorFigure,
        KeepWithNext,
        PrecomposedVectorStyleInitial::Boolean(false),
        false,
        BlockNextKeep
    ),
    precomposed_vector_shared_property!(
        PrecomposedVectorStyleKind::VectorFigure,
        KeepCaption,
        PrecomposedVectorStyleInitial::Boolean(true),
        false,
        FigureCaptionKeep
    ),
];

pub fn precomposed_vector_style_property_descriptor(
    kind: PrecomposedVectorStyleKind,
    property: PrecomposedVectorStyleProperty,
) -> Option<&'static PrecomposedVectorStylePropertyDescriptor> {
    PRECOMPOSED_VECTOR_STYLE_PROPERTIES
        .iter()
        .find(|descriptor| descriptor.kind == kind && descriptor.property == property)
}

impl StyleSheet {
    pub fn validate_precomposed_vector_styles(&self) -> Result<(), StyleValidationError> {
        self.validate_contract_for(true, PRECOMPOSED_VECTOR_STYLEABLE_BLOCK_TYPES)?;
        let by_id: BTreeMap<&StyleId, &StyleRule> = self
            .rules
            .iter()
            .map(|rule| (&rule.style_id, rule))
            .collect();
        for rule in &self.rules {
            let selector_kind = PrecomposedVectorStyleKind::from_str(
                rule.selector.split('.').next().unwrap_or_default(),
            )
            .ok_or(StyleValidationError::InvalidSelector(
                SelectorError::InvalidBlockType,
            ))?;
            let mut current = Some(rule);
            while let Some(origin) = current {
                for declaration in &origin.declarations {
                    let property = PrecomposedVectorStyleProperty::from_str(&declaration.name)
                        .ok_or(StyleValidationError::UnknownProperty)?;
                    if precomposed_vector_style_property_descriptor(selector_kind, property)
                        .is_none()
                    {
                        return Err(StyleValidationError::InapplicableProperty);
                    }
                }
                current = origin
                    .extends
                    .as_ref()
                    .and_then(|parent| by_id.get(parent).copied());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineTextAlign {
    Start,
    End,
    Center,
}

impl MachineTextAlign {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
            Self::Center => "center",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineFigureWidth {
    Auto,
    Length(PositiveLength),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComputedMachineBlockStyle {
    space_before: NonNegativeLength,
    space_after: NonNegativeLength,
    start_indent: NonNegativeLength,
    end_indent: NonNegativeLength,
    text_align: MachineTextAlign,
    width: MachineFigureWidth,
    keep_with_next: bool,
    keep_caption: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrecomposedVectorEquationNumberTextStyle {
    font_families: Option<Vec<String>>,
    font_size: Option<PositiveLength>,
    line_height: Option<PositiveLength>,
}

impl PrecomposedVectorEquationNumberTextStyle {
    pub fn font_families(&self) -> Option<&[String]> {
        self.font_families.as_deref()
    }

    pub const fn font_size(&self) -> Option<PositiveLength> {
        self.font_size
    }

    pub const fn line_height(&self) -> Option<PositiveLength> {
        self.line_height
    }
}

#[derive(Debug, Eq, PartialEq)]
enum PrecomposedVectorStyleSupplement {
    EquationNumber(PrecomposedVectorEquationNumberTextStyle),
    FigureCaption { keep_caption: bool },
}

#[derive(Debug, Eq, PartialEq)]
struct PrecomposedVectorStyleBinding;

/// Sealed result of the private vector-block cascade. The public accessors
/// expose only typed consumer fields: no raw property map, `width`, formula
/// metric, or SVG scale channel exists on this receipt.
#[derive(Debug, Eq, PartialEq)]
pub struct PrecomposedVectorComputedStyleReceipt {
    registry_version: &'static str,
    kind: PrecomposedVectorStyleKind,
    block: ComputedMachineBlockStyle,
    page_name: Option<PageName>,
    supplement: PrecomposedVectorStyleSupplement,
    canonical_jcs: String,
    fingerprint: [u8; 32],
    _binding: PrecomposedVectorStyleBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrecomposedVectorStyleReceiptMismatch;

impl std::fmt::Display for PrecomposedVectorStyleReceiptMismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("I9190: precomposed vector style receipt mismatch")
    }
}

impl std::error::Error for PrecomposedVectorStyleReceiptMismatch {}

impl PrecomposedVectorComputedStyleReceipt {
    pub const fn registry_version(&self) -> &'static str {
        self.registry_version
    }

    pub const fn kind(&self) -> PrecomposedVectorStyleKind {
        self.kind
    }

    pub const fn space_before(&self) -> NonNegativeLength {
        self.block.space_before
    }

    pub const fn space_after(&self) -> NonNegativeLength {
        self.block.space_after
    }

    pub const fn start_indent(&self) -> NonNegativeLength {
        self.block.start_indent
    }

    pub const fn end_indent(&self) -> NonNegativeLength {
        self.block.end_indent
    }

    pub const fn text_align(&self) -> MachineTextAlign {
        self.block.text_align
    }

    pub const fn page_name(&self) -> Option<&PageName> {
        self.page_name.as_ref()
    }

    pub const fn keep_with_next(&self) -> bool {
        self.block.keep_with_next
    }

    pub const fn equation_number_text_style(
        &self,
    ) -> Option<&PrecomposedVectorEquationNumberTextStyle> {
        match &self.supplement {
            PrecomposedVectorStyleSupplement::EquationNumber(style) => Some(style),
            PrecomposedVectorStyleSupplement::FigureCaption { .. } => None,
        }
    }

    pub const fn keep_caption(&self) -> Option<bool> {
        match &self.supplement {
            PrecomposedVectorStyleSupplement::EquationNumber(_) => None,
            PrecomposedVectorStyleSupplement::FigureCaption { keep_caption } => Some(*keep_caption),
        }
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify_for(
        &self,
        kind: PrecomposedVectorStyleKind,
    ) -> Result<(), PrecomposedVectorStyleReceiptMismatch> {
        let observed = encode_precomposed_vector_computed_style(self);
        let supplement_matches_kind = matches!(
            (self.kind, &self.supplement),
            (
                PrecomposedVectorStyleKind::MathVectorBlock,
                PrecomposedVectorStyleSupplement::EquationNumber(_)
            ) | (
                PrecomposedVectorStyleKind::VectorFigure,
                PrecomposedVectorStyleSupplement::FigureCaption { .. }
            )
        );
        if self.registry_version != PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION
            || self.kind != kind
            || !supplement_matches_kind
            || self.block.width != MachineFigureWidth::Auto
            || (self.kind == PrecomposedVectorStyleKind::MathVectorBlock
                && !self.block.keep_caption)
            || self.canonical_jcs != observed
            || self.fingerprint != sha256(observed.as_bytes())
        {
            return Err(PrecomposedVectorStyleReceiptMismatch);
        }
        Ok(())
    }
}

pub fn require_precomposed_vector_style_registry(
    registry_version: &str,
) -> Result<(), PrecomposedVectorStyleReceiptMismatch> {
    if registry_version != PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION {
        return Err(PrecomposedVectorStyleReceiptMismatch);
    }
    Ok(())
}

/// Style-owner copy of the closed semantic kind. Syntax performs the one
/// typed conversion from the document domain; layout never sees wire strings.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticContainerStyleKind {
    Result,
    Proof,
    Exercise,
}

impl SemanticContainerStyleKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Result => "result",
            Self::Proof => "proof",
            Self::Exercise => "exercise",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticContainerInheritanceStyle {
    block_style: ComputedMachineBlockStyle,
    font_families: Option<Vec<String>>,
    font_size: Option<PositiveLength>,
    line_height: Option<PositiveLength>,
}

impl SemanticContainerInheritanceStyle {
    pub const fn block_style(&self) -> ComputedMachineBlockStyle {
        self.block_style
    }

    pub fn font_families(&self) -> Option<&[String]> {
        self.font_families.as_deref()
    }

    pub const fn font_size(&self) -> Option<PositiveLength> {
        self.font_size
    }

    pub const fn line_height(&self) -> Option<PositiveLength> {
        self.line_height
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticContainerComputedStyle {
    semantic_kind: SemanticContainerStyleKind,
    inheritance: SemanticContainerInheritanceStyle,
    page_name: Option<PageName>,
}

impl SemanticContainerComputedStyle {
    pub const fn semantic_kind(&self) -> SemanticContainerStyleKind {
        self.semantic_kind
    }

    pub const fn block_style(&self) -> ComputedMachineBlockStyle {
        self.inheritance.block_style
    }

    pub const fn inheritance_style(&self) -> &SemanticContainerInheritanceStyle {
        &self.inheritance
    }

    pub const fn page_name(&self) -> Option<&PageName> {
        self.page_name.as_ref()
    }
}

/// Closes the contract-1.4 semantic selector cascade into the same typed block
/// domain used by existing layout. Syntax supplies an isolated, validated
/// sheet whose semantic selectors are represented by the paragraph-shaped
/// applicability domain, so the ordinary important/specificity/source-order,
/// `extends`, and inherited `text_align` behavior is reused exactly.
pub fn cascade_staging_semantic_container_style(
    semantic_kind: SemanticContainerStyleKind,
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
) -> Result<SemanticContainerComputedStyle, StyleValidationError> {
    sheet.validate_basic_document_styles()?;
    let computed = sheet.cascade_validated(BasicStyleBlockKind::Paragraph.as_str(), classes)?;
    let inheritance = close_semantic_inheritance_style(&computed, parent)?;
    if inheritance.block_style.width() != MachineFigureWidth::Auto
        || !inheritance.block_style.keep_caption()
    {
        return Err(StyleValidationError::InapplicableProperty);
    }
    Ok(SemanticContainerComputedStyle {
        semantic_kind,
        page_name: computed.page_name()?,
        inheritance,
    })
}

/// Computes the inheritance context of an ordinary block that owns a nested
/// semantic container. Semantic-only selectors are hidden by the syntax
/// owner before this function is called.
pub fn cascade_staging_semantic_descendant_style(
    block_type: &str,
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
) -> Result<SemanticContainerInheritanceStyle, StyleValidationError> {
    sheet.validate_table_document_styles()?;
    if block_type != "table" && BasicStyleBlockKind::from_str(block_type).is_none() {
        return Err(StyleValidationError::InvalidSelector(
            SelectorError::InvalidBlockType,
        ));
    }
    let computed = sheet.cascade_validated(block_type, classes)?;
    close_semantic_inheritance_style(&computed, parent)
}

fn close_semantic_inheritance_style(
    computed: &ComputedStyle,
    parent: Option<&SemanticContainerInheritanceStyle>,
) -> Result<SemanticContainerInheritanceStyle, StyleValidationError> {
    let font_families = match computed
        .properties
        .get(BasicStyleProperty::FontFamily.as_str())
    {
        Some(StyleValue::FontFamilyList(families)) if valid_font_family_list(families) => {
            Some(families.clone())
        }
        None => parent.and_then(|parent| parent.font_families.clone()),
        Some(_) => return Err(StyleValidationError::InvalidDeclarationValue),
    };
    let positive = |property: BasicStyleProperty, inherited: Option<PositiveLength>| match computed
        .properties
        .get(property.as_str())
    {
        Some(StyleValue::Length(value)) => PositiveLength::new(*value)
            .map(Some)
            .ok_or(StyleValidationError::InvalidDeclarationValue),
        None => Ok(inherited),
        Some(_) => Err(StyleValidationError::InvalidDeclarationValue),
    };
    Ok(SemanticContainerInheritanceStyle {
        block_style: close_machine_block_style(
            computed,
            parent
                .map(SemanticContainerInheritanceStyle::block_style)
                .as_ref(),
        )?,
        font_size: positive(
            BasicStyleProperty::FontSize,
            parent.and_then(SemanticContainerInheritanceStyle::font_size),
        )?,
        line_height: positive(
            BasicStyleProperty::LineHeight,
            parent.and_then(SemanticContainerInheritanceStyle::line_height),
        )?,
        font_families,
    })
}

impl StyleSheet {
    /// Cascades one private producer-composed vector block through the shared
    /// precedence engine, then closes it into the registry-specific receipt.
    pub fn cascade_precomposed_vector_style(
        &self,
        kind: PrecomposedVectorStyleKind,
        classes: &[String],
        parent: Option<&SemanticContainerInheritanceStyle>,
    ) -> Result<PrecomposedVectorComputedStyleReceipt, StyleValidationError> {
        self.validate_precomposed_vector_styles()?;
        let computed = self.cascade_precomposed_vector_validated(kind.as_str(), classes)?;
        let inheritance = close_semantic_inheritance_style(&computed, parent)?;
        if inheritance.block_style.width != MachineFigureWidth::Auto {
            return Err(StyleValidationError::InapplicableProperty);
        }
        let page_name = computed.page_name()?;
        let supplement = match kind {
            PrecomposedVectorStyleKind::MathVectorBlock => {
                PrecomposedVectorStyleSupplement::EquationNumber(
                    PrecomposedVectorEquationNumberTextStyle {
                        font_families: inheritance.font_families,
                        font_size: inheritance.font_size,
                        line_height: inheritance.line_height,
                    },
                )
            }
            PrecomposedVectorStyleKind::VectorFigure => {
                PrecomposedVectorStyleSupplement::FigureCaption {
                    keep_caption: inheritance.block_style.keep_caption,
                }
            }
        };
        let mut receipt = PrecomposedVectorComputedStyleReceipt {
            registry_version: PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION,
            kind,
            block: inheritance.block_style,
            page_name,
            supplement,
            canonical_jcs: String::new(),
            fingerprint: [0; 32],
            _binding: PrecomposedVectorStyleBinding,
        };
        receipt.canonical_jcs = encode_precomposed_vector_computed_style(&receipt);
        receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
        receipt
            .verify_for(kind)
            .map_err(|_| StyleValidationError::InapplicableProperty)?;
        Ok(receipt)
    }
}

fn encode_precomposed_vector_computed_style(
    receipt: &PrecomposedVectorComputedStyleReceipt,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION);
    output.push_str(",\"end_indent\":");
    output.push_str(&receipt.block.end_indent.get().raw().to_string());
    match &receipt.supplement {
        PrecomposedVectorStyleSupplement::EquationNumber(style) => {
            output.push_str(",\"equation_number_text\":{\"font_families\":");
            match &style.font_families {
                Some(families) => {
                    output.push('[');
                    for (index, family) in families.iter().enumerate() {
                        if index > 0 {
                            output.push(',');
                        }
                        push_jcs_string(&mut output, family);
                    }
                    output.push(']');
                }
                None => output.push_str("null"),
            }
            output.push_str(",\"font_size\":");
            push_optional_positive_length(&mut output, style.font_size);
            output.push_str(",\"line_height\":");
            push_optional_positive_length(&mut output, style.line_height);
            output.push('}');
        }
        PrecomposedVectorStyleSupplement::FigureCaption { keep_caption } => {
            output.push_str(",\"keep_caption\":");
            output.push_str(if *keep_caption { "true" } else { "false" });
        }
    }
    output.push_str(",\"keep_with_next\":");
    output.push_str(if receipt.block.keep_with_next {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, receipt.kind.as_str());
    output.push_str(",\"page\":");
    match &receipt.page_name {
        Some(page) => push_jcs_string(&mut output, page.as_str()),
        None => output.push_str("null"),
    }
    output.push_str(",\"space_after\":");
    output.push_str(&receipt.block.space_after.get().raw().to_string());
    output.push_str(",\"space_before\":");
    output.push_str(&receipt.block.space_before.get().raw().to_string());
    output.push_str(",\"start_indent\":");
    output.push_str(&receipt.block.start_indent.get().raw().to_string());
    output.push_str(",\"text_align\":");
    push_jcs_string(&mut output, receipt.block.text_align.as_str());
    output.push('}');
    output
}

fn push_optional_positive_length(output: &mut String, value: Option<PositiveLength>) {
    match value {
        Some(value) => output.push_str(&value.get().raw().to_string()),
        None => output.push_str("null"),
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingMathStyleKind {
    Inline,
    Display,
}

impl StagingMathStyleKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline_math",
            Self::Display => "display_math",
        }
    }
}

/// Closed style consumed by the private M4 math binding. It contains no raw
/// declaration channel, and therefore source commands cannot alter layout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathComputedStyle {
    kind: StagingMathStyleKind,
    block: ComputedMachineBlockStyle,
    font_families: Vec<String>,
    font_size: PositiveLength,
    line_height: PositiveLength,
    page_name: Option<PageName>,
}

impl StagingMathComputedStyle {
    pub const fn kind(&self) -> StagingMathStyleKind {
        self.kind
    }
    pub const fn block_style(&self) -> ComputedMachineBlockStyle {
        self.block
    }
    pub fn font_families(&self) -> &[String] {
        &self.font_families
    }
    pub const fn font_size(&self) -> PositiveLength {
        self.font_size
    }
    pub const fn line_height(&self) -> PositiveLength {
        self.line_height
    }
    pub const fn page_name(&self) -> Option<&PageName> {
        self.page_name.as_ref()
    }
}

/// Inline math has no selector or classes. The owning text/block cascade is
/// already closed before this conversion, so the exact inherited font triple
/// is the only independent math style input.
pub fn close_staging_inline_math_style(
    owner: &SemanticContainerInheritanceStyle,
) -> Result<StagingMathComputedStyle, StyleValidationError> {
    close_staging_math_style(StagingMathStyleKind::Inline, owner.clone(), None)
}

/// Syntax passes an isolated sheet where `display_math` has been remapped to
/// the paragraph-shaped applicability engine. Known figure-only properties
/// remain typed but fail closed for math.
pub fn cascade_staging_display_math_style(
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
) -> Result<StagingMathComputedStyle, StyleValidationError> {
    sheet.validate_basic_document_styles()?;
    let computed = sheet.cascade_validated(BasicStyleBlockKind::Paragraph.as_str(), classes)?;
    let inheritance = close_semantic_inheritance_style(&computed, parent)?;
    if inheritance.block_style.width() != MachineFigureWidth::Auto
        || !inheritance.block_style.keep_caption()
    {
        return Err(StyleValidationError::InapplicableProperty);
    }
    close_staging_math_style(
        StagingMathStyleKind::Display,
        inheritance,
        computed.page_name()?,
    )
}

fn close_staging_math_style(
    kind: StagingMathStyleKind,
    inheritance: SemanticContainerInheritanceStyle,
    page_name: Option<PageName>,
) -> Result<StagingMathComputedStyle, StyleValidationError> {
    let font_families = inheritance
        .font_families
        .filter(|families| valid_font_family_list(families))
        .ok_or(StyleValidationError::MissingTextProperty)?;
    let font_size = inheritance
        .font_size
        .ok_or(StyleValidationError::MissingTextProperty)?;
    let line_height = inheritance
        .line_height
        .ok_or(StyleValidationError::MissingTextProperty)?;
    Ok(StagingMathComputedStyle {
        kind,
        block: inheritance.block_style,
        font_families,
        font_size,
        line_height,
        page_name,
    })
}

/// Complete computed style for a generated list marker. Marker shaping uses
/// the same required text triple as paragraph text, while marker placement
/// consumes the list block's typed spacing and indents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputedMachineListStyle {
    block: ComputedMachineBlockStyle,
    font_families: Vec<String>,
    font_size: PositiveLength,
    line_height: PositiveLength,
}

impl ComputedMachineListStyle {
    pub const fn block(&self) -> ComputedMachineBlockStyle {
        self.block
    }

    pub fn font_families(&self) -> &[String] {
        &self.font_families
    }

    pub const fn font_size(&self) -> PositiveLength {
        self.font_size
    }

    pub const fn line_height(&self) -> PositiveLength {
        self.line_height
    }
}

impl ComputedMachineBlockStyle {
    pub const fn space_before(self) -> NonNegativeLength {
        self.space_before
    }
    pub const fn space_after(self) -> NonNegativeLength {
        self.space_after
    }
    pub const fn start_indent(self) -> NonNegativeLength {
        self.start_indent
    }
    pub const fn end_indent(self) -> NonNegativeLength {
        self.end_indent
    }
    pub const fn text_align(self) -> MachineTextAlign {
        self.text_align
    }
    pub const fn width(self) -> MachineFigureWidth {
        self.width
    }
    pub const fn keep_with_next(self) -> bool {
        self.keep_with_next
    }
    pub const fn keep_caption(self) -> bool {
        self.keep_caption
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputedStyle {
    properties: BTreeMap<String, StyleValue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedTextStyle {
    font_families: Vec<String>,
    font_face_id: FontFaceId,
    font_size: PositiveLength,
    line_height: PositiveLength,
}
impl ResolvedTextStyle {
    pub fn try_from_computed(
        computed: &ComputedStyle,
        admitted: AdmittedResourceLedgerToken<'_>,
    ) -> Result<Self, StyleValidationError> {
        let font_families = match computed.properties.get("font_family") {
            Some(StyleValue::FontFamilyList(families)) if !families.is_empty() => families.clone(),
            _ => return Err(StyleValidationError::MissingTextProperty),
        };
        let font_size = match computed.properties.get("font_size") {
            Some(StyleValue::Length(value)) => {
                PositiveLength::new(*value).ok_or(StyleValidationError::InvalidDeclarationValue)?
            }
            _ => return Err(StyleValidationError::MissingTextProperty),
        };
        let line_height = match computed.properties.get("line_height") {
            Some(StyleValue::Length(value)) => {
                PositiveLength::new(*value).ok_or(StyleValidationError::InvalidDeclarationValue)?
            }
            _ => return Err(StyleValidationError::MissingTextProperty),
        };
        let font_face_id = admitted
            .ledger()
            .font_families()
            .resolve(&font_families)
            .map_err(|_| StyleValidationError::UnknownFontFamily)?;
        Ok(Self {
            font_families,
            font_face_id,
            font_size,
            line_height,
        })
    }
    pub fn font_families(&self) -> &[String] {
        &self.font_families
    }
    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub const fn font_size(&self) -> PositiveLength {
        self.font_size
    }
    pub const fn line_height(&self) -> PositiveLength {
        self.line_height
    }
}

type SelectorSpecificity = (u8, u32, u8);
type CascadePriority = (bool, SelectorSpecificity, u32, u32, u32);
type CascadeWinner = (CascadePriority, StyleValue);

impl ComputedStyle {
    pub fn properties(&self) -> &BTreeMap<String, StyleValue> {
        &self.properties
    }

    /// Resolves the closed basic-document figure width into its typed domain.
    /// Layout consumers never need to compare or reinterpret the declaration
    /// name or its wire value.
    pub fn basic_figure_width(&self) -> Result<MachineFigureWidth, StyleValidationError> {
        match self.properties.get(BasicStyleProperty::Width.as_str()) {
            None => Ok(MachineFigureWidth::Auto),
            Some(StyleValue::Keyword(value)) if value == "auto" => Ok(MachineFigureWidth::Auto),
            Some(StyleValue::Length(value)) => PositiveLength::new(*value)
                .map(MachineFigureWidth::Length)
                .ok_or(StyleValidationError::InvalidDeclarationValue),
            Some(_) => Err(StyleValidationError::InvalidDeclarationValue),
        }
    }

    /// Resolves the closed basic-document next-block keep into its typed
    /// boolean domain. Pagination consumers do not inspect declaration names
    /// or reinterpret raw style values.
    pub fn basic_keep_with_next(&self) -> Result<bool, StyleValidationError> {
        match self
            .properties
            .get(BasicStyleProperty::KeepWithNext.as_str())
        {
            None => Ok(false),
            Some(StyleValue::Boolean(value)) => Ok(*value),
            Some(_) => Err(StyleValidationError::InvalidDeclarationValue),
        }
    }

    /// Resolves the computed `page` property. Absence and `auto` both mean no named page.
    pub fn page_name(&self) -> Result<Option<PageName>, StyleValidationError> {
        match self.properties.get("page") {
            None => Ok(None),
            Some(StyleValue::Keyword(value)) if value == "auto" => Ok(None),
            Some(StyleValue::Text(value)) => PageName::new(value.clone())
                .map(Some)
                .map_err(|_| StyleValidationError::InvalidPageProperty),
            Some(_) => Err(StyleValidationError::InvalidPageProperty),
        }
    }
}

fn close_machine_block_style(
    computed: &ComputedStyle,
    parent: Option<&ComputedMachineBlockStyle>,
) -> Result<ComputedMachineBlockStyle, StyleValidationError> {
    let nonnegative = |name: &str| match computed.properties.get(name) {
        None => Ok(NonNegativeLength::ZERO),
        Some(StyleValue::Length(value)) => {
            NonNegativeLength::new(*value).ok_or(StyleValidationError::InvalidDeclarationValue)
        }
        Some(_) => Err(StyleValidationError::InvalidDeclarationValue),
    };
    let text_align = match computed
        .properties
        .get(BasicStyleProperty::TextAlign.as_str())
    {
        Some(StyleValue::Keyword(value)) => match value.as_str() {
            "start" => MachineTextAlign::Start,
            "end" => MachineTextAlign::End,
            "center" => MachineTextAlign::Center,
            _ => return Err(StyleValidationError::InvalidDeclarationValue),
        },
        None => parent
            .map(|style| style.text_align)
            .unwrap_or(MachineTextAlign::Start),
        Some(_) => return Err(StyleValidationError::InvalidDeclarationValue),
    };
    let boolean = |name: &str, initial: bool| match computed.properties.get(name) {
        None => Ok(initial),
        Some(StyleValue::Boolean(value)) => Ok(*value),
        Some(_) => Err(StyleValidationError::InvalidDeclarationValue),
    };
    Ok(ComputedMachineBlockStyle {
        space_before: nonnegative(BasicStyleProperty::SpaceBefore.as_str())?,
        space_after: nonnegative(BasicStyleProperty::SpaceAfter.as_str())?,
        start_indent: nonnegative(BasicStyleProperty::StartIndent.as_str())?,
        end_indent: nonnegative(BasicStyleProperty::EndIndent.as_str())?,
        text_align,
        width: computed.basic_figure_width()?,
        keep_with_next: computed.basic_keep_with_next()?,
        keep_caption: boolean(BasicStyleProperty::KeepCaption.as_str(), true)?,
    })
}

impl StyleSheet {
    pub fn cascade(
        &self,
        block_type: &str,
        classes: &[String],
    ) -> Result<ComputedStyle, StyleValidationError> {
        self.validate()?;
        self.cascade_validated(block_type, classes)
    }

    /// Cascades the complete closed contract-1.2 declaration set while
    /// retaining the ordinary computed text properties used by shaping.
    pub fn cascade_basic_document(
        &self,
        block_type: &str,
        classes: &[String],
    ) -> Result<ComputedStyle, StyleValidationError> {
        // The cascade consumer is shared with `table-1`. Profile preflight
        // remains the authority which prevents a basic-document job from
        // introducing a table selector; accepting that selector here lets
        // ordinary M2 descendants be styled in a table-profile sheet.
        self.validate_table_document_styles()?;
        if block_type != "table" && BasicStyleBlockKind::from_str(block_type).is_none() {
            return Err(StyleValidationError::InvalidSelector(
                SelectorError::InvalidBlockType,
            ));
        }
        self.cascade_validated(block_type, classes)
    }

    /// Cascades contract 1.2 values and immediately closes them into the
    /// fixed-point/enum/boolean domain consumed by layout. Only `text_align`
    /// may inherit from the typed flow-owner parent.
    pub fn cascade_basic_document_style(
        &self,
        block: BasicStyleBlockKind,
        classes: &[String],
        parent: Option<&ComputedMachineBlockStyle>,
    ) -> Result<ComputedMachineBlockStyle, StyleValidationError> {
        self.validate_table_document_styles()?;
        let computed = self.cascade_validated(block.as_str(), classes)?;
        close_machine_block_style(&computed, parent)
    }

    /// Cascades only existing M2 block-placement values for a table. The
    /// returned type has no table-specific raw-property channel; text
    /// alignment, figure width, and caption keep remain their fixed initials.
    pub fn cascade_table_document_style(
        &self,
        classes: &[String],
    ) -> Result<ComputedMachineBlockStyle, StyleValidationError> {
        self.validate_table_document_styles()?;
        let computed = self.cascade_validated("table", classes)?;
        close_machine_block_style(&computed, None)
    }

    /// Cascades a cell paragraph against the sealed table parent using the
    /// same M2 computed-value closure as ordinary paragraph flows.
    pub fn cascade_table_cell_paragraph_style(
        &self,
        classes: &[String],
        table_parent: &ComputedMachineBlockStyle,
    ) -> Result<ComputedMachineBlockStyle, StyleValidationError> {
        self.validate_table_document_styles()?;
        let computed = self.cascade_validated(BasicStyleBlockKind::Paragraph.as_str(), classes)?;
        close_machine_block_style(&computed, Some(table_parent))
    }

    /// Cascades the closed list-marker style. Missing marker text properties
    /// fail here rather than being replaced with backend defaults.
    pub fn cascade_basic_document_list_style(
        &self,
        classes: &[String],
    ) -> Result<ComputedMachineListStyle, StyleValidationError> {
        let block = self.cascade_basic_document_style(BasicStyleBlockKind::List, classes, None)?;
        let computed = self.cascade_validated(BasicStyleBlockKind::List.as_str(), classes)?;
        let font_families = match computed
            .properties
            .get(BasicStyleProperty::FontFamily.as_str())
        {
            Some(StyleValue::FontFamilyList(families)) if !families.is_empty() => families.clone(),
            _ => return Err(StyleValidationError::MissingTextProperty),
        };
        let positive =
            |property: BasicStyleProperty| match computed.properties.get(property.as_str()) {
                Some(StyleValue::Length(value)) => {
                    PositiveLength::new(*value).ok_or(StyleValidationError::InvalidDeclarationValue)
                }
                _ => Err(StyleValidationError::MissingTextProperty),
            };
        Ok(ComputedMachineListStyle {
            block,
            font_families,
            font_size: positive(BasicStyleProperty::FontSize)?,
            line_height: positive(BasicStyleProperty::LineHeight)?,
        })
    }

    fn cascade_validated(
        &self,
        block_type: &str,
        classes: &[String],
    ) -> Result<ComputedStyle, StyleValidationError> {
        self.cascade_validated_with(block_type, classes, selector_matches)
    }

    fn cascade_precomposed_vector_validated(
        &self,
        block_type: &str,
        classes: &[String],
    ) -> Result<ComputedStyle, StyleValidationError> {
        self.cascade_validated_with(block_type, classes, precomposed_vector_selector_matches)
    }

    fn cascade_validated_with(
        &self,
        block_type: &str,
        classes: &[String],
        matches_selector: fn(&str, &str, &[String]) -> Result<bool, SelectorError>,
    ) -> Result<ComputedStyle, StyleValidationError> {
        let by_id: BTreeMap<&StyleId, &StyleRule> = self
            .rules
            .iter()
            .map(|rule| (&rule.style_id, rule))
            .collect();
        let mut winners: BTreeMap<String, CascadeWinner> = BTreeMap::new();
        for matched in &self.rules {
            if !matches_selector(&matched.selector, block_type, classes)
                .map_err(StyleValidationError::InvalidSelector)?
            {
                continue;
            }
            let class_count = cascade_priority_index(matched.selector.matches('.').count())?;
            let specificity = (0, class_count, 1);
            let mut chain = vec![matched];
            let mut current = matched.extends.as_ref();
            while let Some(parent) = current {
                let rule = by_id
                    .get(parent)
                    .ok_or(StyleValidationError::UnknownParent)?;
                chain.push(rule);
                current = rule.extends.as_ref();
            }
            chain.reverse();
            for (depth, origin) in chain.iter().enumerate() {
                for (declaration_order, declaration) in origin.declarations.iter().enumerate() {
                    let key = (
                        declaration.important,
                        specificity,
                        matched.source_order,
                        cascade_priority_index(depth)?,
                        cascade_priority_index(declaration_order)?,
                    );
                    let replace = winners
                        .get(&declaration.name)
                        .map_or(true, |(winner, _)| key > *winner);
                    if replace {
                        winners.insert(declaration.name.clone(), (key, declaration.value.clone()));
                    }
                }
            }
        }
        Ok(ComputedStyle {
            properties: winners
                .into_iter()
                .map(|(name, (_, value))| (name, value))
                .collect(),
        })
    }
}

fn cascade_priority_index(value: usize) -> Result<u32, StyleValidationError> {
    u32::try_from(value).map_err(|_| StyleValidationError::CascadePriorityOverflow)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageParity {
    Any,
    Odd,
    Even,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageMasterRule {
    pub master_id: MasterId,
    pub parity: PageParity,
    pub first: Option<bool>,
    pub named_page: Option<PageName>,
    pub source_order: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageSelectionContext {
    page_index: u32,
    physical_page_number: NonZeroU32,
    named_page: Option<PageName>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageSelectionError {
    PageNumberOverflow,
}
impl PageSelectionContext {
    pub fn new(page_index: u32, named_page: Option<PageName>) -> Result<Self, PageSelectionError> {
        let physical_page_number = page_index
            .checked_add(1)
            .and_then(NonZeroU32::new)
            .ok_or(PageSelectionError::PageNumberOverflow)?;
        Ok(Self {
            page_index,
            physical_page_number,
            named_page,
        })
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn named_page(&self) -> Option<&PageName> {
        self.named_page.as_ref()
    }
    pub const fn is_first(&self) -> bool {
        self.page_index == 0
    }
    pub const fn is_odd(&self) -> bool {
        self.physical_page_number.get() % 2 == 1
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageMaster {
    pub master_id: MasterId,
    pub width: PositiveLength,
    pub height: PositiveLength,
    pub body: Rect,
    pub header: Option<Rect>,
    pub footer: Option<Rect>,
    pub footnote: Option<Rect>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageMasterSet {
    pub default_master_id: MasterId,
    pub masters: Vec<PageMaster>,
    pub selection_rules: Vec<PageMasterRule>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageMasterValidationError {
    DuplicateMasterId,
    NonCanonicalMasterOrder,
    UnknownDefaultMaster,
    UnknownRuleMaster,
    SourceOrderMismatch,
    FrameOutOfBounds,
}

impl PageMasterSet {
    pub fn validate(&self) -> Result<(), PageMasterValidationError> {
        let mut master_ids = BTreeSet::new();
        let mut previous_master: Option<&MasterId> = None;
        for master in &self.masters {
            if previous_master.is_some_and(|previous| previous >= &master.master_id) {
                return Err(PageMasterValidationError::NonCanonicalMasterOrder);
            }
            if !master_ids.insert(&master.master_id) {
                return Err(PageMasterValidationError::DuplicateMasterId);
            }
            previous_master = Some(&master.master_id);
            for frame in [
                Some(master.body),
                master.header,
                master.footer,
                master.footnote,
            ]
            .into_iter()
            .flatten()
            {
                if !frame_is_within_page(master, frame) {
                    return Err(PageMasterValidationError::FrameOutOfBounds);
                }
            }
        }
        if !master_ids.contains(&self.default_master_id) {
            return Err(PageMasterValidationError::UnknownDefaultMaster);
        }
        for (index, rule) in self.selection_rules.iter().enumerate() {
            if rule.source_order
                != u32::try_from(index)
                    .map_err(|_| PageMasterValidationError::SourceOrderMismatch)?
            {
                return Err(PageMasterValidationError::SourceOrderMismatch);
            }
            if !master_ids.contains(&rule.master_id) {
                return Err(PageMasterValidationError::UnknownRuleMaster);
            }
        }
        Ok(())
    }

    pub fn select(
        &self,
        context: &PageSelectionContext,
    ) -> Result<&PageMaster, PageMasterValidationError> {
        self.validate()?;
        let winner = self
            .selection_rules
            .iter()
            .filter(|rule| page_rule_matches(rule, context))
            .max_by_key(|rule| {
                (
                    (
                        u8::from(rule.named_page.is_some()),
                        u8::from(rule.first.is_some()),
                        u8::from(rule.parity != PageParity::Any),
                    ),
                    rule.source_order,
                )
            });
        let master_id = winner
            .map(|rule| &rule.master_id)
            .unwrap_or(&self.default_master_id);
        self.masters
            .iter()
            .find(|master| &master.master_id == master_id)
            .ok_or(PageMasterValidationError::UnknownDefaultMaster)
    }
}

fn page_rule_matches(rule: &PageMasterRule, context: &PageSelectionContext) -> bool {
    let parity_matches = match rule.parity {
        PageParity::Any => true,
        PageParity::Odd => context.is_odd(),
        PageParity::Even => !context.is_odd(),
    };
    let first_matches = rule.first.map_or(true, |first| first == context.is_first());
    let name_matches = rule
        .named_page
        .as_ref()
        .map_or(true, |name| Some(name) == context.named_page());
    parity_matches && first_matches && name_matches
}

fn frame_is_within_page(master: &PageMaster, frame: Rect) -> bool {
    let x = frame.x().raw();
    let y = frame.y().raw();
    x >= 0
        && y >= 0
        && x.checked_add(frame.width().get().raw())
            .is_some_and(|end| end <= master.width.get().raw())
        && y.checked_add(frame.height().get().raw())
            .is_some_and(|end| end <= master.height.get().raw())
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_resource_admission::AdmittedResourceResolver;

    fn rule(style_id: &str, extends: Option<&str>, selector: &str, source_order: u32) -> StyleRule {
        StyleRule {
            style_id: StyleId::new(style_id).unwrap(),
            extends: extends.map(|parent| StyleId::new(parent).unwrap()),
            selector: selector.to_owned(),
            source_order,
            declarations: vec![],
        }
    }

    #[test]
    fn selector_uses_one_block_type_and_unique_classes() {
        assert!(validate_selector("heading.chapter.lead").is_ok());
        assert_eq!(
            validate_selector("heading.chapter.chapter"),
            Err(SelectorError::DuplicateClass)
        );
        assert_eq!(
            validate_selector("unknown.chapter"),
            Err(SelectorError::InvalidBlockType)
        );
        assert_eq!(
            validate_selector("heading.lead.chapter"),
            Err(SelectorError::NonCanonicalClassOrder)
        );
    }

    #[test]
    fn style_sheet_requires_dense_order_and_an_acyclic_known_parent_graph() {
        let valid = StyleSheet {
            rules: vec![
                rule("base", None, "paragraph", 0),
                rule("derived", Some("base"), "paragraph.lead", 1),
            ],
        };
        assert!(valid.validate().is_ok());

        let unknown = StyleSheet {
            rules: vec![rule("derived", Some("missing"), "paragraph", 0)],
        };
        assert_eq!(unknown.validate(), Err(StyleValidationError::UnknownParent));

        let cycle = StyleSheet {
            rules: vec![
                rule("a", Some("b"), "paragraph", 0),
                rule("b", Some("a"), "paragraph", 1),
            ],
        };
        assert_eq!(
            cycle.validate(),
            Err(StyleValidationError::InheritanceCycle)
        );
    }

    #[test]
    fn declarations_validate_and_cascade_with_documented_precedence() {
        let mut base = rule("base", None, "paragraph", 0);
        base.declarations.push(Declaration {
            name: "page".to_owned(),
            value: StyleValue::Keyword("auto".to_owned()),
            important: false,
        });
        base.declarations.extend([
            Declaration {
                name: "font_family".to_owned(),
                value: StyleValue::FontFamilyList(vec!["Body".to_owned()]),
                important: false,
            },
            Declaration {
                name: "font_size".to_owned(),
                value: StyleValue::Length(Length::from_raw(12).unwrap()),
                important: false,
            },
            Declaration {
                name: "line_height".to_owned(),
                value: StyleValue::Length(Length::from_raw(15).unwrap()),
                important: false,
            },
        ]);
        let mut derived = rule("derived", Some("base"), "paragraph.lead", 1);
        derived.declarations.push(Declaration {
            name: "page".to_owned(),
            value: StyleValue::Text("chapter".to_owned()),
            important: false,
        });
        let sheet = StyleSheet {
            rules: vec![base, derived],
        };
        let computed = sheet.cascade("paragraph", &["lead".to_owned()]).unwrap();
        assert_eq!(
            computed.page_name().unwrap(),
            Some(PageName::new("chapter").unwrap())
        );
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let admitted = AdmittedResourceResolver::new_empty(&limits)
            .unwrap()
            .finish()
            .unwrap();
        assert_eq!(
            ResolvedTextStyle::try_from_computed(&computed, admitted.token()),
            Err(StyleValidationError::UnknownFontFamily)
        );

        let mut invalid = rule("invalid", None, "paragraph", 0);
        invalid.declarations.push(Declaration {
            name: "Bad Name".to_owned(),
            value: StyleValue::Integer(0),
            important: false,
        });
        assert_eq!(
            StyleSheet {
                rules: vec![invalid]
            }
            .validate(),
            Err(StyleValidationError::InvalidDeclarationName)
        );

        let mut invalid_page = rule("invalid-page", None, "paragraph", 0);
        invalid_page.declarations.push(Declaration {
            name: "page".to_owned(),
            value: StyleValue::Keyword("chapter".to_owned()),
            important: false,
        });
        assert_eq!(
            StyleSheet {
                rules: vec![invalid_page]
            }
            .validate(),
            Err(StyleValidationError::InvalidPageProperty)
        );

        let mut typo = rule("typo", None, "paragraph", 0);
        typo.declarations.push(Declaration {
            name: "font_famliy".to_owned(),
            value: StyleValue::FontFamilyList(vec!["Body".to_owned()]),
            important: false,
        });
        assert_eq!(
            StyleSheet { rules: vec![typo] }.validate(),
            Err(StyleValidationError::UnknownProperty)
        );

        let mut duplicate = rule("duplicate", None, "paragraph", 0);
        duplicate.declarations = vec![
            Declaration {
                name: "font_size".to_owned(),
                value: StyleValue::Length(Length::from_raw(12).unwrap()),
                important: false,
            },
            Declaration {
                name: "font_size".to_owned(),
                value: StyleValue::Length(Length::from_raw(14).unwrap()),
                important: false,
            },
        ];
        let duplicate_sheet = StyleSheet {
            rules: vec![duplicate],
        };
        assert!(duplicate_sheet.validate().is_ok());
        assert_eq!(
            duplicate_sheet
                .cascade("paragraph", &[])
                .unwrap()
                .properties()
                .get("font_size"),
            Some(&StyleValue::Length(Length::from_raw(14).unwrap()))
        );
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn cascade_priority_indices_fail_instead_of_saturating() {
        let too_large = usize::try_from(u64::from(u32::MAX) + 1).unwrap();
        assert_eq!(
            cascade_priority_index(too_large),
            Err(StyleValidationError::CascadePriorityOverflow)
        );
    }

    #[test]
    fn page_master_set_rejects_unknown_and_out_of_bounds_frames() {
        let length = |raw| PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap();
        let body = Rect::new(Length::ZERO, Length::ZERO, length(100), length(100));
        let mut masters = PageMasterSet {
            default_master_id: MasterId::new("default").unwrap(),
            masters: vec![PageMaster {
                master_id: MasterId::new("default").unwrap(),
                width: length(100),
                height: length(100),
                body,
                header: None,
                footer: None,
                footnote: None,
            }],
            selection_rules: vec![],
        };
        assert!(masters.validate().is_ok());
        masters.masters[0].body = Rect::new(
            Length::from_raw(1).unwrap(),
            Length::ZERO,
            length(100),
            length(100),
        );
        assert_eq!(
            masters.validate(),
            Err(PageMasterValidationError::FrameOutOfBounds)
        );
    }

    #[test]
    fn page_master_selection_uses_specificity_then_source_order() {
        let length = |raw| PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap();
        let rect = Rect::new(Length::ZERO, Length::ZERO, length(100), length(100));
        let master = |name: &str| PageMaster {
            master_id: MasterId::new(name).unwrap(),
            width: length(100),
            height: length(100),
            body: rect,
            header: None,
            footer: None,
            footnote: None,
        };
        let set = PageMasterSet {
            default_master_id: MasterId::new("default").unwrap(),
            masters: vec![master("chapter"), master("default"), master("odd")],
            selection_rules: vec![
                PageMasterRule {
                    master_id: MasterId::new("odd").unwrap(),
                    parity: PageParity::Odd,
                    first: None,
                    named_page: None,
                    source_order: 0,
                },
                PageMasterRule {
                    master_id: MasterId::new("chapter").unwrap(),
                    parity: PageParity::Odd,
                    first: None,
                    named_page: Some(PageName::new("chapter").unwrap()),
                    source_order: 1,
                },
            ],
        };
        let context =
            PageSelectionContext::new(0, Some(PageName::new("chapter").unwrap())).unwrap();
        assert_eq!(set.select(&context).unwrap().master_id.as_str(), "chapter");
    }

    fn machine_declaration(name: &str, value: StyleValue) -> Declaration {
        Declaration {
            name: name.to_owned(),
            value,
            important: false,
        }
    }

    #[test]
    fn machine_properties_have_closed_shape_initials_inheritance_and_override() {
        assert_eq!(BASIC_BLOCK_STYLE_PROPERTIES.len(), 8);
        assert_eq!(
            BASIC_BLOCK_STYLE_PROPERTIES
                .iter()
                .map(|entry| entry.property)
                .collect::<BTreeSet<_>>()
                .len(),
            8
        );
        assert!(BASIC_BLOCK_STYLE_PROPERTIES.iter().all(|entry| entry
            .property
            .applies_to(BasicStyleBlockKind::Paragraph)
            || entry.property.applies_to(BasicStyleBlockKind::Figure)));

        let mut base = rule("base", None, "paragraph", 0);
        base.declarations.extend([
            machine_declaration(
                "space_before",
                StyleValue::Length(Length::from_raw(0).unwrap()),
            ),
            machine_declaration(
                "space_after",
                StyleValue::Length(Length::from_raw(JSON_SAFE_INTEGER_MAX).unwrap()),
            ),
            machine_declaration(
                "start_indent",
                StyleValue::Length(Length::from_raw(2).unwrap()),
            ),
            machine_declaration(
                "end_indent",
                StyleValue::Length(Length::from_raw(3).unwrap()),
            ),
            machine_declaration("text_align", StyleValue::Keyword("end".to_owned())),
            machine_declaration("keep_with_next", StyleValue::Boolean(true)),
        ]);
        let mut override_rule = rule("override", None, "paragraph.lead", 1);
        override_rule.declarations.push(machine_declaration(
            "space_before",
            StyleValue::Length(Length::from_raw(7).unwrap()),
        ));
        let sheet = StyleSheet {
            rules: vec![base, override_rule],
        };
        let computed = sheet
            .cascade_basic_document_style(
                BasicStyleBlockKind::Paragraph,
                &["lead".to_owned()],
                None,
            )
            .unwrap();
        assert_eq!(computed.space_before().get().raw(), 7);
        assert_eq!(computed.space_after().get().raw(), JSON_SAFE_INTEGER_MAX);
        assert_eq!(computed.start_indent().get().raw(), 2);
        assert_eq!(computed.end_indent().get().raw(), 3);
        assert_eq!(computed.text_align(), MachineTextAlign::End);
        assert_eq!(computed.width(), MachineFigureWidth::Auto);
        assert!(computed.keep_with_next());
        assert!(computed.keep_caption());

        let inherited = StyleSheet { rules: vec![] }
            .cascade_basic_document_style(BasicStyleBlockKind::Heading, &[], Some(&computed))
            .unwrap();
        assert_eq!(inherited.text_align(), MachineTextAlign::End);
        assert_eq!(inherited.space_before(), NonNegativeLength::ZERO);
        assert!(!inherited.keep_with_next());
        assert!(Length::from_raw(JSON_SAFE_INTEGER_MAX.checked_add(1).unwrap()).is_none());
    }

    #[test]
    fn machine_properties_reject_wrong_tags_keywords_unknown_and_inapplicable_use() {
        for (name, value) in [
            ("space_before", StyleValue::Integer(0)),
            ("width", StyleValue::Length(Length::ZERO)),
            ("text_align", StyleValue::Keyword("justify".to_owned())),
            ("keep_caption", StyleValue::Keyword("true".to_owned())),
            ("future_property", StyleValue::Boolean(true)),
        ] {
            let mut invalid = rule("invalid", None, "paragraph", 0);
            invalid.declarations.push(machine_declaration(name, value));
            assert!(StyleSheet {
                rules: vec![invalid]
            }
            .validate_basic_document_style_shape()
            .is_err());
        }

        let mut invalid = rule("figure-align", None, "figure", 0);
        invalid.declarations.push(machine_declaration(
            "text_align",
            StyleValue::Keyword("center".to_owned()),
        ));
        assert_eq!(
            StyleSheet {
                rules: vec![invalid]
            }
            .validate_basic_document_styles(),
            Err(StyleValidationError::InapplicableProperty)
        );

        let mut staging = rule("staging", None, "paragraph", 0);
        staging.declarations.push(machine_declaration(
            "space_before",
            StyleValue::Length(Length::ZERO),
        ));
        assert_eq!(
            StyleSheet {
                rules: vec![staging]
            }
            .validate(),
            Err(StyleValidationError::UnknownProperty)
        );
    }

    #[test]
    fn machine_properties_figure_width_and_caption_keep_are_typed() {
        let mut figure = rule("figure", None, "figure", 0);
        figure.declarations.extend([
            machine_declaration("width", StyleValue::Length(Length::from_raw(100).unwrap())),
            machine_declaration("keep_caption", StyleValue::Boolean(false)),
        ]);
        let computed = StyleSheet {
            rules: vec![figure],
        }
        .cascade_basic_document_style(BasicStyleBlockKind::Figure, &[], None)
        .unwrap();
        assert_eq!(
            computed.width(),
            MachineFigureWidth::Length(
                PositiveLength::new(Length::from_raw(100).unwrap()).unwrap()
            )
        );
        assert!(!computed.keep_caption());
    }

    #[test]
    fn table_style_profile_reuses_only_typed_m2_placement_properties() {
        let mut table = rule("table", None, "table.matrix", 0);
        table.declarations.extend([
            machine_declaration(
                "start_indent",
                StyleValue::Length(Length::from_raw(2).unwrap()),
            ),
            machine_declaration(
                "end_indent",
                StyleValue::Length(Length::from_raw(3).unwrap()),
            ),
            machine_declaration("page", StyleValue::Keyword("auto".to_owned())),
            machine_declaration("keep_with_next", StyleValue::Boolean(true)),
        ]);
        let sheet = StyleSheet { rules: vec![table] };
        assert_eq!(
            sheet.validate_basic_document_styles(),
            Err(StyleValidationError::InapplicableProperty)
        );
        sheet.validate_table_document_styles().unwrap();
        let computed = sheet
            .cascade_table_document_style(&["matrix".to_owned()])
            .unwrap();
        assert_eq!(computed.start_indent().get().raw(), 2);
        assert_eq!(computed.end_indent().get().raw(), 3);
        assert_eq!(computed.text_align(), MachineTextAlign::Start);
        assert_eq!(computed.width(), MachineFigureWidth::Auto);
        assert!(computed.keep_with_next());

        let mut paragraph = rule("paragraph", None, "paragraph", 1);
        paragraph.declarations.push(machine_declaration(
            "text_align",
            StyleValue::Keyword("end".to_owned()),
        ));
        let with_paragraph = StyleSheet {
            rules: vec![sheet.rules[0].clone(), paragraph],
        };
        let child = with_paragraph
            .cascade_table_cell_paragraph_style(&[], &computed)
            .unwrap();
        assert_eq!(child.text_align(), MachineTextAlign::End);

        for (name, value, expected) in [
            (
                "text_align",
                StyleValue::Keyword("center".to_owned()),
                StyleValidationError::InapplicableProperty,
            ),
            (
                "page",
                StyleValue::Text("chapter".to_owned()),
                StyleValidationError::InvalidPageProperty,
            ),
        ] {
            let mut invalid = rule("invalid", None, "table", 0);
            invalid.declarations.push(machine_declaration(name, value));
            assert_eq!(
                StyleSheet {
                    rules: vec![invalid]
                }
                .validate_table_document_styles(),
                Err(expected)
            );
        }
    }

    #[test]
    fn math_styles_inherit_inline_text_and_close_display_applicability() {
        let mut owner = rule("math-owner", None, "paragraph", 0);
        owner.declarations.extend([
            machine_declaration(
                "font_family",
                StyleValue::FontFamilyList(vec!["Math".to_owned()]),
            ),
            machine_declaration(
                "font_size",
                StyleValue::Length(Length::from_raw(12 * 65_536).unwrap()),
            ),
            machine_declaration(
                "line_height",
                StyleValue::Length(Length::from_raw(16 * 65_536).unwrap()),
            ),
            machine_declaration(
                "space_before",
                StyleValue::Length(Length::from_raw(2 * 65_536).unwrap()),
            ),
        ]);
        let owner = cascade_staging_semantic_container_style(
            SemanticContainerStyleKind::Result,
            &[],
            &StyleSheet { rules: vec![owner] },
            None,
        )
        .unwrap();
        let inline = close_staging_inline_math_style(owner.inheritance_style()).unwrap();
        assert_eq!(inline.kind(), StagingMathStyleKind::Inline);
        assert_eq!(inline.font_families(), ["Math"]);
        assert_eq!(inline.font_size().get().raw(), 12 * 65_536);
        assert_eq!(inline.line_height().get().raw(), 16 * 65_536);
        assert_eq!(inline.block_style().space_before().get().raw(), 2 * 65_536);

        let mut display = rule("equation", None, "paragraph.equation", 0);
        display.declarations.extend([
            machine_declaration("text_align", StyleValue::Keyword("center".to_owned())),
            machine_declaration(
                "start_indent",
                StyleValue::Length(Length::from_raw(4 * 65_536).unwrap()),
            ),
            machine_declaration(
                "end_indent",
                StyleValue::Length(Length::from_raw(4 * 65_536).unwrap()),
            ),
        ]);
        let display = cascade_staging_display_math_style(
            &["equation".to_owned()],
            &StyleSheet {
                rules: vec![display],
            },
            Some(owner.inheritance_style()),
        )
        .unwrap();
        assert_eq!(display.kind(), StagingMathStyleKind::Display);
        assert_eq!(display.font_families(), ["Math"]);
        assert_eq!(display.block_style().text_align(), MachineTextAlign::Center);
        assert_eq!(display.block_style().start_indent().get().raw(), 4 * 65_536);

        let mut inapplicable = rule("bad-math", None, "paragraph", 0);
        inapplicable.declarations.push(machine_declaration(
            "width",
            StyleValue::Length(Length::from_raw(10).unwrap()),
        ));
        assert_eq!(
            cascade_staging_display_math_style(
                &[],
                &StyleSheet {
                    rules: vec![inapplicable]
                },
                Some(owner.inheritance_style())
            ),
            Err(StyleValidationError::InapplicableProperty)
        );
    }

    #[test]
    fn precomposed_vector_styles_registry_is_exhaustive_and_basic_registry_stays_closed() {
        assert!(validate_precomposed_vector_selector("math_vector_block.equation").is_ok());
        assert!(validate_precomposed_vector_selector("vector_figure.diagram").is_ok());
        assert_eq!(
            validate_selector("math_vector_block.equation"),
            Err(SelectorError::InvalidBlockType)
        );

        let mut pairs = BTreeSet::new();
        for descriptor in PRECOMPOSED_VECTOR_STYLE_PROPERTIES {
            assert!(pairs.insert((descriptor.kind, descriptor.property)));
            assert_eq!(
                precomposed_vector_style_property_descriptor(descriptor.kind, descriptor.property),
                Some(descriptor)
            );
        }
        for kind in PRECOMPOSED_VECTOR_STYLE_KINDS {
            for property in PRECOMPOSED_VECTOR_STYLE_PROPERTY_DOMAIN {
                let accepted = precomposed_vector_style_property_descriptor(*kind, *property);
                let expected = !matches!(
                    (kind, property),
                    (_, PrecomposedVectorStyleProperty::Width)
                        | (
                            PrecomposedVectorStyleKind::MathVectorBlock,
                            PrecomposedVectorStyleProperty::KeepCaption,
                        )
                        | (
                            PrecomposedVectorStyleKind::VectorFigure,
                            PrecomposedVectorStyleProperty::FontFamily
                                | PrecomposedVectorStyleProperty::FontSize
                                | PrecomposedVectorStyleProperty::LineHeight,
                        )
                );
                assert_eq!(accepted.is_some(), expected, "{kind:?} {property:?}");
            }
        }

        let basic = StyleSheet {
            rules: vec![rule("new-kind", None, "math_vector_block", 0)],
        };
        assert_eq!(
            basic.validate_basic_document_styles(),
            Err(StyleValidationError::InvalidSelector(
                SelectorError::InvalidBlockType
            ))
        );

        for (selector, property, value) in [
            (
                "math_vector_block",
                "width",
                StyleValue::Length(Length::from_raw(1).unwrap()),
            ),
            (
                "math_vector_block",
                "keep_caption",
                StyleValue::Boolean(true),
            ),
            (
                "vector_figure",
                "font_size",
                StyleValue::Length(Length::from_raw(1).unwrap()),
            ),
        ] {
            let mut invalid = rule("invalid", None, selector, 0);
            invalid
                .declarations
                .push(machine_declaration(property, value));
            assert_eq!(
                StyleSheet {
                    rules: vec![invalid]
                }
                .validate_precomposed_vector_styles(),
                Err(StyleValidationError::InapplicableProperty)
            );
        }

        let mut unknown = rule("unknown", None, "vector_figure", 0);
        unknown.declarations.push(machine_declaration(
            "vector_color",
            StyleValue::Keyword("currentColor".to_owned()),
        ));
        assert_eq!(
            StyleSheet {
                rules: vec![unknown]
            }
            .validate_precomposed_vector_styles(),
            Err(StyleValidationError::UnknownProperty)
        );
    }

    #[test]
    fn precomposed_vector_styles_cascade_inheritance_override_and_typed_consumers() {
        let mut parent_rule = rule("owner", None, "paragraph", 0);
        parent_rule.declarations.extend([
            machine_declaration(
                "font_family",
                StyleValue::FontFamilyList(vec!["Math".to_owned()]),
            ),
            machine_declaration(
                "font_size",
                StyleValue::Length(Length::from_raw(10 * 65_536).unwrap()),
            ),
            machine_declaration(
                "line_height",
                StyleValue::Length(Length::from_raw(12 * 65_536).unwrap()),
            ),
            machine_declaration("text_align", StyleValue::Keyword("end".to_owned())),
        ]);
        let parent = cascade_staging_semantic_container_style(
            SemanticContainerStyleKind::Result,
            &[],
            &StyleSheet {
                rules: vec![parent_rule],
            },
            None,
        )
        .unwrap();

        let mut base = rule("base", None, "math_vector_block", 0);
        base.declarations.extend([
            machine_declaration(
                "space_before",
                StyleValue::Length(Length::from_raw(2).unwrap()),
            ),
            machine_declaration(
                "font_size",
                StyleValue::Length(Length::from_raw(11 * 65_536).unwrap()),
            ),
        ]);
        let mut specific = rule("specific", Some("base"), "math_vector_block.numbered", 1);
        specific.declarations.extend([
            machine_declaration(
                "space_before",
                StyleValue::Length(Length::from_raw(3).unwrap()),
            ),
            machine_declaration("keep_with_next", StyleValue::Boolean(true)),
        ]);
        let mut important = rule("important", None, "math_vector_block.numbered", 2);
        important.declarations.push(Declaration {
            name: "space_before".to_owned(),
            value: StyleValue::Length(Length::from_raw(4).unwrap()),
            important: true,
        });
        let mut math = StyleSheet {
            rules: vec![base, specific, important],
        }
        .cascade_precomposed_vector_style(
            PrecomposedVectorStyleKind::MathVectorBlock,
            &["numbered".to_owned()],
            Some(parent.inheritance_style()),
        )
        .unwrap();
        assert_eq!(
            math.registry_version(),
            PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION
        );
        assert_eq!(math.space_before().get().raw(), 4);
        assert_eq!(math.text_align(), MachineTextAlign::End);
        assert!(math.keep_with_next());
        assert_eq!(math.keep_caption(), None);
        let number = math.equation_number_text_style().unwrap();
        assert_eq!(number.font_families().unwrap(), ["Math"]);
        assert_eq!(number.font_size().unwrap().get().raw(), 11 * 65_536);
        assert_eq!(number.line_height().unwrap().get().raw(), 12 * 65_536);
        math.verify_for(PrecomposedVectorStyleKind::MathVectorBlock)
            .unwrap();
        assert_eq!(math.fingerprint(), sha256(math.canonical_jcs().as_bytes()));
        math.supplement = PrecomposedVectorStyleSupplement::FigureCaption { keep_caption: true };
        math.canonical_jcs = encode_precomposed_vector_computed_style(&math);
        math.fingerprint = sha256(math.canonical_jcs.as_bytes());
        assert_eq!(
            math.verify_for(PrecomposedVectorStyleKind::MathVectorBlock),
            Err(PrecomposedVectorStyleReceiptMismatch)
        );

        let mut figure = rule("figure", None, "vector_figure", 0);
        figure.declarations.extend([
            machine_declaration("keep_caption", StyleValue::Boolean(false)),
            machine_declaration("text_align", StyleValue::Keyword("center".to_owned())),
        ]);
        let figure = StyleSheet {
            rules: vec![figure],
        }
        .cascade_precomposed_vector_style(
            PrecomposedVectorStyleKind::VectorFigure,
            &[],
            Some(parent.inheritance_style()),
        )
        .unwrap();
        assert_eq!(figure.keep_caption(), Some(false));
        assert!(figure.equation_number_text_style().is_none());
        assert_eq!(figure.text_align(), MachineTextAlign::Center);

        let mismatch =
            require_precomposed_vector_style_registry(BASIC_BLOCK_STYLE_REGISTRY_VERSION)
                .unwrap_err();
        assert!(mismatch.to_string().starts_with("I9190:"));
    }

    #[test]
    fn precomposed_vector_styles_enforce_fixed_point_min_max_and_max_plus_one() {
        for (property, raw) in [
            ("space_before", 0),
            ("space_before", JSON_SAFE_INTEGER_MAX),
            ("font_size", 1),
            ("font_size", JSON_SAFE_INTEGER_MAX),
        ] {
            let mut accepted = rule("accepted", None, "math_vector_block", 0);
            accepted.declarations.push(machine_declaration(
                property,
                StyleValue::Length(Length::from_raw(raw).unwrap()),
            ));
            assert!(StyleSheet {
                rules: vec![accepted]
            }
            .validate_precomposed_vector_styles()
            .is_ok());
        }

        for (property, value) in [
            (
                "space_before",
                StyleValue::Length(Length::from_raw(-1).unwrap()),
            ),
            (
                "font_size",
                StyleValue::Length(Length::from_raw(0).unwrap()),
            ),
            (
                "space_before",
                StyleValue::Integer(JSON_SAFE_INTEGER_MAX + 1),
            ),
            ("font_size", StyleValue::Integer(JSON_SAFE_INTEGER_MAX + 1)),
        ] {
            let mut rejected = rule("rejected", None, "math_vector_block", 0);
            rejected
                .declarations
                .push(machine_declaration(property, value));
            assert_eq!(
                StyleSheet {
                    rules: vec![rejected]
                }
                .validate_precomposed_vector_styles(),
                Err(StyleValidationError::InvalidDeclarationValue)
            );
        }
    }
}
