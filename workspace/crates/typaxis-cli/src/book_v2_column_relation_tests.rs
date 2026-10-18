use super::*;
use crate::book_v2_resources::with_budgeted_book_v2_column_relations as relation_drive;
use typaxis_display_list::book_v2::BookV2RelationDisplay;
use typaxis_pdf::book_v2::{
    BookV2ColumnStructureRelationBuilder as Relations, BookV2ColumnStructureRelations,
    BookV2FontStreamError as RelationError, BookV2ListNumbering,
};
use typaxis_syntax::book_v2::BookV2StructureSlot as Slot;
#[path = "book_v2_column_relation_callback_ownership_tests.rs"]
mod callback_ownership_tests;
#[path = "book_v2_column_navigation_tests.rs"]
mod navigation_tests;
#[path = "book_v2_column_relation_probes.rs"]
mod relation_probes;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedEdge {
    reference: usize,
    note: usize,
    definition: usize,
    origin: Option<usize>,
    reference_link: usize,
    reference_label: usize,
    definition_link: usize,
    definition_label: usize,
    painted: bool,
    next: Option<usize>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedNote {
    node: usize,
    link: usize,
    label: usize,
    painted: bool,
    edges: Vec<usize>,
    return_edge: Option<usize>,
    reading: Option<usize>,
    after: Option<usize>,
}
#[derive(Debug)]
struct RelationFacts {
    headers: usize,
    notes: usize,
    edges: usize,
    unpainted: usize,
    relocated: usize,
    back_edges: usize,
    selected: Vec<Option<u32>>,
    reading: Vec<Option<u32>>,
    returning: Vec<Option<u32>>,
}
fn package_wire(display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>) -> Value {
    let flow = display
        .source()
        .original_graph()
        .lines()
        .prepared()
        .source_flow();
    serde_json::from_str(
        &typaxis_document_package::book_v2::BookV2DocumentPackageEncoder::new()
            .encode(flow.body().body().wire())
            .unwrap(),
    )
    .unwrap()
}
fn authored_reference_facts(document: &Value) -> BTreeMap<u32, (usize, Option<usize>)> {
    let ids = document["footnotes"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, n)| (n["footnote_id"].as_str().unwrap().to_owned(), i))
        .collect::<BTreeMap<_, _>>();
    fn walk(
        v: &Value,
        origin: Option<usize>,
        ids: &BTreeMap<String, usize>,
        out: &mut BTreeMap<u32, (usize, Option<usize>)>,
    ) {
        match v {
            Value::Array(a) => {
                for v in a {
                    walk(v, origin, ids, out)
                }
            }
            Value::Object(o) => {
                if v["kind"] == "footnote_reference" {
                    assert!(out
                        .insert(
                            v["node_id"].as_u64().unwrap() as u32,
                            (ids[v["footnote_id"].as_str().unwrap()], origin)
                        )
                        .is_none());
                }
                for (k, v) in o {
                    if k != "footnotes" && matches!(v, Value::Array(_) | Value::Object(_)) {
                        walk(v, origin, ids, out)
                    }
                }
            }
            _ => {}
        }
    }
    let mut out = BTreeMap::new();
    walk(&document["blocks"], None, &ids, &mut out);
    for (i, n) in document["footnotes"].as_array().unwrap().iter().enumerate() {
        walk(n, Some(i), &ids, &mut out)
    }
    out
}
fn move_note(
    parents: &mut [Option<usize>],
    children: &mut [Vec<usize>],
    target: usize,
    parent: usize,
    after: usize,
) {
    let old = parents[target].unwrap();
    let i = children[old].iter().position(|&n| n == target).unwrap();
    children[old].remove(i);
    let i = children[parent].iter().position(|&n| n == after).unwrap();
    children[parent].insert(i + 1, target);
    parents[target] = Some(parent);
}
fn original_anchor(
    nodes: &[SourceNode],
    parents: &[Option<usize>],
    reference: usize,
) -> (usize, usize) {
    let mut branch = reference;
    while let Some(p) = parents[branch] {
        let n = &nodes[p];
        if matches!(
            n.role.as_str(),
            "P" | "H1" | "H2" | "H3" | "H4" | "H5" | "H6"
        ) || (n.role == "Lbl" && n.key.1 == "Source")
        {
            return (p, branch);
        }
        branch = p;
    }
    panic!("authored reference has no inline-bearing source ancestor")
}
fn has_ancestor(parents: &[Option<usize>], mut at: Option<usize>, target: usize) -> bool {
    let mut seen = BTreeSet::new();
    while let Some(i) = at {
        assert!(seen.insert(i), "cycle in independent selected forest");
        if i == target {
            return true;
        }
        at = parents[i];
    }
    false
}
fn inspect_relations(
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
) -> RelationFacts {
    let source = tree.source();
    let content = source.source();
    let display = content.source().display();
    inspect_marked(content, source);
    let view = display.relation_source();
    let wire = package_wire(display);
    let document = &wire["document"];
    let originals = original_source_nodes(document);
    assert_eq!(tree.nodes().len(), originals.len());
    let lookup = originals
        .iter()
        .enumerate()
        .map(|(i, n)| (n.key.clone(), i))
        .collect::<BTreeMap<_, _>>();
    let locate = |owner: u32, slot: &str| lookup[&(owner, slot.to_owned())];
    let original_parents = originals
        .iter()
        .map(|n| n.parent.as_ref().map(|k| lookup[k]))
        .collect::<Vec<_>>();
    let mut parents = original_parents.clone();
    let mut children = vec![Vec::new(); originals.len()];
    for (i, p) in parents.iter().enumerate() {
        if let Some(p) = p {
            children[*p].push(i)
        }
    }
    let definitions = document["footnotes"].as_array().unwrap();
    assert_eq!(view.definition_count(), definitions.len());
    let mut notes = Vec::new();
    for (di, d) in definitions.iter().enumerate() {
        let owner = d["node_id"].as_u64().unwrap() as u32;
        assert_eq!(view.definition_owner(di).unwrap().get(), owner);
        let node = locate(owner, "Source");
        let link = locate(owner, "FootnoteLink");
        let label = locate(owner, "FootnoteLabel");
        assert_eq!(originals[node].role, "Note");
        notes.push(ExpectedNote {
            node,
            link,
            label,
            painted: source.nodes()[label].first_binding().is_some(),
            edges: Vec::new(),
            return_edge: None,
            reading: None,
            after: None,
        });
    }
    let references = authored_reference_facts(document);
    let reference_order = originals
        .iter()
        .filter(|n| n.key.1 == "Source" && references.contains_key(&n.key.0))
        .map(|n| n.key.0)
        .collect::<Vec<_>>();
    assert_eq!(view.reference_count(), reference_order.len());
    let mut edges: Vec<ExpectedEdge> = Vec::new();
    let mut related = vec![Vec::new(); originals.len()];
    for (ei, &owner) in reference_order.iter().enumerate() {
        let (definition, origin) = references[&owner];
        let r = view.reference(ei).unwrap();
        assert_eq!(
            (r.owner().get(), r.definition_index(), r.source_definition()),
            (owner, definition, origin)
        );
        let reference = locate(owner, "Source");
        let reference_link = locate(owner, "FootnoteLink");
        let reference_label = locate(owner, "FootnoteLabel");
        let note = &mut notes[definition];
        let mut ancestor = original_parents[reference];
        let mut actual_origin = None;
        while let Some(a) = ancestor {
            if let Some(di) = definitions.iter().position(|d| {
                d["node_id"].as_u64() == Some(u64::from(originals[a].key.0))
                    && originals[a].key.1 == "Source"
            }) {
                actual_origin = Some(di);
                break;
            }
            ancestor = original_parents[a];
        }
        assert_eq!(actual_origin, origin);
        let painted = source.nodes()[reference_label].first_binding().is_some();
        assert!(!painted || note.painted);
        if let Some(&prior) = note.edges.last() {
            edges[prior].next = Some(ei)
        }
        note.edges.push(ei);
        if painted && note.return_edge.is_none() {
            note.return_edge = Some(ei)
        }
        related[reference].push(note.node);
        related[note.node].push(reference);
        edges.push(ExpectedEdge {
            reference,
            note: note.node,
            definition,
            origin,
            reference_link,
            reference_label,
            definition_link: note.link,
            definition_label: note.label,
            painted,
            next: None,
        });
    }
    assert!(view.reference(usize::MAX).is_none());
    assert!(view.definition_owner(usize::MAX).is_none());
    let pages = display.source().source().stable().sequence().pages();
    assert_eq!(view.has_selected_demand(), !pages.is_empty());
    assert!(view.has_selected_demand());
    let demand = pages.last().unwrap().next_state().source_state();
    let selected = (0..notes.len())
        .map(|di| view.first_reference(di).map(|n| n.get()))
        .collect::<Vec<_>>();
    for (di, note) in notes.iter_mut().enumerate() {
        assert_eq!(selected[di], demand.first_reference(di).map(|n| n.get()));
        assert_eq!(selected[di].is_some(), note.painted);
        if let Some(owner) = selected[di] {
            let choices = note
                .edges
                .iter()
                .copied()
                .filter(|&ei| originals[edges[ei].reference].key.0 == owner && edges[ei].painted)
                .collect::<Vec<_>>();
            assert_eq!(choices.len(), 1);
            let ei = choices[0];
            let (parent, branch) =
                original_anchor(&originals, &original_parents, edges[ei].reference);
            note.reading = Some(ei);
            note.after = Some(branch);
            move_note(&mut parents, &mut children, note.node, parent, branch);
        }
    }
    for note in &notes {
        if note.painted {
            assert!(!has_ancestor(&parents, parents[note.node], note.node));
        }
    }
    // This oracle uses independent child vectors and ancestry sets. It does not use
    // the production detach/insert/header_pairs helpers or display paint ordering.
    for di in 0..notes.len() {
        if !notes[di].painted {
            continue;
        }
        let target = notes[di].node;
        let mut selected = None;
        for &ei in &notes[di].edges {
            if !edges[ei].painted {
                continue;
            }
            let (parent, branch) =
                original_anchor(&originals, &original_parents, edges[ei].reference);
            if !has_ancestor(&parents, Some(parent), target) {
                selected = Some((ei, parent, branch))
            }
        }
        let (ei, parent, branch) = selected.unwrap();
        notes[di].reading = Some(ei);
        notes[di].after = Some(branch);
        let key = (
            originals[edges[ei].reference].key.0,
            originals[target].key.0,
        );
        let mut after = branch;
        let mut index = children[parent].iter().position(|&i| i == branch).unwrap() + 1;
        while let Some(&next) = children[parent].get(index) {
            if next == target {
                index += 1;
                continue;
            }
            let Some(other) = notes.iter().position(|n| n.node == next) else {
                break;
            };
            if notes[other].after != Some(branch) {
                break;
            }
            let oe = notes[other].reading.unwrap();
            let other_key = (originals[edges[oe].reference].key.0, originals[next].key.0);
            if other_key > key {
                break;
            }
            after = next;
            index += 1;
        }
        move_note(&mut parents, &mut children, target, parent, after);
    }
    let mut expected_order = Vec::new();
    let mut stack = vec![(0usize, 1u32)];
    let mut depths = vec![0; originals.len()];
    while let Some((i, depth)) = stack.pop() {
        assert_eq!(depths[i], 0);
        depths[i] = depth;
        expected_order.push(i);
        stack.extend(children[i].iter().rev().map(|&c| (c, depth + 1)));
    }
    assert_eq!(expected_order.len(), originals.len());
    assert_eq!(tree.reading_order(), expected_order);
    fn list_flags(v: &Value, out: &mut BTreeMap<u32, bool>) {
        match v {
            Value::Array(a) => {
                for v in a {
                    list_flags(v, out)
                }
            }
            Value::Object(o) => {
                if v["kind"] == "list" {
                    assert!(out
                        .insert(
                            v["node_id"].as_u64().unwrap() as u32,
                            v["ordered"].as_bool().unwrap()
                        )
                        .is_none());
                }
                for v in o.values() {
                    list_flags(v, out)
                }
            }
            _ => {}
        }
    }
    let mut ordered_lists = BTreeMap::new();
    list_flags(document, &mut ordered_lists);
    assert_eq!(
        view.lists()
            .iter()
            .map(|l| (l.owner().get(), l.ordered()))
            .collect::<BTreeMap<_, _>>(),
        ordered_lists
    );
    let mut header_count = 0;
    for (i, n) in tree.nodes().iter().enumerate() {
        assert_eq!((n.parent(), n.depth()), (parents[i], depths[i]));
        assert_eq!(n.first_child(), children[i].first().copied());
        let sibling = parents[i].and_then(|p| {
            children[p]
                .iter()
                .position(|&n| n == i)
                .and_then(|j| children[p].get(j + 1).copied())
        });
        assert_eq!(n.next_sibling(), sibling);
        let expected_headers = originals[i]
            .cell
            .filter(|c| c[5] == 0)
            .map(|c| {
                originals
                    .iter()
                    .enumerate()
                    .filter(|(_, h)| {
                        h.cell.is_some_and(|h| {
                            h[0] == c[0] && h[5] == 1 && c[2] < h[2] + h[3] && h[2] < c[2] + c[3]
                        })
                    })
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        header_count += expected_headers.len();
        assert_eq!(tree.headers(i).unwrap(), expected_headers);
        assert_eq!(tree.related_nodes(i).unwrap(), related[i]);
        let numbering = if originals[i].key.1 == "Source" {
            ordered_lists.get(&originals[i].key.0).map(|o| {
                if *o {
                    BookV2ListNumbering::Decimal
                } else {
                    BookV2ListNumbering::Disc
                }
            })
        } else {
            None
        };
        assert_eq!(n.list_numbering(), numbering);
    }
    for table in view.tables() {
        for row in table.rows() {
            for cell in &table.cells()[row.cells()] {
                let c = originals[locate(cell.owner().get(), "Source")]
                    .cell
                    .unwrap();
                assert_eq!(
                    [c[0], c[1], c[2], c[3], c[4]],
                    [
                        table.owner().get(),
                        cell.row(),
                        cell.column(),
                        u32::from(cell.colspan().get()),
                        u32::from(cell.rowspan().get())
                    ]
                );
                assert_eq!(
                    c[5] != 0,
                    row.section() == typaxis_syntax::ProductionTableSection::Head
                );
            }
        }
    }
    assert_eq!(tree.edges().len(), edges.len());
    for (a, e) in tree.edges().iter().zip(&edges) {
        assert_eq!(
            (
                a.reference_node(),
                a.note_node(),
                a.definition_index(),
                a.source_definition(),
                a.reference_link(),
                a.reference_label(),
                a.definition_link(),
                a.definition_label(),
                a.painted(),
                a.next_for_note()
            ),
            (
                e.reference,
                e.note,
                e.definition,
                e.origin,
                e.reference_link,
                e.reference_label,
                e.definition_link,
                e.definition_label,
                e.painted,
                e.next
            )
        );
    }
    assert_eq!(tree.notes().len(), notes.len());
    for (a, n) in tree.notes().iter().zip(&notes) {
        assert_eq!(
            (
                a.node_index(),
                a.link_node(),
                a.label_node(),
                a.painted(),
                a.first_reference(),
                a.return_reference(),
                a.reading_reference(),
                a.insertion_after()
            ),
            (
                n.node,
                n.link,
                n.label,
                n.painted,
                n.edges.first().copied(),
                n.return_edge,
                n.reading,
                n.after
            )
        );
    }
    assert_eq!(
        notes.iter().filter(|n| !n.painted).count(),
        view.unreferenced_definitions()
    );
    assert!(tree.headers(usize::MAX).is_none());
    assert!(tree.related_nodes(usize::MAX).is_none());
    assert_eq!(tree.output_charge(), source.output_charge());
    RelationFacts {
        headers: header_count,
        notes: notes.len(),
        edges: edges.len(),
        unpainted: notes.iter().filter(|n| !n.painted).count(),
        relocated: notes.iter().filter(|n| n.reading.is_some()).count(),
        back_edges: edges.iter().filter(|e| e.origin.is_some()).count(),
        selected,
        reading: notes
            .iter()
            .map(|n| n.reading.map(|ei| originals[edges[ei].reference].key.0))
            .collect(),
        returning: notes
            .iter()
            .map(|n| n.return_edge.map(|ei| originals[edges[ei].reference].key.0))
            .collect(),
    }
}
fn input_relations(
    case: &str,
    input: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
) -> RelationFacts {
    let mut budget = BookV2ColumnPageBudget::new(caps, MAXIMUM);
    relation_drive(input, caps, MODE, &mut budget, 4, |bundle, _, observed| {
        let facts = inspect_relations(bundle.relations());
        assert!(std::ptr::eq(
            bundle.relations().source(),
            bundle.marked().structure()
        ));
        assert_eq!(
            observed.output_charge(),
            bundle.marked().content().output_charge()
        );
        relation_probes::emit(case, bundle, caps, observed, MAXIMUM);
        facts
    })
    .unwrap_or_else(|e| panic!("{case}: {e:?}"))
}

fn table_data(mode: &str, text: &str) -> Value {
    super::super::super::super::super::super::super::data(false, mode, text)
}
fn header_data(vector: bool, notes: bool) -> Value {
    super::super::super::super::super::math_terminals::header_data(vector, notes, true)
}
fn dependency_data(cycle: bool) -> Value {
    let mut data = plain_data("Pro", true, false);
    let p = data["document"]["footnotes"][0]["blocks"][0].clone();
    let span = p["span"].clone();
    let reference =
        |id: &str| json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":id});
    let mut body = p.clone();
    body["language"] = "fr".into();
    body["children"].as_array_mut().unwrap().push(if cycle{reference("a")}else{json!({"kind":"strong","node_id":0,"span":span,"children":[reference("a"),reference("b"),reference("a")]})});
    data["document"]["blocks"] = json!([body]);
    let (mut a, mut b, mut unused) = (p.clone(), p.clone(), p);
    if cycle {
        a["children"].as_array_mut().unwrap().push(reference("b"));
        b["children"].as_array_mut().unwrap().push(reference("a"));
    }
    unused["children"]
        .as_array_mut()
        .unwrap()
        .push(reference("a"));
    data["document"]["footnotes"] = json!([{"node_id":0,"span":span,"footnote_id":"b","blocks":[b],"language":"en"},{"node_id":0,"span":span,"footnote_id":"a","blocks":[a],"language":"de"},{"node_id":0,"span":span,"footnote_id":"unused","blocks":[unused],"language":"ja"}]);
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        m["width"] = 12_000_000.into();
        m["height"] = 36_000_000.into();
        m["trim"] = json!({"x":0,"y":0,"width":12_000_000,"height":36_000_000});
        m["body"] = json!({"x":500000,"y":500000,"width":8100000,"height":8000000});
        m["footnote"] = json!({"x":500000,"y":20000000,"width":8100000,"height":10000000});
        m["column_layout"] = json!({"count":2,"gap":100000,"fill":"sequential","balance":"none"});
    }
    semantic_renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_column_relations_original_headers_survive_rowspan_nested_caption_and_forced_pages() {
    let caps = display_caps();
    for (mode, height, balanced) in [
        ("span", 256, false),
        ("nested-body-span", 544, false),
        ("nested-body-caption", 512, true),
        ("forced", 256, false),
    ] {
        let mut data = table_data(mode, "Pro Pro Pro Pro Pro Pro Pro");
        for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
            m["body"]["height"] = (height * 65536).into();
        }
        balance(&mut data, balanced);
        anchors(&mut data);
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let facts = input_relations(&format!("relations-table-{mode}"), &input, &caps);
        assert!(facts.headers > 0);
        assert_eq!((facts.notes, facts.edges), (0, 0));
    }
}
#[test]
fn book_v2_column_relations_actual_repeated_headers_and_definition_tables_use_original_th_nodes_once(
) {
    let caps = display_caps();
    for vector in [false, true] {
        for notes in [false, true] {
            let mut data = header_data(vector, notes);
            balance(&mut data, !notes);
            anchors(&mut data);
            let root = Root::new();
            let input = if vector {
                vector_input(&root, data, &caps)
            } else {
                let text = data["text_buffers"][0]["utf8"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .to_vec();
                native_input_with_source(&root, data, &text, &caps)
            };
            let facts =
                input_relations(&format!("relations-header-{vector}-{notes}"), &input, &caps);
            assert!(facts.headers > 0);
            assert_eq!(facts.notes, usize::from(notes));
            assert_eq!(facts.edges, usize::from(notes));
        }
    }
}
#[test]
fn book_v2_column_relations_selected_first_forest_keeps_backwards_notes_shared_branches_and_cycles()
{
    let caps = display_caps();
    for cycle in [false, true] {
        let root = Root::new();
        let input = display_input(
            &root,
            dependency_data(cycle),
            Some(DISPLAY_LIST_FONT),
            &caps,
        );
        let facts = input_relations(&format!("relations-backwards-cycle-{cycle}"), &input, &caps);
        assert_eq!((facts.notes, facts.unpainted, facts.relocated), (3, 1, 2));
        assert_eq!(facts.edges, 4);
        assert_eq!(facts.back_edges, if cycle { 3 } else { 1 });
        assert!(
            facts.selected[0].is_some()
                && facts.selected[1].is_some()
                && facts.selected[2].is_none()
        );
        if !cycle {
            assert_ne!(facts.reading[1], facts.returning[1]);
        } else {
            assert_eq!(facts.reading[1], facts.selected[1]);
        }
    }
}
#[test]
fn book_v2_column_relations_lists_named_balance_numbers_and_empty_sources_remain_complete() {
    let caps = display_caps();
    for ordered in [false, true] {
        let mut data = plain_data("Pro", true, false);
        wrap_list(&mut data["document"]["blocks"], ordered);
        list_font_rule(&mut data);
        balance(&mut data, true);
        semantic_renumber(&mut data["document"], &mut 0);
        let root = Root::new();
        let input = display_input(&root, data, Some(DISPLAY_LIST_FONT), &caps);
        let facts = input_relations(&format!("relations-list-{ordered}"), &input, &caps);
        assert_eq!((facts.notes, facts.edges, facts.unpainted), (2, 1, 1));
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
            let facts = input_relations(
                &format!("relations-named-{blanks}-{balanced}"),
                &input,
                &caps,
            );
            assert_eq!((facts.notes, facts.edges), (0, 0));
        }
    }
    for balanced in [false, true] {
        let mut data = table_data("common", "Pro");
        for section in ["head", "body"] {
            for row in data["document"]["blocks"][0][section]
                .as_array_mut()
                .unwrap()
            {
                for cell in row["cells"].as_array_mut().unwrap() {
                    cell["blocks"] = json!([]);
                }
            }
        }
        balance(&mut data, balanced);
        semantic_renumber(&mut data["document"], &mut 0);
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let facts = input_relations(
            &format!("relations-unpainted-zero-table-{balanced}"),
            &input,
            &caps,
        );
        assert!(facts.headers > 0);
    }
    let root = Root::new();
    let input = display_input(&root, number_reference_data(), None, &caps);
    let facts = input_relations("relations-generic-number-references", &input, &caps);
    assert_eq!((facts.notes, facts.edges), (0, 0));
}
#[test]
fn book_v2_column_relations_headers_include_unreferenced_nested_definition_tables() {
    let caps = display_caps();
    let mut data = header_data(true, true);
    let mut unused = data["document"]["footnotes"][0].clone();
    unused["footnote_id"] = "unused-relation-table".into();
    fn unique_anchors(v: &mut Value) {
        match v {
            Value::Array(a) => {
                for v in a {
                    unique_anchors(v)
                }
            }
            Value::Object(o) => {
                if let Some(id) = o.get_mut("anchor_id").filter(|v| v.is_string()) {
                    *id = format!("unused-relations-{}", id.as_str().unwrap()).into();
                }
                for v in o.values_mut() {
                    unique_anchors(v)
                }
            }
            _ => {}
        }
    }
    unique_anchors(&mut unused);
    data["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(unused);
    anchors(&mut data);
    let root = Root::new();
    let input = vector_input(&root, data, &caps);
    let facts = input_relations(
        "relations-unreferenced-nested-definition-table",
        &input,
        &caps,
    );
    assert_eq!((facts.notes, facts.edges, facts.unpainted), (2, 1, 1));
    assert!(facts.headers > 24);
}
#[test]
fn book_v2_column_relations_multi_head_intervals_include_only_overlapping_original_cells() {
    let caps = display_caps();
    let mut data = table_data("common", "Pro");
    let table = &mut data["document"]["blocks"][0];
    let paragraph = table["body"][0]["cells"][0]["blocks"][0].clone();
    let span = table["span"].clone();
    let cell = |colspan, rowspan| json!({"node_id":0,"span":span,"colspan":colspan,"rowspan":rowspan,"blocks":[paragraph.clone()]});
    let row = |cells: Vec<Value>| json!({"node_id":0,"span":span,"cells":cells});
    table["columns"] = json!([{"kind":"fraction","weight":1},{"kind":"fraction","weight":1},{"kind":"fraction","weight":1}]);
    table["head"] = json!([
        row(vec![cell(2, 1), cell(1, 2)]),
        row(vec![cell(1, 1), cell(1, 1)])
    ]);
    table["body"] = json!([
        row(vec![cell(1, 1), cell(2, 1)]),
        row(vec![cell(2, 1), cell(1, 1)])
    ]);
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        m["body"]["height"] = (256 * 65536).into();
    }
    anchors(&mut data);
    let root = Root::new();
    let input = display_input(&root, data, None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    relation_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            let facts = inspect_relations(bundle.relations());
            assert!(facts.headers >= 8);
            let counts = bundle
                .relations()
                .source()
                .nodes()
                .iter()
                .enumerate()
                .filter(|(_, n)| n.source().pdf_role() == "TD")
                .map(|(i, _)| bundle.relations().headers(i).unwrap().len())
                .collect::<Vec<_>>();
            assert_eq!(counts, vec![2, 3, 3, 1]);
            relation_probes::emit(
                "relations-multi-head-intervals",
                bundle,
                &caps,
                observed,
                MAXIMUM,
            );
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_relations_note_only_continuations_and_forced_blank_end_keep_actual_final_forest()
{
    let caps = display_caps();
    for forced in [false, true] {
        let mut data = plain_data("Pro", true, false);
        let p = data["document"]["footnotes"][0]["blocks"][0].clone();
        data["document"]["footnotes"][0]["blocks"] = vec![p; 7].into();
        for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
            m["footnote"]["height"] =
                (24 * 65536 + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW).into();
        }
        if forced {
            let span = data["document"]["blocks"][0]["span"].clone();
            data["document"]["blocks"]
                .as_array_mut()
                .unwrap()
                .push(json!({"kind":"page_break","node_id":0,"classes":[],"span":span}));
        }
        semantic_renumber(&mut data["document"], &mut 0);
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        relation_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, _, observed| {
                let facts = inspect_relations(bundle.relations());
                assert_eq!((facts.notes, facts.edges, facts.unpainted), (2, 1, 1));
                let pages = bundle.display().source().source().geometry().pages();
                assert!(pages.len() > 1);
                assert!(pages.iter().skip(1).all(|p| {
                    p.selection()
                        .candidate()
                        .columns()
                        .iter()
                        .all(|c| c.used_height().raw() == 0)
                }));
                assert!(pages
                    .iter()
                    .skip(1)
                    .any(|p| p.fragments().iter().any(|f| f.definition_index().is_some())));
                if forced {
                    assert!(pages.iter().any(|p| p.selection().forced_break().is_some()));
                }
                relation_probes::emit(
                    &format!("relations-note-only-forced-{forced}"),
                    bundle,
                    &caps,
                    observed,
                    MAXIMUM,
                );
            },
        )
        .unwrap();
    }
}
#[test]
fn book_v2_column_relations_reference_origins_in_authored_terms_and_cells_remain_original() {
    let caps = display_caps();
    for term in [false, true] {
        let mut data = if term {
            description_data()
        } else {
            table_data("common", "Pro")
        };
        let plain = plain_data("Pro", true, false);
        let mut note = plain["document"]["footnotes"][0].clone();
        note["footnote_id"] = "relation-term-note".into();
        data["document"]["footnotes"] = json!([note]);
        let span = if term {
            data["document"]["blocks"][0]["items"][0]["term"]["children"][0]["span"].clone()
        } else {
            data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0]["span"].clone()
        };
        let reference = json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"relation-term-note"});
        if term {
            data["document"]["blocks"][0]["items"][0]["term"]["children"][0]["children"]
                .as_array_mut()
                .unwrap()
                .push(reference);
        } else {
            data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0]["children"]
                .as_array_mut()
                .unwrap()
                .push(reference);
        }
        for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
            let body = m["body"].clone();
            let bottom = m["height"].as_i64().unwrap();
            m["footnote"] = json!({"x":body["x"],"y":bottom-3_000_000,"width":body["width"],"height":2_000_000});
        }
        semantic_renumber(&mut data["document"], &mut 0);
        let root = Root::new();
        let input = display_input(&root, data, Some(DISPLAY_LIST_FONT), &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        relation_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, _, observed| {
                let facts = inspect_relations(bundle.relations());
                assert_eq!((facts.notes, facts.edges, facts.relocated), (1, 1, 1));
                let note = &bundle.relations().notes()[0];
                let branch = note.insertion_after().unwrap();
                let parent = bundle.relations().source().nodes()[branch]
                    .parent()
                    .unwrap();
                assert_eq!(
                    bundle.relations().source().nodes()[parent]
                        .source()
                        .pdf_role(),
                    if term { "Lbl" } else { "P" }
                );
                if term {
                    assert_eq!(
                        bundle.relations().source().nodes()[parent]
                            .source()
                            .key()
                            .slot(),
                        Slot::Source
                    );
                }
                relation_probes::emit(
                    &format!("relations-reference-term-{term}"),
                    bundle,
                    &caps,
                    observed,
                    MAXIMUM,
                );
            },
        )
        .unwrap();
    }
}
#[test]
fn book_v2_column_relations_exact_four_caps_constructor_prefix_identity_and_repeat_never_credit_output(
) {
    let caps = display_caps();
    for complex in [false, true] {
        let data = if complex {
            dependency_data(true)
        } else {
            table_data("span", "Pro Pro Pro")
        };
        let root = Root::new();
        let input = display_input(&root, data, Some(DISPLAY_LIST_FONT), &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        relation_drive(&input, &caps, MODE, &mut budget, 4, |bundle, _, _| {
            let source = bundle.relations().source();
            let floor = [
                source.record_charge(),
                source.spool_charge(),
                source.output_charge(),
                source.work_steps(),
            ];
            let run = |prior: [u64; 4], maximum: u64, limits: &M4EffectiveResourceLimits| {
                let (mut r, mut s, mut o, mut w) = (0, 0, 0, 0);
                let made = Relations::new_counted(
                    source, limits, maximum, prior[0], prior[1], prior[2], prior[3], &mut r,
                    &mut s, &mut o, &mut w,
                );
                let constructor = [r, s, o, w];
                let mut b = match made {
                    Ok(b) => b,
                    Err(e) => {
                        return EncodedAttempt {
                            constructor,
                            after: constructor,
                            error: Some(format!("{e:?}")),
                            fingerprint: None,
                        };
                    }
                };
                let result = b.build();
                EncodedAttempt {
                    constructor,
                    after: [
                        b.record_charge(),
                        b.spool_charge(),
                        b.output_charge(),
                        b.work_steps(),
                    ],
                    fingerprint: result.as_ref().ok().map(|v| v.fingerprint()),
                    error: result.err().map(|e| format!("{e:?}")),
                }
            };
            let baseline = run([0; 4], MAXIMUM, &caps);
            assert!(baseline.error.is_none());
            let view = bundle.display().relation_source();
            let n = source.nodes().len();
            let nd = view.definition_count();
            let ne = view.reference_count();
            let headers = (0..n)
                .map(|i| bundle.relations().headers(i).unwrap().len())
                .sum::<usize>();
            let nr = 2 * ne;
            let records = 2 * n + nd + ne + headers + nr;
            let spool = n * std::mem::size_of::<typaxis_pdf::book_v2::BookV2RelatedStructureNode>()
                + n * std::mem::size_of::<usize>()
                + nd * std::mem::size_of::<typaxis_pdf::book_v2::BookV2NoteStructure>()
                + ne * std::mem::size_of::<typaxis_pdf::book_v2::BookV2NoteStructureEdge>()
                + (headers + nr) * std::mem::size_of::<usize>();
            assert_eq!(baseline.after[0] - floor[0], 1 + records as u64);
            assert_eq!(baseline.after[1] - floor[1], spool as u64);
            assert_eq!(baseline.after[2], floor[2]);
            for (axis, cap, error) in [
                (0, caps.base().get().max_fragments, "Records"),
                (1, caps.base().get().max_spool_bytes, "Spool"),
            ] {
                let delta = baseline.after[axis] - floor[axis];
                let mut prior = [0; 4];
                prior[axis] = cap - delta;
                let exact = run(prior, MAXIMUM, &caps);
                assert!(exact.error.is_none());
                assert_eq!(exact.after[axis], cap);
                assert_eq!(exact.fingerprint, baseline.fingerprint);
                prior[axis] += 1;
                let short = run(prior, MAXIMUM, &caps);
                assert_eq!(short.error.as_deref(), Some(error));
                assert!(short.after[axis] <= cap);
            }
            let output = caps.base().get().max_output_bytes;
            let exact = run([0, 0, output, 0], MAXIMUM, &caps);
            assert!(exact.error.is_none());
            assert_eq!(exact.after[2], output);
            let over = run([0, 0, output + 1, 0], MAXIMUM, &caps);
            assert_eq!(over.error.as_deref(), Some("Output"));
            assert_eq!(over.constructor, [floor[0], floor[1], output + 1, floor[3]]);
            let exact = run([0; 4], baseline.after[3], &caps);
            assert_eq!(exact.fingerprint, baseline.fingerprint);
            let short = run([0; 4], baseline.after[3] - 1, &caps);
            assert_eq!(short.error.as_deref(), Some("Work"));
            assert_eq!(&short.after[..3], &baseline.after[..3]);
            assert!(short.after[3] <= baseline.after[3] - 1);
            let shifted = run([0, 0, 0, floor[3] + 17], MAXIMUM, &caps);
            assert_eq!(shifted.after[3], baseline.after[3] + 17);
            let changed = changed_caps(&caps, |c| c.max_fragments += 1);
            let mismatch = run([0; 4], MAXIMUM, &changed);
            assert_eq!(mismatch.error.as_deref(), Some("Identity"));
            assert_eq!(mismatch.constructor, floor);
            let cap = caps.base().get().max_fragments;
            let exact = run([cap - 1, 0, 0, 0], MAXIMUM, &caps);
            assert_eq!(exact.constructor[0], cap);
            let refused = run([cap, 0, 0, 0], MAXIMUM, &caps);
            assert_eq!(refused.error.as_deref(), Some("Records"));
            assert_eq!(refused.constructor[0], cap);
            let lower = run([0; 4], floor[3] - 1, &caps);
            assert_eq!(lower.error.as_deref(), Some("Work"));
            assert_eq!(
                lower.constructor,
                [floor[0] + 1, floor[1], floor[2], floor[3]]
            );
            let mut b = Relations::new(source, &caps, MAXIMUM, 0, 0, 0, 0).unwrap();
            let a = b.build().unwrap();
            let charge = [
                a.record_charge(),
                a.spool_charge(),
                a.output_charge(),
                a.work_steps(),
            ];
            let again = b.build().unwrap();
            assert_eq!(a.fingerprint(), again.fingerprint());
            assert_eq!(again.record_charge() - charge[0], records as u64);
            assert_eq!(again.spool_charge() - charge[1], spool as u64);
            assert_eq!(again.output_charge(), charge[2]);
            assert!(again.work_steps() > charge[3]);
            let mut failed =
                Relations::new(source, &caps, baseline.after[3] - 1, 0, 0, 0, 0).unwrap();
            assert!(matches!(failed.build(), Err(RelationError::Work)));
            let accepted = [
                failed.record_charge(),
                failed.spool_charge(),
                failed.output_charge(),
                failed.work_steps(),
            ];
            assert!(failed.build().is_err());
            let extra_reservation = if view.tables().is_empty() {
                [records as u64, spool as u64]
            } else {
                [0, 0]
            };
            assert_eq!(
                [
                    failed.record_charge(),
                    failed.spool_charge(),
                    failed.output_charge(),
                    failed.work_steps()
                ],
                [
                    accepted[0] + extra_reservation[0],
                    accepted[1] + extra_reservation[1],
                    accepted[2],
                    accepted[3]
                ]
            );
        })
        .unwrap();
    }
}

#[test]
fn book_v2_column_relations_original_media_native_vector_and_balanced_six_stream_sources() {
    let caps = display_caps();
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        for balanced in [false, true] {
            let mut data=super::super::super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,false,false);
            balance(&mut data, balanced);
            anchors(&mut data);
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
                _ => figure_input(
                    &root,
                    data,
                    match kind {
                        "png" => PNG,
                        "jpeg" => JPEG,
                        _ => SVG,
                    },
                    &caps,
                ),
            };
            let facts =
                input_relations(&format!("relations-media-{kind}-{balanced}"), &input, &caps);
            assert_eq!((facts.notes, facts.edges), (0, 0));
        }
    }
    let root = Root::new();
    let input = vector_input(&root, balanced_six_stream_data(), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    relation_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            let facts = inspect_relations(bundle.relations());
            assert_eq!((facts.notes, facts.edges, facts.relocated), (1, 1, 1));
            assert!(bundle
                .display()
                .source()
                .source()
                .stable()
                .sequence()
                .pages()
                .iter()
                .any(|p| p.balance().is_some()));
            assert!(bundle
                .display()
                .paints()
                .iter()
                .any(|p| matches!(p, Paint::Image(_))));
            relation_probes::emit(
                "relations-balanced-six-stream",
                bundle,
                &caps,
                observed,
                MAXIMUM,
            );
        },
    )
    .unwrap();
    for balanced in [false, true] {
        let mut data = plain_data("Pro", false, true);
        let span = data["document"]["blocks"][0]["span"].clone();
        let force = json!({"kind":"page_break","node_id":0,"classes":[],"span":span});
        data["document"]["blocks"]
            .as_array_mut()
            .unwrap()
            .insert(0, force.clone());
        data["document"]["blocks"]
            .as_array_mut()
            .unwrap()
            .push(force);
        balance(&mut data, balanced);
        semantic_renumber(&mut data["document"], &mut 0);
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let facts = input_relations(
            &format!("relations-forced-empty-body-{balanced}"),
            &input,
            &caps,
        );
        assert_eq!((facts.notes, facts.edges, facts.headers), (0, 0, 0));
    }
}
#[test]
fn book_v2_column_relations_inherit_actual_negative_content_settlement_without_new_output_credit() {
    const FONT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/vmb-book/cid-font/extraction.ttf"
    ));
    let caps = display_caps();
    let mut data = plain_data(&"AB ".repeat(48), false, false);
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        m["width"] = (660 * 65536).into();
        m["trim"]["width"] = (660 * 65536).into();
        m["body"]["width"] = (610 * 65536).into();
    }
    let root = Root::new();
    let input = display_input(&root, data, Some(FONT), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    relation_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, builder, observed| {
            let content = bundle.marked().content();
            assert!(content.output_charge() < content.source().output_charge());
            assert_eq!(bundle.relations().output_charge(), content.output_charge());
            let repeated = builder.build().unwrap();
            assert_eq!(repeated.output_charge(), observed.output_charge());
            assert_eq!(repeated.fingerprint(), bundle.relations().fingerprint());
            inspect_relations(&repeated);
            relation_probes::emit(
                "relations-negative-settled-output",
                bundle,
                &caps,
                observed,
                MAXIMUM,
            );
        },
    )
    .unwrap();
}
#[test]
fn book_v2_column_relations_driver_exact_limits_callback_failure_and_cumulative_retry_keep_all_axes(
) {
    let caps = display_caps();
    let data = plain_data("Pro", true, false);
    let root = Root::new();
    let input = display_input(&root, data.clone(), None, &caps);
    let mut warm = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut peak = 0;
    relation_drive(&input, &caps, MODE, &mut warm, 4, |bundle, _, _| {
        peak = bundle
            .encoded_prefix()
            .output_charge()
            .max(bundle.marked().content().source().output_charge())
            .max(bundle.marked().content().output_charge());
    })
    .unwrap();
    let mut totals = paint_observation(warm.observation());
    totals[2] = peak;
    for axis in 0..4 {
        for short in [false, true] {
            let cap = totals[axis] - u64::from(short);
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
                _ => caps.clone(),
            };
            let maximum = if axis == 3 { cap } else { MAXIMUM };
            let root = Root::new();
            let input = display_input(&root, data.clone(), None, &limits);
            let mut budget = BookV2ColumnPageBudget::new(&limits, maximum);
            let mut delivered = false;
            let result = relation_drive(
                &input,
                &limits,
                MODE,
                &mut budget,
                4,
                |bundle, _, observed| {
                    delivered = true;
                    inspect_relations(bundle.relations());
                    if axis == 2 {
                        assert_eq!(
                            bundle
                                .encoded_prefix()
                                .output_charge()
                                .max(bundle.marked().content().source().output_charge())
                                .max(bundle.marked().content().output_charge()),
                            cap
                        );
                    }
                    relation_probes::emit(
                        &format!("relations-driver-exact-{axis}"),
                        bundle,
                        &limits,
                        observed,
                        maximum,
                    );
                },
            );
            if short {
                assert!(result.is_err(), "axis{axis}:{result:?}");
                assert!(!delivered);
                let prefix = paint_observation(budget.observation());
                assert!(
                    relation_drive(&input, &limits, MODE, &mut budget, 4, |_, _, _| delivered =
                        true)
                    .is_err()
                );
                assert!(!delivered);
                let after = paint_observation(budget.observation());
                for i in [0, 1, 3] {
                    assert!(after[i] >= prefix[i]);
                }
                assert!(
                    after[2] >= prefix[2],
                    "retry may reserve fresh upstream bytes: {after:?} after {prefix:?}"
                );
            } else {
                assert!(result.is_ok(), "axis{axis}:{result:?}");
                assert!(delivered);
                if axis != 2 {
                    assert_eq!(paint_observation(budget.observation())[axis], cap);
                }
            }
            assert!(paint_observation(budget.observation())[axis] <= cap);
        }
    }
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut callback_after = [0; 4];
    let result: Result<(), _> = relation_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, builder, observed| {
            relation_probes::emit(
                "relations-driver-before-callback-repeat",
                bundle,
                &caps,
                observed,
                MAXIMUM,
            );
            let repeated = builder.build().unwrap();
            assert_eq!(repeated.fingerprint(), bundle.relations().fingerprint());
            assert_eq!(repeated.output_charge(), bundle.relations().output_charge());
            callback_after = [
                builder.record_charge(),
                builder.spool_charge(),
                builder.output_charge(),
                builder.work_steps(),
            ];
            Err("relation callback refusal")
        },
    )
    .unwrap();
    assert_eq!(result, Err("relation callback refusal"));
    assert_eq!(paint_observation(budget.observation()), callback_after);
    relation_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            assert_eq!(bundle.caller_before(), callback_after);
            assert!(observed.record_charge() > callback_after[0]);
            relation_probes::emit(
                "relations-driver-cumulative-retry",
                bundle,
                &caps,
                observed,
                MAXIMUM,
            );
        },
    )
    .unwrap();
    let maximum = callback_after[3] - 1;
    let mut budget = BookV2ColumnPageBudget::new(&caps, maximum);
    let mut accepted = [0; 4];
    relation_drive(&input, &caps, MODE, &mut budget, 4, |_, builder, _| {
        assert!(matches!(builder.build(), Err(RelationError::Work)));
        accepted = [
            builder.record_charge(),
            builder.spool_charge(),
            builder.output_charge(),
            builder.work_steps(),
        ];
    })
    .unwrap();
    assert_eq!(paint_observation(budget.observation()), accepted);
    assert!(accepted[3] <= maximum);
}

fn manual_relations(
    case: &str,
    observed: Option<BookV2PdfConvergenceObservation>,
    scope: &str,
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    caps: &M4EffectiveResourceLimits,
) {
    let mut constructors = [[0; 4]; 12];
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
    macro_rules! stage {($slot:expr,$builder:ident,$source:expr $(,$arg:expr)?)=>{{let mut b=$builder::new($source,$($arg,)?caps,MAXIMUM,0,0,0,0).unwrap();constructors[$slot]=[b.record_charge(),b.spool_charge(),b.output_charge(),b.work_steps()];b.build().unwrap()}};}
    let streams = stage!(1, BookV2ColumnFontStreamBuilder, &cids);
    let objects = stage!(2, BookV2ColumnFontObjectBuilder, &streams, 4);
    let text = stage!(3, BookV2ColumnTextCommandBuilder, &objects);
    let mut images = fonts
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
        images.record_charge(),
        images.spool_charge(),
        text.output_charge(),
        images.work_steps(),
    ];
    let selected = images.select_images().unwrap();
    let raster = images.write_raster_programs(&selected).unwrap();
    let mut vb = BookV2ColumnVectorProgramBuilder::new(
        &raster,
        caps,
        MAXIMUM,
        0,
        0,
        text.output_charge(),
        0,
    )
    .unwrap();
    constructors[5] = [
        vb.record_charge(),
        vb.spool_charge(),
        vb.output_charge(),
        vb.work_steps(),
    ];
    let vectors = vb.build().unwrap();
    let objects = stage!(
        6,
        BookV2ColumnImageObjectBuilder,
        &vectors,
        objects.next_object()
    );
    let commands = stage!(7, BookV2ColumnImageCommandBuilder, &objects);
    let scopes = stage!(8, BookV2ColumnMarkedScopeBuilder, &commands);
    let mut cb =
        BookV2ColumnMarkedContentBuilder::new(&scopes, &text, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[9] = [
        cb.record_charge(),
        cb.spool_charge(),
        cb.output_charge(),
        cb.work_steps(),
    ];
    let content = cb.build().unwrap();
    let source = stage!(10, BookV2ColumnSourceStructureBuilder, &content);
    let mut builder = Relations::new(&source, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[11] = [
        builder.record_charge(),
        builder.spool_charge(),
        builder.output_charge(),
        builder.work_steps(),
    ];
    let tree = builder.build().unwrap();
    let facts = inspect_relations(&tree);
    if scope == "retained_source_cut" {
        assert!(facts.headers > 0);
        assert_eq!((facts.notes, facts.edges), (0, 0));
    } else {
        assert_eq!((facts.headers, facts.notes, facts.edges), (0, 1, 1));
    }
    assert_eq!(facts.notes, facts.relocated + facts.unpainted);
    relation_probes::emit_inner(
        case,
        &tree,
        caps,
        observed,
        observed,
        None,
        scope,
        MAXIMUM,
        constructors,
        units,
        None,
    );
}
#[test]
fn book_v2_column_relations_retained_source_cut_gid71_keeps_original_header_registry() {
    with_retained_cut_display(|display, caps| {
        let original = display
            .text()
            .draws()
            .iter()
            .filter(|d| !d.repeated_header())
            .flat_map(|d| d.glyphs())
            .map(|g| g.original_gid().get())
            .collect::<BTreeSet<_>>();
        let repeated = display
            .text()
            .draws()
            .iter()
            .filter(|d| d.repeated_header())
            .flat_map(|d| d.glyphs())
            .map(|g| g.original_gid().get())
            .collect::<BTreeSet<_>>();
        assert!(original.contains(&59) && !original.contains(&71));
        assert!(repeated.contains(&71));
        manual_relations(
            "relations-context-retained-cut",
            None,
            "retained_source_cut",
            display,
            caps,
        );
    });
}
#[test]
#[ignore = "requires TYPAXIS_HARANO_FONT pointing to original HaranoAjiMincho-Regular.otf SHA256 66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"]
fn book_v2_column_relations_original_harano_cid_cff_selected_source_relations() {
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
    let facts = input_relations("relations-original-harano-cid-cff", &input, &caps);
    assert_eq!((facts.notes, facts.edges, facts.unpainted), (2, 1, 1));
}

#[test]
fn book_v2_column_relations_manual_source_callbacks_use_actual_native_vector_definition_graphs() {
    let caps = display_caps();
    for kind in ["native", "vector"] {
        let data=super::super::super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,true,true);
        let root = Root::new();
        let input = if kind == "native" {
            let text = data["text_buffers"][0]["utf8"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec();
            native_input_with_source(&root, data, &text, &caps)
        } else {
            vector_input(&root, data, &caps)
        };
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        math_drive(&input, &caps, MODE, &mut budget, |source, _, observed| {
            let mut b =
                BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                    .unwrap();
            let display = b.build().unwrap();
            manual_relations(
                &format!("relations-manual-source-{kind}"),
                Some(observed),
                "source_callback",
                &display,
                &caps,
            );
        })
        .unwrap();
    }
}

#[test]
fn book_v2_column_relations_original_header_reference_survives_actual_artifact_repetitions() {
    let caps = display_caps();
    let mut data = table_data("common", "Pro");
    let plain = plain_data("Pro", true, false);
    let mut note = plain["document"]["footnotes"][0].clone();
    note["footnote_id"] = "header-relation-note".into();
    data["document"]["footnotes"] = json!([note]);
    let table = &mut data["document"]["blocks"][0];
    let span = table["head"][0]["cells"][0]["blocks"][0]["span"].clone();
    table["head"][0]["cells"][0]["blocks"][0]["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"header-relation-note"}));
    let row = table["body"][0].clone();
    table["body"] = vec![row; 40].into();
    // Preserve all original rows; this authored height bounds each joint page search.
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        m["body"]["height"] = (1024 * 65536).into();
        m["body"]["width"] = (290 * 65536).into();
        let body = m["body"].clone();
        let bottom = m["height"].as_i64().unwrap();
        m["footnote"] =
            json!({"x":body["x"],"y":bottom-9_000_000,"width":body["width"],"height":128*65536});
    }
    semantic_renumber(&mut data["document"], &mut 0);
    let root = Root::new();
    let input = display_input(&root, data, Some(DISPLAY_LIST_FONT), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    relation_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            let wire = package_wire(bundle.display());
            let original_rows = wire["document"]["blocks"][0]["body"].as_array().unwrap();
            assert_eq!(original_rows.len(), 40);
            assert!(original_rows.iter().all(|row| row["cells"]
                .as_array()
                .unwrap()
                .iter()
                .all(|cell| cell["blocks"].as_array().unwrap().len() == 16)));
            let facts = inspect_relations(bundle.relations());
            assert_eq!((facts.notes, facts.edges, facts.relocated), (1, 1, 1));
            let reference_owner = bundle.relations().source().nodes()
                [bundle.relations().edges()[0].reference_node()]
            .source()
            .key()
            .owner();
            let groups = bundle
                .marked()
                .content()
                .source()
                .groups()
                .iter()
                .filter(|g| g.owner() == Some(reference_owner))
                .collect::<Vec<_>>();
            assert!(groups.iter().any(|g| g.artifact().is_some()));
            assert_eq!(groups.iter().filter(|g| g.artifact().is_none()).count(), 1);
            assert_eq!(facts.selected[0], Some(reference_owner.get()));
            relation_probes::emit(
                "relations-original-header-reference",
                bundle,
                &caps,
                observed,
                MAXIMUM,
            );
        },
    )
    .unwrap();
}
#[test]
fn book_v2_column_relations_equal_payload_image_ids_keep_distinct_original_source_nodes() {
    let caps = display_caps();
    for (kind, bytes) in [("png", PNG), ("jpeg", JPEG), ("svg", PAINT_CURVES)] {
        let mut data=super::super::super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,false,false);
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
        relation_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, _, observed| {
                inspect_relations(bundle.relations());
                let selected = bundle.marked().paints().rasters().source();
                assert_eq!(selected.images().len(), 1);
                assert_eq!(
                    selected
                        .uses()
                        .iter()
                        .map(|u| u.usage().image().image_id().get())
                        .collect::<BTreeSet<_>>(),
                    BTreeSet::from([0, 1])
                );
                let owners = selected
                    .uses()
                    .iter()
                    .map(|u| u.usage().owner())
                    .collect::<BTreeSet<_>>();
                assert!(owners.len() >= 2);
                for owner in owners {
                    let node = bundle
                        .relations()
                        .source()
                        .node_index(Key::new(owner, Slot::Source))
                        .unwrap();
                    assert_eq!(
                        bundle.relations().source().nodes()[node]
                            .source()
                            .pdf_role(),
                        "Figure"
                    );
                    assert!(bundle.relations().headers(node).unwrap().is_empty());
                }
                relation_probes::emit(
                    &format!("relations-image-alias-{kind}"),
                    bundle,
                    &caps,
                    observed,
                    MAXIMUM,
                );
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_column_relations_original_marked_companions_keep_complete_source_relations() {
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
        let facts = input_relations(
            &format!("relations-original-list-notes-{balanced}"),
            &input,
            &caps,
        );
        assert!(facts.notes > 0 && facts.edges > 0 && facts.unpainted > 0);
    }
    for repeated in [false, true] {
        let data = fraction_data(repeated);
        let source = data["text_buffers"][0]["utf8"]
            .as_str()
            .unwrap()
            .as_bytes()
            .to_vec();
        let root = Root::new();
        let input = native_input_with_source(&root, data, &source, &caps);
        let facts = input_relations(
            &format!("relations-original-fraction-{repeated}"),
            &input,
            &caps,
        );
        assert_eq!((facts.notes, facts.edges), (0, 0));
        assert_eq!(facts.headers > 0, repeated);
    }
    const FONT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/vmb-book/cid-font/extraction.ttf"
    ));
    for (case, data, font) in [
        (
            "relations-original-real-gsub-extraction",
            plain_data("ABA fi X D E f i", false, false),
            Some(FONT),
        ),
        (
            "relations-original-description-languages",
            description_data(),
            None,
        ),
        (
            "relations-original-positive-content-settlement",
            plain_data("Pro", false, false),
            None,
        ),
    ] {
        let root = Root::new();
        let input = display_input(&root, data, font, &caps);
        let facts = input_relations(case, &input, &caps);
        assert_eq!((facts.notes, facts.edges, facts.headers), (0, 0, 0));
    }
    let mut data=super::super::super::super::super::super::super::super::super::super::block_widths::column_block_data("vector",true,false);
    let mut unused = data["document"]["footnotes"][0].clone();
    unused["footnote_id"] = "marked-unused-rich".into();
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
    relation_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            let facts = inspect_relations(bundle.relations());
            assert!(facts.unpainted > 0);
            assert_eq!(
                bundle
                    .display()
                    .source()
                    .source()
                    .unreferenced_definitions(),
                1
            );
            let source = bundle.relations().source();
            assert!(source
                .nodes()
                .iter()
                .any(|n| excluded.contains(&n.source().key().owner().get())));
            for n in source.nodes() {
                if excluded.contains(&n.source().key().owner().get()) {
                    assert!(n.first_binding().is_none());
                }
            }
            assert!(bundle
                .marked()
                .content()
                .source()
                .groups()
                .iter()
                .all(|g| g.owner().is_none_or(|o| !excluded.contains(&o.get()))));
            relation_probes::emit(
                "relations-original-unused-rich",
                bundle,
                &caps,
                observed,
                MAXIMUM,
            );
        },
    )
    .unwrap();
}
