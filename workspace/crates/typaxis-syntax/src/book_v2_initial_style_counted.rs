//! Shared initial style preparation storage operations.
use super::*;
use std::convert::Infallible;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BookV2InitialStyleError<E> {
    Budget(E),
    Allocation,
    Syntax(StagingSemanticSyntaxError),
}
impl<E> From<StagingSemanticSyntaxError> for BookV2InitialStyleError<E> {
    fn from(e: StagingSemanticSyntaxError) -> Self {
        Self::Syntax(e)
    }
}
type Error<E> = BookV2InitialStyleError<E>;
pub(super) fn legacy_error(e: Error<Infallible>) -> StagingSemanticSyntaxError {
    match e {
        Error::Syntax(e) => e,
        Error::Budget(e) => match e {},
        Error::Allocation => {
            unreachable!("legacy allocations keep their original failure behavior")
        }
    }
}
pub(super) struct Context<'q, E> {
    charge: Option<&'q mut dyn FnMut(u64, u64, u64) -> Result<(), E>>,
}
impl Context<'_, Infallible> {
    pub(super) fn legacy() -> Self {
        Self { charge: None }
    }
}
impl<'q, E> Context<'q, E> {
    pub(super) fn counted(charge: &'q mut dyn FnMut(u64, u64, u64) -> Result<(), E>) -> Self {
        Self {
            charge: Some(charge),
        }
    }
    fn counted_mode(&self) -> bool {
        self.charge.is_some()
    }
    pub(super) fn charge(&mut self, r: u64, s: u64, w: u64) -> Result<(), Error<E>> {
        if let Some(f) = self.charge.as_deref_mut() {
            f(r, s, w).map_err(Error::Budget)?
        }
        Ok(())
    }
    pub(super) fn step(&mut self) -> Result<(), Error<E>> {
        self.charge(0, 0, 1)
    }
    pub(super) fn bytes(&mut self, n: usize) -> Result<(), Error<E>> {
        self.charge(0, 0, u64::try_from(n).map_err(|_| Error::Allocation)?)
    }
    pub(super) fn vector<T>(&mut self, n: usize) -> Result<Vec<T>, Error<E>> {
        if !self.counted_mode() {
            return Ok(Vec::new());
        }
        let bytes = n
            .checked_mul(std::mem::size_of::<T>())
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(Error::Allocation)?;
        self.charge(u64::try_from(n).map_err(|_| Error::Allocation)?, bytes, 1)?;
        allocation_attempt(bytes)?;
        let mut v = Vec::new();
        v.try_reserve_exact(n).map_err(|_| Error::Allocation)?;
        Ok(v)
    }
    fn string(&mut self, s: &str) -> Result<String, Error<E>> {
        if !self.counted_mode() {
            return Ok(s.to_owned());
        }
        self.charge(0, u64::try_from(s.len()).map_err(|_| Error::Allocation)?, 1)?;
        allocation_attempt(s.len() as u64)?;
        let mut v = String::new();
        v.try_reserve_exact(s.len())
            .map_err(|_| Error::Allocation)?;
        self.bytes(s.len())?;
        v.push_str(s);
        Ok(v)
    }
    fn mapped_selector(&mut self, prefix: &str, suffix: &str) -> Result<String, Error<E>> {
        if !self.counted_mode() {
            return Ok(format!("{prefix}{suffix}"));
        }
        let n = prefix
            .len()
            .checked_add(suffix.len())
            .ok_or(Error::Allocation)?;
        self.charge(0, n as u64, 1)?;
        allocation_attempt(n as u64)?;
        let mut s = String::new();
        s.try_reserve_exact(n).map_err(|_| Error::Allocation)?;
        self.bytes(n)?;
        s.push_str(prefix);
        s.push_str(suffix);
        Ok(s)
    }
    fn copy_value(&mut self, v: &StyleValue) -> Result<StyleValue, Error<E>> {
        if !self.counted_mode() {
            return Ok(v.clone());
        }
        self.step()?;
        Ok(match v {
            StyleValue::Keyword(v) => StyleValue::Keyword(self.string(v)?),
            StyleValue::Text(v) => StyleValue::Text(self.string(v)?),
            StyleValue::FontFamilyList(values) => {
                let mut out = self.vector(values.len())?;
                for s in values {
                    self.step()?;
                    out.push(self.string(s)?);
                }
                StyleValue::FontFamilyList(out)
            }
            StyleValue::Integer(v) => StyleValue::Integer(*v),
            StyleValue::Length(v) => StyleValue::Length(*v),
            StyleValue::Boolean(v) => StyleValue::Boolean(*v),
            StyleValue::Ratio {
                numerator,
                denominator,
            } => StyleValue::Ratio {
                numerator: *numerator,
                denominator: *denominator,
            },
        })
    }
    pub(super) fn lower_value(
        &mut self,
        v: &WireStagingStyleValue,
    ) -> Result<StyleValue, Error<E>> {
        self.step()?;
        Ok(match v {
            WireStagingStyleValue::Keyword { value } => StyleValue::Keyword(self.string(value)?),
            WireStagingStyleValue::String { value } => StyleValue::Text(self.string(value)?),
            WireStagingStyleValue::Integer { value } => StyleValue::Integer(*value),
            WireStagingStyleValue::Length { value } => StyleValue::Length(
                Length::from_raw(*value).ok_or(StagingSemanticSyntaxError::InvalidStyle)?,
            ),
            WireStagingStyleValue::Boolean { value } => StyleValue::Boolean(*value),
            WireStagingStyleValue::FontFamilyList { families } => {
                let out = if !self.counted_mode() {
                    families.clone()
                } else {
                    let mut out = self.vector(families.len())?;
                    for f in families {
                        self.step()?;
                        out.push(self.string(f)?);
                    }
                    out
                };
                StyleValue::FontFamilyList(out)
            }
            WireStagingStyleValue::Ratio {
                numerator,
                denominator,
            } => StyleValue::Ratio {
                numerator: *numerator,
                denominator: NonZeroU64::new(*denominator)
                    .ok_or(StagingSemanticSyntaxError::InvalidStyle)?,
            },
        })
    }
    fn copy_rule(&mut self, rule: &StyleRule) -> Result<StyleRule, Error<E>> {
        if !self.counted_mode() {
            return Ok(rule.clone());
        }
        self.step()?;
        self.bytes(rule.style_id.as_str().len())?;
        let style_id = StyleId::new(self.string(rule.style_id.as_str())?)
            .map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?;
        let extends = rule
            .extends
            .as_ref()
            .map(|v| {
                self.bytes(v.as_str().len())?;
                self.string(v.as_str()).and_then(|v| {
                    StyleId::new(v).map_err(|_| StagingSemanticSyntaxError::InvalidStyle.into())
                })
            })
            .transpose()?;
        let selector = self.string(&rule.selector)?;
        let mut declarations = self.vector(rule.declarations.len())?;
        for d in &rule.declarations {
            self.step()?;
            declarations.push(Declaration {
                name: self.string(&d.name)?,
                value: self.copy_value(&d.value)?,
                important: d.important,
            })
        }
        Ok(StyleRule {
            style_id,
            extends,
            selector,
            source_order: rule.source_order,
            declarations,
        })
    }
    fn copy_rules(&mut self, rules: &[StyleRule]) -> Result<Vec<StyleRule>, Error<E>> {
        if !self.counted_mode() {
            return Ok(rules.to_vec());
        }
        let mut out = self.vector(rules.len())?;
        for rule in rules {
            out.push(self.copy_rule(rule)?)
        }
        Ok(out)
    }
    fn validate(&mut self, sheet: &StyleSheet, kind: u8) -> Result<(), Error<E>> {
        #[cfg(feature = "book-v2-staging")]
        if let Some(charge) = self.charge.as_deref_mut() {
            use typaxis_style::book_v2::*;
            return match kind {
                0 => validate_book_v2_style_shape_counted(sheet, &mut |r, s, w| charge(r, s, w)),
                1 => {
                    validate_book_v2_document_styles_counted(sheet, &mut |r, s, w| charge(r, s, w))
                }
                2 => validate_book_v2_vector_styles_counted(sheet, &mut |r, s, w| charge(r, s, w)),
                _ => validate_book_v2_basic_styles_counted(sheet, &mut |r, s, w| charge(r, s, w)),
            }
            .map_err(style_error);
        }
        match kind {
            0 => sheet.validate_staging_precomposed_vector_style_shape(),
            1 => validate_semantic_table_sheet(sheet, true),
            2 => sheet.validate_precomposed_vector_styles(),
            _ => sheet.validate_basic_document_styles(),
        }
        .map_err(|e| Error::Syntax(map_semantic_style_error(e)))
    }
    fn find_rule(&mut self, rules: &[StyleRule], id: &StyleId) -> Result<Option<usize>, Error<E>> {
        for (i, r) in rules.iter().enumerate() {
            self.step()?;
            self.bytes(r.style_id.as_str().len().min(id.as_str().len()))?;
            if r.style_id == *id {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    pub(super) fn copy_inheritance(
        &mut self,
        v: &SemanticContainerInheritanceStyle,
    ) -> Result<SemanticContainerInheritanceStyle, Error<E>> {
        #[cfg(feature = "book-v2-staging")]
        if let Some(charge) = self.charge.as_deref_mut() {
            return v
                .try_copy_book_v2_counted(&mut |r, s, w| charge(r, s, w))
                .map_err(|e| match e {
                    typaxis_style::book_v2::BookV2StyleCopyError::Budget(e) => Error::Budget(e),
                    typaxis_style::book_v2::BookV2StyleCopyError::Allocation => Error::Allocation,
                });
        }
        Ok(v.clone())
    }
    pub(super) fn descendant(
        &mut self,
        rules: &StagingSemanticStyleSheets,
        block: &str,
        classes: &[String],
        sheet: &StyleSheet,
        parent: Option<&SemanticContainerInheritanceStyle>,
    ) -> Result<SemanticContainerInheritanceStyle, Error<E>> {
        #[cfg(feature = "book-v2-staging")]
        if let Some(charge) = self.charge.as_deref_mut() {
            return typaxis_style::book_v2::cascade_book_v2_descendant_style_counted(
                block,
                classes,
                sheet,
                parent,
                &mut |r, s, w| charge(r, s, w),
            )
            .map_err(style_error);
        }
        rules
            .descendant_style(block, classes, sheet, parent)
            .map_err(|e| Error::Syntax(map_semantic_style_error(e)))
    }
    pub(super) fn semantic<C: Copy, K: Copy>(
        &mut self,
        legacy: ContainerStyleCascade<C, K>,
        convert: Option<fn(C) -> K>,
        kind: C,
        classes: &[String],
        sheet: &StyleSheet,
        parent: Option<&SemanticContainerInheritanceStyle>,
    ) -> Result<typaxis_style::ComputedSemanticContainerStyle<K>, Error<E>> {
        #[cfg(feature = "book-v2-staging")]
        if let Some(charge) = self.charge.as_deref_mut() {
            return typaxis_style::book_v2::cascade_book_v2_container_kind_counted(
                convert.expect("authentic counted kind conversion")(kind),
                classes,
                sheet,
                parent,
                &mut |r, s, w| charge(r, s, w),
            )
            .map_err(style_error);
        }
        let _ = convert;
        legacy(kind, classes, sheet, parent).map_err(|e| Error::Syntax(map_semantic_style_error(e)))
    }
    pub(super) fn inline(
        &mut self,
        v: &SemanticContainerInheritanceStyle,
    ) -> Result<StagingMathComputedStyle, Error<E>> {
        #[cfg(feature = "book-v2-staging")]
        if let Some(charge) = self.charge.as_deref_mut() {
            return typaxis_style::book_v2::close_book_v2_inline_math_style_counted(
                v,
                &mut |r, s, w| charge(r, s, w),
            )
            .map_err(style_error);
        }
        close_staging_inline_math_style(v).map_err(|e| Error::Syntax(map_semantic_style_error(e)))
    }
    pub(super) fn display(
        &mut self,
        classes: &[String],
        sheet: &StyleSheet,
        parent: Option<&SemanticContainerInheritanceStyle>,
    ) -> Result<StagingMathComputedStyle, Error<E>> {
        #[cfg(feature = "book-v2-staging")]
        if let Some(charge) = self.charge.as_deref_mut() {
            return typaxis_style::book_v2::cascade_book_v2_display_math_style_counted(
                classes,
                sheet,
                parent,
                &mut |r, s, w| charge(r, s, w),
            )
            .map_err(style_error);
        }
        cascade_staging_display_math_style(classes, sheet, parent)
            .map_err(|e| Error::Syntax(map_semantic_style_error(e)))
    }
    pub(super) fn vector_style(
        &mut self,
        sheet: &StyleSheet,
        kind: PrecomposedVectorStyleKind,
        classes: &[String],
        parent: Option<&SemanticContainerInheritanceStyle>,
    ) -> Result<PrecomposedVectorComputedStyleReceipt, Error<E>> {
        #[cfg(feature = "book-v2-staging")]
        if let Some(charge) = self.charge.as_deref_mut() {
            return typaxis_style::book_v2::cascade_book_v2_vector_style_counted(
                sheet,
                kind,
                classes,
                parent,
                &mut |r, s, w| charge(r, s, w),
            )
            .map_err(style_error);
        }
        sheet
            .cascade_precomposed_vector_style(kind, classes, parent)
            .map_err(|e| Error::Syntax(map_semantic_style_error(e)))
    }
    pub(super) fn cell(
        &mut self,
        owner: NodeId,
        classes: &[String],
        rules: &StagingSemanticStyleSheets,
        parent: &SemanticContainerInheritanceStyle,
    ) -> Result<SemanticContainerInheritanceStyle, Error<E>> {
        #[cfg(feature = "book-v2-staging")]
        if let Some(sheet) = &rules.table_cells {
            if sheet.rules.is_empty() {
                return self.copy_inheritance(parent);
            }
            return self.descendant(rules, "paragraph", classes, sheet, Some(parent));
        }
        if !classes.is_empty() {
            return Err(StagingSemanticSyntaxError::TableCellStyleStaging(owner).into());
        }
        self.copy_inheritance(parent)
    }
}
#[cfg(feature = "book-v2-staging")]
fn style_error<E>(e: typaxis_style::book_v2::BookV2StylePreparationError<E>) -> Error<E> {
    match e {
        typaxis_style::book_v2::BookV2StylePreparationError::Budget(e) => Error::Budget(e),
        typaxis_style::book_v2::BookV2StylePreparationError::Allocation => Error::Allocation,
        typaxis_style::book_v2::BookV2StylePreparationError::Style(e) => {
            Error::Syntax(map_semantic_style_error(e))
        }
    }
}

enum Flags {
    Legacy(Vec<bool>),
    Counted(Vec<u8>),
}
impl Flags {
    fn new<E>(n: usize, ctx: &mut Context<'_, E>) -> Result<Self, Error<E>> {
        Ok(if ctx.counted_mode() {
            Self::Counted(ctx.vector(n)?)
        } else {
            Self::Legacy(Vec::new())
        })
    }
    fn push(&mut self, v: bool) {
        match self {
            Self::Legacy(a) => a.push(v),
            Self::Counted(a) => a.push(u8::from(v)),
        }
    }
    fn get(&self, i: usize) -> bool {
        match self {
            Self::Legacy(a) => a[i],
            Self::Counted(a) => a[i] != 0,
        }
    }
    fn len(&self) -> usize {
        match self {
            Self::Legacy(a) => a.len(),
            Self::Counted(a) => a.len(),
        }
    }
}

pub(super) fn lower<E>(
    sheet: &WireStagingStyleSheet,
    limits: &ValidatedResourceLimits,
    book_v2: bool,
    ctx: &mut Context<'_, E>,
) -> Result<StagingSemanticStyleSheets, Error<E>> {
    let rules = &sheet.rules;
    if u64::try_from(rules.len()).map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?
        > limits.get().max_style_rules
    {
        return Err(StagingSemanticSyntaxError::InvalidStyle.into());
    }
    let mut parsed = ctx.vector(rules.len())?;
    let mut semantic = Flags::new(rules.len(), ctx)?;
    let mut math = Flags::new(rules.len(), ctx)?;
    let mut descriptions = Flags::new(rules.len(), ctx)?;
    let mut terms = Flags::new(rules.len(), ctx)?;
    let mut cells = Flags::new(rules.len(), ctx)?;
    let mut vectors = Flags::new(rules.len(), ctx)?;
    for (index, rule) in rules.iter().enumerate() {
        ctx.step()?;
        let source_order = rule.source_order;
        if usize::try_from(source_order) != Ok(index) {
            return Err(StagingSemanticSyntaxError::InvalidStyle.into());
        }
        let selector = rule.selector.as_str();
        ctx.bytes(selector.len())?;
        let mut parts = selector.split('.');
        let block = parts
            .next()
            .ok_or(StagingSemanticSyntaxError::InvalidStyle)?;
        let description = book_v2 && matches!(block, "description_list" | "description_term");
        let cell = book_v2 && block == "table_cell";
        if !description
            && !cell
            && !matches!(
                block,
                "paragraph"
                    | "heading"
                    | "list"
                    | "table"
                    | "figure"
                    | "page_break"
                    | "semantic_container"
                    | "display_math"
                    | "math_vector_block"
                    | "vector_figure"
            )
        {
            return Err(StagingSemanticSyntaxError::InvalidStyle.into());
        }
        let classes = if ctx.counted_mode() {
            ctx.bytes(selector.len())?;
            let mut v = ctx.vector(parts.clone().count())?;
            for p in parts {
                v.push(ctx.string(p)?)
            }
            v
        } else {
            parts.map(str::to_owned).collect()
        };
        ctx.step()?;
        for c in &classes {
            ctx.step()?;
            // Hidden-class comparison, identifier scan, and the ordered
            // adjacent comparison each receive a prepaid UTF-8 bound.
            ctx.bytes(c.len())?;
            ctx.bytes(c.len())?;
            ctx.bytes(c.len())?;
        }
        validate_classes(&classes)?;
        ctx.bytes(rule.style_id.len())?;
        let style_id = StyleId::new(ctx.string(&rule.style_id)?)
            .map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?;
        let extends = rule
            .extends
            .as_ref()
            .map(|v| {
                ctx.bytes(v.len())?;
                ctx.string(v).and_then(|v| {
                    StyleId::new(v).map_err(|_| StagingSemanticSyntaxError::InvalidStyle.into())
                })
            })
            .transpose()?;
        let mut declarations = ctx.vector(rule.declarations.len())?;
        for d in &rule.declarations {
            ctx.step()?;
            ctx.bytes(d.name.len())?;
            if BasicStyleProperty::from_str(&d.name).is_none() {
                return Err(StagingSemanticSyntaxError::InvalidStyle.into());
            }
            declarations.push(Declaration {
                name: ctx.string(&d.name)?,
                value: ctx.lower_value(&d.value)?,
                important: d.important,
            })
        }
        let mapped =
            if description || cell || matches!(block, "semantic_container" | "display_math") {
                ctx.mapped_selector("paragraph", &selector[block.len()..])?
            } else {
                ctx.string(selector)?
            };
        parsed.push(StyleRule {
            style_id,
            extends,
            selector: mapped,
            source_order,
            declarations,
        });
        descriptions.push(block == "description_list");
        terms.push(block == "description_term");
        cells.push(cell);
        semantic.push(block == "semantic_container");
        math.push(block == "display_math");
        vectors.push(matches!(block, "math_vector_block" | "vector_figure"));
    }
    let validation = StyleSheet {
        rules: ctx.copy_rules(&parsed)?,
    };
    ctx.validate(&validation, 0)?;
    let by_id = if ctx.counted_mode() {
        None
    } else {
        Some(
            parsed
                .iter()
                .enumerate()
                .map(|(i, r)| (&r.style_id, i))
                .collect::<BTreeMap<_, _>>(),
        )
    };
    for (index, rule) in parsed.iter().enumerate() {
        ctx.step()?;
        if let Some(parent) = rule.extends.as_ref() {
            let parent_index = if let Some(map) = &by_id {
                map.get(parent).copied()
            } else {
                ctx.find_rule(&parsed, parent)?
            }
            .ok_or(StagingSemanticSyntaxError::InvalidStyle)?;
            if vectors.get(index) != vectors.get(parent_index) {
                return Err(StagingSemanticSyntaxError::InvalidStyle.into());
            }
        }
    }
    let mut current = ctx.vector(parsed.len())?;
    let mut vector = ctx.vector(parsed.len())?;
    let mut current_semantic = Flags::new(parsed.len(), ctx)?;
    let mut current_math = Flags::new(parsed.len(), ctx)?;
    let mut current_description = Flags::new(parsed.len(), ctx)?;
    let mut current_term = Flags::new(parsed.len(), ctx)?;
    let mut current_cell = Flags::new(parsed.len(), ctx)?;
    for (index, mut rule) in parsed.into_iter().enumerate() {
        ctx.step()?;
        if vectors.get(index) {
            rule.source_order = u32::try_from(vector.len())
                .map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?;
            vector.push(rule)
        } else {
            rule.source_order = u32::try_from(current.len())
                .map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?;
            current.push(rule);
            current_semantic.push(semantic.get(index));
            current_math.push(math.get(index));
            current_description.push(descriptions.get(index));
            current_term.push(terms.get(index));
            current_cell.push(cells.get(index));
        }
    }
    let validation = StyleSheet {
        rules: ctx.copy_rules(&current)?,
    };
    if ctx.counted_mode() {
        ctx.validate(&validation, 1)?
    } else {
        validate_semantic_table_sheet(&validation, book_v2).map_err(map_semantic_style_error)?
    }
    let vector = StyleSheet { rules: vector };
    ctx.validate(&vector, 2)?;
    let mut ordinary = ctx.copy_rules(&current)?;
    for (index, rule) in ordinary.iter_mut().enumerate() {
        ctx.step()?;
        if current_semantic.get(index)
            || current_math.get(index)
            || current_description.get(index)
            || current_term.get(index)
            || current_cell.get(index)
        {
            rule.selector = ctx.mapped_selector("paragraph.", INTERNAL_HIDDEN_STYLE_CLASS)?
        }
    }
    let ordinary = StyleSheet { rules: ordinary };
    if ctx.counted_mode() {
        ctx.validate(&ordinary, 1)?
    } else {
        validate_semantic_table_sheet(&ordinary, book_v2).map_err(map_semantic_style_error)?
    }
    let semantic = isolate(&current, &current_semantic, ctx)?;
    let math = isolate(&current, &current_math, ctx)?;
    #[cfg(feature = "book-v2-staging")]
    let descriptions = if book_v2 {
        Some((
            isolate(&current, &current_description, ctx)?,
            isolate(&current, &current_term, ctx)?,
        ))
    } else {
        None
    };
    #[cfg(feature = "book-v2-staging")]
    let table_cells = if book_v2 {
        let sheet = isolate(&current, &current_cell, ctx)?;
        for d in sheet.rules.iter().flat_map(|r| &r.declarations) {
            ctx.step()?;
            if !matches!(
                d.name.as_str(),
                "text_align" | "font_family" | "font_size" | "line_height"
            ) {
                return Err(StagingSemanticSyntaxError::InapplicableStyle.into());
            }
        }
        Some(sheet)
    } else {
        None
    };
    Ok(StagingSemanticStyleSheets {
        #[cfg(feature = "book-v2-staging")]
        book_v2,
        #[cfg(feature = "book-v2-staging")]
        table_cells,
        #[cfg(feature = "book-v2-staging")]
        descriptions,
        semantic,
        ordinary,
        math,
        vector,
    })
}
fn isolate<E>(
    parsed: &[StyleRule],
    direct: &Flags,
    ctx: &mut Context<'_, E>,
) -> Result<StyleSheet, Error<E>> {
    let by_id = if ctx.counted_mode() {
        None
    } else {
        Some(
            parsed
                .iter()
                .enumerate()
                .map(|(i, r)| (&r.style_id, i))
                .collect::<BTreeMap<_, _>>(),
        )
    };
    let mut legacy = BTreeSet::new();
    let mut counted = if ctx.counted_mode() {
        let mut v = ctx.vector(parsed.len())?;
        ctx.charge(0, 0, parsed.len() as u64)?;
        v.resize(parsed.len(), 0u8);
        v
    } else {
        Vec::new()
    };
    for index in 0..direct.len() {
        ctx.step()?;
        if !direct.get(index) {
            continue;
        }
        let mut current = Some(index);
        while let Some(i) = current {
            ctx.step()?;
            let fresh = if ctx.counted_mode() {
                let was = counted[i] == 0;
                counted[i] = 1;
                was
            } else {
                legacy.insert(i)
            };
            if !fresh {
                break;
            }
            current = if let Some(parent) = parsed[i].extends.as_ref() {
                if let Some(map) = &by_id {
                    map.get(parent).copied()
                } else {
                    ctx.find_rule(parsed, parent)?
                }
            } else {
                None
            };
        }
    }
    let mut rules = ctx.vector(parsed.len())?;
    for (index, source) in parsed.iter().enumerate() {
        ctx.step()?;
        let mut rule = ctx.copy_rule(source)?;
        if if ctx.counted_mode() {
            counted[index] == 0
        } else {
            !legacy.contains(&index)
        } {
            continue;
        }
        for d in &rule.declarations {
            ctx.step()?;
            if matches!(
                BasicStyleProperty::from_str(&d.name),
                Some(BasicStyleProperty::Width | BasicStyleProperty::KeepCaption)
            ) {
                return Err(StagingSemanticSyntaxError::InapplicableStyle.into());
            }
        }
        if !direct.get(index) {
            rule.selector = ctx.mapped_selector("paragraph.", INTERNAL_HIDDEN_STYLE_CLASS)?
        }
        rule.source_order =
            u32::try_from(rules.len()).map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?;
        rules.push(rule);
    }
    let sheet = StyleSheet { rules };
    ctx.validate(&sheet, 3)?;
    Ok(sheet)
}

pub(super) trait Output<T> {
    fn insert<E>(
        &mut self,
        owner: NodeId,
        value: T,
        ctx: &mut Context<'_, E>,
    ) -> Result<bool, Error<E>>;
}
impl<T> Output<T> for BTreeMap<NodeId, T> {
    fn insert<E>(
        &mut self,
        owner: NodeId,
        value: T,
        _: &mut Context<'_, E>,
    ) -> Result<bool, Error<E>> {
        Ok(BTreeMap::insert(self, owner, value).is_some())
    }
}
pub(super) struct CountedRegistry<T>(pub(super) Vec<(NodeId, T)>, usize);
impl<T> CountedRegistry<T> {
    pub(super) fn new<E>(n: usize, ctx: &mut Context<'_, E>) -> Result<Self, Error<E>> {
        Ok(Self(ctx.vector(n)?, n))
    }
    pub(super) fn finish<E>(
        mut self,
        ctx: &mut Context<'_, E>,
    ) -> Result<Vec<(NodeId, T)>, Error<E>> {
        fn sift<T, E>(
            items: &mut [(NodeId, T)],
            mut root: usize,
            end: usize,
            ctx: &mut Context<'_, E>,
        ) -> Result<(), Error<E>> {
            loop {
                let Some(mut child) = root
                    .checked_mul(2)
                    .and_then(|v| v.checked_add(1))
                    .filter(|v| *v < end)
                else {
                    return Ok(());
                };
                if child + 1 < end {
                    ctx.step()?;
                    if items[child].0 < items[child + 1].0 {
                        child += 1
                    }
                }
                ctx.step()?;
                if items[root].0 >= items[child].0 {
                    return Ok(());
                }
                ctx.step()?;
                items.swap(root, child);
                root = child;
            }
        }
        for root in (0..self.0.len() / 2).rev() {
            let end = self.0.len();
            sift(&mut self.0, root, end, ctx)?
        }
        for end in (1..self.0.len()).rev() {
            ctx.step()?;
            self.0.swap(0, end);
            sift(&mut self.0, 0, end, ctx)?
        }
        Ok(self.0)
    }
}
impl<T> Output<T> for CountedRegistry<T> {
    fn insert<E>(
        &mut self,
        owner: NodeId,
        value: T,
        ctx: &mut Context<'_, E>,
    ) -> Result<bool, Error<E>> {
        for (key, _) in &self.0 {
            ctx.step()?;
            if *key == owner {
                return Ok(true);
            }
        }
        ctx.step()?;
        if self.0.len() == self.1 {
            return Err(Error::Allocation);
        }
        self.0.push((owner, value));
        Ok(false)
    }
}

pub(super) fn census<C: Copy, E>(
    blocks: &[SemanticBlock<C>],
    counts: &mut [usize; 2],
    ctx: &mut Context<'_, E>,
) -> Result<(), Error<E>> {
    for block in blocks {
        ctx.step()?;
        match block {
            SemanticBlock::SemanticContainer { blocks, .. } => {
                counts[0] = counts[0].checked_add(1).ok_or(Error::Allocation)?;
                census(blocks, counts, ctx)?
            }
            SemanticBlock::MathVectorBlock { .. } => {
                counts[1] = counts[1].checked_add(1).ok_or(Error::Allocation)?
            }
            SemanticBlock::VectorFigure { caption, .. } => {
                counts[1] = counts[1].checked_add(1).ok_or(Error::Allocation)?;
                census(caption, counts, ctx)?
            }
            SemanticBlock::Figure { caption, .. } => census(caption, counts, ctx)?,
            SemanticBlock::List { items, .. } => {
                for item in items {
                    ctx.step()?;
                    census(&item.blocks, counts, ctx)?
                }
            }
            SemanticBlock::DescriptionList { items, .. } => {
                for item in items {
                    ctx.step()?;
                    census(&item.blocks, counts, ctx)?
                }
            }
            SemanticBlock::Table {
                caption,
                head,
                body,
                ..
            } => {
                census(caption, counts, ctx)?;
                for row in head.iter().chain(body) {
                    ctx.step()?;
                    for cell in &row.cells {
                        ctx.step()?;
                        census(&cell.blocks, counts, ctx)?
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn collect<C: Copy, K: Copy, M: AsRef<StagingM4MathNode>, E>(
    cascade_kind: ContainerStyleCascade<C, K>,
    convert: Option<fn(C) -> K>,
    blocks: &[SemanticBlock<C>],
    rules: &StagingSemanticStyleSheets,
    parent: Option<&SemanticContainerInheritanceStyle>,
    pending: &[M],
    output: &mut impl Output<typaxis_style::ComputedSemanticContainerStyle<K>>,
    vectors: &mut impl Output<PrecomposedVectorComputedStyleReceipt>,
    math: &mut impl Output<StagingMathComputedStyle>,
    ctx: &mut Context<'_, E>,
) -> Result<(), Error<E>> {
    for block in blocks {
        ctx.step()?;
        match block {
            SemanticBlock::SemanticContainer {
                common,
                semantic_kind,
                blocks,
            } => {
                let style = ctx.semantic(
                    cascade_kind,
                    convert,
                    *semantic_kind,
                    &common.classes,
                    &rules.semantic,
                    parent,
                )?;
                let inherited = ctx.copy_inheritance(style.inheritance_style())?;
                if output.insert(common.node_id, style, ctx)? {
                    return Err(StagingSemanticSyntaxError::InvalidNodeOrder.into());
                }
                collect(
                    cascade_kind,
                    convert,
                    blocks,
                    rules,
                    Some(&inherited),
                    pending,
                    output,
                    vectors,
                    math,
                    ctx,
                )?;
            }
            SemanticBlock::Paragraph { common, .. } | SemanticBlock::Heading { common, .. } => {
                let name = if matches!(block, SemanticBlock::Paragraph { .. }) {
                    "paragraph"
                } else {
                    "heading"
                };
                let inherited =
                    ctx.descendant(rules, name, &common.classes, &rules.ordinary, parent)?;
                for value in pending {
                    ctx.step()?;
                    let value = value.as_ref();
                    if value.owner_node_id == common.node_id
                        && value.kind == StagingM4MathKind::Inline
                    {
                        let style = ctx.inline(&inherited)?;
                        if math.insert(value.node_id, style, ctx)? {
                            return Err(StagingSemanticSyntaxError::InvalidNodeOrder.into());
                        }
                    }
                }
            }
            SemanticBlock::DisplayMath { common } => {
                let style = ctx.display(&common.classes, &rules.math, parent)?;
                let mut found = false;
                for value in pending {
                    ctx.step()?;
                    let value = value.as_ref();
                    if value.node_id == common.node_id && value.kind == StagingM4MathKind::Display {
                        found = true;
                        break;
                    }
                }
                if !found || math.insert(common.node_id, style, ctx)? {
                    return Err(StagingSemanticSyntaxError::ReceiptMismatch.into());
                }
            }
            #[cfg(not(feature = "book-v2-staging"))]
            SemanticBlock::DescriptionList { common, .. } => {
                return Err(
                    StagingSemanticSyntaxError::DescriptionListStaging(common.node_id).into(),
                )
            }
            #[cfg(feature = "book-v2-staging")]
            SemanticBlock::DescriptionList { common, items } => {
                let (list_rules, term_rules) = rules.descriptions.as_ref().ok_or(
                    StagingSemanticSyntaxError::DescriptionListStaging(common.node_id),
                )?;
                let inherited =
                    ctx.descendant(rules, "paragraph", &common.classes, list_rules, parent)?;
                for item in items {
                    ctx.step()?;
                    let term = &item.term.common;
                    let term_style = ctx.descendant(
                        rules,
                        "paragraph",
                        &term.classes,
                        term_rules,
                        Some(&inherited),
                    )?;
                    for value in pending {
                        ctx.step()?;
                        let value = value.as_ref();
                        if value.owner_node_id == term.node_id
                            && value.kind == StagingM4MathKind::Inline
                        {
                            let style = ctx.inline(&term_style)?;
                            if math.insert(value.node_id, style, ctx)? {
                                return Err(StagingSemanticSyntaxError::InvalidNodeOrder.into());
                            }
                        }
                    }
                    collect(
                        cascade_kind,
                        convert,
                        &item.blocks,
                        rules,
                        Some(&inherited),
                        pending,
                        output,
                        vectors,
                        math,
                        ctx,
                    )?;
                }
            }
            SemanticBlock::List { common, items } => {
                let inherited =
                    ctx.descendant(rules, "list", &common.classes, &rules.ordinary, parent)?;
                for item in items {
                    ctx.step()?;
                    collect(
                        cascade_kind,
                        convert,
                        &item.blocks,
                        rules,
                        Some(&inherited),
                        pending,
                        output,
                        vectors,
                        math,
                        ctx,
                    )?
                }
            }
            SemanticBlock::Table {
                common,
                caption,
                head,
                body,
            } => {
                let inherited =
                    ctx.descendant(rules, "table", &common.classes, &rules.ordinary, parent)?;
                collect(
                    cascade_kind,
                    convert,
                    caption,
                    rules,
                    Some(&inherited),
                    pending,
                    output,
                    vectors,
                    math,
                    ctx,
                )?;
                for row in head.iter().chain(body) {
                    ctx.step()?;
                    for cell in &row.cells {
                        ctx.step()?;
                        let cell_style =
                            ctx.cell(cell.node_id, &cell.classes, rules, &inherited)?;
                        collect(
                            cascade_kind,
                            convert,
                            &cell.blocks,
                            rules,
                            Some(&cell_style),
                            pending,
                            output,
                            vectors,
                            math,
                            ctx,
                        )?
                    }
                }
            }
            SemanticBlock::Figure {
                common, caption, ..
            } => {
                let inherited =
                    ctx.descendant(rules, "figure", &common.classes, &rules.ordinary, parent)?;
                collect(
                    cascade_kind,
                    convert,
                    caption,
                    rules,
                    Some(&inherited),
                    pending,
                    output,
                    vectors,
                    math,
                    ctx,
                )?;
            }
            SemanticBlock::VectorFigure {
                common, caption, ..
            } => {
                let style = ctx.vector_style(
                    &rules.vector,
                    PrecomposedVectorStyleKind::VectorFigure,
                    &common.classes,
                    parent,
                )?;
                if vectors.insert(common.node_id, style, ctx)? {
                    return Err(StagingSemanticSyntaxError::InvalidNodeOrder.into());
                }
                collect(
                    cascade_kind,
                    convert,
                    caption,
                    rules,
                    parent,
                    pending,
                    output,
                    vectors,
                    math,
                    ctx,
                )?;
            }
            SemanticBlock::MathVectorBlock { common, .. } => {
                let style = ctx.vector_style(
                    &rules.vector,
                    PrecomposedVectorStyleKind::MathVectorBlock,
                    &common.classes,
                    parent,
                )?;
                if vectors.insert(common.node_id, style, ctx)? {
                    return Err(StagingSemanticSyntaxError::InvalidNodeOrder.into());
                }
            }
            SemanticBlock::PageBreak { .. } => {}
        }
    }
    Ok(())
}

#[cfg(test)]
thread_local! { static ALLOCATION_FAULT: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) }; }
fn allocation_attempt<E>(bytes: u64) -> Result<(), Error<E>> {
    if bytes == 0 {
        return Ok(());
    }
    #[cfg(test)]
    return ALLOCATION_FAULT.with(|slot| match slot.get() {
        None => Ok(()),
        Some(0) => Err(Error::Allocation),
        Some(n) => {
            slot.set(Some(n - 1));
            Ok(())
        }
    });
    #[cfg(not(test))]
    Ok(())
}
#[cfg(test)]
pub(super) fn with_allocation_fault<R>(at: usize, run: impl FnOnce() -> R) -> R {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ALLOCATION_FAULT.with(|slot| slot.set(None));
        }
    }
    ALLOCATION_FAULT.with(|slot| slot.set(Some(at)));
    let _reset = Reset;
    run()
}
