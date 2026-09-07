use super::*;
use std::{collections::BTreeSet, sync::Arc};
use typaxis_core::{M4EffectiveResourceLimits, M4ResourceLimits, ResourceLimits, TextBufferId};
use typaxis_font::{admit_sfnt_cff1_v2, Cff1SubsetSessionV2};
fn input(text: &str) -> Cff1ShapeInputV2<'_> {
    Cff1ShapeInputV2 {
        run_id: GlyphRunId::new(1),
        font: FontInstanceId::new(1),
        source: ShapeSourceSpan::Parsed(
            TextSpan::new(
                TextBufferId::new(29),
                Utf8ByteOffset::new(31),
                Utf8ByteOffset::new(31 + text.len() as u32),
            )
            .unwrap(),
        ),
        utf8: text,
        font_size: PositiveLength::new(Length::from_raw(11 * 65536).unwrap()).unwrap(),
        bidi_level: BidiLevel::new(0).unwrap(),
        script: OpenTypeTag::new(*b"Hani").unwrap(),
        language: Some("ja"),
        pre_context: None,
        post_context: None,
    }
}
fn original() -> Cff1AdmissionV2 {
    let bytes: Arc<[u8]> = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap())
        .unwrap()
        .into();
    let hex = |b: &[u8]| b.iter().map(|v| format!("{v:02x}")).collect::<String>();
    assert_eq!(
        hex(&sha256(&bytes)),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let limits = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap();
    admit_sfnt_cff1_v2(bytes, 0, &limits).unwrap()
}
fn original_pairs(admission: &Cff1AdmissionV2) -> Vec<(char, char)> {
    let font = harfrust::FontRef::from_index(admission.source(), 0).unwrap();
    let raw = font
        .data_for_tag(read_fonts::types::Tag::new(b"cmap"))
        .unwrap();
    let b = raw.as_bytes();
    let u16at = |p| u16::from_be_bytes(b[p..p + 2].try_into().unwrap());
    let u32at = |p| u32::from_be_bytes(b[p..p + 4].try_into().unwrap());
    let u24at = |p| u32::from_be_bytes([0, b[p], b[p + 1], b[p + 2]]);
    let mut pairs = BTreeSet::new();
    for i in 0..usize::from(u16at(2)) {
        let r = 4 + i * 8;
        if u16at(r) != 0 || u16at(r + 2) != 5 {
            continue;
        }
        let base = u32at(r + 4) as usize;
        for i in 0..u32at(base + 6) as usize {
            let r = base + 10 + i * 11;
            let selector = char::from_u32(u24at(r)).unwrap();
            for (field, stride) in [(r + 3, 4), (r + 7, 5)] {
                let off = u32at(field) as usize;
                if off == 0 {
                    continue;
                }
                let off = base + off;
                for i in 0..u32at(off) as usize {
                    let r = off + 4 + i * stride;
                    let scalar = u24at(r);
                    let extra = if stride == 4 { u32::from(b[r + 3]) } else { 0 };
                    for scalar in scalar..=scalar + extra {
                        pairs.insert((char::from_u32(scalar).unwrap(), selector));
                    }
                }
            }
        }
    }
    pairs.into_iter().collect()
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn cff_v2_shape_original_all_variations_preserve_gids_and_utf8_clusters() {
    let admission = original();
    let pairs = original_pairs(&admission);
    assert_eq!(pairs.len(), 14780);
    let distinct: BTreeSet<_> = pairs
        .iter()
        .map(|&(base, vs)| admission.cmap().glyph_for_sequence(base, Some(vs)).unwrap())
        .collect();
    assert!(
        distinct.len() < pairs.len(),
        "fixture must cover multiple sequences sharing one GID"
    );
    let space = admission.cmap().glyph_for_sequence(' ', None).unwrap();
    let mut selected = BTreeSet::new();
    for chunk in pairs.chunks(200) {
        let mut text = String::new();
        let mut expected = Vec::new();
        for &(base, vs) in chunk {
            let start = text.len() as u32;
            text.push(base);
            text.push(vs);
            expected.push((
                start,
                text.len() as u32,
                admission.cmap().glyph_for_sequence(base, Some(vs)).unwrap(),
            ));
            let start = text.len() as u32;
            text.push(' ');
            expected.push((start, text.len() as u32, space));
        }
        let shaped = shape_cff1_run_v2(&admission, input(&text)).unwrap();
        assert_eq!(shaped.utf8(), text);
        assert_eq!(shaped.admission().fingerprint(), admission.fingerprint());
        let run = shaped.glyph_run();
        assert_eq!(run.clusters.len(), expected.len());
        assert_eq!(run.glyphs.len(), expected.len());
        for (index, (cluster, (start, end, gid))) in run.clusters.iter().zip(expected).enumerate() {
            assert_eq!(
                shaped.cluster_text(index),
                Some(&text[start as usize..end as usize])
            );
            assert_eq!(source_range(cluster.source_span), (31 + start, 31 + end));
            assert_eq!(cluster.glyph_end, cluster.glyph_start + 1);
            let got = run.glyphs[cluster.glyph_start as usize].original_gid;
            assert_eq!(
                got.get(),
                gid,
                "pair {:?}",
                &text[start as usize..end as usize]
            );
            selected.insert(got);
        }
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
    let subset = session.subset(&admission, closure).unwrap();
    assert_eq!(subset.original_to_subset().len(), 14674);
    assert_eq!(subset.bytes().len(), 6563684);
    let hex = |b: &[u8]| b.iter().map(|v| format!("{v:02x}")).collect::<String>();
    assert_eq!(
        hex(&subset.sha256()),
        "aad51459429ea109f404e42b29e5b38e350430d9bdebbefbaff10c328b1d481c"
    );
    let font = harfrust::FontRef::from_index(subset.bytes(), 0).unwrap();
    let raw = font
        .data_for_tag(read_fonts::types::Tag::new(b"cmap"))
        .unwrap();
    let cmap =
        typaxis_font::validate_cff_cmap_v2(raw.as_bytes(), font.maxp().unwrap().num_glyphs())
            .unwrap();
    for (base, vs) in pairs {
        let source =
            OriginalGlyphId::new(admission.cmap().glyph_for_sequence(base, Some(vs)).unwrap());
        assert_eq!(
            cmap.glyph_for_sequence(base, Some(vs)),
            Some(subset.original_to_subset()[&source].get())
        );
    }
    if let Ok(path) = std::env::var("TYPAXIS_HARANO_IVS_SUBSET_OUTPUT") {
        std::fs::write(&path, subset.bytes()).unwrap();
        let mut mapping = String::new();
        for (&source, &dense) in subset.original_to_subset() {
            mapping.push_str(&format!("{} {}\n", source.get(), dense.get()));
        }
        std::fs::write(format!("{path}.gids"), mapping).unwrap();
    }
    eprintln!(
        "IVS glyphs={} subset_bytes={} operations={} segments={}",
        subset.original_to_subset().len(),
        subset.bytes().len(),
        session.operations_used(),
        session.outline_segments_used()
    );
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn cff_v2_shape_original_rejects_missing_split_and_isolated_selectors() {
    let admission = original();
    assert!(matches!(
        shape_cff1_run_v2(&admission, input("日A\u{e01ef}")),
        Err(Cff1ShapeErrorV2::MissingCoverage {
            byte_start: 3,
            byte_end: 8
        })
    ));
    for text in [
        "\u{e0100}",
        "A\u{e01ef}",
        "日\u{e01ef}",
        "日\u{e0100}\u{e0100}",
    ] {
        assert!(
            matches!(
                shape_cff1_run_v2(&admission, input(text)),
                Err(Cff1ShapeErrorV2::MissingCoverage { .. })
            ),
            "{text:?}"
        );
    }
    let mut request = input("日");
    request.post_context = Some("\u{e0100}");
    assert!(matches!(
        shape_cff1_run_v2(&admission, request),
        Err(Cff1ShapeErrorV2::SplitVariationSequence)
    ));
    let text = "日".repeat(23000);
    assert!(matches!(
        shape_cff1_run_v2(&admission, input(&text)),
        Err(Cff1ShapeErrorV2::ContextLimit { .. })
    ));
    let mut request = input("日");
    request.source = ShapeSourceSpan::Parsed(
        TextSpan::new(
            TextBufferId::new(29),
            Utf8ByteOffset::new(31),
            Utf8ByteOffset::new(35),
        )
        .unwrap(),
    );
    assert!(matches!(
        shape_cff1_run_v2(&admission, request),
        Err(Cff1ShapeErrorV2::Backend(
            LinkedShaperError::SourceLengthMismatch
        ))
    ));
}
