use std::collections::BTreeSet;

use typaxis_core::{push_jcs_string, sha256, ImageResourceId, LayoutStateFingerprint, Rect};
use typaxis_layout::StagingJpegSelectedLayout;

use super::{
    DisplayCommand, DisplayDocument, DisplayPage, DisplaySourceLayout,
    StructurallyValidatedDisplayDocument, ValidatedDisplayDocument, ValidatedDisplayPageGeometry,
};

pub const STAGING_JPEG_DISPLAY_ALGORITHM: &str = "typaxis.jpeg-display/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingJpegDisplayError {
    MissingDrawImage(ImageResourceId),
    ExtraDrawImage(ImageResourceId),
    WrongDrawImage {
        expected: ImageResourceId,
        actual: ImageResourceId,
    },
    PageClosure,
    PlacementOutOfBounds,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingJpegDisplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for StagingJpegDisplayError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingJpegDisplayDraw {
    occurrence: u32,
    image_id: ImageResourceId,
    page_index: u32,
    rect: Rect,
    placement_fingerprint: [u8; 32],
}

impl StagingJpegDisplayDraw {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn rect(&self) -> Rect {
        self.rect
    }
    pub const fn placement_fingerprint(&self) -> [u8; 32] {
        self.placement_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingJpegDisplayFacts {
    selected_layout_fingerprint: [u8; 32],
    selected_state_fingerprint: LayoutStateFingerprint,
    page_count: u32,
    draws: Vec<StagingJpegDisplayDraw>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingJpegDisplayFacts {
    pub const fn selected_layout_fingerprint(&self) -> [u8; 32] {
        self.selected_layout_fingerprint
    }
    pub const fn selected_state_fingerprint(&self) -> LayoutStateFingerprint {
        self.selected_state_fingerprint
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub fn draws(&self) -> &[StagingJpegDisplayDraw] {
        &self.draws
    }
    pub fn used_image_ids(&self) -> BTreeSet<ImageResourceId> {
        self.draws
            .iter()
            .map(StagingJpegDisplayDraw::image_id)
            .collect()
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        selected: &StagingJpegSelectedLayout,
    ) -> Result<(), StagingJpegDisplayError> {
        if self.selected_layout_fingerprint != selected.fingerprint()
            || self.selected_state_fingerprint != selected.state_fingerprint()
            || self.page_count != selected.page_count()
            || self.draws.len() != selected.placements().len()
            || self
                .draws
                .iter()
                .zip(selected.placements())
                .any(|(draw, placement)| {
                    draw.occurrence != placement.occurrence()
                        || draw.image_id != placement.image_id()
                        || draw.page_index != placement.page_index()
                        || draw.rect != placement.rect()
                        || draw.placement_fingerprint != placement.fingerprint()
                })
        {
            return Err(StagingJpegDisplayError::ReceiptMismatch);
        }
        let canonical = encode_display(
            self.selected_layout_fingerprint,
            self.selected_state_fingerprint,
            self.page_count,
            &self.draws,
        );
        if self.canonical_jcs != canonical || self.fingerprint != sha256(canonical.as_bytes()) {
            return Err(StagingJpegDisplayError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct StagingJpegDisplay {
    trusted: ValidatedDisplayDocument,
    facts: StagingJpegDisplayFacts,
}

impl StagingJpegDisplay {
    pub const fn validated_document(&self) -> &ValidatedDisplayDocument {
        &self.trusted
    }
    pub const fn facts(&self) -> &StagingJpegDisplayFacts {
        &self.facts
    }
    pub fn canonical_jcs(&self) -> &str {
        self.facts.canonical_jcs()
    }
    pub fn verify(
        &self,
        selected: &StagingJpegSelectedLayout,
    ) -> Result<(), StagingJpegDisplayError> {
        self.facts.verify(selected)?;
        let document = self.trusted.document();
        if document.source_layout().layout_epoch() != selected.epoch()
            || document.source_layout().state_fingerprint() != selected.state_fingerprint()
            || document.pages.len() != usize::try_from(selected.page_count()).unwrap_or(usize::MAX)
            || self.trusted.selected_page_geometry().len() != document.pages.len()
            || !document.text_buffers.is_empty()
            || !document.font_instances.is_empty()
            || !document.destinations.is_empty()
            || document.pages.iter().any(|page| {
                !page.annotations.is_empty()
                    || page
                        .commands
                        .iter()
                        .any(|command| !matches!(command, DisplayCommand::DrawImage { .. }))
            })
        {
            return Err(StagingJpegDisplayError::ReceiptMismatch);
        }
        let observed: Vec<_> = document
            .pages
            .iter()
            .flat_map(|page| {
                page.commands
                    .iter()
                    .filter_map(move |command| match command {
                        DisplayCommand::DrawImage { image_id, rect } => {
                            Some((page.page_index, *image_id, *rect))
                        }
                        _ => None,
                    })
            })
            .collect();
        if observed.len() != self.facts.draws.len()
            || observed
                .iter()
                .zip(&self.facts.draws)
                .any(|((page, image, rect), draw)| {
                    *page != draw.page_index || *image != draw.image_id || *rect != draw.rect
                })
        {
            return Err(StagingJpegDisplayError::ReceiptMismatch);
        }
        Ok(())
    }
    pub fn into_parts(self) -> (ValidatedDisplayDocument, StagingJpegDisplayFacts) {
        (self.trusted, self.facts)
    }
}

pub fn build_staging_jpeg_display(
    selected: &StagingJpegSelectedLayout,
) -> Result<StagingJpegDisplay, StagingJpegDisplayError> {
    let ids: Vec<_> = selected
        .placements()
        .iter()
        .map(|placement| placement.image_id())
        .collect();
    build_staging_jpeg_display_with_ids(selected, &ids)
}

#[doc(hidden)]
pub fn build_staging_jpeg_display_with_ids(
    selected: &StagingJpegSelectedLayout,
    ids: &[ImageResourceId],
) -> Result<StagingJpegDisplay, StagingJpegDisplayError> {
    for (index, placement) in selected.placements().iter().enumerate() {
        let Some(actual) = ids.get(index).copied() else {
            return Err(StagingJpegDisplayError::MissingDrawImage(
                placement.image_id(),
            ));
        };
        if actual != placement.image_id() {
            return Err(StagingJpegDisplayError::WrongDrawImage {
                expected: placement.image_id(),
                actual,
            });
        }
    }
    if let Some(extra) = ids.get(selected.placements().len()).copied() {
        return Err(StagingJpegDisplayError::ExtraDrawImage(extra));
    }

    let geometry = selected.page_geometry();
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(
            usize::try_from(selected.page_count())
                .map_err(|_| StagingJpegDisplayError::AllocationFailure)?,
        )
        .map_err(|_| StagingJpegDisplayError::AllocationFailure)?;
    for page_index in 0..selected.page_count() {
        pages.push(DisplayPage {
            page_index,
            width: geometry.page_width(),
            height: geometry.page_height(),
            commands: Vec::new(),
            annotations: Vec::new(),
        });
    }
    let mut draws = Vec::new();
    draws
        .try_reserve_exact(selected.placements().len())
        .map_err(|_| StagingJpegDisplayError::AllocationFailure)?;
    for placement in selected.placements() {
        let page = pages
            .get_mut(
                usize::try_from(placement.page_index())
                    .map_err(|_| StagingJpegDisplayError::PageClosure)?,
            )
            .ok_or(StagingJpegDisplayError::PageClosure)?;
        if !rect_within_page(placement.rect(), page) {
            return Err(StagingJpegDisplayError::PlacementOutOfBounds);
        }
        page.commands.push(DisplayCommand::DrawImage {
            image_id: placement.image_id(),
            rect: placement.rect(),
        });
        draws.push(StagingJpegDisplayDraw {
            occurrence: placement.occurrence(),
            image_id: placement.image_id(),
            page_index: placement.page_index(),
            rect: placement.rect(),
            placement_fingerprint: placement.fingerprint(),
        });
    }
    let selected_page_geometry = pages
        .iter()
        .map(|page| ValidatedDisplayPageGeometry {
            page_index: page.page_index,
            master_id: geometry.master_id().clone(),
            width: page.width,
            height: page.height,
        })
        .collect();
    let trusted = ValidatedDisplayDocument {
        structural: StructurallyValidatedDisplayDocument {
            document: DisplayDocument {
                source_layout: DisplaySourceLayout {
                    layout_epoch: selected.epoch(),
                    state_fingerprint: selected.state_fingerprint(),
                },
                text_buffers: Vec::new(),
                font_instances: Vec::new(),
                destinations: Vec::new(),
                pages,
            },
            selected_page_geometry,
        },
    };
    let canonical_jcs = encode_display(
        selected.fingerprint(),
        selected.state_fingerprint(),
        selected.page_count(),
        &draws,
    );
    let display = StagingJpegDisplay {
        trusted,
        facts: StagingJpegDisplayFacts {
            selected_layout_fingerprint: selected.fingerprint(),
            selected_state_fingerprint: selected.state_fingerprint(),
            page_count: selected.page_count(),
            draws,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
    };
    display.verify(selected)?;
    Ok(display)
}

fn rect_within_page(rect: Rect, page: &DisplayPage) -> bool {
    rect.x().raw() >= 0
        && rect.y().raw() >= 0
        && rect
            .x()
            .raw()
            .checked_add(rect.width().get().raw())
            .is_some_and(|right| right <= page.width.get().raw())
        && rect
            .y()
            .raw()
            .checked_add(rect.height().get().raw())
            .is_some_and(|bottom| bottom <= page.height.get().raw())
}

fn encode_display(
    selected_layout: [u8; 32],
    selected_state: LayoutStateFingerprint,
    page_count: u32,
    draws: &[StagingJpegDisplayDraw],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_JPEG_DISPLAY_ALGORITHM);
    output.push_str(",\"draws\":[");
    for (index, draw) in draws.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"image_id\":");
        output.push_str(&draw.image_id.get().to_string());
        output.push_str(",\"occurrence\":");
        output.push_str(&draw.occurrence.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&draw.page_index.to_string());
        output.push_str(",\"placement_fingerprint\":");
        push_hash(&mut output, draw.placement_fingerprint);
        output.push_str(",\"rect\":[");
        output.push_str(&draw.rect.x().raw().to_string());
        output.push(',');
        output.push_str(&draw.rect.y().raw().to_string());
        output.push(',');
        output.push_str(&draw.rect.width().get().raw().to_string());
        output.push(',');
        output.push_str(&draw.rect.height().get().raw().to_string());
        output.push_str("]}");
    }
    output.push_str("],\"page_count\":");
    output.push_str(&page_count.to_string());
    output.push_str(",\"selected_layout_fingerprint\":");
    push_hash(&mut output, selected_layout);
    output.push_str(",\"selected_state_fingerprint\":");
    push_hash(&mut output, selected_state.bytes());
    output.push('}');
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
