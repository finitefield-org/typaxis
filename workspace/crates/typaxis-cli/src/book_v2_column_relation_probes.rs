use super::*;
use crate::book_v2_resources::BookV2ColumnRelationBundle;
pub(super) fn emit(
    case: &str,
    bundle: &BookV2ColumnRelationBundle<
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
    maximum: u64,
) {
    let constructors = bundle.constructor_charges();
    emit_inner(
        case,
        bundle.relations(),
        caps,
        Some(bundle.encoded_prefix()),
        Some(bundle.marked_prefix()),
        Some(observed),
        "relation_driver",
        maximum,
        constructors,
        bundle.marked().paints().reservation_sizes(),
        Some(bundle.caller_before()),
    );
}
pub(super) fn emit_inner(
    case: &str,
    tree: &BookV2ColumnStructureRelations<
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
    encoded: Option<BookV2PdfConvergenceObservation>,
    marked_observed: Option<BookV2PdfConvergenceObservation>,
    observed: Option<BookV2PdfConvergenceObservation>,
    scope: &str,
    maximum: u64,
    constructors: [[u64; 4]; 12],
    units: [usize; 2],
    caller_before: Option<[u64; 4]>,
) {
    let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory).join("column-structure-relations");
    fs::create_dir_all(&directory).unwrap();
    let value = snapshot(
        &directory,
        case,
        tree,
        caps,
        encoded,
        marked_observed,
        observed,
        scope,
        maximum,
        constructors,
        units,
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
    tree: &BookV2ColumnStructureRelations<
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
    encoded: Option<BookV2PdfConvergenceObservation>,
    marked_observed: Option<BookV2PdfConvergenceObservation>,
    observed: Option<BookV2PdfConvergenceObservation>,
    scope: &str,
    maximum: u64,
    constructors: [[u64; 4]; 12],
    units: [usize; 2],
    caller_before: Option<[u64; 4]>,
) -> Value {
    let source = tree.source();
    let display = source.source().source().display();
    let view = display.relation_source();
    let marked_scope = if scope == "relation_driver" {
        "marked_driver"
    } else {
        scope
    };
    let marked_probe = marked_probes::snapshot(
        &directory,
        case,
        source.source(),
        source,
        caps,
        encoded,
        marked_observed,
        marked_scope,
        maximum,
        constructors[..11].try_into().unwrap(),
        units,
        caller_before,
    );
    let nd = view.definition_count();
    let selected = display.source().source().stable().sequence();
    let selected_pages=selected.pages().iter().map(|p|{let state=p.next_state().source_state();json!({"page":p.page_index(),"name":p.named_page(),"forced":p.forced_break().map(|n|n.get()),"balanced":p.balance().map(|b|probes::hex(&b.fingerprint())),"next_item":state.next_item(),"next_table":state.next_table_index(),"is_complete":state.is_complete(),"pending":state.pending_definitions(),"statuses":(0..nd).map(|i|format!("{:?}",state.definition_status(i).unwrap())).collect::<Vec<_>>(),"first_references":(0..nd).map(|i|state.first_reference(i).map(|n|n.get())).collect::<Vec<_>>()})}).collect::<Vec<_>>();
    let value = json!({"wire":"typaxis.book-2-column-structure-relations-probe/1","case":case,"marked_probe":marked_probe,
    "inputs":{"tables":view.tables().iter().map(|t|json!({"owner":t.owner().get(),"rows":t.rows().iter().map(|r|json!({"owner":r.owner().get(),"section":format!("{:?}",r.section()),"cells":[r.cells().start,r.cells().end]})).collect::<Vec<_>>(),"cells":t.cells().iter().map(|c|json!({"owner":c.owner().get(),"row":c.row(),"column":c.column(),"colspan":c.colspan().get(),"rowspan":c.rowspan().get()})).collect::<Vec<_>>()})).collect::<Vec<_>>(),"lists":view.lists().iter().map(|l|json!({"owner":l.owner().get(),"ordered":l.ordered()})).collect::<Vec<_>>(),"definitions":(0..nd).map(|i|json!({"index":i,"owner":view.definition_owner(i).unwrap().get()})).collect::<Vec<_>>(),"references":(0..view.reference_count()).map(|i|{let r=view.reference(i).unwrap();json!({"index":i,"owner":r.owner().get(),"definition":r.definition_index(),"source_definition":r.source_definition()})}).collect::<Vec<_>>(),"has_selected_demand":view.has_selected_demand(),"first_demand":(0..nd).map(|i|view.first_reference(i).map(|n|n.get())).collect::<Vec<_>>(),"unreferenced":view.unreferenced_definitions(),"selected_pages":selected_pages,"measurements_fingerprint":probes::hex(&selected.measurements_fingerprint()),"math_terminal_fingerprint":probes::hex(&display.source().fingerprint())},
    "relations":{"fingerprint":probes::hex(&tree.fingerprint()),"charges":[tree.record_charge(),tree.spool_charge(),tree.output_charge(),tree.work_steps()],"nodes":tree.nodes().iter().enumerate().map(|(i,n)|json!({"parent":n.parent(),"first_child":n.first_child(),"next_sibling":n.next_sibling(),"depth":n.depth(),"headers":tree.headers(i).unwrap(),"related":tree.related_nodes(i).unwrap(),"list_numbering":n.list_numbering().map(|v|format!("{v:?}"))})).collect::<Vec<_>>(),"reading_order":tree.reading_order(),"notes":tree.notes().iter().map(|n|json!({"node":n.node_index(),"link":n.link_node(),"label":n.label_node(),"painted":n.painted(),"first":n.first_reference(),"last":tree.edges().iter().enumerate().filter(|(_,e)|e.note_node()==n.node_index()).map(|(i,_)|i).next_back(),"return":n.return_reference(),"reading":n.reading_reference(),"after":n.insertion_after()})).collect::<Vec<_>>(),"edges":tree.edges().iter().map(|e|json!({"reference":e.reference_node(),"note":e.note_node(),"definition":e.definition_index(),"source_definition":e.source_definition(),"ref_link":e.reference_link(),"ref_label":e.reference_label(),"def_link":e.definition_link(),"def_label":e.definition_label(),"painted":e.painted(),"next":e.next_for_note()})).collect::<Vec<_>>()},
    "observed":{"scope":scope,"maximum_work":maximum,"constructors":constructors,"caller_before":caller_before,"caller_after":if scope=="relation_driver"{observed.map(paint_observation)}else{None},"marked_prefix":if scope=="relation_driver"{marked_observed.map(paint_observation)}else{None},"ancestor_callback":marked_observed.map(paint_observation),"source_prefix":[source.record_charge(),source.spool_charge(),source.output_charge(),source.work_steps()]},
    "size_of":{"related_node":std::mem::size_of::<typaxis_pdf::book_v2::BookV2RelatedStructureNode>(),"note":std::mem::size_of::<typaxis_pdf::book_v2::BookV2NoteStructure>(),"edge":std::mem::size_of::<typaxis_pdf::book_v2::BookV2NoteStructureEdge>(),"usize":std::mem::size_of::<usize>()},
    });
    value
}
