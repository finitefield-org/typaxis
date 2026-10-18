use super::*;
use typaxis_resources::{AdmittedImage, AdmittedImageMediaKind};

pub(super) type ImageKey = (u8, [u8; 32], String, String, [u8; 32]);
pub(super) fn image_key(image: &AdmittedImage) -> ImageKey {
    let kind = match image.media_kind() {
        AdmittedImageMediaKind::Png => 0,
        AdmittedImageMediaKind::JpegBaseline => 1,
        AdmittedImageMediaKind::SafeVector => 2,
        AdmittedImageMediaKind::SafeVector2 => 3,
    };
    let vector = VectorContentKey::from_admitted(image).ok();
    (
        kind,
        image.content_hash(),
        vector.map(|v| v.parser_id().into()).unwrap_or_default(),
        vector.map(|v| v.ir_id().into()).unwrap_or_default(),
        vector.map(|v| v.ir_fingerprint()).unwrap_or([0; 32]),
    )
}
fn artifact(directory: &std::path::Path, bytes: &[u8], kind: &str) -> Value {
    let hash = probes::hex(&typaxis_core::sha256(bytes));
    let name = format!("{kind}-{hash}.bin");
    fs::write(directory.join(&name), bytes).unwrap();
    json!({"file":name,"sha256":hash,"length":bytes.len()})
}
fn text(value: BookV2FontUseText<'_>) -> String {
    match value {
        BookV2FontUseText::Text(s) => s.into(),
        BookV2FontUseText::Scalar(c) => c.to_string(),
    }
}
fn source(value: BookV2FontUseSource) -> Value {
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
fn paint(value: Paint) -> Value {
    let (tag, i) = match value {
        Paint::PageRegionText { .. } => panic!("running region outside the legacy Column fixture"),
        Paint::Text(i) => ("text", i),
        Paint::Marker(i) => ("marker", i),
        Paint::EquationNumber(i) => ("number", i),
        Paint::Math(i) => ("math", i),
        Paint::Image(i) => ("image", i),
        Paint::FootnoteSeparator(i) => ("separator", i),
    };
    json!([tag, i])
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

pub(super) fn emit(
    case: &str,
    cids: &BookV2ColumnCidPlans<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    rasters: &BookV2ColumnRasterPrograms<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    caps: &M4EffectiveResourceLimits,
    observed: Option<BookV2PdfConvergenceObservation>,
    scope: &str,
    maximum_work: u64,
    constructors: [[u64; 3]; 2],
    reservation_sizes: [usize; 2],
    caller_before: Option<[u64; 3]>,
) {
    let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory).join("column-resources");
    fs::create_dir_all(&directory).unwrap();
    let programs = cids.source();
    let closures = programs.source();
    let fonts = closures.selection();
    let display = fonts.display();
    let images = rasters.source();
    let display_scope = if scope == "resource_driver" {
        "display_callback"
    } else {
        scope
    };
    let snapshot = probes::snapshot(
        case,
        display,
        caps,
        observed,
        display_scope,
        maximum_work,
        if scope == "resource_driver" {
            None
        } else {
            Some((0, 0))
        },
    );
    let original = display
        .source()
        .original_graph()
        .lines()
        .prepared()
        .shaped()
        .font_instances();
    let origins = display.paints().iter().copied().enumerate().filter_map(|(index,p)|match p {
        Paint::Text(i)=>Some(json!({"paint":index,"draw":["text",i],"instance":display.text().draws()[i].cluster().run().glyph_run().font.get()})),
        Paint::Marker(i)=>Some(json!({"paint":index,"draw":["marker",i],"instance":display.markers().draws()[i].source().glyph_run().font.get()})),
        Paint::EquationNumber(i)=>Some(json!({"paint":index,"draw":["number",i],"instance":display.numbers().draws()[i].shape().font_instance_id().get()})),
        Paint::Math(i)=>match display.math().draws()[i].terminal().source() {BookV2BodyMathSource::Native(r)=>Some(json!({"paint":index,"draw":["math",i],"instance":r.font_instance_id().get()})),_=>None},
        _=>None,
    }).collect::<Vec<_>>();
    let value = json!({
        "wire":"typaxis.book-2-column-resources-probe/1","case":case,"display_probe":snapshot,
        "original_font_instances":{"fingerprint":probes::hex(&original.fingerprint()),"count":original.len(),"instances":(0..original.len()).map(|i|{let instance=original.resolve(typaxis_core::FontInstanceId::new(i as u32)).unwrap();json!({"id":i,"face_id":instance.font().font_face_id().get(),"sha256":probes::hex(&instance.font().content_hash()),"face_index":instance.font().face_index(),"table_fingerprint":probes::hex(&instance.table_fingerprint())})}).collect::<Vec<_>>()},
        "original_font_origins":origins,
        "original_images":display.admitted().images().iter().map(|i|{let k=image_key(i);json!({"id":i.image_id().get(),"media":i.media_kind().as_str(),"sha256":probes::hex(&i.content_hash()),"source":artifact(&directory,i.bytes(),"image-source"),"width":i.width().get(),"height":i.height().get(),"decoded_bytes":i.decoded_bytes(),"key":[k.0,probes::hex(&k.1),k.2,k.3,probes::hex(&k.4)]})}).collect::<Vec<_>>(),
        "fonts_selection":{"fingerprint":probes::hex(&fonts.fingerprint()),"charges":[fonts.record_charge(),fonts.spool_charge(),fonts.work_steps()],
            "fonts":fonts.fonts().iter().enumerate().map(|(i,f)|{let instance=f.instance();let original=instance.font();json!({"instance":instance.font_instance_id().get(),"table_fingerprint":probes::hex(&instance.table_fingerprint()),"face_id":original.font_face_id().get(),"face_index":original.face_index(),"source":artifact(&directory,original.bytes(),"font-source"),"selected":fonts.glyphs(i).unwrap().map(|g|g.get()).collect::<Vec<_>>(),"units_per_em":original.metadata().units_per_em,"glyph_count":original.metadata().glyph_count})}).collect::<Vec<_>>(),
            "uses":fonts.uses().iter().map(|u|{let usage=u.usage();json!({"paint":u.paint_index(),"typed_paint":paint(usage.paint()),"slot":usage.slot(),"instance":usage.instance().font_instance_id().get(),"source":source(usage.source()),"text":text(usage.text()),"glyphs":(0..usage.glyphs().len()).map(|i|usage.glyphs().get(i).unwrap().get()).collect::<Vec<_>>(),"size":usage.size().get().raw()})}).collect::<Vec<_>>()},
        "closures":{"fingerprint":probes::hex(&closures.fingerprint()),"charges":[closures.record_charge(),closures.spool_charge(),closures.work_steps()],"fonts":closures.fonts().iter().map(|f|json!({"fingerprint":probes::hex(&f.fingerprint()),"kind":format!("{:?}",f.kind()),"mapping":f.glyphs().map(|g|json!([g.get(),f.subset_gid(g).unwrap().get()])).collect::<Vec<_>>()})).collect::<Vec<_>>()},
        "programs":{"fingerprint":probes::hex(&programs.fingerprint()),"charges":[programs.record_charge(),programs.spool_charge(),programs.work_steps()],"fonts":programs.fonts().iter().map(|f|json!({"fingerprint":probes::hex(&f.fingerprint()),"program":artifact(&directory,f.bytes(),"font-program"),"sha256":probes::hex(&f.sha256()),"postscript_name":f.postscript_name(),"advances":f.source().glyphs().map(|g|json!([g.get(),f.advance(g)])).collect::<Vec<_>>(),"metrics":{"ascent":f.metrics().ascent_1000,"descent":f.metrics().descent_1000,"cap_height":f.metrics().cap_height_1000,"stem_v":f.metrics().stem_v_1000,"italic_angle":f.metrics().italic_angle_fixed_16_16,"flags":f.metrics().flags,"bbox":f.metrics().bbox_1000}})).collect::<Vec<_>>()},
        "cids":{"fingerprint":probes::hex(&cids.fingerprint()),"charges":[cids.record_charge(),cids.spool_charge(),cids.work_steps()],"fonts":cids.fonts().iter().enumerate().map(|(i,f)|json!({"instance":f.font_instance_id().get(),"bindings":cids.bindings(i).unwrap().iter().map(|b|json!({"gid":b.original_gid().get(),"cid":b.cid().get(),"subset":b.subset_gid().get(),"width":b.width_1000(),"unicode":b.unicode().map(|c|c as u32)})).collect::<Vec<_>>()})).collect::<Vec<_>>(),"uses":cids.uses().iter().enumerate().map(|(i,u)|json!({"paint":u.source().paint_index(),"slot":u.source().usage().slot(),"font":u.font_index(),"cids":cids.cids(i).unwrap().iter().map(|c|c.get()).collect::<Vec<_>>(),"requires_actual_text":u.requires_actual_text(),"actual_text":u.actual_text().map(text)})).collect::<Vec<_>>()},
        "images_selection":{"fingerprint":probes::hex(&images.fingerprint()),"charges":[images.record_charge(),images.spool_charge(),images.work_steps()],"prior":images.prior_charges(),"images":images.images().iter().map(|i|{let i=i.image();let k=image_key(i);json!({"id":i.image_id().get(),"media":i.media_kind().as_str(),"key":[k.0,probes::hex(&k.1),k.2,k.3,probes::hex(&k.4)],"source":artifact(&directory,i.bytes(),"image-source"),"width":i.width().get(),"height":i.height().get(),"decoded_bytes":i.decoded_bytes()})}).collect::<Vec<_>>(),"uses":images.uses().iter().map(|u|json!({"paint":u.paint_index(),"typed_paint":paint(u.usage().paint()),"resource":u.resource_index(),"image_id":u.usage().image().image_id().get(),"owner":u.usage().owner().get(),"page":u.usage().page_index(),"geometry":geometry(u.usage().geometry())})).collect::<Vec<_>>()},
        "rasters":{"fingerprint":probes::hex(&rasters.fingerprint()),"charges":[rasters.record_charge(),rasters.spool_charge(),rasters.work_steps()],"programs":(0..images.images().len()).map(|i|rasters.program(i).map(|r|json!({"fingerprint":probes::hex(&r.fingerprint()),"program":artifact(&directory,r.bytes(),"raster-program"),"sha256":probes::hex(&r.sha256()),"width":r.width(),"height":r.height(),"color":format!("{:?}",r.color_space()),"encoding":format!("{:?}",r.encoding()),"alpha":r.alpha_mask().map(|a|json!({"program":artifact(&directory,&a.encoded_bytes,"raster-alpha"),"width":a.width.get(),"height":a.height.get(),"encoding":format!("{:?}",a.encoding)})),"jpeg":r.jpeg().map(|j|json!({"source_sha256":probes::hex(&j.source_sha256()),"normalized_sha256":probes::hex(&j.normalized_sha256()),"width":j.width().get(),"height":j.height().get(),"decoded_bytes":j.decoded_byte_length(),"color":format!("{:?}",j.color_kind())}))}))).collect::<Vec<_>>()},
        "observed":{"scope":scope,"maximum_work":maximum_work,"constructors":constructors,"caller_before":caller_before,"caller_after":if scope=="resource_driver" {observed.map(|o|[o.record_charge(),o.spool_charge(),o.work_steps()])} else {None},"ancestor_callback":observed.map(|o|[o.record_charge(),o.spool_charge(),o.work_steps()])},
        "size_of":{"font_glyph":reservation_sizes[0],"sfnt_rewrite_table":reservation_sizes[1],"usize":std::mem::size_of::<usize>(),"selected_font":std::mem::size_of::<typaxis_resources::book_v2::BookV2SelectedFont<'_>>(),"selected_font_use":std::mem::size_of::<typaxis_resources::book_v2::BookV2ColumnSelectedFontUse<'_,'_>>(),"closed_font":std::mem::size_of::<typaxis_resources::book_v2::BookV2ClosedFont<'_,'_>>(),"font_program":std::mem::size_of::<typaxis_resources::book_v2::BookV2FontProgram<'_,'_,'_>>(),"cid_font":std::mem::size_of::<typaxis_resources::book_v2::BookV2CidFont<'_,'_,'_,'_>>(),"binding":std::mem::size_of::<typaxis_resources::book_v2::BookV2CidBinding>(),"cid_use":std::mem::size_of::<typaxis_resources::book_v2::BookV2ColumnCidUse<'_,'_,'_>>(),"cid":std::mem::size_of::<typaxis_font::Cid>(),"selected_image":std::mem::size_of::<typaxis_resources::book_v2::BookV2SelectedImage<'_>>(),"selected_image_use":std::mem::size_of::<typaxis_resources::book_v2::BookV2ColumnSelectedImageUse<'_>>(),"raster_slot":std::mem::size_of::<Option<typaxis_resources::book_v2::BookV2RasterProgram<'_, '_>>>()}
    });
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    fs::write(
        directory.join(format!(
            "{case}-{}.json",
            probes::hex(&typaxis_core::sha256(&bytes))
        )),
        bytes,
    )
    .unwrap();
}
