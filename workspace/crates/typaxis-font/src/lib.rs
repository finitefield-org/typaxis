#![forbid(unsafe_code)]

use core::num::NonZeroU16;
use std::collections::BTreeMap;
use typaxis_core::{FontFaceId, FontInstanceId, OpenTypeTag};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OriginalGlyphId(u16);
impl OriginalGlyphId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SubsetGlyphId(u16);
impl SubsetGlyphId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cid(NonZeroU16);
impl Cid {
    pub const fn new(value: u16) -> Option<Self> {
        match NonZeroU16::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
    pub const fn get(self) -> u16 {
        self.0.get()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnicodeScalar(char);
impl UnicodeScalar {
    pub const fn new(value: char) -> Self {
        Self(value)
    }
    pub const fn get(self) -> char {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeatureSetting {
    pub tag: OpenTypeTag,
    pub value: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontRequest {
    pub families: Vec<String>,
    pub language: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontInstance {
    pub id: FontInstanceId,
    pub face_id: FontFaceId,
    pub admitted_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontFamilyError {
    EmptyFamily,
    DuplicateFamily,
    EmptyFallbackList,
    UnknownFamily,
}

/// Unique Profile 1.0 family lookup used before shaping. Fallback order is
/// request order; the first declared family wins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontFamilyTable {
    by_family: BTreeMap<String, FontFaceId>,
}
impl FontFamilyTable {
    pub fn new(families: Vec<(String, FontFaceId)>) -> Result<Self, FontFamilyError> {
        let mut by_family = BTreeMap::new();
        for (family, face_id) in families {
            if family.trim().is_empty() || family.chars().any(char::is_control) {
                return Err(FontFamilyError::EmptyFamily);
            }
            if by_family.insert(family, face_id).is_some() {
                return Err(FontFamilyError::DuplicateFamily);
            }
        }
        Ok(Self { by_family })
    }
    pub fn resolve(&self, fallback: &[String]) -> Result<FontFaceId, FontFamilyError> {
        if fallback.is_empty() {
            return Err(FontFamilyError::EmptyFallbackList);
        }
        fallback
            .iter()
            .find_map(|family| self.by_family.get(family).copied())
            .ok_or(FontFamilyError::UnknownFamily)
    }
}
pub trait FontResolver {
    type Error;
    fn resolve_cluster(
        &self,
        request: &FontRequest,
        features: &[FeatureSetting],
        cluster: &str,
    ) -> Result<FontInstance, Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlyphSubsetBinding {
    pub original_gid: OriginalGlyphId,
    pub subset_gid: SubsetGlyphId,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CidBinding {
    pub cid: Cid,
    pub subset_gid: SubsetGlyphId,
    pub unicode: Vec<UnicodeScalar>,
    pub width_1000: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontSubsetPlan {
    /// Must contain original GID 0 -> subset GID 0 and unique keys/values.
    pub glyphs: Vec<GlyphSubsetBinding>,
    /// CID 0 is reserved for .notdef and therefore absent here.
    pub cids: Vec<CidBinding>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FontPlanError {
    MissingNotdef,
    DuplicateOriginalGlyph,
    DuplicateSubsetGlyph,
    DuplicateCid,
    UnknownSubsetGlyph,
    NonCanonicalGlyphOrder,
    NonDenseSubsetGlyph,
    NonCanonicalCidOrder,
}
impl FontSubsetPlan {
    pub fn validate(&self) -> Result<(), FontPlanError> {
        use std::collections::BTreeSet;
        let mut originals = BTreeSet::new();
        let mut subsets = BTreeSet::new();
        let mut previous_original = None;
        for (index, binding) in self.glyphs.iter().enumerate() {
            if previous_original.is_some_and(|previous| previous >= binding.original_gid) {
                return Err(FontPlanError::NonCanonicalGlyphOrder);
            }
            if usize::from(binding.subset_gid.get()) != index {
                return Err(FontPlanError::NonDenseSubsetGlyph);
            }
            if !originals.insert(binding.original_gid) {
                return Err(FontPlanError::DuplicateOriginalGlyph);
            }
            if !subsets.insert(binding.subset_gid) {
                return Err(FontPlanError::DuplicateSubsetGlyph);
            }
            previous_original = Some(binding.original_gid);
        }
        if !self
            .glyphs
            .iter()
            .any(|b| b.original_gid.get() == 0 && b.subset_gid.get() == 0)
        {
            return Err(FontPlanError::MissingNotdef);
        }
        let mut cids = BTreeSet::new();
        let mut previous_cid = None;
        for (index, binding) in self.cids.iter().enumerate() {
            if binding.cid.get() as usize != index + 1
                || previous_cid.is_some_and(|previous| previous >= binding.cid)
            {
                return Err(FontPlanError::NonCanonicalCidOrder);
            }
            if !cids.insert(binding.cid) {
                return Err(FontPlanError::DuplicateCid);
            }
            if !subsets.contains(&binding.subset_gid) {
                return Err(FontPlanError::UnknownSubsetGlyph);
            }
            previous_cid = Some(binding.cid);
        }
        Ok(())
    }
}
pub trait FontSubsetter {
    type Error;
    fn subset(&self, face: FontFaceId, plan: &FontSubsetPlan) -> Result<Vec<u8>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cid_zero_is_reserved() {
        assert!(Cid::new(0).is_none());
        assert!(Cid::new(1).is_some());
    }
    #[test]
    fn plan_requires_notdef() {
        let plan = FontSubsetPlan {
            glyphs: vec![],
            cids: vec![],
        };
        assert_eq!(plan.validate(), Err(FontPlanError::MissingNotdef));
    }

    #[test]
    fn family_table_is_unique_and_uses_declared_fallback_order() {
        let table = FontFamilyTable::new(vec![
            ("Body".to_owned(), FontFaceId::new(1)),
            ("Fallback".to_owned(), FontFaceId::new(2)),
        ])
        .unwrap();
        assert_eq!(
            table
                .resolve(&["Missing".to_owned(), "Fallback".to_owned()])
                .unwrap(),
            FontFaceId::new(2)
        );
        assert_eq!(
            FontFamilyTable::new(vec![
                ("Body".to_owned(), FontFaceId::new(1)),
                ("Body".to_owned(), FontFaceId::new(2)),
            ]),
            Err(FontFamilyError::DuplicateFamily)
        );
        assert_eq!(
            table.resolve(&["Missing".to_owned()]),
            Err(FontFamilyError::UnknownFamily)
        );
    }
}
