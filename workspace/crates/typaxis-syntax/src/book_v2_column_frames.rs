//! Source-bound column templates. This plan grants no selected-page or paint
//! authority and cannot be passed to the existing single-frame layout API.
use super::*;
use typaxis_core::{Length, PositiveLength, Rect};
use typaxis_document_package::WireColumnLayout;

/// Cumulative accepted caller and constructor reservations, including a failed
/// prefix. Rejected reservations do not change these counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BookV2ColumnFramePlanObservation {
    records: u64,
    spool: u64,
}
impl BookV2ColumnFramePlanObservation {
    pub fn record_charge(self) -> u64 {
        self.records
    }
    pub fn spool_charge(self) -> u64 {
        self.spool
    }
}
pub(super) struct PlanBudget<'o> {
    pub(super) records: u64,
    pub(super) spool: u64,
    maximum_records: u64,
    maximum_spool: u64,
    observation: Option<&'o mut BookV2ColumnFramePlanObservation>,
}
impl<'o> PlanBudget<'o> {
    pub(super) fn new(
        records: u64,
        spool: u64,
        maximum_records: u64,
        maximum_spool: u64,
        observation: Option<&'o mut BookV2ColumnFramePlanObservation>,
    ) -> Self {
        Self {
            records,
            spool,
            maximum_records,
            maximum_spool,
            observation,
        }
    }
    pub(super) fn take_records(&mut self, count: u64) -> Result<(), BookV2PageMasterError> {
        self.records = self
            .records
            .checked_add(count)
            .filter(|n| *n <= self.maximum_records)
            .ok_or(BookV2PageMasterError::RecordLimit)?;
        Ok(())
    }
    pub(super) fn take_spool(&mut self, count: u64) -> Result<(), BookV2PageMasterError> {
        self.spool = self
            .spool
            .checked_add(count)
            .filter(|n| *n <= self.maximum_spool)
            .ok_or(BookV2PageMasterError::SpoolLimit)?;
        Ok(())
    }
}
impl Drop for PlanBudget<'_> {
    fn drop(&mut self) {
        if let Some(observation) = &mut self.observation {
            observation.records = self.records;
            observation.spool = self.spool;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ColumnPartition {
    layout: WireColumnLayout,
    base: PositiveLength,
    residual: Length,
}
impl ColumnPartition {
    pub(super) fn new(body: Rect, layout: WireColumnLayout) -> Result<Self, BookV2PageMasterError> {
        use BookV2PageMasterError as E;
        if layout.count < 2 || layout.gap < 0 || Length::from_raw(layout.gap).is_none() {
            return Err(E::Geometry);
        }
        let count = i64::from(layout.count);
        let available = (count - 1)
            .checked_mul(layout.gap)
            .and_then(|gaps| body.width().get().raw().checked_sub(gaps))
            .filter(|n| *n >= count)
            .ok_or(E::Geometry)?;
        let base = Length::from_raw(available / count)
            .and_then(PositiveLength::new)
            .ok_or(E::Geometry)?;
        let residual = Length::from_raw(available % count).ok_or(E::Geometry)?;
        let partition = Self {
            layout,
            base,
            residual,
        };
        // Endpoints prove closure without a count-sized allocation or scan.
        // Every intermediate x lies between these monotonically increasing x's.
        let last = partition.column(body, layout.count - 1)?;
        if last.x().checked_add(last.width().get())
            != Some(
                body.x()
                    .checked_add(body.width().get())
                    .ok_or(E::Geometry)?,
            )
        {
            return Err(E::Geometry);
        }
        Ok(partition)
    }
    fn column(self, body: Rect, index: u16) -> Result<Rect, BookV2PageMasterError> {
        use BookV2PageMasterError as E;
        if index >= self.layout.count {
            return Err(E::Identity);
        }
        let x = self
            .base
            .get()
            .raw()
            .checked_add(self.layout.gap)
            .and_then(|stride| stride.checked_mul(i64::from(index)))
            .and_then(|offset| body.x().raw().checked_add(offset))
            .and_then(Length::from_raw)
            .ok_or(E::Geometry)?;
        let width = if index + 1 == self.layout.count {
            self.base
                .get()
                .checked_add(self.residual)
                .and_then(PositiveLength::new)
                .ok_or(E::Geometry)?
        } else {
            self.base
        };
        Ok(Rect::new(x, body.y(), width, body.height()))
    }
}

/// The authored physical body and its left-to-right templates. Neither this
/// object nor a returned rectangle proves that source content fits a column.
#[derive(Clone, Copy)]
pub struct BookV2ColumnPageFrames<'a> {
    source: &'a StyledBookV2Body,
    page: u32,
    name: Option<usize>,
    frames: BookV2PageFrames,
}
impl<'a> BookV2ColumnPageFrames<'a> {
    pub fn source(self) -> &'a StyledBookV2Body {
        self.source
    }
    pub fn page_index(self) -> u32 {
        self.page
    }
    pub fn named_page_index(self) -> Option<usize> {
        self.name
    }
    pub fn body(self) -> Rect {
        self.frames.body
    }
    /// The authored full-page note region remains separate from body columns.
    /// Joint reservation by a physical-page consumer is still required.
    pub fn footnote(self) -> Option<Rect> {
        self.frames.footnote
    }
    pub fn column_layout(self) -> Option<WireColumnLayout> {
        self.frames.columns.map(|c| c.layout)
    }
    pub fn column_count(self) -> u16 {
        self.column_layout().map_or(1, |c| c.count)
    }
    pub fn column(self, index: u16) -> Result<Rect, BookV2PageMasterError> {
        match self.frames.columns {
            Some(partition) => partition.column(self.frames.body, index),
            None if index == 0 => Ok(self.frames.body),
            None => Err(BookV2PageMasterError::Identity),
        }
    }
}

/// A distinct source plan prevents old single-body consumers from silently
/// laying out columns as full-width pages. Original names and region geometry
/// are retained; balance is an authored request, not a computed receipt.
///
/// ```compile_fail
/// use typaxis_syntax::book_v2::{BookV2ColumnFramePlan, BookV2PageFramePlan};
/// fn single_body<'p, 'a>(plan: &'p BookV2ColumnFramePlan<'a>) -> &'p BookV2PageFramePlan<'a> {
///     plan
/// }
/// ```
pub struct BookV2ColumnFramePlan<'a> {
    inner: BookV2PageFramePlan<'a>,
}
impl<'a> BookV2ColumnFramePlan<'a> {
    pub fn source(&self) -> &'a StyledBookV2Body {
        self.inner.source()
    }
    pub fn verify(&self, source: &StyledBookV2Body) -> Result<(), BookV2PageMasterError> {
        if std::ptr::eq(self.source(), source) {
            Ok(())
        } else {
            Err(BookV2PageMasterError::Identity)
        }
    }
    /// Maximum actual column width and body height, for initial measurement.
    /// It is not the geometry of any necessarily selected physical column.
    pub fn measurement_body(&self) -> Rect {
        self.inner.measurement_body()
    }
    pub fn measurement_footnote(&self) -> Option<Rect> {
        self.inner.measurement_footnote()
    }
    pub fn requires_width_reflow(&self) -> bool {
        self.inner.requires_width_reflow()
    }
    pub fn minimum_body_height(&self) -> Length {
        self.inner.minimum_body_height()
    }
    pub fn minimum_footnote_height(&self) -> Option<Length> {
        self.inner.minimum_footnote_height()
    }
    pub fn has_source_names(&self) -> bool {
        self.inner.has_source_names()
    }
    pub fn record_charge(&self) -> u64 {
        self.inner.record_charge()
    }
    pub fn spool_charge(&self) -> u64 {
        self.inner.spool_charge()
    }
    pub fn name(&self, index: usize) -> Option<&str> {
        self.inner.name(index)
    }
    pub fn source_name_index(&self, owner: typaxis_core::NodeId) -> Option<usize> {
        self.inner.source_name_index(owner)
    }
    pub fn page(&self, page: u32) -> Result<BookV2ColumnPageFrames<'a>, BookV2PageMasterError> {
        self.named_page(page, None)
    }
    pub fn named_page(
        &self,
        page: u32,
        name: Option<usize>,
    ) -> Result<BookV2ColumnPageFrames<'a>, BookV2PageMasterError> {
        Ok(BookV2ColumnPageFrames {
            source: self.source(),
            page,
            name,
            frames: self.inner.named_page(page, name)?,
        })
    }
}

/// Prepare actual column widths for every reachable first/parity/name class.
/// Logical column templates, source names and source scans share caller bounds.
/// Running regions and full-page footnote frames are retained for their future
/// consumers; existing single-frame/PDF APIs keep their unsupported guards.
pub fn prepare_book_v2_column_frame_plan<'a>(
    flow: &super::super::PreparedBookV2TextFlow<'a>,
    work: &mut u64,
    maximum: u64,
    prior_records: u64,
    prior_spool: u64,
) -> Result<BookV2ColumnFramePlan<'a>, BookV2PageMasterError> {
    prepare_book_v2_column_frame_plan_counted(
        flow,
        work,
        maximum,
        prior_records,
        prior_spool,
        &mut BookV2ColumnFramePlanObservation::default(),
    )
}

/// Report accepted template/name/source reservations on every return path.
/// Work remains in the supplied shared counter on both success and failure.
pub fn prepare_book_v2_column_frame_plan_counted<'a>(
    flow: &super::super::PreparedBookV2TextFlow<'a>,
    work: &mut u64,
    maximum: u64,
    prior_records: u64,
    prior_spool: u64,
    observation: &mut BookV2ColumnFramePlanObservation,
) -> Result<BookV2ColumnFramePlan<'a>, BookV2PageMasterError> {
    use BookV2PageMasterError as E;
    let mut inner = prepare_flow_frames(
        flow,
        work,
        maximum,
        prior_records,
        prior_spool,
        true,
        true,
        true,
        Some(observation),
    )?;
    let first = inner.pages[0].ok_or(E::PageLimit)?.body;
    let (mut minimum, mut width, mut height) = (None::<Length>, Length::ZERO, first.height());
    let mut note_width = None;
    let mut reflow = false;
    for frames in inner.all_frames() {
        *work = work
            .checked_add(1)
            .filter(|n| *n <= maximum)
            .ok_or(E::WorkLimit)?;
        let base = frames
            .columns
            .map_or(frames.body.width().get(), |c| c.base.get());
        let widest = frames.columns.map_or(Ok(base), |c| {
            c.base.get().checked_add(c.residual).ok_or(E::Geometry)
        })?;
        minimum = Some(minimum.map_or(base, |old| old.min(base)));
        width = width.max(widest);
        if height.get() < frames.body.height().get() {
            height = frames.body.height();
        }
        if let Some(region) = frames.footnote {
            reflow |= note_width.is_some_and(|old| old != region.width());
            note_width = Some(region.width());
        }
    }
    inner.body = Rect::new(
        first.x(),
        first.y(),
        PositiveLength::new(width).ok_or(E::Geometry)?,
        height,
    );
    inner.width_reflow = reflow || minimum != Some(width);
    Ok(BookV2ColumnFramePlan { inner })
}
