use super::*;

/// Ordinary source Figure placed by the common block selection. It has no
/// producer metrics or math binding; content identity comes from admission.
#[derive(Debug)]
pub struct ProductionBodySvgFigureDraw<'d> {
    pub(super) source: &'d typaxis_syntax::ProductionFigure<'d>,
    pub(super) page_index: u32,
    pub(super) fragment_index: u32,
    pub(super) content_key: VectorContentKey,
    pub(super) scale_raw: i32,
    pub(super) viewport: Rect,
    pub(super) fingerprint: [u8; 32],
}
impl<'d> ProductionBodySvgFigureDraw<'d> {
    pub fn source(&self) -> &'d typaxis_syntax::ProductionFigure<'d> {
        self.source
    }
    pub fn owner(&self) -> NodeId {
        self.source.owner()
    }
    pub fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub fn viewport(&self) -> Rect {
        self.viewport
    }
    pub fn alternative(&self) -> &'d str {
        self.source.alternative()
    }
}

/// Borrowed vector paint facts shared by ordinary figures and precomposed
/// content. Source variants remain distinct through structure and manifests.
#[derive(Clone, Copy)]
pub enum ProductionVectorPaint<'v, 'd> {
    Precomposed(&'v ProductionBodyVectorDraw<'d>),
    Figure(&'v ProductionBodySvgFigureDraw<'d>),
}
impl<'d> ProductionBodyDraw<'d> {
    pub fn vector_paint(&self) -> Option<ProductionVectorPaint<'_, 'd>> {
        match self {
            Self::Vector(v) => Some(ProductionVectorPaint::Precomposed(v)),
            Self::SvgFigure(v) => Some(ProductionVectorPaint::Figure(v)),
            _ => None,
        }
    }
}
impl ProductionVectorPaint<'_, '_> {
    pub fn owner(self) -> NodeId {
        match self {
            Self::Precomposed(v) => v.binding().node_id(),
            Self::Figure(v) => v.owner(),
        }
    }
    pub fn image_id(self) -> typaxis_core::ImageResourceId {
        match self {
            Self::Precomposed(v) => v.binding().resource().image_id(),
            Self::Figure(v) => v.source.image_id(),
        }
    }
    pub fn kind(self) -> crate::StagingCombinedVectorKindV2 {
        match self {
            Self::Precomposed(v) => v.binding().kind().into(),
            Self::Figure(_) => crate::StagingCombinedVectorKindV2::Figure,
        }
    }
    pub fn content_key(self) -> VectorContentKey {
        match self {
            Self::Precomposed(v) => v.content_key(),
            Self::Figure(v) => v.content_key,
        }
    }
    pub fn page_index(self) -> u32 {
        match self {
            Self::Precomposed(v) => v.page_index(),
            Self::Figure(v) => v.page_index,
        }
    }
    pub fn fragment_index(self) -> u32 {
        match self {
            Self::Precomposed(v) => v.fragment_index(),
            Self::Figure(v) => v.fragment_index,
        }
    }
    pub fn viewport(self) -> Rect {
        match self {
            Self::Precomposed(v) => v.viewport(),
            Self::Figure(v) => v.viewport,
        }
    }
    pub fn scale_raw(self) -> i32 {
        match self {
            Self::Precomposed(v) => v.scale_raw(),
            Self::Figure(v) => v.scale_raw,
        }
    }
    pub fn matrix(self) -> AffineTransform {
        placement_matrix(self.viewport(), self.scale_raw())
    }
    pub fn resolved_current_color(self) -> [u8; 3] {
        match self {
            Self::Precomposed(v) => v.resolved_current_color(),
            Self::Figure(_) => [0, 0, 0],
        }
    }
    pub fn fingerprint(self) -> [u8; 32] {
        match self {
            Self::Precomposed(v) => v.fingerprint(),
            Self::Figure(v) => v.fingerprint,
        }
    }
}
