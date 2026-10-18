use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::Any;

use super::ot_map::*;
use super::ot_shape::*;
use super::ot_shaper::*;
use super::{hb_font_t, hb_mask_t, Direction, Feature, Language, Script};

/// A reusable plan for shaping a text buffer.
pub struct hb_ot_shape_plan_t {
    pub(crate) direction: Direction,
    pub(crate) script: Option<Script>,
    pub(crate) shaper: &'static hb_ot_shaper_t,
    pub(crate) ot_map: hb_ot_map_t,
    pub(crate) data: Option<Box<dyn Any + Send + Sync>>,
    pub(crate) observed_data: Option<ObservedPlanData>,

    pub(crate) frac_mask: hb_mask_t,
    pub(crate) numr_mask: hb_mask_t,
    pub(crate) dnom_mask: hb_mask_t,
    pub(crate) rtlm_mask: hb_mask_t,
    pub(crate) kern_mask: hb_mask_t,

    pub(crate) requested_kerning: bool,
    pub(crate) has_frac: bool,
    pub(crate) has_vert: bool,
    pub(crate) has_gpos_mark: bool,
    pub(crate) zero_marks: bool,
    pub(crate) fallback_glyph_classes: bool,
    pub(crate) fallback_mark_positioning: bool,
    pub(crate) adjust_mark_positioning_when_zeroing: bool,

    pub(crate) apply_gpos: bool,
    pub(crate) apply_fallback_kern: bool,
    pub(crate) apply_kern: bool,
    pub(crate) apply_kerx: bool,
    pub(crate) apply_morx: bool,
    pub(crate) apply_trak: bool,
}

impl hb_ot_shape_plan_t {
    /// Returns a plan that can be used for shaping any buffer with the
    /// provided properties.
    pub fn new(
        face: &hb_font_t,
        direction: Direction,
        script: Option<Script>,
        language: Option<&Language>,
        user_features: &[Feature],
    ) -> Self {
        assert_ne!(direction, Direction::Invalid);
        let mut planner = hb_ot_shape_planner_t::new(face, direction, script, language);
        planner.collect_features(user_features);
        planner.compile()
    }

    /// Construct a plan after paying for its requested storage and instrumented work.
    ///
    /// The budget is borrowed only while constructing the plan. Refusals retain
    /// prior charges and publish no partial plan. Observed sorting uses no heap
    /// scratch; script data uses a fallibly reserved single-record vector.
    /// Subsequent shaping buffers and allocator overcapacity are outside this
    /// constructor's scope. The unobserved path retains the original constructor.
    pub fn try_new_with_scratch<B: crate::scratch::Budget>(
        face: &hb_font_t,
        direction: Direction,
        script: Option<Script>,
        language: Option<&Language>,
        user_features: &[Feature],
        budget: &mut B,
    ) -> Result<Self, crate::scratch::Error> {
        if !B::OBSERVED {
            return Ok(Self::new(face, direction, script, language, user_features));
        }
        assert_ne!(direction, Direction::Invalid);
        let mut observer = crate::scratch::ObservePlan(budget);
        let mut planner =
            hb_ot_shape_planner_t::try_new(face, direction, script, language, &mut observer)?;
        planner.collect_features(user_features);
        planner.try_compile()
    }

    pub(crate) fn data<T: 'static>(&self) -> &T {
        if let Some(data) = self.observed_data.as_ref() {
            data.as_any().downcast_ref().unwrap()
        } else {
            self.data.as_ref().unwrap().downcast_ref().unwrap()
        }
    }
}

pub(crate) enum ObservedPlanData {
    Arabic(Vec<super::ot_shaper_arabic::arabic_shape_plan_t>),
    Hangul(Vec<super::ot_shaper_hangul::hangul_shape_plan_t>),
    Indic(Vec<super::ot_shaper_indic::IndicShapePlan>),
    Khmer(Vec<super::ot_shaper_khmer::KhmerShapePlan>),
    Universal(Vec<super::ot_shaper_use::UniversalShapePlan>),
}

impl ObservedPlanData {
    fn as_any(&self) -> &dyn Any {
        match self {
            Self::Arabic(value) => &value[0],
            Self::Hangul(value) => &value[0],
            Self::Indic(value) => &value[0],
            Self::Khmer(value) => &value[0],
            Self::Universal(value) => &value[0],
        }
    }
}

#[cfg(test)]
#[path = "shape_plan_scratch_tests.rs"]
mod scratch_tests;

#[cfg(test)]
mod tests {
    use super::hb_ot_shape_plan_t;

    #[test]
    fn test_shape_plan_is_send_and_sync() {
        fn ensure_send_and_sync<T: Send + Sync>() {}
        ensure_send_and_sync::<hb_ot_shape_plan_t>();
    }
}
