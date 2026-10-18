use crate::hb::common::{HB_FEATURE_GLOBAL_END, HB_FEATURE_GLOBAL_START};
use crate::scratch::{Error, PlanScratch, Site};
use crate::Feature;
use alloc::vec::Vec;
use core::cmp::Ordering;

use super::layout::*;
use crate::hb::{hb_font_t, hb_mask_t, hb_tag_t};

/// HB: hb_aat_map_t
///
/// See <https://github.com/harfbuzz/harfbuzz/blob/2c22a65f0cb99544c36580b9703a43b5dc97a9e1/src/hb-aat-map.hh#L33>
#[doc(alias = "hb_aat_map_t")]
#[derive(Default)]
pub struct AatMap {
    pub chain_flags: Vec<Vec<RangeFlags>>,
}

/// HB: hb_aat_map_t::range_flags_t
///
/// See <https://github.com/harfbuzz/harfbuzz/blob/2c22a65f0cb99544c36580b9703a43b5dc97a9e1/src/hb-aat-map.hh#L38>
#[derive(Copy, Clone)]
pub struct RangeFlags {
    pub flags: hb_mask_t,
    pub cluster_first: u32,
    pub cluster_last: u32, // end - 1
}

/// HB: hb_aat_map_builder_t
///
/// See <https://github.com/harfbuzz/harfbuzz/blob/2c22a65f0cb99544c36580b9703a43b5dc97a9e1/src/hb-aat-map.hh#L49>
#[doc(alias = "hb_aat_map_builder_t")]
pub struct AatMapBuilder {
    pub current_features: Vec<FeatureInfo>,
    pub features: Vec<FeatureRange>,
    pub range_first: usize,
    pub range_last: usize,
}

impl Default for AatMapBuilder {
    fn default() -> Self {
        Self {
            range_first: HB_FEATURE_GLOBAL_START as usize,
            range_last: HB_FEATURE_GLOBAL_END as usize,
            current_features: Vec::default(),
            features: Vec::default(),
        }
    }
}

impl AatMapBuilder {
    pub fn add_feature(&mut self, face: &hb_font_t, feature: &Feature) -> Option<()> {
        self.add_feature_with_scratch(face, feature, &mut PlanScratch::new(None))
            .expect("unobserved AAT feature construction cannot refuse scratch")
    }

    pub(crate) fn add_feature_with_scratch(
        &mut self,
        face: &hb_font_t,
        feature: &Feature,
        scratch: &mut PlanScratch<'_>,
    ) -> Result<Option<()>, Error> {
        scratch.status()?;
        let Some(feat) = face.aat_tables.feat.as_ref() else {
            return Ok(None);
        };
        scratch.work(1)?;

        if feature.tag == hb_tag_t::new(b"aalt") {
            let exposes_feature = feat
                .find(FEATURE_TYPE_CHARACTER_ALTERNATIVES as u16)
                .is_some_and(|f| f.n_settings() != 0);

            if !exposes_feature {
                return Ok(Some(()));
            }

            scratch.push(
                &mut self.features,
                FeatureRange {
                    start: feature.start,
                    end: feature.end,
                    info: FeatureInfo {
                        kind: FEATURE_TYPE_CHARACTER_ALTERNATIVES as u16,
                        setting: u16::try_from(feature.value).unwrap(),
                        is_exclusive: true,
                    },
                },
                Site::AatFeatures,
            )?;
        }

        let result = feature_mappings.binary_search_by(|map| {
            if scratch.work(1).is_err() {
                return Ordering::Equal;
            }
            map.ot_feature_tag.cmp(&feature.tag)
        });
        scratch.status()?;
        let Ok(idx) = result else { return Ok(None) };
        let mapping = &feature_mappings[idx];

        scratch.work(1)?;
        let mut feature_name = feat.find(mapping.aat_feature_type as u16);

        match feature_name {
            Some(feature) if feature.n_settings() != 0 => {}
            _ => {
                // Special case: Chain::compile_flags will fall back to the deprecated version of
                // small-caps if necessary, so we need to check for that possibility.
                // https://github.com/harfbuzz/harfbuzz/issues/2307
                if mapping.aat_feature_type == FEATURE_TYPE_LOWER_CASE
                    && mapping.selector_to_enable == FEATURE_SELECTOR_LOWER_CASE_SMALL_CAPS
                {
                    scratch.work(1)?;
                    feature_name = feat.find(FEATURE_TYPE_LETTER_CASE as u16);
                }
            }
        }

        match feature_name {
            Some(feature_name) if feature_name.n_settings() != 0 => {
                let setting = if feature.value != 0 {
                    mapping.selector_to_enable
                } else {
                    mapping.selector_to_disable
                } as u16;

                scratch.push(
                    &mut self.features,
                    FeatureRange {
                        start: feature.start,
                        end: feature.end,
                        info: FeatureInfo {
                            kind: mapping.aat_feature_type as u16,
                            setting,
                            is_exclusive: feature_name.is_exclusive(),
                        },
                    },
                    Site::AatFeatures,
                )?;
            }
            _ => {}
        }

        Ok(Some(()))
    }

    pub fn compile(&mut self, face: &hb_font_t, m: &mut AatMap) {
        self.compile_with_scratch(face, m, &mut PlanScratch::new(None))
            .expect("unobserved AAT map construction cannot refuse scratch");
    }

    pub(crate) fn compile_with_scratch(
        &mut self,
        face: &hb_font_t,
        m: &mut AatMap,
        scratch: &mut PlanScratch<'_>,
    ) -> Result<(), Error> {
        scratch.status()?;
        // Compute active features per range, and compile each.
        let mut feature_events = Vec::new();
        for feature in &self.features {
            scratch.work(1)?;
            if feature.start == feature.end {
                continue;
            }

            scratch.push(
                &mut feature_events,
                FeatureEvent {
                    index: feature.start as usize,
                    start: true,
                    feature: feature.info,
                },
                Site::AatFeatureEvents,
            )?;

            scratch.push(
                &mut feature_events,
                FeatureEvent {
                    index: feature.end as usize,
                    start: false,
                    feature: feature.info,
                },
                Site::AatFeatureEvents,
            )?;
        }

        scratch.stable_sort(&mut feature_events)?;

        // Add a strategic final event.
        scratch.push(
            &mut feature_events,
            FeatureEvent {
                index: u32::MAX as usize,
                start: false,
                feature: FeatureInfo::default(),
            },
            Site::AatFeatureEvents,
        )?;

        // Scan events and save features for each range.
        let mut active_features = Vec::new();
        let mut last_index = 0;

        for event in &feature_events {
            scratch.work(1)?;
            if event.index != last_index {
                // Save a snapshot of active features and the range.
                // Sort features and merge duplicates.
                if scratch.observed() {
                    scratch.reserve(
                        &mut self.current_features,
                        active_features.len(),
                        Site::AatCurrentFeatures,
                    )?;
                    scratch.work(active_features.len())?;
                }
                self.current_features.clone_from(&active_features);
                self.range_first = last_index;
                self.range_last = event.index.wrapping_sub(1);

                if !self.current_features.is_empty() {
                    scratch.stable_sort(&mut self.current_features)?;
                    let mut j = 0;
                    for i in 1..self.current_features.len() {
                        scratch.work(1)?;
                        // Nonexclusive feature selectors come in even/odd pairs to turn a setting on/off
                        // respectively, so we mask out the low-order bit when checking for "duplicates"
                        // (selectors referring to the same feature setting) here.
                        let non_exclusive = !self.current_features[i].is_exclusive
                            && (self.current_features[i].setting & !1)
                                != (self.current_features[j].setting & !1);

                        if self.current_features[i].kind != self.current_features[j].kind
                            || non_exclusive
                        {
                            scratch.work(1)?;
                            j += 1;
                            self.current_features[j] = self.current_features[i];
                        }
                    }
                    self.current_features.truncate(j + 1);
                }

                if scratch.observed() {
                    super::layout_morx_table::compile_flags_with_scratch(face, self, m, scratch)?;
                } else {
                    super::layout_morx_table::compile_flags(face, self, m);
                }
                last_index = event.index;
            }

            if event.start {
                scratch.push(&mut active_features, event.feature, Site::AatActiveFeatures)?;
            } else {
                let mut found = None;
                for (index, feature) in active_features.iter().enumerate() {
                    scratch.work(1)?;
                    if *feature == event.feature {
                        found = Some(index);
                        break;
                    }
                }
                if let Some(index) = found {
                    scratch.work(active_features.len() - index)?;
                    active_features.remove(index);
                }
            }
        }

        for chain_flags in &mut m.chain_flags {
            scratch.work(1)?;
            if let Some(last) = chain_flags.last_mut() {
                last.cluster_last = HB_FEATURE_GLOBAL_END;
            }
        }
        Ok(())
    }
}

/// HB: hb_aat_map_builder_t::feature_info_t
///
/// See <https://github.com/harfbuzz/harfbuzz/blob/2c22a65f0cb99544c36580b9703a43b5dc97a9e1/src/hb-aat-map.hh#L63>
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct FeatureInfo {
    pub kind: u16,
    pub setting: u16,
    pub is_exclusive: bool,
}

impl Ord for FeatureInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for FeatureInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.kind != other.kind {
            Some(self.kind.cmp(&other.kind))
        } else if !self.is_exclusive && (self.setting & !1) != (other.setting & !1) {
            Some(self.setting.cmp(&other.setting))
        } else {
            Some(Ordering::Equal)
        }
    }
}

/// HB: hb_aat_map_builder_t::feature_range_t
///
/// See <https://github.com/harfbuzz/harfbuzz/blob/2c22a65f0cb99544c36580b9703a43b5dc97a9e1/src/hb-aat-map.hh#L88>
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct FeatureRange {
    pub info: FeatureInfo,
    pub start: u32,
    pub end: u32,
}

/// HB: hb_aat_map_builder_t::feature_event_t
///
/// See <https://github.com/harfbuzz/harfbuzz/blob/2c22a65f0cb99544c36580b9703a43b5dc97a9e1/src/hb-aat-map.hh#L96>
#[derive(Copy, Clone, Eq, PartialEq)]
struct FeatureEvent {
    pub index: usize,
    pub start: bool,
    pub feature: FeatureInfo,
}

impl Ord for FeatureEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for FeatureEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.index != other.index {
            Some(self.index.cmp(&other.index))
        } else if self.start != other.start {
            Some(self.start.cmp(&other.start))
        } else {
            Some(Ordering::Equal)
        }
    }
}

#[cfg(test)]
#[path = "scratch_tests.rs"]
mod scratch_tests;
