use super::*;
use crate::book_v2_resources::with_budgeted_book_v2_column_math as math_drive;
use typaxis_display_list::book_v2::{
    BookV2ColumnDisplay, BookV2ColumnDisplayBuilder, BookV2ColumnPaintIndex as Paint,
    BookV2ImagePaint, BookV2ImageSource, BookV2MarkerPlacement, BookV2MarkerSource,
    BookV2MathDisplayError, BookV2MathPaint,
};
use typaxis_display_list::{
    ProductionBodyDisplayErrorKind as DisplayError, ProductionNativeMathPaint,
};
use typaxis_layout::{
    PrecomposedVectorPlacementInput, ProductionFigureMedia, ProductionPlacedInline,
};
use typaxis_pagination::book_v2::{BookV2BodyMathSource, BookV2ColumnMathTerminals};
use typaxis_resources::VectorContentKey;
use typaxis_shaping::ShapeSourceSpan;
use typaxis_syntax::PrecomposedVectorKind;

#[path = "book_v2_column_display_probes.rs"]
mod probes;
#[path = "book_v2_column_resource_tests.rs"]
mod resource_tests;

const DISPLAY_LIST_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/vmb-book/list-fonts/body-list-visible.ttf"
));

#[derive(Clone, Debug, PartialEq)]
struct DisplayFacts {
    fingerprint: [u8; 32],
    text: usize,
    vector_math: usize,
    native_math: usize,
    numbers: usize,
    markers: usize,
    separators: usize,
    images: usize,
    anchors: usize,
    nonpainting: usize,
    repeated: usize,
    notes: usize,
    balanced: usize,
    columns: BTreeSet<u16>,
    pages: usize,
}

fn display_caps() -> M4EffectiveResourceLimits {
    changed_caps(&caps(), |c| {
        c.max_layout_passes = 512;
        c.max_line_reshape_passes = 512;
        c.max_footnote_reflows_per_page = 512;
        c.max_column_balance_candidates = 128;
    })
}

fn display_input(
    root: &Root,
    mut data: Value,
    font: Option<&[u8]>,
    caps: &M4EffectiveResourceLimits,
) -> PreparedBookV2Resources {
    let text = data["text_buffers"][0]["utf8"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    if let Some(font) = font {
        data["resources"]["font_faces"][0]["media_type"] = if font.starts_with(b"OTTO") {
            "sfnt-cff1"
        } else {
            "sfnt-truetype-glyf"
        }
        .into();
        data["resources"]["font_faces"][0]["expected_sha256"] =
            probes::hex(&typaxis_core::sha256(font)).into();
    }
    let body = body_with_source(root, data, &text, caps);
    if let Some(font) = font {
        fs::write(root.0.join("body.bin"), font).unwrap();
    }
    // Fixtures retain their authored image identities. These are the exact
    // checked-in resources named by the existing original-source fixtures.
    fs::write(root.0.join("vector.svg"), SVG).unwrap();
    prepare_book_v2_resources(
        body,
        &root.context(),
        &config_with_extension(caps.base().get().clone(), caps.extension().get().clone()),
        caps,
    )
    .unwrap()
}

fn balance(data: &mut Value, enabled: bool) {
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        master["column_layout"]["balance"] = if enabled { "last_page" } else { "none" }.into();
    }
}

fn renumber(data: &mut Value) {
    crate::book_v2_resources::tests::shaping_tests::native_tests::header_terminals::renumber(
        &mut data["document"],
        &mut 0,
    );
}

fn anchors(data: &mut Value) {
    fn visit(value: &mut Value, serial: &mut usize) {
        if value["kind"] == "paragraph" {
            let id = *serial;
            *serial += 1;
            let span = value["span"].clone();
            value["children"].as_array_mut().unwrap().insert(0, json!({"kind":"anchor","node_id":0,"span":span,"anchor_id":format!("column-display-{id}")}));
            return;
        }
        match value {
            Value::Array(items) => {
                for item in items {
                    visit(item, serial);
                }
            }
            Value::Object(items) => {
                for (key, item) in items {
                    if key != "span" {
                        visit(item, serial);
                    }
                }
            }
            _ => (),
        }
    }
    visit(&mut data["document"], &mut 0);
    renumber(data);
}

// Every expectation below starts from selected original graphs and source
// placements. Component arrays are consumed once in that source order.
fn inspect(display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>) -> DisplayFacts {
    let source = display.source();
    for component in [
        display.math().source(),
        display.text().source(),
        display.numbers().source(),
        display.markers().source(),
        display.images().source(),
        display.anchors().source(),
    ] {
        assert!(std::ptr::eq(source, component));
    }
    for component in [
        display.math().admitted(),
        display.text().admitted(),
        display.numbers().admitted(),
        display.markers().admitted(),
        display.images().admitted(),
    ] {
        assert!(std::ptr::eq(display.admitted(), component));
    }
    let mut facts = DisplayFacts {
        fingerprint: display.fingerprint(),
        text: 0,
        vector_math: 0,
        native_math: 0,
        numbers: 0,
        markers: 0,
        separators: 0,
        images: 0,
        anchors: 0,
        nonpainting: 0,
        repeated: 0,
        notes: 0,
        balanced: 0,
        columns: BTreeSet::new(),
        pages: source.source().geometry().pages().len(),
    };
    let mut paints = Vec::new();
    let mut text = display.text().draws().iter();
    let mut math = display.math().draws().iter();
    let mut images = display.images().draws().iter();
    let mut marker = display.markers().draws().iter();
    let mut number = display.numbers().draws().iter();
    let mut anchor = display.anchors().positions().iter();
    let mut empty = display.anchors().nonpainting_lines().iter();
    let mut separators = display.markers().separators().iter();
    let mut fragment_index = 0usize;
    for page in source.source().geometry().pages() {
        facts.balanced += usize::from(page.selection().balance().is_some());
        for (local_index, (placed, cell, repeated)) in page.fragments_with_roles().enumerate() {
            let fragment = placed.fragment();
            let graph = source.fragment_graph(fragment_index).unwrap();
            let lines = graph.lines();
            let prepared = lines.prepared();
            let parsed = prepared
                .source_flow()
                .body()
                .body()
                .wire()
                .text_buffers()
                .len() as u32;
            assert!(std::ptr::eq(
                lines,
                source.fragment_lines(fragment_index).unwrap()
            ));
            assert_eq!(
                display.fragment_column(fragment_index),
                page.fragment_column(local_index)
            );
            if let Some(column) = page.fragment_column(local_index) {
                facts.columns.insert(column);
            }
            facts.notes += usize::from(placed.definition_index().is_some());
            facts.repeated += usize::from(repeated);
            if page.separator_ink().is_some()
                && placed.definition_index().is_some()
                && !page.fragments()[..local_index]
                    .iter()
                    .any(|f| f.definition_index().is_some())
            {
                let draw = separators.next().expect("original note separator missing");
                assert_eq!(draw.page_index(), page.selection().page_index());
                assert_eq!(draw.before_fragment_index(), fragment_index);
                assert_eq!(draw.ink(), page.separator_ink().unwrap());
                paints.push(Paint::FootnoteSeparator(facts.separators));
                facts.separators += 1;
            }
            // Note labels precede list labels sharing an original fragment.
            let mut original_markers = Vec::new();
            original_markers.extend(
                page.footnote_markers()
                    .iter()
                    .filter(|m| m.fragment_index() as usize == local_index)
                    .map(BookV2MarkerPlacement::Footnote),
            );
            original_markers.extend(
                page.list_markers()
                    .iter()
                    .filter(|m| m.fragment_index() as usize == local_index)
                    .map(BookV2MarkerPlacement::List),
            );
            for placement in original_markers {
                let draw = marker.next().expect("original source marker missing");
                assert_eq!(draw.fragment_index(), fragment_index);
                assert!(std::ptr::eq(draw.fragment(), placed));
                assert_eq!(draw.cell_role(), cell);
                assert_eq!(draw.repeated_header(), repeated);
                let shaped = prepared.shaped();
                match (draw.placement(), placement, draw.source()) {
                    (
                        BookV2MarkerPlacement::List(a),
                        BookV2MarkerPlacement::List(b),
                        BookV2MarkerSource::List(shape),
                    ) => {
                        assert!(std::ptr::eq(a, b));
                        assert!(std::ptr::eq(
                            shape,
                            &shaped.list_markers()[b.marker_index() as usize]
                        ));
                    }
                    (
                        BookV2MarkerPlacement::Footnote(a),
                        BookV2MarkerPlacement::Footnote(b),
                        BookV2MarkerSource::Footnote(shape),
                    ) => {
                        assert!(std::ptr::eq(a, b));
                        assert!(std::ptr::eq(
                            shape,
                            &shaped.footnote_markers()[b.definition_index()]
                        ));
                    }
                    _ => panic!("source marker kind or owner changed"),
                }
                let shape = draw.source();
                assert_eq!(shape.advance(), placement.bounds().width());
                assert_eq!(draw.clusters().len(), shape.glyph_run().clusters.len());
                for (index, (paint, cluster)) in draw
                    .clusters()
                    .iter()
                    .zip(&shape.glyph_run().clusters)
                    .enumerate()
                {
                    assert_eq!(paint.cluster_index(), index as u32);
                    let ShapeSourceSpan::Generated(p) = cluster.source_span else {
                        panic!("source label became authored text")
                    };
                    assert_eq!(paint.provenance(), p);
                    assert_eq!(
                        paint.text_span().text_id().get(),
                        parsed + p.text_span().text_id().get()
                    );
                    assert_eq!(paint.text_span().range(), p.text_span().range());
                    let run = shape.glyph_run();
                    let start = cluster.glyph_start as usize;
                    let end = cluster.glyph_end as usize;
                    assert_eq!(paint.glyphs().len(), end - start);
                    for (ordinal, (glyph, original)) in paint
                        .glyphs()
                        .iter()
                        .zip(&run.glyphs[start..end])
                        .enumerate()
                    {
                        assert_eq!(glyph.original_gid(), original.original_gid);
                        assert_eq!(
                            glyph.x().raw(),
                            placement.bounds().x().raw()
                                + run.glyphs[..start + ordinal]
                                    .iter()
                                    .map(|g| g.advance_x.raw())
                                    .sum::<i64>()
                                + original.offset_x.raw()
                        );
                        assert_eq!(
                            glyph.y().raw(),
                            placement.baseline().raw()
                                - run.glyphs[..start + ordinal]
                                    .iter()
                                    .map(|g| g.advance_y.raw())
                                    .sum::<i64>()
                                - original.offset_y.raw()
                        );
                    }
                }
                assert_eq!(
                    draw.clusters()
                        .iter()
                        .map(|c| c.exact_text())
                        .collect::<String>(),
                    shape.utf8()
                );
                paints.push(Paint::Marker(facts.markers));
                facts.markers += 1;
            }
            if let Source::ParagraphLine {
                paragraph_index,
                line_index,
            } = fragment.source()
            {
                let paragraph = &lines.paragraphs()[paragraph_index as usize];
                assert_eq!(paragraph.owner(), fragment.owner());
                let line = &paragraph.lines()[line_index as usize];
                if line
                    .items()
                    .iter()
                    .all(|item| matches!(item, ProductionPlacedInline::Break(_)))
                {
                    let draw = empty
                        .next()
                        .expect("actual nonpainting source line missing");
                    assert!(std::ptr::eq(draw.fragment(), placed));
                    assert_eq!(draw.fragment_index(), fragment_index);
                    assert_eq!(draw.cell_role(), cell);
                    assert_eq!(draw.repeated_header(), repeated);
                    facts.nonpainting += 1;
                }
                for a in paragraph
                    .anchors()
                    .iter()
                    .filter(|a| a.position().is_some_and(|p| p.line_index() == line_index))
                {
                    let draw = anchor.next().expect("source anchor missing");
                    assert!(std::ptr::eq(draw.anchor(), a));
                    assert!(std::ptr::eq(draw.fragment(), placed));
                    assert_eq!(draw.fragment_index(), fragment_index);
                    assert_eq!(draw.cell_role(), cell);
                    assert_eq!(draw.repeated_header(), repeated);
                    let position = a.position().unwrap();
                    assert_eq!(
                        draw.x().raw(),
                        fragment.bounds().x().raw() + position.x().raw()
                    );
                    assert_eq!(draw.baseline(), fragment.baseline().unwrap());
                    facts.anchors += 1;
                }
                for (inline, item) in line.items().iter().enumerate() {
                    match item {
                        ProductionPlacedInline::Text(cluster) => {
                            let draw = text.next().expect("selected original text cluster missing");
                            let font = paragraph.font().unwrap();
                            assert!(std::ptr::eq(draw.cluster(), cluster));
                            assert!(std::ptr::eq(draw.font(), font));
                            assert_eq!(
                                (
                                    draw.page_index(),
                                    draw.fragment_index(),
                                    draw.inline_index()
                                ),
                                (fragment.page_index(), fragment_index, inline as u32)
                            );
                            assert_eq!(
                                (
                                    draw.definition_index(),
                                    draw.item_index(),
                                    draw.cell_owner(),
                                    draw.repeated_header()
                                ),
                                (
                                    placed.definition_index(),
                                    placed.item_index(),
                                    cell.map(|c| c.owner()),
                                    repeated
                                )
                            );
                            assert_eq!(draw.owner(), cluster.run().owner());
                            assert_eq!(draw.exact_text(), cluster.utf8());
                            let face = display.admitted().font(font.face_id()).unwrap();
                            assert_eq!(
                                (face.content_hash(), face.face_index()),
                                (font.content_hash(), font.face_index())
                            );
                            let (id, start, end) = match cluster.source_span() {
                                ShapeSourceSpan::Parsed(s) => {
                                    assert_eq!(draw.generated_provenance(), None);
                                    (s.text_id().get(), s.start_byte(), s.end_byte())
                                }
                                ShapeSourceSpan::Generated(p) => {
                                    assert_eq!(draw.generated_provenance(), Some(p));
                                    let s = p.text_span();
                                    (
                                        parsed + s.text_id().get(),
                                        s.range().start_byte(),
                                        s.range().end_byte(),
                                    )
                                }
                            };
                            assert_eq!(draw.text_span().text_id().get(), id);
                            assert_eq!(
                                (
                                    draw.text_span().range().start_byte(),
                                    draw.text_span().range().end_byte()
                                ),
                                (start, end)
                            );
                            assert_eq!(draw.glyphs().len(), cluster.glyphs().len());
                            for (paint, glyph) in draw.glyphs().iter().zip(cluster.glyphs()) {
                                assert_eq!(paint.original_gid(), glyph.glyph().original_gid);
                                assert_eq!(
                                    paint.x().raw(),
                                    fragment.bounds().x().raw() + glyph.x().raw()
                                );
                                assert_eq!(
                                    paint.y().raw(),
                                    fragment.bounds().y().raw() + glyph.y().raw()
                                );
                            }
                            let width = cluster
                                .glyphs()
                                .iter()
                                .map(|g| g.glyph().advance_x.raw())
                                .sum::<i64>();
                            let height = font.ascender().raw() - font.descender().raw();
                            if width > 0 && height > 0 {
                                let bounds = draw.logical_bounds().unwrap();
                                assert_eq!(
                                    bounds.x().raw(),
                                    fragment.bounds().x().raw() + cluster.pen_x().raw()
                                );
                                assert_eq!(
                                    bounds.y().raw(),
                                    fragment.baseline().unwrap().raw() - font.ascender().raw()
                                );
                                assert_eq!(
                                    (bounds.width().get().raw(), bounds.height().get().raw()),
                                    (width, height)
                                );
                            } else {
                                assert_eq!(draw.logical_bounds(), None);
                            }
                            paints.push(Paint::Text(facts.text));
                            facts.text += 1;
                        }
                        ProductionPlacedInline::Vector(selected) => {
                            let binding = prepared
                                .vector_bindings()
                                .unwrap()
                                .receipt(selected.occurrence().item().node_id())
                                .unwrap();
                            if binding.kind() == PrecomposedVectorKind::InlineVector {
                                let draw =
                                    images.next().expect("non-formula inline vector missing");
                                check_image(
                                    draw,
                                    BookV2ImageSource::Vector(binding),
                                    placed,
                                    fragment_index,
                                    Some(inline as u32),
                                    cell,
                                    repeated,
                                    display.admitted(),
                                );
                                let viewport = draw.paint().viewport();
                                let local = selected.geometry().viewport();
                                assert_eq!(
                                    (viewport.x().raw(), viewport.y().raw()),
                                    (
                                        fragment.bounds().x().raw() + local.x().raw(),
                                        fragment.bounds().y().raw() + local.y().raw()
                                    )
                                );
                                paints.push(Paint::Image(facts.images));
                                facts.images += 1;
                            } else {
                                let draw = math.next().expect("original inline formula missing");
                                check_math(
                                    draw,
                                    source,
                                    fragment_index,
                                    Some(inline as u32),
                                    display.admitted(),
                                    &mut facts,
                                );
                                paints.push(Paint::Math(facts.vector_math + facts.native_math - 1));
                            }
                        }
                        ProductionPlacedInline::BookV2Math(_) => {
                            let draw = math.next().expect("original native inline formula missing");
                            check_math(
                                draw,
                                source,
                                fragment_index,
                                Some(inline as u32),
                                display.admitted(),
                                &mut facts,
                            );
                            paints.push(Paint::Math(facts.vector_math + facts.native_math - 1));
                        }
                        ProductionPlacedInline::Break(_) => (),
                        ProductionPlacedInline::Math(_) => {
                            panic!("legacy native math cannot replace a column Book /2 receipt")
                        }
                    }
                }
            } else {
                match fragment.source() {
                    Source::Figure { figure_index } => {
                        let original = &prepared.figures()[figure_index as usize];
                        let draw = images.next().expect("original raster/SVG figure missing");
                        check_image(
                            draw,
                            BookV2ImageSource::Figure(original),
                            placed,
                            fragment_index,
                            None,
                            cell,
                            repeated,
                            display.admitted(),
                        );
                        assert_eq!(
                            (
                                draw.paint().viewport().width(),
                                draw.paint().viewport().height()
                            ),
                            (original.width(), original.height())
                        );
                        paints.push(Paint::Image(facts.images));
                        facts.images += 1;
                    }
                    Source::VectorBlock { block_index } => {
                        let binding =
                            graph.blocks().unwrap().blocks()[block_index as usize].binding();
                        if binding.kind() == PrecomposedVectorKind::VectorFigure {
                            let draw = images.next().expect("original vector figure missing");
                            check_image(
                                draw,
                                BookV2ImageSource::Vector(binding),
                                placed,
                                fragment_index,
                                None,
                                cell,
                                repeated,
                                display.admitted(),
                            );
                            paints.push(Paint::Image(facts.images));
                            facts.images += 1;
                        } else {
                            let draw = math.next().expect("original vector block formula missing");
                            check_math(
                                draw,
                                source,
                                fragment_index,
                                None,
                                display.admitted(),
                                &mut facts,
                            );
                            paints.push(Paint::Math(facts.vector_math + facts.native_math - 1));
                        }
                    }
                    Source::NativeMathBlock { .. } => {
                        let draw = math.next().expect("original native block formula missing");
                        check_math(
                            draw,
                            source,
                            fragment_index,
                            None,
                            display.admitted(),
                            &mut facts,
                        );
                        paints.push(Paint::Math(facts.vector_math + facts.native_math - 1));
                    }
                    _ => unreachable!(),
                }
            }
            for placement in page
                .equation_numbers()
                .iter()
                .filter(|n| n.geometry().fragment_index() as usize == fragment_index)
            {
                let draw = number
                    .next()
                    .expect("original independent equation number missing");
                assert!(std::ptr::eq(draw.placement(), placement));
                let geometry = placement.geometry();
                let shape = graph
                    .blocks()
                    .unwrap()
                    .numbers()
                    .unwrap()
                    .shape(geometry.parent_owner())
                    .unwrap();
                assert!(std::ptr::eq(draw.shape(), shape));
                assert_eq!(shape.fingerprint(), geometry.shape_fingerprint());
                assert_eq!(
                    draw.clusters()
                        .iter()
                        .map(|c| c.exact_text())
                        .collect::<String>(),
                    shape.text()
                );
                check_number(draw);
                assert!(matches!(paints.last(), Some(Paint::Math(_))));
                paints.push(Paint::EquationNumber(facts.numbers));
                facts.numbers += 1;
            }
            fragment_index += 1;
        }
    }
    assert_eq!(fragment_index, source.fragment_count());
    assert_eq!(display.fragment_column(fragment_index), None);
    assert!(
        text.next().is_none()
            && math.next().is_none()
            && images.next().is_none()
            && marker.next().is_none()
            && number.next().is_none()
            && anchor.next().is_none()
            && empty.next().is_none()
            && separators.next().is_none(),
        "display invented or reordered a source occurrence"
    );
    assert_eq!(display.paints(), paints);
    assert_eq!(display.math().draws().len(), source.terminals().len());
    let unpositioned = source
        .original_graph()
        .lines()
        .paragraphs()
        .iter()
        .flat_map(|p| p.anchors())
        .filter(|a| a.position().is_none())
        .collect::<Vec<_>>();
    assert_eq!(display.anchors().unpositioned().len(), unpositioned.len());
    for (draw, original) in display.anchors().unpositioned().iter().zip(unpositioned) {
        assert!(std::ptr::eq(*draw, original));
    }
    facts
}

fn check_image(
    draw: &typaxis_display_list::book_v2::BookV2ImageDraw<'_, '_, '_, '_>,
    original: BookV2ImageSource<'_, '_>,
    placed: &typaxis_pagination::ProductionBodyFootnotePlacedFragment,
    index: usize,
    inline: Option<u32>,
    cell: Option<typaxis_pagination::ProductionTablePlacedCellRole>,
    repeated: bool,
    admitted: &AdmittedProductionResourceLedgerV3,
) {
    assert!(std::ptr::eq(draw.fragment(), placed));
    assert_eq!(
        (
            draw.fragment_index(),
            draw.inline_index(),
            draw.cell_role(),
            draw.repeated_header()
        ),
        (index, inline, cell, repeated)
    );
    assert_eq!(draw.source().alternative(), original.alternative());
    let (id, vector) = match (draw.source(), original) {
        (BookV2ImageSource::Figure(a), BookV2ImageSource::Figure(b)) => {
            assert!(std::ptr::eq(a, b));
            (
                b.source().image_id(),
                matches!(b.media(), ProductionFigureMedia::Svg { .. }),
            )
        }
        (BookV2ImageSource::Vector(a), BookV2ImageSource::Vector(b)) => {
            assert!(std::ptr::eq(a, b));
            (b.resource().image_id(), true)
        }
        _ => panic!("ordinary image source kind changed"),
    };
    assert!(std::ptr::eq(draw.image(), admitted.image(id).unwrap()));
    assert_eq!(
        vector,
        matches!(draw.paint(), BookV2ImagePaint::Vector { .. })
    );
    if let BookV2ImagePaint::Vector {
        content_key,
        viewport,
        scale_raw,
        matrix,
        ..
    } = draw.paint()
    {
        assert_eq!(
            content_key,
            VectorContentKey::from_admitted(draw.image()).unwrap()
        );
        assert_eq!(
            (
                matrix.a.raw(),
                matrix.b.raw(),
                matrix.c.raw(),
                matrix.d.raw()
            ),
            (scale_raw, 0, 0, scale_raw)
        );
        assert_eq!((matrix.e, matrix.f), (viewport.x(), viewport.y()));
    }
}

fn check_math(
    draw: &typaxis_display_list::book_v2::BookV2ColumnMathDraw<'_, '_, '_>,
    source: &BookV2ColumnMathTerminals<'_, '_, '_, '_, '_, '_, '_>,
    fragment: usize,
    inline: Option<u32>,
    admitted: &AdmittedProductionResourceLedgerV3,
    facts: &mut DisplayFacts,
) {
    let terminal = source
        .terminals()
        .iter()
        .find(|t| t.fragment_index() == fragment && t.inline_index() == inline)
        .unwrap();
    assert!(std::ptr::eq(draw.terminal(), terminal));
    match (draw.paint(), terminal.source()) {
        (BookV2MathPaint::Vector(paint), BookV2BodyMathSource::Vector(binding)) => {
            assert_eq!(
                paint.content_key(),
                VectorContentKey::from_admitted(
                    admitted.image(binding.resource().image_id()).unwrap()
                )
                .unwrap()
            );
            assert_eq!(paint.viewport(), terminal.viewport().unwrap());
            let (scale, color) = match binding.placement() {
                PrecomposedVectorPlacementInput::Inline(p) => (p.scale().get().raw(), p.paint()),
                PrecomposedVectorPlacementInput::MathVectorBlock(p) => {
                    (p.scale().get().raw(), p.paint())
                }
                _ => panic!("ordinary vector received formula paint"),
            };
            assert_eq!(
                (paint.scale_raw(), paint.color()),
                (scale, [color.red(), color.green(), color.blue()])
            );
            let matrix = paint.matrix();
            assert_eq!(
                (
                    matrix.a.raw(),
                    matrix.b.raw(),
                    matrix.c.raw(),
                    matrix.d.raw()
                ),
                (scale, 0, 0, scale)
            );
            assert_eq!(
                (matrix.e, matrix.f),
                (paint.viewport().x(), paint.viewport().y())
            );
            assert_eq!(
                draw.alternative(),
                binding.source().alternative().alternative()
            );
            assert_eq!(
                draw.actual_text(),
                binding.source().alternative().resolved_actual_text()
            );
            facts.vector_math += 1;
        }
        (BookV2MathPaint::Native(paint), BookV2BodyMathSource::Native(receipt)) => {
            let (left, top, right, bottom) = receipt.computation().dimensions().bbox();
            assert_eq!(
                (
                    paint.bounds().x().raw(),
                    paint.bounds().y().raw(),
                    paint.bounds().width().get().raw(),
                    paint.bounds().height().get().raw()
                ),
                (
                    terminal.origin_x().raw() + left,
                    terminal.baseline().raw() + top,
                    right - left,
                    bottom - top
                )
            );
            assert_eq!(paint.paints().len(), receipt.computation().paints().len());
            for (actual, original) in paint.paints().iter().zip(receipt.computation().paints()) {
                match (actual, original) {
                    (
                        ProductionNativeMathPaint::Glyph {
                            original_gid,
                            unicode,
                            logical_ordinal,
                            x,
                            y,
                            font_size,
                        },
                        typaxis_math::MathPaint::Glyph(g),
                    ) => {
                        assert_eq!(
                            (*original_gid, *unicode, *logical_ordinal),
                            (g.original_gid(), g.unicode(), g.logical_ordinal())
                        );
                        assert_eq!(
                            (x.raw(), y.raw(), font_size.get().raw()),
                            (
                                terminal.origin_x().raw() + g.x(),
                                terminal.baseline().raw() + g.y(),
                                g.font_size_raw()
                            )
                        );
                    }
                    (ProductionNativeMathPaint::Rule(rect), typaxis_math::MathPaint::Rule(r)) => {
                        assert_eq!(
                            (
                                rect.x().raw(),
                                rect.y().raw(),
                                rect.width().get().raw(),
                                rect.height().get().raw()
                            ),
                            (
                                terminal.origin_x().raw() + r.x(),
                                terminal.baseline().raw() + r.y(),
                                r.width(),
                                r.height()
                            )
                        )
                    }
                    _ => panic!("native source paint kind changed"),
                }
            }
            assert_eq!(draw.alternative(), receipt.source().domain().speech);
            assert_eq!(
                draw.actual_text(),
                Some(receipt.source().domain().speech.as_str())
            );
            facts.native_math += 1;
        }
        _ => panic!("source formula paint kind changed"),
    }
}

fn check_number(draw: &typaxis_display_list::book_v2::BookV2EquationNumberDraw<'_, '_, '_>) {
    let shape = draw.shape();
    let rect = draw.placement().geometry().bounds();
    let font = draw.font();
    assert_eq!(
        (
            font.face_id(),
            font.content_hash(),
            font.face_index(),
            font.size()
        ),
        (
            shape.font_face_id(),
            shape.font_sha256(),
            shape.face_index(),
            shape.font_size()
        )
    );
    let baseline = rect.y().raw()
        + (rect.height().get().raw() - font.ascender().raw() + font.descender().raw()) / 2
        + font.ascender().raw();
    let mut paints = draw.clusters().iter();
    for (run_index, run) in shape.runs().iter().enumerate() {
        let mut left = rect.x().raw();
        for (other, r) in shape.runs().iter().enumerate() {
            if other == run_index {
                continue;
            }
            let minimum = shape.runs()[run_index.min(other)..=run_index.max(other)]
                .iter()
                .map(|r| r.bidi_level().get())
                .min()
                .unwrap();
            if (minimum % 2 == 0 && other < run_index) || (minimum % 2 != 0 && other > run_index) {
                left += r.glyphs().iter().map(|g| g.advance_x.raw()).sum::<i64>();
            }
        }
        for (cluster_index, cluster) in run.clusters().iter().enumerate() {
            let paint = paints.next().unwrap();
            assert_eq!(
                (paint.run_index(), paint.cluster_index()),
                (run_index as u32, cluster_index as u32)
            );
            let ShapeSourceSpan::Parsed(span) = cluster.source_span else {
                panic!("number lost authored span")
            };
            assert_eq!(paint.text_span().text_id().get(), span.text_id().get());
            assert_eq!(
                (
                    paint.text_span().range().start_byte(),
                    paint.text_span().range().end_byte()
                ),
                (span.start_byte(), span.end_byte())
            );
            let start = cluster.glyph_start as usize;
            let end = cluster.glyph_end as usize;
            assert_eq!(paint.glyphs().len(), end - start);
            for (i, (glyph, original)) in paint
                .glyphs()
                .iter()
                .zip(&run.glyphs()[start..end])
                .enumerate()
            {
                assert_eq!(glyph.original_gid(), original.original_gid);
                assert_eq!(
                    glyph.x().raw(),
                    left + run.glyphs()[..start + i]
                        .iter()
                        .map(|g| g.advance_x.raw())
                        .sum::<i64>()
                        + original.offset_x.raw()
                );
                assert_eq!(glyph.y().raw(), baseline - original.offset_y.raw());
            }
        }
    }
    assert!(paints.next().is_none());
}

fn inspect_input(
    case: &str,
    input: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
) -> DisplayFacts {
    let mut budget = BookV2ColumnPageBudget::new(caps, 1_000_000_000);
    math_drive(input, caps, MODE, &mut budget, |source, _, observed| {
        let mut builder =
            BookV2ColumnDisplayBuilder::new(source, input.resources(), caps, 1_000_000_000, 0, 0)
                .unwrap();
        let display = builder.build().unwrap();
        display.verify_resources(input.resources(), caps).unwrap();
        assert_eq!(
            (display.record_charge(), display.work_steps()),
            (builder.record_charge(), builder.work_steps())
        );
        let facts = inspect(&display);
        probes::emit(
            case,
            &display,
            caps,
            observed,
            "source_callback",
            1_000_000_000,
            Some((0, 0)),
        );
        facts
    })
    .unwrap_or_else(|e| panic!("{case}: {e:?}"))
}

#[test]
fn book_v2_column_display_projects_original_text_notes_and_anchors_at_actual_origins() {
    let caps = display_caps();
    for balanced in [false, true] {
        let mut data = plain_data("Pro", true, false);
        anchors(&mut data);
        balance(&mut data, balanced);
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let facts = inspect_input(&format!("text-notes-anchors-{balanced}"), &input, &caps);
        assert!(
            facts.text > 0
                && facts.markers > 0
                && facts.separators > 0
                && facts.anchors > 0
                && facts.notes > 0
        );
        assert_eq!(facts.balanced > 0, balanced);
        assert!(facts.columns.contains(&0) && facts.columns.contains(&1));
    }
}

#[test]
fn book_v2_column_display_rejects_name_keyed_cff_before_any_column_pipeline() {
    let font = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/profiles/production-book-1/combined/job/typaxis-cff-fixture.otf"
    ));
    let caps = display_caps();
    for balanced in [false, true] {
        let mut data = plain_data("AB", false, false);
        anchors(&mut data);
        balance(&mut data, balanced);
        let root = Root::new();
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] =
            probes::hex(&typaxis_core::sha256(font)).into();
        let body = body_with_source(&root, data, b"AB", &caps);
        fs::write(root.0.join("body.bin"), font).unwrap();
        let result = prepare_book_v2_resources(
            body,
            &root.context(),
            &config_with_extension(caps.base().get().clone(), caps.extension().get().clone()),
            &caps,
        );
        let error = result
            .err()
            .expect("name-keyed CFF entered the CID CFF /2 column path");
        let crate::book_v2_resources::BookV2ResourceFailure::Admission(
            typaxis_resources::ProductionResourceErrorV3::Cff {
                font_face_id,
                error,
            },
        ) = error.failure()
        else {
            panic!("wrong CFF admission rejection: {}", error.failure())
        };
        assert_eq!(font_face_id.get(), 0);
        assert_eq!(
            error.kind,
            typaxis_font::Cff1FailureKindV2::Program(
                typaxis_font::CffProgramErrorKindV2::ExpectedCidFont
            )
        );
        assert_eq!(error.context.table_tag, Some(*b"CFF "));
        assert_eq!(
            (error.context.file_offset, error.context.table_offset),
            (Some(907), Some(39))
        );
    }
}

#[test]
fn book_v2_column_display_projects_native_vector_numbers_and_png_jpeg_svg_source_graphs() {
    let caps = display_caps();
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        for notes in [false, true] {
            for tables in [false, true] {
                for balanced in [false, true] {
                    let mut data =
                        super::super::super::super::super::super::block_widths::column_block_data(
                            kind, notes, tables,
                        );
                    anchors(&mut data);
                    // Each original-media graph also reaches a balanced body page;
                    // note continuations retain their independent global endpoint.
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
                    let facts = inspect_input(
                        &format!("blocks-{kind}-{notes}-{tables}-{balanced}"),
                        &input,
                        &caps,
                    );
                    assert!(facts.text > 0 && facts.anchors > 0);
                    if kind == "vector" {
                        assert!(facts.vector_math > 0 && facts.numbers > 0 && facts.images > 0);
                    } else if kind == "native" {
                        assert!(facts.native_math > 0);
                    } else {
                        assert!(facts.images > 0);
                    }
                    assert_eq!(facts.notes > 0, notes);
                    if !notes {
                        assert_eq!(facts.balanced > 0, balanced);
                    }
                }
            }
        }
    }
}

#[test]
fn book_v2_column_display_uses_actual_nested_repeated_math_header_graphs() {
    let caps = display_caps();
    for vector in [false, true] {
        for notes in [false, true] {
            for nested in [false, true] {
                for balanced in [false, true] {
                    let mut data = math_terminals::header_data(vector, notes, nested);
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
                    let facts = inspect_input(
                        &format!("headers-{vector}-{notes}-{nested}-{balanced}"),
                        &input,
                        &caps,
                    );
                    assert!(facts.repeated > 0 && facts.anchors > 0 && facts.text > 0);
                    assert_eq!(facts.notes > 0, notes);
                    assert_eq!(facts.vector_math > 0, vector);
                    assert_eq!(facts.native_math > 0, !vector);
                    if vector {
                        assert!(facts.numbers > 0);
                    }
                    // Notes-only final pages deliberately have no positive body balance.
                    assert_eq!(facts.balanced > 0, balanced && !notes);
                }
            }
        }
    }
}

#[test]
fn book_v2_column_display_retains_actual_rowspan_nested_caption_and_forced_table_text() {
    let caps = display_caps();
    let text = "Pro Pro Pro Pro Pro Pro Pro";
    for (mode, height) in [
        ("common", 256),
        ("span", 256),
        ("nested-body-span", 544),
        ("nested-body-caption", 512),
        ("forced", 256),
    ] {
        for notes in [false, true] {
            for balanced in [false, true] {
                let mut data = super::super::super::super::data(notes, mode, text);
                for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
                    master["body"]["height"] = (height * 65536).into();
                    if notes {
                        master["footnote"]["height"] = (height * 65536).into();
                    }
                }
                anchors(&mut data);
                balance(&mut data, balanced);
                let root = Root::new();
                let input = display_input(&root, data, None, &caps);
                let facts =
                    inspect_input(&format!("tables-{mode}-{notes}-{balanced}"), &input, &caps);
                assert!(facts.text > 0 && facts.anchors > 0);
                assert_eq!(facts.notes > 0, notes);
                assert_eq!(facts.balanced > 0, balanced && !notes);
                if mode == "common" {
                    assert!(facts.repeated > 0);
                }
            }
        }
    }
}

fn wrap_list(blocks: &mut Value, ordered: bool) {
    let paragraphs = blocks.as_array().unwrap().clone();
    let span = paragraphs[0]["span"].clone();
    let items = paragraphs
        .into_iter()
        .map(|p| json!({"node_id":0,"span":span,"blocks":[p]}))
        .collect::<Vec<_>>();
    *blocks = json!([{"kind":"list","node_id":0,"span":span,"classes":[],"ordered":ordered,"start":if ordered {json!(1)} else {Value::Null},"items":items}]);
}
fn list_font_rule(data: &mut Value) {
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    let mut rule = rules
        .iter()
        .find(|r| {
            r["selector"] == "paragraph"
                && r["declarations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|d| d["name"] == "font_family")
        })
        .unwrap()
        .clone();
    rule["selector"] = "list".into();
    rule["style_id"] = "column-display-list-font".into();
    rule["source_order"] = (rules
        .iter()
        .filter_map(|r| r["source_order"].as_u64())
        .max()
        .unwrap()
        + 1)
    .into();
    rules.push(rule);
}

#[test]
fn book_v2_column_display_keeps_original_list_and_definition_label_glyphs() {
    let caps = display_caps();
    for scope in ["body", "table", "notes"] {
        for balanced in [false, true] {
            let mut data = if scope == "table" {
                super::super::super::super::data(false, "common", "Pro")
            } else {
                plain_data("Pro", true, false)
            };
            if scope == "body" {
                data["document"]["blocks"]
                    .as_array_mut()
                    .unwrap()
                    .truncate(5);
                wrap_list(&mut data["document"]["blocks"], true);
            } else if scope == "table" {
                wrap_list(
                    &mut data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"],
                    false,
                );
            } else {
                wrap_list(&mut data["document"]["footnotes"][0]["blocks"], false);
            }
            list_font_rule(&mut data);
            anchors(&mut data);
            balance(&mut data, balanced);
            let root = Root::new();
            let input = display_input(&root, data, Some(DISPLAY_LIST_FONT), &caps);
            let facts = inspect_input(&format!("lists-{scope}-{balanced}"), &input, &caps);
            assert!(facts.markers >= if scope == "body" { 6 } else { 2 });
            assert!(facts.text > 0 && facts.anchors > 0);
            if scope != "table" {
                assert!(facts.notes > 0 && facts.separators > 0);
            }
        }
    }
}

#[test]
fn book_v2_column_display_keeps_empty_anchor_lines_and_forced_blank_pages_nonpainting() {
    let caps = display_caps();
    for balanced in [false, true] {
        let mut data = plain_data("Pro", false, true);
        let span = data["document"]["blocks"][0]["span"].clone();
        let forced = json!({"kind":"page_break","node_id":0,"classes":[],"span":span});
        let blocks = data["document"]["blocks"].as_array_mut().unwrap();
        blocks.insert(0, forced.clone());
        blocks.insert(0, forced.clone());
        blocks.push(forced);
        balance(&mut data, balanced);
        renumber(&mut data);
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let facts = inspect_input(&format!("empty-forced-{balanced}"), &input, &caps);
        assert_eq!(
            facts.text
                + facts.vector_math
                + facts.native_math
                + facts.numbers
                + facts.markers
                + facts.separators
                + facts.images,
            0
        );
        assert_eq!((facts.anchors, facts.nonpainting), (6, 6));
        assert!(facts.pages >= 4);
        assert_eq!(facts.balanced > 0, balanced);
    }
}

#[test]
fn book_v2_column_display_preserves_all_zero_table_source_without_inventing_paint() {
    let caps = display_caps();
    for balanced in [false, true] {
        let mut data = super::super::super::super::data(false, "common", "Pro");
        let table = &mut data["document"]["blocks"][0];
        for name in ["head", "body"] {
            for row in table.get_mut(name).unwrap().as_array_mut().unwrap() {
                for cell in row["cells"].as_array_mut().unwrap() {
                    cell["blocks"] = json!([]);
                }
            }
        }
        balance(&mut data, balanced);
        renumber(&mut data);
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let facts = inspect_input(&format!("zero-table-{balanced}"), &input, &caps);
        assert_eq!(
            facts.text
                + facts.vector_math
                + facts.native_math
                + facts.numbers
                + facts.markers
                + facts.separators
                + facts.images
                + facts.anchors
                + facts.nonpainting,
            0
        );
        assert_eq!(facts.balanced, 0);
        assert_eq!(facts.pages, 1);
    }
}

#[test]
fn book_v2_column_display_work_records_and_repeated_failure_prefix_are_exact() {
    let caps = display_caps();
    let root = Root::new();
    let mut data = math_terminals::header_data(true, false, true);
    balance(&mut data, true);
    let input = vector_input(&root, data, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, 1_000_000_000);
    math_drive(&input,&caps,MODE,&mut budget,|source,_,_| {
        let run=|prior_records,maximum,prior_work|->Result<_,BookV2MathDisplayError> {
            let mut b=BookV2ColumnDisplayBuilder::new(source,input.resources(),&caps,maximum,prior_records,prior_work)?;
            let first=b.build()?; inspect(&first);
            let fp=first.fingerprint(); let paints=first.paints().to_vec();
            let second=b.build()?;
            assert_eq!(second.fingerprint(),fp); assert_eq!(second.paints(),paints);
            assert!(second.record_charge()>first.record_charge() && second.work_steps()>first.work_steps());
            Ok((b.record_charge(),b.work_steps(),fp))
        };
        let (records,work,fp)=run(0,1_000_000_000,0).unwrap();
        let prior=caps.base().get().max_fragments-(records-source.record_charge());
        assert_eq!(run(prior,work,0).unwrap(),(caps.base().get().max_fragments,work,fp));
        assert!(matches!(run(prior+1,work,0),Err(BookV2MathDisplayError::Display(e)) if e.kind==DisplayError::RecordLimit));
        assert!(matches!(run(0,work-1,0),Err(BookV2MathDisplayError::WorkLimit(_))));
        assert_eq!(run(0,work+17,source.work_steps()+17).unwrap(),(records,work+17,fp));
        let mut failed=BookV2ColumnDisplayBuilder::new(source,input.resources(),&caps,work-1,0,0).unwrap();
        let first=failed.build().unwrap();
        let first_counters=(first.record_charge(),first.work_steps());
        assert!(matches!(failed.build(),Err(BookV2MathDisplayError::WorkLimit(_))));
        let prefix=(failed.record_charge(),failed.work_steps());
        assert!(prefix.0>=first_counters.0 && prefix.1>first_counters.1);
        assert_eq!(prefix.1,work-1);
        assert!(matches!(failed.build(),Err(BookV2MathDisplayError::WorkLimit(_))));
        assert_eq!((failed.record_charge(),failed.work_steps()),prefix);
        let mut observed_records=0; let mut observed_work=0;
        let constructor=BookV2ColumnDisplayBuilder::new_counted(source,input.resources(),&caps,1_000_000_000,0,0,&mut observed_records,&mut observed_work).unwrap();
        assert_eq!((observed_records,observed_work),(constructor.record_charge(),constructor.work_steps()));
        assert!(observed_records>source.record_charge() && observed_work>source.work_steps());
        let mut rejected_records=0; let mut rejected_work=0;
        let rejected=BookV2ColumnDisplayBuilder::new_counted(source,input.resources(),&caps,observed_work-1,0,0,&mut rejected_records,&mut rejected_work);
        assert!(matches!(rejected,Err(BookV2MathDisplayError::WorkLimit(_))));
        assert!(rejected_records>source.record_charge()); assert_eq!(rejected_work,observed_work-1);
        let mut retry_records=rejected_records; let mut retry_work=rejected_work;
        let retry=BookV2ColumnDisplayBuilder::new_counted(source,input.resources(),&caps,rejected_work,rejected_records,rejected_work,&mut retry_records,&mut retry_work);
        assert!(matches!(retry,Err(BookV2MathDisplayError::WorkLimit(_))));
        assert!(retry_records>=rejected_records); assert_eq!(retry_work,rejected_work);
    }).unwrap();
}

#[test]
fn book_v2_column_display_rejects_foreign_admission_changed_limits_and_prior_overage() {
    let caps = display_caps();
    let root = Root::new();
    let foreign_root = Root::new();
    let data = plain_data("Pro", false, false);
    let input = display_input(&root, data.clone(), None, &caps);
    let foreign = display_input(&foreign_root, data, None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, 1_000_000_000);
    math_drive(&input,&caps,MODE,&mut budget,|source,_,_| {
        let different=changed_caps(&caps,|c|c.max_layout_passes+=1);
        assert!(matches!(BookV2ColumnDisplayBuilder::new(source,input.resources(),&different,1_000_000_000,0,0),Err(BookV2MathDisplayError::Display(e)) if e.kind==DisplayError::ReceiptMismatch));
        assert!(matches!(BookV2ColumnDisplayBuilder::new(source,foreign.resources(),&caps,1_000_000_000,0,0),Err(BookV2MathDisplayError::Display(e)) if e.kind==DisplayError::ReceiptMismatch));
        assert!(matches!(BookV2ColumnDisplayBuilder::new(source,input.resources(),&caps,source.work_steps()-1,0,0),Err(BookV2MathDisplayError::WorkLimit(_))));
        assert!(matches!(BookV2ColumnDisplayBuilder::new(source,input.resources(),&caps,1_000_000_000,caps.base().get().max_fragments+1,0),Err(BookV2MathDisplayError::Display(e)) if e.kind==DisplayError::RecordLimit));
        let mut own=BookV2ColumnDisplayBuilder::new(source,input.resources(),&caps,1_000_000_000,0,0).unwrap();
        let display=own.build().unwrap();
        assert!(matches!(display.verify_resources(foreign.resources(),&caps),Err(BookV2MathDisplayError::Display(e)) if e.kind==DisplayError::ReceiptMismatch));
        assert!(matches!(display.verify_resources(input.resources(),&different),Err(BookV2MathDisplayError::Display(e)) if e.kind==DisplayError::ReceiptMismatch));
        let maximum_records=caps.base().get().max_fragments;
        let mut records=0; let mut work=0;
        let exact=BookV2ColumnDisplayBuilder::new_counted(source,input.resources(),&caps,1_000_000_000,maximum_records-1,0,&mut records,&mut work).unwrap();
        assert_eq!(records,maximum_records); assert_eq!(records,exact.record_charge());
        assert_eq!(work,exact.work_steps());
        let prefix=work;
        let short=BookV2ColumnDisplayBuilder::new_counted(source,input.resources(),&caps,1_000_000_000,maximum_records,0,&mut records,&mut work);
        assert!(matches!(short,Err(BookV2MathDisplayError::Display(e)) if e.kind==DisplayError::RecordLimit));
        assert_eq!(records,maximum_records); assert_eq!(work,source.work_steps()); assert!(work<=prefix);
    }).unwrap();
}

#[test]
fn book_v2_column_display_excludes_every_unreferenced_definition_paint_and_positioned_anchor() {
    let caps = display_caps();
    let mut data = super::super::super::super::super::super::block_widths::column_block_data(
        "vector", true, false,
    );
    let mut unreferenced = data["document"]["footnotes"][0].clone();
    unreferenced["footnote_id"] = "unreferenced-display-rich".into();
    data["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(unreferenced);
    anchors(&mut data);
    let unused = &data["document"]["footnotes"][1];
    fn owners(v: &Value, set: &mut BTreeSet<u32>) {
        if let Some(owner) = v.get("node_id").and_then(Value::as_u64) {
            set.insert(owner as u32);
        }
        match v {
            Value::Array(items) => {
                for item in items {
                    owners(item, set);
                }
            }
            Value::Object(items) => {
                for item in items.values() {
                    owners(item, set);
                }
            }
            _ => (),
        }
    }
    let mut excluded = BTreeSet::new();
    owners(unused, &mut excluded);
    let root = Root::new();
    let input = vector_input(&root, data, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, 1_000_000_000);
    math_drive(&input, &caps, MODE, &mut budget, |source, _, observed| {
        assert_eq!(source.source().unreferenced_definitions(), 1);
        let original = source.original_graph().lines();
        assert!(
            original
                .paragraphs()
                .iter()
                .any(|p| excluded.contains(&p.owner().get()))
        );
        assert!(
            original
                .prepared()
                .vector_bindings()
                .unwrap()
                .receipts()
                .iter()
                .any(|b| excluded.contains(&b.node_id().get()))
        );
        assert!(source.source().geometry().pages().iter().all(|p| {
            p.fragments()
                .iter()
                .all(|f| f.definition_index() != Some(1))
        }));
        let mut builder =
            BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, 1_000_000_000, 0, 0)
                .unwrap();
        let display = builder.build().unwrap();
        inspect(&display);
        assert!(
            display
                .text()
                .draws()
                .iter()
                .all(|d| d.definition_index() != Some(1) && !excluded.contains(&d.owner().get()))
        );
        assert!(
            display
                .math()
                .draws()
                .iter()
                .all(|d| d.terminal().definition_index() != Some(1)
                    && !excluded.contains(&d.terminal().source().owner().get()))
        );
        assert!(
            display
                .images()
                .draws()
                .iter()
                .all(|d| d.fragment().definition_index() != Some(1)
                    && !excluded.contains(&d.source().owner().get()))
        );
        assert!(
            display
                .numbers()
                .draws()
                .iter()
                .all(|d| !excluded.contains(&d.placement().geometry().owner().get()))
        );
        assert!(
            display
                .markers()
                .draws()
                .iter()
                .all(|d| d.fragment().definition_index() != Some(1)
                    && !excluded.contains(&d.source().owner().get()))
        );
        assert!(
            display
                .anchors()
                .positions()
                .iter()
                .all(|d| d.fragment().definition_index() != Some(1)
                    && !excluded.contains(&d.anchor().source().owner().get()))
        );
        let unused_positioned = original
            .paragraphs()
            .iter()
            .filter(|p| excluded.contains(&p.owner().get()))
            .flat_map(|p| p.anchors())
            .filter(|a| a.position().is_some())
            .collect::<Vec<_>>();
        assert!(!unused_positioned.is_empty());
        assert!(unused_positioned.iter().all(|a| {
            display
                .anchors()
                .positions()
                .iter()
                .all(|d| !std::ptr::eq(d.anchor(), *a))
        }));
        probes::emit(
            "unreferenced-rich-definition",
            &display,
            &caps,
            observed,
            "source_callback",
            1_000_000_000,
            Some((0, 0)),
        );
    })
    .unwrap();
}

fn named_display_data(text: &str, blanks: usize, balanced: bool) -> Value {
    let mut data = plain_data(text, false, false);
    data["document"]["blocks"]
        .as_array_mut()
        .unwrap()
        .truncate(5);
    for p in data["document"]["blocks"].as_array_mut().unwrap() {
        p["classes"] = json!(["display-appendix"]);
    }
    let command = json!({"kind":"page_break","node_id":0,"classes":[],"span":data["document"]["blocks"][0]["span"]});
    for _ in 0..blanks {
        data["document"]["blocks"]
            .as_array_mut()
            .unwrap()
            .insert(0, command.clone());
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"display-appendix","selector":"paragraph.display-appendix","source_order":rules.len(),"extends":null,"declarations":[{"name":"page","important":false,"value":{"kind":"string","value":"display-appendix"}}]}));
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        master["body"] = json!({"x":17*65536,"y":10*65536,"width":210*65536,"height":256*65536});
    }
    let template = data["page_masters"]["masters"][0].clone();
    for (parity, count) in [("left", 3), ("right", 2)] {
        let mut master = template.clone();
        master["master_id"] = format!("z-display-appendix-{parity}").into();
        master["body"]["width"] = ((count * 100 + (count - 1) * 10) * 65536 + 1).into();
        master["body"]["height"] = ((if balanced {
            768
        } else if count == 3 {
            192
        } else {
            288
        }) * 65536)
            .into();
        master["column_layout"]["count"] = count.into();
        data["page_masters"]["masters"]
            .as_array_mut()
            .unwrap()
            .push(master);
    }
    data["page_masters"]["selection_rules"] = json!([
{"master_id":"z-display-appendix-left","parity":"even","first":null,"named_page":"display-appendix","source_order":0},
{"master_id":"z-display-appendix-right","parity":"odd","first":null,"named_page":"display-appendix","source_order":1}]);
    anchors(&mut data);
    balance(&mut data, balanced);
    data
}

#[test]
fn book_v2_column_display_respects_named_parity_two_three_columns_and_residual_raw_widths() {
    let caps = display_caps();
    let text = "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro";
    for blanks in [1, 2] {
        for balanced in [false, true] {
            let data = named_display_data(text, blanks, balanced);
            let root = Root::new();
            let input = display_input(&root, data, None, &caps);
            let mut budget = BookV2ColumnPageBudget::new(&caps, 1_000_000_000);
            math_drive(&input, &caps, MODE, &mut budget, |source, _, observed| {
                let pages = source.source().geometry().pages();
                assert_eq!(pages.len(), blanks + 1);
                assert!(
                    pages[..blanks]
                        .iter()
                        .all(|p| p.fragments().is_empty() && p.selection().balance().is_none())
                );
                let page = pages.last().unwrap();
                assert_eq!(page.selection().named_page(), Some("display-appendix"));
                let count = if blanks == 1 { 3 } else { 2 };
                assert_eq!(page.columns().len(), count);
                let mut widths = vec![100 * 65536; count];
                *widths.last_mut().unwrap() += 1;
                assert_eq!(
                    page.columns()
                        .iter()
                        .map(|c| c.bounds().width().get().raw())
                        .collect::<Vec<_>>(),
                    widths
                );
                assert_eq!(page.selection().balance().is_some(), balanced);
                let mut actual_widths = BTreeSet::new();
                for (local, placed) in page.fragments().iter().enumerate() {
                    if let Source::ParagraphLine {
                        paragraph_index,
                        line_index,
                    } = placed.fragment().source()
                    {
                        let offset = pages[..blanks]
                            .iter()
                            .map(|p| p.fragments().len())
                            .sum::<usize>();
                        let lines = source.fragment_lines(offset + local).unwrap();
                        let selected = &lines.paragraphs()[paragraph_index as usize]
                            .selected()
                            .unwrap()
                            .lines()[line_index as usize];
                        assert_eq!(
                            selected.inline_size(),
                            page.columns()[page.fragment_column(local).unwrap() as usize]
                                .bounds()
                                .width()
                        );
                        actual_widths.insert(selected.inline_size().get().raw());
                    }
                }
                assert_eq!(
                    actual_widths,
                    BTreeSet::from([100 * 65536, 100 * 65536 + 1])
                );
                let mut builder = BookV2ColumnDisplayBuilder::new(
                    source,
                    input.resources(),
                    &caps,
                    1_000_000_000,
                    0,
                    0,
                )
                .unwrap();
                let display = builder.build().unwrap();
                let facts = inspect(&display);
                assert_eq!(facts.pages, blanks + 1);
                probes::emit(
                    &format!("named-residual-{blanks}-{balanced}"),
                    &display,
                    &caps,
                    observed,
                    "source_callback",
                    1_000_000_000,
                    Some((0, 0)),
                );
            })
            .unwrap();
        }
    }
}

fn balanced_six_stream_data() -> Value {
    let mut data = super::super::super::super::super::super::block_widths::column_block_data(
        "vector", false, false,
    );
    let mut note = data["document"]["blocks"][0]["blocks"][0].clone();
    note["children"].as_array_mut().unwrap().truncate(1);
    let span = note["span"].clone();
    data["document"]["footnotes"] =
        json!([{"node_id":0,"span":span,"footnote_id":"balanced-display-note","blocks":[note]}]);
    let reference_span = data["document"]["blocks"][4]["blocks"][0]["children"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["span"]
        .clone();
    data["document"]["blocks"][4]["blocks"][0]["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":reference_span,"footnote_id":"balanced-display-note"}));
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        master["footnote"] = json!({"x":23*65536,"y":150*65536,"width":160*65536,"height":90*65536+typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW});
    }
    anchors(&mut data);
    balance(&mut data, true);
    data
}

#[test]
fn book_v2_column_display_balanced_page_merges_text_formula_numbers_images_and_notes() {
    let caps = display_caps();
    let data = balanced_six_stream_data();
    let root = Root::new();
    let input = vector_input(&root, data, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, 1_000_000_000);
    math_drive(&input, &caps, MODE, &mut budget, |source, _, observed| {
        let balanced = source
            .source()
            .geometry()
            .pages()
            .iter()
            .filter(|p| p.selection().balance().is_some())
            .collect::<Vec<_>>();
        assert_eq!(balanced.len(), 1);
        assert!(
            balanced[0]
                .fragments()
                .iter()
                .any(|f| f.definition_index().is_some())
        );
        let mut builder =
            BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, 1_000_000_000, 0, 0)
                .unwrap();
        let display = builder.build().unwrap();
        let facts = inspect(&display);
        assert!(
            facts.text > 0
                && facts.vector_math > 0
                && facts.numbers > 0
                && facts.images > 0
                && facts.markers > 0
                && facts.separators > 0
                && facts.notes > 0
                && facts.anchors > 0
                && facts.balanced > 0
        );
        probes::emit(
            "balanced-six-stream-notes",
            &display,
            &caps,
            observed,
            "source_callback",
            1_000_000_000,
            Some((0, 0)),
        );
    })
    .unwrap();
}

fn with_retained_cut_display<R>(
    inspect_display: impl for<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &M4EffectiveResourceLimits,
    ) -> R,
) -> R {
    // The original 31pt vector atoms prevent natural widths below18pt. The
    // lawful retained source cut changes shaping context at a genuine100pt
    // column width; this case exercises the typed graph adapters directly.
    const FONT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/vmb-book/header-context-font/header-context.ttf"
    ));
    let caps = display_caps();
    let root = Root::new();
    let (mut data,_)=crate::book_v2_resources::tests::shaping_tests::native_tests::header_terminals::header_fixture(true,true,false,false);
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        master["body"]["width"] = (210 * 65536).into();
        master["body"]["height"] = (160 * 65536).into();
        master["column_layout"] =
            json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"none"});
    }
    let input = display_input(&root, data, Some(FONT), &caps);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 100_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &caps).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &caps).unwrap();
    let mut allowance =
        BookV2LineVariantBudget::new(100_000_000, caps.base().get().max_line_reshape_passes);
    let base = prepare_budgeted_book_v2_column_line_variant_seed(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &caps,
        MODE,
        &plan,
        &mut allowance,
        0,
        None,
        None,
    )
    .unwrap();
    let (widths, header, units) = with_budgeted_rebuilt_book_v2_column_line_variants(
        &[&base],
        &mut BookV2LineVariantBudget::new(100_000_000, 0),
        0,
        |set| {
            let lines = set.variant(0).unwrap().lines();
            let header = lines
                .paragraphs()
                .iter()
                .position(|p| {
                    p.lines()
                        .iter()
                        .flat_map(|l| l.items())
                        .any(|i| matches!(i, ProductionPlacedInline::Vector(_)))
                })
                .unwrap();
            let widths = lines
                .prepared()
                .paragraphs()
                .iter()
                .zip(lines.paragraphs())
                .map(|(p, l)| vec![l.inline_size(); p.items().unwrap().units().len().max(1)])
                .collect::<Vec<_>>();
            let units = widths[header].len() as u32;
            (widths, header, units)
        },
    )
    .unwrap();
    let refs = widths
        .iter()
        .map(|w| Some(w.as_slice()))
        .collect::<Vec<_>>();
    let mut ends = vec![None; widths.len()];
    let header_ends = [2, units];
    ends[header] = Some(header_ends.as_slice());
    let assignment =
        BookV2SourceWidthAssignments::with_retained_line_ends(&flow, &refs, &ends).unwrap();
    let split = base
        .prepare_budgeted_with_source_widths(&assignment, &mut allowance, base.record_charge())
        .unwrap();
    with_budgeted_rebuilt_book_v2_column_line_variants(
        &[&base, &split],
        &mut BookV2LineVariantBudget::new(100_000_000, 0),
        0,
        |set| {
            let views = set.variants().collect::<Vec<_>>();
            for view in &views {
                assert_eq!(
                    view.lines()
                        .frames()
                        .unwrap()
                        .measurement_region(flow.tables()[0].owner())
                        .unwrap()
                        .width()
                        .get()
                        .raw(),
                    100 * 65536
                );
            }
            assert_eq!(
                views[1].lines().paragraphs()[header]
                    .selected()
                    .unwrap()
                    .lines()[0]
                    .line()
                    .end_unit(),
                2
            );
            let first_gid = |i: usize| {
                views[i].lines().paragraphs()[header].lines()[0]
                    .items()
                    .iter()
                    .find_map(|i| match i {
                        ProductionPlacedInline::Text(c) => {
                            c.glyphs().first().map(|g| g.glyph().original_gid.get())
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            assert_eq!((first_gid(0), first_gid(1)), (59, 71));
            assert!(!std::ptr::eq(views[0].lines(), views[1].lines()));
            let numbers = views
                .iter()
                .map(|v| {
                    shape_book_v2_equation_numbers(v.lines().prepared().shaped(), &caps, 0).unwrap()
                })
                .collect::<Vec<_>>();
            let blocks = views
                .iter()
                .zip(&numbers)
                .map(|(v, n)| {
                    prepare_book_v2_vector_blocks(v.lines(), n.as_ref(), &caps, 0).unwrap()
                })
                .collect::<Vec<_>>();
            let mut records = set.record_charge();
            let measurements = views
                .iter()
                .zip(&blocks)
                .map(|(v, b)| {
                    let f = prepare_book_v2_rebuilt_column_flow_counted(
                        v,
                        b.as_ref(),
                        &caps,
                        records,
                        &mut records,
                    )
                    .unwrap();
                    prepare_book_v2_column_table_measurements_counted(f, &caps, &mut records)
                        .unwrap()
                })
                .collect::<Vec<_>>();
            let variant = prepare_book_v2_column_table_header_variant_counted(
                &set,
                &measurements[0],
                &measurements[1],
                0,
                &caps,
                100_000_000,
                records,
                &mut records,
                &mut 0,
            )
            .unwrap();
            let catalog = prepare_book_v2_column_table_header_catalog_counted(
                &measurements[0],
                &[&variant],
                &caps,
                100_000_000,
                records,
                &mut records,
                &mut 0,
            )
            .unwrap();
            let mut search = prepare_book_v2_column_page_search_with_headers_counted(
                &catalog,
                &caps,
                100_000_000,
                records,
                &mut 0,
                &mut 0,
            )
            .unwrap();
            let stable = search
                .select_stable_column_pages_counted(4, &mut 0)
                .unwrap();
            let widths = search.paragraph_frame_feedback(stable.sequence()).unwrap();
            assert!(
                widths.matches_selected_line_widths() && widths.matches_selected_block_widths()
            );
            let placed = search.place_column_pages(stable.sequence()).unwrap();
            let closed = search.close_column_page_sources(&stable, &placed).unwrap();
            let source = search.finalize_column_page_math(closed, &caps, 0).unwrap();
            assert!(source.source().has_header_variants());
            assert!(source.source().geometry().pages().len() > 1);
            assert!(
                source
                    .source()
                    .geometry()
                    .pages()
                    .iter()
                    .flat_map(|p| p.columns())
                    .all(|c| c.bounds().width().get().raw() == 100 * 65536)
            );
            let mut builder = BookV2ColumnDisplayBuilder::new(
                &source,
                input.resources(),
                &caps,
                100_000_000,
                0,
                0,
            )
            .unwrap();
            let display = builder.build().unwrap();
            let facts = inspect(&display);
            assert!(facts.repeated > 0 && facts.vector_math > 0 && facts.numbers > 0);
            let glyphs = |repeated| {
                display
                    .text()
                    .draws()
                    .iter()
                    .filter(|d| d.repeated_header() == repeated)
                    .flat_map(|d| d.glyphs())
                    .map(|g| g.original_gid().get())
                    .collect::<BTreeSet<_>>()
            };
            assert!(glyphs(false).contains(&59));
            assert!(!glyphs(false).contains(&71));
            assert!(glyphs(true).contains(&71));
            inspect_display(&display, &caps)
        },
    )
    .unwrap()
}

#[test]
fn book_v2_column_display_preserves_repeated_only_contextual_glyph_from_retained_source_cut() {
    with_retained_cut_display(|display, caps| {
        probes::emit_retained_cut("contextual-retained-source-cut", display, caps, 100_000_000);
    });
}

#[test]
fn book_v2_column_display_driver_retains_exact_limits_callback_failure_and_retry_charges() {
    use crate::book_v2_resources::with_budgeted_book_v2_column_display as display_drive;
    let caps = display_caps();
    let root = Root::new();
    let mut data = plain_data("Pro", true, false);
    anchors(&mut data);
    balance(&mut data, true);
    let input = display_input(&root, data.clone(), None, &caps);
    let run = |input: &PreparedBookV2Resources, caps: &M4EffectiveResourceLimits, maximum| {
        let mut budget = BookV2ColumnPageBudget::new(caps, maximum);
        let mut called = false;
        let result = display_drive(
            input,
            caps,
            MODE,
            &mut budget,
            |display, builder, observed| {
                called = true;
                assert_eq!(
                    (observed.record_charge(), observed.work_steps()),
                    (display.record_charge(), display.work_steps())
                );
                assert_eq!(
                    (builder.record_charge(), builder.work_steps()),
                    (display.record_charge(), display.work_steps())
                );
                let facts = inspect(display);
                probes::emit(
                    "driver-exact",
                    display,
                    caps,
                    observed,
                    "display_callback",
                    maximum,
                    None,
                );
                facts
            },
        );
        (result, budget.observation(), called)
    };
    let first = run(&input, &caps, 1_000_000_000);
    assert!(first.2);
    let facts = first.0.unwrap();
    let exact = run(&input, &caps, first.1.work_steps());
    assert!(exact.2);
    assert_eq!(exact.0.unwrap(), facts);
    assert_eq!(exact.1, first.1);
    let short = run(&input, &caps, first.1.work_steps() - 1);
    assert!(!short.2 && short.0.is_err());
    assert_eq!(short.1.work_steps(), first.1.work_steps() - 1);
    assert!(short.1.record_charge() > 0);
    let exact_caps = changed_caps(&caps, |c| c.max_fragments = first.1.record_charge());
    let exact_root = Root::new();
    let exact_input = display_input(&exact_root, data.clone(), None, &exact_caps);
    let record_exact = run(&exact_input, &exact_caps, first.1.work_steps());
    assert!(record_exact.2);
    assert!(record_exact.0.is_ok());
    assert_eq!(
        record_exact.1.record_charge(),
        exact_caps.base().get().max_fragments
    );
    let short_caps = changed_caps(&caps, |c| c.max_fragments = first.1.record_charge() - 1);
    let short_root = Root::new();
    let short_input = display_input(&short_root, data, None, &short_caps);
    let record_short = run(&short_input, &short_caps, 1_000_000_000);
    assert!(!record_short.2 && record_short.0.is_err());
    assert!(
        record_short.1.record_charge() > 0
            && record_short.1.record_charge() <= short_caps.base().get().max_fragments
    );
    let mut successful = BookV2ColumnPageBudget::new(&caps, 1_000_000_000);
    let mut callback_final = None;
    let result = display_drive(
        &input,
        &caps,
        MODE,
        &mut successful,
        |display, builder, observed| {
            assert_eq!(inspect(display), facts);
            let again = builder.build().unwrap();
            assert_eq!(inspect(&again), facts);
            assert!(
                again.record_charge() > observed.record_charge()
                    && again.work_steps() > observed.work_steps()
            );
            callback_final = Some((builder.record_charge(), builder.work_steps()));
            Err::<(), _>("intentional consumer failure")
        },
    );
    assert_eq!(result.unwrap().unwrap_err(), "intentional consumer failure");
    let after = successful.observation();
    assert_eq!(
        (after.record_charge(), after.work_steps()),
        callback_final.unwrap()
    );
    assert!(
        after.record_charge() > first.1.record_charge()
            && after.work_steps() > first.1.work_steps()
    );
    let mut exhausted = BookV2ColumnPageBudget::new(&caps, after.work_steps() - 1);
    let mut reached = false;
    let result = display_drive(&input, &caps, MODE, &mut exhausted, |_, builder, _| {
        reached = true;
        let err = builder
            .build()
            .err()
            .expect("callback one-short work unexpectedly succeeded");
        assert!(matches!(err, BookV2MathDisplayError::WorkLimit(_)));
        (builder.record_charge(), builder.work_steps())
    });
    assert!(reached);
    let prefix = result.unwrap();
    assert_eq!(
        (
            exhausted.observation().record_charge(),
            exhausted.observation().work_steps()
        ),
        prefix
    );
    assert_eq!(prefix.1, after.work_steps() - 1);
    let before = exhausted.observation();
    let mut called = false;
    let retry = display_drive(&input, &caps, MODE, &mut exhausted, |_, _, _| called = true);
    assert!(retry.is_err() && !called);
    let retained = exhausted.observation();
    assert!(retained.record_charge() >= before.record_charge());
    assert_eq!(retained.work_steps(), before.work_steps());
}

#[test]
#[ignore = "requires original Harano Aji font pinned by SHA-256; set TYPAXIS_HARANO_FONT"]
fn book_v2_column_display_preserves_original_harano_text_lists_notes_and_glyph_origins() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        probes::hex(&typaxis_core::sha256(&font)),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let caps = display_caps();
    for balanced in [false, true] {
        let mut data = plain_data("本文", true, false);
        data["document"]["blocks"]
            .as_array_mut()
            .unwrap()
            .truncate(5);
        wrap_list(&mut data["document"]["blocks"], true);
        list_font_rule(&mut data);
        anchors(&mut data);
        balance(&mut data, balanced);
        let root = Root::new();
        let input = display_input(&root, data, Some(&font), &caps);
        let facts = inspect_input(&format!("original-harano-{balanced}"), &input, &caps);
        assert!(facts.text > 0 && facts.markers > 0 && facts.notes > 0 && facts.anchors > 0);
    }
}

#[path = "book_v2_column_display_callback_ownership_tests.rs"]
mod callback_ownership;
