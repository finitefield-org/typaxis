use std::collections::BTreeSet;

use typaxis_core::{
    materialized_pagination_state_fingerprint_from_jcs, push_jcs_string, sha256, ImageResourceId,
    LayoutStateFingerprint, Length, M4EffectiveResourceLimits, NodeId, PositiveLength, Rect,
    SourceSpan,
};
use typaxis_layout_contract::{LayoutEpoch, LayoutEpochError};
use typaxis_resource_admission::{
    close_staging_declared_media, AdmittedImageMediaKind, AdmittedResourceLedger, JpegColorKind,
    JpegSampling,
};
use typaxis_syntax::{
    StagingJpegProfileView, StagingM4PageGeometry, ValidatedStagingSemanticPackage,
};

pub const STAGING_JPEG_SELECTED_LAYOUT_ALGORITHM: &str = "typaxis.jpeg-selected-layout/1";
pub const STAGING_JPEG_SIZING_ALGORITHM: &str = "typaxis.jpeg-body-width-pixel-aspect-ties-even/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingJpegLayoutError {
    ProfileMismatch,
    AdmissionMismatch,
    MissingImage(ImageResourceId),
    ExtraImage(ImageResourceId),
    WrongMedia(ImageResourceId),
    FigureTooTall(NodeId),
    FragmentLimit,
    PageLimit,
    ArithmeticOverflow,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingJpegLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for StagingJpegLayoutError {}

impl From<LayoutEpochError> for StagingJpegLayoutError {
    fn from(_: LayoutEpochError) -> Self {
        Self::ReceiptMismatch
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingJpegPlacement {
    occurrence: u32,
    owner: NodeId,
    image_id: ImageResourceId,
    source_span: SourceSpan,
    alternative: String,
    page_break_before: bool,
    page_index: u32,
    rect: Rect,
    source_sha256: [u8; 32],
    normalized_sha256: [u8; 32],
    pixel_sha256: [u8; 32],
    decoded_byte_length: u64,
    peak_workspace_bytes: u64,
    color_kind: JpegColorKind,
    sampling: JpegSampling,
    fingerprint: [u8; 32],
}

impl StagingJpegPlacement {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub fn alternative(&self) -> &str {
        &self.alternative
    }
    pub const fn page_break_before(&self) -> bool {
        self.page_break_before
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn rect(&self) -> Rect {
        self.rect
    }
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub const fn normalized_sha256(&self) -> [u8; 32] {
        self.normalized_sha256
    }
    pub const fn pixel_sha256(&self) -> [u8; 32] {
        self.pixel_sha256
    }
    pub const fn decoded_byte_length(&self) -> u64 {
        self.decoded_byte_length
    }
    pub const fn peak_workspace_bytes(&self) -> u64 {
        self.peak_workspace_bytes
    }
    pub const fn color_kind(&self) -> JpegColorKind {
        self.color_kind
    }
    pub const fn sampling(&self) -> JpegSampling {
        self.sampling
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingJpegSelectedLayout {
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    declared_media_fingerprint: [u8; 32],
    epoch: LayoutEpoch,
    state_fingerprint: LayoutStateFingerprint,
    page_geometry: StagingM4PageGeometry,
    page_count: u32,
    placements: Vec<StagingJpegPlacement>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingJpegSelectedLayout {
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
    pub const fn declared_media_fingerprint(&self) -> [u8; 32] {
        self.declared_media_fingerprint
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn state_fingerprint(&self) -> LayoutStateFingerprint {
        self.state_fingerprint
    }
    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        &self.page_geometry
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub fn placements(&self) -> &[StagingJpegPlacement] {
        &self.placements
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingJpegProfileView,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
    ) -> Result<(), StagingJpegLayoutError> {
        profile
            .authorizes(package, limits)
            .map_err(|_| StagingJpegLayoutError::ProfileMismatch)?;
        let media = close_staging_declared_media(admitted, package.resources())
            .map_err(|_| StagingJpegLayoutError::AdmissionMismatch)?;
        let epoch =
            LayoutEpoch::from_staging_jpeg_inputs(package, profile, limits, admitted.token())?;
        let canonical = encode_layout(
            package.semantic_fingerprint(),
            profile.profile_fingerprint(),
            limits.fingerprint(),
            admitted.fingerprint().bytes(),
            media.fingerprint(),
            self.page_count,
            &self.placements,
            &self.page_geometry,
        );
        if self.package_fingerprint != package.semantic_fingerprint()
            || self.profile_fingerprint != profile.profile_fingerprint()
            || self.limits_fingerprint != limits.fingerprint()
            || self.admitted_fingerprint != admitted.fingerprint().bytes()
            || self.declared_media_fingerprint != media.fingerprint()
            || self.epoch != epoch
            || self.page_geometry != *profile.page_geometry()
            || !placements_are_closed(&self.placements, &self.page_geometry, limits)
            || self.page_count
                != self
                    .placements
                    .last()
                    .and_then(|placement| placement.page_index.checked_add(1))
                    .unwrap_or(0)
            || self.canonical_jcs != canonical
            || self.fingerprint != sha256(canonical.as_bytes())
            || self.state_fingerprint
                != materialized_pagination_state_fingerprint_from_jcs(&canonical)
        {
            return Err(StagingJpegLayoutError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn layout_staging_jpeg_figures(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingJpegProfileView,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
) -> Result<StagingJpegSelectedLayout, StagingJpegLayoutError> {
    profile
        .authorizes(package, limits)
        .map_err(|_| StagingJpegLayoutError::ProfileMismatch)?;
    let media = close_staging_declared_media(admitted, package.resources())
        .map_err(|_| StagingJpegLayoutError::AdmissionMismatch)?;
    let expected: BTreeSet<_> = profile.jpeg_resource_ids().iter().copied().collect();
    for image_id in &expected {
        if admitted.image(*image_id).is_none() {
            return Err(StagingJpegLayoutError::MissingImage(*image_id));
        }
    }
    if let Some(extra) = admitted
        .images()
        .iter()
        .find(|image| !expected.contains(&image.image_id()))
    {
        return Err(StagingJpegLayoutError::ExtraImage(extra.image_id()));
    }

    let geometry = profile.page_geometry().clone();
    let body = geometry.body();
    let width = body.width();
    let mut page_index = 0u32;
    let mut cursor = 0i64;
    let placement_capacity = checked_placement_capacity(profile.figures().len(), limits)?;
    let mut placements = Vec::new();
    placements
        .try_reserve_exact(placement_capacity)
        .map_err(|_| StagingJpegLayoutError::AllocationFailure)?;
    for (index, figure) in profile.figures().iter().enumerate() {
        let image = admitted
            .image(figure.image_id())
            .ok_or(StagingJpegLayoutError::MissingImage(figure.image_id()))?;
        let jpeg = image
            .jpeg_attestation()
            .ok_or(StagingJpegLayoutError::WrongMedia(figure.image_id()))?;
        if image.media_kind() != AdmittedImageMediaKind::JpegBaseline
            || jpeg.source_sha256() != image.content_hash()
            || jpeg.profile_fingerprint() != profile.profile_fingerprint()
            || jpeg.limits_fingerprint() != limits.fingerprint()
        {
            return Err(StagingJpegLayoutError::WrongMedia(figure.image_id()));
        }
        let height_raw = round_ratio(
            i128::from(width.get().raw())
                .checked_mul(i128::from(jpeg.height().get()))
                .ok_or(StagingJpegLayoutError::ArithmeticOverflow)?,
            i128::from(jpeg.width().get()),
        )?;
        let height = Length::from_raw(height_raw)
            .and_then(PositiveLength::new)
            .ok_or(StagingJpegLayoutError::ArithmeticOverflow)?;
        if height.get().raw() > body.height().get().raw() {
            return Err(StagingJpegLayoutError::FigureTooTall(figure.owner()));
        }
        let would_overflow = cursor
            .checked_add(height.get().raw())
            .map_or(true, |end| end > body.height().get().raw());
        if figure.page_break_before() || would_overflow {
            page_index = page_index
                .checked_add(1)
                .ok_or(StagingJpegLayoutError::PageLimit)?;
            cursor = 0;
        }
        if page_index >= limits.base().get().max_pages {
            return Err(StagingJpegLayoutError::PageLimit);
        }
        let y = body
            .y()
            .checked_add(
                Length::from_raw(cursor).ok_or(StagingJpegLayoutError::ArithmeticOverflow)?,
            )
            .ok_or(StagingJpegLayoutError::ArithmeticOverflow)?;
        let rect = Rect::new(body.x(), y, width, height);
        let mut placement = StagingJpegPlacement {
            occurrence: u32::try_from(index)
                .map_err(|_| StagingJpegLayoutError::AllocationFailure)?,
            owner: figure.owner(),
            image_id: figure.image_id(),
            source_span: figure.source_span(),
            alternative: figure.alternative().to_owned(),
            page_break_before: figure.page_break_before(),
            page_index,
            rect,
            source_sha256: jpeg.source_sha256(),
            normalized_sha256: jpeg.normalized_sha256(),
            pixel_sha256: jpeg.pixel_sha256(),
            decoded_byte_length: jpeg.decoded_byte_length(),
            peak_workspace_bytes: jpeg.peak_workspace_bytes(),
            color_kind: jpeg.color_kind(),
            sampling: jpeg.sampling(),
            fingerprint: [0; 32],
        };
        placement.fingerprint = sha256(encode_placement(&placement).as_bytes());
        placements.push(placement);
        cursor = cursor
            .checked_add(height.get().raw())
            .ok_or(StagingJpegLayoutError::ArithmeticOverflow)?;
    }
    let page_count = placements
        .last()
        .and_then(|placement| placement.page_index.checked_add(1))
        .ok_or(StagingJpegLayoutError::ReceiptMismatch)?;
    let epoch = LayoutEpoch::from_staging_jpeg_inputs(package, profile, limits, admitted.token())?;
    let canonical_jcs = encode_layout(
        package.semantic_fingerprint(),
        profile.profile_fingerprint(),
        limits.fingerprint(),
        admitted.fingerprint().bytes(),
        media.fingerprint(),
        page_count,
        &placements,
        &geometry,
    );
    let selected = StagingJpegSelectedLayout {
        package_fingerprint: package.semantic_fingerprint(),
        profile_fingerprint: profile.profile_fingerprint(),
        limits_fingerprint: limits.fingerprint(),
        admitted_fingerprint: admitted.fingerprint().bytes(),
        declared_media_fingerprint: media.fingerprint(),
        epoch,
        state_fingerprint: materialized_pagination_state_fingerprint_from_jcs(&canonical_jcs),
        page_geometry: geometry,
        page_count,
        placements,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    selected.verify(package, profile, limits, admitted)?;
    Ok(selected)
}

fn round_ratio(numerator: i128, denominator: i128) -> Result<i64, StagingJpegLayoutError> {
    if numerator <= 0 || denominator <= 0 {
        return Err(StagingJpegLayoutError::ArithmeticOverflow);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let twice = remainder
        .checked_mul(2)
        .ok_or(StagingJpegLayoutError::ArithmeticOverflow)?;
    let rounded = if twice < denominator || (twice == denominator && quotient % 2 == 0) {
        quotient
    } else {
        quotient
            .checked_add(1)
            .ok_or(StagingJpegLayoutError::ArithmeticOverflow)?
    };
    i64::try_from(rounded).map_err(|_| StagingJpegLayoutError::ArithmeticOverflow)
}

fn placements_are_closed(
    placements: &[StagingJpegPlacement],
    geometry: &StagingM4PageGeometry,
    limits: &M4EffectiveResourceLimits,
) -> bool {
    if placements.is_empty()
        || u64::try_from(placements.len())
            .map_or(true, |count| count > limits.base().get().max_fragments)
    {
        return false;
    }
    let body = geometry.body();
    let mut page = 0u32;
    let mut cursor = 0i64;
    for (index, placement) in placements.iter().enumerate() {
        let height = placement.rect.height().get().raw();
        let overflow = cursor
            .checked_add(height)
            .map_or(true, |end| end > body.height().get().raw());
        if placement.page_break_before || overflow {
            let Some(next) = page.checked_add(1) else {
                return false;
            };
            page = next;
            cursor = 0;
        }
        let Some(y) = body.y().raw().checked_add(cursor) else {
            return false;
        };
        if usize::try_from(placement.occurrence) != Ok(index)
            || placement.page_index != page
            || page >= limits.base().get().max_pages
            || placement.rect.x() != body.x()
            || placement.rect.y().raw() != y
            || placement.rect.width() != body.width()
            || height <= 0
            || height > body.height().get().raw()
            || placement.fingerprint != sha256(encode_placement(placement).as_bytes())
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

fn checked_placement_capacity(
    count: usize,
    limits: &M4EffectiveResourceLimits,
) -> Result<usize, StagingJpegLayoutError> {
    if count == 0
        || u64::try_from(count).map_or(true, |count| count > limits.base().get().max_fragments)
        || u32::try_from(count).is_err()
    {
        return Err(StagingJpegLayoutError::FragmentLimit);
    }
    Ok(count)
}

#[allow(clippy::too_many_arguments)]
fn encode_layout(
    package: [u8; 32],
    profile: [u8; 32],
    limits: [u8; 32],
    admitted: [u8; 32],
    declared_media: [u8; 32],
    page_count: u32,
    placements: &[StagingJpegPlacement],
    geometry: &StagingM4PageGeometry,
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_JPEG_SELECTED_LAYOUT_ALGORITHM);
    output.push_str(",\"declared_media_fingerprint\":");
    push_hash(&mut output, declared_media);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits);
    output.push_str(",\"package_fingerprint\":");
    push_hash(&mut output, package);
    output.push_str(",\"page_count\":");
    output.push_str(&page_count.to_string());
    output.push_str(",\"page_geometry\":");
    output.push_str(geometry.canonical_jcs());
    output.push_str(",\"placements\":[");
    for (index, placement) in placements.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&encode_placement(placement));
    }
    output.push_str("],\"profile_fingerprint\":");
    push_hash(&mut output, profile);
    output.push_str(",\"sizing_algorithm\":");
    push_jcs_string(&mut output, STAGING_JPEG_SIZING_ALGORITHM);
    output.push_str(",\"state_algorithm\":\"typaxis.pagination-fingerprint/1\"}");
    output
}

fn encode_placement(value: &StagingJpegPlacement) -> String {
    let mut output = String::from("{\"alternative_sha256\":");
    push_hash(&mut output, sha256(value.alternative.as_bytes()));
    output.push_str(",\"color_kind\":");
    push_jcs_string(&mut output, value.color_kind.as_str());
    output.push_str(",\"decoded_byte_length\":");
    output.push_str(&value.decoded_byte_length.to_string());
    output.push_str(",\"image_id\":");
    output.push_str(&value.image_id.get().to_string());
    output.push_str(",\"node_id\":");
    output.push_str(&value.owner.get().to_string());
    output.push_str(",\"normalized_sha256\":");
    push_hash(&mut output, value.normalized_sha256);
    output.push_str(",\"occurrence\":");
    output.push_str(&value.occurrence.to_string());
    output.push_str(",\"page_break_before\":");
    output.push_str(if value.page_break_before {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"page_index\":");
    output.push_str(&value.page_index.to_string());
    output.push_str(",\"peak_workspace_bytes\":");
    output.push_str(&value.peak_workspace_bytes.to_string());
    output.push_str(",\"pixel_sha256\":");
    push_hash(&mut output, value.pixel_sha256);
    output.push_str(",\"rect\":[");
    output.push_str(&value.rect.x().raw().to_string());
    output.push(',');
    output.push_str(&value.rect.y().raw().to_string());
    output.push(',');
    output.push_str(&value.rect.width().get().raw().to_string());
    output.push(',');
    output.push_str(&value.rect.height().get().raw().to_string());
    output.push_str("],\"sampling\":");
    push_jcs_string(&mut output, value.sampling.as_str());
    output.push_str(",\"source_sha256\":");
    push_hash(&mut output, value.source_sha256);
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

#[cfg(test)]
mod tests {
    use typaxis_core::{M4ResourceLimits, ResourceLimits, ValidatedResourceLimits};

    use super::*;

    #[test]
    fn jpeg_placement_limit_accepts_exact_and_rejects_max_plus_one() {
        let base = ResourceLimits {
            max_fragments: 3,
            ..ResourceLimits::default()
        };
        let limits = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(base).unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap();
        assert_eq!(checked_placement_capacity(3, &limits), Ok(3));
        assert_eq!(
            checked_placement_capacity(4, &limits),
            Err(StagingJpegLayoutError::FragmentLimit)
        );
    }
}
