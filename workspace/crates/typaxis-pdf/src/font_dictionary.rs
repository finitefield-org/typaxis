//! Shared physical font dictionary fields, independent of receipt authority.
use super::*;
#[cfg(feature = "book-v2-staging")]
use crate::font_encoding::Sink;

#[derive(Clone, Copy)]
pub(super) enum Role {
    Type0,
    CidFont,
    Descriptor,
}
#[derive(Clone, Copy)]
pub(super) enum Widths<'a> {
    TrueType(&'a [typaxis_font::CidBinding]),
    Cff(&'a [u32]),
    #[cfg(feature = "book-v2-staging")]
    Book {
        bindings: &'a [typaxis_resources::book_v2::BookV2CidBinding],
        notdef: Option<u32>,
    },
}
impl Widths<'_> {
    fn dense(&self) -> bool {
        match self {
            Self::TrueType(_) => false,
            Self::Cff(_) => true,
            #[cfg(feature = "book-v2-staging")]
            Self::Book { notdef, .. } => notdef.is_some(),
        }
    }
    fn len(&self) -> usize {
        match self {
            Self::TrueType(v) => v.len(),
            Self::Cff(v) => v.len(),
            #[cfg(feature = "book-v2-staging")]
            Self::Book { bindings, notdef } => bindings.len() + usize::from(notdef.is_some()),
        }
    }
    fn get(&self, index: usize) -> (u16, u32) {
        match self {
            Self::TrueType(v) => (v[index].cid.get(), v[index].width_1000),
            Self::Cff(v) => (index as u16, v[index]),
            #[cfg(feature = "book-v2-staging")]
            Self::Book { bindings, notdef } => {
                if let Some(width) = notdef {
                    if index == 0 {
                        return (0, *width);
                    }
                }
                let b = &bindings[index - usize::from(notdef.is_some())];
                (b.cid().get(), b.width_1000())
            }
        }
    }
    fn value(self) -> PdfValue {
        if self.dense() {
            PdfValue::Array(vec![
                PdfValue::Integer(0),
                PdfValue::Array(
                    (0..self.len())
                        .map(|i| PdfValue::Integer(i64::from(self.get(i).1)))
                        .collect(),
                ),
            ])
        } else {
            PdfValue::Array(
                (0..self.len())
                    .flat_map(|i| {
                        let (cid, width) = self.get(i);
                        [
                            PdfValue::Integer(i64::from(cid)),
                            PdfValue::Array(vec![PdfValue::Integer(i64::from(width))]),
                        ]
                    })
                    .collect(),
            )
        }
    }
    #[cfg(feature = "book-v2-staging")]
    fn write<S: Sink>(self, out: &mut S) -> Result<(), S::Error> {
        if self.dense() {
            out.extend(b"[0 [")?;
        } else {
            out.push(b'[')?;
        }
        for i in 0..self.len() {
            if i > 0 {
                out.push(b' ')?;
            }
            let (cid, width) = self.get(i);
            if !self.dense() {
                out.unsigned(u64::from(cid))?;
                out.extend(b" [")?;
            }
            out.unsigned(u64::from(width))?;
            if !self.dense() {
                out.push(b']')?;
            }
        }
        if self.dense() {
            out.push(b']')?;
        }
        out.push(b']')
    }
}
enum Field<'a> {
    Name(&'a [u8]),
    Integer(i64),
    Decimal(PdfDecimal),
    Reference(ObjectId),
    Descendant(ObjectId),
    Bbox([i32; 4]),
    SystemInfo,
    Widths(Widths<'a>),
}
impl Field<'_> {
    fn value(self) -> Result<PdfValue, PdfError> {
        Ok(match self {
            Self::Name(s) => PdfValue::Name(pdf_name(s)?),
            Self::Integer(n) => PdfValue::Integer(n),
            Self::Decimal(n) => PdfValue::Decimal(n),
            Self::Reference(id) => PdfValue::Reference(id),
            Self::Descendant(id) => PdfValue::Array(vec![PdfValue::Reference(id)]),
            Self::Bbox(b) => PdfValue::Array(
                b.into_iter()
                    .map(|n| PdfValue::Integer(i64::from(n)))
                    .collect(),
            ),
            Self::SystemInfo => {
                let mut d = PdfDictionary::new();
                d.insert(
                    pdf_name(b"Registry")?,
                    PdfValue::ByteString(b"Adobe".to_vec()),
                );
                d.insert(
                    pdf_name(b"Ordering")?,
                    PdfValue::ByteString(b"Identity".to_vec()),
                );
                d.insert(pdf_name(b"Supplement")?, PdfValue::Integer(0));
                PdfValue::Dictionary(d)
            }
            Self::Widths(w) => w.value(),
        })
    }
    #[cfg(feature = "book-v2-staging")]
    fn write<S: Sink>(self, out: &mut S) -> Result<(), S::Error> {
        match self {
            Self::Name(s) => font_encoding::name(out, s),
            Self::Integer(n) => font_encoding::decimal(
                out,
                PdfDecimal {
                    coefficient: i128::from(n),
                    scale: 0,
                },
            ),
            Self::Decimal(n) => font_encoding::decimal(out, n),
            Self::Reference(id) => reference(out, id),
            Self::Descendant(id) => {
                out.push(b'[')?;
                reference(out, id)?;
                out.push(b']')
            }
            Self::Bbox(b) => {
                out.push(b'[')?;
                for (i, n) in b.into_iter().enumerate() {
                    if i > 0 {
                        out.push(b' ')?;
                    }
                    Self::Integer(i64::from(n)).write(out)?;
                }
                out.push(b']')
            }
            Self::SystemInfo => out
                .extend(b"<< /Ordering <4964656E74697479> /Registry <41646F6265> /Supplement 0 >>"),
            Self::Widths(w) => w.write(out),
        }
    }
}
#[cfg(feature = "book-v2-staging")]
fn reference<S: Sink>(out: &mut S, id: ObjectId) -> Result<(), S::Error> {
    out.unsigned(u64::from(id.get()))?;
    out.extend(b" 0 R")
}
pub(super) struct FontDictionaryFields<'a> {
    pub kind: PdfFontProgramKind,
    pub name: &'a str,
    pub metrics: typaxis_resources::PdfFontMetrics,
    pub ids: FontObjectIds,
    pub widths: Widths<'a>,
}
impl<'a> FontDictionaryFields<'a> {
    pub fn from_legacy(plan: &'a FrozenPdfFontPlan, ids: FontObjectIds) -> Result<Self, PdfError> {
        Ok(Self {
            kind: plan.program_kind(),
            name: plan.embedded_postscript_name(),
            metrics: plan.metrics().clone(),
            ids,
            widths: match plan.program_kind() {
                PdfFontProgramKind::TrueTypeGlyf => Widths::TrueType(&plan.subset_plan().cids),
                PdfFontProgramKind::OpenTypeCff1 => Widths::Cff(
                    plan.cff1_plan()
                        .ok_or(PdfError::ResourcePlanMismatch)?
                        .dense_widths_1000(),
                ),
            },
        })
    }
    // Sorted field declarations also preserve legacy BTreeMap serialization.
    fn visit<E>(
        &self,
        role: Role,
        mut f: impl FnMut(&[u8], Field<'_>) -> Result<(), E>,
    ) -> Result<(), E> {
        use Field::*;
        let cff = self.kind == PdfFontProgramKind::OpenTypeCff1;
        let m = &self.metrics;
        match role {
            Role::Type0 => {
                f(b"BaseFont", Name(self.name.as_bytes()))?;
                f(b"DescendantFonts", Descendant(self.ids.cid_font))?;
                f(b"Encoding", Name(b"Identity-H"))?;
                f(b"Subtype", Name(b"Type0"))?;
                f(b"ToUnicode", Reference(self.ids.to_unicode))?;
                f(b"Type", Name(b"Font"))?;
            }
            Role::CidFont => {
                f(b"BaseFont", Name(self.name.as_bytes()))?;
                f(b"CIDSystemInfo", SystemInfo)?;
                if !cff {
                    f(b"CIDToGIDMap", Reference(self.ids.auxiliary))?;
                }
                f(b"DW", Integer(1000))?;
                f(b"FontDescriptor", Reference(self.ids.descriptor))?;
                f(
                    b"Subtype",
                    Name(if cff {
                        b"CIDFontType0"
                    } else {
                        b"CIDFontType2"
                    }),
                )?;
                f(b"Type", Name(b"Font"))?;
                if self.widths.len() > 0 {
                    f(b"W", Widths(self.widths))?;
                }
            }
            Role::Descriptor => {
                f(b"Ascent", Integer(i64::from(m.ascent_1000)))?;
                if cff {
                    f(b"CIDSet", Reference(self.ids.auxiliary))?;
                }
                f(b"CapHeight", Integer(i64::from(m.cap_height_1000)))?;
                f(b"Descent", Integer(i64::from(m.descent_1000)))?;
                f(b"Flags", Integer(i64::from(m.flags)))?;
                f(b"FontBBox", Bbox(m.bbox_1000))?;
                f(
                    if cff { b"FontFile3" } else { b"FontFile2" },
                    Reference(self.ids.font_program),
                )?;
                f(b"FontName", Name(self.name.as_bytes()))?;
                let angle = pdf_font_italic_angle(self.kind, m.italic_angle_fixed_16_16);
                f(b"ItalicAngle", Decimal(angle))?;
                f(b"StemV", Integer(i64::from(m.stem_v_1000)))?;
                f(b"Type", Name(b"FontDescriptor"))?;
            }
        }
        Ok(())
    }
    pub fn dictionary(&self, role: Role) -> Result<PdfDictionary, PdfError> {
        let mut d = PdfDictionary::new();
        self.visit(role, |key, field| {
            d.insert(pdf_name(key)?, field.value()?);
            Ok(())
        })?;
        Ok(d)
    }
    #[cfg(feature = "book-v2-staging")]
    pub fn write<S: Sink>(&self, role: Role, out: &mut S) -> Result<(), S::Error> {
        out.extend(b"<<")?;
        self.visit(role, |key, field| {
            out.push(b' ')?;
            font_encoding::name(out, key)?;
            out.push(b' ')?;
            field.write(out)
        })?;
        out.extend(b" >>")
    }
}
