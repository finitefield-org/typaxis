//! Initial Book-2 style operations with observable, fallible owned storage.
//! Requested payload bytes do not describe allocator capacity or live memory.
//! Work counts logical entry/allocation visits, visited or copied UTF-8 bytes,
//! and prepaid bounded closed-registry comparisons (12 properties, at most 32
//! bytes per literal). It is not an instruction or complete engine-work count.
#[cfg(feature = "book-v2-staging")]
use super::book_v2::{BookV2SemanticContainerComputedStyle, BookV2SemanticContainerStyleKind};
use super::*;
use std::convert::Infallible;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BookV2StylePreparationError<E> {
    Budget(E),
    Allocation,
    Style(StyleValidationError),
}
impl<E> From<StyleValidationError> for BookV2StylePreparationError<E> {
    fn from(error: StyleValidationError) -> Self {
        Self::Style(error)
    }
}
pub(super) fn legacy_error(error: BookV2StylePreparationError<Infallible>) -> StyleValidationError {
    match error {
        BookV2StylePreparationError::Style(error) => error,
        BookV2StylePreparationError::Budget(error) => match error {},
        BookV2StylePreparationError::Allocation => {
            unreachable!("legacy storage keeps its original allocation operations")
        }
    }
}
type Error<E> = BookV2StylePreparationError<E>;
pub(super) struct Context<'q, E> {
    observer: Option<&'q mut dyn FnMut(u64, u64, u64) -> Result<(), E>>,
}
impl Context<'_, Infallible> {
    pub(super) fn legacy() -> Self {
        Self { observer: None }
    }
}
impl<'q, E> Context<'q, E> {
    pub(super) fn counted(observer: &'q mut dyn FnMut(u64, u64, u64) -> Result<(), E>) -> Self {
        Self {
            observer: Some(observer),
        }
    }
    pub(super) fn counted_mode(&self) -> bool {
        self.observer.is_some()
    }
    pub(super) fn charge(&mut self, r: u64, s: u64, w: u64) -> Result<(), Error<E>> {
        if let Some(observer) = self.observer.as_deref_mut() {
            observer(r, s, w).map_err(Error::Budget)?;
        }
        Ok(())
    }
    pub(super) fn step(&mut self) -> Result<(), Error<E>> {
        self.charge(0, 0, 1)
    }
    pub(super) fn bytes(&mut self, n: usize) -> Result<(), Error<E>> {
        self.charge(0, 0, u64::try_from(n).map_err(|_| Error::Allocation)?)
    }
    pub(super) fn vec<T>(&mut self, n: usize) -> Result<Vec<T>, Error<E>> {
        let bytes = n
            .checked_mul(std::mem::size_of::<T>())
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(Error::Allocation)?;
        self.charge(u64::try_from(n).map_err(|_| Error::Allocation)?, bytes, 1)?;
        allocation_attempt(bytes)?;
        let mut result = Vec::new();
        result.try_reserve_exact(n).map_err(|_| Error::Allocation)?;
        Ok(result)
    }
    pub(super) fn string(&mut self, value: &str) -> Result<String, Error<E>> {
        if !self.counted_mode() {
            return Ok(value.to_owned());
        }
        self.charge(
            0,
            u64::try_from(value.len()).map_err(|_| Error::Allocation)?,
            1,
        )?;
        allocation_attempt(value.len() as u64)?;
        let mut result = String::new();
        result
            .try_reserve_exact(value.len())
            .map_err(|_| Error::Allocation)?;
        self.bytes(value.len())?;
        result.push_str(value);
        Ok(result)
    }
    pub(super) fn families(&mut self, values: &[String]) -> Result<Vec<String>, Error<E>> {
        if !self.counted_mode() {
            return Ok(values.to_vec());
        }
        let mut result = self.vec(values.len())?;
        for value in values {
            self.step()?;
            result.push(self.string(value)?);
        }
        Ok(result)
    }
    fn equal(&mut self, a: &str, b: &str) -> Result<bool, Error<E>> {
        self.step()?;
        if a.len() != b.len() {
            return Ok(false);
        }
        for (a, b) in a.bytes().zip(b.bytes()) {
            self.step()?;
            if a != b {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn valid_identifier(&mut self, s: &str) -> Result<bool, Error<E>> {
        self.bytes(s.len())?;
        Ok(is_style_identifier(s))
    }
}

pub(super) fn validate_selector<E>(
    selector: &str,
    blocks: &[&str],
    ctx: &mut Context<'_, E>,
) -> Result<(), Error<E>> {
    ctx.bytes(selector.len())?;
    let mut parts = selector.split('.');
    let block = parts.next().unwrap_or_default();
    ctx.step()?;
    ctx.charge(0, 0, (blocks.len() * 32) as u64)?;
    if !blocks.contains(&block) {
        return Err(StyleValidationError::InvalidSelector(SelectorError::InvalidBlockType).into());
    }
    let mut legacy = BTreeSet::new();
    let mut previous: Option<&str> = None;
    for (index, class) in parts.enumerate() {
        ctx.step()?;
        if !ctx.valid_identifier(class)? {
            return Err(StyleValidationError::InvalidSelector(SelectorError::InvalidClass).into());
        }
        let duplicate = if ctx.counted_mode() {
            let mut duplicate = false;
            ctx.bytes(selector.len())?;
            for earlier in selector.split('.').skip(1).take(index) {
                if ctx.equal(earlier, class)? {
                    duplicate = true;
                    break;
                }
            }
            duplicate
        } else {
            !legacy.insert(class)
        };
        if duplicate {
            return Err(
                StyleValidationError::InvalidSelector(SelectorError::DuplicateClass).into(),
            );
        }
        if let Some(prior) = previous {
            ctx.bytes(class.len().min(prior.len()))?;
            if prior > class {
                return Err(StyleValidationError::InvalidSelector(
                    SelectorError::NonCanonicalClassOrder,
                )
                .into());
            }
        }
        previous = Some(class);
    }
    Ok(())
}
pub(super) fn families_valid<E>(
    families: &[String],
    ctx: &mut Context<'_, E>,
) -> Result<bool, Error<E>> {
    if families.is_empty() {
        return Ok(false);
    }
    let mut legacy = BTreeSet::new();
    for (i, family) in families.iter().enumerate() {
        ctx.step()?;
        ctx.bytes(family.len())?;
        ctx.bytes(family.len())?;
        if family.trim().is_empty() || family.chars().any(char::is_control) {
            return Ok(false);
        }
        if ctx.counted_mode() {
            for prior in &families[..i] {
                if ctx.equal(prior, family)? {
                    return Ok(false);
                }
            }
        } else if !legacy.insert(family.as_str()) {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn selector_matches<E>(
    selector: &str,
    block: &str,
    classes: &[String],
    vector: bool,
    ctx: &mut Context<'_, E>,
) -> Result<bool, Error<E>> {
    let blocks = if vector {
        PRECOMPOSED_VECTOR_STYLEABLE_BLOCK_TYPES
    } else {
        STYLEABLE_BLOCK_TYPES
    };
    validate_selector(selector, blocks, ctx)?;
    // Retain exactly the legacy set allocations. The counted side borrows the
    // validated class atoms and pays its actual membership comparisons.
    ctx.bytes(selector.len())?;
    let mut parts = selector.split('.');
    let selected = parts.next().unwrap_or_default();
    let legacy = if ctx.counted_mode() {
        None
    } else {
        Some((
            parts.clone().collect::<BTreeSet<_>>(),
            classes.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        ))
    };
    if !ctx.equal(selected, block)? {
        return Ok(false);
    }
    if let Some((required, target)) = legacy {
        return Ok(required.is_subset(&target));
    }
    for class in parts {
        let mut found = false;
        for target in classes {
            if ctx.equal(class, target)? {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(false);
        }
    }
    Ok(true)
}

enum Lookup<'s> {
    Legacy(BTreeMap<&'s StyleId, &'s StyleRule>),
    Counted(&'s [StyleRule]),
}
impl<'s> Lookup<'s> {
    fn get<E>(
        &self,
        id: &StyleId,
        ctx: &mut Context<'_, E>,
    ) -> Result<Option<&'s StyleRule>, Error<E>> {
        match self {
            Self::Legacy(map) => Ok(map.get(id).copied()),
            Self::Counted(rules) => {
                for rule in *rules {
                    if ctx.equal(rule.style_id.as_str(), id.as_str())? {
                        return Ok(Some(rule));
                    }
                }
                Ok(None)
            }
        }
    }
}
pub(super) fn validate_contract<E>(
    sheet: &StyleSheet,
    staging: bool,
    blocks: &[&str],
    ctx: &mut Context<'_, E>,
) -> Result<(), Error<E>> {
    let mut by_id = BTreeMap::new();
    for (index, rule) in sheet.rules.iter().enumerate() {
        ctx.step()?;
        if rule.source_order
            != u32::try_from(index).map_err(|_| StyleValidationError::SourceOrderMismatch)?
        {
            return Err(StyleValidationError::SourceOrderMismatch.into());
        }
        validate_selector(&rule.selector, blocks, ctx)?;
        for declaration in &rule.declarations {
            ctx.step()?;
            ctx.bytes(declaration.name.len())?;
            if !valid_declaration_name(&declaration.name) {
                return Err(StyleValidationError::InvalidDeclarationName.into());
            }
            match declaration.name.as_str() {
                "font_family" => match &declaration.value {
                    StyleValue::FontFamilyList(values) if families_valid(values, ctx)? => {}
                    _ => return Err(StyleValidationError::InvalidDeclarationValue.into()),
                },
                "font_size" | "line_height" => {
                    if !matches!(declaration.value,StyleValue::Length(value) if value.raw()>0 && value.raw()<=JSON_SAFE_INTEGER_MAX)
                    {
                        return Err(StyleValidationError::InvalidDeclarationValue.into());
                    }
                }
                "page" => {
                    if let StyleValue::Text(value) | StyleValue::Keyword(value) = &declaration.value
                    {
                        ctx.bytes(value.len())?;
                    }
                    if !valid_page_value(&declaration.value) {
                        return Err(StyleValidationError::InvalidPageProperty.into());
                    }
                }
                "space_before" | "space_after" | "start_indent" | "end_indent" if staging => {
                    if !matches!(declaration.value,StyleValue::Length(value) if value.raw()>=0&&value.raw()<=JSON_SAFE_INTEGER_MAX)
                    {
                        return Err(StyleValidationError::InvalidDeclarationValue.into());
                    }
                }
                "text_align" if staging => {
                    if !matches!(&declaration.value,StyleValue::Keyword(value) if matches!(value.as_str(),"start"|"end"|"center"))
                    {
                        return Err(StyleValidationError::InvalidDeclarationValue.into());
                    }
                }
                "width" if staging => {
                    if !matches!(&declaration.value,StyleValue::Keyword(value) if value=="auto")
                        && !matches!(declaration.value,StyleValue::Length(value) if value.raw()>0&&value.raw()<=JSON_SAFE_INTEGER_MAX)
                    {
                        return Err(StyleValidationError::InvalidDeclarationValue.into());
                    }
                }
                "keep_with_next" | "keep_caption" if staging => {
                    if !matches!(declaration.value, StyleValue::Boolean(_)) {
                        return Err(StyleValidationError::InvalidDeclarationValue.into());
                    }
                }
                _ => return Err(StyleValidationError::UnknownProperty.into()),
            }
        }
        let duplicate = if ctx.counted_mode() {
            let mut found = false;
            for prior in &sheet.rules[..index] {
                if ctx.equal(prior.style_id.as_str(), rule.style_id.as_str())? {
                    found = true;
                    break;
                }
            }
            found
        } else {
            by_id.insert(&rule.style_id, rule).is_some()
        };
        if duplicate {
            return Err(StyleValidationError::DuplicateStyleId.into());
        }
    }
    let lookup = if ctx.counted_mode() {
        Lookup::Counted(&sheet.rules)
    } else {
        Lookup::Legacy(by_id)
    };
    for rule in &sheet.rules {
        ctx.step()?;
        if let Some(parent) = rule.extends.as_ref() {
            if lookup.get(parent, ctx)?.is_none() {
                return Err(StyleValidationError::UnknownParent.into());
            }
            let mut legacy = BTreeSet::new();
            let mut counted: Vec<&StyleId> = if ctx.counted_mode() {
                ctx.vec(sheet.rules.len())?
            } else {
                Vec::new()
            };
            let mut current = Some(&rule.style_id);
            while let Some(id) = current {
                ctx.step()?;
                let duplicate = if ctx.counted_mode() {
                    let mut found = false;
                    for prior in &counted {
                        if ctx.equal((*prior).as_str(), id.as_str())? {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        counted.push(id)
                    }
                    found
                } else {
                    !legacy.insert(id)
                };
                if duplicate {
                    return Err(StyleValidationError::InheritanceCycle.into());
                }
                current = lookup
                    .get(id, ctx)?
                    .and_then(|style| style.extends.as_ref());
            }
        }
    }
    Ok(())
}
#[derive(Clone, Copy)]
pub(super) enum Domain {
    Basic,
    Table(bool),
    Shape,
    Vector,
}
pub(super) fn validate_sheet<E>(
    sheet: &StyleSheet,
    domain: Domain,
    ctx: &mut Context<'_, E>,
) -> Result<(), Error<E>> {
    let blocks = match domain {
        Domain::Shape => STAGING_PRECOMPOSED_VECTOR_STYLEABLE_BLOCK_TYPES,
        Domain::Vector => PRECOMPOSED_VECTOR_STYLEABLE_BLOCK_TYPES,
        _ => STYLEABLE_BLOCK_TYPES,
    };
    validate_contract(sheet, true, blocks, ctx)?;
    if matches!(domain, Domain::Shape) {
        return Ok(());
    }
    let lookup = if matches!(domain, Domain::Vector) && !ctx.counted_mode() {
        Lookup::Legacy(sheet.rules.iter().map(|r| (&r.style_id, r)).collect())
    } else {
        Lookup::Counted(&sheet.rules)
    };
    for rule in &sheet.rules {
        ctx.step()?;
        let block = rule.selector.split('.').next().unwrap_or_default();
        match domain {
            Domain::Vector => {
                let kind = PrecomposedVectorStyleKind::from_str(block).ok_or(
                    StyleValidationError::InvalidSelector(SelectorError::InvalidBlockType),
                )?;
                let mut current = Some(rule);
                while let Some(origin) = current {
                    ctx.step()?;
                    for declaration in &origin.declarations {
                        ctx.step()?;
                        let property = PrecomposedVectorStyleProperty::from_str(&declaration.name)
                            .ok_or(StyleValidationError::UnknownProperty)?;
                        if precomposed_vector_style_property_descriptor(kind, property).is_none() {
                            return Err(StyleValidationError::InapplicableProperty.into());
                        }
                    }
                    current = match origin.extends.as_ref() {
                        Some(id) => lookup.get(id, ctx)?,
                        None => None,
                    };
                }
            }
            Domain::Basic | Domain::Table(_) => {
                if !(matches!(domain, Domain::Table(_)) && block == "table")
                    && BasicStyleBlockKind::from_str(block).is_none()
                {
                    return Err(StyleValidationError::InapplicableProperty.into());
                }
                for declaration in &rule.declarations {
                    ctx.step()?;
                    let property = BasicStyleProperty::from_str(&declaration.name)
                        .ok_or(StyleValidationError::UnknownProperty)?;
                    let applies = if block == "table" && matches!(domain, Domain::Table(_)) {
                        property.applies_to_table()
                    } else {
                        property.applies_to_str(block)
                    };
                    if !applies {
                        return Err(StyleValidationError::InapplicableProperty.into());
                    }
                    if matches!(domain, Domain::Table(false))
                        && block == "table"
                        && property == BasicStyleProperty::Page
                        && !matches!(&declaration.value,StyleValue::Keyword(value) if value=="auto")
                    {
                        return Err(StyleValidationError::InvalidPageProperty.into());
                    }
                }
            }
            Domain::Shape => unreachable!(),
        }
    }
    Ok(())
}

pub(super) trait Properties {
    fn get(&self, name: &str) -> Option<&StyleValue>;
}
impl Properties for ComputedStyle {
    fn get(&self, name: &str) -> Option<&StyleValue> {
        self.properties.get(name)
    }
}
const PROPERTIES: [BasicStyleProperty; 12] = [
    BasicStyleProperty::FontFamily,
    BasicStyleProperty::FontSize,
    BasicStyleProperty::LineHeight,
    BasicStyleProperty::Page,
    BasicStyleProperty::SpaceBefore,
    BasicStyleProperty::SpaceAfter,
    BasicStyleProperty::StartIndent,
    BasicStyleProperty::EndIndent,
    BasicStyleProperty::TextAlign,
    BasicStyleProperty::Width,
    BasicStyleProperty::KeepWithNext,
    BasicStyleProperty::KeepCaption,
];
pub(super) enum Values<'s> {
    Legacy(ComputedStyle),
    Counted([Option<&'s StyleValue>; 12]),
}
impl Properties for Values<'_> {
    fn get(&self, name: &str) -> Option<&StyleValue> {
        match self {
            Self::Legacy(v) => v.get(name),
            Self::Counted(v) => PROPERTIES
                .iter()
                .position(|p| p.as_str() == name)
                .and_then(|i| v[i]),
        }
    }
}
pub(super) fn cascade<'s, E>(
    sheet: &'s StyleSheet,
    block: &str,
    classes: &[String],
    vector: bool,
    ctx: &mut Context<'_, E>,
) -> Result<Values<'s>, Error<E>> {
    let lookup = if ctx.counted_mode() {
        Lookup::Counted(&sheet.rules)
    } else {
        Lookup::Legacy(sheet.rules.iter().map(|r| (&r.style_id, r)).collect())
    };
    let mut legacy: BTreeMap<String, CascadeWinner> = BTreeMap::new();
    let mut counted: [Option<(CascadePriority, &StyleValue)>; 12] = [None; 12];
    for matched in &sheet.rules {
        ctx.step()?;
        if !selector_matches(&matched.selector, block, classes, vector, ctx)? {
            continue;
        }
        ctx.bytes(matched.selector.len())?;
        let class_count = cascade_priority_index(matched.selector.matches('.').count())?;
        let specificity = (0, class_count, 1);
        let mut chain = if ctx.counted_mode() {
            let mut v = ctx.vec(sheet.rules.len())?;
            v.push(matched);
            v
        } else {
            vec![matched]
        };
        let mut current = matched.extends.as_ref();
        while let Some(parent) = current {
            ctx.step()?;
            let rule = lookup
                .get(parent, ctx)?
                .ok_or(StyleValidationError::UnknownParent)?;
            chain.push(rule);
            current = rule.extends.as_ref();
        }
        ctx.charge(0, 0, chain.len() as u64)?;
        chain.reverse();
        for (depth, origin) in chain.iter().enumerate() {
            ctx.step()?;
            for (order, declaration) in origin.declarations.iter().enumerate() {
                ctx.step()?;
                let key = (
                    declaration.important,
                    specificity,
                    matched.source_order,
                    cascade_priority_index(depth)?,
                    cascade_priority_index(order)?,
                );
                if ctx.counted_mode() {
                    ctx.charge(0, 0, 12 * 32)?;
                    let index = PROPERTIES
                        .iter()
                        .position(|p| p.as_str() == declaration.name)
                        .ok_or(StyleValidationError::UnknownProperty)?;
                    if counted[index].map_or(true, |(prior, _)| key > prior) {
                        counted[index] = Some((key, &declaration.value));
                    }
                } else if legacy
                    .get(&declaration.name)
                    .map_or(true, |(prior, _)| key > *prior)
                {
                    legacy.insert(declaration.name.clone(), (key, declaration.value.clone()));
                }
            }
        }
    }
    Ok(if ctx.counted_mode() {
        ctx.charge(0, 0, counted.len() as u64)?;
        Values::Counted(counted.map(|v| v.map(|(_, v)| v)))
    } else {
        Values::Legacy(ComputedStyle {
            properties: legacy
                .into_iter()
                .map(|(name, (_, value))| (name, value))
                .collect(),
        })
    })
}

fn get<'s, E>(
    values: &'s impl Properties,
    name: &str,
    ctx: &mut Context<'_, E>,
) -> Result<Option<&'s StyleValue>, Error<E>> {
    ctx.charge(0, 0, 12 * 32)?;
    Ok(values.get(name))
}
pub(super) fn page<E>(
    values: &impl Properties,
    ctx: &mut Context<'_, E>,
) -> Result<Option<PageName>, Error<E>> {
    match get(values, "page", ctx)? {
        None => Ok(None),
        Some(StyleValue::Keyword(value)) if value == "auto" => Ok(None),
        Some(StyleValue::Text(value)) => {
            ctx.bytes(value.len())?;
            PageName::new(ctx.string(value)?)
                .map(Some)
                .map_err(|_| StyleValidationError::InvalidPageProperty.into())
        }
        Some(_) => Err(StyleValidationError::InvalidPageProperty.into()),
    }
}
pub(super) fn block<E>(
    values: &impl Properties,
    parent: Option<&ComputedMachineBlockStyle>,
    ctx: &mut Context<'_, E>,
) -> Result<ComputedMachineBlockStyle, Error<E>> {
    fn nonnegative<E>(
        values: &impl Properties,
        name: &str,
        ctx: &mut Context<'_, E>,
    ) -> Result<NonNegativeLength, Error<E>> {
        match get(values, name, ctx)? {
            None => Ok(NonNegativeLength::ZERO),
            Some(StyleValue::Length(v)) => NonNegativeLength::new(*v)
                .ok_or(StyleValidationError::InvalidDeclarationValue.into()),
            _ => Err(StyleValidationError::InvalidDeclarationValue.into()),
        }
    }
    fn boolean<E>(
        values: &impl Properties,
        name: &str,
        initial: bool,
        ctx: &mut Context<'_, E>,
    ) -> Result<bool, Error<E>> {
        match get(values, name, ctx)? {
            None => Ok(initial),
            Some(StyleValue::Boolean(v)) => Ok(*v),
            _ => Err(StyleValidationError::InvalidDeclarationValue.into()),
        }
    }
    // Keep text_align before the spacing/width/boolean fields, as the original
    // closure does: malformed fields must retain their first semantic error.
    let text_align = match get(values, "text_align", ctx)? {
        Some(StyleValue::Keyword(v)) => match v.as_str() {
            "start" => MachineTextAlign::Start,
            "end" => MachineTextAlign::End,
            "center" => MachineTextAlign::Center,
            _ => return Err(StyleValidationError::InvalidDeclarationValue.into()),
        },
        None => parent.map_or(MachineTextAlign::Start, |p| p.text_align),
        _ => return Err(StyleValidationError::InvalidDeclarationValue.into()),
    };
    let space_before = nonnegative(values, "space_before", ctx)?;
    let space_after = nonnegative(values, "space_after", ctx)?;
    let start_indent = nonnegative(values, "start_indent", ctx)?;
    let end_indent = nonnegative(values, "end_indent", ctx)?;
    let width = match get(values, "width", ctx)? {
        None => MachineFigureWidth::Auto,
        Some(StyleValue::Keyword(v)) if v == "auto" => MachineFigureWidth::Auto,
        Some(StyleValue::Length(v)) => MachineFigureWidth::Length(
            PositiveLength::new(*v).ok_or(StyleValidationError::InvalidDeclarationValue)?,
        ),
        _ => return Err(StyleValidationError::InvalidDeclarationValue.into()),
    };
    let keep_with_next = boolean(values, "keep_with_next", false, ctx)?;
    let keep_caption = boolean(values, "keep_caption", true, ctx)?;
    Ok(ComputedMachineBlockStyle {
        space_before,
        space_after,
        start_indent,
        end_indent,
        text_align,
        width,
        keep_with_next,
        keep_caption,
    })
}
pub(super) fn inheritance<E>(
    values: &impl Properties,
    parent: Option<&SemanticContainerInheritanceStyle>,
    ctx: &mut Context<'_, E>,
) -> Result<SemanticContainerInheritanceStyle, Error<E>> {
    let font_families = match get(values, "font_family", ctx)? {
        Some(StyleValue::FontFamilyList(families)) if families_valid(families, ctx)? => {
            Some(ctx.families(families)?)
        }
        None => match parent.and_then(|p| p.font_families.as_deref()) {
            Some(values) => Some(ctx.families(values)?),
            None => None,
        },
        Some(_) => return Err(StyleValidationError::InvalidDeclarationValue.into()),
    };
    fn positive<E>(
        values: &impl Properties,
        name: &str,
        inherited: Option<PositiveLength>,
        ctx: &mut Context<'_, E>,
    ) -> Result<Option<PositiveLength>, Error<E>> {
        match get(values, name, ctx)? {
            None => Ok(inherited),
            Some(StyleValue::Length(v)) => PositiveLength::new(*v)
                .map(Some)
                .ok_or(StyleValidationError::InvalidDeclarationValue.into()),
            _ => Err(StyleValidationError::InvalidDeclarationValue.into()),
        }
    }
    let block_style = block(values, parent.map(|p| p.block_style).as_ref(), ctx)?;
    let font_size = positive(values, "font_size", parent.and_then(|p| p.font_size), ctx)?;
    let line_height = positive(
        values,
        "line_height",
        parent.and_then(|p| p.line_height),
        ctx,
    )?;
    Ok(SemanticContainerInheritanceStyle {
        block_style,
        font_families,
        font_size,
        line_height,
    })
}
pub(super) fn semantic<K, E>(
    kind: K,
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
    ctx: &mut Context<'_, E>,
) -> Result<ComputedSemanticContainerStyle<K>, Error<E>> {
    validate_sheet(sheet, Domain::Basic, ctx)?;
    let values = cascade(sheet, "paragraph", classes, false, ctx)?;
    let inheritance = inheritance(&values, parent, ctx)?;
    if inheritance.block_style.width() != MachineFigureWidth::Auto
        || !inheritance.block_style.keep_caption()
    {
        return Err(StyleValidationError::InapplicableProperty.into());
    }
    Ok(ComputedSemanticContainerStyle {
        semantic_kind: kind,
        page_name: page(&values, ctx)?,
        inheritance,
    })
}
pub(super) fn descendant<E>(
    block: &str,
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
    named_tables: bool,
    ctx: &mut Context<'_, E>,
) -> Result<SemanticContainerInheritanceStyle, Error<E>> {
    validate_sheet(sheet, Domain::Table(named_tables), ctx)?;
    if block != "table" && BasicStyleBlockKind::from_str(block).is_none() {
        return Err(StyleValidationError::InvalidSelector(SelectorError::InvalidBlockType).into());
    }
    inheritance(&cascade(sheet, block, classes, false, ctx)?, parent, ctx)
}
pub(super) fn math_close<E>(
    kind: StagingMathStyleKind,
    inheritance: SemanticContainerInheritanceStyle,
    page_name: Option<PageName>,
    ctx: &mut Context<'_, E>,
) -> Result<StagingMathComputedStyle, Error<E>> {
    let font_families = inheritance
        .font_families
        .ok_or(StyleValidationError::MissingTextProperty)?;
    if !families_valid(&font_families, ctx)? {
        return Err(StyleValidationError::MissingTextProperty.into());
    }
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
pub(super) fn display_math<E>(
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
    ctx: &mut Context<'_, E>,
) -> Result<StagingMathComputedStyle, Error<E>> {
    validate_sheet(sheet, Domain::Basic, ctx)?;
    let values = cascade(sheet, "paragraph", classes, false, ctx)?;
    let inheritance = inheritance(&values, parent, ctx)?;
    if inheritance.block_style.width() != MachineFigureWidth::Auto
        || !inheritance.block_style.keep_caption()
    {
        return Err(StyleValidationError::InapplicableProperty.into());
    }
    math_close(
        StagingMathStyleKind::Display,
        inheritance,
        page(&values, ctx)?,
        ctx,
    )
}

// These operations are the same canonical field writer for both legacy String
// construction and counted measurement/retention/full comparison.
trait CanonicalWriter {
    type Error;
    fn text(&mut self, value: &str) -> Result<(), Self::Error>;
    fn string(&mut self, value: &str) -> Result<(), Self::Error>;
    fn number(&mut self, value: i64) -> Result<(), Self::Error>;
}
struct LegacyWriter(String);
impl CanonicalWriter for LegacyWriter {
    type Error = Infallible;
    fn text(&mut self, v: &str) -> Result<(), Infallible> {
        if self.0.is_empty() {
            self.0 = String::from(v)
        } else {
            self.0.push_str(v)
        }
        Ok(())
    }
    fn string(&mut self, v: &str) -> Result<(), Infallible> {
        push_jcs_string(&mut self.0, v);
        Ok(())
    }
    fn number(&mut self, v: i64) -> Result<(), Infallible> {
        self.0.push_str(&v.to_string());
        Ok(())
    }
}
fn vector_canonical<W: CanonicalWriter>(
    r: &PrecomposedVectorComputedStyleReceipt,
    w: &mut W,
) -> Result<(), W::Error> {
    w.text("{\"algorithm\":")?;
    w.string(PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION)?;
    w.text(",\"end_indent\":")?;
    w.number(r.block.end_indent.get().raw())?;
    match &r.supplement {
        PrecomposedVectorStyleSupplement::EquationNumber(s) => {
            w.text(",\"equation_number_text\":{\"font_families\":")?;
            match &s.font_families {
                None => w.text("null")?,
                Some(values) => {
                    w.text("[")?;
                    for (i, v) in values.iter().enumerate() {
                        if i > 0 {
                            w.text(",")?
                        }
                        w.string(v)?;
                    }
                    w.text("]")?
                }
            }
            w.text(",\"font_size\":")?;
            optional_length(s.font_size, w)?;
            w.text(",\"line_height\":")?;
            optional_length(s.line_height, w)?;
            w.text("}")?;
        }
        PrecomposedVectorStyleSupplement::FigureCaption { keep_caption } => {
            w.text(",\"keep_caption\":")?;
            w.text(if *keep_caption { "true" } else { "false" })?;
        }
    }
    w.text(",\"keep_with_next\":")?;
    w.text(if r.block.keep_with_next {
        "true"
    } else {
        "false"
    })?;
    w.text(",\"kind\":")?;
    w.string(r.kind.as_str())?;
    w.text(",\"page\":")?;
    match &r.page_name {
        None => w.text("null")?,
        Some(v) => w.string(v.as_str())?,
    }
    w.text(",\"space_after\":")?;
    w.number(r.block.space_after.get().raw())?;
    w.text(",\"space_before\":")?;
    w.number(r.block.space_before.get().raw())?;
    w.text(",\"start_indent\":")?;
    w.number(r.block.start_indent.get().raw())?;
    w.text(",\"text_align\":")?;
    w.string(r.block.text_align.as_str())?;
    w.text("}")
}
fn optional_length<W: CanonicalWriter>(
    value: Option<PositiveLength>,
    w: &mut W,
) -> Result<(), W::Error> {
    match value {
        None => w.text("null"),
        Some(v) => w.number(v.get().raw()),
    }
}
pub(super) fn legacy_vector_canonical(r: &PrecomposedVectorComputedStyleReceipt) -> String {
    let mut w = LegacyWriter(String::from(""));
    match vector_canonical(r, &mut w) {
        Ok(()) => w.0,
        Err(e) => match e {},
    }
}
struct PaidWriter<'c, 'q, E> {
    ctx: &'c mut Context<'q, E>,
    length: usize,
    output: Option<String>,
    expected: Option<&'c str>,
    hash: typaxis_core::Sha256,
}
impl<E> CanonicalWriter for PaidWriter<'_, '_, E> {
    type Error = Error<E>;
    fn text(&mut self, v: &str) -> Result<(), Error<E>> {
        self.ctx.step()?;
        self.ctx.bytes(v.len())?;
        let end = self.length.checked_add(v.len()).ok_or(Error::Allocation)?;
        if let Some(expected) = self.expected {
            if expected.as_bytes().get(self.length..end) != Some(v.as_bytes()) {
                return Err(StyleValidationError::InapplicableProperty.into());
            }
        }
        if let Some(out) = self.output.as_mut() {
            out.push_str(v)
        }
        std::fmt::Write::write_str(&mut self.hash, v).expect("SHA writer");
        self.length = end;
        Ok(())
    }
    fn string(&mut self, v: &str) -> Result<(), Error<E>> {
        struct Escaped<'w, 'c, 'q, E>(&'w mut PaidWriter<'c, 'q, E>, Option<Error<E>>);
        impl<E> std::fmt::Write for Escaped<'_, '_, '_, E> {
            fn write_str(&mut self, v: &str) -> std::fmt::Result {
                match self.0.text(v) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        self.1 = Some(e);
                        Err(std::fmt::Error)
                    }
                }
            }
        }
        self.ctx.bytes(v.len())?;
        let mut writer = Escaped(self, None);
        if typaxis_core::write_jcs_string(&mut writer, v).is_err() {
            return Err(writer.1.expect("paid write refusal"));
        }
        Ok(())
    }
    fn number(&mut self, v: i64) -> Result<(), Error<E>> {
        use std::fmt::Write;
        struct Digits {
            bytes: [u8; 32],
            len: usize,
        }
        impl Write for Digits {
            fn write_str(&mut self, v: &str) -> std::fmt::Result {
                let end = self.len + v.len();
                self.bytes[self.len..end].copy_from_slice(v.as_bytes());
                self.len = end;
                Ok(())
            }
        }
        self.ctx.step()?;
        let mut digits = Digits {
            bytes: [0; 32],
            len: 0,
        };
        write!(&mut digits, "{v}").expect("i64 stack buffer");
        self.text(std::str::from_utf8(&digits.bytes[..digits.len]).expect("decimal"))
    }
}
fn paid_canonical<E>(
    r: &PrecomposedVectorComputedStyleReceipt,
    ctx: &mut Context<'_, E>,
) -> Result<String, Error<E>> {
    let mut measure = PaidWriter {
        ctx,
        length: 0,
        output: None,
        expected: None,
        hash: typaxis_core::Sha256::new(),
    };
    vector_canonical(r, &mut measure)?;
    let len = measure.length;
    ctx.charge(0, u64::try_from(len).map_err(|_| Error::Allocation)?, 1)?;
    allocation_attempt(len as u64)?;
    let mut output = String::new();
    output
        .try_reserve_exact(len)
        .map_err(|_| Error::Allocation)?;
    let mut writer = PaidWriter {
        ctx,
        length: 0,
        output: Some(output),
        expected: None,
        hash: typaxis_core::Sha256::new(),
    };
    vector_canonical(r, &mut writer)?;
    Ok(writer.output.expect("owned paid canonical"))
}
pub(super) fn vector<E>(
    sheet: &StyleSheet,
    kind: PrecomposedVectorStyleKind,
    classes: &[String],
    parent: Option<&SemanticContainerInheritanceStyle>,
    ctx: &mut Context<'_, E>,
) -> Result<PrecomposedVectorComputedStyleReceipt, Error<E>> {
    validate_sheet(sheet, Domain::Vector, ctx)?;
    let values = cascade(sheet, kind.as_str(), classes, true, ctx)?;
    let inherited = inheritance(&values, parent, ctx)?;
    if inherited.block_style.width != MachineFigureWidth::Auto {
        return Err(StyleValidationError::InapplicableProperty.into());
    }
    let page_name = page(&values, ctx)?;
    let supplement = match kind {
        PrecomposedVectorStyleKind::MathVectorBlock => {
            PrecomposedVectorStyleSupplement::EquationNumber(
                PrecomposedVectorEquationNumberTextStyle {
                    font_families: inherited.font_families,
                    font_size: inherited.font_size,
                    line_height: inherited.line_height,
                },
            )
        }
        PrecomposedVectorStyleKind::VectorFigure => {
            PrecomposedVectorStyleSupplement::FigureCaption {
                keep_caption: inherited.block_style.keep_caption,
            }
        }
    };
    let mut r = PrecomposedVectorComputedStyleReceipt {
        registry_version: PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION,
        kind,
        block: inherited.block_style,
        page_name,
        supplement,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
        _binding: PrecomposedVectorStyleBinding,
    };
    if ctx.counted_mode() {
        r.canonical_jcs = paid_canonical(&r, ctx)?;
        ctx.bytes(r.canonical_jcs.len())?;
        r.fingerprint = sha256(r.canonical_jcs.as_bytes());
        let mut verify = PaidWriter {
            ctx,
            length: 0,
            output: None,
            expected: Some(&r.canonical_jcs),
            hash: typaxis_core::Sha256::new(),
        };
        vector_canonical(&r, &mut verify)?;
        if verify.length != r.canonical_jcs.len()
            || verify.hash.finish() != r.fingerprint
            || (kind == PrecomposedVectorStyleKind::MathVectorBlock && !r.block.keep_caption)
        {
            return Err(StyleValidationError::InapplicableProperty.into());
        }
    } else {
        r.canonical_jcs = legacy_vector_canonical(&r);
        r.fingerprint = sha256(r.canonical_jcs.as_bytes());
        r.verify_for(kind)
            .map_err(|_| StyleValidationError::InapplicableProperty)?;
    }
    Ok(r)
}

#[cfg(feature = "book-v2-staging")]
pub fn validate_book_v2_style_shape_counted<E>(
    sheet: &StyleSheet,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<(), Error<E>> {
    validate_sheet(sheet, Domain::Shape, &mut Context::counted(charge))
}
#[cfg(feature = "book-v2-staging")]
pub fn validate_book_v2_document_styles_counted<E>(
    sheet: &StyleSheet,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<(), Error<E>> {
    validate_sheet(sheet, Domain::Table(true), &mut Context::counted(charge))
}
#[cfg(feature = "book-v2-staging")]
pub fn validate_book_v2_basic_styles_counted<E>(
    sheet: &StyleSheet,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<(), Error<E>> {
    validate_sheet(sheet, Domain::Basic, &mut Context::counted(charge))
}
#[cfg(feature = "book-v2-staging")]
pub fn validate_book_v2_vector_styles_counted<E>(
    sheet: &StyleSheet,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<(), Error<E>> {
    validate_sheet(sheet, Domain::Vector, &mut Context::counted(charge))
}
#[cfg(feature = "book-v2-staging")]
pub fn cascade_book_v2_semantic_container_style_counted<E>(
    kind: BookV2SemanticContainerStyleKind,
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<BookV2SemanticContainerComputedStyle, Error<E>> {
    semantic(kind, classes, sheet, parent, &mut Context::counted(charge))
}
#[cfg(feature = "book-v2-staging")]
pub fn cascade_book_v2_container_kind_counted<K: Copy, E>(
    kind: K,
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<ComputedSemanticContainerStyle<K>, Error<E>> {
    semantic(kind, classes, sheet, parent, &mut Context::counted(charge))
}
#[cfg(feature = "book-v2-staging")]
pub fn cascade_book_v2_descendant_style_counted<E>(
    block: &str,
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<SemanticContainerInheritanceStyle, Error<E>> {
    descendant(
        block,
        classes,
        sheet,
        parent,
        true,
        &mut Context::counted(charge),
    )
}
#[cfg(feature = "book-v2-staging")]
pub fn cascade_book_v2_display_math_style_counted<E>(
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<StagingMathComputedStyle, Error<E>> {
    display_math(classes, sheet, parent, &mut Context::counted(charge))
}
#[cfg(feature = "book-v2-staging")]
pub fn cascade_book_v2_vector_style_counted<E>(
    sheet: &StyleSheet,
    kind: PrecomposedVectorStyleKind,
    classes: &[String],
    parent: Option<&SemanticContainerInheritanceStyle>,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<PrecomposedVectorComputedStyleReceipt, Error<E>> {
    vector(sheet, kind, classes, parent, &mut Context::counted(charge))
}
#[cfg(feature = "book-v2-staging")]
pub fn close_book_v2_inline_math_style_counted<E>(
    owner: &SemanticContainerInheritanceStyle,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<StagingMathComputedStyle, Error<E>> {
    let inherited = owner
        .try_copy_book_v2_counted(charge)
        .map_err(|e| match e {
            super::book_v2::BookV2StyleCopyError::Budget(e) => Error::Budget(e),
            super::book_v2::BookV2StyleCopyError::Allocation => Error::Allocation,
        })?;
    math_close(
        StagingMathStyleKind::Inline,
        inherited,
        None,
        &mut Context::counted(charge),
    )
}

/// Resolve the ordinary basic/table profile's `page` value directly from the
/// shared borrowed twelve-slot cascade. Named table pages retain the original
/// basic/table profile refusal. No `ComputedStyle` map is constructed.
#[cfg(feature = "book-v2-staging")]
pub fn cascade_book_v2_basic_page_name_counted<E>(
    sheet: &StyleSheet,
    block: &str,
    classes: &[String],
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<Option<PageName>, Error<E>> {
    ordinary_page(sheet, block, classes, false, &mut Context::counted(charge))
}

/// Resolve the Book-2 ordinary document `page` value, including its named-table
/// extension, without building a raw-property map. An owned page name is copied
/// only after its concrete UTF-8 request and validation work have been accepted.
#[cfg(feature = "book-v2-staging")]
pub fn cascade_book_v2_document_page_name_counted<E>(
    sheet: &StyleSheet,
    block: &str,
    classes: &[String],
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<Option<PageName>, Error<E>> {
    ordinary_page(sheet, block, classes, true, &mut Context::counted(charge))
}

#[cfg(feature = "book-v2-staging")]
fn ordinary_page<E>(
    sheet: &StyleSheet,
    block: &str,
    classes: &[String],
    named_tables: bool,
    ctx: &mut Context<'_, E>,
) -> Result<Option<PageName>, Error<E>> {
    // Validation precedes block-kind dispatch exactly as both existing public
    // ordinary cascade entry points do.
    validate_sheet(sheet, Domain::Table(named_tables), ctx)?;
    ctx.step()?;
    ctx.bytes(block.len())?;
    if block != "table" && BasicStyleBlockKind::from_str(block).is_none() {
        return Err(StyleValidationError::InvalidSelector(SelectorError::InvalidBlockType).into());
    }
    let values = cascade(sheet, block, classes, false, ctx)?;
    page(&values, ctx)
}

/// Compare actual inherited style fields and UTF-8 family contents with caller
/// accepted Work before each comparison. This allocates no storage, copies no
/// families, and uses no fingerprint or pointer identity shortcut.
#[cfg(feature = "book-v2-staging")]
pub fn book_v2_inheritance_styles_equal_counted<E>(
    a: &SemanticContainerInheritanceStyle,
    b: &SemanticContainerInheritanceStyle,
    charge: &mut impl FnMut(u64, u64, u64) -> Result<(), E>,
) -> Result<bool, Error<E>> {
    let mut ctx = Context::counted(charge);
    macro_rules! same {
        ($a:expr, $b:expr, $bound:expr) => {
            ctx.charge(0, 0, $bound)?;
            if $a != $b {
                return Ok(false);
            }
        };
    }
    // Preserve the derived equality's field order; enum/optional scalar values
    // have a prepaid bound covering their discriminant and payload.
    same!(a.block_style.space_before, b.block_style.space_before, 1);
    same!(a.block_style.space_after, b.block_style.space_after, 1);
    same!(a.block_style.start_indent, b.block_style.start_indent, 1);
    same!(a.block_style.end_indent, b.block_style.end_indent, 1);
    same!(a.block_style.text_align, b.block_style.text_align, 1);
    same!(a.block_style.width, b.block_style.width, 2);
    same!(
        a.block_style.keep_with_next,
        b.block_style.keep_with_next,
        1
    );
    same!(a.block_style.keep_caption, b.block_style.keep_caption, 1);
    ctx.step()?;
    match (&a.font_families, &b.font_families) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            ctx.step()?;
            if a.len() != b.len() {
                return Ok(false);
            }
            for (a, b) in a.iter().zip(b) {
                if !ctx.equal(a, b)? {
                    return Ok(false);
                }
            }
        }
        _ => return Ok(false),
    }
    same!(a.font_size, b.font_size, 2);
    same!(a.line_height, b.line_height, 2);
    Ok(true)
}

#[cfg(test)]
thread_local! { static ALLOCATION_FAULT: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) }; }
pub(super) fn allocation_attempt<E>(bytes: u64) -> Result<(), BookV2StylePreparationError<E>> {
    if bytes == 0 {
        return Ok(());
    }
    #[cfg(test)]
    return ALLOCATION_FAULT.with(|slot| match slot.get() {
        None => Ok(()),
        Some(0) => Err(BookV2StylePreparationError::Allocation),
        Some(n) => {
            slot.set(Some(n - 1));
            Ok(())
        }
    });
    #[cfg(not(test))]
    Ok(())
}
#[cfg(all(test, feature = "book-v2-staging"))]
mod tests {
    use super::*;
    fn sheet(selector: &str) -> StyleSheet {
        StyleSheet {
            rules: vec![StyleRule {
                style_id: StyleId::new("initial-style").unwrap(),
                extends: None,
                selector: selector.to_owned(),
                source_order: 0,
                declarations: vec![
                    Declaration {
                        name: "font_family".to_owned(),
                        value: StyleValue::FontFamilyList(vec![
                            "日本語\"\\".to_owned(),
                            "Body".to_owned(),
                        ]),
                        important: false,
                    },
                    Declaration {
                        name: "font_size".to_owned(),
                        value: StyleValue::Length(Length::from_raw(786432).unwrap()),
                        important: false,
                    },
                    Declaration {
                        name: "line_height".to_owned(),
                        value: StyleValue::Length(Length::from_raw(1048576).unwrap()),
                        important: false,
                    },
                ],
            }],
        }
    }
    #[test]
    fn counted_initial_style_kernel_values_and_vector_canonical_are_legacy_exact() {
        let sheet = sheet("paragraph");
        let legacy = semantic(17u8, &[], &sheet, None, &mut Context::legacy()).unwrap();
        let mut requests = Vec::new();
        let counted = semantic(
            17u8,
            &[],
            &sheet,
            None,
            &mut Context::counted(&mut |r, s, w| {
                requests.push([r, s, w]);
                Ok::<_, ()>(())
            }),
        )
        .unwrap();
        assert_eq!(legacy, counted);
        assert!(requests.iter().any(|v| v[1] > 0));
        let sheet = super::tests::sheet("math_vector_block");
        let legacy = vector(
            &sheet,
            PrecomposedVectorStyleKind::MathVectorBlock,
            &[],
            None,
            &mut Context::legacy(),
        )
        .unwrap();
        let counted = cascade_book_v2_vector_style_counted(
            &sheet,
            PrecomposedVectorStyleKind::MathVectorBlock,
            &[],
            None,
            &mut |_, _, _| Ok::<_, ()>(()),
        )
        .unwrap();
        assert_eq!(legacy, counted);
        counted
            .verify_for(PrecomposedVectorStyleKind::MathVectorBlock)
            .unwrap();
    }
    #[test]
    fn counted_initial_style_kernel_allocation_faults_are_typed_and_paid() {
        let mut vector_sheet = sheet("math_vector_block");
        let mut inherited_rule = vector_sheet.rules[0].clone();
        inherited_rule.style_id = StyleId::new("inherited-style").unwrap();
        inherited_rule.source_order = 1;
        inherited_rule.extends = Some(vector_sheet.rules[0].style_id.clone());
        vector_sheet.rules.push(inherited_rule);
        let parent = semantic(0u8, &[], &sheet("paragraph"), None, &mut Context::legacy())
            .unwrap()
            .inheritance;
        let mut named_sheet = sheet("paragraph");
        named_sheet.rules[0].declarations.push(Declaration {
            name: "page".to_owned(),
            value: StyleValue::Text("chapter".to_owned()),
            important: false,
        });
        let run = |case,
                   charge: &mut dyn FnMut(u64, u64, u64) -> Result<(), ()>|
         -> Result<(), Error<()>> {
            let mut observer = |r, s, w| charge(r, s, w);
            match case {
                0 => cascade_book_v2_vector_style_counted(
                    &vector_sheet,
                    PrecomposedVectorStyleKind::MathVectorBlock,
                    &[],
                    None,
                    &mut observer,
                )
                .map(|_| ()),
                1 => cascade_book_v2_descendant_style_counted(
                    "paragraph",
                    &[],
                    &StyleSheet { rules: vec![] },
                    Some(&parent),
                    &mut observer,
                )
                .map(|_| ()),
                2 => close_book_v2_inline_math_style_counted(&parent, &mut observer).map(|_| ()),
                3 => cascade_book_v2_container_kind_counted(
                    0u8,
                    &[],
                    &named_sheet,
                    None,
                    &mut observer,
                )
                .map(|_| ()),
                _ => unreachable!(),
            }
        };
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                ALLOCATION_FAULT.with(|v| v.set(None));
            }
        }
        let mut total = 0;
        for case in 0..4 {
            let mut full = Vec::new();
            run(case, &mut |r, s, w| {
                full.push([r, s, w]);
                Ok(())
            })
            .unwrap();
            // Every callback refusal retains exactly the accepted requests;
            // the refused operation has not yet allocated or copied.
            for denied in 0..full.len() {
                let mut accepted = Vec::new();
                assert_eq!(
                    run(case, &mut |r, s, w| {
                        if accepted.len() == denied {
                            return Err(());
                        }
                        accepted.push([r, s, w]);
                        Ok(())
                    }),
                    Err(Error::Budget(()))
                );
                assert_eq!(accepted, full[..denied]);
            }
            let mut completed = false;
            let mut failures = 0;
            for at in 0..100 {
                ALLOCATION_FAULT.with(|v| v.set(Some(at)));
                let _reset = Reset;
                let mut accepted = Vec::new();
                let result = run(case, &mut |r, s, w| {
                    accepted.push([r, s, w]);
                    Ok(())
                });
                match result {
                    Ok(()) => {
                        assert_eq!(accepted, full);
                        completed = true;
                        break;
                    }
                    Err(Error::Allocation) => {
                        failures += 1;
                        assert_eq!(accepted, full[..accepted.len()]);
                        assert!(accepted.last().unwrap()[1] > 0);
                    }
                    Err(error) => panic!("unexpected case {case}: {error:?}"),
                }
            }
            assert!(completed && failures > 0);
            total += failures;
            eprintln!(
                "counted Style allocation backend case={case} requests={} failures={failures}",
                full.len()
            );
        }
        assert!(total >= 12);
        eprintln!("counted Style allocation backend total={total}");
    }

    #[test]
    fn counted_initial_style_kernel_selector_and_sheet_first_errors_match() {
        for selector in [
            "paragraph.a.b.a",
            "paragraph.z.a",
            "unknown.a",
            "paragraph.9bad",
        ] {
            let sheet = sheet(selector);
            let expected =
                validate_sheet(&sheet, Domain::Basic, &mut Context::legacy()).unwrap_err();
            let actual =
                validate_book_v2_basic_styles_counted(&sheet, &mut |_, _, _| Ok::<_, ()>(()))
                    .unwrap_err();
            let expected = match expected {
                BookV2StylePreparationError::Style(e) => e,
                _ => unreachable!(),
            };
            assert_eq!(actual, BookV2StylePreparationError::Style(expected));
        }
    }

    fn page_sheet(block: &str, value: Option<StyleValue>) -> StyleSheet {
        StyleSheet {
            rules: vec![StyleRule {
                style_id: StyleId::new("source-page").unwrap(),
                extends: None,
                selector: block.to_owned(),
                source_order: 0,
                declarations: value
                    .into_iter()
                    .map(|value| Declaration {
                        name: "page".to_owned(),
                        important: false,
                        value,
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn counted_source_style_page_projection_matches_both_legacy_profiles_and_first_errors() {
        for block in [
            "paragraph",
            "heading",
            "list",
            "table",
            "figure",
            "page_break",
        ] {
            for value in [
                None,
                Some(StyleValue::Keyword("auto".into())),
                Some(StyleValue::Text("chapter".into())),
                Some(StyleValue::Boolean(true)),
            ] {
                let sheet = page_sheet(block, value);
                let old_basic = sheet
                    .cascade_basic_document(block, &[])
                    .and_then(|v| v.page_name());
                let old_document =
                    super::super::book_v2::cascade_book_v2_document_style(&sheet, block, &[])
                        .and_then(|v| v.page_name());
                assert_eq!(
                    cascade_book_v2_basic_page_name_counted(
                        &sheet,
                        block,
                        &[],
                        &mut |_, _, _| Ok::<_, ()>(())
                    ),
                    old_basic.map_err(Error::Style)
                );
                assert_eq!(
                    cascade_book_v2_document_page_name_counted(
                        &sheet,
                        block,
                        &[],
                        &mut |_, _, _| Ok::<_, ()>(())
                    ),
                    old_document.map_err(Error::Style)
                );
            }
        }
        let mut sheet = page_sheet("paragraph", Some(StyleValue::Text("chapter".into())));
        sheet.rules[0].extends = Some(StyleId::new("missing").unwrap());
        // Invalid sheet takes precedence over the invalid requested block.
        assert_eq!(
            cascade_book_v2_document_page_name_counted(
                &sheet,
                "unknown",
                &[],
                &mut |_, _, _| Ok::<_, ()>(())
            ),
            Err(Error::Style(StyleValidationError::UnknownParent))
        );
        sheet.rules[0].extends = None;
        assert_eq!(
            cascade_book_v2_document_page_name_counted(
                &sheet,
                "unknown",
                &[],
                &mut |_, _, _| Ok::<_, ()>(())
            ),
            Err(Error::Style(StyleValidationError::InvalidSelector(
                SelectorError::InvalidBlockType
            )))
        );
    }

    #[test]
    fn counted_source_style_page_projection_retains_budget_and_allocation_prefixes() {
        let mut sheet = page_sheet("paragraph", Some(StyleValue::Text("chapter".into())));
        let mut second = sheet.rules[0].clone();
        second.style_id = StyleId::new("source-child").unwrap();
        second.source_order = 1;
        second.extends = Some(sheet.rules[0].style_id.clone());
        sheet.rules.push(second);
        let mut full = Vec::new();
        let result =
            cascade_book_v2_document_page_name_counted(&sheet, "paragraph", &[], &mut |r, s, w| {
                full.push([r, s, w]);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(result.unwrap().as_str(), "chapter");
        for denied in 0..full.len() {
            let mut accepted = Vec::new();
            assert_eq!(
                cascade_book_v2_document_page_name_counted(
                    &sheet,
                    "paragraph",
                    &[],
                    &mut |r, s, w| {
                        if accepted.len() == denied {
                            return Err(denied);
                        };
                        accepted.push([r, s, w]);
                        Ok(())
                    }
                ),
                Err(Error::Budget(denied))
            );
            assert_eq!(accepted, full[..denied]);
        }
        let total = full.iter().fold([0u64; 3], |mut a, b| {
            for i in 0..3 {
                a[i] += b[i]
            }
            a
        });
        for axis in 0..3 {
            for short in [false, true] {
                let mut used = [7u64; 3];
                let mut caps = total.map(|n| n + 7);
                caps[axis] -= u64::from(short);
                let result = cascade_book_v2_document_page_name_counted(
                    &sheet,
                    "paragraph",
                    &[],
                    &mut |r, s, w| {
                        let request = [r, s, w];
                        let next = std::array::from_fn::<_, 3, _>(|i| used[i] + request[i]);
                        if (0..3).any(|i| next[i] > caps[i]) {
                            return Err(axis);
                        };
                        used = next;
                        Ok(())
                    },
                );
                if short {
                    assert_eq!(result, Err(Error::Budget(axis)))
                } else {
                    assert_eq!(result.unwrap().unwrap().as_str(), "chapter");
                    assert_eq!(used, caps)
                }
            }
        }
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                ALLOCATION_FAULT.with(|v| v.set(None));
            }
        }
        let mut completed = false;
        let mut failures = 0;
        for at in 0..100 {
            ALLOCATION_FAULT.with(|v| v.set(Some(at)));
            let _reset = Reset;
            let mut accepted = Vec::new();
            match cascade_book_v2_document_page_name_counted(
                &sheet,
                "paragraph",
                &[],
                &mut |r, s, w| {
                    accepted.push([r, s, w]);
                    Ok::<_, ()>(())
                },
            ) {
                Ok(result) => {
                    assert_eq!(result.unwrap().as_str(), "chapter");
                    assert_eq!(accepted, full);
                    completed = true;
                    break;
                }
                Err(Error::Allocation) => {
                    failures += 1;
                    assert_eq!(accepted, full[..accepted.len()]);
                    assert!(accepted.last().unwrap()[1] > 0);
                }
                Err(error) => panic!("unexpected: {error:?}"),
            }
        }
        assert!(completed && failures >= 3);
        ALLOCATION_FAULT.with(|v| v.set(Some(0)));
        let _reset = Reset;
        assert!(cascade_book_v2_document_page_name_counted(
            &StyleSheet { rules: vec![] },
            "paragraph",
            &[],
            &mut |_, _, _| Ok::<_, ()>(())
        )
        .unwrap()
        .is_none());
        eprintln!(
            "source page projection requests={} allocation_failures={failures} R/S/W={total:?}",
            full.len()
        );
    }

    #[test]
    fn counted_source_style_inheritance_equality_visits_actual_fields_without_allocating() {
        let base = semantic(0u8, &[], &sheet("paragraph"), None, &mut Context::legacy())
            .unwrap()
            .inheritance;
        let mut cases = vec![base.clone()];
        let mut other = base.clone();
        other.font_families = None;
        cases.push(other);
        let mut other = base.clone();
        other.font_families = Some(vec![]);
        cases.push(other);
        let mut other = base.clone();
        other.font_families.as_mut().unwrap()[1] = "Bodz".into();
        cases.push(other);
        let mut other = base.clone();
        other.font_size = None;
        cases.push(other);
        let mut other = base.clone();
        other.line_height = Some(PositiveLength::new(Length::from_raw(1).unwrap()).unwrap());
        cases.push(other);
        let mut other = base.clone();
        other.block_style.keep_caption = false;
        cases.push(other);
        let mut other = base.clone();
        other.block_style.width =
            MachineFigureWidth::Length(PositiveLength::new(Length::from_raw(19).unwrap()).unwrap());
        cases.push(other);
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                ALLOCATION_FAULT.with(|v| v.set(None));
            }
        }
        for other in cases {
            ALLOCATION_FAULT.with(|v| v.set(Some(0)));
            let _reset = Reset;
            let mut full = Vec::new();
            let equal = book_v2_inheritance_styles_equal_counted(&base, &other, &mut |r, s, w| {
                assert_eq!([r, s], [0, 0]);
                assert!(w > 0);
                full.push([r, s, w]);
                Ok::<_, ()>(())
            })
            .unwrap();
            assert_eq!(equal, base == other);
            for denied in 0..full.len() {
                let mut accepted = Vec::new();
                assert_eq!(
                    book_v2_inheritance_styles_equal_counted(&base, &other, &mut |r, s, w| {
                        if accepted.len() == denied {
                            return Err(denied);
                        };
                        accepted.push([r, s, w]);
                        Ok(())
                    }),
                    Err(Error::Budget(denied))
                );
                assert_eq!(accepted, full[..denied]);
            }
        }
    }
}
