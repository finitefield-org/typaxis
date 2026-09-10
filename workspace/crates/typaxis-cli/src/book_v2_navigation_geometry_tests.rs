use super::*;
use typaxis_display_list::book_v2::{BookV2BodyPaintIndex as Paint, BookV2MathPaint};
use typaxis_pdf::book_v2::{
    BookV2DestinationKind as Kind, BookV2NavigationGeometryBuilder as Navigation,
    BookV2NavigationGeometryError as NE, BookV2NavigationTarget as Target,
    BookV2StructureRelations,
};
pub(super) fn check(
    source: &BookV2StructureRelations<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
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
    let registry = source.source();
    let marked = registry.source();
    let scopes = marked.source();
    let display = scopes.display();
    let flow = display
        .source()
        .source()
        .flow()
        .lines()
        .prepared()
        .source_flow();
    let rectangle = |r: typaxis_core::Rect| {
        [
            r.x().raw(),
            r.y().raw(),
            r.width().get().raw(),
            r.height().get().raw(),
        ]
    };
    let raw_groups=scopes.groups().iter().map(|g| {
        let paints=g.paints().map(|pi| {
            let bounds=match display.paints()[pi]{
        Paint::Text(i)=>display.text().draws()[i].logical_bounds(),
        Paint::Marker(i)=>Some(display.markers().draws()[i].placement().bounds()),
        Paint::Math(i)=>Some(match display.math().draws()[i].paint(){BookV2MathPaint::Native(n)=>n.bounds(),BookV2MathPaint::Vector(v)=>v.viewport()}),
        Paint::Image(i)=>Some(display.images().draws()[i].paint().viewport()),
        Paint::EquationNumber(i)=>Some(display.numbers().draws()[i].placement().geometry().bounds()),
        Paint::FootnoteSeparator(_)=>None,
            };bounds.map(rectangle)
        }).collect::<Vec<_>>();
        serde_json::json!({"page":g.page_index(),"fragment":g.fragment_index(),"artifact":g.artifact().is_some(),"paints":paints})
    }).collect::<Vec<_>>();

    let nonpainting_lines = display.anchors().nonpainting_lines().iter().map(|line| {
        let f = line.fragment().fragment();
        let typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine { paragraph_index, line_index } = f.source() else { panic!("nonparagraph empty line"); };
        let selected = &display.source().fragment_flow(line.fragment_index()).unwrap().lines().paragraphs()[paragraph_index as usize].lines()[line_index as usize];
        let breaks = selected.items().iter().map(|item| {
            let typaxis_layout::ProductionPlacedInline::Break(b) = item else { panic!("paint in empty line"); };
            b.owner().get()
        }).collect::<Vec<_>>();
        serde_json::json!({"owner":f.owner().get(),"page":f.page_index(),"fragment":line.fragment_index(),"line":line_index,"breaks":breaks,"bounds":rectangle(f.bounds()),"repeated":line.repeated_header()})
    }).collect::<Vec<_>>();
    let raw_input = serde_json::json!({
        "nonpainting_lines":nonpainting_lines, "raw_groups":raw_groups,"raw_page_count":marked.pages().len(),
        "source_anchors":flow.navigation().anchors().iter().map(|(a,n)|serde_json::json!([a.as_str(),n.get()])).collect::<Vec<_>>(),
        "inline_anchors":display.anchors().positions().iter().map(|a|serde_json::json!({"owner":a.anchor().source().owner().get(),"page":a.fragment().fragment().page_index(),"fragment":a.fragment_index(),"x":a.x().raw(),"y":a.baseline().raw(),"repeated":a.repeated_header()})).collect::<Vec<_>>(),
        "source_outline":flow.navigation().outline().iter().map(|e|serde_json::json!({"id":e.outline_id,"parent":e.parent_outline_id,"level":e.level,"target":e.destination.as_str(),"label":e.label})).collect::<Vec<_>>()
    });
    use typaxis_syntax::{ProductionInlineLinkTarget as LT, ProductionInlineReference as IR};
    let mut roots = std::collections::BTreeSet::new();
    for p in flow.paragraphs() {
        for i in p.items() {
            if i.link_target().is_some() {
                roots.insert(
                    registry
                        .node_index(Key::new(i.owner(), Slot::Source))
                        .unwrap(),
                );
            }
            if matches!(i.reference(), Some(IR::Anchor { .. })) {
                roots.insert(
                    registry
                        .node_index(Key::new(i.owner(), Slot::ReferenceLink))
                        .unwrap(),
                );
            }
        }
    }
    for e in source.edges() {
        roots.insert(e.reference_link());
    }
    for n in source.notes() {
        if n.return_reference().is_some() {
            roots.insert(n.link_node());
        }
    }
    let mut expected_error = None;
    for &node in &roots {
        let mut p = registry.nodes()[node].parent();
        while let Some(i) = p {
            if roots.contains(&i) {
                expected_error = Some(NE::ConflictingLinks(
                    registry.nodes()[node].source().key().owner(),
                ));
                break;
            }
            p = registry.nodes()[i].parent();
        }
        if expected_error.is_some() {
            break;
        }
    }
    let mut painted = vec![false; registry.nodes().len()];
    for (gi, raw) in raw_groups.iter().enumerate() {
        if !raw["paints"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| !v.is_null())
        {
            continue;
        }
        if let Some(b) = registry.for_group(gi) {
            let mut n = Some(b.node_index());
            while let Some(i) = n {
                painted[i] = true;
                n = registry.nodes()[i].parent();
            }
        }
    }
    for line in display.anchors().nonpainting_lines() {
        if line.repeated_header() {
            continue;
        }
        let mut cursor = Some(
            registry
                .node_index(Key::new(line.fragment().fragment().owner(), Slot::Source))
                .unwrap(),
        );
        while let Some(i) = cursor {
            painted[i] = true;
            cursor = registry.nodes()[i].parent();
        }
    }
    let unplaced = |target: &str| {
        let owner = flow
            .navigation()
            .anchors()
            .iter()
            .find(|(a, _)| a.as_str() == target)
            .unwrap()
            .1;
        let has_paint = registry
            .node_index(Key::new(owner, Slot::Source))
            .is_some_and(|i| painted[i]);
        let has_anchor = display
            .anchors()
            .positions()
            .iter()
            .any(|a| a.anchor().source().owner() == owner && !a.repeated_header());
        (!has_paint && !has_anchor).then_some(owner)
    };
    if expected_error.is_none() {
        for p in flow.paragraphs() {
            for i in p.items() {
                let slot = if matches!(i.reference(), Some(IR::Anchor { .. })) {
                    Slot::ReferenceLink
                } else {
                    Slot::Source
                };
                let target = match (i.link_target(), i.reference()) {
                    (Some(LT::Internal { anchor_id }), _) => Some(anchor_id),
                    (_, Some(IR::Anchor { target, .. })) => Some(target),
                    _ => None,
                };
                if let Some(target) = target {
                    if painted[registry.node_index(Key::new(i.owner(), slot)).unwrap()] {
                        if let Some(owner) = unplaced(target) {
                            expected_error = Some(NE::UnplacedDestination(owner));
                            break;
                        }
                    }
                }
            }
            if expected_error.is_some() {
                break;
            }
        }
    }
    if expected_error.is_none() {
        for e in flow.navigation().outline() {
            if let Some(owner) = unplaced(e.destination.as_str()) {
                expected_error = Some(NE::UnplacedDestination(owner));
                break;
            }
        }
    }
    if let Some(expected) = expected_error {
        let mut pipeline = typaxis_pdf::book_v2::BookV2PdfPipeline::new(
            display,
            marked.text().source().first_object(),
            limits,
            WORK,
            0,
            0,
            0,
            0,
        )
        .unwrap();
        for _ in 0..2 {
            let before = pipeline.work_steps();
            let error = pipeline
                .with_pdf(|_| panic!("failed navigation reached callback"))
                .unwrap_err();
            assert!(
                matches!(error, typaxis_pdf::book_v2::BookV2PdfPipelineError::Navigation(e) if e == expected)
            );
            assert!(pipeline.work_steps() > before);
        }
        let mut builder = Navigation::new(source, limits, WORK, 0, 0, 0, 0).unwrap();
        assert_eq!(builder.build().err(), Some(expected));
        let before = (
            builder.record_charge(),
            builder.spool_charge(),
            builder.work_steps(),
        );
        assert!(
            before.0 > source.record_charge()
                && before.1 > source.spool_charge()
                && before.2 > source.work_steps()
        );
        assert_eq!(builder.build().err(), Some(expected));
        assert!(
            builder.record_charge() > before.0
                && builder.spool_charge() > before.1
                && builder.work_steps() > before.2
        );
        let mut result = raw_input;
        result["error"] = match expected {
            NE::ConflictingLinks(owner) => serde_json::json!(["conflicting_links", owner.get()]),
            NE::UnplacedDestination(owner) => {
                serde_json::json!(["unplaced_destination", owner.get()])
            }
            _ => unreachable!(),
        };
        return result;
    }
    let probe = std::cell::RefCell::new(None);
    let run = |records, spool, output, work, prior_work, capture| -> Result<_, NE> {
        let mut builder =
            Navigation::new(source, limits, work, records, spool, output, prior_work)?;
        let nav = builder.build()?;
        assert!(std::ptr::eq(nav.source(), source));
        assert_eq!(nav.output_charge(), source.output_charge().max(output));
        assert!(nav.page_rectangles(usize::MAX).is_none());
        assert_eq!(nav.outline().len(), flow.navigation().outline().len());
        let mut all = std::collections::BTreeSet::new();
        for (i, link) in nav.links().iter().enumerate() {
            let mut next = link.first_rectangle();
            let mut fragments = std::collections::BTreeSet::new();
            while let Some(index) = next {
                assert!(all.insert(index));
                let r = nav.rectangles()[index];
                assert_eq!(r.link_index(), i);
                assert!(fragments.insert((r.page_index(), r.fragment_index())));
                let g = &scopes.groups()[r.first_group()];
                assert_eq!(g.page_index(), r.page_index());
                assert_eq!(g.fragment_index(), r.fragment_index());
                assert!(g.artifact().is_none());
                assert!(g.mcid().is_some());
                if let Target::Destination(d) = link.target() {
                    assert!(nav.destinations()[d].position().is_some());
                }
                next = r.next_for_link();
            }
        }
        assert_eq!(all.len(), nav.rectangles().len());
        assert_eq!(
            (0..marked.pages().len())
                .flat_map(|i| nav.page_rectangles(i).unwrap())
                .count(),
            nav.rectangles().len()
        );
        for i in 0..marked.pages().len() {
            assert!(nav
                .page_rectangles(i)
                .unwrap()
                .iter()
                .all(|r| r.page_index() as usize == i));
        }
        for d in nav.destinations() {
            if let Some(p) = d.position() {
                assert!((p.page_index() as usize) < marked.pages().len());
            }
        }
        if capture {
            *probe.borrow_mut() = Some(serde_json::json!({
                "assembly":pdf_assembly::check(&nav,limits),
                "destinations":nav.destinations().iter().map(|d|serde_json::json!({"owner":d.owner().get(),"kind":match d.kind(){Kind::Anchor(s)=>serde_json::json!(["anchor",s]),Kind::Footnote(i)=>serde_json::json!(["footnote",i]),Kind::Reference(i)=>serde_json::json!(["reference",i])},"position":d.position().map(|p|serde_json::json!([p.page_index(),p.fragment_index(),p.x().raw(),p.y().raw()]))})).collect::<Vec<_>>(),
                "links":nav.links().iter().map(|l|serde_json::json!({"node":l.structure_node(),"target":match l.target(){Target::Destination(i)=>serde_json::json!(["destination",i]),Target::Uri(s)=>serde_json::json!(["uri",s])},"first":l.first_rectangle()})).collect::<Vec<_>>(),
                "rectangles":nav.rectangles().iter().map(|r|serde_json::json!({"link":r.link_index(),"page":r.page_index(),"fragment":r.fragment_index(),"group":r.first_group(),"bounds":rectangle(r.bounds()),"next":r.next_for_link()})).collect::<Vec<_>>(),
                "page_counts":(0..marked.pages().len()).map(|i|nav.page_rectangles(i).unwrap().len()).collect::<Vec<_>>(),
                "outline":nav.outline().iter().map(|o|serde_json::json!({"destination":o.destination(),"parent":o.parent(),"first":o.first_child(),"last":o.last_child(),"previous":o.previous(),"next":o.next(),"descendants":o.descendant_count()})).collect::<Vec<_>>(),
                "nonpainting_lines":nonpainting_lines, "raw_groups":raw_groups,
                "raw_page_count":marked.pages().len(),
                "source_anchors":flow.navigation().anchors().iter().map(|(a,n)|serde_json::json!([a.as_str(),n.get()])).collect::<Vec<_>>(),
                "inline_anchors":display.anchors().positions().iter().map(|a|serde_json::json!({"owner":a.anchor().source().owner().get(),"page":a.fragment().fragment().page_index(),"fragment":a.fragment_index(),"x":a.x().raw(),"y":a.baseline().raw(),"repeated":a.repeated_header()})).collect::<Vec<_>>(),
                "source_outline":flow.navigation().outline().iter().map(|e|serde_json::json!({"id":e.outline_id,"parent":e.parent_outline_id,"level":e.level,"target":e.destination.as_str(),"label":e.label})).collect::<Vec<_>>()
            }));
            let again = builder.build()?;
            assert_eq!(again.fingerprint(), nav.fingerprint());
            assert_eq!(again.output_charge(), nav.output_charge());
            assert!(again.record_charge() > nav.record_charge());
        }
        Ok((
            nav.record_charge(),
            nav.spool_charge(),
            nav.output_charge(),
            nav.work_steps(),
            nav.fingerprint(),
        ))
    };
    let full = run(0, 0, 0, WORK, 0, true).unwrap();
    let base = limits.base().get();
    let pr = base.max_fragments - (full.0 - source.record_charge());
    let ps = base.max_spool_bytes - (full.1 - source.spool_charge());
    let exact = run(pr, ps, base.max_output_bytes, full.3, 0, false).unwrap();
    assert_eq!(
        (exact.0, exact.1, exact.2, exact.3, exact.4),
        (
            base.max_fragments,
            base.max_spool_bytes,
            base.max_output_bytes,
            full.3,
            full.4
        )
    );
    assert!(matches!(
        run(pr + 1, ps, 0, WORK, 0, false),
        Err(NE::Resource(VE::Records))
    ));
    assert!(matches!(
        run(pr, ps + 1, 0, WORK, 0, false),
        Err(NE::Resource(VE::Spool))
    ));
    assert!(matches!(
        run(0, 0, base.max_output_bytes + 1, WORK, 0, false),
        Err(NE::Resource(VE::Output))
    ));
    assert!(matches!(
        run(0, 0, 0, full.3 - 1, 0, false),
        Err(NE::Resource(VE::Work))
    ));
    let extra = source.work_steps() + 17;
    assert_eq!(run(0, 0, 0, WORK, extra, false).unwrap().3, full.3 + 17);
    let mut late = Navigation::new(source, limits, full.3 - 1, 0, 0, 0, 0).unwrap();
    assert!(matches!(late.build(), Err(NE::Resource(VE::Work))));
    let consumed = (late.record_charge(), late.spool_charge(), late.work_steps());
    assert!(consumed.0 > source.record_charge());
    assert!(matches!(late.build(), Err(NE::Resource(VE::Work))));
    assert!(
        late.record_charge() >= consumed.0
            && late.spool_charge() >= consumed.1
            && late.work_steps() >= consumed.2
    );
    let mut changed = base.clone();
    changed.max_output_bytes -= 1;
    let changed = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(changed).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    assert!(matches!(
        Navigation::new(source, &changed, WORK, 0, 0, 0, 0),
        Err(NE::Resource(VE::Identity))
    ));
    probe.into_inner().unwrap()
}

#[path = "book_v2_pdf_assembly_tests.rs"]
mod pdf_assembly;
