use super::*;

pub(super) struct Context<'c> {
    pub scope: &'c str,
    pub parent_scope: &'c str,
    pub maximum: u64,
    pub constructors: [[u64; 4]; 13],
    pub font_units: [usize; 2],
    pub first_bounds_size: usize,
    pub encoded: Option<BookV2PdfConvergenceObservation>,
    pub marked: Option<BookV2PdfConvergenceObservation>,
    pub relations: Option<BookV2PdfConvergenceObservation>,
    pub ancestor: Option<BookV2PdfConvergenceObservation>,
    pub parent_caller_before: Option<[u64; 4]>,
    pub caller_before: Option<[u64; 4]>,
    pub observed: Option<BookV2PdfConvergenceObservation>,
    pub build_after: [u64; 4],
}

pub(super) fn emit(
    case: &str,
    nav: Option<
        &BookV2ColumnNavigationGeometry<
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
    >,
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
    error: Option<&NavigationError>,
    caps: &M4EffectiveResourceLimits,
    context: Context<'_>,
) {
    let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory).join("column-navigation");
    fs::create_dir_all(&directory).unwrap();
    let registry = tree.source();
    let marked = registry.source();
    let display = marked.source().display();
    let view = display.navigation_source();
    let relation_probe = relation_probes::snapshot(
        &directory,
        case,
        tree,
        caps,
        context.encoded,
        context.marked,
        context.relations,
        context.parent_scope,
        context.maximum,
        context.constructors[..12].try_into().unwrap(),
        context.font_units,
        context.parent_caller_before,
    );
    let paragraphs=view.paragraphs().iter().map(|p| json!({
        "owner":p.owner().get(),"items":p.items().iter().map(|i| {
            let span=i.source_span();
            let link=i.link_target().map(|l|match l{LinkTarget::Internal{anchor_id}=>json!({"kind":"internal","value":anchor_id}),LinkTarget::Uri{uri}=>json!({"kind":"uri","value":uri})});
            let reference=i.reference().map(|r|match r{Reference::Anchor{target,target_owner,format}=>json!({"kind":"anchor","target":target,"target_owner":target_owner.get(),"format":format!("{format:?}").to_ascii_lowercase()}),Reference::Footnote{footnote_id}=>json!({"kind":"footnote","id":footnote_id})});
            json!({"owner":i.owner().get(),"kind":i.content().as_str(),"span":[span.source_id().get(),span.start_byte().get(),span.end_byte().get()],"link":link,"reference":reference})
        }).collect::<Vec<_>>()
    })).collect::<Vec<_>>();
    let raw_groups=marked.source().groups().iter().enumerate().map(|(i,g)|json!({
        "group":i,"page":g.page_index(),"fragment":g.fragment_index(),"column":display.fragment_column(g.fragment_index()),"artifact":g.artifact().is_some(),
        "paints":g.paints().map(|pi|match display.paints()[pi]{Paint::FootnoteSeparator(_)=>None,_=>component_bounds(display,pi).map(raw_rect)}).collect::<Vec<_>>()
    })).collect::<Vec<_>>();
    let inline_anchors=display.anchors().positions().iter().map(|a|json!({
        "owner":a.anchor().source().owner().get(),"page":a.fragment().fragment().page_index(),"fragment":a.fragment_index(),"column":display.fragment_column(a.fragment_index()),"x":a.x().raw(),"baseline":a.baseline().raw(),"repeated":a.repeated_header()
    })).collect::<Vec<_>>();
    let nonpainting_lines=display.anchors().nonpainting_lines().iter().map(|a|{
        let f=a.fragment().fragment();
        let typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine { paragraph_index, line_index } = f.source() else { panic!("nonparagraph empty source"); };
        let graph=display.source().fragment_lines(a.fragment_index()).unwrap();
        let line=&graph.paragraphs()[paragraph_index as usize].lines()[line_index as usize];
        let breaks=line.items().iter().map(|i|match i{typaxis_layout::ProductionPlacedInline::Break(b)=>b.owner().get(),_=>panic!("paint in original nonpainting line")}).collect::<Vec<_>>();
        json!({"owner":f.owner().get(),"page":f.page_index(),"fragment":a.fragment_index(),"column":display.fragment_column(a.fragment_index()),"bounds":raw_rect(f.bounds()),"repeated":a.repeated_header(),"breakowners":breaks})
    }).collect::<Vec<_>>();
    let navigation=nav.map(|n|{
        assert!(std::ptr::eq(n.source(),tree));
        let mut cursor=0;
        let pages=(0..marked.pages().len()).map(|p|{let start=cursor;cursor+=n.page_rectangles(p).unwrap().len();[start,cursor]}).collect::<Vec<_>>();
        json!({"fingerprint":probes::hex(&n.fingerprint()),"charges":[n.record_charge(),n.spool_charge(),n.output_charge(),n.work_steps()],
            "destinations":n.destinations().iter().map(|d|json!({"owner":d.owner().get(),"kind":match d.kind(){DestinationKind::Anchor(s)=>json!(["anchor",s]),DestinationKind::Footnote(i)=>json!(["footnote",i]),DestinationKind::Reference(i)=>json!(["reference",i])},"position":d.position().map(|p|[i64::from(p.page_index()),p.fragment_index() as i64,p.x().raw(),p.y().raw()])})).collect::<Vec<_>>(),
            "links":n.links().iter().map(|l|json!({"node":l.structure_node(),"target":match l.target(){Target::Destination(i)=>json!(["destination",i]),Target::Uri(s)=>json!(["uri",s])},"first":l.first_rectangle()})).collect::<Vec<_>>(),
            "rectangles":n.rectangles().iter().map(|r|json!({"link":r.link_index(),"page":r.page_index(),"fragment":r.fragment_index(),"group":r.first_group(),"bounds":raw_rect(r.bounds()),"next":r.next_for_link()})).collect::<Vec<_>>(),
            "page_ranges":pages,"outline":n.outline().iter().map(|o|json!({"destination":o.destination(),"parent":o.parent(),"first":o.first_child(),"last":o.last_child(),"previous":o.previous(),"next":o.next(),"descendants":o.descendant_count()})).collect::<Vec<_>>()})
    });
    let error=error.map(|e|match e{
        NavigationError::ConflictingLinks(owner)=>json!({"kind":"conflicting_links","owner":owner.get(),"resource":null,"charges":context.build_after}),
        NavigationError::UnplacedDestination(owner)=>json!({"kind":"unplaced_destination","owner":owner.get(),"resource":null,"charges":context.build_after}),
        NavigationError::Resource(e)=>json!({"kind":"resource","owner":null,"resource":format!("{e:?}"),"charges":context.build_after}),
    });
    let value = json!({"wire":"typaxis.book-2-column-navigation-probe/1","case":case,"relation_probe":relation_probe,
        "navigation_source":{"paragraphs":paragraphs,"anchors":view.anchors().iter().map(|(id,owner)|json!({"id":id.as_str(),"owner":owner.get()})).collect::<Vec<_>>(),"outline":view.outline().iter().map(|e|json!({"id":e.outline_id,"parent":e.parent_outline_id,"level":e.level,"target":e.destination.as_str(),"label":e.label})).collect::<Vec<_>>()},
        "raw_groups":raw_groups,"inline_anchors":inline_anchors,"nonpainting_lines":nonpainting_lines,"navigation":navigation,"error":error,
        "observed":{"scope":context.scope,"parent_scope":context.parent_scope,"maximum_work":context.maximum,"constructors":context.constructors,"caller_before":context.caller_before,"caller_after":if context.scope=="navigation_driver"{context.observed.map(paint_observation)}else{None},"relations_prefix":if context.scope=="navigation_driver"{context.relations.map(paint_observation)}else{None},"ancestor_callback":context.ancestor.map(paint_observation),"source_prefix":[tree.record_charge(),tree.spool_charge(),tree.output_charge(),tree.work_steps()],"build_after":context.build_after},
        "size_of":{"destination":std::mem::size_of::<typaxis_pdf::book_v2::BookV2NavigationDestination>(),"link":std::mem::size_of::<typaxis_pdf::book_v2::BookV2NavigationLink>(),"rectangle":std::mem::size_of::<typaxis_pdf::book_v2::BookV2NavigationRectangle>(),"outline":std::mem::size_of::<typaxis_pdf::book_v2::BookV2NavigationOutline>(),"range":std::mem::size_of::<std::ops::Range<usize>>(),"option_usize":std::mem::size_of::<Option<usize>>(),"option_first_bounds":context.first_bounds_size,"usize":std::mem::size_of::<usize>()}});
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
