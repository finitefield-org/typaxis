//! Source-bound selection and aggregate evaluation for CID CFF /2. Only sealed
//! admission and selection owners feed the cache; layout advances come from hmtx.
use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cff1GlyphClosureV2 {
    admission_fingerprint: [u8; 32],
    source_sha256: [u8; 32],
    font_face_id: FontFaceId,
    font_instance_id: FontInstanceId,
    source_gids: Vec<OriginalGlyphId>,
    fingerprint: [u8; 32],
    canonical_jcs: String,
}
impl Cff1GlyphClosureV2 {
    pub fn source_gids(&self) -> &[OriginalGlyphId] {
        &self.source_gids
    }
    pub fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub fn font_instance_id(&self) -> FontInstanceId {
        self.font_instance_id
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub fn subset_gid(&self, gid: OriginalGlyphId) -> Option<SubsetGlyphId> {
        self.source_gids
            .binary_search(&gid)
            .ok()
            .and_then(|i| u16::try_from(i).ok())
            .map(SubsetGlyphId::new)
    }
}
#[derive(Debug)]
pub enum CffSelectionFailureV2 {
    Selection(Cff1Error),
    Glyph(CffGlyphFailureV2),
}
impl std::fmt::Display for CffSelectionFailureV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CFF /2 selection {self:?}")
    }
}
impl std::error::Error for CffSelectionFailureV2 {}
impl From<Cff1Error> for CffSelectionFailureV2 {
    fn from(e: Cff1Error) -> Self {
        Self::Selection(e)
    }
}

/// This owner has one fixed /2 profile and standalone face index 0. Its cache
/// key is therefore (source SHA-256, GID); font-instance aliases share work.
#[derive(Debug)]
pub struct Cff1SubsetSessionV2 {
    limits_fingerprint: [u8; 32],
    evaluator: CffProgramEvaluationSessionV2,
    evaluated: BTreeMap<([u8; 32], u16), CffEvaluatedGlyphV2>,
}
impl Cff1SubsetSessionV2 {
    pub fn new(limits: &M4EffectiveResourceLimits) -> Self {
        Self {
            limits_fingerprint: limits.fingerprint(),
            evaluator: CffProgramEvaluationSessionV2::new(limits),
            evaluated: BTreeMap::new(),
        }
    }
    pub fn from_admission(admission: &Cff1AdmissionV2) -> Self {
        Self::new(admission.effective_limits())
    }
    pub fn operations_used(&self) -> u64 {
        self.evaluator.operations_used()
    }
    pub fn outline_segments_used(&self) -> u64 {
        self.evaluator.outline_segments_used()
    }
    pub fn cached_glyph_count(&self) -> usize {
        self.evaluated.len()
    }
    pub fn close_instance_selection(
        admission: &Cff1AdmissionV2,
        font_face_id: FontFaceId,
        font_instance_id: FontInstanceId,
        selected: &BTreeSet<OriginalGlyphId>,
        max_cids_per_font: u16,
    ) -> Result<Cff1GlyphClosureV2, Cff1Error> {
        let nonzero = selected.iter().filter(|g| g.get() != 0).count();
        if nonzero > usize::from(max_cids_per_font.min(65534)) {
            return Err(Cff1Error::SelectedGlyphLimit);
        }
        if selected.iter().any(|g| g.get() >= admission.glyph_count()) {
            return Err(Cff1Error::InvalidSelectedGlyph);
        }
        let mut source_gids = Vec::new();
        source_gids
            .try_reserve_exact(nonzero + 1)
            .map_err(|_| Cff1Error::SelectedGlyphLimit)?;
        source_gids.push(OriginalGlyphId::new(0));
        source_gids.extend(selected.iter().copied().filter(|g| g.get() != 0));
        let mut canonical_jcs = String::from("{\"admission_fingerprint\":");
        push_hash(&mut canonical_jcs, admission.fingerprint());
        canonical_jcs.push_str(",\"algorithm\":\"typaxis.cff1-glyph-closure/2\",\"font_face_id\":");
        canonical_jcs.push_str(&font_face_id.get().to_string());
        canonical_jcs.push_str(",\"font_instance_id\":");
        canonical_jcs.push_str(&font_instance_id.get().to_string());
        canonical_jcs.push_str(",\"source_gids\":[");
        for (i, gid) in source_gids.iter().enumerate() {
            if i != 0 {
                canonical_jcs.push(',');
            }
            canonical_jcs.push_str(&gid.get().to_string());
        }
        canonical_jcs.push_str("],\"source_sha256\":");
        push_hash(&mut canonical_jcs, admission.source_sha256());
        canonical_jcs.push('}');
        Ok(Cff1GlyphClosureV2 {
            admission_fingerprint: admission.fingerprint(),
            source_sha256: admission.source_sha256(),
            font_face_id,
            font_instance_id,
            source_gids,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }
    /// Aggregate owners first close all instance selections, form each face's
    /// union and invoke this in FontFaceId order. GIDs execute in sorted order.
    pub fn prepare_face(
        &mut self,
        admission: &Cff1AdmissionV2,
        selected: &BTreeSet<OriginalGlyphId>,
    ) -> Result<(), CffSelectionFailureV2> {
        self.require_admission(admission)?;
        // Validate the entire request before spending any Type2 work.
        if selected.iter().any(|g| g.get() >= admission.glyph_count()) {
            return Err(Cff1Error::InvalidSelectedGlyph.into());
        }
        self.evaluate(admission, 0)?;
        for gid in selected {
            self.evaluate(admission, gid.get())?;
        }
        Ok(())
    }
    pub fn prepare_closure(
        &mut self,
        admission: &Cff1AdmissionV2,
        closure: &Cff1GlyphClosureV2,
    ) -> Result<(), CffSelectionFailureV2> {
        self.prepare_closure_with_charge(admission, closure, &mut |_, _, _| Ok(()))
    }
    pub fn prepare_closure_with_charge(
        &mut self,
        admission: &Cff1AdmissionV2,
        closure: &Cff1GlyphClosureV2,
        charge: &mut dyn FnMut(usize, usize, usize) -> Result<(), Cff1Error>,
    ) -> Result<(), CffSelectionFailureV2> {
        self.require_closure(admission, closure)?;
        for gid in &closure.source_gids {
            self.evaluate_with_charge(admission, gid.get(), charge)?;
        }
        Ok(())
    }
    pub fn evaluated_glyph(
        &self,
        admission: &Cff1AdmissionV2,
        gid: OriginalGlyphId,
    ) -> Result<Option<&CffEvaluatedGlyphV2>, Cff1Error> {
        self.require_admission(admission)?;
        Ok(self.evaluated.get(&(admission.source_sha256(), gid.get())))
    }
    fn require_admission(&self, admission: &Cff1AdmissionV2) -> Result<(), Cff1Error> {
        if self.limits_fingerprint != admission.limits_fingerprint() {
            return Err(Cff1Error::ReceiptMismatch);
        }
        Ok(())
    }
    pub(super) fn require_closure(
        &self,
        admission: &Cff1AdmissionV2,
        closure: &Cff1GlyphClosureV2,
    ) -> Result<(), Cff1Error> {
        self.require_admission(admission)?;
        if closure.admission_fingerprint != admission.fingerprint() {
            return Err(Cff1Error::ReceiptMismatch);
        }
        Ok(())
    }
    fn evaluate(
        &mut self,
        admission: &Cff1AdmissionV2,
        gid: u16,
    ) -> Result<(), CffSelectionFailureV2> {
        self.evaluate_with_charge(admission, gid, &mut |_, _, _| Ok(()))
    }
    fn evaluate_with_charge(
        &mut self,
        admission: &Cff1AdmissionV2,
        gid: u16,
        charge: &mut dyn FnMut(usize, usize, usize) -> Result<(), Cff1Error>,
    ) -> Result<(), CffSelectionFailureV2> {
        let depth = (usize::BITS - self.evaluated.len().saturating_add(1).leading_zeros()) as usize;
        let lookup = depth * 12 * 32;
        charge(0, 0, lookup)?;
        let key = (admission.source_sha256(), gid);
        if self.evaluated.contains_key(&key) {
            return Ok(());
        }
        let entry = std::mem::size_of::<(([u8; 32], u16), CffEvaluatedGlyphV2)>();
        let storage = entry * 4
            + if self.evaluated.is_empty() {
                entry * 24 + 512
            } else {
                0
            };
        charge(1, storage, lookup)?;
        let advance = admission
            .horizontal_metric(gid)
            .ok_or(Cff1Error::InvalidSelectedGlyph)?
            .0;
        let result = self
            .evaluator
            .evaluate_with_charge(admission.program(), gid, advance, charge)
            .map_err(CffSelectionFailureV2::Glyph)?;
        self.evaluated.insert(key, result);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits(extension: M4ResourceLimits) -> M4EffectiveResourceLimits {
        M4EffectiveResourceLimits::new(
            typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
                .unwrap(),
            extension,
        )
        .unwrap()
    }
    #[test]
    #[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
    fn cff_v2_selection_original_harano_shares_source_work_and_seals_dense_mapping() {
        let source: Arc<[u8]> = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap())
            .unwrap()
            .into();
        let hex = |b: &[u8]| b.iter().map(|v| format!("{v:02x}")).collect::<String>();
        assert_eq!(
            hex(&sha256(&source)),
            "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
        );
        let defaults = limits(M4ResourceLimits::default());
        let admission = admit_sfnt_cff1_v2(source.clone(), 0, &defaults).unwrap();
        let gids: BTreeSet<_> = [233u16, 151, 233, 0]
            .into_iter()
            .map(OriginalGlyphId::new)
            .collect();
        let first = Cff1SubsetSessionV2::close_instance_selection(
            &admission,
            FontFaceId::new(1),
            FontInstanceId::new(1),
            &gids,
            2,
        )
        .unwrap();
        let second = Cff1SubsetSessionV2::close_instance_selection(
            &admission,
            FontFaceId::new(2),
            FontInstanceId::new(2),
            &gids,
            2,
        )
        .unwrap();
        assert_eq!(
            first
                .source_gids()
                .iter()
                .map(|g| g.get())
                .collect::<Vec<_>>(),
            [0, 151, 233]
        );
        for (gid, dense) in [(0, 0), (151, 1), (233, 2)] {
            assert_eq!(
                first.subset_gid(OriginalGlyphId::new(gid)).unwrap().get(),
                dense
            );
        }
        assert_eq!(first.subset_gid(OriginalGlyphId::new(1)), None);
        assert_ne!(first.fingerprint(), second.fingerprint());
        assert_eq!(
            Cff1SubsetSessionV2::close_instance_selection(
                &admission,
                FontFaceId::new(1),
                FontInstanceId::new(1),
                &gids,
                1
            )
            .unwrap_err(),
            Cff1Error::SelectedGlyphLimit
        );
        let mut session = Cff1SubsetSessionV2::from_admission(&admission);
        let invalid = [OriginalGlyphId::new(23060)].into_iter().collect();
        assert!(matches!(
            session.prepare_face(&admission, &invalid),
            Err(CffSelectionFailureV2::Selection(
                Cff1Error::InvalidSelectedGlyph
            ))
        ));
        assert_eq!(session.operations_used(), 0);
        session.prepare_closure(&admission, &first).unwrap();
        let work = (session.operations_used(), session.outline_segments_used());
        assert!(work.0 > 0 && work.1 > 0);
        assert_eq!(session.cached_glyph_count(), 3);
        session.prepare_closure(&admission, &second).unwrap();
        let mut alias_charge = (0, 0, 0);
        session
            .prepare_closure_with_charge(&admission, &second, &mut |records, bytes, work| {
                alias_charge.0 += records;
                alias_charge.1 += bytes;
                alias_charge.2 += work;
                Ok(())
            })
            .unwrap();
        assert_eq!((alias_charge.0, alias_charge.1), (0, 0));
        assert!(alias_charge.2 > 0);
        session.prepare_face(&admission, &gids).unwrap();
        assert_eq!(
            (session.operations_used(), session.outline_segments_used()),
            work
        );
        for (gid, advance) in [(151, 1000), (233, 500)] {
            assert_eq!(
                session
                    .evaluated_glyph(&admission, OriginalGlyphId::new(gid))
                    .unwrap()
                    .unwrap()
                    .advance(),
                advance
            );
        }
        assert!(session
            .evaluated_glyph(&admission, OriginalGlyphId::new(152))
            .unwrap()
            .is_none());
        // A separately admitted source with identical outlines must not reuse
        // the original source's closure or cache entries. Change only fsType
        // to allowed PreviewAndPrint, then repair the SFNT checksums.
        let records = preflight_sfnt_with_vertical(
            &source,
            *defaults.extension().get(),
            &mut FontFailureContext::new(0),
            true,
        )
        .unwrap();
        let head = records.iter().find(|r| r.tag == *b"head").unwrap().offset;
        let os2 = records.iter().find(|r| r.tag == *b"OS/2").unwrap().offset;
        let mut alternate = source.to_vec();
        alternate[head + 8..head + 12].fill(0);
        alternate[os2 + 9] = 4;
        for (i, r) in records.iter().enumerate() {
            let checksum = sfnt_checksum(&alternate[r.offset..r.offset + r.length]);
            alternate[16 + i * 16..20 + i * 16].copy_from_slice(&checksum.to_be_bytes());
        }
        let adjustment = SFNT_CHECKSUM_MAGIC.wrapping_sub(sfnt_checksum(&alternate));
        alternate[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
        let alternate = admit_sfnt_cff1_v2(alternate.into(), 0, &defaults).unwrap();
        assert_ne!(alternate.source_sha256(), admission.source_sha256());
        assert!(matches!(
            session.prepare_closure(&alternate, &first),
            Err(CffSelectionFailureV2::Selection(Cff1Error::ReceiptMismatch))
        ));
        assert_eq!(
            (session.operations_used(), session.outline_segments_used()),
            work
        );
        let alternate_closure = Cff1SubsetSessionV2::close_instance_selection(
            &alternate,
            FontFaceId::new(3),
            FontInstanceId::new(3),
            &gids,
            2,
        )
        .unwrap();
        session
            .prepare_closure(&alternate, &alternate_closure)
            .unwrap();
        assert_eq!(session.cached_glyph_count(), 6);
        assert_eq!(
            (session.operations_used(), session.outline_segments_used()),
            (work.0 * 2, work.1 * 2)
        );
        for (operations, segments, success) in [
            (work.0, work.1, true),
            (work.0 - 1, work.1, false),
            (work.0, work.1 - 1, false),
        ] {
            let mut extension = M4ResourceLimits::default();
            extension.max_cff_charstring_operations = operations;
            extension.max_cff_outline_segments = segments;
            let bounded = limits(extension);
            let bounded_admission = admit_sfnt_cff1_v2(source.clone(), 0, &bounded).unwrap();
            let closure = Cff1SubsetSessionV2::close_instance_selection(
                &bounded_admission,
                FontFaceId::new(1),
                FontInstanceId::new(1),
                &gids,
                2,
            )
            .unwrap();
            let mut bounded_session = Cff1SubsetSessionV2::from_admission(&bounded_admission);
            assert!(matches!(
                bounded_session.prepare_closure(&bounded_admission, &first),
                Err(CffSelectionFailureV2::Selection(Cff1Error::ReceiptMismatch))
            ));
            assert_eq!(bounded_session.operations_used(), 0);
            assert!(matches!(
                bounded_session.prepare_closure(&admission, &first),
                Err(CffSelectionFailureV2::Selection(Cff1Error::ReceiptMismatch))
            ));
            assert_eq!(
                bounded_session
                    .prepare_closure(&bounded_admission, &closure)
                    .is_ok(),
                success
            );
            assert!(bounded_session.operations_used() <= operations);
            assert!(bounded_session.outline_segments_used() <= segments);
        }
    }
}
