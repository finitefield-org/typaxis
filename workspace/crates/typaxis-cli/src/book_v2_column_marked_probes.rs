use super::*;
fn artifact(directory: &std::path::Path, bytes: &[u8], kind: &str) -> Value {
    let digest = probes::hex(&typaxis_core::sha256(bytes));
    let file = format!("{kind}-{digest}.bin");
    fs::write(directory.join(&file), bytes).unwrap();
    json!({"file":file,"sha256":digest,"length":bytes.len()})
}
fn rect(r: typaxis_core::Rect) -> [i64; 4] {
    [
        r.x().raw(),
        r.y().raw(),
        r.width().get().raw(),
        r.height().get().raw(),
    ]
}

pub(super) fn emit(
    case: &str,
    content: &BookV2ColumnMarkedContent<
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
    structure: &BookV2ColumnSourceStructure<
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
    encoded: Option<BookV2PdfConvergenceObservation>,
    observed: Option<BookV2PdfConvergenceObservation>,
    scope: &str,
    maximum: u64,
    constructors: [[u64; 4]; 11],
    reservation_sizes: [usize; 2],
    caller_before: Option<[u64; 4]>,
) {
    let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory).join("column-marked-source");
    fs::create_dir_all(&directory).unwrap();
    let value = snapshot(
        &directory,
        case,
        content,
        structure,
        caps,
        encoded,
        observed,
        scope,
        maximum,
        constructors,
        reservation_sizes,
        caller_before,
    );
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

pub(super) fn snapshot(
    directory: &std::path::Path,
    case: &str,
    content: &BookV2ColumnMarkedContent<
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
    structure: &BookV2ColumnSourceStructure<
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
    encoded: Option<BookV2PdfConvergenceObservation>,
    observed: Option<BookV2PdfConvergenceObservation>,
    scope: &str,
    maximum: u64,
    constructors: [[u64; 4]; 11],
    reservation_sizes: [usize; 2],
    caller_before: Option<[u64; 4]>,
) -> Value {
    fs::create_dir_all(directory).unwrap();
    let scopes = content.source();
    let display = scopes.display();
    let text = content.text();
    let images = scopes.source();
    let paint_scope = if scope == "marked_driver" {
        "paint_driver"
    } else {
        scope
    };
    let constructors8 = constructors[..8].try_into().unwrap();
    let paint_probe = paint_probes::snapshot(
        &directory,
        case,
        text,
        images,
        caps,
        encoded,
        paint_scope,
        maximum,
        constructors8,
        reservation_sizes,
        caller_before,
    );
    let flow = display
        .source()
        .original_graph()
        .lines()
        .prepared()
        .source_flow();
    let package = typaxis_document_package::book_v2::BookV2DocumentPackageEncoder::new()
        .encode(flow.body().body().wire())
        .unwrap();
    let wire: Value = serde_json::from_str(&package).unwrap();
    let credit = text.byte_length()
        + images.bytes().len()
        + (0..text.commands().len())
            .map(|i| text.actual_text(i).map_or(0, |b| b.len()))
            .sum::<usize>()
        + (0..scopes.groups().len())
            .map(|i| scopes.begin_bytes(i).unwrap().len() + scopes.end_bytes(i).unwrap().len())
            .sum::<usize>();
    let content_length = content.byte_length() + content.anchor_object_bytes().len();
    let value = json!({
        "wire":"typaxis.book-2-column-marked-probe/1","case":case,"paint_probe":paint_probe,
        "source_package":{"text_buffers":flow.body().body().wire().text_buffers().iter().map(|b|json!({"id":b.text_id,"bytes":artifact(&directory,b.utf8.as_bytes(),"source-text")})).collect::<Vec<_>>(),"canonical_jcs":package,"canonical_sha256":probes::hex(&typaxis_core::sha256(package.as_bytes())),"source_fingerprint":probes::hex(&flow.body().body().canonical_jcs_sha256()),"flow_fingerprint":probes::hex(&flow.fingerprint()),"wire":wire,"max_ast_depth":flow.body().body().limits().get().max_ast_nesting_depth,"max_ast_nodes":flow.body().body().limits().get().max_ast_nodes},
        "navigation":{"flow_fingerprint":probes::hex(&flow.fingerprint()),"ast_node_count":flow.navigation().ast_node_count(),"languages":flow.navigation().languages().iter().map(|l|json!({"owner":l.node_id().get(),"parent":l.parent().map(|p|p.get()),"kind":format!("{:?}",l.kind()),"span":l.source_span().map(|s|[s.source_id().get(),s.start_byte().get(),s.end_byte().get()]),"language":l.effective_language(),"page_region":l.page_region().map(|p|p.get()),"number":l.vector().and_then(|v|v.equation_number()).map(|n|json!({"owner":n.node_id().get(),"span":[n.span().source_id().get(),n.span().start_byte().get(),n.span().end_byte().get()]}))})).collect::<Vec<_>>(),"language_children":flow.navigation().language_children().iter().map(|n|json!({"owner":n.node_id().get(),"parent":n.parent().get(),"language":n.effective_language(),"span":[n.source_span().source_id().get(),n.source_span().start_byte().get(),n.source_span().end_byte().get()]})).collect::<Vec<_>>()},
        "marked_scopes":{"fingerprint":probes::hex(&scopes.fingerprint()),"charges":[scopes.record_charge(),scopes.spool_charge(),scopes.output_charge(),scopes.work_steps()],"groups":scopes.groups().iter().enumerate().map(|(i,g)|json!({"owner":g.owner().map(|o|o.get()),"page":g.page_index(),"fragment":g.fragment_index(),"role":format!("{:?}",g.role()),"artifact":g.artifact().map(|a|format!("{a:?}")),"mcid":g.mcid(),"paints":[g.paints().start,g.paints().end],"begin":artifact(&directory,scopes.begin_bytes(i).unwrap(),"scope-begin"),"end":artifact(&directory,scopes.end_bytes(i).unwrap(),"scope-end"),"alternative":scopes.alternative(i),"actual_text":scopes.actual_text(i)})).collect::<Vec<_>>()},
        "marked_content":{"fingerprint":probes::hex(&content.fingerprint()),"charges":[content.record_charge(),content.spool_charge(),content.output_charge(),content.work_steps()],"pages":content.pages().iter().enumerate().map(|(i,p)|json!({"page":p.page_index(),"groups":[p.groups().start,p.groups().end],"bytes":artifact(&directory,content.page_bytes(i).unwrap(),"marked-page")})).collect::<Vec<_>>(),"groups":(0..scopes.groups().len()).map(|i|artifact(&directory,content.group_bytes(i).unwrap(),"marked-group")).collect::<Vec<_>>(),"body_length":content.byte_length(),"next_object":content.next_object(),"anchor_font":content.anchor_font_object().map(|o|o.get()),"anchor_bytes":artifact(&directory,content.anchor_object_bytes(),"anchor-objects"),"anchor_objects":content.anchor_objects().iter().map(|o|json!({"id":o.id().get(),"role":format!("{:?}",o.role()),"range":[o.byte_range().start,o.byte_range().end]})).collect::<Vec<_>>(),"anchors":content.anchors().iter().map(|a|json!({"page":a.page_index(),"group":a.group_index(),"paint":a.paint_index(),"viewport":rect(a.viewport()),"baseline":a.baseline().raw()})).collect::<Vec<_>>()},
        "source_structure":{"fingerprint":probes::hex(&structure.fingerprint()),"charges":[structure.record_charge(),structure.spool_charge(),structure.output_charge(),structure.work_steps()],"nodes":structure.nodes().iter().map(|n|{let s=n.source();json!({"key":[s.key().owner().get(),format!("{:?}",s.key().slot())],"parent":n.parent(),"depth":n.depth(),"first_child":n.first_child(),"next_sibling":n.next_sibling(),"first_binding":n.first_binding(),"role":s.pdf_role(),"semantic_kind":s.semantic_kind(),"language":s.language(),"alternative":s.alternative(),"span":s.source_span().map(|s|[s.source_id().get(),s.start_byte().get(),s.end_byte().get()]),"cell":s.table_cell().map(|c|json!({"table":c.table.get(),"row":c.row,"column":c.column,"colspan":c.colspan,"rowspan":c.rowspan,"header":c.header}))})}).collect::<Vec<_>>(),"bindings":structure.bindings().iter().map(|b|json!({"node":b.node_index(),"group":b.group_index(),"page":b.page_index(),"mcid":b.mcid(),"next":b.next_for_node()})).collect::<Vec<_>>(),"group_bindings":(0..scopes.groups().len()).map(|i|structure.for_group(i).map(|b|[b.node_index(),b.group_index(),b.page_index() as usize,b.mcid() as usize])).collect::<Vec<_>>()},
        "settlement":{"initial_credit":credit,"raw_before_output":scopes.output_charge(),"materialized_length":content_length,"reserved_output":content.output_charge(),"net_output":content.output_charge() as i128-scopes.output_charge() as i128},
        "observed":{"scope":scope,"maximum_work":maximum,"constructors":constructors,"caller_before":caller_before,"encoded_before":if scope=="marked_driver"{encoded.map(paint_observation)}else{None},"caller_after":if scope=="marked_driver"{observed.map(paint_observation)}else{None},"ancestor_callback":observed.map(paint_observation)},
        "size_of":{"scope":std::mem::size_of::<typaxis_pdf::book_v2::BookV2MarkedScope>(),"page":std::mem::size_of::<typaxis_pdf::book_v2::BookV2MarkedPage>(),"anchor":std::mem::size_of::<typaxis_pdf::book_v2::BookV2SemanticAnchor>(),"anchor_object":std::mem::size_of::<typaxis_pdf::book_v2::BookV2SemanticAnchorObject>(),"source_node":std::mem::size_of::<typaxis_pdf::book_v2::BookV2SourceStructureNode<'_>>(),"lookup":std::mem::size_of::<(Key,usize)>(),"binding":std::mem::size_of::<typaxis_pdf::book_v2::BookV2StructureContentBinding>(),"option_usize":std::mem::size_of::<Option<usize>>(),"range":std::mem::size_of::<std::ops::Range<usize>>()},
    });
    value
}
