use super::*;
use typaxis_display_list::book_v2::BookV2AssemblyDisplay;

fn artifact(directory: &std::path::Path, bytes: &[u8], role: &str) -> Value {
    let hash = probes::hex(&typaxis_core::sha256(bytes));
    let name = format!("{role}-{hash}.bin");
    fs::write(directory.join(&name), bytes).unwrap();
    json!({"file":name,"sha256":hash,"length":bytes.len()})
}

pub(super) struct Pending {
    directory: std::path::PathBuf,
    case: String,
    value: Value,
}
pub(super) fn finish(mut pending: Option<Pending>, caller_after: BookV2PdfConvergenceObservation) {
    let Some(mut pending) = pending.take() else {
        return;
    };
    pending.value["observed"]["caller_after"] = json!(pdf_quartet(caller_after));
    write_pending(pending);
}
fn write_pending(pending: Pending) {
    let bytes = serde_json::to_vec_pretty(&pending.value).unwrap();
    let hash = probes::hex(&typaxis_core::sha256(&bytes));
    fs::write(
        pending
            .directory
            .join(format!("{}-{hash}.json", pending.case)),
        bytes,
    )
    .unwrap();
}

pub(super) fn emit(
    case: &str,
    bundle: &crate::book_v2_resources::BookV2ColumnPdfBundle<
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
        '_,
    >,
    caps: &M4EffectiveResourceLimits,
    observed: BookV2PdfConvergenceObservation,
    local: &[Value],
    sizes: [usize; 4],
) -> Option<Pending> {
    let root = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE")?;
    let directory = std::path::PathBuf::from(root).join("column-pdf");
    fs::create_dir_all(&directory).unwrap();
    let parent = region_probes::snapshot_nav(
        &directory,
        case,
        bundle.navigation_bundle(),
        caps,
        bundle.navigation_prefix(),
        local,
    );
    let context = json!({"scope":"pdf_driver","parent_scope":"navigation_driver","maximum_work":MAXIMUM,"constructors":bundle.constructor_charges(),"caller_before":bundle.caller_before(),"previous":pdf_quartet(bundle.previous_observation()),"navigation_prefix":pdf_quartet(bundle.navigation_prefix()),"build_after":pdf_quartet(observed),"caller_after":null});
    emit_roots(case, bundle.pdf(), caps, sizes, parent, context)
}

pub(super) fn emit_retained(
    case: &str,
    pdf: &typaxis_pdf::book_v2::BookV2ColumnPdfAssembly<
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
    sizes: [usize; 4],
    constructors: [[u64; 4]; 14],
    parent: Value,
) {
    let nav = pdf.navigation();
    let context = json!({"scope":"retained_source_cut","parent_scope":"retained_source_cut","maximum_work":MAXIMUM,"constructors":constructors,"caller_before":null,"previous":null,"navigation_prefix":null,"source_prefix":[nav.record_charge(),nav.spool_charge(),nav.output_charge(),nav.work_steps()],"build_after":[pdf.record_charge(),pdf.spool_charge(),pdf.output_charge(),pdf.work_steps()],"caller_after":null});
    if let Some(pending) = emit_roots(case, pdf, caps, sizes, parent, context) {
        write_pending(pending);
    }
}

fn emit_roots(
    case: &str,
    pdf: &typaxis_pdf::book_v2::BookV2ColumnPdfAssembly<
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
    sizes: [usize; 4],
    parent: Value,
    context: Value,
) -> Option<Pending> {
    let root = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE")?;
    let directory = std::path::PathBuf::from(root).join("column-pdf");
    fs::create_dir_all(&directory).unwrap();
    let value = snapshot_roots(
        &directory,
        case,
        pdf,
        caps,
        sizes,
        parent,
        context,
        "typaxis.book-2-column-pdf-probe/1",
        MAXIMUM,
    );
    Some(Pending {
        directory,
        case: case.to_owned(),
        value,
    })
}

pub(super) fn snapshot_roots(
    directory: &std::path::Path,
    case: &str,
    pdf: &typaxis_pdf::book_v2::BookV2ColumnPdfAssembly<
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
    sizes: [usize; 4],
    parent: Value,
    context: Value,
    wire: &str,
    maximum_work: u64,
) -> Value {
    let navigation = pdf.navigation();
    let relations = navigation.source();
    let structure = relations.source();
    let marked = structure.source();
    let scopes = marked.source();
    let display = scopes.display();
    let original = display
        .source()
        .original_graph()
        .lines()
        .prepared()
        .source_flow();
    let view = display.navigation_source();
    let assembly_source = display.assembly_source();
    let pdf_bytes = artifact(&directory, pdf.bytes(), "pdf");
    let nodes = structure.nodes().iter().map(|node| {
        let source = node.source();
        let key = source.key();
        json!({"key":[key.owner().get(),format!("{:?}",key.slot())],"parent":node.parent(),"depth":node.depth(),"first_child":node.first_child(),"next_sibling":node.next_sibling(),"first_binding":node.first_binding(),"role":source.pdf_role(),"semantic_kind":source.semantic_kind(),"language":source.language(),"alt":source.alternative(),"span":source.source_span().map(|s|[s.source_id().get(),s.start_byte().get(),s.end_byte().get()]),"cell":source.table_cell().map(|c|json!({"table":c.table.get(),"row":c.row,"column":c.column,"colspan":c.colspan,"rowspan":c.rowspan,"header":c.header}))})
    }).collect::<Vec<_>>();
    let bindings = structure.bindings().iter().map(|b|json!({"node":b.node_index(),"group":b.group_index(),"page":b.page_index(),"mcid":b.mcid(),"next":b.next_for_node()})).collect::<Vec<_>>();
    let groups = scopes.groups().iter().enumerate().map(|(i,g)|json!({"owner":g.owner().map(|n|n.get()),"role":format!("{:?}",g.role()),"page":g.page_index(),"bytes":String::from_utf8(marked.group_bytes(i).unwrap().to_vec()).unwrap()})).collect::<Vec<_>>();
    let raw_groups = scopes.groups().iter().map(|g|json!({"page":g.page_index(),"fragment":g.fragment_index(),"artifact":g.artifact().is_some(),"paints":g.paints().map(|i|if g.artifact().is_some(){None}else{component_bounds(display,i).map(raw_rect)}).collect::<Vec<_>>()})).collect::<Vec<_>>();
    let nonpainting = display.anchors().nonpainting_lines().iter().map(|line| {
        let fragment = line.fragment().fragment();
        let typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine{paragraph_index,line_index} = fragment.source() else {panic!("nonpainting original paragraph");};
        let lines = display.source().fragment_lines(line.fragment_index()).unwrap();
        let selected = &lines.paragraphs()[paragraph_index as usize].lines()[line_index as usize];
        let breaks = selected.items().iter().map(|item| {
            let ProductionPlacedInline::Break(value) = item else {panic!("nonpainting control");}; value.owner().get()
        }).collect::<Vec<_>>();
        json!({"owner":fragment.owner().get(),"page":fragment.page_index(),"fragment":line.fragment_index(),"line":line_index,"breaks":breaks,"bounds":raw_rect(fragment.bounds()),"repeated":line.repeated_header()})
    }).collect::<Vec<_>>();
    let page_names = display
        .source()
        .source()
        .geometry()
        .pages()
        .iter()
        .map(|p| p.selection().named_page())
        .collect::<Vec<_>>();
    let pages = display.source().source().geometry().pages().iter().enumerate().map(|(i,p)| {
        let candidate = p.selection().candidate();
        let frames = candidate.frames();
        let selected = assembly_source.select_page_master(i,&mut 0,maximum_work).unwrap();
        json!({"index":i,"named_page":p.selection().named_page(),"named_page_index":frames.named_page_index(),"master_id":selected.master().master_id,"full_body":raw_rect(frames.body()),"declared_note":frames.footnote().map(raw_rect),"columns":p.columns().iter().map(|c|raw_rect(c.bounds())).collect::<Vec<_>>(),"balance":p.selection().balance().map(|b|json!({"fingerprint":probes::hex(&b.fingerprint()),"full_height":b.full_height().get().raw(),"selected_height":b.selected_height().get().raw()}))})
    }).collect::<Vec<_>>();
    let reference_candidates = assembly_source
        .page_reference_values()
        .unwrap_or(&[])
        .iter()
        .map(|(owner, page)| [owner.get(), *page])
        .collect::<Vec<_>>();
    let references = pdf.page_references().iter().map(|r|json!({"owner":r.owner().get(),"candidate":r.candidate_page(),"target":r.target_page(),"placed":r.is_placed()})).collect::<Vec<_>>();
    let assembly = json!({"pdf":pdf_bytes["file"],"hash":typaxis_core::sha256(pdf.bytes()),"bytes":pdf.bytes().len(),"catalog":pdf.catalog_object(),"pages":(0..pages.len()).map(|i|pdf.page_object(i).unwrap()).collect::<Vec<_>>(),"page_names":page_names,"page_frames":pages.iter().map(|p|p["full_body"].clone()).collect::<Vec<_>>(),"footnote_regions":pages.iter().map(|p|p["declared_note"].clone()).collect::<Vec<_>>(),"candidates":reference_candidates,"references":references,"labels_match":pdf.page_reference_labels_match()});
    let nav = json!({"assembly":assembly,"destinations":navigation.destinations().iter().map(|d|json!({"owner":d.owner().get(),"kind":match d.kind(){DestinationKind::Anchor(s)=>json!(["anchor",s]),DestinationKind::Footnote(i)=>json!(["footnote",i]),DestinationKind::Reference(i)=>json!(["reference",i])},"position":d.position().map(|p|[p.page_index() as i64,p.fragment_index() as i64,p.x().raw(),p.y().raw()])})).collect::<Vec<_>>(),"links":navigation.links().iter().map(|l|json!({"node":l.structure_node(),"target":match l.target(){Target::Destination(i)=>json!(["destination",i]),Target::Uri(s)=>json!(["uri",s])},"first":l.first_rectangle()})).collect::<Vec<_>>(),"rectangles":navigation.rectangles().iter().map(|r|json!({"link":r.link_index(),"page":r.page_index(),"fragment":r.fragment_index(),"group":r.first_group(),"bounds":raw_rect(r.bounds()),"next":r.next_for_link()})).collect::<Vec<_>>(),"page_counts":(0..pages.len()).map(|i|navigation.page_rectangles(i).unwrap().len()).collect::<Vec<_>>(),"outline":navigation.outline().iter().map(|o|json!({"destination":o.destination(),"parent":o.parent(),"first":o.first_child(),"last":o.last_child(),"previous":o.previous(),"next":o.next(),"descendants":o.descendant_count()})).collect::<Vec<_>>(),"raw_groups":raw_groups,"nonpainting_lines":nonpainting,"raw_page_count":pages.len(),"source_anchors":view.anchors().iter().map(|(id,owner)|json!([id.as_str(),owner.get()])).collect::<Vec<_>>(),"inline_anchors":display.anchors().positions().iter().map(|a|json!({"owner":a.anchor().source().owner().get(),"page":a.fragment().fragment().page_index(),"fragment":a.fragment_index(),"x":a.x().raw(),"y":a.baseline().raw(),"repeated":a.repeated_header()})).collect::<Vec<_>>(),"source_outline":view.outline().iter().map(|o|json!({"id":o.outline_id,"parent":o.parent_outline_id,"level":o.level,"target":o.destination.as_str(),"label":o.label})).collect::<Vec<_>>()});
    let relation = json!({"navigation":nav,"nodes":relations.nodes().iter().enumerate().map(|(i,n)|json!({"parent":n.parent(),"first_child":n.first_child(),"next":n.next_sibling(),"depth":n.depth(),"headers":relations.headers(i).unwrap(),"related":relations.related_nodes(i).unwrap(),"list_numbering":n.list_numbering().map(|v|format!("{v:?}"))})).collect::<Vec<_>>(),"order":relations.reading_order(),"notes":relations.notes().iter().map(|n|json!({"node":n.node_index(),"link":n.link_node(),"label":n.label_node(),"painted":n.painted(),"first":n.first_reference(),"return":n.return_reference(),"reading":n.reading_reference(),"after":n.insertion_after()})).collect::<Vec<_>>(),"edges":relations.edges().iter().map(|e|json!({"reference":e.reference_node(),"note":e.note_node(),"definition":e.definition_index(),"source_definition":e.source_definition(),"ref_link":e.reference_link(),"ref_label":e.reference_label(),"def_link":e.definition_link(),"def_label":e.definition_label(),"painted":e.painted(),"next":e.next_for_note()})).collect::<Vec<_>>(),"first_demand":(0..relations.notes().len()).map(|i|display.source().source().stable().sequence().pages().last().and_then(|p|p.next_state().source_state().first_reference(i)).map(|n|n.get())).collect::<Vec<_>>()});
    let structural_probe = json!({"display":display.fingerprint(),"wire":package_wire(display),"nodes":nodes,"bindings":bindings,"groups":groups,"relations":relation});
    let font_objects = marked.text().source();
    let image_objects = scopes.source().source();
    let cids = font_objects.source().source();
    let engine = typaxis_core::EngineIdentity::compiled();
    let mut value = json!({"wire":wire,"case":case,"source_canonical_jcs":typaxis_document_package::book_v2::BookV2DocumentPackageEncoder::new().encode(original.body().body().wire()).unwrap(),"limits_canonical_jcs":caps.canonical_jcs(),"limits_fingerprint":probes::hex(&caps.fingerprint()),"pages":pages,"structural_probe":structural_probe,"parent":parent,"navigation":{"fingerprint":probes::hex(&navigation.fingerprint()),"charges":[navigation.record_charge(),navigation.spool_charge(),navigation.output_charge(),navigation.work_steps()]},"engine":{"name":engine.name(),"version":engine.version(),"rust_version":engine.rust_version(),"git_commit":engine.git_commit()},"size_of":{"selected_page_master":sizes[0],"object_range":sizes[1],"page_reference":sizes[2],"option_usize":sizes[3],"usize":std::mem::size_of::<usize>()},"observed":context});
    value["prior_stages"] = json!({
        "syntax_flow_fingerprint":probes::hex(&original.fingerprint()),
        "scopes":{"fingerprint":probes::hex(&scopes.fingerprint()),"charges":[scopes.record_charge(),scopes.spool_charge(),scopes.output_charge(),scopes.work_steps()]},
        "marked":{"fingerprint":probes::hex(&marked.fingerprint()),"charges":[marked.record_charge(),marked.spool_charge(),marked.output_charge(),marked.work_steps()]},
        "structure":{"fingerprint":probes::hex(&structure.fingerprint()),"charges":[structure.record_charge(),structure.spool_charge(),structure.output_charge(),structure.work_steps()]},
        "relations":{"fingerprint":probes::hex(&relations.fingerprint()),"charges":[relations.record_charge(),relations.spool_charge(),relations.output_charge(),relations.work_steps()]},
        "navigation":{"fingerprint":probes::hex(&navigation.fingerprint()),"charges":[navigation.record_charge(),navigation.spool_charge(),navigation.output_charge(),navigation.work_steps()]}
    });
    value["assembly"] = json!({"pdf":pdf_bytes,"fingerprint":probes::hex(&pdf.fingerprint()),"charges":[pdf.record_charge(),pdf.spool_charge(),pdf.output_charge(),pdf.work_steps()],"first":pdf.first_object(),"next":pdf.next_object(),"objects":(0..pdf.next_object()).map(|id|pdf.object_range(id).map(|r|[r.start,r.end])).collect::<Vec<_>>(),"catalog":pdf.catalog_object(),"pages":(0..pages.len()).map(|i|pdf.page_object(i).unwrap()).collect::<Vec<_>>(),"candidates":structural_probe["relations"]["navigation"]["assembly"]["candidates"],"references":structural_probe["relations"]["navigation"]["assembly"]["references"],"labels_match":pdf.page_reference_labels_match()});
    value["font_objects"] = json!({"fingerprint":probes::hex(&font_objects.fingerprint()),"bytes":artifact(&directory,font_objects.bytes(),"font-objects"),"first":font_objects.first_object(),"next":font_objects.next_object(),"objects":font_objects.objects().iter().map(|o|json!({"id":o.id().get(),"font":o.font_index(),"role":format!("{:?}",o.role()),"range":[o.byte_range().start,o.byte_range().end]})).collect::<Vec<_>>(),"resources":cids.fonts().iter().enumerate().map(|(i,f)|json!({"index":i,"instance":f.font_instance_id().get(),"object":font_objects.font_object(i).unwrap().get()})).collect::<Vec<_>>()});
    value["image_objects"] = json!({"fingerprint":probes::hex(&image_objects.fingerprint()),"bytes":artifact(&directory,image_objects.bytes(),"image-objects"),"first":image_objects.first_object(),"next":image_objects.next_object(),"objects":image_objects.objects().iter().map(|o|json!({"id":o.id().get(),"image":o.image_index(),"state":o.state_index(),"role":format!("{:?}",o.role()),"range":[o.byte_range().start,o.byte_range().end]})).collect::<Vec<_>>(),"resources":(0..image_objects.source().source().source().images().len()).map(|i|json!({"index":i,"object":image_objects.resource_object(i).unwrap().get()})).collect::<Vec<_>>()});
    value["marked"] = json!({"fingerprint":probes::hex(&marked.fingerprint()),"byte_length":marked.byte_length(),"next_object":marked.next_object(),"anchor_font":marked.anchor_font_object().map(|o|o.get()),"anchor_bytes":artifact(&directory,marked.anchor_object_bytes(),"anchor-objects"),"anchor_objects":marked.anchor_objects().iter().map(|o|json!({"id":o.id().get(),"role":format!("{:?}",o.role()),"range":[o.byte_range().start,o.byte_range().end]})).collect::<Vec<_>>(),"pages":marked.pages().iter().enumerate().map(|(i,p)|json!({"index":p.page_index(),"groups":[p.groups().start,p.groups().end],"bytes":artifact(&directory,marked.page_bytes(i).unwrap(),"marked-page")})).collect::<Vec<_>>()});
    value["marked"]["anchors"] = json!(marked.anchors().iter().map(|a|json!({"page":a.page_index(),"group":a.group_index(),"paint":a.paint_index(),"viewport":raw_rect(a.viewport()),"baseline":a.baseline().raw()})).collect::<Vec<_>>());
    value["marked"]["groups"] = json!((0..scopes.groups().len())
        .map(|i| artifact(&directory, marked.group_bytes(i).unwrap(), "marked-group"))
        .collect::<Vec<_>>());
    value
}
