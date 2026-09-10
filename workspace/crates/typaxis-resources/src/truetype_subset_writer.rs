//! Shared byte writer. Measure original closed glyphs before any output allocation.
use super::*;

pub(super) struct WrittenTrueTypeSubset {
    pub bytes: Vec<u8>,
    pub widths: BTreeMap<OriginalGlyphId, u16>,
    pub metrics: PdfFontMetrics,
}
type Charge<'a> = &'a mut dyn FnMut(usize, usize, usize) -> Result<(), ResourceError>;
fn add(a: usize, b: usize) -> Result<usize, ResourceError> {
    a.checked_add(b).ok_or(ResourceError::ResourceLimit)
}
fn mul(a: usize, b: usize) -> Result<usize, ResourceError> {
    a.checked_mul(b).ok_or(ResourceError::ResourceLimit)
}
fn padded(n: usize) -> Result<usize, ResourceError> {
    Ok(add(n, 3)? & !3)
}
fn buffer(capacity: usize) -> Result<Vec<u8>, ResourceError> {
    let mut out = Vec::new();
    out.try_reserve_exact(capacity)
        .map_err(|_| ResourceError::ResourceLimit)?;
    Ok(out)
}

pub(super) fn write(
    prepared: &PreparedTrueTypeSubset<'_>,
    instance: FontInstanceId,
    maximum_bytes: u64,
    charge: Charge<'_>,
) -> Result<WrittenTrueTypeSubset, ResourceError> {
    // A canonical name has six base-26 letters and the fixed suffix. Reject
    // invalid instance IDs without allocating the name or output tables.
    if instance.get() >= 26u32.pow(6) {
        return Err(ResourceError::InvalidFontPlan);
    }
    charge(0, 0, 128 * 16)?;
    let tables = &prepared.tables;
    let head = table_bytes(tables, *b"head")?;
    let hhea = table_bytes(tables, *b"hhea")?;
    let maxp = table_bytes(tables, *b"maxp")?;
    let hmtx = table_bytes(tables, *b"hmtx")?;
    let glyf = table_bytes(tables, *b"glyf")?;
    let os2 = tables.get(b"OS/2").map(|t| t.bytes);
    let source_post = tables.get(b"post").map(|t| t.bytes);
    let n = prepared.closure.len();
    let subset_count = u16::try_from(n).map_err(|_| ResourceError::ResourceLimit)?;
    let mut glyf_size = 0;
    for original in &prepared.closure {
        charge(0, 0, 1)?;
        let glyph = glyph_bytes(glyf, &prepared.locations, *original)?;
        glyf_size = add(glyf_size, padded(glyph.len())?)?;
        // Preflight every remapping and metric before reserving output. The
        // second walk writes into the destination using this immutable source;
        // it needs neither a cloned glyph nor a replacement-offset vector.
        visit_composite_components(glyph, |_, component| {
            charge(0, 0, 128)?;
            prepared
                .original_to_subset
                .get(&OriginalGlyphId::new(component))
                .ok_or(ResourceError::InvalidFontPlan)?;
            Ok(())
        })?;
        horizontal_metric(
            hmtx,
            prepared.glyph_count,
            prepared.number_of_h_metrics,
            usize::from(*original),
        )?;
    }
    u32::try_from(glyf_size).map_err(|_| ResourceError::ResourceLimit)?;
    let loca_size = mul(add(n, 1)?, 4)?;
    let hmtx_size = mul(n, 4)?;
    let name_size = 18 + (6 + "+Typaxis".len()) * 2;
    let lengths = [
        glyf_size,
        head.len(),
        hhea.len(),
        hmtx_size,
        loca_size,
        maxp.len(),
        32,
        name_size,
        os2.map_or(0, <[u8]>::len),
    ];
    let table_count = 8 + usize::from(os2.is_some());
    let mut payload = 0;
    let mut storage = 0;
    for length in lengths {
        payload = add(payload, padded(length)?)?;
        storage = add(storage, length)?;
    }
    let output_size = add(add(12, mul(table_count, 16)?)?, payload)?;
    if output_size as u64 > maximum_bytes || output_size > u32::MAX as usize {
        return Err(ResourceError::ResourceLimit);
    }
    // Exact byte buffers plus bounded tree nodes and table headers. Temporary
    // canonical-name storage is included. No reservation is refunded on error.
    let bytes = add(
        add(add(storage, output_size)?, add(512, mul(n, 128)?)?)?,
        add(
            mul(table_count, std::mem::size_of::<SfntRewriteTable>())?,
            14,
        )?,
    )?;
    let records = add(mul(n, 3)?, add(table_count, 5)?)?;
    // Copying, zeroing, both checksum passes, map insert/lookup and the second
    // composite walk are bounded by actual output bytes and closed glyphs.
    let work = add(
        add(mul(output_size, 4)?, mul(n, 128)?)?,
        mul(table_count, 128)?,
    )?;
    charge(records, bytes, work)?;
    let metrics = pdf_metrics(head, hhea, os2, source_post)?;
    let mut new_glyf = buffer(glyf_size)?;
    let mut new_loca = buffer(loca_size)?;
    let mut new_hmtx = buffer(hmtx_size)?;
    let mut widths = BTreeMap::new();
    for original in &prepared.closure {
        let start = new_glyf.len();
        new_loca.extend_from_slice(&(start as u32).to_be_bytes());
        let source = glyph_bytes(glyf, &prepared.locations, *original)?;
        new_glyf.extend_from_slice(source);
        visit_composite_components(source, |offset, component| {
            charge(0, 0, 128)?;
            let subset = prepared
                .original_to_subset
                .get(&OriginalGlyphId::new(component))
                .ok_or(ResourceError::InvalidFontPlan)?;
            new_glyf[start + offset..start + offset + 2]
                .copy_from_slice(&subset.get().to_be_bytes());
            Ok(())
        })?;
        new_glyf.resize(padded(new_glyf.len())?, 0);
        let (advance, bearing) = horizontal_metric(
            hmtx,
            prepared.glyph_count,
            prepared.number_of_h_metrics,
            usize::from(*original),
        )?;
        widths.insert(OriginalGlyphId::new(*original), advance);
        new_hmtx.extend_from_slice(&advance.to_be_bytes());
        new_hmtx.extend_from_slice(&bearing.to_be_bytes());
    }
    new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
    let mut new_head = head.to_vec();
    new_head[8..12].fill(0);
    new_head[50..52].copy_from_slice(&1i16.to_be_bytes());
    let mut new_hhea = hhea.to_vec();
    new_hhea[34..36].copy_from_slice(&subset_count.to_be_bytes());
    let mut new_maxp = maxp.to_vec();
    new_maxp[4..6].copy_from_slice(&subset_count.to_be_bytes());
    let mut post = vec![0; 32];
    post[..4].copy_from_slice(&0x0003_0000u32.to_be_bytes());
    if let Some(source) = source_post.filter(|p| p.len() >= 16) {
        post[4..16].copy_from_slice(&source[4..16]);
    }
    let mut output_tables = Vec::new();
    output_tables
        .try_reserve_exact(table_count)
        .map_err(|_| ResourceError::ResourceLimit)?;
    for (tag, bytes) in [
        (*b"glyf", new_glyf),
        (*b"head", new_head),
        (*b"hhea", new_hhea),
        (*b"hmtx", new_hmtx),
        (*b"loca", new_loca),
        (*b"maxp", new_maxp),
        (*b"post", post),
    ] {
        output_tables.push(SfntRewriteTable { tag, bytes });
    }
    if let Some(source) = os2 {
        output_tables.push(SfntRewriteTable {
            tag: *b"OS/2",
            bytes: source.to_vec(),
        });
    }
    let name = canonical_subset_name_table(instance)?;
    if name.len() != name_size {
        return Err(ResourceError::InvalidFontPlan);
    }
    output_tables.push(SfntRewriteTable {
        tag: *b"name",
        bytes: name,
    });
    let bytes = rebuild_sfnt(output_tables)?;
    if bytes.len() != output_size {
        return Err(ResourceError::InvalidFontPlan);
    }
    Ok(WrittenTrueTypeSubset {
        bytes,
        widths,
        metrics,
    })
}

#[cfg(test)]
pub(super) fn assert_budgeted_writer(prepared: &PreparedTrueTypeSubset<'_>, legacy: &[u8]) {
    use read_fonts::TableProvider;
    let instance = FontInstanceId::new(37);
    let expected = rewrite_subset_postscript_name(legacy, instance).unwrap();
    let run = |maximum: (usize, usize, usize), byte_limit: u64| {
        let mut spent = (0usize, 0usize, 0usize);
        let result = write(
            prepared,
            instance,
            byte_limit,
            &mut |records, bytes, work| {
                let records = spent
                    .0
                    .checked_add(records)
                    .filter(|n| *n <= maximum.0)
                    .ok_or(ResourceError::ResourceLimit)?;
                let bytes = spent
                    .1
                    .checked_add(bytes)
                    .filter(|n| *n <= maximum.1)
                    .ok_or(ResourceError::ResourceLimit)?;
                spent.0 = records;
                spent.1 = bytes;
                for _ in 0..work {
                    spent.2 = spent
                        .2
                        .checked_add(1)
                        .filter(|n| *n <= maximum.2)
                        .ok_or(ResourceError::ResourceLimit)?;
                }
                Ok(())
            },
        );
        (result, spent)
    };
    let (actual, spent) = run((usize::MAX, usize::MAX, usize::MAX), expected.len() as u64);
    let actual = actual.unwrap();
    assert_eq!(actual.bytes, expected);
    assert_eq!(
        extract_subset_postscript_name(&actual.bytes, instance).unwrap(),
        expected_subset_postscript_name(instance).unwrap()
    );
    assert!(run(spent, expected.len() as u64).0.is_ok());
    for maximum in [
        (spent.0 - 1, spent.1, spent.2),
        (spent.0, spent.1 - 1, spent.2),
        (spent.0, spent.1, spent.2 - 1),
    ] {
        assert!(matches!(
            run(maximum, expected.len() as u64).0,
            Err(ResourceError::ResourceLimit)
        ));
    }
    let (_, failed) = run((spent.0, spent.1, spent.2 - 1), expected.len() as u64);
    assert_eq!(failed, (spent.0, spent.1, spent.2 - 1));
    let (rejected, before_output) = run(spent, expected.len() as u64 - 1);
    assert!(matches!(rejected, Err(ResourceError::ResourceLimit)));
    assert_eq!((before_output.0, before_output.1), (0, 0));
    let mut called = false;
    assert!(matches!(
        write(
            prepared,
            FontInstanceId::new(u32::MAX),
            u64::MAX,
            &mut |_, _, _| {
                called = true;
                Ok(())
            }
        ),
        Err(ResourceError::InvalidFontPlan)
    ));
    assert!(!called);
    // Independent parser follows loca/glyf and checks remapped compound data.
    let parsed = read_fonts::FontRef::new(&actual.bytes).unwrap();
    assert_eq!(
        parsed.maxp().unwrap().num_glyphs() as usize,
        prepared.closure.len()
    );
    parsed.head().unwrap();
    parsed.hhea().unwrap();
    parsed.hmtx().unwrap();
    parsed.loca(None).unwrap();
    parsed.glyf().unwrap();
    let tables = parse_sfnt_table_map(&actual.bytes, 0).unwrap();
    let glyf = table_bytes(&tables, *b"glyf").unwrap();
    let locations = parse_loca(
        table_bytes(&tables, *b"loca").unwrap(),
        prepared.closure.len(),
        1,
        glyf.len(),
    )
    .unwrap();
    assert_eq!(
        composite_components(glyph_bytes(glyf, &locations, 2).unwrap()).unwrap(),
        [1]
    );
}
