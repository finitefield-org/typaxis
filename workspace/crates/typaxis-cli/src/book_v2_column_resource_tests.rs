use super::*;
use crate::book_v2_resources::with_budgeted_book_v2_column_resources as resource_drive;
use crate::book_v2_resources::BookV2PdfConvergenceObservation;
use std::collections::BTreeMap;
use typaxis_display_list::book_v2::{BookV2FontUseSource, BookV2FontUseText};
use typaxis_font::OriginalGlyphId;
use typaxis_resources::book_v2::{
    BookV2ColumnCidPlans, BookV2ColumnFontSelectionBuilder, BookV2ColumnRasterPrograms,
    BookV2FontClosureKind, BookV2FontSelectionError,
};

#[path = "book_v2_column_resource_continuation_tests.rs"]
mod continuation_tests;
#[path = "book_v2_column_paint_tests.rs"]
mod paint_tests;
#[path = "book_v2_column_resource_probes.rs"]
mod resource_probes;

const MAXIMUM: u64 = 1_000_000_000;

#[derive(Clone, Debug)]
struct ExpectedUse {
    paint: usize,
    slot: usize,
    instance: u32,
    source: BookV2FontUseSource,
    text: String,
    glyphs: Vec<u16>,
    size: i64,
    face: u32,
    sha256: [u8; 32],
    face_index: u32,
}

// This expectation traverses the actual component sources. It never calls the
// resource font-slot, font-use or selection helpers to obtain expected uses.
fn expected_fonts(
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
) -> Vec<ExpectedUse> {
    let mut uses = Vec::new();
    for (paint, kind) in display.paints().iter().copied().enumerate() {
        let mut text = |slot,
                        instance,
                        span,
                        exact: &str,
                        glyphs: &[typaxis_display_list::ProductionBodyGlyph],
                        font: &typaxis_shaping::ProductionBodyFont| {
            uses.push(ExpectedUse {
                paint,
                slot,
                instance,
                source: BookV2FontUseSource::Text(span),
                text: exact.into(),
                glyphs: glyphs.iter().map(|g| g.original_gid().get()).collect(),
                size: font.size().get().raw(),
                face: font.face_id().get(),
                sha256: font.content_hash(),
                face_index: font.face_index(),
            });
        };
        match kind {
            Paint::PageRegionText { .. } => {
                panic!("running region outside the legacy Column fixture")
            }
            Paint::Text(i) => {
                let d = &display.text().draws()[i];
                text(
                    0,
                    d.cluster().run().glyph_run().font.get(),
                    d.text_span(),
                    d.exact_text(),
                    d.glyphs(),
                    d.font(),
                );
            }
            Paint::Marker(i) => {
                let d = &display.markers().draws()[i];
                for (slot, c) in d.clusters().iter().enumerate() {
                    text(
                        slot,
                        d.source().glyph_run().font.get(),
                        c.text_span(),
                        c.exact_text(),
                        c.glyphs(),
                        d.source().font(),
                    );
                }
            }
            Paint::EquationNumber(i) => {
                let d = &display.numbers().draws()[i];
                for (slot, c) in d.clusters().iter().enumerate() {
                    text(
                        slot,
                        d.shape().font_instance_id().get(),
                        c.text_span(),
                        c.exact_text(),
                        c.glyphs(),
                        d.font(),
                    );
                }
            }
            Paint::Math(i) => {
                let d = &display.math().draws()[i];
                if let BookV2MathPaint::Native(n) = d.paint() {
                    let BookV2BodyMathSource::Native(receipt) = d.terminal().source() else {
                        panic!("native source receipt changed kind")
                    };
                    for (slot, p) in n.paints().iter().enumerate() {
                        if let ProductionNativeMathPaint::Glyph {
                            original_gid,
                            unicode,
                            logical_ordinal,
                            font_size,
                            ..
                        } = p
                        {
                            uses.push(ExpectedUse {
                                paint,
                                slot,
                                instance: receipt.font_instance_id().get(),
                                source: BookV2FontUseSource::NativeMath {
                                    owner: receipt.node_id(),
                                    receipt: receipt.fingerprint(),
                                    computation: receipt.computation().fingerprint(),
                                    paint_index: slot as u32,
                                    logical_ordinal: *logical_ordinal,
                                },
                                text: unicode.to_string(),
                                glyphs: vec![original_gid.get()],
                                size: font_size.get().raw(),
                                face: receipt.font_face_id().get(),
                                sha256: receipt.font_sha256(),
                                face_index: receipt.face_index(),
                            });
                        }
                    }
                }
            }
            Paint::FootnoteSeparator(_) | Paint::Image(_) => (),
        }
    }
    uses
}

fn u16be(bytes: &[u8], p: usize) -> u16 {
    u16::from_be_bytes(bytes[p..p + 2].try_into().unwrap())
}
fn u32be(bytes: &[u8], p: usize) -> u32 {
    u32::from_be_bytes(bytes[p..p + 4].try_into().unwrap())
}
fn sfnt_table<'a>(bytes: &'a [u8], tag: &[u8; 4]) -> &'a [u8] {
    let n = u16be(bytes, 4) as usize;
    for i in 0..n {
        let p = 12 + i * 16;
        if &bytes[p..p + 4] == tag {
            let offset = u32be(bytes, p + 8) as usize;
            let length = u32be(bytes, p + 12) as usize;
            return &bytes[offset..offset + length];
        }
    }
    panic!("original sfnt missing {tag:?}")
}
fn original_advance(bytes: &[u8], gid: u16) -> u16 {
    let n = u16be(sfnt_table(bytes, b"hhea"), 34);
    u16be(sfnt_table(bytes, b"hmtx"), usize::from(gid.min(n - 1)) * 4)
}
fn original_closure(bytes: &[u8], requested: &BTreeSet<u16>) -> BTreeSet<u16> {
    let mut result = BTreeSet::from([0]);
    let mut stack = requested.iter().copied().collect::<Vec<_>>();
    let long = u16be(sfnt_table(bytes, b"head"), 50) != 0;
    let loca = sfnt_table(bytes, b"loca");
    let glyf = sfnt_table(bytes, b"glyf");
    let offset = |gid: u16| {
        if long {
            u32be(loca, usize::from(gid) * 4) as usize
        } else {
            usize::from(u16be(loca, usize::from(gid) * 2)) * 2
        }
    };
    while let Some(gid) = stack.pop() {
        if !result.insert(gid) {
            continue;
        }
        let (start, end) = (offset(gid), offset(gid + 1));
        if start == end || (u16be(glyf, start) as i16) >= 0 {
            continue;
        }
        let mut p = start + 10;
        loop {
            let flags = u16be(glyf, p);
            stack.push(u16be(glyf, p + 2));
            p += 4 + if flags & 1 != 0 { 4 } else { 2 };
            p += if flags & 128 != 0 {
                8
            } else if flags & 64 != 0 {
                4
            } else if flags & 8 != 0 {
                2
            } else {
                0
            };
            assert!(p <= end);
            if flags & 32 == 0 {
                break;
            }
        }
    }
    result
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResourceFacts {
    fingerprints: [[u8; 32]; 6],
    stages: [[u64; 3]; 8],
    font_uses: usize,
    images: usize,
    image_uses: usize,
    glyphs: BTreeSet<u16>,
}
fn inspect_resources(
    cids: &BookV2ColumnCidPlans<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    rasters: &BookV2ColumnRasterPrograms<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    constructors: [[u64; 3]; 2],
) -> ResourceFacts {
    let programs = cids.source();
    let closures = programs.source();
    let selection = closures.selection();
    let display = selection.display();
    let images = rasters.source();
    assert!(std::ptr::eq(display, images.display()));
    let expected = expected_fonts(display);
    assert_eq!(selection.uses().len(), expected.len());
    let mut unions = BTreeMap::<u32, BTreeSet<u16>>::new();
    let mut observations = BTreeMap::<(u32, u16), BTreeSet<char>>::new();
    for (i, (selected, e)) in selection.uses().iter().zip(&expected).enumerate() {
        let actual = selected.usage();
        assert_eq!(
            (selected.paint_index(), actual.slot(), actual.paint()),
            (e.paint, e.slot, display.paints()[e.paint])
        );
        assert_eq!(actual.instance().font_instance_id().get(), e.instance);
        assert_eq!(
            (
                actual.instance().font().font_face_id().get(),
                actual.instance().font().content_hash(),
                actual.instance().font().face_index()
            ),
            (e.face, e.sha256, e.face_index)
        );
        assert!(std::ptr::eq(actual.instance().ledger(), display.admitted()));
        assert_eq!(actual.source(), e.source);
        let exact = match actual.text() {
            BookV2FontUseText::Text(s) => s.to_owned(),
            BookV2FontUseText::Scalar(c) => c.to_string(),
        };
        assert_eq!(exact, e.text);
        assert_eq!(actual.size().get().raw(), e.size);
        assert_eq!(
            (0..actual.glyphs().len())
                .map(|g| actual.glyphs().get(g).unwrap().get())
                .collect::<Vec<_>>(),
            e.glyphs
        );
        unions
            .entry(e.instance)
            .or_default()
            .extend(e.glyphs.iter().copied());
        let chars = e.text.chars().collect::<Vec<_>>();
        if e.glyphs.len() == 1 && chars.len() == 1 {
            observations
                .entry((e.instance, e.glyphs[0]))
                .or_default()
                .insert(chars[0]);
        }
        assert_eq!(cids.usage_index(e.paint, e.slot), Some(i));
        assert!(std::ptr::eq(cids.uses()[i].source(), selected));
    }
    assert_eq!(selection.fonts().len(), unions.len());
    assert_eq!(closures.fonts().len(), unions.len());
    assert_eq!(programs.fonts().len(), unions.len());
    assert_eq!(cids.fonts().len(), unions.len());
    for (i, (instance, requested)) in unions.iter().enumerate() {
        let font = &selection.fonts()[i];
        assert_eq!(font.instance().font_instance_id().get(), *instance);
        assert_eq!(
            selection
                .glyphs(i)
                .unwrap()
                .map(|g| g.get())
                .collect::<BTreeSet<_>>(),
            *requested
        );
        let closed = &closures.fonts()[i];
        let program = &programs.fonts()[i];
        assert!(std::ptr::eq(closed.source(), font));
        assert!(std::ptr::eq(program.source(), closed));
        assert!(std::ptr::eq(cids.fonts()[i].source(), program));
        let original = font.instance().font();
        let bytes = original.bytes();
        assert_eq!(typaxis_core::sha256(bytes), original.content_hash());
        let expected_closure = match original {
            typaxis_resources::AdmittedProductionFontV3::TrueType(_) => {
                assert_eq!(closed.kind(), BookV2FontClosureKind::TrueType);
                original_closure(bytes, requested)
            }
            typaxis_resources::AdmittedProductionFontV3::Cff1V2(_) => {
                assert_eq!(closed.kind(), BookV2FontClosureKind::Cff1V2);
                assert_eq!(&program.bytes()[..4], b"OTTO");
                assert_eq!(sfnt_table(program.bytes(), b"CFF ")[0], 1);
                assert_eq!(u32be(sfnt_table(program.bytes(), b"maxp"), 0), 0x0000_5000);
                // Type2 subroutine dependencies do not add glyph slots. CFF
                // closure retains the requested GIDs and the .notdef slot.
                let mut glyphs = requested.clone();
                glyphs.insert(0);
                glyphs
            }
        };
        assert_eq!(program.kind(), closed.kind());
        assert_eq!(
            closed.glyphs().map(|g| g.get()).collect::<BTreeSet<_>>(),
            expected_closure
        );
        for (dense, gid) in expected_closure.iter().enumerate() {
            assert_eq!(
                closed.subset_gid(OriginalGlyphId::new(*gid)).unwrap().get() as usize,
                dense
            );
        }
        assert_eq!(
            u16be(sfnt_table(program.bytes(), b"maxp"), 4) as usize,
            expected_closure.len()
        );
        assert_eq!(typaxis_core::sha256(program.bytes()), program.sha256());
        let bindings = cids.bindings(i).unwrap();
        assert_eq!(bindings.len(), requested.len());
        for (ordinal, (gid, binding)) in requested.iter().zip(bindings).enumerate() {
            assert_eq!(binding.original_gid().get(), *gid);
            assert_eq!(binding.cid().get() as usize, ordinal + 1);
            let subset = closed.subset_gid(OriginalGlyphId::new(*gid)).unwrap();
            assert_eq!(binding.subset_gid(), subset);
            assert_eq!(program.subset_gid(OriginalGlyphId::new(*gid)), Some(subset));
            let advance = original_advance(bytes, *gid);
            assert_eq!(program.advance(OriginalGlyphId::new(*gid)), Some(advance));
            assert_eq!(original_advance(program.bytes(), subset.get()), advance);
            let upm = u64::from(u16be(sfnt_table(bytes, b"head"), 18));
            assert_eq!(
                binding.width_1000(),
                ((u64::from(advance) * 1000 + upm / 2) / upm) as u32
            );
            let unicode = observations
                .get(&(*instance, *gid))
                .filter(|s| s.len() == 1)
                .and_then(|s| s.first())
                .copied();
            assert_eq!(binding.unicode(), unicode);
        }
    }
    for (i, e) in expected.iter().enumerate() {
        let usage = &cids.uses()[i];
        let bindings = cids.bindings(usage.font_index()).unwrap();
        let mapped = e
            .glyphs
            .iter()
            .map(|gid| {
                bindings
                    .iter()
                    .find(|b| b.original_gid().get() == *gid)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            cids.cids(i)
                .unwrap()
                .iter()
                .map(|c| c.get())
                .collect::<Vec<_>>(),
            mapped.iter().map(|b| b.cid().get()).collect::<Vec<_>>()
        );
        let unicode = mapped
            .iter()
            .map(|b| b.unicode())
            .collect::<Option<String>>();
        let actual_text = unicode.as_deref() != Some(e.text.as_str());
        assert_eq!(usage.requires_actual_text(), actual_text);
        assert_eq!(
            usage.actual_text().map(|t| match t {
                BookV2FontUseText::Text(s) => s.to_owned(),
                BookV2FontUseText::Scalar(c) => c.to_string(),
            }),
            actual_text.then(|| e.text.clone())
        );
    }
    assert_eq!(cids.usage_index(usize::MAX, 0), None);
    assert!(display.font_use(usize::MAX, 0).unwrap().is_none());
    assert!(display.image_use(usize::MAX).unwrap().is_none());
    for (paint, p) in display.paints().iter().copied().enumerate() {
        if let Paint::Math(i) = p {
            if let BookV2MathPaint::Native(n) = display.math().draws()[i].paint() {
                for (slot, p) in n.paints().iter().enumerate() {
                    if !matches!(p, ProductionNativeMathPaint::Glyph { .. }) {
                        assert_eq!(cids.usage_index(paint, slot), None);
                        assert!(display.font_use(paint, slot).unwrap().is_none());
                    }
                }
            }
        }
    }
    let mut expected_images = Vec::new();
    for (paint, kind) in display.paints().iter().copied().enumerate() {
        match kind {
            Paint::Image(i) => {
                let d = &display.images().draws()[i];
                expected_images.push((
                    paint,
                    kind,
                    d.image(),
                    d.source().owner(),
                    d.fragment().fragment().page_index(),
                    d.paint(),
                ));
            }
            Paint::Math(i) => {
                let d = &display.math().draws()[i];
                if let BookV2MathPaint::Vector(v) = d.paint() {
                    let BookV2BodyMathSource::Vector(b) = d.terminal().source() else {
                        panic!("vector source receipt changed kind")
                    };
                    expected_images.push((
                        paint,
                        kind,
                        display.admitted().image(b.resource().image_id()).unwrap(),
                        b.node_id(),
                        d.terminal().page_index(),
                        BookV2ImagePaint::Vector {
                            content_key: v.content_key(),
                            viewport: v.viewport(),
                            scale_raw: v.scale_raw(),
                            matrix: v.matrix(),
                            color: v.color(),
                        },
                    ));
                }
            }
            _ => (),
        }
    }
    assert_eq!(images.uses().len(), expected_images.len());
    let expected_keys = expected_images
        .iter()
        .map(|e| resource_probes::image_key(e.2))
        .collect::<BTreeSet<_>>();
    assert_eq!(images.images().len(), expected_keys.len());
    for (actual, expected) in images.uses().iter().zip(&expected_images) {
        assert_eq!(
            (actual.paint_index(), actual.usage().paint()),
            (expected.0, expected.1)
        );
        assert!(std::ptr::eq(actual.usage().image(), expected.2));
        assert_eq!(
            (
                actual.usage().owner(),
                actual.usage().page_index(),
                actual.usage().geometry()
            ),
            (expected.3, expected.4, expected.5)
        );
        assert_eq!(
            resource_probes::image_key(images.images()[actual.resource_index()].image()),
            resource_probes::image_key(expected.2)
        );
        assert!(std::ptr::eq(images.for_paint(expected.0).unwrap(), actual));
    }
    for (i, image) in images.images().iter().enumerate() {
        let admitted = image.image();
        assert!(std::ptr::eq(
            display.admitted().image(admitted.image_id()).unwrap(),
            admitted
        ));
        assert_eq!(
            typaxis_core::sha256(admitted.bytes()),
            admitted.content_hash()
        );
        match admitted.media_kind() {
            typaxis_resources::AdmittedImageMediaKind::SafeVector
            | typaxis_resources::AdmittedImageMediaKind::SafeVector2 => {
                assert!(rasters.program(i).is_none())
            }
            _ => {
                let p = rasters.program(i).unwrap();
                assert!(std::ptr::eq(p.source(), image));
                assert_eq!(typaxis_core::sha256(p.bytes()), p.sha256());
                assert_eq!(
                    (p.width(), p.height()),
                    (admitted.width().get(), admitted.height().get())
                );
                if let Some(j) = p.jpeg() {
                    assert!(std::ptr::eq(j, admitted.jpeg_attestation().unwrap()));
                    assert_eq!(p.bytes(), j.normalized_bytes());
                    assert_eq!(j.source_sha256(), admitted.content_hash());
                    assert_eq!(p.sha256(), j.normalized_sha256());
                } else {
                    assert_eq!(p.encoding(), typaxis_resources::ImageEncoding::Flate);
                }
            }
        }
    }
    ResourceFacts {
        fingerprints: [
            selection.fingerprint(),
            closures.fingerprint(),
            programs.fingerprint(),
            cids.fingerprint(),
            images.fingerprint(),
            rasters.fingerprint(),
        ],
        stages: [
            constructors[0],
            [
                selection.record_charge(),
                selection.spool_charge(),
                selection.work_steps(),
            ],
            [
                closures.record_charge(),
                closures.spool_charge(),
                closures.work_steps(),
            ],
            [
                programs.record_charge(),
                programs.spool_charge(),
                programs.work_steps(),
            ],
            [cids.record_charge(), cids.spool_charge(), cids.work_steps()],
            constructors[1],
            [
                images.record_charge(),
                images.spool_charge(),
                images.work_steps(),
            ],
            [
                rasters.record_charge(),
                rasters.spool_charge(),
                rasters.work_steps(),
            ],
        ],
        font_uses: expected.len(),
        images: images.images().len(),
        image_uses: expected_images.len(),
        glyphs: unions.values().flatten().copied().collect(),
    }
}

fn chain(
    case: &str,
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    caps: &M4EffectiveResourceLimits,
    observed: Option<BookV2PdfConvergenceObservation>,
) -> ResourceFacts {
    inspect(display);
    let mut builder =
        BookV2ColumnFontSelectionBuilder::new(display, caps, MAXIMUM, 0, 0, 0).unwrap();
    let before_fonts = [
        builder.record_charge(),
        builder.spool_charge(),
        builder.work_steps(),
    ];
    let fonts = builder.build().unwrap();
    let closures = builder.prepare_font_closures(&fonts).unwrap();
    let programs = builder.write_font_programs(&closures).unwrap();
    let cids = builder.plan_cids(&programs).unwrap();
    let prefix = [
        builder.record_charge(),
        builder.spool_charge(),
        builder.work_steps(),
    ];
    let mut image_builder = builder.next_stage_counted(&mut 0, &mut 0, &mut 0).unwrap();
    let before_images = [
        image_builder.record_charge(),
        image_builder.spool_charge(),
        image_builder.work_steps(),
    ];
    let images = image_builder.select_images().unwrap();
    assert_eq!(images.prior_charges(), prefix);
    let rasters = image_builder.write_raster_programs(&images).unwrap();
    let facts = inspect_resources(&cids, &rasters, [before_fonts, before_images]);
    resource_probes::emit(
        case,
        &cids,
        &rasters,
        caps,
        observed,
        if observed.is_some() {
            "source_callback"
        } else {
            "retained_source_cut"
        },
        MAXIMUM,
        [before_fonts, before_images],
        image_builder.reservation_sizes(),
        None,
    );
    facts
}
fn inspect_input_resources(
    case: &str,
    input: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
) -> ResourceFacts {
    let mut budget = BookV2ColumnPageBudget::new(caps, MAXIMUM);
    math_drive(input, caps, MODE, &mut budget, |source, _, observed| {
        let mut builder =
            BookV2ColumnDisplayBuilder::new(source, input.resources(), caps, MAXIMUM, 0, 0)
                .unwrap();
        let display = builder.build().unwrap();
        chain(case, &display, caps, Some(observed))
    })
    .unwrap_or_else(|e| panic!("{case}: {e:?}"))
}

#[test]
fn book_v2_column_resources_close_original_body_notes_markers_and_unreferenced_definitions() {
    let caps = display_caps();
    for balanced in [false, true] {
        let mut data = plain_data("Pro", true, false);
        anchors(&mut data);
        wrap_list(&mut data["document"]["blocks"], true);
        list_font_rule(&mut data);
        balance(&mut data, balanced);
        renumber(&mut data);
        let root = Root::new();
        let input = display_input(&root, data, Some(DISPLAY_LIST_FONT), &caps);
        let facts =
            inspect_input_resources(&format!("resource-lists-notes-{balanced}"), &input, &caps);
        assert!(facts.font_uses > 0 && !facts.glyphs.is_empty());
        assert_eq!((facts.images, facts.image_uses), (0, 0));
    }
}

#[test]
fn book_v2_column_resources_select_actual_native_vector_numbers_and_media_payloads() {
    let caps = display_caps();
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        for balanced in [false, true] {
            let mut data =
                super::super::super::super::super::super::super::block_widths::column_block_data(
                    kind, true, true,
                );
            if matches!(kind, "png" | "jpeg" | "svg") {
                fn has_figure(v: &Value) -> bool {
                    v.get("kind").is_some_and(|k| k == "figure")
                        || match v {
                            Value::Array(a) => a.iter().any(has_figure),
                            Value::Object(o) => o.values().any(has_figure),
                            _ => false,
                        }
                }
                let mut duplicate = data["document"]["footnotes"][0]["blocks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|b| has_figure(b))
                    .unwrap()
                    .clone();
                fn replace_image(v: &mut Value) {
                    match v {
                        Value::Array(a) => {
                            for v in a {
                                replace_image(v)
                            }
                        }
                        Value::Object(o) => {
                            if let Some(id) = o.get_mut("image_id") {
                                *id = 1.into();
                            }
                            for v in o.values_mut() {
                                replace_image(v)
                            }
                        }
                        _ => (),
                    }
                }
                replace_image(&mut duplicate);
                let mut declaration = data["resources"]["images"][0].clone();
                declaration["image_id"] = 1.into();
                data["resources"]["images"]
                    .as_array_mut()
                    .unwrap()
                    .push(declaration);
                data["document"]["footnotes"][0]["blocks"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            anchors(&mut data);
            balance(&mut data, balanced);
            let root = Root::new();
            let input = match kind {
                "vector" => vector_input(&root, data, &caps),
                "native" => {
                    let text = data["text_buffers"][0]["utf8"]
                        .as_str()
                        .unwrap()
                        .as_bytes()
                        .to_vec();
                    native_input_with_source(&root, data, &text, &caps)
                }
                _ => figure_input(
                    &root,
                    data,
                    if kind == "png" {
                        PNG
                    } else if kind == "jpeg" {
                        JPEG
                    } else {
                        SVG
                    },
                    &caps,
                ),
            };
            let facts = inspect_input_resources(
                &format!("resource-media-{kind}-{balanced}"),
                &input,
                &caps,
            );
            assert!(facts.font_uses > 0);
            assert_eq!(facts.image_uses > 0, kind != "native");
            if kind != "native" {
                assert!(facts.image_uses > facts.images);
            }
        }
    }
}

#[test]
fn book_v2_column_resources_close_actual_nested_repeated_header_graphs() {
    let caps = display_caps();
    for vector in [false, true] {
        for balanced in [false, true] {
            let mut data = math_terminals::header_data(vector, true, true);
            anchors(&mut data);
            balance(&mut data, balanced);
            let root = Root::new();
            let input = if vector {
                vector_input(&root, data, &caps)
            } else {
                crate::book_v2_resources::tests::shaping_tests::native_tests::native_input(
                    &root, data, &caps,
                )
            };
            let facts = inspect_input_resources(
                &format!("resource-nested-header-{vector}-{balanced}"),
                &input,
                &caps,
            );
            assert!(facts.font_uses > 0);
            assert_eq!(facts.image_uses > 0, vector);
        }
    }
}

#[test]
fn book_v2_column_resources_preserve_repeated_only_contextual_gid_through_cids() {
    with_retained_cut_display(|display, caps| {
        let semantic = display
            .text()
            .draws()
            .iter()
            .filter(|d| !d.repeated_header())
            .flat_map(|d| d.glyphs())
            .map(|g| g.original_gid().get())
            .collect::<BTreeSet<_>>();
        assert!(semantic.contains(&59) && !semantic.contains(&71));
        let facts = chain("resource-contextual-retained-cut", display, caps, None);
        assert!(facts.glyphs.contains(&71));
        assert!(facts.font_uses > 0 && facts.image_uses > 0);
    });
}

#[test]
fn book_v2_column_resources_empty_forced_and_zero_table_graphs_allocate_no_paint_uses() {
    let caps = display_caps();
    for table in [false, true] {
        for balanced in [false, true] {
            let mut data = if table {
                super::super::super::super::super::data(false, "common", "Pro")
            } else {
                plain_data("Pro", false, true)
            };
            if table {
                for name in ["head", "body"] {
                    for row in data["document"]["blocks"][0][name].as_array_mut().unwrap() {
                        for cell in row["cells"].as_array_mut().unwrap() {
                            cell["blocks"] = json!([]);
                        }
                    }
                }
            } else {
                let span = data["document"]["blocks"][0]["span"].clone();
                let forced = json!({"kind":"page_break","node_id":0,"classes":[],"span":span});
                let blocks = data["document"]["blocks"].as_array_mut().unwrap();
                blocks.insert(0, forced.clone());
                blocks.push(forced);
            }
            balance(&mut data, balanced);
            renumber(&mut data);
            let root = Root::new();
            let input = display_input(&root, data, None, &caps);
            let facts = inspect_input_resources(
                &format!("resource-zero-{table}-{balanced}"),
                &input,
                &caps,
            );
            assert_eq!((facts.font_uses, facts.images, facts.image_uses), (0, 0, 0));
            assert!(facts.glyphs.is_empty());
        }
    }
}

#[derive(Debug)]
struct Attempt {
    stage: usize,
    charges: [u64; 3],
    error: Option<String>,
}
fn attempt_stage(
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    caps: &M4EffectiveResourceLimits,
    target: usize,
    maximum: u64,
    prior: [u64; 3],
) -> Attempt {
    // Each observed variable is distinct, so accepted constructor prefixes can
    // also be examined when no owned builder is returned.
    let mut r = 0;
    let mut s = 0;
    let mut w = 0;
    let constructor = BookV2ColumnFontSelectionBuilder::new_counted(
        display, caps, maximum, prior[0], prior[1], prior[2], &mut r, &mut s, &mut w,
    );
    let mut builder = match constructor {
        Ok(b) => b,
        Err(e) => {
            return Attempt {
                stage: 0,
                charges: [r, s, w],
                error: Some(format!("{e:?}")),
            };
        }
    };
    macro_rules! finish {
        ($stage:expr) => {
            if target == $stage {
                return Attempt {
                    stage: $stage,
                    charges: [
                        builder.record_charge(),
                        builder.spool_charge(),
                        builder.work_steps(),
                    ],
                    error: None,
                };
            }
        };
    }
    macro_rules! operation {
        ($stage:expr,$op:expr) => {
            match $op {
                Ok(value) => value,
                Err(e) => {
                    return Attempt {
                        stage: $stage,
                        charges: [
                            builder.record_charge(),
                            builder.spool_charge(),
                            builder.work_steps(),
                        ],
                        error: Some(format!("{e:?}")),
                    }
                }
            }
        };
    }
    finish!(0);
    let fonts = operation!(1, builder.build());
    finish!(1);
    let closures = operation!(2, builder.prepare_font_closures(&fonts));
    finish!(2);
    let programs = operation!(3, builder.write_font_programs(&closures));
    finish!(3);
    let cids = operation!(4, builder.plan_cids(&programs));
    finish!(4);
    let constructor = builder.next_stage_counted(&mut r, &mut s, &mut w);
    builder = match constructor {
        Ok(b) => b,
        Err(e) => {
            return Attempt {
                stage: 5,
                charges: [r, s, w],
                error: Some(format!("{e:?}")),
            };
        }
    };
    finish!(5);
    let images = operation!(6, builder.select_images());
    finish!(6);
    let _rasters = operation!(7, builder.write_raster_programs(&images));
    drop(cids);
    Attempt {
        stage: 7,
        charges: [
            builder.record_charge(),
            builder.spool_charge(),
            builder.work_steps(),
        ],
        error: None,
    }
}

#[test]
fn book_v2_column_resources_each_stage_honors_exact_one_short_records_work_and_spool() {
    let caps = display_caps();
    for kind in ["native", "png", "jpeg", "vector"] {
        let data = super::super::super::super::super::super::super::block_widths::column_block_data(
            kind, false, false,
        );
        let root = Root::new();
        let input = match kind {
            "native" => {
                let text = data["text_buffers"][0]["utf8"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .to_vec();
                native_input_with_source(&root, data, &text, &caps)
            }
            "vector" => vector_input(&root, data, &caps),
            _ => figure_input(&root, data, if kind == "png" { PNG } else { JPEG }, &caps),
        };
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        math_drive(&input, &caps, MODE, &mut budget, |source, _, _| {
            let mut display_builder =
                BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                    .unwrap();
            let display = display_builder.build().unwrap();
            let base = [
                display.record_charge(),
                display.source().spool_charge(),
                display.work_steps(),
            ];
            for target in 0..8 {
                let baseline = attempt_stage(&display, &caps, target, MAXIMUM, [0; 3]);
                assert!(
                    baseline.error.is_none(),
                    "{kind} stage{target}: {baseline:?}"
                );
                // Record and spool caps are exercised with the same admitted
                // limits and actual source graph. Only the accepted prior
                // ledger changes, so no limits identity mismatch masks them.
                for (axis, cap) in [
                    (0, caps.base().get().max_fragments),
                    (1, caps.base().get().max_spool_bytes),
                ] {
                    let delta = baseline.charges[axis] - base[axis];
                    if axis == 1
                        && (target == 0
                            || baseline.charges[1]
                                == attempt_stage(&display, &caps, target - 1, MAXIMUM, [0; 3])
                                    .charges[1])
                    {
                        continue;
                    }
                    assert!(delta > 0);
                    let mut prior = [0; 3];
                    prior[axis] = cap - delta;
                    assert!(prior[axis] >= base[axis]);
                    let exact = attempt_stage(&display, &caps, target, MAXIMUM, prior);
                    assert!(
                        exact.error.is_none(),
                        "{kind} axis{axis} exact{target}: {exact:?}"
                    );
                    assert_eq!(exact.charges[axis], cap);
                    prior[axis] += 1;
                    let short = attempt_stage(&display, &caps, target, MAXIMUM, prior);
                    assert!(
                        short
                            .error
                            .as_deref()
                            .is_some_and(|e| e.contains(if axis == 0 {
                                "Records"
                            } else {
                                "Spool"
                            })),
                        "{kind} axis{axis} short{target}: {short:?}"
                    );
                    assert_eq!(
                        short.stage, target,
                        "{kind} axis{axis} failed prior stage: {short:?}"
                    );
                    assert!(short.charges[axis] <= cap);
                }
                let exact = attempt_stage(&display, &caps, target, baseline.charges[2], [0; 3]);
                assert!(
                    exact.error.is_none(),
                    "{kind} work exact{target}: {exact:?}"
                );
                let short = attempt_stage(&display, &caps, target, baseline.charges[2] - 1, [0; 3]);
                assert!(
                    short.error.is_some(),
                    "{kind} work short{target}: {short:?}"
                );
                if target == 0 {
                    // This lower cap rejects a prefix already issued by the
                    // display. The constructor retains that prefix and adds
                    // no work; it cannot refund the ancestor's accepted work.
                    assert_eq!(short.charges[2], base[2]);
                    assert_eq!(short.error.as_deref(), Some("Work"));
                } else {
                    assert!(
                        short.charges[2] <= baseline.charges[2] - 1,
                        "{kind} work stage{target}: {short:?}"
                    );
                }
                if target != 5 {
                    assert_eq!(short.stage, target);
                }
            }
        })
        .unwrap();
    }
}

#[test]
fn book_v2_column_resources_constructor_limits_and_failed_retry_keep_accepted_prefixes() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, plain_data("Pro", false, false), None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    math_drive(&input, &caps, MODE, &mut budget, |source, _, _| {
        let mut db =
            BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                .unwrap();
        let display = db.build().unwrap();
        let changed = changed_caps(&caps, |c| c.max_fragments += 1);
        assert!(matches!(
            BookV2ColumnFontSelectionBuilder::new(&display, &changed, MAXIMUM, 0, 0, 0),
            Err(BookV2FontSelectionError::Identity)
        ));
        let cap = caps.base().get().max_fragments;
        let mut r = 0;
        let mut s = 0;
        let mut w = 0;
        let exact = BookV2ColumnFontSelectionBuilder::new_counted(
            &display,
            &caps,
            MAXIMUM,
            cap - 1,
            0,
            0,
            &mut r,
            &mut s,
            &mut w,
        )
        .unwrap();
        assert_eq!(exact.record_charge(), cap);
        drop(exact);
        assert!(matches!(
            BookV2ColumnFontSelectionBuilder::new_counted(
                &display, &caps, MAXIMUM, cap, 0, 0, &mut r, &mut s, &mut w
            ),
            Err(BookV2FontSelectionError::Records)
        ));
        assert_eq!(r, cap);
        let spool_cap = caps.base().get().max_spool_bytes;
        let exact = BookV2ColumnFontSelectionBuilder::new_counted(
            &display, &caps, MAXIMUM, 0, spool_cap, 0, &mut r, &mut s, &mut w,
        )
        .unwrap();
        assert_eq!(exact.spool_charge(), spool_cap);
        drop(exact);
        assert!(matches!(
            BookV2ColumnFontSelectionBuilder::new_counted(
                &display,
                &caps,
                MAXIMUM,
                0,
                spool_cap + 1,
                0,
                &mut r,
                &mut s,
                &mut w
            ),
            Err(BookV2FontSelectionError::Spool)
        ));
        assert_eq!((r, s), (display.record_charge() + 1, spool_cap + 1));
        assert!(matches!(
            BookV2ColumnFontSelectionBuilder::new_counted(
                &display,
                &caps,
                MAXIMUM,
                0,
                0,
                MAXIMUM + 1,
                &mut r,
                &mut s,
                &mut w
            ),
            Err(BookV2FontSelectionError::Work)
        ));
        assert_eq!((r, w), (display.record_charge() + 1, MAXIMUM + 1));
        assert!(matches!(
            BookV2ColumnFontSelectionBuilder::new_counted(
                &display,
                &caps,
                MAXIMUM,
                cap,
                spool_cap + 1,
                MAXIMUM + 1,
                &mut r,
                &mut s,
                &mut w
            ),
            Err(BookV2FontSelectionError::Records)
        ));
        assert_eq!(r, cap);
        let baseline = attempt_stage(&display, &caps, 1, MAXIMUM, [0; 3]);
        let maximum = baseline.charges[2] - 1;
        let mut builder =
            BookV2ColumnFontSelectionBuilder::new(&display, &caps, maximum, 0, 0, 0).unwrap();
        assert!(matches!(
            builder.build(),
            Err(BookV2FontSelectionError::Work)
        ));
        let prefix = [
            builder.record_charge(),
            builder.spool_charge(),
            builder.work_steps(),
        ];
        assert_eq!(prefix[2], maximum);
        assert!(matches!(
            builder.build(),
            Err(BookV2FontSelectionError::Work)
        ));
        assert_eq!(
            [
                builder.record_charge(),
                builder.spool_charge(),
                builder.work_steps()
            ],
            prefix
        );
    })
    .unwrap();
}

#[test]
fn book_v2_column_resources_failed_raster_retry_retains_each_accepted_reservation() {
    let caps = display_caps();
    let data = super::super::super::super::super::super::super::block_widths::column_block_data(
        "png", false, false,
    );
    let root = Root::new();
    let input = figure_input(&root, data, PNG, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    math_drive(&input, &caps, MODE, &mut budget, |source, _, _| {
        let mut db =
            BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                .unwrap();
        let display = db.build().unwrap();
        let baseline = attempt_stage(&display, &caps, 7, MAXIMUM, [0; 3]);
        let maximum = baseline.charges[2] - 1;
        let mut builder =
            BookV2ColumnFontSelectionBuilder::new(&display, &caps, maximum, 0, 0, 0).unwrap();
        let fonts = builder.build().unwrap();
        let closures = builder.prepare_font_closures(&fonts).unwrap();
        let programs = builder.write_font_programs(&closures).unwrap();
        let _cids = builder.plan_cids(&programs).unwrap();
        let mut builder = builder.next_stage_counted(&mut 0, &mut 0, &mut 0).unwrap();
        let images = builder.select_images().unwrap();
        assert!(matches!(
            builder.write_raster_programs(&images),
            Err(typaxis_resources::book_v2::BookV2RasterError::Budget(
                BookV2FontSelectionError::Work
            ))
        ));
        let prefix = [
            builder.record_charge(),
            builder.spool_charge(),
            builder.work_steps(),
        ];
        assert_eq!(prefix[2], maximum);
        assert!(matches!(
            builder.write_raster_programs(&images),
            Err(typaxis_resources::book_v2::BookV2RasterError::Budget(
                BookV2FontSelectionError::Work
            ))
        ));
        assert_eq!(builder.work_steps(), prefix[2]);
        assert_eq!(
            builder.record_charge(),
            prefix[0] + images.images().len() as u64 + 1
        );
        assert_eq!(
            builder.spool_charge(),
            prefix[1]
                + (images.images().len()
                    * std::mem::size_of::<
                        Option<typaxis_resources::book_v2::BookV2RasterProgram<'_, '_>>,
                    >()) as u64
        );
    })
    .unwrap();
}

#[test]
fn book_v2_column_resources_driver_accounts_exact_work_callback_failure_and_retry() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, plain_data("Pro", true, false), None, &caps);
    let mut warm = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let warm_facts = resource_drive(
        &input,
        &caps,
        MODE,
        &mut warm,
        |bundle, builder, observed| {
            assert_eq!(bundle.caller_before(), [0; 3]);
            assert_eq!(
                bundle.images().prior_charges(),
                [
                    bundle.cids().record_charge(),
                    bundle.cids().spool_charge(),
                    bundle.cids().work_steps()
                ]
            );
            assert_eq!(
                [
                    builder.record_charge(),
                    builder.spool_charge(),
                    builder.work_steps()
                ],
                [
                    observed.record_charge(),
                    observed.spool_charge(),
                    observed.work_steps()
                ]
            );
            let facts = inspect_resources(
                bundle.cids(),
                bundle.rasters(),
                bundle.constructor_charges(),
            );
            resource_probes::emit(
                "resource-driver-warm",
                bundle.cids(),
                bundle.rasters(),
                &caps,
                Some(observed),
                "resource_driver",
                MAXIMUM,
                bundle.constructor_charges(),
                builder.reservation_sizes(),
                Some(bundle.caller_before()),
            );
            facts
        },
    )
    .unwrap();
    let maximum = warm.observation().work_steps();
    let mut exact = BookV2ColumnPageBudget::new(&caps, maximum);
    let actual = resource_drive(
        &input,
        &caps,
        MODE,
        &mut exact,
        |bundle, builder, observed| {
            resource_probes::emit(
                "resource-driver-exact",
                bundle.cids(),
                bundle.rasters(),
                &caps,
                Some(observed),
                "resource_driver",
                maximum,
                bundle.constructor_charges(),
                builder.reservation_sizes(),
                Some(bundle.caller_before()),
            );
            inspect_resources(
                bundle.cids(),
                bundle.rasters(),
                bundle.constructor_charges(),
            )
        },
    )
    .unwrap();
    assert_eq!(actual, warm_facts);
    assert_eq!(exact.observation().work_steps(), maximum);
    let mut short = BookV2ColumnPageBudget::new(&caps, maximum - 1);
    let mut delivered = false;
    assert!(resource_drive(&input, &caps, MODE, &mut short, |_, _, _| delivered = true).is_err());
    assert!(!delivered);
    assert!(short.observation().work_steps() <= maximum - 1);
    let before = short.observation();
    assert!(resource_drive(&input, &caps, MODE, &mut short, |_, _, _| delivered = true).is_err());
    assert!(!delivered);
    assert_eq!(short.observation().work_steps(), before.work_steps());
    assert!(short.observation().record_charge() >= before.record_charge());
    let mut failed = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut callback_before = [0; 3];
    let mut callback_after = [0; 3];
    let result: Result<Result<(), &str>, _> = resource_drive(
        &input,
        &caps,
        MODE,
        &mut failed,
        |bundle, builder, observed| {
            callback_before = [
                observed.record_charge(),
                observed.spool_charge(),
                observed.work_steps(),
            ];
            let repeated = builder.select_images().unwrap();
            assert!(std::ptr::eq(repeated.display(), bundle.display()));
            callback_after = [
                builder.record_charge(),
                builder.spool_charge(),
                builder.work_steps(),
            ];
            Err("resource callback sentinel")
        },
    );
    assert_eq!(result.unwrap(), Err("resource callback sentinel"));
    assert!(callback_after
        .iter()
        .zip(callback_before)
        .all(|(after, before)| *after >= before));
    assert!(callback_after[0] > callback_before[0] && callback_after[2] > callback_before[2]);
    assert_eq!(
        [
            failed.observation().record_charge(),
            failed.observation().spool_charge(),
            failed.observation().work_steps()
        ],
        callback_after
    );
    let caller_before = callback_after;
    resource_drive(&input, &caps, MODE, &mut failed, |bundle, _, _| {
        assert_eq!(bundle.caller_before(), caller_before)
    })
    .unwrap();
    assert!(failed.observation().record_charge() > caller_before[0]);
    assert!(failed.observation().work_steps() > caller_before[2]);
}

#[test]
fn book_v2_column_resources_driver_exact_one_short_records_and_spool_never_deliver_partial_roots() {
    let caps = display_caps();
    let root = Root::new();
    let data = plain_data("Pro", true, false);
    let input = display_input(&root, data.clone(), None, &caps);
    let mut warm = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let uses = resource_drive(&input, &caps, MODE, &mut warm, |bundle, _, _| {
        bundle.fonts().uses().len()
    })
    .unwrap();
    assert!(uses > 0);
    for axis in [0, 1] {
        let cap = if axis == 0 {
            warm.observation().record_charge()
        } else {
            warm.observation().spool_charge()
        };
        for short in [false, true] {
            let cap = cap - u64::from(short);
            let limits = if axis == 0 {
                changed_caps(&caps, |c| c.max_fragments = cap)
            } else {
                let mut base = caps.base().get().clone();
                base.max_spool_bytes = cap;
                let mut extension = caps.extension().get().clone();
                extension.max_font_subset_bytes = extension.max_font_subset_bytes.min(cap);
                M4EffectiveResourceLimits::new(
                    ValidatedResourceLimits::new(base).unwrap(),
                    extension,
                )
                .unwrap()
            };
            let root = Root::new();
            let input = display_input(&root, data.clone(), None, &limits);
            let mut budget = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
            let mut delivered = false;
            let result = resource_drive(
                &input,
                &limits,
                MODE,
                &mut budget,
                |bundle, builder, observed| {
                    delivered = true;
                    assert_eq!(bundle.fonts().uses().len(), uses);
                    inspect_resources(
                        bundle.cids(),
                        bundle.rasters(),
                        bundle.constructor_charges(),
                    );
                    resource_probes::emit(
                        &format!("resource-driver-exact-axis-{axis}"),
                        bundle.cids(),
                        bundle.rasters(),
                        &limits,
                        Some(observed),
                        "resource_driver",
                        MAXIMUM,
                        bundle.constructor_charges(),
                        builder.reservation_sizes(),
                        Some(bundle.caller_before()),
                    );
                },
            );
            if short {
                assert!(result.is_err(), "axis{axis}: {result:?}");
                assert!(!delivered);
                let before = budget.observation();
                assert!(
                    resource_drive(&input, &limits, MODE, &mut budget, |_, _, _| delivered =
                        true)
                    .is_err()
                );
                assert!(!delivered);
                assert!(budget.observation().record_charge() >= before.record_charge());
                assert!(budget.observation().spool_charge() >= before.spool_charge());
            } else {
                assert!(result.is_ok(), "axis{axis}: {result:?}");
                assert!(delivered);
            }
            let charged = if axis == 0 {
                budget.observation().record_charge()
            } else {
                budget.observation().spool_charge()
            };
            assert!(charged <= cap, "axis{axis} short{short}: {charged}>{cap}");
            if !short {
                assert_eq!(charged, cap);
            }
        }
    }
}

#[test]
fn book_v2_column_resources_reject_foreign_display_parents_before_ledger_adoption() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, plain_data("Pro", false, false), None, &caps);
    let mut outer = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    math_drive(&input, &caps, MODE, &mut outer, |source, _, _| {
        let mut db =
            BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                .unwrap();
        let display = db.build().unwrap();
        let mut owner =
            BookV2ColumnFontSelectionBuilder::new(&display, &caps, MAXIMUM, 0, 0, 0).unwrap();
        let selection = owner.build().unwrap();
        let closures = owner.prepare_font_closures(&selection).unwrap();
        let programs = owner.write_font_programs(&closures).unwrap();
        let images = owner.select_images().unwrap();
        let mut inner = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        math_drive(&input, &caps, MODE, &mut inner, |other, _, _| {
            let mut db =
                BookV2ColumnDisplayBuilder::new(other, input.resources(), &caps, MAXIMUM, 0, 0)
                    .unwrap();
            let other_display = db.build().unwrap();
            assert!(!std::ptr::eq(&display, &other_display));
            let mut foreign =
                BookV2ColumnFontSelectionBuilder::new(&other_display, &caps, MAXIMUM, 0, 0, 0)
                    .unwrap();
            let before = [
                foreign.record_charge(),
                foreign.spool_charge(),
                foreign.work_steps(),
            ];
            assert!(matches!(
                foreign.prepare_font_closures(&selection),
                Err(typaxis_resources::book_v2::BookV2FontClosureError::Budget(
                    BookV2FontSelectionError::Identity
                ))
            ));
            assert!(foreign.write_font_programs(&closures).is_err());
            assert!(foreign.plan_cids(&programs).is_err());
            assert!(foreign.write_raster_programs(&images).is_err());
            assert_eq!(
                [
                    foreign.record_charge(),
                    foreign.spool_charge(),
                    foreign.work_steps()
                ],
                before
            );
        })
        .unwrap();
    })
    .unwrap();
}

#[test]
fn book_v2_column_resources_named_parity_residual_columns_and_blank_prefix_keep_source_glyphs() {
    let caps = display_caps();
    let text = "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro";
    for blanks in [1, 2] {
        for balanced in [false, true] {
            let root = Root::new();
            let input = display_input(
                &root,
                named_display_data(text, blanks, balanced),
                None,
                &caps,
            );
            let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
            resource_drive(
                &input,
                &caps,
                MODE,
                &mut budget,
                |bundle, builder, observed| {
                    let display = bundle.display();
                    inspect(display);
                    let pages = display.source().source().geometry().pages();
                    assert_eq!(pages.len(), blanks + 1);
                    assert!(pages[..blanks].iter().all(|p| p.fragments().is_empty()));
                    let last = pages.last().unwrap();
                    assert_eq!(last.selection().named_page(), Some("display-appendix"));
                    let count = if blanks == 1 { 3 } else { 2 };
                    assert_eq!(last.columns().len(), count);
                    assert_eq!(last.selection().balance().is_some(), balanced);
                    let widths = last
                        .columns()
                        .iter()
                        .map(|c| c.bounds().width().get().raw())
                        .collect::<Vec<_>>();
                    assert_eq!(widths[count - 1], 100 * 65536 + 1);
                    assert!(widths[..count - 1].iter().all(|w| *w == 100 * 65536));
                    let facts = inspect_resources(
                        bundle.cids(),
                        bundle.rasters(),
                        bundle.constructor_charges(),
                    );
                    assert!(facts.font_uses > 0);
                    assert!(bundle
                        .fonts()
                        .uses()
                        .iter()
                        .all(|u| matches!(display.paints()[u.paint_index()], Paint::Text(_))));
                    resource_probes::emit(
                        &format!("resource-named-{blanks}-{balanced}"),
                        bundle.cids(),
                        bundle.rasters(),
                        &caps,
                        Some(observed),
                        "resource_driver",
                        MAXIMUM,
                        bundle.constructor_charges(),
                        builder.reservation_sizes(),
                        Some(bundle.caller_before()),
                    );
                },
            )
            .unwrap();
        }
    }
}

#[test]
fn book_v2_column_resources_balanced_six_stream_page_preserves_notes_formula_number_and_images() {
    let caps = display_caps();
    let root = Root::new();
    let input = vector_input(&root, balanced_six_stream_data(), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    resource_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        |bundle, builder, observed| {
            let facts = inspect(bundle.display());
            assert!(
                facts.balanced > 0
                    && facts.notes > 0
                    && facts.vector_math > 0
                    && facts.numbers > 0
                    && facts.markers > 0
                    && facts.images > 0
            );
            let resources = inspect_resources(
                bundle.cids(),
                bundle.rasters(),
                bundle.constructor_charges(),
            );
            assert!(resources.font_uses > 0 && resources.image_uses > resources.images);
            resource_probes::emit(
                "resource-balanced-six-stream-notes",
                bundle.cids(),
                bundle.rasters(),
                &caps,
                Some(observed),
                "resource_driver",
                MAXIMUM,
                bundle.constructor_charges(),
                builder.reservation_sizes(),
                Some(bundle.caller_before()),
            );
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_resources_unreferenced_rich_definition_never_enters_font_or_image_uses() {
    let caps = display_caps();
    let mut data = super::super::super::super::super::super::super::block_widths::column_block_data(
        "vector", true, false,
    );
    let mut unused = data["document"]["footnotes"][0].clone();
    unused["footnote_id"] = "resource-unused-rich".into();
    data["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(unused);
    anchors(&mut data);
    fn owners(v: &Value, set: &mut BTreeSet<u32>) {
        if let Some(n) = v.get("node_id").and_then(Value::as_u64) {
            set.insert(n as u32);
        }
        match v {
            Value::Array(a) => {
                for v in a {
                    owners(v, set)
                }
            }
            Value::Object(o) => {
                for v in o.values() {
                    owners(v, set)
                }
            }
            _ => (),
        }
    }
    let mut excluded = BTreeSet::new();
    owners(&data["document"]["footnotes"][1], &mut excluded);
    let root = Root::new();
    let input = vector_input(&root, data, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    resource_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        |bundle, builder, observed| {
            let display = bundle.display();
            assert_eq!(display.source().source().unreferenced_definitions(), 1);
            assert!(display
                .source()
                .original_graph()
                .lines()
                .paragraphs()
                .iter()
                .any(|p| excluded.contains(&p.owner().get())));
            inspect_resources(
                bundle.cids(),
                bundle.rasters(),
                bundle.constructor_charges(),
            );
            for u in bundle.fonts().uses() {
                let owner = match u.usage().paint() {
                    Paint::Text(i) => display.text().draws()[i].owner(),
                    Paint::Marker(i) => display.markers().draws()[i].source().owner(),
                    Paint::EquationNumber(i) => {
                        display.numbers().draws()[i].placement().geometry().owner()
                    }
                    Paint::Math(i) => display.math().draws()[i].terminal().source().owner(),
                    _ => panic!("non-font paint authorized glyph selection"),
                };
                assert!(!excluded.contains(&owner.get()));
            }
            assert!(bundle
                .images()
                .uses()
                .iter()
                .all(|u| !excluded.contains(&u.usage().owner().get())));
            assert!(display
                .anchors()
                .positions()
                .iter()
                .all(|a| !excluded.contains(&a.anchor().source().owner().get())));
            assert!(display
                .source()
                .source()
                .geometry()
                .pages()
                .iter()
                .flat_map(|p| p.fragments())
                .all(|f| f.definition_index() != Some(1)));
            resource_probes::emit(
                "resource-unused-rich",
                bundle.cids(),
                bundle.rasters(),
                &caps,
                Some(observed),
                "resource_driver",
                MAXIMUM,
                bundle.constructor_charges(),
                builder.reservation_sizes(),
                Some(bundle.caller_before()),
            );
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_resources_keep_original_rowspan_nested_caption_and_forced_table_glyphs() {
    let caps = display_caps();
    let text = "Pro Pro Pro Pro Pro Pro Pro";
    for (mode, height, balanced) in [
        ("span", 256, false),
        ("nested-body-span", 544, false),
        ("nested-body-caption", 512, true),
        ("forced", 256, false),
    ] {
        let mut data = super::super::super::super::super::data(false, mode, text);
        for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
            master["body"]["height"] = (height * 65536).into();
        }
        anchors(&mut data);
        balance(&mut data, balanced);
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let facts =
            inspect_input_resources(&format!("resource-table-{mode}-{balanced}"), &input, &caps);
        assert!(facts.font_uses > 0 && !facts.glyphs.is_empty());
    }
}

#[test]
#[ignore = "requires TYPAXIS_HARANO_FONT pointing to original HaranoAjiMincho-Regular.otf SHA256 66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"]
fn book_v2_column_resources_close_original_harano_cid_cff_programs() {
    let path = std::env::var_os("TYPAXIS_HARANO_FONT").expect("original font path");
    let bytes = fs::read(path).unwrap();
    assert_eq!(
        probes::hex(&typaxis_core::sha256(&bytes)),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(
        &root,
        plain_data("日本語", true, false),
        Some(&bytes),
        &caps,
    );
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    resource_drive(&input, &caps, MODE, &mut budget, |bundle, _, _| {
        assert!(bundle
            .cids()
            .source()
            .fonts()
            .iter()
            .all(|p| p.kind() == BookV2FontClosureKind::Cff1V2));
        assert!(bundle.fonts().uses().len() > 0);
        for p in bundle.cids().source().fonts() {
            assert_eq!(typaxis_core::sha256(p.bytes()), p.sha256());
        }
    })
    .unwrap();
}
