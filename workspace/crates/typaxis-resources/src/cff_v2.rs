//! Private-staging /2 font plans. These are distinct from /1 encoder receipts
//! and do not authorize document paint, terminal state or PDF publication.
use super::*;
use typaxis_core::{FontFaceId, GlyphRunId, M4EffectiveResourceLimits, PositiveLength};
use typaxis_font::{Cff1AdmissionV2, Cff1SubsetSessionV2, Cff1SubsetV2, CffSelectionFailureV2};
use typaxis_shaping::{Cff1ShapedRunV2, ShapeSourceSpan};

pub struct Cff1PdfFontInputV2<'a, 'font> {
    pub font_face_id: FontFaceId,
    pub font_instance_id: FontInstanceId,
    pub admission: &'font Cff1AdmissionV2,
    pub runs: &'a [&'a Cff1ShapedRunV2<'font>],
}
#[derive(Debug)]
pub enum Cff1PdfPlanErrorV2 {
    InvalidInput,
    IdentityMismatch,
    ResourceLimit,
    Font(Cff1Error),
    Selection(CffSelectionFailureV2),
}
impl std::fmt::Display for Cff1PdfPlanErrorV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CFF /2 PDF plan {self:?}")
    }
}
impl std::error::Error for Cff1PdfPlanErrorV2 {}
#[derive(Debug)]
pub struct FrozenPdfCff1ClusterV2 {
    run_id: GlyphRunId,
    cluster_index: u32,
    font_size: PositiveLength,
    source: ShapeSourceSpan,
    exact_text: String,
    cids: Vec<Cid>,
    requires_actual_text: bool,
}
impl FrozenPdfCff1ClusterV2 {
    pub fn font_size(&self) -> PositiveLength {
        self.font_size
    }
    pub fn run_id(&self) -> GlyphRunId {
        self.run_id
    }
    pub fn cluster_index(&self) -> u32 {
        self.cluster_index
    }
    pub fn source(&self) -> ShapeSourceSpan {
        self.source
    }
    pub fn exact_text(&self) -> &str {
        &self.exact_text
    }
    pub fn cids(&self) -> &[Cid] {
        &self.cids
    }
    pub fn requires_actual_text(&self) -> bool {
        self.requires_actual_text
    }
}
#[derive(Debug)]
pub struct FrozenPdfCff1PlanV2 {
    subset: Cff1SubsetV2,
    admission_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    bindings: Vec<CidBinding>,
    dense_widths: Vec<u32>,
    clusters: Vec<FrozenPdfCff1ClusterV2>,
    fingerprint: [u8; 32],
    canonical_jcs: String,
}
impl FrozenPdfCff1PlanV2 {
    pub fn plan_id(&self) -> &'static str {
        "typaxis.cff1-pdf-plan/2"
    }
    pub fn font_face_id(&self) -> FontFaceId {
        self.subset.closure().font_face_id()
    }
    pub fn font_instance_id(&self) -> FontInstanceId {
        self.subset.closure().font_instance_id()
    }
    pub fn subset(&self) -> &Cff1SubsetV2 {
        &self.subset
    }
    pub fn admission_fingerprint(&self) -> [u8; 32] {
        self.admission_fingerprint
    }
    pub fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub fn bindings(&self) -> &[CidBinding] {
        &self.bindings
    }
    pub fn dense_widths_1000(&self) -> &[u32] {
        &self.dense_widths
    }
    pub fn clusters(&self) -> &[FrozenPdfCff1ClusterV2] {
        &self.clusters
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub fn indirect_object_blueprint(&self) -> &[PdfFontIndirectObjectRole; 6] {
        &PDF_CFF1_FONT_OBJECT_BLUEPRINT
    }
}
use Cff1PdfPlanErrorV2 as E;
struct Budget<'a> {
    limits: &'a M4EffectiveResourceLimits,
    records: u64,
    bytes: u64,
}
impl Budget<'_> {
    fn records(&mut self, n: usize) -> Result<(), E> {
        self.records = self
            .records
            .checked_add(n as u64)
            .filter(|v| *v <= self.limits.base().get().max_fragments)
            .ok_or(E::ResourceLimit)?;
        Ok(())
    }
    fn bytes(&mut self, n: usize) -> Result<(), E> {
        self.bytes = self
            .bytes
            .checked_add(n as u64)
            .filter(|v| *v <= self.limits.base().get().max_spool_bytes)
            .ok_or(E::ResourceLimit)?;
        Ok(())
    }
}
struct Prepared<'a, 'font> {
    input: &'a Cff1PdfFontInputV2<'a, 'font>,
    closure: typaxis_font::Cff1GlyphClosureV2,
    unicode: BTreeMap<OriginalGlyphId, Option<UnicodeScalar>>,
}

/// Close every instance before executing any glyph. Face unions execute in
/// FontFaceId/GID order under one budget; output plans are FontInstanceId ordered.
pub fn freeze_cff1_pdf_fonts_v2<'a, 'font>(
    inputs: &'a [Cff1PdfFontInputV2<'a, 'font>],
) -> Result<Vec<FrozenPdfCff1PlanV2>, E> {
    let Some(first) = inputs.first() else {
        return Ok(Vec::new());
    };
    let limits = first.admission.effective_limits();
    let mut budget = Budget {
        limits,
        records: 0,
        bytes: 0,
    };
    budget.records(inputs.len())?;
    let mut prepared = BTreeMap::new();
    let mut faces: BTreeMap<FontFaceId, (&Cff1AdmissionV2, BTreeSet<OriginalGlyphId>)> =
        BTreeMap::new();
    for input in inputs {
        if input.admission.limits_fingerprint() != limits.fingerprint() {
            return Err(E::IdentityMismatch);
        }
        if input.runs.is_empty() {
            return Err(E::InvalidInput);
        }
        let mut gids = BTreeSet::new();
        let mut unicode = BTreeMap::new();
        let mut run_ids = BTreeSet::new();
        for shaped in input.runs {
            let run = shaped.glyph_run();
            if shaped.admission().fingerprint() != input.admission.fingerprint()
                || run.font != input.font_instance_id
            {
                return Err(E::IdentityMismatch);
            }
            budget.records(1)?;
            if !run_ids.insert(run.run_id) {
                return Err(E::InvalidInput);
            }
            budget.records(run.glyphs.len())?;
            budget.records(run.clusters.len())?;
            for glyph in &run.glyphs {
                if glyph.original_gid.get() == 0 {
                    return Err(E::InvalidInput);
                }
                gids.insert(glyph.original_gid);
            }
            for (i, cluster) in run.clusters.iter().enumerate() {
                let text = shaped.cluster_text(i).ok_or(E::InvalidInput)?;
                if text.is_empty() {
                    return Err(E::InvalidInput);
                }
                budget.bytes(text.len())?;
                let glyphs = run
                    .glyphs
                    .get(cluster.glyph_start as usize..cluster.glyph_end as usize)
                    .filter(|g| !g.is_empty())
                    .ok_or(E::InvalidInput)?;
                let mut chars = text.chars();
                let scalar = chars.next().filter(|_| chars.next().is_none());
                let candidate = if glyphs.len() == 1 {
                    scalar.map(UnicodeScalar::new)
                } else {
                    None
                };
                for g in glyphs {
                    unicode
                        .entry(g.original_gid)
                        .and_modify(|old| {
                            if *old != candidate {
                                *old = None;
                            }
                        })
                        .or_insert(candidate);
                }
            }
        }
        budget.records(gids.len())?;
        let closure = Cff1SubsetSessionV2::close_instance_selection(
            input.admission,
            input.font_face_id,
            input.font_instance_id,
            &gids,
            limits.base().get().max_cids_per_font,
        )
        .map_err(E::Font)?;
        let face = faces
            .entry(input.font_face_id)
            .or_insert_with(|| (input.admission, BTreeSet::new()));
        if face.0.fingerprint() != input.admission.fingerprint() {
            return Err(E::IdentityMismatch);
        }
        face.1.extend(gids);
        if prepared
            .insert(
                input.font_instance_id,
                Prepared {
                    input,
                    closure,
                    unicode,
                },
            )
            .is_some()
        {
            return Err(E::InvalidInput);
        }
    }
    if faces.len() > limits.base().get().max_fonts as usize {
        return Err(E::ResourceLimit);
    }
    let mut session = Cff1SubsetSessionV2::from_admission(first.admission);
    for (admission, gids) in faces.values() {
        session
            .prepare_face(admission, gids)
            .map_err(E::Selection)?;
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(prepared.len())
        .map_err(|_| E::ResourceLimit)?;
    for prepared in prepared.into_values() {
        let subset = session
            .subset(prepared.input.admission, prepared.closure)
            .map_err(E::Selection)?;
        budget.bytes(subset.bytes().len())?;
        budget.records(subset.original_to_subset().len())?;
        let mut bindings = Vec::new();
        let mut dense_widths = Vec::new();
        bindings
            .try_reserve_exact(subset.original_to_subset().len())
            .map_err(|_| E::ResourceLimit)?;
        dense_widths
            .try_reserve_exact(subset.original_to_subset().len())
            .map_err(|_| E::ResourceLimit)?;
        for (&source, &dense) in subset.original_to_subset() {
            if usize::from(dense.get()) != dense_widths.len() {
                return Err(E::InvalidInput);
            }
            let width = u32::from(subset.original_widths()[&source]);
            dense_widths.push(width);
            if dense.get() != 0 {
                bindings.push(CidBinding {
                    cid: Cid::new(dense.get()).ok_or(E::InvalidInput)?,
                    subset_gid: dense,
                    unicode: prepared
                        .unicode
                        .get(&source)
                        .copied()
                        .flatten()
                        .into_iter()
                        .collect(),
                    width_1000: width,
                });
            }
        }
        let mut clusters = Vec::new();
        for shaped in prepared.input.runs {
            for (i, cluster) in shaped.glyph_run().clusters.iter().enumerate() {
                let text = shaped.cluster_text(i).ok_or(E::InvalidInput)?;
                let mut cids = Vec::new();
                cids.try_reserve_exact((cluster.glyph_end - cluster.glyph_start) as usize)
                    .map_err(|_| E::ResourceLimit)?;
                for glyph in &shaped.glyph_run().glyphs
                    [cluster.glyph_start as usize..cluster.glyph_end as usize]
                {
                    let dense = subset.original_to_subset()[&glyph.original_gid];
                    cids.push(Cid::new(dense.get()).ok_or(E::InvalidInput)?);
                }
                let mut extracted = cids.iter().flat_map(|cid| {
                    bindings[usize::from(cid.get()) - 1]
                        .unicode
                        .iter()
                        .map(|s| s.get())
                });
                let exact = extracted.by_ref().eq(text.chars());
                let mut owned = String::new();
                owned
                    .try_reserve_exact(text.len())
                    .map_err(|_| E::ResourceLimit)?;
                owned.push_str(text);
                clusters.try_reserve(1).map_err(|_| E::ResourceLimit)?;
                clusters.push(FrozenPdfCff1ClusterV2 {
                    run_id: shaped.glyph_run().run_id,
                    cluster_index: i as u32,
                    font_size: shaped.font_size(),
                    source: cluster.source_span,
                    exact_text: owned,
                    cids,
                    requires_actual_text: !exact,
                });
            }
        }
        let mut plan = FrozenPdfCff1PlanV2 {
            subset,
            admission_fingerprint: prepared.input.admission.fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            bindings,
            dense_widths,
            clusters,
            fingerprint: [0; 32],
            canonical_jcs: String::new(),
        };
        // Reserve the worst-case escaped representation before serialization.
        // This local spool charge is conservative; the document owner must
        // additionally account for all earlier pipeline allocations.
        let capacity = canonical_capacity(&plan)?;
        budget.bytes(capacity)?;
        plan.canonical_jcs = encode(&plan, capacity)?;
        plan.fingerprint = sha256(plan.canonical_jcs.as_bytes());
        output.push(plan);
    }
    Ok(output)
}
fn canonical_capacity(plan: &FrozenPdfCff1PlanV2) -> Result<usize, E> {
    plan.clusters.iter().try_fold(512usize, |total, c| {
        total
            .checked_add(512)
            .and_then(|n| {
                c.exact_text
                    .len()
                    .checked_mul(6)
                    .and_then(|v| n.checked_add(v))
            })
            .and_then(|n| c.cids.len().checked_mul(6).and_then(|v| n.checked_add(v)))
            .ok_or(E::ResourceLimit)
    })
}
fn encode(plan: &FrozenPdfCff1PlanV2, capacity: usize) -> Result<String, E> {
    let mut s = String::new();
    s.try_reserve_exact(capacity)
        .map_err(|_| E::ResourceLimit)?;
    s.push_str("{\"admission_fingerprint\":");
    push_resource_hash(&mut s, plan.admission_fingerprint);
    s.push_str(",\"algorithm\":\"typaxis.cff1-pdf-plan/2\",\"clusters\":[");
    for (i, c) in plan.clusters.iter().enumerate() {
        if i != 0 {
            s.push(',');
        }
        s.push_str("{\"actual_text\":");
        s.push_str(if c.requires_actual_text {
            "true"
        } else {
            "false"
        });
        s.push_str(",\"cids\":[");
        for (i, cid) in c.cids.iter().enumerate() {
            if i != 0 {
                s.push(',');
            }
            s.push_str(&cid.get().to_string());
        }
        s.push_str("],\"cluster_index\":");
        s.push_str(&c.cluster_index.to_string());
        s.push_str(",\"font_size\":");
        s.push_str(&c.font_size.get().raw().to_string());
        s.push_str(",\"run_id\":");
        s.push_str(&c.run_id.get().to_string());
        s.push_str(",\"source\":");
        encode_source(&mut s, c.source);
        s.push_str(",\"text\":");
        push_jcs_string(&mut s, &c.exact_text);
        s.push('}');
    }
    s.push_str("],\"limits_fingerprint\":");
    push_resource_hash(&mut s, plan.limits_fingerprint);
    s.push_str(",\"subset_fingerprint\":");
    push_resource_hash(&mut s, plan.subset.fingerprint());
    s.push('}');
    debug_assert!(s.len() <= capacity);
    Ok(s)
}

fn encode_source(s: &mut String, source: ShapeSourceSpan) {
    use std::fmt::Write;
    use typaxis_core::GenerationKind;
    match source {
        ShapeSourceSpan::Parsed(span) => {
            write!(
                s,
                "{{\"end_byte\":{},\"kind\":\"parsed\",\"start_byte\":{},\"text_id\":{}}}",
                span.end_byte().get(),
                span.start_byte().get(),
                span.text_id().get()
            )
            .unwrap();
        }
        ShapeSourceSpan::Generated(provenance) => {
            let key = provenance.buffer_key();
            let span = provenance.text_span();
            let kind = match key.generation_kind() {
                GenerationKind::PageReference => "page-reference",
                GenerationKind::Counter => "counter",
                GenerationKind::ListMarker => "list-marker",
                GenerationKind::FootnoteMarker => "footnote-marker",
                GenerationKind::Discretionary => "discretionary",
            };
            write!(s, "{{\"end_byte\":{},\"generation_kind\":\"{}\",\"kind\":\"generated\",\"owner\":{},\"owner_local_ordinal\":{},\"start_byte\":{},\"text_id\":{}}}", span.range().end_byte().get(), kind, key.owner().get(), key.owner_local_ordinal(), span.range().start_byte().get(), span.text_id().get()).unwrap();
        }
    }
}

#[cfg(test)]
#[path = "cff_v2_tests.rs"]
mod tests;
