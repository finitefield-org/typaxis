//! Closed input adapters for the shared paragraph and generated-label engine.
use super::*;
#[derive(Clone, Copy)]
pub(super) enum BodyFlow<'a> {
    Legacy(&'a ProductionTextFlow<'a>),
    #[cfg(feature = "book-v2-staging")]
    BookV2(&'a typaxis_syntax::book_v2::PreparedBookV2TextFlow<'a>),
    #[cfg(feature = "book-v2-staging")]
    PageRegion(&'a typaxis_syntax::book_v2::BookV2PageRegionTextFlow<'a>),
}
macro_rules! flow_call {
    ($flow:expr, $method:ident ( $($arg:expr),* )) => {
        match $flow {
            BodyFlow::Legacy(flow) => flow.$method($($arg),*),
            #[cfg(feature = "book-v2-staging")]
            BodyFlow::BookV2(flow) => flow.$method($($arg),*),
            #[cfg(feature = "book-v2-staging")]
            BodyFlow::PageRegion(flow) => flow.text_flow().$method($($arg),*),
        }
    };
}
pub(super) use flow_call;
#[derive(Clone, Copy)]
pub(super) enum BodyFonts<'a> {
    Legacy(&'a AdmittedResourceLedger),
    #[cfg(feature = "book-v2-staging")]
    BookV2(
        &'a typaxis_resource_admission::AdmittedProductionFontInstancesV3<'a>,
        &'a typaxis_resource_admission::AdmittedProductionResourceLedgerV3,
    ),
}
impl<'a> BodyFonts<'a> {
    pub fn font_families(self) -> &'a typaxis_font::FontFamilyTable {
        match self {
            Self::Legacy(v) => v.font_families(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(_, v) => v.font_families(),
        }
    }
    pub fn font(self, id: FontFaceId) -> Option<BodyFont<'a>> {
        match self {
            Self::Legacy(v) => v.font(id).map(BodyFont::Legacy),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(instances, _) => {
                // The constructor includes every declared face in canonical
                // dense order. Resolve the issued instance and check the join.
                let instance = instances.resolve(FontInstanceId::new(id.get()))?;
                (instance.font().font_face_id() == id).then_some(BodyFont::BookV2(instance))
            }
        }
    }
    pub fn fingerprint(self) -> [u8; 32] {
        match self {
            Self::Legacy(v) => v.fingerprint().bytes(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(_, v) => v.fingerprint(),
        }
    }
}
#[derive(Clone, Copy)]
pub(super) enum BodyFont<'a> {
    Legacy(&'a AdmittedFont),
    #[cfg(feature = "book-v2-staging")]
    BookV2(typaxis_resource_admission::AdmittedProductionFontInstanceV3<'a>),
}
impl<'a> BodyFont<'a> {
    pub fn bytes(self) -> &'a [u8] {
        match self {
            Self::Legacy(f) => f.bytes(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(f) => f.font().bytes(),
        }
    }
    pub fn face_index(self) -> u32 {
        match self {
            Self::Legacy(f) => f.face_index(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(f) => f.font().face_index(),
        }
    }
    pub fn content_hash(self) -> [u8; 32] {
        match self {
            Self::Legacy(f) => f.content_hash(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(f) => f.font().content_hash(),
        }
    }
    pub fn metadata(self) -> typaxis_resource_admission::AdmittedFontMetadata {
        match self {
            Self::Legacy(f) => f.metadata().clone(),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(f) => f.font().metadata(),
        }
    }
    pub fn instance_id(self) -> FontInstanceId {
        match self {
            Self::Legacy(f) => FontInstanceId::new(f.font_face_id().get()),
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(f) => f.font_instance_id(),
        }
    }
    pub fn coverage(self, text: &str) -> Result<(), ProductionTextShapeErrorKind> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::BookV2(instance) = self {
            if let typaxis_resource_admission::AdmittedProductionFontV3::Cff1V2(font) =
                instance.font()
            {
                return crate::cff_v2::coverage(font.admission(), text)
                    .map_err(ProductionTextShapeErrorKind::CffV2);
            }
        }
        let font = match self {
            Self::Legacy(font) => font,
            #[cfg(feature = "book-v2-staging")]
            Self::BookV2(instance) => match instance.font() {
                typaxis_resource_admission::AdmittedProductionFontV3::TrueType(font) => font,
                _ => unreachable!("CFF handled above"),
            },
        };
        validate_admitted_font_coverage(font, text).map_err(|e| match e {
            StagingEquationNumberShapeError::MissingDeclaredFontCoverage => {
                ProductionTextShapeErrorKind::MissingDeclaredFontCoverage
            }
            _ => ProductionTextShapeErrorKind::InvalidFontMetrics,
        })
    }
    pub fn run_coverage(
        self,
        text: &str,
        post_context: &str,
    ) -> Result<(), ProductionTextShapeErrorKind> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::BookV2(instance) = self {
            if let typaxis_resource_admission::AdmittedProductionFontV3::Cff1V2(font) =
                instance.font()
            {
                if post_context
                    .chars()
                    .next()
                    .is_some_and(|c| matches!(c as u32,0xfe00..=0xfe0f | 0xe0100..=0xe01ef))
                {
                    return Err(ProductionTextShapeErrorKind::CffV2(
                        Cff1ShapeErrorV2::SplitVariationSequence,
                    ));
                }
                return crate::cff_v2::coverage(font.admission(), text)
                    .map_err(ProductionTextShapeErrorKind::CffV2);
            }
        }
        let _ = (text, post_context);
        Ok(())
    }
}
