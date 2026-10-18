use super::*;
use crate::book_v2_resources::with_budgeted_book_v2_column_paints as paint_drive;
use typaxis_pdf::book_v2::{
    BookV2ColumnFontObjectBuilder, BookV2ColumnFontStreamBuilder, BookV2ColumnImageCommandBuilder,
    BookV2ColumnImageCommands, BookV2ColumnImageObjectBuilder, BookV2ColumnTextCommandBuilder,
    BookV2ColumnTextCommands, BookV2ColumnVectorProgramBuilder, BookV2FontAuxiliaryKind,
    BookV2FontObjectRole, BookV2ImageObjectRole,
};

#[path = "book_v2_column_paint_probes.rs"]
mod paint_probes;

#[path = "book_v2_column_paint_callback_ownership_tests.rs"]
mod callback_ownership_tests;

#[path = "book_v2_column_marked_tests.rs"]
mod marked_tests;

fn fixed(token: &str) -> i128 {
    let negative = token.starts_with('-');
    let token = token.strip_prefix('-').unwrap_or(token);
    let (whole, fraction) = token.split_once('.').unwrap_or((token, ""));
    let divisor = 10i128.pow(fraction.len() as u32);
    let numerator = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i128>().unwrap()
    } * 65536;
    assert_eq!(numerator % divisor, 0, "inexact command scalar {token}");
    let raw = whole.parse::<i128>().unwrap() * 65536 + numerator / divisor;
    if negative { -raw } else { raw }
}
fn utf16(token: &[u8], bom: bool) -> String {
    let s = std::str::from_utf8(token).unwrap();
    assert!(s.starts_with('<') && s.ends_with('>'));
    let s = &s[1..s.len() - 1];
    assert_eq!(s.len() % 4, 0);
    let mut units = (0..s.len())
        .step_by(4)
        .map(|i| u16::from_str_radix(&s[i..i + 4], 16).unwrap())
        .collect::<Vec<_>>();
    if bom {
        assert_eq!(units.remove(0), 0xfeff);
    }
    String::from_utf16(&units).unwrap()
}
fn cmap(bytes: &[u8]) -> BTreeMap<u16, String> {
    let s = std::str::from_utf8(bytes).unwrap();
    assert!(s.contains("<0000> <FFFF>"));
    let mut result = BTreeMap::new();
    let mut lines = s.lines();
    while let Some(line) = lines.next() {
        if let Some(n) = line.strip_suffix(" beginbfchar") {
            let n: usize = n.parse().unwrap();
            assert!((1..=100).contains(&n));
            for _ in 0..n {
                let fields = lines.next().unwrap().split_whitespace().collect::<Vec<_>>();
                assert_eq!(fields.len(), 2);
                let cid = u16::from_str_radix(&fields[0][1..5], 16).unwrap();
                assert!(cid > 0);
                assert!(
                    result
                        .insert(cid, utf16(fields[1].as_bytes(), false))
                        .is_none()
                );
            }
            assert_eq!(lines.next(), Some("endbfchar"));
        }
    }
    result
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PaintFacts {
    fingerprints: [[u8; 32]; 6],
    text_commands: usize,
    image_commands: usize,
    rules: usize,
    actual_text: usize,
    sizes: BTreeSet<i64>,
    columns: BTreeSet<Option<u16>>,
    repeated: usize,
}

fn inspect_paints(
    text: &BookV2ColumnTextCommands<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    images: &BookV2ColumnImageCommands<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
) -> PaintFacts {
    let objects = text.source();
    let streams = objects.source();
    let cids = streams.source();
    let image_objects = images.source();
    let vectors = image_objects.source();
    let rasters = vectors.source();
    let display = text.display();
    assert!(std::ptr::eq(display, rasters.source().display()));
    inspect(display);
    // Keep the original resource oracle: exact glyph unions, original sfnt
    // dependencies/advances, source pointers, CID mappings and image payloads.
    inspect_resources(cids, rasters, [[0; 3]; 2]);
    let expected = expected_fonts(display);
    let mut maps = Vec::new();
    for (font, bindings) in cids
        .fonts()
        .iter()
        .enumerate()
        .map(|(i, _)| (i, cids.bindings(i).unwrap()))
    {
        let map = cmap(streams.to_unicode(font).unwrap());
        assert_eq!(
            map,
            bindings
                .iter()
                .filter_map(|b| b.unicode().map(|c| (b.cid().get(), c.to_string())))
                .collect()
        );
        maps.push(map);
        match cids.fonts()[font].source().kind() {
            BookV2FontClosureKind::TrueType => {
                assert_eq!(
                    streams.fonts()[font].auxiliary_kind(),
                    BookV2FontAuxiliaryKind::TrueTypeCidToGid
                );
                let gids = streams
                    .auxiliary(font)
                    .unwrap()
                    .chunks_exact(2)
                    .map(|b| u16::from_be_bytes(b.try_into().unwrap()))
                    .collect::<Vec<_>>();
                assert_eq!(gids.len(), bindings.len() + 1);
                assert_eq!(gids[0], 0);
                for b in bindings {
                    assert_eq!(gids[b.cid().get() as usize], b.subset_gid().get());
                }
            }
            BookV2FontClosureKind::Cff1V2 => {
                assert_eq!(
                    streams.fonts()[font].auxiliary_kind(),
                    BookV2FontAuxiliaryKind::CffCidSet
                );
                let count = cids.fonts()[font].source().source().glyphs().len();
                let bits = streams.auxiliary(font).unwrap();
                assert_eq!(bits.len(), count.div_ceil(8));
                for bit in 0..bits.len() * 8 {
                    assert_eq!(bits[bit / 8] & (0x80 >> (bit % 8)) != 0, bit < count);
                }
            }
        }
    }
    for (i, e) in expected.iter().enumerate() {
        let use_cids = cids.cids(i).unwrap();
        let extracted = use_cids
            .iter()
            .filter_map(|c| maps[cids.uses()[i].font_index()].get(&c.get()))
            .cloned()
            .collect::<String>();
        let replacement = streams.actual_text(i);
        assert_eq!(replacement.is_some(), cids.uses()[i].requires_actual_text());
        assert_eq!(
            replacement.map(|r| utf16(r, true)).unwrap_or(extracted),
            e.text
        );
    }
    let mut cursor = 0;
    assert_eq!(objects.objects().len(), streams.fonts().len() * 6);
    for (i, o) in objects.objects().iter().enumerate() {
        assert_eq!(o.id().get(), objects.first_object() + i as u32);
        assert_eq!(o.font_index(), i / 6);
        assert_eq!(
            o.role(),
            [
                BookV2FontObjectRole::Type0,
                BookV2FontObjectRole::CidFont,
                BookV2FontObjectRole::Descriptor,
                BookV2FontObjectRole::Program,
                BookV2FontObjectRole::ToUnicode,
                BookV2FontObjectRole::Auxiliary
            ][i % 6]
        );
        assert_eq!(o.byte_range().start, cursor);
        cursor = o.byte_range().end;
        let bytes = &objects.bytes()[o.byte_range()];
        assert!(bytes.starts_with(format!("{} 0 obj\n", o.id().get()).as_bytes()));
        assert!(bytes.ends_with(b"\nendobj\n"));
        let program = cids.fonts()[i / 6].source();
        let payload = match o.role() {
            BookV2FontObjectRole::Program => Some(program.bytes()),
            BookV2FontObjectRole::ToUnicode => Some(streams.to_unicode(i / 6).unwrap()),
            BookV2FontObjectRole::Auxiliary => Some(streams.auxiliary(i / 6).unwrap()),
            _ => None,
        };
        if let Some(payload) = payload {
            let start = bytes.windows(8).position(|b| b == b"\nstream\n").unwrap() + 8;
            assert_eq!(&bytes[start..start + payload.len()], payload);
            assert!(
                std::str::from_utf8(&bytes[..start])
                    .unwrap()
                    .contains(&format!("/Length {}", payload.len()))
            );
        } else {
            let s = std::str::from_utf8(bytes).unwrap();
            assert!(s.contains(program.postscript_name()));
            match o.role() {
                BookV2FontObjectRole::Type0 => assert!(s.contains("/Subtype /Type0")),
                BookV2FontObjectRole::CidFont => {
                    let cff = program.kind() == BookV2FontClosureKind::Cff1V2;
                    assert_eq!(s.contains("/Subtype /CIDFontType0"), cff);
                    assert_eq!(s.contains("/Subtype /CIDFontType2"), !cff);
                    assert_eq!(s.contains("/CIDToGIDMap"), !cff);
                }
                BookV2FontObjectRole::Descriptor => {
                    let cff = program.kind() == BookV2FontClosureKind::Cff1V2;
                    assert_eq!(s.contains("/FontFile2"), !cff);
                    assert_eq!(s.contains("/FontFile3"), cff);
                }
                _ => unreachable!(),
            }
        }
    }
    assert_eq!(cursor, objects.bytes().len());
    assert_eq!(
        objects.next_object(),
        objects.first_object() + objects.objects().len() as u32
    );
    let selected = rasters.source();
    let mut image_cursor = 0;
    let mut object_count = 0;
    for (index, selected_image) in selected.images().iter().enumerate() {
        let raster = rasters.program(index);
        let vector = vectors.program(index);
        let entries = image_objects.for_image(index).unwrap();
        let mut payloads = Vec::new();
        if let Some(r) = raster {
            assert!(vector.is_none());
            assert!(std::ptr::eq(r.source(), selected_image));
            assert_eq!(entries.len(), 1 + usize::from(r.alpha_mask().is_some()));
            assert_eq!(entries[0].role(), BookV2ImageObjectRole::Image);
            payloads.push(r.bytes());
            if let Some(a) = r.alpha_mask() {
                assert_eq!(entries[1].role(), BookV2ImageObjectRole::SoftMask);
                payloads.push(&a.encoded_bytes);
            }
        } else {
            let v = vector.unwrap();
            assert!(std::ptr::eq(v.source(), selected_image));
            assert!(std::ptr::eq(
                v.ir(),
                selected_image.image().admitted_safe_vector().unwrap()
            ));
            let pairs = match v.ir() {
                typaxis_resources::AdmittedSafeVector::V1(ir) => ir
                    .draws()
                    .iter()
                    .map(|_| (65536, 65536))
                    .collect::<Vec<_>>(),
                typaxis_resources::AdmittedSafeVector::V2(ir) => ir
                    .draws()
                    .iter()
                    .map(|d| (d.fill().alpha().raw(), d.stroke().paint().alpha().raw()))
                    .collect::<Vec<_>>(),
            };
            let sorted = pairs
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            assert_eq!(
                v.states()
                    .iter()
                    .map(|s| (s.fill_alpha_raw(), s.stroke_alpha_raw()))
                    .collect::<Vec<_>>(),
                sorted
            );
            assert_eq!(
                v.bbox(),
                [
                    0,
                    0,
                    v.ir().intrinsic_width().get().raw(),
                    v.ir().intrinsic_height().get().raw()
                ]
            );
            let content = std::str::from_utf8(v.content()).unwrap();
            assert_eq!(
                content.lines().filter(|s| *s == "q").count(),
                pairs.len() + 1
            );
            assert_eq!(
                content.lines().filter(|s| *s == "Q").count(),
                pairs.len() + 1
            );
            let actual = content
                .lines()
                .filter(|s| s.ends_with(" gs"))
                .map(|s| {
                    s.strip_prefix("/GS")
                        .unwrap()
                        .strip_suffix(" gs")
                        .unwrap()
                        .parse::<usize>()
                        .unwrap()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                actual,
                pairs
                    .iter()
                    .map(|p| sorted.binary_search(p).unwrap())
                    .collect::<Vec<_>>()
            );
            for forbidden in ["/MCID", "/Alt", "/ActualText", "/Subtype /Image", " Do"] {
                assert!(!content.contains(forbidden));
            }
            assert_eq!(entries.len(), v.states().len() + 1);
            assert_eq!(entries[0].role(), BookV2ImageObjectRole::Form);
            payloads.push(v.content());
            for (state, o) in entries[1..].iter().enumerate() {
                assert_eq!(o.role(), BookV2ImageObjectRole::ExtGState);
                assert_eq!(o.state_index(), Some(state));
            }
        }
        assert_eq!(image_objects.resource_object(index), Some(entries[0].id()));
        for (part, o) in entries.iter().enumerate() {
            assert_eq!(o.image_index(), index);
            assert_eq!(
                o.id().get(),
                image_objects.first_object() + object_count as u32
            );
            assert_eq!(o.byte_range().start, image_cursor);
            image_cursor = o.byte_range().end;
            object_count += 1;
            let b = &image_objects.bytes()[o.byte_range()];
            assert!(b.starts_with(format!("{} 0 obj\n", o.id().get()).as_bytes()));
            assert!(b.ends_with(b"\nendobj\n"));
            if let Some(payload) = payloads.get(part) {
                let start = b.windows(8).position(|b| b == b"\nstream\n").unwrap() + 8;
                assert_eq!(&b[start..start + payload.len()], *payload);
                let dictionary = std::str::from_utf8(&b[..start]).unwrap();
                assert!(dictionary.contains(&format!("/Length {}", payload.len())));
                if raster.is_some() {
                    assert!(dictionary.contains("/Subtype /Image"));
                    if part == 0 && payloads.len() == 2 {
                        assert!(
                            dictionary.contains(&format!("/SMask {} 0 R", entries[1].id().get()))
                        );
                    }
                } else {
                    assert!(dictionary.contains("/Subtype /Form"));
                }
            }
        }
    }
    assert_eq!(image_cursor, image_objects.bytes().len());
    assert_eq!(object_count, image_objects.objects().len());
    assert_eq!(
        image_objects.next_object(),
        image_objects.first_object() + object_count as u32
    );
    let mut facts = PaintFacts {
        fingerprints: [
            streams.fingerprint(),
            objects.fingerprint(),
            text.fingerprint(),
            vectors.fingerprint(),
            image_objects.fingerprint(),
            images.fingerprint(),
        ],
        text_commands: 0,
        image_commands: 0,
        rules: 0,
        actual_text: 0,
        sizes: BTreeSet::new(),
        columns: BTreeSet::new(),
        repeated: 0,
    };
    let mut command = 0;
    let mut use_index = 0;
    let mut offset = 0;
    // Expected slots and coordinates come from the actual Column component
    // graph, independently of the sealed CommandDisplay encoder view.
    for (paint, p) in display.paints().iter().copied().enumerate() {
        let mut slots = Vec::new(); // (page,slot,physical glyph positions,rule,fragment,repeated)
        match p {
            Paint::PageRegionText { .. } => panic!("running region outside the legacy Column fixture"),
            Paint::Text(i) => {
                let d = &display.text().draws()[i];
                slots.push((
                    d.page_index(),
                    Some(0),
                    d.glyphs()
                        .iter()
                        .map(|g| (g.x().raw(), g.y().raw()))
                        .collect::<Vec<_>>(),
                    None,
                    Some(d.fragment_index()),
                    d.repeated_header(),
                ));
            }
            Paint::Marker(i) => {
                let d = &display.markers().draws()[i];
                for (slot, c) in d.clusters().iter().enumerate() {
                    slots.push((
                        d.fragment().fragment().page_index(),
                        Some(slot),
                        c.glyphs()
                            .iter()
                            .map(|g| (g.x().raw(), g.y().raw()))
                            .collect(),
                        None,
                        Some(d.fragment_index()),
                        d.repeated_header(),
                    ));
                }
            }
            Paint::EquationNumber(i) => {
                let d = &display.numbers().draws()[i];
                for (slot, c) in d.clusters().iter().enumerate() {
                    slots.push((
                        d.placement().geometry().page_index(),
                        Some(slot),
                        c.glyphs()
                            .iter()
                            .map(|g| (g.x().raw(), g.y().raw()))
                            .collect(),
                        None,
                        Some(d.placement().geometry().fragment_index() as usize),
                        d.placement().repeated_header(),
                    ));
                }
            }
            Paint::Math(i) => {
                let d = &display.math().draws()[i];
                if let BookV2MathPaint::Native(n) = d.paint() {
                    for (slot, p) in n.paints().iter().enumerate() {
                        let (glyphs, rule) = match p {
                            ProductionNativeMathPaint::Glyph { x, y, .. } => {
                                (vec![(x.raw(), y.raw())], None)
                            }
                            ProductionNativeMathPaint::Rule(r) => (vec![], Some(*r)),
                        };
                        slots.push((
                            d.terminal().page_index(),
                            Some(slot),
                            glyphs,
                            rule,
                            Some(d.terminal().fragment_index()),
                            d.terminal().repeated_header(),
                        ));
                    }
                }
            }
            Paint::FootnoteSeparator(i) => {
                let d = &display.markers().separators()[i];
                slots.push((d.page_index(), None, vec![], Some(d.ink()), None, false));
            }
            Paint::Image(_) => (),
        }
        let actual = text.for_paint(paint).unwrap();
        assert_eq!(actual.len(), slots.len());
        for (page, slot, glyphs, rule, fragment, repeated) in slots {
            let c = &text.commands()[command];
            assert_eq!(
                (c.paint_index(), c.slot_index(), c.page_index()),
                (paint, slot, page)
            );
            assert_eq!(c.byte_range().start, offset);
            offset = c.byte_range().end;
            let b = text.command_bytes(command).unwrap();
            let t = std::str::from_utf8(b)
                .unwrap()
                .split_whitespace()
                .collect::<Vec<_>>();
            assert_eq!(&t[..2], ["0", "g"]);
            for forbidden in ["BDC", "BMC", "EMC", "/MCID", "/ActualText", "/Alt"] {
                assert!(!t.contains(&forbidden));
            }
            if let Some(r) = rule {
                assert!(c.usage_index().is_none());
                assert!(text.actual_text(command).is_none());
                assert_eq!(t.len(), 8);
                assert_eq!(&t[6..], ["re", "f"]);
                for (token, raw) in t[2..6].iter().zip([
                    r.x().raw(),
                    r.y().raw(),
                    r.width().get().raw(),
                    r.height().get().raw(),
                ]) {
                    assert_eq!(fixed(token), i128::from(raw));
                }
                facts.rules += 1;
            } else {
                let e = &expected[use_index];
                assert_eq!((e.paint, e.slot), (paint, slot.unwrap()));
                assert_eq!(c.usage_index(), Some(use_index));
                assert_eq!(t[2], "BT");
                assert_eq!(t[3], format!("/PB{}", e.instance));
                assert_eq!(fixed(t[4]), i128::from(e.size));
                facts.sizes.insert(e.size);
                assert_eq!(
                    &t[5..18],
                    [
                        "Tf", "0", "Tr", "0", "Tc", "0", "Tw", "100", "Tz", "0", "TL", "0", "Ts"
                    ]
                );
                assert_eq!(t.last(), Some(&"ET"));
                let actual_glyphs = &t[18..t.len() - 1];
                assert_eq!(actual_glyphs.len(), glyphs.len() * 9);
                for ((tokens, (x, y)), cid) in actual_glyphs
                    .chunks_exact(9)
                    .zip(glyphs)
                    .zip(cids.cids(use_index).unwrap())
                {
                    assert_eq!(&tokens[..4], ["1", "0", "0", "-1"]);
                    assert_eq!(
                        (fixed(tokens[4]), fixed(tokens[5])),
                        (i128::from(x), i128::from(y))
                    );
                    assert_eq!(tokens[6], "Tm");
                    assert_eq!(tokens[8], "Tj");
                    assert_eq!(tokens[7], format!("<{:04X}>", cid.get()));
                }
                match (text.actual_text(command), streams.actual_text(use_index)) {
                    (Some(a), Some(b)) => {
                        assert!(std::ptr::eq(a, b));
                        facts.actual_text += 1
                    }
                    (None, None) => (),
                    _ => panic!("wrong occurrence replacement token"),
                };
                use_index += 1;
            }
            if let Some(fragment) = fragment {
                facts.columns.insert(display.fragment_column(fragment));
            }
            facts.repeated += usize::from(repeated);
            facts.text_commands += 1;
            command += 1;
        }
    }
    assert_eq!(command, text.commands().len());
    assert_eq!(offset, text.byte_length());
    assert_eq!(use_index, cids.uses().len());
    let selected = rasters.source();
    assert_eq!(images.commands().len(), selected.uses().len());
    let mut offset = 0;
    for (i, (c, u)) in images.commands().iter().zip(selected.uses()).enumerate() {
        assert_eq!(
            (
                c.paint_index(),
                c.resource_index(),
                c.page_index(),
                c.owner()
            ),
            (
                u.paint_index(),
                u.resource_index(),
                u.usage().page_index(),
                u.usage().owner()
            )
        );
        assert_eq!(
            c.resource_object(),
            image_objects.resource_object(u.resource_index()).unwrap()
        );
        assert!(std::ptr::eq(images.for_paint(u.paint_index()).unwrap(), c));
        assert_eq!(c.byte_range().start, offset);
        offset = c.byte_range().end;
        let s = std::str::from_utf8(images.command_bytes(i).unwrap()).unwrap();
        let mut lines = s.lines();
        assert_eq!(lines.next(), Some("q"));
        let original_geometry = match display.paints()[u.paint_index()] {
            Paint::Image(i) => {
                let d = &display.images().draws()[i];
                assert_eq!(
                    (c.owner(), c.page_index()),
                    (d.source().owner(), d.fragment().fragment().page_index())
                );
                d.paint()
            }
            Paint::Math(i) => {
                let d = &display.math().draws()[i];
                let BookV2MathPaint::Vector(v) = d.paint() else {
                    panic!("native math authorized image command")
                };
                assert_eq!(
                    (c.owner(), c.page_index()),
                    (d.terminal().source().owner(), d.terminal().page_index())
                );
                BookV2ImagePaint::Vector {
                    content_key: v.content_key(),
                    viewport: v.viewport(),
                    scale_raw: v.scale_raw(),
                    matrix: v.matrix(),
                    color: v.color(),
                }
            }
            _ => panic!("non-image source authorized a Do command"),
        };
        assert_eq!(u.usage().geometry(), original_geometry);
        let matrix = match original_geometry {
            BookV2ImagePaint::Raster { viewport } => [
                viewport.width().get().raw(),
                0,
                0,
                -viewport.height().get().raw(),
                viewport.x().raw(),
                viewport.y().raw() + viewport.height().get().raw(),
            ],
            BookV2ImagePaint::Vector { matrix, color, .. } => {
                for operator in ["rg", "RG"] {
                    let t = lines.next().unwrap().split_whitespace().collect::<Vec<_>>();
                    assert_eq!(t[3], operator);
                    for (token, c) in t[..3].iter().zip(color) {
                        assert_eq!(fixed(token), i128::from((u64::from(c) * 65536 + 127) / 255));
                    }
                }
                [
                    i64::from(matrix.a.raw()),
                    i64::from(matrix.b.raw()),
                    i64::from(matrix.c.raw()),
                    i64::from(matrix.d.raw()),
                    matrix.e.raw(),
                    matrix.f.raw(),
                ]
            }
        };
        let t = lines.next().unwrap().split_whitespace().collect::<Vec<_>>();
        assert_eq!(t[6], "cm");
        for (token, raw) in t[..6].iter().zip(matrix) {
            assert_eq!(fixed(token), i128::from(raw));
        }
        assert_eq!(
            lines.next().unwrap(),
            format!("/BI{} Do", u.resource_index())
        );
        assert_eq!(lines.next(), Some("Q"));
        assert_eq!(lines.next(), None);
        for forbidden in ["BDC", "BMC", "/MCID", "/Alt", "/ActualText", "BT"] {
            assert!(!s.contains(forbidden));
        }
        facts.image_commands += 1;
    }
    assert_eq!(offset, images.bytes().len());
    assert!(text.for_paint(usize::MAX).is_none());
    assert!(images.for_paint(usize::MAX).is_none());
    facts
}

fn input_paints(
    case: &str,
    input: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
) -> PaintFacts {
    let mut budget = BookV2ColumnPageBudget::new(caps, MAXIMUM);
    paint_drive(input, caps, MODE, &mut budget, 4, |bundle, _, observed| {
        let text = bundle.text();
        let images = bundle.images();
        assert!(std::ptr::eq(text.display(), bundle.display()));
        let prefix = bundle.rasters().source().prior_charges();
        assert!(
            prefix[0] >= text.record_charge()
                && prefix[1] >= text.spool_charge()
                && prefix[2] >= text.work_steps()
        );
        assert!(images.source().source().prior_output_charge() >= text.output_charge());
        assert_eq!(images.source().first_object(), text.source().next_object());
        let facts = inspect_paints(text, images);
        paint_probes::emit(
            case,
            text,
            images,
            caps,
            Some(observed),
            "paint_driver",
            MAXIMUM,
            bundle.constructor_charges(),
            bundle.reservation_sizes(),
            Some(bundle.caller_before()),
        );
        facts
    })
    .unwrap_or_else(|e| panic!("{case}: {e:?}"))
}
fn manual_paints_with_scope(
    case: &str,
    observed: Option<BookV2PdfConvergenceObservation>,
    scope: &str,
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    caps: &M4EffectiveResourceLimits,
) -> PaintFacts {
    let mut constructors = [[0; 4]; 8];
    let mut fonts = BookV2ColumnFontSelectionBuilder::new(display, caps, MAXIMUM, 0, 0, 0).unwrap();
    let units = fonts.reservation_sizes();
    constructors[0] = [
        fonts.record_charge(),
        fonts.spool_charge(),
        0,
        fonts.work_steps(),
    ];
    let selection = fonts.build().unwrap();
    let closed = fonts.prepare_font_closures(&selection).unwrap();
    let programs = fonts.write_font_programs(&closed).unwrap();
    let cids = fonts.plan_cids(&programs).unwrap();
    let mut b = BookV2ColumnFontStreamBuilder::new(&cids, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[1] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let streams = b.build().unwrap();
    let mut b = BookV2ColumnFontObjectBuilder::new(&streams, 4, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[2] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let objects = b.build().unwrap();
    let mut b = BookV2ColumnTextCommandBuilder::new(&objects, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[3] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let text = b.build().unwrap();
    let mut image_builder = fonts
        .next_stage_after_counted(
            text.record_charge(),
            text.spool_charge(),
            text.work_steps(),
            &mut 0,
            &mut 0,
            &mut 0,
        )
        .unwrap();
    constructors[4] = [
        image_builder.record_charge(),
        image_builder.spool_charge(),
        text.output_charge(),
        image_builder.work_steps(),
    ];
    let images = image_builder.select_images().unwrap();
    let rasters = image_builder.write_raster_programs(&images).unwrap();
    let mut b = BookV2ColumnVectorProgramBuilder::new(
        &rasters,
        caps,
        MAXIMUM,
        0,
        0,
        text.output_charge(),
        0,
    )
    .unwrap();
    constructors[5] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let vectors = b.build().unwrap();
    let mut b = BookV2ColumnImageObjectBuilder::new(
        &vectors,
        objects.next_object(),
        caps,
        MAXIMUM,
        0,
        0,
        0,
        0,
    )
    .unwrap();
    constructors[6] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let objects = b.build().unwrap();
    let mut b = BookV2ColumnImageCommandBuilder::new(&objects, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[7] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let commands = b.build().unwrap();
    let facts = inspect_paints(&text, &commands);
    paint_probes::emit(
        case,
        &text,
        &commands,
        caps,
        observed,
        scope,
        MAXIMUM,
        constructors,
        units,
        None,
    );
    facts
}

#[test]
fn book_v2_column_paints_encode_actual_lists_notes_balanced_and_named_geometry() {
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
        let facts = input_paints(&format!("paint-list-notes-{balanced}"), &input, &caps);
        assert!(facts.text_commands > 0 && facts.rules > 0);
        assert!(facts.columns.contains(&None) && facts.columns.contains(&Some(0)));
    }
    for blanks in [1, 2] {
        for balanced in [false, true] {
            let root = Root::new();
            let input = display_input(
                &root,
                named_display_data(
                    "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro",
                    blanks,
                    balanced,
                ),
                None,
                &caps,
            );
            let facts = input_paints(&format!("paint-named-{blanks}-{balanced}"), &input, &caps);
            assert!(facts.columns.contains(&Some(0)) && facts.columns.contains(&Some(1)));
            assert_eq!(facts.columns.contains(&Some(2)), blanks == 1);
        }
    }
}

#[test]
fn book_v2_column_paints_encode_original_vector_native_numbers_png_jpeg_svg() {
    let caps = display_caps();
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        for balanced in [false, true] {
            let mut data=super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,false,false);
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
            let facts = input_paints(&format!("paint-media-{kind}-{balanced}"), &input, &caps);
            assert!(facts.text_commands > 0);
            assert_eq!(facts.image_commands > 0, kind != "native");
            if kind == "native" {
                assert!(facts.sizes.contains(&629146) && facts.sizes.contains(&786432));
            }
        }
    }
}

#[test]
fn book_v2_column_paints_encode_actual_nested_header_slots_and_balanced_six_stream_page() {
    let caps = display_caps();
    for vector in [false, true] {
        for notes in [false, true] {
            let mut data = math_terminals::header_data(vector, notes, true);
            anchors(&mut data);
            balance(&mut data, !notes);
            let root = Root::new();
            let input = if vector {
                vector_input(&root, data, &caps)
            } else {
                crate::book_v2_resources::tests::shaping_tests::native_tests::native_input(
                    &root, data, &caps,
                )
            };
            let facts = input_paints(&format!("paint-header-{vector}-{notes}"), &input, &caps);
            assert!(facts.repeated > 0);
            assert_eq!(facts.columns.contains(&None), notes);
        }
    }
    let root = Root::new();
    let input = vector_input(&root, balanced_six_stream_data(), &caps);
    let facts = input_paints("paint-balanced-six-stream-notes", &input, &caps);
    assert!(facts.image_commands > 0 && facts.rules > 0 && facts.columns.contains(&None));
}

#[test]
fn book_v2_column_paints_encode_repeated_only_gid71_from_same_retained_source_cut() {
    with_retained_cut_display(|display, caps| {
        let facts = manual_paints_with_scope(
            "paint-contextual-retained-cut",
            None,
            "retained_source_cut",
            display,
            caps,
        );
        assert!(facts.repeated > 0 && facts.image_commands > 0);
    });
}

fn fraction_data(repeated: bool) -> Value {
    let mut data = crate::book_v2_resources::tests::shaping_tests::native_tests::native_data();
    let mut source = b"Result \\frac{x}{1} Proof\\frac{x}{2}".to_vec();
    let inline_end = 7 + b"\\frac{x}{1}".len();
    let display_start = inline_end + 6;
    fn rebase(v: &mut Value, inline_end: usize, display_start: usize, end: usize) {
        match v {
            Value::Object(o) => {
                for (key, v) in o {
                    if matches!(key.as_str(), "start_byte" | "end_byte") {
                        *v = match v.as_u64().unwrap() {
                            12 => inline_end.into(),
                            18 => display_start.into(),
                            21 => end.into(),
                            n => n.into(),
                        };
                    } else {
                        rebase(v, inline_end, display_start, end);
                    }
                }
            }
            Value::Array(a) => {
                for v in a {
                    rebase(v, inline_end, display_start, end)
                }
            }
            _ => (),
        }
    }
    rebase(&mut data, inline_end, display_start, source.len());
    data["text_buffers"][0]["utf8"] = std::str::from_utf8(&source).unwrap().into();
    data["document"]["blocks"][0]["blocks"][0]["children"][1]["speech"] = "x over one".into();
    data["document"]["blocks"][0]["blocks"][1]["speech"] = "x over two".into();
    if repeated {
        let paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
        let mut display = data["document"]["blocks"][0]["blocks"][1].clone();
        let span = |start, end| json!({"source_id":0,"start_byte":start,"end_byte":end});
        let text_span = |start, end| json!({"text_id":0,"start_byte":start,"end_byte":end});
        let row = |block: Value| {
            let span = block["span"].clone();
            json!({"node_id":0,"span":span,"cells":[{"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[block]}]})
        };
        source = b"\\frac{x}{2}".to_vec();
        display["span"] = span(0, source.len());
        display["math_source"]["text_span"] = text_span(0, source.len());
        let mapping = |start, end| json!({"kind":"identity","source_span":span(start,end),"text_range":{"start_byte":start,"end_byte":end}});
        let mut mappings = vec![mapping(0, source.len())];
        let mut rows = Vec::new();
        for _ in 0..12 {
            let start = source.len();
            source.extend_from_slice(b"Result \\frac{x}{1} Proof");
            let mut block = paragraph.clone();
            block["span"] = span(start, source.len());
            for (i, (left, right)) in [
                (start, start + 7),
                (start + 7, start + 18),
                (start + 18, source.len()),
            ]
            .into_iter()
            .enumerate()
            {
                let child = &mut block["children"][i];
                child["span"] = span(left, right);
                if i == 1 {
                    child["math_source"]["text_span"] = text_span(left, right);
                } else {
                    child["text_span"] = text_span(left, right);
                }
                mappings.push(mapping(left, right));
            }
            rows.push(row(block));
        }
        data["text_buffers"][0]["utf8"] = std::str::from_utf8(&source).unwrap().into();
        data["text_buffers"][0]["mappings"] = mappings.into();
        let all = span(0, source.len());
        data["document"]["blocks"][0]["span"] = all.clone();
        data["document"]["blocks"][0]["blocks"] = json!([{"kind":"table","node_id":0,"span":all,"classes":[],"columns":[{"kind":"fraction","weight":1}],"head":[row(display)],"body":rows}]);
    }
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        master["width"] = 22_000_000.into();
        master["height"] = 24_000_000.into();
        master["trim"] = json!({"x":0,"y":0,"width":22_000_000,"height":24_000_000});
        master["body"] = json!({"x":500_000,"y":500_000,"width":20_000_000,"height":if repeated{6_000_000}else{20_000_000}});
        master["column_layout"] =
            json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"none"});
    }
    anchors(&mut data);
    data
}

#[test]
fn book_v2_column_paints_encode_actual_fraction_rules_and_each_native_font_size() {
    let caps = display_caps();
    for repeated in [false, true] {
        let data = fraction_data(repeated);
        let text = data["text_buffers"][0]["utf8"]
            .as_str()
            .unwrap()
            .as_bytes()
            .to_vec();
        let root = Root::new();
        let input = native_input_with_source(&root, data, &text, &caps);
        let facts = input_paints(&format!("paint-fraction-{repeated}"), &input, &caps);
        assert!(facts.rules > 0 && facts.text_commands > facts.rules);
        assert_eq!(facts.repeated > 0, repeated);
    }
}

#[test]
fn book_v2_column_paints_encode_ambiguous_ligature_and_multiple_substitution_actual_text() {
    const FONT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/vmb-book/cid-font/extraction.ttf"
    ));
    const TEXT: &str = "ABA fi X D E f i";
    let caps = display_caps();
    let mut data = plain_data(TEXT, false, false);
    anchors(&mut data);
    let root = Root::new();
    let input = display_input(&root, data, Some(FONT), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    paint_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            let cids = bundle.cids();
            let uses = cids.uses();
            let text = |i: usize| match uses[i].source().usage().text() {
                BookV2FontUseText::Text(s) => s,
                BookV2FontUseText::Scalar(_) => panic!("authored source cluster"),
            };
            let letters = (0..uses.len())
                .filter(|i| matches!(text(*i), "A" | "B"))
                .collect::<Vec<_>>();
            assert!(letters.len() >= 3);
            let shared = cids.cids(letters[0]).unwrap();
            for i in letters {
                assert_eq!(cids.cids(i).unwrap(), shared);
                assert!(uses[i].requires_actual_text());
            }
            let ligature = (0..uses.len()).find(|i| text(*i) == "fi").unwrap();
            assert_eq!(cids.cids(ligature).unwrap().len(), 1);
            assert!(uses[ligature].requires_actual_text());
            let multiple = (0..uses.len()).find(|i| text(*i) == "X").unwrap();
            assert_eq!(cids.cids(multiple).unwrap().len(), 2);
            assert!(uses[multiple].requires_actual_text());
            let facts = inspect_paints(bundle.text(), bundle.images());
            assert!(facts.actual_text > 0);
            paint_probes::emit(
                "paint-real-gsub-extraction",
                bundle.text(),
                bundle.images(),
                &caps,
                Some(observed),
                "paint_driver",
                MAXIMUM,
                bundle.constructor_charges(),
                bundle.reservation_sizes(),
                Some(bundle.caller_before()),
            );
        },
    )
    .unwrap();
}

#[derive(Debug)]
struct EncodedAttempt {
    constructor: [u64; 4],
    after: [u64; 4],
    fingerprint: Option<[u8; 32]>,
    error: Option<String>,
}

#[test]
fn book_v2_column_paints_every_encoded_stage_honors_exact_one_short_all_four_caps() {
    let caps = display_caps();
    for kind in ["vector", "png", "jpeg"] {
        let data =
            super::super::super::super::super::super::super::super::block_widths::column_block_data(
                kind, false, false,
            );
        let root = Root::new();
        let input = if kind == "vector" {
            vector_input(&root, data, &caps)
        } else {
            figure_input(&root, data, if kind == "png" { PNG } else { JPEG }, &caps)
        };
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        paint_drive(&input,&caps,MODE,&mut budget,4,|bundle,_,_| {
            macro_rules! stage {($name:literal,$builder:ident,$source:expr,$output:expr $(,$first:expr)?)=>{{
                let source=$source;let base=[source.record_charge(),source.spool_charge(),$output,source.work_steps()];
                let run=|prior:[u64;4],maximum:u64| {
                    let(mut r,mut s,mut o,mut w)=(0,0,0,0);
                    let made=$builder::new_counted(source,$($first,)? &caps,maximum,prior[0],prior[1],prior[2],prior[3],&mut r,&mut s,&mut o,&mut w);
                    let constructor=[r,s,o,w];
                    let mut builder=match made {Ok(b)=>b,Err(e)=>return EncodedAttempt{constructor,after:constructor,fingerprint:None,error:Some(format!("{e:?}"))}};
                    let result=builder.build();let fingerprint=result.as_ref().ok().map(|s|s.fingerprint());let error=result.err().map(|e|format!("{e:?}"));
                    EncodedAttempt{constructor,after:[builder.record_charge(),builder.spool_charge(),builder.output_charge(),builder.work_steps()],fingerprint,error}
                };
                let baseline=run([0;4],MAXIMUM);assert!(baseline.error.is_none(),"{kind} {}: {baseline:?}",$name);
                for (axis,cap,error) in [(0,caps.base().get().max_fragments,"Records"),(1,caps.base().get().max_spool_bytes,"Spool"),(2,caps.base().get().max_output_bytes,"Output")] {
                    let delta=baseline.after[axis]-base[axis];
                    if delta==0 {continue;}
                    let mut prior=[0;4];prior[axis]=cap-delta;assert!(prior[axis]>=base[axis]);
                    let exact=run(prior,MAXIMUM);assert!(exact.error.is_none(),"{kind} {} axis{axis}: {exact:?}",$name);assert_eq!(exact.after[axis],cap);assert_eq!(exact.fingerprint,baseline.fingerprint);
                    prior[axis]+=1;let short=run(prior,MAXIMUM);assert!(short.error.as_deref().is_some_and(|e|e.contains(error)),"{kind} {} axis{axis}: {short:?}",$name);assert!(short.after[axis]<=cap);
                }
                let exact=run([0;4],baseline.after[3]);assert!(exact.error.is_none());assert_eq!(exact.fingerprint,baseline.fingerprint);
                let short=run([0;4],baseline.after[3]-1);assert!(short.error.as_deref().is_some_and(|e|e.contains("Work")));assert!(short.after[3]<=baseline.after[3]-1);
                assert!(baseline.constructor[0]>base[0]);
            }};}
            let font_objects=bundle.text().source();let streams=font_objects.source();let cids=streams.source();
            stage!("font streams",BookV2ColumnFontStreamBuilder,cids,0);
            stage!("font objects",BookV2ColumnFontObjectBuilder,streams,streams.output_charge(),font_objects.first_object());
            stage!("text commands",BookV2ColumnTextCommandBuilder,font_objects,font_objects.output_charge());
            let objects=bundle.images().source();let vectors=objects.source();let rasters=vectors.source();
            // Raster roots do not have an output counter. Vector programs
            // start from the actual text output, tested separately below.
            stage!("image objects",BookV2ColumnImageObjectBuilder,vectors,vectors.output_charge(),objects.first_object());
            stage!("image commands",BookV2ColumnImageCommandBuilder,objects,objects.output_charge());
            let base=[rasters.record_charge(),rasters.spool_charge(),bundle.text().output_charge(),rasters.work_steps()];
            let run=|prior:[u64;4],maximum:u64|{let(mut r,mut s,mut o,mut w)=(0,0,0,0);let made=BookV2ColumnVectorProgramBuilder::new_counted(rasters,&caps,maximum,prior[0],prior[1],prior[2].max(base[2]),prior[3],&mut r,&mut s,&mut o,&mut w);let constructor=[r,s,o,w];let mut b=match made{Ok(b)=>b,Err(e)=>return EncodedAttempt{constructor,after:constructor,fingerprint:None,error:Some(format!("{e:?}"))}};let result=b.build();let fingerprint=result.as_ref().ok().map(|s|s.fingerprint());let error=result.err().map(|e|format!("{e:?}"));EncodedAttempt{constructor,after:[b.record_charge(),b.spool_charge(),b.output_charge(),b.work_steps()],fingerprint,error}};
            let baseline=run([0;4],MAXIMUM);assert!(baseline.error.is_none());
            for (axis,cap,error) in [(0,caps.base().get().max_fragments,"Records"),(1,caps.base().get().max_spool_bytes,"Spool"),(2,caps.base().get().max_output_bytes,"Output")]{let delta=baseline.after[axis]-base[axis];if delta==0{continue;}let mut prior=[0;4];prior[axis]=cap-delta;let exact=run(prior,MAXIMUM);assert!(exact.error.is_none());assert_eq!(exact.after[axis],cap);assert_eq!(exact.fingerprint,baseline.fingerprint);prior[axis]+=1;let short=run(prior,MAXIMUM);assert!(short.error.as_deref().is_some_and(|e|e.contains(error)));assert!(short.after[axis]<=cap);}
            assert!(run([0;4],baseline.after[3]).error.is_none());assert!(run([0;4],baseline.after[3]-1).error.as_deref().is_some_and(|e|e.contains("Work")));
        }).unwrap();
    }
}

// Original SafeSVG2 admission used by the existing Body image regression.
// The extra rectangle repeats one alpha pair, so states are deduplicated while
// every actual draw, clip, transform, cubic and quadratic remains present.
const PAINT_CURVES: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="48pt" height="32pt" viewBox="-2 -1 24 16"><defs><clipPath id="c"><path d="M 0 0 L 19 0 L 19 12 L 0 12 Z"/></clipPath></defs><g clip-path="url(#c)" transform="translate(1 1)"><path d="M 1 3 Q 5 0 8 4 C 10 12 12 1 16 7 L 2 10 Z" fill="#4080c0" fill-opacity="0.5" stroke="#c08040" stroke-opacity="0.75" stroke-width="0.75" fill-rule="evenodd"/><path d="M 3 4 Q 8 1 14 8" fill="none" stroke="currentColor" stroke-width="0.5"/><rect x="2" y="2" width="3" height="4" fill="#4080c0" fill-opacity="0.5" stroke="#c08040" stroke-opacity="0.75" stroke-width="0.75"/></g></svg>"##;

#[test]
fn book_v2_column_paints_encode_clip_curves_color_alpha_and_equal_payload_occurrence_ids() {
    let caps = display_caps();
    for (kind, bytes) in [("png", PNG), ("jpeg", JPEG), ("svg", PAINT_CURVES)] {
        let mut data =
            super::super::super::super::super::super::super::super::block_widths::column_block_data(
                kind, false, false,
            );
        data["resources"]["images"][0]["expected_sha256"] =
            probes::hex(&typaxis_core::sha256(bytes)).into();
        let mut alias = data["resources"]["images"][0].clone();
        alias["image_id"] = 1.into();
        data["resources"]["images"]
            .as_array_mut()
            .unwrap()
            .push(alias);
        let blocks = data["document"]["blocks"][0]["blocks"]
            .as_array_mut()
            .unwrap();
        let i = blocks.iter().position(|b| b["kind"] == "figure").unwrap();
        blocks[i]["image_id"] = 1.into();
        let mut second = blocks[i].clone();
        second["image_id"] = 0.into();
        blocks.push(second);
        anchors(&mut data);
        let root = Root::new();
        let input = figure_input(&root, data, bytes, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        paint_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, _, observed| {
                let images = bundle.rasters().source();
                assert_eq!(images.images().len(), 1);
                assert!(images.uses().len() > 1);
                let ids = images
                    .uses()
                    .iter()
                    .map(|u| u.usage().image().image_id().get())
                    .collect::<BTreeSet<_>>();
                assert_eq!(ids, BTreeSet::from([0, 1]));
                assert!(images.uses().iter().all(|u| u.resource_index() == 0));
                assert!(!std::ptr::eq(
                    input
                        .resources()
                        .image(typaxis_core::ImageResourceId::new(0))
                        .unwrap(),
                    input
                        .resources()
                        .image(typaxis_core::ImageResourceId::new(1))
                        .unwrap()
                ));
                assert_eq!(bundle.images().commands().len(), images.uses().len());
                if kind == "svg" {
                    let vector = bundle.images().source().source().program(0).unwrap();
                    let typaxis_resources::AdmittedSafeVector::V2(ir) = vector.ir() else {
                        panic!("original SafeSVG2")
                    };
                    assert_eq!(ir.draws().len(), 3);
                    assert_eq!(vector.states().len(), 2);
                    let s = std::str::from_utf8(vector.content()).unwrap();
                    assert!(
                        s.contains("W n")
                            && s.contains(" c\n")
                            && s.contains("f*\n")
                            && s.contains("S\n")
                            && !s.contains("B*")
                    );
                    assert!(
                        s.lines()
                            .any(|l| l.ends_with(" cm") && l != "1 0 0 1 0 0 cm")
                    );
                    assert!(s.contains(" rg\n") && s.contains(" RG\n"));
                }
                inspect_paints(bundle.text(), bundle.images());
                paint_probes::emit(
                    &format!("paint-alias-curves-{kind}"),
                    bundle.text(),
                    bundle.images(),
                    &caps,
                    Some(observed),
                    "paint_driver",
                    MAXIMUM,
                    bundle.constructor_charges(),
                    bundle.reservation_sizes(),
                    Some(bundle.caller_before()),
                );
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_column_paints_preserve_empty_forced_and_zero_table_pages_without_commands() {
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
            let facts = input_paints(&format!("paint-zero-{table}-{balanced}"), &input, &caps);
            assert_eq!(
                (
                    facts.text_commands,
                    facts.image_commands,
                    facts.rules,
                    facts.actual_text
                ),
                (0, 0, 0, 0)
            );
            assert!(facts.columns.is_empty());
        }
    }
}

#[test]
fn book_v2_column_paints_preserve_original_rowspan_nested_caption_and_forced_table_sources() {
    let caps = display_caps();
    for (mode, height, balanced) in [
        ("span", 256, false),
        ("nested-body-span", 544, false),
        ("nested-body-caption", 512, true),
        ("forced", 256, false),
    ] {
        let mut data =
            super::super::super::super::super::data(false, mode, "Pro Pro Pro Pro Pro Pro Pro");
        for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
            master["body"]["height"] = (height * 65536).into();
        }
        anchors(&mut data);
        balance(&mut data, balanced);
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let facts = input_paints(&format!("paint-table-{mode}-{balanced}"), &input, &caps);
        assert!(facts.text_commands > 0);
    }
}

fn source_owners(v: &Value, set: &mut BTreeSet<u32>) {
    if let Some(n) = v.get("node_id").and_then(Value::as_u64) {
        set.insert(n as u32);
    }
    match v {
        Value::Array(a) => {
            for v in a {
                source_owners(v, set)
            }
        }
        Value::Object(o) => {
            for v in o.values() {
                source_owners(v, set)
            }
        }
        _ => (),
    }
}
#[test]
fn book_v2_column_paints_never_encode_unreferenced_rich_definition_owners() {
    let caps = display_caps();
    let mut data =
        super::super::super::super::super::super::super::super::block_widths::column_block_data(
            "vector", true, false,
        );
    let mut unused = data["document"]["footnotes"][0].clone();
    unused["footnote_id"] = "paint-unused-rich".into();
    data["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(unused);
    anchors(&mut data);
    let mut excluded = BTreeSet::new();
    source_owners(&data["document"]["footnotes"][1], &mut excluded);
    let root = Root::new();
    let input = vector_input(&root, data, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    paint_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            let display = bundle.display();
            assert_eq!(display.source().source().unreferenced_definitions(), 1);
            assert!(
                display
                    .source()
                    .original_graph()
                    .lines()
                    .paragraphs()
                    .iter()
                    .any(|p| excluded.contains(&p.owner().get()))
            );
            for c in bundle.text().commands() {
                let owner = match display.paints()[c.paint_index()] {
                    Paint::Text(i) => Some(display.text().draws()[i].owner()),
                    Paint::Marker(i) => Some(display.markers().draws()[i].source().owner()),
                    Paint::EquationNumber(i) => {
                        Some(display.numbers().draws()[i].placement().geometry().owner())
                    }
                    Paint::Math(i) => Some(display.math().draws()[i].terminal().source().owner()),
                    Paint::FootnoteSeparator(_) => None,
                    _ => panic!("image entered text commands"),
                };
                if let Some(owner) = owner {
                    assert!(!excluded.contains(&owner.get()));
                }
            }
            assert!(
                bundle
                    .images()
                    .commands()
                    .iter()
                    .all(|c| !excluded.contains(&c.owner().get()))
            );
            assert!(
                display
                    .anchors()
                    .positions()
                    .iter()
                    .all(|a| !excluded.contains(&a.anchor().source().owner().get()))
            );
            assert!(
                display
                    .source()
                    .source()
                    .geometry()
                    .pages()
                    .iter()
                    .flat_map(|p| p.fragments())
                    .all(|f| f.definition_index() != Some(1))
            );
            inspect_paints(bundle.text(), bundle.images());
            paint_probes::emit(
                "paint-unused-rich",
                bundle.text(),
                bundle.images(),
                &caps,
                Some(observed),
                "paint_driver",
                MAXIMUM,
                bundle.constructor_charges(),
                bundle.reservation_sizes(),
                Some(bundle.caller_before()),
            );
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_paints_counted_constructors_preserve_caps_identity_and_object_scan_prefixes() {
    let caps = display_caps();
    let root = Root::new();
    let input = vector_input(
        &root,
        super::super::super::super::super::super::super::super::block_widths::column_block_data(
            "vector", false, false,
        ),
        &caps,
    );
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    paint_drive(&input,&caps,MODE,&mut budget,4,|bundle,_,_| {
        let changed=changed_caps(&caps,|c|c.max_fragments+=1);
        macro_rules! constructors {($builder:ident,$source:expr,$output:expr $(,$first:expr)?)=>{{
            let source=$source;let base=[source.record_charge(),source.spool_charge(),$output,source.work_steps()];
            let run=|limits:&M4EffectiveResourceLimits,maximum:u64,prior:[u64;4]|{let(mut r,mut s,mut o,mut w)=(0,0,0,0);let made=$builder::new_counted(source,$($first,)? limits,maximum,prior[0],prior[1],prior[2],prior[3],&mut r,&mut s,&mut o,&mut w);(made.is_ok(),made.err().map(|e|format!("{e:?}")),[r,s,o,w])};
            let baseline=run(&caps,MAXIMUM,[0;4]);assert!(baseline.0);assert_eq!(baseline.2[0],base[0]+1);
            let identity=run(&changed,MAXIMUM,[0;4]);assert!(identity.1.as_deref().is_some_and(|e|e.contains("Identity")));assert_eq!(identity.2,base);
            let record_cap=caps.base().get().max_fragments;let exact=run(&caps,MAXIMUM,[record_cap-1,0,0,0]);assert!(exact.0);assert_eq!(exact.2[0],record_cap);let short=run(&caps,MAXIMUM,[record_cap,0,0,0]);assert!(short.1.as_deref().is_some_and(|e|e.contains("Records")));assert_eq!(short.2[0],record_cap);
            for (axis,cap,error) in [(1,caps.base().get().max_spool_bytes,"Spool"),(2,caps.base().get().max_output_bytes,"Output")] {let mut prior=[0;4];prior[axis]=cap;let exact=run(&caps,MAXIMUM,prior);assert!(exact.0);assert_eq!(exact.2[axis],cap);prior[axis]+=1;let short=run(&caps,MAXIMUM,prior);assert!(short.1.as_deref().is_some_and(|e|e.contains(error)));assert_eq!(short.2[axis],cap+1);assert_eq!(short.2[0],base[0]+1);}
            let exact=run(&caps,baseline.2[3],[0;4]);assert!(exact.0);assert_eq!(exact.2,baseline.2);
            let short=run(&caps,baseline.2[3]-1,[0;4]);assert!(short.1.as_deref().is_some_and(|e|e.contains("Work")));assert_eq!(short.2[0],base[0]+1);if baseline.2[3]==base[3]{assert_eq!(short.2[3],base[3]);}else{assert!(short.2[3]<=baseline.2[3]-1);}
        }};}
        let fonts=bundle.text().source();let streams=fonts.source();let cids=streams.source();constructors!(BookV2ColumnFontStreamBuilder,cids,0);constructors!(BookV2ColumnFontObjectBuilder,streams,streams.output_charge(),fonts.first_object());constructors!(BookV2ColumnTextCommandBuilder,fonts,fonts.output_charge());
        let images=bundle.images().source();let vectors=images.source();let rasters=vectors.source();constructors!(BookV2ColumnVectorProgramBuilder,rasters,0);constructors!(BookV2ColumnImageObjectBuilder,vectors,vectors.output_charge(),images.first_object());constructors!(BookV2ColumnImageCommandBuilder,images,images.output_charge());
        for first in [0,u32::MAX,caps.base().get().max_pdf_objects] {
            let(mut r,mut s,mut o,mut w)=(0,0,0,0);let result=BookV2ColumnImageObjectBuilder::new_counted(vectors,first,&caps,MAXIMUM,0,0,0,0,&mut r,&mut s,&mut o,&mut w);assert!(matches!(result,Err(typaxis_pdf::book_v2::BookV2FontStreamError::Objects)));assert_eq!((r,s,o),(vectors.record_charge()+1,vectors.spool_charge(),vectors.output_charge()));
            let expected_scan=rasters.source().images().iter().enumerate().map(|(i,_)|1+vectors.program(i).map_or(0,|v|v.states().len() as u64)).sum::<u64>();assert_eq!(w,vectors.work_steps()+expected_scan);
            let result=BookV2ColumnFontObjectBuilder::new_counted(streams,first,&caps,MAXIMUM,0,0,0,0,&mut r,&mut s,&mut o,&mut w);assert!(matches!(result,Err(typaxis_pdf::book_v2::BookV2FontStreamError::Objects)));assert!(r>=streams.record_charge()+1&&w>=streams.work_steps());
        }
        // Constructor scan consumes its accepted work before reaching the
        // object-number refusal. No builder exists to capture this prefix.
        let(mut r,mut s,mut o,mut w)=(0,0,0,0);let result=BookV2ColumnImageObjectBuilder::new_counted(vectors,0,&caps,vectors.work_steps()+1,0,0,0,0,&mut r,&mut s,&mut o,&mut w);assert!(matches!(result,Err(typaxis_pdf::book_v2::BookV2FontStreamError::Work)));assert_eq!(w,vectors.work_steps()+1);assert_eq!(r,vectors.record_charge()+1);
    }).unwrap();
}

#[test]
fn book_v2_column_paints_repeated_builds_and_failures_preserve_vector_credit_once() {
    let caps = display_caps();
    let root = Root::new();
    let input = vector_input(
        &root,
        super::super::super::super::super::super::super::super::block_widths::column_block_data(
            "vector", false, false,
        ),
        &caps,
    );
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    paint_drive(&input,&caps,MODE,&mut budget,4,|bundle,_,_| {
        macro_rules! repeats {($builder:ident,$source:expr,$credit:expr $(,$first:expr)?)=>{{
            let source=$source;let mut b=$builder::new(source,$($first,)? &caps,MAXIMUM,0,0,0,0).unwrap();let ctor=[b.record_charge(),b.spool_charge(),b.output_charge(),b.work_steps()];let first=b.build().unwrap();let a=[b.record_charge(),b.spool_charge(),b.output_charge(),b.work_steps()];let fp=first.fingerprint();drop(first);let second=b.build().unwrap();assert_eq!(second.fingerprint(),fp);let z=[b.record_charge(),b.spool_charge(),b.output_charge(),b.work_steps()];for axis in [0,1,3]{assert_eq!(z[axis]-a[axis],a[axis]-ctor[axis],"{} axis{axis}: ctor={ctor:?} first={a:?} second={z:?}",stringify!($builder));}assert_eq!(z[2]-a[2],a[2]-ctor[2]+$credit);drop(second);
            let maximum=a[3]-1;let mut b=$builder::new(source,$($first,)? &caps,maximum,0,0,0,0).unwrap();assert!(matches!(b.build(),Err(typaxis_pdf::book_v2::BookV2FontStreamError::Work)));let prefix=[b.record_charge(),b.spool_charge(),b.output_charge(),b.work_steps()];assert!(prefix[0]>ctor[0]&&prefix[1]>ctor[1]);assert_eq!(prefix[2],a[2],"{} failed prefix={prefix:?} first={a:?}",stringify!($builder));assert!(b.build().is_err());let retry=[b.record_charge(),b.spool_charge(),b.output_charge(),b.work_steps()];for axis in 0..4{assert!(retry[axis]>=prefix[axis]);}assert!(retry[3]<=maximum);
        }};}
        let objects=bundle.text().source();let streams=objects.source();let cids=streams.source();repeats!(BookV2ColumnFontStreamBuilder,cids,0);let font_credit=(0..streams.fonts().len()).map(|i|streams.to_unicode(i).unwrap().len()+streams.auxiliary(i).unwrap().len()).sum::<usize>() as u64;assert!(font_credit>0);repeats!(BookV2ColumnFontObjectBuilder,streams,font_credit,objects.first_object());repeats!(BookV2ColumnTextCommandBuilder,objects,0);
        let objects=bundle.images().source();let vectors=objects.source();let rasters=vectors.source();repeats!(BookV2ColumnVectorProgramBuilder,rasters,0);
        let credit=(0..rasters.source().images().len()).filter_map(|i|vectors.program(i)).map(|v|v.content().len()+(0..v.states().len()).map(|i|v.state_dictionary(i).unwrap().len()).sum::<usize>()).sum::<usize>() as u64;assert!(credit>0);repeats!(BookV2ColumnImageObjectBuilder,vectors,credit,objects.first_object());repeats!(BookV2ColumnImageCommandBuilder,objects,0);
        // An exact first output budget includes the reusable Form payload
        // credit. A second build cannot claim those bytes a second time.
        let mut warm=BookV2ColumnImageObjectBuilder::new(vectors,objects.first_object(),&caps,MAXIMUM,0,0,0,0).unwrap();let warm_ctor=warm.output_charge();let first=warm.build().unwrap();let delta=first.output_charge()-warm_ctor;assert_eq!(delta,first.bytes().len() as u64-credit);drop(first);
        let cap=caps.base().get().max_output_bytes;let mut b=BookV2ColumnImageObjectBuilder::new(vectors,objects.first_object(),&caps,MAXIMUM,0,0,cap-delta,0).unwrap();assert_eq!(b.build().unwrap().output_charge(),cap);let prefix=[b.record_charge(),b.spool_charge(),b.output_charge(),b.work_steps()];assert!(matches!(b.build(),Err(typaxis_pdf::book_v2::BookV2FontStreamError::Output)));assert_eq!(b.output_charge(),cap);assert!(b.record_charge()>=prefix[0]&&b.spool_charge()>=prefix[1]&&b.work_steps()>=prefix[3]);
    }).unwrap();
}

fn paint_observation(o: BookV2PdfConvergenceObservation) -> [u64; 4] {
    [
        o.record_charge(),
        o.spool_charge(),
        o.output_charge(),
        o.work_steps(),
    ]
}
#[test]
fn book_v2_column_paints_driver_retains_exact_all_caps_and_failed_callback_retry() {
    let caps = display_caps();
    let data = plain_data("Pro", true, false);
    let root = Root::new();
    let input = display_input(&root, data.clone(), None, &caps);
    let mut warm = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut fingerprint = None;
    paint_drive(&input, &caps, MODE, &mut warm, 4, |bundle, _, observed| {
        fingerprint = Some(inspect_paints(bundle.text(), bundle.images()));
        paint_probes::emit(
            "paint-driver-warm",
            bundle.text(),
            bundle.images(),
            &caps,
            Some(observed),
            "paint_driver",
            MAXIMUM,
            bundle.constructor_charges(),
            bundle.reservation_sizes(),
            Some(bundle.caller_before()),
        );
    })
    .unwrap();
    let total = paint_observation(warm.observation());
    for axis in 0..4 {
        for short in [false, true] {
            let cap = total[axis] - u64::from(short);
            let limits = match axis {
                0 => changed_caps(&caps, |c| c.max_fragments = cap),
                1 => {
                    let mut b = caps.base().get().clone();
                    b.max_spool_bytes = cap;
                    let mut e = caps.extension().get().clone();
                    e.max_font_subset_bytes = e.max_font_subset_bytes.min(cap);
                    M4EffectiveResourceLimits::new(ValidatedResourceLimits::new(b).unwrap(), e)
                        .unwrap()
                }
                2 => changed_caps(&caps, |c| c.max_output_bytes = cap),
                3 => caps.clone(),
                _ => unreachable!(),
            };
            let maximum = if axis == 3 { cap } else { MAXIMUM };
            let root = Root::new();
            let input = display_input(&root, data.clone(), None, &limits);
            let mut budget = BookV2ColumnPageBudget::new(&limits, maximum);
            let mut delivered = false;
            let result = paint_drive(
                &input,
                &limits,
                MODE,
                &mut budget,
                4,
                |bundle, _, observed| {
                    delivered = true;
                    let facts = inspect_paints(bundle.text(), bundle.images());
                    let baseline = fingerprint.as_ref().unwrap();
                    assert_eq!(
                        (
                            facts.text_commands,
                            facts.image_commands,
                            facts.rules,
                            facts.actual_text,
                            &facts.sizes
                        ),
                        (
                            baseline.text_commands,
                            baseline.image_commands,
                            baseline.rules,
                            baseline.actual_text,
                            &baseline.sizes
                        )
                    );
                    paint_probes::emit(
                        &format!("paint-driver-exact-{axis}"),
                        bundle.text(),
                        bundle.images(),
                        &limits,
                        Some(observed),
                        "paint_driver",
                        maximum,
                        bundle.constructor_charges(),
                        bundle.reservation_sizes(),
                        Some(bundle.caller_before()),
                    );
                },
            );
            if short {
                assert!(result.is_err(), "axis{axis}: {result:?}");
                assert!(!delivered);
                let prefix = paint_observation(budget.observation());
                assert!(
                    paint_drive(&input, &limits, MODE, &mut budget, 4, |_, _, _| delivered =
                        true)
                    .is_err()
                );
                assert!(!delivered);
                let after = paint_observation(budget.observation());
                for i in 0..4 {
                    assert!(after[i] >= prefix[i]);
                }
            } else {
                assert!(result.is_ok(), "axis{axis}: {result:?}");
                assert!(delivered);
            }
            let actual = paint_observation(budget.observation())[axis];
            assert!(actual <= cap, "axis{axis} short{short}: {actual}>{cap}");
            if !short {
                assert_eq!(actual, cap);
            }
        }
    }
    let callback_root = Root::new();
    let input = vector_input(
        &callback_root,
        super::super::super::super::super::super::super::super::block_widths::column_block_data(
            "vector", false, false,
        ),
        &caps,
    );
    let mut failed = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut callback_before = [0; 4];
    let mut callback_after = [0; 4];
    let result = paint_drive(
        &input,
        &caps,
        MODE,
        &mut failed,
        4,
        |bundle, builder, observed| {
            callback_before = [
                builder.record_charge(),
                builder.spool_charge(),
                builder.output_charge(),
                builder.work_steps(),
            ];
            let again = builder.build().unwrap();
            assert_eq!(again.fingerprint(), bundle.images().fingerprint());
            callback_after = [
                builder.record_charge(),
                builder.spool_charge(),
                builder.output_charge(),
                builder.work_steps(),
            ];
            paint_probes::emit(
                "paint-driver-callback-before-repeat",
                bundle.text(),
                bundle.images(),
                &caps,
                Some(observed),
                "paint_driver",
                MAXIMUM,
                bundle.constructor_charges(),
                bundle.reservation_sizes(),
                Some(bundle.caller_before()),
            );
            Err::<(), _>("paint callback sentinel")
        },
    );
    assert_eq!(result.unwrap(), Err("paint callback sentinel"));
    for axis in 0..4 {
        assert!(callback_after[axis] >= callback_before[axis]);
    }
    assert!(
        callback_after[0] > callback_before[0]
            && callback_after[1] > callback_before[1]
            && callback_after[3] > callback_before[3]
    );
    assert_eq!(paint_observation(failed.observation()), callback_after);
    paint_drive(
        &input,
        &caps,
        MODE,
        &mut failed,
        4,
        |bundle, _, observed| {
            assert_eq!(bundle.caller_before(), callback_after);
            inspect_paints(bundle.text(), bundle.images());
            paint_probes::emit(
                "paint-driver-retry-retained-prefix",
                bundle.text(),
                bundle.images(),
                &caps,
                Some(observed),
                "paint_driver",
                MAXIMUM,
                bundle.constructor_charges(),
                bundle.reservation_sizes(),
                Some(bundle.caller_before()),
            );
        },
    )
    .unwrap();
    assert!(
        failed.observation().record_charge() > callback_after[0]
            && failed.observation().work_steps() > callback_after[3]
    );
    let mut budget = BookV2ColumnPageBudget::new(&caps, callback_after[3] - 1);
    let mut delivered = false;
    paint_drive(&input, &caps, MODE, &mut budget, 4, |_, builder, _| {
        delivered = true;
        assert!(matches!(
            builder.build(),
            Err(typaxis_pdf::book_v2::BookV2FontStreamError::Work)
        ));
        let prefix = [
            builder.record_charge(),
            builder.spool_charge(),
            builder.output_charge(),
            builder.work_steps(),
        ];
        assert!(builder.build().is_err());
        for (a, b) in [
            builder.record_charge(),
            builder.spool_charge(),
            builder.output_charge(),
            builder.work_steps(),
        ]
        .into_iter()
        .zip(prefix)
        {
            assert!(a >= b);
        }
    })
    .unwrap();
    assert!(delivered);
    assert!(budget.observation().record_charge() >= callback_before[0]);
    assert!(budget.observation().work_steps() <= callback_after[3] - 1);
}

#[test]
fn book_v2_column_paints_driver_rejects_foreign_limits_and_invalid_font_object_numbers() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, plain_data("Pro", false, false), None, &caps);
    let changed = changed_caps(&caps, |c| c.max_fragments += 1);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let before = paint_observation(budget.observation());
    let mut delivered = false;
    assert!(
        paint_drive(&input, &changed, MODE, &mut budget, 4, |_, _, _| {
            delivered = true
        })
        .is_err()
    );
    assert!(!delivered);
    assert_eq!(paint_observation(budget.observation()), before);
    for first in [0, u32::MAX, caps.base().get().max_pdf_objects] {
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let mut delivered = false;
        let result = paint_drive(&input, &caps, MODE, &mut budget, first, |_, _, _| {
            delivered = true
        });
        assert!(result.is_err());
        assert!(!delivered);
        assert!(
            budget.observation().record_charge() > 0
                && budget.observation().spool_charge() > 0
                && budget.observation().work_steps() > 0
        );
        let prefix = paint_observation(budget.observation());
        assert!(
            paint_drive(&input, &caps, MODE, &mut budget, first, |_, _, _| {
                delivered = true
            })
            .is_err()
        );
        assert!(!delivered);
        for (a, b) in paint_observation(budget.observation())
            .into_iter()
            .zip(prefix)
        {
            assert!(a >= b);
        }
    }
}

#[test]
#[ignore = "requires TYPAXIS_HARANO_FONT pointing to original HaranoAjiMincho-Regular.otf SHA256 66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"]
fn book_v2_column_paints_original_harano_cid_cff_commands() {
    let bytes =
        fs::read(std::env::var_os("TYPAXIS_HARANO_FONT").expect("original font path")).unwrap();
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
    paint_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            let streams = bundle.text().source().source();
            assert!(
                streams
                    .fonts()
                    .iter()
                    .all(|f| f.auxiliary_kind() == BookV2FontAuxiliaryKind::CffCidSet)
            );
            let facts = inspect_paints(bundle.text(), bundle.images());
            assert!(facts.text_commands > 0);
            for i in 0..streams.fonts().len() {
                let bindings = streams.source().bindings(i).unwrap();
                let set = streams.auxiliary(i).unwrap();
                assert!(bindings.iter().all(|b| set[b.cid().get() as usize / 8]
                    & (1 << (7 - b.cid().get() % 8))
                    != 0));
            }
            paint_probes::emit(
                "paint-original-harano-cid-cff",
                bundle.text(),
                bundle.images(),
                &caps,
                Some(observed),
                "paint_driver",
                MAXIMUM,
                bundle.constructor_charges(),
                bundle.reservation_sizes(),
                Some(bundle.caller_before()),
            );
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_paints_manual_source_callback_keeps_actual_native_and_vector_parents() {
    let caps = display_caps();
    for kind in ["native", "vector"] {
        let data =
            super::super::super::super::super::super::super::super::block_widths::column_block_data(
                kind, true, true,
            );
        let root = Root::new();
        let input = if kind == "native" {
            let source = data["text_buffers"][0]["utf8"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec();
            native_input_with_source(&root, data, &source, &caps)
        } else {
            vector_input(&root, data, &caps)
        };
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        math_drive(&input, &caps, MODE, &mut budget, |source, _, observed| {
            let mut db =
                BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                    .unwrap();
            let display = db.build().unwrap();
            let facts = manual_paints_with_scope(
                &format!("paint-manual-source-{kind}"),
                Some(observed),
                "source_callback",
                &display,
                &caps,
            );
            assert!(facts.text_commands > 0 && facts.columns.contains(&None));
            assert_eq!(facts.image_commands > 0, kind == "vector");
        })
        .unwrap();
    }
}
