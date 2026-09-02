use std::collections::BTreeSet;
use typaxis_core::{
    push_jcs_string, sha256, ImageResourceId, Length, M4EffectiveResourceLimits, NodeId,
    PositiveLength, Rect, SourceSpan,
};
use typaxis_document::{
    ImageMediaDeclaration, ImageMediaType, StagingM4Block, StagingM4FigurePlacement,
};
use typaxis_resource_admission::{AdmittedImageMediaKind, AdmittedResourceLedger};
use typaxis_syntax::{
    StagingM4PageGeometry, StagingSafeVectorProfileView, ValidatedStagingSemanticPackage,
};

pub const STAGING_SAFE_VECTOR_SELECTED_LAYOUT_ALGORITHM: &str =
    "typaxis.safe-vector-selected-layout/1";
const FIXED_ONE: i64 = 65_536;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPlacement {
    occurrence: u32,
    owner: NodeId,
    image_id: ImageResourceId,
    placement: StagingM4FigurePlacement,
    alternative: String,
    source_span: SourceSpan,
    page_index: u32,
    frame_index: u32,
    bounds: Rect,
    scale: i32,
    admitted_sha256: [u8; 32],
    ir_fingerprint: [u8; 32],
    fingerprint: [u8; 32],
}

impl StagingSafeVectorPlacement {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn placement(&self) -> StagingM4FigurePlacement {
        self.placement
    }
    pub fn alternative(&self) -> &str {
        &self.alternative
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn frame_index(&self) -> u32 {
        self.frame_index
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
    pub const fn scale_raw(&self) -> i32 {
        self.scale
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub const fn ir_fingerprint(&self) -> [u8; 32] {
        self.ir_fingerprint
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorSelectedLayoutReceipt {
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    page_geometry_fingerprint: [u8; 32],
    placement_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorSelectedLayoutReceipt {
    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.package_fingerprint
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub const fn admitted_fingerprint(&self) -> [u8; 32] {
        self.admitted_fingerprint
    }
    pub const fn page_geometry_fingerprint(&self) -> [u8; 32] {
        self.page_geometry_fingerprint
    }
    pub const fn placement_count(&self) -> u32 {
        self.placement_count
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorSelectedLayout {
    placements: Vec<StagingSafeVectorPlacement>,
    page_geometry: StagingM4PageGeometry,
    receipt: StagingSafeVectorSelectedLayoutReceipt,
}

impl StagingSafeVectorSelectedLayout {
    pub fn placements(&self) -> &[StagingSafeVectorPlacement] {
        &self.placements
    }
    pub const fn receipt(&self) -> &StagingSafeVectorSelectedLayoutReceipt {
        &self.receipt
    }
    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        &self.page_geometry
    }

    pub fn verify_downstream(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingSafeVectorProfileView,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSafeVectorLayoutError> {
        let expected_profile = StagingSafeVectorProfileView::new(package, limits)
            .map_err(|_| StagingSafeVectorLayoutError::ReceiptMismatch)?;
        let canonical = encode_layout(
            package.semantic_fingerprint(),
            profile.profile_fingerprint(),
            limits.fingerprint(),
            self.receipt.admitted_fingerprint,
            &self.placements,
            &self.page_geometry,
        );
        if *profile != expected_profile
            || self.page_geometry != *profile.page_geometry()
            || self.receipt.package_fingerprint != package.semantic_fingerprint()
            || self.receipt.profile_fingerprint != profile.profile_fingerprint()
            || self.receipt.limits_fingerprint != limits.fingerprint()
            || self.receipt.page_geometry_fingerprint != self.page_geometry.fingerprint()
            || usize::try_from(self.receipt.placement_count) != Ok(self.placements.len())
            || self.receipt.canonical_jcs != canonical
            || self.receipt.fingerprint != sha256(canonical.as_bytes())
            || self.placements.len() as u64 > limits.base().get().max_fragments
            || !placements_match_package(&self.placements, package, profile)
            || self
                .placements
                .iter()
                .enumerate()
                .any(|(index, placement)| {
                    usize::try_from(placement.occurrence) != Ok(index)
                        || placement.scale <= 0
                        || i64::from(placement.scale) > FIXED_ONE
                        || sha256(encode_placement(placement).as_bytes()) != placement.fingerprint
                })
            || !placements_are_closed(&self.placements, &self.page_geometry, limits)
        {
            return Err(StagingSafeVectorLayoutError::ReceiptMismatch);
        }
        Ok(())
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingSafeVectorProfileView,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
    ) -> Result<(), StagingSafeVectorLayoutError> {
        self.verify_downstream(package, profile, limits)?;
        if self.receipt.admitted_fingerprint != admitted.fingerprint().bytes() {
            return Err(StagingSafeVectorLayoutError::ReceiptMismatch);
        }
        for (index, placement) in self.placements.iter().enumerate() {
            let image = admitted.image(placement.image_id).ok_or(
                StagingSafeVectorLayoutError::MissingAdmittedVector(placement.image_id),
            )?;
            let vector = image
                .safe_vector()
                .ok_or(StagingSafeVectorLayoutError::WrongMedia(placement.image_id))?;
            if usize::try_from(placement.occurrence) != Ok(index)
                || image.media_kind() != AdmittedImageMediaKind::SafeVector
                || image.content_hash() != placement.admitted_sha256
                || vector.fingerprint() != placement.ir_fingerprint
                || image.m4_limits_fingerprint() != Some(limits.fingerprint())
                || image.m4_profile_fingerprint() != Some(profile.profile_fingerprint())
                || scale_to_fit(
                    vector.intrinsic_width().get().raw(),
                    self.page_geometry.body().width().get().raw(),
                )? != Some(placement.scale)
                || scaled_dimension(vector.intrinsic_width().get().raw(), placement.scale)?
                    != placement.bounds.width().get().raw()
                || scaled_dimension(vector.intrinsic_height().get().raw(), placement.scale)?
                    != placement.bounds.height().get().raw()
                || sha256(encode_placement(placement).as_bytes()) != placement.fingerprint
            {
                return Err(StagingSafeVectorLayoutError::ReceiptMismatch);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorLayoutError {
    ProfileMismatch,
    MissingAdmittedVector(ImageResourceId),
    WrongMedia(ImageResourceId),
    IntrinsicGeometry(ImageResourceId),
    PlacementLimit,
    PageLimit,
    Oversize(NodeId),
    ArithmeticOverflow,
    ReceiptMismatch,
    AllocationFailure,
    PrecomposedVectorStaging(NodeId),
}

impl std::fmt::Display for StagingSafeVectorLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileMismatch => formatter.write_str("I9190: SafeVector profile mismatch"),
            Self::MissingAdmittedVector(id) => {
                write!(formatter, "R7100: missing admitted vector {}", id.get())
            }
            Self::WrongMedia(id) => {
                write!(formatter, "R7100: image {} is not SafeVector", id.get())
            }
            Self::IntrinsicGeometry(id) => write!(
                formatter,
                "L5100: invalid vector intrinsic geometry {}",
                id.get()
            ),
            Self::PlacementLimit => {
                formatter.write_str("L5110: SafeVector placement limit exceeded")
            }
            Self::PageLimit => formatter.write_str("L5100: SafeVector page limit exceeded"),
            Self::Oversize(owner) => write!(
                formatter,
                "L5100: vector Figure {} exceeds an empty frame",
                owner.get()
            ),
            Self::ArithmeticOverflow => {
                formatter.write_str("L5100: SafeVector layout arithmetic overflow")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: SafeVector layout receipt mismatch")
            }
            Self::AllocationFailure => {
                formatter.write_str("L5100: SafeVector layout allocation failed")
            }
            Self::PrecomposedVectorStaging(owner) => write!(
                formatter,
                "P1102: precomposed vector at node {} requires SafeVector /2 layout",
                owner.get()
            ),
        }
    }
}

impl std::error::Error for StagingSafeVectorLayoutError {}

pub fn layout_staging_safe_vectors(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSafeVectorProfileView,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
) -> Result<StagingSafeVectorSelectedLayout, StagingSafeVectorLayoutError> {
    if *profile
        != StagingSafeVectorProfileView::new(package, limits)
            .map_err(|_| StagingSafeVectorLayoutError::ProfileMismatch)?
        || !admitted.matches_declarations(
            typaxis_resource_admission::staging_declared_base_catalog(package.resources())
                .map_err(|_| StagingSafeVectorLayoutError::ProfileMismatch)?
                .resource_catalog(),
        )
        || !admitted_matches_profile(package, profile, limits, admitted)
    {
        return Err(StagingSafeVectorLayoutError::ProfileMismatch);
    }
    let mut figures = Vec::new();
    let vector_ids: BTreeSet<_> = profile.vector_resource_ids().iter().copied().collect();
    collect_figures(&package.document().blocks, &vector_ids, &mut figures)?;
    for footnote in &package.document().footnotes {
        collect_figures(&footnote.blocks, &vector_ids, &mut figures)?;
    }
    if figures.len() as u64 > limits.base().get().max_fragments {
        return Err(StagingSafeVectorLayoutError::PlacementLimit);
    }
    let mut placements = Vec::new();
    placements
        .try_reserve_exact(figures.len())
        .map_err(|_| StagingSafeVectorLayoutError::AllocationFailure)?;
    let mut page_index = 0u32;
    let mut cursor = 0i64;
    let page_geometry = profile.page_geometry().clone();
    let body = page_geometry.body();
    for (index, figure) in figures.into_iter().enumerate() {
        let image = admitted.image(figure.image_id).ok_or(
            StagingSafeVectorLayoutError::MissingAdmittedVector(figure.image_id),
        )?;
        let ir = image
            .safe_vector()
            .ok_or(StagingSafeVectorLayoutError::WrongMedia(figure.image_id))?;
        if image.m4_limits_fingerprint() != Some(limits.fingerprint())
            || image.m4_profile_fingerprint() != Some(profile.profile_fingerprint())
        {
            return Err(StagingSafeVectorLayoutError::ProfileMismatch);
        }
        let scale = scale_to_fit(ir.intrinsic_width().get().raw(), body.width().get().raw())?
            .ok_or(StagingSafeVectorLayoutError::Oversize(figure.owner))?;
        let width_raw = scaled_dimension(ir.intrinsic_width().get().raw(), scale)?;
        let height_raw = scaled_dimension(ir.intrinsic_height().get().raw(), scale)?;
        let width = PositiveLength::new(
            Length::from_raw(width_raw).ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
        )
        .ok_or(StagingSafeVectorLayoutError::IntrinsicGeometry(
            figure.image_id,
        ))?;
        let height = PositiveLength::new(
            Length::from_raw(height_raw).ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
        )
        .ok_or(StagingSafeVectorLayoutError::IntrinsicGeometry(
            figure.image_id,
        ))?;
        if height.get().raw() > body.height().get().raw() {
            return Err(StagingSafeVectorLayoutError::Oversize(figure.owner));
        }
        if cursor
            .checked_add(height.get().raw())
            .map_or(true, |end| end > body.height().get().raw())
        {
            page_index = page_index
                .checked_add(1)
                .ok_or(StagingSafeVectorLayoutError::PageLimit)?;
            cursor = 0;
        }
        if page_index >= limits.base().get().max_pages {
            return Err(StagingSafeVectorLayoutError::PageLimit);
        }
        let x_raw = match figure.placement {
            StagingM4FigurePlacement::Block => body.x().raw(),
            StagingM4FigurePlacement::Float => body
                .x()
                .raw()
                .checked_add(
                    body.width()
                        .get()
                        .raw()
                        .checked_sub(width.get().raw())
                        .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
                )
                .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
        };
        let y_raw = body
            .y()
            .raw()
            .checked_add(cursor)
            .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?;
        let mut placement = StagingSafeVectorPlacement {
            occurrence: u32::try_from(index)
                .map_err(|_| StagingSafeVectorLayoutError::PlacementLimit)?,
            owner: figure.owner,
            image_id: figure.image_id,
            placement: figure.placement,
            alternative: figure.alternative.to_owned(),
            source_span: figure.span,
            page_index,
            frame_index: 0,
            bounds: Rect::new(
                Length::from_raw(x_raw).ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
                Length::from_raw(y_raw).ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
                width,
                height,
            ),
            scale,
            admitted_sha256: image.content_hash(),
            ir_fingerprint: ir.fingerprint(),
            fingerprint: [0; 32],
        };
        placement.fingerprint = sha256(encode_placement(&placement).as_bytes());
        cursor = cursor
            .checked_add(height.get().raw())
            .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?;
        placements.push(placement);
    }
    let canonical_jcs = encode_layout(
        package.semantic_fingerprint(),
        profile.profile_fingerprint(),
        limits.fingerprint(),
        admitted.fingerprint().bytes(),
        &placements,
        &page_geometry,
    );
    let selected = StagingSafeVectorSelectedLayout {
        receipt: StagingSafeVectorSelectedLayoutReceipt {
            package_fingerprint: package.semantic_fingerprint(),
            profile_fingerprint: profile.profile_fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            admitted_fingerprint: admitted.fingerprint().bytes(),
            page_geometry_fingerprint: page_geometry.fingerprint(),
            placement_count: u32::try_from(placements.len())
                .map_err(|_| StagingSafeVectorLayoutError::PlacementLimit)?,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
        placements,
        page_geometry,
    };
    selected.verify(package, profile, limits, admitted)?;
    Ok(selected)
}

struct FigureRef<'a> {
    owner: NodeId,
    image_id: ImageResourceId,
    placement: StagingM4FigurePlacement,
    alternative: &'a str,
    span: SourceSpan,
}

fn collect_figures<'a>(
    blocks: &'a [StagingM4Block],
    vector_ids: &BTreeSet<ImageResourceId>,
    output: &mut Vec<FigureRef<'a>>,
) -> Result<(), StagingSafeVectorLayoutError> {
    for block in blocks {
        match block {
            StagingM4Block::Figure {
                common,
                image_id,
                placement,
                alternative,
                caption,
                ..
            } => {
                if vector_ids.contains(image_id) {
                    output.push(FigureRef {
                        owner: common.node_id,
                        image_id: *image_id,
                        placement: *placement,
                        alternative,
                        span: common.span,
                    });
                }
                collect_figures(caption, vector_ids, output)?;
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    collect_figures(&item.blocks, vector_ids, output)?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_figures(&cell.blocks, vector_ids, output)?;
                }
            }
            StagingM4Block::SemanticContainer { blocks, .. } => {
                collect_figures(blocks, vector_ids, output)?;
            }
            StagingM4Block::VectorFigure { common, .. }
            | StagingM4Block::MathVectorBlock { common, .. } => {
                return Err(StagingSafeVectorLayoutError::PrecomposedVectorStaging(
                    common.node_id,
                ));
            }
            StagingM4Block::Paragraph { inline_vectors, .. }
            | StagingM4Block::Heading { inline_vectors, .. } => {
                if let Some(vector) = inline_vectors.first() {
                    return Err(StagingSafeVectorLayoutError::PrecomposedVectorStaging(
                        vector.node_id,
                    ));
                }
            }
            StagingM4Block::PageBreak { .. } | StagingM4Block::DisplayMath { .. } => {}
        }
    }
    Ok(())
}

fn admitted_matches_profile(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSafeVectorProfileView,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
) -> bool {
    package.resources().images.iter().all(|declaration| {
        let Some(image) = admitted.image(declaration.image_id) else {
            return false;
        };
        match declaration.media {
            ImageMediaDeclaration::Declared(ImageMediaType::Png) => {
                image.media_kind() == AdmittedImageMediaKind::Png
                    && image.safe_vector().is_none()
                    && image.m4_limits_fingerprint().is_none()
                    && image.m4_profile_fingerprint().is_none()
            }
            ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1) => {
                image.media_kind() == AdmittedImageMediaKind::SafeVector
                    && image.safe_vector().is_some()
                    && image.m4_limits_fingerprint() == Some(limits.fingerprint())
                    && image.m4_profile_fingerprint() == Some(profile.profile_fingerprint())
            }
            ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2) => false,
            ImageMediaDeclaration::LegacyUnspecified => false,
        }
    })
}

fn placements_match_package(
    placements: &[StagingSafeVectorPlacement],
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSafeVectorProfileView,
) -> bool {
    fn visit(
        blocks: &[StagingM4Block],
        vector_ids: &[ImageResourceId],
        figure_owners: &[NodeId],
        placements: &[StagingSafeVectorPlacement],
        next: &mut usize,
    ) -> bool {
        for block in blocks {
            match block {
                StagingM4Block::Figure {
                    common,
                    image_id,
                    placement,
                    alternative,
                    caption,
                    ..
                } => {
                    if vector_ids.binary_search(image_id).is_ok() {
                        let Some(expected) = placements.get(*next) else {
                            return false;
                        };
                        if figure_owners.get(*next) != Some(&common.node_id)
                            || usize::try_from(expected.occurrence) != Ok(*next)
                            || expected.owner != common.node_id
                            || expected.image_id != *image_id
                            || expected.placement != *placement
                            || expected.alternative != *alternative
                            || expected.source_span != common.span
                        {
                            return false;
                        }
                        *next += 1;
                    }
                    if !visit(caption, vector_ids, figure_owners, placements, next) {
                        return false;
                    }
                }
                StagingM4Block::List { items, .. } => {
                    for item in items {
                        if !visit(&item.blocks, vector_ids, figure_owners, placements, next) {
                            return false;
                        }
                    }
                }
                StagingM4Block::Table { head, body, .. } => {
                    for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                        if !visit(&cell.blocks, vector_ids, figure_owners, placements, next) {
                            return false;
                        }
                    }
                }
                StagingM4Block::SemanticContainer { blocks, .. }
                    if !visit(blocks, vector_ids, figure_owners, placements, next) =>
                {
                    return false;
                }
                StagingM4Block::SemanticContainer { .. } => {}
                StagingM4Block::VectorFigure { .. } | StagingM4Block::MathVectorBlock { .. } => {
                    return false
                }
                StagingM4Block::Paragraph { inline_vectors, .. }
                | StagingM4Block::Heading { inline_vectors, .. }
                    if !inline_vectors.is_empty() =>
                {
                    return false;
                }
                StagingM4Block::Paragraph { .. }
                | StagingM4Block::Heading { .. }
                | StagingM4Block::PageBreak { .. }
                | StagingM4Block::DisplayMath { .. } => {}
            }
        }
        true
    }

    let mut next = 0usize;
    if !visit(
        &package.document().blocks,
        profile.vector_resource_ids(),
        profile.figure_owners(),
        placements,
        &mut next,
    ) {
        return false;
    }
    for footnote in &package.document().footnotes {
        if !visit(
            &footnote.blocks,
            profile.vector_resource_ids(),
            profile.figure_owners(),
            placements,
            &mut next,
        ) {
            return false;
        }
    }
    next == placements.len() && next == profile.figure_owners().len()
}

fn round_ratio(numerator: i128, denominator: i128) -> Result<i64, StagingSafeVectorLayoutError> {
    if denominator <= 0 {
        return Err(StagingSafeVectorLayoutError::ArithmeticOverflow);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let twice = remainder
        .unsigned_abs()
        .checked_mul(2)
        .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?;
    let denominator_unsigned = denominator as u128;
    let rounded =
        if twice < denominator_unsigned || (twice == denominator_unsigned && quotient % 2 == 0) {
            quotient
        } else {
            quotient
                .checked_add(if remainder >= 0 { 1 } else { -1 })
                .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?
        };
    i64::try_from(rounded).map_err(|_| StagingSafeVectorLayoutError::ArithmeticOverflow)
}

fn scale_to_fit(
    intrinsic_width: i64,
    available_width: i64,
) -> Result<Option<i32>, StagingSafeVectorLayoutError> {
    if intrinsic_width <= 0 || available_width <= 0 {
        return Err(StagingSafeVectorLayoutError::ArithmeticOverflow);
    }
    if intrinsic_width <= available_width {
        return Ok(Some(FIXED_ONE as i32));
    }
    let candidate = i128::from(available_width)
        .checked_mul(i128::from(FIXED_ONE))
        .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?
        / i128::from(intrinsic_width);
    let candidate =
        i32::try_from(candidate).map_err(|_| StagingSafeVectorLayoutError::ArithmeticOverflow)?;
    Ok((candidate > 0).then_some(candidate))
}

fn scaled_dimension(intrinsic: i64, scale: i32) -> Result<i64, StagingSafeVectorLayoutError> {
    round_ratio(
        i128::from(intrinsic)
            .checked_mul(i128::from(scale))
            .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
        i128::from(FIXED_ONE),
    )
}

fn placements_are_closed(
    placements: &[StagingSafeVectorPlacement],
    page_geometry: &StagingM4PageGeometry,
    limits: &M4EffectiveResourceLimits,
) -> bool {
    let body = page_geometry.body();
    let mut expected_page = 0u32;
    let mut cursor = 0i64;
    for placement in placements {
        let height = placement.bounds.height().get().raw();
        if height > body.height().get().raw() {
            return false;
        }
        if cursor
            .checked_add(height)
            .map_or(true, |end| end > body.height().get().raw())
        {
            let Some(next_page) = expected_page.checked_add(1) else {
                return false;
            };
            expected_page = next_page;
            cursor = 0;
        }
        let expected_x = match placement.placement {
            StagingM4FigurePlacement::Block => body.x().raw(),
            StagingM4FigurePlacement::Float => {
                let Some(remaining) = body
                    .width()
                    .get()
                    .raw()
                    .checked_sub(placement.bounds.width().get().raw())
                else {
                    return false;
                };
                let Some(x) = body.x().raw().checked_add(remaining) else {
                    return false;
                };
                x
            }
        };
        let Some(expected_y) = body.y().raw().checked_add(cursor) else {
            return false;
        };
        if placement.page_index != expected_page
            || placement.frame_index != 0
            || placement.bounds.x().raw() != expected_x
            || placement.bounds.y().raw() != expected_y
            || placement.bounds.width().get().raw() > body.width().get().raw()
            || placement.page_index >= limits.base().get().max_pages
        {
            return false;
        }
        let Some(next_cursor) = cursor.checked_add(height) else {
            return false;
        };
        cursor = next_cursor;
    }
    true
}

fn encode_layout(
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    placements: &[StagingSafeVectorPlacement],
    page_geometry: &StagingM4PageGeometry,
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted_fingerprint);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_SAFE_VECTOR_SELECTED_LAYOUT_ALGORITHM);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits_fingerprint);
    output.push_str(",\"package_fingerprint\":");
    push_hash(&mut output, package_fingerprint);
    output.push_str(",\"page_geometry\":");
    output.push_str(page_geometry.canonical_jcs());
    output.push_str(",\"placements\":[");
    for (index, placement) in placements.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&encode_placement(placement));
    }
    output.push_str("],\"profile_fingerprint\":");
    push_hash(&mut output, profile_fingerprint);
    output.push('}');
    output
}

fn encode_placement(value: &StagingSafeVectorPlacement) -> String {
    let mut output = String::from("{\"admitted_sha256\":");
    push_hash(&mut output, value.admitted_sha256);
    output.push_str(",\"alternative_sha256\":");
    push_hash(&mut output, sha256(value.alternative.as_bytes()));
    output.push_str(",\"bounds\":{");
    output.push_str("\"height\":");
    output.push_str(&value.bounds.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&value.bounds.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&value.bounds.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&value.bounds.y().raw().to_string());
    output.push_str("},\"frame_index\":");
    output.push_str(&value.frame_index.to_string());
    output.push_str(",\"image_id\":");
    output.push_str(&value.image_id.get().to_string());
    output.push_str(",\"ir_fingerprint\":");
    push_hash(&mut output, value.ir_fingerprint);
    output.push_str(",\"occurrence\":");
    output.push_str(&value.occurrence.to_string());
    output.push_str(",\"owner\":");
    output.push_str(&value.owner.get().to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&value.page_index.to_string());
    output.push_str(",\"placement\":");
    push_jcs_string(&mut output, value.placement.as_str());
    output.push_str(",\"scale\":");
    output.push_str(&value.scale.to_string());
    output.push_str(",\"source_span\":{");
    output.push_str("\"end_byte\":");
    output.push_str(&value.source_span.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&value.source_span.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&value.source_span.start_byte().get().to_string());
    output.push_str("}}");
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

#[cfg(any(test, feature = "staging-fixtures"))]
pub struct StagingSafeVectorLayoutFixture {
    pub package: ValidatedStagingSemanticPackage,
    pub profile: StagingSafeVectorProfileView,
    pub limits: M4EffectiveResourceLimits,
    pub admitted: AdmittedResourceLedger,
    pub media: typaxis_resource_admission::StagingDeclaredMediaLedger,
    pub selected: StagingSafeVectorSelectedLayout,
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_safe_vector_layout_fixture(
) -> Result<StagingSafeVectorLayoutFixture, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::PathBuf;
    use typaxis_core::{
        ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions, HostAdmissionContext, HostPath,
        M4ResourceLimits, PdfStreamCompression, ResourceLimits, ValidatedResourceLimits,
        DEFAULT_ALLOWED_URI_SCHEMES,
    };
    use typaxis_resource_admission::{
        close_staging_declared_media, staging_declared_base_catalog, AdmittedResourceResolver,
        HostResourceAdmissionSession,
    };
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::StagingSemanticPackageParser;

    let job = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../samples/machine-package/staging/production-book-1/vector-media/job");
    let package_path = job.join("document-package.json");
    let base_limits = ValidatedResourceLimits::new(ResourceLimits::default())?;
    let limits = M4EffectiveResourceLimits::new(base_limits.clone(), M4ResourceLimits::default())?;
    let decoded = StagingSemanticDocumentPackageDecoder::new().decode(
        &fs::read(&package_path)?,
        &DocumentPackageDecodePolicy::new(&base_limits),
    )?;
    let package = StagingSemanticPackageParser::new().parse(decoded, &base_limits)?;
    let profile = StagingSafeVectorProfileView::new(&package, &limits)?;
    let base = staging_declared_base_catalog(package.resources())?;
    let config = EffectiveConfig::new(
        true,
        PdfStreamCompression::None,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0")
            .expect("registered fixture data versions"),
        ResourceLimits::default(),
    )?;
    let context = HostAdmissionContext::new(
        HostPath::new(package_path)?,
        HostPath::new(job)?,
        None,
        Vec::new(),
    );
    let session = HostResourceAdmissionSession::new(&context, &config, &base)?;
    let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
        &base,
        &limits,
        profile.profile_fingerprint(),
        session.roots(),
    )?;
    for declaration in &package.resources().images {
        let pending = resolver.read_image(session.open_image(declaration.image_id)?)?;
        resolver.parse_and_bind_declared_image(pending)?;
    }
    let admitted = resolver.finish()?;
    let media = close_staging_declared_media(&admitted, package.resources())?;
    let selected = layout_staging_safe_vectors(&package, &profile, &limits, &admitted)?;
    Ok(StagingSafeVectorLayoutFixture {
        package,
        profile,
        limits,
        admitted,
        media,
        selected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_layout_preserves_intrinsic_ratio_and_closes_selected_figure() {
        let fixture = staging_safe_vector_layout_fixture().unwrap();
        assert_eq!(fixture.selected.placements().len(), 1);
        let placement = &fixture.selected.placements()[0];
        assert_eq!(placement.image_id(), ImageResourceId::new(0));
        assert_eq!(placement.bounds().width().get().raw(), 80 * 65_536);
        assert_eq!(placement.bounds().height().get().raw(), 40 * 65_536);
        assert_eq!(placement.bounds().x().raw(), 100 * 65_536);
        assert_eq!(placement.bounds().y().raw(), 100 * 65_536);
        assert_eq!(
            fixture.selected.page_geometry().page_width().get().raw(),
            1_000 * 65_536
        );
        assert_eq!(
            fixture.selected.page_geometry().page_height().get().raw(),
            800 * 65_536
        );
        fixture
            .selected
            .verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
            )
            .unwrap();
        assert!(fixture
            .selected
            .receipt()
            .canonical_jcs()
            .contains("\"admitted_fingerprint\""));
        assert!(placements_match_package(
            fixture.selected.placements(),
            &fixture.package,
            &fixture.profile,
        ));
        let mut wrong_resource = fixture.selected.placements().to_vec();
        wrong_resource[0].image_id = ImageResourceId::new(1);
        assert!(!placements_match_package(
            &wrong_resource,
            &fixture.package,
            &fixture.profile,
        ));

        let odd_ratio_scale = scale_to_fit(3 * FIXED_ONE, 2 * FIXED_ONE).unwrap().unwrap();
        assert_eq!(odd_ratio_scale, 43_690);
        assert_eq!(
            scaled_dimension(3 * FIXED_ONE, odd_ratio_scale).unwrap(),
            131_070
        );
        assert_eq!(
            scaled_dimension(2 * FIXED_ONE, odd_ratio_scale).unwrap(),
            87_380
        );
    }
}
