use super::*;
#[path = "book_v2_structure_relation_tests.rs"]
mod relations;
use typaxis_pdf::book_v2::{BookV2MarkedContent, BookV2SourceStructureBuilder as Structure};
use typaxis_syntax::book_v2::{BookV2StructureKey as Key, BookV2StructureSlot as Slot};
pub(super) fn check(
    source: &BookV2MarkedContent<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
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
) {
    const WORK: u64 = 1_000_000_000;
    let flow = source
        .source()
        .display()
        .source()
        .source()
        .flow()
        .lines()
        .prepared()
        .source_flow();
    let run = |records, spool, output, work, prior_work, capture| -> Result<_, VE> {
        let mut builder = Structure::new(source, limits, work, records, spool, output, prior_work)?;
        let tree = builder.build()?;
        assert!(std::ptr::eq(source, tree.source()));
        assert_eq!(tree.output_charge(), output.max(source.output_charge()));
        assert!(tree.for_group(usize::MAX).is_none());
        assert!(tree
            .node_index(Key::new(typaxis_core::NodeId::new(u32::MAX), Slot::Source))
            .is_none());
        let mut children = std::collections::BTreeSet::new();
        let mut bindings = std::collections::BTreeSet::new();
        let mut all_keys = std::collections::BTreeSet::new();
        for (i, n) in tree.nodes().iter().enumerate() {
            let s = n.source();
            assert!(all_keys.insert(s.key()));
            assert_eq!(tree.node_index(s.key()), Some(i));
            if i == 0 {
                assert_eq!(n.depth(), 1);
                assert!(n.parent().is_none());
            } else {
                assert!(n.parent().unwrap() < i);
                assert_eq!(n.depth(), tree.nodes()[n.parent().unwrap()].depth() + 1);
            }
            let mut child = n.first_child();
            while let Some(c) = child {
                assert!(children.insert(c));
                assert_eq!(tree.nodes()[c].parent(), Some(i));
                child = tree.nodes()[c].next_sibling();
            }
            let mut binding = n.first_binding();
            while let Some(b) = binding {
                assert!(bindings.insert(b));
                assert_eq!(tree.bindings()[b].node_index(), i);
                binding = tree.bindings()[b].next_for_node();
            }
            if let Some(language) = flow.navigation().language(s.key().owner()) {
                assert_eq!(s.language(), language.effective_language());
                assert_eq!(s.source_span(), language.source_span());
                if s.key().slot() == Slot::Source {
                    assert_eq!(s.parent().map(|p| p.owner()), language.parent());
                    if let Some(kind) = flow.navigation().semantic_kind(s.key().owner()) {
                        assert_eq!(s.semantic_kind(), Some(kind.as_str()));
                        assert_eq!(
                            s.pdf_role(),
                            if kind.as_str() == "quote" {
                                "BlockQuote"
                            } else {
                                "Sect"
                            }
                        );
                    }
                }
            } else {
                let number = flow
                    .navigation()
                    .language_children()
                    .iter()
                    .find(|r| r.node_id() == s.key().owner())
                    .unwrap();
                assert_eq!(s.language(), number.effective_language());
                assert_eq!(s.source_span(), Some(number.source_span()));
                assert_eq!(s.pdf_role(), "Span");
                assert_eq!(s.key().slot(), Slot::Source);
            }
        }
        assert_eq!(children.len(), tree.nodes().len() - 1);
        assert_eq!(bindings.len(), tree.bindings().len());
        assert_eq!(
            all_keys.iter().filter(|k| k.slot() == Slot::Source).count(),
            flow.navigation().languages().len() + flow.navigation().language_children().len()
        );
        let mut expected = 0;
        for (i, group) in source.source().groups().iter().enumerate() {
            if group.artifact().is_some() {
                assert!(tree.for_group(i).is_none());
                continue;
            }
            let b = tree.for_group(i).unwrap();
            assert!(std::ptr::eq(b, &tree.bindings()[expected]));
            expected += 1;
            assert_eq!(
                (b.group_index(), b.page_index(), Some(b.mcid())),
                (i, group.page_index(), group.mcid())
            );
            let node = tree.nodes()[b.node_index()].source();
            assert_eq!(Some(node.key().owner()), group.owner());
            assert_eq!(node.alternative(), source.source().alternative(i));
            if group.role() == R::Label
                || (group.role() == R::Text
                    && flow.footnote_marker_text(group.owner().unwrap()).is_some())
            {
                assert_eq!(node.pdf_role(), "Lbl");
                assert!(matches!(
                    node.key().slot(),
                    Slot::ListLabel | Slot::FootnoteLabel
                ));
            } else if group.role() == R::Text
                && flow
                    .navigation()
                    .references()
                    .iter()
                    .any(|(owner, target)| {
                        Some(*owner) == group.owner()
                            && matches!(
                                target,
                                typaxis_syntax::book_v2::BookV2ReferenceTarget::Anchor { .. }
                            )
                    })
            {
                assert_eq!(node.key().slot(), Slot::ReferenceLabel);
                assert_eq!(node.pdf_role(), "Span");
            } else {
                assert_eq!(node.key().slot(), Slot::Source);
            }
        }
        assert_eq!(expected, tree.bindings().len());
        // Table row/cell attributes come from the independently prepared grid.
        for table in flow.tables() {
            for row in table.rows() {
                for cell in &table.cells()[row.cells()] {
                    let index = tree
                        .node_index(Key::new(cell.owner(), Slot::Source))
                        .unwrap();
                    let attributes = tree.nodes()[index].source().table_cell().unwrap();
                    assert_eq!(
                        (
                            attributes.table,
                            attributes.row,
                            attributes.column,
                            attributes.colspan,
                            attributes.rowspan
                        ),
                        (
                            table.owner(),
                            cell.row(),
                            cell.column(),
                            cell.colspan().get(),
                            cell.rowspan().get()
                        )
                    );
                    assert_eq!(
                        attributes.header,
                        row.section() == typaxis_syntax::ProductionTableSection::Head
                    );
                }
            }
        }
        if capture {
            let relations = relations::check(&tree, limits);
            if let Ok(directory) = std::env::var("TYPAXIS_BOOK_SOURCE_STRUCTURE_PROBE")
                .or_else(|_| std::env::var("TYPAXIS_BOOK_STRUCTURE_RELATIONS_PROBE"))
                .or_else(|_| std::env::var("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE"))
            {
                std::fs::create_dir_all(&directory).unwrap();
                let key =
                    |key: Key| serde_json::json!([key.owner().get(), format!("{:?}", key.slot())]);
                let nodes = tree.nodes().iter().map(|node| {
                    let s = node.source();
                    serde_json::json!({"key":key(s.key()), "parent":node.parent(), "depth":node.depth(),
                        "first_child":node.first_child(), "next_sibling":node.next_sibling(), "first_binding":node.first_binding(),
                        "role":s.pdf_role(), "semantic_kind":s.semantic_kind(), "language":s.language(), "alt":s.alternative(),
                        "span":s.source_span().map(|v| [v.source_id().get(),v.start_byte().get(),v.end_byte().get()]),
                        "cell":s.table_cell().map(|v| serde_json::json!({"table":v.table.get(),"row":v.row,"column":v.column,
                            "colspan":v.colspan,"rowspan":v.rowspan,"header":v.header}))})
                }).collect::<Vec<_>>();
                let bindings = tree
                    .bindings()
                    .iter()
                    .map(|b| {
                        serde_json::json!({"node":b.node_index(),"group":b.group_index(),
                    "page":b.page_index(),"mcid":b.mcid(),"next":b.next_for_node()})
                    })
                    .collect::<Vec<_>>();
                let groups = source.source().groups().iter().enumerate().map(|(i,g)| serde_json::json!({
                    "owner":g.owner().map(|n| n.get()), "role":format!("{:?}",g.role()), "page":g.page_index(),
                    "bytes":String::from_utf8(source.group_bytes(i).unwrap().to_vec()).unwrap()})).collect::<Vec<_>>();
                let probe = serde_json::json!({"display":source.source().display().fingerprint(),"wire":serde_json::from_str::<serde_json::Value>(&typaxis_document_package::book_v2::BookV2DocumentPackageEncoder::new().encode(flow.body().body().wire()).unwrap()).unwrap(),"nodes":nodes,"bindings":bindings,"groups":groups,"relations":relations});
                let name = tree
                    .fingerprint()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                std::fs::write(
                    std::path::Path::new(&directory).join(format!("{name}.json")),
                    serde_json::to_vec(&probe).unwrap(),
                )
                .unwrap();
            }
            let repeat = builder.build()?;
            assert_eq!(repeat.fingerprint(), tree.fingerprint());
            assert_eq!(repeat.output_charge(), tree.output_charge());
            assert!(repeat.record_charge() > tree.record_charge());
            assert!(repeat.spool_charge() > tree.spool_charge());
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
    let mut failed = Structure::new(source, limits, expected.3 - 1, 0, 0, 0, 0).unwrap();
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
        Structure::new(source, &changed, WORK, 0, 0, 0, 0),
        Err(VE::Identity)
    ));
}
