//! Canonical dense-CID OpenType subsets produced from sealed /2 selections.
use super::*;
#[path = "cff_v2_subset_budget.rs"]
mod budget;
use budget::{add, mul, Charge};
#[cfg(test)]
#[path = "cff_v2_subset_budget_tests.rs"]
mod budget_tests;

#[derive(Debug)]
pub struct Cff1SubsetV2 {
    bytes: Vec<u8>,
    sha256: [u8; 32],
    postscript_name: String,
    mapping: BTreeMap<OriginalGlyphId, SubsetGlyphId>,
    widths: BTreeMap<OriginalGlyphId, u16>,
    metrics: Cff1PdfMetrics,
    closure: Cff1GlyphClosureV2,
    fingerprint: [u8; 32],
    canonical_jcs: String,
}
impl Cff1SubsetV2 {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
    pub fn postscript_name(&self) -> &str {
        &self.postscript_name
    }
    pub fn original_to_subset(&self) -> &BTreeMap<OriginalGlyphId, SubsetGlyphId> {
        &self.mapping
    }
    pub fn original_widths(&self) -> &BTreeMap<OriginalGlyphId, u16> {
        &self.widths
    }
    pub fn metrics(&self) -> &Cff1PdfMetrics {
        &self.metrics
    }
    pub fn closure(&self) -> &Cff1GlyphClosureV2 {
        &self.closure
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}
impl Cff1SubsetSessionV2 {
    pub fn subset(
        &mut self,
        admission: &Cff1AdmissionV2,
        closure: Cff1GlyphClosureV2,
    ) -> Result<Cff1SubsetV2, CffSelectionFailureV2> {
        self.prepare_closure(admission, &closure)?;
        Ok(write_subset(self, admission, closure)?)
    }
}
fn write_subset(
    session: &Cff1SubsetSessionV2,
    admission: &Cff1AdmissionV2,
    closure: Cff1GlyphClosureV2,
) -> Result<Cff1SubsetV2, Cff1Error> {
    session.write_prepared_subset_with_charge(admission, closure, &mut |_, _, _| Ok(()))
}
impl Cff1SubsetSessionV2 {
    /// Only writes already evaluated glyphs. The owning book session must
    /// close all fonts and perform aggregate evaluation before calling this.
    pub fn write_prepared_subset_with_charge(
        &self,
        admission: &Cff1AdmissionV2,
        closure: Cff1GlyphClosureV2,
        charge: Charge<'_>,
    ) -> Result<Cff1SubsetV2, Cff1Error> {
        self.require_closure(admission, &closure)?;
        write_subset_charged(self, admission, closure, charge)
    }
}
fn write_subset_charged(
    session: &Cff1SubsetSessionV2,
    admission: &Cff1AdmissionV2,
    closure: Cff1GlyphClosureV2,
    charge: Charge<'_>,
) -> Result<Cff1SubsetV2, Cff1Error> {
    let max_bytes = admission
        .effective_limits()
        .extension()
        .get()
        .max_font_subset_bytes;
    let n = closure.source_gids().len();
    let headers = mul(
        n,
        std::mem::size_of::<Vec<u8>>() + std::mem::size_of::<[i16; 4]>(),
    )?;
    charge(
        add(mul(n, 4)?, 1)?,
        add(add(headers, mul(n, 256)?)?, 1038)?,
        mul(n, 256)?,
    )?;
    let name = subset_postscript_name(closure.font_instance_id())?;
    let mut mapping = BTreeMap::new();
    let mut widths = BTreeMap::new();
    let mut charstrings = Vec::new();
    let mut bboxes = Vec::new();
    charstrings
        .try_reserve_exact(closure.source_gids().len())
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    bboxes
        .try_reserve_exact(closure.source_gids().len())
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    let mut global: Option<[i16; 4]> = None;
    let mut charstring_bytes = 0u64;
    for (dense, gid) in closure.source_gids().iter().enumerate() {
        let depth = (usize::BITS
            - session
                .cached_glyph_count()
                .saturating_add(1)
                .leading_zeros()) as usize;
        charge(0, 0, depth * 12 * 32)?;
        let glyph = session
            .evaluated_glyph(admission, *gid)?
            .ok_or(Cff1Error::InvalidGlyphClosure)?;
        let bbox = glyph
            .control_bounds()
            .map(outward_i16_bbox)
            .transpose()?
            .unwrap_or([0; 4]);
        if glyph.control_bounds().is_some() {
            global = Some(match global {
                Some(old) => [
                    old[0].min(bbox[0]),
                    old[1].min(bbox[1]),
                    old[2].max(bbox[2]),
                    old[3].max(bbox[3]),
                ],
                None => bbox,
            });
        }
        charge(0, 0, mul(glyph.commands().len(), 128)?)?;
        let length = glyph.canonical_charstring_len()?;
        charstring_bytes = charstring_bytes
            .checked_add(length as u64)
            .filter(|n| *n <= max_bytes)
            .ok_or(Cff1Error::SubsetByteLimit)?;
        charge(0, length, add(mul(glyph.commands().len(), 256)?, length)?)?;
        let encoded = glyph.canonical_charstring()?;
        if encoded.len() != length {
            return Err(Cff1Error::InvalidSubset);
        }
        charstrings.push(encoded);
        bboxes.push(bbox);
        mapping.insert(
            *gid,
            SubsetGlyphId::new(u16::try_from(dense).map_err(|_| Cff1Error::SelectedGlyphLimit)?),
        );
        widths.insert(*gid, glyph.advance());
    }
    let bbox = global.ok_or(Cff1Error::InvalidSubset)?;
    if bbox[0] >= bbox[2] || bbox[1] >= bbox[3] {
        return Err(Cff1Error::InvalidSubset);
    }
    budget::cid_tables(
        n,
        usize::try_from(charstring_bytes).map_err(|_| Cff1Error::SubsetByteLimit)?,
        charge,
    )?;
    let cff = build_cid_cff(&name, bbox, &charstrings)?;
    let cmap = build_cmap_with_charge(admission.cmap(), &mapping, charge)?;
    budget::copied_tables(admission, n, charge)?;
    let head = build_subset_head(admission.table_bytes(b"head")?, bbox)?;
    let (advances, bearings) = admission.horizontal_metrics();
    let (hhea, hmtx) = build_subset_horizontal_metrics_from(
        admission.table_bytes(b"hhea")?,
        advances,
        bearings,
        closure.source_gids(),
        &bboxes,
    )?;
    let maxp = build_subset_maxp(closure.source_gids().len())?;
    let (family, subfamily) = admission.family_names();
    let names = build_subset_name_table(family, subfamily, &name)?;
    let tables = vec![
        RewriteTable {
            tag: *b"CFF ",
            bytes: cff,
        },
        RewriteTable {
            tag: *b"OS/2",
            bytes: admission.table_bytes(b"OS/2")?.to_vec(),
        },
        RewriteTable {
            tag: *b"cmap",
            bytes: cmap,
        },
        RewriteTable {
            tag: *b"head",
            bytes: head,
        },
        RewriteTable {
            tag: *b"hhea",
            bytes: hhea,
        },
        RewriteTable {
            tag: *b"hmtx",
            bytes: hmtx,
        },
        RewriteTable {
            tag: *b"maxp",
            bytes: maxp,
        },
        RewriteTable {
            tag: *b"name",
            bytes: names,
        },
        RewriteTable {
            tag: *b"post",
            bytes: admission.table_bytes(b"post")?.to_vec(),
        },
    ];
    let size = sfnt_output_size(&tables)?;
    if size > max_bytes {
        return Err(Cff1Error::SubsetByteLimit);
    }
    let output_size = usize::try_from(size).map_err(|_| Cff1Error::SubsetByteLimit)?;
    charge(1, output_size, add(mul(output_size, 4)?, 1024)?)?;
    let bytes = rebuild_sfnt(tables)?;
    if bytes.len() as u64 != size {
        return Err(Cff1Error::InvalidSubset);
    }
    let hash = sha256(&bytes);
    let metrics = subset_pdf_metrics_from(
        admission.table_bytes(b"hhea")?,
        admission.table_bytes(b"OS/2")?,
        admission.table_bytes(b"post")?,
        bbox,
    )?;
    charge(1, 532, 1024)?;
    let mut identity = String::with_capacity(512);
    identity.push_str("{\"algorithm\":\"typaxis.cff1-subset/2\",\"byte_length\":");
    identity.push_str(&size.to_string());
    identity.push_str(",\"closure_fingerprint\":");
    push_hash(&mut identity, closure.fingerprint());
    identity.push_str(",\"postscript_name\":");
    push_jcs_string(&mut identity, &name);
    identity.push_str(",\"sha256\":");
    push_hash(&mut identity, hash);
    identity.push_str(",\"source_sha256\":");
    push_hash(&mut identity, admission.source_sha256());
    identity.push('}');
    Ok(Cff1SubsetV2 {
        bytes,
        sha256: hash,
        postscript_name: name,
        mapping,
        widths,
        metrics,
        closure,
        fingerprint: sha256(identity.as_bytes()),
        canonical_jcs: identity,
    })
}
#[cfg(test)]
pub(super) fn build_cmap(
    source: &CffCmapV2,
    mapping: &BTreeMap<OriginalGlyphId, SubsetGlyphId>,
) -> Result<Vec<u8>, Cff1Error> {
    build_cmap_with_charge(source, mapping, &mut |_, _, _| Ok(()))
}
fn build_cmap_with_charge(
    source: &CffCmapV2,
    mapping: &BTreeMap<OriginalGlyphId, SubsetGlyphId>,
    charge: Charge<'_>,
) -> Result<Vec<u8>, Cff1Error> {
    let count = source.base_mapping_count();
    // Existing format-12 encoder reserves source pairs and selected groups;
    // both are bounded by the real base-map count (8 + 12 + 12 bytes/pair).
    charge(
        add(mul(count, 2)?, 1)?,
        add(mul(count, 32)?, 28)?,
        add(mul(count, 160)?, 64)?,
    )?;
    let base = build_subset_cmap_with_empty(source.base_map(), mapping, true)?;
    let mut selectors: BTreeMap<u32, Vec<(u32, u16)>> = BTreeMap::new();
    if let Some(variation) = source.variation_sequences() {
        variation.visit_pairs(|scalar, selector, _| {
            // Three coverage binary searches, base/selected tree lookups and
            // the selector map are bounded by the Unicode/16-bit key domains.
            charge(0, 0, 768)?;
            let gid = source.glyph_for_sequence(
                char::from_u32(scalar).unwrap(),
                Some(char::from_u32(selector).unwrap()),
            );
            if let Some(gid) = gid
                .filter(|g| *g != 0)
                .and_then(|g| mapping.get(&OriginalGlyphId::new(g)))
            {
                if gid.get() != 0 {
                    if !selectors.contains_key(&selector) {
                        charge(1, if selectors.is_empty() { 1536 } else { 512 }, 1)?;
                    }
                    let values = selectors.entry(selector).or_default();
                    if values.len() == values.capacity() {
                        let next = mul(values.capacity(), 2)?.max(4);
                        let delta = next - values.capacity();
                        charge(delta, mul(delta, std::mem::size_of::<(u32, u16)>())?, delta)?;
                        values
                            .try_reserve_exact(delta)
                            .map_err(|_| Cff1Error::SubsetByteLimit)?;
                    }
                    values.push((scalar, gid.get()));
                }
            }
            Ok::<_, Cff1Error>(())
        })?;
    }
    if selectors.is_empty() {
        return Ok(base);
    }
    let records = selectors.len();
    let total = selectors.values().try_fold(10 + 11 * records, |sum, v| {
        sum.checked_add(4 + 5 * v.len())
            .ok_or(Cff1Error::SubsetByteLimit)
    })?;
    let mut sort_work = 0;
    for values in selectors.values() {
        let bits = (usize::BITS - values.len().leading_zeros()) as usize;
        sort_work = add(sort_work, mul(mul(values.len(), bits + 1)?, 128)?)?;
    }
    charge(1, total, add(mul(total, 2)?, sort_work)?)?;
    let mut uvs = Vec::new();
    uvs.try_reserve_exact(total)
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    uvs.extend(14u16.to_be_bytes());
    uvs.extend(
        u32::try_from(total)
            .map_err(|_| Cff1Error::SubsetByteLimit)?
            .to_be_bytes(),
    );
    uvs.extend((records as u32).to_be_bytes());
    let mut offset = 10 + 11 * records;
    for (&selector, values) in &selectors {
        uvs.extend(&selector.to_be_bytes()[1..]);
        uvs.extend(0u32.to_be_bytes());
        uvs.extend((offset as u32).to_be_bytes());
        offset += 4 + 5 * values.len();
    }
    // Canonical subset UVS are explicit non-default mappings, including source
    // defaults. Every retained scalar pair resolves to exactly its dense GID.
    for values in selectors.values_mut() {
        values.sort_unstable_by_key(|p| p.0);
        uvs.extend((values.len() as u32).to_be_bytes());
        for &(scalar, gid) in values.iter() {
            uvs.extend(&scalar.to_be_bytes()[1..]);
            uvs.extend(gid.to_be_bytes());
        }
    }
    let base_subtable = &base[12..];
    let size = 20usize
        .checked_add(base_subtable.len())
        .and_then(|n| n.checked_add(uvs.len()))
        .ok_or(Cff1Error::SubsetByteLimit)?;
    charge(1, size, size)?;
    let mut out = Vec::new();
    out.try_reserve_exact(size)
        .map_err(|_| Cff1Error::SubsetByteLimit)?;
    out.extend([0, 0, 0, 2, 0, 0, 0, 5]);
    out.extend(
        u32::try_from(20 + base_subtable.len())
            .map_err(|_| Cff1Error::SubsetByteLimit)?
            .to_be_bytes(),
    );
    out.extend([0, 3, 0, 10]);
    out.extend(20u32.to_be_bytes());
    out.extend(base_subtable);
    out.extend(uvs);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
    fn cff_v2_subset_original_harano_selected_outlines_and_uvs() {
        let source: Arc<[u8]> = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap())
            .unwrap()
            .into();
        let hex = |b: &[u8]| b.iter().map(|v| format!("{v:02x}")).collect::<String>();
        assert_eq!(
            hex(&sha256(&source)),
            "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
        );
        let limits = M4EffectiveResourceLimits::new(
            typaxis_core::ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
                .unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap();
        let admission = admit_sfnt_cff1_v2(source, 0, &limits).unwrap();
        let mut selected: BTreeSet<_> = [151, 233].into_iter().map(OriginalGlyphId::new).collect();
        for c in "日本語明朝漢字、。「」A a".chars() {
            selected.insert(OriginalGlyphId::new(
                admission.cmap().glyph_for_sequence(c, None).unwrap(),
            ));
        }
        let mut fds = BTreeSet::new();
        for gid in 0..admission.glyph_count() {
            if fds.insert(admission.program().fd_for_gid(usize::from(gid)).unwrap()) {
                selected.insert(OriginalGlyphId::new(gid));
            }
        }
        let mut first_default = None;
        let mut first_nondefault = None;
        admission
            .cmap()
            .variation_sequences()
            .unwrap()
            .visit_pairs::<std::convert::Infallible>(|base, vs, coverage| {
                match coverage {
                    VariationCoverage::Default if first_default.is_none() => {
                        first_default = Some((base, vs))
                    }
                    VariationCoverage::NonDefault(gid)
                        if gid != 0 && first_nondefault.is_none() =>
                    {
                        first_nondefault = Some((base, vs))
                    }
                    _ => {}
                }
                Ok(())
            })
            .unwrap();
        for (base, vs) in [first_default.unwrap(), first_nondefault.unwrap()] {
            selected.insert(OriginalGlyphId::new(
                admission
                    .cmap()
                    .glyph_for_sequence(
                        char::from_u32(base).unwrap(),
                        Some(char::from_u32(vs).unwrap()),
                    )
                    .unwrap(),
            ));
        }
        let closure = Cff1SubsetSessionV2::close_instance_selection(
            &admission,
            FontFaceId::new(1),
            FontInstanceId::new(1),
            &selected,
            65534,
        )
        .unwrap();
        let mut session = Cff1SubsetSessionV2::from_admission(&admission);
        let subset = session.subset(&admission, closure.clone()).unwrap();
        assert_eq!(subset.bytes().len(), 5052);
        assert_eq!(
            hex(&subset.sha256()),
            "3b9ddbc2e0415a302c95f550113ddacfcc60d2f6fb219ad7f9296753f8823463"
        );
        budget_tests::verify(&session, &admission, &closure, &subset);
        let work = (session.operations_used(), session.outline_segments_used());
        let repeated = session.subset(&admission, closure).unwrap();
        assert_eq!(subset.bytes(), repeated.bytes());
        assert_eq!(subset.fingerprint(), repeated.fingerprint());
        assert_eq!(
            (session.operations_used(), session.outline_segments_used()),
            work
        );
        let font = read_fonts::FontRef::new(subset.bytes()).unwrap();
        let glyph_count = font.maxp().unwrap().num_glyphs();
        assert_eq!(
            usize::from(glyph_count),
            subset.closure().source_gids().len()
        );
        preflight_sfnt(
            subset.bytes(),
            *limits.extension().get(),
            &mut FontFailureContext::new(0),
        )
        .unwrap();
        let hmtx = font
            .data_for_tag(read_fonts::types::Tag::new(b"hmtx"))
            .unwrap();
        let cmap = font
            .data_for_tag(read_fonts::types::Tag::new(b"cmap"))
            .unwrap();
        let output_cmap = validate_cff_cmap_v2(cmap.as_bytes(), glyph_count).unwrap();
        for (&original, &dense) in subset.original_to_subset() {
            let expected = session
                .evaluated_glyph(&admission, original)
                .unwrap()
                .unwrap();
            assert_eq!(
                read_u16(
                    hmtx.as_bytes(),
                    usize::from(dense.get()) * 4,
                    Cff1Error::InvalidHmtx
                )
                .unwrap(),
                expected.advance()
            );
            assert_eq!(subset.original_widths()[&original], expected.advance());
        }
        admission
            .cmap()
            .variation_sequences()
            .unwrap()
            .visit_pairs::<std::convert::Infallible>(|base, vs, _| {
                let base = char::from_u32(base).unwrap();
                let vs = char::from_u32(vs).unwrap();
                let expected = admission
                    .cmap()
                    .glyph_for_sequence(base, Some(vs))
                    .and_then(|g| subset.original_to_subset().get(&OriginalGlyphId::new(g)))
                    .map(|g| g.get())
                    .filter(|g| *g != 0);
                assert_eq!(output_cmap.glyph_for_sequence(base, Some(vs)), expected);
                Ok(())
            })
            .unwrap();
        for (size, success) in [
            (subset.bytes().len() as u64, true),
            (subset.bytes().len() as u64 - 1, false),
        ] {
            let mut extension = *limits.extension().get();
            extension.max_font_subset_bytes = size;
            let bounded = M4EffectiveResourceLimits::new(limits.base().clone(), extension).unwrap();
            let bounded_admission =
                admit_sfnt_cff1_v2(Arc::from(admission.source()), 0, &bounded).unwrap();
            let closure = Cff1SubsetSessionV2::close_instance_selection(
                &bounded_admission,
                FontFaceId::new(1),
                FontInstanceId::new(1),
                &selected,
                65534,
            )
            .unwrap();
            let result = Cff1SubsetSessionV2::from_admission(&bounded_admission)
                .subset(&bounded_admission, closure);
            if success {
                assert_eq!(result.unwrap().bytes(), subset.bytes());
            } else {
                assert!(matches!(
                    result,
                    Err(CffSelectionFailureV2::Selection(Cff1Error::SubsetByteLimit))
                ));
            }
        }
        if let Ok(path) = std::env::var("TYPAXIS_HARANO_SUBSET_OUTPUT") {
            std::fs::write(&path, subset.bytes()).unwrap();
            let mut mapping = String::new();
            for (&original, &dense) in subset.original_to_subset() {
                mapping.push_str(&format!("{} {}\n", original.get(), dense.get()));
            }
            std::fs::write(format!("{path}.gids"), mapping).unwrap();
        }
    }
}
