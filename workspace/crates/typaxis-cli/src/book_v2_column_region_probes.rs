use super::*;

fn typed_paint(value: Paint) -> Value {
    match value {
        Paint::PageRegionText { region_draw } => json!(["region", region_draw]),
        Paint::Text(i) => json!(["text", i]),
        Paint::Marker(i) => json!(["marker", i]),
        Paint::Math(i) => json!(["math", i]),
        Paint::Image(i) => json!(["image", i]),
        Paint::EquationNumber(i) => json!(["number", i]),
        Paint::FootnoteSeparator(i) => json!(["separator", i]),
    }
}
fn artifact(directory: &std::path::Path, bytes: &[u8], kind: &str) -> Value {
    let hash = probes::hex(&typaxis_core::sha256(bytes));
    let name = format!("{kind}-{hash}.bin");
    fs::write(directory.join(&name), bytes).unwrap();
    json!({"file":name,"sha256":hash,"length":bytes.len()})
}
fn use_text(value: typaxis_display_list::book_v2::BookV2FontUseText<'_>) -> String {
    match value {
        typaxis_display_list::book_v2::BookV2FontUseText::Text(s) => s.into(),
        typaxis_display_list::book_v2::BookV2FontUseText::Scalar(c) => c.to_string(),
    }
}
fn font_source(value: BookV2FontUseSource) -> Value {
    match value {
        BookV2FontUseSource::Text(s) => {
            json!({"kind":"text","span":[s.text_id().get(),s.range().start_byte().get(),s.range().end_byte().get()]})
        }
        BookV2FontUseSource::NativeMath {
            owner,
            receipt,
            computation,
            paint_index,
            logical_ordinal,
        } => {
            json!({"kind":"native","owner":owner.get(),"receipt":probes::hex(&receipt),"computation":probes::hex(&computation),"paint_index":paint_index,"logical_ordinal":logical_ordinal})
        }
    }
}
fn geometry(value: BookV2ImagePaint) -> Value {
    match value {
        BookV2ImagePaint::Raster { viewport } => json!({"kind":"raster","viewport":rect(viewport)}),
        BookV2ImagePaint::Vector {
            content_key,
            viewport,
            scale_raw,
            matrix,
            color,
        } => {
            json!({"kind":"vector","viewport":rect(viewport),"scale":scale_raw,"matrix":[matrix.a.raw(),matrix.b.raw(),matrix.c.raw(),matrix.d.raw(),matrix.e.raw(),matrix.f.raw()],"color":color,"key":{"source_sha256":probes::hex(&content_key.source_sha256()),"parser":content_key.parser_id(),"ir":content_key.ir_id(),"ir_fingerprint":probes::hex(&content_key.ir_fingerprint())}})
        }
    }
}
fn rect(r: typaxis_core::Rect) -> Value {
    json!([
        r.x().raw(),
        r.y().raw(),
        r.width().get().raw(),
        r.height().get().raw()
    ])
}

fn quartet(o: BookV2PdfConvergenceObservation) -> [u64; 4] {
    [
        o.record_charge(),
        o.spool_charge(),
        o.output_charge(),
        o.work_steps(),
    ]
}

pub(super) fn emit_nav(
    case: &str,
    bundle: &crate::book_v2_resources::BookV2ColumnNavigationBundle<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    caps: &M4EffectiveResourceLimits,
    observed: BookV2PdfConvergenceObservation,
    local: &[Value],
) {
    let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory).join("column-running-regions");
    fs::create_dir_all(&directory).unwrap();
    let value = snapshot_nav(&directory, case, bundle, caps, observed, local);
    write(&directory, case, &value);
}

pub(super) fn snapshot_nav(
    directory: &std::path::Path,
    case: &str,
    bundle: &crate::book_v2_resources::BookV2ColumnNavigationBundle<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    caps: &M4EffectiveResourceLimits,
    observed: BookV2PdfConvergenceObservation,
    local: &[Value],
) -> Value {
    snapshot_nav_scoped(
        directory,
        case,
        bundle,
        caps,
        observed,
        local,
        "navigation_driver",
        MAXIMUM,
    )
}

// Data from a real convergence attempt, never an old public navigation callback.
pub(super) fn snapshot_convergence_nav(
    directory: &std::path::Path,
    case: &str,
    bundle: &crate::book_v2_resources::BookV2ColumnNavigationBundle<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    caps: &M4EffectiveResourceLimits,
    observed: BookV2PdfConvergenceObservation,
    local: &[Value],
    maximum_work: u64,
) -> Value {
    snapshot_nav_scoped(
        directory,
        case,
        bundle,
        caps,
        observed,
        local,
        "convergence_attempt",
        maximum_work,
    )
}

fn snapshot_nav_scoped(
    directory: &std::path::Path,
    case: &str,
    bundle: &crate::book_v2_resources::BookV2ColumnNavigationBundle<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    caps: &M4EffectiveResourceLimits,
    observed: BookV2PdfConvergenceObservation,
    local: &[Value],
    scope: &str,
    maximum_work: u64,
) -> Value {
    let display = bundle.display();
    let mut value = snapshot_scoped(case, display, caps, observed, local, scope, maximum_work);
    let paint_bundle = bundle.marked().paints();
    let cids = paint_bundle.cids();
    let programs = cids.source();
    let closures = programs.source();
    let fonts = closures.selection();
    let commands = paint_bundle.text();
    let font_objects = commands.source();
    let streams = font_objects.source();
    let image_commands = paint_bundle.images();
    let image_objects = image_commands.source();
    let vectors = image_objects.source();
    let rasters = vectors.source();
    let images = rasters.source();
    value["fonts_selection"] = json!({"fingerprint":probes::hex(&fonts.fingerprint()),"charges":[fonts.record_charge(),fonts.spool_charge(),fonts.work_steps()],"fonts":fonts.fonts().iter().enumerate().map(|(i,f)|{let original=f.instance().font();json!({"instance":f.instance().font_instance_id().get(),"table_fingerprint":probes::hex(&f.instance().table_fingerprint()),"face_id":original.font_face_id().get(),"face_index":original.face_index(),"source":artifact(&directory,original.bytes(),"font-source"),"selected":fonts.glyphs(i).unwrap().map(|g|g.get()).collect::<Vec<_>>(),"units_per_em":original.metadata().units_per_em,"glyph_count":original.metadata().glyph_count})}).collect::<Vec<_>>(),"uses":fonts.uses().iter().map(|u|{let usage=u.usage();json!({"paint":u.paint_index(),"typed_paint":typed_paint(usage.paint()),"slot":usage.slot(),"instance":usage.instance().font_instance_id().get(),"source":font_source(usage.source()),"text":use_text(usage.text()),"glyphs":(0..usage.glyphs().len()).map(|i|usage.glyphs().get(i).unwrap().get()).collect::<Vec<_>>(),"size":usage.size().get().raw()})}).collect::<Vec<_>>()});
    value["closures"] = json!({"fingerprint":probes::hex(&closures.fingerprint()),"charges":[closures.record_charge(),closures.spool_charge(),closures.work_steps()],"fonts":closures.fonts().iter().map(|f|json!({"fingerprint":probes::hex(&f.fingerprint()),"kind":format!("{:?}",f.kind()),"mapping":f.glyphs().map(|g|json!([g.get(),f.subset_gid(g).unwrap().get()])).collect::<Vec<_>>()})).collect::<Vec<_>>()});
    value["programs"] = json!({"fingerprint":probes::hex(&programs.fingerprint()),"charges":[programs.record_charge(),programs.spool_charge(),programs.work_steps()],"fonts":programs.fonts().iter().map(|f|json!({"fingerprint":probes::hex(&f.fingerprint()),"program":artifact(&directory,f.bytes(),"font-program"),"sha256":probes::hex(&f.sha256()),"postscript_name":f.postscript_name(),"advances":f.source().glyphs().map(|g|json!([g.get(),f.advance(g)])).collect::<Vec<_>>(),"metrics":{"ascent":f.metrics().ascent_1000,"descent":f.metrics().descent_1000,"cap_height":f.metrics().cap_height_1000,"stem_v":f.metrics().stem_v_1000,"italic_angle":f.metrics().italic_angle_fixed_16_16,"flags":f.metrics().flags,"bbox":f.metrics().bbox_1000}})).collect::<Vec<_>>()});
    value["cids"] = json!({"fingerprint":probes::hex(&cids.fingerprint()),"charges":[cids.record_charge(),cids.spool_charge(),cids.work_steps()],"fonts":cids.fonts().iter().enumerate().map(|(i,f)|json!({"instance":f.font_instance_id().get(),"bindings":cids.bindings(i).unwrap().iter().map(|b|json!({"gid":b.original_gid().get(),"cid":b.cid().get(),"subset":b.subset_gid().get(),"width":b.width_1000(),"unicode":b.unicode().map(|c|c as u32)})).collect::<Vec<_>>()})).collect::<Vec<_>>(),"uses":cids.uses().iter().enumerate().map(|(i,u)|json!({"paint":u.source().paint_index(),"slot":u.source().usage().slot(),"font":u.font_index(),"cids":cids.cids(i).unwrap().iter().map(|c|c.get()).collect::<Vec<_>>(),"requires_actual_text":u.requires_actual_text(),"actual_text":u.actual_text().map(use_text)})).collect::<Vec<_>>()});
    value["font_streams"] = json!({"fingerprint":probes::hex(&streams.fingerprint()),"charges":[streams.record_charge(),streams.spool_charge(),streams.output_charge(),streams.work_steps()],"fonts":streams.fonts().iter().enumerate().map(|(i,f)|json!({"font":f.font_index(),"auxiliary_kind":format!("{:?}",f.auxiliary_kind()),"to_unicode":artifact(&directory,streams.to_unicode(i).unwrap(),"font-cmap"),"auxiliary":artifact(&directory,streams.auxiliary(i).unwrap(),"font-auxiliary")})).collect::<Vec<_>>(),"actual_text":(0..cids.uses().len()).map(|i|streams.actual_text(i).map(|s|artifact(&directory,s,"font-actual-text"))).collect::<Vec<_>>()});
    value["font_objects"] = json!({"fingerprint":probes::hex(&font_objects.fingerprint()),"charges":[font_objects.record_charge(),font_objects.spool_charge(),font_objects.output_charge(),font_objects.work_steps()],"first":font_objects.first_object(),"next":font_objects.next_object(),"bytes":artifact(&directory,font_objects.bytes(),"font-objects"),"objects":font_objects.objects().iter().map(|o|json!({"id":o.id().get(),"font":o.font_index(),"role":format!("{:?}",o.role()),"range":[o.byte_range().start,o.byte_range().end]})).collect::<Vec<_>>()});
    value["text_commands"] = json!({"fingerprint":probes::hex(&commands.fingerprint()),"charges":[commands.record_charge(),commands.spool_charge(),commands.output_charge(),commands.work_steps()],"bytes":artifact(&directory,&(0..commands.commands().len()).flat_map(|i|commands.command_bytes(i).unwrap().iter().copied()).collect::<Vec<_>>(),"text-commands"),"commands":commands.commands().iter().enumerate().map(|(i,c)|json!({"paint":c.paint_index(),"typed_paint":typed_paint(display.paints()[c.paint_index()]),"slot":c.slot_index(),"usage":c.usage_index(),"page":c.page_index(),"range":[c.byte_range().start,c.byte_range().end],"actual_text":commands.actual_text(i).map(|s|artifact(&directory,s,"font-actual-text"))})).collect::<Vec<_>>()});
    value["original_images"]=json!(display.admitted().images().iter().map(|i|{let k=paint_probes::image_key(i);json!({"id":i.image_id().get(),"media":i.media_kind().as_str(),"sha256":probes::hex(&i.content_hash()),"source":artifact(&directory,i.bytes(),"image-source"),"width":i.width().get(),"height":i.height().get(),"decoded_bytes":i.decoded_bytes(),"key":[k.0,probes::hex(&k.1),k.2,k.3,probes::hex(&k.4)]})}).collect::<Vec<_>>());
    value["images_selection"] = json!({"fingerprint":probes::hex(&images.fingerprint()),"charges":[images.record_charge(),images.spool_charge(),images.work_steps()],"prior":images.prior_charges(),"images":images.images().iter().map(|i|{let i=i.image();let k=paint_probes::image_key(i);json!({"id":i.image_id().get(),"media":i.media_kind().as_str(),"key":[k.0,probes::hex(&k.1),k.2,k.3,probes::hex(&k.4)],"source":artifact(&directory,i.bytes(),"image-source"),"width":i.width().get(),"height":i.height().get(),"decoded_bytes":i.decoded_bytes()})}).collect::<Vec<_>>(),"uses":images.uses().iter().map(|u|json!({"paint":u.paint_index(),"typed_paint":typed_paint(u.usage().paint()),"resource":u.resource_index(),"image_id":u.usage().image().image_id().get(),"owner":u.usage().owner().get(),"page":u.usage().page_index(),"geometry":geometry(u.usage().geometry())})).collect::<Vec<_>>()});
    value["rasters"] = json!({"fingerprint":probes::hex(&rasters.fingerprint()),"charges":[rasters.record_charge(),rasters.spool_charge(),rasters.work_steps()],"programs":(0..images.images().len()).map(|i|rasters.program(i).map(|r|json!({"fingerprint":probes::hex(&r.fingerprint()),"program":artifact(&directory,r.bytes(),"raster-program"),"sha256":probes::hex(&r.sha256()),"width":r.width(),"height":r.height(),"color":format!("{:?}",r.color_space()),"encoding":format!("{:?}",r.encoding()),"alpha":r.alpha_mask().map(|a|json!({"program":artifact(&directory,&a.encoded_bytes,"raster-alpha"),"width":a.width.get(),"height":a.height.get(),"encoding":format!("{:?}",a.encoding)})),"jpeg":r.jpeg().map(|j|json!({"source_sha256":probes::hex(&j.source_sha256()),"normalized_sha256":probes::hex(&j.normalized_sha256()),"width":j.width().get(),"height":j.height().get(),"decoded_bytes":j.decoded_byte_length(),"color":format!("{:?}",j.color_kind())}))}))).collect::<Vec<_>>()});
    value["vector_programs"] = json!({"fingerprint":probes::hex(&vectors.fingerprint()),"charges":[vectors.record_charge(),vectors.spool_charge(),vectors.output_charge(),vectors.work_steps()],"prior_output":vectors.prior_output_charge(),"programs":(0..images.images().len()).map(|i|vectors.program(i).map(|v|json!({"fingerprint":probes::hex(&v.fingerprint()),"ir":match v.ir(){typaxis_resources::AdmittedSafeVector::V1(ir)=>serde_json::from_str::<Value>(ir.canonical_jcs()).unwrap(),typaxis_resources::AdmittedSafeVector::V2(ir)=>serde_json::from_str::<Value>(ir.canonical_jcs()).unwrap()},"ir_fingerprint":probes::hex(&v.ir().fingerprint()),"bbox":v.bbox(),"content":artifact(&directory,v.content(),"vector-content"),"states":v.states().iter().enumerate().map(|(i,s)|json!({"fill_alpha":s.fill_alpha_raw(),"stroke_alpha":s.stroke_alpha_raw(),"dictionary":artifact(&directory,v.state_dictionary(i).unwrap(),"vector-state")})).collect::<Vec<_>>()}))).collect::<Vec<_>>()});
    value["image_objects"] = json!({"fingerprint":probes::hex(&image_objects.fingerprint()),"charges":[image_objects.record_charge(),image_objects.spool_charge(),image_objects.output_charge(),image_objects.work_steps()],"first":image_objects.first_object(),"next":image_objects.next_object(),"bytes":artifact(&directory,image_objects.bytes(),"image-objects"),"objects":image_objects.objects().iter().map(|o|json!({"id":o.id().get(),"image":o.image_index(),"role":format!("{:?}",o.role()),"state":o.state_index(),"range":[o.byte_range().start,o.byte_range().end]})).collect::<Vec<_>>(),"resources":(0..images.images().len()).map(|i|image_objects.resource_object(i).map(|o|o.get())).collect::<Vec<_>>()});
    value["image_commands"] = json!({"fingerprint":probes::hex(&image_commands.fingerprint()),"charges":[image_commands.record_charge(),image_commands.spool_charge(),image_commands.output_charge(),image_commands.work_steps()],"bytes":artifact(&directory,image_commands.bytes(),"image-commands"),"commands":image_commands.commands().iter().map(|c|json!({"paint":c.paint_index(),"typed_paint":typed_paint(display.paints()[c.paint_index()]),"resource":c.resource_index(),"object":c.resource_object().get(),"page":c.page_index(),"owner":c.owner().get(),"range":[c.byte_range().start,c.byte_range().end]})).collect::<Vec<_>>()});
    value["image_boundary"] = json!({"text_prefix":[commands.record_charge(),commands.spool_charge(),commands.output_charge(),commands.work_steps()],"resource_prior":images.prior_charges(),"vector_prior_output":vectors.prior_output_charge()});
    let origins = display.paints().iter().copied().enumerate().filter_map(|(index,p)|match p {
        Paint::Text(i)=>Some(json!({"paint":index,"draw":["text",i],"instance":display.text().draws()[i].cluster().run().glyph_run().font.get()})),
        Paint::Marker(i)=>Some(json!({"paint":index,"draw":["marker",i],"instance":display.markers().draws()[i].source().glyph_run().font.get()})),
        Paint::EquationNumber(i)=>Some(json!({"paint":index,"draw":["number",i],"instance":display.numbers().draws()[i].shape().font_instance_id().get()})),
        Paint::Math(i)=>match display.math().draws()[i].terminal().source() {BookV2BodyMathSource::Native(r)=>Some(json!({"paint":index,"draw":["math",i],"instance":r.font_instance_id().get()})),_=>None},
        Paint::PageRegionText{region_draw}=>Some(json!({"paint":index,"draw":["region",region_draw],"instance":display.page_regions().iter().flat_map(|r|r.draws()).nth(region_draw).unwrap().font_instance().font_instance_id().get()})),
        _=>None,
    }).collect::<Vec<_>>();
    value["original_font_origins"] = json!(origins);
    let tree = bundle.relations();
    let structure = tree.source();
    let content = bundle.marked().content();
    let scopes = content.source();
    value["marked_scopes"] = json!({"fingerprint":probes::hex(&scopes.fingerprint()),"charges":[scopes.record_charge(),scopes.spool_charge(),scopes.output_charge(),scopes.work_steps()],"groups":scopes.groups().iter().enumerate().map(|(i,g)|json!({"owner":g.owner().map(|n|n.get()),"page":g.page_index(),"artifact":g.artifact().map(|a|format!("{a:?}")),"mcid":g.mcid(),"paints":[g.paints().start,g.paints().end],"source_binding":structure.for_group(i).map(|n|structure.nodes()[n.node_index()].source().key().owner().get())})).collect::<Vec<_>>()});
    value["source_structure_owners"] = json!(structure
        .nodes()
        .iter()
        .map(|n| n.source().key().owner().get())
        .collect::<Vec<_>>());
    value["navigation_owners"] = json!(bundle
        .navigation()
        .destinations()
        .iter()
        .map(|d| d.owner().get())
        .collect::<Vec<_>>());
    value["downstream"] = json!({"encoded_prefix":quartet(bundle.encoded_prefix()),"marked_prefix":quartet(bundle.marked_prefix()),"relations_prefix":quartet(bundle.relations_prefix()),"caller_before":bundle.caller_before(),"constructors":bundle.constructor_charges(),"content":[content.record_charge(),content.spool_charge(),content.output_charge(),content.work_steps()],"structure":[structure.record_charge(),structure.spool_charge(),structure.output_charge(),structure.work_steps()],"relations":[tree.record_charge(),tree.spool_charge(),tree.output_charge(),tree.work_steps()],"navigation":[bundle.navigation().record_charge(),bundle.navigation().spool_charge(),bundle.navigation().output_charge(),bundle.navigation().work_steps()]});
    value["size_of"] = json!({"scope":std::mem::size_of::<typaxis_pdf::book_v2::BookV2MarkedScope>(),"paint":std::mem::size_of::<Paint>(),"usize":std::mem::size_of::<usize>(),"locator":std::mem::size_of::<(usize,usize)>(),"font_glyph":paint_bundle.reservation_sizes()[0],"sfnt_rewrite_table":paint_bundle.reservation_sizes()[1],"selected_font":std::mem::size_of::<typaxis_resources::book_v2::BookV2SelectedFont<'_>>(),"selected_font_use":std::mem::size_of::<typaxis_resources::book_v2::BookV2ColumnSelectedFontUse<'_,'_>>(),"closed_font":std::mem::size_of::<typaxis_resources::book_v2::BookV2ClosedFont<'_,'_>>(),"font_program":std::mem::size_of::<typaxis_resources::book_v2::BookV2FontProgram<'_,'_,'_>>(),"cid_font":std::mem::size_of::<typaxis_resources::book_v2::BookV2CidFont<'_,'_,'_,'_>>(),"binding":std::mem::size_of::<typaxis_resources::book_v2::BookV2CidBinding>(),"cid_use":std::mem::size_of::<typaxis_resources::book_v2::BookV2ColumnCidUse<'_,'_,'_>>(),"cid":std::mem::size_of::<typaxis_font::Cid>(),"font_stream":std::mem::size_of::<typaxis_pdf::book_v2::BookV2FontStream>(),"font_object":std::mem::size_of::<typaxis_pdf::book_v2::BookV2FontObject>(),"text_command":std::mem::size_of::<typaxis_pdf::book_v2::BookV2TextCommand>(),"option_range":std::mem::size_of::<Option<std::ops::Range<usize>>>(),"range":std::mem::size_of::<std::ops::Range<usize>>()});
    value["size_of"]["selected_image"] =
        std::mem::size_of::<typaxis_resources::book_v2::BookV2SelectedImage<'_>>().into();
    value["size_of"]["selected_image_use"] =
        std::mem::size_of::<typaxis_resources::book_v2::BookV2ColumnSelectedImageUse<'_>>().into();
    value["size_of"]["raster_slot"] =
        std::mem::size_of::<Option<typaxis_resources::book_v2::BookV2RasterProgram<'_, '_>>>()
            .into();
    value["size_of"]["vector_slot"] =
        std::mem::size_of::<Option<typaxis_pdf::book_v2::BookV2VectorProgram<'_, '_>>>().into();
    value["size_of"]["vector_state"] =
        std::mem::size_of::<typaxis_pdf::book_v2::BookV2VectorState>().into();
    value["size_of"]["image_object"] =
        std::mem::size_of::<typaxis_pdf::book_v2::BookV2ImageObject>().into();
    value["size_of"]["image_command"] =
        std::mem::size_of::<typaxis_pdf::book_v2::BookV2ImageCommand>().into();
    value
}

fn snapshot_scoped(
    case: &str,
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    caps: &M4EffectiveResourceLimits,
    observed: BookV2PdfConvergenceObservation,
    local: &[Value],
    scope: &str,
    maximum_work: u64,
) -> Value {
    let source = display.source().original_graph().lines().prepared();
    let styled = source.source_flow().body();
    let canonical = typaxis_document_package::book_v2::BookV2DocumentPackageEncoder::new()
        .encode(styled.body().wire())
        .unwrap();
    assert_eq!(
        typaxis_core::sha256(canonical.as_bytes()),
        styled.body().canonical_jcs_sha256()
    );
    let base = BookV2ColumnDisplayBuilder::new(
        display.source(),
        display.admitted(),
        caps,
        maximum_work,
        0,
        0,
    )
    .unwrap()
    .build()
    .unwrap();
    let base_snapshot = probes::snapshot(
        case,
        &base,
        caps,
        None,
        "local_source_companion",
        maximum_work,
        Some((0, 0)),
    );
    let plan = display
        .source()
        .original_graph()
        .lines()
        .frames()
        .unwrap()
        .column_plan()
        .unwrap();
    let mut names = Vec::new();
    while let Some(name) = plan.name(names.len()) {
        names.push(name);
    }
    let pages=display.source().source().geometry().pages().iter().enumerate().map(|(i,p)|{
        let selected=typaxis_syntax::book_v2::select_book_v2_page_master(styled,i as u32,p.selection().named_page(),&mut 0,maximum_work).unwrap();
        json!({"index":i,"named_page":p.selection().named_page(),"selected_master_id":selected.master().master_id,"full_body":raw_rect(p.selection().candidate().frames().body()),"declared_note":p.selection().candidate().frames().footnote().map(raw_rect),"columns":p.columns().iter().map(|c|raw_rect(c.bounds())).collect::<Vec<_>>(),"balance_fingerprint":p.selection().balance().map(|b|probes::hex(&b.fingerprint()))})
    }).collect::<Vec<_>>();
    let regions=display.page_regions().iter().map(|r|json!({"page":r.page_index(),"owner":r.owner().get(),"kind":r.kind().as_str(),"frame":raw_rect(r.frame()),"fingerprint":probes::hex(&r.fingerprint()),"layout_fingerprint":probes::hex(&r.layout_fingerprint()),"records":r.record_charge(),"work":r.work_steps(),"owned_records":r.owned_records(),"projection_work":r.projection_work(),"draws":r.draws().iter().map(|d|json!({"owner":d.owner().get(),"paragraph":d.paragraph_owner().get(),"line":d.line_index(),"text":d.exact_text(),"source":[d.source_span().text_id().get(),d.source_span().start_byte().get(),d.source_span().end_byte().get()],"font":{"instance":d.font_instance().font_instance_id().get(),"instance_fingerprint":probes::hex(&d.font_instance().table_fingerprint()),"face":d.font().face_id().get(),"sha256":probes::hex(&d.font().content_hash()),"index":d.font().face_index(),"size":d.font().size().get().raw(),"ascent":d.font().ascender().raw(),"descent":d.font().descender().raw(),"line_gap":d.font().line_gap().raw()},"bounds":d.logical_bounds().map(raw_rect),"glyphs":d.glyphs().iter().map(|g|json!([g.original_gid().get(),g.x().raw(),g.y().raw()])).collect::<Vec<_>>()})).collect::<Vec<_>>()})).collect::<Vec<_>>();
    let instances = json!({"fingerprint":probes::hex(&source.shaped().font_instances().fingerprint()),"count":source.shaped().font_instances().len(),"instances":(0..source.shaped().font_instances().len()).map(|i|{let f=source.shaped().font_instances().resolve(typaxis_core::FontInstanceId::new(i as u32)).unwrap();json!({"id":i,"face_id":f.font().font_face_id().get(),"sha256":probes::hex(&f.font().content_hash()),"face_index":f.font().face_index(),"table_fingerprint":probes::hex(&f.table_fingerprint())})}).collect::<Vec<_>>()});
    json!({"wire":"typaxis.book-2-column-running-regions-probe/1","case":case,"source_package":package_wire(display),"source_canonical_jcs":canonical,"source_package_fingerprint":probes::hex(&styled.body().canonical_jcs_sha256()),"source_fingerprint":probes::hex(&display.source().fingerprint()),"binding_epoch":probes::hex(&source.shaped().binding_epoch()),"admitted_canonical_jcs":display.admitted().canonical_jcs(),"admitted_fingerprint":probes::hex(&display.admitted().fingerprint()),"display_fingerprint":probes::hex(&display.fingerprint()),"limits_canonical_jcs":caps.canonical_jcs(),"limits_fingerprint":probes::hex(&caps.fingerprint()),"observed":{"scope":scope,"maximum_work":maximum_work,"records":observed.record_charge(),"spool":observed.spool_charge(),"output":observed.output_charge(),"work":observed.work_steps(),"line_passes":observed.line_reshape_passes()},"base_display":base_snapshot,"original_page_names":names,"original_font_instances":instances,"pages":pages,"original_region_lines":local,
        "join":display.page_region_join_observation().map(|j|json!({"base_fingerprint":probes::hex(&j.base_fingerprint()),"base_records":j.base_records(),"base_work":j.base_work(),"prior_records":j.prior_records(),"prior_work":j.prior_work(),"records":display.record_charge(),"work":display.work_steps(),"fingerprint":probes::hex(&display.fingerprint())})),
        "paints":display.paints().iter().copied().map(typed_paint).collect::<Vec<_>>(),
        "semantic_paints":(0..display.paints().len()).map(|i|display.semantic_paint(i).unwrap().map(|s|json!({"paint":i,"page":s.page_index(),"owner":s.owner().map(|n|n.get()),"fragment":s.fragment_index(),"artifact":s.artifact().map(|a|format!("{a:?}")),"alternative":s.alternative(),"actual_text":s.actual_text()}))).collect::<Vec<_>>(),
        "regions":regions})
}
fn write(directory: &std::path::Path, case: &str, value: &Value) {
    let bytes = serde_json::to_vec_pretty(value).unwrap();
    let hash = probes::hex(&typaxis_core::sha256(&bytes));
    fs::write(directory.join(format!("{case}-{hash}.json")), bytes).unwrap();
}
