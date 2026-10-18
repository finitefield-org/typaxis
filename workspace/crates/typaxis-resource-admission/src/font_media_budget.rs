//! Permits for the resource layer's outline attestation and error-only walk.
use super::*;
use typaxis_font::{
    CffAdmissionBudgetFailureV2, CffAdmissionBudgetV2, CffSfntBudgetFailureV2, CffSfntStageV2,
    CffSfntWorkPointV2,
};

#[derive(Debug)]
enum WalkError<E> {
    Native(ResourceAdmissionError),
    Budget(E),
}
impl<E> From<ResourceAdmissionError> for WalkError<E> {
    fn from(error: ResourceAdmissionError) -> Self {
        Self::Native(error)
    }
}
pub(super) fn point(stage: CffSfntStageV2, offset: usize) -> CffSfntWorkPointV2 {
    CffSfntWorkPointV2 {
        stage,
        table_tag: None,
        file_offset: Some(offset as u64),
        table_offset: None,
    }
}
pub(super) fn attest_native(
    bytes: &[u8],
    face: u32,
) -> Result<AdmittedFontMediaKind, ResourceAdmissionError> {
    match attest_walk(bytes, face, &mut |_, _| {
        Ok::<_, std::convert::Infallible>(())
    }) {
        Ok(value) => Ok(value),
        Err(WalkError::Native(error)) => Err(error),
        Err(WalkError::Budget(error)) => match error {},
    }
}
pub(super) fn declared_native(
    bytes: &[u8],
    face: u32,
) -> Result<AdmittedFontMediaKind, ResourceAdmissionError> {
    attest_native(bytes, face).map_err(|_| {
        ResourceAdmissionError::FontContainerDetailed(font_diagnostic::diagnose_font_container(
            bytes, face,
        ))
    })
}
pub(super) fn declared_with_budget<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    face: u32,
    budget: &mut B,
) -> Result<
    Result<AdmittedFontMediaKind, ResourceAdmissionError>,
    CffAdmissionBudgetFailureV2<B::Error>,
> {
    if !B::OBSERVED || !B::OBSERVE_RESOURCE_MEDIA {
        return Ok(declared_native(bytes, face));
    }
    let mut work = |point, amount| {
        budget.work_sfnt(amount).map_err(|error| {
            CffAdmissionBudgetFailureV2::Sfnt(CffSfntBudgetFailureV2 { error, point })
        })
    };
    match attest_walk(bytes, face, &mut work) {
        Ok(value) => Ok(Ok(value)),
        Err(WalkError::Budget(error)) => Err(error),
        Err(WalkError::Native(_)) => {
            let diagnostic =
                font_diagnostic::diagnose_font_container_with_work(bytes, face, &mut work)?;
            Ok(Err(ResourceAdmissionError::FontContainerDetailed(
                diagnostic,
            )))
        }
    }
}

fn attest_walk<E>(
    bytes: &[u8],
    face_index: u32,
    work: &mut impl FnMut(typaxis_font::CffSfntWorkPointV2, u64) -> Result<(), E>,
) -> Result<AdmittedFontMediaKind, WalkError<E>> {
    work(
        point(typaxis_font::CffSfntStageV2::DeclaredMediaHeader, 0),
        64,
    )
    .map_err(WalkError::Budget)?;
    let media_kind = attest_font_media_kind(bytes)?;
    let directory_offset = match media_kind {
        AdmittedFontMediaKind::SfntTrueTypeGlyf => {
            if face_index != 0 {
                return Err(ResourceAdmissionError::InvalidMetadata.into());
            }
            0usize
        }
        AdmittedFontMediaKind::TtcTrueTypeGlyf => {
            let count = read_be_u32(bytes, 8)?;
            if face_index >= count {
                return Err(ResourceAdmissionError::InvalidMetadata.into());
            }
            let offset_position = 12usize
                .checked_add(
                    usize::try_from(face_index)
                        .map_err(|_| ResourceAdmissionError::InvalidMetadata)?
                        .checked_mul(4)
                        .ok_or(ResourceAdmissionError::InvalidMetadata)?,
                )
                .ok_or(ResourceAdmissionError::InvalidMetadata)?;
            usize::try_from(read_be_u32(bytes, offset_position)?)
                .map_err(|_| ResourceAdmissionError::InvalidMetadata)?
        }
        AdmittedFontMediaKind::SfntCff1 => {
            if face_index != 0 {
                return Err(ResourceAdmissionError::InvalidMetadata.into());
            }
            0usize
        }
    };
    let signature_end = directory_offset
        .checked_add(4)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let observed_signature = bytes.get(directory_offset..signature_end);
    let signature_matches = match media_kind {
        AdmittedFontMediaKind::SfntTrueTypeGlyf | AdmittedFontMediaKind::TtcTrueTypeGlyf => {
            observed_signature == Some(&0x0001_0000u32.to_be_bytes())
        }
        AdmittedFontMediaKind::SfntCff1 => observed_signature == Some(b"OTTO"),
    };
    if !signature_matches {
        return Err(ResourceAdmissionError::InvalidMetadata.into());
    }
    let table_count = usize::from(read_be_u16(
        bytes,
        directory_offset
            .checked_add(4)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?,
    )?);
    let directory_start = directory_offset
        .checked_add(12)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let mut has_glyf = false;
    let mut has_loca = false;
    let mut has_cff1 = false;
    let mut has_cff2 = false;
    for index in 0..table_count {
        let record = directory_start
            .checked_add(
                index
                    .checked_mul(16)
                    .ok_or(ResourceAdmissionError::InvalidMetadata)?,
            )
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        work(
            point(typaxis_font::CffSfntStageV2::DeclaredMediaRecord, record),
            64,
        )
        .map_err(WalkError::Budget)?;
        let tag_end = record
            .checked_add(4)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        match bytes
            .get(record..tag_end)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?
        {
            b"glyf" => has_glyf = true,
            b"loca" => has_loca = true,
            b"CFF " => has_cff1 = true,
            b"CFF2" => has_cff2 = true,
            _ => {}
        }
    }
    let valid_outline_tables = match media_kind {
        AdmittedFontMediaKind::SfntTrueTypeGlyf | AdmittedFontMediaKind::TtcTrueTypeGlyf => {
            has_glyf && has_loca && !has_cff1 && !has_cff2
        }
        AdmittedFontMediaKind::SfntCff1 => has_cff1 && !has_cff2 && !has_glyf && !has_loca,
    };
    if !valid_outline_tables {
        return Err(ResourceAdmissionError::InvalidMetadata.into());
    }
    Ok(media_kind)
}

#[cfg(test)]
#[path = "font_media_budget_tests.rs"]
mod tests;
