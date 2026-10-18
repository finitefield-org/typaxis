use super::*;

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn rect(r: typaxis_core::Rect) -> Value {
    json!([
        r.x().raw(),
        r.y().raw(),
        r.width().get().raw(),
        r.height().get().raw()
    ])
}
fn glyphs(g: &[typaxis_display_list::ProductionBodyGlyph]) -> Value {
    Value::Array(
        g.iter()
            .map(|g| json!([g.original_gid().get(), g.x().raw(), g.y().raw()]))
            .collect(),
    )
}
fn span(s: typaxis_core::DisplayTextSpan) -> Value {
    json!([
        s.text_id().get(),
        s.range().start_byte().get(),
        s.range().end_byte().get()
    ])
}
fn source_span(s: ShapeSourceSpan, parsed: u32) -> Value {
    match s {
        ShapeSourceSpan::Parsed(s) => {
            json!({"generated":false,"span":[s.text_id().get(),s.start_byte().get(),s.end_byte().get()]})
        }
        ShapeSourceSpan::Generated(p) => {
            let s = p.text_span();
            json!({"generated":true,"span":[parsed+s.text_id().get(),s.range().start_byte().get(),s.range().end_byte().get()]})
        }
    }
}
fn font(f: &typaxis_shaping::ProductionBodyFont) -> Value {
    json!({"face":f.face_id().get(),"sha256":hex(&f.content_hash()),"index":f.face_index(),"size":f.size().get().raw(),"ascent":f.ascender().raw(),"descent":f.descender().raw(),"line_gap":f.line_gap().raw()})
}
fn graph(g: typaxis_pagination::book_v2::BookV2SourceGraph<'_, '_, '_, '_, '_>) -> Value {
    let l = g.lines();
    let p = l.prepared();
    json!({"line_fingerprint":hex(&l.fingerprint()),"prepared_fingerprint":hex(&p.fingerprint()),"shape_fingerprint":hex(&p.shaped().fingerprint()),"binding_epoch":hex(&p.shaped().binding_epoch()),"block_fingerprint":g.blocks().map(|b|hex(&b.fingerprint())),"vector_fingerprint":p.vector_bindings().map(|b|hex(&b.fingerprint())),"native_fingerprint":p.native_math().map(|n|hex(&n.fingerprint()))})
}
fn original_glyphs(g: &[typaxis_shaping::ShapedGlyph]) -> Value {
    json!(g.iter().map(|g|json!({"gid":g.original_gid.get(),"advance_x":g.advance_x.raw(),"advance_y":g.advance_y.raw(),"offset_x":g.offset_x.raw(),"offset_y":g.offset_y.raw()})).collect::<Vec<_>>())
}
fn figure_source(f: &typaxis_layout::ProductionPreparedFigure<'_>) -> Value {
    let media = match f.media() {
        ProductionFigureMedia::Raster {
            pixel_width,
            pixel_height,
        } => json!({"kind":"raster","pixel_width":pixel_width,"pixel_height":pixel_height}),
        ProductionFigureMedia::Svg {
            content_key,
            scale_raw,
        } => {
            json!({"kind":"svg","scale":scale_raw,"source_sha256":hex(&content_key.source_sha256()),"ir_fingerprint":hex(&content_key.ir_fingerprint())})
        }
    };
    json!({"kind":"figure","image":f.image_id().get(),"owner":f.owner().get(),"alternative":f.source().alternative(),"width":f.width().get().raw(),"height":f.height().get().raw(),"source_sha256":hex(&f.admitted_sha256()),"media":media})
}
fn metrics(m: typaxis_layout::BoundPrecomposedVectorMetrics) -> Value {
    json!({"advance":m.advance().get().raw(),"ascent":m.ascent().get().raw(),"baseline":m.baseline().get().raw(),"descent":m.descent().get().raw(),"origin_x":m.origin_x().raw(),"viewport":{"height":m.viewport_height().get().raw(),"width":m.viewport_width().get().raw()},"viewport_right_from_pen":m.viewport_right_from_pen().raw()})
}
fn vector_source(
    b: &typaxis_layout::book_v2::BookV2BoundVector<'_>,
    admitted: &AdmittedProductionResourceLedgerV3,
) -> Value {
    let (scale, color) = match b.placement() {
        PrecomposedVectorPlacementInput::Inline(p) => (p.scale().get().raw(), p.paint()),
        PrecomposedVectorPlacementInput::MathVectorBlock(p) => (p.scale().get().raw(), p.paint()),
        PrecomposedVectorPlacementInput::VectorFigure(p) => (p.scale().get().raw(), p.paint()),
    };
    let image = admitted.image(b.resource().image_id()).unwrap();
    let key = VectorContentKey::from_admitted(image).unwrap();
    let paint = json!({"blue":color.blue(),"green":color.green(),"red":color.red()});
    let placement = match b.placement() {
        PrecomposedVectorPlacementInput::Inline(p) => {
            json!({"kind":"inline","metrics":metrics(p.metrics()),"paint":paint,"scale":scale,"spacing_after":p.spacing_after().get().raw(),"spacing_before":p.spacing_before().get().raw()})
        }
        PrecomposedVectorPlacementInput::MathVectorBlock(p) => {
            json!({"kind":"math_vector_block","metrics":metrics(p.metrics()),"paint":paint,"scale":scale,"style_fingerprint":hex(&p.style().fingerprint())})
        }
        PrecomposedVectorPlacementInput::VectorFigure(p) => {
            json!({"kind":"vector_figure","paint":paint,"scale":scale,"style_fingerprint":hex(&p.style().fingerprint()),"viewport":{"height":p.viewport_height().get().raw(),"width":p.viewport_width().get().raw()}})
        }
    };
    let r = b.resource();
    let s = b.owner_source_span();
    let mut epoch = typaxis_core::sha256(b"typaxis.book-2-vector-epoch/1");
    for hash in [admitted.profile_fingerprint(), admitted.fingerprint()] {
        epoch = typaxis_core::sha256(&[epoch.as_slice(), hash.as_slice()].concat());
    }
    let binding = json!({"algorithm":"typaxis.book-2-vector-binding/1","epoch":hex(&epoch),"node_id":b.node_id().get(),"placement":placement,"resource":{"admitted_media":r.admitted_media().as_str(),"declared_media":r.declared_media().as_str(),"image_id":r.image_id().get(),"intrinsic_height":r.intrinsic_height().get().raw(),"intrinsic_width":r.intrinsic_width().get().raw(),"ir_fingerprint":hex(&r.ir_fingerprint()),"ir_fingerprint_id":r.ir_fingerprint_id(),"ir_id":r.ir_id(),"limits_fingerprint":hex(&r.limits_fingerprint()),"parser_id":r.parser_id(),"profile_fingerprint":hex(&r.profile_fingerprint()),"source_sha256":hex(&r.source_sha256()),"view_box":r.view_box()},"source_span":{"end_byte":s.end_byte().get(),"source_id":s.source_id().get(),"start_byte":s.start_byte().get()}});
    json!({"kind":"vector","owner":b.node_id().get(),"fingerprint":hex(&b.fingerprint()),"binding":binding,"image":b.resource().image_id().get(),"source_sha256":hex(&key.source_sha256()),"ir_fingerprint":hex(&key.ir_fingerprint()),"scale":scale,"color":[color.red(),color.green(),color.blue()],"alternative":b.source().alternative().alternative(),"actual_text":b.source().alternative().resolved_actual_text()})
}
fn original_math(
    source: BookV2BodyMathSource<'_, '_>,
    admitted: &AdmittedProductionResourceLedgerV3,
) -> Value {
    match source {
        BookV2BodyMathSource::Vector(b) => vector_source(b, admitted),
        BookV2BodyMathSource::Native(n) => {
            let (left, top, right, bottom) = n.computation().dimensions().bbox();
            let style = n.style();
            let block = style.block_style();
            json!({"kind":"native","owner":n.node_id().get(),"fingerprint":hex(&n.fingerprint()),"face":n.font_face_id().get(),"font_sha256":hex(&n.font_sha256()),"face_index":n.face_index(),"bbox":[left,top,right,bottom],"alternative":n.source().domain().speech,"actual_text":n.source().domain().speech,
                "computation_canonical_jcs":n.computation().canonical_jcs(),"computation_fingerprint":hex(&n.computation().fingerprint()),
                "style":{"algorithm":"typaxis.math-computed-style/1","end_indent":block.end_indent().get().raw(),"font_families":style.font_families(),"font_size":style.font_size().get().raw(),"keep_with_next":block.keep_with_next(),"kind":style.kind().as_str(),"line_height":style.line_height().get().raw(),"page":style.page_name().map(|p|p.as_str()),"space_after":block.space_after().get().raw(),"space_before":block.space_before().get().raw(),"start_indent":block.start_indent().get().raw(),"text_align":block.text_align().as_str()},
                "paints":n.computation().paints().iter().map(|p|match p {
                    typaxis_math::MathPaint::Glyph(g)=>json!({"gid":g.original_gid().get(),"unicode":g.unicode() as u32,"ordinal":g.logical_ordinal(),"x":g.x(),"y":g.y(),"font_size":g.font_size_raw(),"advance":g.advance()}),
                    typaxis_math::MathPaint::Rule(r)=>json!({"rule":[r.x(),r.y(),r.width(),r.height()]}),
                }).collect::<Vec<_>>()})
        }
    }
}
fn anchor_source(a: &typaxis_layout::ProductionPlacedInlineAnchor) -> Value {
    let s = a.source().source_span();
    json!({"owner":a.source().owner().get(),"source_span":[s.source_id().get(),s.start_byte().get(),s.end_byte().get()],"boundary_unit":a.source().boundary_unit(),"position":a.position().map(|p|json!({"line":p.line_index(),"x":p.x().raw(),"baseline":p.baseline().raw()}))})
}
fn paint(p: Paint) -> Value {
    let (kind, index) = match p {
        Paint::PageRegionText { .. } => panic!("running region outside the legacy Column fixture"),
        Paint::FootnoteSeparator(i) => ("separator", i),
        Paint::Marker(i) => ("marker", i),
        Paint::Text(i) => ("text", i),
        Paint::Math(i) => ("math", i),
        Paint::Image(i) => ("image", i),
        Paint::EquationNumber(i) => ("number", i),
    };
    json!([kind, index])
}

/// Actual callback diagnostics live in a nested directory; the existing body
/// PDF probe format and its top-level glob remain separate.
pub(super) fn emit(
    case: &str,
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
    observed: crate::book_v2_resources::BookV2PdfConvergenceObservation,
    scope: &str,
    maximum_work: u64,
    builder_prior: Option<(u64, u64)>,
) {
    emit_inner(
        case,
        display,
        limits,
        Some(observed),
        scope,
        maximum_work,
        builder_prior,
    );
}
pub(super) fn emit_retained_cut(
    case: &str,
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
) {
    emit_inner(
        case,
        display,
        limits,
        None,
        "retained_source_cut",
        maximum_work,
        Some((0, 0)),
    );
}
fn emit_inner(
    case: &str,
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
    observed: Option<crate::book_v2_resources::BookV2PdfConvergenceObservation>,
    scope: &str,
    maximum_work: u64,
    builder_prior: Option<(u64, u64)>,
) {
    let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") else {
        return;
    };
    let value = snapshot(
        case,
        display,
        limits,
        observed,
        scope,
        maximum_work,
        builder_prior,
    );
    let directory = std::path::PathBuf::from(directory).join("column-display");
    fs::create_dir_all(&directory).unwrap();
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    fs::write(
        directory.join(format!(
            "{case}-{}.json",
            hex(&typaxis_core::sha256(&bytes))
        )),
        bytes,
    )
    .unwrap();
}

pub(super) fn snapshot(
    case: &str,
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
    observed: Option<crate::book_v2_resources::BookV2PdfConvergenceObservation>,
    scope: &str,
    maximum_work: u64,
    builder_prior: Option<(u64, u64)>,
) -> Value {
    let has_driver_observation = observed.is_some();
    let observed = observed.unwrap_or_default();
    let source = display.source();
    let mut original = Vec::new();
    let mut offset = 0;
    let pages=source.source().geometry().pages().iter().map(|page| {
        let current=offset;
        for (local,(placed,cell,repeated)) in page.fragments_with_roles().enumerate() {
            let index=current+local;
            let f=placed.fragment();
            let graph=source.fragment_graph(index).unwrap();
            let lines=graph.lines();
            let parsed=lines.prepared().source_flow().body().body().wire().text_buffers().len() as u32;
            let inline=if let Source::ParagraphLine{paragraph_index,line_index}=f.source() {
                let paragraph=&lines.paragraphs()[paragraph_index as usize];
                let line=&paragraph.lines()[line_index as usize];
                let font=paragraph.font();
                json!({"paragraph":paragraph_index,"owner":paragraph.owner().get(),"line":line_index,"baseline":line.baseline().raw(),
                    "font":font.map(self::font),
                    "items":line.items().iter().enumerate().map(|(inline,item)|match item {
                        ProductionPlacedInline::Text(c)=>json!({"kind":"text","inline":inline,"owner":c.run().owner().get(),"run":c.run_index(),"cluster":c.cluster_index(),"text":c.utf8(),"source":source_span(c.source_span(),parsed),"pen_x":c.pen_x().raw(),
                            "glyphs":c.glyphs().iter().map(|g|json!({"gid":g.glyph().original_gid.get(),"x":g.x().raw(),"y":g.y().raw(),"advance_x":g.glyph().advance_x.raw()})).collect::<Vec<_>>()}),
                        ProductionPlacedInline::Vector(v)=>{let b=lines.prepared().vector_bindings().unwrap().receipt(v.occurrence().item().node_id()).unwrap();json!({"kind":if b.kind()==PrecomposedVectorKind::InlineVector {"image"} else {"math"},"inline":inline,"owner":b.node_id().get(),"fingerprint":hex(&b.fingerprint()),"viewport":rect(v.geometry().viewport()),"pen_x":v.geometry().pen_origin_x().raw(),"baseline":v.geometry().line_baseline_y().raw()})},
                        ProductionPlacedInline::BookV2Math(m)=>json!({"kind":"math","inline":inline,"owner":m.owner().get(),"fingerprint":hex(&m.receipt().fingerprint()),"pen_x":m.pen_x().raw(),"baseline":m.baseline().raw()}),
                        ProductionPlacedInline::Break(_)=>json!({"kind":"break","inline":inline}),
                        ProductionPlacedInline::Math(_)=>panic!("legacy math cannot replace Book /2 probe source"),
                    }).collect::<Vec<_>>(),
                    "anchors":paragraph.anchors().iter().filter(|a|a.position().is_some_and(|p|p.line_index()==line_index)).map(anchor_source).collect::<Vec<_>>()})
            } else {Value::Null};
            let source_kind=match f.source() {
                Source::ParagraphLine{paragraph_index,line_index}=>json!({"kind":"paragraph","paragraph":paragraph_index,"line":line_index}),
                Source::Figure{figure_index}=>{let figure=&lines.prepared().figures()[figure_index as usize];let mut source=figure_source(figure);source["index"]=figure_index.into();source},
                Source::VectorBlock{block_index}=>{let b=&graph.blocks().unwrap().blocks()[block_index as usize];let origin_x=match b.binding().placement(){PrecomposedVectorPlacementInput::MathVectorBlock(p)=>Some(p.metrics().origin_x().raw()),_=>None};json!({"kind":"vector_block","index":block_index,"owner":b.owner().get(),"source_fingerprint":hex(&b.binding().fingerprint()),"origin_x":origin_x,"baseline":b.baseline().map(|b|b.get().raw()),"vector":vector_source(b.binding(),display.admitted())})},
                Source::NativeMathBlock{block_index}=>{let b=&lines.prepared().native_math().unwrap().display_blocks()[block_index as usize];json!({"kind":"native_block","index":block_index,"owner":b.owner().get(),"left":b.left().raw(),"baseline":b.baseline().raw()})},
            };
            original.push(json!({"fragment":index,"local_fragment":local,"page":f.page_index(),"column":page.fragment_column(local),"owner":f.owner().get(),"definition":placed.definition_index(),"item":placed.item_index(),"cell":cell.map(|c|c.owner().get()),"repeated":repeated,"bounds":rect(f.bounds()),"baseline":f.baseline().map(|b|b.raw()),"viewport":f.viewport().map(rect),"source":source_kind,"graph":self::graph(graph),"header":page.header_variant(local).map(|h|json!({"fingerprint":hex(&h.fingerprint()),"table":h.table_index(),"definition":h.definition_index(),"global_item":h.global_item_index(),"fragment":h.fragment_index()})),"line":inline}));
        }
        offset+=page.fragments().len();
        json!({"page":page.selection().page_index(),"named_index":page.selection().candidate().frames().named_page_index(),"body":rect(page.selection().candidate().frames().body()),"columns":page.columns().iter().map(|c|json!({"bounds":rect(c.bounds()),"fragments":[c.fragments().start,c.fragments().end]})).collect::<Vec<_>>(),"footnote":page.selection().candidate().frames().footnote().map(rect),"placed_footnote":page.selection().candidate().footnote_bounds().map(rect),"separator_ink":page.separator_ink().map(rect),
            "list_markers":page.list_markers().iter().map(|m|json!({"fragment":m.fragment_index(),"marker":m.marker_index(),"bounds":rect(m.bounds()),"baseline":m.baseline().raw()})).collect::<Vec<_>>(),
            "footnote_markers":page.footnote_markers().iter().map(|m|json!({"fragment":m.fragment_index(),"definition":m.definition_index(),"bounds":rect(m.bounds()),"baseline":m.baseline().raw()})).collect::<Vec<_>>(),
            "equation_numbers":page.equation_numbers().iter().map(|n|json!({"owner":n.geometry().owner().get(),"parent":n.geometry().parent_owner().get(),"fragment":n.geometry().fragment_index(),"bounds":rect(n.geometry().bounds()),"shape_fingerprint":hex(&n.geometry().shape_fingerprint()),"repeated":n.repeated_header()})).collect::<Vec<_>>(),
            "forced":page.selection().forced_break().map(|n|n.get()),"fragments":[current,offset],"balance":page.selection().balance().map(|b|json!({"fingerprint":hex(&b.fingerprint()),"selected_height":b.selected_height().get().raw(),"canonical":b.canonical_bytes()}))})
    }).collect::<Vec<_>>();
    let text=display.text().draws().iter().map(|d|json!({"owner":d.owner().get(),"fragment":d.fragment_index(),"page":d.page_index(),"definition":d.definition_index(),"item":d.item_index(),"inline":d.inline_index(),"cell":d.cell_owner().map(|n|n.get()),"repeated":d.repeated_header(),"run":d.cluster().run_index(),"cluster":d.cluster().cluster_index(),"text":d.exact_text(),"span":span(d.text_span()),"generated":d.generated_provenance().is_some(),"bounds":d.logical_bounds().map(rect),"glyphs":glyphs(d.glyphs())})).collect::<Vec<_>>();
    let math=display.math().draws().iter().map(|d| {
        let t=d.terminal();
        let paints=match d.paint() {
            BookV2MathPaint::Vector(v)=>json!({"kind":"vector","viewport":rect(v.viewport()),"source_sha256":hex(&v.content_key().source_sha256()),"ir_fingerprint":hex(&v.content_key().ir_fingerprint()),"scale":v.scale_raw(),"color":v.color(),"matrix":[v.matrix().a.raw(),v.matrix().b.raw(),v.matrix().c.raw(),v.matrix().d.raw(),v.matrix().e.raw(),v.matrix().f.raw()]}),
            BookV2MathPaint::Native(n)=>json!({"kind":"native","bounds":rect(n.bounds()),"paints":n.paints().iter().map(|p|match p {
                ProductionNativeMathPaint::Rule(r)=>json!({"rule":rect(*r)}),
                ProductionNativeMathPaint::Glyph{original_gid,unicode,logical_ordinal,x,y,font_size}=>json!({"gid":original_gid.get(),"unicode":*unicode as u32,"ordinal":logical_ordinal,"x":x.raw(),"y":y.raw(),"font_size":font_size.get().raw()}),
            }).collect::<Vec<_>>() }),
        };
        json!({"owner":t.source().owner().get(),"fragment":t.fragment_index(),"page":t.page_index(),"column":t.column_index(),"definition":t.definition_index(),"item":t.item_index(),"inline":t.inline_index(),"cell":t.cell_owner().map(|n|n.get()),"repeated":t.repeated_header(),"source_fingerprint":hex(&t.source().fingerprint()),"origin_x":t.origin_x().raw(),"baseline":t.baseline().raw(),"alternative":d.alternative(),"actual_text":d.actual_text(),"original":original_math(t.source(),display.admitted()),"paint":paints})
    }).collect::<Vec<_>>();
    let numbers=display.numbers().draws().iter().map(|d|json!({"owner":d.placement().geometry().owner().get(),"parent":d.placement().geometry().parent_owner().get(),"fragment":d.placement().geometry().fragment_index(),"page":d.placement().geometry().page_index(),"repeated":d.placement().repeated_header(),"shape_fingerprint":hex(&d.shape().fingerprint()),"bounds":rect(d.placement().geometry().bounds()),"text":d.shape().text(),
        "original":{"font":font(d.font()),"font_instance":d.shape().font_instance_id().get(),"height":d.shape().height().get().raw(),"width":d.shape().width().get().raw(),"text_start":d.shape().source().equation_number().unwrap().text().text_span().start_byte().get(),"runs":d.shape().runs().iter().map(|r|json!({"bidi_level":r.bidi_level().get(),"glyphs":original_glyphs(r.glyphs()),"clusters":r.clusters().iter().map(|c|json!({"source":source_span(c.source_span,0),"glyph_range":[c.glyph_start,c.glyph_end]})).collect::<Vec<_>>()})).collect::<Vec<_>>()},
        "clusters":d.clusters().iter().map(|c|json!({"run":c.run_index(),"cluster":c.cluster_index(),"text":c.exact_text(),"span":span(c.text_span()),"bounds":c.logical_bounds().map(rect),"glyphs":glyphs(c.glyphs())})).collect::<Vec<_>>()})).collect::<Vec<_>>();
    let parsed = source
        .original_graph()
        .lines()
        .prepared()
        .source_flow()
        .body()
        .body()
        .wire()
        .text_buffers()
        .len() as u32;
    let markers=display.markers().draws().iter().map(|d|json!({"owner":d.source().owner().get(),"fragment":d.fragment_index(),"cell":d.cell_role().map(|r|r.owner().get()),"repeated":d.repeated_header(),"bounds":rect(d.placement().bounds()),"baseline":d.placement().baseline().raw(),"kind":match d.placement(){BookV2MarkerPlacement::List(_)=>"list",BookV2MarkerPlacement::Footnote(_)=>"footnote"},"text":d.source().utf8(),
        "original":{"owner":d.source().owner().get(),"fingerprint":hex(&d.source().fingerprint()),"font":font(d.source().font()),"advance":d.source().advance().get().raw(),"full_start":d.source().provenance().text_span().range().start_byte().get(),"glyphs":original_glyphs(&d.source().glyph_run().glyphs),"clusters":d.source().glyph_run().clusters.iter().map(|c|json!({"source":source_span(c.source_span,parsed),"glyph_range":[c.glyph_start,c.glyph_end]})).collect::<Vec<_>>()},
        "clusters":d.clusters().iter().map(|c|json!({"cluster":c.cluster_index(),"text":c.exact_text(),"span":span(c.text_span()),"bounds":c.logical_bounds().map(rect),"glyphs":glyphs(c.glyphs())})).collect::<Vec<_>>()})).collect::<Vec<_>>();
    let images=display.images().draws().iter().map(|d|{
        let original=match d.source(){BookV2ImageSource::Vector(b)=>vector_source(b,display.admitted()),BookV2ImageSource::Figure(f)=>figure_source(f)};
        let vector=match d.paint(){BookV2ImagePaint::Raster{..}=>Value::Null,BookV2ImagePaint::Vector{content_key,scale_raw,matrix,color,..}=>json!({"source_sha256":hex(&content_key.source_sha256()),"ir_fingerprint":hex(&content_key.ir_fingerprint()),"scale":scale_raw,"color":color,"matrix":[matrix.a.raw(),matrix.b.raw(),matrix.c.raw(),matrix.d.raw(),matrix.e.raw(),matrix.f.raw()]})};
        json!({"owner":d.source().owner().get(),"fragment":d.fragment_index(),"inline":d.inline_index(),"cell":d.cell_role().map(|r|r.owner().get()),"repeated":d.repeated_header(),"alternative":d.source().alternative(),"image_sha256":hex(&d.image().content_hash()),"viewport":rect(d.paint().viewport()),"vector":vector,"original":original})
    }).collect::<Vec<_>>();
    let mut value = json!({"wire":"typaxis.book-2-column-display-probe/1","case":case,"source_fingerprint":hex(&source.fingerprint()),"source_canonical":source.canonical_bytes(),"original_graph":graph(source.original_graph()),"source_text_buffers":parsed,"admitted_fingerprint":hex(&display.admitted().fingerprint()),"admitted_canonical_jcs":display.admitted().canonical_jcs(),"limits_fingerprint":hex(&limits.fingerprint()),"limits_canonical_jcs":limits.canonical_jcs(),"fingerprint":hex(&display.fingerprint()),
        "component_fingerprints":{"math":hex(&display.math().fingerprint()),"text":hex(&display.text().fingerprint()),"numbers":hex(&display.numbers().fingerprint()),"markers":hex(&display.markers().fingerprint()),"images":hex(&display.images().fingerprint()),"anchors":hex(&display.anchors().fingerprint())},
        "pages":pages,"original_fragments":original,"text":text,"math":math,"numbers":numbers,"markers":markers,"images":images,"separators":display.markers().separators().iter().map(|s|json!({"page":s.page_index(),"before_fragment":s.before_fragment_index(),"ink":rect(s.ink())})).collect::<Vec<_>>(),
        "anchors":display.anchors().positions().iter().map(|a|json!({"owner":a.anchor().source().owner().get(),"fragment":a.fragment_index(),"cell":a.cell_role().map(|r|r.owner().get()),"repeated":a.repeated_header(),"x":a.x().raw(),"baseline":a.baseline().raw()})).collect::<Vec<_>>(),"nonpainting":display.anchors().nonpainting_lines().iter().map(|a|json!({"fragment":a.fragment_index(),"cell":a.cell_role().map(|r|r.owner().get()),"repeated":a.repeated_header()})).collect::<Vec<_>>(),"unpositioned":display.anchors().unpositioned().iter().map(|a|anchor_source(a)).collect::<Vec<_>>(),"original_unpositioned":source.original_graph().lines().paragraphs().iter().flat_map(|p|p.anchors()).filter(|a|a.position().is_none()).map(anchor_source).collect::<Vec<_>>(),"paints":display.paints().iter().copied().map(paint).collect::<Vec<_>>(),"observed":{"scope":scope,"maximum_work":maximum_work,"builder_prior":builder_prior,"source_records":source.record_charge(),"source_work":source.work_steps(),"display_records":display.record_charge(),"display_work":display.work_steps(),"driver_records":observed.record_charge(),"driver_work":observed.work_steps(),"driver_spool":observed.spool_charge(),"balance_candidates":observed.balance_candidates()}});
    value["selection_mode"] = if scope == "retained_source_cut" {
        "retained_source_cut"
    } else {
        "automatic_driver"
    }
    .into();
    value["source_package_fingerprint"] = hex(&source
        .original_graph()
        .lines()
        .prepared()
        .source_flow()
        .body()
        .body()
        .canonical_jcs_sha256())
    .into();
    value["source_measurements_fingerprint"] = hex(&source
        .source()
        .stable()
        .sequence()
        .measurements_fingerprint())
    .into();
    let closed = source.source();
    value["source_coverage"] = json!({"semantic_fragments":closed.semantic_fragments(),"repeated_fragments":closed.repeated_fragments(),"header_variant_fragments":closed.header_variant_fragments(),"unreferenced_definitions":closed.unreferenced_definitions(),"semantic_math":closed.semantic_math(),"repeated_math":closed.repeated_math()});
    value["observed"]["source_spool"] = source.spool_charge().into();
    if !has_driver_observation {
        for key in [
            "driver_records",
            "driver_work",
            "driver_spool",
            "balance_candidates",
        ] {
            value["observed"][key] = Value::Null;
        }
    }
    value
}
