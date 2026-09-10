use super::*;
#[path = "book_v2_navigation_geometry_tests.rs"]
mod navigation_geometry;
use typaxis_pdf::book_v2::{
    BookV2ListNumbering, BookV2SourceStructure, BookV2StructureRelationBuilder as Relations,
};
pub(super) fn check(
    source: &BookV2SourceStructure<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
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
    limits: &M4EffectiveResourceLimits,
) -> serde_json::Value {
    const WORK: u64 = 1_000_000_000;
    let prepared = source.source().source().display().source().source().flow();
    let flow = prepared.lines().prepared().source_flow();
    let probe = std::cell::RefCell::new(None);
    let run = |records, spool, output, work, prior_work, capture| -> Result<_, VE> {
        let mut builder = Relations::new(source, limits, work, records, spool, output, prior_work)?;
        let tree = builder.build()?;
        assert!(std::ptr::eq(tree.source(), source));
        assert_eq!(tree.nodes().len(), source.nodes().len());
        assert_eq!(tree.output_charge(), output.max(source.output_charge()));
        assert!(tree.headers(usize::MAX).is_none());
        assert!(tree.related_nodes(usize::MAX).is_none());
        let mut seen = BTreeSet::new();
        for &i in tree.reading_order() {
            assert!(seen.insert(i));
            if i == 0 {
                assert_eq!(tree.nodes()[i].depth(), 1);
            } else {
                let parent = tree.nodes()[i].parent().unwrap();
                assert!(seen.contains(&parent));
                assert_eq!(tree.nodes()[i].depth(), tree.nodes()[parent].depth() + 1);
            }
        }
        assert_eq!(seen.len(), source.nodes().len());
        let mut children = BTreeSet::new();
        for (i, n) in tree.nodes().iter().enumerate() {
            let mut next = n.first_child();
            while let Some(child) = next {
                assert!(children.insert(child));
                assert_eq!(tree.nodes()[child].parent(), Some(i));
                next = tree.nodes()[child].next_sibling();
            }
        }
        assert_eq!(children.len(), tree.nodes().len() - 1);
        assert_eq!(tree.edges().len(), prepared.references().len());
        assert_eq!(tree.notes().len(), prepared.footnotes().definitions().len());
        let mut edge_seen = BTreeSet::new();
        for (di, note) in tree.notes().iter().enumerate() {
            let original = &prepared.footnotes().definitions()[di];
            assert_eq!(
                source.nodes()[note.node_index()].source().key().owner(),
                original.owner()
            );
            assert_eq!(
                note.painted(),
                source.nodes()[note.label_node()].first_binding().is_some()
            );
            let mut edge = note.first_reference();
            let mut expected = Vec::new();
            let mut painted = Vec::new();
            while let Some(ei) = edge {
                assert!(edge_seen.insert(ei));
                let e = &tree.edges()[ei];
                assert_eq!(e.definition_index(), di);
                assert_eq!(e.note_node(), note.node_index());
                expected.push(e.reference_node());
                if e.painted() {
                    painted.push(ei);
                }
                edge = e.next_for_note();
            }
            assert_eq!(tree.related_nodes(note.node_index()).unwrap(), expected);
            assert_eq!(note.return_reference(), painted.first().copied());
            if note.painted() {
                let ei = note.reading_reference().unwrap();
                assert!(painted.contains(&ei));
                let mut branch = tree.edges()[ei].reference_node();
                loop {
                    let parent = source.nodes()[branch].parent().unwrap();
                    let node = source.nodes()[parent].source();
                    if matches!(
                        node.pdf_role(),
                        "P" | "H1" | "H2" | "H3" | "H4" | "H5" | "H6"
                    ) || (node.pdf_role() == "Lbl"
                        && node.key().slot()
                            == typaxis_syntax::book_v2::BookV2StructureSlot::Source)
                    {
                        break;
                    }
                    branch = parent;
                }
                assert_eq!(note.insertion_after(), Some(branch));
                assert_eq!(
                    tree.nodes()[note.node_index()].parent(),
                    source.nodes()[branch].parent()
                );
                let mut next = tree.nodes()[branch].next_sibling();
                let mut found = false;
                while let Some(i) = next {
                    if i == note.node_index() {
                        found = true;
                        break;
                    }
                    assert_eq!(source.nodes()[i].source().pdf_role(), "Note");
                    next = tree.nodes()[i].next_sibling();
                }
                assert!(found);
            } else {
                assert!(note.reading_reference().is_none());
                assert_eq!(tree.nodes()[note.node_index()].parent(), Some(0));
            }
        }
        assert_eq!(edge_seen.len(), tree.edges().len());
        for (e, r) in tree.edges().iter().zip(prepared.references()) {
            assert_eq!(
                source.nodes()[e.reference_node()].source().key().owner(),
                r.source().owner()
            );
            assert_eq!(e.source_definition(), r.source().source_definition());
            assert_eq!(e.definition_index(), r.source().definition_index());
            assert_eq!(
                e.painted(),
                source.nodes()[e.reference_label()]
                    .first_binding()
                    .is_some()
            );
            assert_eq!(
                tree.related_nodes(e.reference_node()).unwrap(),
                [e.note_node()]
            );
        }
        for table in flow.tables() {
            let head = table
                .rows()
                .iter()
                .filter(|r| r.section() == typaxis_syntax::ProductionTableSection::Head)
                .flat_map(|r| table.cells()[r.cells()].iter())
                .collect::<Vec<_>>();
            for row in table.rows() {
                for cell in &table.cells()[row.cells()] {
                    let i = source
                        .node_index(Key::new(cell.owner(), Slot::Source))
                        .unwrap();
                    let mut expected = Vec::new();
                    if row.section() == typaxis_syntax::ProductionTableSection::Body {
                        let columns =
                            cell.column()..cell.column() + u32::from(cell.colspan().get());
                        for header in &head {
                            if (header.column()
                                ..header.column() + u32::from(header.colspan().get()))
                                .any(|c| columns.contains(&c))
                            {
                                expected.push(
                                    source
                                        .node_index(Key::new(header.owner(), Slot::Source))
                                        .unwrap(),
                                );
                            }
                        }
                    }
                    assert_eq!(tree.headers(i).unwrap(), expected);
                }
            }
        }
        for list in flow.lists() {
            let i = source
                .node_index(Key::new(list.owner(), Slot::Source))
                .unwrap();
            assert_eq!(
                tree.nodes()[i].list_numbering(),
                Some(if list.ordered() {
                    BookV2ListNumbering::Decimal
                } else {
                    BookV2ListNumbering::Disc
                })
            );
        }
        if capture {
            *probe.borrow_mut() = Some(serde_json::json!({
                "navigation":navigation_geometry::check(&tree,limits),
                "nodes":tree.nodes().iter().enumerate().map(|(i,n)| serde_json::json!({"parent":n.parent(),"first_child":n.first_child(),"next":n.next_sibling(),
                    "depth":n.depth(),"headers":tree.headers(i).unwrap(),"related":tree.related_nodes(i).unwrap(),
                    "list_numbering":n.list_numbering().map(|v| format!("{v:?}"))})).collect::<Vec<_>>(),
                "order":tree.reading_order(),
                "notes":tree.notes().iter().map(|n| serde_json::json!({"node":n.node_index(),"link":n.link_node(),"label":n.label_node(),"painted":n.painted(),
                    "first":n.first_reference(),"return":n.return_reference(),"reading":n.reading_reference(),"after":n.insertion_after()})).collect::<Vec<_>>(),
                "edges":tree.edges().iter().map(|e| serde_json::json!({"reference":e.reference_node(),"note":e.note_node(),"definition":e.definition_index(),
                    "source_definition":e.source_definition(),"ref_link":e.reference_link(),"ref_label":e.reference_label(),
                    "def_link":e.definition_link(),"def_label":e.definition_label(),"painted":e.painted(),"next":e.next_for_note()})).collect::<Vec<_>>(),
                "first_demand": (0..tree.notes().len()).map(|i| source.source().source().display().source().source().stable().sequence().pages().last().unwrap()
                    .next_state().source_state().demand().first_reference(i).map(|n| n.get())).collect::<Vec<_>>()
            }));
            let repeat = builder.build()?;
            assert_eq!(repeat.fingerprint(), tree.fingerprint());
            assert_eq!(repeat.output_charge(), tree.output_charge());
        }
        Ok((
            tree.record_charge(),
            tree.spool_charge(),
            tree.output_charge(),
            tree.work_steps(),
            tree.fingerprint(),
        ))
    };
    let expected = run(0, 0, 0, WORK, 0, true).unwrap();
    let base = limits.base().get();
    let pr = base.max_fragments - (expected.0 - source.record_charge());
    let ps = base.max_spool_bytes - (expected.1 - source.spool_charge());
    let exact = run(pr, ps, base.max_output_bytes, expected.3, 0, false).unwrap();
    assert_eq!(
        (exact.0, exact.1, exact.2),
        (
            base.max_fragments,
            base.max_spool_bytes,
            base.max_output_bytes
        )
    );
    assert_eq!(exact.4, expected.4);
    assert!(matches!(
        run(pr + 1, ps, 0, WORK, 0, false),
        Err(VE::Records)
    ));
    assert!(matches!(run(pr, ps + 1, 0, WORK, 0, false), Err(VE::Spool)));
    assert!(matches!(
        run(0, 0, base.max_output_bytes + 1, WORK, 0, false),
        Err(VE::Output)
    ));
    assert!(matches!(
        run(0, 0, 0, expected.3 - 1, 0, false),
        Err(VE::Work)
    ));
    let shifted = run(0, 0, 0, WORK, source.work_steps() + 17, false).unwrap();
    assert_eq!(shifted.3, expected.3 + 17);
    assert_eq!(shifted.4, expected.4);
    let mut failed = Relations::new(source, limits, expected.3 - 1, 0, 0, 0, 0).unwrap();
    assert!(matches!(failed.build(), Err(VE::Work)));
    assert_eq!(
        (
            failed.record_charge(),
            failed.spool_charge(),
            failed.output_charge()
        ),
        (expected.0, expected.1, expected.2)
    );
    assert!(failed.build().is_err());
    assert_eq!(failed.work_steps(), expected.3 - 1);
    let mut changed = base.clone();
    changed.max_output_bytes -= 1;
    let changed = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(changed).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    assert!(matches!(
        Relations::new(source, &changed, WORK, 0, 0, 0, 0),
        Err(VE::Identity)
    ));
    probe.into_inner().unwrap()
}
